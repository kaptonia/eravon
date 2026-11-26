use eravon_standard as a1;
use a1::proposal::{Original, Plan};

use crate::deliver::Sheet;
use eravon_watch::order::Order;
use eravon_watch::{Why, Wire};

fn a1w(f: a1::fault::Fault) -> Why {
    (f.name(), f.detail())
}

fn calls_of(_o: &Order, claim: &a1::call::Call) -> Result<Vec<a1::proposal::PCall>, Why> {
    Ok(vec![a1::proposal::of_call(claim).map_err(a1w)?])
}


pub fn deliver(w: &Wire<'_>, o: &Order, s: &Sheet) -> Result<Plan, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    crate::deliver::gate(crate::deliver::phase(w, &o.instance)?)?;
    a1::chain::code_matches(w.t, w.table, &o.instance, w.logic).map_err(a1w)?;
    let claim = a1::call::Action::ClaimDelivery(s.manifest).call(&o.instance).map_err(a1w)?;
    let calls = calls_of(o, &claim)?;
    let originals = vec![
        a1::proposal::abi_doc(&[a1::sol::CLAIM_DELIVERY], a1::proposal::INSTANCE_ABI_NAME).map_err(a1w)?,
        Original { kind: "manifest".into(), name: crate::deliver::MANIFEST_PART.into(), bytes: s.bytes.clone() },
    ];
    Ok(Plan { verb: "claimDelivery", calls, originals, items: Vec::new(), face: Vec::new(), instance: o.instance.clone() })
}
