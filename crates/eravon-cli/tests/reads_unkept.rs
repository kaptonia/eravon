#![cfg(unix)]

mod common;

use common::{node, ok, Home, Scene, PIN};

fn list(h: &Home, url: &str) -> common::Ran {
    h.run(&["buyer-list", "--data", h.data_s(), "--writer", "t", "--now", "1758412800", "--chain", "31337", "--endpoint", url, "--passcode", PIN], |_| {})
}

#[test]
fn a_folder_that_cannot_be_written_is_said_in_the_answer() {
    use std::os::unix::fs::PermissionsExt;
    let n = node();
    n.set(Scene { head: ok("\"0x30000\""), logs: Some("[]".into()), ..Scene::default() });
    let h = Home::new("unkept-written");
    let r = list(&h, &n.url);
    assert_eq!(r.code, 0, "{} {}", r.text, r.err);
    assert!(r.out.member("unkept").is_none(), "written as ever: nothing said: {}", r.text);
    let h = Home::new("unkept-readonly");
    let checked = h.machine().join("checked");
    std::fs::create_dir_all(&checked).expect("the folder");
    std::fs::set_permissions(&checked, std::fs::Permissions::from_mode(0o555)).expect("read-only");
    let r = list(&h, &n.url);
    std::fs::set_permissions(&checked, std::fs::Permissions::from_mode(0o755)).expect("writable again");
    println!("ROW read-only code={} unkept={:?}", r.code, r.out.member("unkept"));
    assert_eq!(r.code, 0, "the read stands: {} {}", r.text, r.err);
    let parts = r.out.member("unkept").and_then(eravon::json::Value::as_arr).cloned().unwrap_or_default();
    assert!(!parts.is_empty(), "each part not kept, with why: {}", r.text);
}
