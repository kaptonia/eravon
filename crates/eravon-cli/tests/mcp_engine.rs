use eravon::json::Value;
use eravon_cli::mcp::{self, Table};
use eravon_cli::verbs;

fn told(tool: &str) -> &'static str {
    if tool == "seller-note-handle" { "one" } else { "other" }
}

const KEPT: [&str; 3] = ["seller-note-handle", "seller-order-label", "seller-export"];

fn table() -> Table {
    let not_writes: Vec<&'static str> = verbs::HOME.iter().map(|(v, _)| *v).filter(|v| !KEPT.contains(v)).collect();
    Table {
        name: "engine-test",
        not_writes: Box::leak(not_writes.into_boxed_slice()),
        reads: &[("seller-terms-hash", &["passcode"]), ("seller-zikaron-status", &["machine"]), ("seller-export", &["passcode"])],
        writes_too: &["seller-export"],
        home_optional: &[],
        as_home: &[("seller-zikaron-status", "home")],
        paths: &[("", "desktop")],
        joined: &[],
        sends: &["seller-note-handle"],
        alone_home: ("data", ""),
        alone_more: &[],
        door_reads: &[],
        told,
    }
}

fn tool<'a>(list: &'a Value, name: &str) -> &'a Value {
    let Some(Value::Arr(all)) = list.member("tools") else { panic!("tools") };
    all.iter().find(|t| t.member("name").and_then(Value::as_str) == Some(name)).unwrap_or_else(|| panic!("{name} listed"))
}

fn params(t: &Value) -> Vec<String> {
    match t.member("inputSchema").and_then(|s| s.member("properties")) {
        Some(Value::Obj(ms)) => ms.iter().map(|(k, _)| k.clone()).collect(),
        _ => panic!("properties"),
    }
}

fn required(t: &Value) -> bool {
    t.member("inputSchema").and_then(|s| s.member("required")).is_some()
}

fn required_names(t: &Value) -> Vec<String> {
    match t.member("inputSchema").and_then(|s| s.member("required")) {
        Some(Value::Arr(a)) => a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect(),
        _ => Vec::new(),
    }
}

fn hint(t: &Value, k: &str) -> Option<bool> {
    match t.member("annotations").and_then(|a| a.member(k)) {
        Some(Value::Bool(b)) => Some(*b),
        _ => None,
    }
}

#[test]
fn the_tools_are_read_off_the_verb_table_and_the_table_handed_in() {
    let t = table();
    let list = mcp::listed(&t);
    let Some(Value::Arr(all)) = list.member("tools") else { panic!("tools") };
    let names: Vec<&str> = all.iter().filter_map(|t| t.member("name").and_then(Value::as_str)).collect();
    assert_eq!(names, ["seller-note-handle", "seller-order-label", "seller-terms-hash", "seller-zikaron-status", "seller-export"]);

    let handle = tool(&list, "seller-note-handle");
    assert_eq!(params(handle), ["desktop", "now", "writer", "endpoint", "table", "roots", "proxy", "chain", "logic", "address", "reminder"]);
    assert_eq!(required_names(handle), ["desktop"], "a write needs its home, and nothing more: the desktop gives its own now");
    assert_eq!((hint(handle, "readOnlyHint"), hint(handle, "destructiveHint"), hint(handle, "openWorldHint")), (Some(false), Some(true), Some(true)));
    assert_eq!(handle.member("description").and_then(Value::as_str), Some("one"));

    let label = tool(&list, "seller-order-label");
    assert_eq!(hint(label, "destructiveHint"), None, "only the table's senders send");
    let endpoint = label.member("inputSchema").and_then(|s| s.member("properties")).and_then(|p| p.member("endpoint"));
    assert_eq!(endpoint.and_then(|e| e.member("type")).and_then(Value::as_str), Some("array"), "a repeated flag is a list");

    let hash = tool(&list, "seller-terms-hash");
    assert_eq!(params(hash), ["data", "now", "writer", "endpoint", "table", "roots", "proxy", "chain", "logic", "file"], "a withheld flag is no parameter");
    assert_eq!(hint(hash, "readOnlyHint"), Some(true));
    assert_eq!(required_names(hash), ["data", "now"]);

    let status = tool(&list, "seller-zikaron-status");
    assert!(params(status).contains(&"desktop".to_string()) && !params(status).contains(&"home".to_string()), "the flag given as the home is named home");
    assert!(!params(status).contains(&"machine".to_string()));

    let export = tool(&list, "seller-export");
    assert_eq!(hint(export, "readOnlyHint"), Some(false), "a read tool that writes");
    assert!(required(export), "its home is required unless the table lets it be left out");
    assert!(!params(export).iter().any(|p| p == "passcode"));
}

#[test]
fn a_refused_token_is_named_where_it_stands() {
    assert_eq!(mcp::fault_at(br#"{"params":{"arguments":{"n":-1}}}"#), "params.arguments.n");
    assert_eq!(mcp::fault_at(br#"{"a":[1,2,3.5]}"#), "a.2");
    assert_eq!(mcp::fault_at(br#"{"a":9007199254740992}"#), "a");
    assert_eq!(mcp::fault_at(br#"{"a":9007199254740991,"b":1,"b":2}"#), "b");
    assert_eq!(mcp::fault_at(br#"{"a":01}"#), "a");
    let deep = format!("{}{}", "[".repeat(129), "]".repeat(129));
    assert_eq!(mcp::fault_at(deep.as_bytes()), (0..128).map(|_| "0").collect::<Vec<_>>().join("."));
    assert_eq!(mcp::fault_at(br#"{"a":"-1.5e3","b":[0,7]}"#), ".");
}

#[test]
fn a_row_of_the_desktops_verbs_can_be_offered_as_a_read_through_the_desktop() {
    let reads = Table { name: "engine-door-reads", door_reads: &["seller-order-label"], ..table() };
    let list = mcp::listed(&reads);
    let label = tool(&list, "seller-order-label");
    assert_eq!(hint(label, "readOnlyHint"), Some(true), "offered as a read");
    assert_eq!(required_names(label).first().map(String::as_str), Some("desktop"), "its home still required");
    let home = std::env::temp_dir().join("h");
    let given = vec![("desktop".to_string(), Value::Str(home.display().to_string()))];
    let through = mcp::argv_of(&reads, "seller-order-label", &given, false).expect("a tool").expect("an argv");
    assert_eq!(&through[..3], ["seller-order-label", "--desktop", &home.display().to_string()], "through the desktop, as a write goes");
    let alone = mcp::argv_of(&reads, "seller-order-label", &given, true).expect("a tool").expect("an argv");
    assert_eq!(&alone[..3], ["seller-order-label", "--data", &home.display().to_string()], "with the credential, as a write goes");
    assert_eq!(hint(tool(&mcp::listed(&table()), "seller-order-label"), "readOnlyHint"), Some(false), "none named: it writes");
}

