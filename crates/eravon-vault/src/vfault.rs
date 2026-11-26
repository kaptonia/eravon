#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    Rule(&'static str),
    WrongPasscode {
        wrong: u32,
        max: u32,
    },
    Dead {
        wrong: u32,
    },
    Locked,
    Mnemonic(String),
    Sealed,
    Vault(String),
    KdfBound(String),
    Path(String),
    Store(String),
    Down { token: &'static str, detail: String },
    KeyFileBroken(String),
    Exists(String),
    NoIdentity(String),
    Migrate(String),
    Address(String),
    NotBackup(String),
    BackupPassword,
    BackupVersion(String),
    KeystoreParams(String),
    HeadUnread(String),
    NameKeySealed(String),
    PasswordLong(String),
    BackupNotLanded(String),
    KdfFloor(String),
    BackupShape(String),
    SellerKey(String),
    PartyKey(String),
}

impl Fault {
    pub fn name(&self) -> &'static str {
        match self {
            Fault::Rule(_) => "E_VAULT_RULE",
            Fault::WrongPasscode { .. } => "E_VAULT_PASSCODE",
            Fault::Dead { .. } => "E_VAULT_DEAD",
            Fault::Locked => "E_VAULT_LOCKED",
            Fault::Mnemonic(_) => "E_VAULT_MNEMONIC",
            Fault::Sealed => "E_VAULT_SEALED",
            Fault::Vault(_) => "E_VAULT_FORM",
            Fault::KdfBound(_) => "E_VAULT_KDF_BOUND",
            Fault::Path(_) => "E_VAULT_PATH",
            Fault::Store(_) => "E_VAULT_IO",
            Fault::Down { token, .. } => token,
            Fault::KeyFileBroken(_) => "E_VAULT_KEYFILE_BROKEN",
            Fault::Exists(_) => "E_VAULT_EXISTS",
            Fault::NoIdentity(_) => "E_VAULT_NO_IDENTITY",
            Fault::Migrate(_) => "E_VAULT_MIGRATE",
            Fault::Address(_) => "E_VAULT_ADDRESS",
            Fault::NotBackup(_) => "E_VAULT_NOT_BACKUP",
            Fault::BackupPassword => "E_VAULT_BACKUP_PASSWORD",
            Fault::BackupVersion(_) => "E_VAULT_BACKUP_VERSION",
            Fault::KeystoreParams(_) => "E_VAULT_KEYSTORE_PARAMS",
            Fault::HeadUnread(_) => "E_VAULT_HEAD_UNREAD",
            Fault::NameKeySealed(_) => "E_VAULT_NAME_KEY",
            Fault::PasswordLong(_) => "E_VAULT_PASSWORD_LONG",
            Fault::BackupNotLanded(_) => "E_VAULT_BACKUP_NOT_LANDED",
            Fault::KdfFloor(_) => "E_VAULT_KDF_FLOOR",
            Fault::BackupShape(_) => "E_VAULT_BACKUP_SHAPE",
            Fault::SellerKey(_) => "E_VAULT_SELLER_KEY",
            Fault::PartyKey(_) => "E_VAULT_PARTY_KEY",
        }
    }

    pub fn detail(&self) -> String {
        match self {
            Fault::Rule(r) => format!("{r} {}", rule_said(r)),
            Fault::WrongPasscode { wrong, max } => {
                format!("口令错误,还可再试 {} 次", max.saturating_sub(*wrong))
            }
            Fault::Dead { wrong } => {
                format!("口令连续错误 {wrong} 次。输入主钥的助记词以恢复并重设口令。")
            }
            Fault::Locked => "已锁定".to_string(),
            Fault::Mnemonic(_) => "助记词不正确".to_string(),
            Fault::Sealed => "这一段解不开".to_string(),
            Fault::Vault(s)
            | Fault::KdfBound(s)
            | Fault::Path(s)
            | Fault::Store(s)
            | Fault::KeyFileBroken(s)
            | Fault::Exists(s)
            | Fault::NoIdentity(s)
            | Fault::Migrate(s)
            | Fault::Address(s)
            | Fault::NotBackup(s)
            | Fault::BackupVersion(s) => s.clone(),
            Fault::BackupPassword => "备份密码不对".to_string(),
            Fault::KeystoreParams(s) => format!("这一份密钥文件要的派生代价超出上限(keystore-params):{s}"),
            Fault::HeadUnread(s) => format!("口令对,但密钥库档头读不动,没有解锁:{s}"),
            Fault::NameKeySealed(s) => format!("口令对,但名钥那一封解不开(密钥库档头坏了或被改过),没有解锁:{s}"),
            Fault::Down { detail, .. } => detail.clone(),
            Fault::PasswordLong(s) => format!("密码长过 {s} 字节,没有截短:请换一个短一些的"),
            Fault::BackupNotLanded(s) => format!("写下的那一份读回来不对,不算已备份:{s}"),
            Fault::KdfFloor(s) => format!("这本库记的派生参数低于出厂下界,不开也不封:{s}"),
            Fault::BackupShape(s) => format!("这一份备份的档头形不合:{s}"),
            Fault::SellerKey(s) | Fault::PartyKey(s) => s.clone(),
        }
    }

    pub fn counts_as_wrong(&self) -> bool {
        matches!(self, Fault::WrongPasscode { .. })
    }
}

fn rule_said(r: &str) -> &'static str {
    use crate::seam::*;
    match r {
        x if x == RULE_SHAPE => "口令要 8 位字母或数字(区分大小写),请换一个",
        x if x == RULE_SAME => "口令不能 8 位全同,请换一个",
        x if x == RULE_RUN => "口令不能是连号,请换一个",
        x if x == RULE_DATE => "口令不能像日期,请换一个",
        x if x == RULE_MATCH => "两次输入不一致,请重新设置",
        x if x == RULE_FILE_LEN => "文件口令或备份密码要至少八位,请换一个",
        _ => "口令不合规则,请换一个",
    }
}

impl From<eravon_store::fault::Fault> for Fault {
    fn from(f: eravon_store::fault::Fault) -> Fault {
        match f {
            eravon_store::fault::Fault::NoKey(_) => Fault::Locked,
            f => Fault::Down { token: f.name(), detail: f.detail() },
        }
    }
}

impl From<crate::fault::Fault> for Fault {
    fn from(f: crate::fault::Fault) -> Fault {
        use crate::fault::Known as F;
        let tail = f.tail().to_string();
        match f.which() {
            Some(F::AddressMismatch) => Fault::known(Known::AddressMismatch, tail),
            Some(F::BackupNotLanded) => Fault::known(Known::BackupNotLanded, tail),
            Some(F::BackupNotOurs) => Fault::known(Known::BackupNotOurs, tail),
            Some(F::BackupPassword) => Fault::known(Known::BackupPassword, tail),
            Some(F::BackupShape) => Fault::known(Known::BackupShape, tail),
            Some(F::BackupTooNew) => Fault::known(Known::BackupTooNew, tail),
            Some(F::BadPassword) => Fault::known(Known::BadPassword, tail),
            Some(F::KdfBelowFloor) => Fault::known(Known::KdfBelowFloor, tail),
            Some(F::KeyMalformed) => Fault::known(Known::KeyMalformed, tail),
            Some(F::KeyboxLocked) => Fault::known(Known::KeyboxLocked, tail),
            Some(F::KeyboxMissing) => Fault::known(Known::KeyboxMissing, tail),
            Some(F::KeyboxNotEmpty) => Fault::known(Known::KeyboxNotEmpty, tail),
            Some(F::KeyboxShape) => Fault::known(Known::KeyboxShape, tail),
            Some(F::KeyboxSlot) => Fault::known(Known::KeyboxSlot, tail),
            Some(F::KeychainMissing) => Fault::known(Known::KeychainMissing, tail),
            Some(F::KeystoreParams) => Fault::known(Known::KeystoreParams, tail),
            Some(F::Locked) => Fault::known(Known::Locked, tail),
            Some(F::LockedOut) => Fault::known(Known::LockedOut, tail),
            Some(F::PasswordLong) => Fault::known(Known::PasswordLong, tail),
            Some(F::PinSet) => Fault::known(Known::PinSet, tail),
            Some(F::PinShape) => Fault::known(Known::PinShape, tail),
            Some(F::PinWrong) => Fault::known(Known::PinWrong, tail),
            Some(F::RecoveryNoMatch) => Fault::known(Known::RecoveryNoMatch, tail),
            Some(F::RecoveryNotPrimary) => Fault::known(Known::RecoveryNotPrimary, tail),
            Some(F::NoIdentity) => Fault::NoIdentity(tail),
            Some(k) => Fault::Down { token: k.as_str(), detail: tail },
            None => Fault::Store(format!("{}: {tail}", f.said())),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Known {
    AddressMismatch,
    BackupNotLanded,
    BackupNotOurs,
    BackupPassword,
    BackupShape,
    BackupTooNew,
    BadPassword,
    KdfBelowFloor,
    KeyMalformed,
    KeyboxLocked,
    KeyboxMissing,
    KeyboxNotEmpty,
    KeyboxShape,
    KeyboxSlot,
    KeychainMissing,
    KeystoreParams,
    Locked,
    LockedOut,
    PasswordLong,
    PinSet,
    PinShape,
    PinWrong,
    RecoveryNoMatch,
    RecoveryNotPrimary,
}

impl Fault {
    pub fn family(&self) -> Option<crate::fault::Fault> {
        use crate::fault::{Fault as F, Known as K};
        match self {
            Fault::Rule(r) if *r == crate::seam::RULE_MATCH => Some(F::known(K::PasswordsDiffer, String::new())),
            Fault::Rule(r) if *r == crate::seam::RULE_FILE_LEN => Some(F::known(K::PasswordShort, String::new())),
            Fault::Rule(r) => Some(F::known(K::PinShape, *r)),
            _ => None,
        }
    }

    pub fn known(k: Known, detail: String) -> Fault {
        let max = eravon_literals::constants::PASSCODE_MAX_ATTEMPTS;
        match k {
            Known::AddressMismatch => Fault::KeyFileBroken(detail),
            Known::BackupNotLanded => Fault::BackupNotLanded(detail),
            Known::BackupNotOurs => Fault::NotBackup(if detail.is_empty() { "这不是 ERAVON 的整机备份".into() } else { detail }),
            Known::BackupPassword => Fault::BackupPassword,
            Known::BackupShape => Fault::BackupShape(detail),
            Known::BackupTooNew => Fault::BackupVersion(format!("这一份备份的格式版本是 {detail},本机只认到 {}:请先升级", crate::backup::FORMAT)),
            Known::BadPassword => Fault::Sealed,
            Known::KdfBelowFloor => Fault::KdfFloor(detail),
            Known::KeyMalformed => Fault::KeyFileBroken(detail),
            Known::KeyboxLocked => Fault::Store(format!("密钥库的锁取不到:{detail}")),
            Known::KeyboxMissing => Fault::from(eravon_store::fault::Fault::Missing(if detail.is_empty() { "密钥库".into() } else { detail })),
            Known::KeyboxNotEmpty => Fault::Vault(format!("密钥库里还有身份或钥,不能当空库撤掉:{detail}")),
            Known::KeyboxShape if detail.starts_with("kdf ") => Fault::KdfBound(format!("这本库的派生代价超出家族那一张界表({detail}):内存至多 1 GiB、p 至多 16、总功至多 4 GiB")),
            Known::KeyboxShape => Fault::Vault(format!("密钥库的形不合:{detail}")),
            Known::KeyboxSlot => Fault::Sealed,
            Known::KeychainMissing => Fault::Vault(format!("密钥库里没有这一格:{detail}")),
            Known::KeystoreParams => Fault::KeystoreParams(detail),
            Known::Locked => Fault::Locked,
            Known::LockedOut => Fault::Dead { wrong: max },
            Known::PasswordLong => Fault::PasswordLong(detail),
            Known::PinSet => Fault::Exists("这本密钥库已经设过口令了".into()),
            Known::PinShape => Fault::Rule(rule_of(&detail)),
            Known::PinWrong => match detail.parse::<u32>() {
                Ok(0) => Fault::Dead { wrong: max },
                Ok(left) => Fault::WrongPasscode { wrong: max.saturating_sub(left), max },
                Err(_) => Fault::WrongPasscode { wrong: 0, max },
            },
            Known::RecoveryNoMatch => Fault::Mnemonic("这一段助记词解不开这本密钥库的主钥".into()),
            Known::RecoveryNotPrimary => Fault::Mnemonic("这一段助记词是本机另一枚钥的,恢复只认主身份的那一段".into()),
        }
    }
}

fn rule_of(word: &str) -> &'static str {
    use crate::seam::*;
    match word {
        x if x == RULE_SAME => RULE_SAME,
        x if x == RULE_RUN => RULE_RUN,
        x if x == RULE_DATE => RULE_DATE,
        _ => RULE_SHAPE,
    }
}

pub fn classify(e: &std::io::Error, at: &str) -> Fault {
    Fault::Store(format!("{at}:{e}"))
}

pub type R<T> = Result<T, Fault>;

impl From<Fault> for crate::fault::Fault {
    fn from(e: Fault) -> crate::fault::Fault {
        let name = e.name();
        match crate::fault::Known::ALL.iter().copied().find(|k| k.as_str() == name) {
            Some(k) => crate::fault::Fault::known(k, e.detail()),
            None => crate::fault::Fault::unknown(format!("{name}: {}", e.detail())),
        }
    }
}
