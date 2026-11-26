# eravon.dossier/1: the review record

The convergence of `eravon.dossier/1` (docs/eravon-dossier-v1.md) is run
the way `galeed/1`, `galeed.papers/1`, `galeed.roster/1`, and
`courtevidence/1` were run: adversarial readers of the text and
independent implementations written from the text and the harness alone,
one round at a time, every finding folded in one pass, the criterion, the
harness, and the corpus following, until the readers report nothing open
and the implementations reach zero divergences with every closed boundary
witnessed on both sides. The seating of each round is described in the
`eravon.terms/1` record; this record carries what concerns
`eravon.dossier/1`.

## Round 0: the criterion against its own generator

The criterion (`eravon-conformance/dossier-py/`) and the corpus generator
(`eravon-conformance/dossier-corpus/gen.py`, 105 cases at seed 1) were
written from the text by one hand and compared under seeds 1 to 4: zero
disagreements with the generator's hand-written predictions.

## Round 1: five readers and three implementations

The three implementations reached zero divergences on the 105-case corpus
at seeds 1 to 6 on their first measurement and found no disagreement
beyond it. The readers found the law's two conflicts with the frozen
GALEED texts: `E_DOSSIER_ENTRY_AMANAH` equated a judgment's `amanah`, the
`entry_id` of the judge's amanah entry under `galeed/1` §6.3, with the
seat's `amanahHash`, the registry word or the guild text digest under
`galeed.venue/1` §2.4, so that no honest export could pass; and
`E_DOSSIER_ENTRY_JUDGE` refused a judge who rotated keys mid-case, which
`galeed.venue/1` §1 admits and which an offline predicate cannot resolve.
Beside them: the `[C]` section compared a terminal `name` with an event
that carries a byte nothing numbered; its kind-to-event map covered
thirteen of seventeen kinds and named neither `Priced` nor
`PanelFrozen`; the missing `dossier.json` was decided by the harness
alone; the answer's `candidate` was undefined; §7.1's precondition named
the index tokens and dropped the file tokens; §1's prose-subtree gloss
read as the rule; extra files, repeated references, and the pool seat's
`judge` and `seat` were decided by nothing; `subjectRef.seat` and
`decision.rung` were compared with nothing; `setPayout` and `withdraw`
had no kind.

Folded (Court commit after this record):

- §3.1 drops `E_DOSSIER_ENTRY_AMANAH` and `E_DOSSIER_ENTRY_JUDGE`; the
  entry's `judge` (the seat's or a key of its lineage) and its `amanah`
  (resolved in the judge's ledger against the word the seat froze) are
  §5.1's chain comparisons; `E_DOSSIER_SUBJECT` gains the `seat` member,
  present and equal to the index's; the answer's `candidate` is the
  receipt's; the unreachable verdicts are named.
- §2.6 opens with the index file's readability, `E_DOSSIER_FILE` carrying
  the key `index`; §1 states the `_md` rule, the extra file, the repeated
  reference, and a reference naming `dossier.json`; §2.2 tests
  `amanahHash` nonzero and says what a pool seat's `judge` and `seat` are
  and that §3 reads neither; §2.3 has nineteen kinds; §2.5 says a
  conversion's `tx` is related to no row.
- §5.1 is a list: the code comparison only where a `deploy` row exists,
  a kind-to-witness table over all nineteen kinds, `Seated` and `seatOf`
  for the seat, `PanelFrozen` for a pool candidate, the terminal byte's
  numbering (`Refunded` 0 through `Lapsed` 5, now fixed in the design
  volume §3.12), the carrying-form `Anchored` named as the amended
  venue's, the lineage and the amanah resolution, and `decision.rung`
  against `Ruled`; §6 carries the terminal-and-entry bullet, the class
  item a reader adds to its basis, and the index's own check; §7.1 ranges
  over both token families, names `galeed/1` §4 and §6 and the four
  harness contracts among the sources, and says the law is not frozen
  before the venue amendments; §7.4 says the digest is quoted once it
  exists.
- Harness: `E_DOSSIER_FILE` carries `index` as a fifth key value and
  `E_DOSSIER_DIGEST` does not; a `<dir>` that is not a directory is
  misuse and the files inside are the law's; the nested verdict keeps its
  index.
- Corpus: 132 cases, with the `E_DOSSIER_SUBJECT` comparisons of
  `contract`, `codeHash`, `seat` (differing, absent, and a pool seat),
  the three missing `(token, key)` pairs, eight more §2.6 adjacencies
  and seven of §3.1, `amanahHash` zero, an extra file, two references to
  one file, a reference to `dossier.json`, and entries signed by another
  key or naming another `amanah` accepted offline.

## Round 2: the same eight seats on the revised text

The three implementations reached zero divergences on the 132-case corpus
at seeds 1 to 6 on their first measurement and found no disagreement
beyond it. The readers closed every round-1 item; the load-bearing
findings on this law: nothing fixed the order of tests inside the
compliance page, and §2.6 and §4 disagreed on when `E_DOSSIER_COMPLIANCE`
carries an index; §3.1's read-and-digest loop admitted two orders that
name different files; the three items that change a frozen output.
Beside them: §3.1 required `seat` on a resignation where
`galeed.venue/1` §2.1 puts the ordinal on judgments alone, and gave
`subjectRef.seat` no form; §2.2 said no predicate of §3 reads a pool
seat's `seat` where §3.1's subject test does; §2.2 tested `amanahHash`
nonzero and `quoteHash` not; §3.2 claimed more of the entry than §3.1
tests; in `[C]` §5.1, an omitted `deploy` row turned into a claim that
the account has no code, the `Anchored` sender clause admitted a sender
the standard refuses, the `PanelFrozen` condition could not be decided
before the comparison it conditions, the kind table was a witness
relation called a function, a no-op `advance` had no event, `expire` was
mapped to a cause only `advance` emits, and the named track's registry
word derivation was unstated; §7.1 named one of the two amendments the
law leans on; the Status clause named a core §7.5 says does not exist.

Folded (Court commit after this record):

- §2.5 decides the compliance rows in written order, each conversion in
  array order and its members in row order, and carries the index for an
  element's own form and nothing elsewhere; §3.1 decides both tests of
  one key before the next key is read; `subjectRef.seat` is an `int`,
  present on a judgment and optional on a resignation.
- §1 says `bindEntry` yields `entryType` and `body` for §3 and `judge`
  for §5.1; §2.2 says §3.1's subject test reads `seat` under both forms
  and that `quoteHash` is made nonzero by §3.1's comparison; §3.2 claims
  what §3.1 tests and leaves the key and the word to §5.1.
- §5.1: the code comparison where a `deploy` row or a `terminal` is
  present, an account with no code otherwise being the funded undeployed
  case; the table a witness relation over every kind, `accept` by the
  call, `expire` with cause 2, `advance` with no event where the call was
  a no-op, `PanelFrozen` by index and, where the indices are equal, by
  `panelHash`, `Anchored`'s sender the address `galeed.venue/1` §2.9
  admits, the named track's word read from the registry's `register` log;
  §7.1 names `galeed/1` §3 through §6 among the sources and both pending
  amendments; the Status clause says the digest is agreed with once
  named.
- Corpus: 136 cases, with a resignation whose subject carries no seat
  accepted and one whose seat differs refused, a `seat` that is no `int`,
  and two compliance faults decided in row order.

## Round 3: the same eight seats on the round-2 text

The three implementations reached zero divergences on the 136-case corpus
at seeds 1 to 6 on their first measurement and found no disagreement
beyond it. The readers closed every round-2 item and answered the two
questions asked: the pending-amendment clause could be dropped, both
forms standing in the recast package; and the `[D]` predicates were
sound but for one place. The load-bearing findings: §2.5 ordered the
compliance page's rows and left the closed test unplaced, observable
through the index it carries, the one item that changes a frozen output;
§6 spelled the class item with a member `galeed/1` §9.4 refuses; §7.1
called the amendments pending and omitted `galeed/1` §1 from its
sources; §3.2 claimed a seat number of a resignation; §5.1 called the
entry bytes the log's data where they are what its data encodes, named
no root for the lineage, no emitter for `PanelFrozen`, a wrong preimage
for the pool digest and a log no cited law defines for the named track;
§1 matched a file name by the host's own folding, so that one dossier
answered differently on two filesystems; §3.1 counted four keys where
one is optional; the corpus witnessed two of the eight `galeed/1` §3
tokens on an index.

Folded (Court commit after this record):

- §2.5 decides the value's form, then no member beyond the list, then
  the rows; §1 says a file the index names is the directory entry
  byte-equal to it and names the three `_md` keys beside the parent's
  suffix rule; §3.1 counts every key the index names; §3.2 claims the
  seat number on a judgment; §5.1 names the carried registry form with
  its ABI-encoded data, the lineage rooted at the seat's judge, the
  instance as `PanelFrozen`'s emitter, the pool digest over the decoded
  text and the named track's registry publication; §6 spells the class
  item as §9.4 does; §7.1 names `galeed/1` §1 and says nothing waits on
  an amendment, and ranges over pairs where §4 names a finite carried
  set.
- Harness: the name is matched byte for byte against the directory's
  entries.
- Corpus: 146 cases, with the six missing canonical-form tokens on an index, an
  unlisted `_md` key, the closed test before the rows, a name differing
  in case from the entry, and a name matching a directory.

## Round 4: the same eight seats on the round-3 text

The three implementations reached zero divergences on the 146-case corpus
at seeds 1 to 6 after the one predicted update (the file name matched
byte for byte). The readers closed every round-3 item; the findings, one
changing a frozen output: §1 scoped the byte-equal rule to the files the
index names and left `dossier.json` itself to the host; §7.1's pair
sentence enumerated two cases where §4 has three and dropped the tokens
that carry an index; §7.1's closing sentence said the two forms stand in
texts §7.4 names, where §7.4 names cores; §7.4 omitted `galeed.venue/1`;
§5.1 mixed two topic conventions, named a `word` by the log's name, and
cited §2.9 alone for the resignation's form; the entry's kind was left to
the host where a link is followed.

Folded (Court commit after this record):

- §1 applies the byte-equal rule to `dossier.json` as to every named file
  and says an entry a read does not open as a file is unreadable, a link
  followed; §5.1 says "first indexed topic", cites §2.9 and §2.10, and
  names the log's `amanahHash` beside the record's `word`; §7.1 names the
  three carriers it takes in pairs and `E_TRANSACTION` with its index,
  and says the two forms stand in the frozen texts of `galeed/1` and
  `galeed.venue/1`; §7.4 names `galeed.venue/1` as an identity and no
  core.
- Harness: `dossier.json` under the same rule, links followed.
- Corpus: 148 cases, with the index named in another case and the index
  as a directory.

## Round 5 and the naming

The three implementations reached zero divergences on the 151-case corpus
at seeds 1 to 6 with no change to their code. The dossier reader asked
for one deletion in §7.1's parenthetical; the corpus auditor for
symbolic links in the corpus (§1 having said a link followed to a file
opens); the Rust and Python seats for the entry's kind to be named, a
read of a named pipe never answering; the Node seat for §7.1's key
clause to name the two file tokens that carry one. Folded: §1 says the
entry is a regular file, a symbolic link resolving to one included, and
carries the parent's `_md` form; the harness says the same; §7.1 names
the two key carriers; the corpus carries a link to a file, the index
through a link, and a dangling link, 151 cases.

Named on 2026-09-16, after `eravon.link/1` §10.5 and with §7.4 quoting
that digest: the release digest
`0x8a9ccd74ce047be8c07a6d92e6fcaac53e3a42838217218b99c88787aed7bebc`
over `erd.py`, `erdlaw.py` and the `eravon.link/1` core, as §7.5 quotes
it; the text's own sha256 at the naming is
`f2729b76b8ab24ade85f02088c7323524b7d7b3770fff4247bd5bd6e1561249e`; the fuzzing of
§7.1's deferral was not run.

## Textbook revision after the client design cross-check (2026-09-17)

§5.1, a [C] section outside the corpus, said that `setPayout` and
`withdraw` "emit nothing"; the frozen standard's `withdraw` emits `Swept`
where it pays (`Eravon.sol`, `_swept`), so the sentence now reads:
`setPayout` emits nothing, `withdraw` emits `Swept` where it pays and
nothing where the seat had nothing claimable. No predicate moved; the core
§7.5 names is unchanged, `criterion-digest-dossier.sh` recomputing
`0x8a9ccd74ce047be8c07a6d92e6fcaac53e3a42838217218b99c88787aed7bebc`;
`dossier-corpus/gen.py --seed 1` rebuilds the committed corpus and
`compare.py dossier` reports zero divergences and zero disagreements with
the predictions. The text's own sha256 after this revision is
`f6e9c6c7218a7fa840d3e1f470e30177a13f5d726a5d4ac8a96efd4ea0c77146`.

## Revision in place for consent by call (2026-09-23)

The seating and the reasons are in the `eravon.terms/1` record; this
record carries what concerns `eravon.dossier/1`.

### Round 0: the criterion against its own generator

`bindReceipt` may yield `RECEIPT_BY_CALL`; the transaction kinds gain
`consent` (twenty); the accepted answer gains `buyerConsent` and
`sellerConsent`; §5.1 gains the chain reading of a by-call consent; §1,
§2.3, §3.1, §3.2, §4, §7.1 were written; the Status clause, §7.4 and
§7.5 returned to the unnamed forms. The criterion and the generator (159
cases) agreed at seeds 1 to 4.

### Round 1: five readers and three implementations

The three implementations reached zero divergences on the 159-case
corpus at seeds 1 to 6 on their first measurement and probed beyond it
with no disagreement. The load-bearing findings on this law, none
changing a frozen output: §5.1 said "the `init` transaction's sender"
where the standard compares the immediate caller, so a reader would
report a failed consent on every company buyer that called `init`
through its executor; the registry entry was required "in a block at or
before `init`'s", which admits a later log of the same block; the
`settle` row could not be recomputed from `Settled`, which carries no
`expiry`; the `consent` row admitted a digest the dossier cannot
recompute and paired no key with a digest; the table's lead sentence
and §6's completeness bullet spoke of the instance alone where
`Consented` is the logic's; the event was cited to a section that does
not name it; "the bound order's" in §2.1 and §3.1 could be read as
excluding `RECEIPT_BY_CALL`, so that the instance and case mismatches
would be skipped on a by-call receipt, which the criterion does not
skip; §3.1 did not say which slot value gives each consent form; §4
closed no answer vocabulary; §1's `bindReceipt` clause named a receipt
where the verdict is the dossier's; fourteen of the twenty kinds had no
passing witness although the README said `ok_ruled` covered them all.

Folded (Court commit after this record):

- §5.1 reads the caller of `init` and of `settle` from the call (the
  trace where the call was inner), a `Consented` before the call in
  chain order, and the `settle` call's arguments; the `consent` row is
  confined to the three digests the dossier can recompute, the rest
  reported as not recomputed and no fault; the lead sentence and §6
  name the logic's registry.
- §2.1 and §3.1 read the values the receipt's binding yields under
  either verdict; §3.1 states each consent form's slot value; §4 closes
  the answer values; §1's clause speaks of every whole dossier; the
  `consent` row spells the event and cites the venue document.
- Corpus: 162 cases, with one row of each of the twenty kinds and the
  two mismatches under a by-call receipt.

### Round 2: five readers and three implementations

The three implementations, updated from the revised texts alone, reached
zero divergences at seeds 1 to 6 on their first measurement (162 cases
per seed) and probed beyond the corpus with no disagreement inside the
measuring interface; the corpus auditor found nothing open in this law's
corpus, README or harness. The readers found the [C] section short of
what round 1 had recorded as folded: §5.1 still said "the bound order's
digest" for a receipt whose verdict is `RECEIPT_BY_CALL`; the `consent`
row let any of the case's two keys pair with any of the three digests
and read a `settle` that emitted nothing; the consent bullet hung on the
dossier's rows rather than on the `init` that emitted `Opened`, and
"before the `init` call in chain order" did not say what order two logs
of one block take; the `settle` reading was a clause of the buyer's
bullet; "never the transaction's origin" excluded a party's own
top-level `init`; and a reader whose endpoints serve no call trace (a
company's `init` is an inner call of its proposal executor, so no
transaction input carries its arguments) had no reading and would report
a failed consent on an honest case. Also §5.2 and §6 said nothing of a
`Consented` the reader cannot place, §5.1 cited no document for the
event's shape, and §7.5 did not state the naming order §10.5 of the
link law states.

Folded (Court commit after this record):

- §5.1's `consent` row pairs each key with its digest (the seller's with
  a candidate's offer digest, the buyer's with the `orderDigest` the
  binding yields, either with the `Settle` digest of the call that
  emitted `Settled`) and cites the venue document's §10 for the event;
  the consent bullet reads the call of `init` that emitted `Opened`,
  the caller being the transaction's sender for a top-level call and
  the address the trace shows for an inner one, and a `Consented`
  before it in chain order, a lower log index in the same block
  included; the `settle` reading is its own bullet over the call that
  emitted `Settled`; a third bullet gives the reading where no call
  trace is served (the `Consented` entries alone, and a form no entry
  decides reported as not recomputed, the instance's `Opened` or
  `Settled` being the standard's own admission); §5.2 and §6 place a
  `Consented` the reader cannot assign as the exporter's word and no
  row the dossier owes; §7.5 names no digest before the link law's.
- Corpus: 162 cases, unchanged.

### Round 3: five readers and three implementations

The three implementations, updated from the texts alone, reached zero
divergences at seeds 1 to 6 on their first measurement (162 cases per
seed) and probed beyond the corpus with no disagreement inside the
measuring interface. On this law, all in [C] §5.1: "the call of `init`
that emitted that `Opened`" can be found by the frame that holds the
log, which under an EIP-1167 proxy is the delegation frame whose `from`
is the instance; the round-2 no-trace bullet took `expiry` from "the
dossier's settlement offer", a file §2.1's closed `files` cannot carry,
dropped the chain-order condition, and merged the answer's form with a
`settle` slot; the `settle` bullet was keyed on rows where the `Opened`
bullet is keyed on the event, so a dossier whose exporter omitted the
`settle` row left the settlement unread; the `advance` row named no
call and no non-terminal effect (a guild seat's offer clock running out
leaves a redraw pending and emits nothing). On the corpus: the five
document tests of §3.1 and the mismatches were witnessed in no order
beside each other, and no row's form beside `terminal`, so a checker
parsing every document before binding, or comparing the index to the
payload first, passed every seed.

Folded (Court commit after this record):

- §5.1 finds the call of `init` by its target, the account at
  `instance`, never the delegation beneath it, and reads the caller as
  that call's `msg.sender` (the transaction's sender, or the `from` of
  the call's own frame); the `Settled` bullet reads whatever rows the
  dossier carries and finds its call the same way; the no-trace bullet
  reads the logic's `Consented` alone under the same pairing and chain
  order, reports a consent at `init` no entry decides as not
  recomputed, and for `settle` forms no digest (the `expiry` only the
  call carries) and reports both consents as not recomputed, neither a
  fault; `advance` is a successful call of the venue's `advance`,
  whatever it emitted.
- Corpus: 168 cases; the five document tests in their order and before
  the mismatches, and a row's form before `terminal`.

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
and 6; none changed a frozen output. 
On this law, all in [C] §5.1: `advance(caseId, seat)` read as binding
the arguments to the index's, which the guild redraw's `advance(caseId,
0)` fails, and "a terminal event" was undefined where a T2 lapse emits
`Closed` alone; rows witnessed by a call alone (`deploy` through a
batch, `accept`, `advance`, `setPayout`, `withdraw`) had no reading
without a trace; "serve no call trace" did not say of which
transaction; "the call that emitted that `Opened`" was found by a log
only one tracer places, and a rolled-back successful `init` frame beside
the kept one split readers; "no digest can be formed" was refutable by a
counterparty holding the offer and unsound (same `buyerBps`, another
`expiry`); the no-fault rationale's premise held for every instance of
every logic. On the corpus: every key's file before any document test
was unwitnessed and `d114` a no-op for it; the terminal's form before
the `Ruled` rule, the entry's type before its subject, and two faulting
rows were unwitnessed.

Folded (Court commit after this record):

- §5.1: `advance` is witnessed by a successful call whatever pair it
  names, `Closed` where it ended the case; a row witnessed by a call
  the reader cannot trace is reported as not recomputed; the no-trace
  condition is per transaction; the call of `init` is the one the
  transaction kept (returned without reverting beneath no reverting
  frame), exactly one, found alike from either tracer; the reader forms
  no settlement digest whatever it holds; the no-fault reason states
  its premise (the standard emits only after admitting; whether `logic`
  is the standard the instance's code does not answer).
- Corpus: 174 cases; every key's file before any document test, the
  entry key included; the terminal before the `Ruled` rule; the entry's
  type before its subject; row 0 before row 1; `d114` renamed to what
  it witnesses; canonical before the root on an index that is neither.

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
`0xffe009e042b5b4c470bd93b93b82c549a87e0cbc66f2e491d05393de3f1e524c`
over `erd.py`, `erdlaw.py` and the `eravon.link/1` core, as §7.5 quotes it, §7.4 quoting the link digest. The text's own sha256 at the
naming is `68c977e8ea4d9384c172b55cd307c61f6e841e3bcff6bc5e9f0e2d2859b9ff96`. The fuzzing of the deferral was run at the
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
eight seats over the regenerated corpora at seeds 1 to 6. This law's own
answers hold no EIP-712 digest (`hashStruct(order)`, the CREATE2 address
and two sha256 values only), so the domain change touches its measured
surface only through the payload signatures the link law verifies; the
implementations' first measurement diverged on 67 `check` cases per seed,
all `E_DOSSIER_PAYLOAD` on payloads signed under the new domain, zero after
the one constant in the terms core. Reader C found nothing open beyond a
trailing newline the withdrawal edit had stripped (restored); reader D's
144 dossier predictions recomputed from the text agreed at every seed; the
probes and swap audits are recorded with the terms law.

Named on 2026-09-24 at Court commit after this record: the release digest
`0x897fc8910f26d56a8c5021f3da58f03b8b5639d9f0feab9186b4b85bddb64a9b`
over `erd.py`, `erdlaw.py` and the `eravon.link/1` core, as §7.5 quotes it,
§7.4 quoting the link digest. The text's own sha256 at the naming is
`2472839b7922990175be05dbea3cf05aae7e8ac36f7597ca9f9832c075c1f88c`. The fuzzing of the deferral was run at the
naming, as the digest's note records.

## Re-pin of 2026-10-14: the link core under the open funding interface

`eravon.link/1` was revised in place (its record, "Revision in place for
the open funding interface"): a payload may carry `fund` and `fundSig`,
and the link core changed. This law's text changed in one [C] sentence
alone, §5.1's reading of `Consented` admitting a fourth pair, the
seller and the payload's `fundDigest(P)`, where the payload carries
`fund`; no [D] predicate, member table or answer reads the new members,
`E_DOSSIER_PAYLOAD` carrying whatever `eravon.link/1` §2.5 answers. The
Status clause, §7.4 and §7.5 returned to the unnamed form while the link
core was under review and were named again on 2026-10-14 at Court
commit after this record: §7.4 quotes the link core
`0x1fde7250ea1830cd31ec5d3d4dd54404276936c2725308711e88e639b6b9898c`,
and §7.5 names the release digest
`0x5777358de42001e5fc76048e3ada3210df75235af9af18196b1c0492bdc75014`
over `erd.py` and `erdlaw.py`, unchanged in every byte, and that link
core. The dossier corpus (179 cases) agreed with the criterion at seed 1
through the revision. The text's own sha256 at the naming is
`28216bfe643dc6367f59887dc3df25ea053188083cb4f8d1b5691f2e654c0222`.


## Revision in place of 2026-10-17: the posting, the attachments, twenty-two kinds

The judge's directions of 2026-10-17: the index carries `posted` (the listing's posting transaction and document hash, §2), `files` names the listing, the receipt, the entry and the attachments (the private sections' texts, each named in the listing's document by its sha256, §3), the file keys are `index`, `listing`, `receipt`, `entry` and `attachment:<i>`, the kinds are twenty-two (`post`, `cancel` and `revoke` added, `bond` gone), the answer counts the attachments; §5 says how a reader builds the audit basis from block 0 with the judge's keys among `senders` and a pool judge's root from the roster's listed address, and that the buyer's own `init` calldata rebuilds the receipt where the buyer opened; §7.4 embeds the `eravon.link/1` core and §7.5 names this one, both unnamed until the convergence and the contracts' acceptance gate close.

### Round 0: the criterion against its own generator

§1, §2 (the index), §3 (the files and the answer), §5, §6 and §7 were written; HARNESS-DOSSIER's file keys and carried members followed. The criterion (`erd.py`, `erdlaw.py` over the link core) and the generator (194 cases) agreed at seeds 1 to 6.


### Round 1: five readers, three implementers

Five readers (A this law's text, B the standard against the contracts, C `eravon.link/1`, D `eravon.dossier/1`, E the four laws together with the plan, the tooling and MIKNAH's citations) and three implementers (Rust, Node, Python, each written from the texts and the harnesses alone, `ert`, `erl` and `erd` in one program each) read the round-0 texts. The implementers measured zero divergences from the criterion and zero differences from prediction at seeds 1 to 6 on their first measurement of each law, 18 runs of 18 for each, and probed the criterion beyond the corpus (Rust 27,920 terms, 18,871 link and 7,251 dossier inputs; Node 21,772, 21,293 and 12,203; Python 21,581, 39,764 and 4,106) with no disagreement class; one rendering class appeared in Rust's first link run alone, carried faults written with an `ok` member, which the harnesses then fixed (a fault or a verdict under `inner` carries no `ok`, at every depth). Four audit seats read the contracts in the same round (security, the eight vulnerability classes, the adversarial targets, the fuzz and invariant campaign); their findings changed the contracts and the standard's text and touched this law where noted below.

Reader D found 21 items (13 load-bearing) on this law, E 32 (6) across the four, A 20 (8), B 37 (17), C 20 (10). Folded into this law: the Status paragraph; §1 a symbolic link read as the file it resolves to through any number of further links, the file keys as §2.1 defines them; §2.1 `chainId` "equal", `terminal` present exactly where the instance emitted `Closed` (absent after the expiry branch, which closes no case); §2.2 the seat as the last seat opened, `judge` optional for a pool case whose draw was pending or never made, `entryId` where anchored; §2.4 presence; §3.1 `judge` present and equal for a named candidate, attachments by equality with `goodsHash` or `deliveryHash`; §4 the vocabularies closed and the literals separated from the directory names; §5.1 `Seated` and `PanelFrozen` conditioned on `Opened` and on the seat carrying `judge`, the terminal bullet on `terminal` present, the `init` row's `Pulled` where P + `lf` is nonzero with P and `lf` read from the quote bytes the seat's `quoteHash` names and `Priced` where `feeUnit` is nonzero, `advance` with whatever arguments it carries, `sigBuyer` and `sigSeller`, `boardCodeHash` cited to the Shape and CODEHASH.md, the audit root and basis as one `chains` object from block 0 to the head with the lineage fixpoint among `senders`, an entry absent from the audited ledger reported as not recomputed and one held and different as failed; §5.2 `serial` among the exporter's words; §5.3 the rebuild list in full; §6 completeness per transaction (every transaction carrying an event of the instance, `Posted` at `posted.tx`, or a logic log one of §5.1's rows recomputes), `fund` as a `Transfer` log of the token to the instance; §7.1 the fuzzing deferral; §7.4 the re-pin by this law's own act; §7.5 "computes". The standard's two-step guild draw of the same round reached §2.3 and §5.1: `redraw` is the transaction that seats a pool judge, ordinal 0 included, or re-arms the draw and emits `DrawArmed` alone, `init` on the guild track emits `DrawArmed` and seats nobody, and `advance` closes a case whose armed draw nobody made. HARNESS-DOSSIER: a `<dir>` is a path that resolves, links followed, to a directory whose entries can be listed, anything else misuse, and a named entry the process cannot open is unreadable; symbolic-link chains; the `galeed/1` tokens inside `E_DOSSIER_ENTRY_FORM` carry nothing; nested `inner` without `ok`. The criterion followed: attachments through the link core's named hashes, `judge` optional in the seat. The corpus grew from 194 to 220 cases (the placeholder directory removed; a link to a link and an index link to a file; an index name by case; a files name with a dot; a short judge, a short quote and a string number in the seat; a named seat without a judge and a guild seat without one; a guild seat whose quote differs; a short transaction hash and a string timestamp; a transaction row before the terminal and a terminal before `Ruled`; a leading zero and 2^256 in the terminal's amounts; the compliance page's int seller, int serial, conversion time, empty unit and extra member; a subject of another kind, a resignation's subject on another seat, a boolean seat; a prose digest and a covenant text as attachments); zero differences from prediction at seeds 1 to 6.


### Round 2: the same seats over the revised texts

The same eight law seats re-read the round-1 texts (Court 1e643b19): readers A 13 items (3 load-bearing), B 29 (15) with 6 security observations, C 19 (8), D 19 (9), E 19 (5, the plan's 37 items all in the texts, round-1 items 31 of 32 landed and one withdrawn, every tooling check passing, a dry run of the naming act passing all three digest scripts' guards), and the three implementers, each continuing its round-1 program, zero divergences and zero differences from prediction at seeds 1 to 6 on the first measurement after bringing the programs to the revised sentences (the round-1 programs diverged on the revised cases alone), with 58,937 (Rust), 59,674 (Node) and 71,757 (Python) further probe inputs and no disagreement class. The audit seats of round 2 (security, the eight classes, the adversarial targets) found no finding above medium; their findings (the re-arm as a re-roll by delay, a surrender cheaper than a lower rung, a settlement past T2 stripping the judge's P, a draw window shorter than a block, the arming block's producer, uncapped pushes and feed reads, a codeless token at the gate, a Refund() signed earlier landing as the surrender) changed the contracts and the standard and are recorded in the standard's CODEHASH.md paragraphs; the fuzz seat's own finding on the round-1 fold, a double credit at the expiry branch and cancel, was fixed before the round's audits began.

Folded into this law: §1 a symbolic link read where the host's path resolution follows it (a dangling link, a loop, a chain past the host's own limit unreadable), and the definitions of a named and a pool candidate; §2.1 `terminal` present exactly where the instance emitted `Closed`, the buyer's `cancel` included, absent for an instance that holds no case and was not cancelled and after an expiry branch not followed by a cancellation; §2.2 the seat as the seat the order names and, where opened, the last seat opened (a named candidate's `judge` and ordinal 0 whether or not the seat opened), `judge` absent only with `seat` 0 and no `entryId` (`E_DOSSIER_SEAT`), the first draw armed and not yet made; §2.4 `Ruled` requiring the seat's `judge` and `entryId` (`E_DOSSIER_RULED`); §3.1 "named candidate"; §5.1 the `resign` row as a `Resigned` of form 0, 1 or 2 with its `Anchored` (form 3 the closing, emitted by whichever transaction closes), the `advance` row with `DrawArmed` where it arms the next draw, the `redraw` row as a kept successful call that seats the drawn ordinal, 0 included, or, where the seed was no longer served, closes the case as Resigned with form 3, the `cancel` row with its `Closed`, `setPayout` and `withdraw` from the judge of any seat the instance opened, the `init` row naming where the quote bytes come from and the not-recomputed report where a reader holds none, the `consent` row citing `eravon.standard/1` §10; the `Opened` bullet keyed on the slot the call carried (a slot with bytes verifying as the party's signature, the answer's form standing in where the input is unread); a bullet on what the chain holds beyond the dossier (a higher `Seated`, a `Seated` where `judge` is absent, an `Anchored` where `entryId` is absent, a `Closed` where `terminal` is absent: a failed comparison where its transaction is a dossier row's, otherwise reported and read by §6); the root and basis bullet over the registries the reader declares, `JUDGE` taken only where the lineage under `ledger.root` holds the seat's `judge`, `senders` from `resolveIdentity(root)`, the `chainId` object reaching every anchor and the whole basis covering where the document's other objects do, a lineage without the judge or a `NO_LABEL` input reported as not recomputed and every other label read with the ledger the view holds; §5.3 the rebuild conditioned on an `init` that opened the case carrying `listingOffer(S, c)`, no whole dossier otherwise; §6 completeness owing a `Transfer` of the token to the instance (a stranger's included), the logic logs the bullets read for this case (the Consented pairs, the Pulled, a Revoked at or before an expiry branch taken), a `fund` beside the pulls among the further rows, another candidate's registration, a revocation after the close, reposts and a settlement offer posted under `caseId` owed by nobody, and the `chainId` object reaching the case's first block. HARNESS-DOSSIER: `<dir>` under the host's path resolution, a chain past the host's own limit (32 links on macOS, 40 on Linux) unreadable, no corpus chain over eight links. The criterion gained the two seat rules. The corpus grew from 220 to 234 cases (`ok_refunded`, `ok_settled`, `ok_terminal_amount_max`, `missing_transactions`, `compliance_conversion_source_empty`, `compliance_conversion_rate_int`, `file_entry_unread_without_entry_id`, `transaction_two_faults`, `attachment_two_absent`, `attachment_two_unnamed`, `seat_amanah_short`, `seat_no_judge_with_entry`, `seat_no_judge_seat_1`, `ruled_without_judge`); zero differences from prediction at seeds 1 to 6; the README's rows follow, its symbolic-link sentence counting four cases. The standard's §7 of the same round ended the re-arm (a seed nobody drew on closes the case as Resigned with form 3, by `redraw` or `advance`), which §5.1's `redraw` row reflects.


### Round 3: the same seats over the round-2 texts

The same eight law seats re-read the round-2 texts (Court c25210b0; HEAD moved to 74b81962 during the round, a commit touching one test file alone): readers A 7 items (0 load-bearing), B 7 (4) with 2 security observations, C 12 (3), D 16 (8), E 10 (1; the plan's 37 items all in the texts or withdrawn by the plan, round-2 items 19 of 19 landed, every tooling check passing but naming.py's terms Status anchor and the foundry.toml comment, a dry run of the naming act passing all three digest scripts' guards), and the three implementers, each continuing its program, zero divergences and zero differences from prediction at seeds 1 to 6 on the first measurement after bringing the programs to round 2's two seat rules, with 78,476 (Rust), 74,293 (Node) and 88,419 (Python) further probe inputs and no disagreement class on the measuring interface. The audit seats of round 3 (security, the eight classes, the adversarial targets, the fuzz seat) found two findings of low severity, a seller's `refund` sent while Claimed and landing behind the buyer's `dispute` executing as the surrender, and a `refund` while Claimed under a seated seat returning P to the seller so that a refund and a payment off the instance completed the trade with P stripped, and informational items beside them; the findings changed the contracts and the standard (`refund` refused while Disputed at or before T3's end, the surrender row gone, `refundBySig` testing the signature before the phase, a refund while Claimed under a seated seat paying P to the judge as T2's lapse would) and are recorded in the standard's CODEHASH.md paragraphs together with reader B's two observations on return data copied beyond a gas allowance, now copied to a fixed size by the verifier's reads and not at all by a push.

Folded into this law: §1 a resolution past the host's limit counted over the one path the reader opens, the candidate definitions moved into the list, and a slot read (an empty slot the empty string, 65 bytes a hex65, any other slot the six-member object the standard's packing decodes to, read as the contract reads it), which §5.1's `Opened`, `Settled` and `cancel` rows and §5.3 cite; §2.4 `Ruled` requiring `entryId`, §2.2 having made `judge` present; §5.1 `redraw` and `sweep` in both call-reading rules, `redraw` named as the standard's function, the `init` row scoping its not-recomputed report to the `Pulled` comparison, `setPayout` and `withdraw` from the seat's judge or any seat's, the `sweep` row as a `Swept` from any push or a kept call, the `Settled` bullet verifying a slot that carries bytes, the beyond-the-dossier bullet counting `posted.tx` and the `Anchored`'s transaction among the dossier's, `PanelFrozen`'s `index` equal to the `seq`, `senders` with the final address and the trail, and the rebuilt receipt's `candidate` as the `c` of the condition; §6 every `Consented` under the three pairs wherever it stands, and a `Resigned` of form 3 anchoring no entry. HARNESS-DOSSIER: each file read as the one path `<dir>/<name>`, every link that resolution follows counting, the corpus's own chains at most two links. The corpus grew from 234 to 245 cases (`missing_v` through `missing_files`, `compliance_serial_before_conversion`, `ok_file_name_dots`, `link_loop`, `entry_zero`); zero differences from prediction at seeds 1 to 6; the README's rows follow, `seat_amanah_short` moved to the form row and `ruled_without_judge` read as a second `entryId` witness.


### Round 4: the same seats over the round-3 texts

The same eight law seats re-read the round-3 texts (Court ecf085ed): readers A 7 items (1 load-bearing), B 9 (1) with 1 security observation, C 7 (2), D 9 (3), E 8 (1; the plan's 37 items all in the texts or withdrawn by the plan, round-3 items 10 of 10 landed, every tooling check passing but an indentation error the round-3 fold had put into naming.py), and the three implementers, each continuing its program, zero divergences and zero differences from prediction at seeds 1 to 6 on the first measurement (the round-3 programs already met every revised sentence; Node read the previous path sentence to the letter, a `/` always, and found the one case where that and the criterion's joining rule decide apart), with 88,343 (Rust), 83,648 (Node) and 34,326 (Python) further probe inputs; one disagreement class, Node's, on a `<dir>` ending in `/` at the host's path-length limit, which the harness and the law now decide the criterion's way. The audit seats of round 4 (security, the eight classes, the adversarial targets, the fuzz seat) found no finding above informational, their informational items being one sentence of the standard's §5 that still described the round-3 refund, a blank line splitting §8's terminal table, and two stale rows of the design volume; round 4 is the first of the two clean rounds the judge's ruling of 2026-10-17 asks of the contracts, and the code and its identity are unchanged by this fold.

Folded into this law: §1 the slot read's values named as `eravon.terms/1` §5.3's, and the one path the reader opens spelled out (the directory as handed, unnormalised, then `/` unless it ends in one, then the name); §5.1 the `Settled` bullet forming `d` as the EIP-712 digest over the two `uint256` arguments as the call carried them, an `expiry` above 2^53 − 1 included, and the `consent` row's third pair citing it, the `Opened` bullet adding nothing where the slot read is the receipt's `sig` or the receipt's candidate's, the `sweep` row recomputed from a `Swept` in its transaction and read as a call only without one, `setPayout` and `withdraw` reading their caller as the `init` bullet does, and the basis bullet naming `galeed/1` §9.1's anchor form through the registries the reader declared; §6 the three pairs with the first narrowed to the receipt's candidate, and every `Consented` under either key whose digest is neither of the other two reported as not recomputed where the call of `settle` is unread. HARNESS-DOSSIER: the one path as the law spells it, the path-length limit beside the link limit, and the loop of `link_loop` as the one corpus entry whose resolution reaches a limit. The corpus grew from 245 to 268 cases (every required member of every table of §2 absent alone and every table with a member beyond its list; `seat_no_judge_with_entry` at seat 0 so that the `entryId` half alone refuses it); zero differences from prediction at seeds 1 to 6; the README's rows follow (`files_name_dot` moved to the `files` row, the `Ruled` row's second witness explained as a seat §2.2 admits, the member rule stated once).


### Round 5: the same seats over the round-4 texts

The same eight law seats re-read the round-4 texts (Court ebcd2eb5): readers A 5 items (0 load-bearing), B 6 (1) with 1 security observation, C 3 (1, in the corpus alone), D 11 (2, one in the corpus alone), E 2 (1; the plan's 37 items all in the texts or withdrawn by the plan, every round-4 item landed, every tooling check passing), and the three implementers, each continuing its program, zero divergences and zero differences from prediction at seeds 1 to 6 on the first measurement (Rust 5 text items, Node 3, Python 3; Node's one round-4 class is gone, the text now taking the criterion's join rule), with 107,762 (Rust), 91,329 (Node) and 24,050 (Python) further probe inputs and no disagreement class. This is the closing round of the law texts, as the rounds of 2026-09-24, 2026-09-23 and 2026-10-14 closed: the three programs changed in no line and reached zero divergences on their first measurement, and what the readers reported was folded in one pass (three sentences the chain reader of the dossier law decides, one sentence of the standard's §8 on a token outside its rule, two corpus witnesses the programs already passed, and prose), the judge ruling on 2026-10-17 that the earlier standard governs; the three round-5 programs, unchanged, were then measured again over the folded corpora (571, 647 and 293 cases) at seeds 1 to 6, 54 runs of 54 with zero divergences and zero differences from prediction. The audit seats of round 5 (security, the eight classes, the adversarial targets, the fuzz and invariant campaign with a 55-invariant handler, a 2,575-run matrix of every pair of acts in both orders and two deep campaigns) found no finding above informational, their informational items being text and tooling (a starved or failing balance read after a push, the landing order of two acts that reach one instance, two client-rule races, the replay harness's step 9a against the guild track's second-block draw, and the design volume's rows of the earlier money model), all folded; round 5 is the second of the two clean rounds the judge's ruling of 2026-10-17 asks of the contracts, which closes the contracts' gate, the code and its identity unchanged since the round-3 fold.

Folded into this law: §1 the name-folding sentence (a symbolic link's target and the directory's own path resolve by the host's rules, its folding included), the two limits (the links followed and the path's length) and the entry the host refuses to open for that length or for want of permission, both limits counted over the one path; §3.2 the seat number on a resignation that carries one; §5.1 each row decided alone (one transaction the `tx` of rows of several kinds), the `consent` row's digest a pair names under the other key alone being under no pair, the `Settled` bullet's `d` over `buyerBps` the `uint16` and `expiry` the `uint256` with `eravon.terms/1` §4.5 named, the `PanelFrozen` bullet bounding `index` by the element's `seq` only where the `Posted` precedes the `Opened`, the basis bullet reading every carried byte string that passes `galeed/1` §4.3 and is a `succession`; §6 the not-recomputed report conditioned on a `Settled` the instance emitted. HARNESS-DOSSIER: the link limit counted as links followed, the path-length limit counting the terminating NUL (1023 and 4095 bytes the longest paths that open) and macOS applying it to the path as resolution rewrites it, the name-folding parenthetical. The corpus grew from 268 to 293 cases (the duplicate `posted_missing_hash` dropped; every int member with `true`, `subject_seat_bool_pool` at a pool seat 1 among them; every hex member in uppercase; `v_case` and `terminal_name_case`; `instance_zero` and `posted_zero` passing §2's forms and failing at §3.1; `ok_conversions_empty`; `attachment_terms_text`; `compliance_buyer_before_conversion`); zero differences from prediction at seeds 1 to 6; the README moves the member-rule slugs to the fails column, names `seat_missing` and `transaction_missing` there, and states the `true` and uppercase rule once.

Named on 2026-10-17 at Court commit after this record: the release digest
`0x64c96e45993219ae6a0e2d0c2750c2dd154b422ac3345152ff73afea429bf0f8`
over `erd.py`, `erdlaw.py` and the `eravon.link/1` core, as §7.5 quotes it, §7.4 quoting the link digest named by the same act (`0x3e6ba9d8aa3171dde8bd8085d44654cdfb216c8b2f374d6938ddc01c2309b2d5`). The text's own sha256 at the naming is
`c5cf74baa2ed819995b824baeab729f3e32571a92d5103c08bf83f0d83590a67`. The fuzzing this law defers stands as the digest's note records.

## Revision in place of 2026-10-18: the seller's deposit address

The judge's direction of 2026-10-18 (the plan `修法方案-公司卖家押金地址sellerFund-2026-10-18`): a seller that is a company keeps its capital in its wallet and holds nothing at its own address, so a deposit pulled from `sellerKey` could never be found there and no company seller could open a case; the order now names `sellerFund`, the address the deposit is pulled from and returned to, which is the seller's key or an address that bound itself to the seller on the logic (`acceptFunding`), the bond tested at the gate after the consents whether or not the deposit is zero; and a buyer that consents by call counts `openWindow` from its export plus the earliest landing of its consent. The three cores were recast (`Offer` gained `sellerFund` after `sellerPayout`, so every offer and order digest changed), the naming sections of the three laws returned to their unnamed form, the standard changed with the contracts (three registries, `pull` over the fund, five parties, `FundUnbound`, the terminal split computed on the verifier for room: logic 24,131 bytes, verifier 9,450; a new identity table), and the three laws are named again after the rounds below.

### Round 0: the criterion against its own generator

§2.3 the kind `fundBind`, twenty-three kinds; §4 the literal; §5.1 the `init` row's `Pulled` with both addresses and the transfer from `sellerFund`, the `fundBind` row (`FundBound(sellerKey, sellerFund)` of the logic before the `Opened`, owed by no dossier whose `sellerFund` is `sellerKey`), the logic's doings naming it; §6 a `FundBound` owing the dossier no row. The corpus stays at 293 cases, `ok_every_kind` carrying the twenty-third kind; zero differences from prediction at seeds 1 to 6.


### Round 1: the twelve seats over the checkpoint

The twelve seats read the checkpoint (Court 849ad3cb): readers A 8 items (0 load-bearing), B 13 (1), C 4 (2, both in the corpus alone), D 8 (1), E 17 (2, the same two as B's and D's; of the plan's 41 rows 37 in the texts, 1 done otherwise, 1 pending the renaming, 2 the FUZZ seat supplies), and the three implementers, each brought to the revised type string (Rust 28 lines, Node and Python ten lines in three files; before the change 386, 330 and 102 divergences across the three laws, the digests of the old type string), zero divergences and zero differences from prediction at seeds 1 to 6 (Rust 7 text items, Node 7, Python 9), with 123,154 (Rust), 98,900 (Node) and 40,710 (Python) further probe inputs and no disagreement class. The four audit seats over the checkpoint found nothing above informational: SEC 3 informational items (426 proofs in its tree, the whole machine against round 5's at 300 fixed seeds and 5,000 fuzz runs equal), VULN 2 (360 proofs, the split equal to the old in-logic arithmetic over 50,000 runs, thirteen unbound addresses each refused), ADV 3 (369 proofs, 46,656 ordered pairs of acts, 987 fault combinations of the gate's order), FUZZ 5 (sixty invariants under a five-way draw of `sellerFund`, two campaigns of 256 runs at depth 500 at seeds 0x6a and 0x6b with 128,000 calls each, 258 paths reached, eight mutants of the revision caught), the informational items all in the texts and the comments; this is the first of the two clean rounds the contracts' gate asks. Four items bore load, two of the revision and two of the link corpus from before it: the standard's §10 naming the seller's key as the `Swept` creditor where the deposit now returns to `sellerFund`; the dossier law's §5.1 `fundBind` row placing the bond before an `Opened` the instance may never emit; the link corpus leaving nine members of §7.1's three comparison lists unwitnessed alone; and §6.1's nested objects without their closure cases; a reader skipping any of the last two passed every seed. Everything was folded in one pass with the text items, the code's bytes unchanged (`identity.sh` agreeing with CODEHASH.md after the comments and one local name were corrected); the three round-6 programs, unchanged, are measured again over the folded corpora in the next round, the closing round of the texts if that first measurement is zero.

Folded into this law: §5.1 the `fundBind` row conditioned on an `Opened` existing (before it in chain order where the instance emitted one; wherever it stands up to the head the reader read where it emitted none, no gate having read the bond), a row §6 owes no dossier, the bond serving every case whose order names that pair, and witnessed all the same where `sellerFund` is `sellerKey` though the gate reads no bond there; the `init` row's parenthesis naming the event of the one `transferFrom` that `pull` runs, the token's own log being the `fund` row's; the table's opening sending `fundBind` to the logic for the pair the order names; §6 saying which cases a `FundBound` serves. The corpus grew from 293 to 300 cases (`ok_company_fund`, a company seller's dossier with a `fundBind` row before the `init`'s; `ok_zero_edges`, every zero and empty value §2.3 and §2.5 admit; `chain_zero`, `logic_zero`, `codehash_zero`, `case_zero` and `files_sha_zero` passing §2's forms and failing at §3.1); `index_dir` carries a file inside the directory so that the committed tree carries it; the generator's kind note counts twenty-three; zero differences from prediction at seeds 1 to 6; the README's forms, transactions, mismatch and digest rows name them.


### Round 2: the twelve seats over the round-1 fold

The twelve seats read the round-1 fold (Court 90c01f71): readers A 5 items (0 load-bearing), B 7 (0), C 4 (2, both in the corpus alone), D 3 (0), E 8 (0), node 6 (1, in the corpus alone), and the three implementers, each with its round-6 program unchanged, zero divergences and zero differences from prediction at seeds 1 to 6 on the first measurement (583, 668 and 300 cases; 18 runs of 18 for each of Rust, Node and Python), with 145,516 (Rust), 117,318 (Node) and 73,588 (Python) further probe inputs and no disagreement class (Rust 5 text items, Python 7). The four audit seats found nothing above informational: SEC 1 informational item (its round-6 proofs at fresh seeds, the fund invariant widened to every placement of `sellerFund` at 512 runs of depth 128, the grief scanner for a company seller), VULN 1 (370 proofs in its tree, class 4 taken deeper with an opcode census, look-alike proxies, delegation and a clone logic, the split equal to the round-5 arithmetic over 100,000 runs), ADV 2 (1,679,616 ordered triples of acts with a third actor in the same block, §7a sentence by sentence, nine mutants caught), FUZZ none at any severity (the folded handler at 256 runs of depth 500 twice at fresh seeds, 60 of 60; twelve mutants of the revision caught; the handler extended to one fund drawn on by several instances of one seller and to a judge that funds the case it judges, 63 invariants and 295 paths, folded under `test/`); this is the second consecutive round with nothing above informational, and the contracts' gate is closed. This is the closing round of the law texts, as the rounds of 2026-09-24, 2026-09-23, 2026-10-14 and 2026-10-17 closed: the three programs changed in no line and reached zero divergences on their first measurement, and what the readers reported was folded in one pass. The items that bore load were corpus witnesses alone, the texts and the criteria deciding every case already: the link corpus left §6.1's nested objects without a case for each member absent and the receipt's passkey object without a member beyond its six, §7.1's guild array without its index rule on two guilds, and §7.1's ladder without a case on a rung's shares or the ladder's length; a reader departing from the text there passed every seed. The three round-7 programs, unchanged, were then measured again over the folded corpora at seeds 1 to 6, 54 runs of 54 with zero divergences and zero differences from prediction; ten criterion mutants aimed at the new cases each fail on the cases written for them.

Folded into this law: §5.1's self-bond clause witnessed under the same two branches on `Opened`. The corpus grew from 300 to 303 cases (`ok_rows_unrelated`, rows related to nothing: kinds and transactions repeated, blocks and timestamps out of order, rows beside a seat with no judge and no terminal; `ok_company_no_bind_row`, a company seller's dossier with no `fundBind` row; `compliance_seller_before_conversion`); zero differences from prediction at seeds 1 to 6; the README's transactions and compliance rows name them.

Named again on 2026-10-19, at the Court commit after this record: the release digest
`0x0a6b5b321985f2f22f4f66752695b86fe0e23b00260bacf4d2a654db6f92932e`
over `erd.py`, `erdlaw.py` and the `eravon.link/1` core, as §7.5 quotes it, §7.4 quoting the link digest named by the same act (`0x29ff5e4651ee99cdc94521157ef96ad1e943473bea645cc24a569fb4429c80ef`). The text's own sha256 at the naming is
`0f92ce9cae368316dd58b6ee8d804949d96dee306fe23461feda5cfa17cb87a1`. The fuzzing this law defers stands as the digest's note records.

## Revision in place of 2026-10-24: the succession book, the venue's new interface, and two of the family's own

The judge's revision of 2026-10-24 (`逐条修订稿-换钥簿与场所新接口及本家两项-2026-10-24`, 186 items over the four ERAVON laws, with the full revised texts beside it): the family reads every key through GALEED's succession book (`galeed.roster/1` §4), so that a party's, a judge's and a guild's key is its record's current key on the book the standard's code names; the standard follows the venue interface's new form (`galeed.venue/1`: `partyRoles`, `outcomeOf`, `advanceOf`, `successionBook`, `routeGas`, the seat's two credited fields, a panel's router called with exactly `routeGas()`, one panel book per chain filing under the guild's root); and two items of the family's own: the quote loses its payout word, a judge registering its own payout on the instance, and `lapseDefault` admits 1, an even split at a lapse; the passkey (P-256) form is removed from the family. For this law: the entry rules of `galeed/1` §4.3 carried in the core itself (`erdentry.py`, taken from GALEED's current core, migration included) in place of an embedded `galeed/1` core, and the passkey case gone with the form. The naming sections §7.4 and §7.5 returned to their unnamed form.

### Round 0: the criterion against its own generator

The items applied in place with the other three laws (Court 9ab8f451). The criterion: `erdentry.py` new (GALEED's current `glentry` over `ertcanon` and `ertcrypto`, with the EIP-191 digest and message inline), `erdlaw.py` and `erd.py` importing it; `criterion-digest-dossier.sh` adds `erdentry.py`. The comparator and the generators read GALEED's base core from `galeed-conformance/impl-py`, and `eravon-conformance/impl-py` with `criterion-digest.sh` is removed. The corpus holds 303 cases at seed 1 (`entry_type_migration` added, the passkey case removed); zero differences from prediction at seeds 1 to 6. Over the old corpus the old and the new criterion differ in 1 answer, the revision's.

### Round 1: the twelve seats over the round-0 checkpoint

The twelve seats read the round-0 checkpoint (Court 16c772f9): readers A (this law) 9 items (0 load-bearing), B (the standard against the contracts and the replay) 10 (2), C (`eravon.link/1`) 9 (5, four in the corpus alone), D (`eravon.dossier/1`) 10 (2), E (the four laws against the judge's draft, the tooling and the corpora) 16 (1), and the three implementers, each starting from its round-7 program: measured unchanged, the round-7 programs diverged on the revision's classes alone (terms 52, link 56, dossier 0 at every seed: the 480-byte quote, the object signature, the three identity commands, the pack without its key), and, brought to the revised laws from the texts and the harnesses alone, each measured zero divergences and zero differences from prediction at seeds 1 to 6, 18 runs of 18, with 62,288 (Rust), 70,441 (Node) and 20,096 (Python) further probe inputs and no disagreement class (Rust 8 items, 1 load-bearing; Node 5, 2; Python 6, 0); Node also checked the standard's EIP-712 vectors, 231 comparisons with none differing. The four audit seats read the new contract generation: SEC one low and two informational items, VULN, ADV and FUZZ nothing above informational (4, 3 and 3); the low item, a router's answer naming a panel's zero entry, was verified a text item (the standard's audit record). One item bore on this law: the entry test this core now carries among its own files (`erdentry.py`) was measured by nothing, §7.1 still witnessing `E_DOSSIER_ENTRY_FORM` once as another document's vocabulary; fourteen mutants of it passed every seed, and so did the pre-revision criterion without `galeed/1` §6.6a's table (seats D, Node, Rust, Python). Folded into this law: §7.1 takes `E_DOSSIER_ENTRY_FORM` with each token `galeed/1` §4.3 reaches from its step 2 on and `E_BODY_FIELD` under each type of §6; §7.4 names the passages of `galeed/1` it reads as they stand when §7.5 names the core; §1 reads a slot as `eravon.standard/1` §9 reads one, takes the succession book `eravon.link/1` §1 names, and reads an address's root at a call as it reads a current key; §5.1's unread call reports a `secp256k1` party's current-key fact as not recomputed; the `PanelFrozen` bullet reads the root at the call of `init` and the panel book's record under `(root, proxyCodeHash)`, the clause on the `Posted`'s order gone; the judge's audit root for a named candidate is the first key of the record of the seat's `judge`, the basis's `senders` add every key of that record and of the root's, and the judge is resolved over the book §1 reads. The corpus grew from 303 to 332 cases: `entry_form_envelope` through `entry_form_signer` (every token of §4.3 from step 2), `E_BODY_FIELD` under each of the eight types (`entry_form_body_genesis` through `entry_form_body_keybinding`, the migration's two among them), and the adjacent steps (`entry_form_sigform_before_place`, `entry_form_place_before_body`, `entry_form_body_before_sig`); zero differences from prediction at seeds 1 to 6; `bcov.py` measures `erdentry.py`. The router's zero entry (seat D's other item) is the standard's text, folded there.

### Round 2: the twelve seats over the round-1 fold

The twelve seats read the round-1 fold (Court 9dd79a98): readers A (this law) 7 items (2 load-bearing, both in the corpus alone), B (the standard against the contracts and the replay) 5 (0), C (`eravon.link/1`) 4 (2, both in the corpus alone), D (`eravon.dossier/1`) 3 (1, in the corpus alone), E (the four laws against the judge's draft, the tooling and the corpora) 9 (1, in the corpus alone), and the three implementers: measured unchanged, the round-1 programs diverged on the one change the round-1 fold made to the criterion's answers and on nothing else (terms 0, link 10, dossier 0 at every seed: `history`'s separate `source` member), each wrote that member with one edit, and so changed each measured zero divergences and zero differences from prediction at seeds 1 to 6, 18 runs of 18, with 93,312 (Rust), 65,282 (Node) and 45,935 (Python) further probe inputs and no disagreement class (Rust 5 items, 0 load-bearing; Node 4, 1; Python 2, 0); Node checked the standard's EIP-712 vectors again, 231 comparisons with none differing. The four audit seats found nothing above informational on the contract code (SEC 4, VULN 5, ADV 2, FUZZ 3), the compiled bytes unchanged since 16c772f9; the judge ruled on 2026-10-25 that round 1 counts as the new generation's first clean round, so the contract gate closed at this round (the standard's audit record). The fold changes no answer of any criterion, the three digests standing as they were (terms `0xca99…4a9a`, link `0x9465…28e5`, dossier `0x7e2c…86f0`): it adds corpus cases and changes text, so the round-2 programs, unchanged, measure round 3 as the closing round if they show zero at seeds 1 to 6 on their first run. The one load-bearing item fell on this law's corpus (seats D, E, Rust, Node, Python): the round-1 cases witnessed the carried entry test at its tokens and at three of its step boundaries, and 25 mutants of `erdentry.py` passed every seed (twelve orders of adjacent steps or tokens, five passing sides of the type tables and an unknown type, three member forms, §6.4's clause rule kept in its forbidding half, the `hash` subject table, a `sig` nested in the body, step 1 skipped), six of them certifying as whole a dossier the criterion refuses. Folded into this law: §7.1 takes `E_DOSSIER_ENTRY_FORM` with every input of the `galeed/1` corpus's `check` set that passes `galeed/1` §3.5, each carried as the entry file of an otherwise whole dossier and predicted as that corpus predicts it, and once with a token of §3.5, so that the carried section is measured by the witnesses of the law it is copied from and no hand list is kept (seat D's inversion; seat D measured all 64 mutants of the carried test differing on that corpus), and §7.5 records the fuzzing over §1's file names and the carried entry test (seat D); §7.4 names what the pin covers (`galeed/1` §1, §4.1 through §4.3, §5.1 through §5.5, §6 and §10's tokens) and reads §3 and §5.4's arithmetic through the embedded `eravon.terms/1` core as that law's §10.4 pins them, its own implementation living in `erdentry.py` (seats D, Node, E); §1 and §7.5 name `erdentry.py`, the file the digest script hashes, `naming.py`'s anchor following (seat E). The corpus grew from 332 to 1,187 cases: the 854 inputs of `galeed-conformance/corpus/canon/` that pass `galeed/1` §3.5, each the entry file of a copy of `ok_ruled` (`gl1_*`: a token of §4.3 carried, or an accepted entry going on to `E_DOSSIER_ENTRY_TYPE`, an unknown type among the eight, or `E_DOSSIER_SUBJECT`), and `entry_not_canonical`; zero differences from prediction at seeds 1 to 6. The README's entry row names the imported set and this corpus's own cases, and the migration cases as two. `naming.py` carries the record of the fuzzing run for the core named on 2026-10-19 beside the new digest, bound to that core. Recorded and not folded: seat D's 26 probes and seat Node's 46 witness dossiers, each a hand list the imported set covers.

### Round 3: the eight law seats over the round-2 fold

The eight law seats read the round-2 fold (Court 7e4001bb); the contract gate having closed at round 2, no audit seat sat. Readers A (this law) 6 items (2 in the corpus alone, tagged load-bearing by seat A), B (the standard against the contracts, the tests and the replay) 3 (0), C (`eravon.link/1`) 5 (0), D (`eravon.dossier/1`) 1 (0), E (the four laws against the judge's draft, the tooling and the corpora) 6 (0); the three implementers, their round-2 programs unchanged, measured zero divergences and zero differences from prediction at seeds 1 to 6 on their first run, 18 runs of 18 each (terms 4,179, link 708, dossier 1,187 cases per seed), with 157,270 (Rust), 76,093 (Node) and 57,292 (Python) further probe inputs and no disagreement class (Rust 3 items, Node 2, Python 4, none load-bearing); Node checked the EIP-712 vectors once more, 231 comparisons with none differing. So round 3 is the closing round the gate asks for: the previous round's programs unchanged, zero at seeds 1 to 6 on the first measurement, no item that two careful implementations could read apart. Every item of the round is text or corpus, folded here in one pass; the criteria stand (terms `0xca99…4a9a`, link `0x9465…28e5`, dossier `0x7e2c…86f0`). The naming waits: the judge's plan of 2026-10-26 (stages, prepayment interlocked) is applied next in this generation and changes the three criteria, so the texts reopen under it. Folded into this law: §7.4 pins every GALEED passage this text cites at the naming, `galeed/1` §1, §4.1 through §4.3 with every passage of §5 and §6 that §4.3 invokes (§5.1 through §5.3, §5.6 and §6) and §10's tokens among them, and reads §3, §5.4 and §5.5 through the embedded `eravon.terms/1` core as that law's §10.4 pins them, each passage pinned once (seats D, Node, Rust, E); the preamble cites §7.4; §7.1 names the imported set at seed 1 as committed and adds every adjacent pair of §4.3's tests that set leaves unwitnessed (seat Python). The corpus grew from 1,187 to 1,189 cases: `entry_form_closed_before_spec` and `entry_form_prevseq_before_body`, the two pairs of seat Python's two surviving mutants; zero differences from prediction at seeds 1 to 6. Seat D measured all 64 mutants of the carried entry test caught at every seed by the imported set, and `bcov` leaves four arms of the core unwitnessed, three in `erd.py`'s handlers and one unreachable after step 1.

## Revision in place of 2026-10-26: stages, a sale delivered in parts under one judge

The judge's plan of 2026-10-26 (`修法方案-分段交付系列标识-2026-10-26`, its fourth version, with the ruling of the same day that a stage ended Lapsed carries no judge forward), applied in one pass as round 0 of the 2026-10-24 generation after that generation's round 3 closed the texts and before their naming: an order names the stage before it (`prev`), the standard opens a later stage only once the stage before is paid and lets the buyer cancel it only once the stage before failed, a document may declare the series its stages belong to, and the draw without a router seats the stage before's judge first. The three criteria change with it, so the texts reopen under it; the contract bytes change, so the contract gate restarts at this generation's next round (Court 56081380). The twelve seats sit again over this checkpoint.

### Round 0: the plan applied in place

§5.3 names `prev` among the `OrderTerms` members the receipt carries;
nothing else of this law changes, and the criterion's own files are
untouched, its digest changing through the embedded link core. The corpus
holds 1,189 cases as before, every receipt regenerated with `prev` (zero
throughout, the corpus carrying first stages alone); zero differences from
prediction at seeds 1 to 6.

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
under the same two keys, a client rule. Folded into this law: §7.1 asks that
a first stage and a later stage, the receipt's `prev` zero and nonzero, be
witnessed as whole dossiers under a named judge, a guild's pool and consent
by call (seats D, Rust, E: every receipt of the corpus carried `prev` zero,
so three link-side mutants that drop or zero `prev` passed the dossier
corpus at every seed while the link corpus killed them). The corpus grew
from 1,189 to 1,197 cases: `ok_later_stage`, `ok_later_stage_pool`,
`ok_later_stage_by_call`; `later_stage_instance_of_first` and
`later_stage_case_of_first` (the index naming the first stage's instance or
salt, `E_DOSSIER_INSTANCE_MISMATCH`, `E_DOSSIER_CASE_MISMATCH`);
`later_stage_prev_unsigned` (the receipt's `prev` changed after signing,
`E_DOSSIER_ORDER` carrying `RECEIPT_SIG`); `later_stage_prev_form` and
`later_stage_adj_prev_before_sig` (`E_DOSSIER_RECEIPT` carrying
`E_RECEIPT_PREV`); zero differences from prediction at seeds 1 to 6. Seat
D's reading stands as this law's: the stage before is the receipt's `prev`,
the series lives in the listing's document, and the two gates need no
recomputation in a dossier, an `Opened` or a cancelling `Closed` meaning
they passed. The README's row and this record's Round 0 sentence follow
(seat D).

### Round 2: the twelve seats over the round-1 fold

The twelve seats sat over the round-1 fold (Court f5b6a5d2). The three implementers, their round-1 programs unchanged (Rust's binaries byte-identical, Node's and Python's sources checked by digest), measured zero divergences and zero differences from prediction at seeds 1 to 6 on their first run, 18 runs of 18 each (terms 4,238, link 4,505, dossier 1,197 cases per seed), with 26,584 (Rust), 27,539 (Node) and 50,718 (Python) further probe inputs and no disagreement class (Rust 0 items, Node 2, Python 2, all text). Readers A (`eravon.terms/1`) 3 items, B (the standard against the contracts) 8 (one of them on the contract's code, informational, folded as text by the judge's ruling), C (`eravon.link/1`) 4, D (`eravon.dossier/1`) 3, E (the four laws against the plan and the tooling) 4 and a family report; every item text or corpus, none load-bearing, every seat saying it would name. The four audit seats found nothing above informational on the fold's bytes (the audit record), so the fold changes no byte. **So round 2 is the closing round the gate asks for**: the previous round's programs unchanged, zero at seeds 1 to 6 on the first measurement, no load-bearing item; the residue is folded here in one pass (Court f78f62c8), and the folded sentences are read once more by the readers of the next round, which is the audit's second round over these bytes; the naming follows the contract gate (`law-revision-standard` 3.1: the type string and the verifier are shared with the contract). The judge's rulings of 2026-10-26 on the round's two questions: an inherited series judge who declines or resigns a later stage stays the series judge (the parties end the series by a dispute where they will not go on; no judge is drawn to replace it for that alone), and an account of other code than the proxy's that proves an address at the doors is a text matter (no reader admits it; events are read by the instance's address), the bytes standing. Folded into this law: §7.1's clause names its witnesses in §3.1's terms, a first and a later stage each under a named candidate, a pool candidate and the buyer's consent by call (seat D). The corpus is unchanged at 1,197 cases; zero differences from prediction at seeds 1 to 6. The README's receipt row cites §3.1 and `eravon.link/1` §4.4 in place of the `[O]` rule §5.3, and the generator's notes follow (seat D). Seat D's reading of a later stage's dossier stands: the stage before is the receipt's `prev`, the series lives in the listing's document, and an `Opened` or a cancelling `Closed` means the gate passed.

### Round 3: the re-reading of the round-2 fold

The twelve seats sat over the round-2 fold (Court 86dc194c), the re-reading `law-revision-standard` 3.2 asks for. The three implementers, their programs unchanged since round 1, measured zero divergences and zero differences from prediction at seeds 1 to 6 on their first run, 18 runs of 18 each (terms 4,239, link 4,505, dossier 1,197 cases per seed, the new `series_stages_bool` among them), with 29,172 (Rust), 30,568 (Node) and 57,178 (Python) further probe inputs and no disagreement class (Rust 1 item, Node 3, Python 3, all text). Readers A 2 items, B 4, C 4, D 2, E 2 and a family report; every item text, none load-bearing, every seat ready to name the moment the contract gate closes. The four audit seats found nothing above informational on the bytes of the round-1 fold, the second consecutive clean round over them, **so the contract gate closed** (the audit record). The residue is folded here in one pass (Court a29c4a10): the sentences the round-2 fold had added are read as one reading now, or reworded where two readers took them two ways; no criterion file changes and the three digests stand. The naming follows. Folded into this law: §7.1's clause lists its witnesses on their two axes (each under a named candidate and under a pool candidate, with a signed receipt and with the buyer's consent by call; seat Python); the README's receipt row and the generator's note say the instance and salt are the order's, `prev` among its members (seat D). The corpus is unchanged at 1,197 cases (three notes reworded); zero differences from prediction at seeds 1 to 6.

### The naming of 2026-10-27

The contract gate having closed at the stages' rounds 2 and 3 (the audit record), and the texts at round 2 with their residue read once more in round 3 and by a re-reading seat over the round-3 fold (16 of 18 sentences read as one reading, two reworded: terms §8's lock sentence and dossier §7.1's witness clause), `naming.py 2026-10-27` named the three cores in the order they nest, each note carrying the fuzzing of this revision (the three seats' 951,220 differential inputs over its six rounds: Rust 391,921, Node 295,290, Python 264,009) and, after it, the record of the fuzzing run for the core named on 2026-10-19. After the act the three digest scripts recomputed the digests the sections carry, `compare.py` at seeds 1 to 6 exited 0 for the three laws, `forge test` passed 335 of 335 and `identity.sh` agreed with CODEHASH.md (Court 6d561dad). This law's §7.5 (§7.4 pinning the `eravon.link/1` core) names `0x93ae4df7f2058a724d5da8ef38925e7d3b99a2f2861c40b4a42d3c61a350bf3d`, superseding `0x0a6b5b321985f2f22f4f66752695b86fe0e23b00260bacf4d2a654db6f92932e` (named on 2026-10-19, the sellerFund revision) and the unnamed candidate `0x7e2c…86f0` of the 2026-10-24 revision's rounds 1 to 3, which the stages' `prev`, `series` and receipt member replaced before any naming. The text's sha256 after the act is `79bbc1bc30bdd28773b902cce31cd5ea79c8db64966fc8d6d776a5f2a83dfd44`.

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

The passages §7.4 freezes, `galeed/1` §1, §4.1 through §4.3 with what §4.3
invokes, and §10's tokens, read as they did. The reader's bullets of §5.1
and §6 cite `galeed/1` §5.7, §7.4 and §8 and `galeed.roster/1` §9, which
moved: a reader recomputing a judge's audit now reads the lineage and the
trim of `galeed/1` as revised on 2026-10-30. `galeed.papers/1` §9.0 and §9.2
read as they did. The core's reading is unchanged.

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
`0x7ec2c54da3ac8fbbae93ce50902d16ad17ff3a194310e567110ce9e2c2752135` (before
it, `0x79bbc1bc30bdd28773b902cce31cd5ea79c8db64966fc8d6d776a5f2a83dfd44`).
