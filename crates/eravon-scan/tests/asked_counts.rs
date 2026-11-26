use eravon_chain::endpoints::Table;
use eravon_chain::transport::{Answer, Transport};
use eravon_galeed::Value;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct Count {
    head: u64,
    logs: Vec<(String, u64, Vec<String>)>,
    asked: std::sync::Arc<Mutex<Vec<String>>>,
    blocks: std::sync::Arc<Mutex<u64>>,
}

impl Count {
    fn n(&self, method: &str) -> usize {
        self.asked.lock().unwrap().iter().filter(|m| *m == method).count()
    }
}

impl Transport for Count {
    fn post(&self, _url: &eravon_chain::seam::Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let b = String::from_utf8_lossy(body).to_string();
        let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
        let method = b.split("\"method\":\"").nth(1).and_then(|x| x.split('"').next()).unwrap_or("").to_string();
        self.asked.lock().unwrap().push(method.clone());
        let q = |k: &str| b.split(&format!("\"{k}\":\"0x")).nth(1).and_then(|x| x.split('"').next()).and_then(|h| u64::from_str_radix(h, 16).ok()).unwrap_or(0);
        let result = match method.as_str() {
            "eth_blockNumber" => format!("\"0x{:x}\"", self.head),
            "eth_chainId" => "\"0x1\"".to_string(),
            "eth_getBlockByNumber" => {
                let n = b.split("\"params\":[\"0x").nth(1).and_then(|x| x.split('"').next()).and_then(|h| u64::from_str_radix(h, 16).ok()).unwrap_or(0);
                format!(r#"{{"hash":"0x{}{n:08x}","number":"0x{n:x}"}}"#, "bb".repeat(28))
            }
            "eth_getLogs" => {
                let (from, to) = (q("fromBlock"), q("toBlock"));
                *self.blocks.lock().unwrap() += to.saturating_sub(from) + 1;
                let rows: Vec<String> = self
                    .logs
                    .iter()
                    .filter(|(a, n, _)| b.contains(a.as_str()) && (from..=to).contains(n))
                    .map(|(a, n, t)| {
                        let topics: Vec<String> = t.iter().map(|x| format!("\"{x}\"")).collect();
                        format!(r#"{{"address":"{a}","blockHash":"0x{}{n:08x}","blockNumber":"0x{n:x}","data":"0x","logIndex":"0x0","topics":[{}],"transactionHash":"0x{}{n:08x}"}}"#, "bb".repeat(28), topics.join(","), "cc".repeat(28))
                    })
                    .collect();
                format!("[{}]", rows.join(","))
            }
            _ => "null".to_string(),
        };
        Ok(Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
    }
}

#[derive(Default)]
struct Record(Mutex<std::collections::BTreeMap<String, Vec<u8>>>);

impl eravon_chain::settled::Keep for Record {
    fn load(&self, part: &str) -> Option<Vec<u8>> {
        self.0.lock().unwrap().get(part).cloned()
    }
    fn save(&self, part: &str, plain: &[u8]) -> Result<(), eravon_chain::settled::Unkept> {
        self.0.lock().unwrap().insert(part.to_string(), plain.to_vec());
        Ok(())
    }
    fn parts(&self) -> Vec<String> {
        self.0.lock().unwrap().keys().cloned().collect()
    }
}

#[derive(Default)]
struct Room(Mutex<std::collections::BTreeMap<String, Vec<u8>>>);

impl eravon_send::checked::Room for Room {
    type Fail = eravon_seal::Trouble;
    fn read(&self, file: &str) -> Option<Vec<u8>> {
        self.0.lock().unwrap().get(file).cloned()
    }
    fn put(&self, file: &str, plain: &[u8]) -> Result<(), eravon_seal::Trouble> {
        self.0.lock().unwrap().insert(file.to_string(), plain.to_vec());
        Ok(())
    }
}

fn table(port: u16) -> Table {
    let t = Table::new(vec![format!("http://127.0.0.1:{port}")]).unwrap();
    let _ = t.chain.set(1);
    t
}

fn addr(i: usize) -> String {
    format!("0x{:040x}", 0xa000 + i)
}

#[test]
fn a_log_read_asked_again_asks_only_what_is_new() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let logs = (0..40).map(|i| (addr(0), 1_000 + i * 50, Vec::new())).collect::<Vec<_>>();
    let run = |keep: bool| -> (usize, u64) {
        let tb = table(30301);
        let record: Arc<Record> = Default::default();
        let (mut asks, mut blocks) = (0, 0);
        for head in [4_000u64, 4_100, 4_200] {
            let t = Count { head, logs: logs.clone(), asked: Default::default(), blocks: Default::default() };
            let _a = eravon_chain::action::open(keep.then(|| record.clone() as Arc<dyn eravon_chain::settled::Keep>));
            let (rows, _) = eravon_chain::read::logs(&t, &tb, &addr(0), 0, eravon_chain::seam::Upto::Latest).expect("read");
            assert_eq!(rows.len(), 40);
            asks += t.n("eth_getLogs");
            blocks += *t.blocks.lock().unwrap();
        }
        (asks, blocks)
    };
    let (b_asks, b_blocks) = run(false);
    let (a_asks, a_blocks) = run(true);
    println!("ASKED log-read-x3 eth_getLogs before={b_asks} blocks={b_blocks} after={a_asks} blocks={a_blocks} (and one block-hash check per kept read)");
    assert!(a_blocks < b_blocks);
}

#[test]
fn an_orders_page_reads_its_instances_events_in_one_question() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let n = 20;
    let logs = (0..n).map(|i| (addr(i), 2_000 + i as u64, Vec::new())).collect::<Vec<_>>();
    let tb = table(30302);
    let before = {
        let t = Count { head: 3_000, logs: logs.clone(), asked: Default::default(), blocks: Default::default() };
        let _a = eravon_chain::action::open(None);
        for i in 0..n {
            let _ = eravon_chain::read::logs(&t, &tb, &addr(i), 2_000, eravon_chain::seam::Upto::Latest).expect("read");
        }
        t.n("eth_getLogs")
    };
    let after = {
        let t = Count { head: 3_000, logs, asked: Default::default(), blocks: Default::default() };
        let _a = eravon_chain::action::open(None);
        let all: Vec<String> = (0..n).map(addr).collect();
        let (rows, _) = eravon_chain::read::logs_at_any(&t, &tb, &all, 2_000, eravon_chain::seam::Upto::Latest).expect("read");
        assert_eq!(rows.len(), n);
        t.n("eth_getLogs")
    };
    println!("ASKED orders-page-events-{n} eth_getLogs before={before} after={after}");
    assert!(after < before);
}

struct Kits(Mutex<usize>);

impl eravon_scan::ledger::Evidence for Kits {
    fn tx(&self, _chain_id: u64, _tx: &str) -> eravon_scan::fault::R<Option<Value>> {
        *self.0.lock().unwrap() += 1;
        Ok(Some(Value::Obj(vec![("hash".to_string(), Value::Str(format!("0x{}", "ee".repeat(32))))])))
    }
    fn status(&self, _tx: &str) -> eravon_scan::fault::R<Option<bool>> {
        Ok(Some(true))
    }
    fn time(&self, _chain_id: u64, _block_hash: &str, number: u64) -> eravon_scan::fault::R<u64> {
        Ok(1_700_000_000 + number)
    }
    fn code_at(&self, _chain_id: u64, _block_hash: &str, _who: &str, _number: u64) -> Option<bool> {
        Some(false)
    }

    fn code_hash_at(&self, _addr: &str, _block: u64) -> eravon_scan::fault::R<String> {
        Ok(format!("0x{}", "cc".repeat(32)))
    }
}

#[test]
fn a_ledger_opened_again_gathers_no_evidence_it_judged() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let reg = addr(99);
    let root = addr(98);
    let sender = format!("0x{}{}", "0".repeat(24), &root[2..]);
    let anchored = eravon_scan::scan::anchored_topic0();
    let logs: Vec<(String, u64, Vec<String>)> = (0..30).map(|i| (reg.clone(), 100 + i, vec![anchored.clone(), sender.clone(), format!("0x{:064x}", i + 1)])).collect();
    let w = eravon_scan::ledger::Window { chain_id: 31_337, from: 0, to: 1_000, registries: vec![eravon_scan::ledger::Emitter::Address(reg.clone())], senders: vec![root.clone()] };
    let basis = eravon_scan::ledger::basis_of(&[w]);
    let tb = table(30303);
    let opens = |keep: bool| -> Vec<usize> {
        let record: Arc<Record> = Default::default();
        eravon_scan::judged::keep_in(Some(Arc::new(Room::default())));
        let mut kits = Vec::new();
        for _ in 0..2 {
            let t = Count { head: 1_000, logs: logs.clone(), asked: Default::default(), blocks: Default::default() };
            let ev = Kits(Mutex::new(0));
            let _a = eravon_chain::action::open(keep.then(|| record.clone() as Arc<dyn eravon_chain::settled::Keep>));
            let l = eravon_scan::ledger::read(&t, &tb, &ev, 31_337, &root, &basis, 1_000).expect("read");
            assert_eq!(l.not_anchors.len(), 30);
            kits.push(ev.0.into_inner().unwrap());
        }
        eravon_scan::judged::keep_in(None);
        kits
    };
    let before = opens(false);
    let after = opens(true);
    println!("ASKED ledger-30-anchors-opened-twice kits before={before:?} after={after:?} (a kit: four agreed questions of every node on a live chain)");
    assert_eq!(after[1], 0);
}

#[derive(Clone)]
struct Logic {
    head: std::sync::Arc<Mutex<u64>>,
    logic: String,
    logs: std::sync::Arc<Mutex<Vec<(u64, Vec<String>)>>>,
    ranges: std::sync::Arc<Mutex<Vec<(u64, u64)>>>,
}

impl Transport for Logic {
    fn post(&self, _url: &eravon_chain::seam::Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let req = eravon_literals::json::read(body).expect("问是 JSON");
        let method = req.get("method").and_then(eravon_literals::json::In::as_str).unwrap_or_default().to_string();
        let id = match req.get("id") {
            Some(eravon_literals::json::In::Int(n)) => n.to_string(),
            Some(eravon_literals::json::In::Str(s)) => format!("\"{s}\""),
            _ => "1".into(),
        };
        let p = req.get("params").and_then(eravon_literals::json::In::as_arr).map(<[eravon_literals::json::In]>::to_vec).unwrap_or_default();
        let num = |h: &str| u64::from_str_radix(h.trim_start_matches("0x"), 16).unwrap_or(0);
        let head = *self.head.lock().unwrap();
        let result = match method.as_str() {
            "eth_chainId" => "\"0x7a69\"".to_string(),
            "eth_blockNumber" => format!("\"0x{head:x}\""),
            "eth_getBlockByNumber" => {
                let n = p.first().and_then(eravon_literals::json::In::as_str).map(num).unwrap_or(head);
                format!(r#"{{"hash":"0x{}{n:08x}","number":"0x{n:x}"}}"#, "bb".repeat(28))
            }
            "eth_getLogs" => {
                let f = &p[0];
                let (lo, hi) = (num(f.get("fromBlock").and_then(eravon_literals::json::In::as_str).unwrap()), num(f.get("toBlock").and_then(eravon_literals::json::In::as_str).unwrap()));
                self.ranges.lock().unwrap().push((lo, hi));
                let want: Vec<Vec<String>> = f.get("topics").and_then(eravon_literals::json::In::as_arr).map(|a| a.iter().map(|x| match x {
                    eravon_literals::json::In::Str(s) => vec![s.to_ascii_lowercase()],
                    eravon_literals::json::In::Arr(c) => c.iter().filter_map(eravon_literals::json::In::as_str).map(str::to_ascii_lowercase).collect(),
                    _ => Vec::new(),
                }).collect()).unwrap_or_default();
                let rows: Vec<String> = self
                    .logs
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|(b, t)| (lo..=hi).contains(b) && want.iter().enumerate().all(|(i, c)| c.is_empty() || t.get(i).is_some_and(|x| c.contains(x))))
                    .map(|(b, t)| {
                        let topics: Vec<String> = t.iter().map(|x| format!("\"{x}\"")).collect();
                        format!(r#"{{"address":"{}","topics":[{}],"data":"0x","blockNumber":"0x{b:x}","logIndex":"0x0","transactionHash":"0x{}{b:08x}","blockHash":"0x{}{b:08x}"}}"#, self.logic, topics.join(","), "cc".repeat(28), "bb".repeat(28))
                    })
                    .collect();
                format!("[{}]", rows.join(","))
            }
            _ => "null".to_string(),
        };
        Ok(Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
    }
}

#[derive(Default)]
struct Settled(Mutex<std::collections::BTreeMap<String, Vec<u8>>>);

impl eravon_chain::settled::Keep for Settled {
    fn load(&self, part: &str) -> Option<Vec<u8>> {
        self.0.lock().unwrap().get(part).cloned()
    }
    fn save(&self, part: &str, plain: &[u8]) -> Result<(), eravon_chain::settled::Unkept> {
        self.0.lock().unwrap().insert(part.to_string(), plain.to_vec());
        Ok(())
    }
    fn parts(&self) -> Vec<String> {
        self.0.lock().unwrap().keys().cloned().collect()
    }
}

fn topic(n: u64) -> String {
    format!("0x{n:064x}")
}

const SELLER: &str = "0x00000000000000000000000000000000000000a1";
const BUYER: &str = "0x00000000000000000000000000000000000000b1";
const LOGIC: &str = "0x00000000000000000000000000000000000000ee";

fn records(orders: u64) -> Vec<(u64, Vec<String>)> {
    let ev = |e: eravon_standard::sol::Ev| eravon_standard::order::hex_of(&e.topic0());
    let (s, b) = (eravon_scan::log::address_topic(SELLER).unwrap(), eravon_scan::log::address_topic(BUYER).unwrap());
    let mut out = Vec::new();
    for i in 0..orders {
        let d = topic(0xd000 + i);
        out.push((100 + i * 10, vec![ev(eravon_standard::sol::CONSENTED_EV), s.clone(), d.clone()]));
        out.push((101 + i * 10, vec![ev(eravon_standard::sol::CONSENTED_EV), b.clone(), d.clone()]));
        out.push((102 + i * 10, vec![ev(eravon_standard::sol::PULLED_EV), b.clone(), d.clone()]));
        if i % 2 == 1 {
            out.push((103 + i * 10, vec![ev(eravon_standard::sol::REVOKED_EV), s.clone(), d.clone()]));
        }
    }
    out.push((99, vec![ev(eravon_standard::sol::FUND_BOUND_EV), s.clone(), topic(0xf0)]));
    out.push((98, vec![ev(eravon_standard::sol::CONSENTED_EV), topic(0xc1), topic(0xd000)]));
    out
}

fn table_at(host: &str) -> Table {
    Table::new(vec![format!("http://{host}:1")]).expect("端点表")
}

#[test]
fn the_pairs_records_are_one_question_whatever_the_orders() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let consented = eravon_standard::sol::CONSENTED_EV.topic0();
    let run = |host: &str, orders: u64| -> (usize, Vec<usize>) {
        let t = Logic { head: std::sync::Arc::new(Mutex::new(5_000)), logic: LOGIC.into(), logs: std::sync::Arc::new(Mutex::new(records(orders))), ranges: Default::default() };
        let tb = table_at(host);
        let _a = eravon_chain::action::open(None);
        let got = eravon_scan::scan::logic_records(&t, &tb, LOGIC, &[SELLER.into(), BUYER.into()], eravon_chain::seam::Upto::Latest).expect("读得成");
        let per: Vec<usize> = (0..orders)
            .map(|i| eravon_scan::scan::picked(&got, &consented, &[Some(eravon_scan::log::address_topic(BUYER).unwrap()), Some(topic(0xd000 + i))]).len())
            .collect();
        (t.ranges.lock().unwrap().len(), per)
    };
    let (q2, p2) = run("127.0.9.1", 2);
    let (q6, p6) = run("127.0.9.2", 6);
    note("logic_records", format!("ASKED logic records orders 2={q2} 6={q6}"));
    assert_eq!((q2, q6), (1, 1), "一对钥一问,订单数不涨问数");
    assert!(p2.iter().chain(&p6).all(|n| *n == 1), "每单的买家同意各取得恰一条:{p2:?} {p6:?}");
}

#[test]
fn the_next_order_of_the_same_pair_asks_only_the_tail() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let t = Logic { head: std::sync::Arc::new(Mutex::new(5_000)), logic: LOGIC.into(), logs: std::sync::Arc::new(Mutex::new(records(3))), ranges: Default::default() };
    let tb = table_at("127.0.9.3");
    let record: Arc<Settled> = Default::default();
    {
        let _a = eravon_chain::action::open(Some(record.clone()));
        eravon_scan::scan::logic_records(&t, &tb, LOGIC, &[SELLER.into(), BUYER.into()], eravon_chain::seam::Upto::Latest).expect("头一单");
    }
    *t.head.lock().unwrap() = 5_100;
    {
        let _a = eravon_chain::action::open(Some(record.clone()));
        eravon_scan::scan::logic_records(&t, &tb, LOGIC, &[BUYER.into(), SELLER.into()], eravon_chain::seam::Upto::Latest).expect("第二单");
    }
    let ranges = t.ranges.lock().unwrap().clone();
    note("logic_records_tail", format!("ASKED logic records second order ranges {ranges:?}"));
    assert_eq!(ranges.len(), 2);
    assert_eq!(ranges[1], (5_000 - eravon_chain::settled::MARGIN + 1, 5_100), "第二单只问头一单落定之后到新头");
}

#[derive(Clone)]
struct Board {
    logs: Vec<(String, u64, Vec<String>)>,
    posts: std::sync::Arc<Mutex<usize>>,
    log_questions: std::sync::Arc<Mutex<usize>>,
}

impl Board {
    fn answer(&self, q: &eravon_literals::json::In) -> String {
        use eravon_literals::json::In;
        let id = match q.get("id") {
            Some(In::Int(n)) => n.to_string(),
            Some(In::Str(s)) => format!("\"{s}\""),
            _ => "0".into(),
        };
        let p = q.get("params").and_then(In::as_arr).map(<[In]>::to_vec).unwrap_or_default();
        let num = |h: &str| u64::from_str_radix(h.trim_start_matches("0x"), 16).unwrap_or(0);
        let result = match q.get("method").and_then(In::as_str).unwrap_or_default() {
            "eth_chainId" => "\"0x7a69\"".to_string(),
            "eth_blockNumber" => "\"0x3e8\"".to_string(),
            "eth_getBlockByNumber" => r#"{"number":"0x3e8","hash":"0x1111111111111111111111111111111111111111111111111111111111111111"}"#.to_string(),
            "eth_getCode" => format!("\"0x60{}\"", &p[0].as_str().unwrap_or_default()[40..]),
            "eth_getLogs" => {
                *self.log_questions.lock().unwrap() += 1;
                let f = &p[0];
                let (lo, hi) = (num(f.get("fromBlock").and_then(In::as_str).unwrap()), num(f.get("toBlock").and_then(In::as_str).unwrap()));
                let addr = f.get("address").and_then(In::as_str).map(str::to_ascii_lowercase);
                let rows: Vec<String> = self
                    .logs
                    .iter()
                    .filter(|(a, n, _)| addr.as_ref().is_none_or(|x| x == a) && (lo..=hi).contains(n))
                    .map(|(a, n, t)| {
                        let topics: Vec<String> = t.iter().map(|x| format!("\"{x}\"")).collect();
                        format!(r#"{{"address":"{a}","blockHash":"0x{}{n:08x}","blockNumber":"0x{n:x}","data":"0x","logIndex":"0x0","topics":[{}],"transactionHash":"0x{}{n:08x}"}}"#, "bb".repeat(28), topics.join(","), "cc".repeat(28))
                    })
                    .collect();
                format!("[{}]", rows.join(","))
            }
            _ => "null".to_string(),
        };
        format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#)
    }
}

impl Transport for Board {
    fn post(&self, _url: &eravon_chain::seam::Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        *self.posts.lock().unwrap() += 1;
        let v = eravon_literals::json::read(body).expect("问是 JSON");
        let out = match &v {
            eravon_literals::json::In::Arr(qs) => format!("[{}]", qs.iter().map(|q| self.answer(q)).collect::<Vec<_>>().join(",")),
            q => self.answer(q),
        };
        Ok(Answer { status: 200, location: None, body: out.into_bytes() })
    }
}

struct Codes<'a> {
    t: &'a Board,
    tb: &'a Table,
}

impl eravon_scan::ledger::Evidence for Codes<'_> {
    fn tx(&self, _chain_id: u64, _tx: &str) -> eravon_scan::fault::R<Option<Value>> {
        Ok(Some(Value::Obj(vec![("hash".to_string(), Value::Str(format!("0x{}", "ee".repeat(32))))])))
    }
    fn status(&self, _tx: &str) -> eravon_scan::fault::R<Option<bool>> {
        Ok(Some(true))
    }
    fn time(&self, _chain_id: u64, _block_hash: &str, number: u64) -> eravon_scan::fault::R<u64> {
        Ok(1_700_000_000 + number)
    }
    fn code_at(&self, _chain_id: u64, _block_hash: &str, _who: &str, _number: u64) -> Option<bool> {
        Some(false)
    }

    fn code_hash_at(&self, addr: &str, block: u64) -> eravon_scan::fault::R<String> {
        let (code, _) = eravon_chain::read::code(self.t, self.tb, addr, &eravon_chain::seam::quantity(block)).map_err(eravon_scan::fault::Fault::from)?;
        Ok(eravon_standard::order::hex_of(&eravon_terms::derive::keccak(&code)))
    }
    fn ahead_codes(&self, wants: &[(String, u64)]) {
        eravon_chain::read::ahead_codes_by(self.t, self.tb, wants);
    }
}

#[test]
fn a_ledger_with_class_items_asks_once_whatever_the_anchors() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let root = addr(198);
    let sender = format!("0x{}{}", "0".repeat(24), &root[2..]);
    let anchored = eravon_scan::scan::anchored_topic0();
    let run = |port: u16, n: usize| -> (usize, usize, usize) {
        let reg = addr(300 + port as usize);
        let mut logs: Vec<(String, u64, Vec<String>)> = vec![(reg.clone(), 90, vec![anchored.clone(), sender.clone(), format!("0x{:064x}", 7)])];
        logs.extend((0..n).map(|i| (addr(1_000 * port as usize + i), 100 + i as u64, vec![anchored.clone(), sender.clone(), format!("0x{:064x}", i + 1)])));
        let t = Board { logs, posts: Default::default(), log_questions: Default::default() };
        let tb = Table::new(vec![format!("http://127.0.0.1:{port}"), format!("http://127.0.0.1:{}", port + 1)]).unwrap();
        let w = eravon_scan::ledger::Window {
            chain_id: 31_337,
            from: 0,
            to: 1_000,
            registries: vec![eravon_scan::ledger::Emitter::Address(reg), eravon_scan::ledger::Emitter::Class(format!("0x{}", "cc".repeat(32)))],
            senders: vec![root.clone()],
        };
        let ev = Codes { t: &t, tb: &tb };
        let _a = eravon_chain::action::open(None);
        let l = eravon_scan::ledger::read(&t, &tb, &ev, 31_337, &root, &eravon_scan::ledger::basis_of(&[w]), 1_000).expect("read");
        assert_eq!(l.unchecked.len(), 0, "{:?}", l.unchecked);
        let questions = *t.log_questions.lock().unwrap();
        let posts = *t.posts.lock().unwrap();
        (questions, posts, l.not_anchors.len() + l.records.len())
    };
    let (q2, trips2, _) = run(30411, 2);
    let (q6, trips6, _) = run(30421, 6);
    note("ledger_class", format!("ASKED ledger-class anchors 2: log questions={q2} round trips={trips2}; 6: log questions={q6} round trips={trips6}"));
    assert_eq!(q2, q6, "类目项在时日志只一问(每端点一回),锚数不涨");
    assert_eq!(trips2, trips6, "候选 2 个与 6 个往返相同");
}

fn note(test: &str, line: String) {
    println!("{line}");
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("asked");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(format!("{test}.txt")), format!("{line}\n"));
}
