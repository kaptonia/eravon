#![cfg(unix)]

use eravon::json::Value;
use eravon_cli::door::{Reply, Request};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const BIN: &str = env!("CARGO_BIN_EXE_eravon");
const EN: &str = "en_US.UTF-8";

struct Bench {
    root: PathBuf,
    user: PathBuf,
    machine: PathBuf,
    data: PathBuf,
}

impl Bench {
    fn new(tag: &str) -> Bench {
        let base = if std::env::temp_dir().as_os_str().len() <= 40 { std::env::temp_dir() } else { PathBuf::from("/tmp") };
        let root = base.join(format!("evm-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (user, machine, data) = (root.join("u"), root.join("m"), root.join("h"));
        for d in [&user, &machine, &data] {
            std::fs::create_dir_all(d).expect("dir");
        }
        let machine = std::fs::canonicalize(&machine).expect("machine");
        let data = std::fs::canonicalize(&data).expect("data");
        let ptr = eravon_os::machine::pointer_dir(&user);
        std::fs::create_dir_all(&ptr).expect("pointer folder");
        std::fs::write(ptr.join(eravon_os::machine::POINTER), format!("{}\n", machine.display())).expect("pointer");
        Bench { root, user, machine, data }
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(BIN);
        c.args(args).current_dir(&self.root).env(eravon_os::HOME_VAR, &self.user).env_remove("LC_ALL").env_remove("LC_MESSAGES").env_remove("LANGUAGE").env("LANG", EN);
        c
    }

    fn typed(&self, args: &[&str]) -> (i32, Vec<u8>, String) {
        let out = self.command(args).stdin(Stdio::null()).output().expect("runs");
        (out.status.code().unwrap_or(-1), out.stdout, String::from_utf8_lossy(&out.stderr).into_owned())
    }

    fn session(&self) -> Session {
        self.session_with(&[])
    }

    fn session_with(&self, flags: &[&str]) -> Session {
        let args: Vec<&str> = std::iter::once("mcp").chain(flags.iter().copied()).collect();
        let mut child = self.command(&args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().expect("mcp starts");
        let out = child.stdout.take().expect("stdout");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(out).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        Session { child, lines: rx }
    }

    fn door(&self) -> PathBuf {
        eravon_cli::door::place(&self.machine, &self.data)
    }

    fn data_s(&self) -> &str {
        self.data.to_str().expect("utf-8")
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

struct Session {
    child: std::process::Child,
    lines: mpsc::Receiver<String>,
}

impl Session {
    fn send(&mut self, line: &str) {
        let input = self.child.stdin.as_mut().expect("stdin");
        input.write_all(line.as_bytes()).expect("writes");
        input.write_all(b"\n").expect("writes");
        input.flush().expect("flush");
    }

    fn next(&self, secs: u64) -> Option<(String, Value)> {
        let line = self.lines.recv_timeout(Duration::from_secs(secs)).ok()?;
        let v = eravon::json::parse_tests_1_3(line.replacen("\"code\":-", "\"code\":", 1).as_bytes()).ok()?;
        Some((line, v))
    }

    fn ask(&mut self, line: &str) -> (String, Value) {
        self.send(line);
        self.next(60).expect("an answer")
    }

    fn call(&mut self, id: u64, tool: &str, args: &str) -> Value {
        self.ask(&format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"{tool}","arguments":{args}}}}}"#)).1
    }

    fn end(mut self) -> i32 {
        drop(self.child.stdin.take());
        self.child.wait().expect("ends").code().unwrap_or(-1)
    }
}

fn result(v: &Value) -> &Value {
    v.member("result").unwrap_or_else(|| panic!("no result: {v:?}"))
}

fn text_of(r: &Value) -> String {
    r.member("content").and_then(|c| c.as_arr()).and_then(|c| c.first()).and_then(|c| c.member("text")).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn exit_of(r: &Value) -> Option<u64> {
    r.member("structuredContent").and_then(|s| s.member("exit")).and_then(Value::as_int)
}

fn is_error(r: &Value) -> bool {
    r.member("isError") == Some(&Value::Bool(true))
}

fn error_code(v: &Value) -> Option<u64> {
    v.member("error").and_then(|e| e.member("code")).and_then(Value::as_int)
}

fn desktop(at: &Path, answer: impl Fn(&Request) -> Option<Vec<u8>> + Send + 'static) -> (eravon_os::door::Closer, mpsc::Receiver<Request>) {
    let (l, closer) = eravon_os::door::listen(at).expect("the door opens");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(Some(mut s)) = l.accept() {
            let Ok(bytes) = eravon_cli::door::take(&mut s) else { continue };
            let Some(q) = Request::of_bytes(&bytes) else { continue };
            if let Some(r) = answer(&q) {
                let _ = eravon_cli::door::put(&mut s, &r);
            }
            let _ = tx.send(q);
        }
    });
    (closer, rx)
}

#[test]
fn the_handshake_answers_one_of_two_versions() {
    let b = Bench::new("hello");
    for (asked, answered) in [("2025-11-25", "2025-11-25"), ("2025-06-18", "2025-06-18"), ("2024-11-05", "2025-11-25")] {
        let mut s = b.session();
        let (_, v) = s.ask(&format!(r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"{asked}"}}}}"#));
        let r = result(&v);
        assert_eq!(r.member("protocolVersion").and_then(Value::as_str), Some(answered));
        assert_eq!(r.member("serverInfo").and_then(|i| i.member("name")).and_then(Value::as_str), Some("eravon"));
        assert_eq!(s.end(), 0, "the input closed, the session ends");
    }
}

#[test]
fn the_tools_listed_are_the_products_table() {
    let b = Bench::new("list");
    let mut s = b.session();
    let (_, v) = s.ask(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#);
    let listed = result(&v).member("tools").expect("tools");
    let table = eravon_cli::mcp::listed(&eravon_cli::mcptools::TABLE);
    assert_eq!(eravon::json::canon_bytes(listed), eravon::json::canon_bytes(table.member("tools").expect("the table's")));
    assert!(listed.as_arr().is_some_and(|tools| !tools.is_empty()), "the table lists tools");
    s.end();
}

#[test]
fn a_read_tool_answers_as_the_terminal() {
    let b = Bench::new("read");
    let (code, out, _) = b.typed(&["contract"]);
    let mut s = b.session();
    let r = s.call(1, "contract", "{}");
    let r = result(&r);
    assert_eq!(text_of(r).as_bytes(), out.as_slice());
    assert_eq!((exit_of(r), is_error(r)), (Some(code as u64), false));
    s.end();
}

#[test]
fn a_write_goes_through_the_desktop_and_answers_as_the_terminal() {
    let b = Bench::new("write");
    let args = format!(r#"{{"desktop":"{}","now":"1800000000","zone":"utc"}}"#, b.data_s());
    let mut s = b.session();
    let none = s.call(1, "seller-settings-zone", &args);
    let none = result(&none);
    assert_eq!((exit_of(none), is_error(none)), (Some(4), false));
    assert!(text_of(none).contains("E_DESKTOP"), "{}", text_of(none));
    let (closer, came) = desktop(&b.door(), |_| Some(Reply::Answered { body: r#"{"ok":true,"zone":"utc"}"#.into() }.to_bytes()));
    let through = s.call(2, "seller-settings-zone", &args);
    let through = result(&through);
    assert_eq!((exit_of(through), is_error(through)), (Some(0), false));
    let q = came.recv_timeout(Duration::from_secs(30)).expect("the desktop was asked");
    assert_eq!((q.verb.as_str(), q.home.as_str()), ("seller-settings-zone", b.data_s()));
    closer.close();
    s.end();
}

#[test]
fn what_is_refused_is_refused_by_name() {
    let b = Bench::new("refuse");
    let mut s = b.session();
    let missing = s.call(1, "seller-settings-zone", r#"{"zone":"utc"}"#);
    let missing = result(&missing);
    assert!(is_error(missing) && exit_of(missing) == Some(2) && text_of(missing).starts_with("E_ARGS --desktop"), "{}", text_of(missing));
    let relative = s.call(2, "seller-settings-zone", r#"{"desktop":"h","zone":"utc"}"#);
    let relative = result(&relative);
    assert!(is_error(relative) && text_of(relative).starts_with("E_ARGS --desktop"), "{}", text_of(relative));
    let secret = s.call(3, "seller-vault-list", &format!(r#"{{"data":"{}","passcode":"x"}}"#, b.data_s()));
    let secret = result(&secret);
    assert!(is_error(secret) && exit_of(secret) == Some(2) && text_of(secret).starts_with("E_ARGS #2"), "{}", text_of(secret));
    for (line, code) in [
        (r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"seller-vault-init","arguments":{}}}"#, 32602),
        (r#"{"jsonrpc":"2.0","id":5,"method":"resources/list"}"#, 32601),
        ("not json", 32700),
        (r#"{"jsonrpc":"2.0","id":6,"id":7,"method":"ping"}"#, 32600),
    ] {
        let (raw, v) = s.ask(line);
        assert_eq!(error_code(&v), Some(code), "{raw}");
        assert!(raw.contains(&format!("\"code\":-{code}")), "an error's code is written with its sign: {raw}");
    }
    assert_eq!(s.end(), 0);
    let (code, out, _) = b.typed(&["mcp", "--data", b.data_s()]);
    assert_eq!((code, out.len()), (2, 0));
}

#[test]
fn a_call_cancelled_before_it_starts_is_not_done() {
    let b = Bench::new("cancel");
    let (release, held) = mpsc::channel::<()>();
    let held = std::sync::Mutex::new(held);
    let (closer, came) = desktop(&b.door(), move |_| {
        let _ = held.lock().map(|h| h.recv_timeout(Duration::from_secs(30)));
        Some(Reply::Closed.to_bytes())
    });
    let mut s = b.session();
    s.send(&format!(r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"seller-settings-zone","arguments":{{"desktop":"{}","now":"1800000000","zone":"utc"}}}}}}"#, b.data_s()));
    s.send(r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"contract","arguments":{}}}"#);
    s.send(r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":2}}"#);
    let (_, ping) = s.ask(r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#);
    assert_eq!(ping.member("id").and_then(Value::as_int), Some(3));
    release.send(()).expect("release");
    let (_, v) = s.next(60).expect("the write is answered");
    assert_eq!(v.member("id").and_then(Value::as_int), Some(1));
    assert!(text_of(result(&v)).contains("CLOSED"), "the desktop's CLOSED said as it is");
    came.recv_timeout(Duration::from_secs(30)).expect("the write reached the desktop");
    let (_, last) = s.ask(r#"{"jsonrpc":"2.0","id":4,"method":"ping"}"#);
    assert_eq!(last.member("id").and_then(Value::as_int), Some(4), "nothing came for the cancelled call");
    closer.close();
    s.end();
}

#[test]
fn a_write_with_the_passcode_file_runs_here_with_it() {
    const PIN: &str = "48261537";
    const PHRASE: &str = "test test test test test test test test test test test junk";
    let b = Bench::new("pcfile");
    let (made, out, err) = b.typed(&["buyer-vault-init", "--data", b.data_s(), "--writer", "t", "--now", "1758412800", "--passcode", PIN, "--phrase", PHRASE]);
    assert_eq!(made, 0, "the vault is made: {} {err}", String::from_utf8_lossy(&out));
    let file = b.root.join("pc");
    std::fs::write(&file, PIN).expect("the passcode file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).expect("private");
    }
    let file = file.to_str().expect("utf-8").to_string();
    let args = format!(r#"{{"desktop":"{}","now":"1800000000","zone":"utc"}}"#, b.data_s());
    let mut plain = b.session();
    let none = plain.call(1, "buyer-settings-zone", &args);
    assert_eq!(exit_of(result(&none)), Some(4), "without the file it goes to the desktop: {}", text_of(result(&none)));
    plain.end();
    let mut s = b.session_with(&["--passcode-file", &file]);
    let r = s.call(1, "buyer-settings-zone", &args);
    let r = result(&r);
    assert_eq!((exit_of(r), is_error(r)), (Some(0), false), "{}", text_of(r));
    assert!(text_of(r).contains("utc") && !text_of(r).contains(PIN), "{}", text_of(r));
    let (code, typed, _) = b.typed(&["buyer-settings-zone", "--data", b.data_s(), "--passcode-file", &file, "--now", "1800000000", "--zone", "utc"]);
    assert_eq!((code, String::from_utf8_lossy(&typed).to_string()), (0, text_of(r)), "the same command typed in a terminal");
    let read = s.call(2, "buyer-vault-accounts", &format!(r#"{{"desktop":"{}","now":"1800000000"}}"#, b.data_s()));
    let read = result(&read);
    assert_eq!((exit_of(read), is_error(read)), (Some(0), false), "{}", text_of(read));
    let (code, typed, _) = b.typed(&["buyer-vault-accounts", "--data", b.data_s(), "--passcode-file", &file, "--now", "1800000000"]);
    assert_eq!((code, String::from_utf8_lossy(&typed).to_string()), (0, text_of(read)), "the same command typed in a terminal");
    s.end();
}
