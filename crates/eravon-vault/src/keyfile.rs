use eravon_store::vault::IdentityKind;

use crate::vfault::{Fault, R};
use crate::key::Secret;
use crate::keystore;

pub const PASSWORD_MAX: usize = 1024;

pub fn need_password(pw: &str) -> R<()> {
    if pw.len() > PASSWORD_MAX {
        return Err(Fault::PasswordLong(PASSWORD_MAX.to_string()));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyFile {
    pub address: Option<String>,
}

pub fn shape(bytes: &[u8]) -> R<KeyFile> {
    let s = keystore::shape(bytes)?;
    let bad = s.unshaped();
    if !bad.is_empty() {
        return Err(Fault::from(crate::fault::Fault::known(crate::fault::Known::KeystoreShape, crate::lang::both_of(|| crate::lang::filln(crate::lang::Key::Tail169, &[&bad.join(" ")])))));
    }
    Ok(KeyFile { address: s.address.map(|a| a.hex()) })
}

pub struct Opened {
    pub address: String,
    pub key: Secret,
    pub kind: IdentityKind,
}

pub fn accept(bytes: &[u8], file_passcode: &str) -> R<Opened> {
    need_password(file_passcode)?;
    let key = keystore::decrypt(bytes, file_passcode)?;
    let address = key.address().map(|a| a.hex()).ok_or_else(|| Fault::KeyFileBroken(crate::lang::t(crate::lang::Key::Tail013).into()))?;
    Ok(Opened { address, key, kind: IdentityKind::Existing })
}

impl std::fmt::Debug for Opened {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Opened").field("address", &self.address).field("kind", &self.kind).finish_non_exhaustive()
    }
}

pub fn write_params() -> keystore::Params {
    crate::keybox::params()
}

pub fn write(secret: &[u8; 32], file_passcode: &str) -> R<Vec<u8>> {
    need_password(file_passcode)?;
    let key = Secret::take(*secret).ok_or_else(|| Fault::KeyFileBroken(crate::lang::t(crate::lang::Key::Tail176).into()))?;
    Ok(keystore::encrypt(&key, file_passcode, write_params(), 0)?.json)
}

pub fn landed(path: &str, address: &str) -> R<()> {
    let back = std::fs::read(path).map_err(|e| Fault::BackupNotLanded(format!("{path}:读不回:{e}")))?;
    let s = keystore::shape(&back).map_err(|f| Fault::BackupNotLanded(format!("{path}:{}", f.tail())))?;
    let bad = s.compliant();
    if !bad.is_empty() {
        return Err(Fault::BackupNotLanded(format!("{path}:{}", bad.join(" "))));
    }
    match &s.address {
        Some(a) if a.hex().eq_ignore_ascii_case(address) => Ok(()),
        _ => Err(Fault::BackupNotLanded(format!("{path}:{} ≠ {address}", s.address_text))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::derive;

    fn light() {
        crate::kdf::set_light(None);
    }

    const KEY: [u8; 32] = [0x42; 32];

    fn key_address() -> Option<crate::key::Address> {
        Secret::take(KEY).and_then(|s| s.address())
    }

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    fn v3(pw: &str, p: keystore::Params) -> Vec<u8> {
        keystore::encrypt(&Secret::take(KEY).unwrap(), pw, p, 0).unwrap().json
    }

    fn opened(bytes: &[u8], pw: &str) -> R<Option<crate::key::Address>> {
        keystore::decrypt(bytes, pw).map(|k| k.address()).map_err(Fault::from)
    }

    const LIGHT: keystore::Params = keystore::Params { n: 1024, r: 8, p: 1 };

    #[test]
    fn a_file_written_here_is_v3_scrypt_and_reads_back() {
        let bytes = v3("file-pass-8", LIGHT);
        let s = keystore::shape(&bytes).unwrap();
        assert!(s.compliant().is_empty(), "{:?}", s.compliant());
        assert_eq!((s.kdf.as_str(), s.cipher.as_str(), s.version, s.n, s.r, s.p), ("scrypt", "aes-128-ctr", 3, 1024, 8, 1));
        assert_eq!(opened(&bytes, "file-pass-8").unwrap(), key_address());
        assert_eq!(s.address.map(|a| a.hex()), Some(derive::address(&KEY).unwrap()));
        assert!(!String::from_utf8_lossy(&bytes).contains(&hex(&KEY)), "明文私钥不落档");
        assert_eq!(keystore::Params::standard(), keystore::Params { n: 1 << 18, r: 8, p: 1 }, "the standard level, from its one source");
    }

    #[test]
    fn a_wrong_password_fails_at_the_mac_and_a_contradicting_address_is_named() {
        let bytes = v3("file-pass-8", LIGHT);
        assert_eq!(opened(&bytes, "file-pass-9").unwrap_err(), Fault::Sealed);
        let other = String::from_utf8(bytes).unwrap();
        let addr = keystore::shape(other.as_bytes()).unwrap().address_text;
        let swapped = other.replace(&addr, &"1".repeat(40));
        assert_eq!(opened(swapped.as_bytes(), "file-pass-8").unwrap_err().name(), "E_VAULT_KEYFILE_BROKEN");
    }

    fn pbkdf2_file(c: u64) -> Vec<u8> {
        let (salt, iv) = ([7u8; 32], [9u8; 16]);
        let mut dk = [0u8; keystore::DKLEN];
        if c <= 16 {
            assert!(crate::cryptx::pbkdf2_sha256(b"file-pass-8", &salt, c as u32, &mut dk));
        }
        let mut ct = KEY.to_vec();
        crate::cryptx::aes128_ctr(dk[..16].try_into().expect("十六字节"), &iv, &mut ct);
        let mac = keystore::mac_of(&dk, &ct);
        let addr = derive::address(&KEY).unwrap();
        format!(
            r#"{{"address":"{}","crypto":{{"cipher":"aes-128-ctr","ciphertext":"{}","cipherparams":{{"iv":"{}"}},"kdf":"pbkdf2","kdfparams":{{"c":{c},"dklen":32,"prf":"hmac-sha256","salt":"{}"}},"mac":"{}"}},"id":"id","version":3}}"#,
            addr.trim_start_matches("0x"),
            hex(&ct),
            hex(&iv),
            hex(&salt),
            hex(&mac)
        )
        .into_bytes()
    }

    #[test]
    fn a_pbkdf2_file_reads_and_its_round_count_is_bounded_both_ways() {
        let file = pbkdf2_file;
        assert_eq!(opened(&file(2), "file-pass-8").unwrap(), key_address());
        assert!(keystore::c_out_of_bounds(1).is_empty() && keystore::c_out_of_bounds(keystore::MAX_C).is_empty());
        assert_eq!(keystore::c_out_of_bounds(keystore::MAX_C + 1), vec!["c"]);
        assert_eq!(keystore::c_out_of_bounds(0), vec!["c"]);
        assert_eq!(keystore::MAX_C, 16_777_216);
        assert_eq!(opened(&file(keystore::MAX_C + 1), "file-pass-8").unwrap_err().name(), "E_VAULT_KEYSTORE_PARAMS");
    }

    #[test]
    fn the_scrypt_table_names_each_bound_it_breaks() {
        use keystore::{in_range, n_shaped_for, out_of_bounds};
        assert!(in_range(1 << 18, 8, 1));
        assert!(in_range(1 << 20, 8, 1), "内存恰 2^30");
        assert_eq!(out_of_bounds(1 << 21, 8, 1), vec!["memory"], "内存 2^31 越界,总功 2^31 在界内");
        assert_eq!(out_of_bounds(1 << 21, 8, 4), vec!["memory", "work"]);
        assert!(out_of_bounds(1 << 18, 8, 16).is_empty(), "p 16、总功恰 2^32");
        assert_eq!(out_of_bounds(1 << 18, 8, 17), vec!["p", "work"]);
        assert_eq!(out_of_bounds(1024, 0, 1), vec!["r"]);
        assert_eq!(out_of_bounds(usize::MAX, usize::MAX, 1), vec!["memory", "work"], "乘不进即越界,不绕回");
        assert!(!n_shaped_for(1000, 8) && !n_shaped_for(1, 8));
    }

    #[test]
    fn each_malformed_member_is_named_before_any_work() {
        let good = String::from_utf8(v3("file-pass-8", LIGHT)).unwrap();
        for (from, to, member) in [
            ("\"version\":3", "\"version\":4", "version"),
            ("\"cipher\":\"aes-128-ctr\"", "\"cipher\":\"aes-256-ctr\"", "cipher"),
            ("\"kdf\":\"scrypt\"", "\"kdf\":\"argon2\"", "kdf"),
            ("\"dklen\":32", "\"dklen\":16", "dklen"),
            ("\"n\":1024", "\"n\":1000", "n"),
        ] {
            let bad = good.replace(from, to);
            let e = opened(bad.as_bytes(), "file-pass-8").unwrap_err();
            assert!(e.name() == "KEYSTORE_SHAPE" && e.detail().contains(member), "{member}:{e:?}");
        }
        assert_eq!(opened(b"not json", "x").unwrap_err().name(), "KEYSTORE_SHAPE");
        let with = |doc: &str, member: &str, value: &str| -> String {
            let head = format!("\"{member}\":\"");
            let at = doc.find(&head).expect("member there") + head.len();
            let end = at + doc[at..].find('"').expect("closing quote");
            format!("{}{value}{}", &doc[..at], &doc[end..])
        };
        let mac_short = { let i = good.find("\"mac\":\"").unwrap() + 7; let e = i + good[i..].find('"').unwrap(); with(&good, "mac", &good[i..e - 2]) };
        let text_short = { let i = good.find("\"ciphertext\":\"").unwrap() + 14; let e = i + good[i..].find('"').unwrap(); with(&good, "ciphertext", &good[i..e - 2]) };
        for (bad, member) in [
            (with(&good, "salt", ""), "salt"),
            (with(&good, "iv", "00"), "iv"),
            (mac_short, "mac"),
            (text_short, "ciphertext"),
            (with(&good, "address", "zz"), "address"),
        ] {
            let e = opened(bad.as_bytes(), "file-pass-8").unwrap_err();
            assert!(e.name() == "KEYSTORE_SHAPE" && e.detail().contains(member), "{member}:{e:?}");
        }
        let gone = good.replacen("\"crypto\"", "\"cryptx\"", 1);
        let e = opened(gone.as_bytes(), "file-pass-8").unwrap_err();
        assert!(e.name() == "KEYSTORE_SHAPE" && e.detail().contains("crypto"), "{e:?}");
    }

    #[test]
    fn pbkdf2_rounds_of_zero_or_none_are_refused_by_name() {
        let good = String::from_utf8(v3("file-pass-8", LIGHT)).unwrap();
        let i = good.find("\"kdfparams\":{").unwrap();
        let end = i + good[i..].find('}').unwrap() + 1;
        let salt = { let s = good.find("\"salt\":\"").unwrap() + 8; let e = s + good[s..].find('"').unwrap(); good[s..e].to_string() };
        let pb = |params: &str| format!("{}\"kdfparams\":{{{params}}}{}", &good[..i], &good[end..]).replace("\"kdf\":\"scrypt\"", "\"kdf\":\"pbkdf2\"");
        let zero = pb(&format!("\"c\":0,\"dklen\":32,\"prf\":\"hmac-sha256\",\"salt\":\"{salt}\""));
        assert_eq!(opened(zero.as_bytes(), "file-pass-8").unwrap_err().name(), "E_VAULT_KEYSTORE_PARAMS");
        let none = pb(&format!("\"dklen\":32,\"prf\":\"hmac-sha256\",\"salt\":\"{salt}\""));
        let e = opened(none.as_bytes(), "file-pass-8").unwrap_err();
        assert!(e.name() == "KEYSTORE_SHAPE" && e.detail().contains('c'), "{e:?}");
    }

    #[test]
    fn a_keystore_of_the_wrong_shape_is_said_as_that_before_any_work() {
        let good = String::from_utf8(v3("file-pass-8", LIGHT)).unwrap();
        let before = crate::cryptx::derivations();
        assert_eq!(opened(good.as_bytes(), "file-pass-8").unwrap(), Secret::take(KEY).and_then(|s| s.address()));
        assert_eq!(crate::cryptx::derivations(), before + 1, "the counter sees a derivation");
        let refused = |bad: &str, name: &str, what: &str| {
            let at = crate::cryptx::derivations();
            let e = opened(bad.as_bytes(), "file-pass-8").unwrap_err();
            assert_eq!((e.name(), crate::cryptx::derivations()), (name, at), "{what}: {e:?}");
            let e = accept(bad.as_bytes(), "file-pass-8").unwrap_err();
            assert_eq!((e.name(), crate::cryptx::derivations()), (name, at), "{what} (accept): {e:?}");
        };
        for (from, to, what) in [
            ("\"p\":1", "\"p\":\"1\"", "p as text"),
            ("\"p\":1", "\"p\":1.5", "p a fraction"),
            ("\"r\":8", "\"r\":\"8\"", "r as text"),
            ("\"n\":1024", "\"n\":1000", "n not a power of two"),
            ("\"version\":3", "\"version\":4", "version"),
        ] {
            assert!(good.contains(from), "{from}");
            refused(&good.replacen(from, to, 1), "KEYSTORE_SHAPE", what);
        }
        refused(&good.replacen("\"p\":1", "", 1).replace(",,", ",").replace("{,", "{").replace(",}", "}"), "KEYSTORE_SHAPE", "p absent");
        for (from, to, what) in [
            ("\"p\":1", "\"p\":0", "p zero"),
            ("\"p\":1", "\"p\":17", "p past 16"),
            ("\"n\":1024", "\"n\":2097152", "memory past 2^30"),
        ] {
            refused(&good.replacen(from, to, 1), "E_VAULT_KEYSTORE_PARAMS", what);
        }
        refused("not json", "KEYSTORE_SHAPE", "not json");
    }

    #[test]
    fn scrypt_matches_rfc_7914_first_two_vectors() {
        let run = |pw: &[u8], salt: &[u8], n: usize, r: usize, p: usize| {
            let mut dk = [0u8; 32];
            assert!(crate::cryptx::scrypt(pw, salt, n, r, p, &mut dk));
            hex(&dk)
        };
        assert_eq!(run(b"", b"", 16, 1, 1), "77d6576238657b203b19ca42c18a0497f16b4844e3074ae8dfdffa3fede21442");
        assert_eq!(run(b"password", b"NaCl", 1024, 8, 16), "fdbabe1c9d3472007856e7190d01e9fe7c6ad7cbc8237830e77376634b373162");
    }

    #[test]
    fn aes128_ctr_matches_sp_800_38a_and_is_its_own_inverse() {
        let bytes = |h: &str| (0..h.len()).step_by(2).map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let key: [u8; 16] = bytes("2b7e151628aed2a6abf7158809cf4f3c").try_into().unwrap();
        let iv: [u8; 16] = bytes("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff").try_into().unwrap();
        let mut block = bytes("6bc1bee22e409f96e93d7e117393172a");
        crate::cryptx::aes128_ctr(&key, &iv, &mut block);
        assert_eq!(hex(&block), "874d6191b620e3261bef6864990db6ce");
        let clear = "keystore V3 的密文走这一条".as_bytes();
        let mut once = clear.to_vec();
        crate::cryptx::aes128_ctr(&[7u8; 16], &[3u8; 16], &mut once);
        assert_ne!(once, clear, "密文与明文一样即没有加密");
        crate::cryptx::aes128_ctr(&[7u8; 16], &[3u8; 16], &mut once);
        assert_eq!(once, clear, "同一条路走回来即原文");
    }

    #[test]
    fn a_password_of_1024_bytes_passes_and_1025_is_refused_on_both_sides() {
        light();
        let at = "a".repeat(PASSWORD_MAX);
        let over = "a".repeat(PASSWORD_MAX + 1);
        assert!(need_password(&"a".repeat(PASSWORD_MAX - 1)).is_ok());
        let bytes = write(&KEY, &at).unwrap();
        assert_eq!(accept(&bytes, &at).unwrap().key.address(), key_address());
        assert_eq!(write(&KEY, &over).unwrap_err().name(), "E_VAULT_PASSWORD_LONG");
        assert_eq!(accept(&bytes, &over).unwrap_err().name(), "E_VAULT_PASSWORD_LONG");
    }

    #[test]
    fn a_file_holds_a_private_key_and_passes_its_three_gates() {
        light();
        let key = [7u8; 32];
        let addr = derive::address(&key).unwrap();
        let bytes = write(&key, "file-pass-8").unwrap();
        assert_eq!(shape(&bytes).unwrap().address.as_deref(), Some(addr.as_str()));
        let got = accept(&bytes, "file-pass-8").unwrap();
        assert_eq!(got.key.address().map(|a| a.hex()), Some(addr.clone()));
        assert_eq!((got.address, got.kind), (addr, IdentityKind::Existing));
    }

    #[test]
    fn the_three_gates_each_refuse_by_their_own_name() {
        light();
        let key = [7u8; 32];
        let bytes = write(&key, "file-pass-8").unwrap();
        assert_eq!(accept(b"{\"hello\":1}", "file-pass-8").unwrap_err().name(), "KEYSTORE_SHAPE");
        assert_eq!(accept(b"not json at all", "x").unwrap_err().name(), "KEYSTORE_SHAPE");
        assert_eq!(accept(&bytes, "wrong-pass-8").unwrap_err().name(), "E_VAULT_SEALED");
        let text = String::from_utf8(bytes.clone()).unwrap();
        let addr = keystore::shape(&bytes).unwrap().address_text;
        let edited = text.replace(&addr, "00000000000000000000000000000000000000a1");
        assert_eq!(accept(edited.as_bytes(), "file-pass-8").unwrap_err().name(), "E_VAULT_KEYFILE_BROKEN");
    }

    #[test]
    fn a_file_of_anything_but_thirty_two_bytes_is_refused() {
        light();
        let bytes = write(&[7u8; 32], "file-pass-8").unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let ct = text.split("\"ciphertext\":\"").nth(1).unwrap().split('"').next().unwrap().to_string();
        let short = text.replace(&ct, &ct[..32]);
        assert_eq!(accept(short.as_bytes(), "file-pass-8").unwrap_err().name(), "KEYSTORE_SHAPE");
    }

    #[test]
    fn two_exports_of_the_same_key_do_not_look_alike() {
        light();
        let key = [7u8; 32];
        assert_ne!(write(&key, "file-pass-8").unwrap(), write(&key, "file-pass-8").unwrap(), "盐与 iv 每导一次现取");
    }

    #[test]
    fn a_landed_file_is_checked_on_disk_and_each_failure_is_named() {
        light();
        let key = [7u8; 32];
        let addr = derive::address(&key).unwrap();
        let dir = std::env::temp_dir().join(format!("eravon-landed-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("k.json").display().to_string();
        std::fs::write(&path, write(&key, "file-pass-8").unwrap()).unwrap();
        assert!(landed(&path, &addr).is_ok());
        assert_eq!(landed(&path, "0x00000000000000000000000000000000000000a1").unwrap_err().name(), "E_VAULT_BACKUP_NOT_LANDED");
        std::fs::write(&path, b"{\"truncated\":").unwrap();
        assert_eq!(landed(&path, &addr).unwrap_err().name(), "E_VAULT_BACKUP_NOT_LANDED");
        let _ = std::fs::remove_file(&path);
        assert_eq!(landed(&path, &addr).unwrap_err().name(), "E_VAULT_BACKUP_NOT_LANDED");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
