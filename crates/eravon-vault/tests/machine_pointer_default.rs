use eravon_os::machine::{default_dir, of, Layer, POINTER};
use eravon_vault::home::{pointer_dir, settle_machine, write_machine_pointer};

fn scratch() -> std::path::PathBuf {
    let base = if std::env::temp_dir().as_os_str().len() <= 40 { std::env::temp_dir() } else { std::path::PathBuf::from("/tmp") };
    let d = base.join(format!("evmp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("scratch");
    d
}

#[test]
fn the_default_place_keeps_no_pointer_and_another_place_keeps_one() {
    let u = scratch().join("u");
    std::fs::create_dir_all(&u).expect("user home");
    assert!(eravon_vault::places::set(eravon_vault::places::Places { key_account: eravon_vault::places::ACCOUNT.to_string(), machine_dir: None, user_home: Some(u.clone()) }));
    let ptr = pointer_dir(&u).join(POINTER);

    let s = settle_machine().expect("settled");
    assert_eq!(s.machine, default_dir(&u));
    assert!(!ptr.exists(), "no pointer for the default place");
    assert_eq!(of(&u).expect("read"), (default_dir(&u), Layer::Default));

    std::fs::create_dir_all(pointer_dir(&u)).expect("pointer dir");
    std::fs::write(&ptr, format!("{}\n", default_dir(&u).display())).expect("an old pointer");
    assert!(write_machine_pointer(&default_dir(&u)).expect("written"), "the old pointer was there to take away");
    assert!(!ptr.exists(), "and is gone");
    assert!(!write_machine_pointer(&default_dir(&u)).expect("written"), "nothing to take away a second time");

    let copy = u.parent().expect("parent").join("copy");
    std::fs::create_dir_all(default_dir(&copy)).expect("the copy");
    assert_eq!(of(&copy).expect("read").0, default_dir(&copy));

    let away = u.parent().expect("parent").join("away");
    std::fs::create_dir_all(&away).expect("away");
    assert!(write_machine_pointer(&away).expect("written"));
    assert!(ptr.is_file(), "a pointer for a place elsewhere");
    assert_eq!(of(&u).expect("read"), (away, Layer::Pointer));
}
