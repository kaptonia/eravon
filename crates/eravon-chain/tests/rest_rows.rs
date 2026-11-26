use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use eravon_chain::endpoints::Table;
use eravon_chain::json_in::In;
use eravon_chain::read::{self, Agreement};
use eravon_chain::transport::Auto;

const TX: &str = "0x1111111111111111111111111111111111111111111111111111111111111111";
const TOPIC: &str = "0x2222222222222222222222222222222222222222222222222222222222222222";
const KEY: &str = "REST-TEST-KEY-5d1e";

fn result_of(method: &str) -> &'static str {
    match method {
        "eth_chainId" => "\"0x7a69\"",
        "eth_blockNumber" => "\"0x10\"",
        "eth_getBlockByNumber" => r#"{"number":"0x10","hash":"0x3333333333333333333333333333333333333333333333333333333333333333","timestamp":"0x64"}"#,
        "eth_getTransactionByHash" => r#"{"hash":"0x1111111111111111111111111111111111111111111111111111111111111111","blockNumber":"0x9","from":"0x00000000000000000000000000000000000000aa","to":"0x00000000000000000000000000000000000000bb","value":"0x0","nonce":"0x0","input":"0x"}"#,
        "eth_feeHistory" => r#"{"oldestBlock":"0xc","baseFeePerGas":["0x1","0x1","0x1","0x1","0x1","0x1"],"gasUsedRatio":[0.5,0.5,0.5,0.5,0.5],"reward":[["0x1"],["0x1"],["0x1"],["0x1"],["0x1"]]}"#,
        "eth_getLogs" => "[]",
        _ => "null",
    }
}

fn request(s: &mut impl Read) -> Option<(String, String)> {
    let mut got = Vec::new();
    let mut buf = [0u8; 8192];
    let head_end = loop {
        match s.read(&mut buf) {
            Ok(0) | Err(_) => return None,
            Ok(n) => got.extend_from_slice(&buf[..n]),
        }
        if let Some(i) = got.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let head = String::from_utf8_lossy(&got[..head_end]).to_string();
    let len: usize = head.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse().unwrap_or(0))).unwrap_or(0);
    while got.len() < head_end + len {
        match s.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => got.extend_from_slice(&buf[..n]),
        }
    }
    let target = head.lines().next().unwrap_or("").split(' ').nth(1).unwrap_or("").to_string();
    Some((target, String::from_utf8_lossy(&got[head_end..]).to_string()))
}

fn reply(s: &mut impl Write, body: &str) {
    let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
    let _ = s.flush();
}

fn rpc_node() -> String {
    rpc_node_with("[]")
}

fn rpc_node_with(logs: &'static str) -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", l.local_addr().expect("addr"));
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            std::thread::spawn(move || {
                let mut s = s;
                let Some((_, body)) = request(&mut s) else { return };
                let method = body.split("\"method\":\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("").to_string();
                let id = body.split("\"id\":").nth(1).and_then(|r| r.split([',', '}']).next()).unwrap_or("1").trim().to_string();
                let result = if method == "eth_getLogs" { logs } else { result_of(&method) };
                reply(&mut s, &format!("{{\"id\":{id},\"jsonrpc\":\"2.0\",\"result\":{result}}}"));
            });
        }
    });
    url
}

fn member(target: &str, k: &str) -> String {
    target.split_once('?').map(|(_, q)| q).unwrap_or("").split('&').filter_map(|kv| kv.split_once('=')).find(|(x, _)| *x == k).map(|(_, v)| v.to_string()).unwrap_or_default()
}

fn rest_service() -> (u16, Arc<AtomicUsize>, Arc<Mutex<Vec<String>>>) {
    rest_service_with("[]")
}

fn rest_service_with(logs: &'static str) -> (u16, Arc<AtomicUsize>, Arc<Mutex<Vec<String>>>) {
    let cert = rustls::pki_types::CertificateDer::from(include_bytes!("../../eravon-net/tests/fixtures/tls/localhost.crt.der").to_vec());
    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(include_bytes!("../../eravon-net/tests/fixtures/tls/localhost.key.der").to_vec().into());
    let config = Arc::new(
        rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .expect("versions")
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .expect("the test certificate"),
    );
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = l.local_addr().expect("addr").port();
    let asked = Arc::new(AtomicUsize::new(0));
    let keys = Arc::new(Mutex::new(Vec::new()));
    let l6 = std::net::TcpListener::bind(("::1", port)).ok();
    for l in std::iter::once(l).chain(l6) {
        let (config, asked, keys) = (config.clone(), asked.clone(), keys.clone());
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                let (config, asked, keys) = (config.clone(), asked.clone(), keys.clone());
                std::thread::spawn(move || {
                    let Ok(conn) = rustls::ServerConnection::new(config) else { return };
                    let mut tls = rustls::StreamOwned::new(conn, s);
                    let Some((target, _)) = request(&mut tls) else { return };
                    asked.fetch_add(1, Ordering::SeqCst);
                    keys.lock().unwrap_or_else(|e| e.into_inner()).push(member(&target, "apikey"));
                    let body = match member(&target, "module").as_str() {
                        "proxy" => format!("{{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}}", result_of(&member(&target, "action"))),
                        "logs" => format!(r#"{{"status":"1","message":"OK","result":{logs}}}"#),
                        _ => r#"{"status":"0","message":"NOTOK","result":"unknown module"}"#.to_string(),
                    };
                    reply(&mut tls, &body);
                    tls.conn.send_close_notify();
                    let _ = tls.flush();
                });
            }
        });
    }
    (port, asked, keys)
}

fn setup() -> Auto {
    static ROOT: OnceLock<bool> = OnceLock::new();
    assert!(*ROOT.get_or_init(|| eravon_net::drive_trust_root(include_bytes!("../../eravon-net/tests/fixtures/tls/ca.crt.der"))), "the test root is added before any handshake");
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    eravon_chain::rpc::follow_the_terminal();
    eravon_chain::transport::set_way(eravon_net::Choice::Off);
    Auto::web_roots().expect("a transport")
}

fn turn() -> std::sync::MutexGuard<'static, ()> {
    static TURN: Mutex<()> = Mutex::new(());
    TURN.lock().unwrap_or_else(|e| e.into_inner())
}

fn lane(port: u16) -> String {
    eravon_chain::rest::lane(&format!("https://localhost:{port}/v2/api"), 31337)
}

#[test]
fn a_rest_row_is_one_witness_beside_a_json_rpc_node() {
    let _t = turn();
    let t = setup();
    let (port, asked, keys) = rest_service();
    eravon_chain::rest::hold_keys(vec![(format!("localhost:{port}"), KEY.into())]);
    let table = Table::new(vec![rpc_node(), lane(port)]).expect("a table with a REST row");
    let (tx, r) = read::transaction(&t, &table, TX).expect("the transaction, agreed");
    assert_eq!(tx.get("blockNumber").and_then(In::as_str), Some("0x9"));
    assert_eq!((r.agreement, r.endpoints), (Agreement::Agreed, 2), "{:?}", r.left);
    assert!(r.left.is_empty(), "{:?}", r.left);
    assert!(asked.load(Ordering::SeqCst) >= 1);
    assert!(keys.lock().unwrap().iter().all(|k| k == KEY), "asked with this machine's key");
    assert!(!format!("{:?}", r.left).contains(KEY));
}

#[test]
fn what_the_service_does_not_serve_is_named_and_the_read_goes_on() {
    let _t = turn();
    let t = setup();
    let (port, _, _) = rest_service();
    eravon_chain::rest::hold_keys(vec![(format!("localhost:{port}"), KEY.into())]);
    let table = Table::new(vec![rpc_node(), lane(port)]).expect("a table with a REST row");
    let r = read::read_agreed_raw(&t, &table, "eth_feeHistory", r#"["0x5","0xc",[50]]"#).expect("the fee history from the JSON-RPC node");
    assert_eq!((r.agreement, r.endpoints), (Agreement::Single, 1));
    let named: Vec<String> = r.left.iter().map(|(n, why)| format!("{n} {why}")).collect();
    assert!(named.len() == 1 && named[0].contains("NOT_SERVED") && named[0].contains(&format!("localhost:{port}")) && !named[0].contains("/v2/api"), "{named:?}");
    let (logs, r) = read::logs_any(&t, &table, None, &[vec![TOPIC.to_string()]], 1, eravon_chain::seam::Upto::Block(16)).expect("the logs from the JSON-RPC node");
    assert!(logs.is_empty());
    assert_eq!(r.endpoints, 2, "the row asked by the topics alone: {:?}", r.left);
    assert!(r.left.is_empty(), "{:?}", r.left);
}

#[test]
fn a_row_with_no_key_here_asks_nothing_and_is_named() {
    let _t = turn();
    let t = setup();
    let (port, asked, _) = rest_service();
    eravon_chain::rest::hold_keys(Vec::new());
    let table = Table::new(vec![rpc_node(), lane(port)]).expect("a table with a REST row");
    let (_, r) = read::transaction(&t, &table, TX).expect("the transaction from the JSON-RPC node");
    assert_eq!((r.agreement, r.endpoints), (Agreement::Single, 1));
    assert!(r.left.len() == 1 && r.left[0].1.contains("NO_KEY_HERE"), "{:?}", r.left);
    let alone = Table::new(vec![lane(port)]).expect("a table of the row alone");
    let f = read::transaction(&t, &alone, TX).expect_err("no node answers");
    assert_eq!(f.name(), "E_CHAIN_ENDPOINT", "{}", f.detail());
    assert!(f.detail().contains("NO_KEY_HERE"), "{}", f.detail());
    assert_eq!(asked.load(Ordering::SeqCst), 0, "nothing was asked of the service");
}

macro_rules! log_at {
    ($block:literal, $tx:literal) => {
        concat!(r#"{"address":"0x00000000000000000000000000000000000000bb","blockHash":"0x44444444444444444444444444444444444444444444444444444444444444"#, $block, r#"","blockNumber":"0x"#, $block, r#"","data":"0x","logIndex":"0x0","removed":false,"topics":["0x2222222222222222222222222222222222222222222222222222222222222222"],"transactionHash":"0x55555555555555555555555555555555555555555555555555555555555555"#, $tx, r#"","transactionIndex":"0x0"}"#)
    };
}

#[test]
fn a_rest_row_only_late_with_its_logs_leaves_the_read_and_is_no_disagreement() {
    let _t = turn();
    let t = setup();
    const BOTH: &str = concat!("[", log_at!("08", "08"), ",", log_at!("09", "09"), "]");
    const EARLY: &str = concat!("[", log_at!("08", "08"), "]");
    const OTHER: &str = concat!("[", log_at!("08", "08"), ",", log_at!("09", "0a"), "]");
    const HOLE: &str = concat!("[", log_at!("09", "09"), "]");
    let node = rpc_node_with(BOTH);
    let (port, _, _) = rest_service_with(EARLY);
    eravon_chain::rest::hold_keys(vec![(format!("localhost:{port}"), KEY.into())]);
    let table = Table::new(vec![node.clone(), lane(port)]).expect("a table with a REST row");
    let (logs, r) = read::logs_any(&t, &table, None, &[vec![TOPIC.to_string()]], 1, eravon_chain::seam::Upto::Block(16)).expect("the node's logs");
    assert_eq!(logs.len(), 2, "every log the node gave");
    assert_eq!((r.agreement, r.endpoints), (Agreement::Single, 1), "{:?}", r.left);
    assert!(r.left.len() == 1 && r.left[0].1 == format!("LOGS_NOT_YET https://localhost:{port}"), "{:?}", r.left);
    for wrong in [OTHER, HOLE] {
        let (port, _, _) = rest_service_with(wrong);
        eravon_chain::rest::hold_keys(vec![(format!("localhost:{port}"), KEY.into())]);
        let table = Table::new(vec![node.clone(), lane(port)]).expect("a table with a REST row");
        let f = read::logs_any(&t, &table, None, &[vec![TOPIC.to_string()]], 1, eravon_chain::seam::Upto::Block(16)).expect_err("a disagreement");
        assert_eq!(f.name(), "E_CHAIN_DISAGREE", "{wrong}: {}", f.detail());
    }
}
