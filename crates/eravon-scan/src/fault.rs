#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    Form(String),
    Rebuild { want: String, got: String },
    Upstream { token: String, detail: String },
}

impl Fault {
    pub fn name(&self) -> String {
        match self {
            Fault::Form(_) => "E_A3_FORM".into(),
            Fault::Rebuild { .. } => "E_A3_REBUILD".into(),
            Fault::Upstream { token, .. } => token.clone(),
        }
    }
    pub fn detail(&self) -> String {
        match self {
            Fault::Form(s) => s.clone(),
            Fault::Rebuild { want, got } => {
                format!("链上那一枚案号是 {want},重建出来的条款算出的是 {got}")
            }
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

pub type R<T> = Result<T, Fault>;
