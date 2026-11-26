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

impl From<eravon_scan::fault::Fault> for Fault {
    fn from(f: eravon_scan::fault::Fault) -> Fault {
        Fault::new(f.name(), f.detail())
    }
}

impl From<eravon_cli::deskops::No> for Fault {
    fn from(n: eravon_cli::deskops::No) -> Fault {
        Fault::new(n.name.clone(), n.detail())
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

impl From<eravon_link::token::Fault> for Fault {
    fn from(f: eravon_link::token::Fault) -> Fault {
        Fault::new(f.token.name(), format!("{f:?}"))
    }
}

impl From<eravon_dispute::Why> for Fault {
    fn from(w: eravon_dispute::Why) -> Fault {
        Fault::new(w.0, w.1)
    }
}

pub type R<T> = Result<T, Fault>;
