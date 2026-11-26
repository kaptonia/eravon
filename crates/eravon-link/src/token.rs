use eravon_terms::token::{Fault as TFault, Token as TToken};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Token {
    Up(TToken),

    Listing,
    ListingMissing,
    ListingClosed,
    ListingV,
    ListingChain,
    ListingToken,
    ListingLogic,
    ListingCodehash,
    ListingTerms,
    ListingTermsHash,
    ListingTermsDoc,
    ListingOrderMissing,
    ListingOrderClosed,
    ListingSeller,
    ListingPayout,
    ListingFund,
    ListingAmount,
    ListingDeliveryWindow,
    ListingConfirmWindow,
    ListingOpenWindow,
    ListingBuyerOpen,
    ListingLadder,
    ListingNonce,
    ListingExpiry,
    ListingJudges,
    ListingGuilds,
    ListingDupJudge,
    ListingDupGuild,
    ListingNoCandidate,
    ListingBind,
    ListingSig,

    JudgeSig,
    GuildSeq,
    GuildPanel,
    GuildSig,

    Link,
    LinkMissing,
    LinkClosed,
    LinkV,
    LinkChain,
    LinkTx,
    LinkHash,

    LinkPrefix,
    LinkB64,
    LinkDoc,
    ReceiptPrefix,
    ReceiptB64,
    ReceiptDoc,

    Receipt,
    ReceiptMissing,
    ReceiptClosed,
    ReceiptV,
    ReceiptTerms,
    ReceiptCandidate,
    ReceiptBuyer,
    ReceiptPayout,
    ReceiptOpenBy,
    ReceiptPrev,
    ReceiptSig,
    ReceiptSeq,

    Pack,
    PackMissing,
    PackClosed,
    PackV,
    PackChain,
    PackInstance,
    PackCase,
    PackLink,
    PackListing,
    PackHash,
    PackReceipt,
    PackOrder,
    PackChainMismatch,
    PackInstanceMismatch,
    PackCaseMismatch,
    PackAttachments,
    PackAttachment,
    PackAttachmentHash,
    PackAttachmentUnnamed,

    Settle,
    SettleMissing,
    SettleClosed,
    SettleV,
    SettleChain,
    SettleInstance,
    SettleBps,
    SettleExpiry,
    SettleSig,
}

impl Token {
    pub fn name(self) -> &'static str {
        match self {
            Token::Up(t) => t.name(),
            Token::Listing => "E_LISTING",
            Token::ListingMissing => "E_LISTING_MISSING",
            Token::ListingClosed => "E_LISTING_CLOSED",
            Token::ListingV => "E_LISTING_V",
            Token::ListingChain => "E_LISTING_CHAIN",
            Token::ListingToken => "E_LISTING_TOKEN",
            Token::ListingLogic => "E_LISTING_LOGIC",
            Token::ListingCodehash => "E_LISTING_CODEHASH",
            Token::ListingTerms => "E_LISTING_TERMS",
            Token::ListingTermsHash => "E_LISTING_TERMS_HASH",
            Token::ListingTermsDoc => "E_LISTING_TERMS_DOC",
            Token::ListingOrderMissing => "E_LISTING_ORDER_MISSING",
            Token::ListingOrderClosed => "E_LISTING_ORDER_CLOSED",
            Token::ListingSeller => "E_LISTING_SELLER",
            Token::ListingPayout => "E_LISTING_PAYOUT",
            Token::ListingFund => "E_LISTING_FUND",
            Token::ListingAmount => "E_LISTING_AMOUNT",
            Token::ListingDeliveryWindow => "E_LISTING_DELIVERY_WINDOW",
            Token::ListingConfirmWindow => "E_LISTING_CONFIRM_WINDOW",
            Token::ListingOpenWindow => "E_LISTING_OPENWINDOW",
            Token::ListingBuyerOpen => "E_LISTING_BUYEROPEN",
            Token::ListingLadder => "E_LISTING_LADDER",
            Token::ListingNonce => "E_LISTING_NONCE",
            Token::ListingExpiry => "E_LISTING_EXPIRY",
            Token::ListingJudges => "E_LISTING_JUDGES",
            Token::ListingGuilds => "E_LISTING_GUILDS",
            Token::ListingDupJudge => "E_LISTING_DUP_JUDGE",
            Token::ListingDupGuild => "E_LISTING_DUP_GUILD",
            Token::ListingNoCandidate => "E_LISTING_NO_CANDIDATE",
            Token::ListingBind => "E_LISTING_BIND",
            Token::ListingSig => "E_LISTING_SIG",
            Token::JudgeSig => "E_JUDGE_SIG",
            Token::GuildSeq => "E_GUILD_SEQ",
            Token::GuildPanel => "E_GUILD_PANEL",
            Token::GuildSig => "E_GUILD_SIG",
            Token::Link => "E_LINK",
            Token::LinkMissing => "E_LINK_MISSING",
            Token::LinkClosed => "E_LINK_CLOSED",
            Token::LinkV => "E_LINK_V",
            Token::LinkChain => "E_LINK_CHAIN",
            Token::LinkTx => "E_LINK_TX",
            Token::LinkHash => "E_LINK_HASH",
            Token::LinkPrefix => "E_LINK_PREFIX",
            Token::LinkB64 => "E_LINK_B64",
            Token::LinkDoc => "E_LINK_DOC",
            Token::ReceiptPrefix => "E_RECEIPT_PREFIX",
            Token::ReceiptB64 => "E_RECEIPT_B64",
            Token::ReceiptDoc => "E_RECEIPT_DOC",
            Token::Receipt => "E_RECEIPT",
            Token::ReceiptMissing => "E_RECEIPT_MISSING",
            Token::ReceiptClosed => "E_RECEIPT_CLOSED",
            Token::ReceiptV => "E_RECEIPT_V",
            Token::ReceiptTerms => "E_RECEIPT_TERMS",
            Token::ReceiptCandidate => "E_RECEIPT_CANDIDATE",
            Token::ReceiptBuyer => "E_RECEIPT_BUYER",
            Token::ReceiptPayout => "E_RECEIPT_PAYOUT",
            Token::ReceiptOpenBy => "E_RECEIPT_OPENBY",
            Token::ReceiptPrev => "E_RECEIPT_PREV",
            Token::ReceiptSig => "E_RECEIPT_SIG",
            Token::ReceiptSeq => "E_RECEIPT_SEQ",
            Token::Pack => "E_PACK",
            Token::PackMissing => "E_PACK_MISSING",
            Token::PackClosed => "E_PACK_CLOSED",
            Token::PackV => "E_PACK_V",
            Token::PackChain => "E_PACK_CHAIN",
            Token::PackInstance => "E_PACK_INSTANCE",
            Token::PackCase => "E_PACK_CASE",
            Token::PackLink => "E_PACK_LINK",
            Token::PackListing => "E_PACK_LISTING",
            Token::PackHash => "E_PACK_HASH",
            Token::PackReceipt => "E_PACK_RECEIPT",
            Token::PackOrder => "E_PACK_ORDER",
            Token::PackChainMismatch => "E_PACK_CHAIN_MISMATCH",
            Token::PackInstanceMismatch => "E_PACK_INSTANCE_MISMATCH",
            Token::PackCaseMismatch => "E_PACK_CASE_MISMATCH",
            Token::PackAttachments => "E_PACK_ATTACHMENTS",
            Token::PackAttachment => "E_PACK_ATTACHMENT",
            Token::PackAttachmentHash => "E_PACK_ATTACHMENT_HASH",
            Token::PackAttachmentUnnamed => "E_PACK_ATTACHMENT_UNNAMED",
            Token::Settle => "E_SETTLE",
            Token::SettleMissing => "E_SETTLE_MISSING",
            Token::SettleClosed => "E_SETTLE_CLOSED",
            Token::SettleV => "E_SETTLE_V",
            Token::SettleChain => "E_SETTLE_CHAIN",
            Token::SettleInstance => "E_SETTLE_INSTANCE",
            Token::SettleBps => "E_SETTLE_BPS",
            Token::SettleExpiry => "E_SETTLE_EXPIRY",
            Token::SettleSig => "E_SETTLE_SIG",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Fault {
    pub token: Token,
    pub index: Option<usize>,
    pub inner: Option<Box<Carried>>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Carried {
    Up(TFault),
    Own(Fault),
    Verdict(Verdict),
}

impl Fault {
    pub fn new(token: Token) -> Fault {
        Fault { token, index: None, inner: None }
    }
    pub fn at(token: Token, index: usize) -> Fault {
        Fault { token, index: Some(index), inner: None }
    }
    pub fn up(mut self, f: TFault) -> Fault {
        self.inner = Some(Box::new(Carried::Up(f)));
        self
    }
    pub fn own(mut self, f: Fault) -> Fault {
        self.inner = Some(Box::new(Carried::Own(f)));
        self
    }
    pub fn verdict(mut self, v: Verdict) -> Fault {
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
        if let Some(c) = &self.inner {
            m.push(("inner".into(), carried_value(c)));
        }
        Value::Obj(m)
    }
}

pub fn carried_value(c: &Carried) -> eravon_galeed::Value {
    match c {
        Carried::Up(f) => f.to_value(false),
        Carried::Own(f) => f.to_value(false),
        Carried::Verdict(v) => v.to_value(false),
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Verdict {
    pub name: &'static str,
    pub index: Option<usize>,
    pub inner: Option<Box<Carried>>,
}

impl Verdict {
    pub fn new(name: &'static str) -> Verdict {
        Verdict { name, index: None, inner: None }
    }
    pub fn at(name: &'static str, index: usize) -> Verdict {
        Verdict { name, index: Some(index), inner: None }
    }
    pub fn up(mut self, f: TFault) -> Verdict {
        self.inner = Some(Box::new(Carried::Up(f)));
        self
    }
    pub fn own(mut self, f: Fault) -> Verdict {
        self.inner = Some(Box::new(Carried::Own(f)));
        self
    }

    pub fn to_value(&self, _with_ok: bool) -> eravon_galeed::Value {
        use eravon_galeed::Value;
        let mut m: Vec<(String, Value)> = vec![("token".into(), Value::Str(self.name.into()))];
        if let Some(i) = self.index {
            m.push(("index".into(), Value::Int(i as u64)));
        }
        if let Some(c) = &self.inner {
            m.push(("inner".into(), carried_value(c)));
        }
        Value::Obj(m)
    }

    pub fn to_answer(&self) -> eravon_galeed::Value {
        use eravon_galeed::Value;
        let mut m: Vec<(String, Value)> = vec![("verdict".into(), Value::Str(self.name.into()))];
        if let Some(i) = self.index {
            m.push(("index".into(), Value::Int(i as u64)));
        }
        if let Some(c) = &self.inner {
            m.push(("inner".into(), carried_value(c)));
        }
        Value::Obj(m)
    }
}

pub fn up(t: TToken) -> Token {
    Token::Up(t)
}
