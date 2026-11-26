use eravon_galeed as k1;

use crate::vfault::{Fault, R};
use crate::seam;

pub fn split(text: &str) -> Vec<String> {
    text.split_whitespace().map(|w| w.to_ascii_lowercase()).collect()
}

pub fn strangers<S: AsRef<str>>(cells: &[S]) -> Vec<bool> {
    cells.iter().map(|c| stranger(c.as_ref().trim())).collect()
}

const LONGEST: usize = 8;

fn stranger(w: &str) -> bool {
    if w.is_empty() {
        return false;
    }
    if w.len() > LONGEST || !w.is_ascii() {
        return true;
    }
    let mut buf = [0u8; (LONGEST + 1) * seam::WORDS];
    let mut n = 0;
    for k in 0..seam::WORDS {
        if k > 0 {
            buf[n] = b' ';
            n += 1;
        }
        for b in w.bytes() {
            buf[n] = b.to_ascii_lowercase();
            n += 1;
        }
    }
    let out = match std::str::from_utf8(&buf[..n]) {
        Ok(q) => matches!(k1::words::entropy(q), Err(k1::words::MnemonicError::UnknownWord(_))),
        Err(_) => true,
    };
    eravon_os::wipe(&mut buf);
    out
}

pub fn filled(text: &str) -> bool {
    text.split_whitespace().count() == seam::WORDS
}

pub fn fresh() -> R<String> {
    let e = k1::words::random_bytes(16).ok_or_else(|| Fault::Vault("取不到随机".into()))?;
    k1::words::from_entropy(&e).ok_or_else(|| Fault::Vault("这十六字节成不了助记词".into()))
}

pub fn entropy(phrase: &str) -> R<Vec<u8>> {
    k1::words::entropy(phrase).map_err(|e| Fault::Mnemonic(e.name().to_string()))
}


pub fn seed(phrase: &str) -> R<[u8; 64]> {
    k1::words::seed(phrase, "").ok_or_else(|| Fault::Mnemonic("word-count".into()))
}


#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "test test test test test test test test test test test junk";

    #[test]
    fn pasting_one_blob_splits_into_words() {
        assert_eq!(split("  Alpha\tBETA\n gamma ").len(), 3);
        assert_eq!(split("Alpha BETA")[0], "alpha", "一律转小写");
        assert!(filled(GOOD));
        assert!(!filled("test test test"));
    }

    #[test]
    fn the_wordlist_and_the_checksum_are_both_asked() {
        assert!(entropy(GOOD).is_ok());
        let e = entropy(&GOOD.replacen("junk", "zzzzz", 1)).unwrap_err();
        assert_eq!(e.name(), "E_VAULT_MNEMONIC");
        assert!(entropy(&GOOD.replacen("junk", "zoo", 1)).is_err(), "校验和不对也要拒");
    }

    #[test]
    fn a_fresh_phrase_reads_back() {
        let p = fresh().unwrap();
        assert_eq!(split(&p).len(), seam::WORDS);
        assert_eq!(entropy(&p).unwrap().len(), 16);
    }
}

pub fn challenge() -> R<[usize; 3]> {
    let b = k1::words::random_bytes(8).ok_or_else(|| Fault::Vault("取不到随机".into()))?;
    let mut picked: Vec<usize> = Vec::new();
    let mut i = 0usize;
    while picked.len() < 3 {
        let x = usize::from(b[i % b.len()]).wrapping_add(i * 7) % seam::WORDS;
        if !picked.contains(&x) {
            picked.push(x);
        }
        i += 1;
        if i > 64 {
            return Err(Fault::Vault("取不出三枚互异的位".into()));
        }
    }
    picked.sort_unstable();
    Ok([picked[0], picked[1], picked[2]])
}


pub fn verify<S: AsRef<str>>(phrase: &str, at: &[usize], given: &[S]) -> R<()> {
    let w: Vec<&str> = phrase.split_whitespace().collect();
    if w.len() != seam::WORDS {
        return Err(Fault::Mnemonic(format!("要恰 {} 个词", seam::WORDS)));
    }
    if at.len() != given.len() {
        return Err(Fault::Mnemonic("问了几位就要答几位".into()));
    }
    for (i, g) in at.iter().zip(given.iter()) {
        let want = w.get(*i).ok_or_else(|| Fault::Mnemonic(format!("第 {} 个词不在", i + 1)))?;
        if !g.as_ref().trim().eq_ignore_ascii_case(want) {
            return Err(Fault::Mnemonic(format!("第 {} 个词对不上", i + 1)));
        }
    }
    Ok(())
}

#[cfg(test)]
mod challenge_tests {

    #[test]
    fn a_word_not_in_the_list_is_marked_and_empty_cells_are_not() {
        let cells: Vec<String> = ["abandon", "abandn", "", "Zoo", "xyzzy"].map(String::from).to_vec();
        assert_eq!(super::strangers(&cells), vec![false, true, false, false, true]);
        let lent: Vec<&str> = vec!["ABANDON", " zoo ", "abandonabandon", "ábandon", "abandon about"];
        assert_eq!(super::strangers(&lent), vec![false, false, true, true, true], "borrowed text, any case, longer or not ASCII");
        for _ in 0..40 {
            let p = super::fresh().unwrap();
            let ws: Vec<&str> = p.split_whitespace().collect();
            assert!(super::strangers(&ws).iter().all(|b| !b), "{p}");
        }
    }

    use super::*;

    const GOOD: &str = "test test test test test test test test test test test junk";

    #[test]
    fn the_three_places_are_distinct_in_range_and_not_always_the_same() {
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..24 {
            let c = challenge().unwrap();
            assert!(c[0] < c[1] && c[1] < c[2], "三枚互异且升序:{c:?}");
            assert!(c[2] < seam::WORDS, "都在十二位之内");
            seen.insert(c);
        }
        assert!(seen.len() > 1, "三位固定即核对变成背那三个词");
    }

    #[test]
    fn the_right_words_pass_and_a_wrong_one_says_which_place() {
        let at = [0usize, 5, 11];
        let w: Vec<String> = split(GOOD);
        let right: Vec<String> = at.iter().map(|i| w[*i].clone()).collect();
        assert!(verify(GOOD, &at, &right).is_ok());

        let loose: Vec<String> = at.iter().map(|i| format!("  {}  ", w[*i].to_uppercase())).collect();
        assert!(verify(GOOD, &at, &loose).is_ok());

        let mut bad = right.clone();
        bad[1] = "zoo".into();
        let e = verify(GOOD, &at, &bad).unwrap_err();
        assert_eq!(e.name(), "E_VAULT_MNEMONIC");
        assert!(e.detail().contains("助记词"), "{}", e.detail());
    }

    #[test]
    fn asking_three_and_answering_two_is_refused() {
        assert!(verify(GOOD, &[0, 5, 11], &["test".to_string(), "test".to_string()]).is_err());
    }
}
