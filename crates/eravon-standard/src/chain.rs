use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_literals::constants as k7;
use eravon_terms as k2;

use crate::abi::{self, Ty, Val};
use crate::call;
use crate::fault::{Fault, R};
use crate::order;
use crate::sol;

#[derive(Clone, Debug)]
pub struct Read<T> {
    pub value: T,
    pub agreement: &'static str,
    pub endpoints: usize,
}

fn wrap<T>(v: T, r: &a2::read::Reading) -> Read<T> {
    Read { value: v, agreement: r.agreement.word(), endpoints: r.endpoints }
}

fn call_at<T: Transport>(
    t: &T,
    table: &Table,
    to: &str,
    data: &[u8],
) -> R<(Vec<u8>, a2::read::Reading)> {
    crate::mark::mark(crate::mark::FEATURE);
    let (hex, r) = a2::read::call(t, table, to, &order::hex_of(data), a2::seam::TAG_LATEST)?;
    Ok((order::bytes_of("答", &hex)?, r))
}

pub fn code_matches<T: Transport>(
    t: &T,
    table: &Table,
    instance: &str,
    logic: &str,
) -> R<Read<bool>> {
    let (code, r) = a2::read::code(t, table, instance, a2::seam::TAG_LATEST)?;
    writable(judge(&code, logic, k7::past_rows())?)?;
    Ok(wrap(true, &r))
}


pub fn code_generation<T: Transport>(t: &T, table: &Table, instance: &str, logic: &str) -> R<Read<k7::Generation>> {
    let (code, r) = a2::read::code(t, table, instance, a2::seam::TAG_LATEST)?;
    Ok(wrap(judged(&code, logic)?, &r))
}

pub fn generation_of_code(code: &[u8], logic: &str) -> R<k7::Generation> {
    judged(code, logic)
}

pub fn case_generation<T: Transport>(t: &T, table: &Table, instance: &str, logic: &str, listing_logic: Option<&str>) -> R<k7::Generation> {
    let (code, _) = a2::read::code(t, table, instance, a2::seam::TAG_LATEST)?;
    if code.is_empty() {
        return Ok(listing_logic.and_then(k7::generation_of_logic).unwrap_or(k7::Generation::Current));
    }
    judged(&code, logic)
}

fn judged(code: &[u8], logic: &str) -> R<k7::Generation> {
    judge(code, logic, k7::past_rows())
}


pub fn acting_key<T: Transport>(t: &T, table: &Table, key: &str, past: bool) -> R<String> {
    if past {
        return Ok(key.to_ascii_lowercase());
    }
    Ok(current(t, table, key)?.value)
}

pub fn judge(code: &[u8], logic: &str, past: &[k7::StandardTable]) -> R<k7::Generation> {
    let want = k2::derive::runtime(logic);
    if code == want.as_slice() {
        return Ok(k7::generation_among(logic, &k7::active_standard(), past).unwrap_or(k7::Generation::Current));
    }
    let current = k7::active_standard().logic;
    if !current.eq_ignore_ascii_case(logic) && code == k2::derive::runtime(current).as_slice() {
        return Ok(k7::Generation::Current);
    }
    if let Some(p) = past.iter().find(|p| code == k2::derive::runtime(p.logic).as_slice()) {
        return Ok(k7::Generation::Past(*p));
    }
    Err(Fault::CodeHash { want: order::hex_of(&want), got: order::hex_of(code) })
}

pub fn writable(g: k7::Generation) -> R<()> {
    match g {
        k7::Generation::Current => Ok(()),
        k7::Generation::Past(p) => past_gate(true).map_err(|_| Fault::PastContract { logic: p.logic.to_string() }),
    }
}


pub fn past_gate(past: bool) -> R<()> {
    if past && !k7::past_switch() {
        return Err(Fault::PastContract { logic: String::new() });
    }
    Ok(())
}


pub fn listing_writable(logic: &str) -> R<()> {
    match k7::generation_of_logic(logic) {
        Some(g) => writable(g),
        None => Ok(()),
    }
}

pub fn listing_payable(logic: &str) -> R<()> {
    payable_under(k7::generation_of_logic(logic))
}


pub fn payable_under(g: Option<k7::Generation>) -> R<()> {
    match g {
        Some(k7::Generation::Past(p)) => Err(Fault::PastListing { logic: p.logic.to_string() }),
        _ => Ok(()),
    }
}

pub fn has_code<T: Transport>(t: &T, table: &Table, addr: &str) -> R<bool> {
    let (code, _) = a2::read::code(t, table, addr, a2::seam::TAG_LATEST)?;
    Ok(!code.is_empty())
}

pub fn verifier_present<T: Transport>(
    t: &T,
    table: &Table,
) -> R<Read<(bool, String, String)>> {
    crate::mark::mark(crate::mark::FEATURE);
    let (code, r) = a2::read::code(t, table, k7::VERIFIER, a2::seam::TAG_LATEST)?;
    let got = if code.is_empty() {
        "0x".to_string() + &"0".repeat(64)
    } else {
        order::hex_of(&k2::derive::keccak(&code))
    };
    let want = k7::VERIFIER_CODE_HASH.to_ascii_lowercase();
    let same = got.eq_ignore_ascii_case(&want);
    Ok(wrap((same, got, want), &r))
}

pub mod status_col {
    pub use crate::sol::src_abi::{
        ERAVON_T_CASE_STATUS_COL_CLAIMED_AT as CLAIMED_AT, ERAVON_T_CASE_STATUS_COL_DEPOSIT as DEPOSIT,
        ERAVON_T_CASE_STATUS_COL_DISPUTED_AT as DISPUTED_AT, ERAVON_T_CASE_STATUS_COL_DRAW_BLOCK as DRAW_BLOCK,
        ERAVON_T_CASE_STATUS_COL_LF_HELD as LF_HELD, ERAVON_T_CASE_STATUS_COL_PANEL_HASH as PANEL_HASH,
        ERAVON_T_CASE_STATUS_COL_PANEL_INDEX as PANEL_INDEX, ERAVON_T_CASE_STATUS_COL_PANEL_SIZE as PANEL_SIZE,
        ERAVON_T_CASE_STATUS_COL_PENDING_REDRAW as PENDING_REDRAW, ERAVON_T_CASE_STATUS_COL_PHASE as PHASE,
        ERAVON_T_CASE_STATUS_COL_PREMIUM as PREMIUM, ERAVON_T_CASE_STATUS_COL_REDRAW_DEADLINE as REDRAW_DEADLINE,
        ERAVON_T_CASE_STATUS_COL_ROOT as ROOT, ERAVON_T_CASE_STATUS_COL_ROUTER as ROUTER,
        ERAVON_T_CASE_STATUS_COL_SEATS as SEATS, ERAVON_T_CASE_STATUS_COL_UNPAID as UNPAID,
    };
}

pub mod seat_col {
    pub use crate::sol::src_abi::{
        I_GALEED_VENUE_T_SEAT_COL_ACCEPTED as ACCEPTED, I_GALEED_VENUE_T_SEAT_COL_AMANAH_HASH as AMANAH_HASH,
        I_GALEED_VENUE_T_SEAT_COL_AMOUNT as AMOUNT, I_GALEED_VENUE_T_SEAT_COL_JUDGE as JUDGE,
        I_GALEED_VENUE_T_SEAT_COL_PARTIES as PARTIES, I_GALEED_VENUE_T_SEAT_COL_QUOTE_HASH as QUOTE_HASH,
        I_GALEED_VENUE_T_SEAT_COL_STATE as STATE, I_GALEED_VENUE_T_SEAT_COL_TERMS_HASH as TERMS_HASH,
        I_GALEED_VENUE_T_SEAT_COL_TOKEN as TOKEN,
    };
}

pub mod panel_col {
    pub use crate::sol::src_abi::{
        ERAVON_PANEL_BOOK_PANEL_AT_COL_EXPIRES as EXPIRES, ERAVON_PANEL_BOOK_PANEL_AT_COL_PANEL_HASH as PANEL_HASH,
        ERAVON_PANEL_BOOK_PANEL_AT_COL_RECORDED as RECORDED, ERAVON_PANEL_BOOK_PANEL_AT_COL_ROSTER_HASH as ROSTER_HASH,
        ERAVON_PANEL_BOOK_PANEL_AT_COL_ROUTER as ROUTER, ERAVON_PANEL_BOOK_PANEL_AT_COL_ROUTER_HASH as ROUTER_HASH,
    };
}

mod checked_col {
    pub use crate::sol::src_abi::{
        ERAVON_VERIFIER_T_CHECKED_COL_CAP as CAP, ERAVON_VERIFIER_T_CHECKED_COL_FUND_UNBOUND as FUND_UNBOUND,
        ERAVON_VERIFIER_T_CHECKED_COL_LF as LF, ERAVON_VERIFIER_T_CHECKED_COL_OVERFLOW as OVERFLOW,
        ERAVON_VERIFIER_T_CHECKED_COL_PRICE_TOKEN as PRICE_TOKEN, ERAVON_VERIFIER_T_CHECKED_COL_PRICE_UNIT as PRICE_UNIT,
        ERAVON_VERIFIER_T_CHECKED_COL_QUOTE_BAD as QUOTE_BAD, ERAVON_VERIFIER_T_CHECKED_COL_RATE_AT as RATE_AT,
        ERAVON_VERIFIER_T_CHECKED_COL_SERIES_JUDGE as SERIES_JUDGE, ERAVON_VERIFIER_T_CHECKED_COL_STAGE_FAULT as STAGE_FAULT,
        ERAVON_VERIFIER_T_CHECKED_COL_STALE as STALE, ERAVON_VERIFIER_T_CHECKED_COL_TERMS_FAULT as TERMS_FAULT,
    };
}

pub fn status<T: Transport>(t: &T, table: &Table, instance: &str) -> R<Read<Vec<Val>>> {
    let (out, r) = call_at(t, table, instance, &call::view(sol::STATUS)?)?;
    Ok(wrap(status_of(&out)?, &r))
}

pub fn status_of(out: &[u8]) -> R<Vec<Val>> {
    if out.len() == 32 * TRIAL_STATUS_WORDS {
        return trial_status_of(out);
    }
    let v = abi::decode(&abi::tuple_types(sol::src_abi::ERAVON_STATUS_RETURNS)?, out)?;
    Ok(v.first().ok_or_else(|| Fault::Abi("status 没有答".into()))?.items()?.to_vec())
}

const TRIAL_STATUS_WORDS: usize = 14;

fn trial_status_of(out: &[u8]) -> R<Vec<Val>> {
    use sol::past_trial as p;
    let v = abi::decode(&abi::tuple_types(p::ERAVON_STATUS_RETURNS)?, out)?;
    let pick = |i: usize| v.get(i).cloned().ok_or_else(|| Fault::Abi(format!("往代 status 缺第 {i} 字")));
    let mut cols = vec![Val::Addr([0; 20]); 16];
    for (cur, old) in [
        (status_col::PHASE, p::ERAVON_STATUS_COL_PHASE),
        (status_col::SEATS, p::ERAVON_STATUS_COL_SEATS),
        (status_col::PREMIUM, p::ERAVON_STATUS_COL_PREMIUM),
        (status_col::DEPOSIT, p::ERAVON_STATUS_COL_DEPOSIT),
        (status_col::LF_HELD, p::ERAVON_STATUS_COL_LF_HELD),
        (status_col::UNPAID, p::ERAVON_STATUS_COL_UNPAID),
        (status_col::CLAIMED_AT, p::ERAVON_STATUS_COL_CLAIMED_AT),
        (status_col::DISPUTED_AT, p::ERAVON_STATUS_COL_DISPUTED_AT),
        (status_col::PENDING_REDRAW, p::ERAVON_STATUS_COL_PENDING_REDRAW),
        (status_col::REDRAW_DEADLINE, p::ERAVON_STATUS_COL_REDRAW_DEADLINE),
        (status_col::DRAW_BLOCK, p::ERAVON_STATUS_COL_DRAW_BLOCK),
        (status_col::PANEL_INDEX, p::ERAVON_STATUS_COL_PANEL_INDEX),
        (status_col::PANEL_HASH, p::ERAVON_STATUS_COL_PANEL_HASH),
        (status_col::PANEL_SIZE, p::ERAVON_STATUS_COL_PANEL_SIZE),
    ] {
        cols[cur] = pick(old)?;
    }
    Ok(cols)
}

pub fn fee_quoted<T: Transport>(t: &T, table: &Table, instance: &str) -> R<Read<Vec<Val>>> {
    let (out, r) = call_at(t, table, instance, &call::view(sol::FEE_QUOTED)?)?;
    Ok(wrap(fee_quoted_of(&out)?, &r))
}

pub fn fee_quoted_of(out: &[u8]) -> R<Vec<Val>> {
    abi::decode(&abi::tuple_types(sol::src_abi::ERAVON_FEE_QUOTED_RETURNS)?, out)
}

pub fn status_and_fee<T: Transport>(t: &T, table: &Table, instance: &str) -> R<(Read<Vec<Val>>, Read<Vec<Val>>)> {
    let asks = [crate::multi::status(instance)?, crate::multi::Ask { to: instance.to_string(), data: call::view(sol::FEE_QUOTED)? }];
    match crate::multi::read(t, table, &asks, a2::seam::TAG_LATEST)? {
        Some(r) => {
            let got = |i: usize, what: &str| r.value.get(i).cloned().flatten().ok_or_else(|| Fault::Abi(format!("{instance} 的 {what} 回退了")));
            let at = |v: Vec<Val>| Read { value: v, agreement: r.agreement, endpoints: r.endpoints };
            Ok((at(status_of(&got(0, "status")?)?), at(fee_quoted_of(&got(1, "feeQuoted")?)?)))
        }
        None => Ok((status(t, table, instance)?, fee_quoted(t, table, instance)?)),
    }
}

pub fn ahead_status_and_fee<T: Transport>(t: &T, table: &Table, instances: &[String]) {
    if instances.len() < 2 {
        return;
    }
    let (Ok(status), Ok(fee)) = (call::view(sol::STATUS), call::view(sol::FEE_QUOTED)) else { return };
    let sets: Vec<Vec<crate::multi::Ask>> = instances
        .iter()
        .filter(|i| order::addr_of("实例", i).is_ok())
        .map(|i| vec![crate::multi::Ask { to: i.clone(), data: status.clone() }, crate::multi::Ask { to: i.clone(), data: fee.clone() }])
        .collect();
    if !crate::multi::ahead(t, table, &sets, a2::seam::TAG_LATEST) {
        let one = |data: &[u8]| instances.iter().map(|i| (i.clone(), order::hex_of(data))).collect::<Vec<_>>();
        a2::read::ahead_calls(t, table, None, &one(&status), a2::seam::TAG_LATEST);
        a2::read::ahead_calls(t, table, None, &one(&fee), a2::seam::TAG_LATEST);
    }
}

pub fn ahead_views<T: Transport>(t: &T, table: &Table, views: &[(String, Vec<u8>)]) {
    let calls: Vec<(String, String)> = views.iter().map(|(to, d)| (to.clone(), order::hex_of(d))).collect();
    a2::read::ahead_calls(t, table, None, &calls, a2::seam::TAG_LATEST);
}

pub fn ahead_decimals<T: Transport>(t: &T, table: &Table, tokens: &[String]) {
    let Ok(data) = call::view(sol::ERC20_DECIMALS) else { return };
    ahead_views(t, table, &tokens.iter().map(|k| (k.clone(), data.clone())).collect::<Vec<_>>());
}

pub fn ahead_seats<T: Transport>(t: &T, table: &Table, seats: &[(String, [u8; 32], u64)]) {
    let calls: Vec<(String, String)> = seats.iter().filter_map(|(i, c, s)| Some((i.clone(), order::hex_of(&call::seat_of(c, *s).ok()?)))).collect();
    a2::read::ahead_calls(t, table, None, &calls, a2::seam::TAG_LATEST);
}

pub fn ahead_stages<T: Transport>(t: &T, table: &Table, instances: &[String]) {
    let Ok(data) = call::view(sol::STAGE) else { return };
    a2::read::ahead_codes(t, table, instances, a2::seam::TAG_LATEST);
    ahead_views(t, table, &instances.iter().map(|i| (i.clone(), data.clone())).collect::<Vec<_>>());
}

pub fn ahead_erc20_balances<T: Transport>(t: &T, table: &Table, held: &[(String, String)]) {
    let views: Vec<(String, Vec<u8>)> = held.iter().filter_map(|(k, who)| Some((k.clone(), call::erc20_balance_of(&order::addr_of("谁", who).ok()?).ok()?))).collect();
    ahead_views(t, table, &views);
}

pub fn ahead_act<T: Transport>(t: &T, table: &Table, o: &order::Order, logic: &str) {
    let mut calls: Vec<(String, String)> = Vec::new();
    if let (Ok(fund), Ok(spender)) = (order::addr_of("押金地址", &o.offer.seller_fund), order::addr_of("逻辑", logic)) {
        if let Ok(d) = call::erc20_balance_of(&fund) {
            calls.push((o.offer.token.clone(), order::hex_of(&d)));
        }
        if let Ok(d) = call::erc20_allowance(&fund, &spender) {
            calls.push((o.offer.token.clone(), order::hex_of(&d)));
        }
    }
    if let Ok(d) = order::addr_of("卖家钥", &o.offer.seller_key).and_then(|k| call::revoked(&k, &o.offer.terms_hash)) {
        calls.push((logic.to_string(), order::hex_of(&d)));
    }
    a2::read::ahead_reads(t, table, &[logic.to_string()], &[], &calls, a2::seam::TAG_LATEST);
}

pub fn ahead_fund<T: Transport>(t: &T, table: &Table, logic: &str, seller: &str, fund: &str, token: &str) {
    let (Ok(f), Ok(l)) = (order::addr_of("押金地址", fund), order::addr_of("逻辑", logic)) else { return };
    let mut calls: Vec<(String, String)> = Vec::new();
    if !fund.eq_ignore_ascii_case(seller) {
        if let Ok(d) = order::addr_of("卖家钥", seller).and_then(|k| call::funds(&k, &f)) {
            calls.push((logic.to_string(), order::hex_of(&d)));
        }
    }
    for d in [call::erc20_balance_of(&f), call::erc20_allowance(&f, &l)].into_iter().flatten() {
        calls.push((token.to_string(), order::hex_of(&d)));
    }
    a2::read::ahead_reads(t, table, &[], &[], &calls, a2::seam::TAG_LATEST);
}

pub fn ahead_current_and_pending<T: Transport>(t: &T, table: &Table, key: &str) {
    let Ok(a) = order::addr_of("钥", key) else { return };
    let views: Vec<(String, Vec<u8>)> = [call::succession_current(&a), call::succession_pending(&a)].into_iter().filter_map(Result::ok).map(|d| (k7::SUCCESSION_BOOK.to_string(), d)).collect();
    ahead_views(t, table, &views);
}

pub fn ahead_current<T: Transport>(t: &T, table: &Table, keys: &[String]) {
    let views: Vec<(String, Vec<u8>)> = keys.iter().filter_map(|k| Some((k7::SUCCESSION_BOOK.to_string(), call::succession_current(&order::addr_of("钥", k).ok()?).ok()?))).collect();
    ahead_views(t, table, &views);
}

pub fn seat_of<T: Transport>(
    t: &T,
    table: &Table,
    instance: &str,
    case_id: &[u8; 32],
    seat: u64,
) -> R<Read<Vec<Val>>> {
    let (out, r) = call_at(t, table, instance, &call::seat_of(case_id, seat)?)?;
    Ok(wrap(seat_answer(&out)?, &r))
}

pub fn seat_answer(out: &[u8]) -> R<Vec<Val>> {
    let word = |i: usize| out.get(32 * i..32 * (i + 1)).map(|w| w.iter().rev().take(8).rev().fold(0u64, |a, b| (a << 8) | u64::from(*b)));
    use sol::past_trial as p;
    use sol::src_abi as c;
    let old_parties = p::I_GALEED_VENUE_T_SEAT_COL_PARTIES;
    let new_parties = c::I_GALEED_VENUE_T_SEAT_COL_PARTIES;
    if word(1 + old_parties) == Some(32 * (old_parties as u64 + 1)) && word(1 + new_parties) != Some(32 * (new_parties as u64 + 1)) {
        let v = abi::decode(&abi::tuple_types(p::ERAVON_SEAT_OF_RETURNS)?, out)?;
        let old = v.first().ok_or_else(|| Fault::Abi("往代 seatOf 没有答".into()))?.items()?.to_vec();
        let pick = |i: usize| old.get(i).cloned().ok_or_else(|| Fault::Abi(format!("往代 seatOf 缺第 {i} 栏")));
        let mut cols = vec![Val::u64(0); new_parties + 1];
        for (cur, was) in [
            (c::I_GALEED_VENUE_T_SEAT_COL_JUDGE, p::I_GALEED_VENUE_T_SEAT_COL_JUDGE),
            (c::I_GALEED_VENUE_T_SEAT_COL_AMANAH_HASH, p::I_GALEED_VENUE_T_SEAT_COL_AMANAH_HASH),
            (c::I_GALEED_VENUE_T_SEAT_COL_TERMS_HASH, p::I_GALEED_VENUE_T_SEAT_COL_TERMS_HASH),
            (c::I_GALEED_VENUE_T_SEAT_COL_QUOTE_HASH, p::I_GALEED_VENUE_T_SEAT_COL_QUOTE_HASH),
            (c::I_GALEED_VENUE_T_SEAT_COL_WINDOW_START, p::I_GALEED_VENUE_T_SEAT_COL_WINDOW_START),
            (c::I_GALEED_VENUE_T_SEAT_COL_WINDOW_END, p::I_GALEED_VENUE_T_SEAT_COL_WINDOW_END),
            (c::I_GALEED_VENUE_T_SEAT_COL_ACCEPTED, p::I_GALEED_VENUE_T_SEAT_COL_ACCEPTED),
            (c::I_GALEED_VENUE_T_SEAT_COL_STATE, p::I_GALEED_VENUE_T_SEAT_COL_STATE),
            (c::I_GALEED_VENUE_T_SEAT_COL_RUNGS, p::I_GALEED_VENUE_T_SEAT_COL_RUNGS),
            (c::I_GALEED_VENUE_T_SEAT_COL_RUNG_MIN, p::I_GALEED_VENUE_T_SEAT_COL_RUNG_MIN),
            (c::I_GALEED_VENUE_T_SEAT_COL_RUNG_MAX, p::I_GALEED_VENUE_T_SEAT_COL_RUNG_MAX),
            (c::I_GALEED_VENUE_T_SEAT_COL_TOKEN, p::I_GALEED_VENUE_T_SEAT_COL_TOKEN),
            (c::I_GALEED_VENUE_T_SEAT_COL_AMOUNT, p::I_GALEED_VENUE_T_SEAT_COL_AMOUNT),
            (new_parties, old_parties),
        ] {
            cols[cur] = pick(was)?;
        }
        return Ok(cols);
    }
    let v = abi::decode(&abi::tuple_types(sol::src_abi::ERAVON_SEAT_OF_RETURNS)?, out)?;
    Ok(v.first().ok_or_else(|| Fault::Abi("seatOf 没有答".into()))?.items()?.to_vec())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub state: u64,
    pub shares: Vec<[u8; 32]>,
    pub ruling_fee_from: Vec<[u8; 32]>,
    pub standby_fee: [u8; 32],
    pub ruling_fee: [u8; 32],
}

fn outcome_read<T: Transport>(t: &T, table: &Table, instance: &str, data: &[u8]) -> R<Read<Outcome>> {
    let (out, r) = call_at(t, table, instance, data)?;
    let v = abi::decode(&abi::tuple_types(sol::src_abi::ERAVON_OUTCOME_OF_RETURNS)?, &out)?;
    let o = v.first().ok_or_else(|| Fault::Abi("Outcome 的答是空的".into()))?.items()?;
    let col = |i: usize| o.get(i).ok_or_else(|| Fault::Abi(format!("Outcome 缺第 {i} 栏")));
    let words = |i: usize| -> R<Vec<[u8; 32]>> { col(i)?.items()?.iter().map(Val::word).collect() };
    use sol::src_abi as c;
    let got = Outcome {
        state: col(c::I_GALEED_VENUE_T_OUTCOME_COL_STATE)?.small()?,
        shares: words(c::I_GALEED_VENUE_T_OUTCOME_COL_SHARES)?,
        ruling_fee_from: words(c::I_GALEED_VENUE_T_OUTCOME_COL_RULING_FEE_FROM)?,
        standby_fee: col(c::I_GALEED_VENUE_T_OUTCOME_COL_STANDBY_FEE)?.word()?,
        ruling_fee: col(c::I_GALEED_VENUE_T_OUTCOME_COL_RULING_FEE)?.word()?,
    };
    Ok(wrap(got, &r))
}

pub fn outcome_of<T: Transport>(t: &T, table: &Table, instance: &str, case_id: &[u8; 32], seat: u64, rung: u64) -> R<Read<Outcome>> {
    outcome_read(t, table, instance, &outcome_of_data(case_id, seat, rung))
}

fn outcome_of_data(case_id: &[u8; 32], seat: u64, rung: u64) -> Vec<u8> {
    let mut data = sol::OUTCOME_OF.selector().to_vec();
    data.extend(abi::encode(&[Val::B32(*case_id), Val::u64(seat), Val::u64(rung), Val::Bytes(Vec::new())]));
    data
}

pub fn ahead_outcomes<T: Transport>(t: &T, table: &Table, instance: &str, case_id: &[u8; 32], seat: u64, rungs: std::ops::Range<u64>) {
    ahead_views(t, table, &rungs.map(|r| (instance.to_string(), outcome_of_data(case_id, seat, r))).collect::<Vec<_>>());
}

pub fn advance_of<T: Transport>(t: &T, table: &Table, instance: &str, case_id: &[u8; 32], seat: u64) -> R<Read<Outcome>> {
    outcome_read(t, table, instance, &advance_of_data(case_id, seat))
}

fn advance_of_data(case_id: &[u8; 32], seat: u64) -> Vec<u8> {
    let mut data = sol::ADVANCE_OF.selector().to_vec();
    data.extend(abi::encode(&[Val::B32(*case_id), Val::u64(seat)]));
    data
}

pub fn ahead_seat_and_advance<T: Transport>(t: &T, table: &Table, instance: &str, case_id: &[u8; 32], seat: u64) {
    let Ok(s) = call::seat_of(case_id, seat) else { return };
    ahead_views(t, table, &[(instance.to_string(), s), (instance.to_string(), advance_of_data(case_id, seat))]);
}

pub fn account_of<T: Transport>(
    t: &T,
    table: &Table,
    instance: &str,
    who: &str,
    token: &str,
) -> R<Read<(String, [u8; 32])>> {
    let data = call::account_of(
        &order::addr_of("谁", who)?,
        &order::addr_of("币", token)?,
    )?;
    let (out, r) = call_at(t, table, instance, &data)?;
    let v = abi::decode(&abi::tuple_types(sol::src_abi::ERAVON_ACCOUNT_OF_RETURNS)?, &out)?;
    let col = |i: usize| v.get(i).ok_or_else(|| Fault::Abi(format!("accountOf 缺第 {i} 栏")));
    Ok(wrap((order::hex_of(&col(sol::src_abi::ERAVON_ACCOUNT_OF_COL_PAYOUT)?.addr()?), col(sol::src_abi::ERAVON_ACCOUNT_OF_COL_CLAIMABLE)?.word()?), &r))
}

pub fn owed<T: Transport>(t: &T, table: &Table, instance: &str, who: &str, token: &str) -> R<Read<[u8; 32]>> {
    let data = call::with(sol::OWED, &[Val::Addr(order::addr_of("谁", who)?), Val::Addr(order::addr_of("币", token)?)])?;
    let (out, r) = call_at(t, table, instance, &data)?;
    let v = abi::decode(&abi::tuple_types(sol::src_abi::ERAVON_OWED_RETURNS)?, &out)?;
    Ok(wrap(v[0].word()?, &r))
}

pub fn panel_book<T: Transport>(t: &T, table: &Table, logic: &str) -> R<Read<String>> {
    let (out, r) = call_at(t, table, logic, &call::view(sol::PANEL_BOOK)?)?;
    let v = abi::decode(&abi::tuple_types(sol::src_abi::ERAVON_PANEL_BOOK_RETURNS)?, &out)?;
    Ok(wrap(order::hex_of(&v[0].addr()?), &r))
}

pub fn book_count<T: Transport>(t: &T, table: &Table, book: &str, root: &str, key: &[u8; 32]) -> R<Read<u64>> {
    let (out, r) = call_at(t, table, book, &call::book_count(&order::addr_of("行会根", root)?, key)?)?;
    let v = abi::decode(&abi::tuple_types(sol::src_abi::ERAVON_PANEL_BOOK_COUNT_RETURNS)?, &out)?;
    Ok(wrap(v[0].small()?, &r))
}

pub fn book_panel_at<T: Transport>(t: &T, table: &Table, book: &str, root: &str, key: &[u8; 32], index: u64) -> R<Read<Vec<Val>>> {
    let (out, r) = call_at(t, table, book, &call::book_panel_at(&order::addr_of("行会根", root)?, key, index)?)?;
    Ok(wrap(abi::decode(&abi::tuple_types(sol::src_abi::ERAVON_PANEL_BOOK_PANEL_AT_RETURNS)?, &out)?, &r))
}

pub fn book_latest<T: Transport>(t: &T, table: &Table, book: &str, root: &str, key: &[u8; 32]) -> R<Option<Read<Vec<Val>>>> {
    let n = book_count(t, table, book, root, key)?.value;
    let Some(last) = n.checked_sub(1) else { return Ok(None) };
    let r = book_panel_at(t, table, book, root, key, last)?;
    let v = &r.value;
    let at = |i: usize| v.get(i).cloned().ok_or_else(|| Fault::Abi(format!("panelAt 的答缺第 {i} 栏")));
    use sol::src_abi as c;
    let value = vec![
        Val::u64(last),
        at(c::ERAVON_PANEL_BOOK_PANEL_AT_COL_PANEL_HASH)?,
        at(c::ERAVON_PANEL_BOOK_PANEL_AT_COL_ROSTER_HASH)?,
        at(c::ERAVON_PANEL_BOOK_PANEL_AT_COL_EXPIRES)?,
        at(c::ERAVON_PANEL_BOOK_PANEL_AT_COL_RECORDED)?,
        at(c::ERAVON_PANEL_BOOK_PANEL_AT_COL_ROUTER)?,
        at(c::ERAVON_PANEL_BOOK_PANEL_AT_COL_ROUTER_HASH)?,
    ];
    Ok(Some(Read { value, agreement: r.agreement, endpoints: r.endpoints }))
}

pub fn current<T: Transport>(t: &T, table: &Table, a: &str) -> R<Read<String>> {
    succession_read(t, table, &call::succession_current(&order::addr_of("钥", a)?)?, sol::src_abi::I_GALEED_SUCCESSION_BOOK_CURRENT_RETURNS)
}

pub fn root<T: Transport>(t: &T, table: &Table, a: &str) -> R<Read<String>> {
    succession_read(t, table, &call::succession_root(&order::addr_of("钥", a)?)?, sol::src_abi::I_GALEED_SUCCESSION_BOOK_ROOT_RETURNS)
}

pub fn pending<T: Transport>(t: &T, table: &Table, a: &str) -> R<Read<String>> {
    succession_read(t, table, &call::succession_pending(&order::addr_of("钥", a)?)?, sol::src_abi::GALEED_SUCCESSION_BOOK_PENDING_RETURNS)
}

pub fn entered<T: Transport>(t: &T, table: &Table, a: &str) -> R<Read<bool>> {
    let (out, r) = call_at(t, table, k7::SUCCESSION_BOOK, &call::succession_entered(&order::addr_of("钥", a)?)?)?;
    if out.is_empty() {
        return Err(book_absent(t, table));
    }
    let v = abi::decode(&abi::tuple_types(sol::src_abi::GALEED_SUCCESSION_BOOK_ENTERED_RETURNS)?, &out)?;
    Ok(wrap(v.first().ok_or_else(|| Fault::Abi("簿的答是空的".into()))?.boolean()?, &r))
}

fn succession_read<T: Transport>(t: &T, table: &Table, data: &[u8], returns: &str) -> R<Read<String>> {
    let (out, r) = call_at(t, table, k7::SUCCESSION_BOOK, data)?;
    if out.is_empty() {
        return Err(book_absent(t, table));
    }
    let v = abi::decode(&abi::tuple_types(returns)?, &out)?;
    Ok(wrap(order::hex_of(&v.first().ok_or_else(|| Fault::Abi("簿的答是空的".into()))?.addr()?), &r))
}

fn book_absent<T: Transport>(t: &T, table: &Table) -> Fault {
    let got = match a2::read::code(t, table, k7::SUCCESSION_BOOK, a2::seam::TAG_LATEST) {
        Ok((code, _)) if code.is_empty() => "0x".to_string() + &"0".repeat(64),
        Ok((code, _)) => order::hex_of(&k2::derive::keccak(&code)),
        Err(f) => return f.into(),
    };
    Fault::Precheck {
        gate: crate::precheck::GATE_CURRENT,
        why: format!("换钥簿 {} 上的码 {got} 不是换钥簿的 {}:这条链上读不出谁是现行钥", k7::SUCCESSION_BOOK, k7::BOOK_CODE_HASH),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stage {
    pub seller_key: String,
    pub buyer_key: String,
    pub ended: bool,
    pub buyer_bps: u16,
    pub series_judge: String,
    pub case_id: [u8; 32],
    pub past: bool,
    pub logic: String,
}

pub fn stage<T: Transport>(t: &T, table: &Table, instance: &str) -> R<Read<Stage>> {
    let (code, _) = a2::read::code(t, table, instance, a2::seam::TAG_LATEST)?;
    let logic = match judge(&code, k7::active_standard().logic, k7::past_rows())? {
        k7::Generation::Current => k7::active_standard().logic,
        k7::Generation::Past(p) => return past_stage(t, table, instance, &p),
    };
    let (out, r) = call_at(t, table, instance, &call::view(sol::STAGE)?)?;
    let not = |why: &str| Fault::Precheck { gate: crate::precheck::GATE_STAGE, why: format!("{instance} 的 stage() {why}") };
    if out.len() != 192 {
        return Err(not(&format!("答了 {} 字节,不是六字 192 字节", out.len())));
    }
    let w = |i: usize| -> [u8; 32] { out[32 * i..32 * (i + 1)].try_into().expect("三十二字节") };
    let narrow = |x: &[u8; 32], keep: usize| x[..32 - keep].iter().all(|b| *b == 0);
    if !narrow(&w(0), 20) || !narrow(&w(1), 20) || !narrow(&w(2), 1) || w(2)[31] > 1 || !narrow(&w(3), 2) || !narrow(&w(4), 20) {
        return Err(not("有一字越出声明的宽"));
    }
    let salt = w(5);
    let at = k2::derive::create2(k7::PUBLIC_DEPLOYER, &salt, &k2::derive::keccak(&k2::derive::initcode(logic)));
    if !at.eq_ignore_ascii_case(instance) {
        return Err(not("的盐证不出这个地址(不是本标准的实例,或还没开案)"));
    }
    Ok(wrap(
        Stage {
            seller_key: order::hex_of(&w(0)[12..]),
            buyer_key: order::hex_of(&w(1)[12..]),
            ended: w(2)[31] == 1,
            buyer_bps: u16::from_be_bytes([w(3)[30], w(3)[31]]),
            series_judge: order::hex_of(&w(4)[12..]),
            case_id: salt,
            past: false,
            logic: logic.to_string(),
        },
        &r,
    ))
}

fn past_stage<T: Transport>(t: &T, table: &Table, instance: &str, row: &k7::StandardTable) -> R<Read<Stage>> {
    let opened = sol::OPENED;
    let topic = order::hex_of(&opened.topic0());
    let (logs, r) = a2::read::logs_where(t, table, Some(instance), &[Some(topic)], row.first_block, a2::seam::Upto::Latest)?;
    let not = |why: &str| Fault::Precheck { gate: crate::precheck::GATE_STAGE, why: format!("{instance}(往代实例){why}") };
    let first = logs.first().ok_or_else(|| not("没有 Opened"))?;
    let topics: Vec<String> = first
        .get("topics")
        .and_then(|v| v.as_arr())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    if topics.len() < 4 {
        return Err(not("的 Opened 不足四个 topic"));
    }
    let word = |i: usize| -> R<[u8; 32]> { order::bytes_of("topic", &topics[i])?.try_into().map_err(|_| Fault::Abi("topic 不是三十二字节".into())) };
    let phase = status(t, table, instance)?.value.get(status_col::PHASE).ok_or_else(|| Fault::Abi("status 缺 phase".into()))?.small()?;
    Ok(wrap(
        Stage {
            seller_key: order::hex_of(&word(2)?[12..]),
            buyer_key: order::hex_of(&word(3)?[12..]),
            ended: phase >= crate::precheck::PHASE_FIRST_TERMINAL,
            buyer_bps: 0,
            series_judge: order::hex_of(&[0u8; 20]),
            case_id: word(1)?,
            past: true,
            logic: row.logic.to_string(),
        },
        &r,
    ))
}

pub fn past_panel<T: Transport>(t: &T, table: &Table, instance: &str, row: &k7::StandardTable, index: u64, panel_hash: &[u8; 32]) -> R<(Vec<String>, Vec<[u8; 32]>)> {
    let frozen = order::hex_of(&sol::TRIAL_PANEL_FROZEN.topic0());
    let (logs, _) = a2::read::logs_where(t, table, Some(instance), &[Some(frozen)], row.first_block, a2::seam::Upto::Latest)?;
    let guild_topic = logs
        .first()
        .and_then(|l| l.get("topics").and_then(|v| v.as_arr()).and_then(|a| a.get(1)).and_then(|x| x.as_str()).map(str::to_string))
        .ok_or_else(|| Fault::Precheck { gate: crate::precheck::GATE_STAGE, why: format!("{instance}(往代实例)没有 PanelFrozen:不是行会案") })?;
    let guild = order::hex_of(&order::bytes_of("topic", &guild_topic)?[12..]);
    let (out, _) = call_at(t, table, instance, &crate::call::with(sol::TRIAL_PANEL_BOOK_OF, &[Val::Addr(order::addr_of("行会", &guild)?)])?)?;
    let book = order::hex_of(&abi::decode(&abi::tuple_types("(address)")?, &out)?.first().ok_or_else(|| Fault::Abi("panelBookOf 没有答".into()))?.addr()?);
    let id = row.proxy_code_hash.to_ascii_lowercase();
    let engaged = order::hex_of(&sol::TRIAL_ENGAGED.topic0());
    let (rows, _) = a2::read::logs_where(t, table, Some(&book), &[Some(engaged), Some(guild_topic.clone()), Some(id)], row.first_block, a2::seam::Upto::Latest)?;
    for l in &rows {
        let data = order::bytes_of("data", l.get("data").and_then(|x| x.as_str()).unwrap_or_default())?;
        let v = abi::decode(&abi::tuple_types("(uint256,address[],bytes32[],bytes32,uint64)")?, &data)?;
        if v.first().map(|x| x.small()).transpose()? != Some(index) {
            continue;
        }
        let members: Vec<String> = v[1].items()?.iter().map(|m| m.addr().map(|a| order::hex_of(&a))).collect::<R<_>>()?;
        let hashes: Vec<[u8; 32]> = v[2].items()?.iter().map(|h| h.word()).collect::<R<_>>()?;
        let mut enc = Vec::new();
        enc.extend(abi::encode(&[Val::Arr(members.iter().map(|m| order::addr_of("成员", m).map(Val::Addr)).collect::<R<_>>()?), Val::Arr(hashes.iter().map(|h| Val::B32(*h)).collect())]));
        if k2::derive::keccak(&enc) != *panel_hash {
            return Err(Fault::Precheck { gate: crate::precheck::GATE_STAGE, why: format!("往代行会 {guild} 第 {index} 条名单与案冻结的 panelHash 不符") });
        }
        return Ok((members, hashes));
    }
    Err(Fault::Precheck { gate: crate::precheck::GATE_STAGE, why: format!("往代行会 {guild} 的簿 {book} 上没有第 {index} 条名单") })
}

pub fn board_present<T: Transport>(t: &T, table: &Table) -> R<Read<(bool, String, String)>> {
    crate::mark::mark(crate::mark::FEATURE);
    let (code, r) = a2::read::code(t, table, k7::BOARD, a2::seam::TAG_LATEST)?;
    let got = if code.is_empty() { "0x".to_string() + &"0".repeat(64) } else { order::hex_of(&k2::derive::keccak(&code)) };
    let want = k7::BOARD_CODE_HASH.to_ascii_lowercase();
    let same = got.eq_ignore_ascii_case(&want);
    Ok(wrap((same, got, want), &r))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    pub terms_fault: u64,
    pub quote_bad: bool,
    pub lf: [u8; 32],
    pub cap: [u8; 32],
    pub price_token: [u8; 32],
    pub price_unit: [u8; 32],
    pub rate_at: u64,
    pub stale: bool,
    pub overflow: bool,
    pub fund_unbound: bool,
    pub stage_fault: Option<&'static str>,
    pub series_judge: String,
}

pub fn verifier_check<T: Transport>(t: &T, table: &Table, o: &order::Order, quote: &[u8]) -> R<Read<Checked>> {
    let data = call::with(
        sol::VERIFIER_CHECK,
        &[o.abi()?, Val::Bytes(quote.to_vec()), Val::Bytes(Vec::new()), Val::Bytes(Vec::new()), Val::Arr(Vec::new()), Val::Arr(Vec::new())],
    )?;
    crate::mark::mark(crate::mark::FEATURE);
    let logic = k7::active_standard().logic;
    let (hex, r) = a2::read::call_from(t, table, logic, k7::VERIFIER, &order::hex_of(&data), a2::seam::TAG_LATEST)?;
    let out = order::bytes_of("答", &hex)?;
    let v = abi::decode(&abi::tuple_types(sol::src_abi::ERAVON_VERIFIER_CHECK_RETURNS)?, &out)?;
    let c = v.first().ok_or_else(|| Fault::Abi("check 没有答".into()))?.items()?;
    let at = |i: usize| c.get(i).ok_or_else(|| Fault::Abi(format!("check 的答缺第 {i} 栏")));
    use checked_col as k;
    let stage = at(k::STAGE_FAULT)?.word()?;
    let stage_fault = if stage == [0u8; 32] {
        None
    } else {
        let sel: [u8; 4] = stage[..4].try_into().expect("四字节");
        Some(sol::error_by_selector(&sel).and_then(|sig| sig.split('(').next()).ok_or_else(|| Fault::Abi(format!("check 的 stageFault {} 不是本标准的错", order::hex_of(&sel))))?)
    };
    Ok(wrap(
        Checked {
            terms_fault: at(k::TERMS_FAULT)?.small()?,
            quote_bad: at(k::QUOTE_BAD)?.boolean()?,
            lf: at(k::LF)?.word()?,
            cap: at(k::CAP)?.word()?,
            price_token: at(k::PRICE_TOKEN)?.word()?,
            price_unit: at(k::PRICE_UNIT)?.word()?,
            rate_at: at(k::RATE_AT)?.small()?,
            stale: at(k::STALE)?.boolean()?,
            overflow: at(k::OVERFLOW)?.boolean()?,
            fund_unbound: at(k::FUND_UNBOUND)?.boolean()?,
            stage_fault,
            series_judge: order::hex_of(&at(k::SERIES_JUDGE)?.addr()?),
        },
        &r,
    ))
}

fn logic_flag<T: Transport>(t: &T, table: &Table, logic: &str, data: &[u8], returns: &str) -> R<Read<bool>> {
    let (out, r) = call_at(t, table, logic, data)?;
    let v = abi::decode(&abi::tuple_types(returns)?, &out)?;
    Ok(wrap(v[0].boolean()?, &r))
}

pub fn revoked<T: Transport>(t: &T, table: &Table, logic: &str, seller: &str, terms_hash: &[u8; 32]) -> R<Read<bool>> {
    logic_flag(t, table, logic, &call::revoked(&order::addr_of("卖家钥", seller)?, terms_hash)?, sol::src_abi::ERAVON_REVOKED_RETURNS)
}

pub fn funds<T: Transport>(t: &T, table: &Table, logic: &str, seller: &str, fund: &str) -> R<Read<bool>> {
    logic_flag(t, table, logic, &call::funds(&order::addr_of("卖家钥", seller)?, &order::addr_of("押金地址", fund)?)?, sol::src_abi::ERAVON_FUNDS_RETURNS)
}

pub fn call_raw<T: Transport>(t: &T, table: &Table, to: &str, data: &[u8]) -> R<Read<Vec<u8>>> {
    let (out, r) = call_at(t, table, to, data)?;
    Ok(wrap(out, &r))
}

pub fn call_read<T: Transport>(
    t: &T,
    table: &Table,
    to: &str,
    data: &[u8],
    tys: &[Ty],
) -> R<Read<Vec<Val>>> {
    let (out, r) = call_at(t, table, to, data)?;
    Ok(wrap(abi::decode(tys, &out)?, &r))
}

pub fn erc20_balance<T: Transport>(
    t: &T,
    table: &Table,
    token: &str,
    who: &str,
) -> R<Read<[u8; 32]>> {
    let data = call::erc20_balance_of(&order::addr_of("谁", who)?)?;
    let (out, r) = call_at(t, table, token, &data)?;
    let v = abi::decode(&abi::tuple_types(sol::src_abi::IERC20_BALANCE_OF_RETURNS)?, &out)?;
    Ok(wrap(v[0].word()?, &r))
}

pub fn erc20_allowance<T: Transport>(
    t: &T,
    table: &Table,
    token: &str,
    owner: &str,
    spender: &str,
) -> R<Read<[u8; 32]>> {
    let data = call::erc20_allowance(
        &order::addr_of("授权人", owner)?,
        &order::addr_of("被授权人", spender)?,
    )?;
    let (out, r) = call_at(t, table, token, &data)?;
    let v = abi::decode(&[Ty::Uint], &out)?;
    Ok(wrap(v[0].word()?, &r))
}

pub fn erc20_decimals<T: Transport>(t: &T, table: &Table, token: &str) -> Option<u64> {
    call_read(t, table, token, &call::view(sol::ERC20_DECIMALS).ok()?, &[Ty::Uint]).ok()?.value.first()?.small().ok().filter(|d| *d <= u64::from(eravon_literals::units::DECIMALS_MAX))
}

pub fn amount_decimals<T: Transport>(t: &T, table: &Table, rows: &[eravon_store::settings::TokenRow], token: &str) -> Option<u8> {
    use eravon_vault::tokens;
    let row = tokens::find(rows, tokens::chain(), token);
    if let Some(d) = row.filter(|r| matches!(r.source.as_str(), tokens::FROM_FACTORY | tokens::FROM_CHAIN | tokens::FROM_CHAIN_SINGLE)).and_then(|r| r.decimals) {
        return Some(d);
    }
    match token_probe(t, table, token) {
        TokenProbe::Decimals { decimals, .. } => Some(decimals),
        TokenProbe::NoDecimals => row.and_then(|r| r.decimals),
        TokenProbe::NoCode | TokenProbe::Unreachable(_) => None,
    }
}


pub fn side_decimals<T: Transport>(t: &T, table: &Table, token: &str) -> Option<u64> {
    static KEPT: std::sync::Mutex<Vec<(String, u64)>> = std::sync::Mutex::new(Vec::new());
    let key = token.trim().to_ascii_lowercase();
    if let Some(d) = KEPT.lock().ok().and_then(|k| k.iter().find(|(a, _)| *a == key).map(|(_, d)| *d)) {
        return Some(d);
    }
    let d = erc20_decimals(t, table, &key)?;
    if let Ok(mut k) = KEPT.lock() {
        k.push((key, d));
    }
    Some(d)
}

pub fn reverted(e: &Fault) -> bool {
    matches!(e, Fault::Chain { token, detail } if a2::fault::reverted_word(token, detail))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenProbe {
    NoCode,
    Decimals { decimals: u8, single: bool },
    NoDecimals,
    Unreachable(String),
}

pub fn token_probe<T: Transport>(t: &T, table: &Table, token: &str) -> TokenProbe {
    match a2::read::code(t, table, token, a2::seam::TAG_LATEST) {
        Err(e) => return TokenProbe::Unreachable(format!("{}:{}", e.name(), e.detail())),
        Ok((code, _)) if code.is_empty() => return TokenProbe::NoCode,
        Ok(_) => {}
    }
    let data = match call::view(sol::ERC20_DECIMALS) {
        Ok(d) => d,
        Err(e) => return TokenProbe::Unreachable(format!("{}:{}", e.name(), e.detail())),
    };
    match call_read(t, table, token, &data, &[Ty::Uint]) {
        Ok(r) => match r.value.first().and_then(|v| v.small().ok()).filter(|d| *d <= u64::from(eravon_literals::units::DECIMALS_MAX)) {
            Some(d) => TokenProbe::Decimals { decimals: d as u8, single: r.agreement == "single" },
            None => TokenProbe::NoDecimals,
        },
        Err(e) if matches!(e, Fault::Abi(_)) || reverted(&e) => TokenProbe::NoDecimals,
        Err(e) => TokenProbe::Unreachable(format!("{}:{}", e.name(), e.detail())),
    }
}


fn wants_fill(r: &eravon_store::settings::TokenRow, endpoints: usize) -> bool {
    use eravon_vault::tokens;
    r.source == tokens::FROM_PENDING || r.source == tokens::FROM_MANUAL || (r.source == tokens::FROM_CHAIN_SINGLE && endpoints >= 2)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filled {
    pub changed: usize,
    pub read: Vec<(String, String)>,
}

pub fn refill_tokens<T: Transport>(data: &str, writer: &str, now: u64, t: &T, table: &Table) -> Filled {
    use eravon_vault::tokens;
    let chain = tokens::chain();
    let rows = tokens::list(data);
    let mut out = Filled::default();
    let mut found: Vec<(String, String, Option<u8>, String)> = Vec::new();
    let asked: Vec<String> = rows.iter().filter(|r| r.chain == chain && wants_fill(r, table.len())).map(|r| r.address.clone()).collect();
    a2::read::ahead_codes(t, table, &asked, a2::seam::TAG_LATEST);
    ahead_decimals(t, table, &asked);
    for r in rows.iter().filter(|r| r.chain == chain && wants_fill(r, table.len())) {
        let said = match token_probe(t, table, &r.address) {
            TokenProbe::Decimals { decimals, single } => {
                let source = if single { tokens::FROM_CHAIN_SINGLE } else { tokens::FROM_CHAIN };
                if r.decimals != Some(decimals) || r.source != source {
                    found.push((r.address.clone(), r.source.clone(), Some(decimals), source.to_string()));
                }
                match r.decimals {
                    Some(typed) if r.source == tokens::FROM_MANUAL && typed != decimals => format!("decimals() = {decimals};原手填 {typed} 与链答不同,改记链上的"),
                    Some(_) if r.source == tokens::FROM_MANUAL => format!("decimals() = {decimals};与手填相同,改记链上的"),
                    _ => format!("decimals() = {decimals}"),
                }
            }
            TokenProbe::NoDecimals => {
                if r.source == tokens::FROM_PENDING {
                    found.push((r.address.clone(), r.source.clone(), None, tokens::FROM_MANUAL.to_string()));
                }
                "合约在,decimals() 没有答出 0 到 77 之间的数".to_string()
            }
            TokenProbe::NoCode => "这一地址上没有码(照旧待读链,每一趟再问;可手填或移除)".to_string(),
            TokenProbe::Unreachable(why) => format!("读不成:{why}"),
        };
        out.read.push((r.address.clone(), said));
    }
    if found.is_empty() {
        return out;
    }
    let written = eravon_vault::store::open_write(data, writer, now).and_then(|s| {
        let mut g = eravon_vault::store::read_global(s.data())?;
        let mut cur = g.tokens.clone().unwrap_or_else(|| tokens::list(s.data()));
        let mut n = 0usize;
        for (addr, was, decimals, source) in &found {
            if let Some(r) = cur.iter_mut().find(|r| r.chain == chain && r.address.eq_ignore_ascii_case(addr) && r.source == *was) {
                r.decimals = *decimals;
                r.source = source.clone();
                n += 1;
            }
        }
        if n > 0 {
            g.tokens = Some(cur);
            eravon_vault::store::write_global(s.data(), &g)?;
        }
        Ok(n)
    });
    match written {
        Ok(n) => out.changed = n,
        Err(e) => {
            let why = format!("写进登载表没成:{}:{}", e.name(), e.detail());
            for (addr, said) in out.read.iter_mut() {
                if found.iter().any(|(a, ..)| a.eq_ignore_ascii_case(addr)) {
                    said.push_str(&format!(" · {why}"));
                }
            }
        }
    }
    out
}


pub fn permit_facts<T: Transport>(t: &T, table: &Table, token: &str, owner: &str) -> R<Option<([u8; 32], [u8; 32])>> {
    let no = |e: &Fault| matches!(e, Fault::Abi(_)) || reverted(e);
    let domain = match call_read(t, table, token, &call::view(sol::ERC20_DOMAIN_SEPARATOR)?, &[Ty::B32]) {
        Ok(r) => r.value[0].word()?,
        Err(e) if no(&e) => return Ok(None),
        Err(e) => return Err(e),
    };
    let nonce = match call_read(t, table, token, &call::erc20_nonces(&order::addr_of("授权人", owner)?)?, &[Ty::Uint]) {
        Ok(r) => r.value[0].word()?,
        Err(e) if no(&e) => return Ok(None),
        Err(e) => return Err(e),
    };
    Ok(Some((domain, nonce)))
}

pub fn standard_table() -> (u64, &'static str, &'static str) {
    let row = k7::active_standard();
    (row.chain_id, row.logic, row.proxy_code_hash)
}

#[cfg(test)]
mod generation_tests {
    use super::*;

    #[test]
    fn a_trial_answer_is_read_into_the_current_columns_by_name() {
        let w = |n: u64| { let mut x = [0u8; 32]; x[24..].copy_from_slice(&n.to_be_bytes()); x };
        let mut out = Vec::new();
        for i in 0..14u64 {
            out.extend_from_slice(&w(if i == 8 { 1 } else { 100 + i }));
        }
        let cols = status_of(&out).expect("十四字的往代 status 读得成");
        assert_eq!(cols.len(), 16);
        assert_eq!(cols[status_col::PHASE].small().unwrap(), 100);
        assert_eq!(cols[status_col::SEATS].small().unwrap(), 101);
        assert_eq!(cols[status_col::DEPOSIT].small().unwrap(), 103);
        assert_eq!(cols[status_col::PANEL_SIZE].small().unwrap(), 113);
        assert_eq!(cols[status_col::ROOT].addr().unwrap(), [0u8; 20]);
        assert_eq!(cols[status_col::ROUTER].addr().unwrap(), [0u8; 20]);
        let mut seat = w(32).to_vec();
        for i in 0..13u64 {
            seat.extend_from_slice(&w(i + 1));
        }
        seat.extend_from_slice(&w(32 * 14));
        seat.extend_from_slice(&w(1));
        seat.extend_from_slice(&w(0xaa));
        let got = seat_answer(&seat).expect("往代 seatOf 读得成");
        assert_eq!(got.len(), 16);
        assert_eq!(got[12].small().unwrap(), 13, "amount 照名落位");
        assert_eq!((got[13].small().unwrap(), got[14].small().unwrap()), (0, 0), "两项入账往代没有,读作零");
        assert_eq!(got[15].items().unwrap().len(), 1, "parties 照名落位");
    }

    fn past_on(n: k7::Network) -> k7::StandardTable {
        k7::StandardTable { logic: "0x00000000000000000000000000000000000F1A57", shape: k7::Shape::Trial, ..n.standard }
    }

    #[test]
    fn a_past_generation_is_written_only_with_the_switch_on() {
        for n in k7::NETWORKS {
            k7::pretend_chain(Some(n.standard.chain_id));
            assert_eq!(k7::current_network().name, n.name);
            let cur = n.standard.logic;
            let past = past_on(n);
            let current = k2::derive::runtime(cur);
            let past_code = k2::derive::runtime(past.logic);
            assert!(matches!(judge(&current, cur, &[past]), Ok(k7::Generation::Current)));
            assert!(writable(judge(&current, cur, &[past]).unwrap()).is_ok());
            let g = judge(&past_code, cur, &[past]).expect("a past row reads");
            assert!(g.is_past() && g.row().logic == past.logic && g.shape() == k7::Shape::Trial);
            assert!(matches!(judge(&past_code, past.logic, &[past]), Ok(k7::Generation::Past(p)) if p.logic == past.logic));
            assert!(matches!(judge(&current, past.logic, &[past]), Ok(k7::Generation::Current)));
            k7::set_past_switch(false);
            let refused = writable(g).expect_err("开关关:往代案拒写");
            assert_eq!(refused.name(), crate::fault::PAST_CONTRACT);
            assert_eq!(refused.detail(), crate::fault::PAST_CONTRACT_SAYS);
            assert!(past_gate(true).is_err() && past_gate(false).is_ok());
            k7::set_past_switch(true);
            assert!(writable(g).is_ok(), "开关开:往代案照它那一代的形写");
            assert!(past_gate(true).is_ok());
            k7::set_past_switch(false);
            assert!(matches!(judge(&past_code, cur, &[]), Err(Fault::CodeHash { .. })));
            assert!(matches!(judge(b"\x60\x00", cur, &[past]), Err(Fault::CodeHash { .. })));
            assert!(matches!(judge(&[], cur, &[past]), Err(Fault::CodeHash { .. })));
            assert!(listing_writable(cur).is_ok() && listing_payable(cur).is_ok());
            for on in [false, true] {
                k7::set_past_switch(on);
                let no = payable_under(Some(g)).expect_err("往代清单不开新案");
                assert_eq!((no.name(), no.detail()), (crate::fault::PAST_LISTING.to_string(), crate::fault::PAST_LISTING_SAYS.to_string()));
            }
            k7::set_past_switch(false);
            assert!(payable_under(Some(k7::Generation::Current)).is_ok() && payable_under(None).is_ok());
        }
        k7::pretend_chain(None);
    }
}
