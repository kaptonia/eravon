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
        let root = std::env::temp_dir().join(format!("ev-cred-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (data, home) = (root.join("data"), root.join("home"));
        std::fs::create_dir_all(&data).expect("data root");
        std::fs::create_dir_all(&home).expect("home");
        Bench { root, data, home }
    }

    fn drive(&self, tag: &str, steps: &str, unlocked: bool) -> eravon::json::Value {
        let script = self.root.join(format!("{tag}.json"));
        std::fs::write(&script, format!("{{\"steps\":[{steps}]}}")).expect("script");
        let mut args = vec!["drive".to_string(), "--data".into(), self.data.display().to_string(), "--script".into(), script.display().to_string(), "--now".into(), NOW.into()];
        if unlocked {
            args.extend(["--passcode".to_string(), PIN.to_string()]);
        }
        let out = Command::new(env!("CARGO_BIN_EXE_eravon-app")).args(&args).env("HOME", &self.home).output().expect("the binary runs");
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        assert_eq!(out.status.code(), Some(0), "{tag}: {text}");
        eravon::json::parse(text.as_bytes()).expect("the answer is JSON")
    }

    fn first_run(&self) {
        let steps = format!(
            r#"{{"do":"first"}},{{"do":"set-pin","d":"{PIN}"}},{{"do":"set-pin","d":"{PIN}"}},{{"do":"set-way","to":"http://127.0.0.1:9"}},{{"do":"way","w":"gen"}},{{"do":"reveal"}},{{"do":"verify"}},{{"do":"answer"}},{{"do":"role","to":"seller"}},{{"do":"payout"}}"#
        );
        self.drive("first", &steps, false);
    }

    fn credential(&self) -> PathBuf {
        eravon_seal::credential::path(&self.data, eravon_vault::places::ACCOUNT)
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn text(v: &eravon::json::Value, path: &[&str]) -> String {
    let mut at = v;
    for k in path {
        at = at.member(k).unwrap_or_else(|| panic!("no {path:?}"));
    }
    at.as_str().unwrap_or_default().to_string()
}

fn list(v: &eravon::json::Value, path: &[&str]) -> Vec<eravon::json::Value> {
    let mut at = v;
    for k in path {
        at = at.member(k).unwrap_or_else(|| panic!("no {path:?}"));
    }
    match at {
        eravon::json::Value::Arr(a) => a.clone(),
        other => panic!("{path:?} is not a list: {other:?}"),
    }
}

fn fault_codes(v: &eravon::json::Value) -> Vec<String> {
    list(v, &["faults"]).iter().map(|f| f.member("code").and_then(|c| c.as_str()).unwrap_or_default().to_string()).collect()
}

fn opens(data: &Path, key: &str) -> Result<(), eravon_seal::Code> {
    eravon_seal::credential::open(data, eravon_vault::places::ACCOUNT, NOW.parse().expect("now"), Some(key)).map(|_| ()).map_err(|t| t.code)
}

#[test]
fn the_drive_door_adds_shows_once_and_deletes_program_keys() {
    let b = Bench::new("keys");
    b.first_run();
    let steps = format!(
        r#"{{"do":"set-cli-alone","secs":"never","pin":"{PIN}"}},{{"do":"set-cli-pass-add","note":"tool a","pin":"{PIN}"}},{{"do":"set-cli-pass-add","note":"tool b","pin":"{PIN}"}},{{"do":"set-cli-pass-read"}}"#
    );
    let v = b.drive("add", &steps, true);
    assert_eq!(text(&v, &["cliPasses", "alone"]), "keyed");
    let rows = list(&v, &["cliPasses", "rows"]);
    assert_eq!(rows.iter().map(|r| r.member("note").and_then(|n| n.as_str()).unwrap_or_default().to_string()).collect::<Vec<_>>(), ["tool a", "tool b"]);
    let shown: Vec<String> = list(&v, &["cliPasses", "shown"]).iter().map(|k| k.as_str().unwrap_or_default().to_string()).collect();
    assert_eq!(shown.len(), 2, "each key made, once");
    for k in &shown {
        assert!(k.starts_with(eravon_seal::credential::PASS_PREFIX) && k.len() == 4 + 64, "a key's form");
    }
    let hex: Vec<&str> = shown.iter().map(|k| &k[4..]).collect();
    let trace = format!("{:?}", v.member("trace"));
    let trace_file = match text(&v, &["tracePath"]) {
        p if p.is_empty() => String::new(),
        p => std::fs::read_to_string(p).expect("the trace file"),
    };
    let on_disk = std::fs::read_to_string(b.credential()).expect("the credential").to_ascii_lowercase();
    for h in &hex {
        assert!(!trace.contains(h) && !trace_file.contains(h) && !on_disk.contains(h), "a key outside the answer's one place");
    }
    assert_eq!(opens(&b.data, &shown[0]), Ok(()));
    assert_eq!(opens(&b.data, &shown[1]), Ok(()));

    let tag_a = rows[0].member("tag").and_then(|t| t.as_str()).expect("a tag").to_string();
    let v = b.drive("drop", &format!(r#"{{"do":"set-cli-pass-drop","tag":"{tag_a}"}},{{"do":"set-cli-pass-read"}}"#), true);
    assert_eq!(list(&v, &["cliPasses", "rows"]).len(), 1, "one row left");
    assert!(list(&v, &["cliPasses", "shown"]).is_empty(), "no key shown again");
    assert_eq!(opens(&b.data, &shown[0]), Err(eravon_seal::Code::CredentialPassUnknown), "the deleted key opens nothing");
    assert_eq!(opens(&b.data, &shown[1]), Ok(()), "the other still opens");
}

#[test]
fn an_earlier_versions_credential_is_removed_at_unlock_said_once() {
    let b = Bench::new("old");
    b.first_run();
    b.drive("on", &format!(r#"{{"do":"set-cli-alone","secs":"never","pin":"{PIN}"}}"#), true);
    let earlier = format!("{{\"expires\":null,\"issued\":1,\"key\":\"0x{}\",\"master\":\"0x{}\"}}", "11".repeat(32), "22".repeat(72));
    std::fs::write(b.credential(), earlier).expect("laid");
    let v = b.drive("unlock", r#"{"do":"set-cli-pass-read"}"#, true);
    assert!(!b.credential().exists(), "removed");
    assert_eq!(fault_codes(&v).iter().filter(|c| *c == "CREDENTIAL_RECLAIMED").count(), 1, "said once: {:?}", fault_codes(&v));
    assert_eq!(text(&v, &["cliPasses", "alone"]), "off", "writing alone off");
}

#[test]
fn every_desktop_refusal_is_replied_as_its_name_exits_run_directly() {
    use eravon_cli::door::Reply;
    use eravon_literals::schema::{class, Exit};
    for k in app::fault::Known::ALL {
        let r = app::door::refused(&app::fault::Fault::known(k, "t".to_string()));
        let want = eravon_cli::codes_own::desk_reason(k.as_str()).map_or(Exit::Unanswered, |(n, _)| class(n.as_str()));
        match (&r, want) {
            (Reply::Partial { code, .. }, Exit::Partial) => assert_eq!(code, k.as_str()),
            (Reply::Refused { code, network, .. }, w) if w != Exit::Partial => {
                assert_eq!(code, k.as_str());
                assert_eq!(*network, w == Exit::Unanswered, "{code}: no answer exactly when its name exits 4");
            }
            other => panic!("{}: {other:?}", k.as_str()),
        }
    }
}
