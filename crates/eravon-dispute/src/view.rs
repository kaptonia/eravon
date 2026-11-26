use eravon_standard as a1;
use eravon_terms::u256::U256;

use a1::chain::status_col as col;

use crate::seat_col;
use crate::{Why, Wire, a1w, big, small, why};

#[derive(Clone, Debug)]
pub struct Ev {
    pub name: String,
    pub block: u64,
    pub log_index: u64,
    pub tx: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct CaseView {
    pub phase: u64,
    pub seats: u64,
    pub premium: U256,
    pub deposit: U256,
    pub lf_held: U256,
    pub pending_redraw: bool,
    pub redraw_deadline: u64,
    pub judge: String,
    pub accepted: u64,
    pub window_end: u64,
    pub seat_state: Option<u64>,
    pub amount: U256,
    pub token: String,
    pub buyer_payout: String,
    pub seller_fund: String,
    pub seller_payout: String,
    pub lf: U256,
    pub events: Vec<Ev>,
    pub past: bool,
    pub logic: String,
}

fn arg(v: &a1::abi::Val) -> String {
    match v {
        a1::abi::Val::Addr(a) => a1::order::hex_of(a),
        a1::abi::Val::U256(w) => U256::from_be(w).to_dec(),
        a1::abi::Val::B32(w) => a1::order::hex_of(w),
        a1::abi::Val::Bool(b) => b.to_string(),
        a1::abi::Val::Str(s) => s.clone(),
        a1::abi::Val::Bytes(b) => a1::order::hex_of(b),
        a1::abi::Val::Arr(_) | a1::abi::Val::Tuple(_) => String::new(),
    }
}

pub fn events(w: &Wire<'_>, instance: &str, opened: u64) -> Result<Vec<Ev>, Why> {
    let (rows, _) = eravon_chain::read::logs(w.t, w.table, instance, eravon_chain::read::instance_from(opened), eravon_chain::seam::Upto::Latest)
        .map_err(|e| why(e.name(), e.detail()))?;
    let mut out = Vec::new();
    for row in &rows {
        let l = eravon_scan::log::Log::read(row).map_err(|e| why(&e.name(), e.detail()))?;
        let Ok(d) = a1::event::decode(&l.topics, &l.data) else {
            continue;
        };
        out.push(Ev { name: d.name.to_string(), block: l.block, log_index: l.log_index, tx: l.tx.clone(), args: d.args.iter().map(arg).collect() });
    }
    out.sort_by(|a, b| a.block.cmp(&b.block).then(a.log_index.cmp(&b.log_index)));
    Ok(out)
}

pub fn case(w: &Wire<'_>, instance: &str, case_id: &str, opened: u64) -> Result<CaseView, Why> {
    case_with(w, instance, case_id, opened, true)
}

fn case_with(w: &Wire<'_>, instance: &str, case_id: &str, opened: u64, with_events: bool) -> Result<CaseView, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let g = a1::chain::code_generation(w.t, w.table, instance, w.logic).map_err(a1w)?.value;
    case_of(w, instance, case_id, opened, with_events, g)
}

fn case_of(w: &Wire<'_>, instance: &str, case_id: &str, opened: u64, with_events: bool, g: eravon_literals::constants::Generation) -> Result<CaseView, Why> {
    let mut v = read_with(w, instance, case_id, opened, with_events)?;
    v.past = g.is_past();
    if v.past {
        v.logic = g.row().logic.to_string();
    }
    Ok(v)
}

pub fn case_if_deployed(w: &Wire<'_>, instance: &str, case_id: &str, opened: u64) -> Result<Option<CaseView>, Why> {
    deployed(w, instance, case_id, opened, true)
}

fn deployed(w: &Wire<'_>, instance: &str, case_id: &str, opened: u64, with_events: bool) -> Result<Option<CaseView>, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let (code, _) = eravon_chain::read::code(w.t, w.table, instance, eravon_chain::seam::TAG_LATEST)
        .map_err(|e| why(e.name(), e.detail()))?;
    if code.is_empty() {
        return Ok(None);
    }
    crate::mark::mark(crate::mark::FEATURE);
    let g = a1::chain::generation_of_code(&code, w.logic).map_err(a1w)?;
    case_of(w, instance, case_id, opened, with_events, g).map(Some)
}


pub fn events_of_many(w: &Wire<'_>, instances: &[(String, u64)]) -> Result<Vec<Vec<Ev>>, Why> {
    let mut out: Vec<Vec<Ev>> = vec![Vec::new(); instances.len()];
    let mut order: Vec<usize> = (0..instances.len()).collect();
    order.sort_by(|a, b| instances[*a].1.cmp(&instances[*b].1).then_with(|| instances[*a].0.to_ascii_lowercase().cmp(&instances[*b].0.to_ascii_lowercase())));
    let reads: Vec<(Vec<String>, u64, eravon_chain::seam::Upto)> = order
        .chunks(eravon_chain::seam::FILTER_FAN)
        .map(|part| (part.iter().map(|i| instances[*i].0.to_ascii_lowercase()).collect(), part.iter().map(|i| eravon_chain::read::instance_from(instances[*i].1)).min().unwrap_or(0), eravon_chain::seam::Upto::Latest))
        .collect();
    eravon_chain::read::logs_at_any_ahead(w.t, w.table, &reads);
    for part in order.chunks(eravon_chain::seam::FILTER_FAN) {
        let addrs: Vec<String> = part.iter().map(|i| instances[*i].0.to_ascii_lowercase()).collect();
        let from = part.iter().map(|i| eravon_chain::read::instance_from(instances[*i].1)).min().unwrap_or(0);
        let (rows, _) = eravon_chain::read::logs_at_any(w.t, w.table, &addrs, from, eravon_chain::seam::Upto::Latest).map_err(|e| why(e.name(), e.detail()))?;
        for row in &rows {
            let l = eravon_scan::log::Log::read(row).map_err(|e| why(&e.name(), e.detail()))?;
            let Some(i) = part.iter().copied().find(|i| instances[*i].0.eq_ignore_ascii_case(&l.address) && l.block >= eravon_chain::read::instance_from(instances[*i].1)) else {
                continue;
            };
            let Ok(d) = a1::event::decode(&l.topics, &l.data) else {
                continue;
            };
            out[i].push(Ev { name: d.name.to_string(), block: l.block, log_index: l.log_index, tx: l.tx.clone(), args: d.args.iter().map(arg).collect() });
        }
    }
    for evs in &mut out {
        evs.sort_by(|a, b| a.block.cmp(&b.block).then(a.log_index.cmp(&b.log_index)));
    }
    Ok(out)
}

pub fn ahead_cases(w: &Wire<'_>, cases: &[(String, String)]) {
    if cases.len() < 2 {
        return;
    }
    let instances: Vec<String> = cases.iter().map(|(i, _)| i.clone()).collect();
    eravon_chain::read::together(w.t, w.table, || {
        eravon_chain::read::ahead_codes(w.t, w.table, &instances, eravon_chain::seam::TAG_LATEST);
        a1::chain::ahead_status_and_fee(w.t, w.table, &instances);
    });
    if !eravon_chain::read::holds_ahead(w.table) {
        return;
    }
    let mut seats: Vec<(String, [u8; 32], u64)> = Vec::new();
    for (i, c) in cases {
        let Ok((st, _)) = a1::chain::status_and_fee(w.t, w.table, i) else { continue };
        let (Ok(n), Ok(case)) = (small(&st.value, col::SEATS), a1::order::b32_of("案号", c)) else { continue };
        if n > 0 {
            seats.push((i.clone(), case, n - 1));
        }
    }
    a1::chain::ahead_seats(w.t, w.table, &seats);
}

pub fn case_if_deployed_with(w: &Wire<'_>, instance: &str, case_id: &str, opened: u64, events: Option<Vec<Ev>>) -> Result<Option<CaseView>, Why> {
    let Some(mut v) = deployed(w, instance, case_id, opened, events.is_none())? else { return Ok(None) };
    if let Some(evs) = events {
        v.events = evs;
    }
    Ok(Some(v))
}

fn read_with(w: &Wire<'_>, instance: &str, case_id: &str, opened: u64, with_events: bool) -> Result<CaseView, Why> {
    let (st, fq) = a1::chain::status_and_fee(w.t, w.table, instance).map_err(a1w)?;
    let (st, fq) = (st.value, fq.value);
    let seats = small(&st, col::SEATS)?;
    let none = || (String::new(), 0, 0, None, U256::from_be(&[0u8; 32]), String::new(), [String::new(), String::new(), String::new()]);
    let (judge, accepted, window_end, seat_state, amount, token, [buyer_payout, seller_fund, seller_payout]) = if seats == 0 {
        none()
    } else {
        let case = a1::order::b32_of("案号", case_id).map_err(a1w)?;
        let seat = a1::chain::seat_of(w.t, w.table, instance, &case, seats - 1).map_err(a1w)?.value;
        let addr = |i: usize| -> Result<String, Why> {
            Ok(a1::order::hex_of(&seat.get(i).ok_or_else(|| why("E_DISPUTE_READ", format!("seatOf 第 {i} 栏缺")))?.addr().map_err(a1w)?))
        };
        let parties = seat.get(seat_col::PARTIES).ok_or_else(|| why("E_DISPUTE_READ", "seatOf 没有答当事人表"))?.items().map_err(a1w)?;
        let party = |i: usize, what: &str| -> Result<String, Why> {
            Ok(a1::order::hex_of(&parties.get(i).ok_or_else(|| why("E_DISPUTE_READ", format!("seatOf 当事人表缺 {what}")))?.addr().map_err(a1w)?))
        };
        let ps = [party(seat_col::PARTY_BUYER_PAYOUT, "buyerPayout")?, party(seat_col::PARTY_SELLER_FUND, "sellerFund")?, party(seat_col::PARTY_SELLER_PAYOUT, "sellerPayout")?];
        (
            addr(seat_col::JUDGE)?,
            small(&seat, seat_col::ACCEPTED)?,
            small(&seat, seat_col::WINDOW_END)?,
            Some(small(&seat, seat_col::STATE)?),
            big(&seat, seat_col::AMOUNT)?,
            addr(seat_col::TOKEN)?,
            ps,
        )
    };
    let pending = st.get(col::PENDING_REDRAW).ok_or_else(|| why("E_DISPUTE_READ", "status 没有答 pendingRedraw"))?.boolean().map_err(a1w)?;
    Ok(CaseView {
        phase: small(&st, col::PHASE)?,
        seats,
        premium: big(&st, col::PREMIUM)?,
        deposit: big(&st, col::DEPOSIT)?,
        lf_held: big(&st, col::LF_HELD)?,
        pending_redraw: pending,
        redraw_deadline: small(&st, col::REDRAW_DEADLINE)?,
        judge,
        accepted,
        window_end,
        seat_state,
        amount,
        token,
        buyer_payout,
        seller_fund,
        seller_payout,
        lf: big(&fq, 0)?,
        events: if with_events { events(w, instance, opened)? } else { Vec::new() },
        past: false,
        logic: w.logic.to_string(),
    })
}
