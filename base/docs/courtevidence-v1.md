# GALEED evidence manifests v1 (`courtevidence/1`)

The evidence law: the short law that says what an evidence manifest is, how
its digest is taken, to whom evidence is encrypted, and which of a judge's
published encryption recipients a producer picks. Where [`galeed/1`](galeed-v1.md)
§6.3 cites a manifest by its digest in a judgment's `evidence` element,
[`galeed.papers/1`](galeed-papers-v1.md) §6.1 cites one in a verifier's
report, and [`galeed.venue/1`](galeed-venue-v1.md) §2.8 records a commitment
to one, this law says what those three digests are digests of. It is
self-sufficient in the same sense as its parents: every predicate a producer
or a reader needs is stated here in full, and it consumes `galeed/1` only
through that law's core (§8.4).

**Status.** The law is the set of total predicates in this text. A
conforming implementation embeds the `galeed/1` core of the release digest
named in §8.4, and agrees with the core named in §8.5 of this text on every
input; until that core is named this text is the criterion. Evolution goes
through `courtevidence/2`.

**Rule classes.** Three classes appear, and every rule below is marked:

- **[D]** document predicates: decided offline from one byte string, or
  from one string, and nothing else.
- **[R]** reading predicates: decided offline from a `galeed/1` audit
  input (the two inputs of `galeed/1` §8), and nothing else. No predicate of this law consults a chain, a
  clock, or a registry.
- **[O]** producer obligations: rules a producer keeps, decided by no
  predicate of this law, carried in no answer, and measured by no corpus; a
  tool that breaks one produces no token, and what follows is the
  covenant's and the forum's.

---

## 1. Notation

`galeed/1` §1 applies: byte strings, `hex20`, `hex32`, `token`, `skeleton
string`, `prose string`, `int`, bytewise order, `sha256`, UTF-8. The
preamble of `galeed/1` §6 applies to every member table of this law:
`required` means the key is present, `optional` means the key may be
absent, a key present with any value, `null` included, is a member present,
and where a rule says a member equals a literal or a digest it means the
value is a string byte-equal to that literal or to the value's spelling
under §1's hex functions. In addition:

- **`canonical(b)`**: `galeed/1` §3.5 alone, all six of its tests, yielding
  the value or one of the eight tokens that section can name (`E_UTF8`,
  `E_JSON`, `E_NUMBER`, `E_DEPTH`, `E_DUP_KEY`, `E_KEY_CHARSET`,
  `E_VALUE_CHARSET`, `E_NOT_CANONICAL`). `galeed/1` §3.3's charsets apply
  inside it exactly as the parent states them, subtrees included: a string
  value below a key ending in `_md` is prose, and every other string value
  of the manifest is a skeleton string before any table below reads it.
  `note_md` is the one key of §2's own tables that ends in `_md`, and the
  `hash` kind's `note_md` (`galeed/1` §6.3) the one such key of the table §2
  imports; a key ending in `_md` may also be an extra member of a closed
  table or of a `subjectRef`, or sit below any member whose value is an
  object or an array, and opens a prose subtree there that `canonical(b)`
  admits, which the row reading that member then decides under its own
  token, a `subjectRef`'s extra member admitted, a closed table's refused
  (`E_MANIFEST_CLOSED`, `E_FILE_CLOSED`, `E_FILE_ZIKARON`) where §2.3's
  order reaches that test, and never `E_VALUE_CHARSET`; and no key on the
  path from the root to `kind`, `name`, or `mediaType` ends in `_md`, so
  `galeed/1` §6's charset shortcut applies to those three unchanged: a
  `token` row tests non-emptiness and the absence of U+0020, a `skeleton
  string` row tests non-emptiness, the rest having been decided by
  `canonical(b)`.
- **`manifest(b)`** = `sha256(b)` for a byte string `b` that §2 accepts as a
  manifest. Identity is not total here, unlike `galeed/1` §2.1: a byte
  string §2 refuses has no manifest digest under this law, so that every
  digest this law defines is the digest of a manifest. What a citation of
  some other hex32 is worth is §7's.
- **`subjectRef`**: the object of `galeed/1` §6.3, decided by that row's
  form in full: an object whose `kind` is a token, a known kind (`evm`,
  `hash`) forcing its table, members beyond the table permitted and
  untested.

---

## 2. The manifest

A manifest lists the files of one body of evidence, for one subject, from
one party.

**[D] 2.1 Members, closed.** A manifest is a canonical object with exactly
these members and no others:

| key | presence | form |
|---|---|---|
| `spec` | required | string byte-equal to `courtevidence/1`; a `spec` that is not a string fails the same test |
| `subjectRef` | required | `subjectRef` (§1): the case or object the evidence concerns, as the judgment that cites it will name its subject |
| `party` | required | hex20: the address on whose behalf the manifest is submitted; whose it is, and whether it matches the venue's `Committed` record, is the reader's (§7). A document citing this manifest may carry an address of its own beside the digest, the venue's `Committed` sender among them, and the two answer different questions |
| `files` | required | array of file items (§2.2); the empty array is permitted |
| `note_md` | optional | prose string: the producer's own words; the absent key is the only spelling of an absent `note_md`, and a present `null` fails |

**[D] 2.2 File items.** Every element of `files` is an object with exactly
these four members, and at most the fifth, and no others:

| key | form |
|---|---|
| `name` | skeleton string, non-empty: the file's name as the producer lists it |
| `sha256` | hex32: `sha256` of the file's bytes as the producer lists them, which are the file's plaintext bytes and never a ciphertext's (an obligation, §3.3, tested by nothing here) |
| `bytes` | int: the length of those same bytes |
| `mediaType` | skeleton string, non-empty: the file's media type as the producer lists it |
| `zikaron` | optional: object with exactly the members `root` (hex20), `chainId` (int), `tx` (hex32), and `entry` (a string of `0x` followed by an even number of lowercase hexadecimal digits, the empty byte string written `0x`): where this file's bytes are recorded in a ZIKARON author's ledger, the root that ledger is read under, the chain and the transaction that anchored the recording entry, and that entry's bytes; this law tests the four forms and nothing of their meaning, which is the ZIKARON law's, and a reader that wants the recording's time takes the four to that law's tools |

No predicate relates `bytes` to `sha256`: a length of 0 beside a digest that
is not the digest of the empty byte string is a file item all the same, and
whether the producer lied is §2.4's finding, which is one finding over the
whole manifest: an item no byte string can satisfy makes `produced` false
for every item beside it, and a judge that wants to say which item failed
says it in `verdict_md` or in the extension members its covenant
names (§6). A producer that finds such an item in a manifest already
committed writes a new manifest and commits it while the venue's window is
open (`galeed.venue/1` §2.8), an obligation of §3.3's class (`[O]`); this
law provides no correction.

**[D] 2.3 Order of decision.** `canonical(b)` first, its token naming the
fault, `E_NOT_CANONICAL` included, so that a numeric spelling outside
`galeed/1` §3.2 has already failed `E_NUMBER` and the `int` rows below are
failed only by values of non-numeric shape; root an object (`E_MANIFEST`);
the four required members present (`E_MANIFEST_MISSING`), then no member
beyond the five (`E_MANIFEST_CLOSED`); `spec` (`E_MANIFEST_SPEC`);
`subjectRef` an object whose `kind` is a token, a known kind's table met
(`E_MANIFEST_SUBJECT`); `party` (`E_MANIFEST_PARTY`); `files` an array
(`E_MANIFEST_FILES`); then each element of `files` in order, the first
faulting element naming the fault with its index counted from 0: the element
an object (`E_FILE`), its four members present (`E_FILE_MISSING`), then no
member beyond the five (`E_FILE_CLOSED`), `name` (`E_FILE_NAME`), `sha256`
(`E_FILE_SHA256`), `bytes` (`E_FILE_BYTES`), `mediaType`
(`E_FILE_MEDIATYPE`), `zikaron`, where present (`E_FILE_ZIKARON`, whether
not an object, a member of the four absent, a member beyond the four
present, or a member outside its form); then `note_md`, where present
(`E_MANIFEST_NOTE`). A
byte string that passes is a manifest; nothing else is. Two file items may
repeat a `name` or a `sha256`; the law reads each item alone.

**2.4 Meaning.** A manifest commits its producer to a list of files by
digest and length, so that whoever is later shown the files can decide,
file by file, whether the bytes shown are the bytes listed: `produced` in
`galeed/1` §6.3 is the judge's finding that, for every item of the
manifest, bytes of the listed length hashing to the listed digest reached
the judge, and this law defines that finding and tests nothing about it.
The bytes a file item lists are plaintext: a ciphertext §3 makes is a
transport of them, its own length and digest are no member of any manifest,
and a judge decides `produced` over what it opened. One manifest therefore
stands for every recipient the files were encrypted to, and a new
`keybinding` changes no digest of it. A manifest with no file items commits
its producer to nothing, and `produced` over it is true with nothing behind
it: a reader that treats `produced` as evidence that bytes arrived reads the
manifest as well as the judgment. `name` and `mediaType` are the producer's
words: a tool spells a name in the skeleton charset or refuses to list the
file, and this law takes no position on how a name outside it is
transliterated. The manifest is cited by `manifest(b)` in a judgment
(`galeed/1` §6.3), in a report (`galeed.papers/1` §6.1), and in a venue's
commitment (`galeed.venue/1` §2.8); the citing document carries the digest
and never the manifest, and a manifest is resolved by being shown its bytes
and recomputing `sha256`. A `zikaron` member is the producer's pointer to a
recording of the same bytes under another law; it changes no digest of this
law, `produced` reads nothing of it, and a producer that hands over a
ZIKARON kit lists the kit's files like any other files, with or without the
member.

---

## 3. Encryption

**[O]** Where a producer encrypts a file a manifest lists, or the manifest itself,
for delivery to a judge, it encrypts to the recipient that judge published
in its ledger (§4), so that the judge's key opens it and the producer never
learns a key, §3.1 saying what else a ciphertext may name; what a party
does with its own copies is outside this law. Whether a given file must be encrypted at all, and what a
party may keep in the clear, is the covenant's and the forum's (§6);
this law fixes the scheme and the recipient of every encryption a producer
performs and requires none. A case may seat more than one judge
(`galeed.venue/1` keys a seating by `(caseId, seat)`); §4 is run once per
seated judge, over that judge's own ledger, and the recipients so picked are
the recipients a producer's ciphertexts name, whether one ciphertext
carries every seat's recipient or each seat has its own; a producer that
encrypts nothing breaches nothing here (§6). The seats are independent:
a seat whose pick is `one` receives its ciphertext, and a seat whose pick is
`none` or `candidates` receives none until the judge publishes a usable
binding or a person chooses from the list (§4.2); a producer that cannot
encrypt to one seat has not thereby failed to produce to the others, and
whether evidence a seat cannot open is evidence before that seat is the
covenant's and the forum's (§6). The manifest lists the files once
(§2.2) and is indifferent to how many ciphertexts carry them. Which seats a
case has and who sits in each is a fact the venue's record carries (§6).

**[O] 3.1 Scheme.** Encryption is the age file format, version 1; every
recipient stanza of a ciphertext is an X25519 stanza naming either a
recipient §4 picked for a seat of the case it is made for or an identity
the producer itself controls, and no recipient is named twice; a
ciphertext may carry one seat's recipient, several, or every seat's, and a
seat whose pick is withheld is named by none; the stanza count then
discloses how many keys open the ciphertext and never which or whose, an
X25519 stanza naming nobody; the ciphertext is the binary format and never the ASCII armor. What age is,
how a recipient is derived from an identity, and how a file is opened are
the age specification's, and a conforming tool interoperates with the
reference age tools on every ciphertext it makes.

**[D] 3.2 Recipient form.** A recipient is the spelling the age
specification gives an X25519 recipient, narrowed to what age itself emits:
a string of sixty-two characters, every one at or below U+007E, so that its
character count and its byte count agree, of the shape `age`, then the separator `1`,
then fifty-eight characters of the Bech32 alphabet
(`qpzry9x8gf2tvdw0s3jn54khce6mua7l`), the last six of which are the Bech32
checksum; the checksum verifies under BIP-173, whose constant is 1 (the
Bech32m constant of BIP-350 names another form, which this law refuses);
the first fifty-two of those characters convert from five-bit groups to
exactly thirty-two bytes, two hundred and sixty bits of which the last four
are padding and are zero, the five-bit groups packed most significant bit
first and the checksum computed over BIP-173's expansion of the
human-readable part (`age`: each character's high three bits, then a zero,
then each character's low five bits), both as BIP-173 fixes them; and every
letter is lowercase. The predicate is
over the string's scalar values, and a string carrying any scalar value
above U+007E fails for that reason alone; one token, `E_RECIPIENT`, covers
every way a string leaves this form, so no order of these tests is part of
the law. This form is narrower than some age tools accept: BIP-173 admits
an all-uppercase string, and at least one widely used Bech32 library ignores
the padding bits, so a key has spellings those tools open and this law
refuses; the narrow form is the one age emits, and it is the only spelling a
seat publishes, the recipient being a token readers compare byte for byte.
This is the spelling `galeed/1` §6.7 leaves to the evidence family law: it
is a token, and it is published in a `keybinding` entry's `recipient` as it
stands, with no further encoding.

**[O] 3.3 Producer obligations.** A producer encrypts to the recipient §4
picks and to no other; a producer never re-encrypts a ciphertext made to an
earlier recipient, since the earlier key still opens it and the earlier
binding still stands (`galeed/1` §6.7); evidence produced after a judge
publishes a new `keybinding` is encrypted to the new recipient, and evidence
produced before stays as it was. A case that seats another judge (a later
`galeed.venue/1` `Seated`) seats a new judge with a ledger and a root of its
own (§4.1): the pick is run for it as for any first delivery, and the
ciphertexts the earlier judge holds stay as they are. A judge whose key
moves by a succession is the same judge, read under the same root, and its
bindings stand until a later one. Evidence committed before any judge is
seated has no recipient yet: the pick is run, and the encryption performed,
at the moment of delivery to a seated judge, and the rule of stability binds
a producer only from the first ciphertext it made. A tool that meets a
recipient outside §3.2 at the moment of use fails loudly and encrypts
nothing. A recipient §3.2 accepts is not thereby a recipient age can use: an
X25519 point of small order yields an all-zero shared secret, which is the
condition a tool tests, the byte string's own value being an unreliable
guide since spellings of a `u` coordinate at or above the field prime reduce
to the same points, and no conforming implementation encrypts to it; a tool
that meets one fails the same way, loudly, encrypting nothing, and never by
aborting; `E_RECIPIENT` names the form alone and is not emitted here. Either
failure is the judge's failure to publish a usable binding, and never a
reason to send evidence in the clear or to a key the judge did not name.

---

## 4. The recipient

A judge names the key evidence is encrypted to by a `keybinding` entry
(`galeed/1` §6.7), and names another by publishing another; no predicate of
that law relates two bindings, so the choice of the current one is this
law's.

**[R] 4.1 Bindings.** Let `I` be the `galeed/1` audit input whose root is
the judge's root address, the address that judge's ledger opened under
(`galeed/1` §7.3, the `judge` of the ledger's `genesis`; the `ledger.root`
of the judge document, `galeed.roster/1` §3a), which no succession changes:
a producer that names the current key as root, where that key opened no
ledger of its own (`galeed/1` §7.5), reads a ledger whose `genesis` that
root's lineage excludes, and is answered `gap` where that ledger holds a
binding and rule 2 has not answered, and `none` where it holds none; and
whose anchor set and basis are those of a scan the producer made (`galeed/1`
§9.4) under a basis of its own declaration, the judge's declared
`ledger.basis` being where a producer finds one that reaches every anchor
the seat promised. Let `report` be the audit of `I` under `galeed/1` §8
where `I` is a `galeed/1` audit input; where it is not (bytes the reader of
`galeed.roster/1` §1 refuses, the reader `galeed/1`'s measuring contract
describes for its `audit` command; a root value that is not an object; a
member absent or outside its form; or a refusal under `galeed/1` §9.4), the
pick is `unread`. The ledger entries of `I` are the carried byte strings
`galeed/1` §8.1 admits, those whose `entry_id` appears in `report.anchored`,
each a byte string of a record of `I`. The **bindings** are the ledger
entries of `I` whose `entryType` is `keybinding`, each carried as `(seq,
entry_id, body.recipient, body.effective, recipientForm)`, the members read
from the entry's own bytes, which the core has already accepted as an entry:
`galeed/1` §4.3 step 12 has applied that law's §6.7 table, so `recipient` is
present and a token and `effective` is present and an int, and no binding is
ever carried with either member missing; `recipientForm` being `true` iff
`body.recipient` satisfies §3.2; ordered by `seq` descending and, at equal
`seq`, by `entry_id` bytewise ascending.

**[R] 4.2 The pick.** Let `top` be the greatest `seq` among the bindings,
never among the other entries. The reasons are tested in the order written
and the first that holds is the one carried. The pick is:

1. `none`, where there are no bindings;
2. else `candidates` with reason `fork`, where `report.findings` carries an
   `EQUIVOCATION` whose `position` is `top`: some position the judge's
   latest binding shares is contested, whatever entries contest it and
   whatever ground `galeed/1` §8.4 recorded, so the ledger is not to be read
   at the top on a tool's own authority. The list may hold one binding:
   `candidates` says that no tool chose, and never that there were two keys
   to choose from. An `EQUIVOCATION` grounded on a shared `prev` carries the
   smaller of its two `seq` values as its `position` (`galeed/1` §8.7 item
   3), so such a pair reaches this rule exactly when its lower entry sits at
   `top`: a pair whose upper entry is the top binding itself, a pair that
   straddles `top`, and a pair wholly on one side of it all pass rule 2, to
   be answered by rule 3 where the scan has a hole and by rule 4 otherwise,
   and a reason of `broken` therefore says nothing about which entries were
   contested;
3. else `candidates` with reason `gap`, where the scan has a **hole**:
   `report.findings` carries a `SEQ_GAP`, or some `hash`, of a
   `report.missing` row or of an `unproven` row of the report, is the
   `entry_id` of no ledger entry, of no `MALFORMED` row, and of no
   `EXCLUDED` row. The ledger as scanned then has a position below its
   greatest that no ledger entry filled (an entry never anchored, anchored
   only by records that did not count or that the trim of `galeed/1` §8.1
   discarded, or anchored outside the basis), or a commitment whose bytes
   the scan did not read at all or read only in records that did not count
   (a counted record that carried nothing, a record the chain did not prove)
   or that the trim of `galeed/1` §8.1 discarded; and no tool reads a
   judge's current key out of a ledger with a hole in it. Bytes the scan
   read and found to be no entry of this ledger (`MALFORMED`, `EXCLUDED`)
   leave no hole, since no record can make them an entry of this ledger, and
   where a counted record that `galeed/1` §8.1 did not discard carried them
   they are rule 4's hard finding (`galeed/1` §8.4), so a report labelled
   `GAPS` by `MISSING` rows alone, each of them a `MALFORMED` or an
   `EXCLUDED` row, has no hole, and rule 3 does not answer it. The hole's
   position is not tested: a hole below `top` withholds the pick like any
   other, since what is wrong is the scan and not the position; a later
   binding whose bytes only `VOID` records or records the trim of `galeed/1`
   §8.1 discarded carry, and whose hash no counted or `UNPROVEN` record
   undiscarded by that trim committed to, is no act this ledger records,
   adds no hole of its own, and moves no pick, its empty position being limb
   1's hole like any other where a ledger entry sits above it;
4. else `candidates` with reason `broken`, where any finding of
   `report.findings` is hard: the ledger convicts itself, and a tool picks
   nothing from it on its own;
5. else `one`: the first binding of §4.1's order, which is the single
   binding at `top`, since two ledger entries at one `seq` equivocate under
   `galeed/1` §8.4 and rule 2 has already answered them, while
   byte-identical copies collapse under §8.1 and never equivocate.

A binding whose `recipientForm` is false occupies its position like any
other: the pick is over positions alone, it never skips such a binding and
never falls through to an earlier one, since a recipient the judge has
superseded is not the judge's current key; what a tool does with it is
§3.3's, and the answer carries the flag so that the tool can say why it
refused. `candidates` carries the whole ordered list of §4.1 for a person to
choose from; `one` carries the binding it picked; `effective` is carried on
every binding an answer carries and read by nothing, since `galeed/1` §7.6
reads no clock and the current binding is the one at the greatest position,
whatever dates the judge wrote. A ledger that is both gapped and broken
answers `gap` unless rule 2 has answered `fork`: the gap names what is still
to be fetched or re-anchored, and a producer that closes it and reads again
may then be answered `broken`. A scan with a hole in it, or one whose first
held position is not 0, gives no `one`; a scan whose basis ends before the
chain's head records no finding and no hole, so a `one` says that the ledger
read under the basis holds no later binding and that the scan read bytes
under every commitment its report lists as `missing` or `unproven`, and
never that the judge published none. Freshness is the producer's, and the
remedy is to scan to the head immediately before encrypting.

---

## 5. Tokens and vocabularies

Every token an answer of this law carries is one of these, and no reader
emits another:

- from `canonical(b)`: the eight tokens `galeed/1` §3.5 can name, which
  §10 of that law tabulates;
- the manifest (§2.3): `E_MANIFEST`, `E_MANIFEST_MISSING`,
  `E_MANIFEST_CLOSED`, `E_MANIFEST_SPEC`, `E_MANIFEST_SUBJECT`,
  `E_MANIFEST_PARTY`, `E_MANIFEST_FILES`, `E_MANIFEST_NOTE`; and, with an
  index, `E_FILE`, `E_FILE_MISSING`, `E_FILE_CLOSED`, `E_FILE_NAME`,
  `E_FILE_SHA256`, `E_FILE_BYTES`, `E_FILE_MEDIATYPE`, `E_FILE_ZIKARON`;
- the recipient (§3.2): `E_RECIPIENT`;
- the pick (§4.1, §4.2): `one`, `candidates`, `none`, `unread`; reasons
  `fork`, `gap`, `broken`;
- and, named here though no answer carries it, the one literal this law
  fixes inside a document: the spec literal `courtevidence/1`, a literal of
  the GALEED family.

---

## 6. Outside the grammar

The following are decided by judges, venues, verifiers, and readers, never
by this law:

- **What a manifest should hold.** Which files a protocol demands, what a
  judge's covenant requires of a party's evidence, and whether a manifest is
  complete: the protocol, the covenant, and the forum.
- **Whether to encrypt.** Which files a party must encrypt, and which a
  protocol requires in the clear: the covenant and the forum. This law fixes
  the scheme and the recipient of every encryption a producer performs, and
  requires none (§3).
- **A ledger that moved to another law.** A ledger closed by a `migration`
  (`galeed/1` §6.6a) is read like any other: where the pick is `one`, it
  names the last binding the ledger holds as read under the basis (§4.2),
  which the seat's ordering of `galeed/1` §5.7 makes its last, and no key
  re-anchors a binding that an earlier record of the migration discarded;
  whether the seat has since named another key under the law it moved to is
  that law's family laws' (`galeed/1` §12.3).
- **Freshness.** Whether the basis reached the chain's head. No predicate
  here sees an anchor past the basis's `toBlock`, and the pick is relative
  to the scan (§4.1).
- **Commitment.** Whether a venue took a commitment of the digest, from
  whom, and at which block: `galeed.venue/1` §2.8's `Committed` event; a
  judgment's `evidence` element carries beside `manifest` and `produced` the
  extension members the judge's covenant names, and `galeed/1` §6.9 reads
  none of them.
- **Production.** Whether the bytes reached the judge, and by which channel:
  the judge's own finding (§2.4) and the parties' delivery. The law provides
  no delivery and no storage.
- **Whose manifest.** Whether `party` names the party that committed the
  manifest, a verifier's requester, or a bureau: the citing document and the
  venue's record.
- **Retention and destruction.** How long a judge keeps the files, and what
  is destroyed when: the covenant; the manifest and its digest outlive the
  files by design.
- **Keys.** How a judge generates and stores its age identity, and how a
  producer scans the judge's ledger: the desk and the judge's declared basis
  (`galeed.roster/1` §3a); this law reads the audit input it is handed.
- **Which root, which judges.** Where a producer learns a judge's root
  address and basis: the judge document (`galeed.roster/1` §3a). Which seats
  a case has, who sits in each, and when a seating changes: the venue's
  record (`galeed.venue/1`'s `seatOf` and its `Seated` event) and the desk
  that reads it. This law is run once per judge a producer is told of, with
  the root it is handed, and reads no chain to find either.
- **Second rails.** A recipient placed in a registry document or a venue
  record is a second rail (`galeed/1` §11); this law reads bindings from the
  ledger alone.

---

## 7. Relation to the parents

A judgment's `evidence` element (`galeed/1` §6.3) carries `manifest`, a
hex32 that `galeed/1` tests for form alone, and `produced`, a `bool`; this
law says the hex32 is `manifest(b)` for a manifest `b` and says what
`produced` means (§2.4). A report (`galeed.papers/1` §6.1) carries
`manifest` likewise. A venue's `commit(caseId, manifest)` (`galeed.venue/1`
§2.8) records the same digest. None of the three reads the manifest, and
this law adds no fault to any of them: a judgment citing a digest that no
manifest bears is a judgment all the same, and what it is worth is the
reader's.

---

## 8. Freeze and evolution

**8.1** The reference implementation freezes when independently written
implementations, produced from this text and its harness contract alone,
agree with it on every input of a generated corpus (every manifest token
with its index and every accepted manifest's digest, the file-item view,
`bytes` and `sha256`, of a byte string, the empty one included, every
recipient form, and every pick with its bindings), with every closed
boundary of this text witnessed on both sides by at least one corpus input
under every seed the corpus is generated with, the boundary list being a
precondition on the corpus and never a property claimed of it afterwards;
the corpus carries the list beside itself. No `[O]` rule is in the corpus,
the freeze certifies none of them, and the harness measures the recipient
form alone of §3: interoperation with the age tools is the producer's own
test. The fuzzing `galeed/1` §12.1 asks of its core is not asked of this
law's core: its predicates over bytes are `galeed/1` §3.5, which that
law's freeze covers, followed by finite member tests, and its reading
predicate is `galeed/1` §8, which that freeze covers, followed by a finite
choice.

**8.2** Once §8.5 names a release digest, agreement with the frozen core
on the corpus is the test anyone can run in seconds, and it is evidence of
conformance and never its definition.

**8.3** `courtevidence/2` is a new `spec` literal; a manifest accepted
under `courtevidence/1` is accepted under it forever, and a later law
reads it by this one. The age format's own versioning is the age
specification's; a ciphertext this law's tools make is a version 1 age
file until a later law says otherwise.

**8.4** This law and `galeed/1` are two laws: a change to one is never a
change to the other, and each names its own core. This law's core embeds the
`galeed/1` core of release digest
`0x5f24cd212705b0cbfd00e5fe0c720ca059b662276c7eaac9a7907e621c50e65d`
(named in that law's §12.5) for `canonical(b)`, the `subjectRef` form, and
the audit of §4.1; an evidence core built on any other `galeed/1` core is
another core.

**8.5 The core's identity.** Naming a release digest here is the freeze,
and it is the only act that performs it. The release digest of the
`courtevidence/1` core is

    0xe66eb68d47d5dd03767eb7728e5610f1a9a9a3a9cc0a6dfeb5ab559e1f1b5d15

named on 2026-10-30: the pure Python implementation in
`galeed-conformance/evidence-py/` (two source files: `gev.py`, `gevlaw.py`;
the language's standard library and the `galeed/1` core of §8.4, nothing
else), whose release digest
`galeed-conformance/criterion-digest-evidence.sh` computes over the two
files and the `galeed/1` core digest of §8.4; the corpus, the harness
contract, and the review record beside it say how it converged. What stands
beside the digest is a two-round convergence of three independent
implementations (Rust, Node.js, Python), each written from this text and its
harness contract alone, run unchanged in the closing round at zero
divergences over the corpus on six seeds and more than 110,000 black-box
probes with zero disagreement, the hole rule of §4.2 witnessed on every side
the seats' laboratories named, and, at the re-pinning of 2026-10-30 after
the `galeed/1` core moved, the same three implementations run unchanged at
zero divergences on six seeds and more than 22,000 picks, recorded in the
review record; the age tools were not run at this naming, §8.1 saying the
freeze certifies none of §3. From the naming the core binds and this text is
its textbook; a dispute about what `courtevidence/1` says on some input is
settled by running the core on it. A change to either file is a new core and
a new digest named by the same act.