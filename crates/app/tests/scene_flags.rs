use std::process::Command;

fn args_of(register: &eravon::json::Value, verb: &str) -> Vec<String> {
    let Some(eravon::json::Value::Arr(rows)) = register.member("register") else { panic!("no register") };
    let row = rows.iter().find(|r| matches!(r.member("verb"), Some(eravon::json::Value::Str(v)) if v == verb)).unwrap_or_else(|| panic!("no {verb}"));
    let Some(eravon::json::Value::Arr(args)) = row.member("args") else { panic!("{verb} has no args") };
    args.iter().filter_map(|a| match a.member("name") {
        Some(eravon::json::Value::Str(n)) => Some(n.clone()),
        _ => None,
    }).collect()
}

#[test]
fn drive_and_page_frames_take_one_scene() {
    let out = Command::new(env!("CARGO_BIN_EXE_eravon-app")).arg("schema").output().expect("the binary runs");
    let v = eravon::json::parse(&out.stdout).expect("the schema is JSON");
    let drive = args_of(&v, "drive");
    let frames = args_of(&v, "page-frames");
    assert!(drive.iter().any(|a| a == "--registry"), "{drive:?}");
    assert_eq!(frames.iter().filter(|a| !["--page", "--width", "--look"].contains(&a.as_str())).cloned().collect::<Vec<_>>(), drive, "page-frames takes drive's scene, then its own");

    let root = std::env::temp_dir().join(format!("ev-scene-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (data, home) = (root.join("data"), root.join("home"));
    std::fs::create_dir_all(&data).expect("data root");
    std::fs::create_dir_all(&home).expect("home");
    let script = root.join("s.json");
    std::fs::write(&script, "{\"steps\":[]}").expect("script");
    let reg = format!("0x{}", "5a".repeat(20));
    let out = Command::new(env!("CARGO_BIN_EXE_eravon-app"))
        .args(["page-frames", "--data", &data.display().to_string(), "--script", &script.display().to_string(), "--page", "here", "--now", "1758412800", "--registry", &reg, "--home", &home.display().to_string()])
        .env("HOME", &home)
        .output()
        .expect("the binary runs");
    assert_ne!(out.status.code(), Some(2), "a registry is no misuse of page-frames: {}", String::from_utf8_lossy(&out.stderr));
    let _ = std::fs::remove_dir_all(&root);
}
