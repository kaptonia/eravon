#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Status,
    Button,
    Hint,
    Name,
}

impl Kind {
    pub const fn name(self) -> &'static str {
        match self {
            Kind::Status => "status",
            Kind::Button => "button",
            Kind::Hint => "hint",
            Kind::Name => "name",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Word {
    pub key: &'static str,
    pub kind: Kind,
    pub zh: &'static str,
    pub en: &'static str,
}

pub fn words() -> &'static [Word] {
    static WORDS: std::sync::OnceLock<Vec<Word>> = std::sync::OnceLock::new();
    WORDS.get_or_init(|| {
        meta::META
            .iter()
            .map(|(k, name, kind)| {
                let (zh, en) = crate::lang::both(*k);
                Word { key: name, kind: *kind, zh, en }
            })
            .collect()
    })
}

mod alias;
mod meta;
mod shown;
pub use shown::{absent_detail, judge_state, library, order_word, CONSTS};

pub use crate::lang::Key;

pub fn name(k: Key) -> &'static str {
    meta::META.iter().find(|(x, _, _)| *x == k).map(|(_, n, _)| *n).unwrap_or("")
}

pub fn key_of(name: &str) -> Option<Key> {
    meta::META.iter().find(|(_, n, _)| *n == name).map(|(k, _, _)| *k).or_else(|| folded(name))
}

fn folded(name: &str) -> Option<Key> {
    alias::ALIAS.iter().find(|(n, _)| *n == name).map(|(_, k)| *k)
}

fn word_of(name: &str) -> Option<&'static Word> {
    words().iter().find(|w| w.key == name).or_else(|| folded(name).and_then(|k| words().iter().find(|w| w.key == self::name(k)).or_else(|| folded_row(k))))
}

fn folded_row(k: Key) -> Option<&'static Word> {
    static ROWS: std::sync::OnceLock<Vec<(Key, Word)>> = std::sync::OnceLock::new();
    let rows = ROWS.get_or_init(|| {
        alias::ALIAS
            .iter()
            .filter(|(_, k)| self::name(*k).is_empty())
            .map(|(n, k)| {
                let (zh, en) = crate::lang::both(*k);
                (*k, Word { key: n, kind: Kind::Name, zh, en })
            })
            .collect()
    });
    rows.iter().find(|(x, _)| *x == k).map(|(_, w)| w)
}

pub use crate::lang::Lang;

pub const fn lang_name(l: Lang) -> &'static str {
    match l {
        Lang::Zh => "zh",
        Lang::En => "en",
    }
}

pub fn lang_of(s: &str) -> Option<Lang> {
    match s {
        "zh" => Some(Lang::Zh),
        "en" => Some(Lang::En),
        _ => None,
    }
}

pub fn set(lang: Lang) {
    crate::lang::set(lang);
}

pub fn current() -> Lang {
    crate::lang::lang()
}

pub fn t(key: Key) -> String {
    get(current(), key).0
}

pub fn t_named(key: &str) -> String {
    get_named(current(), key).0
}

pub fn filln(key: Key, args: &[&str]) -> String {
    filln_in(current(), key, args)
}

pub fn filln_in(lang: Lang, key: Key, args: &[&str]) -> String {
    fill_phrase(&get(lang, key).0, args)
}

pub fn fill_phrase(tpl: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(tpl.len() + 16);
    let mut rest = tpl;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('}').and_then(|j| after[..j].parse::<usize>().ok().map(|n| (j, n))) {
            Some((j, n)) if n < args.len() => {
                out.push_str(args[n]);
                rest = &after[j + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn arg<S: AsRef<str> + ?Sized>(s: &S) -> &str {
    s.as_ref()
}

pub fn slots(phrase: &str) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    let mut rest = phrase;
    while let Some(i) = rest.find('{') {
        let after = &rest[i + 1..];
        match after.find('}').and_then(|j| after[..j].parse::<usize>().ok().map(|n| (j, n))) {
            Some((j, n)) => {
                if !out.contains(&n) {
                    out.push(n);
                }
                rest = &after[j + 1..];
            }
            None => rest = after,
        }
    }
    out.sort_unstable();
    out
}

pub fn get(lang: Lang, key: Key) -> (String, bool) {
    let (zh, en) = crate::lang::both(key);
    (match lang {
        Lang::Zh => zh,
        Lang::En => en,
    }
    .to_string(), true)
}

pub fn get_named(lang: Lang, key: &str) -> (String, bool) {
    match word_of(key) {
        Some(w) => (pick(w, lang).to_string(), true),
        None => (format!("⟦{key}⟧"), false),
    }
}


fn pick(w: &Word, lang: Lang) -> &'static str {
    match lang {
        Lang::Zh => w.zh,
        Lang::En => w.en,
    }
}

pub fn keys(lang: Lang) -> Vec<&'static str> {
    words().iter().filter(|w| !pick(w, lang).is_empty()).map(|w| w.key).collect()
}

fn diff<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<&'a str> {
    a.iter().filter(|k| !b.contains(k)).copied().collect()
}


pub fn keyset_self_check() -> Result<(), &'static str> {
    let a = ["x", "y"];
    let b = ["x"];
    if diff(&a, &b) != vec!["y"] {
        return Err("E_I18N_KEYSET_BLIND");
    }
    if !diff(&b, &a).is_empty() {
        return Err("E_I18N_KEYSET_FALSE_ALARM");
    }
    Ok(())
}

pub fn only_in(a: Lang, b: Lang) -> Vec<&'static str> {
    diff(&keys(a), &keys(b))
}

fn phrase_ok(s: &str) -> Result<(), &'static str> {
    if s.contains('—') {
        return Err("E_I18N_DASH");
    }
    if s.contains("不是") && s.contains("而是") {
        return Err("E_I18N_NEGATIVE_CONTRAST");
    }
    if s.contains('\n') {
        return Err("E_I18N_MULTILINE");
    }
    Ok(())
}


pub fn checker_self_check() -> Result<(), &'static str> {
    for (s, want) in [
        ("按一下——就好", "E_I18N_DASH"),
        ("不是这一枚,而是那一枚", "E_I18N_NEGATIVE_CONTRAST"),
        ("头一行\n第二行", "E_I18N_MULTILINE"),
    ] {
        match phrase_ok(s) {
            Err(e) if e == want => {}
            _ => return Err("E_I18N_BLIND"),
        }
    }
    if phrase_ok("刷新").is_err() {
        return Err("E_I18N_FALSE_ALARM");
    }
    Ok(())
}

pub fn self_check() -> Result<(), &'static str> {
    if !only_in(Lang::Zh, Lang::En).is_empty() || !only_in(Lang::En, Lang::Zh).is_empty() {
        return Err("E_I18N_KEYSET");
    }
    let mut seen: Vec<&str> = Vec::new();
    for w in words() {
        if seen.contains(&w.key) {
            return Err("E_I18N_DUPLICATE");
        }
        slots_agree(w)?;
        seen.push(w.key);
        if w.kind == Kind::Status {
            let n = w.zh.chars().count();
            if !(2..=5).contains(&n) {
                return Err("E_I18N_STATUS_LENGTH");
            }
        }
        phrase_ok(w.zh)?;
        phrase_ok(w.en)?;
    }
    Ok(())
}

fn slots_agree(w: &Word) -> Result<(), &'static str> {
    let (z, e) = (slots(w.zh), slots(w.en));
    if z != e {
        return Err("E_I18N_SLOTS");
    }
    if z.iter().enumerate().any(|(i, n)| i != *n) {
        return Err("E_I18N_SLOTS");
    }
    let named = |s: &str| s.split('{').skip(1).any(|r| r.split('}').next().is_some_and(|n| !n.is_empty() && n.parse::<usize>().is_err() && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')));
    if named(w.zh) || named(w.en) || w.zh.contains("{}") || w.en.contains("{}") {
        return Err("E_I18N_SLOTS");
    }
    Ok(())
}

pub const LANGS: [Lang; 2] = [Lang::Zh, Lang::En];

pub const FEATURE: eravon_literals::trace::Feature = eravon_literals::trace::Feature::H5;


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_past_every_nodes_limit_on_questions_says_what_happened_and_what_to_change_in_both_languages() {
        for lang in [Lang::Zh, Lang::En] {
            let at = |w: (&'static str, &'static str)| match lang {
                Lang::Zh => w.0,
                Lang::En => w.1,
            };
            assert_eq!(get_named(lang, "err.E_CHAIN_OVER_ASKS"), (at(eravon_glue::book::OVER_ASKS_WORDS).to_string(), true));
            assert_eq!(get_named(lang, "err.E_CHAIN_OVER_ASKS.next"), (at(eravon_glue::book::OVER_ASKS_NEXT_WORDS).to_string(), true));
        }
    }

    #[test]
    fn the_table_passes_its_own_five_questions() {
        assert_eq!(self_check(), Ok(()));
    }

    #[test]
    fn the_slot_check_sees_two_languages_that_do_not_agree() {
        let w = |zh: &'static str, en: &'static str| Word { key: "x", kind: Kind::Hint, zh, en };
        assert_eq!(slots_agree(&w("已发 {0},共 {1} 条", "Sent {0}")), Err("E_I18N_SLOTS"));
        assert_eq!(slots_agree(&w("已发 {1}", "Sent {1}")), Err("E_I18N_SLOTS"));
        assert_eq!(slots_agree(&w("已发 {tx}", "Sent {tx}")), Err("E_I18N_SLOTS"));
        assert_eq!(slots_agree(&w("已发 {0},共 {1} 条", "{1} in all, sent {0}")), Ok(()));
    }

    #[test]
    fn every_key_has_its_name_and_class_in_the_tables_order() {
        assert_eq!(meta::META.len(), words().len());
        for (k, n, _) in meta::META.iter() {
            assert_eq!(name(*k), *n);
            assert_eq!(key_of(n), Some(*k));
        }
        assert_eq!(key_of("nope.not.here"), None);
    }

    #[test]
    fn filling_is_one_pass() {
        assert_eq!(fill_phrase(&get_named(Lang::Zh, "nope.not.here").0, &["a"]), "⟦nope.not.here⟧");
        let k = words().iter().find(|w| slots(w.zh) == vec![0, 1]).map(|w| w.key).expect("有两格的词");
        let got = filln_in(Lang::Zh, key_of(k).expect("表上的键"), &["{1}", "B"]);
        assert!(got.contains("{1}") && got.contains('B'), "{got}");
        assert_eq!(fill_phrase("a {0} b {1}", &["x"]), "a x b {1}");
        assert_eq!(fill_phrase("a {0}", &["x", "y", "z"]), "a x");
        assert_eq!(fill_phrase("{ not a slot } {0}", &["x"]), "{ not a slot } x");
    }

    #[test]
    fn home_is_called_home_page_in_both_tables() {
        assert_eq!(get(Lang::Zh, crate::house::i18n::Key::NavHome).0, "首页");
        assert_eq!(get(Lang::En, crate::house::i18n::Key::NavHome).0, "Home");
    }

    #[test]
    fn human_words_carry_no_code_names() {
        for w in words().iter() {
            let zh = w.zh;
            assert!(!zh.contains("settings"), "{} 里写了代码名 settings:{zh}", w.key);
            assert!(!zh.contains("E_"), "{} 里写了错误代码:{zh}", w.key);
        }
    }

    #[test]
    fn the_keyset_comparator_passes_its_own_planted_pair() {
        assert_eq!(keyset_self_check(), Ok(()));
    }

    #[test]
    fn the_two_key_sets_are_equal() {
        assert!(only_in(Lang::Zh, Lang::En).is_empty());
        assert!(only_in(Lang::En, Lang::Zh).is_empty());
        assert_eq!(keys(Lang::Zh).len(), words().len());
    }

    #[test]
    fn a_key_that_is_not_there_comes_back_named() {
        let (s, got) = get_named(Lang::Zh, "nope.not.here");
        assert!(!got, "取不到那一趟要说出来");
        assert_eq!(s, "⟦nope.not.here⟧");
    }

    #[test]
    fn the_wording_checker_passes_its_own_three_planted_sentences() {
        assert_eq!(checker_self_check(), Ok(()));
    }

    #[test]
    fn every_network_word_is_in_both_tables() {
        for n in eravon_literals::constants::NETWORKS {
            assert!(get_named(Lang::Zh, n.word).1 && get_named(Lang::En, n.word).1, "{} 两语都要有", n.word);
        }
    }
}
