use std::collections::BTreeMap;
use std::sync::Mutex;

use eravon_chain::endpoints::Table;
use eravon_chain::seam::Url;
use eravon_chain::transport::{Answer, Transport};
use eravon_literals::json::In;

#[derive(Clone)]
struct Node {
    head: u64,
    logs: Vec<(String, u64, String)>,
    posts: std::sync::Arc<Mutex<Vec<(u16, Vec<String>)>>>,
}

impl Node {
    fn new(head: u64, logs: Vec<(String, u64, String)>) -> Node {
        Node { head, logs, posts: std::sync::Arc::new(Mutex::new(Vec::new())) }
    }

    fn rounds(&self) -> usize {
        self.posts.lock().unwrap().len()
    }

    fn asked(&self, method: &str) -> usize {
        self.posts.lock().unwrap().iter().map(|(_, m)| m.iter().filter(|x| *x == method).count()).sum()
    }

    fn answer(&self, q: &In) -> String {
        let id = match q.get("id") {
            Some(In::Int(n)) => n.to_string(),
            Some(In::Str(s)) => format!("\"{s}\""),
            _ => "0".into(),
        };
        let method = q.get("method").and_then(In::as_str).unwrap_or_default();
        let params = q.get("params").and_then(In::as_arr).map(<[In]>::to_vec).unwrap_or_default();
        let s = |i: usize| params.get(i).and_then(In::as_str).unwrap_or_default().to_ascii_lowercase();
        let num = |h: &str| u64::from_str_radix(h.trim_start_matches("0x"), 16).unwrap_or(0);
        let result = match method {
            "eth_chainId" => "\"0x1\"".to_string(),
            "eth_blockNumber" => format!("\"0x{:x}\"", self.head),
            "eth_getBlockByNumber" => {
                let n = if s(0) == "latest" { self.head } else { num(&s(0)) };
                format!(r#"{{"number":"0x{n:x}","timestamp":"0x{:x}","hash":"0x{}{n:08x}","baseFeePerGas":"0x7"}}"#, 1_700_000_000 + n * 12, "bb".repeat(28))
            }
            "eth_getCode" => format!("\"0x60{}\"", &s(0)[40..]),
            "eth_getBalance" => format!("\"0x{}\"", &s(0)[38..]),
            "eth_getTransactionReceipt" => {
                let h = s(0);
                format!(r#"{{"transactionHash":"{h}","blockHash":"0x{}","status":"0x1","blockNumber":"0x{:x}","gasUsed":"0x5208","effectiveGasPrice":"0x1","logs":[]}}"#, "bb".repeat(32), 100 + num(&h[62..]))
            }
            "eth_getLogs" => {
                let f = params.first().cloned().unwrap_or(In::Null);
                let (from, to) = (num(f.get("fromBlock").and_then(In::as_str).unwrap_or("0x0")), num(f.get("toBlock").and_then(In::as_str).unwrap_or("0x0")));
                let addrs: Vec<String> = match f.get("address") {
                    Some(In::Str(a)) => vec![a.to_ascii_lowercase()],
                    Some(In::Arr(a)) => a.iter().filter_map(In::as_str).map(str::to_ascii_lowercase).collect(),
                    _ => Vec::new(),
                };
                let rows: Vec<String> = self
                    .logs
                    .iter()
                    .filter(|(a, n, _)| (addrs.is_empty() || addrs.contains(a)) && (from..=to).contains(n))
                    .map(|(a, n, t)| {
                        format!(r#"{{"address":"{a}","blockHash":"0x{}{n:08x}","blockNumber":"0x{n:x}","data":"0x","logIndex":"0x0","removed":false,"topics":["{t}"],"transactionHash":"0x{}{n:08x}","transactionIndex":"0x0"}}"#, "bb".repeat(28), "cc".repeat(28))
                    })
                    .collect();
                format!("[{}]", rows.join(","))
            }
            _ => "null".to_string(),
        };
        format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#)
    }
}

impl Transport for Node {
    fn post(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let v = eravon_literals::json::read(body).expect("问是 JSON");
        let (methods, out) = match &v {
            In::Arr(qs) => (
                qs.iter().map(|q| q.get("method").and_then(In::as_str).unwrap_or_default().to_string()).collect::<Vec<_>>(),
                format!("[{}]", qs.iter().map(|q| self.answer(q)).collect::<Vec<_>>().join(",")),
            ),
            q => (vec![q.get("method").and_then(In::as_str).unwrap_or_default().to_string()], self.answer(q)),
        };
        self.posts.lock().unwrap().push((url.port, methods));
        Ok(Answer { status: 200, location: None, body: out.into_bytes() })
    }
}

fn table(port: u16) -> Table {
    let t = Table::new(vec![format!("http://127.0.0.1:{port}"), format!("http://127.0.0.1:{}", port + 1)]).unwrap();
    let _ = t.chain.set(1);
    t
}

fn addr(i: usize) -> String {
    format!("0x{:040x}", 0xa000 + i)
}

fn hash(i: usize) -> String {
    format!("0x{}{i:04x}", "d".repeat(60))
}

fn quiet() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
}

#[test]
fn codes_asked_ahead_take_the_same_round_trips_for_twice_the_addresses() {
    quiet();
    let run = |port: u16, n: usize, ahead: bool| -> (usize, Vec<Vec<u8>>) {
        let t = Node::new(500, Vec::new());
        let tb = table(port);
        let _a = eravon_chain::action::open(None);
        let addrs: Vec<String> = (0..n).map(addr).collect();
        if ahead {
            eravon_chain::read::ahead_codes(&t, &tb, &addrs, eravon_chain::seam::TAG_LATEST);
        }
        let codes = addrs.iter().map(|a| eravon_chain::read::code(&t, &tb, a, eravon_chain::seam::TAG_LATEST).expect("码").0).collect();
        (t.rounds(), codes)
    };
    let (trips3, c3) = run(31501, 3, true);
    let (trips6, c6) = run(31503, 6, true);
    let (alone3, d3) = run(31505, 3, false);
    let (alone6, _) = run(31507, 6, false);
    note("codes_ahead", format!("ASKED codes ahead 3={trips3} 6={trips6}; one by one 3={alone3} 6={alone6}"));
    assert_eq!(trips3, trips6, "先问齐:地址加倍,往返不变");
    assert!(alone6 > alone3, "逐个问:往返随地址涨");
    assert_eq!(c3, d3, "先问齐与逐个问,答逐字相同");
    for (i, c) in c6.iter().enumerate() {
        assert_eq!(c.last().copied(), u8::from_str_radix(&addr(i)[40..], 16).ok(), "第 {i} 个地址答的是它自己的码");
    }
}

#[test]
fn receipts_then_blocks_ahead_take_the_same_round_trips_for_twice_the_transactions() {
    quiet();
    let run = |port: u16, n: usize| -> (usize, Vec<(u64, String)>) {
        let t = Node::new(500, Vec::new());
        let tb = table(port);
        let _a = eravon_chain::action::open(None);
        let hashes: Vec<String> = (0..n).map(hash).collect();
        eravon_chain::read::ahead_receipts(&t, &tb, &hashes);
        let blocks: Vec<u64> = hashes.iter().map(|h| eravon_chain::sending::receipt(&t, &tb, h).expect("回执").expect("已上链").block).collect();
        eravon_chain::read::ahead_blocks(&t, &tb, &blocks);
        let times = blocks
            .iter()
            .map(|b| {
                let r = eravon_chain::read::read_agreed(&t, &tb, eravon_chain::seam::M_GET_BLOCK, vec![eravon_literals::json::Out::S(eravon_chain::seam::quantity(*b)), eravon_literals::json::Out::B(false)]).expect("块");
                (*b, r.value.get("timestamp").and_then(In::as_str).unwrap_or_default().to_string())
            })
            .collect();
        (t.rounds(), times)
    };
    let (trips3, _) = run(31511, 3);
    let (trips6, got) = run(31513, 6);
    note("receipts_blocks_ahead", format!("ASKED receipts+blocks ahead 3={trips3} 6={trips6}"));
    assert_eq!(trips3, trips6, "回执与块先问齐:交易加倍,往返不变");
    assert_eq!(got.iter().map(|(b, _)| *b).collect::<Vec<_>>(), (0..6).map(|i| 100 + i as u64).collect::<Vec<_>>(), "各笔回执答的是它自己的块");
}

#[test]
fn the_refreshers_reads_asked_ahead_take_the_same_round_trips_for_three_times_the_wants() {
    quiet();
    let run = |port: u16, n: usize| -> (usize, BTreeMap<String, String>) {
        let t = Node::new(500, Vec::new());
        let tb = table(port);
        let _a = eravon_chain::action::open(None);
        let mut store = eravon_chain::recall::Store::fresh(1);
        store.set_head(500, 1_700_006_000, eravon_chain::read::Agreement::Agreed, 2);
        let params: Vec<String> = (0..n).map(|i| format!("[\"{}\",\"latest\"]", addr(i))).collect();
        eravon_chain::read::ask_ahead_raw(&t, &tb, eravon_chain::seam::M_GET_CODE, &params);
        for p in &params {
            store.fetch(&t, &tb, &eravon_chain::recall::Want::Read(eravon_chain::seam::M_GET_CODE.to_string(), p.clone())).expect("取");
        }
        let kept = store.answers.iter().filter(|(k, _)| k.starts_with(eravon_chain::seam::M_GET_CODE)).map(|(k, v)| (k.clone(), format!("{:?}", v.value))).collect();
        (t.rounds(), kept)
    };
    let (trips4, k4) = run(31521, 4);
    let (trips12, k12) = run(31523, 12);
    note("refresher_reads_ahead", format!("ASKED refresher reads ahead 4={trips4} 12={trips12}"));
    assert_eq!(trips4, trips12, "刷新器欠读先问齐:四问与十二问往返相同");
    assert_eq!(k4.len(), 4);
    assert_eq!(k12.len(), 12);
    for (k, v) in &k12 {
        let a = k.split('"').nth(1).expect("地址");
        assert!(v.contains(&format!("60{}", &a[40..])), "{k} 记的是它自己的答:{v}");
    }
}

#[test]
fn new_log_queries_of_one_shape_are_one_question_and_each_keeps_its_own() {
    quiet();
    let topic = format!("0x{}", "ee".repeat(32));
    let logs: Vec<(String, u64, String)> = (0..12).flat_map(|i| [(addr(i), 100 + i as u64, topic.clone()), (addr(i), 300 + i as u64, topic.clone())]).collect();
    let rest = |i: usize| format!(",\"address\":\"{}\",\"topics\":[\"{topic}\"]", addr(i));
    let from = |i: usize| if i % 2 == 0 { 0 } else { 200 };
    let run = |port: u16, n: usize, joined: bool| -> (usize, Vec<Vec<String>>) {
        let t = Node::new(500, logs.clone());
        let tb = table(port);
        let _a = eravon_chain::action::open(None);
        let mut store = eravon_chain::recall::Store::fresh(1);
        store.set_head(500, 1_700_006_000, eravon_chain::read::Agreement::Agreed, 2);
        let wants: Vec<(String, u64)> = (0..n).map(|i| (rest(i), from(i))).collect();
        if joined {
            let got = store.read_logs(&t, &tb, &wants);
            for ((r, f), g) in wants.iter().zip(got) {
                store.keep_logs(r, *f, g).expect("记");
            }
        } else {
            for (r, f) in &wants {
                store.fetch(&t, &tb, &eravon_chain::recall::Want::Logs(r.clone(), *f)).expect("取");
            }
        }
        let rows = wants.iter().map(|(r, _)| store.tracks.iter().find(|x| x.rest == *r).expect("记下了").rows.clone()).collect();
        (t.asked("eth_getLogs"), rows)
    };
    let (q4, _) = run(31531, 4, true);
    let (q12, rows12) = run(31533, 12, true);
    let (one12, alone12) = run(31535, 12, false);
    note("new_log_queries", format!("ASKED new log queries joined 4={q4} 12={q12}; alone 12={one12}"));
    assert_eq!(q4, q12, "同形新日志欠读:四条与十二条问数相同");
    assert!(one12 > q12, "逐条首读:问数随条数涨");
    assert_eq!(rows12, alone12, "并组读与逐条读,每条记下的日志逐字相同");
    assert!(rows12.iter().enumerate().all(|(i, r)| r.len() == if from(i) == 0 { 2 } else { 1 }), "各条只记自己起点之后的");
}

#[test]
fn side_by_side_answers_in_item_order_and_the_first_refusal_is_the_first_item_refused() {
    let items: Vec<usize> = (0..13).collect();
    let got = eravon_chain::read::side_by_side(items.clone(), |i| {
        std::thread::sleep(std::time::Duration::from_millis(((13 - i) * 3) as u64));
        if i == 4 || i == 9 { Err(format!("第 {i} 项拒")) } else { Ok(i * 10) }
    });
    assert_eq!(got.len(), 13);
    assert_eq!(got.iter().filter_map(|x| x.as_ref().ok().copied()).collect::<Vec<_>>(), items.iter().filter(|i| **i != 4 && **i != 9).map(|i| i * 10).collect::<Vec<_>>());
    let first: Result<Vec<usize>, String> = got.into_iter().collect();
    assert_eq!(first, Err("第 4 项拒".to_string()), "首拒是项序里的第一拒");
    let (a, b) = eravon_chain::read::both(
        || {
            std::thread::sleep(std::time::Duration::from_millis(20));
            Err::<u8, &str>("估气拒")
        },
        || Err::<u8, &str>("余额拒"),
    );
    assert_eq!(a.and(b), Err("估气拒"), "两路并排,判序照调用者的序:先到的不抢先");
}

#[derive(Clone)]
struct OneTracer {
    in_flight: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl Transport for OneTracer {
    fn post(&self, _url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        use std::sync::atomic::Ordering::SeqCst;
        let v = eravon_literals::json::read(body).expect("问是 JSON");
        let id = match v.get("id") {
            Some(In::Int(n)) => n.to_string(),
            _ => "0".into(),
        };
        let out = match v.get("method").and_then(In::as_str).unwrap_or_default() {
            "eth_chainId" => r#""result":"0x1""#.to_string(),
            "debug_traceTransaction" => {
                let others = self.in_flight.fetch_add(1, SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(20));
                self.in_flight.fetch_sub(1, SeqCst);
                if others > 0 {
                    r#""error":{"code":-32005,"message":"rate limit exceeded"}"#.to_string()
                } else {
                    r#""result":{"from":"0xe0","to":"0xc0","input":"0x","type":"CALL","calls":[{"from":"0xc0","to":"0xaa","input":"0x1111111100","type":"CALL"}]}"#.to_string()
                }
            }
            _ => r#""error":{"code":-32601,"message":"the method does not exist"}"#.to_string(),
        };
        Ok(Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0",{out}}}"#).into_bytes() })
    }
}

fn traced(port: u16, warm: bool) -> Vec<String> {
    quiet();
    let t = OneTracer { in_flight: Default::default() };
    let tb = Table::new(vec![format!("http://127.0.0.1:{port}")]).unwrap();
    let _ = tb.chain.set(1);
    let _a = eravon_chain::action::open(None);
    let txs: Vec<String> = (0..6).map(|i| format!("0x{}{i:04x}", "f".repeat(60))).collect();
    if warm {
        eravon_chain::trace::ahead(&t, &tb, &txs);
    }
    txs.iter().map(|h| format!("{:?}", eravon_chain::trace::find(&t, &tb, h, "0xaa", "0x11111111").expect("追踪").0)).collect()
}

#[test]
fn tracing_ahead_never_teaches_the_endpoint_that_traces_none() {
    let warmed = traced(31811, true);
    let plain = traced(31813, false);
    assert_eq!(warmed, plain, "预热之后各笔的追踪与不预热逐字相同");
    assert!(warmed.iter().all(|x| x.starts_with("Found")), "供追踪的那一家仍供:{warmed:?}");
}

fn note(test: &str, line: String) {
    println!("{line}");
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("asked");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(format!("{test}.txt")), format!("{line}\n"));
}
