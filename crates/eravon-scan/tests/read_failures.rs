use eravon_chain::endpoints::Table;
use eravon_chain::seam::Url;
use eravon_chain::transport::{Answer, Transport};

#[derive(Clone)]
struct Fake<F: Fn(&str, &str) -> Option<String> + Sync + Send + Clone + 'static>(F);

impl<F: Fn(&str, &str) -> Option<String> + Sync + Send + Clone + 'static> Fake<F> {
    fn answer_one(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let b = String::from_utf8_lossy(body).to_string();
        let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
        let result = if b.contains("eth_chainId") {
            Some("\"0x7a69\"".to_string())
        } else if b.contains("eth_blockNumber") {
            Some("\"0x10\"".to_string())
        } else {
            (self.0)(&url.host, &b)
        };
        match result {
            None => Err(eravon_chain::fault::Fault::Transport("连不上".into())),
            Some(r) if r.starts_with("RAW:") => Ok(Answer { status: 200, location: None, body: r[4..].as_bytes().to_vec() }),
            Some(r) => Ok(Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{r}}}"#).into_bytes() }),
        }
    }
}

impl<F: Fn(&str, &str) -> Option<String> + Sync + Send + Clone + 'static> Transport for Fake<F> {
    fn post(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        eravon_chain::ask::each_question(body, |b| self.answer_one(url, b))
    }
}

fn table(n: usize) -> Table {
    Table::new((1..=n).map(|i| format!("http://127.0.0.{i}:1")).collect()).expect("端点表")
}

const TX: &str = "0x1111111111111111111111111111111111111111111111111111111111111111";

fn receipt(logs: &str) -> String {
    format!(r#"{{"blockNumber":"0x5","contractAddress":null,"effectiveGasPrice":"0x1","gasUsed":"0x1","logs":{logs},"status":"0x1"}}"#)
}

#[test]
fn a_board_read_without_an_answer_is_refused() {
    let t = Fake(|_, b: &str| if b.contains("eth_getTransactionReceipt") { None } else { Some("null".into()) });
    assert!(eravon_scan::board::by_tx(&t, &table(1), TX, &[0; 32]).is_err());
}

#[test]
fn a_board_read_answered_by_half_is_refused() {
    let t = Fake(|_, b: &str| if b.contains("eth_getTransactionReceipt") { Some("RAW:{\"id\":1,\"jsonrpc\":\"2.0\",\"result\":{\"logs\":[".into()) } else { Some("null".into()) });
    assert!(eravon_scan::board::by_tx(&t, &table(1), TX, &[0; 32]).is_err());
}

#[test]
fn a_board_read_of_the_wrong_shape_is_refused_by_name() {
    let t = Fake(|_, b: &str| if b.contains("eth_getTransactionReceipt") { Some(receipt("{}")) } else { Some("null".into()) });
    let e = eravon_scan::board::by_tx(&t, &table(1), TX, &[0; 32]).expect_err("错形");
    assert!(e.detail().contains("logs"), "{}", e.detail());
}

#[test]
fn a_board_read_the_endpoints_disagree_on_is_refused() {
    let log = format!(r#"[{{"address":"0x{}","topics":["0x{}"],"data":"0x","blockNumber":"0x5","logIndex":"0x0","transactionHash":"{TX}"}}]"#, "ab".repeat(20), "cd".repeat(32));
    let t = Fake(move |u: &str, b: &str| if b.contains("eth_getTransactionReceipt") { Some(receipt(if u.contains("127.0.0.1") { &log } else { "[]" })) } else { Some("null".into()) });
    assert!(eravon_scan::board::by_tx(&t, &table(2), TX, &[0; 32]).is_err());
}

#[test]
fn a_block_that_does_not_hash_to_its_name_is_refused() {
    let block = r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000005","parentHash":"0x00","sha3Uncles":"0x00","miner":"0x00","stateRoot":"0x00","transactionsRoot":"0x00","receiptsRoot":"0x00","logsBloom":"0x00","difficulty":"0x0","number":"0x5","gasLimit":"0x1","gasUsed":"0x1","timestamp":"0x1","extraData":"0x","mixHash":"0x00","nonce":"0x00","transactions":[]}"#;
    let t = Fake(move |_, b: &str| if b.contains("eth_getBlockByNumber") { Some(block.into()) } else { Some("null".into()) });
    let table = table(1);
    let e = eravon_scan::evidence::block_txs_at(&t, &table, 5).expect_err("不合块哈希");
    assert!(e.detail().contains("块哈希"), "{}", e.detail());
    let down = Fake(|_, _: &str| None);
    assert!(eravon_scan::evidence::block_txs_at(&down, &table, 5).is_err());
}
