use eravon_chain::endpoints::Table;
use eravon_chain::fault::Fault;
use eravon_chain::read::Agreement;
use eravon_chain::transport::Http;
use std::io::{Read, Write};

const TO: &str = "0x1111111111111111111111111111111111111111";

fn node(status: &'static str, body: String) -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", l.local_addr().expect("addr"));
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let body = body.clone();
            std::thread::spawn(move || {
                let mut s = s;
                let mut raw = Vec::new();
                let mut buf = [0u8; 4096];
                let req = loop {
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
                let id = req.split("\"id\":").nth(1).and_then(|r| r.split([',', '}']).next()).unwrap_or("1").trim().to_string();
                let method = req.split("\"method\":\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("").to_string();
                let (status, body) = match method.as_str() {
                    "eth_chainId" => ("200 OK", "{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":\"0x7a69\"}".to_string()),
                    "eth_blockNumber" => ("200 OK", "{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":\"0x100\"}".to_string()),
                    _ => (status, body),
                };
                if status.is_empty() {
                    return;
                }
                let body = body.replace("{id}", &id);
                let _ = s.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
            });
        }
    });
    url
}

fn erring(err: &str) -> String {
    node("200 OK", format!("{{\"jsonrpc\":\"2.0\",\"id\":{{id}},\"error\":{err}}}"))
}

fn member(f: &Fault) -> String {
    match eravon_chain::fault::refusal(f) {
        Some(r) => eravon_chain::fault::word(&r).to_string(),
        None => f.name().to_string(),
    }
}

fn refused(urls: &[String]) -> Fault {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let t = Http::with_timeout(std::time::Duration::from_secs(5));
    let table = match Table::new(urls.to_vec()) {
        Ok(t) => t,
        Err(f) => return f,
    };
    match eravon_chain::gas::estimate(&t, &table, None, TO, None, None) {
        Ok((n, _)) => panic!("answered: {n}"),
        Err(f) => f,
    }
}

fn estimate(urls: &[String]) -> (String, bool) {
    let f = refused(urls);
    (member(&f), matches!(f, Fault::Execution(_)))
}

fn one(url: String) -> (String, bool) {
    estimate(&[url])
}

fn is(m: &str, call: bool) -> (String, bool) {
    (m.to_string(), call)
}

#[test]
fn a_marker_about_the_call_is_the_calls() {
    assert_eq!(one(erring("{\"code\":-32000,\"message\":\"insufficient funds for gas\"}")), is("funds", true));
}

#[test]
fn a_marker_about_the_node_is_not_the_calls() {
    assert_eq!(one(erring("{\"code\":-32000,\"message\":\"unauthorized: bad api key\"}")), is("auth", false));
}

#[test]
fn a_revert_is_the_calls() {
    assert_eq!(one(erring("{\"code\":3,\"message\":\"execution reverted\"}")), is("reverted", true));
}

#[test]
fn another_code_with_a_reverts_data_is_the_calls_revert() {
    assert_eq!(one(erring("{\"code\":-32000,\"data\":\"0x\",\"message\":\"VM Exception\"}")), is("reverted", true));
}

#[test]
fn words_not_known_under_a_code_are_the_calls_and_said_as_the_nodes_refusal() {
    assert_eq!(one(erring("{\"code\":-32000,\"message\":\"gas required exceeds allowance\"}")), is("coded", true));
}

#[test]
fn an_internal_error_code_is_the_calls() {
    assert_eq!(one(erring("{\"code\":-32603,\"message\":\"internal error\"}")), is("coded", true));
}

#[test]
fn the_rate_limit_code_is_not_the_calls() {
    assert_eq!(one(erring("{\"code\":-32005,\"message\":\"x\"}")), is("rate-limited", false));
}

#[test]
fn a_code_written_as_text_is_not_the_calls() {
    assert_eq!(one(erring("{\"code\":\"-32000\",\"message\":\"x\"}")), is("other", false));
}

#[test]
fn a_message_without_a_code_is_not_the_calls() {
    assert_eq!(one(erring("{\"message\":\"upstream unavailable\"}")), is("other", false));
}

#[test]
fn an_error_that_is_bare_text_is_not_the_calls() {
    assert_eq!(one(erring("\"upstream unavailable\"")), is("other", false));
}

#[test]
fn an_answer_that_is_not_json_is_not_the_calls() {
    let f = refused(&[node("200 OK", "<html>bad gateway</html>".into())]);
    assert_eq!((member(&f), matches!(f, Fault::Execution(_))), is("E_CHAIN_ENDPOINT", false));
    assert!(f.detail().starts_with("malformed"), "an answer that is not JSON: {}", f.detail());
}

#[test]
fn a_server_error_page_is_not_the_calls() {
    assert_eq!(one(node("502 Bad Gateway", "<html>busy</html>".into())), is("other", false));
}

#[test]
fn a_rate_limit_page_is_not_the_calls() {
    assert_eq!(one(node("429 Too Many Requests", "<html>slow</html>".into())), is("rate-limited", false));
}

#[test]
fn a_forbidden_page_is_not_the_calls() {
    assert_eq!(one(node("403 Forbidden", "<html>no</html>".into())), is("auth", false));
}

#[test]
fn a_coded_error_under_a_server_error_status_is_the_calls() {
    let url = node("500 Internal Server Error", "{\"jsonrpc\":\"2.0\",\"id\":{id},\"error\":{\"code\":-32000,\"message\":\"x\"}}".into());
    assert_eq!(one(url), is("coded", true));
}

#[test]
fn a_closed_connection_is_not_the_calls() {
    assert_eq!(one(node("", String::new())).1, false);
}

#[test]
fn no_place_at_all_is_not_the_calls() {
    let f = refused(&[]);
    assert_eq!((member(&f), matches!(f, Fault::Execution(_))), is("E_CHAIN_USAGE", false));
    assert!(f.detail().starts_with("端点表是空的"), "no place at all: {}", f.detail());
}

#[test]
fn places_whose_addresses_do_not_read_are_not_the_calls() {
    let f = refused(&["wss://node.example".to_string()]);
    assert_eq!((member(&f), matches!(f, Fault::Execution(_))), is("E_CHAIN_USAGE", false));
    assert!(f.detail().starts_with("scheme"), "the address's shape: {}", f.detail());
}

#[test]
fn places_that_answer_differently_are_not_the_calls() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let a = node("200 OK", "{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":\"0x5208\"}".into());
    let b = node("200 OK", "{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":\"0x5209\"}".into());
    let t = Http::with_timeout(std::time::Duration::from_secs(5));
    let table = Table::new(vec![a, b]).expect("a table");
    let (n, r) = eravon_chain::gas::estimate(&t, &table, None, TO, None, None).map_err(|f| (member(&f), f.detail())).expect("differing estimates are read");
    assert_eq!((n, r.endpoints, r.agreement == Agreement::Single), (0x5209, 2, false));
}

#[test]
fn places_answering_no_quantity_are_the_chains_shape() {
    let a = node("200 OK", "{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":\"0xzz\"}".into());
    let b = node("200 OK", "{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":16}".into());
    assert_eq!(estimate(&[a, b]), is("E_CHAIN_ENDPOINT", false));
    let (a, b) = (node("200 OK", "{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":null}".into()), node("200 OK", "{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":null}".into()));
    assert_eq!(estimate(&[a, b]), is("E_CHAIN_ENDPOINT", false));
}

#[test]
fn several_refusing_places_are_read_off_the_first() {
    let coded = || erring("{\"code\":-32000,\"message\":\"x\"}");
    let text = || erring("{\"code\":\"-32000\",\"message\":\"x\"}");
    assert_eq!(estimate(&[coded(), text()]), is("coded", true));
    assert_eq!(estimate(&[text(), coded()]), is("coded", true));
    let limited = || erring("{\"code\":-32005,\"message\":\"x\"}");
    assert_eq!(estimate(&[limited(), coded()]), is("coded", true));
    assert_eq!(estimate(&[limited(), text()]), is("rate-limited", false));
}
