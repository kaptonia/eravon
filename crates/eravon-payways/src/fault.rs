use eravon_store::fault::Fault as StoreFault;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    OpenWindow(String),
    Fund(String),
    Store { name: &'static str, detail: String },
}

pub type R<T> = Result<T, Fault>;

impl Fault {
    pub fn name(&self) -> &'static str {
        match self {
            Fault::OpenWindow(_) => "E_PAYWAYS_OPEN_WINDOW",
            Fault::Fund(_) => "E_PAYWAYS_FUND",
            Fault::Store { name, .. } => name,
        }
    }

    pub fn detail(&self) -> String {
        match self {
            Fault::OpenWindow(s) => format!("开案期限 {s} 秒不收:要大于 0、至多 {} 秒", u32::MAX),
            Fault::Fund(s) => format!("{s:?} 不是金库地址(0x 加 40 位小写十六进制、不为零)"),
            Fault::Store { detail, .. } => detail.clone(),
        }
    }

    pub const NAMES: [&'static str; 3] = ["E_PAYWAYS_OPEN_WINDOW", "E_PAYWAYS_FUND", "E_STORE_*"];
}

impl From<StoreFault> for Fault {
    fn from(e: StoreFault) -> Fault {
        Fault::Store { name: e.name(), detail: e.detail() }
    }
}
