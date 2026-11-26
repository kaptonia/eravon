# venue replay

`galeed.venue/1` §8 (docs/galeed-venue-v1.md) as an abstract Foundry test.
`src/IGaleedVenue.sol` is the interface verbatim; `src/VenueReplay.sol` runs
the replay's steps, reading the venue through the interface's members and
events, its code, and the accesses and calls its execution makes, beside the
state of the token and of the book, against any venue that completes the
hooks:

| hook | what the venue supplies |
|---|---|
| `deployVenue()` | the deployed venue; `token` (a plain ERC-20 with a free mint) and `succession` (the succession book of `galeed.roster/1` §4, `src/GaleedSuccessionBook.sol`, the core's contract byte for byte) exist already, and the venue names `succession` as its `successionBook()` |
| `bookAt()` | optional: where the venue's code names its book by a constant address, that address; the harness places the book's code there before `deployVenue()` runs and moves every key on it |
| `venueOf(caseId)` | the venue contract that holds a case: `venue` by default; for a standard that deploys one venue per case, the instance holding `caseId` (the replay's per-case calls, reads, and log filters all go through it; step 2 runs on `venue`, so a per-case standard returns from `deployVenue()` the instance of the first case `openCase()` will hand back, prepared or opened, since log recording is armed before `deployVenue()` runs) |
| `venueDraws()` | whether judges are drawn from an engaged panel; the harness engages `[JUDGE]` with `[AMANAH]` under `panelTerms()` as the guild's latest panel before any case |
| `panelBook()` | where the panel face lives (§4): `address(0)` when the venue holds it, else the panel book the venue's standard names; the harness then sends the guild's `engage` to the book and expects the venue's own to revert |
| `seatsBeforeAcceptance()` | whether the seating `openCase()` opens waits for `accept` (§2.7) or carries the judge's consent; the replay reads `accepted` on every seat it opens and follows the seat |
| `openCarriedCase()` | optional: a seating that carries consent at a venue whose ordinary path opens an offer, so both branches of step 4a are measured |
| `offerClockRuns()`, `reassignsAfterExpiredOffer()`, `redrawsAfterLapse()` | whether an open offer runs a clock, and whether the call that closes an expired offer, or the call that lapses a seat, opens a new seating, each a statement the replay measures: `offerClockRuns()` against the offer's `windowEnd`; for a venue that never draws, the other two against what follows at steps 8 and 9a; for a drawing venue, against what step 14 observes on each path |
| `openCase()` | the venue's own path from nothing to a `Seated` for `JUDGE`; where the venue draws and no usable panel exists, the path reverts, which step 14 reads as the refusal §4 requires |
| `opensCaseNaming()`, `openCaseNaming(guild)` | optional: the same path with the terms naming the guild by another address of its record, so that step 14 measures the root a case freezes |
| `dispute(caseId, seat)` | the venue's own path from an accepted seat to `disputed`, in the form the venue's documentation names for the dispute `outcomeOf` takes as made |
| `party(caseId)` | an address the venue counts as a party |
| `rulingExtra`, `feeToken` | optional: the `extra` a ruling carries (the `abi.encode` of the values `extraType()` names), the token fees accrue in |
| `roleAddresses(caseId, role)`, `collect(caseId, who)` | optional: the addresses that stand in each role of `partyRoles()` on the case, as the venue's documentation states them, and the venue's own path by which such an address takes what the venue holds for it; with them step 15 checks that each role receives at least its element of `shares`, counted as what the token records its addresses received (the ERC-20 transfers into them where the token keeps no such record), and without them it reads `shares` for shape only (`rulingFeeFrom` is read for shape only in every case) |
| `quoteOf(caseId)` | optional: the quote bytes the creation path of `openCase()` copied, where the venue decodes a quote; step 4 then checks that the seat freezes their `keccak256` |
| `ladderDetermined(caseId, seat)` | optional, `true` by default: whether the seating's ladder is determined; on a seating with `rungs` zero whose ladder waits for a claim, the replay reads no outcome until the dispute determines it |

The entry bytes a ruling and the resignations carry are the harness's own
(`N1`, `N3`, `N5`, `N6`, and `N7`, `N8`, `N9` from moved keys), their
`entryId`s being `sha256` of them (§2.9, §2.10); the replay also sends bytes
that do not hash to the id and expects the refusal. Step 1 checks that the
venue names the book the harness deployed, that the venue's code, placed at
another address, reads the same book and route gas, and, under state-diff
recording, that the two reads touch no storage, balance or code of another
account, then answer alike once balances, the clock and the block have
moved. Step 3 engages `[J]` last, so that whatever rule the venue draws by,
the first cases seat `J`. A stranger's commitment is sent on the offer,
through a contract, during a dispute and on every seat a redraw opens. A
seat the replay takes out of flight on state it keeps is checked to keep its
window, `accepted` and range, and the clocks of the first seats are tried at
their last second, where nothing is due, before they are run out. In step
13, and in step 14 before the guild's last `engage`, the replay moves the
record's key on the book, twice before the record's first gated call, before
every call §1 gates by the current key (`accept`, `rule`, `resign`,
`setPayout`, `withdraw`, and the guild's `engage` in step 14), and sends
each call from every key the record held before, from the current key
through a contract (but for `engage`, which a contract sends under its own
root), and, for the three direct calls, carrying code and from a contract's
constructor at the current key's address, each refused, before the current
key's own call is taken; the ruling it takes is measured as step 15 measures
every ruling; the accounts it measures are the judge's, its payout
registered at step 2, and two keys with no payout, one of which registers a
contract.

Step 14, for a venue that draws, keeps its own record of the panels it
engaged, each under a word of its own, and predicts, for every case and
every draw, the panel §4 freezes, the member it seats and the word the seat
carries: panels recorded in the creation block, under other terms, at and
past their `expires` second over an older unexpired one, after a case's
creation, and by a contract the guild's current key calls, which files under
the contract's own root; the routers of `src/ReplayRouters.sol` (a fixed
index, the first eligible member, one that reads the venue, chain, time,
members and number seated of the view, the same over a panel listing a
member twice, two whose index hashes the view's case, terms, token, amount,
rungs and parties, one that answers by the gas it receives within a few gas
of `routeGas()` at its own entry, one that answers only there and reverts at
once otherwise, one that reverts, spends all the gas it is given (reverting
at once when given more than `routeGas()`, so that the replay's own budget
never decides), returns two words or 31 bytes, writes state, names an index
outside `members`, or fails or names an index outside `members` from a given
ordinal on, one that names the member of the ordinal's parity, and one whose
code the replay replaces after the engagement or after a case's creation);
and a sweep of gas limits over a creation and over each redraw's call,
placed by the path's own cost, that finds the lowest limit at which the call
lands and tries every limit from 50 below it to 400 above, at each of which
the call reverts with every effect it had or the router answers under
`routeGas()`; under the router that reverts off `routeGas()` the sweep also
tries every limit from 50 below the least limit at which a redraw whose
router fails lands to 400 above, the band in which a venue that calls the
router short would finish a draw that finds no member. Redraws are found by
observation: on each path by which a call takes a seat out of flight (an
expired offer or a lapse under `advance`, the judge's decline, the judge's
resignation of a seated and of a disputed seat, an `advance` naming a seat
that left flight while a later seat's clock has run out) the replay sends
the call twice, once while the frozen panel is unexpired and the latest and
once after it has expired under a later panel of a member and a word of its
own, and, on each path whose call takes the seat it names out of flight,
again on the seat a redraw opened, and, where a `Seated` follows, runs the
whole set of redraw measures on that path: the frozen panel after it
expired, the routers, the gas sweep, the view's fields, and the root a case
froze. The view a venue hands a router at a redraw is read from the call
itself, out of the state diff, and every field but the seed is compared.
Every seat a redraw opens refuses a stranger's `accept` where it waits for
one, a stranger's `rule` and `resign`, and its judge's `rule` through a
contract and carrying code, and is accepted, disputed and ruled on snapshots
at its ruling window's last second at each of its first 16 rungs, each
ruling emitting one `Anchored`. Over the whole replay every `Seated` of a
drawn case carries the frozen panel's word and seats a member of it no
earlier seat holds.

Step 15 reads `outcomeOf` and `advanceOf` around every ruling and advance it
measures, and checks that `outcomeOf` for each rung of a ladder's range
where the replay reads the range, and `advanceOf` and `seatOf` around every
advance, answer alike from `J`, from the zero address and from a contract
and under another coinbase, randomness, base fee and gas price; rules each
of the first 64 rungs of the range on a snapshot; compares what `outcomeOf`
reported before acceptance with what it reports once the seat is disputed;
and checks the seat's two credited fields, the judge's claimable and,
through the two optional hooks, what each role receives against them, read
from what the token records each address received (`MockERC20.received`,
which a reverted frame leaves untouched; for a token without that record,
the ERC-20 transfers into the addresses). A role's receipt is measured as a
lower bound in the action itself; what a role receives above its share out
of money the venue holds for others, or out of none, is caught by the
venue's books (§3): after each rung of the first ruling seat's range, after
each ruling of a redrawn seat, and at the end of the replay, on the state
then found, every seat in flight, and every seat those endings open (up to
sixteen rounds), is ended by its judge's resignation (past its window, by
`advance`), every party collects where the suite names the roles' addresses
and their path (the two optional hooks), and each venue the replay met must
hold, in the fee token and the harness token, what every judge it seated
there can claim, so that a sum moved to a role out of another case's money,
or a credit out of nobody's, is a collection the venue cannot pay or a
claimable it does not hold. The replay also checks that no credited field
falls, that for every judge whose seatings are all in the fee token what its
payout received and what it can still claim are exactly the credited fields
of its seatings (a judge with a seating in another token is skipped whole),
that every `Seated` names the seat's judge once per pair, that a seat that
gave no ruling carries no ruling fee, that a lapsed seat was accepted, that
no party's money is claimable through the economic face, and that no judge
but `J` has a payout. Step 12 is checked over every `Anchored` before the
closing solvency sweep, which comes last.

forge 1.8.3 records the logs of a frame that reverts, which no chain keeps.
The replay therefore drains the logs it keeps before every call it expects
or allows to be refused and discards what that call recorded, and reads
receipts from the token's state. A log emitted in an inner frame that
reverts inside a call that stands cannot be told from a kept one, so a venue
that emits `Committed` or `Anchored` in a frame it then reverts is not
caught by the replay; a judge reads that from the venue's code.

Run inside this directory:

```
forge test -vv
```

A replay of a drawing venue spends about 7.3 billion gas in one test, so
`foundry.toml` raises `gas_limit` and `memory_limit`; a venue's own suite
that inherits the replay needs the same settings.

`test/MinimalVenue.sol`, `test/MinimalDrawingVenue.sol` and
`test/MinimalBookVenue.sol` (with `test/MinimalPanelBook.sol`) and their
replays are the harness's self-check, one reference venue per branch of §4
(a venue that never draws, one that holds its panels, one that draws from a
panel book), with three more for the branches those do not take:
`test/MinimalLapseVenue.sol` draws again after a lapse and not after an
expired offer, `test/MinimalExtraVenue.sol` has no ladder and carries the
ruling in `extra`, and `test/MinimalNarrowVenue.sol` narrows the range at
the dispute. Each is a fixture and no venue for use: they escrow the amount
and the fee, refuse a zero word at creation, carry the copied consent to a
redrawn seat, report `partyRoles()` as `buyer,seller` and every ruling and
advance by `outcomeOf` and `advanceOf`, hold what a party receives until it
claims it, count every credit on the seating, read the acting key of a seat
and of an account from the succession book, freeze the guild's root and the
latest of the panels recorded before the creation block that are unexpired
at it, call a panel's router by `staticcall` with exactly `routeGas()`, and
draw again from the frozen panel, before writing the seat's new state. The
suites read `forge-std` from the repository's `contracts/lib` (`libs` in
`foundry.toml`), so the tree builds inside the repository or inside a
package that carries that library at the same relative path. The harness is
tooling: it measures a venue against the text and binds nothing; a venue
that fails a step disagrees with the text, and the disagreement is settled
by reading the text.
