use super::*;

pub(super) fn fresh_id(s: &eravon_vault::store::Store, data: &str, me: &str, now: u64) -> Result<String, Fault> {
    let cells = eravon_vault::store::archive_of(data, me)?;
    let base = format!("p{now}");
    let mut id = base.clone();
    let mut n = 1;
    while cells.iter().any(|o| s.get(o, Box_::Products, &id).is_ok()) {
        n += 1;
        id = format!("{base}-{n}");
    }
    Ok(id)
}

pub fn save(c: &Ctx<'_>, id: Option<&str>, d: &Draft) -> Result<Saved, Fault> {
    save_with(c, id, d, &[], false, false, None)
}

pub fn publish(c: &Ctx<'_>, id: Option<&str>, d: &Draft) -> Result<Saved, Fault> {
    save_with(c, id, d, &[], false, true, None)
}

pub fn publish_seen(c: &Ctx<'_>, id: Option<&str>, d: &Draft, seen: &[crate::resolve::Fee]) -> Result<Saved, Fault> {
    save_with(c, id, d, &[], false, true, Some(seen))
}

pub fn publish_recorded(c: &Ctx<'_>, id: Option<&str>, d: &Draft) -> Result<Saved, Fault> {
    let unseen = || Fault::Unseen("这一件还没有存过、示过费用:先保存,看过费用再发布".into());
    let i = id.ok_or_else(unseen)?;
    let seen = match prev_of(c, i)? {
        Ok(r) => r.field(field::FEES_SHOWN).and_then(crate::resolve::fees_of_text).filter(|f| !f.is_empty()).ok_or_else(unseen)?,
        Err(eravon_store::fault::Fault::Missing(_)) => return Err(unseen()),
        Err(e) => return Err(e.into()),
    };
    save_with(c, id, d, &[], false, true, Some(&seen))
}

pub fn confirm_fees(c: &Ctx<'_>, id: &str) -> Result<Saved, Fault> {
    let rec = prev_of(c, id)??;
    let seen = rec
        .field(field::FEES_NEW)
        .and_then(crate::resolve::fees_of_text)
        .filter(|f| !f.is_empty())
        .ok_or_else(|| Fault::Nothing(format!("{id} 没有待确认的新费用")))?;
    let why: Vec<String> = rec.field(field::PENDING).unwrap_or_default().split(',').filter(|w| !w.is_empty() && *w != super::FEES_DUE && *w != POST).map(str::to_string).collect();
    let d = load(c.data, c.me, id)?;
    save_with(c, Some(id), &d, &why, true, true, Some(&seen))
}

fn seen_check(now: &[crate::resolve::Fee], seen: &[crate::resolve::Fee]) -> Result<(), Fault> {
    if now.len() != seen.len() || now.iter().zip(seen).any(|(a, b)| !a.same_terms(b)) {
        return Err(Fault::Unseen("要签的费用与确认页所示不同(候选、报价或价格变了):先保存,看过费用再发布".into()));
    }
    Ok(())
}


fn resign_seen(now: &[crate::resolve::Fee], base: Option<&[crate::resolve::Fee]>) -> bool {
    base.is_some_and(|b| crate::resolve::same_fees(now, b))
}


fn prev_of(c: &Ctx<'_>, id: &str) -> Result<Result<eravon_store::record::Record, eravon_store::fault::Fault>, Fault> {
    let s = eravon_vault::store::open_read(c.data)?;
    let owner = super::view::owner_in(&s, c.data, c.me, id)?;
    Ok(s.get(&owner, Box_::Products, id))
}

pub(super) fn save_with(c: &Ctx<'_>, id: Option<&str>, d: &Draft, why: &[String], auto: bool, publish: bool, seen: Option<&[crate::resolve::Fee]>) -> Result<Saved, Fault> {
    crate::mark::mark(crate::mark::FEATURE);
    if let Some(i) = id {
        super::view::here_or_refused(c.data, c.me, i)?;
    }
    let (prev, listed, signed_prev) = match id {
        Some(i) => match prev_of(c, i)? {
            Ok(r) => (
                r.field(field::VERSION).and_then(|x| x.parse::<u64>().ok()).unwrap_or(0),
                r.field(field::LISTED).map(str::to_string),
                r.field(field::FEES_SIGNED).and_then(crate::resolve::fees_of_text),
            ),
            Err(eravon_store::fault::Fault::Missing(_)) => (0, None, None),
            Err(e) => return Err(e.into()),
        },
        None => (0, None, None),
    };
    let co = crate::company::of(c.data, c.me, &c.route, c.registries, c.now);
    let (verdict, resolved, made, decimals) = judge_with(c, d, id, prev + 1, &co);
    gate(&verdict)?;
    if let (true, Some(seen), Some(_)) = (publish, seen, made.as_ref()) {
        seen_check(&resolved.fees, seen)?;
    }
    if auto && made.is_some() {
        let base = seen.map(<[_]>::to_vec).or(signed_prev);
        if !resign_seen(&resolved.fees, base.as_deref()) {
            let id = id.map(str::to_string).ok_or_else(|| Fault::Missing(String::new()))?;
            let mut owed = why.to_vec();
            if !owed.iter().any(|w| w == super::FEES_DUE) {
                owed.push(super::FEES_DUE.into());
            }
            keep_pending_with(c, &id, &owed, super::E_GOODS_FEES_CHANGED, &[(field::FEES_NEW.to_string(), crate::resolve::fees_text(&resolved.fees))])?;
            return Ok(Saved {
                id,
                draft: false,
                version: prev,
                terms_hash: None,
                verdict,
                resolved,
                link: None,
                link_state: Some((super::E_GOODS_FEES_CHANGED.into(), "费用变了:新费用待卖家确认,确认之后才重签".into())),
                company_refused: None,
                pending: owed,
            });
        }
    }
    let made = made.filter(|_| publish);
    let bytes = made.as_ref().map(|m| m.bytes.clone());
    let company = co.ok().flatten();
    if auto && bytes.is_none() {
        let id = id.map(str::to_string).ok_or_else(|| Fault::Missing(String::new()))?;
        let first = verdict.missing.first().map(|m| m.name.clone()).unwrap_or_else(|| "E_GOODS_NO_LINK".into());
        keep_pending(c, &id, why, &first)?;
        return Ok(Saved {
            id,
            draft: false,
            version: prev,
            terms_hash: None,
            verdict,
            resolved,
            link: None,
            link_state: Some((first, "自动重签未完成,已保留原样".into())),
            company_refused: None,
            pending: why.to_vec(),
        });
    }
    let mut link = None;
    let mut link_state = None;
    let mut company_refused: Option<(String, String)> = None;
    let mut pending: Vec<String> = Vec::new();
    let mut signed: Vec<String> = Vec::new();
    let mut exported: Option<crate::company::Exported> = None;
    if let Some(b) = &bytes {
        match sigs_of(c, company.as_ref(), b) {
            Err(w) => presign_retry(&mut pending, &mut link_state, w),
            Ok(sigs) => {
                signed = sigs.clone();
                match crate::link::make(b, &sigs, &resolved) {
                    Ok(m) => {
                        if let Some(co) = &company {
                            match crate::company::export(co, &c.route, b, &m.listing, c.now, &quotes_of(&resolved)) {
                                Ok(x) => exported = Some(x),
                                Err(w) => company_refused = Some(w),
                            }
                        }
                        link = Some(m)
                    }
                    Err(w) => presign_retry(&mut pending, &mut link_state, w),
                }
            }
        }
    }
    if !c.signer.live() {
        let (name, detail) = crate::sign::stale();
        return Err(Fault::Store { name: "E_GOODS_STALE", detail: format!("{name}:{detail}") });
    }
    let s = eravon_vault::store::open_write(c.data, c.writer, c.now)?;
    let id = match id {
        Some(i) => i.to_string(),
        None => fresh_id(&s, c.data, c.me, c.now)?,
    };
    let owner = super::view::owner_in(&s, c.data, c.me, &id)?;
    let formal = bytes.is_some();
    let version = if formal { prev + 1 } else { prev };
    let old = s.get(&owner, Box_::Products, &id).ok();
    let chain = match old.as_ref().and_then(|o| super::view::chain_of(&s, &owner, o)) {
        Some(ch) => ch,
        None => super::view::chain_here(c.data, c.me)?,
    };
    if let Some(old) = old.as_ref().filter(|o| !formal && stays_live(Some(o))) {
        let mut fields: Vec<(String, String)> = old.fields.iter().filter(|(k, _)| k != field::EDITING && k != field::CHAIN && (resolved.fees.is_empty() || k != field::FEES_SHOWN)).cloned().collect();
        fields.push((field::EDITING.to_string(), "draft".into()));
        fields.push((field::CHAIN.to_string(), chain.to_string()));
        if !resolved.fees.is_empty() {
            fields.push((field::FEES_SHOWN.to_string(), crate::resolve::fees_text(&resolved.fees)));
        }
        s.put(&owner, Kind::Product, &id, &fields, &[(edit_part().to_string(), d.seal())], c.now)?;
        return Ok(Saved { id, draft: true, version: prev, terms_hash: None, verdict, resolved, link: None, link_state, company_refused, pending });
    }
    let mut fields = vec![
        (field::TITLE.to_string(), d.name.clone()),
        (field::DRAFT.to_string(), (!formal).to_string()),
        (field::VERSION.to_string(), version.to_string()),
        (field::LISTED.to_string(), listed.unwrap_or_else(|| "true".into())),
        (field::CHAIN.to_string(), chain.to_string()),
    ];
    let mut terms_hash = None;
    if let Some(b) = &bytes {
        let h = eravon_galeed::hash::hex_encode(&eravon_terms::doc::terms_hash(b));
        let tid = format!("terms-{id}-v{version}");
        let mut tparts = vec![(PART_TERMS.to_string(), b.clone())];
        if let Some(m) = &made {
            for (name, text) in [(PART_GOODS_TEXT, &m.goods_text), (PART_DELIVERY_TEXT, &m.delivery_text)] {
                if let Some(t) = text {
                    tparts.push((name.to_string(), t.as_bytes().to_vec()));
                }
            }
        }
        s.put(&owner, Kind::TermsDocument, &tid, &[(field::TERMS_HASH.into(), h.clone())], &tparts, c.now)?;
        fields.push((field::TERMS_HASH.to_string(), h.clone()));
        fields.push((field::TERMS_ID.to_string(), tid));
        terms_hash = Some(h);
    }
    if !signed.is_empty() && company.is_none() {
        let rows: Vec<(String, String)> = signed.iter().enumerate().map(|(i, sig)| (format!("c{i}"), sig.clone())).collect();
        s.put(&owner, Kind::OfferSignature, &format!("sig-{id}-v{version}"), &rows, &[], c.now)?;
    }
    if let Some(m) = &link {
        put_listing(&s, c, &owner, &id, version, m, decimals, exported.as_ref())?;
        if !pending.iter().any(|p| p == POST) {
            pending.push(POST.into());
        }
        if link_state.is_none() {
            link_state = Some((crate::link::E_GOODS_LINK_UNPOSTED.into(), "清单尚未发上公告板,发布后才有付款链接".into()));
        }
        if let Some(co) = &company {
            company_marks(&mut fields, exported.as_ref(), co);
            if let Some((n, d)) = &company_refused {
                fields.push((field::COMPANY_REFUSED.to_string(), format!("{n}|{d}")));
            }
        }
    } else if formal {
        if let Some(old) = &old {
            for k in kept_fields() {
                if let Some(v) = old.field(k) {
                    fields.push((k.to_string(), v.to_string()));
                }
            }
        }
    }
    let shown_now = !resolved.fees.is_empty() && (!auto || seen.is_some());
    if shown_now {
        fields.push((field::FEES_SHOWN.to_string(), crate::resolve::fees_text(&resolved.fees)));
    } else if let Some(v) = old.as_ref().and_then(|o| o.field(field::FEES_SHOWN)) {
        fields.push((field::FEES_SHOWN.to_string(), v.to_string()));
    }
    if formal && !resolved.fees.is_empty() {
        fields.push((field::FEES_SIGNED.to_string(), crate::resolve::fees_text(&resolved.fees)));
    }
    if auto && old.as_ref().is_some_and(|o| o.field(field::EDITING).is_some()) {
        fields.push((field::EDITING.to_string(), "draft".into()));
    }
    if let Some((n, _)) = &link_state {
        fields.push((field::LINK_STATE.to_string(), n.clone()));
    }
    if !pending.is_empty() {
        fields.push((field::PENDING.to_string(), pending.join(",")));
    }
    s.put(&owner, Kind::Product, &id, &fields, &[(PART_DRAFT.into(), d.seal())], c.now)?;
    drop(s);
    Ok(Saved { id, draft: !formal, version, terms_hash, verdict, resolved, link: None, link_state, company_refused, pending })
}

fn quotes_of(r: &Resolved) -> Vec<String> {
    r.judges.iter().map(|j| j.quote.clone()).chain(r.guilds.iter().map(|g| g.quote.clone())).collect()
}

pub const POST: &str = "post";

pub const PRESIGN: &str = "presign";

fn put_listing(s: &eravon_vault::store::Store, c: &Ctx<'_>, owner: &str, id: &str, version: u64, m: &crate::link::Made, decimals: Option<u64>, exported: Option<&crate::company::Exported>) -> Result<(), Fault> {
    let mut parts = vec![(PART_LISTING.to_string(), m.listing.clone())];
    if let Some(x) = exported {
        parts.push((PART_PROPOSAL.to_string(), x.proposal.clone()));
        for o in &x.docs {
            if !parts.iter().any(|(n, _)| *n == o.name) {
                parts.push((o.name.clone(), o.bytes.clone()));
            }
        }
    }
    s.put(
        owner,
        Kind::Payload,
        &listing_id(id, version),
        &[
            (field::DECIMALS.into(), decimals.map(|x| x.to_string()).unwrap_or_default()),
            (field::EXPIRY.into(), m.expiry.to_string()),
            (field::VERSION.into(), version.to_string()),
        ],
        &parts,
        c.now,
    )?;
    Ok(())
}

pub trait Sender {
    fn send(&mut self, call: &eravon_standard::call::Call) -> Result<String, (String, String)>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Posted {
    pub link: String,
    pub tx: String,
    pub hash: String,
    pub expiry: u64,
}

fn chain_fault((name, detail): (String, String)) -> Fault {
    Fault::Chain { name, detail }
}

fn unsent((name, detail): (String, String)) -> Fault {
    Fault::Unsent { name, detail }
}

pub fn post(c: &Ctx<'_>, id: &str, sender: &mut dyn Sender) -> Result<Posted, Fault> {
    crate::mark::mark(crate::mark::FEATURE);
    super::view::here_or_refused(c.data, c.me, id)?;
    let r = eravon_vault::store::open_read(c.data)?;
    let owner = super::view::owner_in(&r, c.data, c.me, id)?;
    let rec = r.get(&owner, Box_::Products, id)?;
    if rec.field(field::REVOKED).is_some() {
        return Err(Fault::Revoked(id.to_string()));
    }
    if rec.field(field::DRAFT) != Some("false") {
        return Err(Fault::Nothing(format!("{id} 还是草稿:先发布(product-publish)再上板")));
    }
    let version: u64 = rec.field(field::VERSION).and_then(|x| x.parse().ok()).unwrap_or(0);
    posted_seen(&rec, id, version)?;
    let lid = listing_id(id, version);
    let lrec = match r.get(&owner, Box_::Products, &lid) {
        Ok(x) => x,
        Err(eravon_store::fault::Fault::Missing(_)) => return Err(Fault::Nothing(format!("{id} 第 {version} 版的清单没建成({}):先补签", rec.field(field::LINK_STATE).unwrap_or(PRESIGN)))),
        Err(e) => return Err(e.into()),
    };
    let listing = r.part(&owner, Box_::Products, &lid, PART_LISTING)?;
    let expiry: u64 = lrec.field(field::EXPIRY).and_then(|x| x.parse().ok()).unwrap_or(0);
    if let (Some(tx), Some(link)) = (lrec.field(field::POSTED_TX), lrec.field(field::LINK)) {
        return Ok(Posted { link: link.to_string(), tx: tx.to_string(), hash: lrec.field(field::POSTED_HASH).unwrap_or_default().to_string(), expiry });
    }
    let tid = rec.field(field::TERMS_ID).ok_or_else(|| Fault::Nothing(format!("{id} 没有条款")))?.to_string();
    let terms = r.part(&owner, Box_::Products, &tid, PART_TERMS)?;
    let terms_hash = eravon_terms::doc::terms_hash(&terms);
    drop(r);
    let t = eravon_terms::doc::terms(&terms).map_err(|f| Fault::Form(format!("{id}:{}", f.token.name())))?;
    handed_on(&eravon_vault::store::keys_of(c.data, c.me)?, c.me, id, &t.seller_key)?;
    company_proposes(&[c.me.to_string()], id, &t.seller_key)?;
    key_stands(c, true).map_err(|(name, detail)| if name == E_GOODS_KEY_HANDED_ON { Fault::HandedOn(detail) } else { Fault::Chain { name, detail } })?;
    let call = crate::link::post_call(&terms_hash, &listing).map_err(chain_fault)?;
    let tx = sender.send(&call).map_err(unsent)?;
    let hash = eravon_galeed::hash::sha256(&listing);
    let (t, table) = c.route.clone().map_err(chain_fault)?;
    let seen = eravon_scan::board::by_tx(&t, &table, &tx, &hash).map_err(|e| Fault::Chain { name: e.name(), detail: e.detail() })?;
    if !seen.iter().any(|p| p.key == terms_hash) {
        return Err(Fault::Chain { name: E_GOODS_POST_UNSEEN.into(), detail: format!("交易 {tx} 的回执里读不到这份清单(公告板码、hash 与 key 三样)") });
    }
    let link = crate::link::link_of(eravon_literals::constants::active_standard().chain_id, &tx, &listing).map_err(chain_fault)?;
    let hash = eravon_galeed::hash::hex_encode(&hash);
    let s = eravon_vault::store::open_write(c.data, c.writer, c.now)?;
    let lrec = s.get(&owner, Box_::Products, &lid)?;
    let mut lf: Vec<(String, String)> = lrec.fields.iter().filter(|(k, _)| ![field::POSTED_TX, field::POSTED_HASH, field::LINK].contains(&k.as_str())).cloned().collect();
    lf.push((field::POSTED_TX.to_string(), tx.clone()));
    lf.push((field::POSTED_HASH.to_string(), hash.clone()));
    lf.push((field::LINK.to_string(), link.clone()));
    s.put(&owner, Kind::Payload, &lid, &lf, &[], c.now)?;
    let rec = s.get(&owner, Box_::Products, id)?;
    let mut fields: Vec<(String, String)> = rec.fields.iter().filter(|(k, _)| ![field::LINK, field::LINK_EXPIRY, field::LINK_STATE, field::PENDING].contains(&k.as_str())).cloned().collect();
    fields.push((field::LINK.to_string(), link.clone()));
    fields.push((field::LINK_EXPIRY.to_string(), expiry.to_string()));
    let left: Vec<&str> = rec.field(field::PENDING).unwrap_or_default().split(',').filter(|x| !x.is_empty() && *x != POST).collect();
    if !left.is_empty() {
        fields.push((field::PENDING.to_string(), left.join(",")));
    }
    s.put(&owner, Kind::Product, id, &fields, &[], c.now)?;
    Ok(Posted { link, tx, hash, expiry })
}

fn handed_on(archive: &[String], me: &str, id: &str, seller: &str) -> Result<(), Fault> {
    if !seller.eq_ignore_ascii_case(me) && archive.iter().any(|k| seller.eq_ignore_ascii_case(k)) {
        return Err(Fault::HandedOn(format!("{id} 这一版签的是换钥前的钥 {seller}:换钥之后登的清单谁也付不进;改一改再发布一版")));
    }
    Ok(())
}

fn company_proposes(keys: &[String], id: &str, seller: &str) -> Result<(), Fault> {
    if keys.iter().any(|k| seller.eq_ignore_ascii_case(k)) {
        return Ok(());
    }
    Err(Fault::Proposes(format!("{id} 是公司 {seller} 的商品:上板与撤单由公司执行提案,本席钥不发")))
}


pub const E_GOODS_COMPANY_PROPOSES: &str = "E_GOODS_COMPANY_PROPOSES";

fn posted_seen(rec: &eravon_store::record::Record, id: &str, version: u64) -> Result<(), Fault> {
    let read = |k: &str| rec.field(k).and_then(crate::resolve::fees_of_text).filter(|f| !f.is_empty());
    let (signed, shown) = (read(field::FEES_SIGNED), read(field::FEES_SHOWN));
    let unseen = || Fault::Unseen(format!("{id} 第 {version} 版签进去的费用({})与示过的费用({})对不上:先保存,看过费用再发布,再上板", fees_said(signed.as_deref()), fees_said(shown.as_deref())));
    let (Some(s), Some(w)) = (signed.as_deref(), shown.as_deref()) else { return Err(unseen()) };
    seen_check(s, w).map_err(|_| unseen())
}

fn fees_said(f: Option<&[crate::resolve::Fee]>) -> String {
    let Some(f) = f else { return "没有记下".into() };
    f.iter()
        .map(|x| match &x.priced {
            Ok(q) => format!("{} 金额 {} · 判官费 {} · 诉讼费 {} · 押金 {}", x.candidate, x.amount.to_dec(), q.premium.to_dec(), q.lf.to_dec(), q.deposit.to_dec()),
            Err((n, _)) => format!("{} 金额 {} · 费用没定出({n})", x.candidate, x.amount.to_dec()),
        })
        .collect::<Vec<_>>()
        .join(";")
}

pub const E_GOODS_POST_UNSEEN: &str = "E_GOODS_POST_UNSEEN";
pub const E_GOODS_KEY_HANDED_ON: &str = "E_GOODS_KEY_HANDED_ON";
pub const E_GOODS_REVOKE_UNSEEN: &str = "E_GOODS_REVOKE_UNSEEN";

pub fn revoke(c: &Ctx<'_>, id: &str, sender: &mut dyn Sender) -> Result<String, Fault> {
    crate::mark::mark(crate::mark::FEATURE);
    super::view::here_or_refused(c.data, c.me, id)?;
    let r = eravon_vault::store::open_read(c.data)?;
    let owner = super::view::owner_in(&r, c.data, c.me, id)?;
    let rec = r.get(&owner, Box_::Products, id)?;
    if let Some(tx) = rec.field(field::REVOKED) {
        return Ok(tx.to_string());
    }
    let tid = rec.field(field::TERMS_ID).ok_or_else(|| Fault::Nothing(format!("{id} 没有上过架的条款,无可下架")))?.to_string();
    let terms = r.part(&owner, Box_::Products, &tid, PART_TERMS)?;
    let t = eravon_terms::doc::terms(&terms).map_err(|f| Fault::Form(format!("{id}:{}", f.token.name())))?;
    drop(r);
    company_proposes(&eravon_vault::store::keys_of(c.data, c.me)?, id, &t.seller_key)?;
    let call = eravon_standard::call::revoke(&t.logic, &t.seller_key, &t.terms_hash).map_err(|f| Fault::Chain { name: f.name(), detail: f.detail() })?;
    let tx = sender.send(&call).map_err(unsent)?;
    let (tr, table) = c.route.clone().map_err(chain_fault)?;
    let on = eravon_standard::chain::revoked(&tr, &table, &t.logic, &t.seller_key, &t.terms_hash).map_err(|f| Fault::Chain { name: f.name(), detail: f.detail() })?;
    if !on.value {
        return Err(Fault::Chain { name: E_GOODS_REVOKE_UNSEEN.into(), detail: format!("交易 {tx} 已上链,逻辑合约读 revoked 仍为否") });
    }
    revoked_recorded(c.data, c.me, c.writer, c.now, id, &tx)?;
    Ok(tx)
}

pub fn revoked_recorded(data: &str, me: &str, writer: &str, now: u64, id: &str, tx: &str) -> Result<(), Fault> {
    let s = eravon_vault::store::open_write(data, writer, now)?;
    let owner = super::view::owner_in(&s, data, me, id)?;
    let rec = s.get(&owner, Box_::Products, id)?;
    let mut fields: Vec<(String, String)> = rec.fields.iter().filter(|(k, _)| ![field::LISTED, field::REVOKED].contains(&k.as_str())).cloned().collect();
    fields.push((field::LISTED.to_string(), "false".into()));
    fields.push((field::REVOKED.to_string(), tx.to_string()));
    s.put(&owner, Kind::Product, id, &fields, &[], now)?;
    Ok(())
}

pub fn revoke_proposal(c: &Ctx<'_>, id: &str, dir: &str) -> Result<String, Fault> {
    crate::mark::mark(crate::mark::FEATURE);
    super::view::here_or_refused(c.data, c.me, id)?;
    let s = eravon_vault::store::open_write(c.data, c.writer, c.now)?;
    let owner = super::view::owner_in(&s, c.data, c.me, id)?;
    let rec = s.get(&owner, Box_::Products, id)?;
    if rec.field(field::REVOKED).is_some() {
        return Err(Fault::Revoked(id.to_string()));
    }
    let tid = rec.field(field::TERMS_ID).ok_or_else(|| Fault::Nothing(format!("{id} 没有上过架的条款,无可下架")))?.to_string();
    let terms = s.part(&owner, Box_::Products, &tid, PART_TERMS)?;
    let t = eravon_terms::doc::terms(&terms).map_err(|f| Fault::Form(format!("{id}:{}", f.token.name())))?;
    if eravon_vault::store::keys_of(c.data, c.me)?.iter().any(|k| t.seller_key.eq_ignore_ascii_case(k)) {
        return Err(Fault::Nothing(format!("{id} 是本席钥的商品:直接链上下架")));
    }
    let (proposal, docs) = crate::company::revoke_export(&c.route, t.chain_id, &t.logic, &t.seller_key, &t.terms_hash).map_err(|(n, d)| Fault::Company(format!("{n}:{d}")))?;
    let version: u64 = rec.field(field::VERSION).and_then(|x| x.parse().ok()).unwrap_or(0);
    let out = super::view::revoke_dir(dir, id, version);
    let path = super::view::write_proposal(&out, &proposal, &docs)?;
    let mut fields: Vec<(String, String)> = rec.fields.iter().filter(|(k, _)| k != field::REVOKING).cloned().collect();
    fields.push((field::REVOKING.to_string(), path.clone()));
    s.put(&owner, Kind::Product, id, &fields, &[], c.now)?;
    Ok(path)
}

pub(super) fn company_marks(fields: &mut Vec<(String, String)>, exported: Option<&crate::company::Exported>, co: &crate::company::Company) {
    fields.push((field::COMPANY.to_string(), "awaiting".into()));
    fields.push((field::COMPANY_DIGESTS.to_string(), exported.map(|x| x.digests.join(",")).unwrap_or_default()));
    fields.push((field::COMPANY_ADDRESS.to_string(), co.address.clone()));
    if let Some(x) = exported.filter(|x| !x.time.enough) {
        fields.push((field::COMPANY_TIME.to_string(), format!("{}|{}", x.time.bound, x.time.say_short())));
    }
}

pub(super) fn stays_live(old: Option<&eravon_store::record::Record>) -> bool {
    old.is_some_and(|r| r.field(field::DRAFT) == Some("false"))
}


pub(super) fn kept_fields() -> &'static [&'static str] {
    &[field::LINK, field::LINK_EXPIRY, field::COMPANY, field::COMPANY_DIGESTS, field::COMPANY_ADDRESS, field::COMPANY_TIME, field::COMPANY_REFUSED]
}

pub(super) fn version_company(c: &Ctx<'_>, bytes: &[u8], co: Result<Option<crate::company::Company>, (String, String)>) -> Result<Option<crate::company::Company>, (String, String)> {
    let t = eravon_terms::doc::terms(bytes).map_err(|f| (f.token.name().to_string(), "无法读取条款".to_string()))?;
    if t.seller_key.eq_ignore_ascii_case(c.me) {
        return Ok(None);
    }
    match co? {
        Some(x) if x.address.eq_ignore_ascii_case(&t.seller_key) => Ok(Some(x)),
        _ => Err(("E_GOODS_SELLER_KEY".into(), format!("这一版条款的卖家是 {},当前身份不是它、公司模式也没开着那一家:补签不成", t.seller_key))),
    }
}

pub(super) fn sigs_of(c: &Ctx<'_>, company: Option<&crate::company::Company>, bytes: &[u8]) -> Result<Vec<String>, (String, String)> {
    match company {
        Some(_) => crate::company::unsigned(c.signer, bytes),
        None => {
            key_stands(c, false)?;
            crate::sign::sign_all(c.signer, bytes)
        }
    }
}

pub(super) fn key_stands(c: &Ctx<'_>, strict: bool) -> Result<(), (String, String)> {
    let read = c.route.clone().and_then(|(t, table)| eravon_standard::chain::current(&t, &table, c.me).map_err(|e| (e.name(), e.detail())));
    match read {
        Ok(r) if r.value.eq_ignore_ascii_case(c.me) => Ok(()),
        Ok(r) => Err((E_GOODS_KEY_HANDED_ON.into(), format!("这把钥已把记录交给 {}:交出之后签的要约、登的清单谁也付不进", r.value))),
        Err(e) if strict => Err(e),
        Err(_) => Ok(()),
    }
}

pub(super) fn presign_retry(pending: &mut Vec<String>, state: &mut Option<(String, String)>, w: (String, String)) {
    if !pending.iter().any(|p| p == PRESIGN) {
        pending.push(PRESIGN.into());
    }
    *state = Some(w);
}

pub(super) fn keep_pending(c: &Ctx<'_>, id: &str, why: &[String], state: &str) -> Result<(), Fault> {
    keep_pending_with(c, id, why, state, &[])
}

pub(super) fn keep_pending_with(c: &Ctx<'_>, id: &str, why: &[String], state: &str, extra: &[(String, String)]) -> Result<(), Fault> {
    let s = eravon_vault::store::open_write(c.data, c.writer, c.now)?;
    let owner = super::view::owner_in(&s, c.data, c.me, id)?;
    let rec = s.get(&owner, Box_::Products, id)?;
    let mut fields: Vec<(String, String)> = rec
        .fields
        .iter()
        .filter(|(k, _)| k != field::PENDING && k != field::LINK_STATE && !extra.iter().any(|(e, _)| e == k))
        .cloned()
        .collect();
    fields.push((field::PENDING.to_string(), why.join(",")));
    fields.push((field::LINK_STATE.to_string(), state.to_string()));
    fields.extend(extra.iter().cloned());
    s.put(&owner, Kind::Product, id, &fields, &[], c.now)?;
    Ok(())
}

pub fn sweep(c: &Ctx<'_>) -> Result<Vec<(String, Vec<String>, bool)>, Fault> {
    let rows = list(c.data, c.me, "")?;
    let table = eravon_cands::table::list(c.data, c.me);
    let mut asked: Vec<String> = table.as_ref().map(|t| t.iter().map(|r| r.address.clone()).collect()).unwrap_or_default();
    let mut docs: Vec<Vec<u8>> = Vec::new();
    for r in rows.iter().filter(|r| !r.draft) {
        let s = eravon_vault::store::open_read(c.data)?;
        let owner = super::view::owner_in(&s, c.data, c.me, &r.id)?;
        let Ok(rec) = s.get(&owner, Box_::Products, &r.id) else { continue };
        let Some(tid) = rec.field(field::TERMS_ID).map(str::to_string) else { continue };
        if let Ok(bytes) = s.part(&owner, Box_::Products, &tid, PART_TERMS) {
            asked.extend(crate::resign::named(&bytes));
            docs.push(bytes);
        }
    }
    let chain = match table {
        Ok(_) => eravon_cands::wire::Wire::on(&c.route, c.registries, c.now).and_then(|w| eravon_cands::registry::logs_of(&w, &asked).map(|l| (w, l))).ok(),
        Err(_) => None,
    };
    let read = chain.as_ref().map(|(w, l)| crate::resign::Read::of(w, l, &docs.iter().map(Vec::as_slice).collect::<Vec<_>>()));
    let mut out = Vec::new();
    for r in rows.iter().filter(|r| !r.draft) {
        let s = eravon_vault::store::open_read(c.data)?;
        let owner = super::view::owner_in(&s, c.data, c.me, &r.id)?;
        let rec = s.get(&owner, Box_::Products, &r.id)?;
        let Some(tid) = rec.field(field::TERMS_ID).map(str::to_string) else { continue };
        let Ok(bytes) = s.part(&owner, Box_::Products, &tid, PART_TERMS) else { continue };
        let triggers: Vec<String> = crate::resign::due_with(c.data, c.me, &bytes, c.now, read.as_ref())
            .into_iter()
            .map(|x| x.word().to_string())
            .collect();
        let mut why: Vec<String> = r.pending.iter().filter(|p| *p != POST).cloned().collect();
        for t in &triggers {
            if !why.contains(t) {
                why.push(t.clone());
            }
        }
        if why.is_empty() {
            continue;
        }
        let d = load(c.data, c.me, &r.id)?;
        if why.iter().all(|w| w == super::FEES_DUE) {
            out.push((r.id.clone(), why, false));
            continue;
        }
        let done = if why.iter().any(|w| crate::resign::Reason::of(w).is_some()) {
            let sv = save_with(c, Some(&r.id), &d, &why, true, true, None)?;
            !sv.draft && sv.pending.iter().all(|p| p == POST)
        } else {
            presign_now(c, &r.id, r.version, &d, &bytes, &rec)?
        };
        out.push((r.id.clone(), why, done));
    }
    Ok(out)
}

pub fn note_payout(data: &str, writer: &str, me: &str, now: u64) -> Result<usize, Fault> {
    note_due(data, writer, me, now, crate::resign::Reason::Payout)
}

pub fn note_fund(data: &str, writer: &str, me: &str, now: u64) -> Result<usize, Fault> {
    note_due(data, writer, me, now, crate::resign::Reason::Fund)
}

fn note_due(data: &str, writer: &str, me: &str, now: u64, reason: crate::resign::Reason) -> Result<usize, Fault> {
    let mut n = 0;
    for r in super::view::list_every_chain(data, me, "")?.into_iter().filter(|r| !r.draft) {
        let s = eravon_vault::store::open_read(data)?;
        let owner = super::view::owner_in(&s, data, me, &r.id)?;
        let rec = s.get(&owner, Box_::Products, &r.id)?;
        let Some(tid) = rec.field(field::TERMS_ID) else { continue };
        let Ok(bytes) = s.part(&owner, Box_::Products, tid, PART_TERMS) else { continue };
        if !crate::resign::due(data, me, &bytes, now, None).contains(&reason) {
            continue;
        }
        n += 1;
        if r.pending.iter().any(|p| p == reason.word()) {
            continue;
        }
        let mut why = r.pending.clone();
        why.push(reason.word().into());
        let w = eravon_vault::store::open_write(data, writer, now)?;
        let mut fields: Vec<(String, String)> = rec.fields.iter().filter(|(k, _)| k != field::PENDING).cloned().collect();
        fields.push((field::PENDING.to_string(), why.join(",")));
        w.put(&owner, Kind::Product, &r.id, &fields, &[], now)?;
    }
    Ok(n)
}

pub(super) fn presign_now(c: &Ctx<'_>, id: &str, version: u64, d: &Draft, bytes: &[u8], rec: &eravon_store::record::Record) -> Result<bool, Fault> {
    let co = crate::company::of(c.data, c.me, &c.route, c.registries, c.now);
    let (_, resolved, _, decimals) = judge_with(c, d, Some(id), version, &co);
    let company = version_company(c, bytes, co);
    let made = company.clone().and_then(|company| {
        let sigs = sigs_of(c, company.as_ref(), bytes)?;
        let m = crate::link::make(bytes, &sigs, &resolved)?;
        let x = company.as_ref().map(|co| crate::company::export(co, &c.route, bytes, &m.listing, c.now, &quotes_of(&resolved)));
        Ok((sigs, m, company, x))
    });
    let s = eravon_vault::store::open_write(c.data, c.writer, c.now)?;
    let owner = super::view::owner_in(&s, c.data, c.me, id)?;
    let mut fields: Vec<(String, String)> =
        rec.fields.iter().filter(|(k, _)| ![field::PENDING, field::LINK_STATE, field::LINK, field::LINK_EXPIRY].contains(&k.as_str())).cloned().collect();
    let ok = match made {
        Ok((sigs, m, company, x)) => {
            if company.is_none() {
                let rows: Vec<(String, String)> = sigs.iter().enumerate().map(|(i, sig)| (format!("c{i}"), sig.clone())).collect();
                s.put(&owner, Kind::OfferSignature, &format!("sig-{id}-v{version}"), &rows, &[], c.now)?;
            }
            let mut refused = None;
            let exported = match x {
                Some(Ok(x)) => Some(x),
                Some(Err((n, d))) => {
                    refused = Some(format!("{n}|{d}"));
                    None
                }
                None => None,
            };
            put_listing(&s, c, &owner, id, version, &m, decimals, exported.as_ref())?;
            if !fields.iter().any(|(k, _)| k == field::LINK_STATE) {
                fields.push((field::LINK_STATE.to_string(), crate::link::E_GOODS_LINK_UNPOSTED.into()));
            }
            fields.push((field::PENDING.to_string(), POST.into()));
            if let Some(co) = &company {
                fields.retain(|(k, _)| ![field::COMPANY, field::COMPANY_DIGESTS, field::COMPANY_ADDRESS, field::COMPANY_TIME, field::COMPANY_REFUSED].contains(&k.as_str()));
                company_marks(&mut fields, exported.as_ref(), co);
                if let Some(r) = refused {
                    fields.push((field::COMPANY_REFUSED.to_string(), r));
                }
            }
            true
        }
        Err((n, _)) => {
            for k in [field::LINK, field::LINK_EXPIRY] {
                if let Some(v) = rec.field(k) {
                    fields.push((k.to_string(), v.to_string()));
                }
            }
            fields.push((field::LINK_STATE.to_string(), n.clone()));
            fields.push((field::PENDING.to_string(), PRESIGN.into()));
            false
        }
    };
    s.put(&owner, Kind::Product, id, &fields, &[], c.now)?;
    Ok(ok)
}

pub fn set_listed(c: &Ctx<'_>, id: &str, on: bool) -> Result<(), Fault> {
    super::view::here_or_refused(c.data, c.me, id)?;
    let s = eravon_vault::store::open_write(c.data, c.writer, c.now)?;
    let owner = super::view::owner_in(&s, c.data, c.me, id)?;
    let r = s.get(&owner, Box_::Products, id)?;
    if on && r.field(field::REVOKED).is_some() {
        return Err(Fault::Revoked(id.to_string()));
    }
    let mut fields: Vec<(String, String)> = r.fields.iter().filter(|(k, _)| k != field::LISTED).cloned().collect();
    fields.push((field::LISTED.to_string(), listed_word(on).to_string()));
    s.put(&owner, Kind::Product, id, &fields, &[], c.now)?;
    Ok(())
}

pub(super) fn listed_word(on: bool) -> &'static str {
    if on { "true" } else { "false" }
}


pub fn delete(c: &Ctx<'_>, id: &str) -> Result<(), Fault> {
    super::view::here_or_refused(c.data, c.me, id)?;
    let s = eravon_vault::store::open_write(c.data, c.writer, c.now)?;
    let owner = super::view::owner_in(&s, c.data, c.me, id)?;
    let r = s.get(&owner, Box_::Products, id)?;
    if r.field(field::DRAFT) == Some("false") {
        return Err(Fault::NotDraft(id.to_string()));
    }
    s.delete(&owner, Kind::Product, id)?;
    Ok(())
}


#[cfg(test)]
mod post_gate_tests {
    use super::*;
    use crate::resolve::{fees_text, Fee};
    use eravon_terms::u256::U256;

    fn fee(quote: &str, amount: u64) -> Fee {
        Fee { candidate: format!("0x{}", "a1".repeat(20)), quote: quote.into(), token: format!("0x{}", "c2".repeat(20)), decimals: Some(18), amount: U256::from_u64(amount), priced: Err(("E_A1_PRECHECK".into(), String::new())) }
    }

    fn rec(signed: Option<&[Fee]>, shown: Option<&[Fee]>) -> eravon_store::record::Record {
        let mut fields = Vec::new();
        if let Some(s) = shown {
            fields.push((field::FEES_SHOWN.to_string(), fees_text(s)));
        }
        if let Some(s) = signed {
            fields.push((field::FEES_SIGNED.to_string(), fees_text(s)));
        }
        eravon_store::record::Record { id: "p1".into(), kind: Kind::Product, created: 1, updated: 1, writer: "t".into(), fields, parts: Vec::new() }
    }

    #[test]
    fn a_version_is_sent_only_by_the_key_its_terms_name() {
        let former = "0x1111111111111111111111111111111111111111";
        let company = "0x2222222222222222222222222222222222222222";
        let keys = ["0x3333333333333333333333333333333333333333".to_string(), former.to_string()];
        assert_eq!(company_proposes(&keys, "p1", &former.to_ascii_uppercase().replacen("0X", "0x", 1)), Ok(()));
        assert_eq!(company_proposes(&keys, "p1", &keys[0]), Ok(()));
        let e = company_proposes(&keys, "p1", company).expect_err("公司版");
        assert_eq!(e.name(), E_GOODS_COMPANY_PROPOSES);
    }

    #[test]
    fn a_take_down_is_recorded_where_the_item_is_kept() {
        const NOW: u64 = 1_790_000_000;
        let data = eravon_vault::testing::fresh();
        let mut v = eravon_vault::vault::create("48261537", eravon_store::vault::Seat::Seller, "主钥", Some("test test test test test test test test test test test junk"), NOW).expect("建得出库");
        v.write(&data).expect("落得下盘");
        let mut session = eravon_vault::session::Session::locked(&data);
        session.unlock("48261537").expect("开得了");
        let me = eravon_vault::accounts::who(&data, eravon_store::vault::Seat::Seller).expect("首枚钥").to_ascii_lowercase();
        let s = eravon_vault::store::open_write(&data, "goods-test", NOW).expect("可写");
        s.init(&eravon_store::settings::Global::default()).expect("铺得下");
        s.init_identity(&me, NOW).expect("登记身份");
        s.put(&me, Kind::Product, "p1", &[(field::LISTED.to_string(), "true".into()), (field::DRAFT.to_string(), "false".into())], &[], NOW).expect("落档");
        drop(s);
        revoked_recorded(&data, &me, "goods-test", NOW + 1, "p1", "0xabc").expect("记下");
        let r = eravon_vault::store::open_read(&data).expect("可读").get(&me, Box_::Products, "p1").expect("那一件");
        assert_eq!((r.field(field::LISTED), r.field(field::REVOKED)), (Some("false"), Some("0xabc")));
    }

    #[test]
    fn an_item_belongs_to_its_chain_and_a_record_from_before_reads_three_ways() {
        const NOW: u64 = 1_790_000_000;
        let data = eravon_vault::testing::fresh();
        let mut v = eravon_vault::vault::create("48261537", eravon_store::vault::Seat::Seller, "主钥", Some("test test test test test test test test test test test junk"), NOW).expect("建得出库");
        v.write(&data).expect("落得下盘");
        let mut session = eravon_vault::session::Session::locked(&data);
        session.unlock("48261537").expect("开得了");
        let me = eravon_vault::accounts::who(&data, eravon_store::vault::Seat::Seller).expect("首枚钥").to_ascii_lowercase();
        let s = eravon_vault::store::open_write(&data, "goods-test", NOW).expect("可写");
        s.init(&eravon_store::settings::Global::default()).expect("铺得下");
        s.init_identity(&me, NOW).expect("登记身份");
        let blank = Draft::default().seal();
        let draft = |id: &str, f: &[(&str, &str)]| s.put(&me, Kind::Product, id, &f.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<Vec<_>>(), &[(PART_DRAFT.to_string(), blank.clone())], NOW).expect("落档");
        draft("p-sepolia", &[(field::DRAFT, "false"), (field::VERSION, "1"), (field::CHAIN, "11155111")]);
        draft("p-listed", &[(field::DRAFT, "false"), (field::VERSION, "1")]);
        s.put(&me, Kind::Payload, &listing_id("p-listed", 1), &[], &[(PART_LISTING.to_string(), br#"{"chainId":1,"v":"eravon.listing/1"}"#.to_vec())], NOW).expect("清单");
        draft("p-old", &[(field::DRAFT, "true")]);
        drop(s);
        let r = eravon_vault::store::open_read(&data).expect("可读");
        let of = |id: &str| super::super::view::chain_of(&r, &me, &r.get(&me, Box_::Products, id).expect("那一件"));
        assert_eq!((of("p-sepolia"), of("p-listed"), of("p-old")), (Some(11_155_111), Some(1), None), "有格、无格已发布、无格草稿");
        drop(r);
        let ids = |rows: Vec<super::super::view::Row>| { let mut v: Vec<String> = rows.into_iter().map(|r| r.id).collect(); v.sort(); v };
        let on = |chain: u64| eravon_vault::store::change_home(&data, &me, |h| h.chain_id = Some(chain)).expect("选得了网");
        for (chain, want) in [(1, vec!["p-listed", "p-old"]), (11_155_111, vec!["p-old", "p-sepolia"])] {
            on(chain);
            assert_eq!(ids(list(&data, &me, "").expect("列得出")), want, "{chain}");
            assert_eq!(super::super::view::list_every_chain(&data, &me, "").expect("列得出").len(), 3, "a rotation and a payout change read every chain");
        }
        on(1);
        let k = crate::sign::Signer::locked();
        let c = Ctx { data: &data, writer: "goods-test", now: NOW + 1, me: &me, registries: &[], signer: &k, route: Err(("E_TEST".into(), "no route".into())) };
        for e in [set_listed(&c, "p-sepolia", false).expect_err("别链"), delete(&c, "p-sepolia").expect_err("别链"), super::super::view::view(&data, &me, "p-sepolia", NOW).err().expect("别链")] {
            assert_eq!(e.name(), super::super::E_GOODS_OTHER_CHAIN);
            assert!(e.detail().contains("在链 11155111 上") && e.detail().contains("家现在在链 1:"), "{}", e.detail());
        }
        let back = eravon_vault::store::open_read(&data).expect("可读").get(&me, Box_::Products, "p-sepolia").expect("那一件");
        assert_eq!(back.field(field::LISTED), None, "nothing written");
        on(11_155_111);
        let mut d = Draft::default();
        d.name = "旧草稿".into();
        save(&c, Some("p-old"), &d).expect("存得下");
        let r = eravon_vault::store::open_read(&data).expect("可读");
        assert_eq!(super::super::view::chain_of(&r, &me, &r.get(&me, Box_::Products, "p-old").expect("那一件")), Some(11_155_111));
        drop(r);
        on(1);
        assert_eq!(ids(list(&data, &me, "").expect("列得出")), vec!["p-listed"]);
    }

    #[test]
    fn a_post_of_a_former_keys_version_is_refused_as_handed_on() {
        let (me, former, company) = ("0x3333333333333333333333333333333333333333", "0x1111111111111111111111111111111111111111", "0x2222222222222222222222222222222222222222");
        let archive = [me.to_string(), former.to_string()];
        let e = handed_on(&archive, me, "p1", &former.to_ascii_uppercase().replacen("0X", "0x", 1)).expect_err("前任钥签的版本");
        assert_eq!(e.name(), E_GOODS_KEY_HANDED_ON);
        assert!(!e.detail().starts_with("E_GOODS"), "第一层话不带拒名:{}", e.detail());
        assert_eq!(handed_on(&archive, me, "p1", me), Ok(()));
        assert_eq!(handed_on(&archive, me, "p1", company), Ok(()), "公司版交给 company_proposes");
    }

    #[test]
    fn a_version_posts_only_with_the_fees_it_showed() {
        let shown = [fee("0xq1", 1200)];
        assert_eq!(posted_seen(&rec(Some(&shown), Some(&shown)), "p1", 2), Ok(()));
        for (signed, seen) in [
            (Some(vec![fee("0xq2", 1200)]), Some(shown.to_vec())),
            (Some(vec![fee("0xq1", 1300)]), Some(shown.to_vec())),
            (None, Some(shown.to_vec())),
            (Some(shown.to_vec()), None),
            (Some(Vec::new()), Some(Vec::new())),
        ] {
            let e = posted_seen(&rec(signed.as_deref(), seen.as_deref()), "p1", 2).expect_err("该拒");
            assert_eq!(e.name(), E_GOODS_FEES_UNSEEN);
            assert!(e.detail().contains("金额") || signed.as_ref().is_none_or(|v| v.is_empty()), "拒时说出签进去的费用:{}", e.detail());
        }
    }
}

#[cfg(test)]
mod cell_tests {
    #[test]
    fn no_single_item_read_or_write_names_the_current_keys_cell() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        for f in ["src/goods/save.rs", "src/goods/view.rs", "src/series.rs"] {
            let text = std::fs::read_to_string(root.join(f)).expect("读得到源");
            let code = text.split("\n#[cfg(test)]").next().unwrap_or(&text);
            for pat in ["get(c.me,", "put(c.me,", "part(c.me,", "delete(c.me,", "list(c.me,"] {
                assert!(!code.contains(pat), "{f} 直问现行钥那一格:{pat}");
            }
        }
    }
}
