#![forbid(unsafe_code)]

pub mod batch;
pub mod behind;
pub mod checked;
pub mod node;
pub mod sent;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Code {
    NoEndpoint,
    SettingsShape,
    Unreachable,
    Disagree,
    ChainShape,
    WrongChain,
    InsufficientFunds,
    NonceUsed,
    AlreadyPending,
    Underpriced,
    GasTooLow,
    ContractRefused,
    RateLimited,
    MethodMissing,
    NodeAuth,
    ServiceUnserved,
    ServiceKeyMissing,
    NodeRefused,
    NodeTls,
    NodeTimeout,
    AnswerTooLong,
    AnswerNotJson,
    ProxyUnreachable,
    ProxyCredentialsMissing,
    ProxyCredentialsRefused,

}

impl Code {
    pub fn another_chain(self) -> bool {
        matches!(self, Code::WrongChain)
    }

    pub const ALL: [Code; 25] = [
        Code::NoEndpoint,
        Code::SettingsShape,
        Code::Unreachable,
        Code::Disagree,
        Code::ChainShape,
        Code::WrongChain,
        Code::InsufficientFunds,
        Code::NonceUsed,
        Code::AlreadyPending,
        Code::Underpriced,
        Code::GasTooLow,
        Code::ContractRefused,
        Code::RateLimited,
        Code::MethodMissing,
        Code::NodeAuth,
        Code::ServiceUnserved,
        Code::ServiceKeyMissing,
        Code::NodeRefused,
        Code::NodeTls,
        Code::NodeTimeout,
        Code::AnswerTooLong,
        Code::AnswerNotJson,
        Code::ProxyUnreachable,
        Code::ProxyCredentialsMissing,
        Code::ProxyCredentialsRefused,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Code::NoEndpoint => "NO_ENDPOINT",
            Code::SettingsShape => "SETTINGS_SHAPE",
            Code::Unreachable => "UNREACHABLE",
            Code::Disagree => "DISAGREE",
            Code::ChainShape => "CHAIN_SHAPE",
            Code::WrongChain => "WRONG_CHAIN",
            Code::InsufficientFunds => "INSUFFICIENT_FUNDS",
            Code::NonceUsed => "NONCE_USED",
            Code::AlreadyPending => "ALREADY_PENDING",
            Code::Underpriced => "UNDERPRICED",
            Code::GasTooLow => "GAS_TOO_LOW",
            Code::ContractRefused => "CONTRACT_REFUSED",
            Code::RateLimited => "RATE_LIMITED",
            Code::MethodMissing => "METHOD_MISSING",
            Code::NodeAuth => "NODE_AUTH",
            Code::ServiceUnserved => "SERVICE_UNSERVED",
            Code::ServiceKeyMissing => "SERVICE_KEY_MISSING",
            Code::NodeRefused => "NODE_REFUSED",
            Code::NodeTls => "NODE_TLS",
            Code::NodeTimeout => "NODE_TIMEOUT",
            Code::AnswerTooLong => "ANSWER_TOO_LONG",
            Code::AnswerNotJson => "ANSWER_NOT_JSON",
            Code::ProxyUnreachable => "PROXY_UNREACHABLE",
            Code::ProxyCredentialsMissing => "PROXY_CREDENTIALS_MISSING",
            Code::ProxyCredentialsRefused => "PROXY_CREDENTIALS_REFUSED",
        }
    }

    pub const NETWORK: [Code; 18] = [
        Code::Unreachable,
        Code::NoEndpoint,
        Code::Disagree,
        Code::ChainShape,
        Code::NodeTls,
        Code::ProxyUnreachable,
        Code::ProxyCredentialsMissing,
        Code::ProxyCredentialsRefused,
        Code::NodeTimeout,
        Code::AnswerTooLong,
        Code::AnswerNotJson,
        Code::RateLimited,
        Code::MethodMissing,
        Code::NodeAuth,
        Code::ServiceUnserved,
        Code::ServiceKeyMissing,
        Code::WrongChain,
        Code::NodeRefused,
    ];
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Why {
    Text(String),
    Each(Vec<Why>),
    Node { node: String, trouble: eravon_chain::rpc::Trouble },
    Unreadable { node: String },
    Differs { node: String },
    Scheme { node: String },
    Port { node: String },
    NotAddress { node: String },
    NotSpec { node: String },
    NoHeadNode { chain: u64 },
    NoTimeNode { chain: u64 },
    NotHeight { node: String, text: String },
    Behind { node: String, lag: u64, high: u64 },
    NoTime { node: String },
    NotYetAt { has: String, not_yet: String },
    Differ { count: usize, sources: String },
    WrongChainNode { node: String, serves: u64, chain: u64 },
    BalanceNotText(String),
    BalanceNotQty(String),
    Funds { need: u128, have: u128 },
}

#[derive(Clone, PartialEq, Debug)]
pub struct Trouble {
    pub code: Code,
    pub why: Why,
    pub script: Option<eravon_net::PacTrouble>,
}

impl Trouble {
    pub fn new(code: Code, why: Why) -> Trouble {
        Trouble { code, why, script: None }
    }

    pub fn text(code: Code, text: impl Into<String>) -> Trouble {
        Trouble::new(code, Why::Text(text.into()))
    }

    pub fn retold(mut self, why: Why) -> Trouble {
        self.why = why;
        self
    }

    pub fn network(&self) -> bool {
        Code::NETWORK.contains(&self.code)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Note {
    Stage(u8),
    Balance,
}

pub fn node_said(t: &eravon_chain::rpc::Trouble) -> Option<&str> {
    match t {
        eravon_chain::rpc::Trouble::Node(e) => Some(e.as_str()),
        _ => None,
    }
}
