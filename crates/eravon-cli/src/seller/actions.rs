use super::*;

pub(super) fn action(c: &Ctx, f: &Flags, a: a1::call::Action) -> R<Out> {
    action_with(c, f, a, None)
}

pub(super) fn action_with(c: &Ctx, f: &Flags, a: a1::call::Action, evidence: Option<&[u8]>) -> R<Out> {
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    a1::chain::code_matches(&w.t, &w.table, &instance, &w.logic)?;
    let call = a.call(&instance)?;
    if let Some(path) = exporting(f) {
        let case = match &a {
            a1::call::Action::Commit { case_id, .. } => hex(case_id),
            _ => String::new(),
        };
        let o = order_at(c, &who, &instance, f.opt("--id"), &case)?;
        return company_export(c, f, &who, &path, |w, _co| {
            let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
            let p = match &a {
                a1::call::Action::Refund => eravon_dispute::company::refund(&sw, &o),
                a1::call::Action::Commit { manifest, .. } => eravon_dispute::company::commit_of(&sw, &o, *manifest, evidence),
                other => return Err(Fault::Arg(format!("{} 不出公司提案", other.fun().name))),
            }
            .map_err(s6)?;
            Ok(crate::company::Export::of(p))
        });
    }
    let mut rows = vec![
        ("action", Out::S(a.fun().name.to_string())),
        ("instance", Out::S(instance)),
    ];
    rows.extend(shared::deliver_rows(c, f, &w, &who, &call, "0")?);
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub(super) fn claim(c: &Ctx, f: &Flags) -> R<Out> {
    let dir = f.opt("--manifest-dir").unwrap_or_default();
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let case_id = f.opt("--case").ok_or_else(|| Fault::Arg("要 --case".into()))?;
    let (kit_items, kit_rows) = kit_files(c, f)?;
    let party = match exporting(f) {
        Some(_) => crate::company::company(c, f, &w, &who)?.address,
        None => who.clone(),
    };
    let o = order_at(c, &who, &instance, f.opt("--id"), &case_id)?;
    let mut items = eravon_deliver::deliver::items(&dir).map_err(s5_down)?;
    items.extend(kit_items);
    items.sort_by(|a, b| a.name().as_bytes().cmp(b.name().as_bytes()));
    let s5 = eravon_deliver::deliver::sheet_of(items, &party, &o, w.chain_id, &w.logic, f.opt("--note")).map_err(s5_down)?;
    let param = s5.manifest;
    if let Some(path) = exporting(f) {
        return company_export(c, f, &who, &path, |w, co| {
            let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
            let _ = co;
            let p = eravon_deliver::company::deliver(&sw, &o, &s5).map_err(s5_down)?;
            Ok(crate::company::Export::of(p))
        });
    }

    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let mut post = ActPost::new(c, f, &w, &who);
    let r = eravon_deliver::deliver::claim(&c.data, &c.writer, c.now, &who, &sw, &o, &s5, "", f.one("--note").unwrap_or(""), &dir, &mut post);
    let sent = r.as_ref().ok().cloned();
    let mut rows = vec![
        ("action", Out::S("claimDelivery".into())),
        ("instance", Out::S(instance)),
        ("manifest", Out::S(hex(&param))),
    ];
    rows.extend(kit_rows);
    rows.extend(post.finish("S5", r)?);
    if let Some(tx) = sent {
        rows.push(("handOver", hand_over_row(eravon_deliver::deliver::hand_over(&sw, &tx))));
    }
    rows.extend(w.rows());
    Ok(ok(rows))
}

fn hand_over_out(h: &eravon_deliver::deliver::HandOver) -> Out {
    Out::O(vec![("claimBlock", Out::N(h.claim_block)), ("finalized", Out::N(h.finalized)), ("ready", Out::B(h.ready))])
}

pub(super) fn hand_over_row(h: Result<eravon_deliver::deliver::HandOver, eravon_watch::Why>) -> Out {
    match h {
        Ok(h) => hand_over_out(&h),
        Err((name, detail)) => Out::O(vec![("detail", Out::S(detail)), ("name", Out::S(name))]),
    }
}

pub(super) fn zk_rows(s: &a4::Survey) -> Vec<(&'static str, Out)> {
    let mut rows = vec![
        ("at", Out::S(s.link.at.clone())),
        ("connected", Out::B(true)),
        ("layer", Out::S(s.link.layer.name().to_string())),
    ];
    if let Some(a) = &s.absent {
        rows.push(("absent", Out::S(a.name().to_string())));
    }
    rows
}

pub(super) fn zk_absent_rows(a: &a4::fault::Absent) -> Vec<(&'static str, Out)> {
    let mut rows = vec![
        ("absent", Out::S(a.name().to_string())),
        ("connected", Out::B(false)),
    ];
    if let Some(at) = a.at() {
        rows.push(("at", Out::S(at.to_string())));
    }
    rows
}

pub(super) enum Zk {
    Off,
    Absent(a4::fault::Absent),
    Found(a4::Survey),
}

pub(super) fn zk_survey(c: &Ctx, f: &Flags) -> R<Zk> {
    let who = c.who(f.one("--address"))?;
    let (linked, dir) = crate::deskops::network::zikaron_of(&c.data, &who)?;
    if !linked {
        return Ok(Zk::Off);
    }
    let home = f.opt("--home").ok_or_else(|| Fault::Arg("要 --home".into()))?;
    let machine = f.opt("--machine").or(Some(dir).filter(|d| !d.trim().is_empty()));
    match a4::survey(&home, machine.as_deref()) {
        Ok(s) => Ok(Zk::Found(s)),
        Err(e) => match e.absent() {
            Some(a) => Ok(Zk::Absent(a.clone())),
            None => Err(Fault::from(e)),
        },
    }
}

fn zk_off_rows() -> Vec<(&'static str, Out)> {
    vec![("absent", Out::S(crate::deskops::network::ZK_OFF.to_string())), ("connected", Out::B(false))]
}

pub(super) fn kit_files(c: &Ctx, f: &Flags) -> R<(Vec<eravon_evidence::sheet::FileItem>, Vec<(&'static str, Out)>)> {
    let Some(id) = f.opt("--kit") else {
        return Ok((Vec::new(), Vec::new()));
    };
    let s = match zk_survey(c, f)? {
        Zk::Found(s) => s,
        Zk::Off => {
            return Err(Fault::Down { layer: "S9", token: crate::deskops::network::ZK_OFF.to_string(), why: "这一身份的设置里关了「连接 ZIKARON」,不探".into() });
        }
        Zk::Absent(a) => {
            return Err(Fault::Down { layer: "A4", token: a.name().to_string(), why: a.detail() });
        }
    };
    if let Some(a) = &s.absent {
        return Err(Fault::Down { layer: "A4", token: a.name().to_string(), why: a.detail() });
    }
    let kit = a4::index::pick(&s.kits, &id)?;
    let media = f.opt("--media-type").unwrap_or_else(|| a4::seam::MEDIA_DEFAULT.to_string());
    let items = a4::pack::items(kit, &media)?;
    let rows = vec![
        ("kitFiles", Out::N(items.len() as u64)),
        ("kitId", Out::S(kit.id.clone())),
        ("kitLink", Out::S(kit.link.clone())),
    ];
    Ok((items, rows))
}

pub(super) fn zikaron_status(c: &Ctx, f: &Flags) -> R<Out> {
    match zk_survey(c, f)? {
        Zk::Off => Ok(ok(zk_off_rows())),
        Zk::Absent(a) => Ok(ok(zk_absent_rows(&a))),
        Zk::Found(s) => {
            let mut rows = zk_rows(&s);
            rows.push(("count", Out::N(s.kits.len() as u64)));
            Ok(ok(rows))
        }
    }
}

pub(super) fn zikaron_kits(c: &Ctx, f: &Flags) -> R<Out> {
    let rows = match zk_survey(c, f)? {
        Zk::Off => zk_off_rows(),
        Zk::Absent(a) => zk_absent_rows(&a),
        Zk::Found(s) => {
            let list: Vec<Out> = s
                .kits
                .iter()
                .map(|k| {
                    Out::O(vec![
                        ("contents", Out::N(k.contents.len() as u64)),
                        ("created", Out::N(k.created)),
                        ("id", Out::S(k.id.clone())),
                        ("link", Out::S(k.link.clone())),
                        ("noteMd", Out::S(k.note_md.clone())),
                    ])
                })
                .collect();
            let mut rows = zk_rows(&s);
            rows.push(("kits", Out::A(list)));
            return Ok(ok(rows));
        }
    };
    let mut rows = rows;
    rows.push(("kits", Out::A(Vec::new())));
    Ok(ok(rows))
}

fn post_offer(c: &Ctx, f: &Flags, who: &str, instance: &str, bytes: &[u8]) -> R<Vec<(&'static str, Out)>> {
    let w = shared::wire(f, c, Some(who))?;
    let case = match eravon_watch::order::all(&c.data, who).map_err(s4)?.into_iter().find(|o| o.instance.eq_ignore_ascii_case(instance)) {
        Some(o) => o.case_id,
        None => f.opt("--case").ok_or_else(|| Fault::Arg("本机没有这一单的订单记录:要 --case(案号)".into()))?,
    };
    let case_id = a1::order::b32_of("案号", &case)?;
    let call = eravon_dispute::settle::post_call(&case_id, bytes).map_err(s6)?;
    let mut rows = vec![("caseId", Out::S(case.clone()))];
    rows.extend(shared::deliver_rows(c, f, &w, who, &call, "0")?);
    if crate::send::asked(f) {
        let tx = rows.iter().find_map(|(k, v)| if *k == "txHash" { if let Out::S(s) = v { Some(s.clone()) } else { None } } else { None }).unwrap_or_default();
        let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
        eravon_dispute::settle::posted(&sw, &tx, &case_id, bytes).map_err(s6)?;
        let link = eravon_goods::link::link_of(w.chain_id, &tx, bytes).map_err(|(n, d)| Fault::Down { layer: "S6", token: n, why: d })?;
        rows.push(("link", Out::S(link)));
    }
    Ok(rows)
}

pub(super) fn settle_offer(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    if let Some(path) = exporting(f) {
        let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
        let buyer_bps = f.int("--buyer-bps").map_err(|_| Fault::Arg("要 --buyer-bps".into()))?;
        let expiry = f.int("--expiry").map_err(|_| Fault::Arg("要 --expiry".into()))?;
        let chain_id = home::chain_id(f.opt_int("--chain")?);
        let found = eravon_watch::order::all(&c.data, &who).map_err(s4)?.into_iter().find(|o| o.instance.eq_ignore_ascii_case(&instance));
        let case = match &found {
            Some(o) => o.case_id.clone(),
            None => f.opt("--case").ok_or_else(|| Fault::Arg("本机没有这一单的订单记录:要 --case(案号)".into()))?,
        };
        let answer = company_export(c, f, &who, &path, |w, co| {
            let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
            let g = eravon_watch::poll::generation_at(&sw, &c.data, &who, &instance, found.as_ref()).map_err(s4)?;
            a1::chain::past_gate(g.is_past())?;
            let logic = g.row().logic.to_string();
            let (p, _) = eravon_dispute::company::settle(chain_id, &logic, &co.address, &instance, &case, buyer_bps, expiry).map_err(s6)?;
            Ok(crate::company::Export::of(p))
        })?;
        let Out::O(mut m) = answer else { return Ok(answer) };
        m.push(("post", Out::O(vec![("caseId", Out::S(case)), ("inProposal", Out::B(true))])));
        return Ok(Out::O(m));
    }
    let key = c.key(&who, f.one("--passcode"))?;
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let buyer_bps = f.int("--buyer-bps").map_err(|_| Fault::Arg("要 --buyer-bps".into()))?;
    let expiry = f.int("--expiry").map_err(|_| Fault::Arg("要 --expiry".into()))?;
    let chain_id = home::chain_id(f.opt_int("--chain")?);
    let sign = |d: &[u8; 32]| shared::sign(&key, d).map_err(|e| (e.name().to_string(), e.detail()));
    {
        let w = shared::wire(f, c, Some(&who))?;
        let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
        let o = order_at(c, &who, &instance, None, &f.opt("--case").unwrap_or_default())?;
        a1::chain::code_matches(&w.t, &w.table, &instance, &w.logic)?;
        eravon_dispute::settle::full_is_refund(&sw, &o, buyer_bps).map_err(s6)?;
    }
    let of = eravon_dispute::settle::offer(&sign, chain_id, &instance, buyer_bps, expiry).map_err(s6)?;
    if let Some(o) = eravon_watch::order::all(&c.data, &who).map_err(s4)?.into_iter().find(|o| o.instance.eq_ignore_ascii_case(&instance)) {
        eravon_dispute::settle::remember(&c.data, &c.writer, c.now, &who, &o, &of).map_err(s6)?;
    }
    let posted = post_offer(c, f, &who, &instance, &of.bytes)?;
    Ok(ok(vec![("bytes", Out::N(of.bytes.len() as u64)), ("digest", Out::S(hex(&of.digest))), ("post", Out::O(posted)), ("sig", Out::S(of.sig))]))
}

pub(super) fn settle(c: &Ctx, f: &Flags) -> R<Out> {
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let buyer_bps = f.int("--buyer-bps").map_err(|_| Fault::Arg("要 --buyer-bps".into()))?;
    let expiry = a1::order::b32_of(
        "--expiry",
        &f.opt("--expiry-word").ok_or_else(|| Fault::Arg("要 --expiry-word(三十二字节)".into()))?,
    )?;
    let sig_buyer = f.opt("--sig-buyer").ok_or_else(|| Fault::Arg("要 --sig-buyer(hex65,或空串表示买家以登记同意)".into()))?;
    let sb = a1::order::bytes_of("--sig-buyer", &sig_buyer)?;
    let ss = a1::order::bytes_of("--sig-seller", &f.opt("--sig-seller").ok_or_else(|| Fault::Arg("要 --sig-seller".into()))?)?;
    let bps = u16::try_from(buyer_bps).map_err(|_| Fault::Arg("--buyer-bps 超界".into()))?;
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let e = eravon_terms::u256::U256::from_be(&expiry).to_dec();
    let exp: u64 = e.parse().map_err(|_| Fault::Arg(format!("--expiry-word 装不进秒数:{e}")))?;
    let others = eravon_dispute::settle::others(&c.data, &who).map_err(s6)?;
    eravon_dispute::settle::check_pasted(&sw, &instance, eravon_dispute::settle::Paste::Settle { buyer_bps, expiry: exp }, &sig_buyer, &others).map_err(s6)?;
    let digest = eravon_terms::derive::digest(w.chain_id, &instance, &eravon_terms::derive::Action::Settle { buyer_bps, expiry: eravon_terms::u256::U256::from_u64(exp) }.struct_hash());
    let (sb, ss) = a1::precheck::settle_slots(&w.t, &w.table, &w.logic, &instance, &digest, &sb, &ss, &who)?;
    action(c, f, a1::call::Action::Settle { buyer_bps: bps, expiry, sig_buyer: sb, sig_seller: ss })
}

pub(super) fn confirm_bysig(c: &Ctx, f: &Flags) -> R<Out> {
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let sig = f.opt("--sig").ok_or_else(|| Fault::Arg("要 --sig(买家的 hex65)".into()))?;
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let others = eravon_dispute::settle::others(&c.data, &who).map_err(s6)?;
    eravon_dispute::settle::check_pasted(&sw, &instance, eravon_dispute::settle::Paste::Confirm, &sig, &others).map_err(s6)?;
    a1::chain::code_matches(&w.t, &w.table, &instance, &w.logic)?;
    let call = a1::call::BySig::Confirm { sig: a1::order::bytes_of("--sig", &sig)? }.call(&instance)?;
    let mut rows = vec![("action", Out::S("confirmBySig".into())), ("instance", Out::S(instance))];
    rows.extend(shared::deliver_rows(c, f, &w, &who, &call, "0")?);
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub(super) fn commit(c: &Ctx, f: &Flags) -> R<Out> {
    let case_id = a1::order::b32_of("--case", &f.opt("--case").ok_or_else(|| Fault::Arg("要 --case".into()))?)?;
    if f.opt("--manifest-dir").is_some() || f.opt("--kit").is_some() {
        return commit_built(c, f, case_id);
    }
    let (manifest, bytes) = shared::evidence_arg(f)?;
    action_with(c, f, a1::call::Action::Commit { case_id, manifest }, bytes.as_deref())
}

fn commit_built(c: &Ctx, f: &Flags, case_id: [u8; 32]) -> R<Out> {
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let party = match exporting(f) {
        Some(_) => crate::company::company(c, f, &w, &who)?.address,
        None => who.clone(),
    };
    let case = hex(&case_id);
    let ev = shared::evidence_built(c, f, &who, &party, &instance, &case, &w)?.ok_or_else(|| Fault::Arg("要 --manifest-dir 或 --kit".into()))?;
    if exporting(f).is_some() {
        let Out::O(mut m) = action_with(c, f, a1::call::Action::Commit { case_id, manifest: ev.manifest }, Some(&ev.bytes))? else { unreachable!("an answer is an object") };
        m.extend(shared::evidence_rows(&ev));
        return Ok(Out::O(m));
    }
    let o = order_at(c, &who, &instance, None, &case)?;
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let mut post = ActPost::new(c, f, &w, &who);
    let r = eravon_dispute::evidence::commit(&c.data, &c.writer, c.now, &who, &sw, &o, &ev, &mut post);
    let mut rows = vec![("action", Out::S("commit".into())), ("instance", Out::S(instance))];
    rows.extend(post.finish("S6", r)?);
    rows.extend(shared::evidence_rows(&ev));
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub(super) fn advance(c: &Ctx, f: &Flags) -> R<Out> {
    let case_id = a1::order::b32_of("--case", &f.opt("--case").ok_or_else(|| Fault::Arg("要 --case".into()))?)?;
    let seat = f.int("--seat").map_err(|_| Fault::Arg("要 --seat".into()))?;
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    shared::advance_gate(f, c, &instance, &case_id, seat)?;
    action(c, f, a1::call::Action::Advance { case_id, seat })
}

pub(super) fn redraw(c: &Ctx, f: &Flags) -> R<Out> {
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let (members, amanah_hashes) = shared::redraw_lists(f, &w, &instance)?;
    action(c, f, a1::call::Action::Redraw { members, amanah_hashes })
}

pub(super) fn sweep(c: &Ctx, f: &Flags) -> R<Out> {
    let who = f.opt("--who").ok_or_else(|| Fault::Arg("要 --who".into()))?;
    let token = f.opt("--token").ok_or_else(|| Fault::Arg("要 --token".into()))?;
    action(c, f, a1::call::Action::Sweep { who, token })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unread_hand_over_is_said_in_its_member() {
        let text = |o: &Out| String::from_utf8(eravon_literals::json::write(o)).unwrap();
        let got = hand_over_row(Err(("E_CHAIN_DISAGREE".into(), "eth_getBlockByNumber 各家说法不一".into())));
        assert_eq!(text(&got), r#"{"detail":"eth_getBlockByNumber 各家说法不一","name":"E_CHAIN_DISAGREE"}"#);
        let read = eravon_deliver::deliver::HandOver { claim_block: 5, finalized: 8, ready: true };
        assert_eq!(text(&hand_over_row(Ok(read))), r#"{"claimBlock":5,"finalized":8,"ready":true}"#);
    }
}
