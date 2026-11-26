#![cfg(unix)]

mod common;

use common::{Home, PIN};
use std::path::Path;

const KEY: &str = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";

fn run(h: &Home, extra: &[&str]) -> (i32, String, String) {
    let mut a = vec!["buyer-vault-import", "--data", h.data_s(), "--writer", "t", "--now", "1758412800", "--passcode", PIN];
    a.extend_from_slice(extra);
    let r = h.run(&a, |c| {
        c.env("LANG", "en_US.UTF-8");
    });
    (r.code, r.text, r.err)
}

const PATH_AT: usize = 11;

fn key_file(dir: &Path, name: &str, bytes: &[u8]) -> String {
    use std::io::Write;
    let p = dir.join(name);
    let mut o = eravon_os::Options::new();
    o.write(true).create_new(true);
    eravon_os::owner_only(&mut o).open(&p).expect("the file").write_all(bytes).expect("its bytes");
    p.display().to_string()
}

#[test]
fn a_misuse_after_the_key_flag_is_named_where_it_was_typed() {
    let h = Home::new("kf-place");
    let f = key_file(&h.root, "good.key", KEY.as_bytes());
    let (code, out, err) = run(&h, &["--key-file", &f, "stray"]);
    assert_eq!((code, out.is_empty()), (2, true), "misuse, nothing on stdout: {err}");
    assert!(err.starts_with(&format!("E_ARGS #{} (5 bytes)", PATH_AT + 1)), "the word after the key file's path, by its own place: {err}");
    let (code, _, err) = run(&h, &["--key", KEY, "stray"]);
    assert_eq!(code, 2);
    assert!(err.starts_with(&format!("E_ARGS #{} (5 bytes)", PATH_AT + 1)), "the same place given the key itself: {err}");
}
