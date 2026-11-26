use eravon_literals::misuse::Misuse;
use eravon_literals::schema::{T_ARR, T_BOOL, T_INT, T_PATH, T_STR};

use crate::seam;

const TYPES: [(&str, &str); 123] = [
    ("--address", T_STR),
    ("--advance", T_STR),
    ("--again", T_STR),
    ("--amanah", T_ARR),
    ("--amount", T_STR),
    ("--amount-text", T_STR),
    ("--anchor", T_STR),
    ("--attach", T_ARR),
    ("--auto-open", T_BOOL),
    ("--block", T_PATH),
    ("--buyer", T_STR),
    ("--buyer-bps", T_INT),
    ("--buyer-key", T_STR),
    ("--buyer-may-open", T_BOOL),
    ("--buyer-payout", T_STR),
    ("--by-call", T_BOOL),
    ("--candidate", T_ARR),
    ("--case", T_STR),
    ("--chain", T_INT),
    ("--continue", T_BOOL),
    ("--check-only", T_BOOL),
    ("--company", T_STR),
    ("--count", T_INT),
    ("--data", T_PATH),
    ("--decimals", T_STR),
    ("--draft", T_BOOL),
    ("--draft-file", T_PATH),
    ("--endpoint", T_ARR),
    ("--expiry", T_INT),
    ("--expiry-word", T_STR),
    ("--export-proposal", T_PATH),
    ("--file", T_PATH),
    ("--file-passcode", T_STR),
    ("--from", T_STR),
    ("--from-block", T_INT),
    ("--fund", T_STR),
    ("--fund-own", T_BOOL),
    ("--gas-limit", T_INT),
    ("--guild", T_STR),
    ("--guild-panel", T_STR),
    ("--guild-seq", T_INT),
    ("--guild-sig", T_STR),
    ("--home", T_PATH),
    ("--id", T_STR),
    ("--instance", T_STR),
    ("--judge-sig", T_STR),
    ("--key", T_STR),
    ("--kind", T_STR),
    ("--kit", T_STR),
    ("--label", T_STR),
    ("--let-expire", T_ARR),
    ("--link", T_STR),
    ("--listing", T_PATH),
    ("--logic", T_STR),
    ("--machine", T_PATH),
    ("--manifest", T_STR),
    ("--manifest-dir", T_PATH),
    ("--manifest-file", T_PATH),
    ("--max-fee", T_STR),
    ("--media-type", T_STR),
    ("--member", T_ARR),
    ("--new-passcode", T_STR),
    ("--nodes", T_STR),
    ("--note", T_STR),
    ("--now", T_INT),
    ("--old", T_STR),
    ("--on", T_BOOL),
    ("--open-by", T_INT),
    ("--open-window", T_INT),
    ("--out", T_PATH),
    ("--out-pack", T_PATH),
    ("--out-receipt", T_PATH),
    ("--pack", T_STR),
    ("--passcode", T_STR),
    ("--payout", T_STR),
    ("--phrase", T_STR),
    ("--price", T_STR),
    ("--price-feed", T_STR),
    ("--priority-fee", T_STR),
    ("--proxy", T_STR),
    ("--pull", T_BOOL),
    ("--q", T_STR),
    ("--receipt", T_STR),
    ("--registry", T_ARR),
    ("--reminder", T_STR),
    ("--revoke", T_ARR),
    ("--role", T_STR),
    ("--roots", T_PATH),
    ("--scan", T_BOOL),
    ("--seat", T_STR),
    ("--segment", T_STR),
    ("--send", T_BOOL),
    ("--series-id", T_STR),
    ("--settle", T_STR),
    ("--sig", T_STR),
    ("--sig-buyer", T_STR),
    ("--sig-seller", T_STR),
    ("--stage", T_INT),
    ("--stages", T_INT),
    ("--t1", T_STR),
    ("--t1h", T_STR),
    ("--t1m", T_STR),
    ("--t2", T_STR),
    ("--t2h", T_STR),
    ("--t2m", T_STR),
    ("--tab", T_STR),
    ("--table", T_PATH),
    ("--terms", T_PATH),
    ("--text", T_STR),
    ("--time-check", T_STR),
    ("--title", T_STR),
    ("--to", T_STR),
    ("--to-again", T_STR),
    ("--to-block", T_STR),
    ("--token", T_STR),
    ("--tx", T_STR),
    ("--units-text", T_BOOL),
    ("--wait", T_INT),
    ("--way", T_STR),
    ("--who", T_STR),
    ("--writer", T_STR),
    ("--zikaron-dir", T_STR),
    ("--zone", T_STR),
];

const WRITES: [&str; 105] = [
    "seller-vault-init", "seller-vault-import", "seller-vault-unlock", "seller-vault-passcode",
    "seller-vault-recover", "seller-vault-new-key", "seller-vault-import-words", "seller-vault-switch", "seller-vault-rotate",
    "seller-vault-remove", "seller-vault-reset", "seller-vault-backup", "seller-vault-restore",
    "seller-vault-rebuild", "seller-vault-phrase", "seller-payout-set", "seller-terms-build", "seller-offer-sign",
    "seller-receipt-paste", "seller-claim", "seller-dossier-export", "seller-product-new", "seller-product-edit",
    "seller-product-save", "seller-product-publish", "seller-product-link", "seller-product-qr",
    "seller-product-resign", "seller-candidate-add", "seller-candidate-remove", "buyer-vault-init",
    "buyer-vault-import", "buyer-vault-unlock", "buyer-vault-passcode", "buyer-vault-recover", "buyer-vault-new-key",
    "buyer-vault-import-words", "buyer-vault-switch", "buyer-vault-rotate", "buyer-vault-remove", "buyer-vault-reset", "buyer-vault-backup",
    "buyer-vault-restore", "buyer-vault-rebuild", "buyer-vault-phrase", "buyer-payout-set", "buyer-open",
    "buyer-order-sign", "buyer-import", "seller-vault-rename", "seller-seat-set", "seller-backup-export",
    "seller-backup-restore", "seller-settings-endpoints", "seller-settings-network", "seller-settings-zikaron",
    "seller-settings-zikaron-link", "seller-settings-zone", "seller-settings-cli-path", "seller-settings-proxy",
    "seller-settings-token-add", "seller-settings-token-remove", "seller-settings-token-decimals",
    "seller-settings-company", "seller-settings-time-check", "seller-settings-past-cases", "seller-note-handle", "seller-export",
    "seller-direct-check", "seller-direct-give-up", "seller-payways-set", "seller-candidate-rename",
    "seller-product-listed", "seller-product-delete", "seller-product-export-proposal", "seller-pending-bump",
    "seller-pending-void", "seller-order-label", "seller-deliver-save", "seller-statement-export",
    "buyer-vault-rename", "buyer-seat-set", "buyer-backup-export", "buyer-backup-restore", "buyer-settings-endpoints", "buyer-settings-network",
    "buyer-settings-zikaron", "buyer-settings-zikaron-link", "buyer-settings-zone",
    "buyer-settings-cli-path", "buyer-settings-proxy", "buyer-settings-token-add", "buyer-settings-token-remove",
    "buyer-settings-token-decimals", "buyer-settings-company", "buyer-settings-time-check", "buyer-settings-past-cases", "buyer-note-handle",
    "buyer-export", "buyer-direct-check", "buyer-direct-give-up", "seller-product-post", "seller-product-revoke",
    "buyer-attach", "buyer-confirm",
];

fn typed(flag: &str) -> Option<(&'static str, &'static str)> {
    TYPES.iter().find(|(n, _)| n.trim_start_matches("--") == flag).copied()
}

pub fn repeats(flag: &str) -> bool {
    typed(flag).is_some_and(|(_, t)| t == T_ARR)
}

pub fn is_path(flag: &str) -> bool {
    typed(flag).is_some_and(|(_, t)| t == T_PATH)
}

pub fn verb_names(buyer: bool) -> Vec<&'static str> {
    (if buyer { seam::BUYER_VERBS } else { seam::SELLER_VERBS }).to_vec()
}

pub fn args_at(seat: &str, verb: &str) -> Option<Vec<(&'static str, &'static str)>> {
    let row = crate::verbs::accepts(&format!("{seat}-{verb}"), false)?;
    Some(row.into_iter().filter_map(typed).collect())
}

pub fn writes(seat: &str, verb: &str) -> bool {
    WRITES.contains(&format!("{seat}-{verb}").as_str())
}

pub fn sends(seat: &str, verb: &str) -> bool {
    args_at(seat, verb).is_some_and(|a| a.iter().any(|(n, _)| *n == "--send"))
}

pub fn passcode_only(verb: &str) -> bool {
    let secret = |n: &str| n.strip_prefix("--").is_some_and(|f| f != "passcode" && eravon_literals::schema::SECRET_FLAGS.contains(&f));
    [seam::SEAT_SELLER, seam::SEAT_BUYER]
        .into_iter()
        .any(|seat| args_at(seat, verb).is_some_and(|a| (verb.starts_with("vault-") && writes(seat, verb) && !sends(seat, verb)) || a.iter().any(|(n, _)| secret(n))))
}

const HERE: [&str; 11] = [
    "seller-vault-lock", "seller-vault-list", "seller-home-stand", "seller-terms-hash", "seller-terms-check",
    "seller-link-encode", "seller-link-decode", "buyer-vault-lock", "buyer-vault-list", "buyer-home-stand", "buyer-choose",
];

pub fn runs_here(flat: &str) -> bool {
    HERE.contains(&flat)
}

pub fn door_row(seat: &str, verb: &str) -> bool {
    (seat == seam::SEAT_SELLER || seat == seam::SEAT_BUYER) && args_at(seat, verb).is_some() && !runs_here(&format!("{seat}-{verb}"))
}

pub fn door_takes(seat: &str, verb: &str) -> bool {
    door_row(seat, verb) && !passcode_only(verb)
}

pub fn row_parts(verb: &str) -> Option<(&'static [&'static str], &'static [&'static str])> {
    crate::verbs_own::ACCEPTS.iter().find(|r| r.0 == verb).map(|r| (r.1, r.2))
}

pub fn door_reads(seat: &str, verb: &str) -> bool {
    door_takes(seat, verb)
        && !writes(seat, verb)
        && !sends(seat, verb)
        && !SIGNING.contains(&verb)
        && !args_at(seat, verb).unwrap_or_default().iter().any(|(n, _)| *n == "--out")
}

const SIGNING: [&str; 9] = ["offer-sign", "order-sign", "product-publish", "product-resign", "settle-offer", "confirm", "dispute", "settle-sign", "complaint-write"];

pub fn signs(seat: &str, verb: &str, args: &[(String, String)]) -> bool {
    let ours = (seat == seam::SEAT_SELLER || seat == seam::SEAT_BUYER) && args_at(seat, verb).is_some();
    let flag = |name: &str| args.iter().any(|(k, v)| k.trim_start_matches('-') == name && v == "true");
    let countersigns = seat == seam::SEAT_BUYER && verb == "settle" && !flag("by-call");
    ours && (SIGNING.contains(&verb) || countersigns || (sends(seat, verb) && flag("send")))
}

pub fn may_wait(seat: &str, verb: &str) -> bool {
    let ours = (seat == seam::SEAT_SELLER || seat == seam::SEAT_BUYER) && args_at(seat, verb).is_some();
    ours && (SIGNING.contains(&verb) || sends(seat, verb))
}

const OWN_CHAIN: [&str; 3] = ["settings-network", "settings-endpoints", "vault-rotate"];

pub fn chain_is_own(verb: &str) -> bool {
    OWN_CHAIN.contains(&verb)
}

pub fn place_flags(seat: &str, verb: &str) -> Vec<&'static str> {
    args_at(seat, verb).unwrap_or_default().into_iter().filter(|(_, t)| *t == T_PATH).map(|(n, _)| n).collect()
}

pub fn path_gate(seat: &str, verb: &str, pairs: &[(String, String)]) -> Result<(), Misuse> {
    let Some(args) = args_at(seat, verb) else { return Ok(()) };
    eravon_literals::schema::one_value(pairs, |k| args.iter().any(|(n, t)| *n == k && *t == T_ARR))?;
    for (k, v) in pairs {
        if v.is_empty() && args.iter().any(|(n, t)| n == k && *t == T_PATH) {
            return Err(Misuse::bad(k, v, eravon_literals::misuse::EMPTY_PATH));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_door_takes_every_verb_that_opens_the_vault() {
        for (seat, verb, taken) in [
            (seam::SEAT_SELLER, "payout-set", false),
            (seam::SEAT_BUYER, "payout-set", false),
            (seam::SEAT_SELLER, "watch", true),
            (seam::SEAT_SELLER, "terms-hash", false),
            (seam::SEAT_BUYER, "choose", false),
            (seam::SEAT_SELLER, "vault-lock", false),
            (seam::SEAT_BUYER, "vault-passcode", false),
            (seam::SEAT_BUYER, "import", true),
            (seam::SEAT_SELLER, "product-new", true),
            (seam::SEAT_SELLER, "fund-check", true),
            (seam::SEAT_SELLER, "order-list", true),
            (seam::SEAT_SELLER, "candidate-list", true),
            (seam::SEAT_BUYER, "list", true),
            (seam::SEAT_BUYER, "offers", true),
            (seam::SEAT_BUYER, "settle-sign", true),
            (seam::SEAT_SELLER, "vault-rotate", true),
            (seam::SEAT_BUYER, "vault-rotate", true),
            ("judge", "product-new", false),
        ] {
            assert_eq!(door_takes(seat, verb), taken, "{seat} {verb}");
        }
    }

    #[test]
    fn the_door_rows_are_the_verb_books_under_one_judgement() {
        let mut want: Vec<String> = Vec::new();
        for (seat, books) in [(seam::SEAT_SELLER, seam::SELLER_VERBS), (seam::SEAT_BUYER, seam::BUYER_VERBS)] {
            want.extend(books.iter().filter(|v| door_row(seat, v)).map(|v| format!("{seat}-{v}")));
        }
        let mut have: Vec<String> = crate::verbs::HOME.iter().map(|(v, _)| v.to_string()).collect();
        want.sort();
        have.sort();
        assert_eq!(have, want, "the door's rows are the verbs that open the vault");
        for flat in HERE {
            assert!(crate::verbwire::split(flat).is_some(), "{flat} is a verb of a seat");
        }
        let secret = |f: &str| eravon_literals::schema::SECRET_FLAGS.contains(&f);
        for (verb, row) in crate::verbs::HOME {
            let (shared, own) = row_parts(verb).expect("a verb");
            let derived: Vec<&str> = shared
                .iter()
                .copied()
                .filter(|f| !["data", "passcode", "rest-keys"].contains(f))
                .chain(own.iter().copied().filter(|f| !secret(f) && *f != crate::verbs::HOME_FLAG))
                .collect();
            assert_eq!(crate::doorverbs::names(row), derived, "{verb}: its door row is its flags less the data root and the secrets");
        }
        for verb in crate::verbs::VERBS {
            let homed = crate::verbs::HOME.iter().any(|(v, _)| *v == verb);
            let takes = row_parts(verb).is_some_and(|(_, own)| own.contains(&crate::verbs::HOME_FLAG));
            assert_eq!(takes, homed, "{verb}: takes --desktop exactly when it is a row of the door");
        }
    }

    #[test]
    fn each_door_flag_is_a_path_exactly_when_its_verb_book_says_so() {
        for (verb, row) in crate::verbs::HOME {
            for (flag, kind) in row.iter() {
                assert_eq!(kind.is_path(), crate::deskwire::is_place(verb, flag), "{verb} --{flag}: its door row's kind against its verb book");
            }
        }
    }

    #[test]
    fn the_key_rotation_goes_through_the_door_by_the_cell() {
        let p = |k: &str, v: &str| (k.to_string(), v.to_string());
        assert!(signs(seam::SEAT_BUYER, "settle", &[]), "the buyer's settlement countersigns, sent or not");
        assert!(!signs(seam::SEAT_BUYER, "settle", &[p("by-call", "true")]), "by call it signs nothing");
        assert!(signs(seam::SEAT_BUYER, "settle", &[p("by-call", "true"), p("send", "true")]), "by call and sent, it sends");
        assert!(!signs(seam::SEAT_SELLER, "settle", &[]), "the seller's settle forms its call from given signatures");
        let send = vec![("send".to_string(), "true".to_string())];
        let list = vec![("continue".to_string(), "true".to_string())];
        for seat in [seam::SEAT_SELLER, seam::SEAT_BUYER] {
            assert!(door_takes(seat, "vault-rotate") && !passcode_only("vault-rotate"), "{seat}: the door takes the rotation");
            assert!(signs(seat, "vault-rotate", &send), "{seat}: a rotation asked to send goes by the cell");
            assert!(!signs(seat, "vault-rotate", &list), "{seat}: its list runs at once");
            for kept in ["vault-new-key", "vault-switch", "vault-remove", "vault-rename", "vault-phrase", "vault-unlock"] {
                assert!(passcode_only(kept) && !door_takes(seat, kept), "{seat} {kept}: the vault's own, done on the desktop");
            }
        }
    }

    #[test]
    fn choosing_the_network_takes_the_endpoint_tables_gate_on_both_seats() {
        for seat in [seam::SEAT_SELLER, seam::SEAT_BUYER] {
            let books = if seat == seam::SEAT_BUYER { seam::BUYER_VERBS } else { seam::SELLER_VERBS };
            assert!(books.contains(&"settings-network") && crate::deskverbs::VERBS.contains(&"settings-network"), "{seat}: in the book and the desktop's table");
            assert_eq!(door_takes(seat, "settings-network"), door_takes(seat, "settings-endpoints"), "{seat}: the door takes both or neither");
            assert!(door_takes(seat, "settings-network") && !passcode_only("settings-network") && !sends(seat, "settings-network"), "{seat}: a local write the door takes");
            let home = |v: &str| crate::verbs::HOME.iter().find(|(n, _)| *n == format!("{seat}-{v}")).map(|(_, r)| crate::doorverbs::names(r)).unwrap_or_default();
            let endpoints: Vec<&str> = home("settings-endpoints").into_iter().filter(|f| *f != "nodes").collect();
            assert_eq!(home("settings-network"), endpoints, "{seat}: its door row is the endpoint table's less the nodes");
            assert!(args_at(seat, "settings-network").is_some_and(|a| a.iter().any(|(n, t)| *n == "--chain" && *t == T_INT)), "{seat}: --chain, a number");
        }
    }

    #[test]
    fn every_verb_that_can_wait_on_the_card_takes_wait_and_no_other() {
        let send = vec![("send".to_string(), "true".to_string())];
        let mut waiting = 0;
        for (flat, row) in crate::verbs::HOME {
            let (seat, verb) = crate::verbwire::split(flat).expect("a seat's verb");
            assert_eq!(crate::doorverbs::names(row).contains(&"wait"), may_wait(seat, verb), "{flat}: takes --wait exactly when it can wait on the card");
            assert!(!signs(seat, verb, &send) || may_wait(seat, verb), "{flat}: a request that signs or sends is of a verb that can wait");
            waiting += usize::from(may_wait(seat, verb));
        }
        for flat in ["seller-offer-sign", "seller-product-resign", "buyer-order-sign", "buyer-settle-sign"] {
            let (seat, verb) = crate::verbwire::split(flat).expect("a seat's verb");
            assert!(may_wait(seat, verb), "{flat}: signs whether or not it sends");
        }
        assert!(waiting > 4);
    }

    #[test]
    fn every_request_that_signs_or_sends_goes_by_the_cell_and_none_else() {
        let send = vec![("--send".to_string(), "true".to_string())];
        let bare = vec![("send".to_string(), "true".to_string())];
        let dry = vec![("--send".to_string(), "false".to_string())];
        let none: Vec<(String, String)> = Vec::new();
        for (seat, verb, args, asks) in [
            (seam::SEAT_SELLER, "offer-sign", &none, true),
            (seam::SEAT_SELLER, "settle-offer", &none, true),
            (seam::SEAT_SELLER, "product-publish", &none, true),
            (seam::SEAT_BUYER, "order-sign", &none, true),
            (seam::SEAT_BUYER, "confirm", &none, true),
            (seam::SEAT_BUYER, "dispute", &dry, true),
            (seam::SEAT_SELLER, "claim", &send, true),
            (seam::SEAT_SELLER, "claim", &bare, true),
            (seam::SEAT_BUYER, "wallet-send", &bare, true),
            (seam::SEAT_SELLER, "refund", &send, true),
            (seam::SEAT_SELLER, "watch", &send, true),
            (seam::SEAT_BUYER, "wallet-send", &send, true),
            (seam::SEAT_SELLER, "claim", &none, false),
            (seam::SEAT_SELLER, "watch", &dry, false),
            (seam::SEAT_SELLER, "watch", &none, false),
            (seam::SEAT_BUYER, "wallet-send", &dry, false),
            (seam::SEAT_SELLER, "settings-zone", &send, false),
            (seam::SEAT_SELLER, "product-save", &none, false),
            (seam::SEAT_BUYER, "order-sign-x", &none, false),
        ] {
            assert_eq!(signs(seat, verb, args), asks, "{seat} {verb} {args:?}");
        }
    }
}
