use std::sync::Mutex;

use eravon_chain::endpoints::Table;
use eravon_chain::seam::Url;
use eravon_chain::transport::{Answer, Transport};
use eravon_literals::constants as k7;
use eravon_standard::{chain, fault::Fault};

#[derive(Clone)]
struct Nodes {
    code: fn(&str) -> Option<String>,
    seen: std::sync::Arc<Mutex<Vec<String>>>,
}

impl Nodes {
    fn answer_one(&self, _url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let b = String::from_utf8_lossy(body).to_string();
        self.seen.lock().expect("锁").push(b.clone());
        let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
        let method = b.split("\"method\":\"").nth(1).and_then(|x| x.split('"').next()).unwrap_or("");
        let addr = b.split("\"params\":[\"").nth(1).and_then(|x| x.split('"').next()).unwrap_or("").to_ascii_lowercase();
        let result = match method {
            "eth_chainId" => format!("\"0x{:x}\"", k7::active_standard().chain_id),
            "eth_blockNumber" => "\"0x40\"".to_string(),
            "eth_getBlockByNumber" => "{\"number\":\"0x40\",\"timestamp\":\"0x64\",\"baseFeePerGas\":\"0x1\"}".to_string(),
            "eth_getCode" => format!("\"{}\"", (self.code)(&addr).unwrap_or_else(|| "0x".into())),
            "eth_call" => format!("\"0x{}{}\"", "00".repeat(12), "22".repeat(20)),
            _ => "null".to_string(),
        };
        Ok(Answer { status: 200, location: None, body: format!("{{\"id\":{id},\"jsonrpc\":\"2.0\",\"result\":{result}}}").into_bytes() })
    }
}

impl Transport for Nodes {
    fn post(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        eravon_chain::ask::each_question(body, |b| self.answer_one(url, b))
    }
}

fn table() -> Table {
    Table::new(vec!["http://127.0.0.1:8001".into(), "http://127.0.0.1:8002".into()]).expect("两个端点")
}

fn hex(b: &[u8]) -> String {
    eravon_standard::order::hex_of(b)
}

const CURRENT_AT: &str = "0x1000000000000000000000000000000000000001";
const TRIAL_AT: &str = "0x2000000000000000000000000000000000000002";
const FOREIGN_AT: &str = "0x3000000000000000000000000000000000000003";

fn code_of(addr: &str) -> Option<String> {
    match addr {
        CURRENT_AT => Some(hex(&eravon_terms::derive::runtime(k7::active_standard().logic))),
        TRIAL_AT => Some(hex(&eravon_terms::derive::runtime(k7::TRIAL_SEPOLIA.logic))),
        FOREIGN_AT => Some("0x6000".into()),
        _ => None,
    }
}

#[test]
fn a_cases_generation_is_its_instances_code() {
    assert!(k7::select_chain(k7::TRIAL_SEPOLIA.chain_id), "Sepolia carries the trial row");
    let n = Nodes { code: code_of, seen: std::sync::Arc::new(Mutex::new(Vec::new())) };
    let t = table();
    let trial = Some(k7::TRIAL_SEPOLIA.logic);
    let current = Some(k7::active_standard().logic);
    let here = k7::active_standard().logic;
    assert!(!chain::case_generation(&n, &t, CURRENT_AT, here, trial).expect("current code").is_past());
    for listing in [None, current, trial] {
        let g = chain::case_generation(&n, &t, TRIAL_AT, here, listing).expect("trial code");
        assert!(g.is_past() && g.row().logic == k7::TRIAL_SEPOLIA.logic, "{listing:?}");
    }
    assert!(chain::code_generation(&n, &t, TRIAL_AT, here).expect("trial code").value.is_past());
    assert!(matches!(chain::case_generation(&n, &t, FOREIGN_AT, here, trial), Err(Fault::CodeHash { .. })));
    let unopened = "0x4000000000000000000000000000000000000004";
    assert!(chain::case_generation(&n, &t, unopened, here, trial).expect("no code").is_past(), "no code yet: the listing's");
    assert!(!chain::case_generation(&n, &t, unopened, here, None).expect("no code").is_past(), "no listing: the current row");
    assert!(!chain::case_generation(&n, &t, unopened, here, current).expect("no code").is_past());
}

#[test]
fn the_acting_key_is_the_books_except_on_a_past_case() {
    let n = Nodes { code: code_of, seen: std::sync::Arc::new(Mutex::new(Vec::new())) };
    let key = "0x1111111111111111111111111111111111111111";
    assert_eq!(chain::acting_key(&n, &table(), key, true).expect("raw"), key);
    assert!(n.seen.lock().expect("锁").is_empty(), "a past case asks no book");
    assert_eq!(chain::acting_key(&n, &table(), key, false).expect("book"), format!("0x{}", "22".repeat(20)));
}
