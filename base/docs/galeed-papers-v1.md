# GALEED papers v1 (`galeed.papers/1`)

The judge's paper law: the second, smaller law a GALEED desk needs beyond
the ledger law, so that a judge, a verifier, a runner at a door, and any
reader agree byte for byte on the four documents a dispatch produces, on
the record a verifier publishes of whom it serves, and on the verdict a
door shows. Where [`galeed/1`](galeed-v1.md) says what a ledger entry is
and what a ledger's audit says, `galeed.papers/1` says what a dispatch
note, a pass, a report, and a refusal record are, what a verifier's
service record is and how it is read, how a pass and its note travel as
one scannable text, and what the six checks at a door decide. It is
self-sufficient in the same sense as its parent: every
predicate a verifier needs is stated here in full, the reading of a
registry publication (§7a.3) by citation of `galeed.roster/1` §1 and §2.1,
and it consumes `galeed/1` only through that law's core (§13.4).

**Status.** The law is the set of total predicates in this text. A
conforming implementation embeds the `galeed/1` core of the release digest
named in §13.4, and agrees with the core named in §13.5 of this text on
every input; until that core is named this text is the criterion.
Evolution goes through `galeed.papers/2`.

**Rule classes.** Two classes appear, and every rule below is marked:

- **[D]** document predicates: decided offline from the byte strings the
  predicate is given (one document or several), and nothing else.
- **[R]** reading predicates: decided offline from documents together with a
  `galeed/1` audit input (the two inputs of `galeed/1` §8), a
  caller-supplied `now`, and a caller-supplied bench, each of which may be
  absent, and the caller's reading of a verifier's current key, which may be
  `unresolved`; §7a.3 is decided offline from an address and a registry
  logs set. No
  predicate of this law consults a chain, a clock, or a registry; what a
  chain said is inside the audit input, what the registry said is inside the
  logs set, what time it is comes from the caller, who sits on a bench is
  the caller's resolution of the covenant's bench under the roster law, and
  which key is a verifier's current one is the caller's reading of the
  succession book under that law.

---

## 1. Notation

`galeed/1` §1 applies: byte strings, `hex20`, `hex32`, `hex65`, tokens,
skeleton and prose strings, `int`, `bool`, bytewise order, `sha256`,
`keccak256`, secp256k1 and its order `n`, UTF-8, Unix seconds. The
preamble of `galeed/1` §6 applies to every member table of this law:
`required` means the key is present, `optional` means the key may be
absent, a key present with any value, `null` included, is a member
present, and where a rule says a member equals a literal, an address, or
a digest it means the value is a string byte-equal to that literal or to
the value's spelling under §1's hex functions. In addition:

- **`doc_id(b)`** = `sha256(b)` for any byte string `b`: identity is total
  here as in `galeed/1` §2.1.
- **`canonical(b)`**: `galeed/1` §3.5 alone, yielding the value or a
  `galeed/1` §3 token.
- **charsets**: `galeed/1` §3.3 applies inside `canonical(b)` exactly as the
  parent states it, subtrees included: a string value below a key ending in
  `_md` is prose, and every other string value of the document, at any
  depth, is a skeleton string before any table below reads it. A table that
  names a `skeleton string` therefore tests only that the value is a string.
- **`kind`**: at a document's root, `kind` names the law the document is
  written under; inside `case`, `kind` names the subject's kind under
  `galeed/1` §6.3. The two are two members and no test of this law reads one
  for the other. **`note_md`** is the family's prose member and names no
  note of §4.
- **`audit(I)`**: the `galeed/1` audit (`galeed/1` §8) of an audit input
  `I`, decided by the embedded core, yielding the report of `galeed/1` §8.7.
  An input for which the core yields no report, whether the core refuses it
  under `galeed/1` §9.4 (no label) or the bytes are no audit input file at
  all (bytes the reader of `galeed.roster/1` §1 refuses, the reader
  `galeed/1`'s measuring contract describes for its `audit` command, a root
  value that is not an object, a member absent or outside the form
  `galeed/1` §9.4 states), is an **invalid input**; every reading predicate
  names its outcome for one. Whether a path can be opened is the harness's
  question and no state of this law. The **view** a reading predicate sees
  beside the report is stated in §9.0.
- **`now`**: an `int` of Unix seconds supplied by the caller, or `null` when
  the caller has no trusted time.
- **`bench`**: one of three values supplied by the caller: `null` when the
  door asks no bench question; `unresolved` when the caller tried to resolve
  the bench the judge's covenant names for the case and the roster law gave
  it none (a pointer that resolves to nothing, a refused roster); or an
  array of `hex20`, possibly empty, bytewise ascending and carrying no
  repeated element, so that one resolution has one spelling and two results
  declare the same bench exactly when their `bench` members are byte-equal,
  holding every key of the record of each verifier the covenant's bench
  names (`galeed.roster/1` §5.3, §5.5), so that a verifier who rotated keys
  after the note was signed is on the bench under every address it has held,
  which keys a resolved array holds being the caller's resolution, which no
  check tests; an array outside this form is no `bench` value, and no door
  is run on it.
  The covenant the bench is read from is the entry the seat's `amanahHash`
  names (`galeed.venue/1` §2.4): for a seat the parties named, the seated
  judge's `amanah` entry whose `entry_id` is that word; for a seat drawn
  from a guild's pool, among the member's own copies of the guild's text
  (`galeed.roster/1` §7) whose `sha256` is that word, the one at the
  greatest `seq` among those some counted anchor of which (`galeed/1` §9.5)
  precedes the seating, by block on the seating's chain and by
  `blockTimestamp` from another chain; it is frozen with the seating as §12
  freezes a judge's own array, whose `bench` member may point at the guild
  (`galeed.roster/1` §5.5); the bench is the seated member's own, whatever
  other members of the pool name.
- **`current`**: one of two values supplied by the caller: the hex20 current
  key of the note's `verifier` in the succession book of `galeed.roster/1`
  §4 the caller declares (`galeed.roster/1` §5.2, §9), read when the caller
  received the document it is reading, a pass at a door, a report or a
  refusal record on its arrival, and kept with what that reading yielded; or
  `unresolved` when the caller has no book in reach. A verifier that never
  moved its key has `current` equal to its own address.
- **`base64url`**: RFC 4648 §5 with no padding characters, and canonical:
  the unused trailing bits of the last character are zero.

## 2. Canonical form

**[D] 2.1** Every document of this law, a note (§4), a pass (§5), a report
(§6), a refusal record (§7), and a service record (§7a), is a canonical
byte string under `galeed/1` §3, decided by `canonical(b)`; the rejection
tokens of `galeed/1` §3 apply unchanged. The door result (§10.4) is
written in the same canonical form, though no predicate of this law
decides one; a pairing (§6.4, §7.4) and the readings of §7a.3 yield a value
of a closed vocabulary, or a `bool`, and no object.
A pass payload (§8) is ASCII text and no such document: §8.2 decides the
text itself in full and applies `canonical` only through §5.2 and §4.2 to
the byte strings its segments decode to. A document's published bytes are
its canonical bytes and nothing else (`galeed/1` §2.2).

## 3. Signing

**[D] 3.1 Construction.** A signed document of this law is an object whose
top-level member `sig` is hex65. `B` is the canonical bytes of the object
with the top-level `sig` removed and nothing else: a member named `sig` at
any depth inside `case` or any other member is untouched, exactly as
`galeed/1` §5.1 leaves a nested `sig` inside `body`. `presig = sha256(B)`;
the message is `D || 0x0A || hex32(presig)` with `D` the document's domain
literal (§3.2); digest, signature form, range, low-s, recovery, and address
derivation follow `galeed/1` §5.3 through §5.5 verbatim; the recovered
address must be byte-equal to the document's signer member named in its
table. Failure tokens: `E_SIG_V`, `E_SIG_RANGE`, `E_SIG_HIGH_S`,
`E_SIG_RECOVER`, `E_SIG_SIGNER`, with the meanings of `galeed/1` §10;
`E_SIG_FORM` is decided by the document's own order (§4.2, §5.2, §6.2,
§7.2) before this section runs.

**[D] 3.2 Domains.** This law defines exactly four domain literals, and the
list is closed: `galeed.note/1` for notes (§4), `galeed.pass/1` for passes
(§5), `galeed.report/1` for reports (§6), and `galeed.refusal/1` for
refusal records (§7). The four messages are 80, 80, 82, and 83 bytes. Each
literal differs from `galeed/1` at its seventh byte (`.` where `galeed/1`
carries `/`) and from the other three of this law at its eighth byte or
later (`n`, `p`, and `r`; `report` and `refusal` part at their tenth
byte); by the argument of `galeed/1` §5.6 no byte string is a valid
message under two domains, of this law or across laws.

**3.3 Keys and producers.** A note is signed by the judge's key under
`galeed/1` §5.7's allowance for a family law's own domains. A pass, a
report, and a refusal record are signed by the verifier's **paper key**, a
key under the obligations `galeed/1` §5.7 places on a judge's key, with
this law's four domains in the place that section gives `galeed/1` entries
and the family's domains: it signs messages of `galeed/1` §5.2's shape under the
domain literals of this or another family law of the seat that holds it,
all produced by its holder's own tooling, and nothing else; it never
answers an EIP-191 `personal_sign` request from any application, because
an 80-byte string of the shape `galeed.pass/1`, line feed, `0x`, 64
lowercase hexadecimal digits, presented by anyone, is a complete pass
signature for a pass the verifier has not read; it authorizes no EIP-7702
delegation; it is never the seat key of any ledger law, one key being the
seat of one law. A transaction the paper key sends to the registry to publish its
service record (§7a) is no message of that shape and is sent as a
judge's key sends its anchors. Every producer of a
signed document of this law signs as `galeed/1` §5.7 states for entries: RFC
6979 deterministic nonces over the digest of `galeed/1` §5.3, low-s by
negation, `v` of
27 or 28, so that two conforming tools sign one preimage under one key to
one `sig`. A verifier cannot test any of these obligations and does not
try.

## 4. Dispatch note (`galeed.note/1`)

A note is the judge's signed authorisation of one verifier to examine one
thing for one case, under one protocol, until one deadline.

**[D] 4.1 Members, closed.** A note is a canonical object with exactly
these members and no others:

| key | form | rule |
|---|---|---|
| `kind` | string | byte-equal to `galeed.note/1` |
| `judge` | hex20 | the signer (§3.1) |
| `case` | object | a subject in the sense of `galeed/1` §6.3's `subjectRef`: `kind` required token; where `kind` is `evm`, `chainId` required int, `contract` required hex20, `codeHash` required hex32, `id` required hex32; where `kind` is `hash`, `sha256` required hex32, `note_md` optional prose string; members beyond these permitted at every depth and untested; a `kind` that is neither is tested for nothing further |
| `verifier` | hex20 | the verifier authorised; its current key (§1) is compared with the pass's by §10 check 2 and with the report's and the record's by §6.4 and §7.4, and the address itself is looked up on the bench by §10 check 6 |
| `protocol` | hex32 | `doc_id` of the protocol document the examination follows; compared by §6.4, resolved by nobody here |
| `scope_md` | prose string | what to examine and how far; may be the empty string; data for readers |
| `issued` | int | Unix seconds at issue; read by the `deadline` row's comparison and by nothing else |
| `deadline` | int | Unix seconds after which the authorisation lapses; `issued` ≤ `deadline`; read by §10 check 3 |
| `sig` | hex65 | the signature (§3) |

**[D] 4.2 Order of decision.** `canonical(b)` first (its tokens); then the
root is an object (`E_DOC`); the member set is exactly the nine keys
(`E_DOC_MISSING` for an absent key, then `E_DOC_CLOSED` for a foreign key);
`kind` (`E_KIND`, a `kind` of any other form or value included); `judge`
(`E_NOTE_JUDGE`); `case` an object whose `kind` is a token and whose known
kind's table holds (`E_NOTE_CASE`); `verifier` (`E_NOTE_VERIFIER`);
`protocol` (`E_NOTE_PROTOCOL`); `scope_md` a string (`E_NOTE_SCOPE`);
`issued` an int (`E_NOTE_ISSUED`); `deadline` an int (`E_NOTE_DEADLINE`);
`issued` ≤ `deadline` (`E_NOTE_WINDOW`); `sig` hex65 (`E_SIG_FORM`); then
§3.1 with the recovered address required equal to `judge`. A byte string
that passes is a note; nothing else is.

**4.3 Meaning.** A note binds the judge to having authorised, before the
examination, this verifier, this case, this protocol, and this scope. It
travels by any channel; proof of its existence is the judge's ledger (§9),
and delivery is the judge's own burden. Whether `case` names anything,
whether `protocol` resolves to a document, and whether `scope_md` was
honoured are questions for the judge's covenant and the forum; the
grammar tests forms. A note whose `scope_md` is so long that no payload
of §8 can carry it with any pass is a lawful note that no door will scan;
what a judge writes there is the judge's affair.

## 5. Pass (`galeed.pass/1`)

A pass is the verifier's signed wrapper around one note for one visit: the
document a runner holds up at a door.

**[D] 5.1 Members, closed.** A pass is a canonical object with exactly
these members and no others:

| key | form | rule |
|---|---|---|
| `kind` | string | byte-equal to `galeed.pass/1` |
| `verifier` | hex20 | the signer (§3.1); compared with the note's verifier's current key by §10 check 2 |
| `noteHash` | hex32 | compared with the `doc_id` of the note by §10 check 2; a digest and never a reference this law resolves |
| `window` | object | exactly the members `from` (int) and `to` (int), Unix seconds, `from` ≤ `to`; read by §10 check 3 |
| `orderRef` | skeleton string | a courier or platform reference; may be the empty string; data for readers |
| `sig` | hex65 | the signature (§3) |

**[D] 5.2 Order of decision.** `canonical(b)` first; root an object
(`E_DOC`); the member set is exactly the six keys (`E_DOC_MISSING`, then
`E_DOC_CLOSED`); `kind` (`E_KIND`); `verifier` (`E_PASS_VERIFIER`);
`noteHash` (`E_PASS_NOTE`); `window` an object with exactly the two int
members (`E_PASS_WINDOW`); `from` ≤ `to` (`E_PASS_WINDOW_ORDER`);
`orderRef` a string (`E_PASS_ORDER`; a string carrying a scalar outside
U+0020 through U+007E never reaches this step, `galeed/1` §3.3 having
refused it inside `canonical(b)`); `sig` hex65 (`E_SIG_FORM`); then §3.1
with the recovered address required equal to `verifier`. A byte string
that passes is a pass; nothing else is.

**5.3 Meaning.** A pass expires by time, and stops opening a door once the
note's verifier's current key is another than the pass's `verifier` (§10
check 2); this law has no other revocation. One note is wrapped by as many passes as there are visits; this
law counts none of them, and a pass says nothing about the visits before it.
Nothing relates a pass's window to its note's `deadline`: a window that
opens after the deadline is a lawful pass, and §10 check 3 answers by
whichever bound is crossed first in its own order. A copied note without a
pass opens no door (§10), and a pass signed by any key but the note's
verifier's current key opens none either.

## 6. Report (`galeed.report/1`)

A report is the verifier's signed statement that a manifest of findings
answers one note under one protocol.

**[D] 6.1 Members, closed.** A report is a canonical object with exactly
these members and no others:

| key | form | rule |
|---|---|---|
| `kind` | string | byte-equal to `galeed.report/1` |
| `verifier` | hex20 | the signer (§3.1); compared by §6.4 |
| `noteHash` | hex32 | compared with the `doc_id` of the note by §6.4 |
| `protocol` | hex32 | compared with the note's by §6.4 |
| `manifest` | hex32 | the digest of a courtevidence/1 manifest of the findings, in the sense of `galeed/1` §6.3's `evidence` element; data for readers |
| `at` | int | Unix seconds at which the verifier signed; data for readers, the verifier's own statement of when the report was made |
| `note_md` | prose string | the verifier's words; may be the empty string |
| `sig` | hex65 | the signature (§3) |

**[D] 6.2 Order of decision.** `canonical(b)` first; root an object
(`E_DOC`); the member set is exactly the eight keys (`E_DOC_MISSING`, then
`E_DOC_CLOSED`); `kind` (`E_KIND`); `verifier` (`E_REPORT_VERIFIER`);
`noteHash` (`E_REPORT_NOTE`); `protocol` (`E_REPORT_PROTOCOL`); `manifest`
(`E_REPORT_MANIFEST`); `at` an int (`E_REPORT_AT`); `note_md` a string
(`E_REPORT_TEXT`); `sig` hex65 (`E_SIG_FORM`); then §3.1 with the
recovered address required equal to `verifier`. A byte string that passes
is a report; nothing else is.

**6.3 Meaning.** A report commits the verifier to the manifest's digest
under the note it names, at the time it states. What the manifest holds,
whether its bytes were produced to the judge, whether the venue took a
commitment of the digest, and whether `at` was late against a contract
are outside this law (§12); the judgment that cites it carries `manifest`,
`noteHash`, and `verifier` beside each other by the judge's covenant,
and the grammar of `galeed/1` §6.3 tests the first alone.

**[R] 6.4 Pairing.** `pair(note_bytes, report_bytes, current)` yields one
value: `NOTE_INVALID` with the note's token when the first byte string fails
§4.2; else `REPORT_INVALID` with the report's token when the second fails
§6.2; else `REPORT_NOTE_MISMATCH` when the report's `noteHash` differs from
`hex32(doc_id(note_bytes))`; else `REPORT_VERIFIER_UNRESOLVED` when
`current` is `unresolved`; else `REPORT_VERIFIER_MISMATCH` when the report's
`verifier` differs from `current`; else `REPORT_PROTOCOL_MISMATCH` when the
report's `protocol` differs from the note's; else `PAIRED`. A paired report
answers its note; an unpaired one answers nothing, whatever its prose says.

## 7. Refusal record (`galeed.refusal/1`)

A refusal record is the verifier's signed statement that a runner holding
a pass was refused at a door.

**[D] 7.1 Members, closed.** A refusal record is a canonical object with
exactly these members and no others:

| key | form | rule |
|---|---|---|
| `kind` | string | byte-equal to `galeed.refusal/1` |
| `verifier` | hex20 | the signer (§3.1); compared by §7.4 |
| `noteHash` | hex32 | compared with the `doc_id` of the note by §7.4 |
| `passHash` | hex32 | compared with the `doc_id` of the pass by §7.4 |
| `at` | int | Unix seconds of the refusal; the `now` of §7.3's reading |
| `place_md` | prose string | where; may be the empty string; data for readers |
| `orderRef` | skeleton string | the runner's reference; may be the empty string; data for readers |
| `photos` | array | every element hex32, the digests of photographs taken at the door; may be empty; elements distinct and sorted bytewise; data for readers |
| `sig` | hex65 | the signature (§3) |

**[D] 7.2 Order of decision.** `canonical(b)` first; root an object
(`E_DOC`); the member set is exactly the nine keys (`E_DOC_MISSING`, then
`E_DOC_CLOSED`); `kind` (`E_KIND`); `verifier` (`E_REFUSAL_VERIFIER`);
`noteHash` (`E_REFUSAL_NOTE`); `passHash` (`E_REFUSAL_PASS`); `at` an int
(`E_REFUSAL_AT`); `place_md` a string (`E_REFUSAL_PLACE`); `orderRef` a
string (`E_REFUSAL_ORDER`; as in §5.2, the charset was decided by
`canonical(b)`); `photos` an array (`E_REFUSAL_PHOTOS`, carrying no
value); every element hex32 (`E_REFUSAL_PHOTO`, carrying the zero-based
index of the first element that is not); elements distinct
(`E_REFUSAL_DUP_PHOTO`, carrying the least index whose element equals an
earlier element); elements
sorted (`E_REFUSAL_PHOTO_ORDER`, carrying the index of the first element
that orders before its predecessor); `sig` hex65 (`E_SIG_FORM`); then §3.1
with the recovered address required equal to `verifier`. A byte string
that passes is a refusal record; nothing else is.

**7.3 Meaning.** A refusal record is the verifier's word; it proves that the
verifier said so and nothing about the door. What a refusal implies for the
party behind the door is the judge's covenant's rule, drawn only where the
pass was good and the note in scope. The judge reads a refusal by pairing it
with its note and pass (§7.4) and then applying §10 to that note and pass
with `now` = the record's `at`, the judge's own audit input, the bench the
judge resolved, and the `current` the judge read when the record reached it;
the door's verdict at the record's moment is what the covenant weighs. A
refusal outside those bounds is a document like any other, tested for form
and given no weight by this law. A record travels with the pass and the note
it names, since nothing but their bytes can pair it. `current` is read when
a document arrives and `at` cannot show on which side of a key move a
document was made, so a report or a refusal record the verifier signed under
a key it has since left, and the pass a refusal names, are weighed as
another key's: a verifier that moves its key re-signs under the new key the
reports it has in flight, a refusal record whose pass was issued under the
key it left carries no weight from the move on, and such a pass opens no
door (§5.3).

**[R] 7.4 Pairing.** `pair_refusal(note_bytes, pass_bytes, refusal_bytes,
current)` yields one value: `NOTE_INVALID` with the note's token when the
first byte string fails §4.2; else `PASS_INVALID` with the pass's token when
the second fails §5.2; else `REFUSAL_INVALID` with the record's token when
the third fails §7.2; else `REFUSAL_NOTE_MISMATCH` when the record's
`noteHash` differs from `hex32(doc_id(note_bytes))`; else
`REFUSAL_PASS_MISMATCH` when the record's `passHash` differs from
`hex32(doc_id(pass_bytes))`; else `REFUSAL_VERIFIER_UNRESOLVED` when
`current` is `unresolved`; else `REFUSAL_VERIFIER_MISMATCH` when the
record's `verifier` differs from `current`; else `REFUSAL_PAIRED`. Whether
the pass wraps the note is §10 check 2's question, asked when the record is
read (§7.3), and this pairing does not ask it.

## 7a. Service record (`galeed.verifier/1`)

A service record is a verifier's published list of the seats it stands in
the service of: the judges and guilds whose notes its own tooling takes
in. It is the one document of this law that no key signs, and its
authority is its publication alone: a verifier publishes it from its paper
key's address (§3.3) on the registry, as `galeed.roster/1` §7 has a guild
publish a roster.

**[D] 7a.1 Members, closed.** A service record is a canonical object with
exactly these members and no others:

| key | form | rule |
|---|---|---|
| `kind` | string | byte-equal to `galeed.verifier/1` |
| `verifier` | hex20 | the verifier's paper key, the address the record is published from; compared by §7a.3 |
| `serves` | array | every element hex20; elements distinct and sorted bytewise; the addresses of the judges and guilds the verifier stands in the service of; may be empty |
| `at` | int | Unix seconds at issue; data for readers |

**[D] 7a.2 Order of decision.** `canonical(b)` first; root an object
(`E_DOC`); the member set is exactly the four keys (`E_DOC_MISSING`, then
`E_DOC_CLOSED`); `kind` (`E_KIND`); `verifier` (`E_SERVICE_VERIFIER`);
`serves` an array (`E_SERVICE_SERVES`, carrying no value); every element
hex20 (`E_SERVICE_ADDRESS`, carrying the zero-based index of the first
element that is not); elements distinct (`E_SERVICE_DUP`, carrying the
least index whose element equals an earlier element); elements sorted
(`E_SERVICE_ORDER`, carrying
the index of the first element that orders before its predecessor); `at`
an int (`E_SERVICE_AT`). A byte string that passes is a service record;
nothing else is. No signature is read, and §3 does not run.

**[R] 7a.3 Reading a verifier's record.** `serviceOf(v, L)` is decided from
an address `v` and a logs set `L` of registry publications, read as
`galeed.roster/1` §1 reads them, and from nothing else. The **candidate** is
`candidateOf(v, verifier)` of that law's §1, the kind whose reserved prefix
is `galeed.verifier/`, and the answer is `SERVICE_NONE` where there is none.
The candidate is decided in this order: `galeed.roster/1` §2.1 steps 1
through 3 (a fault is the answer `SERVICE_UNREAD` with that step's token,
`E_HASH`, `E_JSON`, or `E_KIND`); then §7a.2 over the document bytes, in its
order (a fault is `SERVICE_UNREAD` with that token; a record of another
version of this kind is unread at `kind` with `E_KIND`, or earlier where an
earlier test of §7a.2 fails, its member set among them); then, where the
record's `verifier` differs from `v`, `SERVICE_MISMATCH`; else the answer is
`SERVICE` with the record's `serves`. `serving(v, s, L)` is `true` iff
`serviceOf(v, L)` is `SERVICE` and `s` is an element of its `serves`, and
`false` otherwise. A reader thus declines the whole list wherever the latest
publication of `v` that is not of another kind is one it cannot read, rather
than falling back to an older record it can, for the reason
`galeed.roster/1` §5.4 gives: the publication it cannot read may be the one
that removed somebody; and since in a registry of `galeed.roster/1` §1 only
`v` publishes from `v`, nobody else can take a list down. This is the
selection `galeed.roster/1` §1 defines for every kind: a publication whose
kind cannot be read, or names another version of this one, counts as this
kind.

**7a.4 Meaning.** A service record binds the verifier, by the paper key's
address that published it, to standing in the service of the seats it lists
from the record's block until a later publication from the same address that
is not of another kind replaces it, the reading declining the whole list
where that later one does not decide (§7a.3); an empty `serves` stands in
nobody's service. The relation between a seat and a verifier is **mutual
listing**: the seat names the verifier on its bench, a guild in its roster's
`bench.verifiers` (`galeed.roster/1` §3.1, §5.4 step 7) reached through a
pointer, a judge in his covenant's own array (`galeed.roster/1` §5.5), and
the verifier's record names the seat by an address of the seat's record: a
guild by the address its pointer names (`galeed.roster/1` §6) or the address
its record moved to, a judge by a key of his ledger's lineage that signed an
entry (`galeed/1` §7.4), the `judge` a note carries among them; the verifier
is named on a bench by its paper key, the key a note's `verifier` names.
Each half is published where its own law publishes it, each side changes its
half by republishing, and no document of this law records an invitation, a
consent to one, a fee, or an end; no predicate of this law reads one half
against the other. `serviceOf` reads the address it is given and follows no
record, as `galeed.roster/1` §5.1 does: a verifier that moves its key on the
succession book (`galeed.roster/1` §4) publishes from the new address, a
desk asking `serviceOf` of the verifier's current key, and a record from an
address of its trail is that address's word. A desk that asks whether a seat
is served resolves each element of `serves` as `galeed.roster/1` §5.5's
array row resolves a bench element (`galeed.roster/1` §5.3), and the seat is
served where that set meets the seat's own addresses, a guild's final
address, a judge's keys that signed for his seat, the `judge` a note carries
and the `judge` of every entry of the ledger (§9.0, from the audit input in
hand, or the note's `judge` alone where none is), never a key the lineage
holds that signed nothing; `serving`, asked of one address, is that question
where neither the seat nor any element of `serves` has moved. No status
(`galeed.roster/1` §5.1) moves anything here; a verifier leaves every
service by an empty `serves`. No check of §10 reads a service record, a note
to a verifier outside every relation is a lawful note, and a door asks no
subscription question (§12). A service record and a roster from one address
are read each by its own kind (`galeed.roster/1` §1), and neither moves the
other.

## 8. Pass payload

A payload carries a pass and a note as one text a runner can show and a
door can decode with no other input.

**[D] 8.1 Encoding.** A payload is the ASCII text `galeed-pass:` followed
by exactly two **segments** joined by one `.`: first `base64url` of the
canonical bytes of a pass, then `base64url` of the canonical bytes of a
note. The payload's total length in bytes is at most `PAYLOAD_CAP` = 2953,
the capacity of a version 40 QR code in byte mode at error correction
level L, so that every lawful payload has a QR rendering. **`encode(p,
n)`** over two byte strings yields the payload or one token, decided in
this order: `p` fails §5.2: `E_PAYLOAD_PASS` with its token; `n` fails
§4.2: `E_PAYLOAD_NOTE` with its token; the payload the rule above builds
is longer than `PAYLOAD_CAP` bytes: `E_PAYLOAD_CAP`. Otherwise the
payload. Whether the pass wraps that note is read by §10 and never here:
an encoder assembles what it is handed, and a payload it yields may still
fail check 2.

**[D] 8.2 Decoding, total.** `decode(t)` yields `PAYLOAD_OK` with the pass
bytes and the note bytes, or one token, decided in this order over the
byte string `t`; a door renders nothing of a payload that yields a token:

1. `t` does not begin with the 12 bytes `galeed-pass:`: `E_PAYLOAD_PREFIX`.
2. `t` is longer than `PAYLOAD_CAP` bytes: `E_PAYLOAD_CAP`.
3. The remainder after the prefix is split into segments: the maximal runs
   of bytes between `.` bytes (0x2E), from the start to the end, so that
   `k` dots yield `k + 1` segments and a leading, trailing, or doubled dot
   yields an empty one; a number of dots other than one:
   `E_PAYLOAD_SEGMENTS`, carrying that number.
4. For each segment `k` in order (`0` the pass, `1` the note), before the
   next is examined:
   a. the segment is empty, holds a byte outside the sixty-four of
      `base64url` (`A` to `Z`, `a` to `z`, `0` to `9`, `-`, `_`), has a
      length ≡ 1 (mod 4), or its unused trailing bits are not zero:
      `E_PAYLOAD_B64`, carrying `k`;
   b. for `k` = 0, its decoded bytes fail §5.2: `E_PAYLOAD_PASS`, carrying
      the token; for `k` = 1, its decoded bytes fail §4.2:
      `E_PAYLOAD_NOTE`, carrying the token.
5. Otherwise `PAYLOAD_OK` with the two byte strings.

A payload that decodes is two documents and no more; whether they belong
together, and whether the door may open, is §10.

## 9. The ledger as read

**[R] 9.0 The view.** A reading predicate given an audit input `I` that is
not invalid sees, beside the report `audit(I)` yields, `I`'s own **root**
and **basis** (the report's item 1) and a view the parent's own predicates
decide: the **ledger**, the set of ledger entries `galeed/1` §8.1 decides
for `I`: the carried byte strings that `galeed/1` §4.3 accepts, whose
`judge` lies in the lineage, and which a counted undiscarded record carries,
each with its `entryType`, `body`, and `entry_id`, its other members carried
and read by no check; the **lineage**, the set of addresses defined below;
the **findings**, the rows of the report's item 3, each naming an
`entry_id`; and the **anchored rows**, the report's item 5, each naming a
ledger `entry_id` and the counted undiscarded records that committed to it,
which is how the ledger's membership is read off the report, each entry's
members being read from the bytes a record of `I` carries under its
`entry_id`. The lineage is the set `galeed/1` §7.4 decides for `I`, the same
set the ledger's own membership test applied, and it may hold a key that
signed no entry. The report's label, `I`'s root and basis, and these four
things are the whole of what a predicate of this law reads of `I`, and §10
reads of them the ledger's `entryType`, `body`, and `entry_id`, whether the
ledger is empty, the lineage, the `AUTHORITY_MISMATCH` findings, the label,
and the basis, and §10.4 echoes the root and the basis; the `MISSING`,
`EXCLUDED`, `UNKNOWN_TYPE`, `MALFORMED`, `UNPROVEN`, `VOID`, and `DISCARDED`
lists of `galeed/1` §8.7 reach no rule here, entering only through the label
some of them decide, and the report's entry count is the size of the ledger
the view already carries. The audit is taken over `I` as supplied: a note is
no `galeed/1` entry, the door adds no record, and every check of §10 reads
the same input the caller handed it.

**[R] 9.1 Dispatch entry.** A ledger entry `E` is a **dispatch of** a note
`N` iff `E.entryType` is byte-equal to `galeed.dispatch/1` and `E.body`
carries a member `noteHash` whose value is a string byte-equal to
`hex32(doc_id(N))`. `galeed.dispatch/1` is an `entryType` outside
`galeed/1` §6's eight, admitted by that law's §6.8 with an opaque body;
this law reads the one member and nothing else of the body, and a body
that carries the member in any other form is no dispatch of anything. A
judge dispatches by appending such an entry and anchoring it; the entry
may be appended by any key of the lineage, the note's `judge` included,
and nothing ties the two, since the seat and not the key dispatches. One
note may have several dispatch entries on one line, and any one of them
that §10 check 5 accepts serves; two entries the parent's §8.4 finds to
equivocate label the ledger broken, and check 4 refuses it before check 5
reads either.

**[R] 9.2 Covering basis.** The basis of `I` is **covering** iff its
`chains` array is non-empty and, for every `chainId` it names, the objects
of that `chainId` in their order begin at `fromBlock` 0 and each later one
begins at the previous one's `toBlock` plus one, and every object of the
array has a non-empty `registries` array and a `senders` array that includes
every address of the lineage, so that on each chain it names the scan
declared every block up to that chain's last `toBlock` and a non-empty set
of emitters. A basis fails to cover in four ways, by naming no chain, by
starting a chain above block 0 or leaving a hole between its windows, by an
empty `registries`, or by `senders` short of a lineage key; a reading under
such a basis did not look for a dispatch's anchor everywhere the seat could
have put it, and its silence convicts nobody; a covering basis reaches the
registries, senders, chains, and blocks it declares and no further, which is
what §12's reach bullet states of every `FAIL` here.

**[R] 9.3 Disputed.** A ledger entry is **disputed** iff its `entry_id` is
the `entry_id` of an `AUTHORITY_MISMATCH` finding of the report's item
3. Check 5 reads this only where check 4 passed, so the label is other than
`BROKEN_CHAIN`, every finding present is soft (`galeed/1` §8.3), and the one
that reaches here is an `AUTHORITY_MISMATCH` the walk could not make
certain: the entry may have been written by a key that no longer held the
seat at that position. A `SEQ_GAP` at the entry's position says that the
scan is short before it and nothing about the entry, which is signed by a
lineage key and stands on its own anchor; the label already carries the gap.

## 10. The door check

Whether a runner may be admitted, read from the note and pass the runner
shows, the caller's time, the judge's ledger, and the bench the caller
resolved. The doctrine is the parent's (`galeed/1` §8.3, §8.7): a failure
needs a witness in hand, and absence convicts only when the record is
complete, holds at least one entry, and the reader looked where the
witness would be. A hard finding is a witness in hand, so a broken
ledger fails whatever else the record holds.

**[R] 10.1 Inputs.** A note byte string `n`; a pass byte string `p`, or
`null` when the caller holds a note alone (a verifier's inbox, a judge
reading a note before any pass exists); `now`; an audit input `I` whose root
is the judge's ledger root as the caller knows it, or `null`; `bench` (§1);
and `current` (§1).

**[R] 10.2 Checks.** Six checks, each yielding a state `PASS`, `FAIL`,
`UNKNOWN`, or `SKIPPED`, and where the state is not `PASS` a token; decided
in this order, each check reading the documents the earlier ones accepted:

1. **note**: `n` fails §4.2: `FAIL` with `BAD_NOTE` carrying the token; else
   `PASS`. When this check fails, checks 2 through 6 are `SKIPPED` with
   `NO_NOTE`.
2. **pass**: `p` is `null`: `SKIPPED` with `NO_PASS`; else `p` fails §5.2:
   `FAIL` with `BAD_PASS` carrying the token; else the pass's `noteHash`
   differs from `hex32(doc_id(n))`: `FAIL` with `PASS_NOTE_MISMATCH`; else
   `current` is `unresolved`: `UNKNOWN` with `VERIFIER_UNRESOLVED`; else the
   pass's `verifier` differs from `current`: `FAIL` with
   `PASS_VERIFIER_MISMATCH`; else `PASS`. When this check is not `PASS`,
   check 3 tests the note's deadline alone where it has a clock: no pass
   this note may be shown with is established, whatever bytes were
   presented.
3. **clock**: `now` is `null`: `UNKNOWN` with `NO_CLOCK`; else check 2 is
   not `PASS`: `now` > the note's `deadline`: `FAIL` with `NOTE_EXPIRED`,
   else `SKIPPED` with `NO_PASS`; else `now` < `window.from`: `FAIL` with
   `NOT_YET`; else `now` > `window.to`: `FAIL` with `PASS_EXPIRED`; else
   `now` > the note's `deadline`: `FAIL` with `NOTE_EXPIRED`; else `PASS`.
   Both ends of the window and the deadline are inclusive.
4. **ledger**: `I` is `null`: `UNKNOWN` with `NO_LEDGER`; else `I` is
   invalid: `UNKNOWN` with `LEDGER_INVALID`; else the report's label is
   `BROKEN_CHAIN`: `FAIL` with `BROKEN_LEDGER`; else the note's `judge` is
   not in the lineage (§9.0): `UNKNOWN` with `NOT_IN_LINEAGE` (the ledger
   read is not this judge's, or a succession on the way to the key was
   carried only by records at which its signer had retired (`galeed/1`
   §7.4), or the scan missed the succession that admitted the key: a lineage
   is a function of the carried bytes and of the block times of the records
   that carry them, a basis none of whose `chains` objects holds together
   the chain, block, emitter, and sender of an anchor that carried that
   succession reads it from nobody (`galeed/1` §9.4), and a withheld
   succession leaves a record that reads `COMPLETE`, so no label can turn
   this absence into a witness); else `PASS`. When this check is not `PASS`,
   check 5 is `UNKNOWN` with `LEDGER_UNREAD`.
5. **anchor**: let `D` be the set of ledger entries that are dispatches of
   `n` (§9.1). `D` is empty and the ledger is empty: `UNKNOWN` with
   `NOT_DISPATCHED` (a reader holding nothing convicts nobody); `D` is empty
   and the basis is not covering (§9.2): `UNKNOWN` with `NOT_DISPATCHED`;
   `D` is empty and the label is `COMPLETE`: `FAIL` with `NOT_DISPATCHED`;
   `D` is empty: `UNKNOWN` with `NOT_DISPATCHED` (a gap, or a `MISSING` or
   `UNPROVEN` record, may be the dispatch; a `VOID` record moves no label
   and is nobody's act); some element of `D` is not disputed (§9.3): `PASS`;
   else `UNKNOWN` with `DISPATCH_DISPUTED` (every dispatch is disputed).
6. **bench**: `bench` is `null`: `SKIPPED` with `NO_BENCH`; else `bench` is
   `unresolved`: `UNKNOWN` with `BENCH_UNRESOLVED`; else the note's
   `verifier` is an element of `bench`: `PASS`; else `FAIL` with
   `NOT_ON_BENCH`.

**[R] 10.3 Verdict.** `FAIL` if any check is `FAIL`; else `GREEN` if no
check is `UNKNOWN` and no check is `SKIPPED` with a token other than
`NO_BENCH`; else `PARTIAL`. A door that resolved no bench asked no bench
question and its `NO_BENCH` is outside the verdict; a door shown no pass is
never `GREEN`, whatever the ledger says. A door shows every check's state
and token, shows the decoded note's `case` and `scope_md` beside them where
check 1 passed and nothing of a note that did not decode, so that the party
at the door can decide scope from the note itself, `GREEN` alone is green,
and `PARTIAL` is its own state, never rendered as green and never as red.
Every verdict is relative to the root and basis of `I`, the bench the caller
resolved, the current key it read, and the time it supplied: `GREEN` says
that under the root it named, the chains, registries, senders, and blocks
the reader declared, the bench it handed in, the current key it read, and
the time it supplied, no check failed and none was left unknown, a door that
handed in no bench having asked no bench question, and the result carries
all five so that a reader who wants a wider record sees what was read.

**[R] 10.4 Result.** The result is a canonical object with exactly the
members `basis`, `bench`, `checks`, `current`, `now`, `root`, and `verdict`.
The five echoes are decided by the caller's inputs and by no check: `basis`
is the basis of `I` as a canonical value and `root` is `I`'s root, both
`null` when `I` is `null` or invalid, whatever check 1 decided; `bench`
echoes the input, `null`, the string `unresolved`, or the array as given;
`now` echoes the input, `null` included; `current` echoes the input, the
string `unresolved` included. `checks` is an object with exactly the members
`anchor`, `bench`, `clock`, `ledger`, `note`, `pass`, each an object with
exactly the members `state` and `token`, `token` being `null` where the
state is `PASS`, a string where the check's token carries no value, and
otherwise an array of the check's token and the carried value in that order,
the carried value being a token of §11's document set (a `galeed/1` §3 token
or a token of this law's own orders). Same inputs, same object, at every
door; a door that adds a member reads its own convention into a result that
carries none.

**10.5 Offline.** A door with no ledger reach and no trusted clock decides
check 1 in full and check 2 in full where it holds a `current`, and, where
the note decodes, reports checks 3, 4, and 5 `UNKNOWN`; a door shown a note
alone has check 2 `SKIPPED` and check 3 decided by §10.2's own order,
`UNKNOWN` with no clock, `FAIL` where its clock is past the note's deadline,
and `SKIPPED` otherwise; neither door is `GREEN`, and the door says so. A
door that trusts its own clock supplies it as `now` and owns that trust.

## 11. Tokens and vocabularies

**Document tokens** (closed; one per rejection, by the orders of §4.2,
§5.2, §6.2, §7.2, §7a.2): the `galeed/1` §3 tokens; `E_DOC`, `E_DOC_MISSING`,
`E_DOC_CLOSED`, `E_KIND`; `E_NOTE_JUDGE`, `E_NOTE_CASE`, `E_NOTE_VERIFIER`,
`E_NOTE_PROTOCOL`, `E_NOTE_SCOPE`, `E_NOTE_ISSUED`, `E_NOTE_DEADLINE`,
`E_NOTE_WINDOW`; `E_PASS_VERIFIER`, `E_PASS_NOTE`, `E_PASS_WINDOW`,
`E_PASS_WINDOW_ORDER`, `E_PASS_ORDER`; `E_REPORT_VERIFIER`,
`E_REPORT_NOTE`, `E_REPORT_PROTOCOL`, `E_REPORT_MANIFEST`, `E_REPORT_AT`,
`E_REPORT_TEXT`; `E_REFUSAL_VERIFIER`, `E_REFUSAL_NOTE`, `E_REFUSAL_PASS`,
`E_REFUSAL_AT`, `E_REFUSAL_PLACE`, `E_REFUSAL_ORDER`, `E_REFUSAL_PHOTOS`,
`E_REFUSAL_PHOTO` (carrying an index), `E_REFUSAL_DUP_PHOTO` (carrying an
index), `E_REFUSAL_PHOTO_ORDER` (carrying an index); `E_SERVICE_VERIFIER`,
`E_SERVICE_SERVES`, `E_SERVICE_ADDRESS` (carrying an index),
`E_SERVICE_DUP` (carrying an index), `E_SERVICE_ORDER` (carrying an
index), `E_SERVICE_AT`; `E_SIG_FORM`, `E_SIG_V`, `E_SIG_RANGE`,
`E_SIG_HIGH_S`, `E_SIG_RECOVER`, `E_SIG_SIGNER`.

**Payload tokens** (closed; by the orders of §8.1 and §8.2):
`E_PAYLOAD_PREFIX`, `E_PAYLOAD_CAP`, `E_PAYLOAD_SEGMENTS` (carrying the
dot count), `E_PAYLOAD_B64` (carrying `k`), `E_PAYLOAD_PASS` and
`E_PAYLOAD_NOTE` (carrying the document's token).

**Result vocabularies** (closed): report pairing `PAIRED`, `NOTE_INVALID`
and `REPORT_INVALID` (each carrying the failing document's token),
`REPORT_NOTE_MISMATCH`, `REPORT_VERIFIER_UNRESOLVED`,
`REPORT_VERIFIER_MISMATCH`, `REPORT_PROTOCOL_MISMATCH`; refusal pairing
`REFUSAL_PAIRED`, `NOTE_INVALID`, `PASS_INVALID` and `REFUSAL_INVALID` (each
carrying the failing document's token), `REFUSAL_NOTE_MISMATCH`,
`REFUSAL_PASS_MISMATCH`, `REFUSAL_VERIFIER_UNRESOLVED`,
`REFUSAL_VERIFIER_MISMATCH`; service reading `SERVICE` (carrying the
record's `serves`), `SERVICE_NONE`, `SERVICE_UNREAD` (carrying the
candidate's token: a token of §7a.2, or `galeed.roster/1` §2.1's `E_HASH`,
`E_JSON`, or `E_KIND`), `SERVICE_MISMATCH`, and `serving`'s `true` and
`false` (`galeed/1` §1's `bool`); payload `PAYLOAD_OK` (carrying the pass
bytes and the note bytes); check states `PASS`, `FAIL`, `UNKNOWN`,
`SKIPPED`; check tokens `BAD_NOTE` (carrying a document token), `NO_NOTE`,
`BAD_PASS` (carrying a document token), `NO_PASS`, `PASS_NOTE_MISMATCH`,
`VERIFIER_UNRESOLVED`, `PASS_VERIFIER_MISMATCH`, `NO_CLOCK`, `NOT_YET`,
`PASS_EXPIRED`, `NOTE_EXPIRED`, `NO_LEDGER`, `LEDGER_INVALID`,
`NOT_IN_LINEAGE`, `BROKEN_LEDGER`, `LEDGER_UNREAD`, `NOT_DISPATCHED`,
`DISPATCH_DISPUTED`, `NO_BENCH`, `BENCH_UNRESOLVED`, `NOT_ON_BENCH`;
verdicts `GREEN`, `PARTIAL`, `FAIL`; bench spellings `null`, `unresolved`, a
sorted array without repeats; result echoes `basis`, `bench`, `current`,
`now`, `root`.

## 12. Outside the grammar, and the openings by name

- **Chains, clocks, registries.** This law consults none of them; the audit
  input carries what the chain said, the logs set carries what the registry
  said (§7a.3), `now` carries what the caller trusts of the time, `bench`
  carries the caller's resolution of the covenant's bench under the roster
  law, and `current` carries the caller's reading of the succession book
  under that law. Which root is the judge's is the caller's statement, and
  check 4 tests that the note's signer belongs to it.
- **A verdict reaches as far as its basis.** The parent scans what a basis
  declares and nothing else; check 5 convicts nobody under a basis that is
  not covering (§9.2), answering `UNKNOWN` wherever it would otherwise
  `FAIL`, and a `FAIL` there is a failure on the chains, registries,
  senders, and blocks the basis names and never a statement about a chain it
  does not name, which is why the result carries the basis it was read
  under, and why a judge who anchors where no door will scan is anchored in
  no report drawn under that basis.
- **A door may test only the logs its checks read.** The parent's
  completeness rule binds what a verifier obtains: every log it enumerates
  under the basis, or no anchor set (`galeed/1` §9.4). A caller that
  obtained every such log may hand §10 an input whose anchor set carries a
  record only for each log it tested against `galeed/1` §9.1 and §9.3, the
  other logs left untested and carrying no record, where no check of §10.2
  would take another state or another token whichever of the untested logs
  are anchors and whatever verdicts `galeed/1` §9.3 gives them: the result
  does not then depend on the logs left untested, and stands under the basis
  it echoes. Whether that holds is decided before §10 runs, from the hashes
  the untested logs name, the bytes they carry, and the records in hand;
  where it cannot be so decided, the input does not serve §10, and the door
  is decided on a conforming anchor set or is not decided. An input so drawn
  serves §10 alone: it is not the conforming anchor set of `galeed/1` §9.4,
  a log it leaves untested is not a log its scan failed to read, and the
  label, the lists, and the entry count of a report taken over it are not
  the parent's report of that basis and are presented as none. Every check
  still reads the input the caller handed it (§9.0), and no check's
  condition moves.
- **A dispatch the judge did not publish does not exist.** A dispatch entry
  is a ledger entry only once a counted record the trim of `galeed/1` §8.1
  keeps carries it; the doctrine's answer is to publish dispatches before
  the runner leaves.
- **A retired key's anchors count for nothing.** A key that handed off
  retires at the smallest `blockTimestamp`, on any chain and whatever their
  sender and verdict, of the records carrying a succession or a migration it
  signed (`galeed/1` §7.4, §11), and in an audit whose anchor set holds such
  a record, which the parent has the seat carry on every chain of its basis
  (`galeed/1` §5.7), every record it sends at which it has retired, one in a
  block whose timestamp is later or one of its handing off's own second that
  carries what it signed at a position after its retiring seq, stands in
  `DISCARDED` (`galeed/1` §7.4, §8.1): it makes no `MISSING` row, no
  `CARRIED_MALFORMED`, and no ledger entry, so checks 4 and 5 read the
  ledger as its keys anchored it at records at which they had not retired, a
  dispatch the ceding key anchored after a carrier elsewhere dated its
  handing off staying out of the ledger until the successor re-carries it
  (`galeed/1` §8.6). A basis that widens its `senders` can add such carriers
  and so take entries out of the ledger, since any copy of the handing-off
  bytes proves them in existence at its block time. No check of §10 reads
  `DISCARDED`, so a `GREEN` here says that under the anchor set the reader
  scanned every check passed, and never that the scan reached the whole
  ledger; a reader who wants that silence closed reads the parent's
  `DISCARDED` list beside the result.
- **A ledger read short.** A door cannot tell a stranger's note from a scan
  that missed a key: a withheld succession, or a basis none of whose
  `chains` objects holds together the chain, block, emitter, and sender of
  an anchor that carried it, leaves a lineage that lacks the sitting key, so
  check 4 answers `UNKNOWN`; a basis whose `senders` omit a key the lineage
  holds is not covering (§9.2), so check 5 answers `UNKNOWN` where no
  dispatch is in hand; either way a door that holds no dispatch does not
  open (§10.3); the doctrine's answer is the parent's, to publish every
  entry where the door will read it, and the reader's, to scan every key the
  judge's document declares (`galeed.roster/1` §3a).
- **Withdrawal.** A note lapses at its `deadline` and by nothing else; no
  document of this law withdraws one, no check reads a withdrawal, and what
  a judge's later word is worth against a note already dispatched is the
  covenant's and the forum's.
- **Delivery.** A note reaches its verifier by any channel; the ledger
  carries `noteHash` and never the note, so nobody reads a dispatch's
  content from the chain, though its existence is read there like any
  entry's, and a report that never comes is the dispatching judge's loss.
  The law states this by name and provides no delivery.
- **A report's time and its ledger relation.** `at` is the verifier's word;
  whether it was late against a contract, and whether the report was entered
  in a ledger the verifier keeps, are the contract's and that ledger's law.
  No predicate of this law reads a report against a ledger.
- **Scope, protocol, and the refusal inference.** What `scope_md` allows,
  what a `protocol` demands, what a manifest must hold, and what a refusal
  at a good door implies: the judge's covenant and the forum.
- **Commitment and production.** Whether a venue took a commitment of a
  report's `manifest`, and whether its bytes were produced: the venue
  interface and the evidence family law.
- **Conflicts.** Whether the verifier is a party, a party's successor, or a
  bureau examining its own work: the judge's dispatch tooling, reading the
  venue and the roster law, before the note is signed.
- **Subscription.** Whether a verifier stands in a judge's or a guild's
  service is read from two published lists, the seat's bench
  (`galeed.roster/1` §3.1, §5.4 step 7, §5.5) and the verifier's service
  record (§7a), by the judge's dispatch tooling before a note is signed and
  by the verifier's inbox before a note is taken in, beside the conflicts
  above; no check of §10 asks it, and a note to a verifier outside every
  relation is a lawful note. How a seat invites a verifier, how the verifier
  learns of it, what either side charges or pays, and when a relation ends
  are the two desks' and the forum's: the law provides no invitation and no
  notice, a desk that wants one sends it by any channel as data, and each
  half is its side's own word in the form its own law gives it: a guild's
  bench is a roster published from the guild's address, whatever stands
  behind that address, a key, a multisig, or a company that executes a
  proposal (`galeed.roster/1` §9); a judge's array stands in a covenant his
  key signed; a verifier's record is published from its paper key (§7a.1),
  the key that signs its passes, so a verifier's practice that is a company
  is paid where it chooses and its record is sent by whoever holds that key.
  A judge whose covenant names no bench, or names a guild's pointer, lists
  no verifier of his own, and a verifier that lists such a judge stands in
  his service by its own word alone. A guild's bench is read through its
  pointer at every act and every door, so a listing or a removal reaches
  every case from the next resolution; a judge's own array is frozen with
  the covenant a case was seated under, so it reaches the cases seated after
  it: `galeed.roster/1` §5.5's pointer row and array row, unchanged here.
- **Rendering.** How checks and states are shown, beyond the rules §10.3
  states: a door shows every check's state and token and the decoded note's
  `case` and `scope_md` where the note decoded, `GREEN` alone is green,
  `PARTIAL` is its own state, and a payload yielding a token shows nothing.
  How a payload text becomes a QR code and back, and at what error
  correction level, is the renderer's; §8.1 names a level only to justify
  its cap.
- **Payment and escrow.** Not here.

## 13. Freeze and evolution

**13.1** The reference implementation freezes when independently written
implementations, produced from this text and a conforming `galeed/1` core
alone, agree with it on every input of a generated corpus (every document
token and every accepted document's `doc_id`, every signature the harness
asks for, every pairing value, every payload encoding and every payload
decoding, every door result as §10.4's whole canonical object, every
service reading and every `serving` answer), with every closed boundary in this text witnessed on both
sides by at least one corpus input under every seed the corpus is
generated with, the boundary list being a precondition on the corpus and
never a property claimed of it afterwards; the seed varies the key
material and the digests it moves and never the case list, so the
boundary list is checked once against the corpus's shape and re-checked
under every seed only to catch an answer that turns on a digest. A closed boundary of this text
is a boundary the corpus can witness on the surface §13.2 compares; a
rule with no reachable failing side is witnessed once and states no
boundary in this sense. The fuzzing `galeed/1` §12.1 asks of its core is
deferred by the judge's ruling; §13.5 records beside every digest it
names whether that fuzzing was run and with what result, a naming that
records none is a naming under deferral, and the deferral travels on the
face of the digest for every reader who relies on it.

**13.2** Once §13.5 names a release digest, agreement with the frozen core
on the corpus is the test anyone can run in seconds, and it is evidence of
conformance and never its definition: conformance is agreement on every
input, and an input outside the corpus on which a verifier differs from the
core is a non-conformance the corpus did not catch and a reason to grow it.

**13.3** `galeed.papers/2` is a new `kind` in every document, new domain
literals, a new `entryType` for dispatches, and a new payload prefix; a
document accepted under `galeed.papers/1` is accepted under
`galeed.papers/1` forever, and a later law reads it by this one.

**13.4** This law and `galeed/1` are two laws: a change to one is never a
change to the other, and each names its own core. This law's core embeds
the `galeed/1` core of release digest
`0x5f24cd212705b0cbfd00e5fe0c720ca059b662276c7eaac9a7907e621c50e65d`
(named in that law's §12.5); a papers core built on any other `galeed/1`
core is another core.

**13.5 The core's identity.** Naming a release digest here is the
freeze, and it is the only act that performs it. The release digest of
the `galeed.papers/1` core is

    0xfcc4ee4905921f5dc815f47c6b0cf38505c29ca7dc6b27e4b298e097faf60759

named on 2026-10-30: the pure Python implementation in
`galeed-conformance/papers-py/` (three source files: `gpp.py`, `gppdoc.py`,
`gppdoor.py`; the language's standard library and the `galeed/1` core of
§13.4, nothing else), whose release digest
`galeed-conformance/criterion-digest-papers.sh` computes over the three
files and the `galeed/1` core digest of §13.4; the corpus, the harness
contract, and the review record beside it say how it converged. The fuzzing
`galeed/1` §12.1 asks of its core was not run at this naming: the naming is
under §13.1's deferral, and what stands beside the digest is a two-round
convergence of three independent implementations (Rust, Node.js, Python),
each written from this text and its harness contract alone, run unchanged in
the closing round at zero divergences over the corpus on six seeds and more
than 100,000 black-box probes with zero disagreement, every branch arm of
the criterion but its main guard witnessed on both sides, and, at the
re-pinning of 2026-10-30 after the `galeed/1` core moved, the same three
implementations brought up to that core's revised lineage at zero
divergences on six seeds and more than 34,000 door probes, recorded in the
review record. From the naming the core binds and this text is its textbook;
a dispute about what `galeed.papers/1` says on some input is settled by
running the core on it. A change to any of the three files is a new core and
a new digest named by the same act.