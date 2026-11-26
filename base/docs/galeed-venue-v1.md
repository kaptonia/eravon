# GALEED venue interface v1 (`galeed.venue/1`)

The construction standard for any contract that wants a GALEED judge. A
venue is a contract that seats a judge on a case, takes evidence
commitments, accepts the ruling, and pays the seat. A judge's desk reads
facts from a venue and never constructs them: who assented, who was drawn,
which covenant version was frozen, how long the judge has left, what a
ruling or an advance pays and what each seating earned, are all computed
inside the venue and served out through the members below. Anyone may build
a venue. A contract that implements this interface can be declared in a
judge's covenant and served by the desk; a contract that does not is
incompatible, and no second adapter exists.

**Status.** A frozen family document of the GALEED seat, read together with
[`galeed/1`](galeed-v1.md). It fixes an ABI and the meaning of each member;
it fixes nothing about how a venue reaches those meanings. Words in capitals
(MUST, MUST NOT, MAY) carry their usual force. The literal `galeed.venue/1`
is registered in the family literal registry and names this text; §10 names
the interface's identity, and from that naming the ABI, every MUST of this
text and every line of §7 bind, a change to any of them being
`galeed.venue/2` (until a venue deployed for use binds to the identity, §10
lets the naming be redone).

---

## 1. Shape

One interface, three faces, twenty ABI members; the same file declares
the succession book the venue reads (`galeed.roster/1` §4) and the router a
guild may name (§4).

```solidity
interface IGaleedVenue {
    // ---- case face (the judge) ----
    event Seated(address indexed judge, bytes32 indexed caseId, uint64 seat);
    event Committed(bytes32 indexed caseId, address indexed by, bytes32 manifest);
    event Anchored(address indexed sender, bytes32 indexed hash, bytes entry);

    struct Seat {
        address   judge;
        bytes32   amanahHash;
        bytes32   termsHash;
        bytes32   quoteHash;
        uint64    windowStart;
        uint64    windowEnd;
        uint64    accepted;       // unix second the judge accepted the seat; 0 while the offer stands open
        uint8     state;          // 0 seated, 1 disputed, 2 ruled, 3 resigned, 4 lapsed, 5 closed
        uint64    rungs;
        uint64    rungMin;        // the rungs accepted from this seat now: rungMin..rungMax inclusive
        uint64    rungMax;
        address   token;
        uint256   amount;
        uint256   standbyCredited; // credited to the judge on this seating so far, in token, as standby fee
        uint256   rulingCredited;  // credited to the judge on this seating so far, in token, as ruling fee
        address[] parties;
    }

    struct Outcome {
        uint8     state;          // the seat's state once the action is taken
        uint256[] shares;         // per role of partyRoles(): the part of amount the role receives
        uint256[] rulingFeeFrom;  // per role: the part of the ruling fee the role bears
        uint256   standbyFee;     // what the action credits to the judge as standby fee
        uint256   rulingFee;      // what the action credits to the judge as ruling fee
    }

    function seatOf(bytes32 caseId, uint64 seat) external view returns (Seat memory);
    function quoteType() external view returns (string memory);
    function extraType() external view returns (string memory);
    function partyRoles() external view returns (string memory);
    function outcomeOf(bytes32 caseId, uint64 seat, uint64 rung, bytes calldata extra) external view returns (Outcome memory);
    function advanceOf(bytes32 caseId, uint64 seat) external view returns (Outcome memory);
    function accept(bytes32 caseId, uint64 seat) external;
    function commit(bytes32 caseId, bytes32 manifest) external;
    function rule(bytes32 caseId, uint64 seat, bytes32 entryId, uint64 rung, bytes calldata entry, bytes calldata extra) external;
    function resign(bytes32 caseId, uint64 seat, bytes32 entryId, bytes calldata entry) external;
    function advance(bytes32 caseId, uint64 seat) external;
    function successionBook() external view returns (address);

    // ---- economic face (any seat: judge, verifier) ----
    function setPayout(address key, address payout) external;
    function withdraw(address token, address key, address payout) external;
    function accountOf(address who, address token) external view returns (address payout, uint256 claimable);

    // ---- panel face (the guild) ----
    function engage(bytes32 termsHash, address[] calldata members, bytes32[] calldata amanahHashes, bytes32 rosterHash, uint64 expires, address router) external;
    function routeGas() external view returns (uint64);
}

interface IGaleedSuccessionBook {
    function current(address a) external view returns (address);
    function root(address a) external view returns (address);
}

struct RouteView {
    bytes32   caseId;
    uint64    seat;
    bytes32   termsHash;
    address   token;
    uint256   amount;
    uint64    rungs;
    address[] parties;
    address[] members;
    address[] seated;
    bytes32   seed;
    uint64    blockTime;
    uint256   chainId;
    address   venue;
}

interface IGaleedRouter {
    function route(RouteView calldata v) external view returns (uint256 index);
}
```

**Identification.** There is no runtime interface detection. A judge
declares a venue as `{chainId, contract, codeHash}` in the covenant
(`galeed/1` §6.2), and the desk recognises a deployment by that
`codeHash` alone. It follows that the code at a declared address MUST be
immutable: no upgradeable proxy, no delegate to mutable logic, no
`selfdestruct`, no metamorphic redeployment. A new version of a venue is a
new deployment at a new address with a new `codeHash`, declared anew.
Cases in flight on the old deployment finish there.

**Keys.** Everything about a seating is keyed by `(caseId, seat)`, on the
read side and on the write side alike. A venue that never seats more than
once per case passes `0` everywhere.

**Frozen addresses.** Every address a venue records (a seated judge, a guild
that engaged a panel, a party, a payout target) is frozen for the life of
the record that holds it. Which key acts for a recorded judge's or guild's
address is read, when the call is made, from the succession book
`successionBook()` names (`galeed.roster/1` §4), the book the family
declares for the deployment's chain (`galeed.roster/1` §9), named by a view
constant for the life of the deployment and fixed by its code, a constant or
an immutable, so that the code hash a judge declares pins it: the address
recorded stays the one seated or engaged, and the key that may act for it is
that address's current key, so that a judge or a guild that moved its key
goes on acting in every record in flight with the new key and the key it
left can act in none. A party's address and a payout target are read as
recorded.

**Vocabulary.** The venue MAY expose any number of other events, views and
functions. The desk reads none of them on its own account; a view a judge's
own covenant names for the desk to call and show generically is the judge's
data and adds nothing to the desk's vocabulary. Nothing in this interface
asks the venue to change how it settles, escrows, prices, draws, or times
its own business; it asks the venue to report the results in the fixed
shape.

---

## 2. Case face

### 2.1 Case identity

`caseId` is a `bytes32` the venue derives however it likes (a sequential
id left-padded, a hash of the terms and the parties, anything). Within one
deployment it MUST be unique and MUST never change. It is the `id` of the
ruling's `subjectRef` of kind `evm` (`galeed/1` §6.3), so the venue's
documentation MUST state the derivation. The judgment's `subjectRef` also
carries the seat ordinal as an extra member `seat` (an `int`), which
`galeed/1` §6.9 admits and no predicate of that law reads, so that two
rulings on one case name two seatings.

### 2.2 Seating

`Seated(judge, caseId, seat)` is emitted at the moment a venue opens a
seating for a judge, whether the judge's consent is already in hand or still
to come (§2.7 says which): when a case is created under terms both parties
signed that name the judge, when a draw from a panel a guild engaged (§4),
on the venue or on the panel book its standard names, picks the judge, when
a redraw after a lapse or after an expired offer picks a judge again, when a
claim on a standing relationship opens a new seating for the judge who
stands by on it, when a venue assigns a judge the parties have agreed on and
now waits for the judge's word. `seat` is the ordinal of that seating on
that case: the first seating is `0`, each later seating on the same case
takes the next integer, and a case that seats several judges at once (an
appellate bench, say) gives each its own ordinal. Both parameters are
indexed so that a desk reads its own seatings with one bounded `getLogs`
over the contract address and the judge topic.

The venue MUST emit `Seated` exactly once per `(caseId, seat)`, MUST NOT
emit it for a judge who was neither named in terms the parties signed nor
drawn from a panel a guild engaged, and MUST NOT seat the same `(caseId,
seat)` twice. What counts as a party's signature, whether a company
container's executed proposal is a signature, how the draw is made where the
panel names no router (§4) and how fair it is, the member it seats being one
§4 admits, are the venue's affair; `Seated` is the venue's conclusion and
the desk does not review it.

### 2.3 The seat view

`seatOf(caseId, seat)` returns the whole state of one seating, computed by
the venue. It MUST be a pure view, consistent with every event already
emitted, and callable for every `(caseId, seat)` that was ever `Seated`.
For an unknown pair it MUST revert.

| field | contract |
|---|---|
| `judge` | the seated address; equal to the `judge` topic of the matching `Seated` |
| `amanahHash` | the covenant word the venue froze for this seating (§2.4); MUST NOT be zero |
| `termsHash` | sha256 of the document that governs the seat (§5); the document MUST determine this seat's ladder uniquely, so a venue whose terms carry several claim classes reports the digest of the class's own document; a seating that stands on a relationship before any claim names a class reports the relationship's own document with `rungs` zero, and reports the class's document from the moment a claim narrows it |
| `quoteHash` | keccak256 of the judge's quote bytes the venue decoded for this seat (§2.5); zero when the venue decoded no quote for it |
| `windowStart`, `windowEnd` | unix seconds of the clock whose expiry moves this seat (§2.6); both zero when the seat is in flight and no clock runs on it; a terminal seat keeps the window that was in force at the transition |
| `accepted` | the unix second at which the venue counted the judge's acceptance of this seat (§2.7); zero while the offer stands open; nonzero from the `Seated` block on where the seating itself carried the judge's consent; a terminal seat keeps the value it had |
| `state` | one of six values (§2.6) |
| `rungs` | the length of the outcome ladder; zero when the venue has no ladder |
| `rungMin`, `rungMax` | the range of rungs accepted from this seat while it is in flight, both ends inclusive (§2.5); a ladder venue reports the range a claim leaves open once a claim narrows it (for a ladder ordered from the respondent's side, `0` through the claimant's demanded rung) and `0` through `rungs − 1` before that; a venue whose outcomes form a set and no order (a two-outcome appeal, one outcome for each side, is such a set) reports `0` through `rungs − 1`; both zero when `rungs` is zero, and a terminal seat keeps the range it had |
| `token`, `amount` | the settlement token, an ERC-20 (§5), and the amount this seat's ruling could move: the escrowed or bonded amount before a claim narrows it, the claimed amount after, and the whole amount throughout where every rung is a split of the whole and a claim narrows the range and never the sum |
| `standbyCredited`, `rulingCredited` | what the venue has credited to the judge's account (§3), in `token`, on account of this seating: as ruling fee a credit that only a ruling the seat gave makes, as standby fee every other credit of the seating; zero until the venue credits; each rises in the transaction that credits and never decreases; every amount the venue credits to the seated judge's account in `token` on account of the seating is counted in its two fields, and a credit in any other token in neither |
| `parties` | the addresses the venue can act on or pay in this case, in the venue's own order; a party that has no address does not appear, and no reader takes the array as a full account of who stands behind the case |

The venue does all clock arithmetic itself. Suspension periods, filing
clocks, confirmation windows, grace periods, how they nest and which one
runs now, are folded by the venue into the single window it reports, and
`state` says what the window means: in `seated` it is the clock whose expiry
will move the seat (an open offer's acceptance deadline, a standing term's
end, a counterparty's confirmation window ending), in `disputed` it is the
ruling window. One guarantee attached to it: while `state` is `disputed` and
`block.timestamp` lies in `[windowStart, windowEnd]`, a `rule` or `resign`
from the seated judge's current key (§2.9) MUST NOT be refused on grounds of
time; §2.7 and §2.10 state those of an open offer's `accept` and of
`resign`. `windowEnd` is the last second at which the action is accepted,
never the deadline instant.

### 2.4 The frozen covenant

`amanahHash` is the 32-byte word that identifies the covenant version the
seat runs under: for a seat the parties named, the `entry_id` of the
covenant entry the judge's document offers (`galeed.roster/1` §3a.3, the
`amanah.entry_id` member), which is the word a ruling names in its own
`amanah` member (`galeed/1` §6.3); for a seat drawn from a guild's pool, the
`sha256` of the guild's covenant text (`galeed.roster/1` §3b.3), which every
member of that pool carries as an `amanah` entry of its own ledger. The
registry keeps no state, so a venue cannot read the word from it: the venue
receives it, stores it, and serves it. A venue that names judges in terms
takes the word as an argument at case creation, copied by the parties from
the judge's document; a venue that draws takes it from the panel the guild
engaged (§4), one word per member. Where the venue obtains it is the venue's
business; that every seat carries a nonzero word is the venue's obligation.
The desk resolves the word against the judge's own ledger, for a named seat
against the `entry_id` of each of the judge's `amanah` entries and for a
drawn seat against the `sha256` of the text of the `amanah` entry the judge
carries as the guild's copy (`galeed.roster/1` §7), and refuses to treat a
seat as ready for ruling when it matches none.

### 2.5 The quote and the ladder

`quoteType()` returns an ABI tuple type string naming the fields of the
judge's quote the venue reads, for example `(uint16 premiumBps,uint256
litigationFee,uint32 rulingWindow)`, and the empty string when the venue
reads no judge quote (a venue whose fees and windows come from network terms
or a bureau standard). It MUST be constant for the life of the deployment.
The quote bytes are `abi.encode` of the values in the order the string names
them, with no enclosing tuple; the field names and the spacing of the string
serve the form the desk renders and never enter the encoding. The judge's
desk renders the form, encodes, publishes the bytes in the covenant
(`galeed/1` §6.2's `quotes`, read by `galeed.roster/1` §5.10), and keeps
`keccak256` of the bytes per venue and version. A seat for which the venue
decoded a quote MUST freeze `keccak256(quote bytes)` into `quoteHash`; a
seat for which it decoded none (a drawn seat whose guild's covenant quotes
nothing for this venue: a drawn seat's quote is read from the guild's
covenant, `galeed.roster/1` §7, and never from the drawn judge's own)
reports zero, and the desk then resolves the version by `amanahHash` alone.
How the venue decodes the bytes into fees and windows is the venue's rule. A
quote field may carry the judge's standing consent to be seated (§2.7): a
venue that names such a field seats with consent carried when the copied
value says so and opens an offer otherwise, its documentation says which
field it is, and the pinned `quoteHash` is what lets the judge's desk check
that the parties copied the value the judge published, or, for a drawn seat,
the value the guild's covenant quotes (`galeed.roster/1` §7).

The ladder is the finite list of outcomes the governing document states in
prose, indexed from zero; its length is unbounded by this law. A venue whose
outcomes form such a list reports its length in `rungs` and, in `rungMin`
and `rungMax`, which rungs it accepts from the seat now: a claimant's demand
narrows the range, a set of outcomes with no order leaves it whole. A venue
whose ruling is a continuous quantity that no index names reports `rungs ==
0` and carries the ruling in `extra` (§2.9); so does a seating on a
relationship before any claim names a class (§2.3), whose ladder is not yet
determined. The desk refuses to sign a ruling whose rung is outside the
range; the ladder's text is read by people, and what each rung pays is read
from `outcomeOf` (§2.12).

### 2.6 States

| value | name | meaning |
|---|---|---|
| 0 | `seated` | the seat is open for the judge, no ruling is due yet; `accepted` says whether the judge has taken it (§2.7) |
| 1 | `disputed` | a ruling is due; the window in the view is the ruling window |
| 2 | `ruled` | the venue accepted a `rule` for this seat |
| 3 | `resigned` | the venue accepted a `resign` for this seat |
| 4 | `lapsed` | the ruling window passed with no ruling from this seat |
| 5 | `closed` | the seat ended in any other way: settled, withdrawn, executed by default before any ruling was due, superseded by another seat's ruling, an offer that expired unaccepted |

The first two values are in flight; the desk keeps a seat on its watch table
while the state is one of them and removes it otherwise. `lapsed` is the one
terminal value that the judge's own inaction produces, and a venue MUST NOT
report it for any other ending. A seat in `disputed` MUST run the ruling
window: its `windowEnd` is nonzero. A seat's state changes only in a
transaction: a seat past `windowEnd` on which nobody has called `advance`
still reads `disputed`, so that every change of state is a fact with a block
number. Each seat has its own state: a lapse followed by a redraw leaves
seat `n` at `lapsed` and opens seat `n+1` in flight, at whichever of the two
in-flight values the venue's rules give it. A venue need not use every
value; a venue where a dispute exists before any judge is seated and seats
with the judge's consent carried (§2.7) never reports `seated`. The venue
maps its own internal machine onto these six values; the mapping is
documented by the venue and never read by the desk.

### 2.7 Acceptance

A venue may seat a judge before it has the judge's word: parties agree on a
judge, the venue assigns the seat, and what follows (funding an escrow,
starting the case's clocks) waits on the judge. `accept(caseId, seat)` is
that word. The venue MUST accept it only under the three conditions of §2.9
(`msg.sender` is the current key of the seat's judge, equals `tx.origin`,
carries no code) and only while the seat is `seated` with `accepted == 0`;
every other call reverts. On acceptance the venue MUST set `accepted` to
`block.timestamp`, MUST leave the seat in flight (at `seated`, or at
`disputed` where its rules make a ruling due at once), MUST NOT emit
`Anchored`, and does whatever its own rules attach to the moment (moving the
parties' money into escrow, opening the ruling window, notifying its own
clients); none of that is read by the desk.

A seating that already carries the judge's consent (a judge named in terms
the judge negotiated, terms that copy a standing consent the judge published
in its covenant as a field of its quote bytes and the seat's `quoteHash`
pins, a member drawn from a panel the guild engaged under §4 where the
venue's rules take as the member's word the engagement or a consent field of
the guild's covenant quote that the terms copied (§2.5), a seating reopened
on a relationship the judge already stands by on) reports `accepted` nonzero
from the `Seated` block on, and `accept` on such a seat reverts like any
second acceptance. Consent is a property of the seating and never of the
deployment: one venue may open a first seating as an offer and every later
seating on the same case with consent carried, and its documentation says
which seatings are which. `accepted` records the second at which the venue
counted the consent, whether the judge's own key wrote it through `accept`
or the form of the seating carried it, and the field says nothing about
which of the two it was. The venue MUST NOT report `disputed` on a seat
whose `accepted` is zero: a ruling is never due from a judge who has not
taken the seat.

While `accepted` is zero the seat's window, when the venue runs one, is the
offer's deadline: `windowEnd` is the last second at which `accept` is taken,
and while the offer stands open an `accept` that meets the three conditions
MUST NOT be refused on grounds of time at any moment up to `windowEnd` (at
every moment where the venue runs no window) and MUST be refused past a
nonzero `windowEnd`; once it has passed `advance` (§2.11) moves the seat to
`closed`, never to `lapsed`, since no ruling was due; whether a redraw or a
new assignment follows is the venue's rule and shows up as a later `Seated`.
Declining an open offer is `resign` (§2.10) on the unaccepted seat, which
leaves `accepted` at zero beside `resigned`, so that a decline and a
resignation after acceptance stay distinguishable on chain. The desk shows
an open offer as awaiting the judge's word with `accept` and `resign` side
by side, and shows nothing of the kind once `accepted` is nonzero.

### 2.8 Commitments

`commit(caseId, manifest)` records a commitment to a courtevidence/1
manifest digest and emits `Committed(caseId, msg.sender, manifest)`. The
venue MUST accept a commitment from any address at every moment, from the
case's existence on, at which some seat of it is in flight or, until the
case ends, no seat of it has yet existed, MUST NOT gate it on identity, and
MAY refuse it at any other moment. Every commitment the venue counts, by
whatever function it arrived (a specification pinned at creation, a delivery
proof, a supplement, an expert report, a verifier's report committed by the
party that requested it, a record the venue folds into the case from before
the case existed), MUST emit `Committed` in the same shape; a record from
before the case existed is emitted at the moment the venue folds it in,
under that case's `caseId`, with the address that originally sent it as
`by`, and a record the venue never folds into any case is emitted for none.
The case exists from the moment the venue records its `caseId`, which its
documentation states; a redraw that opens a seat after every earlier seat
has left flight opens the window again for as long as that seat is in
flight; a case that ends with no seat ever opened closes the window at its
own end. `by` is the address that sent the commitment and claims nothing
about whose evidence it is; a relayer, a container, a bureau may stand
there. The block of a `Committed` is an upper bound on when the commitment
was made, never its moment. A venue MAY emit more than one `Committed` for
one digest, one per address that stands behind it. The desk's committed set
for a case is the set of digests in the `Committed` events for its `caseId`,
whatever their block; a judgment may cite only digests in that set.

### 2.9 Ruling

`rule(caseId, seat, entryId, rung, entry, extra)` accepts the ruling for one
seating. The venue MUST accept it only when three conditions hold together:
`msg.sender ==
IGaleedSuccessionBook(successionBook()).current(seatOf(caseId,
seat).judge)`, `msg.sender == tx.origin`, and `msg.sender` carries no code
at the time of the call. The three together are the executable form of "a
direct call from the seated key's current key": the first names the seat and
reads its current key from the succession book (§1, Frozen addresses), the
second excludes every inner call, relayer, batcher and entry point, the
third excludes a key that carries an EIP-7702 designator or any other code,
which `galeed/1` §9.3 would void. Neither boundary `galeed/1` §9.3 reads is
visible from inside a call, the state at the end of the previous block, and
the state at the end of the ruling's own block, which a later transaction of
that block can change; both remain the judge's own key discipline
(`galeed/1` §5.7). A venue that accepts a ruling through ERC-1271, an
account abstraction entry point, a relayer, or any inner call emits an
`Anchored` log whose `sender` topic (`galeed/1` §9.1's topic 1) names the
wrong address, which no reader counts.

`entryId` is the `entry_id` of the judgment entry in the judge's ledger,
and `entry` is that entry's bytes, byte for byte as the ledger holds them
(`galeed/1` §4): the venue MUST reject the call unless `sha256(entry) ==
entryId`, and it reads nothing inside the bytes, which is the desk's and
every reader's affair under that law, as this text's §5 records.
`rung` is the chosen rung, zero-based; where `rungs > 0` the venue MUST
reject a `rung` outside `rungMin..rungMax`; where `rungs == 0` the venue
ignores it. `extra` carries the rest of the ruling (a split in basis
points, an amount, a case file digest, a boolean) in the shape
`extraType()` names: an ABI tuple type string in the form of
`quoteType()` (§2.5), constant for the life of the deployment, empty
when the venue takes no `extra`. Where `extraType()` is empty the venue
MUST reject a non-empty `extra`; where it is not, `extra` is
`abi.encode` of the values in the order the string names them, with no
enclosing tuple, and the venue MUST reject an `extra` that is not that
encoding, the empty one included. The string names static types only,
so that one tuple of values has exactly one encoding and the venue tests
the length as well as the decoding; the shape is one for the deployment
and every rung carries it, a rung whose outcome fixes the value carrying
the value pinned. The string bounds the shape and nothing more: the
venue MAY refuse a value its own rules do not admit (a share above the
whole, a zero digest where its terms require one), as it refuses a rung
outside the range, and a refused ruling is a refused transaction the
desk reports as such. The desk renders the form from the string as it
renders the quote form, and echoes `rung` into the judgment body's
`decision` member (`galeed/1` §6.3) and, when `extra` is non-empty,
`keccak256(extra)` beside it. What a venue cannot name in a type string
it cannot ask of the desk.

On acceptance the venue MUST emit `Anchored(address indexed sender, bytes32
indexed hash, bytes entry)` with `sender == msg.sender`, `hash == entryId`,
and `entry` the bytes the call carried, three topics and the ABI encoding of
those bytes as data (the form `galeed/1` §9.1 reads: an offset word of 32, a
length word, the bytes, zero padding to a multiple of 32), exactly once.
That log is the form of `galeed/1` §9.1: the ruling transaction anchors the
judgment and publishes its bytes in the same block that executes it, a
reader whose basis admits the venue among its registries, by address or by
code, and lists the judge's key in `senders` counts it where that law's §9.3
does, and that law's §8.1 reads the bytes as a ledger entry, so that a
judgment a venue acted on, whose bytes pass that law's §4.3 under a key of
the lineage, is in the ledger under every such basis where that law's §9.3
counts its record and §8.1 of that law leaves it undiscarded; a key the book
names that the ledger has retired or that its lineage does not hold is
accepted here and anchors nothing there. The venue MUST NOT emit `Anchored`
from any other path and MUST NOT emit it with any word other than the
`entryId` argument of `rule` or `resign`, nor with any data other than the
`entry` argument of that call, since a reader admitting the venue charges
the seat with every word the venue echoes. State moves to `ruled`; what the
ruling does to money is the venue's, and `outcomeOf` reports it (§2.12).

### 2.10 Resignation

`resign(caseId, seat, entryId, entry)` is the one exit for a seated judge:
declining a draw, closing before a dispute, stepping down during one. The
venue MUST accept one that meets the same three conditions as `rule` and
whose `entry` hashes to `entryId`, on a seat in flight, accepted or not, at
every moment up to the seat's `windowEnd` (at every moment while the window
is zero), a revert §4 requires of a draw the resignation leads to apart,
MUST refuse one that fails any of the three conditions or whose `entry` does
not hash to `entryId`, and every call on a seat out of flight, and takes or
refuses one that meets them past a nonzero `windowEnd`, before `advance`
moves the seat, by its own rules; it emits `Anchored(msg.sender, entryId,
entry)` in the same shape, where `entryId` is the `entry_id` of the
resignation entry and `entry` its bytes, `sha256(entry) == entryId` tested
as `rule` tests it. State moves to `resigned` and the seat keeps the window
and the `accepted` value that were in force, so that a declined offer, a
resignation before a dispute and one during it stay distinguishable on
chain. Whether fees are forfeited, whether the case falls to a default
outcome, whether a redraw follows, are the venue's rules and show up in the
states of this and later seats.

### 2.11 Mechanical advance

`advance(caseId, seat)` is callable by anyone. When the named seat is in
flight and `block.timestamp > windowEnd` with a nonzero window, the venue
MUST move that seat out of the in-flight states by its own rules (to
`lapsed` when a ruling was due and none came, to `closed` otherwise, an
offer that expired unaccepted included, with or without opening a later
seat). When nothing is due on the named seat by its own clock, a pair that
was never seated included, the call MUST NOT revert and MUST NOT change that
seat's state; what it does to the venue's other clocks in the same call is
the venue's, within §2.12, a revert §4 requires of a draw included. A redraw
that follows opens another seat and does not contradict the rule that the
named seat left the in-flight states. The desk offers the call for any seat
past its window, shows what `advanceOf` reports for it (§2.12), and reads
the state before and after; gas belongs to whoever clicks.

### 2.12 Outcomes

What a venue's rules do to the money of a seat, the venue reports in one
shape, so that a judge's desk shows the consequence of a ruling or an
advance before it is sent and the earnings of each seating after, and
computes none of it (§6).

`partyRoles()` names the roles whose money the venue reports, in order:
distinct words of lowercase ASCII letters and digits, each beginning with a
letter, joined by single commas, for example `buyer,seller`, and the empty
string where the venue reports money for no role. It MUST be constant for
the life of the deployment. The words serve the labels the desk renders, as
the field names of `quoteType()` do (§2.5), and enter no decision; the
venue's documentation states which address stands in each role. Each array
of an `Outcome` has one element per role, in that order, and an element
counts together every address that stands in its role on the case.

An `Outcome` describes one action on one seat. `state` is the seat's state
once the action is taken (§2.6). `shares` is the part of the seat's `amount`
(§2.3) that each role receives by the action, the elements together no more
than `amount`; where every rung is a split of the whole among the roles, the
shares of a ruling sum to `amount`. `standbyFee` and `rulingFee` are what
the action credits to the judge's account (§3) on account of the seat, split
as `seatOf`'s two credited fields split them (§2.3). `rulingFeeFrom` is the
part of `rulingFee` each role bears, the elements together no more than
`rulingFee` and the rest borne by no role. Every amount is in the seat's
`token`; what the action moves besides, each sum going back to the address
that put it in or to the party whose sum the venue held (a deposit returned,
a fee refunded to the party that paid it, an inflow the venue holds for a
party), and what an `advance` does on the venue's other clocks to other
seats, and to this one where the call finds it out of the in-flight states
(§2.11), is outside the `Outcome`; every other sum the action moves to a
role counts in that role's element of `shares`.

`outcomeOf(caseId, seat, rung, extra)` reports a ruling with that rung and
that `extra` as `rule` (§2.9) takes it: on a seat in flight, the ruling
accepted on the state the call finds, the acceptance and the dispute a
ruling presupposes taken as made, in the form the venue's documentation
names, with the seat's `amount` and range as `seatOf` reports them, and what
the acceptance or the dispute would itself credit or move left out; on a
seat in `ruled`, called with the rung and the `extra` of the ruling the seat
took, that ruling as it was accepted; its `state` is `ruled`. It MUST answer
on every seat in flight, a seating whose ladder is not yet determined (§2.5)
excepted, for every rung inside `rungMin..rungMax` (any rung where `rungs`
is zero) with every `extra` the venue would accept, and on every seat in
`ruled` for the rung and the `extra` it was ruled with; it MUST revert where
`rule` would reject the rung for its range or the `extra` for its shape
(§2.9), and MAY revert for an `extra` the venue refuses for its value, on a
seat whose ladder is not yet determined, on a seat in `ruled` for any other
rung or `extra`, and on a seat that left the in-flight states without a
ruling.

`advanceOf(caseId, seat)` reports what `advance(caseId, seat)` (§2.11) does
by the named seat's own clock, the window `seatOf` reports (§2.3), on the
state the call finds: the named seat's state after it, the shares of the
seat's `amount` it assigns, and what it credits on account of the named
seat; where nothing is due on the seat, `state` is the seat's present state
and every amount is zero. On a seat in flight an `advance` credits and
assigns on account of that seat only what its own clock makes due; on a seat
the call finds out of the in-flight states, what the call does on the
venue's other clocks, to this seat's `amount` and to the judge's account
included, is outside the `Outcome` and the binding below, `seatOf`'s two
credited fields counting every credit of the seating however it came. It
MUST answer for every `(caseId, seat)` ever `Seated` and, as `seatOf` does,
revert for a pair never seated.

The reports bind the venue: a `rule` or an `advance` it accepts MUST move
the seat to the `state`, credit the judge the `standbyFee` and the
`rulingFee`, assign each role its element of `shares`, transferred to it or
held for it, and leave each role bearing its element of `rulingFeeFrom`,
that `outcomeOf` or `advanceOf`, called with the same arguments, reports on
the state the accepted call finds. `outcomeOf` and `advanceOf` MUST answer
by their arguments, the chain's state and the block's timestamp and number
alone, whoever calls them. The desk calls both views to show what an action
does, and never to decide a fact that `seatOf`, an event, or a ledger
states.

---

## 3. Economic face

The economic face is the seat's ledger and nothing else. Whatever a seat
earns (a judge's standby and ruling fees, a verifier's engagement fee)
accrues to the seat's own identity, keyed by `(who, token)` and never by
case, and leaves the venue only through `withdraw`; what a seating adds to
its judge's ledger in its `token` is reported on the seating itself (§2.3);
on a standard that deploys one venue per case the ledger is one per
deployment, and a seat registers its payout on each deployment it earns in.
Money the venue owes to anyone who is not a seat, an escrow refund, a
ruling's payout to a party, MUST NOT pass through this face: a judge whose
payout address is also a party's wallet, or a container whose door splits
every inflow as revenue, would otherwise see a party's money in `accountOf`
and lose it in `withdraw`.

The venue pays out of what it holds. After every action it accepts, the
tokens it holds MUST cover, in each token, everything claimable from it
through this face once it has wound down: every seat then in flight, and
every seat the wind-down opens, ended by its judge's resignation or, past
its window, under `advance`, every case with no seat in flight ended as its
documentation states (§2.8, §7.17), and every party having taken what the
venue holds for it. A credit, or a sum moved to a role, that only another
case's money or no money covers breaks this rule; one the venue covers from
tokens of its own or from a sponsor's transfer keeps it.

`setPayout(key, payout)` registers, once and irrevocably, the address that
withdrawals for `key` go to. The venue MUST accept it only from `key`'s
current key in the succession book, MUST reject a second registration for
the same `key`, MUST accept any nonzero address whether it carries code or
not (zero is the unset reading of `accountOf`), and MUST NOT require the
payout to be the sender. The venue MUST NOT register a payout for any key by
any path but this call and `withdraw` below; no registration is made at case
creation from bytes the parties supplied, and no quote field carries a
payout. The desk refuses to put a signing key into this field; the venue
does not know and does not care.

`withdraw(token, key, payout)` pushes everything claimable by `key` in
`token` to `key`'s registered payout as an ERC-20 transfer, and the venue
MUST accept it only from `key`'s current key in the succession book. Where
`key` has no payout and `payout` is nonzero, the call first registers
`payout` as `setPayout` would; where `key` has a payout, a `payout` of zero
or equal to it changes nothing and one naming another address MUST revert;
the call MUST revert when, after that, no payout is registered. Payment is
pull-shaped: nothing in the interface pays a seat without the seat asking; a
venue MAY, outside this interface, push a seat's claimable amount to the
payout that seat registered, and pays a seat nowhere else.

`accountOf(who, token)` returns the registered payout (zero when unset) and
the amount claimable now. The desk reads it for readiness, for the
remuneration row, as the evidence of a fee left unwithdrawn, and when it
adopts an existing key at first launch. No reader derives a per-case amount
from it; what each seating earned in its `token` is `seatOf`'s two credited
fields (§2.3).

A contract that hires a verifier (an escrow that pays a verification seat)
implements this face and `successionBook()` and nothing else of this
interface; the verifier's desk reads it the same way.

---

## 4. Panel face

A guild's roster lives in registry calldata and no contract can read it, so
a guild that wants its judges drawn puts the panel on chain: on the venue
itself, or on a **panel book** the venue's standard names, a contract
deployed apart from any case that the standard's text designates by address
or by code and that carries this face for every venue built to that
standard, a standard that deploys one venue per case having no venue that
could hold a panel before its cases exist. The book's code MUST be immutable
under §1's Identification rule, and the standard's text is what designates
it, by that code or by address; a venue's documentation repeats the
designation and adds nothing to it, and the standard's text names the view
by which a desk reads the book's record. Whichever contract holds the face,
`engage(termsHash, members, amanahHashes, rosterHash, expires, router)` is
sent to it from the guild's current key, the contract rejecting a call whose
sender is not the current key of its own record in the succession book it
names (a panel book naming it by `successionBook()` as a venue does):
`termsHash` names the terms or standard the panel serves, `members` is the
ordered array of judge addresses as the sender resolved them through
succession for this transaction, `amanahHashes` is the parallel array of the
guild's covenant word (§2.4), one element per member and every element
equal, `rosterHash` is the roster document's `word` (`galeed.roster/1` §1)
for later audit, `expires` is the unix second after which the panel may no
longer be frozen into a case, and `router` is the zero address or the
address of a router the guild names for the draw (below). The contract that
holds the face MUST reject a call whose two arrays differ in length, whose
`members` is empty, whose `amanahHashes` contains a zero word or two
elements that differ, whose `expires` is not after `block.timestamp`, or
whose `router` is nonzero and carries no code; it does not verify the
resolution, and the sender bears the drift between drafting and inclusion.

The contract that holds the face, venue or book, MUST record every accepted
panel, with its `router` and, where `router` is nonzero, that router's code
hash at the call, as one more entry in an append-only sequence under `(root,
termsHash)`, `root` being the root of the sender's record in the succession
book; a later `engage` adds an entry and retires nothing. The panel a case
draws from is frozen at the case's creation and is one of two, the venue's
documentation saying which: the latest of the panels recorded under the
guild's root and the terms before the creation block that are unexpired at
it, or one frozen into a record the case later opens under, such as a
certification a suit is later brought on, the expiry being tested at the
record and the case inheriting the panel the record froze; the guild's root
is read from the succession book at the case's creation (for a panel frozen
into a record, at the record's freezing) and frozen with it, and no panel
recorded after the case exists reaches it; the `termsHash` a panel is
recorded under names what the panel serves and need not equal the
`termsHash` of any seat drawn from it, a multi-class venue reporting a class
document on the seat (§2.3) and recording its panels under the terms that
name the guild; the venue MUST draw only from a panel recorded, on itself or
on the book its standard names, under the root frozen for the guild address
the terms or standard names, MUST NOT freeze into a case a panel whose
`expires` has passed at the moment of freezing, and, when a draw seats a
member, MUST freeze that member's word from the same panel's `amanahHashes`
into the seat's `amanahHash` and emit `Seated`. Expiry gates the freezing
and nothing after it: a panel frozen into a case serves that case, redraws
after a lapse included, until the case ends, so that no party loses a bench
to a third party's clock. A later `engage` reaches only cases created after
it. Where the frozen panel names a router, the venue MUST, at every draw for
the case, check that the router's code hash equals the one the panel
recorded, call `route(RouteView)` on it by `staticcall` with exactly
`routeGas()` gas (a value a guild reads before it engages, fixed by the
code, a constant or an immutable, as `successionBook()` is, §1), revert the
call, and with it every effect of the call, where the router call cannot be
given that gas, and seat `members[index]` where the call returns exactly one
32-byte word, an `index` inside `members` naming a member neither seated nor
offered on the case; a code hash that differs, a call that reverts or runs
out of that gas, return data of any other length, and an `index` outside
`members` or naming a member seated or offered on the case each leave the
case with no usable panel. `RouteView` carries what the venue holds at the
draw: the `caseId`, the seat's ordinal, the seat's `termsHash` (§2.3),
`token`, `amount`, and `rungs`, the parties as `seatOf` reports them, the
panel's `members`, every judge seated or offered on the case, whatever its
seat's state, one entry per seat in the order of the seats' ordinals, a seed
the venue's documentation states, the block's timestamp, the chain id, and
the venue's address. Where the panel names no router, the venue draws by its
own documented rule. Every draw, by a router or by that rule, seats only a
member neither seated nor offered on the case, the members compared by
address, and a draw that finds none leaves the case with no usable panel.
What a router computes is the guild's, and this text reads nothing of it.

Where a draw finds no usable panel (none recorded usable at the case's
creation, or the frozen panel's draw failing), the venue seats nobody at
that step: a creation that needs the seat is refused, and a redraw opens no
seat, the seat it follows leaving the in-flight states as the call that ends
it requires (§2.10, §2.11); the venue's documentation states what becomes of
the case. How the draw is made where the panel names no router, and how
standby fees are split among the members a case seats, are the venue's
rules; the venue MUST credit each member's share on account of that member's
seating (§2.3). The desk never recomputes a draw: `Seated` is the fact.

A venue that never draws implements `engage` and reverts, and so does a
venue whose panel face lives on the book its standard names: the book takes
the guild's call, applies the six rejections, and keeps the append-only
record, exposing `successionBook()` and `routeGas()` as a venue does and
naming the same succession book and the same gas as the venues built to its
standard, and the venue draws from what the book recorded. The venue's
documentation names the book, by address or by code, and the desk sends a
guild's `engage` to whichever contract the documentation names and reads the
record there.

---

## 5. Conventions that travel with the interface

- `termsHash` is `sha256` over the bytes of the governing document, and
  the venue's documentation states how a seated judge obtains those bytes.
- `manifest` is the digest of a courtevidence/1 manifest.
- `amanahHash` is the word §2.4 names, byte for byte.
- `entry`, in `rule` and `resign`, is the bytes of the judgment or
  resignation entry whose `entry_id` is `entryId`, as `galeed/1` §2.1 and
  §4 define them; the venue tests `sha256` alone, and what the bytes say
  is read by the desk and by every reader under that law.
- Timestamps are unix seconds as the chain reports `block.timestamp`;
  every window is inclusive of its last second.
- `token` is an ERC-20. The interface speaks no native coin: a venue
  that takes the chain's coin wraps it at its own door.
- Quote bytes are `abi.encode` of the values in `quoteType()` order with
  no enclosing tuple; `extra` bytes are `abi.encode` of the values in
  `extraType()` order with no enclosing tuple, and empty where the
  string is empty.
- Rungs are zero-based indices into the ladder the governing document
  states in prose; the document is read by people, never parsed by the desk.
- Role words are spelled as §2.12 states, in the order `partyRoles()`
  gives; every array of an `Outcome` follows that order,
  and every amount in an `Outcome` and in `seatOf`'s two credited fields
  is in the seat's `token`.

---

## 6. What the desk never does

Because these are the venue's, the desk has no code for them and a venue
that leaves them out is not served:

- deriving any clock, window, grace period or suspension from raw events;
- checking who assented to what, or in which form;
- recomputing a draw, comparing a panel to a live roster, or judging a
  draw's fairness;
- parsing a terms document, a standard, or a quote schema from prose;
- computing what a ruling or an advance pays, or what a seating earned,
  from a terms document, a quote, a ladder, or the venue's rules;
- reading the internal state of a company container, a multisig, or a
  smart account that is a party, a guild, or a payout target;
- learning any event, view, or state name beyond this text.

The same boundary serves the venue. A venue that needs a fact about a
party with code (its objection window, its funding table, the address its
refunds and payouts go to) takes that fact as a parameter of the party's
own consent call and freezes it as a term the party agreed to; it never
reads the party's code and never pushes money to an address the party did
not name for that purpose, so that a container, a multisig, and every
account shape still to come stand on equal footing.

---

## 7. Compatibility preconditions

A venue is compatible when every line below holds. Each is testable on a
local chain without the desk, but for item 3's equality with the judge's
document, which the desk reads.

1. The code at the declared address is immutable, and its `codeHash` is the
   one the judge declares.
2. Every seating emits `Seated` once, with the judge as its first indexed
   topic and `caseId` as the second, and `seatOf` answers for that pair from
   that block on and reverts for a pair never seated.
3. `seatOf(...).amanahHash` is nonzero for every seat, and equals the
   `entry_id` of the covenant entry the judge's document offers
   (`galeed.roster/1` §3a.3), or, for a drawn seat, the `sha256` of the
   guild's covenant text (§2.4).
4. `seatOf(...).termsHash` is the sha256 of the document that determines
   this seat's ladder, or of the relationship's own document while no claim
   has named a class (§2.3), and the venue's documentation says how the
   judge obtains that document.
5. `quoteType()` and `extraType()` are constant; every seat for which the
   venue decoded a quote freezes `keccak256` of the `abi.encode`d values
   into `quoteHash`, and every other seat reports zero; `rule` rejects a
   non-empty `extra` where `extraType()` is empty and rejects any `extra`
   but the named encoding where it is not.
6. The window in `seatOf` is the clock whose expiry moves the seat; while
   the seat is `disputed`, a `rule` or `resign` inside it from the seated
   judge's current key is never refused for time; a disputed seat's
   `windowEnd` is nonzero; a terminal seat keeps its last window.
7. `state` uses the six values of §2.6, changes only in a transaction,
   reports `lapsed` for the judge's inaction and for nothing else, and each
   seat's state is its own.
8. `accept` accepts only a call where `msg.sender` is the current key of the
   seat's judge in the succession book, equals `tx.origin`, and carries no
   code, and only on a seat that is `seated` with `accepted == 0` and not
   past a nonzero `windowEnd`, refusing none of these for time at any moment
   up to `windowEnd` (every moment where it runs no window); it sets
   `accepted` to `block.timestamp`, leaves the seat in flight, emits no
   `Anchored`; `disputed` is never reported with `accepted == 0`; a seating
   that carries the judge's consent reports `accepted` nonzero from its
   `Seated` block and rejects `accept`; an offer past its window goes to
   `closed` under `advance`.
9. Every commitment the venue counts emits `Committed`; `commit` accepts any
   address at every moment from the case's existence on at which some seat
   is in flight or, until the case ends, no seat has yet existed; a digest
   may carry several `Committed`.
10. `rule` and `resign` accept only a call where `msg.sender` is the current
    key of the seat's judge in the succession book, equals `tx.origin`, and
    carries no code, and both reject a call whose `entry` does not hash to
    `entryId` under `sha256`; `resign` is taken on a seat in flight, an
    unaccepted seat as an accepted one, at every moment up to its
    `windowEnd` (every moment while the window is zero), a revert §4
    requires of a draw apart, is refused on a seat out of flight, and leaves
    `accepted` as it was.
11. `rule` rejects a rung outside `rungMin..rungMax` whenever `rungs > 0`,
    and an in-flight seat with `rungs > 0` reports `rungMin ≤ rungMax <
    rungs`.
12. `rule` and `resign` emit `Anchored(msg.sender, entryId, entry)` with
    three topics and the call's `entry` as data, once each; nothing else in
    the venue emits `Anchored`.
13. `advance` is callable by anyone and, a revert §4 requires of a draw
    apart, moves the named seat past its window out of the in-flight states
    and neither reverts nor changes that seat's state otherwise.
14. `setPayout` and `withdraw` accept only the current key of the `key` they
    name; a payout is registered once per key, by `setPayout` or by the
    first `withdraw` that names one, and may be a contract; `withdraw`
    pushes an ERC-20 amount to the payout, reverts naming another payout,
    and reverts without one; no other path registers a payout; `accountOf`
    reports both; no party money passes through the face; and after every
    action the venue accepts, once every seat in flight and every seat the
    wind-down opens has ended by its judge's resignation or, past its
    window, under `advance`, every case with no seat in flight has ended as
    its documentation states, and every party has taken what the venue holds
    for it, the tokens it holds cover, in each token, everything claimable
    through the face (§3).
15. One of three branches. A venue that never draws: `engage` reverts. A
    venue that draws from panels it holds: `engage` accepts only a sender
    that is its record's current key, rejects unequal arrays, an empty
    `members`, a zero word, two words that differ, an `expires` not in the
    future, and a nonzero `router` without code, and records panels
    append-only under the sender's root, each with its router and that
    router's code hash. A venue that draws from panels on the book its
    standard names: its own `engage` reverts, and the book accepts only a
    sender that is its record's current key, applies the six rejections,
    keeps the append-only record under the sender's root with each router
    and its code hash, and names the venue's succession book and route gas.
    In both drawing branches the panel a case draws from is frozen at the
    case's creation and is one of §4's two, the latest of the panels
    recorded before the creation block under the root frozen with it and the
    terms that are unexpired at it, or the one a record the case opens under
    froze, and serves that case to its end; where it names a router the draw
    calls it as §4 states, the venue's call reverting, with every effect it
    had, where it cannot give the router `routeGas()`; every draw seats only
    a member neither seated nor offered on the case; the draw seats only a
    member of the frozen panel the guild the terms name recorded, and that
    member's word becomes the seat's `amanahHash`.
16. Every address the venue records stays bound to its record for the
    record's life; the key that acts for a recorded judge's or guild's
    address is that address's current key in the succession book, read at
    the call; `successionBook()` names the book the family declares for the
    venue's chain (`galeed.roster/1` §9) and is fixed by the code, a
    constant or an immutable, and so is `routeGas()`.
17. The venue's documentation states the `caseId` derivation, the moment the
    venue records a `caseId` and the moment a case with no seat ends (§2.8),
    what becomes of a case where a draw finds no usable panel (§4), the
    state mapping, which seatings carry the judge's consent and which wait
    for `accept`, which contract holds the panel face (the venue, or the
    book its standard's text designates, repeated from that text), which of
    §4's two panels a case draws from and the `termsHash` it reads them
    under, the rule it draws by where a panel names no router, the seed a
    router is handed and what of it and of the `caseId` a party or a block
    producer can choose, the meaning of the fields `extraType()` names, the
    quote decoding, the address that stands in each role of `partyRoles()`,
    the form of the acceptance and the dispute `outcomeOf` takes as made,
    any fee it credits a seating in a token other than the seating's
    `token`, and how the terms bytes are obtained.
18. `partyRoles()` is constant and well formed, and every `Outcome` carries
    one element per role in each array, the shares together no more than the
    seat's `amount` and the parts of `rulingFeeFrom` no more than
    `rulingFee`; `outcomeOf` answers on every seat in flight but one whose
    ladder is not yet determined, with its present `amount` and range, for
    every rung inside the range (any rung where `rungs` is zero) and every
    `extra` the venue accepts, and on every ruled seat for the ruling it
    took, every such answer reporting `ruled`, and reverts for a rung or an
    `extra` that `rule` rejects for range or shape; `advanceOf` answers for
    every seated pair, reverts for a pair never seated, and reports the
    present state and zero amounts where nothing is due; every `rule` and
    `advance` the venue accepts does what the matching view reports on the
    state the accepted call finds, apart from what §2.12 puts outside the
    `Outcome`; `standbyCredited` and `rulingCredited` never decrease and
    count every amount credited to the judge, in the seat's `token`, on
    account of the seating, a ruling fee only where a ruling the seat gave
    makes it; a share of a standby split is credited on account of the
    member's own seating; and `outcomeOf` and `advanceOf` answer alike
    whoever calls them.

---

## 8. Conformance replay

The desk's harness (`galeed-conformance/venue-replay/`, an abstract Foundry
test a venue completes by supplying its own creation and dispute paths)
replays the following against any venue on a local chain and reads the venue
through the interface, its code, and the accesses and calls its execution
makes (steps 1 and 14), beside the state of the token and of the book and
the logs of the calls it keeps. Seats are named by role, never by ordinal,
and states by class: *in flight* is `seated` or `disputed`, *terminal* is
any other value. A venue passes when every step lands where it is stated to,
every emitted log has the stated shape, and every amount the replay measures
is as stated.

1. Deploy; read `codeHash`; `quoteType()`, `extraType()` and `partyRoles()`
   are constant across two reads, and `partyRoles()` is well formed, its
   words distinct (§2.12); `successionBook()` names the book the replay
   deployed, which carries the code of `GaleedSuccessionBook`, the book of
   `galeed.roster/1` §4, and on which the replay moves every key (where the
   venue's code names the book by a constant address, the replay places that
   code there); the venue's code placed at another address reads the same
   `successionBook()` and `routeGas()`; and the two reads touch no storage
   of any account, the venue's own included, and no balance or code of
   another, call no account whose code is a delegation designator, and
   answer alike under other balances, a later clock and another block, on
   the venue, on every other venue of its standard the replay meets, which
   answers the five reads as the venue does, and on the panel book.
2. Once the first case of step 4 seats `J` (where the venue draws, after
   step 3) and before any registration, `accountOf(J, token)` reports a zero
   payout; judge key `J` calls `setPayout(J, P)`; `accountOf(J, token)`
   reports `P`; a second `setPayout(J, …)` reverts; `setPayout(J, …)` from a
   stranger reverts. The replay seats `J` before any payout is registered,
   so a venue that seats only a judge with a payout is not measured by this
   replay.
3. Where the venue draws from panels it holds: guild address calls
   `engage(T, [J], [A], R, X, 0)` on the venue with `X` in the future, the
   guild's latest panel before step 4, so that every draw rule seats `J`; a
   call with arrays of unequal length, an empty `members`, a zero word, two
   words that differ, an `X` not after the block's timestamp, or a nonzero
   router without code reverts. Where it draws from panels on the book its
   standard names: the same calls go to the book with the same outcomes, and
   `engage` on the venue reverts. Where it never draws: `engage` reverts. A
   panel of two members under one word is taken, and panels a stranger
   engages under the same terms, before and after the guild's, are never
   drawn for a case naming the guild.
4. Create a case under the venue's own creation path and bring it to the
   point where the judge is seated, by whatever path the venue documents:
   exactly one `Seated(J, C, s)` for the seat `s` that path opens;
   `seatOf(C, s)` shows `judge == J`, `amanahHash == A`, nonzero
   `termsHash`, a state in flight, a zero `quoteHash` where `quoteType()` is
   empty, and, where the venue's suite names the quote bytes the creation
   copied, `quoteHash` equal to their `keccak256`; a stranger's `commit` on
   the case as opened is taken; `seatOf` and `advanceOf` revert for an
   unknown case and for an ordinal the case never seated, and `advance` on
   either neither reverts nor changes the state of the case's seat. 4a.
   Where the seating waits for the judge's word: `accepted == 0`, the state
   is `seated`, and `windowEnd` is nonzero exactly where the suite says the
   offer runs a clock; `accept` from a stranger reverts, from `J` through a
   contract reverts, from `J` carrying code reverts; from `J` succeeds,
   emits no `Anchored`, leaves the state in flight with `accepted ==
   block.timestamp`, and what it credits is counted on the seating; a second
   `accept` from `J` reverts. Where the seating carries the judge's consent:
   `accepted` is nonzero already and `accept` from `J` reverts; so too on a
   case the suite opens with the judge's standing consent copied into the
   terms.
5. `commit(C, M1)` from a party, `commit(C, M2)` from a stranger and
   `commit(C, M4)` from a stranger through a contract each emit `Committed`
   naming its sender, the contract for the last; the desk's committed set
   contains `{M1, M2, M4}` plus whatever the creation path and step 4
   committed.
6. Bring the seat that will rule to `disputed` by the venue's own path,
   which may be the same call as step 4; its window is nonzero and
   `windowEnd >= block.timestamp`, `accepted` is nonzero, and where `rungs >
   0` the range lies inside the ladder; a stranger's `commit` is still
   taken.
7. On that seat, `rule` from a stranger reverts; from `J` through a contract
   that calls the venue reverts; from `J` carrying code reverts; from `J`
   with a rung below or above the range reverts when `rungs > 0`; from `J`
   with a non-empty `extra` reverts where `extraType()` is empty, and where
   it is not, with an empty `extra`, with one a byte longer, and with each
   `extra` of the right length in which one word of a type narrower than a
   word carries a bit outside its type, reverts; from `J` with an `entry`
   whose `sha256` is not `E`, or an empty one, reverts; from `J` with a
   permitted rung (where `rungs` is zero, rung 7, which the venue ignores),
   the entry bytes, and the named `extra` at `windowEnd` succeeds, emits
   exactly one `Anchored(J, E, entry)` with three topics and the bytes as
   data, and leaves the seat `ruled` with the window, the `accepted` and the
   range it had.
8. Second case, seated and disputed; at `windowEnd` nothing is due, and
   `advance` from a stranger does what `advanceOf` reports and leaves the
   seat `disputed`; warp past `windowEnd`, where the seat still reads
   `disputed`; `advance` from a stranger succeeds and the seat reads
   `lapsed` with the window, the `accepted` and the range it had; where a
   venue that never draws reassigns, a `Seated(J', C2, s')` follows with a
   new ordinal and `seatOf(C2, s')` reads in flight, and where its suite
   says it does not, no seat follows (a drawing venue's redraw is step
   14's).
9. Third case, seated and accepted; `resign` from `J` on its seat with bytes
   whose `sha256` is not `E3` reverts; with the entry bytes it succeeds,
   emits exactly one `Anchored(J, E3, entry)`, leaves the seat `resigned`
   with the window, the `accepted` and the range it had, and a second
   `resign` reverts. 9a. Where the seating waits for the judge's word and
   runs an offer clock: fourth case, seated, unaccepted, `windowEnd`
   nonzero; at `windowEnd` nothing is due, `accept` from `J` is taken (on a
   snapshot), and `advance` from a stranger does what `advanceOf` reports
   and leaves the offer open; warp past `windowEnd`; `accept` from `J`
   reverts; `advance` from a stranger succeeds and the seat reads `closed`,
   never `lapsed`, with the window, the `accepted` and the range it had;
   where a venue that never draws reassigns, a `Seated(J', C4, s')` follows
   with a new ordinal, and where its suite says it does not, no seat
   follows. Fifth case, seated, unaccepted; `resign` from `J` succeeds,
   emits exactly one `Anchored(J, E5, entry)`, leaves the seat `resigned`
   with `accepted == 0` and the window and range it had, and `accept` then
   reverts. An accepted seat whose own clock runs: at its last second
   nothing is due and `advance` leaves it seated; past it, it reads `closed`
   under `advance`, never `lapsed`, with the window, the `accepted` and the
   range it had. 9b. A seat disputed: at `windowEnd`, `resign` from `J`
   succeeds, emits exactly one `Anchored(J, E6, entry)`, and leaves the seat
   `resigned` with the window, the `accepted` and the range it had.
10. `advance` on a seat with nothing due, in flight or terminal, neither
    reverts nor changes its state.
11. `withdraw(token, J, 0)` from `J` moves `claimable` to `P` as an ERC-20
    transfer, from a stranger it reverts; `withdraw(token, J, Q)` with `Q`
    another address reverts; from a key with no payout and a zero `payout`
    it reverts.
12. Over the whole replay, the closing solvency sweep of step 15 apart,
    `Anchored` logs number exactly the accepted `rule` and `resign` calls,
    each sent by a key of `J`'s record or of the record of a judge step 14
    has resign, every hash among them is an `entryId` argument, and every
    log's data decodes to the `entry` argument of that call, whose `sha256`
    is its hash.
13. In this step, and in step 14 before the guild's last `engage`, before
    each call §1 gates by the current key, the replay moves the record's key
    on the succession book to a fresh key, twice before the record's first
    gated call, so that the record holds a root, a key between and the
    current key and the key moves again and again: `accept` on an offer,
    `resign` on an accepted seat and on an offer, and `rule` on a disputed
    seat, all opened before the first move, each revert from every key the
    record held before, its root and every key between, from the current key
    through a contract, from the current key carrying code, and from a
    contract's constructor at the current key's address, and succeed from
    the current key, the ruling doing what `outcomeOf` reported for it (step
    15); `withdraw(token, J, P)` on `J`'s account, and `setPayout` and a
    `withdraw` that registers a payout on two keys with no payout, each
    revert from every key the record held before and from the current key
    through a contract, and succeed from the current key; `setPayout` from a
    stranger and from the key `J` has just left reverts; the withdraw moves
    `J`'s claimable to `P`; on an account with no payout, `accountOf`
    reports zero until the current key registers one, and the payout it
    registers may be a contract.
14. Where the venue draws, the replay keeps its own record of the panels it
    engaged and predicts, for every case it opens and every draw, the panel
    §4 freezes: the latest of the panels recorded under the root of the
    guild address the terms name and the terms before the creation block
    that are unexpired at it, or none, in which case the creation reverts.
    The record holds a latest panel expired over an older one unexpired, a
    panel at its `expires` second and one second past it, a panel recorded
    in the creation block, a panel under other terms recorded after the
    guild's, a guild address named by its current key after the guild moved,
    and a panel recorded after a case's creation, which is never drawn for
    it; every panel carries a word of its own, and each seat carries the
    word of the panel it was drawn from. When the guild moves its key,
    `engage` reverts from every key its record held before and succeeds from
    the current key, the panel filed under the guild's root; an `engage`
    from a contract the current key calls is filed under the contract's own
    root, and no case naming the guild draws from it. A router whose `route`
    returns a member's index seats that member; a router that reads the
    venue, the chain, the time, the members and the number seated, one over
    a panel that lists a member twice, and one whose index is the hash of
    the case, the seat's terms, token, amount and rungs and the parties,
    each seat only where those fields are the venue's; a router that answers
    by the gas it receives seats only the member it names where the gas it
    reads at its entry lies within a few units of what `routeGas()` leaves
    there, and over a sweep of gas limits placed by the path's own cost,
    which finds the lowest limit at which the call lands and tries every
    limit from 50 below it to 400 above, the call either reverts with every
    effect it had or draws, drawing at some limit; and a router that
    reverts, spends the gas it is given (failing at once where it is given
    more than `routeGas()`), returns two words or 31 bytes, writes state,
    returns an index outside `members` or one naming a member already
    seated, or whose code hash changed after the panel recorded it, leaves
    the case with no usable panel. Every redraw the venue makes, whatever
    call makes it, is measured where it is observed: on each path by which a
    call takes a seat out of flight (an expired offer under `advance`, a
    lapse under `advance`, the judge's decline, the judge's resignation of a
    seated and of a disputed seat, and an `advance` naming a seat that left
    flight while a later seat's own clock has run out, that seat an offer
    past its window, an accepted seat past its clock, or a disputed seat
    past its ruling window), the replay sends the call twice, once while the
    frozen panel is unexpired and the latest and once after it has expired
    under a later panel of a member and a word of its own, and, on each path
    whose call takes the seat it names out of flight, again on the seat a
    redraw opened, and, where a `Seated` follows, measures redraws on that
    path: each reads the frozen panel over a panel recorded later, after it
    has expired where the path waits on a clock, and under the root the case
    froze after the named address joined another record; its router call is
    swept as the creation's is, and swept again under a router that answers
    only where the gas it reads at its entry lies within a few units of what
    `routeGas()` leaves there and fails at once otherwise, over every limit
    from 50 below the least limit at which a redraw whose router fails lands
    to 400 above; a router naming a seated member, an index outside
    `members`, failing, or whose code changed after the case's creation
    leaves the case with no usable panel, no seat following and the seat it
    follows leaving flight as the path requires; the routers that read the
    view and the case's fields seat only where the view carries the venue's
    own, the fields as `seatOf` reports them for the seat drawn; and the
    view the venue hands a router at a redraw, read from the call itself,
    carries every field §4 names but the seed exactly. Where the suite says
    whether the venue redraws on a path, what the replay observes agrees.
    Every seat a redraw opens takes a stranger's `commit` while in flight,
    refuses a stranger's `accept` where it waits for one, a stranger's
    `rule` and `resign`, and its judge's `rule` through a contract and
    carrying code, and is accepted, disputed, and ruled on a snapshot at its
    ruling window's last second at each of the first 16 rungs of its range,
    each ruling emitting one `Anchored` and doing what `outcomeOf` reported
    for it (step 15); what the step's resignations credit is counted on the
    seatings. Over the whole replay every `Seated` of a case drawn from a
    panel carries the frozen panel's word and names a member of it that no
    earlier seat of the case holds. A drawing venue's suite supplies the
    path that opens a case naming the guild by a given address. A panel book
    names the venue's succession book and route gas.
15. Around every ruling and every advance the replay measures on a seated
    pair, `outcomeOf` with the ruling's arguments, and `advanceOf` on the
    seat, read on the state the call then finds, report the state the seat
    moves to and arrays of one element per role, the shares together no more
    than the seat's `amount` and an advance's `rulingFee` zero; on a seat in
    flight the seat's `standbyCredited` and `rulingCredited` rise by exactly
    the reported `standbyFee` and `rulingFee`, and `accountOf(J, token)`'s
    claimable by their sum, plus what the transaction adds to the credited
    fields of `J`'s other seatings in that token, less what the venue pushed
    to `J`'s payout in that transaction, and on a seat that left flight by
    what the transaction adds to the credited fields of `J`'s seatings, less
    what it pushed; where the venue's documentation names the addresses that
    stand in the roles and the path by which they take what it holds for
    them, each role receives at least its element of `shares`, counted as
    what the token records its addresses received (the ERC-20 transfers into
    them where the token keeps no such record), that path included, and
    before a party takes what the venue holds for it nothing of it is
    claimable through the economic face. Each of the first 64 rungs of the
    ruling seat's range, ruled on a snapshot, does what `outcomeOf` reported
    for it, and what `outcomeOf` reported for each rung before acceptance is
    what it reports once the seat is disputed, where the amount, the range
    and the time are unchanged; where the seating's ladder is determined
    (§2.5), `outcomeOf` answers on the seat in flight before acceptance,
    accepted, disputed, past its window, and on an expired offer, for each
    of the first 64 rungs of the range or, where `rungs` is zero, for any
    rung, and reverts for a rung outside the range and for an `extra` of
    another shape (step 7's); after the ruling, `outcomeOf` with the same
    arguments on the ruled seat reports what it reported before; `advanceOf`
    reports the present state and zero amounts at step 10, at a window's
    last second, and on every seat that left flight, `outcomeOf` for each
    rung of a ladder's range where the replay reads the range, and
    `advanceOf` and `seatOf` around every advance, answer alike from `J`,
    from the zero address and from a contract, and under another coinbase,
    randomness, base fee and gas price; at step 10 the advance on the seat
    in flight moves neither of that seat's credited fields, and the
    claimable only by what it adds to the credited fields of `J`'s other
    seatings. Over the whole replay no `standbyCredited` or `rulingCredited`
    falls, the credited fields of every judge whose seatings are all in the
    fee token sum to what its payout received and what it can still claim,
    every `Seated` names the seat's judge and comes once per pair, a seat
    that gave no ruling carries no ruling fee, a lapsed seat was accepted,
    no party's money is claimable through the economic face, and no judge
    but `J` has a payout. After each rung of the ruling seat ruled on a
    snapshot, after each ruling of a seat a redraw opened, and at the end of
    the replay, once every seat in flight, and every seat those endings open
    (up to sixteen rounds), has ended by its judge's resignation or, past
    its window, under `advance`, and, where the suite names the roles'
    addresses and their path, every party has collected, each venue the
    replay met holds, in the fee token and the harness token, what every
    judge it seated there can claim (§3).

---

## 9. Outside the interface

Everything else is the venue's freedom: escrow and settlement, fee schedules
and how a quote decodes, the draw algorithm where a panel names no router,
the number of judges a case seats, appeal chains, evidence formats beyond
the manifest digest, the parties' account shapes, the terms document's
structure, and every additional event and view the venue's own clients want.
None of it is read by the desk beyond the results §2.12 reports, and none of
it needs the desk's permission.

---

## 10. The interface's identity

Naming a digest here is the freeze, and it is the only act that performs it.
What a venue binds to is the ABI: the three events, the `Seat` and `Outcome`
structs in their field order, the seventeen functions of `IGaleedVenue` in
§1, each with its selector, and the functions and the struct the same file
declares for the succession book and the router; what a desk binds to is the
meaning this text gives each member. The identity of `galeed.venue/1` is the
sha256 of the interface file
`galeed-conformance/venue-replay/src/IGaleedVenue.sol`, which carries §1's
declaration verbatim after its license line, its pragma, a blank line, and
one comment line, and nothing else:

    0xf97ad599d9d4f5149fb93962c15f18b2f601251f2085358c3117ad8fe5af391e

named on 2026-10-24, computed by `galeed-conformance/venue-digest.sh`; the
review record beside this text (docs/galeed-venue-v1-review.md) records the
text's own digest at the naming and how the interface converged on the
shared replay of §8.

From the naming, the ABI is fixed for every contract that declares the
literal, and this text is its meaning: a contract that implements the ABI,
honours every MUST of §2 through §7 and meets every line of §7 is a
`galeed.venue/1` venue, and a judge's desk reads it by this text forever,
however the desk itself changes. A change to any member, event, struct
field, selector, MUST clause, or line of §7 is not a revision of this text:
it is `galeed.venue/2`, a new literal with its own identity, read beside
this one and never in its place, since a deployed venue cannot follow. Until
a venue deployed for use binds to the identity named here, the naming may be
redone by the same act, the superseded identity standing in the review
record beside this text; from that first deployment on, the sentence before
this one governs. The replay of §8 and the harness that runs it are measures
and bind nothing; they may be corrected at any time without touching the
identity named here.