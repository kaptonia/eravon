use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use eravon_literals::json::{In, Out};

type Asked = Arc<Mutex<Vec<(String, String)>>>;

fn case() -> eravon_recheck::dossier::Case {
    match eravon_recheck::dossier::open(&dossier()).expect("夹具读得出") {
        eravon_recheck::dossier::Opened::Whole(c) => *c,
        eravon_recheck::dossier::Opened::Refused(_) => panic!("夹具是一份完整卷宗"),
    }
}

fn dossier() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../base/eravon-conformance/dossier-corpus/dossiers/d002_ok_open")
}

fn out_of(v: &In) -> Out {
    match v {
        In::Null => Out::Null,
        In::Bool(b) => Out::B(*b),
        In::Int(n) => Out::N(*n as u64),
        In::Str(s) => Out::S(s.clone()),
        In::Arr(a) => Out::A(a.iter().map(out_of).collect()),
        In::Obj(m) => Out::O(m.iter().map(|(k, v)| (Box::leak(k.clone().into_boxed_str()) as &'static str, out_of(v))).collect()),
    }
}

fn answer(q: &In, chain: u64, tx: &str, instance: &str, initcode: &str, asked: &Asked) -> String {
    let id = q.get("id").map(|x| String::from_utf8_lossy(&eravon_literals::json::write(&out_of(x))).to_string()).unwrap_or_else(|| "0".into());
    let method = q.get("method").and_then(In::as_str).unwrap_or("").to_string();
    let params = q.get("params").map(|x| String::from_utf8_lossy(&eravon_literals::json::write(&out_of(x))).to_string()).unwrap_or_default();
    asked.lock().unwrap().push((method.clone(), params.clone()));
    let block = r#"{"number":"0x100","timestamp":"0x6a000000","hash":"0x1111111111111111111111111111111111111111111111111111111111111111","parentHash":"0x2222222222222222222222222222222222222222222222222222222222222222","baseFeePerGas":"0x1"}"#;
    let result = match method.as_str() {
        "eth_chainId" => format!("\"0x{chain:x}\""),
        "eth_blockNumber" => "\"0x100\"".to_string(),
        "eth_getBlockByNumber" | "eth_getBlockByHash" => block.to_string(),
        "eth_getLogs" => "[]".to_string(),
        "eth_getCode" => "\"0x\"".to_string(),
        "eth_getTransactionByHash" if params.contains(tx) => {
            format!(r#"{{"hash":"{tx}","from":"0x00000000000000000000000000000000000000f1","to":"0x4e59b44847b379578588920ca78fbf26c0b4956c","input":"0x{}","blockNumber":"0x5"}}"#, "ab".repeat(8))
        }
        "eth_getTransactionReceipt" if params.contains(tx) => r#"{"status":"0x1","blockNumber":"0x5","logs":[]}"#.to_string(),
        "eth_getTransactionReceipt" => r#"{"status":"0x1","blockNumber":"0x3","logs":[]}"#.to_string(),
        "debug_traceTransaction" => format!(
            r#"{{"type":"CALL","from":"0x00000000000000000000000000000000000000f1","to":"0x4e59b44847b379578588920ca78fbf26c0b4956c","input":"0x","calls":[{{"type":"CREATE2","from":"0x4e59b44847b379578588920ca78fbf26c0b4956c","to":"{instance}","input":"{initcode}"}}]}}"#
        ),
        _ => "null".to_string(),
    };
    format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{result}}}"#)
}

fn node(asked: Asked) -> String {
    let c = case();
    let initcode = eravon_standard::order::hex_of(&eravon_terms::derive::initcode(&c.logic));
    let (chain, tx, instance) = (c.chain_id, format!("0x{}", "ea".repeat(32)), c.instance.clone());
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("占得到端口");
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for s in l.incoming() {
            let Ok(mut s) = s else { continue };
            let (asked, tx, instance, initcode) = (asked.clone(), tx.clone(), instance.clone(), initcode.clone());
            std::thread::spawn(move || loop {
                let mut head = Vec::new();
                let mut b = [0u8; 1];
                while !head.ends_with(b"\r\n\r\n") {
                    match s.read(&mut b) {
                        Ok(1) => head.push(b[0]),
                        _ => return,
                    }
                }
                let h = String::from_utf8_lossy(&head).to_ascii_lowercase();
                let n = h.split("content-length:").nth(1).and_then(|x| x.split("\r\n").next()).and_then(|x| x.trim().parse::<usize>().ok()).unwrap_or(0);
                let mut body = vec![0u8; n];
                if s.read_exact(&mut body).is_err() {
                    return;
                }
                let v = eravon_literals::json::read(&body).unwrap_or(In::Null);
                let reply = match &v {
                    In::Arr(qs) => format!("[{}]", qs.iter().map(|q| answer(q, chain, &tx, &instance, &initcode, &asked)).collect::<Vec<_>>().join(",")),
                    q => answer(q, chain, &tx, &instance, &initcode, &asked),
                };
                let out = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{reply}", reply.len());
                if s.write_all(out.as_bytes()).is_err() {
                    return;
                }
            });
        }
    });
    format!("http://127.0.0.1:{port}")
}

fn erk(home: &std::path::Path, url: &str) -> Vec<u8> {
    let o = std::process::Command::new(env!("CARGO_BIN_EXE_erk"))
        .args(["check", "--dossier", &dossier().display().to_string(), "--endpoint", url])
        .env(eravon_os::HOME_VAR, home)
        .env_remove("XDG_DATA_HOME")
        .output()
        .expect("erk 起得来");
    assert!(!o.stdout.is_empty(), "erk 答了:{}", String::from_utf8_lossy(&o.stderr));
    o.stdout
}

const HEAD: u64 = 0x100;

fn log_starts(a: &[(String, String)]) -> Vec<u64> {
    a.iter()
        .filter(|(m, _)| m == "eth_getLogs")
        .filter_map(|(_, p)| p.split("\"fromBlock\":\"0x").nth(1)?.split('"').next().and_then(|h| u64::from_str_radix(h, 16).ok()))
        .collect()
}

fn tally(a: &[(String, String)]) -> (usize, usize, usize) {
    let logs = a.iter().filter(|(m, _)| m == "eth_getLogs").count();
    let blocks = a.iter().filter(|(m, p)| (m == "eth_getBlockByNumber" || m == "eth_getBlockByHash") && p.ends_with(",true]")).count();
    (logs, blocks, a.len())
}

fn files(d: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![d.to_path_buf()];
    while let Some(p) = stack.pop() {
        for e in std::fs::read_dir(&p).into_iter().flatten().flatten() {
            let q = e.path();
            if q.is_dir() {
                stack.push(q);
            } else if let Ok(b) = std::fs::read(&q) {
                out.push((q, b));
            }
        }
    }
    out
}

fn scratch(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("eravon-recheck-kept-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("scratch");
    d
}

#[test]
fn a_second_re_check_on_this_machine_asks_less_and_answers_the_same() {
    let asked: Asked = Default::default();
    let url = node(asked.clone());
    let home = scratch("home");
    let machine = eravon_os::machine::of(&home).expect("机器目录的位置").0;
    std::fs::create_dir_all(&machine).expect("机器目录");
    let first = erk(&home, &url);
    let l1 = std::mem::take(&mut *asked.lock().unwrap());
    let one = tally(&l1);
    let second = erk(&home, &url);
    let l2 = std::mem::take(&mut *asked.lock().unwrap());
    let two = tally(&l2);
    let (from1, from2) = (log_starts(&l1), log_starts(&l2));
    note("erk_kept", format!("ASKED erk-check d002 on one machine: first logs={} blocks={} questions={} log starts {from1:?}; second logs={} blocks={} questions={} log starts {from2:?}", one.0, one.1, one.2, two.0, two.1, two.2));
    assert_eq!(first, second, "两趟的答逐字相同");
    let floor = HEAD - eravon_chain::settled::MARGIN;
    assert!(!from1.is_empty() && from1.iter().all(|f| *f == 0), "头一趟的日志从头读:{from1:?}");
    assert!(from2.len() == from1.len() && from2.iter().all(|f| *f > floor), "第二趟的日志只问落定之后:{from2:?}(落定到 {floor})");
    let rest = |a: &[(String, String)]| a.iter().filter(|(m, _)| m != "eth_getLogs").cloned().collect::<std::collections::BTreeSet<_>>();
    assert!(rest(&l2).is_subset(&rest(&l1)), "第二趟没有头一趟没问过的问:{:?}", rest(&l2).difference(&rest(&l1)).collect::<Vec<_>>());
    assert!(two.0 <= one.0 && two.1 <= one.1, "第二趟的日志与整块不多于头一趟:{one:?} → {two:?}");
    assert_eq!(two.1, 0, "整块原文不再取");
    let kept = files(&machine);
    assert!(!kept.is_empty(), "本机留下了核过的事实");
    for (p, b) in &kept {
        let s = String::from_utf8_lossy(b);
        assert!(!s.contains("127.0.0.1") && !s.contains("http://") && !s.contains("localhost"), "留档里有节点地址:{}", p.display());
        assert!(!s.contains("passcode") && !s.contains("privkey") && !s.contains("phrase"), "留档里有钥或口令:{}", p.display());
    }
    let me = machine.join("identities").join("0x98e503f35d0a019cb0a251ad243a4ccfcf371f46");
    std::fs::create_dir_all(me.join("orders").join("x")).expect("身份目录");
    std::fs::write(me.join("candidates.json"), br#"{"candidates":[{"address":"0x00000000000000000000000000000000000000aa","kind":"judge"}]}"#).expect("候选表");
    std::fs::write(me.join("orders").join("x").join("record.json"), format!(r#"{{"instance":"{}","state":"settled"}}"#, case().instance)).expect("订单记录");
    assert_eq!(erk(&home, &url), first, "产品数据不改答");
    let bare = scratch("bare");
    assert_eq!(erk(&bare, &url), first, "没有机器目录照冷读");
    assert!(files(&bare).is_empty() && std::fs::read_dir(&bare).map(|r| r.count()).unwrap_or(0) == 0, "没有为它建目录");
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&bare);
}

fn note(test: &str, line: String) {
    println!("{line}");
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("asked");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(format!("{test}.txt")), format!("{line}\n"));
}
