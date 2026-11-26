use super::*;

fn order_at(c: &Ctx, who: &str, instance: &str) -> R<eravon_watch::order::Order> {
    let all = eravon_watch::order::all(&c.data, who).map_err(|(token, why)| Fault::Down { layer: "S4", token, why })?;
    all.into_iter().find(|o| o.instance.eq_ignore_ascii_case(instance)).ok_or_else(|| Fault::Missing(format!("没有实例为 {instance} 的订单")))
}

fn watch_down((token, why): eravon_watch::Why) -> Fault {
    Fault::Down { layer: "S4", token, why }
}

pub(super) fn payways_set(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let flag = |k: &str| -> R<Option<bool>> {
        match f.one(k) {
            None => Ok(None),
            Some("true") => Ok(Some(true)),
            Some("false") => Ok(Some(false)),
            Some(v) => Err(Fault::Arg(format!("{k} 只收 true 或 false,给的是 {v:?}"))),
        }
    };
    let e = eravon_payways::ways::Edit { auto_open: flag("--auto-open")?, open_window: f.opt_int("--open-window")?, buyer_may_open: flag("--buyer-may-open")?, fund: f.opt("--fund"), own: flag("--fund-own")? };
    let saved = crate::deskops::payways::save(&c.data, &c.writer, c.now, &who, &e).map_err(|n| n.fault())?;
    Ok(ok(vec![("address", Out::S(who)), ("saved", Out::O(eravon_payways::ways::rows(&saved.ways)))]))
}

pub(super) fn candidate_rename(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let at = f.opt("--candidate").ok_or_else(|| Fault::Arg("要 --candidate".into()))?;
    let label = f.opt("--label").unwrap_or_default();
    eravon_cands::table::rename(&c.data, &c.writer, c.now, &who, &at.to_ascii_lowercase(), &label).map_err(cands_fault)?;
    Ok(ok(vec![("candidate", Out::S(at.to_ascii_lowercase())), ("label", Out::S(label))]))
}

fn goods_at<'a>(c: &'a Ctx, f: &Flags, who: &'a str, signer: &'a eravon_goods::sign::Signer, registries: &'a [String]) -> eravon_goods::goods::Ctx<'a> {
    eravon_goods::goods::Ctx { data: &c.data, writer: &c.writer, now: c.now, me: who, registries, signer, route: shared::route(f, c, Some(who)) }
}

pub(super) fn product_listed(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let id = f.opt("--id").ok_or_else(|| Fault::Arg("要 --id".into()))?;
    let on = match f.one("--on") {
        Some("true") => true,
        Some("false") => false,
        x => return Err(Fault::Arg(format!("--on 只收 true 或 false,给的是 {x:?}"))),
    };
    let k = eravon_goods::sign::Signer::locked();
    eravon_goods::goods::set_listed(&goods_at(c, f, &who, &k, &[]), &id, on).map_err(goods_down)?;
    Ok(ok(vec![("id", Out::S(id)), ("listed", Out::B(on))]))
}

pub(super) fn product_delete(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let id = f.opt("--id").ok_or_else(|| Fault::Arg("要 --id".into()))?;
    let k = eravon_goods::sign::Signer::locked();
    eravon_goods::goods::delete(&goods_at(c, f, &who, &k, &[]), &id).map_err(goods_down)?;
    Ok(ok(vec![("removed", Out::S(id))]))
}

pub(super) fn product_export_proposal(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let id = f.opt("--id").ok_or_else(|| Fault::Arg("要 --id".into()))?;
    let dir = f.opt("--out").ok_or_else(|| Fault::Arg("要 --out".into()))?;
    let short = eravon_goods::goods::proposal_time(&c.data, &who, &id).map_err(goods_down)?;
    let path = eravon_goods::goods::export_proposal(&c.data, &who, &id, &dir).map_err(goods_down)?;
    let mut m = vec![("file", Out::S(path)), ("id", Out::S(id))];
    if let Some((bound, short)) = short {
        m.push(("timeShort", Out::S(crate::company::ask_say(bound, &short))));
    }
    Ok(ok(m))
}

pub(super) fn pending_bump(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let o = order_at(c, &who, &f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?)?;
    let w = shared::wire(f, c, Some(&who))?;
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let mut p = watch::ActPost::new(c, f, &w, &who);
    let r = eravon_watch::send::bump(&c.data, &c.writer, c.now, &who, &sw, &o.id, &mut p).map(|t| t.order.note);
    let mut rows = p.finish("S4", r.clone())?;
    rows.push(("instance", Out::S(o.instance)));
    rows.push(("note", Out::S(r.unwrap_or_default())));
    Ok(ok(rows))
}

pub(super) fn pending_void(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let o = order_at(c, &who, &f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?)?;
    let w = shared::wire(f, c, Some(&who))?;
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let done = eravon_watch::send::void(&c.data, &c.writer, c.now, &who, &who, &sw, &o.id).map_err(watch_down)?;
    Ok(ok(vec![("instance", Out::S(done.instance)), ("note", Out::S(done.note))]))
}

pub(super) fn order_label(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let buyer = f.opt("--buyer").ok_or_else(|| Fault::Arg("要 --buyer".into()))?;
    let label = f.opt("--label").unwrap_or_default();
    eravon_orders::label::set(&c.data, &c.writer, c.now, &who, &buyer, &label).map_err(|(token, why)| Fault::Down { layer: "S2", token, why })?;
    Ok(ok(vec![("buyer", Out::S(buyer)), ("label", Out::S(label))]))
}

pub(super) fn deliver_save(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let o = order_at(c, &who, &f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?)?;
    let dir = f.opt("--out").ok_or_else(|| Fault::Arg("要 --out".into()))?;
    let d = eravon_deliver::deliver::delivery(&c.data, &who, &o).ok_or_else(|| Fault::Down { layer: "S5", token: "E_DELIVER_ORIGINAL".into(), why: o.instance.clone() })?;
    let paths = eravon_deliver::deliver::save_originals(&d, &dir).map_err(|(token, why)| Fault::Down { layer: "S5", token, why })?;
    Ok(ok(vec![("files", Out::A(paths.into_iter().map(Out::S).collect())), ("instance", Out::S(o.instance))]))
}
