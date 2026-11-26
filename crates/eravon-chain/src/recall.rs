use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use eravon_literals::json::{In, Out};

use crate::ask::fingerprint;

use crate::endpoints::Table;
use crate::fault::{Fault, R};
use crate::read::{Agreement, Reading};
use crate::seam;
use crate::trace::Traced;
use crate::transport::Transport;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kept {
    pub value: Result<String, Refused>,
    pub agreement: Agreement,
    pub endpoints: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    Reverted(String),
    Split(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub rest: String,
    pub lo: u64,
    pub hi: u64,
    pub rows: Vec<String>,
    pub segs: Vec<Seg>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seg {
    pub lo: u64,
    pub agreement: Agreement,
    pub endpoints: usize,
    pub left: Vec<(crate::seam::NodeAt, String)>,
}

impl Seg {
    fn of(lo: u64, r: &Reading) -> Seg {
        Seg { lo, agreement: r.agreement, endpoints: r.endpoints, left: r.left.clone() }
    }

    pub fn read(&self) -> bool {
        self.endpoints > 0 && !self.left.iter().any(|(_, why)| why.starts_with(crate::read::UNANSWERED))
    }
}

impl Track {
    pub fn next_from(&self, from_of: impl Fn(u64) -> u64) -> u64 {
        let tail = from_of(self.hi);
        self.owed(tail).map_or(tail, |(a, _)| a.min(tail))
    }

    pub fn owed(&self, tail: u64) -> Option<(u64, u64)> {
        let i = self.segs.iter().position(|s| !s.read() && s.lo < tail)?;
        let a = self.segs[i].lo;
        Some((a, self.segs.get(i + 1).map_or(self.hi, |n| n.lo.saturating_sub(1)).min(tail.saturating_sub(1))))
    }


    pub fn provenance(&self, from: u64, to: u64) -> (Agreement, usize, Vec<(crate::seam::NodeAt, String)>) {
        let mut agreement = Agreement::Agreed;
        let mut endpoints = usize::MAX;
        let mut left: Vec<(crate::seam::NodeAt, String)> = Vec::new();
        for (i, s) in self.segs.iter().enumerate() {
            let end = self.segs.get(i + 1).map_or(self.hi, |n| n.lo.saturating_sub(1));
            if end < from || s.lo > to {
                continue;
            }
            if s.agreement == Agreement::Single {
                agreement = Agreement::Single;
            }
            endpoints = endpoints.min(s.endpoints);
            for l in &s.left {
                if !left.iter().any(|(e, _)| *e == l.0) {
                    left.push(l.clone());
                }
            }
        }
        if endpoints == usize::MAX {
            return (Agreement::Single, 0, left);
        }
        (agreement, endpoints, left)
    }

    fn read_on(&mut self, from: u64, hi: u64, r: &Reading) {
        self.segs.retain(|s| s.lo < from);
        self.segs.push(Seg::of(from, r));
        self.segs.dedup_by(|b, a| a.agreement == b.agreement && a.endpoints == b.endpoints && a.left == b.left);
        self.lo = self.lo.min(from);
        self.hi = hi;
        if let Some(s) = self.segs.first_mut() {
            s.lo = self.lo;
        }
    }

    fn read_span(&mut self, a: u64, b: u64, r: &Reading) {
        let at = |x: u64| self.segs.iter().rev().find(|s| s.lo <= x).cloned();
        let after = if b < self.hi { at(b + 1) } else { None };
        let mut segs: Vec<Seg> = self.segs.iter().filter(|s| s.lo < a).cloned().collect();
        segs.push(Seg::of(a, r));
        if let Some(mut s) = after {
            s.lo = b + 1;
            segs.push(s);
        }
        segs.extend(self.segs.iter().filter(|s| s.lo > b + 1).cloned());
        segs.dedup_by(|y, x| x.agreement == y.agreement && x.endpoints == y.endpoints && x.left == y.left);
        self.segs = segs;
    }

    fn void(&mut self) {
        self.segs = vec![Seg { lo: self.lo, agreement: Agreement::Single, endpoints: 0, left: Vec::new() }];
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Store {
    pub chain: u64,
    pub head: u64,
    pub answers: BTreeMap<String, Kept>,
    pub tracks: Vec<Track>,
    pub traces: BTreeMap<String, Traced>,
    pub retrace: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Want {
    Read(String, String),
    Logs(String, u64),
    Trace(String, String, String),
}

#[derive(Debug)]
pub struct Recall {
    store: Arc<Store>,
    wanted: Mutex<Vec<Want>>,
    used: Mutex<BTreeSet<String>>,
    instead: Option<Fault>,
}

impl Recall {
    pub fn over(store: Arc<Store>) -> Arc<Recall> {
        Arc::new(Recall { store, wanted: Mutex::new(Vec::new()), used: Mutex::new(BTreeSet::new()), instead: None })
    }

    pub fn over_failed(store: Arc<Store>, why: Option<Fault>) -> Arc<Recall> {
        Arc::new(Recall { store, wanted: Mutex::new(Vec::new()), used: Mutex::new(BTreeSet::new()), instead: why })
    }

    fn unread(&self, what: &str) -> Fault {
        self.instead.clone().unwrap_or_else(|| unread(what))
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn take_wanted(&self) -> Vec<Want> {
        self.wanted.lock().map(|mut w| std::mem::take(&mut *w)).unwrap_or_default()
    }

    pub fn missed(&self) -> bool {
        self.wanted.lock().map(|w| !w.is_empty()).unwrap_or(false)
    }

    pub fn used(&self) -> BTreeSet<String> {
        self.used.lock().map(|u| u.clone()).unwrap_or_default()
    }

    fn want(&self, w: Want) {
        if let Ok(mut v) = self.wanted.lock()
            && !v.contains(&w)
        {
            v.push(w);
        }
    }

    fn use_(&self, k: String) {
        if let Ok(mut u) = self.used.lock() {
            u.insert(k);
        }
    }
}

thread_local! {
    static SCOPE: RefCell<Option<Arc<Recall>>> = const { RefCell::new(None) };
}

pub fn within<X>(r: &Arc<Recall>, f: impl FnOnce() -> X) -> X {
    struct Back(Option<Arc<Recall>>);
    impl Drop for Back {
        fn drop(&mut self) {
            let was = self.0.take();
            SCOPE.with(|s| *s.borrow_mut() = was);
        }
    }
    let was = SCOPE.with(|s| s.borrow_mut().replace(r.clone()));
    let _back = Back(was);
    f()
}

pub fn current() -> Option<Arc<Recall>> {
    SCOPE.with(|s| s.borrow().clone())
}

pub fn key(method: &str, params_json: &str) -> String {
    let p = eravon_literals::json::read(params_json.as_bytes()).map(|v| fingerprint(&v)).unwrap_or_else(|_| params_json.to_string());
    format!("{method} {p}")
}

fn trace_key(tx: &str, to: &str, sel: &str) -> String {
    format!("{} {} {}", tx.to_ascii_lowercase(), to.to_ascii_lowercase(), sel.to_ascii_lowercase())
}

fn unread(what: &str) -> Fault {
    Fault::Unread(format!("读数册里还没有这一问:{what}"))
}

fn parse(text: &str) -> R<In> {
    eravon_literals::json::read(text.as_bytes()).map_err(|e| Fault::Endpoint(format!("读数册里一份答读不回({e})")))
}

pub(crate) fn answer(method: &str, params_json: &str) -> Option<R<Reading>> {
    let r = current()?;
    let k = key(method, params_json);
    r.use_(format!("a:{k}"));
    Some(match r.store.answers.get(&k) {
        Some(Kept { value: Ok(text), agreement, endpoints }) => parse(text).map(|value| Reading { value, agreement: *agreement, endpoints: *endpoints, left: Vec::new() }),
        Some(Kept { value: Err(Refused::Reverted(why)), .. }) => Err(Fault::Execution(why.clone())),
        Some(Kept { value: Err(Refused::Split(why)), .. }) => Err(Fault::Disagree(why.clone())),
        None => {
            let (m, p) = k.split_once(' ').unwrap_or((method, ""));
            r.want(Want::Read(m.to_string(), p.to_string()));
            Err(r.unread(&k))
        }
    })
}

pub(crate) fn params_text(params: &[Out]) -> String {
    String::from_utf8(eravon_literals::json::write(&Out::A(params.to_vec()))).unwrap_or_default()
}

fn block_of(row: &In) -> Option<u64> {
    row.get("blockNumber").and_then(In::as_str).and_then(|h| seam::read_quantity(h).ok())
}

pub(crate) fn logs(rest: &str, from: u64, to: seam::Upto) -> Option<R<(Vec<In>, Reading)>> {
    let r = current()?;
    r.use_(format!("l:{rest}"));
    let Some(tr) = r.store.tracks.iter().find(|t| t.rest == rest && t.lo <= from) else {
        r.want(Want::Logs(rest.to_string(), from));
        return Some(Err(r.unread(&format!("{}{rest} 自块 {from}", seam::M_GET_LOGS))));
    };
    let hi = match to {
        seam::Upto::Block(n) => n.min(tr.hi),
        seam::Upto::Latest => tr.hi,
    };
    let mut list = Vec::new();
    for text in &tr.rows {
        let row = match parse(text) {
            Ok(x) => x,
            Err(e) => return Some(Err(e)),
        };
        if block_of(&row).is_some_and(|b| b >= from && b <= hi) {
            list.push(row);
        }
    }
    let (agreement, endpoints, left) = tr.provenance(from, hi);
    let reading = Reading { value: In::Arr(list.clone()), agreement, endpoints, left };
    Some(Ok((list, reading)))
}

pub(crate) fn traced(tx: &str, to: &str, sel: &str) -> Option<R<Traced>> {
    let r = current()?;
    let k = trace_key(tx, to, sel);
    r.use_(format!("t:{k}"));
    Some(match r.store.traces.get(&k) {
        Some(t) => Ok(t.clone()),
        None => {
            r.want(Want::Trace(tx.to_string(), to.to_string(), sel.to_string()));
            Err(r.unread(&format!("追踪 {k}")))
        }
    })
}

pub(crate) fn refuse(method: &str) -> Option<Fault> {
    current().map(|_| Fault::Unread(format!("只看读数册的时候不上网:{method} 没有发出")))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Age {
    Head,
    Batch,
    Probe,
    Kept,
    Again,
}

pub fn age(key: &str, k: &Kept, head: u64) -> Age {
    let (m, p) = key.split_once(' ').unwrap_or((key, ""));
    let latest = p.contains(&format!("\"{}\"", seam::TAG_LATEST));
    let null = matches!(&k.value, Ok(v) if v == "null");
    let deep = |block: Option<u64>| block.is_some_and(|b| b.saturating_add(eravon_literals::constants::SCAN_BACKOFF_BLOCKS) <= head);
    let block_in = |field: &str| -> Option<u64> {
        let v = eravon_literals::json::read(k.value.as_ref().ok()?.as_bytes()).ok()?;
        seam::read_quantity(v.get(field)?.as_str()?).ok()
    };
    let has = |field: &str| k.value.as_ref().ok().and_then(|t| eravon_literals::json::read(t.as_bytes()).ok()).is_some_and(|v| v.get(field).is_some());
    match m {
        seam::M_BLOCK_NUMBER => Age::Head,
        seam::M_GET_BLOCK if latest => Age::Head,
        seam::M_GET_BLOCK | seam::M_CHAIN_ID if k.endpoints > 0 => Age::Kept,
        seam::M_CALL if latest && sent_from(p) => Age::Again,
        seam::M_CALL | seam::M_GET_BALANCE if latest => Age::Batch,
        seam::M_GET_CODE if latest => match &k.value {
            Ok(v) if v == "\"0x\"" => Age::Probe,
            _ if k.endpoints == 0 => Age::Again,
            _ => Age::Kept,
        },
        _ if k.endpoints == 0 => Age::Again,
        seam::M_GET_TX if !null && !has("hash") => Age::Again,
        seam::M_GET_TX | seam::M_GET_RECEIPT if !null && deep(block_in("blockNumber")) => Age::Kept,
        seam::M_GET_BLOCK_BY_HASH | seam::M_GET_BLOCK_RECEIPTS | seam::M_GET_RAW_TX if !null => Age::Kept,
        seam::M_GET_PROOF if !null && deep(proof_block(p)) => Age::Kept,
        _ => Age::Again,
    }
}

fn proof_block(params_json: &str) -> Option<u64> {
    let v = eravon_literals::json::read(params_json.as_bytes()).ok()?;
    seam::read_quantity(v.as_arr()?.get(2)?.as_str()?).ok()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Batched {
    Call(String, String),
    Balance(String),
}

fn sent_from(params_json: &str) -> bool {
    match eravon_literals::json::read(params_json.as_bytes()) {
        Ok(In::Arr(ps)) => ps.first().is_some_and(|x| x.get("from").is_some()),
        _ => false,
    }
}

pub fn batched_of(key: &str) -> Option<Batched> {
    let (m, p) = key.split_once(' ')?;
    if !p.contains(&format!("\"{}\"", seam::TAG_LATEST)) || sent_from(p) {
        return None;
    }
    let Ok(In::Arr(ps)) = eravon_literals::json::read(p.as_bytes()) else { return None };
    let first = ps.first()?;
    match m {
        seam::M_CALL => Some(Batched::Call(first.get("to")?.as_str()?.to_string(), first.get("data")?.as_str()?.to_string())),
        seam::M_GET_BALANCE => Some(Batched::Balance(first.as_str()?.to_string())),
        _ => None,
    }
}

pub fn method_of(m: &str) -> Option<&'static str> {
    BOOK_READS.into_iter().find(|x| *x == m)
}

const BOOK_READS: [&str; 13] = [
    seam::M_CHAIN_ID,
    seam::M_BLOCK_NUMBER,
    seam::M_GET_BLOCK,
    seam::M_GET_BALANCE,
    seam::M_CALL,
    seam::M_GET_CODE,
    seam::M_GET_TX,
    seam::M_GET_RECEIPT,
    seam::M_GET_TX_COUNT,
    seam::M_GET_BLOCK_BY_HASH,
    seam::M_GET_BLOCK_RECEIPTS,
    seam::M_GET_RAW_TX,
    seam::M_GET_PROOF,
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Filter {
    address: Vec<String>,
    topics: Vec<Vec<String>>,
}

impl Filter {
    fn of(rest: &str) -> Option<Filter> {
        let v = eravon_literals::json::read(format!("{{{}}}", rest.strip_prefix(',').unwrap_or(rest)).as_bytes()).ok()?;
        let low = |x: &In| x.as_str().map(str::to_ascii_lowercase);
        let address = match v.get("address") {
            Some(In::Str(a)) => vec![a.to_ascii_lowercase()],
            Some(In::Arr(a)) => a.iter().filter_map(low).collect(),
            _ => Vec::new(),
        };
        let topics = match v.get("topics") {
            Some(In::Arr(a)) => a
                .iter()
                .map(|x| match x {
                    In::Str(s) => vec![s.to_ascii_lowercase()],
                    In::Arr(c) => c.iter().filter_map(low).collect(),
                    _ => Vec::new(),
                })
                .collect(),
            _ => Vec::new(),
        };
        Some(Filter { address, topics })
    }

    fn text(&self) -> String {
        let list = |v: &[String]| format!("[{}]", v.iter().map(|x| format!("\"{x}\"")).collect::<Vec<_>>().join(","));
        let mut rest = String::new();
        match self.address.len() {
            0 => {}
            1 => rest.push_str(&format!(",\"address\":\"{}\"", self.address[0])),
            _ => rest.push_str(&format!(",\"address\":{}", list(&self.address))),
        }
        if !self.topics.is_empty() {
            let parts: Vec<String> = self
                .topics
                .iter()
                .map(|t| match t.len() {
                    0 => "null".to_string(),
                    1 => format!("\"{}\"", t[0]),
                    _ => list(t),
                })
                .collect();
            rest.push_str(&format!(",\"topics\":[{}]", parts.join(",")));
        }
        rest
    }

    fn admits(&self, row: &In) -> bool {
        let low = |x: Option<&In>| x.and_then(In::as_str).map(str::to_ascii_lowercase).unwrap_or_default();
        if !self.address.is_empty() && !self.address.contains(&low(row.get("address"))) {
            return false;
        }
        let topics: Vec<String> = row.get("topics").and_then(In::as_arr).map(|a| a.iter().map(|x| low(Some(x))).collect()).unwrap_or_default();
        self.topics.iter().enumerate().all(|(i, c)| c.is_empty() || topics.get(i).is_some_and(|t| c.contains(t)))
    }

    fn shape(&self) -> (bool, usize, Vec<String>) {
        (self.address.is_empty(), self.topics.len(), self.topics.first().cloned().unwrap_or_default())
    }

    fn join(&self, o: &Filter) -> Option<Filter> {
        let union = |a: &[String], b: &[String]| -> Vec<String> {
            let mut v = a.to_vec();
            for x in b {
                if !v.contains(x) {
                    v.push(x.clone());
                }
            }
            v
        };
        let address = union(&self.address, &o.address);
        let topics: Vec<Vec<String>> = self.topics.iter().zip(&o.topics).map(|(a, b)| if a.is_empty() || b.is_empty() { Vec::new() } else { union(a, b) }).collect();
        if address.len() > seam::FILTER_FAN || topics.iter().any(|t| t.len() > seam::FILTER_FAN) {
            return None;
        }
        Some(Filter { address, topics })
    }
}

impl Store {
    pub fn fresh(chain: u64) -> Store {
        Store { chain, ..Store::default() }
    }

    pub fn put(&mut self, key: String, k: Kept) {
        self.answers.insert(key, k);
    }

    pub fn set_head(&mut self, block: u64, time: u64, agreement: Agreement, endpoints: usize) {
        self.head = block;
        let latest = params_text(&[Out::S(seam::TAG_LATEST.to_string()), Out::B(false)]);
        let body = fingerprint(&In::Obj(vec![("number".into(), In::Str(seam::quantity(block))), ("timestamp".into(), In::Str(seam::quantity(time)))]));
        self.put(key(seam::M_GET_BLOCK, &latest), Kept { value: Ok(body), agreement, endpoints });
        self.put(key(seam::M_BLOCK_NUMBER, "[]"), Kept { value: Ok(format!("\"{}\"", seam::quantity(block))), agreement, endpoints });
    }

    pub fn batched(&self) -> Vec<(String, Batched)> {
        self.answers.iter().filter(|(k, v)| age(k, v, self.head) == Age::Batch).filter_map(|(k, _)| batched_of(k).map(|b| (k.clone(), b))).collect()
    }

    pub fn probes(&self) -> Vec<(String, String)> {
        self.answers
            .iter()
            .filter(|(k, v)| age(k, v, self.head) == Age::Probe)
            .filter_map(|(k, _)| {
                let p = k.split_once(' ')?.1;
                let Ok(In::Arr(ps)) = eravon_literals::json::read(p.as_bytes()) else { return None };
                Some((k.clone(), ps.first()?.as_str()?.to_string()))
            })
            .collect()
    }

    pub fn again(&self) -> Vec<Want> {
        let answers = self
            .answers
            .iter()
            .filter(|(k, v)| age(k, v, self.head) == Age::Again)
            .filter_map(|(k, _)| k.split_once(' ').map(|(m, p)| Want::Read(m.to_string(), p.to_string())));
        let traces = self.traces.iter().filter(|(k, v)| matches!(v, Traced::Found { endpoints: 0, .. } | Traced::NotFound { endpoints: 0, .. }) || self.retrace.contains(*k)).filter_map(|(k, _)| {
            let mut p = k.split(' ');
            Some(Want::Trace(p.next()?.to_string(), p.next()?.to_string(), p.next()?.to_string()))
        });
        answers.chain(traces).collect()
    }

    pub fn void(&mut self) {
        for k in self.answers.values_mut() {
            k.endpoints = 0;
            k.agreement = Agreement::Single;
        }
        for t in &mut self.tracks {
            t.void();
        }
        self.retrace = self.traces.iter().filter(|(_, v)| matches!(v, Traced::Untraced)).map(|(k, _)| k.clone()).collect();
        for v in self.traces.values_mut() {
            if let Traced::Found { agreement, endpoints, .. } | Traced::NotFound { agreement, endpoints } = v {
                *agreement = Agreement::Single;
                *endpoints = 0;
            }
        }
    }

    pub fn forget(&mut self, key: &str) {
        self.answers.remove(key);
    }

    pub fn prune(&mut self, used: &BTreeSet<String>) {
        self.answers.retain(|k, v| age(k, v, self.head) == Age::Head || k.starts_with(seam::M_CHAIN_ID) || used.contains(&format!("a:{k}")));
        self.tracks.retain(|t| used.contains(&format!("l:{}", t.rest)));
        self.traces.retain(|k, _| used.contains(&format!("t:{k}")));
    }

    pub fn fetch<T: Transport>(&mut self, t: &T, table: &Table, w: &Want) -> R<()> {
        match w {
            Want::Read(m, p) => {
                let k = format!("{m} {p}");
                let method = method_of(m).ok_or_else(|| Fault::Usage(format!("读数册不认这一问:{m}")))?;
                let got = if method == seam::M_GET_TX { crate::read::transaction_kept(t, table, p) } else { crate::read::read_agreed_raw(t, table, method, p) };
                match got {
                    Ok(r) => self.put(k, Kept { value: Ok(fingerprint(&r.value)), agreement: r.agreement, endpoints: r.endpoints }),
                    Err(e) if crate::fault::reverted(&e) => self.put(k, Kept { value: Err(Refused::Reverted(e.detail())), agreement: Agreement::Single, endpoints: table.len() }),
                    Err(Fault::Disagree(why)) if method == seam::M_GET_RECEIPT || method == seam::M_GET_TX => {
                        self.put(k, Kept { value: Err(Refused::Split(why)), agreement: Agreement::Single, endpoints: table.len() })
                    }
                    Err(e) => return Err(e),
                }
            }
            Want::Logs(rest, from) => {
                let got = crate::read::logs_rest_as_asked(t, table, rest, *from, seam::Upto::Block(self.head));
                self.keep_logs(rest, *from, got)?;
            }
            Want::Trace(tx, to, sel) => {
                let (got, _) = crate::trace::find(t, table, tx, to, sel)?;
                let k = trace_key(tx, to, sel);
                self.retrace.remove(&k);
                self.traces.insert(k, got);
            }
        }
        Ok(())
    }

    pub fn extend<T: Transport>(&mut self, t: &T, table: &Table, from_of: impl Fn(u64) -> u64) -> R<usize> {
        let head = self.head;
        let filters: Vec<Option<Filter>> = self.tracks.iter().map(|x| Filter::of(&x.rest)).collect();
        let owed: Vec<(usize, u64, u64)> = (0..self.tracks.len()).filter_map(|i| self.tracks[i].owed(from_of(self.tracks[i].hi)).map(|(a, b)| (i, a, b))).collect();
        let froms: Vec<u64> = self.tracks.iter().map(|x| from_of(x.hi)).collect();
        let groups = join_groups(&filters, &froms, u64::MAX);
        let mut asks: Vec<(String, u64, seam::Upto)> = owed.iter().map(|&(i, a, b)| (self.tracks[i].rest.clone(), a, seam::Upto::Block(b))).collect();
        let live: Vec<&(Filter, u64, Vec<usize>)> = groups.iter().filter(|(_, from, _)| *from <= head).collect();
        asks.extend(live.iter().map(|(g, from, members)| {
            let rest = if filters[members[0]].is_some() && members.len() > 1 { g.text() } else { self.tracks[members[0]].rest.clone() };
            (rest, *from, seam::Upto::Block(head))
        }));
        let (tt, tb) = (t.clone(), table.clone());
        let mut got = crate::read::side_by_side(asks, move |(rest, from, to)| crate::read::logs_rest_as_asked(&tt, &tb, &rest, from, to)).into_iter();
        let mut paid = 0usize;
        for &(i, a, b) in &owed {
            let (rows, r) = got.next().ok_or_else(|| Fault::Usage("欠读的段少答了一段".into()))??;
            let tr = &mut self.tracks[i];
            tr.rows.retain(|x| eravon_literals::json::read(x.as_bytes()).ok().and_then(|v| block_of(&v)).is_none_or(|n| n < a || n > b));
            tr.rows.extend(rows.iter().map(fingerprint));
            sort_rows(&mut tr.rows);
            tr.read_span(a, b, &r);
            paid += 1;
        }
        let asked = groups.len();
        for (_, from, members) in live {
            let (rows, r) = got.next().ok_or_else(|| Fault::Usage("并组的问少答了一组".into()))??;
            for &i in members {
                let f = filters[i].clone();
                let tr = &mut self.tracks[i];
                tr.rows.retain(|x| eravon_literals::json::read(x.as_bytes()).ok().and_then(|v| block_of(&v)).is_some_and(|b| b < *from));
                for row in &rows {
                    if f.as_ref().is_none_or(|f| f.admits(row)) {
                        tr.rows.push(fingerprint(row));
                    }
                }
                tr.read_on(*from, head, &r);
            }
        }
        Ok(asked + paid)
    }

    pub fn read_logs<T: Transport>(&self, t: &T, table: &Table, wants: &[(String, u64)]) -> Vec<R<(Vec<In>, Reading)>> {
        let head = self.head;
        let filters: Vec<Option<Filter>> = wants.iter().map(|(rest, _)| Filter::of(rest)).collect();
        let froms: Vec<u64> = wants.iter().map(|(_, from)| *from).collect();
        let groups = join_groups(&filters, &froms, head);
        let asks: Vec<(String, u64)> = groups
            .iter()
            .map(|(g, from, members)| (if filters[members[0]].is_some() && members.len() > 1 { g.text() } else { wants[members[0]].0.clone() }, *from))
            .collect();
        let (tt, tb) = (t.clone(), table.clone());
        let got = crate::read::side_by_side(asks, move |(rest, from)| crate::read::logs_rest_as_asked(&tt, &tb, &rest, from, seam::Upto::Block(head)));
        let mut out: Vec<Option<R<(Vec<In>, Reading)>>> = wants.iter().map(|_| None).collect();
        for ((_, _, members), g) in groups.iter().zip(got) {
            for &i in members {
                let own = |row: &&In| filters[i].as_ref().is_none_or(|f| f.admits(row)) && block_of(row).is_none_or(|b| b >= froms[i]);
                out[i] = Some(match &g {
                    Ok((rows, r)) if members.len() > 1 => Ok((rows.iter().filter(own).cloned().collect(), r.clone())),
                    Ok((rows, r)) => Ok((rows.clone(), r.clone())),
                    Err(e) => Err(e.clone()),
                });
            }
        }
        out.into_iter().map(|x| x.unwrap_or_else(|| Err(Fault::Usage("日志欠读少答了一问".into())))).collect()
    }

    pub fn keep_logs(&mut self, rest: &str, from: u64, got: R<(Vec<In>, Reading)>) -> R<()> {
        let (rows, r) = got?;
        self.tracks.retain(|x| x.rest != rest);
        self.tracks.push(Track { rest: rest.to_string(), lo: from, hi: self.head, rows: rows.iter().map(fingerprint).collect(), segs: vec![Seg::of(from, &r)] });
        Ok(())
    }
}

fn join_groups(filters: &[Option<Filter>], froms: &[u64], head: u64) -> Vec<(Filter, u64, Vec<usize>)> {
    let mut groups: Vec<(Filter, u64, Vec<usize>)> = Vec::new();
    for (i, f) in filters.iter().enumerate() {
        let from = froms[i];
        let Some(f) = f.as_ref().filter(|_| from <= head) else {
            groups.push((Filter::default(), from, vec![i]));
            continue;
        };
        let slot = groups.iter_mut().find(|(g, gfrom, m)| *gfrom <= head && filters[m[0]].is_some() && g.shape() == f.shape() && g.join(f).is_some());
        match slot {
            Some((g, gfrom, m)) => {
                *g = g.join(f).unwrap_or_else(|| g.clone());
                *gfrom = (*gfrom).min(from);
                m.push(i);
            }
            None => groups.push((f.clone(), from, vec![i])),
        }
    }
    groups
}

fn sort_rows(rows: &mut [String]) {
    let key = |x: &String| {
        let v = eravon_literals::json::read(x.as_bytes()).ok();
        let n = v.as_ref().and_then(block_of).unwrap_or(0);
        let i = v.as_ref().and_then(|v| v.get("logIndex").and_then(In::as_str).and_then(|h| seam::read_quantity(h).ok())).unwrap_or(0);
        (n, i)
    };
    rows.sort_by_key(key);
}

fn agreement_of(w: &str) -> Agreement {
    if w == Agreement::Agreed.word() { Agreement::Agreed } else { Agreement::Single }
}

impl Store {
    pub fn to_out(&self) -> Out {
        let n = |x: usize| Out::N(x as u64);
        let answers = self
            .answers
            .iter()
            .map(|(k, v)| {
                let (ok, err, split) = match &v.value {
                    Ok(x) => (Out::S(x.clone()), Out::Null, Out::Null),
                    Err(Refused::Reverted(e)) => (Out::Null, Out::S(e.clone()), Out::Null),
                    Err(Refused::Split(e)) => (Out::Null, Out::Null, Out::S(e.clone())),
                };
                Out::O(vec![("agreement", Out::S(v.agreement.word().into())), ("endpoints", n(v.endpoints)), ("err", err), ("key", Out::S(k.clone())), ("ok", ok), ("split", split)])
            })
            .collect();
        let tracks = self
            .tracks
            .iter()
            .map(|t| {
                let segs = t
                    .segs
                    .iter()
                    .map(|g| {
                        Out::O(vec![
                            ("agreement", Out::S(g.agreement.word().into())),
                            ("endpoints", n(g.endpoints)),
                            ("left", Out::A(g.left.iter().map(|(e, w)| Out::A(vec![Out::S(e.written().to_string()), Out::S(w.clone())])).collect())),
                            ("lo", Out::N(g.lo)),
                        ])
                    })
                    .collect();
                Out::O(vec![
                    ("hi", Out::N(t.hi)),
                    ("lo", Out::N(t.lo)),
                    ("rest", Out::S(t.rest.clone())),
                    ("rows", Out::A(t.rows.iter().map(|r| Out::S(r.clone())).collect())),
                    ("segs", Out::A(segs)),
                ])
            })
            .collect();
        let traces = self
            .traces
            .iter()
            .map(|(k, v)| {
                let (kind, call, agreement, endpoints) = match v {
                    Traced::Found { call, agreement, endpoints } => ("found", Some(call), Some(*agreement), *endpoints),
                    Traced::NotFound { agreement, endpoints } => ("none", None, Some(*agreement), *endpoints),
                    Traced::Untraced => ("untraced", None, None, 0),
                };
                Out::O(vec![
                    ("agreement", Out::S(agreement.map(|a| a.word()).unwrap_or("").into())),
                    ("endpoints", n(endpoints)),
                    ("from", Out::S(call.map(|c| c.from.clone()).unwrap_or_default())),
                    ("input", Out::S(call.map(|c| c.input.clone()).unwrap_or_default())),
                    ("key", Out::S(k.clone())),
                    ("kind", Out::S(kind.into())),
                ])
            })
            .collect();
        Out::O(vec![("answers", Out::A(answers)), ("chain", Out::N(self.chain)), ("head", Out::N(self.head)), ("traces", Out::A(traces)), ("tracks", Out::A(tracks))])
    }

    pub fn of_in(v: &In) -> Option<Store> {
        let s = |v: &In, k: &str| v.get(k).and_then(In::as_str).map(str::to_string);
        let n = |v: &In, k: &str| match v.get(k) {
            Some(In::Int(i)) if *i >= 0 => Some(*i as u64),
            _ => None,
        };
        let arr = |v: &In, k: &str| v.get(k).and_then(In::as_arr).map(<[In]>::to_vec).unwrap_or_default();
        let mut out = Store { chain: n(v, "chain")?, head: n(v, "head")?, ..Store::default() };
        for a in arr(v, "answers") {
            let value = match (s(&a, "ok"), s(&a, "err"), s(&a, "split")) {
                (Some(x), None, None) => Ok(x),
                (None, Some(e), None) => Err(Refused::Reverted(e)),
                (None, None, Some(e)) => Err(Refused::Split(e)),
                _ => return None,
            };
            out.answers.insert(s(&a, "key")?, Kept { value, agreement: agreement_of(&s(&a, "agreement")?), endpoints: n(&a, "endpoints")? as usize });
        }
        for t in arr(v, "tracks") {
            let mut segs = Vec::new();
            for g in arr(&t, "segs") {
                let left = arr(&g, "left").iter().map(|p| Some((crate::seam::NodeAt::new(p.as_arr()?.first()?.as_str()?), p.as_arr()?.get(1)?.as_str()?.to_string()))).collect::<Option<Vec<_>>>()?;
                segs.push(Seg { lo: n(&g, "lo")?, agreement: agreement_of(&s(&g, "agreement")?), endpoints: n(&g, "endpoints")? as usize, left });
            }
            let rows = arr(&t, "rows").iter().map(|r| r.as_str().map(str::to_string)).collect::<Option<Vec<_>>>()?;
            let (lo, hi) = (n(&t, "lo")?, n(&t, "hi")?);
            if segs.first().is_none_or(|g: &Seg| g.lo != lo) || segs.windows(2).any(|w| w[0].lo >= w[1].lo) || segs.last().is_some_and(|g| g.lo > hi) {
                return None;
            }
            out.tracks.push(Track { rest: s(&t, "rest")?, lo, hi, rows, segs });
        }
        for x in arr(v, "traces") {
            let endpoints = n(&x, "endpoints")? as usize;
            let agreement = agreement_of(&s(&x, "agreement")?);
            let got = match s(&x, "kind")?.as_str() {
                "found" => Traced::Found { call: crate::trace::Call { from: s(&x, "from")?, input: s(&x, "input")? }, agreement, endpoints },
                "none" => Traced::NotFound { agreement, endpoints },
                "untraced" => Traced::Untraced,
                _ => return None,
            };
            out.traces.insert(s(&x, "key")?, got);
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(addr: &str, topics: &[&str], block: u64) -> In {
        In::Obj(vec![
            ("address".into(), In::Str(addr.into())),
            ("blockNumber".into(), In::Str(seam::quantity(block))),
            ("topics".into(), In::Arr(topics.iter().map(|t| In::Str((*t).into())).collect())),
        ])
    }

    #[test]
    fn inside_a_scope_a_kept_answer_is_served_and_an_unkept_one_is_wanted() {
        let mut s = Store::fresh(1);
        s.put(key(seam::M_GET_BALANCE, "[\"0xab\",\"latest\"]"), Kept { value: Ok("\"0x5\"".into()), agreement: Agreement::Agreed, endpoints: 2 });
        let r = Recall::over(Arc::new(s));
        within(&r, || {
            let got = answer(seam::M_GET_BALANCE, "[\"0xab\", \"latest\"]").unwrap().unwrap();
            assert_eq!(got.value.as_str(), Some("0x5"));
            assert_eq!(got.agreement, Agreement::Agreed);
            assert!(matches!(answer(seam::M_GET_BALANCE, "[\"0xcd\",\"latest\"]"), Some(Err(Fault::Unread(_)))));
            assert!(refuse(seam::M_SEND_RAW).is_some());
        });
        assert!(answer(seam::M_GET_BALANCE, "[\"0xab\",\"latest\"]").is_none());
        assert_eq!(r.take_wanted(), vec![Want::Read(seam::M_GET_BALANCE.into(), "[\"0xcd\",\"latest\"]".into())]);
        assert!(r.used().contains(&format!("a:{}", key(seam::M_GET_BALANCE, "[\"0xab\",\"latest\"]"))));
    }

    #[test]
    fn a_disagreement_is_kept_and_answered_by_its_name_never_as_not_there() {
        let mut s = Store::fresh(1);
        s.head = 1000;
        let k = key(seam::M_GET_RECEIPT, "[\"0x1\"]");
        let split = Kept { value: Err(Refused::Split("一家有、一家没有".into())), agreement: Agreement::Single, endpoints: 2 };
        assert_eq!(age(&k, &split, 1000), Age::Again, "说法不一每趟重问");
        s.put(k, split);
        let text = eravon_literals::json::write(&s.to_out());
        let back = Store::of_in(&eravon_literals::json::read(&text).unwrap()).unwrap();
        assert_eq!(back, s, "说法不一照原样存回");
        within(&Recall::over(Arc::new(s)), || {
            assert!(matches!(answer(seam::M_GET_RECEIPT, "[\"0x1\"]"), Some(Err(Fault::Disagree(_)))));
        });
    }

    #[test]
    fn a_kept_log_query_answers_within_the_blocks_it_covers() {
        let mut s = Store::fresh(1);
        s.tracks.push(Track { rest: ",\"address\":\"0xaa\"".into(), lo: 10, hi: 50, rows: vec![fingerprint(&log("0xaa", &[], 12)), fingerprint(&log("0xaa", &[], 40))], segs: vec![Seg { lo: 10, agreement: Agreement::Single, endpoints: 1, left: Vec::new() }] });
        let r = Recall::over(Arc::new(s));
        within(&r, || {
            let (rows, _) = logs(",\"address\":\"0xaa\"", 20, seam::Upto::Latest).unwrap().unwrap();
            assert_eq!(rows.len(), 1);
            assert!(matches!(logs(",\"address\":\"0xaa\"", 5, seam::Upto::Latest), Some(Err(Fault::Unread(_)))));
        });
        assert_eq!(r.take_wanted(), vec![Want::Logs(",\"address\":\"0xaa\"".into(), 5)]);
    }

    #[test]
    fn queries_of_one_shape_join_and_each_takes_back_only_what_its_filter_admits() {
        let a = Filter::of(",\"address\":\"0xaa\"").unwrap();
        let b = Filter::of(",\"address\":\"0xbb\"").unwrap();
        let j = a.join(&b).unwrap();
        assert_eq!(j.text(), ",\"address\":[\"0xaa\",\"0xbb\"]");
        assert!(a.admits(&log("0xAA", &[], 1)) && !a.admits(&log("0xbb", &[], 1)));
        let c = Filter::of(",\"address\":\"0xcc\",\"topics\":[\"0x01\",\"0x02\"]").unwrap();
        let d = Filter::of(",\"address\":\"0xcc\",\"topics\":[\"0x01\",\"0x03\"]").unwrap();
        assert_eq!(c.shape(), d.shape());
        let cd = c.join(&d).unwrap();
        assert_eq!(cd.text(), ",\"address\":\"0xcc\",\"topics\":[\"0x01\",[\"0x02\",\"0x03\"]]");
        assert!(c.admits(&log("0xcc", &["0x01", "0x02"], 1)) && !c.admits(&log("0xcc", &["0x01", "0x03"], 1)));
        assert_ne!(a.shape(), c.shape());
    }

    #[test]
    fn each_answer_ages_as_its_kind() {
        let k = |v: &str| Kept { value: Ok(v.into()), agreement: Agreement::Single, endpoints: 1 };
        let h = 1000;
        assert_eq!(age(&key(seam::M_CALL, "[{\"data\":\"0x\",\"to\":\"0xa\"},\"latest\"]"), &k("\"0x\""), h), Age::Batch);
        assert_eq!(age(&key(seam::M_GET_CODE, "[\"0xa\",\"latest\"]"), &k("\"0x\""), h), Age::Probe);
        assert_eq!(age(&key(seam::M_GET_CODE, "[\"0xa\",\"latest\"]"), &k("\"0x60\""), h), Age::Kept);
        assert_eq!(age(&key(seam::M_GET_BLOCK, "[\"0x10\",false]"), &k("{}"), h), Age::Kept);
        assert_eq!(age(&key(seam::M_GET_RECEIPT, "[\"0x1\"]"), &k("null"), h), Age::Again);
        assert_eq!(age(&key(seam::M_GET_RECEIPT, "[\"0x1\"]"), &k("{\"blockNumber\":\"0x10\"}"), h), Age::Kept);
        assert_eq!(age(&key(seam::M_GET_RECEIPT, "[\"0x1\"]"), &k("{\"blockNumber\":\"0x3e0\"}"), h), Age::Again);
    }

    #[test]
    fn a_call_asked_as_from_a_sender_is_asked_again_as_itself_never_batched() {
        let k = Kept { value: Ok("\"0x\"".into()), agreement: Agreement::Single, endpoints: 1 };
        let from = key(seam::M_CALL, "[{\"data\":\"0x\",\"from\":\"0xb\",\"to\":\"0xa\"},\"latest\"]");
        let bare = key(seam::M_CALL, "[{\"data\":\"0x\",\"to\":\"0xa\"},\"latest\"]");
        assert_eq!(age(&from, &k, 1000), Age::Again);
        assert_eq!(batched_of(&from), None);
        assert_eq!(batched_of(&bare), Some(Batched::Call("0xa".into(), "0x".into())));
        let mut s = Store::fresh(1);
        s.put(from.clone(), k.clone());
        s.put(bare.clone(), k);
        assert_eq!(s.batched().into_iter().map(|(x, _)| x).collect::<Vec<_>>(), vec![bare]);
    }

    #[test]
    fn a_store_goes_to_its_document_part_and_back_unchanged() {
        let mut s = Store::fresh(31337);
        s.set_head(9, 100, Agreement::Agreed, 2);
        s.put(key(seam::M_CALL, "[{\"data\":\"0x01\",\"to\":\"0xa\"},\"latest\"]"), Kept { value: Err(Refused::Reverted("reverted".into())), agreement: Agreement::Single, endpoints: 1 });
        s.tracks.push(Track { rest: ",\"address\":\"0xaa\"".into(), lo: 0, hi: 9, rows: vec![fingerprint(&log("0xaa", &["0x01"], 3))], segs: vec![Seg { lo: 0, agreement: Agreement::Single, endpoints: 1, left: vec![("http://x".into(), "限流".into())] }, Seg { lo: 5, agreement: Agreement::Agreed, endpoints: 2, left: Vec::new() }] });
        s.traces.insert("0x1 0xa 0x12345678".into(), Traced::Found { call: crate::trace::Call { from: "0xb".into(), input: "0x12345678".into() }, agreement: Agreement::Agreed, endpoints: 2 });
        s.traces.insert("0x2 0xa 0x12345678".into(), Traced::Untraced);
        let text = eravon_literals::json::write(&s.to_out());
        let back = Store::of_in(&eravon_literals::json::read(&text).unwrap()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn a_stretch_read_with_an_endpoint_gone_keeps_its_provenance_and_is_read_again() {
        let rd = |agreement, endpoints, left: Vec<(crate::seam::NodeAt, String)>| Reading { value: In::Arr(Vec::new()), agreement, endpoints, left };
        let gone = vec![(crate::seam::NodeAt::from("http://a"), format!("{}E_CHAIN_TRANSPORT:connect refused", crate::read::UNANSWERED))];
        let mut t = Track { rest: ",\"address\":\"0xaa\"".into(), lo: 100, hi: 500, rows: Vec::new(), segs: vec![Seg::of(100, &rd(Agreement::Single, 1, gone.clone()))] };
        let from_of = |hi: u64| hi - 10;
        assert_eq!(t.next_from(from_of), 100, "出列那一段从它的首块重读");
        t.read_on(490, 600, &rd(Agreement::Agreed, 2, Vec::new()));
        assert_eq!(t.next_from(from_of), 100);
        assert_eq!(t.provenance(100, 600), (Agreement::Single, 1, gone.clone()), "整条的来路取最弱的一段");
        assert_eq!(t.provenance(495, 600), (Agreement::Agreed, 2, Vec::new()), "只问近端即近端那一段的来路");
        t.read_on(100, 610, &rd(Agreement::Agreed, 2, Vec::new()));
        assert_eq!(t.segs.len(), 1);
        assert_eq!(t.next_from(from_of), 600);
        assert_eq!(t.provenance(100, 610), (Agreement::Agreed, 2, Vec::new()));
    }

    #[test]
    fn an_owed_stretch_is_paid_back_whole_in_one_refresh() {
        let rd = |agreement, endpoints, left: Vec<(crate::seam::NodeAt, String)>| Reading { value: In::Arr(Vec::new()), agreement, endpoints, left };
        let gone = vec![(crate::seam::NodeAt::from("http://a"), format!("{}E_CHAIN_TRANSPORT:connect refused", crate::read::UNANSWERED))];
        let w = seam::LOG_WINDOW;
        let mut t = Track { rest: ",\"address\":\"0xaa\"".into(), lo: 0, hi: 3 * w, rows: Vec::new(), segs: vec![Seg::of(0, &rd(Agreement::Single, 1, gone.clone()))] };
        let tail = 3 * w - 64;
        assert_eq!(t.owed(tail), Some((0, tail - 1)), "欠的整段一趟问");
        t.read_span(0, tail - 1, &rd(Agreement::Single, 1, gone.clone()));
        assert_eq!(t.owed(tail), Some((0, tail - 1)));
        assert_eq!(t.provenance(0, w), (Agreement::Single, 1, gone.clone()));
        t.read_span(0, tail - 1, &rd(Agreement::Agreed, 2, Vec::new()));
        assert_eq!(t.owed(tail), None);
        assert_eq!(t.next_from(|hi| hi - 64), tail);
        assert_eq!(t.provenance(0, 3 * w), (Agreement::Single, 1, gone.clone()), "末段那一段仍按读它那一趟的来路");
    }

    #[test]
    fn another_endpoint_table_leaves_every_kept_reading_vouched_for_by_none_and_asked_again() {
        let mut s = Store::fresh(1);
        s.head = 1000;
        let rk = key(seam::M_GET_RECEIPT, "[\"0x1\"]");
        s.put(rk.clone(), Kept { value: Ok("{\"blockNumber\":\"0x10\"}".into()), agreement: Agreement::Agreed, endpoints: 2 });
        s.tracks.push(Track { rest: ",\"address\":\"0xaa\"".into(), lo: 10, hi: 900, rows: Vec::new(), segs: vec![Seg { lo: 10, agreement: Agreement::Agreed, endpoints: 2, left: Vec::new() }] });
        s.traces.insert("0x1 0xa 0x12345678".into(), Traced::NotFound { agreement: Agreement::Agreed, endpoints: 2 });
        s.traces.insert("0x2 0xa 0x12345678".into(), Traced::Untraced);
        assert!(s.again().is_empty());
        s.void();
        assert_eq!(age(&rk, &s.answers[&rk], 1000), Age::Again);
        assert_eq!(s.tracks[0].next_from(|hi| hi), 10, "扫链记录整条重读");
        assert_eq!(s.tracks[0].provenance(10, 900).1, 0);
        assert_eq!(
            s.again(),
            vec![Want::Read(seam::M_GET_RECEIPT.into(), "[\"0x1\"]".into()), Want::Trace("0x1".into(), "0xa".into(), "0x12345678".into()), Want::Trace("0x2".into(), "0xa".into(), "0x12345678".into())]
        );
        assert_eq!(s.traces.len(), 2, "追不到的那一条照留,页上照旧有值");
    }

    #[test]
    fn the_head_answers_the_latest_block_and_the_block_number() {
        let mut s = Store::fresh(1);
        s.set_head(0x20, 0x99, Agreement::Agreed, 2);
        let r = Recall::over(Arc::new(s));
        within(&r, || {
            let b = answer(seam::M_GET_BLOCK, &params_text(&[Out::S(seam::TAG_LATEST.into()), Out::B(false)])).unwrap().unwrap();
            assert_eq!(b.value.get("timestamp").and_then(In::as_str), Some("0x99"));
            let n = answer(seam::M_BLOCK_NUMBER, "[]").unwrap().unwrap();
            assert_eq!(n.value.as_str(), Some("0x20"));
        });
    }

    #[test]
    fn a_transaction_is_kept_whole_and_proves_itself_inside_the_book() {
        const HASH: &str = "0x9a87b002786a874b0f0923cf2ebd9272f4606d17719dce3b4fc6207a547d95fb";
        const TX: &str = r#"{"blockNumber":"0x12","from":"0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266","type":"0x0","chainId":"0x1","nonce":"0x0","gasPrice":"0x1","gas":"0x5208","to":"0x00000000000000000000000000000000000000aa","value":"0x0","input":"0x","r":"0x15ab622cc30dbb812e3296d47cf141340366c693f3bbda47d026ece18da773","s":"0x36cb4723171ae7d61e4d5b9ae0b282b96021c482e65208b685ce454c8129cc9","v":"0x25","hash":"0x9a87b002786a874b0f0923cf2ebd9272f4606d17719dce3b4fc6207a547d95fb"}"#;
        const CUT: &str = r#"{"blockNumber":"0x12","from":"0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266","input":"0x","to":"0x00000000000000000000000000000000000000aa","value":"0x0"}"#;
        #[derive(Clone)]
        struct One;
        impl crate::transport::Transport for One {
            fn post(&self, _url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
                let b = String::from_utf8_lossy(body).to_string();
                let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
                let chain = format!("\"0x{:x}\"", eravon_literals::constants::current_network().standard.chain_id);
                let result = if b.contains(seam::M_CHAIN_ID) { chain.as_str() } else if b.contains(seam::M_GET_TX) { TX } else { "null" };
                Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
            }
        }
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let table = Table::new(vec!["http://127.0.0.1:20977".to_string()]).expect("table");
        let params = format!("[\"{HASH}\"]");
        let k = key(seam::M_GET_TX, &params);
        let hash: [u8; 32] = seam::bytes_of("hash", HASH).ok().and_then(|b| b.try_into().ok()).expect("hash");
        let mut s = Store::fresh(1);
        s.fetch(&One, &table, &Want::Read(seam::M_GET_TX.into(), params.clone())).expect("fetched");
        let kept = s.answers[&k].clone();
        assert!(kept.value.as_ref().is_ok_and(|t| t.contains("\"r\":") && t.contains("\"hash\":")), "kept whole: {kept:?}");
        assert_eq!(age(&k, &kept, 1000), Age::Kept, "a whole mined transaction stays kept");
        let r = Recall::over(Arc::new(s));
        within(&r, || {
            let v = crate::read::proven_transaction(&One, &table, 1, &hash).expect("proves itself inside the book");
            assert_eq!(v.get("nonce").and_then(In::as_str), Some("0x0"));
        });
        let mut c = Store::fresh(1);
        c.put(k.clone(), Kept { value: Ok(CUT.into()), agreement: Agreement::Agreed, endpoints: 1 });
        assert_eq!(age(&k, &c.answers[&k], 1000), Age::Again, "a cut transaction is asked again");
        let r = Recall::over(Arc::new(c));
        within(&r, || assert!(crate::read::proven_transaction(&One, &table, 1, &hash).is_none(), "a cut transaction proves nothing"));
    }
}
