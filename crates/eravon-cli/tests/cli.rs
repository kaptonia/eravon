mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eravon");

fn src(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("读不出 {}:{e}", p.display()))
}

fn sources() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    let mut stack = vec![dir];
    while let Some(d) = stack.pop() {
        for item in std::fs::read_dir(&d).expect("列不动 src") {
            let e = item.expect("列不动");
            if e.path().is_dir() {
                stack.push(e.path());
            } else if e.path().extension().and_then(|x| x.to_str()) == Some("rs") {
                let name = e.path().strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")).join("src")).expect("under src").display().to_string();
                out.push((name, std::fs::read_to_string(e.path()).expect("读不出")));
            }
        }
    }
    out
}

fn in_force(text: &str) -> String {
    let mut out = Vec::new();
    for l in text.lines() {
        if !l.trim_start().starts_with("//") {
            out.push(l);
        }
    }
    out.join("\n")
}

struct Scratch(PathBuf);

impl std::ops::Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("eravon-cli-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).expect("建不出草稿地");
    Scratch(p)
}

struct Ran {
    code: i32,
    out: Vec<u8>,
    err: String,
}

fn run_in(dir: &Path, args: &[&str]) -> Ran {
    let o = Command::new(BIN).args(args).current_dir(dir).env(eravon_os::HOME_VAR, dir.join("home")).env_remove("LANG").env_remove("LC_ALL").output().expect("起不动 eravon");
    Ran { code: o.status.code().unwrap_or(-1), out: o.stdout, err: String::from_utf8_lossy(&o.stderr).into_owned() }
}

fn terms(w: &Path) -> String {
    std::fs::write(w.join("t.json"), b"{}").expect("a terms file");
    "t.json".into()
}

#[test]
fn a_misuse_writes_not_one_byte_to_stdout() {
    let w = scratch("misuse");
    for args in [
        vec!["conjure"],
        vec!["contract", "--seed", "1"],
        vec!["seller-terms-hash", "--data", "d", "--now", "1", "--file", "/nowhere/at/all"],
        vec!["seller-terms-hash", "--data", "d", "--now", "1", "--file", ""],
        vec!["seller-terms-hash", "--data", "d"],
    ] {
        let r = run_in(&w, &args);
        assert_eq!(r.code, 2, "{args:?} 该退 2");
        assert!(r.out.is_empty(), "{args:?} 的 stdout 该是零字节");
    }
}

#[test]
fn a_misuse_names_its_subject_on_the_first_line_of_stderr() {
    let w = scratch("stderr");
    let r = run_in(&w, &["contract", "/nowhere/at/all"]);
    assert_eq!(r.err.lines().next().unwrap_or_default(), "E_ARGS #2 (15 bytes)");
    let r = run_in(&w, &["seller-terms-hash", "--data", "d", "--now", "1", "--file", ""]);
    assert_eq!(r.err.lines().next().unwrap_or_default(), "E_ARGS --file");
    let r = run_in(&w, &["seller-terms-hash", "--data", "d", "--now", "1", "--file", "/nowhere/at/all"]);
    assert_eq!(r.err.lines().next().unwrap_or_default(), "E_UNREADABLE #7 (15 bytes)");
}

#[test]
fn an_answer_carries_no_trailing_newline() {
    let w = scratch("newline");
    let r = run_in(&w, &["contract"]);
    assert_eq!(r.code, 0);
    assert!(!r.out.is_empty());
    assert_ne!(*r.out.last().expect("空答"), b'\n');
}

#[test]
fn an_answer_is_canonical_json_with_members_in_byte_order() {
    let w = scratch("canon");
    let t = terms(&w);
    let r = run_in(&w, &["seller-terms-hash", "--data", "d", "--now", "1", "--file", &t]);
    assert_eq!(r.code, 0, "{}", r.err);
    let v = eravon::json::parse(&r.out).expect("one value");
    assert_eq!(eravon::json::canon_bytes(&v), r.out, "the bytes are the canonical form: members in byte order, no white space");
}

#[test]
fn the_law_refuses_what_the_shell_hands_it_and_the_token_comes_back_verbatim() {
    let h = common::Home::new("down");
    let r = h.run(&["buyer-wallet-balance", "--data", h.data_s(), "--writer", "t", "--now", "1", "--endpoint", "wss://node.example/", "--chain", "31337", "--address", &h.address, "--passcode", common::PIN], |_| {});
    assert_eq!(r.code, 1, "{}", r.text);
    assert_eq!((r.get("reason"), r.get("layer"), r.get("downToken")), ("E_V_DOWN".into(), "A2".into(), "E_CHAIN_USAGE".into()), "{}", r.text);
}

#[test]
fn partial_gets_its_own_exit_code_and_is_never_folded_into_green() {
    let h = common::Home::new("partial");
    let n = common::node();
    let mut args = h.send_args(&n);
    let at = args.iter().position(|a| *a == "--wait").expect("the send waits") + 1;
    args[at] = "1";
    let r = h.run(&args, |_| {});
    assert_eq!(r.code, 4, "E_TX_NOT_YET 该有自己那一格:{}", r.text);
    assert_eq!((r.get("reason"), r.get("tx")), ("E_TX_NOT_YET".to_string(), n.hash(0)), "{}", r.text);
    assert_eq!(r.out.member("count"), Some(&eravon::json::Value::Int(1)), "the seconds waited: {}", r.text);
    use eravon_literals::json::Out;
    let refused = |txs: Vec<Out>| Out::O(vec![("ok", Out::B(false)), ("token", Out::S("E_V_REVERTED".into())), ("txs", Out::A(txs))]);
    assert_eq!(eravon_literals::schema::exit_of(&refused(vec![Out::S("0xa".into())])), 3);
    assert_eq!(eravon_literals::schema::exit_of(&refused(Vec::new())), 1);
}

#[test]
fn a_world_that_cannot_answer_is_not_a_world_that_answers_no() {
    let h = common::Home::new("unanswered");
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").expect("绑不到端口");
        let p = l.local_addr().expect("读不出端口").port();
        drop(l);
        p
    };
    let ep = format!("http://127.0.0.1:{port}");
    let r = h.run(&["buyer-wallet-balance", "--data", h.data_s(), "--writer", "t", "--now", "1", "--endpoint", &ep, "--chain", "31337", "--address", &h.address, "--passcode", common::PIN], |_| {});
    assert_eq!(r.code, 4, "答不出该走 4:{}", r.text);
}

#[test]
fn a_rest_keys_file_is_misuse_by_its_flag_and_its_key_is_never_said_back() {
    let h = common::Home::new("rest-keys");
    let key = "SECRET-REST-KEY-5d1e";
    let dead = std::net::TcpListener::bind("127.0.0.1:0").expect("bind").local_addr().expect("addr").port();
    let node = format!("http://127.0.0.1:{dead}");
    let run = |keys: &Path| {
        let keys = keys.to_str().expect("a path").to_string();
        h.run(&["buyer-wallet-balance", "--data", h.data_s(), "--writer", "t", "--now", "1", "--endpoint", &node, "--chain", "31337", "--address", &h.address, "--passcode", common::PIN, "--rest-keys", &keys], |_| {})
    };
    let at = std::env::temp_dir().join(format!("eravon-cli-rest-keys-{}", std::process::id()));
    std::fs::create_dir_all(&at).expect("a folder");
    let bad = at.join("bad.keys");
    std::fs::write(&bad, format!("etherscan {key} more\n")).expect("a file");
    let r = run(&bad);
    assert_eq!(r.code, 2, "a line of another shape is misuse: {}", r.err);
    assert!(r.text.is_empty(), "misuse writes nothing to stdout");
    assert!(r.err.lines().next().unwrap_or_default().contains("--rest-keys"), "named by its flag: {}", r.err);
    assert!(!r.err.contains(key) && !r.err.contains("etherscan"), "the file's text is not said back: {}", r.err);
    let good = at.join("good.keys");
    std::fs::write(&good, format!("# keys\n\netherscan {key}\n")).expect("a file");
    let r = run(&good);
    assert_ne!(r.code, 2, "a file that reads is taken: {}", r.err);
    assert!(!r.text.contains(key) && !r.err.contains(key), "the key is never said back: {} {}", r.text, r.err);
    let _ = std::fs::remove_dir_all(&at);
}

#[cfg(unix)]
#[test]
fn a_disk_failure_says_the_systems_words_in_detail() {
    let h = common::Home::new("disk");
    let w = h.root.clone();
    let locked = w.join("locked");
    std::fs::create_dir_all(&locked).expect("a folder");
    let shut = |on: bool| {
        let mut p = std::fs::metadata(&locked).expect("the folder").permissions();
        p.set_readonly(on);
        std::fs::set_permissions(&locked, p).expect("its permissions");
    };
    let block = common::terms_block(&w);
    shut(true);
    let r = run_in(&w, &["seller-terms-build", "--data", h.data_s(), "--now", "1758412800", "--writer", "t", "--passcode", common::PIN, "--address", &h.address, "--block", block.to_str().expect("utf-8"), "--out", "locked/terms.md"]);
    shut(false);
    let said = String::from_utf8_lossy(&r.out).into_owned();
    assert_eq!(r.code, 4, "{said}");
    let v = eravon::json::parse(&r.out).expect("one value");
    assert_eq!(v.member("reason").and_then(|x| x.as_str()), Some("E_V_IO"), "{said}");
    let detail = v.member("detail").and_then(|x| x.as_str()).unwrap_or_default().to_string();
    assert!(detail.contains("locked") && detail.contains("Permission denied"), "the path and the system's words: {detail}");
    assert!(!locked.join("terms.md").exists());
}

#[test]
fn every_verb_the_shell_runs_marks_the_trace_and_the_bases_it_leans_on_mark_theirs() {
    let w = scratch("trace");
    let trace = w.join("trace.log");
    let o = Command::new(BIN)
        .args(["buyer-vault-init", "--data", "d", "--writer", "t", "--now", "1758412800", "--passcode", common::PIN, "--phrase", common::PHRASE])
        .current_dir(&*w)
        .env(eravon_os::HOME_VAR, w.join("home"))
        .env("ERAVON_TRACE", &trace)
        .output()
        .expect("起不动");
    assert_eq!(o.status.code(), Some(0));
    let marks = std::fs::read_to_string(&trace).unwrap_or_default();
    for id in ["V1", "K1", "A5"] {
        assert!(marks.lines().any(|l| l.trim() == id), "痕迹里没有 {id}:{marks}");
    }
}

#[test]
fn not_one_of_the_twenty_verbs_passes_without_marking_the_trace() {
    for verb in eravon_cli::verbs::VERBS {
        let w = scratch(&format!("mark-{verb}"));
        let trace = w.join("trace.log");
        let _ = Command::new(BIN).arg(verb).current_dir(&*w).env(eravon_os::HOME_VAR, w.join("home")).env("ERAVON_TRACE", &trace).output().expect("起不动");
        let marks = std::fs::read_to_string(&trace).unwrap_or_default();
        assert!(marks.lines().any(|l| l.trim() == "V1"), "verb {verb} ran but the trace has no command-line mark");
    }
}

#[test]
fn not_one_path_that_exits_two_writes_a_byte_to_stdout() {
    let w = scratch("misuse-all");
    let bad_key = "0xzz";
    let out_of_range = format!("0x{}", "00".repeat(32));
    let key = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";
    let mut two = 0usize;
    for c in [
        vec!["buyer-vault-import", "--key", bad_key],
        vec!["buyer-vault-import", "--key", &out_of_range],
        vec!["buyer-vault-import", "--key", key, "--key", key],
        vec!["seller-vault-import", "--key", key, "--key-file", "k"],
    ] {
        let r = run_in(&w, &c);
        assert_eq!(r.code, 2, "{c:?}");
        two += 1;
        assert!(r.out.is_empty(), "{c:?} 退 2 而 stdout 印了 {} 字节", r.out.len());
    }
    for verb in eravon_cli::verbs::VERBS {
        for args in [vec![verb], vec![verb, "word"], vec![verb, "--nosuch", "x"]] {
            let r = run_in(&w, &args);
            if r.code != 2 {
                continue;
            }
            two += 1;
            assert!(r.out.is_empty(), "{args:?} 退 2 而 stdout 印了 {} 字节", r.out.len());
        }
    }
    assert!(two >= 30, "只走到 {two} 条误用路,表太瘦了");
}

#[test]
fn the_crate_leans_on_nothing_but_the_four_bases() {
    let manifest = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).expect("读不出 Cargo.toml");
    let deps = manifest.split("[dependencies]").nth(1).expect("没有 [dependencies]");
    for line in deps.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            break;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        assert!(line.contains("path ="), "第三方依赖:{line}");
    }
}

#[test]
fn every_flag_a_verb_asks_for_lives_in_the_flag_table() {
    let mut asked: Vec<String> = Vec::new();
    for (_, text) in sources() {
        for call in ["f.one(\"--", "f.opt(\"--", "f.need(\"--", "f.many(\"--", "f.int(\"--", "f.opt_int(\"--", "f.int_or(\"--"] {
            let mut rest = text.as_str();
            while let Some(i) = rest.find(call) {
                rest = &rest[i + call.len()..];
                if let Some(j) = rest.find('"') {
                    asked.push(rest[..j].to_string());
                }
            }
        }
    }
    assert!(asked.len() > 30, "扫到的旗太少({}):扫法坏了", asked.len());
    for name in asked {
        assert!(eravon_cli::verbs::is_known_flag(&name), "--{name} 不在旗名闭表里");
    }
}

#[test]
fn the_verbs_are_counted_and_each_one_answers() {
    let names = eravon_cli::verbs::VERBS;
    assert_eq!(names.len(), 175);
    let text = src("verbs_own.rs");
    for v in names {
        assert!(text.contains(&format!("\"{v}\" =>")), "{v} 没有分发的落点");
    }
    let mut sorted = names.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "动词名里有重的");
}

#[test]
fn the_exit_table_is_five_codes_and_they_are_all_different() {
    use eravon_cli::codes::Exit;
    let codes: Vec<u8> = Exit::ALL.iter().map(|e| e.code()).collect();
    assert_eq!(codes, vec![0, 1, 2, 3, 4]);
    let mut uniq = codes.clone();
    uniq.sort_unstable();
    uniq.dedup();
    assert_eq!(uniq.len(), 5);
}

#[test]
fn the_output_key_table_and_the_reason_table_have_no_twin_spellings() {
    use eravon_cli::codes::{Key, Reason};
    let mut reasons: Vec<&str> = Reason::ALL.iter().map(|r| r.as_str()).collect();
    let n = reasons.len();
    reasons.sort_unstable();
    reasons.dedup();
    assert_eq!(reasons.len(), n, "拒因表里有重名");
    for r in Reason::ALL {
        assert!(r.as_str().starts_with("E_"), "{} 不像一枚拒因", r.as_str());
    }
    let mut keys: Vec<&str> = Key::ALL.iter().map(|k| k.as_str()).collect();
    let m = keys.len();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), m, "键表里有重名");
}

#[test]
fn the_entropy_well_is_read_by_length_and_has_no_fallback() {
    let text = src("rand.rs");
    assert!(text.contains("eravon_os::random("), "熵只经 eravon_os 那一处取");
    assert!(!text.contains("fs::read("), "整档读一个无尽的档会挂住");
    assert!(!text.to_lowercase().contains("fallback"));
    let os = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../eravon-os/src/lib.rs")).expect("eravon-os");
    assert!(os.contains("imp::fill_random(buf)"), "取熵在 eravon-os 里每个系统一份实现");
}

#[test]
fn stdout_has_one_writer_and_that_writer_never_returns() {
    for (name, text) in sources() {
        if name == "out.rs" {
            continue;
        }
        let code = in_force(&text);
        for w in ["println!", "print!", "stdout()", "process::exit"] {
            assert!(!code.contains(w), "{name} 里有一处 {w}:印字与退出只许住 out.rs");
        }
    }
    let out_rs = src("out.rs");
    assert!(out_rs.contains("pub fn emit(a: Answer) -> !"), "emit 不再是不返回的:两个出口就不互斥了");
    assert!(out_rs.contains("pub fn misuse(reason: Reason, subject: Subject, said: Said) -> !"));
    assert!(out_rs.contains("pub fn frame(v: &Value)") && out_rs.contains("pub fn finish() -> !"), "mcp 的分帧写口与收尾");
    assert_eq!(out_rs.matches("std::process::exit").count(), 3, "out.rs 里的退出口不是三处");
    for (name, text) in sources() {
        if name == "out.rs" || name == "mcp.rs" {
            continue;
        }
        assert!(!text.contains("out::frame") && !text.contains("out::finish"), "{name} 调了只许 mcp 调的写口");
    }
    for (name, text) in sources() {
        if name == "out.rs" || name == "codes.rs" {
            continue;
        }
        assert!(!text.contains("Exit::Misuse"), "{name} 里出现了 Exit::Misuse");
    }
}

#[test]
fn the_flag_names_the_code_knows_are_each_named_once() {
    let mut real: Vec<String> = eravon_cli::verbs::all_flag_names()
        .into_iter()
        .map(str::to_string)
        .collect();
    let m = real.len();
    real.sort();
    real.dedup();
    assert_eq!(real.len(), m, "码里的旗名闭表有重名");
}
