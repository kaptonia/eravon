use eravon_cli::shared::Wire;
use eravon_standard as a1;
use eravon_terms::u256::U256;

use crate::fault::{Fault, R};
use crate::tx::{self, Keep, Kind, Tag};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Amount {
    All,
    Of(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sent {
    pub tx: String,
    pub amount: String,
}

pub fn units(s: &str, decimals: u8) -> R<U256> {
    let v = amount_of(s, decimals)?;
    eravon_cli::shared::send_amount(&v, s).map_err(|(n, d)| Fault::new(n, d))?;
    Ok(v)
}

pub fn amount_of(s: &str, decimals: u8) -> R<U256> {
    let raw = eravon_literals::units::units_of(s, decimals).map_err(|name| Fault::new(name, format!("金额 {s:?} 折不成最小单位(至多 {decimals} 位小数)")))?;
    U256::from_dec(&raw).ok_or_else(|| Fault::new(eravon_literals::units::AMOUNT_TOO_LARGE, format!("金额 {s:?} 超出上限")))
}

fn decimals_for(w: &Wire, token: &str) -> R<u8> {
    let rows = eravon_vault::tokens::current();
    a1::chain::amount_decimals(&w.t, &w.table, &rows, token).ok_or_else(|| Fault::new("E_P7_DECIMALS", format!("读不到 {token} 的位数,金额折不成最小单位")))
}

fn all_of(have: U256, _decimals: impl FnOnce() -> Option<u8>) -> U256 {
    have
}


fn eth_word(wei: &U256) -> String {
    eravon_vault::tokens::amount(&wei.to_dec(), Some(eravon_literals::constants::NATIVE_DECIMALS), eravon_literals::constants::NATIVE_SYMBOL, eravon_vault::tokens::Form::Full, &eravon_vault::tokens::ZH)
}

fn gas_for(w: &Wire, me: &str, call: &a1::call::Call, value: &U256) -> R<u64> {
    Ok(eravon_cli::send::limit(w, me, call, &value.to_dec())?)
}


fn within(want: &U256, avail: &U256, who: &str) -> R<()> {
    let wei = |x: &U256| {
        let b = x.to_be();
        let mut low = [0u8; 16];
        low.copy_from_slice(&b[16..]);
        if b[..16].iter().any(|&c| c != 0) { u128::MAX } else { u128::from_be_bytes(low) }
    };
    eravon_send::batch::funds_check(wei(want), wei(avail)).map_err(|_| Fault::new("E_P7_OVER", format!("超过可用余额:{who} 可用 {},要转 {}", eth_word(avail), eth_word(want))))
}


pub fn plan(token: Option<&str>, amount: &Amount, to: &str) -> R<()> {
    let typed = match amount {
        Amount::All => None,
        Amount::Of(x) => Some(x.as_str()),
    };
    eravon_cli::shared::send_plan(token, typed, to).map(|_| ()).map_err(|(n, d)| Fault::new(n, d))
}

struct Room {
    gas: u64,
    price_dec: String,
    tip_dec: String,
    avail: U256,
}

fn eth_room(w: &Wire, me: &str, call: &a1::call::Call, have: &U256, probe: &U256) -> R<Room> {
    let gas = gas_for(w, me, call, probe)?;
    let eravon_cli::former::Room { cap, tip, avail } = eravon_cli::former::eth_room(w, gas, have).map_err(crate::fault::lift)?;
    Ok(Room { gas, price_dec: cap, tip_dec: tip, avail })
}

pub fn sendable(w: &Wire, me: &str, token: Option<&str>, to: &str) -> R<U256> {
    crate::mark::mark(crate::mark::FEATURE);
    let to = if to.trim().is_empty() { me.to_string() } else { eravon_vault::payout::accept(to.trim())? };
    match token {
        Some(t) => {
            let token = eravon_vault::payout::accept(t.trim())?;
            let have = U256::from_be(&a1::chain::erc20_balance(&w.t, &w.table, &token, me)?.value);
            Ok(all_of(have, || decimals_for(w, &token).ok()))
        }
        None => {
            let have = U256::from_dec(&eravon_chain::read::balance(&w.t, &w.table, me, "latest")?.0).unwrap_or(U256::ZERO);
            if have.is_zero() {
                return Ok(U256::ZERO);
            }
            let call = a1::call::Call { to, data: Vec::new() };
            Ok(eth_room(w, me, &call, &have, &have)?.avail)
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn transfer(k: &Keep<'_>, w: &Wire, s: &eravon_vault::session::Session, me: &str, token: Option<&str>, amount: &Amount, to: &str, still: &dyn Fn() -> bool) -> R<Sent> {
    crate::mark::mark(crate::mark::FEATURE);
    eravon_vault::session::need_open(s)?;
    let to = eravon_vault::payout::accept(to.trim())?;
    let stale = || Fault::new("E_P7_STALE", "情况已变,这一手没有执行");
    let key = || s.own_key(me, me).map_err(|e| eravon_cli::fault::Fault::Arg(format!("{}:{}", e.name(), e.detail())));
    match token {
        Some(t) => {
            let token = eravon_vault::payout::accept(t.trim())?;
            let have = U256::from_be(&a1::chain::erc20_balance(&w.t, &w.table, &token, me)?.value);
            let want = match amount {
                Amount::All => all_of(have, || decimals_for(w, &token).ok()),
                Amount::Of(x) => units(x, decimals_for(w, &token)?)?,
            };
            if want.is_zero() {
                return Err(Fault::new("E_P7_AMOUNT", "余额为零,没有可转的"));
            }
            a1::precheck::funds(&w.t, &w.table, &token, me, &want.to_be())?;
            let call = a1::call::erc20_transfer(&token, &to, &want.to_be())?;
            let tag = Tag::moving(Kind::Send, "", &token, &want.to_dec());
            if !still() {
                return Err(stale());
            }
            let tx = tx::post(k, w, me, &key, &eravon_cli::send::Opts::default(), &call, "0", &tag)?;
            Ok(Sent { tx, amount: want.to_dec() })
        }
        None => {
            let have = U256::from_dec(&eravon_chain::read::balance(&w.t, &w.table, me, "latest")?.0).unwrap_or(U256::ZERO);
            let typed = match amount {
                Amount::All => None,
                Amount::Of(x) => Some(units(x, eravon_literals::constants::NATIVE_DECIMALS)?),
            };
            if typed.is_none() && have.is_zero() {
                return Err(Fault::new("E_P7_AMOUNT", "余额为零,没有可转的 ETH"));
            }
            let call = a1::call::Call { to: to.clone(), data: Vec::new() };
            let probe = typed.filter(|t| t.cmp_u(&have) != std::cmp::Ordering::Greater).unwrap_or(have);
            let Room { gas, price_dec, tip_dec, avail } = eth_room(w, me, &call, &have, &probe)?;
            let want = typed.unwrap_or(avail);
            if want.is_zero() {
                return Err(Fault::new("E_P7_AMOUNT", "留足手续费之后没有可转的 ETH"));
            }
            within(&want, &avail, me)?;
            let o = eravon_cli::send::Opts { gas_limit: Some(gas), fees: Some((price_dec, tip_dec)), wait: None, nonce: None };
            let tag = Tag::moving(Kind::Send, "", "", &want.to_dec());
            if !still() {
                return Err(stale());
            }
            let tx = tx::post(k, w, me, &key, &o, &call, &want.to_dec(), &tag)?;
            Ok(Sent { tx, amount: want.to_dec() })
        }
    }
}

pub fn move_former(k: &Keep<'_>, w: &Wire, s: &eravon_vault::session::Session, from: &str, me: &str, b: &crate::balance::Balances, still: &dyn Fn() -> bool) -> R<eravon_cli::former::Moved> {
    crate::mark::mark(crate::mark::FEATURE);
    eravon_vault::session::need_open(s)?;
    let tokens: Vec<String> = b.coins.iter().filter(|c| c.have.as_ref().is_ok_and(|x| x != "0")).map(|c| c.token.clone()).collect();
    let eth = b.native.as_ref().is_ok_and(|x| x != "0");
    let mut post = |p: &eravon_cli::former::Post<'_>| {
        let who = p.from.to_string();
        let key = move || s.own_key(&who, &who).map_err(|e| eravon_cli::fault::Fault::Arg(format!("{}:{}", e.name(), e.detail())));
        tx::post(k, w, p.from, &key, p.opts, p.call, p.value, &Tag::moving(Kind::Send, "", p.token, p.amount)).map_err(|f| eravon_cli::former::p7(&f.token, f.detail))
    };
    Ok(eravon_cli::former::run(w, from, me, &tokens, eth, &eravon_cli::send::Opts::default(), &mut post, still))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_never_round() {
        let raw = |s: &str, d: u8| units(s, d).map(|v| v.to_dec()).map_err(|f| f.token);
        assert_eq!(raw("1.5", 6), Ok("1500000".into()));
        assert_eq!(raw("0.000001", 6), Ok("1".into()));
        assert_eq!(raw("12", 0), Ok("12".into()));
        assert_eq!(raw(".5", 1), Ok("5".into()));
        assert_eq!(raw("1,234.5", 6), Ok("1234500000".into()));
        assert_eq!(raw("０．５", 6), Ok("500000".into()));
        assert_eq!(raw("0.0000001", 6), Err("amount-too-precise".into()));
        assert_eq!(raw("0", 6), Err("E_P7_AMOUNT".into()));
        assert_eq!(amount_of("0", 6).map(|v| v.to_dec()).map_err(|f| f.token), Ok("0".into()), "an allowance taken back to zero converts");
        assert_eq!(raw("", 6), Err("amount-empty".into()));
        assert_eq!(raw("-1", 6), Err("amount-form".into()));
    }
}
