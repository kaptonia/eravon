mod common;

use common::{node, Home, Node, Ran, Scene, AMOUNT, PIN, TO, TOKEN};
use std::time::{Duration, Instant};

fn send(h: &Home, nodes: &[&Node]) -> (Ran, Duration) {
    let mut args: Vec<&str> = vec!["buyer-wallet-send", "--data", h.data_s(), "--writer", "t", "--now", "1758412800", "--chain", "31337"];
    for n in nodes {
        args.extend(["--endpoint", n.url.as_str()]);
    }
    args.extend(["--to", TO, "--token", TOKEN, "--amount", AMOUNT, "--send", "true", "--wait", "0", "--passcode", PIN]);
    let start = Instant::now();
    let o = std::process::Command::new(common::BIN).args(&args).env(eravon_os::HOME_VAR, &h.home).output().expect("eravon runs");
    let took = start.elapsed();
    let text = String::from_utf8_lossy(&o.stdout).to_string();
    (Ran { code: o.status.code().unwrap_or(-1), out: eravon::json::parse(&o.stdout).unwrap_or(eravon::json::Value::Null), text, err: String::from_utf8_lossy(&o.stderr).to_string() }, took)
}

#[derive(Clone, Copy, Debug)]
enum Silent {
    Fails,
    Hangs,
}

#[derive(Clone, Copy, Debug)]
enum Nonce {
    Both,
    HolderOnly,
    BehindOnly,
}

fn rerun(tag: &str, silent: Silent, nonce: Nonce) -> (Ran, Duration, usize, usize) {
    let h = Home::new(tag);
    let (holder, behind) = (node(), node());
    holder.set(Scene::default());
    behind.set(Scene::default());
    let (first, _) = send(&h, &[&holder, &behind]);
    assert_eq!(first.code, 0, "the first send: {} {}", first.text, first.err);
    let about = vec!["eth_getTransactionReceipt", "eth_getTransactionByHash"];
    let (mut hang, mut fail) = (Vec::new(), Vec::new());
    match silent {
        Silent::Fails => fail.extend(about.iter().copied()),
        Silent::Hangs => hang.extend(about.iter().copied()),
    }
    if matches!(nonce, Nonce::BehindOnly) {
        fail.push("eth_getTransactionCount");
    }
    holder.set(Scene { latest: 1, pending: Some(1), hang, fail, ..Scene::default() });
    let behind_fail = if matches!(nonce, Nonce::HolderOnly) { vec!["eth_getTransactionCount"] } else { Vec::new() };
    behind.set(Scene { latest: 0, pending: Some(0), fail: behind_fail, ..Scene::default() });
    let (r, took) = send(&h, &[&holder, &behind]);
    (r, took, holder.broadcasts(), behind.broadcasts())
}

#[test]
#[ignore]
fn void_rule_reading() {
    for silent in [Silent::Fails, Silent::Hangs] {
        for nonce in [Nonce::Both, Nonce::HolderOnly, Nonce::BehindOnly] {
            let (r, took, a, b) = rerun(&format!("void-{silent:?}-{nonce:?}").to_lowercase(), silent, nonce);
            println!("ROW silent={silent:?} nonce={nonce:?} code={} reason={} token={} stepMs={} broadcastsHolder={a} broadcastsBehind={b} err={}", r.code, r.get("reason"), r.get("downToken"), took.as_millis(), r.text.chars().take(200).collect::<String>().replace('\n', " "));
        }
    }
}

#[test]
fn a_silent_holder_and_a_node_behind_never_make_a_send_void() {
    for nonce in [Nonce::Both, Nonce::HolderOnly] {
        let (r, _, a, b) = rerun(&format!("void-guard-{nonce:?}").to_lowercase(), Silent::Fails, nonce);
        assert_eq!((r.get("reason"), r.get("downToken")), ("E_V_DOWN".to_string(), "E_WATCH_NOT_VOID".to_string()), "{nonce:?}: {} {}", r.text, r.err);
        assert_eq!((a, b), (1, 1), "{nonce:?}: nothing handed again");
    }
    let h = Home::new("void-guard-honest");
    let (holder, behind) = (node(), node());
    holder.set(Scene::default());
    behind.set(Scene::default());
    assert_eq!(send(&h, &[&holder, &behind]).0.code, 0);
    holder.set(Scene { latest: 1, pending: Some(1), ..Scene::default() });
    behind.set(Scene { latest: 0, pending: Some(0), ..Scene::default() });
    let (r, _) = send(&h, &[&holder, &behind]);
    assert_eq!((r.get("reason"), r.get("downToken")), ("E_V_DOWN".to_string(), "E_WATCH_NOT_VOID".to_string()), "one past, one behind: {} {}", r.text, r.err);
    assert_eq!((holder.broadcasts(), behind.broadcasts()), (1, 1), "one past, one behind: nothing handed again");
    behind.set(Scene { latest: 1, pending: Some(1), ..Scene::default() });
    let (r, _) = send(&h, &[&holder, &behind]);
    assert_eq!(r.get("reason"), "E_V_SENT_VOID", "{} {}", r.text, r.err);
}
