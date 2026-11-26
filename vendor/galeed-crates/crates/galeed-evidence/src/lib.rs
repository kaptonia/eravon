//! The `courtevidence/1` core.
//!
//! The law's predicates: §2 manifest judgment (decision order, index-carrying file-item tokens, the optional
//! `zikaron` member) and `manifest(b)`; the §3.2 recipient form; §4 recipient selection from an audit input the
//! producer scanned (`unread` where the input is no audit input; the bindings are the `keybinding` ledger entries the
//! report's anchored rows name; `one` / `none` / `candidates`, reasons `fork`, `gap`, `broken` in that order, the
//! gap naming the holes the producer can still fetch). The producer's obligations of §3 are kept here in one place
//! each: a file is listed by its plaintext bytes ([`FileItem::listed`], [`seal`]), its name spelled in the skeleton
//! charset or refused ([`file_name`]); evidence is encrypted to a binding the pick answered and to nothing else
//! ([`seal`]); a recipient outside §3.2 or of small order is refused loudly and nothing is encrypted, never sent in
//! the clear ([`usable`]). The harness commands live in [`harness`] and [`answer`].
#![forbid(unsafe_code)]

pub mod answer;
pub mod harness;

use galeed::audit::{self, Input};
use galeed::body;
use galeed::cryptox;
use galeed::envelope;
use galeed::hexform;
use galeed::json::{self, Value};
use galeed::token::Token;
use std::collections::BTreeSet;

pub const SPEC: &str = "courtevidence/1";

/// Manifest member names (§2.1, §2.2) and keybinding body member names (`galeed/1` §6.7).
pub mod key {
    pub const SPEC: &str = "spec";
    pub const SUBJECT: &str = "subjectRef";
    pub const PARTY: &str = "party";
    pub const FILES: &str = "files";
    pub const NOTE: &str = "note_md";
    pub const NAME: &str = "name";
    pub const SHA256: &str = "sha256";
    pub const BYTES: &str = "bytes";
    pub const MEDIA_TYPE: &str = "mediaType";
    pub const ZIKARON: &str = "zikaron";
    pub const ROOT: &str = "root";
    pub const CHAIN_ID: &str = "chainId";
    pub const TX: &str = "tx";
    pub const ENTRY: &str = "entry";
    pub const SEQ: &str = "seq";
    pub const RECIPIENT: &str = "recipient";
    pub const EFFECTIVE: &str = "effective";
    pub const ENTRY_ID: &str = "entry_id";
    pub const POSITION: &str = "position";
    pub const HARD: &str = "hard";
    pub const FINDINGS: &str = "findings";
    pub const EXCLUDED: &str = "excluded";
    pub const MALFORMED: &str = "malformed";
    pub const MISSING: &str = "missing";
    pub const UNPROVEN: &str = "unproven";
    pub const ANCHORED: &str = "anchored";
    pub const HASH: &str = "hash";
    pub const EXPECTED: &str = "expected";
    pub const ACTUAL: &str = "actual";
}

const REQUIRED: [&str; 4] = [key::SPEC, key::SUBJECT, key::PARTY, key::FILES];
const FILE_MEMBERS: [&str; 4] = [key::NAME, key::SHA256, key::BYTES, key::MEDIA_TYPE];
const ZIKARON_MEMBERS: [&str; 4] = [key::ROOT, key::CHAIN_ID, key::TX, key::ENTRY];

/// §2.2 `zikaron`: where a file's bytes are recorded in a ZIKARON author's ledger: the root that ledger is read
/// under, the chain and the transaction that anchored the recording entry, and that entry's bytes (lowercase `0x`
/// hex). This law tests the four forms and nothing of their meaning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Zikaron {
    pub root: String,
    pub chain_id: u64,
    pub tx: String,
    pub entry: String,
}

/// One §2.2 file item. **Its digest and length are computed from the bytes it lists, in one place**
/// ([`FileItem::listed`], and [`seal`] for an encrypted file, which lists the plaintext): no caller writes a digest of
/// its own, so a manifest never lists a ciphertext's digest or a length that is not its bytes'.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileItem {
    name: String,
    sha256: String,
    bytes: u64,
    media_type: String,
    zikaron: Option<Zikaron>,
}

impl FileItem {
    /// The item that lists `bytes` (the file's plaintext) under `name` and `media_type`: the name passes
    /// [`file_name`] or the file is not listed.
    pub fn listed(name: &str, bytes: &[u8], media_type: &str) -> Result<FileItem, Unlisted> {
        file_name(name)?;
        if media_type.is_empty() || !json::is_skeleton(media_type) {
            return Err(Unlisted::MediaType);
        }
        Ok(FileItem { name: name.to_string(), sha256: hexform::encode(&cryptox::sha256(bytes)), bytes: bytes.len() as u64, media_type: media_type.to_string(), zikaron: None })
    }

    /// The same item, pointing at a ZIKARON recording of the same bytes (§2.2).
    pub fn with_zikaron(mut self, z: Zikaron) -> FileItem {
        self.zikaron = Some(z);
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
    pub fn media_type(&self) -> &str {
        &self.media_type
    }
    pub fn zikaron(&self) -> Option<&Zikaron> {
        self.zikaron.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub subject_ref: Value,
    pub party: String,
    pub files: Vec<FileItem>,
    pub note_md: Option<String>,
}

/// Why a file is not listed (§2.4: a tool spells a name in the skeleton charset or refuses to list the file). Named
/// at the moment the file is added, never as a whole manifest's failure at the last step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unlisted {
    /// The name is empty.
    NameEmpty,
    /// The name carries a scalar value outside U+0020 to U+007E (a Chinese file name among them).
    NameCharset,
    /// The name is no package-relative path: a `\` or `:` in it, an empty, `.` or `..` segment, or a leading `/`.
    NamePath,
    /// A segment is a desktop's own bookkeeping file (`.DS_Store`), which no recipient's tool reads as evidence.
    NameJunk,
    /// The media type is empty or outside the skeleton charset.
    MediaType,
}

impl Unlisted {
    pub const fn name(self) -> &'static str {
        match self {
            Unlisted::NameEmpty => "file-name-empty",
            Unlisted::NameCharset => "file-name-charset",
            Unlisted::NamePath => "file-name-path",
            Unlisted::NameJunk => "file-name-junk",
            Unlisted::MediaType => "file-media-type",
        }
    }
}

/// The segment names a desk writes beside a folder's files for itself and never as evidence.
pub const JUNK_SEGMENTS: [&str; 1] = [".DS_Store"];

/// §2.4 and §2.2: the name a producer lists a file under: a non-empty skeleton string (U+0020 to U+007E), a
/// package-relative path whose segments, split at `/`, are each non-empty and neither `.` nor `..`, carrying no `\`
/// and no `:` (a recipient unpacks the files by these names on any system), and no segment a desk keeps for itself.
/// **The one test**, run the moment a file is added.
#[cfg(not(feature = "ablate-k3-file-name"))]
pub fn file_name(name: &str) -> Result<(), Unlisted> {
    if name.is_empty() {
        return Err(Unlisted::NameEmpty);
    }
    if !json::is_skeleton(name) {
        return Err(Unlisted::NameCharset);
    }
    if name.contains(['\\', ':']) || name.split('/').any(|seg| seg.is_empty() || seg == "." || seg == "..") {
        return Err(Unlisted::NamePath);
    }
    if name.split('/').any(|seg| JUNK_SEGMENTS.contains(&seg)) {
        return Err(Unlisted::NameJunk);
    }
    Ok(())
}

/// Fault-injection variant for the conformance gauges: only emptiness is tested, so a Chinese file name is listed
/// and the whole manifest fails `E_VALUE_CHARSET` at the last step.
#[cfg(feature = "ablate-k3-file-name")]
pub fn file_name(name: &str) -> Result<(), Unlisted> {
    if name.is_empty() {
        return Err(Unlisted::NameEmpty);
    }
    Ok(())
}

/// The §5 manifest tokens. The eight file-item tokens carry an index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// The eight of `canonical(b)` (`galeed/1` §3.5).
    Canonical(Token),
    Manifest,
    Missing,
    Closed,
    Spec,
    Subject,
    Party,
    Files,
    Note,
    File(usize),
    FileMissing(usize),
    FileClosed(usize),
    FileName(usize),
    FileSha256(usize),
    FileBytes(usize),
    FileMediaType(usize),
    FileZikaron(usize),
}

impl Fault {
    /// The spelling in §5, verbatim.
    pub const fn name(&self) -> &'static str {
        match self {
            Fault::Canonical(t) => t.name(),
            Fault::Manifest => "E_MANIFEST",
            Fault::Missing => "E_MANIFEST_MISSING",
            Fault::Closed => "E_MANIFEST_CLOSED",
            Fault::Spec => "E_MANIFEST_SPEC",
            Fault::Subject => "E_MANIFEST_SUBJECT",
            Fault::Party => "E_MANIFEST_PARTY",
            Fault::Files => "E_MANIFEST_FILES",
            Fault::Note => "E_MANIFEST_NOTE",
            Fault::File(_) => "E_FILE",
            Fault::FileMissing(_) => "E_FILE_MISSING",
            Fault::FileClosed(_) => "E_FILE_CLOSED",
            Fault::FileName(_) => "E_FILE_NAME",
            Fault::FileSha256(_) => "E_FILE_SHA256",
            Fault::FileBytes(_) => "E_FILE_BYTES",
            Fault::FileMediaType(_) => "E_FILE_MEDIATYPE",
            Fault::FileZikaron(_) => "E_FILE_ZIKARON",
        }
    }

    pub const fn index(&self) -> Option<usize> {
        match self {
            Fault::File(i) | Fault::FileMissing(i) | Fault::FileClosed(i) | Fault::FileName(i) | Fault::FileSha256(i) | Fault::FileBytes(i) | Fault::FileMediaType(i) | Fault::FileZikaron(i) => Some(*i),
            _ => None,
        }
    }
}

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

fn zikaron_value(z: &Zikaron) -> Value {
    Value::Obj(vec![(key::CHAIN_ID.into(), Value::Int(z.chain_id)), (key::ENTRY.into(), s(&z.entry)), (key::ROOT.into(), s(&z.root)), (key::TX.into(), s(&z.tx))])
}

/// Construction: the canonical bytes of a manifest. Whether they pass [`check`] depends on the members' own forms; a
/// file size outside the galeed/1 integer universe has no canonical bytes and is refused (`E_NUMBER`).
pub fn build(m: &Manifest) -> Result<Vec<u8>, galeed::token::Token> {
    let files: Vec<Value> = m
        .files
        .iter()
        .map(|f| {
            let mut item = vec![(key::NAME.into(), s(&f.name)), (key::SHA256.into(), s(&f.sha256)), (key::BYTES.into(), Value::Int(f.bytes)), (key::MEDIA_TYPE.into(), s(&f.media_type))];
            if let Some(z) = &f.zikaron {
                item.push((key::ZIKARON.into(), zikaron_value(z)));
            }
            Value::Obj(item)
        })
        .collect();
    let mut members = vec![(key::SPEC.to_string(), s(SPEC)), (key::SUBJECT.to_string(), m.subject_ref.clone()), (key::PARTY.to_string(), s(&m.party)), (key::FILES.to_string(), Value::Arr(files))];
    if let Some(n) = &m.note_md {
        members.push((key::NOTE.to_string(), s(n)));
    }
    json::canonical(&Value::Obj(members))
}

/// §2.2 `entry`: `0x` followed by an even number of lowercase hexadecimal digits, `0x` alone the empty byte string.
fn lower_hex_bytes(v: &Value) -> bool {
    v.as_str().and_then(|t| t.strip_prefix("0x")).is_some_and(|d| d.len() % 2 == 0 && d.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
}

/// §2.2 `zikaron`: an object with exactly the four members, each in its form.
#[cfg(not(feature = "ablate-k3-zikaron"))]
fn zikaron_form(v: &Value) -> Option<Zikaron> {
    let m = v.as_obj()?;
    if m.len() != ZIKARON_MEMBERS.len() || ZIKARON_MEMBERS.iter().any(|k| v.get(k).is_none()) {
        return None;
    }
    let Some(Value::Int(chain_id)) = v.get(key::CHAIN_ID) else { return None };
    let root = v.get(key::ROOT).filter(|x| body::is_hex20(x))?.as_str()?.to_string();
    let tx = v.get(key::TX).filter(|x| body::is_hex32(x))?.as_str()?.to_string();
    let entry = v.get(key::ENTRY).filter(|x| lower_hex_bytes(x))?.as_str()?.to_string();
    Some(Zikaron { root, chain_id: *chain_id, tx, entry })
}

/// Fault-injection variant for the conformance gauges: any object is taken, so a pointer missing its transaction
/// or carrying an extra member is listed as a ZIKARON recording no tool can open.
#[cfg(feature = "ablate-k3-zikaron")]
fn zikaron_form(v: &Value) -> Option<Zikaron> {
    v.as_obj()?;
    let text = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
    let chain_id = match v.get(key::CHAIN_ID) {
        Some(Value::Int(n)) => *n,
        _ => 0,
    };
    let _ = (ZIKARON_MEMBERS, lower_hex_bytes as fn(&Value) -> bool);
    Some(Zikaron { root: text(key::ROOT), chain_id, tx: text(key::TX), entry: text(key::ENTRY) })
}

/// One §2.2 file item, in order: an object, the four members present, no members beyond the five, `name`,
/// `sha256`, `bytes`, `mediaType`, then `zikaron` where present. The character set was settled by `canonical(b)`, so
/// a skeleton row only adds non-empty.
fn file_item(i: usize, v: &Value) -> Result<FileItem, Fault> {
    let m = v.as_obj().ok_or(Fault::File(i))?;
    if FILE_MEMBERS.iter().any(|k| v.get(k).is_none()) {
        return Err(Fault::FileMissing(i));
    }
    if m.iter().any(|(k, _)| !FILE_MEMBERS.contains(&k.as_str()) && k != key::ZIKARON) {
        return Err(Fault::FileClosed(i));
    }
    let name = v.get(key::NAME).filter(|x| body::is_skeleton_nonempty(x)).and_then(Value::as_str).ok_or(Fault::FileName(i))?;
    let sha256 = v.get(key::SHA256).filter(|x| body::is_hex32(x)).and_then(Value::as_str).ok_or(Fault::FileSha256(i))?;
    let Some(Value::Int(bytes)) = v.get(key::BYTES) else { return Err(Fault::FileBytes(i)) };
    let media_type = v.get(key::MEDIA_TYPE).filter(|x| body::is_skeleton_nonempty(x)).and_then(Value::as_str).ok_or(Fault::FileMediaType(i))?;
    let zikaron = match v.get(key::ZIKARON) {
        None => None,
        Some(z) => Some(zikaron_form(z).ok_or(Fault::FileZikaron(i))?),
    };
    Ok(FileItem { name: name.to_string(), sha256: sha256.to_string(), bytes: *bytes, media_type: media_type.to_string(), zikaron })
}

/// The §2.3 decision order; the first fault answers.
#[cfg(not(feature = "ablate-k3-manifest"))]
pub fn check(b: &[u8]) -> Result<Manifest, Fault> {
    let (v, items) = manifest_root(b)?;
    let files = items.iter().enumerate().map(|(i, x)| file_item(i, x)).collect::<Result<Vec<_>, _>>()?;
    manifest_rest(&v, files)
}

/// Fault-injection variant for the conformance gauges: file-item tokens are all reported at index 0.
#[cfg(feature = "ablate-k3-manifest")]
pub fn check(b: &[u8]) -> Result<Manifest, Fault> {
    let (v, items) = manifest_root(b)?;
    let files = items.iter().map(|x| file_item(0, x)).collect::<Result<Vec<_>, _>>()?;
    manifest_rest(&v, files)
}

/// §2.3 up to and including `files` an array: the value and the file-item values.
fn manifest_root(b: &[u8]) -> Result<(Value, Vec<Value>), Fault> {
    let v = json::accept(b).map_err(Fault::Canonical)?;
    let m = v.as_obj().ok_or(Fault::Manifest)?;
    if REQUIRED.iter().any(|k| v.get(k).is_none()) {
        return Err(Fault::Missing);
    }
    if m.len() != REQUIRED.len() + usize::from(v.get(key::NOTE).is_some()) {
        return Err(Fault::Closed);
    }
    if v.get(key::SPEC).and_then(Value::as_str) != Some(SPEC) {
        return Err(Fault::Spec);
    }
    if !v.get(key::SUBJECT).is_some_and(body::subject_ref) {
        return Err(Fault::Subject);
    }
    if !v.get(key::PARTY).is_some_and(body::is_hex20) {
        return Err(Fault::Party);
    }
    let Some(Value::Arr(items)) = v.get(key::FILES) else { return Err(Fault::Files) };
    let items = items.clone();
    Ok((v, items))
}

/// §2.3 after the file items: `note_md` where present.
fn manifest_rest(v: &Value, files: Vec<FileItem>) -> Result<Manifest, Fault> {
    let note_md = match v.get(key::NOTE) {
        None => None,
        Some(Value::Str(n)) => Some(n.clone()),
        Some(_) => return Err(Fault::Note),
    };
    let subject_ref = v.get(key::SUBJECT).cloned().unwrap_or(Value::Null);
    let party = v.get(key::PARTY).and_then(Value::as_str).unwrap_or_default().to_string();
    Ok(Manifest { subject_ref, party, files, note_md })
}

/// §1 `manifest(b)`: the sha256 of bytes §2 accepts; no digest otherwise.
pub fn digest(b: &[u8]) -> Result<[u8; 32], Fault> {
    check(b)?;
    Ok(cryptox::sha256(b))
}

pub fn file_view(b: &[u8]) -> (u64, [u8; 32]) {
    (b.len() as u64, cryptox::sha256(b))
}

const HRP: &str = "age";
const BECH32_ALPHABET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
/// §3.2: a recipient is sixty-two characters.
pub const RECIPIENT_LEN: usize = 62;

fn polymod(values: impl Iterator<Item = u8>) -> u32 {
    const GEN: [u32; 5] = [0x3b6a_57b2, 0x2650_8e6d, 0x1ea1_19fa, 0x3d42_33dd, 0x2a14_62b3];
    let mut chk: u32 = 1;
    for v in values {
        let top = chk >> 25;
        chk = ((chk & 0x01ff_ffff) << 5) ^ u32::from(v);
        for (i, g) in GEN.iter().enumerate() {
            if (top >> i) & 1 == 1 {
                chk ^= g;
            }
        }
    }
    chk
}

/// The thirty-two key bytes when the §3.2 recipient form holds: sixty-two characters, all below U+007E,
/// `age` `1` plus fifty-eight Bech32 characters, the BIP-173 checksum (constant 1), the first fifty-two
/// 5-bit groups forming exactly thirty-two bytes with the last four padding bits zero, all lowercase.
#[cfg(not(feature = "ablate-k3-recipient-form"))]
pub fn recipient_key(r: &str) -> Option<[u8; 32]> {
    let b = r.as_bytes();
    if b.len() != RECIPIENT_LEN || !b.iter().all(|c| *c <= 0x7e) || !r.starts_with("age1") {
        return None;
    }
    let groups: Vec<u8> = b[HRP.len() + 1..].iter().map(|c| BECH32_ALPHABET.iter().position(|a| a == c).map(|p| p as u8)).collect::<Option<_>>()?;
    let expand = HRP.bytes().map(|c| c >> 5).chain(std::iter::once(0)).chain(HRP.bytes().map(|c| c & 31));
    if polymod(expand.chain(groups.iter().copied())) != 1 {
        return None;
    }
    key_bytes(&groups)
}

/// Fault-injection variant for the conformance gauges: the checksum is not verified.
#[cfg(feature = "ablate-k3-recipient-form")]
pub fn recipient_key(r: &str) -> Option<[u8; 32]> {
    let b = r.as_bytes();
    if b.len() != RECIPIENT_LEN || !b.iter().all(|c| *c <= 0x7e) || !r.starts_with("age1") {
        return None;
    }
    let groups: Vec<u8> = b[HRP.len() + 1..].iter().map(|c| BECH32_ALPHABET.iter().position(|a| a == c).map(|p| p as u8)).collect::<Option<_>>()?;
    let _ = polymod(std::iter::empty());
    key_bytes(&groups)
}

/// The first fifty-two 5-bit groups, most significant first, make 260 bits: 256 bits of key and four zero bits.
fn key_bytes(groups: &[u8]) -> Option<[u8; 32]> {
    let mut acc: u32 = 0;
    let mut bits = 0;
    let mut out = Vec::with_capacity(32);
    for g in &groups[..52] {
        acc = (acc << 5) | u32::from(*g);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    (bits == 4 && acc == 0).then(|| out.try_into().ok()).flatten()
}

/// The §3.2 predicate.
pub fn recipient_form(r: &str) -> bool {
    recipient_key(r).is_some()
}

/// One §4.1 binding: `(seq, entry_id, body.recipient, body.effective, recipientForm)`. **Only the pick makes one**
/// ([`pick`]): a binding is a judge's published recipient read from a ledger the producer scanned, and [`seal`]
/// encrypts to a binding and to nothing else (§3.3: a producer encrypts to the recipient §4 picks and to no other).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    seq: u64,
    entry_id: String,
    recipient: String,
    /// Carried out; no predicate reads it (§4.2).
    effective: u64,
    recipient_form: bool,
}

impl Binding {
    pub fn seq(&self) -> u64 {
        self.seq
    }
    pub fn entry_id(&self) -> &str {
        &self.entry_id
    }
    pub fn recipient(&self) -> &str {
        &self.recipient
    }
    pub fn effective(&self) -> u64 {
        self.effective
    }
    pub fn recipient_form(&self) -> bool {
        self.recipient_form
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Fork,
    Gap,
    Broken,
}

impl Reason {
    pub const fn name(self) -> &'static str {
        match self {
            Reason::Fork => "fork",
            Reason::Gap => "gap",
            Reason::Broken => "broken",
        }
    }
}

/// One hole of §4.2 rule 3, what the producer can still fetch (§4.2: "the gap names the fetch the producer can still
/// make"): a run of positions the scan found no entry at, or a commitment whose bytes the scan did not read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hole {
    /// A `SEQ_GAP`: the walk expected `expected` and met `actual`, so positions `expected` to `actual − 1` are
    /// unfilled.
    Positions { expected: u64, actual: u64 },
    /// A `missing` row's hash that no ledger entry, `MALFORMED` row or `EXCLUDED` row names: a counted anchor whose
    /// bytes were never read.
    Missing(String),
    /// An `unproven` row's hash that no ledger entry, `MALFORMED` row or `EXCLUDED` row names: a record whose
    /// codeless test the reader's node could not answer.
    Unproven(String),
}

/// The answers of §4. **One form** every face renders: the reason a pick is withheld travels with it, the holes with
/// a `gap`, and every binding carries its `recipientForm`, so that a tool can say why it refused (§4.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    One(Binding),
    Candidates { reason: Reason, list: Vec<Binding>, holes: Vec<Hole> },
    None,
    /// The input is no `galeed/1` audit input (§4.1).
    Unread,
}

impl Pick {
    pub const ONE: &'static str = "one";
    pub const CANDIDATES: &'static str = "candidates";
    pub const NONE: &'static str = "none";
    pub const UNREAD: &'static str = "unread";

    pub const fn name(&self) -> &'static str {
        match self {
            Pick::One(_) => Pick::ONE,
            Pick::Candidates { .. } => Pick::CANDIDATES,
            Pick::None => Pick::NONE,
            Pick::Unread => Pick::UNREAD,
        }
    }
}

fn int(v: Option<&Value>) -> Option<u64> {
    match v? {
        Value::Int(n) => Some(*n),
        _ => None,
    }
}

fn rows<'a>(report: &'a Value, k: &str) -> &'a [Value] {
    match report.get(k) {
        Some(Value::Arr(a)) => a,
        _ => &[],
    }
}

fn column(report: &Value, list: &str, k: &str) -> BTreeSet<String> {
    rows(report, list).iter().filter_map(|r| r.get(k).and_then(Value::as_str)).map(str::to_string).collect()
}

/// §4.1: the bindings: the ledger entries of `I` (the carried byte strings `galeed/1` §8.1 admits, those whose
/// `entry_id` the report's anchored rows name) whose `entryType` is `keybinding`, members read from the entry's own
/// bytes, ordered by `seq` descending and, at equal `seq`, by `entry_id` ascending.
fn bindings(input: &Input, report: &Value) -> Vec<Binding> {
    let anchored = column(report, key::ANCHORED, key::ENTRY_ID);
    let mut out = Vec::new();
    for b in audit::carried(input) {
        let Ok(e) = envelope::accept(b) else { continue };
        let entry_id = hexform::encode(&e.entry_id);
        if !anchored.contains(&entry_id) || e.value.get("entryType").and_then(Value::as_str) != Some(body::KEYBINDING) {
            continue;
        }
        let bd = e.value.get("body");
        let recipient = bd.and_then(|x| x.get(key::RECIPIENT)).and_then(Value::as_str).unwrap_or_default().to_string();
        out.push(Binding {
            seq: int(e.value.get(key::SEQ)).unwrap_or_default(),
            entry_id,
            recipient_form: recipient_form(&recipient),
            recipient,
            effective: int(bd.and_then(|x| x.get(key::EFFECTIVE))).unwrap_or_default(),
        });
    }
    out.sort_by(|a, b| b.seq.cmp(&a.seq).then_with(|| a.entry_id.as_bytes().cmp(b.entry_id.as_bytes())));
    out
}

/// §4.2 rule 3, the holes: every `SEQ_GAP`, and every hash of a `missing` or `unproven` row that is the `entry_id`
/// of no ledger entry, no `MALFORMED` row and no `EXCLUDED` row. Bytes the scan read and found to be no entry of this
/// ledger leave no hole; a binding only a `VOID` record carries is nobody's act and adds none.
#[cfg(not(feature = "ablate-k3-hole"))]
pub fn holes(report: &Value) -> Vec<Hole> {
    let mut read = column(report, key::ANCHORED, key::ENTRY_ID);
    read.extend(column(report, key::MALFORMED, key::ENTRY_ID));
    read.extend(column(report, key::EXCLUDED, key::ENTRY_ID));
    let mut out: Vec<Hole> = rows(report, key::FINDINGS)
        .iter()
        .filter(|f| f.get(key::NAME).and_then(Value::as_str) == Some(audit::SEQ_GAP))
        .map(|f| Hole::Positions { expected: int(f.get(key::EXPECTED)).unwrap_or_default(), actual: int(f.get(key::ACTUAL)).unwrap_or_default() })
        .collect();
    out.extend(column(report, key::MISSING, key::HASH).into_iter().filter(|h| !read.contains(h)).map(Hole::Missing));
    out.extend(column(report, key::UNPROVEN, key::HASH).into_iter().filter(|h| !read.contains(h)).map(Hole::Unproven));
    out
}

/// Fault-injection variant for the conformance gauges: only a `SEQ_GAP` is a hole, so a counted anchor whose bytes
/// the scan never read lets the pick answer `one` from a ledger that may hold a later binding.
#[cfg(feature = "ablate-k3-hole")]
pub fn holes(report: &Value) -> Vec<Hole> {
    rows(report, key::FINDINGS)
        .iter()
        .filter(|f| f.get(key::NAME).and_then(Value::as_str) == Some(audit::SEQ_GAP))
        .map(|f| Hole::Positions { expected: int(f.get(key::EXPECTED)).unwrap_or_default(), actual: int(f.get(key::ACTUAL)).unwrap_or_default() })
        .collect()
}

/// §4: the pick over an audit input as handed over (the value a file read to): `unread` where it is no `galeed/1`
/// audit input, else [`pick_of`].
pub fn pick(input: &Value) -> Pick {
    match audit::read_input(input) {
        Some(i) => pick_of(&i),
        None => Pick::Unread,
    }
}

/// §4.2 over a well-formed audit input: five rules in order; the first that holds answers.
#[cfg(not(feature = "ablate-k3-pick"))]
pub fn pick_of(input: &Input) -> Pick {
    let report = audit::audit(input);
    let mut list = bindings(input, &report);
    let Some(top) = list.first().map(|b| b.seq) else { return Pick::None };
    let findings = rows(&report, key::FINDINGS);
    let named = |n: &'static str| findings.iter().filter(move |f| f.get(key::NAME).and_then(Value::as_str) == Some(n));
    if named(audit::EQUIVOCATION).any(|f| int(f.get(key::POSITION)) == Some(top)) {
        return Pick::Candidates { reason: Reason::Fork, list, holes: Vec::new() };
    }
    let gap = holes(&report);
    if !gap.is_empty() {
        return Pick::Candidates { reason: Reason::Gap, list, holes: gap };
    }
    if findings.iter().any(|f| matches!(f.get(key::HARD), Some(Value::Bool(true)))) {
        return Pick::Candidates { reason: Reason::Broken, list, holes: Vec::new() };
    }
    Pick::One(list.remove(0))
}

/// Fault-injection variant for the conformance gauges: a gap is checked only after a break.
#[cfg(feature = "ablate-k3-pick")]
pub fn pick_of(input: &Input) -> Pick {
    let report = audit::audit(input);
    let mut list = bindings(input, &report);
    let Some(top) = list.first().map(|b| b.seq) else { return Pick::None };
    let findings = rows(&report, key::FINDINGS);
    let named = |n: &'static str| findings.iter().filter(move |f| f.get(key::NAME).and_then(Value::as_str) == Some(n));
    if named(audit::EQUIVOCATION).any(|f| int(f.get(key::POSITION)) == Some(top)) {
        return Pick::Candidates { reason: Reason::Fork, list, holes: Vec::new() };
    }
    if findings.iter().any(|f| matches!(f.get(key::HARD), Some(Value::Bool(true)))) {
        return Pick::Candidates { reason: Reason::Broken, list, holes: Vec::new() };
    }
    let gap = holes(&report);
    if !gap.is_empty() {
        return Pick::Candidates { reason: Reason::Gap, list, holes: gap };
    }
    Pick::One(list.remove(0))
}

/// The two reasons a recipient is refused for use; both answered loudly, nothing encrypted (§3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    RecipientForm,
    /// The recipient is a small-order point: the shared secret with any ephemeral key is all zeros.
    SmallOrder,
}

impl Refused {
    pub const fn name(self) -> &'static str {
        match self {
            Refused::RecipientForm => "recipient-form",
            Refused::SmallOrder => "small-order",
        }
    }
}

/// Every spelling of an X25519 small-order point (top bit cleared): 0, 1, the two order-8 points, p − 1,
/// p and p + 1. Key decoding ignores the top bit and reduces mod p, so these seven are exactly the keys
/// whose shared secret is all zeros.
const SMALL_ORDER: [[u8; 32]; 7] = [
    [0; 32],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f, 0xc4, 0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16, 0x5f, 0x49, 0xb8, 0x00],
    [0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1, 0x55, 0x9c, 0x83, 0xef, 0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c, 0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd, 0xd0, 0x9f, 0x11, 0x57],
    [0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f],
    [0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f],
    [0xee, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f],
];

fn small_order(key: &[u8; 32]) -> bool {
    let mut k = *key;
    k[31] &= 0x7f;
    SMALL_ORDER.contains(&k)
}

/// §3.2 and §3.3: whether a recipient can be encrypted to: its form, then not a point of small order. **The one
/// gate**: [`seal`] runs it at the moment of use, and a judge's tool runs it before it publishes a `keybinding`
/// (`galeed/1` §6.7; §3.2: the narrow form is the only spelling a seat publishes), so an unusable recipient is refused
/// where it is written and never blocks the evidence after it.
#[cfg(not(feature = "ablate-k3-usable"))]
pub fn usable(recipient: &str) -> Result<[u8; 32], Refused> {
    let key = recipient_key(recipient).ok_or(Refused::RecipientForm)?;
    if small_order(&key) {
        return Err(Refused::SmallOrder);
    }
    Ok(key)
}

/// Fault-injection variant for the conformance gauges: a point of small order passes, so a judge publishes a
/// binding no producer can encrypt to.
#[cfg(feature = "ablate-k3-usable")]
pub fn usable(recipient: &str) -> Result<[u8; 32], Refused> {
    let _ = small_order as fn(&[u8; 32]) -> bool;
    recipient_key(recipient).ok_or(Refused::RecipientForm)
}

/// age encryption to one recipient (binary format, one X25519 stanza), behind [`usable`]. Crate-private: a producer
/// reaches it only through [`seal`], to a binding the pick answered; the harness's own age self-checks call it with
/// the recipient their command names.
#[cfg(not(feature = "ablate-k3-age"))]
pub(crate) fn encrypt_to(recipient: &str, plaintext: &[u8]) -> Result<Vec<u8>, Refused> {
    usable(recipient)?;
    cryptox::age_encrypt(recipient, plaintext).ok_or(Refused::RecipientForm)
}

/// Fault-injection variant for the conformance gauges: the plaintext is returned as the ciphertext.
#[cfg(feature = "ablate-k3-age")]
pub(crate) fn encrypt_to(recipient: &str, plaintext: &[u8]) -> Result<Vec<u8>, Refused> {
    usable(recipient)?;
    Ok(plaintext.to_vec())
}

/// Why a file is not sealed: the recipient is refused (§3.3), or the file is not listed (§2.4). Either way nothing is
/// encrypted, nothing is listed, and no plaintext leaves: there is no fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsealed {
    Recipient(Refused),
    Listing(Unlisted),
}

impl Unsealed {
    pub const fn name(self) -> &'static str {
        match self {
            Unsealed::Recipient(r) => r.name(),
            Unsealed::Listing(u) => u.name(),
        }
    }
}

/// One file sealed for a seat: the manifest item, which lists the **plaintext** (§2.2, §2.4: one manifest stands for
/// every recipient), and the ciphertext that carries it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sealed {
    pub item: FileItem,
    pub ciphertext: Vec<u8>,
}

/// §3 for one file: list it by its plaintext and encrypt it to the binding the pick answered (`one`, or one a
/// person chose from `candidates`). The answer is the sealed file or why it was refused; no path returns the
/// plaintext as a ciphertext or lists the ciphertext.
pub fn seal(to: &Binding, name: &str, media_type: &str, plaintext: &[u8]) -> Result<Sealed, Unsealed> {
    let item = FileItem::listed(name, plaintext, media_type).map_err(Unsealed::Listing)?;
    let ciphertext = encrypt_to(&to.recipient, plaintext).map_err(Unsealed::Recipient)?;
    Ok(Sealed { item, ciphertext })
}

pub fn decrypt(identity: &cryptox::AgeIdentity, ciphertext: &[u8]) -> Option<Vec<u8>> {
    cryptox::age_decrypt(identity, ciphertext)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject() -> Value {
        Value::Obj(vec![("kind".into(), s("hash")), ("sha256".into(), s(&format!("0x{}", "1".repeat(64))))])
    }

    fn party() -> String {
        format!("0x{}", "a".repeat(40))
    }

    #[test]
    fn manifest_order_index_and_the_zikaron_member() {
        let a = FileItem::listed("a.pdf", b"one", "application/pdf").unwrap();
        let z = Zikaron { root: party(), chain_id: 0, tx: format!("0x{}", "3".repeat(64)), entry: "0x".into() };
        let m = Manifest { subject_ref: subject(), party: party(), files: vec![a.clone(), a.clone().with_zikaron(z.clone())], note_md: Some("附注".into()) };
        let b = build(&m).unwrap();
        assert_eq!(check(&b), Ok(m.clone()), "第五员在场照收,`0x` 是空字节");
        assert_eq!(digest(&b).unwrap(), cryptox::sha256(&b));
        let bad = String::from_utf8(b.clone()).unwrap().replacen(r#""entry":"0x""#, r#""entry":"0X""#, 1);
        let f = check(bad.as_bytes()).unwrap_err();
        assert_eq!((f.name(), f.index()), ("E_FILE_ZIKARON", Some(1)), "第五员形外,携序号");
        let extra = String::from_utf8(b).unwrap().replacen(r#""entry":"0x","#, r#""entry":"0x","note_md":"x","#, 1);
        let f = check(extra.as_bytes()).unwrap_err();
        assert_eq!((f.name(), f.index()), ("E_FILE_ZIKARON", Some(1)), "_md 键在封闭表里照该表拒,不报字符集");
        assert_eq!(check(b"{\"spec\":\"courtevidence/1\"}").unwrap_err().name(), "E_MANIFEST_MISSING");
        assert_eq!(check(b" {}").unwrap_err().name(), "E_NOT_CANONICAL");
    }

    #[test]
    fn a_file_is_listed_by_its_plaintext_under_a_name_every_recipient_can_unpack() {
        let f = FileItem::listed("photos/door-1.jpg", b"bytes", "image/jpeg").unwrap();
        assert_eq!((f.sha256(), f.bytes()), (hexform::encode(&cryptox::sha256(b"bytes")).as_str(), 5));
        assert_eq!(FileItem::listed("现场照片.jpg", b"x", "image/jpeg"), Err(Unlisted::NameCharset), "中文名加文件时当场拒,不留到整份清单末步");
        assert_eq!(file_name("a\\b"), Err(Unlisted::NamePath));
        assert_eq!(file_name("C:x"), Err(Unlisted::NamePath));
        assert_eq!(file_name("/abs"), Err(Unlisted::NamePath));
        assert_eq!(file_name("a/../b"), Err(Unlisted::NamePath));
        assert_eq!(file_name("dir/.DS_Store"), Err(Unlisted::NameJunk));
        assert_eq!(file_name(""), Err(Unlisted::NameEmpty));
        assert_eq!(file_name("a b.txt"), Ok(()), "空格在 skeleton 之内");
    }

    #[test]
    fn recipient_form_and_age() {
        let id = cryptox::AgeIdentity::generate();
        let r = id.recipient();
        assert!(recipient_form(&r));
        assert!(!recipient_form(&r.to_ascii_uppercase()), "大写不收");
        let mut flipped = r.clone().into_bytes();
        flipped[10] = if flipped[10] == b'q' { b'p' } else { b'q' };
        assert!(!recipient_form(std::str::from_utf8(&flipped).unwrap()), "校验和");
        let ct = encrypt_to(&r, b"findings").unwrap();
        assert_eq!(decrypt(&id, &ct), Some(b"findings".to_vec()));
        assert_eq!(decrypt(&cryptox::AgeIdentity::generate(), &ct), None);
        assert_eq!(encrypt_to("not-a-recipient", b"x"), Err(Refused::RecipientForm));
        let data = "q".repeat(52);
        let zero = format!("age1{data}{}", checksum_for(&data));
        assert!(recipient_form(&zero), "全零钥成形");
        assert_eq!(usable(&zero), Err(Refused::SmallOrder), "小阶点响亮地拒,不中止");
        assert_eq!(encrypt_to(&zero, b"x"), Err(Refused::SmallOrder));
    }

    mod fixtures {
        use galeed::cryptox::Privkey;
        use galeed::hexform;
        use galeed::json::{self, Value};

        pub fn key(n: u8) -> Privkey {
            let mut k = [0u8; 32];
            k[31] = n;
            Privkey::from_bytes(&k).unwrap()
        }
        pub fn addr(k: &Privkey) -> String {
            hexform::encode(&k.address())
        }
        pub fn entry(k: &Privkey, seq: u64, prev: Option<&str>, entry_type: &str, body: &str) -> Vec<u8> {
            let mut six = vec![
                ("body".to_string(), json::read(body.as_bytes()).unwrap()),
                ("entryType".to_string(), Value::Str(entry_type.into())),
                ("judge".to_string(), Value::Str(addr(k))),
                ("prev".to_string(), prev.map_or(Value::Null, |p| Value::Str(p.into()))),
                ("seq".to_string(), Value::Int(seq)),
                ("spec".to_string(), Value::Str(galeed::LAW.into())),
            ];
            let signed = galeed::sign::sign_as(k, &json::canonical(&Value::Obj(six.clone())).unwrap(), galeed::literals::SEAT_JUDGE, galeed::LAW).unwrap();
            six.push(("sig".to_string(), Value::Str(hexform::encode(&signed.sig))));
            json::canonical(&Value::Obj(six)).unwrap()
        }
        pub fn id(b: &[u8]) -> String {
            hexform::encode(&galeed::cryptox::sha256(b))
        }
        pub fn record(n: u64, sender: &str, verdict: &str, hash: &str, carried: Option<&[u8]>) -> String {
            let entry = carried.map(|b| format!(r#","entry":"{}""#, hexform::encode(b))).unwrap_or_default();
            format!(r#"{{"chainId":1,"blockNumber":{n},"blockTimestamp":{n},"tx":"0x{n:064x}","sender":"{sender}","hash":"{hash}","verdict":"{verdict}"{entry}}}"#)
        }
        pub fn input(root: &str, records: &[String], senders: &[String]) -> Value {
            let mut s: Vec<String> = senders.to_vec();
            s.sort();
            s.dedup();
            let senders = s.iter().map(|a| format!("\"{a}\"")).collect::<Vec<_>>().join(",");
            let text = format!(r#"{{"root":"{root}","anchors":[{}],"basis":{{"chains":[{{"chainId":1,"fromBlock":0,"toBlock":100,"registries":[],"senders":[{senders}]}}]}}}}"#, records.join(","));
            json::read_open(text.as_bytes()).unwrap()
        }
    }

    #[test]
    fn the_pick_reads_the_scan_and_names_its_holes() {
        use fixtures::*;
        let (k, n) = (key(1), key(2));
        let (j, jn) = (addr(&k), addr(&n));
        let first = cryptox::AgeIdentity::generate().recipient();
        let g = entry(&k, 0, None, "genesis", r#"{"statement_md":""}"#);
        let b1 = entry(&k, 1, Some(&id(&g)), "keybinding", &format!(r#"{{"effective":1,"recipient":"{first}"}}"#));
        let counted = |n: u64, b: &[u8]| record(n, &j, "counted", &id(b), Some(b));
        assert_eq!(pick(&Value::Null), Pick::Unread, "不是审计输入即 unread");
        let one = pick(&input(&j, &[counted(1, &g), counted(2, &b1)], &[j.clone()]));
        assert!(matches!(&one, Pick::One(b) if b.recipient() == first && b.seq() == 1));
        assert_eq!(pick(&input(&j, &[counted(1, &g)], &[j.clone()])), Pick::None);
        // A counted anchor whose bytes no record carried is a hole: the producer can still fetch it.
        let lost = record(3, &j, "counted", &format!("0x{}", "9".repeat(64)), None);
        let gap = pick(&input(&j, &[counted(1, &g), counted(2, &b1), lost], &[j.clone()]));
        assert!(matches!(&gap, Pick::Candidates { reason: Reason::Gap, holes, .. } if holes == &vec![Hole::Missing(format!("0x{}", "9".repeat(64)))]));
        // A later binding only a VOID record carries is nobody's act: no hole, the pick stays on the counted one.
        let b2 = entry(&k, 2, Some(&id(&b1)), "keybinding", &format!(r#"{{"effective":2,"recipient":"{first}"}}"#));
        let void = record(4, &j, "VOID", &id(&b2), Some(&b2));
        assert!(matches!(pick(&input(&j, &[counted(1, &g), counted(2, &b1), void], &[j.clone()])), Pick::One(b) if b.seq() == 1), "只由 VOID 携带的后续绑定不造洞");
        // Naming the current key as root: the genesis is outside that lineage, so a binding the new key wrote sits
        // above a hole.
        let s = entry(&k, 2, Some(&id(&b1)), "succession", &format!(r#"{{"effective":1,"kind":"keyrotation","statement_md":"","to":"{jn}"}}"#));
        let b3 = entry(&n, 3, Some(&id(&s)), "keybinding", &format!(r#"{{"effective":3,"recipient":"{first}"}}"#));
        let recs = vec![counted(1, &g), counted(2, &b1), counted(3, &s), record(5, &jn, "counted", &id(&b3), Some(&b3))];
        assert!(matches!(pick(&input(&j, &recs, &[j.clone(), jn.clone()])), Pick::One(b) if b.seq() == 3), "以根读,换钥后的新绑定是当前的");
        assert!(matches!(pick(&input(&jn, &recs, &[j.clone(), jn.clone()])), Pick::Candidates { reason: Reason::Gap, .. }), "以当前钥作根:有绑定即 gap");
        assert_eq!(pick(&input(&jn, &[counted(1, &g)], &[j.clone(), jn.clone()])), Pick::None, "以当前钥作根且无绑定即 none");
    }

    #[test]
    fn seal_lists_the_plaintext_and_encrypts_only_to_a_picked_binding() {
        use fixtures::*;
        let k = key(1);
        let j = addr(&k);
        let id_age = cryptox::AgeIdentity::generate();
        let g = entry(&k, 0, None, "genesis", r#"{"statement_md":""}"#);
        let b1 = entry(&k, 1, Some(&id(&g)), "keybinding", &format!(r#"{{"effective":1,"recipient":"{}"}}"#, id_age.recipient()));
        let recs = vec![record(1, &j, "counted", &id(&g), Some(&g)), record(2, &j, "counted", &id(&b1), Some(&b1))];
        let Pick::One(to) = pick(&input(&j, &recs, &[j.clone()])) else { panic!("应挑出一枚") };
        let sealed = seal(&to, "report.txt", "text/plain", b"plain findings").unwrap();
        assert_eq!(sealed.item.sha256(), hexform::encode(&cryptox::sha256(b"plain findings")), "清单登明文摘要");
        assert_eq!(decrypt(&id_age, &sealed.ciphertext), Some(b"plain findings".to_vec()));
        assert_eq!(seal(&to, "报告.txt", "text/plain", b"x"), Err(Unsealed::Listing(Unlisted::NameCharset)));
        // A small-order binding: refused loudly, nothing sealed, no plaintext returned.
        let data = "q".repeat(52);
        let zero = format!("age1{data}{}", checksum_for(&data));
        let bz = entry(&k, 2, Some(&id(&b1)), "keybinding", &format!(r#"{{"effective":2,"recipient":"{zero}"}}"#));
        let recs = vec![record(1, &j, "counted", &id(&g), Some(&g)), record(2, &j, "counted", &id(&b1), Some(&b1)), record(3, &j, "counted", &id(&bz), Some(&bz))];
        let Pick::One(to) = pick(&input(&j, &recs, &[j.clone()])) else { panic!("应挑出顶上那一枚") };
        assert!(to.recipient_form(), "形合;用不用得上是 §3.3 的");
        assert_eq!(seal(&to, "report.txt", "text/plain", b"x"), Err(Unsealed::Recipient(Refused::SmallOrder)));
    }

    fn checksum_for(data: &str) -> String {
        let groups: Vec<u8> = data.bytes().map(|c| BECH32_ALPHABET.iter().position(|a| *a == c).unwrap() as u8).collect();
        let expand: Vec<u8> = HRP.bytes().map(|c| c >> 5).chain(std::iter::once(0)).chain(HRP.bytes().map(|c| c & 31)).collect();
        let pm = polymod(expand.into_iter().chain(groups).chain([0u8; 6])) ^ 1;
        (0..6).map(|i| char::from(BECH32_ALPHABET[((pm >> (5 * (5 - i))) & 31) as usize])).collect()
    }
}
