mod common;

use common::{node, ok, Home, Node, Scene, PIN};

const CAP: u64 = 30_000;

fn ranges(n: &Node) -> Vec<(u64, u64)> {
    let q = |p: &str, k: &str| -> Option<u64> {
        let at = p.find(&format!("\"{k}\":\"0x"))? + k.len() + 6;
        u64::from_str_radix(p[at..].split('"').next()?, 16).ok()
    };
    n.questions.lock().unwrap_or_else(|e| e.into_inner()).iter().filter(|(m, _)| m == "eth_getLogs").filter_map(|(_, p)| Some((q(p, "fromBlock")?, q(p, "toBlock")?))).collect()
}

fn list(tag: &str, nodes: &[&Node], head: u64) -> (i32, String, Vec<Vec<(u64, u64)>>) {
    let h = Home::new(tag);
    let mut args: Vec<String> = ["buyer-list", "--data", h.data_s(), "--writer", "t", "--now", "1758412800", "--chain", "31337"].iter().map(|s| s.to_string()).collect();
    for n in nodes {
        args.extend(["--endpoint".to_string(), n.url.clone()]);
    }
    let _ = head;
    args.extend(["--passcode".to_string(), PIN.to_string()]);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let r = h.run(&refs, |_| {});
    (r.code, format!("{} {}", r.text, r.err), nodes.iter().map(|n| ranges(n)).collect())
}

#[test]
fn a_cold_read_on_bounded_nodes_is_read_in_halved_segments_and_one_whole_node_is_never_cut() {
    let head = 0x30000u64;
    let bounded = || {
        let n = node();
        n.set(Scene { head: ok(&format!("\"0x{head:x}\"")), logs: Some("[]".into()), log_cap: Some(CAP), ..Scene::default() });
        n
    };
    let halved = |(f, t): &(u64, u64)| *t >= head || (t - f + 1) * 2 <= head + 2;
    let (a, b) = (bounded(), bounded());
    let (code, said, asked) = list("logs-seg-both", &[&a, &b], head);
    assert_eq!(code, 0, "the list reads: {said}");
    assert!(asked.iter().any(|r| r.iter().any(|(f, t)| t - f + 1 <= CAP)), "a node read in segments: {asked:?}");
    assert!(asked.iter().flatten().all(halved), "every segment a halving of the range: {asked:?}");
    let whole = node();
    whole.set(Scene { head: ok(&format!("\"0x{head:x}\"")), logs: Some("[]".into()), ..Scene::default() });
    let narrow = bounded();
    let (code, said, asked) = list("logs-seg-one", &[&whole, &narrow], head);
    assert_eq!(code, 0, "the list reads: {said}");
    assert!(asked.iter().flatten().all(|(_, t)| *t >= head), "nothing cut: {asked:?}");
}

#[test]
fn a_node_that_did_not_answer_holds_no_read_back_and_each_node_refused_is_cut_by_halves() {
    let head = 0x30000u64;
    let at = |cap: Option<u64>, fail: bool| {
        let n = node();
        n.set(Scene { head: ok(&format!("\"0x{head:x}\"")), logs: Some("[]".into()), log_cap: cap, fail: if fail { vec!["eth_getLogs"] } else { Vec::new() }, ..Scene::default() });
        n
    };
    let halved = |(f, t): &(u64, u64)| *t >= head || (t - f + 1) * 2 <= head + 2;
    let (whole, thirty, ten) = (at(None, true), at(Some(CAP), false), at(Some(10_000), false));
    let (code, said, asked) = list("logs-seg-own", &[&whole, &thirty, &ten], head);
    assert_eq!(code, 0, "the list reads: {said}");
    assert!(asked[1].iter().any(|(f, t)| t - f + 1 <= CAP) || asked[2].iter().any(|(f, t)| t - f + 1 <= 10_000), "a bounded node read in segments it answered: {asked:?}");
    assert!(asked.iter().flatten().all(halved), "halves, never the number a refusal says: {asked:?}");
}
