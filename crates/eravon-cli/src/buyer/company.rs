use super::*;

pub(super) fn exporting(f: &Flags) -> Option<String> {
    crate::company::asked(f)
}


pub(super) fn time_gate(on: bool) -> bool {
    on
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

pub(super) fn order_export(c: &Ctx, f: &Flags, who: &str, path: &str) -> R<Out> {
    let l = linked(c, f)?;
    let p = &l.s;
    let candidate = f.int("--candidate").map_err(|_| Fault::Arg("要 --candidate".into()))?;
    must_pass(&gate_of(c, f, p), candidate)?;
    let terms = p.terms_text.as_bytes().to_vec();
    let seq = pool_seq(c, f, p, candidate)?;
    let mut draft: Option<CoDraft> = None;
    let mut company = String::new();
    let mut opens_by = 0u64;
    let answer = company_export(c, f, who, path, |w, co| {
        let (_, open_by) = shared::open_by_by_call(p.order.expiry, p.order.open_window, c.now, co.views.calendar_bound)?;
        opens_by = open_by;
        let attachments: Vec<String> = segments(c, who, p).into_iter().filter_map(|(_, _, held)| held).collect();
        let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
        let prev = crate::buyer::series_prev(&c.data, who, &co.address, p, &crate::buyer::live_on(w, c.now))?;
        let d = co_draft(&CoOrder { p, link: &l.link, listing: &l.listing, candidate, seq, buyer: &co.address, payout: &co.views.wallet, open_by, chain: w.chain_id, attachments: &attachments, w: &sw, prev: &prev })?;
        crate::write::free(&[("--out-receipt", d.receipt.as_slice()), ("--out-pack", d.pack.as_slice())].iter().filter_map(|(k, b)| f.one(k).map(|p| (p, *b))).collect::<Vec<_>>())?;
        let e = co_registered(&d, p, &co.views.wallet);
        company = co.address.clone();
        draft = Some(d);
        e
    })?;
    if let Some(d) = &draft {
        for (k, b) in [("--out-receipt", &d.receipt), ("--out-pack", &d.pack)] {
            if let Some(path) = f.one(k) {
                crate::write::land(path, b)?;
            }
        }
        co_keep(&c.write()?, who, &d.case_id[2..18], d, &company, "registered", &terms, c.now)?;
    }
    let mut answer = answer;
    if let (Some(d), Out::O(m)) = (&draft, &mut answer) {
        m.push(("id", Out::S(d.case_id[2..18].to_string())));
        m.push(("budget", Out::S(d.instance.clone())));
        m.push(("instance", Out::S(d.instance.clone())));
        m.push(("caseId", Out::S(d.case_id.clone())));
        m.push(("orderDigest", Out::S(hex(&d.order_digest))));
        m.push(("openBy", Out::N(opens_by)));
    }
    Ok(answer)
}

#[allow(clippy::too_many_arguments)]
pub fn co_keep(s: &eravon_vault::store::Store, owner: &str, id: &str, d: &CoDraft, company: &str, form: &str, terms: &[u8], now: u64) -> R<()> {
    let mut fields = match s.get(owner, Box_::Packs, id) {
        Ok(r) => r.fields,
        Err(eravon_store::fault::Fault::Missing(_)) => Vec::new(),
        Err(e) => return Err(e.into()),
    };
    let mine = [("caseId", d.case_id.as_str()), ("instance", d.instance.as_str()), ("company", company), ("form", form), ("mode", "byCall")];
    fields.retain(|(k, _)| !mine.iter().any(|(m, _)| m == k));
    fields.extend(mine.iter().map(|(k, v)| (k.to_string(), v.to_string())));
    s.put(owner, Kind::OrderFile, id, &fields, &[("pack.json".into(), d.pack.clone()), ("receipt.json".into(), d.receipt.clone()), ("terms.md".into(), terms.to_vec())], now)?;
    Ok(())
}

pub struct CoOrder<'a> {
    pub prev: &'a str,
    pub p: &'a k3::listing::Listing,
    pub link: &'a [u8],
    pub listing: &'a [u8],
    pub candidate: u64,
    pub seq: Option<u64>,
    pub buyer: &'a str,
    pub payout: &'a str,
    pub open_by: u64,
    pub chain: u64,
    pub attachments: &'a [String],
    pub w: &'a eravon_watch::Wire<'a>,
}

pub struct CoDraft {
    pub order: a1::order::Order,
    pub instance: String,
    pub case_id: String,
    pub order_digest: [u8; 32],
    pub receipt: Vec<u8>,
    pub pack: Vec<u8>,
    pub items: Vec<a1::company::Item>,
    pub docs: Vec<a1::proposal::Original>,
}

pub const E_COMPANY_TAKEN: &str = "E_P3_COMPANY_TAKEN";

pub fn co_draft(i: &CoOrder<'_>) -> R<CoDraft> {
    let p = i.p;
    if p.chain_id != i.chain {
        return Err(Fault::Precheck { gate: crate::fault::GATE_COMPANY.into(), why: format!("清单的 chainId 是 {},连线的链是 {}:提案与订单摘要不在同一条链上", p.chain_id, i.chain) });
    }
    crate::buyer::new_order_under(&p.logic)?;
    super::stands(i.w, i.listing, p, i.buyer, i.candidate)?;
    let o = shared::order_of(p.terms_text.as_bytes(), i.candidate, i.buyer, i.payout, i.open_by, i.prev)?;
    super::standing::must_pay(i.w, &o)?;
    let d = o.derive(p.chain_id, &p.logic)?;
    if occupied(i.w.t, i.w.table, &d.instance, &p.logic)? {
        return Err(Fault::Down { layer: "P3", token: E_COMPANY_TAKEN.into(), why: format!("地址 {} 已有订单", d.instance) });
    }
    let case_id = hex(&d.salt);
    let receipt = write::receipt(&p.terms_hash, i.candidate, i.buyer, i.payout, i.open_by, i.prev, "", i.seq)?;
    let pack = write::pack(p.chain_id, &d.instance, &case_id, i.link, i.listing, &receipt, i.attachments)?;
    let cand = eravon_cands::act::Candidate::of(&o);
    let q = eravon_watch::act::quote(i.w, &cand, &a1::order::hex_of(&o.offer.quote_hash)).map_err(|(token, why)| Fault::Down { layer: "S4", token, why })?;
    let zero = |a: &str| a.trim_start_matches("0x").bytes().all(|b| b == b'0');
    let who = if zero(&o.offer.judge) { o.offer.guild.clone() } else { o.offer.judge.clone() }.to_ascii_lowercase();
    let items = vec![
        a1::company::Item::Window { name: "t2", who: String::new(), secs: p.order.confirm_window },
        a1::company::Item::Deadline { name: "offerExpiry", who: String::new(), at: p.order.expiry },
        a1::company::Item::Window { name: "t3", who, secs: q.ruling_window },
    ];
    let docs = vec![
        a1::proposal::Original { kind: seam::V_LISTING.into(), name: "listing.json".into(), bytes: i.listing.to_vec() },
        a1::proposal::Original { kind: seam::V_TERMS.into(), name: "terms.md".into(), bytes: p.terms_text.as_bytes().to_vec() },
    ];
    Ok(CoDraft { instance: d.instance.clone(), case_id, order_digest: d.order_digest, receipt, pack, items, docs, order: o })
}

pub fn co_registered(d: &CoDraft, p: &k3::listing::Listing, wallet: &str) -> R<crate::company::Export> {
    let (pay, wallet_abi) = a1::proposal::wallet_pay(wallet, &p.token, &d.instance, &p.order.amount.to_be())?;
    let original = a1::proposal::Original { kind: seam::V_RECEIPT.into(), name: "receipt.json".into(), bytes: d.receipt.clone() };
    let (consent, docs) = a1::proposal::consent(&p.logic, &d.order.buyer_key, &d.order_digest, original)?;
    let mut originals = vec![wallet_abi];
    originals.extend(docs);
    originals.extend(d.docs.iter().cloned());
    Ok(crate::company::Export {
        verb: "order-sign",
        calls: vec![pay, consent],
        originals,
        items: d.items.clone(),
        face: vec![
            a1::proposal::face_consent(p.order.expiry),
            format!("预算地址 {}:登记与到账之后由卖家开案;条款许买家开案时,由公司执行开案提案。", d.instance),
        ],
        deal: crate::company::Deal::Unopened(d.order.offer.seller_key.clone()),
    })
}

pub(super) fn dispute_wallet_export(c: &Ctx, f: &Flags, path: &str) -> R<Out> {
    let instance = f.opt("--instance").ok_or_else(|| Fault::Arg("要 --instance".into()))?;
    let who = c.who(f.one("--address"))?;
    company_export(c, f, &who, path, |w, co| {
        let (case_id, token, window_end) = co_case(w, &instance)?;
        let at = crate::after_company::At { instance: &instance, case_id: &case_id, token: &token, window_end };
        Ok(export_of(crate::after_company::dispute_plan(w, &at, crate::after_company::Form::Wallet, &co.views.wallet, None).map_err(p6)?))
    })
}

pub(super) fn co_case(w: &shared::Wire, instance: &str) -> R<(String, String, u64)> {
    let dw = eravon_dispute::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let (case_id, _, _, block) = eravon_dispute::settle::opened(&dw, instance).map_err(p6)?;
    let v = eravon_dispute::view::case(&dw, instance, &case_id, block).map_err(p6)?;
    Ok((case_id, v.token, v.window_end))
}

pub(super) fn p6((token, why): (String, String)) -> Fault {
    Fault::Down { layer: "P6", token, why }
}

pub(super) fn export_of(p: a1::proposal::Plan) -> crate::company::Export {
    crate::company::Export::of(p)
}

pub(super) fn pull_route(f: &Flags) -> R<bool> {
    let pull = f.one("--pull") == Some("true");
    if pull && exporting(f).is_some() {
        return Err(Fault::Precheck {
            gate: crate::fault::GATE_COMPANY.into(),
            why: "--pull 与 --export-proposal 不并用:公司那一形的诉讼费由提案里的钱包 pay(--advance wallet)或个人垫付(--advance personal)补,不从当前钥拉取".into(),
        });
    }
    Ok(pull)
}


pub fn expire_plan(w: &shared::Wire, o: &a1::order::Order, logic: &str, now: u64) -> R<(a1::proposal::Plan, String)> {
    let (d, deploy, init) = crate::seller::expire_calls(w, o, logic, now)?;
    if deploy.is_none() {
        a1::chain::code_matches(&w.t, &w.table, &d.instance, logic)?;
    }
    let mut calls = Vec::with_capacity(2);
    for c in deploy.iter().chain(std::iter::once(&init)) {
        calls.push(a1::proposal::of_call(c)?);
    }
    let p = a1::proposal::Plan {
        verb: "expire",
        calls,
        originals: vec![a1::proposal::abi_doc(&[a1::sol::INIT], a1::proposal::INSTANCE_ABI_NAME)?],
        items: Vec::new(),
        face: Vec::new(),
        instance: d.instance.clone(),
    };
    Ok((p, o.offer.seller_key.to_ascii_lowercase()))
}

pub fn seat_party(data: &str, me: &str) -> R<String> {
    let parties = eravon_vault::store::parties_of(data, me).map_err(|e| Fault::Down { layer: "A5", token: e.name().to_string(), why: e.detail() })?;
    Ok(parties.last().cloned().unwrap_or_else(|| me.to_ascii_lowercase()))
}

pub fn open_plan(w: &shared::Wire, o: &a1::order::Order, sig_seller: &[u8], guild: bool, now: u64, company: &str) -> R<(a1::proposal::Plan, String)> {
    if guild {
        return Err(Fault::Down { layer: "buyer", token: E_COMPANY_GUILD.into(), why: "行会轨上公司不经提案开案:这一单由卖家开".into() });
    }
    let p = crate::seller::open_calls(&crate::seller::OpenIn { w, o, sig_seller, sig_buyer: &[], from: company, guild: None, now })?;
    let mut calls = Vec::with_capacity(2);
    for c in p.deploy.iter().chain(std::iter::once(&p.init)) {
        calls.push(a1::proposal::of_call(c)?);
    }
    let plan = a1::proposal::Plan {
        verb: "init",
        calls,
        originals: vec![a1::proposal::abi_doc(&[a1::sol::INIT], a1::proposal::INSTANCE_ABI_NAME)?],
        items: Vec::new(),
        face: Vec::new(),
        instance: p.instance.clone(),
    };
    Ok((plan, o.offer.seller_key.to_ascii_lowercase()))
}

pub const E_COMPANY_GUILD: &str = "E_V_COMPANY_GUILD";

pub const E_COMPANY_PROPOSES: &str = "E_V_COMPANY_PROPOSES";

pub fn expire_export(w: &shared::Wire, o: &a1::order::Order, logic: &str, now: u64) -> R<crate::company::Export> {
    let (p, seller) = expire_plan(w, o, logic, now)?;
    let mut e = export_of(p);
    e.deal = crate::company::Deal::Unopened(seller);
    Ok(e)
}
