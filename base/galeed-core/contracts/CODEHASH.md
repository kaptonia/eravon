# GaleedRegistry codeHash

Runtime bytecode length: 997 bytes.
codeHash (keccak256 of the runtime bytecode): `0x5e12bdb00bad03039a110c509428bbb7cd2247c80d27a867c391a92bd4140443`

Settings that produce it (foundry.toml): solc 0.8.28, evm_version cancun,
optimizer on with 200 runs, via_ir true, bytecode_hash none, cbor_metadata
false. Any change to the source or to these settings changes the codeHash;
a reader that pins a registry by codeHash is pinning exactly this.

Interface read by the laws: `galeed/1` section 9.1 reads the event
`Anchored(address indexed sender, bytes32 indexed hash, bytes entry)`, topic 0
`keccak256("Anchored(address,bytes32,bytes)")`, emitted by
`anchorCarried(bytes32,bytes)` and `anchorCarriedMany(bytes32[],bytes[])`,
each of which refuses a call whose entry does not hash (sha256) to its hash,
so that no log of this contract carries bytes that do not match their word;
the hash sits in the calldata at offset 4 (`anchorCarried`) or, under the
canonical ABI encoding, at 4 plus a multiple of 32 (`anchorCarriedMany`'s
elements), which is the containment section 9.1 requires; the contract also
emits for a non-canonical `anchorCarriedMany` encoding that places an element
off every admissible offset, and section 9.1 then declines the log (fixture
14). `galeed.roster/1` section 1 reads the events
`Published(address indexed publisher, bytes32 indexed word, bytes document)`,
emitted by `publish(bytes)` with `word` the keccak256 the contract computes
of the document bytes, and `StatusChanged(address indexed who, bool active)`,
emitted by `setStatus(bool)`. Every document of the family (a roster, a
guild card, a judge document, a guild covenant, a verifier's service
record) is published whole through `publish`; the covenant a judge offers is
an `amanah` entry of the judge's ledger, anchored through `anchorCarried` like
every other entry. The contract holds no state and has no owner.

Fork floor: the pinned build (evm_version cancun) emits PUSH0, so this
codeHash is deployable only on chains at or above Shanghai. A chain below
that needs its own pinned build and its own recorded codeHash; the law
reads registries by the basis's declared addresses or by a class item's
codeHash, so a second build is a deployment convenience and never a grammar
matter.

The suite reads `forge-std` from the repository's `contracts/lib` (`libs`
in `foundry.toml`); the codeHash depends on solc 0.8.28 and the pinned
settings alone, and the suite builds inside the repository or inside a
package that carries that library at the same relative path.

# GaleedSuccessionBook codeHash

Runtime bytecode length: 913 bytes.
codeHash (keccak256 of the runtime bytecode): `0xdb3293b42187b8b7270e62d5eaafee45af02c9caf3a0c8fe376cb4195f2f6aca`

Settings: the same `foundry.toml` as the registry, and the same rule: any
change to the source or to the settings changes the codeHash, and a reader
that pins the book by codeHash (`galeed.roster/1` section 9) is pinning
exactly this.

Interface read by the laws: `galeed.roster/1` section 4 reads the events
`Declared(address indexed from, address indexed to)`, emitted by
`declare(address)` with `from` the caller, and `Acceded(address indexed
holder, address indexed to, address indexed root)`, emitted by
`accede(address)` with `to` the caller; the contract admits a call exactly
where that section's replay binds a declaration or counts an accession, and
`current(address)` and `root(address)` (the two views `galeed.venue/1`
section 1 declares as `IGaleedSuccessionBook`) return the replay's current
key and root after every call, `pending(address)` and `entered(address)`
its pending key and whether the address has entered the book. The contract
has no owner, no upgrade path, and no external call, reads no registry, and
carries no anchor. The family declares, for each chain, the address of the
book its readers resolve identities on, the mainnet book being the reference
(`galeed.roster/1` section 9); every such book carries this codeHash, and a
venue names its chain's book in `successionBook()`.

# GaleedRouter codeHash

Runtime bytecode length: 3496 bytes.
codeHash (keccak256 of the runtime bytecode): `0x492e67518544e136c69897b93df1555ccf722f438fbb5bbd3d00f9a1f6348986`

Settings: the same `foundry.toml`, and the same rule. Every deployment
carries this runtime code and this codeHash whatever its rule table: the
table is the constructor's argument, kept as the code of a data contract
the constructor creates (the first CREATE of the router's address, its
first byte STOP), and the runtime code holds no immutable, so that a desk
recognises the standard router by this codeHash alone and reads its table
through `rules()`. `rules()` and `route` revert (`ForeignTable`) where the
code at that address is empty or does not begin with STOP, so that the
table this code reads is one nothing can execute or destroy, on any fork.
The codeHash attests the runtime code and nothing else of how the table was
written: init code that creates a table without the constructor's checks
and then returns this runtime code carries the same codeHash, so a desk
reads `rules()` and refuses every table the constructor's checks refuse,
those checks being the rule: among them a table with no rule, a range whose
minimum exceeds its maximum, a mode other than the three, a `WEIGHTED` rule
whose pool is empty or whose weights are not one positive weight per pool
element, a rule of another mode that carries weights, a zero or repeated
pool address, and a table whose encoding, with the STOP byte before it,
exceeds 24,576 bytes (the largest accepted encoding is 24,544 bytes). The
constructor copies memory with `MCOPY` and runs only on a chain at Cancun or
later; the runtime code uses no opcode later than Shanghai's `PUSH0`.

Interface read by the laws: `galeed.venue/1` section 4 calls
`route(RouteView)`, the selector of the `IGaleedRouter` member that section
1 declares, by `staticcall` with the venue's `routeGas()`. The router has no
owner, no upgrade path, no `delegatecall`, and no storage; `route` reads its
argument and the table's code and nothing else, and reverts where no rule
decides. A rule holds where its conditions hold (settlement token, the zero
address matching every token; an inclusive amount range; terms, the zero
word matching every terms; an inclusive rung count range; an inclusive seat
ordinal range); its action draws among its pool, or the panel's members
where the pool is empty, the members already seated or offered on the case
excluded, uniformly (`UNIFORM`) or in proportion to positive weights
(`WEIGHTED`), by the venue's seed with the case id and the seat ordinal, or
by rotation from the place the hash of the case id names (`ROTATION`); the
first rule that holds and finds an eligible member decides. A rule whose
pool is the panel's members draws among the places of `members` and returns
the place it lands on, so that a member the panel lists twice is drawn twice
as often; a rule with its own pool returns its member's first place in
`members`.

Cost: `route` decodes the whole table on every call and then walks the
rules; a rule over the panel's members costs about 1,500 gas per member and,
for every judge seated or offered on the case, about 150 more per member,
rising a little with the panel's size as the call's memory grows, and its
draw then walks the eligible places up to the one it lands on, about 300 gas
each, so that one panel's cost varies with the seed and the case id (a
uniform rule over 128 members with eight seated takes from about 326,000 to
about 361,000 gas); a rule with its own pool adds a scan of `members` per
pool element; and the first touch of the table's account in a transaction
costs 2,500 more than a later one (EIP-2929). The sender of a drawing call
can lower the cost, by the block it chooses and by an access list naming the
table, and cannot raise it above a draw that lands on the last eligible
place with the table untouched. A guild measures that cost on the router it
names against the `routeGas()` of the venues it engages before it engages,
since a draw the gas cannot pay leaves the case with no usable panel, and a
panel that fits at some seeds and not at others lets the sender of each
redraw choose between a draw and no usable panel.

A draw is as unpredictable as the seed the venue hands the router and the
case id: where a party or a block producer can choose either, it can steer
the draw, and the venue's documentation says which seed it hands
(`galeed.venue/1` section 7 item 17). A venue pins a router by its code
hash; an account delegated by EIP-7702 carries the delegation designator
as its code, whose hash pins the delegate's address and none of the
delegate's code, so a guild names a deployed router and never a delegated
account.
