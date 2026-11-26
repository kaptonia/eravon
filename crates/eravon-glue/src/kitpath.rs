pub fn is_kit_path(p: &str) -> bool {
    let b = p.as_bytes();
    if b.is_empty() || b.len() > 1024 || b[0] == b'/' {
        return false;
    }
    p.split('/').all(|seg| {
        let s = seg.as_bytes();
        !s.is_empty()
            && s.len() <= 255
            && seg != "."
            && seg != ".."
            && s[0] != b'-'
            && s.iter().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'.' | b'_' | b'-'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_kit_paths_pass_and_none_climbs_out() {
        for ok in ["manifest.json", "files/a.txt", "entries/0a.zk1", "files/notes/delivery.txt", "a-b_c.d", &"x".repeat(255)] {
            assert!(is_kit_path(ok), "{ok}");
        }
        let at_most = ["a".repeat(255), "b".repeat(255), "c".repeat(255), "d".repeat(254), "e".to_string()].join("/");
        assert_eq!(at_most.len(), 1024);
        let long = format!("{at_most}f");
        for bad in ["", "/etc/passwd", "../x", "files/../../x", "files/./x", "files//x", "files/", "-x", "files/-x", "Files/a", "files/a b", "files/交付.txt", &"x".repeat(256), long.as_str()] {
            assert!(!is_kit_path(bad), "{bad:?}");
        }
        assert!(is_kit_path(&at_most), "1024 in all passes");
    }
}
