use super::vault_open;

use app::action::{apply, Action, Applied};

use eravon_chain::fault::Fault;

use eravon_chain::said::Refusal;

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use std::time::Duration;

pub(super) enum Reply {
    Result(String),
    Error(String),
    Close,
}

#[derive(Clone)]
pub(super) struct Node {
    pub(super) url: String,
    log: Arc<Mutex<Vec<String>>>,
}

pub(super) fn node(answer: impl Fn(&str, &str) -> Reply + Send + Sync + 'static) -> Node {
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", l.local_addr().expect("addr"));
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let (kept, answer) = (log.clone(), Arc::new(answer));
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let (log, answer) = (kept.clone(), answer.clone());
            std::thread::spawn(move || {
                let mut s = s;
                let mut raw = Vec::new();
                let mut buf = [0u8; 8192];
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
                let one = |body: &str| -> Option<String> {
                    log.lock().expect("log").push(body.to_string());
                    let method = method_of(body);
                    let id = body.split("\"id\":").nth(1).and_then(|r| r.split([',', '}']).next()).unwrap_or("1").trim().to_string();
                    match answer(&method, body) {
                        Reply::Result(v) => Some(format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{v}}}")),
                        Reply::Error(e) => Some(format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"error\":{e}}}")),
                        Reply::Close => None,
                    }
                };
                let reply = match eravon::json::parse(body.as_bytes()) {
                    Ok(eravon::json::Value::Arr(asks)) => {
                        let mut parts = Vec::new();
                        for a in &asks {
                            let Some(r) = one(&String::from_utf8_lossy(&eravon::json::canon_bytes(a))) else { return };
                            parts.push(r);
                        }
                        format!("[{}]", parts.join(","))
                    }
                    _ => match one(&body) {
                        Some(r) => r,
                        None => return,
                    },
                };
                let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}", reply.len()).as_bytes());
            });
        }
    });
    Node { url, log }
}

pub(super) fn method_of(body: &str) -> String {
    body.split("\"method\":\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("").to_string()
}

pub(super) fn first_param(body: &str) -> String {
    body.split("\"params\":[\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("").to_string()
}

pub(super) fn hash_of(raw_hex: &str) -> String {
    eravon::hexfmt::encode(&eravon::cryptox::keccak256(&eravon::hexfmt::decode(raw_hex).expect("raw hex")))
}

impl Node {
    pub(super) fn asked(&self, method: &str) -> Vec<String> {
        self.log.lock().expect("log").iter().filter(|b| method_of(b) == method).cloned().collect()
    }

    pub(super) fn raws(&self) -> Vec<String> {
        self.asked("eth_sendRawTransaction").iter().map(|b| first_param(b)).collect()
    }
}

pub(super) const PLENTY: &str = "\"0xde0b6b3a7640000\"";

pub(super) fn chain(method: &str, body: &str, balance: &str) -> Reply {
    Reply::Result(match method {
        "eth_chainId" => "\"0x7a69\"".into(),
        "eth_blockNumber" => "\"0x40\"".into(),
        "eth_getBlockByNumber" => "{\"baseFeePerGas\":\"0x3b9aca00\",\"number\":\"0x40\",\"timestamp\":\"0x64\"}".into(),
        "eth_feeHistory" => "{\"oldestBlock\":\"0x3e\",\"gasUsedRatio\":[0.5,0.25,0.125],\"reward\":[[\"0x5f5e100\"],[\"0x5f5e100\"],[\"0x5f5e100\"]]}".into(),
        "eth_estimateGas" => "\"0xc350\"".into(),
        "eth_getBalance" => balance.into(),
        "eth_getLogs" => "[]".into(),
        "eth_getTransactionCount" => "\"0x5\"".into(),
        "eth_sendRawTransaction" => format!("\"{}\"", hash_of(&first_param(body))),
        _ => "null".into(),
    })
}

fn known_tx(body: &str) -> Reply {
    Reply::Result(format!("{{\"hash\":\"{}\",\"blockNumber\":null,\"from\":\"0x{}\",\"input\":\"0x\"}}", first_param(body), "11".repeat(20)))
}

pub(super) fn homes() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("eravon-sg-send-{}", std::process::id()))
}

pub(super) fn tidy() {
    let _ = std::fs::remove_dir_all(homes());
}

fn signed(form: &str) -> String {
    let mut raw = vec![0x02];
    raw.extend_from_slice(form.as_bytes());
    eravon::hexfmt::encode(&raw)
}

fn send(raw: &str, nodes: &[&Node]) -> Result<eravon_chain::sending::Sent, Fault> {
    eravon_chain::patience::set_waits(Some(Duration::ZERO));
    let t = eravon_chain::transport::Http::with_timeout(Duration::from_secs(10));
    let table = eravon_chain::table::Table::new(nodes.iter().map(|n| n.url.clone()).collect()).expect("a table");
    let bytes = eravon::hexfmt::decode(raw).expect("raw hex");
    eravon_chain::sending::send_everywhere(&t, &table, &bytes, &hash_of(raw))
}

fn at(n: &Node) -> eravon_chain::seam::NodeAt {
    eravon_chain::seam::NodeAt::new(n.url.clone())
}

pub(super) fn nothing_broadcast(form: &str, nodes: &[&Node]) {
    for n in nodes {
        assert!(n.asked("eth_sendRawTransaction").is_empty(), "{form}: nothing broadcast at {}", n.url);
        assert!(n.asked("eth_getTransactionCount").is_empty(), "{form}: no nonce asked at {}", n.url);
    }
}

fn direct(nodes: &[&Node]) -> eravon_cli::fault::R<eravon_cli::send::Delivered> {
    eravon_chain::patience::set_waits(Some(Duration::ZERO));
    eravon_store::writer::machine_at(Some(homes().join("machine")));
    let w = eravon_cli::shared::Wire {
        t: eravon_chain::transport::Auto::web_roots().expect("a transport"),
        table: eravon_chain::table::Table::new(nodes.iter().map(|n| n.url.clone()).collect()).expect("a table"),
        source: eravon_cli::home::Source::Inline,
        chain_id: 31337,
        logic: String::new(),
        registries: Vec::new(),
    };
    let key = [0x42u8; 32];
    let who = app::key::Secret::take(key).expect("a key").address().expect("its address").hex();
    let call = eravon_standard::call::Call { to: format!("0x{}", "00".repeat(20)), data: Vec::new() };
    let o = eravon_cli::send::Opts { gas_limit: None, fees: None, wait: Some(0), nonce: None };
    eravon_cli::send::deliver_key(&|| Ok(key), &o, &w, &who, &call, "0")
}

#[test]
fn a_test_run_alone_gets_its_own_process_machine_and_home() {
    if super::alone_in(module_path!(), "a_test_run_alone_gets_its_own_process_machine_and_home") {
        return;
    }
    let marker = std::env::var("ERAVON_CHECKS_ALONE").unwrap_or_default();
    assert!(marker.ends_with("::a_test_run_alone_gets_its_own_process_machine_and_home") && !marker.starts_with("::"), "{marker}");
    eravon_store::writer::machine_at(Some(homes().join("machine")));
    vault_open();
    let machine = app::home::machine_dir().expect("the machine directory");
    assert!(machine.display().to_string().ends_with(&format!("zk-checks-machine-{}", std::process::id())), "{}", machine.display());
    let dir = homes().join("alone");
    let _ = std::fs::remove_dir_all(&dir);
    let ctx = eravon_ui::egui::Context::default();
    let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    super::seated(&mut shell, &dir.join("home"));
    answers!(apply(&mut shell, Action::OpenHome { root: dir.join("home").display().to_string() }), Applied::Homed { .. }, "open a home in a child alone: {:?}", shell.faults);
    drop(shell);
    tidy();
}

#[test]
fn the_balance_gate_refuses_by_name_and_nothing_is_broadcast() {
    if super::alone_in(module_path!(), "the_balance_gate_refuses_by_name_and_nothing_is_broadcast") {
        return;
    }
    let with = |balance: &'static str| node(move |m, b| chain(m, b, balance));
    for form in ["short", "notAQuantity", "null", "refused", "closed", "differ", "enough"] {
        let (nodes, want): (Vec<Node>, Option<&str>) = match form {
            "short" => (vec![with("\"0x1\"")], Some("E_V_PRECHECK")),
            "notAQuantity" => (vec![with("\"0xzz\"")], Some("E_CHAIN_ENDPOINT")),
            "null" => (vec![with("null")], Some("E_CHAIN_ENDPOINT")),
            "refused" => (
                vec![node(|m, b| match m {
                    "eth_getBalance" => Reply::Error("{\"code\":-32000,\"message\":\"header not found\"}".into()),
                    _ => chain(m, b, PLENTY),
                })],
                None,
            ),
            "closed" => (
                vec![node(|m, b| match m {
                    "eth_getBalance" => Reply::Close,
                    _ => chain(m, b, PLENTY),
                })],
                None,
            ),
            "differ" => (vec![with(PLENTY), with("\"0xde0b6b3a7640001\"")], Some("E_CHAIN_DISAGREE")),
            "enough" => (vec![with(PLENTY)], None),
            other => panic!("no form {other}"),
        };
        let refs: Vec<&Node> = nodes.iter().collect();
        let got = direct(&refs);
        if form == "enough" {
            let d = got.unwrap_or_else(|f| panic!("{form}: a balance that covers it is sent: {} {}", f.name(), f.detail()));
            for n in &refs {
                assert_eq!(n.raws().len(), 1, "{form}: handed once at {}", n.url);
            }
            assert_eq!(d.tx_hash, hash_of(&refs[0].raws()[0]), "{form}: the hash of the bytes handed over");
            continue;
        }
        let Err(f) = got else { panic!("{form}: refused before sending") };
        let token = f.extra().into_iter().find(|(k, _)| *k == "downToken").and_then(|(_, v)| match v {
            eravon_literals::json::Out::S(t) => Some(t),
            _ => None,
        });
        let named = token.clone().unwrap_or_else(|| f.name().to_string());
        match want {
            Some(k) => assert_eq!(named, k, "{form}: {} {}", f.name(), f.detail()),
            None => assert!(named != "E_V_PRECHECK" && token.is_some(), "{form}: an unread balance is not guessed: {} {}", f.name(), f.detail()),
        }
        if form == "short" {
            assert!(f.detail().contains("余额 1 wei") && f.extra().iter().any(|(k, v)| *k == "gate" && matches!(v, eravon_literals::json::Out::S(g) if g == "fee")), "{form}: both numbers, the gate by name: {}", f.detail());
        }
        nothing_broadcast(form, &refs);
    }
}

#[test]
fn one_signing_the_same_bytes_to_every_node_and_the_echo_must_match() {
    if super::alone_in(module_path!(), "one_signing_the_same_bytes_to_every_node_and_the_echo_must_match") {
        return;
    }
    let wrong_echo = || {
        node(|m, b| match m {
            "eth_sendRawTransaction" => Reply::Result(format!("\"0x{}\"", "ab".repeat(32))),
            _ => chain(m, b, PLENTY),
        })
    };
    let form = "third";
    let silent = node(|m, b| match m {
        "eth_sendRawTransaction" => Reply::Close,
        _ => chain(m, b, PLENTY),
    });
    let wrong = wrong_echo();
    let right = node(|m, b| chain(m, b, PLENTY));
    let refs = [&silent, &wrong, &right];
    let raw = signed(form);
    let got = send(&raw, &refs);
    for n in refs {
        assert_eq!(n.raws(), vec![raw.clone()], "{form}: handed once, the same bytes, at {}", n.url);
    }
    match got {
        Ok(sent) => {
            assert_eq!(sent.endpoint, at(&right), "{form}: the node whose echo matched took it");
            assert_eq!(sent.tx_hash, hash_of(&raw), "{form}: the hash of the bytes handed over");
        }
        other => panic!("{form}: sent: {other:?}"),
    }
    let form = "wrongEchoOnly";
    let wrong = wrong_echo();
    let raw = signed(form);
    let got = send(&raw, &[&wrong]);
    assert_eq!(wrong.raws().len(), 1, "{form}: handed once");
    let f = got.expect_err("a wrong echo is not received");
    assert!(matches!(f, Fault::Echo(_)), "{form}: refused by name: {f:?}");
    let asked: Vec<String> = wrong.asked("eth_getTransactionByHash").iter().map(|b| first_param(b)).collect();
    assert!(asked.iter().all(|h| *h == hash_of(&raw)), "{form}: asked by the signed hash: {asked:?}");
}

#[test]
fn already_taken_answers_are_asked_by_hash_and_no_new_nonce_is_signed() {
    if super::alone_in(module_path!(), "already_taken_answers_are_asked_by_hash_and_no_new_nonce_is_signed") {
        return;
    }
    for (form, said, refused) in [
        ("pending", (|| Reply::Error("{\"code\":-32000,\"message\":\"already known\"}".into())) as fn() -> Reply, Some(Refusal::Pending)),
        ("nonceUsed", || Reply::Error("{\"code\":-32000,\"message\":\"nonce too low\"}".into()), Some(Refusal::NonceUsed)),
        ("noAnswer", || Reply::Close, None),
    ] {
        for known in [true, false] {
            let form = format!("{form}{}", if known { "Known" } else { "Unknown" });
            let n = node(move |m, b| match m {
                "eth_sendRawTransaction" => said(),
                "eth_getTransactionByHash" if known => known_tx(b),
                _ => chain(m, b, PLENTY),
            });
            let raw = signed(&form);
            let got = send(&raw, &[&n]);
            assert_eq!(n.raws(), vec![raw.clone()], "{form}: handed over once");
            assert!(n.asked("eth_getTransactionCount").is_empty(), "{form}: no nonce asked");
            let hash = hash_of(&raw);
            let asked: Vec<String> = n.asked("eth_getTransactionByHash").iter().map(|b| first_param(b)).collect();
            assert!(!asked.is_empty(), "{form}: asked about by hash");
            assert!(asked.iter().all(|h| *h == hash), "{form}: by the hash signed: {asked:?} against {hash}");
            if known {
                match got {
                    Ok(sent) => assert_eq!((sent.tx_hash, sent.endpoint), (hash, at(&n)), "{form}: taken"),
                    other => panic!("{form}: a transaction the node knows was taken: {other:?}"),
                }
            } else {
                let f = got.expect_err("not known: refused");
                match refused.clone() {
                    Some(r) => assert_eq!(eravon_chain::fault::refusal(&f), Some(r), "{form}: {f:?}"),
                    None => assert!(matches!(f, Fault::Transport(_)), "{form}: {f:?}"),
                }
            }
        }
    }
}
