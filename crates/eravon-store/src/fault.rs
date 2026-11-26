#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    Root(String),
    Name(String),
    Kind(String),
    Version { want: &'static str, got: String },
    Form(String),
    Missing(String),
    Exists(String),
    Io(String),
    Locked { holder: String },
    Archive(String),
    Mirror(String),
    Time(u64),
    Cap { what: &'static str, cap: usize },
    NoKey(String),
    Sealed(String),
    Unread(String),
    Stale(String),
    OtherMachine(String),
    Machine(String),
}

impl Fault {
    pub fn name(&self) -> &'static str {
        match self {
            Fault::Root(_) => "E_STORE_ROOT",
            Fault::Name(_) => "E_STORE_NAME",
            Fault::Kind(_) => "E_STORE_KIND",
            Fault::Version { .. } => "E_STORE_VERSION",
            Fault::Form(_) => "E_STORE_FORM",
            Fault::Missing(_) => "E_STORE_MISSING",
            Fault::Exists(_) => "E_STORE_EXISTS",
            Fault::Io(_) => "E_STORE_IO",
            Fault::Locked { .. } => "E_STORE_LOCKED",
            Fault::Archive(_) => "E_STORE_ARCHIVE",
            Fault::Mirror(_) => "E_STORE_MIRROR",
            Fault::Time(_) => "E_STORE_TIME",
            Fault::Cap { .. } => "E_STORE_CAP",
            Fault::NoKey(_) => "E_STORE_NO_KEY",
            Fault::Sealed(_) => "E_STORE_SEALED",
            Fault::Unread(_) => "E_STORE_UNREAD",
            Fault::Stale(_) => "E_STORE_STALE",
            Fault::OtherMachine(_) => "E_STORE_OTHER_MACHINE",
            Fault::Machine(_) => "E_STORE_MACHINE",
        }
    }

    pub fn detail(&self) -> String {
        match self {
            Fault::Root(s)
            | Fault::Name(s)
            | Fault::Kind(s)
            | Fault::Form(s)
            | Fault::Missing(s)
            | Fault::Exists(s)
            | Fault::Io(s)
            | Fault::Archive(s)
            | Fault::Mirror(s)
            | Fault::NoKey(s)
            | Fault::Sealed(s)
            | Fault::Unread(s)
            | Fault::Stale(s)
            | Fault::OtherMachine(s)
            | Fault::Machine(s) => s.clone(),
            Fault::Version { want, got } => format!("认得的版本是 {want},档上写的是 {got}"),
            Fault::Locked { holder } => format!("另一个实例握着这一家的笔,这里只读得:{holder}"),
            Fault::Time(t) => format!("{t} 超出正典整数界"),
            Fault::Cap { what, cap } => format!("{what}至多 {cap} 位"),
        }
    }
}

pub type R<T> = Result<T, Fault>;

pub fn classify(e: &std::io::Error, subject: &str) -> Fault {
    let said = format!("{subject}: {e}");
    match e.kind() {
        std::io::ErrorKind::NotFound => Fault::Missing(said),
        _ => Fault::Io(said),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Known {
    MachineShape,
    SettingsShape,
}

impl Fault {
    pub fn known(k: Known, subject: impl Into<String>) -> Fault {
        let subject = subject.into();
        match k {
            Known::MachineShape => Fault::Machine(format!("{subject} 在,但不是一枚本机标")),
            Known::SettingsShape => Fault::Form(format!("{subject} 不是一行本机标(三十二位小写十六进制)")),
        }
    }

    pub fn evidence(&self) -> String {
        self.detail()
    }
}

impl From<eravon_seal::Trouble> for Fault {
    fn from(t: eravon_seal::Trouble) -> Fault {
        use eravon_seal::Code as C;
        match t.code {
            C::FileMissing => Fault::Missing(t.tail),
            C::MachineShape => Fault::known(Known::MachineShape, t.tail),
            C::SettingsShape => Fault::known(Known::SettingsShape, t.tail),
            C::LocalSeal => Fault::Sealed(t.tail),
            C::Locked => Fault::NoKey(t.tail),
            C::Denied
            | C::NotAFolder
            | C::Unknown
            | C::LockedOut
            | C::KeyboxMissing
            | C::KeyboxShape
            | C::KeyboxSlot
            | C::KeyboxLocked
            | C::KeyboxNotEmpty
            | C::KdfBelowFloor
            | C::KeystoreParams
            | C::PinSet
            | C::PinShape
            | C::PinWrong
            | C::RecoveryNoMatch
            | C::RecoveryNotPrimary
            | C::IdentitiesShape
            | C::CredentialExpired
            | C::CredentialMissing
            | C::CredentialShape
            | C::CredentialPassMissing
            | C::CredentialPassUnknown
            | C::CredentialNote
            | C::CredentialFull
            | C::CredentialRowGone => Fault::Io(t.tail),
        }
    }
}
