use std::sync::Arc;

use eravon_chain as a2;
use eravon_chain::endpoints::Table;
use eravon_chain::recall::{self, Batched, Recall, Store, Want};
use eravon_chain::transport::Auto;
use eravon_literals::constants as k7;
use eravon_scan::scan::Side;
use eravon_standard::multi;
use eravon_terms::u256::U256;

use crate::house::book::{Balances, Book, Case, Coin, Head, Meta, State};
use crate::house::seller::Said;
use crate::house::shell::Seat;
use crate::house::sync::Tick;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Start,
    Unlock,
    Key,
    Switch,
    Endpoints,
    Landed,
    Period,
    Page,
}

impl Cause {
    pub const fn name(self) -> &'static str {
        match self {
            Cause::Start => "start",
            Cause::Unlock => "unlock",
            Cause::Key => "key",
            Cause::Switch => "switch",
            Cause::Endpoints => "endpoints",
            Cause::Landed => "landed",
            Cause::Period => "period",
            Cause::Page => "page",
        }
    }

    pub const ALL: [Cause; 8] = [Cause::Start, Cause::Unlock, Cause::Key, Cause::Switch, Cause::Endpoints, Cause::Landed, Cause::Period, Cause::Page];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    Seat,
    Order,
}

impl crate::reads::Cause for Cause {
    type Reading = Reading;
    const ALL: &'static [Cause] = &Cause::ALL;
    fn stales(self) -> &'static [Reading] {
        match self {
            Cause::Landed => &[Reading::Order],
            Cause::Start | Cause::Unlock | Cause::Key | Cause::Switch | Cause::Endpoints | Cause::Period | Cause::Page => &[Reading::Seat],
        }
    }
}

pub struct Plan {
    pub data: String,
    pub me: String,
    pub seat: Seat,
    pub logic: String,
    pub only: Option<String>,
    pub cause: Cause,
    pub now: u64,
    session: eravon_vault::session::Session,
    ticket: crate::house::Ticket,
}

fn landed_only(only: Option<String>) -> Option<String> {
    only.map(|x| x.to_ascii_lowercase())
}


#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
pub fn plan(s: &eravon_vault::session::Session, data: &str, me: &str, seat: Seat, cause: Cause, only: Option<String>, now: u64, ticket: crate::house::Ticket) -> Result<Plan, Said> {
    eravon_vault::session::need_open(s).map_err(|e| crate::house::said_of(&e))?;
    let session = s.fork().map_err(|e| crate::house::said_of(&e))?;
    Ok(Plan {
        data: data.to_string(),
        me: me.to_ascii_lowercase(),
        seat,
        logic: eravon_standard::chain::standard_table().1.to_ascii_lowercase(),
        only: if crate::reads::Cause::stales(cause).contains(&Reading::Seat) { None } else { landed_only(only) },
        cause,
        now,
        session,
        ticket,
    })
}

#[derive(Clone, Debug)]
pub struct Pass {
    pub cause: Cause,
    pub book: Option<Book>,
    pub tick: Option<Tick>,
    pub failed: Option<a2::fault::Fault>,
    pub stored: bool,
    pub filled: usize,
    pub from: Option<u64>,
    pub only: Option<String>,
    pub ms: u64,
}

pub fn due(last: Option<u64>, now: u64, every: u64) -> bool {
    every > 0 && last.is_some_and(|t| now >= t.saturating_add(every) || now < t)
}


pub fn endpoints_of(table: &Table) -> String {
    table.list.join("\n")
}

pub fn scan_from(cursor: Option<u64>) -> u64 {
    cursor.map_or(0, |c| c.saturating_sub(k7::SCAN_BACKOFF_BLOCKS))
}


fn columns(v: &[eravon_standard::abi::Val]) -> Vec<String> {
    use eravon_standard::abi::Val;
    v.iter()
        .map(|x| match x {
            Val::U256(w) => U256::from_be(w).to_dec(),
            Val::B32(w) => eravon_standard::order::hex_of(w),
            Val::Bool(b) => b.to_string(),
            Val::Addr(a) => eravon_standard::order::hex_of(a),
            _ => String::new(),
        })
        .collect()
}

fn back(name: &str, detail: String) -> a2::fault::Fault {
    use a2::fault::Fault as F;
    match crate::house::sync::chain_class(name) {
        Some(F::Transport(_)) => F::Transport(detail),
        Some(F::Execution(_)) => F::Execution(detail),
        Some(F::Disagree(_)) => F::Disagree(detail),
        Some(F::WrongChain(_)) => F::WrongChain(detail),
        Some(F::Usage(_)) => F::Usage(detail),
        Some(F::Endpoint(_)) => F::Endpoint(detail),
        Some(F::Unread(_)) => F::Unread(detail),
        Some(F::Echo(_)) => F::Echo(detail),
        Some(F::OverAsks(_)) => F::OverAsks(detail),
        None => F::Endpoint(format!("{name}:{detail}")),
    }
}

fn fault_of(e: eravon_standard::fault::Fault) -> a2::fault::Fault {
    back(&e.name(), e.detail())
}

fn tokens_of(data: &str) -> Vec<String> {
    eravon_vault::tokens::on_chain(&eravon_vault::tokens::list(data), eravon_vault::tokens::chain()).into_iter().map(|r| r.address.to_ascii_lowercase()).collect()
}

fn keys_of(p: &Plan) -> Result<(Side, Vec<String>), a2::fault::Fault> {
    let refused = |e: eravon_vault::vfault::Fault| a2::fault::Fault::Usage(format!("{}:{}", e.name(), e.detail()));
    match p.seat {
        Seat::Seller => eravon_vault::keyset::seller(&p.data, &p.me).map(|k| (Side::Seller, k)).map_err(refused),
        Seat::Buyer => eravon_vault::keyset::buyer(&p.session, &p.me).map(|b| (Side::Buyer, b.keys)).map_err(refused),
    }
}

pub fn shown(book: Option<&Book>) -> Arc<Recall> {
    Recall::over(book.map_or_else(|| Arc::new(Store::default()), |b| b.kept.clone()))
}

pub fn shown_on(sh: &crate::house::shell::Shell, key: &str) -> Arc<Recall> {
    let book = sh.book.as_ref().filter(|b| b.key.eq_ignore_ascii_case(key));
    Recall::over_failed(book.map_or_else(|| Arc::new(Store::default()), |b| b.kept.clone()), sh.read_failed.clone())
}

pub fn draw<X>(r: &Arc<Recall>, f: impl FnOnce() -> X) -> X {
    recall::within(r, f)
}


#[derive(Clone, Debug, Default)]
pub struct Follow(Option<Base>);

#[derive(Clone, Debug)]
struct Base {
    id: String,
    shape: String,
    whole: bool,
}

impl Follow {
    pub fn moved(&mut self, full: bool, before: Option<(String, Option<String>)>, now: Option<(String, Option<String>)>) -> bool {
        let Some((id, shape)) = now else {
            self.0 = None;
            return false;
        };
        if self.0.as_ref().is_some_and(|b| !b.id.eq_ignore_ascii_case(&id)) {
            self.0 = None;
        }
        if let Some((_, Some(s))) = before.filter(|(b, _)| b.eq_ignore_ascii_case(&id)) {
            self.keep(&id, s);
        }
        if !full {
            if let Some(s) = shape {
                self.keep(&id, s);
            }
            return false;
        }
        let moved = match (&self.0, &shape) {
            (Some(b), Some(s)) => &b.shape != s,
            _ => false,
        };
        if let Some(s) = shape {
            self.0 = Some(Base { id, shape: s, whole: true });
        }
        moved
    }

    fn keep(&mut self, id: &str, shape: String) {
        if !self.0.as_ref().is_some_and(|b| b.whole) {
            self.0 = Some(Base { id: id.to_string(), shape, whole: false });
        }
    }
}

pub fn aged(book: Option<&Book>, now: u64) -> bool {
    book.and_then(Book::read_at).is_none_or(|at| due(Some(at), now, k7::READ_REFRESH_SECS))
}

fn quantity_of(w: &[u8]) -> String {
    let hex: String = w.iter().map(|b| format!("{b:02x}")).collect();
    let t = hex.trim_start_matches('0');
    format!("0x{}", if t.is_empty() { "0" } else { t })
}

fn about(b: &Batched, instance: &str) -> bool {
    let tail = instance.trim_start_matches("0x").to_ascii_lowercase();
    match b {
        Batched::Call(to, data) => to.eq_ignore_ascii_case(instance) || data.to_ascii_lowercase().contains(&tail),
        Batched::Balance(who) => who.eq_ignore_ascii_case(instance),
    }
}

fn ask_of(b: &Batched) -> Result<multi::Ask, a2::fault::Fault> {
    match b {
        Batched::Call(to, data) => Ok(multi::Ask { to: to.clone(), data: eravon_standard::order::bytes_of("calldata", data).map_err(fault_of)? }),
        Batched::Balance(who) => multi::eth_balance(who).map_err(fault_of),
    }
}

fn kept_of(b: &Batched, got: Option<Vec<u8>>, agreement: &str, endpoints: usize) -> recall::Kept {
    let agreement = if agreement == "single" { a2::read::Agreement::Single } else { a2::read::Agreement::Agreed };
    let value = match (b, got) {
        (Batched::Call(..), Some(bytes)) => Ok(format!("\"{}\"", eravon_standard::order::hex_of(&bytes))),
        (Batched::Balance(_), Some(bytes)) => Ok(format!("\"{}\"", quantity_of(multi::word(&bytes).as_ref().map_or(&[][..], |w| &w[..])))),
        (_, None) => Err(recall::Refused::Reverted("码 3:execution reverted(Multicall3 里这一件回滚了)".to_string())),
    };
    recall::Kept { value, agreement, endpoints }
}

fn batch(t: &Auto, table: &Table, store: &mut Store, h: u64, only: Option<&str>) -> Result<(u64, &'static str, usize), a2::fault::Fault> {
    let keep = |b: &Batched| only.is_none_or(|i| about(b, i));
    let reads: Vec<(String, Batched)> = store.batched().into_iter().filter(|(_, b)| keep(b)).collect();
    let probes: Vec<(String, String)> = store.probes().into_iter().filter(|(_, a)| only.is_none_or(|i| a.eq_ignore_ascii_case(i))).collect();
    let at = a2::seam::quantity(h);
    let mut asks = vec![multi::timestamp().map_err(fault_of)?];
    for (_, b) in &reads {
        asks.push(ask_of(b)?);
    }
    for (_, a) in &probes {
        asks.push(multi::status(a).map_err(fault_of)?);
    }
    if let Some(r) = multi::read(t, table, &asks, &at).map_err(fault_of)? {
        let mut it = r.value.into_iter();
        let time = it.next().flatten().and_then(|b| multi::word(&b)).map(|w| U256::from_be(&w)).and_then(|x| x.to_dec().parse::<u64>().ok()).ok_or_else(|| a2::fault::Fault::Endpoint("Multicall3 没有答出这一块的链时".into()))?;
        for (k, b) in &reads {
            let got = it.next().flatten();
            store.put(k.clone(), kept_of(b, got, r.agreement, r.endpoints));
        }
        for (k, _) in &probes {
            if !matches!(it.next(), Some(Some(b)) if b.is_empty()) {
                store.forget(k);
            }
        }
        return Ok((time, r.agreement, r.endpoints));
    }
    let r = a2::read::read_agreed(t, table, a2::seam::M_GET_BLOCK, vec![eravon_literals::json::Out::S(at), eravon_literals::json::Out::B(false)])?;
    let time = r.value.get("timestamp").and_then(|x| x.as_str()).map(a2::seam::read_quantity).transpose()?.unwrap_or(0);
    let keys: Vec<String> = reads.iter().map(|(k, _)| k.clone()).chain(probes.iter().map(|(k, _)| k.clone())).collect();
    ask_ahead(t, table, keys.iter().filter_map(|k| k.split_once(' ')));
    for k in keys {
        if let Some((m, p)) = k.split_once(' ') {
            store.fetch(t, table, &Want::Read(m.to_string(), p.to_string()))?;
        }
    }
    Ok((time, r.agreement.word(), r.endpoints))
}

fn ends_round(e: &a2::fault::Fault) -> bool {
    matches!(e, a2::fault::Fault::Transport(_) | a2::fault::Fault::WrongChain(_) | a2::fault::Fault::Usage(_))
}

fn fetch_all(t: &Auto, table: &Table, store: &mut Store, wanted: &[Want]) -> Result<Option<u64>, a2::fault::Fault> {
    let mut batched: Vec<(String, Batched)> = Vec::new();
    let mut from: Option<u64> = None;
    let mut refused: Option<a2::fault::Fault> = None;
    let mut settle = |got: Result<(), a2::fault::Fault>| -> Result<(), a2::fault::Fault> {
        match got {
            Err(e) if ends_round(&e) => Err(e),
            Err(e) => {
                refused.get_or_insert(e);
                Ok(())
            }
            Ok(()) => Ok(()),
        }
    };
    ask_ahead(t, table, wanted.iter().filter_map(|w| match w {
        Want::Read(m, p) if recall::batched_of(&format!("{m} {p}")).is_none() => Some((m.as_str(), p.as_str())),
        _ => None,
    }));
    let logs: Vec<(String, u64)> = wanted.iter().filter_map(|w| if let Want::Logs(rest, f) = w { Some((rest.clone(), *f)) } else { None }).collect();
    let mut logs = store.read_logs(t, table, &logs).into_iter();
    for w in wanted {
        match w {
            Want::Read(m, p) => match recall::batched_of(&format!("{m} {p}")) {
                Some(b) => batched.push((format!("{m} {p}"), b)),
                None => settle(store.fetch(t, table, w))?,
            },
            Want::Logs(rest, f) => {
                from = Some(from.map_or(*f, |x| x.min(*f)));
                let got = logs.next().unwrap_or_else(|| Err(a2::fault::Fault::Usage("日志欠读少答了一问".into())));
                settle(store.keep_logs(rest, *f, got))?;
            }
            Want::Trace(..) => settle(store.fetch(t, table, w))?,
        }
    }
    if batched.is_empty() {
        return refused.map_or(Ok(from), Err);
    }
    let asks = batched.iter().map(|(_, b)| ask_of(b)).collect::<Result<Vec<_>, _>>()?;
    match multi::read(t, table, &asks, &a2::seam::quantity(store.head)).map_err(fault_of)? {
        Some(r) => {
            for ((k, b), got) in batched.iter().zip(r.value) {
                store.put(k.clone(), kept_of(b, got, r.agreement, r.endpoints));
            }
        }
        None => {
            ask_ahead(t, table, batched.iter().filter_map(|(k, _)| k.split_once(' ')));
            for (k, _) in &batched {
                if let Some((m, p)) = k.split_once(' ') {
                    settle(store.fetch(t, table, &Want::Read(m.to_string(), p.to_string())))?;
                }
            }
        }
    }
    refused.map_or(Ok(from), Err)
}

fn ask_ahead<'a>(t: &Auto, table: &Table, reads: impl Iterator<Item = (&'a str, &'a str)>) {
    let mut by: Vec<(&'static str, Vec<String>)> = Vec::new();
    for (m, p) in reads {
        let Some(m) = recall::method_of(m) else { continue };
        match by.iter_mut().find(|(x, _)| *x == m) {
            Some((_, ps)) => ps.push(p.to_string()),
            None => by.push((m, vec![p.to_string()])),
        }
    }
    for (m, ps) in by {
        a2::read::ask_ahead_raw(t, table, m, &ps);
    }
}

struct Summary {
    cases: Vec<Case>,
    foreign: u64,
    scan: &'static str,
    balances: Option<(Option<String>, Vec<Coin>, &'static str)>,
    states: Vec<(String, bool, Vec<String>, &'static str)>,
}

fn summary(t: &Auto, table: &Table, p: &Plan, side: Side, keys: &[String]) -> Result<Summary, a2::fault::Fault> {
    let sw = eravon_scan::scan::by_keys(t, table, &p.logic, side, keys, 0, a2::seam::Upto::Latest, &[]).map_err(|e| back(&e.name(), e.detail()))?;
    let cases: Vec<Case> = sw.cases.iter().map(|c| Case { case_id: c.case_id.clone(), instance: c.instance.to_ascii_lowercase(), seller: c.seller.clone(), buyer: c.buyer.clone(), block: c.block, tx: c.tx.clone() }).collect();
    let tokens = tokens_of(&p.data);
    let mut asks = vec![multi::eth_balance(&p.me).map_err(fault_of)?];
    for tok in &tokens {
        asks.push(multi::erc20_balance(tok, &p.me).map_err(fault_of)?);
    }
    for c in &cases {
        asks.push(multi::status(&c.instance).map_err(fault_of)?);
    }
    let dec = |b: &[u8]| multi::word(b).map(|w| U256::from_be(&w).to_dec());
    let mut refused: Option<a2::fault::Fault> = None;
    let got: Vec<(Option<Vec<u8>>, &'static str)> = asks
        .iter()
        .map(|a| match a2::read::call(t, table, &a.to, &eravon_standard::order::hex_of(&a.data), a2::seam::TAG_LATEST) {
            Ok((hex, r)) => (eravon_standard::order::bytes_of("答", &hex).ok(), r.agreement.word()),
            Err(e) if a2::fault::reverted(&e) => (None, "agreed"),
            Err(e) => {
                refused.get_or_insert(e);
                (None, "agreed")
            }
        })
        .collect();
    if let Some(e) = refused {
        return Err(e);
    }
    let mut it = got.into_iter();
    let (native, w0) = it.next().map(|(b, w)| (b.and_then(|b| dec(&b)), w)).unwrap_or((None, "agreed"));
    let mut word = w0;
    let coins = tokens
        .iter()
        .map(|tok| {
            let (b, w) = it.next().unwrap_or((None, "agreed"));
            if w == "single" {
                word = "single";
            }
            Coin { token: tok.clone(), have: b.and_then(|b| dec(&b)) }
        })
        .collect();
    let states = cases
        .iter()
        .map(|c| {
            let (b, w) = it.next().unwrap_or((None, "agreed"));
            let code = b.as_ref().is_none_or(|b| !b.is_empty());
            let cols = b.filter(|b| !b.is_empty()).and_then(|b| eravon_standard::chain::status_of(&b).ok()).map(|v| columns(&v)).unwrap_or_default();
            (c.instance.clone(), code, cols, w)
        })
        .collect();
    Ok(Summary { cases, foreign: sw.foreign, scan: sw.agreement, balances: Some((native, coins, word)), states })
}

fn build(t: &Auto, table: &Table, p: &Plan, side: Side, keys: &[String]) -> Result<Summary, a2::fault::Fault> {
    match p.seat {
        Seat::Seller => {
            let _ = crate::house::home::Home::open(&p.data, &p.me);
            let _ = crate::house::orders::Orders::table_only(&p.data, &p.me);
            let _ = crate::house::export::Export::open(&p.session, &p.data, &p.me, false);
            let _ = crate::house::settings::gas_of(&p.data, &p.me);
            let mut w = crate::house::wallet::Wallet::open(&p.session, &p.data, &p.me, true);
            w.read_vault(&p.data, &p.me);
        }
        Seat::Buyer => {
            let _ = crate::house::buyer_home::BHome::open(&p.session, &p.data, &p.me);
            let _ = crate::house::wallet::Wallet::open(&p.session, &p.data, &p.me, false);
            let _ = crate::house::export::Export::open(&p.session, &p.data, &p.me, true);
        }
    }
    summary(t, table, p, side, keys)
}

const ROUNDS: usize = 512;

fn kept_fail(mut out: Pass, _p: &Plan, mut book: Book, store: &Store, e: a2::fault::Fault) -> Pass {
    book.kept = Arc::new(store.clone());
    out.book = Some(book);
    out.failed = Some(e);
    out
}

pub fn run(table: &Table, p: Plan, prior: Option<Book>) -> Pass {
    let began = std::time::Instant::now();
    let _ = eravon_vault::tokens::look();
    let table = asked(table);
    let floor = || eravon_chain::read::floor_of(&table);
    let pass = stand(run_in(&table, &p, prior), |x| x.tick.as_ref().map(|t| t.block), floor, |x| run_in(&table, &p, x.book.clone()));
    Pass { ms: u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX), ..pass }
}

pub const STAND_AGAIN: usize = 2;

fn stand<P>(mut pass: P, stood: impl Fn(&P) -> Option<u64>, floor: impl Fn() -> Option<u64>, mut again: impl FnMut(&P) -> P) -> P {
    for _ in 0..STAND_AGAIN {
        if !floor().is_some_and(|f| stood(&pass).is_some_and(|h| f > h)) {
            break;
        }
        pass = again(&pass);
    }
    pass
}

#[cfg(test)]
mod stand_tests {
    use super::*;

    #[test]
    fn a_pass_below_the_receipt_it_read_is_made_again_at_most_twice() {
        let made = |heads: Vec<Option<u64>>, floor: Option<u64>| -> (Option<u64>, usize) {
            let mut n = 0;
            let mut rest = heads.into_iter();
            let first = rest.next().unwrap();
            let got = stand(first, |h| *h, || floor, |_| {
                n += 1;
                rest.next().unwrap_or(Some(0))
            });
            (got, n)
        };
        assert_eq!(made(vec![Some(9), Some(10)], Some(10)), (Some(10), 1), "reached on the next head");
        assert_eq!(made(vec![Some(8), Some(9), Some(9), Some(10)], Some(10)), (Some(9), STAND_AGAIN), "at most twice more");
        assert_eq!(made(vec![Some(10)], Some(10)), (Some(10), 0), "at the receipt");
        assert_eq!(made(vec![Some(12)], Some(10)), (Some(12), 0), "above it");
        assert_eq!(made(vec![Some(3)], None), (Some(3), 0), "no receipt read");
        assert_eq!(made(vec![None], Some(10)), (None, 0), "no head stood on");
    }
}

fn asked(table: &Table) -> std::borrow::Cow<'_, Table> {
    std::borrow::Cow::Borrowed(table)
}


fn void() -> a2::fault::Fault {
    a2::fault::Fault::Usage("这一趟起时的会话已锁定或换了钥、席、节点表,停下不读不写".into())
}

fn run_in(table: &Table, p: &Plan, prior: Option<Book>) -> Pass {
    crate::house::mark::mark(crate::house::mark::FEATURE);
    let prior = prior.filter(|b| b.key.eq_ignore_ascii_case(&p.me)).or_else(|| crate::house::book::load(&p.data, &p.me));
    let mut out = Pass { cause: p.cause, book: prior.clone(), tick: None, failed: None, stored: false, filled: 0, from: None, only: p.only.clone(), ms: 0 };
    let fail = |mut out: Pass, e: a2::fault::Fault| {
        out.failed = Some(e);
        out
    };
    let t = match Auto::web_roots() {
        Ok(t) => t,
        Err(e) => return fail(out, e),
    };
    let (side, keys) = match keys_of(&p) {
        Ok(x) => x,
        Err(e) => return fail(out, e),
    };
    if !p.ticket.live() {
        return fail(out, void());
    }
    let (h, _) = match a2::read::height(&t, table) {
        Ok(x) => x,
        Err(e) => return fail(out, e),
    };
    let chain_id = a2::read::table_chain(table);
    let ends = endpoints_of(table);
    let mut book = match prior {
        Some(b) if b.holds(&p.me, chain_id, &p.logic) => b,
        _ => Book::fresh(&p.me, chain_id, &p.logic),
    };
    book.void_provenance(&ends);
    let mut store: Store = (*book.kept).clone();
    store.chain = chain_id;
    store.head = h;
    if !p.ticket.live() {
        return fail(out, void());
    }
    let (time, word, endpoints) = match batch(&t, table, &mut store, h, p.only.as_deref()) {
        Ok(x) => x,
        Err(e) => return kept_fail(out, &p, book, &store, e),
    };
    let agreement = if word == "single" { a2::read::Agreement::Single } else { a2::read::Agreement::Agreed };
    store.set_head(h, time, agreement, endpoints);
    out.tick = Some(Tick { block: h, chain_time: time, agreement: word, endpoints, at: p.now, pass: None });
    let meta = |w: &str| Meta { block: h, at: p.now, agreement: w.to_string() };
    let read_wall = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let head = Head { chain_time: time, meta: meta(word), read_wall };
    let only = p.only.clone();
    let from_of = |hi: u64| scan_from(Some(hi));
    if !p.ticket.live() {
        return fail(out, void());
    }
    let mine_only = |x: &&a2::recall::Track| only.as_ref().is_none_or(|i| x.rest.to_ascii_lowercase().contains(i));
    out.from = store.tracks.iter().filter(mine_only).map(|x| x.next_from(from_of)).min();
    let carried = match &only {
        None => store.extend(&t, table, from_of),
        Some(i) => {
            let mine = Store { tracks: store.tracks.iter().filter(|x| x.rest.to_ascii_lowercase().contains(i)).cloned().collect(), ..Store::fresh(chain_id) };
            let mut mine = Store { head: h, ..mine };
            let n = mine.extend(&t, table, from_of);
            for tr in mine.tracks {
                if let Some(x) = store.tracks.iter_mut().find(|x| x.rest == tr.rest) {
                    *x = tr;
                }
            }
            n
        }
    };
    if let Err(e) = carried {
        return kept_fail(out, &p, book, &store, e);
    }
    if only.is_none() {
        for w in store.again() {
            if let Err(e) = store.fetch(&t, table, &w) {
                return kept_fail(out, &p, book, &store, e);
            }
        }
    }
    let mut last: Option<(Result<Summary, a2::fault::Fault>, std::collections::BTreeSet<String>)> = None;
    let mut fetched: Vec<Want> = Vec::new();
    for _ in 0..ROUNDS {
        if !p.ticket.live() {
            return fail(out, void());
        }
        let rec = Recall::over(Arc::new(store.clone()));
        let got = recall::within(&rec, || build(&t, table, &p, side, &keys));
        let wanted = rec.take_wanted();
        if wanted.is_empty() {
            last = Some((got, rec.used()));
            break;
        }
        if wanted.iter().all(|w| fetched.contains(w)) {
            return kept_fail(out, &p, book, &store, a2::fault::Fault::Unread(format!("刷新读回来的读数页面仍认不得:{wanted:?}")));
        }
        for w in &wanted {
            if !fetched.contains(w) {
                fetched.push(w.clone());
            }
        }
        match fetch_all(&t, table, &mut store, &wanted) {
            Ok(f) => {
                if let Some(f) = f {
                    out.from = Some(out.from.map_or(f, |x| x.min(f)));
                }
            }
            Err(e) => return kept_fail(out, &p, book, &store, e),
        }
    }
    let Some((got, used)) = last else {
        return kept_fail(out, &p, book, &store, a2::fault::Fault::Unread(format!("刷新问了 {ROUNDS} 轮,页面仍要没读过的读数")));
    };
    let sm = match got {
        Ok(x) => x,
        Err(e) => return kept_fail(out, &p, book, &store, e),
    };
    if only.is_none() {
        store.prune(&used);
    }
    let was = |instance: Option<&str>| -> Option<Meta> {
        match instance {
            Some(i) => book.states.iter().find(|x| x.instance.eq_ignore_ascii_case(i)).map(|x| x.meta.clone()),
            None => book.balances.as_ref().map(|b| b.meta.clone()),
        }
    };
    let kept_meta = |instance: Option<&str>, fresh: Meta| -> Meta {
        match &only {
            Some(o) if instance.is_none_or(|i| !i.eq_ignore_ascii_case(o)) => was(instance).unwrap_or(fresh),
            _ => fresh,
        }
    };
    let balances = sm.balances.map(|(native, coins, w)| Balances { holder: p.me.clone(), native, coins, meta: kept_meta(None, meta(w)) });
    let states: Vec<State> = sm.states.into_iter().map(|(instance, code, columns, w)| State { meta: kept_meta(Some(&instance), meta(w)), instance, code, columns }).collect();
    if only.is_none() {
        book.cursor = Some(h);
        book.scanned = keys.iter().map(|k| k.to_ascii_lowercase()).collect();
        book.scan = Some(meta(sm.scan));
    }
    book.cases = sm.cases;
    book.foreign = sm.foreign;
    if balances.is_some() {
        book.balances = balances;
    }
    book.states = states;
    book.head = Some(head);
    book.kept = Arc::new(store);
    if only.is_none() {
        let f = eravon_standard::chain::refill_tokens(&p.data, crate::house::seam::WRITER, p.now, &t, table);
        out.filled = f.changed;
        book.tokens = f.read;
    }
    out.stored = eravon_vault::reading::put(&p.data, &p.me, &book.to_bytes()).is_ok();
    out.book = Some(book);
    out
}

pub const KEPT_PASSES: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seen {
    pub cause: Cause,
    pub failed: String,
    pub from: Option<u64>,
    pub stored: bool,
    pub filled: usize,
    pub only: Option<String>,
    pub ms: u64,
}

impl Seen {
    pub fn row(&self) -> eravon_literals::json::Out {
        use eravon_literals::json::Out;
        let mut m = vec![
            ("cause", Out::S(self.cause.name().to_string())),
            ("failed", Out::S(self.failed.clone())),
            ("filled", Out::N(self.filled as u64)),
            ("ms", Out::N(self.ms)),
            ("stored", Out::B(self.stored)),
        ];
        if let Some(f) = self.from {
            m.push(("from", Out::N(f)));
        }
        if let Some(o) = &self.only {
            m.push(("only", Out::S(o.clone())));
        }
        Out::O(m)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn note(sh: &mut crate::house::shell::Shell, cause: Cause, failed: Option<&a2::fault::Fault>, from: Option<u64>, stored: bool, filled: usize, only: Option<String>, ms: u64) {
    sh.refreshes.push(Seen { cause, failed: failed.map(|e| e.name().to_string()).unwrap_or_default(), from, stored, filled, only, ms });
    if sh.refreshes.len() > KEPT_PASSES {
        sh.refreshes.remove(0);
    }
}

pub fn failed(sh: &mut crate::house::shell::Shell, cause: Cause, only: Option<String>, e: &a2::fault::Fault) {
    sh.read_failed = Some(e.clone());
    note(sh, cause, Some(e), None, false, 0, only, 0);
}

pub const UNLANDED: &str = "E_APP_PASS_UNLANDED";

pub fn unlanded(sh: &mut crate::house::shell::Shell, cause: Cause, only: Option<String>, budget_ms: u64) {
    sh.read_failed = Some(a2::fault::Fault::Unread(format!("刷新在 {budget_ms} 毫秒里没落地")));
    sh.refreshes.push(Seen { cause, failed: UNLANDED.to_string(), from: None, stored: false, filled: 0, only, ms: budget_ms });
    if sh.refreshes.len() > KEPT_PASSES {
        sh.refreshes.remove(0);
    }
}

pub fn land(sh: &mut crate::house::shell::Shell, p: Pass) {
    if let Some(b) = p.book {
        sh.book = Some(b);
        sh.book_gen = sh.book_gen.wrapping_add(1);
    }
    sh.read_failed = p.failed.clone();
    note(sh, p.cause, p.failed.as_ref(), p.from, p.stored, p.filled, p.only, p.ms);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_open_order_is_compared_only_on_a_whole_reading() {
        let at = |s: Option<&str>| Some(("0xaa".to_string(), s.map(str::to_string)));
        let mut f = Follow::default();
        assert!(!f.moved(true, at(Some("a")), at(Some("a"))), "same row: nothing read again");
        assert!(!f.moved(false, at(Some("a")), at(None)), "table unread: compares nothing");
        assert!(!f.moved(false, at(None), at(Some("b"))), "scan incomplete: compares nothing");
        assert!(f.moved(true, at(Some("b")), at(Some("b"))), "whole again: compared with the last whole reading");
        assert!(!f.moved(true, at(Some("b")), at(Some("b"))));
        assert!(!f.moved(true, None, None), "no order open");
        assert!(!f.moved(true, at(None), at(Some("c"))), "first whole reading of a row that was not there: nothing to compare");
    }

    #[test]
    fn an_order_opened_on_a_part_reading_is_read_again_when_a_whole_one_finds_it_changed() {
        let at = |s: Option<&str>| Some(("0xaa".to_string(), s.map(str::to_string)));
        let mut f = Follow::default();
        assert!(!f.moved(false, at(Some("a")), at(Some("a"))), "opened on a part reading: compares nothing");
        assert!(!f.moved(false, at(Some("a")), at(None)), "the next read fails: compares nothing, keeps the row");
        assert!(f.moved(true, at(None), at(Some("b"))), "whole and changed: read again");
        assert!(!f.moved(true, at(Some("b")), at(Some("b"))));
        assert!(!f.moved(false, at(Some("b")), at(Some("z"))), "a part reading after a whole one compares nothing");
        assert!(!f.moved(true, at(Some("z")), at(Some("b"))), "against the whole pass's reading, not the part one");
        assert!(!f.moved(true, at(Some("b")), Some(("0xbb".to_string(), Some("q".to_string())))), "another order: its own base");
        assert!(!f.moved(false, None, Some(("0xbb".to_string(), Some("q".to_string())))));
        assert!(f.moved(true, None, Some(("0xbb".to_string(), Some("r".to_string())))));
    }

    #[test]
    fn a_scan_starts_back_off_blocks_before_the_cursor_and_from_genesis_without_one() {
        assert_eq!(scan_from(None), 0);
        assert_eq!(scan_from(Some(10)), 0);
        assert_eq!(scan_from(Some(1000)), 1000 - k7::SCAN_BACKOFF_BLOCKS);
    }

    #[test]
    fn every_cause_has_a_word_of_its_own() {
        let mut names: Vec<&str> = Cause::ALL.iter().map(|c| c.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Cause::ALL.len());
    }

    #[test]
    fn each_cause_makes_its_readings_stale_and_a_landing_only_its_order() {
        use crate::reads::Cause as _;
        assert_eq!(<Cause as crate::reads::Cause>::ALL, &Cause::ALL[..]);
        for c in Cause::ALL {
            let stale = c.stales();
            assert!(!stale.is_empty(), "{c:?} makes nothing stale");
            assert_eq!(stale == [Reading::Order], c == Cause::Landed, "{c:?}: {stale:?}");
        }
        assert!(Cause::Period.stales().contains(&Reading::Seat) && Cause::Page.stales().contains(&Reading::Seat));
    }
}
