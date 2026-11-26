use super::vault_open;
use app::action::{apply, Action, Applied};

fn files(dir: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    fn walk(root: &std::path::Path, at: &std::path::Path, out: &mut Vec<(String, Vec<u8>)>) {
        let Ok(list) = std::fs::read_dir(at) else { return };
        for e in list.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                let rel = p.strip_prefix(root).map(|r| r.display().to_string()).unwrap_or_default();
                out.push((rel, std::fs::read(&p).unwrap_or_default()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).expect("copy root");
    for e in std::fs::read_dir(from).expect("list").flatten() {
        let p = e.path();
        let q = to.join(e.file_name());
        if p.is_dir() {
            copy_tree(&p, &q);
        } else {
            std::fs::copy(&p, &q).expect("copy");
        }
    }
}

#[test]
fn a_home_copied_from_another_machine_is_refused_and_not_written() {
    if super::alone_in(module_path!(), "a_home_copied_from_another_machine_is_refused_and_not_written") {
        return;
    }
    vault_open();
    let base = std::env::temp_dir().join(format!("eravon-fh-label-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (mine, theirs) = (base.join("mine"), base.join("theirs"));
    let ctx = eravon_ui::egui::Context::default();
    {
        let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
        answers!(apply(&mut shell, Action::OpenHome { root: mine.display().to_string() }), Applied::Homed { .. });
    }
    copy_tree(&mine, &theirs);
    let label = app::local::label_path(&theirs);
    let mut b = std::fs::read(&label).expect("a label");
    let last = b.len() - 1;
    b[last] ^= 0x01;
    std::fs::write(&label, &b).expect("their label");
    let _ = std::fs::remove_file(app::lock::mark_path(&app::home::Home::open_or_create(&theirs).expect("a home")));
    app::local::forget_labels();
    let before = files(&theirs);
    let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    match apply(&mut shell, Action::OpenHome { root: theirs.display().to_string() }) {
        Applied::Trouble(f) => assert_eq!(app::local::Unread::of(&f), Some(app::local::Unread::Unopenable), "{f:?}"),
        other => panic!("opened: {other:?}"),
    }
    assert_eq!(files(&theirs), before, "not one byte of their home changed");
    drop(shell);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn a_writer_mark_that_does_not_read_opens_read_only_and_is_kept() {
    if super::alone_in(module_path!(), "a_writer_mark_that_does_not_read_opens_read_only_and_is_kept") {
        return;
    }
    vault_open();
    let dir = std::env::temp_dir().join(format!("eravon-fh-mark-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let home = app::home::Home::open_or_create(&dir).expect("a home");
    let me = app::lock::this_machine().expect("this machine's mark");
    let at = app::lock::mark_path(&home);
    let st = eravon_store::writer::Home::of(&home.dir(app::home::Slot::Settings).display().to_string());
    {
        let l = app::lock::take(&home).expect("taken");
        assert_eq!((l.mode(), eravon_store::lock::mark_read(&st)), (app::lock::Mode::Writer, app::lock::Mark::Machine(me.clone())), "no mark");
    }
    assert_eq!(app::lock::take(&home).expect("taken").mode(), app::lock::Mode::Writer, "this machine's mark");
    let other = "0123456789abcdef0123456789abcdef\n";
    std::fs::write(&at, other).unwrap();
    {
        let l = app::lock::take(&home).expect("taken");
        assert_eq!((l.mode(), l.mark_trouble().is_some()), (app::lock::Mode::OtherMachine, false), "another machine's");
    }
    assert_eq!(std::fs::read_to_string(&at).unwrap(), other);
    for (form, bytes) in [("wrongShape", "not a mark\n".to_string()), ("laterVersion", format!("{{\"form\":\"writer-mark/2\",\"machine\":\"{me}\"}}\n"))] {
        std::fs::write(&at, &bytes).unwrap();
        let l = app::lock::take(&home).expect("taken");
        assert_eq!(l.mode(), app::lock::Mode::OtherMachine, "{form}");
        let f = app::lock::mark_trouble(&l).expect("named");
        assert_eq!(f.which(), Some(app::fault::Known::SettingsShape), "{form}");
        assert!(l.holder().contains(&at.display().to_string()), "{form}: the holder words name the mark: {}", l.holder());
        drop(l);
        assert_eq!(std::fs::read_to_string(&at).unwrap(), bytes, "{form}: not rewritten");
    }
    std::fs::remove_file(&at).unwrap();
    std::fs::create_dir(&at).unwrap();
    {
        let l = app::lock::take(&home).expect("taken");
        assert_eq!((l.mode(), l.mark_trouble().is_some()), (app::lock::Mode::OtherMachine, true), "unreadable");
    }
    assert!(at.is_dir(), "left as it is");
    std::fs::remove_dir(&at).unwrap();
    std::fs::write(&at, "not a mark\n").unwrap();
    let mut l = app::lock::take(&home).expect("taken");
    l.take_over(&st).expect("taken over");
    assert_eq!((l.mode(), l.mark_trouble().is_some(), eravon_store::lock::mark_of(&st)), (app::lock::Mode::Writer, false, Some(me)));
    drop(l);
    let _ = std::fs::remove_dir_all(&dir);
}
