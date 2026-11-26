mod common;

use common::{node, Home, Node, Ran, Scene, AMOUNT, PIN, TO, TOKEN};
use std::time::{Duration, Instant};

const POINT: [&str; 8] = ["eth_chainId", "eth_blockNumber", "eth_getTransactionCount", "eth_getBlockByNumber", "eth_feeHistory", "eth_estimateGas", "eth_call", "eth_getBalance"];

fn dry(h: &Home, nodes: &[&Node]) -> (Ran, Duration) {
    let mut args: Vec<&str> = vec!["buyer-wallet-send", "--data", h.data_s(), "--writer", "t", "--now", "1758412800", "--chain", "31337"];
    for n in nodes {
        args.extend(["--endpoint", n.url.as_str()]);
    }
    args.extend(["--to", TO, "--token", TOKEN, "--amount", AMOUNT, "--send", "false", "--passcode", PIN]);
    let start = Instant::now();
    let o = std::process::Command::new(common::BIN).args(&args).env(eravon_os::HOME_VAR, &h.home).output().expect("eravon runs");
    let took = start.elapsed();
    let text = String::from_utf8_lossy(&o.stdout).to_string();
    (Ran { code: o.status.code().unwrap_or(-1), out: eravon::json::parse(&o.stdout).unwrap_or(eravon::json::Value::Null), text, err: String::from_utf8_lossy(&o.stderr).to_string() }, took)
}

fn but_nodes(v: &eravon::json::Value) -> eravon::json::Value {
    match v {
        eravon::json::Value::Obj(m) => eravon::json::Value::Obj(m.iter().filter(|(k, _)| !["endpoint", "endpoints", "sources", "singleSource", "missing"].contains(&k.as_str())).map(|(k, v)| (k.clone(), but_nodes(v))).collect()),
        eravon::json::Value::Arr(a) => eravon::json::Value::Arr(a.iter().map(but_nodes).collect()),
        other => other.clone(),
    }
}

fn three(hung: bool) -> [Node; 3] {
    let [a, b, c] = [node(), node(), node()];
    a.set(Scene::default());
    b.set(Scene::default());
    c.set(Scene { hang: if hung { POINT.to_vec() } else { Vec::new() }, ..Scene::default() });
    [a, b, c]
}

#[test]
#[ignore]
fn hung_reads_reading() {
    let h = Home::new("hung-reads-well");
    let well = three(false);
    let (plain, took) = dry(&h, &[&well[0], &well[1], &well[2]]);
    let asked = |n: &Node| POINT.iter().map(|m| n.asked(m)).sum::<usize>();
    println!("ROW form=all-answer code={} reason={} stepMs={} asked={:?}", plain.code, plain.get("reason"), took.as_millis(), well.iter().map(asked).collect::<Vec<_>>());
    let h = Home::new("hung-reads-hung");
    let hung = three(true);
    let (r, took) = dry(&h, &[&hung[0], &hung[1], &hung[2]]);
    println!("ROW form=third-hung code={} reason={} stepMs={} sameAnswer={} asked={:?} held={}", r.code, r.get("reason"), took.as_millis(), but_nodes(&r.out) == but_nodes(&plain.out), hung.iter().map(asked).collect::<Vec<_>>(), hung[2].held.lock().unwrap().len());
    println!("PLAIN {}", plain.text.chars().take(600).collect::<String>());
    println!("HUNG {}", r.text.chars().take(600).collect::<String>());
}

#[test]
fn a_hung_node_costs_the_reads_before_a_send_one_bound_a_question() {
    let h = Home::new("hung-guard-well");
    let well = three(false);
    let (plain, _) = dry(&h, &[&well[0], &well[1], &well[2]]);
    assert_eq!(plain.code, 0, "{} {}", plain.text, plain.err);
    let asked = |n: &Node| POINT.iter().map(|m| n.asked(m)).sum::<usize>();
    assert!(well.iter().map(asked).all(|n| (3..=6).contains(&n)), "each node asked the point reads: {:?}", well.iter().map(asked).collect::<Vec<_>>());
    let h = Home::new("hung-guard-hung");
    let hung = three(true);
    let (r, took) = dry(&h, &[&hung[0], &hung[1], &hung[2]]);
    assert_eq!(r.code, 0, "{} {}", r.text, r.err);
    assert_eq!(but_nodes(&r.out), but_nodes(&plain.out), "the same answer as when all three answer");
    let held = hung[2].held.lock().unwrap().len() as u32;
    assert!(held > 0 && took < eravon_chain::send::ONE_ASK * held + Duration::from_secs(5), "within its hung questions' bounds: {took:?} for {held}");
}

fn balance(h: &Home, nodes: &[&Node]) -> (Ran, Duration) {
    let mut args: Vec<&str> = vec!["buyer-wallet-balance", "--data", h.data_s(), "--writer", "t", "--now", "1758412800", "--chain", "31337"];
    for n in nodes {
        args.extend(["--endpoint", n.url.as_str()]);
    }
    args.extend(["--passcode", PIN]);
    let start = Instant::now();
    let o = std::process::Command::new(common::BIN).args(&args).env(eravon_os::HOME_VAR, &h.home).output().expect("eravon runs");
    let took = start.elapsed();
    let text = String::from_utf8_lossy(&o.stdout).to_string();
    (Ran { code: o.status.code().unwrap_or(-1), out: eravon::json::parse(&o.stdout).unwrap_or(eravon::json::Value::Null), text, err: String::from_utf8_lossy(&o.stderr).to_string() }, took)
}

#[test]
fn a_hung_node_costs_a_read_only_action_no_bound() {
    let h = Home::new("hung-balance-well");
    let well = three(false);
    let (plain, _) = balance(&h, &[&well[0], &well[1], &well[2]]);
    assert_eq!(plain.code, 0, "{} {}", plain.text, plain.err);
    let h = Home::new("hung-balance-hung");
    let hung = three(true);
    let (r, took) = balance(&h, &[&hung[0], &hung[1], &hung[2]]);
    assert_eq!(r.code, 0, "{} {}", r.text, r.err);
    assert_eq!(r.get("native"), plain.get("native"), "the same balance");
    assert!(took < eravon_chain::send::ONE_ASK, "no hung question's bound waited: {took:?}");
}
