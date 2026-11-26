use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use eravon_chain::endpoints::Table;
use eravon_chain::seam::Url;
use eravon_chain::transport::{Answer, Transport};
use eravon_galeed as k1;
use eravon_literals::json::In;

#[derive(Clone)]

struct Chain {
    f: In,
    limits: usize,
    seen: std::sync::Arc<Mutex<HashMap<String, usize>>>,
    posts: std::sync::Arc<Mutex<Vec<(String, bool, usize)>>>,
}

fn text(v: &In) -> String {
    String::from_utf8(eravon_literals::json::write(&out(v))).expect("utf8")
}

fn out(v: &In) -> eravon_literals::json::Out {
    use eravon_literals::json::Out;
    match v {
        In::Null => Out::Null,
        In::Bool(b) => Out::B(*b),
        In::Int(n) => Out::N(*n as u64),
        In::Str(s) => Out::S(s.clone()),
        In::Arr(a) => Out::A(a.iter().map(out).collect()),
        In::Obj(m) => Out::O(m.iter().map(|(k, v)| (Box::leak(k.clone().into_boxed_str()) as &'static str, out(v))).collect()),
    }
}

impl Chain {
    fn new(limits: usize) -> Chain {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/anvil-anchor.json");
        let f = eravon_literals::json::read(&std::fs::read(p).expect("夹具读得出")).expect("夹具是 JSON");
        Chain { f, limits, seen: std::sync::Arc::new(Mutex::new(HashMap::new())), posts: std::sync::Arc::new(Mutex::new(Vec::new())) }
    }

    fn answer_one(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let b = String::from_utf8_lossy(body).to_string();
        let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
        let f = &self.f;
        let get = |k: &str| f.get(k).expect(k);
        let result = if b.contains("eth_chainId") {
            "\"0x7a69\"".to_string()
        } else if b.contains("eth_blockNumber") {
            "\"0x10\"".to_string()
        } else if b.contains("eth_getLogs") {
            format!("[{}]", text(get("log")))
        } else if b.contains("eth_getTransactionByHash") {
            text(get("tx"))
        } else if b.contains("eth_getTransactionReceipt") {
            if url.host.ends_with(".2") {
                let tx = b.split("\"params\":[\"").nth(1).and_then(|x| x.split('"').next()).unwrap_or_default().to_string();
                let mut seen = self.seen.lock().unwrap();
                let n = seen.entry(tx).or_insert(0);
                *n += 1;
                if *n <= self.limits {
                    return Ok(Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","error":{{"code":-32005,"message":"rate limit exceeded"}}}}"#).into_bytes() });
                }
            }
            text(&get("receipts").as_arr().expect("一列")[0])
        } else if b.contains("eth_getBlockByHash") || b.contains("eth_getBlockByNumber") {
            let parent = get("parent");
            let (hash, number) = (parent.get("hash").and_then(In::as_str).expect("hash"), parent.get("number").and_then(In::as_str).expect("number"));
            text(if b.contains(hash) || b.contains(&format!("\"{number}\"")) { parent } else { get("block") })
        } else if b.contains("eth_getProof") {
            let p = if b.contains("\"0x5\"") { get("proofBefore") } else { get("proofAfter") };
            format!(r#"{{"accountProof":{}}}"#, text(p))
        } else if b.contains("eth_getCode") {
            "\"0x\"".to_string()
        } else {
            "null".to_string()
        };
        Ok(Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
    }

    fn receipt_posts(&self, host: &str) -> Vec<(bool, usize)> {
        self.posts.lock().unwrap().iter().filter(|(h, _, n)| h.ends_with(host) && *n > 0).map(|(_, b, n)| (*b, *n)).collect()
    }
}

impl Transport for Chain {
    fn post(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let b = String::from_utf8_lossy(body);
        self.posts.lock().unwrap().push((url.host.clone(), b.trim_start().starts_with('['), b.matches("eth_getTransactionReceipt").count()));
        eravon_chain::ask::each_question(body, |q| self.answer_one(url, q))
    }
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
struct Keeper(Mutex<BTreeMap<String, Vec<u8>>>);

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

fn read(t: &Chain, net: u8) -> eravon_scan::ledger::Ledger {
    let log = t.f.get("log").expect("log");
    let emitter = log.get("address").and_then(In::as_str).expect("address").to_ascii_lowercase();
    let sender = format!("0x{}", &log.get("topics").and_then(In::as_arr).expect("topics")[1].as_str().expect("topic")[26..]);
    let table = Table::new((1..=2).map(|i| format!("http://127.0.{net}.{i}:1")).collect()).expect("端点表");
    let basis = eravon_scan::ledger::basis_of(&[eravon_scan::ledger::Window { chain_id: 31_337, from: 0, to: 16, registries: vec![eravon_scan::ledger::Emitter::Address(emitter)], senders: vec![sender.clone()] }]);
    let ev = eravon_scan::evidence::OnChain { t, table: &table };
    let _a = eravon_chain::action::open(Some(Arc::new(Keeper::default())));
    eravon_scan::ledger::read(t, &table, &ev, 31_337, &sender, &basis, 16).expect("账本读得成")
}

#[test]
fn a_node_limiting_receipts_one_by_one_leaves_the_ledger_read_and_the_judged_anchor_kept() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));

    eravon_scan::judged::keep_in(Some(Arc::new(Room::default())));
    let once = Chain::new(1);
    let l = read(&once, 61);
    assert_eq!((l.records.len(), l.unchecked.len()), (1, 0), "{:?} {:?}", l.unchecked, l.not_anchors);
    assert_eq!(l.records[0].verdict, k1::audit::Verdict::Counted);
    assert_eq!(once.receipt_posts(".2").iter().map(|(_, n)| n).sum::<usize>(), 2, "limited once, then answered: {:?}", once.receipt_posts(".2"));

    eravon_scan::judged::keep_in(Some(Arc::new(Room::default())));
    let never = Chain::new(usize::MAX);
    let l = read(&never, 62);
    assert_eq!((l.records.len(), l.unchecked.len()), (1, 0), "the other endpoint's word: {:?} {:?}", l.unchecked, l.not_anchors);
    assert_eq!(l.records[0].verdict, k1::audit::Verdict::Counted);
    assert!(!never.receipt_posts(".2").is_empty(), "the limiting node was asked");
    never.posts.lock().unwrap().clear();
    let again = read(&never, 62);
    assert_eq!(again.records.len(), 1, "the judged anchor kept");
    assert!(never.receipt_posts("").is_empty(), "the next read asks no receipt: {:?}", never.receipt_posts(""));
    eravon_scan::judged::keep_in(None);
}
