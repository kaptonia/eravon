# evidence corpus

The generated corpus of `courtevidence/1` (docs/courtevidence-v1.md), built
by `gen.py --seed N` from the law and HARNESS-EVIDENCE.md, borrowing the
`galeed/1` generator's signing path (`../corpus/gen.py`, which needs
`pycryptodome`) to build ledger entries. Seed 1 is committed; `../compare-evidence.py` runs a candidate
named by `GEV_CANDIDATE` and the criterion `../evidence-py/gev.py` over
`manifest.json`, counts every divergence and every disagreement with the
generator's prediction, prints the first sixty, and exits 1 where it found
any. The file `--` of this directory is an
argument-vector witness; a script that globs here copies it as `./*` or
after `--`.

- `manifests/`: manifests and near-manifests for `check`.
- `files/`: byte strings for `digest`.
- `audits/`: `galeed/1` audit inputs (HARNESS.md's shape: `root`, `anchors`,
  `basis`) for `pick-recipient`; the root is the judge key's address, drawn
  from the seed, and the entries travel as the bytes counted records carry.
- `recipient-check` cases carry their argument in `args` and no file.

`manifest.json` lists every case as `{cmd, args, path, note, predicted}`,
or as `{argv, path, note, predicted}` where `argv` is the whole argument
vector after the executable name; paths are relative to this directory; a
case whose `predicted` is `null` is harness misuse and the expected answer
is exit code 2 with no output. The seed draws the keys, the digests and the
recipients and never the case list; the two equal-`seq` fork cases search
over `effective` so that each `entry_id` order appears under every seed.
The prediction is the generator's own reading, written by hand from the
text, and never authoritative.

## Closed boundaries (law §8.1's precondition)

### §2 the manifest (`check`)

| boundary | fails | passes |
|---|---|---|
| `canonical(b)`, its eight tokens | `canon_utf8`, `canon_json`, `canon_empty`, `canon_number`, `canon_negative_bytes`, `canon_depth`, `canon_dup_key`, `canon_key_charset`, `canon_value_charset_name`, `canon_value_charset_tab`, `canon_value_charset_del`, `subject_extra_non_prose_control`, `canon_pretty`, `canon_trailing_newline`, `canon_unsorted` | every accepted case; `ok_subject_prose_subtree` (an `_md` member inside `subjectRef`) |
| root an object | `root_array`, `root_string` | |
| required members, then closed | `missing_party`, `closed`, `closed_note` | `ok_minimal` |
| `spec` | `spec_v2`, `spec_int` | |
| `subjectRef`: object, `kind` token, known kinds' tables, `note_md` of `hash` | `subject_not_object`, `subject_kind_empty`, `subject_kind_space`, `subject_evm_missing_id`, `subject_hash_upper`, `subject_hash_note_int`, `subject_hash_note_null` | `ok_subject_evm`, `ok_subject_evm_extra`, `ok_subject_unknown_kind`, `ok_subject_hash_note` |
| `party` | `party_upper`, `party_short`, `party_int` | |
| `files` an array, possibly empty | `files_object`, `files_null` | `ok_empty_files` |
| file items: object, missing, closed, the four members, the index | `file_string`, `file_missing_bytes_index_1`, `file_closed`, `file_closed_sixth_member`, `file_name_empty`, `file_name_int`, `file_sha_short`, `file_sha_upper`, `file_bytes_string`, `file_bytes_bool`, `file_mediatype_empty`, `file_mediatype_null`, `file_order_index_2` | `ok_name_space_bytes_zero`, `ok_max_bytes_tilde`, `ok_repeated_items`, `ok_bytes_zero_any_digest` |
| the optional `zikaron` member: object, exactly four members, each form, the index | `zikaron_null`, `zikaron_array`, `zikaron_missing_tx`, `zikaron_extra_member`, `zikaron_root_short`, `zikaron_root_upper`, `zikaron_chain_string`, `zikaron_chain_bool`, `zikaron_tx_hex20`, `zikaron_entry_odd`, `zikaron_entry_upper`, `zikaron_entry_no_prefix`, `zikaron_entry_int`, `zikaron_index_1` | `ok_zikaron_full`, `ok_zikaron_entry_empty`, `ok_zikaron_some_items`, `ok_zikaron_chain_zero` |
| `note_md` | `note_int`, `note_null` | `ok_full`, `ok_note_empty` |
| §2.3 order, every adjacent pair at the root and inside an item | `missing_and_extra`, `order_closed_before_spec`, `spec_and_party`, `order_subject_before_party`, `order_party_before_files`, `note_after_files`, `file_missing_and_closed`, `file_order_closed_before_name`, `file_order_closed_before_zikaron`, `file_order_name_before_sha`, `file_order_sha_before_bytes`, `file_order_bytes_before_mediatype`, `file_order_mediatype_before_zikaron`, `file_order_zikaron_before_note`, `file_order_first_faulting_element` | |

### `digest`

`text`, `empty` (the empty byte string), `binary`.

### §3.2 the recipient (`recipient-check`)

Passes: a well formed recipient, the all-zero key, the all-ones key.
Fails: a bad checksum, uppercase, mixed case, sixty-one and sixty-three
characters, a wrong human-readable part, no separator, a character outside
the alphabet, non-zero padding bits, the empty string, the prefix alone, a
data part whose checksum fails, the Bech32m spelling of a key, a scalar
value above U+007E.

### §1 and §2 additions of the 2026-10-16 revision

| boundary | fails | passes |
|---|---|---|
| the `evm` table's members, each form and each absence (`galeed/1` §6.3 via §2.3); `kind` compared bytewise | `subject_evm_chain_string`, `subject_evm_contract_hex32`, `subject_evm_codehash_hex20`, `subject_evm_id_int`, `subject_evm_missing_chain`, `subject_evm_missing_contract`, `subject_evm_missing_codehash`, `subject_evm_missing_id` | `ok_subject_evm`, `subject_kind_evm_upper` |
| an extra `_md` member of a closed table opens a prose subtree and the closure refuses it under its own token | `root_extra_md_control`, `file_extra_md_control`, `zikaron_extra_md_control` | `ok_subject_prose_subtree` (the one open table, a `subjectRef`, admits it) |
| HARNESS: `-h` as a recipient argument, a file named `--` | | `recipient-check -h` (`E_RECIPIENT`), `check --` |

### §4 the pick (`pick-recipient`)

| boundary | witnesses |
|---|---|
| §4.1 `unread`: the file the reader refuses, a root that is not an object, a root outside hex20, a member absent, an entry outside the transport spelling, a basis outside `galeed/1` §9.4, a record the basis does not reach | `unread_unreadable`, `unread_root_array`, `unread_root_upper`, `unread_basis_absent`, `unread_entry_0X`, `unread_basis_member`, `unread_record_unreached` |
| §4.1 bindings: lineage, successor key (carried by the root in the second it handed off and by itself; carried by the root after its retirement, `galeed/1` §7.4, no binding), the retired key's own binding after its retirement (no binding), the successor's key as root, excluded judge (carried by itself and by the root), malformed bytes (by a stranger, by the root), duplicates collapse, order at equal seq (both orders), a binding carried only by a VOID record (no hole, the earlier binding picked), a binding carried only by an UNPROVEN record and a binding whose counted record carries nothing (each a hole, rule 3), an UNPROVEN record of a hash the ledger holds (no hole), a stranger's UNPROVEN record (discarded, no hole), a recipient carrying a quotation mark and a backslash, a binding at seq 2^53 - 1 | `one`, `lineage_successor`, `lineage_successor_own_anchor`, `successor_binding_carried_by_retired_root`, `retired_key_binding_after_retirement`, `root_is_current_key`, `excluded_other_judge`, `excluded_other_judge_carried_by_root`, `malformed_ignored`, `malformed_carried_by_root_broken`, `duplicates_collapse`, `fork`, `fork_order_small_effective_first`, `fork_order_large_effective_first`, `candidates_order`, `binding_carried_by_void_record`, `binding_carried_by_unproven_record`, `binding_hash_only_gap`, `unproven_record_hash_also_counted_no_hole`, `unproven_record_of_stranger_discarded_no_hole`, `recipient_with_quote_and_backslash`, `binding_at_max_seq` |
| `recipientForm` true and false; false at the top above a valid earlier binding (no fall-through) | `one`, `recipient_form_false`, `recipient_form_false_at_top_above_valid` |
| rule 1 `none` | `none`, `none_empty`, `empty_anchor_set_none`, `basis_covers_no_chain_none` |
| rule 2 `fork`: two bindings, a binding and a non-binding, shared-prev equivocations whose position equals `top` | `fork`, `fork_with_annotation`, `shared_prev_same_seq_at_top`, `shared_prev_position_is_smaller_seq`, `shared_prev_below_top_after_bindings` |
| `top` among the bindings only; an equivocation above `top`, a shared-prev pair straddling `top`, a shared-prev pair whose upper entry is the top binding | `equivocation_above_top`, `shared_prev_pair_straddling_top`, `shared_prev_upper_entry_is_top_binding` |
| §4.1 with `galeed/1` §7.4: a key retires at the smallest block time, on any chain, of a record carrying its succession or migration, whatever the sender and the verdict (the root's VOID and UNPROVEN records, a stranger's counted record); a record of its own second carrying no bytes, another key's entry or what the key signed at a position before its handing off counts, and one carrying what it signed at a position after its handing off is discarded, as is every record a second later (`retired_key_binding_certain_broken`, `retired_key_binding_after_hole_gap`: `none`; `root_binding_carried_by_successor_same_second`: the same binding on the successor's record, a ledger entry); a migration closes the ledger and new-law bytes after it are discarded, and a migration first carried before the last binding leaves that binding out for good | `skew_succession_void_on_another_chain_earlier`, `skew_succession_counted_on_another_chain_earlier`, `skew_succession_strangers_record_on_another_chain_earlier`, `skew_succession_unproven_on_another_chain_earlier`, `migration_first_carried_void_on_another_chain_earlier`, `skew_successor_reanchors_binding`, `migration_closed_ledger_one`, `migration_binding_above_same_second`, `migration_binding_above_later`, `migration_new_law_genesis_same_second`, `migration_new_law_genesis_later`, `retired_root_unproven_commitment_after`, `retired_root_unproven_commitment_same_second`, `retired_root_hash_only_commitment_after`, `retired_root_hash_only_commitment_same_second`, `root_binding_anchored_after_its_retirement`, `root_binding_reanchored_by_successor` |
| rule 3 `gap`: a SEQ_GAP anywhere (below, above `top`), a MISSING hash the scan never read (a counted record carrying nothing, at any position), an UNPROVEN row whose hash is no ledger entry; before `broken`; a shared-prev pair not at `top` beside a hole | `gap`, `genesis_not_carried`, `candidates_order`, `equivocation_below_top_with_gap`, `hole_above_top`, `binding_hash_only_gap`, `missing_hash_above_top_is_hole`, `binding_carried_by_unproven_record`, `shared_prev_pair_then_gap` |
| rule 3 no hole: bytes the scan read and found to be no entry (`MALFORMED` carried by the root, `EXCLUDED` carried by the root) leave no hole and fall to rule 4; an UNPROVEN row whose hash is a MALFORMED or an EXCLUDED row is no hole | `malformed_carried_by_root_broken`, `ledger_binding_beside_root_carried_stranger_entry`, `excluded_other_judge_carried_by_root`, `unproven_junk_no_hole`, `unproven_stranger_entry_no_hole` |
| rule 3 hole: a MISSING or UNPROVEN hash whose bytes ride only in records that did not count or that the trim of `galeed/1` §8.1 discarded (a VOID carrier, a stranger's carrier, the retired root's carrier, beside the same carrier at the root's retirement second, where the binding is an entry); a middle binding carried only by a VOID record (a SEQ_GAP); a VOID-carried later binding beside an UNPROVEN no-bytes record of its hash | `missing_bytes_only_in_void_carrier_hole`, `missing_bytes_only_in_discarded_carrier_hole`, `missing_bytes_only_in_retired_root_record_after`, `missing_bytes_only_in_retired_root_record_same_second`, `unproven_nobytes_bytes_only_in_retired_root_record_after`, `unproven_nobytes_bytes_only_in_retired_root_record_same_second`, `middle_binding_void_only_gap`, `void_bytes_beside_unproven_nobytes_gap` |
| rule 3 no hole beside a MISSING row: the bytes under a counted no-bytes record's hash are MALFORMED or EXCLUDED in a discarded or VOID carrier, so GAPS and `one` stand together, and carried at the retirement's second by a counted record of the root they are rule 4's hard finding; a later binding whose bytes only VOID or discarded records carry is no act and no hole | `missing_malformed_bytes_in_discarded_carrier_no_hole`, `missing_malformed_bytes_in_void_carrier_no_hole`, `missing_foreign_entry_in_discarded_carrier_no_hole`, `missing_malformed_bytes_in_retired_root_record_after`, `missing_malformed_bytes_in_retired_root_record_same_second`, `missing_foreign_entry_in_retired_root_record_after`, `missing_foreign_entry_in_retired_root_record_same_second`, `void_and_discarded_carriers_of_a_later_binding` |
| no hole: a later binding relayed only by a stranger's discarded record; a VOID record carrying nothing; the same VOID-only later binding below a counted annotation is limb 1's hole (`gap`) | `top_binding_relayed_by_stranger_only`, `void_record_carrying_nothing_no_hole`, `void_only_later_binding_below_counted_annotation_gap` |
| the successor as root where its ledger holds no binding: `none` (nothing signed, COMPLETE; an annotation alone, GAPS, rule 1 answering before rule 3; its binding relayed only by the root's key, every record discarded) | `successor_root_signed_nothing`, `successor_root_signed_only_annotation`, `successor_root_binding_relayed_by_root_only` |
| rule 4 `broken`: PREV_MISMATCH, an equivocation below `top`, a certain AUTHORITY_MISMATCH, ROOT_MISMATCH alone, CARRIED_MALFORMED | `broken_prev`, `equivocation_below_top`, `root_binding_carried_by_successor_same_second`, `root_mismatch_alone_broken`, `malformed_carried_by_root_broken`, `ledger_binding_beside_root_carried_stranger_entry` |
| rule 2 before rule 3 | `fork_over_gap` |
| rule 5 `one`, `effective` read by nothing, extra body members | `one`, `one_unordered_records`, `extra_body_members` |

### HARNESS-EVIDENCE

Transport digits of either case (`upper_digits_ok`), an escaped entry
(`escaped_entry`), an unreadable audit path, argument counts for every
command, no command, an unknown command; every refusal of the audit input
itself is the law's `unread` and never misuse.
