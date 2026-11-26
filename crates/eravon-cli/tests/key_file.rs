#![cfg(unix)]

mod common;

use common::{Home, PIN};
use std::path::Path;

const KEY: &str = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";
const ORDER: &str = "0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141";

fn run(h: &Home, extra: &[&str]) -> (i32, String, String) {
    let mut a = vec!["buyer-vault-import", "--data", h.data_s(), "--writer", "t", "--now", "1758412800", "--passcode", PIN];
    a.extend_from_slice(extra);
    let r = h.run(&a, |c| {
        c.env("LANG", "en_US.UTF-8");
    });
    (r.code, r.text, r.err)
}

const PATH_AT: usize = 11;

#[derive(Clone, Copy)]
enum Reach {
    Owner,
    Group,
    Everyone,
}

fn key_file(dir: &Path, name: &str, bytes: &[u8], reach: Reach) -> String {
    use std::io::Write;
    let p = dir.join(name);
    let mut o = eravon_os::Options::new();
    o.write(true).create_new(true);
    eravon_os::owner_only(&mut o).open(&p).expect("the file").write_all(bytes).expect("its bytes");
    match reach {
        Reach::Owner => {}
        Reach::Group => eravon_os::open_to_others(&p, true).expect("its group reads it"),
        Reach::Everyone => eravon_os::open_to_others(&p, false).expect("everyone reads it"),
    }
    p.display().to_string()
}

#[test]
fn a_key_file_gives_the_key_the_command_line_gives() {
    let (code, by_flag, _) = run(&Home::new("kf-flag"), &["--key", KEY]);
    assert!(code != 0 && by_flag.contains("E_VAULT_PARTY_KEY"), "{by_flag}");
    for (form, bytes) in [
        ("asWritten", KEY.to_string().into_bytes()),
        ("lineEnd", format!("{KEY}\n").into_bytes()),
        ("crlf", format!("{KEY}\r\n").into_bytes()),
        ("whiteSpaceAround", format!("  \t{KEY} \n\n").into_bytes()),
        ("withoutPrefix", KEY.trim_start_matches("0x").as_bytes().to_vec()),
    ] {
        let h = Home::new(&format!("kf-{form}"));
        let f = key_file(&h.root, &format!("{form}.key"), &bytes, Reach::Owner);
        let (code_file, by_file, err) = run(&h, &["--key-file", &f]);
        assert_eq!(code_file, code, "{form}: {err}");
        assert_eq!(by_file, by_flag, "{form}: the same answer as `--key` gives");
    }
}

#[test]
fn every_other_form_is_misuse_by_name() {
    let h = Home::new("kf-misuse");
    let w = &h.root;
    let good = key_file(w, "good.key", KEY.as_bytes(), Reach::Owner);
    let open = key_file(w, "open.key", KEY.as_bytes(), Reach::Everyone);
    let group = key_file(w, "group.key", KEY.as_bytes(), Reach::Group);
    let not_hex = key_file(w, "not-hex.key", b"0xzz", Reach::Owner);
    let empty = key_file(w, "empty.key", b"", Reach::Owner);
    let long = key_file(w, "long.key", &[b'a'; 4097], Reach::Owner);
    let not_text = key_file(w, "not-text.key", &[0xff, 0xfe, 0x00], Reach::Owner);
    let bom = key_file(w, "bom.key", format!("\u{feff}{KEY}").as_bytes(), Reach::Owner);
    let zero = key_file(w, "zero.key", format!("0x{}", "0".repeat(64)).as_bytes(), Reach::Owner);
    let order = key_file(w, "order.key", ORDER.as_bytes(), Reach::Owner);
    std::fs::create_dir_all(w.join("a-directory")).expect("a directory");
    let dir = w.join("a-directory").display().to_string();
    let nowhere = w.join("nowhere.key").display().to_string();
    let before = common::tree(&h.data);
    for (form, extra, head, said) in [
        ("both", vec!["--key", KEY, "--key-file", good.as_str()], "E_ARGS --key --key-file".to_string(), "exactly one of these flags"),
        ("notThere", vec!["--key-file", nowhere.as_str()], format!("E_UNREADABLE #{PATH_AT} ({} bytes)", nowhere.len()), ""),
        ("aDirectory", vec!["--key-file", dir.as_str()], format!("E_UNREADABLE #{PATH_AT} ({} bytes)", dir.len()), ""),
        ("readableByOthers", vec!["--key-file", open.as_str()], format!("E_KEY #{PATH_AT} ({} bytes)", open.len()), "other accounts can read this key file"),
        ("readableByTheGroup", vec!["--key-file", group.as_str()], format!("E_KEY #{PATH_AT} ({} bytes)", group.len()), "other accounts can read this key file"),
        ("notHex", vec!["--key-file", not_hex.as_str()], "E_KEY --key-file".to_string(), ""),
        ("empty", vec!["--key-file", empty.as_str()], "E_KEY --key-file".to_string(), ""),
        ("pastTheCap", vec!["--key-file", long.as_str()], "E_KEY --key-file".to_string(), ""),
        ("notText", vec!["--key-file", not_text.as_str()], "E_KEY --key-file".to_string(), ""),
        ("aByteOrderMark", vec!["--key-file", bom.as_str()], "E_KEY --key-file".to_string(), ""),
        ("zero", vec!["--key-file", zero.as_str()], "E_KEY --key-file".to_string(), "not within the curve's range"),
        ("theOrder", vec!["--key-file", order.as_str()], "E_KEY --key-file".to_string(), "not within the curve's range"),
    ] {
        let (code, out, err) = run(&h, &extra);
        assert_eq!((code, out.is_empty()), (2, true), "{form}: misuse, nothing on stdout: {err}");
        assert!(err.starts_with(&head), "{form}: named: {err}");
        assert!(err.contains(said), "{form}: {err}");
        assert!(!err.contains(&KEY[2..]), "{form}: the key is never echoed");
    }
    assert_eq!(common::tree(&h.data), before, "nothing written by any of them");
}

#[test]
fn every_verb_that_takes_the_key_takes_the_key_file() {
    let h = Home::bare("kf-verbs");
    let open = key_file(&h.root, "open.key", KEY.as_bytes(), Reach::Everyone);
    for verb in ["seller-vault-import", "buyer-vault-import"] {
        let r = h.run(&[verb, "--key-file", &open], |c| {
            c.env("LANG", "en_US.UTF-8");
        });
        assert!(!r.err.contains("not among this verb's flags"), "{verb}: takes --key-file: {}", r.err);
    }
    for verb in ["seller-terms-hash", "buyer-wallet-balance", "buyer-offers"] {
        let r = h.run(&[verb, "--key-file", &open], |c| {
            c.env("LANG", "en_US.UTF-8");
        });
        assert_eq!(r.code, 2, "{verb}");
        assert!(r.err.starts_with("E_ARGS --key-file") && r.err.contains("not among this verb's flags"), "{verb}: a verb without --key does not take it: {}", r.err);
    }
}
