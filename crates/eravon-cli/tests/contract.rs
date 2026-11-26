use std::path::{Path, PathBuf};
use std::process::Command;
use eravon::json::Value;
use eravon_cli::codes::{Contract, Exit, Key, Reason};

const BIN: &str = env!("CARGO_BIN_EXE_eravon");

fn scratch(tag: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let p = std::env::temp_dir().join(format!("eravon-cli-contract-{tag}-{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).expect("建不出草稿地");
    p
}

fn run(dir: &Path, args: &[&str]) -> (i32, Vec<u8>, String) {
    let o = Command::new(BIN)
        .args(args)
        .current_dir(dir)
        .env("LC_ALL", "C")
        .env_remove("LC_MESSAGES")
        .env_remove("LANG")
        .env_remove("LANGUAGE")
        .output()
        .unwrap_or_else(|e| panic!("起不动 eravon {args:?}:在 {}(此处{}):{e}", dir.display(), if dir.is_dir() { "在" } else { "不在" }));
    (o.status.code().unwrap_or(-1), o.stdout, String::from_utf8_lossy(&o.stderr).into_owned())
}

fn member(v: &Value, k: Contract) -> &Value {
    v.member(k.as_str()).unwrap_or_else(|| panic!("答里没有 {}", k.as_str()))
}

fn arr(v: &Value) -> &Vec<Value> {
    match v {
        Value::Arr(xs) => xs,
        _ => panic!("不是数组"),
    }
}

fn text(v: &Value) -> String {
    v.as_str().expect("不是串").to_string()
}

fn names_of(v: &Value) -> Vec<String> {
    match v {
        Value::Obj(ms) => ms.iter().map(|(k, _)| k.clone()).collect(),
        _ => panic!("不是对象"),
    }
}

fn answer() -> (Vec<u8>, Value) {
    let w = scratch("answer");
    let (code, out, _) = run(&w, &["contract"]);
    let _ = std::fs::remove_dir_all(&w);
    assert_eq!(code, 0, "contract 该退 0");
    let v = eravon::json::parse_tests_1_5(&out).expect("contract 的答不是法收的 JSON");
    (out, v)
}

fn listed(v: &Value) -> Vec<(String, Vec<String>)> {
    arr(member(v, Contract::Verbs))
        .iter()
        .map(|x| (text(member(x, Contract::Name)), arr(member(x, Contract::Flags)).iter().map(text).collect()))
        .collect()
}

fn stands_for(v: &Value) -> Vec<(String, String)> {
    arr(member(v, Contract::StandsFor))
        .iter()
        .map(|x| (text(member(x, Contract::Flag)), text(member(x, Contract::For))))
        .collect()
}

#[test]
fn the_contract_answer_is_what_the_tables_say() {
    let (bytes, v) = answer();
    assert_eq!(bytes, eravon::json::canon_bytes(&v), "答不是正典字节");
    assert_ne!(bytes.last(), Some(&b'\n'));
    let mut top = names_of(&v);
    top.sort();
    let mut want: Vec<String> = [Key::Ok.as_str(), Contract::Exits.as_str(), Contract::Reasons.as_str(), Contract::StandsFor.as_str(), Contract::Verbs.as_str()]
        .iter()
        .map(|x| x.to_string())
        .collect();
    want.sort();
    assert_eq!(top, want);
    assert_eq!(v.member(Key::Ok.as_str()), Some(&Value::Bool(true)));
    let exits: Vec<(u64, String)> = arr(member(&v, Contract::Exits))
        .iter()
        .map(|x| {
            let mut ks = names_of(x);
            ks.sort();
            assert_eq!(ks, vec![Contract::Code.as_str().to_string(), Contract::Name.as_str().to_string()]);
            let Value::Int(c) = member(x, Contract::Code) else { panic!("code 不是整数") };
            (*c, text(member(x, Contract::Name)))
        })
        .collect();
    let real: Vec<(u64, String)> = Exit::ALL.iter().map(|e| (e.code() as u64, e.name().to_string())).collect();
    assert_eq!(exits, real);
    let pinned: Vec<(u64, String)> = [(0, "affirmed"), (1, "denied"), (2, "misuse"), (3, "partial"), (4, "unanswered")]
        .iter()
        .map(|(c, n)| (*c, n.to_string()))
        .collect();
    assert_eq!(exits, pinned);
    let reasons: Vec<String> = arr(member(&v, Contract::Reasons)).iter().map(text).collect();
    let real: Vec<String> = Reason::ALL.iter().map(|r| r.as_str().to_string()).chain(eravon_cli::codes_own::desk_names().into_iter().map(str::to_string)).collect();
    assert_eq!(reasons, real);
    let stands = stands_for(&v);
    let real: Vec<(String, String)> = eravon_cli::args::STANDS_FOR.iter().map(|(f, o)| (f.to_string(), o.to_string())).collect();
    assert_eq!(stands, real);
    assert_eq!(stands.last(), Some(&(eravon_cli::args::KEY_FILE.to_string(), "key".to_string())));
    let verbs = listed(&v);
    let names: Vec<&str> = verbs.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, eravon_cli::verbs::VERBS.to_vec());
    assert_eq!(names.len(), 175);
    for (name, fl) in &verbs {
        let row: Vec<String> = eravon_cli::verbs::accepts(name, false).expect("动词没有行").iter().map(|x| x.to_string()).collect();
        assert_eq!(fl, &row, "{name} 的旗单与表不一");
        for f in fl {
            assert!(eravon_cli::verbs::is_known_flag(f), "{name} 的 --{f} 不在旗名闭表里");
        }
        let mut uniq = fl.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(uniq.len(), fl.len(), "{name} 的旗单里有重的");
    }
    assert!(eravon_cli::verbs::accepts("conjure", false).is_none() && eravon_cli::verbs::accepts("conjure", true).is_none(), "不是动词的名字没有行");
    for x in arr(member(&v, Contract::Verbs)) {
        let name = text(member(x, Contract::Name));
        let home: Option<Vec<String>> = x.member(Contract::HomeFlags.as_str()).map(|h| arr(h).iter().map(text).collect());
        let row = eravon_cli::verbs::HOME.iter().find(|(v, _)| *v == name).map(|(_, fl)| fl.iter().map(|(f, _)| f.to_string()).collect::<Vec<_>>());
        assert_eq!(home, row, "{name} 的 homeFlags 与 verbs::HOME 不一");
        let flags: Vec<String> = arr(member(x, Contract::Flags)).iter().map(text).collect();
        assert_eq!(flags.iter().any(|f| f == eravon_cli::verbs::HOME_FLAG), row.is_some(), "{name}: home 在旗单里当且仅当它经桌面做");
    }
    fn walk(v: &Value, seen: &mut Vec<String>) {
        match v {
            Value::Obj(ms) => {
                for (k, x) in ms {
                    seen.push(k.clone());
                    walk(x, seen);
                }
            }
            Value::Arr(xs) => xs.iter().for_each(|x| walk(x, seen)),
            _ => {}
        }
    }
    let mut seen: Vec<String> = Vec::new();
    walk(&v, &mut seen);
    seen.sort();
    seen.dedup();
    let mut table: Vec<String> = Contract::ALL.iter().map(|c| c.as_str().to_string()).collect();
    table.push(Key::Ok.as_str().to_string());
    table.sort();
    assert_eq!(seen, table, "答里的成员名与 codes::Contract 不一");
}

fn flag_subject(f: &str, at: usize) -> String {
    if eravon_cli::verbs::is_known_flag(f) {
        format!("E_ARGS --{f}")
    } else {
        format!("E_ARGS #{at} ({} bytes)", f.len() + 2)
    }
}

#[test]
fn the_contract_takes_no_flag() {
    let w = scratch("noflag");
    for f in eravon_cli::verbs::all_flag_names().into_iter().chain(["nope"]) {
        let (code, out, err) = run(&w, &["contract", &format!("--{f}"), "x"]);
        assert_eq!(code, 2, "contract --{f}");
        assert!(out.is_empty(), "contract --{f}: misuse writes nothing to stdout");
        assert_eq!(err.lines().next(), Some(flag_subject(f, 2).as_str()), "contract --{f}: {err}");
        assert_eq!(err.lines().nth(1), Some("not among this verb's flags"), "contract --{f}: {err}");
    }
    let _ = std::fs::remove_dir_all(&w);
}

#[test]
fn every_verb_takes_exactly_the_flags_the_contract_lists() {
    let (_, v) = answer();
    let stands = stands_for(&v);
    let w = scratch("close");
    let mut asked = 0usize;
    for (verb, fl) in listed(&v) {
        for f in eravon_cli::verbs::all_flag_names().into_iter().chain(["nope", "seed"]) {
            let taken = fl.iter().any(|x| x == f) || stands.iter().any(|(s, o)| s == f && fl.contains(o));
            let (code, out, err) = run(&w, &[&verb, &format!("--{f}"), "x"]);
            let refused = err.lines().next() == Some(flag_subject(f, 2).as_str())
                && err.lines().nth(1) == Some("not among this verb's flags");
            assert_eq!(!refused, taken, "{verb} --{f}: listed {taken}, but close {} it (exit {code}): {err}", if refused { "refused" } else { "took" });
            if refused {
                assert!(out.is_empty(), "{verb} --{f}: misuse writes nothing to stdout");
            }
            asked += 1;
        }
    }
    assert_eq!(asked, 175 * 131);
    let _ = std::fs::remove_dir_all(&w);
}

#[test]
fn with_home_a_verb_takes_exactly_its_home_row() {
    let w = scratch("homeclose");
    let mut asked = 0usize;
    for verb in eravon_cli::verbs::VERBS {
        let home_row = eravon_cli::verbs::HOME.iter().find(|(v, _)| *v == verb).map(|(_, fl)| eravon_cli::doorverbs::names(fl));
        let own = eravon_cli::verbs::accepts(verb, false).expect("行");
        for f in eravon_cli::verbs::all_flag_names().into_iter().filter(|f| *f != eravon_cli::verbs::HOME_FLAG) {
            let (code, out, err) = run(&w, &[verb, "--desktop", "/nowhere/at/all", &format!("--{f}"), "x"]);
            let first = err.lines().next().unwrap_or_default().to_string();
            let second = err.lines().nth(1).unwrap_or_default().to_string();
            asked += 1;
            assert_eq!(code, 2, "{verb} --home --{f}: {err}");
            assert!(out.is_empty(), "{verb} --home --{f}: misuse writes nothing to stdout");
            match &home_row {
                None => assert_eq!((first.as_str(), second.as_str()), ("E_ARGS --desktop", "not among this verb's flags"), "{verb}: --desktop refused"),
                Some(row) if row.contains(&f) => assert_eq!(first, "E_UNREADABLE #3 (15 bytes)", "{verb} --{f} is taken with --home; the home is judged next: {err}"),
                Some(_) => {
                    let said = if own.contains(&f) || eravon_cli::args::STANDS_FOR.iter().any(|(flag, of)| *flag == f && own.contains(of)) { "not taken together with --desktop (the desktop uses its own open vault)" } else { "not among this verb's flags" };
                    assert_eq!((first.as_str(), second.as_str()), (format!("E_ARGS --{f}").as_str(), said), "{verb} --home --{f}: {err}");
                }
            }
        }
    }
    assert_eq!(asked, 175 * 128);
    let _ = std::fs::remove_dir_all(&w);
}

#[test]
fn close_is_fed_only_from_the_per_verb_table() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut calls = 0usize;
    let mut stack = vec![dir.clone()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).expect("列不动 src") {
            let p = e.expect("列不动").path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|x| x.to_str()) == Some("rs") {
                calls += std::fs::read_to_string(&p).expect("读不出").matches(".close(").count();
            }
        }
    }
    assert_eq!(calls, 1, "close 该只在一处被调");
    let verbs = std::fs::read_to_string(dir.join("verbs.rs")).expect("读不出 verbs.rs");
    assert!(
        verbs.contains("if let Some(allowed) = accepts(a.verb(), a.homed()) {\n        a.close(&allowed, required(a.verb(), a.homed()));\n    }"),
        "run 里那一处 close 不再按表"
    );
}

#[test]
fn every_verb_in_the_table_has_its_arm_in_run() {
    let verbs = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/verbs_own.rs")).expect("读不出 verbs_own.rs");
    let at = verbs.find("pub(crate) fn dispatch(a: &Args) -> Answer {").expect("dispatch");
    let body = &verbs[at..at + verbs[at..].find("\n}\n").expect("dispatch 的尾")];
    let missing: Vec<&str> = eravon_cli::verbs::VERBS.iter().copied().filter(|v| !body.contains(&format!("        \"{v}\" => "))).collect();
    assert!(missing.is_empty(), "表里有、run 里没有分派臂:{missing:?}");
    let arms: Vec<&str> = body.lines().filter_map(|l| l.trim().strip_prefix('"')).filter_map(|l| l.split_once("\" => ").map(|(v, _)| v)).collect();
    let stray: Vec<&str> = arms.iter().copied().filter(|v| !eravon_cli::verbs::VERBS.contains(v)).collect();
    assert!(stray.is_empty() && arms.len() == eravon_cli::verbs::VERBS.len(), "run 里有、表里没有行的臂:{stray:?}(臂 {})", arms.len());
}

#[test]
fn the_usage_line_lists_every_verb() {
    let inner = eravon_cli::args::usage()
        .split('<')
        .nth(1)
        .and_then(|x| x.split('>').next())
        .expect("usage 行没有 <…>");
    assert_eq!(inner.split('|').collect::<Vec<_>>(), eravon_cli::verbs::VERBS.to_vec());
}

#[test]
fn each_desktop_refusal_is_answered_by_its_own_name_or_the_command_lines() {
    let mut codes = Vec::new();
    for (code, name) in eravon_cli::codes_own::DESK {
        assert!(!codes.contains(&code), "{code} twice");
        codes.push(code);
        assert!(name == code || name == format!("E_{}", code.to_ascii_uppercase()), "{code}: {name} is neither its own spelling nor its own name");
        let (r, t) = eravon_cli::codes_own::desk_reason(code).expect("the table answers it");
        assert_eq!((r.as_str(), t), (name, (code != name).then_some(code)), "{code}");
        assert!(Reason::ALL.contains(&r), "{code}: a reason of the closed set");
    }
    assert!(eravon_cli::codes_own::desk_names().is_empty(), "every desktop name is a reason of the closed set");
    for (code, name) in [("UNREACHABLE", "E_UNREACHABLE"), ("LOCKED", "E_LOCKED"), ("CREDENTIAL_PASS_MISSING", "E_CREDENTIAL_PASS_MISSING")] {
        assert_eq!(eravon_cli::codes_own::desk_reason(code).map(|(r, _)| r.as_str()), Some(name), "{code}");
    }
    assert_eq!(eravon_cli::codes_own::desk_reason("A_CODE_OF_A_NEWER_DESKTOP"), None, "a code this version does not know");
}
