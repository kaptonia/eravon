
use eravon_literals::constants as k;
use eravon_literals::json::{self, In};

fn lock() -> In {
    let at = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../BASE-LOCK.json");
    let b = std::fs::read(&at).unwrap_or_else(|e| panic!("读不出 {}:{e}", at.display()));
    json::read(&b).unwrap_or_else(|e| panic!("{} 不是 JSON:{e}", at.display()))
}

fn pairs() -> [(&'static str, &'static str); 12] {
    [
        ("logicInitcodeHash", k::LOGIC_INITCODE_HASH),
        ("logic", k::STANDARD_TABLE.logic),
        ("proxyCodeHash", k::STANDARD_TABLE.proxy_code_hash),
        ("panelInitcodeHash", k::PANEL_INITCODE_HASH),
        ("panel", k::PANEL),
        ("panelCodeHash", k::PANEL_CODE_HASH),
        ("verifierInitcodeHash", k::VERIFIER_INITCODE_HASH),
        ("verifier", k::VERIFIER),
        ("verifierCodeHash", k::VERIFIER_CODE_HASH),
        ("boardInitcodeHash", k::BOARD_INITCODE_HASH),
        ("board", k::BOARD),
        ("boardCodeHash", k::BOARD_CODE_HASH),
    ]
}

#[test]
fn the_twelve_constants_are_the_lock_s_twelve_values() {
    let lock = lock();
    let sec = lock.get("standard").expect("锁里没有 standard 节");
    for (key, constant) in pairs() {
        let want = sec.get(key).and_then(In::as_str).unwrap_or_else(|| panic!("锁的 standard 节没有 {key}"));
        assert_eq!(constant, want, "constants 里的 {key} 与锁不等");
    }
}

#[test]
fn the_lock_s_standard_section_has_exactly_these_twelve_keys() {
    let lock = lock();
    let In::Obj(m) = lock.get("standard").expect("锁里没有 standard 节") else { panic!("standard 节不是对象") };
    let mut got: Vec<&str> = m.iter().map(|(key, _)| key.as_str()).collect();
    let mut want: Vec<&str> = pairs().iter().map(|(key, _)| *key).collect();
    got.sort_unstable();
    want.sort_unstable();
    assert_eq!(got, want);
}
