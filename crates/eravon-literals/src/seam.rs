use crate::constants as k;
use crate::literals as lit;

pub fn with_prefix(prefix: &str, body: &str) -> String {
    let mut s = String::with_capacity(prefix.len() + body.len());
    s.push_str(prefix);
    s.push_str(body);
    s
}

pub fn link_scheme(link_b64url: &str) -> String {
    with_prefix(lit::PREFIX_LINK_SCHEME, link_b64url)
}

pub fn receipt_text(receipt_b64url: &str) -> String {
    with_prefix(lit::PREFIX_RECEIPT, receipt_b64url)
}

pub fn quote_key(chain_id: u64, code_hash: &str, token: &str) -> String {
    let mut s = String::new();
    s.push_str(&chain_id.to_string());
    s.push(':');
    s.push_str(code_hash);
    s.push(':');
    s.push_str(token);
    s
}


pub fn logic_salt_preimage() -> Vec<u8> {
    lit::SALT_LOGIC.as_bytes().to_vec()
}

pub fn panel_book_salt_preimage() -> Vec<u8> {
    lit::SALT_PANEL_BOOK.as_bytes().to_vec()
}

pub fn derivation_path(group: u32, index: u32) -> String {
    let mut s = String::from("m/44'/60'/2'/");
    s.push_str(&group.to_string());
    s.push('/');
    s.push_str(&index.to_string());
    s
}

pub fn dossier_index_path(dir: &str) -> String {
    let mut s = String::from(dir);
    if !s.ends_with('/') {
        s.push('/');
    }
    s.push_str(lit::DOSSIER_INDEX_FILE);
    s
}

pub fn file_name_of(path: &str) -> String {
    std::path::Path::new(path.trim()).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn named_under(root: &std::path::Path, name: &str) -> Result<std::path::PathBuf, String> {
    name_portable(name)?;
    let mut out = root.to_path_buf();
    for seg in name.split('/') {
        out.push(seg);
    }
    Ok(out)
}

pub fn name_portable(name: &str) -> Result<(), String> {
    let bad = |why: &str| Err(format!("名字 {name:?} {why},装不进清单,也另存不出来"));
    if name.is_empty() {
        return bad("是空的");
    }
    for seg in name.split('/') {
        match seg {
            "" => return bad("有空段(以 / 开头或连写两个 /)"),
            "." | ".." => return bad("有一段是 . 或 .."),
            s if s.contains('\\') => return bad("带反斜杠(别的系统的分隔符)"),
            s if s.contains(':') => return bad("带冒号(盘符或流名;访达里键入的 / 存成冒号)"),
            s if s.chars().any(char::is_control) => return bad("有一段带控制字符"),
            s if s.ends_with('.') || s.ends_with(' ') => return bad("有一段以点或空格收尾(Windows 会把它去掉,两个名落到同一个档)"),
            s if device(s) => return bad("有一段是 Windows 的设备名(在那里打开的是设备,不是档)"),
            _ => {}
        }
    }
    Ok(())
}


fn device(seg: &str) -> bool {
    let stem = seg.split('.').next().unwrap_or(seg).to_ascii_uppercase();
    match stem.as_str() {
        "CON" | "PRN" | "AUX" | "NUL" => true,
        s => ["COM", "LPT"].iter().any(|p| s.strip_prefix(p).is_some_and(|d| matches!(d, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"))),
    }
}

pub fn self_check() -> Result<(), &'static str> {
    const SAMPLE_CODE_HASH: &str =
        "0x1111111111111111111111111111111111111111111111111111111111111111";
    let key = quote_key(1, SAMPLE_CODE_HASH, "0x0000000000000000000000000000000000000001");
    let want = "1:0x1111111111111111111111111111111111111111111111111111111111111111:0x0000000000000000000000000000000000000001";
    if key != want {
        return Err("E_SEAM_FORM");
    }
    if derivation_path(1, 0) != k::WINDOW_ONE_TIME_KEY_PATH {
        return Err("E_SEAM_PATH");
    }
    if link_scheme("AA") != "eravon://open?e=AA" {
        return Err("E_SEAM_PREFIX");
    }
    if dossier_index_path("/tmp/case") != "/tmp/case/dossier.json" {
        return Err("E_SEAM_FILE");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seams_pass_their_own_self_check() {
        assert_eq!(self_check(), Ok(()));
    }

    #[test]
    fn a_name_from_outside_stays_under_its_root_on_every_system() {
        let root = std::path::Path::new("/r");
        assert_eq!(named_under(root, "a/b.txt").unwrap(), std::path::Path::new("/r/a/b.txt"));
        assert!(named_under(root, "a\\b.txt").is_err(), "反斜杠不收:别的系统的分隔符");
        assert!(name_portable("report 10:04.pdf").is_err(), "访达里键入的 / 存成冒号");
        for bad in ["", "/etc/x", "a//b", "..", "a/../../x", "..\\x", "./a", "C:x", "C:\\x", "\\\\host\\s", "a\u{0}b", "a/"] {
            assert!(named_under(root, bad).is_err(), "{bad:?} 该拒");
        }
        for bad in ["CON", "con.txt", "a/NUL", "Aux.log", "prn", "COM1", "lpt9.pdf", "COM¹", "a.", "a /b", "dir./x"] {
            assert!(name_portable(bad).is_err(), "{bad:?} 该拒:Windows 的设备名或点、空格收尾");
        }
        for good in ["CONFIG.txt", "console.log", "COM10", "LPT0", "nul_report.pdf", "a.b", ".hidden"] {
            assert!(name_portable(good).is_ok(), "{good:?} 该收");
        }
    }

    #[test]
    fn the_three_derived_keys_of_the_family_table_read_back() {
        assert_eq!(derivation_path(0, 0), "m/44'/60'/2'/0/0");
        assert_eq!(derivation_path(1, 0), "m/44'/60'/2'/1/0");
        assert_eq!(derivation_path(1, 7), "m/44'/60'/2'/1/7");
    }

    #[test]
    fn panel_salt_preimage_is_the_one_literal_with_no_guild_after_it() {
        assert_eq!(panel_book_salt_preimage(), b"eravon/1/panel".to_vec());
    }

    #[test]
    fn dossier_index_path_does_not_double_the_separator() {
        assert_eq!(dossier_index_path("/a/b"), "/a/b/dossier.json");
        assert_eq!(dossier_index_path("/a/b/"), "/a/b/dossier.json");
    }
}
