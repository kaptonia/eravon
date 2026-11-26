//! The `galeed.roster/1` core (the registry law: rosters, guild cards, judge documents, guild covenants, and the
//! succession book).
//!
//! The law's predicates: §1 `candidateOf` by reserved prefix (a publication no reader can place is of every kind),
//! §2.1 decoding in four steps (an unknown kind and decode faults are the document's own answers), the §3 roster
//! table, the §3a judge document table (its `ledger.basis` judged by the `galeed/1` core), the §3b guild covenant
//! table, the §3c guild card table, the §4.2 replay of the succession book, §5.1 publications and standing, §5.2
//! `currentAt`, §5.3 `resolveIdentity` (total: every record is a finite list of keys), §5.4 `resolveGuild`, §5.5 the
//! three bench values, §5.6 the candidate set, §5.7 `judgeOf`, §5.8 `resolveJudge`, §5.9 `covenantOf`, §5.10
//! `quoteOf`, §5.11 `guildCardOf`, and §6 pointers. Inputs are a logs set, a chain id and the query; this crate is
//! pure: no chain reads, no clock, no sorting beyond the byte orders the law names. The harness commands live in
//! [`harness`] and [`answer`].
#![forbid(unsafe_code)]
#![cfg_attr(any(feature = "ablate-k4-decode", feature = "ablate-k4-tables"), allow(dead_code, unused_imports))]

pub mod answer;
pub mod harness;

use galeed::cryptox;
use galeed::hexform;
use galeed::json::{self, Value};
use std::collections::BTreeMap;

/// The four kinds of §2.1 step 4 (registry document-kind role).
pub const KIND_ROSTER: &str = "galeed.roster/1";
pub const KIND_GUILDCARD: &str = "galeed.guildcard/1";
pub const KIND_JUDGE: &str = "galeed.judge/1";
pub const KIND_GUILDAMANAH: &str = "galeed.guildamanah/1";
/// The §6.1 pointer prefix (registry pointer-prefix role).
pub const POINTER_PREFIX: &str = "galeed:guild:";
/// The zero address, which a declaration never binds to (§4.2).
pub const ZERO: &str = "0x0000000000000000000000000000000000000000";
/// §5.2: the greatest position, `(2^53 − 1, 2^53 − 1)`, which every `currentAt` leaves unread and `current` reads.
pub const LAST_POSITION: (u64, u64) = (json::INT_MAX, json::INT_MAX);

/// The five reserved prefixes of §1, one per kind a reader asks [`Logs::candidate_of`] for. `Verifier` is
/// `galeed.papers/1` §7a's kind, read there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Roster,
    Guildcard,
    Judge,
    Guildamanah,
    Verifier,
}

impl Kind {
    /// The closed set, in the order §1 lists the prefixes.
    pub const ALL: [Kind; 5] = [Kind::Roster, Kind::Guildcard, Kind::Judge, Kind::Guildamanah, Kind::Verifier];

    /// The reserved prefix (registry reserved-prefix role).
    pub const fn prefix(self) -> &'static str {
        match self {
            Kind::Roster => "galeed.roster/",
            Kind::Guildcard => "galeed.guildcard/",
            Kind::Judge => "galeed.judge/",
            Kind::Guildamanah => "galeed.guildamanah/",
            Kind::Verifier => "galeed.verifier/",
        }
    }

    /// The short name §1 gives the prefix (HARNESS-ROSTER `<kind>`).
    pub const fn word(self) -> &'static str {
        match self {
            Kind::Roster => "roster",
            Kind::Guildcard => "guildcard",
            Kind::Judge => "judge",
            Kind::Guildamanah => "guildamanah",
            Kind::Verifier => "verifier",
        }
    }

    pub fn of_word(w: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.word() == w)
    }
}

pub mod key {
    pub const KIND: &str = "kind";
    pub const GUILD: &str = "guild";
    pub const CHAIN_ID: &str = "chainId";
    pub const ISSUED: &str = "issued";
    pub const NAME: &str = "name";
    pub const JUDGES: &str = "judges";
    pub const BENCH: &str = "bench";
    pub const VERIFIERS: &str = "verifiers";
    pub const PROTOCOLS: &str = "protocols";
    pub const ADDR: &str = "addr";
    pub const VERSION: &str = "version";
    pub const SHA256: &str = "sha256";
    pub const QUOTES: &str = "quotes";
    pub const TO: &str = "to";
    pub const HOLDER: &str = "holder";
    pub const JUDGE: &str = "judge";
    pub const AMANAH: &str = "amanah";
    pub const ENTRY_ID: &str = "entry_id";
    pub const LEDGER: &str = "ledger";
    pub const ROOT: &str = "root";
    pub const BASIS: &str = "basis";
    pub const HOME: &str = "home";
    pub const NOTE_MD: &str = "note_md";
    pub const TEXT_MD: &str = "text_md";
    pub const VENUES: &str = "venues";
    pub const CONTRACT: &str = "contract";
    pub const CODE_HASH: &str = "codeHash";
}

/// Root keys of the §3.1 roster table.
pub const ROSTER_TABLE: [&str; 6] = [key::KIND, key::GUILD, key::CHAIN_ID, key::ISSUED, key::JUDGES, key::BENCH];
/// Root keys of the §3c.1 guild card table.
const GUILDCARD_TABLE: [&str; 6] = [key::KIND, key::GUILD, key::ISSUED, key::NAME, key::HOME, key::NOTE_MD];
/// Root keys of the §3a.1 judge document table.
const JUDGE_TABLE: [&str; 8] = [key::KIND, key::JUDGE, key::ISSUED, key::AMANAH, key::LEDGER, key::NAME, key::HOME, key::NOTE_MD];
/// Root keys of the §3b.1 guild covenant table.
const GUILDAMANAH_TABLE: [&str; 8] = [key::KIND, key::GUILD, key::CHAIN_ID, key::ISSUED, key::VERSION, key::TEXT_MD, key::VENUES, key::QUOTES];

/// A publication from one `Published` log (§1): `document` is the log's document octets, never text; `word` is
/// the log's word as hex32.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Publication {
    pub emitter: String,
    pub block_number: u64,
    pub log_index: u64,
    pub document: Vec<u8>,
    pub word: String,
}

impl Publication {
    /// A publication of `document` from `emitter` at one log position, its word the document's keccak: how the
    /// registry itself spells one (§1), for callers that hold the bytes they published.
    pub fn of(emitter: &str, block_number: u64, log_index: u64, document: &[u8]) -> Publication {
        Publication { emitter: emitter.to_string(), block_number, log_index, document: document.to_vec(), word: hexform::encode(&cryptox::keccak256(document)) }
    }
}

/// A status from one `StatusChanged` log (§1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub emitter: String,
    pub block_number: u64,
    pub log_index: u64,
    pub active: bool,
}

/// A declaration from one `Declared` log of the succession book (§4.1): `emitter` is the log's `from`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    pub emitter: String,
    pub block_number: u64,
    pub log_index: u64,
    pub to: String,
}

/// An accession from one `Acceded` log of the succession book (§4.1): `emitter` is the log's `to`; `root` is the
/// book's own statement, read for its form and by no step of the replay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Accession {
    pub emitter: String,
    pub block_number: u64,
    pub log_index: u64,
    pub holder: String,
    pub root: String,
}

/// The §1 logs set: the registry's publications and statuses and the succession book's declarations and accessions;
/// only `(blockNumber, logIndex)` carries order; list order carries nothing.
#[derive(Clone, Debug, Default)]
pub struct Logs {
    pub publications: Vec<Publication>,
    pub statuses: Vec<Status>,
    pub declarations: Vec<Declaration>,
    pub accessions: Vec<Accession>,
}

/// The decoding token groups of §8 (§2.1, §3.2, §3a.2, §3b.2, §4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Hash,
    Json,
    Kind,
    RosterGuild,
    RosterChain,
    RosterIssued,
    RosterJudges,
    RosterBench,
    JudgeJudge,
    JudgeIssued,
    JudgeAmanah,
    JudgeLedger,
    JudgeName,
    JudgeHome,
    JudgeNote,
    GuildamanahGuild,
    GuildamanahChain,
    GuildamanahIssued,
    GuildamanahVersion,
    GuildamanahText,
    GuildamanahVenues,
    GuildamanahQuotes,
    GuildcardGuild,
    GuildcardIssued,
    GuildcardName,
    GuildcardHome,
    GuildcardNote,
}

impl DecodeError {
    /// The spelling in §8, verbatim.
    pub const fn name(self) -> &'static str {
        match self {
            DecodeError::Hash => "E_HASH",
            DecodeError::Json => "E_JSON",
            DecodeError::Kind => "E_KIND",
            DecodeError::RosterGuild => "E_ROSTER_GUILD",
            DecodeError::RosterChain => "E_ROSTER_CHAIN",
            DecodeError::RosterIssued => "E_ROSTER_ISSUED",
            DecodeError::RosterJudges => "E_ROSTER_JUDGES",
            DecodeError::RosterBench => "E_ROSTER_BENCH",
            DecodeError::JudgeJudge => "E_JUDGE_JUDGE",
            DecodeError::JudgeIssued => "E_JUDGE_ISSUED",
            DecodeError::JudgeAmanah => "E_JUDGE_AMANAH",
            DecodeError::JudgeLedger => "E_JUDGE_LEDGER",
            DecodeError::JudgeName => "E_JUDGE_NAME",
            DecodeError::JudgeHome => "E_JUDGE_HOME",
            DecodeError::JudgeNote => "E_JUDGE_NOTE",
            DecodeError::GuildamanahGuild => "E_GUILDAMANAH_GUILD",
            DecodeError::GuildamanahChain => "E_GUILDAMANAH_CHAIN",
            DecodeError::GuildamanahIssued => "E_GUILDAMANAH_ISSUED",
            DecodeError::GuildamanahVersion => "E_GUILDAMANAH_VERSION",
            DecodeError::GuildamanahText => "E_GUILDAMANAH_TEXT",
            DecodeError::GuildamanahVenues => "E_GUILDAMANAH_VENUES",
            DecodeError::GuildamanahQuotes => "E_GUILDAMANAH_QUOTES",
            DecodeError::GuildcardGuild => "E_GUILDCARD_GUILD",
            DecodeError::GuildcardIssued => "E_GUILDCARD_ISSUED",
            DecodeError::GuildcardName => "E_GUILDCARD_NAME",
            DecodeError::GuildcardHome => "E_GUILDCARD_HOME",
            DecodeError::GuildcardNote => "E_GUILDCARD_NOTE",
        }
    }
}

/// A §3.1 roster. `bench` is kept as the reader read it (§3.2 already checked its form); see
/// [`Roster::bench_view`] for how the document is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Roster {
    pub guild: String,
    pub chain_id: u64,
    pub issued: u64,
    /// The address of each element (§3.3), in document order, duplicates kept.
    pub judges: Vec<String>,
    pub bench: Option<Value>,
    /// Root keys outside the table, ascending by UTF-8 bytes (§2.2).
    pub unknown_fields: Vec<String>,
    /// The whole document as read (§7: a republisher starts from the document bytes and carries unknown members back).
    pub raw: Value,
}

/// A §3c.1 guild card: who the guild is and how to reach it; it lists nobody and binds no case (§3c.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuildCard {
    pub guild: String,
    pub issued: u64,
    pub name: Option<String>,
    pub home: Option<String>,
    pub note_md: Option<String>,
    pub unknown_fields: Vec<String>,
    pub raw: Value,
}

/// A §3a.1 judge document: the judge's card, which carries no covenant text and no quote (§3a.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JudgeDoc {
    pub judge: String,
    pub issued: u64,
    /// `amanah.entry_id`: the covenant entry the document offers, the word a named seat freezes (§3a.3).
    pub entry_id: String,
    /// `ledger.root`: the root the judge's ledger is audited under.
    pub root: String,
    /// `ledger.basis`, as read: a `galeed/1` §9.4 basis the core accepted.
    pub basis: Value,
    pub name: Option<String>,
    pub home: Option<String>,
    pub note_md: Option<String>,
    pub unknown_fields: Vec<String>,
    pub raw: Value,
}

/// One §3b.1 `venues` element: the three members a reader carries (§2.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Venue {
    pub chain_id: u64,
    pub contract: String,
    pub code_hash: String,
}

/// A §3b.1 guild covenant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Covenant {
    pub guild: String,
    pub chain_id: u64,
    pub issued: u64,
    pub version: u64,
    pub text_md: String,
    /// §3b.3: the `sha256` of `text_md`'s UTF-8, the word a pool seat freezes, computed by the reader.
    pub sha256: String,
    pub venues: Option<Vec<Venue>>,
    /// The `quotes` object as written, every member kept (§2.2).
    pub quotes: Option<Value>,
    pub unknown_fields: Vec<String>,
    pub raw: Value,
}

/// The results of §2.1 step 4; faults are [`DecodeError`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Doc {
    Roster(Roster),
    GuildCard(GuildCard),
    Judge(JudgeDoc),
    Covenant(Covenant),
    /// An unknown kind is still a decode result: the kind comes out as data and nothing else is read.
    Unknown(String),
}

/// How §3.1 `bench` is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bench {
    pub verifiers: Vec<Verifier>,
    pub protocols: Vec<String>,
}

/// One element of `verifiers`: an address string, or a verifier object (with `name` and `protocols`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verifier {
    pub addr: String,
    pub named: Option<(String, Vec<String>)>,
}

fn str_of<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

fn hex20_of(v: &Value, k: &str) -> Option<String> {
    str_of(v, k).filter(|s| hexform::is_hex20(s)).map(str::to_string)
}

fn hex32_of(v: &Value, k: &str) -> Option<String> {
    str_of(v, k).filter(|s| hexform::is_hex32(s)).map(str::to_string)
}

fn hex32_list(v: Option<&Value>) -> Option<Vec<String>> {
    match v? {
        Value::Arr(a) => a.iter().map(|x| x.as_str().filter(|s| hexform::is_hex32(s)).map(str::to_string)).collect(),
        _ => None,
    }
}

fn int_member(v: &Value, k: &str) -> Option<u64> {
    match v.get(k) {
        Some(Value::Int(n)) => Some(*n),
        _ => None,
    }
}

/// An optional member: absent is `Ok(None)`; present (`null` counts as present) must be well formed, or `Err`.
fn optional<T>(v: &Value, k: &str, form: impl Fn(&Value) -> Option<T>, fault: DecodeError) -> Result<Option<T>, DecodeError> {
    match v.get(k) {
        None => Ok(None),
        Some(x) => form(x).map(Some).ok_or(fault),
    }
}

fn string_form(x: &Value) -> Option<String> {
    x.as_str().map(str::to_string)
}

/// The integer spelling of `galeed/1` §3.2 (decimal digits, unsigned, no leading zero, `0` itself
/// allowed, within [0, 2^53 − 1]): the core's reader decides the number; this only adds "every character
/// is a digit", which excludes the whitespace the reader allows around it.
pub fn int_spelling(s: &str) -> Option<u64> {
    if !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    match json::read_open(s.as_bytes()) {
        Some(Value::Int(n)) => Some(n),
        _ => None,
    }
}

/// §3.3: the address of a `judges` element: a string is itself, an object is its `addr`.
fn element_address(e: &Value) -> Option<String> {
    match e {
        Value::Str(s) if hexform::is_hex20(s) => Some(s.clone()),
        Value::Obj(_) => hex20_of(e, key::ADDR),
        _ => None,
    }
}

fn verifier_form(e: &Value) -> Option<Verifier> {
    match e {
        Value::Str(s) if hexform::is_hex20(s) => Some(Verifier { addr: s.clone(), named: None }),
        Value::Obj(_) => {
            let addr = hex20_of(e, key::ADDR)?;
            let name = str_of(e, key::NAME)?.to_string();
            let protocols = hex32_list(e.get(key::PROTOCOLS))?;
            Some(Verifier { addr, named: Some((name, protocols)) })
        }
        _ => None,
    }
}

/// Form of §3.1 `bench`: an object with two required members (others allowed and not tested).
pub fn bench_form(v: &Value) -> Option<Bench> {
    v.as_obj()?;
    let verifiers = match v.get(key::VERIFIERS)? {
        Value::Arr(a) => a.iter().map(verifier_form).collect::<Option<Vec<_>>>()?,
        _ => return None,
    };
    let protocols = hex32_list(v.get(key::PROTOCOLS))?;
    Some(Bench { verifiers, protocols })
}

/// §3a.1 `ledger.basis`: a `galeed/1` §9.4 basis, **decided by the embedded core** (§10.4): the core refuses an
/// audit input of any hex20 root with this `basis` and an empty anchor set (`NO_LABEL`) exactly when the basis
/// fails §9.4's rules. No basis rule is restated here.
pub fn basis_form(basis: &Value) -> bool {
    let input = Value::Obj(vec![
        ("anchors".to_string(), Value::Arr(Vec::new())),
        (key::BASIS.to_string(), basis.clone()),
        (key::ROOT.to_string(), Value::Str(hexform::encode(&[0u8; 20]))),
    ]);
    galeed::audit::read_input(&input).is_some()
}

/// §3b.1 `venues`: an array of objects, each with `chainId` int, `contract` hex20, `codeHash` hex32 (the table of
/// `galeed/1` §6.2), members beyond permitted and untested; carried with the three members alone (§2.2).
fn venues_form(v: &Value) -> Option<Vec<Venue>> {
    match v {
        Value::Arr(a) => a
            .iter()
            .map(|e| {
                e.as_obj()?;
                Some(Venue { chain_id: int_member(e, key::CHAIN_ID)?, contract: hex20_of(e, key::CONTRACT)?, code_hash: hex32_of(e, key::CODE_HASH)? })
            })
            .collect(),
        _ => None,
    }
}

fn unknown_fields(v: &Value, table: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = v.as_obj().unwrap_or_default().iter().map(|(k, _)| k.clone()).filter(|k| !table.contains(&k.as_str())).collect();
    out.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    out
}

/// §2.1 steps 1 and 2: the word is the keccak of the document bytes, and the bytes read to an object.
fn readable(p: &Publication) -> Result<Value, DecodeError> {
    if hexform::encode(&cryptox::keccak256(&p.document)) != p.word {
        return Err(DecodeError::Hash);
    }
    json::read_open(&p.document).filter(|v| v.as_obj().is_some()).ok_or(DecodeError::Json)
}

/// Fault-injection variant for the conformance gauges: step 1 does not compare the word.
#[cfg(feature = "ablate-k4-decode")]
fn readable_unhashed(p: &Publication) -> Result<Value, DecodeError> {
    json::read_open(&p.document).filter(|v| v.as_obj().is_some()).ok_or(DecodeError::Json)
}

/// §2.1 `decode(p)`, in order; the first fault answers.
#[cfg(not(feature = "ablate-k4-decode"))]
pub fn decode(p: &Publication) -> Result<Doc, DecodeError> {
    kind_of(readable(p)?)
}

/// Fault-injection variant for the conformance gauges: step 1 does not compare the word.
#[cfg(feature = "ablate-k4-decode")]
pub fn decode(p: &Publication) -> Result<Doc, DecodeError> {
    kind_of(readable_unhashed(p)?)
}

/// §2.1 steps 3 and 4.
fn kind_of(v: Value) -> Result<Doc, DecodeError> {
    let kind = str_of(&v, key::KIND).ok_or(DecodeError::Kind)?.to_string();
    match kind.as_str() {
        KIND_ROSTER => roster_table(v).map(Doc::Roster),
        KIND_GUILDCARD => card_table(v).map(Doc::GuildCard),
        KIND_JUDGE => judge_table(v).map(Doc::Judge),
        KIND_GUILDAMANAH => covenant_table(v).map(Doc::Covenant),
        _ => Ok(Doc::Unknown(kind)),
    }
}

/// §2.1 steps 1 to 3 over one publication: the decoded `kind` string, or the step's token (`E_HASH`, `E_JSON`,
/// `E_KIND`). The one reading of those three steps: [`of_kind`] and every family law that reads a publication's
/// kind before its own table (`galeed.papers/1` §7a.3) take it from here.
pub fn kind_read(p: &Publication) -> Result<String, DecodeError> {
    let v = readable(p)?;
    str_of(&v, key::KIND).map(str::to_string).ok_or(DecodeError::Kind)
}

/// §1 "of a kind": the decoded `kind` string where §2.1 steps 1 to 3 pass, else `None` (the publication is of
/// every kind, so that bytes no reader can read never let an older document of the kind stand in for them).
fn placed_kind(p: &Publication) -> Option<String> {
    kind_read(p).ok()
}

/// §1: a publication is of kind `k` when it fails §2.1 steps 1 to 3, or its `kind` begins with `k`'s reserved
/// prefix. **The one test**: every reading that takes one current document of a kind selects by it.
#[cfg(not(feature = "ablate-k4-candidate-of"))]
pub fn of_kind(p: &Publication, k: Kind) -> bool {
    placed_kind(p).is_none_or(|s| s.as_bytes().starts_with(k.prefix().as_bytes()))
}

/// Fault-injection variant for the conformance gauges: a publication no reader can place is of no kind, so an
/// older readable document of the kind stands in for the bytes that may have removed somebody.
#[cfg(feature = "ablate-k4-candidate-of")]
pub fn of_kind(p: &Publication, k: Kind) -> bool {
    placed_kind(p).is_some_and(|s| s.as_bytes().starts_with(k.prefix().as_bytes()))
}

/// §3.2 roster table decision order.
fn roster_table(v: Value) -> Result<Roster, DecodeError> {
    let guild = hex20_of(&v, key::GUILD).ok_or(DecodeError::RosterGuild)?;
    let chain_id = int_member(&v, key::CHAIN_ID).ok_or(DecodeError::RosterChain)?;
    let issued = int_member(&v, key::ISSUED).ok_or(DecodeError::RosterIssued)?;
    let judges = match v.get(key::JUDGES) {
        Some(Value::Arr(a)) => a.iter().map(element_address).collect::<Option<Vec<_>>>().ok_or(DecodeError::RosterJudges)?,
        _ => return Err(DecodeError::RosterJudges),
    };
    let bench = bench_member(&v)?;
    let unknown_fields = unknown_fields(&v, &ROSTER_TABLE);
    Ok(Roster { guild, chain_id, issued, judges, bench, unknown_fields, raw: v })
}

/// §3.2 `bench`, where present, in its whole form.
#[cfg(not(feature = "ablate-k4-tables"))]
fn bench_member(v: &Value) -> Result<Option<Value>, DecodeError> {
    optional(v, key::BENCH, |x| bench_form(x).map(|_| x.clone()), DecodeError::RosterBench)
}

/// Fault-injection variant for the conformance gauges: `bench` is taken when present without testing its form.
#[cfg(feature = "ablate-k4-tables")]
fn bench_member(v: &Value) -> Result<Option<Value>, DecodeError> {
    Ok(v.get(key::BENCH).cloned())
}

/// §3c.2 guild card decision order: `guild`, `issued`, then `name`, `home`, `note_md` where present.
fn card_table(v: Value) -> Result<GuildCard, DecodeError> {
    let guild = hex20_of(&v, key::GUILD).ok_or(DecodeError::GuildcardGuild)?;
    let issued = int_member(&v, key::ISSUED).ok_or(DecodeError::GuildcardIssued)?;
    let name = optional(&v, key::NAME, string_form, DecodeError::GuildcardName)?;
    let home = optional(&v, key::HOME, string_form, DecodeError::GuildcardHome)?;
    let note_md = optional(&v, key::NOTE_MD, string_form, DecodeError::GuildcardNote)?;
    let unknown_fields = unknown_fields(&v, &GUILDCARD_TABLE);
    Ok(GuildCard { guild, issued, name, home, note_md, unknown_fields, raw: v })
}

/// §3a.2 judge document decision order: `judge`, `issued`, `amanah`, `ledger` (its `basis` judged by the core),
/// then `name`, `home`, `note_md` where present.
#[cfg(not(feature = "ablate-k4-judge-table"))]
fn judge_table(v: Value) -> Result<JudgeDoc, DecodeError> {
    let judge = hex20_of(&v, key::JUDGE).ok_or(DecodeError::JudgeJudge)?;
    let issued = int_member(&v, key::ISSUED).ok_or(DecodeError::JudgeIssued)?;
    let entry_id = v.get(key::AMANAH).filter(|a| a.as_obj().is_some()).and_then(|a| hex32_of(a, key::ENTRY_ID)).ok_or(DecodeError::JudgeAmanah)?;
    let ledger = v.get(key::LEDGER).filter(|l| l.as_obj().is_some()).ok_or(DecodeError::JudgeLedger)?;
    let root = hex20_of(ledger, key::ROOT).ok_or(DecodeError::JudgeLedger)?;
    let basis = ledger.get(key::BASIS).filter(|b| basis_form(b)).ok_or(DecodeError::JudgeLedger)?.clone();
    judge_rest(v, judge, issued, entry_id, root, basis)
}

/// Fault-injection variant for the conformance gauges: `ledger.basis` is taken whatever its form, so a judge
/// document whose basis no reader can scan under passes as a card.
#[cfg(feature = "ablate-k4-judge-table")]
fn judge_table(v: Value) -> Result<JudgeDoc, DecodeError> {
    let judge = hex20_of(&v, key::JUDGE).ok_or(DecodeError::JudgeJudge)?;
    let issued = int_member(&v, key::ISSUED).ok_or(DecodeError::JudgeIssued)?;
    let entry_id = v.get(key::AMANAH).filter(|a| a.as_obj().is_some()).and_then(|a| hex32_of(a, key::ENTRY_ID)).ok_or(DecodeError::JudgeAmanah)?;
    let ledger = v.get(key::LEDGER).filter(|l| l.as_obj().is_some()).ok_or(DecodeError::JudgeLedger)?;
    let root = hex20_of(ledger, key::ROOT).ok_or(DecodeError::JudgeLedger)?;
    let basis = ledger.get(key::BASIS).ok_or(DecodeError::JudgeLedger)?.clone();
    judge_rest(v, judge, issued, entry_id, root, basis)
}

fn judge_rest(v: Value, judge: String, issued: u64, entry_id: String, root: String, basis: Value) -> Result<JudgeDoc, DecodeError> {
    let name = optional(&v, key::NAME, string_form, DecodeError::JudgeName)?;
    let home = optional(&v, key::HOME, string_form, DecodeError::JudgeHome)?;
    let note_md = optional(&v, key::NOTE_MD, string_form, DecodeError::JudgeNote)?;
    let unknown_fields = unknown_fields(&v, &JUDGE_TABLE);
    Ok(JudgeDoc { judge, issued, entry_id, root, basis, name, home, note_md, unknown_fields, raw: v })
}

/// §3b.2 guild covenant decision order.
fn covenant_table(v: Value) -> Result<Covenant, DecodeError> {
    let guild = hex20_of(&v, key::GUILD).ok_or(DecodeError::GuildamanahGuild)?;
    let chain_id = int_member(&v, key::CHAIN_ID).ok_or(DecodeError::GuildamanahChain)?;
    let issued = int_member(&v, key::ISSUED).ok_or(DecodeError::GuildamanahIssued)?;
    let version = int_member(&v, key::VERSION).ok_or(DecodeError::GuildamanahVersion)?;
    let text_md = str_of(&v, key::TEXT_MD).ok_or(DecodeError::GuildamanahText)?.to_string();
    let venues = optional(&v, key::VENUES, venues_form, DecodeError::GuildamanahVenues)?;
    let quotes = optional(&v, key::QUOTES, |x| x.as_obj().map(|_| x.clone()), DecodeError::GuildamanahQuotes)?;
    let sha256 = text_word(&text_md);
    let unknown_fields = unknown_fields(&v, &GUILDAMANAH_TABLE);
    Ok(Covenant { guild, chain_id, issued, version, text_md, sha256, venues, quotes, unknown_fields, raw: v })
}

/// §3b.3: the word a pool seat freezes, the `sha256` of the UTF-8 of a covenant text's scalar values, as hex32.
/// **The reader computes it** from the text (`galeed.roster/1` §3b.3, `galeed.venue/1` §2.4): a member's own copy of
/// the text resolves to the same word in its ledger, whatever its writer stored beside the text.
pub fn text_word(text_md: &str) -> String {
    hexform::encode(&cryptox::sha256(text_md.as_bytes()))
}

impl Roster {
    /// How `bench` is read; `None` when absent. Its form was checked by §3.2.
    pub fn bench_view(&self) -> Option<Bench> {
        self.bench.as_ref().and_then(bench_form)
    }
}

impl Logs {
    /// §5.1 `pubs(a)`: publications whose emitter equals `a` byte for byte, ascending by `(blockNumber, logIndex)`.
    pub fn publications_of(&self, addr: &str) -> Vec<&Publication> {
        let mut v: Vec<&Publication> = self.publications.iter().filter(|p| p.emitter == addr).collect();
        v.sort_by_key(|p| (p.block_number, p.log_index));
        v
    }

    /// §5.1 `active(a)`: the status with the largest `(blockNumber, logIndex)`; no status means `true`. A later
    /// `true` after retiring reactivates.
    pub fn active(&self, addr: &str) -> bool {
        self.statuses.iter().filter(|s| s.emitter == addr).max_by_key(|s| (s.block_number, s.log_index)).is_none_or(|s| s.active)
    }

    /// §1 `candidateOf(a, K)`: the last publication of `a`, by `(blockNumber, logIndex)`, that is of kind `K`. One
    /// address may carry several kinds and each reader reads its own; a publication that cannot be placed is every
    /// kind's candidate.
    pub fn candidate_of(&self, addr: &str, k: Kind) -> Option<&Publication> {
        self.publications_of(addr).into_iter().rev().find(|p| of_kind(p, k))
    }
}

/// One §5.3 attempt: a declaration of a key of the record from the address on, bound or not, that no counting
/// accession took up, as `(blockNumber, logIndex, to)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attempt {
    pub block_number: u64,
    pub log_index: u64,
    pub to: String,
}

/// §4.2 the replay of the succession book: for each address its root, for each root its head and its keys in the
/// order they acceded, each address's pending declaration, which addresses have entered the book, and which
/// declarations a counting accession took up. **The one replay**: every reading of the book in the family
/// (`current`, `currentAt`, `resolveIdentity`, the keys of a record) is a question to it.
#[derive(Clone, Debug, Default)]
pub struct Book {
    root_of: BTreeMap<String, String>,
    head: BTreeMap<String, String>,
    /// root -> its keys: the root first, then each key in the order it acceded.
    keys: BTreeMap<String, Vec<String>>,
    pending: BTreeMap<String, Declaration>,
    entered: std::collections::BTreeSet<String>,
    taken: std::collections::BTreeSet<(u64, u64)>,
    /// Every declaration of the set, in replay order.
    declarations: Vec<Declaration>,
    /// The records that bound or counted nothing, in replay order (§4.2 last paragraph: a reader holding every record
    /// of a book before such a one is reading no book of this law).
    idle: Vec<(u64, u64)>,
}

impl Book {
    /// The replay of every declaration and accession of `logs`.
    pub fn of(logs: &Logs) -> Book {
        Book::before(logs, LAST_POSITION, true)
    }

    /// The replay of the records strictly before `p` (§5.2), or up to and including it where `inclusive`.
    fn before(logs: &Logs, p: (u64, u64), inclusive: bool) -> Book {
        let at = |bn: u64, li: u64| if inclusive { (bn, li) <= p } else { (bn, li) < p };
        let mut recs: Vec<(u64, u64, Result<&Declaration, &Accession>)> = logs
            .declarations
            .iter()
            .filter(|d| at(d.block_number, d.log_index))
            .map(|d| (d.block_number, d.log_index, Ok(d)))
            .chain(logs.accessions.iter().filter(|x| at(x.block_number, x.log_index)).map(|x| (x.block_number, x.log_index, Err(x))))
            .collect();
        recs.sort_by_key(|r| (r.0, r.1));
        let mut book = Book::default();
        for (bn, li, r) in recs {
            let moved = match r {
                Ok(d) => {
                    book.declarations.push(d.clone());
                    book.bind(d)
                }
                Err(x) => book.count(x),
            };
            if !moved {
                book.idle.push((bn, li));
            }
        }
        book
    }

    /// §4.2: a declaration binds iff its emitter is the current key of its own record and its `to` is not the zero
    /// address, not the emitter, and has not entered the book.
    fn bind(&mut self, d: &Declaration) -> bool {
        let e = d.emitter.as_str();
        if self.current(e) == e && d.to != ZERO && d.to != e && !self.entered.contains(&d.to) {
            self.pending.insert(e.to_string(), d.clone());
            self.entered.insert(e.to_string());
            return true;
        }
        false
    }

    /// §4.2: an accession counts iff its emitter has not entered the book, is the pending key of `holder`, and
    /// `holder` is the current key of its own record.
    fn count(&mut self, x: &Accession) -> bool {
        let (e, h) = (x.emitter.as_str(), x.holder.as_str());
        if !self.entered.contains(e) && self.pending_is(h, e) && self.current(h) == h {
            let r = self.root(h);
            self.root_of.insert(e.to_string(), r.clone());
            self.head.insert(r.clone(), e.to_string());
            self.keys.entry(r.clone()).or_insert_with(|| vec![r]).push(e.to_string());
            if let Some(d) = self.pending.remove(h) {
                self.taken.insert((d.block_number, d.log_index));
            }
            self.entered.insert(e.to_string());
            return true;
        }
        false
    }

    /// Whether `e` is `holder`'s pending key.
    #[cfg(not(feature = "ablate-k4-identity"))]
    fn pending_is(&self, holder: &str, e: &str) -> bool {
        self.pending.get(holder).is_some_and(|d| d.to == e)
    }

    /// Fault-injection variant for the conformance gauges: any key that has not entered takes up a record whose holder
    /// declared at all, so a key the holder never named takes the record.
    #[cfg(feature = "ablate-k4-identity")]
    fn pending_is(&self, holder: &str, e: &str) -> bool {
        let _ = e;
        self.pending.contains_key(holder)
    }

    /// The root of `a`'s record.
    pub fn root(&self, a: &str) -> String {
        self.root_of.get(a).cloned().unwrap_or_else(|| a.to_string())
    }

    /// The current key of `a`'s record: the head of its root.
    pub fn current(&self, a: &str) -> String {
        let r = self.root(a);
        self.head.get(&r).cloned().unwrap_or(r)
    }

    /// §5.3 the keys of `a`'s record: its root first, then each key in the order it acceded.
    pub fn record_keys(&self, a: &str) -> Vec<String> {
        let r = self.root(a);
        self.keys.get(&r).cloned().unwrap_or_else(|| vec![r])
    }

    /// The key `a` has declared and that has not yet taken the record up (the pending of §4.2), with the declaration's
    /// position; `None` where `a` has none.
    pub fn pending(&self, a: &str) -> Option<&Declaration> {
        self.pending.get(a)
    }

    /// Whether `a` has entered the book (declared a binding declaration or acceded).
    pub fn entered(&self, a: &str) -> bool {
        self.entered.contains(a)
    }

    /// The records of the set that bound or counted nothing, as positions (§4.2 last paragraph).
    pub fn idle(&self) -> &[(u64, u64)] {
        &self.idle
    }

    /// §5.3 `resolveIdentity(a)`.
    pub fn identity(&self, a: &str) -> Identity {
        let final_addr = self.current(a);
        let keys = self.record_keys(a);
        let from = keys.iter().position(|k| k == a).unwrap_or(0);
        let to = keys.iter().position(|k| *k == final_addr).unwrap_or(from);
        let trail: Vec<String> = keys.get(from..to.max(from)).unwrap_or_default().to_vec();
        let attempts = self
            .declarations
            .iter()
            .filter(|d| (trail.contains(&d.emitter) || d.emitter == final_addr) && !self.taken.contains(&(d.block_number, d.log_index)))
            .map(|d| Attempt { block_number: d.block_number, log_index: d.log_index, to: d.to.clone() })
            .collect();
        Identity { final_addr, trail, attempts }
    }
}

/// §5.2 `currentAt(a, p)`: the current key of `a`'s record by the replay of the records strictly before `p`.
#[cfg(not(feature = "ablate-k4-current-at"))]
pub fn current_at(logs: &Logs, a: &str, p: (u64, u64)) -> String {
    Book::before(logs, p, false).current(a)
}

/// Fault-injection variant for the conformance gauges: the replay takes the record at `p` itself as well.
#[cfg(feature = "ablate-k4-current-at")]
pub fn current_at(logs: &Logs, a: &str, p: (u64, u64)) -> String {
    Book::before(logs, p, true).current(a)
}

/// §5.2 `current(a)`: the current key after every record of the set.
pub fn current(logs: &Logs, a: &str) -> String {
    Book::of(logs).current(a)
}

/// §5.3 the keys of `a`'s record, root first.
pub fn record_keys(logs: &Logs, a: &str) -> Vec<String> {
    Book::of(logs).record_keys(a)
}

/// §5.3 `resolveIdentity`: the final address, the keys from the address up to and excluding the final one, and the
/// attempts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub final_addr: String,
    pub trail: Vec<String>,
    pub attempts: Vec<Attempt>,
}

/// §5.3 `resolveIdentity(a)`: total; the book admits no return and no merger, so every record is a finite list.
pub fn resolve_identity(logs: &Logs, addr: &str) -> Identity {
    Book::of(logs).identity(addr)
}

/// An address as every reader of the family shows it after following its record (§9 "What a trail means"): the
/// address it was listed or handed under, the current key of its record, and every key walked from the listed address
/// to it. **The one reading** every face displays, so that an address that moved reads as continuing elsewhere and
/// never silently becomes another party.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Followed {
    pub listed: String,
    pub final_addr: String,
    pub trail: Vec<String>,
}

impl Followed {
    /// Whether the address moved (its record continues elsewhere).
    pub fn moved(&self) -> bool {
        !self.trail.is_empty()
    }
    /// Every address of the record from the listed one on: the trail and the final address.
    pub fn addresses(&self) -> Vec<String> {
        let mut out = self.trail.clone();
        out.push(self.final_addr.clone());
        out
    }
}

/// [`resolve_identity`] read as a [`Followed`].
pub fn follow(logs: &Logs, addr: &str) -> Followed {
    followed(addr, &resolve_identity(logs, addr))
}

/// One resolution as a [`Followed`].
pub fn followed(listed: &str, id: &Identity) -> Followed {
    Followed { listed: listed.to_string(), final_addr: id.final_addr.clone(), trail: id.trail.clone() }
}

/// One row of §5.4 steps 6 and 7: the listed address, its resolution, the keys of its record, and a verifier
/// object's `name` and `protocols`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub listed: String,
    pub walk: Identity,
    /// §5.3 the keys of the listed address's record, root first (the bench and `deny` read every one of them).
    pub keys: Vec<String>,
    pub named: Option<(String, Vec<String>)>,
}

impl Row {
    pub fn followed(&self) -> Followed {
        followed(&self.listed, &self.walk)
    }
}

/// A guild as §5.4 resolves it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Guild {
    pub guild: String,
    pub trail: Vec<String>,
    pub judges: Vec<Row>,
    pub verifiers: Vec<Row>,
    /// Step 7: every key of the record of every verifier row's listed address, ascending by bytes, no duplicates.
    pub bench: Vec<String>,
    pub protocols: Vec<String>,
    /// Step 8: the roster's `issued` as written, a datum for display and never a predicate.
    pub issued: u64,
    /// Step 8: `guildCardOf(G)`, present in every guild answer.
    pub card: CardReading,
    /// Step 8: `covenantOf(G)`, present in every guild answer.
    pub covenant: CovenantReading,
    pub roster: Roster,
}

impl Guild {
    /// The guild's current covenant where §5.9 reads one (`None` where the guild published none or the reading
    /// stopped; the stop is in [`Guild::covenant`]).
    pub fn current_covenant(&self) -> Option<&CovenantFace> {
        match &self.covenant {
            CovenantReading::Covenant(c) => Some(c),
            _ => None,
        }
    }
}

/// Refusals of §5.4 steps 2 to 5.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    Retired,
    NoDocument,
    Decode(DecodeError),
    /// Another version of the kind read (the kind string carried).
    UnknownKind(String),
    GuildMismatch,
    ChainMismatch,
}

pub const E_UNKNOWN_KIND: &str = "E_UNKNOWN_KIND";

impl Refusal {
    /// The spelling in §8.
    pub fn name(&self) -> &'static str {
        match self {
            Refusal::Retired => "E_RETIRED",
            Refusal::NoDocument => "E_NO_DOCUMENT",
            Refusal::Decode(d) => d.name(),
            Refusal::UnknownKind(_) => E_UNKNOWN_KIND,
            Refusal::GuildMismatch => "E_GUILD_MISMATCH",
            Refusal::ChainMismatch => "E_CHAIN_MISMATCH",
        }
    }
}

/// §5.4 steps 3 and 4: the roster candidate, decoded; a kind under the roster prefix that is not this version is
/// `E_UNKNOWN_KIND`. A publication of another kind never reaches here (§1).
fn roster_at(logs: &Logs, at: &str) -> Result<Roster, Refusal> {
    let p = logs.candidate_of(at, Kind::Roster).ok_or(Refusal::NoDocument)?;
    match decode(p).map_err(Refusal::Decode)? {
        Doc::Roster(r) => Ok(r),
        Doc::Unknown(k) => Err(Refusal::UnknownKind(k)),
        // A decoded document of another known kind is not of the roster kind; `candidate_of` never hands one over.
        Doc::GuildCard(_) | Doc::Judge(_) | Doc::Covenant(_) => Err(Refusal::NoDocument),
    }
}

/// §5.4 step 2: a guild whose latest status carries `false` is retired (§5.1), at its final address.
#[cfg(not(feature = "ablate-k4-retired"))]
fn retired(logs: &Logs, at: &str) -> bool {
    !logs.active(at)
}

/// Fault-injection variant for the conformance gauges: retirement is not read, so a panel is engaged from a guild that
/// retired and its parties are handed to rules no one keeps.
#[cfg(feature = "ablate-k4-retired")]
fn retired(logs: &Logs, at: &str) -> bool {
    let _ = (logs, at);
    false
}

/// §5.4 step 5, second half: the roster's `chainId` equals the chain the set was scanned on.
#[cfg(not(feature = "ablate-k4-guild"))]
fn other_chain(roster: &Roster, chain: u64) -> bool {
    roster.chain_id != chain
}

/// Fault-injection variant for the conformance gauges: the roster's chain is not compared, so a roster written for
/// another chain lists judges here.
#[cfg(feature = "ablate-k4-guild")]
fn other_chain(roster: &Roster, chain: u64) -> bool {
    let _ = (roster, chain);
    false
}

/// §5.4 `resolveGuild(g)`, in order.
pub fn guild_of(logs: &Logs, chain: u64, g: &str) -> Result<Guild, Refusal> {
    guild_in(logs, &Book::of(logs), chain, g)
}

fn guild_in(logs: &Logs, book: &Book, chain: u64, g: &str) -> Result<Guild, Refusal> {
    let id = book.identity(g);
    let at = id.final_addr;
    if retired(logs, &at) {
        return Err(Refusal::Retired);
    }
    let roster = roster_at(logs, &at)?;
    if roster.guild != at {
        return Err(Refusal::GuildMismatch);
    }
    if other_chain(&roster, chain) {
        return Err(Refusal::ChainMismatch);
    }
    Ok(rows_of(logs, book, chain, at, id.trail, roster))
}

/// §5.4 steps 6 to 8: judge rows, verifier rows, the bench set, protocols, issued, card, covenant.
fn rows_of(logs: &Logs, book: &Book, chain: u64, guild: String, trail: Vec<String>, roster: Roster) -> Guild {
    let row = |listed: &str, named: Option<(String, Vec<String>)>| Row { listed: listed.to_string(), walk: book.identity(listed), keys: book.record_keys(listed), named };
    let judges: Vec<Row> = roster.judges.iter().map(|listed| row(listed, None)).collect();
    let view = roster.bench_view();
    let verifiers: Vec<Row> = view.as_ref().map(|b| b.verifiers.iter().map(|v| row(&v.addr, v.named.clone())).collect()).unwrap_or_default();
    let bench = bytewise_set(verifiers.iter().flat_map(|r| bench_keys(r).into_iter()));
    let protocols = view.map(|b| b.protocols).unwrap_or_default();
    let card = guild_card_of(logs, &guild);
    let covenant = covenant_of(logs, chain, &guild);
    Guild { guild, trail, judges, verifiers, bench, protocols, issued: roster.issued, card, covenant, roster }
}

/// §5.4 step 7 and §5.5's array row: the keys of a listed address's record a bench holds, the keys before the listed
/// address included.
#[cfg(not(feature = "ablate-k4-bench"))]
fn bench_keys(r: &Row) -> Vec<String> {
    r.keys.clone()
}

/// Fault-injection variant for the conformance gauges: only the listed address and the keys after it, so a verifier
/// the guild re-lists under its new key loses the reports its earlier key signed.
#[cfg(feature = "ablate-k4-bench")]
fn bench_keys(r: &Row) -> Vec<String> {
    followed(&r.listed, &r.walk).addresses()
}

/// Ascending by UTF-8 bytes, no duplicates.
fn bytewise_set(addrs: impl Iterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = addrs.collect();
    out.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    out.dedup();
    out
}

/// §5.9 a covenant reading: the current covenant's members a reader carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CovenantFace {
    pub word: String,
    pub block_number: u64,
    pub version: u64,
    pub sha256: String,
    pub venues: Option<Vec<Venue>>,
    pub quotes: Option<Value>,
    /// The decoded covenant (its text included), for readers that show or reconcile it.
    pub covenant: Covenant,
}

/// §5.9's three answers: no candidate, a stop with its token, or the covenant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CovenantReading {
    None,
    Stop(&'static str),
    Covenant(Box<CovenantFace>),
}

pub const E_GUILDAMANAH_GUILD_MISMATCH: &str = "E_GUILDAMANAH_GUILD_MISMATCH";
pub const E_GUILDAMANAH_CHAIN_MISMATCH: &str = "E_GUILDAMANAH_CHAIN_MISMATCH";

/// §5.9 `covenantOf(G)`: `G` a final address; follows no record.
#[cfg(not(feature = "ablate-k4-covenant"))]
pub fn covenant_of(logs: &Logs, chain: u64, g: &str) -> CovenantReading {
    let Some(p) = logs.candidate_of(g, Kind::Guildamanah) else { return CovenantReading::None };
    let c = match decode(p) {
        Err(e) => return CovenantReading::Stop(e.name()),
        Ok(Doc::Covenant(c)) => c,
        Ok(_) => return CovenantReading::Stop(E_UNKNOWN_KIND),
    };
    if c.guild != g {
        return CovenantReading::Stop(E_GUILDAMANAH_GUILD_MISMATCH);
    }
    if c.chain_id != chain {
        return CovenantReading::Stop(E_GUILDAMANAH_CHAIN_MISMATCH);
    }
    covenant_face(p, c)
}

/// Fault-injection variant for the conformance gauges: the covenant's `guild` is not compared with the address it
/// was published from, so a covenant naming another guild prices this guild's pool.
#[cfg(feature = "ablate-k4-covenant")]
pub fn covenant_of(logs: &Logs, chain: u64, g: &str) -> CovenantReading {
    let Some(p) = logs.candidate_of(g, Kind::Guildamanah) else { return CovenantReading::None };
    let c = match decode(p) {
        Err(e) => return CovenantReading::Stop(e.name()),
        Ok(Doc::Covenant(c)) => c,
        Ok(_) => return CovenantReading::Stop(E_UNKNOWN_KIND),
    };
    if c.chain_id != chain {
        return CovenantReading::Stop(E_GUILDAMANAH_CHAIN_MISMATCH);
    }
    covenant_face(p, c)
}

fn covenant_face(p: &Publication, c: Covenant) -> CovenantReading {
    CovenantReading::Covenant(Box::new(CovenantFace { word: p.word.clone(), block_number: p.block_number, version: c.version, sha256: c.sha256.clone(), venues: c.venues.clone(), quotes: c.quotes.clone(), covenant: c }))
}

/// §5.11's four answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CardReading {
    None,
    /// The candidate does not read: a token of §2.1 or §3c.2, or `E_UNKNOWN_KIND`.
    Unread(&'static str),
    Mismatch,
    Card { word: String, block_number: u64, document: Box<GuildCard> },
}

pub const CARD: &str = "CARD";
pub const CARD_NONE: &str = "CARD_NONE";
pub const CARD_UNREAD: &str = "CARD_UNREAD";
pub const CARD_MISMATCH: &str = "CARD_MISMATCH";

impl CardReading {
    pub const fn name(&self) -> &'static str {
        match self {
            CardReading::None => CARD_NONE,
            CardReading::Unread(_) => CARD_UNREAD,
            CardReading::Mismatch => CARD_MISMATCH,
            CardReading::Card { .. } => CARD,
        }
    }
}

/// §5.11 `guildCardOf(G)`: `G` a final address; follows no record of the book.
#[cfg(not(feature = "ablate-k4-guild-card"))]
pub fn guild_card_of(logs: &Logs, g: &str) -> CardReading {
    let Some(p) = logs.candidate_of(g, Kind::Guildcard) else { return CardReading::None };
    let c = match decode(p) {
        Err(e) => return CardReading::Unread(e.name()),
        Ok(Doc::GuildCard(c)) => c,
        Ok(_) => return CardReading::Unread(E_UNKNOWN_KIND),
    };
    if c.guild != g {
        return CardReading::Mismatch;
    }
    CardReading::Card { word: p.word.clone(), block_number: p.block_number, document: Box::new(c) }
}

/// Fault-injection variant for the conformance gauges: a card is taken whatever guild it names.
#[cfg(feature = "ablate-k4-guild-card")]
pub fn guild_card_of(logs: &Logs, g: &str) -> CardReading {
    let Some(p) = logs.candidate_of(g, Kind::Guildcard) else { return CardReading::None };
    match decode(p) {
        Err(e) => CardReading::Unread(e.name()),
        Ok(Doc::GuildCard(c)) => CardReading::Card { word: p.word.clone(), block_number: p.block_number, document: Box::new(c) },
        Ok(_) => CardReading::Unread(E_UNKNOWN_KIND),
    }
}

/// §5.7's four answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JudgeReading {
    None,
    /// The candidate does not read: a token of §2.1 or §3a.2, or `E_UNKNOWN_KIND`.
    Unread(&'static str),
    Mismatch,
    Judge { word: String, block_number: u64, document: Box<JudgeDoc> },
}

pub const JUDGE: &str = "JUDGE";
pub const JUDGE_NONE: &str = "JUDGE_NONE";
pub const JUDGE_UNREAD: &str = "JUDGE_UNREAD";
pub const JUDGE_MISMATCH: &str = "JUDGE_MISMATCH";

impl JudgeReading {
    pub const fn name(&self) -> &'static str {
        match self {
            JudgeReading::None => JUDGE_NONE,
            JudgeReading::Unread(_) => JUDGE_UNREAD,
            JudgeReading::Mismatch => JUDGE_MISMATCH,
            JudgeReading::Judge { .. } => JUDGE,
        }
    }
}

/// §5.7 `judgeOf(a)`: the address it is given, following no record.
pub fn judge_of(logs: &Logs, a: &str) -> JudgeReading {
    let Some(p) = logs.candidate_of(a, Kind::Judge) else { return JudgeReading::None };
    let d = match decode(p) {
        Err(e) => return JudgeReading::Unread(e.name()),
        Ok(Doc::Judge(d)) => d,
        Ok(_) => return JudgeReading::Unread(E_UNKNOWN_KIND),
    };
    if d.judge != a {
        return JudgeReading::Mismatch;
    }
    JudgeReading::Judge { word: p.word.clone(), block_number: p.block_number, document: Box::new(d) }
}

/// §5.8 a resolved judge: the final address, the trail, standing at the final address, and its judge reading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedJudge {
    pub final_addr: String,
    pub trail: Vec<String>,
    pub active: bool,
    pub judge: JudgeReading,
}

/// §5.8 `resolveJudge(a)`: decided from `a` and the logs set.
pub fn resolve_judge(logs: &Logs, a: &str) -> ResolvedJudge {
    let id = resolve_identity(logs, a);
    let judge = judge_of(logs, &id.final_addr);
    ResolvedJudge { active: logs.active(&id.final_addr), final_addr: id.final_addr, trail: id.trail, judge }
}

/// §5.10's three answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuoteReading {
    /// The first key the order reached and its value, a quote spelling.
    Quote { key: String, quote: String },
    /// The first key the order reached carries a value that is not a quote spelling.
    Form { key: String },
    None,
}

pub const QUOTE: &str = "QUOTE";
pub const QUOTE_NONE: &str = "QUOTE_NONE";
pub const E_QUOTE_FORM: &str = "E_QUOTE_FORM";

impl QuoteReading {
    pub const fn name(&self) -> &'static str {
        match self {
            QuoteReading::Quote { .. } => QUOTE,
            QuoteReading::Form { .. } => E_QUOTE_FORM,
            QuoteReading::None => QUOTE_NONE,
        }
    }
}

/// §5.10 quote spelling: lowercase `0x` plus an even number of lowercase hex digits; `0x` alone is the empty byte
/// string, a quote present.
pub fn quote_spelling(s: &str) -> bool {
    s.strip_prefix("0x").is_some_and(|d| d.len() % 2 == 0 && d.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
}

/// §5.10 the order the keys are tried in, a key naming a token skipped where `token` is `None`: address before
/// code, a token-carrying key before the one without, the bare chain key last.
#[cfg(not(feature = "ablate-k4-quote"))]
pub fn quote_order(chain_id: u64, contract: &str, code_hash: &str, token: Option<&str>) -> Vec<String> {
    let mut keys = Vec::with_capacity(5);
    if let Some(t) = token {
        keys.push(format!("{chain_id}:{contract}:{t}"));
    }
    keys.push(format!("{chain_id}:{contract}"));
    if let Some(t) = token {
        keys.push(format!("{chain_id}:{code_hash}:{t}"));
    }
    keys.push(format!("{chain_id}:{code_hash}"));
    keys.push(chain_id.to_string());
    keys
}

/// Fault-injection variant for the conformance gauges: the first two forms trade places, so where a pool quotes an
/// address both with the token and without, the tokenless quote is read and the token's is never reached.
#[cfg(feature = "ablate-k4-quote")]
pub fn quote_order(chain_id: u64, contract: &str, code_hash: &str, token: Option<&str>) -> Vec<String> {
    let mut keys = Vec::with_capacity(5);
    keys.push(format!("{chain_id}:{contract}"));
    if let Some(t) = token {
        keys.push(format!("{chain_id}:{contract}:{t}"));
        keys.push(format!("{chain_id}:{code_hash}:{t}"));
    }
    keys.push(format!("{chain_id}:{code_hash}"));
    keys.push(chain_id.to_string());
    keys
}

/// §5.10 `quoteOf(Q, chainId, contract, codeHash, token)`: one function reads a judge's quotes and a pool's alike.
/// `q` is the `quotes` object (a carrier without one hands the empty object); keys the order never reaches are data.
pub fn quote_of(q: &Value, chain_id: u64, contract: &str, code_hash: &str, token: Option<&str>) -> QuoteReading {
    for k in quote_order(chain_id, contract, code_hash, token) {
        if let Some(v) = q.get(&k) {
            return match v.as_str().filter(|s| quote_spelling(s)) {
                Some(s) => QuoteReading::Quote { key: k, quote: s.to_string() },
                None => QuoteReading::Form { key: k },
            };
        }
    }
    QuoteReading::None
}

/// §6.1: `galeed:guild:` plus an integer spelling plus `:` plus hex20; answers `(chainId, address)`.
pub fn pointer(s: &str) -> Option<(u64, &str)> {
    let (chain, addr) = s.strip_prefix(POINTER_PREFIX)?.split_once(':')?;
    Some((int_spelling(chain)?, addr)).filter(|_| hexform::is_hex20(addr))
}

pub fn pointer_string(chain: u64, addr: &str) -> String {
    format!("{POINTER_PREFIX}{chain}:{addr}")
}

/// A venue key of a declared quote table read into its parts: chain id, item (address item or standard item), and
/// token (three-segment keys only).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuoteKey {
    pub chain: u64,
    pub item: QuoteItem,
    pub token: Option<String>,
}

/// The two item forms: a hex20 middle segment names by address, hex32 by code; the lengths keep them apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuoteItem {
    Address(String),
    Standard(String),
}

/// Why a key does not read (one name per reason, no guessing).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuoteKeyError {
    /// Not two or three segments, or an empty segment.
    Parts,
    /// The first segment is not a `galeed/1` §3.2 integer spelling.
    Chain,
    /// The middle segment is neither form (hex20 and hex32 are lowercase only).
    Item,
    /// The last segment is present but not hex20 (the settlement token is an ERC-20 address; hex32 is out of form).
    Token,
}

impl QuoteKeyError {
    pub fn name(&self) -> &'static str {
        match self {
            QuoteKeyError::Parts => "E_QUOTE_KEY_PARTS",
            QuoteKeyError::Chain => "E_QUOTE_KEY_CHAIN",
            QuoteKeyError::Item => "E_QUOTE_KEY_ITEM",
            QuoteKeyError::Token => "E_QUOTE_KEY_TOKEN",
        }
    }
}

/// The item-carrying venue keys of a declared quote table (§5.10's four forms with a middle segment): two or three
/// segments joined by `:`, the first an integer chain id, the middle hex20 (address item) or hex32 (standard item),
/// the last, when present, hex20 (settlement token). Keys are read byte for byte; uppercase spellings are not keys.
pub fn quote_key(s: &str) -> Result<QuoteKey, QuoteKeyError> {
    let parts: Vec<&str> = s.split(':').collect();
    if !(parts.len() == 2 || parts.len() == 3) || parts.iter().any(|p| p.is_empty()) {
        return Err(QuoteKeyError::Parts);
    }
    let chain = int_spelling(parts[0]).ok_or(QuoteKeyError::Chain)?;
    let item = if hexform::is_hex20(parts[1]) {
        QuoteItem::Address(parts[1].to_string())
    } else if hexform::is_hex32(parts[1]) {
        QuoteItem::Standard(parts[1].to_string())
    } else {
        return Err(QuoteKeyError::Item);
    };
    let token = match parts.get(2) {
        None => None,
        Some(t) if hexform::is_hex20(t) => Some((*t).to_string()),
        Some(_) => return Err(QuoteKeyError::Token),
    };
    Ok(QuoteKey { chain, item, token })
}

/// A §5.10 venue key read whole: the bare chain key `<chainId>`, which quotes every venue on its chain, or one of
/// the four item-carrying forms ([`quote_key`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VenueKey {
    Chain(u64),
    Item(QuoteKey),
}

/// §5.10's five forms, told apart by the segment count: one segment is the bare chain key (a `galeed/1` §3.2 integer
/// spelling), two or three an item key read by [`quote_key`], whose refusals these are.
pub fn venue_key(s: &str) -> Result<VenueKey, QuoteKeyError> {
    if s.contains(':') {
        return quote_key(s).map(VenueKey::Item);
    }
    if s.is_empty() {
        return Err(QuoteKeyError::Parts);
    }
    int_spelling(s).map(VenueKey::Chain).ok_or(QuoteKeyError::Chain)
}

/// **Whether the quote table `q` quotes this venue** (§5.10 read as "is a quote here", whatever the token): a key on
/// `chain` naming its address, its code (where the code is known), or the bare chain key. A key that does not read
/// quotes nothing. The one reading every door asking this takes: the guild's own engagement gate, the coverage a guild
/// track gives a seat call, and the venues a pool member watches.
pub fn quotes_venue(q: &Value, chain: u64, contract: &str, code_hash: Option<&str>) -> bool {
    let Value::Obj(keys) = q else { return false };
    keys.iter().any(|(k, _)| match venue_key(k) {
        Ok(VenueKey::Chain(c)) => c == chain,
        Ok(VenueKey::Item(k)) if k.chain == chain => match k.item {
            QuoteItem::Address(a) => a.eq_ignore_ascii_case(contract),
            QuoteItem::Standard(h) => code_hash.is_some_and(|c| h.eq_ignore_ascii_case(c)),
        },
        _ => false,
    })
}

pub fn quote_key_string(chain: u64, item: &QuoteItem, token: Option<&str>) -> String {
    let middle = match item {
        QuoteItem::Address(a) => a.as_str(),
        QuoteItem::Standard(h) => h.as_str(),
    };
    match token {
        Some(t) => format!("{chain}:{middle}:{t}"),
        None => format!("{chain}:{middle}"),
    }
}

/// The two address forms in §5.10 order: the token-carrying one first.
pub fn quote_keys_address(chain: u64, contract: &str, token: &str) -> [String; 2] {
    let item = QuoteItem::Address(contract.to_string());
    [quote_key_string(chain, &item, Some(token)), quote_key_string(chain, &item, None)]
}

/// The two standard forms in §5.10 order; they come after the two address forms (address before code).
pub fn quote_keys_standard(chain: u64, code_hash: &str, token: &str) -> [String; 2] {
    let item = QuoteItem::Standard(code_hash.to_string());
    [quote_key_string(chain, &item, Some(token)), quote_key_string(chain, &item, None)]
}

/// The `quoteKeys` narrowing a standard applies to quote keys (§7: a venue standard may narrow §5.10's key forms):
/// a closed set of two names, `any` by default.
pub const QUOTE_KEYS_ANY: &str = "any";
pub const QUOTE_KEYS_TOKEN: &str = "token";
/// The closed set (ordered from narrowest to widest).
pub const QUOTE_KEYS: [&str; 2] = [QUOTE_KEYS_TOKEN, QUOTE_KEYS_ANY];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QuoteKeys {
    /// Default: both forms, the token-carrying one first.
    #[default]
    Any,
    /// Only the token-carrying form: if it does not resolve there is no quote for this token, and the
    /// tokenless key is never used as a fallback.
    Token,
}

impl QuoteKeys {
    /// Read the declared cell. Not declared means the default `any`; declared but outside the closed set is
    /// `None`: no narrowing is guessed, and callers treat it as "no quote".
    #[cfg(not(feature = "ablate-j5-quote-outside"))]
    pub fn of(declared: Option<&str>) -> Option<QuoteKeys> {
        match declared {
            None => Some(QuoteKeys::Any),
            Some(QUOTE_KEYS_ANY) => Some(QuoteKeys::Any),
            Some(QUOTE_KEYS_TOKEN) => Some(QuoteKeys::Token),
            Some(_) => None,
        }
    }

    /// Fault-injection variant for the conformance gauges: an out-of-set value reads as the default `any`.
    #[cfg(feature = "ablate-j5-quote-outside")]
    pub fn of(declared: Option<&str>) -> Option<QuoteKeys> {
        match declared {
            Some(QUOTE_KEYS_TOKEN) => Some(QuoteKeys::Token),
            _ => Some(QuoteKeys::Any),
        }
    }

    /// Whether a key is taken under the narrowing: `Any` takes both forms, `Token` only the token-carrying one.
    #[cfg(not(feature = "ablate-j5-quote-narrowed"))]
    pub fn takes(self, key: &str) -> bool {
        match self {
            QuoteKeys::Any => true,
            QuoteKeys::Token => matches!(quote_key(key), Ok(k) if k.token.is_some()),
        }
    }

    /// Fault-injection variant for the conformance gauges: the narrowing is ignored.
    #[cfg(feature = "ablate-j5-quote-narrowed")]
    pub fn takes(self, key: &str) -> bool {
        let _ = (self, key);
        true
    }
}

/// The keys of one item under its narrowing: `item` is the item's parsed form and `keys` its declared narrowing.
pub fn quote_keys_of(chain: u64, item: &QuoteItem, token: &str, keys: QuoteKeys) -> Vec<String> {
    [quote_key_string(chain, item, Some(token)), quote_key_string(chain, item, None)].into_iter().filter(|k| keys.takes(k)).collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BenchThree {
    Null,
    Unresolved,
    Sorted(Vec<String>),
}

/// §5.5 `bench(B)`, by the value of `B`.
pub fn bench_value(logs: &Logs, chain: u64, b: &Value) -> BenchThree {
    match b {
        Value::Null => BenchThree::Null,
        Value::Str(s) => match pointer(s) {
            Some((c, addr)) if c == chain => guild_of(logs, chain, addr).map_or(BenchThree::Unresolved, |g| BenchThree::Sorted(g.bench)),
            _ => BenchThree::Unresolved,
        },
        Value::Arr(_) => bench_array(logs, b),
        _ => BenchThree::Unresolved,
    }
}

/// §5.5's array row: every element hex20, the set of every key of the record of each element; any other array is
/// `unresolved`.
pub fn bench_array(logs: &Logs, b: &Value) -> BenchThree {
    let Value::Arr(a) = b else { return BenchThree::Unresolved };
    match a.iter().map(|e| e.as_str().filter(|s| hexform::is_hex20(s))).collect::<Option<Vec<_>>>() {
        Some(list) => {
            let book = Book::of(logs);
            BenchThree::Sorted(bytewise_set(list.iter().flat_map(|e| {
                let row = Row { listed: e.to_string(), walk: book.identity(e), keys: book.record_keys(e), named: None };
                bench_keys(&row).into_iter()
            })))
        }
        None => BenchThree::Unresolved,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PointerResult {
    NotPointer,
    OtherChain,
    Guild(Result<Guild, Refusal>),
}

impl PointerResult {
    pub const E_POINTER: &'static str = "E_POINTER";
    pub const E_POINTER_CHAIN: &'static str = "E_POINTER_CHAIN";
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidates {
    /// Construction order, not re-sorted.
    pub list: Vec<String>,
    /// Pointer order: each pointer string verbatim with its result.
    pub per_pointer: Vec<(String, PointerResult)>,
}

/// §5.6 `candidates(pointers, allow, deny)`: first, per pointer and per judges row of a resolved guild, add the row's
/// final address unless `deny` holds a key of the record of the row's listed address; then add each `allow`; nothing
/// already listed is added again.
pub fn candidates(logs: &Logs, chain: u64, pointers: &[String], allow: &[String], deny: &[String]) -> Candidates {
    let book = Book::of(logs);
    let mut list: Vec<String> = Vec::new();
    let mut per_pointer = Vec::new();
    for s in pointers {
        let result = match pointer(s) {
            None => PointerResult::NotPointer,
            Some((c, _)) if c != chain => PointerResult::OtherChain,
            Some((_, addr)) => PointerResult::Guild(guild_in(logs, &book, chain, addr)),
        };
        if let PointerResult::Guild(Ok(g)) = &result {
            for r in &g.judges {
                if !denied(r, deny) && !list.contains(&r.walk.final_addr) {
                    list.push(r.walk.final_addr.clone());
                }
            }
        }
        per_pointer.push((s.clone(), result));
    }
    for a in allow {
        if !list.contains(a) {
            list.push(a.clone());
        }
    }
    Candidates { list, per_pointer }
}

/// §5.6: `deny` reaches every key of the judge's record, the keys before the listed address and after it alike.
#[cfg(not(feature = "ablate-k4-candidates"))]
fn denied(r: &Row, deny: &[String]) -> bool {
    r.keys.iter().any(|k| deny.contains(k))
}

/// Fault-injection variant for the conformance gauges: `deny` is compared with the listed address and the keys after
/// it only, so a party that denied a judge's earlier key loses the denial once the guild lists the judge's new key.
#[cfg(feature = "ablate-k4-candidates")]
fn denied(r: &Row, deny: &[String]) -> bool {
    r.followed().addresses().iter().any(|k| deny.contains(k))
}

/// Three-value projection of a bench pointer `(chainId, guild address)`: no pointer is `null`, otherwise
/// the pointer string goes to [`bench_value`] (with the pointer's own chain id).
pub fn bench_three(logs: &Logs, pointer: Option<(u64, &str)>) -> BenchThree {
    match pointer {
        None => BenchThree::Null,
        Some((chain, guild)) => bench_value(logs, chain, &Value::Str(pointer_string(chain, guild))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn publish(emitter: &str, block: u64, idx: u64, doc: &str) -> Publication {
        Publication::of(emitter, block, idx, doc.as_bytes())
    }
    fn a(n: u8) -> String {
        format!("0x{}", format!("{:02x}", n).repeat(20))
    }
    fn declare(from: &str, to: &str, block: u64, idx: u64) -> Declaration {
        Declaration { emitter: from.to_string(), block_number: block, log_index: idx, to: to.to_string() }
    }
    fn accede(to: &str, holder: &str, block: u64, idx: u64) -> Accession {
        Accession { emitter: to.to_string(), block_number: block, log_index: idx, holder: holder.to_string(), root: holder.to_string() }
    }
    /// `from` hands its record to `to`: a declaration at `(block, 0)` and the accession at `(block, 1)`.
    fn hand(logs: &mut Logs, from: &str, to: &str, block: u64) {
        logs.declarations.push(declare(from, to, block, 0));
        logs.accessions.push(accede(to, from, block, 1));
    }
    fn basis(sender: &str) -> String {
        format!(r#"{{"chains":[{{"chainId":1,"fromBlock":0,"toBlock":9,"registries":["{}"],"senders":["{sender}"]}}]}}"#, a(0xee))
    }
    fn judge_doc(judge: &str, entry: &str, basis: &str) -> String {
        format!(r#"{{"kind":"galeed.judge/1","judge":"{judge}","issued":1,"amanah":{{"entry_id":"{entry}"}},"ledger":{{"root":"{judge}","basis":{basis}}}}}"#)
    }
    fn h(n: u8) -> String {
        format!("0x{}", format!("{:02x}", n).repeat(32))
    }

    #[test]
    fn decode_order_and_tables() {
        let g = a(1);
        let ok = format!(r#"{{"kind":"galeed.roster/1","guild":"{g}","chainId":1,"issued":10,"judges":["{}"],"zz":1,"aa":2}}"#, a(2));
        match decode(&publish(&g, 1, 0, &ok)) {
            Ok(Doc::Roster(r)) => assert_eq!(r.unknown_fields, vec!["aa".to_string(), "zz".to_string()]),
            other => panic!("{other:?}"),
        }
        let upper = format!(r#"{{"kind":"galeed.roster/1","guild":"{}","chainId":1,"issued":10,"judges":[]}}"#, a(0xab).to_ascii_uppercase().replacen("0X", "0x", 1));
        assert_eq!(decode(&publish(&g, 1, 0, &upper)), Err(DecodeError::RosterGuild), "大小写永不归一");
        let bad_bench = format!(r#"{{"kind":"galeed.roster/1","guild":"{g}","chainId":1,"issued":10,"judges":[],"bench":["{}"]}}"#, a(5));
        assert_eq!(decode(&publish(&g, 1, 0, &bad_bench)), Err(DecodeError::RosterBench));
        let mut p = publish(&g, 1, 0, &ok);
        p.word = h(0);
        assert_eq!(decode(&p), Err(DecodeError::Hash), "第一步:字是文书字节的 keccak");
        assert_eq!(decode(&publish(&g, 1, 0, "[1]")), Err(DecodeError::Json), "根不是对象与读不成同一枚");
        assert_eq!(decode(&publish(&g, 1, 0, r#"{"kind":1}"#)), Err(DecodeError::Kind));
        // The old succession kind is no kind of this law: an unknown kind, data and no fault.
        let old_succ = format!(r#"{{"kind":"galeed.succession/1","from":"{g}","to":"{}","chainId":1,"issued":1}}"#, a(2));
        assert_eq!(decode(&publish(&g, 1, 0, &old_succ)), Ok(Doc::Unknown("galeed.succession/1".into())));
        // A roster's old `expires`, `name` and `terms` are members beyond the table: data, never a fault.
        let three = format!(r#"{{"kind":"galeed.roster/1","guild":"{g}","chainId":1,"issued":10,"judges":[],"expires":"soon","name":1,"terms":null}}"#);
        assert!(matches!(decode(&publish(&g, 1, 0, &three)), Ok(Doc::Roster(r)) if r.unknown_fields == vec!["expires".to_string(), "name".into(), "terms".into()]));
        let card = format!(r#"{{"kind":"galeed.guildcard/1","guild":"{g}","issued":3,"home":"https://x","z":1}}"#);
        assert!(matches!(decode(&publish(&g, 1, 0, &card)), Ok(Doc::GuildCard(c)) if c.home.as_deref() == Some("https://x") && c.unknown_fields == vec!["z".to_string()]));
        assert_eq!(decode(&publish(&g, 1, 0, &card.replace(r#""issued":3"#, r#""issued":"3""#))), Err(DecodeError::GuildcardIssued));
        assert_eq!(decode(&publish(&g, 1, 0, &card.replace(r#""home":"https://x""#, r#""note_md":2"#))), Err(DecodeError::GuildcardNote));
        assert_eq!(decode(&publish(&g, 1, 0, r#"{"kind":"galeed.roster/2"}"#)), Ok(Doc::Unknown("galeed.roster/2".into())));
        // A roster's old `guildTerms` member is a member beyond the table now: data, never a fault (§2.2).
        let old = format!(r#"{{"kind":"galeed.roster/1","guild":"{g}","chainId":1,"issued":10,"judges":[],"guildTerms":{{"version":1}}}}"#);
        assert!(matches!(decode(&publish(&g, 1, 0, &old)), Ok(Doc::Roster(r)) if r.unknown_fields == vec!["guildTerms".to_string()]));
    }

    #[test]
    fn the_judge_document_table_decides_in_order_and_the_core_judges_its_basis() {
        let j = a(3);
        let good = judge_doc(&j, &h(7), &basis(&j));
        let Ok(Doc::Judge(d)) = decode(&publish(&j, 1, 0, &good)) else { panic!("良构的判官文档") };
        assert_eq!((d.entry_id.as_str(), d.root.as_str()), (h(7).as_str(), j.as_str()));
        // §9.4 refusals come from the core: an empty registries element order, senders out of order, a sixth member.
        let unsorted = format!(r#"{{"chains":[{{"chainId":1,"fromBlock":0,"toBlock":9,"registries":["{}"],"senders":["{}","{}"]}}]}}"#, a(0xee), a(9), a(8));
        assert_eq!(decode(&publish(&j, 1, 0, &judge_doc(&j, &h(7), &unsorted))), Err(DecodeError::JudgeLedger), "核心判基准:senders 不升序");
        let bare_tx = format!(r#"{{"bareTx":[],"chains":[]}}"#);
        assert_eq!(decode(&publish(&j, 1, 0, &judge_doc(&j, &h(7), &bare_tx))), Err(DecodeError::JudgeLedger), "旧基准成员在形外");
        assert_eq!(decode(&publish(&j, 1, 0, &judge_doc(&j, "0x12", &basis(&j)))), Err(DecodeError::JudgeAmanah));
        assert_eq!(decode(&publish(&j, 1, 0, &good.replace(r#""issued":1"#, r#""issued":"1""#))), Err(DecodeError::JudgeIssued));
        let named = good.replacen('{', r#"{"home":3,"#, 1);
        assert_eq!(decode(&publish(&j, 1, 0, &named)), Err(DecodeError::JudgeHome));
    }

    #[test]
    fn the_covenant_table_computes_its_word_from_the_text() {
        let g = a(1);
        let text = "池席约法 · 全文";
        let doc = format!(r#"{{"kind":"galeed.guildamanah/1","guild":"{g}","chainId":1,"issued":1,"version":2,"text_md":"{text}","quotes":{{"1":"0x"}},"venues":[{{"chainId":1,"contract":"{}","codeHash":"{}","x":1}}]}}"#, a(4), h(5));
        let Ok(Doc::Covenant(c)) = decode(&publish(&g, 1, 0, &doc)) else { panic!("良构的行会判法") };
        assert_eq!(c.sha256, hexform::encode(&cryptox::sha256(text.as_bytes())), "词由正文现算");
        assert_eq!(c.venues.as_ref().map(Vec::len), Some(1));
        assert_eq!(decode(&publish(&g, 1, 0, &doc.replace(r#""quotes":{"1":"0x"}"#, r#""quotes":[]"#))), Err(DecodeError::GuildamanahQuotes));
        assert_eq!(decode(&publish(&g, 1, 0, &doc.replace(r#""version":2"#, r#""version":null"#))), Err(DecodeError::GuildamanahVersion));
    }

    #[test]
    fn candidate_of_selects_by_kind_and_an_unplaced_publication_is_every_kind() {
        let g = a(1);
        let roster = format!(r#"{{"kind":"galeed.roster/1","guild":"{g}","chainId":1,"issued":10,"judges":[]}}"#);
        let card = judge_doc(&g, &h(7), &basis(&g));
        let mut logs = Logs { publications: vec![publish(&g, 1, 0, &roster), publish(&g, 2, 0, &card)], ..Logs::default() };
        assert!(guild_of(&logs, 1, &g).is_ok(), "别种文书不挡名册");
        assert!(matches!(judge_of(&logs, &g), JudgeReading::Judge { .. }), "一址多种,各读各的");
        let mut junk = publish(&g, 3, 0, &roster);
        junk.word = h(0);
        logs.publications.push(junk);
        assert_eq!(guild_of(&logs, 1, &g).err(), Some(Refusal::Decode(DecodeError::Hash)), "读不出的挡所有 kind");
        assert_eq!(judge_of(&logs, &g), JudgeReading::Unread("E_HASH"));
        // An address that published only a judge document has no roster: E_NO_DOCUMENT.
        let only_card = Logs { publications: vec![publish(&a(3), 1, 0, &judge_doc(&a(3), &h(7), &basis(&a(3))))], ..Logs::default() };
        assert_eq!(guild_of(&only_card, 1, &a(3)).err(), Some(Refusal::NoDocument));
    }

    #[test]
    fn judge_resolution_follows_the_record_and_reads_no_clock() {
        let (old, new) = (a(3), a(4));
        let mut logs = Logs { publications: vec![publish(&new, 2, 0, &judge_doc(&new, &h(7), &basis(&new))), publish(&a(5), 3, 0, &judge_doc(&a(6), &h(7), &basis(&a(6))))], ..Logs::default() };
        hand(&mut logs, &old, &new, 1);
        let r = resolve_judge(&logs, &old);
        assert_eq!((r.final_addr.as_str(), r.trail.as_slice(), r.active), (new.as_str(), [old.clone()].as_slice(), true));
        assert_eq!(r.judge.name(), JUDGE);
        assert_eq!(judge_of(&logs, &a(5)), JudgeReading::Mismatch, "陌生人的名片");
        assert_eq!(judge_of(&logs, &a(9)), JudgeReading::None);
    }

    /// §4.2: a declaration moves nothing until the key it names accedes; a later declaration replaces an earlier
    /// pending one; a key that entered the book is never named again; an accession from a key the holder did not name
    /// counts for nothing. §5.3: the keys before an address begin the root's trail, and the attempts are the
    /// declarations no accession took up.
    #[test]
    fn the_replay_binds_counts_and_answers_every_record() {
        let (r0, k1, k2, x) = (a(1), a(2), a(3), a(9));
        let mut logs = Logs::default();
        logs.declarations.push(declare(&r0, &x, 1, 0));
        assert_eq!(current(&logs, &r0), r0, "声明不动记录");
        assert_eq!(Book::of(&logs).pending(&r0).map(|d| d.to.clone()), Some(x.clone()));
        logs.declarations.push(declare(&r0, &k1, 2, 0));
        logs.accessions.push(accede(&x, &r0, 3, 0));
        assert_eq!(current(&logs, &r0), r0, "被替换的声明之所指不能承接");
        logs.accessions.push(accede(&k1, &r0, 4, 0));
        assert_eq!(current(&logs, &r0), k1);
        hand(&mut logs, &k1, &k2, 5);
        logs.declarations.push(declare(&k2, &r0, 6, 0));
        let book = Book::of(&logs);
        assert_eq!(book.current(&k1), k2);
        assert!(book.pending(&k2).is_none(), "入过簿的钥不再被点名");
        assert_eq!(book.record_keys(&k2), vec![r0.clone(), k1.clone(), k2.clone()]);
        let id = resolve_identity(&logs, &k1);
        assert_eq!((id.final_addr.as_str(), id.trail.as_slice()), (k2.as_str(), [k1.clone()].as_slice()), "trail 从所问之址起");
        let at: Vec<(u64, String)> = id.attempts.iter().map(|t| (t.block_number, t.to.clone())).collect();
        assert_eq!(at, vec![(6, r0.clone())], "被承接取走的声明不算尝试;未绑定的照记");
        assert_eq!(resolve_identity(&logs, &r0).attempts.len(), 2, "根的尝试含被替换的声明");
        assert_eq!(book.idle(), &[(3, 0), (6, 0)], "不绑不计的两条");
        // §5.2: a key asked at its own accession is not current there yet.
        assert_eq!(current_at(&logs, &r0, (4, 0)), r0);
        assert_eq!(current_at(&logs, &r0, (4, 1)), k1);
        assert_eq!(current_at(&logs, &r0, LAST_POSITION), k2, "末位之前的都读");
        logs.declarations.push(declare(&k2, &x, LAST_POSITION.0, LAST_POSITION.1 - 1));
        logs.accessions.push(accede(&x, &k2, LAST_POSITION.0, LAST_POSITION.1));
        assert_eq!(current_at(&logs, &r0, LAST_POSITION), k2, "末位上的记录 currentAt 不读");
        assert_eq!(current(&logs, &r0), x, "current 连末位上的记录也读");
    }

    #[test]
    fn retire_resume_and_card() {
        let g = a(1);
        let r = format!(r#"{{"kind":"galeed.roster/1","guild":"{g}","chainId":1,"issued":10,"judges":["{}","{}"]}}"#, a(6), a(2));
        let mut logs = Logs { publications: vec![publish(&g, 5, 0, &r)], ..Logs::default() };
        let got = guild_of(&logs, 1, &g).unwrap();
        assert_eq!((got.issued, got.card.name(), &got.covenant), (10, CARD_NONE, &CovenantReading::None));
        assert_eq!(guild_of(&logs, 2, &g).err(), Some(Refusal::ChainMismatch));
        logs.publications.push(publish(&g, 8, 0, &format!(r#"{{"kind":"galeed.guildcard/1","guild":"{}","issued":1}}"#, a(7))));
        assert_eq!(guild_of(&logs, 1, &g).unwrap().card, CardReading::Mismatch);
        logs.statuses.push(Status { emitter: g.clone(), block_number: 6, log_index: 0, active: false });
        assert_eq!(guild_of(&logs, 1, &g).err(), Some(Refusal::Retired));
        logs.statuses.push(Status { emitter: g.clone(), block_number: 7, log_index: 0, active: true });
        assert!(guild_of(&logs, 1, &g).is_ok(), "复业");
    }

    #[test]
    fn bench_and_deny_reach_every_key_of_the_record() {
        let g = a(1);
        let roster = format!(r#"{{"kind":"galeed.roster/1","guild":"{g}","chainId":1,"issued":10,"judges":["{}","{}"]}}"#, a(3), a(7));
        let mut logs = Logs { publications: vec![publish(&g, 1, 0, &roster)], ..Logs::default() };
        hand(&mut logs, &a(2), &a(3), 2);
        let c = candidates(&logs, 1, &[pointer_string(1, &g), "galeed:guild:01:x".into(), pointer_string(2, &g)], &[a(8)], &[a(2)]);
        assert_eq!(c.list, vec![a(7), a(8)], "deny 命中所列之址之前的钥;allow 在后");
        assert!(matches!(c.per_pointer[1].1, PointerResult::NotPointer));
        assert!(matches!(c.per_pointer[2].1, PointerResult::OtherChain));
        assert_eq!(bench_value(&logs, 1, &Value::Arr(vec![Value::Str(a(3))])), BenchThree::Sorted(vec![a(2), a(3)]), "所列之址之前的钥也在席");
        assert_eq!(bench_value(&logs, 1, &Value::Bool(true)), BenchThree::Unresolved);
        assert_eq!(bench_three(&logs, None), BenchThree::Null);
        let f = follow(&logs, &a(2));
        assert_eq!((f.final_addr.as_str(), f.moved(), f.addresses()), (a(3).as_str(), true, vec![a(2), a(3)]));
    }

    #[test]
    fn quote_of_tries_five_keys_address_before_code_token_before_none() {
        let (c, h32, t) = (a(0xaa), h(0x11), a(0xbb));
        let q = |m: &[(&str, &str)]| Value::Obj(m.iter().map(|(k, v)| ((*k).to_string(), Value::Str((*v).to_string()))).collect());
        let all = q(&[("1", "0x01"), (&format!("1:{h32}"), "0x02"), (&format!("1:{h32}:{t}"), "0x03"), (&format!("1:{c}"), "0x04"), (&format!("1:{c}:{t}"), "0x05")]);
        assert_eq!(quote_of(&all, 1, &c, &h32, Some(&t)), QuoteReading::Quote { key: format!("1:{c}:{t}"), quote: "0x05".into() });
        assert_eq!(quote_of(&all, 1, &c, &h32, None), QuoteReading::Quote { key: format!("1:{c}"), quote: "0x04".into() }, "不带币即跳过带币的键");
        assert_eq!(quote_of(&q(&[("1", "0x")]), 1, &c, &h32, Some(&t)), QuoteReading::Quote { key: "1".into(), quote: "0x".into() }, "裸链键在末;0x 是有报价");
        assert_eq!(quote_of(&q(&[(&format!("1:{h32}"), "0xA1")]), 1, &c, &h32, None), QuoteReading::Form { key: format!("1:{h32}") });
        assert_eq!(quote_of(&q(&[("2", "0x01")]), 1, &c, &h32, None), QuoteReading::None);
        assert_eq!(quote_of(&Value::Obj(vec![]), 1, &c, &h32, None), QuoteReading::None);
    }

    #[test]
    fn quote_keys_read_the_four_forms_and_name_each_refusal() {
        let a20 = "0x00000000000000000000000000000000000000aa";
        let t20 = "0x00000000000000000000000000000000000000bb";
        let h32 = "0x1111111111111111111111111111111111111111111111111111111111111111";
        assert_eq!(quote_key(&format!("1:{a20}")), Ok(QuoteKey { chain: 1, item: QuoteItem::Address(a20.into()), token: None }));
        assert_eq!(quote_key(&format!("31337:{h32}:{t20}")), Ok(QuoteKey { chain: 31337, item: QuoteItem::Standard(h32.into()), token: Some(t20.into()) }));
        assert_eq!(quote_key(a20), Err(QuoteKeyError::Parts), "一段");
        assert_eq!(quote_key(&format!("01:{a20}")), Err(QuoteKeyError::Chain), "链号不是整数拼法");
        assert_eq!(quote_key(&format!("1:{}", a20.to_uppercase())), Err(QuoteKeyError::Item), "大写拼法不成键");
        assert_eq!(quote_key(&format!("1:{a20}:{h32}")), Err(QuoteKeyError::Token), "末段另一枚 hex32 在形外");
        assert_eq!(quote_keys_address(1, a20, t20), [format!("1:{a20}:{t20}"), format!("1:{a20}")]);
        assert_eq!(quote_keys_standard(1, h32, t20), [format!("1:{h32}:{t20}"), format!("1:{h32}")]);
    }

    /// §5.10, the five forms: the bare chain key is the one-segment form, covering every venue on its chain. victim: a guild engaging through the bare key, which three readers must answer alike.
    #[test]
    fn the_bare_chain_key_is_a_venue_key_and_quotes_every_venue_on_its_chain() {
        let (a20, h32) = (a(0x11), format!("0x{}", "22".repeat(32)));
        assert_eq!(venue_key("31337"), Ok(VenueKey::Chain(31337)));
        assert_eq!(venue_key(&format!("1:{a20}")), Ok(VenueKey::Item(QuoteKey { chain: 1, item: QuoteItem::Address(a20.clone()), token: None })));
        assert_eq!(venue_key(""), Err(QuoteKeyError::Parts), "空键");
        assert_eq!(venue_key("01"), Err(QuoteKeyError::Chain), "前导零不是整数拼法");
        assert_eq!(venue_key("9007199254740992"), Err(QuoteKeyError::Chain), "超 2^53 − 1");
        let q = |k: &str| Value::Obj(vec![(k.to_string(), Value::Str("0x".into()))]);
        assert!(quotes_venue(&q("1"), 1, &a20, None), "裸键报其链上任一场所");
        assert!(!quotes_venue(&q("1"), 2, &a20, None), "别链不报");
        assert!(quotes_venue(&q(&format!("1:{a20}")), 1, &a20, None));
        assert!(!quotes_venue(&q(&format!("1:{h32}")), 1, &a20, None), "码不知即码键不报");
        assert!(quotes_venue(&q(&format!("1:{h32}:{a20}")), 1, &a(0x33), Some(&h32)));
        assert!(!quotes_venue(&q("01"), 1, &a20, None), "读不成的键不报");
        assert!(!quotes_venue(&Value::Null, 1, &a20, None));
    }

    #[test]
    fn the_narrowing_drops_the_tokenless_key_and_never_guesses_an_unknown_word() {
        let t20 = "0x00000000000000000000000000000000000000bb";
        let h32 = "0x1111111111111111111111111111111111111111111111111111111111111111";
        let item = QuoteItem::Standard(h32.into());
        assert_eq!(quote_keys_of(1, &item, t20, QuoteKeys::Any), vec![format!("1:{h32}:{t20}"), format!("1:{h32}")]);
        assert_eq!(quote_keys_of(1, &item, t20, QuoteKeys::Token), vec![format!("1:{h32}:{t20}")]);
        assert_eq!(QuoteKeys::of(None), Some(QuoteKeys::Any));
        assert_eq!(QuoteKeys::of(Some("TOKEN")), None, "闭集逐字节,大写拼法不成");
    }
}
