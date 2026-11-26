# Boundary audit: both sides of every closed boundary

A law is closed at a boundary when the criterion decides every input on one
side or the other of it. A boundary is **witnessed on both sides** when the
corpus holds an input the criterion decides one way and an input it decides
the other way, so that an implementation which draws the line anywhere else
diverges from the criterion on at least one case. This audit measures that
mechanically for the criterion programs and their corpora, then reads every
boundary the corpora had not witnessed against the law, and fills the ones
the law decides.

## Method

`bcov.py core|scan|papers|roster <out.json>` runs one criterion program in-process
over its corpus with a tracer that records every executed line-to-line arc
inside the criterion's own files. Every `if` and `elif`, every loop, and
every exception handler in those files is a branch point; a branch point
whose arms were both taken by some case is witnessed on both sides; the rest
are listed with their source line. The measure is a tool in the sense of the
first principle of this repository: it never enters a digest, the criterion
is not modified for it, and a reading it gives is only as good as the
reading of the law that follows it.

The corpus generator already counts the closed vocabularies the law names
(every token, verdict, label and finding name) and reports each one witnessed
in the output of the criterion; it reports none unwitnessed. Branch arms are
the finer measure: they see a boundary the vocabulary cannot, such as the
positive side of a rule whose negative side already has a token.

## Result, seed 1 (the revisions of 2026-10-16, the chain as the ledger, of 2026-10-23, retirement and migration, and of 2026-10-30, the second of the handing off)

| program | corpus | comparisons | branch arms | arms not both-sided |
|---|---|---:|---:|---:|
| `impl-py` (galeed/1 criterion) | `corpus/` | 8074 | 660 | 27 |
| `scan-py` (§9 scanner) | `galeed-core/fixtures/` | 76 | 350 | 71 |
| `papers-py` (galeed.papers/1 criterion) | `papers-corpus/` | 693 | 509 | 1 |
| `roster-py` (galeed.roster/1 criterion) | `roster-corpus/` | 660 | 509 | 7 |

The numbers are read under Python 3.12 with `pycryptodome` present, the
interpreter the criteria are run with; under another interpreter the tracer
attributes a few loop and compound-condition arms differently, and where
`pycryptodome` is absent the scanner's Keccak fallback runs and its arms
leave the count. Every arm that remains is classified below; none of them decides a label, a
token, a verdict, or a record. The `impl-py` figures are the two-input core's
(the anchor set and the basis; the ledger the carried entries a counted
undiscarded record carries; `UNPROVEN` as a label; the `entry` of an
`UNPROVEN`, `VOID` or `DISCARDED` row; no supplied byte strings, no retrieval
set, no second basis member): every branch point the audit of §8 states is witnessed on
both sides by the carried cases of seed 1, and the twenty-seven that remain
are, arm for arm, the harness-misuse and unreachable classes below; the
round-1 cases of the review (the sign surface at its edges, the misuse argument
vectors through `compare.py`, the hash-only record beside a non-counted
carrier, the case variants of a type and a kind) took seven arms off the
earlier count of thirty-four. The revision of 2026-10-23 added seventeen arms
(the retirement of §7.4 and its test at a record, the lineage's admission
by a record at which the succession's signer has not retired, the trim's
second ground, the `migration` arm of the walk and its precedence over a
succession at one position, the `migration` body table); the
retirement and migration cases of `corpus/audit/` (`retire_*`,
`retired_*`, `migration_*`) witness each on both sides, at the
retirement's own second and one second after it, on one chain and on
two, and the twenty-seven that remain are the same arms as before. The
revision of 2026-10-30 added six arms, the three tests of `retired_at` (no
retirement, a record later than it, a record of its own second that carries
bytes); the retiring seq and the carried entry's `judge` and `seq` are
compared inside one condition each and have no arm of their own, and the
same-second cases of `corpus/audit/` witness every outcome of those
comparisons on both sides (the section on that revision below); the
twenty-seven that remain are, once more, the same arms.
The `scan-py` row is the scanner's measure over the seventy-six fixtures of
the one form: the scanner recovers every sender from the transaction's
signature (§9.1), tests the ordering and uniqueness rules of §9.4 with class
items, keys records on `(chainId, blockNumber, tx, hash)` and lets any log of
the key carry the entry, decodes the form's data into the entry or into
nothing, admits an emitter by address or by the code hash of its account at the
log's block, reads the chain's head before any log and fails on a range past
it, fails on a chain that answers `eth_getLogs` with an error or with no
array and on a signed envelope it cannot decode, refuses a transaction whose
envelope names another chain, and refuses creations and signature-less
transactions; the fixtures witness those arms, and the seventy-one that
remain are the environment and unreachable classes below (the keccak
fallback's arms among them, which run only where pycryptodome is absent).

**Seeds.** The corpus mixes cases built by construction with seeded
mutations, so an arm can be witnessed by luck under one seed and missed under
another. Seeds 2 to 6 were generated beside seed 1 and compared the same
way: every seed's predictions agree with the criterion on every case of the
four corpora (`compare.py`, `compare-roster.py`, `compare-papers.py`,
`compare-evidence.py`, the criterion standing as its own candidate), and the
two classes that once flickered between seeds in an earlier generator (a text
ending where a value should begin, and the order of the two ids an
`EQUIVOCATION` names) are closed by the same constructions as before.

## Two boundaries closed by construction

**Grammar states (§3.5).** Every arm of the parser is a state of the grammar
meeting either the end of the text or a byte the state does not admit. The
corpus holds, seed-free, every prefix and every single-byte substitution
(from a thirteen-byte alphabet covering each byte class the grammar
distinguishes) of nine short templates that together reach every construct:
object, array, every escape and a `\u` escape, integers, the three literals,
nesting, whitespace, multibyte UTF-8, a duplicate key. About 2,650 `canon/`
cases from the templates, and about 3,100 seed-free cases in all, deterministic
under every seed; a state the templates reach has both
its end-of-text arm and its junk-byte arm witnessed, and a junk byte no one
thought of is covered with the rest.

**Bytewise order (§8.2, §8.4, §8.7).** Wherever the law orders two entries
by `entry_id`, which of two hashes is smaller is luck, so the generator
builds each such case in both orders by a deterministic search over the
competitor's prose: the two-way fork, two successions competing at one seq
(authority follows the smaller id, so which seq-4 entry is an
`AUTHORITY_MISMATCH` flips), two entries on one `prev`, and a retired key's
succession competing with the honest one. Wherever the law sorts a list of
the report or says the report does not depend on input order (§8, §8.7),
every audit case with array inputs registers a twin with every input array
reversed, so each sorted list is witnessed from an input in another order.

## Boundaries of the revision of 2026-10-23, filled in `corpus/gen.py`

A key's retirement (§7.4) and the trim's second ground (§8.1) were decided,
at that revision, by comparisons the criterion made with one arm each, so
several of their boundaries have no branch of their own; the corpus
witnesses each on both sides by hand-built cases at distinct block times,
each with its reversed twin:

- **what retires a key**: a record of any verdict (`retire_by_void_record`,
  `retire_by_unproven_record` and its same-second companion) and any sender
  (`retire_by_strangers_record`, `retire_by_strangers_earlier_record`); only
  a record that carries the bytes (`hash_only_anchor_of_succession_retires_nobody`)
  of a succession or a migration that passes §4.3
  (`malformed_succession_retires_nobody`, `seq0_succession_retires_nobody`,
  `seq0_migration_retires_nobody`, `other_spec_succession_retires_nobody`,
  `resignation_and_keybinding_retire_nobody`,
  `unknown_type_spelled_like_migration`), a succession to the key itself
  included (`retire_by_succession_to_self`); the smallest time over every
  such record on any chain, whichever of the succession and the migration
  is carried first (`retire_smallest_time_across_chains`,
  `retire_min_across_succession_and_migration`,
  `retire_min_migration_before_succession` and its same-second companion),
  a stray migration at a position never held
  (`stray_migration_retires_before_succession`,
  `stray_migration_same_second_as_succession`), and a key that retires
  before it is named (`retired_before_named`,
  `retired_before_named_successor_carries`); the retirement's own second
  and one second after it (`retire_stray_same_second`,
  `retire_stray_after`), across two chains with a stray on a third
  (`two_chain_tie_stray_same_second`, `two_chain_tie_stray_one_later`), at
  second 0 and at 2^53 − 1 (`retire_at_second_zero`,
  `retire_at_second_zero_one_later`, `retire_at_the_top_of_the_range`,
  `retire_at_the_top_stray_at_the_top`);
- **the trim reads no verdict**: a retired key's `UNPROVEN` and `VOID`
  records after its retirement (`retired_key_unproven_record_after`,
  `retired_key_void_record_after`) beside the same record at its second
  (`retired_key_unproven_record_same_second`); junk at the second and after
  it (`retired_key_junk_same_second`, `retired_key_junk_after`);
- **the lineage's admission**: a late fork, a late succession, a second
  succession at a held position, a stray succession at a position never
  held (`retired_fork_*`, `retired_key_late_succession*`,
  `second_succession_*`, `stray_succession_at_a_position_never_held`), and
  a ledger succession the lineage does not admit, which moves the walk to a
  key outside the lineage (`walked_authority_outside_the_lineage`), that key
  admitted later by a lineage key's succession that is no ledger entry, or
  not where that key had retired first
  (`walked_outsider_admitted_by_void_succession`,
  `walked_outsider_admitter_retired_first`);
- **migration**: closing, beside a succession, out of office, two at one
  position, across a gap before and after it, carried `UNPROVEN` and
  `VOID` (`migration_*`, `two_migrations_at_one_position`,
  `closure_then_gap_same_second`, `closure_after_gap`), a migration out of
  office retiring its signer (`out_of_office_migration_retires_its_signer`),
  and a retired key's migration re-carried by the successor
  (`retired_key_migration_recarried_by_successor`); the new law on two
  chains read whole and with one chain cut, beside a genesis at the second
  of its chain's own carrier (`migration_two_chains_*`), and between the two
  chains' carriers (`new_law_between_carriers_*`); a seat that kept §5.7
  read whole and with one chain cut below the migration
  (`migration_kept_rule_*`);
- **the chains' clocks**: a chain whose clock runs behind retires a key
  before an earlier entry's record, and the successor re-carries it
  (`lagging_chain_*`); a succession dated before the true hand-off
  (`early_clock_late_succession`).


## Boundaries of the revision of 2026-10-30, filled in `corpus/gen.py`

The second of the handing off (§7.4): a key's retiring seq is the smallest
`seq` among the successions and migrations it signed that a record at its
retirement carries, and the key has retired at a record of that second that
carries a byte string passing §4.3, signed by the key, at a `seq` greater
than its retiring seq. The criterion decides this with six arms, each
witnessed on both sides at seed 1 by hand-built cases with their reversed
twins:

- **what the key signed at a position after its handing off, carried in
  that second, is `DISCARDED`**: a resend by the key itself
  (`retired_key_extra_entry_same_second`, beside
  `retired_key_extra_entry_after` one second later), on another chain of the
  basis (`retired_key_extra_entry_same_second_other_chain`), a fork
  (`retired_fork_same_second`, `fork_by_retired_key`), a late succession that
  admits nobody (`retired_key_late_succession_same_second`,
  `fork_retired_key_succession_ids_*`, `succession_at_k_none_in_office`,
  `succession_at_k_none_in_office_successor_signs`,
  `succession_signed_by_retired_key_counted`), a migration out of office
  (`migration_by_retired_key_out_of_office`), an entry past a migration
  (`migration_then_entry_same_second`, `closure_then_gap_same_second`,
  `closure_after_gap`, `migration_carried_void_entry_same_second`), and the
  key's own entry after a succession to itself (`succession_to_root`,
  `succession_to_prior_holder`, `uncounted_self_succession_to_root`,
  `void_only_self_succession_beside_counted_twin`,
  `void_only_self_succession_beside_root_entries`);
- **what counts in that second as at any earlier record**: an entry the key
  signed at a position before its handing off
  (`retired_key_backfill_same_second`), the successor's entry the old key
  carries (`successor_entry_carried_by_retiring_key_same_second`), a
  stranger's entry the old key carries
  (`foreign_entry_carried_by_retiring_key_same_second`: `CARRIED_MALFORMED`,
  `BROKEN_CHAIN`), a hash-only record (`retire_stray_same_second`,
  `two_chain_tie_stray_same_second`, `retire_at_the_top_stray_at_the_top`),
  junk (`retired_key_junk_same_second`), new-law bytes
  (`migration_new_spec_same_second`), and the handing off itself
  (`retired_key_unproven_record_same_second`,
  `retire_by_unproven_record_stray_same_second`);
- **the retiring seq**: the smallest `seq` among the handing offs carried at
  the retirement's second, so a stray migration at a later position in that
  second is discarded while the succession at the smaller position admits
  its key (`stray_migration_same_second_as_succession`), and two handing
  offs at one position both sit at the retiring seq, so both records stand
  and the fork is found (`two_migrations_at_one_position`,
  `second_succession_same_second`).

The fifty-four cases of the corpus of 2026-10-23 whose reports the revision
changed, twenty-eight of them in their label, are listed in the review
record's round 0 of this revision; in fifty of them every difference is a
newly discarded record of its sender's retirement second carrying the
sender's own entry above its retiring seq, and in the other four the
lineage refuses a succession carried only in such records, so that its `to`
and that key's own record leave the ledger.

Round 1 of the revision added sixteen scenarios with their twins at the
boundaries the seats found unwitnessed:

- **the record's sender against the entry's signer**: the successor's
  record of that second carrying the ceding key's entry or late succession
  at a position after its retiring seq stands and its bytes are ledger
  entries (`retired_key_extra_entry_carried_by_successor_same_second`,
  `retired_key_late_succession_carried_by_successor_same_second`), a
  stranger's record of that second carrying the late succession admits
  nobody (`retired_key_late_succession_carried_by_stranger_same_second`),
  the ceding key's hash-only record of its own late entry stands beside its
  discarded byte-carrying twin and one second later falls with it
  (`retired_key_extra_entry_hash_only_beside_bytes_same_second`,
  `retired_key_extra_entry_hash_only_beside_bytes_after`), the handing off
  carried by the successor alone dates the retirement and sets the retiring
  seq (`handoff_carried_by_successor_only_resend_same_second`), and the
  clause reads no earlier record
  (`retired_key_extra_entry_anchored_before_its_handoff`);
- **what sets the retiring seq**: the byte strings that pass §4.3 and
  nothing else, so a body fault or a wrong signer in succession-shaped
  bytes a stranger carries in that second moves it by nothing
  (`forged_succession_body_fault_same_second`,
  `forged_succession_wrong_signer_same_second`); a lower handing off of the
  key riding a stranger's `VOID` record moves it down and leaves the
  successor outside the lineage
  (`lower_stray_handoff_sets_retiring_seq_same_second`, beside the ceding
  key carrying the successor's entry, `CARRIED_MALFORMED` at `FOREIGN` and
  `BROKEN_CHAIN`,
  `lower_stray_handoff_successor_entry_carried_by_ceding_key_same_second`);
  a higher succession on another chain in that second admits nobody
  (`late_succession_on_chain_2_same_second`); a migration and a succession
  of one second keep the lower (`migration_then_succession_same_second`);
- **the verdict of the record**: an `UNPROVEN` record of the resend is
  `DISCARDED` and makes no `UNPROVEN` row
  (`retired_key_extra_entry_same_second_unproven`);
- **two keys in one second**: each reads its own retiring seq
  (`two_keys_hand_off_same_second`), and the key that hands on carries the
  next key's entry as at any earlier record
  (`successor_handoff_carries_next_key_entry_same_second`).

## Boundaries of the revision of 2026-10-16, filled in `corpus/gen.py`

The revision of 2026-10-16 made the chain the ledger and the audit a function
of the root and the anchor set with its basis; the corpus witnesses every new
line on both sides:

- **§8.1 the ledger**: an entry carried by a counted record of a lineage key
  (every clean case); the same bytes carried by two records (one entry, two
  anchors); an entry carried only by an `UNPROVEN` record, only by a `VOID`
  record, or only by a discarded record (no ledger entry; the bytes in that
  row, and a `SEQ_GAP` where a position is left empty); the same bytes
  carried by a `VOID` record and later by a counted one (a ledger entry from
  the counted record on); a counted record of a ledger entry's hash that
  carries nothing (one `ANCHORED` row carrying both records).
- **§8.1 the lineage** over every carried byte string whatever the record: a
  succession carried only by a stranger's record, only by a `VOID` record,
  only by an `UNPROVEN` record (the successor joins the lineage and its own
  records count; the position is a gap); a stranger's succession extending
  nothing; a key that joins through a succession that moved nothing.
- **§8.1 and §8.2 authority**: a `VOID`-only succession beside a counted
  twin to the same key (the twin moves the seat, the successor's entry
  grades nothing); a counted succession the retired key signed after its
  handover (a ledger entry, yet `AUTHORITY_MISMATCH` on it and on the
  entries of the key it named, the walk being certain); a succession to the
  root itself carried by no counted record (the root's later entry graded
  against the root's own authority, no finding beyond the gap).
- **§8.4 the carried record** at every ground and position: junk, the empty
  byte string, a stranger's entry (at its seq), a stranger's genesis (at 0),
  a `galeed/2` entry (`E_SPEC`, the migration of §12.3), a stranger-signed succession, a root-signed succession with a body
  fault, and one with a signature fault carried by the root and by a
  stranger alone;
  junk and the stranger's entry by two records; junk by an `UNPROVEN` and by a `VOID`
  record, the stranger's entries by a `VOID`, an `UNPROVEN` and a discarded
  record (no finding, the entries `EXCLUDED`).
- **§8.5 `MISSING`**: a counted record carrying nothing, by hash alone and
  beside a carrying record of the same hash; beside an `UNPROVEN`, a `VOID`
  and a discarded carrier of the same bytes (no ledger entry, the hash
  `MISSING`); `MISSING` rows with two transactions on one chain and on two
  chains.
- **§8.7 the label**: `UNPROVEN` over a record whose hash no counted record
  carries, with and without bytes of its own; not over a record whose hash
  is counted elsewhere; shadowed by no discarded counted record; a
  discarded `UNPROVEN` record moving nothing; before `GAPS` and after
  `BROKEN_CHAIN`.
- **§8.7 the rows**: `UNPROVEN` and `VOID` rows with and without `entry`,
  sorted by the four keys; `DISCARDED` rows with `entry`, ordered by block
  before tx and tx before hash; `EXCLUDED` over entries carried by their own
  signers and by records that did not count; `ANCHORED` rows naming a
  counted record that carried nothing beside one that carried the bytes;
  `UNKNOWN_TYPE` over a case variant of a known type.
- **§9.2 and HARNESS**: every member of the record, the transport spelling
  of `entry` at each of its edges (an odd digit count whose padded or
  truncated reading would hash to the record's `hash`, and a `null`, among
  them), a verdict in another case, two records agreeing on the four keys
  and differing in `entry`, the two legacy members of the earlier input
  shape ignored as members beyond the three; the misuse argument vectors of
  `check`, `canon` and `audit` (command names that are inherited property
  names of a common runtime's object type among them), and the `sign`
  surface at the domain's edges (`len(M)` at 99, 100, 999 and 1000) and the
  key's edges with its misuse exits.
- **§3.5 and §5.4**: the numeric runs `0-`, `1-2`, `1e`, `0e`, `1E+2`; `[1 -2]`
  beside `[1,-2]`; form feed, vertical tab, U+00A0 and U+0085 as whitespace;
  `kind` and `entryType` literals in another case; a signature whose `r`
  names no curve point while `r + n` does.
- **§9.4 the basis of one member**: a second member refused, `chains`
  absent refused, every ordering and uniqueness rule on both sides, class
  items at every edge, the reach test over one `chains` object.
- **§6.2 `quotes`**: present as an object (members untested), present as an
  array, a string, `null`.

### Chain-facing §9, in `scan-py/scenarios.py`, fixtures 1 to 76

- **§9.1 the form** recorded against the GaleedRegistry: one anchor with its
  entry (1); three in one transaction (2); a stranger declared and absent
  (3, 4); the registry's refusal of bytes that do not hash (5); an
  undeclared emitter (6); a derived topic (7); a contract caller's topic 1
  (13); a skewed encoding (14); one hash twice (20); no registries (21).
- **§9.3** recorded: a delegated sender (8), a cleared designator (9), the
  designator installed by the anchoring transaction itself (10), a node that
  will not serve the state (11); hand-written, block 0 (29).
- **§9.1 rogue logs from a declared registry**: two topics (15), the
  two-argument declaration's topic 0 (16), four topics (17), raw data (18),
  bytes that do not hash to the topic (19); hand-written, the five refusals
  of the data (35 to 39).
- **§9.2, §9.4** two chains (12); two adjacent windows with different
  senders (22); two logs of one hash in one transaction, one carrying the
  bytes (34); class items (31 to 33).
- **§9.1 the sender is the recovered signer**, hand-written and signed with
  test keys: a signature-less transaction (23), a legacy signature naming no
  chain (24), a type this scanner does not encode (25), and the positive
  side over the type 1, 3 and 4 signing payloads (26 to 28); a log inside a
  creation (30).
- **§9.4 basis forms**, eighteen fixtures with an empty `rpc` and the
  no-label fragment (40 to 57).
- **Round 1 of the review (58 to 75)**, hand-written: a receipt without a
  status and a status-0 log (58, 59); the hash at offsets 0 and 32 and a
  window not wholly within the calldata (60, 61); a returned log under
  another topic 0 (62); a lying `from` (63); the empty entry (64); one
  transaction hash in two blocks (65); `VOID` on the first boundary alone
  and `UNPROVEN` with one boundary unanswered (66, 67); the failing scans of
  §9.4, an `eth_getLogs` error, a null answer and a range past the head (68
  to 70), beside the undecodable envelope of 25; adjacent windows differing
  in registries (71); a class emitter whose code changes across blocks (72);
  a signed genesis carried into a ledger entry (73); envelopes naming another
  chain (74, 75); a node answer whose envelope fields do not hash to the
  transaction, failing before any field decides anything (76).

## Arms that remain, by class

**Harness misuse.** Argument counts, an unreadable file, an unknown command.
The harness contract fixes exit status 2 and no message for these; since the
revision the corpus carries them as `misuse` argument vectors and `sign`
cases predicting exit 2, which `compare.py` runs and `bcov.py`, measuring
over the corpus files alone, does not: seven arms in `gl1.py` (the argument
counts, the unreadable path and the command dispatch; its main guard is a
tracing artefact below), and in
`scan_replay.py` the fixture argument, the main guard and `try_run`, which
the scenario tooling calls.

**Arms the law makes unreachable.** The fork walk's `a.eid == b.eid` (§8.1
collapses byte-identical copies before the walk); the canonicalizer's
fall-through over a value outside the universe (no such value parses in);
the skeleton test's control-byte arm (§3.5 refuses the byte at test 2 before
any row reads it); the integer literal's empty-run arm in `glcanon.py`
(`int_literal`'s `if not run:`, which the number grammar never reaches with
an empty digit run); the recovery test `r >= P` (`r < N < P` after the range
test); the criterion's intermediate Jacobian arms, which are arithmetic and
no rule (§5.4's own point-at-infinity boundary is witnessed by the corpus),
and the RFC 6979 retry, which sits in §5.7's producer obligation outside the
surface §12.1 compares; the parser's unbounded loops' exhaustion arms (each
returns from inside); in the scanner's recovery, `r` or `s` out of range, an
`r` that is no curve abscissa, and a recovered point at infinity (a
transaction a chain included carries a signature a node validated), and the
Jacobian infinity arms again.

**Environment, in `scan_replay.py`.** A node answering outside the JSON-RPC
forms: a transaction, receipt, log or header that is not an object, a header
without `timestamp`, a null `eth_getCode` for an emitter's class read, a
node that cannot serve the code of an emitter a class item would admit (the
scan is loud, since an emitter it cannot classify is a log it cannot decide), a
recording whose `rpc` member is not an object, an exchange that is not an
object, an access or authorization list entry that is not an object, a typed transaction whose parity is neither 0 nor 1, and a hex field that
is not a string or is odd or non-hex; a transaction object whose signed
bytes do not hash to the hash it was asked for by is witnessed by fixture
76 and tested before any field of the envelope decides anything. The scanner raises a loud error and produces no fragment; §9.3 names
the one case the law decides (state that cannot be consulted is `UNPROVEN`,
fixture 11), and the rest is the scanner's own conduct toward a broken node.

**Filter-implied, in `scan_replay.py`.** A log whose address is not declared
under a basis without class items, whose topic 0 is not the form's, or
whose block lies outside the range: `eth_getLogs` is asked with the address
list or the sender topics, the topic 0 word and the range, so a conforming
node never returns such a log; the fixtures nonetheless witness a lying node
on each (62 for another topic 0, 59 for a status-0 transaction's log, 63 for
a `from` field). The form's data decoder has five refusing arms, each
witnessed by one of the fixtures 35 to 39; the arms a node cannot steer
guard against a lying node, and fixtures 4, 6, 7, 13 and 16 witness the same
sentences of §9.1 on the side a node can produce.

**Tracing artefacts.** Module-level loops that ran before the tracer was
installed (the scanner's Keccak fallback, which never runs where
`pycryptodome` is present), `__main__` guards, `__repr__`, and helpers no
command reaches (`jneg`, the JSON-equality arms of value kinds the recordings
never hold, the canonical writer's escape arms for characters no fixture
carries).

## The registry criterion, `bcov.py roster`

The same measure over `roster-py/` and `roster-corpus/` (seed 1, 660 cases):
509 arms, and the seven not both-sided are the `__main__` guard, the 16 MiB
bound on a file, and three helpers no command calls (`_int_spelling`,
`venue_key_ok`, which carries three of the seven, and `of_kind`). Every
branch point a command reaches is witnessed on both sides, the form cells
of §3.1 and §3b.1 and the logs file's member forms among them; the
boundaries no branch carries alone (§5.2's position and the greatest
position, §5.3's take-up, §5.6's rows) are named in the corpus README's §4
and §5 tables.

## The papers criterion, `bcov.py papers`

The same measure over `papers-py/` and `papers-corpus/` (seed 1, 692 cases
under the revised door): 509 arms, and the one arm not taken is `gpp.py`'s
`__main__` guard. The arms of 2026-10-23 (check 2 and the two pairings
against the caller's `current`, the door's `--current` option, and the
book's two arrays of the logs file read for form) are witnessed on both
sides by the `current_*`, `cur_*`, and `book_*` cases. Every branch point of the three files that an input can
steer is witnessed on both sides, the reading of §7a.3 among them (a
publication of another kind passed over; the candidate unread at
`galeed.roster/1` §2.1's steps 1 to 3 with `E_HASH`, `E_JSON` and `E_KIND`,
bytes of another kind under a wrong word, unreadable bytes and a kind that
is no string among them; unread under §7a.2 at every token the order can
carry; each `E_SERVICE` token passing through the reading with its index;
mismatched; and read; the logs file's every misuse form) and the door's
view over the ledger `galeed/1` §8.1 decides (a dispatch carried by a
counted record, by an `UNPROVEN` record, by a `VOID` record, by a stranger's
record, by a record that carries nothing; a succession carried by an
`UNPROVEN` record and by a stranger's; check 5's four cases and the disputed
dispatch). The generator's §11 census stands at zero unwitnessed literals,
the service tokens and the four reading verdicts included.

## Reading

With seed 1 of the corpus and the seventy-six fixtures, every branch point of
the three criterion programs that an input can steer is witnessed on both
sides. An implementation that agrees with the criteria on these corpora
agrees with them at every boundary either criterion draws with a branch of
its own, and the arms it may still take differently are the ones listed
above, none of which is a decision of the law. A boundary a criterion decides
without a branch is invisible to this measure, which is why the corpus also
carries hand-built cases for the ordering rules of §3.5, for the decision
order of §4.3 step 8 and §5.4, and for the sort keys of §8.7, and why the
boundary list of §12.1 is read against the law and never read off the arcs.
Regenerate with another seed and rerun `bcov.py` to repeat the measure; the
generator's own count of closed vocabularies stays at zero unwitnessed.
