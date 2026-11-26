use eravon_chain::endpoints::Table;
use eravon_chain::seam::{Upto, Url};
use eravon_chain::transport::{Answer, Transport};
use eravon_literals::constants as k7;
use eravon_literals::json::In;
use eravon_scan::scan::Side;

const HEAD: u64 = 500;

#[derive(Clone)]
struct Chain {
    logs: Vec<String>,
    standard: Vec<String>,
}

impl Chain {
    fn answer_one(&self, _url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let req = eravon_literals::json::read(body).expect("问是 JSON");
        let method = req.get("method").and_then(In::as_str).unwrap_or_default().to_string();
        let id = match req.get("id") {
            Some(In::Int(n)) => n.to_string(),
            Some(In::Str(s)) => format!("\"{s}\""),
            _ => "1".into(),
        };
        let params = req.get("params").and_then(In::as_arr).map(<[In]>::to_vec).unwrap_or_default();
        let result = match method.as_str() {
            "eth_chainId" => format!("\"0x{:x}\"", k7::active_standard().chain_id),
            "eth_blockNumber" => format!("\"0x{HEAD:x}\""),
            "eth_getCode" => {
                let addr = params[0].as_str().expect("地址").to_ascii_lowercase();
                if self.standard.contains(&addr) {
                    format!("\"{}\"", eravon_standard::order::hex_of(&eravon_terms::derive::runtime(k7::active_standard().logic)))
                } else {
                    "\"0x6000600055\"".into()
                }
            }
            "eth_getLogs" => match params[0].get("address") {
                Some(In::Arr(a)) if a.is_empty() => format!("[{}]", self.logs.join(",")),
                Some(_) => "[]".into(),
                None => format!("[{}]", self.logs.join(",")),
            },
            _ => "null".into(),
        };
        Ok(Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
    }
}

impl Transport for Chain {
    fn post(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        eravon_chain::ask::each_question(body, |b| self.answer_one(url, b))
    }
}

fn addr(b: u8) -> String {
    format!("0x{}", format!("{b:02x}").repeat(20))
}

fn topic_of(a: &str) -> String {
    format!("\"0x{}{}\"", "0".repeat(24), a.trim_start_matches("0x"))
}

fn opened(emitter: &str, topics: &[String], block: u64) -> String {
    let t0 = format!("\"{}\"", eravon_standard::order::hex_of(&eravon_standard::sol::OPENED.topic0()));
    let all: Vec<String> = std::iter::once(t0).chain(topics.iter().cloned()).collect();
    format!(
        r#"{{"address":"{emitter}","topics":[{}],"data":"0x","blockNumber":"0x{block:x}","logIndex":"0x0","transactionHash":"0x{:064x}","blockHash":"0x{:064x}"}}"#,
        all.join(","),
        block,
        block + 1
    )
}

fn table(host: u8) -> Table {
    Table::new(vec![format!("http://127.0.9.{host}:1")]).expect("端点表")
}

#[test]
fn a_foreign_three_topic_opened_is_dropped_and_counted() {
    let (instance, forger, seller, buyer) = (addr(0x11), addr(0x22), addr(0xa1), addr(0xb1));
    let case = format!("\"0x{}\"", "c7".repeat(32));
    let t = Chain {
        logs: vec![
            opened(&forger, &[case.clone(), topic_of(&seller)], 10),
            opened(&instance, &[case, topic_of(&seller), topic_of(&buyer)], 20),
        ],
        standard: vec![instance.clone()],
    };
    let sw = eravon_scan::scan::by_key(&t, &table(1), k7::active_standard().logic, Side::Seller, &seller, 0, Upto::Block(HEAD))
        .expect("他码发出者的三 topic 日志不拒整趟");
    assert_eq!(sw.foreign, 1, "他码那一条丢掉并计数");
    assert_eq!(sw.cases.len(), 1, "真案照收");
    assert_eq!((sw.cases[0].instance.as_str(), sw.cases[0].buyer.as_str()), (instance.as_str(), buyer.as_str()));
}

#[test]
fn a_standard_emitter_out_of_form_is_refused() {
    let (instance, seller) = (addr(0x33), addr(0xa2));
    let case = format!("\"0x{}\"", "d8".repeat(32));
    let t = Chain { logs: vec![opened(&instance, &[case, topic_of(&seller)], 30)], standard: vec![instance.clone()] };
    let e = eravon_scan::scan::by_key(&t, &table(2), k7::active_standard().logic, Side::Seller, &seller, 0, Upto::Block(HEAD))
        .expect_err("标准码发出者的不合形日志是坏读数");
    assert_eq!(e.name(), "E_A3_FORM");
}
