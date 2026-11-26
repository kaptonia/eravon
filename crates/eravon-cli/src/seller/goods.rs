use super::*;

pub(super) fn product_save(c: &Ctx, f: &Flags) -> R<Out> {
    store(c, f, false)
}

pub(super) fn product_publish(c: &Ctx, f: &Flags) -> R<Out> {
    store(c, f, true)
}

pub(super) struct KeySend<'a> {
    pub c: &'a Ctx,
    pub f: &'a Flags,
    pub w: &'a shared::Wire,
    pub who: &'a str,
    pub rows: Vec<Out>,
    pub held: Option<a1::call::Call>,
}

impl eravon_goods::goods::Sender for KeySend<'_> {
    fn send(&mut self, call: &a1::call::Call) -> Result<String, (String, String)> {
        if !crate::send::asked(self.f) {
            self.held = Some(call.clone());
            return Err((NOT_SENT.into(), "没给 --send true,故只把这一笔的字节交出来".into()));
        }
        let d = crate::send::deliver(self.c, self.f, self.w, self.who, call, "0").map_err(|e| (e.name().to_string(), e.detail()))?;
        self.rows.push(Out::O(d.rows()));
        d.outcome()
    }
}

fn held_rows(held: &Option<a1::call::Call>) -> Vec<(&'static str, Out)> {
    match held {
        Some(c) => {
            let mut v = shared::tx_rows(c, "0");
            v.push(("sent", Out::B(false)));
            v.push(("why", Out::S("没给 --send true,故只把这一笔的字节交出来".into())));
            v
        }
        None => Vec::new(),
    }
}

pub(super) fn product_post(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let id = f.opt("--id").ok_or_else(|| Fault::Arg("要 --id".into()))?;
    let w = shared::wire(f, c, Some(&who))?;
    let signer = eravon_goods::sign::Signer::of(None, &who);
    let registries = f.many("--registry");
    let g = goods_ctx(c, f, &who, &signer, &registries);
    let mut s = KeySend { c, f, w: &w, who: &who, rows: Vec::new(), held: None };
    let mut rows = vec![("id", Out::S(id.clone()))];
    match eravon_goods::goods::post(&g, &id, &mut s) {
        Ok(p) => rows.extend([("expiry", Out::N(p.expiry)), ("link", Out::S(p.link)), ("postedHash", Out::S(p.hash)), ("postedTx", Out::S(p.tx)), ("sent", Out::B(true))]),
        Err(eravon_goods::goods::Fault::Unsent { name, .. }) if name == NOT_SENT => rows.extend(held_rows(&s.held)),
        Err(e) => return Err(goods_down(e)),
    }
    rows.push(("txs", Out::A(s.rows)));
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub(super) fn product_revoke(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let id = f.opt("--id").ok_or_else(|| Fault::Arg("要 --id".into()))?;
    let w = shared::wire(f, c, Some(&who))?;
    let signer = eravon_goods::sign::Signer::of(None, &who);
    let registries = f.many("--registry");
    let g = goods_ctx(c, f, &who, &signer, &registries);
    let mut s = KeySend { c, f, w: &w, who: &who, rows: Vec::new(), held: None };
    let mut rows = vec![("id", Out::S(id.clone()))];
    match eravon_goods::goods::revoke(&g, &id, &mut s) {
        Ok(tx) => rows.extend([("revoked", Out::S(tx)), ("sent", Out::B(true))]),
        Err(eravon_goods::goods::Fault::Unsent { name, .. }) if name == NOT_SENT => rows.extend(held_rows(&s.held)),
        Err(e) => return Err(goods_down(e)),
    }
    rows.push(("txs", Out::A(s.rows)));
    rows.extend(w.rows());
    Ok(ok(rows))
}

pub(super) fn goods_ctx<'a>(c: &'a Ctx, f: &Flags, who: &'a str, signer: &'a eravon_goods::sign::Signer, registries: &'a [String]) -> eravon_goods::goods::Ctx<'a> {
    eravon_goods::goods::Ctx { data: &c.data, writer: &c.writer, now: c.now, me: who, registries, signer, route: shared::route(f, c, Some(who)) }
}

fn store(c: &Ctx, f: &Flags, publish: bool) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let id = f.opt("--id");
    let mut d = match (f.opt("--draft-file"), &id) {
        (Some(p), _) => {
            let b = home::read_bounded(&p)?;
            eravon_goods::draft::Draft::open(&b).ok_or_else(|| Fault::Arg(format!("{p} 不是一份读得回的商品稿")))?
        }
        (None, Some(i)) => match eravon_goods::goods::load(&c.data, &who, i) {
            Ok(d) => d,
            Err(eravon_goods::goods::Fault::Missing(_)) => eravon_goods::draft::Draft::default(),
            Err(e) => return Err(goods_down(e)),
        },
        (None, None) => eravon_goods::draft::Draft::default(),
    };
    let session = c.session(f.one("--passcode"))?;
    if let Some(v) = f.one("--series-id") {
        let known = eravon_goods::series::known(&eravon_goods::series::stages(&c.data, &who), id.as_deref());
        eravon_goods::series::pick(&mut d, v, &known).map_err(|n| Fault::Down { layer: "S1", token: n.name().to_string(), why: n.detail() })?;
    }
    for (k, cell) in [("--title", "name"), ("--price", "price"), ("--note", "description"), ("--t1", "t1"), ("--t1h", "t1h"), ("--t1m", "t1m"), ("--t2", "t2"), ("--t2h", "t2h"), ("--t2m", "t2m"), ("--stage", "stage"), ("--stages", "stages")] {
        if let Some(v) = f.one(k) {
            d.set(cell, v);
        }
    }
    let signer = eravon_goods::sign::Signer::of(Some(&session), &who);
    let registries = f.many("--registry");
    let g = goods_ctx(c, f, &who, &signer, &registries);
    let mut sv = if publish { eravon_goods::goods::publish_recorded(&g, id.as_deref(), &d) } else { eravon_goods::goods::save(&g, id.as_deref(), &d) }.map_err(goods_down)?;
    if let Some(m) = sv.verdict.missing.iter().find(|m| m.name.starts_with(eravon_goods::series::E_SERIES)) {
        return Err(Fault::Down { layer: "S1", token: m.name.clone(), why: format!("{}({})", m.detail, m.cell) });
    }
    let mut post_rows: Vec<(&'static str, Out)> = Vec::new();
    if publish && sv.pending.iter().any(|p| p == eravon_goods::goods::POST) {
        let w = shared::wire(f, c, Some(&who))?;
        let mut s = KeySend { c, f, w: &w, who: &who, rows: Vec::new(), held: None };
        match eravon_goods::goods::post(&g, &sv.id, &mut s) {
            Ok(p) => {
                sv.link = Some(p.link);
                sv.link_state = None;
                sv.pending.retain(|x| x != eravon_goods::goods::POST);
                post_rows.extend([("postedHash", Out::S(p.hash)), ("postedTx", Out::S(p.tx))]);
            }
            Err(eravon_goods::goods::Fault::Unsent { name, .. }) if name == NOT_SENT => post_rows.extend(held_rows(&s.held)),
            Err(eravon_goods::goods::Fault::Proposes(_)) => {}
            Err(e) => return Err(goods_down(e)),
        }
        post_rows.push(("txs", Out::A(s.rows)));
    }
    let r = eravon_goods::goods::record(&c.data, &who, &sv.id).map_err(goods_down)?;
    let (state, state_detail) = sv.link_state.clone().unwrap_or_default();
    let (refused, refused_detail) = sv.company_refused.clone().unwrap_or_default();
    let mut rows = vec![
        ("companyRefused", Out::S(refused)),
        ("companyRefusedDetail", Out::S(refused_detail)),
        ("createdAt", Out::N(r.created)),
        ("draft", Out::B(sv.draft)),
        ("excluded", Out::A(sv.resolved.excluded.iter().map(|x| Out::O(vec![("address", Out::S(x.address.clone())), ("detail", Out::S(x.detail.clone())), ("name", Out::S(x.name.clone()))])).collect())),
        ("fees", Out::A(sv.resolved.fees.iter().map(fee_out).collect())),
        ("id", Out::S(sv.id.clone())),
        ("link", Out::S(sv.link.clone().unwrap_or_default())),
        ("linkState", Out::S(state)),
        ("linkStateDetail", Out::S(state_detail)),
        ("missing", Out::A(sv.verdict.missing.iter().map(|m| Out::S(m.cell.clone())).collect())),
        ("pending", Out::A(sv.pending.iter().map(|p| Out::S(p.clone())).collect())),
        ("series", Out::O(match eravon_goods::series::cells(&d) {
            Ok(Some(s)) => vec![("id", Out::S(s.id)), ("stage", Out::N(s.stage)), ("stages", Out::N(s.stages))],
            _ => Vec::new(),
        })),
        ("termsHash", Out::S(sv.terms_hash.clone().unwrap_or_default())),
        ("updatedAt", Out::N(r.updated)),
        ("version", Out::N(sv.version)),
    ];
    rows.extend(post_rows);
    Ok(ok(rows))
}

pub(super) fn fee_out(f: &eravon_goods::resolve::Fee) -> Out {
    let mut r = vec![("amount", Out::S(f.amount.to_dec())), ("candidate", Out::S(f.candidate.clone())), ("token", Out::S(f.token.clone()))];
    match &f.priced {
        Ok(q) => {
            r.push(("deposit", Out::S(q.deposit.to_dec())));
            r.push(("lf", Out::S(q.lf.to_dec())));
            r.push(("premium", Out::S(q.premium.to_dec())));
            r.push(("pricedAtOpen", Out::B(q.rate.is_some())));
            if let Some(x) = &q.rate {
                r.push(("rateAt", Out::N(x.rate_at)));
            }
        }
        Err((n, d)) => r.push(("why", Out::O(vec![("detail", Out::S(d.clone())), ("name", Out::S(n.clone()))]))),
    }
    r.sort_by(|a, b| a.0.cmp(b.0));
    Out::O(r)
}

pub(super) fn goods_down(e: eravon_goods::goods::Fault) -> Fault {
    let (token, why) = e.said();
    Fault::Down { layer: "S1", token, why }
}

pub(super) fn product_qr(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let id = f.opt("--id").ok_or_else(|| Fault::Arg("要 --id".into()))?;
    let out = f.opt("--out").ok_or_else(|| Fault::Arg("要 --out".into()))?;
    let v = eravon_goods::goods::view(&c.data, &who, &id, c.now).map_err(goods_down)?;
    let Some(link) = v.current_link() else {
        let (name, why) = if v.row.draft {
            ("E_GOODS_DRAFT", "草稿没有链接".to_string())
        } else if !v.row.listed {
            ("E_GOODS_UNLISTED", "已下架,本端不出链接".to_string())
        } else {
            ("E_GOODS_NO_LINK", format!("这一版没有现行链接:{} · 待办 {}", v.link_state, v.pending.join(",")))
        };
        return Err(Fault::Down { layer: "S1", token: name.to_string(), why });
    };
    let side = eravon_goods::link::qr_png(&link.link, &out)
        .map_err(|(n, d)| Fault::Down { layer: "S1", token: n, why: d })?;
    Ok(ok(vec![
        ("bytes", Out::N(link.link.len() as u64)),
        ("budget", Out::N(eravon_literals::constants::QR_BYTE_BUDGET as u64)),
        ("id", Out::S(id)),
        ("out", Out::S(out)),
        ("px", Out::N(side as u64)),
        ("version", Out::N(link.version)),
    ]))
}

pub(super) fn product_list(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let rows = eravon_goods::goods::records_here(&c.data, &who).map_err(record_down)?;
    let want = f.one("--draft");
    let list: Vec<Out> = rows
        .iter()
        .map(|(_, r)| r)
        .filter(|r| r.kind == Kind::Product)
        .filter(|r| want.is_none_or(|w| r.field("draft").unwrap_or("false") == w))
        .map(|r| r.row())
        .collect();
    Ok(ok(vec![("count", Out::N(list.len() as u64)), ("products", Out::A(list))]))
}

pub(super) fn product_show(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let id = f.opt("--id").ok_or_else(|| Fault::Arg("要 --id".into()))?;
    let r = eravon_goods::goods::record(&c.data, &who, &id).map_err(record_down)?;
    Ok(ok(vec![("product", r.row())]))
}

fn record_down(e: eravon_goods::goods::Fault) -> Fault {
    match e {
        eravon_goods::goods::Fault::Missing(m) => Fault::from(eravon_store::fault::Fault::Missing(m)),
        eravon_goods::goods::Fault::Store { name, detail } => crate::fault::store_down(name, detail),
        other => goods_down(other),
    }
}

pub(super) fn product_live(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let id = f.opt("--id").ok_or_else(|| Fault::Arg("要 --id".into()))?;
    let b = eravon_goods::goods::live_bytes(&c.data, &who, &id).map_err(goods_down)?;
    let d = eravon_goods::draft::Draft::open(&b).ok_or_else(|| goods_down(eravon_goods::goods::Fault::Form(id.clone())))?;
    let text = String::from_utf8(b).map_err(|_| goods_down(eravon_goods::goods::Fault::Form(id.clone())))?;
    let ladder = d.ladder.iter().map(|t| Out::O(vec![("name", Out::S(t.name.clone())), ("pct", Out::S(t.pct.clone()))])).collect();
    Ok(ok(vec![
        ("cands", strs(&d.cands)),
        ("deliveryNote", Out::S(d.delivery_note)),
        ("deliveryPrivate", Out::B(d.delivery_private)),
        ("description", Out::S(d.description)),
        ("draft", Out::S(text)),
        ("goodsPrivate", Out::B(d.goods_private)),
        ("id", Out::S(id)),
        ("ladder", Out::A(ladder)),
        ("name", Out::S(d.name)),
        ("price", Out::S(d.price)),
        ("t1", Out::S(d.t1)),
        ("t1h", Out::S(d.t1h)),
        ("t1m", Out::S(d.t1m)),
        ("t2", Out::S(d.t2)),
        ("t2h", Out::S(d.t2h)),
        ("t2m", Out::S(d.t2m)),
        ("token", Out::S(d.token)),
    ]))
}

pub(super) fn product_link(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let terms = home::read_bounded(&f.opt("--terms").ok_or_else(|| Fault::Arg("要 --terms".into()))?)?;
    let t = k2::doc::terms(&terms).map_err(Fault::from)?;
    let sigs = f.many("--judge-sig");
    let guild_sigs = f.many("--guild-sig");
    if sigs.len() != t.judges.len() || guild_sigs.len() != t.guilds.len() {
        return Err(Fault::Arg(format!(
            "要 {} 枚 --judge-sig 与 {} 枚 --guild-sig,给了 {} 与 {}",
            t.judges.len(),
            t.guilds.len(),
            sigs.len(),
            guild_sigs.len()
        )));
    }
    let panels = f.many("--guild-panel");
    let guild_seq = f.opt_int("--guild-seq")?.unwrap_or(0);
    let now: Vec<eravon_goods::resolve::GuildEl> = t
        .guilds
        .iter()
        .enumerate()
        .map(|(i, g)| eravon_goods::resolve::GuildEl {
            pointer: g.address.clone(),
            amanah_hash: g.amanah_hash.clone(),
            quote: String::new(),
            seq: guild_seq,
            panel_hash: panels.get(i).cloned().unwrap_or_default(),
            text: String::new(),
        })
        .collect();
    let all_sigs: Vec<String> = sigs.into_iter().chain(guild_sigs).collect();
    let bytes = eravon_goods::link::listing(&terms, &all_sigs, &now).map_err(|(n, d)| Fault::Down { layer: "S1", token: n, why: d })?;
    let id = record_id(Kind::Payload, f.opt("--id"), &hex(&t.terms_hash)[2..18]);
    crate::write::free(&f.one("--out").map(|p| (p, bytes.as_slice())).into_iter().collect::<Vec<_>>())?;
    let s = c.write()?;
    s.put(&who, Kind::Payload, &id, &[("termsHash".into(), hex(&t.terms_hash))], &[(eravon_goods::goods::PART_LISTING.into(), bytes.clone())], c.now)?;
    if let Some(p) = f.one("--out") {
        crate::write::land(p, &bytes)?;
    }
    Ok(ok(vec![("bytes", Out::N(bytes.len() as u64)), ("id", Out::S(id)), ("termsHash", Out::S(hex(&t.terms_hash)))]))
}

pub(super) fn product_resign(c: &Ctx, f: &Flags) -> R<Out> {
    let (doc, a) = terms_built(c, f)?;
    let b = offer_signed(c, f, &doc, &f.opt("--out").unwrap_or_default())?;
    Ok(ok(vec![("offers", b), ("terms", a)]))
}

pub(crate) fn record_id(kind: Kind, given: Option<String>, fallback: &str) -> String {
    let base = given.unwrap_or_else(|| fallback.to_string());
    let tag = match kind {
        Kind::TermsDocument => "terms-",
        Kind::OfferSignature => "sig-",
        Kind::Payload => "listing-",
        _ => "",
    };
    format!("{tag}{base}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_locked_record_read_answers_as_a_locked_store_read() {
        let straight = crate::fault::Fault::from(eravon_store::fault::Fault::NoKey("锁着".into()));
        let through = super::record_down(eravon_goods::goods::Fault::from(eravon_store::fault::Fault::NoKey("锁着".into())));
        let token = |f: &crate::fault::Fault| match f {
            crate::fault::Fault::Down { token, .. } => token.clone(),
            other => other.name().to_string(),
        };
        assert_eq!(token(&through), token(&straight));
        assert_eq!(token(&through), "E_VAULT_LOCKED");
        let other = super::record_down(eravon_goods::goods::Fault::from(eravon_store::fault::Fault::Io("盘读不成".into())));
        assert_eq!(token(&other), "E_STORE_IO");
    }
}
