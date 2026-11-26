use eravon_vault::fault::{classify, known_of_sealed, Known};

fn scratch(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("eravon-not-a-folder-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("scratch");
    d
}

#[test]
fn a_file_where_a_folder_should_be_is_said_by_name_in_both_languages() {
    let d = scratch("classify");
    let home = d.join("home");
    std::fs::write(&home, b"a plain file").expect("the file in the way");
    let missing = std::io::Error::from(std::io::ErrorKind::NotFound);
    let below = home.join("settings").join("home-label.json");
    let f = classify(&missing, &below.display().to_string());
    assert_eq!(f.which(), Some(Known::NotAFolder));
    assert_eq!(Known::NotAFolder.as_str(), "NOT_A_FOLDER");
    assert_eq!(known_of_sealed(eravon_seal::Code::NotAFolder), Some(Known::NotAFolder), "the sealed layer's code reads back as the same member");
    assert_eq!(eravon_seal::Trouble::io(&missing, &below.display().to_string()).code, eravon_seal::Code::NotAFolder, "both ends judge by one table");
    let (zh, en) = eravon_vault::lang::both(Known::NotAFolder.what());
    assert!(zh.contains("文件夹") && en.contains("folder"), "what is wrong, both languages: {zh} / {en}");
    let next = Known::NotAFolder.next().expect("a next step");
    let (zh, en) = eravon_vault::lang::both(next);
    assert!(zh.contains("挪开") && en.contains("Move"), "what to do, both languages: {zh} / {en}");
    assert_eq!(classify(&missing, &d.join("gone").join("x").display().to_string()).which(), Some(Known::FileMissing), "only folders on the way: missing");
    assert_eq!(classify(&missing, "a relative subject").which(), Some(Known::FileMissing));
    let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
    assert_eq!(classify(&denied, &below.display().to_string()).which(), Some(Known::Denied), "denied stays denied");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_store_layer_says_it_as_a_disk_error_never_as_missing() {
    let t = eravon_seal::Trouble::new(eravon_seal::Code::NotAFolder, "/x/home/settings: Not a directory");
    let f = eravon_store::fault::Fault::from(t);
    assert!(matches!(f, eravon_store::fault::Fault::Io(_)), "{f:?}");
}

#[test]
fn the_system_saying_not_a_directory_is_said_by_name() {
    let d = scratch("enotdir");
    let home = d.join("home");
    std::fs::write(&home, b"a plain file").expect("the file in the way");
    let below = home.join("settings.json");
    let e = std::fs::read(&below).expect_err("a path below a plain file does not open");
    #[cfg(unix)]
    assert_eq!(e.kind(), std::io::ErrorKind::NotADirectory, "the system itself says not a directory: {e}");
    let said = std::io::Error::from(std::io::ErrorKind::NotADirectory);
    for err in [&e, &said] {
        assert_eq!(classify(err, &below.display().to_string()).which(), Some(Known::NotAFolder), "{err}");
        assert_eq!(eravon_seal::Trouble::io(err, &below.display().to_string()).code, eravon_seal::Code::NotAFolder, "{err}");
    }
    assert_eq!(classify(&said, "a relative subject").which(), Some(Known::NotAFolder), "by the kind alone, no way to look at");
    let _ = std::fs::remove_dir_all(&d);
}
