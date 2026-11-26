use eravon_zikaron::{discover, seam};

const SAMPLE: &str = include_str!("fixtures/kits-index-sample.json");

fn home(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("eravon-zk-ptr-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let m = std::path::PathBuf::from(seam::default_dir(&d.to_string_lossy()));
    std::fs::create_dir_all(m.join("kits")).expect("machine directory");
    std::fs::write(m.join("kits/index.json"), SAMPLE).expect("index");
    d
}

#[test]
fn a_relative_pointer_is_read_inside_the_pointers_own_folder() {
    let d = home("rel");
    let base = d.to_string_lossy().into_owned();
    let default = seam::default_dir(&base);
    let name = std::path::Path::new(&default).file_name().expect("a name").to_string_lossy().into_owned();
    std::fs::write(seam::pointer_path(&base), format!("{name}\n")).expect("pointer");
    let link = discover::discover(&base, None).expect("connected");
    assert_eq!((link.at.as_str(), link.layer), (default.as_str(), discover::Layer::Pointer), "the relative pointer reads inside its folder");

    std::fs::write(seam::pointer_path(&base), format!("{default}\n")).expect("pointer");
    let link = discover::discover(&base, None).expect("connected");
    assert_eq!((link.at.as_str(), link.layer), (default.as_str(), discover::Layer::Pointer), "an absolute pointer as before");

    for bad in [format!("../{name}\n"), format!("./{name}\n"), format!("{name}"), format!("{name}\r\n")] {
        std::fs::write(seam::pointer_path(&base), &bad).expect("pointer");
        assert!(discover::discover(&base, None).is_err(), "{bad:?} is a stale pointer, never the default");
    }
    let folder = std::path::Path::new("/f");
    assert_eq!(discover::read_pointer(b"a/b\n", folder).as_deref().map(std::path::Path::new), Some(std::path::Path::new("/f/a/b")));
    assert_eq!(discover::read_pointer(b"a/../b\n", folder), None);
    let _ = std::fs::remove_dir_all(&d);
}
