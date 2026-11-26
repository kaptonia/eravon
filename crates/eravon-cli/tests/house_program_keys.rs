#![cfg(unix)]

mod common;

use common::{Home, PIN};
use eravon_seal::credential::{Term, PASS_ENV, PASS_PREFIX};

const NOW: &str = "1758412800";

fn write(h: &Home, pass: Option<&str>) -> common::Ran {
    h.run(&["buyer-settings-zone", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--zone", "system"], |c| {
        c.env_remove(PASS_ENV).env("LANG", "en_US.UTF-8");
        if let Some(p) = pass {
            c.env(PASS_ENV, p);
        }
    })
}

fn named(r: &common::Ran) -> (i32, String) {
    (r.code, r.get("reason"))
}

#[test]
fn each_program_brings_its_own_key_and_a_deleted_row_ends_that_key_alone() {
    let mut h = Home::bare("pk");
    let ptr = eravon_os::machine::pointer_dir(&h.home);
    std::fs::create_dir_all(&ptr).expect("pointer folder");
    std::fs::write(ptr.join(eravon_os::machine::POINTER), format!("{}\n", h.data.display())).expect("pointer");
    let made = h.run(&["buyer-vault-init", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--phrase", common::PHRASE], |_| {});
    assert_eq!(made.code, 0, "the vault is made: {} {}", made.text, made.err);
    h.address = made.get("address");
    // SAFETY: this test is the only one in its process; nothing else reads the environment meanwhile.
    unsafe { std::env::set_var(eravon_os::HOME_VAR, &h.home) };
    let mut s = eravon_vault::session::Session::locked(h.data_s());
    s.unlock(PIN).expect("unlocks");
    eravon_vault::keybox::issue_credential(Term { issued: 1, expires: None }).expect("the switch on: a credential with no key");
    let credential = eravon_seal::credential::path(&h.machine(), eravon_vault::places::ACCOUNT);
    assert!(credential.is_file(), "the credential in the machine folder");
    let (a, row_a) = eravon_vault::keybox::add_pass("tool a", 1).expect("a key for one program");
    let (b, _) = eravon_vault::keybox::add_pass("tool b", 1).expect("a key for another");
    let (a, b) = (a.expose().to_string(), b.expose().to_string());
    drop(s);
    eravon_store::writer::let_go(h.data_s());

    let r = write(&h, Some(&a));
    assert_eq!(r.code, 0, "the key brought writes: {} {}", r.text, r.err);
    for pass in [None, Some("")] {
        let r = write(&h, pass);
        assert_eq!(named(&r), (1, "E_CREDENTIAL_PASS_MISSING".to_string()), "{pass:?}: {}", r.text);
    }
    let never = format!("{PASS_PREFIX}{}", "ab".repeat(32));
    for pass in [never.as_str(), "not a key"] {
        let r = write(&h, Some(pass));
        assert_eq!(named(&r), (1, "E_CREDENTIAL_PASS_UNKNOWN".to_string()), "{pass}: {}", r.text);
        assert!(!r.text.contains(pass) && !r.err.contains(pass), "the key is never in the answer: {}", r.text);
    }
    let mut s = eravon_vault::session::Session::locked(h.data_s());
    s.unlock(PIN).expect("unlocks");
    eravon_vault::keybox::drop_pass(&row_a.tag).expect("deleted");
    drop(s);
    eravon_store::writer::let_go(h.data_s());
    let r = write(&h, Some(&a));
    assert_eq!(named(&r), (1, "E_CREDENTIAL_PASS_UNKNOWN".to_string()), "the deleted key: {}", r.text);
    assert!(!r.text.contains(&a[PASS_PREFIX.len()..]), "never the key in the answer: {}", r.text);
    let r = write(&h, Some(&b));
    assert_eq!(r.code, 0, "the other key still writes: {} {}", r.text, r.err);
    eravon_store::writer::need_writer(h.data_s(), "desktop", 1).expect("held here, as the desktop holds it");
    let r = write(&h, Some(&b));
    assert_eq!((r.code, r.get("reason"), r.get("downToken")), (1, "E_V_DOWN".to_string(), eravon_cli::home::E_CREDENTIAL_DESKTOP_HOLDS.to_string()), "{}", r.text);
    eravon_store::writer::let_go(h.data_s());

    let earlier = format!("{{\"expires\":null,\"issued\":1,\"key\":\"0x{}\",\"master\":\"0x{}\"}}", "11".repeat(32), "22".repeat(72));
    std::fs::write(&credential, earlier).expect("laid");
    let r = write(&h, Some(&b));
    assert_eq!(named(&r), (1, "E_CREDENTIAL_SHAPE".to_string()), "an earlier version's record: {}", r.text);
    assert!(!credential.exists(), "removed");
    let r = write(&h, Some(&b));
    assert_eq!((named(&r), r.get("downToken")), ((1, "E_V_DOWN".to_string()), "E_VAULT_LOCKED".to_string()), "nothing left to open: {}", r.text);
}
