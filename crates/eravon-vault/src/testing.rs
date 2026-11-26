use std::path::PathBuf;

fn dir() -> PathBuf {
    std::env::temp_dir().join(format!("eravon-vault-machine-{}", std::process::id()))
}

pub fn fresh() -> String {
    crate::glue::take_turn();
    crate::kdf::set_light(None);
    let ours = dir();
    crate::places::set(crate::places::Places { key_account: crate::places::ACCOUNT.to_string(), machine_dir: Some(ours.clone()), user_home: None });
    let d = crate::home::machine_dir().expect("the test machine directory");
    let tmp = std::fs::canonicalize(std::env::temp_dir()).unwrap_or_else(|_| std::env::temp_dir());
    let under_tmp = d.starts_with(std::env::temp_dir()) || std::fs::canonicalize(d.parent().unwrap_or(&d)).is_ok_and(|p| p.starts_with(&tmp));
    let home_tmp = std::env::var_os("HOME").map(PathBuf::from).is_some_and(|h| d.starts_with(&h) && (h.starts_with(std::env::temp_dir()) || h.starts_with(&tmp)));
    assert!(d == ours || under_tmp || home_tmp, "the test machine directory {} is not under the temporary directory", d.display());
    let data = d.display().to_string();
    crate::keybox::lock();
    eravon_store::writer::let_go(&data);
    crate::tokens::forget(&data);
    crate::local::forget_labels();
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("the test machine directory");
    crate::records::bind(&data).expect("the run binds the test machine directory");
    data
}
