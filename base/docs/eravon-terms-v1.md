# ERAVON terms v1 (`eravon.terms/1`)

The terms law of ERAVON: the first of the three document laws a party's
tooling needs beside the standard contract, so that a seller's client, a
buyer's client, and a judge's desk agree byte for byte on what a terms
document is, on the offer a seller signs, on the order a buyer signs, on the
address the money goes to before any code exists, on the two consents the
contract's `init` will check, each a signature or a call (§5.4), and on the
action messages the parties sign afterwards. Where
[`eravon.standard/1`](eravon-standard-v1.md) says what the standard does
with these values, `eravon.terms/1` says what the values are and how each is
derived from a document, and it reads [`galeed/1`](galeed-v1.md) for
canonical form, hashing and secp256k1 through an implementation its own core
carries (§10.4).

**Status.** The law is the set of total predicates in this text. A
conforming implementation agrees with the core named in §10.5 of this text
on every input. Evolution goes through `eravon.terms/2`.

**Rule classes.** Two classes appear, and every rule below is marked; a
passage that states no predicate carries no mark, §2.6, §3.3, §4.1, and
§6.2 being such passages, §4.1 defining the shape of a value that §4.3
builds and §2's tests have already decided:

- **[D]** document predicates: decided offline from the byte strings and
  the arguments the predicate is given, and nothing else. No predicate of
  this law consults a chain, a clock, a price feed, or a registry; what a
  contract will do with a value is the standard's business, and what a
  price feed says is read once by the contract at `init` and by nobody
  here.
- **[O]** producer obligations: rules a producer keeps, decided by no
  predicate of this law, so that a verifier tests none of them. §5.5's
  determinism clause is the obligation the harness's producer commands
  realise, and the corpus measures those commands' output; every other
  clause of §5.5 is carried in no answer and measured by no corpus.

---

## 1. Notation

`galeed/1` §1 applies: byte strings, `hex20`, `hex32`, `hex65`, tokens,
skeleton and prose strings, `int`, `bool`, bytewise order, `sha256`,
`keccak256`, secp256k1 and its order `n`, UTF-8, Unix seconds. The preamble
of `galeed/1` §6 applies to every member table of this law: `required` means
the key is present, `optional` means the key may be absent, a key present
with any value, `null` included, is a member present, and where a rule says
a member equals a literal, an address, or a digest it means the value is a
string byte-equal to that literal or to the value's spelling under §1's hex
functions, and `exactly one` over a pair of keys means that one of the two
is present and the other absent. In addition:

- **`canonical(b)`**: `galeed/1` §3.5 alone, all six of its tests,
  yielding the value or one of the eight tokens that section can name.
  `galeed/1` §3.3's charsets apply inside it exactly as the parent states
  them, subtrees included: a member whose key is `_md` or ends in it opens
  a prose subtree where it sits, its own string value and every string value at
  any depth within it included, and every other string value of a
  document of this law, at any depth, is a skeleton string before any
  table below reads it.
- **`dec256`**: a string of decimal digits, no sign, no leading zero (the
  integer zero is written `0`), whose value lies in [0, 2^256 − 1]. It
  spells a `uint256` of the contract. It is a string and never a JSON
  number, since `galeed/1` §3.2 bounds numbers at 2^53 − 1 and a token
  amount in minimal units passes that bound in ordinary use. `dec256(x)`
  writes the integer `x` in this form.
- **`zero`**: an address or a digest is zero when every byte of it is
  `0x00`; `nonzero` is its negation.
- **`word(x)`**: the 32-byte big-endian encoding of an integer `x` in
  [0, 2^256 − 1]; `word(a)` for a 20-byte address is 12 zero bytes then
  the address.
- **`u32`**: an `int` at most 4294967295.
- **`pointer`**: `galeed.roster/1` §6.1's string, exactly the bytes
  `galeed:guild:`, then an `int` spelled as `galeed/1` §3.2 spells one,
  then `:`, then a hex20; its `chainId` is the integer and its `address`
  the hex20. Any other string is no pointer.
- **`base64url`**: RFC 4648 §5 with no padding characters, and canonical:
  the unused trailing bits of the last character are zero.
- **`DEPLOYER`**: the address `0x4e59b44847b379578588920ca78fbf26c0b4956c`,
  the keyless CREATE2 deployment proxy through which `eravon.standard/1`
  deploys its logic and every instance; the one constant address of this
  law.
- **`initcode(L)`** for a 20-byte address `L`: the 55 bytes
  `0x3d602d80600a3d3981f3363d3d373d3d3d363d73` ‖ `L` ‖
  `0x5af43d82803e903d91602b57fd5bf3`, the EIP-1167 minimal proxy's
  creation code with `L` as the implementation. **`runtime(L)`** is its
  last 45 bytes, `0x363d3d373d3d3d363d73` ‖ `L` ‖
  `0x5af43d82803e903d91602b57fd5bf3`, the proxy's runtime code.
  **`proxyCodeHash(L)`** = `keccak256(runtime(L))`, the code hash every
  instance of the standard at `L` carries, and the value a judge declares.
- **`create2(D, s, h)`** = the last 20 bytes of
  `keccak256(0xff ‖ D ‖ s ‖ h)` for a 20-byte address `D`, a 32-byte salt
  `s`, and a 32-byte digest `h`, as EIP-1014 fixes it.
- **EIP-712.** `hashStruct`, `encodeData`, `typeHash`, and the domain
  separator are as EIP-712 fixes them, restated in §4.2 and §4.5 in full
  for the types this law uses, so that no reader needs the EIP to decide
  a digest.

---

## 2. The document

A terms document is one Markdown text a seller writes for one sale: a
fenced machine block the tooling reads, whose prose members carry what the
parties read, hashed as one byte string, so that the words the buyer saw
are the words the hash binds.

**[D] 2.1 Bytes and hash.** A terms document is a byte string `b`. Its
digest is `termsHash(b)` = `sha256(b)` over every byte of `b`, defined for
every byte string as `galeed/1` §2.1 defines identity: whether `b` is a
terms document is §2.4's separate question, so that an offer may commit to
the digest of bytes nobody has read yet and a reader may name a document
that fails to verify.

**[D] 2.2 The machine block.** `b` is valid UTF-8. Its **lines** are the
maximal runs of bytes between line feed bytes (`0x0A`), from the start of
`b` to its end, so that `k` line feeds yield `k + 1` lines, the last
possibly empty; a carriage return is a byte of its line like any other.
The **opening fence** is the line byte-equal to the 17 bytes
` ```eravon.terms/1 ` (three grave accents then the literal, no spaces);
the **closing fence** is the line byte-equal to the 3 bytes ` ``` `.
`b` carries exactly one opening fence among all its lines, and some line
after it is a closing fence; the **block** is the bytes of the lines
strictly between the opening fence and the first closing fence after it,
joined by one line feed each, so that a block of one line carries no line
feed at all and a block of no lines is the empty byte string. Every line
before the opening fence and every line after that closing fence is
empty: a document is its two fences and its block, and what the parties
read stands inside the block as its prose members. A document whose line
endings are `0x0D 0x0A` has no opening fence, since the carriage return
is a byte of every line, and is refused.

**[D] 2.3 Members, closed.** The block is a canonical object under
`canonical(block)` with exactly these members and no others:

| key | presence | form | rule |
|---|---|---|---|
| `v` | required | string | byte-equal to `eravon.terms/1` |
| `chainId` | required | int | the chain the instance will live on; read by the two domains of §4 and by the quote key of §2.5, and compared with every pointer's |
| `logic` | required | hex20 | the standard's logic contract; nonzero; read by `initcode`, by the seller's domain, and by the quote key of §2.5, which no predicate here reads |
| `sellerKey` | required | hex20 | the seller's signing key, or the address of a seller that is a contract (§5.4); nonzero |
| `sellerPayout` | required | hex20 | where the price goes; nonzero |
| `sellerFund` | required | hex20 | the address the seller's deposit is pulled from and returned to: `sellerKey` itself where the deposit sits at the seller's own address, otherwise the address that holds it, bound to the seller on the logic as a company's wallet is (`eravon.standard/1` §4 step 5, §7a); nonzero |
| `token` | required | hex20 | the settlement token; nonzero; every candidate's quote names it, which the tooling reads at the act (§8) |
| `amount` | required | dec256 | the price, in `token`'s minimal unit; nonzero and at most ⌊(2^256 − 1) / 10000⌋ = 11579208923731619542357098500868790785326998466564056403945758400791312963, so that its product with every basis-point value (`eravon.standard/1` §3, §8) fits a `uint256` |
| `deliveryWindow` | required | u32 | seconds the seller has to claim delivery once the judge has accepted the seat (`eravon.standard/1` §5, T1) |
| `confirmWindow` | required | u32 | seconds the buyer has to dispute once delivery is claimed, after which anyone's `advance` releases the price (T2; a `confirm` is taken while the case is Claimed, past T2 included, `eravon.standard/1` §5) |
| `openWindow` | required | u32 | seconds the seller promises to open the case in, counted from the buyer's signing, or, for a buyer that consents by call, from E of §5.5 (its export plus the longest time its consent can take before it can first land); the buyer's client derives the order's `openBy` from it (§4.4, §5.5) |
| `buyerMayOpen` | required | bool | whether the standard's `init` admits a call from `buyerKey`'s current key, as well as from `sellerKey`'s, to open the case (`eravon.standard/1` §4); read by the offer |
| `goods_md`, `goodsHash` | exactly one | prose string, hex32 | what is sold: as text, or as the `sha256` of the UTF-8 bytes of a text the seller hands the buyer and the judge apart from the document (§8, `eravon.link/1` §5) |
| `delivery_md`, `deliveryHash` | exactly one | prose string, hex32 | how it is delivered and what counts as delivered: as text, or as the `sha256` of a text handed apart, as the row above |
| `ladder` | required | array | two to sixteen rungs (§2.5) |
| `judges` | required | array | the named candidates (§2.5); may be empty |
| `guilds` | required | array | the pool candidates (§2.5); may be empty |
| `nonce` | required | int | the seller's document number, at most 2^53 − 1 as `galeed/1` §3.2 bounds an `int`, a narrowing of the contract's `uint256` this law makes deliberately, a nonce being drawn by a seller's own counter; data for readers, read by the offer alone; one document serves every buyer who signs under it, each order being its own case (§4.4) |
| `expiry` | required | int | Unix seconds after which the contract's `init` refuses to open a case, at most 2^53 − 1 as an `int`, the same deliberate narrowing the `nonce` row makes, which no second of this century reaches; data for readers here, read by the offer and by the buyer's client setting `openBy` (§5.5), and read beside the order's `openBy` by the contract (§4.4, `eravon.standard/1` §4 step 3) |
| `compliance_md` | optional | object | the parties' legal identities, tax numbers, governing law, invoice rule, in whatever members the parties chose; members untested; the key ends in `_md` so that every string value inside it, at any depth, is prose under §1 and a name or an address outside U+0020 through U+007E is admitted; the members' keys stay keys under `galeed/1` §3.3 wherever they sit, so a party spells a key in U+0020 through U+007E and puts the language in the value |
| `series` | optional | object | exactly `id` (hex32, nonzero), `stage` (int, at least 1) and `stages` (int, at least `stage`): the sale this document is one stage of, the ordinal of this stage and the number of stages; the documents of one series carry one `id`, their `stage`s running 1 to `stages`; data for clients (§8), read by no predicate but this row's form test |

A document describes one sale with one price and one set of windows: a sale
delivered in stages is as many documents as it has stages, each its own
order and its own case, the documents naming one `series` and the orders
chained by `prev` (§4.1, §8).

**[D] 2.4 Order of decision.** `terms(b)` yields the document or one token,
decided in this order: `b` is valid UTF-8 (`E_TERMS_UTF8`); the lines carry
an opening fence (`E_TERMS_BLOCK`), exactly one (`E_TERMS_FENCES`), and a
closing fence after it (`E_TERMS_UNCLOSED`); every line before the opening
fence and after the first closing fence is empty (`E_TERMS_OUTSIDE`);
`canonical(block)` (its tokens); the root is an object (`E_TERMS`); the
seventeen required members are present (`E_TERMS_MISSING`), then no member
beyond the twenty-three keys the table names (`E_TERMS_CLOSED`); `v`
(`E_TERMS_V`); `chainId` an int (`E_TERMS_CHAIN`); `logic` hex20 and nonzero
(`E_TERMS_LOGIC`); `sellerKey` hex20 and nonzero (`E_TERMS_SELLER`);
`sellerPayout` hex20 and nonzero (`E_TERMS_PAYOUT`); `sellerFund` hex20 and
nonzero (`E_TERMS_FUND`); `token` hex20 and nonzero (`E_TERMS_TOKEN`);
`amount` dec256, nonzero, and at most ⌊(2^256 − 1) / 10000⌋
(`E_TERMS_AMOUNT`); `deliveryWindow` u32 (`E_TERMS_DELIVERY_WINDOW`);
`confirmWindow` u32 (`E_TERMS_CONFIRM_WINDOW`); `openWindow` u32
(`E_TERMS_OPENWINDOW`); `buyerMayOpen` a bool (`E_TERMS_BUYEROPEN`); exactly
one of `goods_md` and `goodsHash` present, and the present one of its form,
`goods_md` a string and `goodsHash` a hex32 (`E_TERMS_GOODS`); exactly one
of `delivery_md` and `deliveryHash` present and of its form likewise
(`E_TERMS_DELIVERY`); `ladder` an array of two to sixteen elements
(`E_TERMS_LADDER`), then each element in order under §2.5's rung table with
its index, then the three rules over the whole ladder in §2.5's order;
`judges` an array (`E_TERMS_JUDGES`), then each element in order under
§2.5's judge table with its index, then no judge address repeated
(`E_TERMS_DUP_JUDGE`, carrying the index of the first element whose address
an earlier element already carries); `guilds` an array (`E_TERMS_GUILDS`),
then each element in order under §2.5's guild table with its index, then no
guild address repeated (`E_TERMS_DUP_GUILD`, carrying the index of the first
element whose address an earlier element already carries); the two arrays
not both empty (`E_TERMS_NO_CANDIDATE`); `nonce` an int (`E_TERMS_NONCE`);
`expiry` an int (`E_TERMS_EXPIRY`); `compliance_md`, where present, an
object (`E_TERMS_COMPLIANCE`); `series`, where present, an object of exactly
`id`, a nonzero hex32, `stage`, an int at least 1, and `stages`, an int at
least `stage` (`E_TERMS_SERIES`). A byte string that passes is a terms
document; nothing else is.

**[D] 2.5 Element tables.** Each is decided in the order its rows are
written, the element an object first, then its required members present,
then no member beyond its list, then the rows.

A **rung** (token prefix `E_RUNG`): an object (`E_RUNG`); `buyerBps`,
`sellerBps`, `feeBearer` required and `note_md` optional
(`E_RUNG_MISSING`, then `E_RUNG_CLOSED`); `buyerBps` an int at most 10000
(`E_RUNG_BUYER`); `sellerBps` an int (`E_RUNG_SELLER`); `feeBearer` an
int at most 2 (`E_RUNG_BEARER`: 0 the buyer bears the litigation fee, 1
the seller, 2 half each, the odd unit of an odd fee borne by the buyer,
`eravon.standard/1` §8); `note_md`, where present, a string
(`E_RUNG_NOTE`); `buyerBps + sellerBps` equal to 10000 (`E_RUNG_SUM`).
Then over the whole ladder: the first rung's `buyerBps` is 0
(`E_LADDER_FIRST`), the last rung's `buyerBps` is 10000 (`E_LADDER_LAST`),
and no rung's `buyerBps` is below its predecessor's (`E_LADDER_ORDER`,
carrying the index of the first rung that is). The rungs are the closed
set of outcomes a judge may choose from; rung 0 dismisses the claim and
the last rung refunds in full (`eravon.standard/1` §5, §8).

A **judge** (token prefix `E_JUDGE`): an object (`E_JUDGE`); `judge`,
`amanahHash`, `quoteHash` required and `source` optional
(`E_JUDGE_MISSING`, then `E_JUDGE_CLOSED`); `judge` hex20, nonzero, and
not byte-equal to `sellerKey` (`E_JUDGE_ADDRESS`); `amanahHash` hex32 and
nonzero (`E_JUDGE_AMANAH`: the `entry_id` of the judge's `amanah` entry,
the `amanah.entry_id` of the judge document `galeed.roster/1` §3a
publishes); `quoteHash` hex32 and nonzero (`E_JUDGE_QUOTE`: the
`keccak256` of the quote bytes that entry's `quotes` member (`galeed/1`
§6.2) carries under the key `<chainId>:<codeHash>:<token>`, `chainId`
being the document's, `codeHash` `proxyCodeHash(logic)` and `token` the
document's, the one key of
`galeed.roster/1` §5.10 an ERAVON reader reads (`eravon.standard/1` §3),
the bytes themselves travelling in no document of this family); `source`,
where present, a pointer whose `chainId` equals the document's `chainId`,
its address untested (`E_JUDGE_SOURCE`: the guild whose candidate set the judge was taken
from, data for readers).

A **guild** (token prefix `E_GUILD`): an object (`E_GUILD`); `pointer`,
`amanahHash`, `quoteHash` required (`E_GUILD_MISSING`, then
`E_GUILD_CLOSED`); `pointer` a pointer whose address is nonzero and not byte-equal to
`sellerKey`, and whose `chainId` equals the document's `chainId`
(`E_GUILD_POINTER`);
`amanahHash` hex32 and nonzero (`E_GUILD_AMANAH`: the `sha256` of the
UTF-8 bytes of the `text_md` of the guild's covenant document,
`galeed.roster/1` §3b.3); `quoteHash` hex32 and nonzero
(`E_GUILD_QUOTE`: the `keccak256` of the quote bytes that document's
`quotes` member carries under the key the judge row names). A guild's address is
its pointer's address, which is what §2.4 tests for a repeat after every
element has passed.

**2.6 Meaning.** The block is what the tooling derives the offers from
(§4.3) and what the parties read, the two being one object under one
digest. Nothing stands outside the fences but empty lines, so a document
carries one machine block and no text that escapes it, and a second
opening fence refuses it. `nonce` is read by nobody here but
the offer, and `expiry` by the offer and by §5.5; a document past its `expiry` is a terms document all the same,
and whether a case can still open under it is the contract's answer. A
candidate's `amanahHash` and `quoteHash` are copied by the seller from
the chain at the moment of writing, from the judge's `amanah` entry or
the guild's covenant document; the quote bytes themselves stay where they
were published, and a reader fetches them there and compares their
`keccak256` with the copy (§8). Whether the copy is current and whether
the candidate still stands are questions for the tooling at the moment of
use (§8). A segment carried as a hash (`goodsHash`, `deliveryHash`) is
read by the parties as a text the seller hands them apart from the
document, each comparing its `sha256` with the member; the document binds
that text through the hash as it binds a text carried in the clear.

---

## 3. The quote

A quote is the byte string a judge or a guild publishes for one standard
and one token, encoded as the contract's `quoteType()` fixes it; the
document carries its `keccak256` (§2.5), the contract pins it, and the
tooling reads the bytes where the judge or the guild published them (§8).

**[D] 3.1 Layout.** A quote `q` is exactly 480 bytes: fifteen 32-byte words
`w0` through `w14`, the static ABI encoding of the tuple

    (address token, uint16 premiumBps, address feeUnit, address feedToken,
     address feedUnit, uint32 maxStale, uint256 feeFixed, uint16 feeBps,
     uint256 feeMin, uint256 feeMax, uint256 amountCap, uint32 rulingWindow,
     uint32 offerWindow, uint8 lapseDefault, bool autoAccept)

in that order and with no tuple wrapping, each word read as a big-endian
integer. `quoteHash(q)` = `keccak256(q)`.

**[D] 3.2 Decoding, total.** `quote(q)` yields the fifteen fields or one
token, decided in this order, each clause naming the fault it fails: `q` is
480 bytes long, or `E_QUOTE_LEN`; then each word in index order, the first
word that fails its clause naming `E_QUOTE_FIELD` with its index: `w0`
(`token`), `w2` (`feeUnit`), `w3` (`feedToken`), `w4` (`feedUnit`) each have
their first 12 bytes zero (an address word), and `w0` is nonzero; `w1`
(`premiumBps`) and `w7` (`feeBps`) are at most 10000; `w5` (`maxStale`),
`w11` (`rulingWindow`), `w12` (`offerWindow`) are at most 4294967295; `w6`
(`feeFixed`), `w8` (`feeMin`), `w9` (`feeMax`), `w10` (`amountCap`) are any
word and fault on no input; `w13` (`lapseDefault`) is 0, 1 or 2; `w14`
(`autoAccept`) is 0 or 1; then `feeUnit` is nonzero, or `feedToken` and
`feedUnit` are both zero, or `E_QUOTE_FEED`. A byte string that passes
decodes; nothing else does.

**3.3 Meaning.** `token` is the settlement token the quote is for, and a
document names a quote for its own token alone (§2.3's `token` row and §8;
the contract refuses another at `init`, `eravon.standard/1` §3).
`premiumBps` is the standing fee rate over the price; `feeFixed`, `feeBps`,
`feeMin`, `feeMax` compute the litigation fee; `amountCap` bounds the price
the judge will take, zero meaning no bound. Where `feeUnit` is zero the four
absolute amounts are in `token`'s minimal unit and §6 computes the fee
offline; where it is nonzero they are in `feeUnit`'s minimal unit and the
contract converts once at `init` from the feeds the quote names (§8).
`rulingWindow` and `offerWindow` are the judge's clocks (on the guild track
each draw is also bounded by its arming time plus `offerWindow`, a draw sent
in the block after the arming admitted whatever that bound,
`eravon.standard/1` §7), `lapseDefault` the buyer's share of the price where
the case lapses (the judge silent through the ruling window, a resignation
during the dispute, or a dispute after the seat resigned,
`eravon.standard/1` §5, §6, §8): 0 none of it, 1 half of it (5000 basis
points), 2 all of it, the seller taking the rest and the rungs deciding
nothing there; and `autoAccept` whether a seating carries consent. A quote
carries no payout: a judge registers where its earnings go on the instance
itself (`eravon.standard/1` §8, `galeed.venue/1` §3).

---

## 4. The offer, the order, and the actions

**4.1 The offer value.** An offer is the tuple

    (sellerKey, judge, guild, sellerPayout, sellerFund, token, amount,
     deliveryWindow, confirmWindow, openWindow, buyerMayOpen, ladder,
     termsHash, amanahHash, quoteHash, nonce, expiry)

of six addresses, a uint256 `amount`, three uint32 windows, a bool, a ladder
of two to sixteen rungs each `(buyerBps, sellerBps, feeBearer)` satisfying
§2.5's per-rung rows and its three ladder-wide rules alike, three 32-byte
digests, and two `int`s `nonce` and `expiry` (each a `uint256` of the
contract, at most 2^53 − 1 here as §2.3 bounds them); `amount`, `termsHash`,
`amanahHash`, `quoteHash`, `sellerKey`, `sellerPayout`, `sellerFund`, and
`token` are nonzero, `amount` is at most ⌊(2^256 − 1) / 10000⌋ (§2.3), and
exactly one of `judge` and `guild` is nonzero and that one is not byte-equal
to `sellerKey` (§2.5; a named seat names a judge, a pool seat names a
guild). An order is an offer beside two nonzero addresses `buyerKey` and
`buyerPayout`, `buyerKey` byte-equal to none of `sellerKey`, `judge`, and
`guild`, an `int` `openBy` (a `uint64` of the contract, bounded here as
every `int` is), the Unix second after which the contract's `init` refuses
to open the case (§4.4), and an address `prev`, zero or the instance of the
stage before this one (§4.4, `eravon.standard/1` §4a). This section states
the shape of a value and no token of its own: §4.3 builds an offer whose
shape §2's tests already decided, §4.4 builds an order from an offer, and a
law that builds either from its own members names in its own text the tokens
by which it refuses a tuple outside this shape before §4.2 reads it.

**[D] 4.2 Typed data.** Four type strings, byte for byte:

    Outcome(uint16 buyerBps,uint16 sellerBps,uint8 feeBearer)

    Offer(address sellerKey,address judge,address guild,address sellerPayout,address sellerFund,address token,uint256 amount,uint32 deliveryWindow,uint32 confirmWindow,uint32 openWindow,bool buyerMayOpen,Outcome[] ladder,bytes32 termsHash,bytes32 amanahHash,bytes32 quoteHash,uint256 nonce,uint256 expiry)Outcome(uint16 buyerBps,uint16 sellerBps,uint8 feeBearer)

    OrderTerms(Offer offer,address buyerKey,address buyerPayout,uint64 openBy,address prev)Offer(address sellerKey,address judge,address guild,address sellerPayout,address sellerFund,address token,uint256 amount,uint32 deliveryWindow,uint32 confirmWindow,uint32 openWindow,bool buyerMayOpen,Outcome[] ladder,bytes32 termsHash,bytes32 amanahHash,bytes32 quoteHash,uint256 nonce,uint256 expiry)Outcome(uint16 buyerBps,uint16 sellerBps,uint8 feeBearer)

    EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)

each type's hash being `keccak256` of its string as ASCII. Then:

- `hashStruct(rung)` = `keccak256(typeHash(Outcome) ‖ word(buyerBps) ‖
  word(sellerBps) ‖ word(feeBearer))`.
- `hashStruct(offer)` = `keccak256(typeHash(Offer) ‖ word(sellerKey) ‖
  word(judge) ‖ word(guild) ‖ word(sellerPayout) ‖ word(sellerFund) ‖
  word(token) ‖
  word(amount) ‖ word(deliveryWindow) ‖ word(confirmWindow) ‖
  word(openWindow) ‖ word(buyerMayOpen) ‖
  keccak256(hashStruct(rung_0) ‖ … ‖ hashStruct(rung_k)) ‖ termsHash ‖
  amanahHash ‖ quoteHash ‖ word(nonce) ‖ word(expiry))`, the ladder's
  element hashes concatenated in rung order, the three digests written as
  their 32 bytes, and `word(buyerMayOpen)` being `word(1)` for `true` and
  `word(0)` for `false`.
- `hashStruct(order)` = `keccak256(typeHash(OrderTerms) ‖ hashStruct(offer)
  ‖ word(buyerKey) ‖ word(buyerPayout) ‖ word(openBy) ‖ word(prev))`.
- `domain(chainId, V)` = `keccak256(typeHash(EIP712Domain) ‖
  keccak256("ERAVON") ‖ keccak256("1") ‖ word(chainId) ‖ word(V))` for a
  chain id and a verifying contract `V`.
- `digest(chainId, V, s)` = `keccak256(0x1901 ‖ domain(chainId, V) ‖ s)`
  for a struct hash `s`.

The seller's digest of an offer under a document with chain `chainId`
and logic `L` is `offerDigest` = `digest(chainId, L, hashStruct(offer))`:
the logic's address, since no instance exists when the seller signs and
the same offer must not be replayable under another standard. The buyer's
digest of an order is `orderDigest` = `digest(chainId, instance,
hashStruct(order))` with `instance` as §4.4 derives it: the instance's
own address, which the contract recomputes as `address(this)`.

**[D] 4.3 The document's offer.** `offer(T, c)` over a terms document
`T` (§2) and a candidate number `c` yields an offer value or one token:
`c` is not below the sum of the lengths of `judges` and `guilds`
(`E_OFFER_CANDIDATE`), the candidates being numbered judges first in
array order, then guilds. Otherwise the offer is: `sellerKey`,
`sellerPayout`, `sellerFund`, `token`, `amount`, `deliveryWindow`,
`confirmWindow`, `openWindow`, `buyerMayOpen`, `nonce`, `expiry` from the
document;
`ladder` the document's ladder with each rung's `note_md` dropped;
`termsHash(T)`; for a judge candidate `judge` = its `judge`, `guild` =
zero, `amanahHash` = its `amanahHash`, `quoteHash` = its `quoteHash`; for
a guild candidate `guild` = its pointer's address, `judge` = zero, and
the other two likewise from the element. Whether the quote `quoteHash`
names caps the price below `amount` is read by nobody here: the tooling
reads the quote bytes at the act and the contract tests the cap at `init`
(§8).

**[D] 4.4 The order and the instance.** `order(offer, chainId, L, buyerKey,
buyerPayout, openBy, prev)` over an offer value, the chain id and logic of
the document or law it came from, two strings, a value and a string yields
the order's digests or one token: `buyerKey` is not a nonzero hex20, or is
byte-equal to `sellerKey`, to `judge`, or to `guild` (`E_ORDER_BUYER`);
`buyerPayout` is not a nonzero hex20 (`E_ORDER_PAYOUT`); `openBy` is not an
`int` (`E_ORDER_OPENBY`); `prev` is not a hex20, zero admitted
(`E_ORDER_PREV`). Otherwise `salt` = `hashStruct(order)`; `caseId` = `salt`;
`instance` = `create2(DEPLOYER, salt, keccak256(initcode(L)))`;
`orderDigest` = `digest(chainId, instance, salt)`. The salt is the order's
own struct hash, which is how the instance's address proves itself
(`eravon.standard/1` §1). `openBy` is the buyer's: the second after which
money sent under this order is refunded rather than held for a case
(`eravon.standard/1` §4), set when the buyer signs the order or, consenting
by call, exports it, as §5.5 states; it is part of the salt, so two orders
by one buyer under one offer that differ in it alone are two cases. `prev`
is the buyer's too: zero for a sale of one stage and for the first stage of
a series, and the instance of the stage before for every later stage, which
the buyer's client computes from that stage's own order before it signs or
exports this one (§8), under the same two keys; the standard opens a stage
only once the stage at `prev`, an instance of the same logic on the same
chain naming this order's `sellerKey` and `buyerKey`, has ended leaving the
buyer no share of its amount, and lets the buyer cancel it only once the
instance at `prev`, whatever keys it names, has ended leaving the buyer a
share (`eravon.standard/1` §4a).

**[D] 4.5 Action messages.** Around a case the parties sign six kinds of
message under the instance's domain, which the contract's `…BySig`,
`settle`, and `cancel` functions verify. Six type strings, byte for byte:

    ClaimDelivery(bytes32 manifest)

    Confirm()

    Dispute()

    Refund()

    Settle(uint16 buyerBps,uint256 expiry)

    Cancel()

each type's hash being `keccak256` of its string as ASCII, an empty member
list hashing the string with its empty parentheses. `hashStruct(action)` for
a message of type `t` among the six = `keccak256(typeHash(t) ‖ the members'
words in order)`, a `bytes32` member written as its 32 bytes and an empty
member list contributing nothing; `actionDigest` = `digest(chainId,
instance, hashStruct(action))` with the instance of §4.4. An action is read
over an order: whoever decides an action under a document, a candidate, and
an order's arguments decides §2.4, §4.3, and §4.4 first and this section's
argument tests and party rule after them, so that an order §4.4 refuses
answers §4.4's token whatever the action. `action(name, args)` over a name
of the six and its arguments yields the message's struct hash or one token,
in this order: for `ClaimDelivery`, `manifest` is a nonzero hex32
(`E_ACTION_MANIFEST`); for `Settle`, `buyerBps` is an int at most 10000
(`E_ACTION_BPS`) and `expiry` an int, at most 2^53 − 1 as every int, a
narrowing of the contract's `uint256` this law makes as §2.3 makes it for an
offer's `expiry`, which every caller hands in as one; `Confirm`, `Dispute`,
`Refund`, and `Cancel` take no argument and fault on nothing. Who signs
which: `ClaimDelivery` and `Refund` the seller's key, `Confirm`, `Dispute`,
and `Cancel` the buyer's key (§5.2), `Settle` both, each over the same
digest; of the six, `Settle` alone admits the second form (§5.4), on either
side. A verifier handed an action, a party, and a signature answers
`E_ACTION_PARTY` where the message does not admit that party, decided after
this section's argument tests and before any test of §5, so that a
`ClaimDelivery` with a zero `manifest` presented as the buyer's answers
`E_ACTION_MANIFEST` and a `Cancel` presented as the seller's answers
`E_ACTION_PARTY`. The messages carry no nonce, the domain binding them to
one instance and the instance's state making each single-shot: `Cancel` is
admitted before the case opens and never after, the other five after and
never before (`eravon.standard/1` §4, §9).

---

## 5. Signatures

**[D] 5.1 The seller's.** `verifyOffer(offerDigest, sig, sellerKey)`: `sig`
byte-equal to the empty string is §5.4's answer and reaches no test here;
otherwise `sig` is hex65 (`E_SIG_FORM`); then `galeed/1` §5.4 and §5.5 over
the 65 bytes and `offerDigest` verbatim, the digest here being the EIP-712
digest of §4.2 in place of the parent's EIP-191 digest and wrapped no
further, with the recovered address required byte-equal to `sellerKey`:
`E_SIG_V`, `E_SIG_RANGE`, `E_SIG_HIGH_S`, `E_SIG_RECOVER`, `E_SIG_SIGNER`,
in the parent's order and with its meanings. A signature that passes is the
assent to that offer of the key the predicate is handed, and no other value
of `sig` is that key's assent, the empty string being §5.4's claim of
assent, which the chain decides (§8); whether that key acts for the seller
at the call is the succession book's (§8), the standard admitting a
signature by the seller's current key alone (`eravon.standard/1` §9). The
seller's action messages of §4.5 are verified the same way over
`actionDigest`, the empty string being §5.4's answer for `Settle` and
`E_SIG_FORM` for `ClaimDelivery` and `Refund` (the other three messages
answer `E_ACTION_PARTY` under the seller before any test here, §4.5).

**[D] 5.2 The buyer's.** `verifyOrder(orderDigest, sig, buyerKey)`: `sig`
byte-equal to the empty string is §5.4's answer; otherwise it is §5.1
verbatim with `orderDigest` and `buyerKey`, a value that is no string, or a
string outside hex65, being `E_SIG_FORM`; this is the form a buyer's account
key produces. The buyer's action messages of §4.5 are verified the same way
over `actionDigest`, the empty string being §5.4's answer for `Settle` and
`E_SIG_FORM` for `Confirm`, `Dispute`, and `Cancel` (the other two messages
answer `E_ACTION_PARTY` under the buyer before any test here, §4.5).

**[D] 5.3 One form for both parties.** The seller and the buyer sign alike:
a party's signature is the 65 bytes of §5.1 by its key, and the empty string
of §5.4 is the one other value a slot may carry. No form is particular to
either party, and a value of any other shape, an object among them, is
`E_SIG_FORM`.

**[D] 5.4 The second form: consent by call.** A party that is a contract
signs nothing; it consents by calling. A document of `eravon.link/1` or
`eravon.dossier/1` that carries a party's signature (§9) says so by the
empty string in that signature's place, the party's **slot**, which in this
law is the `sig` a verification of §5 is handed and at the standard the
`bytes` argument `init` or `settle` takes for it, empty for this form: where
`verifyOffer`, `verifyOrder`, or the verification of a `Settle` message
under §4.5 is handed a `sig` byte-equal to the empty string, it answers
`SIG_BY_CALL`, an answer of its own that is neither an acceptance (it names
no signer) nor a token of §7: the document claims that the party's consent
to this digest is given by a call, and a reader reads the chain to decide it
(§8). Nothing here decides the claim: no test of §5.1 through §5.3 runs on
an empty string, and no other value is this form (`0x`, a string spelling
zero bytes, is no hex65 and answers `E_SIG_FORM` under §5.1). The five
messages `ClaimDelivery`, `Confirm`, `Dispute`, `Refund`, and `Cancel` admit
no empty slot: a party that is a contract performs those acts by calling the
instance, their signed forms exist for signing parties alone, and an empty
string presented for any of them under a party §4.5 admits for it is
`E_SIG_FORM`, the party rule having answered `E_ACTION_PARTY` under the
other. Whether the party has code is not read: the form is the slot's, and a
key-holding party that chooses to send the call itself makes the same claim
by the same slot.

**[O] 5.5 Producers.** A party signs with RFC 6979 deterministic nonces over
the digest and low-s by negation, as `galeed/1` §5.7 states for entries, so
that two conforming clients sign one message under one key to one `sig`: the
seller's offers, the buyer's orders and both parties' action messages alike.
A party's key, the seller's and the buyer's alike, is a key the party's
client made and keeps for use in that client alone: it signs the party's
offers or orders and its action messages, and it sends the party's own
transactions (the seller's: posting a listing on the standard's board,
approving the logic to pull the deposit where `sellerFund` is the key,
revoking a document, opening a case, claiming delivery, refunding; the
buyer's: paying the instance, opening a case its order admits, confirming,
disputing, cancelling; either's: registering its consent (§8), running
`init`'s expiry branch, settling, sending a signed message of §4.5, drawing
on the guild track, sweeping, and declaring on the succession book
`eravon.standard/1` §9 names, the key the declaration names acceding there;
each call the standard gates it admits from the current keys that standard's
§9 names for that call, the board takes a posting from any address (its
§7b), the token decides the approval, sent from the address that grants it,
and the payment, sent from any address, and the book takes a declaration
from the record's current key and an accession from the record's pending
key, the key its last declaration that bound named (`galeed.roster/1` §4.2,
§4.3)), so it holds the gas those take. The seller's key holds never the
price, which goes to `sellerPayout`, and the deposit stands at `sellerFund`
(§2.3); the buyer's key is the buyer's account and holds what it pays. The
instance pulls from the addresses the order names, whatever key now acts for
each (`eravon.standard/1` §9): the deposit from `sellerFund` under its
allowance to the logic, so that where the client writes `sellerFund` as the
key itself, that key holds the deposit and sends the approval, after its
record moved as before (that standard's §7a), and where it writes another
address, that address holds the deposit, grants the allowance, and has bound
itself to the seller on the logic; and a dispute's fee, beyond what flowed
into the instance unrecorded since `init`, from `buyerKey` under its
allowance to the instance, so that a buyer whose record moved sends the fee
to the instance after the case opens and before it disputes, money sent
before `init` being the surplus `init` credits to `buyerPayout` (that
standard's §4 step 8, §8). A party's key answers no signing request from
outside the client, never a typed-data request from another application,
which could carry a digest of §4, never an EIP-191 `personal_sign` request,
and never an EIP-7702 delegation, whose code would call the logic's
`consent`, `revoke` and `acceptFunding`, the succession book's `declare`,
and the instance as the party (§8; an `acceptFunding` sent from a seller's
key binds the deposit the key holds to another seller's cases,
`eravon.standard/1` §7a). A party signs an action message of §4.5 only to
submit it at once, and treats every message of §4.5 it has signed and handed
out as valid in every later state that admits the action until its record
moves to another key, at the accession that takes up its declaration and
never at the declaration (`galeed.roster/1` §4.2, §4.3; `eravon.standard/1`
§9), the messages carrying no clock but `Settle`, which may be submitted
until its `expiry` (§4.5, `eravon.standard/1` §9; a `Refund()` is admitted
by its signed form outside a dispute alone, and by the seller's own call
outside a dispute or past the end of T3, the `rulingWindow` that opens at
the dispute, that standard's §5 and §8). A seller that is a contract (§5.4)
has no signing key, and what its address holds is its own affair. A buyer's
client sets an order's `openBy` to the earlier of the offer's `expiry` and
the second of signing plus the document's `openWindow`, so that a seller who
does not open the case in the window it promised leaves the buyer's money
refundable (`eravon.standard/1` §4), and it signs no order whose `openBy` is
already past. A buyer that consents by call (§5.4) has no second of signing:
its client sets `openBy` to the earlier of the offer's `expiry` and E plus
`openWindow`, E being the second it exports the order plus the longest time
its consent can take before it can first land on the chain (for a buyer that
is a contract whose consent waits on its own rules, the longest delay those
rules allow for the consent as it is posted, plus a margin its client
fixes), and exports no order whose `openBy` so set is at or before E. A
producer that fills a party's slot by call (§5.4) writes the empty string
there, and never a signature or any other string; a party that is a contract
performs the acts of §4.5 other than `Settle` by calls to the instance, and
its producer writes no message for them.

---

## 6. Fees

**[D] 6.1 The fee value.** `fee(quote, amount)` over a decoded quote and a
uint256 yields: `premium` = ⌊`amount` × `premiumBps` / 10000⌋, the
standing fee `eravon.standard/1` §8 names P; and, where `feeUnit` is zero,
`lf` = `feeFixed` + ⌊`amount` × `feeBps` / 10000⌋, then raised to
`feeMin` where `feeMin` is nonzero and `lf` is below it, then lowered to
`feeMax` where `feeMax` is nonzero and `lf` is above it, in that order so
that a quote whose nonzero `feeMax` is below its nonzero `feeMin` yields
`feeMax`, the litigation fee that standard names LF, which with P makes
the seller's deposit D = P + `lf`; there, `feeFixed` + ⌊`amount` ×
`feeBps` / 10000⌋ above 2^256 − 1 before the two bounds apply, or
`premium` + `lf` above 2^256 − 1 after them, is `E_FEE_OVERFLOW`, the
first in that order that overflows. The products and quotients of this
section are exact integers, never reduced modulo 2^256, and no product is
a fault: ⌊`amount` × bps / 10000⌋ is at most `amount` for every uint256
`amount`, and under §2.3's bound no product passes 2^256 − 1, which is
why the contract may form them in 256 bits (`eravon.standard/1` §3).
Where `feeUnit` is zero the value carries `lf` with `priced` true; where
`feeUnit` is nonzero the fee is priced by the contract at `init` from the
feeds the quote names, no sum of this section is formed and nothing here
faults, and this law yields `premium` alone with `priced` false.

**6.2 Meaning.** The offline fee is what a buyer's client shows a buyer
and a seller's client shows a seller before either signs, each having
read the candidate's quote bytes where the candidate published them (§8);
the contract computes the same value from the same words and pins it.
What each party bears at each terminal is `eravon.standard/1` §8's table.

---

## 7. Tokens and vocabularies

**Document tokens** (closed; one per rejection, by the orders of §2.4 and
§2.5): the `galeed/1` §3 tokens other than `E_UTF8`, which §2.4's own UTF-8
test reaches first and `canonical(block)` therefore never names;
`E_TERMS_UTF8`, `E_TERMS_BLOCK`, `E_TERMS_FENCES`, `E_TERMS_UNCLOSED`,
`E_TERMS_OUTSIDE`, `E_TERMS`, `E_TERMS_MISSING`, `E_TERMS_CLOSED`,
`E_TERMS_V`, `E_TERMS_CHAIN`, `E_TERMS_LOGIC`, `E_TERMS_SELLER`,
`E_TERMS_PAYOUT`, `E_TERMS_FUND`, `E_TERMS_TOKEN`, `E_TERMS_AMOUNT`,
`E_TERMS_DELIVERY_WINDOW`, `E_TERMS_CONFIRM_WINDOW`, `E_TERMS_OPENWINDOW`,
`E_TERMS_BUYEROPEN`, `E_TERMS_GOODS`, `E_TERMS_DELIVERY`, `E_TERMS_LADDER`,
`E_TERMS_JUDGES`, `E_TERMS_GUILDS`, `E_TERMS_NO_CANDIDATE`, `E_TERMS_NONCE`,
`E_TERMS_EXPIRY`, `E_TERMS_COMPLIANCE`, `E_TERMS_SERIES`; with an index:
`E_TERMS_DUP_JUDGE`, `E_TERMS_DUP_GUILD`, `E_LADDER_ORDER`, `E_RUNG`,
`E_RUNG_MISSING`, `E_RUNG_CLOSED`, `E_RUNG_BUYER`, `E_RUNG_SELLER`,
`E_RUNG_BEARER`, `E_RUNG_NOTE`, `E_RUNG_SUM`, `E_JUDGE`, `E_JUDGE_MISSING`,
`E_JUDGE_CLOSED`, `E_JUDGE_ADDRESS`, `E_JUDGE_AMANAH`, `E_JUDGE_QUOTE`,
`E_JUDGE_SOURCE`, `E_GUILD`, `E_GUILD_MISSING`, `E_GUILD_CLOSED`,
`E_GUILD_POINTER`, `E_GUILD_AMANAH`, `E_GUILD_QUOTE`; without an index:
`E_LADDER_FIRST`, `E_LADDER_LAST`.

**Quote tokens** (closed; by §3.2): `E_QUOTE_LEN`, `E_QUOTE_FIELD` (carrying
the word index, which is one of 0, 1, 2, 3, 4, 5, 7, 11, 12, 13, 14; words
6, 8, 9, and 10 fault on no input), `E_QUOTE_FEED`.

**Derivation tokens** (closed): `E_OFFER_CANDIDATE` (§4.3); `E_ORDER_BUYER`,
`E_ORDER_PAYOUT`, `E_ORDER_OPENBY`, `E_ORDER_PREV` (§4.4);
`E_ACTION_MANIFEST`, `E_ACTION_BPS` (§4.5); `E_FEE_OVERFLOW` (§6.1).

**Signature tokens** (closed): `E_SIG_FORM`, `E_SIG_V`, `E_SIG_RANGE`,
`E_SIG_HIGH_S`, `E_SIG_RECOVER`, `E_SIG_SIGNER` (§5.1 to §5.4);
`E_ACTION_PARTY` (§4.5's party rule, decided after that section's argument
tests and before any test of §5).

**The answer `SIG_BY_CALL`** (§5.4) is no token and no acceptance: it is
the third outcome of `verifyOffer`, of `verifyOrder`, and of the
verification of a `Settle` message under either party, given on an empty
slot alone, and no other verification of §5 has it; a law that carries
one of these outcomes carries it under a name of its own, apart from
every acceptance and every token.

**Literals this law fixes**: inside a document, the version literal
`eravon.terms/1`, the two fence lines of §2.2, and the pointer prefix
`galeed:guild:` of §1; outside one, the domain name and version of §4.2's
`domain`, the two bytes `0x1901` of §4.2's digest, the four type strings of
§4.2, the six of §4.5, the tuple of §3.1 whose encoding a quote is and whose
`keccak256` a document's `quoteHash` members carry, the deployer address and
the two EIP-1167 byte strings of §1.

---

## 8. Outside the grammar

- **Chains, clocks, feeds, registries.** This law reads none of them.
  Whether `logic` is the standard; whether a judge's `amanahHash` is the
  `entry_id` the judge document now offers (`galeed.roster/1` §5.8) and
  whether that entry stands anchored in the judge's ledger; whether a
  guild's `amanahHash` is the `sha256` of the UTF-8 bytes of the `text_md`
  of the covenant document the guild now publishes (`galeed.roster/1` §3b.3,
  §5.9); whether the quote bytes the entry or the document carries under the
  key of §2.5 hash to `quoteHash`, name this document's `token`, and cap the
  price at or above `amount`; whether a guild has a panel engaged; for a
  guild candidate, whether the quote's `offerWindow` reaches 256 times the
  chain's nominal block interval (its slot time where it has one), the span
  the draw's seed is served for, and whether the chain's block hashes come
  from consensus (`eravon.standard/1` §3, §7); whether a pointer's guild
  still stands; whether the price feed a quote names is live; whether
  `sellerFund` is `sellerKey` or bound to it on the logic (`funds(sellerKey,
  sellerFund)`, `eravon.standard/1` §4 step 5, §7a), without which no case
  opens whatever the deposit, and whether it holds a balance and an
  allowance to the logic each at least P + `lf`, `lf` being §6.1's where the
  quote's `feeUnit` is zero and the fee the quote's feeds price at that
  moment otherwise (`init` prices it again); and whether the seller has
  revoked this document at the logic (`eravon.standard/1` §4), under which
  no case opens; and which key acts for each party now, which the succession
  book answers (`eravon.standard/1` §9): the tooling at the moment of use,
  reading the chain and the registries, and the contract at `init` and at
  every act.
- **The reference-unit cap and fee.** Where `feeUnit` is nonzero, the cap
  comparison and the litigation fee need a price; §4.3 and §6.1 say so and
  yield nothing that would need one. The buyer's client shows a
  conversion from the same feed the contract will read and says the
  pinned value comes at `init`.
- **The prose.** What the prose members promise, whether the goods and the
  delivery they describe are what the parties want, and what the
  `compliance_md` members mean: the parties. A segment carried as a hash
  names a text the seller hands the buyer and the judge apart from the
  document (`eravon.link/1` §5), and each of them compares its `sha256` with
  the member before reading it as the document's.
- **Stages.** A sale delivered in stages is as many documents as it has
  stages, each naming the same `series.id` and its own `stage` (§2.3), and
  as many orders, each later one naming in `prev` the instance of the stage
  before (§4.4). Whether the documents of one series agree in `sellerKey`,
  `chainId`, `logic` and `stages` and run 1 to `stages`, and whether the
  orders' `prev`s chain them in that order under one `buyerKey`, is read by
  nobody here: a buyer's client signs or exports the stages in order under
  one key, each `prev` computed from the order it has just signed or
  exported, funds every stage's instance, and hands the seller every receipt
  at once; a seller's client takes the receipts of a series as
  `eravon.link/1` §10 states, and begins no stage before the standard opens
  it; a seller writes every stage of a series under one judge or one guild,
  so that the standard seats the series judge first where its §7 and §4a
  allow, and sets a later stage's `openWindow` and its offer's `expiry` so
  that its `openBy` (§5.5: the earlier of `expiry` and `openWindow` seconds
  after the buyer's signing, or after E for a buyer that consents by call)
  falls after the latest second at which the stages before it can still end.
  What the chain does with `prev` is the standard's (`eravon.standard/1`
  §4a): a stage opens only after the stage at `prev` ended leaving the buyer
  no share, and is cancelled only after that stage ended leaving the buyer a
  share, so that neither party takes a later stage's money, or opens it,
  while the stage before is in flight or unopened, a later stage's own
  `openBy`, `expiry` or revocation alone returning its money to the buyer.
  The lock is the order's, not the document's: under an offer that admits
  the buyer's own `init` (`buyerMayOpen`) the buyer may open a stage's
  document under another order it writes (a zero `prev`, or any `prev` the
  standard's §4a gate passes) while the stage before is in flight, which a
  seller's client takes as no stage of the series (`eravon.link/1` §10) and
  which returns the money to the buyer by T1's end at the latest; a seller
  who wants the lock on its later stages writes `buyerMayOpen` false on
  them.
- **Expiry, windows, and the token.** Whether a case can still open, and
  what the windows mean in the contract's clocks: the contract. Whether
  `token` deducts a fee on transfer this law does not read; under such a
  token `eravon.standard/1` §8 opens no case where the deposit P + `lf` is
  nonzero, since `init` tests the deposit that arrived against P + `lf`;
  where that sum is zero the case opens and each payee bears the token's
  fee.
- **The contract's ABI.** How an offer, an order, a quote, the two signature
  slots, an empty one included, and the action messages are passed to the
  contract: the standard's interface, which is the standard's own bytes, and
  no member of any document.
- **Fee bearing and splits.** `eravon.standard/1` §8's table.
- **Malleability.** Every signature of this law is secp256k1 under the
  parent's low-s rule, which is this law's test (§5.1) and the parent's
  producer obligation; `eravon.standard/1` §9 has the standard refuse a
  high-s signature likewise.
- **The candidate's standing.** Whether a judge or guild still stands,
  still takes cases, still names this token, or has left the guild the
  `source` pointer names: the roster law and the registries, read at the
  act.
- **Consent by call.** Whether a party whose slot is empty (§5.4) has
  consented: the standard, which admits the claim at `init` and at `settle`
  alone, and there only thus: the seller's slot at `init` where `init` is
  called by `sellerKey`'s current key or the logic's registry holds
  `offerDigest` (§4.2, under the logic's domain) under `sellerKey`; the
  buyer's slot at `init` where `init` is called by `buyerKey`'s current key
  under an offer whose `buyerMayOpen` is `true` (§2.3), or the registry
  holds `orderDigest` (§4.4, under the instance's domain) under `buyerKey`;
  either party's slot at `settle` where `settle` is called by that party's
  current key or the registry holds the `Settle` `actionDigest` (§4.5, under
  the instance's domain) under that party; a party's **current key** is the
  key the succession book `eravon.standard/1` §9 names answers for it at the
  call (`galeed.roster/1` §4), the party itself where it never moved. A
  call's caller is the address the called contract sees calling it (the
  standard's `msg.sender`), the instance for `init` and `settle` and the
  logic for `consent`: the party's own account where the call leaves from
  that account, whether as a transaction's top-level call or as an inner
  call, and whatever code the account runs to make it (its own, a library it
  reaches by `DELEGATECALL`, an EIP-7702 delegation's); the intermediary,
  and not the party, where the call leaves from an intermediary's own
  address (Multicall3, a forwarder) on the party's behalf, so that a party
  consenting by its own call, or registering, makes that call itself; and
  never the origin of a transaction in which someone else made that call.
  The registry is the logic's own: `consent(party, digest)` called on the
  logic's address by `party`'s current key records the digest under `party`
  and is never withdrawn; `consented(party, digest)` answers anyone
  (`eravon.standard/1` §9). Each of the three digests has an end (the
  offer's `expiry`; for the order, the earlier of that `expiry` and its own
  `openBy`; the settlement's `expiry`) past which the standard admits
  nothing under it, and that is what ends a registration, a buyer's `cancel`
  ending its order's before then (`eravon.standard/1` §4); a seller's
  revocation of the document (`eravon.standard/1` §4) ends every offer under
  it, a registered consent included. The tooling before the standard reads
  the registry at the moment of use. This law tests the slot's emptiness and
  nothing else, and reads no code at the party's address.

---

## 9. Relation to the other laws

A terms document's `termsHash` is the `bytes32 termsHash` of the offer the
contract verifies. The document itself is carried by
[`eravon.link/1`](eravon-link-v1.md) in the listing a seller posts on the
standard's board (`eravon.standard/1` §7b), the link being a pointer to that
listing, and by [`eravon.dossier/1`](eravon-dossier-v1.md) in a dossier;
both laws build offers of §4.1 from their own members, decide signatures by
§5, and carry §5.4's answer under names of their own where a slot is empty;
a document those laws bind to is a document of this law. A judge's judgment
under `galeed/1` §6.3 names the case by `subjectRef.id` = `caseId` and
`subjectRef.contract` = `instance`, both of §4.4. A judge's `amanahHash` is
the `entry_id` of the `amanah` entry its judge document names
(`galeed.roster/1` §3a.3), a guild's is the `sha256` of the `text_md` of its
covenant document (`galeed.roster/1` §3b.3), and a pointer is that law's §6.
The standard contract recomputes every digest of §4 and verifies every
signature of §5 against the party's current key, which is the key the
document names where the party never moved its record (§8,
`eravon.standard/1` §9). A standard whose bytes differ from this text on
some input is a standard this law does not describe, whatever part of this
text it differs from (§10.3 states it for the typed data), and the
discrepancy is a defect in one artifact or the other, settled by amending
whichever is wrong; until then the core of §10.5 decides what this law says,
and where §10.5 names none, this text does.

---

## 10. Freeze and evolution

**10.1** The reference implementation freezes when independently written
implementations, produced from this text, the passages of `galeed/1` it
invokes, its harness contract, and the `galeed/1` harness that contract
cites, alone, agree with it on every input of a generated corpus (every
document token with its index, every accepted document's `termsHash`, every
quote decoding, every offer value with its struct hash and digest, every
order with its salt, instance, and digest, every action struct hash and
digest, every signature the harness's producer commands ask for, every
verification token, `SIG_BY_CALL` on an empty slot once for the offer, once
for the order, and once for `Settle` under each party, with `E_SIG_FORM` on
an empty slot once for each of the other five messages under the party §4.5
admits for it, and every fee value), with every closed boundary of this text
witnessed on both sides by at least one corpus input under every seed the
corpus is generated with, the boundary list being a precondition on the
corpus and never a property claimed of it afterwards; the corpus carries the
list beside itself. No token of this law carries another token; the index a
document token carries is asked for by the clause above, and `E_QUOTE_FIELD`
once with each of the eleven word indices §3.2 can name; the canonical form
the core carries (§10.4) is witnessed on every input of
`galeed-conformance/corpus/canon/`, the `galeed/1` corpus's `check` set as
generated at seed 1 and committed, each carried as the block of a document
and predicted from that corpus's prediction through §2.4 (an input that is
no UTF-8 `E_TERMS_UTF8`, §2.4's first test deciding before `galeed/1`
§3.5's; a token of §3.5 carried over; a canonical value then `E_TERMS` or
`E_TERMS_MISSING`), so that every boundary of `galeed/1` §3 past its first
test that corpus witnesses, the order of its tests among them, is witnessed
here, and on every closed boundary of `galeed/1` §3 that set leaves
unwitnessed, an object's trailing comma among them, the boundary list naming
them; `galeed/1` §1's hex forms are witnessed on each refusal that section
names (an uppercase digit, an uppercase or absent prefix, another length);
and the secp256k1 arithmetic the core carries is witnessed on each adjacent
pair of `galeed/1` §5.4's tests, the range test once by `r` and once by `s`.
A closed boundary of this text is a boundary the corpus can witness on the
surface §10.2 compares; a rule with no reachable failing side on that
surface (the form halves the harness makes misuse, those of `E_ORDER_BUYER`,
`E_ORDER_PAYOUT`, `E_ORDER_OPENBY`, `E_ORDER_PREV` and `E_ACTION_BPS` among
them; a range half that repeats `galeed/1` §3.2's bound on an `int`, which
canonical form decides first) is witnessed once and states no boundary in
this sense. No `[O]` rule is decided by a predicate of this law, and the
freeze certifies only the producer commands' output, never the obligations
behind them. The fuzzing `galeed/1` §12.1 asks of its core is deferred here
as `galeed.papers/1` §13.1 defers it; §10.5 records beside every digest it
names whether that fuzzing was run over §2.2's byte scan, `dec256`, and the
canonical form and secp256k1 arithmetic its core carries (§10.4), and with
what result, a naming that records none being a naming under deferral.

**10.2** Once §10.5 names a release digest, agreement with the frozen core
on the corpus is the test anyone can run in seconds, and it is evidence
of conformance and never its definition.

**10.3** `eravon.terms/2` is a new version literal and a new fence line;
a document accepted under `eravon.terms/1` is accepted under
`eravon.terms/1` forever, and a later law reads it by this one. A
standard whose typed data differs from §4.2 or §4.5 is a standard this
law does not describe; the fence line of the document an instance's
`termsHash` names (§2.2) carries the version of this law that document is
read under.

**10.4** This law and `galeed/1` are two laws: a change to one is never a
change to the other. This law reads every passage of a GALEED law it cites
as it stands when §10.5 names this law's core: of `galeed/1`, §1 for the
forms, `sha256` and `keccak256`, §2.1 for a byte string's identity, §3 for
`canonical(b)`, §5.4, §5.5 and §5.7 for the secp256k1 arithmetic and the
signing this law's §5 invokes, the preamble of §6 for its member tables, and
§6.2, §6.3, §12.1 and §12.5 where this text cites them; of
`galeed.roster/1`, every section this text cites, §6.1 for the pointer §1
restates and §4 for the current key among them; and the sections of
`galeed.venue/1` and `galeed.papers/1` this text cites. Its core carries its
own implementation of what its predicates read of them, the canonical form
and the secp256k1 arithmetic in `ertcanon.py` and `ertcrypto.py` and the
member tables' reading in `ertlaw.py`, counted in its own digest (§10.5) and
naming no GALEED core, and a later text of any such passage, revised in
place under `galeed/1` §12.5 or published under a new version, changes
nothing here until this law's own revision follows it.

**10.5 The core's identity.** Naming a release digest here is the freeze,
and it is the only act that performs it. The release digest of the
reference implementation in `eravon-conformance/terms-py/` (`ert.py`,
`ertlaw.py`, `ertcrypto.py` and `ertcanon.py`, which carry §10.4's
implementation as that section names; the language's standard library,
nothing else),
as `criterion-digest-terms.sh` computes it, is

`0x4984258d42de40eb19e51a8291cb9b572d9b9ab184e12e7eda19893a61dcd0a9`

named on 2026-10-27 and named again on 2026-11-16, the same digest, so that
§10.4 reads the GALEED passages it cites as they stand on 2026-11-16; the
review record beside this text (docs/eravon-terms-v1-review.md) records how
it converged. The fuzzing of this core was the three implementation seats'
differential probing against it across the six rounds of the revision of
2026-10-24 and its stages of 2026-10-26 (three rounds each): Rust 391,921,
Node 295,290 and Python 264,009 inputs beyond the corpora, 951,220 in all,
over the fences and bytes, the canonical form, every document member with
the `series` object's three members at every type and spelling, `prev` in
every spelling on each of the six order and action commands, the action
digests under a later stage's instance and the signatures across instances,
with zero disagreements; the closing round's first measurement, the programs
unchanged, was zero at seeds 1 to 6 over 4,239 cases per seed (rounds 2 and
3 of the stages). The fuzzing of the deferral over the core named on
2026-10-19
(`0x5cb4a7f34c2a5b46b290614165c32a5ceb95b928b7f355ca3e467d0143b1945e`) was
run across the five rounds of the revision of 2026-10-17 and the two of
2026-10-18 and 2026-10-19 by the three implementation seats' differential
probes against that core: over §2.2's fences and bytes (49 fence shapes,
4,000 grammar-fuzzed texts and 1,320 byte mutations in round 1 by the Node
seat, 810 fence cases and 1,200 byte mutations by the Python seat, and the
structural and byte mutation of every document kind every seat repeated at
fresh seeds in rounds 2 to 7) and over `dec256` (the amount at ⌊(2^256 −
1)/10000⌋ and beyond, leading zeros and every spelling, through every
command that reads one), zero disagreements with that core; beside that
digest stood the three seats' 1,634,961 differential inputs over the seven
rounds across the three laws (Rust 656,230, Node 580,430, Python 398,301).
