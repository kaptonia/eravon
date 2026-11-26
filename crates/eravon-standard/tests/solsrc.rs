#[path = "../solsrc.rs"]
mod solsrc;

fn read(src: &str) -> Vec<solsrc::Item> {
    solsrc::read(&[("t.sol".into(), src.into())])
}

fn sig(items: &[solsrc::Item], name: &str) -> String {
    items.iter().find(|i| i.name == name).unwrap_or_else(|| panic!("{name} read")).sig.clone()
}

#[test]
fn a_struct_is_its_fields_an_enum_is_uint8_and_a_contract_is_an_address() {
    let items = read(
        "contract A { struct In { uint16 x; uint8 y; } struct Out { address k; In[] ins; bytes32 h; } enum E { P, Q }\n\
         function f(Out calldata o, bytes calldata b, E e, A a) external {}\n\
         function g(A.Out memory o) public view returns (uint256) { return 0; }\n\
         function h(uint x) internal {}\n\
         function i(address[] calldata, bytes32) external pure {} }",
    );
    assert_eq!(sig(&items, "f"), "f((address,(uint16,uint8)[],bytes32),bytes,uint8,address)");
    assert_eq!(sig(&items, "g"), "g((address,(uint16,uint8)[],bytes32))");
    assert!(items.iter().all(|i| i.name != "h"), "internal functions are no ABI");
    assert_eq!(sig(&items, "i"), "i(address[],bytes32)");
}

#[test]
fn a_public_variable_is_its_getter() {
    let items = read(
        "contract B { mapping(address => mapping(address => bool)) public funds; address public immutable LOGIC;\n\
         uint256 internal constant N = 1_000; bytes32 public constant K = keccak256(\"a=b\"); uint64[] public xs;\n\
         mapping(address => uint256) internal hidden; }",
    );
    assert_eq!(sig(&items, "funds"), "funds(address,address)");
    assert_eq!(sig(&items, "LOGIC"), "LOGIC()");
    assert_eq!(sig(&items, "K"), "K()");
    assert_eq!(sig(&items, "xs"), "xs(uint256)");
    assert!(items.iter().all(|i| i.name != "N" && i.name != "hidden"));
}

#[test]
fn an_event_carries_its_indexed_row_and_comments_and_strings_are_not_read() {
    let items = read(
        "contract C {\n// event Fake(uint256 a);\n/* function ghost() external {} */\n\
         string internal constant S = \"function nope() external {}\";\n\
         event Posted(address indexed poster, bytes32 indexed key, bytes32 indexed hash, bytes document);\n\
         error BadTerms(uint8 guard); }",
    );
    let p = items.iter().find(|i| i.name == "Posted").expect("Posted read");
    assert_eq!(p.sig, "Posted(address,bytes32,bytes32,bytes)");
    assert_eq!(p.indexed, vec![true, true, true, false]);
    assert_eq!(sig(&items, "BadTerms"), "BadTerms(uint8)");
    assert!(items.iter().all(|i| i.name != "Fake" && i.name != "ghost" && i.name != "nope"));
}

#[test]
fn returns_are_a_tuple_with_their_names() {
    let items = read(
        "contract D { enum P { A } function status() external view returns (P phase, uint256 seats, bytes32 panelHash_) { }\n\
         function bare() external {} function anon() external view returns (address, uint64) { } }",
    );
    let st = items.iter().find(|i| i.name == "status").expect("status read");
    assert_eq!(st.returns, "(uint8,uint256,bytes32)");
    assert_eq!(st.return_names, vec!["phase".to_string(), "seats".into(), "panelHash_".into()]);
    assert_eq!(items.iter().find(|i| i.name == "bare").expect("bare").returns, "()");
    let an = items.iter().find(|i| i.name == "anon").expect("anon");
    assert_eq!((an.returns.as_str(), an.return_names.clone()), ("(address,uint64)", vec![String::new(), String::new()]));
}

#[test]
fn names_turn_to_upper_snake() {
    assert_eq!(solsrc::upper_snake("acceptFunding"), "ACCEPT_FUNDING");
    assert_eq!(solsrc::upper_snake("PROXY_CODEHASH"), "PROXY_CODEHASH");
    assert_eq!(solsrc::upper_snake("panelInitCodeHash"), "PANEL_INIT_CODE_HASH");
    assert_eq!(solsrc::upper_snake("IGaleedVenue"), "I_GALEED_VENUE");
    assert_eq!(solsrc::upper_snake("EravonPanelBook"), "ERAVON_PANEL_BOOK");
}

#[test]
fn a_structs_tuple_is_its_fields() {
    let s = solsrc::structs(&[("t.sol".into(), "contract E { struct O { uint16 a; uint8 b; } struct F { address k; O[] os; } }".into())]);
    let names = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<String>>();
    assert_eq!(
        s,
        vec![
            ("E".into(), "O".into(), "(uint16,uint8)".into(), names(&["a", "b"])),
            ("E".into(), "F".into(), "(address,(uint16,uint8)[])".into(), names(&["k", "os"])),
        ]
    );
}

#[test]
fn a_file_level_struct_and_an_assembly_body_read() {
    let src = "struct R { bytes32 c; address[] p; }\n\
               interface I { function route(R calldata v) external view returns (uint256 index); }\n\
               contract C { function f() external { assembly (\"memory-safe\") { let x := 1 } } struct S { uint8 a; R r; } function g(S calldata s) external {} }";
    let items = solsrc::read(&[("V.sol".into(), src.into())]);
    assert_eq!(sig(&items, "route"), "route((bytes32,address[]))");
    assert_eq!(sig(&items, "g"), "g((uint8,(bytes32,address[])))");
    assert!(items.iter().any(|i| i.name == "f"));
    let st = solsrc::structs(&[("V.sol".into(), src.into())]);
    assert!(st.iter().any(|(c, n, t, _)| c == "V" && n == "R" && t == "(bytes32,address[])"), "{st:?}");
}
