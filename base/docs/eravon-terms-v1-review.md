# eravon.terms/1: the review record

The convergence of `eravon.terms/1` (docs/eravon-terms-v1.md) is run the way
`galeed/1`, `galeed.papers/1`, `galeed.roster/1`, and `courtevidence/1`
were run: adversarial readers of the text and independent implementations
written from the text and the harness alone, one round at a time, every
finding folded in one pass, the criterion, the harness, and the corpus
following, until the readers report nothing open and the implementations
reach zero divergences with every closed boundary witnessed on both
sides. The law is written from the design volume (docs/eravon-design.md)
and the frozen `galeed/1`, `galeed.roster/1`, and `galeed.venue/1`
texts. The three ERAVON document laws are run together, each round
seating one reader per law, a corpus-and-harness auditor, an
ecosystem-fit reader against the design volume and the GALEED laws, and
three implementers (Rust, Node.js, Python) each implementing all three
laws; this record carries what concerns `eravon.terms/1` and the shared
seats, and the two sibling records the rest.

## Round 0: the criterion against its own generator

The criterion (`eravon-conformance/terms-py/`) and the corpus generator
(`eravon-conformance/terms-corpus/gen.py`, 307 cases at seed 1) were
written from the text by one hand and compared under seeds 1 to 4: zero
disagreements with the generator's hand-written predictions.

## Round 1: five readers and three implementations

Readers: the terms text (A), the link text (B), the dossier text (C), the
corpora and harnesses (D, who re-derived 121 predictions by hand with
zero disagreement and regenerated seeds 2 to 6), ecosystem fit against
the design volume and the GALEED laws (E). Implementations: Rust (no
dependencies, its own secp256k1, P-256, AES, PBKDF2, canonical form and
entry test), Node.js (standard library plus its own keccak, secp256k1,
P-256), and a second Python (its own primitives, no `json` module in the
acceptance path), each checking its `galeed/1` pieces against the frozen
corpus first (3,616 canon cases and 29 sign cases each, zero
disagreements).

The three implementations reached zero divergences on all three corpora
at seeds 1 to 6 (3,990 case runs each) and probed beyond them (Rust
5,097 inputs, Node 8,286, Python 5,233) with one disagreement in all,
on a sentence the text left open (the length `BIND_JUDGES` carries).
No finding was against the criterion. The readers found 20 (A), 20
(B), 18 (C), 18 (D), and 34 (E) items and the implementations 10, 11,
and 15; deduplicated, the load-bearing ones on this law were: the letter
`c` bound to the chain id in §4.2 and §4.4 and to the candidate number
in §4.3; `E_JUDGE_QUOTE` and `E_GUILD_QUOTE` said to carry nothing where
§7 and the harness carry an index; a `compliance` object whose strings
were skeleton strings under §1, so that a legal name outside ASCII was
refused; `keccak256(x ‖ y)` not saying bytes; the `EIP712Domain` string
line-wrapped in prose; the passkey low-s rule that the P-256 precompile
does not impose and that no producer obligation backed; the `[O]` class
said to be in no corpus while the producer commands measure §5.4's
determinism; `nonce` a `uint256` in the type string and an `int` in the
document; `E_UTF8` and quote words 7, 9, 10, 11 unreachable yet inside
closed lists; nothing comparing a pointer's `chainId` with the
document's; §9 putting an unnamed contract above the frozen core; the
five `…BySig` and `settle` messages the design volume names being in no
law; `clientDataJSON` a prose string here and a skeleton string inside
every canonical document of the link law.

Folded (Court commit after this record):

- §4.2 and §4.4 name the chain id `chainId`; §4.2 shows the
  `EIP712Domain` string in the display block with the other three; §1
  carries P-256's base point and drops `u16`, `u8`, `hex16`; `termsHash`
  is total as `galeed/1` §2.1 makes identity.
- §2.3's compliance member is `compliance_md`, so every string within it
  is prose; `nonce`'s narrowing to 2^53 − 1 is stated as deliberate; §2.4
  says which element the repeat's index names; §2.5 says
  `E_JUDGE_QUOTE` and `E_GUILD_QUOTE` carry their index and no inner
  fault where the member was no hexbytes, and compares a pointer's
  `chainId`, a guild's and a judge's `source` alike, with the document's;
  §2.6 says the prose may not carry the opening fence.
- §3.2 is written with every clause naming the fault it fails; §4.1 lists
  the nonzero guards the design volume states, includes the ladder-wide
  rules, and says it states no token of its own; §4.3 takes `quoteHash`
  over the bytes the member spells.
- §4.5 fixes the five action messages (`ClaimDelivery`, `Confirm`,
  `Dispute`, `Refund`, `Settle`) as EIP-712 types under the instance's
  domain, with their argument tests and the party rule; the design
  volume §3.10 names them.
- §5.3 spells the address derivation in bytes, makes `clientDataJSON` a
  skeleton string tested inside `E_PASSKEY_FORM`, drops the low-s rule
  (`E_PASSKEY_HIGH_S` gone from §7; §8 says why), and takes the
  challenge over whichever digest is being verified; §5.4 states the
  passkey producer's obligations as they are.
- §7 names the unreachable `E_UTF8` and the four unfaulting quote words;
  §8 carries the P-256 malleability bullet; §9 says a differing standard
  is a standard the law does not describe and the core decides until one
  artifact is amended; §10.1 states the boundary definition, the `[O]`
  measure, and the fuzz deferral as `galeed.papers/1` §13.1 states it;
  §10.4 says a `galeed/1` recast re-pins the digest.
- The design volume: `init` enforces the quote decoder's bounds
  (`premiumBps`, `feeBps` ≤ 10000; `lapseDefault` ≤ 2; `autoAccept` ∈
  {0, 1}), the ladder's endpoints and monotonicity in §3.3's guard
  column, the floor on LF's bps term, the passkey tests the contract
  runs and the absence of a P-256 low-s rule, the five action type
  strings and the settlement document, the `onramp` row of §6.1's
  payload table, a guild pointer and an unnumbered milestone in §7.1's
  machine block, the terminal numbering on `Closed`, the buyer seat's
  own receipt and pack on the client-paid path, and the compliance
  member's name.
- Harness: the producer commands answer with `ok`; the index-carrying
  glob names the four bare element tokens; the empty `<m>` is misuse;
  `action`, `sign-action`, `sign-passkey-action`, `verify-action` added
  with the action name's closed list and the party argument.
- Corpus: 372 cases, with the `nonce` before `expiry` adjacency, the
  empty `<m>` and the empty pointer chain, `verify-offer` over a faulting
  document and a faulting derivation, a wrong signature over a real point
  (`E_SIG_SIGNER`) beside a non-point (`E_SIG_RECOVER`), the pointer
  chain both ways, `compliance_md` with prose inside and `compliance`
  without the suffix, the passkey twin verifying and a non-ASCII client
  data refused, and the action families; the generator's P-256 is its
  own, checked against RFC 6979's A.2.5 vector at import, and its
  keystore uses pycryptodome's AES, so no prediction passes through the
  criterion.

## Round 2: the same eight seats on the revised text

Every seat re-read the revised text and harness in full. The three
implementations, rewritten from the texts alone with the criterion a
black box until after the first measurement, reached zero divergences on
all three corpora at seeds 1 to 6 (5,034 case runs each) and probed
beyond them (Rust 4,375 inputs, Node 6,475, Python 4,019) with one
disagreement in all, on a harness sentence that left the order of the
`<c>` bound against a pack's own fault open. The readers closed every
round-1 item and found 13 (A), 10 (B), 12 (C), 6 (D), and 11 (E) items
and the implementations 7, 6, and 4; deduplicated, the load-bearing ones
on this law were: §4.5 not saying whether the party rule runs before or
after a message's own argument tests, the one item that changes a frozen
output; `typeHash(Action)` naming a type the law does not define; the
rule-class preamble saying every passage is marked where four state no
predicate; §5.3's reason for the `clientDataJSON` charset misstating
WebAuthn's serialisation; §5.4 telling a passkey producer to hand the
assertion on unchanged where the harness negates `s`; §7's closed
enumeration omitting the four bare element tokens its clause includes and
filing `E_ACTION_PARTY` among derivation tokens; §8 asserting a seller
low-s check the design volume did not state; §10.1 naming two
unwitnessable form halves where there are four; the harness's action
commands inconsistent about carrying `instance`, and `verify-action`'s
positions depending on a token the contract did not say is read first.

Folded (Court commit after this record):

- §4.5 decides the party rule after its argument tests and before any
  test of §5, with the two collisions spelled; `hashStruct` names
  `typeHash(t)` for the message's own type; §7 lists `E_ACTION_PARTY`
  beside the signature tokens and names the four bare element tokens
  outright.
- The rule-class preamble names the four passages that carry no mark;
  §1 says §5.3's skeleton-string row tests its own charset; §5.3 gives
  the charset's reason in WebAuthn's own members; §5.4 says the law
  admits either twin and the harness's producer commands hand on the
  low-s one; §2.3 states `expiry`'s narrowing as it states `nonce`'s and
  says `compliance_md`'s keys stay keys.
- §8 says the low-s rule is this law's test and the parent's producer
  obligation, and carries the `sigBuyer` packing sentence from §5.3;
  §10.1 names the four form halves the harness makes misuse; §10.4 pins
  the recast `galeed/1` core `0x75db…8a51`.
- The design volume: `init` and the `…BySig` entry points refuse a high-s
  secp256k1 signature from either party (§3.3); the seller signs
  `ClaimDelivery` and `Refund` (§3.10); the pointer chain equals the
  instance's (§九).
- Harness: `<name>` is read before the tokens after it take positions;
  `sign-action` and `sign-passkey-action` answer with `instance`;
  `verify-action` names the key each party is verified against.
- Corpus: 375 cases, with the three collisions of §4.5's order
  (`E_ACTION_RUNG` and `E_ACTION_MANIFEST` before the party rule,
  `E_ACTION_BPS` before any signature test).

## Round 3: the same eight seats on the round-2 text

The three implementations, updated from the revised texts alone, reached
zero divergences on all three corpora at seeds 1 to 6 on their first
measurement (5,280 case runs each) and probed beyond them (Rust 2,131
inputs, Node 8,684 with round 2's regression, Python 4,497) with no
disagreement. The readers closed every round-2 item. Every seat but one
answered that this text could be named now; the corpus auditor held all
three laws for one sentence: the precondition of §10.1 quantified over
carried tokens in two readings, one of which the tree obeyed for
`E_LINK_SIG` and the other everywhere else. The load-bearing findings on
this law, none changing a frozen output: the design volume's `init`
quote checks named four clauses where §3.2 has five; `eravon.link/1`
imported a predicate name `terms(b)` this text never gave; §1's gloss of
the prose subtree was narrower than the parent clause it invokes; §7
filed `E_ACTION_PARTY` with half of §4.5's order; §8 said the passkey
packing twice; §7's last heading covered literals in no document; §9's
closing clause had no referent before a core is named; the `[O]`
preamble's count was one behind §5.4; §6.1 took the quote from a value
that carries only its hash and fixed `priced` on one side only; the
corpus witnessed `E_GUILD_QUOTE` carrying two of §3's three tokens.

Folded (Court commit after this record):

- §10.1 ranges over `(token, carried token)` pairs where the text names
  the carried set as a finite list and over the carrier alone where it
  carries another section's or another document's whole vocabulary; §1's
  gloss names the member's own value; §2.4 names `terms(b)`; §6.1 names
  the candidate's decoded quote and fixes `priced` on both sides; §7
  gives `E_ACTION_PARTY` both halves of its order and heads its literals
  by where they live; §8 says the packing once; §9's clause names this
  text where no core is named; the `[O]` preamble counts two clauses.
- The design volume: `init`'s quote checks are §3.2's five, with the
  512-byte length; the criteria of §2.1 name the recast core and the
  interface identity, `galeed/1` §8 and §9.4, and the replay's four
  hooks for a per-case standard; §3.5 and §3.11 cite `galeed.venue/1`
  §2.3 and §3 where they promised a record entry; §3.14 item 17 names
  the panel face's contract and its read views; the native seat accepts
  `eravon://settle?s=`; the amendment is spoken of as landed throughout.
- Corpus: 376 cases, with `E_GUILD_QUOTE` carrying `E_QUOTE_FEED`.

## Round 4: the same eight seats on the round-3 text

The three implementations reached zero divergences on the 376-case corpus
at seeds 1 to 6 and no disagreement beyond it. The readers closed every
round-3 item; every seat answered that this text could be named, the
corpus auditor asking first for one case. The findings on this law, none
changing a frozen output: §1's gloss said "ends in `_md`" where the parent
says "is `_md` or ends in it"; §10.1's quantifier sentence was no
partition on this law, whose two quote carriers fit both of its arms; the
corpus list of §10.1 closed twice; `terms(b)` was given on its passing
side alone; the design volume's `init` list counted five clauses where
§3.2 has more, and its criteria list omitted the three document laws; the
corpus lacked a bare `E_GUILD_QUOTE`.

Folded (Court commit after this record):

- §1 says "is `_md` or ends in it"; §2.4 says `terms(b)` yields the
  document or the token; §10.1 names the two quote carriers and what it
  asks of them, once bare and once with each of §3's three tokens, in
  place of the general quantifier sentence.
- The design volume: `init` checks every clause of §3.2, listed; the
  criteria of §2.1 name the three document laws beside the venue law;
  the guild draw's random source names the submitter's choice of block;
  `init`'s gas is stated as measured.
- Corpus: 377 cases, with the bare `E_GUILD_QUOTE`.

## Round 5 and the naming

The three implementations reached zero divergences on the 378-case corpus
at seeds 1 to 6 with no change to their code, and probed the round-4
sentences (Rust, Node and Python each weighting the `_md` forms, the
quote carriers bare and carrying, and §6.1's `priced`) with no
disagreement. Every reader answered that the digest could be named; the
corpus auditor asked first for one case, a key that is exactly `_md`,
which the §1 fold of round 4 had added without a witness, and for the
digest scripts' guards to be format-independent and keyed on a digest
being present. Folded: the case (`closed_md_bare`), §2.4 stating
`terms(b)` total at its head, §10.1 confining its carrying-token sentence
to the tokens that carry a token and naming `E_QUOTE_FIELD`'s twelve
words, §7's literal list widened to what §1, §4.2 and §5.3 fix outside a
document, and the four digest scripts rewritten with two guards each.

Named on 2026-09-16 at Court commit after this record: the release digest
`0x3a369b263bed8f85a0e11f8431c44cbf9817ce3444be34b5ca06569f0c6502be`
over `ert.py`, `ertlaw.py`, `ertcrypto.py` and the `galeed/1` core
`0x75dbdb2c…8a51`, as §10.5 quotes it. The text's own sha256 at the
naming is `a5895466befa6ff8bffa5899ee43bd529bbcf955a514254f810a6400fd019064`. The fuzzing of §10.1's deferral was
not run; beside the digest stand the three implementations' more than
64,000 differential inputs over five rounds.

## Revision in place for consent by call (2026-09-23)

The judge ruled on 2026-09-23 that a party that is a contract consents
by calling: at `init` and at `settle` the standard admits an empty
signature slot where the party is the immediate caller or the logic's
registry (`consent(digest)`, `consented(party, digest)`) holds the digest
under the party. The three laws were revised in place, the version
literals, type strings and domains untouched, and the convergence was run
again the way the first one was: the same eight seats per round (five
readers, three implementations written from the texts alone), the
corpora regenerated at seeds 1 to 6 for every round, and the naming
withheld until the readers reported nothing open and the implementations
reached zero divergences with every closed boundary witnessed on both
sides.

### Round 0: the criterion against its own generator

§5.4 (the third form: `SIG_BY_CALL` on an empty slot for `verifyOffer`,
`verifyOrder` and the `Settle` message under either party; `E_SIG_FORM`
for the other four messages), §5.5 (the producers), §4.5, §5.1, §5.2,
§7, §8, §9 and §10.1 were written; §10.5 returned to the unnamed form.
The criterion and the generator (399 cases) agreed at seeds 1 to 4.

### Round 1: five readers and three implementations

The three implementations, written from the revised texts and the
harness contracts alone (Rust with no dependencies; Node.js with its
standard library and its own keccak, secp256k1 and P-256; a second
Python with its own primitives and no `json` module in the acceptance
path), each checked their `galeed/1` pieces against the frozen `galeed/1`
corpus (7,261 runs each, zero divergences) and reached zero divergences
on all three corpora at seeds 1 to 6 on their first measurement (399,
442 and 159 cases per seed), then probed beyond them (Rust 18,478
inputs, Node 17,710 with three outside the measuring interface, Python
20,162) with no disagreement inside the interface. The readers found 10
(A), 12 (B), 9 (C), 18 (D) and 29 (E) items and the implementations 6,
10 and 6; none changed a frozen output. Deduplicated, the load-bearing
ones on this law: five sentences said an empty slot on "the other four
messages" answers `E_SIG_FORM` where §4.5's party rule answers
`E_ACTION_PARTY` first for two of them under each party; §8 said "the
transaction's sender" where the standard compares the immediate caller,
which for a company acting through its proposal executor is never the
transaction's origin; §8 named neither the digest the registry holds for
each of the three slots nor what ends a registration; "slot" was used
six times and defined nowhere; §4.5 counted the buyer's forms as two;
§5.4 said "as before"; the §7 paragraph on `SIG_BY_CALL` covered
verifications that can never yield it and read as if the name itself
must be carried; §5.5's producer clause could be read as forbidding a
company seller's client to carry the buyer's signature into a pack; the
independence clause of §10.1 omitted `galeed/1` §1, §3 and §5 and the
`galeed/1` harness the terms harness cites; the harness's `<sig.json>`
bullet made every empty root the third form and listed `call` under
"Accepted"; the harness said nothing of an `<origin>` whose bytes are
not UTF-8, which the criterion refuses as misuse.

Folded (Court commit after this record):

- §5.4 defines the slot and names the two laws that carry one, states
  `0x`'s refusal by the test that makes it, and admits the empty string
  for the other four messages under the party §4.5 admits for each, the
  party rule answering first under the other; §5.1 and §5.2 say which
  two messages reach `E_SIG_FORM` under each party and confine "nothing
  else is" to the values the section decides; §4.5 names the forms by
  their sections; §5.5 binds the producer's obligation to the slot.
- §7 lists the three verifications that have the outcome and says a law
  carries it under a name of its own; the signature tokens cite §5.4.
- §8 states the admission per slot (the seller's at `init` where the
  seller calls `init` or the registry holds `offerDigest` under the
  logic's domain; the buyer's likewise with `orderDigest` under the
  instance's; either party's at `settle` with the `Settle` digest), what
  a caller is, that the registry is the logic's own and never withdrawn,
  and that each digest's own expiry ends a registration.
- §10.1 names the sources an implementation is produced from and the
  party under which each empty-slot witness stands; the preamble and
  §8's ABI bullet speak of consents and slots.
- Harness: `<sig.json>`'s empty root is decided by §5.4 per command and
  message; `verify-order` and `verify-action` keep the §5.4 answer apart
  from acceptance; `<origin>` must be valid UTF-8.
- Corpus: 402 cases, with the cap before the third form under
  `verify-offer`, the payout's fault before it under `verify-order`, and
  `0x` on `Settle` under the buyer; five notes cite §5.5.

### Round 2: five readers and three implementations

The three implementations updated their round-1 programs from the
revised texts and harness contracts alone (the one measured change being
`erl check`'s `byCall` shape) and reached zero divergences on all three
corpora at seeds 1 to 6 on their first measurement (402, 446 and 162
cases per seed), then probed beyond them: Rust 68,191 inputs, with the
differential fuzzing the naming note of §10.1 asks for (the §2.2 byte
scan 10,367 inputs, `hexbytes` 10,631, `dec256` 17,966, at seeds 1 to 3)
and the link law's §2.7 prefix and base64url scan (6,000); Node 23,352,
of which 23,343 inside the measuring interface; Python 25,564. No
disagreement inside the interface. The readers found 6 (A), 3 (B), 9
(C), 6 (D) and 19 (E) items and the implementations 3, 6 and 4; none
changed a frozen output, and each was verified against the text before
folding. On this law: §8 defined the caller by the party's nature and
said the caller is "never a batching contract", where Multicall3's
forwarding makes Multicall3 the caller (the venue document says so), and
its companion in `eravon.dossier/1` excluded the transaction's origin
without limit, which rules out a key-holding party's own top-level
`init`; §9 kept "by name"; §10.1 listed `galeed/1` §1, §3 and §5 by
number where the text invokes other passages too; `sellerKey` was called
a signing key in three places after §5.4 admitted a seller that signs
nothing; §5.4 said "this section" for a slot the standard also takes;
the harness's `<sig.json>` bullet opened as if every root were decided
by §5.2 and §5.3. On the corpus: the passkey challenge's base64url
carried `-` at some seeds only and `_` at others, with no
standard-alphabet refusal, so an implementation writing `+` for index 62
passed the committed seed; the `<origin>` verbatim rule was exercised by
an ASCII URL alone; the quote cases carried no file, so the README's
slugs named nothing in the manifest; the conformance README named the
superseded digests. On the criterion, outside the interface:
`sign-passkey-action` exited 1 on a non-UTF-8 `<origin>` where
`sign-passkey` exits 2.

Folded (Court commit after this record):

- §8 defines the caller as the address the instance sees (`msg.sender`):
  the party's own account whether the call is a transaction's top-level
  call or an inner call of a transaction someone else sent, never an
  intermediary that calls on the party's behalf, which the instance then
  sees as the caller, and never the origin of a transaction whose call
  someone else made; §9 carries §5.4's answer under names of their own;
  §10.1 names the passages of `galeed/1` the text invokes; §2.3, §5.5
  and the preamble speak of a seller that consents by call; §5.4 names
  the standard's `bytes` argument as the slot's other home.
- Harness: the `<sig.json>` bullet decides the root per command and
  party, the empty string by §5.4 on either command.
- Criterion: `sign-passkey-action` refuses a non-UTF-8 `<origin>` as
  misuse, as `sign-passkey` does.
- Corpus: 406 cases; at every seed the generator takes the smallest
  `buyerPayout` whose order digest encodes with both `-` and `_`, and
  witnesses that challenge accepted, its standard-alphabet twin refused,
  and the producer writing it; an `<origin>` with a quote, a backslash,
  a tab and a non-ASCII character placed verbatim; every quote case's
  note and the cap-first case's note begin with the slug the README
  cites; the conformance README speaks of unnamed digests.

### Round 3: five readers and three implementations

The three implementations, updated from the texts alone, changed no rule
(Node moved two arguments' raw bytes through its wrapper, `<origin>` and
`<password>`) and reached zero divergences on all three corpora at
seeds 1 to 6 on their first measurement (406, 448 and 162 cases per
seed), then probed beyond them: Rust 72,478 inputs with the naming
note's fuzzing rerun at seeds 1 to 3 (the §2.2 byte scan 10,367,
`hexbytes` 10,631, `dec256` 17,966, the link law's transport scans
6,000); Node 25,779; Python 31,188, with two mutants (a
standard-alphabet decoder on the link law's three other transports
passing every seed, a standard-alphabet action challenge passing seed
5). No disagreement inside the measuring interface, and no place where
the criterion departs from the text. The readers found 2 (A), 5 (B), 6
(C), 5 (D) and 15 (E) items and the implementations 5, 4 and 6; none
changed a frozen output. On this law: the round-2 caller clause of §8
said the caller is "never an intermediary" in the same breath as saying
the instance sees the intermediary as the caller, listed the call's
positions as if complete, and re-read a definition written for the
instance onto `consent`; §5.5's new sentence and a clause of §5.4 used
"a party that consents by call" as a kind of party, denying a key
holder that sends one call itself its key, the "holds no money" rule
and its signed acts, against §5.4's per-slot rule; the harness's
`<authData-hex>` bullet named a length test the producers do not run.
On the corpus: the action challenge under `verify-action` carried both
URL characters at some seeds only and none at seed 5; two of RFC
4648's four decoder leniencies on the challenge (nonzero trailing bits,
a character outside the alphabet) were unwitnessed, so a
decode-and-compare verifier passed every seed; the §2.4 decision order
was witnessed by hand-picked pairs, so a checker deciding every
top-level member before the element tables, or a table's rows in
another order, passed every seed; two §5.3 adjacencies the README
counted were absent.

Folded (Court commit after this record):

- §8 defines the caller as the address the called contract sees, the
  instance for `init` and `settle` and the logic for `consent`: the
  party's own account where that account makes the call, top-level or
  inner; the intermediary, and not the party, where one makes the call
  on the party's behalf; never the origin of a transaction whose call
  someone else made. §5.5 and §5.4 speak of a party that is a contract
  where they speak of what a party has or does, and of a slot filled by
  call where they speak of one consent; §2.3's row likewise.
- Harness: `<authData-hex>`'s length test is `quote`'s; the two
  producers sign any authenticator data.
- Corpus: 430 cases; the generator takes the smallest `buyerPayout`
  whose `Dispute` digest under the order's instance encodes with both
  URL characters (`action_passkey_url_alphabet`,
  `action_passkey_std_alphabet`); the challenge's two remaining
  leniencies as re-signed twins (`passkey_challenge_trailing_bits`,
  `passkey_challenge_non_alphabet`); the §2.4 stage bridges (an
  element-table fault beside the ladder, the ladder-wide rules beside
  judges, each repeat beside the next member, the guilds form beside
  nonce) and every element table's adjacent rows; the two §5.3 pairs;
  the producer over 36 bytes of authenticator data
  (`sign_passkey_short_auth`).

### Round 4: five readers and three implementations

The three implementations, changed in no rule, reached zero divergences
on all three corpora at seeds 1 to 6 on their first measurement (430,
448 and 168 cases per seed), then probed beyond them with the decision
orders as the subject: Rust 94,174 inputs (16,760 two-fault inputs over
every ordered clause, 1,849 of them one step outside the corpus's
adjacent pairs; the naming note's fuzzing rerun at seeds 1 to 3), Node
43,111 (14,848 two-fault inputs; 101 swap mutants, of which the corpus
separated 59, 8 were exclusive and 34 passed every seed), Python
63,755 (324 mutants, the round-3 two now caught at every seed, three strip-before-decode mutants on the other transports caught by the newline cases). No disagreement inside the measuring interface, and no
place where the criterion departs from the text. The readers found 2
(A), 7 (B), 6 (C), 8 (D) and 7 (E) items and the implementations 0, 1
and 6; none changed a frozen output. On this law: §8's
second member named the intermediary by kind, so a library a party
reaches by `DELEGATECALL` or an EIP-7702 delegation's code read as an
intermediary though the call leaves from the party's own address, and
the third member had lost its object; the venue document §9 and the
design §3.3 still spoke of "a party that consents by call" as a kind of
party. On the corpus: §2.4's UTF-8 test was called exclusive with the
fence tests, which a byte string with no fence refutes; the fence
count was witnessed beside no unclosed block, and `d023` was a no-op
for the pair its note named; the element tables' prefixes (missing,
closed) beside their first rows, the token row beside `source`, the
ladder count beside a faulty rung, the ladder-wide rules among
themselves, and two faulting elements were unwitnessed; §3.2's word
chain and §5.1's signature order were witnessed one fault at a time;
the `<authData-hex>` producers were witnessed at one point.

Folded (Court commit after this record):

- §8 defines both members by the address the call leaves from: the
  party's own account, whatever code it runs to make the call (its own,
  a library by `DELEGATECALL`, an EIP-7702 delegation's); the
  intermediary where the call leaves from its own address; never the
  origin of a transaction in which someone else made that call. The
  venue document §9 and the design §3.3 speak of a party that is a
  contract, a key holder signing or calling those four acts whatever
  form its slots took.
- Corpus: 461 cases; UTF-8 before each fence test, the fence count
  before an unclosed block, `two_openings_one_block` renamed to the pair
  it witnesses; each table's closed member before its first row and
  missing before closed, the token row before `source`, the ladder
  count before any rung and before the seventeenth, the last rung's
  rule before the order rule, two faulting elements in two tables; the
  §3.2 word chain and §5.1's order, adjacent pairs; the producers over
  uppercase transport hex, a flags byte without the presence bit, and
  empty authenticator data under the action producer; canonical before
  the root on a block that is neither; the offer's token before
  `E_FEE_OVERFLOW`; the order's token before §4.5's argument tests under
  `action` and `verify-action`; an argument beyond a variable-length
  command's list as misuse before the document's fault; the README's
  exclusion list rewritten to what holds.

### Round 5 and the naming

The three implementations, changed in no line, reached zero divergences
on all three corpora at seeds 1 to 6 on their first measurement (461,
566 and 174 cases per seed), then probed beyond them: Rust 110,846
inputs (22,539 with answers computed from the text, all agreeing; the
naming note's fuzzing rerun at seeds 1 to 3, the §2.2 byte scan 10,367,
`hexbytes` 10,631 with 438 transport hex beside, `dec256` 17,966, the
link law's four transports 6,000 and `hexdigits` 5,069), Node 44,611
(221 swap mutants, 192 separated by the corpus at every seed, 8
exclusive, 21 unseparated), Python 28,952 new beside the 63,755 of round 4 (326 mutants, every round-4 gap caught at every seed). No disagreement inside
the measuring interface, no place where the criterion departs from the
text, and no finding against any text's predicates. The terms reader
and the Rust seat reported nothing open; the ecosystem reader reported
nothing open on the terms and link laws. What remained was prose
outside the predicates and corpus witnesses: on the terms law, none in
the text (the venue document §9 and the design §3.2, §3.3 and §3.15
said "direct(ly)" for the party's own call where both define a direct
call as `msg.sender == tx.origin` with no code); on the link law, §2.5
joined the rung stage and the ladder-wide rules with "and"; on the
dossier law, five [C] §5.1 sentences (the kept call admitted frames that
did not run at the instance; the calls witnessing `advance`, `setPayout`,
`withdraw` and `deploy` were not defined as kept calls; the no-trace
rule omitted `deploy` and applied to failed transactions; `redraw`'s
"next ordinal" also read `init`'s seat 0); and on the corpora, the
adjacent pairs the fold of round 4 had not reached (§3.2's word chain in
full, the element loops of every array, the first of two descents and of
two repeats, every element's table before the repeat test, `unlock`'s
three stages, the pipeline seams of HARNESS-TERMS under every command,
the dossier's key order and the entry file before the documents, misuse
before the law's fault on every variable-length command), which the
round-4 program of every seat passed with the pair swapped. All of it
was folded in one pass, as the round of 2026-09-16 folded its residue:
the sentences above, the READMEs narrowed to what their cases carry,
and 508, 585 and 179 cases at every seed, the
criterion agreeing with every prediction at seeds 1 to 6.

Named on 2026-09-24 at Court commit after this record: the release digest
`0xd01dda51c513a20ac1d5b14051f1e8561d76e67646c3cb957c0166e7acbb9af5`
over `ert.py`, `ertlaw.py`, `ertcrypto.py` and the `galeed/1` core, as §10.5 quotes it. The text's own sha256 at the
naming is `45645090355682d5343ab70d4a05b0cea82a37559cc09ffdabe5291c4b95d800`. The fuzzing of the deferral was run at the
naming, as the digest's note records.

Withdrawn the same day. The judge ruled on 2026-09-24 that an unreleased
standard carries no internal generation number: the EIP-712 domain version
became "1" (`eravon.terms/1` §4.2 and `ertcrypto.py`), the three CREATE2
salts became `eravon/1`, `eravon/1/verify` and `eravon/1/panel`, and the
venue document's title became "ERAVON standard v1". The three cores changed
through the one constant, so the digests above no longer name any core;
the three texts returned to the unnamed form, the EIP-712 vectors and the
corpora (508, 585 and 179 cases) were regenerated under the new domain, and
a sixth round was run before naming again.

Round 6 (2026-09-24), the confirmation of the generation change: five
readers (this law, the link law, the dossier law, the corpus and harness,
the ecosystem) and three implementations (Rust, Node, Python), each
implementation the round-5 program unchanged on its first measurement over
the regenerated corpora at seeds 1 to 6. The first measurement diverged on
53 terms cases, 171 link cases and 67 dossier cases per seed, every one an
answer downstream of a domain digest (the digests themselves, the
signatures over them, the verifiers' verdicts, and the payload faults the
two laws above report when a signature made under the new domain is read
under the old); after each program changed its one domain constant, all
three reached zero divergence and zero difference from prediction at every
seed, 18 runs of 18. Beyond the corpus: Rust's 8,642 domain probes (6,874
of them recomputed from the text alone) and 1,122 old-domain replays, all
refused as §5 says; Node's 12,832 probes over both domains, every carrier,
and the withdrawn version; Python's 7,588 domain probes and the 28,952
probes of round 5 rerun under the new domain; zero disagreement with the
criterion in every set. Rust's differential fuzzing of the naming note at
seeds 1 to 3 (50,471 inputs) found none. The swap audits: Node separates
213 of 221 mutants at every seed, the 21 round 5 left unwitnessed now all
witnessed, 8 structurally exclusive; Python catches 234 of 326, no
regression, the three round-5 findings now caught. No criterion finding.
Prose: §7's registry of fixed literals still copied the domain version
(reader A's inversion taken: it now names §4.2's domain instead of a
value); a trailing newline lost by the withdrawal edit restored. Outside
this law: the conformance README and the design volume's §2.1 asserted the
withdrawn namings; the design's succession step hard-coded the next salt;
`identity.sh` accepted a value from any table in CODEHASH.md (scoped to the
current table); two dispatch documents copied standard constants (now
pointers to CODEHASH.md's current table); the design volume took ruling 45.
None of it changes a measured answer.

Named on 2026-09-24 at Court commit after this record: the release digest
`0x35044ebe574f160cc50c2056bf00086c2d933b5aeebba80ce5f72e665b16534e`
over `ert.py`, `ertlaw.py`, `ertcrypto.py` and the `galeed/1` core, as
§10.5 quotes it. The text's own sha256 at the naming is
`aa06a844066cb38e9ca7d09aebd3d0d82bc77f411d200c1712d7a022930989c8`. The fuzzing of the deferral was run at the
naming, as the digest's note records.


## Revision in place of 2026-10-17: the document carries the whole order, the deposit is an allowance, the link is a pointer

The judge's directions of 2026-10-17 (the revision plan of that date, sections 一 to 六): a terms document is one order, one case, one instance, and carries the amount, the two windows, the open window and whether the buyer may open, in place of a milestones array and a licence grant; a candidate names its quote by the keccak256 of the quote bytes a client reads on chain (`quoteHash`), the quote bytes themselves leaving the document; a judge's `amanahHash` is the `entry_id` of its covenant entry and a guild's the sha256 of its covenant document's text; the goods and the delivery sections are each prose or a sha256 of prose, exactly one; the order carries `openBy`, the buyer's absolute deadline for opening, so both EIP-712 type strings change; the dispute names no rung and `Cancel()` is the sixth action message, the buyer's alone, for the refund of a funded address before any case; `lapseDefault` is 0 or 2; the fee is read from quote bytes and an amount alone. The `galeed/1` core embedded by §10.4 is the one named on 2026-10-16 (`0x58b621c3…4a0d`). The design volume is sealed and every reference to it points at this law's own sentences or at `eravon.standard/1`. The two sibling laws and the standard were revised by the same act; the standard's contracts were redone in full, and the naming of the three cores waits for the contracts' acceptance gate as well as for this convergence, since the type strings and the verifier are shared.

### Round 0: the criterion against its own generator

§1 (`dec256`, the pairs), §2.2 (fences and the block alone outside), §2.3 (the sixteen required members, the two pairs, `compliance_md`), §2.4 (`E_TERMS_OUTSIDE`, `E_TERMS_AMOUNT`, the two window tokens, `E_TERMS_OPENWINDOW`, `E_TERMS_BUYEROPEN`, `E_TERMS_GOODS` and `E_TERMS_DELIVERY` as the pair tokens), §2.5 (`quoteHash` on both candidate tables, the milestone table gone), §2.6, §3.3 (two values), §4.1 and §4.2 (the two type strings), §4.3 (`offer(T, c)`), §4.4 (`order(…, openBy)`, `E_ORDER_OPENBY`), §4.5 (six messages), §5.1 to §5.5 (the seller key as the client's hot key), §6, §7, §8, §9, §10.1, §10.3 and §10.4 were written; §10.5 returned to the unnamed form. The criterion (`ert.py`, `ertlaw.py`, `ertcrypto.py` over the new core) and the generator (507 cases) agreed at seeds 1 to 6.


### Round 1: five readers, three implementers

Five readers (A this law's text, B the standard against the contracts, C `eravon.link/1`, D `eravon.dossier/1`, E the four laws together with the plan, the tooling and MIKNAH's citations) and three implementers (Rust, Node, Python, each written from the texts and the harnesses alone, `ert`, `erl` and `erd` in one program each) read the round-0 texts. The implementers measured zero divergences from the criterion and zero differences from prediction at seeds 1 to 6 on their first measurement of each law, 18 runs of 18 for each, and probed the criterion beyond the corpus (Rust 27,920 terms, 18,871 link and 7,251 dossier inputs; Node 21,772, 21,293 and 12,203; Python 21,581, 39,764 and 4,106) with no disagreement class; one rendering class appeared in Rust's first link run alone, carried faults written with an `ok` member, which the harnesses then fixed (a fault or a verdict under `inner` carries no `ok`, at every depth). Four audit seats read the contracts in the same round (security, the eight vulnerability classes, the adversarial targets, the fuzz and invariant campaign); their findings changed the contracts and the standard's text and touched this law where noted below.

Reader A found 20 items (8 load-bearing), B 37 (17) with 9 security notes, C 20 (10), D 21 (13), E 32 (6). Folded into this law: the Status paragraph (a core that agrees with the named one on every input); §1 the skeleton-row note; §2.3 and §2.4 the amount bound ⌊(2^256 − 1)/10000⌋ under `E_TERMS_AMOUNT`, `expiry` read by §5.5 too, one set of windows; §2.5 the key `<chainId>:<codeHash>:<token>` named and cited from `E_GUILD_QUOTE`, a judge's source address asked nothing, a guild pointer equal to the seller's key refused; §2.6 the expiry's readers; §3.3 the citation for a quote of the document's own token; §4.1 `openBy` an int and `buyerKey` neither the seller's key, the judge nor the guild; §4.4 `E_ORDER_BUYER` for a buyer that is the guild; §4.5 the composition sentence (an action is read over an order, §2.4, §4.3 and §4.4 deciding first); §5.3 bit 0 as the value 0x01 and the contract running every test of that section in its own code, the charset of `clientDataJSON` included; §5.5 this key's own reasons, the producer's rules, an action signature valid in every later state that admits the action, the deterministic nonce of the harness; §6.1 the overflow rule over every product and sum of the section, `feeFixed` + ⌊A × feeBps / 10000⌋ before the bounds and P + `lf` after them, the first in the order written being `E_FEE_OVERFLOW`; §8 a guild's word as the sha256 of its covenant's `text_md`, the key of §2.5, `openBy` ending a registered order; §10.1 the classes of rules with no reachable failing side described in place of a list; §10.3 the version read from the fence line, the sentence on the standard's table deleted; §10.4 each law moving to another core only by its own naming act, which re-pins what it embeds; §10.5 "computes". HARNESS-TERMS: the path of `galeed-conformance/HARNESS.md`, the element index in place of a candidate index, the prefix rule scoped to this law's tokens, nested `inner` without `ok`, `sign-passkey`'s RFC 6979 input as the 32-byte hash hashed no further and the low-s twin. The criterion followed: the amount bound, the guild pointer against the seller's key, the buyer as the guild, `E_FEE_OVERFLOW` on P + `lf`. The corpus grew from 507 to 524 cases (the amount at and over the bound, a string rung bearer, a short guild word, a guild pointer that is the seller's key, the fee sum at and over 2^256 − 1, recovery to the point at infinity on both curves, passkey `r` zero and `s` at n_p, hex32 forms of `x`, `y` and `s`, uppercase authenticator data, `cdj` with U+007F, a point with `y` at or past p, the buyer as the guild); zero differences from prediction at seeds 1 to 6. Withdrawn at the fold: A's `SIG_BY_CALL` spelling, zero source and non-UTF-8 `sig` (withdrawn by the seat itself), and E's rename of `bondToBuyer` (plan item 16 keeps the `Closed` fields; the standard glosses the name). The standard's own fold of the same round (the two-step guild draw, the seller's surrender during a dispute, the amount and int bounds as contract guards, every pricing overflow named, the passkey charset test in code) is recorded in the standard's text and CODEHASH.md.


### Round 2: the same seats over the revised texts

The same eight law seats re-read the round-1 texts (Court 1e643b19): readers A 13 items (3 load-bearing), B 29 (15) with 6 security observations, C 19 (8), D 19 (9), E 19 (5, the plan's 37 items all in the texts, round-1 items 31 of 32 landed and one withdrawn, every tooling check passing, a dry run of the naming act passing all three digest scripts' guards), and the three implementers, each continuing its round-1 program, zero divergences and zero differences from prediction at seeds 1 to 6 on the first measurement after bringing the programs to the revised sentences (the round-1 programs diverged on the revised cases alone), with 58,937 (Rust), 59,674 (Node) and 71,757 (Python) further probe inputs and no disagreement class. The audit seats of round 2 (security, the eight classes, the adversarial targets) found no finding above medium; their findings (the re-arm as a re-roll by delay, a surrender cheaper than a lower rung, a settlement past T2 stripping the judge's P, a draw window shorter than a block, the arming block's producer, uncapped pushes and feed reads, a codeless token at the gate, a Refund() signed earlier landing as the surrender) changed the contracts and the standard and are recorded in the standard's CODEHASH.md paragraphs; the fuzz seat's own finding on the round-1 fold, a double credit at the expiry branch and cancel, was fixed before the round's audits began.

Folded into this law: §2.3 the `logic` row read by the quote key too, the `amount` row's reason (its product with every basis-point value fits a `uint256`), the `confirmWindow` row (the buyer's time to dispute, a `confirm` taken until `advance`), the `expiry` row citing the standard's §4 step 3; §3.3 `offerWindow` as the guild track's draw deadline too; §4.1 `nonce` and `expiry` as ints bounded as §2.3 bounds them, and the nonzero one of `judge` and `guild` byte-equal to no `sellerKey`; §4.5 `Settle`'s `expiry` at most 2^53 − 1 as every int; §5.3 the contract running the form, key, flag, type and challenge tests in its own code and the point, range and verification tests through the precompile, in an order of its own; §5.5 every message of §4.5 valid in every later state that admits it, `Settle` until its `expiry`, and a `Refund()` admitted by its signed form outside a dispute alone, the surrender being the seller's own call; §6.1 a quote whose nonzero `feeMax` is below its nonzero `feeMin`, the `E_FEE_OVERFLOW` clause inside the zero-unit branch (a nonzero unit forms no sum and nothing faults), and the products and quotients as exact integers for every uint256 `amount`, no product a fault, under §2.3's bound each within a `uint256`; §9 the citation's reach (whatever part of this text a standard differs from); the Status paragraph (the implementation agrees with the core named in §10.5, its embedded `galeed/1` core with the one named in §10.4). HARNESS-TERMS: `<privkey>` in [1, n − 1] for secp256k1 and [1, n_p − 1] for P-256; `ok` at the top of a fault answer and of an accepted answer, never in a carried fault or verdict, an accepted answer carried under `document` written as its command writes it. The corpus grew from 524 to 540 cases (`payout_upper`; `unit_nonzero_no_sum`; an unreadable path on the ten commands that read one and had no such case; an argument beyond the list on `fee`, `sign-offer`, `sign-order` and `sign-passkey-action`); zero differences from prediction at seeds 1 to 6. The README's §4.4 row names the guild, its §6 table carries the P + `lf` boundary and the nonzero-unit case, its misuse paragraph says what is witnessed, and a closing paragraph says which clauses of §1 state no boundary and where `runtime` and `proxyCodeHash` are measured. The standard's guard table now states that its order is its own and pairs no guard with this law's token.


### Round 3: the same seats over the round-2 texts

The same eight law seats re-read the round-2 texts (Court c25210b0; HEAD moved to 74b81962 during the round, a commit touching one test file alone): readers A 7 items (0 load-bearing), B 7 (4) with 2 security observations, C 12 (3), D 16 (8), E 10 (1; the plan's 37 items all in the texts or withdrawn by the plan, round-2 items 19 of 19 landed, every tooling check passing but naming.py's terms Status anchor and the foundry.toml comment, a dry run of the naming act passing all three digest scripts' guards), and the three implementers, each continuing its program, zero divergences and zero differences from prediction at seeds 1 to 6 on the first measurement after bringing the programs to round 2's two seat rules, with 78,476 (Rust), 74,293 (Node) and 88,419 (Python) further probe inputs and no disagreement class on the measuring interface. The audit seats of round 3 (security, the eight classes, the adversarial targets, the fuzz seat) found two findings of low severity, a seller's `refund` sent while Claimed and landing behind the buyer's `dispute` executing as the surrender, and a `refund` while Claimed under a seated seat returning P to the seller so that a refund and a payment off the instance completed the trade with P stripped, and informational items beside them; the findings changed the contracts and the standard (`refund` refused while Disputed at or before T3's end, the surrender row gone, `refundBySig` testing the signature before the phase, a refund while Claimed under a seated seat paying P to the judge as T2's lapse would) and are recorded in the standard's CODEHASH.md paragraphs together with reader B's two observations on return data copied beyond a gas allowance, now copied to a fixed size by the verifier's reads and not at all by a push.

Folded into this law: §2.3 the `confirmWindow` row (a `confirm` taken while the case is Claimed, past T2 included); §3.3 `offerWindow` as the deadline of each draw after the block that follows its arming, a client offering a guild's quote only where it reaches 256 of the chain's block intervals; §4.1 "not byte-equal to `sellerKey`"; §5.5 a `Refund()` admitted by its signed form outside a dispute alone and by the seller's own call outside a dispute or past T3's end. HARNESS-TERMS: `<privkey>` and `<p256-privkey>` both named with their ranges; `ok` at the top of every answer that is not a verdict, the §5.4 answers of the `verify-*` commands included, `lock`'s key object carrying none. The corpus grew from 540 to 555 cases (`ok_source_zero_address`, `ok_source_seller_address` and `judge_source_upper`, the two pointer rows differing; `overflow_before_max` and `sum_after_min`, the two order clauses of §6.1; `Settle`'s `expiry` at 2^53 − 1, at 2^53 and with a leading zero; the keys per curve, `n_p` on `sign-offer`, n − 1 on `sign-order`, `n_p` − 1 on `sign-passkey` and `n_p` on `sign-passkey-action`; three misuse cases on a document that fails §2.4, the misuse answering first); zero differences from prediction at seeds 1 to 6; the README's rows follow. The standard's §3 pairs `FeeOverflow` with `E_FEE_OVERFLOW` on the zero-unit branch alone, and its §4 guard table covers every tuple of §4.1's shape, whichever law built it.


### Round 4: the same seats over the round-3 texts

The same eight law seats re-read the round-3 texts (Court ecf085ed): readers A 7 items (1 load-bearing), B 9 (1) with 1 security observation, C 7 (2), D 9 (3), E 8 (1; the plan's 37 items all in the texts or withdrawn by the plan, round-3 items 10 of 10 landed, every tooling check passing but an indentation error the round-3 fold had put into naming.py), and the three implementers, each continuing its program, zero divergences and zero differences from prediction at seeds 1 to 6 on the first measurement (the round-3 programs already met every revised sentence; Node read the previous path sentence to the letter, a `/` always, and found the one case where that and the criterion's joining rule decide apart), with 88,343 (Rust), 83,648 (Node) and 34,326 (Python) further probe inputs; one disagreement class, Node's, on a `<dir>` ending in `/` at the host's path-length limit, which the harness and the law now decide the criterion's way. The audit seats of round 4 (security, the eight classes, the adversarial targets, the fuzz seat) found no finding above informational, their informational items being one sentence of the standard's §5 that still described the round-3 refund, a blank line splitting §8's terminal table, and two stale rows of the design volume; round 4 is the first of the two clean rounds the judge's ruling of 2026-10-17 asks of the contracts, and the code and its identity are unchanged by this fold.

Folded into this law: §2.3 the `chainId` row read by the quote key of §2.5, and §2.5 naming `chainId` as the document's among the key's three segments; §3.3 the guild-track parenthetical as a meaning (each draw bounded by its arming time plus `offerWindow`, the block after the arming drawing whatever that bound), the client's two checks moved to §8's first bullet (the quote's `offerWindow` reaching 256 times the chain's nominal block interval, the chain's block hashes coming from consensus); §5.5 T3 named as the `rulingWindow` that opens at the dispute. HARNESS-TERMS: `ok` at the top of every answer but a verdict answer and `lock`'s key object, `unlock`'s rendering of `UNLOCKED` among the accepted answers. The corpus grew from 555 to 570 cases (`v_zero`, `v_one`, `v_26`, `r_n`, `r_zero_s_in_range`, `s_half`, `s_half_plus_one`, the boundaries of `galeed/1` §5.4 as §5.1 runs them; `passkey_form_auth_no_prefix`, `passkey_auth_empty`, `passkey_cdj_tilde_ok`, the edges of `hexbytes` and of the charset; `privkey_0X_prefix`, `privkey_mixed_case`, `privkey_non_hex_digit`, the key spellings; `n_p` + 1 on both passkey commands); zero differences from prediction at seeds 1 to 6; the README's rows follow. The standard's §3 names the offline counterpart of P + `lf`'s overflow and the nominal block interval, its §4 states the guard table's two claims as two sentences, its §5 states each form of `refund` and what `refund` and `settle` read the windows for, and its §8 gives `feeBearer` 2's rounding.


### Round 5: the same seats over the round-4 texts

The same eight law seats re-read the round-4 texts (Court ebcd2eb5): readers A 5 items (0 load-bearing), B 6 (1) with 1 security observation, C 3 (1, in the corpus alone), D 11 (2, one in the corpus alone), E 2 (1; the plan's 37 items all in the texts or withdrawn by the plan, every round-4 item landed, every tooling check passing), and the three implementers, each continuing its program, zero divergences and zero differences from prediction at seeds 1 to 6 on the first measurement (Rust 5 text items, Node 3, Python 3; Node's one round-4 class is gone, the text now taking the criterion's join rule), with 107,762 (Rust), 91,329 (Node) and 24,050 (Python) further probe inputs and no disagreement class. This is the closing round of the law texts, as the rounds of 2026-09-24, 2026-09-23 and 2026-10-14 closed: the three programs changed in no line and reached zero divergences on their first measurement, and what the readers reported was folded in one pass (three sentences the chain reader of the dossier law decides, one sentence of the standard's §8 on a token outside its rule, two corpus witnesses the programs already passed, and prose), the judge ruling on 2026-10-17 that the earlier standard governs; the three round-5 programs, unchanged, were then measured again over the folded corpora (571, 647 and 293 cases) at seeds 1 to 6, 54 runs of 54 with zero divergences and zero differences from prediction. The audit seats of round 5 (security, the eight classes, the adversarial targets, the fuzz and invariant campaign with a 55-invariant handler, a 2,575-run matrix of every pair of acts in both orders and two deep campaigns) found no finding above informational, their informational items being text and tooling (a starved or failing balance read after a push, the landing order of two acts that reach one instance, two client-rule races, the replay harness's step 9a against the guild track's second-block draw, and the design volume's rows of the earlier money model), all folded; round 5 is the second of the two clean rounds the judge's ruling of 2026-10-17 asks of the contracts, which closes the contracts' gate, the code and its identity unchanged since the round-3 fold.

Folded into this law: §2.5 `feeBearer` 2's gloss naming the odd unit of an odd fee as the buyer's (`eravon.standard/1` §8); §3.3 the guild-track parenthetical saying a draw sent in the block after the arming is admitted whatever the bound (the standard's §3 saying the same); §5.3 naming `Q` as the point `(x, y)`; §8 the first bullet's nominal block interval glossed as the chain's slot time where it has one, and the fee-on-transfer sentence saying such a token opens no case where P + `lf` is nonzero and opens one, each payee bearing the fee, where that sum is zero (the standard's §7a saying the same). The corpus grew from 570 to 571 cases (`s_zero_r_in_range`); the README adds the §1 `zero` row and the new case; zero differences from prediction at seeds 1 to 6. The standard changed in wording alone, the code not at all: §3 the block after the arming; §4 the guard table's second claim (guards 1 to 16 passed, guard 20 reading the panel argument no law builds) and the clocks a buyer waits for after a lost `cancel` race; §6 a resignation against `claimDelivery` while Funded; §8 a settlement token's `balanceOf` answering with one word, a balance read that runs out answering zero so that a push whose second read runs out is debited in full, the debit rule where both reads answer and the token outside it, and a settlement against an acceptance while no seat has accepted; §10 the actor test on an instance holding no case; §12 why the replay's guild track runs on one member. The design volume's §三 money and action rows and its §五 deposit rows now state the standard's model (the deposit pulled at `init`, P out of the deposit, no bond and no dues).

Named on 2026-10-17 at Court commit after this record: the release digest
`0x1b94150b6c5726c6f16992c9dd4a57279e5fac51be148ccc62466013afb95a52`
over `ert.py`, `ertlaw.py`, `ertcrypto.py` and the `galeed/1` core of §10.4
(`0x58b621c37880c60ff88380c6a90c16f6eeb18a24fc31949bd31796ee770e4a0d`, the chain-as-ledger revision's), as §10.5 quotes it. The text's own sha256 at the naming is
`d39125eca03e18dd1ee71b05c7768bb89aaad4ce1b8aa8b353235fd40e0a8f7f`. The fuzzing of the deferral was run across the five rounds of this revision, as the digest's note records. `eravon.link/1` and `eravon.dossier/1`, whose cores embed this one, were named by the same act (their records).

## Revision in place of 2026-10-18: the seller's deposit address

The judge's direction of 2026-10-18 (the plan `修法方案-公司卖家押金地址sellerFund-2026-10-18`): a seller that is a company keeps its capital in its wallet and holds nothing at its own address, so a deposit pulled from `sellerKey` could never be found there and no company seller could open a case; the order now names `sellerFund`, the address the deposit is pulled from and returned to, which is the seller's key or an address that bound itself to the seller on the logic (`acceptFunding`), the bond tested at the gate after the consents whether or not the deposit is zero; and a buyer that consents by call counts `openWindow` from its export plus the earliest landing of its consent. The three cores were recast (`Offer` gained `sellerFund` after `sellerPayout`, so every offer and order digest changed), the naming sections of the three laws returned to their unnamed form, the standard changed with the contracts (three registries, `pull` over the fund, five parties, `FundUnbound`, the terminal split computed on the verifier for room: logic 24,131 bytes, verifier 9,450; a new identity table), and the three laws are named again after the rounds below.

### Round 0: the criterion against its own generator

§2.3 the `sellerFund` row; §2.4 seventeen required members and twenty-two keys, `E_TERMS_FUND` after `E_TERMS_PAYOUT`; §4.1 six addresses, `sellerFund` among the nonzero; §4.2 both type strings and `hashStruct(offer)`; §4.3 the member drawn from the document; §5.5 the key holding no deposit, the by-call buyer's `openBy`; §7 the token. HARNESS-TERMS: seventeen members of §4.1. The corpus grew from 571 to 578 cases (`fund_zero`, `fund_upper`, `fund_short`, `missing_fund`, `ok_fund_other`, `order_payout_before_fund`, `order_fund_before_token`), zero differences from prediction at seeds 1 to 6; the README's rows follow.


### Round 1: the twelve seats over the checkpoint

The twelve seats read the checkpoint (Court 849ad3cb): readers A 8 items (0 load-bearing), B 13 (1), C 4 (2, both in the corpus alone), D 8 (1), E 17 (2, the same two as B's and D's; of the plan's 41 rows 37 in the texts, 1 done otherwise, 1 pending the renaming, 2 the FUZZ seat supplies), and the three implementers, each brought to the revised type string (Rust 28 lines, Node and Python ten lines in three files; before the change 386, 330 and 102 divergences across the three laws, the digests of the old type string), zero divergences and zero differences from prediction at seeds 1 to 6 (Rust 7 text items, Node 7, Python 9), with 123,154 (Rust), 98,900 (Node) and 40,710 (Python) further probe inputs and no disagreement class. The four audit seats over the checkpoint found nothing above informational: SEC 3 informational items (426 proofs in its tree, the whole machine against round 5's at 300 fixed seeds and 5,000 fuzz runs equal), VULN 2 (360 proofs, the split equal to the old in-logic arithmetic over 50,000 runs, thirteen unbound addresses each refused), ADV 3 (369 proofs, 46,656 ordered pairs of acts, 987 fault combinations of the gate's order), FUZZ 5 (sixty invariants under a five-way draw of `sellerFund`, two campaigns of 256 runs at depth 500 at seeds 0x6a and 0x6b with 128,000 calls each, 258 paths reached, eight mutants of the revision caught), the informational items all in the texts and the comments; this is the first of the two clean rounds the contracts' gate asks. Four items bore load, two of the revision and two of the link corpus from before it: the standard's §10 naming the seller's key as the `Swept` creditor where the deposit now returns to `sellerFund`; the dossier law's §5.1 `fundBind` row placing the bond before an `Opened` the instance may never emit; the link corpus leaving nine members of §7.1's three comparison lists unwitnessed alone; and §6.1's nested objects without their closure cases; a reader skipping any of the last two passed every seed. Everything was folded in one pass with the text items, the code's bytes unchanged (`identity.sh` agreeing with CODEHASH.md after the comments and one local name were corrected); the three round-6 programs, unchanged, are measured again over the folded corpora in the next round, the closing round of the texts if that first measurement is zero.

Folded into this law: §2.3 the `sellerFund` row stating the gate's one test (`sellerKey` itself where the deposit sits at the seller's own address, otherwise the address that holds it, bound to the seller on the logic) and the `openWindow` row counting from a by-call buyer's export plus its consent's earliest landing; §5.5 rewritten where it contradicted itself (the key approves the logic only where `sellerFund` is the key; the deposit stands at `sellerFund`) and the by-call buyer's `openBy` set to the earlier of the offer's `expiry` and E plus `openWindow`, E the second of export plus the longest time its consent can take before it first lands (for a CHELEK company the `calendarBound()` its `chelek.standard/1` contract serves, the bound of `chelek.charter/1` §5.5, plus a margin), no order exported whose `openBy` is at or before that landing; §8 the first bullet reading whether `sellerFund` is `sellerKey` or bound to it (`funds`) and holds a balance and an allowance to the logic each at least P + `lf`. The corpus grew from 578 to 583 cases (`derivations_fund_other`, a document whose `sellerFund` is another address, and over it `offer_fund_other`, `order_fund_other`, `order_buyer_is_fund`, `verify_offer_fund_other`); the generator's `closed` note counts the table's keys; the README's §4.4 row names the fund as no excluded address and a new row the three derivations; zero differences from prediction at seeds 1 to 6. The EIP-712 vectors carry a ninth document with another `sellerFund`, the vectors test binding and funding from it. The standard changed in wording alone, the code not at all: the Shape and §10 list `split` on the verifier and `funds` among the logic's views; §2's excerpt names `sellerFund`; the Shape, §9 and §12 say `sellerFund` grants the allowance; §6's Funded race names the resignation path; §7a sorts sellers by where the deposit sits, cites `chelek.standard/1` §7, says what a bond and an allowance consent to (every case under an offer the bound seller consented to pulls from the funder's allowance on that case's token, D leaving for good where §8 gives P to the judge or the seller's LF to the buyer; a funder binds a seller it trusts with the whole allowance and funds sellers apart from separate addresses) and the pool rule over allowance and balance, a returned deposit refilling the balance alone; §8 implements the table through `split`, its gloss covering every return of the deposit, and `Closed`'s four sums are by role, an address that is two of the three payees receiving the sum of their parts; §9's caller rule reaches `acceptFunding` and the registries' views answer false on an instance; §10's `Swept` creditor is the deposit's address `sellerFund`; §13 says a comment changes no byte under the pinned compile. CODEHASH.md's newest paragraph cites standard s8; the conformance README's commit rule names a member of a binding's comparison list; `Eravon.sol`'s header (three registries), `sweep`'s and `_finish`'s comments and the local name `toSellerFund`, and `EravonVerifier.sol`'s header (`split`, `panel`) were corrected, the identity unchanged. The round-6 FUZZ seat's handler is folded under `test/` (the five-way draw of `sellerFund`, the funder's acts, sixty invariants, `invariant_pullFromBoundFundOnly` the one §11 cites, 258 paths), with its round-6 proofs under `test/r6/`.


### Round 2: the twelve seats over the round-1 fold

The twelve seats read the round-1 fold (Court 90c01f71): readers A 5 items (0 load-bearing), B 7 (0), C 4 (2, both in the corpus alone), D 3 (0), E 8 (0), node 6 (1, in the corpus alone), and the three implementers, each with its round-6 program unchanged, zero divergences and zero differences from prediction at seeds 1 to 6 on the first measurement (583, 668 and 300 cases; 18 runs of 18 for each of Rust, Node and Python), with 145,516 (Rust), 117,318 (Node) and 73,588 (Python) further probe inputs and no disagreement class (Rust 5 text items, Python 7). The four audit seats found nothing above informational: SEC 1 informational item (its round-6 proofs at fresh seeds, the fund invariant widened to every placement of `sellerFund` at 512 runs of depth 128, the grief scanner for a company seller), VULN 1 (370 proofs in its tree, class 4 taken deeper with an opcode census, look-alike proxies, delegation and a clone logic, the split equal to the round-5 arithmetic over 100,000 runs), ADV 2 (1,679,616 ordered triples of acts with a third actor in the same block, §7a sentence by sentence, nine mutants caught), FUZZ none at any severity (the folded handler at 256 runs of depth 500 twice at fresh seeds, 60 of 60; twelve mutants of the revision caught; the handler extended to one fund drawn on by several instances of one seller and to a judge that funds the case it judges, 63 invariants and 295 paths, folded under `test/`); this is the second consecutive round with nothing above informational, and the contracts' gate is closed. This is the closing round of the law texts, as the rounds of 2026-09-24, 2026-09-23, 2026-10-14 and 2026-10-17 closed: the three programs changed in no line and reached zero divergences on their first measurement, and what the readers reported was folded in one pass. The items that bore load were corpus witnesses alone, the texts and the criteria deciding every case already: the link corpus left §6.1's nested objects without a case for each member absent and the receipt's passkey object without a member beyond its six, §7.1's guild array without its index rule on two guilds, and §7.1's ladder without a case on a rung's shares or the ladder's length; a reader departing from the text there passed every seed. The three round-7 programs, unchanged, were then measured again over the folded corpora at seeds 1 to 6, 54 runs of 54 with zero divergences and zero differences from prediction; ten criterion mutants aimed at the new cases each fail on the cases written for them.

Folded into this law: §2.3's `openWindow` row naming E of §5.5; §5.5 the deposit stated by where it sits (the key holding it and sending the approval where `sellerFund` is the key; another address holding it, granting the allowance and bound to the seller otherwise), the delegation's reasons reaching `acceptFunding`, the by-call buyer's E taken for a CHELEK company as `bound(S, w)` of `chelek.charter/1` §5.5 over the window its consent's proposal is posted under (the `calendarBound()` its contract serves, docs/chelek-standard-v1.md §2, where that window is the shortest), and no order exported whose `openBy` is at or before E; §8's first bullet naming `lf` where the quote prices by feeds. The corpus grew from 583 to 590 cases (`verify_offer_by_fund` and `act_refund_by_fund`, the fund's key signing, and `act_refund_seller_fund_other`; `ok_int_top`, `chainId`, `nonce` and `expiry` at 2^53 − 1 with a source and a pointer on that chain, through `offer` and `order`); zero differences from prediction at seeds 1 to 6; the README's rows name them. The standard changed in wording alone, the code not at all: the Shape's head names every terminal among the verifier's callers; §7a says a withdrawal holds per token and only until the funder grants the logic an allowance again, the bond outliving every allowance, and that granting one again grants it to every bound seller at once, names the part of D §8 gives away, cites `chelek.standard/1` by its name and file, and says sellers naming one `sellerFund` share its allowance and balance; §11 names each cited test's file; §12 describes the company run's acts without an order; §13 lets `identity.sh` decide and names the directive comment that is code. CODEHASH.md's verifier paragraph names every terminal; `Eravon.sol`'s `init` comment points to §4 instead of restating its steps, its `_finish` comment names the verifier's `split`, and `EravonVerifier.sol`'s header names the pricing as the standard's; the identity unchanged.

Named again on 2026-10-19, at the Court commit after this record: the release digest
`0x5cb4a7f34c2a5b46b290614165c32a5ceb95b928b7f355ca3e467d0143b1945e`
over `ert.py`, `ertlaw.py`, `ertcrypto.py` and the `galeed/1` core of §10.4
(`0x58b621c37880c60ff88380c6a90c16f6eeb18a24fc31949bd31796ee770e4a0d`, the chain-as-ledger revision's), as §10.5 quotes it. The text's own sha256 at the naming is
`93cd2e0cab9acdaa09b83123e0fb9e5646dc1433b2289a7aa58f250d1d1d5c29`. The fuzzing of the deferral was run across the five rounds of the revision of 2026-10-17 and the two of this one, as the digest's note records. `eravon.link/1` and `eravon.dossier/1`, whose cores embed this one, were named by the same act (their records).

Prose in place after the naming, 2026-10-19: §5.5's parenthetical on a by-call buyer's E now states the rule for any buyer that is a contract whose consent waits on its own rules (the longest delay those rules allow for the consent as it is posted, plus a margin its client fixes), and `eravon.standard/1` §7a's parenthetical on a funder that is a contract states that such a contract binds itself by its own call of `acceptFunding(seller)`; neither text names another system, compatibility with one following from the general rule. Both sentences are [O] or prose and measured by no corpus: the three cores and their digests are unchanged, as the three digest scripts recompute, and the corpora show zero differences from prediction at seeds 1 to 6. The text's own sha256 after this revision is
`e5ded84daf454e76a727604e93c3fbe676a1c268a16a45fc4d41a52d9b83c13e`.

## Revision in place of 2026-10-24: the succession book, the venue's new interface, and two of the family's own

The judge's revision of 2026-10-24 (`逐条修订稿-换钥簿与场所新接口及本家两项-2026-10-24`, 186 items over the four ERAVON laws, with the full revised texts beside it): the family reads every key through GALEED's succession book (`galeed.roster/1` §4), so that a party's, a judge's and a guild's key is its record's current key on the book the standard's code names; the standard follows the venue interface's new form (`galeed.venue/1`: `partyRoles`, `outcomeOf`, `advanceOf`, `successionBook`, `routeGas`, the seat's two credited fields, a panel's router called with exactly `routeGas()`, one panel book per chain filing under the guild's root); and two items of the family's own: the quote loses its payout word, a judge registering its own payout on the instance, and `lapseDefault` admits 1, an even split at a lapse; the passkey (P-256) form is removed from the family. For this law: §3 the quote of fifteen words, 480 bytes, with no payout and `lapseDefault` 0, 1 or 2; §5 one signature form for both parties, the P-256 form and its faults gone; the core carries `galeed/1`'s canonical form and secp256k1 itself (`ertcanon.py`, `ertcrypto.py`) and embeds no `galeed/1` core, so §10.4 pins no GALEED digest; `sign-passkey` removed from the commands. The naming section returned to its unnamed form.

### Round 0: the criterion against its own generator

The 186 items applied in place by script, every existing text found exactly once in the base and the result word for word the judge's revised text, each touched paragraph rewrapped at 76 columns (Court 9ab8f451). The criterion: the quote decoder of fifteen words, `lapseDefault` admitting 1, the passkey verifier and its command removed, the two new modules in place of the embedded core; `criterion-digest-terms.sh` over `ert.py`, `ertlaw.py`, `ertcrypto.py` and `ertcanon.py` with no GALEED line. HARNESS-TERMS, the generator and the README follow. The corpus went from 590 to 514 cases at seed 1 (the passkey cases removed; the quote cases rebuilt over fifteen words, with lengths 0, 479, 481 and 512 and `w13` at each of its three values and past them; an object where a signature string stands); zero differences from prediction at seeds 1 to 6. Over the old corpus the old and the new criterion differ in 116 answers, each one of them the revision's. The contracts were rebuilt for the new generation in the same round (the standard's audit record); the EIP-712 vectors of `eravon/standard/test/vectors/` were regenerated from this criterion over fifteen-word quotes.

### Round 1: the twelve seats over the round-0 checkpoint

The twelve seats read the round-0 checkpoint (Court 16c772f9): readers A (this law) 9 items (0 load-bearing), B (the standard against the contracts and the replay) 10 (2), C (`eravon.link/1`) 9 (5, four in the corpus alone), D (`eravon.dossier/1`) 10 (2), E (the four laws against the judge's draft, the tooling and the corpora) 16 (1), and the three implementers, each starting from its round-7 program: measured unchanged, the round-7 programs diverged on the revision's classes alone (terms 52, link 56, dossier 0 at every seed: the 480-byte quote, the object signature, the three identity commands, the pack without its key), and, brought to the revised laws from the texts and the harnesses alone, each measured zero divergences and zero differences from prediction at seeds 1 to 6, 18 runs of 18, with 62,288 (Rust), 70,441 (Node) and 20,096 (Python) further probe inputs and no disagreement class (Rust 8 items, 1 load-bearing; Node 5, 2; Python 6, 0); Node also checked the standard's EIP-712 vectors, 231 comparisons with none differing. The four audit seats read the new contract generation: SEC one low and two informational items, VULN, ADV and FUZZ nothing above informational (4, 3 and 3); the low item, a router's answer naming a panel's zero entry, was verified a text item (the standard's audit record). No load-bearing item fell on this law's predicates. Folded into this law: §5.1's gloss on a passing signature (the assent of the key the predicate is handed, whether that key acts for the seller being the succession book's, the standard admitting the current key's signature alone); §5.5's list of a party's own transactions (claiming delivery, refunding, confirming, the expiry branch, settling, sending a signed message, drawing, sweeping) and the dispute fee pulled from the recorded `buyerKey` after a move; §8's consent by the buyer's call at `init` under an offer whose `buyerMayOpen` is `true`; §10.1's fuzzing record over the canonical form and secp256k1 arithmetic the core carries; §10.4 rewritten to name the passages of `galeed/1` it reads (§1, §2.1, §3, §5.4, §5.5, §5.7, the preamble of §6) as they stand when §10.5 names the core, a later text of them, in place under `galeed/1` §12.5 or as a `galeed/2`, changing nothing here until this law's own revision follows it; §10.5 naming `ertcanon.py` among the digested files, `naming.py`'s anchor following. The corpus grew from 514 to 521 cases (seat A's measurement gap: a decoder that truncates a word to its ABI type's width, or stores the free words in 128 bits, passed every seed; `w1_high_bits`, `w7_high_bits`, `w13_high_bits`, `w14_high_bits`, `ok_words_top`, and the fees `min_top` and `min_top_sum`), the README naming them and stating that §1's `base64url` is read by no predicate here; zero differences from prediction at seeds 1 to 6. `bcov.py` measures `ertcanon.py`, and the conformance README states the unnamed form. Recorded and not folded: the draft's item terms-18 is met by the digest script and now by §10.5's file list.

### Round 2: the twelve seats over the round-1 fold

The twelve seats read the round-1 fold (Court 9dd79a98): readers A (this law) 7 items (2 load-bearing, both in the corpus alone), B (the standard against the contracts and the replay) 5 (0), C (`eravon.link/1`) 4 (2, both in the corpus alone), D (`eravon.dossier/1`) 3 (1, in the corpus alone), E (the four laws against the judge's draft, the tooling and the corpora) 9 (1, in the corpus alone), and the three implementers: measured unchanged, the round-1 programs diverged on the one change the round-1 fold made to the criterion's answers and on nothing else (terms 0, link 10, dossier 0 at every seed: `history`'s separate `source` member), each wrote that member with one edit, and so changed each measured zero divergences and zero differences from prediction at seeds 1 to 6, 18 runs of 18, with 93,312 (Rust), 65,282 (Node) and 45,935 (Python) further probe inputs and no disagreement class (Rust 5 items, 0 load-bearing; Node 4, 1; Python 2, 0); Node checked the standard's EIP-712 vectors again, 231 comparisons with none differing. The four audit seats found nothing above informational on the contract code (SEC 4, VULN 5, ADV 2, FUZZ 3), the compiled bytes unchanged since 16c772f9; the judge ruled on 2026-10-25 that round 1 counts as the new generation's first clean round, so the contract gate closed at this round (the standard's audit record). The fold changes no answer of any criterion, the three digests standing as they were (terms `0xca99…4a9a`, link `0x9465…28e5`, dossier `0x7e2c…86f0`): it adds corpus cases and changes text, so the round-2 programs, unchanged, measure round 3 as the closing round if they show zero at seeds 1 to 6 on their first run. Seat A's two load-bearing items were corpus gaps: the fee's `feeMax` had been measured only at small values, so a core reading it as a signed word or holding it in 128 bits passed every seed; and the canonical form the core carries had been witnessed at a handful of escapes alone, so seven mutants of it (a leading zero answering `E_NOT_CANONICAL`, integers to 2^64, `\"` refused, a tab written as `\u0009`, `0X` admitted, a trailing comma, a depth limit of 127) passed every seed. Folded into this law: §5.5 states who sends each call by the contract that gates it (the standard its gates from the party's current key, `sweep` from a seated judge's and `redraw` from the guild's as well, the board a posting from any address, the token the approval from the address that grants it and the payment from any address, the book a declaration from the current key and an accession from the key the declaration named), one rule for what the instance pulls (the deposit from `sellerFund` under its allowance, the fee from `buyerKey` beyond what flowed in unrecorded since `init`, money sent before `init` being the surplus `init` credits), and the move as the accession, never the declaration (seats A, B, E, Node); §10.1 states that the canonical form the core carries is witnessed on every input of the `galeed/1` corpus's `check` set, each carried as the block of a document and predicted as that corpus predicts it, and the secp256k1 arithmetic on each adjacent pair of `galeed/1` §5.4's tests (seats A, Rust); §10.4 pins every passage of a GALEED law this text cites, `galeed.roster/1` §6.1 and §4 among them, names `ertlaw.py` for the member tables' reading and keeps its in-place form against the draft's item terms-17 (`galeed/1` §12.5 admits revision in place; reported to the judge). The corpus grew from 521 to 4,179 cases: `max_top` and `max_high_lowers` for the fee, and the 3,656 inputs of `galeed-conformance/corpus/canon/` as document blocks (`gl1_*`), predicted from that corpus's own prediction (an input that is no UTF-8 `E_TERMS_UTF8`, a token of §3.5 carried, a canonical value then `E_TERMS` or `E_TERMS_MISSING`), so that every boundary of `galeed/1` §3 that corpus witnesses is witnessed here and no hand list of escapes is kept; zero differences from prediction at seeds 1 to 6. The README's rows follow (the high-bits row names `w5_over`, `w11_over` and `w12_over` for the three words whose bound is their type's maximum, seat E). `naming.py` now writes, after the naming-day note, the record of the fuzzing run for the core named on 2026-10-19, bound to that core's digest, as the draft's item terms-33 keeps it (seats A, E). Recorded and not folded: MIKNAH's terms §3.1 still says "ERAVON's sixteen" of a quote that now has fifteen words (seat E; outside this revision, reported to the judge).

### Round 3: the eight law seats over the round-2 fold

The eight law seats read the round-2 fold (Court 7e4001bb); the contract gate having closed at round 2, no audit seat sat. Readers A (this law) 6 items (2 in the corpus alone, tagged load-bearing by seat A), B (the standard against the contracts, the tests and the replay) 3 (0), C (`eravon.link/1`) 5 (0), D (`eravon.dossier/1`) 1 (0), E (the four laws against the judge's draft, the tooling and the corpora) 6 (0); the three implementers, their round-2 programs unchanged, measured zero divergences and zero differences from prediction at seeds 1 to 6 on their first run, 18 runs of 18 each (terms 4,179, link 708, dossier 1,187 cases per seed), with 157,270 (Rust), 76,093 (Node) and 57,292 (Python) further probe inputs and no disagreement class (Rust 3 items, Node 2, Python 4, none load-bearing); Node checked the EIP-712 vectors once more, 231 comparisons with none differing. So round 3 is the closing round the gate asks for: the previous round's programs unchanged, zero at seeds 1 to 6 on the first measurement, no item that two careful implementations could read apart. Every item of the round is text or corpus, folded here in one pass; the criteria stand (terms `0xca99…4a9a`, link `0x9465…28e5`, dossier `0x7e2c…86f0`). The naming waits: the judge's plan of 2026-10-26 (stages, prepayment interlocked) is applied next in this generation and changes the three criteria, so the texts reopen under it. Folded into this law: §5.5 says each call the standard gates is admitted from the current keys that standard's §9 names for that call, and an accession is sent from the record's pending key, the key its last declaration that bound named (`galeed.roster/1` §4.2, §4.3) (seat A); §10.1 states the map from the `galeed/1` corpus's prediction to this law's answer (`E_TERMS_UTF8` where §2.4's first test decides, a §3.5 token carried, a canonical value then `E_TERMS` or `E_TERMS_MISSING`), names the imported set at seed 1 as committed, claims the boundaries past §3.5's first test, adds the boundaries that set leaves unwitnessed (an object's trailing comma), `galeed/1` §1's hex forms on each refusal, and the secp256k1 range test once by `r` and once by `s` (seats A, E, Node, Rust, Python); §10.4's roster clause reads "every section this text cites"; §10.5 names the files' carrying as §10.4 names it, HARNESS-TERMS and `naming.py`'s anchors following (seats A, E). The corpus grew from 4,179 to 4,200 cases: `canon_trailing_comma_object`; the hex prefix on a hex20, a hex32 and a hex65 (`payout_prefix_upper`, `payout_prefix_other`, `payout_prefix_absent`, `goodshash_prefix_upper`, `goodshash_prefix_absent`, `sig_prefix_upper`, `sig_prefix_absent`), the two mutants of seat A's round-2 seven the import had left (`0X` admitted, a trailing comma) now meeting a case; `sig_order_r_zero_before_high_s` and `sig_order_r_n_before_high_s`; and the `<sig.json>` reader's refusals, each once as misuse, with three neighbours it admits (`reader_*`); zero differences from prediction at seeds 1 to 6. The README's rows follow.

## Revision in place of 2026-10-26: stages, a sale delivered in parts under one judge

The judge's plan of 2026-10-26 (`修法方案-分段交付系列标识-2026-10-26`, its fourth version, with the ruling of the same day that a stage ended Lapsed carries no judge forward), applied in one pass as round 0 of the 2026-10-24 generation after that generation's round 3 closed the texts and before their naming: an order names the stage before it (`prev`), the standard opens a later stage only once the stage before is paid and lets the buyer cancel it only once the stage before failed, a document may declare the series its stages belong to, and the draw without a router seats the stage before's judge first. The three criteria change with it, so the texts reopen under it; the contract bytes change, so the contract gate restarts at this generation's next round (Court 56081380). The twelve seats sit again over this checkpoint.

### Round 0: the plan applied in place

§4.4 adds `prev` to `OrderTerms`, in the type string and the salt: a hex20,
zero for a first stage, else the instance of the stage before, read as the
buyer's client set it (`E_ORDER_PREV` on another form); §2.3 admits an
optional `series` member, an object of exactly `id` (a nonzero hex32),
`stage` (an integer from 1) and `stages` (an integer not below `stage`),
else `E_TERMS_SERIES`, read after the compliance test, the last of §2.4; §8
says what a series is to the clients and that the standard reads none of it.
The criterion follows in `ertlaw.py` (the series test, `order_of` with
`prev`), `ertcrypto.py` (the type string, `hash_order` of five words) and
`ert.py` (the six order and action commands take `<prev>` after `<openBy>`);
HARNESS-TERMS names the argument and its misuse; the digest is a candidate
until the naming. The corpus grew from 4,200 to 4,218 cases: `ok_series` and
`ok_series_last_stage` (`stages` equal to `stage`), eight refusals of the
member's form (`series_array`, `series_id_zero`, `series_id_short`,
`series_stage_zero`, `series_stage_string`, `series_stages_below_stage`,
`series_missing_member`, `series_extra_member`), the adjacent pair
`order_compliance_before_series`, and the order with a nonzero `prev` (its
salt, instance and digest differing from the zero form), a `prev` of another
form and a missing `prev` as misuse (`E_ORDER_PREV`'s only half is
unreachable on this surface, and `E_ORDER_OPENBY` before `E_ORDER_PREV` is
witnessed on link's as `adj_openby_before_prev`), a signature over an order
with a nonzero `prev` and the same signature against another `prev`
(`secp_prev_ok`, `secp_other_prev`), and the uppercase argument as misuse;
zero differences from prediction at seeds 1 to 6. The README's §2 and §4
rows follow.

### Round 1: the twelve seats over the stages round 0

The twelve seats sat over the stages round 0 (Court afa7e8e4): readers A
(`eravon.terms/1`) 11 items (2 tagged load-bearing, both corpus gaps), B
(the standard against the contracts) 8 (0), C (`eravon.link/1`) 8 (0), D
(`eravon.dossier/1`) 1 (0), E (the four laws against the plan and the
tooling) 7 (0); the three implementers brought their programs to the
reopened texts from the texts alone and measured once: zero divergences and
zero differences from prediction at seeds 1 to 6 in all 18 runs each (terms
4,218, link 4,487, dossier 1,189 cases per seed), with 23,295 (Rust), 25,367
(Node) and 32,790 (Python) probe inputs and no disagreement class, each
reproducing the EIP-712 vectors (Rust 3 items, Node 3, Python 4, all text);
the four audit seats found one low finding on the contract code (SEC26-L1,
the audit record) and fifteen informational items. Every law item is text or
corpus and is folded here in one pass; no criterion file changes, so the
three candidate digests stand (terms `0x4984…d0a9`, link `0x25eb…11dc`,
dossier `0x93ae…bf3d`) and the next round, the implementers' programs
unchanged, can close the texts. The contract bytes change with the fold (the
audit record), so the contract gate counts from the next round (Court
43f56b07). The judge's rulings of 2026-10-26 on the seats' items: the series
judge, the judge who last heard a stage of the sale, is handed on from stage
to stage through `stage()` and seated first at a later stage's draw (a stage
that opened no served seat hands the inherited judge on; a judge who
resigned or declined the seat it entered by this stage's own draw is not it,
while an inherited judge stays the series judge though it declines or
resigns a later stage, the buyer ending the series by a dispute where it
will not go on; a Lapsed stage hands none on); `cancel`'s faults return to
the plan's order, read on the verifier; a series is one chain and one logic
under the same two keys, a client rule. Folded into this law: §4.4 says the
stage before is an instance of the same logic on the same chain naming this
order's two keys, the chain written under the same two keys (seats A, B,
ADV); §8's Stages names the buyer's client's two acts (signs or exports, one
`buyerKey`, seat A), cites `eravon.link/1` §10 for the seller's client rule
so that the rule lives in one text (seat Node), states the series judge with
§7's conditions in place of "one judge hears them all" (seats A, B, C), asks
for a later stage's `openWindow` and its offer's `expiry` together, since
§5.5's `openBy` is the earlier of the two (seats A, B, E), and says what the
lock does in the standard's words, a later stage's own `openBy`, `expiry` or
revocation alone returning its money (seat A); §10.1 lists `E_ORDER_PREV`
among the form halves the harness makes misuse, as the plan asked (seats A,
E, Rust, Node, Python); HARNESS-TERMS says `E_ORDER_PREV` has no other half
(seat Rust). The corpus grew from 4,218 to 4,238 cases: the `series`
member's refusals one per form (`series_missing_id`, `series_missing_stage`,
`series_empty`, `series_null`, `series_stage_bool`, `series_stages_string`,
`series_id_hex20`, `series_id_upper`; seats A, E, Rust, the six mutants seat
A found surviving now meeting a case at every seed), the `<prev>` misuse on
`order` (uppercase digits, no prefix, the empty string) and once on each of
the other five commands (seat A), and the action family at a nonzero `prev`
(`act_confirm_prev`, `sign_action_prev`, `act_confirm_prev_ok`, and
`act_confirm_other_prev`, a Confirm signed under the first stage's instance
verifying under no other `prev`; seats A, E: a candidate deriving every
action's instance at `prev` zero passed all six seeds before); zero
differences from prediction at seeds 1 to 6. The vectors carry each order
again as a later stage (seats A, Python). The README's rows follow
(twenty-three keys, seats A and Rust), and this record's Round 0 reads as
the corpus stands (seats A, E, Node).

### Round 2: the twelve seats over the round-1 fold

The twelve seats sat over the round-1 fold (Court f5b6a5d2). The three implementers, their round-1 programs unchanged (Rust's binaries byte-identical, Node's and Python's sources checked by digest), measured zero divergences and zero differences from prediction at seeds 1 to 6 on their first run, 18 runs of 18 each (terms 4,238, link 4,505, dossier 1,197 cases per seed), with 26,584 (Rust), 27,539 (Node) and 50,718 (Python) further probe inputs and no disagreement class (Rust 0 items, Node 2, Python 2, all text). Readers A (`eravon.terms/1`) 3 items, B (the standard against the contracts) 8 (one of them on the contract's code, informational, folded as text by the judge's ruling), C (`eravon.link/1`) 4, D (`eravon.dossier/1`) 3, E (the four laws against the plan and the tooling) 4 and a family report; every item text or corpus, none load-bearing, every seat saying it would name. The four audit seats found nothing above informational on the fold's bytes (the audit record), so the fold changes no byte. **So round 2 is the closing round the gate asks for**: the previous round's programs unchanged, zero at seeds 1 to 6 on the first measurement, no load-bearing item; the residue is folded here in one pass (Court f78f62c8), and the folded sentences are read once more by the readers of the next round, which is the audit's second round over these bytes; the naming follows the contract gate (`law-revision-standard` 3.1: the type string and the verifier are shared with the contract). The judge's rulings of 2026-10-26 on the round's two questions: an inherited series judge who declines or resigns a later stage stays the series judge (the parties end the series by a dispute where they will not go on; no judge is drawn to replace it for that alone), and an account of other code than the proxy's that proves an address at the doors is a text matter (no reader admits it; events are read by the instance's address), the bytes standing. Folded into this law: §4.4's last sentence attaches the two keys to the opening alone, the buyer cancelling once the instance at `prev`, whatever keys it names, ended leaving it a share, as the standard's `cancel` reads no key (seats A, E); §8 cites the standard's §7 and §4a for the series judge in place of a paraphrase (seats B, C, Node, Python), and its parenthesis on `openBy` states §5.5's derivation (the earlier of `expiry` and the second of signing, or E, plus `openWindow`; seat Node). The corpus grew from 4,238 to 4,239 cases: `series_stages_bool` (seat A's surviving mutant, a bool admitted as `stages`; seat E's plant TS had met `series_stages_string`); zero differences from prediction at seeds 1 to 6. The README's row follows.

### Round 3: the re-reading of the round-2 fold

The twelve seats sat over the round-2 fold (Court 86dc194c), the re-reading `law-revision-standard` 3.2 asks for. The three implementers, their programs unchanged since round 1, measured zero divergences and zero differences from prediction at seeds 1 to 6 on their first run, 18 runs of 18 each (terms 4,239, link 4,505, dossier 1,197 cases per seed, the new `series_stages_bool` among them), with 29,172 (Rust), 30,568 (Node) and 57,178 (Python) further probe inputs and no disagreement class (Rust 1 item, Node 3, Python 3, all text). Readers A 2 items, B 4, C 4, D 2, E 2 and a family report; every item text, none load-bearing, every seat ready to name the moment the contract gate closes. The four audit seats found nothing above informational on the bytes of the round-1 fold, the second consecutive clean round over them, **so the contract gate closed** (the audit record). The residue is folded here in one pass (Court a29c4a10): the sentences the round-2 fold had added are read as one reading now, or reworded where two readers took them two ways; no criterion file changes and the three digests stand. The naming follows. Folded into this law: §8's parenthesis on `openBy` states §5.5's grouping (the earlier of `expiry` and `openWindow` seconds after the buyer's signing, or after E; seats A, Rust); §8 names `sellerKey` where it said parties (a document carries no buyer's key; seat C) and says the lock is the order's that names `prev`, not the document's, a seller who wants it on later stages writing `buyerMayOpen` false on them (seat ADV: under `buyerMayOpen` the buyer may open a later stage's document under an order of its own writing, which a seller's client takes as no stage and which runs out to the buyer at T1). The corpus is unchanged at 4,239 cases; zero differences from prediction at seeds 1 to 6. The README's row loses a stray space (seat A).

### The naming of 2026-10-27

The contract gate having closed at the stages' rounds 2 and 3 (the audit record), and the texts at round 2 with their residue read once more in round 3 and by a re-reading seat over the round-3 fold (16 of 18 sentences read as one reading, two reworded: terms §8's lock sentence and dossier §7.1's witness clause), `naming.py 2026-10-27` named the three cores in the order they nest, each note carrying the fuzzing of this revision (the three seats' 951,220 differential inputs over its six rounds: Rust 391,921, Node 295,290, Python 264,009) and, after it, the record of the fuzzing run for the core named on 2026-10-19. After the act the three digest scripts recomputed the digests the sections carry, `compare.py` at seeds 1 to 6 exited 0 for the three laws, `forge test` passed 335 of 335 and `identity.sh` agreed with CODEHASH.md (Court 6d561dad). This law's §10.5 names `0x4984258d42de40eb19e51a8291cb9b572d9b9ab184e12e7eda19893a61dcd0a9`, superseding `0x5cb4a7f34c2a5b46b290614165c32a5ceb95b928b7f355ca3e467d0143b1945e` (named on 2026-10-19, the sellerFund revision) and the unnamed candidate `0xca99…4a9a` of the 2026-10-24 revision's rounds 1 to 3, which the stages' `prev`, `series` and receipt member replaced before any naming. The text's sha256 after the act is `b6e2431299820f5922fe9aac5667252d23948e5dd97254e6fc1823499ad6aad2`.

## Named again on 2026-11-16: the GALEED texts of that date

The GALEED texts this law cites moved after this law's naming of 2026-10-27:
`galeed/1` was revised in place on 2026-10-30 (§7.4: a key's records in the
second of its handing off that carry what it signed at a position after its
retiring seq count for nothing), and its four cores were named again
(`galeed/1`
`0x5f24cd212705b0cbfd00e5fe0c720ca059b662276c7eaac9a7907e621c50e65d`,
`galeed.roster/1`
`0x9d0ce54d8b74ca3e97f3e2e5c22200f4eea2b2c3143c6770bee9d453320963d0`,
`galeed.papers/1`
`0xfcc4ee4905921f5dc815f47c6b0cf38505c29ca7dc6b27e4b298e097faf60759`,
`courtevidence/1`
`0xe66eb68d47d5dd03767eb7728e5610f1a9a9a3a9cc0a6dfeb5ab559e1f1b5d15`);
`galeed.papers/1` §12 gained a bullet on 2026-11-16 by a textbook revision,
the venue's interface identity
`0xf97ad599d9d4f5149fb93962c15f18b2f601251f2085358c3117ad8fe5af391e` and the
three GALEED contracts unchanged. The judge ordered on 2026-11-16 that the
ERAVON laws follow. Each ERAVON core carries its own implementation of what
it reads of the GALEED laws and names no GALEED core, so following is one
act of each naming section: the three cores are named again on 2026-11-16 at
the digests of 2026-10-27, and each law then reads the GALEED passages it
cites as they stand on that date.

Of the passages §10.4 names, two moved: `galeed/1` §5.7, whose re-anchoring
sentence now names the position of the ceding key's entry, its signing and
low-s sentences, the ones this law reads, as they were; and `galeed/1`
§12.5, which names the new core. Every `galeed.roster/1`, `galeed.venue/1`
and `galeed.papers/1` passage this text cites reads as it did. The core's
reading is unchanged.

Measured with the current GALEED tree beside the ERAVON one (the generators
and the comparator borrow the `galeed/1` core and corpus): the three digest
scripts recompute
`0x4984258d42de40eb19e51a8291cb9b572d9b9ab184e12e7eda19893a61dcd0a9`,
`0x25eb3810d9a870633608d36054a8399e1445435caa703c7ed5e8fd1bae8f11dc` and
`0x93ae4df7f2058a724d5da8ef38925e7d3b99a2f2861c40b4a42d3c61a350bf3d` over
cores unchanged in every byte; `gen.py --seed 1` rebuilds the three
committed corpora byte for byte, the dossier corpus's seven deliberate links
among them; `compare.py`, the criterion against the predictions, reads zero
at seeds 1 to 6 for each law; `forge test` passes 340 tests in
`eravon/standard` (list
`0xed8fea2ecd84669b3e00db44e95a4589edcc0360b4d41244c652b26aa0e74593`) and
the venue replay's 6; `identity.sh` recomputes the twelve values
`CODEHASH.md` carries. No corpus input, prediction or answer moved. The
text's own sha256 after this naming is
`0xbceb42dd7a3ff0ec5853d1a528d94ce49c5f908fb6ae709fce066f216fadb190` (before
it, `0xb6e2431299820f5922fe9aac5667252d23948e5dd97254e6fc1823499ad6aad2`).
