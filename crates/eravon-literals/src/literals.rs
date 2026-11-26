pub const DOC_TERMS: &str = "eravon.terms/1";
pub const DOC_LISTING: &str = "eravon.listing/1";
pub const DOC_LINK: &str = "eravon.link/1";
pub const DOC_RECEIPT: &str = "eravon.receipt/1";
pub const DOC_PACK: &str = "eravon.pack/1";
pub const DOC_SETTLE: &str = "eravon.settle/1";
pub const DOC_DOSSIER: &str = "eravon.dossier/1";

pub const DOCUMENT_VERSIONS: [&str; 7] =
    [DOC_TERMS, DOC_LISTING, DOC_LINK, DOC_RECEIPT, DOC_PACK, DOC_SETTLE, DOC_DOSSIER];

pub const DOC_CHELEK_PROPOSAL: &str = "chelek.proposal/1";

pub const DOC_COMPLAINT: &str = "eravon.complaint/1";

pub const COMPANY_VIEWS: [&str; 4] = ["calendarBound()", "wallet()", "ledger()", "holds(address)"];

pub const TYPE_OUTCOME: &str = "Outcome(uint16 buyerBps,uint16 sellerBps,uint8 feeBearer)";
pub const TYPE_OFFER: &str = "Offer(address sellerKey,address judge,address guild,address sellerPayout,address sellerFund,address token,uint256 amount,uint32 deliveryWindow,uint32 confirmWindow,uint32 openWindow,bool buyerMayOpen,Outcome[] ladder,bytes32 termsHash,bytes32 amanahHash,bytes32 quoteHash,uint256 nonce,uint256 expiry)Outcome(uint16 buyerBps,uint16 sellerBps,uint8 feeBearer)";
pub const TYPE_ORDER_TERMS: &str = "OrderTerms(Offer offer,address buyerKey,address buyerPayout,uint64 openBy,address prev)Offer(address sellerKey,address judge,address guild,address sellerPayout,address sellerFund,address token,uint256 amount,uint32 deliveryWindow,uint32 confirmWindow,uint32 openWindow,bool buyerMayOpen,Outcome[] ladder,bytes32 termsHash,bytes32 amanahHash,bytes32 quoteHash,uint256 nonce,uint256 expiry)Outcome(uint16 buyerBps,uint16 sellerBps,uint8 feeBearer)";
pub const TYPE_EIP712_DOMAIN: &str =
    "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)";

pub const TYPE_CLAIM_DELIVERY: &str = "ClaimDelivery(bytes32 manifest)";
pub const TYPE_CONFIRM: &str = "Confirm()";
pub const TYPE_DISPUTE: &str = "Dispute()";
pub const TYPE_REFUND: &str = "Refund()";
pub const TYPE_SETTLE: &str = "Settle(uint16 buyerBps,uint256 expiry)";
pub const TYPE_CANCEL: &str = "Cancel()";

pub const TYPE_STRINGS: [&str; 10] = [
    TYPE_OUTCOME,
    TYPE_OFFER,
    TYPE_ORDER_TERMS,
    TYPE_EIP712_DOMAIN,
    TYPE_CLAIM_DELIVERY,
    TYPE_CONFIRM,
    TYPE_DISPUTE,
    TYPE_REFUND,
    TYPE_SETTLE,
    TYPE_CANCEL,
];

pub const DOMAIN_NAME: &str = "ERAVON";
pub const DOMAIN_VERSION: &str = "1";

pub const PREFIX_LINK_SCHEME: &str = "eravon://open?e=";
pub const PREFIX_RECEIPT: &str = "eravon-receipt:";

pub const TRANSPORT_PREFIXES: [&str; 2] = [PREFIX_LINK_SCHEME, PREFIX_RECEIPT];

pub const TRANSACTION_KINDS: [&str; 23] = [
    "post", "fund", "deploy", "init", "accept", "delivery", "confirm", "dispute", "commit", "settle", "rule", "resign",
    "refund", "expire", "cancel", "revoke", "advance", "redraw", "setPayout", "withdraw", "sweep", "consent", "fundBind",
];

pub const TERMINAL_NAMES: [&str; 6] =
    ["Refunded", "Released", "Settled", "Ruled", "Resigned", "Lapsed"];

pub const CAUSE_T1: u8 = 0;
pub const CAUSE_OFFER_CLOCK: u8 = 1;
pub const CAUSE_EXPIRY: u8 = 2;
pub const CAUSE_SELLER_REFUND: u8 = 3;
pub const CAUSE_BUYER_CANCEL: u8 = 4;

pub const DOSSIER_INDEX_FILE: &str = "dossier.json";

pub const CASE_STATES: [&str; 4] = ["None", "Funded", "Claimed", "Disputed"];

pub const TERMS_FENCE_OPEN: &str = "```eravon.terms/1";
pub const TERMS_FENCE_CLOSE: &str = "```";

pub const EIP712_DIGEST_PREFIX: &str = "0x1901";

pub const PROXY_INITCODE_PREFIX: &str = "0x3d602d80600a3d3981f3363d3d373d3d3d363d73";
pub const PROXY_INITCODE_SUFFIX: &str = "0x5af43d82803e903d91602b57fd5bf3";

pub const QUOTE_TYPE: &str = "(address token,uint16 premiumBps,address feeUnit,address feedToken,address feedUnit,uint32 maxStale,uint256 feeFixed,uint16 feeBps,uint256 feeMin,uint256 feeMax,uint256 amountCap,uint32 rulingWindow,uint32 offerWindow,uint8 lapseDefault,bool autoAccept)";

pub const EXTRA_TYPE: &str = "";

pub const SALT_LOGIC: &str = "eravon/1";
pub const SALT_PANEL_BOOK: &str = "eravon/1/panel";
pub const SALT_VERIFIER: &str = "eravon/1/verify";
pub const SALT_BOARD: &str = "eravon/1/board";

pub const ZIKARON_KIT_PREFIX: &str = "zikaron-kit/";
pub const ZIKARON_POINTER_FILE: &str = ".zikaron-desk";
pub const ZIKARON_DEFAULT_DIR: &str = ".zikaron-desk.d";
pub const ZIKARON_LEGACY_DIR: &str = "Library/Application Support/ZIKARON";
pub const ZIKARON_INDEX_PATH: &str = "kits/index.json";
pub const ZIKARON_INDEX_FORM: &str = "zikaron.kits-index/1";

pub const URL_SCHEME: &str = "eravon";

pub const BUNDLE_ID: &str = "com.kaptonia.eravon.app";

pub const LOCAL_KEY_INFO: &str = "eravon/local/v1";

pub const BACKUP_SUFFIX: &str = ".eravon";

pub const BACKUP_MAGIC: &str = "eravon-backup/1\n";

pub const LOCAL_DATA_KINDS: [&str; 15] = [
    "eravon/local/global-settings",
    "eravon/local/product",
    "eravon/local/draft",
    "eravon/local/terms-original",
    "eravon/local/offer-signature",
    "eravon/local/payload",
    "eravon/local/receipt",
    "eravon/local/order-archive",
    "eravon/local/candidates",
    "eravon/local/order-label",
    "eravon/local/tx-receipt",
    "eravon/local/notice-read",
    "eravon/local/identity-settings",
    "eravon/local/reading",
    "eravon/local/send-queue",
];

pub const VAULT_FORMAT: &str = "eravon.store.vault/3";

pub const NAME_KINDS: [&str; 6] = [
    "eravon/name/identity",
    "eravon/name/products",
    "eravon/name/orders",
    "eravon/name/cases",
    "eravon/name/packs",
    "eravon/name/staging",
];

pub const LOCAL_BIND_INFO: &str = "eravon/local-bind/v1";
