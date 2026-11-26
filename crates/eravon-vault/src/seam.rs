pub const RULE_SHAPE: &str = "shape";
pub const RULE_MATCH: &str = "match";
pub const RULE_SAME: &str = "all-same";
pub const RULE_RUN: &str = "run";
pub const RULE_DATE: &str = "date-like";
pub const RULE_FILE_LEN: &str = "too-short";

pub const SAY_LOCAL_RETIRED: &str = "本机数据随之撤掉:它们由旧口令那一枚主钥封着,新库解不开";

pub const RULES: [&str; 5] = [RULE_SHAPE, RULE_SAME, RULE_RUN, RULE_DATE, RULE_MATCH];

pub const KDF_ALG: &str = "scrypt";

pub const KDF_N: &str = "n";
pub const KDF_R: &str = "r";
pub const KDF_P: &str = "p";

pub const AEAD_ALG: &str = "xchacha20poly1305";
pub const KDF_AEAD: &str = "aead";

pub const HKDF_RECOVERY: &str = "recovery";

pub const V_INIT: &str = "vault-init";
pub const V_IMPORT: &str = "vault-import";
pub const V_DERIVE: &str = "vault-derive";
pub const V_UNLOCK: &str = "vault-unlock";
pub const V_LOCK: &str = "vault-lock";
pub const V_PASSCODE: &str = "vault-passcode";
pub const V_RECOVER: &str = "vault-recover";
pub const V_STATUS: &str = "vault-status";

pub const GATE_OPEN: &str = "open";
pub const GATE_LOCKED: &str = "locked";
pub const GATE_DEAD: &str = "dead";

pub const WORDS: usize = 12;

pub const KEYFILE_PREFIX: &str = "eravon-key-";
pub const KEYFILE_EXT: &str = "json";

pub fn keyfile_name(address: &str) -> String {
    format!("{KEYFILE_PREFIX}{}.{KEYFILE_EXT}", address.trim_start_matches("0x").to_ascii_lowercase())
}

pub const V_BACKUP: &str = "vault-backup";
pub const V_RESTORE: &str = "vault-restore";
pub const V_REBUILD: &str = "vault-rebuild";
pub const V_NEW_KEY: &str = "vault-new-key";
pub const V_SWITCH: &str = "vault-switch";
pub const V_REMOVE: &str = "vault-remove";
pub const V_RESET: &str = "vault-reset";
pub const V_PHRASE: &str = "vault-phrase";
pub const V_MIGRATE: &str = "vault-migrate";
pub const V_PAYOUT: &str = "payout-set";

pub const V2_FORM: &str = "eravon.vault/2";

pub const BACKUP_STEM: &str = "eravon-backup-";
pub const BACKUP_PW_MIN: usize = 8;
pub const BACKUP_PW_GOOD: usize = 12;

pub const SLOT_BASE: &str = "eravon-key";
pub const SLOT_ROSTER: &str = "eravon-key-roster";
pub const V_ROSTER: &str = "eravon.vault.roster/1";

pub fn slot_of(address: &str) -> String {
    format!("{SLOT_BASE}-{}", address.trim().trim_start_matches("0x").to_ascii_lowercase())
}
