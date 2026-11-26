use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use eravon::json::Value;
use eravon_send::node::{Endpoint, NodeAddr};

const CHAIN: u64 = 31337;

type Answer = Arc<dyn Fn(&str, &Value) -> String + Send + Sync>;

fn node(answer: Answer) -> (String, Arc<Mutex<Vec<String>>>) {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", l.local_addr().expect("addr"));
    let asked = Arc::new(Mutex::new(Vec::new()));
    let kept = asked.clone();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let (answer, kept) = (answer.clone(), kept.clone());
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
                let req = eravon::json::parse(body.as_bytes()).unwrap_or(Value::Null);
                let method = req.member("method").and_then(|m| m.as_str()).unwrap_or("").to_string();
                let params = req.member("params").cloned().unwrap_or(Value::Null);
                kept.lock().expect("kept").push(method.clone());
                let id = body.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("1").trim().to_string();
                let result = answer(&method, &params);
                let body = format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result}}}");
                let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
            });
        }
    });
    (url, asked)
}

fn chain_node(chain: u64, tip: u64) -> (String, Arc<Mutex<Vec<String>>>) {
    node(Arc::new(move |method, params| match method {
        "eth_chainId" => format!("\"0x{chain:x}\""),
        "eth_blockNumber" => "\"0x40\"".into(),
        "eth_getBlockByNumber" => match params.as_arr().and_then(|a| a.first()).and_then(|v| v.as_str()) {
            Some("0x30") => "{\"baseFeePerGas\":\"0x3b9aca00\",\"number\":\"0x30\",\"timestamp\":\"0x50\"}".into(),
            _ => "{\"baseFeePerGas\":\"0x3b9aca00\",\"number\":\"0x40\",\"timestamp\":\"0x64\"}".into(),
        },
        "eth_feeHistory" => format!("{{\"oldestBlock\":\"0x2d\",\"gasUsedRatio\":[0.5],\"reward\":[[\"0x{tip:x}\"],[\"0x{tip:x}\"],[\"0x{tip:x}\"]]}}"),
        "eth_estimateGas" => "\"0xc350\"".into(),
        "eth_getLogs" => "[]".into(),
        _ => "null".into(),
    }))
}

fn eps(urls: &[&str]) -> Vec<Endpoint> {
    urls.iter().map(|u| Endpoint::at(CHAIN, NodeAddr::new(*u))).collect()
}

fn quiet() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
}

#[test]
fn two_nodes_whose_fee_histories_differ_give_the_lower_number() {
    quiet();
    let (low, _) = chain_node(CHAIN, 30_000_000);
    let (high, _) = chain_node(CHAIN, 700_000_000);
    let r = eravon_send::node::fee_reading(&eps(&[&high, &low]), CHAIN);
    assert!(r.fees.from_chain, "{r:?}");
    assert_eq!(r.fees.priority, 30_000_000, "the lower median of the two, not the ceiling: {r:?}");
    let (nulls, _) = node(Arc::new(|method, _| match method {
        "eth_chainId" => format!("\"0x{CHAIN:x}\""),
        "eth_blockNumber" => "\"0x40\"".into(),
        "eth_getBlockByNumber" => "{\"baseFeePerGas\":\"0x3b9aca00\",\"number\":\"0x40\",\"timestamp\":\"0x64\"}".into(),
        _ => "null".into(),
    }));
    let one = eravon_send::node::fee_reading(&eps(&[&nulls, &high]), CHAIN);
    assert_eq!(one.fees.priority, eravon_chain::send::PRIORITY_FEE.min(700_000_000), "a node keeping no fee history gives no number: {one:?}");
}
