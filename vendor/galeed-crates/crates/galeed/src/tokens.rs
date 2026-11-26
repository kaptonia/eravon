//! The galeed/1 closed tables in the shape the family's shared desk code reads them (the desk's ledger, audit
//! and signing layers were written against these names): refusal tokens (§10), finding names (§8.3), labels and
//! report keys (§8.7), signing domains (§5.6) and entry types (§6). Every byte here comes from the law modules
//! that own it ([`crate::token`], [`crate::audit`], [`crate::body`], [`crate::LAW`]); this module only names
//! them, it decides nothing.

pub use crate::token::Token;

impl Token {
    /// The token's spelling, by the name the family's shared code calls it.
    pub fn as_str(self) -> &'static str {
        self.name()
    }
}

/// The finding names of §8.3 (plus the two this law adds: carried bytes that are not an entry, and a foreign
/// record).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FindingName {
    SeqGap,
    PrevMismatch,
    AuthorityMismatch,
    RootMismatch,
    Equivocation,
    CarriedMalformed,
    Foreign,
}

impl FindingName {
    pub fn as_str(self) -> &'static str {
        match self {
            FindingName::SeqGap => crate::audit::SEQ_GAP,
            FindingName::PrevMismatch => crate::audit::PREV_MISMATCH,
            FindingName::AuthorityMismatch => crate::audit::AUTHORITY_MISMATCH,
            FindingName::RootMismatch => crate::audit::ROOT_MISMATCH,
            FindingName::Equivocation => crate::audit::EQUIVOCATION,
            FindingName::CarriedMalformed => crate::audit::CARRIED_MALFORMED,
            FindingName::Foreign => crate::audit::FOREIGN,
        }
    }

    pub const ALL: [FindingName; 7] = [
        FindingName::SeqGap,
        FindingName::PrevMismatch,
        FindingName::AuthorityMismatch,
        FindingName::RootMismatch,
        FindingName::Equivocation,
        FindingName::CarriedMalformed,
        FindingName::Foreign,
    ];
}

/// The four labels of §8.7 item 12 and the no-label answer of §9.4.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Label {
    BrokenChain,
    Unproven,
    Gaps,
    Complete,
    NoLabel,
}

impl Label {
    pub fn as_str(self) -> &'static str {
        match self {
            Label::BrokenChain => crate::audit::LABEL_BROKEN_CHAIN,
            Label::Unproven => crate::audit::LABEL_UNPROVEN,
            Label::Gaps => crate::audit::LABEL_GAPS,
            Label::Complete => crate::audit::LABEL_COMPLETE,
            Label::NoLabel => crate::audit::NO_LABEL,
        }
    }

    pub const ALL: [Label; 5] = [Label::BrokenChain, Label::Unproven, Label::Gaps, Label::Complete, Label::NoLabel];
}

/// The report's member names (the thirteen of §8.7, the no-label members of §9.4, and the row keys the report
/// writes).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Root,
    Basis,
    Entries,
    Findings,
    Missing,
    Anchored,
    Excluded,
    UnknownType,
    Malformed,
    Unproven,
    Void,
    Discarded,
    LabelKey,
    Ok,
    Reason,
    Name,
    Position,
    EntryId,
    Hard,
    Expected,
    Actual,
    SeqKey,
    Certain,
    A,
    B,
    Hash,
    Anchors,
    ChainId,
    Tx,
    BlockNumber,
    Author,
    Index,
    Sender,
    BlockTimestamp,
    EntryTypeKey,
    TokenKey,
    VerdictKey,
}

impl Key {
    pub fn as_str(self) -> &'static str {
        match self {
            Key::Root => "root",
            Key::Basis => "basis",
            Key::Entries => "entries",
            Key::Findings => "findings",
            Key::Missing => "missing",
            Key::Anchored => "anchored",
            Key::Excluded => "excluded",
            Key::UnknownType => "unknown_type",
            Key::Malformed => "malformed",
            Key::Unproven => "unproven",
            Key::Void => "void",
            Key::Discarded => "discarded",
            Key::LabelKey => "label",
            Key::Ok => "ok",
            Key::Reason => "reason",
            Key::Name => "name",
            Key::Position => "position",
            Key::EntryId => "entry_id",
            Key::Hard => "hard",
            Key::Expected => "expected",
            Key::Actual => "actual",
            Key::SeqKey => "seq",
            Key::Certain => "certain",
            Key::A => "a",
            Key::B => "b",
            Key::Hash => "hash",
            Key::Anchors => "anchors",
            Key::ChainId => "chainId",
            Key::Tx => "tx",
            Key::BlockNumber => "blockNumber",
            Key::Author => "judge",
            Key::Index => "index",
            Key::Sender => "sender",
            Key::BlockTimestamp => "blockTimestamp",
            Key::EntryTypeKey => "entryType",
            Key::TokenKey => "token",
            Key::VerdictKey => "verdict",
        }
    }
}

/// The ledger entry's signing domain (§5.6): the law's own literal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Domain {
    Entry,
}

impl Domain {
    pub fn as_str(self) -> &'static str {
        match self {
            Domain::Entry => crate::LAW,
        }
    }
}

/// The spec literal of §4.1.
pub const SPEC: &str = crate::LAW;

/// The eight types of §6, plus `Other` for a type outside the table (§6.8: still an entry).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EntryType {
    Genesis,
    Amanah,
    Judgment,
    Resignation,
    Annotation,
    Succession,
    Migration,
    Keybinding,
    Other,
}

impl EntryType {
    pub fn as_str(self) -> &'static str {
        match self {
            EntryType::Genesis => crate::body::GENESIS,
            EntryType::Amanah => crate::body::AMANAH,
            EntryType::Judgment => crate::body::JUDGMENT,
            EntryType::Resignation => crate::body::RESIGNATION,
            EntryType::Annotation => crate::body::ANNOTATION,
            EntryType::Succession => crate::body::SUCCESSION,
            EntryType::Migration => crate::body::MIGRATION,
            EntryType::Keybinding => crate::body::KEYBINDING,
            EntryType::Other => "",
        }
    }

    /// Known types map to themselves, anything else to `Other`.
    pub fn of(x: &str) -> EntryType {
        [
            EntryType::Genesis,
            EntryType::Amanah,
            EntryType::Judgment,
            EntryType::Resignation,
            EntryType::Annotation,
            EntryType::Succession,
            EntryType::Migration,
            EntryType::Keybinding,
        ]
        .into_iter()
        .find(|t| t.as_str() == x)
        .unwrap_or(EntryType::Other)
    }
}
