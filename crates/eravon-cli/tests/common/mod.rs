#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

pub const BIN: &str = env!("CARGO_BIN_EXE_eravon");

pub const PIN: &str = "48261537";
pub const PHRASE: &str = "test test test test test test test test test test test junk";

pub const CHAIN: u64 = 31337;
pub const TO: &str = "0x00000000000000000000000000000000000000b0";
pub const TOKEN: &str = "0x00000000000000000000000000000000000000c0";
pub const AMOUNT: &str = "1000";

#[derive(Clone)]
pub enum Reply {
    Say(u16, String),
    Close,
}

pub fn ok(result: &str) -> Reply {
    Reply::Say(200, format!("{{\"jsonrpc\":\"2.0\",\"id\":{{id}},\"result\":{result}}}"))
}

pub fn error(status: u16, err: &str) -> Reply {
    Reply::Say(status, format!("{{\"jsonrpc\":\"2.0\",\"id\":{{id}},\"error\":{err}}}"))
}

#[derive(Clone)]
pub struct Scene {
    pub estimate: Reply,
    pub head: Reply,
    pub pending: Option<u64>,
    pub latest: u64,
    pub refuse: bool,
    pub receipt: Option<String>,
    pub holds: bool,
    pub only: Option<usize>,
    pub logs: Option<String>,
    pub hang_receipt: bool,
    pub hang: Vec<&'static str>,
    pub fail: Vec<&'static str>,
    pub log_cap: Option<u64>,
    pub natives: Vec<(String, String)>,
}

impl Default for Scene {
    fn default() -> Scene {
        Scene { estimate: ok("\"0x5208\""), head: ok("\"0x40\""), pending: Some(0), latest: 0, refuse: false, receipt: None, holds: false, only: None, logs: None, hang_receipt: false, hang: Vec::new(), fail: Vec::new(), log_cap: None, natives: Vec::new() }
    }
}

pub struct Node {
    pub url: String,
    scene: Arc<Mutex<Scene>>,
    asked: Arc<Mutex<Vec<String>>>,
    pub questions: Arc<Mutex<Vec<(String, String)>>>,
    pub raws: Arc<Mutex<Vec<Vec<u8>>>>,
    pub held: Arc<Mutex<Vec<std::time::Duration>>>,
}

impl Node {
    pub fn set(&self, s: Scene) {
        *self.scene.lock().unwrap_or_else(|e| e.into_inner()) = s;
        self.asked.lock().unwrap_or_else(|e| e.into_inner()).clear();
        self.questions.lock().unwrap_or_else(|e| e.into_inner()).clear();
        self.held.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
    pub fn asked(&self, m: &str) -> usize {
        self.asked.lock().unwrap_or_else(|e| e.into_inner()).iter().filter(|x| *x == m).count()
    }
    pub fn broadcasts(&self) -> usize {
        self.raws.lock().unwrap_or_else(|e| e.into_inner()).len()
    }
    pub fn hash(&self, n: usize) -> String {
        eravon::hexfmt::encode(&eravon::cryptox::keccak256(&self.raws.lock().unwrap_or_else(|e| e.into_inner())[n]))
    }
}

fn body_of(s: &mut std::net::TcpStream) -> Option<String> {
    let mut raw = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match s.read(&mut buf) {
            Ok(0) | Err(_) => return None,
            Ok(n) => raw.extend_from_slice(&buf[..n]),
        }
        let text = String::from_utf8_lossy(&raw).to_string();
        if let Some(i) = text.find("\r\n\r\n") {
            let len: usize = text[..i].lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").and_then(|n| n.trim().parse().ok())).unwrap_or(0);
            if raw.len() >= i + 4 + len {
                return Some(text[i + 4..].to_string());
            }
        }
    }
}

pub fn node() -> Node {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", l.local_addr().expect("addr"));
    let scene: Arc<Mutex<Scene>> = Arc::new(Mutex::new(Scene::default()));
    let asked: Arc<Mutex<Vec<String>>> = Arc::default();
    let raws: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();
    let questions: Arc<Mutex<Vec<(String, String)>>> = Arc::default();
    let held: Arc<Mutex<Vec<std::time::Duration>>> = Arc::default();
    let (sc, a, r, q, hd) = (scene.clone(), asked.clone(), raws.clone(), questions.clone(), held.clone());
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let (sc, a, r, q, hd) = (sc.clone(), a.clone(), r.clone(), q.clone(), hd.clone());
            std::thread::spawn(move || {
                let mut s = s;
                let Some(body) = body_of(&mut s) else { return };
                let req = eravon::json::parse(body.as_bytes()).unwrap_or(eravon::json::Value::Null);
                let method = req.member("method").and_then(|m| m.as_str()).unwrap_or("").to_string();
                let params: Vec<eravon::json::Value> = match req.member("params") {
                    Some(eravon::json::Value::Arr(p)) => p.clone(),
                    _ => Vec::new(),
                };
                let id = body.split("\"id\":").nth(1).and_then(|x| x.split([',', '}']).next()).unwrap_or("1").trim().to_string();
                a.lock().unwrap_or_else(|e| e.into_inner()).push(method.clone());
                let canon = String::from_utf8_lossy(&eravon::json::canon_bytes(&eravon::json::Value::Arr(params.clone()))).into_owned();
                q.lock().unwrap_or_else(|e| e.into_inner()).push((method.clone(), canon));
                let scene = sc.lock().unwrap_or_else(|e| e.into_inner()).clone();
                if scene.fail.contains(&method.as_str()) {
                    let body = format!("{{\"error\":{{\"code\":-32603,\"message\":\"internal error\"}},\"id\":{id},\"jsonrpc\":\"2.0\"}}");
                    let _ = s.write_all(format!("HTTP/1.1 200 X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
                    return;
                }
                if (scene.hang_receipt && method == "eth_getTransactionReceipt") || scene.hang.contains(&method.as_str()) {
                    let start = std::time::Instant::now();
                    let mut buf = [0u8; 64];
                    while matches!(s.read(&mut buf), Ok(n) if n > 0) {}
                    hd.lock().unwrap_or_else(|e| e.into_inner()).push(start.elapsed());
                    return;
                }
                let reply = match method.as_str() {
                    "eth_chainId" => ok("\"0x7a69\""),
                    "eth_blockNumber" => scene.head.clone(),
                    "eth_getBlockByNumber" => ok("{\"baseFeePerGas\":\"0x3b9aca00\",\"number\":\"0x40\",\"timestamp\":\"0x64\"}"),
                    "eth_feeHistory" => ok("{\"oldestBlock\":\"0x2d\",\"reward\":[[\"0x5f5e100\"]]}"),
                    "eth_estimateGas" => scene.estimate.clone(),
                    "eth_call" => ok("\"0x00000000000000000000000000000000000000000000000000000000000f4240\""),
                    "eth_getBalance" => {
                        let who = params.first().and_then(|x| x.as_str()).unwrap_or_default().to_ascii_lowercase();
                        match scene.natives.iter().find(|(a, _)| *a == who) {
                            Some((_, v)) => ok(v),
                            None => ok("\"0xde0b6b3a7640000\""),
                        }
                    }
                    "eth_getTransactionCount" => match params.get(1).and_then(|x| x.as_str()) {
                        Some("pending") => match scene.pending {
                            Some(n) => ok(&format!("\"0x{n:x}\"")),
                            None => ok("\"not a number\""),
                        },
                        _ => ok(&format!("\"0x{:x}\"", scene.latest)),
                    },
                    "eth_sendRawTransaction" => {
                        let bytes = params.first().and_then(|x| x.as_str()).and_then(eravon::hexfmt::decode).unwrap_or_default();
                        let h = eravon::cryptox::keccak256(&bytes);
                        r.lock().unwrap_or_else(|e| e.into_inner()).push(bytes);
                        if scene.refuse {
                            error(200, "{\"code\":-32000,\"message\":\"insufficient funds for gas * price + value\"}")
                        } else {
                            ok(&format!("\"{}\"", eravon::hexfmt::encode(&h)))
                        }
                    }
                    "eth_getTransactionReceipt" => {
                        let asked = params.first().and_then(|x| x.as_str()).unwrap_or_default().to_ascii_lowercase();
                        let this = scene.only.map(|i| r.lock().unwrap_or_else(|e| e.into_inner()).get(i).map(|b| eravon::hexfmt::encode(&eravon::cryptox::keccak256(b))) == Some(asked.clone()));
                        match this {
                            Some(false) => ok("null"),
                            _ => ok(&scene.receipt.clone().unwrap_or_else(|| "null".into())),
                        }
                    }
                    "eth_getLogs" if scene.log_cap.is_some_and(|cap| {
                        let at = |k: &str| params.first().and_then(|f| f.member(k)).and_then(|v| v.as_str()).and_then(|h| u64::from_str_radix(h.trim_start_matches("0x"), 16).ok());
                        matches!((at("fromBlock"), at("toBlock")), (Some(f), Some(t)) if t - f + 1 > cap)
                    }) => error(200, &format!("{{\"code\":-32012,\"message\":\"exceed maximum block range: {}\"}}", scene.log_cap.unwrap_or(0))),
                    "eth_getLogs" if scene.logs.is_some() => ok(scene.logs.as_deref().unwrap_or("[]")),
                    "eth_getTransactionByHash" if scene.holds => {
                        let h = params.first().and_then(|x| x.as_str()).unwrap_or_default().to_string();
                        ok(&format!("{{\"blockNumber\":null,\"hash\":\"{h}\",\"nonce\":\"0x0\"}}"))
                    }
                    _ => ok("null"),
                };
                if let Reply::Say(status, body) = reply {
                    let body = body.replace("{id}", &id);
                    let _ = s.write_all(format!("HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes());
                }
            });
        }
    });
    Node { url, scene, asked, questions, raws, held }
}

pub struct Home {
    pub root: PathBuf,
    pub home: PathBuf,
    pub data: PathBuf,
    pub address: String,
}

impl Home {
    pub fn new(tag: &str) -> Home {
        let mut h = Home::bare(tag);
        let made = h.run(&["buyer-vault-init", "--data", h.data_s(), "--writer", "t", "--now", "1758412800", "--passcode", PIN, "--phrase", PHRASE], |_| {});
        assert_eq!(made.code, 0, "the vault is made: {} {}", made.text, made.err);
        h.address = made.get("address");
        h
    }

    pub fn bare(tag: &str) -> Home {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("eravon-cli-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (home, data) = (root.join("home"), root.join("data"));
        std::fs::create_dir_all(&home).expect("a home");
        std::fs::create_dir_all(&data).expect("a data root");
        Home { root, home, data, address: String::new() }
    }

    pub fn data_s(&self) -> &str {
        self.data.to_str().expect("utf-8")
    }

    pub fn run(&self, args: &[&str], set: impl FnOnce(&mut Command)) -> Ran {
        let mut c = Command::new(BIN);
        c.args(args).env(eravon_os::HOME_VAR, &self.home);
        set(&mut c);
        let o = c.output().expect("eravon runs");
        let text = String::from_utf8_lossy(&o.stdout).to_string();
        Ran { code: o.status.code().unwrap_or(-1), out: eravon::json::parse(&o.stdout).unwrap_or(eravon::json::Value::Null), text, err: String::from_utf8_lossy(&o.stderr).to_string() }
    }

    pub fn send(&self, n: &Node) -> Ran {
        self.send_where(n, |_| {})
    }

    pub fn send_where(&self, n: &Node, set: impl FnOnce(&mut Command)) -> Ran {
        self.run(&self.send_args(n), set)
    }

    pub fn send_args<'a>(&'a self, n: &'a Node) -> Vec<&'a str> {
        vec![
            "buyer-wallet-send", "--data", self.data_s(), "--writer", "t", "--now", "1758412800", "--endpoint", &n.url, "--chain", "31337",
            "--to", TO, "--token", TOKEN, "--amount", AMOUNT, "--send", "true", "--wait", "0", "--passcode", PIN,
        ]
    }

    pub fn machine(&self) -> PathBuf {
        eravon_os::machine::of(&self.home).expect("the machine folder").0
    }

    pub fn record(&self) -> PathBuf {
        let a = |x: &str| -> [u8; 20] { eravon::hexfmt::decode(x).and_then(|b| b.try_into().ok()).expect("an account") };
        let call = transfer_data();
        eravon_send::sent::place(&self.machine(), CHAIN, &a(&self.address), &a(TOKEN), &call, &[0u8; 32])
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

pub fn transfer_data() -> Vec<u8> {
    let mut d = vec![0xa9, 0x05, 0x9c, 0xbb];
    let mut to = [0u8; 32];
    to[12..].copy_from_slice(&eravon::hexfmt::decode(TO).expect("hex"));
    d.extend_from_slice(&to);
    let mut amount = [0u8; 32];
    amount[24..].copy_from_slice(&AMOUNT.parse::<u64>().expect("a number").to_be_bytes());
    d.extend_from_slice(&amount);
    d
}

pub struct Ran {
    pub code: i32,
    pub out: eravon::json::Value,
    pub text: String,
    pub err: String,
}

impl Ran {
    pub fn get(&self, k: &str) -> String {
        self.out.member(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
    }
}

pub fn tree(at: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut dirs = vec![at.to_path_buf()];
    while let Some(d) = dirs.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                dirs.push(p);
            } else {
                let bytes = std::fs::read(&p).unwrap_or_default();
                out.push((p, bytes));
            }
        }
    }
    out.sort();
    out
}

pub fn seal(at: &Path) -> eravon_send::sent::Seal {
    let entropy = eravon_vault::cryptx::entropy_of(PHRASE).expect("the words");
    let key = eravon_vault::cryptx::derive(&entropy, &eravon_vault::family::path(eravon_vault::roles::Role::Author)).expect("a key");
    eravon_send::sent::Seal::of(&key, at)
}

pub fn recorded(at: &Path) -> Vec<(u64, [u8; 32])> {
    let entropy = eravon_vault::cryptx::entropy_of(PHRASE).expect("the words");
    let keys: Vec<[u8; 32]> = eravon_vault::roles::Role::ALL.iter().filter_map(|r| eravon_vault::cryptx::derive(&entropy, &eravon_vault::family::path(*r))).collect();
    keys.iter().find_map(|k| eravon_send::sent::read(at, &eravon_send::sent::Seal::of(k, at)).ok()).expect("the record reads with a seat's key")
}

#[allow(dead_code)]
pub fn terms_block(dir: &Path) -> PathBuf {
    let ladder = vec![("驳回".to_string(), 0), ("全额退款".to_string(), 10_000)];
    let judges = vec![eravon_goods::resolve::JudgeEl {
        judge: "0x00000000000000000000000000000000000000b1".into(),
        amanah_hash: format!("0x{}", "11".repeat(32)),
        quote: "0x0102".into(),
        source: None,
    }];
    let g = eravon_goods::terms::generate(&eravon_goods::terms::Parts {
        name: "海报",
        description: "一张 A2 海报",
        delivery_note: "PNG 一份",
        goods_private: false,
        delivery_private: false,
        seller_key: "0x00000000000000000000000000000000000000a1",
        seller_payout: "0x00000000000000000000000000000000000000a2",
        seller_fund: "0x00000000000000000000000000000000000000a3",
        token: "0x00000000000000000000000000000000000000a4",
        amount: "1000",
        t1_secs: 86_400,
        t2_secs: 3_600,
        open_window: 7_200,
        buyer_may_open: false,
        ladder: &ladder,
        judges: &judges,
        guilds: &[],
        nonce: 1,
        expiry: 2_000_000_000,
        series: None,
    });
    let text = String::from_utf8(g.bytes).expect("utf-8");
    let block = text.lines().nth(1).expect("the block line");
    let p = dir.join("block.json");
    std::fs::write(&p, block).expect("a block file");
    p
}
