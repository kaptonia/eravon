use eravon_chain::endpoints::Table;
use eravon_chain::seam::Url;
use eravon_chain::transport::{Answer, Transport};

#[derive(Clone)]
struct Chain {
    tx: String,
    instance: String,
    initcode: String,
    traces: bool,
}

impl Transport for Chain {
    fn post(&self, _url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let b = String::from_utf8_lossy(body).to_string();
        let id = b.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("0").to_string();
        let method = b.split("\"method\":\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("").to_string();
        let err = |code: i64| Ok(Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","error":{{"code":{code},"message":"the method does not exist"}}}}"#).into_bytes() });
        let block = r#"{"number":"0x10","timestamp":"0x6a000000","hash":"0x1111111111111111111111111111111111111111111111111111111111111111","parentHash":"0x2222222222222222222222222222222222222222222222222222222222222222","baseFeePerGas":"0x1"}"#;
        let result = match method.as_str() {
            "eth_chainId" => "\"0x7a69\"".to_string(),
            "eth_blockNumber" => "\"0x10\"".to_string(),
            "eth_getBlockByNumber" => block.to_string(),
            "eth_getLogs" => "[]".to_string(),
            "eth_getCode" => "\"0x\"".to_string(),
            "eth_getTransactionByHash" if b.contains(&self.tx) => {
                format!(r#"{{"hash":"{}","from":"0x00000000000000000000000000000000000000f1","to":"0x4e59b44847b379578588920ca78fbf26c0b4956c","input":"0x{}","blockNumber":"0x5"}}"#, self.tx, "ab".repeat(8))
            }
            "eth_getTransactionReceipt" if b.contains(&self.tx) => r#"{"status":"0x1","blockNumber":"0x5","logs":[]}"#.to_string(),
            "eth_getTransactionReceipt" => r#"{"status":"0x1","blockNumber":"0x3","logs":[]}"#.to_string(),
            "debug_traceTransaction" if self.traces => format!(
                r#"{{"type":"CALL","from":"0x00000000000000000000000000000000000000f1","to":"0x4e59b44847b379578588920ca78fbf26c0b4956c","input":"0x","calls":[{{"type":"CREATE2","from":"0x4e59b44847b379578588920ca78fbf26c0b4956c","to":"{}","input":"{}"}}]}}"#,
                self.instance, self.initcode
            ),
            "debug_traceTransaction" | "trace_transaction" => return err(-32601),
            _ => "null".to_string(),
        };
        Ok(Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#).into_bytes() })
    }
}

fn case() -> eravon_recheck::dossier::Case {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../base/eravon-conformance/dossier-corpus/dossiers/d002_ok_open");
    match eravon_recheck::dossier::open(&dir).expect("夹具读得出") {
        eravon_recheck::dossier::Opened::Whole(c) => *c,
        eravon_recheck::dossier::Opened::Refused(_) => panic!("夹具是一份完整卷宗"),
    }
}

fn kinds(traces: bool, tx: &str) -> Vec<String> {
    let c = case();
    let initcode = eravon_standard::order::hex_of(&eravon_terms::derive::initcode(&c.logic));
    let t = Chain { tx: tx.to_string(), instance: c.instance.clone(), initcode, traces };
    let table = Table::new(vec!["http://127.0.0.1:1".into()]).expect("端点表");
    let rows = eravon_recheck::check::rows_of(&t, &table, &c, &[tx.to_string()], 0).expect("行读得成");
    rows.iter().filter(|r| r.tx == tx).map(|r| r.kind.clone()).collect()
}

#[test]
fn a_creation_no_endpoint_traces_makes_no_deploy_row() {
    assert_eq!(kinds(true, &format!("0x{}", "a1".repeat(32))), vec!["deploy".to_string()]);
    assert!(kinds(false, &format!("0x{}", "c2".repeat(32))).is_empty(), "不供追踪即不成 deploy 行");
}

#[derive(Clone)]
struct Batching<T> {
    inner: T,
    chain: u64,
    posts: std::sync::Arc<std::sync::Mutex<usize>>,
    questions: std::sync::Arc<std::sync::Mutex<usize>>,
}

impl<T: Transport> Transport for Batching<T> {
    fn post(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        *self.posts.lock().unwrap() += 1;
        let v = eravon_literals::json::read(body).expect("问是 JSON");
        let eravon_literals::json::In::Arr(qs) = v else {
            *self.questions.lock().unwrap() += 1;
            return Ok(Answer { status: 200, location: None, body: self.one(url, &v)?.into_bytes() });
        };
        let mut out = Vec::new();
        for q in &qs {
            *self.questions.lock().unwrap() += 1;
            out.push(self.one(url, q)?);
        }
        Ok(Answer { status: 200, location: None, body: format!("[{}]", out.join(",")).into_bytes() })
    }
}

impl<T: Transport> Batching<T> {
    fn one(&self, url: &Url, q: &eravon_literals::json::In) -> eravon_chain::fault::R<String> {
        if q.get("method").and_then(eravon_literals::json::In::as_str) == Some("eth_chainId") {
            let id = q.get("id").map(|x| String::from_utf8_lossy(&eravon_literals::json::write(&out_of(x))).to_string()).unwrap_or_else(|| "0".into());
            return Ok(format!(r#"{{"id":{id},"jsonrpc":"2.0","result":"0x{:x}"}}"#, self.chain));
        }
        Ok(String::from_utf8_lossy(&self.inner.post(url, &eravon_literals::json::write(&out_of(q)))?.body).to_string())
    }
}

fn out_of(v: &eravon_literals::json::In) -> eravon_literals::json::Out {
    use eravon_literals::json::{In, Out};
    match v {
        In::Null => Out::Null,
        In::Bool(b) => Out::B(*b),
        In::Int(n) => Out::N(*n as u64),
        In::Str(s) => Out::S(s.clone()),
        In::Arr(a) => Out::A(a.iter().map(out_of).collect()),
        In::Obj(m) => Out::O(m.iter().map(|(k, v)| (Box::leak(k.clone().into_boxed_str()) as &'static str, out_of(v))).collect()),
    }
}

#[test]
fn the_re_check_asked_as_one_action_never_asks_more() {
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let c = case();
    let initcode = eravon_standard::order::hex_of(&eravon_terms::derive::initcode(&c.logic));
    let count = |port: u16, action: bool| -> (usize, usize) {
        let t = Batching { inner: Chain { tx: format!("0x{}", "ea".repeat(32)), instance: c.instance.clone(), initcode: initcode.clone(), traces: true }, chain: c.chain_id, posts: Default::default(), questions: Default::default() };
        let table = Table::new(vec![format!("http://127.0.0.1:{port}"), format!("http://127.0.0.1:{}", port + 1)]).expect("端点表");
        let _a = action.then(|| eravon_chain::action::open(None));
        let got = eravon_recheck::check::run(&t, &table, &c, &eravon_recheck::check::Opts::default());
        assert!(got.is_ok(), "复核走得完全程:{:?}", got.err().map(|e| e.detail()));
        let n = (*t.posts.lock().unwrap(), *t.questions.lock().unwrap());
        n
    };
    let (before_rounds, before_asks) = count(31701, false);
    let (after_rounds, after_asks) = count(31703, true);
    note("erk_check", format!("ASKED erk-check d002 without action: round trips={before_rounds} questions={before_asks}; as one action: round trips={after_rounds} questions={after_asks}"));
    assert!(after_asks <= before_asks && after_rounds <= before_rounds, "开动作之后问数与往返不多于之前");
}

fn note(test: &str, line: String) {
    println!("{line}");
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("asked");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(format!("{test}.txt")), format!("{line}\n"));
}
