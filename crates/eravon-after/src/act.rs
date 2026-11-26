use eravon_cli::shared::Wire;
use eravon_dispute::view::CaseView;
use eravon_link as k3;
use eravon_standard as a1;
use eravon_terms as k2;
use eravon_terms::u256::U256;
use eravon_vault::session::Session;
use eravon_wallet::tx::{Keep, Kind, Tag};

use crate::fault::{Fault, R};

pub use eravon_cli::deskops::buyerpage::{Access, Case};

pub(crate) fn past_gate(c: &Case) -> R<()> {
    Ok(a1::chain::past_gate(c.past())?)
}

pub const PAST_LACKS: &str = "E_P6_PAST_LACKS";

pub use eravon_cli::deskops::buyerpage::PAST_LACKS_ACTS;

fn guard(c: &Case) -> R<()> {
    past_gate(c)?;
    if c.access != Access::Key {
        return Err(Fault::new("E_P6_READONLY", c.access.note()));
    }
    Ok(())
}


fn guard_any(c: &Case) -> R<()> {
    past_gate(c)?;
    if c.access == Access::Company {
        return Ok(());
    }
    guard(c)
}

fn chain(c: &Case) -> R<&CaseView> {
    c.chain.as_ref().filter(|v| v.phase != 0).ok_or_else(|| Fault::new("E_P6_NOT_OPEN", "订单尚未开启"))
}

#[derive(Clone, Copy)]
pub struct By<'a> {
    pub s: &'a Session,
    pub me: &'a str,
    pub writer: &'a str,
    pub still: &'a dyn Fn() -> bool,
}

fn post(w: &Wire, b: &By<'_>, from: &str, call: &a1::call::Call, tag: &Tag) -> R<String> {
    if !(b.still)() {
        return Err(Fault::new("E_P6_STALE", "情况已变,这一手没有执行"));
    }
    let key = || b.s.own_key(b.me, from).map_err(|e| eravon_cli::fault::Fault::Arg(format!("{}:{}", e.name(), e.detail())));
    let k = Keep { data: b.s.data(), writer: b.writer, owner: b.me };
    eravon_wallet::tx::post(&k, w, from, &key, &eravon_cli::send::Opts::default(), call, "0", tag).map_err(|f| Fault::new(f.token, f.detail))
}

fn then(done: &mut Vec<String>, step: R<String>) -> R<String> {
    match step {
        Ok(tx) => {
            if !tx.is_empty() {
                done.push(tx.clone());
            }
            Ok(tx)
        }
        Err(e) if done.is_empty() => Err(e),
        Err(e) => Err(Fault::new(e.token, format!("{};这一动作前面 {} 笔已上链:{}", e.detail, done.len(), done.join("、")))),
    }
}

fn as_buyer(w: &Wire, b: &By<'_>, c: &Case, direct: a1::call::Action, tag: &Tag) -> R<String> {
    acting(w, b, c)?;
    post(w, b, b.me, &direct.call(&c.instance)?, tag)
}

fn acting(w: &Wire, b: &By<'_>, c: &Case) -> R<()> {
    if acts(w, b, c)? {
        return Ok(());
    }
    Err(a1::fault::Fault::Precheck { gate: a1::precheck::GATE_CURRENT, why: format!("NotCurrent:本席钥 {} 不是为买家钥 {} 行事的钥", b.me, c.buyer_key) }.into())
}

fn acts(w: &Wire, b: &By<'_>, c: &Case) -> R<bool> {
    Ok(a1::chain::acting_key(&w.t, &w.table, &c.buyer_key, c.past())?.eq_ignore_ascii_case(b.me))
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfirmCard {
    pub amount: String,
    pub earns: bool,
    pub judge_fee: Option<String>,
    pub to_seller: Option<String>,
    pub to_fund: Option<String>,
}

pub fn confirm_card(w: &Wire, c: &Case) -> R<ConfirmCard> {
    crate::mark::mark(crate::mark::FEATURE);
    let v = chain(c)?;
    let now = a1::precheck::chain_now(&w.t, &w.table)?;
    let seat = v.seat_state.map(|state| a1::precheck::Seat { state, accepted: v.accepted != 0, window_end: v.window_end });
    let earns = a1::precheck::premium_to_judge(a1::precheck::Close::Confirm, v.phase, seat, now);
    let (judge_paid, seller_a, to_fund) = card_split(w, v, earns)?;
    Ok(ConfirmCard { amount: v.amount.to_dec(), earns, judge_fee: Some(judge_paid.to_dec()), to_seller: Some(seller_a.to_dec()), to_fund: Some(to_fund.to_dec()) })
}

fn card_split(w: &Wire, v: &CaseView, earns: bool) -> R<(U256, U256, U256)> {
    let dw = eravon_dispute::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let [_, seller_a, judge_paid, to_fund, _] = eravon_dispute::refund::split(&dw, &v.amount, 0, &v.premium, &v.deposit, &v.lf_held, earns, false, 0)
        .map_err(|(n, d)| Fault::new(n, d))?;
    Ok((judge_paid, seller_a, to_fund))
}


pub fn confirm(w: &Wire, b: &By<'_>, c: &Case) -> R<String> {
    crate::mark::mark(crate::mark::FEATURE);
    past_gate(c)?;
    let tag = Tag::of(Kind::Confirm, &c.instance);
    guard(c)?;
    chain(c)?;
    a1::chain::code_matches(&w.t, &w.table, &c.instance, &w.logic)?;
    as_buyer(w, b, c, a1::call::Action::Confirm, &tag)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Handed {
    pub to: String,
    pub data: String,
    pub sig: String,
    pub recorded: bool,
}

pub fn confirm_hand_out(w: &Wire, b: &By<'_>, c: &Case, now: u64) -> R<Handed> {
    crate::mark::mark(crate::mark::FEATURE);
    guard(c)?;
    chain(c)?;
    if !(b.still)() {
        return Err(Fault::new("E_P6_STALE", "情况已变,这一手没有执行"));
    }
    let d = k2::derive::digest(w.chain_id, &c.instance, &k2::derive::Action::Confirm.struct_hash());
    acting(w, b, c)?;
    let mut k = b.s.own_key(b.me, b.me)?;
    let sig = eravon_cli::shared::sign(&k, &d);
    for x in k.iter_mut() {
        *x = 0;
    }
    let _ = std::hint::black_box(&k);
    let sig = sig?;
    let call = a1::call::BySig::Confirm { sig: a1::order::bytes_of("buyerKey 签名", &sig)? }.call(&c.instance)?;
    let store = eravon_vault::store::open_write(b.s.data(), b.writer, now).map_err(eravon_cli::fault::Fault::from)?;
    let owners = eravon_vault::keyset::buyer(b.s, b.me).map_err(|e| Fault::new(e.name(), e.detail()))?.owners;
    let recorded = eravon_cli::buyer::record_confirm_in(&store, &owners, b.me, &c.instance, &sig, now)?;
    Ok(Handed { to: call.to.clone(), data: call.hex(), sig, recorded })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refund {
    pub lf: String,
    pub stray: String,
    pub need: String,
    pub rungs: Vec<(u64, u64, u64)>,
    pub lapse: Result<(u64, u64, u64), String>,
    pub pull: &'static str,
}

fn dw(w: &Wire) -> eravon_dispute::Wire<'_> {
    eravon_dispute::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries }
}

pub fn refund_view(w: &Wire, c: &Case) -> R<Refund> {
    let (lf, stray, need) = eravon_cli::after_company::refund_need(w, &c.instance, &c.case_id)?;
    let rungs: Vec<(u64, u64, u64)> = c
        .pack
        .as_ref()
        .and_then(|b| k3::pack::pack(b).ok())
        .map(|pk| pk.listing.order.ladder.iter().map(|r| (r.buyer_bps, r.seller_bps, r.fee_bearer)).collect())
        .unwrap_or_default();
    let pull = if need.is_zero() {
        "none"
    } else if a1::chain::permit_facts(&w.t, &w.table, &c.token, &c.buyer_key)?.is_some() {
        "permit"
    } else {
        "approve"
    };
    let past = c.chain.as_ref().is_some_and(|v| v.past);
    let lapse = lapse_split(w, c);
    if !past {
        held(w, c, &rungs, lapse.as_ref().ok().map(|(b, _, _)| *b))?;
    }
    Ok(Refund { lf: lf.to_dec(), stray: stray.to_dec(), need: need.to_dec(), rungs, pull, lapse })
}

fn held(w: &Wire, c: &Case, rungs: &[(u64, u64, u64)], lapse_bps: Option<u64>) -> R<()> {
    outcomes_hold(w, c, rungs)?;
    if let Some(b) = lapse_bps {
        lapse_holds(w, c, b)?;
    }
    Ok(())
}


fn outcomes_hold(w: &Wire, c: &Case, rungs: &[(u64, u64, u64)]) -> R<()> {
    if rungs.is_empty() {
        return Ok(());
    }
    let v = chain(c)?;
    let seat = v.seats.saturating_sub(1);
    let id = a1::order::b32_of("案号", &c.case_id)?;
    let s = a1::chain::seat_of(&w.t, &w.table, &c.instance, &id, seat)?.value;
    let state = s.get(eravon_dispute::seat_col::STATE).ok_or_else(|| Fault::new("E_A1_CODEHASH", "seatOf 缺 state"))?.small()?;
    let amount = U256::from_be(&s.get(eravon_dispute::seat_col::AMOUNT).ok_or_else(|| Fault::new("E_A1_CODEHASH", "seatOf 缺 amount"))?.word()?);
    let held = |i: usize, buyer_bps: u64| -> R<()> {
        let o = a1::chain::outcome_of(&w.t, &w.table, &c.instance, &id, seat, i as u64)?.value;
        let got = o.shares.first().map(|b| U256::from_be(b));
        let want = share_of(&amount, buyer_bps);
        if got.as_ref() != Some(&want) {
            return Err(Fault::new("E_A1_CODEHASH", format!("第 {i} 档:案上 outcomeOf 答买家得 {},订单文件的阶梯算出 {}", got.map(|g| g.to_dec()).unwrap_or_default(), want.to_dec())));
        }
        Ok(())
    };
    match ruling_kind(state) {
        Ruling::None => return Ok(()),
        Ruling::Ruled => {
            let Some(rung) = ruled_rung(w, c)? else { return Ok(()) };
            let (buyer_bps, _, _) = *rungs
                .get(rung as usize)
                .ok_or_else(|| Fault::new("E_A1_CODEHASH", format!("案上裁的是第 {rung} 档,订单文件的阶梯只有 {} 档", rungs.len())))?;
            return held(rung as usize, buyer_bps);
        }
        Ruling::Open => {}
    }
    a1::chain::ahead_outcomes(&w.t, &w.table, &c.instance, &id, seat, 0..rungs.len() as u64 + 1);
    for (i, (buyer_bps, _, _)) in rungs.iter().enumerate() {
        held(i, *buyer_bps)?;
    }
    if a1::chain::outcome_of(&w.t, &w.table, &c.instance, &id, seat, rungs.len() as u64).is_ok() {
        return Err(Fault::new("E_A1_CODEHASH", format!("案上的阶梯多于订单文件的 {} 档", rungs.len())));
    }
    Ok(())
}

fn lapse_holds(w: &Wire, c: &Case, buyer_bps: u64) -> R<()> {
    let v = chain(c)?;
    let seat = v.seats.saturating_sub(1);
    let id = a1::order::b32_of("案号", &c.case_id)?;
    let o = a1::chain::advance_of(&w.t, &w.table, &c.instance, &id, seat)?.value;
    if o.state != LAPSED_STATE {
        return Ok(());
    }
    let s = a1::chain::seat_of(&w.t, &w.table, &c.instance, &id, seat)?.value;
    let amount = U256::from_be(&s.get(eravon_dispute::seat_col::AMOUNT).ok_or_else(|| Fault::new("E_A1_CODEHASH", "seatOf 缺 amount"))?.word()?);
    let got = o.shares.first().map(|b| U256::from_be(b));
    let want = share_of(&amount, buyer_bps);
    if got.as_ref() != Some(&want) {
        return Err(Fault::new("E_A1_CODEHASH", format!("辞任失效:案上 advanceOf 答买家得 {},报价算出 {}", got.map(|g| g.to_dec()).unwrap_or_default(), want.to_dec())));
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
enum Ruling {
    Open,
    Ruled,
    None,
}

fn ruling_kind(state: u64) -> Ruling {
    match state {
        eravon_dispute::seat_col::STATE_SEATED | eravon_dispute::seat_col::STATE_RULING => Ruling::Open,
        eravon_dispute::seat_col::STATE_RULED => Ruling::Ruled,
        _ => Ruling::None,
    }
}

fn ruled_rung(w: &Wire, c: &Case) -> R<Option<u64>> {
    let (rows, _) = eravon_chain::read::logs(&w.t, &w.table, &c.instance, events_from(c), eravon_chain::seam::Upto::Latest).map_err(|e| Fault::new(e.name(), e.detail()))?;
    for r in &rows {
        let Ok(l) = eravon_scan::log::Log::read(r) else { continue };
        let Ok(d) = a1::event::decode(&l.topics, &l.data) else { continue };
        if d.name == "Ruled" {
            return Ok(d.args.first().and_then(|v| v.small().ok()));
        }
    }
    Ok(None)
}

fn events_from(c: &Case) -> u64 {
    c.chain.as_ref().and_then(|v| v.events.iter().find(|e| e.name == "Opened")).map_or(0, |e| eravon_chain::read::instance_from(e.block))
}

const LAPSED_STATE: u64 = 4;

fn share_of(amount: &U256, bps: u64) -> U256 {
    amount.mul_div_small(bps, 10_000).unwrap_or_else(|| amount.clone())
}

fn lapse_split(w: &Wire, c: &Case) -> Result<(u64, u64, u64), String> {
    let (rows, _) = eravon_chain::read::logs(&w.t, &w.table, &c.instance, events_from(c), eravon_chain::seam::Upto::Latest).map_err(|e| format!("{}:{}", e.name(), e.detail()))?;
    let t0 = a1::sol::OPENED.topic0();
    let opened = rows
        .iter()
        .filter_map(|r| eravon_scan::log::Log::read(r).ok())
        .find(|l| l.topics.first() == Some(&t0))
        .ok_or_else(|| format!("E_DISPUTE_NOT_OPEN:实例 {} 上没有开案(Opened)", c.instance))?;
    let (init, past) = match eravon_scan::scan::rebuild(&w.t, &w.table, &opened.tx, w.chain_id, &w.logic, &c.case_id).map_err(|e| format!("{}:{}", e.name(), e.detail()))? {
        eravon_scan::scan::Rebuild::Done(r) => (r.init, r.past),
        eravon_scan::scan::Rebuild::Untraced => return Err(format!("E_SCAN_UNTRACED:开案交易 {} 的 init 在内层调用,端点追踪不了", opened.tx)),
    };
    if past {
        return past_lapse_at(&init);
    }
    let q = k2::quote::decode(&init.quote).map_err(|f| format!("E_ACT_BAD_QUOTE:{:?}", f.token))?;
    lapse_at(q.lapse_default)
}

fn past_lapse_at(init: &a1::order::Init) -> Result<(u64, u64, u64), String> {
    let q = k2::quote::decode_under(eravon_literals::constants::Shape::Trial, &init.quote).map_err(|f| format!("E_ACT_BAD_QUOTE:{:?}", f.token))?;
    let ladder = &init.order.offer.ladder;
    let rung = if q.lapse_default == 0 { 0 } else { ladder.len().saturating_sub(1) };
    let r = ladder.get(rung).ok_or_else(|| "E_ACT_BAD_QUOTE:往代案的阶梯是空的".to_string())?;
    Ok((r.buyer_bps, r.seller_bps, r.fee_bearer))
}

fn lapse_at(lapse_default: u64) -> Result<(u64, u64, u64), String> {
    let b = u64::from(eravon_cands::lapse::share(lapse_default).ok_or_else(|| format!("E_QUOTE_FIELD 13 lapseDefault {lapse_default}"))?);
    Ok((b, 10_000 - b, 0))
}

pub fn dispute(w: &Wire, b: &By<'_>, c: &Case) -> R<String> {
    crate::mark::mark(crate::mark::FEATURE);
    past_gate(c)?;
    guard(c)?;
    chain(c)?;
    eravon_cli::buyer::refund_after(c.handed).map_err(|(n, d)| Fault::new(n, d))?;
    a1::chain::code_matches(&w.t, &w.table, &c.instance, &w.logic)?;
    eravon_dispute::refund::dispute_open(&dw(w), &c.instance, &c.case_id).map_err(|(n, d)| Fault::new(n, d))?;
    let fee = eravon_cli::after_company::refund_fee(w, &c.instance, &c.case_id, b.me).map_err(|(n, d)| Fault::new(n, d))?;
    let need = fee.need;
    acting(w, b, c)?;
    let mut done = Vec::new();
    if !need.is_zero() {
        then(&mut done, post(w, b, b.me, &a1::call::erc20_transfer(&fee.token, &c.instance, &need.to_be())?, &Tag::moving(Kind::Fee, &c.instance, &fee.token, &need.to_dec())))?;
    }
    then(&mut done, as_buyer(w, b, c, a1::call::Action::Dispute, &Tag::of(Kind::Dispute, &c.instance)))
}

pub fn dispute_pull(w: &Wire, b: &By<'_>, c: &Case) -> R<String> {
    crate::mark::mark(crate::mark::FEATURE);
    guard(c)?;
    chain(c)?;
    eravon_cli::buyer::refund_after(c.handed).map_err(|(n, d)| Fault::new(n, d))?;
    a1::chain::code_matches(&w.t, &w.table, &c.instance, &w.logic)?;
    eravon_dispute::refund::dispute_open(&dw(w), &c.instance, &c.case_id).map_err(|(n, d)| Fault::new(n, d))?;
    let (fee, frozen) = eravon_cli::after_company::pull_fee(w, &c.instance, &c.case_id, b.me).map_err(|(n, d)| Fault::new(n, d))?;
    let need = fee.need;
    let now = a1::precheck::chain_now(&w.t, &w.table)?;
    let mut k = b.s.own_key(b.me, b.me)?;
    let got = eravon_cli::buyer::pull_calls(w, &k, &frozen, &c.instance, &fee.token, &need, now);
    for x in k.iter_mut() {
        *x = 0;
    }
    let _ = std::hint::black_box(&k);
    let p = got?;
    let mut done: Vec<String> = Vec::new();
    let mut last = String::new();
    for (kind, call) in &p.calls {
        let tag = match *kind {
            "approve" | "permit" => Tag::moving(Kind::Approve, &c.instance, &c.token, &need.to_dec()),
            "reset" => Tag::moving(Kind::Approve, &c.instance, &c.token, "0"),
            _ if p.way == "permit" => Tag::moving(Kind::Dispute, &c.instance, &c.token, &need.to_dec()),
            _ => Tag::of(Kind::Dispute, &c.instance),
        };
        last = then(&mut done, post(w, b, b.me, call, &tag))?;
    }
    Ok(last)
}

pub fn commit(w: &Wire, b: &By<'_>, c: &Case, items: Vec<eravon_evidence::sheet::FileItem>, source: String, note: Option<String>) -> R<(String, eravon_dispute::evidence::Evidence)> {
    crate::mark::mark(crate::mark::FEATURE);
    guard(c)?;
    chain(c)?;
    let ev = eravon_dispute::evidence::sheet_for(items, source, &c.buyer_key, &c.instance, &c.case_id, w.chain_id, &w.logic, note)?;
    a1::chain::code_matches(&w.t, &w.table, &c.instance, &w.logic)?;
    let case_id = a1::order::b32_of("案号", &c.case_id)?;
    let tx = post(w, b, b.me, &a1::call::Action::Commit { case_id, manifest: ev.manifest }.call(&c.instance)?, &Tag::of(Kind::Commit, &c.instance))?;
    Ok((tx, ev))
}

pub fn redraw(w: &Wire, b: &By<'_>, c: &Case) -> R<Option<String>> {
    crate::mark::mark(crate::mark::FEATURE);
    past_gate(c)?;
    if c.access == Access::ByCall {
        return Ok(None);
    }
    chain(c)?;
    let ww = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let Some(call) = eravon_watch::pool::draw_due(&ww, &c.instance).map_err(|(n, d)| Fault::new(n, d))? else { return Ok(None) };
    if !(b.still)() {
        return Err(Fault::new("E_P6_STALE", "情况已变,这一手没有执行"));
    }
    let key = || b.s.own_key(b.me, b.me).map_err(|e| eravon_cli::fault::Fault::Arg(format!("{}:{}", e.name(), e.detail())));
    let d = eravon_cli::send::deliver_key(&key, &eravon_cli::send::Opts::default(), w, b.me, &call, "0").map_err(|f| Fault::new(f.name(), f.detail()))?;
    d.outcome().map(Some).map_err(|(n, dd)| Fault::new(n, dd))
}

pub fn advance(w: &Wire, b: &By<'_>, c: &Case) -> R<String> {
    crate::mark::mark(crate::mark::FEATURE);
    guard_any(c)?;
    let v = chain(c)?;
    a1::chain::code_matches(&w.t, &w.table, &c.instance, &w.logic)?;
    let case_id = a1::order::b32_of("案号", &c.case_id)?;
    let seat = v.seats.saturating_sub(1);
    if !due(w, &c.instance, &case_id, seat)?.moves() {
        return Err(Fault::new(eravon_watch::pool::ADVANCE_NOT_DUE, eravon_watch::pool::ADVANCE_NOT_DUE_SAYS));
    }
    post(w, b, b.me, &a1::call::Action::Advance { case_id, seat }.call(&c.instance)?, &Tag::of(Kind::Advance, &c.instance))
}

pub fn due(w: &Wire, instance: &str, case_id: &[u8; 32], seat: u64) -> R<eravon_watch::pool::Due> {
    let ww = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    eravon_watch::pool::advance_due(&ww, instance, case_id, seat).map_err(|(n, d)| Fault::new(n, d))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdvanceCard {
    pub due: eravon_watch::pool::Due,
    pub shares: Option<(String, String, String)>,
}

pub fn advance_card(w: &Wire, c: &Case) -> R<AdvanceCard> {
    let v = chain(c)?;
    let case_id = a1::order::b32_of("案号", &c.case_id)?;
    let d = due(w, &c.instance, &case_id, v.seats.saturating_sub(1))?;
    let dw = eravon_dispute::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let closed = |buyer_bps: u16| -> R<(String, String, String)> {
        let [buyer, seller, judge, _, _] = eravon_dispute::refund::split(&dw, &v.amount, buyer_bps, &v.premium, &v.deposit, &v.lf_held, false, false, 0).map_err(|(n, d)| Fault::new(n, d))?;
        Ok((buyer.to_dec(), seller.to_dec(), judge.to_dec()))
    };
    let shares = match &d {
        eravon_watch::pool::Due::Seat(o) => {
            let word = |i: usize| o.shares.get(i).map(|b| U256::from_be(b)).unwrap_or(U256::ZERO);
            let judge = U256::from_be(&o.standby_fee).overflowing_add(&U256::from_be(&o.ruling_fee)).0;
            Some((word(0).to_dec(), word(1).to_dec(), judge.to_dec()))
        }
        eravon_watch::pool::Due::Resigned => Some(closed(0)?),
        eravon_watch::pool::Due::Draw => Some(closed(10_000)?),
        eravon_watch::pool::Due::No | eravon_watch::pool::Due::Past => None,
    };
    Ok(AdvanceCard { due: d, shares })
}

#[derive(Clone, Debug)]
pub struct Offer {
    pub st: k3::settle::Settle,
    pub digest: [u8; 32],
    pub seller_by_call: bool,
}

fn posted(w: &Wire, c: &Case, link: &[u8]) -> R<Vec<u8>> {
    let l = k3::link::link(link)?;
    if let Some(why) = k3::link::other_chain(&l, w.chain_id) {
        return Err(Fault::new(k3::link::E_LINK_OTHER_CHAIN, why));
    }
    let hash = a1::order::b32_of("链接的 hash", &l.hash)?;
    let case = a1::order::b32_of("案号", &c.case_id)?;
    let posts = eravon_scan::board::by_tx(&w.t, &w.table, &l.tx, &hash)?;
    posts
        .into_iter()
        .find(|p| p.key == case)
        .map(|p| p.document)
        .ok_or_else(|| Fault::new("E_P6_SETTLE_UNPOSTED", format!("交易 {} 的回执里没有以本案案号为 key 的这份和解要约", l.tx)))
}

pub fn offers(w: &Wire, c: &Case) -> R<Vec<(String, k3::settle::Settle, Vec<u8>)>> {
    crate::mark::mark(crate::mark::FEATURE);
    let (case_id, seller, _, block) = eravon_dispute::settle::opened(&dw(w), &c.instance).map_err(|(n, d)| Fault::new(n, d))?;
    let case = a1::order::b32_of("案号", &case_id)?;
    let got = eravon_scan::board::settles_by_case(&w.t, &w.table, &case, w.chain_id, &c.instance, &seller, block)?;
    Ok(got.into_iter().map(|(p, st)| (p.tx, st, p.document)).collect())
}

pub fn offer(w: &Wire, c: &Case, text: &str) -> R<Offer> {
    crate::mark::mark(crate::mark::FEATURE);
    let (link, _) = k3::transport::decode_link(text.trim().as_bytes())?;
    let bytes = posted(w, c, &link)?;
    let st = match k3::link::resolve(&link, &bytes).map_err(|v| Fault::new(v.name, "链接与它所指的文档不符"))? {
        k3::link::Resolved::Settle(st) => st,
        k3::link::Resolved::Listing(_) => return Err(Fault::new("E_P6_SETTLE_FORM", "这条链接指向清单,不是和解要约")),
    };
    offer_of(w, c, st)
}

pub fn offer_of(w: &Wire, c: &Case, st: k3::settle::Settle) -> R<Offer> {
    let k = eravon_cli::deskops::settle::consent(w, &c.instance, c.pack.as_deref(), &c.seller, &st)?;
    Ok(Offer { st, digest: k.digest, seller_by_call: k.seller_by_call })
}

pub fn settle(w: &Wire, b: &By<'_>, c: &Case, o: &Offer, by_call: bool) -> R<String> {
    crate::mark::mark(crate::mark::FEATURE);
    past_gate(c)?;
    let s = eravon_cli::deskops::settle::Settlement { instance: &c.instance, buyer_key: &c.buyer_key, offer: &o.st, digest: &o.digest };
    let tag = Tag::of(Kind::Settle, &c.instance);
    guard(c)?;
    chain(c)?;
    acting(w, b, c)?;
    let s = eravon_cli::deskops::settle::Settlement { buyer_key: b.me, ..s };
    let key_for = |k: &str| b.s.own_key(b.me, k).map_err(Fault::from);
    eravon_cli::deskops::settle::submit(w, &s, b.me, true, by_call, &key_for, |from, call| post(w, b, from, call, &tag))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cancelled {
    pub deploy: String,
    pub cancel: String,
}

pub fn cancel(w: &Wire, b: &By<'_>, c: &Case) -> R<Cancelled> {
    crate::mark::mark(crate::mark::FEATURE);
    past_gate(c)?;
    let pk = k3::pack::pack(c.pack.as_ref().ok_or_else(|| Fault::new("E_P6_NO_PACK", "请先导入订单文件"))?)?;
    let o = eravon_cli::shared::order_of(pk.listing.terms_text.as_bytes(), pk.receipt.candidate, &pk.receipt.buyer_key, &pk.receipt.buyer_payout, pk.receipt.open_by, &pk.receipt.prev)?;
    let logic = pk.listing.logic.clone();
    let k = eravon_cli::buyer::cancel_calls(w, &o, &logic)?;
    let tag = Tag::of(Kind::Cancel, &c.instance);
    if c.access != Access::Key {
        return Err(Fault::new("E_P6_READONLY", c.access.note()));
    }
    acting(w, b, c)?;
    let mut done = Vec::new();
    let deploy = match &k.deploy {
        Some(dep) => then(&mut done, post(w, b, b.me, dep, &tag))?,
        None => {
            a1::chain::code_matches(&w.t, &w.table, &c.instance, &logic)?;
            String::new()
        }
    };
    let cancel = match then(&mut done, post(w, b, b.me, &k.cancel, &tag)) {
        Err(e) if e.token == "E_V_REVERTED" || e.token == "E_TX_STATUS" => {
            let lost = eravon_cli::buyer::cancel_lost(w, &k.instance, eravon_cli::fault::Fault::Arg(e.detail.clone()));
            return Err(if lost.name() == "E_CANCEL_OPENED" { lost.into() } else { e });
        }
        r => r?,
    };
    Ok(Cancelled { deploy, cancel })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expired {
    pub deploy: String,
    pub init: String,
    pub amount: String,
    pub token: String,
    pub payout: String,
}

pub fn expire(w: &Wire, b: &By<'_>, c: &Case) -> R<Expired> {
    crate::mark::mark(crate::mark::FEATURE);
    past_gate(c)?;
    company_proposes(c)?;
    if c.chain.as_ref().is_some_and(|v| v.phase != 0) {
        return Err(Fault::new("E_P6_OPENED", "订单已开启,不能过期退款"));
    }
    let pk = k3::pack::pack(c.pack.as_ref().ok_or_else(|| Fault::new("E_P6_NO_PACK", "请先导入订单文件"))?)?;
    let held = a1::chain::erc20_balance(&w.t, &w.table, &c.token, &c.instance)?.value;
    if !held.iter().any(|b| *b != 0) {
        return Err(Fault::new(E_P6_NOTHING, "预算地址上已没有这一单的款,没有可退回的"));
    }
    let o = eravon_cli::shared::order_of(pk.listing.terms_text.as_bytes(), pk.receipt.candidate, &pk.receipt.buyer_key, &pk.receipt.buyer_payout, pk.receipt.open_by, &pk.receipt.prev)?;
    let now = a1::precheck::chain_now(&w.t, &w.table)?;
    let logic = pk.listing.logic.clone();
    let (_, deploy, init) = eravon_cli::seller::expire_calls(w, &o, &logic, now)?;
    let mut done = Vec::new();
    let t1 = match &deploy {
        Some(dep) => then(&mut done, post(w, b, b.me, dep, &Tag::of(Kind::Expire, &c.instance)))?,
        None => {
            a1::chain::code_matches(&w.t, &w.table, &c.instance, &logic)?;
            String::new()
        }
    };
    let t2 = then(&mut done, expire_init(w, b, &init, &c.instance))?;
    Ok(Expired { deploy: t1, init: t2, amount: k2::u256::U256::from_be(&held).to_dec(), token: c.token.clone(), payout: pk.receipt.buyer_payout.to_ascii_lowercase() })
}

fn company_proposes(c: &Case) -> R<()> {
    if c.access == Access::Company {
        return Err(Fault::new("E_P6_READONLY", c.access.note()));
    }
    Ok(())
}

pub(crate) use eravon_cli::deskops::buyerpage::unopened;

pub use eravon_cli::deskops::buyerpage::{take_from, Take};

pub fn take(w: &Wire, c: &Case, now: u64) -> R<Take> {
    let how = closed(w, c, now)?;
    Ok(take_from(how, how.is_some() && funded(w, c)?))
}

pub fn closed(w: &Wire, c: &Case, now: u64) -> R<Option<Take>> {
    Ok(eravon_cli::deskops::buyerpage::closed(w, c, now)?)
}

fn funded(w: &Wire, c: &Case) -> R<bool> {
    Ok(a1::chain::erc20_balance(&w.t, &w.table, &c.token, &c.instance)?.value.iter().any(|b| *b != 0))
}

pub const E_P6_NOTHING: &str = "E_P6_NOTHING";

fn expire_init(w: &Wire, b: &By<'_>, init: &a1::call::Call, instance: &str) -> R<String> {
    post(w, b, b.me, init, &Tag::of(Kind::Expire, instance))
}


pub use eravon_cli::after_company::Owed;

pub fn owed(w: &Wire, c: &Case, token: Option<&str>) -> R<Owed> {
    eravon_cli::after_company::owed(w, &c.instance, &c.token, c.chain.as_ref(), c.pack.as_deref(), token).map_err(|(n, d)| Fault::new(n, d))
}

pub fn sweep(w: &Wire, b: &By<'_>, c: &Case, token: Option<&str>) -> R<(String, Owed)> {
    crate::mark::mark(crate::mark::FEATURE);
    past_gate(c)?;
    company_proposes(c)?;
    let o = eravon_cli::after_company::sweep_due(w, &c.instance, &c.token, c.chain.as_ref(), c.pack.as_deref(), token).map_err(|(n, d)| Fault::new(n, d))?;
    a1::chain::code_matches(&w.t, &w.table, &c.instance, &w.logic)?;
    let tx = post(w, b, b.me, &a1::call::Action::Sweep { who: o.payout.clone(), token: o.token.clone() }.call(&c.instance)?, &Tag::of(Kind::Sweep, &c.instance))?;
    Ok((tx, o))
}

pub fn export_gate(c: &Case) -> Vec<&'static str> {
    match c.pack.as_ref().map(|b| k3::pack::pack(b)) {
        Some(Ok(_)) => Vec::new(),
        _ => vec!["上板记录", "清单", "回执"],
    }
}

#[cfg(test)]
mod tests {
    use super::{ruling_kind, Ruling};

    #[test]
    fn nothing_left_is_nothing_to_take() {
        use super::{take_from, Take};
        for how in [Take::Lapsed, Take::Revoked] {
            assert_eq!(take_from(Some(how), true), how);
            assert_eq!(take_from(Some(how), false), Take::No);
        }
        assert_eq!(take_from(None, true), Take::No);
        assert_eq!(take_from(None, false), Take::No);
    }

    #[test]
    fn the_ladder_is_held_to_what_the_seat_still_answers() {
        assert_eq!(ruling_kind(0), Ruling::Open);
        assert_eq!(ruling_kind(1), Ruling::Open);
        assert_eq!(ruling_kind(2), Ruling::Ruled);
        for closed in 3..=5 {
            assert_eq!(ruling_kind(closed), Ruling::None);
        }
    }


    use super::*;

    #[test]
    fn an_unruled_case_lapses_at_the_share_its_quote_names() {
        assert_eq!(lapse_at(0), Ok((0, 10000, 0)));
        assert_eq!(lapse_at(1), Ok((5000, 5000, 0)));
        assert_eq!(lapse_at(2), Ok((10000, 0, 0)));
        assert!(lapse_at(3).unwrap_err().starts_with("E_QUOTE_FIELD"));
    }

    fn view(past: bool) -> CaseView {
        let z = || U256::from_be(&[0u8; 32]);
        CaseView {
            phase: 3,
            seats: 1,
            premium: z(),
            deposit: z(),
            lf_held: z(),
            pending_redraw: false,
            redraw_deadline: 0,
            judge: String::new(),
            accepted: 0,
            window_end: 0,
            seat_state: Some(1),
            amount: z(),
            token: String::new(),
            buyer_payout: String::new(),
            seller_fund: String::new(),
            seller_payout: String::new(),
            lf: z(),
            events: Vec::new(),
            past,
            logic: String::new(),
        }
    }

    fn case(access: Access, past: bool) -> Case {
        Case { instance: String::new(), case_id: String::new(), buyer_key: String::new(), token: String::new(), seller: String::new(), access, pack: None, chain: Some(view(past)), handed: false }
    }

    #[test]
    fn a_case_of_an_older_contract_is_written_only_with_the_switch_on() {
        eravon_literals::constants::set_past_switch(false);
        for a in [Access::Key, Access::Company, Access::ByCall] {
            let c = case(a, true);
            assert!(c.past());
            let f = past_gate(&c).expect_err("past");
            assert_eq!((f.token.as_str(), f.detail.as_str()), (a1::fault::PAST_CONTRACT, a1::fault::PAST_CONTRACT_SAYS));
            assert_eq!(guard(&c).expect_err("past").token, a1::fault::PAST_CONTRACT);
            assert_eq!(guard_any(&c).expect_err("past").token, a1::fault::PAST_CONTRACT);
        }
        eravon_literals::constants::set_past_switch(true);
        let c = case(Access::Key, true);
        assert!(past_gate(&c).is_ok());
        eravon_literals::constants::set_past_switch(false);
        let now = case(Access::Key, false);
        assert!(!now.past() && past_gate(&now).is_ok() && guard(&now).is_ok());
    }
}
