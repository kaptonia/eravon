# ERAVON link v1 (`eravon.link/1`)

The link law of ERAVON: the second document law, so that a seller's client,
a buyer's client, and a judge's desk agree byte for byte on the five
documents that carry an order between the seller and the buyer: the
**listing** a seller posts on the standard's board, the **link** a seller
hands a buyer, which points at that listing, the **receipt** a buyer hands
back to the seller, the **case pack** a buyer keeps so that nothing from the
seller is needed afterwards, and the **settlement offer** a seller posts on
the board during a case. Where [`eravon.terms/1`](eravon-terms-v1.md) says
what a terms document, an offer, an order, the action messages, and the
signatures are, `eravon.link/1` says how they travel, and, over the records
one chain holds, who a listing's seller is and what a key's record has done
(§6); it consumes that law only through its frozen core, and reads the
GALEED family's succession book only by that contract's own code (§1, §6.1).
What a seller sends a buyer is a pointer, and the content stands on the
chain; what a buyer sends a seller is a text, the receipt from which the
seller's client opens the case (§4).

**Status.** The law is the set of total predicates in this text. A
conforming implementation embeds an `eravon.terms/1` core that agrees on
every input with the core of the release digest §11.4 names and agrees with the core §11.5 of this text names on
every input. Evolution goes through `eravon.link/2`.

**Rule classes.** Three classes appear, and every rule below is marked:

- **[D]** document predicates: decided offline from the byte strings and
  the arguments the predicate is given, and nothing else. No predicate of
  this law consults a chain, a clock, or a registry; `resolveLink` (§3.3)
  is handed the bytes a client fetched and reads nothing itself.
- **[R]** record predicates: decided from a record set of one chain (§6.1),
  the succession book's records among them, and the query, and nothing else;
  no predicate of this law consults a chain, the reader handing it the
  records it took (§10).
- **[O]** producer obligations: rules a producer keeps, decided by no
  predicate of this law, carried in no answer, and measured by no corpus.

---

## 1. Notation

`galeed/1` §1 and `eravon.terms/1` §1 apply: byte strings, the hex forms,
`dec256`, tokens, skeleton and prose strings, `int`, `bool`, `u32`, `zero`,
`pointer`, `base64url`, `word`, `DEPLOYER`, `initcode`, `proxyCodeHash`,
`create2`, and the `galeed/1` §6 preamble over member tables. In addition:

- **`canonical(b)`**: as `eravon.terms/1` §1 defines it, the `_md`
  suffix rule of `galeed/1` §3.3 deciding every string before any table
  below reads it. Every document of this law is a canonical object; the
  keys that are `_md` or end in it a passing document carries are
  `inline_md` below a listing's `terms` and `text_md` in a case pack's
  attachment; `galeed/1` §3.3's suffix rule decides every other key of
  every input alike (a `note_md` in a listing's rung being prose and then
  a member beyond the rung's list), and every string outside a prose
  subtree is a skeleton string.
- **`terms(b)`**, **`termsHash(b)`**, **`offer`**, **`order`**,
  **`action`**, **`verifyOffer`**, **`verifyOrder`**: the predicates of
  `eravon.terms/1` §2 through §5, decided by the embedded core.
- **`SIG_BY_CALL`**: that law's §5.4 answer on an empty signature slot,
  which the predicates below carry under names of their own:
  `OFFER_BY_CALL` for a candidate (§2.5), `RECEIPT_BY_CALL` for a
  receipt (§4.4), `SETTLE_BY_CALL` for a settlement offer (§8.3), each
  where that party's slot is empty.
- **position**: a pair `(blockNumber, logIndex)` of one chain, ordered
  `blockNumber` first, as `galeed.roster/1` §4.2 orders records.
- **the book's replay**: over the declarations (the book's `Declared(from,
  to)` logs) and accessions (its `Acceded(holder, to, root)` logs) of a
  record set (§6.1), walked in ascending position, a reader keeps for each
  address its root (initially itself), for each root its head (initially the
  root), for each address its pending key (initially none) and whether it
  has entered (initially not); the **current key** of an address is the head
  of its root. A declaration binds where its `from` is the current key of
  its own record, its `to` is neither zero nor `from` and has not entered,
  making `to` the pending key of `from` in place of any earlier one, and
  `from` enters; an accession counts where its `to` has not entered, is the
  pending key of `holder`, and `holder` is the current key of its own
  record, setting `to`'s root to `holder`'s root and that root's head to
  `to`, clearing `holder`'s pending key, and `to` enters; every other record
  does nothing, and an accession's `root` is the book's statement, read by
  no step. This is the replay `galeed.roster/1` §4.2 states, and the book's
  code, at the address `successionBook()` answers and with the code hash the
  standard's Shape names (`eravon.standard/1`, Shape and §10), decides it on
  chain, admitting exactly the records it binds and counts; no predicate of
  this law reads any other part of `galeed.roster/1`.
- **`currentAt(a, p)`**, **the keys of a record**: the current key of `a`
  with the records at `p` and after it unread; the keys of `a`'s record are
  its root and then each key in the order it acceded; a key **handed its
  record on** at the position of the accession that took the record from it.
- **the board**: the standard's `EravonBoard` (`eravon.standard/1` §7b),
  whose `Posted(poster, key, hash, document)` log carries a document's
  bytes in full with `hash` their `sha256`.

---

## 2. The listing (`eravon.listing/1`)

A listing is everything a buyer's client needs to show an order and take
a signature, posted once on the board, with no buyer field in it: the
terms document stands inside it in full, the seller's offers are already
signed, or claimed by call, inside it, and the buyer's choice, key, and
refund address are added by the buyer.

**[D] 2.1 Members, closed.** A listing is a canonical object with exactly
these members and no others:

| key | presence | form | rule |
|---|---|---|---|
| `v` | required | string | byte-equal to `eravon.listing/1` |
| `chainId` | required | int | the instance's chain, and the chain of the board the listing is posted on |
| `token` | required | hex20 | the settlement token; nonzero |
| `logic` | required | hex20 | the standard's logic; nonzero |
| `proxyCodeHash` | required | hex32 | byte-equal to `proxyCodeHash(logic)`: the code hash the judge declared, carried so that a reader compares it with a judge's declaration; §2.5 recomputes it |
| `terms` | required | object | exactly `hash` (hex32, nonzero) and `inline_md` (prose string, the whole terms document as text), both required; `sha256` of `inline_md`'s UTF-8 bytes is byte-equal to `hash`; those bytes pass `eravon.terms/1` §2.4 and bind to this listing (§7) |
| `order` | required | object | the order's numbers (§2.2) |
| `judges` | required | array | the named candidates (§2.3); may be empty |
| `guilds` | required | array | the pool candidates (§2.3); may be empty |

**[D] 2.2 The order object.** Exactly these members and no others:
`sellerKey` (hex20, nonzero), `sellerPayout` (hex20, nonzero), `sellerFund`
(hex20, nonzero), `amount` (dec256, nonzero; the bound of `eravon.terms/1` §2.3 reaches it through
§7, `BIND_ORDER`, the document carrying it), `deliveryWindow` (u32),
`confirmWindow` (u32),
`openWindow` (u32), `buyerMayOpen` (bool), `ladder` (an array of two to
sixteen rungs, each an object with exactly `buyerBps`, `sellerBps`,
`feeBearer` under `eravon.terms/1` §2.5's rung rows, a `note_md` or any
other member being a member beyond the list here and `E_RUNG_CLOSED` with
the rung's index, and the three ladder-wide rules of that section),
`nonce` (int), `expiry` (int). These are the offer's numbers as
`eravon.terms/1` §4.3 would draw them from the document.

**[D] 2.3 Candidates.** A **judge** element is `eravon.terms/1` §2.5's judge
table with one more required member, `sig` (hex65, or the empty string of
`eravon.terms/1` §5.4), the seller's signature over the offer this candidate
makes, or the seller's claim to consent to it by call. A **guild** element
is that section's guild table with three more required members: `seq` (int,
the index of the panel record under the guild's root in the panel book the
seller read, `eravon.standard/1` §7), `panelHash` (hex32, nonzero, that
record's panel hash), and `sig` (hex65 or the empty string, as the judge
element's). In both inherited tables `sellerKey` is the listing's
`order.sellerKey` and `chainId` the listing's own. The added members are
required members of the element and members of its list, decided with the
inherited table's presence and closure tests (`E_JUDGE_MISSING`,
`E_JUDGE_CLOSED`, `E_GUILD_MISSING`, `E_GUILD_CLOSED`); their forms are
tested after every row of the inherited table, in the order this section
names them. The candidates are numbered judges first in array order, then
guilds, as `eravon.terms/1` §4.3 numbers them.

**[D] 2.4 The candidate's offer.** For candidate `c` of a listing `S`,
`listingOffer(S, c)` is the offer value of `eravon.terms/1` §4.1 with
`sellerKey`, `sellerPayout`, `sellerFund`, `amount`, `deliveryWindow`, `confirmWindow`,
`openWindow`, `buyerMayOpen`, `ladder`, `nonce`, `expiry` from `order`,
`token` from `S`, `termsHash` = `terms.hash`, and `judge`, `guild`,
`amanahHash`, `quoteHash` from the candidate exactly as `eravon.terms/1`
§4.3 draws them from a document's element; it yields no token, `S`'s own tests having
decided every member and §7's `BIND_ORDER` the amount's bound. Its digest
is `digest(chainId, logic,
hashStruct(offer))` of `eravon.terms/1` §4.2.

**[D] 2.5 Order of decision.** `canonical(b)` (its tokens); root an
object (`E_LISTING`); the nine required members present
(`E_LISTING_MISSING`), then no member beyond the nine
(`E_LISTING_CLOSED`); `v` (`E_LISTING_V`); `chainId` (`E_LISTING_CHAIN`);
`token` (`E_LISTING_TOKEN`); `logic` (`E_LISTING_LOGIC`); `proxyCodeHash`
a hex32 byte-equal to `proxyCodeHash(logic)` (`E_LISTING_CODEHASH`);
`terms` an object of exactly `hash`, a nonzero hex32, and `inline_md`, a
string (`E_LISTING_TERMS`), then `sha256` of `inline_md`'s UTF-8 bytes
byte-equal to `hash` (`E_LISTING_TERMS_HASH`), then those bytes pass
`eravon.terms/1` §2.4 (`E_LISTING_TERMS_DOC`, carrying the token); `order`
an object (`E_LISTING_ORDER_MISSING`, a value that is no object carrying
none of the eleven members) with exactly the eleven members
(`E_LISTING_ORDER_MISSING`, then `E_LISTING_ORDER_CLOSED`), then
`sellerKey` (`E_LISTING_SELLER`), `sellerPayout` (`E_LISTING_PAYOUT`),
`sellerFund` (`E_LISTING_FUND`), `amount` (`E_LISTING_AMOUNT`), `deliveryWindow`
(`E_LISTING_DELIVERY_WINDOW`), `confirmWindow`
(`E_LISTING_CONFIRM_WINDOW`), `openWindow`
(`E_LISTING_OPENWINDOW`), `buyerMayOpen` (`E_LISTING_BUYEROPEN`), `ladder`
an array of two to sixteen (`E_LISTING_LADDER`) then each rung in order
with its index, then the three ladder-wide rules in `eravon.terms/1` §2.5's order,
the tokens of `eravon.terms/1` §2.5 and §7, `nonce` (`E_LISTING_NONCE`),
`expiry` (`E_LISTING_EXPIRY`); `judges` an array (`E_LISTING_JUDGES`),
then each element in order under §2.3 with its index, the
`eravon.terms/1` judge tokens in that law's §2.5 row order, then
`E_JUDGE_SIG` for the added member's form, then, every element having
passed, no judge address repeated (`E_LISTING_DUP_JUDGE`, carrying the index of the first element whose
address an earlier element already carries); `guilds` an array
(`E_LISTING_GUILDS`), then each element in order under §2.3 with its
index, the `eravon.terms/1` guild tokens in that law's §2.5 row order,
then `E_GUILD_SEQ`, `E_GUILD_PANEL`, `E_GUILD_SIG` for the added members'
forms, then, every element having passed, no guild address repeated
(`E_LISTING_DUP_GUILD`, carrying
the index); the two arrays not both empty (`E_LISTING_NO_CANDIDATE`);
then §7's comparisons over the listing and the bytes of
`terms.inline_md`, in that section's order from `BIND_CHAIN` on, yield
`BOUND` (`E_LISTING_BIND`, carrying the verdict; `BIND_HASH` is
unreachable here, the hash rule above having decided it); then, for each
candidate in candidate order, `verifyOffer(digest, sig, sellerKey)` over
that candidate's offer digest of §2.4 (`E_LISTING_SIG`, carrying the
candidate index and the signature token), the next candidate untouched
until the present one has passed, or answered `SIG_BY_CALL`. A candidate
whose slot is empty answers `SIG_BY_CALL` there and is a **by-call
candidate** (`OFFER_BY_CALL`), which refuses nothing: the test moves on
to the next candidate, and a listing that passes lists this one by its
candidate index (§2.3's numbering, the index `E_LISTING_SIG` carries)
with its offer's digest of §2.4, the digest a buyer's client looks up in
the logic's registry under `sellerKey` (§10, `eravon.terms/1` §8), and
shows the candidate only once the registry holds it. A byte string that
passes is a listing; nothing else is.

**2.6 Meaning.** A listing that passes carries the whole terms document and,
for every candidate, an offer the seller has signed or claims to consent to
by call, so a buyer's client needs no seller online and nothing beyond the
chain: the buyer picks a candidate, adds a key, a refund address, and the
`openBy` of `eravon.terms/1` §5.5, and the order of `eravon.terms/1` §4.4
exists. The members outside the offer are data for the buyer's client, and a
document's `series` (`eravon.terms/1` §2.3) groups this listing with the
other stages of its sale (§10): `proxyCodeHash` follows from `logic` and a
judge's `source` from the document (§7), and a guild's `seq` and `panelHash`
are the seller's reading of the book, which a repost may change without
touching any signature. A segment of the document carried as a hash
(`eravon.terms/1` §2.3) has its text in no listing: the seller hands it to
the buyer apart, and the buyer's client checks it against the hash and keeps
it in the case pack (§5). A by-call candidate is one whose seller consents
by call (a company, whose `sellerKey` is its contract's address, or any
seller that chooses the call, `eravon.terms/1` §5.4): its offer binds once
the standard admits that consent (`eravon.terms/1` §8), and a buyer's
client, which acts before any transaction, shows the candidate only where
the registry holds the offer's digest under `sellerKey` (§10). The listings
of one document agree in every member but a guild element's `seq` and
`panelHash` and a candidate's `sig`, and a link names one of them by its
digest; a seller who changes the terms writes a new document, posts a new
listing, and hands out a new link, the old link pointing at the old listing
for as long as the chain stands, and the standard's `expiry`, `openBy`, and
revocation deciding whether a case still opens under the old one
(`eravon.standard/1` §4).

**[O] 2.7 Posting.** A seller posts a listing's canonical bytes on the board
of the listing's `chainId` through `post(termsHash, bytes)`, `termsHash`
being `terms.hash`, before it hands out any link to it, from `sellerKey` (a
seller that is a contract by its own call), so that the listing's position
is the seller's own (§6.2); anyone may repost a listing, and a repost moves
no position and enters no history; a reader authenticates a listing by its
content, which §2.5 decides together with every signature the listing
carries, whoever posted it, while its place, and whether a buyer pays under
it, come from its seller's own posting alone (§6.2, §6.4); whether a by-call
candidate's consent stands is the standard's to decide (`eravon.terms/1`
§8); before any `init` the registry is where it is read, so a buyer's client
reads it before it shows that candidate (§10), while a reader of an opened
case reads no registry to find or authenticate the listing
(`eravon.standard/1` §2), the consent `init` admitted being what
`eravon.dossier/1` §5.1 reads at the chain. A seller that revokes a document
(`eravon.standard/1` §4) posts nothing for it; the revocation stands on the
logic, and a buyer's client reads it there before it shows the listing
(§10).

---

## 3. The link (`eravon.link/1`)

A link is a pointer to one posted document: the chain, the transaction
that posted it, and the document's digest. It carries no content and is
the one thing a seller sends a buyer.

**[D] 3.1 Members, closed.** A link is a canonical object with exactly
these members and no others, decided in this order: `canonical(b)` (its
tokens); root an object (`E_LINK`); the four required members present
(`E_LINK_MISSING`), then no member beyond the four (`E_LINK_CLOSED`);
`v` byte-equal to `eravon.link/1` (`E_LINK_V`); `chainId` an int
(`E_LINK_CHAIN`); `tx` a hex32 (`E_LINK_TX`: the transaction whose receipt
carries the board's `Posted` log for the document); `hash` a hex32
(`E_LINK_HASH`: the document's `sha256`, the `hash` topic of that log). A
byte string that passes is a link; nothing else is.

**[D] 3.2 Transport.** The link text is `eravon://open?e=` followed by
`base64url` of the link's canonical bytes; a seller sends it as a text, and
the buyer's client opens it. `encodeLink(b)` yields the text or the token
§3.1 names for `b`; `decodeLink(t)` over a byte string `t` yields the link
bytes or one token, in this order: `t` does not begin with the 16 bytes
`eravon://open?e=` (`E_LINK_PREFIX`); the remainder after the prefix is
empty, holds a byte outside the 64 of RFC 4648 §5's alphabet (`A` to `Z`,
`a` to `z`, `0` to `9`, `-`, `_`), has a length ≡ 1 (mod 4), or its unused
trailing bits are not zero (`E_LINK_B64`); the decoded bytes fail §3.1
(`E_LINK_DOC`, carrying the token).

**[D] 3.3 Resolving a link.** `resolveLink(L, D)` over a link byte string
and the byte string of the document a client fetched for it (§10) yields
one verdict, in this order: `L` fails §3.1 (`LINK_INVALID`, carrying the
token); `sha256(D)` differs from `L`'s `hash` (`LINK_HASH`);
`canonical(D)` yields a token, or its root is no object, or the root
carries no `v`, or `v` is no string (`LINK_KIND`); `v` is byte-equal to
`eravon.listing/1`, and `D` fails §2.5 (`LINK_LISTING`, carrying the
token) or its `chainId` differs from `L`'s (`LINK_CHAIN`), else `LISTING`
with §2.5's answer; `v` is byte-equal to `eravon.settle/1`, and `D` fails
§8.2 (`LINK_SETTLE`, carrying the token) or its `chainId` differs from
`L`'s (`LINK_CHAIN`), else `SETTLE` with §8.2's answer; any other `v`
(`LINK_UNKNOWN`). A link whose document resolves is one the client shows;
whether the document was fetched from the chain `L` names, from a board
of the standard's code, and from the transaction `L` names is the
client's reading, which §10 states and which this law takes as given in
`D`.

**3.4 Meaning.** The link is a pointer and nothing more: two links to one
document differ in nothing a buyer reads, and a link reposted anywhere
still names the same bytes by their digest. A client that cannot fetch
the document shows no order; a client whose fetch yields other bytes than
the digest names has been handed the wrong document and shows nothing
(§10).

---

## 4. The receipt (`eravon.receipt/1`)

A receipt is what a buyer hands back to the seller once it has signed
the order or claimed to consent to it by call: what the seller's client
needs to compute the instance and, once the money arrives, to initialise
it.

**[D] 4.1 Members, closed.** A receipt is a canonical object with exactly
these members and no others:

| key | presence | form | rule |
|---|---|---|---|
| `v` | required | string | byte-equal to `eravon.receipt/1` |
| `termsHash` | required | hex32, nonzero | the document the listing named, tested as §2.1's `terms.hash` is |
| `candidate` | required | int | the candidate the buyer chose, numbered as §2.3 numbers them |
| `buyerKey` | required | hex20 | nonzero |
| `buyerPayout` | required | hex20 | nonzero |
| `openBy` | required | int | the order's `openBy` (`eravon.terms/1` §4.4), as the buyer's client set it at signing or, for a receipt by call, at export (`eravon.terms/1` §5.5) |
| `prev` | required | hex20 | the order's `prev` (`eravon.terms/1` §4.4): zero, or the instance of the stage before, as the buyer's client set it |
| `sig` | required | hex65 or the empty string | the buyer's signature of `eravon.terms/1` §5.2, or the empty string of that law's §5.4, the buyer's claim to consent by call |
| `seq` | optional | int | for a pool candidate, the index of the panel record the buyer's client read when it set `openBy` (`eravon.terms/1` §5.5), the buyer's own record; absent for a named judge |

**[D] 4.2 Order of decision.** `canonical(b)`; root an object (`E_RECEIPT`);
the eight required members present (`E_RECEIPT_MISSING`), then no member
beyond the nine (`E_RECEIPT_CLOSED`); `v` (`E_RECEIPT_V`); `termsHash`
(`E_RECEIPT_TERMS`); `candidate` (`E_RECEIPT_CANDIDATE`); `buyerKey`
(`E_RECEIPT_BUYER`); `buyerPayout` (`E_RECEIPT_PAYOUT`); `openBy` an int
(`E_RECEIPT_OPENBY`); `prev` a hex20, zero admitted (`E_RECEIPT_PREV`);
`sig` a hex65 or the empty string (`E_RECEIPT_SIG`); `seq`, where present,
an int (`E_RECEIPT_SEQ`). A byte string that passes is a receipt; nothing
else is; whether its signature verifies is §4.4's question, which needs the
listing.

**[D] 4.3 Transport.** The receipt text is `eravon-receipt:` followed by
`base64url` of the receipt's canonical bytes; `encodeReceipt` and
`decodeReceipt` are §3.2 with that 15-byte prefix alone
(`E_RECEIPT_PREFIX`), the same base64 rule (`E_RECEIPT_B64`), and §4.2
on the decoded bytes (`E_RECEIPT_DOC`, carrying the token).

**[D] 4.4 Binding a receipt to a listing.** `bindReceipt(S, R)` over a
listing byte string and a receipt byte string yields the order or one token,
in this order: `S` fails §2.5 (`LISTING_INVALID`, carrying the token); `R`
fails §4.2 (`RECEIPT_INVALID`, carrying the token); `R`'s `termsHash`
differs from `S`'s `terms.hash` (`RECEIPT_TERMS_MISMATCH`); `R`'s
`candidate` is not below the number of candidates
(`RECEIPT_CANDIDATE_RANGE`); the candidate is a guild and `seq` is absent,
or a judge and `seq` is present (`RECEIPT_SEQ`); `order(listingOffer(S, c),
S's chainId, S's logic, R's buyerKey, R's buyerPayout, R's openBy, R's
prev)`, `c` being `R`'s `candidate`, yields a token (`RECEIPT_ORDER`,
carrying it); `verifyOrder(orderDigest, R's sig, R's buyerKey)` yields a
token (`RECEIPT_SIG`, carrying it), or yields `SIG_BY_CALL`
(`RECEIPT_BY_CALL`, carrying `salt`, `caseId`, `instance`, and `orderDigest`
as `RECEIPT_BOUND` does: the receipt names this order and claims the buyer's
consent to it by call, which the seller's client reads at the chain before
it sends `init`). Otherwise `RECEIPT_BOUND` with `salt`, `caseId`,
`instance`, and `orderDigest`. A bound receipt is a case the seller's client
can watch the instance address for and initialise; `R`'s `prev` enters the
order as every other member does, and whether a seller takes a receipt with
that `prev` is §10's Stages rule, which this binding does not decide. A
by-call one is such a case once the registry holds the buyer's consent to
`orderDigest`; where the order's `buyerMayOpen` is `true` it also opens by
the buyer's own `init`, whose caller, the buyer's current key, is its
consent (`eravon.terms/1` §8), and where it is `false` the registration is
the buyer's one way to consent, the `init` that opens the case coming from
`sellerKey`'s current key alone (`eravon.standard/1` §4; an `init` sent
after `openBy`, the offer's `expiry` or a revocation refunds the buyer, sent
by either party, that law's §4 step 3). An unbound one is a text. A pool
receipt's `seq` is compared with nothing: the buyer's client re-reads the
panel book when it sets `openBy` (`eravon.terms/1` §5.5) and the instance
freezes the record `eravon.standard/1` §7 names at `init`, so the seller's
`seq` and the buyer's may lawfully differ.

**4.5 Meaning.** The receipt carries no money and no secret: the buyer's key
stays with the buyer, the signature, or the claim to consent by call, is
over one order under one instance address, and a receipt shown to anyone who
holds the listing it binds to lets that person compute the address, and lets
either party, when the balance is there and each party's consent is a
signature by its current key or a registry entry (`eravon.standard/1` §9),
send `init` where `buyerMayOpen` is `true`; where it is `false`, the `init`
that opens the case comes from `sellerKey`'s current key alone
(`eravon.standard/1` §4; the refund `init` of that law's §4 step 3 from
either party). A buyer whose order admits its own `init` (`buyerMayOpen`)
needs no receipt to travel: it funds and opens the case itself, and a
seller's client rebuilds the receipt from the `init` call's data
(`eravon.dossier/1` §5.3). Whether the money has arrived, and whether `init`
will succeed, is the chain's answer.

---

## 5. The case pack (`eravon.pack/1`)

A case pack is the buyer's whole case in one file: the link, a copy of the
listing, the receipt, and the texts of any segment the document carries as a
hash. With it a buyer can act from the client that holds its account key
(`eravon.terms/1` §5.5), with nothing from the seller; it is a file the
buyer keeps and never a transport.

**[D] 5.1 Members, closed.** A case pack is a canonical object with
exactly these members and no others:

| key | presence | form | rule |
|---|---|---|---|
| `v` | required | string | byte-equal to `eravon.pack/1` |
| `chainId` | required | int | equal to the listing's and the link's |
| `instance` | required | hex20 | the instance §4.4 derives |
| `caseId` | required | hex32 | the salt §4.4 derives |
| `link` | required | object | a link: its canonical bytes pass §3.1 |
| `listing` | required | object | the listing's copy: its canonical bytes pass §2.5 and their `sha256` is byte-equal to `link.hash` |
| `receipt` | required | object | a receipt: its canonical bytes pass §4.2 and bind to the listing (§4.4) |
| `attachments` | optional | array | the texts of the document's hashed segments (§5.3); may be empty |

**[D] 5.2 Order of decision.** `canonical(b)`; root an object (`E_PACK`);
the seven required members present (`E_PACK_MISSING`), then no member beyond
the eight (`E_PACK_CLOSED`); `v` (`E_PACK_V`); `chainId` an int
(`E_PACK_CHAIN`); `instance` (`E_PACK_INSTANCE`); `caseId` (`E_PACK_CASE`);
`link`'s canonical bytes pass §3.1 (`E_PACK_LINK`, carrying the token; a
`link` that is no object reaches §3.1's own root test and carries `E_LINK`);
`listing`'s canonical bytes pass §2.5 (`E_PACK_LISTING`, carrying the token;
a `listing` that is no object reaches §2.5's own root test and carries
`E_LISTING`), then their `sha256` is byte-equal to `link.hash`
(`E_PACK_HASH`); `receipt`'s canonical bytes pass §4.2 (`E_PACK_RECEIPT`,
carrying the token, `E_RECEIPT` where the member is no object) and bind to
the listing under §4.4, `RECEIPT_BOUND` or `RECEIPT_BY_CALL`
(`E_PACK_ORDER`, carrying any other verdict); `chainId` equal to the
listing's and to the link's (`E_PACK_CHAIN_MISMATCH`); `instance` byte-equal
to the one §4.4 yielded under either verdict (`E_PACK_INSTANCE_MISMATCH`);
`caseId` byte-equal to the salt §4.4 yielded (`E_PACK_CASE_MISMATCH`);
`attachments`, where present, an array (`E_PACK_ATTACHMENTS`), then each
element in order under §5.3 with its index. A byte string that passes is a
case pack; nothing else is. The pack having passed `canonical(b)`, each
member's canonical bytes pass `galeed/1` §3.5, so no token that
`E_PACK_LINK`, `E_PACK_LISTING` or `E_PACK_RECEIPT` carries is itself a
`galeed/1` §3 token (an `E_LISTING_TERMS_DOC` under `E_PACK_LISTING` carries
the terms document's token, those tokens among them); the listing and the
receipt having passed their own sections first, `E_PACK_ORDER` never carries
`LISTING_INVALID`, `RECEIPT_INVALID`, or a `RECEIPT_SIG` with `E_SIG_FORM`
inside it.

**[D] 5.3 Attachments.** An attachment is an object with exactly `sha256`
(hex32) and `text_md` (prose string), decided in this order with its
index: the element is an object with exactly the two members, each of its
form (`E_PACK_ATTACHMENT`); `sha256` of `text_md`'s UTF-8 bytes is
byte-equal to `sha256` (`E_PACK_ATTACHMENT_HASH`); `sha256` is
byte-equal to the `goodsHash` or to the `deliveryHash` the document under
`terms.inline_md` carries (`eravon.terms/1` §2.3; a document that carries
neither names no attachment) (`E_PACK_ATTACHMENT_UNNAMED`: an attachment
is the text of a segment the document carries as a hash, and a text the
document names in no such member is no attachment of this case, whatever
else in the document spells its digest). Two attachments may carry one text,
and an attachment may stand for either segment; this law reads which
segment names it in no answer.

**5.4 Meaning.** A pack that passes is internally whole: the link names the
listing by digest, the listing is the seller's or claims the seller's
consent by call, the receipt is the buyer's or claims the buyer's consent by
call, the address is the one both derive to, and every attached text is one
the document named. What the chain holds at that address, the case's state,
and whether the money was ever sent are the chain's; a client that imports a
pack reads the chain next. The pack is the buyer's own storage and travels
as a file; this law defines no encoder for it beyond its canonical bytes.

---

## 6. Identity

Who a listing's seller is and what a key's record has done, decided over the
records one chain holds, so that every reader holding the same records shows
the same history and gives the same verdict before a buyer pays.

**[R] 6.1 Inputs.** Every predicate of this section is decided from a
**record set** `R` of one chain and the query, and nothing else. `R` holds,
no two records sharing a position: `posted(position, poster, key, hash,
document)` from a board (`eravon.standard/1` §7b); `consented(position, who,
digest)`, `revoked(position, seller, termsHash)` and `pulled(position,
seller, fund, token, instance, amount)` from the logic; `opened(position,
emitter, caseId, seller, buyer, judge, amount, termsHash)` and every other
log of an instance, as `event(position, emitter, name)`; and the `Declared`
and `Acceded` logs of the succession book at the address `successionBook()`
answers (`eravon.standard/1`, Shape and §10), taken only where its code hash
is the one the standard's Shape names. `currentAt` and the keys of a record
are §1's over those book records. A posting's `hash` and an accession's
`root` are read by no predicate of this section, and no predicate compares a
listing's `chainId` with the chain `R` was read on. Which logs a reader
takes, and from which block, is §10's.

**[R] 6.2 Documents and position.** A **document** of `R` is a pair
`(sellerKey, termsHash)` drawn from a `posted` whose `document` passes §2.5,
whose `key` equals that listing's `terms.hash` and whose `poster` is that
listing's `order.sellerKey` (its `order.sellerKey` and `terms.hash`: the
seller's own posting), from an `opened` (its `seller` and `termsHash`), or
from a `revoked` (its `seller` and `termsHash`). Its **position** is the
first of these that exists: the least position of such a posting (`self`);
the least position of an `opened` drawing it (`opened`); the least position
of a `revoked` drawing it (`revoked`). A `posted` from any other poster is a
repost: it draws no document, places none and enters no history, since a
listing whose candidates are claimed by call carries no signature and anyone
could post one naming any `sellerKey`.

**[R] 6.3 Standing.** A thing a key `k` did at a position `p` **stands** for
`k`'s record where `currentAt(k, p)` is `k`, and is an **attempt**
otherwise, done after `k` handed its record on: a key speaks for its record
until it hands the record on, and a key that accedes to another record
brings into it what it did before, having been its own record's current key
until then. A posting of its own stands or is an attempt as its `poster`
does at the posting's position, and a document with one stands or is an
attempt as the least such posting does, at the document's position (§6.2),
no later posting changing it; a document drawn from an `opened` or a
`revoked` alone stands. Every `opened`, `consented` and `revoked` of `R`
stands for the records of the keys it names, the standard admitting each
only from the current key (`eravon.standard/1` §9).

**[R] 6.4 The check before paying.** `payCheck(S, R)` over a listing byte
string yields one verdict, in this order: `S` fails §2.5 (`LISTING_INVALID`,
carrying the token); `R` holds no posting of its own (§6.2) whose `document`
is byte-equal to `S` (`UNPLACED`); the least such posting stands (`PAYABLE`,
§6.3); otherwise `SELLER_KEY_RETIRED`. The verdict is the listing's own and
never its document's: a listing first posted while its key was current stays
payable after the key hands its record on, its case opening on the current
key's consent (`eravon.standard/1` §9), and no later posting of it changes
that; a listing a key first posts after handing its record on is refused,
whatever document it shares with an earlier listing; a listing its key never
posted is unplaced, whoever posted it; a declaration no accession took up
moves nothing and refuses nothing.

**[R] 6.5 History.** `history(a, R)` is the object with `keys`, the keys of
`a`'s record in order; `documents`, one per document whose `sellerKey` is
among `keys`, ascending by position then by `termsHash`, each with its
position, its source word (`self`, `opened` or `revoked`, §6.2), its
standing word (`stands` or `attempt`, §6.3), the `posted` records that draw
it under §6.2, each with its own standing word, `stands` or `attempt` (a
repost, a posting whose bytes fail §2.5 and one whose `key` is not that
listing's `terms.hash` draw none and are not listed), its `revoked` records,
and its cases, each case with `caseId`, `instance` (the `opened`'s
`emitter`), `buyer`, its position (its `opened`'s), and every record of `R`
whose `emitter` is that instance together with every `pulled` whose
`instance` is it; `consents`, every `consented` whose `who` is among `keys`;
`cases`, every `opened` whose `buyer` is among `keys`, written as a
document's case; and `attempts`, every document of `documents` that is an
attempt; every list of the answer but `keys` in ascending position,
`documents` and `attempts` then by `termsHash`. A segment the document
carries as a hash (`eravon.terms/1` §2.3) shows here as its digest; whoever
holds the text proves it by §5.3's comparison.

**6.6 Meaning.** Identity decides what a reader shows and whether it lets a
buyer pay; the standard decides who acts, from the same book on the same
chain, so that the two never disagree about a key. A record carries every
key's history up to the key's handing on, a key that acceded included; what
it carries cannot be shed by moving it, and a key that does not continue its
record takes a new key that enters no book and shares nothing with the old
(§10).

---

## 7. Binding a listing to its document

**[D] 7.1** `bind(S, T)` over a listing byte string and a terms document
byte string yields one value, in this order: `S` fails §2.5
(`LISTING_INVALID`, carrying the token); `T` fails `eravon.terms/1` §2.4
(`TERMS_INVALID`, carrying the token); `termsHash(T)` differs from `S`'s
`terms.hash` (`BIND_HASH`); `S`'s `chainId`, `token`, or `logic` differs
from the document's (`BIND_CHAIN`, `BIND_TOKEN`, `BIND_LOGIC`); `S`'s
`order` differs from what `eravon.terms/1` §4.3 draws in any of
`sellerKey`, `sellerPayout`, `sellerFund`, `amount`, `deliveryWindow`, `confirmWindow`,
`openWindow`, `buyerMayOpen`, `ladder`, `nonce`, `expiry` (`BIND_ORDER`);
`S`'s `judges` differ from the document's in length or, at some index, in
`judge`, `amanahHash`, `quoteHash`, or `source`, a member present on one
side and absent on the other being a difference (`BIND_JUDGES`, carrying
`S`'s own number of judges where the lengths differ, whatever the
elements, and otherwise the first differing index); `S`'s `guilds` differ
likewise in length or, at some index, in `pointer`, `amanahHash`, or
`quoteHash` (`BIND_GUILDS`, carrying `S`'s own number of guilds where the
lengths differ, whatever the elements, and otherwise the first differing
index). Otherwise `BOUND`. §2.5 runs the comparisons from `BIND_CHAIN` on over a listing and its own
`terms.inline_md`, so that a listing that passes is bound to the document
it carries; `bind` over a listing that passes answers `BOUND`,
`TERMS_INVALID`, or `BIND_HASH`, the comparison verdicts from `BIND_CHAIN`
on occurring inside `E_LISTING_BIND` alone, since a document that passes
`BIND_HASH` is the bytes of `terms.inline_md`. A
bound listing shows the buyer the document's own candidates and numbers,
so the receipt's `candidate` means the same thing in both.

---

## 8. The settlement offer (`eravon.settle/1`)

A settlement offer is what a seller posts on the board during a case: the
seller's `Settle` message of `eravon.terms/1` §4.5, signed or claimed by
call, which the buyer countersigns in its client or consents to by call, and
which either party submits where both consents are signatures or registry
entries (`eravon.standard/1` §9).

**[D] 8.1 Members, closed.** A settlement offer is a canonical object with
exactly these members and no others:

| key | presence | form | rule |
|---|---|---|---|
| `v` | required | string | byte-equal to `eravon.settle/1` |
| `chainId` | required | int | the instance's chain |
| `instance` | required | hex20 | the instance; nonzero |
| `buyerBps` | required | int | at most 10000: the buyer's share of the price |
| `expiry` | required | int | Unix seconds after which the contract refuses the settlement |
| `sig` | required | hex65 or the empty string | the seller's signature over `actionDigest` of `Settle(buyerBps, expiry)` under `(chainId, instance)`, or the empty string of `eravon.terms/1` §5.4, the seller's claim to consent to it by call |

**[D] 8.2 Order of decision.** `canonical(b)`; root an object
(`E_SETTLE`); the six required members present (`E_SETTLE_MISSING`), then
no member beyond the six (`E_SETTLE_CLOSED`); `v` (`E_SETTLE_V`);
`chainId` (`E_SETTLE_CHAIN`); `instance` (`E_SETTLE_INSTANCE`);
`buyerBps` (`E_SETTLE_BPS`); `expiry` (`E_SETTLE_EXPIRY`); `sig` hex65
or the empty string (`E_SETTLE_SIG`). A byte string that passes is a
settlement offer; nothing else is; whose signature it carries is §8.3's
question.

**[D] 8.3 Binding a settlement to a case.** `bindSettle(K, S)` over a
case pack byte string and a settlement byte string yields one value, in
this order: `K` fails §5.2 (`PACK_INVALID`, carrying the token); `S`
fails §8.2 (`SETTLE_INVALID`, carrying the token); `S`'s `chainId`
differs from `K`'s (`SETTLE_CHAIN`); `S`'s `instance` differs from `K`'s
(`SETTLE_INSTANCE`); `verifyOffer(actionDigest, S's sig, sellerKey)` over
the `Settle` digest of `eravon.terms/1` §4.5 under `(K's chainId, K's
instance)` with `sellerKey` = `K`'s listing's `order.sellerKey` yields a
token (`SETTLE_SIG`, carrying it; never `E_SIG_FORM`, §8.2 having decided
the form), or yields `SIG_BY_CALL` (`SETTLE_BY_CALL`, carrying the
`actionDigest`: the seller claims consent by call, which the buyer's
client takes only once the registry holds the digest under `sellerKey`,
§10). Otherwise `SETTLE_BOUND` with the `actionDigest`, which is what the
buyer signs next.

**[O] 8.4 Posting.** A seller posts a settlement offer's canonical bytes
on the board of the instance's chain through `post(caseId, bytes)`, from
`sellerKey` or from any address, and may hand the buyer a link (§3) to
it; a buyer's client that scans the board under its case's `caseId`
finds every offer posted without a link. A settlement offer on chain is
public: `buyerBps`, `expiry`, and the signature are read by anyone, and
the signature serves no other digest than this instance's `Settle`.

**8.5 Meaning.** A bound settlement is one the buyer can accept by
signing the same digest; the contract checks each side's signature, or
its consent by call, and the expiry, and this law verifies the seller's
half alone, the buyer's being made after. A by-call settlement is one
whose seller consents by registering the digest (the buyer's client
consenting to one only once the registration is on chain, §10); a buyer
who consents by call registers the digest on the same condition or
sends `settle` itself, which the standard refuses unless the seller's
consent stands, so a settlement may carry no signature at all and still
close a case. A settlement whose `buyerBps` the buyer dislikes is
answered by silence; nothing here withdraws one, and the contract's clock
does.

---

## 9. Tokens and vocabularies

**Listing tokens** (closed; by §2.5): the `galeed/1` §3 tokens;
`E_LISTING`, `E_LISTING_MISSING`, `E_LISTING_CLOSED`, `E_LISTING_V`,
`E_LISTING_CHAIN`, `E_LISTING_TOKEN`, `E_LISTING_LOGIC`,
`E_LISTING_CODEHASH`, `E_LISTING_TERMS`, `E_LISTING_TERMS_HASH`,
`E_LISTING_TERMS_DOC` (carrying a token of `eravon.terms/1` §7's
document tokens other than `E_TERMS_UTF8`, `inline_md` being a string of
scalar values, with its index where that token carries one),
`E_LISTING_ORDER_MISSING`, `E_LISTING_ORDER_CLOSED`, `E_LISTING_SELLER`,
`E_LISTING_PAYOUT`, `E_LISTING_FUND`, `E_LISTING_AMOUNT`, `E_LISTING_DELIVERY_WINDOW`,
`E_LISTING_CONFIRM_WINDOW`, `E_LISTING_OPENWINDOW`, `E_LISTING_BUYEROPEN`,
`E_LISTING_LADDER`, `E_LISTING_NONCE`, `E_LISTING_EXPIRY`,
`E_LISTING_JUDGES`, `E_LISTING_GUILDS`, `E_LISTING_NO_CANDIDATE`,
`E_LISTING_BIND` (carrying a verdict of §7 other than `BOUND`,
`LISTING_INVALID`, `TERMS_INVALID`, and `BIND_HASH`, with its index where
the verdict carries one); with an index: the rung, judge, and guild
tokens of `eravon.terms/1` §7, `E_RUNG_NOTE` excepted, a listing's rungs
admitting no `note_md` (§2.2), and `E_LADDER_ORDER`, the tokens this law
adds to the two element tables (`E_JUDGE_SIG`, `E_GUILD_SEQ`,
`E_GUILD_PANEL`, `E_GUILD_SIG`), `E_LISTING_DUP_JUDGE`,
`E_LISTING_DUP_GUILD`, `E_LISTING_SIG` (also carrying a secp256k1 token
of that law's §7 other than `E_SIG_FORM`, `E_JUDGE_SIG` and `E_GUILD_SIG`
having decided the form); without an index: `E_LADDER_FIRST`,
`E_LADDER_LAST`.

**Link tokens** (closed; by §3.1): the `galeed/1` §3 tokens; `E_LINK`,
`E_LINK_MISSING`, `E_LINK_CLOSED`, `E_LINK_V`, `E_LINK_CHAIN`, `E_LINK_TX`,
`E_LINK_HASH`.

**Transport tokens** (closed; by §3.2 and §4.3): `E_LINK_PREFIX`,
`E_LINK_B64`, `E_LINK_DOC`; `E_RECEIPT_PREFIX`, `E_RECEIPT_B64`,
`E_RECEIPT_DOC`; the last of each group carrying the document's token.

**Receipt tokens** (closed; by §4.2): the `galeed/1` §3 tokens; `E_RECEIPT`,
`E_RECEIPT_MISSING`, `E_RECEIPT_CLOSED`, `E_RECEIPT_V`, `E_RECEIPT_TERMS`,
`E_RECEIPT_CANDIDATE`, `E_RECEIPT_BUYER`, `E_RECEIPT_PAYOUT`,
`E_RECEIPT_OPENBY`, `E_RECEIPT_PREV`, `E_RECEIPT_SIG`, `E_RECEIPT_SEQ`.

**Pack tokens** (closed; by §5.2 and §5.3): the `galeed/1` §3 tokens;
`E_PACK`, `E_PACK_MISSING`, `E_PACK_CLOSED`, `E_PACK_V`, `E_PACK_CHAIN`,
`E_PACK_INSTANCE`, `E_PACK_CASE`, `E_PACK_LINK` (carrying a link token other
than the `galeed/1` §3 tokens, §5.2), `E_PACK_LISTING` (carrying a listing
token other than those), `E_PACK_HASH`, `E_PACK_RECEIPT` (carrying a receipt
token other than those), `E_PACK_ORDER` (carrying a verdict of §4.4 other
than `RECEIPT_BOUND`, `RECEIPT_BY_CALL`, `LISTING_INVALID`, and
`RECEIPT_INVALID`), `E_PACK_CHAIN_MISMATCH`, `E_PACK_INSTANCE_MISMATCH`,
`E_PACK_CASE_MISMATCH`, `E_PACK_ATTACHMENTS`; with an index:
`E_PACK_ATTACHMENT`, `E_PACK_ATTACHMENT_HASH`, `E_PACK_ATTACHMENT_UNNAMED`.

**Settlement tokens** (closed; by §8.2): the `galeed/1` §3 tokens;
`E_SETTLE`, `E_SETTLE_MISSING`, `E_SETTLE_CLOSED`, `E_SETTLE_V`,
`E_SETTLE_CHAIN`, `E_SETTLE_INSTANCE`, `E_SETTLE_BPS`, `E_SETTLE_EXPIRY`,
`E_SETTLE_SIG`.

**Verdicts** (closed): link resolution `LISTING` and `SETTLE` (each carrying
the document's answer), `LINK_INVALID` (carrying a token), `LINK_HASH`,
`LINK_KIND`, `LINK_LISTING` and `LINK_SETTLE` (each carrying a token of its
document other than the `galeed/1` §3 tokens, the root token, and the `v`
token, `LINK_KIND` having decided those), `LINK_CHAIN`, `LINK_UNKNOWN`;
receipt binding `RECEIPT_BOUND` and `RECEIPT_BY_CALL` (each carrying the
four values of §4.4), `LISTING_INVALID` and `RECEIPT_INVALID` (each carrying
a token), `RECEIPT_TERMS_MISMATCH`, `RECEIPT_CANDIDATE_RANGE`,
`RECEIPT_SEQ`, `RECEIPT_ORDER` (carrying `E_ORDER_BUYER`, and never
`E_ORDER_PAYOUT`, `E_ORDER_OPENBY` or `E_ORDER_PREV`, §4.2 having decided
those members) and `RECEIPT_SIG` (carrying a secp256k1 token of
`eravon.terms/1` §5.1 other than `E_SIG_FORM`, §4.2 having decided the
form); document binding `BOUND`, `LISTING_INVALID` and `TERMS_INVALID` (each
carrying a token), `BIND_HASH`, `BIND_CHAIN`, `BIND_TOKEN`, `BIND_LOGIC`,
`BIND_ORDER`, `BIND_JUDGES` and `BIND_GUILDS` (each carrying an index);
settlement binding `SETTLE_BOUND` and `SETTLE_BY_CALL` (each carrying the
digest), `PACK_INVALID` and `SETTLE_INVALID` (each carrying a token),
`SETTLE_CHAIN`, `SETTLE_INSTANCE`, `SETTLE_SIG` (carrying a secp256k1 token
other than `E_SIG_FORM`); the check before paying `PAYABLE`,
`SELLER_KEY_RETIRED`, `UNPLACED`, and `LISTING_INVALID` (carrying a token,
§6.4).

**Readings** (closed): `OFFER_BY_CALL`, the by-call candidate of §2.5,
carried by a passing listing's answer as the ascending list of such
candidates, each by its candidate index (§2.3's numbering, judges first, the
index `E_LISTING_SIG` carries) with its offer's digest of §2.4, and by no
token; the standing words `stands` and `attempt` and the position sources
`self`, `opened` and `revoked` (§6.2, §6.3), carried by §6's answers and by
no token.

**Literals this law fixes**: inside a document, `eravon.listing/1`,
`eravon.link/1`, `eravon.receipt/1`, `eravon.pack/1`, `eravon.settle/1`;
outside one, the prefixes `eravon://open?e=` and `eravon-receipt:`. Any
other `eravon://` address a client handles is the client's own and lies
outside these; where a later version of this family names a prefix a client
has used, the version's prefix takes it and the client's address moves.

---

## 10. Outside the grammar

- **Fetching a listing or a settlement offer.** A client handed a link asks
  the endpoints it trusts for chain `chainId` for the receipt of transaction
  `tx`, takes from it the `Posted` logs emitted by an account whose code
  hash is the standard's `boardCodeHash` (`eravon.standard/1` §7b,
  CODEHASH.md), keeps those whose `hash` topic equals the link's `hash` (one
  byte string, the board hashing what it emits), and hands those bytes to
  `resolveLink` (§3.3); it takes the bytes only where every endpoint it
  asked answers the same, shows a listing that resolves only where one of
  those logs carries the `key` topic that is the listing's `terms.hash`, the
  key a desk finds it by (`eravon.standard/1` §2) and a dossier's reader
  tests at `posted.tx` (`eravon.dossier/1` §5.1), and shows no order it
  could not fetch or could not resolve. A client without a link finds a
  case's settlement offers by the `key` topic `caseId` on the same board,
  and a client rebuilding a case from the chain finds its listing by the
  `key` topic `termsHash` as `eravon.standard/1` §2 states; every `Posted`
  log so taken is a record of §6.1's record set.
- **The client's checks at the act.** Whether `logic` is the standard and
  `proxyCodeHash` a declared standard; whether a judge's document now offers
  this `amanahHash` (`galeed.roster/1` §5.8), whether that entry stands
  anchored in the judge's ledger, whether the quote bytes its `quotes` carry
  under the key `<chainId>:<codeHash>:<token>` (`eravon.terms/1` §2.5,
  `eravon.standard/1` §3) hash to `quoteHash`, name this `token`, and cap
  the price at or above `amount`; whether the `text_md` of the guild's
  current covenant document hashes under `sha256` to this `amanahHash` and
  its `quotes` carry this quote (`galeed.roster/1` §3b.3, §5.9, §5.10);
  whether the panel book holds, under the guild's root, the record `init`
  would freeze and whether it passes the tests `init` runs on it
  (`eravon.standard/1` §7: the latest recorded before the block and
  unexpired at it, every word this `amanahHash`, and, where it names no
  router, a pool left once the two parties' keys and the zero address are
  removed), and whether `init` and the costliest draw under that record each
  fit one transaction under the chain's limits, the lesser of its block gas
  limit and its transaction gas cap where the chain sets one (2^24 under
  EIP-7825 on chain 1): `init`'s read of a panel, and a draw where the
  record names no router, grow with the square of the panel's length, and
  the draw's cost with the place in the pool the seed lands on, so the
  client measures, as the `redraw` transaction that lands it (the instance's
  storage set to the case's third seat and, for a later stage, to its
  `prev`, the stage at `prev` set as ended at share 0 and answering the
  series judge `eravon.standard/1` §7 posits for the costliest draw, the
  pool's member immediately before its last, seated on the case's second
  seat; the arming block's hash to a seed that lands there; the intrinsic
  gas counted), the draw landing on the pool's last member where the record
  names no router, and a draw whose router spends all of `routeGas()` where
  it names one (`eravon.standard/1` §7); whether the family declares, for
  the chain, the succession book the standard's code names
  (`eravon.standard/1`, Shape; `galeed.roster/1` §9), without which an
  instance there meets `galeed.venue/1` §7 item 16 on no reading; whether
  the price feed a quote names is live; for a pool candidate, whether the
  quote's `offerWindow` reaches 256 times the chain's nominal block interval
  (its slot time where it has one) and whether the chain's block hashes come
  from consensus (`eravon.standard/1` §3, §7); whether the order's
  `sellerFund` is its `sellerKey` or the logic's `funds(sellerKey,
  sellerFund)` holds, and whether `sellerFund` holds, on the settlement
  token, a balance and an allowance to the logic each at least P + `lf` for
  this order, `lf` being `eravon.terms/1` §6.1's where the quote's `feeUnit`
  is zero and the fee the quote's feeds price now otherwise, which `init`
  prices again (`eravon.terms/1` §8) (a client says so where the bond is
  missing or either sum is short: the case will not open, which a buyer
  about to open the case itself needs to know); and whether the seller has
  revoked the document on the logic (`revoked(sellerKey, termsHash)`,
  `eravon.standard/1` §4), a revoked listing being shown to nobody and
  signed by nobody; whether the listing passes §6.4 with `PAYABLE`, a
  listing that answers otherwise being paid by nobody; and whether each
  signature the client hands the standard recovers to that party's current
  key (`eravon.standard/1` §9), a buyer's client opening no case itself
  under a signature by a key the seller has moved from, whose case the
  seller's current key opens by its own call, and paying under an order that
  admits its own `init` (`buyerMayOpen`) only where its candidate's
  signature is by the seller's current key or the registry holds that
  candidate's offer digest under `sellerKey`; and a client holding a party's
  signature by a key that party has moved from handing that slot empty and
  sending `init` or `settle` only once the registry holds the digest under
  that party or the caller is that party's current key (`eravon.terms/1`
  §5.4, §8): the buyer's client when it sets `openBy` (`eravon.terms/1`
  §5.5) and the seller's when it signs or posts an offer, each again before
  an `init` it sends, reading the chain and the registries then, and the
  contract at `init`.
- **Hashed segments.** A segment the document carries as a hash is a
  text the seller hands the buyer apart from the listing; the client shows it only once its `sha256` is the document's `goodsHash`
  or `deliveryHash`, keeps it
  in the case pack (§5.3), and hands it to the judge on request, who
  compares it the same way; a buyer without the text decides whether to
  sign without it.
- **Money.** Whether the amount reached the instance, whether `init` was
  sent, by whom, and whether it succeeded: the chain. An instance funded above its amount has the surplus credited to the
  buyer's payout at `init`, pushed by `sweep` or with the terminal
  (`eravon.standard/1` §4, §8), and one funded below it waits, as any
  instance does, until more arrives and an `init` opens it, or until an
  `init` called after its `openBy` or its offer's `expiry` or after a
  revocation, or the buyer's `cancel`, returns the money
  (`eravon.standard/1` §4).
- **Imports.** How a client imports a pack (a file the buyer chooses or
  drops) and what it asks before importing: the client.
- **The buyer's countersignature.** A settlement the buyer accepts is
  signed by the buyer over the same digest, or consented to by call
  (§8.5), and submitted with whatever signatures the two consents carry;
  this law binds the seller's half and the contract decides the rest.
- **Consent by call.** Whether a by-call candidate's offer, a by-call
  receipt's order, or a by-call settlement is consented to: the standard, as
  `eravon.terms/1` §8 states for each slot. Before the standard, the buyer's
  client and the seller's, reading the logic's registry (`consented(party,
  digest)`, the digest being §2.4's under the logic's domain for a
  candidate, §4.4's `orderDigest` for a receipt, and §8.3's for a
  settlement) at a `logic` they have already matched to a declared standard,
  and taking a digest as registered only where every endpoint they read
  answers that the registry holds it, never on one endpoint's word and never
  on an answer that it does not. A buyer's client shows a by-call candidate,
  and so signs or registers an order on it, only where the offer's digest is
  registered under `sellerKey`, and consents to a by-call settlement, by
  countersigning it or by registering its digest, only where the
  settlement's digest is registered under `sellerKey`: a consent given
  before that binds the buyer, for an order until its `openBy` or the
  offer's `expiry`, whichever is earlier, and only until the buyer's
  `cancel` (`eravon.standard/1` §4), and for a settlement until its `expiry`
  or the case's terminal, while the seller alone chooses whether to complete
  it, by its own `init` or `settle` or by registering later.
- **Reading the chain for §6.** A reader takes §6.1's records from block 0
  to the head it read: the board's `Posted` logs from an account whose code
  hash is `boardCodeHash`, the logic's logs from a `logic` it matched to a
  declared standard, the logs of every account whose code hash is that
  standard's `proxyCodeHash`, and the `Declared` and `Acceded` logs of the
  succession book §1 names, never mixing two chains' records; it resolves
  only the addresses put before it (the seller of a link it opens, the
  judges and guilds it chooses among, the other party of its own cases),
  walks a record's history only where a person asks for it, and never puts
  that walk in the path of another reading.
- **Stages.** Listings whose documents carry one `series.id`
  (`eravon.terms/1` §2.3) are one sale to every client, and their orders are
  chained by `prev` (`eravon.terms/1` §4.4): a buyer's client signs or
  exports the stages in the documents' order under one `buyerKey`, a zero
  `prev` at `stage` 1 and each later `prev` the instance of the order it has
  just signed or exported, funds every stage's instance, hands the seller
  every receipt, and shows the buyer that a later stage's money comes back
  only once the stage before ends leaving the buyer a share
  (`eravon.standard/1` §4a), or by the stage's own expiry branch (its
  `openBy`, its offer's `expiry` or a revocation, `eravon.standard/1` §4
  step 3); a seller's client takes the receipts of a series only where the
  documents name one `chainId`, one `logic` and one `sellerKey`, agree in
  `stages` with their `stage`s running 1 to `stages`, and the receipts name
  one `buyerKey` (where the seller's record moved, a later stage's listing
  is one posted while the former key was current and naming it, §6.4, its
  candidate honoured by the current key's own `init` or by the digest that
  key registers under the former, as Rotation states; a stage listed after
  the move, naming the former key, is paid by nobody under §6.4, so a seller
  lists every stage of a series before it moves its record), the `prev`s
  chain the stages in that order from a zero `prev` at `stage` 1, and each
  later stage's `openBy` falls after the latest second at which the stages
  before it can still end under their windows and quotes
  (`eravon.standard/1` §5); it takes a receipt with a nonzero `prev` only as
  a later stage of a series whose earlier receipt it holds, `prev` being the
  instance of that receipt's order, and one with `prev` zero only for a
  listing with no `series` or at `stage` 1; and it begins a stage only once
  the standard has opened it; a judge's desk shows the stages of one series
  as one matter; where the guild names no router the standard seats the
  judge the stage before's `stage()` answers, the series judge of
  `eravon.standard/1` §4a, on the conditions of its §7, and a guild that
  names a router and wants one judge for a whole series makes it draw by the
  parties `RouteView` carries, the router being the guild's affair
  (`eravon.standard/1` §7, §4a). The lock both parties rely on is the
  standard's, not the client's: a later stage opens for nobody before the
  stage before is paid, and is cancelled by nobody before that stage fails
  (the lock being the order's that names `prev`; a document opened under
  another order, as `buyerMayOpen` admits, is no stage of the series,
  `eravon.terms/1` §8); a client reads a stage's events from the instance's
  address its order derives (`eravon.standard/1` §1, §10), never by `caseId`
  across emitters.
- **Address collision.** Two orders a buyer signs under one offer with one
  key, one payout, one `openBy` and one `prev` are one order and one
  instance (`eravon.terms/1` §4.4, `openBy` and `prev` being part of the
  salt): a buyer's client signs again at a later second, and where the
  offer's `expiry` fixes `openBy` whatever the second (the second of signing
  plus `openWindow` not before `expiry`), it refuses by name and sends
  nothing, the buyer asking the seller for another version.
- **Rotation.** A seller or a buyer that continues its record moves it on
  the succession book, the current key's `declare` and the new key's
  `accede` (`galeed.roster/1` §4), on the book of every chain the family
  declares one for, whether or not the record has left anything there
  (`galeed.roster/1` §7); one that does not continue takes a new key that
  enters no book, its former key staying the current key of its own record
  for the cases it opened. Before it moves, a client lists what the move
  changes: every case in flight is acted on by the new key from the
  accession on; every listing posted while the former key was current stays
  payable (§6.4), its cases opening by the new key's own call; every
  signature the former key handed out that nobody submitted (a receipt of an
  order paid and not opened, an action message, a settlement offer) is
  refused from the accession on, and a buyer's client registers each paid
  and unopened order's digest from the new key under the former
  (`consent(former, orderDigest)`, `eravon.standard/1` §9) or lets it run
  out to its refund; a funder's bond for the former key keeps serving every
  offer naming that key, and a funder of offers naming the new key binds the
  new key (`eravon.standard/1` §7a); a seller's client registers from the
  new key under the former the offer digest of every candidate of a listing
  still on sale whose document admits the buyer's own `init`
  (`buyerMayOpen`), whose buyer, holding no receipt the seller sees, would
  otherwise open nothing, or revokes the document; the fee of a dispute is
  pulled from `buyerKey`'s own address and a deposit from the offer's
  `sellerFund`, whatever key acts (`eravon.standard/1` §7a, §8), so that a
  buyer's client whose record moved transfers the fee to the instance after
  the case opens and before it disputes (`eravon.terms/1` §5.5), and a
  seller's client keeps the deposits of the offers its former key signed at
  their `sellerFund`. After the move, a party fills with no signature a slot
  a predicate of this law verifies against its former key (a receipt naming
  it as `buyerKey`, §4.4; a settlement offer in a case whose listing names
  it as `sellerKey`, §8.3): it leaves the slot empty and registers the
  digest from its current key under the former, or acts by its own call
  (`eravon.terms/1` §5.4, `eravon.standard/1` §9); an action message it
  submits itself, which no document of this law carries, it signs with its
  current key. A client tells the person, before it continues a record, that
  every earlier act of the record becomes readable under it by anyone, and
  that this cannot be withdrawn.

---

## 11. Freeze and evolution

**11.1** The reference implementation freezes when independently written
implementations, produced from this text, `eravon.terms/1`, the passages of
`galeed/1` that this text and `eravon.terms/1` invoke, and the three harness
contracts they rest on (the `galeed/1` harness, HARNESS-TERMS.md,
HARNESS-LINK.md) alone, agree with it on every input of a generated corpus
(every token of §9 with its index, every carrying token of §9 once, and once
with each carried token for the carriers §11.1 lists below, every transport
decoding and every transport encoding this law defines, every verdict of
`resolveLink` with a listing and a settlement offer resolved, a document of
another `v` refused, a document of each kind whose chain differs from the
link's refused, and a document whose bytes miss the hash refused, every
verdict of the three bindings (§4.4, §7.1 and §8.3, the comparison verdicts
of §7.1 as `E_LISTING_BIND` carries them), the two by-call verdicts
included, every by-call candidate a listing lists with its digest (a by-call
judge and a by-call guild, a list of at least two, and a by-call candidate
followed by one that `E_LISTING_SIG` refuses), the string `0x` once in each
of the four slots that admit the empty string (a judge's and a guild's
`sig`, the receipt's `sig`, the settlement's `sig`), a listing whose
document carries each segment once as text and once as a hash, a pack with
no `attachments`, with an empty array, with an attachment for each hashed
segment, with two attachments of one text, and with each of §5.3's three
refusals, every salt, instance, and digest `RECEIPT_BOUND` or
`RECEIPT_BY_CALL` carries, every digest `SETTLE_BOUND` or `SETTLE_BY_CALL`
carries, every position of §6.2 from each of its three sources and none, a
repost placing nothing, every verdict of §6.4, and a history of §6.5 over a
record of one key, over a record of three keys with an attempt, and over a
key that acceded with acts before its accession), with every closed boundary
of this text witnessed on both sides by at least one corpus input under
every seed the corpus is generated with, the boundary list being a
precondition on the corpus and never a property claimed of it afterwards;
the corpus carries the list beside itself. The carriers the precondition
takes in pairs are `E_LISTING_SIG`, `RECEIPT_SIG`, and `SETTLE_SIG` (each
with every token §9 says it can carry), `RECEIPT_ORDER` (with
`E_ORDER_BUYER`), and `E_LISTING_BIND` (with each verdict §9 says it can
carry); every other carrying token of §9 carries another section's or
another document's whole vocabulary and is witnessed once, carrying whatever
it carried there. The carried tokens §2.5, §5.2, §8.3, and §9 name as
unreachable state no boundary in that sense. No `[O]` rule is in the corpus
and the freeze certifies none. The fuzzing `galeed/1` §12.1 asks of its core
is deferred here as `eravon.terms/1` §10.1 defers it; §11.5 records beside
every digest it names whether that fuzzing was run over the prefix and
base64url scans of the two transports (§3.2, §4.3), and with what result, a
naming that records none being a naming under deferral. The canonical form
this law's documents are read under is witnessed on every input of
`galeed-conformance/corpus/canon/`, the `galeed/1` corpus's `check` set as
generated at seed 1 and committed, each carried as a listing file and
predicted as that corpus predicts it, a canonical value going on to §2.5's
root and members, so that every boundary of `galeed/1` §3 that corpus
witnesses, its first test over the whole input among them, is witnessed on a
surface where no test of this law decides before it.

**11.2** Once §11.5 names a release digest, agreement with the frozen core
on the corpus is the test anyone can run in seconds, and it is evidence
of conformance and never its definition.

**11.3** `eravon.listing/2`, `eravon.link/2`, `eravon.receipt/2`,
`eravon.pack/2`, and `eravon.settle/2` are new version literals, and the
transports of the link and the receipt carry new prefixes; a document
accepted under this law is accepted under it forever, and a later law
reads it by this one.

**11.4** This law, `eravon.terms/1`, `galeed/1` and `galeed.roster/1` are
four laws: a change to one is never a change to another, and each names its
own core. This law reads every passage of a GALEED law it cites as it stands
when §11.5 names this law's core, `galeed.roster/1` §4.2 among them, which
it restates in its own §1 and embeds no roster core for, so that a later
text of any such passage changes nothing here until this law's own revision
follows it. This law's core embeds the `eravon.terms/1` core of release digest

`0x4984258d42de40eb19e51a8291cb9b572d9b9ab184e12e7eda19893a61dcd0a9`

(the one that law's §10.5 named when this section was pinned); a link core built on any other terms core is
another core. A recast of `eravon.terms/1` changes nothing here by itself:
this law moves to another terms core only by its own §11.5 act, which
re-pins this section and names the link core built on it in one act.

**11.5 The core's identity.** Naming a release digest here is the freeze,
and it is the only act that performs it. The release digest of the
reference implementation in `eravon-conformance/link-py/` (`erl.py`,
`erllaw.py`; the language's standard library and the `eravon.terms/1`
core, nothing else), as `criterion-digest-link.sh` computes it, is

`0x25eb3810d9a870633608d36054a8399e1445435caa703c7ed5e8fd1bae8f11dc`

named on 2026-10-27 and named again on 2026-11-16, the same digest, so that
§11.4 reads the GALEED passages it cites as they stand on 2026-11-16; the
review record beside this text (docs/eravon-link-v1-review.md) records how
it converged. The fuzzing of this core was the three implementation seats'
differential probing across the same six rounds (the inputs counted at
`eravon.terms/1` §10.5), over the receipt's every hex member under every
break of its form, `prev` missing, misplaced, doubled and signed over
another `prev`, a later stage's receipt, pack and settlement under every
candidate and both consent forms, and the listings of a series, with zero
disagreements; the closing round's first measurement, the programs
unchanged, was zero at seeds 1 to 6 over 4,505 cases per seed. The fuzzing
of the deferral over the core named on 2026-10-19
(`0x29ff5e4651ee99cdc94521157ef96ad1e943473bea645cc24a569fb4429c80ef`) was
run across the five rounds of the revision of 2026-10-17 and the two of
2026-10-18 and 2026-10-19: the two transports' prefix and base64url scans
(`decode` and `receipt-decode` at every residue modulo 4 and at eight
lengths with every last character, stray bytes at the end and in the middle,
eight prefix spellings, the standard and the URL alphabets, padding and
trailing bits, `encode` and `receipt-encode` over documents whose bytes
carry both URL characters, by the Rust, Node and Python seats at seeds 1 to
6), zero disagreements with that core; the three seats' 1,634,961
differential inputs over the seven rounds are counted at `eravon.terms/1`
§10.5.
