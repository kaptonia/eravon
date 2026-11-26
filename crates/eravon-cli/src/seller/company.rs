use super::*;

pub(super) fn exporting(f: &Flags) -> Option<String> {
    crate::company::asked(f)
}


pub(super) fn time_gate(on: bool) -> bool {
    on
}


pub(super) fn seller_is_company(t: &k2::doc::Terms, co: &crate::company::Company) -> R<()> {
    if !t.seller_key.eq_ignore_ascii_case(&co.address) {
        return Err(Fault::Precheck { gate: crate::fault::GATE_COMPANY.into(), why: format!("条款的 sellerKey 是 {},不是公司地址 {}", t.seller_key, co.address) });
    }
    Ok(())
}


pub(super) fn payout_is_ledger(t: &k2::doc::Terms, co: &crate::company::Company) -> R<()> {
    if !t.seller_payout.eq_ignore_ascii_case(&co.views.ledger) {
        return Err(Fault::Precheck { gate: crate::fault::GATE_COMPANY.into(), why: format!("条款的 sellerPayout 是 {},公司模式要填 ledger() {}", t.seller_payout, co.views.ledger) });
    }
    Ok(())
}


pub(super) fn company_export(
    c: &Ctx,
    f: &Flags,
    who: &str,
    path: &str,
    build: impl FnOnce(&shared::Wire, &crate::company::Company) -> R<crate::company::Export>,
) -> R<Out> {
    let w = shared::wire(f, c, Some(who))?;
    let co = crate::company::company(c, f, &w, who)?;
    let gate = time_gate(crate::company::time_switch(c, f, who)?);
    let e = build(&w, &co)?;
    let mut rows = crate::company::export(c, &w, &co, path, gate, e)?;
    rows.extend(w.rows());
    Ok(ok(rows))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn offer_export(co: &crate::company::Company, t: &k2::doc::Terms, bytes: &[u8], path: &str, w: &shared::Wire) -> R<crate::company::Export> {
    let chain = w.chain_id;
    if t.chain_id != chain {
        return Err(Fault::Precheck { gate: crate::fault::GATE_COMPANY.into(), why: format!("条款的 chainId 是 {},连线的链是 {chain}:提案与要约摘要不在同一条链上", t.chain_id) });
    }
    seller_is_company(t, co)?;
    payout_is_ledger(t, co)?;
    if !t.guilds.is_empty() && !t.buyer_may_open {
        return Err(Fault::Down { layer: "S1", token: eravon_goods::company::E_GOODS_GUILD_OPEN.into(), why: "行会候选的单,公司不经提案开案:条款要写 buyerMayOpen 为真".into() });
    }
    let name = eravon_literals::seam::file_name_of(path);
    let original = a1::proposal::Original { kind: seam::V_TERMS.into(), name, bytes: bytes.to_vec() };
    let total = (t.judges.len() + t.guilds.len()) as u64;
    let mut calls = Vec::new();
    let mut originals = Vec::new();
    let mut items = Vec::new();
    items.push(a1::company::Item::Window { name: "t1", who: String::new(), secs: t.delivery_window });
    for cand in 0..total {
        let d = k2::derive::offer_digest_at(t, cand).map_err(Fault::from)?;
        let (pc, docs) = a1::proposal::consent(&t.logic, &t.seller_key, &d, original.clone())?;
        calls.push(pc);
        originals.extend(docs);
    }
    items.push(a1::company::resign_lead_item());
    if !t.buyer_may_open {
        items.push(a1::company::Item::Window { name: "openWindow", who: String::new(), secs: t.open_window });
    }
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let zero = "0x0000000000000000000000000000000000000000".to_string();
    let quoted: Vec<(String, String, String, String)> = t.judges.iter().map(|j| (j.judge.clone(), zero.clone(), j.amanah_hash.clone(), j.quote_hash.clone())).chain(t.guilds.iter().map(|g| (zero.clone(), g.address.clone(), g.amanah_hash.clone(), g.quote_hash.clone()))).collect();
    let mut cands = Vec::with_capacity(quoted.len());
    for (judge, guild, amanah, qh) in quoted {
        let addr = if judge == zero { guild.clone() } else { judge.clone() };
        let cand = a1::order::b32_of("amanahHash", &amanah).map(|h| eravon_cands::act::Candidate { judge, guild, amanah_hash: h, token: t.token.to_ascii_lowercase(), parties: vec![co.address.clone()] });
        cands.push((addr, cand, qh));
    }
    let held = std::sync::Arc::new(sw.held());
    let quotes = eravon_chain::read::side_by_side(cands.iter().map(|(_, cand, qh)| (cand.as_ref().ok().cloned(), qh.clone())).collect::<Vec<_>>(), move |(cand, qh)| cand.map(|c| eravon_watch::act::quote(&held.wire(), &c, &qh)));
    for ((addr, cand, _), q) in cands.into_iter().zip(quotes) {
        cand?;
        let Some(q) = q else { continue };
        let q = q.map_err(|(token, why)| Fault::Down { layer: "S4", token, why })?;
        items.push(a1::company::Item::Window { name: "t3", who: addr.to_ascii_lowercase(), secs: q.ruling_window });
    }
    Ok(crate::company::Export {
        verb: "offer-sign",
        calls,
        originals,
        items,
        face: vec![a1::proposal::face_consent(t.expiry), a1::proposal::FACE_OFFER.to_string()],
        deal: crate::company::Deal::None,
    })
}
