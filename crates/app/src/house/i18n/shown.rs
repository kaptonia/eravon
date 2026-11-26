use super::{get, Key, Lang};

pub const CONSTS: &[(&str, Key)] = &[
    (eravon_export::seam::MISSING_POSTED, Key::ExMissPosted),
    (eravon_export::seam::MISSING_LISTING, Key::ExMissListing),
    (eravon_export::seam::MISSING_RECEIPT, Key::ExMissReceipt),
    (eravon_recheck::check::WHY_UNTRACED, Key::ExRcWhyUntraced),
    (eravon_recheck::check::WHY_OTHER_DIGEST, Key::ExRcWhyOtherDigest),
    (eravon_deliver::todo::TODO_SAY, Key::OrdersTodoSay),
    (eravon_watch::order::GAP_NOTE, Key::OrdersGapNote),
    (eravon_watch::send::NO_KEY_NOTE, Key::OrdersBumpLocked),
    (eravon_watch::send::OPEN_OFF_NOTE, Key::OrdersAwaitOpen),
    (eravon_scan::scan::SAY_UNTRACED, Key::OrdersUntraced),
    (eravon_dispute::seat::RESIGN_HINT, Key::OrdersResignHint),
    (eravon_import::import::SAY_BY_CALL, Key::BoReadOnlyByCall),
    (eravon_import::import::SAY_COMPANY, Key::BoReadOnlyCompany),
    (crate::house::seam::PL_OVER_CAP, Key::PlOverCap),
    (crate::house::seam::PL_PARTY, Key::PlParty),
    (crate::house::seam::PL_KEY_UNOPENED, Key::PlKeyUnopened),
    (crate::house::seam::PL_KEY_FAILED, Key::PlKeyFailed),
    (crate::house::seam::PL_KEY_NO_PICK, Key::PlKeyNoPick),
    (crate::house::seam::PL_KEY_NO_PAYOUT, Key::PlKeyNoPayout),
    (crate::house::seam::PL_KEY_PAID, Key::PlKeyPaid),
    (crate::house::seam::PL_LOCKED_BEFORE, Key::PlLockedBefore),
    (crate::house::seam::PL_LOCKED_BETWEEN, Key::PlLockedBetween),
    (crate::house::seam::PICK_NO_PANEL, Key::PickNoPanel),
];

pub fn absent_detail(lang: Lang, a: &eravon_cands::fault::Absent) -> String {
    use eravon_cands::fault::Absent;
    let (key, inner) = match a {
        Absent::Succeeded { to, block } => return super::filln_in(lang, Key::CandsAbsentDetailSucceeded, &[&eravon_orders::table::short(to), &block.to_string()]),
        Absent::NoRegistry => (Key::CandsAbsentDetailNoRegistry, ""),
        Absent::Unreachable(e) => (Key::CandsAbsentDetailUnreachable, e.as_str()),
        Absent::NoDocument => (Key::ErrECANDSNODOCUMENT, ""),
        Absent::Unverified(e) => (Key::CandsAbsentDetailUnverified, e.as_str()),
        Absent::Retired => (Key::CandsAbsentDetailRetired, ""),
    };
    super::filln_in(lang, key, &[inner])
}

pub fn judge_state(lang: Lang, state: &str) -> String {
    match state {
        "已接单" => get(lang, crate::house::i18n::Key::OrdersJudgeAccepted).0,
        "待接单" => get(lang, crate::house::i18n::Key::OrdersWordAwaitAccept).0,
        "未接单" => get(lang, crate::house::i18n::Key::OrdersJudgeNotAccepted).0,
        other => other.to_string(),
    }
}

pub fn order_word(lang: Lang, w: eravon_orders::status::Word) -> String {
    word_key(w).map(|k| get(lang, k).0).unwrap_or_else(|| w.say().to_string())
}

fn word_key(w: eravon_orders::status::Word) -> Option<Key> {
    use eravon_orders::status::Word;
    Some(match w {
        Word::AwaitPay => Key::OrdersWordAwaitPay,
        Word::AwaitDeploy => Key::OrdersStateReady,
        Word::AwaitAccept => Key::OrdersWordAwaitAccept,
        Word::AwaitDelivery => Key::OrdersWordAwaitDelivery,
        Word::AwaitConfirm => Key::OrdersWordAwaitConfirm,
        Word::Disputed => Key::OrdersWordDisputed,
        Word::AwaitRedraw => Key::OrdersFacePendingRedraw,
        Word::Completed => Key::Done,
        Word::Ruled => Key::OrdersWordRuled,
        Word::Settled => Key::OrdersWordSettled,
        Word::Refunded => Key::OrdersWordRefunded,
        Word::Lapsed => Key::OrdersWordLapsed,
        Word::Expired => Key::OrdersStateExpired,
        Word::ConfirmHanded => Key::OrdersWordConfirmHanded,
        Word::Cancelled => Key::OrdersStateCancelled,
        Word::Revoked => Key::GoodsLinkOff,
    })
}

const EVENTS: &[(&str, Key)] = &[
    ("Opened", Key::OrdersEvOpened),
    ("Priced", Key::OrdersEvPriced),
    ("PanelFrozen", Key::OrdersEvPanelFrozen),
    ("Seated", Key::OrdersEvSeated),
    ("DeliveryClaimed", Key::OrdersEvDeliveryClaimed),
    ("Confirmed", Key::OrdersEvConfirmed),
    ("Disputed", Key::OrdersEvDisputed),
    ("Settled", Key::OrdersEvSettled),
    ("Ruled", Key::OrdersEvRuled),
    ("Resigned", Key::OrdersResignOther),
    ("Lapsed", Key::OrdersEvLapsed),
    ("Refunded", Key::GoodsTabRefund),
    ("Closed", Key::OrdersEvClosed),
    ("Swept", Key::OrdersEvSwept),
    ("Committed", Key::OrdersEvCommitted),
    ("Anchored", Key::OrdersEvAnchored),
    ("Consented", Key::OrdersEvConsented),
    ("", Key::OrdersEvOther),
];

fn enum_key(text: &str) -> Option<Key> {
    use eravon_after::act::Access;
    use eravon_watch::order::State;
    let state = |s: State| match s {
        State::Awaiting => Some(Key::OrdersStateAwaiting),
        State::Short => Some(Key::OrdersStateShort),
        State::Ready => Some(Key::OrdersStateReady),
        State::PendingConsent => Some(Key::OrdersStatePendingConsent),
        State::PriceStale => Some(Key::OrdersStatePriceStale),
        State::Deployed => Some(Key::OrdersStateDeployed),
        State::Expired => Some(Key::OrdersStateExpired),
        State::DepositShort => Some(Key::OrdersStateDepositShort),
        State::Blocked => Some(Key::OrdersStateBlocked),
        State::AwaitPrev => Some(Key::OrdersStateAwaitPrev),
        State::Revoked => Some(Key::GoodsLinkOff),
        State::Cancelled => Some(Key::OrdersStateCancelled),
    };
    let access = |a: Access| match a {
        Access::Key => None,
        Access::ByCall => Some(Key::BoReadOnlyByCall),
        Access::Company => Some(Key::BoReadOnlyCompany),
    };
    if let Some(s) = State::ALL.into_iter().find(|s| s.say() == text) {
        return state(s);
    }
    if let Some(w) = eravon_orders::status::Word::ALL.into_iter().find(|w| w.say() == text) {
        return word_key(w);
    }
    if let Some((_, key)) = EVENTS.iter().find(|(name, _)| eravon_orders::detail::say(name) == text) {
        return Some(*key);
    }
    [Access::ByCall, Access::Company].into_iter().find(|a| a.note() == text).and_then(access)
}

pub fn library(lang: Lang, text: &str) -> String {
    if lang == Lang::Zh {
        return text.to_string();
    }
    let whole = CONSTS.iter().find(|(v, _)| *v == text).map(|(_, key)| *key).or_else(|| enum_key(text));
    match whole {
        Some(key) => get(lang, key).0,
        None => amount_words(lang, text),
    }
}

fn amount_words(lang: Lang, text: &str) -> String {
    let (zh, to) = (&eravon_vault::tokens::ZH, crate::house::coin::words(lang));
    text.replace(zh.unlisted, to.unlisted).replace(zh.smallest, to.smallest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_paired_constant_matches_its_key_in_chinese() {
        for (v, key) in CONSTS {
            assert_eq!(get(Lang::Zh, *key), (v.to_string(), true), "{key:?}");
        }
    }

    #[test]
    fn library_enum_words_match_their_keys() {
        use eravon_dispute::seat::Face;
        assert_eq!(Face::PendingRedraw(0).say(), get(Lang::Zh, crate::house::i18n::Key::OrdersFacePendingRedraw).0);
        use eravon_watch::order::State;
        for s in State::ALL {
            assert!(enum_key(s.say()).is_some_and(|k| get(Lang::Zh, k).0 == s.say()), "{}", s.say());
        }
        for a in [eravon_after::act::Access::ByCall, eravon_after::act::Access::Company] {
            assert!(enum_key(a.note()).is_some_and(|k| get(Lang::Zh, k).0 == a.note()), "{}", a.note());
        }
        for w in eravon_orders::status::Word::ALL {
            match word_key(w) {
                Some(k) => assert_eq!(get(Lang::Zh, k).0, w.say(), "{w:?}"),
                None => assert_eq!(order_word(Lang::Zh, w), w.say(), "{w:?}"),
            }
        }
        for (name, key) in EVENTS {
            assert_eq!(get(Lang::Zh, *key).0, eravon_orders::detail::say(name), "{name}");
        }
    }

    #[test]
    fn a_fixed_phrase_comes_out_whole_and_anything_else_as_written() {
        assert_eq!(library(Lang::En, eravon_export::seam::MISSING_LISTING), get(Lang::En, crate::house::i18n::Key::ExMissListing).0);
        let joined = format!("{} · 0xab", eravon_export::seam::MISSING_LISTING);
        assert_eq!(library(Lang::En, &joined), joined);
        assert_eq!(library(Lang::Zh, eravon_export::seam::MISSING_LISTING), eravon_export::seam::MISSING_LISTING);
    }

    #[test]
    fn an_amount_inside_a_phrase_follows_the_page_language() {
        let raw = "5 最小单位 · 未登载 0x4e4e…4e4e";
        let en = library(Lang::En, raw);
        assert!(en.contains("smallest units") && en.contains("Not listed") && !en.contains("未登载") && !en.contains("最小单位"), "{en}");
        assert_eq!(library(Lang::Zh, raw), raw);
    }
}
