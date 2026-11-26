mod common;
use common::*;

#[test]
fn a_record_reads_back_after_a_new_identity_is_made_primary() {
    let (data, mut s) = home("reset");
    let (before, _) = record_dir(&data);
    eravon_vault::accounts::new_primary(&mut s, eravon_store::vault::Seat::Seller, "新主钥", true, None, PIN, NOW).expect("the new primary");
    let _s = restart(&data, PIN);
    reads_back(&data);
    let (after, names) = record_dir(&data);
    assert_ne!(before.file_name(), after.file_name(), "the record's name is renewed with the names key");
    assert!(!before.exists(), "the old name is gone: {}", before.display());
    assert_eq!(names.len(), 1, "one name in the box, nothing staged or left: {names:?}");
    let _ = std::fs::remove_dir_all(&data);
}
