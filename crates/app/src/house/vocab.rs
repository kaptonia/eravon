use eravon_literals::json::Out;

pub const BLANK: &str = "";

pub const BLANK_FIELDS: [&str; 5] = ["export.dir", "goods.edit.draft.price", "write.chain", "write.local", "write.vault"];

pub fn closed() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        ("first.step", crate::nav::Step::ALL.iter().map(|s| s.name()).collect()),
        ("gate.card", crate::house::gatecard::Card::ALL.iter().map(|c| c.name()).collect()),
        ("flight.kind", crate::task::Kind::ALL.iter().map(|k| k.as_str()).collect()),
        ("refresh.cause", crate::house::refresh::Cause::ALL.iter().map(|c| c.name()).collect()),
        ("said.tone", crate::house::seller::Tone::ALL.iter().map(|t| t.word()).collect()),
    ]
}

pub fn schema_rows() -> Vec<(&'static str, Out)> {
    let words = |v: &[&'static str]| Out::A(v.iter().map(|w| Out::S((*w).to_string())).collect());
    vec![
        ("blank", Out::O(vec![("fields", words(&BLANK_FIELDS)), ("value", Out::S(BLANK.to_string()))])),
        ("closed", Out::O(closed().into_iter().map(|(k, v)| (k, words(&v))).collect())),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use eravon_literals::json::In;

    #[test]
    fn every_member_has_its_own_word() {
        for (path, v) in closed() {
            let mut seen = std::collections::BTreeSet::new();
            for w in &v {
                assert!(!w.is_empty(), "{path}: an empty word");
                assert!(seen.insert(*w), "{path}: {w} twice");
            }
        }
    }

    #[test]
    fn the_lock_lists_the_same_words() {
        let lock = include_bytes!("../../../../tools/schemalock/lock/eravon-app-schema.json");
        let doc = eravon_literals::json::read(lock).expect("the lock reads as JSON");
        let set = doc.get("closed").expect("the lock has a closed block");
        for (path, v) in closed() {
            let got: Vec<&str> = set.get(path).and_then(In::as_arr).unwrap_or(&[]).iter().filter_map(In::as_str).collect();
            assert_eq!(got, v, "{path}");
        }
        let blank = doc.get("blank").expect("the lock has a blank block");
        assert_eq!(blank.get("value").and_then(In::as_str), Some(BLANK));
        let fields: Vec<&str> = blank.get("fields").and_then(In::as_arr).unwrap_or(&[]).iter().filter_map(In::as_str).collect();
        assert_eq!(fields, BLANK_FIELDS);
    }

    #[test]
    fn blank_cells_answer_the_empty_string() {
        let read = |o: Out| eravon_literals::json::read(&eravon_literals::json::write(&o)).expect("answers read back");
        let ex = read(crate::house::export::Export::default().rows());
        assert_eq!(ex.get("dir"), Some(&In::Str(BLANK.to_string())));
        let draft = read(crate::house::seller::goods::draft_out(&eravon_goods::draft::Draft::default()));
        assert_eq!(draft.get("price"), Some(&In::Str(BLANK.to_string())));
        assert_eq!(crate::house::drive::write_word(None), BLANK);
    }
}
