use crate::literals as lit;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    DocumentVersion,
    TypedDataType,
    TypedDataDomain,
    TransportPrefix,
    TransactionKind,
    TerminalName,
    CaseState,
    FileName,
    FenceLine,
    DeploySalt,
    TypedDataPrefix,
    ProxyCodeFragment,
    AbiTuple,
    ZikaronSeam,
    PlatformIdentifier,
    LocalData,
    LocalDataKind,
    NameKind,
    CompanyView,
    ProductName,
    RequestProductName,
    DocumentForm,
    Environment,
}

impl Role {
    pub const fn name(self) -> &'static str {
        match self {
            Role::DocumentVersion => "document-version",
            Role::TypedDataType => "typed-data-type",
            Role::TypedDataDomain => "typed-data-domain",
            Role::TransportPrefix => "transport-prefix",
            Role::TransactionKind => "transaction-kind",
            Role::TerminalName => "terminal-name",
            Role::CaseState => "case-state",
            Role::FileName => "file-name",
            Role::FenceLine => "fence-line",
            Role::DeploySalt => "deploy-salt",
            Role::TypedDataPrefix => "typed-data-prefix",
            Role::ProxyCodeFragment => "proxy-code-fragment",
            Role::AbiTuple => "abi-tuple",
            Role::ZikaronSeam => "zikaron-seam",
            Role::PlatformIdentifier => "platform-identifier",
            Role::LocalData => "local-data",
            Role::LocalDataKind => "local-data-kind",
            Role::NameKind => "name-kind",
            Role::CompanyView => "company-view",
            Role::ProductName => "product-name",
            Role::RequestProductName => "request-product-name",
            Role::DocumentForm => "document-form",
            Role::Environment => "environment",
        }
    }

    pub fn family(self, literal: &str) -> &'static str {
        match self {
            Role::DocumentVersion | Role::TransactionKind | Role::TerminalName | Role::CaseState | Role::FenceLine
            | Role::DeploySalt | Role::ProxyCodeFragment | Role::AbiTuple | Role::CompanyView => "spec-value",
            Role::TypedDataType | Role::TypedDataDomain | Role::TypedDataPrefix => "signing-domain",
            Role::TransportPrefix => "payload-prefix",
            Role::FileName | Role::PlatformIdentifier | Role::ZikaronSeam | Role::ProductName => "name",
            Role::RequestProductName => "request-product-name",
            Role::LocalDataKind | Role::DocumentForm => "document-form",
            Role::NameKind => "derivation-purpose",
            Role::Environment => "environment",
            Role::LocalData => {
                if literal == lit::BACKUP_MAGIC || (literal.starts_with("eravon-local/") && literal.ends_with('\n')) {
                    "file-magic"
                } else if literal == lit::BACKUP_SUFFIX {
                    "name"
                } else if literal == lit::VAULT_FORMAT {
                    "document-form"
                } else {
                    "derivation-purpose"
                }
            }
        }
    }

    pub const ALL: [Role; 23] = [
        Role::DocumentVersion,
        Role::TypedDataType,
        Role::TypedDataDomain,
        Role::TransportPrefix,
        Role::TransactionKind,
        Role::TerminalName,
        Role::CaseState,
        Role::FileName,
        Role::FenceLine,
        Role::DeploySalt,
        Role::TypedDataPrefix,
        Role::ProxyCodeFragment,
        Role::AbiTuple,
        Role::ZikaronSeam,
        Role::PlatformIdentifier,
        Role::LocalData,
        Role::LocalDataKind,
        Role::NameKind,
        Role::CompanyView,
        Role::ProductName,
        Role::RequestProductName,
        Role::DocumentForm,
        Role::Environment,
    ];
}

#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub literal: &'static str,
    pub at: &'static str,
    pub also: &'static [&'static str],
    pub role: Role,
    pub sources: &'static [&'static str],
}

const fn e(literal: &'static str, at: &'static str, role: Role, sources: &'static [&'static str]) -> Entry {
    Entry { literal, at, also: &[], role, sources }
}

const fn e_at(literal: &'static str, at: &'static str, also: &'static [&'static str], role: Role, sources: &'static [&'static str]) -> Entry {
    Entry { literal, at, also, role, sources }
}

impl Entry {
    pub fn places(&self) -> Vec<&'static str> {
        std::iter::once(self.at).chain(self.also.iter().copied()).collect()
    }
}

const S_TERMS: &[&str] = &["eravon.terms/1 §2.3", "eravon.terms/1 §7"];
const S_LISTING_V: &[&str] = &["eravon.link/1 §2.1", "eravon.link/1 §9"];
const S_LINK_V: &[&str] = &["eravon.link/1 §3.1", "eravon.link/1 §9"];
const S_RECEIPT: &[&str] = &["eravon.link/1 §4.1", "eravon.link/1 §9"];
const S_PACK: &[&str] = &["eravon.link/1 §5.1", "eravon.link/1 §9"];
const S_SETTLE_V: &[&str] = &["eravon.link/1 §8.1", "eravon.link/1 §9"];
const S_DOSSIER_V: &[&str] = &["eravon.dossier/1 §2.1", "eravon.dossier/1 §4"];
const S_CHELEK_PROPOSAL: &[&str] =
    &["chosen by this product: company mode proposal file"];
const S_COMPLAINT: &[&str] = &["chosen by this product: complaint file about a case's seated judge (docs/complaint.md)"];
const S_TYPE_712: &[&str] = &["eravon.terms/1 §4.2", "eravon.terms/1 §7"];
const S_TYPE_ACTION: &[&str] = &["eravon.terms/1 §4.5", "eravon.terms/1 §7"];
const S_DOMAIN: &[&str] = &["eravon.terms/1 §4.2", "eravon.terms/1 §7"];
const S_LINK_TRANSPORT: &[&str] = &["eravon.link/1 §3.2", "eravon.link/1 §9"];
const S_RECEIPT_TRANSPORT: &[&str] = &["eravon.link/1 §4.3", "eravon.link/1 §9"];
const S_TX_KIND: &[&str] = &["eravon.dossier/1 §2.3", "eravon.dossier/1 §4"];
const S_TERMINAL: &[&str] = &["eravon.dossier/1 §2.4", "eravon-standard-v1.md §5"];
const S_CASE_STATE: &[&str] = &["eravon-standard-v1.md §5"];
const S_INDEX_FILE: &[&str] = &["eravon.dossier/1 §2.6", "eravon.dossier/1 §4"];
const S_FENCE: &[&str] = &["eravon.terms/1 §2.2", "eravon.terms/1 §7"];
const S_EIP712_PREFIX: &[&str] = &["eravon.terms/1 §4.2", "eravon.terms/1 §7"];
const S_PROXY_CODE: &[&str] =
    &["eravon.terms/1 §1", "eravon.terms/1 §7", "eravon-standard-v1.md §1"];
const S_QUOTE_TYPE: &[&str] =
    &["eravon-standard-v1.md §3", "eravon.terms/1 §3.1", "eravon.terms/1 §7"];
const S_SALT_LOGIC: &[&str] =
    &["eravon-standard-v1.md §11"];
const S_SALT_PANEL: &[&str] = &["eravon-standard-v1.md §7", "base/eravon-standard/CODEHASH.md"];
const S_ZIKARON: &[&str] = &["zikaron.kit/1", "chosen by this product: local ZIKARON discovery"];
const S_SALT_VERIFIER: &[&str] =
    &["eravon-standard-v1.md, Shape"];
const S_SALT_BOARD: &[&str] = &["eravon-standard-v1.md §7b", "base/eravon-standard/CODEHASH.md"];
const S_PLATFORM: &[&str] = &["chosen by this product: URL scheme and bundle identifier"];
const S_NAMES: &[&str] = &["chosen by this product: key store format and file name derivation"];
const S_COMPANY_VIEW: &[&str] =
    &["CHELEK standard (Chelek.sol)", "chosen by this product: company mode views"];
const S_LOCAL: &[&str] = &["chosen by this product: local data encryption and backups"];
const S_PRODUCT: &[&str] = &["chosen by this product: its names"];
const S_LOWER_NAME: &[&str] = &["chosen by this product: its names", "chosen by this product: URL scheme and bundle identifier"];
const S_FORMS: &[&str] = &["chosen by this product: the format of its own files"];
const S_LAYOUT: &[&str] = &["chosen by this product: the data root's layout and the single-writer lock"];
const S_ENV: &[&str] = &["chosen by this product: environment variables it reads or passes"];

pub const ENTRIES: &[Entry] = FULL_ENTRIES;


const FULL_ENTRIES: &[Entry] = &[
    e(lit::DOC_TERMS, "eravon_literals::literals::DOC_TERMS", Role::DocumentVersion, S_TERMS),
    e(lit::DOC_LISTING, "eravon_literals::literals::DOC_LISTING", Role::DocumentVersion, S_LISTING_V),
    e(lit::DOC_LINK, "eravon_literals::literals::DOC_LINK", Role::DocumentVersion, S_LINK_V),
    e(lit::DOC_RECEIPT, "eravon_literals::literals::DOC_RECEIPT", Role::DocumentVersion, S_RECEIPT),
    e(lit::DOC_PACK, "eravon_literals::literals::DOC_PACK", Role::DocumentVersion, S_PACK),
    e(lit::DOC_SETTLE, "eravon_literals::literals::DOC_SETTLE", Role::DocumentVersion, S_SETTLE_V),
    e(lit::DOC_DOSSIER, "eravon_literals::literals::DOC_DOSSIER", Role::DocumentVersion, S_DOSSIER_V),
    e(lit::DOC_CHELEK_PROPOSAL, "eravon_literals::literals::DOC_CHELEK_PROPOSAL", Role::DocumentVersion, S_CHELEK_PROPOSAL),
    e(lit::DOC_COMPLAINT, "eravon_literals::literals::DOC_COMPLAINT", Role::DocumentVersion, S_COMPLAINT),
    e(lit::TYPE_OUTCOME, "eravon_literals::literals::TYPE_OUTCOME", Role::TypedDataType, S_TYPE_712),
    e(lit::TYPE_OFFER, "eravon_literals::literals::TYPE_OFFER", Role::TypedDataType, S_TYPE_712),
    e(lit::TYPE_ORDER_TERMS, "eravon_literals::literals::TYPE_ORDER_TERMS", Role::TypedDataType, S_TYPE_712),
    e(lit::TYPE_EIP712_DOMAIN, "eravon_literals::literals::TYPE_EIP712_DOMAIN", Role::TypedDataType, S_TYPE_712),
    e(lit::TYPE_CLAIM_DELIVERY, "eravon_literals::literals::TYPE_CLAIM_DELIVERY", Role::TypedDataType, S_TYPE_ACTION),
    e(lit::TYPE_CONFIRM, "eravon_literals::literals::TYPE_CONFIRM", Role::TypedDataType, S_TYPE_ACTION),
    e(lit::TYPE_DISPUTE, "eravon_literals::literals::TYPE_DISPUTE", Role::TypedDataType, S_TYPE_ACTION),
    e(lit::TYPE_REFUND, "eravon_literals::literals::TYPE_REFUND", Role::TypedDataType, S_TYPE_ACTION),
    e(lit::TYPE_SETTLE, "eravon_literals::literals::TYPE_SETTLE", Role::TypedDataType, S_TYPE_ACTION),
    e(lit::TYPE_CANCEL, "eravon_literals::literals::TYPE_CANCEL", Role::TypedDataType, S_TYPE_ACTION),
    e(lit::DOMAIN_NAME, "eravon_literals::literals::DOMAIN_NAME", Role::TypedDataDomain, S_DOMAIN),
    e(lit::DOMAIN_VERSION, "eravon_literals::literals::DOMAIN_VERSION", Role::TypedDataDomain, S_DOMAIN),
    e(lit::PREFIX_LINK_SCHEME, "eravon_literals::literals::PREFIX_LINK_SCHEME", Role::TransportPrefix, S_LINK_TRANSPORT),
    e(lit::PREFIX_RECEIPT, "eravon_literals::literals::PREFIX_RECEIPT", Role::TransportPrefix, S_RECEIPT_TRANSPORT),
    e(lit::TRANSACTION_KINDS[0], "eravon_literals::literals::TRANSACTION_KINDS[0]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[1], "eravon_literals::literals::TRANSACTION_KINDS[1]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[2], "eravon_literals::literals::TRANSACTION_KINDS[2]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[3], "eravon_literals::literals::TRANSACTION_KINDS[3]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[4], "eravon_literals::literals::TRANSACTION_KINDS[4]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[5], "eravon_literals::literals::TRANSACTION_KINDS[5]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[6], "eravon_literals::literals::TRANSACTION_KINDS[6]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[7], "eravon_literals::literals::TRANSACTION_KINDS[7]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[8], "eravon_literals::literals::TRANSACTION_KINDS[8]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[9], "eravon_literals::literals::TRANSACTION_KINDS[9]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[10], "eravon_literals::literals::TRANSACTION_KINDS[10]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[11], "eravon_literals::literals::TRANSACTION_KINDS[11]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[12], "eravon_literals::literals::TRANSACTION_KINDS[12]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[13], "eravon_literals::literals::TRANSACTION_KINDS[13]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[14], "eravon_literals::literals::TRANSACTION_KINDS[14]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[15], "eravon_literals::literals::TRANSACTION_KINDS[15]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[16], "eravon_literals::literals::TRANSACTION_KINDS[16]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[17], "eravon_literals::literals::TRANSACTION_KINDS[17]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[18], "eravon_literals::literals::TRANSACTION_KINDS[18]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[19], "eravon_literals::literals::TRANSACTION_KINDS[19]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[20], "eravon_literals::literals::TRANSACTION_KINDS[20]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[21], "eravon_literals::literals::TRANSACTION_KINDS[21]", Role::TransactionKind, S_TX_KIND),
    e(lit::TRANSACTION_KINDS[22], "eravon_literals::literals::TRANSACTION_KINDS[22]", Role::TransactionKind, S_TX_KIND),
    e(lit::TERMINAL_NAMES[0], "eravon_literals::literals::TERMINAL_NAMES[0]", Role::TerminalName, S_TERMINAL),
    e(lit::TERMINAL_NAMES[1], "eravon_literals::literals::TERMINAL_NAMES[1]", Role::TerminalName, S_TERMINAL),
    e(lit::TERMINAL_NAMES[2], "eravon_literals::literals::TERMINAL_NAMES[2]", Role::TerminalName, S_TERMINAL),
    e(lit::TERMINAL_NAMES[3], "eravon_literals::literals::TERMINAL_NAMES[3]", Role::TerminalName, S_TERMINAL),
    e(lit::TERMINAL_NAMES[4], "eravon_literals::literals::TERMINAL_NAMES[4]", Role::TerminalName, S_TERMINAL),
    e(lit::TERMINAL_NAMES[5], "eravon_literals::literals::TERMINAL_NAMES[5]", Role::TerminalName, S_TERMINAL),
    e(lit::CASE_STATES[0], "eravon_literals::literals::CASE_STATES[0]", Role::CaseState, S_CASE_STATE),
    e(lit::CASE_STATES[1], "eravon_literals::literals::CASE_STATES[1]", Role::CaseState, S_CASE_STATE),
    e(lit::CASE_STATES[2], "eravon_literals::literals::CASE_STATES[2]", Role::CaseState, S_CASE_STATE),
    e(lit::CASE_STATES[3], "eravon_literals::literals::CASE_STATES[3]", Role::CaseState, S_CASE_STATE),
    e(lit::DOSSIER_INDEX_FILE, "eravon_literals::literals::DOSSIER_INDEX_FILE", Role::FileName, S_INDEX_FILE),
    e(lit::TERMS_FENCE_OPEN, "eravon_literals::literals::TERMS_FENCE_OPEN", Role::FenceLine, S_FENCE),
    e(lit::TERMS_FENCE_CLOSE, "eravon_literals::literals::TERMS_FENCE_CLOSE", Role::FenceLine, S_FENCE),
    e(lit::EIP712_DIGEST_PREFIX, "eravon_literals::literals::EIP712_DIGEST_PREFIX", Role::TypedDataPrefix, S_EIP712_PREFIX),
    e(lit::PROXY_INITCODE_PREFIX, "eravon_literals::literals::PROXY_INITCODE_PREFIX", Role::ProxyCodeFragment, S_PROXY_CODE),
    e(lit::PROXY_INITCODE_SUFFIX, "eravon_literals::literals::PROXY_INITCODE_SUFFIX", Role::ProxyCodeFragment, S_PROXY_CODE),
    e(lit::QUOTE_TYPE, "eravon_literals::literals::QUOTE_TYPE", Role::AbiTuple, S_QUOTE_TYPE),
    e(lit::SALT_LOGIC, "eravon_literals::literals::SALT_LOGIC", Role::DeploySalt, S_SALT_LOGIC),
    e(lit::SALT_PANEL_BOOK, "eravon_literals::literals::SALT_PANEL_BOOK", Role::DeploySalt, S_SALT_PANEL),
    e(lit::ZIKARON_KIT_PREFIX, "eravon_literals::literals::ZIKARON_KIT_PREFIX", Role::ZikaronSeam, S_ZIKARON),
    e(lit::ZIKARON_POINTER_FILE, "eravon_literals::literals::ZIKARON_POINTER_FILE", Role::ZikaronSeam, S_ZIKARON),
    e(lit::ZIKARON_DEFAULT_DIR, "eravon_literals::literals::ZIKARON_DEFAULT_DIR", Role::ZikaronSeam, S_ZIKARON),
    e(lit::ZIKARON_LEGACY_DIR, "eravon_literals::literals::ZIKARON_LEGACY_DIR", Role::ZikaronSeam, S_ZIKARON),
    e(lit::ZIKARON_INDEX_PATH, "eravon_literals::literals::ZIKARON_INDEX_PATH", Role::ZikaronSeam, S_ZIKARON),
    e(lit::ZIKARON_INDEX_FORM, "eravon_literals::literals::ZIKARON_INDEX_FORM", Role::ZikaronSeam, S_ZIKARON),
    e("zikaron-kit-file/1\n", "eravon_glue::container::MAGIC", Role::ZikaronSeam, S_ZIKARON),
    e(lit::SALT_VERIFIER, "eravon_literals::literals::SALT_VERIFIER", Role::DeploySalt, S_SALT_VERIFIER),
    e(lit::SALT_BOARD, "eravon_literals::literals::SALT_BOARD", Role::DeploySalt, S_SALT_BOARD),
    e_at(lit::URL_SCHEME, "eravon_literals::literals::URL_SCHEME", &["eravon_os::cli_path::NAME", "eravon_vault::backup::EXT", "eravon_cli::verbs_own::PROGRAM"], Role::ProductName, S_LOWER_NAME),
    e(lit::BUNDLE_ID, "eravon_literals::literals::BUNDLE_ID", Role::PlatformIdentifier, S_PLATFORM),
    e_at(lit::LOCAL_KEY_INFO, "eravon_literals::literals::LOCAL_KEY_INFO", &["eravon_seal::keybox::LOCAL_INFO"], Role::LocalData, S_LOCAL),
    e(lit::BACKUP_SUFFIX, "eravon_literals::literals::BACKUP_SUFFIX", Role::LocalData, S_LOCAL),
    e_at(lit::BACKUP_MAGIC, "eravon_literals::literals::BACKUP_MAGIC", &["eravon_vault::backup::MAGIC"], Role::LocalData, S_LOCAL),
    e(lit::LOCAL_DATA_KINDS[0], "eravon_literals::literals::LOCAL_DATA_KINDS[0]", Role::LocalDataKind, S_LOCAL),
    e(lit::LOCAL_DATA_KINDS[1], "eravon_literals::literals::LOCAL_DATA_KINDS[1]", Role::LocalDataKind, S_LOCAL),
    e(lit::LOCAL_DATA_KINDS[2], "eravon_literals::literals::LOCAL_DATA_KINDS[2]", Role::LocalDataKind, S_LOCAL),
    e(lit::LOCAL_DATA_KINDS[3], "eravon_literals::literals::LOCAL_DATA_KINDS[3]", Role::LocalDataKind, S_LOCAL),
    e(lit::LOCAL_DATA_KINDS[4], "eravon_literals::literals::LOCAL_DATA_KINDS[4]", Role::LocalDataKind, S_LOCAL),
    e(lit::LOCAL_DATA_KINDS[5], "eravon_literals::literals::LOCAL_DATA_KINDS[5]", Role::LocalDataKind, S_LOCAL),
    e(lit::LOCAL_DATA_KINDS[6], "eravon_literals::literals::LOCAL_DATA_KINDS[6]", Role::LocalDataKind, S_LOCAL),
    e(lit::LOCAL_DATA_KINDS[7], "eravon_literals::literals::LOCAL_DATA_KINDS[7]", Role::LocalDataKind, S_LOCAL),
    e(lit::LOCAL_DATA_KINDS[8], "eravon_literals::literals::LOCAL_DATA_KINDS[8]", Role::LocalDataKind, S_LOCAL),
    e(lit::LOCAL_DATA_KINDS[9], "eravon_literals::literals::LOCAL_DATA_KINDS[9]", Role::LocalDataKind, S_LOCAL),
    e(lit::LOCAL_DATA_KINDS[10], "eravon_literals::literals::LOCAL_DATA_KINDS[10]", Role::LocalDataKind, S_LOCAL),
    e(lit::LOCAL_DATA_KINDS[11], "eravon_literals::literals::LOCAL_DATA_KINDS[11]", Role::LocalDataKind, S_LOCAL),
    e(lit::LOCAL_DATA_KINDS[12], "eravon_literals::literals::LOCAL_DATA_KINDS[12]", Role::LocalDataKind, S_LOCAL),
    e(lit::LOCAL_DATA_KINDS[13], "eravon_literals::literals::LOCAL_DATA_KINDS[13]", Role::LocalDataKind, S_LOCAL),
    e(lit::LOCAL_DATA_KINDS[14], "eravon_literals::literals::LOCAL_DATA_KINDS[14]", Role::LocalDataKind, S_LOCAL),
    e("eravon-local/1\n", "eravon_glue::sealed::MAGIC", Role::LocalData, S_LOCAL),
    e("eravon-local/2\n", "eravon_glue::sealed::MAGIC_V2", Role::LocalData, S_LOCAL),
    e("eravon-local/gone", "eravon_vault::local::GONE", Role::LocalData, S_LOCAL),
    e("eravon-local/owner", "eravon_seal::envelope::OWNER_DOMAIN", Role::LocalData, S_LOCAL),
    e("eravon-local/place", "eravon_seal::envelope::PLACE_DOMAIN", Role::LocalData, S_LOCAL),
    e("eravon-local/place-key", "eravon_seal::envelope::PLACE_KEY_INFO", Role::LocalData, S_LOCAL),
    e("eravon/cli-credential/2", "eravon_seal::credential::DOMAIN", Role::LocalData, S_LOCAL),
    e("eravon cli-sent/1", "eravon_send::sent::SEAL_INFO", Role::LocalData, S_LOCAL),
    e("eravon.ended-cases/1", "eravon_orders::ended::FORM", Role::LocalData, S_LOCAL),
    e("eravon.block-times/1", "eravon_scan::times::FORM", Role::LocalData, S_LOCAL),
    e("eravon.standard-emitters/1", "eravon_scan::codes::FORM", Role::LocalData, S_LOCAL),
    e(lit::VAULT_FORMAT, "eravon_literals::literals::VAULT_FORMAT", Role::LocalData, S_NAMES),
    e(lit::LOCAL_BIND_INFO, "eravon_literals::literals::LOCAL_BIND_INFO", Role::LocalData, S_NAMES),
    e(lit::NAME_KINDS[0], "eravon_literals::literals::NAME_KINDS[0]", Role::NameKind, S_NAMES),
    e(lit::NAME_KINDS[1], "eravon_literals::literals::NAME_KINDS[1]", Role::NameKind, S_NAMES),
    e(lit::NAME_KINDS[2], "eravon_literals::literals::NAME_KINDS[2]", Role::NameKind, S_NAMES),
    e(lit::NAME_KINDS[3], "eravon_literals::literals::NAME_KINDS[3]", Role::NameKind, S_NAMES),
    e(lit::NAME_KINDS[4], "eravon_literals::literals::NAME_KINDS[4]", Role::NameKind, S_NAMES),
    e(lit::NAME_KINDS[5], "eravon_literals::literals::NAME_KINDS[5]", Role::NameKind, S_NAMES),
    e(lit::COMPANY_VIEWS[0], "eravon_literals::literals::COMPANY_VIEWS[0]", Role::CompanyView, S_COMPANY_VIEW),
    e(lit::COMPANY_VIEWS[1], "eravon_literals::literals::COMPANY_VIEWS[1]", Role::CompanyView, S_COMPANY_VIEW),
    e(lit::COMPANY_VIEWS[2], "eravon_literals::literals::COMPANY_VIEWS[2]", Role::CompanyView, S_COMPANY_VIEW),
    e(lit::COMPANY_VIEWS[3], "eravon_literals::literals::COMPANY_VIEWS[3]", Role::CompanyView, S_COMPANY_VIEW),
    e_at("ERAVON", "eravon_os::house::APP_FOLDER", &["eravon_os::house::DATA_FOLDER", "eravon_vault::home::HOME_STEM", "app::app_name::APP_NAME"], Role::ProductName, S_PRODUCT),
    e("eravon-entry-key", "eravon_ui::nav::ENTRY_KEY", Role::ProductName, S_PRODUCT),
    e("eravon-mdedit", "eravon_ui::mdedit::ID_HEAD", Role::ProductName, S_PRODUCT),
    e(".eravon-desk.d", "eravon_os::machine::APP_DIR", Role::ProductName, S_PRODUCT),
    e(".eravon-desk", "eravon_os::machine::POINTER", Role::ProductName, S_PRODUCT),
    e("eravon-desk", "eravon_vault::backup::APP", Role::ProductName, S_PRODUCT),
    e("eravon-key", "eravon_vault::seam::SLOT_BASE", Role::ProductName, S_NAMES),
    e("eravon-key-roster", "eravon_vault::seam::SLOT_ROSTER", Role::ProductName, S_NAMES),
    e("eravon.vault.roster/1", "eravon_vault::seam::V_ROSTER", Role::DocumentForm, S_FORMS),
    e("eravon.goods-refs/1", "eravon_goods::refs::FENCE_INFO", Role::DocumentForm, S_FORMS),
    e("eravon-desk/keybox/3", "eravon_seal::keybox::SHAPE", Role::DocumentForm, S_FORMS),
    e("eravon-desk/keybox/2", "eravon_seal::keybox::SHAPE_V2", Role::DocumentForm, S_FORMS),
    e("eravon-desk/keybox/1", "eravon_seal::keybox::SHAPE_V1", Role::DocumentForm, S_FORMS),
    e("eravon-keybox/2", "eravon_seal::keybox::BIND_DOMAIN", Role::LocalData, S_NAMES),
    e("eravon/keybox/member/v1", "eravon_seal::keybox::MEMBER_INFO", Role::LocalData, S_NAMES),
    e("eravon/names/v1", "eravon_seal::keybox::NAMES_INFO", Role::LocalData, S_NAMES),
    e("eravon/1", "eravon::tokens::SPEC", Role::DocumentForm, S_FORMS),
    e(crate::REGISTRY_NAME, "eravon_literals::REGISTRY_NAME", Role::ProductName, S_PRODUCT),
    e("ERAVON.app", "app::house::bundle::APP_DIR", Role::ProductName, S_PRODUCT),
    e_at("eravon-app", "app::house::bundle::EXECUTABLE", &["app::house::WRITER"], Role::ProductName, S_PRODUCT),
    e("eravon-door-", "eravon_os::door::NAME_HEAD", Role::ProductName, S_PRODUCT),
    e("eravon-key-", "eravon_vault::seam::KEYFILE_PREFIX", Role::ProductName, S_PRODUCT),
    e("eravon-grid-cell-h", "eravon_ui::grid::TARGET", Role::ProductName, S_PRODUCT),
    e("eravon-probe-rows", "eravon_ui::probe::ROWS", Role::ProductName, S_PRODUCT),
    e("eravon-probe-spots", "eravon_ui::probe::SPOTS", Role::ProductName, S_PRODUCT),
    e("eravon-probe-inner", "eravon_ui::probe::INNER", Role::ProductName, S_PRODUCT),
    e("eravon-probe-inner-head", "eravon_ui::probe::HEAD", Role::ProductName, S_PRODUCT),
    e("eravon-probe-head-keys", "eravon_ui::probe::HEAD_KEYS", Role::ProductName, S_PRODUCT),
    e("eravon-probe-cells", "eravon_ui::probe::CELLS", Role::ProductName, S_PRODUCT),
    e("eravon-probe-tiles", "eravon_ui::probe::TILES", Role::ProductName, S_PRODUCT),
    e("eravon-probe-route", "eravon_ui::probe::ROUTE", Role::ProductName, S_PRODUCT),
    e("eravon-probe-history", "eravon_ui::probe::HISTORY", Role::ProductName, S_PRODUCT),
    e("eravon-probe-qr", "eravon_ui::probe::QR", Role::ProductName, S_PRODUCT),
    e("eravon-probe-parts", "eravon_ui::probe::PARTS", Role::ProductName, S_PRODUCT),
    e("eravon-probe-boxes", "eravon_ui::probe::BOXES", Role::ProductName, S_PRODUCT),
    e("eravon-probe-lits", "eravon_ui::probe::LITS", Role::ProductName, S_PRODUCT),
    e("eravon-probe-sheets", "eravon_ui::probe::SHEETS", Role::ProductName, S_PRODUCT),
    e("eravon-probe-grey-keys", "eravon_ui::probe::GREY_KEYS", Role::ProductName, S_PRODUCT),
    e("eravon-probe-cut-short", "eravon_ui::probe::CUT_SHORT", Role::ProductName, S_PRODUCT),
    e("eravon-probe-whole-on-hover", "eravon_ui::probe::WHOLE_ON_HOVER", Role::ProductName, S_PRODUCT),
    e("eravon-probe-tables", "eravon_ui::probe::TABLES", Role::ProductName, S_PRODUCT),
    e("eravon-menu-card", "eravon_ui::menu::CARD", Role::ProductName, S_PRODUCT),
    e("eravon-page-bar-floor", "eravon_ui::formparts::FLOOR", Role::ProductName, S_PRODUCT),
    e("eravon-backup-", "eravon_vault::seam::BACKUP_STEM", Role::ProductName, S_PRODUCT),
    e("eravon-logic.abi.json", "eravon_standard::proposal::LOGIC_ABI_NAME", Role::ProductName, S_PRODUCT),
    e("eravon-instance.abi.json", "eravon_standard::proposal::INSTANCE_ABI_NAME", Role::ProductName, S_PRODUCT),
    e("eravon-board.abi.json", "eravon_standard::proposal::BOARD_ABI_NAME", Role::ProductName, S_PRODUCT),
    e("ERAVON", "eravon_net::USER_AGENT", Role::RequestProductName, S_PRODUCT),
    e("eravon-unpack-", "eravon_glue::read::UNPACK_HEAD", Role::ProductName, S_PRODUCT),
    e("eravon-", "eravon_os::house::THREAD_HEAD", Role::ProductName, S_PRODUCT),
    e("eravon-window", "eravon_os::unix::TOOLBAR_ID", Role::ProductName, S_PRODUCT),
    e_at(".lock", "eravon_store::seam::F_LOCK", &["eravon_seal::envelope::WRITER_LOCK"], Role::FileName, S_LAYOUT),
    e_at(".writer-mark", "eravon_store::seam::F_WRITER_MARK", &["eravon_seal::writer::MARK_FILE"], Role::FileName, S_LAYOUT),
    e_at("eravon-machine-mark", "eravon_store::seam::F_MACHINE_MARK", &["eravon_seal::writer::MACHINE_FILE"], Role::FileName, S_LAYOUT),
    e(".ev-sealed", "eravon_seal::places::MACHINE_EXT", Role::FileName, S_LAYOUT),
    e(".evs-tmp-", "eravon_glue::layout::TMP_PREFIX", Role::FileName, S_LAYOUT),
    e("sent-direct", "eravon_send::sent::DIR", Role::FileName, S_LAYOUT),
    e("eravon-delivery-manifest.json", "eravon_deliver::deliver::MANIFEST_PART", Role::FileName, S_LAYOUT),
    e(".mirror", "eravon_store::seam::F_MIRROR", Role::FileName, S_LAYOUT),
    e("eravon.store.settings/3", "eravon_store::seam::V_SETTINGS", Role::DocumentForm, S_FORMS),
    e("eravon.store.identity-settings/2", "eravon_store::seam::V_IDENTITY_SETTINGS", Role::DocumentForm, S_FORMS),
    e("eravon-desk/machine/1", "eravon_vault::machine::SHAPE", Role::DocumentForm, S_FORMS),
    e("eravon-desk/identities/2", "eravon_seal::identities::SHAPE", Role::DocumentForm, S_FORMS),
    e("eravon-desk/identities/1", "eravon_seal::identities::SHAPE_V1", Role::DocumentForm, S_FORMS),
    e("eravon-desk/backup-index/1", "eravon_vault::backup::INDEX_FORM", Role::DocumentForm, S_FORMS),
    e("eravon.store.candidates/1", "eravon_store::seam::V_CANDIDATES", Role::DocumentForm, S_FORMS),
    e("eravon.store.reading/3", "eravon_store::seam::V_READING", Role::DocumentForm, S_FORMS),
    e("eravon-door/1", "eravon_cli::door::FORM", Role::DocumentForm, S_FORMS),
    e("eravon.store.record/1", "eravon_store::seam::V_RECORD", Role::DocumentForm, S_FORMS),
    e("eravon.store.archive/1", "eravon_store::seam::V_ARCHIVE", Role::DocumentForm, S_FORMS),
    e("eravon.store.lock/1", "eravon_store::seam::V_LOCK", Role::DocumentForm, S_FORMS),
    e("eravon.store.backup-pack/1", "eravon_store::seam::V_BACKUP_PACK", Role::DocumentForm, S_FORMS),
    e("eravon.store.mirror/1", "eravon_store::seam::V_MIRROR", Role::DocumentForm, S_FORMS),
    e(crate::trace::V_TRACE_DOC, "eravon_literals::trace::V_TRACE_DOC", Role::DocumentForm, S_FORMS),
    e("eravon.app.launch/1", "app::house::bundle::V_LAUNCH", Role::DocumentForm, S_FORMS),
    e("eravon.chain.endpoints/1", "eravon_chain::table::V_TABLE", Role::DocumentForm, S_FORMS),
    e("eravon.goods.draft/1", "eravon_goods::draft::V_DRAFT", Role::DocumentForm, S_FORMS),
    e("eravon.statement/1", "eravon_statement::V_STATEMENT", Role::DocumentForm, S_FORMS),
    e("eravon.compliance/1", "eravon_statement::V_COMPLIANCE", Role::DocumentForm, S_FORMS),
    e("eravon.vault/2", "eravon_vault::seam::V2_FORM", Role::DocumentForm, S_FORMS),
    e("eravon.settled-logs/2", "eravon_chain::settled::FORM", Role::DocumentForm, S_FORMS),
    e("eravon.judged-anchors/1", "eravon_scan::judged::FORM", Role::DocumentForm, S_FORMS),
    e(crate::ANSWER_VERSION, "eravon_literals::ANSWER_VERSION", Role::DocumentForm, S_FORMS),
    e_at(crate::trace::ENV_TRACE, "eravon_literals::trace::ENV_TRACE", &["eravon_glue::trace::TRACE_ENV", "eravon_vault::trace::SINK_ENV"], Role::Environment, S_ENV),
    e(crate::trace::CAP_ENV, "eravon_literals::trace::CAP_ENV", Role::Environment, S_ENV),
    e("ERAVON_CLI_PASS", "eravon_seal::credential::PASS_ENV", Role::Environment, S_ENV),
    e("ERAVON_TIMEOUT_SECS", "eravon_net::env::TIMEOUT_SECS", Role::Environment, S_ENV),
    e("ERAVON_TIMEOUT_MS", "eravon_net::env::TIMEOUT_MS", Role::Environment, S_ENV),
    e("ERAVON_MAX_ANSWER_BYTES", "eravon_net::env::MAX_ANSWER_BYTES", Role::Environment, S_ENV),
    e("ERAVON_PATIENCE_MS", "eravon_chain::ask::ENV_PAUSE_MS", Role::Environment, S_ENV),
    e("ERAVON_NET_ALONE", "eravon_net::keep::CHILD", Role::Environment, S_ENV),
    e("ERAVON_LAUNCH", "app::house::seam::ENV_LAUNCH", Role::Environment, S_ENV),
    e("ERAVON_DESK_HOME", "eravon_vault::home::HOME_ENV", Role::Environment, S_ENV),
    e("ERAVON_NOTIFY_FILE", "app::notifier::NOTIFY_FILE_ENV", Role::Environment, S_ENV),
    e("notify.log", "app::notifier::NOTIFY_FILE", Role::FileName, S_LOCAL),
    e("real-notify", "app::notifier::FEATURE", Role::ProductName, S_PRODUCT),
    e("ERAVON_TOAST_TITLE", "app::notifier::program::TOAST_TITLE", Role::Environment, S_ENV),
    e("ERAVON_TOAST_BODY", "app::notifier::program::TOAST_BODY", Role::Environment, S_ENV),
    e("ERAVON_CLI_LINK_DIR", "eravon_os::cli_path::ENV_DIR", Role::Environment, S_ENV),
    e("ERAVON_REST_KEYS", "eravon_vault::chainx::REST_KEYS_ENV", Role::Environment, S_ENV),
];

pub const PAIRS: &[(&str, &[Role])] = &[
    ("ERAVON", &[Role::TypedDataDomain, Role::ProductName, Role::RequestProductName]),
    ("eravon/1", &[Role::DeploySalt, Role::DocumentForm]),
];

#[derive(Clone, Copy, Debug)]
pub struct DerivationPath {
    pub path: &'static str,
    pub role: &'static str,
    pub sources: &'static [&'static str],
}

const S_DERIVE: &[&str] = &["BIP-44", "chosen by this product: project number 2' and two keys"];

pub const DERIVATION_PATHS: &[DerivationPath] = &[
    DerivationPath { path: "m/44'/60'/2'/0/0", role: "seller-reputation", sources: S_DERIVE },
    DerivationPath { path: "m/44'/60'/2'/1/0", role: "buyer", sources: S_DERIVE },
];

pub fn self_check() -> Result<(), &'static str> {
    if ENTRIES.is_empty() {
        return Err("E_REGISTRY_EMPTY");
    }
    for (i, a) in ENTRIES.iter().enumerate() {
        if a.sources.is_empty() || a.at.is_empty() {
            return Err("E_REGISTRY_NO_SOURCE");
        }
        for b in ENTRIES.iter().skip(i + 1) {
            if a.literal == b.literal {
                if a.role == b.role {
                    return Err("E_REGISTRY_DUPLICATE");
                }
                let paired = PAIRS.iter().any(|(l, roles)| *l == a.literal && roles.contains(&a.role) && roles.contains(&b.role));
                if !paired {
                    return Err("E_REGISTRY_CROSS_ROLE");
                }
            }
        }
    }
    Ok(())
}

pub fn borrowed(e: &Entry) -> bool {
    e.role == Role::ZikaronSeam
}

pub fn literals() -> Vec<&'static str> {
    ENTRIES.iter().filter(|x| !borrowed(x)).map(|x| x.literal).collect()
}

pub fn family_json() -> Vec<u8> {
    use crate::json::Out;
    const ROLES: [&str; 11] = [
        "name", "request-product-name", "file-magic", "document-form", "spec-value", "signing-domain", "payload-prefix", "recording-mark",
        "derivation-purpose", "derivation-path", "environment",
    ];
    let says = |e: &Entry| format!("[{}] {}", e.role.name(), e.sources.join("; "));
    let row = |e: &Entry, role: &str| Out::O(vec![("at", Out::S(e.places().join(", "))), ("literal", Out::S(e.literal.to_string())), ("role", Out::S(role.to_string())), ("says", Out::S(says(e)))]);
    let mut entries: Vec<Out> = ENTRIES.iter().filter(|e| !borrowed(e)).map(|e| row(e, e.role.family(e.literal))).collect();
    for d in DERIVATION_PATHS {
        entries.push(Out::O(vec![
            ("at", Out::S(format!("eravon_literals::registry::DERIVATION_PATHS[{}]", d.role))),
            ("literal", Out::S(d.path.to_string())),
            ("role", Out::S("derivation-path".to_string())),
            ("says", Out::S(format!("[{}] {}", d.role, d.sources.join("; ")))),
        ]));
    }
    let foreign: Vec<&Entry> = ENTRIES.iter().filter(|e| borrowed(e)).collect();
    let pairs = PAIRS.iter().map(|(l, roles)| Out::O(vec![("literal", Out::S(l.to_string())), ("roles", Out::A(roles.iter().map(|r| Out::S(r.family(l).to_string())).collect()))])).collect();
    let mut b = crate::json::write(&Out::O(vec![
        ("schema", Out::S("eravon-family-literals/1".to_string())),
        ("family", Out::S(crate::FAMILY.to_string())),
        ("says", Out::S("ERAVON's family literal register, generated from crates/eravon-literals/src/registry.rs (the one place it lives); entries are this family's own, foreign.zikaron are ZIKARON's literals ERAVON reads and never claims.".to_string())),
        ("roles", Out::A(ROLES.iter().map(|r| Out::S(r.to_string())).collect())),
        ("entries", Out::A(entries)),
        ("pairs", Out::A(pairs)),
        (
            "foreign",
            Out::O(vec![(
                "zikaron",
                Out::O(vec![
                    ("entries", Out::A(foreign.iter().map(|e| row(e, e.role.family(e.literal))).collect())),
                    ("literals", Out::A(foreign.iter().map(|e| Out::S(e.literal.to_string())).collect())),
                ]),
            )]),
        ),
    ]));
    b.push(b'\n');
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_passes_its_own_three_questions() {
        assert_eq!(self_check(), Ok(()));
    }

    #[test]
    fn every_role_of_the_closed_set_is_used() {
        for r in Role::ALL {
            assert!(
                ENTRIES.iter().any(|x| x.role == r),
                "角色闭集里 {} 一条字面也没有:闭集与登记簿对不上",
                r.name()
            );
        }
    }

    #[test]
    fn counts_match_the_claim() {
        let n = |r: Role| ENTRIES.iter().filter(|x| x.role == r).count();
        assert_eq!(n(Role::DocumentVersion), 9, "七文书字面、公司提案文件字面与投诉档字面");
        assert_eq!(n(Role::TypedDataType), 10, "十类型串");
        assert_eq!(n(Role::TransportPrefix), 2, "运输两前缀");
        assert_eq!(n(Role::TransactionKind), 23, "卷宗二十三个交易 kind");
        assert_eq!(n(Role::TerminalName), 6, "六终端名");
        assert_eq!(n(Role::ZikaronSeam), 7, "ZIKARON 接点七条字面(借读,进 foreign 节):六条发现与索引,加单档记录包的首行");
        assert_eq!(n(Role::DeploySalt), 4, "四段部署盐原像(逻辑、名单簿、验证器、公告板)");
        assert_eq!(n(Role::PlatformIdentifier), 1, "本机注册一条(URL 方案与命令行名同一字面,登在 name 那一条、两处 at)");
        assert_eq!(n(Role::LocalData), 19, "本机数据钥 info、备份魔数与后缀、封存魔数(两版)、封存层的删除标与主人域、位置域与位置摘要钥 info、密钥库格式字面第 2 版、绑定钥 info、金库的绑定域与成员和名两枚 info、命令行凭据域、直发记录封钥 info、已结单记录的形、块时刻记录与标准发射者记录的形");
        assert_eq!(n(Role::LocalDataKind), lit::LOCAL_DATA_KINDS.len(), "档种类名表逐条登记");
        assert_eq!(n(Role::NameKind), lit::NAME_KINDS.len(), "出名函数的种类标签表逐条登记");
        assert_eq!(n(Role::CompanyView), lit::COMPANY_VIEWS.len(), "公司视图逐条登记(三视图与 holds)");
    }

    #[test]
    fn backup_suffix_is_not_chelek_s() {
        assert_eq!(lit::BACKUP_SUFFIX, ".eravon");
        assert_ne!(lit::BACKUP_SUFFIX, ".chelek");
        assert!(ENTRIES.iter().all(|x| x.literal != ".chelek"));
    }

    #[test]
    fn fence_lines_have_the_byte_lengths_the_law_states() {
        assert_eq!(lit::TERMS_FENCE_OPEN.len(), 17);
        assert_eq!(lit::TERMS_FENCE_CLOSE.len(), 3);
    }
}
