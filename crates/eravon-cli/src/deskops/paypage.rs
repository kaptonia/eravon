use eravon_chain as a2;
use eravon_link as k3;
use eravon_literals::json::Out;
use eravon_standard as a1;
use eravon_terms as k2;

use super::said::Said;
use crate::shared::Wire;

pub const PL_STEP_PAYLOAD: &str = "payload";
pub const PL_STEP_TERMS: &str = "terms";
pub const PL_STEP_BOUND: &str = "bound";
pub const PL_STEP_JUDGES: &str = "judges";
pub const PL_STEP_QUOTE: &str = "quote";
pub const PL_STEP_STANDARD: &str = "standard";
pub const PL_STEP_PRICE: &str = "price";
pub const PL_STEP_CONSENT: &str = "consent";
pub const PL_STEP_PLACE: &str = "place";
pub const PL_STEP_BOOK: &str = "book";
pub const PL_STEP_REVOKED: &str = "revoked";
pub const PL_STEP_CURRENT: &str = "current";
pub const PL_STEP_SERIES: &str = "series";
pub const PL_STEP_DEPOSIT: &str = "deposit";
pub const PL_STEP_ACT: &str = "act";
pub const PL_OVER_CAP: &str = "超出该判官的接单上限";
pub const PL_PARTY: &str = "判官是本单当事人";

pub struct PayChain<'a> {
    pub w: &'a Wire,
    pub cw: &'a eravon_cands::wire::Wire,
    pub registry: Result<(), Fail>,
    pub buyer: &'a str,
    pub now: u64,
    pub data: &'a str,
}

pub struct PayHeld {
    w: Wire,
    cw: eravon_cands::wire::Wire,
    registry: Result<(), Fail>,
    buyer: String,
    now: u64,
    data: String,
}

impl PayHeld {
    pub fn of(c: &PayChain) -> PayHeld {
        PayHeld { w: c.w.clone(), cw: c.cw.clone(), registry: c.registry.clone(), buyer: c.buyer.to_string(), now: c.now, data: c.data.to_string() }
    }

    pub fn chain(&self) -> PayChain<'_> {
        PayChain { w: &self.w, cw: &self.cw, registry: self.registry.clone(), buyer: &self.buyer, now: self.now, data: &self.data }
    }
}

pub type Fail = (String, String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    Pass,
    Fail(String, String),
}

impl State {
    pub fn word(&self) -> &'static str {
        match self {
            State::Pass => "pass",
            State::Fail(..) => "fail",
        }
    }

    pub fn of<T>(r: &Result<T, Fail>) -> State {
        match r {
            Ok(_) => State::Pass,
            Err((n, d)) => State::Fail(n.clone(), d.clone()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cand {
    pub n: usize,
    pub kind: &'static str,
    pub address: String,
    pub amanah: String,
    pub ruling_window: u64,
    pub premium: String,
    pub lf: String,
    pub deposit: String,
    pub priced_at_open: bool,
    pub priced_why: (String, String),
    pub by_call: bool,
    pub choosable: bool,
    pub refusal: String,
    pub note: String,
    pub joined: bool,
    pub covenant: Option<Result<String, String>>,
    pub name: String,
}

impl Cand {
    pub fn note_said(&self) -> String {
        if self.joined { joined(&self.refusal, &self.note) } else { self.note.clone() }
    }

    pub fn label(&self) -> String {
        if self.name.trim().is_empty() { eravon_orders::table::short(&self.address) } else { self.name.trim().to_string() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opened {
    pub steps: Vec<(&'static str, State)>,
    pub title: String,
    pub amount: String,
    pub token: String,
    pub seller_key: String,
    pub t1: u64,
    pub t2: u64,
    pub rungs: Vec<(u64, u64, u64)>,
    pub terms_text: Option<String>,
    pub expiry: u64,
    pub cands: Vec<Cand>,
    pub quotes: Vec<Option<k2::quote::Quote>>,
    pub chain_id: u64,
    pub logic: String,
    pub open_window: u64,
    pub link: Vec<u8>,
    pub listing: Vec<u8>,
    pub source: String,
    pub balance: Option<String>,
    pub decimals: Option<u64>,
    pub gap: Option<String>,
    pub segments: Vec<(&'static str, String)>,
    pub deposit_short: Vec<(usize, String, String)>,
    pub stage_waits: bool,
    pub series_held: Option<String>,
}

pub fn act(c: &PayChain, o: &a1::order::Order) -> Result<eravon_cands::act::Passed, eravon_cands::act::Refused> {
    let sw = eravon_watch::Wire { t: &c.w.t, table: &c.w.table, chain_id: c.w.chain_id, logic: &c.w.logic, registries: &c.w.registries };
    eravon_watch::act::check(&sw, o)
}
pub fn quote_bytes_of(c: &PayChain, p: &k3::listing::Listing, n: usize) -> Result<Vec<u8>, Fail> {
    let (judge, guild, amanah, want) = match n.checked_sub(p.judges.len()) {
        None => (p.judges[n].judge.clone(), String::new(), p.judges[n].amanah_hash.clone(), p.judges[n].quote_hash.clone()),
        Some(g) => {
            let g = p.guilds.get(g).ok_or_else(|| ("E_P3_QUOTE".to_string(), format!("清单没有第 {n} 位候选")))?;
            (String::new(), g.address.clone(), g.amanah_hash.clone(), g.quote_hash.clone())
        }
    };
    c.registry.clone()?;
    let zero = || "0x0000000000000000000000000000000000000000".to_string();
    let cand = eravon_cands::act::Candidate {
        judge: if judge.is_empty() { zero() } else { judge.to_ascii_lowercase() },
        guild: if guild.is_empty() { zero() } else { guild.to_ascii_lowercase() },
        amanah_hash: a1::order::b32_of("amanahHash", &amanah).map_err(|f| (f.name().to_string(), f.detail()))?,
        token: p.token.to_ascii_lowercase(),
        parties: vec![p.order.seller_key.to_ascii_lowercase(), c.buyer.to_string()],
    };
    let sw = eravon_watch::Wire { t: &c.w.t, table: &c.w.table, chain_id: c.w.chain_id, logic: &c.w.logic, registries: &c.w.registries };
    eravon_watch::act::quote_bytes(&sw, &cand, &want).map_err(|(name, why)| (name, why))
}
pub fn fees(c: &PayChain, p: &k3::listing::Listing, n: usize, quote: &[u8]) -> Result<a1::precheck::Quoted, Fail> {
    let offer = k2::derive::offer(&p.terms, n as u64).map_err(|f| (f.token.name().to_string(), "无法生成订单要约".to_string()))?;
    a1::precheck::quoted(&c.w.t, &c.w.table, &offer, quote).map_err(|f| (f.name().to_string(), f.detail()))
}
pub type Each = Vec<(usize, Result<Vec<u8>, Fail>)>;

pub fn candidates(c: &PayChain, p: &k3::listing::Listing) -> Each {
    let held = std::sync::Arc::new((PayHeld::of(c), p.clone()));
    eravon_chain::read::side_by_side((0..p.candidate_count()).collect(), move |n| (n, quote_bytes_of(&held.0.chain(), &held.1, n)))
}
pub fn standard(c: &PayChain, p: &k3::listing::Listing) -> Result<(), Fail> {
    let (chain_id, logic, _) = a1::chain::standard_table();
    if p.chain_id != chain_id {
        return Err(("E_P3_STANDARD_CHAIN".to_string(), format!("网络不匹配({} ≠ {chain_id})", p.chain_id)));
    }
    if !p.logic.eq_ignore_ascii_case(logic) {
        a1::chain::listing_payable(&p.logic).map_err(|f| (f.name().to_string(), f.detail()))?;
        return Err(("E_P3_STANDARD_LOGIC".to_string(), format!("清单的逻辑地址 {} 不是标准表那一枚", p.logic)));
    }
    let (code, _) = a2::read::code(&c.w.t, &c.w.table, &p.logic, a2::seam::TAG_LATEST).map_err(|f| (f.name().to_string(), f.detail()))?;
    if code.is_empty() {
        return Err(("E_P3_STANDARD_NO_CODE".to_string(), format!("合约 {} 不存在", p.logic)));
    }
    Ok(())
}

pub fn feeds_of(q: &k2::quote::Quote) -> Vec<(String, u64)> {
    let zero = |a: &str| a.trim_start_matches("0x").chars().all(|c| c == '0');
    if zero(&q.fee_unit) {
        return Vec::new();
    }
    [&q.feed_token, &q.feed_unit].into_iter().filter(|a| !zero(a)).map(|a| (a.to_ascii_lowercase(), q.max_stale)).collect()
}
pub fn price(c: &PayChain, q: &k2::quote::Quote) -> Result<(), Fail> {
    for (at, max_stale) in feeds_of(q) {
        a1::feed::gate(&c.w.t, &c.w.table, &at, c.now, max_stale).map_err(|f| (f.name().to_string(), f.detail()))?;
    }
    Ok(())
}

pub fn consent(c: &PayChain, p: &k3::listing::Listing) -> Vec<(usize, bool, String)> {
    crate::buyer::candidate_gate(Some(c.w), p).iter().enumerate().map(|(n, a)| (n, a.choosable(), a.note().to_string())).collect()
}
pub struct Balance {
    pub have: k2::u256::U256,
    pub decimals: Option<u64>,
}

pub fn balance(c: &PayChain, token: &str) -> Result<Balance, Fail> {
    let r = a1::chain::erc20_balance(&c.w.t, &c.w.table, token, c.buyer).map_err(|f| (f.name().to_string(), f.detail()))?;
    Ok(Balance { have: k2::u256::U256::from_be(&r.value), decimals: eravon_cands::judge::decimals_of(&c.cw, token) })
}

pub fn gap(amount: &k2::u256::U256, have: &k2::u256::U256) -> Option<k2::u256::U256> {
    match amount.cmp_u(have) {
        std::cmp::Ordering::Greater => Some(amount.overflowing_sub(have).0),
        _ => None,
    }
}

pub struct PayHooks<'a> {
    pub candidates: fn(&PayChain, &k3::listing::Listing) -> Each,
    pub standard: fn(&PayChain, &k3::listing::Listing) -> Result<(), Fail>,
    pub price: fn(&PayChain, &k2::quote::Quote) -> Result<(), Fail>,
    pub gap: fn(&k2::u256::U256, &k2::u256::U256) -> Option<k2::u256::U256>,
    pub gate: fn(Option<&k2::quote::Quote>, &k2::u256::U256, &str, &[String], &str) -> (bool, &'static str, String),
    pub words: PayWords<'a>,
}

#[derive(Clone, Copy, Default)]
pub struct PayWords<'a> {
    pub link_broken: &'a str,
    pub not_pay_link: &'a str,
    pub revoked: &'a str,
    pub all_hidden_docs: &'a str,
    pub price_stale: &'a str,
    pub all_hidden_unregistered: &'a str,
    pub all_hidden_retired: &'a str,
    pub deposit_short: &'a str,
    pub all_hidden_act: &'a str,
    pub private_goods: &'a str,
}

impl PayHooks<'_> {
    pub fn plain(words: PayWords<'_>) -> PayHooks<'_> {
        PayHooks { candidates, standard, price, gap, gate, words }
    }
}

pub const NOT_LISTING: &str = "E_P1_NOT_LISTING";

pub fn verify_listing(link: &[u8], listing: &[u8], buyer: &str, chain: Result<&PayChain, &Fail>, h: &PayHooks) -> Result<Opened, Said> {
    let p = match k3::link::resolve(link, listing).map_err(|v| Said::bad(&v.name, h.words.link_broken))? {
        k3::link::Resolved::Listing(s) => *s,
        k3::link::Resolved::Settle(_) => return Err(Said::bad(NOT_LISTING, h.words.not_pay_link)),
    };
    let mut steps: Vec<(&'static str, State)> = vec![
        (PL_STEP_PAYLOAD, State::Pass),
        (PL_STEP_TERMS, State::Pass),
        (PL_STEP_BOUND, State::Pass),
    ];
    let c = match chain {
        Ok(c) => c,
        Err(f) => {
            for s in [PL_STEP_JUDGES, PL_STEP_QUOTE, PL_STEP_STANDARD, PL_STEP_PRICE, PL_STEP_CONSENT] {
                steps.push((s, State::Fail(f.0.clone(), f.1.clone())));
            }
            let notes: Vec<Note> = (0..p.candidate_count()).map(|n| (n, f.0.clone(), f.1.clone(), false)).collect();
            let quotes = vec![None; p.candidate_count()];
            return Ok(page(&p, link, steps, buyer, Some((&notes, &quotes)), h));
        }
    };
    let sw = eravon_watch::Wire { t: &c.w.t, table: &c.w.table, chain_id: c.w.chain_id, logic: &c.w.logic, registries: &c.w.registries };
    let standing = crate::buyer::standing(&sw, listing, &p, buyer);
    let state = |r: &Result<(), (String, String)>| match r {
        Ok(()) => State::Pass,
        Err((n, _)) if n == crate::buyer::E_REVOKED => State::Fail(n.clone(), h.words.revoked.to_string()),
        Err((n, d)) => State::Fail(n.clone(), d.clone()),
    };
    steps.push((PL_STEP_BOOK, state(&standing.book)));
    steps.push((PL_STEP_PLACE, state(&standing.place)));
    steps.push((PL_STEP_REVOKED, state(&standing.revoked)));
    let live = crate::buyer::live_on(c.w, c.now);
    let series = p.terms.series.as_ref().map(|_| crate::buyer::series_prev(c.data, buyer, buyer, &p, &live));
    if let Some(r) = &series {
        let held = r.as_ref().err().and_then(crate::buyer::series_held);
        steps.push((PL_STEP_SERIES, held.map_or(State::Pass, |why| State::Fail(crate::buyer::E_SERIES_SAME.into(), why))));
    }
    let mut notes: Vec<Note> = Vec::new();
    let read = (h.candidates)(c, &p);
    for (n, r) in &read {
        if let Err(f) = r {
            notes.push((*n, f.0.clone(), f.1.clone(), true));
        }
    }
    steps.push((PL_STEP_JUDGES, all_hidden(&p, &notes, "E_P3_NO_CANDIDATE", h.words.all_hidden_docs)));
    let mut quotes: Vec<Option<k2::quote::Quote>> = Vec::new();
    let mut spelled: Vec<Vec<u8>> = Vec::new();
    for (n, r) in read {
        let q = r.as_ref().ok().map(|b| k2::quote::decode(b));
        if let Some(Err(f)) = &q {
            notes.push((n, "E_P3_QUOTE".into(), format!("报价读不成:{:?}", f.token), true));
        }
        quotes.push(q.and_then(Result::ok));
        spelled.push(r.unwrap_or_default());
    }
    steps.push((PL_STEP_QUOTE, all_hidden(&p, &notes, "E_P3_QUOTE", h.words.all_hidden_docs)));
    steps.push((PL_STEP_STANDARD, State::of(&(h.standard)(c, &p))));
    let (price, held) = (h.price, std::sync::Arc::new(PayHeld::of(c)));
    for (n, got) in eravon_chain::read::side_by_side(quotes.iter().cloned().enumerate().collect::<Vec<_>>(), move |(n, q)| (n, q.as_ref().map(|q| price(&held.chain(), q)))) {
        if let Some(Err(f)) = got {
            notes.push((n, f.0.clone(), f.1.clone(), true));
        }
    }
    steps.push((PL_STEP_PRICE, all_hidden(&p, &notes, "E_P3_PRICE_STALE", h.words.price_stale)));
    for (n, ok, note) in consent(c, &p) {
        if !ok {
            notes.push((n, String::new(), note, false));
        }
    }
    steps.push((PL_STEP_CONSENT, all_hidden(&p, &notes, "E_P3_NO_CANDIDATE", h.words.all_hidden_unregistered)));
    for (n, ok, note) in standing.may_open {
        if !ok {
            notes.push((n, "E_P3_OFFER_RETIRED".into(), note, false));
        }
    }
    steps.push((PL_STEP_CURRENT, all_hidden(&p, &notes, "E_P3_OFFER_RETIRED", h.words.all_hidden_retired)));
    let mut short: Vec<(usize, String, String)> = Vec::new();
    let mut acted: Vec<Note> = Vec::new();
    let open: Vec<usize> = (0..p.candidate_count()).filter(|n| !notes.iter().any(|(m, ..)| m == n)).collect();
    let held = std::sync::Arc::new((sw.held(), p.terms_text.clone(), buyer.to_string()));
    let gated = eravon_chain::read::side_by_side(open.clone(), move |n| {
        crate::shared::order_of(held.1.as_bytes(), n as u64, &held.2, &held.2, 0, eravon_standard::order::NO_PREV).ok().map(|ord| crate::buyer::pay_gate(&held.0.wire(), &ord))
    });
    for (&n, gate) in open.iter().zip(gated) {
        let Some(gate) = gate else { continue };
        match act_note(n, &gate) {
            Some((note, true)) => {
                short.push((n, note.1.clone(), note.2.split_once(':').map_or(String::new(), |(_, w)| w.to_string())));
                notes.push(note);
            }
            Some((note, false)) => acted.push(note),
            None => {}
        }
    }
    steps.push(deposit_step(&open, &notes, h));
    let rest: Vec<usize> = open.iter().copied().filter(|n| !short.iter().any(|(m, ..)| m == n)).collect();
    notes.extend(acted);
    steps.push(act_step(&rest, &notes, h));
    let mut o = page(&p, link, steps, buyer, Some((&notes, &quotes)), h);
    o.deposit_short = short;
    if o.steps.iter().any(|(n, s)| *n == PL_STEP_SERIES && matches!(s, State::Fail(..))) {
        o.series_held = crate::buyer::series_held_order(c.data, buyer, &p, &live);
    }
    let held = std::sync::Arc::new((PayHeld::of(c), p.clone(), spelled.clone()));
    let priced = eravon_chain::read::side_by_side(o.cands.iter().filter(|x| x.priced_at_open && x.choosable).map(|x| x.n).collect::<Vec<_>>(), move |n| fees(&held.0.chain(), &held.1, n, &held.2[n]));
    for (x, fees) in o.cands.iter_mut().filter(|x| x.priced_at_open && x.choosable).zip(priced) {
        match fees {
            Ok(f) => {
                x.lf = f.lf.to_dec();
                x.deposit = f.deposit.to_dec();
            }
            Err(f) => x.priced_why = f,
        }
    }
    let addrs: Vec<String> = o.cands.iter().map(|x| x.address.to_ascii_lowercase()).collect();
    if !addrs.is_empty() {
        let logs = eravon_cands::registry::logs_of(c.cw, &addrs);
        let guilds: Vec<String> = o.cands.iter().filter(|x| x.kind == "guild").map(|x| x.address.to_ascii_lowercase()).collect();
        let held = std::sync::Arc::new((logs.clone(), c.cw.clone()));
        let mut rosters = eravon_chain::read::side_by_side(guilds, move |at| held.0.as_ref().map_err(|e| format!("{} · {}", e.name(), e.detail())).and_then(|l| eravon_cands::guild::open(&held.1, l, &at).map_err(|e| format!("{} · {}", e.name(), e.detail())))).into_iter();
        for x in o.cands.iter_mut() {
            let at = x.address.to_ascii_lowercase();
            if x.kind == "guild" {
                let Some(view) = rosters.next() else { continue };
                x.name = view.as_ref().ok().and_then(|v| v.name.clone()).unwrap_or_default();
                x.covenant = Some(view.and_then(|v| v.terms.map(|t| t.text).unwrap_or_else(|| Err("E_POOL_NO_GUILD_TERMS".into()))));
            } else if let Ok(l) = &logs {
                x.name = eravon_cands::judge::name_of(l, &at).unwrap_or_default();
            }
        }
    }
    if p.terms.series.as_ref().is_some_and(|s| s.stage > 1) {
        let prev = series.and_then(Result::ok);
        o.stage_waits = prev.zip(o.cands.iter().find(|x| x.choosable)).is_some_and(|(prev, x)| {
            crate::shared::order_of(p.terms_text.as_bytes(), x.n as u64, buyer, buyer, 0, &prev)
                .ok()
                .and_then(|ord| act(c, &ord).err())
                .is_some_and(|r| crate::buyer::stage_gate(&r))
        });
    }
    match balance(c, &p.token) {
        Ok(b) => {
            o.balance = Some(b.have.to_dec());
            o.decimals = b.decimals;
            o.gap = (h.gap)(&p.order.amount, &b.have).map(|g| g.to_dec());
        }
        Err(f) => o.steps.push(("balance", State::Fail(f.0, f.1))),
    }
    Ok(o)
}

pub type Note = (usize, String, String, bool);

pub fn joined(name: &str, detail: &str) -> String {
    if name.is_empty() && detail.is_empty() { String::new() } else { format!("{name} · {detail}") }
}

pub fn all_hidden(p: &k3::listing::Listing, notes: &[Note], token: &str, why: &str) -> State {
    hidden_of(p.candidate_count(), notes, token, why)
}

pub fn deposit_step(open: &[usize], notes: &[Note], h: &PayHooks) -> (&'static str, State) {
    let state = if open.is_empty() { State::Pass } else { hidden_among(open, notes, "E_P3_NO_CANDIDATE", h.words.deposit_short) };
    (PL_STEP_DEPOSIT, state)
}

pub fn act_note(n: usize, gate: &crate::buyer::Gate) -> Option<(Note, bool)> {
    let r = gate.refusal().filter(|_| !gate.pays())?;
    Some(((n, r.name.clone(), format!("{}:{}", r.gate, r.why), true), matches!(gate, crate::buyer::Gate::DepositShort(_))))
}

pub fn act_step(rest: &[usize], notes: &[Note], h: &PayHooks) -> (&'static str, State) {
    let state = if rest.is_empty() { State::Pass } else { hidden_among(rest, notes, "E_P3_NO_CANDIDATE", h.words.all_hidden_act) };
    (PL_STEP_ACT, state)
}

pub fn hidden_of(count: usize, notes: &[Note], token: &str, why: &str) -> State {
    hidden_among(&(0..count).collect::<Vec<usize>>(), notes, token, why)
}

fn hidden_among(among: &[usize], notes: &[Note], token: &str, why: &str) -> State {
    let firsts: Vec<&Note> = among.iter().filter_map(|n| notes.iter().find(|(m, ..)| m == n)).collect();
    if firsts.len() < among.len() {
        return State::Pass;
    }
    match firsts.first() {
        Some((_, name, detail, _)) if !name.is_empty() && firsts.iter().all(|(_, x, ..)| x == name) => State::Fail(name.clone(), detail.clone()),
        _ => State::Fail(token.into(), why.into()),
    }
}

pub fn page(p: &k3::listing::Listing, link: &[u8], steps: Vec<(&'static str, State)>, buyer: &str, chain: Option<(&[Note], &[Option<k2::quote::Quote>])>, h: &PayHooks) -> Opened {
    let mut list = Vec::new();
    if let Some((notes, quotes)) = chain {
        let parties = [p.order.seller_key.to_ascii_lowercase(), buyer.to_ascii_lowercase()];
        let by_call: Vec<usize> = p.by_call.iter().map(|(n, _)| *n).collect();
        let rows = p.judges.iter().map(|j| ("judge", j.judge.clone(), j.amanah_hash.clone()))
            .chain(p.guilds.iter().map(|g| ("guild", g.address.clone(), g.amanah_hash.clone())));
        for (n, (kind, address, amanah)) in rows.enumerate() {
            let q = quotes.get(n).cloned().flatten();
            let fee = q.as_ref().and_then(|q| k2::fee::fee(q, &p.order.amount).ok());
            let (ok_gate, gate_name, note) = (h.gate)(q.as_ref(), &p.order.amount, &address, &parties, kind);
            let chain_note = notes.iter().find(|(m, ..)| *m == n).map(|(_, t, s, j)| (t.clone(), s.clone(), *j));
            let (refusal, note, joined) = if !ok_gate && !note.is_empty() { (gate_name.to_string(), note, false) } else { chain_note.clone().unwrap_or_default() };
            list.push(Cand {
                n,
                kind,
                address,
                amanah,
                ruling_window: q.as_ref().map(|q| q.ruling_window).unwrap_or_default(),
                premium: fee.map(|f| f.premium.to_dec()).unwrap_or_default(),
                lf: fee.and_then(|f| f.lf).map(|x| x.to_dec()).unwrap_or_default(),
                deposit: fee.and_then(|f| f.lf.map(|lf| f.premium.overflowing_add(&lf).0)).map(|d| d.to_dec()).unwrap_or_default(),
                priced_at_open: q.as_ref().is_some_and(|q| !q.fee_unit_is_zero()),
                priced_why: (String::new(), String::new()),
                by_call: by_call.contains(&n),
                choosable: ok_gate && chain_note.is_none(),
                refusal,
                note,
                joined,
                covenant: None,
                name: String::new(),
            });
        }
    }
    let title = match eravon_goods::terms::goods_of(&p.terms.block, None) {
        Some(text) => eravon_goods::terms::name_of(&text),
        None => h.words.private_goods.to_string(),
    };
    Opened {
        steps,
        title,
        amount: p.order.amount.to_dec(),
        token: p.token.clone(),
        seller_key: p.order.seller_key.clone(),
        t1: p.order.delivery_window,
        t2: p.order.confirm_window,
        rungs: p.order.ladder.iter().map(|r| (r.buyer_bps, r.seller_bps, r.fee_bearer)).collect(),
        terms_text: Some(p.terms_text.clone()),
        expiry: p.order.expiry,
        cands: list,
        quotes: chain.map(|(_, q)| q.to_vec()).unwrap_or_default(),
        chain_id: p.chain_id,
        logic: p.logic.clone(),
        open_window: p.order.open_window,
        link: link.to_vec(),
        listing: p.bytes.clone(),
        source: buyer.to_ascii_lowercase(),
        segments: [("goods", "goodsHash"), ("delivery", "deliveryHash")]
            .into_iter()
            .filter_map(|(seg, key)| p.terms.block.get(key).and_then(eravon_galeed::Value::as_str).map(|h| (seg, h.to_string())))
            .collect(),
        balance: None,
        decimals: None,
        gap: None,
        deposit_short: Vec::new(),
        stage_waits: false,
        series_held: None,
    }
}

pub fn gate(q: Option<&k2::quote::Quote>, amount: &k2::u256::U256, who: &str, parties: &[String], kind: &str) -> (bool, &'static str, String) {
    if kind == "judge" && eravon_cands::act::judge_party(who, &parties.iter().map(String::as_str).collect::<Vec<_>>()) {
        return (false, E_PL_JUDGE_PARTY, PL_PARTY.to_string());
    }
    let Some(q) = q else { return (false, "", String::new()) };
    if !q.amount_cap.is_zero() && amount.cmp_u(&q.amount_cap) == std::cmp::Ordering::Greater {
        return (false, E_PL_OVER_CAP, PL_OVER_CAP.to_string());
    }
    (true, "", String::new())
}

pub const E_PL_JUDGE_PARTY: &str = "E_P3_JUDGE_PARTY";
pub const E_PL_OVER_CAP: &str = "E_P3_OVER_CAP";

pub fn opened_rows(o: &Opened) -> Out {
    let steps = o
        .steps
        .iter()
        .map(|(name, s)| {
            let mut r = vec![("name", Out::S((*name).into())), ("state", Out::S(s.word().into()))];
            if let State::Fail(t, d) = s {
                r.push(("token", Out::S(t.clone())));
                r.push(("why", Out::S(d.clone())));
            }
            r.sort_by(|a, b| a.0.cmp(b.0));
            Out::O(r)
        })
        .collect();
    let cands = o
        .cands
        .iter()
        .map(|c| {
            Out::O(vec![
                ("address", Out::S(c.address.clone())),
                ("amanahHash", Out::S(c.amanah.clone())),
                ("byCall", Out::B(c.by_call)),
                ("choosable", Out::B(c.choosable)),
                ("deposit", Out::S(c.deposit.clone())),
                ("kind", Out::S(c.kind.into())),
                ("lf", Out::S(c.lf.clone())),
                ("n", Out::N(c.n as u64)),
                ("note", Out::S(c.note_said())),
                ("refusal", Out::S(c.refusal.clone())),
                ("premium", Out::S(c.premium.clone())),
                ("pricedAtOpen", Out::B(c.priced_at_open)),
                ("pricedWhy", Out::S(joined(&c.priced_why.0, &c.priced_why.1))),
                ("rulingWindow", Out::N(c.ruling_window)),
            ])
        })
        .collect();
    let mut m = vec![
        ("amount", Out::S(o.amount.clone())),
        ("candidates", Out::A(cands)),
        ("checks", Out::A(steps)),
        ("depositShort", Out::A(o.deposit_short.iter().map(|(n, name, why)| Out::O(vec![("candidate", Out::N(*n as u64)), ("why", Out::S(joined(name, why)))])).collect())),
        ("expiry", Out::N(o.expiry)),
        ("rungs", Out::A(o.rungs.iter().map(|(b, s, f)| Out::O(vec![("buyerBps", Out::N(*b)), ("feeBearer", Out::N(*f)), ("sellerBps", Out::N(*s))])).collect())),
        ("sellerKey", Out::S(o.seller_key.clone())),
        ("source", Out::S(o.source.clone())),
        ("t1", Out::N(o.t1)),
        ("t2", Out::N(o.t2)),
        ("title", Out::S(o.title.clone())),
        ("token", Out::S(o.token.clone())),
    ];
    if let Some(b) = &o.balance {
        m.push(("balance", Out::S(b.clone())));
    }
    if let Some(d) = o.decimals {
        m.push(("decimals", Out::N(d)));
    }
    if let Some(g) = &o.gap {
        m.push(("gap", Out::S(g.clone())));
    }
    if let Some(t) = &o.terms_text {
        m.push(("termsText", Out::S(t.clone())));
    }
    m.sort_by(|a, b| a.0.cmp(b.0));
    Out::O(m)
}


#[cfg(test)]
mod page_tests {
    use super::*;

    #[test]
    fn a_candidate_row_says_it_is_priced_at_opening() {
        let cand = Cand { n: 0, kind: "judge", address: "0x00000000000000000000000000000000000000f1".into(), amanah: String::new(), ruling_window: 0, premium: "1".into(), lf: String::new(), deposit: String::new(), priced_at_open: true, priced_why: (String::new(), String::new()), by_call: false, choosable: true, refusal: String::new(), note: String::new(), joined: false, covenant: None, name: String::new() };
        let o = Opened { steps: Vec::new(), title: String::new(), amount: "10".into(), token: String::new(), seller_key: String::new(), t1: 0, t2: 0, rungs: Vec::new(), terms_text: None, expiry: 0, cands: vec![cand], quotes: Vec::new(), chain_id: 1, logic: String::new(), open_window: 0, link: Vec::new(), listing: Vec::new(), source: String::new(), balance: None, decimals: None, gap: None, segments: Vec::new(), deposit_short: Vec::new(), stage_waits: false, series_held: None };
        let Out::O(m) = opened_rows(&o) else { panic!("页答面是一个对象") };
        let Some((_, Out::A(cands))) = m.iter().find(|(k, _)| *k == "candidates") else { panic!("有 candidates") };
        let Out::O(c) = &cands[0] else { panic!("候选行是对象") };
        assert!(c.iter().any(|(k, v)| *k == "pricedAtOpen" && matches!(v, Out::B(true))));
        assert!(c.iter().any(|(k, v)| *k == "choosable" && matches!(v, Out::B(true))));
    }
}
