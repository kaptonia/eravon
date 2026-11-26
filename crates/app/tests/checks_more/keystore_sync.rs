use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("zk-gaps-ks-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn with_member(v: &eravon::json::Value, path: &[&str], to: Option<eravon::json::Value>) -> eravon::json::Value {
    use eravon::json::Value;
    let Value::Obj(m) = v else { return v.clone() };
    let mut m = m.clone();
    match path {
        [k] => {
            m.retain(|(x, _)| x != k);
            if let Some(t) = to {
                m.push((k.to_string(), t));
            }
        }
        [k, rest @ ..] => {
            for (x, inner) in m.iter_mut() {
                if x == k {
                    *inner = with_member(inner, rest, to.clone());
                }
            }
        }
        [] => {}
    }
    Value::Obj(m)
}

#[test]
fn a_keystore_whose_p_is_text_or_a_fraction_is_said_as_its_shape() {
    if super::alone_in(module_path!(), "a_keystore_whose_p_is_text_or_a_fraction_is_said_as_its_shape") {
        return;
    }
    use eravon::json::Value;
    let secret = app::key::Secret::take([0x42; 32]).expect("a key");
    let ks = app::keystore::encrypt(&secret, "pw-probe", app::keystore::Params::light(), 1_700_000_000).expect("encrypted");
    let v = eravon::json::parse(&ks.json).expect("json");
    let open = |v: &Value| app::keystore::decrypt(&eravon::json::canon_bytes(v), "pw-probe").err().map(|f| f.said().split(':').next().unwrap_or("").to_string());
    assert_eq!(open(&v), None, "the file as written opens");
    assert_eq!(open(&with_member(&v, &["crypto", "kdfparams", "p"], Some(Value::Str("1".into())))).as_deref(), Some("KEYSTORE_SHAPE"), "p as text");
    let text = String::from_utf8(eravon::json::canon_bytes(&v)).expect("text");
    let at = text.find("\"p\":").expect("p is written") + 4;
    let end = at + text[at..].find(|c: char| !c.is_ascii_digit()).expect("p ends");
    let fraction = format!("{}1.5{}", &text[..at], &text[end..]);
    assert!(fraction.contains("\"p\":1.5"), "{fraction}");
    let said = app::keystore::decrypt(fraction.as_bytes(), "pw-probe").err().map(|f| f.said().split(':').next().unwrap_or("").to_string());
    assert_eq!(said.as_deref(), Some("KEYSTORE_SHAPE"), "p a fraction");
}

#[test]
fn a_write_whose_directory_sync_fails_has_still_renamed_the_file_into_place() {
    if super::alone_in(module_path!(), "a_write_whose_directory_sync_fails_has_still_renamed_the_file_into_place") {
        return;
    }
    let dir = scratch("put-sync");
    app::home::put_at(&dir, "a.json", b"1").expect("lands");
    std::fs::write(dir.join(".staged"), b"3").expect("a staged file");
    eravon_os::pretend_sync_fails_at(Some(dir.clone()));
    let put = app::home::put_at(&dir, "a.json", b"2");
    let over = app::home::rename_over(&dir.join(".staged"), &dir.join("b.json"));
    eravon_os::pretend_sync_fails_at(None);
    assert!(put.is_err() && over.is_err(), "both said not done");
    assert_eq!(std::fs::read(dir.join("a.json")).expect("a.json is in place"), b"2", "put_at: the new bytes are in place");
    assert_eq!(std::fs::read(dir.join("b.json")).expect("b.json is in place"), b"3", "rename_over: the target exists with the staged bytes");
    assert!(!dir.join(".staged").exists(), "rename_over: the staged name is gone");
    let _ = std::fs::remove_dir_all(&dir);
}

