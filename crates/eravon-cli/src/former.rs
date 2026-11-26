use std::cmp::Ordering;

use eravon_standard as a1;
use eravon_terms::u256::U256;

use crate::fault::{Fault, R};
use crate::send::Opts;
use crate::shared::Wire;

pub const AMOUNT: &str = "E_P7_AMOUNT";
pub const STALE: &str = "E_P7_STALE";

pub fn p7(token: &str, why: impl Into<String>) -> Fault {
    Fault::Down { layer: "P7", token: token.to_string(), why: why.into() }
}

fn p7_of(f: &Fault) -> Option<&str> {
    match f {
        Fault::Down { layer: "P7", token, .. } => Some(token.as_str()),
        _ => None,
    }
}

pub struct Post<'a> {
    pub from: &'a str,
    pub opts: &'a Opts,
    pub call: &'a a1::call::Call,
    pub value: &'a str,
    pub token: &'a str,
    pub amount: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leg {
    pub token: String,
    pub amount: String,
    pub tx: String,
}

#[derive(Clone, Debug)]
pub enum Stop {
    Token(Fault),
    Eth(Fault),
}

#[derive(Clone, Debug, Default)]
pub struct Moved {
    pub moved: Vec<Leg>,
    pub paid: Vec<Leg>,
    pub sent: Vec<String>,
    pub stop: Option<Stop>,
}

pub struct Room {
    pub cap: String,
    pub tip: String,
    pub avail: U256,
}

pub fn eth_room(w: &Wire, gas: u64, have: &U256) -> R<Room> {
    let eravon_chain::gas::Fees { cap, tip, .. } = eravon_chain::gas::suggested_fees(&w.t, &w.table)?;
    let price = U256::from_dec(&cap).unwrap_or(U256::ZERO);
    let wide = price.mul_wide(&U256::from_u64(gas));
    let fee = U256([wide.0[0], wide.0[1], wide.0[2], wide.0[3]]);
    let avail = if have.cmp_u(&fee) == Ordering::Greater { have.overflowing_sub(&fee).0 } else { U256::ZERO };
    Ok(Room { cap, tip, avail })
}

pub fn run(w: &Wire, from: &str, to: &str, tokens: &[String], eth: bool, base: &Opts, post: &mut dyn FnMut(&Post<'_>) -> R<String>, still: &dyn Fn() -> bool) -> Moved {
    let mut m = Moved::default();
    for t in tokens {
        match token(w, from, t, to, base, &mut m, post, still) {
            Ok(leg) => m.moved.push(leg),
            Err(e) => {
                m.stop = Some(Stop::Token(e));
                return m;
            }
        }
    }
    if eth {
        match ether(w, from, to, base, &mut m, post, still) {
            Ok(leg) => m.moved.push(leg),
            Err(e) if !m.moved.is_empty() && p7_of(&e) == Some(AMOUNT) => {}
            Err(e) => m.stop = Some(Stop::Eth(e)),
        }
    }
    m
}

fn stale() -> Fault {
    p7(STALE, "情况已变,这一手没有执行")
}

fn accept(a: &str) -> R<String> {
    eravon_vault::payout::accept(a.trim()).map_err(crate::home::down)
}

#[allow(clippy::too_many_arguments)]
fn token(w: &Wire, from: &str, token: &str, to: &str, base: &Opts, m: &mut Moved, post: &mut dyn FnMut(&Post<'_>) -> R<String>, still: &dyn Fn() -> bool) -> R<Leg> {
    let token = accept(token)?;
    let dest = accept(to)?;
    let have = U256::from_be(&a1::chain::erc20_balance(&w.t, &w.table, &token, from)?.value);
    if have.is_zero() {
        return Err(p7(AMOUNT, "余额为零,没有可转的"));
    }
    let call = a1::call::erc20_transfer(&token, &dest, &have.to_be())?;
    let (terms, most) = crate::send::pinned(base, w, from, &call)?;
    let need = U256::from_dec(&most).ok_or_else(|| p7(AMOUNT, format!("最多要花的费 {most} 读不成整数")))?;
    let eth = U256::from_dec(&eravon_chain::read::balance(&w.t, &w.table, from, "latest")?.0).unwrap_or(U256::ZERO);
    if need.cmp_u(&eth) == Ordering::Greater {
        let lack = need.overflowing_sub(&eth).0.to_dec();
        if !still() {
            return Err(stale());
        }
        let fee = a1::call::Call { to: from.to_ascii_lowercase(), data: Vec::new() };
        let o = Opts { wait: base.wait, ..Opts::default() };
        let tx = post(&Post { from: to, opts: &o, call: &fee, value: &lack, token: "", amount: &lack })?;
        m.sent.push(tx.clone());
        m.paid.push(Leg { token: String::new(), amount: lack, tx });
    }
    if !still() {
        return Err(stale());
    }
    let amount = have.to_dec();
    let tx = post(&Post { from, opts: &terms, call: &call, value: "0", token: &token, amount: &amount })?;
    m.sent.push(tx.clone());
    Ok(Leg { token, amount, tx })
}

fn ether(w: &Wire, from: &str, to: &str, base: &Opts, m: &mut Moved, post: &mut dyn FnMut(&Post<'_>) -> R<String>, still: &dyn Fn() -> bool) -> R<Leg> {
    let dest = accept(to)?;
    let have = U256::from_dec(&eravon_chain::read::balance(&w.t, &w.table, from, "latest")?.0).unwrap_or(U256::ZERO);
    if have.is_zero() {
        return Err(p7(AMOUNT, "余额为零,没有可转的 ETH"));
    }
    let call = a1::call::Call { to: dest, data: Vec::new() };
    let gas = crate::send::limit(w, from, &call, &have.to_dec())?;
    let Room { cap, tip, avail } = eth_room(w, gas, &have)?;
    if avail.is_zero() {
        return Err(p7(AMOUNT, "留足手续费之后没有可转的 ETH"));
    }
    let o = Opts { gas_limit: Some(gas), fees: Some((cap, tip)), wait: base.wait, nonce: None };
    if !still() {
        return Err(stale());
    }
    let amount = avail.to_dec();
    let tx = post(&Post { from, opts: &o, call: &call, value: &amount, token: "", amount: &amount })?;
    m.sent.push(tx.clone());
    Ok(Leg { token: String::new(), amount, tx })
}
