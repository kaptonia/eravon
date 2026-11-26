use eravon::json::Value;
use eravon_literals::json::{In, Out};

use crate::ask::{in_of, Node};
use crate::endpoints::{Disagreement, Table};
use crate::fault::{Fault, R};
use crate::judge::{Judged, Missing, NoReading};
use crate::mark;
use crate::rpc::Trouble;
use crate::seam::{self, NodeAt};
use crate::transport::Transport;
use crate::wire::{Body, W};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Agreement {
    Agreed,
    Single,
}

impl Agreement {
    pub const fn word(self) -> &'static str {
        match self {
            Agreement::Agreed => "agreed",
            Agreement::Single => "single",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Reading {
    pub value: In,
    pub agreement: Agreement,
    pub endpoints: usize,
    pub left: Vec<(NodeAt, String)>,
}

impl Reading {
    pub fn rows(&self) -> Vec<(&'static str, Out)> {
        let mut m = vec![
            ("agreement", Out::S(self.agreement.word().to_string())),
            ("endpoints", Out::N(self.endpoints as u64)),
        ];
        if !self.left.is_empty() {
            let one = |(e, why): &(NodeAt, String)| Out::O(vec![("endpoint", Out::S(e.said())), ("why", Out::S(why.clone()))]);
            m.push(("left", Out::A(self.left.iter().map(one).collect())));
        }
        m
    }
}

fn reading(value: In, n: usize) -> Reading {
    Reading { value, agreement: agreement_of(n), endpoints: n, left: Vec::new() }
}

pub fn places<'a>(nodes: impl IntoIterator<Item = &'a NodeAt>) -> usize {
    crate::endpoints::distinct_places(nodes.into_iter().map(NodeAt::written))
}

fn table_places(table: &Table) -> usize {
    crate::endpoints::distinct_places(table.list.iter().map(String::as_str))
}

fn agreement_of(n: usize) -> Agreement {
    if n >= 2 { Agreement::Agreed } else { Agreement::Single }
}


fn moving(tag: &str) -> bool {
    tag == seam::TAG_LATEST || tag == seam::TAG_SAFE || tag == seam::TAG_FINALIZED
}

fn pin_tags<T: Transport>(t: &T, table: &Table, params: &Value) -> R<Value> {
    if cfg!(feature = "unpinned-tags") {
        return Ok(params.clone());
    }
    let Value::Arr(ps) = params else { return Ok(params.clone()) };
    if table.is_empty() || !ps.iter().any(|p| matches!(p, Value::Str(s) if moving(s))) {
        return Ok(params.clone());
    }
    let mut at: Vec<(String, String)> = Vec::new();
    let mut out = Vec::with_capacity(ps.len());
    for p in ps {
        match p {
            Value::Str(tag) if moving(tag) => {
                let h = match at.iter().find(|(k, _)| k == tag) {
                    Some((_, h)) => h.clone(),
                    None => {
                        let h = seam::quantity(head_at(t, table, tag)?);
                        at.push((tag.clone(), h.clone()));
                        h
                    }
                };
                out.push(Value::Str(h));
            }
            other => out.push(other.clone()),
        }
    }
    Ok(Value::Arr(out))
}

pub(crate) fn asked_until<I: Send + 'static, X: Send + 'static>(items: Vec<I>, work: impl Fn(I) -> X + Send + Sync + 'static, mut enough: impl FnMut(&[Option<&X>]) -> bool) -> Vec<Option<X>> {
    crate::endpoints::until_enough(items, move |i| Some(work(i)), || None, |slots: &[Option<Option<X>>]| {
        let view: Vec<Option<&X>> = slots.iter().map(|x| x.as_ref().and_then(Option::as_ref)).collect();
        enough(&view)
    })
    .into_iter()
    .map(Option::flatten)
    .collect()
}


pub(crate) fn all<X>(_: &[Option<&X>]) -> bool {
    false
}

fn two_places<X>(nodes: &[NodeAt], view: &[Option<&X>], said: impl Fn(&X) -> bool) -> bool {
    crate::endpoints::distinct_places(nodes.iter().zip(view).filter(|(_, x)| x.is_some_and(&said)).map(|(n, _)| n.written())) >= 2
}

fn said_all(nodes: &[NodeAt]) -> String {
    nodes.iter().map(NodeAt::said).collect::<Vec<_>>().join("、")
}

type Asked = (Result<W, Trouble>, Option<Fault>);

fn ask_each<T: Transport>(t: &T, nodes: &[NodeAt], method: &str, params: &Value) -> Vec<Option<Asked>> {
    let (t, m, p) = (t.clone(), method.to_string(), params.clone());
    let (em, ep) = (method.to_string(), params.clone());
    let names: Vec<String> = nodes.iter().map(|n| n.written().to_string()).collect();
    asked_until(
        nodes.to_vec(),
        move |n| {
            let row = n.written().to_string();
            let mut ep = Node::new(&t, n);
            match crate::patience::ask(&mut ep, &m, &p) {
                Ok(w) => {
                    if m == seam::M_CHAIN_ID {
                        heard(&row, &w);
                    }
                    (Ok(w), None)
                }
                Err(tr) => {
                    let f = ep.fault(tr.clone());
                    (Err(tr), Some(f))
                }
            }
        },
        move |view: &[Option<&Asked>]| {
            if waits_for_every(&em, &ep) {
                return all(view);
            }
            let arrived: Vec<(String, Result<W, Missing>)> = names.iter().zip(view).filter_map(|(n, a)| a.map(|(r, _)| (n.clone(), r.clone().map_err(Missing::Trouble)))).collect();
            crate::judge::enough(&em, &ep, &arrived, None)
        },
    )
}

fn waits_for_every(method: &str, params: &Value) -> bool {
    matches!(crate::judge::rule_of(method, params), crate::judge::Rule::Most(_))
}

fn missing_of(node: &NodeAt, m: Missing, faults: &[(NodeAt, Option<Fault>)]) -> (String, Fault) {
    match m {
        Missing::Trouble(t) => {
            let kept = faults.iter().find(|(n, _)| n == node).and_then(|(_, f)| f.clone());
            let f = kept.unwrap_or_else(|| crate::fault::of_trouble(node, t, None));
            (format!("{UNANSWERED}{}:{}", f.name(), f.detail()), f)
        }
        Missing::Unreadable(text) => (format!("unreadable {text}"), Fault::Endpoint(format!("malformed {node} 的答这一问读不出:{text}"))),
    }
}

fn reading_of(j: Judged, faults: &[(NodeAt, Option<Fault>)]) -> Reading {
    let n = crate::endpoints::distinct_places(j.sources.iter().map(String::as_str));
    let left = j.missing.into_iter().map(|(name, m)| {
        let node = NodeAt::new(name);
        let (why, _) = missing_of(&node, m, faults);
        (node, why)
    });
    Reading { value: in_of(&j.value), agreement: agreement_of(n), endpoints: n, left: left.collect() }
}

fn differ(method: &str, d: &Disagreement) -> String {
    let mut groups: Vec<(&[u8], Vec<String>)> = Vec::new();
    for (name, bytes) in d.sources.iter().zip(&d.fragments) {
        let said = NodeAt::new(name.as_str()).said();
        match groups.iter_mut().find(|(b, _)| *b == bytes.as_slice()) {
            Some((_, m)) => m.push(said),
            None => groups.push((bytes, vec![said])),
        }
    }
    let each: Vec<String> = groups
        .iter()
        .map(|(b, m)| {
            let text = String::from_utf8_lossy(b);
            let short = if text.chars().count() > 80 { format!("{}…", text.chars().take(80).collect::<String>()) } else { text.into_owned() };
            format!("{} 答 {short}", m.join("、"))
        })
        .collect();
    format!("{method} 各家说法不一:{}", each.join(";"))
}

fn shared_below_top(answers: &mut [(String, Result<W, Trouble>)]) {
    let objs: Vec<Option<W>> = answers.iter().map(|(_, r)| r.as_ref().ok().filter(|w| matches!(w.body, Body::Obj(_))).cloned()).collect();
    for (i, (_, r)) in answers.iter_mut().enumerate() {
        let Ok(w) = r else { continue };
        let Body::Obj(ms) = &mut w.body else { continue };
        for (k, v) in ms.iter_mut() {
            for (j, o) in objs.iter().enumerate() {
                if let Some(other) = o.as_ref().filter(|_| j != i).and_then(|o| o.member(k)) {
                    *v = common_w(v, other);
                }
            }
        }
    }
}

fn common_w(a: &W, b: &W) -> W {
    let body = match (&a.body, &b.body) {
        (Body::Obj(x), Body::Obj(_)) => Body::Obj(x.iter().filter_map(|(k, v)| b.member(k).map(|w| (k.clone(), common_w(v, w)))).collect()),
        (Body::Arr(x), Body::Arr(y)) if x.len() == y.len() => Body::Arr(x.iter().zip(y).map(|(v, w)| common_w(v, w)).collect()),
        _ => return a.clone(),
    };
    W { body, start: a.start, end: a.end }
}

fn agreed<T: Transport>(t: &T, table: &Table, method: &str, params: &Value) -> R<Reading> {
    agreed_rows(t, table, method, params).map(|(r, _)| r)
}

fn agreed_rows<T: Transport>(t: &T, table: &Table, method: &str, params: &Value) -> R<(Reading, Vec<String>)> {
    crate::ask::scene();
    let pauses = crate::judge::not_yet_pauses();
    let mut round = 0usize;
    let mut last = false;
    loop {
        let nodes = table.nodes();
        let came: Vec<(NodeAt, Asked)> = nodes.into_iter().zip(ask_each(t, &table.nodes(), method, params)).filter_map(|(n, a)| a.map(|a| (n, a))).collect();
        let faults: Vec<(NodeAt, Option<Fault>)> = came.iter().map(|(n, (_, f))| (n.clone(), f.clone())).collect();
        let mut answers: Vec<(String, Result<W, Trouble>)> = came.into_iter().map(|(n, (r, _))| (n.written().to_string(), r)).collect();
        shared_below_top(&mut answers);
        let mut judged = crate::judge::judge(method, params, answers.clone());
        if let Err(NoReading::NotYet { not_yet, .. }) = &judged {
            if !crate::judge::NOT_YET_METHODS.contains(&method) {
                if last || round >= pauses.len() {
                    judged = crate::judge::judge_settled(method, params, answers.iter().cloned().map(|(n, a)| (n, a.map_err(Missing::from))).collect());
                } else if not_yet.iter().all(|n| crate::patience::past_it(&mut Node::new(t, NodeAt::new(n.clone())), method, params)) {
                    last = true;
                    continue;
                }
            }
        }
        match judged {
            Ok(j) => {
                let rows = j.sources.clone();
                return Ok((reading_of(j, &faults), rows));
            }
            Err(NoReading::Differ(d)) => return Err(Fault::Disagree(differ(method, &d))),
            Err(NoReading::NoneAnswered(each)) => {
                let failed = each.into_iter().map(|(name, m)| {
                    let node = NodeAt::new(name);
                    let (_, f) = missing_of(&node, m, &faults);
                    (node, f)
                });
                return Err(crate::ask::none_answered(failed.collect()));
            }
            Err(NoReading::NotYet { has, not_yet }) => match pauses.get(round) {
                Some(p) => {
                    if !p.is_zero() {
                        std::thread::sleep(*p);
                    }
                    round += 1;
                }
                None => {
                    let has: Vec<NodeAt> = has.into_iter().map(NodeAt::new).collect();
                    let not_yet: Vec<NodeAt> = not_yet.into_iter().map(NodeAt::new).collect();
                    return not_yet_split(method, params, answers, &faults, has, not_yet).map(|r| (r, Vec::new()));
                }
            },
        }
    }
}

fn not_yet_split(method: &str, _params: &Value, _answers: Vec<(String, Result<W, Trouble>)>, _faults: &[(NodeAt, Option<Fault>)], has: Vec<NodeAt>, not_yet: Vec<NodeAt>) -> R<Reading> {
    Err(Fault::Disagree(format!("not-yet {method} 一家有、一家还没有,重问之后仍分裂:{} 有;{} 还没有", said_all(&has), said_all(&not_yet))))
}


pub fn read_agreed<T: Transport>(
    t: &T,
    table: &Table,
    method: &'static str,
    params: Vec<Out>,
) -> R<Reading> {
    mark::mark(mark::FEATURE);
    if let Some(kept) = crate::recall::answer(method, &crate::recall::params_text(&params)) {
        return kept;
    }
    if table.is_empty() {
        return Err(Fault::Usage("端点表是空的".into()));
    }
    let cut = pinned(t, table, method)?;
    let r = read_value(t, cut.as_ref().unwrap_or(table), method, &crate::ask::params_of(&params))?;
    said_unchecked(table, cut.as_ref(), r)
}

fn said_unchecked(table: &Table, cut: Option<&Table>, r: Reading) -> R<Reading> {
    if r.agreement != Agreement::Single || table.chain.get().is_some() || crate::recall::current().is_some() {
        return Ok(r);
    }
    let read = cut.unwrap_or(table);
    let id = table_chain(table);
    let on = rows_on(read, id);
    let from: Vec<&String> = read.list.iter().filter(|e| !r.left.iter().any(|(n, _)| n.written() == e.as_str())).collect();
    match from.as_slice() {
        [one] if !on.contains(one) => Err(Fault::Unread(format!("{UNANSWERED}{one} 还没答过链号,它独自答的读不当成链 {id} 的"))),
        _ => Ok(r),
    }
}

fn read_value<T: Transport>(t: &T, table: &Table, method: &str, params: &Value) -> R<Reading> {
    let params = &pin_tags(t, table, params)?;
    match covered_of(method) {
        Some(c) => agreed_on(t, table, method, params, c),
        None => agreed(t, table, method, params),
    }
}

pub fn read_agreed_raw<T: Transport>(
    t: &T,
    table: &Table,
    method: &'static str,
    params_json: &str,
) -> R<Reading> {
    if let Some(kept) = crate::recall::answer(method, params_json) {
        return kept;
    }
    if table.is_empty() {
        return Err(Fault::Usage("端点表是空的".into()));
    }
    let cut = pinned(t, table, method)?;
    let r = read_value(t, cut.as_ref().unwrap_or(table), method, &crate::ask::params_text(params_json)?)?;
    said_unchecked(table, cut.as_ref(), r)
}

pub fn read_agreed_value<T: Transport>(t: &T, table: &Table, method: &'static str, params: &Value) -> R<Reading> {
    read_agreed_raw(t, table, method, &crate::wire::write(&crate::wire::from_core(params)))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderForm {
    Quantity,
    Data,
}

pub const HEADER_MEMBERS: [(&str, HeaderForm); 23] = [
    ("parentHash", HeaderForm::Data),
    ("sha3Uncles", HeaderForm::Data),
    ("miner", HeaderForm::Data),
    ("stateRoot", HeaderForm::Data),
    ("transactionsRoot", HeaderForm::Data),
    ("receiptsRoot", HeaderForm::Data),
    ("logsBloom", HeaderForm::Data),
    ("difficulty", HeaderForm::Quantity),
    ("number", HeaderForm::Quantity),
    ("gasLimit", HeaderForm::Quantity),
    ("gasUsed", HeaderForm::Quantity),
    ("timestamp", HeaderForm::Quantity),
    ("extraData", HeaderForm::Data),
    ("mixHash", HeaderForm::Data),
    ("nonce", HeaderForm::Data),
    ("baseFeePerGas", HeaderForm::Quantity),
    ("withdrawalsRoot", HeaderForm::Data),
    ("blobGasUsed", HeaderForm::Quantity),
    ("excessBlobGas", HeaderForm::Quantity),
    ("parentBeaconBlockRoot", HeaderForm::Data),
    ("requestsHash", HeaderForm::Data),
    ("blockAccessListHash", HeaderForm::Data),
    ("slotNumber", HeaderForm::Quantity),
];

pub const HEADER_ALWAYS: usize = 15;

pub const BESIDE_HEADER: [&str; 6] = ["hash", "size", "transactions", "uncles", "withdrawals", "totalDifficulty"];

fn header_part(v: &In) -> In {
    match v {
        In::Obj(m) => In::Obj(m.iter().filter(|(k, _)| k == "hash" || k == "transactions" || HEADER_MEMBERS.iter().any(|(h, _)| h == k)).cloned().collect()),
        other => other.clone(),
    }
}

pub const RECEIPT_COVERED: [&str; 6] = ["type", "status", "cumulativeGasUsed", "logsBloom", "logs", "transactionHash"];

pub const LOG_COVERED: [&str; 3] = ["address", "topics", "data"];

pub const PROOF_COVERED: [&str; 1] = ["accountProof"];

fn facts_of(v: &In, facts: &[&str]) -> In {
    match v {
        In::Obj(m) => In::Obj(m.iter().filter(|(k, _)| facts.contains(&k.as_str())).cloned().collect()),
        other => other.clone(),
    }
}

fn receipts_covered(v: &In) -> In {
    let receipt = |r: &In| match facts_of(r, &RECEIPT_COVERED) {
        In::Obj(m) => In::Obj(
            m.into_iter()
                .map(|(k, x)| match (k.as_str(), &x) {
                    ("logs", In::Arr(ls)) => (k, In::Arr(ls.iter().map(|l| facts_of(l, &LOG_COVERED)).collect())),
                    _ => (k, x),
                })
                .collect(),
        ),
        other => other,
    };
    match v {
        In::Arr(list) => In::Arr(list.iter().map(receipt).collect()),
        other => other.clone(),
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Covered {
    part: fn(&In) -> In,
    whole: bool,
    noun: &'static str,
}

pub(crate) fn covered_of(method: &str) -> Option<Covered> {
    match method {
        seam::M_GET_BLOCK_BY_HASH => Some(Covered { part: header_part, whole: true, noun: "区块头" }),
        seam::M_GET_BLOCK_RECEIPTS => Some(Covered { part: receipts_covered, whole: false, noun: "回执" }),
        seam::M_GET_PROOF => Some(Covered { part: |v| facts_of(v, &PROOF_COVERED), whole: false, noun: "账户证明" }),
        _ => None,
    }
}

fn where_differ(a: &In, b: &In, at: &str) -> Option<String> {
    let whole = || if at.is_empty() { "整份".to_string() } else { at.to_string() };
    match (a, b) {
        (In::Obj(x), In::Obj(y)) => {
            let mut keys: Vec<&str> = x.iter().chain(y).map(|(k, _)| k.as_str()).collect();
            keys.sort_unstable();
            keys.dedup();
            keys.into_iter().find_map(|k| {
                let p = if at.is_empty() { k.to_string() } else { format!("{at}.{k}") };
                match (a.get(k), b.get(k)) {
                    (Some(u), Some(v)) => where_differ(u, v, &p),
                    _ => Some(p),
                }
            })
        }
        (In::Arr(x), In::Arr(y)) if x.len() == y.len() => x.iter().zip(y).enumerate().find_map(|(i, (u, v))| where_differ(u, v, &format!("{at}[{i}]"))),
        (In::Arr(x), In::Arr(y)) => Some(format!("{}(长 {} 对 {})", whole(), y.len(), x.len())),
        _ => (crate::ask::fingerprint(a) != crate::ask::fingerprint(b)).then(whole),
    }
}

fn agreed_on<T: Transport>(t: &T, table: &Table, method: &str, params: &Value, c: Covered) -> R<Reading> {
    crate::ask::scene();
    let pauses = crate::judge::not_yet_pauses();
    let mut round = 0usize;
    let mut last = false;
    loop {
        let nodes = table.nodes();
        let (tt, m, p, part, whole) = (t.clone(), method.to_string(), params.clone(), c.part, c.whole);
        let seen = nodes.clone();
        let asked = asked_until(
            nodes.clone(),
            move |n| {
                let got = if whole { crate::ask::ask_whole(&tt, &n, &m, &p) } else { crate::ask::ask_value(&tt, &n, &m, &p) };
                got.map(|v| (part(&v), v))
            },
            move |view: &[Option<&R<(In, In)>>]| {
                let read: Vec<(&str, Vec<u8>)> = seen.iter().zip(view).filter_map(|(n, g)| match g {
                    Some(Ok((pt, _))) if !matches!(pt, In::Null) => Some((n.written(), crate::ask::fingerprint(pt).into_bytes())),
                    _ => None,
                }).collect();
                crate::judge::two_places_alike(&read)
            },
        );
        let mut got: Vec<(NodeAt, In, In)> = Vec::new();
        let mut failed: Vec<(NodeAt, Fault)> = Vec::new();
        for (n, r) in nodes.into_iter().zip(asked) {
            match r {
                Some(Ok((pt, v))) => got.push((n, pt, v)),
                Some(Err(f)) => failed.push((n, f)),
                None => {}
            }
        }
        if got.is_empty() {
            return Err(crate::ask::none_answered(failed));
        }
        let judged_as = if method == seam::M_GET_BLOCK { seam::M_GET_BLOCK_BY_HASH } else { method };
        let answers: Vec<(String, Result<W, Missing>)> = got.iter().map(|(n, pt, _)| (n.written().to_string(), Ok(crate::wire::from_core(&crate::ask::core_of(pt).unwrap_or(Value::Null))))).collect();
        let mut judged = crate::judge::judge_read(judged_as, params, answers.clone());
        if let Err(NoReading::NotYet { not_yet, .. }) = &judged {
            if last || round >= pauses.len() {
                judged = crate::judge::judge_settled(judged_as, params, answers);
            } else if not_yet.iter().all(|n| crate::patience::past_it(&mut Node::new(t, NodeAt::new(n.clone())), method, params)) {
                last = true;
                continue;
            } else {
                if !pauses[round].is_zero() {
                    std::thread::sleep(pauses[round]);
                }
                round += 1;
                continue;
            }
        }
        return match judged {
            Ok(j) => {
                let lead = j.sources.first().cloned().unwrap_or_default();
                let (pt, v) = got.iter().find(|(n, _, _)| n.written() == lead).map(|(_, pt, v)| (pt.clone(), v.clone())).unwrap_or((In::Null, In::Null));
                let left = if j.single_source { failed.iter().map(|(n, f)| (n.clone(), format!("{UNANSWERED}{}:{}", f.name(), f.detail()))).collect() } else { Vec::new() };
                let n = crate::endpoints::distinct_places(j.sources.iter().map(String::as_str));
                Ok(Reading { value: if whole { v } else { pt }, agreement: agreement_of(n), endpoints: n, left })
            }
            Err(NoReading::Differ(d)) => {
                let what = match params {
                    Value::Arr(ps) => ps.first().and_then(Value::as_str).unwrap_or(""),
                    _ => "",
                };
                let part_of = |name: &str| got.iter().find(|(n, _, _)| n.written() == name).map(|(n, pt, _)| (n.clone(), pt.clone()));
                let Some((lead, first)) = d.sources.first().and_then(|s| part_of(s)) else { return Err(Fault::Disagree(format!("{method} {what} 的{}各家说法不一", c.noun))) };
                let off: Vec<String> = d.sources.iter().skip(1).filter_map(|s| part_of(s)).filter_map(|(n, pt)| where_differ(&first, &pt, "").map(|at| format!("{} 在 {at}", n.said()))).collect();
                Err(Fault::Disagree(format!("{method} {what} 的{}各家说法不一:以 {} 为准,{}", c.noun, lead.said(), off.join(";"))))
            }
            Err(_) => Err(crate::ask::none_answered(failed)),
        };
    }
}

pub fn held_alike(table: &Table, method: &str, params: &[Out], member: &str) -> Option<String> {
    let p = crate::ask::params_of(params);
    let mut out: Option<String> = None;
    for n in table.nodes() {
        let w = crate::action::answered(n.written(), method, &p)?;
        let v = w.member(member).and_then(|x| x.as_str()).map(str::to_ascii_lowercase)?;
        match &out {
            Some(o) if *o != v => return None,
            _ => out = Some(v),
        }
    }
    out
}

pub fn block_by_hash<T: Transport>(t: &T, table: &Table, hash: &str) -> R<(In, Reading)> {
    let r = read_agreed_raw(t, table, seam::M_GET_BLOCK_BY_HASH, &format!("[\"{}\",false]", hash.to_ascii_lowercase()))?;
    Ok((r.value.clone(), r))
}

pub fn block_by_number<T: Transport>(t: &T, table: &Table, n: u64) -> R<(In, Reading)> {
    let params = format!("[\"{}\",false]", seam::quantity(n));
    if let Some(kept) = crate::recall::answer(seam::M_GET_BLOCK, &params) {
        let r = kept?;
        return Ok((r.value.clone(), r));
    }
    if table.is_empty() {
        return Err(Fault::Usage("端点表是空的".into()));
    }
    let cut = pinned(t, table, seam::M_GET_BLOCK)?;
    let table = cut.as_ref().unwrap_or(table);
    let header = Covered { part: header_part, whole: true, noun: "区块头" };
    let r = agreed_on(t, table, seam::M_GET_BLOCK, &crate::ask::params_text(&params)?, header)?;
    Ok((r.value.clone(), r))
}

pub fn raw_transaction<T: Transport>(t: &T, table: &Table, hash: &str) -> R<(String, Reading)> {
    let r = read_agreed_raw(t, table, seam::M_GET_RAW_TX, &format!("[\"{}\"]", hash.to_ascii_lowercase()))?;
    Ok((as_str(&r)?, r))
}

pub fn block_receipts<T: Transport>(t: &T, table: &Table, block_hash: &str) -> R<(Vec<In>, Reading)> {
    let r = read_agreed_raw(t, table, seam::M_GET_BLOCK_RECEIPTS, &format!("[\"{}\"]", block_hash.to_ascii_lowercase()))?;
    let list = r.value.as_arr().map(<[In]>::to_vec).ok_or_else(|| Fault::Endpoint(format!("块 {block_hash} 的回执不是一列")))?;
    Ok((list, r))
}

pub fn account_proof<T: Transport>(t: &T, table: &Table, addr: &str, block: u64) -> R<(In, Reading)> {
    seam::need_address(addr)?;
    let r = read_agreed_raw(t, table, seam::M_GET_PROOF, &format!("[\"{}\",[],\"{}\"]", addr.to_ascii_lowercase(), seam::quantity(block)))?;
    let p = r.value.get("accountProof").cloned().ok_or_else(|| Fault::Endpoint(format!("{addr} 在块 {block} 的证明答里没有 accountProof")))?;
    Ok((p, r))
}

pub fn as_str(r: &Reading) -> R<String> {
    r.value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| Fault::Endpoint(format!("答该是一串文本,给的是 {:?}", r.value)))
}

pub fn chain_guard<T: Transport>(t: &T, table: &Table, expect: u64) -> R<(u64, Reading)> {
    let (r, rows) = chain_id_read(t, table)?;
    let id = seam::read_quantity(&as_str(&r)?)?;
    if id != expect {
        return Err(Fault::WrongChain(format!(
            "端点表说的是链 {id},本趟认的是链 {expect}"
        )));
    }
    eravon_literals::constants::select_chain(id);
    remember(table, &rows, id);
    Ok((id, r))
}

fn chain_id_read<T: Transport>(t: &T, table: &Table) -> R<(Reading, Vec<String>)> {
    mark::mark(mark::FEATURE);
    if let Some(kept) = crate::recall::answer(seam::M_CHAIN_ID, &crate::recall::params_text(&[])) {
        return kept.map(|r| (r, Vec::new()));
    }
    if table.is_empty() {
        return Err(Fault::Usage("端点表是空的".into()));
    }
    agreed_rows(t, table, seam::M_CHAIN_ID, &Value::Arr(Vec::new()))
}

fn pinned<T: Transport>(t: &T, table: &Table, method: &str) -> R<Option<Table>> {
    if method == seam::M_CHAIN_ID || table.chain.get().is_some() {
        return Ok(None);
    }
    if let Some(done) = checked_whole(table) {
        return Ok(done);
    }
    let one = checking(table);
    let _held = one.lock();
    if let Some(done) = checked_whole(table) {
        return Ok(done);
    }
    pinned_chain(t, table)?;
    Ok(narrowed(table))
}

fn checked_whole(table: &Table) -> Option<Option<Table>> {
    let (id, n) = checked(table)?;
    (n == table.list.len()).then(|| {
        let _ = table.chain.set(id);
        None
    })
}

fn narrowed(table: &Table) -> Option<Table> {
    let (id, n) = checked(table)?;
    if n == table.list.len() {
        let _ = table.chain.set(id);
        return None;
    }
    let g = CHECKED.lock().unwrap_or_else(|e| e.into_inner());
    let rows: Vec<String> = table.list.iter().filter(|e| !g.iter().any(|(k, c)| k == *e && *c != id)).cloned().collect();
    drop(g);
    if rows.len() == table.list.len() {
        return None;
    }
    let cut = Table { list: rows.clone(), trace: table.trace.iter().filter(|(e, _)| rows.contains(e)).cloned().collect(), chain: std::sync::OnceLock::new() };
    let _ = cut.chain.set(id);
    Some(cut)
}

fn checking(table: &Table) -> std::sync::Arc<std::sync::Mutex<()>> {
    static LOCKS: std::sync::Mutex<Vec<(String, std::sync::Arc<std::sync::Mutex<()>>)>> = std::sync::Mutex::new(Vec::new());
    let key = table.list.join("\n");
    let mut g = LOCKS.lock().unwrap_or_else(|e| e.into_inner());
    match g.iter().find(|(k, _)| *k == key) {
        Some((_, l)) => l.clone(),
        None => {
            let l = std::sync::Arc::new(std::sync::Mutex::new(()));
            g.push((key, l.clone()));
            l
        }
    }
}

fn accepted(id: u64) -> bool {
    id == eravon_literals::constants::current_network().standard.chain_id || id == seam::BENCH_CHAIN_ID
}


static CHECKED: std::sync::Mutex<Vec<(String, u64)>> = std::sync::Mutex::new(Vec::new());

fn rows_on(table: &Table, id: u64) -> Vec<String> {
    let g = CHECKED.lock().unwrap_or_else(|e| e.into_inner());
    table.list.iter().filter(|e| g.iter().any(|(k, c)| k == *e && *c == id)).cloned().collect()
}

fn checked(table: &Table) -> Option<(u64, usize)> {
    let g = CHECKED.lock().ok()?;
    let mut best: Option<(u64, usize)> = None;
    for (k, c) in g.iter().filter(|(k, c)| table.list.contains(k) && accepted(*c)) {
        let _ = k;
        let n = table.list.iter().filter(|e| g.iter().any(|(x, y)| x == *e && y == c)).count();
        if best.is_none_or(|(_, m)| n > m) {
            best = Some((*c, n));
        }
    }
    best
}


pub fn covered(table: &Table) -> bool {
    checked(table).is_some_and(|(_, n)| n == table.list.len())
}

pub fn table_chain(table: &Table) -> u64 {
    table.chain.get().copied().or_else(|| checked(table).map(|(id, _)| id)).unwrap_or_else(|| eravon_literals::constants::current_network().standard.chain_id)
}

fn heard(row: &str, w: &W) {
    if crate::recall::current().is_some() {
        return;
    }
    let Some(id) = crate::wire::to_core(w).and_then(|v| v.as_str().map(str::to_string)).and_then(|q| seam::read_quantity(&q).ok()) else { return };
    if let Ok(mut g) = CHECKED.lock() {
        g.retain(|(k, _)| k != row);
        g.push((row.to_string(), id));
    }
}

fn remember(table: &Table, rows: &[String], id: u64) {
    if crate::recall::current().is_some() {
        return;
    }
    if let Ok(mut g) = CHECKED.lock() {
        for r in rows {
            g.retain(|(k, _)| k != r);
            g.push((r.clone(), id));
        }
    }
    if table.list.iter().all(|e| rows.contains(e) || rows_on(table, id).contains(e)) {
        let _ = table.chain.set(id);
    }
}

pub fn forget_checked() {
    if let Ok(mut g) = CHECKED.lock() {
        g.clear();
    }
    crate::trace::forget();
}

fn ahead_chain_and_head<T: Transport>(t: &T, table: &Table) {
    if table.is_empty() || cfg!(feature = "unpinned-tags") {
        return;
    }
    ask_ahead_mixed(t, table, &[(seam::M_CHAIN_ID, "[]".to_string()), (seam::M_BLOCK_NUMBER, "[]".to_string())]);
}

pub fn pinned_chain<T: Transport>(t: &T, table: &Table) -> R<(u64, Reading)> {
    ahead_chain_and_head(t, table);
    let (r, rows) = chain_id_read(t, table)?;
    let id = seam::read_quantity(&as_str(&r)?)?;
    let home = eravon_literals::constants::current_network();
    if !accepted(id) {
        return Err(Fault::WrongChain(format!("节点在链 {id},所选的网在链 {}", home.standard.chain_id)));
    }
    remember(table, &rows, id);
    Ok((id, r))
}


pub fn balance<T: Transport>(t: &T, table: &Table, addr: &str, at: &str) -> R<(String, Reading)> {
    seam::need_address(addr)?;
    let r = read_agreed(
        t,
        table,
        seam::M_GET_BALANCE,
        vec![Out::S(addr.to_string()), Out::S(at.to_string())],
    )?;
    let dec = seam::read_big_decimal(&as_str(&r)?)?;
    Ok((dec, r))
}

pub fn call<T: Transport>(
    t: &T,
    table: &Table,
    to: &str,
    data: &str,
    at: &str,
) -> R<(String, Reading)> {
    seam::need_address(to)?;
    seam::need_hex_bytes("calldata", data)?;
    let r = read_agreed(
        t,
        table,
        seam::M_CALL,
        vec![
            Out::O(vec![("data", Out::S(data.to_string())), ("to", Out::S(to.to_string()))]),
            Out::S(at.to_string()),
        ],
    )?;
    let out = as_str(&r)?;
    Ok((out, r))
}

pub fn call_from<T: Transport>(
    t: &T,
    table: &Table,
    from: &str,
    to: &str,
    data: &str,
    at: &str,
) -> R<(String, Reading)> {
    seam::need_address(from)?;
    seam::need_address(to)?;
    seam::need_hex_bytes("calldata", data)?;
    let r = read_agreed(
        t,
        table,
        seam::M_CALL,
        vec![
            Out::O(vec![("data", Out::S(data.to_string())), ("from", Out::S(from.to_string())), ("to", Out::S(to.to_string()))]),
            Out::S(at.to_string()),
        ],
    )?;
    let out = as_str(&r)?;
    Ok((out, r))
}

pub fn code<T: Transport>(t: &T, table: &Table, addr: &str, at: &str) -> R<(Vec<u8>, Reading)> {
    seam::need_address(addr)?;
    let r = read_agreed(
        t,
        table,
        seam::M_GET_CODE,
        vec![Out::S(addr.to_string()), Out::S(at.to_string())],
    )?;
    let hex = as_str(&r)?;
    let body = hex
        .strip_prefix("0x")
        .ok_or_else(|| Fault::Endpoint(format!("运行码该以 0x 起头:{hex:?}")))?;
    if body.len() % 2 != 0 || !body.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Fault::Endpoint(format!("运行码不是偶数位十六进制:{hex:?}")));
    }
    let mut out = Vec::with_capacity(body.len() / 2);
    let b = body.as_bytes();
    for p in b.chunks(2) {
        let v = |c: u8| (c as char).to_digit(16).unwrap_or(0) as u8;
        out.push((v(p[0]) << 4) | v(p[1]));
    }
    Ok((out, r))
}

fn proven_alone<T: Transport, X>(
    t: &T,
    table: &Table,
    method: &'static str,
    what: &str,
    ask: impl Fn(&NodeAt) -> R<X>,
    proof: impl Fn(&X) -> Result<(), String>,
) -> R<X> {
    mark::mark(mark::FEATURE);
    if let Some(f) = crate::recall::refuse(method) {
        return Err(f);
    }
    if table.is_empty() {
        return Err(Fault::Usage("端点表是空的".into()));
    }
    let cut = pinned(t, table, method)?;
    let table = cut.as_ref().unwrap_or(table);
    crate::ask::scene();
    let mut failed: Vec<(NodeAt, Fault)> = Vec::new();
    let mut wrong: Vec<String> = Vec::new();
    for n in table.nodes() {
        match ask(&n) {
            Ok(x) => match proof(&x) {
                Ok(()) => return Ok(x),
                Err(why) => wrong.push(format!("{} 给的{what}{why}", n.said())),
            },
            Err(f) => failed.push((n, f)),
        }
    }
    if wrong.is_empty() {
        return Err(crate::ask::none_answered(failed));
    }
    let silent: Vec<String> = failed.iter().map(|(n, f)| format!("{n} {}:{}", f.name(), f.detail())).collect();
    Err(Fault::Endpoint(format!("{what}没有一家给得对:{}", wrong.into_iter().chain(silent).collect::<Vec<_>>().join(";"))))
}

pub fn unbatched() -> Vec<(String, String)> {
    crate::ahead::unbatched()
}

pub fn ask_ahead<T: Transport>(t: &T, table: &Table, method: &'static str, params: &[String]) {
    let asks: Vec<(&'static str, String)> = params.iter().map(|p| (method, p.clone())).collect();
    ask_ahead_mixed(t, table, &asks);
}

pub fn ask_ahead_mixed<T: Transport>(t: &T, table: &Table, asks: &[(&'static str, String)]) {
    if gathering(asks) {
        return;
    }
    if asks.len() < 2 || crate::action::current().is_none() || crate::recall::current().is_some() {
        return;
    }
    let mut values: Vec<(&'static str, Value)> = Vec::with_capacity(asks.len());
    for (m, p) in asks {
        let Ok(v) = crate::ask::params_text(p) else { return };
        if !values.iter().any(|(x, w)| x == m && *w == v) {
            values.push((m, v));
        }
    }
    if values.len() < 2 {
        return;
    }
    let nodes = table.nodes();
    let (tt, seen) = (t.clone(), nodes.clone());
    let _ = asked_until(
        nodes,
        move |n| {
            let mut node = Node::new(&tt, n);
            crate::ahead::ask_ahead(&mut node, &values);
        },
        move |view: &[Option<&()>]| two_places(&seen, view, |_| true),
    );
}

thread_local! {
    static GATHER: std::cell::RefCell<Option<Vec<(&'static str, String)>>> = const { std::cell::RefCell::new(None) };
}

fn gathering(asks: &[(&'static str, String)]) -> bool {
    GATHER.with(|g| match g.borrow_mut().as_mut() {
        Some(kept) => {
            kept.extend(asks.iter().cloned());
            true
        }
        None => false,
    })
}

pub fn together<T: Transport, X>(t: &T, table: &Table, work: impl FnOnce() -> X) -> X {
    if GATHER.with(|g| g.borrow().is_some()) {
        return work();
    }
    GATHER.with(|g| *g.borrow_mut() = Some(Vec::new()));
    struct Clear;
    impl Drop for Clear {
        fn drop(&mut self) {
            GATHER.with(|g| *g.borrow_mut() = None);
        }
    }
    let clear = Clear;
    let x = work();
    let asks = GATHER.with(|g| g.borrow_mut().take()).unwrap_or_default();
    drop(clear);
    ask_ahead_mixed(t, table, &asks);
    x
}

pub fn ask_ahead_raw<T: Transport>(t: &T, table: &Table, method: &'static str, params: &[String]) {
    if params.len() < 2 || crate::action::current().is_none() || crate::recall::current().is_some() {
        return;
    }
    let pinned = distinct(params.iter().filter_map(|p| {
        let v = pin_tags(t, table, &crate::ask::params_text(p).ok()?).ok()?;
        let Value::Arr(ps) = &v else { return None };
        if ps.iter().any(|x| matches!(x, Value::Str(s) if moving(s))) {
            return None;
        }
        Some(crate::wire::write(&crate::wire::from_core(&v)))
    }));
    ask_ahead(t, table, method, &pinned);
}

pub fn asked_at<T: Transport>(t: &T, table: &Table, at: &str) -> String {
    match pin_tags(t, table, &Value::Arr(vec![Value::Str(at.to_string())])) {
        Ok(Value::Arr(v)) => match v.first() {
            Some(Value::Str(s)) => s.clone(),
            _ => at.to_string(),
        },
        _ => at.to_string(),
    }
}

fn ahead_at<T: Transport>(t: &T, table: &Table, at: &str) -> Option<String> {
    let at = asked_at(t, table, at);
    (at.len() > 2 && at.starts_with("0x") && at[2..].bytes().all(|b| b.is_ascii_hexdigit())).then_some(at)
}

pub fn holds_ahead(table: &Table) -> bool {
    crate::action::current().is_some() && crate::recall::current().is_none() && !table.is_empty() && !cfg!(feature = "unpinned-tags")
}

fn distinct(items: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for i in items {
        if !out.contains(&i) {
            out.push(i);
        }
    }
    out
}

pub fn ahead_codes<T: Transport>(t: &T, table: &Table, addrs: &[String], at: &str) {
    if addrs.is_empty() {
        return;
    }
    let Some(at) = ahead_at(t, table, at) else { return };
    ask_ahead(t, table, seam::M_GET_CODE, &distinct(addrs.iter().map(|a| format!("[\"{a}\",\"{at}\"]"))));
}

pub fn ahead_codes_by<T: Transport>(t: &T, table: &Table, wants: &[(String, u64)]) {
    ask_ahead(t, table, seam::M_GET_CODE, &distinct(wants.iter().map(|(a, b)| format!("[\"{a}\",\"{}\"]", seam::quantity(*b)))));
}

pub fn ahead_balances<T: Transport>(t: &T, table: &Table, addrs: &[String], at: &str) {
    if addrs.is_empty() {
        return;
    }
    let Some(at) = ahead_at(t, table, at) else { return };
    ask_ahead(t, table, seam::M_GET_BALANCE, &distinct(addrs.iter().map(|a| format!("[\"{a}\",\"{at}\"]"))));
}

pub fn ahead_calls<T: Transport>(t: &T, table: &Table, from: Option<&str>, calls: &[(String, String)], at: &str) {
    if calls.is_empty() {
        return;
    }
    let Some(at) = ahead_at(t, table, at) else { return };
    let one = |(to, data): &(String, String)| match from {
        Some(f) => format!("[{{\"data\":\"{data}\",\"from\":\"{f}\",\"to\":\"{to}\"}},\"{at}\"]"),
        None => format!("[{{\"data\":\"{data}\",\"to\":\"{to}\"}},\"{at}\"]"),
    };
    ask_ahead(t, table, seam::M_CALL, &distinct(calls.iter().map(one)));
}

pub fn ahead_reads<T: Transport>(t: &T, table: &Table, codes: &[String], balances: &[String], calls: &[(String, String)], at: &str) {
    if codes.len() + balances.len() + calls.len() == 0 || !holds_ahead(table) {
        return;
    }
    let Some(at) = ahead_at(t, table, at) else { return };
    let mut asks: Vec<(&'static str, String)> = Vec::new();
    asks.extend(distinct(codes.iter().map(|a| format!("[\"{a}\",\"{at}\"]"))).into_iter().map(|p| (seam::M_GET_CODE, p)));
    asks.extend(distinct(balances.iter().map(|a| format!("[\"{a}\",\"{at}\"]"))).into_iter().map(|p| (seam::M_GET_BALANCE, p)));
    asks.extend(distinct(calls.iter().map(|(to, data)| format!("[{{\"data\":\"{data}\",\"to\":\"{to}\"}},\"{at}\"]"))).into_iter().map(|p| (seam::M_CALL, p)));
    ask_ahead_mixed(t, table, &asks);
}

pub fn ahead_blocks<T: Transport>(t: &T, table: &Table, numbers: &[u64]) {
    ask_ahead(t, table, seam::M_GET_BLOCK, &distinct(numbers.iter().map(|n| format!("[\"{}\",false]", seam::quantity(*n)))));
}

pub fn ahead_transactions<T: Transport>(t: &T, table: &Table, hashes: &[String]) {
    ask_ahead(t, table, seam::M_GET_TX, &distinct(hashes.iter().map(|h| format!("[\"{h}\"]"))));
}

pub fn ahead_receipts<T: Transport>(t: &T, table: &Table, hashes: &[String]) {
    ask_ahead(t, table, seam::M_GET_RECEIPT, &distinct(hashes.iter().map(|h| format!("[\"{h}\"]"))));
}

pub fn side_by_side<I: Send + 'static, X: Send + 'static>(items: Vec<I>, work: impl Fn(I) -> X + Send + Sync + 'static) -> Vec<X> {
    let work = std::sync::Arc::new(work);
    let mut out: Vec<X> = Vec::with_capacity(items.len());
    let mut rest = items.into_iter().peekable();
    while rest.peek().is_some() {
        let part: Vec<I> = rest.by_ref().take(seam::ITEMS_AT_ONCE).collect();
        let w = work.clone();
        let got = crate::endpoints::until_enough(part, move |i| Some(w(i)), || None, crate::endpoints::every);
        out.extend(got.into_iter().map(|x| x.flatten().expect("一项的读没做完就停了")));
    }
    out
}

pub fn both<A: Send + 'static, B: Send + 'static>(a: impl FnOnce() -> A + Send + 'static, b: impl FnOnce() -> B + Send + 'static) -> (A, B) {
    enum One<A, B> {
        A(A),
        B(B),
    }
    type Job<A, B> = Box<dyn FnOnce() -> One<A, B> + Send>;
    let jobs: Vec<Job<A, B>> = vec![Box::new(move || One::A(a())), Box::new(move || One::B(b()))];
    let mut ra = None;
    let mut rb = None;
    for got in side_by_side(jobs, |j| j()) {
        match got {
            One::A(x) => ra = Some(x),
            One::B(x) => rb = Some(x),
        }
    }
    (ra.expect("一项的读没做完就停了"), rb.expect("一项的读没做完就停了"))
}

pub fn block_transactions_proven<T: Transport>(t: &T, table: &Table, hashes: &[String], proof: impl Fn(&[Vec<u8>]) -> Result<(), String>) -> R<Vec<Vec<u8>>> {
    proven_alone(t, table, seam::M_GET_RAW_TX, "交易原文", |n| raws_of(t, n, hashes), |x: &Vec<Vec<u8>>| proof(x))
}

pub fn block_transactions_whole<T: Transport>(
    t: &T,
    table: &Table,
    block_hash: &str,
    hashes: &[String],
    rebuild: impl Fn(&In) -> Option<Vec<u8>> + Sync,
    proof: impl Fn(&[Vec<u8>]) -> Result<(), String>,
) -> R<Vec<Vec<u8>>> {
    let hash_of = |raw: &[u8]| eravon::hexfmt::encode(&eravon::cryptox::keccak256(raw));
    let params = format!("[\"{}\",true]", block_hash.to_ascii_lowercase());
    proven_alone(
        t,
        table,
        seam::M_GET_RAW_TX,
        "交易原文",
        |n| {
            let whole = crate::ask::ask_whole(t, n, seam::M_GET_BLOCK_BY_HASH, &crate::ask::params_text(&params)?);
            let objects: Vec<In> = match whole.as_ref().ok().and_then(|b| b.get("transactions")).and_then(In::as_arr) {
                Some(list) if list.len() == hashes.len() => list.to_vec(),
                _ => return raws_of(t, n, hashes),
            };
            let mut got: Vec<Option<Vec<u8>>> = objects
                .iter()
                .zip(hashes)
                .map(|(o, h)| rebuild(o).filter(|raw| hash_of(raw).eq_ignore_ascii_case(h)))
                .collect();
            let left: Vec<String> = hashes.iter().zip(&got).filter(|(_, g)| g.is_none()).map(|(h, _)| h.clone()).collect();
            if !left.is_empty() {
                let mut asked = raws_of(t, n, &left)?.into_iter();
                for g in got.iter_mut().filter(|g| g.is_none()) {
                    *g = asked.next();
                }
            }
            let raws: Vec<Vec<u8>> = got.into_iter().collect::<Option<_>>().ok_or_else(|| Fault::Endpoint(format!("{n} 的交易原文没有取齐")))?;
            if proof(&raws).is_ok() {
                return Ok(raws);
            }
            raws_of(t, n, hashes)
        },
        |x: &Vec<Vec<u8>>| proof(x),
    )
}

fn raws_of<T: Transport>(t: &T, n: &NodeAt, hashes: &[String]) -> R<Vec<Vec<u8>>> {
    let one = |h: &str, s: &str| seam::bytes_of(&format!("交易 {h} 的原文"), s);
    let param = |h: &str| format!("[\"{}\"]", h.to_ascii_lowercase());
    let asks: Vec<(&str, Value)> = hashes.iter().filter_map(|h| Some((seam::M_GET_RAW_TX, crate::ask::params_text(&param(h)).ok()?))).collect();
    crate::ahead::ask_ahead(&mut Node::new(t, n.clone()), &asks);
    hashes
        .iter()
        .map(|h| {
            let v = crate::ask::call_one_raw(t, n, seam::M_GET_RAW_TX, &param(h))?;
            let s = v.as_str().ok_or_else(|| Fault::Endpoint(format!("malformed {n} 交易 {h} 的原文不是一串文本:{v:?}")))?;
            one(h, s)
        })
        .collect()
}

pub fn block_receipts_proven<T: Transport>(t: &T, table: &Table, block_hash: &str, proof: impl Fn(&[In]) -> Result<(), String>) -> R<Vec<In>> {
    let params = format!("[\"{}\"]", block_hash.to_ascii_lowercase());
    let list = |v: In| match receipts_covered(&v) {
        In::Arr(a) => Ok(a),
        _ => Err(Fault::Endpoint(format!("块 {block_hash} 的回执不是一列"))),
    };
    proven_alone(
        t,
        table,
        seam::M_GET_BLOCK_RECEIPTS,
        "回执",
        |n| list(crate::ask::call_one_raw(t, n, seam::M_GET_BLOCK_RECEIPTS, &params)?),
        |x: &Vec<In>| proof(x),
    )
}

pub fn instance_from(opened: u64) -> u64 {
    opened.saturating_sub(eravon_literals::constants::SCAN_BACKOFF_BLOCKS)
}

pub fn logs<T: Transport>(
    t: &T,
    table: &Table,
    addr: &str,
    from: u64,
    to: seam::Upto,
) -> R<(Vec<In>, Reading)> {
    seam::need_address(addr)?;
    logs_where(t, table, Some(addr), &[], from, to)
}

pub fn logs_at_any<T: Transport>(t: &T, table: &Table, addrs: &[String], from: u64, to: seam::Upto) -> R<(Vec<In>, Reading)> {
    if addrs.is_empty() || addrs.len() > seam::FILTER_FAN {
        return Err(Fault::Usage(format!("一问的地址该有 1 至 {} 个,给的是 {} 个", seam::FILTER_FAN, addrs.len())));
    }
    for a in addrs {
        seam::need_address(a)?;
    }
    if let [one] = addrs {
        return logs(t, table, one, from, to);
    }
    let list: Vec<String> = addrs.iter().map(|a| format!("\"{}\"", a.to_ascii_lowercase())).collect();
    logs_rest(t, table, &format!(",\"address\":[{}]", list.join(",")), from, to)
}

pub fn logs_any_ahead<T: Transport>(t: &T, table: &Table, reads: &[(Option<&str>, &[Vec<String>], u64, seam::Upto)], whole: bool) {
    let first = if whole { u64::MAX } else { eravon_literals::constants::records_floor() };
    let rests: Vec<(String, u64, seam::Upto, u64)> = reads.iter().filter_map(|(a, tp, from, to)| Some((filter_any(*a, tp).ok()?, *from, *to, first))).collect();
    rests_ahead(t, table, &rests);
}

pub fn logs_at_any_ahead<T: Transport>(t: &T, table: &Table, reads: &[(Vec<String>, u64, seam::Upto)]) {
    let rest = |addrs: &[String]| match addrs {
        [one] => filter_any(Some(one), &[]).ok(),
        _ => Some(format!(",\"address\":[{}]", addrs.iter().map(|a| format!("\"{}\"", a.to_ascii_lowercase())).collect::<Vec<_>>().join(","))),
    };
    let first = eravon_literals::constants::records_floor();
    let rests: Vec<(String, u64, seam::Upto, u64)> = reads.iter().filter_map(|(a, from, to)| Some((rest(a)?, *from, *to, first))).collect();
    rests_ahead(t, table, &rests);
}

fn rests_ahead<T: Transport>(t: &T, table: &Table, reads: &[(String, u64, seam::Upto, u64)]) {
    if reads.len() < 2 || crate::action::current().is_none() || crate::recall::current().is_some() || table.is_empty() {
        return;
    }
    let latest = if reads.iter().any(|r| matches!(r.2, seam::Upto::Latest)) {
        match common_height(t, table) {
            Ok(h) => Some(h),
            Err(_) => return,
        }
    } else {
        None
    };
    let mut filters: Vec<(crate::scan::LogFilter, u64, u64)> = Vec::new();
    for (rest, from, to, first) in reads {
        let hi = match to {
            seam::Upto::Block(n) => *n,
            seam::Upto::Latest => latest.unwrap_or(0),
        };
        let lo = floored(*from, hi, *first);
        if lo > hi {
            continue;
        }
        if let Ok(f) = log_filter(rest) {
            filters.push((f, lo, hi));
        }
    }
    let chain = table_chain(table);
    let mut eps: Vec<Node<T>> = table.nodes().into_iter().map(|n| Node::new(t, n)).collect();
    let list = crate::settled::nodes_of(eps.iter().map(|e| crate::rpc::Endpoint::place(e)).collect::<Vec<_>>().iter().map(String::as_str));
    let mut refs: Vec<&mut (dyn crate::rpc::Endpoint + Send)> = eps.iter_mut().map(|e| e as &mut (dyn crate::rpc::Endpoint + Send)).collect();
    let reads: Vec<(&crate::scan::LogFilter, u64, u64)> = filters.iter().map(|(f, lo, hi)| (f, *lo, *hi)).collect();
    crate::scan::logs_ahead(&mut refs, chain, &reads, Some(&list));
}

fn filter_any(addr: Option<&str>, topics: &[Vec<String>]) -> R<String> {
    if addr.is_none() && topics.iter().all(Vec::is_empty) {
        return Err(Fault::Usage(
            "取日志的过滑不许全空:地址与 topic 至少要有一样(零全局订阅)".into(),
        ));
    }
    let mut rest = String::new();
    if let Some(aa) = addr {
        seam::need_address(aa)?;
        rest.push_str(&format!(",\"address\":\"{aa}\""));
    }
    if !topics.is_empty() {
        let mut parts = Vec::with_capacity(topics.len());
        for x in topics {
            let mut each = Vec::with_capacity(x.len());
            for v in x {
                seam::need_hex_bytes("topic", v)?;
                if v.len() != 66 {
                    return Err(Fault::Usage(format!("topic 要恰三十二字节:{v:?}")));
                }
                each.push(format!("\"{v}\""));
            }
            parts.push(match each.len() {
                0 => "null".to_string(),
                1 => each.remove(0),
                _ => format!("[{}]", each.join(",")),
            });
        }
        rest.push_str(&format!(",\"topics\":[{}]", parts.join(",")));
    }
    Ok(rest)
}

fn floored(from: u64, head: u64, floor: u64) -> u64 {
    if head >= floor { from.max(floor) } else { from }
}

pub fn records_from(head: u64) -> u64 {
    floored(0, head, eravon_literals::constants::records_floor())
}

pub const UNANSWERED: &str = "unanswered ";

fn rest_unasked(t: &Trouble) -> bool {
    matches!(t, Trouble::Transport(crate::rpc::Broke::Unserved { .. } | crate::rpc::Broke::NoKey { .. }))
}

fn unanswered(f: &Fault) -> bool {
    matches!(f, Fault::Transport(_) | Fault::Usage(_) | Fault::Unread(_)) || f.detail().starts_with("malformed ")
}

fn filter_value(rest: &str) -> R<Value> {
    crate::ask::params_text(&format!("{{\"fromBlock\":null,\"toBlock\":null{rest}}}"))
}

fn log_filter(rest: &str) -> R<crate::scan::LogFilter> {
    use crate::scan::{LogFilter, Topic};
    let v = filter_value(rest)?;
    let bytes = |x: &Value, n: usize| -> R<Vec<u8>> {
        let s = x.as_str().ok_or_else(|| Fault::Usage(format!("过滤里该是一串十六进制:{x:?}")))?;
        let b = seam::bytes_of("过滤", s)?;
        if b.len() != n {
            return Err(Fault::Usage(format!("过滤里 {s} 该恰 {n} 字节")));
        }
        Ok(b)
    };
    let address = |x: &Value| -> R<[u8; 20]> { bytes(x, 20).map(|b| b.try_into().unwrap_or([0u8; 20])) };
    let topic = |x: &Value| -> R<[u8; 32]> { bytes(x, 32).map(|b| b.try_into().unwrap_or([0u8; 32])) };
    let addresses = match v.member("address") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Arr(xs)) => xs.iter().map(address).collect::<R<_>>()?,
        Some(x) => vec![address(x)?],
    };
    let topics = match v.member("topics") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Arr(ps)) => ps
            .iter()
            .map(|p| match p {
                Value::Null => Ok(Topic::Any),
                Value::Arr(xs) if xs.is_empty() => Ok(Topic::Any),
                Value::Arr(xs) => xs.iter().map(topic).collect::<R<Vec<_>>>().map(Topic::AnyOf),
                x => topic(x).map(Topic::One),
            })
            .collect::<R<_>>()?,
        Some(x) => return Err(Fault::Usage(format!("过滤的 topics 该是一列:{x:?}"))),
    };
    Ok(LogFilter { addresses, topics })
}

fn left_of(node: &NodeAt, refused: &crate::scan::Refusal) -> (String, Option<Fault>) {
    use crate::scan::{Asked, Refusal};
    let trouble = match refused {
        Refusal::Unanswered { asked: Asked::Declined { said, .. }, .. } => Some(Trouble::Node(said.clone())),
        Refusal::Unanswered { asked: Asked::Broke { broke, .. }, .. } => Some(Trouble::Transport(broke.clone())),
        _ => None,
    };
    let failed = |f: Fault| (format!("{UNANSWERED}{}:{}", f.name(), f.detail()), Some(f));
    if let Refusal::Unanswered { asked: Asked::OverAsks { .. }, .. } = refused {
        return failed(Fault::OverAsks(format!("{node} {}", refused.detail())));
    }
    match trouble {
        Some(tr) => {
            let limited = crate::patience::class_of(&tr) == crate::patience::Class::RateLimited;
            let unasked = rest_unasked(&tr);
            let f = crate::fault::of_trouble(node, tr, None);
            if limited || unasked || unanswered(&f) { failed(f) } else { (refused.detail(), None) }
        }
        None => failed(Fault::Endpoint(format!("malformed {node} 取日志答的不是一张日志表:{}", refused.detail()))),
    }
}

fn logs_in(list: &[W]) -> Option<Vec<In>> {
    list.iter().map(|l| crate::wire::to_core(l).map(|v| in_of(&v))).collect()
}

fn agreed_logs<T: Transport>(t: &T, nodes: &[NodeAt], filter: &crate::scan::LogFilter, chain: u64, lo: u64, hi: u64) -> R<(Vec<In>, Reading)> {
    let eps: Vec<Box<dyn crate::rpc::Endpoint + Send>> = nodes.iter().map(|n| Box::new(Node::new(t, n.clone())) as Box<dyn crate::rpc::Endpoint + Send>).collect();
    let got = window_logs(eps, chain, filter, lo, hi);
    let node_of = |place: &str| nodes.iter().find(|n| n.place() == place).cloned().unwrap_or_else(|| NodeAt::new(place));
    let left: Vec<(NodeAt, String, Option<Fault>)> = got.refused.iter().map(|(p, r)| {
        let n = node_of(p);
        let (why, f) = left_of(&n, r);
        (n, why, f)
    }).collect();
    let malformed = || Fault::Endpoint("malformed 取日志答的日志读不进值域".into());
    match got.answered {
        0 => {
            let failed: Vec<(NodeAt, Fault)> = left.iter().filter_map(|(n, _, f)| f.clone().map(|f| (n.clone(), f))).collect();
            if failed.len() == left.len() {
                return Err(crate::ask::none_answered(failed));
            }
            let why: Vec<String> = left.iter().map(|(e, w, _)| format!("{e}:{w}")).collect();
            Err(Fault::Endpoint(format!("left 取日志每一家都出列了:{}", why.join(";"))))
        }
        n if got.agreed => {
            let list = logs_in(&got.logs).ok_or_else(malformed)?;
            Ok((list.clone(), Reading { value: In::Arr(list), agreement: agreement_of(n), endpoints: n, left: Vec::new() }))
        }
        1 => {
            let list = logs_in(&got.logs).ok_or_else(malformed)?;
            Ok((list.clone(), Reading { value: In::Arr(list), agreement: agreement_of(1), endpoints: 1, left: left.into_iter().map(|(n, w, _)| (n, w)).collect() }))
        }
        _ => apart(t, nodes, filter, chain, lo, hi),
    }
}

fn apart<T: Transport>(t: &T, nodes: &[NodeAt], filter: &crate::scan::LogFilter, chain: u64, lo: u64, hi: u64) -> R<(Vec<In>, Reading)> {
    let list = crate::settled::nodes_of(nodes.iter().map(NodeAt::place).collect::<Vec<_>>().iter().map(String::as_str));
    let ask = |at: Vec<NodeAt>| {
        let (tt, f, list) = (t.clone(), filter.clone(), list.clone());
        let places: Vec<String> = at.iter().map(NodeAt::place).collect();
        asked_until(
            at,
            move |n| {
                let mut node = Node::new(&tt, n);
                crate::scan::logs_judged_over(&mut node, chain, &f, lo, hi, &list)
            },
            move |view| crate::action::read_while_cut(places.iter().zip(view).filter(|(_, g)| matches!(g, Some(Ok(_)))).map(|(p, _)| p.as_str())),
        )
    };
    let mut got = ask(nodes.to_vec());
    crate::action::cut_and_ask(
        &mut got,
        |i, _| vec![nodes[i].place()],
        |g| matches!(g, Some(Ok(_))),
        |g| matches!(g, Some(Err(r)) if r.cut()),
        |idx, _| ask(idx.iter().map(|i| nodes[*i].clone()).collect()),
    );
    let mut lists: Vec<(NodeAt, Vec<W>)> = Vec::new();
    let mut left: Vec<(NodeAt, String)> = Vec::new();
    let mut failed: Vec<(NodeAt, Fault)> = Vec::new();
    for (n, g) in nodes.iter().zip(got) {
        match g {
            Some(Ok(l)) => lists.push((n.clone(), l)),
            Some(Err(r)) => {
                let (why, f) = left_of(n, &r);
                failed.extend(f.map(|f| (n.clone(), f)));
                left.push((n.clone(), why));
            }
            None => {}
        }
    }
    let facts = |l: &[W]| crate::wire::from_core(&Value::Arr(l.iter().filter_map(|x| crate::wire::to_core(&crate::wire::project(x, &crate::settled::LOG_FACTS))).collect()));
    let answers: Vec<(String, Result<W, Missing>)> = lists.iter().map(|(n, l)| (n.written().to_string(), Ok(facts(l)))).collect();
    let empty = Value::Arr(Vec::new());
    match crate::judge::judge_read(seam::M_GET_LOGS, &empty, answers) {
        Ok(j) => {
            let lead = j.sources.first().cloned().unwrap_or_default();
            let chosen = lists.iter().find(|(n, _)| n.written() == lead).map(|(_, l)| l.clone()).unwrap_or_default();
            let value = logs_in(&chosen).ok_or_else(|| Fault::Endpoint("malformed 取日志答的日志读不进值域".into()))?;
            let n = crate::endpoints::distinct_places(j.sources.iter().map(String::as_str));
            if j.single_source {
                left.extend(j.missing.into_iter().map(|(name, m)| {
                    let node = NodeAt::new(name);
                    let why = match m {
                        Missing::Trouble(tr) => tr_evidence(&tr),
                        Missing::Unreadable(text) => format!("unreadable {text}"),
                    };
                    (node, why)
                }));
            } else {
                left.clear();
            }
            Ok((value.clone(), Reading { value: In::Arr(value), agreement: agreement_of(n), endpoints: n, left }))
        }
        Err(NoReading::Differ(d)) => {
            let ins: Vec<(NodeAt, In)> = lists.iter().filter_map(|(n, l)| logs_in(l).map(|v| (n.clone(), In::Arr(v)))).collect();
            Err(Fault::Disagree(crate::ask::split(seam::M_GET_LOGS, &ins).unwrap_or_else(|| differ(seam::M_GET_LOGS, &d))))
        }
        Err(_) => {
            if !left.is_empty() && failed.len() == left.len() {
                return Err(crate::ask::none_answered(failed));
            }
            let why: Vec<String> = left.iter().map(|(e, w)| format!("{e}:{w}")).collect();
            Err(Fault::Endpoint(format!("left 取日志每一家都出列了:{}", why.join(";"))))
        }
    }
}

fn tr_evidence(t: &Trouble) -> String {
    match t {
        Trouble::Transport(b) => b.evidence(),
        other => format!("{other:?}"),
    }
}

fn window_logs(nodes: Vec<Box<dyn crate::rpc::Endpoint + Send>>, chain: u64, filter: &crate::scan::LogFilter, lo: u64, hi: u64) -> crate::scan::AgreedLogs {
    crate::scan::logs_agreed(nodes, chain, filter, lo, hi)
}


pub fn logs_where<T: Transport>(
    t: &T,
    table: &Table,
    addr: Option<&str>,
    topics: &[Option<String>],
    from: u64,
    to: seam::Upto,
) -> R<(Vec<In>, Reading)> {
    let any: Vec<Vec<String>> = topics.iter().map(|x| x.iter().cloned().collect()).collect();
    logs_any(t, table, addr, &any, from, to)
}

pub fn logs_any<T: Transport>(
    t: &T,
    table: &Table,
    addr: Option<&str>,
    topics: &[Vec<String>],
    from: u64,
    to: seam::Upto,
) -> R<(Vec<In>, Reading)> {
    let rest = filter_any(addr, topics)?;
    logs_rest(t, table, &rest, from, to)
}

pub fn logs_any_whole<T: Transport>(
    t: &T,
    table: &Table,
    addr: Option<&str>,
    topics: &[Vec<String>],
    from: u64,
    to: seam::Upto,
) -> R<(Vec<In>, Reading)> {
    let rest = filter_any(addr, topics)?;
    logs_rest_at(t, table, &rest, from, to, u64::MAX)
}

pub(crate) fn logs_rest<T: Transport>(t: &T, table: &Table, rest: &str, from: u64, to: seam::Upto) -> R<(Vec<In>, Reading)> {
    logs_rest_at(t, table, rest, from, to, eravon_literals::constants::records_floor())
}

pub(crate) fn logs_rest_as_asked<T: Transport>(t: &T, table: &Table, rest: &str, from: u64, to: seam::Upto) -> R<(Vec<In>, Reading)> {
    logs_rest_at(t, table, rest, from, to, u64::MAX)
}

fn logs_rest_at<T: Transport>(t: &T, table: &Table, rest: &str, from: u64, to: seam::Upto, first: u64) -> R<(Vec<In>, Reading)> {
    if let Some(r) = crate::recall::current() {
        let head = r.store().head;
        let lo = floored(from, head, first);
        return crate::recall::logs(rest, lo, to).unwrap_or_else(|| Err(Fault::Unread(rest.to_string())));
    }
    if table.is_empty() {
        return Err(Fault::Usage("端点表是空的".into()));
    }
    let cut = pinned(t, table, seam::M_GET_LOGS)?;
    let table = cut.as_ref().unwrap_or(table);
    let hi = match to {
        seam::Upto::Block(n) => n,
        seam::Upto::Latest => common_height(t, table)?,
    };
    let lo = floored(from, hi, first);
    if lo > hi {
        return Ok((Vec::new(), reading(In::Arr(Vec::new()), table_places(table))));
    }
    let nodes = table.nodes();
    let asked: &[NodeAt] = &nodes;
    let filter = log_filter(rest)?;
    let chain = table_chain(table);
    agreed_logs(t, asked, &filter, chain, lo, hi)
}

fn common_height<T: Transport>(t: &T, table: &Table) -> R<u64> {
    head_at(t, table, seam::TAG_LATEST)
}

pub fn ahead_anchors<T: Transport>(t: &T, table: &Table, chain: u64, wants: &[crate::ahead::Want]) {
    if wants.is_empty() || crate::action::current().is_none() || crate::recall::current().is_some() {
        return;
    }
    let nodes = table.nodes();
    let places: Vec<String> = nodes.iter().map(|n| n.place()).collect();
    let list = crate::settled::nodes_of(places.iter().map(String::as_str));
    let (tt, wants) = (t.clone(), wants.to_vec());
    let _ = asked_until(
        nodes,
        move |n| {
            let mut node = Node::new(&tt, n.clone());
            crate::ahead::read_ahead(&mut node, chain, &wants, crate::ahead::Share::of(&n.place(), Some(&list)));
        },
        all,
    );
}

static PROVED_TXS: std::sync::Mutex<std::collections::BTreeMap<(u64, [u8; 32]), In>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

pub fn proven_transaction<T: Transport>(t: &T, table: &Table, chain: u64, hash: &[u8; 32]) -> Option<In> {
    if crate::recall::current().is_some() {
        return kept_proven(hash);
    }
    if let Some(v) = PROVED_TXS.lock().unwrap_or_else(|e| e.into_inner()).get(&(chain, *hash)) {
        return Some(v.clone());
    }
    let params = Value::Arr(vec![Value::Str(seam::hex_of(hash))]);
    let v = proven_of(t, table, chain, hash, &params)?;
    PROVED_TXS.lock().unwrap_or_else(|e| e.into_inner()).insert((chain, *hash), v.clone());
    Some(v)
}

fn kept_proven(hash: &[u8; 32]) -> Option<In> {
    let kept = crate::recall::answer(seam::M_GET_TX, &format!("[\"{}\"]", seam::hex_of(hash)))?.ok()?;
    let w = crate::wire::parse(crate::ask::fingerprint(&kept.value).as_bytes())?;
    crate::tx::read(&w).ok().filter(|tx| tx.hash == *hash && !tx.raw.is_empty()).map(|_| kept.value)
}

pub(crate) fn transaction_kept<T: Transport>(t: &T, table: &Table, params_json: &str) -> R<Reading> {
    let agreed = read_agreed_raw(t, table, seam::M_GET_TX, params_json)?;
    if matches!(agreed.value, In::Null) {
        return Ok(agreed);
    }
    let params = crate::ask::params_text(params_json)?;
    let asked = match &params {
        Value::Arr(a) => match a.first() {
            Some(Value::Str(h)) => seam::bytes_of("交易哈希", h).ok().and_then(|b| <[u8; 32]>::try_from(b).ok()),
            _ => None,
        },
        _ => None,
    };
    let Some(hash) = asked else { return Ok(agreed) };
    let want = crate::ask::fingerprint(&agreed.value);
    for n in table.nodes() {
        let mut ep = Node::new(t, n.clone());
        let Ok(w) = crate::patience::ask(&mut ep, seam::M_GET_TX, &params) else { continue };
        let proves = crate::tx::read(&w).is_ok_and(|tx| tx.hash == hash && !tx.raw.is_empty());
        let same = crate::ask::decided(&n, seam::M_GET_TX, &params, &w).is_ok_and(|cut| crate::ask::fingerprint(&cut) == want);
        if let (true, true, Some(whole)) = (proves, same, crate::wire::to_core(&w).map(|v| in_of(&v))) {
            return Ok(Reading { value: whole, ..agreed });
        }
    }
    Ok(agreed)
}

fn proven_of<T: Transport>(t: &T, table: &Table, chain: u64, hash: &[u8; 32], params: &Value) -> Option<In> {
    let mut value = None;
    let got = crate::action::proven(chain, hash, || -> Result<Option<crate::tx::Tx>, ()> {
        for n in table.nodes() {
            let mut ep = Node::new(t, n.clone());
            let Ok(w) = crate::patience::ask(&mut ep, seam::M_GET_TX, params) else { continue };
            let Ok(tx) = crate::tx::read(&w) else { continue };
            if tx.hash == *hash && !tx.raw.is_empty() {
                value = crate::wire::to_core(&w).map(|v| in_of(&v));
                return Ok(Some(tx));
            }
        }
        Ok(None)
    });
    match got {
        Ok(Some(_)) => value.or_else(|| held_whole(table, hash)),
        _ => None,
    }
}

fn held_whole(table: &Table, hash: &[u8; 32]) -> Option<In> {
    let asked = Value::Arr(vec![Value::Str(eravon::hexfmt::encode(hash))]);
    table.nodes().into_iter().find_map(|n| {
        let w = crate::action::answered(n.written(), seam::M_GET_TX, &asked)?;
        let tx = crate::tx::read(&w).ok()?;
        (tx.hash == *hash && !tx.raw.is_empty()).then(|| crate::wire::to_core(&w).map(|v| in_of(&v))).flatten()
    })
}


pub fn proved_time(chain: u64, block_hash: &[u8; 32], number: u64) -> Option<u64> {
    if crate::recall::current().is_some() {
        return None;
    }
    let _at = crate::ahead::reading_at(Some(*block_hash));
    crate::ahead::time_at(chain, number)
}

pub fn proved_code(chain: u64, block_hash: &[u8; 32], who: &[u8; 20], number: u64) -> Option<bool> {
    if crate::recall::current().is_some() {
        return None;
    }
    let _at = crate::ahead::reading_at(Some(*block_hash));
    crate::ahead::code_at(chain, who, number)
}

pub fn floor_of(table: &Table) -> Option<u64> {
    let nodes = table.nodes();
    crate::action::floor_among(nodes.iter().map(|n| n.written()))
}

fn head_at<T: Transport>(t: &T, table: &Table, tag: &str) -> R<u64> {
    if table.is_empty() {
        return Err(Fault::Usage("端点表是空的".into()));
    }
    let nodes = table.nodes();
    let floor = crate::action::floor_among(nodes.iter().map(|n| n.written())).filter(|_| tag == seam::TAG_LATEST);
    let heads = || {
        let (tt, tg, seen) = (t.clone(), tag.to_string(), nodes.clone());
        let empty = Value::Arr(Vec::new());
        let got = asked_until(nodes.clone(), move |e| reached(&tt, &e, &tg), move |view: &[Option<&R<u64>>]| {
            let arrived: Vec<(String, Result<W, Missing>)> = seen.iter().zip(view).filter_map(|(n, h)| match h {
                Some(Ok(h)) => Some((n.written().to_string(), Ok(crate::wire::from_core(&Value::Str(seam::quantity(*h)))))),
                _ => None,
            }).collect();
            crate::judge::enough(seam::M_BLOCK_NUMBER, &empty, &arrived, floor)
        });
        let mut heights: Vec<(NodeAt, u64)> = Vec::new();
        let mut failed: Vec<(NodeAt, Fault)> = Vec::new();
        for (e, h) in nodes.iter().zip(got) {
            match h {
                Some(Ok(n)) => heights.push((e.clone(), n)),
                Some(Err(Fault::Execution(d))) => failed.push((e.clone(), Fault::Endpoint(d))),
                Some(Err(f)) => failed.push((e.clone(), f)),
                None => {}
            }
        }
        if heights.is_empty() {
            return Err(crate::ask::none_answered(failed));
        }
        Ok(heights)
    };
    let head_of = |hs: &[(NodeAt, u64)]| crate::judge::reached(&hs.iter().map(|(n, h)| (n.written(), *h)).collect::<Vec<_>>()).map(|(_, h)| h);
    let mut heights = heads()?;
    if let Some(floor) = floor {
        let mut ticks = 0;
        while head_of(&heights).is_none_or(|h| h < floor) {
            if ticks == FLOOR_TICKS {
                let (at, low) = heights.iter().filter(|(_, h)| *h < floor).max_by_key(|(_, h)| *h).cloned().unwrap_or_else(|| (nodes[0].clone(), 0));
                return Err(Fault::Disagree(format!("behind {at} 的块高 {low} 还没到这一动作读到的回执所在块 {floor}(差 {} 块)", floor - low)));
            }
            ticks += 1;
            heights = crate::action::after_a_beat(crate::patience::waited(FLOOR_TICK), &heads)?;
        }
        {
            let table_of: Vec<String> = nodes.iter().map(|n| n.written().to_string()).collect();
            let mut s = PASSED.lock().unwrap_or_else(|e| e.into_inner());
            for (e, h) in &heights {
                match *h < floor {
                    true => s.insert(e.written().to_string(), Passed { table_of: table_of.clone(), head: *h, floor }),
                    false => s.remove(e.written()),
                };
            }
        }
        heights.retain(|(_, h)| *h >= floor);
    }
    let head = head_of(&heights).unwrap_or(0);
    let high = heights.iter().map(|(_, h)| *h).max().unwrap_or(head);
    if lagging(high, head) {
        let behind: Vec<String> = heights.iter().filter(|(_, h)| high - *h > seam::HEIGHT_SLACK).map(|(e, h)| format!("{e}(块 {h})")).collect();
        let what = if tag == seam::TAG_LATEST { "块高".to_string() } else { format!(" {tag} 块") };
        return Err(Fault::Disagree(format!(
            "behind 端点{what}相差 {} 块,过了 {} 块的界(最高 {high}):落在后面的是 {}",
            high - head,
            seam::HEIGHT_SLACK,
            behind.join("、")
        )));
    }
    Ok(head)
}

pub const FLOOR_TICK: std::time::Duration = std::time::Duration::from_secs(1);
pub const FLOOR_TICKS: usize = 30;
struct Passed {
    table_of: Vec<String>,
    head: u64,
    floor: u64,
}

static PASSED: std::sync::Mutex<std::collections::BTreeMap<String, Passed>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

pub(crate) fn passed_over(written: &str) -> Option<Trouble> {
    let (table_of, was, at) = PASSED.lock().unwrap_or_else(|e| e.into_inner()).get(written).map(|p| (p.table_of.clone(), p.head, p.floor))?;
    let floor = crate::action::floor_among(table_of.iter().map(String::as_str))?;
    let head = crate::action::head_of(written).or((floor == at).then_some(was))?;
    (head < floor).then(|| Trouble::Transport(crate::rpc::Broke::HeadBehind { node: eravon_net::sayable(written), block: floor }))
}

fn reached<T: Transport>(t: &T, e: &NodeAt, tag: &str) -> R<u64> {
    let v = if tag == seam::TAG_LATEST {
        crate::ask::call_one_raw(t, e, seam::M_BLOCK_NUMBER, "[]")?
    } else {
        let b = crate::ask::call_one_raw(t, e, seam::M_GET_BLOCK, &format!("[\"{tag}\",false]"))?;
        b.get("number").cloned().ok_or_else(|| Fault::Endpoint(format!("malformed {e} 的 {tag} 块答里没有 number:{b:?}")))?
    };
    let s = v.as_str().ok_or_else(|| Fault::Endpoint(format!("malformed 块高该答一串文本,给的是 {v:?}")))?;
    seam::read_quantity(s)
}

fn lagging(high: u64, low: u64) -> bool {
    high - low > seam::HEIGHT_SLACK
}

pub fn height<T: Transport>(t: &T, table: &Table) -> R<(u64, Reading)> {
    mark::mark(mark::FEATURE);
    if let Some(kept) = crate::recall::answer(seam::M_BLOCK_NUMBER, "[]") {
        let r = kept?;
        return Ok((seam::read_quantity(&as_str(&r)?)?, r));
    }
    if table.is_empty() {
        return Err(Fault::Usage("端点表是空的".into()));
    }
    let cut = pinned(t, table, seam::M_BLOCK_NUMBER)?;
    let table = cut.as_ref().unwrap_or(table);
    let h = common_height(t, table)?;
    Ok((h, reading(In::Str(seam::quantity(h)), table_places(table))))
}

pub fn transaction<T: Transport>(t: &T, table: &Table, hash: &str) -> R<(In, Reading)> {
    let r = read_agreed(t, table, seam::M_GET_TX, vec![Out::S(hash.to_string())])?;
    Ok((r.value.clone(), r))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VoidSaid {
    Nonce(u64),
    Past(u64, Option<bool>),
    Unread(Fault),
}

pub fn void_read<T: Transport>(t: &T, table: &Table, from: &str, hash: &str, mine: u64) -> R<Vec<(NodeAt, Option<VoidSaid>)>> {
    seam::need_address(from)?;
    if table.is_empty() {
        return Err(Fault::Usage("端点表是空的".into()));
    }
    let cut = pinned(t, table, seam::M_GET_TX)?;
    let table = cut.as_ref().unwrap_or(table);
    let nodes = table.nodes();
    let count = crate::ask::params_of(&[Out::S(from.to_ascii_lowercase()), Out::S(seam::TAG_LATEST.to_string())]);
    let tx = crate::ask::params_of(&[Out::S(hash.to_string())]);
    let (tt, seen) = (t.clone(), nodes.clone());
    let got = asked_until(
        nodes.clone(),
        move |n| {
            let nonce = crate::ask::ask_value(&tt, &n, seam::M_GET_TX_COUNT, &count)
                .and_then(|v| v.as_str().map(str::to_string).ok_or_else(|| Fault::Endpoint(format!("{n} 答的 nonce 不是一串文本"))))
                .and_then(|s| seam::read_quantity(&s));
            match nonce {
                Err(f) => VoidSaid::Unread(f),
                Ok(at) if at > mine => VoidSaid::Past(at, crate::ask::ask_value(&tt, &n, seam::M_GET_TX, &tx).ok().map(|v| matches!(v, In::Null))),
                Ok(at) => VoidSaid::Nonce(at),
            }
        },
        move |view: &[Option<&VoidSaid>]| {
            view.iter().any(|v| matches!(v, Some(VoidSaid::Past(_, Some(false)))))
                || two_places(&seen, view, |v| matches!(v, VoidSaid::Past(_, Some(true))))
                || two_places(&seen, view, |v| matches!(v, VoidSaid::Nonce(_)))
        },
    );
    Ok(nodes.into_iter().zip(got).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct Logs {
        rule: fn(usize, u64, u64) -> Result<(), &'static str>,
        asked: std::sync::Arc<std::sync::Mutex<Vec<(u64, u64)>>>,
        bodies: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl Transport for Logs {
        fn post(&self, _url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let b = String::from_utf8_lossy(body).to_string();
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let q = |k: &str| b.split(&format!("\"{k}\":\"0x")).nth(1).and_then(|x| x.split('"').next()).and_then(|h| u64::from_str_radix(h, 16).ok()).unwrap_or(0);
            let (f, t) = (q("fromBlock"), q("toBlock"));
            self.bodies.lock().unwrap().push(b.clone());
            let mut a = self.asked.lock().unwrap();
            a.push((f, t));
            let body = match (self.rule)(a.len(), f, t) {
                Ok(()) => format!(r#"{{"id":{id},"jsonrpc":"2.0","result":[]}}"#),
                Err(m) => format!(r#"{{"id":{id},"jsonrpc":"2.0","error":{{"code":-32603,"message":"{m}"}}}}"#),
            };
            Ok(crate::transport::Answer { status: 200, location: None, body: body.into_bytes() })
        }
    }

    fn logs_node(rule: fn(usize, u64, u64) -> Result<(), &'static str>) -> Logs {
        Logs { rule, asked: Default::default(), bodies: Default::default() }
    }

    fn swept(rule: fn(usize, u64, u64) -> Result<(), &'static str>, lo: u64, hi: u64) -> (Result<usize, String>, Vec<(u64, u64)>) {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(20_000);
        let endpoint = format!("http://127.0.0.1:{}", NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst));
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let t = logs_node(rule);
        let filter = log_filter(",\"topics\":[\"0x0000000000000000000000000000000000000000000000000000000000000001\"]").unwrap();
        let r = agreed_logs(&t, &[NodeAt::new(endpoint)], &filter, 1, lo, hi).map(|(l, _)| l.len()).map_err(|f| f.detail());
        let asked = t.asked.lock().unwrap().clone();
        (r, asked)
    }

    #[test]
    fn a_log_read_walks_the_family_window_with_its_own_filter() {
        let (r, a) = swept(|n, _, _| if n == 1 { Err("service temporarily unavailable") } else { Ok(()) }, 0, 99_999);
        assert_eq!((r, a), (Ok(0), vec![(0, 99_999), (0, 99_999)]), "拒一次:再问一次、整段、不拆");
        for said in ["invalid block params", "always no"] {
            let rule: fn(usize, u64, u64) -> Result<(), &'static str> = match said {
                "invalid block params" => |_, _, _| Err("invalid block params"),
                _ => |_, _, _| Err("always no"),
            };
            let (r, a) = swept(rule, 0, 99_999);
            assert_eq!(a[..2], [(0, 99_999), (0, 99_999)], "{said}: the whole range asked twice first: {a:?}");
            assert!(a.len() > 2 && a[2..].iter().zip(a[1..].iter()).all(|(p, q)| p.0 == 0 && p.1 - p.0 + 1 == (q.1 - q.0 + 1).div_ceil(2)), "{said}: halved on each refusal: {a:?}");
            assert!(r.as_ref().is_err_and(|w| w.contains(said) && w.contains(crate::scan::OVER_ASKS)), "{said}: gives way by the limit in the node's own words: {r:?}");
        }
        let t = logs_node(|_, _, _| Ok(()));
        let filter = log_filter(",\"address\":\"0x0000000000000000000000000000000000000001\",\"topics\":[null,\"0x0000000000000000000000000000000000000000000000000000000000000002\"]").unwrap();
        assert!(agreed_logs(&t, &["http://127.0.0.1:19998".into()], &filter, 1, 5, 9).is_ok());
        let body = t.bodies.lock().unwrap()[0].clone();
        assert!(body.contains(r#""params":[{"address":["0x0000000000000000000000000000000000000001"],"fromBlock":"0x5","toBlock":"0x9","topics":[null,"0x0000000000000000000000000000000000000000000000000000000000000002"]}]"#), "{body}");
    }

    #[derive(Clone, Copy)]
    enum Answers {
        Whole,
        Cap(u64, &'static str),
        Down,
        Limited,
        Moving,
    }

    #[derive(Clone)]
    struct Capped {
        kinds: [Answers; 3],
        asked: std::sync::Arc<std::sync::Mutex<Vec<(String, u64, u64)>>>,
    }

    impl Transport for Capped {
        fn post(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let b = String::from_utf8_lossy(body).to_string();
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let i = usize::from(url.port % 3);
            let say = |status: u16, r: String| Ok(crate::transport::Answer { status, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0",{r}}}"#).into_bytes() });
            if matches!(self.kinds[i], Answers::Down) {
                return Err(Fault::Transport("connect refused".into()));
            }
            if !b.contains("eth_getLogs") {
                return say(200, format!(r#""result":{}"#, if b.contains("eth_chainId") { "\"0x7a69\"" } else { "\"0x20000\"" }));
            }
            let q = |k: &str| b.split(&format!("\"{k}\":\"0x")).nth(1).and_then(|x| x.split('"').next()).and_then(|h| u64::from_str_radix(h, 16).ok()).unwrap_or(0);
            let (f, to) = (q("fromBlock"), q("toBlock"));
            self.asked.lock().unwrap().push((["a", "b", "c"][i].to_string(), f, to));
            match self.kinds[i] {
                Answers::Limited => return say(429, r#""error":{"code":-32005,"message":"rate limit exceeded"}"#.into()),
                Answers::Cap(c, said) if to - f + 1 > c => return say(200, format!(r#""error":{{"code":-32012,"message":"{said}"}}"#)),
                Answers::Moving if to - f + 1 > 100_000 => return say(200, r#""error":{"code":-32012,"message":"exceed maximum block range: 100000"}"#.into()),
                Answers::Moving if to - f + 1 > 50_000 && to >= 100_000 && f < 200_000 => return say(200, r#""error":{"code":-32012,"message":"exceed maximum block range: 10000"}"#.into()),
                _ => {}
            }
            let logs: Vec<String> = (f..=to).filter(|n| n % 1000 == 0).map(|n| format!(r#"{{"address":"0x0000000000000000000000000000000000000001","blockHash":"0x{n:064x}","blockNumber":"0x{n:x}","data":"0x","logIndex":"0x0","topics":["0x0000000000000000000000000000000000000000000000000000000000000001"],"transactionHash":"0x{n:064x}","transactionIndex":"0x0"}}"#)).collect();
            say(200, format!(r#""result":[{}]"#, logs.join(",")))
        }
    }

    #[test]
    fn the_range_is_cut_only_where_none_answered_it_whole_and_each_node_refused_halves_until_it_answers() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        static PORT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(31_000);
        const B30: Answers = Answers::Cap(30_000, "exceed maximum block range: 30000");
        const B10: Answers = Answers::Cap(10_000, "exceed maximum block range: 10000");
        let read_to = |kinds: [Answers; 3], to: u64| {
            let a = PORT.fetch_add(3, std::sync::atomic::Ordering::SeqCst);
            let a = a - a % 3 + 3;
            let table = Table::new((0..3).map(|k| format!("http://127.0.0.1:{}", a + k)).collect()).expect("table");
            let t = Capped { kinds, asked: Default::default() };
            let _a = crate::action::open(None);
            let got = logs_rest_as_asked(&t, &table, ",\"topics\":[\"0x0000000000000000000000000000000000000000000000000000000000000001\"]", 0, seam::Upto::Block(to));
            let asked = t.asked.lock().unwrap().clone();
            (got, asked)
        };
        let read = |kinds: [Answers; 3]| read_to(kinds, 99_999);
        let of = |asked: &[(String, u64, u64)], n: &str| asked.iter().filter(|(h, _, _)| h == n).map(|(_, f, t)| (*f, *t)).collect::<Vec<_>>();
        let blocks = |logs: &[In]| logs.iter().filter_map(|l| l.get("blockNumber").and_then(In::as_str).and_then(|h| seam::read_quantity(h).ok())).collect::<Vec<u64>>();
        let pieces = |r: &[(u64, u64)], whole: (u64, u64)| r.iter().filter(|x| **x != whole).copied().collect::<Vec<_>>();
        let whole_only = |r: &[(u64, u64)]| r.iter().all(|x| *x == (0, 99_999));
        let covers = |p: &[(u64, u64)], whole: (u64, u64), most: u64| {
            let mut fit: Vec<(u64, u64)> = p.iter().filter(|(f, t)| t - f + 1 <= most).copied().collect();
            fit.sort();
            fit.dedup();
            let mut next = whole.0;
            for (f, t) in fit {
                if f == next {
                    next = t + 1;
                }
            }
            next == whole.1 + 1
        };
        let (got, asked) = read([Answers::Whole, B30, B10]);
        assert_eq!(got.expect("read whole").0.len(), 100);
        for n in ["a", "b", "c"] {
            assert!(whole_only(&of(&asked, n)), "{n} asked only the whole range: {:?}", of(&asked, n));
        }
        for first in [Answers::Down, Answers::Limited] {
            let (got, asked) = read([first, B30, B10]);
            let (logs, r) = got.expect("read in pieces");
            assert_eq!((logs.len(), r.endpoints), (100, 1), "every log once, the node that read it alone: {:?}", r.left);
            assert!(blocks(&logs).windows(2).all(|p| p[0] < p[1]), "in order, none twice");
            let (b, c) = (pieces(&of(&asked, "b"), (0, 99_999)), pieces(&of(&asked, "c"), (0, 99_999)));
            assert!(b.iter().chain(&c).all(|(f, t)| (t - f + 1) * 2 <= 100_000), "halves, never the number a refusal says: {b:?} {c:?}");
            assert!(covers(&b, (0, 99_999), 30_000) || covers(&c, (0, 99_999), 10_000), "a node read the window in pieces it answered: {b:?} {c:?}");
            assert!(whole_only(&of(&asked, "a")), "the one that did not answer is cut for nothing: {:?}", of(&asked, "a"));
        }
        let (got, asked) = read([Answers::Down, Answers::Cap(30_000, "block range too wide"), Answers::Down]);
        assert_eq!(got.expect("read in halves").0.len(), 100);
        let b = pieces(&of(&asked, "b"), (0, 99_999));
        assert!(b.iter().all(|(f, t)| t - f + 1 <= 50_000), "halves and on: {b:?}");
        assert!(b.iter().any(|(f, t)| t - f + 1 == 50_000) && b.iter().any(|(f, t)| t - f + 1 <= 25_000), "halved, then halved again: {b:?}");
        let (got, asked) = read_to([Answers::Moving, Answers::Moving, Answers::Down], 199_999);
        let (logs, _) = got.expect("read with the piece cut again");
        assert_eq!(logs.len(), 200, "every log once");
        let (a, b) = (pieces(&of(&asked, "a"), (0, 199_999)), pieces(&of(&asked, "b"), (0, 199_999)));
        assert!(a.iter().chain(&b).all(|(f, t)| t - f + 1 != 10_000), "a piece refused again is halved, never cut by the number it says: {a:?} {b:?}");
        assert!(covers(&a, (0, 199_999), 100_000) || covers(&b, (0, 199_999), 100_000), "a node read the window in pieces it answered: {a:?} {b:?}");
        let (got, asked) = read([Answers::Down, Answers::Limited, Answers::Down]);
        assert!(got.is_err(), "no reading");
        assert!(asked.iter().all(|(_, f, t)| (*f, *t) == (0, 99_999)), "nothing cut: {asked:?}");
    }

    #[test]
    fn a_read_every_node_gave_up_by_the_limit_on_questions_is_named_so_ahead_of_a_node_that_was_down() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let table = Table::new((36_600..36_603).map(|p| format!("http://127.0.0.1:{p}")).collect()).expect("table");
        let t = Capped { kinds: [Answers::Down, Answers::Cap(0, "always no"), Answers::Cap(0, "block range too wide")], asked: Default::default() };
        let _a = crate::action::open(None);
        let f = logs_rest_as_asked(&t, &table, ",\"topics\":[\"0x0000000000000000000000000000000000000000000000000000000000000001\"]", 0, seam::Upto::Block(99_999)).expect_err("no node read it");
        assert_eq!(f.name(), "E_CHAIN_OVER_ASKS", "the limit leads, not the node that was down: {f:?}");
        let d = f.detail();
        let (lead, each) = d.split_once(" · ").expect("lead, then each node");
        assert!(lead.contains(crate::scan::OVER_ASKS), "{lead}");
        for (port, name, said) in [(36_600, "E_CHAIN_TRANSPORT", ""), (36_601, "E_CHAIN_OVER_ASKS", "always no"), (36_602, "E_CHAIN_OVER_ASKS", "block range too wide")] {
            let row = each.split(';').find(|r| r.contains(&format!(":{port}"))).unwrap_or_else(|| panic!("{port} named: {each}"));
            assert!(row.contains(name) && row.contains(said), "{port}: {row}");
        }
    }

    #[derive(Clone)]
    struct Heads {
        heads: [u64; 3],
        asked: std::sync::Arc<std::sync::Mutex<Vec<(usize, &'static str, Option<u64>)>>>,
    }

    impl Transport for Heads {
        fn post(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let b = String::from_utf8_lossy(body).to_string();
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let i = usize::from(url.port % 3);
            let head = self.heads[i];
            let say = |r: String| Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0",{r}}}"#).into_bytes() });
            if b.contains("eth_chainId") {
                return say(r#""result":"0x7a69""#.into());
            }
            if head == u64::MAX && (b.contains("eth_blockNumber") || b.contains("eth_getBalance")) {
                return say(r#""error":{"code":-32603,"message":"internal error"}"#.into());
            }
            if b.contains("eth_blockNumber") {
                self.asked.lock().unwrap().push((i, "head", None));
                return say(format!(r#""result":"0x{head:x}""#));
            }
            let at = b.split("\"0x").last().and_then(|x| x.split('"').next()).and_then(|h| u64::from_str_radix(h, 16).ok());
            self.asked.lock().unwrap().push((i, "balance", at));
            match at {
                Some(n) if n > head => say(r#""error":{"code":-32000,"message":"header not found"}"#.into()),
                _ => say(r#""result":"0x64""#.into()),
            }
        }
    }

    #[test]
    fn two_endpoints_at_the_receipts_block_are_enough_to_go_on() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        static PORT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(33_000);
        let read_of = |heads: [u64; 3], n: u16| {
            let a = PORT.fetch_add(3, std::sync::atomic::Ordering::SeqCst);
            let a = a - a % 3;
            let urls: Vec<String> = (0..n).map(|k| format!("http://127.0.0.1:{}", a + k)).collect();
            let table = Table::new(urls.clone()).expect("table");
            let t = Heads { heads, asked: Default::default() };
            let _action = crate::action::open(None);
            let receipt = W::of(crate::wire::Body::Obj(vec![("blockNumber".into(), W::of(crate::wire::Body::Str("0x40".into()))), ("status".into(), W::of(crate::wire::Body::Str("0x1".into())))]));
            for u in &urls {
                crate::action::note(u, seam::M_GET_RECEIPT, &Value::Arr(vec![Value::Str("0x01".into())]), &receipt);
            }
            let got = read_agreed(&t, &table, seam::M_GET_BALANCE, vec![Out::S("0x00000000000000000000000000000000000000b0".into()), Out::S(seam::TAG_LATEST.into())]);
            let asked = t.asked.lock().unwrap().clone();
            (got, asked)
        };
        let read = |heads: [u64; 3]| read_of(heads, 3);
        let heads_of = |asked: &[(usize, &str, Option<u64>)], i: usize| asked.iter().filter(|(n, k, _)| *n == i && *k == "head").count();
        let (got, asked) = read([0x40, 0x41, 0x3e]);
        println!("ROW two-there got={:?} heads={:?} balances={:?}", got.as_ref().map(|r| (r.endpoints, r.left.clone())).map_err(|f| f.detail()), (0..3).map(|i| heads_of(&asked, i)).collect::<Vec<_>>(), asked.iter().filter(|(_, k, _)| *k == "balance").collect::<Vec<_>>());
        let r = got.expect("read at once");
        assert_eq!(r.endpoints, 2, "the two that reached it");
        assert!((0..3).all(|i| heads_of(&asked, i) <= 1) && (0..3).filter(|i| heads_of(&asked, *i) == 1).count() >= 2, "no tick waited: {asked:?}");
        assert!(asked.iter().filter(|(_, k, _)| *k == "balance").all(|(_, _, at)| *at == Some(0x40)), "pinned at the receipt's block: {asked:?}");
        let (got, asked) = read([0x40, 0x3e, 0x3e]);
        println!("ROW one-there got={:?} heads={:?}", got.as_ref().map(|r| r.endpoints).map_err(|f| f.detail()), (0..3).map(|i| heads_of(&asked, i)).collect::<Vec<_>>());
        assert!(got.as_ref().is_err_and(|f| f.name() == "E_CHAIN_DISAGREE"), "one there is not enough");
        assert_eq!(heads_of(&asked, 0), 1 + FLOOR_TICKS, "asked again every tick to the end");
        let (got, _) = read([0x40, 0x3f, 0x3d]);
        let said = got.err().map(|f| f.detail()).unwrap_or_default();
        assert!(said.contains("块高 63") && !said.contains("块高 61"), "the head two places reached, named: {said}");
        let (got, asked) = read([0x40, u64::MAX, u64::MAX]);
        println!("ROW one-gave-head got={:?} heads={:?}", got.as_ref().map(|r| (r.endpoints, r.left.len())).map_err(|f| f.detail()), (0..3).map(|i| heads_of(&asked, i)).collect::<Vec<_>>());
        assert_eq!(got.expect("on at once").endpoints, 1, "the one that gave its head and is there");
        assert_eq!(heads_of(&asked, 0), 1, "no tick waited: {asked:?}");
        let (got, asked) = read([0x40, 0x3e, u64::MAX]);
        println!("ROW one-there-one-behind got={:?} heads={:?}", got.as_ref().map(|r| r.endpoints).map_err(|f| f.detail()), (0..3).map(|i| heads_of(&asked, i)).collect::<Vec<_>>());
        assert!(got.is_err_and(|f| f.name() == "E_CHAIN_DISAGREE"), "one there of two that gave their head is not enough");
        assert_eq!(heads_of(&asked, 0), 1 + FLOOR_TICKS, "asked again every tick to the end");
        let (got, asked) = read_of([0x40, 0x41, 0], 2);
        assert_eq!(got.expect("both there").endpoints, 2);
        assert!((0..2).all(|i| heads_of(&asked, i) == 1), "no tick waited: {asked:?}");
        let (got, asked) = read_of([0x40, 0x3e, 0], 2);
        println!("ROW table-of-two-one-behind got={:?} heads={:?}", got.as_ref().map(|r| r.endpoints).map_err(|f| f.detail()), (0..2).map(|i| heads_of(&asked, i)).collect::<Vec<_>>());
        assert!(got.is_err_and(|f| f.name() == "E_CHAIN_DISAGREE"), "a table of two waits for both");
        assert_eq!(heads_of(&asked, 0), 1 + FLOOR_TICKS);
    }

    #[derive(Clone)]
    struct Lagging {
        form: u8,
        asked: std::sync::Arc<std::sync::Mutex<Vec<(usize, String)>>>,
    }

    impl Transport for Lagging {
        fn post(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let b = String::from_utf8_lossy(body).to_string();
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let i = usize::from(url.port % 3);
            let head: u64 = [0x40, 0x41, 0x3e][i];
            let method = b.split("\"method\":\"").nth(1).and_then(|x| x.split('"').next()).unwrap_or("").to_string();
            self.asked.lock().unwrap().push((i, method.clone()));
            let say = |r: String| Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0",{r}}}"#).into_bytes() });
            let blocks: Vec<u64> = b.split("\"0x").skip(1).filter_map(|x| x.split('"').next()).filter(|h| h.len() <= 8).filter_map(|h| u64::from_str_radix(h, 16).ok()).collect();
            let past = i == 2 && blocks.iter().any(|n| *n > head);
            match method.as_str() {
                "eth_chainId" => say(r#""result":"0x7a69""#.into()),
                "eth_blockNumber" => say(format!(r#""result":"0x{head:x}""#)),
                "eth_getBalance" if i == 2 && self.form == 4 => say(r#""result":"0x63""#.into()),
                "eth_getBalance" if past && self.form == 1 => say(r#""error":{"code":-32000,"message":"header not found"}"#.into()),
                "eth_getBalance" => say(r#""result":"0x64""#.into()),
                "eth_getBlockByNumber" if past => say(r#""result":null"#.into()),
                "eth_getBlockByNumber" => {
                    let n = blocks.first().copied().unwrap_or(0);
                    say(format!(r#""result":{{"baseFeePerGas":"0x1","hash":"0x{n:064x}","number":"0x{n:x}","parentHash":"0x{:064x}","timestamp":"0x{:x}","transactions":[]}}"#, n.saturating_sub(1), 1000 + n))
                }
                "eth_getLogs" => {
                    let from = blocks.first().copied().unwrap_or(0);
                    let to = blocks.get(1).copied().unwrap_or(from);
                    let to = if i == 2 { to.min(head) } else { to };
                    let logs: Vec<String> = (from..=to).filter(|n| n % 2 == 0).map(|n| format!(r#"{{"address":"0x00000000000000000000000000000000000000a1","blockHash":"0x{n:064x}","blockNumber":"0x{n:x}","data":"0x","logIndex":"0x0","topics":[],"transactionHash":"0x{n:064x}","transactionIndex":"0x0"}}"#)).collect();
                    say(format!(r#""result":[{}]"#, logs.join(",")))
                }
                "eth_getTransactionReceipt" if i == 2 => say(r#""result":null"#.into()),
                "eth_getTransactionReceipt" => say(format!(r#""result":{{"blockHash":"0x{:064x}","blockNumber":"0x40","gasUsed":"0x5208","status":"0x1","transactionHash":"0x{}"}}"#, 0x40, "01".repeat(32))),
                _ => say(r#""result":null"#.into()),
            }
        }
    }

    #[test]
    fn the_endpoint_behind_the_receipts_block_is_not_asked_in_the_reads_that_follow() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        static PORT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(34_500);
        const WHO: &str = "0x00000000000000000000000000000000000000b0";
        let tx = format!("0x{}", "01".repeat(32));
        for form in 1u8..=5 {
            let a = PORT.fetch_add(3, std::sync::atomic::Ordering::SeqCst);
            let a = a - a % 3;
            let urls: Vec<String> = (0..3).map(|k| format!("http://127.0.0.1:{}", a + k)).collect();
            let table = Table::new(urls.clone()).expect("table");
            let t = Lagging { form, asked: Default::default() };
            let _action = crate::action::open(None);
            let receipt = W::of(crate::wire::Body::Obj(vec![("blockNumber".into(), W::of(crate::wire::Body::Str("0x40".into()))), ("status".into(), W::of(crate::wire::Body::Str("0x1".into())))]));
            for u in &urls[..2] {
                crate::action::note(u, seam::M_GET_RECEIPT, &Value::Arr(vec![Value::Str(tx.clone())]), &receipt);
            }
            let got: R<(usize, Vec<String>)> = match form {
                1 | 4 => balance(&t, &table, WHO, seam::TAG_LATEST).map(|(_, r)| (r.endpoints, r.left.iter().map(|(n, w)| format!("{n}={w}")).collect())),
                2 => height(&t, &table).and_then(|(h, _)| block_by_number(&t, &table, h)).map(|(_, r)| (r.endpoints, r.left.iter().map(|(n, w)| format!("{n}={w}")).collect())),
                3 => logs(&t, &table, "0x00000000000000000000000000000000000000a1", 0x30, seam::Upto::Latest).map(|(l, r)| (l.len() * 100 + r.endpoints, r.left.iter().map(|(n, w)| format!("{n}={w}")).collect())),
                _ => height(&t, &table).and_then(|_| crate::sending::receipt(&t, &table, &tx)).map(|r| (usize::from(r.is_some()), Vec::new())),
            };
            let asked = t.asked.lock().unwrap().clone();
            let behind: Vec<&String> = asked.iter().filter(|(i, m)| *i == 2 && m != "eth_blockNumber" && m != "eth_chainId").map(|(_, m)| m).collect();
            println!("ROW form={form} got={:?} behind-asked={behind:?}", got.as_ref().map_err(|f| format!("{} {}", f.name(), f.detail())));
            let (n, left) = got.unwrap_or_else(|f| panic!("form {form}: read as the two give it: {} {}", f.name(), f.detail()));
            match form {
                3 => assert_eq!(n, 9 * 100 + 2, "form 3: the two's nine logs, two endpoints"),
                5 => assert_eq!(n, 1, "form 5: the receipt, as the two give it"),
                _ => assert_eq!(n, 2, "form {form}: the two at the block"),
            }
            assert!(left.is_empty(), "form {form}: two alike make the reading, nobody listed: {left:?}");
        }
    }

    #[test]
    fn a_block_one_node_gives_beside_a_null_reads_single_and_is_never_refused() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        #[derive(Clone)]
        struct NullBeside {
            heads: [u64; 2],
            asked: std::sync::Arc<std::sync::Mutex<Vec<(usize, String)>>>,
        }
        impl Transport for NullBeside {
            fn post(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
                let b = String::from_utf8_lossy(body).to_string();
                let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
                let i = usize::from(url.port % 2);
                let method = b.split("\"method\":\"").nth(1).and_then(|x| x.split('"').next()).unwrap_or("").to_string();
                self.asked.lock().unwrap().push((i, method.clone()));
                let say = |r: String| Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0",{r}}}"#).into_bytes() });
                match method.as_str() {
                    "eth_chainId" => say(r#""result":"0x7a69""#.into()),
                    "eth_blockNumber" => say(format!(r#""result":"0x{:x}""#, self.heads[i])),
                    "eth_getBlockByNumber" if i == 1 => say(r#""result":null"#.into()),
                    "eth_getBlockByNumber" => say(format!(r#""result":{{"baseFeePerGas":"0x1","hash":"0x{:064x}","number":"0x40","parentHash":"0x{:064x}","timestamp":"0x440","transactions":[]}}"#, 0x40, 0x3f)),
                    _ => say(r#""result":null"#.into()),
                }
            }
        }
        static PORT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(35_600);
        let pauses = crate::judge::not_yet_pauses().len();
        for (heads, asked_again) in [([0x41u64, 0x41], 2usize), ([0x41, 0x3f], pauses + 1)] {
            let a = PORT.fetch_add(2, std::sync::atomic::Ordering::SeqCst);
            let a = a - a % 2;
            let table = Table::new((0..2).map(|k| format!("http://127.0.0.1:{}", a + k)).collect()).expect("table");
            let t = NullBeside { heads, asked: Default::default() };
            let _action = crate::action::open(None);
            let (block, r) = block_by_number(&t, &table, 0x40).unwrap_or_else(|f| panic!("{heads:?}: one node's block, never refused: {} {}", f.name(), f.detail()));
            assert_eq!(block.get("number").and_then(In::as_str), Some("0x40"), "{heads:?}");
            assert_eq!((r.endpoints, r.agreement), (1, Agreement::Single), "{heads:?}: the one node's reading");
            let nulls = t.asked.lock().unwrap().iter().filter(|(i, m)| *i == 1 && m == "eth_getBlockByNumber").count();
            assert_eq!(nulls, asked_again, "{heads:?}: asked again at once when past the block, else after each pause");
        }
    }

    #[test]
    fn an_endpoint_out_of_reach_is_passed_over_in_a_log_read() {
        #[derive(Clone)]
        struct Down(std::sync::Arc<std::sync::atomic::AtomicUsize>);
        impl Transport for Down {
            fn post(&self, _url: &seam::Url, _body: &[u8]) -> R<crate::transport::Answer> {
                self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Err(Fault::Transport("connect 127.0.0.1:1: refused".into()))
            }
        }
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let filter = log_filter(",\"topics\":[\"0x0000000000000000000000000000000000000000000000000000000000000001\"]").unwrap();
        let down = Down(Default::default());
        let f = agreed_logs(&down, &["http://127.0.0.1:19997".into()], &filter, 1, 0, 1 << 20).map(|(l, _)| l.len()).expect_err("out of reach");
        assert_eq!(f.name(), "E_CHAIN_TRANSPORT", "{}", f.detail());
        assert_eq!(down.0.load(std::sync::atomic::Ordering::SeqCst), 1, "out of reach: one ask, not the window's splits");
    }

    #[test]
    fn a_floor_above_the_head_is_no_floor() {
        assert_eq!(floored(0, 26_200_000, 26_108_679), 26_108_679);
        assert_eq!(floored(26_150_000, 26_200_000, 26_108_679), 26_150_000);
        assert_eq!(floored(0, 120, 26_108_679), 0);
        assert_eq!(floored(5, 26_108_679, 26_108_679), 26_108_679);
    }

    #[test]
    fn a_reading_names_who_left_only_when_someone_did() {
        let r = reading(In::Arr(Vec::new()), 2);
        assert_eq!(r.rows().len(), 2);
        let r = Reading { left: vec![("https://a.example".into(), "缩到一块一问仍拒".into())], ..reading(In::Arr(Vec::new()), 1) };
        let rows = r.rows();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[2].0, "left");
        assert_eq!(r.agreement, Agreement::Single);
    }

    #[test]
    fn each_rule_judges_its_methods() {
        use crate::judge::{judge, rule_of, Rule, BLOCK_FACTS};
        let ok = |n: &str, v: &str| (n.to_string(), Ok(crate::wire::parse(v.as_bytes()).unwrap()));
        let none = Value::Arr(Vec::new());
        let j = judge(seam::M_CALL, &none, vec![ok("A", "\"0x1\""), ok("B", "\"0x1\"")]).expect("同");
        assert_eq!((j.value, j.sources.len()), (Value::Str("0x1".into()), 2));
        assert!(matches!(judge(seam::M_CALL, &none, vec![ok("A", "\"0x1\""), ok("B", "\"0x2\"")]), Err(NoReading::Differ(_))));
        let rc = r#"{"blockNumber":"0x5","status":"0x1"}"#;
        let rc_extra = r#"{"blobGasUsed":"0x9","blockNumber":"0x5","status":"0x1"}"#;
        assert!(judge(seam::M_GET_RECEIPT, &none, vec![ok("A", rc), ok("B", rc_extra)]).is_ok(), "不读的成员不比");
        assert!(matches!(judge(seam::M_GET_RECEIPT, &none, vec![ok("A", rc), ok("B", r#"{"blockNumber":"0x5"}"#)]), Err(NoReading::Differ(_))), "点名的事实一处缺即不同");
        let j = judge(seam::M_BLOCK_NUMBER, &none, vec![ok("A", "\"0x10\""), ok("B", "\"0x0e\"")]).expect("取最小");
        assert_eq!(j.value, Value::Str("0x0e".into()));
        let j = judge(seam::M_GET_TX_COUNT, &none, vec![ok("A", "\"0x3\""), ok("B", "\"0x4\"")]).expect("取最大");
        assert_eq!(j.value, Value::Str("0x4".into()));
        let latest = Value::Arr(vec![Value::Str(seam::TAG_LATEST.into()), Value::Bool(false)]);
        assert_eq!(rule_of(seam::M_GET_BLOCK, &latest), Rule::Most(Some("number")));
        assert_eq!(rule_of(seam::M_GET_BLOCK, &Value::Arr(vec![Value::Str("0x5".into()), Value::Bool(false)])), Rule::Facts(&BLOCK_FACTS));
    }

    #[derive(Clone)]
    struct Scene {
        down: Vec<u16>,
        rule: fn(u16, &str) -> String,
        threads: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl Transport for Scene {
        fn post(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            self.threads.lock().unwrap().push(std::thread::current().name().unwrap_or("").to_string());
            if self.down.contains(&url.port) {
                return Err(Fault::Transport(format!("connect {}: refused", url.authority())));
            }
            let b = String::from_utf8_lossy(body).to_string();
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let result = if b.contains(seam::M_CHAIN_ID) { "\"0x7a69\"".to_string() } else { (self.rule)(url.port, &b) };
            Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
        }
    }

    fn table(ports: &[u16]) -> Table {
        Table::new(ports.iter().map(|p| format!("http://127.0.0.1:{p}")).collect()).unwrap()
    }

    #[test]
    fn an_endpoint_that_does_not_answer_is_passed_over_and_named_only_where_the_reading_is_single() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let s = Scene { down: vec![31001], rule: |_, _| "\"0x1\"".into(), threads: Default::default() };
        let r = call(&s, &table(&[31001, 31002, 31003]), "0x0000000000000000000000000000000000000001", "0x", "latest").expect("两家答").1;
        assert_eq!((r.agreement, r.endpoints), (Agreement::Agreed, 2));
        assert!(r.left.is_empty(), "two alike make the reading: nobody listed: {:?}", r.left);
        let r = call(&s, &table(&[31001, 31004]), "0x0000000000000000000000000000000000000001", "0x", "latest").expect("一家答").1;
        assert_eq!((r.agreement, r.endpoints), (Agreement::Single, 1), "只剩一家:单源");
        assert!(r.left.len() == 1 && r.left[0].0.said().ends_with(":31001") && r.left[0].1.starts_with("unanswered E_CHAIN_TRANSPORT"), "single: the one that gave nothing named: {:?}", r.left);
        let all = Scene { down: vec![31005, 31006], rule: |_, _| "\"0x1\"".into(), threads: Default::default() };
        let f = call(&all, &table(&[31005, 31006]), "0x0000000000000000000000000000000000000001", "0x", "latest").unwrap_err();
        assert!(f.name() == "E_CHAIN_TRANSPORT" && f.detail().contains("31005") && f.detail().contains("31006"), "{f:?}");
        let names = s.threads.lock().unwrap().clone();
        assert!(names.iter().any(|n| n.starts_with("eravon-ask-")), "each node on a thread of the family's asking: {names:?}");
    }

    #[test]
    fn a_list_counts_as_checked_only_when_every_endpoint_answered_its_chain() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let s = Scene { down: vec![31031], rule: |_, _| "\"0x1\"".into(), threads: Default::default() };
        let t = table(&[31031, 31032]);
        assert!(pinned_chain(&s, &t).is_ok(), "one answered: the read passes, single");
        assert!(t.chain.get().is_none() && !covered(&t), "but the list is not checked whole");
        assert_eq!(rows_on(&t, 31337), vec!["http://127.0.0.1:31032".to_string()], "the row that answered is checked, alone");
        let all = Scene { down: vec![], rule: |_, _| "\"0x1\"".into(), threads: Default::default() };
        let t2 = table(&[31031, 31032]);
        assert!(pinned_chain(&all, &t2).is_ok());
        heard_all(&t2);
        assert!(covered(&t2), "every endpoint answered: checked");
    }

    fn heard_all(t: &Table) {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !covered(t) && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    fn heard_each(t: &Table) {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let heard = || t.list.iter().all(|e| CHECKED.lock().unwrap_or_else(|x| x.into_inner()).iter().any(|(k, _)| k == e));
        while !heard() && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[derive(Clone)]
    struct Together {
        rows: usize,
        came: std::sync::Arc<(std::sync::Mutex<Vec<u16>>, std::sync::Condvar)>,
    }

    impl Transport for Together {
        fn post(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let b = String::from_utf8_lossy(body).to_string();
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let result = if b.contains(seam::M_CHAIN_ID) {
                let (m, c) = &*self.came;
                let mut g = m.lock().unwrap();
                if !g.contains(&url.port) {
                    g.push(url.port);
                }
                c.notify_all();
                drop(c.wait_timeout_while(g, std::time::Duration::from_secs(30), |g| g.len() < self.rows).unwrap());
                "\"0x7a69\""
            } else {
                "\"0x1\""
            };
            Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
        }
    }

    #[test]
    fn a_row_is_checked_on_its_own_and_a_hung_row_is_never_waited_for() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let all = Scene { down: vec![], rule: |_, _| "\"0x1\"".into(), threads: Default::default() };
        let t = table(&[31801, 31802, 31803]);
        assert!(pinned_chain(&Together { rows: 3, came: Default::default() }, &t).is_ok());
        heard_all(&t);
        assert!(covered(&t) && table_chain(&t) == 31337, "three rows each gave the chain: checked whole");
        assert!(pinned(&all, &t, seam::M_GET_BALANCE).expect("checked").is_none() && t.chain.get().copied() == Some(31337), "checked whole: read as it is, its chain set");
        let one_down = Scene { down: vec![31813], rule: |_, _| "\"0x1\"".into(), threads: Default::default() };
        let t2 = table(&[31811, 31812, 31813]);
        let r = balance(&one_down, &t2, "0x0000000000000000000000000000000000000001", "latest").expect("the read goes on").1;
        assert_eq!(r.endpoints, 2, "read by the two checked rows");
        assert!(t2.chain.get().is_none() && !covered(&t2), "a row that gave nothing: not checked whole");
        assert_eq!(rows_on(&t2, 31337).len(), 2);
        assert_eq!(table_chain(&t2), 31337, "the chain its checked rows answered, never 0");
        #[derive(Clone)]
        struct Other;
        impl Transport for Other {
            fn post(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
                let b = String::from_utf8_lossy(body).to_string();
                let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
                let result = if b.contains(seam::M_CHAIN_ID) { if url.port == 31993 { "\"0x5\"" } else { "\"0x7a69\"" } } else if url.port == 31993 { "\"0x99\"" } else { "\"0x1\"" };
                Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
            }
        }
        let t3 = table(&[31991, 31992, 31993]);
        assert!(pinned_chain(&Other, &t3).is_ok());
        heard_each(&t3);
        let (v, r) = balance(&Other, &t3, "0x0000000000000000000000000000000000000001", "latest").expect("read");
        assert_eq!((v.as_str(), r.endpoints), ("1", 2), "the row on another chain is never read");
        assert!(rows_on(&t3, 31337).len() == 2 && !covered(&t3));
    }

    #[test]
    fn a_read_that_takes_the_largest_waits_for_every_row() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        #[derive(Clone)]
        struct Pool;
        impl Transport for Pool {
            fn post(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
                let b = String::from_utf8_lossy(body).to_string();
                let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
                let result = if b.contains(seam::M_CHAIN_ID) {
                    "\"0x7a69\"".to_string()
                } else if b.contains(seam::M_GET_TX_COUNT) {
                    if url.port == 31823 {
                        std::thread::sleep(std::time::Duration::from_millis(200));
                        "\"0x6\"".to_string()
                    } else {
                        "\"0x5\"".to_string()
                    }
                } else {
                    "\"0x1\"".to_string()
                };
                Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
            }
        }
        let (n, r) = crate::sending::pending_nonce(&Pool, &table(&[31821, 31822, 31823]), "0x0000000000000000000000000000000000000001").expect("read");
        assert_eq!(n, 6, "the largest of every row's answer");
        assert_eq!(r.endpoints, 3);
    }

    #[test]
    fn two_first_reads_together_ask_the_chain_id_once() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        #[derive(Clone)]
        struct Slow {
            ids: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        }
        impl Transport for Slow {
            fn post(&self, _url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
                let b = String::from_utf8_lossy(body).to_string();
                let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
                let result = if b.contains(seam::M_CHAIN_ID) {
                    self.ids.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    std::thread::sleep(std::time::Duration::from_millis(150));
                    "\"0x7a69\""
                } else {
                    "\"0x1\""
                };
                Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
            }
        }
        let s = std::sync::Arc::new(Slow { ids: Default::default() });
        let both: Vec<_> = (0..2)
            .map(|_| {
                let s = s.clone();
                std::thread::spawn(move || call(&*s, &table(&[31831]), "0x0000000000000000000000000000000000000001", "0x", "latest").is_ok())
            })
            .collect();
        assert!(both.into_iter().all(|h| h.join().unwrap()), "both reads answered");
        assert_eq!(s.ids.load(std::sync::atomic::Ordering::SeqCst), 1, "the chain id asked once");
    }

    #[test]
    fn an_endpoint_more_than_the_slack_behind_is_named() {
        assert_eq!(seam::HEIGHT_SLACK, 64);
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        for (gap, ok) in [(63u64, true), (64, true), (65, false)] {
            let rule: fn(u16, &str) -> String = match gap {
                63 => |p, _| if p == 31011 { "\"0x64\"".into() } else { format!("\"0x{:x}\"", 100 + 63) },
                64 => |p, _| if p == 31011 { "\"0x64\"".into() } else { format!("\"0x{:x}\"", 100 + 64) },
                _ => |p, _| if p == 31011 { "\"0x64\"".into() } else { format!("\"0x{:x}\"", 100 + 65) },
            };
            let s = Scene { down: vec![], rule, threads: Default::default() };
            let got = common_height(&s, &table(&[31011, 31012]));
            if ok {
                assert_eq!(got, Ok(100), "差 {gap}");
            } else {
                let f = got.unwrap_err();
                assert!(f.name() == "E_CHAIN_DISAGREE" && f.detail().starts_with("behind ") && f.detail().contains(":31011(块 100)"), "差 {gap}:{f:?}");
            }
        }
    }

    #[test]
    fn a_member_one_provider_adds_below_the_top_is_its_extra() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let s = Scene { down: vec![], rule: |p, _| if p == 31041 { r#"{"blockNumber":"0x5","gasUsed":"0x5208","logs":[{"address":"0x01","blockTimestamp":"0x9"}],"status":"0x1"}"#.into() } else { r#"{"blockNumber":"0x5","gasUsed":"0x5208","logs":[{"address":"0x01"}],"status":"0x1"}"#.into() }, threads: Default::default() };
        let r = read_agreed(&s, &table(&[31041, 31042]), seam::M_GET_RECEIPT, vec![Out::S("0xab".into())]).expect("同一张收据");
        assert_eq!(r.agreement, Agreement::Agreed);
        let logs = r.value.get("logs").and_then(|l| l.as_arr()).expect("logs").to_vec();
        assert!(logs[0].get("blockTimestamp").is_none() && logs[0].get("address").is_some(), "{logs:?}");
        let lacks = Scene { down: vec![], rule: |p, _| if p == 31043 { r#"{"blockNumber":"0x5","status":"0x1"}"#.into() } else { r#"{"blockNumber":"0x5","gasUsed":"0x5208","status":"0x1"}"#.into() }, threads: Default::default() };
        let f = read_agreed(&lacks, &table(&[31043, 31044]), seam::M_GET_RECEIPT, vec![Out::S("0xab".into())]).unwrap_err();
        assert_eq!(f.name(), "E_CHAIN_DISAGREE", "{f:?}");
    }

    const RECEIPTS_EXTRA: &str = r#"[{"blobGasUsed":"0x0","blockHash":"0xb1","cumulativeGasUsed":"0x5208","effectiveGasPrice":"0x7","logs":[{"address":"0x01","blockTimestamp":"0x9","data":"0x","logIndex":"0x0","topics":["0xaa"]}],"logsBloom":"0x00","status":"0x1","transactionHash":"0xt0","type":"0x2"},{"blobGasUsed":"0x0","cumulativeGasUsed":"0xa410","logs":[],"logsBloom":"0x00","status":"0x1","transactionHash":"0xt1","type":"0x0"}]"#;
    const RECEIPTS_PLAIN: &str = r#"[{"blockHash":"0xb1","cumulativeGasUsed":"0x5208","logs":[{"address":"0x01","data":"0x","logIndex":"0x0","topics":["0xaa"]}],"logsBloom":"0x00","status":"0x1","transactionHash":"0xt0","type":"0x2"},{"cumulativeGasUsed":"0xa410","logs":[],"logsBloom":"0x00","status":"0x1","transactionHash":"0xt1","type":"0x0"}]"#;

    fn keys(v: &In) -> Vec<String> {
        match v {
            In::Obj(m) => m.iter().map(|(k, _)| k.clone()).collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn receipts_whose_members_differ_only_beside_the_trie_agree_and_answer_the_covered_members() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let s = Scene { down: vec![], rule: |p, _| if p == 31061 { RECEIPTS_EXTRA.into() } else { RECEIPTS_PLAIN.into() }, threads: Default::default() };
        let t = table(&[31061, 31062]);
        let (list, r) = block_receipts(&s, &t, "0xB1").expect("同一块的回执");
        assert_eq!((r.agreement, r.endpoints, r.left.len()), (Agreement::Agreed, 2, 0));
        assert_eq!(list.len(), 2);
        for rc in &list {
            assert!(keys(rc).iter().all(|k| RECEIPT_COVERED.contains(&k.as_str())), "{rc:?}");
        }
        assert_eq!(keys(&list[0]), ["cumulativeGasUsed", "logs", "logsBloom", "status", "transactionHash", "type"]);
        let log = &list[0].get("logs").and_then(In::as_arr).expect("logs")[0];
        assert_eq!(keys(log), ["address", "data", "topics"]);
        let raw = read_agreed_raw(&s, &t, seam::M_GET_BLOCK_RECEIPTS, "[\"0xb1\"]").expect("读数册重取走同一判法");
        assert_eq!(crate::ask::fingerprint(&raw.value), crate::ask::fingerprint(&In::Arr(list)));
        let swapped = Scene { down: vec![], rule: |p, _| if p == 31071 { RECEIPTS_PLAIN.into() } else { RECEIPTS_EXTRA.into() }, threads: Default::default() };
        let (other, _) = block_receipts(&swapped, &table(&[31071, 31072]), "0xb1").expect("哪家在前都同答");
        assert_eq!(crate::ask::fingerprint(&In::Arr(other)), crate::ask::fingerprint(&raw.value));
    }

    #[test]
    fn receipts_differing_on_a_covered_member_are_refused_naming_the_endpoint_and_the_member() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let cases: [(u16, u16, fn(u16, &str) -> String, &str); 3] = [
            (31073, 31074, |p, _| if p == 31074 { RECEIPTS_PLAIN.replace(r#""status":"0x1","transactionHash":"0xt1""#, r#""status":"0x0","transactionHash":"0xt1""#) } else { RECEIPTS_EXTRA.into() }, ":31074 在 [1].status"),
            (31075, 31076, |p, _| if p == 31076 { RECEIPTS_PLAIN.replace(r#""data":"0x""#, r#""data":"0x01""#) } else { RECEIPTS_EXTRA.into() }, ":31076 在 [0].logs[0].data"),
            (31077, 31078, |p, _| if p == 31078 { RECEIPTS_PLAIN.split_once(",{\"cumulativeGasUsed\":\"0xa410\"").map(|(a, _)| format!("{a}]")).unwrap() } else { RECEIPTS_EXTRA.into() }, ":31078 在 整份(长 1 对 2)"),
        ];
        for (a, b, rule, said) in cases {
            let s = Scene { down: vec![], rule, threads: Default::default() };
            let f = block_receipts(&s, &table(&[a, b]), "0xb1").unwrap_err();
            assert!(f.name() == "E_CHAIN_DISAGREE" && f.detail().starts_with("eth_getBlockReceipts 0xb1 的回执各家说法不一") && f.detail().contains(said), "{f:?}");
        }
    }

    #[test]
    fn receipts_from_one_endpoint_are_single_source() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let s = Scene { down: vec![31079], rule: |_, _| RECEIPTS_EXTRA.into(), threads: Default::default() };
        let (list, r) = block_receipts(&s, &table(&[31079, 31080]), "0xb1").expect("一家答");
        assert_eq!((r.agreement, r.endpoints), (Agreement::Single, 1));
        assert!(r.left.len() == 1 && r.left[0].0.said().ends_with(":31079") && r.left[0].1.starts_with("unanswered E_CHAIN_TRANSPORT"), "{:?}", r.left);
        assert!(list[0].get("blobGasUsed").is_none(), "单源也只答所盖的员");
        let (_, one) = block_receipts(&s, &table(&[31081]), "0xb1").expect("表上一家");
        assert_eq!((one.agreement, one.endpoints), (Agreement::Single, 1));
    }

    #[test]
    fn an_account_proof_is_judged_on_its_node_list() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let s = Scene {
            down: vec![],
            rule: |p, _| {
                if p == 31082 {
                    r#"{"accountProof":["0xf1","0xf2"],"address":"0x000000000000000000000000000000000000AbCd","balance":"0x0","storageProof":[],"extra":"0x1"}"#.into()
                } else {
                    r#"{"accountProof":["0xf1","0xf2"],"address":"0x000000000000000000000000000000000000abcd","balance":"0x0","storageProof":[]}"#.into()
                }
            },
            threads: Default::default(),
        };
        let addr = "0x000000000000000000000000000000000000abcd";
        let (p, r) = account_proof(&s, &table(&[31082, 31083]), addr, 9).expect("同一份证明");
        assert_eq!((r.agreement, keys(&r.value)), (Agreement::Agreed, vec!["accountProof".to_string()]));
        assert_eq!(p.as_arr().map(<[In]>::len), Some(2));
        let off = Scene { down: vec![], rule: |p, _| if p == 31084 { r#"{"accountProof":["0xf1","0xf3"]}"#.into() } else { r#"{"accountProof":["0xf1","0xf2"]}"#.into() }, threads: Default::default() };
        let f = account_proof(&off, &table(&[31084, 31085]), addr, 9).unwrap_err();
        assert!(f.name() == "E_CHAIN_DISAGREE" && f.detail().contains("的账户证明各家说法不一") && f.detail().contains(":31085 在 accountProof[1]"), "{f:?}");
    }

    #[test]
    fn a_block_by_hash_is_judged_on_its_header_on_every_way_in() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let s = Scene {
            down: vec![],
            rule: |p, _| if p == 31086 { r#"{"hash":"0xb1","stateRoot":"0x5","totalDifficulty":"0x0","transactions":[]}"#.into() } else { r#"{"hash":"0xb1","stateRoot":"0x5","transactions":[]}"#.into() },
            threads: Default::default(),
        };
        let r = read_agreed_raw(&s, &table(&[31086, 31087]), seam::M_GET_BLOCK_BY_HASH, "[\"0xb1\",false]").expect("同一块");
        assert_eq!(r.agreement, Agreement::Agreed);
        assert!(r.value.get("totalDifficulty").is_some(), "整块照第一家答");
        let off = Scene { down: vec![], rule: |p, _| if p == 31089 { r#"{"hash":"0xb1","stateRoot":"0x6"}"#.into() } else { r#"{"hash":"0xb1","stateRoot":"0x5"}"#.into() }, threads: Default::default() };
        let f = block_by_hash(&off, &table(&[31088, 31089]), "0xb1").unwrap_err();
        assert!(f.name() == "E_CHAIN_DISAGREE" && f.detail().contains("的区块头各家说法不一") && f.detail().contains(":31089 在 stateRoot"), "{f:?}");
    }

    #[test]
    fn a_block_differing_on_the_newest_header_member_is_refused_at_the_read() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let s = Scene { down: vec![], rule: |p, _| if p == 31091 { r#"{"hash":"0xb1","slotNumber":"0x2"}"#.into() } else { r#"{"hash":"0xb1","slotNumber":"0x1"}"#.into() }, threads: Default::default() };
        let f = block_by_hash(&s, &table(&[31090, 31091]), "0xb1").unwrap_err();
        assert!(f.name() == "E_CHAIN_DISAGREE" && f.detail().contains("的区块头各家说法不一") && f.detail().contains(":31091 在 slotNumber"), "{f:?}");
        assert!(HEADER_MEMBERS.iter().any(|(k, _)| *k == "blockAccessListHash") && HEADER_MEMBERS.last().map(|(k, _)| *k) == Some("slotNumber"));
    }

    #[test]
    fn a_receipt_one_endpoint_lacks_is_asked_again_then_refused() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let s = Scene { down: vec![], rule: |p, _| if p == 31021 { "null".into() } else { r#"{"blockNumber":"0x5","gasUsed":"0x5208","status":"0x1"}"#.into() }, threads: Default::default() };
        let got = read_agreed(&s, &table(&[31021, 31022]), seam::M_GET_RECEIPT, vec![Out::S("0xab".into())]);
        {
            let f = got.unwrap_err();
            assert!(f.name() == "E_CHAIN_DISAGREE" && f.detail().starts_with("not-yet ") && f.detail().contains(":31021 还没有"), "{f:?}");
            let asked = s.threads.lock().unwrap().len();
            assert!(asked >= 2 * 3, "重问两次:{asked}");
        }
    }

    #[derive(Clone)]
    struct Apart {
        asked: std::sync::Arc<std::sync::Mutex<Vec<(u16, String, String)>>>,
    }

    impl Transport for Apart {
        fn post(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let req = eravon::json::parse(body).expect("a question");
            let id = String::from_utf8_lossy(body).split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let method = req.member("method").and_then(Value::as_str).unwrap_or("").to_string();
            let params = match req.member("params") {
                Some(Value::Arr(p)) => p.clone(),
                _ => Vec::new(),
            };
            let written = String::from_utf8_lossy(&eravon::json::canon_bytes(&Value::Arr(params.clone()))).into_owned();
            self.asked.lock().unwrap().push((url.port, method.clone(), written));
            let ahead = u64::from(url.port != 31051);
            let at = |v: Option<&Value>| -> Option<u64> {
                match v?.as_str()? {
                    "latest" => Some(0x10 + ahead),
                    "safe" => Some(0xc + ahead),
                    "finalized" => Some(0x8 + ahead),
                    "pending" => None,
                    n => seam::read_quantity(n).ok(),
                }
            };
            let result = match method.as_str() {
                seam::M_CHAIN_ID => "\"0x7a69\"".to_string(),
                seam::M_BLOCK_NUMBER => format!("\"0x{:x}\"", 0x10 + ahead),
                seam::M_GET_BLOCK => {
                    let n = at(params.first()).expect("a block");
                    format!(r#"{{"baseFeePerGas":"0x7","number":"0x{n:x}","timestamp":"0x{:x}"}}"#, n * 12)
                }
                _ => match at(params.last()) {
                    Some(n) => format!("\"0x{n:x}\""),
                    None => format!("\"0x{:x}\"", 0x20 + ahead),
                },
            };
            Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
        }
    }

    const ADDR: &str = "0x0000000000000000000000000000000000000001";

    #[test]
    fn a_state_read_at_latest_is_asked_at_the_block_every_endpoint_has_reached() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let s = Apart { asked: Default::default() };
        let t = table(&[31051, 31052]);
        let call = Out::O(vec![("data", Out::S("0x".into())), ("to", Out::S(ADDR.into()))]);
        let reads: [(&'static str, Vec<Out>); 5] = [
            (seam::M_CALL, vec![call, Out::S(seam::TAG_LATEST.into())]),
            (seam::M_GET_BALANCE, vec![Out::S(ADDR.into()), Out::S(seam::TAG_LATEST.into())]),
            (seam::M_GET_CODE, vec![Out::S(ADDR.into()), Out::S(seam::TAG_LATEST.into())]),
            ("eth_getStorageAt", vec![Out::S(ADDR.into()), Out::S("0x0".into()), Out::S(seam::TAG_LATEST.into())]),
            (seam::M_GET_TX_COUNT, vec![Out::S(ADDR.into()), Out::S(seam::TAG_LATEST.into())]),
        ];
        for (m, ps) in reads {
            let r = read_agreed(&s, &t, m, ps).unwrap_or_else(|f| panic!("{m}:{f:?}"));
            assert_eq!((r.value.as_str(), r.agreement, r.endpoints), (Some("0x10"), Agreement::Agreed, 2), "{m}");
        }
        let raw = read_agreed_raw(&s, &t, seam::M_GET_BALANCE, &format!("[\"{ADDR}\",\"latest\"]")).expect("the raw entry too");
        assert_eq!(raw.value.as_str(), Some("0x10"));
        let asked = s.asked.lock().unwrap().clone();
        assert!(asked.iter().all(|(_, _, p)| !p.contains("\"latest\"")), "{asked:?}");
    }

    #[test]
    fn safe_and_finalized_are_asked_at_the_lowest_block_any_endpoint_reports() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let s = Apart { asked: Default::default() };
        let t = table(&[31051, 31054]);
        for (tag, want) in [(seam::TAG_FINALIZED, "0x8"), (seam::TAG_SAFE, "0xc")] {
            let r = read_agreed(&s, &t, seam::M_GET_BLOCK, vec![Out::S(tag.into()), Out::B(false)]).unwrap_or_else(|f| panic!("{tag}:{f:?}"));
            assert_eq!((r.value.get("number").and_then(|n| n.as_str()), r.agreement), (Some(want), Agreement::Agreed), "{tag}");
            let call = Out::O(vec![("data", Out::S("0x".into())), ("to", Out::S(ADDR.into()))]);
            let r = read_agreed(&s, &t, seam::M_CALL, vec![call, Out::S(tag.into())]).unwrap_or_else(|f| panic!("{tag}:{f:?}"));
            assert_eq!(r.value.as_str(), Some(want), "{tag}");
        }
        let asked = s.asked.lock().unwrap().clone();
        assert!(asked.iter().filter(|(_, m, _)| m == seam::M_CALL).all(|(_, _, p)| !p.contains("safe") && !p.contains("finalized")), "{asked:?}");
    }

    #[test]
    fn pending_keeps_its_tag_and_a_single_endpoint_pins_its_tags() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let s = Apart { asked: Default::default() };
        let r = read_agreed(&s, &table(&[31051, 31056]), seam::M_GET_TX_COUNT, vec![Out::S(ADDR.into()), Out::S("pending".into())]).expect("pending");
        assert!(r.value.as_str().is_some());
        let call = Out::O(vec![("data", Out::S("0x".into())), ("to", Out::S(ADDR.into()))]);
        read_agreed(&s, &table(&[31057]), seam::M_CALL, vec![call, Out::S(seam::TAG_LATEST.into())]).expect("one endpoint");
        read_agreed(&s, &table(&[31057]), seam::M_GET_BLOCK, vec![Out::S(seam::TAG_FINALIZED.into()), Out::B(false)]).expect("one endpoint");
        let asked = s.asked.lock().unwrap().clone();
        assert!(asked.iter().all(|(p, m, _)| *p == 31057 || m != seam::M_BLOCK_NUMBER), "pending asks no head: {asked:?}");
        assert!(asked.iter().any(|(_, m, p)| m == seam::M_GET_TX_COUNT && p.contains("\"pending\"")), "{asked:?}");
        assert!(asked.iter().any(|(p, m, _)| *p == 31057 && m == seam::M_BLOCK_NUMBER), "one endpoint's head asked: {asked:?}");
        assert!(asked.iter().all(|(p, m, x)| !(*p == 31057 && m == seam::M_CALL && x.contains("\"latest\""))), "{asked:?}");
    }
}

#[cfg(test)]
mod batched {
    use super::*;

    #[derive(Clone)]
    struct Raws {
        takes: bool,
        drop: Option<String>,
        asked: std::sync::Arc<std::sync::Mutex<Vec<(usize, String)>>>,
    }

    fn raw_of(h: &str) -> String {
        format!("0xee{}", &h[2..10])
    }

    fn one(id: &In, result: &str) -> String {
        let id = match id {
            In::Int(n) => n.to_string(),
            _ => "0".into(),
        };
        format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#)
    }

    impl Transport for Raws {
        fn post(&self, _url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let v = eravon_literals::json::read(body).expect("问是 JSON");
            let first_param = |q: &In| q.get("params").and_then(In::as_arr).and_then(|a| a.first()).and_then(In::as_str).unwrap_or_default().to_string();
            let result_of = |q: &In| match q.get("method").and_then(In::as_str).unwrap_or_default() {
                "eth_chainId" => "\"0x7a69\"".to_string(),
                "eth_blockNumber" => "\"0x40\"".to_string(),
                _ => format!("\"{}\"", raw_of(&first_param(q))),
            };
            let answer = |status: u16, body: String| Ok(crate::transport::Answer { status, location: None, body: body.into_bytes() });
            if let In::Arr(qs) = &v {
                let method = qs.first().and_then(|q| q.get("method")).and_then(In::as_str).unwrap_or_default().to_string();
                self.asked.lock().unwrap().push((qs.len(), method));
                if !self.takes {
                    return answer(400, r#"{"error":{"code":-32600,"message":"batch not supported"}}"#.into());
                }
                let rows: Vec<String> = qs
                    .iter()
                    .filter(|q| self.drop.as_deref() != Some(first_param(q).as_str()))
                    .map(|q| one(q.get("id").unwrap_or(&In::Null), &result_of(q)))
                    .collect();
                return answer(200, format!("[{}]", rows.join(",")));
            }
            let method = v.get("method").and_then(In::as_str).unwrap_or_default().to_string();
            self.asked.lock().unwrap().push((1, method.clone()));
            let id = v.get("id").cloned().unwrap_or(In::Null);
            answer(200, one(&id, &result_of(&v)))
        }
    }

    fn raws(takes: bool, drop: Option<String>) -> Raws {
        Raws { takes, drop, asked: Default::default() }
    }

    fn hashes(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("0x{:064x}", i + 1)).collect()
    }

    fn want(hs: &[String]) -> Vec<Vec<u8>> {
        hs.iter().map(|h| seam::bytes_of("期", &raw_of(h)).unwrap()).collect()
    }

    fn table(port: u16) -> Table {
        Table::new(vec![format!("http://127.0.0.1:{port}")]).unwrap()
    }

    fn raw_asks(t: &Raws) -> (usize, usize) {
        let a = t.asked.lock().unwrap();
        let raw: Vec<&(usize, String)> = a.iter().filter(|(_, m)| m == seam::M_GET_RAW_TX).collect();
        (raw.len(), raw.iter().map(|(n, _)| n).sum())
    }

    #[test]
    fn a_block_of_transactions_is_asked_in_batches() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let _a = crate::action::open(None);
        for (port, n, requests) in [(31201u16, 3usize, 1usize), (31202, 250, 13)] {
            let t = raws(true, None);
            let hs = hashes(n);
            let w = want(&hs);
            let got = block_transactions_proven(&t, &table(port), &hs, |x| if x == w.as_slice() { Ok(()) } else { Err("不同".into()) }).expect("批取");
            assert_eq!(got, w);
            assert_eq!(raw_asks(&t), (requests, n), "{n} 笔 {requests} 问");
        }
    }

    #[test]
    fn a_batch_short_or_refused_falls_back_one_at_a_time() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let _a = crate::action::open(None);
        let hs = hashes(4);
        let w = want(&hs);
        let proof = |x: &[Vec<u8>]| if x == w.as_slice() { Ok(()) } else { Err("不同".into()) };
        let short = raws(true, Some(hs[2].clone()));
        assert_eq!(block_transactions_proven(&short, &table(31203), &hs, proof).expect("补问"), w);
        assert_eq!(raw_asks(&short), (2, 5), "批量一问四笔,缺的那一笔单问一回");
        let no = raws(false, None);
        assert_eq!(block_transactions_proven(&no, &table(31204), &hs, proof).expect("退逐笔"), w);
        let batches = |t: &Raws| t.asked.lock().unwrap().iter().filter(|(n, _)| *n > 1).count();
        assert_eq!(batches(&no), 1, "只试过一回批量");
        assert_eq!(raw_asks(&no), (4, 4), "一批被拒,四笔逐笔");
        assert!(unbatched().iter().any(|(e, m)| e.contains("31204") && m == crate::ahead::WHOLE), "具名记下:{:?}", unbatched());
        assert_eq!(block_transactions_proven(&no, &table(31204), &hs, proof).expect("再取"), w);
        assert_eq!(raw_asks(&no), (8, 8), "同一进程不再试批量");
        assert_eq!(batches(&no), 1, "同一进程不再试批量");
    }

    #[derive(Clone)]
    struct Kept {
        heads: std::sync::Arc<std::sync::Mutex<Vec<(u16, u64)>>>,
        logs: Vec<(&'static str, u64)>,
        asked: std::sync::Arc<std::sync::Mutex<Vec<(u16, String, u64, u64)>>>,
        climb: bool,
    }

    impl Transport for Kept {
        fn post(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let b = String::from_utf8_lossy(body).to_string();
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let hex = |n: u64| format!("0x{n:x}");
            let q = |k: &str| b.split(&format!("\"{k}\":\"0x")).nth(1).and_then(|x| x.split('"').next()).and_then(|h| u64::from_str_radix(h, 16).ok()).unwrap_or(0);
            let result = if b.contains("eth_blockNumber") {
                let mut hs = self.heads.lock().unwrap();
                let h = hs.iter_mut().find(|(p, _)| *p == url.port).map(|(_, h)| h).expect("a node of the test");
                let now = *h;
                if self.climb {
                    *h += 1;
                }
                format!("\"{}\"", hex(now))
            } else if b.contains("eth_getBlockByNumber") {
                let n = b.split("\"params\":[\"0x").nth(1).and_then(|x| x.split('"').next()).and_then(|h| u64::from_str_radix(h, 16).ok()).unwrap_or(0);
                format!(r#"{{"hash":"0x{}{n:04x}","number":"{}"}}"#, "bb".repeat(30), hex(n))
            } else if b.contains("eth_getLogs") {
                let (from, to) = (q("fromBlock"), q("toBlock"));
                let mut addrs: Vec<&str> = self.logs.iter().map(|(a, _)| *a).filter(|a| b.contains(a)).collect();
                addrs.dedup();
                self.asked.lock().unwrap().push((url.port, addrs.join(","), from, to));
                let rows: Vec<String> = self
                    .logs
                    .iter()
                    .filter(|(a, n)| addrs.contains(a) && (from..=to).contains(n))
                    .map(|(a, n)| format!(r#"{{"address":"{a}","blockHash":"0x{}{n:04x}","blockNumber":"{}","data":"0x","logIndex":"0x0","topics":[],"transactionHash":"0x{}{n:04x}"}}"#, "bb".repeat(30), hex(*n), "cc".repeat(30)))
                    .collect();
                format!("[{}]", rows.join(","))
            } else {
                "null".to_string()
            };
            Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
        }
    }

    #[derive(Default)]
    struct Record(std::sync::Mutex<std::collections::BTreeMap<String, Vec<u8>>>);

    impl crate::settled::Keep for Record {
        fn load(&self, part: &str) -> Option<Vec<u8>> {
            self.0.lock().unwrap().get(part).cloned()
        }
        fn save(&self, part: &str, plain: &[u8]) -> Result<(), crate::settled::Unkept> {
            self.0.lock().unwrap().insert(part.to_string(), plain.to_vec());
            Ok(())
        }
        fn parts(&self) -> Vec<String> {
            self.0.lock().unwrap().keys().cloned().collect()
        }
    }

    const A: &str = "0x00000000000000000000000000000000000000a1";
    const B: &str = "0x00000000000000000000000000000000000000b2";

    fn kept_chain(port: u16, head: u64) -> (Kept, Table) {
        let t = Kept { heads: std::sync::Arc::new(std::sync::Mutex::new(vec![(port, head)])), logs: vec![(A, 100), (A, 900), (B, 500)], asked: Default::default(), climb: false };
        let table = Table::new(vec![format!("http://127.0.0.1:{port}")]).unwrap();
        let _ = table.chain.set(1);
        (t, table)
    }

    #[test]
    fn a_kept_read_asks_only_after_what_it_settled_and_each_filter_is_its_own() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let (t, table) = kept_chain(30201, 1_000);
        let record: std::sync::Arc<Record> = Default::default();
        let read = |addr: &str| {
            let _a = crate::action::open(Some(record.clone()));
            logs(&t, &table, addr, 0, seam::Upto::Latest).expect("read").0.len()
        };
        assert_eq!(read(A), 2);
        assert_eq!(read(B), 1, "the other filter answers its own log, never the first's");
        t.heads.lock().unwrap()[0].1 = 1_100;
        assert_eq!(read(A), 2, "the kept part and the new part");
        let asked = t.asked.lock().unwrap().clone();
        let settled = 1_000 - crate::settled::MARGIN;
        assert_eq!(asked.iter().map(|(_, a, f, to)| (a.as_str(), *f, *to)).collect::<Vec<_>>(), vec![(A, 0, 1_000), (B, 0, 1_000), (A, settled + 1, 1_100)], "the second read of A asks only after its settled block");
        let _none = crate::action::open(None);
        assert_eq!(logs(&t, &table, A, 0, seam::Upto::Latest).expect("read").0.len(), 2);
        assert_eq!(t.asked.lock().unwrap().last().map(|(_, _, f, _)| *f), Some(0), "no keeper: the whole range");
    }

    #[derive(Clone)]
    struct Lagging {
        short: std::sync::Arc<std::sync::atomic::AtomicU32>,
        asked: std::sync::Arc<std::sync::Mutex<Vec<(u16, u64, u64)>>>,
    }

    impl Transport for Lagging {
        fn post(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let b = String::from_utf8_lossy(body).to_string();
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let q = |k: &str| b.split(&format!("\"{k}\":\"0x")).nth(1).and_then(|x| x.split('"').next()).and_then(|h| u64::from_str_radix(h, 16).ok()).unwrap_or(0);
            let result = if b.contains("eth_blockNumber") {
                "\"0x3e8\"".to_string()
            } else if b.contains("eth_getBlockByNumber") {
                let n = b.split("\"params\":[\"0x").nth(1).and_then(|x| x.split('"').next()).and_then(|h| u64::from_str_radix(h, 16).ok()).unwrap_or(0);
                format!(r#"{{"hash":"0x{}{n:04x}","number":"0x{n:x}"}}"#, "bb".repeat(30))
            } else if b.contains("eth_getLogs") {
                let (from, to) = (q("fromBlock"), q("toBlock"));
                self.asked.lock().unwrap().push((url.port, from, to));
                let short = url.port % 2 == 0 && {
                    let left = self.short.load(std::sync::atomic::Ordering::SeqCst);
                    if left > 0 && left != u32::MAX {
                        self.short.store(left - 1, std::sync::atomic::Ordering::SeqCst);
                    }
                    left > 0
                };
                let rows: Vec<String> = [100u64, 900]
                    .into_iter()
                    .filter(|n| (from..=to).contains(n) && !(short && *n == 100))
                    .map(|n| format!(r#"{{"address":"{A}","blockHash":"0x{}{n:04x}","blockNumber":"0x{n:x}","data":"0x","logIndex":"0x0","topics":[],"transactionHash":"0x{}{n:04x}"}}"#, "bb".repeat(30), "cc".repeat(30)))
                    .collect();
                format!("[{}]", rows.join(","))
            } else if b.contains("eth_chainId") {
                "\"0x1\"".to_string()
            } else {
                "null".to_string()
            };
            Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
        }
    }

    #[test]
    fn a_node_that_once_answered_short_is_asked_again_and_the_read_stands() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let table = Table::new(vec!["http://127.0.0.1:30211".into(), "http://127.0.0.1:30212".into()]).unwrap();
        let _ = table.chain.set(1);
        let record: std::sync::Arc<Record> = Default::default();
        let t = Lagging { short: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(1)), asked: Default::default() };
        let read = |t: &Lagging| {
            let _a = crate::action::open(Some(record.clone()));
            logs(t, &table, A, 0, seam::Upto::Latest)
        };
        let first = read(&t).expect_err("the short one and the whole one apart, two places: refused this once");
        assert_eq!(first.name(), "E_CHAIN_DISAGREE", "{}", first.detail());
        assert_eq!(t.asked.lock().unwrap().len(), 2, "each node asked once, their own lists judged from what the action holds");
        let second = read(&t).expect("nothing kept short: both asked whole, alike");
        assert_eq!(second.0.len(), 2);
        let asked = t.asked.lock().unwrap().clone();
        assert!(asked[2..].iter().all(|(_, f, _)| *f == 0), "the second read asks every node the whole range afresh: {asked:?}");
        let third = read(&t).expect("read on what the two settled");
        assert_eq!(third.0.len(), 2);
        let asked = t.asked.lock().unwrap().clone();
        assert!(asked[4..].iter().all(|(_, f, _)| *f > 0), "the third asks only past what was settled: {asked:?}");
        let always = Lagging { short: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(u32::MAX)), asked: Default::default() };
        let fresh: std::sync::Arc<Record> = Default::default();
        let still = {
            let _a = crate::action::open(Some(fresh));
            logs(&always, &table, A, 0, seam::Upto::Latest).expect_err("a node still short")
        };
        assert_eq!(still.name(), "E_CHAIN_DISAGREE", "{}", still.detail());
        let bare = Lagging { short: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(u32::MAX)), asked: Default::default() };
        {
            let _none = crate::action::open(None);
            assert_eq!(logs(&bare, &table, A, 0, seam::Upto::Latest).expect_err("apart").name(), "E_CHAIN_DISAGREE");
        }
        assert_eq!(bare.asked.lock().unwrap().len(), 2, "nothing kept: asked once, refused at once");
    }

    #[test]
    fn several_contracts_are_one_log_question() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let (t, table) = kept_chain(30202, 1_000);
        let _a = crate::action::open(None);
        let (rows, _) = logs_at_any(&t, &table, &[A.to_string(), B.to_string()], 0, seam::Upto::Latest).expect("read");
        assert_eq!(rows.len(), 3);
        assert_eq!(t.asked.lock().unwrap().len(), 1, "one question for both");
        assert!(logs_at_any(&t, &table, &[], 0, seam::Upto::Latest).is_err(), "no address: refused by name");
    }

    #[test]
    fn a_head_stands_on_the_receipt_the_action_read() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let t = Kept { heads: std::sync::Arc::new(std::sync::Mutex::new(vec![(30203, 50), (30204, 47)])), logs: Vec::new(), asked: Default::default(), climb: true };
        let table = Table::new(vec!["http://127.0.0.1:30203".into(), "http://127.0.0.1:30204".into()]).unwrap();
        let _ = table.chain.set(1);
        {
            let _a = crate::action::open(None);
            assert_eq!(common_height(&t, &table).expect("a head"), 47, "no receipt: the lowest head at once");
        }
        {
            let _a = crate::action::open(None);
            crate::action::landed(table.nodes()[0].written(), 50);
            assert!(common_height(&t, &table).expect("a head") >= 50, "the head stands on the receipt's block");
        }
        let slow = Kept { heads: std::sync::Arc::new(std::sync::Mutex::new(vec![(30205, 50), (30206, 10)])), logs: Vec::new(), asked: Default::default(), climb: false };
        let table = Table::new(vec!["http://127.0.0.1:30205".into(), "http://127.0.0.1:30206".into()]).unwrap();
        let _ = table.chain.set(1);
        let _a = crate::action::open(None);
        crate::action::landed(table.nodes()[0].written(), 50);
        let f = common_height(&slow, &table).expect_err("never reached");
        assert!(f.detail().starts_with("behind") && f.detail().contains("30206"), "{}", f.detail());
    }

    #[test]
    fn only_an_unanswered_stretch_is_owed() {
        let seg = |why: &str| crate::recall::Seg { lo: 0, agreement: Agreement::Single, endpoints: 1, left: vec![(NodeAt::new("http://127.0.0.1:1"), why.to_string())] };
        assert!(seg("E_CHAIN_ENDPOINT limited to a 10,000 range").read(), "stood aside: read");
        assert!(!seg(&format!("{UNANSWERED}E_CHAIN_TRANSPORT:connect refused")).read(), "unanswered: owed");
    }

    #[test]
    fn a_filter_reads_as_the_familys() {
        let t1 = format!("0x{}", "11".repeat(32));
        let t2 = format!("0x{}", "22".repeat(32));
        let f = log_filter(&format!(",\"address\":[\"{A}\",\"{B}\"],\"topics\":[null,\"{t1}\",[\"{t1}\",\"{t2}\"]]")).expect("reads");
        assert_eq!(f.addresses.len(), 2);
        assert_eq!(f.topics.len(), 3);
        assert!(matches!(f.topics[0], crate::scan::Topic::Any) && matches!(f.topics[1], crate::scan::Topic::One(_)) && matches!(&f.topics[2], crate::scan::Topic::AnyOf(x) if x.len() == 2));
        assert!(log_filter(",\"topics\":[\"0x01\"]").is_err(), "a short topic");
        assert!(log_filter(",\"address\":\"0x01\"").is_err(), "a short address");
        assert!(log_filter(&format!(",\"address\":\"{A}\"")).expect("reads").topics.is_empty());
    }

    #[derive(Clone)]
    struct Voided {
        slow: u64,
        asked: std::sync::Arc<std::sync::Mutex<Vec<(u16, String)>>>,
    }

    impl Transport for Voided {
        fn post(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let b = String::from_utf8_lossy(body).to_string();
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let method = b.split("\"method\":\"").nth(1).and_then(|x| x.split('"').next()).unwrap_or("").to_string();
            if url.port % 3 == 2 {
                std::thread::sleep(std::time::Duration::from_millis(self.slow));
            }
            self.asked.lock().unwrap().push((url.port, method.clone()));
            let result = match method.as_str() {
                "eth_chainId" => "\"0x7a69\"",
                "eth_getTransactionCount" => "\"0x5\"",
                _ => "null",
            };
            Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
        }
    }

    #[test]
    fn a_void_is_read_from_the_first_two_places_that_settle_it() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let table = Table::new((0..3).map(|k| format!("http://127.0.0.1:{}", 39_099 + k)).collect()).unwrap();
        let _ = table.chain.set(31337);
        let t = Voided { slow: 1_500, asked: Default::default() };
        let start = std::time::Instant::now();
        let said = void_read(&t, &table, "0x00000000000000000000000000000000000000b0", &format!("0x{}", "ab".repeat(32)), 4).expect("read");
        let took = start.elapsed();
        let came: Vec<&VoidSaid> = said.iter().filter_map(|(_, v)| v.as_ref()).collect();
        assert!(took < std::time::Duration::from_millis(1_400), "the slow third not waited for: {took:?}");
        assert_eq!(came, vec![&VoidSaid::Past(5, Some(true)), &VoidSaid::Past(5, Some(true))], "the two that settled it: {said:?}");
        assert!(said[2].1.is_none(), "the third's word is no part of it");
    }

    #[derive(Clone)]
    struct Paced {
        slow: u64,
        t0: std::time::Instant,
        posts: std::sync::Arc<std::sync::Mutex<Vec<(u16, usize, u128)>>>,
    }

    impl Transport for Paced {
        fn post(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let b = String::from_utf8_lossy(body).to_string();
            let qs: Vec<String> = b.split("\"id\":").skip(1).map(|x| x.split([',', '}']).next().unwrap_or("0").to_string()).collect();
            let pause = if url.port % 3 == 2 { self.slow } else { 30 };
            std::thread::sleep(std::time::Duration::from_millis(pause));
            self.posts.lock().unwrap().push((url.port, qs.len(), self.t0.elapsed().as_millis()));
            let one = |id: &str| format!(r#"{{"id":{id},"jsonrpc":"2.0","result":[]}}"#);
            let out = if b.trim_start().starts_with('[') { format!("[{}]", qs.iter().map(|i| one(i)).collect::<Vec<_>>().join(",")) } else { one(qs.first().map(String::as_str).unwrap_or("0")) };
            Ok(crate::transport::Answer { status: 200, location: None, body: out.into_bytes() })
        }
    }

    struct Alone<T: Transport>(Node<T>);

    impl<T: Transport> crate::rpc::Endpoint for Alone<T> {
        fn call(&mut self, method: &str, params: &Value) -> Result<W, Trouble> {
            self.0.call(method, params)
        }
        fn call_batch(&mut self, asks: &[crate::rpc::Ask<'_>]) -> Option<Result<crate::rpc::Batched, Trouble>> {
            self.0.call_batch(asks)
        }
        fn name(&self) -> String {
            self.0.name()
        }
        fn place(&self) -> String {
            crate::rpc::Endpoint::place(&self.0)
        }
    }

    #[test]
    fn log_reads_asked_ahead_go_to_every_node_at_once_through_another_way() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let filters: Vec<crate::scan::LogFilter> = (1u8..=3).map(|k| crate::scan::LogFilter { addresses: vec![[k; 20]], topics: Vec::new() }).collect();
        let reads: Vec<(&crate::scan::LogFilter, u64, u64)> = filters.iter().map(|f| (f, 0, 1_000)).collect();
        let run = |base: u16, another: bool| {
            let t = Paced { slow: 600, t0: std::time::Instant::now(), posts: Default::default() };
            let nodes: Vec<NodeAt> = (0..3).map(|k| NodeAt::new(format!("http://127.0.0.1:{}", base + k))).collect();
            let list = crate::settled::nodes_of(nodes.iter().map(NodeAt::place).collect::<Vec<_>>().iter().map(String::as_str));
            let _a = crate::action::open(None);
            let start = std::time::Instant::now();
            if another {
                let mut eps: Vec<Node<Paced>> = nodes.iter().map(|n| Node::new(&t, n.clone())).collect();
                let mut refs: Vec<&mut (dyn crate::rpc::Endpoint + Send)> = eps.iter_mut().map(|e| e as &mut (dyn crate::rpc::Endpoint + Send)).collect();
                crate::scan::logs_ahead(&mut refs, 1, &reads, Some(&list));
            } else {
                let mut eps: Vec<Alone<Paced>> = nodes.iter().map(|n| Alone(Node::new(&t, n.clone()))).collect();
                let mut refs: Vec<&mut (dyn crate::rpc::Endpoint + Send)> = eps.iter_mut().map(|e| e as &mut (dyn crate::rpc::Endpoint + Send)).collect();
                crate::scan::logs_ahead(&mut refs, 1, &reads, Some(&list));
            }
            let took = start.elapsed().as_millis();
            let posts = t.posts.lock().unwrap().clone();
            (took, posts)
        };
        let (with, with_posts) = run(39_001, true);
        let (without, without_posts) = run(39_004, false);
        let questions = |p: &[(u16, usize, u128)]| p.iter().map(|(_, q, _)| q).sum::<usize>();
        println!("ROW ahead-another took={with}ms posts={with_posts:?} questions={}", questions(&with_posts));
        println!("ROW ahead-alone took={without}ms posts={without_posts:?} questions={}", questions(&without_posts));
        assert!(with < 600, "through another way: two places enough, the slow node not waited for: {with} ms");
        assert!(without >= 600 + 2 * 30, "alone: one after another, the slow one waited for: {without} ms");
        assert_eq!(questions(&without_posts), 9, "alone: every node asked its three questions");
    }
}
