use std::path::{Path, PathBuf};

fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("crates/").to_path_buf()
}

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n != "target") {
                sources(&p, out);
            }
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn module_of(file: &Path) -> Option<String> {
    let rel = file.strip_prefix(crates_dir()).ok()?;
    let mut parts: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    let krate = parts.remove(0).replace('-', "_");
    if parts.first().map(String::as_str) != Some("src") {
        return None;
    }
    parts.remove(0);
    if parts.first().map(String::as_str) == Some("bin") {
        return None;
    }
    let last = parts.pop()?;
    let stem = last.trim_end_matches(".rs");
    if stem != "lib" && stem != "mod" && stem != "main" {
        parts.push(stem.to_string());
    }
    Some(std::iter::once(krate).chain(parts).collect::<Vec<_>>().join("::"))
}

fn unquote(rest: &str) -> Option<String> {
    let mut out = String::new();
    let mut it = rest.chars();
    while let Some(c) = it.next() {
        match c {
            '"' => return Some(out),
            '\\' => match it.next()? {
                'n' => out.push('\n'),
                '0' => out.push('\0'),
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                't' => out.push('\t'),
                other => {
                    out.push('\\');
                    out.push(other);
                }
            },
            c => out.push(c),
        }
    }
    None
}

fn consts(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut inline: Vec<(String, usize)> = Vec::new();
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        if inline.last().is_some_and(|(_, at)| *at == indent && line.trim() == "}") {
            inline.pop();
            continue;
        }
        let opened = line.trim().strip_prefix("pub mod ").or_else(|| line.trim().strip_prefix("pub(crate) mod ")).or_else(|| line.trim().strip_prefix("mod "));
        if let Some(name) = opened.and_then(|r| r.strip_suffix(" {")) {
            inline.push((name.to_string(), indent));
            continue;
        }
        let l = line.trim_start();
        let Some(i) = l.find("const ") else { continue };
        if !(l.starts_with("pub ") || l.starts_with("const ") || l.starts_with("pub(crate) ")) {
            continue;
        }
        let rest = &l[i + 6..];
        let Some((name, ty)) = rest.split_once(':') else { continue };
        let ty = ty.trim_start();
        let after = if let Some(x) = ty.strip_prefix("&str = \"").or_else(|| ty.strip_prefix("&'static str = \"")) {
            x
        } else if let Some(x) = ty.strip_prefix("&[u8] = b\"").or_else(|| ty.strip_prefix("&'static [u8] = b\"")) {
            x
        } else {
            continue;
        };
        if let Some(v) = unquote(after) {
            let at: Vec<&str> = inline.iter().map(|(n, _)| n.as_str()).chain(std::iter::once(name.trim())).collect();
            out.push((at.join("::"), v));
        }
    }
    out
}

fn family_shaped(v: &str) -> bool {
    let w = v.strip_suffix('\n').unwrap_or(v);
    !w.chars().any(char::is_whitespace) && !w.contains("::") && w.to_ascii_lowercase().contains(eravon_literals::FAMILY)
}

fn all_consts() -> Vec<(String, String)> {
    let mut files = Vec::new();
    sources(&crates_dir(), &mut files);
    let mut out = Vec::new();
    for f in files {
        let Some(m) = module_of(&f) else { continue };
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        for (name, v) in consts(&text) {
            out.push((format!("{m}::{name}"), v));
        }
    }
    out
}

#[test]
fn the_published_register_is_the_generators_output() {
    let at = Path::new(env!("CARGO_MANIFEST_DIR")).join("family-literals.json");
    let on_disk = std::fs::read(&at).expect("family-literals.json is in the source");
    assert!(
        on_disk == eravon_literals::registry::family_json(),
        "{} differs from the register: regenerate with `cargo run -p eravon-literals --example family-literals > crates/eravon-literals/family-literals.json`",
        at.display()
    );
}

#[test]
fn every_entry_equals_the_code_at_its_place() {
    let code = all_consts();
    for e in eravon_literals::registry::ENTRIES {
        for place in e.places() {
            if place.starts_with("eravon_literals::") {
                continue;
            }
            let found: Vec<&String> = code.iter().filter(|(at, _)| at == place).map(|(_, v)| v).collect();
            assert_eq!(found.len(), 1, "{} ({:?}) names a place the source does not have once", place, e.literal);
            assert_eq!(found[0], e.literal, "{}: the source says {:?}, the register {:?}", place, found[0], e.literal);
        }
    }
}

#[test]
fn every_family_shaped_literal_in_the_code_has_its_entry() {
    let unregistered: Vec<(String, String)> = all_consts()
        .into_iter()
        .filter(|(_, v)| family_shaped(v))
        .filter(|(at, v)| !eravon_literals::registry::ENTRIES.iter().any(|e| e.places().contains(&at.as_str()) && e.literal == v))
        .collect();
    assert!(unregistered.is_empty(), "family-shaped literals with no entry at their place: {unregistered:#?}");
}

#[test]
fn the_scan_tells_family_shaped_values() {
    let text = "pub const A: &str = \"eravon.x/1\";\nconst B: &[u8] = b\"eravon-local/1\\n\";\npub const C: &str = \"eravon.chain/1 · prose\";\npub const D: &str = \"eravon-ui::form::Form\";\n";
    let got = consts(text);
    assert_eq!(got.len(), 4, "four consts are read");
    let nested = consts("pub mod env {\n    pub const T: &str = \"ERAVON_X\";\n}\npub const U: &str = \"eravon-u\";\n");
    assert_eq!(nested, vec![("env::T".to_string(), "ERAVON_X".to_string()), ("U".to_string(), "eravon-u".to_string())], "inline modules name their consts");
    assert_eq!(got[1], ("B".to_string(), "eravon-local/1\n".to_string()));
    let shaped: Vec<&str> = got.iter().filter(|(_, v)| family_shaped(v)).map(|(n, _)| n.as_str()).collect();
    assert_eq!(shaped, ["A", "B"]);
}

fn literals_within_register(reg: &eravon_literals::json::In) {
    let text = |v: &eravon_literals::json::In, k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let entries = reg.get("entries").and_then(|e| e.as_arr()).expect("entries");
    let mut seen: Vec<(String, String)> = Vec::new();
    for e in entries {
        let (role, literal) = (text(e, "role"), text(e, "literal"));
        assert!(!seen.contains(&(role.clone(), literal.clone())), "{literal:?} twice in role {role}");
        seen.push((role, literal));
    }
    let pairs: Vec<(String, Vec<String>)> = reg
        .get("pairs")
        .and_then(|p| p.as_arr())
        .expect("pairs")
        .iter()
        .map(|p| (text(p, "literal"), p.get("roles").and_then(|r| r.as_arr()).map(|r| r.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()))
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
        assert!(rs.len() >= 2 && rs.iter().all(|r| seen.contains(&(r.clone(), literal.clone()))), "the pair {literal:?} is a literal in exactly those roles");
    }
}

fn refused(register: &[u8]) -> Option<String> {
    let reg = eravon_literals::json::read(register).expect("a small register reads");
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

#[test]
fn the_published_register_keeps_the_two_rules() {
    let bytes = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("family-literals.json")).expect("the register");
    assert_eq!(refused(&bytes), None);
}
