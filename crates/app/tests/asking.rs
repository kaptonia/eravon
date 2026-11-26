use std::io::{Read, Write};
use std::net::TcpListener;

use eravon_chain::endpoints::Table;
use eravon_chain::fault::Fault;
use eravon_chain::read::Agreement;
use eravon_chain::transport::Http;
use app::chainx::{self, Endpoint};

const KEY: &str = "S3CRETk3y";
const WHO: &str = "0x1111111111111111111111111111111111111111";

fn node(answer: fn(&str) -> (u16, String)) -> String {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = l.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            std::thread::spawn(move || {
                let mut s = s;
                let mut seen = Vec::new();
                let mut buf = [0u8; 4096];
                let body = loop {
                    match s.read(&mut buf) {
                        Ok(0) | Err(_) => return,
                        Ok(n) => seen.extend_from_slice(&buf[..n]),
                    }
                    let text = String::from_utf8_lossy(&seen).to_string();
                    if let Some((head, rest)) = text.split_once("\r\n\r\n") {
                        let len = head.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0))).unwrap_or(0);
                        if rest.len() >= len {
                            break rest[..len].to_string();
                        }
                    }
                };
                let id = body.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("1").to_string();
                let method = body.split("\"method\":\"").nth(1).and_then(|x| x.split('"').next()).unwrap_or("").to_string();
                let (status, said) = answer(&method);
                let said = said.replace("\"id\":1", &format!("\"id\":{id}"));
                let _ = s.write_all(format!("HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{said}", said.len()).as_bytes());
            });
        }
    });
    format!("http://127.0.0.1:{port}")
}

fn result(v: &str) -> (u16, String) {
    (200, format!("{{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"{v}\"}}"))
}

fn live(method: &str) -> (u16, String) {
    match method {
        "eth_chainId" => result("0x7a69"),
        "eth_blockNumber" => result("0x1000"),
        _ => result("0x64"),
    }
}

fn other(method: &str) -> (u16, String) {
    match method {
        "eth_getBalance" => result("0x65"),
        m => live(m),
    }
}

fn behind(method: &str) -> (u16, String) {
    match method {
        "eth_blockNumber" => result("0x1"),
        m => live(m),
    }
}

fn refusing(method: &str) -> (u16, String) {
    match method {
        "eth_getBalance" => (200, "{\"jsonrpc\":\"2.0\",\"id\":1,\"error\":{\"code\":-32000,\"message\":\"header not found\"}}".into()),
        m => live(m),
    }
}

fn status(method: &str) -> (u16, String) {
    match method {
        "eth_getBalance" => (503, "<html>busy</html>".into()),
        m => live(m),
    }
}

fn malformed(method: &str) -> (u16, String) {
    match method {
        "eth_getBalance" => (200, "<html>ok</html>".into()),
        m => live(m),
    }
}

fn no_receipt(method: &str) -> (u16, String) {
    match method {
        "eth_getTransactionReceipt" => (200, "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":null}".into()),
        m => live(m),
    }
}

fn receipt(method: &str) -> (u16, String) {
    match method {
        "eth_getTransactionReceipt" => (200, "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"blockNumber\":\"0x5\",\"status\":\"0x1\",\"gasUsed\":\"0x5208\"}}".into()),
        m => live(m),
    }
}

fn echo(method: &str) -> (u16, String) {
    match method {
        "eth_sendRawTransaction" => result("0x00000000000000000000000000000000000000000000000000000000000000ee"),
        m => live(m),
    }
}

fn dead() -> String {
    let port = TcpListener::bind("127.0.0.1:0").expect("bind").local_addr().expect("addr").port();
    format!("http://127.0.0.1:{port}")
}

fn table(list: &[String]) -> Table {
    Table::new(list.to_vec()).expect("a table")
}

#[test]
fn a_key_in_a_node_address_is_in_no_sentence() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let t = Http::with_timeout(std::time::Duration::from_secs(5));
    let gone = dead();
    let path = |at: &str| format!("{at}/v3/{KEY}");
    let forms: Vec<(&str, Vec<String>)> = vec![
        ("deadInPath", vec![path(&gone)]),
        ("deadInQuery", vec![format!("{gone}/rpc?apikey={KEY}")]),
        ("aliveBesideDead", vec![path(&gone), path(&node(live))]),
        ("refusingInPath", vec![path(&node(refusing))]),
        ("statusInPath", vec![path(&node(status))]),
        ("malformedInPath", vec![path(&node(malformed))]),
        ("differInPath", vec![path(&node(live)), path(&node(other))]),
        ("behindInPath", vec![path(&node(live)), path(&node(behind))]),
    ];
    for (form, list) in forms {
        let tb = table(&list);
        let mut said = 0;
        for got in [eravon_chain::read::balance(&t, &tb, WHO, "latest").map(|(_, r)| r), eravon_chain::read::height(&t, &tb).map(|(_, r)| r)] {
            match got {
                Ok(r) => {
                    for (e, why) in &r.left {
                        assert!(!e.said().contains(KEY) && !format!("{e} {e:?}").contains(KEY) && !why.contains(KEY), "{form}: {e:?} {why}");
                        said += 1;
                    }
                }
                Err(f) => {
                    assert!(!f.detail().contains(KEY), "{form}: {}", f.detail());
                    let toast = app::house::sync::fault_toast(&f);
                    assert!(!toast.detail.as_deref().unwrap_or("").contains(KEY), "{form}: {:?}", toast.detail);
                    said += 1;
                }
            }
        }
        assert!(said > 0, "{form}: something was said about the node");
    }
    const TX: &str = "0x00000000000000000000000000000000000000000000000000000000000000aa";
    let split = table(&[path(&node(receipt)), path(&node(no_receipt))]);
    let f = eravon_chain::sending::receipt(&t, &split, TX).expect_err("split");
    assert!(f.detail().starts_with("not-yet") && !f.detail().contains(KEY), "notYetInPath: {}", f.detail());
    let echoing = table(&[path(&node(echo))]);
    let f = eravon_chain::sending::send_everywhere(&t, &echoing, &[0x02], TX).expect_err("another hash");
    assert!(f.name() == "E_CHAIN_ECHO" && !f.detail().contains(KEY), "echoInPath: {}", f.detail());
    assert!(!app::house::sync::fault_toast(&f).detail.unwrap_or_default().contains(KEY));
}

#[test]
fn the_home_read_failure_toast_names_no_key() {
    let tb = table(&[format!("http://127.0.0.1:1/v3/{KEY}"), format!("http://127.0.0.1:2/?apikey={KEY}")]);
    let s = app::house::home::read_failed(&("E_CHAIN_TRANSPORT".to_string(), "connect".to_string()), &tb);
    let detail = s.detail.unwrap_or_default();
    assert!(!detail.contains(KEY) && detail.contains("http://127.0.0.1:1") && detail.contains("http://127.0.0.1:2"), "{detail}");
}

#[test]
fn a_node_item_that_does_not_read_is_said_by_its_length() {
    const IN: &str = "k3yInTheItem55";
    for item in [format!("http://127.0.0.1:1/{IN} x"), format!("ftp://127.0.0.1:1/{IN}"), format!("http://user:{IN}@127.0.0.1:1/"), format!("http://127.0.0.1:1?apikey={IN}")] {
        let said = Table::new(vec![item.clone()]).expect_err("refused").detail();
        assert!(!said.contains(IN), "{item}: the key is said: {said}");
        assert!(said.contains(&format!("({} 字节)", item.len())), "{item}: said by its length: {said}");
    }
    const KEY: &str = "k3yInTheItem55";
    for item in [format!("31337=http://127.0.0.1:1/{KEY} x"), format!("http://127.0.0.1:1/{KEY}"), format!("chain=http://127.0.0.1:1/{KEY}")] {
        let said = app::chainx::typed(&item).expect_err("refused");
        assert!(!said.contains(KEY), "{item}: the key is said: {said}");
        assert!(said.contains(&eravon_net::sayable(&item)), "{item}: said as a node is said: {said}");
    }
}

fn rpc_node(answer: fn(&str) -> Option<String>) -> String {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", l.local_addr().expect("addr"));
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
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
                let id = body.split("\"id\":").nth(1).and_then(|r| r.split(|c| c == ',' || c == '}').next()).unwrap_or("1").trim().to_string();
                let Some(result) = answer(&method) else { return };
                let body = format!("{{\"id\":{id},\"jsonrpc\":\"2.0\",\"result\":{result}}}");
                let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
            });
        }
    });
    url
}

fn eps(urls: &[&String]) -> Table {
    Table::new(urls.iter().map(|u| u.to_string()).collect()).expect("a table")
}

fn quick() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
}

fn http() -> Http {
    Http::with_timeout(std::time::Duration::from_secs(5))
}

fn head_block(table: &Table) -> Result<(u64, usize), Fault> {
    eravon_chain::read::height(&http(), table).map(|(h, r)| (h, r.endpoints))
}

fn balance(table: &Table) -> Result<(String, eravon_chain::read::Reading), Fault> {
    eravon_chain::read::balance(&http(), table, WHO, "latest")
}

fn node_at_256(m: &str) -> Option<String> {
    Some(match m {
        "eth_chainId" => "\"0x7a69\"".into(),
        "eth_blockNumber" => "\"0x100\"".into(),
        "eth_getBalance" => "\"0x2a\"".into(),
        "eth_getBlockByNumber" => "{\"baseFeePerGas\":\"0x7\",\"number\":\"0x100\",\"timestamp\":\"0x64\"}".into(),
        "eth_feeHistory" => "{\"oldestBlock\":\"0xfb\",\"reward\":[[\"0x3\"],[\"0x3\"]]}".into(),
        _ => "null".into(),
    })
}
fn node_at_240(m: &str) -> Option<String> {
    Some(match m {
        "eth_blockNumber" => "\"0xf0\"".into(),
        "eth_getBlockByNumber" => "{\"baseFeePerGas\":\"0x7\",\"number\":\"0xf0\",\"timestamp\":\"0x60\"}".into(),
        other => return node_at_256(other),
    })
}
fn node_at_128(m: &str) -> Option<String> {
    Some(match m {
        "eth_blockNumber" => "\"0x80\"".into(),
        other => return node_at_256(other),
    })
}

#[test]
fn the_head_is_the_smallest_and_a_node_too_far_behind_is_named() {
    quick();
    let (high, near, far) = (rpc_node(node_at_256), rpc_node(node_at_240), rpc_node(node_at_128));
    assert_eq!(head_block(&eps(&[&high, &near])).expect("a head"), (0xf0, 2));
    assert_eq!(eravon_chain::seam::HEIGHT_SLACK, 64);
    let f = head_block(&eps(&[&high, &far])).expect_err("a node behind");
    assert_eq!(f.name(), "E_CHAIN_DISAGREE");
    assert!(f.detail().contains(&far) && f.detail().contains("128") && f.detail().contains("256"), "{}", f.detail());
    let gone = dead();
    assert_eq!(head_block(&eps(&[&gone, &high])).expect("a head").0, 0x100);
}

#[test]
fn a_silent_node_is_passed_over_named_and_the_reading_is_single_source() {
    quick();
    let (live, gone) = (rpc_node(node_at_256), dead());
    let on = eps(&[&gone, &live]);
    let _ = on.chain.set(31337);
    let (wei, r) = balance(&on).expect("a balance");
    assert_eq!((wei.as_str(), r.endpoints, r.agreement == Agreement::Single), ("42", 1, true));
    assert_eq!(r.left.len(), 1);
    assert!(r.left[0].0.said().contains(&gone), "the silent node is named: {:?}", r.left);
    let live2 = rpc_node(node_at_256);
    let (_, r) = balance(&eps(&[&live, &live2])).expect("a balance");
    assert_eq!((r.endpoints, r.agreement == Agreement::Single, r.left.len()), (2, false, 0));
}

fn has_tx(m: &str) -> Option<String> {
    Some(match m {
        "eth_chainId" => "\"0x7a69\"".into(),
        "eth_getTransactionByHash" => "{\"blockNumber\":\"0x5\",\"from\":\"0x1111111111111111111111111111111111111111\",\"input\":\"0x\",\"hash\":\"0xab\"}".into(),
        _ => "null".into(),
    })
}
fn lacks_tx(m: &str) -> Option<String> {
    Some(match m {
        "eth_chainId" => "\"0x7a69\"".into(),
        _ => "null".into(),
    })
}

#[test]
fn a_transaction_one_node_has_and_another_not_yet_is_refused_naming_the_one_without() {
    quick();
    let (has, lacks) = (rpc_node(has_tx), rpc_node(lacks_tx));
    let q = format!("0x{}", "ab".repeat(32));
    let f = eravon_chain::read::transaction(&http(), &eps(&[&has, &lacks]), &q).expect_err("split");
    assert_eq!(f.name(), "E_CHAIN_DISAGREE");
    assert!(f.detail().contains(&lacks), "the node without it is named: {}", f.detail());
    let has2 = rpc_node(has_tx);
    let (_, r) = eravon_chain::read::transaction(&http(), &eps(&[&has, &has2]), &q).expect("agreed");
    assert_eq!((r.endpoints, r.agreement == Agreement::Single), (2, false));
    let lacks2 = rpc_node(lacks_tx);
    assert_eq!(eravon_chain::read::transaction(&http(), &eps(&[&lacks, &lacks2]), &q).expect("none").0, eravon_literals::json::In::Null);
}

fn height_zero(m: &str) -> Option<String> {
    Some(match m {
        "eth_blockNumber" => "\"0x0\"".into(),
        other => return node_at_256(other),
    })
}
fn height_max(m: &str) -> Option<String> {
    Some(match m {
        "eth_blockNumber" => "\"0xffffffffffffffff\"".into(),
        other => return node_at_256(other),
    })
}
fn height_past_max(m: &str) -> Option<String> {
    Some(match m {
        "eth_blockNumber" => "\"0x10000000000000000\"".into(),
        other => return node_at_256(other),
    })
}
fn height_bare_prefix(m: &str) -> Option<String> {
    Some(match m {
        "eth_blockNumber" => "\"0x\"".into(),
        other => return node_at_256(other),
    })
}
fn height_word(m: &str) -> Option<String> {
    Some(match m {
        "eth_blockNumber" => "\"latest\"".into(),
        other => return node_at_256(other),
    })
}

#[test]
fn a_height_that_does_not_read_as_64_bits_is_named_never_zero() {
    quick();
    let zero = rpc_node(height_zero);
    assert_eq!(head_block(&eps(&[&zero])).expect("zero reads"), (0, 1), "zero is a height");
    let max = rpc_node(height_max);
    assert_eq!(head_block(&eps(&[&max])).expect("the largest reads"), (u64::MAX, 1));
    for (form, node) in [("pastMax", height_past_max as fn(&str) -> Option<String>), ("barePrefix", height_bare_prefix), ("word", height_word)] {
        let url = rpc_node(node);
        let f = head_block(&eps(&[&url])).expect_err(form);
        assert_eq!(f.name(), "E_CHAIN_ENDPOINT", "{form}: {f:?}");
        assert!(f.detail().contains(&url), "{form}: the node named: {}", f.detail());
        let good = rpc_node(node_at_256);
        assert_eq!(head_block(&eps(&[&url, &good])).expect(form).0, 0x100, "{form}: the one that reads");
    }
}

fn node_at_255(m: &str) -> Option<String> {
    Some(match m {
        "eth_blockNumber" => "\"0xff\"".into(),
        other => return node_at_256(other),
    })
}
fn node_richer(m: &str) -> Option<String> {
    Some(match m {
        "eth_getBalance" => "\"0x2b\"".into(),
        other => return node_at_256(other),
    })
}

#[test]
fn the_balance_before_sending_is_pinned_and_compared() {
    quick();
    eravon_chain::read::forget_checked();
    let (a, behind) = (rpc_node(node_at_256), rpc_node(node_at_255));
    let (n, r) = balance(&eps(&[&a, &behind])).expect("a node one block behind agrees");
    assert_eq!((n.as_str(), r.endpoints, r.agreement == Agreement::Single), ("42", 2, false));
    let richer = rpc_node(node_richer);
    let f = balance(&eps(&[&a, &richer])).expect_err("two answers that differ");
    assert_eq!(f.name(), "E_CHAIN_DISAGREE", "{f:?}");
    let gone = dead();
    let (n, r) = balance(&eps(&[&gone, &a])).expect("one node alone");
    assert_eq!((n.as_str(), r.endpoints, r.agreement == Agreement::Single), ("42", 1, true));
    assert!(eravon_send::batch::funds_check(42, 42).is_ok(), "exactly enough");
    let short = app::fault::Fault::from(eravon_send::batch::funds_check(42, 41).expect_err("one wei short"));
    assert_eq!((short.which(), short.tail()), (Some(app::fault::Known::InsufficientFunds), app::chainx::funds_tail(42, 41).as_str()));
}

fn nodes(urls: &[&String]) -> Vec<Endpoint> {
    urls.iter().map(|u| Endpoint::parse(&format!("31337={u}")).expect("an endpoint")).collect()
}

type Answer = std::sync::Arc<dyn Fn(&str, usize) -> Option<String> + Send + Sync>;
type Asked = std::sync::Arc<std::sync::Mutex<Vec<String>>>;
fn counted_node(answer: Answer) -> (String, Asked) {
    use std::io::{Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", l.local_addr().expect("addr"));
    let asked: Asked = Default::default();
    let log = asked.clone();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let (answer, log) = (answer.clone(), log.clone());
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
                let id = body.split("\"id\":").nth(1).and_then(|r| r.split(|c| c == ',' || c == '}').next()).unwrap_or("1").trim().to_string();
                let n = {
                    let mut g = log.lock().expect("log");
                    let n = g.iter().filter(|m| **m == method).count();
                    g.push(method.clone());
                    n
                };
                let Some(member) = answer(&method, n) else { return };
                let body = format!("{{\"id\":{id},\"jsonrpc\":\"2.0\",{member}}}");
                let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
            });
        }
    });
    (url, asked)
}

fn result_256(m: &str) -> Option<String> {
    node_at_256(m).map(|r| format!("\"result\":{r}"))
}

fn times(asked: &Asked, method: &str) -> usize {
    asked.lock().expect("log").iter().filter(|m| *m == method).count()
}

const BUSY: &str = "\"error\":{\"code\":-32603,\"message\":\"service temporarily unavailable\"}";

#[test]
fn a_node_that_gave_nothing_is_asked_nothing_more_in_the_read() {
    quick();
    chainx::forget_serving();
    let forms: [(&str, Answer, &[(&str, usize)]); 3] = [
        ("the head refused", std::sync::Arc::new(|m: &str, _| if m == "eth_blockNumber" { Some(BUSY.into()) } else { result_256(m) }), &[("eth_chainId", 1), ("eth_blockNumber", 2), ("eth_getBalance", 0)]),
        ("the head no number", std::sync::Arc::new(|m: &str, _| if m == "eth_blockNumber" { Some("\"result\":\"x\"".into()) } else { result_256(m) }), &[("eth_chainId", 1), ("eth_blockNumber", 1), ("eth_getBalance", 0)]),
        ("the way broken", std::sync::Arc::new(|_: &str, _| None), &[("eth_blockNumber", 0), ("eth_getBalance", 0)]),
    ];
    for (form, answer, want) in forms {
        let (bad, asked) = counted_node(answer);
        let good = rpc_node(node_at_256);
        let (wei, r) = chainx::balance(&nodes(&[&bad, &good]), 31337, &app::key::Address([0x11; 20])).expect(form);
        assert_eq!((wei, r.sources, r.single_source), (0x2a, 1, true), "{form}");
        for (method, n) in want {
            assert_eq!(times(&asked, method), *n, "{form}: {method}");
        }
        let before = times(&asked, "eth_blockNumber");
        let _ = chainx::head_block(&nodes(&[&bad, &good]), 31337);
        assert!(times(&asked, "eth_blockNumber") > before, "{form}: a new read asks again");
    }
}

#[test]
fn a_node_saying_not_yet_is_asked_again_in_the_read() {
    quick();
    let tx = "{\"blockNumber\":\"0x5\",\"from\":\"0x11\",\"input\":\"0x\"}";
    let (late, asked) = counted_node(std::sync::Arc::new(move |m: &str, n| match m {
        "eth_getTransactionByHash" if n == 0 => Some("\"result\":null".into()),
        "eth_getTransactionByHash" => Some(format!("\"result\":{tx}")),
        other => result_256(other),
    }));
    let (has, _) = counted_node(std::sync::Arc::new(move |m: &str, _| match m {
        "eth_getTransactionByHash" => Some(format!("\"result\":{tx}")),
        other => result_256(other),
    }));
    let q = eravon::json::Value::Arr(vec![eravon::json::Value::Str(format!("0x{}", "ab".repeat(32)))]);
    let r = chainx::ask(&nodes(&[&late, &has]), "eth_getTransactionByHash", &q).expect("both have it");
    assert_eq!((r.sources, r.single_source, times(&asked, "eth_getTransactionByHash")), (2, false, 2));
}

#[test]
fn a_node_passed_over_is_named_once_in_the_read() {
    quick();
    chainx::forget_serving();
    let (gone, asked) = counted_node(std::sync::Arc::new(|_: &str, _| None));
    let good = rpc_node(node_at_256);
    let (_, r) = chainx::balance(&nodes(&[&gone, &good]), 31337, &app::key::Address([0x11; 20])).expect("a balance");
    assert_eq!(r.unanswered.len(), 1, "{:?}", r.unanswered);
    assert!(chainx::said_of(&r.unanswered[0]).contains(&gone), "{:?}", r.unanswered);
    let n = asked.lock().expect("log").len();
    {
        let _read = chainx::OneRead::open();
        let (_, r) = chainx::balance(&nodes(&[&gone, &good]), 31337, &app::key::Address([0x11; 20])).expect("a balance");
        assert_eq!(r.unanswered.iter().filter(|u| chainx::said_of(u).contains(&gone)).count(), 1, "{:?}", r.unanswered);
        assert!(chainx::head_time(&nodes(&[&gone, &good]), 31337).is_ok());
    }
    let asked_in_read = asked.lock().expect("log")[n..].to_vec();
    assert!(asked_in_read.iter().all(|m| m == "eth_chainId"), "only its first question: {asked_in_read:?}");
}

#[test]
fn the_questions_outside_a_read_still_ask_a_node_it_passed_over() {
    quick();
    chainx::forget_serving();
    let (flaky, asked) = counted_node(std::sync::Arc::new(|m: &str, _| match m {
        "eth_blockNumber" => Some(BUSY.into()),
        "eth_getTransactionByHash" => Some("\"result\":{\"blockNumber\":\"0x5\",\"from\":\"0x11\",\"input\":\"0x\"}".into()),
        "eth_getTransactionReceipt" => Some("\"result\":{\"blockNumber\":\"0x5\",\"status\":\"0x1\"}".into()),
        "eth_getLogs" => Some("\"result\":[]".into()),
        other => result_256(other),
    }));
    let good = rpc_node(node_at_256);
    let _read = chainx::OneRead::open();
    assert!(chainx::head_block(&nodes(&[&flaky, &good]), 31337).is_ok());
    assert_eq!(times(&asked, "eth_blockNumber"), 2);
    let nodes: Vec<(String, Box<dyn eravon_chain::rpc::Endpoint + Send>)> = vec![(flaky.clone(), chainx::endpoint_at(&flaky).expect("opens"))];
    let _ = eravon_chain::endpoints::ask_each(nodes, "eth_blockNumber", &eravon::json::Value::Arr(Vec::new()), eravon_chain::endpoints::every_answer);
    assert_eq!(times(&asked, "eth_blockNumber"), 3);
    assert!(chainx::tx_as_held(&[app::chainx::NodeAddr::new(flaky.clone())], 31337, &format!("0x{}", "ab".repeat(32))).held().is_some());
    assert_eq!(times(&asked, "eth_getTransactionByHash"), 1);
    let mut ep = chainx::endpoint_at(&flaky).expect("opens");
    let mut eps_r: Vec<&mut dyn eravon_chain::rpc::Endpoint> = vec![ep.as_mut() as &mut dyn eravon_chain::rpc::Endpoint];
    assert!(matches!(eravon_chain::send::confirm_each(&mut eps_r, &[0xab; 32], std::time::Duration::ZERO, &[]), eravon_chain::send::Confirm::Included { .. }));
    assert_eq!(times(&asked, "eth_getTransactionReceipt"), 1);
    let mut ep = chainx::endpoint_at(&flaky).expect("opens");
    let filter = eravon_chain::scan::LogFilter { addresses: vec![[0x11; 20]], topics: Vec::new() };
    assert!(eravon_chain::scan::logs_over_window(ep.as_mut(), 31337, &filter, 1, 10).is_ok());
    assert_eq!(times(&asked, "eth_getLogs"), 1);
}

#[test]
fn an_answer_the_reading_cannot_take_passes_the_node_over_in_the_read() {
    quick();
    let wide: Answer = std::sync::Arc::new(|m: &str, _| if m == "eth_blockNumber" { Some("\"result\":\"0x10000000000000000\"".into()) } else { result_256(m) });
    let timeless: Answer = std::sync::Arc::new(|m: &str, _| if m == "eth_getBlockByNumber" { Some("\"result\":{\"number\":\"0x100\"}".into()) } else { result_256(m) });
    for (form, answer, first) in [("a head past 64 bits", wide, "eth_blockNumber"), ("a block with no time", timeless, "eth_getBlockByNumber")] {
        let (odd, asked) = counted_node(answer);
        let good = rpc_node(node_at_256);
        let _read = chainx::OneRead::open();
        let first_ok = if first == "eth_blockNumber" { chainx::head_block(&nodes(&[&odd, &good]), 31337).is_ok() } else { chainx::head_time(&nodes(&[&odd, &good]), 31337).is_ok() };
        assert!(first_ok, "{form}: the other node's reading");
        let n = asked.lock().expect("log").len();
        let q = eravon::json::Value::Arr(vec![eravon::json::Value::Str(format!("0x{}", "11".repeat(20))), eravon::json::Value::Str("0x100".into())]);
        let r = chainx::ask(&nodes(&[&odd, &good]), "eth_getBalance", &q).expect(form);
        assert_eq!(asked.lock().expect("log").len(), n, "{form}: asked nothing more in the read");
        assert_eq!((r.sources, r.unanswered.iter().filter(|u| chainx::said_of(u).contains(&odd)).count()), (1, 1), "{form}: {:?}", r.unanswered);
    }
}

#[test]
fn a_rest_row_with_no_key_here_is_asked_nothing_and_named_once_in_each_reading() {
    quick();
    let good = rpc_node(node_at_256);
    let mut nodes = nodes(&[&good]);
    nodes.push(Endpoint::at(31337, app::chainx::NodeAddr::rest("https://api.etherscan.io/v2/api", 31337)));
    let _read = chainx::OneRead::open();
    let head = chainx::ask(&nodes, "eth_blockNumber", &eravon::json::Value::Arr(Vec::new())).expect("the head from the other node");
    let q = eravon::json::Value::Arr(vec![eravon::json::Value::Str(format!("0x{}", "11".repeat(20))), eravon::json::Value::Str("0x100".into())]);
    let code = chainx::ask(&nodes, "eth_getCode", &q).expect("a second question of the read");
    for r in [head, code] {
        assert_eq!(r.sources, 1, "{:?}", r.unanswered);
        assert!(r.single_source, "one place answered");
        let named: Vec<String> = r.unanswered.iter().map(chainx::said_of).filter(|u| u.contains("NO_KEY_HERE")).collect();
        assert_eq!(named.len(), 1, "{:?}", r.unanswered);
        assert!(named[0].contains("https://api.etherscan.io") && !named[0].contains("/v2/api") && !named[0].contains("#rest"), "said by its service alone: {}", named[0]);
    }
}
