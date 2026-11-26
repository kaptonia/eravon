use eravon_zikaron as a4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    Io(String),
    Missing(String),
    Exists(String),
    State(String),
    Precheck { gate: String, why: String },
    Down { layer: &'static str, token: String, why: String },
    Rule { rule: &'static str, why: String },
    NotYet { door: &'static str, owner: &'static str },
    Reverted { tx_hash: String },
    Unconfirmed { tx_hash: String, why: String },
    Series { sent: Vec<(String, &'static str)>, stop: Box<Fault> },
    Arg(String),
    SentVoid { tx_hash: String, nonce: u64 },
    Unheard { tx_hash: String, why: String },
    TxNotYet { tx_hash: String, waited: u64 },
    Named { name: &'static str, why: String },
}

impl Fault {
    pub fn name(&self) -> &'static str {
        match self {
            Fault::Io(_) => "E_V_IO",
            Fault::Missing(_) => "E_V_MISSING",
            Fault::Exists(_) => "E_V_EXISTS",
            Fault::State(_) => "E_V_STATE",
            Fault::Precheck { .. } => "E_V_PRECHECK",
            Fault::Down { .. } | Fault::Rule { .. } => "E_V_DOWN",
            Fault::NotYet { .. } => "E_V_NOT_YET",
            Fault::Reverted { .. } => "E_V_REVERTED",
            Fault::Unconfirmed { .. } => "E_V_UNCONFIRMED",
            Fault::Series { stop, .. } => stop.name(),
            Fault::Arg(_) => "E_V_ARG",
            Fault::SentVoid { .. } => "E_V_SENT_VOID",
            Fault::Unheard { .. } => "E_UNREACHABLE",
            Fault::TxNotYet { .. } => "E_TX_NOT_YET",
            Fault::Named { name, .. } => name,
        }
    }

    pub fn detail(&self) -> String {
        match self {
            Fault::Io(s) | Fault::Missing(s) | Fault::Exists(s) | Fault::State(s) | Fault::Arg(s) => {
                s.clone()
            }
            Fault::Precheck { gate, why } => format!("{PRECHECK_HEAD}{gate}{PRECHECK_SAYS}{why}"),
            Fault::Down { layer, token, why } => format!("{layer}{DOWN_SAYS}{token}:{why}"),
            Fault::Rule { why, .. } => format!("H2{DOWN_SAYS}{RULE_TOKEN}:{why}"),
            Fault::NotYet { door, owner } => format!("{door} 这一扇门命令行没有,归 {owner}"),
            Fault::Reverted { tx_hash } => {
                format!("这一笔发出去了({tx_hash}),而链把它回滚了(收据的 status 是 0)")
            }
            Fault::Unconfirmed { tx_hash, why } => format!("{tx_hash} 发出去了而说不出成败:{why};别再发这一笔,先在链上核实"),
            Fault::Series { sent, stop } => {
                let list: Vec<String> = sent.iter().map(|(tx, word)| format!("{tx}({word})")).collect();
                format!("连发 {} 笔之后停下:{};已发出的是 {}", sent.len(), stop.detail(), list.join("、"))
            }
            Fault::SentVoid { tx_hash, nonce } => format!("本机先前直发的这一笔 {tx_hash}(nonce {nonce})没有节点持有、那一个 nonce 已被用掉,再也上不了链;这一次什么也没发,已记为作废,再跑同一条命令即重新发出"),
            Fault::Unheard { why, .. } | Fault::Named { why, .. } => why.clone(),
            Fault::TxNotYet { tx_hash, waited } => format!("{tx_hash} 发出去了,等了 {waited} 秒还没上块;别再发这一笔,再跑同一条命令即先按哈希问它"),
        }
    }

    pub fn extra(&self) -> Vec<(&'static str, eravon_literals::json::Out)> {
        use eravon_literals::json::Out;
        match self {
            Fault::Precheck { gate, .. } => vec![("gate", Out::S(gate.clone()))],
            Fault::Down { layer, token, .. } => {
                vec![("downToken", Out::S(token.clone())), ("layer", Out::S(layer.to_string()))]
            }
            Fault::Rule { rule, .. } => vec![
                ("downToken", Out::S(RULE_TOKEN.to_string())),
                ("layer", Out::S("H2".to_string())),
                ("rule", Out::S(rule.to_string())),
            ],
            Fault::NotYet { door, owner } => {
                vec![("door", Out::S(door.to_string())), ("owner", Out::S(owner.to_string()))]
            }
            Fault::Reverted { tx_hash } | Fault::Unconfirmed { tx_hash, .. } => vec![("txHash", Out::S(tx_hash.clone()))],
            Fault::SentVoid { tx_hash, nonce } => vec![("nonce", Out::N(*nonce)), ("txHash", Out::S(tx_hash.clone()))],
            Fault::Unheard { tx_hash, .. } => vec![("tx", Out::S(tx_hash.clone()))],
            Fault::TxNotYet { tx_hash, waited } => vec![("count", Out::N(*waited)), ("tx", Out::S(tx_hash.clone()))],
            Fault::Series { sent, stop } => {
                let mut m: Vec<(&'static str, Out)> = stop.extra().into_iter().filter(|(k, _)| *k != "txs").collect();
                m.push(("txs", Out::A(sent.iter().map(|(tx, word)| Out::O(vec![("chain", Out::S((*word).to_string())), ("tx", Out::S(tx.clone()))])).collect())));
                m
            }
            _ => Vec::new(),
        }
    }
}

pub type R<T> = Result<T, Fault>;

const RULE_TOKEN: &str = "E_VAULT_RULE";

impl From<eravon_store::fault::Fault> for Fault {
    fn from(f: eravon_store::fault::Fault) -> Fault {
        store_down(f.name(), f.detail())
    }
}

pub fn store_down(name: &str, why: String) -> Fault {
    if name == (eravon_store::fault::Fault::Cap { what: "", cap: 0 }).name() {
        return Fault::State(why);
    }
    if name == eravon_store::fault::Fault::NoKey(String::new()).name() {
        return Fault::Down { layer: "H2", token: "E_VAULT_LOCKED".to_string(), why };
    }
    Fault::Down { layer: "A5", token: name.to_string(), why }
}

const PRECHECK_HEAD: &str = "本地先验的 ";
const PRECHECK_SAYS: &str = " 那一扇拦住了:";

pub fn precheck_of(detail: &str) -> Option<(&str, &str)> {
    let (gate, why) = detail.strip_prefix(PRECHECK_HEAD)?.split_once(PRECHECK_SAYS)?;
    (!gate.is_empty()).then_some((gate, why))
}

pub const GATE_FEE: &str = "fee";
pub const GATE_ROUTE: &str = "route";
pub const GATE_COMPANY: &str = "company";
pub const GATE_TIME: &str = "time";

const DOWN_SAYS: &str = " 说 ";

pub fn down_of(detail: &str) -> Option<(&str, &str, &str)> {
    let (layer, rest) = detail.split_once(DOWN_SAYS)?;
    let (token, why) = rest.split_once(':')?;
    (!layer.is_empty() && token.starts_with("E_")).then_some((layer, token, why))
}

impl From<eravon_chain::fault::Fault> for Fault {
    fn from(f: eravon_chain::fault::Fault) -> Fault {
        Fault::Down { layer: "A2", token: f.name().to_string(), why: f.detail() }
    }
}

impl From<eravon_standard::fault::Fault> for Fault {
    fn from(f: eravon_standard::fault::Fault) -> Fault {
        match f {
            eravon_standard::fault::Fault::Precheck { gate, why } => {
                Fault::Precheck { gate: gate.to_string(), why }
            }
            other => Fault::Down { layer: "A1", token: other.name(), why: other.detail() },
        }
    }
}

impl From<eravon_scan::fault::Fault> for Fault {
    fn from(f: eravon_scan::fault::Fault) -> Fault {
        Fault::Down { layer: "A3", token: f.name(), why: f.detail() }
    }
}

impl From<a4::fault::Fault> for Fault {
    fn from(f: a4::fault::Fault) -> Fault {
        Fault::Down { layer: "A4", token: f.name().to_string(), why: f.detail() }
    }
}

impl From<eravon_terms::token::Fault> for Fault {
    fn from(f: eravon_terms::token::Fault) -> Fault {
        let why = match f.index {
            Some(i) => format!("第 {i} 位"),
            None => String::new(),
        };
        Fault::Down { layer: "K2", token: f.token.name().to_string(), why }
    }
}

impl From<eravon_link::token::Fault> for Fault {
    fn from(f: eravon_link::token::Fault) -> Fault {
        let why = match f.index {
            Some(i) => format!("第 {i} 位"),
            None => String::new(),
        };
        Fault::Down { layer: "K3", token: f.token.name().to_string(), why }
    }
}

impl From<eravon_link::token::Verdict> for Fault {
    fn from(v: eravon_link::token::Verdict) -> Fault {
        let why = match v.index {
            Some(i) => format!("第 {i} 位"),
            None => String::new(),
        };
        Fault::Down { layer: "K3", token: v.name.to_string(), why }
    }
}
