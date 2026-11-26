mod common;
mod house_route;

use common::{Home, PIN};
use house_route::{clean, node, stub, way, NOW};

#[test]
fn locked_a_read_by_the_homes_nodes_follows_the_system_and_never_the_terminal() {
    let h = Home::new("machine-route-locked");
    let n = node();
    let node_url = format!("http://node.invalid:{n}");
    let set = h.run(&["buyer-settings-endpoints", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--address", &h.address, "--nodes", &node_url], |c| clean(c, &[]));
    assert_eq!(set.code, 0, "the identity's nodes are written: {} {}", set.text, set.err);
    let (machine, machine_seen) = stub(n);
    way(&h, &format!("http://127.0.0.1:{machine}"));
    let (terminal, terminal_seen) = stub(n);
    let env = [("http_proxy", format!("http://127.0.0.1:{terminal}")), ("https_proxy", format!("http://127.0.0.1:{terminal}"))];
    let r = h.run(&["buyer-wallet-balance", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--chain", "31337", "--address", &h.address], |c| clean(c, &env));
    assert_ne!(r.code, 0, "locked: the identity's nodes are not read: {} {}", r.text, r.err);
    assert!(machine_seen.lock().expect("seen").is_empty() && terminal_seen.lock().expect("seen").is_empty(), "no proxy asked on the binary");
    assert!(eravon_vault::places::set(eravon_vault::places::Places { key_account: eravon_vault::places::ACCOUNT.to_string(), machine_dir: Some(h.machine()), user_home: Some(h.home.clone()) }));
    for v in ["all_proxy", "ALL_PROXY", "no_proxy", "NO_PROXY", "HTTP_PROXY", "HTTPS_PROXY"] {
        // SAFETY: this test binary holds this one test; no other thread reads the environment meanwhile.
        unsafe { std::env::remove_var(v) };
    }
    for (k, v) in &env {
        // SAFETY: as above.
        unsafe { std::env::set_var(k, v) };
    }
    let config = h.home.join(".config");
    std::fs::create_dir_all(config.join("glib-2.0/settings")).expect("the desktop's settings folder");
    std::fs::write(config.join("glib-2.0/settings/keyfile"), "[system/proxy]\nmode='none'\n").expect("the desktop's own proxy setting: none");
    // SAFETY: as above.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", &config) };
    // SAFETY: as above.
    unsafe { std::env::remove_var("XDG_CURRENT_DESKTOP") };
    assert_eq!(eravon_cli::home::machine_proxy(), None, "locked: the sealed setting is not in hand");
    let (table, _) = eravon_cli::home::table(&[node_url.clone()], None, None).expect("the table");
    let t = eravon_cli::home::transport(None, None, true).expect("the transport");
    let _ = eravon_chain::read::height(&t, &table);
    assert!(terminal_seen.lock().expect("seen").is_empty(), "a home's read, locked: the terminal's proxy is never asked");
    assert!(machine_seen.lock().expect("seen").is_empty(), "a home's read, locked: the sealed setting is not guessed at");
    let t = eravon_cli::home::transport(None, None, false).expect("the transport");
    eravon_chain::read::height(&t, &table).expect("nodes given on the command line: through the terminal's proxy");
    assert!(terminal_seen.lock().expect("seen").iter().any(|t| t.starts_with("node.invalid:")), "the terminal's proxy resolves the node");
}
