use app::action::{apply, Action, Applied};
use app::fault::Known;

fn files(dir: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push((p.strip_prefix(dir).expect("inside").display().to_string(), std::fs::read(&p).unwrap_or_default()));
            }
        }
    }
    out.sort();
    out
}

fn boot() -> app::shell::Shell {
    let ctx = eravon_ui::egui::Context::default();
    app::shell::Shell::boot(eravon_ui::skin::dress(&ctx))
}

fn which(a: &Applied) -> Option<Known> {
    match a {
        Applied::Trouble(f) => f.which(),
        _ => None,
    }
}

#[test]
fn only_a_store_that_does_not_read_is_set_aside_whole_and_a_held_lock_moves_nothing() {
    let base = std::env::temp_dir().join(format!("evma-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let machine = base.join("m");
    std::fs::create_dir_all(&machine).expect("the machine directory");
    app::places::set(app::places::Places { key_account: app::places::ACCOUNT.to_string(), machine_dir: Some(machine.clone()), user_home: None });
    app::keybox::set_pin("27618394").expect("a store that reads");
    let keybox = machine.join(app::places::keybox_file());

    let before = files(&machine);
    let mut shell = boot();
    assert_eq!(which(&apply(&mut shell, Action::SetMachineAside)), Some(Known::KeyboxNotEmpty));
    assert_eq!(files(&machine), before, "nothing moved over a store that reads");
    drop(shell);

    let good = std::fs::read(&keybox).expect("the store as it reads");
    std::fs::write(&keybox, b"not a key store").expect("the store spoiled");
    let mut shell = boot();
    assert!(matches!(shell.vault, app::shell::Vault::Damaged(_)), "the shell saw it spoiled");
    std::fs::write(&keybox, &good).expect("the store put back");
    let before = files(&machine);
    assert_eq!(which(&apply(&mut shell, Action::SetMachineAside)), Some(Known::KeyboxNotEmpty));
    assert_eq!(files(&machine), before, "nothing moved over a store that reads now");
    assert!(!matches!(shell.vault, app::shell::Vault::Damaged(_)), "the shell takes the reading");
    drop(shell);

    std::fs::write(&keybox, b"not a key store").expect("the store spoiled");
    let home = machine.join("h");
    std::fs::create_dir_all(&home).expect("a home");
    let lock = home.join(app::lock::LOCK_FILE);
    std::fs::write(&lock, b"").expect("its lock file");
    let held = std::fs::File::open(&lock).expect("the lock opens");
    assert!(eravon_os::lock_now(&held), "held here");
    let before = files(&machine);
    let mut shell = boot();
    assert_eq!(which(&apply(&mut shell, Action::SetMachineAside)), Some(Known::MachineHeld));
    assert_eq!(files(&machine), before, "nothing moved while a lock is held");
    drop(held);

    let moved = apply(&mut shell, Action::SetMachineAside);
    let Applied::MachineAside { from, to } = moved else { panic!("set aside: {moved:?}") };
    assert_eq!(std::path::PathBuf::from(&from), machine);
    let to = std::path::PathBuf::from(&to);
    assert_eq!(to.parent(), machine.parent());
    assert!(to.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("m.aside-")), "{}", to.display());
    assert_eq!(files(&to), before, "not a byte deleted");
    assert!(machine.is_dir(), "a new machine directory stands in its place");
    assert!(!machine.join(app::places::keybox_file()).exists(), "with no store: the first-run wizard asks again");

    let again = app::home::aside_name(&machine, "2026-10-26");
    std::fs::create_dir_all(&again).expect("taken");
    assert_ne!(app::home::aside_name(&machine, "2026-10-26"), again);
    let _ = std::fs::remove_dir_all(&base);
}
