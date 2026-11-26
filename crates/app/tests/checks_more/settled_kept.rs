#[test]
fn a_part_the_desktop_cannot_write_for_the_store_locked_is_told_apart() {
    if super::alone_in(module_path!(), "a_part_the_desktop_cannot_write_for_the_store_locked_is_told_apart") {
        return;
    }
    super::vault_open();
    let keep = app::chainx::settled_keeper().expect("a keeper");
    assert_eq!(keep.save("notes", b"{}"), Ok(()), "written while open");
    eravon_seal::keybox::lock();
    assert_eq!(keep.save("notes", b"{}"), Err(eravon_chain::settled::Unkept::Locked), "the lock's write: told apart, never said");
}

#[test]
fn a_part_read_under_a_key_since_replaced_is_dropped_and_said_by_nobody() {
    if super::alone_in(module_path!(), "a_part_read_under_a_key_since_replaced_is_dropped_and_said_by_nobody") {
        return;
    }
    super::vault_open();
    let keep = app::chainx::settled_keeper().expect("a keeper");
    assert_eq!(keep.save("notes", b"{}"), Ok(()), "written under the key it was read under");
    assert!(keep.load("notes").is_some());
    let mut slots: Vec<(String, Vec<u8>)> = Vec::new();
    for a in app::keybox::accounts().expect("the accounts") {
        if let Some(b) = app::keybox::get(&a).expect("a slot") {
            slots.push((a, b));
        }
    }
    let nb = app::keybox::build_new(app::keybox::PinFor::New("48261937"), None, &slots).expect("a new book");
    app::keybox::stage_new(&nb).expect("staged");
    app::keybox::commit_new(nb).expect("the key replaced");
    let _ = eravon_chain::settled::unkept();
    assert_eq!(keep.save("notes", b"{}"), Err(eravon_chain::settled::Unkept::KeyChanged), "read under the key replaced: dropped");
    assert_eq!(keep.load("notes"), None, "nor read back under the new key");
    assert!(eravon_chain::settled::unkept().is_empty(), "said by nobody");
}
