const PW: &str = "a long backup password";

fn machine() -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!("evbs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let machine = base.join("m");
    std::fs::create_dir_all(&machine).expect("the machine directory");
    app::places::set(app::places::Places { key_account: app::places::ACCOUNT.to_string(), machine_dir: Some(machine), user_home: None });
    app::keybox::set_pin("27618394").expect("the vault opens");
    base
}

#[test]
fn the_four_stages_are_told_in_order_and_a_failure_stops_where_it_stood() {
    let base = machine();
    std::fs::create_dir_all(base.join("out")).expect("the folder");
    let mut seen = Vec::new();
    let made = app::backup::export_staged(&base.join("out"), PW, 1_800_000_000, |at| seen.push(at));
    assert!(made.is_ok(), "the export lands: {:?}", made.err());
    assert_eq!(seen, [0, 1, 2, 3], "collect, seal, write, read back, each once, in order");

    let blocked = base.join("blocked");
    std::fs::write(&blocked, b"a file where the folder goes").expect("the blocker");
    let mut seen = Vec::new();
    assert!(app::backup::export_staged(&blocked, PW, 1_800_000_001, |at| seen.push(at)).is_err(), "nothing lands in a file");
    assert_eq!(seen.last(), Some(&2), "stopped at the write: {seen:?}");
    let _ = std::fs::remove_dir_all(&base);
}
