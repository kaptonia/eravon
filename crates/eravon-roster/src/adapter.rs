use crate::mark;

pub type Logs = galeed_roster::Logs;
pub type Publication = galeed_roster::Publication;
pub type Status = galeed_roster::Status;
pub type Doc = galeed_roster::Doc;
pub type Kind = galeed_roster::Kind;
pub type JudgeDoc = galeed_roster::JudgeDoc;
pub type Covenant = galeed_roster::Covenant;
pub type GuildCard = galeed_roster::GuildCard;
pub type CovenantReading = galeed_roster::CovenantReading;
pub type CovenantFace = galeed_roster::CovenantFace;
pub type CardReading = galeed_roster::CardReading;
pub const CARD: &str = galeed_roster::CARD;
pub const CARD_NONE: &str = galeed_roster::CARD_NONE;
pub const CARD_UNREAD: &str = galeed_roster::CARD_UNREAD;
pub const CARD_MISMATCH: &str = galeed_roster::CARD_MISMATCH;
pub type JudgeReading = galeed_roster::JudgeReading;
pub type ResolvedJudge = galeed_roster::ResolvedJudge;
pub type QuoteReading = galeed_roster::QuoteReading;
pub type DecodeError = galeed_roster::DecodeError;
pub type Roster = galeed_roster::Roster;
pub type Identity = galeed_roster::Identity;
pub type Guild = galeed_roster::Guild;
pub type Row = galeed_roster::Row;
pub type Refusal = galeed_roster::Refusal;
pub type Candidates = galeed_roster::Candidates;
pub type QuoteKey = galeed_roster::QuoteKey;
pub type QuoteItem = galeed_roster::QuoteItem;
pub type QuoteKeyError = galeed_roster::QuoteKeyError;

pub const KIND_ROSTER: &str = galeed_roster::KIND_ROSTER;
pub const KIND_GUILDCARD: &str = galeed_roster::KIND_GUILDCARD;
pub const KIND_JUDGE: &str = galeed_roster::KIND_JUDGE;
pub const KIND_GUILDAMANAH: &str = galeed_roster::KIND_GUILDAMANAH;

pub mod field {
    pub const PUBLICATIONS: &str = galeed_roster::harness::F_PUBLICATIONS;
    pub const STATUSES: &str = galeed_roster::harness::F_STATUSES;
    pub const DECLARATIONS: &str = galeed_roster::harness::F_DECLARATIONS;
    pub const ACCESSIONS: &str = galeed_roster::harness::F_ACCESSIONS;
    pub const TO: &str = galeed_roster::harness::F_TO;
    pub const HOLDER: &str = galeed_roster::harness::F_HOLDER;
    pub const ROOT: &str = galeed_roster::harness::F_ROOT;
    pub const EMITTER: &str = galeed_roster::harness::F_EMITTER;
    pub const BLOCK_NUMBER: &str = galeed_roster::harness::F_BLOCK_NUMBER;
    pub const LOG_INDEX: &str = galeed_roster::harness::F_LOG_INDEX;
    pub const DOCUMENT: &str = galeed_roster::harness::F_DOCUMENT;
    pub const WORD: &str = galeed_roster::harness::F_WORD;
    pub const ACTIVE: &str = galeed_roster::harness::F_ACTIVE;
}

pub fn decode(p: &Publication) -> Result<Doc, DecodeError> {
    mark::mark(mark::FEATURE);
    galeed_roster::decode(p)
}


pub fn logs_of(v: &eravon_galeed::Value) -> Option<Logs> {
    mark::mark(mark::FEATURE);
    galeed_roster::harness::logs_of(v)
}

pub fn pubs<'a>(logs: &'a Logs, addr: &str) -> Vec<&'a Publication> {
    mark::mark(mark::FEATURE);
    logs.publications_of(addr)
}

pub fn candidate_of<'a>(logs: &'a Logs, addr: &str, k: Kind) -> Option<&'a Publication> {
    mark::mark(mark::FEATURE);
    logs.candidate_of(addr, k)
}

pub fn judge_of(logs: &Logs, addr: &str) -> JudgeReading {
    mark::mark(mark::FEATURE);
    galeed_roster::judge_of(logs, addr)
}

pub fn resolve_judge(logs: &Logs, addr: &str) -> ResolvedJudge {
    mark::mark(mark::FEATURE);
    galeed_roster::resolve_judge(logs, addr)
}

pub fn covenant_of(logs: &Logs, chain: u64, guild: &str) -> CovenantReading {
    mark::mark(mark::FEATURE);
    galeed_roster::covenant_of(logs, chain, guild)
}

pub fn card_answer(r: &CardReading) -> eravon_galeed::Value {
    mark::mark(mark::FEATURE);
    galeed_roster::answer::card(r)
}

pub fn guild_card_of(logs: &Logs, guild: &str) -> CardReading {
    mark::mark(mark::FEATURE);
    galeed_roster::guild_card_of(logs, guild)
}

pub fn quote_of(quotes: &eravon_galeed::Value, chain: u64, contract: &str, code_hash: &str, token: Option<&str>) -> QuoteReading {
    mark::mark(mark::FEATURE);
    galeed_roster::quote_of(quotes, chain, contract, code_hash, token)
}

pub fn resolve_identity(logs: &Logs, addr: &str) -> Identity {
    mark::mark(mark::FEATURE);
    galeed_roster::resolve_identity(logs, addr)
}

pub fn guild_of(logs: &Logs, chain: u64, g: &str) -> Result<Guild, Refusal> {
    mark::mark(mark::FEATURE);
    galeed_roster::guild_of(logs, chain, g)
}

pub fn int_spelling(s: &str) -> Option<u64> {
    mark::mark(mark::FEATURE);
    galeed_roster::int_spelling(s)
}

pub fn pointer(s: &str) -> Option<(u64, &str)> {
    mark::mark(mark::FEATURE);
    galeed_roster::pointer(s)
}

pub fn pointer_string(chain: u64, addr: &str) -> String {
    mark::mark(mark::FEATURE);
    galeed_roster::pointer_string(chain, addr)
}

pub fn quote_spelling(s: &str) -> bool {
    mark::mark(mark::FEATURE);
    galeed_roster::quote_spelling(s)
}

pub fn quote_key(s: &str) -> Result<QuoteKey, QuoteKeyError> {
    mark::mark(mark::FEATURE);
    galeed_roster::quote_key(s)
}

pub fn quote_keys_standard(chain: u64, code_hash: &str, token: &str) -> [String; 2] {
    mark::mark(mark::FEATURE);
    galeed_roster::quote_keys_standard(chain, code_hash, token)
}

pub fn candidates(
    logs: &Logs,
    chain: u64,
    pointers: &[String],
    allow: &[String],
    deny: &[String],
) -> Candidates {
    mark::mark(mark::FEATURE);
    galeed_roster::candidates(logs, chain, pointers, allow, deny)
}

