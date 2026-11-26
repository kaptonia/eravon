pub const V_SCHEMA: &str = "schema";
pub const V_FORM: &str = "form";
pub const V_SKIN: &str = "skin";
pub const V_I18N: &str = "i18n";
pub const V_DRIVE: &str = "drive";
pub const V_PAGE_FRAMES: &str = "page-frames";
pub const V_TRACE: &str = "trace";
pub const V_BUNDLE: &str = "bundle";
pub const V_WINDOW: &str = "window";

pub const ENV_LAUNCH: &str = "ERAVON_LAUNCH";

pub const WORDMARK: &str = eravon_os::APP_FOLDER;

pub const PAYOUT_PLACEHOLDER: &str = "0x…";

pub const STEP_SKIN: &str = "skin";

pub use eravon_store::seam::{D_TRACE, F_TRACE};

pub use eravon_literals::trace::V_TRACE_DOC;

pub fn door_machine() -> Option<std::path::PathBuf> {
    static KEPT: std::sync::Mutex<Option<(std::path::PathBuf, std::path::PathBuf)>> = std::sync::Mutex::new(None);
    let m = crate::home::machine_dir().ok()?;
    let mut kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, real)) = kept.as_ref() {
        if *at == m {
            return Some(real.clone());
        }
    }
    let real = std::fs::canonicalize(&m).unwrap_or_else(|_| m.clone());
    *kept = Some((m, real.clone()));
    Some(real)
}

pub fn user_home(injected: &str) -> String {
    crate::house::platform::home_dir(injected)
}


pub fn downloads(home: &str, _data: &str) -> Result<String, crate::house::seller::Said> {
    let d = crate::house::platform::downloads_dir(home);
    if !home.is_empty() && std::path::Path::new(&d).is_dir() {
        Ok(d)
    } else {
        Err(crate::house::seller::Said::bad("E_APP_NO_DOWNLOADS", crate::fill!(crate::house::i18n::Key::ExNoDownloads; d.to_string())))
    }
}


pub fn qr_file(dir: &str, id: &str, version: u64) -> String {
    std::path::Path::new(dir).join(format!("{id}-v{version}.png")).display().to_string()
}

pub const QR_SAVE_NO_PATH: &str = "pd-qr-save needs path: the folder is the person's choice";

pub fn trace_dir(data: &str) -> String {
    eravon_store::seam::trace_dir(data)
}

pub fn trace_path(data: &str) -> String {
    eravon_store::seam::trace_path(data)
}

pub const SEAT_SELLER: &str = "seller";
pub const SEAT_BUYER: &str = "buyer";

pub const MODE_LIST: &str = "list";
pub const MODE_SUB: &str = "sub";
pub const MODE_SHEET: &str = "sheet";

pub const LINK_OPEN: &str = "open";

pub const LINK_UNKNOWN: &str = "unknown";

pub const CARD_NONE: &str = "none";
pub const CARD_PASSCODE: &str = "passcode";
pub const CARD_DEAD: &str = "dead";
pub const CARD_RECOVER: &str = "recover";
pub const CARD_SETNEW: &str = "setNew";
pub const CARD_AGAIN: &str = "again";

pub const CARD_KEYFILE: &str = "keyFile";

pub const STEP_PIN: &str = "pin";
pub const STEP_UNLOCK: &str = "unlock";
pub const STEP_LOCK: &str = "lock";
pub const STEP_TAKE_OVER: &str = "take-over";
pub const STEP_FORGET: &str = "forget";
pub const STEP_ASIDE: &str = "aside";
pub const STEP_WORDS: &str = "words";
pub const STEP_RECOVER: &str = "recover";
pub const STEP_USE_KEY: &str = "use-key";

pub const STEP_KEYFILE: &str = "key-file";
pub const STEP_ROUTE: &str = "route";
pub const ROUTE_WORDS: &str = "words";
pub const ROUTE_BACKUP: &str = "backup";
pub const ROUTE_KEYFILE: &str = "keyfile";
pub const STEP_PICK: &str = "pick";
pub const STEP_FILE_PASS: &str = "file-pass";
pub const STEP_STAND: &str = "stand";

pub const MODE_FIRST: &str = "first";
pub const MODE_SEAT: &str = "seat";
pub const MODE_RESET: &str = "reset";
pub const MODE_AGAIN: &str = "again";

pub const STEP_IDENTITY: &str = "identity";
pub const STEP_PASSCODE: &str = "passcode";
pub const STEP_ROLE: &str = "role";
pub const STEP_NETWORK: &str = "network";
pub const STEP_PAYOUT: &str = "payout";
pub const STEP_IMPORT: &str = "import";
pub const STEP_DONE: &str = "done";

pub const WAY_PICK: &str = "pick";
pub const WAY_WRITE: &str = "write";
pub const WAY_VERIFY: &str = "verify";
pub const WAY_IMPORT: &str = "import";
pub const WAY_KEYFILE: &str = "keyFile";
pub const WAY_BACKUP: &str = "backup";

pub const STEP_FIRST: &str = "first";
pub const STEP_WAY: &str = "way";
pub const STEP_REVEAL: &str = "reveal";
pub const STEP_ANSWER: &str = "answer";
pub const STEP_ROLE_PICK: &str = "role";
pub const STEP_PAYOUT_SET: &str = "payout";
pub const STEP_SKIP: &str = "skip";
pub const STEP_LAND: &str = "land";
pub const STEP_WIZARD_EXIT: &str = "wizard-exit";
pub const STEP_SELFCHECK: &str = "selfcheck";
pub const STEP_VERIFY: &str = "verify";
pub const STEP_CELLS: &str = "cells";
pub const STEP_PICKFILE: &str = "pick-file";
pub const STEP_FILEPASS: &str = "first-file-pass";
pub const STEP_FIRST_LABEL: &str = "first-label";
pub const STEP_SETPIN: &str = "set-pin";

pub const WRITER: &str = crate::house::bundle::EXECUTABLE;

pub const PAGE_PAYWAYS: &str = "payways";
pub const PAGE_CANDS: &str = "cands";
pub const PAGE_HOME: &str = "home";
pub const PAGE_ALERTS: &str = "alerts";

pub const STEP_FUND: &str = "fund";
pub const STEP_OPEN_WINDOW: &str = "open-window";
pub const STEP_OPEN_AUTO: &str = "open-auto";
pub const STEP_OPEN_BUYER: &str = "open-buyer";
pub const STEP_OPENING_SAVE: &str = "opening-save";

pub const STEP_CANDS_TAB: &str = "cands-tab";
pub const STEP_CAND_ADD: &str = "cand-add";
pub const STEP_CAND_OPEN: &str = "cand-open";
pub const STEP_CAND_TAB: &str = "cand-tab";
pub const STEP_CAND_RENAME: &str = "cand-rename";
pub const STEP_CAND_REMOVE: &str = "cand-remove";
pub const STEP_CAND_BACK: &str = "cand-back";

pub const PAGE_GOODS: &str = "goods";
pub const STEP_PE_NEW: &str = "pe-new";
pub const STEP_PE_OPEN: &str = "pe-open";
pub const STEP_PE_SET: &str = "pe-set";
pub const STEP_PE_SERIES: &str = "pe-series";
pub const STEP_PE_TIER_ADD: &str = "pe-tier-add";
pub const STEP_PE_TIER_DEL: &str = "pe-tier-del";
pub const STEP_PE_TIER_SET: &str = "pe-tier-set";
pub const STEP_PE_CAND: &str = "pe-cand";
pub const STEP_PE_CANDS: &str = "pe-cands";
pub const STEP_PE_TAB: &str = "pe-tab";
pub const STEP_PE_SAVE: &str = "pe-save";
pub const STEP_PE_PUBLISH: &str = "pe-publish";
pub const STEP_PE_IMPORT: &str = "pe-import";
pub const STEP_PE_LEAVE: &str = "pe-leave";
pub const STEP_PE_ASK: &str = "pe-ask";

pub const STEP_GOODS_SEARCH: &str = "goods-search";
pub const STEP_PD_OPEN: &str = "pd-open";
pub const STEP_PD_BACK: &str = "pd-back";
pub const STEP_PD_COPY: &str = "pd-copy";
pub const STEP_PD_PREVIEW: &str = "pd-preview";
pub const STEP_PD_QR: &str = "pd-qr";
pub const STEP_PD_QR_SAVE: &str = "pd-qr-save";
pub const STEP_PD_QR_COPY: &str = "pd-qr-copy";
pub const STEP_PD_QR_CLOSE: &str = "pd-qr-close";
pub const STEP_PD_LIST: &str = "pd-list";
pub const STEP_PD_EDIT: &str = "pd-edit";
pub const STEP_PD_DELETE: &str = "pd-delete";
pub fn company_order_file(dir: &str) -> String {
    std::path::Path::new(dir).join("eravon-order.json").display().to_string()
}

pub fn link_tag(listing: &[u8], candidate: usize) -> String {
    let p = eravon_link::listing::listing(listing).map(|p| p.terms_hash).unwrap_or_default();
    format!("{}-{candidate}", p.trim_start_matches("0x").get(..8).unwrap_or(""))
}

pub fn order_proposal_dir(dir: &str, id: &str, verb: &str) -> String {
    let stem = format!("eravon-{verb}-{id}");
    let mut n = 1u32;
    loop {
        let name = if n == 1 { stem.clone() } else { format!("{stem}-{n}") };
        let at = std::path::Path::new(dir).join(name);
        if std::fs::symlink_metadata(&at).is_err() {
            return at.display().to_string();
        }
        n += 1;
    }
}
pub const STEP_PD_EXPORT_PROPOSAL: &str = "pd-export-proposal";
pub const STEP_PD_COMPANY_CHECK: &str = "pd-company-check";

pub const STEP_CASE_JUDGE: &str = "case-judge";

pub const STEP_CASE_DOCS: &str = "case-docs";

pub const STEP_INNER_BACK: &str = "inner-back";

pub const STEP_LAPSE_OPEN: &str = "lapse-open";

pub const STEP_PD_DOC: &str = "pd-doc";

pub const STEP_PD_IMAGE_CHECK: &str = "pd-image-check";

pub const STEP_PE_IMAGE: &str = "pe-image";

pub const STEP_PE_IMAGE_DEL: &str = "pe-image-del";

pub const STEP_URL_OPEN: &str = "url-open";

pub const PAGE_SETTINGS: &str = "settings";
pub const STEP_SET_SWITCH: &str = "set-switch";
pub const STEP_SET_REMOVE: &str = "set-remove";
pub const STEP_SET_NEW: &str = "set-new";
pub const STEP_SET_NEW_REVEAL: &str = "set-new-reveal";
pub const STEP_SET_NEW_ASK: &str = "set-new-ask";
pub const STEP_SET_NEW_VERIFY: &str = "set-new-verify";
pub const STEP_SET_NEW_CLOSE: &str = "set-new-close";
pub const STEP_SET_IMPORT: &str = "set-import";
pub const STEP_SET_GAS: &str = "set-gas";
pub const STEP_SET_PAYOUT: &str = "set-payout";
pub const STEP_SET_PASSCODE: &str = "set-passcode";
pub const STEP_SET_PHRASE: &str = "set-phrase";
pub const STEP_SET_PHRASE_CLOSE: &str = "set-phrase-close";
pub const STEP_SET_EXPORT: &str = "set-export";
pub const STEP_SET_RESTORE: &str = "set-restore";
pub const STEP_SET_ENDPOINTS: &str = "set-endpoints";
pub const STEP_SET_ZIKARON: &str = "set-zikaron";
pub const STEP_SET_LANG: &str = "set-lang";
pub const STEP_SET_ZONE: &str = "set-zone";
pub const STEP_SET_EVERY: &str = "set-every";
pub const STEP_DOOR_WAIT: &str = "door-wait";
pub const STEP_DOOR_APPROVE: &str = "door-approve";
pub const STEP_DOOR_DECLINE: &str = "door-decline";
pub const STEP_QUEUE_DIRECT_CHECK: &str = "queue-direct-check";
pub const STEP_QUEUE_DIRECT_GIVE_UP: &str = "queue-direct-give-up";
pub const PAGE_QUEUE: &str = "queue";
pub const STEP_SET_WAY: &str = "set-way";
pub const STEP_SET_LOOK: &str = "set-look";
pub const STEP_SET_RESETUP: &str = "set-resetup";
pub const STEP_SET_SECTION: &str = "set-section";
pub const STEP_SET_TOKEN_ADD: &str = "set-token-add";
pub const STEP_SET_TOKEN: &str = "set-token";
pub const STEP_SET_TOKEN_CLOSE: &str = "set-token-close";
pub const STEP_SET_TOKEN_REMOVE: &str = "set-token-remove";
pub const STEP_SET_TOKEN_DECIMALS: &str = "set-token-decimals";
pub const STEP_SET_KEY: &str = "set-key";
pub const STEP_SET_SWITCH_OPEN: &str = "set-switch-open";
pub const STEP_SET_RENAME: &str = "set-rename";
pub const STEP_SET_KEY_CLOSE: &str = "set-key-close";
pub const STEP_SET_ZK_LINK: &str = "set-zk-link";
pub const STEP_SET_CLI_PATH: &str = "set-cli-path";
pub const STEP_SET_CLI_PATH_READ: &str = "set-cli-path-read";
pub const STEP_SET_CLI_DESKTOP: &str = "set-cli-desktop";
pub const STEP_SET_CLI_ALONE: &str = "set-cli-alone";
pub const STEP_SET_CLI_ANCHOR: &str = "set-cli-anchor";
pub const STEP_SET_CLI_PASS_ADD: &str = "set-cli-pass-add";
pub const STEP_SET_CLI_PASS_DROP: &str = "set-cli-pass-drop";
pub const STEP_SET_CLI_PASS_READ: &str = "set-cli-pass-read";
pub const STEP_SET_REST_MODE: &str = "set-rest-mode";
pub const STEP_SET_REST_ADD: &str = "set-rest-add";
pub const STEP_SET_REST_REMOVE: &str = "set-rest-remove";
pub const STEP_SET_COMPANY: &str = "set-company";
pub const STEP_SET_TIME_CHECK: &str = "set-time-check";
pub const STEP_SET_NETWORK: &str = "set-network";
pub const STEP_SET_PAST_CASES: &str = "set-past-cases";
pub const STEP_SET_ROTATE: &str = "set-rotate";
pub const STEP_SET_ROTATE_GO: &str = "set-rotate-go";
pub const STEP_SET_COMPANY_CHECK: &str = "set-company-check";
pub const STEP_SET_AUTOLOCK: &str = "set-autolock";
pub const STEP_SET_BACKUP: &str = "set-backup";
pub const STEP_SET_BACKUP_RESTORE: &str = "set-backup-restore";
pub const STEP_INPUT: &str = "input";
pub const STEP_TICK: &str = "tick";

pub const STEP_HOME_ORDER: &str = "home-order";

pub const STEP_HOME_CAND: &str = "home-cand";

pub const STEP_ORDER_INFO: &str = "order-info";

pub const STEP_PAGE: &str = "page";
pub const STEP_PAGE_MARKED: &str = "page-marked";
pub const STEP_SEAT: &str = "seat";
pub const STEP_ID_OPEN: &str = "id-open";
pub const STEP_ID_CLOSE: &str = "id-close";
pub const STEP_ID_MOVE: &str = "id-move";
pub const STEP_ID_HOVER: &str = "id-hover";
pub const STEP_ID_ENTER: &str = "id-enter";
pub const STEP_ID_ITEM: &str = "id-item";
pub const ID_ITEMS: [&str; 3] = ["new", "import", "account"];
pub const STEP_MODE: &str = "mode";
pub const STEP_CLOSE: &str = "close";
pub const STEP_URL: &str = "url";
pub const STEP_WORD: &str = "word";
pub const STEP_REFRESH: &str = "refresh";

pub const PAGE_ORDERS: &str = "orders";
pub const STEP_ORDER_PASTE: &str = "order-paste";
pub const STEP_ORDER_OPEN: &str = "order-open";
pub const STEP_ORDER_POLL: &str = "order-poll";
pub const STEP_ORDER_SCAN: &str = "order-scan";
pub const STEP_PENDING_BUMP: &str = "pending-bump";
pub const STEP_PENDING_VOID: &str = "pending-void";
pub const STEP_DELIVER_OPEN: &str = "deliver-open";
pub const STEP_DELIVER_PICK: &str = "deliver-pick";
pub const STEP_DELIVER_NOTE: &str = "deliver-note";
pub const STEP_DELIVER_METHOD: &str = "deliver-method";
pub const STEP_DELIVER_SEND: &str = "deliver-send";
pub const STEP_DELIVER_CLOSE: &str = "deliver-close";
pub const STEP_TODO_COPY: &str = "todo-copy";
pub const STEP_ORDER_BACK: &str = "order-back";
pub const STEP_ORDER_TAB: &str = "order-tab";
pub const STEP_ORDER_LIST_TAB: &str = "order-list-tab";
pub const STEP_ORDER_SEARCH: &str = "order-search";
pub const STEP_ORDER_SEARCH_OFF: &str = "order-search-off";
pub const STEP_ORDER_ROW: &str = "order-row";
pub const STEP_ORDER_LABEL: &str = "order-label";
pub const STEP_ORDER_TERMS: &str = "order-terms";
pub const STEP_ORDER_HISTORY: &str = "order-history";
pub const STEP_ORDER_PASTE_OPEN: &str = "order-paste-open";
pub const STEP_ORDER_PASTE_CLOSE: &str = "order-paste-close";
pub const STEP_DELIVER_SAVE: &str = "deliver-save";
pub const STEP_EVIDENCE_OWN: &str = "evidence-own";

pub const STEP_NOTE_HANDLE: &str = "note-handle";
pub const STEP_NOTE_COPY: &str = "note-copy";
pub const STEP_HOME_REFRESH: &str = "home-refresh";

pub fn s8_step(what: &str) -> bool {
    matches!(what, STEP_NOTE_HANDLE | STEP_NOTE_COPY | STEP_HOME_REFRESH | STEP_HOME_ORDER | STEP_HOME_CAND)
}

pub fn s2_step(what: &str) -> bool {
    matches!(
        what,
        STEP_ORDER_INFO
            | STEP_ORDER_BACK
            | STEP_ORDER_TAB
            | STEP_ORDER_LIST_TAB
            | STEP_ORDER_SEARCH
            | STEP_ORDER_SEARCH_OFF
            | STEP_ORDER_ROW
            | STEP_ORDER_LABEL
            | STEP_ORDER_TERMS
            | STEP_ORDER_HISTORY
            | STEP_ORDER_PASTE_OPEN
            | STEP_ORDER_PASTE_CLOSE
    )
}

pub const STEP_REFUND_OPEN: &str = "refund-open";
pub const STEP_REFUND_SEND: &str = "refund-send";
pub const STEP_REFUND_CLOSE: &str = "refund-close";
pub const STEP_EVIDENCE_OPEN: &str = "evidence-open";
pub const STEP_EVIDENCE_KIT: &str = "evidence-kit";
pub const STEP_EVIDENCE_PICK: &str = "evidence-pick";
pub const STEP_EVIDENCE_NOTE: &str = "evidence-note";
pub const STEP_EVIDENCE_SEND: &str = "evidence-send";
pub const STEP_EVIDENCE_CLOSE: &str = "evidence-close";
pub const STEP_SETTLE_OPEN: &str = "settle-open";
pub const STEP_SETTLE_MAKE: &str = "settle-make";
pub const STEP_SETTLE_PREVIEW: &str = "settle-preview";
pub const STEP_SETTLE_CLOSE: &str = "settle-close";
pub const STEP_SIG_SETTLE: &str = "sig-settle";
pub const STEP_SIG_CONFIRM: &str = "sig-confirm";

pub fn dispute_step(what: &str) -> bool {
    ["refund-", "evidence-", "settle-", "sig-"].iter().any(|p| what.starts_with(p)) || what == STEP_TIME_ASK
}

pub const STEP_TIME_ASK: &str = "time-ask";

pub const STEP_PD_POST: &str = "pd-post";

pub const STEP_PD_FEES_CONFIRM: &str = "pd-fees-confirm";

pub const STEP_EVIDENCE_JUDGE: &str = "evidence-judge";

pub const STEP_EVIDENCE_JUDGE_CLOSE: &str = "evidence-judge-close";

pub const STEP_EVIDENCE_SEAL: &str = "evidence-seal";

pub const STEP_BO_EVIDENCE_JUDGE: &str = "bo-evidence-judge";

pub const STEP_BO_EVIDENCE_JUDGE_CLOSE: &str = "bo-evidence-judge-close";
pub const STEP_BO_HISTORY: &str = "bo-history";

pub const STEP_BO_EVIDENCE_SEAL: &str = "bo-evidence-seal";

pub const STEP_COMPLAINT_OPEN: &str = "complaint-open";

pub const STEP_COMPLAINT_WRITE: &str = "complaint-write";

pub const STEP_PD_REVOKE: &str = "pd-revoke";

pub use eravon_cli::deskops::buyerpage::wallet_uri;

#[cfg(test)]
mod wallet_uri_tests {
    #[test]
    fn a_call_is_spelled_as_a_wallet_reads_it() {
        let word = |n: u8| {
            let mut w = [0u8; 32];
            w[31] = n;
            w
        };
        let call = |sel: [u8; 4], a: [u8; 32], b: [u8; 32]| [sel.as_slice(), &a, &b].concat();
        let to = "0x00000000000000000000000000000000000000AA";
        let t = call([0xa9, 0x05, 0x9c, 0xbb], word(0xbb), word(7));
        assert_eq!(super::wallet_uri(1, to, &t, "0"), "ethereum:0x00000000000000000000000000000000000000aa@1/transfer?address=0x00000000000000000000000000000000000000bb&uint256=7");
        let ap = call([0x09, 0x5e, 0xa7, 0xb3], word(0xcc), word(0));
        assert!(super::wallet_uri(11155111, to, &ap, "0").ends_with("@11155111/approve?address=0x00000000000000000000000000000000000000cc&uint256=0"));
        assert_eq!(super::wallet_uri(1, to, &[], "5"), "ethereum:0x00000000000000000000000000000000000000aa@1?value=5");
        let other = call([1, 2, 3, 4], word(1), word(2));
        assert!(super::wallet_uri(1, to, &other, "0").contains("?value=0&data=0x01020304"));
        assert!(super::wallet_uri(1, to, &t[..t.len() - 1], "0").contains("&data=0xa9059cbb"), "a word short is calldata");
        assert!(super::wallet_uri(1, to, &[t.as_slice(), &[0]].concat(), "0").contains("&data=0xa9059cbb"), "a byte over is calldata");
    }
}

pub const STEP_WL_VAULT_SETUP: &str = "wl-vault-setup";

pub const STEP_WL_VAULT_CLOSE: &str = "wl-vault-close";

pub const STEP_WL_FUND: &str = "wl-fund";

pub const STEP_WL_FUND_OWN: &str = "wl-fund-own";

pub const STEP_BO_CANCEL: &str = "bo-cancel";

pub const STEP_BO_OPEN_CASE: &str = "bo-open-case";

pub const STEP_BO_OFFERS_SCAN: &str = "bo-offers-scan";

pub const STEP_PL_ATTACH: &str = "pl-attach";

pub const STEP_BO_CONFIRM_HAND: &str = "bo-confirm-hand";

pub const STEP_ORDER_COMPANY_OPEN: &str = "order-company-open";

pub const STEP_ORDER_DEPLOY: &str = "order-deploy";

pub const STEP_ORDER_ADVANCE: &str = "order-advance";

include!(concat!(env!("OUT_DIR"), "/steps.rs"));

pub const IDLE_POLL_SECS: u64 = 15;

pub const PAGE_PAYLINK: &str = "paylink";
pub const PL_TAB_PAY: &str = "pay";
pub const PL_TAB_IMPORT: &str = "import";
pub use eravon_cli::deskops::paypage::{
    PL_STEP_ACT, PL_STEP_BOOK, PL_STEP_BOUND, PL_STEP_CONSENT, PL_STEP_CURRENT, PL_STEP_DEPOSIT, PL_STEP_JUDGES, PL_STEP_PAYLOAD, PL_STEP_PLACE, PL_STEP_PRICE, PL_STEP_QUOTE, PL_STEP_REVOKED, PL_STEP_SERIES, PL_STEP_STANDARD, PL_STEP_TERMS,
};
pub use eravon_cli::deskops::paypage::{PL_OVER_CAP, PL_PARTY};
pub const PL_KEY_UNOPENED: &str = "请先打开付款链接";
pub const PL_KEY_FAILED: &str = "链接未通过验证";
pub const PICK_NO_PANEL: &str = "无法打开文件面板,可将文件拖入";
pub const PL_KEY_NO_PICK: &str = "先择一位候选";
pub const PL_KEY_NO_PAYOUT: &str = "请填写退款地址";
pub const PL_KEY_PAID: &str = "该订单已付款";

pub const PL_LOCKED_BEFORE: &str = "锁定了:这一单已签并归档,转账与开案一笔也没发";
pub const PL_LOCKED_BETWEEN: &str = "锁定了:钱已转到预算地址,开案那一笔没发(解锁之后可再付款开案,或过期退款)";

pub const BO_STEP_PREFIX: &str = "bo-";
pub const STEP_BO_LIST_TAB: &str = "bo-list-tab";
pub const STEP_BO_OPEN: &str = "bo-open";
pub const STEP_BO_TAB: &str = "bo-tab";
pub const STEP_BO_RELOAD: &str = "bo-reload";
pub const STEP_BO_CONFIRM_OPEN: &str = "bo-confirm-open";
pub const STEP_BO_CONFIRM: &str = "bo-confirm";
pub const STEP_BO_REFUND_OPEN: &str = "bo-refund-open";
pub const STEP_BO_DISPUTE: &str = "bo-dispute";
pub const STEP_BO_COMMIT: &str = "bo-commit";
pub const STEP_BO_OFFER: &str = "bo-offer";
pub const STEP_BO_SETTLE: &str = "bo-settle";
pub const STEP_BO_ADVANCE: &str = "bo-advance";
pub const STEP_BO_EXPIRE: &str = "bo-expire";
pub const STEP_BO_OWED: &str = "bo-owed";
pub const STEP_BO_SWEEP: &str = "bo-sweep";
pub const STEP_BO_EVIDENCE_OPEN: &str = "bo-evidence-open";
pub const STEP_BO_SEARCH: &str = "bo-search";

pub const PAGE_WALLET: &str = "wallet";
pub const WL_STEP_PREFIX: &str = "wl-";
pub const STEP_WL_RELOAD: &str = "wl-reload";
pub const STEP_WL_SEND: &str = "wl-send";
pub const STEP_WL_FORMER_MOVE: &str = "wl-former-move";
pub const STEP_WL_ALL: &str = "wl-all";
pub const STEP_WL_SHEET: &str = "wl-sheet";
pub const STEP_WL_TX: &str = "wl-tx";
pub const STEP_WL_COPY: &str = "wl-copy";

pub const PAGE_EXPORT: &str = "export";
pub const EX_STEP_PREFIX: &str = "ex-";
pub const STEP_EX_TAB: &str = "ex-tab";
pub const STEP_EX_PICKER: &str = "ex-picker";
pub const STEP_EX_SEARCH: &str = "ex-search";
pub const STEP_EX_PICK: &str = "ex-pick";
pub const STEP_EX_DIR: &str = "ex-dir";
pub const STEP_EX_RUN: &str = "ex-run";
pub const STEP_EX_RELOAD: &str = "ex-reload";
pub const STEP_ORDER_EXPORT: &str = "order-export";
pub const STEP_BO_EXPORT: &str = "bo-export";

pub const PL_STEP_PREFIX: &str = "pl-";
pub const STEP_PL_TAB: &str = "pl-tab";
pub const STEP_PL_OPEN: &str = "pl-open";
pub const STEP_PL_PICK: &str = "pl-pick";
pub const STEP_PL_FILE: &str = "pl-file";
pub const STEP_PL_IMPORT: &str = "pl-import";
pub const STEP_PL_BUMP: &str = "pl-bump";
pub const STEP_PL_HISTORY: &str = "pl-history";
pub const STEP_PL_PAYOUT: &str = "pl-payout";
pub const STEP_PL_PAY: &str = "pl-pay";
pub const STEP_PL_FORM: &str = "pl-form";
pub const STEP_PL_COMPANY_OPEN: &str = "pl-company-open";

pub const STEP_WIN: &str = "win";
pub const STEP_WIN_PAGE: &str = "win-page";
pub const STEP_WIN_RAIL: &str = "win-rail";
pub const STEP_WIN_LOCK: &str = "win-lock";
pub const STEP_WIN_UNLOCK: &str = "win-unlock";
pub const STEP_WIN_KEY: &str = "win-key";
pub const STEP_WIN_ENDPOINTS: &str = "win-endpoints";
pub const STEP_WIN_SYNC: &str = "win-sync";
pub const STEP_WIN_GAS: &str = "win-gas";
pub const STEP_WIN_WALLET_FORM: &str = "win-wallet-form";
pub const STEP_WIN_WALLET_SEND: &str = "win-wallet-send";
pub const STEP_WIN_AWAIT: &str = "win-await";
pub const STEP_WIN_RELEASE: &str = "win-release";
pub const STEP_WIN_JOIN: &str = "win-join";
pub const STEP_WIN_FRAME: &str = "win-frame";
pub const STEP_WIN_SETTLE: &str = "win-settle";
pub const STEP_WIN_PICK: &str = "win-pick";
pub const STEP_WIN_VAULT: &str = "win-vault";

#[cfg(test)]
mod tests {
    #[test]
    fn a_proposal_exported_again_takes_the_next_free_name() {
        let dir = std::env::temp_dir().join(format!("eravon-seam-proposal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");
        let d = dir.display().to_string();
        let first = super::order_proposal_dir(&d, "7", "settle");
        assert_eq!(first, dir.join("eravon-settle-7").display().to_string());
        std::fs::create_dir_all(&first).expect("the first export");
        assert_eq!(super::order_proposal_dir(&d, "7", "settle"), dir.join("eravon-settle-7-2").display().to_string());
        std::fs::write(dir.join("eravon-settle-7-2"), b"x").expect("a file at the next name");
        #[cfg(unix)]
        std::os::unix::fs::symlink(dir.join("nowhere"), dir.join("eravon-settle-7-3")).expect("a link at the one after");
        let next = if cfg!(unix) { "eravon-settle-7-4" } else { "eravon-settle-7-3" };
        assert_eq!(super::order_proposal_dir(&d, "7", "settle"), dir.join(next).display().to_string());
        assert_eq!(super::order_proposal_dir(&d, "8", "settle"), dir.join("eravon-settle-8").display().to_string(), "another order is its own");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

pub(crate) fn import_seat(form: &crate::action::ImportForm) -> Result<crate::roles::Role, crate::fault::Fault> {
    match form {
        crate::action::ImportForm::Words(_) => Ok(crate::roles::Role::Grantee),
        _ => Err(crate::fault::Fault::known(crate::fault::Known::PartyKey, String::new())),
    }
}

