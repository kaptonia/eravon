use app::roles::Role;
use eravon::json::Value;

fn text(v: &Value, k: &str) -> String {
    v.member(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn utf8(b: &[u8]) -> String {
    String::from_utf8(b.to_vec()).expect("text")
}

fn places() -> Vec<(&'static str, String)> {
    vec![
        ("app::home::HOME_STEM", app::home::HOME_STEM.into()),
        ("app::backup::APP", app::backup::APP.into()),
        ("app::backup::EXT", app::backup::EXT.into()),
        ("app::home::APP_DIR", app::home::APP_DIR.into()),
        ("eravon_net::USER_AGENT", eravon_net::USER_AGENT.into()),
        ("app::backup::MAGIC", utf8(app::backup::MAGIC)),
        ("eravon_store::sealed::MAGIC", utf8(eravon_store::sealed::MAGIC)),
        ("eravon_store::sealed::MAGIC_V2", utf8(eravon_store::sealed::MAGIC_V2)),
        ("app::local::GONE", utf8(app::local::GONE)),
        ("app::keybox::SHAPE", app::keybox::SHAPE.into()),
        ("app::keybox::SHAPE_V2", app::keybox::SHAPE_V2.into()),
        ("app::keybox::SHAPE_V1", app::keybox::SHAPE_V1.into()),
        ("app::identity::SHAPE", app::identity::SHAPE.into()),
        ("app::identity::SHAPE_V1", app::identity::SHAPE_V1.into()),
        ("app::machine::SHAPE", app::machine::SHAPE.into()),
        ("app::backup::INDEX_FORM", app::backup::INDEX_FORM.into()),
        ("app::keybox::LOCAL_INFO", utf8(app::keybox::LOCAL_INFO)),
        ("app::keybox::NAMES_INFO", utf8(app::keybox::NAMES_INFO)),
        ("app::keybox::BIND_DOMAIN", utf8(app::keybox::BIND_DOMAIN)),
        ("app::keybox::MEMBER_INFO", utf8(app::keybox::MEMBER_INFO)),
        ("app::local::OWNER_DOMAIN", app::local::OWNER_DOMAIN.into()),
        ("app::local::PLACE_DOMAIN", app::local::PLACE_DOMAIN.into()),
        ("app::local::PLACE_KEY_INFO", app::local::PLACE_KEY_INFO.into()),
        ("eravon_send::sent::SEAL_INFO", eravon_send::sent::SEAL_INFO.into()),
        ("app::family::path_text(Author)", app::family::path_text(Role::Author)),
        ("app::family::path_text(Grantee)", app::family::path_text(Role::Grantee)),
        ("app::home::HOME_ENV", app::home::HOME_ENV.into()),
        ("app::trace::SINK_ENV", app::trace::SINK_ENV.into()),
        ("app::chainx::REST_KEYS_ENV", app::chainx::REST_KEYS_ENV.into()),
        ("eravon_send::sent::LOGS_INFO", eravon_send::sent::LOGS_INFO.into()),
        ("eravon_chain::settled::FORM", eravon_chain::settled::FORM.into()),
        ("eravon_scan::judged::FORM", eravon_scan::judged::FORM.into()),
        ("eravon_ui::mdedit::ID_HEAD", eravon_ui::mdedit::ID_HEAD.into()),
        ("eravon_os::door::NAME_HEAD", eravon_os::door::NAME_HEAD.into()),
        ("eravon_cli::door::FORM", eravon_cli::door::FORM.into()),
        ("eravon_os::cli_path::ENV_DIR", eravon_os::cli_path::ENV_DIR.into()),
        ("eravon_os::machine::POINTER", eravon_os::machine::POINTER.into()),
        ("eravon_glue::face::ENTRY_TYPE", eravon_glue::face::ENTRY_TYPE.into()),
        ("eravon_glue::face::DOMAIN", eravon_glue::face::DOMAIN.into()),
        ("eravon_glue::face::MARKER_PREFIX", eravon_glue::face::MARKER_PREFIX.into()),
        ("eravon_glue::face::CONSENT_PREFIX", eravon_glue::face::CONSENT_PREFIX.into()),
        ("eravon_seal::places::CREDENTIAL_HEAD", eravon_seal::places::CREDENTIAL_HEAD.into()),
        ("eravon_seal::credential::DOMAIN", utf8(eravon_seal::credential::DOMAIN)),
        ("eravon_seal::credential::PASS_ENV", eravon_seal::credential::PASS_ENV.into()),
        ("eravon_seal::credential::PASS_PREFIX", eravon_seal::credential::PASS_PREFIX.into()),
    ]
}

#[test]
fn the_register_is_the_codes_literals_both_ways() {
    let bytes = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("family-literals.json")).expect("the register");
    let reg = eravon::json::parse(&bytes).expect("the register reads");
    assert_eq!(text(&reg, "schema"), "eravon-family-literals/1");
    assert!(reg.member("foreign").map(|f| matches!(f, Value::Obj(m) if m.is_empty())).unwrap_or(false), "foreign is an empty object");
    let roles: Vec<String> = reg.member("roles").and_then(|r| r.as_arr()).expect("roles").iter().filter_map(|r| r.as_str().map(str::to_string)).collect();
    let entries = reg.member("entries").and_then(|e| e.as_arr()).expect("entries").clone();
    let places = places();
    for e in &entries {
        let (role, literal, at) = (text(e, "role"), text(e, "literal"), text(e, "at"));
        assert!(roles.contains(&role), "{at}: role {role} is not in the register's list");
        assert!(!text(e, "says").is_empty(), "{at}: says what it is");
        let value = places.iter().find(|(p, _)| *p == at).map(|(_, v)| v.clone()).unwrap_or_else(|| panic!("{at}: no such place in the code"));
        assert_eq!(literal, value, "{at}: the register and the code differ");
    }
    for (at, _) in &places {
        assert!(entries.iter().any(|e| text(e, "at") == *at), "{at}: a family literal with no entry");
    }
    literals_within_register(&reg);
}

fn literals_within_register(reg: &Value) {
    let entries = reg.member("entries").and_then(|e| e.as_arr()).expect("entries").clone();
    let mut seen: Vec<(String, String)> = Vec::new();
    for e in &entries {
        let (role, literal) = (text(e, "role"), text(e, "literal"));
        assert!(!seen.contains(&(role.clone(), literal.clone())), "{literal:?} twice in role {role}");
        seen.push((role, literal));
    }
    let pairs: Vec<(String, Vec<String>)> = reg
        .member("pairs")
        .and_then(|p| p.as_arr())
        .expect("pairs")
        .iter()
        .map(|p| (text(p, "literal"), p.member("roles").and_then(|r| r.as_arr()).map(|r| r.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()))
        .collect();
    for (_, literal) in &seen {
        let mut in_roles: Vec<String> = seen.iter().filter(|(_, l)| l == literal).map(|(r, _)| r.clone()).collect();
        in_roles.sort();
        if in_roles.len() > 1 {
            let listed = pairs.iter().any(|(l, rs)| {
                let mut rs = rs.clone();
                rs.sort();
                l == literal && rs == in_roles
            });
            assert!(listed, "{literal:?} is in {in_roles:?} without a listed pair");
        }
    }
    for (literal, rs) in &pairs {
        assert!(rs.len() == 2 && rs.iter().all(|r| seen.contains(&(r.clone(), literal.clone()))), "the pair {literal:?} is a literal in exactly those two roles");
    }
}

fn refused(register: &[u8]) -> Option<String> {
    let reg = eravon::json::parse(register).expect("a small register reads");
    let r = std::panic::catch_unwind(|| literals_within_register(&reg));
    r.err().map(|p| p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default())
}

#[test]
fn a_register_with_a_literal_twice_in_one_role_is_refused() {
    let dup = br#"{"entries":[{"role":"magic","literal":"ZK1"},{"role":"magic","literal":"ZK1"}],"pairs":[]}"#;
    let said = refused(dup).expect("refused");
    assert!(said.contains("twice in role magic"), "{said}");
    let apart = br#"{"entries":[{"role":"magic","literal":"ZK1"},{"role":"magic","literal":"ZK2"}],"pairs":[]}"#;
    assert_eq!(refused(apart), None, "two literals in one role pass");
}

#[test]
fn a_register_with_a_cross_role_literal_and_no_pair_is_refused() {
    let unpaired = br#"{"entries":[{"role":"magic","literal":"ZK1"},{"role":"form","literal":"ZK1"}],"pairs":[]}"#;
    let said = refused(unpaired).expect("refused");
    assert!(said.contains("without a listed pair"), "{said}");
    let paired = br#"{"entries":[{"role":"magic","literal":"ZK1"},{"role":"form","literal":"ZK1"}],"pairs":[{"literal":"ZK1","roles":["form","magic"]}]}"#;
    assert_eq!(refused(paired), None, "listed as a pair, it passes");
}
