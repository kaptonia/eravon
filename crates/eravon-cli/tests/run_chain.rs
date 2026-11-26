mod common;

use common::{node, Home, PIN};

const NOW: &str = "1758412900";
const SEPOLIA: &str = "11155111";

fn asked(n: &common::Node) -> usize {
    n.questions.lock().unwrap_or_else(|e| e.into_inner()).len()
}

#[test]
fn a_named_chain_reads_and_sends_by_that_networks_nodes_or_is_refused_by_name() {
    let h = Home::new("run-chain");
    let (main, sep) = (node(), node());
    let set = |chain: &str, url: &str| {
        let r = h.run(&["buyer-settings-endpoints", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--chain", chain, "--nodes", url], |_| {});
        assert_eq!(r.code, 0, "nodes for {chain}: {} {}", r.text, r.err);
    };
    set(SEPOLIA, &sep.url);
    set("1", &main.url);
    let net = h.run(&["buyer-settings-network", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--chain", "1"], |_| {});
    assert_eq!(net.code, 0, "home on mainnet: {} {}", net.text, net.err);
    let read = |chain: Option<&str>| {
        let mut a = vec!["buyer-wallet-balance", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN];
        if let Some(c) = chain {
            a.extend(["--chain", c]);
        }
        h.run(&a, |_| {})
    };
    let send = |chain: &str| {
        h.run(
            &[
                "buyer-wallet-send", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--chain", chain, "--to", common::TO, "--token", common::TOKEN,
                "--amount", "1", "--send", "true",
            ],
            |_| {},
        )
    };
    let r = read(Some(SEPOLIA));
    assert!(asked(&sep) > 0 && asked(&main) == 0, "read on Sepolia by Sepolia's node: sep {} main {}: {} {}", asked(&sep), asked(&main), r.text, r.err);
    let r = send(SEPOLIA);
    assert!(asked(&main) == 0, "send on Sepolia never reaches mainnet's node: {} {}", r.text, r.err);
    let before = asked(&sep);
    let r = read(None);
    assert!(asked(&main) > 0 && asked(&sep) == before, "no --chain: the home's network: {} {}", r.text, r.err);
    let h2 = Home::new("run-chain-none");
    let own = eravon_literals::constants::default_network().standard.chain_id;
    let other = if own == 1 { SEPOLIA.to_string() } else { "1".to_string() };
    let r = h2.run(&["buyer-settings-endpoints", "--data", h2.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--chain", &own.to_string(), "--nodes", &main.url], |_| {});
    assert_eq!(r.code, 0, "{} {}", r.text, r.err);
    let m = asked(&main);
    for chain in [other.as_str()] {
        let read = h2.run(&["buyer-wallet-balance", "--data", h2.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--chain", chain], |_| {});
        let send = h2.run(
            &[
                "buyer-wallet-send", "--data", h2.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--chain", chain, "--to", common::TO, "--token", common::TOKEN,
                "--amount", "1", "--send", "true",
            ],
            |_| {},
        );
        for r in [read, send] {
            assert_ne!(r.code, 0, "{} {}", r.text, r.err);
            assert!(r.text.contains(&format!("--chain {chain}")) && r.text.contains("--endpoint"), "the refusal names the chain and the way out: {}", r.text);
            assert_eq!(asked(&main), m, "nothing asked of another network's node: {}", r.text);
        }
    }
    for chain in ["424242"] {
        let r = h2.run(&["buyer-wallet-balance", "--data", h2.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--chain", chain], |_| {});
        assert!(r.code != 0 && r.text.contains("E_CHAIN_WRONG"), "a chain the home's nodes are not on is refused by name: {} {}", r.text, r.err);
        assert!(!r.text.contains("\"ok\":true"));
    }
    let r = h2.run(&["buyer-wallet-balance", "--data", h2.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--chain", &common::CHAIN.to_string()], |_| {});
    assert!(!r.text.contains("E_CHAIN_WRONG") && !r.text.contains("没有端点表"), "the bench chain its nodes answer is read by them: {} {}", r.text, r.err);
}

#[test]
fn a_run_without_chain_keeps_to_its_homes_network_the_verbs_that_read_no_nodes_included() {
    let h = Home::new("run-chain-home");
    let net = h.run(&["buyer-settings-network", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--chain", SEPOLIA], |_| {});
    assert_eq!(net.code, 0, "home on Sepolia: {} {}", net.text, net.err);
    let usdc = eravon_literals::constants::SETTLEMENT_TOKENS_SEPOLIA.iter().find(|t| t.symbol == "USDC").expect("Sepolia's USDC").address.to_ascii_lowercase();
    let remove = |token: &str| h.run(&["buyer-settings-token-remove", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--token", token], |_| {});
    let r = remove(&usdc);
    assert_eq!(r.code, 0, "Sepolia's factory row is this run's: {} {}", r.text, r.err);
    assert!(r.text.contains(&usdc), "{}", r.text);
    let again = remove(&usdc);
    assert_ne!(again.code, 0, "gone from Sepolia's rows: {}", again.text);
    let main_usdc = eravon_literals::constants::NETWORKS.iter().find(|n| n.standard.chain_id == 1).and_then(|n| n.tokens.iter().find(|t| t.symbol == "USDC")).expect("mainnet USDC").address.to_ascii_lowercase();
    let r = h.run(&["buyer-settings-token-remove", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--chain", "1", "--token", &main_usdc], |_| {});
    assert_eq!(r.code, 0, "mainnet's row untouched by the Sepolia run: {} {}", r.text, r.err);
}
