use std::sync::Mutex;

use eravon_chain::endpoints::Table;
use eravon_chain::seam::Url;
use eravon_chain::transport::{Answer, Transport};
use eravon_literals::json::In;

const CHILD: &str = "ERAVON_CHAIN_TEST_TRACE_LACK_CHILD";

const TRACES: u16 = 31601;
const LIMITED: u16 = 31602;

#[derive(Clone)]

struct Gate {
    asked: std::sync::Arc<Mutex<Vec<(u16, String)>>>,
}

impl Gate {
    fn answer_one(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        let v = eravon_literals::json::read(body).expect("问是 JSON");
        let method = v.get("method").and_then(In::as_str).unwrap_or_default().to_string();
        let id = match v.get("id") {
            Some(In::Int(n)) => n.to_string(),
            _ => "0".into(),
        };
        self.asked.lock().unwrap().push((url.port, method.clone()));
        let result = match method.as_str() {
            "eth_chainId" => r#""result":"0x1""#.to_string(),
            "debug_traceTransaction" if url.port == TRACES => r#""result":{"from":"0xe0","to":"0xc0","input":"0x","type":"CALL","calls":[]}"#.to_string(),
            _ => r#""error":{"code":-32005,"message":"rate limit exceeded"}"#.to_string(),
        };
        Ok(Answer { status: 200, location: None, body: format!(r#"{{"id":{id},"jsonrpc":"2.0",{result}}}"#).into_bytes() })
    }
}

impl Transport for Gate {
    fn post(&self, url: &Url, body: &[u8]) -> eravon_chain::fault::R<Answer> {
        eravon_chain::ask::each_question(body, |b| self.answer_one(url, b))
    }
}

struct File(std::path::PathBuf);

impl eravon_chain::settled::Keep for File {
    fn load(&self, part: &str) -> Option<Vec<u8>> {
        std::fs::read(self.0.join(part)).ok()
    }
    fn save(&self, part: &str, plain: &[u8]) -> Result<(), eravon_chain::settled::Unkept> {
        std::fs::create_dir_all(&self.0).expect("建夹");
        std::fs::write(self.0.join(part), plain).expect("记下");
        Ok(())
    }
    fn parts(&self) -> Vec<String> {
        std::fs::read_dir(&self.0).map(|d| d.filter_map(|e| e.ok()?.file_name().into_string().ok()).collect()).unwrap_or_default()
    }
}

#[test]
fn lack_child() {
    let Some(file) = std::env::var_os(CHILD) else { return };
    eravon_chain::patience::set_waits(Some(std::time::Duration::ZERO));
    let t = Gate { asked: std::sync::Arc::new(Mutex::new(Vec::new())) };
    let table = Table::new(vec![format!("http://127.0.0.1:{TRACES}"), format!("http://127.0.0.1:{LIMITED}")]).unwrap();
    let _ = table.chain.set(1);
    let _a = eravon_chain::action::open(Some(std::sync::Arc::new(File(file.into())) as std::sync::Arc<dyn eravon_chain::settled::Keep>));
    let tx = format!("0x{}", "ab".repeat(32));
    eravon_chain::trace::find(&t, &table, &tx, "0xaa", "0x11111111").expect("追踪");
    let n = |port: u16| t.asked.lock().unwrap().iter().filter(|(p, m)| *p == port && (m == "debug_traceTransaction" || m == "trace_transaction")).count();
    println!("traced {} {}", n(TRACES), n(LIMITED));
}

#[test]
fn a_node_limited_on_tracing_in_two_processes_is_asked_by_the_third_as_by_the_first() {
    let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("trace-lack-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&file);
    let run = || -> (usize, usize) {
        let o = std::process::Command::new(std::env::current_exe().expect("测试二进制"))
            .args(["lack_child", "--exact", "--nocapture", "--test-threads=1"])
            .env(CHILD, &file)
            .output()
            .expect("起子进程");
        let text = String::from_utf8_lossy(&o.stdout).to_string();
        assert!(o.status.success(), "子进程失败:{text}{}", String::from_utf8_lossy(&o.stderr));
        let line = text.lines().find_map(|l| l.split_once("traced ").map(|(_, n)| n.trim().to_string())).unwrap_or_else(|| panic!("子进程没报数:{text}"));
        let n: Vec<usize> = line.split_whitespace().map(|x| x.parse().expect("数")).collect();
        (n[0], n[1])
    };
    let first = run();
    let second = run();
    let third = run();
    note("trace_lack", format!("ASKED trace by process (traces, limited): {first:?} {second:?} {third:?}"));
    assert!(first.1 > 0 && second.1 > 0, "头两个进程都问了限流的那家");
    assert_eq!(third.1, first.1, "限流不算缺方法:第三个进程照第一个进程那样问那一家");
    assert_eq!((first.0, second.0, third.0), (1, 1, 1), "供追踪的那家每个进程第一问就答");
    let _ = std::fs::remove_file(&file);
}

fn note(test: &str, line: String) {
    println!("{line}");
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("asked");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(format!("{test}.txt")), format!("{line}\n"));
}
