use eravon::json::{canon_bytes, Value};
use eravon_literals::json::{In, Out};

use crate::fault::{Fault, R};
use crate::rpc::{self, Endpoint, Trouble};
use crate::seam::{self, NodeAt};
use crate::transport::Transport;
use crate::wire::{self, W};

fn next_id() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}

static ASKED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn asked() -> u64 {
    ASKED.load(std::sync::atomic::Ordering::SeqCst)
}

pub const ENV_PAUSE_MS: &str = "ERAVON_PATIENCE_MS";

fn scene_pause(value: Option<&str>) -> Option<std::time::Duration> {
    value.and_then(|v| v.trim().parse::<u64>().ok()).map(std::time::Duration::from_millis)
}

pub(crate) fn scene() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if let Some(d) = scene_pause(std::env::var(ENV_PAUSE_MS).ok().as_deref()) {
            crate::patience::set_waits(Some(d));
        }
    });
}

pub fn value_of(o: &Out) -> Value {
    match o {
        Out::S(s) => Value::Str(s.clone()),
        Out::N(n) => Value::Int(*n),
        Out::B(b) => Value::Bool(*b),
        Out::A(a) => Value::Arr(a.iter().map(value_of).collect()),
        Out::O(m) => Value::Obj(m.iter().map(|(k, v)| ((*k).to_string(), value_of(v))).collect()),
        Out::Null => Value::Null,
    }
}

pub fn each_question(body: &[u8], one: impl Fn(&[u8]) -> R<crate::transport::Answer>) -> R<crate::transport::Answer> {
    let Ok(In::Arr(qs)) = eravon_literals::json::read(body) else { return one(body) };
    let mut parts: Vec<String> = Vec::with_capacity(qs.len());
    for q in &qs {
        let a = one(&eravon_literals::json::write_in(q))?;
        if a.status != 200 {
            return Ok(a);
        }
        parts.push(String::from_utf8_lossy(&a.body).to_string());
    }
    Ok(crate::transport::Answer { status: 200, location: None, body: format!("[{}]", parts.join(",")).into_bytes() })
}

pub fn params_of(params: &[Out]) -> Value {
    Value::Arr(params.iter().map(value_of).collect())
}

pub fn in_of(v: &Value) -> In {
    match v {
        Value::Null => In::Null,
        Value::Bool(b) => In::Bool(*b),
        Value::Int(n) => In::Int(*n as i64),
        Value::Str(s) => In::Str(s.clone()),
        Value::Arr(a) => In::Arr(a.iter().map(in_of).collect()),
        Value::Obj(m) => In::Obj(m.iter().map(|(k, x)| (k.clone(), in_of(x))).collect()),
    }
}

pub fn core_of(v: &In) -> Option<Value> {
    Some(match v {
        In::Null => Value::Null,
        In::Bool(b) => Value::Bool(*b),
        In::Int(n) => Value::Int(u64::try_from(*n).ok()?),
        In::Str(s) => Value::Str(s.clone()),
        In::Arr(a) => Value::Arr(a.iter().map(core_of).collect::<Option<_>>()?),
        In::Obj(m) => Value::Obj(m.iter().map(|(k, x)| Some((k.clone(), core_of(x)?))).collect::<Option<_>>()?),
    })
}

pub fn params_text(params_json: &str) -> R<Value> {
    wire::parse(params_json.as_bytes())
        .and_then(|w| wire::to_core(&w))
        .ok_or_else(|| Fault::Usage(format!("一问的参数读不成值:{params_json}")))
}

pub const BODY_SEP: &str = " · ";

#[derive(Default)]
struct Said {
    kept: std::sync::Mutex<Option<Fault>>,
    echoed: std::sync::Mutex<Option<W>>,
}

impl Said {
    fn keep(&self, f: Option<Fault>) {
        *self.kept.lock().unwrap_or_else(|e| e.into_inner()) = f;
    }
    fn take(&self) -> Option<Fault> {
        self.kept.lock().unwrap_or_else(|e| e.into_inner()).take()
    }
    fn echo(&self, w: Option<W>) {
        *self.echoed.lock().unwrap_or_else(|e| e.into_inner()) = w;
    }
}

pub struct Node<T: Transport> {
    t: T,
    node: NodeAt,
    said: std::sync::Arc<Said>,
    ep: Box<dyn Endpoint + Send>,
}

impl<T: Transport> Node<T> {
    pub fn new(t: &T, node: NodeAt) -> Node<T> {
        scene();
        let said = std::sync::Arc::new(Said::default());
        let channel = Channel::open(t.clone(), node.clone(), said.clone());
        let ep = crate::endpoints::remembering(node.written(), Box::new(channel));
        Node { t: t.clone(), node, said, ep }
    }

    pub fn node(&self) -> &NodeAt {
        &self.node
    }

    pub fn fault(&mut self, t: Trouble) -> Fault {
        classify(crate::fault::of_trouble(&self.node, t, self.said.take()))
    }

    pub fn echoed(&self) -> Option<W> {
        self.said.echoed.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn refused(&self, method: &str) -> Option<Trouble> {
        let f = crate::recall::refuse(method)?;
        let b = rpc::Broke::Said(f.detail());
        self.said.keep(Some(f));
        Some(Trouble::Transport(b))
    }

    fn at_once(&self, method: &str) -> Option<Trouble> {
        self.said.keep(None);
        if let Some(t) = self.refused(method) {
            return Some(t);
        }
        (method != seam::M_BLOCK_NUMBER).then(|| crate::read::passed_over(self.node.written())).flatten()
    }
}

impl<T: Transport> Endpoint for Node<T> {
    fn call(&mut self, method: &str, params: &Value) -> Result<W, Trouble> {
        match self.at_once(method) {
            Some(t) => Err(t),
            None => self.ep.call(method, params),
        }
    }
    fn call_within(&mut self, method: &str, params: &Value, within: std::time::Duration) -> Result<W, Trouble> {
        match self.at_once(method) {
            Some(t) => Err(t),
            None => self.ep.call_within(method, params, within),
        }
    }
    fn call_batch(&mut self, asks: &[rpc::Ask<'_>]) -> Option<Result<rpc::Batched, Trouble>> {
        self.said.keep(None);
        if let Some(t) = asks.iter().find_map(|(m, _)| self.refused(m)) {
            return Some(Err(t));
        }
        let behind = crate::read::passed_over(self.node.written());
        let Some(behind) = behind.filter(|_| asks.iter().any(|(m, _)| *m != seam::M_BLOCK_NUMBER)) else { return self.ep.call_batch(asks) };
        let sent: Vec<usize> = (0..asks.len()).filter(|i| asks[*i].0 == seam::M_BLOCK_NUMBER).collect();
        let mut out: rpc::Batched = asks.iter().map(|_| Some(Err(behind.clone()))).collect();
        if sent.is_empty() {
            return Some(Ok(out));
        }
        let part: Vec<rpc::Ask<'_>> = sent.iter().map(|i| (asks[*i].0, asks[*i].1.clone())).collect();
        let got = match self.ep.call_batch(&part)? {
            Ok(g) => g,
            Err(t) => return Some(Err(t)),
        };
        for (i, a) in sent.into_iter().zip(got) {
            out[i] = a;
        }
        Some(Ok(out))
    }
    fn another(&self) -> Option<Box<dyn Endpoint + Send>> {
        Some(Box::new(Node::new(&self.t, self.node.clone())))
    }
    fn waits_out_limits(&self, method: &str) -> bool {
        self.ep.waits_out_limits(method)
    }
    fn asks_a_node(&self) -> bool {
        self.ep.asks_a_node()
    }
    fn name(&self) -> String {
        self.node.said()
    }
    fn place(&self) -> String {
        self.node.place()
    }
}

enum Way {
    Rpc,
    Rest(Box<dyn Endpoint + Send>),
    Broken(Fault),
}

struct Channel<T: Transport> {
    t: T,
    node: NodeAt,
    said: std::sync::Arc<Said>,
    way: Way,
}

impl<T: Transport> Channel<T> {
    fn open(t: T, node: NodeAt, said: std::sync::Arc<Said>) -> Channel<T> {
        let way = match crate::rest::of_lane(node.written()) {
            None => Way::Rpc,
            Some((base, _)) => match seam::parse_url(base) {
                Err(f) => Way::Broken(f),
                Ok(url) => {
                    let slot = said.clone();
                    let words = Box::new(move |f: eravon_net::Fail| {
                        slot.keep(Some(crate::transport::transport_fault(&url, &f)));
                        Trouble::Transport(crate::transport::broke_of(&url, &f))
                    });
                    match t.rest_node(node.written(), words) {
                        Some(ep) => Way::Rest(ep),
                        None => Way::Broken(Fault::Usage(format!("{node} 不是一行读得成的 REST 行"))),
                    }
                }
            },
        };
        Channel { t, node, said, way }
    }

    fn refused_as(&self, f: Fault, b: rpc::Broke) -> Result<W, Trouble> {
        self.said.keep(Some(f));
        Err(Trouble::Transport(b))
    }

    fn refused(&self, f: Fault) -> Result<W, Trouble> {
        let b = rpc::Broke::Said(f.detail());
        self.refused_as(f, b)
    }

    fn ask_node(&mut self, method: &str, params: &Value, within: Option<std::time::Duration>) -> Result<W, Trouble> {
        match &mut self.way {
            Way::Broken(f) => {
                let f = f.clone();
                return self.refused(f);
            }
            Way::Rest(ep) => {
                ASKED.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let got = match within {
                    Some(d) => ep.call_within(method, params, d),
                    None => ep.call(method, params),
                };
                if method == rpc::BROADCAST {
                    self.said.echo(got.as_ref().ok().cloned());
                }
                return got;
            }
            Way::Rpc => {}
        }
        let url = match self.node.url() {
            Ok(u) => u,
            Err(f) => return self.refused(f),
        };
        ASKED.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let id = next_id();
        let body = canon_bytes(&Value::Obj(vec![
            ("id".into(), Value::Int(id)),
            ("jsonrpc".into(), Value::Str(seam::JSONRPC.into())),
            ("method".into(), Value::Str(method.into())),
            ("params".into(), params.clone()),
        ]));
        let once = method == rpc::BROADCAST;
        let got = match within {
            Some(d) => self.t.post_within(&url, &body, once, d),
            None => self.t.post_way(&url, &body, once),
        };
        match got {
            Ok(a) => {
                let read = rpc::read_answer(self.node.written(), id, &a);
                if method == rpc::BROADCAST {
                    self.said.echo(read.as_ref().ok().cloned());
                }
                if let Err(t @ Trouble::Transport(_)) = &read {
                    if !(200..300).contains(&a.status) {
                        if let Fault::Endpoint(d) = crate::fault::of_trouble(&self.node, t.clone(), None) {
                            self.said.keep(Some(Fault::Endpoint(format!("{d}{BODY_SEP}{}", String::from_utf8_lossy(&a.body[..a.body.len().min(200)])))));
                        }
                    }
                }
                read
            }
            Err((f, b)) => self.refused_as(f, b),
        }
    }

    fn ask_node_asks(&mut self, asks_of: &[rpc::Ask<'_>]) -> Option<Result<rpc::Batched, Trouble>> {
        match &self.way {
            Way::Rest(_) => return None,
            Way::Broken(f) => return Some(self.refused(f.clone()).map(|_| Vec::new())),
            Way::Rpc => {}
        }
        let url = match self.node.url() {
            Ok(u) => u,
            Err(f) => return Some(self.refused(f).map(|_| Vec::new())),
        };
        ASKED.fetch_add(asks_of.len() as u64, std::sync::atomic::Ordering::SeqCst);
        let ids: Vec<u64> = asks_of.iter().map(|_| next_id()).collect();
        let asks: Vec<Value> = ids
            .iter()
            .zip(asks_of)
            .map(|(id, (method, p))| {
                Value::Obj(vec![
                    ("id".into(), Value::Int(*id)),
                    ("jsonrpc".into(), Value::Str(seam::JSONRPC.into())),
                    ("method".into(), Value::Str((*method).into())),
                    ("params".into(), p.clone()),
                ])
            })
            .collect();
        let body = canon_bytes(&Value::Arr(asks));
        let got = match batch_bound(asks_of.iter().map(|(m, _)| *m)) {
            Some(d) => self.t.post_within(&url, &body, false, d),
            None => self.t.post_way(&url, &body, false),
        };
        let a = match got {
            Ok(a) => a,
            Err((f, b)) => return Some(self.refused_as(f, b).map(|_| Vec::new())),
        };
        let written = self.node.written().to_string();
        let parsed = wire::parse(&a.body);
        if let Some(e) = parsed.as_ref().filter(|v| v.as_arr().is_none()).and_then(|v| v.member("error")).filter(|e| !e.is_null()) {
            return Some(Err(Trouble::Node(wire::write(e))));
        }
        if !(200..300).contains(&a.status) {
            return Some(Err(rpc::status(&written, a.status)));
        }
        let Some(rows) = parsed.as_ref().and_then(W::as_arr) else {
            return Some(Err(match parsed {
                Some(_) => rpc::shapeless(&written),
                None => Trouble::Transport(rpc::Broke::NotJson),
            }));
        };
        Some(Ok(ids
            .iter()
            .map(|id| {
                let row = rows.iter().find(|r| r.member("id").and_then(W::as_u64) == Some(*id))?;
                let one = eravon_net::Answer { status: a.status, location: None, body: wire::write(row).into_bytes() };
                Some(rpc::read_answer(&written, *id, &one))
            })
            .collect()))
    }
}

impl<T: Transport> Endpoint for Channel<T> {
    fn call(&mut self, method: &str, params: &Value) -> Result<W, Trouble> {
        let within = if matches!(self.way, Way::Rest(_)) { None } else { bound_of(method) };
        self.ask_node(method, params, within)
    }
    fn call_within(&mut self, method: &str, params: &Value, within: std::time::Duration) -> Result<W, Trouble> {
        self.ask_node(method, params, Some(within))
    }
    fn call_batch(&mut self, asks: &[rpc::Ask<'_>]) -> Option<Result<rpc::Batched, Trouble>> {
        self.ask_node_asks(asks)
    }
    fn another(&self) -> Option<Box<dyn Endpoint + Send>> {
        Some(Box::new(Channel::open(self.t.clone(), self.node.clone(), std::sync::Arc::new(Said::default()))))
    }
    fn waits_out_limits(&self, method: &str) -> bool {
        match &self.way {
            Way::Rest(ep) => ep.waits_out_limits(method),
            _ => false,
        }
    }
    fn name(&self) -> String {
        self.node.said()
    }
    fn place(&self) -> String {
        self.node.place()
    }
}

pub const ONE_ASK_METHODS: [&str; 16] = [
    "eth_chainId",
    "net_version",
    "eth_blockNumber",
    "eth_getBlockByNumber",
    "eth_getBlockByHash",
    "eth_getTransactionCount",
    "eth_getBalance",
    "eth_getCode",
    "eth_getStorageAt",
    "eth_call",
    "eth_estimateGas",
    "eth_gasPrice",
    "eth_maxPriorityFeePerGas",
    "eth_feeHistory",
    "eth_getTransactionByHash",
    "eth_getRawTransactionByHash",
];

pub const LOG_ASK: std::time::Duration = std::time::Duration::from_secs(15);

pub fn bound_of(method: &str) -> Option<std::time::Duration> {
    if ONE_ASK_METHODS.contains(&method) {
        Some(crate::send::ONE_ASK)
    } else if method == "eth_getLogs" {
        Some(LOG_ASK)
    } else {
        None
    }
}

pub fn batch_bound<'a>(methods: impl IntoIterator<Item = &'a str>) -> Option<std::time::Duration> {
    methods.into_iter().map(bound_of).try_fold(std::time::Duration::ZERO, |m, b| b.map(|b| m.max(b)))
}

pub fn classify(f: Fault) -> Fault {
    f
}


pub fn decided(node: &NodeAt, method: &str, params: &Value, w: &W) -> R<In> {
    crate::judge::read_as_decided(method, params, w).map(|v| in_of(&v)).ok_or_else(|| {
        let text = wire::write(w);
        let short: String = text.chars().take(200).collect();
        Fault::Endpoint(format!("malformed {node} 的答读不进值域:{short}"))
    })
}

pub fn ask_value<T: Transport>(t: &T, node: &NodeAt, method: &str, params: &Value) -> R<In> {
    let mut ep = Node::new(t, node.clone());
    let w = crate::patience::ask(&mut ep, method, params).map_err(|tr| ep.fault(tr))?;
    decided(node, method, params, &w)
}

pub(crate) fn ask_whole<T: Transport>(t: &T, node: &NodeAt, method: &str, params: &Value) -> R<In> {
    let mut ep = Node::new(t, node.clone());
    let w = crate::patience::ask(&mut ep, method, params).map_err(|tr| ep.fault(tr))?;
    wire::to_core(&w).map(|v| in_of(&v)).ok_or_else(|| {
        let short: String = wire::write(&w).chars().take(200).collect();
        Fault::Endpoint(format!("malformed {node} 的答读不进值域:{short}"))
    })
}

pub fn call_one<T: Transport>(t: &T, node: &NodeAt, method: &'static str, params: Vec<Out>) -> R<In> {
    ask_value(t, node, method, &params_of(&params))
}

pub fn call_one_raw<T: Transport>(t: &T, node: &NodeAt, method: &'static str, params_json: &str) -> R<In> {
    ask_value(t, node, method, &params_text(params_json)?)
}

pub(crate) fn call_one_raw_once<T: Transport>(t: &T, node: &NodeAt, method: &'static str, params_json: &str) -> R<In> {
    let params = params_text(params_json)?;
    let mut ep = Node::new(t, node.clone());
    let w = Endpoint::call(&mut ep, method, &params).map_err(|tr| ep.fault(tr))?;
    decided(node, method, &params, &w)
}

pub fn call_one_once<T: Transport>(t: &T, node: &NodeAt, method: &'static str, params: Vec<Out>) -> R<In> {
    ask_value(t, node, method, &params_of(&params))
}

pub fn ask_batch_raw<T: Transport>(t: &T, node: &NodeAt, method: &'static str, params: &[String]) -> R<Vec<Option<In>>> {
    ask_batch_said(t, node, method, params).map_err(|(f, _)| f)
}

pub(crate) fn ask_batch_said<T: Transport>(t: &T, node: &NodeAt, method: &'static str, params: &[String]) -> Result<Vec<Option<In>>, (Fault, bool)> {
    let asks: Vec<Value> = params.iter().map(|p| params_text(p)).collect::<R<_>>().map_err(|f| (f, false))?;
    let mut ep = Node::new(t, node.clone());
    let refused = |why: &str| Fault::Endpoint(format!("{node} 不收批量问({method} {} 问):{why}", params.len()));
    let with_method: Vec<rpc::Ask<'_>> = asks.iter().map(|p| (method, p.clone())).collect();
    let rows = match ep.call_batch(&with_method) {
        None if crate::rest::of_lane(node.written()).is_some() => return Err((refused("REST 行一问一问地问"), false)),
        None => return Err((refused("这一家缺这一方法"), false)),
        Some(Err(tr)) => {
            let limited = crate::said::refusal(&tr) == Some(crate::said::Refusal::RateLimited);
            if let Some(code) = rpc::status_of(&tr) {
                return Err((refused(&format!("HTTP {code}")), limited));
            }
            let f = ep.fault(tr);
            if let Fault::Usage(_) = f {
                return Err((f, false));
            }
            return Err((refused(&f.detail()), limited));
        }
        Some(Ok(rows)) => rows,
    };
    Ok(rows
        .into_iter()
        .zip(&asks)
        .map(|(r, p)| match r {
            Some(Ok(w)) if !w.is_null() => decided(node, method, p, &w).ok(),
            _ => None,
        })
        .collect())
}

pub fn none_answered(failed: Vec<(NodeAt, Fault)>) -> Fault {
    let lead = failed.iter().find(|(_, f)| matches!(f, Fault::Execution(_))).or_else(|| failed.iter().find(|(_, f)| matches!(f, Fault::OverAsks(_)))).or(failed.first()).map(|(_, f)| f.clone());
    let Some(lead) = lead else { return Fault::Usage("端点表是空的".into()) };
    let each: Vec<String> = failed.iter().map(|(n, f)| format!("{n} {}:{}", f.name(), f.detail())).collect();
    let detail = format!("{} · 每一家都没答:{}", lead.detail(), each.join(";"));
    match lead {
        Fault::Transport(_) => Fault::Transport(detail),
        Fault::Endpoint(_) => Fault::Endpoint(detail),
        Fault::Execution(_) => Fault::Execution(detail),
        Fault::Disagree(_) => Fault::Disagree(detail),
        Fault::WrongChain(_) => Fault::WrongChain(detail),
        Fault::Usage(_) => Fault::Usage(detail),
        Fault::Unread(_) => Fault::Unread(detail),
        Fault::Echo(_) => Fault::Echo(detail),
        Fault::OverAsks(_) => Fault::OverAsks(detail),
    }
}

pub fn agree(a: &In, b: &In) -> bool {
    match (a, b) {
        (In::Obj(x), In::Obj(y)) => x.iter().all(|(k, v)| y.iter().find(|(k2, _)| k2 == k).map_or(true, |(_, w)| agree(v, w))),
        (In::Arr(x), In::Arr(y)) => x.len() == y.len() && x.iter().zip(y).all(|(v, w)| agree(v, w)),
        _ => fingerprint(a) == fingerprint(b),
    }
}

pub fn groups(answers: &[(seam::NodeAt, In)]) -> Vec<(&In, Vec<&seam::NodeAt>)> {
    let mut g: Vec<(&In, Vec<&seam::NodeAt>)> = Vec::new();
    for (e, v) in answers {
        match g.iter_mut().find(|(rep, _)| agree(rep, v)) {
            Some((_, m)) => m.push(e),
            None => g.push((v, vec![e])),
        }
    }
    let size = |v: &In| v.as_arr().map_or(0, <[In]>::len);
    g.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(size(b.0).cmp(&size(a.0))));
    g
}

pub fn standing(own: &[In], reference: &[In]) -> (usize, usize) {
    let lacks = reference.iter().filter(|x| !own.iter().any(|y| agree(x, y))).count();
    let extra = own.iter().filter(|y| !reference.iter().any(|x| agree(x, y))).count();
    (lacks, extra)
}

pub fn split(what: &str, answers: &[(seam::NodeAt, In)]) -> Option<String> {
    let groups = groups(answers);
    if groups.len() < 2 {
        return None;
    }
    let who = |m: &[&seam::NodeAt]| m.iter().map(|n| n.said()).collect::<Vec<_>>().join("、");
    if let Some(lists) = groups.iter().map(|(v, m)| v.as_arr().map(|a| (a, m.as_slice()))).collect::<Option<Vec<_>>>() {
        let counts: Vec<String> = lists.iter().map(|(l, m)| format!("{} {} 条", who(m), l.len())).collect();
        let (big, bm) = lists[0];
        let rel: Vec<String> = lists[1..]
            .iter()
            .map(|(l, m)| {
                let (lacks, extra) = standing(l, big);
                if l.is_empty() {
                    format!("{} 是空的", who(m))
                } else if extra == 0 {
                    format!("{} 是 {} 的子集,缺 {lacks} 条", who(m), who(bm))
                } else if lacks == 0 {
                    format!("{} 比 {} 多出 {extra} 条", who(m), who(bm))
                } else {
                    format!("{} 有 {extra} 条 {} 没有,缺 {lacks} 条 {} 有的", who(m), who(bm), who(bm))
                }
            })
            .collect();
        return Some(format!("{what} 各家说法不一:{};{}", counts.join("、"), rel.join(";")));
    }
    let short = |v: &In| {
        let f = fingerprint(v);
        if f.chars().count() > 80 { format!("{}…", f.chars().take(80).collect::<String>()) } else { f }
    };
    if groups.iter().any(|(v, _)| matches!(v, In::Null)) {
        let none: Vec<String> = groups.iter().filter(|(v, _)| matches!(v, In::Null)).flat_map(|(_, m)| m.iter().map(|n| n.said())).collect();
        let some: Vec<String> = groups.iter().filter(|(v, _)| !matches!(v, In::Null)).map(|(v, m)| format!("{} 有({})", who(m), short(v))).collect();
        return Some(format!("{what} 各家说法不一:{} 没有;{}", none.join("、"), some.join(";")));
    }
    let each: Vec<String> = groups.iter().map(|(v, m)| format!("{} 答 {}", who(m), short(v))).collect();
    Some(format!("{what} 各家说法不一:{}", each.join(";")))
}

pub fn common(a: &In, b: &In) -> In {
    match (a, b) {
        (In::Obj(x), In::Obj(y)) => In::Obj(
            x.iter()
                .filter_map(|(k, v)| y.iter().find(|(k2, _)| k2 == k).map(|(_, w)| (k.clone(), common(v, w))))
                .collect(),
        ),
        (In::Arr(x), In::Arr(y)) if x.len() == y.len() => In::Arr(x.iter().zip(y).map(|(v, w)| common(v, w)).collect()),
        _ => a.clone(),
    }
}

pub fn fingerprint(v: &In) -> String {
    let mut s = String::new();
    put(v, &mut s);
    s
}

fn put(v: &In, out: &mut String) {
    match v {
        In::Null => out.push_str("null"),
        In::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        In::Int(n) => out.push_str(&n.to_string()),
        In::Str(s) => {
            out.push('"');
            out.push_str(&s.replace('\\', "\\\\").replace('"', "\\\""));
            out.push('"');
        }
        In::Arr(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                put(x, out);
            }
            out.push(']');
        }
        In::Obj(m) => {
            let mut idx: Vec<usize> = (0..m.len()).collect();
            idx.sort_by(|a, b| m[*a].0.as_bytes().cmp(m[*b].0.as_bytes()));
            out.push('{');
            for (i, k) in idx.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                put(&In::Str(m[k].0.clone()), out);
                out.push(':');
                put(&m[k].1, out);
            }
            out.push('}');
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_is_the_most_endpoints_and_items_compare_as_agreement_does() {
        let log = |b: &str, extra: bool| {
            let mut m = vec![("blockNumber".to_string(), In::Str(b.into()))];
            if extra {
                m.push(("blockTimestamp".to_string(), In::Str("0x1".into())));
            }
            In::Obj(m)
        };
        let a = vec![log("0x1", false)];
        let b = vec![log("0x1", true)];
        assert_eq!(standing(&a, &b), (0, 0));
        assert_eq!(split("eth_getLogs", &[("http://a".into(), In::Arr(a)), ("http://b".into(), In::Arr(b))]), None);
        let two = In::Arr(vec![log("0x1", false)]);
        let more = In::Arr(vec![log("0x1", false), log("0x2", false)]);
        let answers = [(seam::NodeAt::from("http://a"), two.clone()), ("http://b".into(), two.clone()), ("http://c".into(), more.clone())];
        let g = groups(&answers);
        assert_eq!(g[0].1.iter().map(|n| n.said()).collect::<Vec<_>>(), vec!["http://a", "http://b"], "多数那一组为准");
        let reference = g[0].0.as_arr().unwrap().to_vec();
        assert_eq!(standing(two.as_arr().unwrap(), &reference), (0, 0));
        assert_eq!(standing(more.as_arr().unwrap(), &reference), (0, 1));
        assert!(split("eth_getLogs", &answers).unwrap().contains("http://c 比 http://a、http://b 多出 1 条"));
    }

    #[test]
    fn a_split_says_each_endpoints_count_and_who_lacks_what() {
        let l = |n: &[&str]| In::Arr(n.iter().map(|x| In::Str((*x).into())).collect());
        let a = |e: &str, v: In| (seam::NodeAt::from(format!("http://{}", e.to_lowercase())), v);
        assert_eq!(split("m", &[a("A", l(&["x"])), a("B", l(&["x"]))]), None);
        let three = split("eth_getLogs", &[a("A", l(&["x", "y", "z"])), a("B", l(&["x", "y", "z"])), a("C", l(&[]))]).unwrap();
        assert!(three.contains("http://a、http://b 3 条") && three.contains("http://c 0 条") && three.contains("http://c 是空的"), "{three}");
        let sub = split("eth_getLogs", &[a("A", l(&["x"])), a("B", l(&["x", "y"]))]).unwrap();
        assert!(sub.contains("http://a 是 http://b 的子集,缺 1 条"), "比的是各家,不只头一家:{sub}");
        let both = split("eth_getLogs", &[a("A", l(&["x", "w"])), a("B", l(&["x", "y", "z"]))]).unwrap();
        assert!(both.contains("http://a 有 1 条 http://b 没有,缺 2 条 http://b 有的"), "{both}");
        let rc = split("eth_getTransactionReceipt", &[a("A", In::Null), a("B", In::Obj(vec![("status".into(), In::Str("0x1".into()))]))]).unwrap();
        assert!(rc.contains("http://a 没有") && rc.contains("http://b 有"), "{rc}");
    }

    #[test]
    fn the_comparison_key_does_not_depend_on_member_order() {
        let a = eravon_literals::json::read(b"{\"a\":1,\"b\":[true,null]}").unwrap();
        let b = eravon_literals::json::read(b"{\"b\":[true,null],\"a\":1}").unwrap();
        assert_eq!(fingerprint(&a), fingerprint(&b));
    }

    #[derive(Clone)]
    struct Says(u16, &'static str, std::sync::Arc<std::sync::Mutex<Vec<String>>>);

    impl Transport for Says {
        fn post(&self, _url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let b = String::from_utf8_lossy(body).to_string();
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            self.2.lock().unwrap().push(b);
            Ok(crate::transport::Answer { status: self.0, location: None, body: self.1.replace("ID", &id).into_bytes() })
        }
    }

    fn says(status: u16, body: &'static str) -> Says {
        Says(status, body, Default::default())
    }

    fn once(t: &Says) -> R<In> {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        static NEXT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(41_000);
        let at = format!("http://127.0.0.1:{}", NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst));
        call_one(t, &at.as_str().into(), seam::M_CALL, vec![])
    }

    #[test]
    fn a_request_is_canonical_bytes_with_an_id_of_its_own() {
        let t = says(200, r#"{"jsonrpc":"2.0","id":ID,"result":"0x1"}"#);
        call_one(&t, &"http://127.0.0.1:1".into(), seam::M_CHAIN_ID, vec![]).expect("answered");
        call_one(&t, &"http://127.0.0.1:1".into(), seam::M_CHAIN_ID, vec![]).expect("answered");
        let seen = t.2.lock().unwrap().clone();
        assert!(seen[0].starts_with("{\"id\":") && seen[0].ends_with(",\"jsonrpc\":\"2.0\",\"method\":\"eth_chainId\",\"params\":[]}"), "{}", seen[0]);
        assert_ne!(seen[0], seen[1], "每一问的 id 各自一枚");
    }

    #[test]
    fn the_three_classes_come_out_by_their_own_names() {
        let ep = once(&says(200, r#"{"jsonrpc":"2.0","id":ID,"error":{"code":-32601,"message":"Method not found"}}"#));
        assert_eq!(ep.unwrap_err().name(), "E_CHAIN_ENDPOINT");
        let ex = once(&says(200, r#"{"jsonrpc":"2.0","id":ID,"error":{"code":3,"message":"execution reverted"}}"#));
        assert_eq!(ex.unwrap_err().name(), "E_CHAIN_EXECUTION");
        let ex2 = once(&says(200, r#"{"jsonrpc":"2.0","id":ID,"error":{"code":-32000,"message":"insufficient funds"}}"#));
        assert_eq!(ex2.unwrap_err().name(), "E_CHAIN_EXECUTION");
    }

    #[test]
    fn only_a_reported_revert_counts_as_the_contract_saying_no() {
        for (body, want) in [
            (r#"{"jsonrpc":"2.0","id":ID,"error":{"code":3,"message":"execution reverted"}}"#, true),
            (r#"{"jsonrpc":"2.0","id":ID,"error":{"code":-32000,"message":"execution reverted"}}"#, true),
            (r#"{"jsonrpc":"2.0","id":ID,"error":{"code":-32015,"message":"VM execution error: Reverted"}}"#, true),
            (r#"{"jsonrpc":"2.0","id":ID,"error":{"code":-32005,"message":"limit exceeded"}}"#, false),
            (r#"{"jsonrpc":"2.0","id":ID,"error":{"code":-32000,"message":"header not found"}}"#, false),
            (r#"{"jsonrpc":"2.0","id":ID,"error":{"code":-32000,"message":"missing trie node"}}"#, false),
            (r#"{"jsonrpc":"2.0","id":ID,"error":{"code":-32002,"message":"resource unavailable"}}"#, false),
        ] {
            let f = once(&says(200, body)).unwrap_err();
            assert_eq!(crate::fault::reverted(&f), want, "{body}: {f:?}");
            assert_eq!(crate::fault::reverted_word(f.name(), &f.detail()), want);
        }
        assert!(!crate::fault::reverted(&Fault::Transport("revert".into())));
    }

    #[test]
    fn the_nodes_word_is_read_whatever_the_status() {
        let limited = r#"{"jsonrpc":"2.0","id":ID,"error":{"code":-32005,"message":"rate limit exceeded"}}"#;
        for s in [200, 429, 500] {
            let f = once(&says(s, limited)).unwrap_err();
            assert_eq!((f.name(), crate::fault::refusal(&f)), ("E_CHAIN_ENDPOINT", Some(crate::said::Refusal::RateLimited)), "{s}");
            assert!(!f.detail().contains("HTTP"), "按节点的话归,不按状态码:{f:?}");
        }
        let page = once(&says(429, "<html>slow down</html>")).unwrap_err();
        assert_eq!((crate::fault::refusal(&page), crate::fault::status_of(&page)), (Some(crate::said::Refusal::RateLimited), Some(429)));
        assert!(page.detail().ends_with(&format!("{BODY_SEP}<html>slow down</html>")), "the start of what came with it is kept: {page:?}");
        let gone = once(&says(502, "{}")).unwrap_err();
        assert_eq!(crate::fault::status_of(&gone), Some(502));
        for s in [200, 201, 204, 299, 199, 300] {
            assert_eq!(once(&says(s, r#"{"jsonrpc":"2.0","id":ID,"result":"0x1"}"#)), Ok(In::Str("0x1".into())), "{s}");
        }
        for s in [199, 300] {
            let f = once(&says(s, "<html>moved</html>")).expect_err("no node word");
            assert_eq!(crate::fault::status_of(&f), Some(s), "{s}: {f:?}");
        }
        let not_json = once(&says(200, "<html>ok</html>")).unwrap_err();
        assert!(not_json.detail().starts_with("malformed "), "{not_json:?}");
        let shapeless = once(&says(200, r#"{"jsonrpc":"2.0","id":ID}"#)).unwrap_err();
        assert!(shapeless.detail().starts_with("malformed "), "{shapeless:?}");
    }

    #[test]
    fn an_answer_to_someone_elses_question_is_refused() {
        for body in [r#"{"jsonrpc":"2.0","id":999999999,"result":"0x1"}"#, r#"{"jsonrpc":"2.0","result":"0x1"}"#, r#"[{"id":ID,"result":"0x1"}]"#] {
            let f = once(&says(200, body)).unwrap_err();
            assert!(f.detail().starts_with("malformed "), "{body}: {f:?}");
        }
        for body in [r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":"invalid request"}}"#, r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"invalid request"}}"#] {
            let f = once(&says(200, body)).unwrap_err();
            assert!(crate::fault::refusal(&f).is_some(), "空 id 下的 error 仍是节点的拒:{body}: {f:?}");
        }
        for body in [r#"{"jsonrpc":"2.0","id":null,"result":"0x1"}"#, r#"{"jsonrpc":"2.0","id":ID,"result":"0x1","error":{"code":1,"message":"x"}}"#] {
            let f = once(&says(200, body)).unwrap_err();
            assert!(f.detail().starts_with("malformed "), "{body}: {f:?}");
        }
    }

    #[test]
    fn no_endpoint_answering_names_each_and_the_chains_word_leads() {
        let down = Fault::Transport("connect http://127.0.0.1:1: refused".into());
        let f = none_answered(vec![("http://a".into(), down.clone()), ("http://b".into(), down.clone())]);
        assert!(f.name() == "E_CHAIN_TRANSPORT" && f.detail().starts_with("connect ") && f.detail().contains("每一家都没答") && f.detail().contains("http://b"), "{f:?}");
        let rev = crate::fault::node_refusal(r#"{"code":3,"message":"execution reverted"}"#);
        let f = none_answered(vec![("http://a".into(), Fault::Transport("late x".into())), ("http://b".into(), rev)]);
        assert!(crate::fault::reverted(&f), "链上说不的那一家领头:{f:?}");
    }

    #[test]
    fn a_result_is_cut_to_its_named_facts_before_it_is_taken_in() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let t = says(200, r#"{"jsonrpc":"2.0","id":ID,"result":{"oldestBlock":"0x1","gasUsedRatio":[0.5,0.25],"reward":[["0x64"]]}}"#);
        let r = call_one(&t, &"http://127.0.0.1:1".into(), seam::M_FEE_HISTORY, vec![Out::S("0x1".into()), Out::S("0x5".into()), Out::A(vec![Out::N(50)])]).expect("投影之后读得成");
        assert_eq!(r.get("reward").and_then(|x| x.as_arr()).map(<[In]>::len), Some(1));
        assert!(r.get("gasUsedRatio").is_none());
        assert!(call_one(&t, &"http://127.0.0.1:1".into(), seam::M_CALL, vec![]).is_err(), "整份读:小数读不进值域");
    }

    #[test]
    fn a_transport_refusal_leaves_as_it_was_given() {
        #[derive(Clone)]
        struct Down;
        impl Transport for Down {
            fn post(&self, url: &seam::Url, _body: &[u8]) -> R<crate::transport::Answer> {
                Err(Fault::Transport(format!("connect {}: refused", url.authority())))
            }
        }
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let f = call_one(&Down, &"http://127.0.0.1:1/k3y".into(), seam::M_CALL, vec![]).unwrap_err();
        assert_eq!(f, Fault::Transport("connect 127.0.0.1:1: refused".into()));
        let f = call_one(&Down, &"ftp://x".into(), seam::M_CALL, vec![]).unwrap_err();
        assert_eq!(f.name(), "E_CHAIN_USAGE", "{f:?}");
    }

    #[test]
    fn each_class_is_asked_as_many_times_as_its_row_says() {
        use crate::patience::{ask, class_of, Class, RATE_LIMITED, SERVER_ERROR, TABLE};
        static PLACES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        struct Same(Trouble, usize, usize, String);
        impl Endpoint for Same {
            fn call(&mut self, _method: &str, _params: &Value) -> Result<W, Trouble> {
                self.1 += 1;
                if self.2 > 0 && self.1 >= self.2 {
                    return Ok(W::of(crate::wire::Body::Num("7".into())));
                }
                Err(self.0.clone())
            }
            fn name(&self) -> String {
                "same".into()
            }
            fn place(&self) -> String {
                self.3.clone()
            }
        }
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let place = || format!("same-{}", PLACES.fetch_add(1, std::sync::atomic::Ordering::SeqCst));
        let count = |t: Trouble, method: &str| {
            let mut ep = Same(t, 0, 0, place());
            let _ = ask(&mut ep, method, &Value::Arr(Vec::new()));
            ep.1
        };
        let rate = Trouble::Node(r#"{"code":-32005,"message":"rate limit exceeded"}"#.into());
        assert_eq!((class_of(&rate), count(rate.clone(), "eth_call")), (Class::RateLimited, 3));
        assert_eq!(count(rpc::status("https://n.example", 429), "eth_call"), 3);
        let five = rpc::status("https://n.example", 503);
        assert_eq!((class_of(&five), count(five, "eth_call")), (Class::ServerError, 2));
        let late = rpc::late("https://n.example", std::time::Duration::from_secs(30));
        assert_eq!((class_of(&late), count(late, "eth_call")), (Class::Timeout, 1));
        let range = Trouble::Node(r#"{"code":-32005,"message":"block range limit exceeded"}"#.into());
        assert_eq!((class_of(&range), count(range, "eth_call")), (Class::Other, 1));
        assert_eq!(count(Trouble::Transport(rpc::Broke::Said("connect x: refused".into())), "eth_call"), 1);
        assert_eq!(count(rate, rpc::BROADCAST), 1, "广播只交一次");
        let mut ep = Same(rpc::status("https://n.example", 502), 0, 2, place());
        assert!(ask(&mut ep, "eth_call", &Value::Arr(Vec::new())).is_ok());
        assert_eq!(TABLE[0].1, &RATE_LIMITED[..], "限流那一行:300、900 毫秒");
        assert_eq!(RATE_LIMITED, [std::time::Duration::from_millis(300), std::time::Duration::from_millis(900)]);
        assert_eq!(SERVER_ERROR, [std::time::Duration::from_millis(300)]);
        let url = crate::seam::Url { scheme: "https".into(), host: "n.example".into(), port: 443, path: "/".into() };
        assert_eq!(class_of(&Trouble::Transport(crate::transport::broke_of(&url, &eravon_net::Fail::Late(std::time::Duration::from_secs(30))))), Class::Timeout);
    }

    #[test]
    fn the_scene_variable_sets_every_pause_and_nothing_else_does() {
        assert_eq!(scene_pause(Some("0")), Some(std::time::Duration::ZERO));
        assert_eq!(scene_pause(Some(" 25 ")), Some(std::time::Duration::from_millis(25)));
        for other in [None, Some("-1"), Some("fast"), Some(""), Some("1.5")] {
            assert_eq!(scene_pause(other), None, "{other:?}");
        }
        assert_eq!(ENV_PAUSE_MS, "ERAVON_PATIENCE_MS");
    }

    #[test]
    fn a_rest_row_goes_through_the_transports_rest_node() {
        struct Answers(&'static str);
        impl rpc::Endpoint for Answers {
            fn call(&mut self, method: &str, _params: &Value) -> Result<W, Trouble> {
                Ok(W::of(crate::wire::Body::Str(format!("{}:{method}", self.0))))
            }
            fn name(&self) -> String {
                "double".into()
            }
            fn place(&self) -> String {
                "double".into()
            }
        }
        #[derive(Clone)]
        struct Double(std::sync::Arc<std::sync::Mutex<Vec<String>>>, bool);
        impl Transport for Double {
            fn post(&self, _url: &seam::Url, _body: &[u8]) -> R<crate::transport::Answer> {
                Err(Fault::Usage("posted".into()))
            }
            fn rest_node(&self, lane: &str, _said: Box<dyn Fn(eravon_net::Fail) -> Trouble + Send>) -> Option<Box<dyn rpc::Endpoint + Send>> {
                self.0.lock().unwrap().push(lane.to_string());
                self.1.then(|| Box::new(Answers("rest")) as Box<dyn rpc::Endpoint + Send>)
            }
        }
        let lane = crate::rest::lane("https://api.etherscan.io/v2/api", 1);
        let d = Double(Default::default(), true);
        let mut n = Node::new(&d, NodeAt::new(lane.as_str()));
        assert_eq!(n.call(seam::M_BLOCK_NUMBER, &Value::Arr(Vec::new())).ok().and_then(|w| w.as_str().map(str::to_string)).as_deref(), Some("rest:eth_blockNumber"));
        assert!(n.call(rpc::BROADCAST, &Value::Arr(Vec::new())).is_ok() && n.echoed().is_some(), "a broadcast's echo is kept");
        assert_eq!(*d.0.lock().unwrap(), vec![lane.clone()], "the REST node opened once for the node, every question through it");
        let shut = Double(Default::default(), false);
        let mut n = Node::new(&shut, NodeAt::new(lane.as_str()));
        let t = n.call(seam::M_BLOCK_NUMBER, &Value::Arr(Vec::new())).expect_err("refused");
        assert_eq!(n.fault(t).name(), "E_CHAIN_USAGE");
        let f = ask_batch_raw(&d, &NodeAt::new(lane.as_str()), seam::M_GET_RAW_TX, &["[\"0x1\"]".to_string()]).expect_err("no batch");
        assert!(f.detail().contains("REST"), "{}", f.detail());
    }

    #[derive(Clone)]
    struct ByMethod {
        answers: fn(&str) -> Result<&'static str, &'static str>,
        asked: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl Transport for ByMethod {
        fn post(&self, _url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let v = crate::wire::parse(body).expect("a request");
            let one = |q: &W| -> String {
                let m = q.member("method").and_then(W::as_str).unwrap_or_default().to_string();
                let id = q.member("id").and_then(W::as_u64).unwrap_or(0);
                self.asked.lock().unwrap().push(m.clone());
                match (self.answers)(&m) {
                    Ok(r) => format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{r}}}"#),
                    Err(e) => format!(r#"{{"id":{id},"jsonrpc":"2.0","error":{e}}}"#),
                }
            };
            let body = match v.as_arr() {
                Some(qs) => format!("[{}]", qs.iter().map(one).collect::<Vec<_>>().join(",")),
                None => one(&v),
            };
            Ok(crate::transport::Answer { status: 200, location: None, body: body.into_bytes() })
        }
    }

    static LACKS: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn by_method(answers: fn(&str) -> Result<&'static str, &'static str>) -> &'static ByMethod {
        Box::leak(Box::new(ByMethod { answers, asked: Default::default() }))
    }

    #[test]
    fn a_batch_is_noted_as_single_answers_are() {
        let answers: fn(&str) -> Result<&'static str, &'static str> = |m| match m {
            "eth_getBlockByHash" => Ok(r#"{"hash":"0xaa","number":"0x1"}"#),
            _ => Err(r#"{"code":-32601,"message":"method not found"}"#),
        };
        let _turn = LACKS.lock().unwrap_or_else(|e| e.into_inner());
        let t = by_method(answers);
        let q = |h: &str| Value::Arr(vec![Value::Str(h.into()), Value::Bool(false)]);
        let _a = crate::action::open(None);
        let mut n = Node::new(t, NodeAt::new("http://127.0.0.1:30103"));
        let got = n.call_batch(&[("eth_getBlockByHash", q("0x01")), ("eth_getBlockByHash", q("0x02"))]).expect("a batch").expect("answered");
        assert!(got.iter().all(|a| matches!(a, Some(Ok(_)))));
        assert!(n.call("eth_getBlockByHash", &q("0x02")).is_ok());
        assert_eq!(t.asked.lock().unwrap().len(), 2, "the single ask is the action's answer");
        let _ = n.call("eth_getProof", &q("0x01"));
        let before = t.asked.lock().unwrap().len();
        let lacking = n.call_batch(&[("eth_getProof", q("0x01")), ("eth_getProof", q("0x02"))]).expect("the family's memory answers").expect("nothing refused whole");
        assert!(lacking.iter().all(Option::is_none) && t.asked.lock().unwrap().len() == before, "a lacking method is not batched, nothing sent");
        let lane = crate::rest::lane("https://api.etherscan.io/v2/api", 1);
        let mut rest = Node::new(t, NodeAt::new(lane.as_str()));
        assert!(rest.call_batch(&[("eth_getBlockByHash", q("0x01")), ("eth_getBlockByHash", q("0x02"))]).is_none(), "a REST row takes no batch");
    }

}

#[cfg(test)]
mod bound_tests {
    use super::*;
    use crate::transport::Answer;

    #[derive(Clone, Default)]
    struct Timed(std::sync::Arc<std::sync::Mutex<Vec<(String, Option<std::time::Duration>)>>>);

    impl Timed {
        fn answer(&self, body: &[u8], within: Option<std::time::Duration>) -> Result<Answer, (Fault, rpc::Broke)> {
            let v = eravon_literals::json::read(body).expect("a question");
            let method = v.get("method").and_then(In::as_str).unwrap_or_default().to_string();
            let id = match v.get("id") {
                Some(In::Int(n)) => *n,
                _ => 0,
            };
            self.0.lock().unwrap().push((method, within));
            Ok(Answer { status: 200, location: None, body: format!(r#"{{"jsonrpc":"2.0","id":{id},"result":"0x1"}}"#).into_bytes() })
        }
    }

    impl Transport for Timed {
        fn post(&self, _url: &seam::Url, _body: &[u8]) -> R<Answer> {
            unreachable!("asked through post_way or post_within")
        }
        fn post_way(&self, _url: &seam::Url, body: &[u8], _once: bool) -> Result<Answer, (Fault, rpc::Broke)> {
            self.answer(body, None)
        }
        fn post_within(&self, _url: &seam::Url, body: &[u8], _once: bool, within: std::time::Duration) -> Result<Answer, (Fault, rpc::Broke)> {
            self.answer(body, Some(within))
        }
    }

    #[test]
    fn a_point_read_is_given_one_bound_and_a_slow_kind_the_transports_deadline() {
        let t = Timed::default();
        let mut n = Node::new(&t, NodeAt::new("http://127.0.0.1:30301"));
        for m in ONE_ASK_METHODS {
            n.call(m, &Value::Arr(Vec::new())).expect("answered");
        }
        for m in ["eth_getLogs", "eth_getTransactionReceipt", "eth_getProof", "debug_traceTransaction", "eth_getBlockReceipts"] {
            n.call(m, &Value::Arr(Vec::new())).expect("answered");
        }
        let got = t.0.lock().unwrap().clone();
        for (m, within) in &got {
            let want = if ONE_ASK_METHODS.contains(&m.as_str()) {
                Some(crate::send::ONE_ASK)
            } else if m == "eth_getLogs" {
                Some(LOG_ASK)
            } else {
                None
            };
            assert_eq!(*within, want, "{m}");
        }
        assert_eq!(got.len(), ONE_ASK_METHODS.len() + 5);
        assert_eq!(batch_bound(["eth_chainId", "eth_blockNumber"]), Some(crate::send::ONE_ASK));
        assert_eq!(batch_bound(["eth_getCode", "eth_getLogs"]), Some(LOG_ASK));
        assert_eq!(batch_bound(["eth_getCode", "eth_getProof"]), None);
        assert_eq!(super::bound_of("eth_call"), Some(crate::send::ONE_ASK));
    }
}
