mod common;

use common::{tree, Home, PIN};

const NOW: &str = "1758412900";

#[test]
fn a_known_network_is_chosen_and_any_other_is_refused_with_nothing_written() {
    let h = Home::new("net-choose");
    let net = |chain: &str| h.run(&["buyer-settings-network", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--chain", chain], |_| {});
    let chain = |r: &common::Ran| r.out.member("chainId").and_then(|v| v.as_int());
    let main = net("1");
    assert_eq!((main.code, chain(&main)), (0, Some(1)), "to mainnet: {} {}", main.text, main.err);
    let back = net("11155111");
    assert_eq!((back.code, chain(&back)), (0, Some(11155111)), "back to Sepolia: {} {}", back.text, back.err);
    let before = (tree(&h.data), tree(&h.home));
    for chain in ["5", "424242", "mainnet", ""] {
        let r = net(chain);
        assert_eq!((r.code, r.get("reason"), r.get("downToken")), (1, "E_V_DOWN".to_string(), "E_S9_NETWORK_UNKNOWN".to_string()), "{chain:?}: {} {}", r.text, r.err);
    }
    let r = h.run(&["buyer-settings-network", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN], |_| {});
    assert_eq!((r.code, r.get("reason")), (1, "E_V_ARG".to_string()), "--chain left out: {} {}", r.text, r.err);
    assert!(before == (tree(&h.data), tree(&h.home)), "a refused switch writes nothing");
}
