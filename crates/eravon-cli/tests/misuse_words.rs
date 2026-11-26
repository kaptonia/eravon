use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eravon");
const KEY: &str = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";

struct Ran {
    code: i32,
    out: Vec<u8>,
    err: String,
}

fn run(args: &[&str], env: &[(&str, &str)]) -> Ran {
    let mut c = Command::new(BIN);
    c.args(args).current_dir(std::env::temp_dir()).env(eravon_os::HOME_VAR, own_home());
    for k in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
        c.env_remove(k);
    }
    for (k, v) in env {
        c.env(k, v);
    }
    let o = c.output().expect("eravon runs");
    Ran { code: o.status.code().unwrap_or(-1), out: o.stdout, err: String::from_utf8_lossy(&o.stderr).to_string() }
}

fn own_home() -> std::path::PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    std::env::temp_dir().join(format!("eravon-cli-home-{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)))
}

fn first_line(r: &Ran) -> &str {
    r.err.lines().next().unwrap_or("")
}

#[test]
fn the_lines_for_people_follow_the_systems_language() {
    let args = ["seller-terms-hash", "--data", "d", "--now", "1", "--file", ""];
    let (zh_words, en_words) = (eravon_literals::misuse::EMPTY_PATH.zh, eravon_literals::misuse::EMPTY_PATH.en);
    for (env, zh) in [
        (vec![], false),
        (vec![("LANG", "zh_CN.UTF-8")], true),
        (vec![("LANG", "en_US.UTF-8")], false),
        (vec![("LANG", "C")], false),
        (vec![("LANG", "en_US.UTF-8"), ("LC_ALL", "zh_TW.UTF-8")], true),
        (vec![("LC_MESSAGES", "zh_CN"), ("LANG", "en_US")], true),
        (vec![("LANGUAGE", "zh_CN:en")], true),
        (vec![("LANG", "")], false),
    ] {
        let r = run(&args, &env);
        let second = r.err.lines().nth(1).unwrap_or("");
        assert_eq!((r.code, first_line(&r)), (2, "E_ARGS --file"), "{env:?}");
        assert_eq!(second == zh_words, zh, "{env:?}: {second}");
        assert_eq!(second == en_words, !zh, "{env:?}: {second}");
    }
}

#[test]
fn a_word_that_may_be_anything_is_never_echoed() {
    let secret = KEY;
    let r = run(&["contract", secret], &[]);
    assert_eq!((r.code, first_line(&r)), (2, "E_ARGS #2 (66 bytes)"));
    assert!(!r.err.contains(&secret[2..]), "{}", r.err);
    let r = run(&["seller-terms-hash", "--data", "x", secret], &[]);
    assert_eq!(first_line(&r), "E_ARGS #4 (66 bytes)");
    let r = run(&[secret], &[]);
    assert_eq!((r.code, first_line(&r)), (2, "E_ARGS #1 (66 bytes)"));
    assert!(!r.err.contains(&secret[2..]));
    #[cfg(unix)]
    {
        // SAFETY: on unix an OS string's encoded bytes are any bytes at all.
        let not_text = unsafe { std::ffi::OsStr::from_encoded_bytes_unchecked(b"\xffsecret") };
        let o = Command::new(BIN).arg("contract").arg(not_text).env(eravon_os::HOME_VAR, own_home()).output().expect("runs");
        let err = String::from_utf8_lossy(&o.stderr).to_string();
        assert_eq!((o.status.code(), err.lines().next()), (Some(2), Some("E_ARGS #2 (7 bytes)")));
        assert!(!err.contains("secret"), "{err}");
    }
}

#[test]
fn no_misuse_line_echoes_a_word_the_person_typed() {
    let secret = &KEY[2..];
    let pass_eq = format!("--passcode={KEY}");
    let nosuch_eq = format!("--nosuch={KEY}");
    let cases: Vec<(Vec<&str>, String, &str)> = vec![
        (vec!["buyer-wallet-send", &pass_eq], format!("E_ARGS #2 ({} bytes)", pass_eq.len()), eravon_literals::misuse::NEEDS_VALUE.en),
        (vec!["buyer-wallet-send", &pass_eq, "x"], format!("E_ARGS #2 ({} bytes)", pass_eq.len()), eravon_literals::misuse::NOT_THIS_VERBS_FLAG.en),
        (vec!["seller-terms-hash", &nosuch_eq, "x"], format!("E_ARGS #2 ({} bytes)", nosuch_eq.len()), eravon_literals::misuse::NOT_THIS_VERBS_FLAG.en),
        (vec!["seller-terms-hash", "--nosuch", KEY], "E_ARGS #2 (8 bytes)".to_string(), eravon_literals::misuse::NOT_THIS_VERBS_FLAG.en),
        (vec!["contract", "--passcode", KEY], "E_ARGS --passcode".to_string(), eravon_literals::misuse::NOT_THIS_VERBS_FLAG.en),
        (vec!["contract", secret], format!("E_ARGS #2 ({} bytes)", secret.len()), "not in the form --name"),
        (vec![secret], format!("E_ARGS #1 ({} bytes)", secret.len()), "not a verb"),
    ];
    for (args, first, second) in cases {
        let r = run(&args, &[]);
        assert_eq!((r.code, r.out.is_empty()), (2, true), "{args:?}: misuse, nothing on stdout: {}", r.err);
        assert_eq!(first_line(&r), first, "{args:?}: {}", r.err);
        assert!(r.err.lines().nth(1).is_some_and(|l| l.starts_with(second)), "{args:?}: {}", r.err);
        assert!(!r.err.contains(secret), "{args:?}: the word typed is said: {}", r.err);
    }
}
