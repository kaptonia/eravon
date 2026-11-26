use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_terms as k2;

use crate::chain;
use crate::fault::{Fault, R};
use crate::order::Order;

pub const GATE_SIG: &str = "sig";
pub const GATE_CALLER: &str = "caller";
pub const GATE_FUNDS: &str = "funds";
pub const GATE_ALLOWANCE: &str = "allowance";
pub const GATE_WINDOW: &str = "window";
pub const GATE_PRICE: &str = "price";
pub const GATE_VERIFIER: &str = "verifier";
pub const GATE_BOND: &str = "bond";
pub const GATE_TERMS: &str = "terms";
pub const GATE_REVOKED: &str = "revoked";
pub const GATE_CURRENT: &str = "current";
pub const GATE_STAGE: &str = "stage";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Priced {
    pub premium: k2::u256::U256,
    pub lf: k2::u256::U256,
    pub deposit: k2::u256::U256,
    pub fund_unbound: bool,
}

pub fn priced<T: Transport>(t: &T, table: &Table, o: &Order, quote: &[u8]) -> R<Priced> {
    let refuse = |gate: &'static str, why: String| Fault::Precheck { gate, why };
    let c = chain::verifier_check(t, table, o, quote)?.value;
    if c.terms_fault != 0 {
        return Err(refuse(GATE_TERMS, format!("BadTerms({}):条款守卫第 {} 条不过", c.terms_fault, c.terms_fault)));
    }
    if c.quote_bad {
        return Err(refuse(GATE_TERMS, "BadQuote:报价字节不过 eravon.terms/1 §3.2".into()));
    }
    let q = k2::quote::decode(quote).map_err(|f| refuse(GATE_TERMS, format!("BadQuote:{}", f.token.name())))?;
    if k2::derive::keccak(quote) != o.offer.quote_hash {
        return Err(refuse(GATE_TERMS, "QuoteMismatch:报价字节的 keccak256 不是订单的 quoteHash".into()));
    }
    if !q.token.eq_ignore_ascii_case(&o.offer.token) {
        return Err(refuse(GATE_TERMS, format!("QuoteMismatch:报价的币 {} 不是订单的 {}", q.token, o.offer.token)));
    }
    if stale(&c) {
        return Err(refuse(GATE_PRICE, "RateStale:报价点名的价源陈旧或答不出".into()));
    }
    if c.overflow {
        return Err(refuse(GATE_TERMS, "FeeOverflow:定价折算越过 2^256 − 1".into()));
    }
    let cap = k2::u256::U256::from_be(&c.cap);
    if !cap.is_zero() && cap.cmp_u(&o.offer.amount) == std::cmp::Ordering::Less {
        return Err(refuse(GATE_TERMS, format!("OverCap:报价的上限 {} 低于金额 {}", cap.to_dec(), o.offer.amount.to_dec())));
    }
    let premium = k2::fee::fee(&q, &o.offer.amount).map_err(|f| refuse(GATE_TERMS, format!("FeeOverflow:{}", f.token.name())))?.premium;
    let lf = k2::u256::U256::from_be(&c.lf);
    let (deposit, over) = premium.overflowing_add(&lf);
    if over {
        return Err(refuse(GATE_TERMS, "FeeOverflow:P + lf 越过 2^256 − 1".into()));
    }
    if let Some(name) = c.stage_fault {
        return Err(refuse(GATE_STAGE, format!("{name}:订单的前段 {} 不让这一段开案", o.prev)));
    }
    Ok(Priced { premium, lf, deposit, fund_unbound: c.fund_unbound })
}

fn stale(c: &chain::Checked) -> bool {
    c.stale
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quoted {
    pub premium: k2::u256::U256,
    pub lf: k2::u256::U256,
    pub deposit: k2::u256::U256,
    pub rate: Option<Rate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rate {
    pub price_token: k2::u256::U256,
    pub price_unit: k2::u256::U256,
    pub rate_at: u64,
}

pub fn quoted<T: Transport>(t: &T, table: &Table, offer: &k2::derive::Offer, quote: &[u8]) -> R<Quoted> {
    let refuse = |gate: &'static str, why: String| Fault::Precheck { gate, why };
    let q = k2::quote::decode(quote).map_err(|f| refuse(GATE_TERMS, format!("BadQuote:{}", f.token.name())))?;
    let f = k2::fee::fee(&q, &offer.amount).map_err(|f| refuse(GATE_TERMS, format!("FeeOverflow:{}", f.token.name())))?;
    let (lf, rate) = match f.lf {
        Some(lf) => (lf, None),
        None => {
            let zero = format!("0x{}", "0".repeat(40));
            let o = Order { offer: offer.clone(), buyer_key: zero.clone(), buyer_payout: zero, open_by: 0, prev: crate::order::NO_PREV.to_string() };
            let c = chain::verifier_check(t, table, &o, quote)?.value;
            if c.quote_bad {
                return Err(refuse(GATE_TERMS, "BadQuote:报价字节不过 eravon.terms/1 §3.2".into()));
            }
            if c.stale {
                return Err(refuse(GATE_PRICE, "RateStale:报价点名的价源陈旧或答不出".into()));
            }
            if c.overflow {
                return Err(refuse(GATE_TERMS, "FeeOverflow:定价折算越过 2^256 − 1".into()));
            }
            let rate = Rate { price_token: k2::u256::U256::from_be(&c.price_token), price_unit: k2::u256::U256::from_be(&c.price_unit), rate_at: c.rate_at };
            (k2::u256::U256::from_be(&c.lf), Some(rate))
        }
    };
    let (deposit, over) = f.premium.overflowing_add(&lf);
    if over {
        return Err(refuse(GATE_TERMS, "FeeOverflow:P + lf 越过 2^256 − 1".into()));
    }
    Ok(Quoted { premium: f.premium, lf, deposit, rate })
}

pub fn deposit<T: Transport>(t: &T, table: &Table, o: &Order, logic: &str, p: &Priced) -> R<()> {
    let (key, fund) = (&o.offer.seller_key, &o.offer.seller_fund);
    let need = &p.deposit;
    if p.fund_unbound {
        return Err(Fault::Precheck {
            gate: GATE_BOND,
            why: format!("FundUnbound:押金地址 {fund} 不是卖家钥,也没有经 acceptFunding 绑到 {key}"),
        });
    }
    if need.is_zero() {
        return Ok(());
    }
    let d = need.to_be();
    funds(t, table, &o.offer.token, fund, &d)?;
    allowed(t, table, &o.offer.token, fund, logic, &d)?;
    Ok(())
}

pub fn not_revoked<T: Transport>(t: &T, table: &Table, o: &Order, logic: &str) -> R<()> {
    if chain::revoked(t, table, logic, &o.offer.seller_key, &o.offer.terms_hash)?.value {
        return Err(Fault::Precheck { gate: GATE_REVOKED, why: format!("卖家 {} 已撤下这份条款(revoke)", o.offer.seller_key) });
    }
    Ok(())
}

pub fn verifier_there<T: Transport>(t: &T, table: &Table) -> R<(String, String)> {
    let r = chain::verifier_present(t, table)?;
    let (present, got, want) = r.value;
    if !present {
        return Err(Fault::Precheck {
            gate: GATE_VERIFIER,
            why: format!(
                "链上 {} 上没有验证器那一份码(读到 {got},要的是 {want});先经公共部署器按盐 {} 部署它,或把那一件并进同一笔",
                eravon_literals::constants::VERIFIER,
                eravon_literals::literals::SALT_VERIFIER
            ),
        });
    }
    Ok((got, want))
}

pub fn signatures(
    o: &Order,
    chain_id: u64,
    logic: &str,
    sig_seller_hex: &str,
    sig_buyer_hex: &str,
) -> R<()> {
    let d = o.derive(chain_id, logic)?;
    let seller_digest = o.offer.offer_digest(chain_id, logic);
    k2::sig::verify_secp256k1(&seller_digest, sig_seller_hex, &o.offer.seller_key).map_err(
        |f| Fault::Precheck {
            gate: GATE_SIG,
            why: format!("卖家签名验不过:{}", f.token.name()),
        },
    )?;
    k2::sig::verify_secp256k1(&d.order_digest, sig_buyer_hex, &o.buyer_key).map_err(|f| {
        Fault::Precheck { gate: GATE_SIG, why: format!("买家签名验不过:{}", f.token.name()) }
    })?;
    Ok(())
}

pub fn funds<T: Transport>(
    t: &T,
    table: &Table,
    token: &str,
    who: &str,
    need: &[u8; 32],
) -> R<[u8; 32]> {
    let got = chain::erc20_balance(t, table, token, who)?.value;
    enough(&got, need, who, &|v| word(t, table, token, v))?;
    Ok(got)
}

fn enough(got: &[u8; 32], need: &[u8; 32], who: &str, word: &dyn Fn(&[u8; 32]) -> String) -> R<()> {
    if less_than(got, need) {
        return Err(Fault::Precheck { gate: GATE_FUNDS, why: format!("{who} 上有 {},这一笔要 {}", word(got), word(need)) });
    }
    Ok(())
}


pub fn funded<T: Transport>(
    t: &T,
    table: &Table,
    o: &Order,
    instance: &str,
) -> R<[u8; 32]> {
    funds(t, table, &o.offer.token, instance, &o.offer.amount.to_be())
}

pub fn word<T: Transport>(t: &T, table: &Table, token: &str, v: &[u8; 32]) -> String {
    use eravon_vault::tokens;
    let rows = tokens::current();
    let d = tokens::decimals_of(&rows, tokens::chain(), token, None).or_else(|| chain::side_decimals(t, table, token).map(|d| d as u8));
    tokens::amount(&k2::u256::U256::from_be(v).to_dec(), d, &tokens::name(&rows, tokens::chain(), token, &tokens::ZH), tokens::Form::Full, &tokens::ZH)
}

pub fn allowed<T: Transport>(
    t: &T,
    table: &Table,
    token: &str,
    owner: &str,
    spender: &str,
    need: &[u8; 32],
) -> R<[u8; 32]> {
    let got = chain::erc20_allowance(t, table, token, owner, spender)?.value;
    if less_than(&got, need) {
        return Err(Fault::Precheck {
            gate: GATE_ALLOWANCE,
            why: format!("授权了 {},这一步要 {}", word(t, table, token, &got), word(t, table, token, need)),
        });
    }
    Ok(got)
}

pub fn in_window(window_end: u64, now: u64) -> R<()> {
    let w = a2::clock::window(window_end, now);
    if w.action {
        Ok(())
    } else {
        Err(Fault::Precheck {
            gate: GATE_WINDOW,
            why: format!("这一口钟到 {window_end} 为止,现在是 {now}"),
        })
    }
}


pub const PHASE_CLAIMED: u64 = 2;
pub const PHASE_DISPUTED: u64 = 3;
pub const PHASE_FIRST_TERMINAL: u64 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeatExit {
    Open,
    ClockOnly,
}

impl SeatExit {
    pub fn word(self) -> &'static str {
        match self {
            SeatExit::Open => "open",
            SeatExit::ClockOnly => "clock",
        }
    }
}

pub fn seat_exit(accepted: bool, window_end: u64, now: u64) -> SeatExit {
    if accepted && !a2::clock::window(window_end, now).action {
        SeatExit::ClockOnly
    } else {
        SeatExit::Open
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Close {
    Confirm,
    Refund,
    Settle,
    Rule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seat {
    pub state: u64,
    pub accepted: bool,
    pub window_end: u64,
}

pub fn premium_to_judge(close: Close, phase: u64, seat: Option<Seat>, now: u64) -> bool {
    let Some(s) = seat else { return false };
    let inside = in_seat_window(s.window_end, now);
    match close {
        Close::Rule => true,
        Close::Confirm => s.state == 0,
        Close::Refund => s.state == 0 && phase == PHASE_CLAIMED,
        Close::Settle => s.accepted && ((s.state == 0 && (inside || phase == PHASE_CLAIMED)) || (s.state == 1 && inside)),
    }
}

fn in_seat_window(window_end: u64, now: u64) -> bool {
    a2::clock::window(window_end, now).action
}


pub fn chain_now<T: Transport>(t: &T, table: &Table) -> R<u64> {
    Ok(a2::clock::now(t, table)?.0.timestamp)
}

fn less_than(a: &[u8; 32], b: &[u8; 32]) -> bool {
    for i in 0..32 {
        if a[i] != b[i] {
            return a[i] < b[i];
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_endian_words_compare_whole() {
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        a[31] = 1;
        b[31] = 2;
        assert!(less_than(&a, &b));
        assert!(!less_than(&b, &a));
        assert!(!less_than(&a, &a));
        let mut c = [0u8; 32];
        c[0] = 1;
        assert!(less_than(&b, &c));
    }

    #[test]
    fn past_the_window_the_seat_has_only_the_clock_and_a_lapsed_t3_pays_no_fee() {
        assert_eq!(seat_exit(true, 100, 100), SeatExit::Open);
        assert_eq!(seat_exit(true, 100, 101), SeatExit::ClockOnly);
        assert_eq!(seat_exit(false, 100, 101), SeatExit::Open);
    }

    #[test]
    fn p_goes_to_the_judge_as_each_close_hands_it() {
        let s = |state, accepted| Some(Seat { state, accepted, window_end: 100 });
        assert!(premium_to_judge(Close::Rule, PHASE_DISPUTED, s(1, true), 101));
        assert!(premium_to_judge(Close::Confirm, PHASE_CLAIMED, s(0, false), 101));
        assert!(!premium_to_judge(Close::Confirm, PHASE_CLAIMED, s(4, true), 50));
        assert!(premium_to_judge(Close::Refund, PHASE_CLAIMED, s(0, true), 50));
        assert!(!premium_to_judge(Close::Refund, 1, s(0, true), 50), "未申报交付的退款,P 回卖家");
        assert!(premium_to_judge(Close::Settle, PHASE_DISPUTED, s(1, true), 100), "窗的最后一秒照得");
        assert!(!premium_to_judge(Close::Settle, PHASE_DISPUTED, s(1, true), 101));
        assert!(premium_to_judge(Close::Settle, PHASE_CLAIMED, s(0, true), 101), "已申报交付而席在:过窗也照得");
        assert!(!premium_to_judge(Close::Settle, PHASE_CLAIMED, s(0, false), 50), "未接单不得");
        assert!(!premium_to_judge(Close::Confirm, PHASE_CLAIMED, None, 50), "未抽签不得");
    }

    #[test]
    fn the_window_gate_gives_the_last_second_to_the_action() {
        assert!(in_window(100, 100).is_ok());
        assert!(in_window(100, 99).is_ok());
        let f = in_window(100, 101).expect_err("过了就该拒");
        assert_eq!(f.precheck_gate(), Some(GATE_WINDOW));
    }
}

pub fn caller(o: &Order, sender: crate::consent::Sender<'_>, seller_current: &str, buyer_current: &str) -> R<()> {
    if sender.via_multicall {
        return Err(Fault::Precheck { gate: GATE_CALLER, why: "init 经 Multicall3 合并:合约看见的调用者是 Multicall3,不是任何一方".into() });
    }
    let is = |k: &str| sender.from.is_some_and(|s| s.eq_ignore_ascii_case(k));
    if is(seller_current) {
        return Ok(());
    }
    if !o.offer.buyer_may_open {
        return Err(Fault::Precheck {
            gate: GATE_CALLER,
            why: format!("NotSeller:buyerMayOpen 为假,只有卖家钥 {} 的现行钥 {seller_current} 直发 init", o.offer.seller_key),
        });
    }
    if !is(buyer_current) {
        return Err(Fault::Precheck {
            gate: GATE_CALLER,
            why: format!("NotParty:init 只由卖家钥的现行钥 {seller_current} 或买家钥 {} 的现行钥 {buyer_current} 直发", o.buyer_key),
        });
    }
    Ok(())
}

pub fn slots(
    o: &Order,
    chain_id: u64,
    logic: &str,
    sig_seller: &[u8],
    sig_buyer: &[u8],
    current: &mut dyn FnMut(&str) -> R<String>,
) -> R<(Vec<u8>, Vec<u8>)> {
    let d = o.derive(chain_id, logic)?;
    let s = crate::consent::fit(&current(&o.offer.seller_key)?, &o.offer.offer_digest(chain_id, logic), sig_seller);
    let b = crate::consent::fit(&current(&o.buyer_key)?, &d.order_digest, sig_buyer);
    Ok((s, b))
}

pub fn settle_slots<T: Transport>(
    t: &T,
    table: &Table,
    logic: &str,
    instance: &str,
    digest: &[u8; 32],
    sig_buyer: &[u8],
    sig_seller: &[u8],
    from: &str,
) -> R<(Vec<u8>, Vec<u8>)> {
    use crate::consent::{Party, Sender, fit, registered, slot};
    let _ = logic;
    let st = chain::stage(t, table, instance)?.value;
    let mut reg = |who: &str, d: &[u8; 32]| registered(t, table, &st.logic, who, d);
    let sender = Sender { from: Some(from), via_multicall: false };
    let mut out = Vec::with_capacity(2);
    for (party, key, sig) in [(Party::Buyer, &st.buyer_key, sig_buyer), (Party::Seller, &st.seller_key, sig_seller)] {
        let now = chain::acting_key(t, table, key, st.past)?;
        let s = fit(&now, digest, sig);
        slot(party, key, &now, digest, &s, sender, &mut reg)?;
        out.push(s);
    }
    let seller = out.pop().unwrap_or_default();
    let buyer = out.pop().unwrap_or_default();
    Ok((buyer, seller))
}

#[allow(clippy::too_many_arguments)]
pub fn consents(
    o: &Order,
    chain_id: u64,
    logic: &str,
    sig_seller: &[u8],
    sig_buyer: &[u8],
    sender: crate::consent::Sender<'_>,
    current: &mut dyn FnMut(&str) -> R<String>,
    registry: &mut dyn FnMut(&str, &[u8; 32]) -> crate::consent::RegRead,
) -> R<(crate::consent::Form, crate::consent::Form)> {
    use crate::consent::{Party, slot};
    let seller_now = current(&o.offer.seller_key)?;
    let buyer_now = current(&o.buyer_key)?;
    caller(o, sender, &seller_now, &buyer_now)?;
    let d = o.derive(chain_id, logic)?;
    let seller_digest = o.offer.offer_digest(chain_id, logic);
    let s = slot(Party::Seller, &o.offer.seller_key, &seller_now, &seller_digest, sig_seller, sender, registry)?;
    let b = slot(Party::Buyer, &o.buyer_key, &buyer_now, &d.order_digest, sig_buyer, sender, registry)?;
    Ok((s, b))
}
