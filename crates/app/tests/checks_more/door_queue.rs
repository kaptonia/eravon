use app::action::{apply, Action, Applied};
use app::shell::Shell;
use eravon_cli::door::{Reply, Request};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

const PIN: &str = "27618394";

struct Bench {
    root: PathBuf,
    machine: PathBuf,
    here: PathBuf,
    address: String,
}

impl Bench {
    fn new(tag: &str) -> (Bench, Shell) {
        super::vault_open();
        let base = if std::env::temp_dir().as_os_str().len() <= 40 { std::env::temp_dir() } else { PathBuf::from("/tmp") };
        let root = base.join(format!("zkq-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let machine = app::home::machine_dir().expect("machine");
        std::fs::create_dir_all(&machine).expect("machine");
        let machine = std::fs::canonicalize(&machine).expect("machine");
        let ctx = eravon_ui::egui::Context::default();
        let mut shell = Shell::boot(eravon_ui::skin::dress(&ctx));
        shell.reread_vault();
        answers!(apply(&mut shell, Action::SetCliDesktop { on: true }), Applied::CliDesktopSet(true));
        let home = root.join("h");
        let address = super::seated(&mut shell, &home).hex();
        opened(&mut shell, &home);
        shell.drain();
        answers!(apply(&mut shell, Action::SetCliAnchor { to: app::machine::CliAnchor::Queue }), Applied::CliAnchorSet(app::machine::CliAnchor::Queue));
        let (here, _) = eravon_os::machine::here().expect("the machine folder");
        settle(&mut shell);
        (Bench { root, machine, here, address }, shell)
    }

    fn place(&self) -> PathBuf {
        eravon_cli::door::place(&self.here, &self.machine)
    }

    fn ask(&self, verb: &str, args: &[(&str, &str)]) -> Request {
        Request { verb: verb.into(), home: self.machine.display().to_string(), args: args.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect() }
    }

    fn signing(&self, wait: Option<&str>) -> Request {
        let mut args = vec![("now", "1800000000"), ("address", self.address.as_str())];
        if let Some(w) = wait {
            args.push(("wait", w));
        }
        self.ask("seller-offer-sign", &args)
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn opened(shell: &mut Shell, at: &Path) {
    super::seated(shell, at);
    let o = apply(shell, Action::OpenHome { root: at.display().to_string() });
    assert!(matches!(o, Applied::Homed { mode: app::lock::Mode::Writer, .. }), "{o:?}");
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

fn settle(shell: &mut Shell) {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
    shell.drain();
    while !shell.tasks.flying().is_empty() && std::time::Instant::now() < until {
        shell.drain();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn until(shell: &mut Shell, done: impl Fn(&Shell) -> bool) {
    let end = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !done(shell) && std::time::Instant::now() < end {
        shell.drain();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

static LET_GO: AtomicBool = AtomicBool::new(false);
fn held_node(_method: &str) -> Option<String> {
    while !LET_GO.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    Some("HTTP 500 Internal Server Error".to_string())
}

#[test]
fn an_approval_while_the_door_is_busy_runs_once_it_is_free() {
    if super::alone_in(module_path!(), "an_approval_while_the_door_is_busy_runs_once_it_is_free") {
        return;
    }
    let (b, mut shell) = Bench::new("busy");
    assert_eq!(served(&mut shell, send(&b.place(), b.signing(None).to_bytes())), Some(Reply::Queued { count: 1 }));
    let node = super::rpc_node(held_node);
    let read = send(&b.place(), b.ask("seller-order-list", &[("now", "1800000000"), ("address", &b.address), ("endpoint", &node)]).to_bytes());
    until(&mut shell, |s| s.tasks.in_flight(app::task::Kind::Door));
    assert!(shell.tasks.in_flight(app::task::Kind::Door), "the read is in flight through the door");
    let told = shell.door_answered;
    app::house::approve::approve(&mut shell);
    shell.drain();
    assert!(app::house::approve::awaiting(&shell).is_none(), "approved: off the card");
    assert_eq!(app::house::approve::queued(&shell), 1, "still queued while the door is busy");
    assert_eq!(shell.door_answered, told, "nothing answered yet");
    LET_GO.store(true, Ordering::SeqCst);
    let r = served(&mut shell, read);
    assert!(r.is_some(), "the read was answered");
    until(&mut shell, |s| app::house::approve::queued(s) == 0 && s.door_doing.is_none() && !s.tasks.in_flight(app::task::Kind::Door));
    assert_eq!(app::house::approve::queued(&shell), 0, "the approved one ran");
    assert_eq!(shell.door_answered, told + 2, "the read, then the approved request through the door's step");
}

#[test]
fn a_queued_request_outlives_the_lock_of_its_home() {
    if super::alone_in(module_path!(), "a_queued_request_outlives_the_lock_of_its_home") {
        return;
    }
    let (b, mut shell) = Bench::new("lock");
    assert_eq!(served(&mut shell, send(&b.place(), b.signing(None).to_bytes())), Some(Reply::Queued { count: 1 }));
    let open = send(&b.place(), b.signing(Some("600")).to_bytes());
    until(&mut shell, |s| app::house::approve::holding(s));
    assert_eq!(app::house::approve::queued(&shell), 2);
    let _ = apply(&mut shell, Action::Lock);
    assert_eq!(served(&mut shell, open), Some(Reply::Closed), "the kept-open answer is told the desktop closed");
    settle(&mut shell);
    assert_eq!(app::house::approve::queued(&shell), 1, "the one already answered stays through the lock");
    answers!(app::action::apply_settled(&mut shell, Action::Unlock { pin: PIN.into() }), Applied::Unlocked);
    settle(&mut shell);
    assert_eq!(app::house::approve::queued(&shell), 1, "unlocked, the same home: still queued");
    assert_eq!(app::house::approve::awaiting(&shell).map(|r| r.verb.as_str()), Some("seller-offer-sign"), "on the card again");
    let told = shell.door_answered;
    app::house::approve::approve(&mut shell);
    settle(&mut shell);
    assert!(app::house::approve::queued(&shell) == 0 && shell.door_answered == told + 1, "approved after the lock, it ran");
}

#[test]
fn a_home_switched_or_let_go_drops_its_queue_and_says_so() {
    if super::alone_in(module_path!(), "a_home_switched_or_let_go_drops_its_queue_and_says_so") {
        return;
    }
    let (b, mut shell) = Bench::new("switch");
    for n in 1..=2u64 {
        assert_eq!(served(&mut shell, send(&b.place(), b.signing(None).to_bytes())), Some(Reply::Queued { count: n }));
    }
    let said = app::house::i18n::filln(app::house::i18n::Key::DoorQueueDropped, &["2"]);
    opened(&mut shell, &b.root.join("h2"));
    settle(&mut shell);
    assert_eq!(app::house::approve::queued(&shell), 0, "another home: its queue went");
    assert!(shell.desk.sh.toasts.iter().any(|t| t.head == said), "the desktop said how many: {:?}", shell.desk.sh.toasts.iter().map(|t| &t.head).collect::<Vec<_>>());
    assert_eq!(served(&mut shell, send(&b.place(), b.signing(None).to_bytes())), Some(Reply::Queued { count: 1 }));
    shell.close_home();
    settle(&mut shell);
    assert_eq!(app::house::approve::queued(&shell), 0, "a home let go: its queue went");
    assert!(shell.desk.sh.toasts.iter().any(|t| t.head == app::house::i18n::filln(app::house::i18n::Key::DoorQueueDropped, &["1"])));
}

#[test]
fn a_locked_desktop_has_no_card_and_its_queue_waits() {
    if super::alone_in(module_path!(), "a_locked_desktop_has_no_card_and_its_queue_waits") {
        return;
    }
    let (b, mut shell) = Bench::new("card");
    assert_eq!(served(&mut shell, send(&b.place(), b.signing(None).to_bytes())), Some(Reply::Queued { count: 1 }));
    assert!(app::house::approve::awaiting(&shell).is_some(), "unlocked: on the card");
    let _ = apply(&mut shell, Action::Lock);
    settle(&mut shell);
    assert!(app::house::approve::awaiting(&shell).is_none(), "locked: no card");
    assert!(app::house::approve::awaiting_flags(&shell).is_empty());
    assert_eq!(app::house::action::held_back(app::house::action::Action::DoorDecline, !shell.unlocked()), Some(app::house::action::Held::Locked), "decline held while locked");
    assert_eq!(app::house::action::held_back(app::house::action::Action::DoorApprove, !shell.unlocked()), Some(app::house::action::Held::Locked), "approve held while locked");
    let told = shell.door_answered;
    app::house::approve::decline(&mut shell);
    app::house::approve::approve(&mut shell);
    settle(&mut shell);
    assert_eq!(app::house::approve::queued(&shell), 1, "the queue did not move");
    assert_eq!(shell.door_answered, told, "nothing answered while locked");
    answers!(app::action::apply_settled(&mut shell, Action::Unlock { pin: PIN.into() }), Applied::Unlocked);
    settle(&mut shell);
    assert_eq!(app::house::approve::awaiting(&shell).map(|r| r.verb.as_str()), Some("seller-offer-sign"), "unlocked: the card is back");
    app::house::approve::approve(&mut shell);
    settle(&mut shell);
    assert!(app::house::approve::queued(&shell) == 0 && shell.door_answered == told + 1, "approved after the unlock, it ran");
}

#[test]
fn an_approved_request_kept_through_the_lock_runs_once_unlocked() {
    if super::alone_in(module_path!(), "an_approved_request_kept_through_the_lock_runs_once_unlocked") {
        return;
    }
    let (b, mut shell) = Bench::new("kept");
    assert_eq!(served(&mut shell, send(&b.place(), b.signing(None).to_bytes())), Some(Reply::Queued { count: 1 }));
    let node = super::rpc_node(held_node);
    let read = send(&b.place(), b.ask("seller-order-list", &[("now", "1800000000"), ("address", &b.address), ("endpoint", &node)]).to_bytes());
    until(&mut shell, |s| s.tasks.in_flight(app::task::Kind::Door));
    app::house::approve::approve(&mut shell);
    shell.drain();
    assert_eq!(app::house::approve::queued(&shell), 1, "approved, waiting for the door");
    let _ = apply(&mut shell, Action::Lock);
    LET_GO.store(true, Ordering::SeqCst);
    let _ = served(&mut shell, read);
    settle(&mut shell);
    assert_eq!(app::house::approve::queued(&shell), 1, "locked: still approved and queued, nothing ran");
    let told = shell.door_answered;
    answers!(app::action::apply_settled(&mut shell, Action::Unlock { pin: PIN.into() }), Applied::Unlocked);
    until(&mut shell, |s| app::house::approve::queued(s) == 0 && s.door_doing.is_none() && !s.tasks.in_flight(app::task::Kind::Door));
    assert_eq!(app::house::approve::queued(&shell), 0, "unlocked: the approved one ran");
    assert_eq!(shell.door_answered, told + 1, "through the door's step, once");
    let verb = b.signing(None).verb.clone();
    let lines = [app::house::i18n::Key::DoorApprovedDone, app::house::i18n::Key::DoorApprovedFailed].map(|k| app::house::i18n::filln(k, &[&verb]));
    settle(&mut shell);
    assert!(shell.desk.sh.toasts.iter().any(|t| lines.contains(&t.head)), "the desktop said how it went: {:?}", shell.desk.sh.toasts.iter().map(|t| &t.head).collect::<Vec<_>>());
}

#[test]
fn an_approved_request_whose_wait_runs_out_gets_the_verbs_answer() {
    if super::alone_in(module_path!(), "an_approved_request_whose_wait_runs_out_gets_the_verbs_answer") {
        return;
    }
    let (b, mut shell) = Bench::new("wait");
    shell.desk.clock = Some(1_800_000_000);
    let node = super::rpc_node(held_node);
    let slow = b.ask("seller-watch", &[("now", "1800000000"), ("address", &b.address), ("endpoint", &node), ("scan", "true"), ("send", "true")]);
    let first = send(&b.place(), slow.to_bytes());
    until(&mut shell, |s| app::house::approve::queued(s) == 1);
    let second = send(&b.place(), b.signing(Some("5")).to_bytes());
    until(&mut shell, |s| app::house::approve::queued(s) == 2);
    assert_eq!(app::house::approve::queued(&shell), 2, "both queued");
    app::house::approve::approve(&mut shell);
    until(&mut shell, |s| s.tasks.in_flight(app::task::Kind::Door));
    assert!(shell.tasks.in_flight(app::task::Kind::Door), "the first runs, held by the node");
    app::house::approve::approve(&mut shell);
    shell.desk.clock = Some(1_800_000_060);
    for _ in 0..20 {
        shell.drain();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(!second.is_finished(), "its seconds passed while approved: no E_QUEUED, the answer stays open");
    LET_GO.store(true, Ordering::SeqCst);
    let _ = served(&mut shell, first);
    let r = served(&mut shell, second);
    assert!(r.is_some() && !matches!(r, Some(Reply::Queued { .. }) | Some(Reply::Closed)), "the verb's own answer: {r:?}");
    assert_eq!(app::house::approve::queued(&shell), 0);
}

fn line_of(shell: &Shell, verb: &str) -> Option<app::house::shell::Toast> {
    let lines = [app::house::i18n::Key::DoorApprovedDone, app::house::i18n::Key::DoorApprovedFailed].map(|k| app::house::i18n::filln(k, &[verb]));
    shell.desk.sh.toasts.iter().find(|t| lines.contains(&t.head)).cloned()
}

#[test]
fn an_approved_request_running_when_the_desktop_locks_is_said_by_its_own_landing() {
    if super::alone_in(module_path!(), "an_approved_request_running_when_the_desktop_locks_is_said_by_its_own_landing") {
        return;
    }
    let (b, mut shell) = Bench::new("flylock");
    let node = super::rpc_node(held_node);
    let verb = "seller-watch";
    let slow = b.ask(verb, &[("now", "1800000000"), ("address", &b.address), ("endpoint", &node), ("scan", "true"), ("send", "true")]);
    assert_eq!(served(&mut shell, send(&b.place(), slow.to_bytes())), Some(Reply::Queued { count: 1 }));
    let ran = shell.tasks.landings(app::task::Kind::Door);
    app::house::approve::approve(&mut shell);
    assert!(shell.tasks.in_flight(app::task::Kind::Door), "approved: it runs, held by the node");
    let _ = apply(&mut shell, Action::Lock);
    shell.drain();
    assert!(!shell.unlocked(), "locked while it runs");
    LET_GO.store(true, Ordering::SeqCst);
    settle(&mut shell);
    assert_eq!(shell.tasks.landings(app::task::Kind::Door), ran + 1, "its task ran on and landed");
    assert_eq!(app::house::approve::queued(&shell), 0, "a verb that ran does not go back to the queue");
    assert!(line_of(&shell, verb).is_none(), "nothing said over the lock screen");
    assert_eq!(shell.desk.door_lines.len(), 1, "its line waits for the desktop");
    let other = app::house::shell::Toast { tone: app::house::seller::Tone::Note, head: "other".into(), next: String::new(), detail: None };
    shell.desk.sh.toast(other);
    answers!(app::action::apply_settled(&mut shell, Action::Unlock { pin: PIN.into() }), Applied::Unlocked);
    settle(&mut shell);
    assert!(line_of(&shell, verb).is_none(), "another toast on screen: the line waits");
    assert_eq!(shell.desk.door_lines.len(), 1);
    shell.desk.sh.toasts.clear();
    shell.drain();
    let line = line_of(&shell, verb).expect("the line is said once the toast went");
    assert_eq!(line.head, app::house::i18n::filln(app::house::i18n::Key::DoorApprovedFailed, &[verb]), "the node failed it: not done");
    let body = line.detail.expect("the verb's own answer in Details");
    let read = eravon_literals::json::read_raw(body.as_bytes()).expect("the verb's answer");
    assert_eq!(read.get("ok"), Some(&eravon_literals::json::Raw::Bool(false)), "its own answer: {body}");
    assert_eq!(shell.tasks.landings(app::task::Kind::Door), ran + 1, "it ran once");
}

#[test]
fn an_approved_request_the_lock_stopped_before_it_ran_runs_once_unlocked() {
    if super::alone_in(module_path!(), "an_approved_request_the_lock_stopped_before_it_ran_runs_once_unlocked") {
        return;
    }
    let (b, mut shell) = Bench::new("prelock");
    assert_eq!(served(&mut shell, send(&b.place(), b.signing(None).to_bytes())), Some(Reply::Queued { count: 1 }));
    app::keybox::lock();
    app::house::approve::approve(&mut shell);
    let _ = apply(&mut shell, Action::Lock);
    settle(&mut shell);
    assert!(!shell.unlocked());
    let ran = shell.tasks.landings(app::task::Kind::Door);
    assert_eq!(app::house::approve::queued(&shell), 1, "it never ran: back in the queue");
    assert!(app::house::approve::awaiting(&shell).is_none(), "locked: no card");
    assert!(shell.desk.door_lines.is_empty(), "nothing to say yet");
    answers!(app::action::apply_settled(&mut shell, Action::Unlock { pin: PIN.into() }), Applied::Unlocked);
    until(&mut shell, |s| app::house::approve::queued(s) == 0 && s.door_doing.is_none() && !s.tasks.in_flight(app::task::Kind::Door));
    settle(&mut shell);
    assert_eq!(app::house::approve::queued(&shell), 0, "unlocked: it ran");
    assert!(app::house::approve::awaiting(&shell).is_none(), "still approved: never back on the card");
    let verb = b.signing(None).verb.clone();
    let line = line_of(&shell, &verb).expect("the desktop said how it went");
    assert_eq!(line.head, app::house::i18n::filln(app::house::i18n::Key::DoorApprovedFailed, &[&verb]), "{line:?}");
    let body = line.detail.clone().unwrap_or_default();
    let read = eravon_literals::json::read_raw(body.as_bytes()).expect("the verb's own answer in Details");
    assert_eq!(read.get("ok"), Some(&eravon_literals::json::Raw::Bool(false)), "{body}");
    assert!(!body.contains("E_VAULT_LOCKED"), "the verb's own answer, not the lock's: {body}");
    for _ in 0..20 {
        shell.drain();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(shell.tasks.landings(app::task::Kind::Door), ran + 1, "it ran exactly once");
}

#[test]
fn a_chain_that_is_the_verbs_own_argument_goes_through_the_door() {
    if super::alone_in(module_path!(), "a_chain_that_is_the_verbs_own_argument_goes_through_the_door") {
        return;
    }
    let (b, mut shell) = Bench::new("chain");
    let mainnet = eravon_literals::constants::NETWORK_MAINNET.standard.chain_id;
    let sepolia = eravon_literals::constants::NETWORK_SEPOLIA.standard.chain_id;
    let here = eravon_literals::constants::current_network().standard.chain_id;
    let other = if here == mainnet { sepolia } else { mainnet };
    let data = b.machine.display().to_string();
    let body = |r: Option<Reply>| match r {
        Some(Reply::Answered { body }) => body,
        other => panic!("not the verb's own answer: {other:?}"),
    };
    let ok = |s: &str| eravon_literals::json::read_raw(s.as_bytes()).ok().is_some_and(|v| v.get("ok") == Some(&eravon_literals::json::Raw::Bool(true)));
    let (other_s, here_s) = (other.to_string(), here.to_string());

    let read = b.ask("seller-candidate-list", &[("now", "1800000000"), ("address", &b.address), ("chain", &other_s)]);
    let said = body(served(&mut shell, send(&b.place(), read.to_bytes())));
    assert!(said.contains(eravon_cli::deskwire::E_OTHER_CHAIN) && said.contains(&format!("seller-settings-network --chain {other}")), "{said}");

    let before = eravon_vault::store::urls_on(&eravon_vault::store::home_settings(&data, &b.address).expect("reads").expect("the home").endpoints, here);
    let node = "http://127.0.0.1:9";
    let nodes = b.ask("seller-settings-endpoints", &[("now", "1800000000"), ("address", &b.address), ("chain", &other_s), ("nodes", node)]);
    let said = body(served(&mut shell, send(&b.place(), nodes.to_bytes())));
    assert!(ok(&said), "{said}");
    settle(&mut shell);
    let disk = eravon_vault::store::home_settings(&data, &b.address).expect("reads").expect("the home");
    assert_eq!(eravon_vault::store::urls_on(&disk.endpoints, other), vec![node.to_string()], "{:?}", disk.endpoints);
    assert_eq!(eravon_vault::store::urls_on(&disk.endpoints, here), before, "this network's nodes stay");

    let switch = b.ask("seller-settings-network", &[("now", "1800000000"), ("address", &b.address), ("chain", &other_s)]);
    let said = body(served(&mut shell, send(&b.place(), switch.to_bytes())));
    assert!(ok(&said) && said.contains(&format!("\"chainId\":{other}")), "{said}");
    settle(&mut shell);
    assert_eq!(eravon_vault::store::network(&data, &b.address).expect("reads"), other, "the home is on the network asked");
    assert_eq!(eravon_literals::constants::current_network().standard.chain_id, other, "the desktop acts on it now");
    assert_eq!(shell.settings.chain_id, Some(other), "the shell's copy took it");

    let back = b.ask("seller-candidate-list", &[("now", "1800000000"), ("address", &b.address), ("chain", &here_s)]);
    assert!(body(served(&mut shell, send(&b.place(), back.to_bytes()))).contains(eravon_cli::deskwire::E_OTHER_CHAIN));
    let on = b.ask("seller-candidate-list", &[("now", "1800000000"), ("address", &b.address), ("chain", &other_s)]);
    assert!(!body(served(&mut shell, send(&b.place(), on.to_bytes()))).contains(eravon_cli::deskwire::E_OTHER_CHAIN));
}
