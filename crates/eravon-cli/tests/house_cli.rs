use std::path::{Path, PathBuf};

struct Scratch(PathBuf);

impl std::ops::Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("eravon-cli-house-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).expect("建不出草稿地");
    Scratch(p)
}

#[test]
fn an_output_name_already_there_is_refused_before_anything_is_written() {
    let w = scratch("free");
    std::fs::write(w.join("taken"), b"x").expect("a file");
    std::fs::write(w.join("same"), b"these bytes").expect("a file");
    std::fs::create_dir_all(w.join("folder")).expect("a folder");
    #[cfg(unix)]
    std::os::unix::fs::symlink(w.join("nowhere"), w.join("dangling")).expect("a link");
    #[cfg(unix)]
    std::os::unix::fs::symlink(w.join("same"), w.join("to-same")).expect("a link");
    let at = |n: &str| w.join(n).display().to_string();
    let mine: &[u8] = b"these bytes";
    for taken in ["taken", "folder", "dangling", "to-same"] {
        if cfg!(not(unix)) && (taken == "dangling" || taken == "to-same") {
            continue;
        }
        let said = (String::from("E_V_EXISTS"), format!("{} 已经有了,一个字节也没盖", at(taken)));
        for names in [[at(taken), at("free-one")], [at("free-one"), at(taken)]] {
            let f = eravon_cli::write::free(&[(names[0].as_str(), mine), (names[1].as_str(), mine)]).expect_err(taken);
            assert_eq!((f.name().to_string(), f.detail()), said, "{taken}, first or last of two");
        }
        let f = eravon_cli::write::land(&at(taken), mine).expect_err(taken);
        assert_eq!((f.name().to_string(), f.detail()), said, "{taken}: land says the same");
    }
    assert!(eravon_cli::write::free(&[(at("free-one").as_str(), mine), (at("same").as_str(), mine)]).is_ok(), "free, and the very bytes already there");
    let twice = at("twice");
    assert!(eravon_cli::write::free(&[(twice.as_str(), mine), (twice.as_str(), mine)]).is_ok());
    let f = eravon_cli::write::free(&[(twice.as_str(), mine), (at("free-one").as_str(), mine), (twice.as_str(), b"other bytes")]).expect_err("twice, other bytes");
    assert_eq!((f.name().to_string(), f.detail()), (String::from("E_V_EXISTS"), format!("{twice} 已经有了,一个字节也没盖")));
    assert!(!w.join("twice").exists(), "nothing written");
    let before = std::fs::metadata(w.join("same")).and_then(|m| m.modified()).expect("its time");
    assert!(eravon_cli::write::land(&at("same"), mine).is_ok(), "the very bytes already there are landed already");
    assert_eq!(std::fs::metadata(w.join("same")).and_then(|m| m.modified()).expect("its time"), before, "and nothing is written there");
    let f = eravon_cli::write::land(&at("same"), b"other bytes").expect_err("other bytes");
    assert_eq!(f.name(), "E_V_EXISTS");
    assert_eq!(std::fs::read(w.join("same")).expect("still there"), mine, "other bytes overwrite nothing");
    let src = |p: &str| std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(p)).expect(p);
    for (file, made, first_write) in [
        ("src/seller/goods.rs", "let bytes = eravon_goods::link::listing(", "s.put(&who, Kind::Payload"),
        ("src/seller/terms.rs", "let doc = write::terms_document(", "s.put(\n        &who,\n        Kind::TermsDocument"),
        ("src/buyer/order.rs", "let signed = sign_order(", "keep_order(&c.write()?"),
        ("src/buyer/company.rs", "let d = co_draft(", "co_registered(&d"),
        ("src/company.rs", "let bytes = p.bytes()?", "crate::write::land(path, &bytes)"),
    ] {
        let code = src(file);
        let (Some(m), Some(free), Some(write)) = (code.find(made), code.find("crate::write::free("), code.find(first_write)) else { panic!("{file}: all three in place") };
        assert!(m < free && free < write, "{file}: the names are judged with their bytes, before the first write");
    }
}
