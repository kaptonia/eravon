use eravon_store::{lock, writer};

#[test]
fn this_machines_mark_is_made_once() {
    let m = std::env::temp_dir().join(format!("eravon-lock-test-mark-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&m);
    writer::machine_at(Some(m.clone()));
    let at = m.join(lock::MACHINE_FILE);
    assert!(!at.exists());
    let made = lock::this_machine().expect("made");
    assert!(made.len() == 32 && made.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)), "{made}");
    assert_eq!(std::fs::read(&at).expect("on disk"), format!("{made}\n").into_bytes());
    assert_eq!(lock::this_machine().expect("kept"), made);
    std::fs::write(&at, b"not a mark\n").expect("write");
    assert_eq!(lock::this_machine().err().expect("refused").name(), "E_STORE_MACHINE");
    assert_eq!(std::fs::read(&at).expect("kept"), b"not a mark\n");
    let _ = std::fs::remove_dir_all(&m);
}
