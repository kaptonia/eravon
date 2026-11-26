#![cfg(unix)]

use eravon_cli::door::{Reply, Request};
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eravon");

struct Bench {
    root: PathBuf,
    user: PathBuf,
    machine: PathBuf,
    data: PathBuf,
}

impl Bench {
    fn new(tag: &str) -> Bench {
        let base = if std::env::temp_dir().as_os_str().len() <= 40 { std::env::temp_dir() } else { PathBuf::from("/tmp") };
        let root = base.join(format!("evd-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (user, machine, data) = (root.join("u"), root.join("m"), root.join("h"));
        for d in [&user, &machine, &data] {
            std::fs::create_dir_all(d).expect("dir");
        }
        let machine = std::fs::canonicalize(&machine).expect("machine");
        let ptr = eravon_os::machine::pointer_dir(&user);
        std::fs::create_dir_all(&ptr).expect("pointer folder");
        std::fs::write(ptr.join(eravon_os::machine::POINTER), format!("{}\n", machine.display())).expect("pointer");
        Bench { root, user, machine, data }
    }

    fn place(&self) -> PathBuf {
        eravon_cli::door::place(&self.machine, &std::fs::canonicalize(&self.data).expect("data"))
    }

    fn data_s(&self) -> &str {
        self.data.to_str().expect("utf-8")
    }

    fn run(&self, args: &[&str], lang: &str) -> (i32, Vec<u8>, String) {
        let o = Command::new(BIN)
            .args(args)
            .current_dir(&self.root)
            .env(eravon_os::HOME_VAR, &self.user)
            .env_remove("LC_ALL")
            .env_remove("LC_MESSAGES")
            .env_remove("LANGUAGE")
            .env("LANG", lang)
            .output()
            .expect("runs");
        (o.status.code().unwrap_or(-1), o.stdout, String::from_utf8_lossy(&o.stderr).into_owned())
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn desktop(at: &Path, answer: impl Fn(&Request) -> Option<Vec<u8>> + Send + 'static) -> (eravon_os::door::Closer, std::sync::mpsc::Receiver<Request>) {
    let (l, closer) = eravon_os::door::listen(at).expect("the door opens");
    let (tx, rx) = std::sync::mpsc::channel();
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

fn json(b: &[u8]) -> eravon::json::Value {
    eravon::json::parse(b).unwrap_or_else(|_| panic!("not one value: {}", String::from_utf8_lossy(b)))
}

fn text(v: &eravon::json::Value, k: &str) -> String {
    v.member(k).and_then(|x| x.as_str()).unwrap_or_default().to_string()
}

fn has_chinese(s: &str) -> bool {
    s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
}

const EN: &str = "en_US.UTF-8";
const ZH: &str = "zh_CN.UTF-8";

fn verb() -> &'static str {
    eravon_cli::verbs::HOME[0].0
}

#[test]
fn no_desktop_for_that_home_is_no_answer() {
    let b = Bench::new("none");
    for (lang, said) in [
        (EN, "The desktop is locked, not running, not holding this data folder as its writer, or has “Through the desktop app” off; unlock it, turn that switch on in Settings, or give --desktop the data folder the desktop holds"),
        (ZH, "桌面锁着、没有运行、没有以写者开着这个数据目录,或「经桌面 App 操作」关着;解锁桌面、在设置里打开「经桌面 App 操作」,或把 --desktop 改成桌面开着的那个数据目录"),
    ] {
        let (code, out, err) = b.run(&[verb(), "--desktop", b.data_s(), "--now", "1"], lang);
        assert_eq!(code, 4, "{err}");
        let v = json(&out);
        assert_eq!((text(&v, "reason"), text(&v, "detail")), ("E_DESKTOP".into(), "NOT_OPEN".into()));
        let line = err.lines().next().unwrap_or_default();
        assert!(line.contains(said) && has_chinese(line) == (lang == ZH), "{lang}: {line}");
    }
    assert_eq!(std::fs::read_dir(&b.data).expect("data").count(), 0, "nothing written into the data root");
}

#[test]
fn each_reply_is_answered_by_the_verbs_own_answer() {
    let b = Bench::new("each");
    let replies: Vec<(Reply, i32, &str)> = vec![
        (Reply::Answered { body: r#"{"said":"x","ok":true}"#.into() }, 0, r#"{"ok":true,"said":"x"}"#),
        (Reply::Answered { body: r#"{"token":"E_V_STATE","detail":"d","ok":false}"#.into() }, 1, r#"{"detail":"d","ok":false,"reason":"E_V_STATE"}"#),
        (Reply::OnDesktop, 1, r#"{"ok":false,"reason":"E_ON_DESKTOP"}"#),
        (Reply::Queued { count: 3 }, 3, r#"{"count":3,"ok":false,"reason":"E_QUEUED"}"#),
        (Reply::Refused { code: "READ_ONLY".into(), tail: "t".into(), network: false, zh: "只读".into(), en: "read only".into() }, 1, r#"{"detail":"t","ok":false,"reason":"E_READ_ONLY","token":"READ_ONLY"}"#),
        (Reply::Refused { code: "UNREACHABLE".into(), tail: "t".into(), network: true, zh: "连不上".into(), en: "unreachable".into() }, 4, r#"{"detail":"t","ok":false,"reason":"E_UNREACHABLE","token":"UNREACHABLE"}"#),
        (Reply::Refused { code: "E_APP_DENIED".into(), tail: "t".into(), network: false, zh: "已拒".into(), en: "declined".into() }, 1, r#"{"detail":"t","ok":false,"reason":"E_APP_DENIED"}"#),
        (Reply::Refused { code: "OUTCOME_LOST".into(), tail: "t".into(), network: true, zh: "结果丢了".into(), en: "outcome lost".into() }, 4, r#"{"detail":"t","ok":false,"reason":"E_OUTCOME_LOST","token":"OUTCOME_LOST"}"#),
        (Reply::Partial { code: "E_V_UNCONFIRMED".into(), tail: "t".into(), zh: "成败未知".into(), en: "unknown".into() }, 3, r#"{"detail":"t","ok":false,"reason":"E_V_UNCONFIRMED"}"#),
        (Reply::Closed, 4, r#"{"detail":"CLOSED","ok":false,"reason":"E_DESKTOP"}"#),
    ];
    for (reply, code, want) in replies {
        let bytes = reply.to_bytes();
        let (closer, rx) = desktop(&b.place(), move |_| Some(bytes.clone()));
        let (got, out, err) = b.run(&[verb(), "--desktop", b.data_s(), "--now", "1"], EN);
        closer.close();
        assert_eq!((got, String::from_utf8_lossy(&out).to_string()), (code, want.to_string()), "{reply:?}: {err}");
        let q = rx.recv().expect("the request came");
        assert_eq!(q.verb, verb());
        assert_eq!(Path::new(&q.home), std::fs::canonicalize(&b.data).expect("data"), "the data root as the system resolves it");
        let line = err.lines().next().unwrap_or_default().to_string();
        match &reply {
            Reply::Refused { en, .. } | Reply::Partial { en, .. } => assert_eq!(&line, en),
            Reply::OnDesktop => assert_eq!(line, "This action must be done on the desktop"),
            Reply::Queued { .. } => assert_eq!(line, "Queued; waiting for the user to send it from the desktop"),
            Reply::Closed => assert_eq!(line, "The desktop locked, closed this data folder or quit before it answered"),
            _ => assert!(err.is_empty(), "{reply:?}: {err}"),
        }
    }
}

#[test]
fn the_output_through_the_desktop_is_the_same_in_either_language() {
    let b = Bench::new("tongue");
    let replies = [
        Reply::Refused { code: "READ_ONLY".into(), tail: "read-only evidence".into(), network: false, zh: "只读".into(), en: "read only".into() },
        Reply::Refused { code: "UNREACHABLE".into(), tail: "transport failed".into(), network: true, zh: "连不上".into(), en: "unreachable".into() },
    ];
    for reply in replies {
        let mut seen = Vec::new();
        for lang in [EN, ZH] {
            let bytes = reply.to_bytes();
            let (closer, _rx) = desktop(&b.place(), move |_| Some(bytes.clone()));
            let (code, out, err) = b.run(&[verb(), "--desktop", b.data_s(), "--now", "1"], lang);
            closer.close();
            seen.push((code, out, err.lines().next().unwrap_or_default().to_string()));
        }
        assert_eq!((seen[0].0, &seen[0].1), (seen[1].0, &seen[1].1), "{reply:?}: the same output in either language");
        assert_ne!(seen[0].2, seen[1].2, "{reply:?}: the line for people follows the language");
    }
}

#[test]
fn the_request_carries_each_flag_as_read() {
    let b = Bench::new("args");
    let (name, place) = eravon_cli::verbs::HOME
        .iter()
        .find_map(|(v, row)| row.iter().map(|(f, _)| *f).find(|f| eravon_cli::deskwire::is_place(v, f)).map(|f| (v.to_string(), f.to_string())))
        .expect("a verb with a place");
    let answered = Reply::Answered { body: r#"{"ok":true}"#.into() }.to_bytes();
    let (closer, rx) = desktop(&b.place(), move |_| Some(answered.clone()));
    let flag = format!("--{place}");
    let (code, _, err) = b.run(&[&name, "--desktop", b.data_s(), "--now", "7", &flag, "rel/out"], EN);
    closer.close();
    assert_eq!(code, 0, "{err}");
    let q = rx.recv().expect("came");
    assert_eq!(q.one(&place).map(PathBuf::from), Some(std::fs::canonicalize(&b.root).expect("root").join("rel/out")), "the place made whole");
    assert_eq!(q.one("now"), Some("7"), "the clock as given");
}

#[test]
fn misuse_is_judged_before_the_desktop_is_asked() {
    let b = Bench::new("misuse");
    let (closer, rx) = desktop(&b.place(), |_| Some(Reply::Closed.to_bytes()));
    let data = b.data_s().to_string();
    let not_taken = "not taken together with --desktop (the desktop uses its own open vault)";
    let cases: Vec<(Vec<&str>, &str, &str)> = vec![
        (vec![verb(), "--desktop", "/nowhere/at/all", "--now", "1"], "E_UNREADABLE #3 (15 bytes)", "cannot be read"),
        (vec![verb(), "--desktop", &data], "E_ARGS --now", "this flag is missing"),
        (vec![verb(), "--desktop", &data, "--now", "1", "--passcode", "x"], "E_ARGS --passcode", not_taken),
        (vec![verb(), "--desktop", &data, "--now", "1", "--nosuch", "x"], "E_ARGS #6 (8 bytes)", "not among this verb's flags"),
        (vec![verb(), "--desktop", &data, "--now", "1", "--proxy", "not a proxy"], "E_ARGS --proxy", eravon_literals::misuse::PROXY.en),
        (vec![verb(), "--desktop", &data, "--desktop", &data, "--now", "1"], "E_ARGS --desktop", "given more than once"),
        (vec!["contract", "--desktop", &data], "E_ARGS --desktop", "not among this verb's flags"),
    ];
    for (args, first, second) in cases {
        let (code, out, err) = b.run(&args, EN);
        assert_eq!(code, 2, "{args:?}: {err}");
        assert!(out.is_empty(), "{args:?}: misuse writes nothing to stdout");
        let mut lines = err.lines();
        assert_eq!(lines.next(), Some(first), "{args:?}");
        assert_eq!(lines.next(), Some(second), "{args:?}");
    }
    closer.close();
    assert!(rx.try_recv().is_err(), "the desktop was never asked");
}

#[test]
fn a_door_that_breaks_or_cannot_be_reached_is_named() {
    let b = Bench::new("broken");
    for answer in [None, Some(b"{\"form\":\"eravon-door/1\",\"reply\":\"later\"}".to_vec())] {
        let (closer, _rx) = desktop(&b.place(), move |_| answer.clone());
        let (code, out, err) = b.run(&[verb(), "--desktop", b.data_s(), "--now", "1"], EN);
        closer.close();
        let v = json(&out);
        assert_eq!((code, text(&v, "reason"), text(&v, "detail")), (4, "E_DESKTOP".into(), "BROKEN".into()), "{err}");
        assert_eq!(err.lines().next(), Some("The desktop broke off, or answered in a form this version cannot read"));
    }
    let ptr = eravon_os::machine::pointer_dir(&b.user).join(eravon_os::machine::POINTER);
    let deep = b.root.join("d".repeat(120));
    std::fs::create_dir_all(&deep).expect("deep");
    std::fs::write(&ptr, format!("{}\n", deep.display())).expect("pointer");
    let (code, out, _) = b.run(&[verb(), "--desktop", b.data_s(), "--now", "1"], EN);
    assert_eq!((code, text(&json(&out), "detail")), (4, "PATH_TOO_LONG".into()));
    std::fs::write(&ptr, b"../relative\n").expect("pointer");
    let (code, out, err) = b.run(&[verb(), "--desktop", b.data_s(), "--now", "1"], ZH);
    assert_eq!((code, text(&json(&out), "detail")), (4, "NO_MACHINE".into()));
    assert_eq!(err.lines().next(), Some("找不到本机数据所在的文件夹"));
}
