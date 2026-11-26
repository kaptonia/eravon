use crate::rpc::{Broke, Endpoint, Trouble};
use crate::tx;
use eravon::hexfmt;
use crate::wire::W;
use eravon::json::Value;

pub const ANCHORED_SIGNATURE: &str = "Anchored(address,bytes32)";

pub const OVER_ASKS: &str = "OVER_ASKS";

pub fn topic0() -> [u8; 32] {
    eravon::cryptox::keccak256(ANCHORED_SIGNATURE.as_bytes())
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Refusal {
    ChainIdMismatch { declared: u64, served: String },
    NoEndpoint(u64),
    Unanswered { chain: u64, asked: Asked },
    TxNotItself(String),
    Malformed(Shape),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Asked {
    Declined { method: String, said: String, width: Option<(u64, u64)> },
    Broke { method: String, broke: Broke },
    NotServed(String),
    NotLogs,
    NoReceipt(String),
    Short,
    OverAsks { cap: u64, said: Option<String> },
}

impl Asked {
    pub fn evidence(&self) -> String {
        match self {
            Asked::Declined { method, said, .. } => format!("{method} {said}"),
            Asked::Broke { method, broke } => format!("{method} {}", broke.evidence()),
            Asked::NotServed(k) => format!("NOT_SERVED {k}"),
            Asked::NotLogs => "eth_getLogs NOT_LOGS".to_string(),
            Asked::NoReceipt(tx) => format!("eth_getTransactionReceipt NO_RECEIPT {tx}"),
            Asked::Short => "SHORT".to_string(),
            Asked::OverAsks { cap, said: None } => format!("eth_getLogs {OVER_ASKS} {cap}"),
            Asked::OverAsks { cap, said: Some(said) } => format!("eth_getLogs {OVER_ASKS} {cap} {said}"),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Shape {
    TxLacks { part: String, tx: String },
    NoTimestamp(u64),
    CodeNotBytes,
    Basis,
    Contradiction(String),
}

impl Shape {
    pub fn evidence(&self) -> String {
        match self {
            Shape::TxLacks { part, tx } => format!("TX_LACKS {part} {tx}"),
            Shape::NoTimestamp(n) => format!("eth_getBlockByNumber NO_TIMESTAMP {n}"),
            Shape::CodeNotBytes => "eth_getCode NOT_BYTES".to_string(),
            Shape::Basis => "BASIS_UNREAD".to_string(),
            Shape::Contradiction(k) => format!("CONTRADICTION {k}"),
        }
    }
}

impl Refusal {
    pub fn stands_aside(&self) -> bool {
        match self {
            Refusal::Unanswered { asked: Asked::Declined { said, .. }, .. } => crate::patience::stands_aside(&Trouble::Node(said.clone())),
            Refusal::Unanswered { asked: Asked::Broke { broke, .. }, .. } => crate::patience::stands_aside(&Trouble::Transport(broke.clone())),
            _ => false,
        }
    }

    pub fn cut(&self) -> bool {
        matches!(self, Refusal::Unanswered { asked: Asked::Declined { width: Some(_), .. }, .. })
    }

    pub fn over_asks(&self) -> Option<u64> {
        match self {
            Refusal::Unanswered { asked: Asked::OverAsks { cap, .. }, .. } => Some(*cap),
            _ => None,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Refusal::ChainIdMismatch { .. } => "E_CHAIN_ID",
            Refusal::NoEndpoint(_) => "E_NO_ENDPOINT",
            Refusal::Unanswered { .. } => "E_UNANSWERED",
            Refusal::TxNotItself(_) => "E_TX_NOT_ITSELF",
            Refusal::Malformed(_) => "E_MALFORMED",
        }
    }

    pub fn detail(&self) -> String {
        match self {
            Refusal::ChainIdMismatch { declared, served } => format!("chain {declared} eth_chainId {served}"),
            Refusal::NoEndpoint(c) => format!("chain {c}"),
            Refusal::Unanswered { chain, asked } => format!("chain {chain} {}", asked.evidence()),
            Refusal::TxNotItself(h) => format!("tx {h}"),
            Refusal::Malformed(w) => w.evidence(),
        }
    }
}

pub fn chain_served(served: &W) -> Option<u64> {
    tx::hex_qty(served)
}

pub fn serves_another(served: Option<u64>, given: u64) -> Option<u64> {
    served.filter(|s| *s != given)
}

#[cfg(test)]
#[derive(Clone, Debug)]
pub struct Window {
    pub chain_id: u64,
    pub from_block: u64,
    pub to_block: u64,
    pub registries: Vec<[u8; 20]>,
    pub senders: Vec<[u8; 20]>,
}

pub fn hex_quantity(x: u64) -> String {
    format!("0x{x:x}")
}

fn hex20(a: &[u8; 20]) -> String {
    hexfmt::encode(a)
}

pub fn calldata_carries(calldata: &[u8], word: &[u8; 32]) -> bool {
    let mut base = 0usize;
    while base <= 4 {
        let mut off = base;
        while off + 32 <= calldata.len() {
            if &calldata[off..off + 32] == word {
                return true;
            }
            off += 32;
        }
        base += 4;
    }
    false
}

struct Chain<'a> {
    id: u64,
    ep: &'a mut dyn Endpoint,
}

impl<'a> Chain<'a> {
    fn ask(&mut self, method: &str, params: Vec<Value>) -> Result<W, Refusal> {
        crate::patience::ask(self.ep, method, &Value::Arr(params)).map_err(|t| self.refused(method, t))
    }

    fn refused(&self, method: &str, t: Trouble) -> Refusal {
        match t {
            Trouble::Node(e) => Refusal::Unanswered { chain: self.id, asked: Asked::Declined { method: method.to_string(), width: None, said: e } },
            Trouble::NotServed(k) => Refusal::Unanswered { chain: self.id, asked: Asked::NotServed(k) },
            Trouble::Transport(b) => Refusal::Unanswered { chain: self.id, asked: Asked::Broke { method: method.to_string(), broke: b } },
            Trouble::Contradiction(k) => Refusal::Malformed(Shape::Contradiction(k)),
        }
    }

    fn refused_log(&self, params: &Value, t: Trouble, cuttable: bool) -> Refusal {
        let width = (cuttable && matches!(&t, Trouble::Node(_)) && crate::patience::class_of(&t) == crate::patience::Class::Other).then(|| crate::patience::log_blocks("eth_getLogs", params)).flatten();
        if let (Some(_), Trouble::Node(words)) = (width, &t) {
            crate::action::heard(&self.ep.place(), words);
        }
        match self.refused("eth_getLogs", t) {
            Refusal::Unanswered { chain, asked: Asked::Declined { method, said, .. } } => Refusal::Unanswered { chain, asked: Asked::Declined { method, said, width } },
            other => other,
        }
    }
}
#[cfg(test)]
fn padded(a: &[u8; 20]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[12..].copy_from_slice(a);
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogFilter {
    pub addresses: Vec<[u8; 20]>,
    pub topics: Vec<Topic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Topic {
    Any,
    One([u8; 32]),
    AnyOf(Vec<[u8; 32]>),
}

impl LogFilter {
    pub fn at(&self, from: u64, to: u64) -> Value {
        let topic = |t: &Topic| match t {
            Topic::Any => Value::Null,
            Topic::One(x) => Value::Str(hexfmt::encode(x)),
            Topic::AnyOf(xs) => Value::Arr(xs.iter().map(|x| Value::Str(hexfmt::encode(x))).collect()),
        };
        Value::Obj(vec![
            ("address".into(), Value::Arr(self.addresses.iter().map(|r| Value::Str(hex20(r))).collect())),
            ("fromBlock".into(), Value::Str(hex_quantity(from))),
            ("toBlock".into(), Value::Str(hex_quantity(to))),
            ("topics".into(), Value::Arr(self.topics.iter().map(topic).collect())),
        ])
    }
}

pub const SUBJECT_GROUP: usize = 32;

pub fn subject_groups(filter: &LogFilter) -> Vec<LogFilter> {
    let Some(at) = filter.topics.iter().position(|t| matches!(t, Topic::AnyOf(xs) if xs.len() > SUBJECT_GROUP)) else { return vec![filter.clone()] };
    let Topic::AnyOf(xs) = &filter.topics[at] else { return vec![filter.clone()] };
    let mut once: Vec<[u8; 32]> = Vec::new();
    for x in xs {
        if !once.contains(x) {
            once.push(*x);
        }
    }
    if once.len() <= SUBJECT_GROUP {
        return vec![filter.clone()];
    }
    once.chunks(SUBJECT_GROUP)
        .map(|g| {
            let mut f = filter.clone();
            f.topics[at] = Topic::AnyOf(g.to_vec());
            f
        })
        .collect()
}

fn merged(parts: Vec<Vec<W>>) -> Vec<W> {
    let mut all: Vec<W> = parts.into_iter().flatten().collect();
    let index = |l: &W| l.member("logIndex").and_then(crate::tx::hex_qty).unwrap_or(0);
    all.sort_by_key(|l| (crate::settled::block_of(l).unwrap_or(0), index(l)));
    all
}

#[cfg(test)]
fn anchor_filter(w: &Window, t0: &[u8; 32]) -> LogFilter {
    LogFilter { addresses: w.registries.clone(), topics: vec![Topic::One(*t0), Topic::AnyOf(w.senders.iter().map(padded).collect())] }
}

pub fn logs_over_window(ep: &mut dyn Endpoint, chain: u64, filter: &LogFilter, from: u64, to: u64) -> Result<Vec<W>, Refusal> {
    crate::seam();
    let place = ep.place();
    let mut got = [paged(&mut Chain { id: chain, ep: &mut *ep }, filter, from, to, Over::Alone)];
    crate::action::cut_and_ask(&mut got, |_, _| vec![place.clone()], |_| false, |g| g.as_ref().err().is_some_and(Refusal::cut), |_, _| vec![paged(&mut Chain { id: chain, ep }, filter, from, to, Over::Alone)]);
    let [got] = got;
    got
}

pub fn logs_judged_over(ep: &mut dyn Endpoint, chain: u64, filter: &LogFilter, from: u64, to: u64, nodes: &str) -> Result<Vec<W>, Refusal> {
    crate::seam();
    paged(&mut Chain { id: chain, ep }, filter, from, to, Over::Judged { nodes })
}

pub fn log_asks(ep: &dyn Endpoint, chain: u64, filter: &LogFilter, from: u64, to: u64, nodes: Option<&str>) -> Vec<crate::rpc::Ask<'static>> {
    if to < from {
        return Vec::new();
    }
    let groups = subject_groups(filter);
    if groups.len() > 1 {
        return groups.iter().flat_map(|g| log_asks(ep, chain, g, from, to, nodes)).collect();
    }
    let place = ep.place();
    match reading_on(&place, chain, filter, from, nodes).and_then(|r| r.kept) {
        Some(e) => kept_asks(&place, &e, filter, from, to),
        None => whole_ask(&place, filter, from, to),
    }
}

pub fn logs_ahead<'e>(nodes: &mut [&mut (dyn Endpoint + Send + 'e)], chain: u64, reads: &[(&LogFilter, u64, u64)], nodes_list: Option<&str>) {
    crate::seam();
    if reads.len() < 2 || crate::action::current().is_none() {
        return;
    }
    let mut items: Vec<(String, Box<dyn Endpoint + Send>, Vec<crate::rpc::Ask<'static>>)> = Vec::new();
    let mut here: Vec<usize> = Vec::new();
    for (i, ep) in nodes.iter_mut().enumerate() {
        let asks = logs_ahead_asks(&**ep, chain, reads, nodes_list);
        match ep.another() {
            Some(other) => items.push((ep.place(), other, asks)),
            None => here.push(i),
        }
    }
    let done = crate::endpoints::until_enough(
        items,
        |(place, mut ep, asks)| {
            crate::ahead::ask_ahead(ep.as_mut(), &asks);
            place
        },
        String::new,
        |slots: &[Option<String>]| crate::endpoints::distinct_places(slots.iter().flatten().filter(|p| !p.is_empty()).map(String::as_str)) >= 2,
    );
    if crate::endpoints::distinct_places(done.iter().flatten().filter(|p| !p.is_empty()).map(String::as_str)) >= 2 {
        return;
    }
    for i in here {
        let asks = logs_ahead_asks(&*nodes[i], chain, reads, nodes_list);
        crate::ahead::ask_ahead(&mut *nodes[i], &asks);
    }
}

fn logs_ahead_asks(ep: &dyn Endpoint, chain: u64, reads: &[(&LogFilter, u64, u64)], nodes: Option<&str>) -> Vec<crate::rpc::Ask<'static>> {
    let mut asks: Vec<crate::rpc::Ask<'static>> = Vec::new();
    for (filter, from, to) in reads {
        for a in log_asks(ep, chain, filter, *from, *to, nodes) {
            if !asks.contains(&a) {
                asks.push(a);
            }
        }
    }
    asks
}

fn logs_question(filter: &LogFilter, from: u64, to: u64) -> Value {
    Value::Arr(vec![filter.at(from, to)])
}

fn block_params(n: u64) -> Vec<Value> {
    vec![Value::Str(hex_quantity(n)), Value::Bool(false)]
}

fn whole_ask(place: &str, filter: &LogFilter, from: u64, to: u64) -> Vec<crate::rpc::Ask<'static>> {
    if crate::patience::answered_width(place).is_some_and(|w| w < span_of(from, to)) {
        return Vec::new();
    }
    vec![("eth_getLogs", logs_question(filter, from, to))]
}

fn span_of(from: u64, to: u64) -> u64 {
    to.saturating_sub(from).saturating_add(1)
}

struct ReadingOn {
    key: crate::settled::Key,
    own: Option<crate::settled::Entry>,
    kept: Option<crate::settled::Entry>,
}

fn reading_on(place: &str, chain: u64, filter: &LogFilter, from: u64, nodes: Option<&str>) -> Option<ReadingOn> {
    use crate::settled::{self, Key};
    if !settled::keeps() {
        return None;
    }
    let list = match nodes {
        Some(n) => n.to_string(),
        None => settled::nodes_of([place]),
    };
    let key = Key { chain, nodes: settled::spell_nodes(&list), filter: filter_key(filter) };
    let own = settled::entry(&key).filter(|e| e.from <= from);
    let lent = if own.is_none() && nodes.is_none() { settled::lent(&key, from) } else { None };
    let kept = own.clone().or(lent);
    Some(ReadingOn { key, own, kept })
}

pub fn logs_agreed(nodes: Vec<Box<dyn Endpoint + Send>>, chain: u64, filter: &LogFilter, from: u64, to: u64) -> AgreedLogs {
    crate::seam();
    let places: Vec<String> = nodes.iter().map(|ep| ep.place()).collect();
    let list = crate::settled::nodes_of(places.iter().map(String::as_str));
    type Got = (Box<dyn Endpoint + Send>, Result<(Vec<W>, Vec<crate::settled::Settling>), Refusal>);
    let ask = |nodes: Vec<Box<dyn Endpoint + Send>>, places: Vec<String>| -> Vec<Option<Got>> {
        let (filter, list) = (filter.clone(), list.clone());
        crate::endpoints::until_enough(
            nodes,
            move |mut ep| {
                let mut out = Vec::new();
                let got = paged(&mut Chain { id: chain, ep: ep.as_mut() }, &filter, from, to, Over::Agreed { nodes: &list, out: &mut out }).map(|l| (l, out));
                Some((ep, got))
            },
            || None,
            move |slots: &[Option<Option<Got>>]| {
                let read: Vec<(&str, Vec<u8>)> = places.iter().zip(slots).filter_map(|(p, g)| match g {
                    Some(Some((_, Ok((logs, _))))) => Some((p.as_str(), logs_bytes(logs))),
                    _ => None,
                }).collect();
                crate::judge::two_places_alike(&read) || crate::action::read_while_cut(read.iter().map(|(p, _)| *p))
            },
        )
        .into_iter()
        .map(Option::flatten)
        .collect()
    };
    let mut got: Vec<Option<Got>> = ask(nodes, places.clone());
    crate::action::cut_and_ask(
        &mut got,
        |i, _| vec![places[i].clone()],
        |g| matches!(g, Some((_, Ok(_)))),
        |g| matches!(g, Some((_, Err(r))) if r.cut()),
        |idx, got| {
            let again: Vec<Box<dyn Endpoint + Send>> = idx.iter().filter_map(|i| got[*i].take().map(|(ep, _)| ep)).collect();
            let again_places: Vec<String> = idx.iter().map(|i| places[*i].clone()).collect();
            ask(again, again_places)
        },
    );
    let mut read: Vec<(String, Vec<W>, Vec<crate::settled::Settling>)> = Vec::new();
    let mut refused: Vec<(String, Refusal)> = Vec::new();
    for (g, place) in got.into_iter().zip(places) {
        match g {
            Some((_, Ok((logs, out)))) => read.push((place, logs, out)),
            Some((_, Err(r))) => refused.push((place, r)),
            None => {}
        }
    }
    let bytes: Vec<(&str, Vec<u8>)> = read.iter().map(|(p, l, _)| (p.as_str(), logs_bytes(l))).collect();
    let alike = bytes.iter().find(|(_, b)| crate::endpoints::distinct_places(bytes.iter().filter(|(_, x)| x == b).map(|(p, _)| *p)) >= 2).map(|(_, b)| b.clone());
    if let Some(b) = alike {
        let kept: Vec<(String, Vec<W>, Vec<crate::settled::Settling>)> = read.into_iter().filter(|(_, l, _)| logs_bytes(l) == b).collect();
        let runs: Vec<Vec<crate::settled::Settling>> = kept.iter().map(|(_, _, o)| o.clone()).collect();
        crate::settled::settle(&runs, &[]);
        let answered = kept.len();
        return AgreedLogs { answered, agreed: true, logs: kept.into_iter().next().map(|(_, l, _)| l).unwrap_or_default(), refused: Vec::new() };
    }
    let alike = read.iter().all(|(_, l, _)| crate::settled::same_logs(l, &read[0].1));
    let agreed = crate::endpoints::distinct_places(read.iter().map(|(p, _, _)| p.as_str())) >= 2 && alike;
    if !read.is_empty() && alike {
        let runs: Vec<Vec<crate::settled::Settling>> = read.iter().map(|(_, _, o)| o.clone()).collect();
        crate::settled::settle(&runs, &[]);
    }
    AgreedLogs { answered: read.len(), agreed, logs: read.into_iter().next().map(|(_, l, _)| l).unwrap_or_default(), refused }
}

fn logs_bytes(logs: &[W]) -> Vec<u8> {
    logs.iter().flat_map(|l| crate::wire::write(&crate::wire::project(l, &crate::settled::LOG_FACTS)).into_bytes().into_iter().chain(std::iter::once(b'\n'))).collect()
}

#[derive(Debug)]
pub struct AgreedLogs {
    pub logs: Vec<W>,
    pub answered: usize,
    pub agreed: bool,
    pub refused: Vec<(String, Refusal)>,
}

enum Over<'a> {
    Alone,
    Judged { nodes: &'a str },
    Agreed { nodes: &'a str, out: &'a mut Vec<crate::settled::Settling> },
}

impl Over<'_> {
    fn list(&self) -> Option<&str> {
        match self {
            Over::Alone => None,
            Over::Judged { nodes } | Over::Agreed { nodes, .. } => Some(nodes),
        }
    }
}

fn filter_key(filter: &LogFilter) -> String {
    let at = filter.at(0, 0);
    let kept = match at {
        Value::Obj(ms) => Value::Obj(ms.into_iter().filter(|(k, _)| k == "address" || k == "topics").collect()),
        other => other,
    };
    String::from_utf8(eravon::json::canon_bytes(&kept)).unwrap_or_default()
}

fn paged(chain: &mut Chain, filter: &LogFilter, from: u64, to: u64, over: Over) -> Result<Vec<W>, Refusal> {
    use crate::settled::{self, Entry};
    if to < from {
        return Ok(Vec::new());
    }
    let groups = subject_groups(filter);
    if groups.len() > 1 {
        let mut over = over;
        let mut parts = Vec::new();
        for g in &groups {
            let each = match &mut over {
                Over::Alone => Over::Alone,
                Over::Judged { nodes } => Over::Judged { nodes },
                Over::Agreed { nodes, out } => Over::Agreed { nodes, out: &mut **out },
            };
            parts.push(paged(chain, g, from, to, each)?);
        }
        return Ok(merged(parts));
    }
    let place = chain.ep.place();
    let Some(ReadingOn { key, own, kept }) = reading_on(&place, chain.id, filter, from, over.list()) else {
        return asked(chain, filter, from, to);
    };
    if let Some(e) = &kept {
        let asks = kept_asks(&place, e, filter, from, to);
        crate::ahead::ask_ahead(chain.ep, &asks);
    }
    let held = kept.and_then(|e| rechecked(chain, e));
    let in_range = |l: &W| settled::block_of(l).is_some_and(|b| (from..=to).contains(&b));
    let (mut logs, ask_from) = match &held {
        Some(e) => (e.logs.iter().filter(|l| in_range(l)).cloned().collect::<Vec<W>>(), from.max(e.settled.saturating_add(1))),
        None => (Vec::new(), from),
    };
    let fetched = if ask_from <= to { asked(chain, filter, ask_from, to)? } else { Vec::new() };
    logs.extend(fetched.iter().cloned());
    let settle_to = to.saturating_sub(settled::MARGIN);
    let (base_from, base_logs, base_settled) = match &held {
        Some(e) => (e.from, e.logs.clone(), Some(e.settled)),
        None => (from, Vec::new(), None),
    };
    let carried = base_settled.is_none_or(|s| ask_from == s.saturating_add(1) && settle_to > s);
    let proposed = if carried && settle_to >= base_from {
        let mut kept = base_logs;
        let mut readable = true;
        for l in &fetched {
            match settled::block_of(l) {
                Some(b) if b <= settle_to => kept.push(l.clone()),
                Some(_) => {}
                None => readable = false,
            }
        }
        readable.then_some(Entry { from: base_from, settled: settle_to, logs: kept, single_source: true })
    } else {
        None
    };
    let base = held.as_ref().filter(|_| own.is_some()).map(|e| (e.from, e.settled));
    match over {
        Over::Alone => {
            if let Some(e) = proposed {
                settled::keep_on(&key, e, base);
            }
        }
        Over::Agreed { out, .. } => out.push(settled::Settling { key, entry: proposed, served_single: held.is_some_and(|e| e.single_source), base }),
        Over::Judged { .. } => {}
    }
    Ok(logs)
}

fn kept_asks(place: &str, e: &crate::settled::Entry, filter: &LogFilter, from: u64, to: u64) -> Vec<crate::rpc::Ask<'static>> {
    let mut asks = Vec::new();
    if let Some(last) = e.logs.iter().filter(|l| l.member("blockHash").and_then(W::as_str).is_some()).filter_map(crate::settled::block_of).max() {
        asks.push(("eth_getBlockByNumber", Value::Arr(block_params(last))));
    }
    let ask_from = from.max(e.settled.saturating_add(1));
    if ask_from <= to {
        asks.extend(whole_ask(place, filter, ask_from, to));
    }
    asks
}

fn rechecked(chain: &mut Chain, e: crate::settled::Entry) -> Option<crate::settled::Entry> {
    let mut blocks: Vec<(u64, String)> = e.logs.iter().filter_map(|l| Some((crate::settled::block_of(l)?, l.member("blockHash")?.as_str()?.to_ascii_lowercase()))).collect();
    blocks.sort();
    blocks.dedup();
    let Some(last) = blocks.last().map(|(n, _)| *n) else { return Some(e) };
    for (n, hash) in blocks.iter().rev() {
        let said = chain.ask("eth_getBlockByNumber", block_params(*n)).ok()?;
        if said.member("hash").and_then(W::as_str).is_some_and(|h| h.eq_ignore_ascii_case(hash)) {
            if *n == last {
                return Some(e);
            }
            let logs = e.logs.iter().filter(|l| crate::settled::block_of(l).is_some_and(|b| b <= *n)).cloned().collect();
            return Some(crate::settled::Entry { settled: *n, logs, ..e });
        }
    }
    None
}

fn asked(chain: &mut Chain, filter: &LogFilter, from: u64, to: u64) -> Result<Vec<W>, Refusal> {
    if to < from {
        return Ok(Vec::new());
    }
    let place = chain.ep.place();
    let span = span_of(from, to);
    let known = crate::patience::answered_width(&place).filter(|w| *w < span);
    if known.is_none() && !crate::action::splits(&place) {
        return match crate::patience::ask_again_once(chain.ep, "eth_getLogs", &logs_question(filter, from, to)) {
            Ok(answered) => answered.as_arr().map(<[W]>::to_vec).ok_or(Refusal::Unanswered { chain: chain.id, asked: Asked::NotLogs }),
            Err(t) => Err(chain.refused_log(&logs_question(filter, from, to), t, true)),
        };
    }
    match known {
        Some(w) => pieces(chain, filter, from, to, w, 0),
        None => pieces(chain, filter, from, to, span.div_ceil(2), 1),
    }
}

fn pieces(chain: &mut Chain, filter: &LogFilter, from: u64, to: u64, width: u64, asked_before: u64) -> Result<Vec<W>, Refusal> {
    let place = chain.ep.place();
    let cap = crate::action::asks_cap();
    let (mut width, mut used, mut at) = (width.max(1), asked_before, from);
    let mut said = crate::action::heard_from(&place);
    let mut out = Vec::new();
    while at <= to {
        if crate::endpoints::stopped() {
            return Err(Refusal::Unanswered { chain: chain.id, asked: Asked::Short });
        }
        let left = span_of(at, to);
        if used.saturating_add(left.div_ceil(width)) > cap {
            return Err(Refusal::Unanswered { chain: chain.id, asked: Asked::OverAsks { cap, said } });
        }
        let end = at.saturating_add(width.min(left) - 1);
        let q = logs_question(filter, at, end);
        used += 1;
        match crate::patience::ask(chain.ep, "eth_getLogs", &q) {
            Ok(answered) => {
                out.extend(answered.as_arr().map(<[W]>::to_vec).ok_or(Refusal::Unanswered { chain: chain.id, asked: Asked::NotLogs })?);
                if end < to {
                    crate::patience::keep_answered(&place, width);
                }
                if end == u64::MAX {
                    break;
                }
                at = end + 1;
            }
            Err(Trouble::Node(words)) if end > at && crate::patience::class_of(&Trouble::Node(words.clone())) == crate::patience::Class::Other => {
                crate::action::heard(&place, &words);
                said = Some(words);
                width = span_of(at, end).div_ceil(2);
            }
            Err(t) => return Err(chain.refused_log(&q, t, false)),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod ahead_logs_tests;

#[cfg(test)]
mod width_tests;

#[cfg(test)]
mod window_tests {
    use super::*;

    struct Logs<F: FnMut(u64, u64, usize) -> Result<W, Trouble>> {
        rule: F,
        asked: Vec<(u64, u64)>,
        methods: Vec<String>,
        name: String,
    }

    impl<F: FnMut(u64, u64, usize) -> Result<W, Trouble>> Endpoint for Logs<F> {
        fn call(&mut self, method: &str, params: &Value) -> Result<W, Trouble> {
            self.methods.push(method.to_string());
            let at = |k: &str| -> u64 {
                match params {
                    Value::Arr(ps) => match ps.first() {
                        Some(Value::Obj(f)) => f.iter().find(|(n, _)| n == k).and_then(|(_, v)| v.as_str()).and_then(|h| u64::from_str_radix(h.trim_start_matches("0x"), 16).ok()).unwrap_or(0),
                        _ => 0,
                    },
                    _ => 0,
                }
            };
            let (from, to) = (at("fromBlock"), at("toBlock"));
            let n = self.asked.len();
            self.asked.push((from, to));
            (self.rule)(from, to, n)
        }
        fn name(&self) -> String {
            self.name.clone()
        }
        fn place(&self) -> String {
            self.name.clone()
        }
    }

    fn logs<F: FnMut(u64, u64, usize) -> Result<W, Trouble>>(rule: F) -> Logs<F> {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let name = format!("logs-{}", N.fetch_add(1, std::sync::atomic::Ordering::SeqCst));
        Logs { rule, asked: Vec::new(), methods: Vec::new(), name }
    }

    fn window(from: u64, to: u64) -> Window {
        Window { chain_id: 1, from_block: from, to_block: to, registries: vec![[0x11; 20]], senders: vec![[0x22; 20]] }
    }

    fn empty() -> Result<W, Trouble> {
        Ok(W::of(crate::wire::Body::Arr(Vec::new())))
    }

    fn node_says(code: i64, message: &str) -> Trouble {
        Trouble::Node(format!("{{\"code\":{code},\"message\":\"{message}\"}}"))
    }

    fn scan_window<F: FnMut(u64, u64, usize) -> Result<W, Trouble>>(node: &mut Logs<F>, w: &Window) -> Result<Vec<W>, Refusal> {
        logs_over_window(node, w.chain_id, &anchor_filter(w, &topic0()), w.from_block, w.to_block)
    }

    fn no_pauses() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
    }

    #[test]
    fn a_log_filter_is_written_position_by_position() {
        let f = LogFilter { addresses: Vec::new(), topics: vec![Topic::Any, Topic::One([0x33; 32]), Topic::AnyOf(vec![[0x44; 32], [0x55; 32]])] };
        let got = String::from_utf8(eravon::json::canon_bytes(&f.at(0, u64::MAX))).expect("text");
        let word = |b: u8| format!("\"0x{}\"", format!("{b:02x}").repeat(32));
        assert_eq!(
            got,
            format!("{{\"address\":[],\"fromBlock\":\"0x0\",\"toBlock\":\"0xffffffffffffffff\",\"topics\":[null,{},[{},{}]]}}", word(0x33), word(0x44), word(0x55))
        );
        assert_eq!(String::from_utf8(eravon::json::canon_bytes(&LogFilter { addresses: Vec::new(), topics: Vec::new() }.at(5, 5))).expect("text"), "{\"address\":[],\"fromBlock\":\"0x5\",\"toBlock\":\"0x5\",\"topics\":[]}");
        let w = window(1, 9);
        let anchor = anchor_filter(&w, &topic0());
        let mut padded_sender = [0u8; 32];
        padded_sender[12..].copy_from_slice(&[0x22; 20]);
        assert_eq!(anchor, LogFilter { addresses: vec![[0x11; 20]], topics: vec![Topic::One(topic0()), Topic::AnyOf(vec![padded_sender])] });
    }

    #[test]
    fn a_log_read_is_one_question_for_the_whole_range_and_a_width_refusal_is_read_in_pieces() {
        no_pauses();
        let mut node = logs(|_, _, _| empty());
        assert!(scan_window(&mut node, &window(0, 1_208_000)).is_ok());
        assert_eq!(node.asked, vec![(0, 1_208_000)]);
        let sepolia = "{\"data\":{\"code\":\"ErrGetLogsExceededMaxAllowedRange\",\"message\":\"getLogs request exceeded max allowed range\",\"details\":{\"requestRange\":1208001,\"maxAllowedRange\":30000}},\"code\":-32012,\"message\":\"getLogs request exceeded max allowed range\"}";
        let arbitrum = |f: u64, t: u64| format!("{{\"code\":-32602,\"message\":\"query spans {} blocks ({f} to {t}), but only 30000 are allowed for this request; narrow the block range\"}}", t - f + 1);
        let words: [std::sync::Arc<dyn Fn(u64, u64) -> String + Send + Sync>; 2] = [std::sync::Arc::new(move |_: u64, _: u64| sepolia.to_string()), std::sync::Arc::new(arbitrum)];
        for said in words {
            let mut node = logs(move |f, t, _| if t - f + 1 > 30_000 { Err(Trouble::Node(said(f, t))) } else { empty() });
            assert!(scan_window(&mut node, &window(0, 1_208_000)).is_ok(), "read in pieces");
            assert_eq!(node.asked[..2], [(0, 1_208_000), (0, 1_208_000)], "{:?}", &node.asked[..2]);
            assert!(lays(&node.asked, 30_000, 0, 1_208_000), "the pieces lay the window end to end: {:?}", node.asked);
        }
    }

    fn lays(asked: &[(u64, u64)], cap: u64, from: u64, to: u64) -> bool {
        let mut next = from;
        for (a, b) in asked.iter().filter(|(a, b)| b - a < cap) {
            if *a != next {
                return false;
            }
            next = b + 1;
        }
        next == to + 1
    }

    #[test]
    fn a_refusal_naming_no_range_is_passed_on_and_the_range_kept() {
        no_pauses();
        let busy = "service temporarily unavailable";
        let mut node = logs(move |_, _, _| Err(node_says(-32603, busy)));
        let refused = scan_window(&mut node, &window(1, 10_000));
        match &refused {
            Err(Refusal::Unanswered { asked: Asked::OverAsks { cap, said: Some(said) }, .. }) => assert!(*cap == crate::action::asks_cap() && said.contains(busy), "{said}"),
            other => panic!("{other:?}"),
        }
        assert!(refused.as_ref().err().map(Refusal::detail).is_some_and(|d| d.contains(OVER_ASKS) && d.contains(busy)), "{refused:?}");
        assert_eq!(node.asked[..2], [(1, 10_000); 2]);
        assert!(node.asked[2..].iter().all(|(f, t)| t - f + 1 < 10_000), "halved after the whole range: {:?}", node.asked);
        node.asked.clear();
        assert!(scan_window(&mut node, &window(1, 10_000)).is_err());
        assert_eq!(node.asked[..2], [(1, 10_000); 2], "the next read asks it whole");
        let mut node = logs(|_, _, n| if n == 0 { Err(node_says(-32603, busy)) } else { empty() });
        assert!(scan_window(&mut node, &window(1, 10_000)).is_ok(), "answered when asked once more");
        assert_eq!(node.asked, vec![(1, 10_000); 2]);
    }

    #[test]
    fn an_answer_that_is_not_a_log_list_is_refused_never_empty() {
        let mut node = logs(|_, _, _| Ok(W::of(crate::wire::Body::Null)));
        assert!(matches!(scan_window(&mut node, &window(1, 10)), Err(Refusal::Unanswered { asked: Asked::NotLogs, .. })));
    }

    #[test]
    fn a_window_to_the_last_block_is_one_question() {
        let mut node = logs(|_, _, _| empty());
        assert!(scan_window(&mut node, &window(0, u64::MAX)).is_ok());
        assert_eq!(node.asked, vec![(0, u64::MAX)]);
        let mut node = logs(|_, _, _| empty());
        assert!(scan_window(&mut node, &window(u64::MAX, u64::MAX)).is_ok());
        assert_eq!(node.asked.len(), 1);
    }

    #[test]
    fn a_rate_limited_node_is_waited_out_and_its_range_kept() {
        no_pauses();
        let pauses = crate::said::RATE_BACKOFF.len();
        let mut node = logs(|_, _, _| Err(node_says(-32005, "rate limit: max 25 requests per second")));
        assert!(scan_window(&mut node, &window(1, 1000)).is_err());
        assert_eq!(node.asked, vec![(1, 1000); 1 + pauses], "the same range, once and once after each pause");
        let mut node = logs(|_, _, n| if n == 0 { Err(node_says(-32005, "daily request limit exceeded")) } else { empty() });
        assert!(scan_window(&mut node, &window(1, 1000)).is_ok());
        assert_eq!(node.asked, vec![(1, 1000); 2]);
        let mut node = logs(|_, _, _| Err(crate::rpc::status("https://n.example", 429)));
        assert!(scan_window(&mut node, &window(1, 1000)).is_err());
        assert_eq!(node.asked, vec![(1, 1000); 1 + pauses]);
        let mut node = logs(|f, t, _| if t - f + 1 > 500 { Err(node_says(-32005, "query returned more than 10000 results")) } else { empty() });
        assert!(scan_window(&mut node, &window(1, 1000)).is_ok(), "read in pieces");
        assert_eq!(node.asked[..2], [(1, 1000), (1, 1000)], "a refusal naming results is no rate limit: {:?}", &node.asked[..2]);
        assert!(lays(&node.asked[2..], 500, 1, 1000), "the pieces lay the window end to end: {:?}", node.asked);
    }

    #[test]
    fn a_refusals_evidence_is_machine_form_in_no_language() {
        let cjk = |x: &str| x.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
        let declined = Refusal::Unanswered { chain: 1, asked: Asked::Declined { method: "eth_getLogs".into(), said: "{\"code\":-32000,\"message\":\"busy\"}".into(), width: None } };
        assert_eq!(declined.detail(), "chain 1 eth_getLogs {\"code\":-32000,\"message\":\"busy\"}");
        let late = Refusal::Unanswered {
            chain: 11155111,
            asked: Asked::Broke { method: "eth_getCode".into(), broke: Broke::Late { node: "http://n:1".into(), deadline: std::time::Duration::from_secs(5) } },
        };
        assert_eq!(late.detail(), "chain 11155111 eth_getCode TIMEOUT http://n:1 5000ms");
        let all = [
            declined,
            late,
            Refusal::ChainIdMismatch { declared: 1, served: "\"0x5\"".into() },
            Refusal::NoEndpoint(7),
            Refusal::Unanswered { chain: 1, asked: Asked::NotServed("eth_getLogs [..]".into()) },
            Refusal::Unanswered { chain: 1, asked: Asked::NotLogs },
            Refusal::Unanswered { chain: 1, asked: Asked::NoReceipt("0xab".into()) },
            Refusal::Unanswered { chain: 1, asked: Asked::Short },
            Refusal::TxNotItself("0xab".into()),
            Refusal::Malformed(Shape::TxLacks { part: "input".into(), tx: "0xab".into() }),
            Refusal::Malformed(Shape::NoTimestamp(9)),
            Refusal::Malformed(Shape::CodeNotBytes),
            Refusal::Malformed(Shape::Basis),
            Refusal::Malformed(Shape::Contradiction("k".into())),
        ];
        for r in &all {
            assert!(!r.detail().is_empty() && !cjk(&r.detail()), "{r:?}: {}", r.detail());
        }
    }

}
