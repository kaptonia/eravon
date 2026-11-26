use eravon_chain::endpoints::Table;
use eravon_chain::fault::Fault;
use eravon_chain::transport::Http;
use std::io::{Read, Write};

fn node_at(result: &'static str, head: Option<&'static str>) -> String {
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
                let id = req.split("\"id\":").nth(1).and_then(|r| r.split([',', '}']).next()).unwrap_or("1").trim().to_string();
                let method = req.split("\"method\":\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("").to_string();
                let result = match (method.as_str(), head) {
                    ("eth_chainId", _) => "\"0x7a69\"",
                    ("eth_blockNumber", Some(h)) => h,
                    _ => result,
                };
                let body = format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result}}}");
                let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
            });
        }
    });
    url
}

fn node(result: &'static str) -> String {
    node_at(result, None)
}

fn eps(urls: &[String]) -> Table {
    Table::new(urls.to_vec()).expect("a table")
}

fn t() -> Http {
    Http::with_timeout(std::time::Duration::from_secs(5))
}

fn head_block(table: &Table) -> Result<u64, Fault> {
    eravon_chain::read::height(&t(), table).map(|(h, _)| h)
}

fn head_time(table: &Table) -> Result<u64, Fault> {
    eravon_chain::clock::now(&t(), table).map(|(n, _)| n.timestamp)
}

fn name(f: &Fault) -> &'static str {
    f.name()
}

#[test]
fn heads_that_do_not_read_are_the_chains_shape() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let f = head_block(&eps(&[node("\"0xzz\""), node("16")])).expect_err("no head reads");
    assert_eq!(name(&f), "E_CHAIN_ENDPOINT", "{f:?}");
    let f = head_block(&eps(&[node("\"0x10000000000000000\""), node("\"x\"")])).expect_err("no head reads");
    assert_eq!(name(&f), "E_CHAIN_ENDPOINT", "{f:?}");
    let f = head_block(&eps(&[node("null"), node("\"x\"")])).expect_err("no head reads");
    assert_eq!(name(&f), "E_CHAIN_ENDPOINT", "{f:?}");
    let f = head_block(&eps(&[node("null"), node("null")])).expect_err("no head reads");
    assert_eq!(name(&f), "E_CHAIN_ENDPOINT", "{f:?}");
}

#[test]
fn latest_blocks_that_do_not_read_are_the_chains_shape() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let f = head_time(&eps(&[node_at("{\"number\":\"0x20\"}", Some("\"0x20\"")), node_at("\"0x20\"", Some("\"0x20\""))])).expect_err("no time reads");
    assert_eq!(name(&f), "E_CHAIN_DISAGREE", "{f:?}");
    let f = head_time(&eps(&[node_at("null", Some("\"0x20\"")), node_at("null", Some("\"0x20\""))])).expect_err("no time reads");
    assert_eq!(name(&f), "E_CHAIN_ENDPOINT", "{f:?}");
}
