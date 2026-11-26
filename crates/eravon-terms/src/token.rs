use eravon_galeed as k1;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Token {
    Canon(k1::Token),

    TermsUtf8,
    TermsBlock,
    TermsFences,
    TermsUnclosed,
    TermsOutside,
    Terms,
    TermsMissing,
    TermsClosed,
    TermsV,
    TermsChain,
    TermsLogic,
    TermsSeller,
    TermsPayout,
    TermsFund,
    TermsToken,
    TermsAmount,
    TermsDeliveryWindow,
    TermsConfirmWindow,
    TermsOpenWindow,
    TermsBuyerOpen,
    TermsGoods,
    TermsDelivery,
    TermsLadder,
    TermsJudges,
    TermsGuilds,
    TermsDupJudge,
    TermsDupGuild,
    TermsNoCandidate,
    TermsNonce,
    TermsExpiry,
    TermsCompliance,
    TermsSeries,

    Rung,
    RungMissing,
    RungClosed,
    RungBuyer,
    RungSeller,
    RungBearer,
    RungNote,
    RungSum,
    LadderFirst,
    LadderLast,
    LadderOrder,

    Judge,
    JudgeMissing,
    JudgeClosed,
    JudgeAddress,
    JudgeAmanah,
    JudgeQuote,
    JudgeSource,

    Guild,
    GuildMissing,
    GuildClosed,
    GuildPointer,
    GuildAmanah,
    GuildQuote,

    QuoteLen,
    QuoteField,
    QuoteFeed,

    OfferCandidate,
    OrderBuyer,
    OrderPayout,
    OrderOpenBy,
    OrderPrev,
    ActionManifest,
    ActionBps,
    ActionParty,

    FeeOverflow,

    SigForm,
    SigV,
    SigRange,
    SigHighS,
    SigRecover,
    SigSigner,

}

impl Token {
    pub fn name(self) -> &'static str {
        match self {
            Token::Canon(t) => t.name(),
            Token::TermsUtf8 => "E_TERMS_UTF8",
            Token::TermsBlock => "E_TERMS_BLOCK",
            Token::TermsFences => "E_TERMS_FENCES",
            Token::TermsUnclosed => "E_TERMS_UNCLOSED",
            Token::TermsOutside => "E_TERMS_OUTSIDE",
            Token::Terms => "E_TERMS",
            Token::TermsMissing => "E_TERMS_MISSING",
            Token::TermsClosed => "E_TERMS_CLOSED",
            Token::TermsV => "E_TERMS_V",
            Token::TermsChain => "E_TERMS_CHAIN",
            Token::TermsLogic => "E_TERMS_LOGIC",
            Token::TermsSeller => "E_TERMS_SELLER",
            Token::TermsPayout => "E_TERMS_PAYOUT",
            Token::TermsFund => "E_TERMS_FUND",
            Token::TermsToken => "E_TERMS_TOKEN",
            Token::TermsAmount => "E_TERMS_AMOUNT",
            Token::TermsDeliveryWindow => "E_TERMS_DELIVERY_WINDOW",
            Token::TermsConfirmWindow => "E_TERMS_CONFIRM_WINDOW",
            Token::TermsOpenWindow => "E_TERMS_OPENWINDOW",
            Token::TermsBuyerOpen => "E_TERMS_BUYEROPEN",
            Token::TermsGoods => "E_TERMS_GOODS",
            Token::TermsDelivery => "E_TERMS_DELIVERY",
            Token::TermsLadder => "E_TERMS_LADDER",
            Token::TermsJudges => "E_TERMS_JUDGES",
            Token::TermsGuilds => "E_TERMS_GUILDS",
            Token::TermsDupJudge => "E_TERMS_DUP_JUDGE",
            Token::TermsDupGuild => "E_TERMS_DUP_GUILD",
            Token::TermsNoCandidate => "E_TERMS_NO_CANDIDATE",
            Token::TermsNonce => "E_TERMS_NONCE",
            Token::TermsExpiry => "E_TERMS_EXPIRY",
            Token::TermsCompliance => "E_TERMS_COMPLIANCE",
            Token::TermsSeries => "E_TERMS_SERIES",
            Token::Rung => "E_RUNG",
            Token::RungMissing => "E_RUNG_MISSING",
            Token::RungClosed => "E_RUNG_CLOSED",
            Token::RungBuyer => "E_RUNG_BUYER",
            Token::RungSeller => "E_RUNG_SELLER",
            Token::RungBearer => "E_RUNG_BEARER",
            Token::RungNote => "E_RUNG_NOTE",
            Token::RungSum => "E_RUNG_SUM",
            Token::LadderFirst => "E_LADDER_FIRST",
            Token::LadderLast => "E_LADDER_LAST",
            Token::LadderOrder => "E_LADDER_ORDER",
            Token::Judge => "E_JUDGE",
            Token::JudgeMissing => "E_JUDGE_MISSING",
            Token::JudgeClosed => "E_JUDGE_CLOSED",
            Token::JudgeAddress => "E_JUDGE_ADDRESS",
            Token::JudgeAmanah => "E_JUDGE_AMANAH",
            Token::JudgeQuote => "E_JUDGE_QUOTE",
            Token::JudgeSource => "E_JUDGE_SOURCE",
            Token::Guild => "E_GUILD",
            Token::GuildMissing => "E_GUILD_MISSING",
            Token::GuildClosed => "E_GUILD_CLOSED",
            Token::GuildPointer => "E_GUILD_POINTER",
            Token::GuildAmanah => "E_GUILD_AMANAH",
            Token::GuildQuote => "E_GUILD_QUOTE",
            Token::QuoteLen => "E_QUOTE_LEN",
            Token::QuoteField => "E_QUOTE_FIELD",
            Token::QuoteFeed => "E_QUOTE_FEED",
            Token::OfferCandidate => "E_OFFER_CANDIDATE",
            Token::OrderBuyer => "E_ORDER_BUYER",
            Token::OrderPayout => "E_ORDER_PAYOUT",
            Token::OrderOpenBy => "E_ORDER_OPENBY",
            Token::OrderPrev => "E_ORDER_PREV",
            Token::ActionManifest => "E_ACTION_MANIFEST",
            Token::ActionBps => "E_ACTION_BPS",
            Token::ActionParty => "E_ACTION_PARTY",
            Token::FeeOverflow => "E_FEE_OVERFLOW",
            Token::SigForm => "E_SIG_FORM",
            Token::SigV => "E_SIG_V",
            Token::SigRange => "E_SIG_RANGE",
            Token::SigHighS => "E_SIG_HIGH_S",
            Token::SigRecover => "E_SIG_RECOVER",
            Token::SigSigner => "E_SIG_SIGNER",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Fault {
    pub token: Token,
    pub index: Option<usize>,
    pub field: Option<usize>,
    pub inner: Option<Box<Fault>>,
}

impl Fault {
    pub fn new(token: Token) -> Fault {
        Fault { token, index: None, field: None, inner: None }
    }
    pub fn at(token: Token, index: usize) -> Fault {
        Fault { token, index: Some(index), field: None, inner: None }
    }
    pub fn field(token: Token, field: usize) -> Fault {
        Fault { token, index: None, field: Some(field), inner: None }
    }
    pub fn with_inner(mut self, inner: Fault) -> Fault {
        self.inner = Some(Box::new(inner));
        self
    }
    pub fn with_index(mut self, index: usize) -> Fault {
        self.index = Some(index);
        self
    }

    pub fn to_value(&self, with_ok: bool) -> k1::Value {
        let mut m: Vec<(String, k1::Value)> = Vec::new();
        if with_ok {
            m.push(("ok".into(), k1::Value::Bool(false)));
        }
        m.push(("token".into(), k1::Value::Str(self.token.name().into())));
        if let Some(i) = self.index {
            m.push(("index".into(), k1::Value::Int(i as u64)));
        }
        if let Some(f) = self.field {
            m.push(("field".into(), k1::Value::Int(f as u64)));
        }
        if let Some(inner) = &self.inner {
            m.push(("inner".into(), inner.to_value(false)));
        }
        k1::Value::Obj(m)
    }
}

pub fn canon(t: k1::Token) -> Token {
    Token::Canon(t)
}
