#[test]
fn every_column_home_stand_prints_is_in_the_key_table() {
    use eravon_cli::codes::Key;
    use eravon_store::mirror::{Mirror, Stand};
    let full = Stand {
        writable: false,
        holder: Some(eravon_store::writer::Words { pid: 7, since: 1, writer: "w".into() }),
        other: Some("m".into()),
        mark_unread: Some("x".into()),
        mirror: Some(Mirror { cut_at: 1, from: "f".into() }),
    };
    let known: Vec<&str> = Key::ALL.iter().map(|k| k.as_str()).collect();
    for (name, _) in full.rows() {
        assert!(known.contains(&name), "home-stand prints {name}, which codes::Key does not have");
    }
}
