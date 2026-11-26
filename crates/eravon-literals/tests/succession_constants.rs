use eravon_literals::constants as k;

fn base(rel: &str) -> String {
    let at = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../base").join(rel);
    std::fs::read_to_string(&at).unwrap_or_else(|e| panic!("读不出 {}:{e}", at.display()))
}

fn sol_constant(src: &str, name: &str) -> String {
    let pat = format!(" {name} = ");
    let line = src.lines().find(|l| l.contains("constant") && l.contains(&pat)).unwrap_or_else(|| panic!("源里没有常量 {name}"));
    line.split(&pat).nth(1).and_then(|v| v.split(';').next()).expect("常量的值").trim().to_string()
}

#[test]
fn the_book_and_the_route_gas_are_the_standards_own() {
    let src = base("eravon-standard/src/Eravon.sol");
    assert_eq!(sol_constant(&src, "SUCCESSION_BOOK"), k::SUCCESSION_BOOK);
    assert_eq!(sol_constant(&src, "ROUTE_GAS").replace('_', ""), k::ROUTE_GAS.to_string());
}

#[test]
fn the_book_code_hash_is_the_bases() {
    let md = base("galeed-core/contracts/CODEHASH.md");
    let at = md.find("GaleedSuccessionBook").expect("CODEHASH.md 写着换钥簿");
    let line = md[at..].lines().find(|l| l.starts_with("codeHash")).expect("换钥簿的 codeHash 行");
    let value = line.split('`').nth(1).expect("反引号里的值");
    assert_eq!(value, k::BOOK_CODE_HASH);
}

fn lock_book() -> (String, Vec<u64>) {
    let at = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../BASE-LOCK.json");
    let lock = std::fs::read_to_string(&at).unwrap_or_else(|e| panic!("读不出 {}:{e}", at.display()));
    let book = lock.split("\"successionBook\"").nth(1).expect("锁里有 galeedContracts.successionBook");
    let book = &book[..book.find('}').expect("那一键是一张表")];
    let address = book.split("\"address\"").nth(1).and_then(|r| r.split('"').nth(1)).expect("那一张表有 address").to_string();
    let chains = book.split("\"chains\"").nth(1).and_then(|r| r.split('[').nth(1)).and_then(|r| r.split(']').next()).expect("那一张表有 chains");
    let chains = chains.split(',').map(|c| c.trim().parse::<u64>().expect("链号是整数")).collect();
    (address, chains)
}

#[test]
fn the_locked_book_is_the_products_on_every_network_it_knows() {
    let (address, chains) = lock_book();
    assert!(address.eq_ignore_ascii_case(k::SUCCESSION_BOOK), "锁说 {address},字面说 {}", k::SUCCESSION_BOOK);
    for n in k::NETWORKS {
        assert!(chains.contains(&n.standard.chain_id), "已知网 {} 的链号 {} 不在锁的 successionBook.chains {chains:?} 里", n.name, n.standard.chain_id);
    }
}
