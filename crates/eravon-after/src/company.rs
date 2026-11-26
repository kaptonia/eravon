use eravon_cli::after_company as co;
use eravon_cli::shared::Wire;
use eravon_standard::proposal::Plan;

use eravon_standard as a1;

use crate::act::{Access, Case, Offer};
use crate::fault::{Fault, R};

pub use co::Form;

fn company(c: &Case) -> R<&eravon_dispute::view::CaseView> {
    if c.access != Access::Company {
        return Err(Fault::new("E_P6_NOT_COMPANY", "该订单不属于本公司"));
    }
    c.chain.as_ref().filter(|v| v.phase != 0).ok_or_else(|| Fault::new("E_P6_NOT_OPEN", "订单尚未开启"))
}

fn at<'a>(c: &'a Case, v: &eravon_dispute::view::CaseView) -> co::At<'a> {
    co::At { instance: &c.instance, case_id: &c.case_id, token: &c.token, window_end: v.window_end }
}

pub fn confirm(w: &Wire, c: &Case) -> R<Plan> {
    crate::mark::mark(crate::mark::FEATURE);
    let v = company(c)?;
    Ok(co::confirm_plan(w, &at(c, v))?)
}

pub fn dispute(w: &Wire, c: &Case, form: Form, wallet: &str, first: Option<&eravon_dispute::evidence::Evidence>) -> R<Plan> {
    crate::mark::mark(crate::mark::FEATURE);
    let v = company(c)?;
    let ev = first.map(|e| co::Evidence { manifest: e.manifest, bytes: Some(&e.bytes) });
    Ok(co::dispute_plan(w, &at(c, v), form, wallet, ev.as_ref())?)
}

pub fn commit(w: &Wire, c: &Case, ev: &eravon_dispute::evidence::Evidence) -> R<Plan> {
    crate::mark::mark(crate::mark::FEATURE);
    let v = company(c)?;
    Ok(co::commit_plan(w, &at(c, v), &co::Evidence { manifest: ev.manifest, bytes: Some(&ev.bytes) })?)
}

pub fn settle(w: &Wire, c: &Case, o: &Offer) -> R<Plan> {
    crate::mark::mark(crate::mark::FEATURE);
    company(c)?;
    Ok(co::settle_plan(w, &c.buyer_key, &c.instance, &o.st, &o.digest)?)
}

fn plan(verb: &'static str, calls: &[a1::call::Call], funs: &[a1::sol::Fun], instance: &str) -> R<Plan> {
    let mut pcs = Vec::with_capacity(calls.len());
    for c in calls {
        pcs.push(a1::proposal::of_call(c)?);
    }
    Ok(Plan {
        verb,
        calls: pcs,
        originals: vec![a1::proposal::abi_doc(funs, a1::proposal::INSTANCE_ABI_NAME)?],
        items: Vec::new(),
        face: Vec::new(),
        instance: instance.to_string(),
    })
}

pub fn sweep(w: &Wire, c: &Case, token: Option<&str>) -> R<Plan> {
    crate::mark::mark(crate::mark::FEATURE);
    if c.access != Access::Company {
        return Err(Fault::new("E_P6_NOT_COMPANY", "该订单不属于本公司"));
    }
    let o = eravon_cli::after_company::sweep_due(w, &c.instance, &c.token, c.chain.as_ref(), c.pack.as_deref(), token).map_err(|(n, d)| Fault::new(n, d))?;
    a1::chain::code_matches(&w.t, &w.table, &c.instance, &w.logic)?;
    let call = a1::call::Action::Sweep { who: o.payout, token: o.token }.call(&c.instance)?;
    plan("sweep", &[call], &[a1::sol::SWEEP], &c.instance)
}

pub fn open(w: &Wire, c: &Case, data: &str, me: &str) -> R<(Plan, String)> {
    crate::mark::mark(crate::mark::FEATURE);
    if c.access != Access::Company {
        return Err(Fault::new("E_P6_NOT_COMPANY", "该订单不属于本公司"));
    }
    if c.chain.as_ref().is_some_and(|v| v.phase != 0) {
        return Err(Fault::new("E_P6_OPENED", "订单已开启"));
    }
    let pk = eravon_link::pack::pack(c.pack.as_ref().ok_or_else(|| Fault::new("E_P6_NO_PACK", "请先导入订单文件"))?)?;
    a1::chain::listing_payable(&pk.listing.logic)?;
    let o = crate::act::unopened(c).ok_or_else(|| Fault::new("E_P6_NO_PACK", "请先导入订单文件"))?;
    let n = pk.receipt.candidate as usize;
    let sig_seller = a1::order::bytes_of("要约签名", pk.listing.candidate_sig(n)).unwrap_or_default();
    let now = a1::precheck::chain_now(&w.t, &w.table)?;
    Ok(eravon_cli::buyer::open_plan(w, &o, &sig_seller, n >= pk.listing.judges.len(), now, &eravon_cli::buyer::seat_party(data, me)?)?)
}

pub fn expire(w: &Wire, c: &Case) -> R<(Plan, String)> {
    crate::mark::mark(crate::mark::FEATURE);
    crate::act::past_gate(c)?;
    if c.access != Access::Company {
        return Err(Fault::new("E_P6_NOT_COMPANY", "该订单不属于本公司"));
    }
    if c.chain.as_ref().is_some_and(|v| v.phase != 0) {
        return Err(Fault::new("E_P6_OPENED", "订单已开启,不能过期退款"));
    }
    let o = crate::act::unopened(c).ok_or_else(|| Fault::new("E_P6_NO_PACK", "请先导入订单文件"))?;
    let logic = c.pack.as_ref().and_then(|b| eravon_link::pack::pack(b).ok()).map(|p| p.listing.logic).unwrap_or_else(|| w.logic.clone());
    let now = a1::precheck::chain_now(&w.t, &w.table)?;
    Ok(eravon_cli::buyer::expire_plan(w, &o, &logic, now)?)
}
