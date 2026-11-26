//! §10 closed set of refusal tokens: each refusal is exactly one, fixed by the order of §3.5 and §4.3.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token {
    Utf8,
    Json,
    Number,
    Depth,
    DupKey,
    KeyCharset,
    ValueCharset,
    NotCanonical,
    Envelope,
    EnvelopeMissing,
    EnvelopeClosed,
    Spec,
    EntryType,
    Judge,
    Seq,
    Prev,
    PrevSeq,
    Body,
    SigForm,
    GenesisPlace,
    BodyField,
    SigV,
    SigRange,
    SigHighS,
    SigRecover,
    SigSigner,
}

impl Token {
    /// The spelling in the §10 table, verbatim.
    pub const fn name(self) -> &'static str {
        match self {
            Token::Utf8 => "E_UTF8",
            Token::Json => "E_JSON",
            Token::Number => "E_NUMBER",
            Token::Depth => "E_DEPTH",
            Token::DupKey => "E_DUP_KEY",
            Token::KeyCharset => "E_KEY_CHARSET",
            Token::ValueCharset => "E_VALUE_CHARSET",
            Token::NotCanonical => "E_NOT_CANONICAL",
            Token::Envelope => "E_ENVELOPE",
            Token::EnvelopeMissing => "E_ENVELOPE_MISSING",
            Token::EnvelopeClosed => "E_ENVELOPE_CLOSED",
            Token::Spec => "E_SPEC",
            Token::EntryType => "E_ENTRYTYPE",
            Token::Judge => "E_JUDGE",
            Token::Seq => "E_SEQ",
            Token::Prev => "E_PREV",
            Token::PrevSeq => "E_PREV_SEQ",
            Token::Body => "E_BODY",
            Token::SigForm => "E_SIG_FORM",
            Token::GenesisPlace => "E_GENESIS_PLACE",
            Token::BodyField => "E_BODY_FIELD",
            Token::SigV => "E_SIG_V",
            Token::SigRange => "E_SIG_RANGE",
            Token::SigHighS => "E_SIG_HIGH_S",
            Token::SigRecover => "E_SIG_RECOVER",
            Token::SigSigner => "E_SIG_SIGNER",
        }
    }
}
