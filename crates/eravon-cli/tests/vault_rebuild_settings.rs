#![cfg(unix)]

mod common;

use common::{Home, Ran, PIN};

const NOW: &str = "1758412800";
const PIN2: &str = "71639482";
const FILE_PIN: &str = "filepass8";

fn key_file(h: &Home) -> String {
    let out = h.root.join("out");
    std::fs::create_dir_all(&out).expect("an out dir");
    let r = h.run(
        &["buyer-vault-backup", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--out", out.to_str().expect("utf-8"), "--passcode", PIN, "--file-passcode", FILE_PIN, "--again", FILE_PIN],
        |_| {},
    );
    assert_eq!(r.code, 0, "the key file is exported: {} {}", r.text, r.err);
    r.get("file")
}

fn rebuild(h: &Home, file: &str) -> Ran {
    h.run(
        &["buyer-vault-rebuild", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--file", file, "--file-passcode", FILE_PIN, "--new-passcode", PIN2, "--again", PIN2],
        |_| {},
    )
}

fn unlocks_with_new_passcode(h: &Home) {
    let r = h.run(&["buyer-vault-unlock", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN2], |_| {});
    assert_eq!(r.code, 0, "the vault opens with the new passcode: {} {}", r.text, r.err);
}

fn settings_file(h: &Home) -> std::path::PathBuf {
    h.data.join(eravon_vault::records::GLOBAL_FILE)
}

#[test]
fn a_lost_settings_file_is_laid_down_again_by_the_rebuild() {
    let h = Home::new("rb-settings-lost");
    let file = key_file(&h);
    let p = settings_file(&h);
    assert!(p.is_file(), "the vault's birth wrote the global settings: {}", p.display());
    std::fs::remove_file(&p).expect("removed");
    let r = rebuild(&h, &file);
    assert_eq!(r.code, 0, "the rebuild answers: {} {}", r.text, r.err);
    assert!(r.text.contains("\"ok\":true") && r.text.contains("\"primary\":true"), "the primary identity is back: {}", r.text);
    assert!(!r.text.contains("\"trouble\""), "nothing is said beside the answer: {}", r.text);
    assert!(p.is_file(), "the settings file is there again: {}", p.display());
    unlocks_with_new_passcode(&h);
}

#[test]
fn a_settings_file_that_does_not_read_is_never_written_over_by_the_rebuild() {
    let h = Home::new("rb-settings-unread");
    let file = key_file(&h);
    let p = settings_file(&h);
    let junk = b"not a settings file".to_vec();
    std::fs::write(&p, &junk).expect("written over");
    let r = rebuild(&h, &file);
    assert_eq!(r.code, 0, "the rebuild itself happens: {} {}", r.text, r.err);
    assert!(r.text.contains("\"ok\":true") && r.text.contains("\"primary\":true"), "the primary identity is back: {}", r.text);
    assert!(r.text.contains("\"trouble\"") && r.text.contains("E_STORE_UNREAD"), "the unread file is said beside the answer: {}", r.text);
    assert_eq!(std::fs::read(&p).expect("read back"), junk, "not a byte of the unread file changed");
    unlocks_with_new_passcode(&h);
}
