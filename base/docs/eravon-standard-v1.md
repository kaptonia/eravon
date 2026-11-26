# ERAVON standard v1: the venue document

The document item 17 of `galeed.venue/1` §7 asks of every venue, for the
ERAVON standard (sources eravon/standard/src/). A judge's desk reads an
ERAVON instance through the twenty members of `galeed.venue/1`, plus the
parameterless read-only views the judge's own covenant declares for this
standard (the desk executes those generically, as the judge's data, and
learns no name from the venue: `galeed.venue/1` §1 and §6 stand); this
document says how the instance reaches each fact the interface serves, and
what a client of the parties finds beyond the interface. This document
states what the contract does; a disagreement between the two is a defect,
removed before any judge declares `proxyCodeHash`.

**Shape.** One immutable logic contract, `Eravon`, deployed by whoever first
uses it through the public keyless CREATE2 deployer at
`0x4e59b44847b379578588920cA78FbF26c0B4956C` under the salt
`keccak256("eravon/1")`; one EIP-1167 minimal proxy per case, deployed by
anyone through the same deployer at the address the signed order determines;
every instance is one `galeed.venue/1` deployment, and one case is one
instance, a sale delivered in stages being as many documents, orders, and
instances as it has stages, chained by their orders' `prev` (§4a;
`eravon.terms/1` §2.3). No owner, no upgrade, no pause, no allow-list, no
`selfdestruct`, no privileged address. The panel book, `EravonPanelBook`, is
a third contract, one per chain, deployed by whoever first needs it through
the same deployer under the salt `keccak256("eravon/1/panel")` and
designated by code (§7). A fourth, `EravonVerifier`, holds the pure and view
half of `init`, of `cancel`, of every draw, of every terminal and of the
parties' signatures (the terms guards, the quote's decoding,
`hashStruct(Offer)`, the fee's pricing, both parties' signatures over the
digests the logic builds, the `Cancel` digest among them, the two parties'
consents at `init` and `settle`, §9, the guild track's read of the panel
book and the draw `redraw` makes, `panel` and `draw`, §7, the order's struct
hash, `salt`, and §8's terminal split of A, P, the buyer's fee and the
seller's LF, `split`, which also answers the two outcome views of §8), so
that the logic stays under EIP-170; it is stateless, deployed once per chain
by whoever first needs it through the same deployer under the salt
`keccak256("eravon/1/verify")`, and designated by code: the logic derives
its address and checks `extcodehash` against `verifierCodeHash` before every
call, reverting `VerifierMissing` otherwise, and a revert inside it reaches
the caller with the same bytes. A fifth, `EravonBoard`, is the broadcast
board on which sellers post their listings and settlement offers (§7b): no
storage, no constructor argument, no owner, no pause, one per chain,
deployed by whoever first needs it through the same deployer under the salt
`keccak256("eravon/1/board")`, and designated by code, a client reading only
a board whose `extcodehash` is `boardCodeHash`; it is independent of the
GALEED registry, which carries judges' and guilds' documents and nothing of
ERAVON's. The logic itself holds no money: a seller's deposit is an ERC-20
allowance the order's `sellerFund` grants the logic on the settlement token,
pulled into the instance when the case opens (§7a), and the logic's own
storage holds three registries, consents (§9), revocations (§4) and funding
bonds (§7a), and nothing else: the logic runs the same code as its
instances, so every instance function exists on it, but `init` and `cancel`
refuse it (`WrongAddress`), `setPayout` and `withdraw` refuse it
(`NotInstance`), and the rest find no case and no credit there. The compile
is pinned (solc 0.8.28, evm_version cancun, optimizer 200, via_ir, no
metadata); the resulting addresses and code hashes are in
eravon/standard/CODEHASH.md, recomputed by `identity.sh`. The identity a
judge declares is `proxyCodeHash`, the keccak256 of the 45-byte proxy
runtime code that embeds the logic address. Every instance, the verifier and
the panel book read one succession book, `GaleedSuccessionBook`
(`galeed.roster/1` §4) at `0x3dB7c505dd25361F1A66f6c5Bd96b48df27f668c`,
whose code hash is
`0xdb3293b42187b8b7270e62d5eaafee45af02c9caf3a0c8fe376cb4195f2f6aca`, the
public deployer's CREATE2 address under the salt
`keccak256("galeed/succession-book")` of the book's initcode
(`GaleedSuccessionBook.sol` of `galeed-conformance/venue-replay/src/`,
copied under test/replay/, under the pinned compile), whose keccak256 is
`0x711aec6ad1a23808653588aeae574e048be00bbefd513646ed6785e4e465a397`, and
the address the family declares for chains 1 and 11155111 (`galeed.roster/1`
§9); it is a constant of the code that `successionBook()` serves, so that
the logic's address and `proxyCodeHash` are one on both chains. The address
designates that code on every chain the deployer stands on, no other code
being able to stand there: on a chain where nobody has deployed the book
every gate that reads it reverts, and anyone may deploy it there through the
deployer, after which the gates read it; an instance meets `galeed.venue/1`
§7 item 16 on a chain only where the family's declaration names that address
for it (`eravon.link/1` §10). The succession book answers which key acts for
a judge's or a guild's address (`galeed.venue/1` §1) and for each party's
key (§9).

---

## 1. Case identity

`caseId` = `hashStruct(OrderTerms)` under `eravon.terms/1` §4.2, the salt
of that law's §4.4, which is also the CREATE2 salt, so that

    instance = keccak256(0xff ‖ deployer ‖ caseId ‖ keccak256(initcode))[12:]
    initcode = 0x3d602d80600a3d3981f3363d3d373d3d3d363d73 ‖ logic ‖ 0x5af43d82803e903d91602b57fd5bf3

Anyone holding the order computes both before any money moves. The case
exists from the block of the transaction that runs `init` on the instance;
`init` refuses an instance that holds a case already (`AlreadyInitialized`)
and then proves the address: `address(this)` equals the address the order it
was handed determines, else `WrongAddress` (the salt is computed and the
proof made on the verifier, which derives the address from the calling
account's own code under the initcode's prefix, so a chain without the
verifier answers `VerifierMissing` first), so an instance can only ever hold
the case whose order names it. The derivation reads the account's code, so
an account of other code that the deployer placed under an order's salt
behind the initcode's prefix proves its own address, never the order's; it
is no instance for any reader, the deposit pull refusing it (`NotInstance`,
§7a), the stage read refusing it (§4a), no venue declaration carrying its
code hash and no client computing its address, and it holds only what was
sent to it. A reader therefore reads an instance's events from the
instance's address, the one its order derives here and whose code hash is
`proxyCodeHash`, never by `caseId` alone: any contract emits events of the
same form. One instance holds one case; `seatOf` reverts `UnknownSeat` for
every other `caseId` and for a seat ordinal the case never opened, a guild
case's seat 0 before its draw (§7) included.

A judgment or resignation on a seat names it as `galeed/1` §6.3 has it:
`subjectRef = {kind: "evm", chainId, contract: instance, codeHash:
proxyCodeHash, id: caseId}` with the extension member `seat` (the seat
ordinal, an `int`, present on a judgment); `decision = {rung}`, `extra`
being empty. A reader auditing the judge lists the standard in the basis of
`galeed/1` §9.4 as the class item `{codeHash: proxyCodeHash}` under the
chain's `registries`, beside the GALEED registry's address, lists every key
of the judge's lineage (`galeed/1` §7.4) and of its record on the succession
book (`galeed.roster/1` §5.3) in the same `chains` object's `senders` (the
judge document's `ledger.basis` lists the lineage already, `galeed.roster/1`
§7; a basis a reader of this standard builds for itself lists them too),
takes `fromBlock` 0 and `toBlock` the head it read, so that the basis covers
under `galeed.papers/1` §9.2, and counts every `Anchored` an instance of
that code emitted from a key of the judge's lineage that had not retired at
it (`galeed/1` §7.4) as an anchor record.

## 2. The terms bytes

`termsHash` is the sha256 of the parties' `eravon.terms/1` document, the
Markdown text whose fenced canonical block the parties signed excerpts of.
The document is on chain before any buyer sees it: the seller posts its
listing (`eravon.link/1` §2, an `eravon.listing/1` carrying the whole
document under `terms.inline_md`, the order's numbers, and every candidate's
word, quote hash, and signature) on the board through `post(termsHash,
document)` (§7b), and the link the seller hands out is a pointer to that
transaction and that listing's sha256 (`eravon.link/1` §3). A seated judge,
a party's client, or any reader obtains the document from the chain: from
`Opened` it takes `sellerKey` and `termsHash`, reads the board's `Posted`
logs under the key `termsHash` (any poster: the case's `termsHash` fixes the
document whoever posted it), keeps those whose document passes
`eravon.link/1` §2 with `order.sellerKey` equal to `sellerKey` and
`terms.hash` equal to the key and to the case's `termsHash` (the reader
reads no registry for it: `termsHash` fixes the document), and takes the
document from any of them (the listings of one document agree in every
member but a guild element's `seq` and `panelHash` and a candidate's `sig`,
so a listing reposted under another document's key fails the `terms.hash`
test and a listing whose unsigned members were rewritten still carries the
document); a reader takes neither `seq` nor `panelHash` from a listing it
found by key, reading the frozen panel from `PanelFrozen` and `status()`
(§10), and reads a judge's `source` from the document, which fixes it
(`eravon.link/1` §7); a reader holding a party's pack reads the one listing
its `link.tx` and `link.hash` name (`eravon.link/1` §5.1), and a reader
holding a dossier the one its `posted.tx` and `posted.hash` name
(`eravon.dossier/1` §2.1). The desk checks the document's sha256 against
`seatOf(...).termsHash` and reads the ladder's prose from it. A segment the
seller chose to carry as a hash (`goodsHash`, `deliveryHash`,
`eravon.terms/1` §2.3) has no text on chain: either party hands the judge
the text, and the judge compares its sha256 with the member. The machine
excerpt (the `OrderTerms` struct: keys, judge or guild, payouts, the
deposit's address `sellerFund`, token, amount, windows, `buyerMayOpen`,
ladder, `termsHash`, `amanahHash`, `quoteHash`, nonce, expiry, `openBy`,
`prev`) is in the calldata of the `init` call (the transaction's input where
`init` is called at top level; an inner call's input, read from a call
trace, where a batching contract or a party's own contract called it), and
`Opened` carries its summary (the two keys, the judge, the amount,
`termsHash`), so a client rebuilds the case from the chain without the
listing and marks what it has not yet fetched.

## 3. The quote

`quoteType()` is the constant

    (address token,uint16 premiumBps,address feeUnit,address feedToken,address feedUnit,uint32 maxStale,uint256 feeFixed,uint16 feeBps,uint256 feeMin,uint256 feeMax,uint256 amountCap,uint32 rulingWindow,uint32 offerWindow,uint8 lapseDefault,bool autoAccept)

and the quote bytes are the 480-byte static encoding of those fifteen values
in that order with no tuple wrapping, decoded word by word with the tests
`eravon.terms/1` §3.2 names (every address word has its first twelve bytes
zero and `token` is nonzero; `premiumBps` and `feeBps` at most 10000; the
three `uint32` words at most 2^32 − 1; `lapseDefault` 0, 1 or 2;
`autoAccept` 0 or 1; `feeUnit` nonzero or both feeds zero). The opener reads
the quote bytes from the chain and hands them to `init`, which refuses a
quote that fails any test, a quote whose keccak256 is not the order's
`quoteHash`, and a quote whose `token` is not the order's `token`; the terms
document carries the hash alone (`eravon.terms/1` §2.5). A judge publishes
one quote per settlement token in the `quotes` member of its `amanah` entry
(`galeed/1` §6.2) under the key `<chainId>:<codeHash>:<token>`, `codeHash`
being `proxyCodeHash`; a guild publishes its quotes under the same key in
the `quotes` member of its covenant document (`galeed.roster/1` §3b). That
key is the only one an ERAVON client reads. This narrows, and does not
contradict, `galeed.roster/1` §5.10: that section fixes five key forms and
the order in which a venue with a declared address resolves them; a pure
standard has no declared address, so the two address forms cannot be built,
and a key without the token (`<chainId>:<codeHash>`, `<chainId>`) names no
settlement token, so ERAVON treats the other four as absent, since fees and
windows differ by settlement token. A judge or guild that has published no
quote under the exact key is no candidate for that token.

`extraType()` is the empty string; `rule` refuses a non-empty `extra`.

The quote's `autoAccept` is the field that carries the judge's standing
consent (`galeed.venue/1` §2.5, §2.7): a seating under a quote with
`autoAccept` set is seated with consent carried and `accepted` nonzero from
the `Seated` block; a seating under a quote with it clear opens an offer
whose clock is `offerWindow`. On the guild track `offerWindow` is also the
deadline, counted from the arming, of every draw made after the block that
follows its arming (a draw sent in that block is admitted whatever the
deadline, §7), whatever `autoAccept` makes of the offer clock, and a draw
not made within 256 blocks of its arming ends the case; a client treats a
quote whose `offerWindow` is below 256 times the chain's nominal block
interval (its slot time where it has one) as no candidate on that track, so
that, at the nominal pace, the seed's own span, and no shorter clock, bounds
the draw. No quote field carries a payout: every judge, named or drawn,
registers its own on the instance (§8, `galeed.venue/1` §3).

The fee: `feeQuoted()` returns `(lf, capInToken, priceToken, priceUnit,
rateAt)`. With `feeUnit` zero the four absolute amounts of the quote
(`feeFixed`, `feeMin`, `feeMax`, `amountCap`) are in the settlement token,
`lf` = `feeFixed` + ⌊A × feeBps / 10000⌋ raised to `feeMin` then lowered to
`feeMax` (zero meaning no bound), as `eravon.terms/1` §6.1 computes
offline, `capInToken` is `amountCap`, the two prices and `rateAt` answer
zero, and no `Priced` is emitted. With `feeUnit` nonzero each of the four
is converted once at `init` as

    x′ = ⌈x × pU × 10^(dT + decT) / (pT × 10^(dU + decU))⌉

zero staying zero, where `pT` and `dT` are the answer and the `decimals()`
of `feedToken`, `pU` and `dU` those of `feedUnit`, `decT` the `decimals()`
of the settlement token and `decU` that of `feeUnit` (the feeds have
Chainlink's shape, `latestRoundData` and `decimals`; a zero feed address
counts as price 1 with no decimals and no clock), the common power of ten
cancelled first and the product taken in full width; `lf` is then
computed from the converted amounts as above, `priceToken` is `pT`,
`priceUnit` is `pU`, `rateAt` the smaller nonzero `updatedAt` of the two
feeds (`now` where both are zero), `Priced(feeUnit, lf, capInToken,
priceToken, priceUnit, rateAt)` is emitted, and the five values are
pinned. A feed is stale where its answer is not positive or at or past
2^128, where its `updatedAt` is older than `now − maxStale`, later than
`now`, or at or past 2^64, or where `latestRoundData` reverts or answers
fewer than 160 bytes, or a `decimals()` reverts, answers fewer than 32
bytes, or answers past 36: `init` reverts
`RateStale` and nothing lands; every read the pricing makes (a feed's
`latestRoundData`, a `decimals()`) is handed 200,000 gas and copied back to
a fixed size (160 bytes of a round, 32 of a `decimals()`), a read that runs
out being one that did not answer and a longer answer costing the caller
nothing beyond the allowance, so no feed turns `init` into a bare revert
or makes the caller pay for the length of its answer. A conversion whose scaled price (`pU` or
`pT` times the power of ten) or whose result passes 2^256 − 1, and the
sum `feeFixed` + ⌊A × feeBps / 10000⌋ where it does, revert `FeeOverflow`
(the `E_FEE_OVERFLOW` of `eravon.terms/1` §6.1 where `feeUnit` is zero;
with `feeUnit` nonzero that law forms no sum and answers `priced` false,
and this revert has no offline counterpart), and the sum P + `lf`
where it does reverts the same name at §4's step 7, after the seat's
preconditions (the second clause of that `E_FEE_OVERFLOW` where `feeUnit`
is zero; with `feeUnit` nonzero it has no offline counterpart);
where every one of the four amounts is zero nothing is scaled and no
conversion overflows; A itself is at most
⌊(2^256 − 1)/10000⌋ by the terms guards (§4), so no product of A with a
bps value overflows and every share of §8 is exact. Every anomaly of a
feed, a quote or an amount is one of these named reverts, never a bare
revert or a panic: the verifier reads the feeds and reports, the logic
refuses after the signatures (`RateStale`, then `FeeOverflow`, then
`OverCap`), and on the expiry branch the verifier is asked for the offer
hash alone, so that branch never depends on a feed, on the quote bytes or
on the terms tests. A nonzero cap in the settlement token below the amount
refuses the case at `init` (`OverCap`). The premium P = ⌊A × premiumBps /
10000⌋ is computed at `init` from the settlement-token amount and pulled
with `lf` as the seller's deposit (§7a).

## 4. Seating, consent, and the gate

**Named track.** The order's `judge` is nonzero and `guild` zero. `init`
opens seat 0 for the judge with the order's `amanahHash` (the `entry_id`
of the judge's covenant entry, the `amanah.entry_id` of the judge's
document, `galeed.roster/1` §3a.3, copied by the parties; a judgment's own
`amanah` member equals it, `galeed.venue/1` §2.4) and `quoteHash`. The
seating carries consent when the quote's `autoAccept` is set and waits for
`accept` otherwise. A named case has exactly one seat.

**Guild track.** The order's `guild` is nonzero and `judge` zero;
`amanahHash` is the sha256 of the UTF-8 bytes of the `text_md` of the
guild's covenant document (`galeed.roster/1` §3b.3) and `quoteHash` the
`keccak256` of the guild's quote bytes. `init` takes the panel (`members`,
`amanahHashes`) as arguments, reads the panel book under the guild's root
(§7), freezes the panel, and arms the draw of seat 0, which anyone may land
from the next block (§7); every element of `amanahHashes` equals the order's
`amanahHash`, which the book's rejection of two words that differ already
made one word (§7). Consent follows the guild's quote's `autoAccept` in the
same way. A guild case opens at most three seats, later seats only after an
unaccepted seat ended (§7).

**The gate.** `init(t, quote, sigBuyer, sigSeller, members,
amanahHashes)` decides in this order, each step before the next:

1. the instance holds no case yet, else `AlreadyInitialized` (an
   instance cancelled under the paragraph after these steps holds a
   closed one);
2. the address proves itself: `address(this)` is the address `t` derives
   under §1, else `WrongAddress` (the salt is computed and the proof made on
   the verifier from the calling account's own code, as §1 says, so
   `VerifierMissing` comes before it);
3. **the expiry branch**: `now > t.offer.expiry`, or `now > t.openBy`, or
   the logic's `revoked(sellerKey, termsHash)` holds; then, the caller being
   `sellerKey`'s or `buyerKey`'s current key (`NotParty` otherwise), with no
   signature read and no quote decoded, `sellerKey`, `buyerKey`,
   `buyerPayout` (nonzero, else `BadTerms(4)`) and the token are kept, so
   that `sweep` knows who may call it (§9), the balance of the settlement
   token beyond what the instance already owes is credited to `buyerPayout`,
   everything owed is pushed best effort (a push the token refuses leaves
   the credit for `sweep` and for the next run of the branch, which credits
   nothing twice), `Refunded(2)` is emitted and no `Closed`, and no case
   opens; the instance stays uninitialised, so that settlement token sent
   later leaves by the same branch again and any other token by `sweep`;
4. the caller rule: `msg.sender` is `sellerKey`'s current key, or, where
   `t.offer.buyerMayOpen` is `true`, `buyerKey`'s current key (§9), else
   `NotSeller` where `buyerMayOpen` is `false` and `NotParty` where it is
   `true` (a key-holding party's client sends the call from its current key,
   a contract party from its own address; a forwarder's address is no party,
   §9);
5. the terms guards (`BadTerms(n)`, the table below), the quote (`BadQuote`,
   `QuoteMismatch` for a hash or a token that is not the order's), the two
   consents, each by that party's current key, the seller's before the
   buyer's (`BadSellerSig`, then `BadBuyerSig`, §9), the stage (`t.prev`
   zero, or the account at `t.prev` an instance under §4a (its code hash and
   its address proof) answering `stage()` with this order's `sellerKey` and
   `buyerKey`, else `BadPrev`; then that stage ended with `buyerBps` zero,
   else `StageOpen`; §4a), the deposit address (`sellerFund` byte-equal to
   `sellerKey`, or the logic's `funds(sellerKey, sellerFund)` holding, else
   `FundUnbound`, tested whether or not P + `lf` is zero, §7a), the balance
   at least the amount (`Underfunded`), the pricing (`RateStale`,
   `FeeOverflow`, `OverCap`, §3);
6. the seat's preconditions: on the named track an empty panel, else
   `BadTerms(20)`; on the guild track the guild's root read from the
   succession book and the panel read and frozen under it (§7:
   `PanelBookMissing`, `NoPanel`, `PanelMismatch`, `PanelWord`,
   `NoMembers`);
7. the seller's deposit: where P + `lf` is nonzero (`FeeOverflow` where
   the sum passes 2^256 − 1), the instance calls the logic's
   `pull(sellerKey, sellerFund, token, P + lf)` (§7a) and requires its balance of the
   settlement token to have risen by at least that sum: a `transferFrom`
   that reverts, or returns anything other than no data or exactly one
   32-byte word equal to 1, is `TransferFailed`, one that delivers less
   than asked is `DepositShort`;
   a seller whose `sellerFund` is short of allowance or balance opens no
   case, and the buyer's money stays refundable by step 3; where the sum is zero
   nothing is pulled and no `Pulled` is emitted;
8. the seat: seat 0 opened for the judge on the named track, the draw
   armed on the guild track (§7), the case `Funded`, the surplus above
   the amount credited to `buyerPayout` (credited and not pushed at
   `init`; pushed with the terminal or by `sweep`), `Opened` emitted with `judge` the named
   judge, zero on the guild track.

The terms guards: the verifier runs 1 to 16 in this table's order and
the logic runs 20 at step 6, the first that fails naming itself. A tuple
of `eravon.terms/1` §4.1's shape, as that law's §2.4, §4.3 and §4.4 build
one from a document and the sibling laws build one from their own
members, passes guards 1 to 16, and guard 20 reads the panel argument,
which no law builds, so no guard's answer has a counterpart among that
law's tokens:

| n | guard |
|---|---|
| 1 | `sellerKey` or `buyerKey` zero, or the two equal |
| 2 | not exactly one of `judge` and `guild` nonzero |
| 3 | `judge` or `guild` equal to `sellerKey` or to `buyerKey` |
| 4 | `sellerPayout`, `sellerFund` or `buyerPayout` zero |
| 5 | `token` zero |
| 6 | `amount` zero or above ⌊(2^256 − 1)/10000⌋ |
| 7 | fewer than 2 or more than 16 rungs |
| 8, 9 | rung by rung in index order: `buyerBps` + `sellerBps` not 10000 (8), `feeBearer` above 2 (9) |
| 11 | the first rung's `buyerBps` not 0 |
| 12 | the last rung's `buyerBps` not 10000 |
| 10 | a rung's `buyerBps` below the previous rung's |
| 13 | `termsHash` zero |
| 14 | `amanahHash` zero |
| 15 | `quoteHash` zero |
| 16 | `nonce`, `expiry` or `openBy` above 2^53 − 1 |
| 20 | a panel handed in on the named track (step 6) |

**Cancellation.** `cancel(t, sig)` on an instance that holds no case (else
`AlreadyInitialized`): the verifier reads this call's own `t` once
(`cancelCheck`), in this order: `address(this)` proves itself under §1 as
`init`'s step 2 does, else `WrongAddress`; `t.buyerKey` is nonzero, else
`NotBuyer`, and `t.buyerPayout` nonzero, else `BadTerms(4)`; then the stage:
`t.prev` is zero, or the account at `t.prev` is an instance under §4a that
ended with `buyerBps` nonzero, else `StageHeld`; then the buyer's consent:
with `sig` empty the caller is `t.buyerKey`'s current key (§9), else
`NotBuyer` (the direct form), otherwise `sig` is a signature by that current
key over `Cancel()` under the instance's domain (`eravon.terms/1` §4.5,
§5.2) sent by `sellerKey`'s or `buyerKey`'s current key (`NotParty`
otherwise, decided before the signature), else `BadBuyerSig`; then the
balance of the settlement token beyond what the instance already owes (from
a run of the expiry branch whose push failed) is credited to `buyerPayout`,
`Refunded(4)` and `Closed(0, credited, 0, 0, 0)` are emitted with the
terminal `Refunded`, everything owed is pushed best effort (what the token
refuses stays owed for `sweep`), and the instance holds a closed case with
`sellerKey` and `buyerKey` kept, so that `init` after it is
`AlreadyInitialized` and `sweep` knows who may call it (§9). A `cancel` and
an `init` race: whichever lands first decides, and a buyer whose `init`
landed first asks the seller for a refund (§9, `refund`) or waits for the
case's clocks (§5: the offer clock or the draw's deadline before an
acceptance, T1 after it). Nothing is lost on either side by a cancellation:
no deposit was pulled and no seat was opened.

**4a. Stages.** An order's `prev` chains a stage to the stage before it
(`eravon.terms/1` §4.4), and the instance reads the stage at `prev` through
one view, `stage()`, which every instance answers: `(sellerKey, buyerKey,
ended, buyerBps, judge, caseId)`, the two keys as recorded (zero on an
instance that holds no case and ran no expiry branch), `ended` whether the
instance reached a terminal, was cancelled, or ran the expiry branch,
`buyerBps` the buyer's share of the amount at that end in basis points (the
table's `buyerBps` column where it is a share, §8: 10000 at Refunded of
every cause, at `cancel`, at the expiry branch and at Resigned; 0 at
Released; the signed `buyerBps` at Settled; the ruled rung's `buyerBps` at
Ruled; the share `lapseDefault` names at Lapsed; 0 with `ended` false before
any end), `judge` the series judge as of this stage (below), and `caseId`
the salt the instance proved its address under (§1; zero until `init`,
`cancel` or the expiry branch, each of which records it). The account at
`prev` is an instance of this standard where it carries `proxyCodeHash` and
is the address §1 derives under the proxy initcode from the `caseId` it
answers: an account that carries the proxy's code but whose storage no order
wrote proves no address and is no instance, however it answers. `init` on an
order whose `prev` is nonzero requires, after the two consents, the account
at `prev` to be an instance that answers this order's `sellerKey` and
`buyerKey` (`BadPrev`), and to have ended with `buyerBps` zero
(`StageOpen`): the stage before left the buyer no share of its amount, by
the rows of §8's table whose share is 0 (Released by confirmation or T2, a
ruling at a rung whose `buyerBps` is 0, a settlement at 0, a lapse at 0),
before the next stage opens, and no stage opens out of order, over a failed
one, or over one not yet opened. `cancel` on such an order requires the
account at `prev` to be an instance that ended with `buyerBps` nonzero
(`StageHeld`), reading `ended` and the share alone and no key, the chain's
integrity on this side being the seller's client's check (`eravon.link/1`
§10): the buyer takes back a later stage's money only once the stage before
failed it, by the rows whose share is nonzero (every Refunded, Resigned, a
ruling, settlement or lapse that leaves it a share, a `cancel`, or an expiry
branch: a stage nobody opened, which the buyer deploys and runs the branch
on first, as any party may), and never while that stage is in flight, before
it is opened, or after the seller earned it whole; a `prev` that is no
instance of this standard opens nothing and is cancelled by nobody, the
stage's money leaving by its own expiry branch alone, the chain being the
buyer's own writing and a seller's client taking no such receipt
(`eravon.link/1` §10). The stages of one sale are instances of one logic on
one chain: `prev` is read on the chain of the call under this generation's
`proxyCodeHash` alone, while an order's address is the same on every chain,
so a stage whose stage before was written under another generation opens for
nobody and is cancelled by nobody, and a series begun under a generation
runs out under it (§13); on another chain `prev` reads only that chain's
account at the address, which opens or ends only by that chain's calls (the
buyer may deploy it there and run its expiry branch or cancel it, as for a
stage nobody opened), so the lock holds on the chain the series was written
for and nowhere else. Stages cancel in order, each `cancel` ending its
instance Refunded with `buyerBps` 10000 and so freeing the next; the expiry
branch of a later stage (`openBy`, `expiry` or a revocation) runs whatever
the stage before did, so a seller sets a later stage's `openWindow` and its
offer's `expiry` so that its `openBy` (`eravon.terms/1` §5.5: the earlier of
`expiry` and `openWindow` seconds after the buyer's signing, or after E for
a buyer that consents by call) falls after the latest second at which the
stages before it can still end under their windows and quotes (§5), and a
stage whose `openBy` passes returns the buyer's money to the buyer. The
series judge is the judge who last heard a stage of the sale: the judge of
the stage's last seat where that seat was accepted and has not ended by its
judge's resignation or by a lapse, else the judge the stage before handed
on, which `init` records from the stage before's answer when its gate passes
(zero on a first stage; a zero handed on, by a first stage no seat served or
by a Lapsed stage, stays zero on each later stage until one of its seats
serves), and zero where the case ended Lapsed, whatever the cause, a judge
that let a stage lapse carrying nothing forward; an inherited judge that
declines a seat of a later stage, lets that seat's offer run out unanswered,
or resigns its accepted seat there before any dispute stays the series judge
where no later seat of that stage serves, and no draw is made to replace it
as series judge for that alone; where it resigned at Claimed, the buyer's
dispute lapses that stage at once (§6), which hands on no judge, and that
dispute is how the parties end a series they will not go on with; a later
stage on the guild track seats this judge first at its draw (§7). The read
of `stage()`, made only where the account carries `proxyCodeHash`, is a
`staticcall` handed at most 100,000 gas whose answer must be exactly 192
bytes of six well-formed words, the sixth deriving the account's own
address; anything else (a revert, another length, a word out of its type's
range, another address) is `BadPrev` on `init`, `StageHeld` on `cancel` and
no preference at the draw, the length and form tests being defensive behind
the code hash test, which fixes the answer's form. The standard reads no
`series` and nothing of the documents: which stages belong together is the
parties' declaration in their orders, and a client checks the chain before
it acts (`eravon.terms/1` §8, `eravon.link/1` §10).

**Revocation.** `revoke(seller, termsHash)` on the logic (an instance
reverts `NotRegistry`), sent by `seller`'s current key (else `NotCurrent`,
§9), records `revoked[seller][termsHash]` and emits `Revoked(seller,
termsHash)`; it is never withdrawn. It is the seller's way of withdrawing a
document from sale: every offer under that document naming `seller`, signed
or registered, is dead, a buyer's money sent afterwards leaves by `init`'s
step 3 at once, and a case already open is untouched. A seller that wants a
document for one buyer alone keeps `buyerMayOpen` `false`, hands the link
privately, and opens the case itself.

An offer whose clock has run out and on which nobody has called `advance` is
still in flight, so the judge may still decline it by `resign` until then
(`galeed.venue/1` §2.10 takes a resignation on any seat in flight); on the
named track a decline ends the case as Resigned and `advance` as Refunded,
the buyer refunded in full either way and the deposit back to the seller; on
the guild track either arms the next draw or, with three seats opened or,
under a frozen record that names no router, the pool exhausted, ends the
case as Resigned with form 3 (§7).

`accept(caseId, seat)` is accepted only from the seated judge's current key
as a direct call (`msg.sender == current(judge) == tx.origin`, no code at
the caller, `galeed.venue/1` §2.7), on a seat in state `seated` with
`accepted == 0`, at or before the offer's `windowEnd`; it sets `accepted`,
starts T1, emits nothing of the interface. `resign` on an unaccepted seat is
the decline and leaves `accepted` zero.

## 5. States, windows, ranges

The case's own machine has `None, Funded, Claimed, Disputed` and six
terminals `Refunded, Released, Settled, Ruled, Resigned, Lapsed`; the
`Closed` event's terminal byte numbers them 0 to 5 in that order. Each seat
carries its own venue state, window and `accepted`; only the last seat of
a case is ever in flight, a case in `Claimed` whose last seat resigned has
no seat in flight and runs on its clock alone (§6), and a guild case whose
draw is armed has no seat in flight and runs on the draw's deadline (§7).

| case | seat state | window on the seat |
|---|---|---|
| Funded, no seat in flight (a guild draw armed) | — | the draw's deadline `[armed, the earlier of armed + offerWindow and block drawBlock + 256]`, the block after the arming drawing whatever the clock (§7) |
| Funded, `accepted == 0` | 0 seated | the offer clock `[seated, seated + offerWindow]` |
| Funded, `accepted != 0` | 0 seated | T1 `[accepted, accepted + deliveryWindow]` |
| Claimed | 0 seated | T2 `[claimed, claimed + confirmWindow]` |
| Claimed after the seat resigned | 3 resigned | T2 kept; `confirm` or T2's end releases, `dispute` lapses, `refund` and `settle` end the case, the seat staying `resigned` (§6) |
| Disputed | 1 disputed | T3 `[disputed, disputed + rulingWindow]` |
| Ruled | 2 ruled | T3 kept |
| declined offer, or a resignation after acceptance (the case Lapsed by a dispute while the seat was resigned, or by a resignation during the dispute, included) | 3 resigned | the window in force kept |
| Lapsed by T3 | 4 lapsed | T3 kept |
| Refunded, Released, Settled, with the last seat in flight; an expired offer | 5 closed | the last window kept |
| a guild seat that ended unaccepted before the next draw | 3 or 5 as above | kept; the next ordinal opens in flight when drawn (§7) |

`rungs` is the ladder's length (2 to 16), `rungMin` 0, `rungMax` `rungs − 1`
throughout: a dispute names no rung and narrows nothing. `token` and
`amount` are the order's; every rung moves the whole amount, so `amount` is
A throughout. `parties` is `[buyerKey, sellerKey, sellerFund, buyerPayout,
sellerPayout]`, five positions, never deduplicated, each the address the
order recorded whatever key the succession book now answers for a party
(§9).

`windowEnd` is the last second an action the window governs is accepted
(`accept`, `claimDelivery`, `dispute`, `rule`, and `resign` on an accepted
seat); `confirm` and `settle` are taken in their phase whatever the case's
windows, a `confirm` past T2 included, and so is `refund` outside a dispute;
while Disputed the seller's own `refund` is refused at or before T3's end
and is the plain refund past it, and `refundBySig` is refused throughout
(`WrongPhase`, §8, §9); `settle` reads the windows only to decide where P
goes (§8) and refuses a message past its own `expiry` (`SettleExpired`), and
`refund` reads T3's end only to decide whether it is taken, P at a refund
following the phase and the seat (§8). `advance` moves a seat only when `now
> windowEnd`. `advance(caseId, seat)` by anyone: an unaccepted seat past its
offer clock goes to `closed` (named track: the case is Refunded with cause
1, the deposit back to the seller; guild track: the next draw is armed, or
the case ends as Resigned with form 3 where three seats have opened or,
under a frozen record that names no router, the pool is exhausted, §7); an
accepted seat in Funded past T1 goes to `closed` and the case to Refunded
with cause 0, the deposit back to the seller; Claimed past T2 to `closed`
and Released, P to the judge where the seat is seated and back to the seller
where it resigned, a resigned seat staying `resigned`; Disputed past T3 to
`lapsed` and the case to Lapsed at the share `lapseDefault` names (0 none of
A to the buyer, 1 half, 2 all of it), with P and the deposit back to the
seller and the fee back to the buyer. When nothing is due on the named seat,
a pair never seated included, the call changes nothing on that seat and does
not revert; the other clock it runs, whatever seat the call names under this
case's `caseId`, an ordinal never opened included, is an armed draw's
deadline, or the 256 blocks its seed is served for (§7); a call naming
another `caseId` does nothing.

`Refunded`'s causes: 0 T1 ran out, 1 the offer clock ran out unaccepted,
2 the expiry branch (the offer's `expiry` or the order's `openBy` passed,
or the document revoked, before any case opened), 3 the seller's refund,
4 the buyer's cancellation before any case opened.

## 6. Commitments, ruling, resignation

`commit(caseId, manifest)` is accepted from any address while the case is
Funded, Claimed or Disputed, an armed draw included, and emits
`Committed(caseId, msg.sender, manifest)`, a zero manifest admitted; the
seller's `claimDelivery` emits the same event for its manifest, nonzero
(`ZeroManifest`), with the sender as `by`, the submitter under
`claimDeliveryBySig`.

`rule(caseId, seat, entryId, rung, entry, "")` is accepted only as a direct
call from the seated judge's current key (§4), on a seat in `disputed`, at
or before T3's end, with `rung < rungs`, `extra` empty, and `sha256(entry)
== entryId`; it emits `Anchored(msg.sender, entryId, entry)` once,
`Ruled(rung, entryId)`, settles by the ladder's rung (§8) and leaves the
seat `ruled`.

`resign(caseId, seat, entryId, entry)` is accepted under the same three
conditions on a seat in flight, accepted or not, and on an accepted seat at
or before its `windowEnd` (past it the clock's outcome is due and only
`advance` remains, as for `rule`), with `sha256(entry) == entryId`; it emits
`Anchored(msg.sender, entryId, entry)` once and `Resigned(entryId, form)`
with form 0 for a declined offer, 1 before a dispute, 2 during one. What
follows depends on where the case stands, and is the same on both tracks
once a seat is accepted:

- **declined (form 0)**: on the named track the case ends as Resigned,
  the amount back to the buyer and the deposit back to the seller; on the
  guild track the case stays open and the next draw is armed (§7);
- **accepted, case Funded (form 1)**: the case ends as Resigned, the amount
  back to the buyer and the deposit back to the seller, the judge taking
  nothing; a resignation and a `claimDelivery` race while Funded: whichever
  lands first decides, a resignation first refunding the buyer in full, and
  a seller's client hands the goods over only once its claim's block is
  final, since from then on no act of the judge alone takes the sale away
  (after a resignation the buyer's dispute lapses the case at the share
  `lapseDefault` names: the Claimed form below);
- **accepted, case Claimed (form 1)**: the seat goes to `resigned` and the
  case stays Claimed with T2 running: `confirm`, or `advance` past T2,
  releases the amount to the seller with P back to the seller; a `dispute`
  while the seat is resigned pulls no fee and ends the case at once as
  Lapsed at the share `lapseDefault` names, the deposit back to the seller;
  `settle` and `refund` stay open as in any Claimed case, P back to the
  seller at either;
- **accepted, case Disputed (form 2)**: the case ends at once as Lapsed at
  the share `lapseDefault` names, the buyer's fee back to the buyer and the
  deposit back to the seller.

A judge who resigned after accepting is not redrawn on the guild track:
draws follow a decline or an unaccepted seat's expiry alone (§7), so
that one case cannot run a guild's whole pool through one resignation
after another.

`Anchored` is emitted from those two paths and nowhere else.

## 7. The panel face

The instance's own `engage(termsHash, members, amanahHashes, rosterHash,
expires, router)` reverts (`NoPanelFace`). The panel face lives on the panel
book, `EravonPanelBook`, one per chain, deployed by whoever first needs it
through the public deployer under the salt `keccak256("eravon/1/panel")` and
designated by code: the instance derives the book's address as

    book = keccak256(0xff ‖ deployer ‖ keccak256("eravon/1/panel") ‖ panelInitcodeHash)[12:]

with `panelInitcodeHash` from CODEHASH.md (`panelInitCodeHash()` on any
instance or the logic serves it, and `panelBook()` the address), and checks
`extcodehash(book) == panelCodeHash` before reading. The book's
`successionBook()` and `routeGas()` answer the constants every instance
answers (§10). A guild sends `engage(termsHash, members, amanahHashes,
rosterHash, expires, router)` to the book from its current key, with
`termsHash` = `proxyCodeHash` of this standard: the book's key is the
`termsHash` argument of `galeed.venue/1` §4 and is unrelated to any terms
document's digest; `rosterHash` is the roster document's `word`
(`galeed.roster/1` §1); `router` is zero or a router the guild names for its
draws. The book refuses, in this order, a sender that is not the current key
of its own record on the succession book (`NotCurrent`), unequal arrays
(`LengthMismatch`), an empty `members` (`EmptyMembers`), then element by
element a zero word (`ZeroWord`), two words that differ (`WordsDiffer`),
then an `expires` not after the block's timestamp (`ExpiresNotInFuture`) and
a nonzero `router` that carries no code (`RouterNoCode`); it appends the
record under `(root, termsHash)`, `root` being the sender's root on the
succession book, with the panel's hash `keccak256(abi.encode(members,
amanahHashes))`, `rosterHash`, `expires`, `router`, the router's code hash
at the call (zero where `router` is zero) and the block's number; its read
views are `count(root, termsHash)` and `panelAt(root, termsHash, index)`,
and every record is also in the `Engaged` event with the members and words
in full. The book records `members` as sent, in its order and with any
repeat: a panel may list a member twice (`galeed.venue/1` §4), and every
draw compares members by address.

Which record a case draws from: `init` reads the guild's root on the
succession book, `guild` being the order's, and freezes it with the panel,
which is the latest record under `(root, proxyCodeHash)` recorded in a block
before `init`'s and unexpired at `init` (its `expires` at least `now`: the
panel may still be frozen in the second `expires` names, and no later),
found by walking back from the newest record, so that a panel recorded in
`init`'s own block or after it never reaches the case and an expired latest
record gives way to an older one still unexpired. `init` requires, on the
verifier (§4 step 6), the book at the derived address to carry
`panelCodeHash` (`PanelBookMissing`), such a record to exist (`NoPanel`),
the panel it was handed to hash to that record's panel hash
(`PanelMismatch`), every word of `amanahHashes` to equal the order's
`amanahHash` (`PanelWord`), and, where the record names no router, the pool
to be nonempty (`NoMembers`), the pool being the distinct members that are
neither `sellerKey` nor `buyerKey` nor zero, the two keys compared as the
order recorded them and no book read, as guard 3 compares the named judge
with them (a party trades under any key it holds, so the exclusion names
keys and never persons); it stores the root, the record's index, its panel
hash, its router and the router's code hash, and the pool's size, emits
`PanelFrozen(guild, root, index, panelHash, router)`, and arms the draw of
seat 0 in place of opening it. A guild that has recorded no usable panel
makes `init` revert; the money stays at the address and the expiry branch
refunds it later.

The draw. An armed draw records `drawBlock`, the current block, and a
deadline `offerWindow` from now, and emits `DrawArmed(drawBlock, deadline)`.
From the next block on, anyone calls `redraw(members, amanahHashes)` with
the frozen panel (`NotPendingRedraw` where no draw is armed, or where its
deadline has passed and the block is not the one after the arming;
`PanelMismatch` for another panel; `DrawPending` in the block of the arming
itself), the seed being `blockhash(drawBlock)`. Where the frozen record
names no router, the seat opens for the `judge` the instance at the order's
`prev` answers through `stage()` (the series judge, §4a) where `prev` is
nonzero and an instance, that judge is nonzero, is in the pool and no seat
of this case has held or offered it, so that the stages of one sale come
before the judge who last heard one while the guild keeps it on the panel;
otherwise for

    i = uint256(keccak256(caseId ‖ blockhash(drawBlock) ‖ be64(seat))) mod n

over the n members of the pool not seated before on this case, each counted
once, in the order of its first place in the panel, `seat` being the new
ordinal as eight big-endian bytes; the deadline, the seed's 256 blocks and
the closing where neither is met govern both forms alike. A router, where
the record names one, is asked at every draw as `galeed.venue/1` §4
requires, `RouteView` carrying no stage; a guild that wants one judge for a
whole sale names a router that draws by the parties `RouteView` carries,
which is the guild's affair. Where it names a router, the instance requires
the router's code hash to equal the frozen one, calls `route(RouteView)` on
it by `staticcall` with exactly `routeGas()` gas (`galeed.venue/1` §4),
reverting the whole call (`RouterGas`) where the gas left at that point is
not above `routeGas()` + ⌊`routeGas()` / 63⌋ + 20,000, so that the router is
always handed exactly `routeGas()`, and opens the seat for `members[index]`
where the call returns exactly one 32-byte word whose `index` lies inside
`members` and names a nonzero member no seat of the case holds, whoever that
member is, a party's key included (the zero address is no judge address,
`galeed.venue/1` §4, no key acting for it): who sits on a guild's panel and
which router draws from it are the guild's affair, and the guild answers for
them; `RouteView` carries `caseId`, the new ordinal, the seat's `termsHash`
(the terms document's sha256, §2, and never the book's key), `token`,
`amount` and the number of rungs, `parties` as `seatOf` reports them (§5),
the frozen `members`, every judge seated or offered on the case in the order
of the seats' ordinals, the seed, the block's timestamp, the chain id and
the instance's address. A code hash that differs, a call that reverts or
runs out of that gas, an answer of any other length, and an `index` outside
`members`, naming the zero address or naming a member a seat of the case
holds leave the case with no usable panel: the call ends the case as
Resigned with form 3, the amount back to the buyer and the deposit to the
seller. The seat's clock starts at the draw. `init`'s read of a panel grows
with the square of its length, and so does a draw where the record names no
router, whose cost grows also with the place in the pool the seed lands on,
the draw landing on the pool's last member costing about twice the one
landing on its first; each act is bounded by the gas one transaction may
spend on the chain, the lesser of its block gas limit and its transaction
gas cap where the chain sets one (2^24 under EIP-7825 on chain 1), so a
party's client tests, before it pays, that `init` and the costliest draw
under the panel each fit one transaction (`eravon.link/1` §10): where the
record names no router, the `redraw` landing on the pool's last member at
the case's third seat, simulated as the transaction that lands it (the
instance's storage set to that seat and to the order's `prev`, the stage at
`prev` set as ended at share 0 and answering as its series judge the pool's
member standing immediately before its last, that member seated on the
case's second seat (ordinal 1) and the draw landing on the last, so that the
draw pays for the read, the walk to it and one more pass before the pick,
whatever judge the stage before in fact had; the arming block's hash to a
seed that lands there; the transaction's intrinsic gas counted), the
verifier's `draw` alone falling short of it by the instance's own reads and
writes; where it names a router, a `redraw` whose router spends all of
`routeGas()`. The block after the arming may always draw, whatever the
deadline, so that no `offerWindow` makes a draw impossible; from the second
block on the deadline governs. Where `blockhash(drawBlock)` is no longer
served (more than 256 blocks have passed, or the chain serves none), the
call, where the deadline still admits a draw, closes the case as Resigned
with form 3, the amount back to the buyer and the deposit to the seller;
`advance` reads no hash and closes the same way from the second block after
the arming on, once the deadline has passed or more than 256 blocks have (on
a chain that serves no hash, `redraw` closes at once and `advance` waits for
one of the two): a seed nobody drew on buys no second one, so waiting can
end a case and never choose its member where the frozen record names no
router; a router's answer, and whether it answers within `routeGas()` at
all, may turn on anything its static call can read beyond the seed: the
block (its time and number, its randomness, producer and base fee), the
transaction (its origin, its gas price, and the accounts and slots it has
touched, which set what each read costs) and the chain's state as the
block's earlier transactions and the drawing transaction itself left it (the
router's own storage and any code it delegates to, another contract's word,
a balance any address may move), the frozen code hash fixing the router's
code and nothing it reads, and a router whose code is an EIP-7702 delegation
designator being pinned by that hash to the delegate's address and to none
of the delegate's code; whoever sets any of these, by waiting, by sending a
transaction before the draw or the draw itself, by writing that state or by
producing the block, chooses what the router answers, a member it can name
or a failure that ends the case. Nobody who sends an arming transaction
(`init`, a decline, or the `advance` that runs out an offer clock) can see
the hash its block will carry; the producer of the arming block can (a
builder or proposer varying its header, a single sequencer every block), and
a chain whose `BLOCKHASH` is not the consensus hash of that block, or serves
none, gives the draw to whoever can foresee that hash (the producer; where
the hash is computable in advance, whoever times the arming transaction) or
to nobody. A client offers the guild track on chains whose block hashes come
from consensus. Anyone may land `redraw`; every party's client and the
guild's desk send it in the block after `DrawArmed`. The `caseId` is the
order's struct hash, which the buyer fixes when it chooses `buyerPayout` and
`openBy` before it signs (`eravon.terms/1` §4.4), so that a router whose
draw turns on `caseId` without the seed (`GaleedRouter`'s `ROTATION`, below)
lets the buyer choose the member, and the block's timestamp a router is
handed is that of the block `redraw` lands in, so that a router that reads
it leaves the choice among the blocks a draw may land in (up to the
deadline, and at most 256 blocks) to whoever lands the first `redraw` and to
that block's producer; a router is the guild's choice.

A router the guild names is called by whoever lands `redraw`, in the block
and in the transaction that sender chooses, so a router whose answer, or
whether it answers within `routeGas()`, turns on anything the paragraph
above names beyond the seed (the block; the drawing transaction itself, the
accounts and slots it has warmed, which set what each read costs, and
whatever it wrote before the call, the router's transient storage included;
state another transaction can move) lets a stranger choose the member by
choosing the block or the transaction, and a router that fails under some
such choice (reverts, runs out of `routeGas()`, answers other than one word,
or answers an index this section does not seat) lets a stranger end the case
with no usable panel (Resigned, form 3) by landing `redraw` under it. A
router whose answer turns on nothing but `caseId`, `seed`, `seat`,
`termsHash`, `token`, `amount`, `rungs`, `parties`, `members`, `seated`,
`chainId` and `venue`, and which for every such view returns or reverts
within `routeGas()` with every account and slot it reads cold, answers the
same, a member or a failure, in every block and from every sender, and gives
a stranger nothing to choose. The family's standard router (`GaleedRouter`,
codeHash
`0x492e67518544e136c69897b93df1555ccf722f438fbb5bbd3d00f9a1f6348986`,
galeed-core `CODEHASH.md`) is of this kind in its three modes where every
route its table can take returns or reverts within `routeGas()` with the
table's account cold; that account's first touch in a transaction costs
2,500 gas more than a later one, so a table whose route needs more than
`routeGas()` with it cold and no more with it warm lets whoever lands
`redraw` choose between a seat and no usable panel: a guild keeps every
route its table can take under `routeGas()` with the table cold. Guarding a
router against this is the guild's affair; this standard pins a router's
code hash, tests its answer as above, and reads nothing of how the router
reaches it.

After an unaccepted guild seat ends (declined or expired): if the case has
opened three seats, or the frozen record names no router and every member of
the pool has been seated, the case ends as Resigned with form 3 (the
`Resigned` event's `entryId` zero, emitted by the transaction that closes,
which on a decline is the same transaction that emitted `Resigned(entryId,
0)` for the seat, two logs in one transaction), the deposit back to the
seller; otherwise the next draw is armed as above, and `advance` with any
seat argument, from the second block after the arming, past the deadline or
more than 256 blocks after `drawBlock`, closes the case as Resigned with
form 3. The first draw runs under the same deadline, so a case nobody draws
a judge for closes the same way, the amount back to the buyer and the
deposit back to the seller; `refund` and `settle` stay open while a draw is
armed, P back to the seller at either (§8). A panel frozen into a case
serves it to the end whatever the book records later.

## 7a. The deposit face

A seller's deposit is an allowance, never a balance held by the logic.
Before a case whose deposit is nonzero can open, the order's `sellerFund`
grants the logic an ERC-20 allowance on the settlement token
(`approve(logic, n)` on the token, a standard act sent from that address,
adjusted at any time by the same act); the money stays at `sellerFund`, and
the deposit a client shows is the smaller of that address's balance and that
allowance. A seller whose deposit sits at its own address names its
`sellerKey`; a seller whose deposit sits elsewhere (a company whose capital
is in its wallet, or a seller funding from another address) names that
address, which must first have bound itself to the seller:
`acceptFunding(seller)` on the logic alone (an instance reverts
`NotRegistry`) records `funds[seller][msg.sender] = true`, only ever set and
never cleared, and emits `FundBound(seller, msg.sender)`; the view
`funds(seller, fund)` answers on the logic; a funder withdraws by setting
its allowance to zero on every token it approved, and stays withdrawn only
while it grants the logic no allowance again: there being no function to
unbind, a bond lasts as long as the standard, and every allowance the
address grants the logic afterwards, on any token and for whatever document,
is again open to every seller it ever bound, under every offer of theirs
still live that names it (a contract that holds a seller's capital binds
itself by its own call of `acceptFunding(seller)`, whatever code makes that
call; the logic reads no code at either address). A bond and the allowance
are the funder's whole consent: every case opened under an offer the bound
seller's key consented to that names the funder pulls its D from the
funder's allowance on that case's token, whatever seller the allowance was
granted for, and the part of D §8 gives away leaves the funder for good (P
where it goes to the judge, the seller's LF or its half where it goes to the
buyer), as the seller's own choice of judge, quote and ladder decides; a
funder binds only a seller it trusts with the whole allowance on every token
it has approved, and funds sellers it trusts apart from separate addresses,
since setting the allowance to zero withdraws it from every seller at once,
and granting one again grants it to every seller at once. `init` tests the
bond at step 5: `sellerFund` byte-equal to `sellerKey`, or `funds(sellerKey,
sellerFund)` holding, else `FundUnbound`, whether or not the deposit is
zero. A bond names the `sellerKey` it was made for and serves every offer
naming that key, whichever key now consents for it (§9), so that it outlives
the seller's move on the succession book, and a funder that funds offers
naming the seller's new key binds that key as well. The logic has one
function for the money, `pull(seller, fund, token, amount)`, accepted only
on the logic itself (an instance reverts `NotRegistry`) and only from a
caller whose `extcodehash` is `proxyCodeHash`, the logic's own proxies, else
`NotInstance`; it runs `transferFrom(fund, msg.sender, amount)` on the
token, the receiver being the calling instance and never another address,
under a reentrancy lock, and emits `Pulled(seller, fund, token, msg.sender,
amount)`, which a reader lists under the logic's address. Inside an instance
the one path to `pull` is `init`'s step 7, reached after the address proved
itself, the seller's consent held, the bond held and the seat's
preconditions passed, for the sum P + `lf` of that order as §3 fixed them,
where that sum is nonzero, and the instance hands `pull` its own frozen
`sellerKey` and `sellerFund`; the instance measures what arrived by its own
balance and reverts `DepositShort` where it is less. Nothing in the logic
receives, holds or pays money, and the only state the logic shares among
sellers is the code and the three registries; sellers whose orders name one
`sellerFund` share that address's allowance and balance besides (below). The
deposit returns to `sellerFund` at every terminal but those §8 gives it to
the judge or the buyer, so that a returned deposit refills the balance the
allowance draws on. The allowance and the balance are one pool for every
document `sellerFund` funds: each case opened pulls its D from both, and a
returned deposit refills the balance and not a finite allowance, so the
allowance bounds the deposits of every case it ever funds and the balance
those of the cases open at once; a seller whose documents buyers may open
(`buyerMayOpen`) exposes the pool to anyone funding an address under a
public listing; a seller's client keeps `buyerMayOpen` false where D is
large against A, or funds each public listing from its own address. A token
that deducts a fee on transfer delivers less than `pull` asked for, and the
instance refuses the case; where P + `lf` is zero nothing is pulled and such
a token opens a case, its fee falling on each payee (§8).

## 7b. The broadcast face

`EravonBoard` carries one function, `post(key, document)`, which any address
may call with any bytes: it emits `Posted(address indexed poster, bytes32
indexed key, bytes32 indexed hash, bytes document)`, `hash` being
`sha256(document)` computed through the precompile, and does nothing else:
no storage, no owner, no check on the bytes. A seller posts its listing
(`eravon.link/1` §2) under the key `termsHash` and its settlement offer
(`eravon.link/1` §8) under the key `caseId`, the listing from `sellerKey`
(`eravon.link/1` §2.7) and the settlement offer from `sellerKey` or any
address it likes; a reader takes `Posted` only from an account whose code
hash is `boardCodeHash` (CODEHASH.md), finds a document by the `key` or the
`hash` topic, and authenticates it by its content and the signatures it
carries (`eravon.link/1` §2.5, §8.3) whoever posted it, a listing's place
and whether a buyer pays under it coming from its seller's own posting alone
(`eravon.link/1` §6.2, §6.4); a party's client reads the registry for a
by-call slot before it acts on it (`eravon.link/1` §10), and a reader of an
opened case reads none for the listing (§2). The link a seller hands a buyer
is a pointer to the posting transaction and the document's `hash`
(`eravon.link/1` §3). A bad document posted under someone's key costs its
poster the gas and spoils nobody's link, since a link names its document's
`hash`.

## 8. Money

The instance holds at most three sums of the settlement token beyond
strays and credits (the surplus of `init` among them): the amount A, the
seller's deposit D = P + LF pulled at `init`
(§7a), and the buyer's litigation fee LF, equal to `lf`, pulled when the
buyer disputes. P = ⌊A × premiumBps / 10000⌋ is the judge's standing fee,
paid out of D and never out of A: the buyer's share of A is
⌊A × buyerBps / 10000⌋ and the seller's the rest, at every terminal.

Inflows of the settlement token before the dispute, while Funded or Claimed,
are the buyer's: a buyer may transfer the fee first and dispute next, which
a buyer whose record moved and that keeps no allowance at `buyerKey` does,
since the fee is pulled from `buyerKey`'s own address whatever key now acts
for it (§9), and `dispute` takes the fee from whatever flowed in unrecorded
since `init`, pulling the rest by `transferFrom(buyerKey, instance, rest)`
under the buyer's allowance to the instance, never to the logic, with the
return test of §4 step 7 (`Underfunded` where less than the fee is there
after the pull, `TransferFailed` where the pull fails); what flowed in
beyond the fee, and every inflow afterwards, is the buyer's stray, returned
to `buyerPayout` at the terminal, and after a terminal every remaining
balance of the settlement token goes to `buyerPayout` through `sweep`. A
token other than the settlement token sent to an instance is the buyer's
stray too: `sweep(who, token)` by a party or a seated judge (§9) pushes all
of it to `buyerPayout` once `init`, `cancel`, or the expiry branch has fixed
that address, and refuses (`NotForeign`) a token whose transfer lowers the
instance's balance of the settlement token, so a token with two entry points
over one ledger never leaves by that door. Native coin sent to an instance
has no exit; nothing of this standard sends or receives it.

A settlement token of this standard is one whose `balanceOf` answers the
instance with one word and whose balance at the instance changes only by the
instance's own transfers and by inflows the instance measures: `init`
measures the amount that arrived against the order's `amount` and the
deposit that arrived against P + `lf` and refuses a case short of either, so
a token that deducts a fee at the receiver opens no case where P + `lf` is
nonzero; a token that charges the sender, rebases, pays interest at the
holder, or lets a recipient's hook act is outside the rule, and under one
the last payee may find its credit short or the ledger's sums off. A
recipient's hook stops no push of the standard's own: each best-effort push
(a terminal's, `cancel`'s, the expiry branch's) runs under a gas allowance
of 1,000,000, or under what the submitter's gas limit has left it where that
is less, and copies back no return data (a push counts by the balance that
left, below), and what it does not deliver stays owed for `sweep`, whose
caller, like `withdraw`'s, hands the token all the gas it brings. The
instance's reads of the settlement token's balance and its two pulls take
the token's whole answer: a settlement token whose own answers are long, or
spend their gas, makes the calls that read it pay for them, and a balance
read that runs out of gas, reverts or answers fewer than 32 bytes reads
zero, so a push whose read after the transfer runs out is debited in full
whatever left the instance, and under a token whose `balanceOf` spends close
to a million gas and whose `transfer` can answer without moving, whoever
submits a `sweep` or a terminal at a gas limit it chooses turns a credit the
push did not deliver into a stray; the standard bounds this for no token,
the token being the parties' own choice.

The terminal table, as `_finish` implements it through the verifier's
`split`: P goes to the judge where the trade completed under a seated judge
(Released with the seat seated), where the parties settled under an accepted
seat in flight at or before its `windowEnd`, or under a seated seat while
the case is Claimed at any time, since T2's lapse releases with P to the
judge (Settled), where the seller refunds under a seated seat while the case
is Claimed (Refunded cause 3, for the same reason: a refund followed by a
payment off the instance would otherwise complete the trade with P
stripped), and where the judge ruled (Ruled); everywhere else, the case
unfinished and unruled, the seat resigned, unaccepted, past its clock or
never drawn, P returns to the seller, the standing fee being the seller's
cost of doing business that a judge who did not finish does not collect. The
buyer's fee goes to the judge on Ruled and back to the buyer wherever it was
pulled otherwise. The seller's LF, the second half of D, returns to the
seller except on Ruled, where the ruled rung's `feeBearer` sends it to the
buyer (1), sends ⌊LF / 2⌋ to the buyer and the rest to the seller (2), or
returns it (0). The seller's `refund` is refused while Disputed at or before
T3's end (`WrongPhase`), and its signed form throughout a dispute, so that
no ordering of a buyer's `dispute` ahead of a seller's `refund` can turn the
refund into another row: a seller leaves a running dispute by `settle` or by
the ruling, and past T3's end its `refund` is the plain refund. While no
seat has accepted, a settlement and an acceptance race: an acceptance that
lands first (the judge's `accept`, or under `autoAccept` a `redraw`, which
anyone sends) makes the settlement pay P to the judge; parties settling for
the buyer's whole amount use `refund`, which returns P whichever lands
first.

| terminal | buyerBps | P (seller's deposit) | buyer's fee | seller's LF |
|---|---|---|---|---|
| Refunded cause 2 (expiry, `openBy`, or revocation before any case; no case, `Refunded(2)` alone and no `Closed`) | the balance beyond what is owed | never pulled | — | never pulled |
| Refunded cause 4 (the buyer's cancellation before any case) | the balance beyond what is owed | never pulled | — | never pulled |
| Refunded cause 1 (the offer clock ran out, named track) | 10000 | seller | — | seller |
| Refunded cause 0 (T1 ran out) | 10000 | seller | — | seller |
| Refunded cause 3 (the seller's refund) while Claimed under a seated seat | 10000 | judge | — | seller |
| Refunded cause 3 otherwise (Funded, a draw armed, Claimed after the seat resigned, or Disputed past T3's end) | 10000 | seller | buyer, where pulled | seller |
| Released (confirm or T2), seat seated | 0 | judge | — | seller |
| Released after the seat resigned | 0 | seller | — | seller |
| Settled, last seat accepted and in flight at or before its `windowEnd` (seated or disputed), or seated while Claimed at any time | the signed value | judge | buyer, where pulled | seller |
| Settled otherwise (the seat unaccepted, resigned, closed, past its window while Funded or Disputed, or no seat drawn) | the signed value | seller | buyer, where pulled | seller |
| Ruled at rung r | `ladder[r].buyerBps` | judge | judge | by `feeBearer` |
| Resigned (declined on the named track, form 1 while Funded, or form 3: the pool exhausted, no draw made, or no usable panel) | 10000 | seller | — | seller |
| Lapsed (T3 ran out; a dispute after the seat resigned; or a resignation during the dispute) | the share `lapseDefault` names: 0, 5000 or 10000 | seller | buyer, where pulled | seller |

The deposit, where it returns, returns to `sellerFund`, the address it was
pulled from (the `seller` of the deposit columns in the table above, and of
every return of the deposit, P or the seller's LF in this document, is that
address); the seller's share of A goes to `sellerPayout`. Parties' credits
are `owed[payout][token]`, served by the view `owed(payout, token)` and
pushed in the transaction that credits them (a terminal, the expiry branch,
`cancel`) by a best-effort transfer under the gas allowance above, except
the surplus credited at `init` (§4 step 8) and a stray that `sweep` credits
after a terminal, which wait for a `sweep` naming `buyerPayout` (the surplus
also for the terminal): a credit is debited by the balance that left the
instance where both reads answer, whatever the token answered and never by
more than the credit (a balance read that reverts or answers fewer than 32
bytes reads zero, so a token whose `balanceOf` can fail while its `transfer`
moves money is outside the rule, and under one a credit may be paid twice
out of another's), so a short delivery leaves the remainder owed and a token
that moved more never pays twice; what is still owed is left for `sweep(who,
token)` by a party or a seated judge (§9). `sweep` also pushes a seat's
claimable amount to its registered payout, which `galeed.venue/1` §3's pull
shape does not forbid: a seat's money still leaves only to the address the
seat registered. The judge's earnings are the seat ledger of
`galeed.venue/1` §3, `claimable[judge][token]`, the seated address its key,
leaving through `withdraw(token, judge, payout)` from the judge's current
key (§9, `NotCurrent` otherwise), which first registers `payout` where the
key has none and `payout` is nonzero, refuses a nonzero `payout` other than
the registered one (`PayoutSet`), a zero `payout` or the registered one
changing nothing, refuses where none is registered after that (`NoPayout`),
and answers `TransferFailed` where the push delivers nothing and does
nothing where nothing is claimable; or through `sweep` by a party or a
seated judge once a payout is registered (§9); no party money passes through
it. `setPayout(judge, payout)` refuses, in this order, the logic
(`NotInstance`), a sender that is not the judge's current key
(`NotCurrent`), a second registration (`PayoutSet`) and the zero address
(`ZeroPayout`), accepts a contract, and never requires the payout to be the
sender, and `withdraw` refuses the logic as it does (`NotInstance`); nothing
else registers a payout, neither the order nor the quote carrying one
(`galeed.venue/1` §3). What each seating earned is on the seat
(`galeed.venue/1` §2.3): `seatOf`'s `standbyCredited` is P where a terminal
credited it to the seat's judge and `rulingCredited` the buyer's fee where a
ruling did, each zero otherwise and on every seat but the last.

`Closed(terminal, buyerPaid, sellerPaid, judgePaid, bondToBuyer)` reports
what the terminal credited: to the buyer's payout in all (its share of A,
the strays, its fee returned, and the part of the seller's LF sent to it;
the surplus credited at `init` is outside it), to the seller in all (its
share of A to `sellerPayout` and the deposit returned to `sellerFund`), to
the judge (P and the buyer's fee where it earned them), and the part of the
seller's LF that went to the buyer (`bondToBuyer`). The four sums are by
role: where two of `buyerPayout`, `sellerPayout` and `sellerFund` are one
address, that address receives the sum of their parts, and a reader
reconciling `Closed` against the pushes adds them per address.

`stage()` (§4a) reports, beside the two recorded keys, the series judge and
the salt, whether the instance ended and the buyer's share at that end in
basis points: the table's `buyerBps` column where it is a share, and 10000
at Refunded causes 2 (the expiry branch) and 4 (`cancel`), whose column
names an amount.

**Outcomes.** `partyRoles()` is `buyer,seller`: the buyer role is `buyerKey`
and `buyerPayout`, the seller role `sellerKey`, `sellerFund` and
`sellerPayout`. `outcomeOf(caseId, seat, rung, extra)` reports the row Ruled
at rung r of the table above: `state` 2; `shares` ⌊A × `ladder[r].buyerBps`
/ 10000⌋ and the rest; `standbyFee` P; `rulingFee` `lf`; `rulingFeeFrom` by
the rung's `feeBearer`, 0 giving `lf` and 0, 1 giving 0 and `lf`, 2 giving
`lf` − ⌊`lf` / 2⌋ and ⌊`lf` / 2⌋. On a seat in flight the acceptance and the
dispute are taken as made, the acceptance being the judge's `accept` or the
consent the seating carried, and the dispute the buyer's `dispute` after the
seller's `claimDelivery`, with the fee `lf` the instance pinned at `init`
collected; on the ruled seat it answers for the rung it was ruled at; it
reverts for a rung outside the ladder, for a non-empty `extra`, and on a
seat out of flight but the ruled one for its own rung. `advanceOf(caseId,
seat)` reports what `advance` does by that seat's own clock: an unaccepted
named seat past its offer clock `closed`, `shares` A and 0; an unaccepted
guild seat past it `closed`, `shares` A and 0 where the case then ends
(three seats opened or, under a frozen record that names no router, the pool
exhausted) and zero where the next draw is armed; an accepted seat past T1
`closed`, A and 0; a seated seat past T2 `closed`, 0 and A, `standbyFee` P;
a disputed seat past T3 `lapsed`, the share `lapseDefault` names and the
rest; and, where nothing is due on the seat or the seat is out of flight,
the seat's present state with every amount zero. Both views revert
`UnknownSeat` for every pair `seatOf` refuses (another `caseId`, an ordinal
never opened, a guild case's seat 0 before its draw), as `galeed.venue/1`
§2.12 asks. What else a ruling or an advance moves lies outside the
`Outcome` (`galeed.venue/1` §2.12): the deposit's return, the buyer's fee
returned and the strays, each going back to whoever put it in or being a
party's own, and the part of the seller's LF a `feeBearer` sends to the
buyer, no part of `amount`, which refunds that part of the ruling fee the
buyer paid and which `rulingFeeFrom` books as the seller's. One pure
function on the verifier answers both views and computes the terminal's
split, so that an accepted `rule` or `advance` does what the view reported
on the state it found.

## 9. Actions and signatures

EIP-712 domain `{name: "ERAVON", version: "1", chainId,
verifyingContract}`: the seller signs `Offer` under the logic's address,
the buyer signs `OrderTerms` under the instance's, and both sign action
messages under the instance's, `Cancel` included, which is signed before
any case exists under the address the order derives. The type strings,
struct hashes and digests are `eravon.terms/1` §4.2 and §4.5, and the
verifier's `hashOffer` is that law's `hashStruct(Offer)`.

Both parties sign alike (`eravon.terms/1` §5.1 to §5.3): a signature is
65-byte recoverable secp256k1 (`r ‖ s ‖ v`, `v` 27 or 28, `s` at most n/2,
`r` and `s` in range); any other form, one recovering to no address, and one
recovering to an address other than that party's current key (below) is the
party's fault, `BadSellerSig` or `BadBuyerSig`, checked on the verifier
(`verifySeller`, `verifyBuyer`); on a signed action form an empty slot is
that fault too, and at `init` and `settle` an empty slot is a consent by
call, below.

**The parties' keys.** The instance records `sellerKey` and `buyerKey` as
the order names them, and reads at every act the key that acts for each from
the succession book `successionBook()` names (`galeed.roster/1` §4), a
constant of the code: `current(sellerKey)` and `current(buyerKey)`. Every
party gate compares with that current key: the `msg.sender` of
`claimDelivery`, `confirm`, `dispute`, `refund` and `cancel`; `init`'s
caller rule (§4 step 4); the sender of a signed form, of `cancel`'s signed
form, of `settle` and of `sweep`, and the caller of the expiry branch
(`NotParty`, below, §4 step 3); the caller that consents by call at `init`
and `settle`; and the address a signature recovers to in the four signed
forms, in `init`'s and `settle`'s slots and in `cancel`'s. The logic's
registries of consent and revocation record under the party the call names
and accept the call from that party's current key alone (`consent(party,
digest)`, `revoke(seller, termsHash)`, `NotCurrent` otherwise), so that what
a key registered while current stands after it hands its record on, its
successor registers and revokes under the same name, and a key that handed
its record on writes nothing. What a former key registered while current
stands, a consent to its digest's own expiry and a revocation for good, and
no key withdraws either: a registration is a call the chain placed while the
key was current, where a signature carries no time and shows nothing of when
it was made, which is why the move refuses the one and keeps the other; a
seller revokes the documents it knows (§4). `seatOf`'s `parties` reports the
five addresses as recorded, and `buyerPayout`, `sellerPayout`, `sellerFund`
and every address the instance pays are read as recorded, a move on the
succession book changing where no money goes (`galeed.venue/1` §1); which
key acts and signs for a party is this standard's rule, as `galeed.venue/1`
§2.2 and §9 leave it to the venue. A key's record moves from the accession
on (`galeed.roster/1` §4.2): every case in flight is acted on by the new key
from then on, nothing migrates, and a signature the former key made is
refused from then on.

**Consent by call.** A signature slot passed as zero-length bytes is a
consent by call (`eravon.terms/1` §5.4, the document's empty string; the
document string `0x` spells zero bytes as well and is refused by the
document laws before anything reaches the contract): `init` admits the
seller's slot where `msg.sender == current(sellerKey)` or
`consented(sellerKey, offerDigest)` holds on the logic, and the buyer's
where `msg.sender == current(buyerKey)` or `consented(buyerKey,
orderDigest)` holds; `settle` admits either party's where `msg.sender` is
that party's current key or `consented(party, settleDigest)` holds; at both
the seller's slot is decided before the buyer's. The three digests are
`eravon.terms/1` §4.2 (the offer's, under the logic's domain), §4.4 (the
order's, under the instance's) and §4.5 (the `Settle` message's, under the
instance's): the registry holds the digest a signature would have signed.
`msg.sender` is the address the call leaves from, which need be neither the
transaction's origin nor codeless: a party consenting by its own call is the
caller whatever code its account runs to make the call (its own, a library
it reaches by `DELEGATECALL`, an EIP-7702 delegation's), and a company that
acts by executing proposals is the caller of the inner call and never the
transaction's origin; where the call leaves from an intermediary's own
address, as Multicall3's forwarding sends it, the intermediary is the
caller, so a party consenting by its own call never routes it through one.
The same rule reads `init`'s caller for §4's step 4, the callers of `revoke`
and `consent` for their current-key test and `acceptFunding`'s for its bond,
so that a funder which routes the call through an intermediary binds the
intermediary and its orders meet `FundUnbound`. The registry is the logic's
own storage: `consent(party, digest)` records the digest under `party` on
the logic alone, from `party`'s current key alone (`NotCurrent`), reverts
`NotRegistry` through an instance, emits `Consented(party, digest)` and is
never withdrawn, the digest's own expiry ending its use; `consented(who,
digest)` answers on the logic, and an instance, holding no registry, answers
false. Beside it the logic holds the revocation registry of §4 (`revoke`,
`revoked`, `Revoked`) and the funding bonds of §7a (`acceptFunding`,
`funds`, `FundBound`), each written on the logic alone under the same
`NotRegistry` rule, its view answering false on an instance as `consented`
does; the logic holds no deposit (§7a). The decision runs on the verifier
(`consents`), the party's fault (`BadSellerSig`, `BadBuyerSig`) reaching the
caller unchanged. The four signed forms (`claimDeliveryBySig`,
`confirmBySig`, `disputeBySig`, `refundBySig`) refuse an empty signature
with the same faults, and `cancel` reads an empty `sig` as its direct form
(`NotBuyer` for a caller other than the buyer's current key): a party that
is a contract performs those acts by calling the instance, which reads
`msg.sender` for them, and a key-holding party signs or calls them whatever
form its slots at `init` and `settle` took. ERC-1271 contract signatures are
not read, and no code is read at either party's address: a contract party
consents by call, and a key-holding party that sends the call itself makes
the same claim.

| action | who | message | direct form | signed form |
|---|---|---|---|---|
| cancel (before any case) | buyer | `Cancel()` | `cancel(t, "")` by the buyer | `cancel(t, sig)` by either party |
| claim delivery | seller | `ClaimDelivery(bytes32 manifest)` | `claimDelivery(manifest)` | `claimDeliveryBySig(manifest, sig)` |
| confirm | buyer | `Confirm()` | `confirm()` | `confirmBySig(sig)` |
| dispute | buyer | `Dispute()` | `dispute()` | `disputeBySig(sig)` |
| refund | seller | `Refund()` | `refund()` | `refundBySig(sig)` |
| settle | both | `Settle(uint16 buyerBps,uint256 expiry)` | `settle(...)` with an empty slot by the party consenting | `settle(buyerBps, expiry, sigBuyer, sigSeller)` by either party, a slot empty where that party consented by registry |

A signed form, and `settle` in every form, is sent by the current key of
`sellerKey` or of `buyerKey` (`NotParty` otherwise), so that no third party
chooses when a consent the parties gave takes effect, and pays the sender's
gas; each action is single-shot by the case's state and the messages carry
no nonce, so a signed action stays valid in every later state that admits
the action until the case ends or its signer's record moves to another key
(a `Refund()` signed while Funded refunds after a delivery; during a dispute
its signed form is refused, `WrongPhase`, so that a signature handed out
earlier cannot pick the refund over the lapse or the ruling, and the
seller's own call is refused at or before T3's end, §8), and a party signs
an action only when it means it submitted (`eravon.terms/1` §5.5). Two acts
that reach one instance are taken in the order they land, each decided from
the state the other left: a signed `Settle` or `Refund()` takes effect when
it lands, and any act that lands first and ends or moves the case takes the
place of it, whoever sends it (a party's own exit, the judge's `accept`,
`rule` or `resign`, a `redraw` under `autoAccept`, anyone's `advance` once
the window in force has ended); §4 names the opening's races and §6 and §8
the seating's and the dispute's. `sweep` is sent by the current key of
`sellerKey`, of `buyerKey`, or of a judge the case seated, refusing any
other caller (`NotParty`); `redraw` is open to any sender (§7), and where
the frozen record names no router its answer is the same whoever sends it;
`advance` and `commit` stay open to any address, as `galeed.venue/1` §2.11
and §2.8 require. The judge's three calls (`accept`, `rule`, `resign`) have
no signed form, and neither have the seller's `revoke`, a funder's
`acceptFunding`, and the allowance `sellerFund` grants on the token.

## 10. Own events

On an instance:

    Opened(bytes32 indexed caseId, address indexed seller, address indexed buyer, address judge, uint256 amount, bytes32 termsHash)
    Priced(address feeUnit, uint256 lf, uint256 capInToken, uint256 priceToken, uint256 priceUnit, uint64 rateAt)
    PanelFrozen(address indexed guild, address root, uint256 index, bytes32 panelHash, address router)
    DrawArmed(uint64 drawBlock, uint64 deadline)
    DeliveryClaimed(bytes32 manifest)
    Confirmed()
    Disputed()
    Settled(uint16 buyerBps)
    Ruled(uint64 rung, bytes32 entryId)
    Resigned(bytes32 entryId, uint8 form)
    Lapsed(uint16 buyerBps)
    Refunded(uint8 cause)
    Closed(uint8 terminal, uint256 buyerPaid, uint256 sellerPaid, uint256 judgePaid, uint256 bondToBuyer)
    Swept(address indexed who, address token, uint256 amount)

on the logic, and never on an instance:

    Consented(address indexed who, bytes32 indexed digest)
    Pulled(address indexed seller, address indexed fund, address token, address instance, uint256 amount)
    Revoked(address indexed seller, bytes32 indexed termsHash)
    FundBound(address indexed seller, address indexed fund)

on the board, `Posted` (§7b), and on the panel book, `Engaged` (§7). A
reader checks the emitter of the logic's four against the logic's address
and of `Posted` against the board's code hash, not against `proxyCodeHash`;
it reads an instance's events from the instance's address alone, the one its
order derives under §1 and whose code hash is `proxyCodeHash`, never by
`caseId` across emitters (§1).

The desk reads none of them. `Closed` reports the four sums §8 names.
`Swept(who, token, amount)` is emitted by every push that delivered
something, `who` being the creditor (a party's payout, the deposit's address
`sellerFund`, or the seat): the terminal pushes of §8, `withdraw`, `sweep`,
and the pushes of the expiry branch and of `cancel`. `init` emits, in this
order, `Priced` where `feeUnit` is nonzero, `PanelFrozen` on the guild
track, `Pulled` on the logic where P + `lf` is nonzero, `Seated` on the
named track or `DrawArmed` on the guild track, and `Opened`; a dispute after
the seat resigned emits `Disputed()` and then `Lapsed`. `status()` serves
the case's own state beyond the interface, sixteen values: the phase (its
ordinal in §5's list, `None` 0 through `Lapsed` 9, `Closed`'s terminal byte
being that ordinal less 4), the number of seats, P and D while the instance
holds them and the buyer's fee held (each zero once the terminal has
credited it), the sum unpaid (owed and claimable), `claimedAt`,
`disputedAt`, whether a draw is armed, its deadline, its block, the frozen
panel's index, hash and pool size, the guild's root and the panel's router;
`feeQuoted()`, `owed(payout, token)`, `stage()` (§4a),
`panelInitCodeHash()`, `panelBook()`, `LOGIC()`, `PROXY_CODEHASH()` are the
other views; `successionBook()` answers
`0x3dB7c505dd25361F1A66f6c5Bd96b48df27f668c` and `routeGas()` 1000000,
constants of the code; `partyRoles()`, `outcomeOf` and `advanceOf` are §8's;
`consented(who, digest)`, `revoked(seller, termsHash)` and `funds(seller,
fund)` are the logic's, read by the parties' clients and listed in no
judge's declaration (the views a judge declares under the preamble take no
parameters, and the desk never reads who assented or how, `galeed.venue/1`
§1 and §6); `hashOffer(offer)`, `check(...)`, `consents(...)`,
`verifyBuyer(...)`, `verifySeller(...)`, `panel(...)`, `draw(...)`,
`split(...)`, `salt(...)`, `initSalt(...)`, `cancelCheck(...)`,
`stageOf(...)` and `stageGate(...)` are on the verifier.

The faults, by door. The gate's are §4's, in the order §4 writes them
(`init`'s steps; on `cancel`, `AlreadyInitialized`, then `WrongAddress`,
`NotBuyer`, `BadTerms(4)` and `StageHeld`, the four read on the verifier in
one call, then the consent); on the stage, `BadPrev` and `StageOpen` at
`init`'s step 5 after the consents, and `StageHeld` on `cancel` after
`BadTerms(4)` and before the buyer's consent (§4a). On the parties' actions:
`WrongPhase` (the phase does not admit the act; `refund` while Disputed at
or before T3's end, and `refundBySig` throughout a dispute), `NotSeller` and
`NotBuyer` (the direct form from an address other than the party's current
key; `cancel` with a zero `buyerKey` too), `NotParty` (a signed form,
`settle`, `sweep`, the expiry branch, or `init` under `buyerMayOpen`, from
no party's current key, nor, for `sweep`, from a seated judge's),
`BadSellerSig`, `BadBuyerSig`, `NotOpen` (the last seat not seated and
accepted, or no seat), `WindowClosed`, `ZeroManifest`, `Underfunded` and
`TransferFailed` on a dispute whose fee pull falls short or fails, `BpsOver`
and `SettleExpired` on `settle`. On the judge's calls, decided in this
order: `UnknownSeat` (another `caseId`, or an ordinal never opened),
`NotDirect`, `NotOpen` (the seat's state admits no such act),
`WindowClosed`, `RungOutOfRange`, `ExtraNotEmpty`, `EntryMismatch`. On
`commit`, `NotOpen` (another `caseId`, or a case not Funded, Claimed or
Disputed). On the panel face `NoPanelFace`; on the draw, in this order,
`NotPendingRedraw`, `PanelMismatch`, `DrawPending`, `RouterGas`. On the
economic face, in this order, `NotInstance` (`setPayout` on the logic),
`NotCurrent`, `PayoutSet` and `ZeroPayout` on `setPayout`; `NotInstance`,
`NotCurrent`, `PayoutSet` (another payout named), `NoPayout` and
`TransferFailed` on `withdraw`; and `NotForeign` on `sweep`. On the logic's
four doors (`pull`, `revoke`, `consent`, `acceptFunding`) `NotRegistry` (an
instance), `NotInstance` (`pull` from a caller without `proxyCodeHash`),
`NotCurrent` (`revoke` or `consent` from a key that is not the named party's
current key), `TransferFailed`; on the gate `FundUnbound` (§4 step 5, after
the consents and the stage). `Reentered` on any reentry into a locked
function, `VerifierMissing` wherever the verifier is reached and its code is
absent, and the book's own (`NotCurrent`, `LengthMismatch`, `EmptyMembers`,
`ZeroWord`, `WordsDiffer`, `ExpiresNotInFuture`, `RouterNoCode`,
`NoSuchIndex`). `NotParty`, wherever an instance's call tests it (a signed
form, `settle`, `sweep`), comes before every other fault of that call but
`Reentered`; on `init` and `cancel` it stands where §4 puts it. The direct
and the signed form of one action test the actor first (`NotSeller`,
`NotBuyer`, or, after the sender, the signature) and the phase and the seat
after, so the same wrong state answers the same fault under either, except
that `claimDeliveryBySig` tests the manifest before the signature while the
direct `claimDelivery` tests it after the actor, the phase, the seat and the
window, `refundBySig` refuses a dispute past T3's end that the direct
`refund` admits (§8), `settle` tests the phase, `BpsOver` and
`SettleExpired` before the two consents, and `cancel` decides in §4's order.
An instance that holds no case knows neither party's key until the expiry
branch or `cancel` keeps both (§4): before that, `NotParty` and the actor
test fail for every caller and every signature, and `redraw` answers
`NotPendingRedraw`; after either, a party's direct acts and `settle` answer
`WrongPhase`, a signed form its signature's fault first, `redraw`
`NotPendingRedraw` still, and its `sweep` is served (§8); after the expiry
branch `init` runs the branch again and `cancel` closes the instance, and
after `cancel` both answer `AlreadyInitialized` (§4).

## 11. Self-proof of the eighteen preconditions

`galeed.venue/1` §7, item by item, with where each is measured
(eravon/standard/test/): 1 the proxy points at immutable logic and every
instance shares `proxyCodeHash` (replay step 1, `identity.sh`,
`test_verifier_atTheDerivedAddressWithThePinnedCode`); 2 one `Seated` per
seat (replay steps 4 and 14, `test_decline_thenRedraw`,
`test_expiredOffer_thenRedraw`, `test_exhaustion_threeSeats`); 3
`amanahHash` nonzero, from the terms or the frozen panel (`BadTerms` in
`test_guards_eachRefused`, `PanelWord` in `test_freeze_refusals`, replay
step 14); 4 `termsHash` the document's sha256, obtained as §2 says (guard 13
in `test_guards_eachRefused`, the criterion's values in `test_vectors`); 5
constant type strings, `quoteHash` pinned, `extra` refused (replay steps 1
and 7, `test_seatOf`, `test_quote_refusals`); 6 windows as §5, a disputed
seat's `windowEnd` nonzero (the clock tests: `test_clocks_claimLastSecond`,
`test_clocks_disputeLastSecond`, `test_clocks_advanceNothingDue`,
`test_released_T2`, `test_refunded_T1`, `test_refunded_offerClock`); 7 six
values in transactions only, `lapsed` from T3 alone (a resignation's default
and a dispute after a resignation end the case Lapsed with the seat
`resigned`: `test_lapsed_T3_lapseDefault0`, `test_lapsed_T3_lapseDefault1`,
`test_lapsed_T3_lapseDefault2`, `test_resigned_claimed_thenDispute`,
`test_resigned_disputed_form2`); 8 `accept` from the judge's current key
under the three conditions, and the consent branch (replay steps 4a and 13,
`test_acceptGuards`, `test_clocks_acceptOnTheManualTrack`); 9 `commit` open
to any address until the case ends (replay steps 4, 5 and 6); 10 the three
conditions and the entry test on `rule` and `resign` from the judge's
current key, a decline keeping `accepted` zero (replay steps 7, 9a and 13,
`test_ruled_guards`, `test_resign_guards`, `test_resigned_declined`); 11 the
rung range (`test_ruled_eachRung`, replay step 15); 12 `Anchored` from the
two paths only, once each, with the bytes, sent by the current key (replay
step 12); 13 `advance` (replay steps 8 to 10,
`test_clocks_advanceNothingDue`); 14 the economic face: `setPayout` and
`withdraw` from the key's current key, one registration, none at `init`, no
party money through it, and the tokens held covering everything claimable
once the case winds down (replay steps 2, 11, 13 and 15,
`test_setPayoutAndWithdraw`, `test_guildJudgeRegistersItsOwnPayout`,
`test_keys_theJudgesPayoutAndWithdraw`); 15 the book branch: the instance's
`engage` reverts, the book takes the current key's call alone, applies the
six rejections and files each record under the sender's root with its router
and the router's code hash, the case freezes the root and the latest panel
recorded before its block and unexpired at it, keeps it to its end, calls a
named router as §7 states and seats only a nonzero member no seat of the
case holds (replay steps 3 and 14, `test_book_rejectionsAndViews`,
`test_freeze_drawsFromTheLatestRecord`,
`test_freeze_expiredLatestGivesWayToAnOlder`,
`test_frozenPanelServesToTheEnd`, the `test_router_` tests of
Eravon.router.t.sol for every path that arms a draw,
`test_F24_theRouterIsHandedExactlyRouteGas` and
`test_F24_aRouterNamingTheZeroEntryEndsTheCase` in test/fuzz24/, the
`EravonR2RouterGas`, `EravonR2PanelCost`, `EravonR2RouterInputs` and
`EravonR2Verifier` tests of test/r2/ (the `RouterGas` threshold read at the
compiled comparison and in the trace, the panel's cost, the router's inputs,
the draw without a router against the pool rule over panels holding zero,
the parties' keys and repeats, the router's calldata, the freeze of the
latest earlier unexpired record), `invariant_68_theRouterDrawIsTheModel` and
`invariant_70_aFailedRouterEndsItsCase`); 16 a judge's and a guild's address
are read through the succession book `successionBook()` names, a party's key
through the same book at every party gate (§9), `seatOf`'s `parties` and
every payout as recorded, and `successionBook()` and `routeGas()` constants
of the code (replay steps 1 and 13; the party-gate tests, the `test_keys_`
tests of EravonReplay.t.sol: `parties` the recorded addresses after a move
in `test_keys_initTakesTheCurrentKeysConsentsAndCall`, every payout's
destination the same before and after a move in `test_keys_directForms`,
`test_keys_sweep` and `test_keys_theExpiryBranchAndCancel`, no book read but
`successionBook()`'s in `test_keys_noBookButTheOneTheCodeNames`); 17 this
document; 18 `partyRoles()` constant and well formed, `outcomeOf` and
`advanceOf` answering as §8 states and binding what `rule` and `advance` do,
and the two credited fields on the seat (replay step 15,
`invariant_66_lapseSharesAndSeatCredits`,
`invariant_67_outcomeViewsBindTheActs`, and
`testFuzz_R6_splitEveryRowOfTheTable` for the split itself). Beyond the
eighteen, the standard's own preconditions: the deposit is pulled at `init`
and nowhere else, to the calling instance and nowhere else
(`test_pull_onlyInstanceToItself`), and only from the order's `sellerFund`,
which is `sellerKey` or an address bound to it (`test_pull_onlyBoundFund`,
`invariant_pullFromBoundFundOnly`); `cancel` succeeds before `init` and
never after (`test_cancel_byTheBuyer` in Eravon.gate.t.sol,
`test_cancel_beforeInitOnly` in EravonReplay.t.sol,
`invariant_04_cancelOnlyBeforeInit`); a later stage opens only after the
stage at `prev` ended with `buyerBps` zero and cancels only after it ended
with `buyerBps` nonzero, a `prev` that is no instance (no code, another
code, the proxy's code over storage no order wrote), or not yet opened,
opening nothing and cancelled by nobody, stages cancel in order, the series
judge is handed on as §4a says, and the draw without a router seats it where
§7 says and the pool's pick otherwise (the `test_stage_` tests of
Eravon.stages.t.sol, the `EravonStageInvariants` and `EravonStageReach` of
test/stagefuzz/, the `test_R26_` tests of test/r26/); `revoke` closes the
gate for every later `init` and touches no open case
(`test_expiryBranch_revoked`, `test_revokeAfterInitTouchesNoOpenCase`);
every party gate admits the party's current key alone, and a key that handed
its record on acts and signs for nothing and registers nothing after the
move, what it registered before standing (§9) (the `test_keys_` tests, the
`EravonR2Gates` tests of test/r2/, `cancel`'s signed form testing the sender
before the signature among them, `invariant_64_aFormerKeyActsForNothing`);
the invariants of §8's table hold under every path (the invariant suite).

## 12. The replay seat

eravon/standard/test/EravonReplay.t.sol completes the frozen harness
(`galeed-conformance/venue-replay/src/VenueReplay.sol` with
`ReplayRouters.sol`, `GaleedSuccessionBook.sol` and `MockERC20.sol`, copied
under test/replay/, the `IGaleedVenue.sol` import of the first two rewritten
to the standard's src/ and nothing else changed) twice: the named track,
with `seatsBeforeAcceptance` true, `offerClockRuns` true, and a carried case
opened under the same judge's quote with `autoAccept` set; and the guild
track, with the venue drawing from the chain's panel book, `panelTerms()` =
`proxyCodeHash`, each case's first draw made by the seller's current key in
the block after `init`'s `DrawArmed` (§7), and the routers of the replay's
step 14 named in the panels it engages. The replay deploys
`GaleedSuccessionBook` at the constant address the code names and moves
every gated key on it (its step 13). Before any case opens, each order's
`sellerFund` approves the logic on the settlement token for its deposits,
the company run's wallet also binding itself to the seller by
`acceptFunding`. Beside the two tracks the seat runs the named track once
more for a company seller, whose `sellerFund` is an address other than
`sellerKey`, bound by `acceptFunding` before any case opens, the buyer's
cancellation (before `init` it refunds, after `init` it is
`AlreadyInitialized`), and the party gates under moved keys (§11 item 16).
The frozen harness observes a redraw in the transaction that took the seat
out of flight, while this standard draws in a later block by `redraw` (§7),
so the replay sees no redraw on those paths; the standard's own guild draw
tests measure every redraw it makes, its router call, and that call's gas
swept as the replay's step 14 sweeps a creation's (Eravon.guild.t.sol and
Eravon.router.t.sol, the sweeps in `test_router_gasSweepOverEveryPath` and
`test_router_gasSweepAroundTheFailingDraw`). The hooks it completes:
`deployVenue` (the first case's instance, prepared), `bookAt` (the constant
the code names), `venueOf` (the instance of each case), `venueDraws`,
`seatsBeforeAcceptance`, `offerClockRuns`, `openCase`, `dispute` (the
seller's `claimDelivery`, then the buyer's `dispute` with the fee pulled),
`party` (the buyer's current key), `quoteOf` (the case's quote bytes),
`roleAddresses` (§8's two roles: `buyerKey` and `buyerPayout`; `sellerKey`,
`sellerPayout` and, where it differs, `sellerFund`) and `collect` (a party's
`sweep` to the address); on the named track `openCarriedCase`; on the guild
track `panelTerms`, `panelBook` (the chain's book), `opensCaseNaming` and
`openCaseNaming`, its `openCase` reading the panel the case will freeze from
the replay's own record of the panels it engaged and naming that panel's
word in the order, and treating a case whose draw seats nobody as a creation
refused, so that the replay measures the freeze where it predicts a seat,
and the standard's own tests measure it where it predicts none: the
`test_freeze_` tests the refusals at `init`, the `test_router_` and
`test_F24_` tests a router that fails, names a seated member or names the
zero entry, and `test_exhaustion_threeSeats` an exhausted pool.
`reassignsAfterExpiredOffer` and `redrawsAfterLapse` keep their default,
false.

## 13. Succession

A change that reaches a byte of the compiled code, or any pinned setting, is
a new standard (`identity.sh` decides which a change is: a comment changes
no byte under the pinned compile and no value of CODEHASH.md, except the
NatSpec directive `@solidity memory-safe-assembly`, which the compiler reads
as code): new logic address, new `proxyCodeHash`, a new table in CODEHASH.md
(its twelve values: the logic's initcode hash and address, the proxy code
hash, the panel book's initcode hash, address and code hash, the verifier's
initcode hash, address and code hash, the board's initcode hash, address and
code hash), a new declaration by every judge, a new `engage` by every guild
under the new `proxyCodeHash` on the panel book the new table names (the
same book where the table's panel values are unchanged, a new one where they
are not), a new allowance by every seller's `sellerFund` and a new bond by
every funder that is not the seller's own key. Every deployable contract's
runtime code stays under EIP-170's 24576 bytes; `forge build --sizes` is the
measure (this generation's runtime sizes: the logic 24,308 bytes, the
verifier 12,906, the panel book 1,801, the board 241), and forge 1.8.3 is
the toolchain every measure is taken under. Old instances run out under the
old code. The seller's domain names the logic as `verifyingContract`, so its
value changes with every standard; the form of the two domains, the type
strings and the three document laws do not change with the contract unless
the fields they map change. Instances of this generation are read under
`eravon.terms/1`, `eravon.link/1` and `eravon.dossier/1`. A client of a new
generation carries the generation before it, that generation's logic
address, ABI and identity table, and acts on its instances by the `logic`
their documents name, old cases running out under the old code while new
cases open under the new; every clock of the old generation is bounded, so
the client drops it once the last of its cases has run out; a series begun
under a generation runs out under it, its later stages included (§4a). The
key is the same key: a party's record on the succession book belongs to no
generation.