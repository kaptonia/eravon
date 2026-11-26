use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const ELSEWHERE: [(&str, &str, &str); 2] = [
    ("eravon-cli/src/bin/eravon.rs", "eravon-cli/src/verbs.rs", "eravon_chain::action::open("),
    ("app/src/bin/eravon-app.rs", "app/src/task.rs", "eravon_chain::action::open("),
];

fn crates() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn deps() -> BTreeMap<String, BTreeSet<String>> {
    let mut out = BTreeMap::new();
    for e in std::fs::read_dir(crates()).expect("crates 目录").flatten() {
        let Ok(text) = std::fs::read_to_string(e.path().join("Cargo.toml")) else { continue };
        let mut set = BTreeSet::new();
        let mut normal = false;
        for line in text.lines() {
            let l = line.trim();
            if l.starts_with('[') {
                normal = l.ends_with("dependencies]") && !l.contains("dev-dependencies") && !l.contains("build-dependencies");
                continue;
            }
            if normal {
                if let Some(p) = l.split("path = \"../").nth(1).and_then(|x| x.split('"').next()) {
                    set.insert(p.trim_end_matches('/').to_string());
                }
            }
        }
        out.insert(e.file_name().to_string_lossy().to_string(), set);
    }
    out
}

fn reaches(deps: &BTreeMap<String, BTreeSet<String>>, from: &str) -> bool {
    let mut seen = BTreeSet::new();
    let mut todo = vec![from.to_string()];
    while let Some(c) = todo.pop() {
        if c == "eravon-chain" {
            return true;
        }
        if seen.insert(c.clone()) {
            todo.extend(deps.get(&c).into_iter().flatten().cloned());
        }
    }
    false
}

#[test]
fn every_binary_that_reads_the_chain_runs_as_one_action() {
    let deps = deps();
    let mut checked = 0;
    for (krate, _) in &deps {
        let Ok(bins) = std::fs::read_dir(crates().join(krate).join("src/bin")) else { continue };
        if !reaches(&deps, krate) {
            continue;
        }
        for b in bins.flatten().filter(|b| b.path().extension().is_some_and(|x| x == "rs")) {
            let rel = format!("{krate}/src/bin/{}", b.file_name().to_string_lossy());
            let text = std::fs::read_to_string(b.path()).expect("读得到源");
            match ELSEWHERE.iter().find(|(bin, _, _)| *bin == rel) {
                Some((_, file, call)) => {
                    let there = std::fs::read_to_string(crates().join(file)).expect("读得到源");
                    assert!(there.contains(call), "{rel} 的动作开在 {file},那里没有 {call}");
                }
                None => assert!(text.contains("eravon_chain::task::entry("), "{rel} 读链却不经 eravon_chain::task::entry 开动作"),
            }
            checked += 1;
        }
    }
    assert!(checked >= 8, "读链的二进制一个也没核到({checked})");
}
