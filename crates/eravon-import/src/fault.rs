#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault {
    pub token: String,
    pub detail: String,
}

impl Fault {
    pub fn new(token: impl Into<String>, detail: impl Into<String>) -> Fault {
        Fault { token: token.into(), detail: detail.into() }
    }

    pub fn name(&self) -> &str {
        &self.token
    }

    pub fn detail(&self) -> String {
        self.detail.clone()
    }
}

impl From<eravon_link::token::Fault> for Fault {
    fn from(f: eravon_link::token::Fault) -> Fault {
        Fault::new(f.token.name(), "这不是一份订单文件;请选卖家或自己导出的那一份")
    }
}

impl From<eravon_vault::vfault::Fault> for Fault {
    fn from(f: eravon_vault::vfault::Fault) -> Fault {
        Fault::new(f.name(), f.detail())
    }
}

impl From<eravon_store::fault::Fault> for Fault {
    fn from(f: eravon_store::fault::Fault) -> Fault {
        Fault::new(f.name(), f.detail())
    }
}

impl From<eravon_scan::fault::Fault> for Fault {
    fn from(f: eravon_scan::fault::Fault) -> Fault {
        Fault::new(f.name(), f.detail())
    }
}

impl From<eravon_chain::fault::Fault> for Fault {
    fn from(f: eravon_chain::fault::Fault) -> Fault {
        Fault::new(f.name(), f.detail())
    }
}

pub type R<T> = Result<T, Fault>;
