use std::sync::Mutex;

use eravon_chain::endpoints::Table;
use eravon_chain::seam::Url;
use eravon_chain::transport::{Answer, Transport};
use eravon_literals::json::In;

fn hash(i: usize) -> String {
    format!("0x{}{i:04x}", "d".repeat(60))
}

fn note(test: &str, line: String) {
    println!("{line}");
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("asked");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(format!("{test}.txt")), format!("{line}\n"));
}

#[derive(Clone)]
struct Busy {
    in_flight: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    limit_once: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    batches: std::sync::Arc<Mutex<usize>>,
    singles: std::sync::Arc<Mutex<usize>>,
}

impl Transport for Busy {
    fn post(&self, _url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        use std::sync::atomic::Ordering::SeqCst;
        let v = eravon_literals::json::read(body).expect("问是 JSON");
        let answer = |q: &In| {
            let id = match q.get("id") {
                Some(In::Int(n)) => n.to_string(),
                _ => "0".into(),
            };
            match q.get("method").and_then(In::as_str).unwrap_or_default() {
                "eth_chainId" => format!(r#"{{"id":{id},"jsonrpc":"2.0","result":"0x1"}}"#),
                _ => format!(r#"{{"id":{id},"jsonrpc":"2.0","result":"0x01"}}"#),
            }
        };
        let In::Arr(qs) = &v else {
            if v.get("method").and_then(In::as_str) == Some("eth_getRawTransactionByHash") {
                *self.singles.lock().unwrap() += 1;
            }
            return Ok(Answer { status: 200, location: None, body: answer(&v).into_bytes() });
        };
        *self.batches.lock().unwrap() += 1;
        let others = self.in_flight.fetch_add(1, SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(30));
        let once = self.limit_once.load(SeqCst) > 0 && self.limit_once.fetch_sub(1, SeqCst) > 0;
        self.in_flight.fetch_sub(1, SeqCst);
        if others > 0 || once {
            return Ok(Answer { status: 429, location: None, body: b"Too Many Requests".to_vec() });
        }
        Ok(Answer { status: 200, location: None, body: format!("[{}]", qs.iter().map(answer).collect::<Vec<_>>().join(",")).into_bytes() })
    }
}

fn busy_run(port: u16, limit_once: usize) -> (usize, usize, usize, bool) {
    let t = Busy { in_flight: Default::default(), limit_once: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(limit_once)), batches: Default::default(), singles: Default::default() };
    let tb = Table::new(vec![format!("http://127.0.0.1:{port}")]).unwrap();
    let _ = tb.chain.set(1);
    let hashes: Vec<String> = (0..60).map(hash).collect();
    let _a = eravon_chain::action::open(None);
    let got = eravon_chain::read::block_transactions_proven(&t, &tb, &hashes, |_| Ok(())).expect("原文取齐");
    let unbatched = eravon_chain::read::unbatched().iter().any(|(said, _)| said.contains(&port.to_string()));
    (got.len(), *t.batches.lock().unwrap(), *t.singles.lock().unwrap(), unbatched)
}

#[test]
fn a_batch_turned_away_for_its_rate_limit_is_waited_out_not_unbatched() {
    let (n, batches, singles, unbatched) = busy_run(31801, 0);
    note("busy_batches", format!("ASKED raw transactions 60 in batches of 20, one batch at a time taken: batch posts={batches} single asks={singles}"));
    assert_eq!(n, 60, "原文取齐");
    assert!(!unbatched, "限流不当作不收批量");
    assert_eq!(singles, 0, "没有一笔落到逐笔问");
}

#[test]
fn a_batch_limited_once_alone_is_answered_after_its_pause() {
    let (n, _, singles, unbatched) = busy_run(31803, 1);
    assert_eq!(n, 60);
    assert!(!unbatched, "单发限流一次也不当作不收批量");
    assert_eq!(singles, 0);
}
