use eravon_literals::json::Out;

use crate::house::seller::Said;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub hash: String,
    pub on_board: bool,
    pub text: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rung {
    pub name: String,
    pub buyer_bps: u64,
    pub fee_bearer: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Points {
    pub amount: String,
    pub token: String,
    pub delivery_window: u64,
    pub confirm_window: u64,
    pub open_window: u64,
    pub buyer_may_open: bool,
    pub ladder: Vec<Rung>,
    pub judges: Vec<String>,
    pub guilds: Vec<String>,
    pub seller_key: String,
    pub seller_payout: String,
}

pub type Cell = Result<String, Said>;

#[derive(Clone, Debug)]
pub struct Digests {
    pub terms: Cell,
    pub goods: Cell,
    pub delivery: Cell,
    pub amanah: Cell,
    pub offer_sig: Cell,
    pub posted_tx: Cell,
    pub case_id: String,
    pub instance: String,
}

#[derive(Clone, Debug)]
pub struct Papers {
    pub terms: eravon_orders::detail::Terms,
    pub points: Option<Points>,
    pub goods: Option<Segment>,
    pub delivery: Option<Segment>,
    pub digests: Digests,
}

pub struct Sources<'a> {
    pub terms: &'a eravon_orders::detail::Terms,
    pub listing: Result<Vec<u8>, Said>,
    pub candidate: u64,
    pub kept: Result<eravon_goods::goods::Kept, Said>,
    pub case_id: String,
    pub instance: String,
}

fn hex(b: &[u8]) -> String {
    format!("0x{}", b.iter().map(|x| format!("{x:02x}")).collect::<String>())
}

fn not_had(what: &str) -> Said {
    Said::bad("E_CASE_NOT_HELD", what.to_string())
}

fn offer_sig(l: &eravon_link::listing::Listing, c: usize) -> Option<String> {
    if l.by_call.iter().any(|(i, _)| *i == c) {
        return Some(String::new());
    }
    let n = l.judges.len();
    let s = if c < n { l.judge_sigs.get(c) } else { l.guild_sigs.get(c - n) };
    s.cloned()
}

fn amanah_of(doc: &eravon_terms::doc::Terms, c: usize) -> Option<String> {
    let n = doc.judges.len();
    if c < n { doc.judges.get(c).map(|j| j.amanah_hash.clone()) } else { doc.guilds.get(c - n).map(|g| g.amanah_hash.clone()) }
}

pub fn segment_of(d: &eravon_terms::doc::Terms, goods: bool, held: Option<&String>) -> Option<Segment> {
    use eravon_galeed::Value;
    let (md, hash) = if goods { ("goods_md", "goodsHash") } else { ("delivery_md", "deliveryHash") };
    if let Some(t) = d.block.get(md).and_then(Value::as_str) {
        return Some(Segment { hash: eravon_goods::terms::segment_hash(t), on_board: true, text: Some(t.to_string()) });
    }
    let h = d.block.get(hash).and_then(Value::as_str)?.to_string();
    Some(Segment { text: held.filter(|t| eravon_goods::terms::segment_hash(t).trim_start_matches("0x").eq_ignore_ascii_case(h.trim_start_matches("0x"))).cloned(), hash: h, on_board: false })
}

pub fn papers(src: Sources<'_>) -> Papers {
    use eravon_galeed::Value;
    let text = match src.terms {
        eravon_orders::detail::Terms::Ok(t) => Some(t.clone()),
        _ => None,
    };
    let doc = text.as_ref().and_then(|t| eravon_terms::doc::terms(t.as_bytes()).ok());
    let points = doc.as_ref().map(|d| {
        let names: Vec<String> = match d.block.get("ladder") {
            Some(Value::Arr(a)) => a.iter().map(|r| r.get("note_md").and_then(Value::as_str).unwrap_or_default().to_string()).collect(),
            _ => Vec::new(),
        };
        Points {
            amount: d.amount.to_dec(),
            token: d.token.clone(),
            delivery_window: d.delivery_window,
            confirm_window: d.confirm_window,
            open_window: d.open_window,
            buyer_may_open: d.buyer_may_open,
            ladder: d.ladder.iter().enumerate().map(|(i, r)| Rung { name: names.get(i).cloned().unwrap_or_default(), buyer_bps: r.buyer_bps, fee_bearer: r.fee_bearer }).collect(),
            judges: d.judges.iter().map(|j| j.judge.clone()).collect(),
            guilds: d.guilds.iter().map(|g| g.address.clone()).collect(),
            seller_key: d.seller_key.clone(),
            seller_payout: d.seller_payout.clone(),
        }
    });
    let kept = src.kept.as_ref().ok();
    let goods = doc.as_ref().and_then(|d| segment_of(d, true, kept.and_then(|k| k.goods_text.as_ref())));
    let delivery = doc.as_ref().and_then(|d| segment_of(d, false, kept.and_then(|k| k.delivery_text.as_ref())));
    let listing = src.listing.as_ref().ok().and_then(|b| eravon_link::listing::listing(b).ok());
    let c = src.candidate as usize;
    let why_terms = || not_had(&crate::house::i18n::t(crate::house::i18n::Key::CaseTermsNotHeld));
    let digests = Digests {
        terms: doc.as_ref().map(|d| hex(&d.terms_hash)).ok_or_else(why_terms),
        goods: goods.as_ref().map(|s| s.hash.clone()).ok_or_else(why_terms),
        delivery: delivery.as_ref().map(|s| s.hash.clone()).ok_or_else(why_terms),
        amanah: doc.as_ref().and_then(|d| amanah_of(d, c)).ok_or_else(why_terms),
        offer_sig: match (&src.listing, &listing) {
            (Err(e), _) => Err(e.clone()),
            (Ok(_), None) => Err(not_had(&crate::house::i18n::t(crate::house::i18n::Key::CaseListingUnread))),
            (Ok(_), Some(l)) => offer_sig(l, c).ok_or_else(|| not_had(&crate::house::i18n::t(crate::house::i18n::Key::CaseListingUnread))),
        },
        posted_tx: match &src.kept {
            Err(e) => Err(e.clone()),
            Ok(k) => k.posted_tx.clone().ok_or_else(|| not_had(&crate::house::i18n::t(crate::house::i18n::Key::CasePostNotHeld))),
        },
        case_id: src.case_id,
        instance: src.instance,
    };
    Papers { terms: src.terms.clone(), points, goods, delivery, digests }
}

pub fn seller_sources<'a>(data: &str, me: &str, o: &eravon_watch::order::Order, terms: &'a eravon_orders::detail::Terms) -> Sources<'a> {
    let said = |(n, d): (String, String)| Said::bad(&n, d);
    let listing = eravon_watch::order::part(data, me, &o.id, &eravon_watch::order::part_listing()).map_err(said);
    let (th, gh, dh) = match terms {
        eravon_orders::detail::Terms::Ok(t) => match eravon_terms::doc::terms(t.as_bytes()) {
            Ok(d) => (
                hex(&d.terms_hash),
                d.block.get("goodsHash").and_then(eravon_galeed::Value::as_str).unwrap_or_default().to_string(),
                d.block.get("deliveryHash").and_then(eravon_galeed::Value::as_str).unwrap_or_default().to_string(),
            ),
            Err(_) => Default::default(),
        },
        _ => Default::default(),
    };
    let listing_sha = listing.as_ref().map(|b| hex(&eravon_galeed::hash::sha256(b))).unwrap_or_default();
    let kept = eravon_goods::goods::kept(data, me, &th, &gh, &dh, &listing_sha).map_err(|f| Said::bad(f.name(), f.detail()));
    Sources { terms, listing, candidate: o.candidate, kept, case_id: o.case_id.clone(), instance: o.instance.clone() }
}

pub fn buyer_sources<'a>(pack: &[u8], terms: &'a eravon_orders::detail::Terms, case_id: &str, instance: &str) -> Sources<'a> {
    let pk = eravon_link::pack::pack(pack).ok();
    let listing = pk.as_ref().map(|p| p.listing.bytes.clone()).ok_or_else(|| not_had(&crate::house::i18n::t(crate::house::i18n::Key::CaseListingUnread)));
    let held = |hash: Option<&str>| -> Option<String> {
        let h = hash?;
        pk.as_ref()?.attachments.iter().find(|a| a.sha256.trim_start_matches("0x").eq_ignore_ascii_case(h.trim_start_matches("0x"))).map(|a| a.text_md.clone())
    };
    let doc = match terms {
        eravon_orders::detail::Terms::Ok(t) => eravon_terms::doc::terms(t.as_bytes()).ok(),
        _ => None,
    };
    let goods_hash = doc.as_ref().and_then(|d| d.block.get("goodsHash").and_then(eravon_galeed::Value::as_str).map(str::to_string));
    let delivery_hash = doc.as_ref().and_then(|d| d.block.get("deliveryHash").and_then(eravon_galeed::Value::as_str).map(str::to_string));
    let kept = Ok(eravon_goods::goods::Kept { goods_text: held(goods_hash.as_deref()), delivery_text: held(delivery_hash.as_deref()), posted_tx: None });
    Sources { terms, listing, candidate: pk.as_ref().map(|p| p.receipt.candidate).unwrap_or(0), kept, case_id: case_id.to_string(), instance: instance.to_string() }
}

pub fn pack_terms(pack: Option<&[u8]>) -> eravon_orders::detail::Terms {
    match pack.and_then(|b| eravon_link::pack::pack(b).ok()) {
        Some(pk) => eravon_orders::detail::Terms::Ok(pk.listing.terms_text.clone()),
        None => eravon_orders::detail::Terms::Missing,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Covenant {
    pub word: String,
    pub version: Option<u64>,
    pub publisher: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Track {
    Named(String),
    Guild(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quoted {
    pub clocks: eravon_cands::lapse::Clocks,
    pub hash: String,
}

#[derive(Clone, Debug)]
pub struct Bench {
    pub track: Track,
    pub covenant: Result<Covenant, Said>,
    pub quote: Result<Quoted, Said>,
}

fn frozen_quote(_data: &str, _me: &str, w: &eravon_cli::shared::Wire, instance: &str, case_id: &str, _now: u64) -> Result<Vec<u8>, Said> {
    opening_init(w, instance, case_id).map(|i| i.quote)
}


fn opening_init(w: &eravon_cli::shared::Wire, instance: &str, case_id: &str) -> Result<eravon_standard::order::Init, Said> {
    let bad = |n: &str, d: String| Said::bad(n, d);
    let (rows, _) = eravon_chain::read::logs(&w.t, &w.table, instance, 0, eravon_chain::seam::Upto::Latest).map_err(|e| bad(&e.name(), e.detail()))?;
    let t0 = eravon_standard::sol::OPENED.topic0();
    let opened = rows
        .iter()
        .filter_map(|r| eravon_scan::log::Log::read(r).ok())
        .find(|l| l.topics.first() == Some(&t0))
        .ok_or_else(|| bad("E_CASE_NOT_OPEN", crate::house::i18n::t(crate::house::i18n::Key::CaseNotOpen)))?;
    match eravon_scan::scan::rebuild(&w.t, &w.table, &opened.tx, w.chain_id, &w.logic, case_id).map_err(|e| bad(&e.name(), e.detail()))? {
        eravon_scan::scan::Rebuild::Done(r) => Ok(r.init),
        eravon_scan::scan::Rebuild::Untraced => Err(bad("E_SCAN_UNTRACED", opened.tx.clone())),
    }
}

pub fn bench(data: &str, me: &str, o: &eravon_watch::order::Order, card: Option<&crate::house::judge_card::Card>, now: u64) -> Result<Bench, Said> {
    let w = crate::house::orders::wire(data, me)?;
    let d = eravon_watch::poll::deal(data, me, o, &crate::house::orders::sw(&w)).map_err(|(n, x)| Said::bad(&n, x))?;
    let word = eravon_standard::order::hex_of(&d.o.offer.amanah_hash);
    let track = if d.guild { Track::Guild(d.guild_addr.clone()) } else { Track::Named(d.o.offer.judge.clone()) };
    Ok(bench_at(data, me, &w, track, word, &o.instance, &o.case_id, card, now))
}

pub fn buyer_bench(data: &str, me: &str, pack: &[u8], instance: &str, case_id: &str, card: Option<&crate::house::judge_card::Card>, now: u64) -> Result<Bench, Said> {
    let pk = eravon_link::pack::pack(pack).map_err(|e| Said::bad(e.token.name(), crate::house::i18n::t(crate::house::i18n::Key::OrdersNotInTable)))?;
    let w = crate::house::orders::wire(data, me)?;
    let o = eravon_cli::shared::order_of(pk.listing.terms_text.as_bytes(), pk.receipt.candidate, &pk.receipt.buyer_key, &pk.receipt.buyer_payout, pk.receipt.open_by, &pk.receipt.prev).map_err(|f| Said::bad(&f.name(), f.detail()))?;
    let word = eravon_standard::order::hex_of(&o.offer.amanah_hash);
    let c = pk.receipt.candidate as usize;
    let track = match c.checked_sub(pk.listing.judges.len()).and_then(|g| pk.listing.guilds.get(g)) {
        Some(g) => Track::Guild(g.address.to_ascii_lowercase()),
        None => Track::Named(o.offer.judge.clone()),
    };
    Ok(bench_at(data, me, &w, track, word, instance, case_id, card, now))
}

#[allow(clippy::too_many_arguments)]
fn bench_at(data: &str, me: &str, w: &eravon_cli::shared::Wire, track: Track, word: String, instance: &str, case_id: &str, card: Option<&crate::house::judge_card::Card>, now: u64) -> Bench {
    let from_card = |c: &crate::house::judge_card::Card| c.frozen_text.clone().map(|text| Covenant { word: word.clone(), version: c.frozen_version, publisher: c.address.clone(), text });
    let covenant = match &track {
        Track::Named(j) => match card.and_then(from_card) {
            Some(c) => Ok(c),
            None => crate::house::judge_card::read(data, me, j, &word, now).and_then(|c| {
                from_card(&c).ok_or_else(|| {
                    if c.frozen_discarded {
                        Said::bad(eravon_cands::act::E_ACT_AMANAH_DISCARDED, crate::house::i18n::t(crate::house::i18n::Key::ErrEACTAMANAHDISCARDED))
                    } else {
                        Said::bad("E_CASE_COVENANT_ABSENT", crate::house::i18n::t(crate::house::i18n::Key::JudgeFrozenAbsent))
                    }
                })
            }),
        },
        Track::Guild(g) => {
            let absent = |a: eravon_cands::fault::Absent| Said::bad(a.name(), a.detail());
            let read = eravon_cands::wire::Wire::of(data, me, &crate::house::orders::injected(), now)
                .map_err(absent)
                .and_then(|cw| eravon_cands::registry::logs_of(&cw, &[g.to_ascii_lowercase()]).map_err(absent))
                .map(|logs| eravon_cands::guild::covenant_by_word(&logs, g, &word));
            match read {
                Ok(Some(t)) => t.text.map(|text| Covenant { word: word.clone(), version: t.version, publisher: g.clone(), text }).map_err(|e| Said::bad("E_CASE_COVENANT_ABSENT", e)),
                Ok(None) | Err(_) => card.and_then(from_card).ok_or_else(|| Said::bad("E_CASE_COVENANT_ABSENT", crate::house::i18n::t(crate::house::i18n::Key::JudgeFrozenAbsent))),
            }
        }
    };
    let quote = frozen_quote(data, me, w, instance, case_id, now).and_then(|b| {
        let hash = hex(&eravon_galeed::hash::keccak256(&b));
        eravon_cands::lapse::of_bytes(&b).map(|clocks| Quoted { clocks, hash }).map_err(|e| Said::bad("E_ACT_BAD_QUOTE", e))
    });
    Bench { track, covenant, quote }
}

pub fn papers_out(p: &Papers) -> Out {
    let cell = |c: &Cell| match c {
        Ok(v) => Out::S(v.clone()),
        Err(s) => Out::O(vec![("refused", s.out())]),
    };
    let seg = |s: &Option<Segment>| match s {
        None => Out::S(String::new()),
        Some(s) => Out::O(vec![("hash", Out::S(s.hash.clone())), ("held", Out::B(s.text.is_some())), ("onBoard", Out::B(s.on_board))]),
    };
    let terms = match &p.terms {
        eravon_orders::detail::Terms::Ok(_) => "ok",
        eravon_orders::detail::Terms::Missing => "missing",
        eravon_orders::detail::Terms::Unreadable(..) => "unreadable",
        eravon_orders::detail::Terms::Mismatch(..) => "mismatch",
    };
    let d = &p.digests;
    let mut r = vec![
        (
            "digests",
            Out::O(vec![
                ("amanah", cell(&d.amanah)),
                ("caseId", Out::S(d.case_id.clone())),
                ("delivery", cell(&d.delivery)),
                ("goods", cell(&d.goods)),
                ("instance", Out::S(d.instance.clone())),
                ("offerSig", cell(&d.offer_sig)),
                ("postedTx", cell(&d.posted_tx)),
                ("terms", cell(&d.terms)),
            ]),
        ),
        ("delivery", seg(&p.delivery)),
        ("goods", seg(&p.goods)),
        ("terms", Out::S(terms.into())),
    ];
    if let Some(g) = p.goods.as_ref().and_then(|g| g.text.as_ref()) {
        r.push(("goodsRefs", refs_out(&eravon_goods::refs::read(g))));
    }
    if let Some(x) = &p.points {
        r.push((
            "points",
            Out::O(vec![
                ("amount", Out::S(x.amount.clone())),
                ("buyerMayOpen", Out::B(x.buyer_may_open)),
                ("confirmWindow", Out::N(x.confirm_window)),
                ("deliveryWindow", Out::N(x.delivery_window)),
                ("guilds", Out::A(x.guilds.iter().map(|g| Out::S(g.clone())).collect())),
                ("judges", Out::A(x.judges.iter().map(|g| Out::S(g.clone())).collect())),
                ("ladder", Out::A(x.ladder.iter().map(|r| Out::O(vec![("buyerBps", Out::N(r.buyer_bps)), ("feeBearer", Out::N(r.fee_bearer)), ("name", Out::S(r.name.clone()))])).collect())),
                ("openWindow", Out::N(x.open_window)),
                ("sellerKey", Out::S(x.seller_key.clone())),
                ("sellerPayout", Out::S(x.seller_payout.clone())),
                ("token", Out::S(x.token.clone())),
            ]),
        ));
    }
    r.sort_by(|a, b| a.0.cmp(b.0));
    Out::O(r)
}

pub fn refs_out(r: &eravon_goods::refs::Read) -> Out {
    let x = &r.refs;
    Out::O(vec![
        ("goodsPage", Out::S(x.page.clone().unwrap_or_default())),
        ("images", Out::A(x.images.iter().map(|i| Out::O(vec![("name", Out::S(i.name.clone())), ("sha256", Out::S(i.sha256.clone()))])).collect())),
    ])
}

pub fn bench_out(b: &Result<Bench, Said>) -> Out {
    let b = match b {
        Err(s) => return Out::O(vec![("refused", s.out())]),
        Ok(b) => b,
    };
    let (track, at) = match &b.track {
        Track::Named(j) => ("named", j.clone()),
        Track::Guild(g) => ("guild", g.clone()),
    };
    let covenant = match &b.covenant {
        Ok(c) => Out::O(vec![
            ("publisher", Out::S(c.publisher.clone())),
            ("text", Out::S(c.text.clone())),
            ("version", c.version.map(Out::N).unwrap_or(Out::S(String::new()))),
            ("word", Out::S(c.word.clone())),
        ]),
        Err(s) => Out::O(vec![("refused", s.out())]),
    };
    let quote = match &b.quote {
        Ok(q) => Out::O(vec![
            ("autoAccept", Out::B(q.clocks.auto_accept)),
            ("hash", Out::S(q.hash.clone())),
            ("lapse", Out::S(q.clocks.lapse.name().into())),
            ("offerWindow", Out::N(q.clocks.offer_window)),
            ("rulingWindow", Out::N(q.clocks.ruling_window)),
        ]),
        Err(s) => Out::O(vec![("refused", s.out())]),
    };
    Out::O(vec![("at", Out::S(at)), ("covenant", covenant), ("quote", quote), ("track", Out::S(track.into()))])
}

pub fn lapse_out(c: &eravon_cands::lapse::Clocks, guild: bool) -> Out {
    Out::O(vec![
        ("cases", Out::A(eravon_cands::lapse::Case::ALL.iter().map(|k| Out::O(vec![("case", Out::S(k.name().into())), ("outcome", Out::S(k.outcome(c.lapse, guild).name().into()))])).collect())),
        ("lapse", Out::S(c.lapse.name().into())),
        ("offerWindow", Out::N(c.offer_window)),
        ("rulingWindow", Out::N(c.ruling_window)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms_text(goods_private: bool) -> (String, String) {
        let goods = "海报\n\n一张 A2\n\n```eravon.goods-refs/1\npage https://shop.example/a\nimage cee92b3792354def8ff020d77c7496d9615c3193bd7f9e12a5d508ddc01e5173 正面 图.png\n```";
        let g = if goods_private { format!(r#""goodsHash":"{}""#, eravon_goods::terms::segment_hash(goods)) } else { format!(r#""goods_md":{}"#, String::from_utf8(eravon_literals::json::write(&Out::S(goods.into()))).expect("utf-8")) };
        let j = format!("0x{}", "f6".repeat(20));
        let block = format!(
            r#"{{"amount":"1200000000","buyerMayOpen":false,"chainId":1,"confirmWindow":259200,"deliveryWindow":604800,"delivery_md":"七天内交付","expiry":1793001600,{g},"guilds":[],"judges":[{{"amanahHash":"0x{a}","judge":"{j}","quoteHash":"0x{q}"}}],"ladder":[{{"buyerBps":0,"feeBearer":0,"note_md":"驳回","sellerBps":10000}},{{"buyerBps":10000,"feeBearer":1,"note_md":"全额退款","sellerBps":0}}],"logic":"0x{l}","nonce":1,"openWindow":86400,"sellerFund":"0x{s}","sellerKey":"0x{s}","sellerPayout":"0x{p}","token":"0x{t}","v":"eravon.terms/1"}}"#,
            a = "11".repeat(32),
            q = "22".repeat(32),
            l = "33".repeat(20),
            s = "44".repeat(20),
            p = "55".repeat(20),
            t = "66".repeat(20)
        );
        let v = eravon_galeed::canon::read(block.as_bytes()).expect("block");
        let canon = String::from_utf8(eravon_galeed::canon::canonical(&v).expect("canonical")).unwrap();
        (format!("```eravon.terms/1\n{canon}\n```\n"), goods.to_string())
    }

    fn src(terms: &eravon_orders::detail::Terms, kept: eravon_goods::goods::Kept) -> Sources<'_> {
        Sources { terms, listing: Err(Said::bad("E_STORE_MISSING", "无")), candidate: 0, kept: Ok(kept), case_id: "0xc".into(), instance: "0xi".into() }
    }

    #[test]
    fn the_papers_show_what_the_case_froze_and_say_what_is_not_held() {
        let (t, goods) = terms_text(false);
        let ok = eravon_orders::detail::Terms::Ok(t.clone());
        let p = papers(src(&ok, Default::default()));
        let g = p.goods.clone().expect("goods");
        assert!(g.on_board && g.text.as_deref() == Some(goods.as_str()));
        assert_eq!(g.hash, eravon_goods::terms::segment_hash(&goods));
        let pts = p.points.clone().expect("points");
        assert_eq!((pts.delivery_window, pts.confirm_window, pts.ladder[0].name.as_str()), (604_800, 259_200, "驳回"));
        assert_eq!(p.digests.terms.clone().unwrap(), format!("0x{}", eravon_galeed::hash::sha256(t.as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>()));
        assert_eq!(p.digests.amanah.clone().unwrap(), format!("0x{}", "11".repeat(32)));
        assert_eq!(p.digests.posted_tx.clone().unwrap_err().name, "E_CASE_NOT_HELD", "a post not kept is said, never invented");
        assert!(p.digests.offer_sig.is_err(), "no listing, no signature");
        let refs = eravon_goods::refs::read(g.text.as_deref().unwrap()).refs;
        assert_eq!((refs.page.as_deref(), refs.images[0].name.as_str()), (Some("https://shop.example/a"), "正面 图.png"));
        let (t2, goods2) = terms_text(true);
        let ok2 = eravon_orders::detail::Terms::Ok(t2);
        let held = papers(src(&ok2, eravon_goods::goods::Kept { goods_text: Some(goods2.clone()), delivery_text: None, posted_tx: Some("0xtx".into()) }));
        let g2 = held.goods.unwrap();
        assert!(!g2.on_board && g2.text.as_deref() == Some(goods2.as_str()));
        assert_eq!(held.digests.posted_tx.unwrap(), "0xtx");
        let other = papers(src(&ok2, eravon_goods::goods::Kept { goods_text: Some("别的正文".into()), ..Default::default() }));
        assert_eq!(other.goods.unwrap().text, None, "a text whose digest differs is not this case's");
        for odd in [eravon_orders::detail::Terms::Missing, eravon_orders::detail::Terms::Unreadable("E_X".into(), "坏".into()), eravon_orders::detail::Terms::Mismatch("0xa".into(), "0xb".into())] {
            let p = papers(src(&odd, Default::default()));
            assert!(p.points.is_none() && p.goods.is_none());
            assert_eq!(p.terms, odd);
            assert!(p.digests.terms.is_err() && p.digests.amanah.is_err());
        }
    }
}
