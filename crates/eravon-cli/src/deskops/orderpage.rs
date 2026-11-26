use eravon_literals::json::Out;
use eravon_orders::table::Row;
use crate::deskops::said::Said;

pub fn row_cells(r: &Row) -> Vec<(&'static str, Out)> {
    let (state, reason, bad) = match &r.status {
        Ok(st) => (st.word.word(), st.reason.clone(), String::new()),
        Err((n, d)) => ("", d.clone(), n.clone()),
    };
    let s = |x: &str| Out::S(x.to_string());
    vec![
        ("acts", Out::A(acts(r).iter().map(|x| s(x.word())).collect())),
        ("amount", s(&r.amount)),
        ("bad", Out::S(bad)),
        ("buyer", s(&r.buyer)),
        ("caseId", s(&r.case_id)),
        ("ended", ended_out(r)),
        ("instance", s(&r.instance)),
        ("label", s(&r.label)),
        ("number", s(&r.number)),
        ("past", Out::B(r.past)),
        ("product", s(&r.product)),
        ("reason", Out::S(reason)),
        ("record", s(&r.record)),
        ("series", series_out(&r.series)),
        ("state", s(state)),
    ]
}

pub fn ended_out(r: &eravon_orders::table::Row) -> Out {
    Out::S(r.chain.as_ref().and_then(eravon_orders::status::ended).map(|e| e.word()).unwrap_or("").to_string())
}

pub fn series_out(x: &Option<eravon_terms::doc::Series>) -> Out {
    match x {
        Some(x) => Out::O(vec![("id", Out::S(x.id.clone())), ("stage", Out::N(x.stage)), ("stages", Out::N(x.stages))]),
        None => Out::Null,
    }
}

pub fn acts(r: &Row) -> &'static [eravon_orders::status::Act] {
    if r.past && !eravon_literals::constants::past_switch() {
        return &[];
    }
    shown_acts(r)
}

pub fn shown_acts(r: &Row) -> &'static [eravon_orders::status::Act] {
    if r.unrecorded() {
        return &[];
    }
    r.status.as_ref().map(|s| s.word.acts()).unwrap_or(&[])
}

pub fn order_company(data: &str, addr: &str, o: &eravon_watch::order::Order, wire: &dyn Fn() -> Option<crate::shared::Wire>, route: Route) -> Result<Option<String>, Said> {
    let on = crate::deskops::buyerpage::company_mode(data, addr).map(|(c, _)| c);
    match listed_seller(data, addr, o).or_else(|| wire().and_then(|w| chain_seller(&w, o))) {
        None if on.is_none() => Ok(None),
        None => Err(Said::bad("E_ORDERS_SELLER_UNKNOWN", eravon_vault::lang::t(eravon_vault::lang::Key::OrdersSellerUnknown))),
        Some(s) => route(addr, &s, on.as_deref(), &mut || wire().and_then(|w| crate::deskops::buyerpage::acting_on(&w, &o.instance, &s))),
    }
}

pub type Route = fn(&str, &str, Option<&str>, &mut dyn FnMut() -> Option<String>) -> Result<Option<String>, Said>;

pub fn listed_seller(data: &str, addr: &str, o: &eravon_watch::order::Order) -> Option<String> {
    use eravon_watch::order as od;
    od::part(data, addr, &o.id, &od::part_listing()).ok().and_then(|b| eravon_link::listing::listing(&b).ok()).map(|p| p.order.seller_key.to_ascii_lowercase())
}

pub fn chain_seller(w: &crate::shared::Wire, o: &eravon_watch::order::Order) -> Option<String> {
    let st = eravon_standard::chain::status(&w.t, &w.table, &o.instance).ok()?;
    let seats = st.value.get(eravon_standard::chain::status_col::SEATS)?.small().ok()?;
    let case = eravon_standard::order::b32_of("案号", &o.case_id).ok()?;
    let seat = eravon_standard::chain::seat_of(&w.t, &w.table, &o.instance, &case, seats.checked_sub(1)?).ok()?.value;
    let parties = seat.get(eravon_dispute::seat_col::PARTIES)?.items().ok()?;
    parties.get(eravon_dispute::seat_col::PARTY_SELLER_KEY)?.addr().ok().map(|a| eravon_standard::order::hex_of(&a).to_ascii_lowercase())
}

pub fn route_of(addr: &str, seller: &str, company: Option<&str>, acting: &mut dyn FnMut() -> Option<String>) -> Result<Option<String>, Said> {
    if seller.eq_ignore_ascii_case(addr) {
        return Ok(None);
    }
    let now = acting();
    if now.as_deref().is_some_and(|k| k.eq_ignore_ascii_case(addr)) {
        return Ok(None);
    }
    match company {
        Some(c) if crate::deskops::buyerpage::acts_for(c, seller, now) => Ok(Some(c.to_string())),
        _ => Err(Said::bad("E_ORDERS_SELLER_KEY", eravon_vault::lang::filln(eravon_vault::lang::Key::OrdersSellerOther, &[seller]))),
    }
}

#[derive(Clone, Debug)]
pub struct Detail {
    pub overview: eravon_orders::detail::Overview,
    pub delivery: eravon_orders::detail::Delivery,
    pub dispute: Option<eravon_orders::detail::Dispute>,
    pub record: Result<Vec<eravon_orders::detail::Moment>, Said>,
    pub fee: eravon_orders::detail::Fee,
    pub chosen: Option<String>,
    pub advance: Option<Result<eravon_watch::pool::Due, Said>>,
}

pub fn read_detail(data: &str, addr: &str, w: &eravon_watch::Wire<'_>, r: &Row) -> (Detail, Option<Said>) {
    let o = eravon_orders::detail::order_of(data, addr, r);
    let ctx = std::sync::Arc::new((w.held(), data.to_string(), addr.to_string(), r.clone(), o.clone()));
    let delivery = {
        let c = ctx.clone();
        move || eravon_orders::detail::delivery(&c.1, &c.2, &c.0.wire(), &c.3, c.4.as_ref())
    };
    let dispute = {
        let c = ctx.clone();
        move || match eravon_orders::table::keys_of(&c.1, &c.2, eravon_scan::scan::Side::Seller) {
            Ok(keys) => (eravon_orders::detail::dispute_keyed(&c.1, &c.2, &keys, &c.0.wire(), &c.3, c.4.as_ref()), None),
            Err((n, d)) => (None, Some(Said::bad(&n, d))),
        }
    };
    let record = {
        let c = ctx.clone();
        move || eravon_orders::detail::record(&c.0.wire(), &c.3, c.4.as_ref()).map_err(|(n, d)| Said::bad(&n, d))
    };
    let (delivery, ((dispute, refused), record)) = if eravon_chain::recall::current().is_none() {
        eravon_chain::read::both(delivery, move || eravon_chain::read::both(dispute, record))
    } else {
        (delivery(), (dispute(), record()))
    };
    let detail = Detail {
        overview: eravon_orders::detail::overview(data, addr, r, o.as_ref()),
        delivery,
        dispute,
        record,
        fee: eravon_orders::detail::fee(r),
        chosen: None,
        advance: None,
    };
    (detail, refused)
}

fn day(t: u64) -> Out {
    Out::S(if t == 0 { String::new() } else { eravon_orders::day::of(t) })
}

fn at(t: u64) -> Out {
    Out::S(if t == 0 { String::new() } else { eravon_orders::day::at(t) })
}

pub fn page_row(r: &Row, tokens: &[eravon_store::settings::TokenRow]) -> Out {
    let s = |x: &str| Out::S(x.to_string());
    let say = match &r.status {
        Ok(st) => st.word.say().to_string(),
        Err(_) => String::new(),
    };
    let mut m = row_cells(r);
    m.extend(vec![
        ("amountWord", s(&eravon_orders::table::amount_in(tokens, &eravon_vault::tokens::ZH, &r.amount, r.decimals, &r.token, eravon_vault::tokens::Form::Short))),
        ("deadline", day(r.deadline)),
        ("placed", day(r.placed)),
        ("say", Out::S(say)),
        ("token", s(&eravon_vault::tokens::name(tokens, eravon_vault::tokens::chain(), &r.token, &eravon_vault::tokens::ZH))),
    ]);
    Out::O(m)
}

pub fn due_word(d: &eravon_watch::pool::Due) -> &'static str {
    match d {
        eravon_watch::pool::Due::No => "no",
        eravon_watch::pool::Due::Seat(_) => "seat",
        eravon_watch::pool::Due::Resigned => "resigned",
        eravon_watch::pool::Due::Draw => "draw",
        eravon_watch::pool::Due::Past => "past",
    }
}

pub fn detail_out(d: &Detail, r: &Row, order: Option<&eravon_watch::order::Order>, tokens: &[eravon_store::settings::TokenRow], terms_open: bool) -> Out {
    let s = |x: &str| Out::S(x.to_string());
    let ov = &d.overview;
    let mut o = vec![("row", page_row(r, tokens)), ("judge", s(&ov.judge.addr)), ("judgeState", s(ov.judge.state)), ("placed", s(&ov.placed))];
    match &ov.terms {
        eravon_orders::detail::Terms::Ok(t) => {
            o.push(("terms", s("ok")));
            if terms_open {
                o.push(("termsText", s(t)));
            }
        }
        eravon_orders::detail::Terms::Missing => o.push(("terms", s("missing"))),
        eravon_orders::detail::Terms::Unreadable(name, detail) => {
            o.push(("terms", s("unreadable")));
            o.push(("termsGot", s(&format!("{name}:{detail}"))));
        }
        eravon_orders::detail::Terms::Mismatch(want, got) => {
            o.push(("terms", s("mismatch")));
            o.push(("termsGot", s(got)));
            o.push(("termsWant", s(want)));
        }
    }
    if let Some(a) = &d.advance {
        o.push(("advance", a.as_ref().map_or_else(Said::out, |due| s(due_word(due)))));
    }
    let dl = &d.delivery;
    let files = |f: &[(String, u64, String)]| Out::A(f.iter().map(|(n, b, h)| Out::O(vec![("bytes", Out::N(*b)), ("name", s(n)), ("sha256", s(h))])).collect());
    let mut det = vec![
        ("overview", Out::O(o)),
        (
            "delivery",
            Out::O(vec![
                ("at", at(dl.at)),
                ("deadline", day(dl.deadline)),
                ("done", Out::B(dl.done)),
                ("files", dl.files.as_ref().map_or_else(|(n, d)| Said::bad(n, d.clone()).out(), |f| files(f))),
                ("note", s(&dl.note)),
                ("tx", s(&dl.tx)),
            ]),
        ),
    ];
    if let Some(h) = &dl.hand_over {
        det.push((
            "handOver",
            match h {
                Ok(h) => Out::O(vec![("claimBlock", Out::N(h.claim_block)), ("finalized", Out::N(h.finalized)), ("ready", Out::B(h.ready))]),
                Err((n, d)) => Out::O(vec![("detail", s(d)), ("name", s(n))]),
            },
        ));
    }
    let full = |raw: &str, decimals: Option<u64>, token: &str| eravon_orders::table::amount_in(tokens, &eravon_vault::tokens::ZH, raw, decimals, token, eravon_vault::tokens::Form::Full);
    if let Some(x) = &d.dispute {
        let evidence = x
            .evidence
            .iter()
            .map(|e| {
                Out::O(vec![
                    ("at", Out::S(eravon_orders::day::at(e.at))),
                    ("by", s(&e.by)),
                    ("files", e.files.as_ref().map_or_else(|(n, d)| Said::bad(n, d.clone()).out(), |f| Out::A(f.iter().map(|(n, b)| Out::O(vec![("bytes", Out::N(*b)), ("name", s(n))])).collect()))),
                    ("manifest", s(&e.manifest)),
                    ("ours", Out::B(e.ours)),
                    ("tx", s(&e.tx)),
                ])
            })
            .collect();
        let lf_word = full(&x.lf, order.and_then(eravon_watch::units::of), order.map(|o| o.token.as_str()).unwrap_or_default());
        det.push((
            "dispute",
            Out::O(vec![
                ("at", Out::S(eravon_orders::day::at(x.at))),
                ("deadline", day(x.deadline)),
                ("evidence", Out::A(evidence)),
                ("lf", s(&x.lf)),
                ("lfWord", s(&lf_word)),
            ]),
        ));
    }
    match &d.record {
        Ok(rows) => det.push(("record", Out::A(rows.iter().map(|x| Out::O(vec![("at", at(x.at)), ("name", s(&x.name)), ("say", s(&x.say)), ("tx", s(&x.tx))])).collect()))),
        Err(e) => det.push(("record", e.out())),
    }
    let note = order.and_then(|o| o.extra("sweepNote").map(str::to_string)).unwrap_or_default();
    det.push(("recordNote", s(&note)));
    let f = &d.fee;
    let word = |x: &str| s(&full(x, r.decimals, &r.token));
    let mut fee = vec![
        ("amount", s(&f.amount)),
        ("amountWord", word(&f.amount)),
        ("judgeFeeWord", word(&f.judge_fee)),
        ("deposit", s(&f.deposit)),
        ("judgeFee", s(&f.judge_fee)),
        ("lf", s(&f.lf.0)),
        ("lfHeld", s(&f.lf.1)),
    ];
    if let Some((b, se, j, bb)) = &f.split {
        fee.push(("split", Out::O(vec![("bondToBuyer", s(bb)), ("buyer", s(b)), ("judge", s(j)), ("seller", s(se)), ("sellerWord", word(se))])));
    }
    det.push(("fee", Out::O(fee)));
    Out::O(det)
}

#[cfg(test)]
mod unrecorded_tests {
    #[test]
    fn a_row_with_no_local_record_offers_no_write_action() {
        let st = Ok(eravon_orders::status::Status { word: eravon_orders::status::Word::AwaitDelivery, reason: String::new() });
        let mut r = eravon_orders::table::Row {
            instance: "0xi".into(),
            case_id: String::new(),
            record: String::new(),
            number: String::new(),
            buyer: String::new(),
            label: String::new(),
            product: String::new(),
            amount: String::new(),
            token: String::new(),
            status: st,
            deadline: 0,
            placed: 0,
            block: None,
            chain: None,
            local: None,
            handed: false,
            note: String::new(),
            terms_hash: String::new(),
            decimals: None,
            past: false,
            logic: String::new(),
            series: None,
        };
        assert!(super::shown_acts(&r).is_empty(), "no record: no write");
        r.record = "rec-a".into();
        assert!(super::shown_acts(&r).contains(&eravon_orders::status::Act::Deliver), "with its record the status's actions stand");
    }
}
