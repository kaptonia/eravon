mod common;

use common::{Home, PIN};
use std::path::Path;

const KEY: &str = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";
const EN: [(&str, &str); 1] = [("LANG", "en_US.UTF-8")];

#[test]
fn the_language_is_the_first_variable_set_and_not_empty() {
    let h = Home::bare("gaps-lang");
    let args = ["seller-terms-hash", "--data", "d", "--now", "1", "--file", ""];
    let words = eravon_literals::misuse::EMPTY_PATH;
    for (env, second) in [(vec![("LANG", "en_US.UTF-8"), ("LANGUAGE", "zh_CN:en")], words.en), (vec![("LC_ALL", ""), ("LANG", "zh_CN.UTF-8")], words.zh)] {
        let r = h.run(&args, |c| {
            for k in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
                c.env_remove(k);
            }
            for (k, v) in &env {
                c.env(k, v);
            }
        });
        assert_eq!((r.code, r.err.lines().next(), r.err.lines().nth(1)), (2, Some("E_ARGS --file"), Some(second)), "{env:?}");
    }
}

fn key_file(dir: &Path, name: &str, bytes: &[u8], open: bool) -> String {
    use std::io::Write;
    let p = dir.join(name);
    let mut o = eravon_os::Options::new();
    o.write(true).create_new(true);
    eravon_os::owner_only(&mut o).open(&p).expect("the file").write_all(bytes).expect("its bytes");
    if open {
        eravon_os::open_to_others(&p, false).expect("everyone reads it (0644)");
    }
    p.display().to_string()
}

#[test]
fn a_key_file_that_is_not_a_key_is_said_without_its_contents() {
    let h = Home::new("gaps-hex");
    let bom = format!("\u{feff}{KEY}").into_bytes();
    let before = common::tree(&h.data);
    for (form, bytes, telltale) in [
        ("notHex", b"0xzzqq".to_vec(), b"zzqq".to_vec()),
        ("empty", Vec::new(), Vec::new()),
        ("pastTheCap", vec![b'a'; 4097], vec![b'a'; 8]),
        ("notText", vec![0xff, 0xfe, 0x00, b's', b'e', b'c'], vec![0xff, 0xfe]),
        ("aByteOrderMark", bom.clone(), KEY[2..].as_bytes().to_vec()),
    ] {
        let f = key_file(&h.root, &format!("{form}.key"), &bytes, false);
        let r = h.run(&["buyer-vault-import", "--data", h.data_s(), "--writer", "t", "--now", "1", "--passcode", PIN, "--key-file", &f], |c| {
            c.envs(EN);
        });
        let line = |n: usize| r.err.lines().nth(n).unwrap_or_default().to_string();
        assert_eq!((r.code, r.text.is_empty(), line(0).as_str(), line(1).as_str()), (2, true, "E_KEY --key-file", "not 64 hex digits"), "{form}");
        if !telltale.is_empty() {
            assert!(!r.err.as_bytes().windows(telltale.len()).any(|x| x == telltale.as_slice()), "{form}: the file's contents are never said");
        }
        assert!(!r.err.as_bytes().windows(3).any(|x| x == "\u{feff}".as_bytes()), "{form}: no byte order mark said");
    }
    assert_eq!(common::tree(&h.data), before, "nothing written");
}

#[cfg(unix)]
#[test]
fn a_key_file_others_can_read_is_said_by_its_path_for_init_and_annotate() {
    let h = Home::bare("gaps-open");
    let open = key_file(&h.root, "open.key", KEY.as_bytes(), true);
    for verb in ["seller-vault-import", "buyer-vault-import"] {
        let r = h.run(&[verb, "--data", "d", "--now", "1", "--key-file", &open], |c| {
            c.envs(EN);
        });
        let line = |n: usize| r.err.lines().nth(n).unwrap_or_default().to_string();
        assert_eq!((r.code, r.text.is_empty(), line(0)), (2, true, format!("E_KEY #7 ({} bytes)", open.len())), "{verb}");
        assert!(line(1).starts_with("other accounts can read this key file"), "{verb}: {}", line(1));
        assert!(!r.err.contains(&KEY[2..]), "{verb}: the key is never said");
    }
}

#[cfg(unix)]
#[test]
fn audit_out_into_an_unwritable_folder_says_the_systems_words() {
    let h = Home::new("gaps-out");
    let locked = h.root.join("locked");
    std::fs::create_dir_all(&locked).expect("a folder");
    let shut = |on: bool| {
        let mut p = std::fs::metadata(&locked).expect("the folder").permissions();
        p.set_readonly(on);
        std::fs::set_permissions(&locked, p).expect("its permissions");
    };
    let block = common::terms_block(&h.root);
    shut(true);
    let out = locked.join("terms.md").display().to_string();
    let r = h.run(&["seller-terms-build", "--data", h.data_s(), "--now", "1758412800", "--writer", "t", "--passcode", PIN, "--address", &h.address, "--block", block.to_str().expect("utf-8"), "--out", &out], |_| {});
    shut(false);
    assert_eq!((r.code, r.get("reason")), (4, "E_V_IO".to_string()), "{}", r.text);
    assert!(r.get("detail").contains("locked") && r.get("detail").contains("Permission denied"), "the path and the system's words: {}", r.text);
    assert_eq!(std::fs::read_dir(&locked).expect("the folder").count(), 0, "nothing left there");
}
