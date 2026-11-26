mod common;

mod house_route;

use common::{Home, PIN};
use house_route::{balance, clean, node, stub, way, NOW};

#[test]
fn a_read_by_the_homes_nodes_goes_the_machines_way_and_never_the_terminals() {
    let h = Home::new("machine-route");
    let n = node();
    let node_url = format!("http://node.invalid:{n}");
    let set = h.run(&["buyer-settings-endpoints", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN, "--address", &h.address, "--nodes", &node_url], |c| clean(c, &[]));
    assert_eq!(set.code, 0, "the identity's nodes are written: {} {}", set.text, set.err);
    let (terminal, terminal_seen) = stub(n);
    let env = [("http_proxy", format!("http://127.0.0.1:{terminal}")), ("https_proxy", format!("http://127.0.0.1:{terminal}"))];
    way(&h, "none");
    let r = balance(&h, None, &env);
    assert_eq!(r.code, 4, "straight, as the machine is set: {} {}", r.text, r.err);
    assert!(terminal_seen.lock().expect("seen").is_empty(), "the terminal's proxy is never asked by a home's read");
    let (machine, machine_seen) = stub(n);
    way(&h, &format!("http://127.0.0.1:{machine}"));
    let r = balance(&h, None, &env);
    assert_eq!(r.code, 0, "through the machine's proxy: {} {}", r.text, r.err);
    assert!(machine_seen.lock().expect("seen").iter().any(|t| t.starts_with("node.invalid:")), "the machine's proxy resolves the node");
    assert!(terminal_seen.lock().expect("seen").is_empty(), "the terminal's proxy is never asked by a home's read");
    let r = balance(&h, Some(&node_url), &env);
    assert_eq!(r.code, 0, "through the terminal's proxy: {} {}", r.text, r.err);
    assert!(terminal_seen.lock().expect("seen").iter().any(|t| t.starts_with("node.invalid:")), "the terminal's proxy resolves the node");
}
