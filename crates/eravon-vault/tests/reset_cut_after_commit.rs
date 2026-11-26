mod common;
use common::*;

#[test]
fn a_reset_cut_after_commit_is_settled_at_the_next_unlock() {
    let (data, s) = home("cut-after");
    let (before, _) = record_dir(&data);
    let id = new_identity(&s);
    assert!(eravon_vault::local::set_cut(eravon_vault::local::Cut::AfterCommit));
    let e = eravon_vault::rekey::set_primary(&id, PIN2).expect_err("cut");
    assert!(eravon_vault::local::is_cut(&e), "{e:?}");
    let _s = restart(&data, PIN2);
    reads_back(&data);
    let (after, names) = record_dir(&data);
    assert_ne!(before.file_name(), after.file_name(), "the record's name is renewed with the names key");
    assert!(!before.exists(), "the old name is gone: {}", before.display());
    assert_eq!(names.len(), 1, "one name in the box, nothing staged or left: {names:?}");
    let _ = std::fs::remove_dir_all(&data);
}
