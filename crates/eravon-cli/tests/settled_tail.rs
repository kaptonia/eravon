mod common;

use common::{node, ok, Home, Node, Scene, PIN};

const NOW: &str = "1758412800";

fn ranges(n: &Node) -> Vec<(u64, u64)> {
    let q = |p: &str, k: &str| -> Option<u64> {
        let at = p.find(&format!("\"{k}\":\"0x"))? + k.len() + 6;
        u64::from_str_radix(p[at..].split('"').next()?, 16).ok()
    };
    n.questions.lock().unwrap_or_else(|e| e.into_inner()).iter().filter(|(m, _)| m == "eth_getLogs").filter_map(|(_, p)| Some((q(p, "fromBlock")?, q(p, "toBlock")?))).collect()
}

fn parts(h: &Home) -> Vec<String> {
    let dir = h.machine().join(eravon_send::checked::DIR);
    let mut out: Vec<String> = std::fs::read_dir(&dir).map(|d| d.flatten().filter_map(|e| e.file_name().into_string().ok()).filter(|f| f.starts_with("logs-") && f != "logs-notes.json").collect()).unwrap_or_default();
    out.sort();
    out
}

fn twice(h: &Home, n: &Node, with: &[&str]) -> (Vec<(u64, u64)>, Vec<String>, Vec<(u64, u64)>) {
    let run = || {
        let mut args = vec!["buyer-list", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--endpoint", &n.url, "--chain", "31337"];
        args.extend_from_slice(with);
        let r = h.run(&args, |_| {});
        assert_eq!(r.code, 0, "the list reads: {} {}", r.text, r.err);
    };
    n.set(Scene { head: ok("\"0x400\""), logs: Some("[]".into()), ..Scene::default() });
    run();
    let first = ranges(n);
    let kept = parts(h);
    n.set(Scene { head: ok("\"0x480\""), logs: Some("[]".into()), ..Scene::default() });
    run();
    (first, kept, ranges(n))
}

fn tail_only(first: &[(u64, u64)], kept: &[String], second: &[(u64, u64)]) {
    assert!(first.iter().any(|(from, _)| *from == 0), "the first run reads from the first block: {first:?}");
    assert!(!kept.is_empty(), "the first run leaves its settled reads on the machine");
    assert!(!second.is_empty(), "the second run still reads the new blocks");
    assert!(second.iter().all(|(from, _)| *from > 0), "the second run asks only after what the first settled: {second:?}");
}

#[test]
fn the_next_run_with_the_passcode_asks_only_the_tail_of_the_logs() {
    let h = Home::new("settled-pin");
    let n = node();
    let (first, kept, second) = twice(&h, &n, &["--passcode", PIN]);
    tail_only(&first, &kept, &second);
}
