use eravon_standard as a1;
use a1::chain::status_col as col;
use eravon_terms::u256::U256;
use eravon_watch::order::Order;
use eravon_watch::send::Post;

use crate::seat_col;
use crate::{Why, Wire, a1w, big, case_of, small, status, why};

#[derive(Clone, Debug)]
pub struct Card {
    pub amount: U256,
    pub accepted: bool,
    pub earns: bool,
    pub judge_fee: U256,
    pub to_buyer: U256,
    pub bond_back: U256,
    pub lf_back: U256,
    pub stray: U256,
    pub phase: u64,
}

pub fn refundable(phase: u64, seat_state: Option<u64>, window_end: u64, now: u64) -> bool {
    crate::open_phase(phase) && !(seat_state == Some(seat_col::STATE_RULING) && now <= window_end)
}


pub const SPLIT_STATE: u8 = 5;

#[allow(clippy::too_many_arguments)]
pub fn split(w: &Wire<'_>, amount: &U256, buyer_bps: u16, premium: &U256, deposit: &U256, lf_held: &U256, p_to_judge: bool, lf_to_judge: bool, bond_to: u8) -> Result<[U256; 5], Why> {
    use a1::abi::Val;
    let f = a1::sol::Fun { name: "split", sig: a1::sol::src_abi::ERAVON_VERIFIER_SPLIT };
    let args = [
        Val::U256(amount.to_be()),
        Val::u64(u64::from(buyer_bps)),
        Val::U256(premium.to_be()),
        Val::U256(deposit.to_be()),
        Val::U256(lf_held.to_be()),
        Val::Bool(p_to_judge),
        Val::Bool(lf_to_judge),
        Val::u64(u64::from(bond_to)),
        Val::u64(u64::from(SPLIT_STATE)),
    ];
    let open = f.sig.find('(').ok_or_else(|| why("E_A1_ABI", "split 的签名不成形"))?;
    a1::abi::fits(&f.sig[open..], &args).map_err(a1w)?;
    let mut data = f.selector().to_vec();
    data.extend(a1::abi::encode(&args));
    let tys = a1::abi::tuple_types(a1::sol::src_abi::ERAVON_VERIFIER_SPLIT_RETURNS).map_err(a1w)?;
    let v = a1::chain::call_read(w.t, w.table, eravon_literals::constants::VERIFIER, &data, &tys).map_err(a1w)?.value;
    let at = |i: usize| big(&v, i);
    Ok([at(0)?, at(1)?, at(2)?, at(3)?, at(4)?])
}

pub fn card(data: &str, me: &str, w: &Wire<'_>, o: &Order) -> Result<Card, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let _ = (data, me);
    let st = status(w, &o.instance)?;
    let phase = small(&st, col::PHASE)?;
    if !crate::open_phase(phase) {
        return Err(why("E_DISPUTE_CLOSED", format!("当前阶段不能退款(phase {phase})")));
    }
    let seats = small(&st, col::SEATS)?;
    let seat = if seats == 0 { None } else { Some(a1::chain::seat_of(w.t, w.table, &o.instance, &case_of(o)?, seats - 1).map_err(a1w)?.value) };
    let now = a1::precheck::chain_now(w.t, w.table).map_err(a1w)?;
    let (state, window_end, accepted) = match &seat {
        Some(s) => (Some(small(s, seat_col::STATE)?), small(s, seat_col::WINDOW_END)?, small(s, seat_col::ACCEPTED)? != 0),
        None => (None, 0, false),
    };
    if !refundable(phase, state, window_end, now) {
        return Err(why("E_DISPUTE_T3", "争议正在判官的裁决期内:合约此时不收退款;和解或等裁决"));
    }
    let seat = seat.ok_or_else(|| why("E_DISPUTE_READ", "案已开却没有席位"))?;
    let amount = big(&seat, seat_col::AMOUNT)?;
    let token = a1::order::hex_of(&seat.get(seat_col::TOKEN).ok_or_else(|| why("E_DISPUTE_READ", "seatOf 缺 token"))?.addr().map_err(a1w)?);
    let earns = phase == 2 && state == Some(seat_col::STATE_SEATED);
    let (premium, deposit, lf) = (big(&st, col::PREMIUM)?, big(&st, col::DEPOSIT)?, big(&st, col::LF_HELD)?);
    let [buyer_paid, _seller_a, judge_paid, to_fund, _] = split(w, &amount, 10_000, &premium, &deposit, &lf, earns, false, 0)?;
    let stray = eravon_watch::poll::stray(w, &o.instance, &token, &st, &amount)?;
    Ok(Card {
        amount,
        accepted,
        earns,
        judge_fee: judge_paid,
        to_buyer: buyer_paid.overflowing_add(&stray).0,
        bond_back: to_fund,
        lf_back: lf,
        stray,
        phase,
    })
}

pub const CLAIMED: u64 = 2;

pub fn dispute_open(w: &Wire<'_>, instance: &str, case_id: &str) -> Result<(), Why> {
    let v = crate::view::case(w, instance, case_id, 0)?;
    let now = a1::precheck::chain_now(w.t, w.table).map_err(a1w)?;
    if v.phase != CLAIMED || now > v.window_end {
        return Err(why("E_DISPUTE_WINDOW", format!("申请退款只在待确认期内(已申报交付、T2 至 {} 前);现在阶段 {},链上时刻 {now}", v.window_end, v.phase)));
    }
    Ok(())
}

pub fn refund(w: &Wire<'_>, o: &Order, post: &mut dyn Post) -> Result<String, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    a1::chain::code_matches(w.t, w.table, &o.instance, w.logic).map_err(a1w)?;
    post.post(&a1::call::Action::Refund.call(&o.instance).map_err(a1w)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_refund_while_a_dispute_is_ruled_inside_t3() {
        assert!(!refundable(3, Some(seat_col::STATE_RULING), 100, 100), "T3 未满不给退款键");
        assert!(refundable(3, Some(seat_col::STATE_RULING), 100, 101), "T3 过后照退");
        assert!(refundable(1, Some(seat_col::STATE_SEATED), 100, 50));
        assert!(refundable(2, None, 0, 50));
        assert!(!refundable(4, None, 0, 50) && !refundable(0, None, 0, 50));
    }
}
