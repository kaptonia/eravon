use eravon_cli::door::{put, take, Reply, Request};

#[test]
fn the_desktop_flag_takes_the_verb_through_and_without_it_the_verb_runs_here() {
    let base = if std::env::temp_dir().as_os_str().len() <= 40 { std::env::temp_dir() } else { std::path::PathBuf::from("/tmp") };
    let root = base.join(format!("eravon-cli-through-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (home, data, work) = (root.join("home"), root.join("data"), root.join("work"));
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
    let (name, place) = eravon_cli::verbs::HOME
        .iter()
        .find_map(|(v, row)| row.iter().map(|(f, _)| *f).find(|f| eravon_cli::deskwire::is_place(v, f)).map(|f| (v.to_string(), format!("--{f}"))))
        .expect("a verb with a place");
    let run = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_eravon"))
            .current_dir(&work)
            .env(eravon_os::HOME_VAR, &home)
            .args(args)
            .output()
            .expect("ran")
    };
    let data_s = data.to_str().expect("utf-8");

    let here = run(&[&name, "--data", data_s, "--now", "1"]);
    assert!(here.status.code().is_some_and(|c| c != 4) && !String::from_utf8_lossy(&here.stdout).contains("E_DESKTOP"), "ran here: {}", String::from_utf8_lossy(&here.stdout));

    let at = eravon_cli::door::place(&machine, &std::fs::canonicalize(&data).expect("data"));
    let (door, closer) = eravon_os::door::listen(&at).expect("a door");
    let secret = format!("--{}", eravon_literals::schema::SECRET_FLAGS[0]);
    let refused = run(&[&name, "--desktop", data_s, &secret, "x"]);
    assert_eq!(refused.status.code(), Some(2), "{}", String::from_utf8_lossy(&refused.stderr));
    assert!(refused.stdout.is_empty(), "misuse writes nothing to stdout");
    assert_eq!(String::from_utf8_lossy(&refused.stderr).lines().next(), Some(format!("E_ARGS {secret}").as_str()));
    let missing = run(&[&name, "--desktop", data_s, &place, "out/there"]);
    assert_eq!(missing.status.code(), Some(2), "{}", String::from_utf8_lossy(&missing.stderr));
    assert!(missing.stdout.is_empty(), "misuse writes nothing to stdout");
    assert_eq!(String::from_utf8_lossy(&missing.stderr).lines().next(), Some("E_ARGS --now"));

    let body = r#"{"said":"ran on the desktop","ok":true}"#.to_string();
    let answer = body.clone();
    let desk = std::thread::spawn(move || {
        let mut s = loop {
            if let Some(s) = door.accept().expect("accept") {
                break s;
            }
        };
        let q = Request::of_bytes(&take(&mut s).expect("a frame")).expect("a request");
        put(&mut s, &Reply::Answered { body: answer }.to_bytes()).expect("reply");
        q
    });
    let got = run(&[&name, "--desktop", data_s, "--now", "1", &place, "out/there"]);
    let q = desk.join().expect("the desktop side");
    closer.close();
    assert_eq!(got.status.code(), Some(0), "{}", String::from_utf8_lossy(&got.stderr));
    assert_eq!(got.stdout, br#"{"ok":true,"said":"ran on the desktop"}"#, "the desktop's answer, in the canonical form");
    assert_eq!(q.verb, name, "the verb by its flat name");
    assert_eq!(std::path::Path::new(&q.home), std::fs::canonicalize(&data).expect("data"), "the data root, whole");
    let work_whole = if cfg!(windows) { work.clone() } else { std::fs::canonicalize(&work).expect("work") };
    assert_eq!(std::path::Path::new(q.one(place.trim_start_matches("--")).expect("the place")), work_whole.join("out/there"), "the place made whole");
    let _ = std::fs::remove_dir_all(&root);
}
