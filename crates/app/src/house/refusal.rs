use crate::house::i18n::{self, Lang};

pub const NAMES: [&str; 256] = [
    "E_EVIDENCE_SEATS",
    "E_EVIDENCE_NO_RECIPIENT",
    "E_EVIDENCE_CHANGED",
    "E_EVIDENCE_NAME_TAKEN",
    "E_EVIDENCE_WRITE",
    "E_EVIDENCE_EXISTS",
    "E_EVIDENCE_NONE",
    "E_JUDGE_UNREAD",
    "E_JUDGE_NONE",
    "E_DISPUTE_LIST",
    "E_APP_URL_SCHEME",
    "E_GOODS_IMAGE_DIFFERS",
    "E_GOODS_NO_IMAGES",
    "E_CASE_NOT_HELD",
    "E_CASE_NOT_OPEN",
    "E_CASE_COVENANT_ABSENT",
    "E_PAYWAYS_OPEN_WINDOW",
    "E_PAYWAYS_FUND",
    "E_P6_SETTLE_UNPOSTED",
    "E_P3_COMPANY_FORM",
    "E_P7_FEES",
    "E_APP_FILE_MISSING",
    "E_APP_DENIED",
    "E_APP_CLIPBOARD",
    "E_OCCUPIED",
    "E_APP_TRACE_FULL",
    "E_CANDS_LABEL",
    "E_CANDS_ADDRESS",
    "E_CANDS_POINTER",
    "E_CANDS_CHAIN",
    "E_CANDS_EXISTS",
    "E_CANDS_MISSING",
    "E_CANDS_KIND",
    "E_CHAIN_TRANSPORT",
    "E_CHAIN_ENDPOINT",
    "E_CHAIN_EXECUTION",
    "E_CHAIN_DISAGREE",
    "E_CHAIN_WRONG",
    "E_CHAIN_USAGE",
    "E_CHAIN_UNREAD",
    "E_CHAIN_ECHO",
    "E_CHAIN_OVER_ASKS",
    "E_V_IO",
    "E_V_MISSING",
    "E_V_EXISTS",
    "E_V_STATE",
    "E_V_PRECHECK",
    "E_V_DOWN",
    "E_V_NOT_YET",
    "E_V_REVERTED",
    "E_V_UNCONFIRMED",
    "E_V_ARG",
    "E_V_ON_DESKTOP",
    "E_V_DESKTOP",
    "E_V_SENT_VOID",
    "E_UNREACHABLE",
    "E_TX_NOT_YET",
    "E_PAYWAYS_CHANNEL",
    "E_PAYWAYS_ID_FORM",
    "E_PAYWAYS_MIRROR_SCHEME",
    "E_PAYWAYS_MIRROR_FORM",
    "E_PAYWAYS_MIRROR_DUP",
    "E_PAYWAYS_TEST_EMPTY",
    "E_PAYWAYS_PAYLOAD",
    "E_PAYWAYS_RETURN",
    "E_R1_DOSSIER",
    "E_R1_FORM",
    "E_A3_FORM",
    "E_A3_REBUILD",
    "E_A1_FORM",
    "E_A1_ABI",
    "E_A1_CODEHASH",
    "E_A1_PAST_CONTRACT",
    "E_A1_PAST_LISTING",
    "E_P6_PAST_LACKS",
    "E_POOL_GAS",
    "E_GOODS_KEY_HANDED_ON",
    "E_WATCH_SERIES",
    "E_WATCH_ARCHIVE",
    "E_P3_SERIES_PREV",
    "E_ACT_GAS",
    "E_P3_OFFER_RETIRED",
    "UNPLACED",
    "SELLER_KEY_RETIRED",
    "E_A1_REVOKED",
    "E_V_ROTATE_NO_CHAIN",
    "E_V_ROTATE_MOVED",
    "E_V_ROTATE_ENTERED",
    "E_V_ROTATE_NOT_LANDED",
    "E_V_ROTATE_HELD",
    "E_V_ROTATE_OFF_NETWORK",
    "E_P7_AGAIN",
    "E_A1_PRECHECK",
    "E_STORE_ROOT",
    "E_STORE_NAME",
    "E_STORE_KIND",
    "E_STORE_VERSION",
    "E_STORE_FORM",
    "E_STORE_MISSING",
    "E_STORE_EXISTS",
    "E_STORE_IO",
    "E_STORE_LOCKED",
    "E_STORE_ARCHIVE",
    "E_STORE_MIRROR",
    "E_STORE_TIME",
    "E_STORE_CAP",
    "E_STORE_NO_KEY",
    "E_STORE_SEALED",
    "E_STORE_UNREAD",
    "E_STORE_STALE",
    "E_STORE_OTHER_MACHINE",
    "E_STORE_MACHINE",
    "E_VAULT_RULE",
    "E_VAULT_PASSCODE",
    "E_VAULT_DEAD",
    "E_VAULT_LOCKED",
    "E_VAULT_MNEMONIC",
    "E_VAULT_PARTY_KEY",
    "E_VAULT_SEALED",
    "E_VAULT_FORM",
    "E_VAULT_KDF_BOUND",
    "E_VAULT_PATH",
    "E_VAULT_IO",
    "E_VAULT_KEYFILE_BROKEN",
    "E_VAULT_EXISTS",
    "E_VAULT_NO_IDENTITY",
    "E_VAULT_MIGRATE",
    "E_VAULT_ADDRESS",
    "E_VAULT_NOT_BACKUP",
    "E_VAULT_BACKUP_PASSWORD",
    "E_VAULT_BACKUP_VERSION",
    "E_VAULT_KEYSTORE_PARAMS",
    "E_VAULT_HEAD_UNREAD",
    "E_VAULT_NAME_KEY",
    "E_VAULT_PASSWORD_LONG",
    "E_VAULT_BACKUP_NOT_LANDED",
    "E_VAULT_KDF_FLOOR",
    "E_VAULT_BACKUP_SHAPE",
    "E_ZK_INDEX_FORM",
    "E_ZK_INDEX_SHAPE",
    "E_ZK_NO_SUCH_KIT",
    "E_ZK_PACK",
    "E_ZK_IO",
    "E_ZK_ARG",
    "E_P6_BY_CALL_KEY",
    "E_P6_FORM",
    "E_P6_NOTHING",
    "E_P6_NOT_COMPANY",
    "E_P6_NOT_OPEN",
    "E_P6_NO_PACK",
    "E_P6_NO_PAYOUT",
    "E_P6_NO_TERMS",
    "E_P6_OPENED",
    "E_P6_PULL_KEY",
    "E_P6_READONLY",
    "E_P6_SETTLE_EXPIRED",
    "E_P6_SETTLE_UNREAD",
    "E_P6_SETTLE_FORM",
    "E_P6_SETTLE_GATE",
    "E_P6_SHORT",
    "E_P6_STALE",
    "E_P4_NOT_OPENED",
    "E_P4_FOREIGN_KEY",
    "E_P4_NO_ENDPOINT",
    "E_P4_UNTRACED",
    "E_P3_OPEN_BY_PINNED",
    "E_P3_COMPANY_TAKEN",
    "E_P3_QUOTE",
    "E_P3_NO_ENDPOINT",
    "E_P3_STANDARD_CHAIN",
    "E_P3_STANDARD_LOGIC",
    "E_P3_STANDARD_NO_CODE",
    "E_P7_AMOUNT",
    "E_P7_DECIMALS",
    "E_P7_KEEP",
    "E_P7_OVER",
    "E_P7_RECEIPT",
    "E_P7_RECORD",
    "E_P7_STALE",
    "E_POOL_NO_GUILD_TERMS",
    "E_POOL_NO_PANEL",
    "E_POOL_BOOK_UNREAD",
    "E_HOME_SHARE_UNTOLD",
    "E_POOL_TERMS_DIGEST",
    "E_CANDS_NO_REGISTRY",
    "E_CANDS_UNREACHABLE",
    "E_CANDS_NO_DOCUMENT",
    "E_CANDS_UNVERIFIED",
    "E_CANDS_RETIRED",
    "E_CANDS_SUCCEEDED",
    "E_COMPLAINT_TEXT",
    "E_COMPLAINT_NO_GUILD",
    "E_COMPLAINT_SIGNER",
    "E_COMPLAINT_FILE",
    "E_COMPLAINT_WRITE",
    "E_COMPLAINT_EXISTS",
    "E_COMPLAINT_FORM",
    "E_CLI_PATH_TAKEN",
    "E_CLI_PATH_CANCELLED",
    "E_CLI_PATH_NOT_ALLOWED",
    "E_CLI_PATH_UNSUPPORTED",
    "E_APP_CLI_ANCHOR_GREY",
    "E_ZK_NOT_CONNECTED",
    "E_ZK_POINTER_STALE",
    "E_ZK_NO_KITS",
    "E_LINK_UNPOSTED",
    "E_APP_SETTLE_NONE",
    "E_PL_SEGMENT_HASH",
    "E_PL_SEGMENT_READ",
    "E_ACT_JUDGE_AMANAH",
    "E_ACT_JUDGE_PAUSED",
    "E_ACT_JUDGE_PARTY",
    "E_OPEN_OPENED",
    "E_P6_DEPOSIT_SHORT",
    "E_ACT_JUDGE_LINEAGE",
    "E_ACT_AMANAH_UNANCHORED",
    "E_ACT_AMANAH_DISCARDED",
    "E_ACT_JUDGE_NONE",
    "E_ACT_JUDGE_MISMATCH",
    "E_ACT_NO_QUOTE",
    "E_ACT_QUOTE",
    "E_ACT_GUILD_AMANAH",
    "E_ACT_STANDARD_NO_CODE",
    "E_ACT_STANDARD_LOGIC",
    "E_ACT_BAD_QUOTE",
    "E_QUOTE_FORM",
    "E_POOL_NO_GUILDTERMS",
    "E_POOL_POINTER",
    "E_POOL_ROSTER",
    "E_POOL_ROSTER_EMPTY",
    "E_POOL_NO_QUOTE",
    "E_POOL_QUOTE_FORM",
    "E_POOL_PANEL_HASH",
    "E_POOL_PANEL_EXPIRED",
    "E_POOL_PANEL_WORD",
    "E_POOL_PANEL_EMPTY",
    "E_POOL_PACE",
    "E_GOODS_NOT_ACCEPTING",
    "E_GOODS_LEDGER_UNREAD",
    "E_GOODS_NOT_IN_TABLE",
    "E_GOODS_NO_QUOTE",
    "E_GOODS_QUOTE_UNREAD",
    "E_P3_SERIES_SAME",
    "E_GOODS_MISSING",
    "E_GOODS_DRAFT_FORM",
    "E_GOODS_REFUSED",
    "E_GOODS_CHAIN",
    "E_GOODS_REVOKED",
    "E_GOODS_NOTHING",
    "E_GOODS_NOT_DRAFT",
    "E_GOODS_OTHER_CHAIN",
    "E_LINK_OTHER_CHAIN",
    "E_V_ROTATE_WORDS_GONE",
    "E_GOODS_COMPANY",
    "E_GOODS_EXPORT",
    "E_VAULT_SELLER_KEY",
];

const TOKENS: [(&str, &str); 2] = [("eravon-terms", include_str!("../../../eravon-terms/src/token.rs")), ("eravon-link", include_str!("../../../eravon-link/src/token.rs"))];

pub const FORMS: [(&str, &str, &str); 53] = [
    ("eravon-terms", "E_SIG", "sig"),
    ("eravon-terms", "", "terms"),
    ("eravon-link", "E_LISTING", "listing"),
    ("eravon-link", "E_JUDGE", "listing"),
    ("eravon-link", "E_GUILD", "listing"),
    ("eravon-link", "E_LINK", "link"),
    ("eravon-link", "E_PACK", "pack"),
    ("eravon-link", "E_RECEIPT", "receipt"),
    ("eravon-link", "E_SETTLE", "settle"),
    ("", "E_LINK", "link"),
    ("", "E_LISTING", "listing"),
    ("", "E_PACK", "pack"),
    ("", "E_RECEIPT", "receipt"),
    ("", "E_SETTLE", "settle"),
    ("", "E_TERMS", "terms"),
    ("", "E_RUNG", "terms"),
    ("", "E_LADDER", "terms"),
    ("", "E_ACTION", "terms"),
    ("", "E_OFFER", "terms"),
    ("", "E_ORDER", "terms"),
    ("", "E_FEE_OVERFLOW", "terms"),
    ("", "E_JUDGE", "candidate"),
    ("", "E_GUILD", "candidate"),
    ("", "E_QUOTE", "candidate"),
    ("", "E_SIG", "sig"),
    ("", "E_DOSSIER", "dossier"),
    ("", "E_TRANSACTION", "dossier"),
    ("", "E_AUTHOR", "document"),
    ("", "E_BODY", "document"),
    ("", "E_DEPTH", "document"),
    ("", "E_DUP_KEY", "document"),
    ("", "E_ENTRYTYPE", "document"),
    ("", "E_ENVELOPE", "document"),
    ("", "E_GENESIS_PLACE", "document"),
    ("", "E_JSON", "document"),
    ("", "E_KEY_CHARSET", "document"),
    ("", "E_NOT_CANONICAL", "document"),
    ("", "E_NUMBER", "document"),
    ("", "E_PREV", "document"),
    ("", "E_SEQ", "document"),
    ("", "E_SPEC", "document"),
    ("", "E_UTF8", "document"),
    ("", "E_VALUE_CHARSET", "document"),
    ("", "E_ABSENT", "file"),
    ("", "E_BAD_NAME", "file"),
    ("", "E_CONFLICT", "file"),
    ("", "E_IO", "file"),
    ("", "E_NOT_A_DIRECTORY", "file"),
    ("", "E_NOT_A_FILE", "file"),
    ("", "E_TOO_LARGE", "file"),
    ("", "E_UNACCOUNTED", "file"),
    ("", "E_UNREADABLE", "file"),
    ("", "E_FILE", "file"),
];

fn crate_of(name: &str) -> &'static str {
    let quoted = format!("\"{name}\"");
    TOKENS.iter().find(|(_, src)| src.split("\n#[cfg(test)]").next().unwrap_or(src).contains(&quoted)).map_or("", |(c, _)| *c)
}

fn form_of(name: &str) -> Option<&'static str> {
    if !name.starts_with("E_") {
        return None;
    }
    let of = |c: &str| FORMS.iter().filter(|(k, ..)| *k == c).find(|(_, p, _)| p.is_empty() || name == *p || name.strip_prefix(p).is_some_and(|r| r.starts_with('_'))).map(|(.., f)| *f);
    of(crate_of(name)).or_else(|| of(""))
}

pub fn what(lang: Lang, name: &str) -> Option<String> {
    words(lang, name, "")
}

pub fn next(lang: Lang, name: &str) -> Option<String> {
    words(lang, name, ".next")
}

fn words(lang: Lang, name: &str, tail: &str) -> Option<String> {
    if NAMES.contains(&name) {
        return Some(i18n::get_named(lang, &format!("err.{name}{tail}")).0);
    }
    form_of(name).map(|f| i18n::get_named(lang, &format!("err.form.{f}{tail}")).0)
}

pub fn line(lang: Lang, name: &str, detail: &str) -> String {
    if let Some(file) = eravon_dispute::evidence::charset_refused(detail) {
        return i18n::filln_in(lang, i18n::Key::EvidenceNameCharset, &[file]);
    }
    first(lang, name, detail).unwrap_or_else(|| said(lang, name, detail))
}

fn first(lang: Lang, name: &str, detail: &str) -> Option<String> {
    let by_cause = match name {
        "E_V_DOWN" => eravon_cli::fault::down_of(detail).and_then(|(_, token, why)| first(lang, token, why)),
        "E_V_PRECHECK" => eravon_cli::fault::precheck_of(detail).and_then(|(g, _)| gate(lang, g)),
        "E_A1_PRECHECK" => detail.split_once(':').and_then(|(g, _)| gate(lang, g)),
        _ => None,
    };
    by_cause.or_else(|| own(name).map(|k| i18n::get(lang, k).0)).or_else(|| what(lang, name))
}

fn gate(lang: Lang, g: &str) -> Option<String> {
    use eravon_standard::precheck as p;
    use i18n::Key as K;
    let k = match g {
        p::GATE_SIG => K::ErrGateSig,
        p::GATE_CALLER => K::ErrGateCaller,
        p::GATE_FUNDS => K::ErrGateFunds,
        p::GATE_ALLOWANCE => K::ErrGateAllowance,
        p::GATE_WINDOW => K::ErrGateWindow,
        p::GATE_PRICE => K::OrdersStatePriceStale,
        p::GATE_VERIFIER => K::ErrGateVerifier,
        p::GATE_BOND => K::OrdersStateDepositShort,
        p::GATE_TERMS => K::ErrGateTerms,
        p::GATE_REVOKED => K::ErrGateRevoked,
        p::GATE_CURRENT => K::ErrGateCurrent,
        p::GATE_STAGE => K::ErrGateStage,
        eravon_cli::fault::GATE_FEE => K::ErrGateFee,
        eravon_cli::fault::GATE_ROUTE => K::ErrEA1CODEHASH,
        eravon_cli::fault::GATE_COMPANY => K::ErrGateCompany,
        eravon_cli::fault::GATE_TIME | eravon_cli::company::TIME_ASK => K::ErrGateTime,
        eravon_cli::company::HOLDER => K::ErrGateHolder,
        _ => return None,
    };
    Some(i18n::get(lang, k).0)
}

fn own(name: &str) -> Option<i18n::Key> {
    match name {
        eravon_goods::link::E_GOODS_LINK_UNPOSTED => Some(i18n::Key::GoodsPostPending),
        eravon_goods::goods::E_GOODS_FEES_CHANGED => Some(i18n::Key::GoodsFeesNewTitle),
        _ => None,
    }
}

pub fn said(lang: Lang, name: &str, detail: &str) -> String {
    if let Some(file) = eravon_dispute::evidence::charset_refused(detail) {
        return i18n::filln_in(lang, i18n::Key::EvidenceNameCharset, &[file]);
    }
    if lang == Lang::Zh || !detail.chars().any(han) {
        return detail.to_string();
    }
    what(lang, name).or_else(|| Some(i18n::library(lang, detail)).filter(|w| !w.chars().any(han))).unwrap_or_else(|| name.to_string())
}

pub fn line_left(lang: Lang, name: &str, detail: &str, left: u32) -> String {
    match NAMES.contains(&name).then(|| i18n::get_named(lang, &format!("err.{name}.left"))) {
        Some((w, true)) => i18n::fill_phrase(&w, &[&left.to_string()]),
        _ => line(lang, name, detail),
    }
}

fn han(c: char) -> bool {
    matches!(c, '\u{3000}'..='\u{303f}' | '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}' | '\u{ff00}'..='\u{ffef}')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linked(root: &std::path::Path) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut todo = vec!["app".to_string()];
        while let Some(c) = todo.pop() {
            let text = std::fs::read_to_string(root.join(&c).join("Cargo.toml")).expect("读得到清单");
            let deps = text.split("\n[dependencies]").nth(1).and_then(|t| t.split("\n[").next()).unwrap_or_default();
            for l in deps.lines() {
                let Some(d) = l.split("path = \"../").nth(1).and_then(|t| t.split('"').next()).filter(|d| !d.contains('/')) else { continue };
                if !out.iter().any(|x| x == d) {
                    out.push(d.to_string());
                    todo.push(d.to_string());
                }
            }
        }
        out.sort();
        out
    }

    const NOT_SAID: [(&str, &str); 4] = [("eravon-cli/src/codes.rs", "命令行答面的码表:命令行与 AI 应用从答面读它,不经桌面页"), ("eravon-cli/src/codes_own.rs", "命令行答面码表的本家一份(登记口 ⑥,由 codes.rs 重导出):同上"), ("eravon-literals/src/misuse.rs", "误用:命令行退出码 2、只出 stderr,不成页上的拒因"), ("eravon-chain/src/scan.rs", "件的日志窗之拒:读层 read::sweep 改说成节点或通道的错(Node::fault、Fault::Endpoint),这几个名不上页")];

    #[test]
    fn every_refusal_name_has_its_words_and_next_step_in_both_languages() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut given: Vec<String> = Vec::new();
        let mut take = |text: &str, pat: &str| {
            let mut rest = text;
            while let Some(i) = rest.find(pat) {
                let tail = &rest[i + pat.find('E').unwrap_or(0)..];
                let name: String = tail.chars().take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_').collect();
                if !given.contains(&name) {
                    given.push(name);
                }
                rest = &rest[i + pat.len()..];
            }
        };
        let crates = linked(&root);
        assert!(crates.iter().any(|c| c == "eravon-link") && crates.iter().any(|c| c == "eravon-goods"), "the desktop links what it reads: {crates:?}");
        for c in &crates {
            let mut dirs = vec![root.join(c).join("src")];
            while let Some(d) = dirs.pop() {
                for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        dirs.push(p);
                        continue;
                    }
                    let rel = p.strip_prefix(&root).map(|x| x.to_string_lossy().replace('\\', "/")).unwrap_or_default();
                    if p.extension().is_none_or(|x| x != "rs") || NOT_SAID.iter().any(|(f, _)| *f == rel) {
                        continue;
                    }
                    let text = std::fs::read_to_string(&p).unwrap_or_default();
                    take(text.split("\n#[cfg(test)]").next().unwrap_or(&text), "=> \"E_");
                }
            }
        }
        for c in ["after", "import", "pay", "wallet"] {
            let mut dirs = vec![root.join(format!("eravon-{c}/src"))];
            while let Some(d) = dirs.pop() {
                for e in std::fs::read_dir(&d).expect("读得到源").flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        dirs.push(p);
                    } else if p.extension().is_some_and(|x| x == "rs") {
                        take(&std::fs::read_to_string(&p).unwrap_or_default(), "\"E_P");
                    }
                }
            }
        }
        for f in ["eravon-cands/src/act.rs", "eravon-watch/src/act.rs", "eravon-roster/src/pool.rs"] {
            let text = std::fs::read_to_string(root.join(f)).expect("读得到源");
            take(text.split("\n#[cfg(test)]").next().unwrap_or(&text), "\"E_");
        }
        let text = std::fs::read_to_string(root.join("eravon-cli/src/buyer/standing.rs")).expect("读得到源");
        take(text.split("\n#[cfg(test)]").next().unwrap_or(&text), "\"E_");
        let text = std::fs::read_to_string(root.join("eravon-goods/src/resolve.rs")).expect("读得到源");
        take(text.split("\n#[cfg(test)]").next().unwrap_or(&text), "\"E_GOODS_");
        let text = std::fs::read_to_string(root.join("eravon-link/src/identity.rs")).expect("读得到源");
        let words = text.split("impl Pay {").nth(1).and_then(|t| t.split("\n}").next()).unwrap_or_default();
        for l in words.lines().filter(|l| l.contains("Pay::") && l.contains("=> \"") && !l.contains("Pay::Payable")) {
            let name: String = l.split("=> \"").nth(1).unwrap_or_default().chars().take_while(|c| c.is_ascii_uppercase() || *c == '_').collect();
            if !name.is_empty() && !given.contains(&name) {
                given.push(name);
            }
        }
        assert!(given.iter().any(|n| n == "E_RECEIPT_PREFIX") && given.iter().any(|n| n == "E_LINK_PREFIX"), "the link crate's names are read");
        let missing: Vec<&String> = given.iter().filter(|n| !NAMES.contains(&n.as_str()) && form_of(n).is_none()).collect();
        assert!(missing.is_empty(), "拒名表里没有:{missing:?}");
        for n in NAMES.iter().copied().chain(given.iter().map(String::as_str)) {
            for lang in [Lang::Zh, Lang::En] {
                let (w, nx) = (what(lang, n).unwrap_or_default(), next(lang, n).unwrap_or_default());
                assert!(!w.is_empty() && !w.starts_with('\u{27e6}') && !nx.is_empty() && !nx.starts_with('\u{27e6}'), "{n} {lang:?}");
            }
        }
    }

    #[test]
    fn a_form_refusal_falls_into_the_family_of_the_crate_that_gives_it() {
        assert_eq!(crate_of("E_JUDGE_CLOSED"), "eravon-terms");
        assert_eq!(form_of("E_JUDGE_CLOSED"), Some("terms"));
        assert_eq!(form_of("E_GUILD_POINTER"), Some("terms"));
        assert_eq!(form_of("E_QUOTE_FEED"), Some("terms"));
        assert_eq!(form_of("E_SIG_V"), Some("sig"));
        assert_eq!(form_of("E_JUDGE_SIG"), Some("listing"));
        assert_eq!(form_of("E_QUOTE_FORM"), Some("candidate"));
        assert_eq!(form_of("E_RECEIPT_PREFIX"), Some("receipt"));
        for lang in [Lang::Zh, Lang::En] {
            assert_eq!(what(lang, "E_JUDGE_CLOSED"), Some(i18n::get_named(lang, "err.form.terms").0), "{lang:?}");
            assert_ne!(what(lang, "E_JUDGE_CLOSED"), Some(i18n::get_named(lang, "err.form.candidate").0), "{lang:?}");
        }
    }

    #[test]
    fn a_form_refusal_passed_through_is_said_by_its_family() {
        use eravon_cli::fault::Fault as F;
        for lang in [Lang::Zh, Lang::En] {
            let receipt = what(lang, "E_RECEIPT_PREFIX").expect("回执那一族在表里");
            assert_ne!(receipt, what(lang, "E_V_DOWN").unwrap_or_default(), "{lang:?}");
            for token in ["E_RECEIPT_PREFIX", "E_RECEIPT_B64", "E_RECEIPT_SIG"] {
                let down = F::Down { layer: "K3", token: token.into(), why: "不是回执".into() };
                assert_eq!(line(lang, down.name(), &down.detail()), receipt, "{token} {lang:?}");
            }
            assert_ne!(what(lang, "E_LINK_PREFIX"), Some(receipt.clone()), "a link and a receipt are told apart");
            assert_eq!(form_of("E_ORDERS_DATE"), None, "a prefix is a whole segment");
        }
        let decoded = eravon_cli::seller::decode_receipt_field("这不是回执").expect_err("不是回执");
        assert_eq!(eravon_cli::fault::down_of(&decoded.detail()).map(|(_, t, _)| t), Some("E_RECEIPT_PREFIX"), "the receipt field refuses as a receipt: {}", decoded.detail());
    }

    fn cjk(s: &str) -> bool {
        s.chars().any(|c| matches!(c, '\u{3000}'..='\u{303f}' | '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}' | '\u{ff00}'..='\u{ffef}'))
    }

    #[test]
    fn the_english_first_layer_has_no_cjk() {
        assert!(cjk("口令") && !cjk("Wrong passcode · 3"), "the check sees Chinese");
        for n in NAMES {
            let l = line(Lang::En, n, "这一句是下层用中文写的原句");
            assert!(!l.is_empty() && !cjk(&l), "{n}: {l}");
        }
        use eravon_vault::vfault::Fault as F;
        let zh = || "中文的原句".to_string();
        let every = [
            F::Rule("口令不合规则"),
            F::WrongPasscode { wrong: 2, max: 5 },
            F::Dead { wrong: 5 },
            F::Locked,
            F::Mnemonic(zh()),
            F::Sealed,
            F::Vault(zh()),
            F::KdfBound(zh()),
            F::Path(zh()),
            F::Store(zh()),
            F::Down { token: "x", detail: zh() },
            F::KeyFileBroken(zh()),
            F::Exists(zh()),
            F::NoIdentity(zh()),
            F::Migrate(zh()),
            F::Address(zh()),
            F::NotBackup(zh()),
            F::BackupPassword,
            F::BackupVersion(zh()),
            F::KeystoreParams(zh()),
            F::HeadUnread(zh()),
            F::NameKeySealed(zh()),
            F::PasswordLong(zh()),
            F::BackupNotLanded(zh()),
            F::KdfFloor(zh()),
            F::BackupShape(zh()),
        ];
        for e in &every {
            let l = line_left(Lang::En, e.name(), &e.detail(), 3);
            assert!(!l.is_empty() && !cjk(&l), "{}: {l}", e.name());
        }
        let wrong = F::WrongPasscode { wrong: 2, max: 5 };
        assert_eq!(line_left(Lang::En, wrong.name(), &wrong.detail(), 3), "Wrong passcode · tries left: 3");
        assert_eq!(line_left(Lang::Zh, wrong.name(), &wrong.detail(), 3), "口令不对,还可再试 3 次");
        assert_eq!(line(Lang::En, "E_NOT_IN_THE_TABLE", "表外的中文原句"), "E_NOT_IN_THE_TABLE", "a name outside the table is said as its name");
    }

    #[test]
    fn the_first_layer_is_the_tables_words_and_the_answer_keeps_the_sentence() {
        let made = "已锁定,先解锁 0x00000000000000000000000000000000000000ab";
        for lang in [Lang::Zh, Lang::En] {
            assert_eq!(line(lang, "E_VAULT_LOCKED", made), what(lang, "E_VAULT_LOCKED").unwrap(), "{lang:?}");
        }
        assert_eq!(said(Lang::Zh, "E_VAULT_LOCKED", made), made, "the answer keeps the sentence on a Chinese page");
        assert_eq!(said(Lang::En, "E_VAULT_LOCKED", made), what(Lang::En, "E_VAULT_LOCKED").unwrap());
        assert_eq!(said(Lang::En, "E_VAULT_LOCKED", "Locked: unlock first"), "Locked: unlock first");
        assert_eq!(line(Lang::Zh, "E_OUTSIDE_THE_TABLE", "本机自己的句子"), "本机自己的句子", "a name outside the table keeps its sentence");
        assert_eq!(line(Lang::Zh, "ok", "已保存"), "已保存", "a success line is its sentence");
    }

    #[test]
    fn a_precheck_is_said_by_its_gate_and_a_lower_word_by_its_name() {
        use eravon_cli::fault::Fault as F;
        let gates = [
            eravon_standard::precheck::GATE_SIG,
            eravon_standard::precheck::GATE_CALLER,
            eravon_standard::precheck::GATE_FUNDS,
            eravon_standard::precheck::GATE_ALLOWANCE,
            eravon_standard::precheck::GATE_WINDOW,
            eravon_standard::precheck::GATE_PRICE,
            eravon_standard::precheck::GATE_VERIFIER,
            eravon_standard::precheck::GATE_BOND,
            eravon_standard::precheck::GATE_TERMS,
            eravon_standard::precheck::GATE_REVOKED,
            eravon_standard::precheck::GATE_CURRENT,
            eravon_standard::precheck::GATE_STAGE,
            eravon_cli::fault::GATE_FEE,
            eravon_cli::fault::GATE_ROUTE,
            eravon_cli::fault::GATE_COMPANY,
            eravon_cli::fault::GATE_TIME,
            eravon_cli::company::TIME_ASK,
            eravon_cli::company::HOLDER,
        ];
        for lang in [Lang::Zh, Lang::En] {
            let general = what(lang, "E_V_PRECHECK").unwrap();
            for g in gates {
                let f = F::Precheck { gate: g.to_string(), why: "余额 0x00000000000000000000000000000000000000ab 上有 0".into() };
                let w = line(lang, f.name(), &f.detail());
                assert!(!w.is_empty() && w != general && !w.contains("0x") && !w.starts_with('\u{27e6}'), "{g} {lang:?}: {w}");
                assert!(lang == Lang::En || !w.contains(g), "a Chinese page never shows the gate's name: {w}");
                assert_eq!(line(lang, "E_A1_PRECHECK", &format!("{g}:为什么")), w, "{g}: eravon-standard's own precheck says the same gate the same way");
            }
            let other = F::Precheck { gate: "unlisted".into(), why: "x".into() };
            assert_eq!(line(lang, other.name(), &other.detail()), general, "a gate not listed: the precheck's own words");
            let down = F::Down { layer: "A2", token: "E_CHAIN_TRANSPORT".into(), why: "连不上 http://127.0.0.1:9".into() };
            assert_eq!(line(lang, down.name(), &down.detail()), what(lang, "E_CHAIN_TRANSPORT").unwrap(), "{lang:?}");
            let unknown = F::Down { layer: "A2", token: "E_NOT_LISTED".into(), why: "x".into() };
            assert_eq!(line(lang, unknown.name(), &unknown.detail()), what(lang, "E_V_DOWN").unwrap(), "a word outside the table: the pass-through's own");
        }
        assert_eq!(eravon_cli::fault::precheck_of(&F::Precheck { gate: "funds".into(), why: "不够".into() }.detail()), Some(("funds", "不够")));
        assert_eq!(eravon_cli::fault::precheck_of("别的句子"), None);
    }

    #[test]
    fn a_name_refused_for_its_charset_is_said_with_the_file_in_both_languages() {
        let w = eravon_evidence_said("chat/聊天记录.txt");
        for (n, d) in [("E_DISPUTE_LIST", w.clone()), ("E_DELIVER_LIST", format!("无法读取文件:{w}")), ("E_A4_PACK", format!("/k 走不成清单项:NameNotPortable({w:?})"))] {
            for lang in [Lang::Zh, Lang::En] {
                let l = line(lang, n, &d);
                assert!(l.contains("chat/聊天记录.txt"), "{n} {lang:?}:{l}");
                assert_eq!(l, i18n::filln_in(lang, i18n::Key::EvidenceNameCharset, &["chat/聊天记录.txt"]), "{n} {lang:?}");
            }
        }
        assert!(!cjk(&line(Lang::En, "E_DISPUTE_LIST", &w).replace("聊天记录", "")), "英文页除文件名外没有中文");
    }

    fn eravon_evidence_said(name: &str) -> String {
        let d = std::env::temp_dir().join(format!("eravon-refusal-charset-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("chat")).expect("目录");
        std::fs::write(d.join(name), b"x").expect("写");
        let e = eravon_dispute::evidence::items(d.to_str().unwrap_or_default(), None, None, d.to_str()).expect_err("中文名该拒");
        let _ = std::fs::remove_dir_all(&d);
        e.1
    }
}
