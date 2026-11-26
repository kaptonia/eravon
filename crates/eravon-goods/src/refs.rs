pub const FENCE_INFO: &str = "eravon.goods-refs/1";

const FENCE: &str = "```";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub sha256: String,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Refs {
    pub page: Option<String>,
    pub images: Vec<Image>,
}

impl Refs {
    pub fn is_empty(&self) -> bool {
        self.page.is_none() && self.images.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Read {
    pub body: String,
    pub refs: Refs,
}

fn is_open(line: &str) -> bool {
    line.strip_prefix(FENCE).is_some_and(|rest| rest == FENCE_INFO)
}

fn digest_ok(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub fn read(text: &str) -> Read {
    let plain = || Read { body: text.to_string(), refs: Refs::default() };
    let lines: Vec<&str> = text.split('\n').collect();
    let Some(open) = lines.iter().rposition(|l| is_open(l)) else { return plain() };
    let mut refs = Refs::default();
    let mut close = None;
    for (i, l) in lines.iter().enumerate().skip(open + 1) {
        if *l == FENCE {
            close = Some(i);
            break;
        }
        if l.is_empty() {
            continue;
        }
        if let Some(rest) = l.strip_prefix("page ") {
            if refs.page.is_some() || rest.is_empty() || rest.chars().any(char::is_whitespace) {
                return plain();
            }
            refs.page = Some(rest.to_string());
        } else if let Some(rest) = l.strip_prefix("image ") {
            let Some((digest, name)) = rest.split_once(' ') else { return plain() };
            if !digest_ok(digest) || name.trim().is_empty() {
                return plain();
            }
            refs.images.push(Image { sha256: digest.to_string(), name: name.to_string() });
        } else {
            return plain();
        }
    }
    let Some(close) = close else { return plain() };
    if lines.iter().skip(close + 1).any(|l| !l.trim().is_empty()) {
        return plain();
    }
    let body = lines[..open].join("\n").trim_end().to_string();
    Read { body, refs }
}

pub fn compose(body: &str, refs: &Refs) -> String {
    let base = read(body).body;
    let base = base.trim_end().to_string();
    if refs.is_empty() {
        return base;
    }
    let mut out = base;
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(FENCE);
    out.push_str(FENCE_INFO);
    out.push('\n');
    if let Some(p) = &refs.page {
        out.push_str("page ");
        out.push_str(p);
        out.push('\n');
    }
    for im in &refs.images {
        out.push_str("image ");
        out.push_str(&im.sha256);
        out.push(' ');
        out.push_str(&im.name);
        out.push('\n');
    }
    out.push_str(FENCE);
    out
}

pub fn page_of(entered: &str) -> Result<Option<String>, ()> {
    let p = entered.trim();
    if p.is_empty() {
        return Ok(None);
    }
    if p.chars().any(char::is_whitespace) {
        return Err(());
    }
    Ok(Some(p.to_string()))
}

pub fn digest(bytes: &[u8]) -> String {
    eravon_galeed::hash::sha256(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

pub fn name_of(file: &str) -> Option<String> {
    let n = file.replace(['\n', '\r'], " ");
    let n = n.trim();
    (!n.is_empty()).then(|| n.to_string())
}

pub fn check<'a>(refs: &'a Refs, bytes: &[u8]) -> Option<&'a Image> {
    let d = digest(bytes);
    refs.images.iter().find(|i| i.sha256 == d)
}

#[cfg(test)]
mod tests {
    use super::*;

    const H1: &str = "cee92b3792354def8ff020d77c7496d9615c3193bd7f9e12a5d508ddc01e5173";
    const H2: &str = "03a65a8875d330a90799bbabc6fbb330b9a04ee9726d9e0c1663e2979d02df53";

    #[test]
    fn a_text_without_the_block_carries_nothing_and_stays_whole() {
        let r = read("海报一张\n\n- A2");
        assert_eq!(r.refs, Refs::default());
        assert_eq!(r.body, "海报一张\n\n- A2");
        for t in ["说明\n```rust\nfn main() {}\n```", "0xcee92b3792354def8ff020d77c7496d9615c3193bd7f9e12a5d508ddc01e5173"] {
            assert_eq!(read(t), Read { body: t.to_string(), refs: Refs::default() });
        }
    }

    #[test]
    fn what_the_editor_writes_reads_back_the_same_names_with_spaces_and_chinese_included() {
        let refs = Refs { page: Some("https://north.studio/shop/brand".into()), images: vec![Image { sha256: H1.into(), name: "标志样稿 · 横版.png".into() }, Image { sha256: H2.into(), name: "front view  2.png".into() }] };
        let text = compose("## 品牌标识\n\n三稿\n", &refs);
        assert!(text.ends_with("```"), "{text}");
        let r = read(&text);
        assert_eq!(r.refs, refs.clone());
        assert_eq!(r.body, "## 品牌标识\n\n三稿");
        let again = compose(&text, &refs);
        assert_eq!(again, text);
        assert_eq!(again.matches(FENCE_INFO).count(), 1);
        assert_eq!(compose(&text, &Refs::default()), "## 品牌标识\n\n三稿");
    }

    #[test]
    fn a_block_that_does_not_read_is_plain_text() {
        for t in [
            format!("x\n\n```{FENCE_INFO}\npage https://a.example\npage https://b.example\n```"),
            format!("```{FENCE_INFO}\nimage {} a.png\n```", &H1[..63]),
            format!("```{FENCE_INFO}\nimage {H1}\n```"),
            format!("```{FENCE_INFO}\nlink https://a\n```"),
            format!("a\n```{FENCE_INFO}\npage https://a"),
            format!("```{FENCE_INFO}\npage https://a\n```\nmore"),
            format!("```{FENCE_INFO}\npage https://a b\n```"),
        ] {
            assert_eq!(read(&t), Read { body: t.clone(), refs: Refs::default() }, "{t}");
        }
        let ok = format!("```{FENCE_INFO}\n\npage https://a.example\n\n```\n\n");
        assert_eq!(read(&ok).refs.page.as_deref(), Some("https://a.example"));
    }

    #[test]
    fn the_editor_cells_take_a_page_and_a_picture_as_the_block_writes_them() {
        assert_eq!(page_of("  https://a.example/x  "), Ok(Some("https://a.example/x".into())));
        assert_eq!(page_of("   "), Ok(None));
        assert!(page_of("https://a b").is_err());
        assert_eq!(name_of("标志 横版.png"), Some("标志 横版.png".into()));
        assert_eq!(name_of("a\nb.png"), Some("a b.png".into()));
        assert_eq!(name_of(" \n "), None);
        assert_eq!(digest(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        let refs = Refs { page: None, images: vec![Image { sha256: digest(b"abc"), name: "a.png".into() }] };
        assert_eq!(check(&refs, b"abc").map(|i| i.name.as_str()), Some("a.png"));
        assert_eq!(check(&refs, b"abd"), None);
    }

    #[test]
    fn the_editor_writes_the_body_and_the_block_apart_and_keeps_a_broken_block_as_text() {
        let mut d = crate::draft::Draft::default();
        d.set("description", "一张海报");
        assert!(d.set(crate::draft::PAGE, " https://shop.example/a "));
        assert!(d.add_image("正面 图.png", b"abc"));
        assert_eq!(d.page(), "https://shop.example/a");
        d.set_body("两张海报\n");
        assert_eq!(d.body(), "两张海报");
        assert_eq!(d.refs().refs.images[0].name, "正面 图.png");
        assert!(!d.set(crate::draft::PAGE, "https://a b"), "a blank inside is not taken");
        assert_eq!(d.page(), "https://shop.example/a");
        assert!(d.remove_image(0));
        assert!(!d.remove_image(0));
        assert!(d.set(crate::draft::PAGE, ""));
        assert_eq!(d.description, "两张海报", "nothing carried, no block");
        d.set("description", &format!("x\n```{FENCE_INFO}\nbogus\n```"));
        assert!(d.set_page("https://a.example"));
        assert!(d.description.contains("bogus"), "the broken block stays as the seller's text");
        assert_eq!(d.page(), "https://a.example");
    }
}
