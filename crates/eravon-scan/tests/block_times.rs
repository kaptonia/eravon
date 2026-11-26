use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use eravon_chain::endpoints::Table;
use eravon_chain::transport::{Answer, Transport};
use eravon_literals::json::In;

#[derive(Clone)]
struct Chain {
    head: u64,
    asked: std::sync::Arc<Mutex<Vec<u64>>>,
}

impl Chain {
    fn new(head: u64) -> Chain {
        Chain { head, asked: std::sync::Arc::new(Mutex::new(Vec::new())) }
    }

    fn dated(&self) -> Vec<u64> {
        self.asked.lock().unwrap().iter().copied().filter(|n| *n < self.head).collect()
    }

    fn answer(&self, q: &In) -> String {
        let id = match q.get("id") {
            Some(In::Int(n)) => n.to_string(),
            _ => "0".into(),
        };
        let method = q.get("method").and_then(In::as_str).unwrap_or_default();
        let params = q.get("params").and_then(In::as_arr).map(<[In]>::to_vec).unwrap_or_default();
        let s = |i: usize| params.get(i).and_then(In::as_str).unwrap_or_default().to_ascii_lowercase();
        let result = match method {
            "eth_chainId" => "\"0x1\"".to_string(),
            "eth_blockNumber" => format!("\"0x{:x}\"", self.head),
            "eth_getBlockByNumber" => {
                let n = if s(0) == "latest" { self.head } else { u64::from_str_radix(s(0).trim_start_matches("0x"), 16).unwrap_or(0) };
                self.asked.lock().unwrap().push(n);
                format!(r#"{{"hash":"{}","number":"0x{n:x}","timestamp":"0x{:x}","baseFeePerGas":"0x7"}}"#, hash(n), 1_700_000_000 + n * 12)
            }
            _ => "null".to_string(),
        };
        format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#)
    }
}

impl Transport for Chain {
    fn post(&self, _url: &eravon_chain::seam::Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let v = eravon_literals::json::read(body).expect("问是 JSON");
        let out = match &v {
            In::Arr(qs) => format!("[{}]", qs.iter().map(|q| self.answer(q)).collect::<Vec<_>>().join(",")),
            q => self.answer(q),
        };
        Ok(Answer { status: 200, location: None, body: out.into_bytes() })
    }
}

fn hash(n: u64) -> String {
    format!("0x{}{n:08x}", "bb".repeat(28))
}

#[derive(Default)]
struct Room(Mutex<BTreeMap<String, Vec<u8>>>);

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

#[derive(Default)]
struct Keeper(Mutex<std::collections::BTreeMap<String, Vec<u8>>>);

impl eravon_chain::settled::Keep for Keeper {
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

fn table() -> Table {
    let t = Table::new(vec!["http://127.0.0.1:31601".to_string(), "http://127.0.0.1:31602".to_string()]).unwrap();
    let _ = t.chain.set(1);
    t
}

fn step(head: u64, blocks: &[u64]) -> (Vec<u64>, Vec<u64>) {
    let t = Chain::new(head);
    let tb = table();
    let _a = eravon_chain::action::open(Some(Arc::new(Keeper::default())));
    eravon_scan::scan::ahead_stamps(&t, &tb, blocks);
    let times = blocks.iter().map(|b| eravon_scan::scan::block_stamp(&t, &tb, *b).expect("时刻")).collect();
    (times, t.dated())
}

#[test]
fn a_settled_blocks_time_is_asked_once_and_new_blocks_only_after() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    eravon_scan::judged::keep_in(Some(Arc::new(Room::default())));
    let old: Vec<u64> = (100..120).collect();
    let (times1, asked1) = step(1_000, &old);
    let mut more = old.clone();
    more.extend([900, 990]);
    let (times2, asked2) = step(1_010, &more);
    let (times3, asked3) = step(1_020, &more);
    println!("ASKED block-times first={} second={} third={}", asked1.len(), asked2.len(), asked3.len());
    for (b, at) in more.iter().zip(&times2) {
        assert_eq!(*at, 1_700_000_000 + b * 12, "第 {b} 块的时刻是链上的");
    }
    assert_eq!(times1, times2[..old.len()].to_vec());
    assert_eq!(times3, times2);
    let mut a1 = asked1.clone();
    a1.sort_unstable();
    a1.dedup();
    assert_eq!(a1, old, "头一趟:二十个老块各问一次(两家)");
    let mut a2 = asked2.clone();
    a2.sort_unstable();
    a2.dedup();
    assert_eq!(a2, vec![900, 990], "第二趟:只问新出现的两块");
    let mut a3 = asked3.clone();
    a3.sort_unstable();
    a3.dedup();
    assert_eq!(a3, vec![990], "第三趟:离链头不足落定深度的那一块照问,已落定的不问");
    let t = Chain::new(1_020);
    let tb = table();
    let _a = eravon_chain::action::open(Some(Arc::new(Keeper::default())));
    assert_eq!(eravon_scan::scan::block_stamp_at(&t, &tb, 100, Some(&hash(100))).expect("同哈希"), 1_700_000_000 + 1_200);
    assert!(t.dated().is_empty(), "同哈希:不问");
    let other = format!("0x{}", "ee".repeat(32));
    let _ = eravon_scan::scan::block_stamp_at(&t, &tb, 100, Some(&other));
    assert_eq!(t.dated(), vec![100, 100], "留下的块哈希对不上:两家重问");
    eravon_scan::judged::keep_in(None);
}
