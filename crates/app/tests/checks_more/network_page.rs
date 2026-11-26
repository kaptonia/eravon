use super::vault_open;
use app::lang::{fill1, t, Key};
use app::window::NetPress;
use eravon_ui::egui;

const W: f32 = 1180.0;
const H: f32 = 1600.0;
const ETHERSCAN: &str = "https://api.etherscan.io/v2/api";

fn shell_with_home(name: &str, chain: Option<u64>, nodes: &[&str]) -> (app::shell::Shell, std::path::PathBuf) {
    vault_open();
    let dir = std::env::temp_dir().join(format!("eravon-net-page-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let ctx = egui::Context::default();
    let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    let home = app::home::Home::open_or_create(&dir).expect("a home");
    shell.lock = Some(app::lock::take(&home).expect("the lock"));
    shell.home = Some(home);
    shell
        .commit_settings(|s| {
            s.chain_id = chain;
            s.endpoints = nodes.iter().map(|n| n.to_string()).collect();
        })
        .expect("the settings");
    (shell, dir)
}

fn press(shell: app::shell::Shell, p: NetPress) -> (app::shell::Shell, Vec<String>) {
    let ctx = egui::Context::default();
    let _ = eravon_ui::skin::dress(&ctx);
    let (shell, before, after) = app::window::probe_network_press(&ctx, shell, p, W, H);
    let new = after.into_iter().filter(|x| !before.contains(x)).collect();
    (shell, new)
}

#[test]
fn the_rpc_card_adds_and_removes_one_node_a_row() {
    if super::alone_in(module_path!(), "the_rpc_card_adds_and_removes_one_node_a_row") {
        return;
    }
    let here = eravon_literals::constants::current_network().standard.chain_id;
    let on = |u: &str| format!("{here}={u}");
    let first = on("http://127.0.0.1:9");
    let (shell, dir) = shell_with_home("rpc", Some(here), &[first.as_str()]);
    let (shell, _) = press(shell, NetPress::AddRpc("http://127.0.0.1:7".into()));
    assert_eq!(shell.settings.endpoints, vec![on("http://127.0.0.1:9"), on("http://127.0.0.1:7")], "an address alone takes this home's chain");
    let (mut shell, drawn) = press(shell, NetPress::AddRpc("5=http://127.0.0.1:8".into()));
    assert_eq!(shell.settings.endpoints.last().map(String::as_str), Some("5=http://127.0.0.1:8"));
    assert!(drawn.contains(&fill1(Key::ChainNumbered, "5")), "a node of another chain says its chain: {drawn:?}");
    assert!(drawn.contains(&"http://127.0.0.1:8".to_string()), "its row gives the address as written: {drawn:?}");
    for (typed, why) in [(on("http://127.0.0.1:7"), Key::NetRpcListed), ("http://127.0.0.1:6 http://127.0.0.1:5".to_string(), Key::NetRpcSpace)] {
        let before = shell.settings.endpoints.clone();
        let (back, drawn) = press(shell, NetPress::AddRpc(typed.clone()));
        assert_eq!(back.settings.endpoints, before, "{typed}: nothing saved");
        assert!(drawn.contains(&t(why).to_string()), "{typed}: refused under the cell: {drawn:?}");
        shell = back;
    }
    let (shell, _) = press(shell, NetPress::AtRowEnd("http://127.0.0.1:7".into()));
    assert_eq!(shell.settings.endpoints, vec![on("http://127.0.0.1:9"), "5=http://127.0.0.1:8".to_string()], "that node alone is removed");
    let (shell, _) = press(shell, NetPress::AtRowEnd("http://127.0.0.1:8".into()));
    assert_eq!(shell.settings.endpoints, vec![on("http://127.0.0.1:9")], "another chain's last node removed is gone");
    drop(shell);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_address_alone_takes_the_network_the_home_is_on() {
    if super::alone_in(module_path!(), "an_address_alone_takes_the_network_the_home_is_on") {
        return;
    }
    let (shell, dir) = shell_with_home("nochain", None, &[]);
    let chain = eravon_literals::constants::current_network().standard.chain_id;
    let (shell, _) = press(shell, NetPress::AddRpc("http://127.0.0.1:7".into()));
    assert_eq!(shell.settings.endpoints, vec![format!("{chain}=http://127.0.0.1:7")]);
    let (shell, _) = press(shell, NetPress::AddRpc("11155111=http://127.0.0.1:6".into()));
    assert_eq!(shell.settings.endpoints.last().map(String::as_str), Some("11155111=http://127.0.0.1:6"));
    drop(shell);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_rest_card_comes_with_its_switch_and_takes_one_service_once() {
    if super::alone_in(module_path!(), "the_rest_card_comes_with_its_switch_and_takes_one_service_once") {
        return;
    }
    let (shell, dir) = shell_with_home("rest", Some(1), &["1=https://rpc.example"]);
    assert!(!shell.settings.rest_on, "off by default");
    let (shell, drawn) = press(shell, NetPress::AtRowEnd(t(Key::NetRestMode).to_string()));
    assert!(shell.settings.rest_on);
    for k in [Key::NetRestTitle, Key::NetRestBase, Key::NetRestKey] {
        assert!(drawn.contains(&t(k).to_string()), "{:?} shows with the switch on: {drawn:?}", t(k));
    }
    let (shell, drawn) = press(shell, NetPress::AddRest(ETHERSCAN.into(), "ETHERSCAN-TEST-KEY-7f3a".into()));
    assert_eq!(shell.settings.rest, vec![ETHERSCAN.to_string()]);
    for w in [t(Key::NetSvcEtherscan).to_string(), fill1(Key::NetRestKeyTail, "7f3a")] {
        assert!(drawn.contains(&w), "{w}: {drawn:?}");
    }
    let (shell, drawn) = press(shell, NetPress::AddRest("https://API.etherscan.io/api".into(), "OTHER-KEY".into()));
    assert_eq!(shell.settings.rest, vec![ETHERSCAN.to_string()], "one service, one row");
    assert_eq!(app::machine::read().expect("machine").rest_keys.of("api.etherscan.io"), Some("ETHERSCAN-TEST-KEY-7f3a"), "the key kept is the first");
    assert!(drawn.contains(&t(Key::NetRestListed).to_string()), "refused under the cells: {drawn:?}");
    let mut shell = shell;
    shell.machine = app::machine::update(|m| m.rest_keys.0.clear()).expect("this machine's keys cleared");
    let (shell, drawn) = press(shell, NetPress::AddRest("https://api.etherscan.io/api".into(), "REFILLED-KEY-5e7d".into()));
    assert_eq!(shell.settings.rest, vec![ETHERSCAN.to_string()], "the row as it was");
    assert_eq!(shell.rest_key_tail(ETHERSCAN).as_deref(), Some("5e7d"), "its key here now");
    assert!(drawn.contains(&fill1(Key::NetRestKeyTail, "5e7d")), "{drawn:?}");
    let mut shell = shell;
    shell.machine = app::machine::update(|m| m.rest_keys.0.clear()).expect("cleared again");
    let ctx = egui::Context::default();
    let _ = eravon_ui::skin::dress(&ctx);
    let (shell, drawn, _) = app::window::probe_network_press(&ctx, shell, NetPress::AtRowEnd(String::new()), W, H);
    assert!(drawn.contains(&t(Key::NetRestNoKey).to_string()), "a row with no key here says so: {drawn:?}");
    let (shell, _) = press(shell, NetPress::AddRest(ETHERSCAN.into(), "AGAIN-KEY-1a2b".into()));
    let (shell, _) = press(shell, NetPress::AtRowEnd(t(Key::NetRestMode).to_string()));
    assert!(!shell.settings.rest_on && shell.settings.rest == vec![ETHERSCAN.to_string()], "off keeps the row");
    let (shell, _) = press(shell, NetPress::AtRowEnd(t(Key::NetRestMode).to_string()));
    let (shell, _) = press(shell, NetPress::AtRowEnd(ETHERSCAN.into()));
    assert!(shell.settings.rest.is_empty(), "removed");
    assert!(app::machine::read().expect("machine").rest_keys.of("api.etherscan.io").is_none(), "with this machine's key");
    drop(shell);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_cap_on_questions_per_node_is_one_row_taking_one_to_a_thousand_and_kept_with_the_nodes() {
    use app::action::{apply, Action, Applied};
    let (mut shell, dir) = shell_with_home("log-asks", Some(31337), &["31337=http://127.0.0.1:9"]);
    for n in [0u64, 1001] {
        assert!(matches!(apply(&mut shell, Action::SetLogAsks { n }), Applied::Trouble(_)), "{n} is refused");
    }
    let home = app::home::Home::open_or_create(&dir).expect("the home");
    assert_eq!(app::settings::Settings::read(&home).expect("read").log_asks, None, "nothing outside the bounds is kept");
    assert_eq!(shell.settings.log_asks(), eravon_chain::action::ASKS_DEFAULT, "the default until one is set");
    for n in [1u64, 1000, 7] {
        assert!(matches!(apply(&mut shell, Action::SetLogAsks { n }), Applied::LogAsks(m) if m == n), "{n} is taken");
    }
    let kept = app::settings::Settings::read(&home).expect("read");
    assert_eq!((kept.log_asks, kept.endpoints.clone()), (Some(7), vec!["31337=http://127.0.0.1:9".to_string()]), "kept in the settings beside the nodes");
    assert_eq!(kept.ground().log_asks, Some(7), "the command line's reading of the home sees the same number");
    let ctx = egui::Context::default();
    let _ = eravon_ui::skin::dress(&ctx);
    let (_shell, drawn, _) = app::window::probe_network_press(&ctx, shell, NetPress::AddRpc(String::new()), W, H);
    for words in [t(Key::SetLogAsks), t(Key::DoSetLogAsks), "7"] {
        assert!(drawn.iter().any(|d| d == words), "the page draws {words}: {drawn:?}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
