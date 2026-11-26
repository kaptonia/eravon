use crate::abi::{self, Ty, Val};
use crate::fault::{Fault, R};
use crate::sol::{self, Ev};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Log {
    pub name: &'static str,
    pub args: Vec<Val>,
}

impl Log {
    pub fn at(&self, i: usize) -> R<&Val> {
        self.args
            .get(i)
            .ok_or_else(|| Fault::Abi(format!("{} 没有第 {i} 个参数", self.name)))
    }
}

fn arg_types(sig: &str) -> R<Vec<Ty>> {
    let open = sig.find('(').ok_or_else(|| Fault::Abi(format!("{sig} 不成一条签名")))?;
    abi::tuple_types(&sig[open..])
}

pub fn decode(topics: &[[u8; 32]], data: &[u8]) -> R<Log> {
    decode_inner(topics, data, true)
}


fn decode_inner(topics: &[[u8; 32]], data: &[u8], take_topics: bool) -> R<Log> {
    let t0 = topics
        .first()
        .ok_or_else(|| Fault::Abi("一条日志至少要有 topic0".into()))?;
    let ev: Ev = sol::event_by_topic(t0)
        .ok_or_else(|| Fault::Abi(format!("不认得的事件 topic0 {}", crate::order::hex_of(t0))))?;
    let tys = arg_types(ev.sig)?;
    if tys.len() != ev.indexed.len() {
        return Err(Fault::Abi(format!("{} 的签名与 indexed 表不等长", ev.name)));
    }
    let plain: Vec<Ty> = tys
        .iter()
        .zip(ev.indexed)
        .filter(|(_, ix)| !**ix)
        .map(|(t, _)| t.clone())
        .collect();
    let mut from_data = abi::decode(&plain, data)?.into_iter();
    let mut from_topic = topics[1..].iter();
    let mut args = Vec::with_capacity(tys.len());
    for (t, ix) in tys.iter().zip(ev.indexed) {
        if *ix {
            match (take_topics, from_topic.next()) {
                (true, Some(w)) => args.push(topic_val(t, w)?),
                (true, None) => {
                    return Err(Fault::Abi(format!("{} 少了一个 topic", ev.name)));
                }
                (false, _) => {}
            }
        } else {
            args.push(
                from_data
                    .next()
                    .ok_or_else(|| Fault::Abi(format!("{} 的 data 段少了一位", ev.name)))?,
            );
        }
    }
    if take_topics {
        let open = ev.sig.find('(').ok_or_else(|| Fault::Abi(format!("{} 的签名不成形", ev.name)))?;
        abi::fits(&ev.sig[open..], &args).map_err(|f| Fault::Abi(format!("{}:{}", ev.name, f.detail())))?;
    }
    if ev.sig == sol::past_trial::ERAVON_EV_PANEL_FROZEN && args.len() == 3 {
        let mut it = args.into_iter();
        let (guild, index, hash) = (it.next().expect("三位"), it.next().expect("三位"), it.next().expect("三位"));
        args = vec![guild, Val::Addr([0; 20]), index, hash, Val::Addr([0; 20])];
    }
    Ok(Log { name: ev.name, args })
}

fn topic_val(t: &Ty, w: &[u8; 32]) -> R<Val> {
    Ok(match t {
        Ty::Addr => {
            if w[..12].iter().any(|x| *x != 0) {
                return Err(Fault::Abi("topic 上的地址字高十二字节该是零".into()));
            }
            Val::Addr(w[12..].try_into().expect("二十字节"))
        }
        Ty::B32 => Val::B32(*w),
        Ty::Bool => Val::Bool(w[31] == 1),
        Ty::Uint => Val::U256(*w),
        _ => return Err(Fault::Abi("变长形不该出现在 topic 上".into())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signature_comes_apart_into_its_argument_types() {
        let t = arg_types(sol::OPENED.sig).expect("拆得开");
        assert_eq!(t, vec![Ty::B32, Ty::Addr, Ty::Addr, Ty::Addr, Ty::Uint, Ty::B32]);
    }

    #[test]
    fn a_nested_tuple_array_comes_apart_too() {
        let t = arg_types("f((uint16,uint16,uint8)[],bytes)").expect("拆得开");
        assert_eq!(t.len(), 2);
        assert_eq!(t[0], Ty::Arr(Box::new(Ty::Tuple(vec![Ty::Uint, Ty::Uint, Ty::Uint]))));
        assert_eq!(t[1], Ty::Bytes);
    }

    #[test]
    fn the_two_halves_of_a_log_come_back_in_declaration_order() {
        let ev = sol::INSTANCE_EVENTS
            .iter()
            .copied()
            .find(|e| e.name == "Swept")
            .expect("表上该有 Swept");
        let mut who = [0u8; 32];
        who[12..].copy_from_slice(&[0x11; 20]);
        let data = abi::encode(&[Val::Addr([0x22; 20]), Val::u64(7)]);
        let log = decode(&[ev.topic0(), who], &data).expect("读得回来");
        assert_eq!(log.name, "Swept");
        assert_eq!(log.args[0], Val::Addr([0x11; 20]));
        assert_eq!(log.args[1], Val::Addr([0x22; 20]));
        assert_eq!(log.args[2].small().expect("小数"), 7);
    }

    #[test]
    fn an_unknown_topic_is_refused_by_name() {
        let f = decode(&[[0x99; 32]], &[]).expect_err("该拒");
        assert_eq!(f.name(), "E_A1_ABI");
    }
}
