#![cfg(unix)]

use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eravon");
const EN: &str = "en_US.UTF-8";
const ZH: &str = "zh_CN.UTF-8";
const PIN: &str = "48261537";
const WORDS: &str = "test test test test test test test test test test test junk";

struct Bench {
    root: PathBuf,
    user: PathBuf,
    data: PathBuf,
}

impl Bench {
    fn new(tag: &str) -> Bench {
        let base = if std::env::temp_dir().as_os_str().len() <= 40 { std::env::temp_dir() } else { PathBuf::from("/tmp") };
        let root = base.join(format!("evq-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (user, machine, data) = (root.join("u"), root.join("m"), root.join("h"));
        for d in [&user, &machine, &data] {
            std::fs::create_dir_all(d).expect("dir");
        }
        let machine = std::fs::canonicalize(&machine).expect("machine");
        let ptr = eravon_os::machine::pointer_dir(&user);
        std::fs::create_dir_all(&ptr).expect("pointer folder");
        std::fs::write(ptr.join(eravon_os::machine::POINTER), format!("{}\n", machine.display())).expect("pointer");
        Bench { root, user, data }
    }

    fn run(&self, args: &[&str], lang: &str) -> (i32, String) {
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
        (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into_owned())
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn member(out: &str, k: &str) -> String {
    let v = eravon::json::parse(out.as_bytes()).unwrap_or_else(|_| panic!("not one value: {out}"));
    v.member(k).and_then(|x| x.as_str()).unwrap_or_default().to_string()
}

#[test]
fn queue_only_refuses_the_command_lines_own_signature_and_send() {
    let b = Bench::new("q");
    let data = b.data.to_str().expect("utf-8").to_string();
    let (code, out) = b.run(&["buyer-vault-init", "--data", &data, "--writer", "w", "--now", "1", "--passcode", PIN, "--phrase", WORDS], EN);
    assert_eq!(code, 0, "{out}");
    let send = |lang: &str| {
        b.run(
            &[
                "buyer-wallet-send", "--data", &data, "--writer", "w", "--now", "2", "--passcode", PIN, "--endpoint", "http://127.0.0.1:1", "--chain", "1",
                "--to", "0x00000000000000000000000000000000000000c1", "--token", "0x00000000000000000000000000000000000000d1", "--amount", "1", "--send", "true",
            ],
            lang,
        )
    };
    let (_, out) = send(EN);
    assert_ne!(member(&out, "downToken"), eravon_cli::home::E_QUEUE_ONLY, "send automatically: {out}");
    // SAFETY: this test binary holds this one test; nothing else reads the environment meanwhile.
    unsafe { std::env::set_var(eravon_os::HOME_VAR, &b.user) };
    let mut s = eravon_vault::session::Session::locked(&data);
    s.unlock(PIN).expect("unlocks");
    eravon_vault::machine::update(|m| m.cli_anchor = eravon_seal::machine::CliAnchor::Queue).expect("the cell");
    for (lang, word) in [(EN, "--desktop"), (ZH, "--desktop")] {
        let (code, out) = send(lang);
        assert_eq!((code, member(&out, "reason"), member(&out, "downToken")), (1, "E_V_DOWN".to_string(), eravon_cli::home::E_QUEUE_ONLY.to_string()), "{out}");
        assert!(member(&out, "detail").contains(word), "points at the desktop: {out}");
    }
    let said = |out: &str| member(out, "detail").split_once(&format!("{}:", eravon_cli::home::E_QUEUE_ONLY)).map(|(_, s)| s.to_string()).unwrap_or_default();
    let han = |s: &str| s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
    let (zh, en) = (said(&send(ZH).1), said(&send(EN).1));
    assert!(han(&zh) && !en.is_empty() && !han(&en), "zh {zh} · en {en}");
    let (code, out) = b.run(&["buyer-vault-list", "--data", &data, "--writer", "w", "--now", "3", "--passcode", PIN], EN);
    assert_eq!(code, 0, "{out}");
    use eravon_seal::machine::{CliAnchor, Lasts};
    for (alone, desktop, cell, refused) in [
        (Some(Lasts::Secs(3_600)), true, CliAnchor::Queue, false),
        (Some(Lasts::Secs(3_600)), true, CliAnchor::Send, false),
        (None, false, CliAnchor::Queue, false),
        (None, false, CliAnchor::Send, false),
        (None, true, CliAnchor::Send, false),
        (None, true, CliAnchor::Queue, true),
    ] {
        let m = eravon_vault::machine::update(|m| {
            m.cli_alone = alone;
            m.cli_desktop = desktop;
            m.cli_anchor = cell;
        })
        .expect("the switches");
        assert_eq!(eravon_cli::home::anchor_live(&m), alone.is_none() && desktop, "in force only through the desktop");
        assert_eq!(eravon_cli::home::anchor_now(&m), if refused { CliAnchor::Queue } else { CliAnchor::Send });
        let (code, out) = send(EN);
        assert_eq!(member(&out, "downToken") == eravon_cli::home::E_QUEUE_ONLY, refused, "alone {alone:?} desktop {desktop} cell {cell:?}: {code} {out}");
        assert_eq!(eravon_vault::machine::read().expect("reads").cli_anchor, cell, "the stored choice is kept whatever the mode, after the run");
    }
    for verb in ["seller-offer-sign", "seller-product-resign", "buyer-order-sign", "buyer-settle-sign"] {
        let (code, out) = b.run(&[verb, "--desktop", &data, "--now", "4", "--wait", "5"], EN);
        assert_ne!(code, 2, "{verb} --wait through the desktop is no misuse: {out}");
        let (code, out) = b.run(&[verb, "--data", &data, "--now", "4", "--passcode", PIN, "--wait", "5"], EN);
        assert_ne!(code, 2, "{verb} --wait run directly is no misuse: {out}");
    }
}
