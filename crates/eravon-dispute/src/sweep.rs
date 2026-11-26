use eravon_standard as a1;
use eravon_watch::order::{self, Order};
use eravon_watch::send::Post;

use crate::{Why, Wire, a1w, case_of, small, status, why};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Swept {
    Sent(String),
    Skipped(String),
    Nothing(String),
}

fn registered(payout: &str) -> bool {
    !payout.trim_start_matches("0x").bytes().all(|c| c == b'0')
}


pub enum Due {
    Call(a1::call::Call),
    Done(Swept),
}

pub fn due(data: &str, writer: &str, now: u64, me: &str, w: &Wire<'_>, o: &Order) -> Result<Due, Why> {
    if let Some(tx) = o.extra("sweepTx") {
        return Ok(Due::Done(Swept::Nothing(format!("已为判官推送过付款({tx})"))));
    }
    if let Some(p) = o.extra("sweepProposal") {
        return Ok(Due::Done(Swept::Nothing(format!("推付提案已导出({p})"))));
    }
    let st = status(w, &o.instance)?;
    let seats = small(&st, a1::chain::status_col::SEATS)?;
    let seat = a1::chain::seat_of(w.t, w.table, &o.instance, &case_of(o)?, seats.saturating_sub(1)).map_err(a1w)?.value;
    let judge = a1::order::hex_of(&seat.first().ok_or_else(|| why("E_DISPUTE_READ", "seatOf 没有答 judge"))?.addr().map_err(a1w)?);
    let token = seat.get(crate::seat_col::TOKEN).ok_or_else(|| why("E_DISPUTE_READ", "seatOf 没有答 token"))?.addr().map_err(a1w)?;
    let token = a1::order::hex_of(&token);
    let acct = a1::chain::account_of(w.t, w.table, &o.instance, &judge, &token).map_err(a1w)?.value;
    if !registered(&acct.0) {
        a1::chain::code_matches(w.t, w.table, &o.instance, w.logic).map_err(a1w)?;
        let mut o2 = order::load(data, me, &o.id)?;
        let note = format!("判官 {judge} 未登记收款,已跳过推付");
        o2.set_extra("sweepNote", &note);
        order::save(data, writer, now, me, &o2, &[])?;
        return Ok(Due::Done(Swept::Skipped(note)));
    }
    if acct.1 == [0u8; 32] {
        return Ok(Due::Done(Swept::Nothing(format!("判官 {judge} 没有待付款项"))));
    }
    a1::chain::code_matches(w.t, w.table, &o.instance, w.logic).map_err(a1w)?;
    Ok(Due::Call(a1::call::Action::Sweep { who: judge, token }.call(&o.instance).map_err(a1w)?))
}

pub fn judge(data: &str, writer: &str, now: u64, me: &str, w: &Wire<'_>, o: &Order, post: &mut dyn Post) -> Result<Swept, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let call = match due(data, writer, now, me, w, o)? {
        Due::Done(s) => return Ok(s),
        Due::Call(c) => c,
    };
    let tx = post.post(&call)?;
    let mut o2 = order::load(data, me, &o.id)?;
    o2.set_extra("sweepTx", &tx);
    let local = order::save(data, writer, now, me, &o2, &[]);
    eravon_watch::send::after_send(tx, local).map(Swept::Sent)
}

pub fn company(data: &str, writer: &str, now: u64, me: &str, w: &Wire<'_>, o: &Order) -> Result<Result<a1::proposal::Plan, Swept>, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let call = match due(data, writer, now, me, w, o)? {
        Due::Done(s) => return Ok(Err(s)),
        Due::Call(c) => c,
    };
    Ok(Ok(a1::proposal::Plan {
        verb: "sweep",
        calls: vec![a1::proposal::of_call(&call).map_err(a1w)?],
        originals: vec![a1::proposal::abi_doc(&[a1::sol::SWEEP], a1::proposal::INSTANCE_ABI_NAME).map_err(a1w)?],
        items: Vec::new(),
        face: Vec::new(),
        instance: o.instance.clone(),
    }))
}
