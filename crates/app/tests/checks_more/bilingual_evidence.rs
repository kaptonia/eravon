#[test]
fn a_desktop_refusal_says_its_evidence_in_each_language_and_the_output_keeps_one() {
    use app::fault::{Fault, Known};
    use app::lang::{both_of, t, Key};
    use eravon_cli::door::Reply as Door;
    let (tail_zh, tail_en) = app::lang::both(Key::Tail211);
    for speaks in [app::lang::Lang::Zh, app::lang::Lang::En] {
        app::lang::set(speaks);
        let f = Fault::known(Known::NoHome, both_of(|| t(Key::Tail211).to_string()));
        match app::door::refused(&f) {
            Door::Refused { code, tail, zh, en, .. } => {
                assert_eq!(code, "NO_HOME");
                assert_eq!(tail, tail_en, "{speaks:?}: the output's evidence in the command line's machine words");
                assert!(zh.ends_with(tail_zh), "{speaks:?}: the Chinese line ends with the evidence in Chinese: {zh}");
                assert!(en.ends_with(tail_en), "{speaks:?}: the English line ends with the evidence in English: {en}");
                let (what_zh, what_en) = app::lang::both(f.what_key().unwrap_or(Known::NoHome.what()));
                assert!(zh.starts_with(what_zh.trim_end_matches('。')) && en.starts_with(what_en.trim_end_matches('.')), "the sentence first: {zh} / {en}");
            }
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn a_faults_evidence_of_words_is_kept_in_both_languages() {
    use app::fault::{Fault, Known};
    use app::lang::{both_of, filln, Key, Lang};
    let f = Fault::known(Known::QueueShape, both_of(|| filln(Key::Tail006, &["/q"])));
    let (zh, en) = app::lang::both(Key::Tail006);
    assert_eq!(f.tail_in(Lang::Zh), zh.replace("{0}", "/q"));
    assert_eq!(f.tail_in(Lang::En), en.replace("{0}", "/q"));
    assert_eq!(f.tail(), f.tail_in(app::lang::lang()), "the tail is the one in the language spoken");
    let v = Fault::known(Known::QueueShape, "/q".to_string());
    assert_eq!((v.tail_in(Lang::Zh), v.tail_in(Lang::En)), ("/q", "/q"), "a value is the same in both");
    let before = app::lang::lang();
    let _ = both_of(|| app::lang::t(Key::Tail211));
    assert_eq!(app::lang::lang(), before);
}

#[test]
fn every_worded_evidence_is_made_in_both_languages() {
    let mut bare = Vec::new();
    let mut seen = 0;
    let mut stack = vec![super::src(), std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../eravon-vault/src")];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).expect("src reads").flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            if p.extension().and_then(|x| x.to_str()) != Some("rs") {
                continue;
            }
            let text = std::fs::read_to_string(&p).expect("a source");
            for tail in tails(&text) {
                seen += 1;
                let worded = ["lang::", "filln(", "fill1(", "fill2(", "fill3(", "said_of(", "trouble_said(", "t(Key::"].iter().any(|w| tail.contains(w));
                if worded && !tail.starts_with("crate::lang::both_of(") {
                    bare.push(format!("{}: {tail}", p.display()));
                }
            }
        }
    }
    assert!(seen > 100, "the scan reads the faults ({seen})");
    assert!(bare.is_empty(), "evidence of words in one language only:\n{}", bare.join("\n"));
}

fn tails(text: &str) -> Vec<String> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(at) = text[from..].find("Fault::known(") {
        let open = from + at + "Fault::known(".len() - 1;
        from = open + 1;
        let (mut depth, mut i, mut args, mut start) = (0i32, open, Vec::new(), open + 1);
        while i < b.len() {
            match b[i] {
                b'"' => {
                    i += 1;
                    while i < b.len() && b[i] != b'"' {
                        if b[i] == b'\\' {
                            i += 1;
                        }
                        i += 1;
                    }
                }
                b'\'' if i + 2 < b.len() && (b[i + 2] == b'\'' || (b[i + 1] == b'\\' && i + 3 < b.len() && b[i + 3] == b'\'')) => {
                    i += if b[i + 1] == b'\\' { 3 } else { 2 };
                }
                b'/' if b.get(i + 1) == Some(&b'/') => {
                    while i < b.len() && b[i] != b'\n' {
                        i += 1;
                    }
                }
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        args.push(&text[start..i]);
                        break;
                    }
                }
                b',' if depth == 1 => {
                    args.push(&text[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
            i += 1;
        }
        if let Some(t) = args.get(1) {
            out.push(t.split_whitespace().collect::<Vec<_>>().join(" "));
        }
    }
    out
}
