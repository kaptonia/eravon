use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_literals::constants as k7;

use crate::abi::{self, Ty, Val};
use crate::call;
use crate::chain::Read;
use crate::fault::{Fault, R};
use crate::order;
use crate::sol;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ask {
    pub to: String,
    pub data: Vec<u8>,
}

pub fn eth_balance(who: &str) -> R<Ask> {
    let to = k7::MULTICALL3.get().ok_or_else(|| Fault::Form("MULTICALL3 这一槽还没有值".into()))?;
    Ok(Ask { to: (*to).to_string(), data: call::with(sol::MULTICALL3_ETH_BALANCE, &[Val::Addr(order::addr_of("谁", who)?)])? })
}

pub fn timestamp() -> R<Ask> {
    let to = k7::MULTICALL3.get().ok_or_else(|| Fault::Form("MULTICALL3 这一槽还没有值".into()))?;
    Ok(Ask { to: (*to).to_string(), data: call::view(sol::MULTICALL3_TIMESTAMP)? })
}

pub fn erc20_balance(token: &str, who: &str) -> R<Ask> {
    order::addr_of("币", token)?;
    Ok(Ask { to: token.to_string(), data: call::erc20_balance_of(&order::addr_of("谁", who)?)? })
}

pub fn erc20_decimals(token: &str) -> R<Ask> {
    order::addr_of("币", token)?;
    Ok(Ask { to: token.to_string(), data: call::view(sol::ERC20_DECIMALS)? })
}

pub fn status(instance: &str) -> R<Ask> {
    order::addr_of("实例", instance)?;
    Ok(Ask { to: instance.to_string(), data: call::view(sol::STATUS)? })
}

pub fn read<T: Transport>(t: &T, table: &Table, asks: &[Ask], at: &str) -> R<Option<Read<Vec<Option<Vec<u8>>>>>> {
    crate::mark::mark(crate::mark::FEATURE);
    let Some(to) = k7::MULTICALL3.get() else { return Ok(None) };
    if asks.is_empty() {
        return Ok(Some(Read { value: Vec::new(), agreement: "agreed", endpoints: table.len() }));
    }
    if a2::recall::current().is_some() {
        return kept(t, table, asks).map(Some);
    }
    let mut value = Vec::with_capacity(asks.len());
    let (mut agreement, mut endpoints) = ("agreed", table.len());
    for part in asks.chunks(CHUNK) {
        let Some(r) = one(t, table, to, part, at)? else { return Ok(None) };
        if r.agreement == "single" {
            agreement = "single";
        }
        endpoints = endpoints.min(r.endpoints);
        value.extend(r.value);
    }
    Ok(Some(Read { value, agreement, endpoints }))
}

pub const CHUNK: usize = 100;

fn kept<T: Transport>(t: &T, table: &Table, asks: &[Ask]) -> R<Read<Vec<Option<Vec<u8>>>>> {
    let mut value = Vec::with_capacity(asks.len());
    let mut agreement = "agreed";
    let mut endpoints = table.len();
    let mut refused: Option<a2::fault::Fault> = None;
    for a in asks {
        match a2::read::call(t, table, &a.to, &order::hex_of(&a.data), a2::seam::TAG_LATEST) {
            Ok((hex, r)) => {
                if r.agreement.word() == "single" {
                    agreement = "single";
                }
                endpoints = endpoints.min(r.endpoints);
                value.push(Some(order::bytes_of("答", &hex)?));
            }
            Err(e) if a2::fault::reverted(&e) => value.push(None),
            Err(e) => {
                refused.get_or_insert(e);
                value.push(None);
            }
        }
    }
    match refused {
        Some(e) => Err(e.into()),
        None => Ok(Read { value, agreement, endpoints }),
    }
}

fn one<T: Transport>(t: &T, table: &Table, to: &str, asks: &[Ask], at: &str) -> R<Option<Read<Vec<Option<Vec<u8>>>>>> {
    let data = aggregate(asks)?;
    let (hex, r) = a2::read::call(t, table, to, &order::hex_of(&data), at)?;
    let out = order::bytes_of("答", &hex)?;
    if out.is_empty() {
        return Ok(None);
    }
    let vals = abi::decode(&[Ty::Arr(Box::new(Ty::Tuple(vec![Ty::Bool, Ty::Bytes])))], &out)?;
    let Some(Val::Arr(rows)) = vals.first() else {
        return Err(Fault::Abi("aggregate3 的答该是一张表".into()));
    };
    if rows.len() != asks.len() {
        return Err(Fault::Abi(format!("aggregate3 问了 {} 件,答了 {} 件", asks.len(), rows.len())));
    }
    let mut got = Vec::with_capacity(rows.len());
    for row in rows {
        let cols = row.items()?;
        let ok = cols.first().ok_or_else(|| Fault::Abi("aggregate3 的一行缺成败栏".into()))?.boolean()?;
        let bytes = cols.get(1).ok_or_else(|| Fault::Abi("aggregate3 的一行缺答栏".into()))?.bytes()?.to_vec();
        got.push(ok.then_some(bytes));
    }
    Ok(Some(Read { value: got, agreement: r.agreement.word(), endpoints: r.endpoints }))
}

fn aggregate(asks: &[Ask]) -> R<Vec<u8>> {
    let mut items = Vec::with_capacity(asks.len());
    for a in asks {
        items.push(Val::Tuple(vec![Val::Addr(order::addr_of("收件地址", &a.to)?), Val::Bool(true), Val::Bytes(a.data.clone())]));
    }
    call::with(sol::AGGREGATE3, &[Val::Arr(items)])
}

pub fn ahead<T: Transport>(t: &T, table: &Table, sets: &[Vec<Ask>], at: &str) -> bool {
    let Some(to) = k7::MULTICALL3.get() else { return false };
    let calls: Vec<(String, String)> = sets.iter().filter(|s| !s.is_empty() && s.len() <= CHUNK).filter_map(|s| aggregate(s).ok()).map(|d| ((*to).to_string(), order::hex_of(&d))).collect();
    a2::read::ahead_calls(t, table, None, &calls, at);
    true
}



pub fn word(b: &[u8]) -> Option<[u8; 32]> {
    b.get(..32).and_then(|w| w.try_into().ok())
}

pub fn decimals(b: &[u8]) -> Option<u8> {
    let w = word(b)?;
    if w[..31].iter().any(|x| *x != 0) {
        return None;
    }
    Some(w[31]).filter(|d| *d <= eravon_literals::units::DECIMALS_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_decimals_answer_counts_only_from_zero_to_seventy_seven() {
        let w = |n: u64| {
            let mut b = vec![0u8; 32];
            b[24..].copy_from_slice(&n.to_be_bytes());
            b
        };
        assert_eq!(decimals(&w(6)), Some(6));
        assert_eq!(decimals(&w(77)), Some(77));
        assert_eq!(decimals(&w(78)), None);
        assert_eq!(decimals(&w(1 << 40)), None);
        assert_eq!(decimals(&[0u8; 31]), None);
    }

    #[test]
    fn every_inner_call_is_allowed_to_fail_on_its_own() {
        let a = erc20_balance("0x00000000000000000000000000000000000000a1", "0x00000000000000000000000000000000000000b2").unwrap();
        assert_eq!(&a.data[..4], &sol::ERC20_BALANCE_OF.selector());
        let e = eth_balance("0x00000000000000000000000000000000000000b2").unwrap();
        assert_eq!(&e.data[..4], &sol::MULTICALL3_ETH_BALANCE.selector());
        assert!(e.to.eq_ignore_ascii_case(k7::MULTICALL3.get().unwrap()));
    }
}
