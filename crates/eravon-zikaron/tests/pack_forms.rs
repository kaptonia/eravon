use eravon_zikaron::{index, pack, seam};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name).display().to_string()
}

fn row(path: &str, sealed: bool) -> index::Kit {
    index::Kit { id: "0x760a".into(), created: 1, root: "0x00".into(), contents: vec![], note_md: String::new(), path: path.into(), link: String::new(), sealed }
}

fn items(path: &str) -> Vec<String> {
    pack::items(&row(path, false), seam::MEDIA_DEFAULT).expect("the kit reads").iter().map(|f| format!("{} {} {} {}", f.name(), f.sha256(), f.bytes(), f.media_type())).collect()
}

static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn leftovers() -> Vec<PathBuf> {
    std::fs::read_dir(std::env::temp_dir()).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(eravon_glue::read::UNPACK_HEAD))).collect()).unwrap_or_default()
}

#[test]
fn a_package_in_one_file_gives_the_same_items_as_the_folder() {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let before = leftovers();
    let one = items(&fixture("zikaron-0.1.4-kit.zkit"));
    let folder = items(&fixture("zikaron-0.1.4-kit"));
    assert_eq!(one, folder, "one file and folder: the same items");
    assert_eq!(one.len(), 6, "manifest, two entries, three files: {one:#?}");
    assert!(one.iter().all(|i| i.starts_with(seam::kit_prefix())), "{one:#?}");
    assert!(one.iter().any(|i| i.starts_with(&format!("{}files/notes/delivery.txt ", seam::kit_prefix()))), "the nested original by its path in the kit");
    assert_eq!(leftovers(), before, "the folder the file was read into is removed");
}

#[test]
fn a_row_sealed_to_one_key_is_refused_by_name() {
    let e = pack::items(&row(&fixture("zikaron-0.1.4-kit.zkit"), true), seam::MEDIA_DEFAULT).expect_err("sealed refused");
    assert_eq!(e.name(), "E_ZK_PACK");
    assert!(e.detail().contains(seam::F_ONE_TIME) && e.detail().contains("只有收件人打得开"), "{}", e.detail());
}

#[test]
fn a_file_that_is_not_one_package_is_refused_and_leaves_nothing() {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let w = std::env::temp_dir().join(format!("eravon-zk-forms-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&w);
    std::fs::create_dir_all(&w).expect("scratch");
    let raw = std::fs::read(fixture("zikaron-0.1.4-kit.zkit")).expect("the fixture");
    let cut = w.join("cut.zkit");
    std::fs::write(&cut, &raw[..raw.len() / 2]).expect("cut");
    let climbs = w.join("climbs.zkit");
    std::fs::write(&climbs, b"zikaron-kit-file/1\nmanifest.json\n2\n{}\n../x\n1\nx\n").expect("climbs");
    let other = w.join("other.bin");
    std::fs::write(&other, b"PK\x03\x04 not a package").expect("other");
    let before = leftovers();
    for (p, code) in [(&cut, "E_FILE_TRUNCATED"), (&climbs, "E_FILE_PATH")] {
        let e = pack::items(&row(&p.display().to_string(), false), seam::MEDIA_DEFAULT).expect_err("refused");
        assert_eq!(e.name(), "E_ZK_PACK");
        assert!(e.detail().contains(code), "{code}: {}", e.detail());
    }
    assert!(!w.parent().unwrap().join("x").exists(), "nothing written outside");
    assert_eq!(pack::items(&row(&other.display().to_string(), false), seam::MEDIA_DEFAULT).expect_err("not a folder").name(), "E_ZK_PACK");
    assert_eq!(leftovers(), before, "a refused file leaves no folder behind");
    let _ = std::fs::remove_dir_all(&w);
}
