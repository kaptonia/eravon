use eravon_cli::mcptools::TABLE as T;
use eravon_cli::{deskwire, schema, verbs, verbwire};

const HELD_BY_NAME: [&str; 8] = ["seller-pending-bump", "buyer-import", "seller-direct-check", "seller-direct-give-up", "buyer-direct-check", "buyer-direct-give-up", "seller-vault-rotate", "buyer-vault-rotate"];

fn row(verb: &str, homed: bool) -> Vec<&'static str> {
    verbs::accepts(verb, homed).unwrap_or_else(|| panic!("{verb} is a verb"))
}

fn homed(verb: &str) -> bool {
    verbs::HOME.iter().any(|(v, _)| *v == verb)
}

fn read(verb: &str) -> bool {
    T.reads.iter().any(|(r, _)| *r == verb)
}

fn tools() -> Vec<(&'static str, Vec<&'static str>)> {
    let mut all: Vec<(&str, Vec<&str>)> = verbs::HOME.iter().map(|(v, _)| *v).filter(|v| !T.not_writes.contains(v) && !read(v)).map(|v| (v, row(v, true))).collect();
    for &(verb, withheld) in T.reads {
        all.push((verb, row(verb, false).into_iter().filter(|f| !withheld.contains(f)).collect()));
    }
    all
}

#[test]
fn the_write_tools_are_the_rows_the_door_takes_less_three() {
    for v in T.not_writes {
        assert!(homed(v), "{v} is a row of verbs::HOME");
    }
    for (v, _) in verbs::HOME.iter() {
        let secret = schema::passcode_only(verbwire::split(v).map(|(_, x)| x).unwrap_or(v));
        assert_eq!(T.not_writes.contains(v), secret || HELD_BY_NAME.contains(v), "{v}: held back exactly when it needs a secret or is named");
    }
    let writes = tools().into_iter().filter(|(v, _)| !read(v) && !T.door_reads.contains(v)).count();
    assert_eq!(writes, WRITE_TOOLS);
}

const WRITE_TOOLS: usize = 93;

#[test]
fn the_door_reads_are_the_rows_that_write_send_and_sign_nothing() {
    let want: Vec<&str> = verbs::HOME
        .iter()
        .map(|(v, _)| *v)
        .filter(|v| !T.not_writes.contains(v) && verbwire::split(v).is_some_and(|(seat, x)| schema::door_reads(seat, x)))
        .collect();
    let mut have: Vec<&str> = T.door_reads.to_vec();
    let mut want = want;
    have.sort();
    want.sort();
    assert_eq!(have, want, "the door's reads are the read rows of the door");
    for v in T.door_reads {
        assert!(homed(v) && !read(v), "{v}: a row of the door, not a local read");
    }
}

#[test]
fn the_read_tools_write_nothing_and_withhold_flags_of_their_row() {
    assert_eq!(T.reads.len(), 10);
    for &(verb, withheld) in T.reads {
        assert!(!homed(verb), "{verb} is not a desktop verb");
        if let Some((seat, v)) = verbwire::split(verb) {
            assert!(!schema::door_takes(seat, v) && !schema::sends(seat, v), "{verb} writes or sends");
            assert!(!schema::signs(seat, v, &[]), "{verb} signs");
        }
        for f in withheld {
            assert!(row(verb, false).contains(f), "{verb} withholds --{f}, a flag of its row");
        }
    }
    assert!(T.writes_too.is_empty() && T.home_optional.is_empty() && T.as_home.is_empty() && T.joined.is_empty());
}

#[test]
fn no_tool_takes_a_secret() {
    for (verb, params) in tools() {
        for p in params {
            assert!(!deskwire::secret(p) && p != "key" && p != "passcode", "{verb} takes --{p}");
        }
    }
}

#[test]
fn the_senders_are_the_tools_that_send() {
    for (verb, _) in tools() {
        let sends = verbwire::split(verb).is_some_and(|(seat, v)| schema::sends(seat, v));
        assert_eq!(T.sends.contains(&verb), sends, "{verb}");
    }
    for v in T.sends {
        assert!(tools().iter().any(|(t, _)| t == v), "{v} is a tool");
    }
}

#[test]
fn the_paths_are_the_flags_of_type_path() {
    let mut listed: Vec<&str> = T.paths.iter().map(|(t, p)| {
        assert!(t.is_empty(), "a path for any tool");
        *p
    }).collect();
    let mut typed: Vec<&str> = tools().into_iter().flat_map(|(_, ps)| ps).filter(|p| *p == verbs::HOME_FLAG || schema::is_path(p)).collect();
    listed.sort_unstable();
    typed.sort_unstable();
    typed.dedup();
    assert_eq!(listed, typed);
}

#[test]
fn every_tool_says_its_sentence() {
    for (v, _) in tools() {
        assert!(!(T.told)(v).is_empty(), "{v} says its sentence");
    }
}

#[test]
fn a_parameter_says_it_takes_a_path_exactly_where_a_relative_path_is_refused() {
    use eravon::json::Value;
    let list = eravon_cli::mcp::listed(&T);
    let tools = list.member("tools").and_then(Value::as_arr).expect("tools");
    let mut described = 0;
    for tool in tools {
        let name = tool.member("name").and_then(Value::as_str).expect("a name");
        let Some(Value::Obj(props)) = tool.member("inputSchema").and_then(|s| s.member("properties")) else { panic!("properties of {name}") };
        for (param, schema) in props {
            let given = if schema.member("type").and_then(Value::as_str) == Some("array") { Value::Arr(vec![Value::Str("relative".into())]) } else { Value::Str("relative".into()) };
            let mut call = vec![(param.clone(), given)];
            let home = eravon_cli::verbs::HOME_FLAG;
            let home_needed = tool.member("inputSchema").and_then(|s| s.member("required")).and_then(Value::as_arr).is_some_and(|r| r.iter().any(|x| x.as_str() == Some(home)));
            if param != home && home_needed {
                call.insert(0, (home.to_string(), Value::Str(if cfg!(windows) { r"C:\h" } else { "/h" }.into())));
            }
            let refused = matches!(eravon_cli::mcp::argv_of(&T, name, &call, false), Some(Err(e)) if e.contains(eravon_cli::out::Said::McpAbsolute.text()));
            let said = schema.member("description").and_then(Value::as_str);
            assert_eq!(said.is_some(), refused, "{name} {param}: {schema:?}");
            described += usize::from(said.is_some());
            let mut kept: Vec<&String> = match schema {
                Value::Obj(m) => m.iter().map(|(k, _)| k).filter(|k| k.as_str() != "description").collect(),
                _ => Vec::new(),
            };
            kept.sort();
            assert!(kept == ["type"] || kept == ["items", "type"], "{name} {param}: the type is as before");
        }
    }
    assert!(described > 0, "some parameter takes a path");
}
