pub fn text_sha256(text_md: &str) -> String {
    eravon_galeed::hash::hex_encode(&eravon_galeed::hash::sha256(text_md.as_bytes()))
}

pub fn import_text(path: &std::path::Path, _current: &str) -> Result<String, String> {
    let raw = std::fs::read(path).map_err(|e| e.to_string())?;
    String::from_utf8(raw).map_err(|e| e.to_string())
}

pub fn char_count(text_md: &str) -> usize {
    text_md.chars().count()
}

pub fn human(terms_md: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let (mut titled, mut fenced) = (false, false);
    for l in terms_md.lines() {
        if l.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        if !titled && l.starts_with("# ") {
            titled = true;
            continue;
        }
        out.push(l);
    }
    out.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_imported_file_is_the_text_and_its_digest() {
        let dir = std::env::temp_dir().join(format!("eravon-text-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let ok = dir.join("a.md");
        std::fs::write(&ok, "# 标题\n\n- 一项\n").expect("write");
        let got = import_text(&ok, "旧的").expect("reads");
        assert_eq!(got, "# 标题\n\n- 一项\n");
        assert_eq!(text_sha256(&got), eravon_galeed::hash::hex_encode(&eravon_galeed::hash::sha256(&std::fs::read(&ok).expect("read"))));
        assert_eq!(char_count(&got), 11);
        let bad = dir.join("b.md");
        std::fs::write(&bad, [0xff, 0xfe, 0x00]).expect("write");
        assert!(import_text(&bad, "旧的").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_human_text_is_the_body_without_title_and_block() {
        let doc = "# 海报\n\n```eravon.terms/1\n{}\n```\n\n## 说明\n\n一段\n\n## 交付\n\n3 天内交付\n";
        assert_eq!(human(doc), "## 说明\n\n一段\n\n## 交付\n\n3 天内交付");
    }
}
