# scan-py: where the text left a choice

Independent Python implementation of the `galeed/1` chain-facing scan
(`scan_replay.py`), written from `docs/galeed-v1.md` sections 3, 8, 9 and
`galeed-conformance/HARNESS.md` alone. `galeed/1` names one anchor form, the
log of section 9.1 whose data carries the entry, and no transaction outside
it, so the scan is one query surface and one reading. Every place those
texts admitted more than one reading is recorded below: section, the
readings, the choice taken.

## Transport spelling (fixture format, not the law)

1. **Hex quantities.** Readings: the JSON-RPC `QUANTITY` convention (minimal
   lowercase, `0x0` for zero) or a zero-padded spelling. Choice: minimal
   lowercase, `hex(n)` in Python, for `fromBlock`, `toBlock`, the
   `eth_getCode` block, and the `eth_getBlockByNumber` block.
2. **`address` array order.** Readings: the `registries` array as the basis
   declares it, or sorted/deduplicated. Choice: as declared, in order, since
   the recording compares params as JSON values and arrays are ordered.
3. **Exchange matching.** Readings: exchanges consumed once, or reusable.
   Choice: reusable; the first exchange whose `method` and `params` match
   answers, and answers again for a repeated call. Params equality is deep
   and key-order-insensitive, with `true`/`false` never equal to an integer.
4. **Recorded errors.** Readings: a recorded `error` is a transport failure,
   or a definite negative answer. Choice: transport failure. On `eth_getCode`
   for a sender's boundaries that is the section 9.3 `UNPROVEN` verdict; on
   every other method it is a failure of the scan (9.4), exit 2, like a
   missing recording (fixture 68).

## Section 9.4 (scan basis and completeness)

5. **The head.** 9.4 makes a range that reaches past the head of the chain it
   read a failure of the scan, so each chains object's `toBlock` is tested
   against `eth_blockNumber` before that object's logs are asked for
   (fixture 70), whatever its `registries` list; a recording that does not
   carry the head cannot be scanned. The head is read once per chain.
5a. **Basis canonicity.** The basis arrives as a parsed JSON member of the
   fixture, so the section 3.5 roundtrip test over its bytes cannot be run.
   Readings: re-serialize and test, or test the value universe through the
   forms. Choice: the forms. Every member is an `int` in [0, 2^53 − 1] (a
   JSON boolean is not an int) or a lowercase `hex20`/`hex32`, which is what a
   canonical basis of this shape can carry, and the ordering and uniqueness
   rules of 9.4 are tested on the parsed value: `chains` ascending by
   `(chainId, fromBlock)`, two objects of one chain neither overlapping nor
   adjacent with equal `registries` and `senders`; `registries` and `senders`
   bytewise ascending without repeats, `registries` with its addresses first and
   its class items after. Nothing else is testable here.
6. **Absent `basis`.** Readings: harness misuse (exit 2), or not a
   `galeed/1` basis. Choice: not a basis, so `{"ok":false,"reason":"NO_LABEL"}`
   and exit 0. A missing or unreadable *file* is still exit 2.
7. **Empty `registries` on a declared chain.** Readings: still issue
   `eth_getLogs` for every chunk with `address: []`, or issue none. Choice:
   issue none. No log emitted by a declared registry can exist when none is
   declared, so the scan surface is empty; skipping also means an absent
   recording for a degenerate filter is not a loud failure. The anchor set is
   the same under either reading; the head is still read (item 5).
7a. **A chain that does not answer `eth_getLogs` with an array.** `null`, or
   any other non-array, is a chain that did not answer: a failure of the scan
   (9.4) and never an empty result (fixture 69).
8. **Empty `senders`.** Choice: the chunks are still queried. `senders` filters
   the logs the scan admits (9.4), never the query surface, which 9.1 defines
   by emitter and topic 0.
9. **Chunk stepping.** `fromBlock .. min(fromBlock+1999, toBlock)`, then the
   next chunk begins at the previous chunk's end plus one, so the chunks are
   disjoint and cover the inclusive range exactly.
10. **Re-checking what the filter already asked for.** A recording answers
    here, and no node enforces the filter, so the log's `address` is re-tested
    against `registries` and its `blockNumber` against `[fromBlock, toBlock]`
    before the log is read. A log outside either is silently not an anchor.
11. **One record per `(chainId, blockNumber, tx, hash)`.** Readings: reject
    a second, or collapse. Choice: collapse, as 9.4 states; the first record
    formed stands, and it takes the `entry` from whichever log of the key
    carried the bytes (fixture 34). A chain that included one transaction
    hash in two blocks would yield two records, one per block, as 9.2 says.

## Section 9.1 (the form)

12. **Sender.** 9.1 defines the sender as the address recovered from the
    transaction's own signature, so the node's `from` field (on the
    transaction or the receipt) is never read. The transaction object is
    re-encoded (legacy with EIP-155, and types 1 to 4 with their access,
    blob and authorization lists), the signed bytes must hash to the hash
    the transaction was asked for by, and the signer is recovered over the
    signing payload. A node whose object does not hash to the requested hash
    is answering about other bytes: loud failure, exit 2, decided before any
    field of the envelope is read (an altered `chainId`, `v` or `yParity`
    fails there and is never read as another chain's envelope; fixture 76). The recovered
    sender serves the `senders` test, the topic 1 test, the 9.3 boundaries,
    and the record; the receipt is consulted for `status` alone.
12a. **No sender, and no verdict on the sender.** A transaction with `r` and
    `s` both zero (a deposit or other system transaction; a zero `r` is no
    ECDSA signature, so nothing was validated by recovery) and a legacy
    transaction whose `v` is below 35 (27 and 28 name no chain; anything else
    is no signature) has no sender in 9.1's grammar: silently not an anchor,
    exactly as a failed form test; nothing is recovered and no code state is
    consulted (fixtures 23, 24). A transaction that carries a signature inside
    an envelope of a type this scanner cannot decode is one the verifier
    cannot tell the sender of: 9.1 and 9.4 make that a failure of the scan,
    exit 2 (fixture 25).
12c. **Another chain's envelope.** A legacy signature whose EIP-155 chain, or
    a typed envelope whose `chainId`, names a chain other than the one the log
    sits on is not an anchor (9.1), silently (fixtures 74, 75).
12b. **Creation transactions.** A transaction whose `to` is null is refused
    before any recovery: there is no calldata to run the containment test
    against.
12d. **Order of the tests, and the code read last.** For a log of the form
    the scan tests the emitter's declaration (by address, or by the presence
    of a class item), the block range, the transaction's recipient, the
    recovered sender against `senders` and topic 1, the receipt's status, and
    the containment, and only then, for an emitter admitted by class alone,
    reads the emitter's code; a log that fails any earlier test is not an
    anchor whatever the code, and a node that cannot serve that code fails the
    scan only for a log every other test admitted.
12e. **The receipt's status.** A receipt without a `status` field carries no
    status 1, so the transaction is not an anchor and the scan does not fail
    (fixture 58); a status other than 1 is the same silence (fixture 59).
13. **`blockNumber` of the record.** Readings: the log's, the receipt's, or
    the transaction's. Choice: the log's (the log is what 9.1 tests).
14. **Topic 0.** One word, `keccak256("Anchored(address,bytes32,bytes)")`; a
    log under the two-argument declaration's word, which an earlier registry
    emitted, has another topic 0 and is not an anchor (fixture 16). The rogue
    contract that emits it (`rogue/src/Rogue.sol`, `oldForm`) is the one
    place in this tree that spells the retired declaration, as the negative
    witness that recording needs.
15. **Case.** Every hex string the chain answers with is lowercased before
    comparison and before emission; section 1 spells `hex20` and `hex32` in
    lowercase, and a node may answer with a checksummed address.
16. **`removed` logs.** The law knows no reorg flag. Choice: never consulted.
17. **The data.** Read once the log is an anchor: the ABI encoding of one
    `bytes` value (offset word 32, length word, the bytes, zero padding to a
    multiple of 32) whose sha256 is topic 2 gives the record its `entry`;
    anything else, `0x` and an absent `data` member included, is an anchor
    that carries nothing. The decoder's five refusals (a length that is no
    encoding, an offset other than 32, a length word the data does not match,
    non-zero padding, a mismatched hash) are fixtures 35 to 39.
18. **Calldata offsets.** The admissible offsets of a 32-byte window are the
    union of `{0, 32, 64, ...}` and `{4, 36, 68, ...}`, each restricted to
    `o + 32 <= len(calldata)`.
19. **A log whose transaction the node does not return.** A log the
    recording produced whose transaction or receipt it does not carry is a
    gap in the recording: exit 2, never a guess (item 21).
20. **A pending transaction** has no log in any block and never reaches the
    scan.
21. **Silent versus loud.** Every failed form test of 9.1 makes the thing not
    an anchor, silently. Loud, exit 2, are the failures of the scan 9.4
    names and the gaps of a recording: a chain that answers `eth_getLogs`
    with an error or with no array, a range past the head, a signed envelope
    the scan cannot decode, a log the recording produced whose transaction,
    receipt or header it does not carry, and an emitter a class item would
    admit whose code the node will not serve. A fixture of a failing scan
    records `expected` as null.

## Section 9.3 (codeless verdict)

22. **No short-circuit.** Readings: stop at the first boundary that shows
    code, or consult both. Choice: consult both. `VOID` requires that "both
    states were consulted"; where the other boundary cannot be consulted the
    verdict is `UNPROVEN`, so `UNPROVEN` pre-empts `VOID` and the second call
    must be made even after code is seen at the first.
23. **"Cannot be consulted."** Readings: only a recorded error, or also an
    absent recording and a `null` result. Choice: all three yield `UNPROVEN`
    (fixtures 11, 67); the boundary with code beside one without is `VOID`
    (fixture 66).
24. **"Has code."** Any non-empty byte string in the `eth_getCode` answer is
    code, an EIP-7702 delegation designator (23 bytes) included; `0x` is no
    code, and an address absent from the state trie answers `0x`.
25. **Block 0.** Only `eth_getCode(sender, "0x0")` is issued; the pre-genesis
    state is not consulted and is not a failure to consult.

## Section 9.2 (the record)

26. **`blockTimestamp` for `UNPROVEN` and `VOID` records.** Readings: fetch
    the header for every record, or only for `counted` ones. Choice: every
    record; 9.2 states one record shape and 8.7 carries all three verdicts.
    A block header the recording cannot answer is therefore a loud failure,
    since 9.3 names `UNPROVEN` for the two account states and for nothing else.
27. **Header call.** `eth_getBlockByNumber(hexq(number), false)`, cached per
    `(chainId, blockNumber)`; `timestamp` read as a hex quantity.

## Output

28. **Fragment shape.** `{"anchors":[...],"basis":<basis>}` in
    the law's canonical form (3.4): members bytewise sorted, integers plain,
    no whitespace, no trailing newline (HARNESS.md). Anchors sorted by
    `(chainId, blockNumber, tx, hash)` with the ints numeric and the two hex
    strings bytewise.
29. **The basis** is re-emitted canonically from the parsed value, unchanged
    in content.

## Undecided by the text, and why it does not bite here

- The law never spells a JSON-RPC method, a filter shape, or a chunk size:
  section 9 states what an anchor set must *contain*, and every access path is
  outside it. The fixture format fixes the access path for this comparison, so
  the choices in items 1 to 4 belong to the harness. The law states none of
  them, and two
  implementations that agree on them agree on bytes.
- Section 9.4 decides what an earlier reading left open: "a verifier that did
  not obtain every log the completeness rule enumerates has not scanned this
  basis", so a log whose transaction, receipt or header the chain will not
  produce is an unanswerable scan, exit 2, and never an absent anchor; section
  9.3's `UNPROVEN` is about the two account states alone.
