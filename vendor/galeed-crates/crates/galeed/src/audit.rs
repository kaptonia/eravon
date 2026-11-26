//! The [C] side of §7 to §9.5: lineage (§7.4), the chain walk (§8.2), equivocation and the carried record (§8.4),
//! reconciliation (§8.5), the report and its label (§8.7), basis well-formedness and record reach (§9.4), and how
//! the three anchor verdicts are read (§9.3, §8.6). The audit is a pure function of two inputs, the root and the
//! anchor set with the basis that produced it (§8); the entries are the bytes the anchor records carry (§8.1), and
//! nothing a reader holds beside the chain is read. The conformance harness packs the inputs into one object for
//! `gl1 audit`.
//!
//! Input well-formedness is decided in [`read_input`]: any failure is None and the caller answers
//! `{"ok":false,"reason":"NO_LABEL"}`. The report is computed in [`audit`] as the thirteen-member object of §8.7;
//! member order comes from the canonical writer. [`crate::draft`] runs the same walk (§8.2) and the same pair test
//! (§8.4) over a machine's own entries, which is a draft and never a ledger (§5.7).

#![cfg_attr(any(feature = "ablate-k1-basis-form", feature = "ablate-k1-audit-walk"), allow(dead_code))]

use crate::body;
use crate::cryptox;
use crate::envelope;
use crate::hexform;
use crate::json::{Value, INT_MAX};
use std::collections::{BTreeMap, BTreeSet};

/// The three verdicts of §9.3.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verdict {
    Counted,
    Unproven,
    Void,
}

pub const VERDICT_COUNTED: &str = "counted";
pub const VERDICT_UNPROVEN: &str = "UNPROVEN";
pub const VERDICT_VOID: &str = "VOID";

impl Verdict {
    pub fn parse(s: &str) -> Option<Verdict> {
        match s {
            VERDICT_COUNTED => Some(Verdict::Counted),
            VERDICT_UNPROVEN => Some(Verdict::Unproven),
            VERDICT_VOID => Some(Verdict::Void),
            _ => None,
        }
    }
    /// The verdict's name, as the family's shared chain code says it (`galeed_chain::scan`).
    pub fn as_str(self) -> &'static str {
        self.name()
    }
    fn name(self) -> &'static str {
        match self {
            Verdict::Counted => VERDICT_COUNTED,
            Verdict::Unproven => VERDICT_UNPROVEN,
            Verdict::Void => VERDICT_VOID,
        }
    }
}

/// The eight members of a §9.2 anchor record (members outside the table are not read). The eighth,
/// `entry`, appears only on records whose log's data carried the bytes (§9.1): its `sha256` is the record's
/// `hash`. Two records agree on `entry` when neither has it or both carry the same bytes; the bytes are
/// compared, not their transport spelling.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Record {
    pub chain_id: u64,
    pub block_number: u64,
    pub block_timestamp: u64,
    pub tx: String,
    pub sender: String,
    pub hash: String,
    pub verdict: Verdict,
    pub entry: Option<Vec<u8>>,
}

/// The two inputs of §8, already well formed under §9.4: the root, and the anchor set (one record per
/// `(chainId, blockNumber, tx, hash)`) with the basis that produced it.
pub struct Input {
    pub root: String,
    pub anchors: Vec<Record>,
    pub basis: Value,
}

/// The one member name of the §9.4 basis object (`basis` in the harness audit input); other crates borrow it.
pub const BASIS_CHAINS: &str = "chains";

/// The closed set of six finding names (§8.3).
pub const SEQ_GAP: &str = "SEQ_GAP";
pub const PREV_MISMATCH: &str = "PREV_MISMATCH";
pub const AUTHORITY_MISMATCH: &str = "AUTHORITY_MISMATCH";
pub const ROOT_MISMATCH: &str = "ROOT_MISMATCH";
pub const EQUIVOCATION: &str = "EQUIVOCATION";
pub const CARRIED_MALFORMED: &str = "CARRIED_MALFORMED";

/// The token of the second branch of §8.4: a word outside §10, defined only here.
pub const FOREIGN: &str = "FOREIGN";

/// The closed set of four labels (§8.7 item 12), in the order the label is decided. `UNPROVEN` spells a verdict,
/// a list and a label alike (§8.3); the label is this constant and the verdict [`VERDICT_UNPROVEN`], one spelling.
pub const LABEL_BROKEN_CHAIN: &str = "BROKEN_CHAIN";
pub const LABEL_UNPROVEN: &str = VERDICT_UNPROVEN;
pub const LABEL_GAPS: &str = "GAPS";
pub const LABEL_COMPLETE: &str = "COMPLETE";

/// The output when §9.4 well-formedness is refused.
pub const NO_LABEL: &str = "NO_LABEL";

fn int(v: &Value) -> Option<u64> {
    match v {
        Value::Int(n) => Some(*n),
        _ => None,
    }
}
fn hex20(v: &Value) -> Option<String> {
    v.as_str().filter(|s| hexform::is_hex20(s)).map(str::to_string)
}
fn hex32(v: &Value) -> Option<String> {
    v.as_str().filter(|s| hexform::is_hex32(s)).map(str::to_string)
}
fn arr(v: &Value) -> Option<&[Value]> {
    match v {
        Value::Arr(a) => Some(a),
        _ => None,
    }
}
fn exact_keys(v: &Value, keys: &[&str]) -> bool {
    v.as_obj().is_some_and(|m| m.len() == keys.len() && keys.iter().all(|k| v.get(k).is_some()))
}
fn strictly_ascending(items: &[String]) -> bool {
    items.windows(2).all(|w| w[0].as_bytes() < w[1].as_bytes())
}

/// The transport hex of a record's `entry` (HARNESS.md `gl1 audit`): the two lowercase bytes `0x` and an even
/// number of hex digits of either case, `0x` alone being the empty byte string. Any other value, `null` and a `0X`
/// spelling included, is outside the form.
fn transport_bytes(v: &Value) -> Option<Vec<u8>> {
    let s = v.as_str()?;
    let digits = s.strip_prefix("0x")?;
    hexform::decode_digits(digits.as_bytes())
}

fn record(v: &Value) -> Option<Record> {
    v.as_obj()?;
    let hash = hex32(v.get("hash")?)?;
    // §9.2: when present, `entry` is a byte string in the transport spelling and its sha256 is this record's
    // hash; absence is a different case.
    let entry = match v.get("entry") {
        None => None,
        Some(x) => {
            let bytes = transport_bytes(x)?;
            if hexform::encode(&cryptox::sha256(&bytes)) != hash {
                return None;
            }
            Some(bytes)
        }
    };
    Some(Record {
        chain_id: int(v.get("chainId")?)?,
        block_number: int(v.get("blockNumber")?)?,
        block_timestamp: int(v.get("blockTimestamp")?)?,
        tx: hex32(v.get("tx")?)?,
        sender: hex20(v.get("sender")?)?,
        hash,
        verdict: Verdict::parse(v.get("verdict")?.as_str()?)?,
        entry,
    })
}

/// The two forms of a §9.4 `registries` element: a hex20 address, or a class item with exactly one
/// member `codeHash` (hex32). A class item never equals an address.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ablate-k1-basis-registry-class-items", allow(dead_code))]
enum RegistryItem {
    Address(String),
    Class(String),
}

struct Window {
    chain_id: u64,
    from: u64,
    to: u64,
    registries: Vec<RegistryItem>,
    senders: Vec<String>,
}

/// §9.4: the form of `registries`, the order of its two halves and uniqueness within each are decided
/// from the basis bytes alone: addresses first in byte order, class items after in `codeHash` byte
/// order. A duplicate in either half, or an address after a class item, is not a basis.
#[cfg(not(feature = "ablate-k1-basis-registry-class-items"))]
fn registries(v: &Value) -> Option<Vec<RegistryItem>> {
    let mut addresses: Vec<String> = Vec::new();
    let mut classes: Vec<String> = Vec::new();
    for e in arr(v)? {
        match e {
            Value::Str(_) => {
                if !classes.is_empty() {
                    return None;
                }
                addresses.push(hex20(e)?);
            }
            Value::Obj(_) => {
                if !exact_keys(e, &["codeHash"]) {
                    return None;
                }
                classes.push(hex32(e.get("codeHash")?)?);
            }
            _ => return None,
        }
    }
    if !strictly_ascending(&addresses) || !strictly_ascending(&classes) {
        return None;
    }
    Some(addresses.into_iter().map(RegistryItem::Address).chain(classes.into_iter().map(RegistryItem::Class)).collect())
}

/// Fault-injection variant for the conformance gauges: elements are taken as spelled, with no check of
/// form, order or duplicates.
#[cfg(feature = "ablate-k1-basis-registry-class-items")]
fn registries(v: &Value) -> Option<Vec<RegistryItem>> {
    Some(arr(v)?.iter().map(|e| RegistryItem::Address(crate::json::canonical_text(e))).collect())
}

/// §9.4 [C]: the form, arity, order and uniqueness of a basis are decided from its own bytes. The basis is an
/// object with exactly the member `chains`.
#[cfg(not(feature = "ablate-k1-basis-form"))]
fn basis_windows(basis: &Value) -> Option<Vec<Window>> {
    if !exact_keys(basis, &[BASIS_CHAINS]) {
        return None;
    }
    let mut windows: Vec<Window> = Vec::new();
    for c in arr(basis.get(BASIS_CHAINS)?)? {
        if !exact_keys(c, &["chainId", "fromBlock", "toBlock", "registries", "senders"]) {
            return None;
        }
        let senders: Vec<String> = arr(c.get("senders")?)?.iter().map(hex20).collect::<Option<_>>()?;
        let w = Window {
            chain_id: int(c.get("chainId")?)?,
            from: int(c.get("fromBlock")?)?,
            to: int(c.get("toBlock")?)?,
            registries: registries(c.get("registries")?)?,
            senders,
        };
        if w.from > w.to || !strictly_ascending(&w.senders) {
            return None;
        }
        if let Some(p) = windows.last() {
            // (chainId, fromBlock) strictly ascending; no overlap on one chain; adjacent windows on one chain
            // differ in registries or senders.
            if (p.chain_id, p.from) >= (w.chain_id, w.from) {
                return None;
            }
            if p.chain_id == w.chain_id {
                if p.to >= w.from {
                    return None;
                }
                if p.to + 1 == w.from && p.registries == w.registries && p.senders == w.senders {
                    return None;
                }
            }
        }
        windows.push(w);
    }
    Some(windows)
}

/// Fault-injection variant for the conformance gauges: the basis is not checked and the window list is
/// empty.
#[cfg(feature = "ablate-k1-basis-form")]
fn basis_windows(basis: &Value) -> Option<Vec<Window>> {
    let _ = basis;
    Some(Vec::new())
}

/// §9.4: a record is within the basis's reach: some one window of its chain has a range holding its block and
/// `senders` holding its sender. `registries` is not read.
#[cfg(not(feature = "ablate-k1-basis-form"))]
fn reached(r: &Record, windows: &[Window]) -> bool {
    windows.iter().any(|w| w.chain_id == r.chain_id && (w.from..=w.to).contains(&r.block_number) && w.senders.contains(&r.sender))
}

/// Fault-injection variant for the conformance gauges: every record is within reach.
#[cfg(feature = "ablate-k1-basis-form")]
fn reached(r: &Record, windows: &[Window]) -> bool {
    let _ = (r, windows);
    true
}

/// Rows in the order `by` gives (a stable sort): the one place the family's shared chain code sorts its §9.2 tables
/// (`galeed_chain::scan`).
pub fn ordered<T>(mut rows: Vec<T>, by: impl Fn(&T, &T) -> std::cmp::Ordering) -> Vec<T> {
    rows.sort_by(by);
    rows
}

/// Read a `gl1 audit` input object into the two inputs: the members `root`, `anchors` and `basis` (the object is
/// open, and a member beyond them is not read); anything that is not a galeed/1 audit input is None (NO_LABEL).
/// The test is decided on the inputs as supplied, before every trimming of §8.1.
pub fn read_input(v: &Value) -> Option<Input> {
    v.as_obj()?;
    let root = hex20(v.get("root")?)?;
    let raw: Vec<Record> = arr(v.get("anchors")?)?.iter().map(record).collect::<Option<_>>()?;
    let basis = v.get("basis")?.clone();
    let windows = basis_windows(&basis)?;
    // §9.4: records equal on the four keys are one record; equal on the four and different elsewhere is
    // not an audit input.
    let mut by_key: BTreeMap<(u64, u64, String, String), Record> = BTreeMap::new();
    for r in raw {
        let k = (r.chain_id, r.block_number, r.tx.clone(), r.hash.clone());
        match by_key.get(&k) {
            Some(seen) if *seen != r => return None,
            Some(_) => {}
            None => {
                by_key.insert(k, r);
            }
        }
    }
    let anchors: Vec<Record> = by_key.into_values().collect();
    // §9.4: every record is within the basis's reach.
    if !anchors.iter().all(|r| reached(r, &windows)) {
        return None;
    }
    Some(Input { root, anchors, basis })
}

/// An entry value that passed §4.3, read into the members the walk and the lineage need. Shared by [`audit`],
/// [`lineage_of`] and [`crate::draft`]. `raw` is the byte string the entry was read from (empty where the caller held
/// only the value), which is how §7.4 finds the records that carry it.
pub(crate) struct Parsed {
    pub(crate) id: String,
    pub(crate) seq: u64,
    pub(crate) judge: String,
    prev: Option<String>,
    entry_type: String,
    to: Option<String>,
    raw: Vec<u8>,
}

impl Parsed {
    /// §7.4: a `succession` or a `migration`, the two entries whose carrying retires their signer.
    fn hands_off(&self) -> bool {
        self.entry_type == body::SUCCESSION || self.entry_type == body::MIGRATION
    }
}

pub(crate) fn parsed(id: String, v: &Value) -> Parsed {
    let entry_type = v.get("entryType").and_then(Value::as_str).unwrap_or_default().to_string();
    let to = (entry_type == body::SUCCESSION).then(|| v.get("body").and_then(|b| b.get("to")).and_then(Value::as_str).unwrap_or_default().to_string());
    Parsed {
        id,
        seq: v.get("seq").and_then(int).unwrap_or_default(),
        judge: v.get("judge").and_then(Value::as_str).unwrap_or_default().to_string(),
        prev: v.get("prev").and_then(Value::as_str).map(str::to_string),
        entry_type,
        to,
        raw: Vec::new(),
    }
}

/// Byte strings read through §4.3: those that pass, each with its `entry_id`, and those that fail, each named by
/// its `entry_id` and the token the decision order names (§8.1, §8.7 item 8). Shared by [`audit`] and
/// [`crate::draft`].
pub(crate) fn tested<'a>(strings: impl IntoIterator<Item = &'a [u8]>) -> (Vec<Parsed>, Vec<(String, &'static str)>) {
    let mut entries: Vec<Parsed> = Vec::new();
    let mut malformed: Vec<(String, &'static str)> = Vec::new();
    for b in strings {
        let id = hexform::encode(&cryptox::sha256(b));
        match envelope::accept(b) {
            Ok(e) => entries.push(Parsed { raw: b.to_vec(), ..parsed(id, &e.value) }),
            Err(t) => malformed.push((id, t.name())),
        }
    }
    (entries, malformed)
}

/// **§7.4 lineage as a public door**: the lineage of the audit input, the least fixed point seeded by `root` over the
/// carried byte strings that pass §4.3, extended only by a succession some record carries at which its signer has not
/// retired. It is a function of the bytes and of the block times of the records that carry them, so it takes the
/// whole audit input and nothing less; [`audit`] decides over the same set.
///
/// `galeed.papers/1` §9.0 requires the lineage to be decided by the ledger law's own predicate, which is why the
/// papers crate reaches it here.
/// §7.4 over an audit input: whether `key` has a retirement there (a record of the anchor set carries a `succession`
/// or a `migration` it signed). A reader names no seat from a document of a key that has one (`galeed.roster/1` §3a.3).
pub fn has_retirement(input: &Input, key: &str) -> bool {
    let (entries, _) = tested(carried(input));
    retirements(&entries, &input.anchors).contains_key(&key.to_ascii_lowercase())
}

pub fn lineage_of(input: &Input) -> BTreeSet<String> {
    let (entries, _) = tested(carried(input));
    lineage(&input.root, &entries, &input.anchors)
}

/// **The timeless superset of §7.4**, for a reader that holds a machine's own entries and no records: every
/// succession whose signer is in the set extends it, whatever chain time carried it or whether anything did. It reads
/// no time, so the lineage of [`lineage_of`] is a subset of it. They differ on one form: a succession its signer signed
/// after retiring, which §7.4 does not take and this does, its `to` in the set.
///
/// Most callers take a key set where a wider one is harmless (`account_root`, `lineage_keys`, `family_of`, the sender
/// set of `basisgen`, `succeedx`, the desk's `ledger::lineage_keys`). Three decide by it: `v3::seat::standing_gate`
/// (whether a key still stands in the ledger it writes), `ledger::of_member` (which entries are a member's) and
/// `archive::admit` (whether an outside entry's signer may enter). They hold no records to read a retirement from, and
/// the differing form reaches a desk only merged from another machine, since this product refuses to mint it.
pub fn lineage_local(root: &str, entries: &[Value]) -> BTreeSet<String> {
    let parsed: Vec<Parsed> = entries.iter().map(|v| parsed(String::new(), v)).collect();
    fixed_point(root, parsed.iter())
}

/// A finding of §8.2 or §8.4, with the four keys §8.7 item 3 sorts by.
pub(crate) struct Finding {
    name: &'static str,
    position: u64,
    entry_id: String,
    second: String,
    hard: bool,
    args: Vec<(&'static str, Value)>,
}

impl Finding {
    /// The §8.7 item 3 sort key: `(position, finding name, entry_id, second entry_id)`, the empty second entry_id
    /// ordering before every hex32.
    fn key(&self) -> (u64, &[u8], &[u8], &[u8]) {
        (self.position, self.name.as_bytes(), self.entry_id.as_bytes(), self.second.as_bytes())
    }

    pub(crate) fn hard(&self) -> bool {
        self.hard
    }

    /// The finding's row in a report: `name`, `position`, `entry_id`, `hard`, and the arguments §8.2 and §8.4
    /// write for that name.
    pub(crate) fn value(self) -> Value {
        let mut m = vec![("name", s(self.name)), ("position", n(self.position)), ("entry_id", s(&self.entry_id)), ("hard", Value::Bool(self.hard))];
        m.extend(self.args);
        obj(m)
    }
}

/// §8.7 item 3: findings in `(position, finding name, entry_id, second entry_id)` order, each distinct finding once.
pub(crate) fn in_report_order(findings: Vec<Finding>) -> Vec<Finding> {
    let mut by_key: BTreeMap<(u64, Vec<u8>, Vec<u8>, Vec<u8>), Finding> = BTreeMap::new();
    for f in findings {
        let (p, a, b, c) = f.key();
        by_key.insert((p, a.to_vec(), b.to_vec(), c.to_vec()), f);
    }
    by_key.into_values().collect()
}

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}
fn n(x: u64) -> Value {
    Value::Int(x)
}
fn obj(members: Vec<(&str, Value)>) -> Value {
    Value::Obj(members.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn next_expected(seq: u64) -> u64 {
    if seq == INT_MAX { seq } else { seq + 1 }
}

/// §7.4 one key's **retirement** and **retiring seq**: the smallest `blockTimestamp` of the records that carry a
/// `succession` or a `migration` it signed, and the smallest `seq` among those of its hand-offs that a record at that
/// `blockTimestamp` carries. Ordered as the pair, so the smaller of two is the earlier second, then the lower position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Retirement {
    pub at: u64,
    pub seq: u64,
}

/// Each key's [`Retirement`], by the key as its entries spell it.
pub type Retirements = BTreeMap<String, Retirement>;

/// What §7.4 reads of a byte string a record carries, where those bytes pass §4.3: its signer, its position, and
/// whether it hands the seat off (a `succession` or a `migration`). Bytes that fail §4.3 have none: they retire no one
/// and are read at any record as they would be at an earlier one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signed {
    pub judge: String,
    pub seq: u64,
    pub hands_off: bool,
}

/// [`Signed`] of `bytes`, by §4.3 ([`envelope::accept`]); `None` for bytes that fail it.
pub fn signed_of(bytes: &[u8]) -> Option<Signed> {
    let e = envelope::accept(bytes).ok()?;
    let p = parsed(String::new(), &e.value);
    Some(Signed { hands_off: p.hands_off(), judge: p.judge, seq: p.seq })
}

/// **§7.4 retirements over the records read**: each `(blockTimestamp, what it carries)` of a record; every one that
/// carries a hand-off passing §4.3 moves its signer's [`Retirement`] down to the smaller pair, on any chain and whatever
/// the record's `sender` and `verdict` (the caller gives every record it reads, none sifted). The audit, the papers door
/// and the desk's ledger readers build the table here.
pub fn retirements_of<'a>(records: impl IntoIterator<Item = (u64, Option<&'a Signed>)>) -> Retirements {
    let mut out: Retirements = BTreeMap::new();
    for (at, carried) in records {
        if let Some(s) = carried.filter(|s| s.hands_off) {
            let r = Retirement { at, seq: s.seq };
            let cur = out.entry(s.judge.clone()).or_insert(r);
            *cur = (*cur).min(r);
        }
    }
    out
}

/// **§7.4 whether `key` has retired at a record** of block time `at` that carries `carried` (its bytes' [`Signed`], or
/// `None` for a record with no bytes or with bytes that fail §4.3): its block time later than the key's retirement, or
/// equal to it and the record carrying what the key itself signed at a position after its retiring seq. At the
/// retirement's own second, what the key signed at or before its retiring seq, what another key signed, and a record
/// without readable bytes count as at any earlier record. The one predicate of §7.4: the audit's trim asks it of the
/// record's `sender`, the lineage's admission of the succession's `judge`, and the desk's ledger readers of the sender
/// of the record a kit or the registry shows.
#[cfg(not(feature = "ablate-k1-retire"))]
pub fn has_retired(retire: &Retirements, key: &str, at: u64, carried: Option<&Signed>) -> bool {
    retire.get(key).is_some_and(|r| at > r.at || (at == r.at && past_retiring_seq(r, key, carried)))
}

/// Fault-injection variant for the conformance gauges: no key ever retires, so a record a key sent after the first
/// record carrying its succession or migration is neither kept out of the lineage nor discarded.
#[cfg(feature = "ablate-k1-retire")]
pub fn has_retired(retire: &Retirements, key: &str, at: u64, carried: Option<&Signed>) -> bool {
    let _ = (retire, key, at, carried);
    false
}

/// §7.4's second form, at a record of the retirement's own second: it carries what `key` signed at a position after
/// its retiring seq.
#[cfg(not(feature = "ablate-k1-retire-seq"))]
fn past_retiring_seq(r: &Retirement, key: &str, carried: Option<&Signed>) -> bool {
    carried.is_some_and(|s| s.judge == key && s.seq > r.seq)
}

/// Fault-injection variant for the conformance gauges: at the retirement's own second no record is one at which the
/// key has retired, so what the ceding key signed after its handing off and carried in that second stays kept.
#[cfg(feature = "ablate-k1-retire-seq")]
fn past_retiring_seq(r: &Retirement, key: &str, carried: Option<&Signed>) -> bool {
    let _ = (r, key, carried);
    false
}

/// The [`Signed`] of every byte string that passed §4.3, by its bytes: what a record carrying it carries.
fn signed_by_bytes(entries: &[Parsed]) -> BTreeMap<&[u8], Signed> {
    entries.iter().map(|e| (e.raw.as_slice(), Signed { judge: e.judge.clone(), seq: e.seq, hands_off: e.hands_off() })).collect()
}

/// What record `r` carries, as §7.4 reads it ([`Signed`]; `None` with no bytes or bytes that failed §4.3).
fn carried_by<'a>(by_bytes: &'a BTreeMap<&[u8], Signed>, r: &Record) -> Option<&'a Signed> {
    r.entry.as_deref().and_then(|b| by_bytes.get(b))
}

/// §7.4 retirements over the audit's records and the byte strings that passed §4.3 ([`retirements_of`]).
pub(crate) fn retirements(entries: &[Parsed], records: &[Record]) -> Retirements {
    let by_bytes = signed_by_bytes(entries);
    retirements_of(records.iter().map(|r| (r.block_timestamp, carried_by(&by_bytes, r))))
}

/// §7.4 lineage: seeded by root, add the `to` of every carried succession whose judge is already in the set and that
/// some record carries at which its judge has not retired ([`has_retired`]); the least fixed point.
pub(crate) fn lineage(root: &str, entries: &[Parsed], records: &[Record]) -> BTreeSet<String> {
    let retire = retirements(entries, records);
    let by_bytes = signed_by_bytes(entries);
    let mut carriers: BTreeMap<&[u8], Vec<&Record>> = BTreeMap::new();
    for r in records {
        if let Some(b) = r.entry.as_deref() {
            carriers.entry(b).or_default().push(r);
        }
    }
    let admitted = entries.iter().filter(|e| {
        e.to.is_some() && carriers.get(e.raw.as_slice()).is_some_and(|rs| rs.iter().any(|r| !has_retired(&retire, &e.judge, r.block_timestamp, carried_by(&by_bytes, r))))
    });
    fixed_point(root, admitted)
}

/// The least fixed point seeded by `root` over the given successions: add the `to` of every one whose judge is
/// already in the set.
///
/// One pass: index each `judge` to the `to` values it names, then walk from root along the index; every
/// address is queued at most once and every succession looked at at most once. This is the same set as
/// rescanning until nothing changes: each member is reachable from root through a chain of successions
/// whose judges are in the set, and conversely.
#[cfg(not(feature = "ablate-k1-lineage-walk"))]
pub(crate) fn fixed_point<'a>(root: &str, entries: impl Iterator<Item = &'a Parsed>) -> BTreeSet<String> {
    let mut named: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for e in entries {
        if let Some(to) = &e.to {
            named.entry(e.judge.as_str()).or_default().push(to.as_str());
        }
    }
    let mut set: BTreeSet<String> = BTreeSet::new();
    set.insert(root.to_string());
    let mut queue: Vec<&str> = vec![root];
    while let Some(key) = queue.pop() {
        for to in named.remove(key).unwrap_or_default() {
            if set.insert(to.to_string()) {
                queue.push(to);
            }
        }
    }
    set
}

/// Fault-injection variant for the conformance gauges: the lineage is root alone and successions never
/// extend it.
#[cfg(feature = "ablate-k1-lineage-walk")]
pub(crate) fn fixed_point<'a>(root: &str, entries: impl Iterator<Item = &'a Parsed>) -> BTreeSet<String> {
    let _ = entries.count();
    let mut set: BTreeSet<String> = BTreeSet::new();
    set.insert(root.to_string());
    set
}

/// §8.2 chain walk: visit in (seq, entry_id) order, recording SEQ_GAP / PREV_MISMATCH /
/// AUTHORITY_MISMATCH / ROOT_MISMATCH; after each position, update auth per the third rule of §8.2: a `migration`
/// signed by the key in office closes the ledger (no key holds `auth` after it), else the succession of smallest
/// entry_id signed by the key in office moves it.
#[cfg(not(feature = "ablate-k1-audit-walk"))]
pub(crate) fn walk(root: &str, ledger: &[Parsed], findings: &mut Vec<Finding>) {
    let ids_at: BTreeMap<u64, BTreeSet<String>> = ledger.iter().fold(BTreeMap::new(), |mut m, e| {
        m.entry(e.seq).or_default().insert(e.id.clone());
        m
    });
    let mut expected: u64 = 0;
    let mut auth: Option<String> = Some(root.to_string());
    let mut certain = true;
    let mut last: Option<u64> = None;
    let mut i = 0;
    while i < ledger.len() {
        let seq = ledger[i].seq;
        let mut j = i;
        while j < ledger.len() && ledger[j].seq == seq {
            let e = &ledger[j];
            let gap = e.seq != expected && last != Some(e.seq);
            if gap {
                findings.push(Finding {
                    name: SEQ_GAP,
                    position: e.seq,
                    entry_id: e.id.clone(),
                    second: String::new(),
                    hard: false,
                    args: vec![("expected", n(expected)), ("actual", n(e.seq))],
                });
                expected = next_expected(e.seq);
                certain = false;
            } else if e.seq == expected {
                expected = next_expected(e.seq);
            }
            if e.seq >= 1 {
                if let Some(prevs) = ids_at.get(&(e.seq - 1)) {
                    if !e.prev.as_ref().is_some_and(|p| prevs.contains(p)) {
                        findings.push(Finding {
                            name: PREV_MISMATCH,
                            position: e.seq,
                            entry_id: e.id.clone(),
                            second: String::new(),
                            hard: true,
                            args: vec![("seq", n(e.seq))],
                        });
                    }
                }
                if auth.as_deref() != Some(e.judge.as_str()) {
                    findings.push(Finding {
                        name: AUTHORITY_MISMATCH,
                        position: e.seq,
                        entry_id: e.id.clone(),
                        second: String::new(),
                        hard: certain,
                        args: vec![("seq", n(e.seq)), ("certain", Value::Bool(certain))],
                    });
                }
            } else if e.judge != root {
                findings.push(Finding {
                    name: ROOT_MISMATCH,
                    position: 0,
                    entry_id: e.id.clone(),
                    second: String::new(),
                    hard: true,
                    args: vec![("seq", n(0)), ("actual", s(&e.judge))],
                });
            }
            last = Some(e.seq);
            j += 1;
        }
        // Position seq done: a migration signed by the key in office closes the ledger; else, among successions
        // signed by the key in office, the one with the smallest entry_id in byte order moves the seat.
        let in_office = |e: &&Parsed| auth.as_deref() == Some(e.judge.as_str());
        if closes(&ledger[i..j], &auth) {
            auth = None;
        } else if let Some(mover) = ledger[i..j].iter().filter(|e| e.to.is_some()).filter(in_office).min_by(|a, b| a.id.as_bytes().cmp(b.id.as_bytes())) {
            auth = mover.to.clone();
        }
        i = j;
    }
}

/// §8.2: a `migration` at this position signed by the key in office.
#[cfg(not(feature = "ablate-k1-migration-closes"))]
fn closes(position: &[Parsed], auth: &Option<String>) -> bool {
    position.iter().any(|e| e.entry_type == body::MIGRATION && auth.as_deref() == Some(e.judge.as_str()))
}

/// Fault-injection variant for the conformance gauges: a migration never closes the ledger, so the key that signed it
/// stays in office and an entry after it is no authority mismatch.
#[cfg(feature = "ablate-k1-migration-closes")]
fn closes(position: &[Parsed], auth: &Option<String>) -> bool {
    let _ = (position, auth);
    false
}

/// Fault-injection variant for the conformance gauges: the walk records no finding and never moves the seat.
#[cfg(feature = "ablate-k1-audit-walk")]
pub(crate) fn walk(root: &str, ledger: &[Parsed], findings: &mut Vec<Finding>) {
    let _ = (root, ledger, findings);
}

/// §8.4: two different ledger entries with the same seq or the same non-null prev; one per unordered
/// pair, seq taken as the smaller.
pub(crate) fn equivocations(ledger: &[Parsed], findings: &mut Vec<Finding>) {
    let mut pairs: BTreeSet<(String, String)> = BTreeSet::new();
    let mut by_seq: BTreeMap<u64, Vec<&Parsed>> = BTreeMap::new();
    let mut by_prev: BTreeMap<&str, Vec<&Parsed>> = BTreeMap::new();
    for e in ledger {
        by_seq.entry(e.seq).or_default().push(e);
        if let Some(p) = &e.prev {
            by_prev.entry(p.as_str()).or_default().push(e);
        }
    }
    let seq_of: BTreeMap<&str, u64> = ledger.iter().map(|e| (e.id.as_str(), e.seq)).collect();
    for group in by_seq.values().chain(by_prev.values()) {
        for (x, a) in group.iter().enumerate() {
            for b in &group[x + 1..] {
                if a.id != b.id {
                    let (lo, hi) = if a.id.as_bytes() < b.id.as_bytes() { (&a.id, &b.id) } else { (&b.id, &a.id) };
                    pairs.insert((lo.clone(), hi.clone()));
                }
            }
        }
    }
    for (a, b) in pairs {
        let seq = seq_of[a.as_str()].min(seq_of[b.as_str()]);
        findings.push(Finding {
            name: EQUIVOCATION,
            position: seq,
            entry_id: a.clone(),
            second: b.clone(),
            hard: true,
            args: vec![("seq", n(seq)), ("a", s(&a)), ("b", s(&b))],
        });
    }
}

/// §8.4 carried records: a byte string carried (§8.1) by at least one record whose `verdict` is `counted` and which
/// the §8.1 trim keeps (`kept`: its sender in the lineage and not retired at it), that either fails §4.3 (`token` is
/// the one §4.3's order names, `seq` is 0) or passes with a `judge` outside the lineage (`token` is [`FOREIGN`], `seq`
/// is the entry's). One per byte string; the two branches exclude each other; a carried string that is a ledger entry
/// records nothing here.
#[cfg(not(feature = "ablate-k1-carried-malformed"))]
fn carried_malformed(kept: &[&Record], excluded: &[Parsed], malformed: &[(String, &'static str)], findings: &mut Vec<Finding>) {
    let charged: BTreeSet<&[u8]> = kept.iter().filter(|r| r.verdict == Verdict::Counted).filter_map(|r| r.entry.as_deref()).collect();
    for b in charged {
        let id = hexform::encode(&cryptox::sha256(b));
        let (seq, token) = match (malformed.iter().find(|(m, _)| *m == id), excluded.iter().find(|e| e.id == id)) {
            (Some((_, t)), _) => (0, *t),
            (None, Some(e)) => (e.seq, FOREIGN),
            (None, None) => continue,
        };
        findings.push(Finding {
            name: CARRIED_MALFORMED,
            position: seq,
            entry_id: id,
            second: String::new(),
            hard: true,
            args: vec![("seq", n(seq)), ("token", s(token))],
        });
    }
}

/// Fault-injection variant for the conformance gauges: carried byte strings are never recorded.
#[cfg(feature = "ablate-k1-carried-malformed")]
fn carried_malformed(kept: &[&Record], excluded: &[Parsed], malformed: &[(String, &'static str)], findings: &mut Vec<Finding>) {
    let _ = (kept, excluded, malformed, findings);
}

fn record_row(r: &Record, with_timestamp: bool) -> Value {
    let mut m = vec![("chainId", n(r.chain_id)), ("blockNumber", n(r.block_number))];
    if with_timestamp {
        m.push(("blockTimestamp", n(r.block_timestamp)));
    }
    m.push(("tx", s(&r.tx)));
    m.push(("sender", s(&r.sender)));
    obj(m)
}

/// The bytes member of a report row, spelled as §9.2 writes an `entry` in a report: `0x` and lowercase digits.
fn entry_member(r: &Record) -> Option<(&'static str, Value)> {
    r.entry.as_ref().map(|e| ("entry", s(&hexform::encode(e))))
}

fn anchors_of<'a>(records: &[&'a Record], hash: &str) -> Vec<&'a Record> {
    let mut v: Vec<&Record> = records.iter().copied().filter(|r| r.hash == hash).collect();
    v.sort_by(|a, b| (a.chain_id, a.block_number, a.tx.as_bytes()).cmp(&(b.chain_id, b.block_number, b.tx.as_bytes())));
    v
}

/// §8.1 the carried byte strings: the `entry` of every record of the anchor set that carries one, whatever the
/// record's `sender` and `verdict`, byte-identical copies one. The audit reads this set and nothing beside it, and so
/// does every reader that must decide over the same bytes the audit decides over; none rebuilds it.
pub fn carried(input: &Input) -> BTreeSet<&[u8]> {
    input.anchors.iter().filter_map(|r| r.entry.as_deref()).collect()
}

/// The twelve contents of §8.7, thirteen members: the report object (member order from the canonical writer).
pub fn audit(input: &Input) -> Value {
    // §8.1: every carried byte string is tested by §4.3; those that fail are MALFORMED, their entry_id computed all
    // the same. The lineage is the fixed point over the ones that pass.
    let (entries, mut malformed) = tested(carried(input));
    let lineage = lineage(&input.root, &entries, &input.anchors);
    let retire = retirements(&entries, &input.anchors);
    let by_bytes = signed_by_bytes(&entries);
    // The trim (§8.1): a record whose sender is outside the lineage, or at which its sender has retired (§7.4, asked of
    // the record's `sender` with what the record carries), is discarded, and stands in DISCARDED.
    let (kept, mut discarded): (Vec<&Record>, Vec<&Record>) =
        input.anchors.iter().partition(|r| lineage.contains(&r.sender) && !has_retired(&retire, &r.sender, r.block_timestamp, carried_by(&by_bytes, r)));
    let counted: Vec<&Record> = kept.iter().copied().filter(|r| r.verdict == Verdict::Counted).collect();
    let counted_hashes: BTreeSet<&str> = counted.iter().map(|r| r.hash.as_str()).collect();
    // A byte string a counted undiscarded record carries: its sha256 is that record's hash (§9.2).
    let counted_carries: BTreeSet<&str> = counted.iter().filter(|r| r.entry.is_some()).map(|r| r.hash.as_str()).collect();
    // §8.1, §8.6: the ledger is the carried entries whose judge is in the lineage and which a counted undiscarded
    // record carries; a carried entry whose judge is outside the lineage is excluded, whatever carried it.
    let (in_lineage, excluded): (Vec<Parsed>, Vec<Parsed>) = entries.into_iter().partition(|e| lineage.contains(&e.judge));
    let mut ledger: Vec<Parsed> = in_lineage.into_iter().filter(|e| counted_carries.contains(e.id.as_str())).collect();
    ledger.sort_by(|a, b| (a.seq, a.id.as_bytes()).cmp(&(b.seq, b.id.as_bytes())));

    let mut findings: Vec<Finding> = Vec::new();
    walk(&input.root, &ledger, &mut findings);
    equivocations(&ledger, &mut findings);
    carried_malformed(&kept, &excluded, &malformed, &mut findings);
    let findings = in_report_order(findings);
    let any_hard = findings.iter().any(|f| f.hard);
    let any_gap = findings.iter().any(|f| f.name == SEQ_GAP);

    // §8.5: every counted hash stands in `have` (ANCHORED) or in MISSING, and in exactly one of them.
    let have: BTreeSet<&str> = ledger.iter().map(|e| e.id.as_str()).collect();
    let missing: Vec<Value> = counted_hashes
        .iter()
        .filter(|h| !have.contains(*h))
        .map(|h| obj(vec![("hash", s(h)), ("anchors", Value::Arr(anchors_of(&counted, h).into_iter().map(|r| record_row(r, false)).collect()))]))
        .collect();
    let anchored: Vec<Value> = have
        .iter()
        .map(|id| obj(vec![("entry_id", s(id)), ("anchors", Value::Arr(anchors_of(&counted, id).into_iter().map(|r| record_row(r, true)).collect()))]))
        .collect();
    let mut excluded_rows: Vec<&Parsed> = excluded.iter().collect();
    excluded_rows.sort_by(|a, b| (a.seq, a.id.as_bytes()).cmp(&(b.seq, b.id.as_bytes())));
    let mut unknown: Vec<&Parsed> = ledger.iter().filter(|e| !body::KNOWN_TYPES.contains(&e.entry_type.as_str())).collect();
    unknown.sort_by(|a, b| (a.seq, a.entry_type.as_bytes(), a.id.as_bytes()).cmp(&(b.seq, b.entry_type.as_bytes(), b.id.as_bytes())));
    malformed.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    // Items 9 and 10: the undiscarded records of one verdict, the bytes beside them where the record carries any.
    let verdict_rows = |v: Verdict| -> Vec<Value> {
        let mut rows: Vec<&Record> = kept.iter().copied().filter(|r| r.verdict == v).collect();
        rows.sort_by(|a, b| (a.hash.as_bytes(), a.chain_id, a.block_number, a.tx.as_bytes()).cmp(&(b.hash.as_bytes(), b.chain_id, b.block_number, b.tx.as_bytes())));
        rows.into_iter()
            .map(|r| {
                let mut m = vec![("hash", s(&r.hash)), ("chainId", n(r.chain_id)), ("blockNumber", n(r.block_number)), ("tx", s(&r.tx)), ("sender", s(&r.sender))];
                m.extend(entry_member(r));
                obj(m)
            })
            .collect()
    };
    discarded.sort_by(|a, b| (a.sender.as_bytes(), a.chain_id, a.block_number, a.tx.as_bytes(), a.hash.as_bytes()).cmp(&(b.sender.as_bytes(), b.chain_id, b.block_number, b.tx.as_bytes(), b.hash.as_bytes())));

    // Item 12: an UNPROVEN record whose hash is no counted undiscarded record's hash moves the label, so that a node
    // that could not answer for a boundary is never read as the seat's silence.
    let unproven_open = kept.iter().any(|r| r.verdict == Verdict::Unproven && !counted_hashes.contains(r.hash.as_str()));
    let label = if any_hard {
        LABEL_BROKEN_CHAIN
    } else if unproven_open {
        LABEL_UNPROVEN
    } else if any_gap || !missing.is_empty() {
        LABEL_GAPS
    } else {
        LABEL_COMPLETE
    };

    obj(vec![
        ("root", s(&input.root)),
        ("basis", input.basis.clone()),
        ("entries", n(ledger.len() as u64)),
        ("findings", Value::Arr(findings.into_iter().map(Finding::value).collect())),
        ("missing", Value::Arr(missing)),
        ("anchored", Value::Arr(anchored)),
        ("excluded", Value::Arr(excluded_rows.into_iter().map(|e| obj(vec![("seq", n(e.seq)), ("judge", s(&e.judge)), ("entry_id", s(&e.id))])).collect())),
        ("unknown_type", Value::Arr(unknown.into_iter().map(|e| obj(vec![("seq", n(e.seq)), ("entryType", s(&e.entry_type)), ("entry_id", s(&e.id))])).collect())),
        ("malformed", Value::Arr(malformed.into_iter().map(|(id, t)| obj(vec![("entry_id", s(&id)), ("token", s(t))])).collect())),
        ("unproven", Value::Arr(verdict_rows(Verdict::Unproven))),
        ("void", Value::Arr(verdict_rows(Verdict::Void))),
        ("discarded", Value::Arr(discarded.into_iter().map(|r| {
            // Item 11: the seven required members, and the bytes member where the record carries one.
            let mut m = vec![
                ("chainId", n(r.chain_id)), ("blockNumber", n(r.block_number)), ("blockTimestamp", n(r.block_timestamp)),
                ("tx", s(&r.tx)), ("sender", s(&r.sender)), ("hash", s(&r.hash)), ("verdict", s(r.verdict.name())),
            ];
            m.extend(entry_member(r));
            obj(m)
        }).collect())),
        ("label", s(label)),
    ])
}

/// Whether this report finds the ledger broken (§8.7 label `BROKEN_CHAIN`). This is the one place that
/// decides it: the shell's write lock and the CLI exit gate both call it instead of comparing labels.
#[cfg(not(feature = "ablate-k1-broken-chain"))]
pub fn broken_chain(report: &Value) -> bool {
    report.get("label").and_then(Value::as_str) == Some(LABEL_BROKEN_CHAIN)
}

/// Fault-injection variant for the conformance gauges: nothing is ever broken, so a ledger with a hard finding
/// keeps its writes open in the shell and passes the command line's exit gate.
#[cfg(feature = "ablate-k1-broken-chain")]
pub fn broken_chain(report: &Value) -> bool {
    let _ = report;
    false
}

pub fn no_label() -> Value {
    obj(vec![("ok", Value::Bool(false)), ("reason", s(NO_LABEL))])
}

/// Whether an answer face is this law's `NO_LABEL` (§8.7: the inputs are no audit input), the core's own reading of
/// its own outcome, so a reader above it asks this and compares no constant of this law.
pub fn is_no_label(face: &Value) -> bool {
    face.get("reason").and_then(Value::as_str) == Some(NO_LABEL)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json;
    use crate::testkit::{addr, entry, entry_under, id, input, key, record};

    fn run(v: &Value) -> Value {
        read_input(v).map_or_else(no_label, |i| audit(&i))
    }
    fn label(r: &Value) -> &str {
        r.get("label").and_then(Value::as_str).unwrap_or_default()
    }
    fn rows(r: &Value, k: &str) -> Vec<Value> {
        match r.get(k) {
            Some(Value::Arr(a)) => a.clone(),
            _ => panic!("{k} 不是表:{r:?}"),
        }
    }
    fn names(r: &Value) -> Vec<String> {
        rows(r, "findings").iter().map(|f| f.get("name").and_then(Value::as_str).unwrap().to_string()).collect()
    }
    fn hashes(r: &Value, k: &str, member: &str) -> Vec<String> {
        rows(r, k).iter().map(|x| x.get(member).and_then(Value::as_str).unwrap().to_string()).collect()
    }

    /// The report has exactly the thirteen members of §8.7, and a ledger whose every entry a counted record of the
    /// root carries is COMPLETE, each entry in ANCHORED.
    #[test]
    fn a_carried_ledger_reports_thirteen_members_and_complete() {
        let a = key(1);
        let root = addr(&a);
        let g = entry(&a, 0, None, "genesis", r#"{"statement_md":"g"}"#);
        let n1 = entry(&a, 1, Some(&id(&g)), "annotation", r#"{"note_md":"one"}"#);
        let r = run(&input(&root, &[record(1, &root, "counted", &id(&g), Some(&g)), record(2, &root, "counted", &id(&n1), Some(&n1))], &[root.clone()]));
        let members: Vec<&str> = r.as_obj().unwrap().iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(members, ["root", "basis", "entries", "findings", "missing", "anchored", "excluded", "unknown_type", "malformed", "unproven", "void", "discarded", "label"]);
        assert_eq!(label(&r), LABEL_COMPLETE);
        assert_eq!(r.get("entries"), Some(&Value::Int(2)));
        let mut both = vec![id(&g), id(&n1)];
        both.sort();
        assert_eq!(hashes(&r, "anchored", "entry_id"), both);
    }

    /// Bytes a reader holds beside the chain are not read: an input that still names a `pile` audits only what the
    /// records carry, so a seat whose entries were never anchored reads as having none (§8, two inputs). A basis that
    /// still declares `bareTx`, and a record whose `entry` is `null`, are no audit input.
    #[test]
    fn nothing_beside_the_chain_is_read_and_the_retired_shapes_have_no_label() {
        let a = key(1);
        let root = addr(&a);
        let g = entry(&a, 0, None, "genesis", r#"{"statement_md":"g"}"#);
        let mut v = input(&root, &[], &[root.clone()]);
        if let Value::Obj(m) = &mut v {
            m.push(("pile".into(), Value::Arr(vec![Value::Str(hexform::encode(&g))])));
            m.push(("unavailable".into(), Value::Arr(vec![])));
        }
        let r = run(&v);
        assert_eq!(r.get("entries"), Some(&Value::Int(0)), "本机字节不入审计");
        assert_eq!(label(&r), LABEL_COMPLETE);
        let bare = json::read_open(format!(r#"{{"root":"{root}","anchors":[],"basis":{{"bareTx":[],"chains":[]}}}}"#).as_bytes()).unwrap();
        assert_eq!(run(&bare), no_label(), "基准只一员 chains");
        let null_entry = record(1, &root, "counted", &id(&g), None).replace(r#""verdict":"counted""#, r#""verdict":"counted","entry":null"#);
        assert_eq!(run(&input(&root, &[null_entry], &[root.clone()])), no_label(), "entry 为 null 在形外");
        let upper = record(1, &root, "counted", &id(&g), Some(&g)).replace(r#""entry":"0x"#, r#""entry":"0X"#);
        assert_eq!(run(&input(&root, &[upper], &[root.clone()])), no_label(), "0X 拼法在形外");
    }

    /// §8.5: a counted hash with no ledger entry behind it is MISSING, from each of its four sources: no bytes
    /// carried; bytes that are no entry; an entry outside the lineage; bytes carried only by a record that did not
    /// count. A judge whose anchor carried nothing, or carried what is not this ledger's, reads the gap, never a
    /// clean ledger.
    #[test]
    fn missing_has_four_sources_and_each_moves_the_label() {
        let (a, s) = (key(1), key(9));
        let root = addr(&a);
        let g = entry(&a, 0, None, "genesis", r#"{"statement_md":"g"}"#);
        let n1 = entry(&a, 1, Some(&id(&g)), "annotation", r#"{"note_md":"one"}"#);
        let stranger = entry(&s, 0, None, "genesis", r#"{"statement_md":"s"}"#);
        let base = record(1, &root, "counted", &id(&g), Some(&g));
        let junk = b"not an entry".to_vec();
        for (what, recs, expect_label) in [
            ("无字节", vec![base.clone(), record(2, &root, "counted", &id(&n1), None)], LABEL_GAPS),
            ("字节非条目", vec![base.clone(), record(2, &root, "counted", &id(&junk), Some(&junk))], LABEL_BROKEN_CHAIN),
            ("世系外的条目", vec![base.clone(), record(2, &root, "counted", &id(&stranger), Some(&stranger))], LABEL_BROKEN_CHAIN),
            ("字节只在作废的记录上", vec![base.clone(), record(2, &root, "counted", &id(&n1), None), record(3, &root, "VOID", &id(&n1), Some(&n1))], LABEL_GAPS),
        ] {
            let r = run(&input(&root, &recs, &[root.clone()]));
            assert_eq!(rows(&r, "missing").len(), 1, "{what}:{r:?}");
            assert_eq!(label(&r), expect_label, "{what}");
            assert_eq!(r.get("entries"), Some(&Value::Int(1)), "{what}");
        }
    }

    /// §8.1, §8.6: a succession carried only by a record that did not count extends the lineage, so the new key's
    /// records are not discarded, and moves no authority, so the new key's entries are graded against the root and
    /// recorded AUTHORITY_MISMATCH. A seat whose handover anchor was voided reads why, never a clean handover.
    #[test]
    fn a_succession_no_counted_record_carries_extends_the_lineage_and_moves_nothing() {
        let (a, b) = (key(1), key(2));
        let (root, to) = (addr(&a), addr(&b));
        let g = entry(&a, 0, None, "genesis", r#"{"statement_md":"g"}"#);
        let hand = entry(&a, 1, Some(&id(&g)), "succession", &format!(r#"{{"to":"{to}","kind":"handover","effective":0,"statement_md":""}}"#));
        let next = entry(&b, 2, Some(&id(&hand)), "annotation", r#"{"note_md":"the new key"}"#);
        let recs = [record(1, &root, "counted", &id(&g), Some(&g)), record(2, &root, "VOID", &id(&hand), Some(&hand)), record(3, &to, "counted", &id(&next), Some(&next))];
        let r = run(&input(&root, &recs, &[root.clone(), to.clone()]));
        assert!(rows(&r, "discarded").is_empty(), "新钥在世系里,其记录不丢");
        assert_eq!(hashes(&r, "void", "hash"), vec![id(&hand)]);
        assert_eq!(rows(&r, "void")[0].get("entry"), Some(&Value::Str(hexform::encode(&hand))), "作废记录带字节处并 entry");
        assert_eq!(names(&r), vec!["AUTHORITY_MISMATCH", "SEQ_GAP"], "{r:?}");
        assert_eq!(label(&r), LABEL_GAPS, "缺位之后的越权是软的");
    }

    /// §12.3: a key of the lineage that carries a new-spec entry into this ledger's inputs is charged: the bytes fail
    /// §4.3 at E_SPEC, stand in MALFORMED and MISSING, and CARRIED_MALFORMED labels the ledger BROKEN_CHAIN. A reader
    /// auditing past a migration is told so, never shown a clean old ledger.
    #[test]
    fn a_new_spec_entry_carried_past_a_succession_breaks_the_chain() {
        let (a, b) = (key(1), key(2));
        let (root, to) = (addr(&a), addr(&b));
        let g = entry(&a, 0, None, "genesis", r#"{"statement_md":"g"}"#);
        let hand = entry(&a, 1, Some(&id(&g)), "succession", &format!(r#"{{"to":"{to}","kind":"handover","effective":0,"statement_md":""}}"#));
        let fresh = entry_under(&b, "galeed/2", 0, None, "genesis", r#"{"statement_md":"new spec"}"#);
        let recs = [record(1, &root, "counted", &id(&g), Some(&g)), record(2, &root, "counted", &id(&hand), Some(&hand)), record(3, &to, "counted", &id(&fresh), Some(&fresh))];
        let r = run(&input(&root, &recs, &[root.clone(), to.clone()]));
        assert_eq!(names(&r), vec!["CARRIED_MALFORMED"]);
        assert_eq!(rows(&r, "findings")[0].get("token"), Some(&Value::Str("E_SPEC".into())));
        assert_eq!(hashes(&r, "malformed", "entry_id"), vec![id(&fresh)]);
        assert_eq!(hashes(&r, "missing", "hash"), vec![id(&fresh)]);
        assert_eq!(label(&r), LABEL_BROKEN_CHAIN);
        // A basis whose `senders` leaves the successor out reads the old ledger alone.
        let narrow = run(&input(&root, &recs[..2], &[root.clone()]));
        assert_eq!(label(&narrow), LABEL_COMPLETE);
    }

    /// §7.4, §8.1: a key retires at the earliest record carrying a succession it signed; its records after that
    /// block time are discarded, and a second succession it signs and has carried late extends nothing.
    #[test]
    fn a_retired_keys_later_records_are_discarded_and_extend_nothing() {
        let (a, b, c) = (key(1), key(2), key(3));
        let (root, to, late_to) = (addr(&a), addr(&b), addr(&c));
        let g = entry(&a, 0, None, "genesis", r#"{"statement_md":"g"}"#);
        let hand = entry(&a, 1, Some(&id(&g)), "succession", &format!(r#"{{"to":"{to}","kind":"keyrotation","effective":0,"statement_md":""}}"#));
        let after = entry(&b, 2, Some(&id(&hand)), "annotation", r#"{"note_md":"b"}"#);
        let stale = entry(&a, 2, Some(&id(&hand)), "annotation", r#"{"note_md":"stolen"}"#);
        let late = entry(&a, 1, Some(&id(&g)), "succession", &format!(r#"{{"to":"{late_to}","kind":"handover","effective":0,"statement_md":""}}"#));
        let recs = [
            record(1, &root, "counted", &id(&g), Some(&g)),
            record(2, &root, "counted", &id(&hand), Some(&hand)),
            record(3, &to, "counted", &id(&after), Some(&after)),
            record(4, &root, "counted", &id(&stale), Some(&stale)),
            record(5, &root, "counted", &id(&late), Some(&late)),
        ];
        let r = run(&input(&root, &recs, &[root.clone(), to.clone(), late_to.clone()]));
        assert_eq!(label(&r), LABEL_COMPLETE, "{r:?}");
        assert_eq!(hashes(&r, "anchored", "entry_id").len(), 3);
        let mut gone = hashes(&r, "discarded", "hash");
        gone.sort();
        let mut want = vec![id(&stale), id(&late)];
        want.sort();
        assert_eq!(gone, want);
        let lineage = lineage_of(&read_input(&input(&root, &recs, &[root.clone(), to.clone(), late_to.clone()])).unwrap());
        assert!(lineage.contains(&to) && !lineage.contains(&late_to), "迟到的继任不入世系");
        assert!(lineage_local(&root, &[json::read(&late).unwrap(), json::read(&hand).unwrap()]).contains(&late_to), "本地超集不看时刻");
    }

    /// §6.6a, §8.2: a migration signed by the key in office closes the ledger, and the migration itself is a known
    /// type. §7.4: carried in the migration's own second, an entry the key signs at a later position rides a record at
    /// which the key has retired: the record is discarded and the ledger stays whole (corpus a558).
    #[test]
    fn a_migration_closes_the_ledger() {
        let a = key(1);
        let root = addr(&a);
        let g = entry(&a, 0, None, "genesis", r#"{"statement_md":"g"}"#);
        let mig = entry(&a, 1, Some(&id(&g)), "migration", r#"{"spec":"galeed/2","statement_md":"moving"}"#);
        let past = entry(&a, 2, Some(&id(&mig)), "annotation", r#"{"note_md":"after"}"#);
        let mut recs = vec![record(1, &root, "counted", &id(&g), Some(&g)), record(2, &root, "counted", &id(&mig), Some(&mig))];
        let closed = run(&input(&root, &recs, &[root.clone()]));
        assert_eq!(label(&closed), LABEL_COMPLETE);
        assert!(rows(&closed, "unknown_type").is_empty(), "迁法是第八种已知条目");
        recs.push(record(2, &root, "counted", &id(&past), Some(&past)));
        let r = run(&input(&root, &recs, &[root.clone()]));
        assert!(names(&r).is_empty(), "{r:?}");
        assert_eq!(hashes(&r, "discarded", "hash"), vec![id(&past)], "同秒、迁法之后的位置:该记录丢弃");
        assert_eq!(hashes(&r, "anchored", "entry_id").len(), 2);
        assert_eq!(label(&r), LABEL_COMPLETE);
        assert!(crate::body::first_bad("migration", &json::read(br#"{"statement_md":"x"}"#).unwrap()).is_some(), "spec 缺即体表错");
    }

    /// One record on chain 1 at block `n` and block time `t`, transaction word `n` (records of one second sent by
    /// different keys ride different transactions).
    fn record_at(n: u64, t: u64, sender: &str, verdict: &str, hash: &str, carried: Option<&[u8]>) -> String {
        record(n, sender, verdict, hash, carried).replacen(&format!(r#""blockTimestamp":{n}"#), &format!(r#""blockTimestamp":{t}"#), 1)
    }

    /// §7.4 retiring seq: two hand-offs of one key carried in one second, the retiring seq is the smaller `seq` of
    /// the two, whatever record carries it and whatever its verdict. A stray succession at seq 1 on a `VOID` record
    /// pulls the retiring seq below the real one at seq 2: the record carrying the real one rides a record at which the
    /// key has retired, so it is discarded and names nobody, while the stray one, at the retiring seq, is read as at any
    /// earlier record. Victim: a reader who took the larger seq and let the ceding key's later signatures of that second
    /// stand. Expectation from §7.4 L902-917 and `glaudit.retirements` / `retired_at`.
    #[test]
    fn a_lower_stray_hand_off_in_the_same_second_lowers_the_retiring_seq() {
        let (a, b, c) = (key(1), key(2), key(3));
        let (root, to, stray_to) = (addr(&a), addr(&b), addr(&c));
        let g = entry(&a, 0, None, "genesis", r#"{"statement_md":"g"}"#);
        let n1 = entry(&a, 1, Some(&id(&g)), "annotation", r#"{"note_md":"one"}"#);
        let stray = entry(&a, 1, Some(&id(&g)), "succession", &format!(r#"{{"to":"{stray_to}","kind":"handover","effective":0,"statement_md":""}}"#));
        let hand = entry(&a, 2, Some(&id(&n1)), "succession", &format!(r#"{{"to":"{to}","kind":"keyrotation","effective":0,"statement_md":""}}"#));
        let recs = [
            record_at(1, 1, &root, "counted", &id(&g), Some(&g)),
            record_at(2, 2, &root, "counted", &id(&n1), Some(&n1)),
            record_at(5, 5, &root, "counted", &id(&hand), Some(&hand)),
            record_at(6, 5, &root, "VOID", &id(&stray), Some(&stray)),
        ];
        let v = input(&root, &recs, &[root.clone(), to.clone(), stray_to.clone()]);
        let i = read_input(&v).unwrap();
        let (entries, _) = tested(carried(&i));
        assert_eq!(retirements(&entries, &i.anchors).get(&root), Some(&Retirement { at: 5, seq: 1 }), "退任位取同秒两交接中较小的 seq");
        let r = run(&v);
        assert_eq!(hashes(&r, "discarded", "hash"), vec![id(&hand)], "{r:?}");
        let lineage = lineage_of(&i);
        assert!(!lineage.contains(&to) && lineage.contains(&stray_to), "退任位之后所签的继任不接纳任何人;退任位上那一条照早先记录读");
        assert_eq!(hashes(&r, "anchored", "entry_id").len(), 2);
        assert_eq!(label(&r), LABEL_COMPLETE);
    }

    /// §7.4: in the second of its handing off, a record another key sends is read against that key's own retirement;
    /// what the ceding key signed at a position before its handing off counts when the successor carries it there
    /// (§5.7, §8.6), and a succession it signed at a position after it admits nobody whichever record carries it. The
    /// successor's record of the stale succession is kept (the successor has not retired), so the stale entry is a
    /// ledger entry signed out of office (§8.2). Victims: the successor whose re-anchored earlier entry a reader dropped,
    /// and the seat a ceding key's late succession named a stranger into. Expectation from §7.4 L906-917, L926-927 and
    /// `glaudit.retired_at` / `_lineage`.
    #[test]
    fn a_successor_carrying_in_the_hand_off_second_counts_what_was_signed_before_it_only() {
        let (a, b, c) = (key(1), key(2), key(3));
        let (root, to, late_to) = (addr(&a), addr(&b), addr(&c));
        let g = entry(&a, 0, None, "genesis", r#"{"statement_md":"g"}"#);
        let n1 = entry(&a, 1, Some(&id(&g)), "annotation", r#"{"note_md":"before"}"#);
        let hand = entry(&a, 2, Some(&id(&n1)), "succession", &format!(r#"{{"to":"{to}","kind":"keyrotation","effective":0,"statement_md":""}}"#));
        let late = entry(&a, 3, Some(&id(&hand)), "succession", &format!(r#"{{"to":"{late_to}","kind":"handover","effective":0,"statement_md":""}}"#));
        let recs = [
            record_at(1, 1, &root, "counted", &id(&g), Some(&g)),
            record_at(5, 5, &root, "counted", &id(&hand), Some(&hand)),
            record_at(6, 5, &to, "counted", &id(&n1), Some(&n1)),
            record_at(7, 5, &to, "counted", &id(&late), Some(&late)),
        ];
        let v = input(&root, &recs, &[root.clone(), to.clone(), late_to.clone()]);
        let r = run(&v);
        assert!(rows(&r, "discarded").is_empty(), "继任者所发记录按它自己的退任读:{r:?}");
        let anchored = hashes(&r, "anchored", "entry_id");
        assert!(anchored.contains(&id(&n1)), "交接位之前所签、继任者同秒承载:照计");
        let lineage = lineage_of(&read_input(&v).unwrap());
        assert!(lineage.contains(&to) && !lineage.contains(&late_to), "交接位之后所签的继任,由谁承载都不接纳");
        assert_eq!(names(&r), vec!["AUTHORITY_MISMATCH"], "继任者承载的越位条目是账本条目,签者不在任");
        assert_eq!(label(&r), LABEL_BROKEN_CHAIN);
    }

    /// §8.7 item 12: UNPROVEN moves the label only where no counted record carries the same hash, and only below a
    /// hard finding; above GAPS, so a reader whose node could not answer never reads that silence as a gap of the seat.
    #[test]
    fn the_label_order_is_broken_unproven_gaps_complete() {
        let a = key(1);
        let root = addr(&a);
        let g = entry(&a, 0, None, "genesis", r#"{"statement_md":"g"}"#);
        let n1 = entry(&a, 1, Some(&id(&g)), "annotation", r#"{"note_md":"one"}"#);
        let n3 = entry(&a, 3, Some(&id(&n1)), "annotation", r#"{"note_md":"three"}"#);
        let twin = entry(&a, 1, Some(&id(&g)), "annotation", r#"{"note_md":"twin"}"#);
        let c = |n: u64, b: &[u8]| record(n, &root, "counted", &id(b), Some(b));
        let open = record(9, &root, "UNPROVEN", &id(&n1), Some(&n1));
        let with = |mut v: Vec<String>, extra: &str| {
            v.push(extra.to_string());
            run(&input(&root, &v, &[root.clone()]))
        };
        assert_eq!(label(&with(vec![c(1, &g)], &open)), LABEL_UNPROVEN, "未证、无计入记录");
        assert_eq!(label(&with(vec![c(1, &g), c(2, &n1)], &open)), LABEL_COMPLETE, "同 hash 已有计入记录即不移标签");
        assert_eq!(label(&with(vec![c(1, &g), c(3, &n3)], &open)), LABEL_UNPROVEN, "未证在缺位之上");
        assert_eq!(label(&with(vec![c(1, &g), c(2, &n1), c(3, &twin)], &record(9, &root, "UNPROVEN", &id(&n3), Some(&n3)))), LABEL_BROKEN_CHAIN, "硬发现在未证之上");
    }
}
