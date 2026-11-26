mod common;
use common::*;

#[test]
fn a_reset_cut_before_commit_leaves_the_record_as_it_was() {
    let (data, s) = home("cut-before");
    let (before, _) = record_dir(&data);
    let id = new_identity(&s);
    assert!(eravon_vault::local::set_cut(eravon_vault::local::Cut::BeforeCommit));
    let e = eravon_vault::rekey::set_primary(&id, PIN2).expect_err("cut");
    assert!(eravon_vault::local::is_cut(&e), "{e:?}");
    let _s = restart(&data, PIN);
    reads_back(&data);
    let (after, names) = record_dir(&data);
    assert_eq!(before, after, "the change never took effect: the old name stays");
    assert_eq!(names.len(), 1, "nothing staged is left: {names:?}");
    let _ = std::fs::remove_dir_all(&data);
}
