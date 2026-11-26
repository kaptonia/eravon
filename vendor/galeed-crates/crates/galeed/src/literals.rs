//! Reading and checking the family literal registry (the registry is the contract; the checker measures it).
//!
//! `FAMILY-LITERALS.md` is embedded at compile time ([`REGISTRY_TEXT`]): every question about which
//! literals are registered is answered from this one copy, and the signing allow-list is generated from
//! it ([`crate::sign::registered_domains`]). [`check`] reads a registry, the law texts and the two contract
//! sources (the registry and the succession book) from disk and answers a report under the registry's thirteen
//! readings.

#![cfg_attr(feature = "ablate-k5-checker", allow(dead_code))]

use crate::json::{self, Value};

pub const REGISTRY_TEXT: &str = include_str!("../../../FAMILY-LITERALS.md");

pub const ROLE_SIGNING_DOMAIN: &str = "signing-domain";
pub const ROLE_DOCUMENT_KIND: &str = "document-kind";
pub const ROLE_REGISTRY_ABI: &str = "registry-abi";
/// The succession book's deployed names (`galeed.roster/1` §4), read against the book's source as reading ⑤'s second
/// equality.
pub const ROLE_BOOK_ABI: &str = "book-abi";
/// The book the family declares for each chain (`galeed.roster/1` §9), one row per chain, `<chainId>:<address>`.
pub const ROLE_SUCCESSION_BOOK: &str = "succession-book";
pub const ROLE_POINTER_PREFIX: &str = "pointer-prefix";
pub const ROLE_VENUE_INTERFACE: &str = "venue-interface";
pub const ROLE_BASIS_MEMBER: &str = "basis-member";
pub const ROLE_DERIVATION_PURPOSE: &str = "derivation-purpose";
pub const ROLE_ZIKARON_CONTACT: &str = "zikaron-contact";
pub const ROLE_KIT_PREFIX: &str = "kit-prefix";
pub const ROLE_VENUE_DECLARATION: &str = "venue-declaration";
pub const ROLE_VIEW_ROLE: &str = "view-role";
pub const ROLE_QUOTE_FIELD: &str = "quote-field";
pub const ROLE_BACKUP_ENVELOPE: &str = "backup-envelope";
/// `kind` literals of the desk's own document forms (protocol documents, subscription invitations): not
/// family papers, and signed under no domain.
pub const ROLE_DESK_DOCUMENT: &str = "desk-document";
/// Kind prefixes a law keeps for its own kinds (`galeed.papers/1` §7a.3); no product kind begins with one.
pub const ROLE_RESERVED_PREFIX: &str = "reserved-prefix";

/// Literals looked up by `use` name (the registry's `use` cell, read the same way as derivation
/// purposes): the literal lives only in the registry and the product never writes a second copy.
pub const USE_ZIKARON_POINTER: &str = "zikaron-pointer-file";
pub const USE_ZIKARON_DEFAULT_DIR: &str = "zikaron-default-dir";
pub const USE_ZIKARON_INDEX: &str = "zikaron-index-path";
pub const USE_ZIKARON_INDEX_FORM: &str = "zikaron-index-form";
pub const USE_ZIKARON_VERIFIED_DIR: &str = "zikaron-verified-dir";
pub const USE_ZIKARON_VERIFICATION_FORM: &str = "zikaron-verification-form";
pub const USE_ZIKARON_KIT_MANIFEST: &str = "zikaron-kit-manifest";
pub const USE_ZIKARON_KIT_FILE: &str = "zikaron-kit-file";
pub const USE_ZIKARON_KIT_ADDRESSED: &str = "zikaron-kit-addressed";
pub const USE_KIT_PREFIX: &str = "kit-prefix";
pub const USE_BACKUP_MAGIC: &str = "backup-magic";
pub const USE_BACKUP_SUFFIX: &str = "backup-suffix";
pub const USE_BACKUP_APP: &str = "backup-app";
pub const USE_PROTOCOL_KIND: &str = "protocol-kind";
pub const USE_SUBSCRIPTION_KIND: &str = "subscription-kind";

/// Constants in the registry's `data` section, looked up by name (not literals, so not part of the
/// uniqueness checks; the product never writes a second copy). The four cost bounds for importing a
/// keystore v3 file: scrypt memory, parallelism, total work and pbkdf2 rounds.
pub const DATA_KEYSTORE_SCRYPT_MAX_MEMORY: &str = "keystoreScryptMaxMemory";
pub const DATA_KEYSTORE_SCRYPT_MAX_P: &str = "keystoreScryptMaxP";
pub const DATA_KEYSTORE_SCRYPT_MAX_WORK: &str = "keystoreScryptMaxWork";
pub const DATA_KEYSTORE_PBKDF2_MAX_ROUNDS: &str = "keystorePbkdf2MaxRounds";

/// The integer `value` of the constant `name` in the embedded registry's `data` section. An unreadable
/// registry, a missing name or a non-integer value is `None` (callers refuse; no number is guessed).
pub fn datum(name: &str) -> Option<u64> {
    static DATA: std::sync::OnceLock<Option<Value>> = std::sync::OnceLock::new();
    let data = DATA.get_or_init(|| fenced_json(REGISTRY_TEXT).and_then(|b| json::read_open(b.as_bytes())).and_then(|v| v.get("data").cloned()));
    match data.as_ref()?.get(name)?.get("value")? {
        Value::Int(n) => Some(*n),
        _ => None,
    }
}

/// The string `value` of the constant `name` in the `data` section (such as `registryCodeHash`).
/// Unreadable, missing or not a string is `None`. Same `data` as [`datum`].
pub fn datum_text(name: &str) -> Option<String> {
    let data = fenced_json(REGISTRY_TEXT).and_then(|b| json::read_open(b.as_bytes())).and_then(|v| v.get("data").cloned())?;
    data.get(name)?.get("value")?.as_str().map(str::to_string)
}

pub const DATA_REGISTRY_CODE_HASH: &str = "registryCodeHash";
/// The succession book's code hash (`galeed.roster/1` §9: a reader pins the book it declares by its code hash).
pub const DATA_BOOK_CODE_HASH: &str = "bookCodeHash";
/// The standard router's code hash (`galeed.venue/1` §4, CODEHASH.md).
pub const DATA_ROUTER_CODE_HASH: &str = "routerCodeHash";

/// **The book the family declares for `chain`** (`galeed.roster/1` §9), read from the embedded registry's
/// `succession-book` rows (`<chainId>:<address>`): the address in its lowercase hex20 spelling, `None` where the
/// family declares no book for that chain or the registry does not read. The one reading of the declaration: the
/// build's chain row (`galeed_cli::chain`) and every reader of a book take it from here.
pub fn succession_book(chain: u64) -> Option<String> {
    let reg = parse(REGISTRY_TEXT)?;
    let want = chain.to_string();
    reg.entries.iter().filter(|e| e.role == ROLE_SUCCESSION_BOOK).find_map(|e| {
        let (c, addr) = e.literal.split_once(PREFIX_SEP)?;
        (c == want && crate::hexform::is_hex20(addr)).then(|| addr.to_string())
    })
}

/// The closed set of three seats. Seat names live only here.
pub const SEAT_JUDGE: &str = "judge";
pub const SEAT_GUILD: &str = "guild";
pub const SEAT_VERIFIER: &str = "verifier";
pub const SEATS: [&str; 3] = [SEAT_JUDGE, SEAT_GUILD, SEAT_VERIFIER];

pub const GALEED_LAW_FILE: &str = "galeed-v1.md";
pub const PAPERS_LAW_FILE: &str = "galeed-papers-v1.md";
pub const ROSTER_LAW_FILE: &str = "galeed-roster-v1.md";
const GALEED_DOMAIN_MARK: &str = "**5.6 Domain.**";
const PAPERS_DOMAIN_MARK: &str = "**[D] 3.2 Domains.**";
/// Marker of the kinds and pointer-prefix line of galeed.roster/1 §8 (verbatim from the law text).
const ROSTER_VOCAB_MARK: &str = "- kinds: ";
/// Both ends of the galeed/1 §9.1 section on the one anchor form and the start of the §9.4 basis
/// paragraph (verbatim from the law text).
const GALEED_FORMS_MARK: &str = "9.1 The form.";
const GALEED_RECORD_MARK: &str = "9.2 Anchor record.";
const GALEED_BASIS_MARK: &str = "9.4 Scan basis.";
/// How the law text writes a topic 0 preimage: `keccak256("<signature>")`. Signature literals are taken
/// only from this form, so prose event declarations with modifiers and parameter names (the two-topic
/// counterexample) are never picked up.
const TOPIC0_PREIMAGE_MARK: &str = "keccak256(\"";
const PREFIX_SEP: char = ':';
/// The head every family kind and reserved prefix starts with (`galeed.roster/`, `galeed.verifier/`).
const RESERVED_HEAD: &str = "galeed.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub literal: String,
    /// The `literal` member as the register writes it (every line's), whatever spelling this build takes (`literal`).
    pub register_literal: String,
    pub role: String,
    pub sources: Vec<String>,
    pub paired_with: Option<String>,
    pub abi_kind: Option<String>,
    pub abi_signature: Option<String>,
    /// The closed list of seats that may sign this domain (signing-domain role only).
    pub seats: Vec<String>,
    pub purpose_use: Option<String>,
    /// The one place in the code that holds the literal now (`at`): `Some(Some(place))`; `Some(None)` when the
    /// registry writes `null` (the code spells it nowhere, and `note` says why); `None` when the entry has no `at`.
    pub at: Option<Option<String>>,
    /// The entry's one-sentence explanation.
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Derivation {
    pub path: String,
    pub family: String,
    pub project: u64,
    pub role: String,
}

#[derive(Clone, Debug)]
pub struct Registry {
    pub roles: Vec<String>,
    pub entries: Vec<Entry>,
    pub pairs: Vec<String>,
    /// The literals the registry records of other families (`foreign`: a list, in the family's form; each item's
    /// `literals` when it has any), for the distinctness check beside the zikaron law texts read afresh.
    pub foreign: Vec<String>,
    /// The derivation-path table; empty when the registry has no such block (reading ⑧ fails an empty list).
    pub derivation: Vec<Derivation>,
}

pub const DERIVATION_PATHS: &str = "derivationPaths";
pub const INDEX_PLACEHOLDER: &str = "{i}";

fn fenced_json(md: &str) -> Option<&str> {
    let mut blocks = md.split("```json\n").skip(1).filter_map(|rest| rest.split("```").next());
    let first = blocks.next()?;
    if blocks.next().is_some() {
        return None;
    }
    Some(first)
}

/// The `foreign` list: every item's `literals` (an item without them records none); anything but a list does not read.
fn foreign_literals(v: Option<&Value>) -> Option<Vec<String>> {
    match v? {
        Value::Arr(items) => Some(items.iter().map(|i| if i.get("literals").is_some() { strs(i.get("literals")) } else { Some(Vec::new()) }).collect::<Option<Vec<_>>>()?.concat()),
        _ => None,
    }
}

fn strs(v: Option<&Value>) -> Option<Vec<String>> {
    match v? {
        Value::Arr(a) => a.iter().map(|x| x.as_str().map(str::to_string)).collect(),
        _ => None,
    }
}

/// An entry's literal **as this build spells it**: the Sepolia trial build takes the entry's `literalSepolia` when it has
/// one (a name each line spells its own, such as the command line's link), every build `literal` otherwise. The
/// register says both lines; no reading of it pins one.
fn this_line_literal(e: &Value) -> Option<String> {
    let line = if cfg!(feature = "chain-sepolia") { e.get("literalSepolia").and_then(Value::as_str) } else { None };
    line.or_else(|| e.get("literal").and_then(Value::as_str)).map(str::to_string)
}

pub fn parse(md: &str) -> Option<Registry> {
    let v = json::read_open(fenced_json(md)?.as_bytes())?;
    let roles: Vec<String> = v.get("roles")?.as_obj()?.iter().map(|(k, _)| k.clone()).collect();
    let entries: Vec<Entry> = match v.get("entries")? {
        Value::Arr(a) => a
            .iter()
            .map(|e| {
                Some(Entry {
                    literal: this_line_literal(e)?,
                    register_literal: e.get("literal")?.as_str()?.to_string(),
                    role: e.get("role")?.as_str()?.to_string(),
                    sources: strs(e.get("sources"))?,
                    paired_with: e.get("pairedWith").and_then(Value::as_str).map(str::to_string),
                    abi_kind: e.get("abi").and_then(|a| a.get("kind")).and_then(Value::as_str).map(str::to_string),
                    abi_signature: e.get("abi").and_then(|a| a.get("signature")).and_then(Value::as_str).map(str::to_string),
                    seats: match e.get("seats") {
                        None => Vec::new(),
                        Some(_) => strs(e.get("seats"))?,
                    },
                    purpose_use: e.get("use").and_then(Value::as_str).map(str::to_string),
                    at: match e.get("at") {
                        None => None,
                        Some(Value::Null) => Some(None),
                        Some(v) => Some(Some(v.as_str()?.to_string())),
                    },
                    note: e.get("note").and_then(Value::as_str).map(str::to_string),
                })
            })
            .collect::<Option<_>>()?,
        _ => return None,
    };
    let derivation = match v.get(DERIVATION_PATHS).and_then(|d| d.get("entries")) {
        None => Vec::new(),
        Some(Value::Arr(a)) => a
            .iter()
            .map(|e| {
                Some(Derivation {
                    path: e.get("path")?.as_str()?.to_string(),
                    family: e.get("family")?.as_str()?.to_string(),
                    project: match e.get("project")? {
                        Value::Int(n) => *n,
                        _ => return None,
                    },
                    role: e.get("role")?.as_str()?.to_string(),
                })
            })
            .collect::<Option<_>>()?,
        Some(_) => return None,
    };
    Some(Registry {
        roles,
        entries,
        pairs: strs(v.get("pairs"))?,
        foreign: foreign_literals(v.get("foreign"))?,
        derivation,
    })
}

impl Registry {
    pub fn of_role(&self, role: &str) -> Vec<String> {
        self.entries.iter().filter(|e| e.role == role).map(|e| e.literal.clone()).collect()
    }

    /// Signing domains per seat, as `(seat, domain)` pairs sorted by pair. The product allow-list and the
    /// checker both read this one body.
    pub fn domains_by_seat(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = self
            .entries
            .iter()
            .filter(|e| e.role == ROLE_SIGNING_DOMAIN)
            .flat_map(|e| e.seats.iter().map(|s| (s.clone(), e.literal.clone())))
            .collect();
        out.sort();
        out.dedup();
        out
    }

    pub fn purpose(&self, use_name: &str) -> Option<String> {
        self.entries
            .iter()
            .find(|e| e.role == ROLE_DERIVATION_PURPOSE && e.purpose_use.as_deref() == Some(use_name))
            .map(|e| e.literal.clone())
    }
}

/// Signing domains of the embedded registry (five); an unreadable registry is an empty list, so the
/// signing API allows no domain.
#[cfg(not(feature = "ablate-k5-registry"))]
pub fn embedded_signing_domains() -> Vec<String> {
    parse(REGISTRY_TEXT).map(|r| r.of_role(ROLE_SIGNING_DOMAIN)).unwrap_or_default()
}

/// Fault-injection variant for the conformance gauges: the embedded registry is not read and the
/// allow-list is empty.
#[cfg(feature = "ablate-k5-registry")]
pub fn embedded_signing_domains() -> Vec<String> {
    Vec::new()
}

/// Signing domains of the embedded registry per seat, as `(seat, domain)` pairs; an unreadable registry
/// is an empty list, so the per-seat gate allows no pair. Shares the fault-injection switch of
/// [`embedded_signing_domains`] (the same embedded text).
#[cfg(not(feature = "ablate-k5-registry"))]
pub fn embedded_domains_by_seat() -> Vec<(String, String)> {
    parse(REGISTRY_TEXT).map(|r| r.domains_by_seat()).unwrap_or_default()
}

/// Fault-injection variant for the conformance gauges: the per-seat allow-list is empty.
#[cfg(feature = "ablate-k5-registry")]
pub fn embedded_domains_by_seat() -> Vec<(String, String)> {
    Vec::new()
}

/// The HKDF purpose string for a use name in the embedded registry; `None` when unreadable, and callers
/// refuse rather than derive with an empty purpose.
pub fn derivation_purpose(use_name: &str) -> Option<String> {
    parse(REGISTRY_TEXT)?.purpose(use_name)
}

/// The literal for a `use` name in the embedded registry (the ZIKARON contact points and the kit
/// prefix). `None` when unreadable: callers refuse explicitly and never build a path from an empty string.
pub fn by_use(use_name: &str) -> Option<String> {
    parse(REGISTRY_TEXT)?.entries.into_iter().find(|e| e.purpose_use.as_deref() == Some(use_name)).map(|e| e.literal)
}

pub fn of_role(role: &str) -> Vec<String> {
    parse(REGISTRY_TEXT).map(|r| r.of_role(role)).unwrap_or_default()
}

/// The derivation path of one family and role in the embedded registry (`{i}` left as is); `None` when
/// unreadable or absent. Every place that derives a key by role reads it here, so tools share one table.
pub fn derivation_path(family: &str, role: &str) -> Option<String> {
    parse(REGISTRY_TEXT)?.derivation.into_iter().find(|d| d.family == family && d.role == role).map(|d| d.path)
}

/// Path form of reading ⑧: `m/44'/60'/<project>'/<group>/<index>` with the project number equal to
/// `project`; the index may be `{i}`. Segments are read the way the product derives
/// (`cryptox::parse_path`: decimal without leading zero, below 2^31), with `{i}` read as 0, so every row
/// the checker admits is derivable.
fn derivation_form_ok(d: &Derivation) -> bool {
    let segs: Vec<&str> = d.path.split('/').collect();
    // The placeholder may only be the whole index segment; anywhere else is out of form.
    let placeholder_only_as_index = segs.iter().enumerate().all(|(i, s)| !s.contains(INDEX_PLACEHOLDER) || (i == 5 && *s == INDEX_PLACEHOLDER));
    let concrete = if segs.get(5) == Some(&INDEX_PLACEHOLDER) { format!("{}/0", segs[..5].join("/")) } else { d.path.clone() };
    segs.len() == 6
        && placeholder_only_as_index
        && segs[..3] == ["m", "44'", "60'"]
        && segs[3].strip_suffix('\'').is_some_and(|p| p.parse::<u64>().ok() == Some(d.project))
        && !segs[4].ends_with('\'')
        && (segs[5] == INDEX_PLACEHOLDER || !segs[5].ends_with('\''))
        && crate::cryptox::parse_path(&concrete).is_some()
}

/// Whether the path sets of two rows intersect: equal verbatim, or one ends in `{i}` and the other
/// equals it except for the last segment (`{i}` covers every index there).
fn derivation_overlap(a: &str, b: &str) -> bool {
    let prefix = |p: &str| p.rsplit_once('/').map(|(head, last)| (head.to_string(), last == INDEX_PLACEHOLDER));
    a == b
        || match (prefix(a), prefix(b)) {
            (Some((ha, wa)), Some((hb, wb))) => ha == hb && (wa || wb),
            _ => false,
        }
}

fn literal_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '/' | '-')
}

/// Every run in the text that starts with `prefix` and contains `/`, regardless of markup (backticks and
/// fences do not count: reading by markup would slip at an odd backtick and miss half the text; taking
/// too much is harmless, since any intersection must fail).
fn backticked(text: &str, prefix: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut from = 0usize;
    while let Some(i) = text[from..].find(prefix).map(|i| i + from) {
        let before_ok = !text[..i].chars().next_back().is_some_and(literal_char);
        let run: String = text[i..].chars().take_while(|c| literal_char(*c)).collect();
        let run = run.trim_end_matches(['.', '-', '/']).to_string();
        if before_ok && run.contains('/') && !out.contains(&run) {
            out.push(run);
        }
        from = i + prefix.len();
    }
    out
}

/// Every reserved prefix the text spells: a run that starts with `galeed.`, is made of literal characters and ends at
/// its one `/` (`galeed.roster/`, never `galeed.roster/1`, never a path with two slashes), taken without regard to
/// markup like [`backticked`]. Kinds and prefixes are read apart: a prefix is never a kind ([`backticked`] trims it).
fn reserved_prefixes(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut from = 0usize;
    while let Some(i) = text[from..].find(RESERVED_HEAD).map(|i| i + from) {
        let before_ok = !text[..i].chars().next_back().is_some_and(literal_char);
        let run: String = text[i..].chars().take_while(|c| literal_char(*c)).collect();
        let run = run.trim_end_matches(['.', '-']).to_string();
        if before_ok && run.ends_with('/') && run.matches('/').count() == 1 && !out.contains(&run) {
            out.push(run);
        }
        from = i + RESERVED_HEAD.len();
    }
    out
}

fn paragraph<'a>(law: &'a str, mark: &str) -> Option<&'a str> {
    let rest = &law[law.find(mark)?..];
    Some(&rest[..rest.find("\n\n").unwrap_or(rest.len())])
}

fn paragraph_literals(law: &str, mark: &str, prefix: &str) -> Option<Vec<String>> {
    paragraph(law, mark).map(|p| backticked(p, prefix))
}

/// Name of a law text: the first family literal in its title line (`# GALEED roster law v1
/// (`galeed.roster/1`, …)` gives `galeed.roster/1`); an entry whose `sources` starts with it cites that law.
fn law_name(law: &str) -> Option<String> {
    backticked(law.lines().next()?, "galeed").into_iter().next()
}

/// Every pointer-prefix run in the text that starts with `prefix`, ends in `:` and has at least two `:`
/// (alphanumerics and `:`), taken without regard to markup like [`backticked`].
fn pointer_prefixes(text: &str, prefix: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut from = 0usize;
    while let Some(i) = text[from..].find(prefix).map(|i| i + from) {
        let before_ok = !text[..i].chars().next_back().is_some_and(|c| literal_char(c) || c == PREFIX_SEP);
        let run: String = text[i..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == PREFIX_SEP).collect();
        let run = run.trim_end_matches(|c: char| c != PREFIX_SEP).to_string();
        if before_ok && run.matches(PREFIX_SEP).count() >= 2 && !out.contains(&run) {
            out.push(run);
        }
        from = i + prefix.len();
    }
    out
}

fn section<'a>(law: &'a str, from: &str, to: &str) -> Option<&'a str> {
    let i = law.find(from)?;
    let j = law[i..].find(to)? + i;
    Some(&law[i..j])
}

/// The event name, taken from the head of the product's signature constant rather than a second literal.
fn anchored_event() -> &'static str {
    crate::kit::ANCHORED_CARRIED_SIGNATURE.split('(').next().unwrap_or_default()
}

/// Signature literals written as `keccak256("<name>(…)")` in the law text, the topic 0 preimage form.
/// Inside the parentheses only letters, digits, commas and brackets are allowed, so prose examples such
/// as `event Anchored(address indexed sender, bytes32 hash)` (the law's two-topic counterexample) never
/// get in.
fn hashed_signatures(text: &str, name: &str) -> Vec<String> {
    let head = format!("{TOPIC0_PREIMAGE_MARK}{name}(");
    let mut out: Vec<String> = Vec::new();
    let mut from = 0usize;
    while let Some(i) = text[from..].find(&head).map(|i| i + from) {
        from = i + head.len();
        let rest = &text[i + TOPIC0_PREIMAGE_MARK.len()..];
        let Some(end) = rest.find("\")") else { continue };
        let sig = &rest[..end];
        // Strip prefix and suffix on character boundaries, not byte offsets: a multi-byte character in the law
        // text must never crash the checker.
        let Some(inside) = sig.strip_prefix(&head[TOPIC0_PREIMAGE_MARK.len()..]).and_then(|x| x.strip_suffix(')')) else { continue };
        if inside.chars().all(|c| c.is_ascii_alphanumeric() || c == ',' || c == '[' || c == ']') && !out.contains(&sig.to_string()) {
            out.push(sig.to_string());
        }
    }
    out
}

pub fn contract_names(src: &str) -> Vec<String> {
    src.lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("function ").or_else(|| l.strip_prefix("event ")))
        .filter_map(|l| l.split('(').next())
        .map(|s| s.trim().to_string())
        .collect()
}

pub struct RuleResult {
    pub rule: u64,
    pub ok: bool,
    pub detail: String,
}

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v.dedup();
    v
}

/// The thirteen readings. `zikaron_laws` are the texts of every law file in that directory; `contract_src` and
/// `book_src` the registry's and the succession book's sources; `embedded_domains` is the domain list the product's
/// signing API allows now, and `embedded_by_seat` the same gate's list by (seat, domain).
#[cfg(not(feature = "ablate-k5-checker"))]
#[allow(clippy::too_many_arguments)]
pub fn check(reg: &Registry, galeed_law: &str, papers_law: &str, roster_law: &str, zikaron_laws: &[String], contract_src: &str, book_src: &str, embedded_domains: &[String], embedded_by_seat: &[(String, String)]) -> Vec<RuleResult> {
    crate::marks::mark(crate::marks::Feature::K5);
    let mut out = Vec::new();
    // ① (literal, role) pairs are unique.
    let mut seen: Vec<(String, String)> = Vec::new();
    let mut dups: Vec<String> = Vec::new();
    for e in &reg.entries {
        let k = (e.literal.clone(), e.role.clone());
        if seen.contains(&k) {
            dups.push(format!("{}@{}", e.literal, e.role));
        }
        seen.push(k);
    }
    out.push(RuleResult { rule: 1, ok: dups.is_empty(), detail: if dups.is_empty() { "no (literal, role) repeats".into() } else { format!("repeated: {}", dups.join(", ")) } });
    // ② Literals shared across roles are exactly the `pairs` closed table, each pair pointing both ways (no count fixed).
    // The pairing is the register's (`register_literal`, every line's spelling), never one build line's: a name the
    // Sepolia line spells its own way still pairs as the register writes it.
    let mut multi: Vec<String> = Vec::new();
    for e in &reg.entries {
        if reg.entries.iter().any(|o| o.register_literal == e.register_literal && o.role != e.role) && !multi.contains(&e.register_literal) {
            multi.push(e.register_literal.clone());
        }
    }
    let pairs_ok = sorted(multi.clone()) == sorted(reg.pairs.clone())
        && reg.pairs.iter().all(|p| {
            let es: Vec<&Entry> = reg.entries.iter().filter(|e| e.register_literal == *p).collect();
            es.len() == 2 && es.iter().all(|e| e.paired_with.as_deref() == Some(es.iter().find(|o| o.role != e.role).map(|o| o.role.as_str()).unwrap_or("")))
        })
        && reg.entries.iter().filter(|e| e.paired_with.is_some()).all(|e| reg.pairs.contains(&e.register_literal));
    out.push(RuleResult { rule: 2, ok: pairs_ok, detail: format!("cross-role literals: [{}]; pairs: [{}]", multi.join(", "), reg.pairs.join(", ")) });
    // ③ No literal shared with the zikaron family (what `foreign` records plus the law texts read now).
    let mut zk: Vec<String> = reg.foreign.clone();
    for law in zikaron_laws {
        zk.extend(backticked(law, "zikaron"));
    }
    let zk = sorted(zk);
    let clash: Vec<String> = reg.entries.iter().filter(|e| zk.contains(&e.literal)).map(|e| e.literal.clone()).collect();
    // Reading no zikaron law text at all fails: comparing against the recorded lower bound alone is "not
    // connected" and must not pass silently.
    out.push(RuleResult { rule: 3, ok: clash.is_empty() && !zikaron_laws.is_empty(), detail: format!("zikaron law texts read: {}; zikaron literals consulted: {}; clashes: [{}]", zikaron_laws.len(), zk.len(), clash.join(", ")) });
    // ④ signing-domain equals the union of the two laws' closed tables.
    let law_domains = match (paragraph_literals(galeed_law, GALEED_DOMAIN_MARK, "galeed"), paragraph_literals(papers_law, PAPERS_DOMAIN_MARK, "galeed")) {
        (Some(a), Some(b)) => Some(sorted(a.into_iter().chain(b).collect())),
        _ => None,
    };
    let registered = sorted(reg.of_role(ROLE_SIGNING_DOMAIN));
    out.push(match &law_domains {
        None => RuleResult { rule: 4, ok: false, detail: "closed-list paragraph not found in a law text".into() },
        Some(l) => RuleResult { rule: 4, ok: *l == registered, detail: format!("law: [{}]; registered: [{}]", l.join(", "), registered.join(", ")) },
    });
    // ⑤ one equality per contract source: each role's function and event names equal those read from its source.
    let names_of = |role: &str| -> Vec<String> { sorted(reg.entries.iter().filter(|e| e.role == role && matches!(e.abi_kind.as_deref(), Some("function") | Some("event"))).map(|e| e.literal.clone()).collect()) };
    let (src_names, reg_names) = (sorted(contract_names(contract_src)), names_of(ROLE_REGISTRY_ABI));
    let (book_names, reg_book) = (sorted(contract_names(book_src)), names_of(ROLE_BOOK_ABI));
    out.push(RuleResult {
        rule: 5,
        ok: src_names == reg_names && book_names == reg_book,
        detail: format!("registry source: [{}]; registered: [{}]; book source: [{}]; registered: [{}]", src_names.join(", "), reg_names.join(", "), book_names.join(", "), reg_book.join(", ")),
    });
    // ⑥ The product's signing allow-list equals the signing-domain set.
    let emb = sorted(embedded_domains.to_vec());
    out.push(RuleResult { rule: 6, ok: emb == registered, detail: format!("signing face: [{}]; registered: [{}]", emb.join(", "), registered.join(", ")) });
    // ⑦ Every entry's role is in the closed set of roles.
    let stray: Vec<String> = reg.entries.iter().filter(|e| !reg.roles.contains(&e.role)).map(|e| format!("{}@{}", e.literal, e.role)).collect();
    out.push(RuleResult { rule: 7, ok: stray.is_empty(), detail: if stray.is_empty() { "every role is declared".into() } else { format!("undeclared roles: {}", stray.join(", ")) } });
    // ⑧ Derivation-path table: non-empty, well formed, paths pairwise distinct, (family, role) pairwise distinct.
    let bad_form: Vec<String> = reg.derivation.iter().filter(|d| !derivation_form_ok(d)).map(|d| d.path.clone()).collect();
    let mut paths: Vec<String> = Vec::new();
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut repeats: Vec<String> = Vec::new();
    for d in &reg.derivation {
        if paths.iter().any(|p| derivation_overlap(p, &d.path)) {
            repeats.push(d.path.clone());
        }
        if pairs.contains(&(d.family.clone(), d.role.clone())) {
            repeats.push(format!("{}/{}", d.family, d.role));
        }
        paths.push(d.path.clone());
        pairs.push((d.family.clone(), d.role.clone()));
    }
    let ok8 = !reg.derivation.is_empty() && bad_form.is_empty() && repeats.is_empty();
    out.push(RuleResult { rule: 8, ok: ok8, detail: format!("derivation paths: {}; outside the form: [{}]; repeated: [{}]", reg.derivation.len(), bad_form.join(", "), repeats.join(", ")) });
    // ⑨ galeed.roster/1 §8 read in two columns, kinds apart from reserved prefixes: pointer-prefix equals its pointer
    // prefix; document-kind entries citing that law equal its kind table (a reserved prefix, ending in `/`, is never
    // read as a kind); its reserved prefixes are the column ⑬ reads.
    let prefixes = sorted(reg.of_role(ROLE_POINTER_PREFIX));
    let roster_reserved: Option<Vec<String>> = paragraph(roster_law, ROSTER_VOCAB_MARK).map(|p| sorted(reserved_prefixes(p)));
    out.push(match (law_name(roster_law), paragraph(roster_law, ROSTER_VOCAB_MARK)) {
        (None, _) | (_, None) => RuleResult { rule: 9, ok: false, detail: "closed-list paragraph not found in a law text".into() },
        (Some(name), Some(p)) => {
            let roster_kinds = sorted(reg.entries.iter().filter(|e| e.role == ROLE_DOCUMENT_KIND && e.sources.iter().any(|s| s.split_whitespace().next() == Some(name.as_str()))).map(|e| e.literal.clone()).collect());
            let (law_kinds, law_prefixes) = (sorted(backticked(p, "galeed")), sorted(pointer_prefixes(p, "galeed")));
            let law_reserved = roster_reserved.clone().unwrap_or_default();
            RuleResult {
                rule: 9,
                ok: !law_kinds.is_empty() && !law_prefixes.is_empty() && !law_reserved.is_empty() && law_kinds.iter().all(|k| !k.ends_with('/')) && law_kinds == roster_kinds && law_prefixes == prefixes,
                detail: format!(
                    "law kinds: [{}]; registered: [{}]; law reserved prefixes: [{}]; law pointer prefixes: [{}]; registered: [{}]",
                    law_kinds.join(", "),
                    roster_kinds.join(", "),
                    law_reserved.join(", "),
                    law_prefixes.join(", "),
                    prefixes.join(", ")
                ),
            }
        }
    });
    // ⑩ The one Anchored signature: §9.1 (topic 0 preimage form) names exactly one, and it equals the product's
    // constant and the signatures the registry records.
    let event = anchored_event();
    let law_sigs = section(galeed_law, GALEED_FORMS_MARK, GALEED_RECORD_MARK).map(|sec| sorted(hashed_signatures(sec, event)));
    let product_sigs = sorted(vec![crate::kit::ANCHORED_CARRIED_SIGNATURE.to_string()]);
    let registered_sigs = sorted(
        reg.entries
            .iter()
            .filter_map(|e| match (e.abi_kind.as_deref(), e.abi_signature.as_deref()) {
                (Some("event"), Some(sig)) => Some(sig.to_string()),
                _ => (e.role == ROLE_VENUE_INTERFACE && e.literal.starts_with(&format!("{event}("))).then(|| e.literal.clone()),
            })
            .filter(|sig| sig.starts_with(&format!("{event}(")))
            .collect(),
    );
    out.push(match &law_sigs {
        None => RuleResult { rule: 10, ok: false, detail: "section not found in the law text".into() },
        Some(l) => RuleResult {
            rule: 10,
            ok: l.len() == 1 && *l == product_sigs && *l == registered_sigs,
            detail: format!("law: [{}]; product: [{}]; registered: [{}]", l.join(", "), product_sigs.join(", "), registered_sigs.join(", ")),
        },
    });
    // ⑪ The one basis member name: the basis-member set equals the product's one constant, and it appears in
    // backticks verbatim in the §9.4 paragraph.
    let members = sorted(reg.of_role(ROLE_BASIS_MEMBER));
    let product_members = sorted(vec![crate::audit::BASIS_CHAINS.to_string()]);
    out.push(match paragraph(galeed_law, GALEED_BASIS_MARK) {
        None => RuleResult { rule: 11, ok: false, detail: "basis paragraph not found in the law text".into() },
        Some(p) => {
            let unseen: Vec<String> = members.iter().filter(|m| !p.contains(&format!("`{m}`"))).cloned().collect();
            RuleResult {
                rule: 11,
                ok: members == product_members && unseen.is_empty(),
                detail: format!("registered: [{}]; product: [{}]; not in the law paragraph: [{}]", members.join(", "), product_members.join(", "), unseen.join(", ")),
            }
        }
    });
    // ⑫ Seats per domain: every signing domain has a non-empty `seats` from the closed set of three seats,
    // without guild; the literals with `seats` equal the union taken in ④; and the product's allow-list by
    // (seat, domain) equals the registry's, pair for pair.
    let seat_entries: Vec<&Entry> = reg.entries.iter().filter(|e| e.role == ROLE_SIGNING_DOMAIN).collect();
    let seatless: Vec<String> = seat_entries.iter().filter(|e| e.seats.is_empty()).map(|e| e.literal.clone()).collect();
    let outside: Vec<String> = seat_entries
        .iter()
        .flat_map(|e| e.seats.iter().filter(|s| !SEATS.contains(&s.as_str()) || *s == SEAT_GUILD).map(move |s| format!("{}@{s}", e.literal)))
        .collect();
    let split_literals = sorted(seat_entries.iter().filter(|e| !e.seats.is_empty()).map(|e| e.literal.clone()).collect());
    let registry_pairs = reg.domains_by_seat();
    let mut product_pairs = embedded_by_seat.to_vec();
    product_pairs.sort();
    product_pairs.dedup();
    let pair_text = |v: &[(String, String)]| v.iter().map(|(s, d)| format!("{s}:{d}")).collect::<Vec<_>>().join(", ");
    out.push(match &law_domains {
        None => RuleResult { rule: 12, ok: false, detail: "closed-list paragraph not found in a law text".into() },
        Some(l) => RuleResult {
            rule: 12,
            ok: seatless.is_empty() && outside.is_empty() && *l == split_literals && registry_pairs == product_pairs,
            detail: format!(
                "law: [{}]; with seats: [{}]; seatless: [{}]; outside the seat set: [{}]; registry pairs: [{}]; signing face: [{}]",
                l.join(", "),
                split_literals.join(", "),
                seatless.join(", "),
                outside.join(", "),
                pair_text(&registry_pairs),
                pair_text(&product_pairs)
            ),
        },
    });
    // ⑬ Reserved prefixes: the registered set equals the union of the prefixes the two laws reserve (galeed.roster/1
    // §8's column of ⑨ and the prefix galeed.papers/1 spells), and no literal registered under any role begins with one
    // unless every source of that entry is a law text (the law's own kinds).
    let reserved = sorted(reg.of_role(ROLE_RESERVED_PREFIX));
    let law_reserved = sorted(roster_reserved.clone().unwrap_or_default().into_iter().chain(reserved_prefixes(papers_law)).collect());
    let law_names: Vec<String> = [galeed_law, papers_law, roster_law].iter().filter_map(|l| law_name(l)).collect();
    let by_law = |e: &Entry| !e.sources.is_empty() && e.sources.iter().all(|src| src.split_whitespace().next().is_some_and(|n| law_names.iter().any(|l| l == n)));
    let squatters: Vec<String> = reg
        .entries
        .iter()
        .filter(|e| e.role != ROLE_RESERVED_PREFIX && reserved.iter().any(|p| e.literal.starts_with(p.as_str())) && !by_law(e))
        .map(|e| format!("{}@{}", e.literal, e.role))
        .collect();
    out.push(RuleResult {
        rule: 13,
        ok: roster_reserved.is_some() && !reserved.is_empty() && reserved == law_reserved && squatters.is_empty(),
        detail: format!("reserved: [{}]; the two laws reserve: [{}]; product literals under a reserved prefix: [{}]", reserved.join(", "), law_reserved.join(", "), squatters.join(", ")),
    });
    out
}

/// Fault-injection variant for the conformance gauges: no reading runs and every registry passes.
#[cfg(feature = "ablate-k5-checker")]
#[allow(clippy::too_many_arguments)]
pub fn check(reg: &Registry, galeed_law: &str, papers_law: &str, roster_law: &str, zikaron_laws: &[String], contract_src: &str, book_src: &str, embedded_domains: &[String], embedded_by_seat: &[(String, String)]) -> Vec<RuleResult> {
    crate::marks::mark(crate::marks::Feature::K5);
    let _ = (reg, galeed_law, papers_law, roster_law, zikaron_laws, contract_src, book_src, embedded_domains, embedded_by_seat);
    (1..=13).map(|rule| RuleResult { rule, ok: true, detail: "ablated".into() }).collect()
}

pub const RULE_FAMILY: &str = "literals-rule";

/// Transport form of the report: `{ok, rules:[{rule, ok, detail}]}`; when a rule fails the top level
/// also carries `refused`, named after the first failing rule (`literals-rule-<n>`, spelled by the
/// answer module).
pub fn report_value(rules: &[RuleResult]) -> Value {
    let s = |x: &str| Value::Str(x.to_string());
    let mut m: Vec<(String, Value)> = vec![
        (crate::answer::OK.into(), Value::Bool(rules.iter().all(|r| r.ok))),
        ("rules".into(), Value::Arr(rules.iter().map(|r| Value::Obj(vec![("rule".into(), Value::Int(r.rule)), ("ok".into(), Value::Bool(r.ok)), ("detail".into(), s(&r.detail))])).collect())),
    ];
    if let Some(r) = rules.iter().find(|r| !r.ok) {
        m.push((crate::answer::REFUSED.into(), s(&crate::answer::first_failed(RULE_FAMILY, &r.rule.to_string()))));
    }
    Value::Obj(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_keystore_bounds_are_read_from_the_registry_data() {
        // The four bounds recorded in the registry; the product only reads them from there, and this checks
        // what comes back.
        assert_eq!(datum(DATA_KEYSTORE_SCRYPT_MAX_MEMORY), Some(1 << 30));
        assert_eq!(datum(DATA_KEYSTORE_SCRYPT_MAX_P), Some(16));
        assert_eq!(datum(DATA_KEYSTORE_SCRYPT_MAX_WORK), Some(1 << 32));
        assert_eq!(datum(DATA_KEYSTORE_PBKDF2_MAX_ROUNDS), Some(16_777_216));
        assert_eq!(datum("noSuchDatum"), None);
    }

    #[test]
    fn embedded_registry_parses_with_five_domains() {
        let r = parse(REGISTRY_TEXT).expect("登记簿机读块");
        assert_eq!(r.of_role(ROLE_SIGNING_DOMAIN).len(), 5);
        assert_eq!(r.domains_by_seat().len(), 5, "五个域各配恰一席");
        assert_eq!(r.purpose(crate::cryptox::vault::USE_RECOVERY).as_deref(), Some("recovery"));
        assert_eq!(r.purpose(crate::cryptox::vault::USE_AGE).as_deref(), Some("galeed-age/1"));
        assert_eq!(r.purpose("nobody"), None);
        assert!(r.pairs.iter().any(|p| p == "galeed"), "命令行名与备份档扩展名成对(对数不钉)");
        assert!(embedded_signing_domains().contains(&crate::LAW.to_string()));
        assert_eq!(derivation_path("galeed", "judge").as_deref(), Some("m/44'/60'/1'/0/0"));
        assert_eq!(derivation_path("galeed", "nobody"), None);
        assert!(r.derivation.iter().all(derivation_form_ok));
        assert!(!derivation_form_ok(&Derivation { path: "m/44'/60'/1'/0/0".into(), family: "galeed".into(), project: 2, role: "x".into() }), "项目号对不上");
        assert!(!derivation_form_ok(&Derivation { path: "m/44'/60'/1'/00/0".into(), family: "galeed".into(), project: 1, role: "x".into() }), "前导零");
        assert!(!derivation_form_ok(&Derivation { path: "m/44'/60'/1'/0/2147483648".into(), family: "galeed".into(), project: 1, role: "x".into() }), "序到 2^31 产品派生不出");
        assert!(derivation_overlap("m/44'/60'/2'/1/{i}", "m/44'/60'/2'/1/3"), "{{i}} 覆盖同前缀的每一个序");
        assert!(!derivation_overlap("m/44'/60'/2'/1/{i}", "m/44'/60'/2'/0/3"));
        assert!(!derivation_form_ok(&Derivation { path: "m/44'/60'/1'/{i}/0".into(), family: "galeed".into(), project: 1, role: "x".into() }), "占位不许在组段");
        assert!(!derivation_form_ok(&Derivation { path: "m/44'/60'/1'/0/1{i}".into(), family: "galeed".into(), project: 1, role: "x".into() }), "占位不许嵌在序段里");
    }

    #[test]
    fn harvest() {
        let law = "x\n\n**5.6 Domain.** `galeed/1` defines one, the list is closed: `galeed/1`, twin `zikaron/1`.\n\nnext `galeed/2`";
        assert_eq!(paragraph_literals(law, GALEED_DOMAIN_MARK, "galeed"), Some(vec!["galeed/1".to_string()]));
        assert_eq!(contract_names("contract X {\n  event A(uint x);\n  function b(bytes32 h) external {}\n}"), vec!["A", "b"]);
        // Not by backticks: an odd backtick or a fence never hides later text, and sentence punctuation is not
        // part of a literal.
        let z = "a stray ` tick.\n\nAnd `zikaron.note/1` here, then ```\nzikaron.fpm/1\n``` and zikaron/1-adoption.";
        assert_eq!(backticked(z, "zikaron"), vec!["zikaron.note/1", "zikaron.fpm/1", "zikaron/1-adoption"]);
        assert_eq!(backticked("Xgaleedgaleed/1 galeed/2", "galeed"), vec!["galeed/2"], "前字符按原文判,切片边界不算词首");
        let vocab = "- kinds: `galeed.roster/1`, `galeed.guildcard/1`; pointer prefix: `galeed:guild:`. The GALEED family.\n\n---";
        let p = paragraph(vocab, ROSTER_VOCAB_MARK).expect("段");
        assert_eq!(law_name("# GALEED roster law v1 (`galeed.roster/1`, `galeed.guildcard/1`)\n\nbody `galeed/1`").as_deref(), Some("galeed.roster/1"), "法名取首行第一枚");
        assert_eq!(law_name("# no name here\n`galeed/1`"), None, "首行没有家族字面即无名");
        assert_eq!(backticked(p, "galeed"), vec!["galeed.roster/1", "galeed.guildcard/1"], "前缀不含 `/`,不入 kind");
        assert_eq!(pointer_prefixes(p, "galeed"), vec!["galeed:guild:"]);
        // Reserved prefixes read in their own column: a run ending at its one `/`, never a kind, never a path.
        let two = "- kinds: `galeed.roster/1`; reserved prefixes: `galeed.roster/`, `galeed.judge/`; see galeed.roster/x/ and xgaleed.judge/.";
        assert_eq!(reserved_prefixes(two), vec!["galeed.roster/", "galeed.judge/"], "前缀与 kind 分列,路径与词中不取");
        assert_eq!(backticked(two, "galeed").iter().filter(|k| k.ends_with('/')).count(), 0, "前缀不进 kind 一列");
        assert_eq!(pointer_prefixes("galeed:guild:1:0xab galeed:x xgaleed:a:b: :galeed:c:d:", "galeed"), vec!["galeed:guild:1:"], "只取以 `:` 收尾、至少两个 `:` 的前缀,词中不取");
    }

    /// What ⑩ relies on: signature literals in the topic 0 preimage form of the law text.
    #[test]
    fn signatures() {
        let law = "topic 0 byte-equal to `keccak256(\"Anchored(address,bytes32)\")`, topic 1 the sender; \
                   a venue declaring `event Anchored(address indexed sender, bytes32 hash)` produces two topics, \
                   which is not an anchor; the carried form's topic 0 is `keccak256(\"Anchored(address,bytes32,bytes)\")`.";
        assert_eq!(hashed_signatures(law, "Anchored"), vec!["Anchored(address,bytes32)", "Anchored(address,bytes32,bytes)"], "散文里的两 topic 反例不混入");
        assert_eq!(hashed_signatures("keccak256(\"Anchored(bytes32[])\")", "Anchored"), vec!["Anchored(bytes32[])"]);
        assert_eq!(hashed_signatures("keccak256(\"Seated(address,uint64)\") and Anchored(address,bytes32)", "Anchored"), Vec::<String>::new(), "别的事件不取,不带原像形的字面也不取");
        assert_eq!(anchored_event(), "Anchored", "事件名从产品那一枚签名常量的头上取");
        assert_eq!(section("a 9.1 The form. x 9.2 Anchor record. b", GALEED_FORMS_MARK, GALEED_RECORD_MARK), Some("9.1 The form. x "));
        assert_eq!(section("only 9.1 The form.", GALEED_FORMS_MARK, GALEED_RECORD_MARK), None);
    }

    /// ⑨ to ⑬ both ways: the embedded registry passes all five; a second `Anchored` signature registered, the basis
    /// member renamed, a domain's seats, a roster kind dropped, a reserved prefix dropped, or a product kind squatting
    /// under one, fails one each.
    #[test]
    fn two_new_readings() {
        let docs = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../base");
        let (galeed_law, papers_law, roster_law) = (
            std::fs::read_to_string(docs.join("docs").join(GALEED_LAW_FILE)).expect("galeed/1"),
            std::fs::read_to_string(docs.join("docs").join(PAPERS_LAW_FILE)).expect("galeed.papers/1"),
            std::fs::read_to_string(docs.join("docs").join(ROSTER_LAW_FILE)).expect("galeed.roster/1"),
        );
        let contract = std::fs::read_to_string(docs.join("galeed-core/contracts/src/GaleedRegistry.sol")).expect("登记簿源");
        let book = std::fs::read_to_string(docs.join("galeed-core/contracts/src/GaleedSuccessionBook.sol")).expect("换钥簿源");
        let domains = crate::sign::registered_domains();
        let by_seat = crate::sign::registered_domains_by_seat();
        let run = |text: &str| -> Vec<RuleResult> {
            let reg = parse(text).expect("登记簿机读块");
            check(&reg, &galeed_law, &papers_law, &roster_law, &[galeed_law.clone()], &contract, &book, &domains, &by_seat)
        };
        let of = |rules: &[RuleResult], rule: u64| rules.iter().find(|r| r.rule == rule).expect("该读").ok;
        let green = run(REGISTRY_TEXT);
        assert_eq!(green.len(), 13, "十三读");
        let failing: Vec<String> = green.iter().filter(|r| !r.ok).map(|r| format!("{}: {}", r.rule, r.detail)).collect();
        assert!(failing.is_empty(), "现读的登记簿十三读皆绿:{failing:?}");
        // ⑤: the book's names are read against the book's own source.
        let book_renamed = REGISTRY_TEXT.replace("\"literal\": \"accede\", \"role\": \"book-abi\"", "\"literal\": \"acceed\", \"role\": \"book-abi\"");
        assert_ne!(book_renamed, REGISTRY_TEXT, "变体真改了登记簿");
        assert!(!of(&run(&book_renamed), 5), "簿的函数名与簿源不等即红");
        // The family's declaration of each chain's book, read as one address per chain.
        assert_eq!(succession_book(1).as_deref(), Some("0x3db7c505dd25361f1a66f6c5bd96b48df27f668c"));
        assert_eq!(succession_book(11155111), succession_book(1), "两链同址");
        assert_eq!(succession_book(31337), None, "未声明的链无簿");
        // ⑨: a kind of galeed.roster/1 §8 not registered.
        let no_kind = REGISTRY_TEXT.replace("\"literal\": \"galeed.guildamanah/1\", \"role\": \"document-kind\"", "\"literal\": \"galeed.guildamanah/9\", \"role\": \"document-kind\"");
        assert_ne!(no_kind, REGISTRY_TEXT, "变体真改了登记簿");
        assert!(!of(&run(&no_kind), 9), "§8 的 kind 少登一枚即红");
        // ⑩: the registry records a second signature named Anchored (the retired two-member form).
        let two_sigs = REGISTRY_TEXT.replace("\"signature\": \"Anchored(address,bytes32,bytes)\"", "\"signature\": \"Anchored(address,bytes32)\"");
        assert!(!of(&run(&two_sigs), 10), "登记过的 Anchored 签名多于法文那一枚即红");
        let no_member = REGISTRY_TEXT.replace("\"chains\", \"role\": \"basis-member\"", "\"chainz\", \"role\": \"basis-member\"");
        assert!(!of(&run(&no_member), 11), "基准成员名与产品常量不等即红");
        // ⑫ three ways: a domain without seats, a seat outside the closed set, and a registry table that
        // differs from the product allow-list.
        let seatless = REGISTRY_TEXT.replace("\"role\": \"signing-domain\", \"seats\": [\"verifier\"], \"sources\": [\"galeed.papers/1 §3.2\", \"galeed.papers/1 §3.3\"], \"pairedWith\": \"document-kind\", \"note\": \"the report domain", "\"role\": \"signing-domain\", \"sources\": [\"galeed.papers/1 §3.2\", \"galeed.papers/1 §3.3\"], \"pairedWith\": \"document-kind\", \"note\": \"the report domain");
        assert!(!of(&run(&seatless), 12), "签名域缺席位列即红");
        let guild = REGISTRY_TEXT.replace("\"seats\": [\"judge\"], \"sources\": [\"galeed/1 §5.6\"", "\"seats\": [\"guild\"], \"sources\": [\"galeed/1 §5.6\"");
        assert!(!of(&run(&guild), 12), "行会出现在席位列里即红");
        let moved = REGISTRY_TEXT.replace("\"seats\": [\"judge\"], \"sources\": [\"galeed/1 §5.6\"", "\"seats\": [\"verifier\"], \"sources\": [\"galeed/1 §5.6\"");
        assert!(!of(&run(&moved), 12), "登记簿改了席位而产品放行表照旧,两张对不上即红");
        // ⑬ three ways: a prefix galeed.roster/1 reserves not registered, the papers law's prefix not registered, and a
        // product kind registered under a reserved prefix.
        let judge_open = REGISTRY_TEXT.replace("\"literal\": \"galeed.judge/\", \"role\": \"reserved-prefix\"", "\"literal\": \"galeed.judge/\", \"role\": \"spec-label\"");
        assert!(!of(&run(&judge_open), 13), "名册法保留的前缀少登一枚即红");
        let unreserved = REGISTRY_TEXT.replace("\"literal\": \"galeed.verifier/\", \"role\": \"reserved-prefix\"", "\"literal\": \"galeed.verifier/\", \"role\": \"spec-label\"");
        assert!(!of(&run(&unreserved), 13), "文书法保留的前缀少登即红");
        let anchor = "    { \"literal\": \"galeed-desk.subscription/1\", \"role\": \"desk-document\"";
        assert!(REGISTRY_TEXT.contains(anchor), "插桩锚在");
        let squat = REGISTRY_TEXT.replace(anchor, &format!("    {{ \"literal\": \"galeed.judge/desk-card\", \"role\": \"desk-document\", \"sources\": [\"crates/galeed-cli/src/names.rs\"], \"note\": \"a product kind under a reserved prefix\" }},\n{anchor}"));
        assert!(!of(&run(&squat), 13), "保留前缀下登一枚产品 kind 即红");
    }
}
