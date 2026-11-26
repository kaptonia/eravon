use std::time::Duration;

use eravon_literals::json::{In, Out};

use crate::endpoints::Table;
use crate::fault::{Fault, R};
use crate::mark;
use crate::read::{self, Reading};
use crate::ask::{self, Node};
use crate::seam::{self, NodeAt};
use crate::task::{self, Task};
use crate::transport::Transport;

#[derive(Clone, Debug)]
pub struct Sent {
    pub tx_hash: String,
    pub endpoint: NodeAt,
}

#[derive(Clone, Debug)]
pub struct Receipt {
    pub tx_hash: String,
    pub block: u64,
    pub status: u64,
    pub gas_used: u64,
    pub contract: String,
}

pub fn endpoint_for_send(table: &Table) -> R<NodeAt> {
    table
        .nodes()
        .into_iter()
        .next()
        .ok_or_else(|| Fault::Usage("端点表是空的:发一笔要一个端点".into()))
}

pub fn send_raw<T: Transport>(t: &T, table: &Table, raw: &[u8]) -> R<Sent> {
    mark::mark(mark::FEATURE);
    let endpoint = endpoint_for_send(table)?;
    let hex = seam::hex_of(raw);
    let v = ask::call_one_once(t, &endpoint, seam::M_SEND_RAW, vec![Out::S(hex)])?;
    let tx_hash = v
        .as_str()
        .ok_or_else(|| Fault::Endpoint(format!("{} 该答一枚交易号:{v:?}", seam::M_SEND_RAW)))?
        .to_string();
    let body = tx_hash.strip_prefix("0x").unwrap_or("");
    if body.len() != 64 || !body.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Fault::Endpoint(format!("{} 答的交易号形不合(要 0x 加六十四位十六进制):{tx_hash:?}", seam::M_SEND_RAW)));
    }
    Ok(Sent { tx_hash, endpoint })
}


pub fn pending_nonce<T: Transport>(t: &T, table: &Table, who: &str) -> R<(u64, Reading)> {
    seam::need_address(who)?;
    let r = read::read_agreed(t, table, seam::M_GET_TX_COUNT, vec![Out::S(who.to_string()), Out::S("pending".to_string())])?;
    Ok((seam::read_quantity(&read::as_str(&r)?)?, r))
}

fn holds<T: Transport>(t: &T, endpoint: &NodeAt, tx_hash: &str) -> bool {
    ask::call_one(t, endpoint, seam::M_GET_TX, vec![Out::S(tx_hash.to_string())]).is_ok_and(|v| !matches!(v, In::Null))
}

enum Handed {
    Took,
    Stop(Fault),
    Next(Fault),
}

fn hand<T: Transport>(t: &T, endpoint: &NodeAt, raw: &[u8], tx_hash: &str) -> Handed {
    let Some(hash) = eravon::hexfmt::decode(tx_hash).and_then(|h| <[u8; 32]>::try_from(h).ok()) else {
        return Handed::Stop(Fault::Usage(format!("交易号要是 0x 加六十四位十六进制:{tx_hash:?}")));
    };
    let mut waited = 0usize;
    loop {
        let mut ep = Node::new(t, endpoint.clone());
        let tr = match crate::send::submit(&mut ep, raw, &hash) {
            Ok(()) => return Handed::Took,
            Err(tr) => tr,
        };
        if crate::said::says_rate_limited(&tr) && waited < crate::said::RATE_BACKOFF.len() {
            let pause = crate::patience::waited(crate::said::RATE_BACKOFF[waited]);
            if !pause.is_zero() {
                std::thread::sleep(pause);
            }
            waited += 1;
            continue;
        }
        if matches!(tr, crate::rpc::Trouble::Transport(crate::rpc::Broke::EchoNotOurs)) {
            let said = ep.echoed().map(|w| w.as_str().map(str::to_string).unwrap_or_else(|| crate::wire::write(&w))).unwrap_or_default();
            return echo_other(format!("echo {endpoint} 答回的交易号是 {said},不是这一笔的 {tx_hash}"));
        }
        let f = ep.fault(tr);
        let r = crate::fault::refusal(&f);
        let ask = matches!(f, Fault::Transport(_)) || matches!(r, Some(crate::said::Refusal::Pending | crate::said::Refusal::NonceUsed));
        if ask && holds(t, endpoint, tx_hash) {
            return Handed::Took;
        }
        return Handed::Next(f);
    }
}

pub fn send_everywhere<T: Transport>(t: &T, table: &Table, raw: &[u8], tx_hash: &str) -> R<Sent> {
    mark::mark(mark::FEATURE);
    if table.is_empty() {
        return Err(Fault::Usage("端点表是空的:发一笔要一个端点".into()));
    }
    let mut took: Option<NodeAt> = None;
    let mut refused: Vec<(NodeAt, Fault)> = Vec::new();
    for e in &table.nodes() {
        match hand(t, e, raw, tx_hash) {
            Handed::Took => {
                took.get_or_insert_with(|| e.clone());
            }
            Handed::Stop(f) if took.is_none() => return Err(f),
            Handed::Stop(_) => break,
            Handed::Next(f) => refused.push((e.clone(), f)),
        }
    }
    match took {
        Some(endpoint) => Ok(Sent { tx_hash: tx_hash.to_string(), endpoint }),
        None => match refused.iter().find(|(_, f)| matches!(f, Fault::Echo(_))) {
            Some((_, f)) => Err(f.clone()),
            None => Err(crate::ask::none_answered(refused)),
        },
    }
}


pub fn next_nonce_at<T: Transport>(t: &T, endpoint: &NodeAt, who: &str, at: &str) -> R<u64> {
    seam::need_address(who)?;
    let v = ask::call_one(
        t,
        endpoint,
        seam::M_GET_TX_COUNT,
        vec![Out::S(who.to_string()), Out::S(at.to_string())],
    )?;
    let hex = v
        .as_str()
        .ok_or_else(|| Fault::Endpoint(format!("{} 该答一个数量形:{v:?}", seam::M_GET_TX_COUNT)))?;
    seam::read_quantity(hex)
}

pub fn settled_nonce<T: Transport>(
    t: &T,
    table: &Table,
    who: &str,
    at: &str,
) -> R<(u64, Reading)> {
    seam::need_address(who)?;
    let r = read::read_agreed(
        t,
        table,
        seam::M_GET_TX_COUNT,
        vec![Out::S(who.to_string()), Out::S(at.to_string())],
    )?;
    let n = seam::read_quantity(read::as_str(&r)?.as_str())?;
    Ok((n, r))
}

pub fn receipt_once<T: Transport>(t: &T, endpoint: &NodeAt, tx_hash: &str) -> R<Option<Receipt>> {
    let v = ask::call_one(t, endpoint, seam::M_GET_RECEIPT, vec![Out::S(tx_hash.to_string())])?;
    read_receipt(&v, tx_hash)
}

pub fn receipt<T: Transport>(t: &T, table: &Table, tx_hash: &str) -> R<Option<Receipt>> {
    let r = crate::read::read_agreed(t, table, seam::M_GET_RECEIPT, vec![Out::S(tx_hash.to_string())])?;
    read_receipt(&r.value, tx_hash)
}

fn read_receipt(v: &In, tx_hash: &str) -> R<Option<Receipt>> {
    if matches!(v, In::Null) {
        return Ok(None);
    }
    let need = |k: &str| -> R<u64> {
        let x = v
            .get(k)
            .and_then(|x| x.as_str())
            .ok_or_else(|| Fault::Endpoint(format!("收据上没有 {k}")))?;
        seam::read_quantity(x)
    };
    Ok(Some(Receipt {
        tx_hash: tx_hash.to_string(),
        block: need("blockNumber")?,
        status: need("status")?,
        gas_used: need("gasUsed")?,
        contract: v
            .get("contractAddress")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
    }))
}

pub fn await_receipt<T: Transport>(
    t: &T,
    table: &Table,
    first: &NodeAt,
    tx_hash: &str,
    budget: Duration,
    tick: Duration,
) -> R<Receipt> {
    await_receipt_within(t, table, first, tx_hash, budget, tick)?.ok_or_else(|| Fault::Endpoint(format!("等了 {} 秒还没有 {tx_hash} 的收据", budget.as_secs())))
}

pub fn await_receipt_within<T: Transport>(
    t: &T,
    table: &Table,
    first: &NodeAt,
    tx_hash: &str,
    budget: Duration,
    tick: Duration,
) -> R<Option<Receipt>> {
    let hash: [u8; 32] = eravon::hexfmt::decode(tx_hash)
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| Fault::Usage(format!("交易哈希要是 32 字节的十六进制:{tx_hash}")))?;
    let mut nodes: Vec<Heard<T>> = waited_nodes(table, first).into_iter().map(|n| Heard { node: Node::new(t, n), gave: None }).collect();
    let mut eps: Vec<&mut dyn crate::rpc::Endpoint> = nodes.iter_mut().map(|n| n as &mut dyn crate::rpc::Endpoint).collect();
    match confirmed(&mut eps, &hash, budget, tick) {
        crate::send::Confirm::Included { status, block_number } => {
            let q = crate::ask::params_of(&[Out::S(tx_hash.to_string())]);
            let said = nodes.iter().find_map(|n| n.gave.as_ref().filter(|w| crate::tx::qty_u64(w, "status") == Some(status) && crate::tx::qty_u64(w, "blockNumber") == Some(block_number)).map(|w| (n.node.node().clone(), w.clone())));
            let (at, w) = said.ok_or_else(|| Fault::Endpoint(format!("说上了块的 {tx_hash} 没有一家给过整张收据")))?;
            read_receipt(&ask::decided(&at, seam::M_GET_RECEIPT, &q, &w)?, tx_hash)
        }
        crate::send::Confirm::NotYet => Ok(None),
        crate::send::Confirm::Unreachable(why) => Err(Fault::Transport(format!("等 {tx_hash} 的收据时没有一处回答:{why}"))),
        crate::send::Confirm::Split { has, not_yet } => Err(Fault::Disagree(format!("{tx_hash} 的收据:{} 给了,{} 说还没有", has.join("、"), not_yet.join("、")))),
    }
}

fn confirmed(eps: &mut [&mut dyn crate::rpc::Endpoint], hash: &[u8; 32], budget: Duration, tick: Duration) -> crate::send::Confirm {
    crate::send::confirm_each(eps, hash, budget, &[tick])
}


pub fn waited_nodes(table: &Table, first: &NodeAt) -> Vec<NodeAt> {
    let mut out = vec![first.clone()];
    out.extend(table.nodes().into_iter().filter(|n| n.written() != first.written()));
    out
}

struct Heard<T: Transport> {
    node: Node<T>,
    gave: Option<crate::wire::W>,
}

impl<T: Transport> Heard<T> {
    fn kept(&mut self, method: &str, got: Result<crate::wire::W, crate::rpc::Trouble>) -> Result<crate::wire::W, crate::rpc::Trouble> {
        if let Ok(w) = &got {
            if method == seam::M_GET_RECEIPT && !w.is_null() {
                self.gave = Some(w.clone());
            }
        }
        got
    }
}

impl<T: Transport> crate::rpc::Endpoint for Heard<T> {
    fn call(&mut self, method: &str, params: &eravon::json::Value) -> Result<crate::wire::W, crate::rpc::Trouble> {
        let got = self.node.call(method, params);
        self.kept(method, got)
    }
    fn call_within(&mut self, method: &str, params: &eravon::json::Value, within: Duration) -> Result<crate::wire::W, crate::rpc::Trouble> {
        let got = self.node.call_within(method, params, within);
        self.kept(method, got)
    }
    fn call_batch(&mut self, asks: &[crate::rpc::Ask<'_>]) -> Option<Result<crate::rpc::Batched, crate::rpc::Trouble>> {
        self.node.call_batch(asks)
    }
    fn name(&self) -> String {
        self.node.name()
    }
    fn place(&self) -> String {
        self.node.place()
    }
}

pub fn watch_receipt<T: Transport + Send + 'static>(
    t: T,
    table: Table,
    endpoint: NodeAt,
    tx_hash: String,
    budget: Duration,
    tick: Duration,
) -> Task<Receipt> {
    task::spawn("receipt", move |out| {
        match await_receipt(&t, &table, &endpoint, &tx_hash, budget, tick) {
            Ok(r) => {
                let _ = out.send(crate::task::Outcome::Item(r));
                let _ = out.send(crate::task::Outcome::Done);
            }
            Err(e) => {
                let _ = out.send(crate::task::Outcome::Failed(e));
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct Two;

    impl Two {
        fn answer_one(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let b = String::from_utf8_lossy(body);
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let result = if b.contains(seam::M_CHAIN_ID) {
                "\"0x7a69\""
            } else if url.port == 1 {
                "null"
            } else {
                r#"{"blockNumber":"0x5","gasUsed":"0x5208","status":"0x1"}"#
            };
            Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
        }
    }

    impl Transport for Two {
        fn post(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            crate::ask::each_question(body, |b| self.answer_one(url, b))
        }
    }

    #[test]
    fn a_known_transactions_receipt_one_endpoint_lacks_is_refused_after_asking_again() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let table = Table::new(vec!["http://127.0.0.1:1".into(), "http://127.0.0.1:2".into()]).unwrap();
        let got = receipt(&Two, &table, "0xab");
        {
            let f = got.unwrap_err();
            assert!(f.name() == "E_CHAIN_DISAGREE" && f.detail().starts_with("not-yet "), "{f:?}");
        }
        let one = Table::new(vec!["http://127.0.0.1:1".into()]).unwrap();
        assert!(receipt(&Two, &one, "0xab").unwrap().is_none(), "都答空即还没上块");
    }

    #[derive(Clone)]
    struct Script {
        answer: fn(u16, &str) -> R<&'static str>,
        seen: std::sync::Arc<std::sync::Mutex<Vec<(u16, String)>>>,
    }

    impl Script {
        fn answer_one(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            let b = String::from_utf8_lossy(body).to_string();
            self.seen.lock().unwrap().push((url.port, b.clone()));
            let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
            let method = b.split("\"method\":\"").nth(1).and_then(|x| x.split('"').next()).unwrap_or("").to_string();
            let result = if method == seam::M_CHAIN_ID { "\"0x7a69\"" } else { (self.answer)(url.port, &method)? };
            Ok(crate::transport::Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
        }
    }

    impl Transport for Script {
        fn post(&self, url: &seam::Url, body: &[u8]) -> R<crate::transport::Answer> {
            crate::ask::each_question(body, |b| self.answer_one(url, b))
        }
    }

    const HASH: &str = "0x00000000000000000000000000000000000000000000000000000000000000ab";
    const HASH_Q: &str = "\"0x00000000000000000000000000000000000000000000000000000000000000ab\"";
    const OTHER_Q: &str = "\"0x00000000000000000000000000000000000000000000000000000000000000cd\"";

    fn bench(answer: fn(u16, &str) -> R<&'static str>) -> Script {
        Script { answer, seen: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())) }
    }

    fn two() -> Table {
        Table::new(vec!["http://127.0.0.1:1".into(), "http://127.0.0.1:2".into()]).unwrap()
    }

    #[test]
    fn the_same_bytes_are_handed_to_every_endpoint() {
        let t = bench(|_, m| if m == seam::M_SEND_RAW { Ok(HASH_Q) } else { Ok("null") });
        let sent = send_everywhere(&t, &two(), &[0x02, 0xaa, 0xbb], HASH).expect("两家都收");
        assert!(sent.endpoint.said().ends_with(":1") && sent.tx_hash == HASH, "{sent:?}");
        let raws: Vec<(u16, String)> = t.seen.lock().unwrap().iter().filter(|(_, b)| b.contains(seam::M_SEND_RAW)).map(|(p, b)| (*p, b.split("\"params\":").nth(1).unwrap_or("").to_string())).collect();
        assert_eq!(raws.iter().map(|(p, _)| *p).collect::<Vec<_>>(), vec![1, 2], "两家各交一次");
        assert_eq!(raws[0].1, raws[1].1, "两家收到的是同一份字节");
        assert!(raws[0].1.contains("0x02aabb"));
    }

    #[test]
    fn an_echo_that_is_another_hash_is_refused_by_name() {
        let t = bench(|_, m| if m == seam::M_SEND_RAW { Ok(OTHER_Q) } else { Ok("null") });
        let f = send_everywhere(&t, &two(), &[0x02], HASH).unwrap_err();
        assert_eq!(f.name(), "E_CHAIN_ECHO", "{f:?}");
        assert!(f.detail().starts_with("echo ") && f.detail().contains(HASH), "{f:?}");
        assert!(f.detail().contains(OTHER_Q.trim_matches('"')), "the echo the node gave is named: {f:?}");
        let t = bench(|p, m| if m == seam::M_SEND_RAW { Ok(if p == 1 { OTHER_Q } else { HASH_Q }) } else { Ok("null") });
        assert!(send_everywhere(&t, &two(), &[0x02], HASH).expect("第二家回显对").endpoint.said().ends_with(":2"));
    }

    #[test]
    fn a_handing_without_an_answer_is_asked_by_hash() {
        crate::patience::set_waits(Some(std::time::Duration::ZERO));
        let held = |p: u16, m: &str| -> R<&'static str> {
            match m {
                seam::M_SEND_RAW if p == 1 => Err(Fault::Transport("stream 对端没答完就断了".into())),
                seam::M_SEND_RAW => Err(crate::fault::node_refusal(std::str::from_utf8(br#"{"code":-32000,"message":"already known"}"#).unwrap())),
                _ => Ok(r#"{"blockNumber":null,"from":"0x0000000000000000000000000000000000000001","input":"0x","to":null,"value":"0x0"}"#),
            }
        };
        let sent = send_everywhere(&bench(held), &two(), &[0x02], HASH).expect("按哈希问得到即算交到");
        assert!(sent.endpoint.said().ends_with(":1"), "{sent:?}");
        let lost = |_: u16, m: &str| -> R<&'static str> { if m == seam::M_SEND_RAW { Err(Fault::Transport("stream 对端没答完就断了".into())) } else { Ok("null") } };
        let f = send_everywhere(&bench(lost), &two(), &[0x02], HASH).unwrap_err();
        assert!(f.name() == "E_CHAIN_TRANSPORT" && f.detail().contains(":1") && f.detail().contains(":2"), "{f:?}");
    }

    #[test]
    fn the_nonce_is_the_largest_any_endpoint_gives() {
        let t = bench(|p, m| if m == seam::M_GET_TX_COUNT { Ok(if p == 1 { "\"0x5\"" } else { "\"0x7\"" }) } else { Ok("null") });
        let (n, r) = pending_nonce(&t, &two(), "0x0000000000000000000000000000000000000001").expect("读得到");
        assert_eq!((n, r.endpoints), (7, 2));
    }

    static ROUND: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    #[test]
    fn the_receipt_wait_is_the_familys_and_reads_the_whole_receipt_the_node_gave() {
        let at = NodeAt::new("http://127.0.0.1:1");
        let one = Table::new(vec![at.written().to_string()]).unwrap();
        let later = |_: u16, m: &str| -> R<&'static str> {
            if m != seam::M_GET_RECEIPT {
                return Ok("null");
            }
            Ok(if ROUND.fetch_add(1, std::sync::atomic::Ordering::SeqCst) < 2 { "null" } else { r#"{"blockNumber":"0x9","gasUsed":"0x5208","status":"0x1","contractAddress":"0x00000000000000000000000000000000000000c0"}"# })
        };
        ROUND.store(0, std::sync::atomic::Ordering::SeqCst);
        let t = bench(later);
        let r = await_receipt(&t, &one, &at, HASH, std::time::Duration::from_secs(60), std::time::Duration::ZERO).expect("上了块");
        assert_eq!((r.block, r.status, r.gas_used, r.contract.as_str()), (9, 1, 21_000, "0x00000000000000000000000000000000000000c0"));
        assert_eq!(t.seen.lock().unwrap().iter().filter(|(_, b)| b.contains(seam::M_GET_RECEIPT)).count(), 3, "two rounds of not yet, the third gives it, asked no more");
        let never = |_: u16, _: &str| -> R<&'static str> { Ok("null") };
        let f = await_receipt(&bench(never), &one, &at, HASH, std::time::Duration::ZERO, std::time::Duration::ZERO).unwrap_err();
        assert!(f.name() == "E_CHAIN_ENDPOINT" && f.detail().contains("还没有"), "{f:?}");
        let down = |_: u16, _: &str| -> R<&'static str> { Err(Fault::Transport("connect refused".into())) };
        let f = await_receipt(&bench(down), &one, &at, HASH, std::time::Duration::ZERO, std::time::Duration::ZERO).unwrap_err();
        assert_eq!(f.name(), "E_CHAIN_TRANSPORT", "{f:?}");
        assert_eq!(await_receipt(&bench(never), &one, &at, "0xab", std::time::Duration::ZERO, std::time::Duration::ZERO).unwrap_err().name(), "E_CHAIN_USAGE");
    }

    #[test]
    fn the_wait_asks_the_table_with_the_endpoint_sent_to_first() {
        let three = Table::new(vec!["http://127.0.0.1:1".into(), "http://127.0.0.1:2".into(), "http://127.0.0.1:3".into()]).unwrap();
        let second = NodeAt::new("http://127.0.0.1:2");
        assert_eq!(waited_nodes(&three, &second).iter().map(|n| n.written().to_string()).collect::<Vec<_>>(), ["http://127.0.0.1:2", "http://127.0.0.1:1", "http://127.0.0.1:3"]);
        let hung = |p: u16, m: &str| -> R<&'static str> {
            match m {
                seam::M_GET_RECEIPT if p == 2 => Err(Fault::Transport("late 等满了没有答".into())),
                seam::M_GET_RECEIPT => Ok(r#"{"blockNumber":"0x9","gasUsed":"0x5208","status":"0x1","contractAddress":""}"#),
                _ => Ok("null"),
            }
        };
        let t = bench(hung);
        let r = await_receipt(&t, &three, &second, HASH, std::time::Duration::from_secs(60), std::time::Duration::ZERO).expect("别家给了");
        assert_eq!((r.block, r.status, r.gas_used), (9, 1, 21_000));
        let asked: Vec<u16> = t.seen.lock().unwrap().iter().filter(|(_, b)| b.contains(seam::M_GET_RECEIPT)).map(|(p, _)| *p).collect();
        assert_eq!(asked, [2, 1, 3], "the one sent to first, then the others in the same round, each once");
    }
}

fn echo_other(said: String) -> Handed {
    Handed::Next(Fault::Echo(said))
}

