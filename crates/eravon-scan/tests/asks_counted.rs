use std::sync::Mutex;

use eravon_chain::endpoints::Table;
use eravon_chain::seam::Url;
use eravon_chain::transport::{Answer, Transport};
use eravon_galeed as k1;
use eravon_literals::constants as k7;
use eravon_literals::json::In;

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

fn text(v: &In) -> String {
    String::from_utf8(eravon_literals::json::write(&out(v))).expect("utf8")
}

type Asked = (String, String, In);

#[derive(Clone)]
struct Tally<F: Fn(&str, &str, &In) -> Option<String> + Sync + Send + Clone + 'static> {
    answer: F,
    asked: std::sync::Arc<Mutex<Vec<Asked>>>,
}

impl<F: Fn(&str, &str, &In) -> Option<String> + Sync + Send + Clone + 'static> Tally<F> {
    fn new(answer: F) -> Self {
        Tally { answer, asked: std::sync::Arc::new(Mutex::new(Vec::new())) }
    }

    fn count(&self, host: &str, method: &str) -> usize {
        self.asked.lock().unwrap().iter().filter(|(h, m, _)| h == host && m == method).count()
    }

    fn ranges(&self, host: &str) -> Vec<(u64, u64)> {
        let q = |p: &In, k: &str| u64::from_str_radix(p.as_arr().unwrap()[0].get(k).and_then(In::as_str).unwrap().trim_start_matches("0x"), 16).unwrap();
        self.asked.lock().unwrap().iter().filter(|(h, m, _)| h == host && m == "eth_getLogs").map(|(_, _, p)| (q(p, "fromBlock"), q(p, "toBlock"))).collect()
    }
}

impl<F: Fn(&str, &str, &In) -> Option<String> + Sync + Send + Clone + 'static> Transport for Tally<F> {
    fn post(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let req = eravon_literals::json::read(body).expect("问是 JSON");
        let method = req.get("method").and_then(In::as_str).unwrap_or_default().to_string();
        let params = req.get("params").cloned().unwrap_or(In::Arr(Vec::new()));
        let id = req.get("id").map(text).unwrap_or_else(|| "1".into());
        self.asked.lock().unwrap().push((url.host.clone(), method.clone(), params.clone()));
        let result = if method == "eth_chainId" { Some("\"0x7a69\"".to_string()) } else { (self.answer)(&url.host, &method, &params) };
        match result {
            None => Err(eravon_chain::fault::Fault::Transport("连不上".into())),
            Some(r) => Ok(Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{r}}}"#).into_bytes() }),
        }
    }
}

fn table(hosts: &[&str]) -> Table {
    Table::new(hosts.iter().map(|h| format!("http://{h}:1")).collect()).expect("端点表")
}

fn word(n: u64) -> String {
    format!("0x{n:064x}")
}

fn book_log(block: u64, accede: bool) -> String {
    let (ev, topics) = if accede {
        (&eravon_standard::sol::BOOK_ACCEDED, vec![word(0xa1), word(0xb1), word(0xa1)])
    } else {
        (&eravon_standard::sol::BOOK_DECLARED, vec![word(0xa1), word(0xb1)])
    };
    let t0 = eravon_standard::order::hex_of(&ev.topic0());
    let topics: Vec<String> = std::iter::once(t0).chain(topics).map(|t| format!("\"{t}\"")).collect();
    format!(
        r#"{{"address":"{}","topics":[{}],"data":"0x","blockNumber":"0x{block:x}","logIndex":"0x0","transactionHash":"{}","blockHash":"{}"}}"#,
        k7::SUCCESSION_BOOK.to_ascii_lowercase(),
        topics.join(","),
        word(block),
        word(block + 1)
    )
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

#[test]
fn the_book_is_read_from_its_first_block_and_then_only_onward() {
    let floor = k7::records_floor();
    let head = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(floor + 1_000));
    let records = std::sync::Arc::new(Mutex::new(vec![(floor + 10, false), (floor + 990, true)]));
    let (h, r) = (head.clone(), records.clone());
    let t = Tally::new(move |_, m: &str, p: &In| match m {
        "eth_blockNumber" => Some(format!("\"0x{:x}\"", h.load(std::sync::atomic::Ordering::SeqCst))),
        "eth_getBlockByNumber" => {
            let n = u64::from_str_radix(p.as_arr().unwrap()[0].as_str().unwrap().trim_start_matches("0x"), 16).unwrap();
            Some(format!(r#"{{"hash":"{}","number":"0x{n:x}"}}"#, word(n + 1)))
        }
        "eth_getLogs" => {
            let q = |k: &str| u64::from_str_radix(p.as_arr().unwrap()[0].get(k).and_then(In::as_str).unwrap().trim_start_matches("0x"), 16).unwrap();
            let (lo, hi) = (q("fromBlock"), q("toBlock"));
            let rows: Vec<String> = r.lock().unwrap().iter().filter(|(b, _)| *b >= lo && *b <= hi).map(|(b, a)| book_log(*b, *a)).collect();
            Some(format!("[{}]", rows.join(",")))
        }
        _ => Some("null".into()),
    });
    let hosts = ["127.0.1.1", "127.0.1.2"];
    let table = table(&hosts);
    let record: std::sync::Arc<Settled> = Default::default();
    let first = {
        let _a = eravon_chain::action::open(Some(record.clone()));
        eravon_scan::succession::logs(&t, &table).expect("簿读得成")
    };
    assert_eq!((first.declarations.len(), first.accessions.len()), (1, 1));
    for h in hosts {
        assert_eq!(t.ranges(h), vec![(floor, floor + 1_000)], "{h} 头一读自换钥簿起块问到共同头,一问即尽");
    }
    head.store(floor + 1_050, std::sync::atomic::Ordering::SeqCst);
    records.lock().unwrap().push((floor + 1_020, false));
    let second = {
        let _a = eravon_chain::action::open(Some(record.clone()));
        eravon_scan::succession::logs(&t, &table).expect("簿再读得成")
    };
    let settled = floor + 1_000 - eravon_chain::settled::MARGIN;
    for h in hosts {
        assert_eq!(t.ranges(h)[1..], [(settled + 1, floor + 1_050)], "{h} 第二读只问头一读落定之后到新头");
        assert_eq!(t.count(h, "eth_getLogs"), 2);
    }
    let blocks: Vec<u64> = second.declarations.iter().map(|d| d.block_number).chain(second.accessions.iter().map(|a| a.block_number)).collect();
    assert_eq!(blocks, vec![floor + 10, floor + 1_020, floor + 990], "每条记录恰一回(界内那条重读而不重记)");
}

fn fixture() -> In {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/anvil-anchor.json");
    eravon_literals::json::read(&std::fs::read(p).expect("夹具读得出")).expect("夹具是 JSON")
}

fn chain_of(f: In, block: In, raw: impl Fn(&str, &str) -> Option<String> + Sync + Send + Clone + 'static) -> Tally<impl Fn(&str, &str, &In) -> Option<String> + Sync + Send + Clone + 'static> {
    Tally::new(move |host: &str, m: &str, p: &In| {
        let first = p.as_arr().and_then(|a| a.first()).and_then(In::as_str).unwrap_or_default().to_string();
        match m {
            "eth_blockNumber" => Some("\"0x10\"".into()),
            "eth_getBlockByNumber" => Some(text(&block)),
            "eth_getBlockByHash" => {
                let want = block.get("hash").and_then(In::as_str).unwrap();
                Some(text(if first.eq_ignore_ascii_case(want) { &block } else { f.get("parent").unwrap() }))
            }
            "eth_getRawTransactionByHash" => raw(host, &first),
            "eth_getBlockReceipts" => Some(text(f.get("receipts").unwrap())),
            "eth_getProof" => {
                let at = p.as_arr().unwrap()[2].as_str().unwrap().to_string();
                let pr = if at == "0x5" { f.get("proofBefore") } else { f.get("proofAfter") };
                Some(format!(r#"{{"accountProof":{}}}"#, text(pr.unwrap())))
            }
            _ => Some("null".into()),
        }
    })
}

fn the_raw(f: &In) -> String {
    text(&f.get("rawTxs").and_then(In::as_arr).unwrap()[0])
}

fn lists_of<T: Transport>(t: &T, table: &Table, f: &In) -> Result<(), eravon_scan::fault::Fault> {
    let n = f.get("block").and_then(|b| b.get("number")).and_then(In::as_str).and_then(|q| eravon_chain::seam::read_quantity(q).ok()).unwrap();
    eravon_scan::evidence::block_txs_at(t, table, n).map(|_| ()).map_err(eravon_scan::fault::Fault::from)
}

#[test]
fn an_anchor_block_is_fetched_from_one_endpoint_once() {
    let f = fixture();
    let raw = the_raw(&f);
    let t = chain_of(f.clone(), f.get("block").unwrap().clone(), move |_, _| Some(raw.clone()));
    let hosts = ["127.0.2.1", "127.0.2.2"];
    let table = table(&hosts);
    for round in 0..2 {
        lists_of(&t, &table, &f).unwrap_or_else(|e| panic!("第 {round} 回取块:{}", e.detail()));
        assert_eq!((t.count(hosts[0], "eth_getRawTransactionByHash"), t.count(hosts[1], "eth_getRawTransactionByHash")), (1, 0), "第 {round} 回:原文只问一家、只问一回");
        assert_eq!((t.count(hosts[0], "eth_getBlockReceipts"), t.count(hosts[1], "eth_getBlockReceipts")), (0, 0), "第 {round} 回:回执不问");
    }
}

#[test]
fn transactions_that_do_not_rebuild_the_root_are_refused_by_name() {
    let f = fixture();
    let raw = the_raw(&f);
    let hosts = ["127.0.3.1", "127.0.3.2"];
    let wrong = raw.clone();
    let t = chain_of(f.clone(), f.get("block").unwrap().clone(), move |h, _| Some(if h == "127.0.3.1" { "\"0x01\"".into() } else { wrong.clone() }));
    lists_of(&t, &table(&hosts), &f).unwrap_or_else(|e| panic!("头一家答错即问下一家:{}", e.detail()));
    assert_eq!(
        (t.count(hosts[0], "eth_getRawTransactionByHash"), t.count(hosts[1], "eth_getRawTransactionByHash")),
        (2, 1),
        "头一家:整块写回不成即逐笔问,对不上再逐笔问一回;下一家一问即对"
    );

    let extra = "0xc0de";
    let extra_hash = k1::hash::hex_encode(&k1::hash::keccak256(&[0xc0, 0xde]));
    let block = match f.get("block").unwrap().clone() {
        In::Obj(m) => In::Obj(
            m.into_iter()
                .map(|(k, v)| match (k.as_str(), v) {
                    ("transactions", In::Arr(mut a)) => {
                        a.push(In::Str(extra_hash.clone()));
                        (k, In::Arr(a))
                    }
                    (_, v) => (k, v),
                })
                .collect(),
        ),
        x => x,
    };
    let eh = extra_hash.clone();
    let hosts = ["127.0.4.1", "127.0.4.2"];
    let t = chain_of(f.clone(), block, move |_, h| Some(if h == eh { format!("\"{extra}\"") } else { raw.clone() }));
    let e = lists_of(&t, &table(&hosts), &f).expect_err("交易树根对不上");
    assert!(e.detail().contains("transactionsRoot") && hosts.iter().all(|h| e.detail().contains(h)), "{}", e.detail());
    assert!(t.count(hosts[1], "eth_getRawTransactionByHash") >= 1, "头一家对不上即换下一家");
}

const CHILD: &str = "ERAVON_SCAN_TEST_KEPT_CHILD";

#[test]
fn kept_child() {
    let Some(dir) = std::env::var_os(CHILD) else { return };
    eravon_scan::evidence::keep_blocks_in(Some(dir.into()));
    let f = fixture();
    let raw = the_raw(&f);
    let t = chain_of(f.clone(), f.get("block").unwrap().clone(), move |_, _| Some(raw.clone()));
    let hosts = ["127.0.5.1", "127.0.5.2"];
    lists_of(&t, &table(&hosts), &f).unwrap_or_else(|e| panic!("取块:{}", e.detail()));
    let n = |m: &str| hosts.iter().map(|h| t.count(h, m)).sum::<usize>();
    println!("asked {} {}", n("eth_getRawTransactionByHash"), n("eth_getBlockReceipts"));
}

#[test]
fn a_checked_blocks_transactions_are_not_fetched_by_the_next_process() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("kept-blocks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let run = || {
        let o = std::process::Command::new(std::env::current_exe().expect("测试二进制"))
            .args(["kept_child", "--exact", "--nocapture", "--test-threads=1"])
            .env(CHILD, &dir)
            .output()
            .expect("起子进程");
        let text = String::from_utf8_lossy(&o.stdout).to_string();
        assert!(o.status.success(), "子进程失败:{text}{}", String::from_utf8_lossy(&o.stderr));
        let line = text.lines().find_map(|l| l.split_once("asked ").map(|(_, n)| n.trim())).unwrap_or_else(|| panic!("子进程没报数:{text}")).to_string();
        let n: Vec<usize> = line.split_whitespace().map(|x| x.parse().expect("数")).collect();
        (n[0], n[1])
    };
    let f = fixture();
    let hash = f.get("block").and_then(|b| b.get("hash")).and_then(In::as_str).unwrap().to_ascii_lowercase();
    let file = dir.join(format!("{}.json", hash.trim_start_matches("0x")));
    assert_eq!(run(), (1, 0), "头一个进程:原文问一回,回执不问");
    assert!(file.is_file(), "核过的块记在所给目录:{}", file.display());
    assert_eq!(run(), (0, 0), "下一个进程:原文一回也不问");
    std::fs::write(&file, br#"{"rawTxs":["0x01"],"receipts":[]}"#).expect("改坏");
    assert_eq!(run(), (1, 0), "记的对不上树根即不取,再问端点");
    assert_eq!(run(), (0, 0), "重写之后又取得");
    let _ = std::fs::remove_dir_all(&dir);
}
