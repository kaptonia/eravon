use super::vault_open;
use app::action::{apply, Action, Applied};
use app::secret::Secret;

const ETHERSCAN: &str = "https://api.etherscan.io/v2/api";
const KEY: &str = "ETHERSCAN-TEST-KEY-7f3a";

fn shell_with_home(name: &str) -> (app::shell::Shell, std::path::PathBuf) {
    vault_open();
    let dir = std::env::temp_dir().join(format!("eravon-rest-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let ctx = eravon_ui::egui::Context::default();
    let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    let home = app::home::Home::open_or_create(&dir).expect("a home");
    shell.lock = Some(app::lock::take(&home).expect("the lock"));
    shell.home = Some(home);
    (shell, dir)
}

fn settings_plain(dir: &std::path::Path) -> String {
    let file = dir.join("settings").join(app::settings::FILE);
    String::from_utf8(app::local::read(&file, app::local::Doc::Settings).expect("opens").expect("written")).expect("utf-8")
}

fn refused(a: Applied) -> String {
    match a {
        Applied::Trouble(f) => f.said().split([' ', ':']).next().unwrap_or_default().to_string(),
        other => panic!("refused by name: {other:?}"),
    }
}

#[test]
fn a_rest_row_keeps_its_key_on_this_machine_and_only_its_base_url_in_the_ledger() {
    if super::alone_in(module_path!(), "a_rest_row_keeps_its_key_on_this_machine_and_only_its_base_url_in_the_ledger") {
        return;
    }
    let (mut shell, dir) = shell_with_home("add");
    for (base, key) in [("http://api.etherscan.io/v2/api", KEY), ("https://api.etherscan.io/v2/api?apikey=K", KEY), (ETHERSCAN, ""), (ETHERSCAN, "a b")] {
        assert_eq!(refused(apply(&mut shell, Action::AddRest { base: base.into(), key: Secret::of(key) })), "SETTINGS_SHAPE", "{base} {key:?}");
    }
    assert!(shell.settings.rest.is_empty() && app::machine::read().expect("machine").rest_keys.0.is_empty(), "nothing written");
    match apply(&mut shell, Action::AddRest { base: ETHERSCAN.into(), key: Secret::of(KEY) }) {
        Applied::RestAdded { base } => assert_eq!(base, ETHERSCAN),
        other => panic!("{other:?}"),
    }
    assert_eq!(refused(apply(&mut shell, Action::AddRest { base: "https://API.Etherscan.io/api".into(), key: Secret::of("OTHER") })), "NETWORK_LISTED", "one service, one row");
    let plain = settings_plain(&dir);
    assert!(plain.contains(&format!("\"{}\":[\"{ETHERSCAN}\"]", eravon_seal::ground::REST)), "{plain}");
    assert!(!plain.contains(KEY) && !plain.contains("7f3a"), "no byte of the key in the ledger's settings: {plain}");
    let machine = app::machine::read().expect("machine");
    assert_eq!(machine.rest_keys.of("api.etherscan.io"), Some(KEY), "the key is this machine's, by service");
    assert_eq!(shell.rest_key_tail(ETHERSCAN).as_deref(), Some("7f3a"));
    let plain_machine = std::fs::read_to_string(app::machine::path().expect("path")).unwrap_or_default();
    assert!(!plain_machine.contains(KEY), "never in the plain machine file");
    match apply(&mut shell, Action::RemoveRest { base: ETHERSCAN.into() }) {
        Applied::RestRemoved { left } => assert_eq!(left, 0),
        other => panic!("{other:?}"),
    }
    assert!(app::machine::read().expect("machine").rest_keys.of("api.etherscan.io").is_none(), "removing the row removes this machine's key");
    assert_eq!(shell.rest_key_tail(ETHERSCAN), None);
    assert_eq!(refused(apply(&mut shell, Action::RemoveRest { base: ETHERSCAN.into() })), "SETTINGS_SHAPE", "a row not listed");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_change_of_rest_rows_moves_the_nodes_and_keeps_the_network() {
    if super::alone_in(module_path!(), "a_change_of_rest_rows_moves_the_nodes_and_keeps_the_network") {
        return;
    }
    let (mut shell, dir) = shell_with_home("switch");
    shell
        .commit_settings(|s| {
            s.chain_id = Some(11155111);
            s.endpoints = vec!["11155111=https://rpc.sepolia.example".into()];
            s.network = Some("sepolia".into());
        })
        .expect("a basis");
    let nodes = |s: &app::shell::Shell| app::chainx::nodes_of(&s.settings);
    assert_eq!(nodes(&shell).len(), 1);
    assert!(matches!(apply(&mut shell, Action::AddRest { base: ETHERSCAN.into(), key: Secret::of(KEY) }), Applied::RestAdded { .. }));
    assert_eq!(nodes(&shell).len(), 1, "off: the row stays and is not asked");
    assert!(matches!(apply(&mut shell, Action::SetRestMode { on: true }), Applied::RestMode(true)));
    let urls: Vec<String> = nodes(&shell).iter().map(|e| e.url.for_transport().to_string()).collect();
    assert_eq!(urls, vec!["https://rpc.sepolia.example".to_string(), eravon_chain::rest::lane(ETHERSCAN, 11155111)]);
    assert_eq!(nodes(&shell)[1].url.kind(), app::chainx::Kind::Rest);
    assert_eq!(nodes(&shell)[0].url.kind(), app::chainx::Kind::Rpc);
    assert_eq!(shell.settings.network.as_deref(), Some("sepolia"), "the network stays the preset it was");
    assert!(settings_plain(&dir).contains(&format!("\"{}\":true", eravon_seal::ground::REST_ON)));
    assert_eq!(shell.settings.endpoints, vec!["11155111=https://rpc.sepolia.example".to_string()], "the JSON-RPC nodes are untouched");
    assert!(matches!(apply(&mut shell, Action::SetRestMode { on: false }), Applied::RestMode(false)));
    assert_eq!(nodes(&shell).len(), 1);
    assert_eq!(shell.settings.rest, vec![ETHERSCAN.to_string()], "off keeps the row");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_chain_is_read_again_only_when_the_nodes_asked_moved() {
    if super::alone_in(module_path!(), "the_chain_is_read_again_only_when_the_nodes_asked_moved") {
        return;
    }
    let (mut shell, dir) = shell_with_home("moved");
    shell
        .commit_settings(|s| {
            s.chain_id = Some(11155111);
            s.endpoints = vec!["11155111=https://rpc.sepolia.example".into()];
        })
        .expect("a basis");
    shell.desk.sh.refresh_at = None;
    assert!(matches!(apply(&mut shell, Action::SetRestMode { on: true }), Applied::RestMode(true)));
    assert_eq!(shell.desk.sh.refresh_at, None, "on with no row: nothing asked moved, the reading stands");
    assert!(matches!(apply(&mut shell, Action::SetRestMode { on: false }), Applied::RestMode(false)));
    assert!(matches!(apply(&mut shell, Action::AddRest { base: ETHERSCAN.into(), key: Secret::of(KEY) }), Applied::RestAdded { .. }));
    assert_eq!(shell.desk.sh.refresh_at, None, "off: a row added is not asked, the reading stands");
    assert!(matches!(apply(&mut shell, Action::RemoveRest { base: ETHERSCAN.into() }), Applied::RestRemoved { .. }));
    assert_eq!(shell.desk.sh.refresh_at, None, "off: a row removed was not asked, the reading stands");
    assert!(matches!(apply(&mut shell, Action::SetRestMode { on: true }), Applied::RestMode(true)));
    assert!(matches!(apply(&mut shell, Action::AddRest { base: ETHERSCAN.into(), key: Secret::of(KEY) }), Applied::RestAdded { .. }));
    assert!(shell.desk.sh.refresh_at.is_some(), "on: a row added is asked, the chain is read again");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_rest_members_are_their_own_and_a_newer_versions_members_ride_along() {
    let (mut shell, dir) = shell_with_home("members");
    let file = dir.join("settings").join(app::settings::FILE);
    let written = format!(
        r#"{{"auditEvery":300,"capBytes":4294967296,"chainId":1,"endpoints":["1=https://rpc.example"],"fromBlock":0,"futureCell":{{"x":[1,"二"]}},"{}":["{ETHERSCAN}","https://eth.blockscout.com/api"],"{}":true,"reviewEvery":600,"role":"author"}}"#,
        eravon_seal::ground::REST,
        eravon_seal::ground::REST_ON
    );
    app::local::put(&dir.join("settings"), app::settings::FILE, app::local::Doc::Settings, written.as_bytes()).expect("laid");
    let h = app::home::Home::bare(dir.clone());
    let s = app::settings::Settings::read(&h).expect("reads");
    assert_eq!(s.rest, vec![ETHERSCAN.to_string(), "https://eth.blockscout.com/api".to_string()]);
    assert!(s.rest_on);
    assert_eq!(s.endpoints, vec!["1=https://rpc.example".to_string()], "the node list holds JSON-RPC nodes alone");
    let g = s.ground();
    assert_eq!(g.endpoints, s.endpoints, "the command line's reading of the node list: no REST row in it");
    assert_eq!(eravon_send::node::nodes_of(&g, 1).len(), 3, "a JSON-RPC node and two REST rows on chain 1");
    shell.settings = s;
    shell.commit_settings(|s| s.cap_bytes = 8 * 1024 * 1024 * 1024).expect("a save");
    let after = String::from_utf8(app::local::read(&file, app::local::Doc::Settings).expect("opens").expect("written")).expect("utf-8");
    assert!(after.contains("\"futureCell\":{\"x\":[1,\"二\"]}"), "a newer version's member is carried back: {after}");
    assert!(after.contains(eravon_seal::ground::REST) && after.contains(eravon_seal::ground::REST_ON), "{after}");
    app::local::put(&dir.join("settings"), app::settings::FILE, app::local::Doc::Settings, br#"{"chainId":1,"endpoints":["1=https://rpc.example"],"role":"author"}"#).expect("laid");
    let old = app::settings::Settings::read(&h).expect("reads");
    assert!(old.rest.is_empty() && !old.rest_on);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_keys_file_the_window_starts_with_keys_a_row_left_without_one() {
    const FORM: &str = "ERAVON_CHECKS_KEYS_FILE_FORM";
    const FILE_KEY: &str = "FILE-KEY-1c2d";
    let base = "https://rest.example.test/api";
    let full = format!("{}::a_keys_file_the_window_starts_with_keys_a_row_left_without_one", module_path!().split_once("::").map(|(_, m)| m).unwrap_or(module_path!()));
    match std::env::var(FORM).as_deref() {
        Ok("given") => {
            let (mut shell, dir) = shell_with_home("keys-file");
            assert_eq!(app::chainx::keys_file(), None, "the file reads");
            assert_eq!(app::chainx::keys_file_said(), None);
            assert_eq!(refused(apply(&mut shell, Action::AddRest { base: "https://other.example.test/api".into(), key: Secret::of("") })), "SETTINGS_SHAPE", "a service the file gives nothing for");
            match apply(&mut shell, Action::AddRest { base: base.into(), key: Secret::of("") }) {
                Applied::RestAdded { base: b } => assert_eq!(b, base),
                other => panic!("{other:?}"),
            }
            assert!(app::machine::read().expect("machine").rest_keys.0.is_empty(), "this machine's settings untouched");
            assert_eq!(shell.rest_key_said(base), app::lang::t(app::lang::Key::NetRestKeyFromFile));
            assert_eq!(eravon_chain::rest::key_of("rest.example.test").as_deref(), Some(FILE_KEY));
            assert_eq!(refused(apply(&mut shell, Action::AddRest { base: base.into(), key: Secret::of("") })), "NETWORK_LISTED", "one service, one row");
            match apply(&mut shell, Action::AddRest { base: base.into(), key: Secret::of("MACHINE-KEY-9b2c") }) {
                Applied::RestKeyed { .. } => {}
                other => panic!("{other:?}"),
            }
            assert_eq!(eravon_chain::rest::key_of("rest.example.test").as_deref(), Some("MACHINE-KEY-9b2c"), "this machine's key wins");
            assert_eq!(shell.rest_key_said(base), app::lang::fill1(app::lang::Key::NetRestKeyTail, "9b2c"));
            let plain = settings_plain(&dir);
            let sealed = app::machine::read().expect("machine").rest_keys.0;
            let plain_machine = std::fs::read_to_string(app::machine::path().expect("path")).unwrap_or_default();
            assert!(!plain.contains(FILE_KEY) && !plain_machine.contains(FILE_KEY) && sealed.iter().all(|(_, k)| k != FILE_KEY), "the file's key is written nowhere");
            let _ = std::fs::remove_dir_all(&dir);
        }
        Ok(form) => {
            let (mut shell, dir) = shell_with_home(&format!("keys-file-{form}"));
            let want = match form {
                "unreadable" => eravon_chain::rest::KeysFileBad::Unreadable,
                "notUtf8" => eravon_chain::rest::KeysFileBad::NotUtf8,
                _ => eravon_chain::rest::KeysFileBad::Shape,
            };
            assert_eq!(app::chainx::keys_file(), Some(want), "{form}");
            let said = app::chainx::keys_file_said().expect("said");
            assert!(said.contains(app::chainx::REST_KEYS_ENV) && !said.contains(FILE_KEY), "by the variable and its reason alone: {said}");
            assert_eq!(eravon_chain::rest::key_of("rest.example.test"), None, "none of it taken");
            assert_eq!(refused(apply(&mut shell, Action::AddRest { base: base.into(), key: Secret::of("") })), "SETTINGS_SHAPE", "{form}");
            let _ = std::fs::remove_dir_all(&dir);
        }
        Err(_) => {
            let at = std::env::temp_dir().join(format!("eravon-rest-keys-file-{}", std::process::id()));
            let _ = std::fs::create_dir_all(&at);
            let me = std::env::current_exe().expect("this test binary");
            for (form, bytes) in [
                ("given", format!("# 钥匙文件\n\nrest.example.test {FILE_KEY}\n").into_bytes()),
                ("unreadable", Vec::new()),
                ("notUtf8", vec![0xff, 0xfe, b'\n']),
                ("shape", format!("rest.example.test {FILE_KEY} more\n").into_bytes()),
            ] {
                let file = at.join(form);
                if form != "unreadable" {
                    std::fs::write(&file, &bytes).expect("laid");
                }
                let out = super::output_of(std::process::Command::new(&me).args([full.as_str(), "--exact", "--nocapture", "--test-threads=1"]).env(FORM, form).env(app::chainx::REST_KEYS_ENV, &file)).expect("the child runs");
                assert!(out.status.success() && String::from_utf8_lossy(&out.stdout).contains("1 passed"), "{form}\n{}\n{}", super::child_lines(&out.stdout), super::child_lines(&out.stderr));
            }
            let _ = std::fs::remove_dir_all(&at);
        }
    }
}
