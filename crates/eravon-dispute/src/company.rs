use eravon_standard as a1;
use a1::company::Item;
use a1::proposal::{Original, Plan};

use crate::evidence::Evidence;
use crate::settle::Offer;
use crate::{Why, Wire, a1w, case_of};
use eravon_watch::order::Order;

fn refund_to<'a>(o: &'a Order, _logic: &'a str) -> &'a str {
    &o.instance
}


pub fn refund(w: &Wire<'_>, o: &Order) -> Result<Plan, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    a1::chain::code_matches(w.t, w.table, &o.instance, w.logic).map_err(a1w)?;
    let call = a1::call::Action::Refund.call(refund_to(o, w.logic)).map_err(a1w)?;
    Ok(Plan {
        verb: "refund",
        calls: vec![a1::proposal::of_call(&call).map_err(a1w)?],
        originals: vec![a1::proposal::abi_doc(&[a1::sol::REFUND], a1::proposal::INSTANCE_ABI_NAME).map_err(a1w)?],
        items: Vec::new(),
        face: Vec::new(),
        instance: o.instance.clone(),
    })
}

fn registry_of<'a>(logic: &'a str, _instance: &'a str) -> &'a str {
    logic
}


pub fn settle(chain_id: u64, logic: &str, company: &str, instance: &str, case_id: &str, buyer_bps: u64, expiry: u64) -> Result<(Plan, Offer), Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let empty = |_d: &[u8; 32]| Ok(String::new());
    let of = crate::settle::offer(&empty, chain_id, instance, buyer_bps, expiry)?;
    let original = Original { kind: eravon_literals::literals::DOC_SETTLE.into(), name: "settle-offer.json".into(), bytes: of.bytes.clone() };
    let (pc, mut docs) = a1::proposal::consent_at(registry_of(logic, instance), company, &of.digest, original).map_err(a1w)?;
    let case = a1::order::b32_of("案号", case_id).map_err(a1w)?;
    let (post, board) = a1::proposal::board_post(&case, &of.bytes).map_err(a1w)?;
    docs.push(board);
    let plan = Plan {
        verb: "settle",
        calls: vec![pc, post],
        originals: docs,
        items: vec![Item::Deadline { name: "settleExpiry", who: String::new(), at: expiry }],
        face: vec![a1::proposal::face_consent(expiry), a1::proposal::FACE_SETTLE_POST.to_string()],
        instance: instance.to_string(),
    };
    Ok((plan, of))
}

fn evidence_items(w: &Wire<'_>, o: &Order, case: &[u8; 32]) -> Result<Vec<Item>, Why> {
    Ok(vec![a1::company::evidence_item(w.t, w.table, &o.instance, case).map_err(a1w)?])
}


pub fn commit(w: &Wire<'_>, o: &Order, ev: &Evidence) -> Result<Plan, Why> {
    commit_of(w, o, ev.manifest, Some(&ev.bytes))
}

pub fn commit_of(w: &Wire<'_>, o: &Order, manifest: [u8; 32], bytes: Option<&[u8]>) -> Result<Plan, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    a1::chain::code_matches(w.t, w.table, &o.instance, w.logic).map_err(a1w)?;
    let case = case_of(o)?;
    let call = a1::call::Action::Commit { case_id: case, manifest }.call(&o.instance).map_err(a1w)?;
    let mut originals = vec![a1::proposal::abi_doc(&[a1::sol::COMMIT], a1::proposal::INSTANCE_ABI_NAME).map_err(a1w)?];
    if let Some(b) = bytes {
        originals.push(Original { kind: a1::proposal::EVIDENCE_DOC_KIND.into(), name: a1::proposal::EVIDENCE_DOC_NAME.into(), bytes: b.to_vec() });
    }
    Ok(Plan {
        verb: "commit",
        calls: vec![a1::proposal::of_call(&call).map_err(a1w)?],
        originals,
        items: evidence_items(w, o, &case)?,
        face: vec![a1::proposal::FACE_EVIDENCE.to_string()],
        instance: o.instance.clone(),
    })
}
