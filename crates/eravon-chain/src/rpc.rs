use crate::wire::{self, Body, W};
use eravon::json::{canon_bytes, Value};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Trouble {
    NotServed(String),
    Node(String),
    Transport(Broke),
    Contradiction(String),
}

#[derive(Clone, PartialEq, Eq)]
pub enum Broke {
    Late { node: String, deadline: std::time::Duration },
    Overlong { node: String, max: usize },
    Status { node: String, code: u16 },
    NotJson,
    Shapeless { node: String },
    Proxy { node: String, said: String },
    ProxyCredentials { node: String, given: bool, said: String },
    Closed { node: String },
    Said(String),
    Short,
    Unread { method: String },
    NoEndpoint,
    KeyRange,
    Unsigned,
    NoRegistry,
    EchoNotOurs,
    Unserved { node: String, method: String },
    NoKey { node: String },
    LogsBehind { node: String },
    HeadBehind { node: String, block: u64 },
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ProofGap {
    TxUnread(String),
    NotInBlock,
    HeaderUnbuilt,
    HeaderUnread,
    HeaderHash,
    NoTxList,
    BlockTxUnread(String),
    ReceiptUnbuilt,
    NotInItsBlock,
    TxRoot,
    ReceiptRoot,
    NoSender,
    ParentUnbuilt,
    NoAccountProof,
    AccountProof,
    AccountProofShape,
}

impl Broke {
    pub fn evidence(&self) -> String {
        match self {
            Broke::Late { node, deadline } => format!("TIMEOUT {node} {}ms", deadline.as_millis()),
            Broke::Overlong { node, max } => format!("OVERSIZE {node} {max}"),
            Broke::Status { node, code } => format!("HTTP_{code} {node}"),
            Broke::NotJson => "NOT_JSON".to_string(),
            Broke::Shapeless { node } => format!("NOT_AN_ANSWER {node}"),
            Broke::Proxy { node, said } => format!("PROXY {node}: {said}"),
            Broke::ProxyCredentials { node, given: false, said } => format!("{PROXY_ASKS} {node}: {said}"),
            Broke::ProxyCredentials { node, given: true, said } => format!("{PROXY_REFUSES} {node}: {said}"),
            Broke::Closed { node } => format!("CLOSED {node}"),
            Broke::Said(x) => x.clone(),
            Broke::Short => "SHORT".to_string(),
            Broke::Unread { method } => format!("UNREAD {method}"),
            Broke::NoEndpoint => "NO_ENDPOINT".to_string(),
            Broke::KeyRange => "KEY_OUT_OF_RANGE".to_string(),
            Broke::Unsigned => "UNSIGNED".to_string(),
            Broke::NoRegistry => "NO_REGISTRY".to_string(),
            Broke::EchoNotOurs => "ECHO_NOT_OURS".to_string(),
            Broke::Unserved { node, method } => format!("NOT_SERVED {node} {method}"),
            Broke::NoKey { node } => format!("NO_KEY_HERE {node}"),
            Broke::LogsBehind { node } => format!("LOGS_NOT_YET {node}"),
            Broke::HeadBehind { node, block } => format!("HEAD_NOT_YET {node} {block}"),
        }
    }
}

impl std::fmt::Debug for Broke {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.evidence())
    }
}

pub const PROXY_ASKS: &str = "PROXY_CREDENTIALS_MISSING";
pub const PROXY_REFUSES: &str = "PROXY_CREDENTIALS_REFUSED";

pub fn late(url: &str, deadline: std::time::Duration) -> Trouble {
    Trouble::Transport(Broke::Late { node: eravon_net::sayable(url), deadline })
}

pub fn overlong(url: &str, max: usize) -> Trouble {
    Trouble::Transport(Broke::Overlong { node: eravon_net::sayable(url), max })
}

pub fn proxy_leg(url: &str, said: &str) -> Trouble {
    Trouble::Transport(Broke::Proxy { node: eravon_net::sayable(url), said: said.to_string() })
}

pub fn proxy_credentials(url: &str, given: bool, said: &str) -> Trouble {
    Trouble::Transport(Broke::ProxyCredentials { node: eravon_net::sayable(url), given, said: said.to_string() })
}

pub fn proxy_credentials_of(said: &str) -> Option<bool> {
    if said.contains(PROXY_REFUSES) {
        Some(true)
    } else if said.contains(PROXY_ASKS) {
        Some(false)
    } else {
        None
    }
}

pub fn status(url: &str, code: u16) -> Trouble {
    Trouble::Transport(Broke::Status { node: eravon_net::sayable(url), code })
}

pub fn status_of(t: &Trouble) -> Option<u16> {
    match t {
        Trouble::Transport(Broke::Status { code, .. }) => Some(*code),
        _ => None,
    }
}

pub fn shapeless(url: &str) -> Trouble {
    Trouble::Transport(Broke::Shapeless { node: eravon_net::sayable(url) })
}

pub const BROADCAST: &str = "eth_sendRawTransaction";

fn node_word(v: &W, id: u64) -> Option<Result<W, Trouble>> {
    if !matches!(v.body, Body::Obj(_)) {
        return None;
    }
    let ours = v.member("id").and_then(|i| i.as_u64()) == Some(id);
    let unread = v.member("id").is_none_or(|i| i.is_null());
    let error = v.member("error").filter(|e| !e.is_null());
    match (v.member("result"), error) {
        (Some(r), None) if ours => Some(Ok(r.clone())),
        (None, Some(e)) if ours || unread => Some(Err(Trouble::Node(wire::write(e)))),
        _ => None,
    }
}

pub fn read_answer(url: &str, id: u64, got: &eravon_net::Answer) -> Result<W, Trouble> {
    let v = wire::parse(&got.body);
    if let Some(said) = v.as_ref().and_then(|v| node_word(v, id)) {
        return said;
    }
    match (got.status, v) {
        (s, _) if !(200..300).contains(&s) => Err(status(url, s)),
        (_, None) => Err(Trouble::Transport(Broke::NotJson)),
        (_, Some(_)) => Err(shapeless(url)),
    }
}

pub fn ask_node(
    url: &str,
    target: &eravon_net::Target,
    limits: &Limits,
    id: u64,
    method: &str,
    params: &Value,
    said: &dyn Fn(eravon_net::Fail) -> Trouble,
) -> Result<W, Trouble> {
    let body = canon_bytes(&Value::Obj(vec![
        ("id".into(), Value::Int(id)),
        ("jsonrpc".into(), Value::Str("2.0".into())),
        ("method".into(), Value::Str(method.into())),
        ("params".into(), params.clone()),
    ]));
    let how = if method == BROADCAST { eravon_net::Ask::Once } else { eravon_net::Ask::Read };
    let got = eravon_net::post_json(target, &body, limits, how).map_err(said)?;
    read_answer(url, id, &got)
}

pub type Batched = Vec<Option<Result<W, Trouble>>>;

pub type Ask<'a> = (&'a str, Value);

pub fn ask_batch(
    url: &str,
    target: &eravon_net::Target,
    limits: &Limits,
    first_id: u64,
    asks: &[Ask<'_>],
    said: &dyn Fn(eravon_net::Fail) -> Trouble,
) -> Result<Batched, Trouble> {
    let body: Vec<Value> = asks
        .iter()
        .enumerate()
        .map(|(i, (method, p))| {
            Value::Obj(vec![
                ("id".into(), Value::Int(first_id + i as u64)),
                ("jsonrpc".into(), Value::Str("2.0".into())),
                ("method".into(), Value::Str((*method).into())),
                ("params".into(), p.clone()),
            ])
        })
        .collect();
    let got = eravon_net::post_json(target, &canon_bytes(&Value::Arr(body)), limits, eravon_net::Ask::Read).map_err(said)?;
    let parsed = wire::parse(&got.body);
    if let Some(e) = parsed.as_ref().filter(|v| matches!(v.body, Body::Obj(_))).and_then(|v| v.member("error")).filter(|e| !e.is_null()) {
        return Err(Trouble::Node(wire::write(e)));
    }
    if !(200..300).contains(&got.status) {
        return Err(status(url, got.status));
    }
    let rows = match parsed {
        Some(W { body: Body::Arr(rows), .. }) => rows,
        Some(_) => return Err(shapeless(url)),
        None => return Err(Trouble::Transport(Broke::NotJson)),
    };
    Ok((0..asks.len()).map(|i| rows.iter().find_map(|r| node_word(r, first_id + i as u64).filter(|_| r.member("id").and_then(|x| x.as_u64()) == Some(first_id + i as u64)))).collect())
}

pub fn within(limits: Limits, within: std::time::Duration) -> Limits {
    let deadline = match (limits.deadline.is_zero(), within.is_zero()) {
        (_, true) => limits.deadline,
        (true, false) => within,
        (false, false) => limits.deadline.min(within),
    };
    Limits { deadline, ..limits }
}

pub trait Endpoint {
    fn call(&mut self, method: &str, params: &Value) -> Result<W, Trouble>;
    fn call_within(&mut self, method: &str, params: &Value, within: std::time::Duration) -> Result<W, Trouble> {
        let _ = within;
        self.call(method, params)
    }
    fn waits_out_limits(&self, method: &str) -> bool {
        let _ = method;
        false
    }
    fn asks_a_node(&self) -> bool {
        true
    }
    fn call_batch(&mut self, asks: &[Ask<'_>]) -> Option<Result<Batched, Trouble>> {
        let _ = asks;
        None
    }
    fn another(&self) -> Option<Box<dyn Endpoint + Send>> {
        None
    }
    fn name(&self) -> String;
    fn place(&self) -> String;
}

pub use eravon_net::{env, Limits};

pub use eravon_net::{follow_the_terminal, proxy_of, read_address, set_choice, Choice, NotAProxy, NotAnAddress, SystemProxies};

pub use eravon_net::{script_trouble_last, PacTrouble};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NotAnEndpoint {
    NoEquals,
    ChainNotInt,
    NoAddress,
    InnerSpace,
}

pub fn endpoint_spec(spec: &str) -> Result<(u64, String), NotAnEndpoint> {
    let (c, address) = spec.split_once('=').ok_or(NotAnEndpoint::NoEquals)?;
    let chain: u64 = c.trim().parse().map_err(|_| NotAnEndpoint::ChainNotInt)?;
    let address = address.trim();
    if address.is_empty() {
        return Err(NotAnEndpoint::NoAddress);
    }
    if spec.trim().chars().any(char::is_whitespace) {
        return Err(NotAnEndpoint::InnerSpace);
    }
    Ok((chain, address.to_string()))
}

pub struct Http {
    url: String,
    target: eravon_net::Target,
    next_id: u64,
    limits: Limits,
}

impl Http {
    pub fn new(url: &str) -> Option<Http> {
        let target = eravon_net::parse(url)?;
        Some(Http { url: url.trim().to_string(), target, next_id: 1, limits: Limits::from_env() })
    }

    pub fn with_limits(mut self, limits: Limits) -> Http {
        self.limits = limits;
        self
    }
}

pub fn transport_said(url: &str, f: &eravon_net::Fail) -> Trouble {
    let url = &eravon_net::sayable(url);
    match f {
        eravon_net::Fail::Late(d) => late(url, *d),
        eravon_net::Fail::Overlong(max) => overlong(url, *max),
        eravon_net::Fail::Name(x)
        | eravon_net::Fail::Connect(x)
        | eravon_net::Fail::Handshake(x)
        | eravon_net::Fail::Certificate(x)
        | eravon_net::Fail::Stream(x) => Trouble::Transport(Broke::Said(format!("{url}: {x}"))),
        eravon_net::Fail::Proxy(x) => proxy_leg(url, x),
        eravon_net::Fail::ProxyCredentials { given, said } => proxy_credentials(url, *given, said),
        eravon_net::Fail::Closed => Trouble::Transport(Broke::Closed { node: url.clone() }),
    }
}

impl Http {
    fn ask(&mut self, method: &str, params: &Value, limits: Limits) -> Result<W, Trouble> {
        let id = self.next_id;
        self.next_id += 1;
        let url = self.url.clone();
        ask_node(&self.url, &self.target, &limits, id, method, params, &|f| transport_said(&url, &f))
    }
}

impl Endpoint for Http {
    fn call(&mut self, method: &str, params: &Value) -> Result<W, Trouble> {
        self.ask(method, params, self.limits)
    }
    fn call_batch(&mut self, asks: &[Ask<'_>]) -> Option<Result<Batched, Trouble>> {
        let first = self.next_id;
        self.next_id += asks.len() as u64;
        let url = self.url.clone();
        Some(ask_batch(&self.url, &self.target, &self.limits, first, asks, &|f| transport_said(&url, &f)))
    }
    fn another(&self) -> Option<Box<dyn Endpoint + Send>> {
        Some(Box::new(Http { url: self.url.clone(), target: self.target.clone(), next_id: 1, limits: self.limits }))
    }
    fn call_within(&mut self, method: &str, params: &Value, within: std::time::Duration) -> Result<W, Trouble> {
        self.ask(method, params, self::within(self.limits, within))
    }
    fn name(&self) -> String {
        eravon_net::sayable(&self.url)
    }
    fn place(&self) -> String {
        eravon_net::place_key(&self.url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_nodes_said_alike_are_two_places() {
        let eth = Http::new("https://node.example/eth/k1").expect("an address");
        let polygon = Http::new("https://node.example/polygon/k2").expect("an address");
        assert_eq!(eth.name(), polygon.name(), "said alike: scheme, host and port only");
        assert_ne!(eth.place(), polygon.place(), "two places");
        assert_eq!(Http::new("HTTPS://Node.Example:443/eth/k1").expect("an address").place(), eth.place(), "one place however it is written");
    }
}
