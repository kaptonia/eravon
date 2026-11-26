use crate::{Code, Trouble, Why};
use eravon::json::Value;
use eravon_chain::endpoints;
use eravon_chain::rpc;
use eravon_chain::wire;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeAddr(String);

impl NodeAddr {
    pub fn new(written: impl Into<String>) -> NodeAddr {
        NodeAddr(written.into())
    }

    pub fn said(&self) -> String {
        eravon_net::sayable(&self.0)
    }

    pub fn place(&self) -> String {
        endpoints::place_of(&self.0)
    }

    pub fn rest(base: &str, chain: u64) -> NodeAddr {
        NodeAddr(eravon_chain::rest::lane(base, chain))
    }

    pub fn kind(&self) -> Kind {
        if eravon_chain::rest::of_lane(&self.0).is_some() { Kind::Rest } else { Kind::Rpc }
    }

    pub fn for_transport(&self) -> &str {
        &self.0
    }
}

impl From<String> for NodeAddr {
    fn from(s: String) -> NodeAddr {
        NodeAddr(s)
    }
}

impl From<&str> for NodeAddr {
    fn from(s: &str) -> NodeAddr {
        NodeAddr(s.to_string())
    }
}

impl std::fmt::Display for NodeAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.said())
    }
}

impl std::fmt::Debug for NodeAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NodeAddr").field(&self.said()).finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Rpc,
    Rest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoint {
    pub chain: u64,
    pub url: NodeAddr,
}

impl Endpoint {
    pub fn parse(spec: &str) -> Option<Endpoint> {
        let (chain, url) = eravon_chain::rpc::endpoint_spec(spec).ok()?;
        Some(Endpoint { chain, url: NodeAddr(url) })
    }

    pub fn at(chain: u64, url: impl Into<NodeAddr>) -> Endpoint {
        Endpoint { chain, url: url.into() }
    }

    pub fn typed(spec: &str) -> Result<Endpoint, Why> {
        let e = Endpoint::parse(spec).ok_or_else(|| Why::NotSpec { node: eravon_net::sayable(spec) })?;
        eravon_chain::rpc::read_address(e.url.for_transport()).map_err(|_| address_said(e.url.for_transport()))?;
        Ok(e)
    }

    pub fn spec(&self) -> String {
        format!("{}={}", self.chain, self.url.for_transport())
    }
}

pub fn nodes_of(g: &eravon_seal::ground::Ground, chain: u64) -> Vec<Endpoint> {
    let mut out: Vec<Endpoint> = g.endpoints.iter().filter_map(|spec| Endpoint::parse(spec)).filter(|e| e.chain == chain).collect();
    out.extend(g.rest_asked().iter().map(|b| Endpoint { chain, url: NodeAddr::rest(b, chain) }));
    out
}

pub fn settings_nodes(g: &eravon_seal::ground::Ground) -> Vec<Endpoint> {
    eravon_chain::action::take_asks_cap(g.log_asks.unwrap_or(eravon_chain::action::ASKS_DEFAULT));
    let mut out: Vec<Endpoint> = g.endpoints.iter().filter_map(|spec| Endpoint::parse(spec)).collect();
    if let Some(chain) = g.chain_id {
        out.extend(g.rest_asked().iter().map(|b| Endpoint { chain, url: NodeAddr::rest(b, chain) }));
    }
    out
}

#[derive(Clone, Debug)]
pub struct Reading {
    pub value: Value,
    pub sources: usize,
    pub single_source: bool,
    pub unanswered: Vec<Why>,
    pub differed: Vec<Why>,
}

pub fn sayable_all(urls: &[String]) -> String {
    urls.iter().map(|u| eravon_net::sayable(u)).collect::<Vec<_>>().join(" ")
}

pub fn node_why(url: &str, t: &rpc::Trouble) -> Why {
    Why::Node { node: eravon_net::sayable(url), trouble: t.clone() }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Said {
    Node(Refusal),
    Wire(Wire),
}

pub use eravon_chain::said::{refusal_of, Refusal};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Wire {
    Tls(Layer),
    Timeout,
    Oversize,
    NotJson,
    Shape,
    Proxy,
    ProxyCredentials { given: bool },
    Other(String),
}

pub fn said(t: &rpc::Trouble) -> Said {
    if let Some(r) = eravon_chain::said::refusal(t) {
        return Said::Node(r);
    }
    match t {
        rpc::Trouble::Node(e) => Said::Node(refusal_of(e)),
        rpc::Trouble::Transport(b) => Said::Wire(match b {
            rpc::Broke::Late { .. } => Wire::Timeout,
            rpc::Broke::Overlong { .. } => Wire::Oversize,
            rpc::Broke::NotJson => Wire::NotJson,
            rpc::Broke::Shapeless { .. } => Wire::Shape,
            rpc::Broke::ProxyCredentials { given, .. } => Wire::ProxyCredentials { given: *given },
            rpc::Broke::Proxy { .. } => Wire::Proxy,
            rpc::Broke::Said(e) => match Layer::of(e) {
                Some(l) => Wire::Tls(l),
                None => Wire::Other(e.clone()),
            },
            other => Wire::Other(other.evidence()),
        }),
        rpc::Trouble::NotServed(k) | rpc::Trouble::Contradiction(k) => Said::Wire(Wire::Other(k.clone())),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    Retry,
    NextEndpoint,
    Stop,
}

pub fn already_taken(t: &Trouble) -> bool {
    matches!(t.code, Code::AlreadyPending | Code::NonceUsed)
}

pub fn tx_known(urls: &[NodeAddr], chain: u64, hash: &[u8; 32]) -> bool {
    let _ = chain;
    let q = Value::Arr(vec![Value::Str(eravon::hexfmt::encode(hash))]);
    each_node(urls, &endpoint_at, move |_, ep| ep.and_then(|ep| ep.call_within("eth_getTransactionByHash", &q, ONE_ASK).ok()).map(|w| !w.is_null()), |view| view.iter().any(|k| k.is_some_and(|k| *k == Some(true))))
        .into_iter()
        .flatten()
        .any(|known| known == Some(true))
}

pub use eravon_chain::send::ONE_ASK;

pub use eravon_chain::oneread::one_ask;

pub fn next_after(t: &Trouble) -> Next {
    match t.code {
        Code::RateLimited => Next::Retry,
        c if Code::NETWORK.contains(&c) => Next::NextEndpoint,
        _ => Next::Stop,
    }
}

pub fn address_said(url: &str) -> Why {
    let node = eravon_net::sayable(url);
    if let Some((base, _)) = eravon_chain::rest::of_lane(url) {
        return match eravon_chain::rest::base_of(base) {
            Ok(_) => Why::Text(String::new()),
            Err(eravon_chain::rest::NotABase::Scheme) => Why::Scheme { node },
            Err(eravon_chain::rest::NotABase::Address(eravon_net::NotAnAddress::Port)) => Why::Port { node },
            Err(_) => Why::NotAddress { node },
        };
    }
    match eravon_net::read_address(url) {
        Ok(_) => Why::Text(String::new()),
        Err(eravon_net::NotAnAddress::Scheme) => Why::Scheme { node },
        Err(eravon_net::NotAnAddress::Port) => Why::Port { node },
        Err(_) => Why::NotAddress { node },
    }
}

fn no_address_reads(said: Vec<Why>) -> Trouble {
    Trouble::new(Code::SettingsShape, Why::Each(said))
}

pub fn said_trouble(url: &str, t: &rpc::Trouble) -> Trouble {
    let code = match said(t) {
        _ if matches!(t, rpc::Trouble::Transport(rpc::Broke::NoKey { .. })) => Code::ServiceKeyMissing,
        _ if eravon_chain::rest::unserved(t) => Code::ServiceUnserved,
        Said::Node(r) => match r {
            Refusal::Funds => Code::InsufficientFunds,
            Refusal::NonceUsed => Code::NonceUsed,
            Refusal::Pending => Code::AlreadyPending,
            Refusal::Underpriced => Code::Underpriced,
            Refusal::GasTooLow => Code::GasTooLow,
            Refusal::Reverted => Code::ContractRefused,
            Refusal::RateLimited => Code::RateLimited,
            Refusal::NoMethod => Code::MethodMissing,
            Refusal::Auth => Code::NodeAuth,
            Refusal::WrongChain => Code::WrongChain,
            Refusal::Coded(_) | Refusal::Other(_) => Code::NodeRefused,
        },
        Said::Wire(w) => match w {
            Wire::Tls(_) => Code::NodeTls,
            Wire::Timeout => Code::NodeTimeout,
            Wire::Oversize => Code::AnswerTooLong,
            Wire::NotJson => Code::AnswerNotJson,
            Wire::Shape => Code::ChainShape,
            Wire::Proxy => Code::ProxyUnreachable,
            Wire::ProxyCredentials { given: false } => Code::ProxyCredentialsMissing,
            Wire::ProxyCredentials { given: true } => Code::ProxyCredentialsRefused,
            Wire::Other(_) => Code::Unreachable,
        },
    };
    let credentials = matches!(code, Code::ProxyCredentialsMissing | Code::ProxyCredentialsRefused);
    Trouble { code, why: node_why(url, t), script: eravon_net::script_trouble_for(url).filter(|_| !credentials) }
}

pub fn endpoint_at(url: &str) -> Option<Box<dyn rpc::Endpoint + Send>> {
    opened_at(url).map(|ep| endpoints::remembering(url, ep))
}

fn opened_at(url: &str) -> Option<Box<dyn rpc::Endpoint + Send>> {
    if let Some((base, _)) = eravon_chain::rest::of_lane(url) {
        let layers = Https::new(base)?;
        return eravon_chain::rest::Rest::open(url, move |f| layers.said(f)).map(|r| Box::new(r) as Box<dyn rpc::Endpoint + Send>);
    }
    match eravon_net::parse(url)?.scheme {
        eravon_net::Scheme::Http => rpc::Http::new(url).map(|h| Box::new(h) as Box<dyn rpc::Endpoint + Send>),
        eravon_net::Scheme::Https => Https::new(url).map(|h| Box::new(h) as Box<dyn rpc::Endpoint + Send>),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layer {
    Name,
    Connect,
    Handshake,
    Certificate,
}

impl Layer {
    pub const ALL: [Layer; 4] = [Layer::Name, Layer::Connect, Layer::Handshake, Layer::Certificate];

    pub fn code(self) -> &'static str {
        match self {
            Layer::Name => "TLS_NAME",
            Layer::Connect => "TLS_CONNECT",
            Layer::Handshake => "TLS_HANDSHAKE",
            Layer::Certificate => "TLS_CERT",
        }
    }

    pub fn of(said: &str) -> Option<Layer> {
        Layer::ALL.into_iter().find(|l| said.starts_with(l.code()))
    }
}

static LAYER_WORDS: std::sync::OnceLock<fn(Layer, &str, &str) -> String> = std::sync::OnceLock::new();

pub fn say_layers(words: fn(Layer, &str, &str) -> String) {
    let _ = LAYER_WORDS.set(words);
}

fn layer_words(l: Layer, node: &str, detail: &str) -> String {
    match LAYER_WORDS.get() {
        Some(w) => w(l, node, detail),
        None => format!("{node}: {detail}"),
    }
}

pub struct Https {
    url: String,
    target: eravon_net::Target,
    next_id: u64,
    limits: rpc::Limits,
}

impl Https {
    pub fn new(url: &str) -> Option<Https> {
        let target = eravon_net::parse(url).filter(|t| t.scheme == eravon_net::Scheme::Https)?;
        Some(Https { url: url.trim().to_string(), target, next_id: 1, limits: rpc::Limits::from_env() })
    }

    fn fail(&self, layer: Layer, detail: &str) -> rpc::Trouble {
        rpc::Trouble::Transport(rpc::Broke::Said(format!("{} · {}", layer.code(), layer_words(layer, &eravon_net::sayable(&self.url), detail))))
    }

    fn said(&self, f: eravon_net::Fail) -> rpc::Trouble {
        match f {
            eravon_net::Fail::Name(x) => self.fail(Layer::Name, &x),
            eravon_net::Fail::Connect(x) => self.fail(Layer::Connect, &x),
            eravon_net::Fail::Handshake(x) => self.fail(Layer::Handshake, &x),
            eravon_net::Fail::Certificate(x) => self.fail(Layer::Certificate, &x),
            other => rpc::transport_said(&self.url, &other),
        }
    }

    pub fn get(&self, limits: &rpc::Limits) -> Result<Got, rpc::Trouble> {
        let a = eravon_net::get(&self.target, limits).map_err(|f| self.said(f))?;
        Ok(Got { status: a.status, location: a.location, body: a.body })
    }

    pub fn address(&self) -> String {
        self.target.url()
    }

    pub fn parts(&self) -> (&str, u16, &str) {
        (&self.target.host, self.target.port, &self.target.path)
    }
}

pub struct Got {
    pub status: u16,
    pub location: Option<String>,
    pub body: Vec<u8>,
}

impl Https {
    fn ask(&mut self, method: &str, params: &Value, limits: rpc::Limits) -> Result<wire::W, rpc::Trouble> {
        let id = self.next_id;
        self.next_id += 1;
        rpc::ask_node(&self.url, &self.target, &limits, id, method, params, &|f| self.said(f))
    }
}

impl rpc::Endpoint for Https {
    fn call(&mut self, method: &str, params: &Value) -> Result<wire::W, rpc::Trouble> {
        self.ask(method, params, self.limits)
    }
    fn call_batch(&mut self, asks: &[rpc::Ask<'_>]) -> Option<Result<rpc::Batched, rpc::Trouble>> {
        let first = self.next_id;
        self.next_id += asks.len() as u64;
        Some(rpc::ask_batch(&self.url, &self.target, &self.limits, first, asks, &|f| self.said(f)))
    }
    fn another(&self) -> Option<Box<dyn rpc::Endpoint + Send>> {
        Some(Box::new(Https { url: self.url.clone(), target: self.target.clone(), next_id: 1, limits: self.limits }))
    }
    fn call_within(&mut self, method: &str, params: &Value, within: std::time::Duration) -> Result<wire::W, rpc::Trouble> {
        self.ask(method, params, rpc::within(self.limits, within))
    }
    fn name(&self) -> String {
        eravon_net::sayable(&self.url)
    }
    fn place(&self) -> String {
        eravon_net::place_key(&self.url)
    }
}

fn none_answered(first: Option<Trouble>, shapeless: bool, refused: Vec<Why>) -> Trouble {
    match first {
        Some(t) => t.retold(Why::Each(refused)),
        None => Trouble::new(if shapeless { Code::ChainShape } else { Code::Unreachable }, Why::Each(refused)),
    }
}

fn shapeless(m: &eravon_chain::judge::Missing, call: bool) -> bool {
    matches!(m, eravon_chain::judge::Missing::Unreadable(text) if call || text != "null")
}

pub const SEND_BACKOFF: [std::time::Duration; 2] = eravon_chain::said::RATE_BACKOFF;

pub fn route_of(proxy: Option<&str>) -> eravon_net::Choice {
    use eravon_seal::machine::proxy::{NONE, SYSTEM};
    match proxy.map(str::trim) {
        None | Some(SYSTEM) => eravon_net::Choice::System,
        Some(NONE) => eravon_net::Choice::Off,
        Some(p) => eravon_net::proxy_of(p).map(eravon_net::Choice::Manual).unwrap_or(eravon_net::Choice::System),
    }
}

pub fn take_machine_route(proxy: Option<&str>) {
    eravon_net::follow_the_system();
    eravon_net::set_choice(route_of(proxy));
}

pub use eravon_chain::oneread::{one_read, OneRead};
use eravon_chain::oneread::{pass_over, passed_over, passes_over};

pub fn ask(eps: &[Endpoint], method: &str, params: &Value) -> Result<Reading, Trouble> {
    let _read = OneRead::open();
    ask_with(eps, method, params, false).map_err(|(t, _)| t)
}

pub fn ask_call(eps: &[Endpoint], method: &str, params: &Value) -> Result<Reading, (Trouble, bool)> {
    let _read = OneRead::open();
    ask_with(eps, method, params, true)
}

pub const TX_FACTS: [&str; 5] = eravon_chain::judge::TX_FACTS;

pub const FEE_FACTS: [&str; 3] = eravon_chain::judge::BLOCK_FACTS;

pub const TIP_FACTS: [&str; 1] = eravon_chain::judge::FEE_HISTORY_FACTS;

fn ask_all(eps: &[Endpoint], method: &str, params: &Value) -> (Vec<(String, Result<wire::W, eravon_chain::judge::Missing>)>, Vec<Why>) {
    ask_all_with(eps, method, params, &endpoint_at, None, false)
}

pub(crate) type Open<'a> = dyn Fn(&str) -> Option<Box<dyn rpc::Endpoint + Send>> + 'a;

pub(crate) fn each_node<R: Send + 'static>(urls: &[NodeAddr], open: &Open<'_>, ask: impl Fn(&NodeAddr, Option<&mut dyn rpc::Endpoint>) -> R + Send + Sync + 'static, mut enough: impl FnMut(&[Option<&R>]) -> bool) -> Vec<Option<R>> {
    let opened: Vec<(NodeAddr, Option<Box<dyn rpc::Endpoint + Send>>)> = urls.iter().map(|u| (u.clone(), open(u.for_transport()))).collect();
    endpoints::until_enough(opened, move |(u, mut h)| Some(ask(&u, h.as_deref_mut().map(|e| e as &mut dyn rpc::Endpoint))), || None, |slots: &[Option<Option<R>>]| {
        let view: Vec<Option<&R>> = slots.iter().map(|x| x.as_ref().and_then(Option::as_ref)).collect();
        enough(&view)
    })
    .into_iter()
    .map(Option::flatten)
    .collect()
}

pub fn enough_for(method: &str, params: &Value) -> impl Fn(&[(String, Option<&Result<wire::W, rpc::Trouble>>)]) -> bool + 'static {
    let (m, p) = (method.to_string(), params.clone());
    move |view| {
        let arrived: Vec<(String, Result<wire::W, eravon_chain::judge::Missing>)> = view.iter().filter_map(|(n, a)| a.map(|a| (n.clone(), a.clone().map_err(eravon_chain::judge::Missing::Trouble)))).collect();
        eravon_chain::judge::enough(&m, &p, &arrived, None)
    }
}

pub fn came(got: Vec<(String, Option<Result<wire::W, rpc::Trouble>>)>) -> Vec<(String, Result<wire::W, rpc::Trouble>)> {
    got.into_iter().filter_map(|(n, a)| a.map(|a| (n, a))).collect()
}

fn ask_all_with(eps: &[Endpoint], method: &str, params: &Value, open: &Open<'_>, floor: Option<u64>, all: bool) -> (Vec<(String, Result<wire::W, eravon_chain::judge::Missing>)>, Vec<Why>) {
    let mut seen: Vec<String> = Vec::new();
    let mut slots: Vec<(String, Option<eravon_chain::judge::Missing>)> = Vec::new();
    let mut nodes: Vec<(String, Box<dyn rpc::Endpoint + Send>)> = Vec::new();
    let mut unread: Vec<Why> = Vec::new();
    let places = endpoints::places_of(eps.iter().map(|e| e.url.for_transport()));
    for (e, place) in eps.iter().zip(places) {
        if seen.contains(&place) {
            continue;
        }
        seen.push(place);
        let url = e.url.for_transport().to_string();
        if let Some(m) = passed_over(&url) {
            slots.push((url, Some(m)));
            continue;
        }
        match open(&url) {
            Some(h) => {
                slots.push((url.clone(), None));
                nodes.push((url, one_ask(h)));
            }
            None => unread.push(address_said(&url)),
        }
    }
    let (m, p) = (method.to_string(), params.clone());
    let enough = move |view: &[(String, Option<&Result<wire::W, rpc::Trouble>>)]| {
        if all || m == rpc::BROADCAST {
            return false;
        }
        let arrived: Vec<(String, Result<wire::W, eravon_chain::judge::Missing>)> = view.iter().filter_map(|(n, a)| a.map(|a| (n.clone(), a.clone().map_err(eravon_chain::judge::Missing::Trouble)))).collect();
        eravon_chain::judge::enough(&m, &p, &arrived, floor)
    };
    let mut got = endpoints::ask_each(nodes, method, params, &enough);
    let had_enough = enough(&viewed(&got));
    let again: Vec<usize> = if had_enough { Vec::new() } else { got.iter().enumerate().filter(|(_, (_, a))| matches!(a, Some(Err(t)) if asked_again(method, t))).map(|(i, _)| i).collect() };
    if !again.is_empty() && method != rpc::BROADCAST {
        let pause = eravon_chain::patience::waited(AGAIN_AFTER);
        if !pause.is_zero() {
            std::thread::sleep(pause);
        }
        let reopened: Vec<(String, Box<dyn rpc::Endpoint + Send>)> = again.iter().filter_map(|i| open(&got[*i].0).map(|h| (got[*i].0.clone(), one_ask(h)))).collect();
        let before: Vec<(String, Result<wire::W, rpc::Trouble>)> = got.iter().filter(|(n, _)| !reopened.iter().any(|(r, _)| r == n)).filter_map(|(n, a)| a.clone().map(|a| (n.clone(), a))).collect();
        let more = |view: &[(String, Option<&Result<wire::W, rpc::Trouble>>)]| {
            let mut all_in: Vec<(String, Option<&Result<wire::W, rpc::Trouble>>)> = before.iter().map(|(n, a)| (n.clone(), Some(a))).collect();
            all_in.extend(view.iter().cloned());
            enough(&all_in)
        };
        for (name, a) in endpoints::ask_each(reopened, method, params, more) {
            if let Some(slot) = got.iter_mut().find(|(n, _)| *n == name) {
                slot.1 = a;
            }
        }
    }
    let answers: Vec<(String, Result<wire::W, eravon_chain::judge::Missing>)> = slots
        .into_iter()
        .filter_map(|(url, before)| match before {
            Some(m) => Some((url, Err(m))),
            None => {
                let came = got.iter().find(|(n, _)| *n == url).and_then(|(_, a)| a.clone())?;
                let got = match came {
                    Ok(w) => match eravon_chain::judge::unreadable(method, params, &w) {
                        Some(m) => {
                            pass_over(&url, &m);
                            Ok(w)
                        }
                        None => Ok(w),
                    },
                    Err(t) => {
                        let pass = passes_over(method, &t);
                        let m = eravon_chain::judge::Missing::Trouble(t);
                        if pass {
                            pass_over(&url, &m);
                        }
                        Err(m)
                    }
                };
                Some((url, got))
            }
        })
        .collect();
    (answers, unread)
}

fn viewed(got: &[(String, Option<Result<wire::W, rpc::Trouble>>)]) -> Vec<(String, Option<&Result<wire::W, rpc::Trouble>>)> {
    got.iter().map(|(n, a)| (n.clone(), a.as_ref())).collect()
}

pub use eravon_chain::patience::AGAIN_AFTER;
use eravon_chain::patience::asked_again;

fn missing_said(url: &str, m: &eravon_chain::judge::Missing) -> Why {
    match m {
        eravon_chain::judge::Missing::Trouble(t) => node_why(url, t),
        eravon_chain::judge::Missing::Unreadable(_) => Why::Unreadable { node: eravon_net::sayable(url) },
    }
}

fn ask_with(eps: &[Endpoint], method: &str, params: &Value, call_asked: bool) -> Result<Reading, (Trouble, bool)> {
    ask_with_open(eps, method, params, call_asked, &endpoint_at)
}

fn ask_with_open(eps: &[Endpoint], method: &str, params: &Value, call_asked: bool, open: &Open<'_>) -> Result<Reading, (Trouble, bool)> {
    use eravon_chain::judge::{self, NoReading};
    if eps.is_empty() {
        return Err((Trouble::text(Code::NoEndpoint, method), false));
    }
    let pauses = judge::not_yet_pauses();
    let mut round = 0usize;
    let mut last = false;
    loop {
        let (answers, unread) = ask_all_with(eps, method, params, open, None, false);
        let asked = answers.len() + unread.len();
        let mut judged = judge::judge_read(method, params, answers.clone());
        if let Err(NoReading::NotYet { not_yet, .. }) = &judged {
            if !judge::NOT_YET_METHODS.contains(&method) {
                if last || round >= pauses.len() {
                    judged = judge::judge_settled(method, params, answers);
                } else if not_yet.iter().all(|u| open(u).is_some_and(|mut ep| eravon_chain::patience::past_it(&mut *ep, method, params))) {
                    last = true;
                    continue;
                }
            }
        }
        match judged {
            Ok(r) => {
                let unanswered: Vec<Why> = unread.iter().cloned().chain(r.missing.iter().map(|(u, m)| missing_said(u, m))).collect();
                let differed: Vec<Why> = r.differed.iter().map(|u| Why::Differs { node: eravon_net::sayable(u) }).collect();
                return Ok(Reading { value: r.value, sources: r.sources.len(), single_source: r.single_source, unanswered, differed });
            }
            Err(NoReading::NotYet { has, not_yet }) => match pauses.get(round) {
                Some(p) => {
                    round += 1;
                    if !p.is_zero() {
                        std::thread::sleep(*p);
                    }
                }
                None => return Err((Trouble::new(Code::Disagree, Why::NotYetAt { has: sayable_all(&has), not_yet: sayable_all(&not_yet) }), false)),
            },
            Err(NoReading::Differ(d)) => {
                return Err((Trouble::new(Code::Disagree, Why::Differ { count: d.sources.len(), sources: sayable_all(&d.sources) }), false));
            }
            Err(NoReading::NoneAnswered(missing)) => {
                let first = missing.iter().find_map(|(u, m)| match m {
                    judge::Missing::Trouble(t) => Some((u, t)),
                    _ => None,
                });
                let call = first.is_some_and(|(_, t)| eravon_chain::said::refuses_the_call(t));
                let refused: Vec<Why> = unread.iter().cloned().chain(missing.iter().map(|(u, m)| missing_said(u, m))).collect();
                if unread.len() == asked {
                    return Err((no_address_reads(refused), false));
                }
                return Err((none_answered(first.map(|(u, t)| said_trouble(u, t)), missing.iter().any(|(_, m)| shapeless(m, call_asked)), refused), call));
            }
        }
    }
}

pub const MAX_HEAD_LAG: u64 = 64;

pub fn head_block(eps: &[Endpoint], chain: u64) -> Result<(u64, usize), Trouble> {
    let _read = OneRead::open();
    let floor = floor_of(eps, chain);
    let heads = lowest_head(eps, chain, floor)?;
    match floor {
        Some(floor) => reach(eps, chain, heads, floor),
        None => Ok((heads.low, heads.n)),
    }
}

pub(crate) fn floor_of(eps: &[Endpoint], chain: u64) -> Option<u64> {
    eravon_chain::action::floor_among(eps.iter().filter(|e| e.chain == chain).map(|e| e.url.for_transport()))
}

fn reach(eps: &[Endpoint], chain: u64, mut heads: Heads, floor: u64) -> Result<(u64, usize), Trouble> {
    for tick in 0..=FLOOR_TICKS {
        if tick > 0 {
            heads = eravon_chain::action::after_a_beat(eravon_chain::patience::waited(FLOOR_TICK), || lowest_head(eps, chain, Some(floor)))?;
        }
        if let Some(there) = on_floor(&heads.heights, floor) {
            for (url, _) in heads.heights.iter().filter(|(_, h)| *h < floor) {
                pass_over(url, &eravon_chain::judge::Missing::Trouble(rpc::Trouble::Transport(rpc::Broke::HeadBehind { node: eravon_net::sayable(url), block: floor })));
            }
            return Ok((heads.low, there.len()));
        }
    }
    Err(Trouble::new(Code::Disagree, Why::Behind { node: eravon_net::sayable(&heads.low_at), lag: floor - heads.low, high: floor }))
}

pub fn on_floor<T: AsRef<str>>(heights: &[(T, u64)], floor: u64) -> Option<Vec<&(T, u64)>> {
    let (_, head) = eravon_chain::judge::reached(heights)?;
    (head >= floor).then(|| heights.iter().filter(|(_, h)| *h >= floor).collect())
}

pub const FLOOR_TICK: std::time::Duration = std::time::Duration::from_secs(1);
pub const FLOOR_TICKS: usize = 30;

struct Heads {
    low: u64,
    n: usize,
    low_at: String,
    heights: Vec<(String, u64)>,
}

fn lowest_head(eps: &[Endpoint], chain: u64, floor: Option<u64>) -> Result<Heads, Trouble> {
    let _read = OneRead::open();
    use eravon_chain::judge::{self, NoReading};
    let eps: Vec<Endpoint> = eps.iter().filter(|e| e.chain == chain).cloned().collect();
    if eps.is_empty() {
        return Err(Trouble::new(Code::NoEndpoint, Why::NoHeadNode { chain }));
    }
    let empty = Value::Arr(Vec::new());
    let (answers, unread) = ask_all_with(&eps, "eth_blockNumber", &empty, &endpoint_at, floor, false);
    let asked = answers.len() + unread.len();
    let heights: Vec<(String, u64)> = answers.iter().filter_map(|(u, a)| a.as_ref().ok().and_then(height_of).map(|n| (u.clone(), n))).collect();
    let wide: Vec<Why> = answers
        .iter()
        .filter_map(|(u, a)| match a {
            Ok(w) if height_of(w).is_none() && wire::to_core(w).is_some_and(|v| v.as_str().and_then(wei).is_some()) => {
                pass_over(u, &judge::Missing::Unreadable(wire::write(w)));
                Some(Why::NotHeight { node: eravon_net::sayable(u), text: wire::write(w).trim_matches('"').to_string() })
            }
            _ => None,
        })
        .collect();
    let said_height = |u: &str, m: &judge::Missing| match m {
        judge::Missing::Unreadable(text) if text.starts_with('"') => Why::NotHeight { node: eravon_net::sayable(u), text: text.trim_matches('"').to_string() },
        other => missing_said(u, other),
    };
    match judge::judge_read("eth_blockNumber", &empty, answers) {
        Ok(j) => {
            let Some((low_at, low)) = eravon_chain::judge::reached(&heights).map(|(i, h)| (&heights[i].0, h)) else {
                let first = j.missing.iter().find_map(|(u, m)| match m {
                    judge::Missing::Trouble(t) => Some(said_trouble(u, t)),
                    _ => None,
                });
                let refused: Vec<Why> = unread.iter().cloned().chain(wide.iter().cloned()).chain(j.missing.iter().map(|(u, m)| said_height(u, m))).collect();
                return Err(none_answered(first, !wide.is_empty() || j.missing.iter().any(|(_, m)| shapeless(m, false)), refused));
            };
            let high = heights.iter().map(|(_, h)| *h).max().unwrap_or(low);
            if high - low > MAX_HEAD_LAG {
                return Err(Trouble::new(Code::Disagree, Why::Behind { node: eravon_net::sayable(low_at), lag: high - low, high }));
            }
            Ok(Heads { low, n: heights.len(), low_at: low_at.clone(), heights: heights.clone() })
        }
        Err(NoReading::NoneAnswered(missing)) => {
            let first = missing.iter().find_map(|(u, m)| match m {
                judge::Missing::Trouble(t) => Some(said_trouble(u, t)),
                _ => None,
            });
            let refused: Vec<Why> = unread.iter().cloned().chain(missing.iter().map(|(u, m)| said_height(u, m))).collect();
            if unread.len() == asked {
                return Err(no_address_reads(refused));
            }
            Err(none_answered(first, missing.iter().any(|(_, m)| shapeless(m, false)), refused))
        }
        Err(_) => Err(Trouble::text(Code::Unreachable, "")),
    }
}

fn height_of(w: &wire::W) -> Option<u64> {
    wire::to_core(w).and_then(|v| v.as_str().and_then(wei)).and_then(|n| u64::try_from(n).ok())
}

pub fn head_time(eps: &[Endpoint], chain: u64) -> Result<(u64, u64, usize), Trouble> {
    let _read = OneRead::open();
    let eps: Vec<Endpoint> = eps.iter().filter(|e| e.chain == chain).cloned().collect();
    if eps.is_empty() {
        return Err(Trouble::new(Code::NoEndpoint, Why::NoTimeNode { chain }));
    }
    let params = Value::Arr(vec![Value::Str("latest".into()), Value::Bool(false)]);
    let (answers, unread) = ask_all(&eps, "eth_getBlockByNumber", &params);
    let asked = answers.len() + unread.len();
    let mut seen: Vec<(u64, u64)> = Vec::new();
    let mut refused: Vec<Why> = unread.clone();
    let mut first: Option<Trouble> = None;
    let mut shape = false;
    for (url, got) in answers {
        match got {
            Ok(w) => match wire::to_core(&w) {
                Some(Value::Obj(m)) => {
                    let take = |k: &str| -> Option<u64> {
                        m.iter().find(|(n, _)| n == k).and_then(|(_, v)| match v {
                            Value::Str(h) => wei(h).filter(|n| *n <= u128::from(u64::MAX)).map(|n| n as u64),
                            _ => None,
                        })
                    };
                    match (take("timestamp"), take("number")) {
                        (Some(t), Some(n)) => seen.push((t, n)),
                        _ => {
                            shape = true;
                            pass_over(&url, &eravon_chain::judge::Missing::Unreadable(wire::write(&w)));
                            refused.push(Why::NoTime { node: eravon_net::sayable(&url) })
                        }
                    }
                }
                other => {
                    shape |= !matches!(other, Some(Value::Null));
                    refused.push(Why::Unreadable { node: eravon_net::sayable(&url) })
                }
            },
            Err(eravon_chain::judge::Missing::Trouble(t)) => {
                first.get_or_insert_with(|| said_trouble(&url, &t));
                refused.push(node_why(&url, &t));
            }
            Err(eravon_chain::judge::Missing::Unreadable(_)) => {
                shape = true;
                refused.push(Why::Unreadable { node: eravon_net::sayable(&url) });
            }
        }
    }
    match seen.iter().max_by_key(|(t, n)| (*n, *t)) {
        Some((t, n)) => Ok((*t, *n, seen.len())),
        None if unread.len() == asked => Err(no_address_reads(refused)),
        None => Err(none_answered(first, shape, refused)),
    }
}

static SERVES: std::sync::Mutex<std::collections::BTreeMap<(String, u64), u64>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

pub fn serving(eps: &[Endpoint], chain: u64) -> Result<(Vec<Endpoint>, Vec<Why>), Trouble> {
    let _read = OneRead::open();
    let mine: Vec<Endpoint> = eps.iter().filter(|e| e.chain == chain).cloned().collect();
    let unknown: Vec<Endpoint> = {
        let known = SERVES.lock().unwrap_or_else(|e| e.into_inner());
        mine.iter().filter(|e| !known.contains_key(&(e.url.place(), chain))).cloned().collect()
    };
    if !unknown.is_empty() {
        let (answers, _) = ask_all(&unknown, "eth_chainId", &Value::Arr(Vec::new()));
        let mut known = SERVES.lock().unwrap_or_else(|e| e.into_inner());
        for (url, got) in answers {
            if let Some(id) = got.ok().as_ref().and_then(eravon_chain::scan::chain_served) {
                known.insert((endpoints::place_of(&url), chain), id);
            }
        }
    }
    let known = SERVES.lock().unwrap_or_else(|e| e.into_inner());
    let mut kept = Vec::new();
    let mut left: Vec<Why> = Vec::new();
    for e in mine {
        match eravon_chain::scan::serves_another(known.get(&(e.url.place(), chain)).copied(), chain) {
            Some(serves) => left.push(Why::WrongChainNode { node: e.url.said(), serves, chain }),
            None => kept.push(e),
        }
    }
    if kept.is_empty() && !left.is_empty() {
        return Err(Trouble::new(Code::WrongChain, Why::Each(left)));
    }
    Ok((kept, left))
}

pub fn ask_ahead(eps: &[Endpoint], chain: u64, asks: &(dyn Fn(&mut dyn rpc::Endpoint, u64, u64) -> Vec<rpc::Ask<'static>> + Sync)) {
    if eravon_chain::action::current().is_none() {
        return;
    }
    let mine: Vec<Endpoint> = eps.iter().filter(|e| e.chain == chain).cloned().collect();
    let places = endpoints::places_of(mine.iter().map(|e| e.url.for_transport()));
    let mut seen: Vec<String> = Vec::new();
    let mut nodes: Vec<(String, Box<dyn rpc::Endpoint + Send>)> = Vec::new();
    for (e, place) in mine.iter().zip(places) {
        let url = e.url.for_transport().to_string();
        if seen.contains(&place) || passed_over(&url).is_some() {
            continue;
        }
        seen.push(place);
        if let Some(h) = endpoint_at(&url) {
            nodes.push((url, h));
        }
    }
    let empty = Value::Arr(Vec::new());
    let known = move |url: &str| SERVES.lock().unwrap_or_else(|e| e.into_inner()).get(&(endpoints::place_of(url), chain)).copied();
    let firsts: Vec<(String, Box<dyn rpc::Endpoint + Send>, Vec<rpc::Ask<'static>>)> = nodes
        .into_iter()
        .map(|(url, ep)| {
            let mut first: Vec<rpc::Ask<'static>> = Vec::new();
            if known(&url).is_none() {
                first.push(("eth_chainId", empty.clone()));
            }
            if eravon_chain::action::answered(&url, "eth_blockNumber", &empty).is_none() {
                first.push(("eth_blockNumber", empty.clone()));
            }
            (url, ep, first)
        })
        .collect();
    type Came = Option<(String, Box<dyn rpc::Endpoint + Send>, bool, Option<u64>)>;
    let first = endpoints::until_enough(
        firsts,
        move |(url, mut ep, first)| -> Came {
            let empty = Value::Arr(Vec::new());
            match first.as_slice() {
                [(m, p)] if *m == "eth_blockNumber" => {
                    let _ = eravon_chain::patience::ask(ep.as_mut(), m, p);
                }
                _ => eravon_chain::ahead::ask_ahead(ep.as_mut(), &first),
            }
            let held = |method: &str| eravon_chain::action::answered(&url, method, &empty);
            let serves = known(&url).or_else(|| held("eth_chainId").as_ref().and_then(eravon_chain::scan::chain_served));
            let head = held("eth_blockNumber").as_ref().and_then(height_of);
            Some((url, ep, serves.is_none_or(|c| c == chain), head))
        },
        || None,
        |slots: &[Option<Came>]| endpoints::distinct_places(slots.iter().filter_map(|s| s.as_ref().and_then(Option::as_ref)).filter(|(_, _, serves, head)| *serves && head.is_some()).map(|(u, _, _, _)| u.as_str())) >= 2,
    );
    let here: Vec<(String, Box<dyn rpc::Endpoint + Send>, u64)> = first.into_iter().flatten().flatten().filter(|(_, _, serves, _)| *serves).filter_map(|(u, ep, _, head)| head.map(|h| (u, ep, h))).collect();
    let here: Vec<(String, Box<dyn rpc::Endpoint + Send>, u64)> = match floor_of(&mine, chain) {
        Some(floor) => {
            let heads: Vec<(String, u64)> = here.iter().map(|(u, _, h)| (u.clone(), *h)).collect();
            let Some(there) = on_floor(&heads, floor) else { return };
            let keep: Vec<String> = there.iter().map(|(u, _)| u.clone()).collect();
            here.into_iter().filter(|(u, _, _)| keep.contains(u)).collect()
        }
        None => here,
    };
    let heads: Vec<(String, u64)> = here.iter().map(|(u, _, h)| (u.clone(), *h)).collect();
    let Some((_, lowest)) = eravon_chain::judge::reached(&heads) else { return };
    let seconds: Vec<(String, Box<dyn rpc::Endpoint + Send>, Vec<rpc::Ask<'static>>)> = here
        .into_iter()
        .filter(|(_, _, h)| *h >= lowest)
        .map(|(u, mut ep, own)| {
            let these = asks(ep.as_mut(), lowest, own);
            (u, ep, these)
        })
        .collect();
    let _ = endpoints::until_enough(
        seconds,
        |(u, mut ep, these)| {
            eravon_chain::ahead::ask_ahead(ep.as_mut(), &these);
            u
        },
        String::new,
        |slots: &[Option<String>]| endpoints::distinct_places(slots.iter().flatten().filter(|u| !u.is_empty()).map(String::as_str)) >= 2,
    );
}

#[derive(Clone, Debug, PartialEq)]
pub enum AsHeld {
    Held(Value),
    Nobody,
    Unread,
    NoNode,
}

impl AsHeld {
    pub fn held(self) -> Option<Value> {
        match self {
            AsHeld::Held(v) => Some(v),
            AsHeld::Nobody | AsHeld::Unread | AsHeld::NoNode => None,
        }
    }
}

pub fn tx_as_held(urls: &[NodeAddr], chain: u64, tx: &str) -> AsHeld {
    let eps: Vec<Endpoint> = urls.iter().map(|u| Endpoint { chain, url: u.clone() }).collect();
    let Ok((eps, _)) = serving(&eps, chain) else { return AsHeld::NoNode };
    if eps.is_empty() {
        return AsHeld::NoNode;
    }
    let q = Value::Arr(vec![Value::Str(tx.to_string())]);
    let addrs: Vec<NodeAddr> = eps.iter().map(|e| e.url.clone()).collect();
    let held = |w: &wire::W| !w.is_null() && matches!(wire::to_core(w), Some(Value::Obj(_)));
    let came = each_node(&addrs, &endpoint_at, move |_, ep| ep.map(|ep| ep.call_within("eth_getTransactionByHash", &q, ONE_ASK)), |view| view.iter().any(|g| matches!(g, Some(Some(Ok(w))) if held(w))));
    let mut every_said = true;
    for g in came {
        match g.flatten() {
            Some(Ok(w)) if w.is_null() => {}
            Some(Ok(w)) => match wire::to_core(&w) {
                Some(v @ Value::Obj(_)) => return AsHeld::Held(v),
                _ => every_said = false,
            },
            _ => every_said = false,
        }
    }
    if every_said {
        AsHeld::Nobody
    } else {
        AsHeld::Unread
    }
}

pub fn forget_serving() {
    SERVES.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

pub fn balance(eps: &[Endpoint], chain: u64, who: &[u8; 20]) -> Result<(u128, Reading), Trouble> {
    let _read = OneRead::open();
    let eps: Vec<Endpoint> = eps.iter().filter(|e| e.chain == chain).cloned().collect();
    let (eps, left) = serving(&eps, chain)?;
    let (height, _) = head_block(&eps, chain).map_err(|t| crate::batch::with_left(t, &left))?;
    let params = Value::Arr(vec![Value::Str(eravon::hexfmt::encode(who)), Value::Str(format!("0x{height:x}"))]);
    let mut r = ask(&eps, "eth_getBalance", &params).map_err(|t| crate::batch::with_left(t, &left))?;
    r.unanswered.splice(0..0, left);
    let hex = match &r.value {
        Value::Str(s) => s.clone(),
        other => return Err(Trouble::new(Code::ChainShape, Why::BalanceNotText(format!("{:?}", other)))),
    };
    let n = wei(&hex).ok_or_else(|| Trouble::new(Code::ChainShape, Why::BalanceNotQty(hex.clone())))?;
    Ok((n, r))
}

pub fn wei(hex: &str) -> Option<u128> {
    let bare = hex.strip_prefix("0x")?;
    if bare.is_empty() || bare.len() > 32 || !bare.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u128::from_str_radix(bare, 16).ok()
}

pub fn fees(eps: &[Endpoint], chain: u64) -> (eravon_chain::send::Fees, Vec<Why>) {
    let r = fee_reading(eps, chain);
    (r.fees, r.left)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeeReading {
    pub base: Option<u64>,
    pub fees: eravon_chain::send::Fees,
    pub left: Vec<Why>,
}

pub fn fee_reading(eps: &[Endpoint], chain: u64) -> FeeReading {
    let _read = OneRead::open();
    use eravon_chain::send::{tip_of, tip_params, Fees};
    let unread = |left: Vec<Why>| FeeReading { base: None, fees: Fees::fallback(), left };
    ask_ahead(eps, chain, &|_, lowest, _| vec![("eth_getBlockByNumber", Value::Arr(vec![Value::Str(format!("0x{lowest:x}")), Value::Bool(false)])), ("eth_feeHistory", tip_params(lowest))]);
    let (eps, left) = match serving(eps, chain) {
        Ok(x) => x,
        Err(t) => return unread(vec![t.why]),
    };
    let Ok((height, _)) = head_block(&eps, chain) else {
        return unread(left);
    };
    let params = Value::Arr(vec![Value::Str(format!("0x{height:x}")), Value::Bool(false)]);
    let base = ask(&eps, "eth_getBlockByNumber", &params).ok().and_then(|r| eravon_chain::send::base_fee_of(&r.value));
    let tip = ask(&eps, "eth_feeHistory", &tip_params(height)).ok().and_then(|r| tip_of(&r.value));
    FeeReading { base, fees: Fees::of(base, tip), left }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::sync::Arc;

    fn node(answer: fn(&str, usize) -> String) -> (String, Arc<std::sync::Mutex<Vec<String>>>) {
        let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!("http://{}", l.local_addr().expect("addr"));
        let asked = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let log = asked.clone();
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                let log = log.clone();
                std::thread::spawn(move || {
                    let mut s = s;
                    let mut raw = Vec::new();
                    let mut buf = [0u8; 4096];
                    let body = loop {
                        match s.read(&mut buf) {
                            Ok(0) | Err(_) => return,
                            Ok(n) => raw.extend_from_slice(&buf[..n]),
                        }
                        let text = String::from_utf8_lossy(&raw).to_string();
                        if let Some(i) = text.find("\r\n\r\n") {
                            let len: usize = text[..i].lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").and_then(|n| n.trim().parse().ok())).unwrap_or(0);
                            if raw.len() >= i + 4 + len {
                                break text[i + 4..].to_string();
                            }
                        }
                    };
                    let method = body.split("\"method\":\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("").to_string();
                    let id = body.split("\"id\":").nth(1).and_then(|r| r.split([',', '}']).next()).unwrap_or("1").trim().to_string();
                    let n = {
                        let mut l = log.lock().unwrap();
                        let n = l.iter().filter(|m| **m == method).count();
                        l.push(method.clone());
                        n
                    };
                    let body = format!("{{\"id\":{id},\"jsonrpc\":\"2.0\",{}}}", answer(&method, n));
                    let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
                });
            }
        });
        (url, asked)
    }

    fn times(asked: &Arc<std::sync::Mutex<Vec<String>>>, method: &str) -> usize {
        asked.lock().unwrap().iter().filter(|m| *m == method).count()
    }

    const CHAIN_HEX: &str = r#""result":"0x7a69""#;

    #[test]
    fn a_transaction_held_by_none_is_told_apart_from_one_no_node_answered() {
        forget_serving();
        let none = |m: &str, _: usize| if m == "eth_chainId" { CHAIN_HEX.to_string() } else { r#""result":null"#.to_string() };
        let refuses = |m: &str, _: usize| if m == "eth_chainId" { CHAIN_HEX.to_string() } else { r#""error":{"code":-32000,"message":"header not found"}"#.to_string() };
        let holds = |m: &str, _: usize| if m == "eth_chainId" { CHAIN_HEX.to_string() } else { r#""result":{"hash":"0xab","nonce":"0x1"}"#.to_string() };
        let tx = format!("0x{}", "ab".repeat(32));
        let ((a, _), (b, _)) = (node(none), node(none));
        assert_eq!(tx_as_held(&[NodeAddr::new(a.clone()), NodeAddr::new(b)], 31337, &tx), AsHeld::Nobody, "every node said none");
        let (c, _) = node(refuses);
        assert_eq!(tx_as_held(&[NodeAddr::new(a.clone()), NodeAddr::new(c.clone())], 31337, &tx), AsHeld::Unread, "one node gave no answer");
        let (d, _) = node(holds);
        assert!(matches!(tx_as_held(&[NodeAddr::new(c), NodeAddr::new(d), NodeAddr::new(a)], 31337, &tx), AsHeld::Held(_)), "one node holds it");
    }

    #[test]
    fn a_transaction_known_to_any_node_is_known_whichever_refuses() {
        let refuses = |_: &str, _: usize| r#""error":{"code":-32000,"message":"busy"}"#.to_string();
        let knows = |_: &str, _: usize| r#""result":{"hash":"0xab"}"#.to_string();
        let none = |_: &str, _: usize| r#""result":null"#.to_string();
        let ((r, _), (k, asked_k), (n, _)) = (node(refuses), node(knows), node(none));
        assert!(tx_known(&[NodeAddr::new(r.clone()), NodeAddr::new(k)], 31337, &[0xab; 32]));
        assert_eq!(times(&asked_k, "eth_getTransactionByHash"), 1, "asked once");
        assert!(!tx_known(&[NodeAddr::new(r), NodeAddr::new(n)], 31337, &[0xab; 32]));
    }

    #[test]
    fn two_places_that_have_not_heard_of_a_transaction_do_not_outvote_the_one_holding_it() {
        forget_serving();
        let none = |m: &str, _: usize| if m == "eth_chainId" { CHAIN_HEX.to_string() } else { r#""result":null"#.to_string() };
        let late_holds = |m: &str, _: usize| {
            if m == "eth_chainId" {
                return CHAIN_HEX.to_string();
            }
            std::thread::sleep(std::time::Duration::from_millis(400));
            r#""result":{"hash":"0xab","nonce":"0x1"}"#.to_string()
        };
        let ((a, _), (b, _), (c, _)) = (node(none), node(none), node(late_holds));
        let three = [NodeAddr::new(a.clone()), NodeAddr::new(b.clone()), NodeAddr::new(c)];
        assert!(tx_known(&three, 31337, &[0xab; 32]), "the third node knows it");
        assert!(matches!(tx_as_held(&three, 31337, &format!("0x{}", "ab".repeat(32))), AsHeld::Held(_)), "the third node holds it");
        let two = [NodeAddr::new(a), NodeAddr::new(b)];
        assert!(!tx_known(&two, 31337, &[0xab; 32]), "neither of two knows it");
        assert_eq!(tx_as_held(&two, 31337, &format!("0x{}", "ab".repeat(32))), AsHeld::Nobody, "both of two said none");
    }

    #[test]
    fn a_load_bearing_read_gives_each_question_one_asks_time() {
        #[derive(Clone, Default)]
        struct Deadlines(Arc<std::sync::Mutex<Vec<(String, Option<std::time::Duration>)>>>);
        impl rpc::Endpoint for Deadlines {
            fn call(&mut self, m: &str, _: &Value) -> Result<wire::W, rpc::Trouble> {
                self.0.lock().unwrap().push((m.to_string(), None));
                Ok(wire::W::of(wire::Body::Null))
            }
            fn call_within(&mut self, m: &str, _: &Value, within: std::time::Duration) -> Result<wire::W, rpc::Trouble> {
                self.0.lock().unwrap().push((m.to_string(), Some(within)));
                Ok(wire::W::of(wire::Body::Null))
            }
            fn name(&self) -> String {
                "deadlines".into()
            }
            fn place(&self) -> String {
                "deadlines".into()
            }
        }
        let seen = Deadlines::default();
        let mut ep = one_ask(Box::new(seen.clone()));
        let q = Value::Arr(Vec::new());
        let _ = ep.call("eth_getBalance", &q);
        let _ = ep.call(rpc::BROADCAST, &q);
        let _ = ep.call("eth_getLogs", &q);
        let _ = ep.call_within("eth_blockNumber", &q, std::time::Duration::from_secs(1));
        let _ = ep.call_within("eth_blockNumber", &q, std::time::Duration::from_secs(60));
        assert_eq!(
            seen.0.lock().unwrap().clone(),
            vec![("eth_getBalance".to_string(), Some(ONE_ASK)), (rpc::BROADCAST.to_string(), None), ("eth_getLogs".to_string(), None), ("eth_blockNumber".to_string(), Some(std::time::Duration::from_secs(1))), ("eth_blockNumber".to_string(), Some(ONE_ASK))]
        );
    }

    const RAW: &str = "eth_getRawTransactionByHash";

    fn balance_q() -> Value {
        Value::Arr(vec![Value::Str(eravon::hexfmt::encode(&[0x11; 20])), Value::Str("0x100".into())])
    }

    #[test]
    fn a_node_that_lacks_a_method_is_still_asked_the_reads_other_questions_and_never_that_method_again() {
        eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
        let (lacking, lacking_asked) = node(|m, _| match m {
            RAW => r#""error":{"code":-32601,"message":"the method eth_getRawTransactionByHash does not exist/is not available"}"#.into(),
            _ => r#""result":"0x5""#.into(),
        });
        let (full, full_asked) = node(|m, _| match m {
            RAW => r#""result":"0x02f8""#.into(),
            _ => r#""result":"0x5""#.into(),
        });
        let eps = vec![Endpoint::at(31337, NodeAddr::new(lacking.clone())), Endpoint::at(31337, NodeAddr::new(full))];
        let read = OneRead::open();
        let raw = Value::Arr(vec![Value::Str(format!("0x{}", "ab".repeat(32)))]);
        let first = ask(&eps, RAW, &raw).expect("the raw transaction from the node that has the method");
        assert!(first.single_source, "{first:?}");
        assert_eq!(times(&lacking_asked, RAW), 1, "a method missing is not asked again");
        let later = ask(&eps, "eth_getBalance", &balance_q()).expect("the balance");
        assert!(!later.single_source && later.sources == 2, "the node lacking a method still answers the balance: {:?}", later.unanswered);
        assert_eq!(times(&lacking_asked, "eth_getBalance"), 1);
        let again = ask(&eps, RAW, &raw).expect("the raw transaction again");
        drop(read);
        let next_read = ask(&eps, RAW, &raw).expect("the raw transaction in the next read");
        assert_eq!(times(&lacking_asked, RAW), 1, "a method a node said it lacks is not asked of it again");
        assert_eq!(times(&full_asked, RAW), 3, "the node that has it is asked each time");
        for r in [&again, &next_read] {
            assert_eq!((r.single_source, r.sources), (true, 1), "{r:?}");
            assert!(matches!(r.unanswered.as_slice(), [Why::Node { node, trouble: rpc::Trouble::Node(said) }] if *node == eravon_net::sayable(&lacking) && said.contains("-32601")), "{:?}", r.unanswered);
        }
        let balance = ask(&eps, "eth_getBalance", &balance_q()).expect("the balance in the next read");
        assert!(!balance.single_source && balance.sources == 2, "{:?}", balance.unanswered);
        assert_eq!(times(&lacking_asked, "eth_getBalance"), 2, "every other method is still asked of it");
        assert_eq!(endpoints::lacking(&endpoints::place_of(&lacking), RAW).as_deref().map(|s| s.contains("-32601")), Some(true), "kept by place and method");
        assert_eq!(endpoints::lacking(&endpoints::place_of(&lacking), "eth_getBalance"), None);
    }

    #[test]
    fn a_refusal_the_patience_table_has_no_row_for_is_asked_once_more_before_the_node_is_passed_over() {
        eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
        let (hiccup, hiccup_asked) = node(|_, n| match n {
            0 => r#""error":{"code":-32603,"message":"service temporarily unavailable"}"#.into(),
            _ => r#""result":"0x5""#.into(),
        });
        let (steady, _) = node(|_, _| r#""result":"0x5""#.into());
        let eps = vec![Endpoint::at(31337, NodeAddr::new(hiccup)), Endpoint::at(31337, NodeAddr::new(steady))];
        let _read = OneRead::open();
        let r = ask(&eps, "eth_getBalance", &balance_q()).expect("the balance");
        assert!(!r.single_source && r.sources == 2, "{:?}", r.unanswered);
        assert_eq!(times(&hiccup_asked, "eth_getBalance"), 2);

        let node_said = |code: i64, m: &str| rpc::Trouble::Node(format!("{{\"code\":{code},\"message\":\"{m}\"}}"));
        for t in [node_said(3, "execution reverted"), node_said(-32000, "insufficient funds for gas * price + value"), node_said(-32000, "nonce too low"), node_said(-32000, "already known"), node_said(-32000, "replacement transaction underpriced"), node_said(-32000, "intrinsic gas too low"), node_said(-32601, "method not found")] {
            assert!(!passes_over("eth_call", &t), "{t:?}");
            assert!(!asked_again("eth_call", &t), "{t:?}");
        }
        assert!(asked_again("eth_call", &node_said(-32603, "service temporarily unavailable")));
        assert!(passes_over("eth_call", &node_said(-32603, "service temporarily unavailable")));
        assert!(!asked_again("eth_call", &node_said(-32005, "daily request limit exceeded")), "the patience table waits a rate limit out");
        assert!(passes_over("eth_call", &rpc::Trouble::Transport(rpc::Broke::Closed { node: "b".into() })), "a broken way is passed over");
        assert!(!passes_over("eth_call", &rpc::Trouble::Transport(rpc::Broke::NoKey { node: "b".into() })));
        assert!(!passes_over("eth_call", &rpc::Trouble::Transport(rpc::Broke::Unserved { node: "b".into(), method: RAW.into() })));
        let range = node_said(-32012, "getLogs request exceeded max allowed range");
        assert!(!passes_over("eth_getLogs", &range) && eravon_chain::patience::asked_again("eth_getLogs", &range), "拒块段:不跳过,隔拍再问一次");
        assert!(!passes_over("eth_call", &node_said(-32602, "invalid address")), "拒参数形:只这一问不答");
    }

    #[derive(Clone)]
    struct Chan {
        name: String,
        asked: Arc<std::sync::Mutex<Vec<String>>>,
        answer: fn(&str, &str, u64, u64) -> Result<String, rpc::Trouble>,
    }

    impl rpc::Endpoint for Chan {
        fn call(&mut self, method: &str, params: &Value) -> Result<wire::W, rpc::Trouble> {
            let at = |k: &str| match params {
                Value::Arr(ps) => ps.first().and_then(|f| f.member(k)).and_then(Value::as_str).and_then(|x| u64::from_str_radix(x.trim_start_matches("0x"), 16).ok()).unwrap_or(0),
                _ => 0,
            };
            let (from, to) = (at("fromBlock"), at("toBlock"));
            self.asked.lock().unwrap().push(format!("{} {method} {from}..{to}", self.name));
            if self.name.contains("late") {
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            if self.name.contains("hung") {
                std::thread::sleep(std::time::Duration::from_millis(2500));
            }
            (self.answer)(&self.name, method, from, to).map(|t| wire::parse(t.as_bytes()).expect("an answer"))
        }
        fn name(&self) -> String {
            self.name.clone()
        }
        fn place(&self) -> String {
            self.name.clone()
        }
    }

    fn chans(of: &[&str], answer: fn(&str, &str, u64, u64) -> Result<String, rpc::Trouble>) -> (Vec<Endpoint>, Arc<std::sync::Mutex<Vec<String>>>, Box<Open<'static>>) {
        let asked: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
        let all: Vec<Chan> = of.iter().map(|n| Chan { name: n.to_string(), asked: asked.clone(), answer }).collect();
        let eps = of.iter().map(|n| Endpoint::at(1, NodeAddr::new(*n))).collect();
        (eps, asked, Box::new(move |u: &str| all.iter().find(|c| c.name == u).map(|c| Box::new(c.clone()) as Box<dyn rpc::Endpoint + Send>)))
    }

    fn who_asked(asked: &Arc<std::sync::Mutex<Vec<String>>>, method: &str) -> Vec<String> {
        asked.lock().unwrap().iter().filter(|l| l.split(' ').nth(1) == Some(method)).map(|l| l.split(' ').next().unwrap_or("").to_string()).collect()
    }

    fn logs_over(from: u64, to: u64) -> Value {
        Value::Arr(vec![eravon::json::parse(format!("{{\"fromBlock\":\"0x{from:x}\",\"toBlock\":\"0x{to:x}\"}}").as_bytes()).expect("a filter")])
    }

    fn too_wide() -> rpc::Trouble {
        rpc::Trouble::Node("{\"code\":-32012,\"message\":\"getLogs request exceeded max allowed range\"}".into())
    }

    #[test]
    fn a_range_a_node_refused_is_asked_of_it_once_more_and_again_in_the_next_read() {
        eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
        let answer = |name: &str, _: &str, from: u64, to: u64| if name.starts_with("bounded") && to - from + 1 > 100 { Err(too_wide()) } else { Ok("[]".to_string()) };
        let (eps, asked, open) = chans(&["bounded-20", "whole-20"], answer);
        let (first, _) = ask_all_with(&eps, "eth_getLogs", &logs_over(0, 1000), &*open, None, false);
        let mut once: Vec<String> = asked.lock().unwrap().iter().filter(|l| l.split(' ').nth(1) == Some("eth_getLogs")).cloned().collect();
        once.sort();
        assert_eq!(once, ["bounded-20 eth_getLogs 0..1000", "bounded-20 eth_getLogs 0..1000", "whole-20 eth_getLogs 0..1000"], "refused, asked once more");
        asked.lock().unwrap().clear();
        let (second, _) = ask_all_with(&eps, "eth_getLogs", &logs_over(7, 1500), &*open, None, false);
        let mut again = who_asked(&asked, "eth_getLogs");
        again.sort();
        assert_eq!(again, ["bounded-20", "bounded-20", "whole-20"], "as wide: asked again, its words not replayed");
        let words = |a: &[(String, Result<wire::W, eravon_chain::judge::Missing>)]| a.iter().find(|(u, _)| u == "bounded-20").map(|(_, a)| format!("{a:?}")).unwrap_or_default();
        assert_eq!(words(&second), words(&first), "its slot carries the words it said");
        assert!(words(&second).contains("-32012"));
    }

    fn logs_at(from: u64, to: u64) -> String {
        let at: Vec<String> = [7u64, 999].iter().filter(|b| (from..=to).contains(*b)).map(|b| format!("{{\"address\":\"0xab\",\"blockNumber\":\"0x{b:x}\"}}")).collect();
        format!("[{}]", at.join(","))
    }

    #[test]
    fn a_rest_row_is_asked_every_question_with_the_json_rpc_nodes_and_bears_witness_alike() {
        eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
        let row = "https://late.example.test/api#rest=1";
        let answer = |name: &str, method: &str, from: u64, to: u64| match method {
            _ if name.starts_with("down") => Err(rpc::Trouble::Transport(rpc::Broke::Short)),
            _ if name.starts_with("odd") => Ok("\"zz\"".to_string()),
            "eth_getLogs" => Ok(logs_at(from, to)),
            _ => Ok("\"0x60\"".to_string()),
        };
        let code = Value::Arr(Vec::new());
        let (eps, asked, open) = chans(&["up-a", "up-b", row], answer);
        let r = ask_with_open(&eps, "eth_getCode", &code, false, &*open).map_err(|(t, _)| format!("{t:?}")).expect("a reading");
        let code_asked = who_asked(&asked, "eth_getCode");
        for n in ["up-a", "up-b"] {
            assert_eq!(code_asked.iter().filter(|x| *x == n).count(), 1, "{n}: asked once: {code_asked:?}");
        }
        assert!(code_asked.iter().filter(|x| *x == row).count() <= 1 && code_asked.len() <= 3, "{code_asked:?}");
        assert_eq!((r.sources, r.single_source, r.unanswered.len()), (2, false, 0), "two places alike, the row not waited for: {:?}", r.unanswered);
        let r = ask_with_open(&eps, "eth_getLogs", &logs_over(0, 1000), false, &*open).map_err(|(t, _)| format!("{t:?}")).expect("a reading");
        let ranges: Vec<String> = asked.lock().unwrap().iter().filter(|l| l.contains(" eth_getLogs ")).cloned().collect();
        for n in ["up-a", "up-b"] {
            assert_eq!(ranges.iter().filter(|l| **l == format!("{n} eth_getLogs 0..1000")).count(), 1, "{n}: the whole range, one question: {ranges:?}");
        }
        assert!(ranges.iter().filter(|l| l.starts_with(row)).all(|l| *l == format!("{row} eth_getLogs 0..1000")) && ranges.len() <= 3, "{ranges:?}");
        assert_eq!(r.sources, 2);
        for (silent, form) in [("down-c", "a node silent"), ("odd-c", "a node out of shape")] {
            let (eps, _, open) = chans(&[silent, "up-d", row], answer);
            let r = ask_with_open(&eps, "eth_blockNumber", &code, false, &*open).map_err(|(t, _)| format!("{t:?}")).expect("a reading");
            assert_eq!((r.sources, r.single_source), (2, false), "{form}: the row and the other bear it: {:?}", r.unanswered);
            assert!(r.unanswered.is_empty(), "{form}: a reading two places made names nobody (A10): {:?}", r.unanswered);
        }
        let (eps, _, open) = chans(&["up-e", row, "up-f"], answer);
        let urls: Vec<NodeAddr> = eps.iter().map(|e| e.url.clone()).collect();
        let handed: Vec<String> = each_node(&urls, &*open, |u, ep| format!("{} {}", u.for_transport(), ep.is_some()), |_| false).into_iter().flatten().collect();
        assert_eq!(handed, ["up-e true", format!("{row} true").as_str(), "up-f true"], "every node, in the table's order");
    }

    #[test]
    fn after_its_action_reads_a_receipt_a_head_stands_on_that_block() {
        eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
        const RECEIPT: &str = r#""result":{"blockHash":"0xaa","blockNumber":"0x12","status":"0x1","transactionHash":"0x01"}"#;
        fn ahead(m: &str, _: usize) -> String {
            match m {
                "eth_getTransactionReceipt" => RECEIPT.into(),
                _ => r#""result":"0x12""#.into(),
            }
        }
        fn catching(m: &str, n: usize) -> String {
            match m {
                "eth_getTransactionReceipt" => RECEIPT.into(),
                _ if n < 2 => r#""result":"0x11""#.into(),
                _ => r#""result":"0x12""#.into(),
            }
        }
        fn stuck(m: &str, _: usize) -> String {
            match m {
                "eth_getTransactionReceipt" => RECEIPT.into(),
                _ => r#""result":"0x11""#.into(),
            }
        }
        let receipt = Value::Arr(vec![Value::Str(format!("0x{}", "01".repeat(32)))]);
        let (a, _) = node(ahead);
        let (b, asked_b) = node(catching);
        let eps = vec![Endpoint::at(31337, NodeAddr::new(a.clone())), Endpoint::at(31337, NodeAddr::new(b))];
        {
            let _action = eravon_chain::action::open(None);
            assert_eq!(head_block(&eps, 31337).expect("the head").0, 0x11, "no receipt read: the lowest head");
        }
        {
            let _action = eravon_chain::action::open(None);
            ask(&eps, "eth_getTransactionReceipt", &receipt).expect("the receipt");
            assert_eq!(head_block(&eps, 31337).expect("the head").0, 0x12, "the head stands on the receipt's block");
            assert_eq!(times(&asked_b, "eth_blockNumber"), 3, "the slower node asked again until it reached it");
        }
        let (c, _) = node(stuck);
        let eps = vec![Endpoint::at(31337, NodeAddr::new(a)), Endpoint::at(31337, NodeAddr::new(c))];
        let _action = eravon_chain::action::open(None);
        ask(&eps, "eth_getTransactionReceipt", &receipt).expect("the receipt");
        match head_block(&eps, 31337) {
            Err(t) => assert!(matches!(t.why, Why::Behind { lag: 1, high: 0x12, .. }), "named as behind: {t:?}"),
            Ok(h) => panic!("never read before the receipt: {h:?}"),
        }
    }

    #[test]
    fn after_a_receipt_two_nodes_at_its_block_are_read_and_the_one_behind_is_passed_over() {
        eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
        const RECEIPT: &str = r#""result":{"blockHash":"0xaa","blockNumber":"0x12","status":"0x1","transactionHash":"0x01"}"#;
        fn ahead(m: &str, _: usize) -> String {
            match m {
                "eth_getTransactionReceipt" => RECEIPT.into(),
                _ => r#""result":"0x12""#.into(),
            }
        }
        fn stuck(m: &str, _: usize) -> String {
            match m {
                "eth_getTransactionReceipt" => r#""result":null"#.into(),
                _ => r#""result":"0x11""#.into(),
            }
        }
        let receipt = Value::Arr(vec![Value::Str(format!("0x{}", "01".repeat(32)))]);
        let (a, _) = node(ahead);
        let (b, _) = node(ahead);
        let (c, asked_c) = node(stuck);
        let eps = vec![Endpoint::at(31337, NodeAddr::new(a.clone())), Endpoint::at(31337, NodeAddr::new(b)), Endpoint::at(31337, NodeAddr::new(c.clone()))];
        let _action = eravon_chain::action::open(None);
        let _ = ask(&eps, "eth_getTransactionReceipt", &receipt);
        asked_c.lock().unwrap().clear();
        let balance = Value::Arr(vec![Value::Str("0x00".into()), Value::Str("0x12".into())]);
        let (head, n, read) = one_read(|| {
            let (h, n) = head_block(&eps, 31337).expect("two nodes reached the block");
            (h, n, ask(&eps, "eth_getBalance", &balance).map(|r| r.value))
        });
        assert_eq!((head, n), (0x12, 2), "the head of the two that reached it");
        assert!(read.is_ok(), "read from the two: {read:?}");
        assert!(times(&asked_c, "eth_blockNumber") <= 1, "the node behind asked its head at most once, never waited for");
        assert!(times(&asked_c, "eth_getBalance") <= 1, "at most asked once beside the two, never waited for");
        let (d, _) = node(stuck);
        let two = vec![Endpoint::at(31337, NodeAddr::new(a)), Endpoint::at(31337, NodeAddr::new(d))];
        match head_block(&two, 31337) {
            Err(t) => assert!(matches!(t.why, Why::Behind { lag: 1, high: 0x12, .. }), "named as behind: {t:?}"),
            Ok(h) => panic!("a table of two waits for both: {h:?}"),
        }
    }

    #[test]
    fn a_receipt_of_another_chain_is_no_floor_of_this_one() {
        eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
        fn high(m: &str, _: usize) -> String {
            match m {
                "eth_getTransactionReceipt" => r#""result":{"blockHash":"0xaa","blockNumber":"0x1000","status":"0x1","transactionHash":"0x01"}"#.into(),
                "eth_chainId" => r#""result":"0xa4b1""#.into(),
                _ => r#""result":"0x1000""#.into(),
            }
        }
        fn low(m: &str, _: usize) -> String {
            match m {
                "eth_getTransactionReceipt" => r#""result":{"blockHash":"0xbb","blockNumber":"0x12","status":"0x1","transactionHash":"0x02"}"#.into(),
                "eth_chainId" => r#""result":"0x7a69""#.into(),
                _ => r#""result":"0x11""#.into(),
            }
        }
        let receipt = |b: u8| Value::Arr(vec![Value::Str(format!("0x{}", format!("{b:02x}").repeat(32)))]);
        let empty = Value::Arr(Vec::new());
        let (h, _) = node(high);
        let (l, asked_l) = node(low);
        let higher = vec![Endpoint::at(42161, NodeAddr::new(h))];
        let lower = vec![Endpoint::at(31337, NodeAddr::new(l))];
        let _action = eravon_chain::action::open(None);
        ask(&higher, "eth_getTransactionReceipt", &receipt(1)).expect("the other chain's receipt");
        assert_eq!(head_block(&lower, 31337).expect("this chain's head").0, 0x11, "never held under the other chain's block");
        assert_eq!(times(&asked_l, "eth_blockNumber"), 1, "asked once, nothing waited for");
        ask(&lower, "eth_chainId", &empty).expect("the chain id");
        ask(&lower, "eth_blockNumber", &empty).expect("the head");
        let ahead = std::sync::atomic::AtomicBool::new(false);
        ask_ahead(&lower, 31337, &|_, _, _| {
            ahead.store(true, std::sync::atomic::Ordering::SeqCst);
            Vec::new()
        });
        assert!(ahead.load(std::sync::atomic::Ordering::SeqCst), "what stands on this chain's head asked ahead");
        ask(&lower, "eth_getTransactionReceipt", &receipt(2)).expect("this chain's receipt");
        match head_block(&lower, 31337) {
            Err(t) => assert!(matches!(t.why, Why::Behind { lag: 1, high: 0x12, .. }), "this chain's receipt holds it: {t:?}"),
            Ok(h) => panic!("never read before this chain's receipt: {h:?}"),
        }
    }

    #[test]
    fn a_rest_row_whose_logs_stop_short_is_not_yet_there() {
        eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
        let row = "https://lane.example.test/api#rest=1";
        let answer = |name: &str, _: &str, from: u64, to: u64| if name.contains("#rest=") || name.starts_with("short") { Ok(logs_at(from, to.min(998))) } else { Ok(logs_at(from, to)) };
        let (eps, _, open) = chans(&["full-a", "full-b", row], answer);
        let r = ask_with_open(&eps, "eth_getLogs", &logs_over(0, 1000), false, &*open).map_err(|(t, _)| format!("{t:?}")).expect("a reading");
        assert_eq!((r.sources, r.single_source), (2, false), "{:?}", r.unanswered);
        assert!(r.unanswered.is_empty(), "two places alike make the reading and name nobody, the row not yet there among them (A10): {:?}", r.unanswered);
        let (eps, _, open) = chans(&["full-c", "short-d"], answer);
        let r = ask_with_open(&eps, "eth_getLogs", &logs_over(0, 1000), false, &*open).map_err(|(t, _)| format!("{t:?}"));
        assert!(r.is_err_and(|t| t.contains("Disagree")), "a JSON-RPC node short of the other still disagrees");
    }

    #[test]
    fn two_alike_are_enough_and_the_third_is_not_waited_for() {
        eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
        let answer = |name: &str, _: &str, _: u64, _: u64| {
            if name.starts_with("down") {
                Err(rpc::Trouble::Transport(rpc::Broke::Short))
            } else if name.starts_with("other") || name.contains("other") {
                Ok("\"0x6\"".to_string())
            } else {
                Ok("\"0x5\"".to_string())
            }
        };
        let q = Value::Arr(Vec::new());
        let (eps, _, open) = chans(&["up-a", "up-b", "hung-c"], answer);
        let t = std::time::Instant::now();
        let r = ask_with_open(&eps, "eth_getBalance", &q, false, &*open).map_err(|(t, _)| format!("{t:?}")).expect("a reading");
        assert!(t.elapsed() < std::time::Duration::from_millis(1500), "waited for the hung one: {:?}", t.elapsed());
        assert_eq!((r.sources, r.single_source, r.unanswered.len(), r.differed.len()), (2, false, 0, 0));
        let (eps, _, open) = chans(&["up-d", "other-e", "late-f"], answer);
        let r = ask_with_open(&eps, "eth_getBalance", &q, false, &*open).map_err(|(t, _)| format!("{t:?}")).expect("a reading");
        assert_eq!((r.value.clone(), r.sources, r.differed.len()), (Value::Str("0x5".into()), 2, 0));
        let (eps, _, open) = chans(&["up-g", "down-h", "down-i"], answer);
        let r = ask_with_open(&eps, "eth_getBalance", &q, false, &*open).map_err(|(t, _)| format!("{t:?}")).expect("a reading");
        assert_eq!((r.sources, r.single_source, r.unanswered.len()), (1, true, 2));
        let (eps, _, open) = chans(&["up-j"], answer);
        let r = ask_with_open(&eps, "eth_getBalance", &q, false, &*open).map_err(|(t, _)| format!("{t:?}")).expect("a reading");
        assert_eq!((r.sources, r.single_source), (1, true));
    }

    #[test]
    fn a_block_one_place_gives_beside_a_null_is_waited_for_until_two_alike_or_the_null_has_passed_it() {
        eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
        static CAUGHT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let answer = |name: &str, method: &str, _: u64, _: u64| {
            let block = "{\"baseFeePerGas\":\"0x7\"}";
            match (method, name) {
                (_, n) if n.starts_with("busy") => Err(rpc::Trouble::Node("{\"code\":-32005,\"message\":\"Too Many Requests\"}".into())),
                ("eth_getBlockByNumber", n) if n.starts_with("has") => Ok(block.to_string()),
                ("eth_getBlockByNumber", n) if n.starts_with("catching") => Ok(if CAUGHT.fetch_add(1, std::sync::atomic::Ordering::SeqCst) < 2 { "null" } else { block }.to_string()),
                ("eth_getBlockByNumber", _) => Ok("null".to_string()),
                ("eth_blockNumber", n) if n.starts_with("passed") => Ok("\"0x40\"".to_string()),
                ("eth_blockNumber", _) => Ok("\"0xf\"".to_string()),
                _ => Ok("null".to_string()),
            }
        };
        let pinned = Value::Arr(vec![Value::Str("0x10".into()), Value::Bool(false)]);
        let read = |names: &[&str]| {
            let (eps, asked, open) = chans(names, answer);
            let r = ask_with_open(&eps, "eth_getBlockByNumber", &pinned, false, &*open).map_err(|(t, _)| format!("{t:?}")).expect("a reading");
            let blocks = who_asked(&asked, "eth_getBlockByNumber").iter().filter(|n| *n == names[1]).count();
            (r.sources, r.single_source, blocks)
        };
        assert_eq!(read(&["has-a", "catching-b", "busy-c"]), (2, false, 3), "caught up while waited for: two alike");
        assert_eq!(read(&["has-d", "behind-e", "busy-f"]), (1, true, 1 + eravon_chain::judge::NOT_YET_PAUSES.len()), "still behind after the waits: one place, as before");
        assert_eq!(read(&["has-g", "passed-h", "busy-i"]), (1, true, 2), "past the block and without it: asked once more at once, not waited for");
        assert_eq!(read(&["has-j", "has-k", "behind-l"]), (2, false, 1), "two alike: at once");
        eravon_chain::patience::set_waits(None);
    }

    #[test]
    fn a_late_answer_is_kept_nowhere() {
        eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
        let answer = |name: &str, _: &str, _: u64, _: u64| if name.contains("late") { Ok("{\"baseFeePerGas\":\"0x9\"}".to_string()) } else { Ok("{\"baseFeePerGas\":\"0x7\"}".to_string()) };
        let (eps, _, open) = chans(&["up-k", "up-l", "late-m"], answer);
        let remembered = move |u: &str| open(u).map(|ep| eravon_chain::endpoints::remembering(u, ep));
        let _action = eravon_chain::action::open(None);
        let pinned = Value::Arr(vec![Value::Str("0x10".into()), Value::Bool(false)]);
        let r = ask_with_open(&eps, "eth_getBlockByNumber", &pinned, false, &remembered).map_err(|(t, _)| format!("{t:?}")).expect("a reading");
        assert_eq!(r.value.member("baseFeePerGas"), Some(&Value::Str("0x7".into())));
        std::thread::sleep(std::time::Duration::from_millis(600));
        assert!(eravon_chain::action::answered("up-k", "eth_getBlockByNumber", &pinned).is_some(), "an answer that counted is the action's");
        assert!(eravon_chain::action::answered("late-m", "eth_getBlockByNumber", &pinned).is_none(), "the late answer was kept");
    }
}
