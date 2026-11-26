# GALEED registry law v1 (`galeed.roster/1`, `galeed.guildcard/1`, `galeed.judge/1`, `galeed.guildamanah/1`, and the succession book)

The registry law: the third law a GALEED desk needs, so that a judge's desk,
a guild's tool, a door, and any reader that scans one registry agree byte
for byte on who a guild lists, where an address's record continues, which
covenant a judge or a guild offers, and which verifiers sit on a guild's
bench. Where [`galeed/1`](galeed-v1.md) says what a ledger entry is and
[`galeed.papers/1`](galeed-papers-v1.md) says what a door decides, this law
says what a roster, a guild card, a judge document, and a guild covenant
are, how a registry publication decodes to one, what a succession book
records, and what a reader resolves from a scan of the registry and the
book: an address's final key and trail, a guild's judges, verifiers and
bench, a judge's candidate set, a judge's document and current address, a
guild's covenant and card, and a seat's quote. It is self-sufficient in the
same sense as its parents: every predicate a reader needs is stated here in
full, and it consumes `galeed/1` only for the forms of §1 and that law's
core's arithmetic (§10.4). `galeed/1` §11 puts guild rosters and their
resolution outside that grammar and in a family law's hands; this is that
law.

A judge is chosen and never ranked. The law builds no directory, orders
nothing, and ships no guild: it reads what a guild published from the
guild's own address and resolves it the same way for everyone.

**Status.** The law is the set of total predicates in this text. A
conforming implementation agrees with the core named in §10.5 on every
input; until that core is named this text is the criterion. Evolution goes
through `galeed.roster/2`, `galeed.guildcard/2`, `galeed.judge/2`, and
`galeed.guildamanah/2`.

**Rule classes.** Three classes appear, and every rule below is marked:

- **[D]** document predicates: decided offline from one publication record
  (§1) and nothing else.
- **[R]** resolution predicates: decided offline from a logs set (§1)
  together with a chain id and the query, and nothing else. No predicate of
  this law consults a chain, a clock, or a registry, and none reads a time;
  what the registry said is inside the logs set.
- **[O]** obligations: rules a producer or a consuming tool keeps, decided
  by no predicate of this law, carried in no answer, and measured by no
  corpus. §7 is the one section of this class.

---

## 1. Notation

`galeed/1` §1 applies: byte strings, `hex20`, `hex32`, `int`, `bool`,
`token`, bytewise order, `keccak256`, `sha256`, UTF-8, Unix seconds. Where a
rule says a member equals an address or a digest it means the value is a
string byte-equal to that value's spelling under `galeed/1` §1's hex
functions; an uppercase digit, an uppercase or absent `0x` prefix, or any
other length is outside the form, and no predicate of this law normalizes
case. In addition:

- **registry**: a contract whose `Published(address indexed publisher,
  bytes32 indexed word, bytes document)` and `StatusChanged(address indexed
  who, bool active)` logs carry the address that called the registry's
  function (`msg.sender`) as `publisher` and `who` and nothing else there,
  and whose `word` is `keccak256` of `document`. `GaleedRegistry`
  (`galeed-core/contracts/src/GaleedRegistry.sol`, the zero-storage
  megaphone the desk pins by its code hash, which also emits `galeed/1`
  §9.1's anchor form from the same address) is one; this law reads logs by
  the two shapes, tests no address and pins no code. Which addresses' logs a
  reader accepts is the reader's declaration (§9): a contract that lets a
  transaction put an address other than its caller into the `publisher` or
  `who` topic is no registry of this law, and a reader that scans one hands
  that contract's callers the power to publish a record from any address.
- **succession book**: a contract whose `Declared(address indexed from,
  address indexed to)` and `Acceded(address indexed holder, address indexed
  to, address indexed root)` logs carry the address that called it as `from`
  and as `to` respectively, and whose code admits a call, from any sender a
  transaction can carry, exactly where §4.2 binds or counts the record it
  emits. `GaleedSuccessionBook`
  (`galeed-core/contracts/src/GaleedSuccessionBook.sol`, which the desk pins
  by its code hash) is one; this law reads logs by the two shapes and tests
  no address. Which book a reader reads is the reader's declaration (§9), as
  its registries are.
- **publication**: a record `(emitter, blockNumber, logIndex, document,
  word)` taken from one `Published` log: `emitter` is the log's `publisher`
  as `hex20`; `blockNumber` is the number of the block that carries the log
  and `logIndex` the log's index within that block, both `int`, as the chain
  numbers them; `document` is the byte string of the log's `document`,
  octets and never text; `word` is the log's `word` as `hex32`.
- **status**: a record `(emitter, blockNumber, logIndex, active)` taken from
  one `StatusChanged` log, `emitter` the log's `who` as `hex20` and `active`
  a `bool`.
- **logs set**: a finite set of publications and statuses drawn from the
  logs of the addresses the caller declares as registries on one chain, and
  of declarations and accessions (§4.1) drawn from the logs of the
  succession book it declares on that chain, in which no two records share
  `(blockNumber, logIndex)`, a chain numbering each log once. The set
  carries no order beyond that pair. `Anchored` logs are not records of this
  law: a publisher publishes whole documents through `publish`, and this law
  reads nothing else the registry emits.
- **reserved prefix**: one of the byte strings `galeed.roster/`,
  `galeed.guildcard/`, `galeed.judge/`, `galeed.guildamanah/`, and
  `galeed.verifier/` (the last reserved by `galeed.papers/1` §7a and read
  there); each names a kind of this family, and a `kind` string that begins
  with one is a document of that kind, of this version where it equals the
  kind literal (`galeed.verifier/1` being `galeed.papers/1`'s literal) and
  of another version otherwise, a later version among them. The short names
  `roster`, `guildcard`, `judge`, `guildamanah`, and `verifier` name the
  prefixes `galeed.roster/`, `galeed.guildcard/`, `galeed.judge/`,
  `galeed.guildamanah/`, and `galeed.verifier/` in turn.
- **of a kind**, and **`candidateOf`**: a publication is **of kind `K`**
  when it fails step 1, 2, or 3 of §2.1 (a publication a reader cannot place
  is of every kind, so that bytes no reader can read never let an older
  document of the kind stand in for them), or when its document reads to an
  object whose `kind` is a string beginning with `K`'s reserved prefix; it
  is **of another kind** otherwise, a string beginning with another reserved
  prefix or with none among them. `candidateOf(a, K)` is the last
  publication of `a`, ascending by `(blockNumber, logIndex)`, `blockNumber`
  compared first, that is of kind `K`, or none where there is none. Every
  reading that takes one current document of a kind (§5.4, §5.7, §5.9,
  §5.11, and `galeed.papers/1` §7a) selects by this function, so that one
  address may carry several kinds and each reader reads its own.
- **`chain`**: the `int` chain id the logs set was scanned on, supplied by
  the caller beside it.
- **the reader**: an RFC 8259 reader with five restrictions and no others:
  the bytes are valid UTF-8; every number, at any depth, is written as
  `galeed/1` §3.2 writes an integer and lies in [0, 2^53 − 1], so a sign, a
  fraction, an exponent, a leading zero, a value at or above 2^53, or a
  literal such as `NaN` each make the bytes unreadable; no object repeats a
  member name after escape decoding; every string, a member name included,
  decodes to scalar values, a `\u` escape of four hexadecimal digits of
  either case naming a lone surrogate being refused while a surrogate pair
  is read as the one scalar value RFC 8259 says it spells; and a container
  opening at nesting depth 129, the root counted as depth 1, makes the bytes
  unreadable. A byte order mark is not whitespace under RFC 8259's
  `JSON-text` production, so bytes beginning `EF BB BF` are unreadable. The
  reader places no charset on keys or values beyond scalar values, and it
  does not require canonical form: `galeed/1` §3.3's two charsets and §3.5's
  roundtrip test are not predicates of this law, a document is identified by
  the `keccak256` of its bytes as published, and a reader never re-encodes
  one. Every form this law names is decided on the string the reader decoded
  and never on the escapes the document spelled it with. The reader is
  defined here in full; the `galeed/1` core of §10.4 carries an
  implementation of it, which that law's measuring contract describes for
  its own `audit` command.
- **bytewise ascending**, of strings: ascending by the bytewise order
  (`galeed/1` §1) of their UTF-8 encodings. This law widens `galeed/1`
  §3.3's key charset and restates `galeed/1` §3.4 item 5 for every order it
  states and for the member order of every answer its harness spells: the
  order is over UTF-8 bytes, which differs from RFC 8785's UTF-16 code-unit
  order on a string carrying a scalar value above U+FFFF. Every string of an
  answer, a kept member name included, is emitted as `galeed/1` §3.4 item 3
  emits one, under no charset rule of that law.
- **present**: a member is present when its key is present, whatever its
  value; an optional member written with the value `null` is present with a
  form its row refuses, and the only spelling of an absent optional member
  is the absent key.

---

## 2. Decoding a publication

**[D] 2.1 `decode(p)`.** For a publication `p`, decided in this order, the
first failure naming the fault.

1. `keccak256(document)`, spelled as `hex32`, equals `word`: else `E_HASH`.
2. The document bytes are readable by the reader (§1) and the value read is
   an object: else `E_JSON`; unreadable bytes and a readable value that is
   not an object share the one token.
3. `kind` is present and its value is a string: else `E_KIND`.
4. `kind` byte-equal to `galeed.roster/1`: §3 decides the roster table and
   the publication decodes to a **roster**.  `kind` byte-equal to
   `galeed.judge/1`: §3a decides the judge document table and the
   publication decodes to a **judge document**. `kind` byte-equal to
   `galeed.guildamanah/1`: §3b decides the guild covenant table and the
   publication decodes to a **guild covenant**. `kind` byte-equal to
   `galeed.guildcard/1`: §3c decides the guild card table and the
   publication decodes to a **guild card**. Any other string: the
   publication decodes to an **unknown kind**, which is no fault; the kind
   is carried as data and nothing else of the document is read, so an
   unknown-kind answer names no kept members.

A publication that names a fault at any step decodes to nothing: it is not a
roster, not a guild card, not a judge document, not a guild covenant, and
not an unknown kind. `emitter`, `blockNumber`, and `logIndex` enter no test
of this section; they are read by §5.

**2.2 Members beyond a table.** A document of any of the four tables may
carry members beyond its table, at the root and inside any object a table
names, under the reader's rules alone. Such members are data: no predicate
reads their meaning, and no reader refuses a document for what they say;
what they are is decided by §1's reader like every other part of the
document, so an unknown member carrying a fraction, a repeated key, or a
container at depth 129 makes the whole document unreadable (`E_JSON`), and a
guild that writes one takes its own roster off every reader. A reader that
shows a document shows them, and a tool that republishes a document it
fetched carries them unchanged (§7). The harness names the root's members
beyond the table by key, bytewise ascending (§1), so that two readers agree
on what they kept; it names nested ones nowhere. A member whose row carries
its value whole (`quotes` in §3b.1, whose row reads none of its members;
`ledger.basis` in §3a.1, whose row reads all of them and admits no others)
is carried as that value, its own members included; a `venues` element
(§3b.1) is carried with its three members alone, its members beyond them
named nowhere.

---

## 3. The roster (`galeed.roster/1`)

A guild's statement of who it lists, and optionally of which verifiers it
retains; who the guild is, by name, home, and its own words, is its card
(§3c), and the covenant a guild's pool rules under is its own document
(§3b).

**[D] 3.1 Members.**

| key | presence | form |
|---|---|---|
| `kind` | required | string byte-equal to `galeed.roster/1` (decided by §2.1) |
| `guild` | required | hex20: the address that published the roster; compared by §5.4 |
| `chainId` | required | int: the chain the roster is published for; compared by §5.4 |
| `issued` | required | int: Unix seconds, the guild's own statement of when; data for readers, carried by §5.4 |
| `judges` | required | array; every element is either a hex20 string or an object whose `addr` member is present and hex20; members of such an object beyond `addr` are permitted and untested (`since`, an int a guild may write, among them); the empty array is permitted; a repeated address is permitted and is one element per occurrence (§7 binds the producer, never the reader) |
| `bench` | optional | object; two members required, members beyond them permitted and untested. `verifiers`: an array every element of which is either a hex20 string or a **verifier object**, an object whose `addr` is present and hex20, whose `name` is present and a string of unrestricted scalar values, and whose `protocols` is present and an array every element of which is hex32, members beyond those three permitted and untested. `protocols`: an array every element of which is hex32. Both arrays may be empty |


**[D] 3.2 Order of decision.** After §2.1 step 4 has selected this table:
`guild` (`E_ROSTER_GUILD`); `chainId` (`E_ROSTER_CHAIN`); `issued`
(`E_ROSTER_ISSUED`); `judges` (`E_ROSTER_JUDGES`, whether the member is
absent, not an array, or any element is outside its two forms); `bench`,
where present (`E_ROSTER_BENCH`, whether the value is not an object, either
of the two members is absent or outside its form, or any element of either
array is outside its forms). A publication that passes decodes to a roster;
nothing else does. The reader of §1 having refused every number outside
`galeed/1` §3.2's spelling and range, an `int` row of this section can fail
only on a value that is not a number. `E_ROSTER_JUDGES` and `E_ROSTER_BENCH`
each cover several tests and name one token whatever order those tests are
decided in, so this section states no order inside either of them.

**3.3 The address of an element.** The address of a `judges` element, and
of a `verifiers` element, is the string itself where the element is a
string and its `addr` where the element is an object. §5 reads the address
of a `judges` element and nothing else of it; of a verifier object §5 also
carries `name` and `protocols`.

**3.4 Meaning.** `judges` is the guild's list, in the guild's order; the
order is the document's and no reader reorders it. `bench` names the
verifiers the guild retains and the protocol documents it publishes:
`protocols` in a verifier object are that verifier's own protocol digests,
`protocols` of the bench are the guild's; a digest is the `sha256` of the
protocol document's canonical bytes under `galeed/1` §3, the `doc_id` of
`galeed.papers/1` §1, and this law tests the hex32 form alone. A roster
without `bench` retains no verifier; that is a complete statement, and §5.4
resolves it to the empty bench. The asymmetry between the two element rows
is deliberate: a judge is an address, and a verifier object that names
itself names its protocols too; a verifier written as a bare address names
neither and is complete. A guild offers its judges as a **pool** by
publishing a covenant (§3b) from its address: the pool's text and quotes are
that document's, every member carries the text in its own ledger (§7), and a
party who chose the guild by pointer reads one text and one quote as the
covenant that will bind the case, whichever member the venue draws; which
quote a seat reads is §5.10's. A member beyond the table of any nested
object, a `judges` element object, the `bench` object and a verifier object
alike, is the guild's own and reaches no answer (§2.2); a reader that shows
one shows it from the document. A guild that published no covenant is a
candidate set only (§5.6): a party picks a judge from it and reads that
judge's own covenant. How the covenant's word reaches a seat and a member's
own ledger is §7's. What any member beyond the table means is the guild's
(§9).

---
## 3a. The judge document (`galeed.judge/1`)

A judge's public card: who sits at this address, where its ledger is read,
and which covenant version it offers.

**[D] 3a.1 Members.**

| key | presence | form |
|---|---|---|
| `kind` | required | string byte-equal to `galeed.judge/1` (decided by §2.1) |
| `judge` | required | hex20: the address that published the document; compared by §5.7 |
| `issued` | required | int: Unix seconds, the judge's own statement of when; data for readers |
| `amanah` | required | object; `entry_id` required hex32, the `entry_id` of the `amanah` entry of the judge's ledger this document offers as current; members beyond it permitted and untested |
| `ledger` | required | object; `root` required hex20, the root the judge's ledger is audited under (`galeed/1` §7.3); `basis` required, a `galeed/1` §9.4 basis: the rules that section places on the `basis` member alone (an object of the one member `chains`, an array; every `chains` object carrying exactly the five members `chainId`, `fromBlock`, `toBlock` (ints, `fromBlock` ≤ `toBlock`), `registries`, and `senders` (arrays); `chains` ascending by `(chainId, fromBlock)`; two objects of one chain disjoint and, where they touch, differing in `registries` or `senders`; `senders` hex20 ascending without repeats; `registries` hex20 addresses then class items `{codeHash}` whose one member is hex32, each part ascending without repeats), decided by the embedded core (§10.4), which refuses an audit input of any hex20 root with this `basis` and an empty anchor set (`NO_LABEL`) exactly when the basis fails them; under it the judge declares every anchor of its lineage is found; members beyond the two permitted and untested |
| `name`, `home`, `note_md` | optional | string, its scalar values unrestricted: the judge's own words, shown verbatim |

**[D] 3a.2 Order of decision.** After §2.1 step 4 has selected this table:
`judge` (`E_JUDGE_JUDGE`); `issued` (`E_JUDGE_ISSUED`); `amanah`
(`E_JUDGE_AMANAH`, whether the member is absent, not an object, or
`entry_id` absent or not hex32); `ledger` (`E_JUDGE_LEDGER`, whether
absent, not an object, `root` absent or not hex20, or `basis` absent or not
a `galeed/1` basis); `name`, `home`, `note_md`, each where present
(`E_JUDGE_NAME`, `E_JUDGE_HOME`, `E_JUDGE_NOTE`). A publication that passes
decodes to a judge document; nothing else does.

**3a.3 Meaning.** The document carries no covenant text and no quote: the
entry `amanah.entry_id` names carries both (`galeed/1` §6.2), and the chain
carries the entry (`galeed/1` §9), so that a party who reads the document
reads the covenant from the ledger and never from a copy. The document
**offers**: a seat the parties name is frozen under `amanah.entry_id`
(`galeed.venue/1` §2.4), the word a party copies from this document into the
terms; a party has no word for a version the document does not offer, and a
judge who offers another version publishes another document; a seat the
parties nonetheless froze under an entry the ledger carries runs under that
entry (`galeed.venue/1` §2.4), and a judge who will not sit under it resigns
(`galeed/1` §6.4). The document's `word` (§1) pins the document for a reader
that cites it and freezes nothing on a venue; a judge who changes any member
publishes a new document and a new word, and no case is moved by it.
`ledger.basis` is the judge's own declaration of where its anchors are found
(`galeed/1` §9.4: the chains, the emitters by address or class, the lineage
keys in `senders`, the blocks), on chains whose past state readers can
consult (`galeed/1` §9.3); its `toBlock` on each chain is the head the judge
saw when it published, and a reader scans to the head it reads: a reader may
widen the declaration, and narrows it only by saying so. Whether
`amanah.entry_id` names an `amanah` entry of the ledger rooted at
`ledger.root` is resolution under `galeed/1` §11, decided by the reader that
scanned the ledger, and never a fault of this table; and a document whose
`judge` the scanned ledger's lineage (`galeed/1` §7.4) does not hold is a
stranger's card, however the ledger reads: a reader names a seat from a
document only where the document's `judge` is a key of the ledger it points
at, since any address may publish a document naming any root. A reader names
a seat from a document only where its `judge` was the current key of its own
record at the document's position (§5.2) and has no retirement (`galeed/1`
§7.4) in an audit of the ledger it points at: a key that has handed off
publishes a stranger's card.

---

## 3b. The guild covenant (`galeed.guildamanah/1`)

A guild's covenant, published: the rules every member drawn from its pool
rules under, and the quotes the pool charges.

**[D] 3b.1 Members.**

| key | presence | form |
|---|---|---|
| `kind` | required | string byte-equal to `galeed.guildamanah/1` (decided by §2.1) |
| `guild` | required | hex20: the address that published it; compared by §5.9 |
| `chainId` | required | int: the chain the covenant is published for; compared by §5.9 |
| `issued` | required | int: Unix seconds, data for readers |
| `version` | required | int: the guild's own numbering |
| `text_md` | required | string, its scalar values unrestricted: the covenant in full, in the guild's words |
| `venues` | optional | array; every element an object; `chainId` required int, `contract` required hex20, `codeHash` required hex32; members beyond these permitted and untested (the table of `galeed/1` §6.2) |
| `quotes` | optional | object: the pool's quote bytes by venue, read by §5.10; this table tests that it is an object and nothing of its members |

**[D] 3b.2 Order of decision.** After §2.1 step 4: `guild`
(`E_GUILDAMANAH_GUILD`); `chainId` (`E_GUILDAMANAH_CHAIN`); `issued`
(`E_GUILDAMANAH_ISSUED`); `version` (`E_GUILDAMANAH_VERSION`); `text_md`
(`E_GUILDAMANAH_TEXT`); `venues`, where present (`E_GUILDAMANAH_VENUES`,
whether not an array or any element outside its form); `quotes`, where
present (`E_GUILDAMANAH_QUOTES`, not an object). A publication that passes
decodes to a guild covenant; nothing else does.

**3b.3 Meaning.** The table carries the members of a judge's `amanah` entry
(`galeed/1` §6.2: `version`, `text_md`, `venues`, `quotes`) under the
document's own `kind`, `guild`, `chainId`, and `issued`, as a registry
document instead of a ledger entry: a judge's covenant is signed by the
judge's key, a guild's is published from the guild's address, whatever
stands behind that address (§9), and each is read by the one authority its
law gives it. The guild's **current covenant** is the last publication of
its kind from the guild's address, as `candidateOf` reads it (§1, §5.9); a
new version is a new publication, a correction is a new version, and a case
keeps the word it froze. The word a venue freezes for a pool seat is the
`sha256` of the UTF-8 encoding of `text_md`'s scalar values
(`galeed.venue/1` §2.4, §7 below), and never this document's `word`, so that
the member's own copy of the text (§7) resolves to the same word in the
member's ledger.

---

## 3c. The guild card (`galeed.guildcard/1`)

A guild's public card: who the guild is, where it is found, and its own
words.

**[D] 3c.1 Members.**

| key | presence | form |
|---|---|---|
| `kind` | required | string byte-equal to `galeed.guildcard/1` (decided by §2.1) |
| `guild` | required | hex20: the address that published the card; compared by §5.11 |
| `issued` | required | int: Unix seconds, the guild's own statement of when; data for readers |
| `name`, `home`, `note_md` | optional | string, its scalar values unrestricted: the guild's own words, shown verbatim |

**[D] 3c.2 Order of decision.** After §2.1 step 4 has selected this table:
`guild` (`E_GUILDCARD_GUILD`); `issued` (`E_GUILDCARD_ISSUED`); `name`,
`home`, `note_md`, each where present (`E_GUILDCARD_NAME`,
`E_GUILDCARD_HOME`, `E_GUILDCARD_NOTE`). A publication that passes decodes
to a guild card; nothing else does.

**3c.3 Meaning.** The card says who the guild is and how to reach it; it
lists nobody and binds no case. Who the guild lists is its roster (§3), and
what its pool rules under is its covenant (§3b). A guild that changes
anything on its card publishes a new card, which moves no roster, no
covenant, and no case; the current card is the last publication of its kind
from the guild's address, as `candidateOf` reads it (§1, §5.11), and a card
has no expiry. The three optional members are spelled and decided as a judge
document's (§3a.1), the two cards differing in their `kind`, in naming their
publisher `guild` or `judge`, and in the judge document's `amanah` and
`ledger`, which a guild's card leaves to its covenant; what the members say
is the guild's (§9).

---


## 4. The succession book

A book records, for each record, its current key. A key hands its record to
another key by a **declaration**, the other key takes it up by an
**accession**, and only the two together move the record. The book is
decided on chain by its own code and re-derived by every reader from its
logs; it keeps no document and reads none.

**[R] 4.1 Records.** A **declaration** is a record `(emitter, blockNumber,
logIndex, to)` taken from one `Declared` log, `emitter` the log's `from`; an
**accession** is a record `(emitter, blockNumber, logIndex, holder, root)`
taken from one `Acceded` log, `emitter` the log's `to`. Both are records of
the logs set (§1), and no two records of the set share `(blockNumber,
logIndex)`. An accession's `root` is the book's own statement of the
record's root, read for its form and by no step of §4.2.

**[R] 4.2 Replay.** Walking the declarations and accessions of `L` in
ascending `(blockNumber, logIndex)`, `blockNumber` compared first, a reader
keeps for each address its **root** (initially the address itself), for each
root its **head** (initially the root), for each address its **pending** key
(initially none), and whether each address has **entered** the book
(initially not). The **current key** of an address at a position is the head
of that address's root there.

- A declaration **binds** iff its `emitter` is the current key of its own
  record at its position, and its `to` is not the zero address, not the
  `emitter`, and has not entered the book; it makes `to` the `emitter`'s
  pending key, in place of any earlier one, and the `emitter` enters the
  book. Any other declaration binds nothing.
- An accession **counts** iff its `emitter` has not entered the book, is the
  pending key of `holder`, and `holder` is the current key of its own record
  at that position; it sets the `emitter`'s root to `holder`'s root, that
  root's head to the `emitter`, and `holder`'s pending key to none, **taking
  up** the last declaration of `holder` that bound, the one that made the
  `emitter` `holder`'s pending key; and the `emitter` enters the book. Any
  other accession counts for nothing.

The replay is total: every record of any logs set binds, counts, or does
nothing, and every answer of §5 holds relative to the book the reader
declared (§9). A book of §1 admits exactly the records this replay binds and
counts, and a reader that finds one it would not, its logs set holding every
record of the book before it, is reading a contract that is no book of this
law.

**4.3 Meaning.** A declaration is the current key's offer and moves nothing;
the record moves when the key it names accepts, and only that key can, since
an accession is sent from the accepting key's own address. A declaration
that names the wrong key is replaced by declaring again. A key that has
entered the book is never named again by a declaration that binds, so no
record returns to a key it held and no two records merge, and every record
is a finite list of keys (§5.3). The book says which key acts for a record
at a venue and speaks for it in this registry; a judge's authority to sign
ledger entries is the ledger's (`galeed/1` §7.3, §9 below).

---

## 5. Resolution

Every predicate of this section is decided from a logs set `L`, the chain
id `chain` it was scanned on, and the query, and nothing else.

**[R] 5.1 Publications of an address, and standing.** `pubs(a)` is the
sequence of publications of `L` whose `emitter` equals `a`, ascending by
`(blockNumber, logIndex)`, `blockNumber` compared first; `candidateOf(a, K)`
(§1) is the last of them that is of kind `K`. `active(a)`
is the `active` of the status of `L` whose `emitter` equals `a` and whose
`(blockNumber, logIndex)` is greatest among such statuses, `blockNumber`
compared first, and is `true` where `L` holds no status of `a`. A guild
whose latest status carries `false` is **retired**; a later status carrying
`true` resumes it, and nothing else does: retirement is a fact of the latest
status alone and is neither a window nor a terminal state. Both functions
read the address they are given and follow no record: standing at an
address whose record continues elsewhere is read at that address by these
two and at the final address by §5.4 and §5.8.

**[R] 5.2 `currentAt(a, p)`.** A position `p` is a pair `(blockNumber,
logIndex)` in §4.2's order, `blockNumber` compared first, whether or not a
record of `L` sits at it. `currentAt(a, p)` is the current key of `a`'s
record as the replay of §4.2 up to `p` has it, the records at `p` and after
it in that order unread, so that a key asked before it acceded is its own
record's current key there; `current(a)` is the current key after every
record of `L`, a record at the greatest position `(2^53 − 1, 2^53 − 1)`
included, which every `currentAt` leaves unread.

**[R] 5.3 `resolveIdentity(a)`.** The **keys** of `a`'s record are its root
first and then each key in the order it acceded (§4.2); `final` is
`current(a)` (§5.2), the last of them. `trail` is those keys taken from `a`
up to and excluding `final`, empty where `a` is `final`. `attempts` is every
declaration whose `emitter` is a key of that trail or `final`, whether or
not it bound and whenever the key sent it, the declaration `final` has
pending included, other than those a counting accession took up (§4.2), each
recorded as `(blockNumber, logIndex, to)` in the order of §4.2's replay. The
trail never holds the final address, and the final address and the trail
together name every key of the record from `a` on; the keys before `a` begin
the root's trail. The book admits no return and no merger (§4.3), so a
record is a finite list and needs no bound.

**[R] 5.4 `resolveGuild(g)`.** Decided in this order, the first stop
or fault being the answer:

1. `resolveIdentity(g)`: let `G` be the final address and `trail` the trail.
2. `active(G)` is `false`: `E_RETIRED`. Standing is read at `G` alone: a
   retirement at an address of the trail binds no reader, and a retirement
   at `G` retires the guild whatever its origin's status.
3. `candidateOf(G, roster)` is none: `E_NO_DOCUMENT`.
4. `decode(candidateOf(G, roster))`: a fault is the answer with that token,
   a decoding token of §2.1 or §3.2; a kind beginning with `galeed.roster/`
   and not equal to `galeed.roster/1`: `E_UNKNOWN_KIND`, another roster
   version this reader does not know; else let `R` be the roster.
5. `R.guild` equals `G`: else `E_GUILD_MISMATCH`. `R.chainId` equals
   `chain`: else `E_CHAIN_MISMATCH`.
6. The guild resolves. Its **judges** are one row per element of `R.judges`,
   in document order: `listed`, the element's address (§3.3), and
   `resolveIdentity(listed)`, which contributes `final` and `trail`; a row
   carries no attempts (a reader that wants them resolves the address by
   §5.3).
7. Its **verifiers** are one row per element of `R.bench.verifiers`, in
   document order, and none where `R` has no `bench`: `listed`, the
   element's address (§3.3); `name` and `protocols` exactly where the
   element is a verifier object, as written; and `resolveIdentity(listed)`,
   contributing `final` and `trail`. Its **bench** is the set of every key
   of the record of every verifier row's `listed` address (§5.3), the keys
   before the listed address included, bytewise ascending, without repeats;
   the empty array where `R` has no `bench`, which is a resolution and never
   `unresolved`. Its **protocols** are `R.bench.protocols` as written, or
   the empty array where `R` has no `bench`.
8. Its **issued** is `R.issued` as written, the guild's own statement of
   when, a datum for display and never a predicate; how long ago the roster
   reached the chain is the reader's to read from the block that carries it.
   Its **card** is `guildCardOf(G)` (§5.11), the reading as that section
   gives it; the member is present in every guild answer, `CARD_NONE` saying
   that the guild published no card. Its **covenant** is `covenantOf(G)`
   (§5.9): `null` where `covenantOf` finds no candidate, the stop with its
   token where `covenantOf` stops, else the current covenant's `word`,
   `blockNumber`, `version`, `sha256` (of `text_md` as §3b.3 takes it), and
   `venues` and `quotes` exactly where present; the member is present in
   every guild answer, `null` saying that no pool is offered. The root's
   members beyond §3.1's table are carried by key (§2.2). The guild's own
   address in the answer is `G`, which step 5 has made equal to `R.guild`.

The refusals of steps 2 through 5 are what keep the format fail-closed: a
reader that cannot read a guild's last roster declines the whole guild
rather than falling back to an older roster it can read, because the roster
it cannot read may be the one that removed somebody; a publication of
another kind from the guild's address is no roster and moves nothing here
(§1); and a reader never resolves a guild through a document the guild did
not publish under the address the record continues at.

**[R] 5.5 The bench, three-valued.** A judge's covenant names a bench by a
value `B`, carried in the `amanah` body under the member name `bench`, a
member `galeed/1` §6.9 permits and does not read. `bench(B)` is one of three
values, the `bench` input of `galeed.papers/1` §1:

- `B` absent or `null`: `null`; the door asks no bench question.
- `B` a string: where `B` is a pointer (§6) whose `chainId` equals `chain`,
  `resolveGuild(B.address)`, whose bench (§5.4 step 7) is the value where
  the guild resolves, and `unresolved` where it names a fault; where `B` is
  no pointer, or names another chain, `unresolved`. This function names no
  token: the distinctions §5.6 draws are its own.
- `B` an array: where every element is hex20, the set of every key of the
  record of each element `e` (§5.3), bytewise ascending without repeats;
  where any element is not hex20, `unresolved`.
- `B` any other value: `unresolved`.

**[R] 5.6 `candidates(pointers, allow, deny)`.** `pointers` is a sequence of
strings, `allow` and `deny` sequences of hex20; the element types are
preconditions on the query and never an answer of this law. For each pointer
in order its **result** is: `E_POINTER` where the string is no pointer (§6);
`E_POINTER_CHAIN` where its `chainId` differs from `chain`; else
`resolveGuild(pointer.address)`. The **candidate list** is built in this
order, every address, from either half, being appended only where it is not
already in the list: for each pointer in order whose result is a resolved
guild, for each judges row in order, the row's `final`, unless `deny`
contains a key of the record of the row's `listed` address (§5.3); then each
element of `allow` in order. The list is ordered by construction and by
nothing else; the law ranks nothing. `deny` reaches every key of a judge's
record, the keys before the listed address and after it alike, so denying
any key a judge has held denies the judge whichever key the roster lists;
`deny` is read against judges rows alone and suppresses no pointer and no
guild, whatever the guild's own trail carries, a row whose record is the
guild's own being a row like any other; `allow` names final addresses and
needs no guild, and is applied after `deny`: the candidates are the finals
of the judges rows whose records hold no key of `deny`, then `allow`, each
address once.

**[R] 5.7 `judgeOf(a)`.**
Decided from `a` and `L`, and from nothing else. The candidate is
`candidateOf(a, judge)`, and the answer is `JUDGE_NONE` where there is none.
Otherwise, in this order: `decode` (§2.1 and §3a; a fault is `JUDGE_UNREAD`
with its token; a kind beginning with `galeed.judge/` and not equal to
`galeed.judge/1` is `JUDGE_UNREAD` with `E_UNKNOWN_KIND`); the document's
`judge` differs from `a`: `JUDGE_MISMATCH`; else `JUDGE` with the document's
`word`, `blockNumber`, and the decoded document as §2.2 carries it. A reader
thus declines the whole document wherever the last publication of this kind
from `a` is one it cannot read, for the reason §5.4 gives. `judgeOf` reads
the address it is given and follows no record (§5.1); §5.8 is the reading
that follows one.

**[R] 5.8 `resolveJudge(a)`.** Decided from `a` and `L`:
`resolveIdentity(a)` (§5.3); let `J` be the final address and `trail` the
trail. The answer is `J`, `trail`, `active(J)` (§5.1, read at `J` alone as
§5.4 step 2 reads a guild's), and `judgeOf(J)` (§5.7), whatever that reading
answers. A party, a roster's `judges` row, and a venue's record may name an
address the judge has moved from; this function is how every reader of the
family gets from such an address to the judge's current document, and §5.6's
rows are its first half. What a reader does with a judge whose `active(J)`
is `false`, or whose document reads `JUDGE_NONE`, is the reader's (§9).

**[R] 5.9 `covenantOf(G)`.** Decided from `G`, `L`, and `chain`. The
candidate is `candidateOf(G, guildamanah)`; none: `null`. Otherwise, in
this order: `decode`, `D` being the guild covenant it yields (a fault is a
stop with its token; a kind beginning
with `galeed.guildamanah/` and not equal to `galeed.guildamanah/1` is a
stop with `E_UNKNOWN_KIND`); `D.guild` differs from `G`: a stop with
`E_GUILDAMANAH_GUILD_MISMATCH`; `D.chainId` differs from `chain`: a stop
with `E_GUILDAMANAH_CHAIN_MISMATCH`; else the covenant: `word`,
`blockNumber`, `version`, `sha256` (§3b.3), and `venues` and `quotes`
exactly where present. `G` is a final address (§5.3): this function follows
no record, and §5.4 hands it the final address it resolved.

**[R] 5.10 `quoteOf(Q, chainId, contract, codeHash, token)`.** `Q` is the
`quotes` object of an `amanah` entry body (`galeed/1` §6.2) or of a guild
covenant (§3b); `chainId` an int, `contract` a hex20, `codeHash` a hex32,
`token` a hex20 or `null`. A **venue key** is one, two, or three segments
joined by `:`, no segment carrying one and nothing else in the string: the
first an `int` spelled as `galeed/1` §3.2 spells one, in [0, 2^53 − 1], the
venue's chain; the second, where present, a hex20 (an address item, the
declared address) or a hex32 (a standard item, its code hash); the third,
where present, a hex20, the settlement token the quote is priced in. The
five forms `<chainId>`, `<chainId>:<hex20>`, `<chainId>:<hex20>:<hex20>`,
`<chainId>:<hex32>`, `<chainId>:<hex32>:<hex20>` never alias, the segment
count and the second segment's length telling them apart. A **quote
spelling** is the two lowercase bytes `0x` followed by an even number of
lowercase hexadecimal digits, `0x` alone spelling the empty byte string,
lowercase in both halves, since these bytes are compared to what a venue
quotes and never normalized. The keys are tried in this order, a key naming
a token being skipped where `token` is `null`:
`<chainId>:<contract>:<token>`, `<chainId>:<contract>`,
`<chainId>:<codeHash>:<token>`, `<chainId>:<codeHash>`, `<chainId>` spelled
as `galeed/1` §3.2 spells an int, so that a quote keyed to the address
precedes one keyed to the code and a quote keyed to the token precedes one
keyed to none. The answer is `QUOTE` with the key and the value under the
first of those keys `Q` carries, where that value is a quote spelling;
`E_QUOTE_FORM` with that key where the value is not; `QUOTE_NONE` where `Q`
carries none of them, a carrier without a `quotes` member having `Q` empty.
A key of `Q` the order never reaches is data, and a value under it is
untested. One function reads a judge's quotes and a pool's alike, with the
same five arguments (§7); which carrier a seat reads, the judge's own
`amanah` entry for a seat the parties named and the guild's covenant for a
seat the venue drew, is `galeed.venue/1` §2.4's and §2.5's.

**[R] 5.11 `guildCardOf(G)`.** Decided from `G` and `L`, and from nothing
else. The candidate is `candidateOf(G, guildcard)`, and the answer is
`CARD_NONE` where there is none. Otherwise, in this order: `decode` (§2.1
and §3c; a fault is `CARD_UNREAD` with its token; a kind beginning with
`galeed.guildcard/` and not equal to `galeed.guildcard/1` is `CARD_UNREAD`
with `E_UNKNOWN_KIND`); the card's `guild` differs from `G`:
`CARD_MISMATCH`; else `CARD` with the card's `word`, `blockNumber`, and the
decoded card as §2.2 carries it. `G` is a final address (§5.3): this
function follows no record of the succession book, and §5.4 hands it the
final address it resolved.


---

## 6. The pointer

**[D] 6.1** A pointer is a string of exactly the bytes `galeed:guild:`,
then an `int` spelled as `galeed/1` §3.2 spells one (decimal digits, no
sign, no leading zero, the single digit `0` included, in [0, 2^53 − 1]),
then `:`, then a hex20; its `chainId` is the integer and its `address` the
hex20. Any other string is no pointer: an uppercase digit in the address,
a checksummed address, a chain id with a leading zero, a chain id at or
above 2^53, an empty chain id, surrounding whitespace, or a different
prefix among them.

Two fields and nothing else: a roster's content and location are read from
the chain at use time, so a pointer never goes stale.

---

## 7. On-chain expression and obligations

**[O]** This section binds producers and consuming tools alone and states
no predicate; §2 through §6 decide what a reader sees.

Every kind rides the registry's `publish(document)` call, sent from the
publisher's own address, and nothing else: `document` is the document bytes,
and the registry computes the word. The document bytes a tool publishes are
the canonical bytes of the document's value under `galeed/1` §3.4 (members
ordered bytewise over their UTF-8 keys, no whitespace outside a string
value, integers plain, strings escaped as that section's item 3 fixes), and
nothing of `galeed/1` §3.3 or §3.5 applies to them: the writer of `galeed/1`
§3.4 alone produces them, a name in any script is spelled raw, and that
law's §3.5 acceptance test, whose tests 4 and 5 carry §3.3's charsets, is an
entry predicate that refuses a lawful roster. This binds producers alone; a
reader decides nothing on it (§1), a document that is not canonical is a
document all the same, and two tools handed the same roster publish one
word, which is the word `galeed.venue/1` §4 records as `rosterHash`. The
document lives entirely on the chain, in the publishing transaction and its
log, so no document of this law has a host, a surface of its own that can go
down, or a way to leak who reads it. `setStatus(false)` retires and
`setStatus(true)` resumes (§5.1). Every roster change is a whole-document
republish; removal takes effect at the next resolution, with no window and
no revocation list.

A guild's tool refuses to publish a roster whose `guild` is not the sending
address; whose `chainId` is not the chain it sends on; whose `judges` repeat
an address, or carry an element that is not an address of §3.3's forms, a
checksummed address among them, the tool saying which value and that no
reader normalizes case; whose `bench` is outside §3.1's form; or whose
bytes, `keccak256` of them taken as the `word`, §2.1 and §3 do not decode to
a roster, an unknown member outside the reader's universe (§2.2) among the
causes. It carries every member it fetched and does not know back into the
document it republishes, working from the document bytes and never from a
reader's answer, which names nested members nowhere (§2.2). A key's tool
moves its record by a declaration from the current key and an accession from
the new key, the new key's first act on the book, fills the declaration's
`to` from the new key's own signature, never by hand, names as the
accession's `holder` the key it moves from, never a holder found by scanning
logs, and reads `current(holder)` after the accession to see the move
landed. A key's tool moves its record on the book of every chain the family
declares a book for (§9), on each as the family declares it, whether or not
the record has left anything there (a document, a seat, a panel, an account
at a venue), since a guild may list the record on any of them. A judge's
tool also has the ledger carry the succession on every chain of its basis
(`galeed/1` §5.7, §6.6), the ledger's lineage being the seat's authority and
the book only the way to the new key (§9). What a reader would refuse never
reaches the chain; the reader's predicates are nonetheless the only ones
that decide.

A judge's tool publishes a judge document whose `judge` is the sending
address; whose `amanah.entry_id` is the `entry_id` of an `amanah` entry of
the ledger rooted at `ledger.root` (`galeed/1` §8.1), so that a reader
scanning `ledger.basis` finds it; whose `ledger.basis` lists among its
`senders` the root and every key the ledger's successions name, among its
`registries` every emitter the seat anchors through, by address or by class,
and as each chain's `toBlock` the head it saw; and it publishes the document
again whenever the entry it offers changes, since the entry's `entry_id` is
the word a named seat freezes (§3a.3), and whenever the basis or any member
changes. It refuses to publish a document whose `entry_id` names no such
entry, and one whose `basis` omits a lineage key.

A guild's tool publishes a card whose `guild` is the sending address, and
publishes it again whenever any of its members changes, apart from any
roster. A guild's tool publishes a covenant whose `guild` is the sending
address and whose `chainId` is the chain it sends on; the text and the
quotes are the guild's and bind every member alike. It publishes the
covenant before it engages a panel under it, and it refuses to engage a pool
whose members' ledgers lack the current text (the pool form below).

**The pool form.** Where a case's terms name a guild pointer in place of a
judge, the judge is drawn by the venue from a panel the guild engaged
(`galeed.venue/1` §4) and never chosen by the parties' clients. A guild that
engages such a panel gives every member the same word, the `sha256` of its
current covenant's `text_md` (§3b.3, `covenantOf`), as that member's
`amanahHashes` element, so that the venue freezes it as the seat's
`amanahHash` (`galeed.venue/1` §2.4 names this word beside a named seat's,
and §4 refuses a panel whose words differ); every member of the pool carries
in its own `galeed/1` ledger an `amanah` entry whose `text_md`, as the
reader decodes it, is the guild's text scalar value for scalar value, so
that the `sha256` of that value's UTF-8 encoding equals that word, the
entry's own bytes spelling it as `galeed/1` §3.4 item 3 spells any string
and the digest never being taken over those; the seat's word thus resolves
in that member's ledger (`galeed/1` §6.3 has a ruling's `amanah` name an
entry of the seat's own ledger, a reference a reader resolves under that
law's §11), and the guild engages no member whose ledger lacks that entry,
and engages no pool whose `judges` is empty, which the venue would refuse.
The quote a party reads is the guild's and binds every member alike: where a
venue decodes a quote for a drawn seat, the bytes are
`quoteOf(covenant.quotes, chainId, address, codeHash, token)` (§5.10) with
`chainId` the seat's chain, `address` the venue's declared address,
`codeHash` the code hash of the deployment the seat is on, and `token` the
settlement token the case is priced in, the same five arguments a venue
hands `quoteOf` over a named judge's own `quotes`; the seat's `quoteHash` is
those bytes' `keccak256`, and zero where the venue decodes no quote
(`galeed.venue/1` §2.5), which a member's desk reconciles against the
guild's quote and never against its own. A client acting for a party before
it signs such terms resolves the pointer (§5.4), requires the guild's
`covenant` present (step 8), shows the party the text read from the chain
and the quote `quoteOf` yields for this venue, and refuses to sign where
`covenantOf` stopped, where the roster's `judges` is empty, or, at a venue
whose `quoteType()` is non-empty, where `quoteOf` yields `QUOTE_NONE` or
`E_QUOTE_FORM`; a quote of zero bytes, spelled `0x`, is a quote present. A
venue standard's own document may narrow the key forms of §5.10 for its
venues (a standard with no declared address has no address form; one that
prices by settlement token may require the token form), the narrowing being
the standard's to publish, and a client reading such a standard refuses what
the narrowing refuses. Which member is drawn is the venue's rule, or, where
the guild names a router in its panel (`galeed.venue/1` §4), the router's: a
guild's router is immutable, owns nothing, and reads no state the guild or a
member can write, and this law reads nothing of it. Whether an unaccepted
offer is redrawn is the venue's rule alone.

A tool that acts on a candidate set re-resolves every pointer at the moment
of the act and refuses to act on an address the fresh set does not hold; a
cached set is display (§9). This is what gives removal its effect: a guild
that republishes without an address removes it from every act after the
next resolution. Which acts count, and what a tool does when a pointer
fails to resolve, are the tool's (§9).

---

## 8. Tokens and vocabularies

Every token an answer of this law carries is one of these, and no reader
emits another:

- decoding (§2.1): `E_HASH`, `E_JSON`, `E_KIND`;
- the roster (§3.2): `E_ROSTER_GUILD`, `E_ROSTER_CHAIN`, `E_ROSTER_ISSUED`,
  `E_ROSTER_JUDGES`, `E_ROSTER_BENCH`;
- the guild card (§3c.2): `E_GUILDCARD_GUILD`, `E_GUILDCARD_ISSUED`,
  `E_GUILDCARD_NAME`, `E_GUILDCARD_HOME`, `E_GUILDCARD_NOTE`;
- the judge document (§3a.2): `E_JUDGE_JUDGE`, `E_JUDGE_ISSUED`,
  `E_JUDGE_AMANAH`, `E_JUDGE_LEDGER`, `E_JUDGE_NAME`, `E_JUDGE_HOME`,
  `E_JUDGE_NOTE`;
- the guild covenant (§3b.2, §5.9): `E_GUILDAMANAH_GUILD`,
  `E_GUILDAMANAH_CHAIN`, `E_GUILDAMANAH_ISSUED`, `E_GUILDAMANAH_VERSION`,
  `E_GUILDAMANAH_TEXT`, `E_GUILDAMANAH_VENUES`, `E_GUILDAMANAH_QUOTES`,
  `E_GUILDAMANAH_GUILD_MISMATCH`, `E_GUILDAMANAH_CHAIN_MISMATCH`;
- resolution (§5), the tokens that section originates: `E_RETIRED`,
  `E_NO_DOCUMENT`, `E_UNKNOWN_KIND` (another version of the kind read),
  `E_GUILD_MISMATCH`, `E_CHAIN_MISMATCH`, `E_POINTER`, `E_POINTER_CHAIN`
  (`E_CHAIN_MISMATCH` is step 5's, on a roster's own `chainId`;
  `E_POINTER_CHAIN` is §5.6's, on a pointer string, which never reaches a
  guild; one `candidates` answer can carry both); the readings `JUDGE`,
  `JUDGE_NONE`, `JUDGE_UNREAD`, `JUDGE_MISMATCH` (§5.7), `QUOTE`,
  `QUOTE_NONE`, `E_QUOTE_FORM` (§5.10), `CARD`, `CARD_NONE`, `CARD_UNREAD`,
  `CARD_MISMATCH` (§5.11); a guild answer carries any token of the decoding,
  roster, guild card, and guild covenant groups above as well, through §5.4
  steps 4 and 8, and a judge reading any token of the decoding and judge
  document groups, through §5.7;
- kinds: `galeed.roster/1`, `galeed.guildcard/1`, `galeed.judge/1`,
  `galeed.guildamanah/1`; reserved prefixes: `galeed.roster/`,
  `galeed.guildcard/`, `galeed.judge/`, `galeed.guildamanah/`,
  `galeed.verifier/`; bench states: `null`, `unresolved`, an array; pointer
  prefix: `galeed:guild:`. The four kinds, the five reserved prefixes, and
  the pointer prefix are literals of the GALEED family, registered where the
  family registers its literals.

---

## 9. Outside the grammar

The following are decided by guilds, judges, readers, and family laws,
never by this law:

- **Which registries a reader scans.** A reader declares the addresses whose
  logs it reads, as `galeed/1` §9.4's basis declares its registries, and the
  declaration is part of what a resolution is relative to; this law tests no
  address and pins no code (§1). A reader that scans an address whose code
  it has not read accepts every address's record from that address's
  callers; the remedy is pinning a registry by its code hash, which is what
  the desk does, and the same holds for the succession book a reader
  declares (§1). A book answers for the chain it lives on: on a chain whose
  book never recorded a move, the address that moved is its own current key,
  so the family declares, where it registers its literals (§8), for each
  chain the book its readers resolve identities on there, and a record moves
  on the book of every chain the family declares one for (§7).
- **Who a guild admits, and how it governs.** No cap on how long a roster
  may live, no required cadence, no minimum size, no admission rule, and no
  opinion on the guild's account form: a single key, a multisig, a timelock,
  and a company container are indistinguishable, since guild authority is
  expressed only as transactions sent from the guild's address, and no
  detached signature is ever read.
- **What a guild's listing follows.** A roster lists addresses, and §5.4
  step 6 resolves each through §5.3. A guild that does not want a successor
  to carry its guarantee republishes without that address; until it does,
  the successor is a candidate under that guild. Whether the guild removes,
  keeps, or re-admits is the guild's, and a guild's tool that watches the
  succession book for its listed addresses does the guild's work and never
  this law's.
- **Which guilds a reader follows.** A reader ships no guild and suggests
  none; the first pointer is added by hand. A pointer that fails to resolve
  contributes nothing this time (§5.6), and a reader re-resolves before it
  acts (§7): a cached candidate set is display, and display says when the
  cache was taken.
- **Multi-homing.** A judge expelled from one guild and still listed by
  another is a candidate through the other; the law does not merge rosters
  and does not rank them.
- **What a trail means.** A reader shows the trail whenever an address was
  followed, so that an address that moved reads as continuing elsewhere and
  never silently becomes another party; whether a predecessor's record and a
  successor's are one record is the reader's judgement, and the law merges
  nothing.
- **Take-over.** A guild address that is taken over can list whom it likes;
  the reader's remedy is deleting the pointer, and no recovery mechanism
  exists, by design.
- **The registry carries every kind.** One `publish` call carries every
  document a family law puts on the registry, and each reader selects the
  last publication of the kind it reads (§1, `candidateOf`), a publication
  it cannot place counting as every kind. One address may therefore carry a
  roster, a card, and a covenant, or a judge document; which kinds an
  address is expected to carry is the desks' affair. A judge publishes a
  judge document (§3a); a guild publishes a roster (§3), a card (§3c), and a
  covenant (§3b); either moves its key on the succession book (§4).
- **Two rails.** A judge's key moves on two rails: the ledger's `succession`
  entry (`galeed/1` §6.6, §7.3), which is the seat's authority and the only
  thing a ruling's standing turns on, and the succession book (§4), which is
  how a venue and a reader get from an address to the current key and
  nothing more (§5.3, §5.8). The ledger retires the ceding key at the
  earliest block time of a record carrying any succession or migration it
  signed and counts none of its records at which it has retired, those later
  than that among them (`galeed/1` §7.4, §8.1), the seat carrying it on
  every chain of its basis; the book stops it acting at a venue from the
  accession on. A reader that finds a book successor the ledger's lineage
  does not hold reads a judge with no ledger standing: its anchors are
  `DISCARDED` and its rulings `EXCLUDED` under the root (`galeed/1` §8.1),
  whatever a venue executed; the judge's obligation to carry the succession
  on every chain of its basis is `galeed/1` §5.7's, and the reader's check
  that a document's `judge` is a lineage key is §3a.3's.
- **The covenant's text.** What a covenant's text says, which version a
  judge offers (§3a.3), and how a judge's personal covenant and a pool's
  text coexist in one ledger: the covenant and the desks (§7 states the
  obligations alone); this law reads `bench` (§5.5) and `quotes` (§5.10) and
  nothing else of a covenant.
- **The venue's panel.** A guild that wants its judges drawn puts a panel on
  the venue, or on the panel book the venue's standard names
  (`galeed.venue/1` §4), resolving its members through §5.3 for that
  transaction; the venue reads no roster, and the `rosterHash` it records is
  the `keccak256` of the document bytes the guild resolved from, this law's
  `word`.
- **Display.** How a roster's `issued`, a retired guild, an attempt, and a
  member beyond the table are shown; that a retired guild contributes no
  judge is §5.4's, and that it is shown as such is the reader's. An address
  carrying code is shown as such, since control of a contract can rotate
  with no record this law reads and no change to the address a venue froze
  (`galeed.venue/1` §1).

---

## 10. Freeze and evolution

**10.1** The reference implementation freezes when independently written
implementations, produced from this text and its harness contract alone,
agree with it on every input of a generated corpus, on every answer of every
command the harness contract compares (among them every decoding answer with
its token or its decoded document, every publication and standing answer,
every identity answer with its final address, trail, and attempts, every
current key at a position, every guild answer with its final address and
trail, its judges rows, verifier rows, bench, protocols, issued, card,
covenant, and the members it carries as written, every bench value, every
candidate list with its per-pointer results, every judge document reading
and every judge resolution, every guild card reading, every covenant
reading, and every quote reading), with every closed boundary of this text
witnessed on both sides by at least one corpus input under every seed the
corpus is generated with, the boundary list being a precondition on the
corpus and never a property claimed of it afterwards; the corpus carries the
list beside itself. A closed boundary of this text is a boundary the corpus
can witness on the surface the harness contract compares; a rule with no
reachable failing side is witnessed once and states no boundary in this
sense, and the `B` absent half of §5.5's first row, which the harness spells
only as `null`, is such a side; §1's bytewise order over UTF-8 is witnessed
on the answer surface by `unknownFields` and by the members `quotes` keeps,
§3b.1 testing none of them. No `[O]` rule is in the corpus and the freeze
certifies none.

**10.2** Once §10.5 names a release digest, agreement with the frozen core
on the corpus is the test anyone can run in seconds, and it is evidence of
conformance and never its definition: conformance is agreement on every
input, and an input outside the corpus on which a reader differs from the
core is a non-conformance the corpus did not catch and a reason to grow it.

**10.3** `galeed.roster/2`, `galeed.guildcard/2`, `galeed.judge/2`, and
`galeed.guildamanah/2` are new `kind` literals under the reserved prefixes
of §1. A reader of this law that finds the last publication of a kind it
reads under another version of that kind refuses it (§5.4 step 4, §5.7,
§5.9, §5.11): that refusal is the evolution valve, and a publisher that
moves to a later kind moves every reader with it or loses the readers it
left. A document accepted under this law is accepted under it forever.

**10.4** This law and `galeed/1` are two laws: a change to one is never a
change to the other. This law's core embeds the `galeed/1` core of the
release digest named in that law's §12.5 for `keccak256` and `sha256`, for
the canonical form of its answers, and for the basis rules of that law's
§9.4 that §3a.1 names, and reads no other predicate of that law; the reader
of §1 is defined in §1 in full, and that core's implementation of it is what
this law's core runs. That core's release digest is
`0x5f24cd212705b0cbfd00e5fe0c720ca059b662276c7eaac9a7907e621c50e65d`
(named in that law's §12.5); a registry core built on any other `galeed/1`
core is another core.

**10.5 The core's identity.** Naming a release digest here is the freeze,
and it is the only act that performs it. The release digest of the
`galeed.roster/1` core is

    0x9d0ce54d8b74ca3e97f3e2e5c22200f4eea2b2c3143c6770bee9d453320963d0

named on 2026-10-30: the pure Python implementation in
`galeed-conformance/roster-py/` (two source files: `gro.py`, `grolaw.py`;
the language's standard library and the `galeed/1` core of §10.4, nothing
else), whose release digest `galeed-conformance/criterion-digest-roster.sh`
computes over the two files and the `galeed/1` core digest of §10.4; the
corpus, the harness contract, and the review record beside it say how it
converged. What stands beside the digest is a three-round convergence of
three independent implementations (Rust, Node.js, Python), each written from
this text and its harness contract alone, run unchanged in the closing round
at zero divergences over the corpus on six seeds and more than 200,000
black-box probes with zero disagreement, the boundary list of §10.1
witnessed on both sides, and, at the re-pinning of 2026-10-30 after the
`galeed/1` core moved, the same three implementations run unchanged at zero
divergences on six seeds, recorded in the review record. From the naming the
core binds and this text is its textbook; a dispute about what
`galeed.roster/1` says on some input is settled by running the core on it. A
change to either file is a new core and a new digest named by the same act.