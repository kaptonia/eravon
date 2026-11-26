use std::path::{Path, PathBuf};
use std::process::Command;

const PIN: &str = "Qw7Rt2Lp";
const NOW: &str = "1758412800";

struct Bench {
    root: PathBuf,
    data: PathBuf,
    home: PathBuf,
}

impl Bench {
    fn new(tag: &str) -> Bench {
        let root = std::env::temp_dir().join(format!("ev-seat-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (data, home) = (root.join("data"), root.join("home"));
        std::fs::create_dir_all(&data).expect("data root");
        std::fs::create_dir_all(&home).expect("home");
        Bench { root, data, home }
    }

    fn drive(&self, tag: &str, steps: &str, unlocked: bool) -> (Option<i32>, String) {
        let script = self.root.join(format!("{tag}.json"));
        std::fs::write(&script, format!("{{\"steps\":[{steps}]}}")).expect("script");
        let mut args = vec!["drive".to_string(), "--data".into(), self.data.display().to_string(), "--script".into(), script.display().to_string(), "--now".into(), NOW.into()];
        if unlocked {
            args.extend(["--passcode".to_string(), PIN.to_string()]);
        }
        let out = Command::new(env!("CARGO_BIN_EXE_eravon-app")).args(&args).env("HOME", &self.home).output().expect("the binary runs");
        (out.status.code(), String::from_utf8_lossy(&out.stdout).into_owned())
    }

    fn first_run(&self) {
        let steps = format!(
            r#"{{"do":"first"}},{{"do":"set-pin","d":"{PIN}"}},{{"do":"set-pin","d":"{PIN}"}},{{"do":"set-way","to":"http://127.0.0.1:9"}},{{"do":"way","w":"gen"}},{{"do":"reveal"}},{{"do":"verify"}},{{"do":"answer"}},{{"do":"role","to":"seller"}},{{"do":"payout"}}"#
        );
        let (code, out) = self.drive("first", &steps, false);
        assert_eq!(code, Some(0), "the first run: {out}");
    }

    fn identities(&self) -> Vec<(String, bool)> {
        let (code, out) = self.drive("menu", r#"{"do":"id-open"}"#, true);
        assert_eq!(code, Some(0), "the menu opens: {out}");
        rows(&out)
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn rows(answer: &str) -> Vec<(String, bool)> {
    let v = eravon::json::parse(answer.as_bytes()).expect("the answer is JSON");
    let Some(eravon::json::Value::Arr(rows)) = v.member("idMenu").and_then(|m| m.member("rows")) else { panic!("no menu rows: {answer}") };
    rows.iter()
        .map(|r| {
            let address = match r.member("address") {
                Some(eravon::json::Value::Str(s)) => s.clone(),
                _ => panic!("a row without its address: {answer}"),
            };
            (address, matches!(r.member("current"), Some(eravon::json::Value::Bool(true))))
        })
        .collect()
}

fn current(rows: &[(String, bool)]) -> String {
    rows.iter().find(|(_, c)| *c).map(|(a, _)| a.clone()).expect("a current identity")
}

fn seat(answer: &str) -> String {
    let v = eravon::json::parse(answer.as_bytes()).expect("the answer is JSON");
    match v.member("seat") {
        Some(eravon::json::Value::Str(s)) => s.clone(),
        _ => panic!("no seat: {answer}"),
    }
}

fn buyer_locks(data: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(data).expect("the data root lists") {
        let p = e.expect("an entry").path();
        if p.join("seller").is_dir() {
            let settings = p.join("buyer").join("settings");
            std::fs::create_dir_all(&settings).expect("the buyer home's settings folder");
            out.push(settings.join(".lock"));
        }
    }
    out
}

#[test]
fn a_reader_instance_switching_to_a_seat_another_writer_holds_is_refused_by_name_and_moves_nothing() {
    let b = Bench::new("reader");
    b.first_run();
    let before = current(&b.identities());
    let held: Vec<std::fs::File> = buyer_locks(&b.data)
        .iter()
        .map(|p| {
            let f = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(p).expect("the lock file opens");
            f.lock().expect("held");
            f
        })
        .collect();
    assert!(!held.is_empty(), "an identity's home is there");
    let (code, out) = b.drive("to-buyer", r#"{"do":"seat","to":"buyer"}"#, true);
    assert_eq!(code, Some(1), "the switch is refused: {out}");
    assert!(out.contains("READ_ONLY"), "refused by name: {out}");
    let (code, out) = b.drive("after", r#"{"do":"id-open"}"#, true);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(seat(&out), "seller", "the register still names the seller seat: {out}");
    assert_eq!(current(&rows(&out)), before, "the same identity is current");
    assert!(!out.contains("READ_ONLY"), "no home opened read-only at start: {out}");
    drop(held);
}

#[test]
fn a_second_identity_asked_while_a_switch_runs_is_where_the_switch_ends() {
    let b = Bench::new("presses");
    b.first_run();
    for label in ["b", "c"] {
        let steps = format!(r#"{{"do":"page","to":"settings"}},{{"do":"set-new","label":"{label}"}},{{"do":"set-new-reveal"}},{{"do":"set-new-ask"}},{{"do":"set-new-verify"}}"#);
        let (code, out) = b.drive(&format!("new-{label}"), &steps, true);
        assert_eq!(code, Some(0), "identity {label}: {out}");
    }
    let ids = b.identities();
    assert_eq!(ids.len(), 3, "three identities: {ids:?}");
    let start = current(&ids);
    let others: Vec<String> = ids.iter().map(|(a, _)| a.clone()).filter(|a| *a != start).collect();
    let (first, last) = (&others[0], &others[1]);
    let steps = format!(r#"{{"do":"win"}},{{"do":"win-key","to":"{first}","hold":"true"}},{{"do":"win-key","to":"{last}","hold":"true"}},{{"do":"win-settle"}}"#);
    let (code, out) = b.drive("presses", &steps, true);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.matches("在飞").count() >= 2, "both presses went while a switch was in flight: {out}");
    assert_eq!(&current(&b.identities()), last, "the last one asked is current (start {start}, first asked {first})");
}

impl Bench {
    fn wizard_to_role(&self) {
        let steps = format!(
            r#"{{"do":"first"}},{{"do":"set-pin","d":"{PIN}"}},{{"do":"set-pin","d":"{PIN}"}},{{"do":"set-way","to":"http://127.0.0.1:9"}},{{"do":"way","w":"gen"}},{{"do":"reveal"}},{{"do":"verify"}},{{"do":"answer"}}"#
        );
        let (code, out) = self.drive("to-role", &steps, false);
        assert_eq!(code, Some(0), "the wizard to its seat step: {out}");
    }
}

fn wizard_steps(answer: &str) -> (Vec<String>, String) {
    let v = eravon::json::parse(answer.as_bytes()).expect("the answer is JSON");
    let first = v.member("first").unwrap_or_else(|| panic!("no first-run block: {answer}"));
    let done = match first.member("done") {
        Some(eravon::json::Value::Arr(a)) => a.iter().filter_map(|x| if let eravon::json::Value::Str(s) = x { Some(s.clone()) } else { None }).collect(),
        _ => panic!("no steps done: {answer}"),
    };
    let next = match first.member("next") {
        Some(eravon::json::Value::Str(s)) => s.clone(),
        _ => String::new(),
    };
    (done, next)
}

fn fault_codes(answer: &str) -> Vec<String> {
    let v = eravon::json::parse(answer.as_bytes()).expect("the answer is JSON");
    match v.member("faults") {
        Some(eravon::json::Value::Arr(a)) => a.iter().filter_map(|f| match f.member("code") { Some(eravon::json::Value::Str(s)) => Some(s.clone()), _ => None }).collect(),
        _ => Vec::new(),
    }
}

#[test]
fn the_wizard_seat_step_pressed_again_while_its_switch_runs_ends_on_the_seat_last_named() {
    for (second, ends) in [("buyer", "buyer"), ("seller", "seller")] {
        let b = Bench::new(&format!("wiz-{second}"));
        b.wizard_to_role();
        let steps = format!(r#"{{"do":"role","to":"buyer","hold":"true"}},{{"do":"role","to":"{second}"}}"#);
        let (code, out) = b.drive("presses", &steps, true);
        assert_eq!(code, Some(0), "{out}");
        assert_eq!(seat(&out), ends, "the seat last named: {out}");
        let (done, next) = wizard_steps(&out);
        assert!(done.iter().any(|d| d == "role"), "the seat was laid out where the switch landed: {out}");
        assert_eq!(next, if ends == "buyer" { "backup" } else { "tail" }, "{out}");
        assert!(fault_codes(&out).is_empty(), "{out}");
    }
}

#[test]
fn the_wizard_seat_step_whose_switch_fails_says_so_and_lays_nothing_out() {
    let b = Bench::new("wiz-fails");
    b.wizard_to_role();
    let id = std::fs::read_dir(&b.data).expect("the data root lists").flatten().map(|e| e.path()).find(|p| p.join("seller").is_dir()).expect("the identity's folder");
    std::fs::write(id.join("buyer"), b"not a home").expect("a file where the buyer home goes");
    let (code, out) = b.drive("fails", r#"{"do":"role","to":"buyer"}"#, true);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(seat(&out), "seller", "the shell stays where it was: {out}");
    let (done, next) = wizard_steps(&out);
    assert!(!done.iter().any(|d| d == "role" || d == "network"), "nothing laid out: {out}");
    assert_eq!(next, "network", "the step still stands: {out}");
    let named = if cfg!(windows) { "CANNOT_LAY" } else { "NOT_A_FOLDER" };
    assert!(fault_codes(&out).iter().any(|c| c == named), "the refusal is said: {out}");
}

#[test]
fn asking_for_the_identity_in_use_moves_nothing() {
    let b = Bench::new("kept");
    b.first_run();
    let before = current(&b.identities());
    let (code, out) = b.drive("kept", &format!(r#"{{"do":"set-switch","a":"{before}"}}"#), true);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("IdentityKept"), "answered as kept: {out}");
    assert_eq!(seat(&out), "seller", "the same seat: {out}");
    assert_eq!(current(&b.identities()), before, "the same identity is current");
}
