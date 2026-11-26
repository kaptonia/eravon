# papers corpus

The generated corpus of `galeed.papers/1` (docs/galeed-papers-v1.md), built by
`gen.py --seed N` from the law and HARNESS-PAPERS.md alone, borrowing the
`galeed/1` generator's canonical form, signing path, entry builder and label
predictor (`../corpus/gen.py`, which needs `pycryptodome`) and `keccak256`
from `../impl-py/glcrypto.py`. Seed 1 is committed; `../compare-papers.py` runs a candidate named by `GPP_CANDIDATE`
and the criterion `../papers-py/gpp.py` over `manifest.json`, counts every
divergence and every disagreement with the generator's prediction, prints
the first sixty, and exits 1 where it found any. Two
files of this directory begin with `--` (`--now`, `--note-dashes.json`),
argument-vector witnesses; a script that globs here copies them as `./*` or
after `--`.

- `docs/`: one document per file, exercised by `note-check`, `pass-check`,
  `report-check`, `refusal-check`, `service-check`.
- `sign/`: preimages for `sign`, one answering case per domain plus the
  misuse shapes.
- `pair/`: note and report pairs for `pair`, and note, pass and refusal
  triples for `pair-refusal`.
- `qr/`: documents for `qr-encode` and payload texts for `qr-decode`.
- `door/`: a note, passes, audit inputs (HARNESS.md shape) and bench files
  for `door`.
- `service/`: registry logs sets (HARNESS-ROSTER.md's `<logs.json>` form,
  each publication carrying its `document` bytes and `word`, beside the
  statuses and the succession book's declarations and accessions) for
  `service-of` and `serving`.
Every `door`, `pair`, and `pair-refusal` case that is no misuse carries the
caller's `current` (§1): the note's verifier unless the case names another
key or `unresolved`, the one case without `--current` (`current_omitted`)
witnessing the omission.
A case whose `predicted` is `null` is harness misuse and the expected
answer is exit code 2 with no output.

`manifest.json` lists every case as `{cmd, args, path, note, predicted}`,
or as `{argv, path, note, predicted}` where `argv` is the whole argument
vector after the executable name (an empty `argv` runs `gpp` alone);
paths are relative to this directory. The prediction is the generator's
own reading and never authoritative: where two implementations agree with
each other and disagree with it, the prediction is wrong.

## Closed boundaries (law §13.1's precondition)

The generator carries the list: `gen.py` prints the census of every §11
literal (each token, verdict, state and reason must be answered by some case)
and of every state of the door's six checks, and refuses to write a corpus in
which any is unwitnessed. The seed moves key material and digests and never
the case list, so the list is checked once against the corpus's shape.

The door's boundaries of the 2026-10-16 revision, each witnessed by a case
of `door/` named by its slug: the dispatch carried by a counted record of
the root, of the successor, of a stranger (discarded), only by an `UNPROVEN`
record, only by a `VOID` record, and by a `VOID` beside a counted one
(`dispatch_carried*`, `dispatch_by_*`); a counted record carrying nothing
beside an `UNPROVEN` or a stranger's carrier of the same bytes
(`hash_only_counted_beside_*`); a succession that fails §4.3 by its
signature or its body (`succession_bad_*`) and a valid succession signed by
a key outside the lineage (`stranger_succession`, the lineage rule of
`galeed/1` §7.4), the records in reverse order (`records_reverse_order`) and
two successions listed after the dispatch (`two_successions_reverse_order`);
the covering basis on three of its four clauses, the fourth (naming no
chain) having no reachable failing side since an empty `chains` admits no
record and check 5's first clause answers before covering is read
(`not_dispatched_not_covering`, `*_from_block_*`, `one_chain_two_windows_*`,
`one_chain_second_window_senders_short`, `two_chains_*`,
`class_item_only_registry`); a `PASS` under label `UNPROVEN`
(`dispatch_carried_label_unproven`) and under `GAPS`
(`stray_anchor_no_bytes_dispatch_carried`, `dispatch_after_gap`); the
dispatch of the key that handed off at a position after its handing off,
carried by that key in the second of the handing off, `DISCARDED` and no
ledger entry, after a gap and without one (`retired_key_after_gap`,
`retired_key_dispatch_certain`), the same dispatch re-carried by the
successor in that second, a ledger entry and disputed, alone and beside a
clean one (`disputed_alone_successor_carrier`, `disputed_beside_clean`), the
key that handed off anchoring at its
retirement's own second the position it held
(`retired_key_late_anchor_of_held_position`), and the same key's conflicting
entry at that position at that second, an equivocation
(`retired_key_late_conflicting_entry_at_held_position`); a return to an
earlier key (`return_to_earlier_key`); the basis omitting the sitting key
while the carried succession admits it (`senders_omit_sitting_key`), and the
same basis with the dispatch in hand, where a dispatch is a witness and the
door is `GREEN` (`senders_omit_sitting_key_dispatched`); and the harness's
first positional and option values beginning with `--`
(`note_path_begins_with_dashes`, `option_value_begins_with_dashes`,
`bare_double_dash`).

The boundaries of the 2026-10-23 revision: check 2 against `current`
(`current_unresolved`, `current_omitted`, `current_moved_key`,
`current_moved_key_old_pass`, `current_names_note_verifier_moved_pass`), its
order after the pass's own faults (`current_note_mismatch_first`,
`current_bad_pass_first`), its absence where no pass is shown
(`current_no_pass`), check 6 reading the note's verifier itself
(`current_moved_key_bench_reads_note`,
`current_moved_key_bench_holds_only_new_key`), the echo
(`current_bad_note`), §10.5's offline door (`current_offline`), and the
option's spellings (`current_misuse_*`); the two pairings against `current`
(`pair/` cases named `cur_*`: unresolved, the moved key, the key it left,
the note's verifier never compared, and the order before the protocol and
after the note and pass words); and the retired key of §12
(`ret_stray_after_retirement` beside `ret_stray_at_retirement`,
`ret_retired_dispatch_after_retirement` and
`ret_retired_dispatch_at_retirement`, `ret_successor_dispatch_after_retirement`,
`ret_retired_key_carries_successor_dispatch_late`) and the closed ledger
(`ret_dispatch_before_migration`, `ret_entry_after_migration_same_second`,
`ret_entry_after_migration_later`); the retirement read across chains
(`xr_dispatch_before_retirement`, `xr_dispatch_at_retirement`,
`xr_dispatch_after_retirement`, `xr_stranger_early_carrier_retires_key`,
`xr_successor_recarries_held_dispatch`, `xr_genesis_anchored_after_retirement`)
and the lineage admitting a succession only by a record at which its judge
has not retired (`xr_late_fork_admits_nobody` beside
`xr_fork_at_retirement_second`, `xr_succession_after_migration_late` and
`xr_succession_after_migration_same_second`); the papers lineage's own
retirement, read from the carried bytes of a handing off whatever the
record's sender and verdict (`xr_lineage_hashonly_early_record_of_succession`
and `xr_lineage_hashonly_early_record_of_migration` beside
`xr_lineage_migration_bytes_early`,
`xr_lineage_key_admitted_only_by_UNPROVEN_record`,
`xr_lineage_key_admitted_only_by_VOID_record`,
`xr_lineage_VOID_migration_retires_before_succession`,
`xr_lineage_UNPROVEN_migration_retires_before_succession`); a return to the
root, the root retired for good at its first handing off
(`xr_return_to_root_dispatch_by_root` beside
`xr_return_to_root_dispatch_by_successor`); the key's own uncounted early
carrier on another chain (`xr_own_UNPROVEN_carrier_on_another_chain_early`,
`xr_own_VOID_carrier_on_another_chain_early`); the walked authority outside
the lineage (`xr_walked_outsider_own_dispatch`,
`xr_walked_lineage_key_no_dispatch`); the ends of the clock
(`xr_retirement_at_zero_*`, `xr_retirement_at_max_dispatch_at_max`,
`xr_retirement_below_max_dispatch_at_max`); `current_as_note_path` (the
first token after `door` is the note path); `sign` preimages failing
`galeed/1` §3.5 that a plain JSON reader accepts (`misuse_preimage_*`); and
every pair of the four arrays of a logs set sharing a position
(`misuse_shared_position`, `misuse_shared_position_*`,
`misuse_status_shares_position`, `misuse_book_shares_position`,
`misuse_accession_shares_publication`,
`misuse_declaration_shares_accession`).
