use super::vault_open;
use app::action::{apply, Action, Applied};

fn settled(shell: &mut app::shell::Shell, k: app::task::Kind) {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while shell.tasks.in_flight(k) && std::time::Instant::now() < until {
        shell.drain();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    shell.drain();
}

fn import(shell: &mut app::shell::Shell, form: app::action::ImportForm, label: &str) -> String {
    match app::action::apply_settled(shell, Action::ImportIdentity { form, label: label.into() }) {
        Applied::Trouble(f) => f.which().map(|k| k.as_str().to_string()).unwrap_or_default(),
        _ => String::new(),
    }
}

fn rows() -> Vec<(String, String)> {
    app::register::read().ok().flatten().map(|r| r.rows.iter().map(|x| (x.id.clone(), x.label.clone())).collect()).unwrap_or_default()
}

const WORDS_A: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const WORDS_B: &str = "legal winner thank year wave sausage worth useful legal winner thank yellow";

#[test]
fn importing_an_identity_again_is_answered_by_name() {
    if super::alone_in(module_path!(), "importing_an_identity_again_is_answered_by_name") {
        return;
    }
    vault_open();
    let ctx = eravon_ui::egui::Context::default();
    let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    let words = |w: &str| app::action::ImportForm::Words(w.to_string().into());
    assert_eq!(import(&mut shell, words(WORDS_A), "alpha"), "", "a new identity lands");
    let one = rows();
    assert_eq!(one.len(), 1);
    assert_eq!(import(&mut shell, words(WORDS_A), "alpha"), app::fault::Known::IdentityExists.as_str(), "same identity, same name");
    assert_eq!(import(&mut shell, words(WORDS_A), "beta"), app::fault::Known::IdentityHereAs.as_str(), "same identity, another name");
    assert_eq!(rows(), one, "not renamed");
    assert_eq!(import(&mut shell, words(WORDS_B), "alpha"), app::fault::Known::IdentityNameTaken.as_str(), "another identity, a name taken");
    assert_eq!(rows(), one, "no row added");
    assert_eq!(import(&mut shell, words(WORDS_B), ""), "", "another identity, no name");
    assert_eq!(rows().len(), 2);
    let s = app::key::generate().expect("a key");
    let mut slot = Some(s);
    let hex = app::key::reveal_once(&mut slot).expect("its hex");
    let key = |file: Option<std::path::PathBuf>| app::action::ImportForm::PrivateKey {
        key: hex.clone().into(),
        keyfile: file.map(|d| app::action::KeyFileOut { password: "a long key file password".to_string().into(), again: "a long key file password".to_string().into(), dir: d.display().to_string() }),
    };
    assert_eq!(import(&mut shell, key(None), "gamma"), app::fault::Known::PartyKey.as_str(), "a bare key is refused");
    let dir = std::env::temp_dir().join(format!("zk-is-keyfile-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    assert_eq!(import(&mut shell, key(Some(dir.clone())), "gamma"), app::fault::Known::PartyKey.as_str(), "a bare key with a key file named is refused too");
    assert_eq!(rows().len(), 2, "no row added");
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0, "no key file written for an import refused");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_gates_own_mouths_name_the_passcode_they_hand_and_forget_it() {
    let typed = || app::secret::Secret::from(String::from("q7w8e9r0"));
    let mouths = [
        Action::SetPin { pin: typed(), again: typed() },
        Action::Unlock { pin: typed() },
        Action::ChangePin { old: typed(), pin: typed(), again: typed() },
        Action::Reseal { pin: typed() },
        Action::RecoverWords { words: typed(), pin: typed(), again: typed() },
        Action::RecoverKeystore { path: String::new(), password: typed(), pin: typed(), again: typed() },
        Action::SetPrimary { id: String::new(), pin: typed() },
        Action::RestoreBackup { path: String::new(), password: typed(), how: app::action::RestoreHow::Settings { pin: typed() } },
        Action::RestoreBackup { path: String::new(), password: typed(), how: app::action::RestoreHow::Locked { pin: typed(), again: typed() } },
    ];
    for mut a in mouths {
        assert!(a.pin_asked().is_none(), "{}: the gate itself does not pass the gate", a.name());
        assert_eq!(a.gate_pin().map(|p| p.expose().to_string()), Some("q7w8e9r0".to_string()), "{}", a.name());
        a.forget_pin();
        assert_eq!(a.gate_pin().map(|p| p.expose().len()), Some(0), "{}: wiped", a.name());
    }
    let asked = Action::BackupKey { pin: typed(), password: app::secret::Secret::default(), again: app::secret::Secret::default(), dir: String::new() };
    assert!(asked.gate_pin().is_none() && asked.pin_asked().is_some(), "an action that passes the gate is not a gate mouth");
}

#[test]
fn the_words_shown_are_secrets_and_are_let_go() {
    if super::alone_in(module_path!(), "the_words_shown_are_secrets_and_are_let_go") {
        return;
    }
    vault_open();
    let ctx = eravon_ui::egui::Context::default();
    let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    answers!(apply(&mut shell, Action::NewIdentity), Applied::FreshWords);
    let (list, picks) = shell.new_words.as_ref().map(|f| (f.words(), f.picks)).expect("fresh words");
    let typed: &Vec<app::secret::Secret> = &list;
    assert_eq!(typed.len(), 12);
    let answers: Vec<(usize, app::secret::Secret)> = picks.iter().map(|i| (*i, list[*i].clone())).collect();
    apply(&mut shell, Action::ConfirmIdentity { answers, label: "a2".into() });
    settled(&mut shell, app::task::Kind::Vault);
    let pin = || app::secret::Secret::from(String::from("27618394"));
    let reveal = |shell: &mut app::shell::Shell| {
        apply(shell, Action::RevealWords { pin: pin() });
        settled(shell, app::task::Kind::Vault);
        shell.words.as_ref().map(|w| w.len())
    };
    assert_eq!(reveal(&mut shell), Some(12), "shown");
    apply(&mut shell, Action::HideWords);
    assert!(shell.words.is_none(), "hidden");
    assert_eq!(reveal(&mut shell), Some(12));
    apply(&mut shell, Action::Lock);
    assert!(shell.words.is_none(), "locked");
    apply(&mut shell, Action::Unlock { pin: pin() });
    settled(&mut shell, app::task::Kind::Vault);
    assert_eq!(reveal(&mut shell), Some(12));
    answers!(apply(&mut shell, Action::NewIdentity), Applied::FreshWords);
    apply(&mut shell, Action::Quit);
    assert!(shell.words.is_none() && shell.new_words.is_none(), "quit");
}

#[test]
fn the_bar_says_a_mark_that_does_not_read_as_such() {
    if super::alone_in(module_path!(), "the_bar_says_a_mark_that_does_not_read_as_such") {
        return;
    }
    vault_open();
    let base = std::env::temp_dir().join(format!("eravon-is-bar-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let ctx = eravon_ui::egui::Context::default();
    let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    answers!(apply(&mut shell, Action::OpenHome { root: base.display().to_string() }), Applied::Homed { .. });
    let at = app::lock::mark_path(shell.home.as_ref().expect("a home"));
    for (form, bytes, bar) in [("anotherMachine", "0123456789abcdef0123456789abcdef\n", "other_machine"), ("wrongShape", "not a mark\n", "mark_unread")] {
        std::fs::write(&at, bytes).unwrap();
        shell.lock = None;
        let opened = apply(&mut shell, Action::OpenHome { root: base.display().to_string() });
        assert!(matches!(opened, Applied::Homed { .. }), "{form}: {opened:?}");
        assert_eq!(shell.banner().as_str(), bar, "{form}");
    }
    let (zh, en) = app::lang::TABLE.iter().find(|(k, _, _)| *k == app::lang::Key::SetMarkUnread).map(|(_, z, e)| (*z, *e)).expect("the sentence");
    assert!(!zh.contains("另一台机器在写") && !en.contains("Another machine is writing"), "never says another machine writes");
    drop(shell);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn the_backup_lamp_has_four_colours() {
    use app::firstrun::{backup_point, Shade};
    let b = |count: u64| app::machine::Backed { at: 1, path: String::new(), count };
    let (two, four) = (b(2), b(4));
    for (form, backup, shade) in [
        ("fresh", (Some(&four), Some(4), None), Shade::Green),
        ("behind", (Some(&two), Some(4), None), Shade::Amber),
        ("never", (None, Some(4), None), Shade::Grey),
        ("failed", (Some(&four), Some(4), Some(9)), Shade::Red),
        ("failedNever", (None, Some(4), Some(9)), Shade::Red),
        ("unmeasured", (Some(&four), None, None), Shade::Grey),
    ] {
        assert_eq!(backup_point(backup.0, backup.1, backup.2), shade, "{form}");
    }
}

#[test]
fn the_english_first_layer_has_no_cjk() {
    if super::alone_in(module_path!(), "the_english_first_layer_has_no_cjk") {
        return;
    }
    let window = super::shipped().into_iter().find(|(n, _)| n == "window.rs").map(|(_, t)| super::code_only(&t)).expect("the window");
    for door in [".said()", ".evidence()"] {
        assert!(!window.contains(door), "the window calls {door}: the evidence is not the first layer");
    }
    struct Back(app::lang::Lang);
    impl Drop for Back {
        fn drop(&mut self) {
            app::lang::set(self.0);
        }
    }
    let _back = Back(app::lang::lang());
    app::lang::set(app::lang::Lang::En);
    let cjk = |s: &str| s.chars().any(|c| ('\u{2e80}'..='\u{9fff}').contains(&c) || ('\u{f900}'..='\u{faff}').contains(&c) || ('\u{ff00}'..='\u{ffef}').contains(&c));
    for k in app::fault::Known::ALL {
        let f = app::fault::Fault::known(k, "节点答:读不成");
        assert!(!cjk(f.human()), "{k:?}: first sentence {:?}", f.human());
        assert!(!k.next().is_some_and(|n| cjk(app::lang::t(n))), "{k:?}: next sentence");
    }
}

#[test]
fn a_restored_backup_brings_no_failed_mark_back() {
    if super::alone_in(module_path!(), "a_restored_backup_brings_no_failed_mark_back") {
        return;
    }
    vault_open();
    let out = std::env::temp_dir().join(format!("zk-is-restore-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    app::machine::update(|m| m.backup_failed = Some(5)).expect("the mark laid");
    let made = app::backup::export(&out, "a long backup password", 1_800_000_000).expect("the export lands");
    assert_eq!(app::machine::read().expect("settings").backup_failed, None, "cleared by the export that landed");
    app::machine::update(|m| m.backup_failed = Some(9)).expect("the mark laid again");
    app::backup::restore(&made.path, "a long backup password", app::backup::From::Settings("27618394")).expect("restored");
    let after = app::machine::read().expect("settings");
    assert_eq!(after.backup_failed, None, "the restored record is the backup that landed, no red");
    assert!(after.backup.is_some());
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn every_share_of_the_backup_package_is_transcribed_in_one_place() {
    let text = super::read_src_file("backup.rs").expect("backup.rs");
    let code = super::code_only(&text);
    let start = code.find("fn package_bytes(").expect("the package's assembly");
    let body = &code[start..start + code[start..].find("\n}\n").expect("its end")];
    assert_eq!(body.matches("transcribe(").count(), 4, "keys, files, machine, register: {body}");
    for other in ["hex(", "encode(", "b64", "base64", "to_string(", "format!("] {
        let outside = body.replace("transcribe(", "");
        assert!(!outside.contains(other), "a share spelled another way ({other}) in the package's assembly");
    }
    assert_eq!(app::backup::transcribe(&[0x00, 0xab, 0xff]), "0x00abff");
    assert_eq!(app::backup::transcribe(b""), "0x");
}

#[test]
fn a_node_line_typed_wrong_is_refused_without_its_key() {
    if super::alone_in(module_path!(), "a_node_line_typed_wrong_is_refused_without_its_key") {
        return;
    }
    const KEY: &str = "k3yInTheLine77";
    vault_open();
    let lines = [format!("31337=http://127.0.0.1:1/{KEY} x"), format!("31337=http://door:{KEY}@127.0.0.1:1/rpc"), format!("31337=http://127.0.0.1:port/{KEY}")];
    let ctx = eravon_ui::egui::Context::default();
    let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    let seen = |f: &app::fault::Fault| format!("{} {} {} {} {}", f.said(), f.tail(), f.evidence(), f.human(), f.raw());
    let base = std::env::temp_dir().join(format!("eravon-is-nodes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    answers!(apply(&mut shell, Action::OpenHome { root: base.display().to_string() }), Applied::Homed { .. });
    for line in &lines {
        match apply(&mut shell, Action::SetEndpoints { specs: line.clone() }) {
            Applied::Trouble(f) => {
                assert_eq!(f.which(), Some(app::fault::Known::SettingsShape), "{line}");
                assert!(!seen(&f).contains(KEY), "settings say the key: {}", seen(&f));
            }
            other => panic!("{line}: refused: {other:?}"),
        }
    }
    drop(shell);
    let _ = std::fs::remove_dir_all(&base);
}
