use eravon_link::token::{Carried as LCarried, Fault as LFault, Verdict as LVerdict};
use eravon_terms::token::Fault as TFault;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Token {
    Up(eravon_link::token::Token),
    Dossier,
    Missing,
    Closed,
    V,
    Chain,
    Instance,
    Case,
    Logic,
    Codehash,
    Posted,
    Files,
    Seat,
    Transactions,
    Transaction,
    Terminal,
    Ruled,
    Compliance,
    File,
    Digest,
    Listing,
    Hash,
    Receipt,
    Order,
    ChainMismatch,
    LogicMismatch,
    CodehashMismatch,
    InstanceMismatch,
    CaseMismatch,
    SeatMismatch,
    Entry,
    EntryDigest,
    EntryForm,
    EntryType,
    Subject,
    Attachment,
}

impl Token {
    pub fn name(self) -> &'static str {
        match self {
            Token::Up(t) => t.name(),
            Token::Dossier => "E_DOSSIER",
            Token::Missing => "E_DOSSIER_MISSING",
            Token::Closed => "E_DOSSIER_CLOSED",
            Token::V => "E_DOSSIER_V",
            Token::Chain => "E_DOSSIER_CHAIN",
            Token::Instance => "E_DOSSIER_INSTANCE",
            Token::Case => "E_DOSSIER_CASE",
            Token::Logic => "E_DOSSIER_LOGIC",
            Token::Codehash => "E_DOSSIER_CODEHASH",
            Token::Posted => "E_DOSSIER_POSTED",
            Token::Files => "E_DOSSIER_FILES",
            Token::Seat => "E_DOSSIER_SEAT",
            Token::Transactions => "E_DOSSIER_TRANSACTIONS",
            Token::Transaction => "E_TRANSACTION",
            Token::Terminal => "E_DOSSIER_TERMINAL",
            Token::Ruled => "E_DOSSIER_RULED",
            Token::Compliance => "E_DOSSIER_COMPLIANCE",
            Token::File => "E_DOSSIER_FILE",
            Token::Digest => "E_DOSSIER_DIGEST",
            Token::Listing => "E_DOSSIER_LISTING",
            Token::Hash => "E_DOSSIER_HASH",
            Token::Receipt => "E_DOSSIER_RECEIPT",
            Token::Order => "E_DOSSIER_ORDER",
            Token::ChainMismatch => "E_DOSSIER_CHAIN_MISMATCH",
            Token::LogicMismatch => "E_DOSSIER_LOGIC_MISMATCH",
            Token::CodehashMismatch => "E_DOSSIER_CODEHASH_MISMATCH",
            Token::InstanceMismatch => "E_DOSSIER_INSTANCE_MISMATCH",
            Token::CaseMismatch => "E_DOSSIER_CASE_MISMATCH",
            Token::SeatMismatch => "E_DOSSIER_SEAT_MISMATCH",
            Token::Entry => "E_DOSSIER_ENTRY",
            Token::EntryDigest => "E_DOSSIER_ENTRY_DIGEST",
            Token::EntryForm => "E_DOSSIER_ENTRY_FORM",
            Token::EntryType => "E_DOSSIER_ENTRY_TYPE",
            Token::Subject => "E_DOSSIER_SUBJECT",
            Token::Attachment => "E_DOSSIER_ATTACHMENT",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Fault {
    pub token: Token,
    pub index: Option<usize>,
    pub key: Option<String>,
    pub inner: Option<Box<Carried>>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Carried {
    Terms(TFault),
    Link(LFault),
    Verdict(LVerdict),
}

impl Fault {
    pub fn new(token: Token) -> Fault {
        Fault { token, index: None, key: None, inner: None }
    }
    pub fn at(token: Token, index: usize) -> Fault {
        Fault { token, index: Some(index), key: None, inner: None }
    }
    pub fn keyed(token: Token, key: &str) -> Fault {
        Fault { token, index: None, key: Some(key.to_string()), inner: None }
    }
    pub fn terms(mut self, f: TFault) -> Fault {
        self.inner = Some(Box::new(Carried::Terms(f)));
        self
    }
    pub fn link(mut self, f: LFault) -> Fault {
        self.inner = Some(Box::new(Carried::Link(f)));
        self
    }
    pub fn verdict(mut self, v: LVerdict) -> Fault {
        self.inner = Some(Box::new(Carried::Verdict(v)));
        self
    }

    pub fn to_value(&self, with_ok: bool) -> eravon_galeed::Value {
        use eravon_galeed::Value;
        let mut m: Vec<(String, Value)> = Vec::new();
        if with_ok {
            m.push(("ok".into(), Value::Bool(false)));
        }
        m.push(("token".into(), Value::Str(self.token.name().into())));
        if let Some(i) = self.index {
            m.push(("index".into(), Value::Int(i as u64)));
        }
        if let Some(k) = &self.key {
            m.push(("key".into(), Value::Str(k.clone())));
        }
        if let Some(c) = &self.inner {
            m.push((
                "inner".into(),
                match c.as_ref() {
                    Carried::Terms(f) => f.to_value(false),
                    Carried::Link(f) => f.to_value(false),
                    Carried::Verdict(v) => v.to_value(false),
                },
            ));
        }
        Value::Obj(m)
    }
}

pub fn pass_link(f: LFault) -> Fault {
    let mut out = Fault { token: Token::Up(f.token), index: f.index, key: None, inner: None };
    if let Some(inner) = f.inner {
        out.inner = Some(Box::new(match *inner {
            LCarried::Up(t) => Carried::Terms(t),
            LCarried::Own(l) => Carried::Link(l),
            LCarried::Verdict(v) => Carried::Verdict(v),
        }));
    }
    out
}
