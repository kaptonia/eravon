#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault {
    pub token: String,
    pub detail: String,
}

impl Fault {
    pub fn new(token: impl Into<String>, detail: impl Into<String>) -> Fault {
        Fault { token: token.into(), detail: detail.into() }
    }
}

impl From<eravon_cli::fault::Fault> for Fault {
    fn from(f: eravon_cli::fault::Fault) -> Fault {
        Fault::new(f.name(), f.detail())
    }
}

impl From<eravon_vault::vfault::Fault> for Fault {
    fn from(f: eravon_vault::vfault::Fault) -> Fault {
        Fault::new(f.name(), f.detail())
    }
}

impl From<eravon_standard::fault::Fault> for Fault {
    fn from(f: eravon_standard::fault::Fault) -> Fault {
        Fault::new(f.name(), f.detail())
    }
}

impl From<eravon_chain::fault::Fault> for Fault {
    fn from(f: eravon_chain::fault::Fault) -> Fault {
        Fault::new(f.name(), f.detail())
    }
}

impl From<eravon_store::fault::Fault> for Fault {
    fn from(f: eravon_store::fault::Fault) -> Fault {
        Fault::new(f.name(), f.detail())
    }
}

pub fn lift(f: eravon_cli::fault::Fault) -> Fault {
    match f {
        eravon_cli::fault::Fault::Down { token, why, .. } => Fault::new(token, why),
        other => other.into(),
    }
}

pub type R<T> = Result<T, Fault>;
