#![cfg(unix)]

use eravon_cli::door::{put, take, Reply};
use eravon_literals::schema::{class, Exit};

#[test]
fn every_refusal_through_the_door_exits_as_its_name_exits_run_directly() {
    let root = std::env::temp_dir().join(format!("ev-doorx-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (home, data) = (root.join("u"), root.join("d"));
    for d in [&home, &data] {
        std::fs::create_dir_all(d).expect("dir");
    }
    // SAFETY: this test is the only one in its process.
    unsafe { std::env::set_var(eravon_os::HOME_VAR, &home) };
    let ptr = eravon_os::machine::pointer_dir(&home);
    std::fs::create_dir_all(&ptr).expect("pointer folder");
    std::fs::write(ptr.join(eravon_os::machine::POINTER), format!("{}\n", root.join("m").display())).expect("pointer");
    let (machine, _) = eravon_os::machine::here().expect("the machine folder");
    std::fs::create_dir_all(&machine).expect("machine folder");
    let data_s = data.to_str().expect("utf-8").to_string();

    let mut rows: Vec<(String, Reply, String, u8)> = Vec::new();
    for k in eravon_vault::fault::Known::ALL {
        let code = k.as_str().to_string();
        let reply = eravon_cli::deskwire::refusal(code.clone(), "evidence".into(), "原因".into(), "why".into());
        let (name, exit) = match eravon_cli::codes_own::desk_reason(&code) {
            Some((r, _)) => (r.as_str().to_string(), class(r.as_str()).code()),
            None => ("E_DESKTOP".to_string(), Exit::Unanswered.code()),
        };
        rows.push((code, reply, name, exit));
    }
    rows.push(("(queued)".into(), Reply::Queued { count: 1 }, "E_QUEUED".into(), class("E_QUEUED").code()));
    rows.push(("(on desktop)".into(), Reply::OnDesktop, "E_ON_DESKTOP".into(), class("E_ON_DESKTOP").code()));
    rows.push(("(closed)".into(), Reply::Closed, "E_DESKTOP".into(), class("E_DESKTOP").code()));

    let at = eravon_cli::door::place(&machine, &std::fs::canonicalize(&data).expect("data"));
    let (door, closer) = eravon_os::door::listen(&at).expect("a door");
    let replies: Vec<Reply> = rows.iter().map(|r| r.1.clone()).collect();
    let desk = std::thread::spawn(move || {
        for r in replies {
            let mut s = loop {
                if let Some(s) = door.accept().expect("accept") {
                    break s;
                }
            };
            let _ = take(&mut s).expect("a frame");
            put(&mut s, &r.to_bytes()).expect("reply");
        }
    });
    let mut wrong = Vec::new();
    for (code, _, name, exit) in &rows {
        let o = std::process::Command::new(env!("CARGO_BIN_EXE_eravon"))
            .env(eravon_os::HOME_VAR, &home)
            .args(["buyer-settings-zone", "--desktop", &data_s, "--now", "1", "--zone", "system"])
            .output()
            .expect("ran");
        let got = eravon::json::parse(&o.stdout).ok().and_then(|v| v.member("reason").and_then(|x| x.as_str()).map(str::to_string)).unwrap_or_default();
        let got_exit = o.status.code().unwrap_or(-1);
        println!("{code}\t{name}\t{exit}");
        if (got.as_str(), got_exit) != (name.as_str(), i32::from(*exit)) {
            wrong.push(format!("{code}: {got} exit {got_exit}, want {name} exit {exit}"));
        }
    }
    desk.join().expect("the desktop side");
    closer.close();
    let _ = std::fs::remove_dir_all(&root);
    assert!(wrong.is_empty(), "through the door against run directly:\n{}", wrong.join("\n"));
    let exit_of = |code: &str| rows.iter().find(|r| r.0 == code).map(|r| r.3);
    assert_eq!(exit_of("OUTCOME_LOST").or(exit_of("E_OUTCOME_LOST")), Some(4));
    assert_eq!(exit_of("E_V_UNCONFIRMED"), Some(3));
}
