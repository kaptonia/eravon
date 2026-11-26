use crate::fault::Fault;
use crate::key::Address;
use eravon::json::Value;
use eravon_chain::rpc;
pub use eravon_send::Why;

pub use eravon_send::node::{forget_serving, refusal_of, route_of, said, wei, Endpoint, Got, Https, Kind, Layer, NodeAddr, Refusal, Said, Wire, FEE_FACTS, MAX_HEAD_LAG, SEND_BACKOFF, TIP_FACTS, TX_FACTS};

pub fn is_checked_logs(rel: &str) -> bool {
    rel.strip_prefix(&format!("{}/", eravon_send::checked::DIR)).is_some_and(eravon_send::checked::is_logs)
}

pub fn speak() {
    eravon_send::node::say_layers(|l, node, detail| crate::lang::filln(layer_key(l), &[node, detail]));
    eravon_chain::rest::keys_from(crate::machine::rest_keys_now);
}

pub const REST_KEYS_ENV: &str = "ERAVON_REST_KEYS";

pub fn sealed_room() -> Option<std::sync::Arc<eravon_send::checked::Sealed>> {
    crate::home::machine_dir().ok().map(|m| std::sync::Arc::new(eravon_send::checked::Sealed(m)))
}

pub fn settled_keeper() -> Option<std::sync::Arc<dyn eravon_chain::settled::Keep>> {
    let m = crate::home::machine_dir().ok()?;
    let logs = eravon_send::checked::Logs(eravon_send::checked::Sealed(m));
    Some(match key_now() {
        Some(key) => std::sync::Arc::new(KeptAt { key, logs }),
        None => std::sync::Arc::new(logs),
    })
}

fn key_now() -> Option<[u8; 32]> {
    crate::keybox::local_key().ok().map(|k| eravon::cryptox::sha256(k.bytes()))
}

struct KeptAt {
    key: [u8; 32],
    logs: eravon_send::checked::Logs<eravon_send::checked::Sealed>,
}

impl KeptAt {
    fn moved(&self) -> bool {
        key_now().is_some_and(|k| k != self.key)
    }
}

impl eravon_chain::settled::Keep for KeptAt {
    fn load(&self, part: &str) -> Option<Vec<u8>> {
        if self.moved() {
            return None;
        }
        eravon_chain::settled::Keep::load(&self.logs, part)
    }

    fn save(&self, part: &str, plain: &[u8]) -> Result<(), eravon_chain::settled::Unkept> {
        if self.moved() {
            return Err(eravon_chain::settled::Unkept::KeyChanged);
        }
        eravon_chain::settled::Keep::save(&self.logs, part, plain)
    }

    fn parts(&self) -> Vec<String> {
        eravon_chain::settled::Keep::parts(&self.logs)
    }

    fn hold(&self) -> Option<Box<dyn std::any::Any>> {
        eravon_chain::settled::Keep::hold(&self.logs)
    }

    fn public(&self) -> bool {
        eravon_chain::settled::Keep::public(&self.logs)
    }
}

static KEYS_FILE: std::sync::OnceLock<Option<eravon_chain::rest::KeysFileBad>> = std::sync::OnceLock::new();

pub fn keys_file() -> Option<eravon_chain::rest::KeysFileBad> {
    *KEYS_FILE.get_or_init(|| {
        let path = std::env::var_os(REST_KEYS_ENV).filter(|p| !p.is_empty())?;
        match eravon_chain::rest::keys_file(std::path::Path::new(&path)) {
            Ok(keys) => {
                eravon_chain::rest::hold_file_keys(keys);
                None
            }
            Err(why) => Some(why),
        }
    })
}

pub fn keys_file_said() -> Option<String> {
    use crate::lang::{fill1, Key};
    use eravon_chain::rest::KeysFileBad;
    let key = match keys_file()? {
        KeysFileBad::Unreadable => Key::NetRestFileUnreadable,
        KeysFileBad::NotUtf8 => Key::NetRestFileNotUtf8,
        KeysFileBad::Shape => Key::NetRestFileShape,
    };
    Some(fill1(key, REST_KEYS_ENV))
}

pub fn nodes_of(s: &crate::settings::Settings) -> Vec<Endpoint> {
    eravon_send::node::settings_nodes(&s.ground())
}

fn layer_key(l: Layer) -> crate::lang::Key {
    match l {
        Layer::Name => crate::lang::Key::Tail232,
        Layer::Connect => crate::lang::Key::Tail233,
        Layer::Handshake => crate::lang::Key::Tail234,
        Layer::Certificate => crate::lang::Key::Tail235,
    }
}

pub fn typed(spec: &str) -> Result<Endpoint, String> {
    Endpoint::typed(spec).map_err(|w| said_of(&w))
}

pub fn typed_all(specs: &str) -> Result<Vec<String>, crate::fault::Fault> {
    specs
        .split_whitespace()
        .map(|one| typed(one).map(|_| one.to_string()).map_err(|said| crate::fault::Fault::known(crate::fault::Known::SettingsShape, said)))
        .collect()
}

#[derive(Clone, Debug)]
pub struct Reading {
    pub value: Value,
    pub sources: usize,
    pub single_source: bool,
    pub unanswered: Vec<Why>,
    pub differed: Vec<Why>,
}

fn reading_of(r: eravon_send::node::Reading) -> Reading {
    Reading { value: r.value, sources: r.sources, single_source: r.single_source, unanswered: r.unanswered, differed: r.differed }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Unreachable,
    Tls,
    Timeout,
    Unserved,
    NoKey,
    Limited,
    Credentials,
    OtherChain,
    Unreadable,
    Refused,
    Differed,
    Behind,
    BadAddress,
    NoAnswer,
}

impl NodeKind {
    pub fn of_trouble(node: &str, t: &rpc::Trouble) -> NodeKind {
        use eravon_send::Code as C;
        use eravon_send::node::{Layer, Said, Wire};
        if let Said::Wire(Wire::Tls(Layer::Name | Layer::Connect)) = eravon_send::node::said(t) {
            return NodeKind::Unreachable;
        }
        match eravon_send::node::said_trouble(node, t).code {
            C::Unreachable | C::ProxyUnreachable => NodeKind::Unreachable,
            C::NodeTls => NodeKind::Tls,
            C::NodeTimeout => NodeKind::Timeout,
            C::ServiceUnserved | C::MethodMissing => NodeKind::Unserved,
            C::ServiceKeyMissing => NodeKind::NoKey,
            C::RateLimited => NodeKind::Limited,
            C::NodeAuth | C::ProxyCredentialsMissing | C::ProxyCredentialsRefused => NodeKind::Credentials,
            C::WrongChain => NodeKind::OtherChain,
            C::AnswerTooLong | C::AnswerNotJson | C::ChainShape => NodeKind::Unreadable,
            C::NodeRefused | C::ContractRefused => NodeKind::Refused,
            _ => NodeKind::NoAnswer,
        }
    }

    pub fn said(self) -> &'static str {
        use crate::lang::{t, Key};
        t(match self {
            NodeKind::Unreachable => Key::NetSaidUnreachable,
            NodeKind::Tls => Key::NetSaidTls,
            NodeKind::Timeout => Key::NetSaidTimeout,
            NodeKind::Unserved => Key::NetSaidUnserved,
            NodeKind::NoKey => Key::NetSaidNoKey,
            NodeKind::Limited => Key::NetSaidLimited,
            NodeKind::Credentials => Key::NetSaidCredentials,
            NodeKind::OtherChain => Key::NetSaidOtherChain,
            NodeKind::Unreadable => Key::NetSaidUnreadable,
            NodeKind::Refused => Key::NetSaidRefused,
            NodeKind::Differed => Key::NetSaidDiffered,
            NodeKind::Behind => Key::NetSaidBehind,
            NodeKind::BadAddress => Key::NetSaidBadAddress,
            NodeKind::NoAnswer => Key::NetSaidNoAnswer,
        })
    }
}

pub fn kinds_of(w: &Why) -> Vec<(String, NodeKind)> {
    match w {
        Why::Node { node, trouble } => vec![(node.clone(), NodeKind::of_trouble(node, trouble))],
        Why::Unreadable { node } | Why::NotHeight { node, .. } | Why::NoTime { node } => vec![(node.clone(), NodeKind::Unreadable)],
        Why::Differs { node } => vec![(node.clone(), NodeKind::Differed)],
        Why::Scheme { node } | Why::Port { node } | Why::NotAddress { node } | Why::NotSpec { node } => vec![(node.clone(), NodeKind::BadAddress)],
        Why::Behind { node, .. } => vec![(node.clone(), NodeKind::Behind)],
        Why::WrongChainNode { node, .. } => vec![(node.clone(), NodeKind::OtherChain)],
        Why::Each(all) => all.iter().flat_map(kinds_of).collect(),
        _ => Vec::new(),
    }
}

pub fn plain_of(ws: &[Why]) -> Vec<String> {
    ws.iter().flat_map(plain_lines).collect()
}

fn plain_lines(w: &Why) -> Vec<String> {
    match w {
        Why::Node { node, trouble } => {
            let kind = NodeKind::of_trouble(node, trouble);
            vec![match (kind, eravon_send::node_said(trouble)) {
                (NodeKind::Refused, Some(said)) => format!("{node}: {} \u{b7} {said}", kind.said()),
                _ => format!("{node}: {}", kind.said()),
            }]
        }
        Why::Each(all) => all.iter().flat_map(plain_lines).collect(),
        other => kinds_of(other).into_iter().map(|(node, kind)| format!("{node}: {}", kind.said())).collect(),
    }
}

pub fn said_of(w: &Why) -> String {
    use crate::lang::{filln, Key};
    match w {
        Why::Text(s) => s.clone(),
        Why::Each(all) => all.iter().map(said_of).collect::<Vec<_>>().join(" · "),
        Why::Node { node, trouble } => node_words(node, trouble),
        Why::Unreadable { node } => filln(Key::Tail094, &[node]),
        Why::Differs { node } => filln(Key::TailDiffers, &[node]),
        Why::Scheme { node } => filln(Key::Tail093, &[node]),
        Why::Port { node } => filln(Key::TailNodePort, &[node]),
        Why::NotAddress { node } => filln(Key::TailNodeAddress, &[node]),
        Why::NotSpec { node } => filln(Key::Tail008, &[node]),
        Why::NoHeadNode { chain } => filln(Key::Tail095, &[&chain.to_string()]),
        Why::NoTimeNode { chain } => filln(Key::Tail097, &[&chain.to_string()]),
        Why::NotHeight { node, text } => filln(Key::Tail096, &[node, text]),
        Why::Behind { node, lag, high } => filln(Key::TailNodeBehind, &[node, &lag.to_string(), &high.to_string()]),
        Why::NoTime { node } => filln(Key::Tail098, &[node]),
        Why::NotYetAt { has, not_yet } => filln(Key::TailNotYetAt, &[has, not_yet]),
        Why::Differ { count, sources } => filln(Key::Tail086, &[&count.to_string(), sources]),
        Why::WrongChainNode { node, serves, chain } => filln(Key::TailWrongChainNode, &[node, &serves.to_string(), &chain.to_string()]),
        Why::BalanceNotText(s) => filln(Key::Tail099, &[s]),
        Why::BalanceNotQty(s) => filln(Key::Tail100, &[s]),
        Why::Funds { need, have } => funds_tail(*need, *have),
    }
}

pub fn funds_tail(need: u128, have: u128) -> String {
    format!("need={need} have={have}")
}

pub fn trouble_said(url: &str, t: &rpc::Trouble) -> String {
    node_words(&eravon_net::sayable(url), t)
}

fn node_words(url: &str, t: &rpc::Trouble) -> String {
    let (what, inner) = match t {
        rpc::Trouble::Node(e) => (crate::lang::t(crate::lang::Key::Tail089), e.clone()),
        rpc::Trouble::Transport(b @ rpc::Broke::Status { .. }) => (crate::lang::t(crate::lang::Key::Tail089), b.evidence()),
        rpc::Trouble::Transport(b @ rpc::Broke::NoKey { .. }) => (crate::lang::t(crate::lang::Key::TailRestNoKey), b.evidence()),
        rpc::Trouble::Transport(b @ rpc::Broke::Unserved { .. }) => (crate::lang::t(crate::lang::Key::TailRestNotServed), b.evidence()),
        rpc::Trouble::Transport(rpc::Broke::LogsBehind { node }) => (crate::lang::t(crate::lang::Key::TailRestLogsBehind), node.clone()),
        rpc::Trouble::Transport(b @ rpc::Broke::HeadBehind { .. }) => (crate::lang::t(crate::lang::Key::TailHeadBehind), b.evidence()),
        rpc::Trouble::Transport(b) => (crate::lang::t(crate::lang::Key::Tail090), b.evidence()),
        rpc::Trouble::NotServed(k) => (crate::lang::t(crate::lang::Key::Tail091), k.clone()),
        rpc::Trouble::Contradiction(k) => (crate::lang::t(crate::lang::Key::Tail092), k.clone()),
    };
    let reason = match t {
        rpc::Trouble::Node(e) if refusal_of(e) == Refusal::Reverted => eravon_chain::said::reverted(e).map(|r| format!(" \u{b7} {}", r.evidence())).unwrap_or_default(),
        _ => String::new(),
    };
    if inner.contains(url) {
        format!("{what}:{inner}{reason}")
    } else {
        format!("{url}: {what}:{inner}{reason}")
    }
}

pub fn said_fault(url: &str, t: &rpc::Trouble) -> Fault {
    eravon_send::node::said_trouble(url, t).into()
}

pub fn address_said(url: &str) -> String {
    said_of(&eravon_send::node::address_said(url))
}

pub fn fee_source_line(fees: Option<&eravon_chain::send::Fees>) -> Option<crate::lang::Key> {
    match fees {
        Some(f) if f.from_chain => None,
        _ => Some(crate::lang::Key::U3FeeFallback),
    }
}

pub fn endpoint_at(url: &str) -> Option<Box<dyn rpc::Endpoint + Send>> {
    speak();
    eravon_send::node::endpoint_at(url)
}

pub fn https(url: &str) -> Option<Https> {
    speak();
    Https::new(url)
}

pub use eravon_send::node::OneRead;

pub fn ask(eps: &[Endpoint], method: &str, params: &Value) -> Result<Reading, Fault> {
    speak();
    eravon_send::node::ask(eps, method, params).map(reading_of).map_err(Fault::from)
}

pub fn ask_call(eps: &[Endpoint], method: &str, params: &Value) -> Result<Reading, (Fault, bool)> {
    speak();
    eravon_send::node::ask_call(eps, method, params).map(reading_of).map_err(|(t, call)| (Fault::from(t), call))
}

pub fn tx_known(urls: &[NodeAddr], chain: u64, hash: &[u8; 32]) -> bool {
    speak();
    eravon_send::node::tx_known(urls, chain, hash)
}

pub use eravon_send::node::AsHeld;

pub fn tx_as_held(urls: &[NodeAddr], chain: u64, tx: &str) -> AsHeld {
    speak();
    eravon_send::node::tx_as_held(urls, chain, tx)
}

pub fn head_block(eps: &[Endpoint], chain: u64) -> Result<(u64, usize), Fault> {
    speak();
    eravon_send::node::head_block(eps, chain).map_err(Fault::from)
}

pub fn head_time(eps: &[Endpoint], chain: u64) -> Result<(u64, u64, usize), Fault> {
    speak();
    eravon_send::node::head_time(eps, chain).map_err(Fault::from)
}

pub fn serving(eps: &[Endpoint], chain: u64) -> Result<(Vec<Endpoint>, Vec<String>), Fault> {
    speak();
    eravon_send::node::serving(eps, chain).map(|(kept, left)| (kept, left.iter().map(said_of).collect())).map_err(Fault::from)
}

#[cfg(test)]
pub fn disagree_trouble(words: &str) -> SendTrouble {
    eravon_send::Trouble::text(eravon_send::Code::Disagree, words.to_string())
}

pub use eravon_send::Trouble as SendTrouble;

pub fn balance(eps: &[Endpoint], chain: u64, who: &Address) -> Result<(u128, Reading), Fault> {
    crate::trace::mark(crate::feature::Feature::H2);
    speak();
    eravon_send::node::balance(eps, chain, &who.0).map(|(n, r)| (n, reading_of(r))).map_err(Fault::from)
}

pub fn fees(eps: &[Endpoint], chain: u64) -> (eravon_chain::send::Fees, Vec<String>) {
    let r = fee_reading(eps, chain);
    (r.fees, r.left)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeeReading {
    pub base: Option<u64>,
    pub fees: eravon_chain::send::Fees,
    pub left: Vec<String>,
}

pub fn fee_reading(eps: &[Endpoint], chain: u64) -> FeeReading {
    speak();
    let r = eravon_send::node::fee_reading(eps, chain);
    FeeReading { base: r.base, fees: r.fees, left: r.left.iter().map(said_of).collect() }
}

#[cfg(test)]
mod plain_tests {
    use super::*;

    #[test]
    fn a_node_is_said_by_its_kind_without_its_code() {
        let node = |n: &str, b: rpc::Broke| Why::Node { node: n.to_string(), trouble: rpc::Trouble::Transport(b) };
        let ws = vec![
            node("https://a", rpc::Broke::Said("TLS_HANDSHAKE https://a: received fatal alert: HandshakeFailure".into())),
            node("http://b", rpc::Broke::Said("http://b: Connection refused (os error 61)".into())),
            node("https://f", rpc::Broke::Said("TLS_CONNECT https://f: Connection refused (os error 61)".into())),
            node("http://c", rpc::Broke::Late { node: "http://c".into(), deadline: std::time::Duration::from_secs(1) }),
            Why::Differs { node: "http://e".into() },
            Why::Text("eth_getLogs".into()),
        ];
        let w = crate::lang::both_of(|| plain_of(&ws).join(" · "));
        assert_eq!(w.zh, "https://a: 建立安全连接失败 · http://b: 连不上 · https://f: 连不上 · http://c: 未在时限内应答 · http://e: 答得不同");
        assert_eq!(w.en, "https://a: Secure connection failed · http://b: Could not connect · https://f: Could not connect · http://c: No answer in time · http://e: Answered differently");
        let evidence = ws.iter().map(said_of).collect::<Vec<_>>().join(" · ");
        assert!(evidence.contains("TLS_HANDSHAKE") && evidence.contains("os error 61"), "the evidence keeps them: {evidence}");
    }
}
