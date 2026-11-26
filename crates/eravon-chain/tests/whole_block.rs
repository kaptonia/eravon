use std::sync::Mutex;

use eravon_chain::endpoints::Table;
use eravon_chain::seam::Url;
use eravon_chain::transport::{Answer, Transport};
use eravon_literals::json::In;

fn raws(n: usize) -> Vec<Vec<u8>> {
    (0..n).map(|i| vec![0xf0, (i >> 8) as u8, i as u8, 0x5a]).collect()
}

fn hex(b: &[u8]) -> String {
    eravon::hexfmt::encode(b)
}

#[derive(Clone)]
struct Two {
    port: u16,
    txs: Vec<Vec<u8>>,
    odd: usize,
    bad: Option<usize>,
    posts: std::sync::Arc<Mutex<Vec<(u16, Vec<String>)>>>,
}

impl Two {
    fn tx_of(&self, port: u16, i: usize) -> Vec<u8> {
        let mut t = self.txs[i].clone();
        if port == self.port && self.bad == Some(i) {
            t[3] ^= 0xff;
        }
        t
    }

    fn answer(&self, port: u16, q: &In) -> String {
        let id = match q.get("id") {
            Some(In::Int(n)) => n.to_string(),
            _ => "0".into(),
        };
        let params = q.get("params").and_then(In::as_arr).map(<[In]>::to_vec).unwrap_or_default();
        let hashes: Vec<String> = self.txs.iter().map(|t| hex(&eravon::cryptox::keccak256(t))).collect();
        let result = match q.get("method").and_then(In::as_str).unwrap_or_default() {
            "eth_chainId" => "\"0x1\"".to_string(),
            "eth_getBlockByHash" => {
                let objs: Vec<String> = (0..self.txs.len())
                    .map(|i| if i == self.odd { format!(r#"{{"hash":"{}"}}"#, hashes[i]) } else { format!(r#"{{"hash":"{}","raw":"{}"}}"#, hashes[i], hex(&self.tx_of(port, i))) })
                    .collect();
                format!(r#"{{"hash":"0x{}","transactions":[{}]}}"#, "aa".repeat(32), objs.join(","))
            }
            "eth_getRawTransactionByHash" => {
                let h = params.first().and_then(In::as_str).unwrap_or_default().to_ascii_lowercase();
                match hashes.iter().position(|x| *x == h) {
                    Some(i) => format!("\"{}\"", hex(&self.tx_of(port, i))),
                    None => "null".into(),
                }
            }
            _ => "null".to_string(),
        };
        format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#)
    }
}

impl Transport for Two {
    fn post(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let v = eravon_literals::json::read(body).expect("问是 JSON");
        let (methods, out) = match &v {
            In::Arr(qs) => (
                qs.iter().map(|q| q.get("method").and_then(In::as_str).unwrap_or_default().to_string()).collect::<Vec<_>>(),
                format!("[{}]", qs.iter().map(|q| self.answer(url.port, q)).collect::<Vec<_>>().join(",")),
            ),
            q => (vec![q.get("method").and_then(In::as_str).unwrap_or_default().to_string()], self.answer(url.port, q)),
        };
        self.posts.lock().unwrap().push((url.port, methods));
        Ok(Answer { status: 200, location: None, body: out.into_bytes() })
    }
}

fn rebuild(o: &In) -> Option<Vec<u8>> {
    let s = o.get("raw").and_then(In::as_str)?;
    (2..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

fn run(port: u16, n: usize, bad: Option<usize>) -> (Vec<Vec<u8>>, Two) {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let t = Two { port, txs: raws(n), odd: n / 2, bad, posts: std::sync::Arc::new(Mutex::new(Vec::new())) };
    let tb = Table::new(vec![format!("http://127.0.0.1:{port}"), format!("http://127.0.0.1:{}", port + 1)]).unwrap();
    let _ = tb.chain.set(1);
    let hashes: Vec<String> = t.txs.iter().map(|x| hex(&eravon::cryptox::keccak256(x))).collect();
    let want = t.txs.clone();
    let got = eravon_chain::read::block_transactions_whole(&t, &tb, &format!("0x{}", "aa".repeat(32)), &hashes, rebuild, |x| if x == want.as_slice() { Ok(()) } else { Err("树根对不上".into()) }).expect("取齐");
    (got, t)
}

#[test]
fn a_block_is_one_question_and_only_the_transaction_not_written_back_is_asked_alone() {
    for (port, n) in [(31761u16, 110usize), (31763, 220)] {
        let (got, t) = run(port, n, None);
        let posts = t.posts.lock().unwrap().clone();
        let raw_asks: usize = posts.iter().map(|(_, m)| m.iter().filter(|x| *x == "eth_getRawTransactionByHash").count()).sum();
        println!("ASKED one block of {n} transactions had whole: posts={} raw asks={raw_asks}", posts.len());
        assert_eq!(got, raws(n));
        assert_eq!(posts.len(), 2, "整块一问,写不回的那一笔另问一次");
        assert_eq!(raw_asks, 1);
        assert!(posts.iter().all(|(p, _)| *p == port), "只问一家");
    }
}

#[test]
fn an_endpoint_whose_list_does_not_hold_is_passed_over_for_the_next() {
    let (got, t) = run(31765, 40, Some(3));
    assert_eq!(got, raws(40), "第二家给对");
    let posts = t.posts.lock().unwrap().clone();
    assert!(posts.iter().any(|(p, _)| *p == 31766), "换了下一家");
}
