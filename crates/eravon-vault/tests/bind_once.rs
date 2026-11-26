#[test]
fn the_data_root_binds_once_and_another_root_is_refused_by_name() {
    let base = std::env::temp_dir().join(format!("eravon-bind-once-{}", std::process::id()));
    let (a, b) = (base.join("a"), base.join("b"));
    std::fs::create_dir_all(&a).expect("a");
    std::fs::create_dir_all(&b).expect("b");
    let (a_s, b_s) = (a.display().to_string(), b.display().to_string());
    let first = eravon_vault::records::bind(&a_s).expect("the first root binds");
    assert!(eravon_store::writer::same_place(&first, &a), "bound where asked");
    assert!(eravon_vault::records::bind(&a_s).is_ok(), "the same root again is the same binding");
    let other = eravon_vault::records::bind(&b_s).expect_err("another root is refused");
    assert_eq!(other.which(), Some(eravon_vault::fault::Known::HomeUnreachable), "refused by name");
    assert!(eravon_store::writer::same_place(&eravon_vault::home::machine_dir().expect("still bound"), &a), "the binding stays");
    let _ = std::fs::remove_dir_all(&base);
}
