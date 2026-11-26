use app::action::{apply, Action, Applied};
use app::shell::Shell;
use eravon_cli::door::{Reply, Request};
use std::path::{Path, PathBuf};

const PIN: &str = "27618394";

const VERB: &str = "seller-settings-zone";

struct Bench {
    root: PathBuf,
    machine: PathBuf,
    here: PathBuf,
    home: PathBuf,
    address: String,
}

impl Bench {
    fn new(tag: &str) -> (Bench, Shell) {
        super::vault_open();
        let base = if std::env::temp_dir().as_os_str().len() <= 40 { std::env::temp_dir() } else { PathBuf::from("/tmp") };
        let root = base.join(format!("zkb15-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let home = root.join("h");
        let machine = app::home::machine_dir().expect("machine");
        std::fs::create_dir_all(&machine).expect("machine");
        let machine = std::fs::canonicalize(&machine).expect("machine");
        let mut shell = boot();
        answers!(apply(&mut shell, Action::SetCliDesktop { on: true }), Applied::CliDesktopSet(true));
        let address = super::seated(&mut shell, &home).hex();
        let home = home_opened(&mut shell, &home);
        shell.drain();
        let (here, _) = eravon_os::machine::here().expect("the machine folder");
        (Bench { root, machine, here, home, address }, shell)
    }

    fn place(&self) -> PathBuf {
        eravon_cli::door::place(&self.here, &self.machine)
    }

    fn zone(&self, zone: &str) -> Request {
        self.ask(VERB, &[("now", "1800000000"), ("address", &self.address), ("zone", zone)])
    }

    fn ask(&self, verb: &str, args: &[(&str, &str)]) -> Request {
        Request { verb: verb.into(), home: self.machine.display().to_string(), args: args.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect() }
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn boot() -> Shell {
    let ctx = eravon_ui::egui::Context::default();
    let mut shell = Shell::boot(eravon_ui::skin::dress(&ctx));
    shell.reread_vault();
    shell
}

fn home_opened(shell: &mut Shell, at: &Path) -> PathBuf {
    super::seated(shell, at);
    let opened = apply(shell, Action::OpenHome { root: at.display().to_string() });
    assert!(matches!(opened, Applied::Homed { mode: app::lock::Mode::Writer, .. }), "{opened:?}");
    std::fs::canonicalize(at).expect("home")
}

fn send(place: &Path, bytes: Vec<u8>) -> std::thread::JoinHandle<Option<Vec<u8>>> {
    let place = place.to_path_buf();
    std::thread::spawn(move || {
        let mut s = eravon_os::door::connect(&place).ok()?;
        eravon_cli::door::put(&mut s, &bytes).ok()?;
        eravon_cli::door::take(&mut s).ok()
    })
}

fn served(shell: &mut Shell, h: std::thread::JoinHandle<Option<Vec<u8>>>) -> Option<Reply> {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !h.is_finished() && std::time::Instant::now() < until {
        shell.drain();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    h.join().ok().flatten().and_then(|b| Reply::of_bytes(&b))
}

fn answered(r: &Option<Reply>) -> bool {
    matches!(r, Some(Reply::Answered { .. }))
}

fn files(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().and_then(|n| n.to_str()) != Some(app::lock::LOCK_FILE) {
                out.push((p.clone(), std::fs::read(&p).unwrap_or_default()));
            }
        }
    }
    out.sort();
    out
}

fn settle(shell: &mut Shell) {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
    shell.drain();
    while !shell.tasks.flying().is_empty() && std::time::Instant::now() < until {
        shell.drain();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn the_door_follows_the_lock_and_the_home() {
    if super::alone_in(module_path!(), "the_door_follows_the_lock_and_the_home") {
        return;
    }
    let (b, mut shell) = Bench::new("follows");
    let at = b.place();
    assert!(eravon_os::door::connect(&at).is_ok(), "unlocked and writing: the door is open");
    answers!(apply(&mut shell, Action::Lock), Applied::LockedUp);
    assert!(shell.door.is_none());
    assert_eq!(eravon_os::door::connect(&at).err().map(|e| e.kind()), Some(std::io::ErrorKind::NotFound), "locked: nobody at the door");
    answers!(app::action::apply_settled(&mut shell, Action::Unlock { pin: PIN.into() }), Applied::Unlocked);
    shell.drain();
    assert!(eravon_os::door::connect(&at).is_ok(), "unlocked again: the door is back");
    let other = home_opened(&mut shell, &b.root.join("h2"));
    shell.drain();
    assert!(eravon_os::door::connect(&at).is_ok(), "the data root's door stays open");
    assert_eq!(eravon_os::door::connect(&eravon_cli::door::place(&b.here, &other)).err().map(|e| e.kind()), Some(std::io::ErrorKind::NotFound), "no door at a home's own place");
    apply(&mut shell, Action::Quit);
    shell.drain();
    assert!(shell.door.is_none() && eravon_os::door::connect(&at).is_err(), "quit: shut");
    {
        let (left, _never_closed) = eravon_os::door::listen(&b.place()).expect("left behind");
        drop(left);
        let mut again = boot();
        let _ = apply(&mut again, Action::OpenHome { root: b.home.display().to_string() });
        again.drain();
        assert!(again.door.is_some() && eravon_os::door::connect(&b.place()).is_ok(), "a door left behind gave way");
    }
}

#[test]
fn a_reader_keeps_no_door() {
    if super::alone_in(module_path!(), "a_reader_keeps_no_door") {
        return;
    }
    let (b, shell) = Bench::new("reader");
    let mut second = boot();
    let opened = apply(&mut second, Action::OpenHome { root: b.home.display().to_string() });
    assert!(matches!(opened, Applied::Homed { mode: app::lock::Mode::Reader, .. }), "{opened:?}");
    second.drain();
    assert!(second.door.is_none(), "a reader opens no door");
    assert!(shell.door.is_some());
}

#[test]
fn a_passcode_action_is_left_to_the_desktop() {
    if super::alone_in(module_path!(), "a_passcode_action_is_left_to_the_desktop") {
        return;
    }
    let (b, mut shell) = Bench::new("pin");
    settle(&mut shell);
    let before = files(&b.machine);
    let r = served(&mut shell, send(&b.place(), b.ask("seller-vault-passcode", &[]).to_bytes()));
    assert_eq!(r, Some(Reply::OnDesktop));
    settle(&mut shell);
    assert_eq!(files(&b.machine), before, "nothing written");
    assert!(eravon_cli::schema::passcode_only("vault-passcode") && eravon_cli::verbs::HOME.iter().any(|(v, _)| *v == "seller-vault-passcode"));
}

#[test]
fn what_does_not_read_is_answered_and_the_door_stays_open() {
    if super::alone_in(module_path!(), "what_does_not_read_is_answered_and_the_door_stays_open") {
        return;
    }
    let (b, mut shell) = Bench::new("unread");
    let code = |r: Option<Reply>| match r {
        Some(Reply::Refused { code, .. }) => code,
        other => format!("{other:?}"),
    };
    let unread = app::fault::Known::DoorUnread.as_str();
    assert_eq!(code(served(&mut shell, send(&b.place(), b.ask("scan", &[]).to_bytes()))), unread, "a verb not done through the door");
    assert_eq!(code(served(&mut shell, send(&b.place(), b"{\"form\":\"other\"}".to_vec()))), unread, "not the door's form");
    let place = b.place();
    let over = std::thread::spawn(move || {
        use std::io::Write;
        let mut s = eravon_os::door::connect(&place).ok()?;
        s.write_all(&((eravon_cli::door::CAP + 1) as u32).to_be_bytes()).ok()?;
        eravon_cli::door::take(&mut s).ok()
    });
    assert_eq!(code(served(&mut shell, over)), unread, "over the size");
    drop(eravon_os::door::connect(&b.place()).expect("in"));
    let silent = eravon_os::door::connect(&b.place()).expect("in");
    let started = std::time::Instant::now();
    let r = served(&mut shell, send(&b.place(), b.zone("utc").to_bytes()));
    assert!(answered(&r), "{r:?}");
    assert!(started.elapsed() >= app::door::REQUEST_WAIT - std::time::Duration::from_millis(500), "the one who said nothing was waited for, no longer");
    drop(silent);
}

#[test]
fn two_at_once_are_done_in_turn() {
    if super::alone_in(module_path!(), "two_at_once_are_done_in_turn") {
        return;
    }
    let (b, mut shell) = Bench::new("two");
    settle(&mut shell);
    let from = shell.door_answered;
    let handed = |bytes: Vec<u8>| {
        let mut s = eravon_os::door::connect(&b.place()).expect("in");
        eravon_cli::door::put(&mut s, &bytes).expect("handed in");
        std::thread::spawn(move || eravon_cli::door::take(&mut s).ok())
    };
    let one = handed(b.zone("utc").to_bytes());
    let two = handed(b.zone("system").to_bytes());
    let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while shell.door_answered == from && std::time::Instant::now() < until {
        shell.drain();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(shell.door_answered - from, 1, "one at a time: one answer at the first moment there is any");
    let first = one.join().ok().flatten().and_then(|x| Reply::of_bytes(&x));
    assert!(matches!(&first, Some(Reply::Answered { body }) if body.contains("\"zone\":\"utc\"")), "in arrival order, the first is answered first: {first:?}");
    assert!(!two.is_finished(), "the second waits for its turn");
    let second = served(&mut shell, two);
    assert!(matches!(&second, Some(Reply::Answered { body }) if body.contains("\"zone\":\"system\"")), "then the second: {second:?}");
    assert_eq!(shell.door_answered - from, 2, "each answered once");
}

#[test]
fn a_request_waiting_when_the_desktop_locks_is_told_it_closed() {
    if super::alone_in(module_path!(), "a_request_waiting_when_the_desktop_locks_is_told_it_closed") {
        return;
    }
    let (b, mut shell) = Bench::new("lockwait");
    let (tx, rx) = std::sync::mpsc::channel();
    let tx = std::sync::Mutex::new(tx);
    shell.door_waker = Some(std::sync::Arc::new(move || {
        let _ = tx.lock().map(|t| t.send(()));
    }));
    let _ = apply(&mut shell, Action::Lock);
    answers!(app::action::apply_settled(&mut shell, Action::Unlock { pin: PIN.into() }), Applied::Unlocked);
    shell.drain();
    settle(&mut shell);
    let before = files(&b.machine);
    let h = send(&b.place(), b.zone("utc").to_bytes());
    rx.recv_timeout(std::time::Duration::from_secs(30)).expect("the frame was woken when the request came");
    let _ = apply(&mut shell, Action::Lock);
    let r = h.join().ok().flatten().and_then(|x| Reply::of_bytes(&x));
    assert_eq!(r, Some(Reply::Closed));
    answers!(app::action::apply_settled(&mut shell, Action::Unlock { pin: PIN.into() }), Applied::Unlocked);
    settle(&mut shell);
    assert_eq!(files(&b.machine), before, "nothing of it was written");
}

#[cfg(unix)]
#[test]
fn a_door_that_cannot_open_is_kept_by_name() {
    if super::alone_in(module_path!(), "a_door_that_cannot_open_is_kept_by_name") {
        return;
    }
    let (b, mut shell) = Bench::new("taken");
    let _ = apply(&mut shell, Action::Lock);
    std::fs::write(b.place(), b"not a door").expect("taken");
    answers!(app::action::apply_settled(&mut shell, Action::Unlock { pin: PIN.into() }), Applied::Unlocked);
    for _ in 0..5 {
        shell.drain();
    }
    let kept = shell.door_shut.as_ref().map(|(h, f)| (std::fs::canonicalize(h).unwrap_or_else(|_| h.clone()), f.which()));
    assert_eq!(kept, Some((b.machine.clone(), Some(app::fault::Known::DoorShut))), "kept by name as the shell's reading");
    assert!(shell.faults.iter().all(|f| f.which() != Some(app::fault::Known::DoorShut)), "never among the troubles: nobody pressed anything");
    assert!(shell.door.is_none());
    assert!(matches!(shell.door_why(), Some(app::door::DoorWhy::Refused(_))), "{:?}", shell.door_why());
    assert_eq!(std::fs::read(b.place()).expect("kept"), b"not a door", "what was there is left as it is");
    std::fs::remove_file(b.place()).expect("freed");
    shell.drain();
    assert!(shell.door.is_some() && shell.door_shut.is_none());
    assert_eq!(shell.door_why(), None, "open: nothing to say");
    let _ = apply(&mut shell, Action::Lock);
    std::fs::write(b.place(), b"not a door").expect("taken");
    answers!(app::action::apply_settled(&mut shell, Action::Unlock { pin: PIN.into() }), Applied::Unlocked);
    let _ = apply(&mut shell, Action::SetCliDesktop { on: false });
    shell.drain();
    assert_eq!(shell.door_why(), None, "not wanted: nothing to say");
    std::fs::remove_file(b.place()).expect("freed");
}

#[test]
fn each_flag_of_the_door_table_reaches_its_action() {
    if super::alone_in(module_path!(), "each_flag_of_the_door_table_reaches_its_action") {
        return;
    }
    let (_b, shell) = Bench::new("table");
    for (verb, flags) in eravon_cli::verbs::HOME.iter() {
        let args: Vec<(String, String)> = flags.iter().map(|(f, _)| (f.to_string(), format!("v-{f}-1"))).collect();
        let q = Request { verb: verb.to_string(), home: String::new(), args };
        let app::door::Turn::Verb(taken) = app::door::turn_of(&shell, &q).unwrap_or_else(|f| panic!("{verb}: {f:?}"));
        assert_eq!(taken.verb, *verb, "{verb} is taken as itself");
        let said = format!("{taken:?}");
        for (f, _) in flags.iter() {
            assert!(said.contains(&format!("v-{f}-1")), "{verb} --{f} reaches the verb: {said}");
        }
    }
}

#[test]
fn a_window_not_shown_answers_the_door_and_still_locks() {
    if super::alone_in(module_path!(), "a_window_not_shown_answers_the_door_and_still_locks") {
        return;
    }
    let (b, mut shell) = Bench::new("unseen");
    settle(&mut shell);
    shell.machine.auto_lock = true;
    shell.machine.auto_lock_secs = 900;
    let ctx = eravon_ui::egui::Context::default();
    let mut w = app::window::UnseenProbe::new(shell);
    let ask = |w: &mut app::window::UnseenProbe, q: Request| -> Option<Reply> {
        let h = send(&b.place(), q.to_bytes());
        let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !h.is_finished() && std::time::Instant::now() < until {
            w.pass(&ctx, 0.001);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        h.join().ok().flatten().and_then(|b| Reply::of_bytes(&b))
    };
    let wrote = ask(&mut w, b.zone("utc"));
    assert!(answered(&wrote), "a write through the door, the window not shown: {wrote:?}");
    assert!(w.shell().unlocked());
    w.pass(&ctx, 901.0);
    assert!(!w.shell().unlocked(), "the idle lock falls due in a pass the window does not draw");
}

#[test]
fn the_door_does_not_hold_off_idle_locking() {
    let window = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/window/mod.rs")).expect("window");
    let moves: Vec<&str> = window.lines().filter(|l| l.contains("last_input =")).collect();
    assert_eq!(moves.len(), 1, "{moves:?}");
    let at = window.find(moves[0]).expect("there");
    let before = &window[window[..at].rfind('\n').map(|i| window[..i].rfind('\n').unwrap_or(0)).unwrap_or(0)..at];
    assert!(before.contains("i.events.is_empty()"), "moved only by input events: {before}");
    for f in ["src/door.rs", "src/shell.rs"] {
        let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(f)).expect("source");
        assert!(!text.contains("last_input"), "{f} never moves the idle moment");
    }
}

#[cfg(unix)]
#[test]
fn the_command_line_switch_reads_and_refuses_through_the_action_layer() {
    if super::alone_in(module_path!(), "the_command_line_switch_reads_and_refuses_through_the_action_layer") {
        return;
    }
    let (b, mut shell) = Bench::new("clipath");
    let dir = b.root.join("bin");
    // SAFETY: this test runs alone in its own process.
    unsafe { std::env::set_var(eravon_os::cli_path::ENV_DIR, &dir) };
    let beside = app::action::cli_beside().expect("beside this program");
    assert!(!beside.is_file(), "a test binary has no command line beside it");
    answers!(apply(&mut shell, Action::ReadCliPath), Applied::CliPathRead(eravon_os::cli_path::State::Unsupported(_)));
    assert!(matches!(shell.cli_path, Some(eravon_os::cli_path::State::Unsupported(_))), "the row reads the shell's reading");
    let first = apply(&mut shell, Action::SetCliPath { on: true });
    assert_eq!(first, Applied::Started(app::task::Kind::CliPath));
    let mut said = None;
    let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while said.is_none() && std::time::Instant::now() < until {
        said = shell.drain().into_iter().find(|o| o.kind == app::task::Kind::CliPath).map(|o| o.result.err().and_then(|f| f.which()));
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(said, Some(Some(app::fault::Known::CliPathUnsupported)));
    assert!(!dir.exists(), "nothing made where the switch writes");
    unsafe { std::env::remove_var(eravon_os::cli_path::ENV_DIR) };
}

#[test]
fn a_shell_let_go_of_closes_its_door() {
    if super::alone_in(module_path!(), "a_shell_let_go_of_closes_its_door") {
        return;
    }
    let (b, shell) = Bench::new("dropped");
    assert!(shell.door.is_some());
    drop(shell);
    assert_eq!(eravon_os::door::connect(&b.place()).err().map(|e| e.kind()), Some(std::io::ErrorKind::NotFound), "let go: no door");
    let mut next = boot();
    let _ = apply(&mut next, Action::OpenHome { root: b.home.display().to_string() });
    next.drain();
    assert!(next.door.is_some(), "the next shell opens its own");
    let r = served(&mut next, send(&b.place(), b.zone("utc").to_bytes()));
    assert!(answered(&r), "{r:?}");
}

#[test]
fn a_verb_that_signs_goes_where_the_cell_says() {
    if super::alone_in(module_path!(), "a_verb_that_signs_goes_where_the_cell_says") {
        return;
    }
    let (b, mut shell) = Bench::new("approve");
    settle(&mut shell);
    let wait_held = |shell: &mut Shell| {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while app::house::approve::awaiting(shell).is_none() && std::time::Instant::now() < until {
            shell.drain();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    };
    let ask = b.ask("seller-offer-sign", &[("now", "1800000000"), ("address", &b.address)]);
    assert_eq!(shell.machine.cli_anchor, app::machine::CliAnchor::Send, "absent is send");
    assert!(answered(&served(&mut shell, send(&b.place(), ask.to_bytes()))), "sent automatically, it runs at once");
    assert!(app::house::approve::awaiting(&shell).is_none(), "nothing queued");
    answers!(apply(&mut shell, Action::SetCliAnchor { to: app::machine::CliAnchor::Queue }), Applied::CliAnchorSet(app::machine::CliAnchor::Queue));
    assert_eq!(app::machine::read().expect("reads").cli_anchor, app::machine::CliAnchor::Queue, "written to the machine settings");
    let before = files(&b.home);
    assert_eq!(served(&mut shell, send(&b.place(), ask.to_bytes())), Some(Reply::Queued { count: 1 }), "no --wait: queued at once");
    assert_eq!(app::house::approve::awaiting(&shell).map(|r| r.verb.as_str()), Some("seller-offer-sign"), "it waits on the desktop");
    app::house::approve::decline(&mut shell);
    settle(&mut shell);
    assert!(app::house::approve::awaiting(&shell).is_none() && files(&b.home) == before, "declined: gone, nothing written");
    assert_eq!(served(&mut shell, send(&b.place(), ask.to_bytes())), Some(Reply::Queued { count: 1 }));
    let told = shell.door_answered;
    app::house::approve::approve(&mut shell);
    settle(&mut shell);
    assert!(shell.door_answered == told + 1 && app::house::approve::awaiting(&shell).is_none(), "approved, it ran and was answered");
    let waiting = b.ask("seller-offer-sign", &[("now", "1800000000"), ("address", &b.address), ("wait", "30")]);
    let h = send(&b.place(), waiting.to_bytes());
    wait_held(&mut shell);
    assert!(app::house::approve::holding(&shell), "its answer is kept open");
    app::house::approve::decline(&mut shell);
    match served(&mut shell, h) {
        Some(Reply::Refused { code, tail, zh, en, .. }) => {
            assert_eq!((code.as_str(), tail.as_str()), ("E_APP_DENIED", "seller-offer-sign"));
            let (dz, de) = app::lang::both(app::lang::Key::DoorDeclined);
            assert!(zh.starts_with(dz) && zh.ends_with("seller-offer-sign"), "{zh}");
            assert!(en.starts_with(de) && en.ends_with("seller-offer-sign"), "{en}");
        }
        other => panic!("declined is refused by name: {other:?}"),
    }
    let h = send(&b.place(), waiting.to_bytes());
    wait_held(&mut shell);
    app::house::approve::approve(&mut shell);
    assert!(answered(&served(&mut shell, h)), "approved, it runs and answers as its own");
    shell.desk.clock = Some(2_000_000_000);
    let h = send(&b.place(), waiting.to_bytes());
    wait_held(&mut shell);
    shell.desk.clock = Some(2_000_000_031);
    assert_eq!(served(&mut shell, h), Some(Reply::Queued { count: 1 }), "its seconds ran out: queued");
    assert!(!app::house::approve::holding(&shell) && app::house::approve::queued(&shell) == 1, "left queued, the answer given");
    let r = served(&mut shell, send(&b.place(), b.zone("utc").to_bytes()));
    assert!(answered(&r), "a verb that neither signs nor sends runs at once: {r:?}");
}

#[test]
fn a_save_on_the_window_after_the_door_wrote_the_settings_keeps_what_the_door_wrote() {
    if super::alone_in(module_path!(), "a_save_on_the_window_after_the_door_wrote_the_settings_keeps_what_the_door_wrote") {
        return;
    }
    let (b, mut shell) = Bench::new("n4");
    settle(&mut shell);
    let node = "http://127.0.0.1:9";
    let ask = b.ask("seller-settings-endpoints", &[("now", "1800000000"), ("address", &b.address), ("chain", "1"), ("nodes", node)]);
    assert!(answered(&served(&mut shell, send(&b.place(), ask.to_bytes()))), "the door wrote the nodes");
    settle(&mut shell);
    let on_main = |specs: &[String]| eravon_vault::store::urls_on(specs, 1);
    assert_eq!(on_main(&shell.settings.endpoints), vec![node.to_string()], "the shell's copy took what the door wrote");
    let _ = apply(&mut shell, Action::SetZone { zone: app::when::Zone::Utc });
    let disk = eravon_vault::store::home_settings(&b.machine.display().to_string(), &b.address).expect("reads").expect("the home");
    assert_eq!(on_main(&disk.endpoints), vec![node.to_string()], "the window's save kept the door's node: {:?}", disk.endpoints);
}

