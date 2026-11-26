use super::*;

pub(super) fn record_confirm(c: &Ctx, who: &str, instance: &str, sig: &str) -> R<bool> {
    let owners = owners(c, who)?;
    record_confirm_in(&c.write()?, &owners, who, instance, sig, c.now)
}

pub(super) fn owners(c: &Ctx, who: &str) -> R<Vec<String>> {
    let s = c.session(None)?;
    Ok(eravon_vault::keyset::buyer(&s, who).map_err(|e| Fault::State(e.detail()))?.owners)
}

pub(super) fn pack_of(c: &Ctx, who: &str, instance: &str) -> R<Option<Vec<u8>>> {
    let store = c.read()?;
    for owner in owners(c, who)? {
        let Ok(rows) = store.list(&owner, Box_::Packs) else { continue };
        if let Some(r) = rows.into_iter().find(|r| r.fields.iter().any(|(k, v)| k == "instance" && v.eq_ignore_ascii_case(instance))) {
            return Ok(Some(store.part(&owner, Box_::Packs, &r.id, "pack.json")?));
        }
    }
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
pub fn keep_part_in(s: &eravon_vault::store::Store, owner: &str, id: &str, case_id: &str, instance: &str, name: &str, bytes: Vec<u8>, now: u64) -> Result<(), eravon_store::fault::Fault> {
    let mut fields = match s.get(owner, Box_::Packs, id) {
        Ok(old) => old.fields,
        Err(eravon_store::fault::Fault::Missing(_)) => Vec::new(),
        Err(e) => return Err(e),
    };
    for (k, v) in [("caseId", case_id), ("instance", instance)] {
        if !fields.iter().any(|(x, _)| x == k) {
            fields.push((k.to_string(), v.to_string()));
        }
    }
    fields.sort();
    s.put(owner, Kind::OrderFile, id, &fields, &[(name.to_string(), bytes)], now)?;
    Ok(())
}

pub(super) fn keep_evidence(c: &Ctx, who: &str, instance: &str, case_id: &str, name: &str, bytes: Vec<u8>) -> R<()> {
    let s = c.write()?;
    let mut at = None;
    for owner in owners(c, who)? {
        let rows = match s.list(&owner, Box_::Packs) {
            Ok(r) => r,
            Err(eravon_store::fault::Fault::Missing(_)) => continue,
            Err(e) => return Err(e.into()),
        };
        if let Some(r) = rows.into_iter().find(|r| r.kind == Kind::OrderFile && r.field("instance").is_some_and(|i| i.eq_ignore_ascii_case(instance))) {
            at = Some((owner, r.id));
            break;
        }
    }
    let (owner, id) = at.unwrap_or_else(|| (who.to_ascii_lowercase(), case_id.get(2..18).unwrap_or(case_id).to_string()));
    keep_part_in(&s, &owner, &id, case_id, instance, name, bytes, c.now).map_err(Fault::from)
}

pub fn record_confirm_in(s: &eravon_vault::store::Store, owners: &[String], by: &str, instance: &str, sig: &str, now: u64) -> R<bool> {
    use eravon_orders::table::{CONFIRM_BY, CONFIRM_HANDED};
    for owner in owners {
        let rows = match s.list(owner, Box_::Packs) {
            Ok(r) => r,
            Err(eravon_store::fault::Fault::Missing(_)) => continue,
            Err(e) => return Err(e.into()),
        };
        let Some(r) = rows.into_iter().find(|r| r.kind == Kind::OrderFile && r.field("instance").is_some_and(|i| i.eq_ignore_ascii_case(instance))) else {
            continue;
        };
        let mut fields: Vec<(String, String)> = r.fields.iter().filter(|(k, _)| k != CONFIRM_HANDED && k != CONFIRM_BY && k != "confirmSig").cloned().collect();
        fields.push((CONFIRM_HANDED.to_string(), now.to_string()));
        fields.push((CONFIRM_BY.to_string(), by.to_ascii_lowercase()));
        fields.push(("confirmSig".to_string(), sig.to_string()));
        s.put(owner, Kind::OrderFile, &r.id, &fields, &[], now)?;
        return Ok(true);
    }
    Ok(false)
}

pub(super) fn past_at(c: &Ctx, who: &str, instance: &str) -> R<()> {
    if eravon_literals::constants::past_rows().is_empty() {
        return Ok(());
    }
    let s = c.read()?;
    for owner in owners(c, who)? {
        let Ok(rows) = s.list(&owner, Box_::Packs) else { continue };
        for r in rows.into_iter().filter(|r| r.kind == Kind::OrderFile && r.field("instance").is_some_and(|i| i.eq_ignore_ascii_case(instance))) {
            if let Some(pk) = s.part(&owner, Box_::Packs, &r.id, "pack.json").ok().and_then(|b| k3::pack::pack(&b).ok()) {
                a1::chain::listing_writable(&pk.listing.logic)?;
            }
        }
    }
    Ok(())
}

pub struct Cancel {
    pub case_id: String,
    pub instance: String,
    pub deploy: Option<a1::call::Call>,
    pub cancel: a1::call::Call,
}

fn opened(phase: u64) -> Fault {
    Fault::Down {
        layer: "V2",
        token: E_CANCEL_OPENED.into(),
        why: format!("案已开(阶段 {phase}),撤单不成:请卖家退款(refund),或等案件自己的钟(判官未接单或交付逾期即退回)"),
    }
}

fn phase_of(w: &shared::Wire, instance: &str) -> R<u64> {
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    Ok(eravon_watch::poll::phase(&sw, instance).map_err(|(token, why)| Fault::Down { layer: "S4", token, why })?.0)
}

pub fn cancel_calls(w: &shared::Wire, o: &a1::order::Order, logic: &str) -> R<Cancel> {
    let d = o.derive(w.chain_id, logic)?;
    let phase = phase_of(w, &d.instance)?;
    if phase != 0 {
        return Err(opened(phase));
    }
    stage_lets_cancel(w, o)?;
    let (code, _) = eravon_chain::read::code(&w.t, &w.table, &d.instance, eravon_chain::seam::TAG_LATEST)?;
    let deploy = if code.is_empty() { Some(a1::call::deploy_instance(&d.salt, logic)?) } else { None };
    Ok(Cancel { case_id: hex(&d.salt), instance: d.instance.clone(), deploy, cancel: a1::call::cancel_at(o, &d.instance, &[], logic)? })
}

fn stage_lets_cancel(w: &shared::Wire, o: &a1::order::Order) -> R<()> {
    if o.prev.eq_ignore_ascii_case(a1::order::NO_PREV) {
        return Ok(());
    }
    let got = a1::chain::stage(&w.t, &w.table, &o.prev).map(|r| r.value).map_err(|e| e.detail());
    match cancel_held(&o.prev, &got) {
        None => Ok(()),
        Some(why) => Err(a1::fault::Fault::Precheck { gate: a1::precheck::GATE_STAGE, why }.into()),
    }
}


fn cancel_held(prev: &str, got: &Result<a1::chain::Stage, String>) -> Option<String> {
    match got {
        Ok(s) if s.ended && s.buyer_bps > 0 => None,
        Ok(s) if !s.ended => Some(format!("StageHeld:前段 {prev} 还没结束,这一段撤不了")),
        Ok(_) => Some(format!("StageHeld:前段 {prev} 结束时买家没有份额,这一段撤不了")),
        Err(e) => Some(format!("StageHeld:前段 {prev} 读不成本标准的实例:{e}")),
    }
}

pub fn cancel_lost(w: &shared::Wire, instance: &str, reverted: Fault) -> Fault {
    match phase_of(w, instance) {
        Ok(p) if p != 0 => opened(p),
        _ => reverted,
    }
}

pub(super) fn cancel(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let h = seller::held(c, f, &w, &who)?;
    h.writable()?;
    let k = cancel_calls(&w, &h.o, &h.logic)?;
    let mut rows = vec![("caseId", Out::S(k.case_id.clone())), ("instance", Out::S(k.instance.clone()))];
    match seller::two_calls(c, f, &w, &who, k.deploy.as_ref(), &k.cancel) {
        Ok(r) => rows.extend(r),
        Err(e @ Fault::Reverted { .. }) => return Err(cancel_lost(&w, &k.instance, e)),
        Err(e) => return Err(e),
    }
    rows.extend(seller::kept_end(c, f, &h, eravon_watch::order::State::Cancelled));
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub const E_CANCEL_OPENED: &str = "E_CANCEL_OPENED";

pub(super) fn attach(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let l = linked(c, f)?;
    let p = &l.s;
    let seg = f.opt("--segment").ok_or_else(|| Fault::Arg("要 --segment(goods 或 delivery)".into()))?;
    let (key, part) = match seg.as_str() {
        "goods" => ("goodsHash", eravon_goods::goods::PART_GOODS_TEXT),
        "delivery" => ("deliveryHash", eravon_goods::goods::PART_DELIVERY_TEXT),
        other => return Err(Fault::Arg(format!("--segment 只收 goods 与 delivery,给的是 {other:?}"))),
    };
    let want = p.terms.block.get(key).and_then(eravon_galeed::Value::as_str).ok_or_else(|| Fault::Down { layer: "V2", token: "E_SEGMENT_INLINE".into(), why: format!("这份条款的 {seg} 段是明文,不用另附") })?.to_string();
    let path = f.opt("--file").ok_or_else(|| Fault::Arg("要 --file(私密正文)".into()))?;
    let text = String::from_utf8(home::read_bounded(&path)?).map_err(|_| Fault::Arg(format!("{path} 不是 UTF-8 文字")))?;
    let got = eravon_goods::terms::segment_hash(&text);
    if !got.eq_ignore_ascii_case(&want) {
        return Err(Fault::Down { layer: "V2", token: "E_SEGMENT_HASH".into(), why: format!("这份正文的 sha256 是 {got},条款写的是 {want}") });
    }
    let id = seller::record_id(Kind::Payload, None, &p.terms_hash[2..18]);
    let s = c.write()?;
    let fields = match s.get(&who, Box_::Products, &id) {
        Ok(r) => r.fields,
        Err(eravon_store::fault::Fault::Missing(_)) => vec![("termsHash".into(), p.terms_hash.clone())],
        Err(e) => return Err(e.into()),
    };
    s.put(&who, Kind::Payload, &id, &fields, &[(eravon_goods::goods::PART_LISTING.into(), l.listing.clone()), (part.into(), text.clone().into_bytes())], c.now)?;
    let mut rows = vec![("hash", Out::S(got)), ("segment", Out::S(seg)), ("stored", Out::S(id))];
    if let Some(oid) = f.opt("--id") {
        let old = s.part(&who, Box_::Packs, &oid, "pack.json")?;
        let pk = k3::pack::pack(&old).map_err(Fault::from)?;
        let mut texts: Vec<String> = pk.attachments.iter().map(|a| a.text_md.clone()).filter(|t| *t != text).collect();
        texts.push(text);
        let receipt = s.part(&who, Box_::Packs, &oid, "receipt.json")?;
        let pack = write::pack(pk.chain_id, &pk.instance, &pk.case_id, &l.link, &l.listing, &receipt, &texts)?;
        let r = s.get(&who, Box_::Packs, &oid)?;
        s.put(&who, Kind::OrderFile, &oid, &r.fields, &[("pack.json".into(), pack)], c.now)?;
        rows.push(("pack", Out::S(oid)));
    }
    Ok(ok(rows))
}

pub(super) fn offers(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let dw = eravon_dispute::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let (case_id, seller_key, _, block) = eravon_dispute::settle::opened(&dw, &instance).map_err(p6)?;
    let case = a1::order::b32_of("案号", &case_id)?;
    let got = eravon_scan::board::settles_by_case(&w.t, &w.table, &case, w.chain_id, &instance, &seller_key, block).map_err(Fault::from)?;
    let now = a1::precheck::chain_now(&w.t, &w.table)?;
    let list: Vec<Out> = got
        .iter()
        .map(|(p, st)| {
            Out::O(vec![
                ("buyerBps", Out::N(st.buyer_bps)),
                ("document", Out::S(String::from_utf8_lossy(&p.document).into_owned())),
                ("expiry", Out::N(st.expiry)),
                ("live", Out::B(now <= st.expiry)),
                ("tx", Out::S(p.tx.clone())),
            ])
        })
        .collect();
    let mut rows = vec![("caseId", Out::S(case_id)), ("count", Out::N(list.len() as u64)), ("instance", Out::S(instance)), ("offers", Out::A(list))];
    rows.extend(w.rows());
    Ok(ok(rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_later_stage_cancels_only_over_a_stage_the_buyer_won() {
        let st = |ended, buyer_bps| a1::chain::Stage { seller_key: String::new(), buyer_key: String::new(), ended, buyer_bps, series_judge: String::new(), case_id: [0; 32], past: false, logic: String::new() };
        assert_eq!(cancel_held("0xprev", &Ok(st(true, 5000))), None, "结束且买家有份额:可撤");
        assert!(cancel_held("0xprev", &Ok(st(false, 5000))).is_some_and(|w| w.contains("还没结束")));
        assert!(cancel_held("0xprev", &Ok(st(true, 0))).is_some_and(|w| w.contains("没有份额")));
        assert!(cancel_held("0xprev", &Err("E_A1_CODEHASH".into())).is_some_and(|w| w.contains("读不成") && w.starts_with("StageHeld")));
    }

    #[test]
    fn a_confirmation_handed_out_is_recorded_on_its_order_file_at_once() {
        let data = eravon_vault::testing::fresh();
        let mut v = eravon_vault::vault::create("48261537", Seat::Buyer, "主钥", None, 1_790_000_000).expect("建得出库");
        v.write(&data).expect("落得下盘");
        let mut session = eravon_vault::session::Session::locked(&data);
        session.unlock("48261537").expect("开得了");
        let me = v.address.clone();
        let (mine, other) = (format!("0x{}", "c2".repeat(20)), format!("0x{}", "d3".repeat(20)));
        let s = eravon_vault::store::open_write(&data, "eravon-test", 1_790_000_000).expect("可写");
        s.init(&eravon_store::settings::Global::default()).expect("铺得下");
        s.init_identity(&me, 1_790_000_000).expect("登记身份");
        for (id, inst) in [("p1", &mine), ("p2", &other)] {
            s.put(&me, Kind::OrderFile, id, &[("instance".to_string(), inst.clone())], &[], 1_790_000_000).expect("落档");
        }
        let by = format!("0x{}", "a6".repeat(20));
        let owners = [by.clone(), me.clone()];
        assert!(record_confirm_in(&s, &owners, &by, &mine.to_ascii_uppercase().replacen("0X", "0x", 1), "0xsig", 1_790_000_123).expect("记得下"));
        let got = |id: &str, k: &str| s.get(&me, Box_::Packs, id).unwrap().field(k).map(str::to_string);
        assert_eq!(got("p1", eravon_orders::table::CONFIRM_HANDED).as_deref(), Some("1790000123"));
        assert_eq!(got("p1", eravon_orders::table::CONFIRM_BY), Some(by.clone()), "the key that signed");
        assert_eq!(got("p1", "confirmSig").as_deref(), Some("0xsig"));
        assert_eq!(got("p2", eravon_orders::table::CONFIRM_HANDED), None, "别一单不动");
        assert!(!record_confirm_in(&s, &owners, &by, &format!("0x{}", "d4".repeat(20)), "0xsig", 1_790_000_124).expect("答得出"), "没有订单档:不记");
    }
}
