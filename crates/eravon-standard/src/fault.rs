#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    Form(String),
    Abi(String),
    CodeHash { want: String, got: String },
    PastContract { logic: String },
    PastListing { logic: String },
    Precheck { gate: &'static str, why: String },
    Chain { token: String, detail: String },
    Law { token: String, detail: String },
}

impl Fault {
    pub fn name(&self) -> String {
        match self {
            Fault::Form(_) => "E_A1_FORM".into(),
            Fault::Abi(_) => "E_A1_ABI".into(),
            Fault::CodeHash { .. } => "E_A1_CODEHASH".into(),
            Fault::PastContract { .. } => PAST_CONTRACT.into(),
            Fault::PastListing { .. } => PAST_LISTING.into(),
            Fault::Precheck { .. } => "E_A1_PRECHECK".into(),
            Fault::Chain { token, .. } | Fault::Law { token, .. } => token.clone(),
        }
    }

    pub fn detail(&self) -> String {
        match self {
            Fault::Form(s) | Fault::Abi(s) => s.clone(),
            Fault::CodeHash { want, got } => {
                format!("这个地址上的 codeHash 是 {got},标准那一枚是 {want}")
            }
            Fault::PastContract { .. } => PAST_CONTRACT_SAYS.into(),
            Fault::PastListing { .. } => PAST_LISTING_SAYS.into(),
            Fault::Precheck { gate, why } => format!("{gate}:{why}"),
            Fault::Chain { detail, .. } | Fault::Law { detail, .. } => detail.clone(),
        }
    }

    pub fn precheck_gate(&self) -> Option<&'static str> {
        match self {
            Fault::Precheck { gate, .. } => Some(gate),
            _ => None,
        }
    }
}

impl From<eravon_chain::fault::Fault> for Fault {
    fn from(f: eravon_chain::fault::Fault) -> Fault {
        Fault::Chain { token: f.name().to_string(), detail: f.detail() }
    }
}

pub type R<T> = Result<T, Fault>;

pub const PAST_CONTRACT: &str = "E_A1_PAST_CONTRACT";

pub const PAST_CONTRACT_SAYS: &str = "往代合约上的案:在设置里打开往代案(命令行 settings-past-cases --on true)后操作";

pub const PAST_LISTING: &str = "E_A1_PAST_LISTING";

pub const PAST_LISTING_SAYS: &str = "往代合约的清单不能再开新案";
