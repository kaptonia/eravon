#[test]
fn a_key_file_written_is_never_below_the_backup_floor() {
    if super::alone_in(module_path!(), "a_key_file_written_is_never_below_the_backup_floor") {
        return;
    }
    use app::action::Action;
    super::vault_open();
    let floor = app::keystore::BACKUP_FLOOR;
    assert_eq!(app::keystore::Params::standard(), floor, "the level written is the floor");
    let ctx = eravon_ui::egui::Context::default();
    let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    let dir = std::env::temp_dir().join(format!("eravon-hl-keyfile-{}", std::process::id()));
    super::seated(&mut shell, &dir.join("home"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let pw = "a backup password";
    let answered = app::action::apply_settled(&mut shell, Action::BackupKey { pin: "27618394".into(), password: pw.into(), again: pw.into(), dir: dir.display().to_string() });
    let until = std::time::Instant::now() + std::time::Duration::from_secs(120);
    while shell.tasks.in_flight(app::task::Kind::Keystore) && std::time::Instant::now() < until {
        shell.drain();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    shell.drain();
    let files: Vec<_> = std::fs::read_dir(&dir).expect("the folder").flatten().map(|e| e.path()).collect();
    assert_eq!(files.len(), 1, "one key file landed: {files:?}, answered {answered:?}, {:?}", shell.faults);
    let s = app::keystore::shape(&std::fs::read(&files[0]).expect("the key file")).expect("a key file's shape");
    let (mem, work) = (|n: usize, r: usize| n * r, |n: usize, r: usize, p: usize| n * r * p);
    assert!(mem(s.n, s.r) >= mem(floor.n, floor.r) && work(s.n, s.r, s.p) >= work(floor.n, floor.r, floor.p), "n={} r={} p={} below the floor", s.n, s.r, s.p);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_home_laid_incomplete_is_judged_in_one_place() {
    let makers: Vec<String> = super::shipped()
        .into_iter()
        .filter(|(name, _)| !name.ends_with("fault.rs"))
        .filter(|(_, text)| super::code_only(text).contains("Known::CannotLay"))
        .map(|(name, _)| name)
        .collect();
    assert_eq!(makers, vec!["home.rs".to_string()], "one maker, the home layer (a folder module reads as one text)");
    let lay = super::code_only(&super::read_src_file("home.rs").expect("home.rs"));
    let at = lay.find("pub fn lay(").expect("lay");
    assert!(lay[at..].split("\n}\n").next().unwrap_or_default().contains("Known::CannotLay"), "inside lay");
}

#[test]
fn the_apps_node_channels_are_kept_apart_by_place() {
    for scheme in ["http", "https"] {
        let a = app::chainx::endpoint_at(&format!("{scheme}://node.example/eth/k1")).expect("a channel");
        let b = app::chainx::endpoint_at(&format!("{scheme}://node.example/polygon/k2")).expect("a channel");
        assert_eq!(a.name(), b.name(), "{scheme}: said alike");
        assert_ne!(a.place(), b.place(), "{scheme}: two places");
    }
}

#[test]
fn a_named_landing_never_covers_what_is_there() {
    let d = std::env::temp_dir().join(format!("zk-test-land-numbered-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("dir");
    let land = |dir: &std::path::Path, stem: &str| app::home::land_numbered(dir, stem, "eravon", b"new bytes").map(|p| p.file_name().unwrap().to_string_lossy().to_string());
    assert_eq!(land(&d, "free").ok().as_deref(), Some("free.eravon"), "free: the first name");
    std::fs::write(d.join("file.eravon"), b"old").expect("a file");
    assert_eq!(land(&d, "file").ok().as_deref(), Some("file-2.eravon"), "a file holds the name");
    assert_eq!(std::fs::read(d.join("file.eravon")).expect("kept"), b"old", "untouched");
    std::fs::create_dir_all(d.join("folder.eravon")).expect("a folder");
    assert_eq!(land(&d, "folder").ok().as_deref(), Some("folder-2.eravon"), "a folder holds the name");
    let ro = d.join("ro.eravon");
    std::fs::write(&ro, b"read only").expect("a file");
    let mut p = std::fs::metadata(&ro).expect("meta").permissions();
    p.set_readonly(true);
    std::fs::set_permissions(&ro, p).expect("read-only");
    assert_eq!(land(&d, "ro").ok().as_deref(), Some("ro-2.eravon"), "a read-only file holds the name");
    assert_eq!(std::fs::read(&ro).expect("kept"), b"read only");
    for n in ["many.eravon", "many-2.eravon", "many-3.eravon"] {
        std::fs::write(d.join(n), n.as_bytes()).expect("numbered");
    }
    assert_eq!(land(&d, "many").ok().as_deref(), Some("many-4.eravon"), "several taken: the next free");
    assert_eq!(std::fs::read(d.join("many-2.eravon")).expect("kept"), b"many-2.eravon");
    #[cfg(unix)]
    {
        let shut = d.join("shut");
        std::fs::create_dir_all(&shut).expect("dir");
        let mut p = std::fs::metadata(&shut).expect("meta").permissions();
        p.set_readonly(true);
        std::fs::set_permissions(&shut, p.clone()).expect("read-only folder");
        let refused = land(&shut, "x");
        #[allow(clippy::permissions_set_readonly_false)]
        p.set_readonly(false);
        std::fs::set_permissions(&shut, p).expect("back");
        assert!(refused.is_err(), "a folder that cannot be written is refused: {refused:?}");
        assert_eq!(std::fs::read_dir(&shut).expect("dir").count(), 0, "nothing landed");
    }
    let mut back = std::fs::metadata(&ro).expect("meta").permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    back.set_readonly(false);
    let _ = std::fs::set_permissions(&ro, back);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_backup_never_covers_one_of_the_same_name() {
    if super::alone_in(module_path!(), "a_backup_never_covers_one_of_the_same_name") {
        return;
    }
    super::vault_open();
    let out = std::env::temp_dir().join(format!("zk-test-backup-name-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("dir");
    let now = 1_800_000_000;
    let first = out.join(format!("{}.{}", app::backup::file_stem(now), app::backup::EXT));
    std::fs::write(&first, b"someone's earlier backup").expect("one there");
    let made = app::backup::export(&out, "a long backup password", now).expect("the export lands");
    assert_eq!(made.path, out.join(format!("{}-2.{}", app::backup::file_stem(now), app::backup::EXT)));
    assert_eq!(std::fs::read(&first).expect("kept"), b"someone's earlier backup", "untouched");
    assert_eq!(app::machine::read().expect("settings").backup.map(|b| b.path), Some(made.path.display().to_string()), "the record names the new one");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn a_passcode_handed_is_moved_out_and_handed_once() {
    use app::action::{Action, RestoreHow};
    let typed = || app::secret::Secret::from(String::from("q7w8e9r0"));
    let carrying = vec![
        Action::BackupKey { pin: typed(), password: typed(), again: typed(), dir: String::new() },
        Action::DeleteIdentity { id: String::new(), pin: typed() },
        Action::RevealWords { pin: typed() },
        Action::ExportBackup { pin: typed(), password: typed(), again: typed(), dir: String::new() },
        Action::SetPin { pin: typed(), again: typed() },
        Action::Unlock { pin: typed() },
        Action::ChangePin { old: typed(), pin: typed(), again: typed() },
        Action::Reseal { pin: typed() },
        Action::RecoverWords { words: typed(), pin: typed(), again: typed() },
        Action::RecoverKeystore { path: String::new(), password: typed(), pin: typed(), again: typed() },
        Action::SetPrimary { id: String::new(), pin: typed() },
        Action::RestoreBackup { path: String::new(), password: typed(), how: RestoreHow::Settings { pin: typed() } },
        Action::RestoreBackup { path: String::new(), password: typed(), how: RestoreHow::Locked { pin: typed(), again: typed() } },
    ];
    for mut a in carrying {
        let handed = a.hand_pin().map(|p| p.expose().to_string());
        assert_eq!(handed.as_deref(), Some("q7w8e9r0"), "{}: handed whole", a.name());
        let left = a.pin_asked().or(a.gate_pin()).map(|p| p.expose().len());
        assert_eq!(left, Some(0), "{}: the action's own cell is empty once handed", a.name());
        assert!(a.hand_pin().is_none(), "{}: handing again gives nothing", a.name());
    }
    assert!(Action::Lock.hand_pin().is_none(), "an action with no passcode hands none");
}

#[test]
fn a_node_address_is_said_without_its_key() {
    use app::chainx::{Endpoint, NodeAddr};
    let key = "S3CRETk3y";
    for (written, said) in [
        (format!("https://node.example/v3/{key}"), "https://node.example"),
        (format!("https://node.example:8443/rpc?apikey={key}"), "https://node.example:8443"),
        (format!("https://node.example/rpc#{key}"), "https://node.example"),
        ("http://[::1]:8545/x".to_string(), "http://[::1]:8545"),
        ("https://node.example:443/".to_string(), "https://node.example"),
    ] {
        let a = NodeAddr::new(written.as_str());
        assert_eq!(a.said(), said, "{said}");
        assert_eq!(a.to_string(), said);
        let e = Endpoint::at(1, written.as_str());
        for shown in [format!("{a:?}"), format!("{e:?}")] {
            assert!(!shown.contains(key), "{shown}");
        }
    }
    let user = NodeAddr::new(format!("https://user:{key}@node.example/"));
    assert!(!user.said().contains(key) && !user.said().contains("node.example"), "user part: not an address, said by length: {}", user.said());
    let unread = NodeAddr::new(format!("node.example/{key}"));
    assert_eq!(unread.said(), format!("({} bytes)", format!("node.example/{key}").len()));
    assert_eq!(NodeAddr::new("https://n.example/v3/aaaa").said(), NodeAddr::new("https://n.example/v3/bbbb").said(), "only the key differs: said alike");
}

#[test]
fn a_node_address_gives_its_written_form_to_nobody_outside() {
    let read = |f: &str| super::code_only(&std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../eravon-send/src").join(f)).expect("sender source"));
    let node = read("node.rs");
    assert!(node.contains("pub struct NodeAddr(String);"), "the written form is a private field");
    let at = node.find("impl NodeAddr {").expect("impl");
    let body = &node[at..at + node[at..].find("\n}\n").expect("end")];
    let public: Vec<&str> = body.lines().map(str::trim).filter(|l| l.starts_with("pub fn")).collect();
    assert_eq!(
        public,
        vec![
            "pub fn new(written: impl Into<String>) -> NodeAddr {",
            "pub fn said(&self) -> String {",
            "pub fn place(&self) -> String {",
            "pub fn rest(base: &str, chain: u64) -> NodeAddr {",
            "pub fn kind(&self) -> Kind {",
            "pub fn for_transport(&self) -> &str {"
        ],
        "making one (a JSON-RPC node, a REST row), saying it, which list it came from, and the two written readings"
    );
    assert!(node.contains("pub url: NodeAddr,"), "an endpoint's address is the type, not a string");
    let readers = [".for_transport()", ".spec()", ".place()"];
    let sender_ok = ["node.rs", "agree.rs", "batch.rs"];
    let mut files: Vec<String> = std::fs::read_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../eravon-send/src")).expect("sender sources").flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".rs")).collect();
    files.sort();
    assert!(files.contains(&"node.rs".to_string()), "{files:?}");
    for f in &files {
        let f = f.as_str();
        let code = read(f);
        if readers.iter().any(|r| code.contains(r)) {
            assert!(sender_ok.contains(&f), "{f} reads a node's written form");
        }
    }
    let app_ok = ["archive.rs", "check.rs", "audit.rs", "deploy.rs"];
    let mut stack = vec![std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
    let door = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/app-drive/src");
    if door.is_dir() {
        stack.push(door);
    }
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).expect("a source folder").flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            if p.extension().and_then(|x| x.to_str()) != Some("rs") {
                continue;
            }
            let code = super::code_only(&std::fs::read_to_string(&p).expect("a source file"));
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
            let reads = code.contains("url.for_transport()") || code.contains(".spec()") || code.contains("url.place()");
            if reads {
                assert!(app_ok.contains(&name.as_str()) && !p.to_string_lossy().contains("app-drive"), "{} reads a node's written form", p.display());
            }
        }
    }
}

#[test]
fn every_keyed_kind_is_renamed_by_the_one_naming() {
    if super::alone_in(module_path!(), "every_keyed_kind_is_renamed_by_the_one_naming") {
        return;
    }
    use app::local::{disk_rel, Doc};
    super::vault_open();
    let nk = app::names::key().expect("the names key");
    let id = "ab".repeat(32);
    let sample = |_: Doc| -> Option<String> { None };
    for d in Doc::ALL {
        match (d.keyed(), sample(d)) {
            (true, Some(logical)) => {
                let named = disk_rel(d, &logical, &nk).unwrap_or_else(|f| panic!("{}: {}", d.tag(), f.said()));
                assert!(named != logical && !named.contains(&id), "{}: {named}", d.tag());
                assert!(disk_rel(d, "a/b/c/d/e", &nk).is_err(), "{}: a path not its shape is refused", d.tag());
            }
            (true, None) => panic!("{}: a keyed kind with no sample here", d.tag()),
            (false, _) => assert_eq!(disk_rel(d, "settings/x.json", &nk).ok().as_deref(), Some("settings/x.json"), "{}: kept as is", d.tag()),
        }
    }
}

#[test]
fn which_kinds_are_renamed_is_judged_once() {
    use eravon_seal::envelope::{disk_rel, envelope, unseal, Doc, Expect, Ident, Owner, NONCE};
    let key = [7u8; 32];
    let nk = eravon_seal::names::NameKey::new([9u8; 32]);
    let logical = "settings/probe.json".to_string();
    for d in Doc::ALL {
        let kept_by_naming = disk_rel(d, &logical, &nk).ok().as_deref() == Some(logical.as_str());
        let sealed = envelope(&key, &Ident { owner: Owner::Machine, doc: d, logical: logical.clone() }, &[1u8; NONCE], b"{}").expect("sealed");
        let taken_by_reading = unseal(&key, &Expect { owner: Some(Owner::Machine), doc: d, rel: Some(logical.clone()) }, &sealed, "probe", Some(&nk)).is_ok();
        assert_eq!((kept_by_naming, taken_by_reading), (!d.keyed(), !d.keyed()), "{}: naming keeps / reading takes the unkeyed name", d.tag());
    }
    assert_eq!(Doc::ALL.iter().filter(|d| d.keyed()).count(), 0, "no keyed kind here");
}
