use super::*;

pub(super) fn cands_fault(e: eravon_cands::fault::Fault) -> Fault {
    use eravon_cands::fault::Fault as C;
    match e {
        C::Exists(a) => Fault::Exists(format!("{a} 已经在候选表上了")),
        C::Missing(a) => Fault::Missing(format!("候选表上没有 {a}")),
        C::Kind(_) => Fault::State(e_detail_kind()),
        C::Store { name: "E_STORE_CAP", detail } => Fault::State(detail),
        C::Store { name, detail } => Fault::Down { layer: "A5", token: name.to_string(), why: detail },
        other => Fault::Down { layer: "S3", token: other.name().to_string(), why: other.detail() },
    }
}

pub(super) fn e_detail_kind() -> String {
    "候选表上有一行的 kind 不在 judge 与 guild 两枚之内;这一本表是早先按错位写下来的,删掉 candidates.json 重加一遍".to_string()
}

pub(super) fn candidate_out(r: &eravon_store::record::Candidate) -> Out {
    Out::O(vec![
        ("addedAt", Out::N(r.added_at)),
        ("address", Out::S(r.address.clone())),
        ("kind", Out::S(r.kind.clone())),
        ("label", Out::S(r.label.clone())),
    ])
}

pub(super) fn candidate_add(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let input = f.opt("--candidate").ok_or_else(|| Fault::Arg("要 --candidate".into()))?;
    let label = f.opt("--label").unwrap_or_default();
    let kind = f.opt("--kind").unwrap_or_else(|| "judge".to_string());
    let side = eravon_cands::table::Side::of(&kind)
        .ok_or_else(|| Fault::Arg(format!("--kind 只有 judge 与 guild 两种,给的是 {kind:?}")))?;
    let row = crate::deskops::cands::add(&c.data, &c.writer, c.now, &who, side, &input, &label, &shared::route(f, c, Some(&who)), &[], &|| true).map_err(|n| match n {
        crate::deskops::cands::No::Cands(e) => cands_fault(e),
        crate::deskops::cands::No::Stale => Fault::State("stale".into()),
    })?;
    let count = eravon_cands::table::list(&c.data, &who).map_err(cands_fault)?.len();
    Ok(ok(vec![
        ("candidate", Out::S(row.address)),
        ("count", Out::N(count as u64)),
        ("kind", Out::S(kind)),
    ]))
}

fn page_said(name: &str, detail: String) -> Out {
    crate::deskops::said::Said::bad(name, detail).out()
}

fn signed_of(c: &Ctx, who: &str) -> Result<Vec<eravon_cands::view::Signed>, Out> {
    eravon_goods::goods::signed_words(&c.data, who).map_err(|e| page_said(e.name(), e.detail()))
}

pub(super) fn candidate_list(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let rows = eravon_cands::table::list(&c.data, &who).map_err(cands_fault)?;
    let route = shared::route(f, c, Some(&who));
    let page = signed_of(c, &who).and_then(|signed| {
        eravon_cands::view::list_on(&c.data, &who, &route, &f.many("--registry"), c.now, &signed)
            .map(|list| Out::A(list.iter().map(crate::deskops::candsview::brief_out).collect()))
            .map_err(|e| page_said(e.name(), e.detail()))
    });
    Ok(ok(vec![
        ("count", Out::N(rows.len() as u64)),
        ("candidates", Out::A(rows.iter().map(candidate_out).collect())),
        ("rows", page.unwrap_or_else(|e| e)),
    ]))
}

pub(super) fn candidate_show(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let addr = f.opt("--candidate").ok_or_else(|| Fault::Arg("要 --candidate".into()))?;
    let r = eravon_cands::table::list(&c.data, &who)
        .map_err(cands_fault)?
        .into_iter()
        .find(|r| r.address == addr.to_ascii_lowercase())
        .ok_or_else(|| Fault::Missing(format!("候选表上没有 {addr}")))?;
    let detail = signed_of(c, &who).and_then(|signed| {
        let route = shared::route(f, c, Some(&who));
        let (row, got, warn) = eravon_cands::view::detail_on(&c.data, &who, &r.address, &route, &f.many("--registry"), c.now, &signed).map_err(|e| page_said(e.name(), e.detail()))?;
        let shown = crate::deskops::candsview::shown(&got);
        let codes = match &route {
            Ok((t, table)) => crate::deskops::candsview::codes(t, table, &shown),
            Err((n, d)) => shown.into_iter().map(|a| (a, Err((n.clone(), d.clone())))).collect(),
        };
        Ok(Out::O(crate::deskops::candsview::detail_out(&row, &got, warn.as_ref(), &codes)))
    });
    Ok(ok(vec![
        ("addedAt", Out::N(r.added_at)),
        ("address", Out::S(r.address.clone())),
        ("detail", detail.unwrap_or_else(|e| e)),
        ("kind", Out::S(r.kind.clone())),
        ("label", Out::S(r.label.clone())),
    ]))
}

pub(super) fn candidate_remove(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let addr = f.opt("--candidate").ok_or_else(|| Fault::Arg("要 --candidate".into()))?;
    eravon_cands::table::remove(&c.data, &c.writer, c.now, &who, &addr.to_ascii_lowercase()).map_err(cands_fault)?;
    let count = eravon_cands::table::list(&c.data, &who).map_err(cands_fault)?.len();
    Ok(ok(vec![("count", Out::N(count as u64)), ("removed", Out::S(addr))]))
}
