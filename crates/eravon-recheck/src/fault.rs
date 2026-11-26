#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    Dossier(String),
    Form(String),
    Upstream { token: String, detail: String },
}

impl Fault {
    pub fn name(&self) -> String {
        match self {
            Fault::Dossier(_) => "E_R1_DOSSIER".into(),
            Fault::Form(_) => "E_R1_FORM".into(),
            Fault::Upstream { token, .. } => token.clone(),
        }
    }
    pub fn detail(&self) -> String {
        match self {
            Fault::Dossier(s) | Fault::Form(s) => s.clone(),
            Fault::Upstream { detail, .. } => detail.clone(),
        }
    }
}

impl From<eravon_chain::fault::Fault> for Fault {
    fn from(f: eravon_chain::fault::Fault) -> Fault {
        Fault::Upstream { token: f.name().to_string(), detail: f.detail() }
    }
}

impl From<eravon_standard::fault::Fault> for Fault {
    fn from(f: eravon_standard::fault::Fault) -> Fault {
        Fault::Upstream { token: f.name(), detail: f.detail() }
    }
}

impl From<eravon_scan::fault::Fault> for Fault {
    fn from(f: eravon_scan::fault::Fault) -> Fault {
        Fault::Upstream { token: f.name(), detail: f.detail() }
    }
}

pub type R<T> = Result<T, Fault>;
