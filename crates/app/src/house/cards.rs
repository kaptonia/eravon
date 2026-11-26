#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetCard {
    Zikaron { dir: String },
    Company { addr: String },
    TokenAdd { address: String, label: String, decimals: String },
    TokenRemove { address: String, label: String },
    TokenDecimals { address: String, text: String },
    Rotate { keep: bool, let_expire: Vec<String>, revoke: Vec<String> },
}

impl SetCard {
    pub fn out(&self) -> eravon_literals::json::Out {
        use eravon_literals::json::Out;
        let name = match self {
            SetCard::Zikaron { .. } => "zikaron",
            SetCard::Company { .. } => "company",
            SetCard::TokenAdd { .. } => "tokenAdd",
            SetCard::TokenRemove { .. } => "tokenRemove",
            SetCard::TokenDecimals { .. } => "tokenDecimals",
            SetCard::Rotate { .. } => "rotate",
        };
        let mut m = vec![("name", Out::S(name.to_string()))];
        if let SetCard::Rotate { keep, .. } = self {
            m.push(("keep", Out::B(*keep)));
        }
        m.sort_by(|a, b| a.0.cmp(b.0));
        Out::O(m)
    }
}
