#![cfg(unix)]

use eravon_cli::door::{put, take, Reply, Request};

#[test]
fn a_relative_path_reaches_the_desktop_whole_by_its_rows_kind() {
    let root = std::env::temp_dir().join(format!("ev-doorp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (home, data, work) = (root.join("u"), root.join("d"), root.join("w"));
    for d in [&home, &data, &work] {
        std::fs::create_dir_all(d).expect("dir");
    }
    // SAFETY: this test is the only one in its process.
    unsafe { std::env::set_var(eravon_os::HOME_VAR, &home) };
    let ptr = eravon_os::machine::pointer_dir(&home);
    std::fs::create_dir_all(&ptr).expect("pointer folder");
    std::fs::write(ptr.join(eravon_os::machine::POINTER), format!("{}\n", root.join("m").display())).expect("pointer");
    let (machine, _) = eravon_os::machine::here().expect("the machine folder");
    std::fs::create_dir_all(&machine).expect("machine folder");
    std::fs::write(work.join("backup.bin"), b"x").expect("a file to read");
    std::fs::write(work.join("nodes.json"), b"{}").expect("a table");
    let data_s = data.to_str().expect("utf-8").to_string();
    let work_real = std::fs::canonicalize(&work).expect("work");

    let cases: Vec<(&str, Vec<&str>, &str, std::path::PathBuf)> = vec![
        ("buyer-backup-restore", vec!["--file", "backup.bin"], "file", work_real.join("backup.bin")),
        ("buyer-export", vec!["--out", "out/statement"], "out", work_real.join("out/statement")),
        ("buyer-settings-zone", vec!["--table", "nodes.json", "--zone", "system"], "table", work_real.join("nodes.json")),
    ];
    for (verb, _, flag, _) in &cases {
        assert!(eravon_cli::doorverbs::kind(verb, flag).is_some_and(eravon_cli::doorverbs::Flag::is_path), "{verb} --{flag} is a path in its row");
    }
    let at = eravon_cli::door::place(&machine, &std::fs::canonicalize(&data).expect("data"));
    let (door, closer) = eravon_os::door::listen(&at).expect("a door");
    let n = cases.len();
    let desk = std::thread::spawn(move || {
        let mut asked = Vec::new();
        for _ in 0..n {
            let mut s = loop {
                if let Some(s) = door.accept().expect("accept") {
                    break s;
                }
            };
            asked.push(Request::of_bytes(&take(&mut s).expect("a frame")).expect("a request"));
            put(&mut s, &Reply::Answered { body: r#"{"ok":true}"#.into() }.to_bytes()).expect("reply");
        }
        asked
    });
    for (verb, given, _, _) in &cases {
        let mut args = vec![verb.to_string(), "--desktop".into(), data_s.clone(), "--now".into(), "1".into()];
        args.extend(given.iter().map(|s| s.to_string()));
        let o = std::process::Command::new(env!("CARGO_BIN_EXE_eravon")).current_dir(&work).env(eravon_os::HOME_VAR, &home).args(&args).output().expect("ran");
        assert_eq!(o.status.code(), Some(0), "{verb}: {} {}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    }
    let asked = desk.join().expect("the desktop side");
    closer.close();
    for ((verb, _, flag, want), q) in cases.iter().zip(&asked) {
        assert_eq!(&q.verb, verb);
        let got = q.one(flag).unwrap_or_else(|| panic!("{verb}: --{flag} went: {:?}", q.args));
        println!("{verb} --{flag}\t{got}");
        assert_eq!(std::path::Path::new(got), want.as_path(), "{verb} --{flag} arrives whole");
    }
    let zone = asked[2].one("zone");
    assert_eq!(zone, Some("system"), "a value goes as typed");
    let _ = std::fs::remove_dir_all(&root);
}
