use eravon_zikaron::index;
#[cfg(unix)]
use eravon_zikaron::seam;

const SAMPLE: &str = include_str!("fixtures/kits-index-sample.json");

fn machine(tag: &str, bytes: &str) -> String {
    let d = std::env::temp_dir().join(format!("eravon-zk-sample-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("kits")).expect("machine directory");
    std::fs::write(d.join("kits/index.json"), bytes).expect("index");
    d.to_string_lossy().into_owned()
}

#[cfg(unix)]
#[test]
fn both_sample_rows_read() {
    let m = machine("both", SAMPLE);
    let ix = index::load(&m).expect("the sample reads");
    assert_eq!(ix.form, seam::form());
    assert_eq!(ix.kits.len(), 2, "no row dropped");
    let (a, b) = (&ix.kits[0], &ix.kits[1]);
    assert_eq!(a.link, "");
    assert_eq!(a.contents.len(), 1);
    assert_eq!(a.path, "/tmp/zk-sample/kits/a");
    assert_eq!(b.link, "https://example.org/kits/b");
    assert_eq!(b.contents.len(), 2);
    assert_eq!(b.path, "/tmp/zk-sample/kits/b");
    assert_eq!(b.created, 1790100000);
    assert!(matches!(index::stands(&m), Ok(index::Stand::Yes)));
    let _ = std::fs::remove_dir_all(&m);
}

#[test]
fn an_integer_form_refuses_the_whole_file() {
    let m = machine("int-form", &SAMPLE.replace("\"form\":\"zikaron.kits-index/1\"", "\"form\":1"));
    let e = index::load(&m).expect_err("integer form refused");
    assert_eq!(e.name(), "E_ZK_INDEX_FORM");
    assert!(index::stands(&m).is_err(), "an unreadable form is a refusal, not an absence");
    let _ = std::fs::remove_dir_all(&m);
}

#[test]
fn contents_as_a_count_is_malformed() {
    let first = "\"contents\":[\"0xd0f631ca1ddba8db3bcfcb9e057cdc98d0379f1bee00e75a545147a27dadd982\"]";
    assert!(SAMPLE.contains(first));
    let m = machine("contents", &SAMPLE.replacen(first, "\"contents\":1", 1));
    let e = index::load(&m).expect_err("integer contents refused");
    assert_eq!(e.name(), "E_ZK_INDEX_SHAPE");
    let _ = std::fs::remove_dir_all(&m);
}

#[test]
fn a_relative_path_is_malformed() {
    let m = machine("relative", &SAMPLE.replacen("\"path\":\"/tmp/zk-sample/kits/a\"", "\"path\":\"kits/a\"", 1));
    let e = index::load(&m).expect_err("relative path refused");
    assert_eq!(e.name(), "E_ZK_INDEX_SHAPE");
    let _ = std::fs::remove_dir_all(&m);
}

#[cfg(unix)]
#[test]
fn a_note_with_a_middle_dot_reads() {
    let line = "anchored-on: eip155:8453 \u{b7} registry 0x7a31c05e9b2d48f6a1e37c90d5b264f8e03a9c04 \u{b7} from 24100000";
    let m = machine("dot", &SAMPLE.replacen("\"note_md\":\"新包,作者写的说明\"", &format!("\"note_md\":\"新包,作者写的说明\\n{line}\""), 1));
    let ix = index::load(&m).expect("a note with U+00B7 reads");
    assert!(ix.kits[1].note_md.ends_with(line));
    let _ = std::fs::remove_dir_all(&m);
}
