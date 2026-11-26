#![cfg(unix)]

use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eravon");
const EN: &str = "en_US.UTF-8";
const ZH: &str = "zh_CN.UTF-8";

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

fn answer(out: &[u8]) -> (String, String) {
    let v = eravon::json::parse(out).unwrap_or_else(|_| panic!("not one value: {}", String::from_utf8_lossy(out)));
    let text = |k: &str| v.member(k).and_then(|x| x.as_str()).unwrap_or_default().to_string();
    (text("reason"), text("detail"))
}

fn has_chinese(s: &str) -> bool {
    s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
}

#[test]
fn a_desktop_write_while_writing_alone_is_refused_by_name() {
    let b = Bench::new("alone");
    let verb = eravon_cli::verbs::HOME[0].0;
    let data = b.data.to_str().expect("utf-8");
    let credential = b.machine.join(eravon_seal::places::credential_file_for(eravon_vault::places::ACCOUNT));
    std::fs::write(&credential, b"{}").expect("a credential in the machine folder");
    for lang in [EN, ZH] {
        let (code, out, err) = b.run(&[verb, "--desktop", data, "--now", "1"], lang);
        assert_eq!((code, answer(&out)), (4, ("E_DESKTOP".into(), "ALONE".into())), "{err}");
        let line = err.lines().next().unwrap_or_default();
        assert!(line.contains("--desktop") && has_chinese(line) == (lang == ZH), "{lang}: {line}");
    }
    assert_eq!(std::fs::read_dir(&b.data).expect("data").count(), 0, "nothing written into the data root");
    std::fs::remove_file(&credential).expect("the credential away");
    let (code, out, err) = b.run(&[verb, "--desktop", data, "--now", "1"], EN);
    assert_eq!((code, answer(&out)), (4, ("E_DESKTOP".into(), "NOT_OPEN".into())), "with no credential the desktop is closed: {err}");
}

#[test]
fn a_data_root_another_machine_writes_is_answered_by_the_stores_name() {
    let b = Bench::new("other");
    let verb = eravon_cli::verbs::HOME[0].0;
    let data = b.data.to_str().expect("utf-8");
    let mark = b.data.join(eravon_store::lock::MARK_FILE);
    for laid in ["0123456789abcdef0123456789abcdef\n", "not a mark\n"] {
        std::fs::write(&mark, laid).expect("a mark");
        let (code, out, err) = b.run(&[verb, "--desktop", data, "--now", "1"], EN);
        let v = eravon::json::parse(&out).unwrap_or_else(|_| panic!("not one value: {}", String::from_utf8_lossy(&out)));
        let text = |k: &str| v.member(k).and_then(|x| x.as_str()).unwrap_or_default().to_string();
        assert_eq!((code, text("reason"), text("downToken")), (1, "E_V_DOWN".into(), "E_STORE_OTHER_MACHINE".into()), "{laid}: {err}");
        assert_eq!(std::fs::read_to_string(&mark).expect("the mark"), laid, "the mark left as it was");
        assert_eq!(std::fs::read_dir(&b.data).expect("data").count(), 1, "nothing else written into the data root");
    }
}
