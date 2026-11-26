use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_terms as k2;

use crate::abi::{self, Val};
use crate::fault::{Fault, R};
use crate::order;
use crate::precheck::{GATE_CURRENT, GATE_SIG};
use crate::sol;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Registration {
    Held,
    NotHeld,
    Unverified,
}

impl Registration {
    pub const fn word(self) -> &'static str {
        match self {
            Registration::Held => "held",
            Registration::NotHeld => "not-held",
            Registration::Unverified => "unverified",
        }
    }
}

#[derive(Clone, Debug)]
pub struct RegRead {
    pub state: Registration,
    pub endpoints: usize,
    pub why: String,
}

pub fn ahead_registered<T: Transport>(t: &T, table: &Table, logic: &str, who: &str, digests: &[[u8; 32]]) {
    let Ok(who_addr) = order::addr_of("登记人", who) else { return };
    let calls: Vec<(String, String)> = digests
        .iter()
        .filter_map(|d| crate::call::with(sol::CONSENTED, &[Val::Addr(who_addr), Val::B32(*d)]).ok())
        .map(|data| (logic.to_string(), order::hex_of(&data)))
        .collect();
    a2::read::ahead_calls(t, table, None, &calls, a2::seam::TAG_LATEST);
}

pub fn registered<T: Transport>(
    t: &T,
    table: &Table,
    logic: &str,
    who: &str,
    digest: &[u8; 32],
) -> RegRead {
    crate::mark::mark(crate::mark::FEATURE);
    let n = table.len();
    let who_addr = match order::addr_of("登记人", who) {
        Ok(a) => a,
        Err(f) => return RegRead { state: Registration::Unverified, endpoints: n, why: f.detail() },
    };
    let data = match crate::call::with(sol::CONSENTED, &[Val::Addr(who_addr), Val::B32(*digest)]) {
        Ok(d) => d,
        Err(f) => return RegRead { state: Registration::Unverified, endpoints: n, why: f.detail() },
    };
    let read = a2::read::call(t, table, logic, &order::hex_of(&data), a2::seam::TAG_LATEST);
    let (hex, reading) = match read {
        Ok(x) => x,
        Err(a2::fault::Fault::Disagree(d)) => {
            return RegRead {
                state: Registration::Unverified,
                endpoints: n,
                why: format!("端点之间答得不一样:{d}"),
            };
        }
        Err(f) => {
            return RegRead {
                state: Registration::Unverified,
                endpoints: n,
                why: format!("有端点读不到登记簿:{}", f.detail()),
            };
        }
    };
    let held = match order::bytes_of("答", &hex).and_then(|b| abi::tuple_types(crate::sol::src_abi::ERAVON_CONSENTED_RETURNS).and_then(|tys| abi::decode(&tys, &b))) {
        Ok(v) => match v.first().map(Val::boolean) {
            Some(Ok(b)) => b,
            _ => {
                return RegRead {
                    state: Registration::Unverified,
                    endpoints: n,
                    why: format!("登记簿答的不是一枚布尔:{hex}"),
                };
            }
        },
        Err(f) => {
            return RegRead {
                state: Registration::Unverified,
                endpoints: n,
                why: format!("登记簿的答读不成(逻辑地址上也许没有码):{}", f.detail()),
            };
        }
    };
    let state = judge(held, reading.agreement);
    let why = match state {
        Registration::Held => String::new(),
        Registration::NotHeld => "登记簿上没有这一枚".to_string(),
        Registration::Unverified => "只问到一个端点:单源的持有不算持有".to_string(),
    };
    RegRead { state, endpoints: reading.endpoints, why }
}

fn judge(held: bool, agreement: a2::read::Agreement) -> Registration {
    match (held, agreement) {
        (false, _) => Registration::NotHeld,
        (true, a2::read::Agreement::Agreed) => Registration::Held,
        (true, a2::read::Agreement::Single) => Registration::Unverified,
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Signature,
    Caller,
    Registry,
}

impl Form {
    pub const fn word(self) -> &'static str {
        match self {
            Form::Signature => "signature",
            Form::Caller => "caller",
            Form::Registry => "registry",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Party {
    Seller,
    Buyer,
}

impl Party {
    fn name(self) -> &'static str {
        match self {
            Party::Seller => "卖家",
            Party::Buyer => "买家",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Sender<'a> {
    pub from: Option<&'a str>,
    pub via_multicall: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn slot(
    party: Party,
    key: &str,
    current: &str,
    digest: &[u8; 32],
    sig: &[u8],
    sender: Sender<'_>,
    registry: &mut dyn FnMut(&str, &[u8; 32]) -> RegRead,
) -> R<Form> {
    if !sig.is_empty() {
        signed_by_current(party, key, current, digest, sig)?;
        return Ok(Form::Signature);
    }
    let is_party = sender.from.is_some_and(|s| s.eq_ignore_ascii_case(current));
    if is_party && !sender.via_multicall {
        return Ok(Form::Caller);
    }
    let r = registry(key, digest);
    match r.state {
        Registration::Held => Ok(Form::Registry),
        state => {
            let merged = if is_party && sender.via_multicall {
                "这一笔经 Multicall3 合并,实例看见的调用者是 Multicall3,调用者形不成立;"
            } else {
                ""
            };
            let what = match state {
                Registration::NotHeld => format!("{}尚未登记", party.name()),
                _ => "登记未能多端点核实".to_string(),
            };
            Err(Fault::Precheck {
                gate: GATE_SIG,
                why: format!(
                    "{}一槽空:{merged}{}的现行钥 {current} 不是这一笔的直接调用者,登记簿上这一枚摘要 {} {what}({})",
                    party.name(),
                    party.name(),
                    order::hex_of(digest),
                    r.why
                ),
            })
        }
    }
}


fn signed_by_current(party: Party, key: &str, current: &str, digest: &[u8; 32], sig: &[u8]) -> R<()> {
    if sig.len() != 65 {
        return Err(Fault::Precheck { gate: GATE_SIG, why: format!("{}签名是六十五字节,给的是 {}", party.name(), sig.len()) });
    }
    let bad = |f: k2::token::Fault| Fault::Precheck { gate: GATE_SIG, why: format!("{}签名验不过:{}", party.name(), f.token.name()) };
    let cur = current.to_ascii_lowercase();
    match k2::sig::verify_secp256k1(digest, &order::hex_of(sig), &cur) {
        Ok(_) => Ok(()),
        Err(f) if f.token == k2::token::Token::SigSigner && k2::sig::verify_secp256k1(digest, &order::hex_of(sig), &key.to_ascii_lowercase()).is_ok() => {
            Err(Fault::Precheck {
                gate: GATE_CURRENT,
                why: format!("{}签名出自冻结钥 {key},而簿上它的现行钥是 {current}:换过钥的一方要用现行钥签", party.name()),
            })
        }
        Err(f) => Err(bad(f)),
    }
}

pub fn fit(current: &str, digest: &[u8; 32], sig: &[u8]) -> Vec<u8> {
    let by_current = sig.len() == 65 && k2::sig::verify_secp256k1(digest, &order::hex_of(sig), &current.to_ascii_lowercase()).is_ok();
    if by_current { sig.to_vec() } else { Vec::new() }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettleGate {
    Pass,
    SellerNotRegistered,
    Unverified,
}

impl SettleGate {
    pub const fn word(self) -> &'static str {
        match self {
            SettleGate::Pass => "pass",
            SettleGate::SellerNotRegistered => "seller-not-registered",
            SettleGate::Unverified => "unverified",
        }
    }

    pub const fn say(self) -> &'static str {
        match self {
            SettleGate::Pass => "卖家的登记已多端点读到",
            SettleGate::SellerNotRegistered => "卖家尚未登记这一份和解",
            SettleGate::Unverified => "卖家的登记未能多端点核实",
        }
    }
}

pub fn settle_gate<T: Transport>(
    t: &T,
    table: &Table,
    logic: &str,
    seller_key: &str,
    digest: &[u8; 32],
) -> (SettleGate, RegRead) {
    let r = registered(t, table, logic, seller_key, digest);
    let g = match r.state {
        Registration::Held => SettleGate::Pass,
        Registration::NotHeld => SettleGate::SellerNotRegistered,
        Registration::Unverified => SettleGate::Unverified,
    };
    (g, r)
}

pub fn from_logic(emitter: &str, logic: &str) -> bool {
    emitter.eq_ignore_ascii_case(logic)
}

#[cfg(test)]
mod fit_tests {
    use super::fit;

    #[test]
    fn a_slot_keeps_only_the_current_keys_signature() {
        let d = [7u8; 32];
        let (old_sig, old) = eravon_terms::sig::sign_secp256k1(&[0x11; 32], &d).expect("签得出");
        let (_, new) = eravon_terms::sig::sign_secp256k1(&[0x22; 32], &d).expect("签得出");
        let old_bytes = crate::order::bytes_of("sig", &old_sig).expect("hex65");
        assert_eq!(fit(&old, &d, &old_bytes), old_bytes, "现行钥自己的签名照交");
        assert!(fit(&new, &d, &old_bytes).is_empty(), "记录换过钥:冻结钥的签名交空槽");
        assert!(fit(&old, &d, &old_bytes[..64]).is_empty(), "不成形的字节交空槽");
        assert!(fit(&old, &d, &[]).is_empty(), "空槽照空");
    }
}
