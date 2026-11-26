use eravon_dispute::complaint::Role;
use eravon_literals::json::Out;

use crate::answer::ok;
use crate::deskops::complaint as cp;
use crate::deskops::said::Said;
use crate::fault::{Fault, R};
use crate::home::Ctx;
use crate::shared::{self, Flags};

fn down(s: Said) -> Fault {
    Fault::Down { layer: "S6", token: s.name, why: s.detail }
}

struct On {
    complaint: cp::Complaint,
    logic: String,
    chain_id: u64,
    role: Role,
}

fn on(c: &Ctx, f: &Flags, who: &str, buyer: bool) -> R<On> {
    let w = shared::wire(f, c, Some(who))?;
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let (row, about, role) = if buyer {
        use crate::deskops::buyerpage as bp;
        let session = c.session(f.one("--passcode"))?;
        let tokens = eravon_vault::tokens::list(&c.data);
        let shut = |at: String, kind: String, why: String| format!("{at} · {kind} · {why}");
        let page = bp::Page { tokens: &tokens, private_goods: "", company_shut: &shut, company_row: bp::Page::company_row_of, column_holder: bp::column_holder };
        let mut owners = Vec::new();
        let t = bp::row_one(&session, &c.data, who, &w, &page, &mut owners, &instance).map_err(down)?;
        let r = t.rows.into_iter().find(|r| r.row.instance.eq_ignore_ascii_case(&instance)).ok_or_else(|| Fault::Missing(format!("本席的订单表上没有 {instance}")))?;
        let about = cp::buyer_about(&r).map_err(down)?;
        (r.row, about, Role::Buyer)
    } else {
        let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
        let s2 = |e: eravon_orders::Why| Fault::Down { layer: "S2", token: e.0, why: e.1 };
        let t = eravon_orders::table::table_one(&c.data, who, eravon_scan::scan::Side::Seller, &sw, &instance).and_then(eravon_orders::table::Table::complete).map_err(s2)?;
        let r = eravon_orders::table::find(&t, &instance).map_err(s2)?.clone();
        let order = eravon_orders::detail::order_of(&c.data, who, &r);
        let wire = || shared::wire(f, c, Some(who)).ok();
        let about = cp::seller_about(&c.data, who, &r, order.as_ref(), |o| crate::deskops::orderpage::order_company(&c.data, who, o, &wire, crate::deskops::orderpage::route_of)).map_err(down)?;
        (r, about, Role::Seller)
    };
    let cw = eravon_cands::wire::Wire::on(&shared::route(f, c, Some(who)), &w.registries, c.now);
    let complaint = cp::read(cw, &row.instance, &row.case_id, &about.judge, about.pool);
    Ok(On { complaint, logic: row.logic.clone(), chain_id: about.chain_id, role })
}

pub(crate) fn open(c: &Ctx, f: &Flags, buyer: bool) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let o = on(c, f, &who, buyer)?;
    Ok(ok(vec![("complaint", cp::complaint_out(&o.complaint))]))
}

pub(crate) fn write(c: &Ctx, f: &Flags, buyer: bool) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let text = f.opt("--text").unwrap_or_default();
    if text.trim().is_empty() {
        return Err(Fault::Arg(eravon_vault::lang::t(eravon_vault::lang::Key::CpTextHint).to_string()));
    }
    let files = cp::files_of(&f.many("--attach").join("\n")).map_err(Fault::Arg)?;
    let form = cp::Form { text, dir: f.opt("--out").unwrap_or_default(), files, guild: f.opt("--guild").unwrap_or_default() };
    let mut o = on(c, f, &who, buyer)?;
    let mut key = c.key(&who, f.one("--passcode"))?;
    let case = cp::Case { chain_id: o.chain_id, logic: &o.logic, me: &who, role: o.role, now: c.now };
    let got = cp::write(&o.complaint, &form, &key, &case);
    for b in key.iter_mut() {
        *b = 0;
    }
    let _ = std::hint::black_box(&key);
    let path = got.map_err(down)?;
    o.complaint.written = Some(Ok(path.clone()));
    Ok(ok(vec![("complaint", cp::complaint_out(&o.complaint)), ("file", Out::S(path))]))
}
