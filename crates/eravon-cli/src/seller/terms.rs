use super::*;

pub(super) fn terms_build(c: &Ctx, f: &Flags) -> R<Out> {
    terms_built(c, f).map(|(_, out)| out)
}

pub(super) fn terms_built(c: &Ctx, f: &Flags) -> R<(Vec<u8>, Out)> {
    let block = home::read_bounded(&f.opt("--block").ok_or_else(|| Fault::Arg("要 --block <机读块档>".into()))?)?;
    let doc = write::terms_document(block.trim_ascii())?;
    let t = k2::doc::terms(&doc).map_err(Fault::from)?;
    let who = c.who(f.one("--address"))?;
    let id = record_id(Kind::TermsDocument, f.opt("--id"), &hex(&t.terms_hash)[2..18]);
    crate::write::free(&f.one("--out").map(|p| (p, doc.as_slice())).into_iter().collect::<Vec<_>>())?;
    let shown = Shown::of(c, f, &who, &doc);
    let s = c.write()?;
    s.put(
        &who,
        Kind::TermsDocument,
        &id,
        &shown.field(vec![("termsHash".into(), hex(&t.terms_hash))]),
        &[("terms.md".into(), doc.clone())],
        c.now,
    )?;
    if let Some(p) = f.one("--out") {
        crate::write::land(p, &doc)?;
    }
    let mut rows = vec![
        ("bytes", Out::N(doc.len() as u64)),
        ("amount", Out::S(t.amount.to_dec())),
        ("id", Out::S(id)),
        ("candidates", Out::N((t.judges.len() + t.guilds.len()) as u64)),
        ("termsHash", Out::S(hex(&t.terms_hash))),
    ];
    shown.rows(&mut rows);
    Ok((doc, ok(rows)))
}

struct Shown {
    fees: Result<Vec<eravon_goods::resolve::Fee>, (String, String)>,
}

impl Shown {
    fn of(c: &Ctx, f: &Flags, who: &str, terms: &[u8]) -> Shown {
        let signer = eravon_goods::sign::Signer::of(None, who);
        let registries = f.many("--registry");
        let wire = shared::wire(f, c, Some(who)).map_err(|e| (e.name().to_string(), e.detail())).and_then(|w| {
            let registries = eravon_cands::wire::registries(&w.registries).map_err(|a| (a.name().to_string(), a.detail()))?;
            Ok(eravon_cands::wire::Wire { t: w.t, table: w.table, registries, now: c.now })
        });
        Shown { fees: wire.and_then(|w| eravon_goods::goods::terms_fees(&super::goods::goods_ctx(c, f, who, &signer, &registries), &w, terms)) }
    }

    fn field(&self, mut fields: Vec<(String, String)>) -> Vec<(String, String)> {
        if let Ok(v) = &self.fees {
            fields.push((eravon_goods::goods::FEES_SHOWN.to_string(), eravon_goods::resolve::fees_text(v)));
        }
        fields
    }

    fn rows(&self, rows: &mut Vec<(&'static str, Out)>) {
        match &self.fees {
            Ok(v) => rows.push(("fees", Out::A(v.iter().map(super::goods::fee_out).collect()))),
            Err((n, d)) => {
                rows.push(("fees", Out::A(Vec::new())));
                rows.push(("feesWhy", Out::O(vec![("detail", Out::S(format!("{d}:费用没读成,上板之前还会再核"))), ("name", Out::S(n.clone()))])));
            }
        }
    }
}

pub(super) fn terms_hash(_c: &Ctx, f: &Flags) -> R<Out> {
    let b = home::read_bounded(&f.opt("--file").ok_or_else(|| Fault::Arg("要 --file".into()))?)?;
    Ok(ok(vec![("termsHash", Out::S(hex(&k2::doc::terms_hash(&b))))]))
}

pub(super) fn terms_check(_c: &Ctx, f: &Flags) -> R<Out> {
    let b = home::read_bounded(&f.opt("--file").ok_or_else(|| Fault::Arg("要 --file".into()))?)?;
    let t = k2::doc::terms(&b).map_err(Fault::from)?;
    Ok(ok(vec![
        ("chainId", Out::N(t.chain_id)),
        ("expiry", Out::N(t.expiry)),
        ("guilds", Out::N(t.guilds.len() as u64)),
        ("judges", Out::N(t.judges.len() as u64)),
        ("ladder", Out::N(t.ladder.len() as u64)),
        ("amount", Out::S(t.amount.to_dec())),
        ("nonce", Out::N(t.nonce)),
        ("sellerKey", Out::S(t.seller_key.clone())),
        ("termsHash", Out::S(hex(&t.terms_hash))),
    ]))
}

pub(super) fn offer_sign(c: &Ctx, f: &Flags) -> R<Out> {
    offer_sign_as(c, f, None)
}

pub(super) fn offer_sign_as(c: &Ctx, f: &Flags, id_override: Option<&str>) -> R<Out> {
    let terms_path = f.opt("--terms").ok_or_else(|| Fault::Arg("要 --terms".into()))?;
    let b = home::read_bounded(&terms_path)?;
    let _ = id_override;
    offer_signed(c, f, &b, &terms_path)
}

pub(super) fn offer_signed(c: &Ctx, f: &Flags, b: &[u8], terms_path: &str) -> R<Out> {
    let t = k2::doc::terms(b).map_err(Fault::from)?;
    let who = c.who(f.one("--address"))?;
    if let Some(path) = exporting(f) {
        return company_export(c, f, &who, &path, |w, co| offer_export(co, &t, b, terms_path, w));
    }
    let key = c.key(&who, f.one("--passcode"))?;
    let total = (t.judges.len() + t.guilds.len()) as u64;
    let mut rows = Vec::new();
    let mut store_rows: Vec<(String, String)> = Vec::new();
    for cand in 0..total {
        let d = k2::derive::offer_digest_at(&t, cand).map_err(Fault::from)?;
        let sig = shared::sign(&key, &d)?;
        store_rows.push((format!("c{cand}"), sig.clone()));
        rows.push(Out::O(vec![("candidate", Out::N(cand)), ("digest", Out::S(hex(&d))), ("sig", Out::S(sig))]));
    }
    let id = record_id(Kind::OfferSignature, f.opt("--id"), &hex(&t.terms_hash)[2..18]);
    let shown = Shown::of(c, f, &who, b);
    let s = c.write()?;
    s.put(&who, Kind::OfferSignature, &id, &shown.field(store_rows), &[], c.now)?;
    let mut out = vec![("count", Out::N(rows.len() as u64)), ("id", Out::S(id)), ("signatures", Out::A(rows))];
    shown.rows(&mut out);
    Ok(ok(out))
}

pub(super) fn link_encode(_c: &Ctx, f: &Flags) -> R<Out> {
    let b = home::read_bounded(&f.opt("--file").ok_or_else(|| Fault::Arg("要 --file".into()))?)?;
    k3::link::link(&b).map_err(Fault::from)?;
    let link = k3::transport::encode_link(&b);
    Ok(ok(vec![("bytes", Out::N(b.len() as u64)), ("link", Out::S(link))]))
}

pub(super) fn link_decode(_c: &Ctx, f: &Flags) -> R<Out> {
    let text = f.opt("--link").ok_or_else(|| Fault::Arg("要 --link".into()))?;
    let (_, l) = k3::transport::decode_link(text.as_bytes()).map_err(Fault::from)?;
    Ok(ok(vec![("chainId", Out::N(l.chain_id)), ("hash", Out::S(l.hash)), ("tx", Out::S(l.tx))]))
}

pub fn decode_receipt_field(text: &str) -> R<Vec<u8>> {
    k3::transport::decode_receipt(text.trim().as_bytes()).map(|(b, _)| b).map_err(Fault::from)
}

pub(super) fn receipt_paste(c: &Ctx, f: &Flags) -> R<Out> {
    let docs = match (f.opt("--listing"), f.opt("--terms")) {
        (Some(l), Some(t)) => Some((home::read_bounded(&l)?, home::read_bounded(&t)?)),
        (None, None) => None,
        (Some(_), None) => return Err(Fault::Arg("要 --terms(条款档),与 --listing 一起给;两份都不给时取本机商品归档".into())),
        (None, Some(_)) => return Err(Fault::Arg("要 --listing(清单档),与 --terms 一起给;两份都不给时取本机商品归档".into())),
    };
    let rtext = f.opt("--receipt").ok_or_else(|| Fault::Arg("要 --receipt".into()))?;
    let rb = match home::read_bounded(&rtext) {
        Ok(b) => b,
        Err(_) => decode_receipt_field(&rtext)?,
    };
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who)).ok();
    let sw = w.as_ref().map(|w| eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries });
    let o = match &docs {
        Some((pb, terms)) => eravon_watch::paste::paste(&c.data, &c.writer, c.now, &who, pb, &rb, terms, f.one("--id"), sw.as_ref()),
        None => eravon_watch::paste::paste_receipt(&c.data, &c.writer, c.now, &who, &rb, f.one("--id"), sw.as_ref()),
    }
    .map_err(s4)?;
    Ok(ok(vec![
        ("budget", Out::S(o.instance.clone())),
        ("buyerKey", Out::S(o.buyer_key.clone())),
        ("candidate", Out::N(o.candidate)),
        ("caseId", Out::S(o.case_id.clone())),
        ("id", Out::S(o.id.clone())),
        ("instance", Out::S(o.instance.clone())),
        ("openBy", Out::N(o.open_by)),
        ("verdict", Out::S(o.verdict.clone())),
    ]))
}
