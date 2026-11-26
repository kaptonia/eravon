use eravon_galeed as k1;
use eravon_literals::registry::DERIVATION_PATHS;

use crate::vfault::{Fault, R};

fn project_segment() -> R<String> {
    let first = DERIVATION_PATHS
        .first()
        .ok_or_else(|| Fault::Path("K7 的派生表是空的".into()))?;
    first
        .path
        .split('/')
        .nth(3)
        .map(str::to_string)
        .ok_or_else(|| Fault::Path(format!("K7 的派生表上这一行不成形:{}", first.path)))
}

pub fn path_of(role: &str, i: u32) -> R<String> {
    let row = DERIVATION_PATHS
        .iter()
        .find(|r| r.role == role)
        .ok_or_else(|| Fault::Path(format!("K7 的派生表上没有角色 {role}")))?;
    Ok(row.path.replace("{i}", &i.to_string()))
}

pub const ROLE_SELLER: &str = "seller-reputation";
pub const ROLE_BUYER: &str = "buyer";

pub fn at(seed: &[u8], path: &str) -> R<[u8; 32]> {
    guard(path)?;
    raw(path, seed)
}

fn guard(path: &str) -> R<()> {
    let want = project_segment()?;
    let got = path
        .split('/')
        .nth(3)
        .ok_or_else(|| Fault::Path(format!("这一条路径不成形:{path}")))?;
    if got != want {
        return Err(Fault::Path(format!(
            "本端只派项目号 {want} 那一支;{path} 的第三段是 {got},那一支不是本场所的(三钥互不混用)"
        )));
    }
    Ok(())
}

fn raw(path: &str, seed: &[u8]) -> R<[u8; 32]> {
    crate::mark::mark(crate::mark::FEATURE);
    if k1::words::parse_path(path).is_none() {
        return Err(Fault::Path(format!("这一条路径读不成:{path}")));
    }
    let ext = k1::words::derive_path(seed, path)
        .ok_or_else(|| Fault::Path(format!("按 {path} 派不出钥")))?;
    Ok(ext.key)
}

pub fn address(key: &[u8; 32]) -> R<String> {
    let k = k1::sign::Privkey::from_bytes(key)
        .ok_or_else(|| Fault::Path("这三十二字节不是一枚 secp256k1 私钥".into()))?;
    Ok(format!("0x{}", k1::hash::hex_encode(&k.address())[2..].to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHRASE: &str = "test test test test test test test test test test test junk";

    fn seed() -> [u8; 64] {
        k1::words::seed(PHRASE, "").expect("参照助记词读得出种子")
    }

    #[test]
    fn the_table_lands_on_the_addresses_the_family_reference_pins() {
        let s = seed();
        let one = |role: &str, i: u32| {
            let p = path_of(role, i).unwrap();
            address(&at(&s, &p).unwrap()).unwrap()
        };
        assert_eq!(one(ROLE_SELLER, 0), "0x98e503f35d0a019cb0a251ad243a4ccfcf371f46", "卖家信誉钥");
        assert_eq!(one(ROLE_BUYER, 0), "0x60526c65a346c9218f9b8bd124444f826f792780", "买家钥");
    }

    #[test]
    fn the_two_foreign_slots_are_refused_by_name() {
        let s = seed();
        let e = at(&s, "m/44'/60'/0'/0/0").unwrap_err();
        assert_eq!(e.name(), "E_VAULT_PATH");
        assert!(at(&s, "m/44'/60'/1'/0/0").is_err());
        assert!(at(&s, "m/44'/60'/1'/0/1").is_err());
        assert!(at(&s, "m/44'/60'/2'/0/0").is_ok());
    }

    #[test]
    fn the_paths_come_from_the_register_not_from_here() {
        assert_eq!(path_of(ROLE_SELLER, 0).unwrap(), "m/44'/60'/2'/0/0");
        assert_eq!(path_of(ROLE_BUYER, 0).unwrap(), "m/44'/60'/2'/1/0");
        assert!(path_of("buyer-case", 3).is_err(), "表上没有逐案钥那一行");
        assert!(path_of("judge", 0).is_err(), "表上没有的角色即拒");
    }
}
