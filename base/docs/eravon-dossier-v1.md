# ERAVON dossier v1 (`eravon.dossier/1`)

The dossier law of ERAVON: the third document law, so that a seller's
client, a buyer's client, and any reader handed the export agree byte for
byte on what a case's dossier is and on what can be checked of it with
nothing but the files in hand. Where [`eravon.terms/1`](eravon-terms-v1.md)
says what the order was and [`eravon.link/1`](eravon-link-v1.md) says how it
travelled, `eravon.dossier/1` says how a closed or open case is exported so
that a counterparty, an adviser, or a tax office can recompute every claim
in it against the chain without the exporter present. It consumes the two
laws only through their frozen cores, and reads [`galeed/1`](galeed-v1.md)
§4.3 through an implementation its own core carries (§1, §7.4, §7.5).

**Status.** The law is the set of total predicates in this text. A
conforming implementation embeds an `eravon.link/1` core that agrees on
every input with the core of the release digest §7.4 names and agrees with the core §7.5 of this text names on
every input. Evolution goes through `eravon.dossier/2`.

**Rule classes.** Three classes appear, and every rule below is marked:

- **[D]** document predicates: decided offline from the files of one
  dossier, and nothing else. No predicate of this law consults a chain.
- **[C]** chain procedures: what a reader with chain access recomputes
  from a dossier, stated so that two readers do the same thing, decided by
  no predicate of this law and measured by no corpus.
- **[O]** producer obligations.

---

## 1. Notation

`galeed/1` §1, `eravon.terms/1` §1, and `eravon.link/1` §1 apply. In
addition:

- **`canonical(b)`**: as `eravon.terms/1` §1 defines it, the `_md`
  suffix rule of `galeed/1` §3.3 deciding every string before any table
  below reads it; the keys that are `_md` or end in it a passing index
  carries are
  `note_md`, `buyer_md`, and `seller_md`, `galeed/1` §3.3's suffix rule
  decides every other key of every input alike (a `zz_md` at the root
  being prose and then a member beyond the list), and every string
  outside a prose subtree is a skeleton string.
- **`entry(b)`**: `galeed/1` §4.3 in full over a byte string, decided by the
  implementation of that section this law's core carries among its own files
  (`erdentry.py`, §7.4, §7.5), yielding the entry with its `entryType` and
  `body`, which §3 reads, and its `judge`, which §5.1 reads against the
  chain, or one of that law's tokens.
- **`bindReceipt`**: the predicate of the link law's §4.4, decided by the
  embedded cores; on every whole dossier it yields `RECEIPT_BOUND` or
  `RECEIPT_BY_CALL`, the second where the buyer's slot is empty
  (`eravon.terms/1` §5.4), §3.1 refusing every other verdict. The
  listing's binding to the document it carries is the link law's §2.5
  and needs no call here, and `resolveLink` is called by nothing here, a
  dossier carrying the listing's bytes and not a pointer to them.
- **a dossier** is a directory: a set of named files, one of which is
  named `dossier.json` and is the **index**; every other file the index
  names is read by name from the same directory. A **file name** is a
  non-empty skeleton string carrying no `/` and byte-equal neither to `.`
  nor to `..`. A file this law names or the index names, `dossier.json`
  included, is the directory entry whose name is byte-equal to it, so that
  no host's name folding decides which entry a name the index gives names
  (a symbolic link's target and the directory's own path resolve by the
  host's rules, its folding included), and a name matching no entry is
  unreadable, as is one matching an entry that is neither a regular file
  nor a symbolic link the host's path resolution follows, through further
  links, to a regular file (a dangling link, a loop, and a resolution past
  either of the host's limits, on the links it follows and on the path's
  length, among what it does not follow), and so is an entry the host
  refuses to open for that path's length or for want of permission; both
  limits count over the one path the reader opens: the path by which the
  reader names the directory, as it was handed and unnormalised, then `/`
  unless that path ends in one, then the name; a symbolic link it so
  follows is read as the file it resolves to. A file the index does not name is read by no predicate of
  this law and is no fault, whatever its name; two references may name
  one file, which is then read once per reference and tested against each
  reference's own `sha256`; a reference may name `dossier.json` itself,
  which is then read like any file.
- **a file key** is the string by which §2.6 and §3.1 name a file in a
  fault: `index` for the index file `dossier.json` itself; `listing`,
  `receipt`, and `entry` for the references of those names in `files`, and
  `attachment:` followed by the element's index in `files.attachments`
  spelled as `galeed/1` §3.2 spells an int (`attachment:0`) for an
  attachment.
- **a named candidate**, **a pool candidate**: the receipt's candidate
  where it numbers a judge element of the listing (`eravon.link/1` §2.3),
  and where it numbers a guild element; a named candidate's seat is the
  named track's and a pool candidate's the guild track's
  (`eravon.standard/1` §4).
- **a slot read**: a `sigSeller` or `sigBuyer` slot of a call of the
  standard's `init` or `settle`, or the signature slot of its `cancel`, read
  as `eravon.standard/1` §9 reads one: an empty slot is the empty string; 65
  bytes are the hex65 of those bytes; any other non-empty slot is no
  signature, the standard admitting none (`eravon.standard/1` §9), so that a
  slot and a receipt's `sig` are compared as hex65 strings.
- **the current key of an address at a call**: `currentAt(a, p)` of
  `eravon.link/1` §1 over the `Declared` and `Acceded` logs of the
  succession book `eravon.link/1` §1 names on `chainId`, `p` being the
  position at which the call began, the first position after every log the
  chain carries before it (in earlier blocks, in earlier transactions of its
  block, and earlier in its own transaction): the key the standard compared
  a party's or a judge's address with at that call; an address's root at a
  call is read the same way.

---

## 2. The index

**[D] 2.1 Members, closed.** The index is a canonical object with exactly
these members and no others:

| key | presence | form | rule |
|---|---|---|---|
| `v` | required | string | byte-equal to `eravon.dossier/1` |
| `chainId` | required | int | the instance's chain; equal to the listing's |
| `instance` | required | hex20 | the instance; equal to the one the receipt's binding yields |
| `caseId` | required | hex32 | the case; equal to the salt the receipt's binding yields |
| `logic` | required | hex20 | equal to the listing's |
| `proxyCodeHash` | required | hex32 | equal to the listing's |
| `posted` | required | object | exactly `tx` (hex32) and `hash` (hex32): the transaction that posted the listing on the board and the listing's `sha256`, which the listing file hashes to (§3.1) |
| `files` | required | object | exactly `listing` and `receipt`, each required, `entry`, optional, each a **file reference**: an object with exactly `name` (a file name) and `sha256` (hex32); and `attachments`, optional, an array every element of which is a file reference |
| `seat` | required | object | the seat (§2.2) |
| `transactions` | required | array | every element a **transaction row** (§2.3); may be empty |
| `terminal` | optional | object | the terminal (§2.4): present exactly where the instance emitted `Closed`, at a case's terminal and at the buyer's `cancel` (`Refunded` with cause 4, which opens no case and closes the instance, `eravon.standard/1` §4); absent for an open case, for an instance that holds no case and was not cancelled, and after an expiry branch not followed by a cancellation (`eravon.standard/1` §4 step 3, which emits `Refunded` with cause 2 and no `Closed`) |
| `compliance` | optional | object | the compliance page (§2.5) |

**[D] 2.2 The seat.** An object with exactly `amanahHash` (hex32,
nonzero), `quoteHash` (hex32), `seat` (an int at most 2), and optionally
`judge` (hex20, nonzero) and `entryId` (hex32): the seat the order names
and, where the instance opened one, the last seat it opened for `caseId`
(the `Seated` of the highest ordinal), its judge, the word and quote
pinned in the case, its ordinal, and the entry that seat's judge anchored
by `rule` or `resign`, where it anchored one (a `Resigned` of form 3
anchors none). For a named candidate `judge` is present and is the
candidate, and `seat` is 0, whether or not the seat opened; for a pool
candidate `judge` is whoever the instance drew to that seat, absent where
the instance has seated nobody (the first draw of `eravon.standard/1` §7
armed and not yet made, or never made), and `seat` is the ordinal, 0
where no seat was drawn; where `judge` is absent, `seat` is 0 and
`entryId` is absent (`E_DOSSIER_SEAT`); §3.1's seat-matching
test reads `judge` and `seat` for a named candidate alone, the pool
candidate's being §5.1's to check against the chain, and its subject test
reads `seat` under both forms. `amanahHash` and `quoteHash` are the
candidate's under both forms, the latter made nonzero on every whole
dossier by §3.1's comparison with the candidate's.

**[D] 2.3 A transaction row.** An object with exactly `kind` (a string
byte-equal to one of `post`, `fund`, `deploy`, `init`, `accept`,
`delivery`, `confirm`, `dispute`, `commit`, `settle`, `rule`, `resign`,
`refund`, `expire`, `cancel`, `revoke`, `advance`, `redraw`, `setPayout`,
`withdraw`, `sweep`, `consent`, `fundBind`), `tx` (hex32), `block` (int), `timestamp`
(int), and optionally `note_md` (prose string). Rows are data for readers
and for §5; no predicate here orders them, relates them to each other, or
relates them to the seat or the terminal.

**[D] 2.4 The terminal.** An object with exactly `name` (a string
byte-equal to one of `Refunded`, `Released`, `Settled`, `Ruled`,
`Resigned`, `Lapsed`), `buyerPaid` (dec256), `sellerPaid` (dec256),
`judgePaid` (dec256), `bondToBuyer` (dec256): the terminal the instance
reached and the four amounts its `Closed` event carries, as
`eravon.standard/1` §8 names them. Where `name` is `Ruled`, `seat.entryId` is present
(`E_DOSSIER_RULED`; §2.2 has then made `judge` present). The member is
present exactly where the instance emitted `Closed` (§2.1).

**[D] 2.5 The compliance page.** An object with exactly these members,
each optional, the value an object first, then no member beyond the list,
then the rows below in the order they are written, and within
`conversions` each element in array order and each element's own
members in the order its row writes them: `buyer_md` (prose string), `seller_md` (prose string), the
parties' legal identities in their own words; `serial` (a non-empty
skeleton string), the exporter's sequential number; `conversions`, an
array every element of which is an object with exactly `tx` (hex32), `at`
(int), `unit` (non-empty skeleton string), `rate` (non-empty skeleton
string), `source` (non-empty skeleton string): the fiat value of a
transaction's amount at a moment, in the unit named, at the rate named,
from the source named; a conversion's `tx` is related to no row. Every
conversion names its moment and its source so that a reader can fetch the
same rate; the rate itself is the exporter's word.

**[D] 2.6 Order of decision.** The directory carries a readable file
named `dossier.json`, else `E_DOSSIER_FILE` carrying the key `index` and
no test below is reached. Over its bytes: `canonical(b)` (its tokens);
root an object (`E_DOSSIER`); the ten required members present
(`E_DOSSIER_MISSING`), then no member beyond the twelve
(`E_DOSSIER_CLOSED`); `v` (`E_DOSSIER_V`); `chainId` (`E_DOSSIER_CHAIN`);
`instance` (`E_DOSSIER_INSTANCE`); `caseId` (`E_DOSSIER_CASE`); `logic`
(`E_DOSSIER_LOGIC`); `proxyCodeHash` (`E_DOSSIER_CODEHASH`); `posted` an
object of exactly `tx` and `hash`, each a hex32 (`E_DOSSIER_POSTED`);
`files` an object with exactly its members, each a file reference of its
form and `attachments`, where present, an array of file references
(`E_DOSSIER_FILES`); `seat` an object of §2.2's form (`E_DOSSIER_SEAT`);
`transactions` an array (`E_DOSSIER_TRANSACTIONS`), then each element in
order under §2.3, the first faulting element naming `E_TRANSACTION` with
its index; `terminal`, where present, an object of §2.4's form
(`E_DOSSIER_TERMINAL`), then its `Ruled` rule (`E_DOSSIER_RULED`);
`compliance`, where present, an object of §2.5's form
(`E_DOSSIER_COMPLIANCE`, carrying the index of the first faulting element
of `conversions`, the element's own form counting as that element's
fault, and carrying nothing where the fault is anywhere else in
`compliance`, `conversions` not being an array among them). An index that
passes is an index; nothing else is.
§3 then reads the files.

---

## 3. The files

**[D] 3.1 Order of decision.** After §2.6, over the directory: for each of
`listing`, `receipt`, `entry` in that order where the reference is present,
and then for each attachment in array order, the named file is readable
(`E_DOSSIER_FILE`, carrying the key) and then its `sha256` is byte-equal to
the reference's (`E_DOSSIER_DIGEST`, carrying the key), both tests of one
key decided before the next key is read and every key the index names before
any document test; the listing file passes `eravon.link/1` §2.5
(`E_DOSSIER_LISTING`, carrying the token); its `sha256` is byte-equal to
`posted.hash` (`E_DOSSIER_HASH`); the receipt file passes `eravon.link/1`
§4.2 (`E_DOSSIER_RECEIPT`, carrying the token); `bindReceipt(listing,
receipt)` is `RECEIPT_BOUND` or `RECEIPT_BY_CALL` (`E_DOSSIER_ORDER`,
carrying any other verdict); the index's `chainId`, `logic`, `proxyCodeHash`
equal the listing's (`E_DOSSIER_CHAIN_MISMATCH`, `E_DOSSIER_LOGIC_MISMATCH`,
`E_DOSSIER_CODEHASH_MISMATCH`); `instance` and `caseId` equal the ones the
receipt's binding yielded, under either verdict
(`E_DOSSIER_INSTANCE_MISMATCH`, `E_DOSSIER_CASE_MISMATCH`); the seat matches
the receipt's candidate: `amanahHash` and `quoteHash` equal the candidate's,
and for a named candidate `judge` is present and equals the candidate and
`seat` is 0 (`E_DOSSIER_SEAT_MISMATCH`); `entryId` is present iff
`files.entry` is (`E_DOSSIER_ENTRY`); where present, `sha256` of the entry
file is byte-equal to `entryId` (`E_DOSSIER_ENTRY_DIGEST`), the entry file
passes `entry(b)` (`E_DOSSIER_ENTRY_FORM`, carrying the token), its
`entryType` is `judgment` or `resignation` (`E_DOSSIER_ENTRY_TYPE`), and its
body's `subjectRef` is of kind `evm` with `chainId`, `contract`, `codeHash`,
`id` equal to the index's `chainId`, `instance`, `proxyCodeHash`, `caseId`,
and with a member `seat` that is an `int` equal to the index's `seat.seat`,
present where the entry is a judgment (`galeed.venue/1` §2.1 puts the
ordinal on judgments) and, where it is a resignation, either absent or so
(`E_DOSSIER_SUBJECT`); for each attachment in array order, its reference's
`sha256` is byte-equal to the `goodsHash` or to the `deliveryHash` the
listing's document carries (`E_DOSSIER_ATTACHMENT`, carrying the index: an
attachment is the text of a segment the document carries as a hash,
`eravon.link/1` §5.3, and a text the document names in no such member is no
attachment of this case). A dossier that passes is whole; the answer carries
`termsHash` (the listing's `terms.hash`), `instance`, `caseId`, `candidate`
(the receipt's `candidate`, numbered as `eravon.link/1` §2.3 numbers them),
`entryId` or `null`, `attachments` (the number of attachments, 0 where there
are none), and the two consent forms: `buyerConsent`, `secp256k1` where the
receipt's `sig` is a hex65 string the link law verified and `call` where it
is the empty string; and `sellerConsent`, `call` where the candidate the
receipt names is a by-call candidate of the link law's §2.5 and `secp256k1`
where that candidate carries a signature the link law verified. Both are
decided by the files alone. The two documents having passed their own
sections first, `E_DOSSIER_ORDER` never carries `LISTING_INVALID` or
`RECEIPT_INVALID`.

**3.2 Meaning.** A whole dossier proves, with no chain, that the listing,
the document inside it, the receipt, the attached texts, and the address
belong to one case, which of each party's consents was a signature the
reader has verified and which a claim by call that §5.1 reads at the
chain, and that the entry file, where there is one, is a judgment or a
resignation whose `subjectRef` names this case, this instance, this code
hash, and, on a judgment and on a resignation that carries one, this seat
number; whose key signed it and
which word it was rendered under are §5.1's. What the chain adds is §5's:
that the listing was posted, that the transactions happened, that the
terminal is the one the instance reached, that the entry's bytes are the
ones the instance anchored, that its `judge` is the seat's judge or a key
of that judge's lineage, and that its `amanah` resolves to the word the
seat froze.

---

## 4. Tokens and vocabularies

**Index tokens** (closed; by §2.6): the `galeed/1` §3 tokens;
`E_DOSSIER`, `E_DOSSIER_MISSING`, `E_DOSSIER_CLOSED`, `E_DOSSIER_V`,
`E_DOSSIER_CHAIN`, `E_DOSSIER_INSTANCE`, `E_DOSSIER_CASE`,
`E_DOSSIER_LOGIC`, `E_DOSSIER_CODEHASH`, `E_DOSSIER_POSTED`,
`E_DOSSIER_FILES`, `E_DOSSIER_SEAT`, `E_DOSSIER_TRANSACTIONS`,
`E_TRANSACTION` (carrying an index), `E_DOSSIER_TERMINAL`,
`E_DOSSIER_RULED`, `E_DOSSIER_COMPLIANCE` (carrying an index where the
fault is a conversion element's).

**File tokens** (closed; by §2.6 and §3.1): `E_DOSSIER_FILE` (carrying a
file key, `index` among them) and `E_DOSSIER_DIGEST` (carrying a file key
other than `index`); `E_DOSSIER_LISTING` (carrying a listing token of
`eravon.link/1` §9), `E_DOSSIER_RECEIPT` (carrying a receipt token of that
section), `E_DOSSIER_ENTRY_FORM` (carrying a token of `galeed/1` §10, which
carries nothing, that law's `E_JUDGE` and `E_SIG_FORM` among them);
`E_DOSSIER_HASH`; `E_DOSSIER_ORDER` (carrying a verdict of `eravon.link/1`
§4.4 other than `RECEIPT_BOUND`, `RECEIPT_BY_CALL`, `LISTING_INVALID`, and
`RECEIPT_INVALID`, with what it carries); `E_DOSSIER_CHAIN_MISMATCH`, `E_DOSSIER_LOGIC_MISMATCH`,
`E_DOSSIER_CODEHASH_MISMATCH`, `E_DOSSIER_INSTANCE_MISMATCH`,
`E_DOSSIER_CASE_MISMATCH`, `E_DOSSIER_SEAT_MISMATCH`, `E_DOSSIER_ENTRY`,
`E_DOSSIER_ENTRY_DIGEST`, `E_DOSSIER_ENTRY_TYPE`, `E_DOSSIER_SUBJECT`;
`E_DOSSIER_ATTACHMENT` (carrying an index).

**Answer values** (closed; by §3.1): `buyerConsent` one of `secp256k1`,
`call`; `sellerConsent` one of `secp256k1`, `call`.

**Literals this law fixes**: inside the index, the version literal
`eravon.dossier/1`, the twenty-three transaction kinds of §2.3, and the six
terminal names of §2.4; outside it, the index file name `dossier.json` and
the file keys of §1 with their prefix `attachment:`.

---

## 5. What the chain adds

**[C] 5.1** A reader with chain access, holding a whole dossier,
recomputes, and says which comparison failed where one does:

- that the transaction `posted.tx` on `chainId` carries a `Posted(poster,
  key, hash, document)` log emitted by an account whose code hash is the
  standard's `boardCodeHash` (`eravon.standard/1`, Shape and §7b;
  CODEHASH.md), whatever its `poster`, with `key` the listing's `terms.hash`, `hash` equal to
  `posted.hash`, and `document` byte-equal to the listing file;
- that the account at `instance` on `chainId` carries code whose
  `keccak256` is `proxyCodeHash`, where the dossier carries a `deploy`
  row or a `terminal`; where it carries neither, the reader reads the
  account and reports what it found, an account with no code being the
  funded, undeployed case and no fault;
- that each transaction row's `tx` is a transaction included in block
  `block` whose header carries `timestamp`, and that the row's `kind` is one
  of the things the transaction did to the instance, or, for `post`, to the
  board, or, for `consent`, `revoke`, and the `Pulled` of an `init`, to the
  logic on this case's behalf, or, for `fundBind`, to the logic for the pair
  the order names, by this table, each row decided alone, so that rows
  naming one transaction and one kind are witnessed alike and one
  transaction may be the `tx` of rows of several kinds, as many as the table
  gives it: `post`, the `Posted` the first bullet reads, so that `posted.tx`
  is this row's `tx`; `fund`, a `Transfer(from, to, value)` log emitted by
  the account at the listing's `token` whose `to` is the instance, whatever
  its `value` and its `from`; `deploy`, the transaction that first leaves
  code at the account, an inner call through the public deployer or a
  Multicall included, so that one transaction may be both the `deploy` and
  the `init` row; `init`, `Opened` (with `DrawArmed` on the guild track),
  and, where P + `lf` is nonzero, in the same transaction `Pulled(sellerKey,
  sellerFund, token, instance, amount)` emitted by the account at `logic`
  with the listing's `order.sellerKey` and `order.sellerFund`, its `token`,
  this instance, and `amount` the sum P + `lf` of `eravon.standard/1` §3
  (the event of the one `transferFrom` from `sellerFund` to the instance
  that `pull` runs, `eravon.standard/1` §7a; the token's own log is the
  `fund` row's and no part of this comparison), P and `lf` as the quote
  bytes whose `keccak256` is the seat's `quoteHash` fix them for the order's
  `amount` (the bytes read from the call's `quote` argument where the call
  is read, or from the carrier's `quotes` under the key of
  `eravon.standard/1` §3; a reader holding neither reports the `Pulled`
  comparison as not recomputed, no fault, and compares `Opened` and
  `DrawArmed` all the same), `lf` being the `Priced` event's where the
  quote's `feeUnit` is nonzero; `accept`, no event, a successful call of the
  venue's `accept(caseId, seat)` to the instance from the current key of the
  seat's `judge` at that call (§1); `delivery`, `DeliveryClaimed`;
  `confirm`, `Confirmed`; `dispute`, `Disputed`; `commit`, `Committed`;
  `settle`, `Settled`; `rule`, `Ruled`; `resign`, a `Resigned` of form 0, 1
  or 2 with its `Anchored` (a `Resigned` of form 3 is the closing of
  `eravon.standard/1` §7, emitted by whichever transaction closes the case,
  and no row of its own); `refund`, `Refunded` with cause 3; `expire`,
  `Refunded` with cause 2 (the offer's `expiry` or the order's `openBy`
  passed, or the document revoked, before any case opened; which of the
  three the reader tells by the block's timestamp against the two deadlines
  and by the logic's `Revoked`); `cancel`, `Refunded` with cause 4 and
  `Closed` with the terminal `Refunded`, emitted by a successful call of
  `cancel` to the instance whose caller, read as the bullet on `init` below
  reads one, is the current key of the receipt's `buyerKey` at that call
  (§1), or whose slot, read as §1 reads one, carries a signature by that
  current key over `Cancel()` under the instance's domain (`eravon.terms/1`
  §4.5); `revoke`, `Revoked(sellerKey, termsHash)` emitted by the account at
  `logic` with the listing's `order.sellerKey` and `terms.hash`; `advance`,
  a successful call to the instance of the venue's `advance`, whatever
  arguments it carries (another `caseId`, an earlier seat of a guild case,
  or a seat never opened included) and whatever it emitted: `Closed`, after
  whatever the terminal it reached emits before it, where it ended the case;
  `DrawArmed` alone where it ran out an unaccepted guild seat's offer clock
  and armed the next draw (`eravon.standard/1` §7); or nothing where it
  changed nothing; `redraw`, a kept successful call of `redraw(members,
  amanahHashes)` (`eravon.standard/1` §7) to the instance, read as the calls
  below are, which emits `Seated` for the drawn ordinal, 0 included (on the
  guild track every seat is opened by `redraw`, `init` arming the draw and
  emitting `DrawArmed`, `eravon.standard/1` §7; the named track's seat 0 is
  opened by `init`), or, where the call found no usable panel (the armed
  draw's block hash no longer served, or the frozen router failing as
  `eravon.standard/1` §7 lists), `Resigned` with form 3 and `Closed` with
  the terminal `Resigned` (`eravon.standard/1` §7); `setPayout`, the venue's
  call of that name whose `key` is the seat's `judge` or the judge of any
  seat the instance opened for `caseId`, at any ordinal, and whose caller,
  read as the bullet on `init` below reads one, is that `key`'s current key
  at the call (§1), which emits nothing; `withdraw`, the venue's call of
  that name with such a `key` from such a caller, which emits `Swept` where
  it pays and nothing where the seat had nothing claimable; `sweep`, a
  `Swept` of the instance in the row's transaction, which recomputes the row
  by itself, or, where the transaction carries none, a kept successful call
  of `sweep` to the instance, read as the calls below are, which emits
  `Swept` for each push that delivered and nothing otherwise; `consent`,
  `Consented(address indexed who, bytes32 indexed digest)` emitted by the
  account at `logic` (the registry of `eravon.terms/1` §8, which lives on
  the logic and not on the instance; the event as `eravon.standard/1` §10
  lists it) under one of three pairs: `who` the listing's `order.sellerKey`
  and `digest` the digest of `eravon.link/1` §2.4 for a candidate of the
  listing; `who` the receipt's `buyerKey` and `digest` the `orderDigest`
  `bindReceipt` yields (`eravon.link/1` §4.4); or `who` either of the two
  and `digest` the `d` the bullet on `settle` below forms from the two
  arguments of the call of `settle` that emitted the instance's `Settled`; a
  `Consented` under either key whose digest none of the three pairs names
  the reader reports as not recomputed, which is no fault, and one whose
  digest a pair names under the other key alone is under no pair, a failed
  comparison; `fundBind`, `FundBound(sellerKey, sellerFund)` (the event as
  `eravon.standard/1` §10 lists it) emitted by the account at `logic` with
  the listing's `order.sellerKey` and `order.sellerFund`: where the instance
  emitted `Opened`, before it in chain order (in an earlier block, or in the
  same block at a lower log index than that `Opened`); where it emitted
  none, wherever it stands up to the head the reader read, no gate having
  read the bond; a row §6 owes no dossier, the bond serving every case whose
  order names that pair, and where `order.sellerFund` is `order.sellerKey`
  the gate reads no bond (`eravon.standard/1` §4 step 5), a row naming
  `FundBound(sellerKey, sellerKey)` being witnessed all the same, under the
  same two branches; a call this table names for an `accept`, `advance`,
  `cancel`, `redraw`, `sweep`, `setPayout` or `withdraw` row counting only
  where it is a call to the account at `instance` that ran there and that
  the transaction kept, as the next bullet reads the call of `init` (the
  proxy's delegation to the logic a frame beneath it from the instance;
  returned without failing beneath no frame that failed), and a `deploy`
  row's creation only where the transaction kept it; and where the
  transaction succeeded, the call or creation by which this table would
  witness a `deploy`, `accept`, `advance`, `cancel`, `redraw`, `sweep`,
  `setPayout` or `withdraw` row is not its own call (a `deploy` row's
  creation never is, the deployer making it), and the reader's endpoints
  serve no call trace of that transaction, that call or creation is unread,
  whatever the transaction's input would decode to, and the reader reports
  the row as not recomputed, which is no fault;
- where the instance emitted `Opened`, which the reader reads whatever rows
  the dossier carries, for each party whose slot in the call of `init` that
  emitted it (`sigSeller`, `sigBuyer`, read from the call's input as the
  bullet on `settle` reads its arguments) is empty: that the party's current
  key at that call (§1) was the caller of the call of `init` that emitted
  that `Opened` (the call of `init` being the call in that transaction whose
  target is the account at `instance` and which ran there, the delegation by
  which that proxy runs it on the logic's code being a frame beneath it from
  the instance, whose input calls `init`, and which returned without failing
  beneath no frame that failed, a revert and an exceptional halt being alike
  a failure; a call to that address made before it carried code, and a
  `DELEGATECALL` or `CALLCODE` into its code from another account, whose
  delegation beneath runs from that account, being no such call; a
  transaction whose receipt carries that `Opened` holding exactly one such
  call, found so from a trace that records logs and from one that records
  none alike; and never that delegation itself, which holds the instance's
  logs where the trace records logs; the caller being that call's
  `msg.sender`: the transaction's sender where the call was the
  transaction's own call, and otherwise the `from` of the call's own frame
  in the transaction's call trace), or that the account at `logic` emitted
  `Consented(party, d)` before that call in chain order (in an earlier
  block, or in the same block at a lower log index than that `Opened`); the
  party and `d` being the receipt's `buyerKey` and the `orderDigest`
  `bindReceipt` yields for `buyerConsent`, and the listing's
  `order.sellerKey` and the digest of `eravon.link/1` §2.4 for the receipt's
  candidate for `sellerConsent`; for a party whose slot carries bytes, that
  the slot, read as §1 reads one, verifies as a signature by that party's
  current key at that call (§1) over `d` under `eravon.terms/1` §5.1 through
  §5.3, whatever form the answer carries (a signature in the files §3.1
  verified, the chain adding for it only that the key the files name was
  that party's current key at the call where the slot so read is the
  receipt's `sig`, the buyer's, or the receipt's candidate's `sig`, the
  seller's); where the instance emitted no `Opened`, no consent was
  exercised at the chain and the reader reads nothing for either, no fault;
  and where the listing's `order.buyerMayOpen` is `false`, that the caller
  of that call of `init` was the current key of the listing's
  `order.sellerKey` at that call (§1; `eravon.standard/1` §4), read as above
  and reported as not recomputed where the call was inner and no trace is
  served;
- where the instance emitted `Settled`, which the reader reads whatever rows
  the dossier carries and whatever forms the answer carries: from the call
  of `settle` that emitted it, found as the bullet above finds the call of
  `init`, its arguments read from the transaction's input or, where the call
  was inner, from the call trace (`Settled` carries `buyerBps` and no
  `expiry`), `d` being the EIP-712 digest of `Settle(buyerBps, expiry)`
  under the instance's domain (`eravon.terms/1` §4.2 and §4.5) formed over
  the call's two arguments as it carried them, `buyerBps` the `uint16` and
  `expiry` the `uint256`, an `expiry` above 2^53 − 1 included
  (`eravon.terms/1` §4.5's narrowing binds documents and producers, never
  this reading): for each of its `sigBuyer` and `sigSeller` that is empty,
  that the call's caller, read as the bullet above reads one, was that
  party's current key at that call (§1; the party being the receipt's
  `buyerKey` or the listing's `order.sellerKey`), or that the account at
  `logic` emitted `Consented(party, d)` before that call in chain order (in
  an earlier block, or in the same block at a lower log index than that
  `Settled`); and for each of the two that carries bytes, that the slot,
  read as §1 reads one, verifies as a signature by that party's current key
  at that call (§1) over `d` under `eravon.terms/1` §5.1 through §5.3;
- where the call of `init` or of `settle` that one of the two bullets above
  reads was not the transaction's own call and the reader's endpoints serve
  no call trace of that transaction, that call's caller and arguments are
  unread, and the reader reads the account at `logic`'s `Consented` alone:
  for `init`, for each party whose form in the answer is `call` (the
  answer's form standing in for the unread slot), a `Consented(party, d)`,
  `d` as the bullet on `init` pairs it, emitted before that call in chain
  order as that bullet orders one, passes the party's consent, and where
  there is none the reader reports that party's consent at `init` as not
  recomputed, and for each party whose form is `secp256k1` it reports as not
  recomputed whether the key the files name was that party's current key at
  that call; for `settle`, the reader forms no digest, whatever it holds
  beside the dossier (`Settled` carries `buyerBps` and no `expiry`, no file
  of a dossier is a settlement offer, and an offer held apart may share
  `buyerBps` with the executed one under another `expiry`), and reports each
  party's consent at that call as not recomputed; neither report is a fault:
  the standard emits `Opened` or `Settled` only after admitting every slot
  of that call, and whether `logic` is the standard is a question the code
  at `instance` does not answer, every instance's code being `proxyCodeHash`
  of the `logic` it was derived from;
- where the instance emitted `Opened` and the seat carries `judge`, that
  the instance emitted `Seated` for `(caseId, seat)` with the seat's
  `judge` as its first indexed topic, and that `seatOf(caseId, seat)`
  reports that judge, the seat's `amanahHash`, and its `quoteHash`; a seat
  without `judge` (a pool case whose first draw was armed and not yet
  made, or never made) has no `Seated` to compare here, the next bullet
  reading what the chain holds;
- what the chain holds beyond the dossier: a `Seated` for `caseId` of an
  ordinal above `seat` (any `Seated` for `caseId` where the seat carries no
  `judge`), an `Anchored` of the instance from the current key of the seat's
  `judge` at its call (§1) where `entryId` is absent, and a `Closed` where
  `terminal` is absent; each is a failed comparison where the transaction
  that emitted it is the `tx` of a row of the dossier, `posted.tx`, or the
  transaction whose `Anchored` the entry bullet below reads, and is
  otherwise reported as what the chain holds beyond the export, no fault,
  which §6's completeness then reads;
- for a pool candidate, where the instance emitted `Opened`, that the
  instance emitted `PanelFrozen` whose `guild` is the candidate's pointer
  address, whose `root` is that address's root on the succession book at
  that call of `init` (§1, `galeed.roster/1` §4.2), whose `panelHash` equals
  the `panelHash` of the listing's guild element the receipt's `candidate`
  names where `index` equals that element's `seq`, and, where the panel
  book's record at that `seq` under `(root, proxyCodeHash)`, `root` the one
  `PanelFrozen` names, was recorded in a block before that `Opened`'s and is
  unexpired at it, whose `index` is at least that `seq` (an expired record
  gives way to an older one and a record made later never reaches the case,
  `eravon.standard/1` §7; the receipt's `seq` is compared with nothing,
  `eravon.link/1` §4.4);
- where the dossier carries `terminal`, that its four amounts equal the
  instance's `Closed` event and that its `name` is the terminal the
  event's `terminal` byte numbers, `Refunded` 0, `Released` 1, `Settled` 2, `Ruled` 3, `Resigned`
  4, `Lapsed` 5, as `eravon.standard/1` §5 fixes;
- where `entryId` is present, that the instance emitted the `Anchored` of
  `galeed/1` §9.1, which `galeed.venue/1` §2.9 and §2.10 fix, with `entryId`
  as its second indexed topic and its data the ABI encoding of one `bytes`
  value whose bytes are the entry file's, from the current key of the seat's
  `judge` at that call (§1) as sender, the address `galeed.venue/1` §2.9
  admits; that the entry's `judge` is the seat's `judge` or a key of the
  lineage `galeed/1` §7.4 computes from that judge's ledger with the root
  and under the basis the next bullet fixes; that the entry's `amanah` names
  an `amanah` entry of that ledger whose word the seat's `amanahHash` is: on
  the named track the entry's `amanah` is byte-equal to the seat's
  `amanahHash`, that being the `entry_id` of the covenant entry the judge's
  document offers (`galeed.venue/1` §2.4), and that entry stands in the
  ledger the audit read; on the pool track the entry's `amanah` names an
  `amanah` entry of that ledger the `sha256` of the UTF-8 encoding of whose
  `text_md`, as the reader decodes it, scalar value for scalar value and
  never the entry's own bytes, is the seat's `amanahHash`, as
  `galeed.roster/1` §3b.3 and §7 fix it; and, where the terminal is `Ruled`,
  that the entry's `decision.rung` equals the `Ruled` event's `rung`, and
  where it is `Resigned`, that the entry is a resignation;
- the root under which the reader audits the judge's ledger, and the basis
  under which it scans the anchor set that is the audit's second input
  (`galeed/1` §8, §9.4): the reader decides `resolveJudge(judge)`
  (`galeed.roster/1` §5.8, `judge` the seat's) over the logs of the
  registries it declares and of the succession book §1 reads on `chainId`
  (`galeed.roster/1` §1, §9), named beside its report, from block 0 to the
  head it read; where the `judgeOf(J)` of its answer is `JUDGE` and the
  lineage the audit computes under the document's `ledger.root` holds the
  seat's `judge`, the root is that `ledger.root`; otherwise, for a named
  candidate, the root is the first key of the record (`galeed.roster/1`
  §5.3) of the seat's `judge`, and for a pool candidate, the first key of
  the record of the first `judges` row, in document order, of
  `resolveGuild(g)` (`galeed.roster/1` §5.4, `g` the candidate's pointer
  address) whose record holds the seat's `judge` among its keys, the keys
  before the row's `listed` included; where no such row exists, the reader
  reports the audit as not recomputed, which is no fault of the dossier. The
  basis carries, for `chainId`, one `chains` object from `fromBlock` 0 to
  the head the reader read, its `registries` the addresses it declared and
  the class item `{codeHash: proxyCodeHash}` of §6, and its `senders` the
  root, the final address and every address of the trail
  `resolveIdentity(root)` gives (`galeed.roster/1` §5.3), every key of the
  record of the seat's `judge` and of the root's record, the seat's `judge`,
  and every `to` of every carried byte string the scan's records carry that
  passes `galeed/1` §4.3, is a `succession`, and whose `judge` is already in
  `senders`, the reader adding each such `to` and scanning again until none
  is added, so that on `chainId` the basis reaches every anchor of
  `galeed/1` §9.1's form by which the judge published its entries through
  the registries the reader declared (`galeed/1` §5.7, §9.1) and every
  ruling an instance of this standard anchored for it (the whole basis
  covering under `galeed.papers/1` §9.2 where the document's other objects
  do); where the root is a document's `ledger.root`, that document's
  `ledger.basis` objects for other chains stand as written, and its objects
  for `chainId` give way to that one object, their `registries` and
  `senders` added to its own. Where the lineage under the root so chosen
  does not hold the seat's `judge`, or the audit's input is invalid
  (`NO_LABEL`), the reader reports the audit as not recomputed, no fault of
  the dossier; under every other label the comparisons above read the ledger
  the report's view holds (`galeed.papers/1` §9.0), the label reported
  beside them; a comparison of the bullet above that needs an entry the
  audit under that basis does not hold the reader reports as not recomputed,
  naming the basis, and a comparison over an entry the audit holds that
  finds it different is a failed comparison.

**[C] 5.2** What the reader cannot recompute is the exporter's word:
`note_md` on a row, the compliance page's identities and its `serial`, and
every conversion's rate, which the reader refetches from the named source at the
named moment if it wants a second opinion. Whether a `Consented` that
§5.1 reports as not recomputed concerns this case is likewise the
exporter's word.

**[O] 5.3** An exporter writes every row from the chain and never from
memory, writes the terminal from the `Closed` event and never from its own
accounting, writes the listing file from the board's `Posted` log and never
from a copy, and writes the entry file from the `Anchored` event's data and
never from the judge's ledger, so that §5.1's comparisons hold for a dossier
it exports; a row it cannot source it omits. Where the exporter holds no
receipt that binds under `eravon.link/1` §4.4 (the buyer opened the case
itself) and the `init` call that opened the case carries an offer that is
`listingOffer(S, c)` for a candidate `c` of the listing, it writes the
receipt file from that call's data: `v` the literal `eravon.receipt/1`,
`termsHash` the offer's, `candidate` the `c` of that condition, the one
candidate whose `listingOffer(S, c)` the call carries (a judge element where
the offer's `judge` is nonzero, a guild element where its `guild` is),
`buyerKey`, `buyerPayout`, `openBy` and `prev` from the `OrderTerms`, `sig`
from `sigBuyer` read as §1 reads a slot where it verifies under `buyerKey`
(`eravon.terms/1` §5.2, §5.3) and the empty string otherwise (a signature by
the key the buyer's record moved to, which the standard admits,
`eravon.standard/1` §9), and `seq`, for a pool candidate, the frozen panel's
index, so that the receipt binds under `eravon.link/1` §4.4 as the buyer's
own would have; where no such call exists (a cancelled instance, an expiry
branch, an offer the listing does not carry), the case has no whole dossier
under this law, and the exporter says so.

---

## 6. Outside the grammar

- **Completeness.** Whether the rows are every transaction the instance
  saw: the chain; the reader lists every transaction carrying an event of
  the instance, a `Transfer` of the listing's `token` to the instance (the
  buyer's payment, a stranger's included), the board's `Posted` at
  `posted.tx`, or a log of the logic under this case (every `Consented` under one
  of three pairs, §5.1's `consent` row's with its first narrowed to the
  receipt's candidate: that candidate's offer digest under `sellerKey`,
  the `orderDigest` under `buyerKey`, the executed `settle`'s digest under
  either key, wherever it stands in chain order and whatever the slot of
  the call carried; where the instance emitted `Settled` and the call of
  `settle` that emitted it is unread, §5.1 forming no settle digest, every
  `Consented` under either key whose digest is neither
  of the other two is reported, for completeness, as not recomputed, no
  fault; the `Pulled` naming the instance, and never a `FundBound`, which
  serves every case whose order names that `sellerKey` and that
  `sellerFund` and is read by §5.1 only where a `fundBind` row names it; a `Revoked(sellerKey,
  termsHash)` in a block at or before an expiry branch the instance took),
  and the dossier is complete where each such transaction is the `tx` of
  at least one row; further rows the table of §5.1 gives one transaction
  (a `delivery` beside its `commit`, a `sweep` beside the terminal that
  pushed, a `fund` beside the `init` that pulled the deposit or the
  `dispute` that pulled the fee) are the exporter's to write or omit, and
  the logic's other logs under either key, which may serve other cases
  (another candidate's registration, a revocation after the case closed),
  reposts of the listing under its key, and a settlement offer posted
  under `caseId` are no row this dossier owes.
- **The terminal and the entry.** Whether a `Ruled` case carries a
  judgment and a `Resigned` case whose seat anchored an entry a
  resignation (a `Resigned` of form 3 anchors none): the chain (§5.1); this
  law tests that the entry is one of the two types and that it names this
  case and this seat.
- **Money.** What the amounts are worth, and to whom the fiat value
  matters: the parties and their authorities.
- **The judge's ledger.** Whether the entry is anchored in the judge's
  ledger and what the ledger's audit says: `galeed/1` §8, run by whoever
  reads the judge under the root and the basis §5.1's last bullet fixes, the
  class item `{codeHash: proxyCodeHash}` such a reader adds to the
  `registries` of the `chains` object for the index's `chainId`, as
  `galeed/1` §9.4 spells one, being what counts this case's anchor, the
  `senders` §5.1's last bullet builds being what reaches the judge's own
  entries, and `fromBlock` 0 being what makes the object for `chainId` reach
  the case's first block; a key's retirement is read from the records of
  every chain the basis covers (`galeed/1` §7.4), so that a basis a reader
  builds for `chainId` alone reads none recorded elsewhere, and the label it
  reports says what that basis covers.
- **The parties' identities.** Who stands behind `order.sellerKey` and the
  receipt's `buyerKey`, and what their records have done: `eravon.link/1`
  §6, which a reader decides over the chain's records at the head it read; a
  dossier records none of it, since what it would record is the exporter's
  reading of a chain that moves on.
- **Storage.** Where a dossier is kept and for how long: the parties; the
  hash of every file the index names is in the index so that a copy can be
  checked, and the index's own bytes are checked by recomputing §2 and §3
  over them.

---

## 7. Freeze and evolution

**7.1** The reference implementation freezes when independently written
implementations, produced from this text, the two document laws, `galeed/1`
§1 and §3 through §6, and the four harness contracts alone, agree with it on
every input of a generated corpus (every index token of §4 with its index,
every file token of §4, the two that carry a key with each key §4 names for
them, an attachment key among them, and every whole dossier's answer, the
two consent forms of §3.1 in each of their values, the attachment count at
0, 1, and 2, and a first stage and a later stage (the receipt's `prev` zero
and nonzero, `eravon.terms/1` §4.4), each under a named candidate, under a
pool candidate, with a signed receipt and with the buyer's consent by call
(each condition once, not their product), included), with every closed
boundary of this text witnessed on both sides by at least one corpus input
under every seed the corpus is generated with, the boundary list being a
precondition on the corpus and never a property claimed of it afterwards;
the corpus carries the list beside itself. The carriers the precondition
takes in pairs are `E_DOSSIER_FILE` and `E_DOSSIER_DIGEST` (each with every
key §4 names for it, an attachment key once) and `E_DOSSIER_COMPLIANCE`
(once with an index and once without); `E_TRANSACTION` and
`E_DOSSIER_ATTACHMENT` are witnessed with an index; `E_DOSSIER_ENTRY_FORM`
is taken with every input of the `galeed/1` corpus's `check` set
(`galeed-conformance/corpus/canon/`, as generated at seed 1 and committed)
that passes `galeed/1` §3.5, each carried as the entry file of an otherwise
whole dossier and predicted as that corpus predicts it (a token of §4.3
carried; an accepted entry going on to §3.1's type and subject tests), so
that every boundary of §4.3 and of the §6 tables that corpus witnesses, the
order of the steps and the rows of the tables among them, is witnessed here,
with every adjacent pair of §4.3's tests that set leaves unwitnessed by an
entry failing both, the boundary list naming them, and once with a token of
§3.5, the step-1 test the embedded `eravon.terms/1` core decides, the
decision being this core's own (§1, §7.4); every other carrying token of §4
carries another document's whole vocabulary and is witnessed once, carrying
whatever it carried there. The verdicts §3.1 names as unreachable state no
boundary in that sense. No `[C]` or `[O]` rule is in the corpus and the
freeze certifies none. The fuzzing `galeed/1` §12.1 asks of its core is
deferred here as `eravon.link/1` §11.1 defers it; this law adds no byte scan
of its own beyond §1's file-name test, and §7.5 records beside every digest
it names whether that fuzzing was run over §1's file names and the entry
test its core carries (§7.4), and with what result, a naming that records
none being a naming under deferral. §5.1's `Anchored` is the form of
`galeed/1` §9.1, which `galeed.venue/1` §2.9 and §2.10 emit, and §6's class
item is a `registries` element of `galeed/1` §9.4; both stand in the texts
of `galeed/1` and `galeed.venue/1`, and nothing of this law waits on an
amendment.

**7.2** Once §7.5 names a release digest, agreement with the frozen core
on the corpus is the test anyone can run in seconds, and it is evidence
of conformance and never its definition.

**7.3** `eravon.dossier/2` is a new version literal; a dossier accepted
under this law is accepted under it forever, and a later law reads it by
this one.

**7.4** This law, the two document laws, and `galeed/1` are four laws: a
change to one is never a change to another, and each names its own core;
`galeed.venue/1`, which §3.1 and §5.1 cite for the seat's facts, names an
interface identity and no core. This law reads every passage of a GALEED law
it cites as it stands when §7.5 names this law's core, `galeed/1` §1, §4.1
through §4.3 with every passage of §5 and §6 that §4.3 invokes (§5.1 through
§5.3, §5.6 and §6), and §10's tokens among them, and reads §3, §5.4 and §5.5
through the `eravon.terms/1` core it embeds, as that law's §10.4 pins them;
its core carries its own implementation of §4.3 and the tables it invokes in
`erdentry.py`, counted in its own digest and naming no `galeed/1` core, and
a later text of any such passage, revised in place under `galeed/1` §12.5 or
published as a `galeed/2`, changes nothing here until this law's own
revision follows it. This law's core embeds the `eravon.link/1` core of release digest

`0x25eb3810d9a870633608d36054a8399e1445435caa703c7ed5e8fd1bae8f11dc`

(the one that law's §11.5 named when this section was pinned), and
through it the `eravon.terms/1` core that law names; a dossier core built on any other link core is another core.
A recast of `eravon.link/1` changes nothing here by itself: this law moves
to another link core only by its own §7.5 act, which re-pins this section
and names the dossier core built on it in one act.

**7.5 The core's identity.** Naming a release digest here is the freeze, and it is the only act that performs it. The release digest of the
reference implementation in `eravon-conformance/dossier-py/` (`erd.py`,
`erdlaw.py` and `erdentry.py`, the last carrying `galeed/1` §4.3 (§7.4);
the language's standard library and the two cores, nothing else), as
`criterion-digest-dossier.sh` computes it, is

`0x93ae4df7f2058a724d5da8ef38925e7d3b99a2f2861c40b4a42d3c61a350bf3d`

named on 2026-10-27 and named again on 2026-11-16, the same digest, so that
§7.4 reads the GALEED passages it cites as they stand on 2026-11-16; the
review record beside this text (docs/eravon-dossier-v1-review.md) records
how it converged. The fuzzing this law defers with the two below it was run
across the same six rounds (the inputs counted at `eravon.terms/1` §10.5):
the whole dossier under a named candidate, a pool candidate and consent by
call as a first and as a later stage, the receipt's `prev` moved, broken,
missing and re-signed with the index's digest recomputed and left stale, and
the host's limits probed again, with zero disagreements; the closing round's
first measurement, the programs unchanged, was zero at seeds 1 to 6 over
1,197 cases per seed. The fuzzing this law defers with the two below it was
run, for the cores named on 2026-10-19
(`0x0a6b5b321985f2f22f4f66752695b86fe0e23b00260bacf4d2a654db6f92932e` here),
across the five rounds of the revision of 2026-10-17 and the two of
2026-10-18 and 2026-10-19 over their scans, as their §10.5 and §11.5 record;
this law's own surface, §1's file names and the one path the reader opens,
the three seats probed at the host's limits (36 spellings of `<dir>`, the
path-length limit straddled from 1,020 to 1,026 bytes under every spelling,
link chains of 15 to 33 and the loop, a link whose target differs in case
alone, entries that are devices), zero disagreements with that core.
