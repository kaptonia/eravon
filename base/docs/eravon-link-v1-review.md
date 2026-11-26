# eravon.link/1: the review record

The convergence of `eravon.link/1` (docs/eravon-link-v1.md) is run the way
`galeed/1`, `galeed.papers/1`, `galeed.roster/1`, and `courtevidence/1`
were run: adversarial readers of the text and independent implementations
written from the text and the harness alone, one round at a time, every
finding folded in one pass, the criterion, the harness, and the corpus
following, until the readers report nothing open and the implementations
reach zero divergences with every closed boundary witnessed on both
sides. The seating of each round is described in the `eravon.terms/1`
record; this record carries what concerns `eravon.link/1`.

## Round 0: the criterion against its own generator

The criterion (`eravon-conformance/link-py/`) and the corpus generator
(`eravon-conformance/link-corpus/gen.py`, 253 cases at seed 1) were
written from the text by one hand and compared under seeds 1 to 4: zero
disagreements with the generator's hand-written predictions.

## Round 1: five readers and three implementations

The three implementations reached zero divergences on the 253-case corpus
at seeds 1 to 6 after settling four first-run divergences (three the
pack's carrying rows, one a misreading of "in `pointer`") and found one
disagreement beyond the corpus, on whose length `BIND_JUDGES` carries
where the payload is the longer. The load-bearing findings on this law:
§2.5 said ten required members and twelve in all where §2.1's table has
eleven and thirteen (every seat named it); `terms_md` of the wrong kind
carried a token of a test never run; `BIND_JUDGES`'s length named
neither side; the added members' position in the element order, and
whether the offer and signature tests interleave per candidate, were
unstated; `milestone` and `milestones` were `int` here and `uint32` in
the offer; `RECEIPT_SEQ_MISMATCH` forbade the window's re-read of the
guild's book; `E_UNLOCK_ADDRESS` compared a `hex20` with a
`hexdigits(20)`; `priv`'s byte width was unstated; the harness bounded
neither `<c>` nor `<id>`; the law closed no token vocabulary; the link
`#e=` alone did not describe what a mirror sends or what the native seat
receives; the settlement link of the design volume had no document;
`E_GUILD_SOURCE` named a member the guild table has not got; `LINK_OK`
reached no answer; `key` had two meanings in the harness.

Folded (Court commit after this record):

- §2.1 and §2.5 count eleven and thirteen; `uri` is required and may be
  empty; §2.2 makes the two indices `u32` and names `E_RUNG_CLOSED` for
  a rung's extra member; §2.3 binds `sellerKey`, `token`, and `chainId`
  to the payload's, orders the added members after the inherited rows,
  and renames the text rule's token `E_GUILD_TEXT`; §2.5 gives `order`'s
  form test its token, compares judges and guilds by address, and
  decides each candidate's offer and signature before the next.
- §2.7 admits the native seat's `eravon://open?e=` beside `#e=`, says a
  mirror sends the fragment inside an `https` URL, and drops the name
  `LINK_OK`; §4.3 fixes `eravon://import?p=` for a pack.
- §3.4 drops `RECEIPT_SEQ_MISMATCH` and says why, binds every name it
  uses, and notes `E_OFFER_CAP` unreachable there; §3.1 makes
  `clientDataJSON` a skeleton string as the terms law now does.
- §4.2 says a `payload` or `receipt` of the wrong kind carries the other
  section's root token and a `terms_md` of the wrong kind carries
  nothing, and names the verdicts a pack can never carry.
- §5.2 strips the `0x` and writes the mac as hexdigits; §5.3 fixes the
  32-byte width and the identifier's charset; §1 says the counter wraps
  modulo 2^128.
- §6.1 carries `P`'s own length and treats a member present on one side
  only as a difference.
- §7 is the settlement offer (`eravon.settle/1`) with its transport
  `eravon-settle:` and its binding to a pack through the terms law's
  `Settle` message; §8 closes every token, verdict, and literal; §10.3
  and §10.4 say what a pack's transport and the terms digest are.
- Harness: `hasKey` replaces the accepted answer's `key`; `<c>` is at
  most 2^20 and `<id>` a skeleton string, an `unlock` above the bound
  being misuse; `pack-decode`, `settle-check`, `settle-encode`,
  `settle-decode`, `bind-settle` added; `termsHash` named as
  `terms.hash`.
- Corpus: 335 cases, with fourteen more §2.5 adjacencies, the
  interleaving witness, the inherited-before-added witnesses, the
  pointers' chain moved with the payload's, the `u32` bounds, the
  open-prefix link and the import prefix refused as a link, a longer
  payload under `bind` for both arrays, the passkey twin binding, the
  published keystore v3 vector as literals under `unlock` and `lock`,
  `lock`'s bounds and forms, and the settlement families.

## Round 2: the same eight seats on the revised text

The three implementations reached zero divergences on the 335-case corpus
at seeds 1 to 6 on their first measurement and found one disagreement
beyond it, on whether `unlock`'s `<c>` bound is tested against a pack
§4.2 refuses. The readers closed every round-1 item; the load-bearing
findings on this law: §2.7 counted `eravon://open?e=` as fifteen bytes
where it is sixteen, and §2.1 tested `terms.hash` for nothing where the
terms law's §4.1 requires it nonzero, the two items that change a frozen
output; §1 said no root key ends in `_md` where a pack's `terms_md`
does; §2.3 bound the inherited names for the judge table alone; the
pack's transport was a total predicate inside an unmarked paragraph, and
§10.1 claimed a transport encoding the pack has not got; §8 named
`E_RUNG_NOTE`, which §2.2 makes unreachable, and `UNLOCKED`, which no
answer produces, and said nothing of which carried tokens `E_LINK_SIG`,
`E_LINK_OFFER`, `RECEIPT_ORDER`, `RECEIPT_SIG`, and `SETTLE_SIG` can
reach; the settlement travelled as a link in the design volume and as a
bare text here; the corpus lacked `E_LADDER_LAST` on a payload, the
thirteen adjacencies inside §2.5's `order` object, the `mirrors` before
`pageHash` pair, `E_SETTLE_CLOSED` before `E_SETTLE_V`, and three of the
five signature tokens `E_LINK_SIG` carries; the harness did not place
the `<c>` refusal against the pack's own fault.

Folded (Court commit after this record):

- §2.7 counts sixteen bytes; §2.1 makes `terms.hash` nonzero; §1 names
  the three `_md` keys and where they sit; §2.3 binds `sellerKey`,
  `token`, and `chainId` in both inherited tables.
- §4.4 is the pack's transport as a `[D]` rule and says the law defines
  no encoder for a pack; §7.3 gives the settlement offer its three
  transports (`eravon-settle:`, `#s=`, `eravon://settle?s=`) and
  `decodeSettle` over all three; §7.4 says `SETTLE_SIG` never carries
  `E_SIG_FORM`.
- §8 excepts `E_RUNG_NOTE`, names for every carrying token which inner
  tokens it can carry, says `UNLOCKED` is what the harness renders as the
  key, and adds the two literals; §10.1 measures transport decoding and
  every encoding this law defines and exempts the unreachable carried
  tokens; the Status clause says the digest is agreed with once named.
- Harness: the `<c>` bound is decided where §5.2 reaches the key and
  after §4.2 has passed; `pack-decode` cites §4.4; `settle-decode` reads
  any of the three prefixes.
- Corpus: 369 cases, with `terms.hash` zero, the payload's last rung, the
  thirteen `order` adjacencies, `mirrors` before `pageHash`, `E_LINK_SIG`
  carrying `E_SIG_RANGE`, `E_SIG_RECOVER`, and `E_SIG_HIGH_S`, the
  settlement's `closed` before `v`, the fragment and native settle
  prefixes with the open-link prefix refused as a settle, and `unlock`
  over an invalid pack whose `c` is above the bound.

## Round 3: the same eight seats on the round-2 text

The three implementations reached zero divergences on the 369-case corpus
at seeds 1 to 6 on their first measurement and found no disagreement
beyond it. The readers closed every round-2 item; the load-bearing
findings on this law: §1's list of `_md` keys read as closed against the
suffix rule the same bullet invokes, the one item with an answer two
honest readings give differently on a constructible input (a `note_md`
in a rung, a `zz_md` at the root); §8 named two carried tokens no input
reaches (`RECEIPT_ORDER` with `E_ORDER_PAYOUT`, `RECEIPT_SIG` with
`E_ACTION_PARTY`) and cited §4.3 for the pack's transport; §7.3 stated
`encodeSettle` without its failing half; §3.1 gave a receipt's
`termsHash` no nonzero rule where §2.1 now has one; §10.1's quantifier
over carried tokens was read two ways; the corpus witnessed none of the
inherited rung, judge, and guild tokens a payload reaches, one passkey
token, two of `SETTLE_SIG`'s five, two of `RECEIPT_SIG`'s twelve, two
adjacencies its own README claimed, and no `E_UTF8`.

Folded (Court commit after this record):

- §1 names the three `_md` keys a passing document carries and says the
  parent's suffix rule decides every other key alike; §3.1 makes a
  receipt's `termsHash` nonzero; §7.3 gives `encodeSettle` its failing
  half; §8 cites §4.4, says `RECEIPT_ORDER` never carries
  `E_ORDER_PAYOUT`, and bounds `RECEIPT_SIG`'s carried set by §5.1
  through §5.3; §10.1 ranges over pairs where §8 names a finite carried
  set and over the carrier alone otherwise.
- Corpus: 407 cases, with every inherited rung, judge, and guild token
  a payload reaches (the quote tokens carried three ways under a judge
  and under a guild), `E_UTF8`, the `note_md` and `zz_md` witnesses,
  `RECEIPT_SIG` carrying each of its twelve reachable tokens,
  `SETTLE_SIG` each of its five, and the two adjacencies (`onramp`
  before the offers, a candidate's offer before its signature).

## Round 4: the same eight seats on the round-3 text

The three implementations reached zero divergences on the 407-case corpus
at seeds 1 to 6. The finding of the round was against the criterion: round
3 made a receipt's `termsHash` nonzero in §3.1 and the criterion kept the
form test alone, so `receipt-check`, `bind-receipt` and `pack-check`
answered a zero `termsHash` other than the text, on an input no corpus
case carried; the readers, the auditor and the Rust seat each found it.
Beside it: §10.1's quantifier sentence had the wrong referent and a
discriminator that sorted none of the seven carriers it was written for;
four of the eight `galeed/1` §3 tokens were still unwitnessed by the
auditor's count and witnessed by the coordinator's, a difference of
vocabulary lists the pair sentence now settles.

Folded (Court commit after this record):

- The criterion tests a receipt's `termsHash` nonzero (`E_RECEIPT_TERMS`);
  §10.1 names the seven carriers the precondition takes in pairs and what
  each carries, and says every other carrying token is witnessed once.
- Corpus: 409 cases, with the zero `termsHash` under `receipt-check` and
  as the receipt's own fault under `bind-receipt`.

## Round 5 and the naming

The three implementations reached zero divergences on the 414-case corpus
at seeds 1 to 6 with no change to their code and no disagreement beyond
it. The link reader asked for the four canonical-form tokens a payload
reaches (`E_DEPTH`, `E_NUMBER`, `E_DUP_KEY`, `E_KEY_CHARSET`) and for two
citations (§7.4's `§4.5` and §10.4's `§10.4`, both `eravon.terms/1`'s);
the corpus auditor for a key that is exactly `_md` at a payload's root;
the Python seat for §1's `_md` sentence to carry the parent's two cases
as the terms law's does. All folded, with §8's literal heading split by
where a literal lives, and the corpus at 414 cases.

Named on 2026-09-16, after `eravon.terms/1` §10.5 and with §10.4 quoting
that digest: the release digest
`0xe4b7e7dece52d3e8bb81e6131866050d02644ecc1a27e9288bd9e5d37254b149`
over `erl.py`, `erllaw.py` and the `eravon.terms/1` core, as §10.5
quotes it; the text's own sha256 at the naming is
`78aeaee6079122bcc0731520c1cadec38e5e8382556e153dec7a8e4d155b1123`; the fuzzing of
§10.1's deferral was not run.

## Textbook revision after the client design cross-check (2026-09-17)

Two clarifications, neither a predicate. §2.1: `pageHash` is "the `sha256`
of the window page's bytes as the browser loads them" (the law tests the
form alone, as §9 says; producers and browsers now share the function and
the range). §8: a window's routes for its other pages lie outside the
seven prefixes this law fixes, and where a later version names a prefix a
window has used, the version's prefix takes it and the window's route
moves. The core §10.5 names is unchanged, `criterion-digest-link.sh`
recomputing `0xe4b7e7dece52d3e8bb81e6131866050d02644ecc1a27e9288bd9e5d37254b149`;
`link-corpus/gen.py --seed 1` rebuilds the committed corpus and
`compare.py link` reports zero divergences and zero disagreements with the
predictions. The text's own sha256 after this revision is
`7f59a7a1c23f98bf9650a738e6a20aa78f90a2bae08a3d46aa4384f4475033ed`.

## Revision in place for consent by call (2026-09-23)

The seating and the reasons are in the `eravon.terms/1` record; this
record carries what concerns `eravon.link/1`.

### Round 0: the criterion against its own generator

A candidate's `sig` may be the empty string (a by-call candidate,
`OFFER_BY_CALL`, listed in the accepted answer), a receipt's `sig` may
be the empty string (`RECEIPT_BY_CALL`, carrying the four values of
`RECEIPT_BOUND`), a settlement offer's `sig` may be the empty string
(`SETTLE_BY_CALL`), and a pack refuses a key beside a by-call receipt;
§1, §2.3, §2.5, §2.6, §3.1, §3.2, §3.4, §3.5, §4.1, §4.2, §7.1, §7.2,
§7.4, §7.5, §8, §9 and §10.1 were written; the Status clause, §10.4 and
§10.5 returned to the unnamed forms. The criterion and the generator
(442 cases) agreed at seeds 1 to 4.

### Round 1: five readers and three implementations

The three implementations reached zero divergences on the 442-case
corpus at seeds 1 to 6 on their first measurement and probed beyond it
with no disagreement. The load-bearing findings on this law, none
changing a frozen output: `byCall` listed candidates "by index" where
the same answer vocabulary numbers a guild element by its own index and
a candidate by the candidate index, two readings that differ on a
by-call guild; the per-candidate gate of §2.5 ("passed both") never
opened after a by-call candidate, since `SIG_BY_CALL` is no pass; the
by-call candidate was the only by-call answer that did not name the
registry key it is checked against (`RECEIPT_BY_CALL` and
`SETTLE_BY_CALL` carry theirs); three sentences explained the empty
slot by the party's having no key, which `eravon.terms/1` §5.4 denies;
§3, §3.4, §3.5, §4.3, §7, §7.4 and §9 still spoke of a signature or of
an `init` anyone can send; §9 named the window and the seat as the
decider and accepted a registry answer of `false` as an answer; §4.3
cited a design-volume section that does not exist; "the bound order's"
in §4.2 and §10.1 could be read as excluding `RECEIPT_BY_CALL`, so that
a pack's two mismatch tests would be skipped on a by-call receipt;
`E_PACK_ORDER`'s carried set was open; §10.1 named no boundary the
revision made; §10.5 did not say that no digest is named here before
`eravon.terms/1` names one, and §10.1 had no deferral clause for the
fuzzing `galeed/1` §12.1 asks of a core, although the earlier naming
note invoked one; the general verdict sentence of the harness omitted
the values the bound and by-call verdicts carry.

Folded (Court commit after this record):

- §2.5 gates the next candidate on the present one having passed or
  answered `SIG_BY_CALL`, and lists a by-call candidate by its candidate
  index with its offer's digest of §2.4; §8 and the harness render
  `byCall` as `{candidate, digest}` pairs in ascending order.
- §2.6, §4.1 and §4.2 speak of the slot and not of the party's key; §3,
  §3.4, §3.5, §4.3, §7, §7.4, §7.5 and §9 admit the by-call forms in
  their prose; §1 names the three readings the predicates carry.
- §9 names the standard as the decider, the digest each reading is
  checked against, and the rule that a window shows a by-call candidate
  only where every endpoint answers that the registry holds it.
- §4.2 and §10.1 read the values §3.4 yields under either verdict; §8
  closes `E_PACK_ORDER` and `E_PACK_BIND`; §10.1 names the revision's
  boundaries, the sources an implementation is produced from, and the
  fuzz deferral; §10.5 says no digest is named before the terms law's;
  §4.3 cites design volume §7.3.
- Harness: the general verdict sentence lists the members each command
  renders.
- Corpus: 446 cases, with the instance before the third form under
  `bind-settle`, the key's form before `E_PACK_KEY_FORM` on a by-call
  receipt, and the two pack mismatches under `RECEIPT_BY_CALL`; a
  refusal case renamed from `ok_milestone_1_cap`, two notes corrected.

### Round 2: five readers and three implementations

The three implementations, updated from the revised texts alone, reached
zero divergences at seeds 1 to 6 on their first measurement (446 cases
per seed) and probed beyond the corpus (Rust 68,191 inputs including the
§2.7 prefix and base64url scan, Node 23,352, Python 25,564) with no
disagreement inside the measuring interface. Five seats (B, D, Node,
Rust, Python) found the same defect first: the round-1 fold of §2.5 had
appended the new gate ("has passed the first and has passed, or
answered `SIG_BY_CALL` at, the second") and left the old one in place
("untouched until the present one has passed both"), which read
literally stops at every by-call candidate. Also: §2.5 cited this law's
§8, the vocabulary, for the registry, and said "the key the registry
holds it under" where the answer carries a digest; §3.5 let a third
party send `init` on the buyer's consent alone, where the seller's slot
must also be a signature or a registry entry; three sentences still
spoke of a buyer without gas, of offers "already signed", and of a
by-call receipt "whose buyer signs nothing"; §9 stated the read-first
rule for candidates and not for the settlement's countersignature, so a
buyer who signs before the seller registers is bound until `expiry`
while the seller alone chooses whether to complete it. On the corpus:
`link_std_alphabet` was filed as a failing witness and passed at every
seed, its mutation a no-op, because ASCII JSON never encodes to `-` or
`_`; so an encoder or decoder on the standard alphabet passed the
committed seed.

Folded (Court commit after this record):

- §2.5 states the gate once; the by-call list is the digest a window
  looks up under `sellerKey` (§9, `eravon.terms/1` §8); §3.5 requires
  each party's consent to be a signature or a registry entry; the
  opening, §2.6 and §4.1 speak of consents where they spoke of
  signatures; §9's consent bullet extends the read-first rule to the
  buyer's side of a by-call settlement, and §7.5 cites it.
- Corpus: 448 cases; a payload whose `return` (a member no signature
  covers) makes its base64url carry both `-` and `_` at every seed,
  witnessed under `encode`, under `decode`, and refused under the
  standard alphabet; the README row files the three.

### Round 3: five readers and three implementations

The three implementations, updated from the texts alone, reached zero
divergences at seeds 1 to 6 on their first measurement (448 cases per
seed) and probed beyond the corpus (Rust 72,478 inputs including the
four transports' prefix and base64url scans, Node 25,779, Python
31,188) with no disagreement inside the measuring interface; two
mutants, a standard-alphabet decoder and a lenient platform decoder on
the receipt, pack and settlement transports, passed every seed. On this
law: §7.4's gloss on `SETTLE_BY_CALL` kept the route §9 closes (the
seller's own `settle` after a countersignature the buyer's side may not
give), and §9 gated the countersignature alone, leaving a buyer that
registers first the same free option; the opening still named the
receipt by who sends `init`; §3.5's fold dropped the payload the
address is computed from; §10.1 listed `galeed/1` by section number and
omitted the §6 preamble §1 invokes, and recorded the fuzzing over one
transport's scans. On the corpus, five seats: §2.7's base64 rule was
witnessed on the link's transport alone (one refusal each on the other
three, and no text carrying a URL character), a settlement that passes
§7.2 holding no byte that encodes to one; the §2.5 decision order was
witnessed by hand-picked pairs, leaving whole stages unpinned, and one
adjacency case (`p060`) was a no-op at every seed, its second fault
never reached.

Folded (Court commit after this record):

- §7.4 says the seller's consent by call is one the buyer's side takes
  only once the registry holds the digest; §7.5 and §9 gate every form
  of the buyer's consent, countersignature or registration, on the
  seller's registration, the seller completing an ungated one by its
  own `settle` or by registering later; the opening's receipt is "what
  a buyer hands back"; §3.5 reads "anyone who holds the payload it
  binds to"; §10.1 names the passages of `galeed/1` it invokes and
  records the fuzzing over the four transports' scans.
- Corpus: 491 cases; §2.7's base64 rule applied from one table to the
  receipt, the pack and the settlement under each of its three prefixes
  (the standard-alphabet twin of a text carrying both URL characters,
  the empty remainder, padding, a length of 1 mod 4, nonzero trailing
  bits), the receipt's URL side a passkey receipt whose client data
  carries `~~~~?>` under `receipt-encode` and `receipt-decode`, the
  pack's a payload `return`, the settlement's a refused document, its
  encode side unreachable; the §2.5 stage bridges (the ladder-wide rules
  beside milestone, expiry beside judges, the arrays' forms beside each
  other and beside mirrors, each element's table beside its repeat) and
  every element table's adjacent rows; `p060` rebuilt as the milestones
  form beside nonce, the range pair being structurally exclusive; the
  `pack-decode` notes cite §4.4; the `texts/` line names the four
  decoders.

### Round 4: five readers and three implementations

The three implementations, changed in no rule, reached zero divergences
on all three corpora at seeds 1 to 6 on their first measurement (430,
491 and 168 cases per seed), then probed beyond them with the decision
orders as the subject: Rust 94,174 inputs (16,760 two-fault inputs over
every ordered clause, 1,849 of them one step outside the corpus's
adjacent pairs; the naming note's fuzzing rerun at seeds 1 to 3), Node
43,111 (14,848 two-fault inputs; 101 swap mutants, of which the corpus
separated 59, 8 were exclusive and 34 passed every seed), Python
63,755 (324 mutants, the round-3 two now caught at every seed, three strip-before-decode mutants on the other transports caught by the newline cases). No disagreement inside the measuring interface, and no
place where the criterion departs from the text. The readers found 2
(A), 7 (B), 6 (C), 8 (D) and 7 (E) items and the implementations 0, 1
and 6; none changed a frozen output. 
On this law: the round-3 fold of §9 had dropped the endpoint rule from
the settlement gate, so one endpoint's word satisfied it; the
candidate gate bound the window alone where a company buyer uses the
seat and a buyer's registration before the seller's gives the seller
the offer's free option; §9 named the window and the seat and not the
seller's client, which reads the receipt's registration; §10.1's "it"
had two antecedents; §6.1's index rule read two ways where the lengths
and an element both differ, the criterion carrying `P`'s count. On the corpus, all seats: §2.5's element tables
and the ladder stage were witnessed by no adjacent pair, `p137`
witnessed a pair the README called exclusive, and the ladder count is
not exclusive with the rung tables; the native link prefix carried no
base64 table; §3.2, §3.4, §4.2 and §6.1 were witnessed one fault at a
time; the other transports lacked the byte outside the alphabet; Node's
lenient-decoder mutant of round 3 now diverges (15 cases per seed).

Folded (Court commit after this record):

- §9 states one bullet: the window, the seat and the seller's client
  read the registry and take a digest as registered only where every
  endpoint answers so; a buyer's side, window or seat, shows a by-call
  candidate, and signs or registers an order on it, only where the
  offer's digest is registered, and consents to a by-call settlement
  only where the settlement's digest is; a consent given before that
  binds the buyer until the offer's or the settlement's `expiry`. §2.5
  and §2.6 say "a buyer's side"; §10.1 names the passages this text and
  `eravon.terms/1` invoke and records the fuzzing over `hexdigits`; §6.1 states the priority (the
  payload's own count where the lengths differ, whatever the elements).
- Corpus: 566 cases; every adjacent pair of §2.5's element tables and
  ladder stage, `p137` renamed to the pair it witnesses; the base64
  table under the native prefix; every adjacent pair of §3.2, §3.4,
  §4.2 (a chain of fifteen) and §6.1 (by-call payloads against edited
  documents); a trailing line feed under every prefix; canonical before
  the root on the four documents; the lengths and an element differing
  together under both arrays; `unlock` above the `c` bound as misuse
  before any password test.

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
`0x3f28846eca5e22c4d74d76c7115adb29ee12d0267ecb5c31987ccc5653cfd177`
over `erl.py`, `erllaw.py` and the `eravon.terms/1` core, as §10.5 quotes it, §10.4 quoting the terms digest. The text's own sha256 at the
naming is `ba02607e50d38a5529354e58c99e2257b746606d0f31fa22153728936c3b9bbc`. The fuzzing of the deferral was run at the
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

Round 6 (2026-09-24), the confirmation of the generation change: the same
eight seats over the regenerated corpora at seeds 1 to 6. This law's text
changed only in Status, §10.4 and §10.5 (back to the unnamed form) and holds
no generation number; every digest it reads comes by reference to the terms
law's domains. Reader B recomputed the EIP-712 arithmetic from the text
alone and reproduced `caseId`, `instance`, `orderDigest`, the three
`byCall` digests and the settlement digest under version "1", none under
"3". The implementations' first measurement diverged on 171 cases per seed
(`byCall`, `bind-receipt`, `bind-settle` digests, and every answer
downstream of a payload whose seller signature is made under the new
domain), zero after the one constant; the probes and swap audits are
recorded with the terms law. Findings, none on a measured answer: the terms
law's §7 literal (fixed there); the conformance README and the design
volume asserting the withdrawn namings; two closed-boundary slugs in this
corpus's README naming no case at any seed (replaced by the slugs that do,
`p067_by_call_instance_mismatch` and `p010_missing` under `unlock`); a
trailing blank line the withdrawal added. Reader D's 720 digest-bearing
predictions recomputed from the text agreed at every seed.

Named on 2026-09-24 at Court commit after this record: the release digest
`0x9c905e751c1de44b52912451fd0123d82d12bf90909b67d7e0052f6ebb8e1dd0`
over `erl.py`, `erllaw.py` and the `eravon.terms/1` core, as §10.5 quotes
it, §10.4 quoting the terms digest. The text's own sha256 at the naming is
`1b6120f76864a7c297f0eb2e12f70d10aae014d84b023a84d8de86f09cc810d5`. The fuzzing of the deferral was run at the
naming, as the digest's note records.

## Revision in place for the open funding interface (2026-10-13)

The judge's direction of 2026-10-13: the law names no provider, a
funding entry is an open interface any provider can be offered through,
and a provider of a kind the law does not know can be added later
without a change to the frozen text. The draft of the amendment
(eravon-kit, `docs/exchange/法文改稿-eravon.link-1-就地修订-开放入金接口-2026-10-13.md`)
left seven points to the judge; the recommendation attached to each was
taken: the new sections sit at §2.8 through §2.14 and no later section
moves; `fundSig` is a member; extension entries are admitted; one to
four entries; five placeholder names; five refusal reasons; the answer's
own literal `eravon.fund-answer/1`.

### Round 0: the criterion against its own generator

`onramp` and its four provider keys left the payload; `fund` (one to
four entries, each a `service` address, a `template` with placeholders,
or an extension entry whose name this law decides and whose value it
does not read) and `fundSig` (the seller's signature over `fundDigest(P)`,
an EIP-712 struct of one `bytes32` under the logic's domain, or the empty
string of `eravon.terms/1` §5.4, read as `FUND_BY_CALL` with the digest
listed in the accepted answer) entered §2.1, §2.5 (the entries and the
signature's form after `return`, the signature itself after the last
candidate), §2.6, §8, §9 and §10.1; §2.8 through §2.14 were written: the
entries and the digest, the funding request `eravon.fund/1` and the
answer `eravon.fund-answer/1` with their closed member tables and
decision orders, `acceptFund`, `fillFund`, the obligations of the window
and of a funding service, and the meaning. The Status clause and §10.5
returned to the unnamed form; `eravon.dossier/1`, whose core embeds this
one, returned its Status clause, §7.4 and §7.5 to the unnamed form with
no other change. HARNESS-LINK gained `fund-check`, `fund-answer-check`,
`accept-fund` and `fill-fund`, and `check`'s accepted answer the member
`fundByCall`. The criterion (`erllaw.py`, `erl.py`) and the generator
(836 cases: 251 new, the seven `onramp` cases replaced) agreed at seeds
1 to 6; the dossier and terms corpora were unchanged and agreed. The
design volume (§2.1, §5.13, §6.1, §6.5) and the architecture book
(contract 14, S10, W4) say the open form.

### Round 1: three readers and three implementations

Three readers (the text, the corpus and harness, the ecosystem) and
three implementations (Rust, Node.js, Python), each written from the
texts and the harness alone. The implementations reached zero
divergences on the 836-case corpus at seeds 1 to 6: Rust and Node on
their first measurement, Python after one fix of its own base64url
table (20 transport cases per seed). Probes beyond the corpus: Rust
13,746 inputs, Node 9,356, Python 5,172, every ordered pair of two
faults in the request and the answer among them; one disagreement
class, found by Rust and Node alike: the criterion filled a `fill-fund`
template argument outside the skeleton charset where HARNESS-LINK and
§2.8 make it misuse. The text reader and the ecosystem reader found
independently that `fundDigest(P)` bound no order, so that a signed or
registered entry set served every later payload of that seller under
the logic, and a by-call registration, which the registry never
withdraws, never ended; the corpus reader found that an extension
entry's value, the one member of a payload whose nesting had no bound,
could make a payload §2.5 accepts that no funding request and no case
pack can carry, `canonical(b)` of the carrier answering `E_DEPTH`, and
listed the boundaries witnessed on one side only (a boolean extension
value, the count before the entries, the co-presence rule against the
tests before it, `fill-fund`'s own order pairs and the misuse of its
template and `decimals` arguments before the law's faults) and the
README slugs the acceptance and template cases, which write no file,
left dangling. The recomputation of every `fundByCall` digest,
instance, amount and filled URL from the text agreed at every seed.

Folded (Court commit after this record):

- §2.8: the type string is `EravonFund(bytes32 entries,bytes32
  termsHash,uint32 milestone)` and `s` covers the entries' canonical
  bytes, `terms.hash` and `word(order.milestone)`: a consent to the
  entries serves one document and one milestone, as an offer does, and
  a registered one ends in use with that document's offers; an
  extension entry's value nests no container at depth 125 or deeper, so
  that a payload nests at most 127 containers and its carriers 128; the
  template sentence states the placeholder as its own clause.
- §2.12 is defined over its domain alone (a skeleton string passing the
  template rule, two byte strings, an `owe` read as §2.9 reads the
  member, `decimals` in [0, 77]) and yields one verdict:
  `FUND_REQUEST_INVALID` carrying the §2.9 token of the first failing
  rule, or `FUND_FILLED`; every occurrence of every placeholder is
  replaced in one pass that does not read what it wrote; the remainder
  is written with zeros on the left to `decimals` digits before its
  trailing zeros go.
- §2.5 states that the funding signature is decided once every
  candidate has passed `payloadOffer` and passed, or answered
  `SIG_BY_CALL` at, `verifyOffer`; §2.6 and §2.14 say the by-call side
  (a changed entry set moves the digest off the registered one); §2.9
  states that the canonical tokens reach neither of its document
  carriers; §2.13 says the seller's key signs `EravonFund` beside the
  messages `eravon.terms/1` §5.5 names, that a window fills only a
  template and asks only a service its payload lists, and that a
  service allows a preflight and a `null` origin; §1 names the keys
  inside an extension value among the prose keys a passing document
  carries; §8 names `eravon.terms/1` §7 as the vocabulary
  `E_LINK_FUND_SIG` carries; §9 separates what the standard decides
  from what the registry alone holds for a funding digest, and says a
  request funds the price alone; §10.1 counts five empty-string slots
  and says its list is among the boundaries the opening clause requires;
  §10.3 names `eravon.fund/2` and `eravon.fund-answer/2`.
- Criterion: `fund_digest` takes the terms hash and the milestone; an
  extension value's depth is bounded; `fill-fund`'s template argument is
  tested for its charset. HARNESS-LINK states the charset of
  `<template>`, spells `<salt-hex32>` and `<iv-hex16>` as HARNESS-TERMS
  spells transport hex, and names HARNESS-TERMS for the order of
  argument tests. Corpus 870 cases: boolean extension values, the depth
  bound on both sides under `check`, `fund-check` and `pack-check`, a
  long and an uppercase `fundSig`, the dec256 bound of `amount` and
  `min`, the limit rule's remaining presences, `closed` before `v` on a
  refusal, the count before the entries, the co-presence rule after
  `v`, `return` and `pageHash`, `fill-fund`'s four order pairs, its
  template argument on both sides of the charset and each argument's
  misuse before the law's fault; `accept-fund` and `fill-fund` cases
  carry a `slug` member, two acceptance pairs are renamed, and the
  README names every case; `bcov.py` takes a corpus directory.
- `eravon.dossier/1` [C] §5.1 reads `Consented` under a fourth pair,
  the seller and `fundDigest(P)`. The design volume, the architecture
  book and the CHELEK concept volume's consent-original rule say the
  open form (the registered funding digest, the first client release
  taking crypto payment alone with `fund` verified and no entry shown,
  the funding service rather than a card channel, the threat row for a
  reposted link with another entry). Left as it stands: `return` admits
  the bare `http://` prefix, a boundary the frozen text already drew.

### Round 2: the same six seats on the round-1 text

The three implementations, changed only where the fold changed the text
(the digest's two new members and the depth bound), reached zero
divergences on the 870-case corpus at seeds 1 to 6 on their first
measurement after the change. Probes: Rust 18,858 inputs (the digest
under both forms and seventeen wrong digests, the depth bound at 123 to
129 inside every carrier, the template argument's every byte, the
remainder at every `decimals`), Node 6,103 (3,096 constructed, 3,007
mutations of corpus inputs over every command), Python 8,413; the corpus
reader recomputed 642 values and ran 1,932 probes from the text alone.
No disagreement anywhere, no place where the criterion departs from the
text, and no finding against a predicate: every seat held the round-1
fold right. What remained was prose and corpus witnesses. On the text:
the digest's formula read two ways on the scope of its inner
`keccak256` and its type string wrapped across a line inside backticks;
`sellerKey` in §2.5's funding test had no referent at the payload's
root; §2.12's "the first of" read two ways; §2.14 claimed more than the
digest binds (an entry set the seller drops stays consented on its own
document and milestone, a signed one on every payload it is spliced
into and a registered one for good); "a later document" in §2.8 let two
documents define one extension name; §1's "any key" inside an
extension value read as every key; the extension name's lower bound
could not be reached from below, the empty key being `canonical(b)`'s;
§10.1's list named no round-1 boundary. On the corpus: every funded
payload sat at milestone 0 under one terms hash, so a little-endian
milestone word passed; `FUND_ANSWER_INVALID` before `FUND_REFUSED`, the
far side of the depth bound inside the carriers and under `fill-fund`,
the `galeed/1` §3 tokens `fill-fund`'s byte strings carry, the receipt's
form before its binding under `fill-fund`, `pageHash` before `fund`,
every placeholder's second occurrence and the two escaped bytes of the
template charset, `max`'s dec256 bound and both limits at their largest,
the `url` prefix's case, and one README slug naming a note rather than a
file. On the ecosystem: the design volume's §3.3 still said the registry
served three digests only, the seller side of the design and the
architecture book carried no first-release clause, and W4, S1, S10 and
K7 kept channel wording.

Folded (Court commit after this record): the sentences above, each as
the seat wrote it or as two seats' proposals were joined (§2.8's digest
as a formula with its type string in a code block, §2.14's exact claim,
"a later document of this family, one document for each name", "at most
thirty-two bytes", "any key that is `_md` or ends in it"); the criterion's
name test loses its unreachable lower bound; `compare.py` labels a
shared-path case by its slug; corpus 892 cases (milestone 1 by call and
signed, a signature made for another milestone and for another terms
document, the depth bound's far side under `fund-check`, `pack-check` and
`fill-fund`, the pretty-printed payload and receipt under `fill-fund`,
the receipt's form before its binding there, `pageHash` before `fund`,
every placeholder twice with zeros on both sides of the remainder, the
quote and the backslash and the space in a template argument, `max`
above 2^256 − 1 and with a leading zero, both limits at 2^256 − 1
passing and carried, the `url` prefix in uppercase, the answer's fault
before `FUND_REFUSED`); the design volume, the architecture book and the
company concept volume's highlighting rule say the fourth registered
digest, the first-release boundary on the seller's side, and the funding
service in place of the card channel. Confirmation: the three round-2
implementations, unchanged, ran over the 892-case corpus at seeds 1 to 6
with zero divergences and zero disagreements with the predictions.

Named on 2026-10-14 at Court commit after this record: the release digest
`0x1fde7250ea1830cd31ec5d3d4dd54404276936c2725308711e88e639b6b9898c`
over `erl.py`, `erllaw.py` and the `eravon.terms/1` core, as §10.5 quotes
it, §10.4 quoting the terms digest unchanged. The text's own sha256 at the
naming is
`672100ecd20137ac58fd037b9988809c0b9decbd7dee7366c180d6fa004c03c2`. The
fuzzing of the deferral stands as the digest's note records: the
transport and `hexdigits` code is the code the 2026-09-24 fuzzing ran
over, and the Node seat's mutation fuzz over every command stood beside
it at this naming. `eravon.dossier/1`, whose core embeds this one, was
renamed by the same act (its record).


## Revision in place of 2026-10-17: five documents on the board, the link a pointer

The judge's directions of 2026-10-17: the seller posts a listing (`eravon.listing/1`, §2: the whole terms document, `order` with `sellerKey`, `termsHash` and the candidates' signatures or registered consents, the guild elements' `seq` and `panelHash`) on the chain's `EravonBoard` under the key `termsHash`, and the link a buyer receives is a pointer, `eravon://open?e=` followed by the base64url of `{v, chainId, tx, hash}` (§3), resolved against the fetched document by §3.3's verdicts; the receipt carries `openBy` (§4); the pack carries `link`, `listing` and the private sections' texts as `attachments` (§5); the settlement offer is posted under the key `caseId` (§8); the funding entries and their four commands left; the key (§6) and the bind (§7) stand, the bind comparing `quoteHash` where it compared quote bytes. §11.4 embeds the `eravon.terms/1` core and §11.5 names this one, both in the unnamed form until the convergence and the contracts' acceptance gate close.

### Round 0: the criterion against its own generator

The five document sections were written with their closed member tables and decision orders, §9's token table, §10 (the client's checks at the act: the standard's identity, the covenant entry and its quote bytes, the panel record, the feed, the seller's balance and allowance against P + LF, `revoked` false, the private texts against their hashes), and §11. HARNESS-LINK gained `link-check`, `encode`, `decode`, `resolve-link`, `receipt-encode`, `receipt-decode`, `settle-check`, `bind-settle`, and `pack-check`'s `attachments` count. The criterion (`erl.py`, `erllaw.py` over the terms core) and the generator (484 cases: listings, links, documents, receipts, packs, texts, terms, settles) agreed at seeds 1 to 6.


### Round 1: five readers, three implementers

Five readers (A this law's text, B the standard against the contracts, C `eravon.link/1`, D `eravon.dossier/1`, E the four laws together with the plan, the tooling and MIKNAH's citations) and three implementers (Rust, Node, Python, each written from the texts and the harnesses alone, `ert`, `erl` and `erd` in one program each) read the round-0 texts. The implementers measured zero divergences from the criterion and zero differences from prediction at seeds 1 to 6 on their first measurement of each law, 18 runs of 18 for each, and probed the criterion beyond the corpus (Rust 27,920 terms, 18,871 link and 7,251 dossier inputs; Node 21,772, 21,293 and 12,203; Python 21,581, 39,764 and 4,106) with no disagreement class; one rendering class appeared in Rust's first link run alone, carried faults written with an `ok` member, which the harnesses then fixed (a fault or a verdict under `inner` carries no `ok`, at every depth). Four audit seats read the contracts in the same round (security, the eight vulnerability classes, the adversarial targets, the fuzz and invariant campaign); their findings changed the contracts and the standard's text and touched this law where noted below.

Reader C found 20 items (10 load-bearing) on this law, E 32 (6) across the four, A 20 (8), B 37 (17), D 21 (13). Folded into this law: the Status paragraph; §2.1 the `proxyCodeHash` row; §2.3 the added members (`sig`, `seq`) required members of the element and of its closed list, their forms tested after the inherited table's rows; §2.5 and §9 the two window tokens renamed `E_LISTING_DELIVERY_WINDOW` and `E_LISTING_CONFIRM_WINDOW`; §2.6 what the listings of one document share (every member but a guild element's `seq` and `panelHash` and a candidate's `sig`); §3.2 the base64url alphabet spelled out; §4.4 and §4.5 the `buyerMayOpen` gate (where false the seller alone opens and a by-call buyer consents by registering); §5.1 and §5.2 "equal to" on `chainId`; §5.3 the attachment predicate as equality of the text's sha256 with the document's `goodsHash` or `deliveryHash` (the 66-byte spelling anywhere in `inline_md` admitted a covenant text or a judge's word; plan item 7's gloss intended the segments); §6.4 the password as the UTF-8 of the typed password's NFKC form; §7.1 the outcomes of `bind` over a passing listing; §9 the carried values that are unreachable named, the `eravon://` literal sentence (any other address a client handles is the client's own); §10 the key-topic check on a `Posted` log (a listing is shown only where the log's key is its `terms.hash`), a guild's word as the `text_md` sha256, the fee priced from the feeds where `feeUnit` is nonzero and priced again at `init`, the hashed segments, the surplus credited and not pushed at `init`, the consent by call's exposure (the order's consent binding until the earlier of `openBy` and `expiry` and until `cancel`, the settlement's until its expiry or the terminal); §11.1 the binding verdicts; §11.4 the re-pin by this law's own naming act; §11.5 "computes". HARNESS-LINK: the `c` bound decided after §5.2 as the one exception to the adopted misuse rule, nested `inner` without `ok`, every address, digest and byte string written as `galeed/1` §1's hex functions write it. The criterion followed: the listing's guild check against the seller's key, the named hashes of a document, the attachment by equality. The corpus grew from 484 to 539 cases (the renamed tokens; `link_padded` padded unconditionally and `link_empty_file` beside `link_empty`; the closed-list extras of every document kind; a string, zero and seller-equal judge, a judge on another chain, a listing whose pointers name another chain, a string guild, a zero guild pointer, a zero guild word, string, seller-equal and summed rungs; the `galeed/1` §3 tokens on every document kind; `LINK_SETTLE` before `LINK_CHAIN`; a prose digest and a covenant text as attachments; a case mismatch before the attachments; a by-call key as a string; the misuse table); zero differences from prediction at seeds 1 to 6. The standard's reader rule of §2 changed in the same round (a reader keeps the listings whose `terms.hash` equals the key and the case's `termsHash`, takes the document from any, and reads no unsigned member from a listing found by key), which this law's §10 already required of the client.


### Round 2: the same seats over the revised texts

The same eight law seats re-read the round-1 texts (Court 1e643b19): readers A 13 items (3 load-bearing), B 29 (15) with 6 security observations, C 19 (8), D 19 (9), E 19 (5, the plan's 37 items all in the texts, round-1 items 31 of 32 landed and one withdrawn, every tooling check passing, a dry run of the naming act passing all three digest scripts' guards), and the three implementers, each continuing its round-1 program, zero divergences and zero differences from prediction at seeds 1 to 6 on the first measurement after bringing the programs to the revised sentences (the round-1 programs diverged on the revised cases alone), with 58,937 (Rust), 59,674 (Node) and 71,757 (Python) further probe inputs and no disagreement class. The audit seats of round 2 (security, the eight classes, the adversarial targets) found no finding above medium; their findings (the re-arm as a re-roll by delay, a surrender cheaper than a lower rung, a settlement past T2 stripping the judge's P, a draw window shorter than a block, the arming block's producer, uncapped pushes and feed reads, a codeless token at the gate, a Refund() signed earlier landing as the surrender) changed the contracts and the standard and are recorded in the standard's CODEHASH.md paragraphs; the fuzz seat's own finding on the round-1 fold, a double credit at the expiry branch and cancel, was fixed before the round's audits began.

Folded into this law: §2.2 the amount's bound reaching the listing through §7's `BIND_ORDER`, the document carrying it, and §2.4 saying so; §2.5 and §9 naming `eravon.terms/1` where "that law" and "that section" were ambiguous; §4.4 and §4.5 the `init` that opens the case coming from `sellerKey` alone where `buyerMayOpen` is false, the refund `init` of the standard's §4 step 3 from anyone; §6.4 a client refusing a password whose code point its Unicode version leaves unassigned; §10 the quote key spelled `<chainId>:<codeHash>:<token>` with its citations, the guild's latest record tested as `init` tests it (unexpired, every word this `amanahHash`, a pool left once the parties' keys are removed), and the fetching sentence (a desk finds the listing by the key, a dossier's reader tests it at `posted.tx`); §11.1 `LISTING_INVALID` among the binding verdicts; §11.4 the embedded terms core as the one that law's §10.5 named when this section was pinned, `naming.py`'s anchor and replacement changed with it. HARNESS-LINK: `resolve-link`'s `document` as the accepted answer its command renders, `"ok":true` included, the one `ok` inside a verdict answer; `E_LISTING_SIG`'s `index` the candidate index of §2.3's numbering; every hex outside the key object of §6.1 as `galeed/1` §1 writes it, `lock` writing that object's members as `hexdigits`; `<password>`'s bytes as `pw` itself, no command applying §6.4. The corpus grew from 539 to 566 cases (`order_amount_over_cap` and its expiry twin; §7.1's order by each adjacent pair; the listing's own count where the lengths differ, three cases; the inherited guild table under the listing's `sellerKey` and `chainId`, two cases; presence before closure before the rows where the added members meet the inherited tests, four cases; `E_JSON` and `E_VALUE_CHARSET` on the link, the pack and the settlement offer and `E_JSON` on the receipt; a key locked under U+212B opened under it and refused under U+00C5, and `lock` under U+212B; `lock` with a seventh argument); zero differences from prediction at seeds 1 to 6; the README's rows follow, the `RECEIPT_BY_CALL` passes list corrected and the misuse paragraph saying what is witnessed. The standard's §2 reader rule dropped the consents clause (the hash authenticates the document and no registry is read), names a pack's `link.tx` and `link.hash` and a dossier's `posted.tx` and `posted.hash`, and takes neither `seq` nor `panelHash` from a listing found by key while reading a judge's `source` from the document.


### Round 3: the same seats over the round-2 texts

The same eight law seats re-read the round-2 texts (Court c25210b0; HEAD moved to 74b81962 during the round, a commit touching one test file alone): readers A 7 items (0 load-bearing), B 7 (4) with 2 security observations, C 12 (3), D 16 (8), E 10 (1; the plan's 37 items all in the texts or withdrawn by the plan, round-2 items 19 of 19 landed, every tooling check passing but naming.py's terms Status anchor and the foundry.toml comment, a dry run of the naming act passing all three digest scripts' guards), and the three implementers, each continuing its program, zero divergences and zero differences from prediction at seeds 1 to 6 on the first measurement after bringing the programs to round 2's two seat rules, with 78,476 (Rust), 74,293 (Node) and 88,419 (Python) further probe inputs and no disagreement class on the measuring interface. The audit seats of round 3 (security, the eight classes, the adversarial targets, the fuzz seat) found two findings of low severity, a seller's `refund` sent while Claimed and landing behind the buyer's `dispute` executing as the surrender, and a `refund` while Claimed under a seated seat returning P to the seller so that a refund and a payment off the instance completed the trade with P stripped, and informational items beside them; the findings changed the contracts and the standard (`refund` refused while Disputed at or before T3's end, the surrender row gone, `refundBySig` testing the signature before the phase, a refund while Claimed under a seated seat paying P to the judge as T2's lapse would) and are recorded in the standard's CODEHASH.md paragraphs together with reader B's two observations on return data copied beyond a gas allowance, now copied to a fixed size by the verifier's reads and not at all by a push.

Folded into this law: §2.7 a reader authenticating a listing by its content and the signatures it carries, a by-call candidate's consent being the registry's to say (a buyer's client reads it before it shows the candidate, a reader of an opened case does not read it); §6.4 a decomposition or a combining class; §9 the four unlock tokens without an index; §10 the pool excluding the zero address, a pool candidate's `offerWindow` reaching 256 of the chain's block intervals and the chain's block hashes coming from consensus, and the fetching sentence keeping every `Posted` log whose `hash` topic is the link's and resolving where one carries the `key`; §11.1 every verdict of the three bindings. HARNESS-LINK: `BIND_JUDGES` and `BIND_GUILDS` carrying the listing's own count where the lengths differ; a password that is not valid UTF-8 outside the measuring interface for a candidate whose runtime decodes its arguments; `lock`'s answer without `ok`. The corpus grew from 566 to 592 cases (the other half of eleven listing rows, two receipt rows and the settlement's `instance`; the order across guild elements and across attachments; the passing edges, the windows at 2^32 − 1, sixteen rungs, `buyerBps` 10000, a key with `c` 1, plaintexts 1 and n − 1, and the transports at the other residues modulo 4); zero differences from prediction at seeds 1 to 6; the README's rows follow, its receipt charset witness named `sig_passkey_cdj_charset`. The standard's §7b authenticates a document by its content and the signatures it carries, the registry read by a party's client before it acts on a by-call slot and by no reader of an opened case.


### Round 4: the same seats over the round-3 texts

The same eight law seats re-read the round-3 texts (Court ecf085ed): readers A 7 items (1 load-bearing), B 9 (1) with 1 security observation, C 7 (2), D 9 (3), E 8 (1; the plan's 37 items all in the texts or withdrawn by the plan, round-3 items 10 of 10 landed, every tooling check passing but an indentation error the round-3 fold had put into naming.py), and the three implementers, each continuing its program, zero divergences and zero differences from prediction at seeds 1 to 6 on the first measurement (the round-3 programs already met every revised sentence; Node read the previous path sentence to the letter, a `/` always, and found the one case where that and the criterion's joining rule decide apart), with 88,343 (Rust), 83,648 (Node) and 34,326 (Python) further probe inputs; one disagreement class, Node's, on a `<dir>` ending in `/` at the host's path-length limit, which the harness and the law now decide the criterion's way. The audit seats of round 4 (security, the eight classes, the adversarial targets, the fuzz seat) found no finding above informational, their informational items being one sentence of the standard's §5 that still described the round-3 refund, a blank line splitting §8's terminal table, and two stale rows of the design volume; round 4 is the first of the two clean rounds the judge's ruling of 2026-10-17 asks of the contracts, and the code and its identity are unchanged by this fold.

Folded into this law: §2.5 the repeat tests of the judge and guild tables run once every element has passed; §2.7 the consent of a by-call candidate as the standard's to decide, the registry read before any `init` by the buyer's client and by no reader of an opened case for the listing, what `init` admitted being the chain's record; §5.2 the reason no carried fault is a `galeed/1` §3 token (the pack passed `canonical(b)`), and §9 saying so for the three carriers; §10 the nominal block interval. The corpus grew from 592 to 618 cases (the order across rungs, within a rung, between the two ladder-wide rules, a guild element's closure before its rows, the settlement offer's presence before closure, and the repeat test run after every element passed, `judge_dup_before_later_row` and `guild_dup_before_later_row`; `encode` at segment lengths 2 and 3 mod 4 and a passing `decode` at 3; `receipt-encode` over a receipt at 2 mod 4 and over the one whose bytes encode with both URL characters; `order_amount_max`; the counter carrying at 64 and at 128 bits on `lock` and `unlock`; the uppercase spelling of `proxyCodeHash`, `terms.hash`, `panelHash`, a receipt's `sig` and `buyerPayout`, a link's `hash`, a pack's and an offer's `instance`); zero differences from prediction at seeds 1 to 6; the README's rows follow. The generator's counter now wraps modulo 2^128 as §1 and the criterion do.


### Round 5: the same seats over the round-4 texts

The same eight law seats re-read the round-4 texts (Court ebcd2eb5): readers A 5 items (0 load-bearing), B 6 (1) with 1 security observation, C 3 (1, in the corpus alone), D 11 (2, one in the corpus alone), E 2 (1; the plan's 37 items all in the texts or withdrawn by the plan, every round-4 item landed, every tooling check passing), and the three implementers, each continuing its program, zero divergences and zero differences from prediction at seeds 1 to 6 on the first measurement (Rust 5 text items, Node 3, Python 3; Node's one round-4 class is gone, the text now taking the criterion's join rule), with 107,762 (Rust), 91,329 (Node) and 24,050 (Python) further probe inputs and no disagreement class. This is the closing round of the law texts, as the rounds of 2026-09-24, 2026-09-23 and 2026-10-14 closed: the three programs changed in no line and reached zero divergences on their first measurement, and what the readers reported was folded in one pass (three sentences the chain reader of the dossier law decides, one sentence of the standard's §8 on a token outside its rule, two corpus witnesses the programs already passed, and prose), the judge ruling on 2026-10-17 that the earlier standard governs; the three round-5 programs, unchanged, were then measured again over the folded corpora (571, 647 and 293 cases) at seeds 1 to 6, 54 runs of 54 with zero divergences and zero differences from prediction. The audit seats of round 5 (security, the eight classes, the adversarial targets, the fuzz and invariant campaign with a 55-invariant handler, a 2,575-run matrix of every pair of acts in both orders and two deep campaigns) found no finding above informational, their informational items being text and tooling (a starved or failing balance read after a push, the landing order of two acts that reach one instance, two client-rule races, the replay harness's step 9a against the guild track's second-block draw, and the design volume's rows of the earlier money model), all folded; round 5 is the second of the two clean rounds the judge's ruling of 2026-10-17 asks of the contracts, which closes the contracts' gate, the code and its identity unchanged since the round-3 fold.

Folded into this law: §5.2's closing sentence naming the tokens `E_PACK_LINK`, `E_PACK_LISTING` and `E_PACK_RECEIPT` carry as never themselves `galeed/1` §3 tokens, an `E_LISTING_TERMS_DOC` under `E_PACK_LISTING` carrying the terms document's token, those among them. The corpus grew from 618 to 647 cases: `canon_pretty_before_missing` on every kind (canonical before presence), `receipt_invalid_before_terms_mismatch`, `unlock_pack_fault_before_no_key`, `settle_invalid_before_chain`, and one uppercase spelling of every remaining hex member (a judge's address, `amanahHash`, `quoteHash` and `sig`; a guild's `amanahHash`, `quoteHash` and `sig`; the listing's `logic` and `order.sellerPayout`; the receipt's `termsHash` and the five hex members of a passkey object; the pack's `caseId` and the key's `iv`, `ciphertext`, `salt` and `mac`; the settlement offer's `sig`); zero differences from prediction at seeds 1 to 6; the README names `settle_bad_chain5` beside `bad_listing_chain_5`, fills the three empty passes cells, states the uppercase rule once, and places the pack's fault before `E_UNLOCK_NO_KEY`.

Named on 2026-10-17 at Court commit after this record: the release digest
`0x3e6ba9d8aa3171dde8bd8085d44654cdfb216c8b2f374d6938ddc01c2309b2d5`
over `erl.py`, `erllaw.py` and the `eravon.terms/1` core, as §11.5 quotes it, §11.4 quoting the terms digest named by the same act (`0x1b94150b6c5726c6f16992c9dd4a57279e5fac51be148ccc62466013afb95a52`). The text's own sha256 at the naming is
`ea0ac1d075ef96322c4b8e296598e88aab90f704e09bd1fd962436bd1cdf988d`. The fuzzing of the deferral was run across the five rounds of this revision, as the digest's note records. `eravon.dossier/1`, whose core embeds this one, was named by the same act (its record).

## Revision in place of 2026-10-18: the seller's deposit address

The judge's direction of 2026-10-18 (the plan `修法方案-公司卖家押金地址sellerFund-2026-10-18`): a seller that is a company keeps its capital in its wallet and holds nothing at its own address, so a deposit pulled from `sellerKey` could never be found there and no company seller could open a case; the order now names `sellerFund`, the address the deposit is pulled from and returned to, which is the seller's key or an address that bound itself to the seller on the logic (`acceptFunding`), the bond tested at the gate after the consents whether or not the deposit is zero; and a buyer that consents by call counts `openWindow` from its export plus the earliest landing of its consent. The three cores were recast (`Offer` gained `sellerFund` after `sellerPayout`, so every offer and order digest changed), the naming sections of the three laws returned to their unnamed form, the standard changed with the contracts (three registries, `pull` over the fund, five parties, `FundUnbound`, the terminal split computed on the verifier for room: logic 24,131 bytes, verifier 9,450; a new identity table), and the three laws are named again after the rounds below.

### Round 0: the criterion against its own generator

§2.2 the `sellerFund` member; §2.4 `listingOffer` drawing it from `order`; §2.5 eleven members and `E_LISTING_FUND` after `E_LISTING_PAYOUT`; §7 `BIND_ORDER` comparing it; §9 the token; §10 the client's check of the bond and of `sellerFund`'s balance and allowance. The corpus grew from 647 to 653 cases (`order_fund`, `order_fund_zero`, `ok_company_fund`, `fund_upper`, `adj_order_payout_before_fund`, `adj_order_fund_before_amount`), zero differences from prediction at seeds 1 to 6; the README's rows follow.


### Round 1: the twelve seats over the checkpoint

The twelve seats read the checkpoint (Court 849ad3cb): readers A 8 items (0 load-bearing), B 13 (1), C 4 (2, both in the corpus alone), D 8 (1), E 17 (2, the same two as B's and D's; of the plan's 41 rows 37 in the texts, 1 done otherwise, 1 pending the renaming, 2 the FUZZ seat supplies), and the three implementers, each brought to the revised type string (Rust 28 lines, Node and Python ten lines in three files; before the change 386, 330 and 102 divergences across the three laws, the digests of the old type string), zero divergences and zero differences from prediction at seeds 1 to 6 (Rust 7 text items, Node 7, Python 9), with 123,154 (Rust), 98,900 (Node) and 40,710 (Python) further probe inputs and no disagreement class. The four audit seats over the checkpoint found nothing above informational: SEC 3 informational items (426 proofs in its tree, the whole machine against round 5's at 300 fixed seeds and 5,000 fuzz runs equal), VULN 2 (360 proofs, the split equal to the old in-logic arithmetic over 50,000 runs, thirteen unbound addresses each refused), ADV 3 (369 proofs, 46,656 ordered pairs of acts, 987 fault combinations of the gate's order), FUZZ 5 (sixty invariants under a five-way draw of `sellerFund`, two campaigns of 256 runs at depth 500 at seeds 0x6a and 0x6b with 128,000 calls each, 258 paths reached, eight mutants of the revision caught), the informational items all in the texts and the comments; this is the first of the two clean rounds the contracts' gate asks. Four items bore load, two of the revision and two of the link corpus from before it: the standard's §10 naming the seller's key as the `Swept` creditor where the deposit now returns to `sellerFund`; the dossier law's §5.1 `fundBind` row placing the bond before an `Opened` the instance may never emit; the link corpus leaving nine members of §7.1's three comparison lists unwitnessed alone; and §6.1's nested objects without their closure cases; a reader skipping any of the last two passed every seed. Everything was folded in one pass with the text items, the code's bytes unchanged (`identity.sh` agreeing with CODEHASH.md after the comments and one local name were corrected); the three round-6 programs, unchanged, are measured again over the folded corpora in the next round, the closing round of the texts if that first measurement is zero.

Folded into this law: §2.4 citing `eravon.terms/1` §4.3 by name; §4.1's `openBy` row (set at signing or, for a receipt by call, at export) and `seq` row, §4.4 and §10 reading "when it set `openBy`" (`eravon.terms/1` §5.5) for "at signing". The corpus grew from 653 to 668 cases: each member of §7.1's three comparison lists not witnessed alone before, the listing re-signed over its own values by the key it names so that a reader skipping the member accepts it outright (`bind_order_seller`, `bind_order_payout`, `bind_order_fund`, `bind_order_delivery`, `bind_order_confirm`, `bind_order_expiry`, `bind_judges_quote`, `bind_guilds_pointer`, `bind_guilds_amanah`), and §6.1's nested objects each no object, a member beyond, `mac` missing (`key_crypto_string`, `key_cipherparams_string`, `key_kdfparams_string`, `key_crypto_extra`, `key_cipherparams_extra`, `key_crypto_missing_mac`); zero differences from prediction at seeds 1 to 6; the README's §2.5 binding row and §6.1 row name them.


### Round 2: the twelve seats over the round-1 fold

The twelve seats read the round-1 fold (Court 90c01f71): readers A 5 items (0 load-bearing), B 7 (0), C 4 (2, both in the corpus alone), D 3 (0), E 8 (0), node 6 (1, in the corpus alone), and the three implementers, each with its round-6 program unchanged, zero divergences and zero differences from prediction at seeds 1 to 6 on the first measurement (583, 668 and 300 cases; 18 runs of 18 for each of Rust, Node and Python), with 145,516 (Rust), 117,318 (Node) and 73,588 (Python) further probe inputs and no disagreement class (Rust 5 text items, Python 7). The four audit seats found nothing above informational: SEC 1 informational item (its round-6 proofs at fresh seeds, the fund invariant widened to every placement of `sellerFund` at 512 runs of depth 128, the grief scanner for a company seller), VULN 1 (370 proofs in its tree, class 4 taken deeper with an opcode census, look-alike proxies, delegation and a clone logic, the split equal to the round-5 arithmetic over 100,000 runs), ADV 2 (1,679,616 ordered triples of acts with a third actor in the same block, §7a sentence by sentence, nine mutants caught), FUZZ none at any severity (the folded handler at 256 runs of depth 500 twice at fresh seeds, 60 of 60; twelve mutants of the revision caught; the handler extended to one fund drawn on by several instances of one seller and to a judge that funds the case it judges, 63 invariants and 295 paths, folded under `test/`); this is the second consecutive round with nothing above informational, and the contracts' gate is closed. This is the closing round of the law texts, as the rounds of 2026-09-24, 2026-09-23, 2026-10-14 and 2026-10-17 closed: the three programs changed in no line and reached zero divergences on their first measurement, and what the readers reported was folded in one pass. The items that bore load were corpus witnesses alone, the texts and the criteria deciding every case already: the link corpus left §6.1's nested objects without a case for each member absent and the receipt's passkey object without a member beyond its six, §7.1's guild array without its index rule on two guilds, and §7.1's ladder without a case on a rung's shares or the ladder's length; a reader departing from the text there passed every seed. The three round-7 programs, unchanged, were then measured again over the folded corpora at seeds 1 to 6, 54 runs of 54 with zero divergences and zero differences from prediction; ten criterion mutants aimed at the new cases each fail on the cases written for them.

Folded into this law: §10's checks at the act timed by each client's own act (the buyer's when it sets `openBy`, the seller's when it signs or posts an offer, each again before an `init` it sends). The corpus grew from 668 to 692 cases: every member of the key, `crypto`, `cipherparams` and `kdfparams` absent alone, drawn from the key object itself (`key_missing_*`, `key_crypto_missing_*`, `key_cipherparams_missing_iv`, `key_kdfparams_missing_*`), the receipt's passkey object with a member beyond its six (`sig_passkey_extra`, and in a pack `receipt_passkey_extra`), the guild array's index rule over two guilds (`ok_two_guilds`, `bind_guilds_one_differing_vs_two`, `bind_guilds_second`, `bind_guilds_first_differing`), the ladder's shares and length (`bind_order_ladder_bps`, `bind_order_ladder_prefix`), and the fund's key signing a candidate (`sig_by_fund`) and a settlement offer (`sig_by_fund` under a company's pack, beside `ok_company_fund`); zero differences from prediction at seeds 1 to 6; the README's rows name them.

Named again on 2026-10-19, at the Court commit after this record: the release digest
`0x29ff5e4651ee99cdc94521157ef96ad1e943473bea645cc24a569fb4429c80ef`
over `erl.py`, `erllaw.py` and the `eravon.terms/1` core, as §11.5 quotes it, §11.4 quoting the terms digest named by the same act (`0x5cb4a7f34c2a5b46b290614165c32a5ceb95b928b7f355ca3e467d0143b1945e`). The text's own sha256 at the naming is
`4db6d2f2c4865233b1a0144696ea4b0b6fda98995247f7703f8b4aa4b04ffef3`. The fuzzing of the deferral was run across the five rounds of the revision of 2026-10-17 and the two of this one, as the digest's note records. `eravon.dossier/1`, whose core embeds this one, was named by the same act (its record).

## Revision in place of 2026-10-24: the succession book, the venue's new interface, and two of the family's own

The judge's revision of 2026-10-24 (`逐条修订稿-换钥簿与场所新接口及本家两项-2026-10-24`, 186 items over the four ERAVON laws, with the full revised texts beside it): the family reads every key through GALEED's succession book (`galeed.roster/1` §4), so that a party's, a judge's and a guild's key is its record's current key on the book the standard's code names; the standard follows the venue interface's new form (`galeed.venue/1`: `partyRoles`, `outcomeOf`, `advanceOf`, `successionBook`, `routeGas`, the seat's two credited fields, a panel's router called with exactly `routeGas()`, one panel book per chain filing under the guild's root); and two items of the family's own: the quote loses its payout word, a judge registering its own payout on the instance, and `lapseDefault` admits 1, an even split at a lapse; the passkey (P-256) form is removed from the family. For this law: §1 the book's replay (`declare`, `accede`) and `currentAt`; §6 identity, new: the record sets, a document's position and source (`self`, `opened`, `revoked`), its standing (`stands` or `attempt`), `place`, `pay-check` (`PAYABLE`, `SELLER_KEY_RETIRED`, `UNPLACED`, `LISTING_INVALID`) and `history`; the keystore and its `unlock` and `lock` removed; the receipt's passkey object gone with the form. The naming sections §11.4 and §11.5 returned to their unnamed form.

### Round 0: the criterion against its own generator

The items applied in place with the other three laws (Court 9ab8f451). The criterion: `erllaw.py` imports the terms core; the keystore code removed; the identity section new (the record sets, the book's replay, `current_at`, documents and their positions, standing, `place`, `pay_check`, `history`); `erl.py` gains `place`, `pay-check` and `history` and refuses a misused records file. The reading taken where the harness, not the text, fixes a shape, for the first round's readers to check: `history` writes each document's `source` inside its `position` object, as `place` writes it, with no separate member. HARNESS-LINK, the generator (its identity cases from a model of their own) and the README follow. The corpus went from 692 to 653 cases at seed 1 (the passkey and keystore cases removed; `closed_key`, `sig_object`, the receipt's `canon_value_charset`, `sig_by_moved_key`, and the `records/` identity cases added); zero differences from prediction at seeds 1 to 6. Over the old corpus the old and the new criterion differ in 150 answers, each one of them the revision's.

### Round 1: the twelve seats over the round-0 checkpoint

The twelve seats read the round-0 checkpoint (Court 16c772f9): readers A (this law) 9 items (0 load-bearing), B (the standard against the contracts and the replay) 10 (2), C (`eravon.link/1`) 9 (5, four in the corpus alone), D (`eravon.dossier/1`) 10 (2), E (the four laws against the judge's draft, the tooling and the corpora) 16 (1), and the three implementers, each starting from its round-7 program: measured unchanged, the round-7 programs diverged on the revision's classes alone (terms 52, link 56, dossier 0 at every seed: the 480-byte quote, the object signature, the three identity commands, the pack without its key), and, brought to the revised laws from the texts and the harnesses alone, each measured zero divergences and zero differences from prediction at seeds 1 to 6, 18 runs of 18, with 62,288 (Rust), 70,441 (Node) and 20,096 (Python) further probe inputs and no disagreement class (Rust 8 items, 1 load-bearing; Node 5, 2; Python 6, 0); Node also checked the standard's EIP-712 vectors, 231 comparisons with none differing. The four audit seats read the new contract generation: SEC one low and two informational items, VULN, ADV and FUZZ nothing above informational (4, 3 and 3); the low item, a router's answer naming a panel's zero entry, was verified a text item (the standard's audit record). One item bore on this law's harness: the judge's harness draft (「HARNESS-LINK 新三命令草案」) gives each `history` document a `source` member beside `position`, where round 0 had written the source inside `position` alone (seats E, Rust, C, D, Node, Python); the draft decides, and HARNESS-LINK, the criterion's writer and the generator's model now write `source` beside a `position` written as `place` writes it. Folded into this law: §1 says an accession's `root` is the book's statement, read by no step; §5 places the account key in the client that holds it (`eravon.terms/1` §5.5); §6.1 says a posting's `hash`, an accession's `root` and the records' chain are read by no predicate; §6.5's postings that are none include one under a key other than `terms.hash`; §10's checks at the act test that `init` and a draw under the record fit the chain's block gas limit (`init`'s read of a panel, and a draw without a router, growing with the square of the panel's length; VULN's informational item), and §10 reads the book §1 names; §11.4 names `galeed.roster/1` among the laws and says a later text of its §4.2 changes nothing here until this law's own revision follows it. HARNESS-LINK states each records member's form by name, that a list's elements stand in any order and that `chainId` is read by no command. The corpus grew from 653 to 674 cases: the pack's `case_upper` and `instance_upper`, lost with round 0's keystore block, restored; §1's replay clause by clause where each alone decides (`pay_declaration_to_entered_keeps_pending`, `history_declaration_to_self_first`, `pay_accession_to_entered`, `pay_accession_enters`, `pay_no_merger`, `pay_accession_root_unread`, `pay_same_block_book_order`, `pay_book_unsorted`), §6.2's own posting under every predicate (`place_own_bad_sig`, `pay_key_mismatch`, `history_key_mismatch`), the unread members (`place_hash_unread`, `pay_chain_unread`), the orders and standings (`place_least_opened`, `history_posted_across_move`, `history_lists_order`, `history_cases_unsorted`, `history_buyer_cases_unsorted`) and `history_event_name_any`; every record list of two or more elements is written out of position order by seed, so that a reader taking the lists as given fails; the twenty-seven mutants of the identity section seat C ran each meet, at every seed, a case on which the criterion answers otherwise. Zero differences from prediction at seeds 1 to 6. The README's rows follow (the five removed keystore names and a stub gone, the two current-key tests of the replay stated as no boundary on this surface, the transports' URL alphabet witnessed on failing bytes). Recorded and not folded: the records form stays closed (seats Rust and Python read the draft's "carrying" as admitting extra members; its "any other input is misuse" closes it, as Court has it); the draft's item link-1, whose explanation line reads "嵌 roster 核" against its own sentence and the draft's core section, is read by the sentence.

### Round 2: the twelve seats over the round-1 fold

The twelve seats read the round-1 fold (Court 9dd79a98): readers A (this law) 7 items (2 load-bearing, both in the corpus alone), B (the standard against the contracts and the replay) 5 (0), C (`eravon.link/1`) 4 (2, both in the corpus alone), D (`eravon.dossier/1`) 3 (1, in the corpus alone), E (the four laws against the judge's draft, the tooling and the corpora) 9 (1, in the corpus alone), and the three implementers: measured unchanged, the round-1 programs diverged on the one change the round-1 fold made to the criterion's answers and on nothing else (terms 0, link 10, dossier 0 at every seed: `history`'s separate `source` member), each wrote that member with one edit, and so changed each measured zero divergences and zero differences from prediction at seeds 1 to 6, 18 runs of 18, with 93,312 (Rust), 65,282 (Node) and 45,935 (Python) further probe inputs and no disagreement class (Rust 5 items, 0 load-bearing; Node 4, 1; Python 2, 0); Node checked the standard's EIP-712 vectors again, 231 comparisons with none differing. The four audit seats found nothing above informational on the contract code (SEC 4, VULN 5, ADV 2, FUZZ 3), the compiled bytes unchanged since 16c772f9; the judge ruled on 2026-10-25 that round 1 counts as the new generation's first clean round, so the contract gate closed at this round (the standard's audit record). The fold changes no answer of any criterion, the three digests standing as they were (terms `0xca99…4a9a`, link `0x9465…28e5`, dossier `0x7e2c…86f0`): it adds corpus cases and changes text, so the round-2 programs, unchanged, measure round 3 as the closing round if they show zero at seeds 1 to 6 on their first run. Seat C's two load-bearing items were corpus gaps of §1 and §6.5: a declaration and its accession in one block, the declaration first, had no case, so a reader merging the book by block number with accessions first passed every seed and answered `PAYABLE` where the law answers `SELLER_KEY_RETIRED`; and every two-element list being written descending (the shuffle reversing an ascending draw), a reader that sorted consents, keys, attempts or cases by their values, or reversed its lists, passed some seeds. Folded into this law: §6.5 says the postings that are none draw no document and are not listed, a posting's standing word being `stands` or `attempt` (seat Rust); §10's check at the act tests that `init` and the costliest draw, the one landing on the pool's last member at the case's third seat, each fit one transaction under the lesser of the chain's block gas limit and its transaction gas cap (2^24 under EIP-7825 on chains 1 and 11155111), and that the family declares the standard's succession book for the chain (VULN, SEC, ADV, FUZZ and seat B, the standard's audit record). HARNESS-LINK says `chainId`, each `hash` and each `root` are read for their form alone (seats Rust, Node, Python, C). The corpus grew from 674 to 708 cases: `pay_same_block_rotation` and `history_same_block_rotation`; `history_revoked_after_move` (a document only revoked, the revoked sent after the move, stands) and `place_revoked_before_opened` (seat Rust); `history_consents_against_digest`, `history_keys_against_address`, `history_two_attempts`, `history_cases_against_values` and `history_buyer_cases_against_values`, each list in ascending position with its members' values in the opposite order, and `history_lists_ascending`, the one record set written in ascending position; and one form case per member of each records list (`form_<list>_<member>`, from the generator's own table of the members and their forms, in place of three hand cases), a member twice and a negative `logIndex` (seat C). Zero differences from prediction at seeds 1 to 6. The README's §1 replay row is written clause by clause, each clause with the case that refuses and the case that lets bind, `pay_declaration_to_self` noted as witnessing no clause, the §6.5 row describing `history_lists_order` as it is and a record as drawing at most one document (seat C). Recorded and not folded: seat C's self-check of the generator over a family of readings (the six cases and the ascending set meet each reading at every seed by construction, so the check would restate them); a client check that a record's router is an EIP-7702 delegation designator (the standard's §7 states what the frozen hash pins, and who sits on a guild's panel and which router draws from it are the guild's affair).

### Round 3: the eight law seats over the round-2 fold

The eight law seats read the round-2 fold (Court 7e4001bb); the contract gate having closed at round 2, no audit seat sat. Readers A (this law) 6 items (2 in the corpus alone, tagged load-bearing by seat A), B (the standard against the contracts, the tests and the replay) 3 (0), C (`eravon.link/1`) 5 (0), D (`eravon.dossier/1`) 1 (0), E (the four laws against the judge's draft, the tooling and the corpora) 6 (0); the three implementers, their round-2 programs unchanged, measured zero divergences and zero differences from prediction at seeds 1 to 6 on their first run, 18 runs of 18 each (terms 4,179, link 708, dossier 1,187 cases per seed), with 157,270 (Rust), 76,093 (Node) and 57,292 (Python) further probe inputs and no disagreement class (Rust 3 items, Node 2, Python 4, none load-bearing); Node checked the EIP-712 vectors once more, 231 comparisons with none differing. So round 3 is the closing round the gate asks for: the previous round's programs unchanged, zero at seeds 1 to 6 on the first measurement, no item that two careful implementations could read apart. Every item of the round is text or corpus, folded here in one pass; the criteria stand (terms `0xca99…4a9a`, link `0x9465…28e5`, dossier `0x7e2c…86f0`). The naming waits: the judge's plan of 2026-10-26 (stages, prepayment interlocked) is applied next in this generation and changes the three criteria, so the texts reopen under it. Folded into this law: §10's Rotation says the fee goes to the instance after the case opens, as `eravon.terms/1` §5.5 has it (seat C); §10's check at the act measures the costliest draw as the `redraw` transaction that lands it, the verifier's `draw` alone falling short by the instance's own reads and writes (seat B measured 443,218 to 698,820 gas, with panels of 264 and 266 members passing the call and failing the transaction), and under a router a draw whose router spends all of `routeGas()` (seats B, C); §11.1 says the canonical form this law's documents are read under is witnessed on every input of the `galeed/1` corpus's `check` set as a listing file, where no test of this law decides before `canonical(b)` (seat Rust: §3.5's first test over the whole input is reachable on this surface alone); §11.4 pins every GALEED passage this text cites at the naming (seat E). The corpus grew from 708 to 4,476 cases: the 3,656 `galeed/1` check inputs as listings (`gl1_*`); every identity case with a list of two or more elements written again in ascending position (`<slug>_asc`, 33 at seed 1, in place of round 2's one ascending set, so that a reader reversing or merging a kind of list as given meets a case at every seed), `history_case_records_order` and `history_postings_against_hash` (seat C); every member of every records list under every break of its form (`form_<list>_<member>_<break>`, 100 cases in place of 25) and the two argument forms (seat C); zero differences from prediction at seeds 1 to 6. The README's replay table says which later record each `enters` row's cases carry and names `pay_retired` on their passing side (seat C).

## Revision in place of 2026-10-26: stages, a sale delivered in parts under one judge

The judge's plan of 2026-10-26 (`修法方案-分段交付系列标识-2026-10-26`, its fourth version, with the ruling of the same day that a stage ended Lapsed carries no judge forward), applied in one pass as round 0 of the 2026-10-24 generation after that generation's round 3 closed the texts and before their naming: an order names the stage before it (`prev`), the standard opens a later stage only once the stage before is paid and lets the buyer cancel it only once the stage before failed, a document may declare the series its stages belong to, and the draw without a router seats the stage before's judge first. The three criteria change with it, so the texts reopen under it; the contract bytes change, so the contract gate restarts at this generation's next round (Court 56081380). The twelve seats sit again over this checkpoint.

### Round 0: the plan applied in place

§4's receipt carries `prev` as the order does (required, a hex20,
`E_RECEIPT_PREV`; `E_RECEIPT_MISSING` where absent), bound into the order's
hash the receipt is checked against; §2.6 says a document's `series` groups
a listing with the other stages of its sale; §10's Stages paragraph says
what each client does with a series (the buyer's client signs the stages in
the documents' order with a zero `prev` at stage 1 and each later `prev` the
instance it has just signed, funds every stage and shows the buyer when a
later stage's money comes back; the seller's client takes a series only
where the `prev`s chain it in order and each later `openBy` outlasts the
stages before, and begins a stage only once the standard opened it; a
judge's desk shows a series as one matter; a guild that names a router and
wants one judge for a series makes the router draw by the parties the view
carries) and that the lock both rely on is the standard's. The criterion
follows in `erllaw.py` and `erl.py`; the dossier core embeds it, so its
digest changes too; HARNESS-LINK's receipt form names the member. The corpus
grew from 4,476 to 4,487 cases: `prev_nonzero` (bound, its digest
differing), `prev_missing`, `prev_short`, `prev_upper`, `prev_int`,
`sig_other_prev` (the signature against another `prev`), the adjacent pairs
`adj_openby_before_prev` and `adj_prev_before_sig` in place of
`adj_openby_before_sig`, two listings of one series (`ok_series_stage_1`,
`ok_series_stage_2`) and a listing whose document's `series` fails its form
(`terms_doc_series`, `E_LISTING_TERMS_DOC`); zero differences from
prediction at seeds 1 to 6. The README's §4 rows follow.

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
under the same two keys, a client rule. Folded into this law: §4.2 says zero
is admitted in `prev`'s own test (seat Python); §10's Stages says the
buyer's client signs or exports under one `buyerKey` (seat C), names the
stage's own expiry branch, its `openBy`, its offer's `expiry` or a
revocation, as what returns a later stage's money (seat C), carries the
seller's client rule in full (one `chainId`, one `logic`, the same two keys,
a later stage's offer naming the record's former `sellerKey` where the
seller's record moved, the `prev`s chaining from a zero `prev` at stage 1,
each later `openBy` after the latest second the stages before can still end;
a receipt with a nonzero `prev` taken only as a later stage of a series
whose earlier receipt the client holds, one with `prev` zero only for no
series or stage 1; seats B, ADV, SEC, Node), states the series judge on §7's
conditions in place of "the stage before's judge while the guild keeps it on
the panel" (seat C), and measures the costliest draw of a later stage with
`prev` in the storage and the series judge standing last in the panel and
already seated (seats B, C, VULN, ADV); the Address collision sentence
counts `prev` among the salt's members (seats ADV, Python). The corpus grew
from 4,487 to 4,505 cases: every hex member of the receipt under every break
of its form (`form_receipt_<member>_<break>`: uppercase, a byte short, a
byte long, no prefix, sixteen cases; the no-prefix reading of `prev` had
passed all six seeds; seat C), a later stage's pack (`ok_later_stage`, its
instance and case the ones the receipt's `prev` derives; a reader binding
packs at `prev` zero had passed all six seeds; seats C, E) and a settlement
bound over it (`bind-settle` on `settles/…_ok_later_stage`); zero
differences from prediction at seeds 1 to 6. The README's rows follow
(`terms_doc_series`, the two `ok_series_stage_*` listings, `prev_upper`;
seat C), and this record's Round 0 cites §2.6 (seats C, Node).

### Round 2: the twelve seats over the round-1 fold

The twelve seats sat over the round-1 fold (Court f5b6a5d2). The three implementers, their round-1 programs unchanged (Rust's binaries byte-identical, Node's and Python's sources checked by digest), measured zero divergences and zero differences from prediction at seeds 1 to 6 on their first run, 18 runs of 18 each (terms 4,238, link 4,505, dossier 1,197 cases per seed), with 26,584 (Rust), 27,539 (Node) and 50,718 (Python) further probe inputs and no disagreement class (Rust 0 items, Node 2, Python 2, all text). Readers A (`eravon.terms/1`) 3 items, B (the standard against the contracts) 8 (one of them on the contract's code, informational, folded as text by the judge's ruling), C (`eravon.link/1`) 4, D (`eravon.dossier/1`) 3, E (the four laws against the plan and the tooling) 4 and a family report; every item text or corpus, none load-bearing, every seat saying it would name. The four audit seats found nothing above informational on the fold's bytes (the audit record), so the fold changes no byte. **So round 2 is the closing round the gate asks for**: the previous round's programs unchanged, zero at seeds 1 to 6 on the first measurement, no load-bearing item; the residue is folded here in one pass (Court f78f62c8), and the folded sentences are read once more by the readers of the next round, which is the audit's second round over these bytes; the naming follows the contract gate (`law-revision-standard` 3.1: the type string and the verifier are shared with the contract). The judge's rulings of 2026-10-26 on the round's two questions: an inherited series judge who declines or resigns a later stage stays the series judge (the parties end the series by a dispute where they will not go on; no judge is drawn to replace it for that alone), and an account of other code than the proxy's that proves an address at the doors is a text matter (no reader admits it; events are read by the instance's address), the bytes standing. Folded into this law: §4.4 says a receipt's `prev` is taken only as §10's Stages states, which the binding does not read (seat Python); §10's Stages says the documents name one `sellerKey` and the receipts one `buyerKey` (a document carries no buyer's key; seat C), describes a later stage's offer after the seller's record moved as Rotation does (its slot empty, its digest registered from the current key under the former; the fold's "signed by the current key" was refused by §2.5; seat C), cites the standard's §4a and §7 for the series judge (seats B, C), places the series judge at the costliest draw as the standard's §7 now says (seated earlier, immediately before the pool's last member; seats B, C, D), and says a client reads a stage's events from the instance's address, never by `caseId` across emitters (the judge's ruling on the doors' address proof). The corpus is unchanged at 4,505 cases; zero differences from prediction at seeds 1 to 6. The README's §5 and §8 rows name the later-stage pack and settlement (seats C, E).

### Round 3: the re-reading of the round-2 fold

The twelve seats sat over the round-2 fold (Court 86dc194c), the re-reading `law-revision-standard` 3.2 asks for. The three implementers, their programs unchanged since round 1, measured zero divergences and zero differences from prediction at seeds 1 to 6 on their first run, 18 runs of 18 each (terms 4,239, link 4,505, dossier 1,197 cases per seed, the new `series_stages_bool` among them), with 29,172 (Rust), 30,568 (Node) and 57,178 (Python) further probe inputs and no disagreement class (Rust 1 item, Node 3, Python 3, all text). Readers A 2 items, B 4, C 4, D 2, E 2 and a family report; every item text, none load-bearing, every seat ready to name the moment the contract gate closes. The four audit seats found nothing above informational on the bytes of the round-1 fold, the second consecutive clean round over them, **so the contract gate closed** (the audit record). The residue is folded here in one pass (Court a29c4a10): the sentences the round-2 fold had added are read as one reading now, or reworded where two readers took them two ways; no criterion file changes and the three digests stand. The naming follows. Folded into this law: §4.4's added clause names its subject (`R`'s `prev` enters the order as every other member does; whether a seller takes a receipt with that `prev` is §10's rule, which the binding does not decide; seats C, D, E, Node); §10's Stages adds the `stages` agreement terms §8 lists (seat C), describes a moved seller's later stage by §6.4 and Rotation (a listing posted while the former key was current, honoured by the current key's own `init` or its registration; a stage listed after the move is paid by nobody, so a seller lists every stage before it moves; seat C), posits the series judge for the costliest draw as the standard's §7 does (seats B, C, Node), and says the lock is the order's that names `prev` (seat ADV). The corpus is unchanged at 4,505 cases; zero differences from prediction at seeds 1 to 6.

### The naming of 2026-10-27

The contract gate having closed at the stages' rounds 2 and 3 (the audit record), and the texts at round 2 with their residue read once more in round 3 and by a re-reading seat over the round-3 fold (16 of 18 sentences read as one reading, two reworded: terms §8's lock sentence and dossier §7.1's witness clause), `naming.py 2026-10-27` named the three cores in the order they nest, each note carrying the fuzzing of this revision (the three seats' 951,220 differential inputs over its six rounds: Rust 391,921, Node 295,290, Python 264,009) and, after it, the record of the fuzzing run for the core named on 2026-10-19. After the act the three digest scripts recomputed the digests the sections carry, `compare.py` at seeds 1 to 6 exited 0 for the three laws, `forge test` passed 335 of 335 and `identity.sh` agreed with CODEHASH.md (Court 6d561dad). This law's §11.5 (§11.4 pinning the `eravon.terms/1` core) names `0x25eb3810d9a870633608d36054a8399e1445435caa703c7ed5e8fd1bae8f11dc`, superseding `0x29ff5e4651ee99cdc94521157ef96ad1e943473bea645cc24a569fb4429c80ef` (named on 2026-10-19, the sellerFund revision) and the unnamed candidate `0x9465…28e5` of the 2026-10-24 revision's rounds 1 to 3, which the stages' `prev`, `series` and receipt member replaced before any naming. The text's sha256 after the act is `a81808f7e4f315e39d55a41adb598e52373708199f45d61cbc6de5ae15950bec`.

Prose in place after the naming, 2026-10-30: §10's clause on the gas one
transaction may spend reads the transaction gas cap as one the chain sets
(2^24 under EIP-7825 on chain 1) in place of a cap stated for chains 1 and
11155111, ERAVON having measured on 2026-10-28 that Sepolia admits
transactions above 2^24 and reported it on 2026-10-30; `eravon.standard/1`
§7 says the same in the same words. The clause is prose and measured by no
corpus: the core and its digest are unchanged, as `criterion-digest-link.sh`
recomputes, seed 1 rebuilds byte for byte, and the corpus shows zero
differences from prediction at seeds 1 to 6. The text's own sha256 after
this revision is
`0xaedb87e90c778d89a11ace06603847e1808befd6896336d11d376645da0662ee`.

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

Of the passages this text cites, one moved: `galeed.roster/1` §9, whose
sentence on a key's records now counts none of those at which the key has
retired, as `galeed/1` §7.4 now reads; §10 cites it among the client's
checks at the act, outside the grammar. `galeed.roster/1` §4.2, which §1
restates, reads as it did. The core's reading is unchanged.

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
`0xd42962fc6802d6279e24a317b4073406d873c028f96b3d95441db27c201cb6bc` (before
it, `0xaedb87e90c778d89a11ace06603847e1808befd6896336d11d376645da0662ee`).
