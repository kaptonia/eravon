use eravon_chain::endpoints::Table;
use eravon_chain::seam::Url;
use eravon_chain::transport::{Answer, Transport};
use eravon_galeed as k1;
use eravon_literals::json::In;

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
enum Knob {
    Whole,
    NoReceipts,
    NoCode,
    ReceiptsDiffer,
}

#[derive(Clone)]

struct Chain {
    f: In,
    knob: Knob,
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
    fn answer_one(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let b = String::from_utf8_lossy(body).to_string();
        let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
        let f = &self.f;
        let get = |k: &str| f.get(k).expect(k);
        let err = |code: i64| Ok(Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","error":{{"code":{code},"message":"the method does not exist"}}}}"#).into_bytes() });
        let result = if b.contains("eth_chainId") {
            "\"0x7a69\"".to_string()
        } else if b.contains("eth_blockNumber") {
            "\"0x10\"".to_string()
        } else if b.contains("eth_getLogs") {
            format!("[{}]", text(get("log")))
        } else if b.contains("eth_getTransactionByHash") {
            text(get("tx"))
        } else if b.contains("eth_getTransactionReceipt") {
            if self.knob == Knob::NoReceipts {
                return err(-32601);
            }
            let mut v = get("receipts").as_arr().expect("一列")[0].clone();
            if self.knob == Knob::ReceiptsDiffer && url.host.ends_with(".2") {
                if let In::Obj(m) = &mut v {
                    for (k, x) in m.iter_mut() {
                        if k == "status" {
                            *x = In::Str("0x0".into());
                        }
                    }
                }
            }
            text(&v)
        } else if b.contains("eth_getBlockByHash") || b.contains("eth_getBlockByNumber") {
            let parent = get("parent");
            let (hash, number) = (parent.get("hash").and_then(In::as_str).expect("hash"), parent.get("number").and_then(In::as_str).expect("number"));
            text(if b.contains(hash) || b.contains(&format!("\"{number}\"")) { parent } else { get("block") })
        } else if b.contains("eth_getProof") {
            let p = if b.contains("\"0x5\"") { get("proofBefore") } else { get("proofAfter") };
            format!(r#"{{"accountProof":{}}}"#, text(p))
        } else if b.contains("eth_getCode") {
            if self.knob == Knob::NoCode {
                return err(-32601);
            }
            "\"0x\"".to_string()
        } else {
            "null".to_string()
        };
        Ok(Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
    }
}

impl Transport for Chain {
    fn post(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        eravon_chain::ask::each_question(body, |b| self.answer_one(url, b))
    }
}

fn fixture() -> In {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/anvil-anchor.json");
    eravon_literals::json::read(&std::fs::read(p).expect("夹具读得出")).expect("夹具是 JSON")
}

fn read(knob: Knob, endpoints: usize) -> eravon_scan::ledger::Ledger {
    let f = fixture();
    let log = f.get("log").expect("log");
    let emitter = log.get("address").and_then(In::as_str).expect("address").to_ascii_lowercase();
    let sender = format!("0x{}", &log.get("topics").and_then(In::as_arr).expect("topics")[1].as_str().expect("topic")[26..]);
    let t = Chain { f, knob };
    let net = knob as u8;
    let table = Table::new((1..=endpoints).map(|i| format!("http://127.0.{net}.{i}:1")).collect()).expect("端点表");
    let basis = eravon_scan::ledger::basis_of(&[eravon_scan::ledger::Window { chain_id: 31_337, from: 0, to: 16, registries: vec![eravon_scan::ledger::Emitter::Address(emitter)], senders: vec![sender.clone()] }]);
    let ev = eravon_scan::evidence::OnChain { t: &t, table: &table };
    eravon_scan::ledger::read(&t, &table, &ev, 31_337, &sender, &basis, 16).expect("账本读得成")
}

#[test]
fn with_every_read_served_the_anchor_is_counted() {
    let l = read(Knob::Whole, 1);
    assert_eq!((l.records.len(), l.unchecked.len()), (1, 0), "{:?} {:?}", l.unchecked, l.not_anchors);
    assert_eq!(l.records[0].verdict, k1::audit::Verdict::Counted);
}

#[test]
fn an_anchor_whose_receipt_does_not_read_is_not_evidence_checked_by_name() {
    for (knob, n, says) in [(Knob::NoReceipts, 1, "-32601"), (Knob::ReceiptsDiffer, 2, "127.0.3.2")] {
        let l = read(knob, n);
        assert!(l.records.is_empty(), "未经证据核的不成审计记录");
        assert_eq!(l.unchecked.len(), 1);
        let why = &l.unchecked[0].reason;
        assert!(why.starts_with(eravon_scan::ledger::UNCHECKED) && why.contains(says), "{why}");
        assert!(l.narrow.iter().any(|x| x.contains("基底剔除")), "取不到证据的那一块剔出基底并写明:{:?}", l.narrow);
    }
}

#[test]
fn an_anchor_whose_boundary_code_does_not_read_is_unproven() {
    let l = read(Knob::NoCode, 1);
    assert_eq!((l.records.len(), l.unchecked.len()), (1, 0), "{:?} {:?}", l.unchecked, l.not_anchors);
    assert_eq!(l.records[0].verdict, k1::audit::Verdict::Unproven);
}
