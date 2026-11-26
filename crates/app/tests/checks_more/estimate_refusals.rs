use super::vault_open;

use std::io::{Read, Write};

fn node(head: Option<&'static str>, estimate: &'static str) -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", l.local_addr().expect("addr"));
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
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
                let method = req.split("\"method\":\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("").to_string();
                let id = req.split("\"id\":").nth(1).and_then(|r| r.split([',', '}']).next()).unwrap_or("1").trim().to_string();
                let answer = |member: &str, v: &str| format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"{member}\":{v}}}");
                let body = match method.as_str() {
                    "eth_blockNumber" => match head {
                        Some(e) => answer("error", e),
                        None => answer("result", "\"0x40\""),
                    },
                    "eth_chainId" => answer("result", "\"0x7a69\""),
                    "eth_getBlockByNumber" => answer("result", "{\"baseFeePerGas\":\"0x3b9aca00\",\"number\":\"0x40\",\"timestamp\":\"0x64\"}"),
                    "eth_estimateGas" => answer("error", estimate),
                    _ => answer("result", "null"),
                };
                let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
            });
        }
    });
    url
}

#[derive(Debug, PartialEq, Eq)]
enum Whose {
    Call,
    Node,
    Shape,
}

fn estimate_refused(name: &str, head: Option<&'static str>, estimate: &'static str) -> Whose {
    estimate_refused_at(name, None, head, estimate)
}

fn estimate_refused_at(_name: &str, address: Option<&str>, head: Option<&'static str>, estimate: &'static str) -> Whose {
    vault_open();
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let url = address.map(str::to_string).unwrap_or_else(|| node(head, estimate));
    let table = match eravon_chain::endpoints::Table::new(vec![url]) {
        Ok(t) => t,
        Err(_) => return Whose::Shape,
    };
    let w = eravon_cli::shared::Wire { t: eravon_chain::transport::Auto::web_roots().expect("a transport"), table, source: eravon_cli::home::Source::Inline, chain_id: 31337, logic: String::new(), registries: Vec::new() };
    let call = eravon_standard::call::Call { to: "0x1111111111111111111111111111111111111111".into(), data: Vec::new() };
    match eravon_cli::send::limit(&w, "0x2222222222222222222222222222222222222222", &call, "0") {
        Ok(n) => panic!("the estimate was not refused: {n}"),
        Err(f) if f.extra().iter().any(|(k, v)| *k == "downToken" && matches!(v, eravon_literals::json::Out::S(t) if t == "E_CHAIN_EXECUTION")) => Whose::Call,
        Err(_) => Whose::Node,
    }
}

#[test]
fn a_coded_refusal_in_words_not_known_is_the_estimate_refused() {
    if super::alone_in(module_path!(), "a_coded_refusal_in_words_not_known_is_the_estimate_refused") {
        return;
    }
    assert_eq!(estimate_refused("coded", None, "{\"code\":-32000,\"message\":\"gas required exceeds allowance\"}"), Whose::Call);
}

#[test]
fn an_internal_error_code_is_the_estimate_refused() {
    if super::alone_in(module_path!(), "an_internal_error_code_is_the_estimate_refused") {
        return;
    }
    assert_eq!(estimate_refused("internal", None, "{\"code\":-32603,\"message\":\"internal error\"}"), Whose::Call);
}

#[test]
fn a_code_written_as_text_is_said_as_the_nodes_refusal_not_the_estimates() {
    if super::alone_in(module_path!(), "a_code_written_as_text_is_said_as_the_nodes_refusal_not_the_estimates") {
        return;
    }
    assert_eq!(estimate_refused("text", None, "{\"code\":\"-32000\",\"message\":\"x\"}"), Whose::Node);
}

#[test]
fn a_message_without_a_code_is_said_as_the_nodes_refusal_not_the_estimates() {
    if super::alone_in(module_path!(), "a_message_without_a_code_is_said_as_the_nodes_refusal_not_the_estimates") {
        return;
    }
    assert_eq!(estimate_refused("uncoded", None, "{\"message\":\"upstream unavailable\"}"), Whose::Node);
}

#[test]
fn a_head_refused_with_a_code_is_never_the_estimate_refused() {
    if super::alone_in(module_path!(), "a_head_refused_with_a_code_is_never_the_estimate_refused") {
        return;
    }
    let said = estimate_refused("head", Some("{\"code\":-32000,\"message\":\"gas required exceeds allowance\"}"), "{\"code\":3,\"message\":\"execution reverted\"}");
    assert_eq!(said, Whose::Node);
}

#[test]
fn addresses_that_do_not_read_are_said_as_their_shape_not_the_estimate_refused() {
    if super::alone_in(module_path!(), "addresses_that_do_not_read_are_said_as_their_shape_not_the_estimate_refused") {
        return;
    }
    assert_eq!(estimate_refused_at("unread", Some("wss://node.example"), None, "{}"), Whose::Shape);
}
