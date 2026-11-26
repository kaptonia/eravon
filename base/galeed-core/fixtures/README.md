# Chain-facing fixture corpus

Each `<nn>_<name>.json` is one scan's whole conversation with a node, together
with the canonical fragment that conversation produced. The sibling
`<nn>_<name>.note` says which sentence of `docs/galeed-v1.md` the scenario is
about.

The corpus is closed: nothing in it needs a chain. A reader who holds these
files can reproduce every fragment offline, byte for byte, with no node, no
network, and no anvil.

## Fixture shape

```
{
  "basis":      the section 9.4 object the scan ran under, as written
  "rpc":        { "<chainId>": [ {method, params, result|error}, ... ] }
  "expected":   the canonical fragment string: {"anchors":[...],"basis":{...}},
                or null where the recorded scan fails (section 9.4: a chain
                that did not answer, a range past the head, an envelope the
                scan cannot decode), which a replay must reproduce as exit 2
}
```

Every fixture that reads a chain records that chain's `eth_blockNumber`: a
declared range that reaches past the head is a failure of the scan (9.4).

Every `eth_getLogs` exchange a fixture records asks for the one topic 0 word
of galeed/1 §9.1 (`Anchored(address,bytes32,bytes)`), by address where the
basis lists addresses and by sender topic without an address filter where it
lists a class item. A record carries its `entry` where the log's data is the
ABI encoding of bytes whose sha256 is topic 2, and carries nothing otherwise.

The scanner reads no `from` field: section 9.1 defines the sender as the
address recovered from the transaction's own signature, so every recorded
transaction object carries a real signature, the scanner re-encodes it, checks
that the signed bytes hash to the transaction's hash, and recovers the signer.
The hand-written recordings are signed with anvil's published test keys.

An exchange the recording does not hold is an error on replay and never a
guess, so a fixture with a hole fails loudly instead of quietly answering
something else. Errors the node returned are recorded as errors, which is how
fixture 11 replays a node that will not serve historical state.

## The scenarios

| Fixture | Section | What it holds |
| --- | --- | --- |
| `01_carried_anchor` | 9.1, 9.2, 9.3 | One anchor through a declared registry from a declared sender, the data carrying the entry. One record, verdict `counted`, with `entry`. |
| `02_anchor_many_three` | 9.1, 9.4 | `anchorCarriedMany` with three entries: one log per element, three records sharing one transaction, each with its entry. |
| `03_foreign_sender_declared` | 9.4 | A sender outside the audited lineage whose address the basis names. Recorded with its entry; section 8.1 is what later discards it. |
| `04_foreign_sender_absent` | 9.4 | The same chain under a basis whose `senders` omits that address. No record, and the basis carries that silence. |
| `05_reverted_mismatch` | 9.1 | The registry refuses bytes that do not hash to their word. Status 0, no log, no record. |
| `06_undeclared_emitter` | 9.1 | Two identical registries, one declared. The undeclared contract's well-formed log is never read. |
| `07_derived_topic_not_in_calldata` | 9.1 | A declared contract emits the form with a topic 2 derived from its argument. Topic 2 is nowhere in the calldata, so it is no anchor. |
| `08_delegation_void` | 9.3, 5.7 | The sender installed an EIP-7702 delegation designator before anchoring. The record carries the entry and its verdict is `VOID`. |
| `09_delegation_revoked_counted` | 9.3, 5.7 | Installed and cleared (an authorization naming the zero address) in earlier blocks. Both boundaries codeless, verdict `counted`. |
| `10_delegation_in_anchor_block` | 9.3, 5.7 | Codeless at the block before; the anchoring transaction itself carries the sender's authorization and installs the designator. `VOID` on the second boundary alone: an anchor that counts for nothing. |
| `11_state_not_served_unproven` | 9.3 | The endpoint refuses `eth_getCode`. Verdict `UNPROVEN`, the record carrying its entry, and the recording carries the node's error verbatim; under a root equal to the sender the audit is labelled `UNPROVEN`. |
| `12_two_chains_same_hash` | 9.2, 9.4 | One entry anchored on chain 31337 and on chain 7777. Two records, one exchange log per chainId; the basis lists the chains in ascending order as 9.4 requires. |
| `13_contract_caller_topic1` | 9.1 | A contract calls the declared registry, so topic 1 is the contract. The hash does sit in the calldata; the log is still no anchor. |
| `14_anchor_many_skewed_offset` | 9.1 | A non-canonical `anchorCarriedMany` encoding places the hash at calldata offset 105. The registry emits with the entry; the law declines to read the hash there. |
| `15_rogue_two_topics` | 9.1 | A declared registry emits a log with the form's topic 0 and only two topics, the hash in the data. Not an anchor. |
| `16_rogue_old_form_three_topics` | 9.1 | Three topics and no data under the two-argument declaration's topic 0, the form an earlier registry emitted. Not the form of this grammar: no anchor. |
| `17_rogue_four_topics` | 9.1 | Four topics and no data from a declared registry. Not an anchor. |
| `18_rogue_raw_data` | 9.1, 8.5 | Three well-formed topics whose data is the raw entry bytes and not the ABI encoding. An anchor carrying nothing: one record without `entry`. |
| `19_rogue_wrong_bytes` | 9.1, 8.5 | Three well-formed topics whose data encodes bytes whose sha256 is not topic 2. An anchor carrying nothing. |
| `20_anchor_many_same_hash_twice` | 9.4 | `anchorCarriedMany` with one entry twice: two logs, one record per `(chainId, blockNumber, tx, hash)`, carrying the entry. |
| `21_chain_without_registries` | 9.1, 9.4 | A chains object whose `registries` is empty. No `eth_getLogs` exchange, no record. |
| `22_two_windows_adjacent_senders_differ` | 9.4 | Hand-written: two `chains` objects of one chain over blocks 0..2 and 3..5 with different `senders`. Each window's senders bound that window alone: two records, and the first sender's anchor in the second window is outside the basis. |
| `23_unsigned_tx_no_sender` | 9.1 | Hand-written: a transaction with no signature (a system type, `r` = `s` = 0, the node asserting a `from`). Its log is no anchor. |
| `24_legacy_unprotected_no_anchor` | 9.1 | Hand-written: a legacy transaction signed without EIP-155 (`v` = 27 or 28). Its signature names no chain, so it is not an anchor under this grammar. |
| `25_unknown_tx_type_scan_fails` | 9.1, 9.4 | Hand-written: a transaction of type 0x10 carrying nonzero `r` and `s`. The verifier cannot tell its sender, so it has not scanned the basis: `expected` null, the replay fails. |
| `26_type1_access_list` | 9.1 | Hand-written: an EIP-2930 transaction with an access list. The sender is recovered over the type 1 payload; one record with its entry, `counted`. |
| `27_type3_blob` | 9.1 | Hand-written: an EIP-4844 transaction with a blob versioned hash. Recovered over the type 3 payload; one record, `counted`. |
| `28_type4_authorization` | 9.1, 9.3 | Hand-written: an EIP-7702 transaction carrying another account's authorization. Recovered over the type 4 payload; the codeless answers at both boundaries, never the list, give the verdict `counted`. |
| `29_anchor_in_block_zero` | 9.3 | Hand-written: an anchor whose legacy EIP-155 transaction sits in block 0, where the codeless test has one boundary. Verdict `counted`. |
| `30_creation_log_not_anchor` | 9.1 | Hand-written: a log of the form emitted during a contract creation, naming the sender, a word the init code carries, and the entry. Init code is not calldata; no record. |
| `31_class_item_admits_emitter` | 9.1, 9.4 | Hand-written: a basis whose `registries` holds one class item; the scan queries the form by sender topic without an address filter, reads the emitter's code at the end of the log's block, and admits it by code hash. One record with `entry`. |
| `32_class_item_code_differs` | 9.1, 9.4 | Hand-written: the same log under a class item naming another code hash. No record. |
| `33_class_item_and_address_mixed` | 9.4 | Hand-written: an address and a class item together; the registry's log admitted by address without a code read, the venue's by code. Two records. |
| `34_two_logs_one_hash_one_record` | 9.4 | Hand-written: one transaction emitting two logs of one hash, one carrying nothing and one carrying the bytes. One record, carrying the entry. |
| `35` to `39`, `_data_*` | 9.1, 8.5 | Hand-written: the form's data with bytes that do not hash to the topic, raw bytes, an offset word other than 32, a length word the data does not match, and non-zero padding. Each is an anchor that carries nothing. |
| `40` to `57`, `_basis_*` | 9.4 | Eighteen bases that fail their form, one clause each: not an object, a member beyond `chains`, `chains` absent, member sets, arities, integer and hex forms, `fromBlock` above `toBlock`, two equal `chains` objects, `registries` unsorted, `senders` repeated, `chains` unordered, overlapping and adjacent-equal windows, a class item's form and its place. Each yields `{"ok":false,"reason":"NO_LABEL"}` with no exchange. |
| `58_receipt_without_status` | 9.1 | Hand-written: a receipt carrying no `status` field carries no status 1; no anchor, and the scan does not fail. |
| `59_status_zero_with_log` | 9.1 | Hand-written: the node returns the form's log from a status-0 transaction; no anchor. |
| `60_hash_at_offsets_zero_and_32` | 9.1 | Hand-written: the hash at calldata offset 0 (calldata of the hash alone) and at offset 32 (one zero word, then the hash); both windows admissible, two records. |
| `61_hash_not_wholly_within_calldata` | 9.1 | Hand-written: the calldata ends inside the window where the hash would lie; no anchor. |
| `62_returned_log_other_topic0` | 9.1 | Hand-written: a returned log under another topic 0 beside the form's log; only the second is a record. |
| `63_lying_from_field` | 9.1 | Hand-written: the node's `from` names another address; the record names the recovered sender. |
| `64_empty_entry` | 9.1 | Hand-written: the data encodes the empty byte string and topic 2 is sha256 of nothing; the record carries the entry `0x`. |
| `65_one_tx_hash_in_two_blocks` | 9.2, 9.4 | Hand-written: the form's log for one transaction hash returned in two blocks; two records, one per block. |
| `66_void_on_first_boundary_alone` | 9.3 | Hand-written: code at the end of the previous block, none at the log's block; `VOID`. |
| `67_unproven_one_boundary_unanswered` | 9.3 | Hand-written: the first boundary codeless, the second unanswered; `UNPROVEN`. |
| `68_getlogs_error_scan_fails` | 9.4 | Hand-written: `eth_getLogs` answers an error; `expected` null, the replay fails. |
| `69_getlogs_null_scan_fails` | 9.4 | Hand-written: `eth_getLogs` answers null, no array of logs; `expected` null, the replay fails. |
| `70_range_past_head_scan_fails` | 9.4 | Hand-written: the declared range reaches block 3 while `eth_blockNumber` answers 2; `expected` null, the replay fails. |
| `71_two_windows_adjacent_registries_differ` | 9.4 | Hand-written: two adjacent windows of one chain whose `registries` differ and whose `senders` agree; one record each. |
| `72_class_emitter_code_differs_across_blocks` | 9.1, 9.4 | Hand-written: a class emitter whose code hash matches at block 2 and not at block 3; the block-2 log is admitted, the block-3 log is not. |
| `73_signed_genesis_carried` | 9.1, 8.1 | Hand-written: a genesis entry signed by the sender's key, carried by the form; the fragment fed to `gl1 audit` with the sender as root yields one ledger entry, `COMPLETE`. |
| `74_legacy_signed_for_other_chain` | 9.1 | Hand-written: a legacy EIP-155 transaction whose signature names chain 1 inside a block of chain 31337; not an anchor. |
| `75_typed_signed_for_other_chain` | 9.1 | Hand-written: a type-2 transaction whose envelope names chain 1 inside a block of chain 31337; not an anchor. |
| `76_altered_envelope_scan_fails` | 9.1, 9.4 | Hand-written: the node's transaction object has an altered `chainId`, so its fields no longer hash to the transaction's hash; the node answers about other bytes, the scan fails before any field decides anything; `expected` null. |

Scenarios that need no recording, the eighteen basis forms, are fixtures
all the same: their `rpc` member is empty and their fragment is the no-label
answer. Thirty-seven recordings (22 to 39 and 58 to 76) are ones no node
produced: they are written by hand, signed with test keys, and their notes
say so; five of them (25, 68, 69, 70, 76) record a scan that fails. The proof
kits of section 9.6 are outside this corpus: they are read against trusted
block hashes, which a recording cannot supply.

## Regenerating

The corpus is produced by `galeed-conformance/scan-py/scenarios.py`, which
starts two anvils on free ports with `--hardfork prague` (chain ids 31337 and
7777), deploys the registry of `galeed-core/contracts` and the rogue
contracts of `galeed-conformance/scan-py/rogue`, drives the transactions with
`cast`, and records each scan through `scan-py/record.py` over
`scan_replay.py`'s own scanner. Every recorded scenario asserts the facts
its fragment must show before the fixture is written; the fragment itself is
the scanner's output, and the written fixture is replayed once more with no
node before it lands. `anvil`,
`forge` and `cast` must be on `PATH`. `scenarios.py hand` writes the
hand-written and basis fixtures alone and needs no node.

Block numbers, block hashes, timestamps, transaction hashes, and deployed
addresses all move when a scenario is re-recorded. A routine run writes only
the fixtures whose files are absent; set `GL_REGEN_FIXTURES=1` to rewrite the
recorded ones too. The hand-written fixtures are deterministic.
