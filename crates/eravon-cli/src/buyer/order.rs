use super::*;

pub(super) struct Linked {
    pub link: Vec<u8>,
    pub listing: Vec<u8>,
    pub s: k3::listing::Listing,
    pub kind: &'static str,
}

pub(super) fn linked(c: &Ctx, f: &Flags) -> R<Linked> {
    let text = f.opt("--link").ok_or_else(|| Fault::Arg("要 --link".into()))?;
    let (link, kind) = match home::read_bounded(&text) {
        Ok(b) => (b, "file"),
        Err(_) => (k3::transport::decode_link(text.trim().as_bytes()).map_err(Fault::from)?.0, "link"),
    };
    let who = c.who(f.one("--address")).ok();
    let w = shared::wire(f, c, who.as_deref())?;
    let listing = posted(&w, &link)?;
    match k3::link::resolve(&link, &listing).map_err(Fault::from)? {
        k3::link::Resolved::Listing(s) => Ok(Linked { link, listing, s: *s, kind }),
        k3::link::Resolved::Settle(_) => Err(Fault::Arg("这条链接指向和解要约,不是清单".into())),
    }
}

pub const E_LINK_UNPOSTED: &str = "E_LINK_UNPOSTED";

pub fn posted(w: &shared::Wire, link: &[u8]) -> R<Vec<u8>> {
    let l = k3::link::link(link).map_err(Fault::from)?;
    let refuse = |why: String| Fault::Down { layer: "V2", token: E_LINK_UNPOSTED.into(), why };
    if let Some(why) = k3::link::other_chain(&l, w.chain_id) {
        return Err(Fault::Down { layer: "V2", token: k3::link::E_LINK_OTHER_CHAIN.into(), why });
    }
    let hash = a1::order::b32_of("链接的 hash", &l.hash)?;
    let posts = eravon_scan::board::by_tx(&w.t, &w.table, &l.tx, &hash).map_err(Fault::from)?;
    for p in &posts {
        if let Ok(s) = k3::listing::listing(&p.document)
            && hex(&p.key).eq_ignore_ascii_case(&s.terms_hash)
        {
            return Ok(p.document.clone());
        }
        if k3::listing::listing(&p.document).is_err() {
            return Ok(p.document.clone());
        }
    }
    Err(refuse(format!("交易 {} 的回执里没有这份文书的公告板记录(公告板码、hash 与 key 三样)", l.tx)))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn act_of(w: &shared::Wire, p: &k3::listing::Listing, candidate: u64, buyer: &str, payout: &str, open_by: u64, prev: &str, held: Option<&str>) -> R<(a1::order::Order, Result<eravon_cands::act::Passed, eravon_cands::act::Refused>)> {
    let o = shared::order_of(p.terms_text.as_bytes(), candidate, buyer, payout, open_by, prev)?;
    a1::precheck::not_revoked(&w.t, &w.table, &o, &w.logic)?;
    if let Some(why) = held {
        return Ok((o, Err(eravon_cands::act::Refused { gate: GATE_SERIES, name: E_SERIES_SAME.to_string(), why: why.to_string() })));
    }
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let got = eravon_watch::act::check(&sw, &o);
    Ok((o, got))
}

pub(super) fn deposit_gate(r: &eravon_cands::act::Refused) -> bool {
    deposit_short_gate(&r.gate)
}

pub fn deposit_short_gate(gate: &str) -> bool {
    [a1::precheck::GATE_BOND, a1::precheck::GATE_FUNDS, a1::precheck::GATE_ALLOWANCE].contains(&gate)
}

pub fn deposit_short_of(f: &Fault) -> Option<String> {
    match f {
        Fault::Precheck { gate, why } if deposit_short_gate(gate) => Some(format!("{gate}:{why}")),
        Fault::Down { why, .. } if crate::seller::act_gate_of(f).is_some_and(deposit_short_gate) => Some(why.clone()),
        _ => None,
    }
}

pub fn stage_gate(r: &eravon_cands::act::Refused) -> bool {
    r.gate == a1::precheck::GATE_STAGE
}

pub const E_SERIES_PREV: &str = "E_P3_SERIES_PREV";
pub const E_SERIES_SAME: &str = "E_P3_SERIES_SAME";

pub fn series_prev(data: &str, owner: &str, buyer: &str, p: &k3::listing::Listing, live: &Live<'_>) -> R<String> {
    let Some(sr) = p.terms.series.as_ref() else { return Ok(a1::order::NO_PREV.to_string()) };
    let refuse = |why: String| Fault::Down { layer: "P3", token: E_SERIES_PREV.into(), why };
    let (held, before) = series_orders(data, owner, p, live)?;
    if let Some(instance) = held {
        return Err(Fault::Down { layer: "P3", token: E_SERIES_SAME.into(), why: format!("这个系列的第 {} 段已有订单 {instance}:接着办那一单,不另付一单", sr.stage) });
    }
    if sr.stage <= 1 {
        return Ok(a1::order::NO_PREV.to_string());
    }
    match before {
        Some((key, _)) if !key.eq_ignore_ascii_case(buyer) => Err(refuse(format!("第 {} 段的订单是另一把钥 {key} 签的:一个系列只用一把买家钥", sr.stage - 1))),
        Some((_, instance)) => Ok(instance),
        None => Err(refuse(format!("先签付这个系列的第 {} 段:后段的 prev 是前段订单的实例", sr.stage - 1))),
    }
}

pub fn series_held_order(data: &str, owner: &str, p: &k3::listing::Listing, live: &Live<'_>) -> Option<String> {
    p.terms.series.as_ref()?;
    series_orders(data, owner, p, live).ok()?.0
}

pub type Live<'a> = dyn Fn(&k3::pack::Pack) -> bool + 'a;

pub fn live_on(w: &shared::Wire, now: u64) -> impl Fn(&k3::pack::Pack) -> bool + '_ {
    move |pk: &k3::pack::Pack| {
        let ww = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
        let l = &pk.listing;
        let until = l.order.expiry.min(pk.receipt.open_by);
        !eravon_watch::poll::lapsed(&ww, &pk.instance, &l.logic, &l.order.seller_key, &l.terms_hash, until, now).unwrap_or(false)
    }
}

fn series_orders(data: &str, owner: &str, p: &k3::listing::Listing, live: &Live<'_>) -> R<(Option<String>, Option<(String, String)>)> {
    let Some(sr) = p.terms.series.as_ref() else { return Ok((None, None)) };
    let s = eravon_vault::store::open_read(data)?;
    let mut rows = Vec::new();
    for me in eravon_vault::store::archive_of(data, owner).map_err(|e| Fault::State(e.detail()))? {
        match s.list(&me, Box_::Packs) {
            Ok(r) => rows.extend(r.into_iter().map(|r| (me.clone(), r))),
            Err(eravon_store::fault::Fault::Missing(_)) => {}
            Err(e) => return Err(e.into()),
        }
    }
    let mut before: Option<(String, String)> = None;
    for (me, r) in rows.into_iter().filter(|(_, r)| r.kind == Kind::OrderFile) {
        let Ok(b) = s.part(&me, Box_::Packs, &r.id, "pack.json") else { continue };
        let Ok(pk) = k3::pack::pack(&b) else { continue };
        let (l, rc) = (&pk.listing, &pk.receipt);
        let Some(held) = l.terms.series.as_ref() else { continue };
        let same = held.id.eq_ignore_ascii_case(&sr.id)
            && held.stages == sr.stages
            && l.chain_id == p.chain_id
            && l.logic.eq_ignore_ascii_case(&p.logic)
            && l.order.seller_key.eq_ignore_ascii_case(&p.order.seller_key);
        if !same || (held.stage + 1 >= sr.stage && !live(&pk)) {
            continue;
        }
        if held.stage == sr.stage {
            return Ok((Some(pk.instance.to_ascii_lowercase()), before));
        }
        if held.stage + 1 == sr.stage && before.is_none() {
            before = Some((rc.buyer_key.clone(), pk.instance.to_ascii_lowercase()));
        }
    }
    Ok((None, before))
}

pub const GATE_SERIES: &str = "series";

pub fn series_held(f: &Fault) -> Option<String> {
    match f {
        Fault::Down { token, why, .. } if token == E_SERIES_SAME => Some(why.clone()),
        _ => None,
    }
}

pub(super) fn act_row(n: u64, got: &Result<eravon_cands::act::Passed, eravon_cands::act::Refused>) -> Out {
    match got {
        Ok(p) => Out::O(vec![
            ("candidate", Out::N(n)),
            ("deposit", Out::S(p.priced.deposit.to_dec())),
            ("lf", Out::S(p.priced.lf.to_dec())),
            ("ok", Out::B(true)),
            ("premium", Out::S(p.priced.premium.to_dec())),
        ]),
        Err(r) => Out::O(vec![
            ("candidate", Out::N(n)),
            ("depositShort", Out::B(deposit_gate(r))),
            ("gate", Out::S(r.gate.to_string())),
            ("ok", Out::B(false)),
            ("token", Out::S(r.name.clone())),
            ("why", Out::S(r.why.clone())),
        ]),
    }
}

pub(super) fn held_segment(c: &Ctx, who: &str, p: &k3::listing::Listing, part: &str) -> Option<String> {
    let id = seller::record_id(Kind::Payload, None, &p.terms_hash[2..18]);
    let b = c.read().ok()?.part(who, Box_::Products, &id, part).ok()?;
    String::from_utf8(b).ok()
}

pub(super) fn segments(c: &Ctx, who: &str, p: &k3::listing::Listing) -> Vec<(&'static str, String, Option<String>)> {
    let block = &p.terms.block;
    let mut out = Vec::new();
    for (seg, key, part) in [("goods", "goodsHash", eravon_goods::goods::PART_GOODS_TEXT), ("delivery", "deliveryHash", eravon_goods::goods::PART_DELIVERY_TEXT)] {
        if let Some(h) = block.get(key).and_then(eravon_galeed::Value::as_str) {
            let held = held_segment(c, who, p, part).filter(|t| eravon_goods::terms::segment_hash(t).eq_ignore_ascii_case(h));
            out.push((seg, h.to_string(), held));
        }
    }
    out
}

pub(super) fn open(c: &Ctx, f: &Flags) -> R<Out> {
    let l = linked(c, f)?;
    let (p, kind) = (&l.s, l.kind);
    let mut pool = Vec::new();
    for g in &p.guilds {
        pool.push(Out::O(vec![
            ("amanahHash", Out::S(g.amanah_hash.clone())),
            ("bookCodeHash", Out::S(eravon_roster::pool::standard_code_hash().to_string())),
            ("bookChainId", Out::N(eravon_roster::pool::standard_chain_id())),
            ("guild", Out::S(g.address.clone())),
        ]));
    }
    let (stored, why) = match c.who(f.one("--address")) {
        Err(_) => (String::new(), "这一家还没有买家席的当前钥,故只读不存"),
        Ok(who) => {
            let id = seller::record_id(Kind::Payload, f.opt("--id"), &p.terms_hash[2..18]);
            c.write()?.put(
                &who,
                Kind::Payload,
                &id,
                &[("termsHash".into(), p.terms_hash.clone())],
                &[(eravon_goods::goods::PART_LISTING.into(), l.listing.clone())],
                c.now,
            )?;
            (id, "")
        }
    };
    let mut rows = vec![
        ("chainId", Out::N(p.chain_id)),
        ("stored", Out::S(stored)),
        ("storedWhy", Out::S(why.to_string())),
        ("candidates", Out::N((p.judges.len() + p.guilds.len()) as u64)),
        ("expiry", Out::N(p.order.expiry)),
        ("guilds", Out::A(pool)),
        (
            "judges",
            Out::A(
                p.judges
                    .iter()
                    .map(|j| {
                        Out::O(vec![
                            ("amanahHash", Out::S(j.amanah_hash.clone())),
                            ("judge", Out::S(j.judge.clone())),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("kind", Out::S(kind.to_string())),
        ("logic", Out::S(p.logic.clone())),
        ("amount", Out::S(p.order.amount.to_dec())),
        ("sellerKey", Out::S(p.order.seller_key.clone())),
        ("sellerPayout", Out::S(p.order.seller_payout.clone())),
        ("openWindow", Out::N(p.order.open_window)),
        ("termsHash", Out::S(p.terms_hash.clone())),
        ("token", Out::S(p.token.clone())),
    ];
    let gate = gate_of(c, f, p);
    let mut choosable: Option<Vec<usize>> = None;
    if let Ok(who) = c.who(f.one("--address")) {
        let w = shared::wire(f, c, Some(&who))?;
        let open_by = shared::open_by_at(p.order.expiry, p.order.open_window, c.now)?;
        let total = (p.judges.len() + p.guilds.len()) as u64;
        let mut acts = Vec::new();
        let prev = series_prev(&c.data, &who, &who, p, &live_on(&w, c.now));
        let held = prev.as_ref().err().and_then(series_held);
        let prev = prev.unwrap_or_else(|_| a1::order::NO_PREV.to_string());
        let payout = shared::payout_flag(f, &who)?;
        let ctx = std::sync::Arc::new((w.clone(), p.clone(), who.clone(), payout.clone(), prev.clone(), held.clone()));
        for (n, got) in (0..total).zip(eravon_chain::read::side_by_side((0..total).collect(), move |n| act_of(&ctx.0, &ctx.1, n, &ctx.2, &ctx.3, open_by, &ctx.4, ctx.5.as_deref()))) {
            let (_, got) = got?;
            acts.push(act_row(n, &got));
        }
        rows.push(("acts", Out::A(acts)));
        rows.push(("openBy", Out::N(open_by)));
        let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
        rows.push(("standing", super::standing(&sw, &l.listing, p, &who).out()));
        let segs = segments(c, &who, p);
        rows.push(("segments", Out::A(segs.iter().map(|(s, h, held)| Out::O(vec![("hash", Out::S(h.clone())), ("held", Out::B(held.is_some())), ("segment", Out::S((*s).to_string()))])).collect())));
        let (page, now_choosable) = page_of(c, &w, &l, &who);
        choosable = Some(now_choosable);
        rows.push(("page", page));
    }
    rows.extend(gate_rows(p, &gate, choosable.as_deref()));
    Ok(ok(rows))
}

fn page_of(c: &Ctx, w: &shared::Wire, l: &Linked, who: &str) -> (Out, Vec<usize>) {
    use crate::deskops::paypage as pp;
    let (registries, registry) = match eravon_cands::wire::registries(&w.registries) {
        Ok(r) => (r, Ok(())),
        Err(a) => (Vec::new(), Err((a.name().to_string(), a.detail()))),
    };
    let cw = eravon_cands::wire::Wire { t: w.t.clone(), table: w.table.clone(), registries, now: c.now };
    let chain = pp::PayChain { w, cw: &cw, registry, buyer: &who.to_ascii_lowercase(), now: c.now, data: &c.data };
    match pp::verify_listing(&l.link, &l.listing, who, Ok(&chain), &pp::PayHooks::plain(pp::PayWords::default())) {
        Ok(o) => (pp::opened_rows(&o), o.cands.iter().filter(|x| x.choosable).map(|x| x.n).collect()),
        Err(s) => (s.out(), Vec::new()),
    }
}

pub(super) fn choose(c: &Ctx, f: &Flags) -> R<Out> {
    let l = linked(c, f)?;
    let p = &l.s;
    let n = f.int("--candidate").map_err(|_| Fault::Arg("要 --candidate".into()))?;
    let total = (p.judges.len() + p.guilds.len()) as u64;
    let n = pick_candidate(n, total)?;
    must_pass(&gate_of(c, f, p), n)?;
    let offer = p.candidate_offer(n as usize);
    let sig = p.candidate_sig(n as usize).to_string();
    Ok(ok(vec![
        ("candidate", Out::N(n)),
        ("offerSig", Out::S(sig)),
        ("quoteHash", Out::S(hex(&offer.quote_hash))),
        ("sellerKey", Out::S(offer.seller_key.clone())),
    ]))
}

pub(super) fn pick_candidate(n: u64, total: u64) -> R<u64> {
    match n < total {
        true => Ok(n),
        false => Err(Fault::Arg(format!("这一份清单上只有 {total} 位候选"))),
    }
}


pub(super) fn pool_seq(c: &Ctx, f: &Flags, p: &k3::listing::Listing, candidate: u64) -> R<Option<u64>> {
    let n = candidate as usize;
    if n.checked_sub(p.judges.len()).and_then(|i| p.guilds.get(i)).is_none() {
        return Ok(None);
    }
    let who = c.who(f.one("--address")).ok();
    let w = shared::wire(f, c, who.as_deref())?;
    pool_seq_on(&w, p, candidate)
}

pub fn pool_seq_on(w: &shared::Wire, p: &k3::listing::Listing, candidate: u64) -> R<Option<u64>> {
    let n = candidate as usize;
    let Some(g) = n.checked_sub(p.judges.len()).and_then(|i| p.guilds.get(i)) else {
        return Ok(None);
    };
    let cw = eravon_cands::wire::Wire { t: w.t.clone(), table: w.table.clone(), registries: Vec::new(), now: 0 };
    let book = eravon_goods::pool::LiveBook { w: &cw };
    let unread = |e: String| Fault::Down { layer: "K5", token: "E_POOL_BOOK_UNREAD".into(), why: format!("行会 {} 的名单簿读不成:{e}", g.address) };
    let root = eravon_roster::book::PanelBook::root_of(&book, &g.address).map_err(unread)?;
    let (head, _) = eravon_chain::read::height(&w.t, &w.table)?;
    let now = eravon_scan::scan::block_stamp(&w.t, &w.table, head).map_err(|e| Fault::Down { layer: "A3", token: e.name(), why: e.detail() })?;
    let rec = eravon_roster::pool::frozen(&book, &root, eravon_roster::pool::standard_code_hash(), now, head.saturating_add(1))
        .map_err(unread)?
        .ok_or_else(|| Fault::Down {
            layer: "K5",
            token: "E_POOL_NO_PANEL".into(),
            why: format!("行会 {} 的名单簿上没有已登记、未过期的记录,池回执写不出序号", g.address),
        })?;
    Ok(Some(rec.index))
}

pub(super) fn order_sign(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    if let Some(path) = exporting(f) {
        return order_export(c, f, &who, &path);
    }
    let s = c.session(f.one("--passcode"))?;
    let mut current = s.key_for(&who).map_err(home::down)?;
    current.fill(0);
    let l = linked(c, f)?;
    let p = &l.s;
    new_order_under(&p.logic)?;
    let candidate = f.int("--candidate").map_err(|_| Fault::Arg("要 --candidate".into()))?;
    let payout = shared::payout_flag(f, &who)?;
    let w = shared::wire(f, c, Some(&who))?;
    let prev = series_prev(&c.data, &who, &who, p, &live_on(&w, c.now))?;
    let seq = pool_seq(c, f, p, candidate)?;
    let open_by = shared::open_by_given(f, p.order.expiry, p.order.open_window, c.now)?;
    let second = f.opt("--open-by").is_none().then_some(c.now);
    let pick = key_pick(&s, &who, &w, p, candidate as usize, &payout, open_by, second, &prev)?;
    let open_by = pick.open_by;
    super::sign_gate(&w, &l.listing, p, &pick.address, candidate)?;
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let o = shared::order_of(p.terms_text.as_bytes(), candidate, &pick.address, &payout, open_by, &prev)?;
    let gate = super::pay_gate(&sw, &o);
    let note = gate.refusal().map(|r| (r.name.clone(), format!("{}:{}", r.gate, r.why), format!("{}:{}", r.name, r.why)));
    let deposit_short = match (gate.signs(), note) {
        (true, n) => n.map(|(_, _, say)| say),
        (false, Some((token, why, _))) => return Err(Fault::Down { layer: "V2", token, why }),
        (false, None) => None,
    };
    let mut key = signing_key(&s, &who, &pick)?;
    let segs = segments(c, &who, p);
    let attachments: Vec<String> = segs.iter().filter_map(|(_, _, held)| held.clone()).collect();
    let missing: Vec<String> = segs.iter().filter(|(_, _, held)| held.is_none()).map(|(s, _, _)| s.to_string()).collect();
    let signed = sign_order(&SignIn { key: &key, buyer: &pick.address, payout: &payout, link: &l.link, listing: &l.listing, p, candidate, seq, open_by, prev: &prev, attachments: &attachments });
    key.fill(0);
    let _ = std::hint::black_box(&key);
    let sd = signed?;
    let id = f.opt("--id").unwrap_or_else(|| sd.case_id[2..18].to_string());
    let decimals = shared::wire(f, c, Some(&who)).ok().and_then(|w| a1::chain::erc20_decimals(&w.t, &w.table, &p.token));
    crate::write::free(&[("--out-receipt", sd.receipt.as_slice()), ("--out-pack", sd.pack.as_slice())].iter().filter_map(|(k, b)| f.one(k).map(|p| (p, *b))).collect::<Vec<_>>())?;
    keep_order(&c.write()?, &who, &id, &sd, decimals, c.now)?;
    let (receipt_bytes, pack_bytes, case_id, sig) = (sd.receipt.clone(), sd.pack.clone(), sd.case_id.clone(), sd.sig.clone());
    let d_instance = sd.instance.clone();
    let d_digest = sd.order_digest.clone();
    for (k, p2) in [("--out-receipt", &receipt_bytes), ("--out-pack", &pack_bytes)] {
        if let Some(path) = f.one(k) {
            crate::write::land(path, p2)?;
        }
    }
    Ok(ok(vec![
        ("budget", Out::S(d_instance.clone())),
        ("caseId", Out::S(case_id)),
        ("depositShort", Out::S(deposit_short.unwrap_or_default())),
        ("id", Out::S(id)),
        ("instance", Out::S(d_instance)),
        ("openBy", Out::N(open_by)),
        ("orderDigest", Out::S(d_digest)),
        ("prev", Out::S(prev)),
        ("receiptText", Out::S(k3::transport::encode_receipt(&receipt_bytes))),
        ("segmentsMissing", strs(&missing)),
        ("sig", Out::S(sig)),
    ]))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pick {
    pub address: String,
    pub open_by: u64,
    pub bumped: bool,
    pub stuck: bool,
    pub instance: String,
}

pub const RESIGN_SECONDS: u64 = 60;

pub fn occupied<T: eravon_chain::transport::Transport>(t: &T, table: &eravon_chain::endpoints::Table, instance: &str, logic: &str) -> R<bool> {
    let down = |e: a1::fault::Fault| Fault::Down { layer: "A1", token: "E_A1_READ".into(), why: format!("{}:{e:?}", e.name()) };
    match a1::chain::code_matches(t, table, instance, logic) {
        Ok(_) => {
            let st = a1::chain::status(t, table, instance).map_err(down)?.value;
            let phase = st.get(a1::chain::status_col::PHASE).ok_or_else(|| Fault::Down { layer: "A1", token: "E_A1_READ".into(), why: "status 没有 phase 一栏".into() })?;
            Ok(phase.small().map_err(down)? != 0)
        }
        Err(a1::fault::Fault::CodeHash { got, .. }) => Ok(!got.trim_start_matches("0x").is_empty()),
        Err(e) => Err(down(e)),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn key_pick(_s: &h2::session::Session, me: &str, w: &shared::Wire, p: &k3::listing::Listing, n: usize, payout: &str, open_by: u64, second: Option<u64>, prev: &str) -> R<Pick> {
    let at = |ob: u64| -> R<(String, bool)> {
        let o = shared::order_of(p.terms_text.as_bytes(), n as u64, me, payout, ob, prev)?;
        let instance = shared::derive_rows(&o, p.chain_id, &p.logic).map(|x| x.1)?;
        let taken = occupied(&w.t, &w.table, &instance, &p.logic)?;
        Ok((instance, taken))
    };
    let next = |s: u64| shared::open_by_at(p.order.expiry, p.order.open_window, s).ok();
    let (open_by, instance, bumped, stuck) = resign(open_by, second, &next, &at)?;
    Ok(Pick { address: me.to_string(), open_by, bumped, stuck, instance })
}

fn resign(open_by: u64, second: Option<u64>, next: &dyn Fn(u64) -> Option<u64>, at: &dyn Fn(u64) -> R<(String, bool)>) -> R<(u64, String, bool, bool)> {
    let (mut ob, mut sec) = (open_by, second);
    for tries in 0..=RESIGN_SECONDS {
        let (instance, taken) = at(ob)?;
        if !taken {
            return Ok((ob, instance, tries > 0, false));
        }
        let later = sec.and_then(|s| s.checked_add(1)).and_then(|s| next(s).map(|o| (s, o)));
        match later {
            Some((s, o)) if o != ob && tries < RESIGN_SECONDS => (ob, sec) = (o, Some(s)),
            _ => return Ok((ob, instance, tries > 0, true)),
        }
    }
    unreachable!("the loop answers on its last turn")
}


pub fn signing_key(s: &h2::session::Session, me: &str, pick: &Pick) -> R<[u8; 32]> {
    if pick.stuck {
        return Err(Fault::Down { layer: "P3", token: "E_P3_OPEN_BY_PINNED".into(), why: "这一单的预算地址已被占用,且要约到期时间定死了 openBy:请卖家另出一版".into() });
    }
    s.key_for(me).map_err(home::down)
}

pub struct SignIn<'a> {
    pub key: &'a [u8; 32],
    pub buyer: &'a str,
    pub payout: &'a str,
    pub link: &'a [u8],
    pub listing: &'a [u8],
    pub p: &'a k3::listing::Listing,
    pub candidate: u64,
    pub seq: Option<u64>,
    pub open_by: u64,
    pub prev: &'a str,
    pub attachments: &'a [String],
}

#[derive(Clone, Debug)]
pub struct Signed {
    pub instance: String,
    pub case_id: String,
    pub order_digest: String,
    pub sig: String,
    pub receipt: Vec<u8>,
    pub pack: Vec<u8>,
}

pub fn sign_order(i: &SignIn<'_>) -> R<Signed> {
    let p = i.p;
    new_order_under(&p.logic)?;
    let o = shared::order_of(p.terms_text.as_bytes(), i.candidate, i.buyer, i.payout, i.open_by, i.prev)?;
    let d = o.derive(p.chain_id, &p.logic)?;
    let sig = shared::sign(i.key, &d.order_digest)?;
    let receipt = write::receipt(&p.terms_hash, i.candidate, i.buyer, i.payout, i.open_by, i.prev, &sig, i.seq)?;
    let case_id = hex(&d.salt);
    let pack = write::pack(p.chain_id, &d.instance, &case_id, i.link, i.listing, &receipt, i.attachments)?;
    Ok(Signed { instance: d.instance.clone(), case_id, order_digest: hex(&d.order_digest), sig, receipt, pack })
}

pub fn new_order_under(logic: &str) -> R<()> {
    Ok(a1::chain::listing_payable(logic)?)
}


pub fn new_order_held(c: &Ctx, f: &Flags, w: &shared::Wire, who: &str) -> R<seller::Held> {
    let h = seller::held(c, f, w, who)?;
    new_order_under(&h.logic)?;
    Ok(h)
}

pub fn keep_order(s: &eravon_vault::store::Store, owner: &str, id: &str, sd: &Signed, decimals: Option<u64>, now: u64) -> R<()> {
    let mut fields = vec![("caseId".to_string(), sd.case_id.clone()), ("instance".to_string(), sd.instance.clone())];
    if let Some(d) = decimals {
        fields.push((eravon_watch::units::FIELD.to_string(), d.to_string()));
    }
    s.put(owner, Kind::OrderFile, id, &fields, &[("pack.json".into(), sd.pack.clone()), ("receipt.json".into(), sd.receipt.clone())], now)?;
    Ok(())
}

pub(super) fn pay(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let to = f.opt("--to").ok_or_else(|| Fault::Arg("要 --to(预算地址)".into()))?;
    let token = f.opt("--token").ok_or_else(|| Fault::Arg("要 --token".into()))?;
    let amount = shared::amount_flag(f, "--amount", "--amount-text", || shared::decimals_for(&c.data, shared::wire(f, c, Some(&who)).ok().as_ref(), &token))?
        .ok_or_else(|| Fault::Arg("要 --amount 或 --amount-text".into()))?;
    if let Some(path) = exporting(f) {
        let word = k2::u256::U256::from_dec(&amount).ok_or_else(|| Fault::Arg(format!("--amount 要是十进制整数:{amount}")))?;
        return company_export(c, f, &who, &path, |w, co| {
            order_check(c, f, w, &who, &to)?;
            let (pc, abi) = a1::proposal::wallet_pay(&co.views.wallet, &token, &to, &word.to_be())?;
            Ok(crate::company::Export { verb: "pay", calls: vec![pc], originals: vec![abi], items: Vec::new(), face: Vec::new(), deal: crate::company::Deal::None })
        });
    }
    let w = shared::wire(f, c, Some(&who))?;
    order_check(c, f, &w, &who, &to)?;
    let have = a1::chain::erc20_balance(&w.t, &w.table, &token, &who)?;
    let call = erc20_transfer(&token, &to, &amount)?;
    let balance = k2::u256::U256::from_be(&have.value).to_dec();
    let d = if shared::units_asked(f) { shared::decimals_for(&c.data, Some(&w), &token) } else { None };
    let mut rows = shared::units_rows(f, "amount", &amount, d);
    rows.extend(shared::units_rows(f, "balance", &balance, d));
    rows.extend(vec![("balance", Out::S(balance)), ("amount", Out::S(amount)), ("to", Out::S(to)), ("token", Out::S(token))]);
    rows.extend(shared::deliver_rows(c, f, &w, &who, &call, "0")?);
    rows.extend(w.rows());
    Ok(ok(rows))
}

fn order_check(c: &Ctx, f: &Flags, w: &shared::Wire, who: &str, to: &str) -> R<()> {
    if f.opt("--id").is_none() {
        return Ok(());
    }
    let h = new_order_held(c, f, w, who)?;
    let sw = eravon_watch::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let (listing, p, n) = super::standing::listing_of_pack(&h.pack)?;
    super::stands(&sw, &listing, &p, &h.o.buyer_key, n)?;
    super::standing::must_pay(&sw, &h.o)?;
    let (_, instance, _) = shared::derive_rows(&h.o, w.chain_id, &w.logic)?;
    if !instance.eq_ignore_ascii_case(to) {
        return Err(Fault::Arg(format!("--to 不是这张订单的预算地址 {instance}")));
    }
    Ok(())
}

pub(super) fn erc20_transfer(token: &str, to: &str, amount_dec: &str) -> R<a1::call::Call> {
    let word = k2::u256::U256::from_dec(amount_dec)
        .ok_or_else(|| Fault::Arg(format!("--amount 要是十进制整数:{amount_dec}")))?;
    Ok(a1::call::erc20_transfer(token, to, &word.to_be())?)
}

pub(super) fn deploy(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    if let Some(path) = super::company::exporting(f) {
        return super::company::company_export(c, f, &who, &path, |w, co| {
            let h = new_order_held(c, f, w, &who)?;
            let (p, seller) = super::company::open_plan(w, &h.o, &h.sig_seller, h.guild.is_some(), c.now, &co.address)?;
            let mut e = crate::company::Export::of(p);
            e.deal = crate::company::Deal::Unopened(seller);
            Ok(e)
        });
    }
    if let Ok(Some(key)) = receipt_buyer(c, f, &who) {
        if !key.eq_ignore_ascii_case(&who) {
            return Err(Fault::Down { layer: "buyer", token: super::company::E_COMPANY_PROPOSES.into(), why: format!("这一单的 buyerKey 是 {key},不是本席钥:由它开案(公司以 --export-proposal 导出提案)") });
        }
    }
    seller::deploy(c, f)
}

fn receipt_buyer(c: &Ctx, f: &Flags, who: &str) -> R<Option<String>> {
    let Some(id) = f.opt("--id") else { return Ok(None) };
    let Ok(b) = c.read()?.part(who, Box_::Packs, &id, "pack.json") else { return Ok(None) };
    Ok(k3::pack::pack(&b).ok().map(|pk| pk.receipt.buyer_key))
}

pub(super) fn rebuild_tx(f: &Flags) -> Option<String> {
    f.opt("--tx")
}


pub(super) fn import(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let w = shared::wire(f, c, Some(&who))?;
    let text = f.opt("--pack").ok_or_else(|| Fault::Arg("要 --pack".into()))?;
    let bytes = home::read_bounded(&text)?;
    let pk = k3::pack::pack(&bytes).map_err(Fault::from)?;
    let mut rebuilt = Vec::new();
    if let Some(tx) = rebuild_tx(f) {
        rebuilt = match eravon_scan::scan::rebuild(&w.t, &w.table, &tx, w.chain_id, &w.logic, &pk.case_id).map_err(Fault::from)? {
            eravon_scan::scan::Rebuild::Done(r) => vec![
                ("amount", Out::S(r.amount.clone())),
                ("buyer", Out::S(r.buyer.clone())),
                ("caseId", Out::S(r.case_id.clone())),
                ("members", Out::N(r.members as u64)),
                ("path", Out::S(r.path.to_string())),
                ("seller", Out::S(r.seller.clone())),
                ("termsHash", Out::S(r.terms_hash.clone())),
                ("token", Out::S(r.token.clone())),
            ],
            eravon_scan::scan::Rebuild::Untraced => vec![
                ("state", Out::S("untraced".into())),
                ("why", Out::S(eravon_scan::scan::SAY_UNTRACED.into())),
            ],
        };
    }
    let id = f.opt("--id").unwrap_or_else(|| pk.case_id[2..18].to_string());
    c.write()?
        .put(
            &who,
            Kind::OrderFile,
            &id,
            &[("caseId".into(), pk.case_id.clone()), ("instance".into(), pk.instance.clone())],
            &[("pack.json".into(), bytes.clone())],
            c.now,
        )
        .map_err(Fault::from)?;
    let mut rows = vec![
        ("caseId", Out::S(pk.case_id.clone())),
        ("id", Out::S(id)),
        ("instance", Out::S(pk.instance.clone())),
        ("rebuilt", if rebuilt.is_empty() { Out::B(false) } else { Out::O(rebuilt) }),
    ];
    rows.extend(w.rows());
    Ok(ok(rows))
}

#[cfg(test)]
mod series_held_tests {
    use super::*;

    #[test]
    fn only_the_stage_held_is_said_as_held() {
        let same = Fault::Down { layer: "P3", token: E_SERIES_SAME.into(), why: "这个系列的第 2 段已有订单 0x00000000000000000000000000000000000000b2:接着办那一单,不另付一单".into() };
        assert_eq!(series_held(&same).as_deref(), Some("这个系列的第 2 段已有订单 0x00000000000000000000000000000000000000b2:接着办那一单,不另付一单"));
        for other in [
            Fault::Down { layer: "P3", token: E_SERIES_PREV.into(), why: "先签付这个系列的第 1 段:后段的 prev 是前段订单的实例".into() },
            Fault::Down { layer: "P3", token: E_SERIES_PREV.into(), why: "第 1 段的订单是另一把钥 0xab 签的:一个系列只用一把买家钥".into() },
            Fault::Down { layer: "A5", token: "E_STORE_IO".into(), why: "读不成".into() },
            Fault::Arg(E_SERIES_SAME.into()),
        ] {
            assert_eq!(series_held(&other), None, "{other:?}");
        }
    }
}

#[cfg(test)]
mod deposit_tests {
    #[test]
    fn the_three_deposit_gates_are_the_sellers_deposit_short() {
        use eravon_standard::precheck as p;
        for g in [p::GATE_BOND, p::GATE_FUNDS, p::GATE_ALLOWANCE] {
            assert!(super::deposit_short_gate(g), "{g}");
        }
        assert!(!super::deposit_short_gate(p::GATE_WINDOW));
    }

    #[test]
    fn the_deposit_gates_are_read_off_the_refusal_the_open_gives_whichever_its_form() {
        use eravon_standard::precheck as p;
        let at = |gate: &'static str| crate::seller::act_refused(eravon_cands::act::Refused { gate, name: "E_A1_PRECHECK".into(), why: "9.25 TST on hand, 300 TST wanted".into() });
        for g in [p::GATE_BOND, p::GATE_FUNDS, p::GATE_ALLOWANCE] {
            assert_eq!(super::deposit_short_of(&at(g)), Some(format!("{g}:9.25 TST on hand, 300 TST wanted")), "{g}");
            assert_eq!(super::deposit_short_of(&crate::fault::Fault::Precheck { gate: g.into(), why: "short".into() }), Some(format!("{g}:short")), "{g}");
        }
        for g in [p::GATE_WINDOW, p::GATE_SIG] {
            assert_eq!(super::deposit_short_of(&at(g)), None, "{g}");
        }
        let opened = crate::fault::Fault::Down { layer: "S4", token: crate::seller::E_OPEN_OPENED.into(), why: "0xab 的案已开(阶段 1),不必再开".into() };
        assert_eq!(super::deposit_short_of(&opened), None);
    }
}
