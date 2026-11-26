//! Closed tables of `zikaron/2` (`base/docs/zikaron-v2.md`) that differ from `zikaron/1`'s: the spec literal
//! (§4.1), the one domain (§5.6), the eight entry types (§6), the four labels (§8.6) and the report's member
//! and row keys (§8.6). The tables both laws share are the `zikaron/1` core's own and are used from there: the
//! 26 rejection tokens (§10, [`crate::tokens::Token`]), the five finding names (§8.3,
//! [`crate::tokens::FindingName`]) and the three anchor verdicts (§9.3, [`crate::tokens::Verdict`]).

/// The trace mark of this law's core (diagnostic only, never a decision).
pub const K3: &str = "K3";

/// The spec literal of law §4.1 step 4.
pub const SPEC: &str = "zikaron/2";

/// The one signing domain of law §5.6; the list is closed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Domain {
    Entry,
}

impl Domain {
    pub fn as_str(self) -> &'static str {
        match self {
            Domain::Entry => "zikaron/2",
        }
    }
}

/// The eight types of law §6.1 to §6.8, plus the open member of §6.9: `Other` is every type the table does not
/// name (an object body is all it needs).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EntryType {
    Genesis,
    History,
    Grant,
    Revocation,
    Succession,
    Migration,
    Annotation,
    Retraction,
    Other,
}

impl EntryType {
    /// Known types by byte equality of the decoded string (law §6), anything else `Other` (§6.9).
    pub fn of(x: &str) -> EntryType {
        match x {
            "genesis" => EntryType::Genesis,
            "history" => EntryType::History,
            "grant" => EntryType::Grant,
            "revocation" => EntryType::Revocation,
            "succession" => EntryType::Succession,
            "migration" => EntryType::Migration,
            "annotation" => EntryType::Annotation,
            "retraction" => EntryType::Retraction,
            _ => EntryType::Other,
        }
    }
}

/// The token §12.6 matches in a `zikaron/1` succession's `kind`: the closing succession of a continued ledger.
pub const CLOSING_KIND: &str = "migration";

/// The closed label set of law §8.6 item 15, in the order the label is decided: the first that applies wins,
/// and for a continued ledger the first of the two reports' labels in this order is the label.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Label {
    BrokenChain,
    Unproven,
    Gaps,
    Complete,
}

impl Label {
    pub fn as_str(self) -> &'static str {
        match self {
            Label::BrokenChain => "BROKEN_CHAIN",
            Label::Unproven => "UNPROVEN",
            Label::Gaps => "GAPS",
            Label::Complete => "COMPLETE",
        }
    }

    pub fn parse(x: &str) -> Option<Label> {
        Label::ALL.into_iter().find(|l| l.as_str() == x)
    }

    /// A `zikaron/1` report's label read in this law's order (§8.6 item 15): its `UNAVAILABLE` reads as
    /// `UNPROVEN`. `None` for a string that is no label of either law.
    pub fn of_v1(x: &str) -> Option<Label> {
        if x == crate::tokens::Label::Unavailable.as_str() {
            return Some(Label::Unproven);
        }
        Label::parse(x)
    }

    pub const ALL: [Label; 4] = [Label::BrokenChain, Label::Unproven, Label::Gaps, Label::Complete];
}

/// The no-label reason of law §8.6 (`{"ok":false,"reason":"NO_LABEL"}`).
pub const NO_LABEL: &str = "NO_LABEL";

/// Keys of the report of law §8.6: its sixteen members, the `NO_LABEL` object's two, and every row key. The
/// report's byte shape grows only from here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Root,
    Basis,
    Entries,
    Findings,
    Missing,
    Anchored,
    Unanchored,
    Excluded,
    Retired,
    UnknownType,
    Malformed,
    Unproven,
    Void,
    Discarded,
    Continues,
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
    BlockTimestamp,
    Author,
    Sender,
    EntryTypeKey,
    TokenKey,
    VerdictKey,
    Cut,
    Report,
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
            Key::Unanchored => "unanchored",
            Key::Excluded => "excluded",
            Key::Retired => "retired",
            Key::UnknownType => "unknown_type",
            Key::Malformed => "malformed",
            Key::Unproven => "unproven",
            Key::Void => "void",
            Key::Discarded => "discarded",
            Key::Continues => "continues",
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
            Key::BlockTimestamp => "blockTimestamp",
            Key::Author => "author",
            Key::Sender => "sender",
            Key::EntryTypeKey => "entryType",
            Key::TokenKey => "token",
            Key::VerdictKey => "verdict",
            Key::Cut => "cut",
            Key::Report => "report",
        }
    }
}
