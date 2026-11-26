use std::sync::Mutex;

use eravon_chain::endpoints::Table;
use eravon_chain::seam::{Upto, Url};
use eravon_chain::transport::{Answer, Transport};
use eravon_literals::constants as k7;
use eravon_literals::json::In;

#[derive(Clone)]
struct Tally {
    chain: u64,
    head: u64,
    asked: std::sync::Arc<Mutex<Vec<(String, u64)>>>,
}

impl Tally {
    fn answer_one(&self, _url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let req = eravon_literals::json::read(body).expect("问是 JSON");
        let method = req.get("method").and_then(In::as_str).unwrap_or_default().to_string();
        let id = match req.get("id") {
            Some(In::Int(n)) => n.to_string(),
            Some(In::Str(s)) => format!("\"{s}\""),
            _ => "1".into(),
        };
        let result = match method.as_str() {
            "eth_chainId" => format!("\"0x{:x}\"", self.chain),
            "eth_blockNumber" => format!("\"0x{:x}\"", self.head),
            "eth_getCode" => "\"0x\"".into(),
            "eth_getLogs" => {
                let q = &req.get("params").and_then(In::as_arr).expect("params")[0];
                let from = u64::from_str_radix(q.get("fromBlock").and_then(In::as_str).expect("fromBlock").trim_start_matches("0x"), 16).expect("块号");
                let filter = format!("{:?}{:?}", q.get("address"), q.get("topics"));
                self.asked.lock().unwrap().push((filter, from));
                "[]".into()
            }
            _ => "null".into(),
        };
        Ok(Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
    }
}

impl Transport for Tally {
    fn post(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        eravon_chain::ask::each_question(body, |b| self.answer_one(url, b))
    }
}

fn word(b: u8) -> String {
    format!("0x{}", format!("{b:02x}").repeat(20))
}

#[test]
fn every_record_read_starts_at_the_network_floor() {
    for (n, (chain, floor)) in [(11_155_111u64, 11_998_635u64), (1, 0)].into_iter().enumerate() {
        assert!(k7::select_chain(chain), "链 {chain} 在网络表里");
        assert_eq!(k7::records_floor(), floor);
        let t = Tally { chain, head: 12_100_000, asked: std::sync::Arc::new(Mutex::new(Vec::new())) };
        let table = Table::new((1..=2).map(|i| format!("http://127.0.{}.{i}:1", 7 + n)).collect()).expect("端点表");
        let logic = k7::active_standard().logic;
        eravon_scan::records::for_listing(&t, &table, chain, &[7u8; 32]).expect("付款前核的记录集");
        eravon_scan::records::for_record(&t, &table, chain, logic, &word(0xa1)).expect("一把钥的记录集");
        eravon_chain::read::logs(&t, &table, &word(0xc1), 0, Upto::Latest).expect("实例的日志");
        let mut first: Vec<(String, u64)> = Vec::new();
        for (f, from) in t.asked.lock().unwrap().iter() {
            if !first.iter().any(|(g, _)| g == f) {
                first.push((f.clone(), *from));
            }
        }
        assert_eq!(first.len(), 7, "七种读各问到:{first:#?}");
        assert!(first.iter().all(|(_, from)| *from == floor), "链 {chain} 每种读自 {floor} 起:{first:#?}");

        t.asked.lock().unwrap().clear();
        let class = eravon_scan::ledger::Emitter::Class(k7::active_standard().proxy_code_hash.to_ascii_lowercase());
        let w = eravon_scan::ledger::Window::to_head(chain, t.head, vec![class.clone()], vec![word(0xd1)]);
        assert_eq!((w.from, w.to), (floor, t.head));
        assert_eq!(eravon_scan::ledger::Window::to_head(chain, 5, vec![class], vec![word(0xd1)]).from, if floor > 5 { 0 } else { floor }, "链头低于下界即不抬");
        let registry_from = k7::judge_registries().get().and_then(|rs| rs.iter().map(|r| r.from_block).min()).expect("这一网的登记簿");
        let ledger_from = floor.max(registry_from);
        let tall = Tally { chain, head: t.head.max(ledger_from + 1_000), asked: t.asked.clone() };
        let w = eravon_scan::ledger::Window { to: tall.head, ..w };
        let ev = eravon_scan::evidence::OnChain { t: &tall, table: &table };
        eravon_scan::ledger::read(&tall, &table, &ev, chain, &word(0xd1), &eravon_scan::ledger::basis_of(&[w]), tall.head).expect("判官账本读得成");
        let asked = t.asked.lock().unwrap().clone();
        assert!(!asked.is_empty() && asked.iter().all(|(_, from)| *from == ledger_from), "链 {chain} 读者自拼的账本窗自记录下界与登记簿起始块里高的那一个 {ledger_from} 起:{asked:#?}");
        assert_eq!(ledger_from, if chain == 1 { 26_496_122 } else { floor }, "主网自登记簿的部署块起,Sepolia 自记录下界起");

        use eravon_chain::recall::{self, Recall, Store, Want};
        let mut store = Store::fresh(chain);
        store.head = t.head;
        let rec = Recall::over(std::sync::Arc::new(store.clone()));
        let topic = format!("0x{}", "ab".repeat(32));
        recall::within(&rec, || {
            let _ = eravon_chain::read::logs_any_whole(&t, &table, Some(&word(0xe1)), &[vec![topic.clone()]], 5, Upto::Latest);
            let _ = eravon_chain::read::logs(&t, &table, &word(0xe2), 0, Upto::Latest);
        });
        let wanted = rec.take_wanted();
        let froms: Vec<u64> = wanted.iter().map(|w| if let Want::Logs(_, f) = w { *f } else { u64::MAX }).collect();
        assert_eq!(froms, vec![5, floor], "链 {chain}:文书的窗照所问,§6.1 的读自下界");
        t.asked.lock().unwrap().clear();
        for w in &wanted {
            store.fetch(&t, &table, w).expect("刷新取得");
        }
        let mut read: Vec<u64> = t.asked.lock().unwrap().iter().map(|(_, f)| *f).collect();
        read.dedup();
        assert_eq!(read, vec![5, floor], "链 {chain}:照所要的起点读(两家各一问)");
        let lo: Vec<u64> = store.tracks.iter().map(|x| x.lo).collect();
        assert_eq!(lo, vec![5, floor], "链 {chain}:记的范围就是读的范围");
    }
}
