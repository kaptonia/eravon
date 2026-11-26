# GALEED wire grammar v1 (`galeed/1`)

The judge's ledger grammar: `galeed/1` remembers a judge's rulings and the
covenant they were ruled under.
This document is the wire law, and it is self-sufficient: every predicate a
verifier needs is stated here in full. Nothing in any construction document,
design document, essay, or implementation adds to or subtracts from it, and
the earlier texts that carried the name `galeed/1` (an essay and two
textbooks) are superseded by this one; no ledger was ever published under
them.

The ledger is neutral toward everything that gives a ruling effect. It knows
no escrow, no bond, no deposit, no claim: a ruling names its subject through
an open reference (§6.3), and the grammar tests the one anchor form of §9
and nothing about the contract that produced it. Neutrality is toward
meaning; the form is narrow by design. A venue that wants a ruling and its
anchor to land in one transaction meets §9.1's conditions on its log and on
the sending key's own calldata, and this grammar's on the sender: a lineage
key of the seat that has not retired (§7.4) sends the transaction (§7.4,
§8.1), that transaction's data carries the ruling's `entry_id` at a word
boundary, and the venue emits the form itself. The fourth condition is not
the venue's to meet: a log is read only where a reader has admitted the
emitter in a `chains` object's `registries`, by address or by code hash, and
the sending key in the same object's `senders`, over a range that holds the
block (§9.4), so a venue publishes the address it emits from, a seat lists
it among its declared venues (§6.2), and a reader who names it in no
`chains` object's `registries`, or the sending key in no such object's
`senders`, reports no anchor and says so through the basis it published. A
ruling of any other venue, and of every venue that never heard of this
grammar, is anchored from the own transaction of a lineage key that has not
retired (§7.4): through a registry that emits the form from its own code, a
log a reader reaches by naming that registry in `registries` and the sending
key in the same `chains` object's `senders`. Anchoring is in every case
relative to a declared scan and never a fact a report asserts without one.
The covenant, dispatch notes, rosters, and evidence manifests are documents
of family laws that read this ledger; this grammar tests the forms it names
and nothing they mean.

**Status.** The law is the set of total predicates in this text. Every rule
is a machine decision defined over all inputs; there are no example-based
prohibitions and no blacklists. A conforming implementation agrees with the
frozen core on every input from the moment §12.5 names a release digest;
where two implementations disagree after that naming, at least one is wrong,
and the core decides which. Before the naming there is no core, this text is
the whole of the law, and a disagreement is settled by reading it and, where
the reading is contested, by amending it. After the naming this text is the
core's textbook: it says the same thing in prose, and where the two could
ever be read apart, the core binds. Evolution goes through `galeed/2`.

**Rule classes.** Three classes of predicate appear, and every rule below is
marked with its class. A passage that states no predicate (a producer
obligation, a definition of a record, a description of what a report
carries) carries no mark. §5.1 through §5.3, §5.6, and §5.7 are such passages:
the first four define the preimage, the message, the digest, and the domain,
§5.7 binds producers alone, and §5.4 and §5.5 are the predicates the first
four feed. Where a test of §4.3 or of a row of
§6 repeats a §3 predicate exactly, that half of the test has no reachable
failing side and contributes no boundary to §12.1: `token`'s upper edge at
U+007E, on `entryType` at §4.3 step 5 and on every `token` row of §6, is one case, every scalar value above U+007E having failed at step 1, as
`E_VALUE_CHARSET` or, spelled by a surrogate pair, as `E_JSON`, while its
lower edge at U+0020 is reachable on both sides (§1).

- **[E]** entry predicates: decided from one byte string alone, offline.
- **[C]** chain predicates: decided offline from the audit inputs §8
  declares (the anchor set with its §9.3 verdicts and the basis it was
  scanned under), and never by consulting a blockchain.
- **[X]** chain-facing predicates: decided against a blockchain's state or
  history. Where an [X] predicate can fail for want of the chain, it names
  the verdict a verifier returns; §9.1 and §9.2 define the anchor form and
  the anchor record and carry no verdict of their own.

Verification with no chain in reach means class [E] in full and class [C]
over an anchor set a verifier was handed, a set of proof kits (§9.6) among
the ways of handing one; class [X] is the work of producing that input
from a chain. Every
list in an audit report is decided from the two inputs of §8 and nothing
else, so the freeze of §12 covers [E] and [C] in full, the report's
`UNPROVEN` and `VOID` lists and its label included, and never covers how a
verifier obtains the inputs from a chain: two conforming implementations may
disagree about what a chain shows, and §9.4's completeness rule and §9.6's
proof kits, never §12.2, are what settle such a disagreement.

## 1. Notation

- **byte string**: a finite sequence of octets.
- **`hex20`**: a string of exactly 42 characters: `0x` then 40 lowercase
  hexadecimal digits `0-9a-f`. **`hex32`**: `0x` then 64 such digits (66
  characters). **`hex65`**: `0x` then 130 such digits (132 characters).
  Uppercase digits, an uppercase or absent `0x` prefix, or any other length
  fail the form. `hex20(a)`, `hex32(h)`, and `hex65(g)` write a 20-byte
  address, a 32-byte digest, and a 65-byte signature as `0x` followed by the
  bytes in lowercase hexadecimal, most significant nibble of each byte
  first. Where a rule says a field equals an address, a digest, or an
  `entry_id`, it means the field is byte-equal to that value's spelling
  under these functions.
- **bytewise order**: two octet sequences are compared position by position
  from the first; at the first position where they differ, the sequence with
  the smaller octet orders first; where one is a proper prefix of the other,
  the shorter orders first. Where a rule compares an address, a digest, or
  an `entry_id` bytewise, the octet sequence compared is that value's
  `hex20` or `hex32` spelling; the two orders agree, lowercase hexadecimal
  being order-preserving over bytes, and the spelling is what a report
  carries.
- **UTF-8**: the encoding of RFC 3629 exactly: every scalar value in its
  shortest form, no byte sequence encoding a value in U+D800–U+DFFF, and no
  byte sequence encoding a value above U+10FFFF. Any other byte sequence is
  not valid UTF-8.
- **`token`**: a non-empty string every one of whose scalar values lies in
  U+0021 through U+007E (printable ASCII without the space). Tokens compare
  bytewise.
- **`skeleton string`**: a string every one of whose scalar values lies in
  U+0020 through U+007E.
- **`prose string`**: a string, whose scalar values are unrestricted (§3.3).
  Where a row of §6 names one of these three string forms, it tests at §4.3
  step 12 that the value is a string; a `prose string` row tests nothing
  further, a `skeleton string` row tests that it is non-empty where the form
  says so, and a `token` row tests that it is non-empty and carries no
  U+0020, the rest of the token charset having been decided by §3.5 at step
  1, which admits U+0020 in a string value where a token does not (§6).
- **`int`**: a JSON integer in the universe of §3.1 (0 to 2^53 − 1).
- **`bool`**: the literal `true` or the literal `false` (§3.1) and no other
  value; `1`, `"true"`, and `null` are not `bool`.
- **`sha256`**: FIPS 180-4 SHA-256 over bytes. **`keccak256`**: the
  Keccak-256 function as used by Ethereum (the pre-FIPS Keccak padding),
  over bytes.
- **`secp256k1`**: the elliptic curve of that name; `n` denotes its group
  order,
  `0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141`.
- **Unix seconds**: an `int` counting seconds since 1970-01-01T00:00:00Z.
  Every time-valued field of an entry (`effective` in §6.6 and §6.7) is Unix
  seconds and is data for readers. No predicate compares one to a clock, to
  a block, or to a field of any other entry. The `blockTimestamp` of an
  anchor record (§9.2) is not a field of an entry: it is Unix seconds read
  from a block header, it enters no finding except through the retirement of
  §7.4, which decides which records count and which successions extend the
  lineage, and it appears in a report only
  in the `ANCHORED` rows of §8.7 item 5, from which §9.5 computes a reader's
  existence bound, and in the `DISCARDED` rows of item 11, which carry every
  member §9.2 names of a record the trim of §8.1 removed.

## 2. Identity

**[E] 2.1 Identity is total.** For every byte string `b`, `entry_id(b) =
sha256(b)`. Identity and validity are separate predicates: a byte string has
an `entry_id` whether or not it is an entry. This is what lets an anchor
commit to bytes before anyone has read them, and lets an audit name an
anchored byte string that fails to verify.

**[E] 2.2 Publication is unique.** An entry's published bytes are its
canonical bytes (§3) and nothing else: no byte order mark, no trailing
newline, and no whitespace outside a string value. A re-encoding of a valid entry either
reproduces its bytes exactly, in which case it is the same byte string and
the same publication, or differs from them, in which case it fails §3.5 and is not an entry at
all; a re-encoding that differs only in member order, or in an escape §3.5
admits, fails at test 6, one that spells a scalar value by a surrogate pair
fails at test 2 as `E_JSON` (§3.3), and one that differs in a way test 2's
grammar refuses fails there.

## 3. Canonical form

**[E]** `galeed/1` uses one serialization of one closed value universe,
and one acceptance test. The profile is RFC 8785 (JSON Canonicalization
Scheme) with the restrictions stated here; where this section and the RFC
differ, this section governs.

**[E] 3.1 Value universe.** A value is exactly one of: the literal `null`; the
literal `true`; the literal `false`; an integer; a string; an array of values;
an object whose members map keys to values. Nothing else is a value. In
particular no value is a number carrying a minus sign, a fraction, or an
exponent: floating point is unrepresentable. A **value position of numeric
shape** is a position at which the grammar admits a value and whose first
byte is `-`, `+`, `.`, or a decimal digit; the position immediately after an
array's `[` is such a position, the `]` the grammar also admits there taking
nothing from it, and a member-name position is never one. **The bytes there** are the maximal run of bytes beginning
at that first byte and drawn from `0`–`9`, `+`, `-`, `.`, `e`, and `E`. A
value position of numeric shape fails `E_NUMBER` when the bytes there are
not an integer under §3.2, whether they fail RFC 8259's number grammar or
satisfy it outside the universe; every other failure to parse fails
`E_JSON`.

**[E] 3.2 Integers.** An integer is written in decimal with no sign, no leading
zeros (the integer zero is written `0`), and lies in [0, 2^53 − 1]. A number
outside that range fails acceptance.

**[E] 3.3 Strings.** Strings are sequences of Unicode scalar values (no
surrogate code points), encoded as UTF-8. A `\u` escape whose four digits
name a code point in U+D800–U+DFFF fails `E_JSON`, whether or not a second
such escape follows it; §3.4 defines canonical bytes only over scalar
values, and no accepted byte string reaches it carrying a surrogate. Two
charsets apply:

- **Keys** are skeleton strings (every scalar value in U+0020 through
  U+007E) and non-empty. A key carrying any other scalar value, or none,
  fails `E_KEY_CHARSET`.
- **String values** are skeleton strings, except inside a prose subtree. A member
  whose decoded key is the three bytes `_md`, or ends in them, opens a prose
  subtree: that member's own value, and every string value at any depth within it (through
  arrays and nested objects), is a prose string and may carry any scalar
  value, including U+0000 through U+001F and U+007F. Keys inside a prose
  subtree remain keys under the key rule. Outside every prose subtree, a
  string value carrying a scalar value outside U+0020 through U+007E fails
  `E_VALUE_CHARSET`.
  A prose subtree opens at whatever depth its member sits, the root object
  included; no key of the envelope (§4.1) ends in `_md`, so no entry ever
  opens one at the root, and a byte string that is not an entry and does
  open one there is decided by §3.5 like any other. U+FEFF has no special
  standing as a mark: outside a prose subtree the key and value charsets
  refuse it like any scalar value above U+007E, inside a prose subtree it is
  carried and emitted raw like any other scalar value, and at every position outside a string, the first three bytes of `b` among
  them, `EF BB BF` is a sequence RFC 8259's grammar admits nowhere, so `b`
  fails test 2 where test 1 has not already named `E_UTF8` (§3.5).

Every test in this section is applied to decoded strings: the key charset,
the value charset, and the `_md` suffix are each decided after escape
decoding, never on the input's raw bytes. A raw byte in 0x00 through 0x1F
inside a string is not admitted by RFC 8259's `unescaped` production and is
a grammar failure at test 2 of §3.5 whose triggering byte is that raw byte,
named `E_JSON` where `b` is valid UTF-8 and no earlier triggering byte
names another fault, whatever charset would apply to the string it appears
in: the permission for prose strings to carry U+0000 through
U+001F is a permission over decoded scalar values and never over the input's
raw bytes, and §3.4 item 3 fixes their only canonical spelling.

**[E] 3.4 Canonical bytes.** The canonical bytes of a value are produced by:

1. `null`, `true`, `false` as those four, four, and five ASCII bytes.
2. An integer as its decimal digits per §3.2.
3. A string as `"`, its scalar values in order, `"`, where each scalar value
   is emitted as: `"` → `\"`; `\` → `\\`; U+0008 → `\b`; U+0009 → `\t`;
   U+000A → `\n`; U+000C → `\f`; U+000D → `\r`; any other value in
   U+0000–U+001F → `\u00` followed by two lowercase hexadecimal digits; every
   other scalar value (U+0020 and above, U+007F and the C1 range included) as
   its raw UTF-8 bytes, never escaped.
4. An array as `[`, its elements' canonical bytes in order joined by `,`, `]`.
5. An object as `{`, its members as `key-bytes:value-bytes` joined by `,`,
   `}`, where key-bytes are the canonical bytes of the key as a string, and
   members are ordered by the bytewise order of their decoded UTF-8 keys.
   Test 4 of §3.5 restricts every key to U+0020 through U+007E and runs
   before test 6, so no value reaches this item with a key on which that
   order differs from the RFC's UTF-16 code-unit order; a future law that
   widens the key charset must restate this item, the two orders differing
   on keys carrying a scalar value above U+FFFF.
6. No byte lies between the pieces produced by items 1 through 5: no
   whitespace is emitted as structure. A whitespace scalar value inside a
   string is part of that string and is emitted by item 3 like any other
   scalar value. The output is UTF-8.

**[E] 3.5 Acceptance is roundtrip identity.** A byte string `b` is accepted
as canonical iff it passes the following tests, decided in this order, the
first failure naming the fault:

1. `b` is valid UTF-8 (§1) over its whole length, whatever the parse of
   test 2 would have reached: `E_UTF8`.
2. `b` matches RFC 8259's `JSON-text` production: `E_JSON`; within the
   parse, a container (array or object) opening at nesting depth 129, the
   root container counted as depth 1, fails `E_DEPTH` at the moment it
   opens and before any later byte is read; a value position of numeric
   shape that is not an integer under §3.1 and §3.2 fails `E_NUMBER`; a
   surrogate escape fails `E_JSON` (§3.3). Within this test the fault named
   is the one whose triggering byte comes earliest in `b`: the triggering
   byte of `E_DEPTH` is the `[` or `{` of the container opening at depth
   129, the triggering byte of `E_NUMBER` is the first byte of the value
   position of numeric shape, the triggering byte of a surrogate escape is
   its `\`, and the triggering byte of a grammar failure is the first byte
   no continuation of the `JSON-text` production admits, or the position one
   past the end of `b` when `b` is a proper prefix of some JSON text. The
   fault whose triggering byte comes earliest is the one named, and a fault whose triggering byte
   the parse never reaches is never named; one left-to-right parse that
   stops at its first fault realises this rule. Where two faults share a
   triggering byte, the fault named is the one earlier in the order
   `E_DEPTH`, `E_NUMBER`, surrogate escape, grammar failure; in particular a
   value position whose first byte is `+` or `.` names `E_NUMBER` and never
   `E_JSON`, though RFC 8259 admits neither byte there. That is the one
   shape a tie takes: a bracket the grammar refuses opens no container, so
   `E_DEPTH` shares no triggering byte with a grammar failure, and an escape
   the grammar admits is no grammar failure, so neither does a surrogate
   escape. So `0x10` at a value position fails `E_JSON` (the numeric run is
   `0`, an integer, and `x` is the fault) while `01` fails `E_NUMBER`; `0-`
   and `1-2` are runs that are not integers and fail `E_NUMBER` at their
   first byte; in `[1 -2]` the `-` follows a complete value at a position
   where the grammar admits no value, so it fails `E_JSON`, while `[1,-2]`
   fails `E_NUMBER`; and a byte string carrying 128 nested containers passes
   this test while one carrying 129 fails it. A
   scalar root has depth 0.
3. Every object at every depth has distinct keys after escape decoding:
   `E_DUP_KEY`.
4. Every key satisfies §3.3: `E_KEY_CHARSET`.
5. Every string value satisfies §3.3: `E_VALUE_CHARSET`.
6. The canonical bytes of the parsed value, computed by §3.4, equal `b`
   byte for byte: `E_NOT_CANONICAL`.

Test 3 ranges over every object of `b`'s parse and over the member sequence
each object's bytes write, duplicates included; tests 4 and 5 range over
every key and every string value of the whole parsed value, at every depth.
Each of the three is decided in full before the next begins, so a value that both repeats a key and
carries a key outside the charset fails `E_DUP_KEY`; unlike test 2, position
within the value never selects between two of them.

RFC 8259's `JSON-text` production admits insignificant whitespace before and
after the value, so a byte string carrying a trailing newline or a leading space is one JSON
text and, where no earlier test names a fault, fails at test 6 as
`E_NOT_CANONICAL`. A byte order mark is not whitespace under that
production, so a byte string beginning `EF BB BF`, and one carrying those
bytes at any position outside a string, fails test 2 as `E_JSON` where no
earlier test names a fault. Every variant spelling of a
value this universe admits (member order, escape choice) fails at test 6,
except that a spelling carrying a surrogate escape is named `E_JSON` by
test 2, a spelling whose escape decodes to a scalar value a charset refuses
is named by test 4 or test 5, and a spelling of a number outside §3.2's
form, a fraction, an exponent, a leading zero, or a sign among them, is named `E_NUMBER` by test 2, and a spelling of a value nested past 128 containers is named `E_DEPTH` by test 2, and a spelling RFC 8259's grammar does not
admit at all, a raw byte in 0x00 through 0x1F inside a string among them,
is named `E_JSON` by test 2; a byte string that spells no value of this
universe, an unknown literal such as `NaN`, `Infinity`, or `undefined`
among them, fails at test 2 as `E_JSON`, except that `-Infinity` opens a
value position of numeric shape and fails as `E_NUMBER`. None of them needs
an individual prohibition.

## 4. Envelope

**[E] 4.1 Seven members, closed.** An entry is a canonical object with
exactly these seven members and no others:

| key | form | rule |
|---|---|---|
| `spec` | string | byte-equal to `galeed/1`; a `spec` that is not a string fails the same test |
| `entryType` | token | open enumeration (§6.8) |
| `judge` | hex20 | the seat: the signer of this entry (§5.5) |
| `seq` | int | position in the chain (§7.1) |
| `prev` | `null` or hex32 | `null` iff `seq` is 0 (§4.3 step 8); what a non-null value links to is chain law (§7.2) |
| `body` | object | typed per `entryType` (§6); an object for every type, known or unknown |
| `sig` | hex65 | the EIP-191 signature (§5) |

Keys are unique (§3.5), so the member set is a set and is decided in two
tests: if any of `spec`, `entryType`, `judge`, `seq`, `prev`, `body`, `sig`
is absent, the entry fails `E_ENVELOPE_MISSING`; otherwise, if any other key
is present, it fails `E_ENVELOPE_CLOSED`. Member count alone decides
neither, since an object of seven members can both lack one of the seven and
carry an eighth key. Both tests are decided before the signature is
examined. The chain rules of §7 are not entry predicates: a byte string
whose `prev` names no entry of the verifier's inputs is an entry all the same, and
§8.2 records the mismatch when both sides are in hand.

**[E] 4.2 Genesis placement.** `entryType` is `genesis` iff `seq` is 0. A
`genesis` entry at any other position, or a non-`genesis` entry at position 0,
fails `E_GENESIS_PLACE`. One key opens one chain, and this rule is where the
grammar says so (§7.5).

**[E] 4.3 Order of decision.** A verifier decides the tests below in this
order and reports the first failure's token. The order is part of the law, so
that every conforming verifier names the same fault for the same bytes.

1. §3.5 acceptance (its tokens).
2. The root is an object: `E_ENVELOPE`.
3. Member set is exactly the seven keys: `E_ENVELOPE_MISSING`,
   `E_ENVELOPE_CLOSED`.
4. `spec` is a string byte-equal to `galeed/1`: `E_SPEC` (one token covers a
   `spec` of another form, since §10 gives `spec` no form token of its own).
5. `entryType` is a token: `E_ENTRYTYPE`.
6. `judge` is hex20: `E_JUDGE`.
7. `seq` is an int: `E_SEQ`.
8. `prev` is `null` or hex32: `E_PREV`; and `null` iff `seq` is 0:
   `E_PREV_SEQ`.
9. `body` is an object: `E_BODY`.
10. `sig` is hex65: `E_SIG_FORM`.
11. §4.2 genesis placement: `E_GENESIS_PLACE`.
12. §6 body table for the entry's type: `E_BODY_FIELD`.
13. §5 signature: `E_SIG_V`, `E_SIG_RANGE`, `E_SIG_HIGH_S`, `E_SIG_RECOVER`,
    `E_SIG_SIGNER`, decided in the bullet order of §5.4 followed by §5.5.

Where a step names more than one token, its tests are decided in the order
the tokens are written in that step. A byte string that passes all thirteen
is an entry. Nothing else is.

## 5. Signing

**5.1 Pre-signature bytes.** Let `E` be the seven-member envelope. `B6`
is the canonical bytes (§3.4) of the six-member object obtained from `E` by
removing the top-level member whose key is `sig`, and nothing else; a member
named `sig` at any depth inside `body` is untouched. `B6` is not an entry; it
exists only as a preimage. `presig = sha256(B6)`.

**5.2 Message.** `M = D || 0x0A || hex32(presig)`, where `D` is the
domain literal (§5.6), `0x0A` is one line feed byte, and `hex32` is as §1
defines it, so `M` ends in `0x` and 64 lowercase hexadecimal digits. For
`D = galeed/1`, `M` is exactly 75 bytes.

**5.3 Digest.** `digest = keccak256(0x19 || P || decimal(len(M)) || M)`,
where `P` is the 24 bytes `Ethereum Signed Message:` followed by one line
feed byte `0x0A`, so `P` is 25 bytes and `0x19 || P` is the 26-byte EIP-191
version-0x45 prefix (the `E` of `Ethereum` is the version byte 0x45), and
`decimal(len(M))` is the byte length of `M` written in ASCII decimal with no
leading zeros (`75` for entries under `galeed/1`; a family law's literal of
another length gives another decimal, which the signer computes from `M`
itself). `decimal(len(M)) || M` is injective over all messages: the width of
`decimal(l)` is non-decreasing in `l`, so a shorter message yields a
strictly shorter concatenation, and two concatenations of equal length
therefore carry messages of equal length, equal decimal prefixes, and equal
bytes.

**[E] 5.4 Signature form.** `sig` decodes to 65 bytes `r || s || v` with `r`
and `s` as 32-byte big-endian integers and `v` one byte. The closed
boundaries, each tested by every verifier:

- `v` ∈ {27, 28}: `E_SIG_V`.
- 1 ≤ `r` ≤ n − 1 and 1 ≤ `s` ≤ n − 1: `E_SIG_RANGE`.
- `s` ≤ (n − 1) / 2, that is `s` ≤
  `0x7fffffffffffffffffffffffffffffff5d576e7357a4501ddfe92f46681b20a0`:
  `E_SIG_HIGH_S`. Low-s is required so that the malleability twin
  `(r, n − s, 55 − v)`, which flips `v` between 27 and 28 and which any
  reader can compute from a published signature without holding the key, is
  not a second valid encoding of the same authorization: nobody can manufacture a second `entry_id` from bytes
  the judge published. A signer who varies the nonce can still produce a
  second valid signature over one `B6`, which is why §5.7 obliges RFC 6979;
  a verifier holding both copies records equivocation under §8.4, so the
  obligation is enforced by the judge's own exposure.
- Public key recovery from `(digest, r, s, i)`, `i = v − 27`, succeeds:
  `E_SIG_RECOVER`. Let `z` be `digest` read as a 256-bit big-endian integer
  reduced modulo `n`, and let `R` be the point of secp256k1 whose
  x-coordinate is `r` and whose y-coordinate is even when `i` is 0 and odd
  when `i` is 1, the group order `n` lying below the curve's field prime so
  that every `r` the range test admits is already a field element and no
  reduction precedes the curve equation. If no such point exists, recovery
  fails. Otherwise
  `P = r^(−1)(sR − zG)`, where `r^(−1)` is the multiplicative inverse of
  `r` modulo `n`, all scalars are taken modulo `n`, and the point arithmetic
  is on secp256k1; recovery fails if `P` is the point at infinity. Because
  this grammar admits only `v` ∈ {27, 28} and defines `R` as the point whose
  x-coordinate is exactly `r`, the candidate x-coordinate `r + n` that a
  general recovery would also try is never formed, and no recovery id above
  1 exists here.

**[E] 5.5 Signer equals judge.** The recovered public key `P` yields an
address `A = last 20 bytes of keccak256(X || Y)`, where `X` and `Y` are
`P`'s affine coordinates each written as exactly 32 bytes big-endian,
left-padded with zero bytes, so `X || Y` is the 64-byte uncompressed
encoding of `P` without the 0x04 prefix. `A`, written as
hex20, must be byte-equal to `judge`: `E_SIG_SIGNER`.

**5.6 Domain.** `galeed/1` defines exactly one EIP-191 domain, and the
list is closed: `galeed/1`, for entries (§5.2). Every message of this ecosystem has one shape, and a length its literal
fixes: a domain literal containing no line feed, one line feed, and a 32-byte value written by §1's
`hex32`, so `0x` and 64 lowercase hexadecimal digits. Two
domains are separated by their literals: no literal carries a line feed, so
each message's literal is exactly the bytes before that message's first
line feed, and two distinct literals yield messages that differ at some
byte, whether the literals differ at a shared position or one is a proper
prefix of the other (the shorter message then carries `0x0A` where the
longer carries the next byte of its literal); EIP-191 binds the entire
message into the digest preimage, so two messages that differ anywhere have
different digests. No byte string is therefore a valid message under two
domains of this shape, and a signature made under one is not a signature
under another. Lengths decide nothing; the literal decides. A future domain
in any `galeed` version, and every domain of a family law that reads this
ledger (covenant, dispatch notes, passes, and whatever follows),
keeps this shape with a literal that differs from every earlier literal of
this family and from every literal of any other law whose messages share
this shape; equal lengths are permitted. A signature made outside
this family over a byte string of an entry message's shape is a valid entry
signature; §5.7 states the producer obligation that closes that door.

**5.7 Producer obligations.** Producers sign with RFC 6979 deterministic
nonces, the HMAC of that procedure being HMAC-SHA256 and the value handed to
it as its message input the 32 bytes of `digest` (§5.3), hashed no further,
the procedure's own reduction of that input and its rejection of an
out-of-range candidate applying as it states them, and a candidate whose `r`
is not the x-coordinate itself being rejected like an out-of-range one, so
that `v` is always 27 or 28; producers emit low-s encodings by replacing an
`s` above (n − 1) / 2 with n − s and flipping `v` between 27 and 28, never
by drawing a further nonce for that reason; two conforming producers
therefore sign one `B6` under one key to one `sig`, and a seat that signs
one position on two tools publishes one entry. A verifier cannot test
determinism and does not try; its tests are §5.4 and §5.5, which are total.
A seat publishes every entry it signs by an anchor of §9.1's form whose data
carries the entry's bytes, sent from a lineage key that has not retired, so
that the entry exists for every reader (§8.1); an entry signed and not so
published is an entry of no ledger, and a seat that holds one holds a draft.
The order in which a seat publishes is its own: §7 orders entries by `seq`,
and §9.5 bounds each by the earliest counted, undiscarded anchor of itself
or of any later entry on its line. A seat hands itself on only to a key that
has held no office in this ledger (§6.6). It anchors every entry the ceding
key signed before it anchors the succession, at block times earlier than
that of every record that carries the succession on any chain of its basis,
and it carries the succession on every chain of the basis it declares: the
ceding key retires at the earliest block time of a record carrying any
succession or migration it signed (§7.4) in every audit whose anchor set
holds such a record, on whichever chain that record sits, and a chain that
carries it is what puts it in the anchor set of a reader that scans that
chain further than another; an entry the ceding key signed at a position
before its handing off, whose record a chain's clock places after that
retirement, is re-anchored by the successor (§8.6). A seat that adds a chain
to its basis first carries on it every `succession` and `migration` of its
ledger. The move is also recorded in the succession book of
`galeed.roster/1` §4, which is where venues and readers learn the new key
and never the seat's authority (§11). A seat that moves to another ledger
law anchors every earlier entry, at block times earlier than that of every
record that carries its `migration` on any chain of its basis, then carries
its `migration` (§6.6a) on every chain of its basis, and on each chain
anchors nothing under the new law before it has carried the `migration`
there, nor at a block time not later than that of the earliest record on
that chain that carries the `migration`. A judge's key signs `galeed/1`
entries and messages of §5.2's shape under the domain literals of this
seat's family laws, literals that no domain of this law spells, all produced
by the judge's own tooling, and nothing else: it never answers an EIP-191
`personal_sign` request from any application, because the domain literal
lives inside the signed payload, and a 75-byte string of the shape
`galeed/1`, line feed, `0x`, 64 lowercase hexadecimal digits, presented by
anyone, is a complete entry signature for an entry the judge has not read. A
verifier cannot test this obligation either; it is the reason a judge's key
carries nothing but the archive: money, payouts, bonds, and every
convenience of a smart account live on other addresses, and a ruling's
effect on any of them is the venue's business (§11). Judges' keys are raw
secp256k1 EOAs. A judge's key authorizes no EIP-7702 delegation: §9.3 gives
an address carrying a delegation designator code, so a delegation voids
every anchor sent from that key while the designator stands, and delegation
convenience belongs to a separate address that carries no archive; an
authorization the judge signed once and does not hold, one with chain id 0
included, is enough for a third party to land a designator in the block of
the judge's anchor, so a judge's key signs no such authorization at all,
except the one authorization that clears a designator the key carries, an
authorization naming the zero address, sent from the key itself and nothing
else; and a seat that cannot rule out an authorization it signed once hands
the seat on and publishes the old key's entries again from the new key
(§8.6). A verifier cannot test this obligation before the fact either; §9.3
is where its cost is charged. ERC-1271 and every contract-mediated signature
are rejected by construction, since §5.4 admits only a 65-byte recoverable
signature: a permanent archive verifies offline with pure math forever, and
nothing whose answer depends on chain state may stand in for a signature.

## 6. Entry types and bodies

**[E]** `body` is an object for every type. For the eight known types, the
tables below state each field's presence (required, optional, or
conditional), its form, and the test applied when it is present. The eight
are `genesis` (§6.1), `amanah` (§6.2), `judgment` (§6.3), `resignation`
(§6.4), `annotation` (§6.5), `succession` (§6.6), `migration` (§6.6a), and
`keybinding` (§6.7). A member is **present** when its key is present,
whatever its value; the only spelling of an absent optional field is the
absent key, and an optional field written with the value `null` is present
with a form its row refuses. A field listed here fails `E_BODY_FIELD` when
it is required and absent, when it is present with a form its row refuses,
or when the one relation a row of this section states between one member's
presence and another's value does not hold: `clause` is present in a
`resignation` body iff `category` is `unlawful-subject` (§6.4). A known
`kind`'s table (§6.3) is part of the `subjectRef` row's form and is tested
under the second clause, never the third. Every other gloss in a form cell
says what the judge means the value to be and states no predicate; whether a
reference names an existing entry, an entry of the named type, or the
version in force is reading law and never a fault of this step (§11). One
token covers every body fault, so no order of tests within a body is part of
the law.

A row's form names the shape a member must have at §4.3 step 12, after §3.5
has already decided charset and number shape at step 1. A row whose form is
`prose string` tests that the value is a string and nothing else; a row
whose form is `token` tests that the value is a non-empty string carrying no
U+0020, the rest of its charset being already decided; a row whose form is
`skeleton string` tests that the value is a string, non-empty where the form
says so, its charset being already decided; a row whose form is `bool` is
satisfied by the two literals of §1 alone; a row whose form is `int` is
failed at this step only by a value of non-numeric shape (`true`, `false`, a string, `null`, an
array, an object), every numeric
spelling outside §3.2 having failed `E_NUMBER` at step 1. Where a row's form
repeats a §3 predicate exactly, that half of the row has no reachable
failing side and contributes no boundary to §12.1; a `token` row does not
repeat one, U+0020 being the one scalar value §3.5 admits and §1's `token`
refuses, and its exclusion is a boundary of this section. The shortcut is
sound because no key of a `token` or `skeleton string` row, and no key on the path from
the body to one (`subjectRef`, which is the only one), ends in `_md`, so no such value lies inside a
prose subtree; a family law that puts one there tests its charset itself. An array field is satisfied by an
array of zero elements unless its row says otherwise, and no row of this
section says otherwise. Every value this section matches (`category`'s closed set, the known
`kind`s of §6.3, and the eight type names) is matched by byte equality of
the decoded string against the literal as spelled here; a value that
differs in one byte, case included, is a different value, and §6.6's
`handover` and `keyrotation` are examples this grammar names and never
matches. Where a row says "object; members beyond
these permitted", the row names the complete set of tests applied to that
object and never the complete set of members it may carry (§6.9). Within an
entry, only the envelope of §4.1 is arity-closed. Outside an entry, §6.9
reaches nothing this law spells with an exact member list, and three such
lists exist: the scan basis with its `chains` objects (§9.4), which is an
audit input; the report of §8.7 with its findings, its rows,
and the `anchors` elements inside a `MISSING` or `ANCHORED` row, which is an
audit output; and the `NO_LABEL` object of §8.7, which §9.4's
well-formedness test produces and which carries its two members and nothing
else. The anchor record of §9.2 is not arity-closed: §9.4 reads its seven
required members and its optional `entry`, and ignores every member beyond
those eight.

**6.1 `genesis`.** Opens the chain (§4.2).

| field | presence | form |
|---|---|---|
| `statement_md` | required | prose string: who sits here and what this seat does, in the judge's words |

**6.2 `amanah`.** The seat's covenant, published: the rules a judge
binds themself to before any case, and the version every later ruling
names.

| field | presence | form |
|---|---|---|
| `version` | required | int: the seat's own numbering of its covenant |
| `text_md` | required | prose string: the covenant in full, in the judge's words |
| `venues` | optional | array; every element an object; `chainId` required int, `contract` required hex20, `codeHash` required hex32; members beyond these permitted (§6.9) |
| `quotes` | optional | object: the seat's quote bytes by venue, read by the family laws (§11) under the forms and the key order they name; this grammar tests that it is an object and nothing of its members |

`venues` is the list of contracts before which this seat says it sits: the
chain, the address, and the runtime code hash of each, so that a reader
resolving a judgment's subject (§6.3) can tell a declared venue from an
undeclared one. `codeHash` is `keccak256` of the runtime bytecode at
`contract` on `chainId`, and is `keccak256` of the empty byte string where
that address carries no code, at a moment the seat does not name here; it is
a claim of the seat, no predicate of this grammar reads it, a proxy's
`codeHash` names the proxy and never its implementation, and an address
carrying an EIP-7702 delegation designator hashes that designator (§9.3).
A declared venue is what a reader consults when choosing a scan basis
(§9.4), and never a term the basis carries: a venue that emits the form of §9.1 anchors the ruling in the
transaction that calls it under the conditions §6.3 states in full, and a
reader who wants that surface covered lists among the basis's `registries`
those declared venues, by address or by the class item whose `codeHash` the
seat claims (§9.4), whose emitting paths it has read at that code over a
range where it stood, and reports the silence for the rest (§9.1). Everything a covenant says beyond `version`, `text_md`, `venues`, and
`quotes` (windows, benches, guilds, evidence regimes) is read by the family
laws (§11) from members the `amanah` body carries beyond this table, which
this grammar permits and does not test (§6.9); what `quotes` carries is read
by the registry law (§11). `version` is the seat's own
numbering and is data for readers: no predicate compares it to a clock, to
a block, or to the `version` of any other entry, versions may repeat, may
decrease with `seq`, and may skip, and none of that is a fault of §4 or a
finding of §8. Two `amanah` entries with equal `version` are two legal
entries; which is in force, and at which block, is reading law (§11).

**6.3 `judgment`.** A ruling, self-sufficient: readable in a century with no
external context.

| field | presence | form |
|---|---|---|
| `subjectRef` | required | object; `kind` required token; the table for the kind, where the kind is known (below); members beyond these permitted (§6.9) |
| `amanah` | required | hex32: the `entry_id` of the `amanah` entry this ruling was rendered under |
| `verdict_md` | required | prose string: the ruling and its reasons, quoting and summarizing the decisive evidence until the reasoning can be assessed from this text alone |
| `decision` | optional | object; no member is required and every member is permitted (§6.9): the ruling's content in the venue's own vocabulary (a rung index, say), echoed here as data |
| `evidence` | optional | array; every element an object; `manifest` required hex32, `produced` required `bool`; members beyond these permitted (§6.9) |
| `txRefs` | optional | array; every element hex32; the empty array is permitted: transactions this ruling relies on |
| `terms` | optional | hex32: the digest of the terms document the parties signed |

The subject of a ruling is opaque to this grammar: it may be a case in an
escrow, a claim on a bond, a suit against a bureau, a licence dispute, or
anything a later venue invents. `kind` is an open enumeration. Two kinds are
known, and a known kind forces its table:

- `evm`: `chainId` required int, `contract` required hex20, `codeHash`
  required hex32, `id` required hex32. `codeHash` is derived exactly as §6.2 derives a
  venue's, a claim of the seat about a moment it does not name, which no
  predicate of this grammar reads; `id` is the venue's own 32-byte
  identifier of the object ruled on, in whatever way that venue derives it.
- `hash`: `sha256` required hex32, `note_md` optional prose string. An
  object outside any chain, named by the digest of its bytes.

A `subjectRef` whose `kind` is neither is subject to two tests only: that it
is an object and that its `kind` is a token. Every other member it carries,
at every depth, is an extra member under §6.9, is permitted, and is
untested; readers that do not recognize a kind keep the ruling and attach no
meaning to its subject. These kind tables are part of the `subjectRef` row
wherever that row appears, in this section and in §6.4 alike. A chain whose
id is at or above 2^53 cannot be named by `chainId` here, in a `venues`
element (§6.2), in an anchor record (§9.2), or in a basis (§9.4), the
universe of §3.1 having no larger integer; a family law that must reach one
defines a `kind` of its own carrying the id in whatever form it chooses,
since the `evm` kind's `chainId` is required and §6.10 admits no
placeholder, and this grammar reads neither the kind nor the id. An anchor
on such a chain is outside every basis (§9.4). `amanah` is a reference into
this same ledger; whether it resolves, and whether the entry it names is an
`amanah` entry, is reading law (§11). `evidence` records what the judge
read: `manifest` is the digest of an evidence manifest in the sense of the
evidence family law, and `produced` is the judge's own finding that the
bytes behind it reached the judge, a statement of the seat and never a chain
fact. The ruling's effect, if any, is the venue's: a venue that executes
rulings reads its own transaction, never this entry, and the grammar tests
forms alone. The anchored hash of a ruling is this entry's `entry_id`
(§2.1): a venue whose ruling call carries that hash in the transaction's own
calldata at one of §9.1's offsets and emits the form of §9.1 anchors the
ruling in the transaction that executes it, whenever a lineage key that has
not retired (§7.4) is that transaction's sender, the receipt carries status
1, and a reader's basis admits the venue, by address or by class, among the
`registries` of a `chains` object whose range holds the anchor's block and
whose `senders` hold that sender (§9.4); the record so found counts in the
reconciliation of §8.5 where §9.3's codeless test is decided in its favour,
and is carried in the `UNPROVEN` or `VOID` list where it is not. The lineage
condition is this grammar's, the form conditions are §9.1's, the verdict is
§9.3's, and the basis conditions are the reader's, and a venue that meets
§9.1 and is left out of a basis is anchored in no report drawn under that
basis. A ruling a relayer broadcasts, a party submits, or a company's
executing proposal performs emits no anchor this law counts where the sender
is outside the lineage: a log whose topic 1 names the judge while another
address sent the transaction is not the form, since topic 1 is the
transaction's sender (§9.1), and a log naming the caller is the form but
yields a record §8.1 discards. Where a lineage key is itself the sender, the
ordinary conditions of §8.1, §9.1, and §9.4 decide it like any other
transaction. A ruling a venue's transaction did not carry, or carried from a
sender outside the lineage or from a retired key, is published by a lineage
key's own anchor like any other entry (§5.7); a venue that carries it
anchors it in the transaction that executes it, and a reader whose basis
admits the venue reads it there.

**6.4 `resignation`.** The seat declines a subject it was named for.

| field | presence | form |
|---|---|---|
| `subjectRef` | required | object; `kind` required token; the table for the kind, where the kind is known (§6.3); members beyond these permitted (§6.9) |
| `amanah` | required | hex32: the `entry_id` of the `amanah` entry in force |
| `category` | required | token from the closed set {`unlawful-subject`, `other`} |
| `clause` | conditional | skeleton string, non-empty: the clause of the covenant relied on; its charset is already decided by §3.3 and §3.5 at §4.3 step 1, which name the first charset fault §3.5's order reaches, `E_UTF8` for bytes that are not UTF-8, `E_JSON` for a raw byte in 0x00 through 0x1F and for a surrogate escape, a scalar above U+FFFF spelled by escapes being spelled by a surrogate pair and named there, and `E_VALUE_CHARSET` for every other spelling of a scalar value outside U+0020 through U+007E, raw, by one `\u` escape, or by one of the two-character escapes `\b`, `\f`, `\n`, `\r`, and `\t` (§3.3); present iff `category` is `unlawful-subject` |
| `statement_md` | required | prose string: the reason, in the judge's words |

`category` answers one fixed question: whether this resignation stakes the
falsifiable claim that the subject was unlawful under the seat's own
published covenant. A seat that stakes it names the clause, so that a party
holding the co-signed terms can refute the claim by showing them; a seat
that declines for any other reason says `other` and states it. The set is
closed and never grows: `other` absorbs every future reason, and the
grammar refuses any third value. What follows from a resignation (fees
forgone, a case timed out, a record marked) is the venue's business.

**6.5 `annotation`.** A note about an earlier entry, or about the ledger.

| field | presence | form |
|---|---|---|
| `subject` | optional | hex32: the `entry_id` of an entry in this ledger |
| `note_md` | required | prose string |

Published bytes are never edited; a correction is an annotation.

**6.6 `succession`.** The seat changes hands.

| field | presence | form |
|---|---|---|
| `to` | required | hex20: the key that signs every later entry (§7.3) |
| `kind` | required | token: why the seat moved; the grammar names `handover` and `keyrotation` as examples, reads neither, and refuses no token beyond them |
| `effective` | required | int: Unix seconds, data for readers |
| `statement_md` | required | prose string |

`kind` records why the seat moved; every token is a legal value, the chain
predicates of §7 treat every value alike, and a future reason for a seat to
move needs no `galeed/2`. `to` names a key that has held no office in this
ledger, the signer included: §7.3 walks whatever a seat writes, §7.5 says
why `to` never names a key that has opened a ledger of its own, and the
succession book of the registry law (`galeed.roster/1` §4) is why a seat
writes no return to a key that held office before, the book never naming a
key that has entered it. A ledger that stops has no closing entry to write
unless it moves to another ledger law (§6.6a): judges carry no duty to keep
writing, and silence is the ordinary end.

**6.6a `migration`.** The seat closes this ledger and moves to another
ledger law under the same key.

| field | presence | form |
|---|---|---|
| `spec` | required | token: the new law's spec id; data for readers |
| `statement_md` | required | prose string |

A `migration` signed by the key in office that the ledger holds (§8.1)
closes the ledger: no key holds authority at any later position (§7.3).
Whatever the ledger holds, the key that signed a migration retires at the
earliest block time of a record carrying any succession or migration it
signed (§7.4), so that nothing it anchors under the new law at a later block
time enters the reconciliation of this ledger, its findings, or its
`MISSING` list in an audit whose anchor set holds a record carrying the
`migration`, which the seat carries on every chain of its basis (§5.7); such
records stand in `DISCARDED` (§12.3). The new law's genesis is signed by the
same key and names this ledger's root and this entry, which makes the new
ledger this one's successor and this ledger its history (§12.3); how the new
law spells that, and how it audits its own entries, are the new law's. A
seat writes one `migration`, at its ledger's last position, and anchors it
as §5.7 states.

**6.7 `keybinding`.** The seat names the key that evidence is encrypted to.

| field | presence | form |
|---|---|---|
| `recipient` | required | token: the encryption recipient, in the spelling the seat's evidence tooling uses where that spelling is a token; a seat whose tooling spells recipients with a space or outside ASCII publishes an encoding of that spelling which is a token, and the evidence family law says which |
| `effective` | required | int: Unix seconds, data for readers |

The grammar tests that `recipient` is a token and nothing about what it
encodes: which encryption scheme, which spelling, and what happens to a
malformed recipient are the evidence family law's, whose tooling produces
the string and fails loudly on a bad one at the moment of use. A seat that
rotates its encryption key publishes another `keybinding`; ciphertext made
to an earlier recipient stays readable by the earlier key, and no predicate
relates two bindings.

**[E] 6.8 Unknown types.** `entryType` is an open enumeration. An entry
whose type is not one of the eight above verifies with §4 in full, its body
an object by §4.3 step 9 like every other body, and §6.9 alone reaching that
body's members: nothing beyond the object test applies, and §4.3 step 12
never fails for such an entry. Readers that do not recognize a type keep the
entry, its position, and its signature, and attach no meaning to its body.
An entry of an unrecognized type is an entry all the same, and an
unrecognized type buys a judge no silence: a report that this ledger is
silent on some subject covers only the types its reader recognized, and §11
leaves to readers and venues what such a claim is worth beside the
`UNKNOWN_TYPE` list of §8.7, which names every ledger entry whose type is
none of the eight of this section, whatever family laws its verifier reads,
and from which a reader strikes the types its own family laws name to be
left with the entries nobody read. §8.7 lists every ledger entry of a type
this grammar does not know, so that a claim of silence can be checked
against what the reader did not read. This is the evolution valve: the
family laws of this seat (dispatch notes among them) add types here, and a
reader that knows a family law applies that type's table as a reading test
of its own (§11) and never as a `galeed/1` entry predicate. No family law
adds a fault to §4.3 step 12 and none removes one; a byte string of an
unrecognized type that passes §4.3 is an entry for every verifier, including
one that reads the family law and finds the body wanting.

**[E] 6.9 Extra members.** Every body may carry members beyond its table, at
any depth, under the value rules of §3. Such members are data: no predicate
in this grammar reads their meaning, and no verifier accepts or rejects an
entry because of them. They are nonetheless inside the bytes the entry's
own preimage `B6` (§5.1) covers. Whether a document the entry hashes, or a
family law, gives an extra member meaning is that document's or that law's
affair (§11); the covenant family law reads its fields from the
members an `amanah` body carries beyond §6.2, and this grammar never sees
them. What no extra member's meaning ever supplies is any output of this
grammar: validity under §4, authority under §7, any finding of §8.2 or §8.4
whether hard or soft, any row or list of §8.7, the label, or an anchor
verdict of §9.3. Every predicate of this grammar reads the members its rows
name and the members §4.1 names, and nothing else; a reader that derives
any output from what an extra member says is reading its own convention
into bytes that carry none. Identity is another matter and is total
(§2.1): an extra member is part of the bytes, so two byte strings that
differ only in one are two entries with two `entry_id`s, they do not
collapse under §8.1, and at one `seq` they equivocate under §8.4 like any
other pair.

**[E] 6.10 No sentinels.** Inside a body, every one of the 2^256 values of
a hex32 field, every one of the 2^160 values of a hex20 field, every
integer of §3.2 in an `int` field, both literals of a `bool` field, every
string a `prose string` or `skeleton string` row admits in such a field,
and every token of §1 in a `token` field whose row does not close the set
(§6.4's `category` is the one row that does) is a legal spelling of that
field, the all-zero hash, the zero integer, the one-byte token, and the
empty prose string included; a legal spelling is not by itself a legal body, since a `subjectRef`
requires its `kind` (§6.3, §6.4); §6.2's `venues` elements, §6.3's
`evidence` elements, and §6.3's known `kind`s each require members of their
own; §6.4's `clause` is non-empty wherever it is present; and §6.4's
`category` and `clause` come together only under one of the two values. The grammar reserves no "unset" marker inside a body: `null` is a
value of the universe (§3.1), a body field that may be absent is optional
and is spelled by the absent key, and a present body field means what its
judge wrote. The one place a `null` carries a fixed meaning is the
envelope's `prev`, where §4.1 gives it that meaning explicitly and
`E_PREV_SEQ` enforces it.

## 7. Chain

**[C] 7.1 Sequence.** Entries of one ledger carry `seq` 0, 1, 2, … with no
gap and no repeat.

**[C] 7.2 Link.** For `seq` ≥ 1, `prev` equals the `entry_id` of a ledger
entry at `seq − 1` (§8.1): a commitment to that entry's exact published
bytes. Where more than one ledger entry occupies `seq − 1`, the link is
satisfied by any one of them and §8.4 records the fork separately; §8.2
states the test, and an entry the lineage left out (§8.7's `EXCLUDED`)
satisfies no link. Through this
link every entry commits to the whole prefix before it along one line of
any fork.

**[C] 7.3 Authority.** A ledger is read under a **root**, the address the
reader names as its judge. The authority at position `k`, written
`auth(k)`, is the key entitled to sign the entry at `seq` `k`:

- `auth(0)` is the root. The `judge` of the entry at `seq` 0 (§4.1) is the
  address that entry claims as its seat; a `seq`-0 ledger entry whose
  `judge` differs from the root is a fault (§8.2) and never redefines
  `auth`.
- If the entry at `seq` `k` is a `succession` whose `judge` is `auth(k)`,
  then `auth(j) = to` for every `j` > `k`, until a later succession changes
  it again. A succession signed by a key that is not `auth(k)` is an
  authority fault at `k` and moves nothing. Where more than one succession
  at `k` carries `auth(k)` as its `judge`, §8.2 states which one moves the
  seat.
- If the entry at `seq` `k` is a `migration` whose `judge` is `auth(k)`, no
  key holds authority at any position after `k`, whatever successions share
  the position, and every ledger entry there is an authority fault.
- Otherwise `auth(k + 1) = auth(k)`.

A ledger entry at `seq` `k` with `k` ≥ 1 must have `judge` equal to
`auth(k)`; its
signature already recovers to `judge` (§5.5), so the chain rule adds exactly
one test: the signer was the key in office. Position 0 carries no authority
test of this kind: a `seq`-0 entry is graded against the root alone (§8.2),
because §4.2 puts a `genesis` and nothing else there and no succession can
precede it. The walk is total: a succession to
the key already in office, to the root, or to any earlier key moves the
seat as written, authority being a function of the prefix; a seat writes
no such succession (§6.6), and the walk decides over every input all the
same. A key that has handed
off and signs again at a later position is an authority fault (§8.2).

**[C] 7.4 Lineage and retirement.** A key's **retirement** is the smallest
`blockTimestamp` of the records of the anchor set, on any chain and whatever
their `sender` and `verdict`, that carry a byte string that passes §4.3, is
a `succession` or a `migration`, and whose `judge` is that key; a key no
such record carries has no retirement. A key's **retiring seq** is the
smallest `seq` among those byte strings, the ones that pass §4.3, are a
`succession` or a `migration` and have that key as `judge`, that a record of
the anchor set whose `blockTimestamp` equals its retirement carries, on any
chain and whatever that record's `sender` and `verdict`. A key **has
retired** at a record whose `blockTimestamp` is later than its retirement,
and at a record whose `blockTimestamp` equals its retirement and that
carries a byte string that passes §4.3, whose `judge` is that key, at a
`seq` greater than its retiring seq, on whichever chain the record sits, and
at no other record: in the second of its handing off, the records the key
sends that carry what it signed at a position after its retiring seq count
for nothing, and a succession it signed at such a position admits nobody
whichever record of that second carries it; what it signed at or before its
retiring seq, what other keys signed, bytes that fail §4.3 and a record that
carries no bytes count as they would at any earlier record, and a record
another key sends is read against that key's own retirement (§8.1). This is
the one place a predicate of the grammar orders two records by time: it
orders a key's records against its own handing off, a `succession` or a
`migration`, every chain alike, reads, at a record of that handing off's own
second, the `seq` of the entry the record carries against the key's retiring
seq, and decides nothing else (§7.6). The **lineage** of a root is the least
fixpoint seeded with the root and extended by the `to` of every carried byte
string the audit reads (§8.1: the `entry` of every record of the anchor set,
whatever the record's `sender` and `verdict`) that passes §4.3, is a
`succession`, whose `judge` is already in the set, and that some record
carries at which its `judge` has not retired, at whatever `seq` ≥ 1 it sits;
§4.2 puts a `genesis` and nothing else at `seq` 0, so no succession opens a
chain and the seed is the root alone. It decides which entries are ledger
entries and which anchor records are this seat's (§8.1): every lineage key,
the keys that have held the seat and the keys a lineage key named as `to` in
a succession this section admits, can put a hash into this ledger's
reconciliation by every record at which it has not retired, and no key
outside the lineage can; a succession carried by no record at which its
signer has not retired extends nothing, on any chain, so a key that handed
off names nobody into the seat. A key that joins the lineage later never
retroactively authorizes an earlier entry or signature; authority at a
position is the walked state of §7.3, and the lineage is the wider set of
the root and every key a lineage key named as `to` in a succession this
section admits, whether or not that succession moved the seat; a succession
that is a ledger entry moves the walk whether or not this section admits it
(§8.2), so the walked authority can be a key outside the lineage, whose own
entries stand excluded (§8.1).

**[C] 7.5 One key, one ledger.** A key opens one chain: §4.2 puts every
genesis at `seq` 0, so two ledgers under one key are two entries at `seq` 0
by one signer, which §8.4 records as equivocation. A succession therefore
names a key that has opened no `galeed/1` chain of its own and holds no seat
under any other ledger law (§12.4) (a genesis under another ledger law,
`galeed/2` included, is no such chain, since it fails §4.3, at `E_SPEC`
where it keeps this envelope and earlier where it does not, and merges no
entries, and what such a `to` imports is that ledger's anchors; a seat moves
to another ledger law under its own key by a `migration` (§6.6a, §12.3) and
never by a succession): a `to` that already holds a `galeed/1` ledger merges
that ledger into the audited set under the ceding root alone, where the
audit reads `BROKEN_CHAIN` from the doubled `seq` 0, while the audit under
the recipient's own root is untouched, because lineage runs forward only
from the root a reader names (§7.4) and the ceding ledger's entries are
excluded there (§8.1); the grammar cannot tell that merger from a fork and
does not try. A seat is handed to a fresh key, and never back (§6.6). The
uniqueness property is detectable exactly when both ledgers reach one
verifier's inputs and the audited root is the one whose lineage reaches
both; a clean label never asserts that no second ledger exists, only that
none was in the inputs. It follows that any lineage key can, at any record
at which it has not retired, name a stranger's address as `to` and force
`BROKEN_CHAIN` on this seat's own audit by importing that stranger's
entries; the exposure is the custody burden §8.4 states, and no reading of
this grammar lets it reach the stranger's audit. Uniqueness is a claim
readers can falsify, never one a label certifies.

**7.6 No clocks.** `effective` and every other time-valued field are data.
No predicate in §7 or §8 compares a clock to anything; §7.4 compares with a
key's retirement the `blockTimestamp` of the records the key sends and of
the records that carry its successions and migrations, and, at a record of
the retirement's own second, reads the `seq` of the entry the record carries
against the key's retiring seq, and §9.4 reads `blockTimestamp` to decide
whether two records are one; the only time this grammar knows is that of
blocks that carry anchors (§9), and that enters only through the anchor set
a verifier declares.

## 8. Audit

An audit is a pure function of two inputs:

1. the audited **root** address;
2. an **anchor set** of records `(chainId, blockNumber, blockTimestamp, tx,
   sender, hash, verdict, entry)` (§9.2), `entry` being the byte string the
   record carries where it carries one and absent otherwise, `verdict` being
   `counted` when the codeless test of §9.3 was decided in the anchor's
   favour, `UNPROVEN` when either of its two states could not be consulted,
   and `VOID` when both were consulted and the test was decided against it,
   with the scan basis that produced it (§9.4).

Its output is a report (§8.7). The same two inputs produce the same report
in every conforming implementation. The anchor set is a set and the basis
has one spelling per scan (§9.4), so no input carries an order that could
reach the report. Two verifiers whose bases are byte-equal, whose chains
agree, and that answer for both account states §9.3 consults produce
byte-equal anchor sets (§9.4) and therefore byte-equal reports; what a
reader holds beside the chain is nothing this law reads.

**[C] 8.1 Input.** The **carried byte strings** are the `entry` of every
record of the anchor set that carries one, each entering once, whatever the
record's `sender` and `verdict`; byte-identical copies collapse to one. Each
carried byte string is tested by §4.3. Those that fail are not entries: they
enter no walk, no `have`, and no `EXCLUDED` row, their `entry_id` is
nonetheless computed (§2.1), and §8.7 lists them as `MALFORMED`. The
**lineage** (§7.4) is the least fixpoint seeded with the root over the
carried byte strings that pass §4.3, each read with the `blockTimestamp` of
every record that carries it. The audit then discards every anchor record
whose `sender` is not in the lineage, and every anchor record at which its
`sender` has retired (§7.4), so that no key outside the lineage, and no
lineage key at a record at which it has retired, can put a hash or a byte
string into this ledger's reconciliation and every other lineage key can;
§8.7's `DISCARDED` list carries every such record with its bytes. The
**ledger** is the set of carried entries whose `judge` is in the lineage and
which at least one undiscarded record with verdict `counted` carries; §8.6
says what the other records make of their bytes. A succession that no
counted undiscarded record carries extends the lineage where §7.4 admits it
and is no ledger entry, so the walk of §8.2 moves no authority by it, and
the ledger entries its `to` signs are graded by §8.2 against the authority
the ledger's own successions set, recording `AUTHORITY_MISMATCH` at every
position 1 and above where that authority is another key (§8.6). A carried
entry whose `judge` is outside the lineage is no ledger entry, whatever
record carried it: it is not suspicious, it is no act of this ledger's
lineage, the key a ledger succession put in office outside the lineage
included (§7.4), and §8.7 lists it as excluded, §8.4 charging the lineage
key whose counted undiscarded record carried it where one did. Nobody is
framed by junk a stranger anchored beside their ledger, and nobody launders
their own record by self-pollution, because signatures decide membership and
nothing else does. The lineage is a function of the carried byte strings and
of the block times of the records that carry them, so a seat that withholds
one succession shrinks it and with it the records the audit reads; nothing
the trim discards leaves the report silently, the `DISCARDED` list carrying
every record the trim discarded, a list bounded by the `senders` the reader
itself declared, and the `EXCLUDED` rows carrying the `judge` of every entry
the shrunken lineage left out. A reader who sees there an address the
ledger's successions never name asks for the succession that would admit it,
and a reader who sees there an address a ledger succession does name asks
for a record carrying that succession at which its signer had not retired
(§7.4); what a seat that withholds a succession buys is a clean label beside
a row it cannot erase, under every basis whose `senders` reached the
withheld key, and silence under every basis that did not.

**[C] 8.2 Walk.** Ledger entries are ordered by `(seq, entry_id)`, `seq`
comparing numerically ascending and `entry_id` bytewise. The walk visits them in that order and records
the findings of this section from the closed vocabulary of §8.3; §8.4's
findings are recorded over pairs and over carried byte strings and by no
visit. It tracks `expected`, the next
`seq` it expects (initially 0), `auth`, the authority in force (initially
the root), `certain`, true until the first gap, and `last`, the `seq` of the
entry visited immediately before the current one, absent at the first visit
and set at the end of every visit, gap or no gap. A visited entry whose
`seq` equals `expected` records no gap, leaves `certain` unchanged, and sets
`expected` to that `seq` plus 1, except that where the `seq` is 2^53 − 1 the
walk leaves `expected` at that value; `last` then equals `expected` at the end of that visit, so the next entry
at that `seq` satisfies both the "equals `expected`" rule and the fork-twin
clause of the first bullet, and the two agree: no gap, and `expected` and
`certain` unchanged. Within one visit the tests are
decided in the order the bullets below are written: `SEQ_GAP` first, so
that the entry whose `seq` opens a gap is graded for authority with
`certain` already false and its own `AUTHORITY_MISMATCH`, if any, is soft.
The first visited entry has no "`seq` visited just before it" and is judged
by `expected` alone. Every finding of this section carries the `entry_id` of the entry visited
when it was recorded.

- **`SEQ_GAP(expected, actual, entry_id)`**, soft: the visited entry's `seq`
  differs from `expected` and from the `seq` visited just before it.
  Afterwards `expected = actual + 1` and `certain = false`; where `actual`
  is 2^53 − 1 the walk leaves `expected` at `actual`, since no entry can
  occupy a position beyond the universe of §3.1. A ledger whose smallest
  `seq` is not 0 records `SEQ_GAP(0, s, …)` first. An entry whose `seq`
  equals the `seq` visited just before it is a fork twin: it records no gap
  and leaves `expected` and `certain` unchanged; §8.4 records the fork.
- **`PREV_MISMATCH(seq, entry_id)`**, hard: the visited entry has `seq` `s`
  ≥ 1, at least one ledger entry has `seq` `s − 1`, and none of those has
  `entry_id` equal to the visited entry's `prev`. The test is over the set,
  so the order of twins at a fork cannot change it. Across a gap no entry at
  `s − 1` exists, the link is unverifiable, and no prev finding is recorded:
  the gap already is the finding.
- **`AUTHORITY_MISMATCH(seq, certain, entry_id)`**, hard iff `certain`: the
  visited entry has `seq` ≥ 1 and its `judge` differs from `auth` for its
  position. Authority is walked state, and walked state is certain only
  while the walk has been contiguous from 0, because a gap could hide a
  succession; after a gap the same finding is recorded soft. After every
  entry at position `k` has been visited, if any of them is a `migration`
  whose `judge` equals `auth` for position `k`, no key holds `auth` for any
  position after `k`, whatever successions share the position, and every
  entry visited later records `AUTHORITY_MISMATCH`; otherwise, if any of
  them is a succession whose `judge` equals `auth` for position `k`, `auth`
  for positions after `k` becomes the `to` of, among those successions, the
  one with the bytewise-smallest `entry_id`; where no succession at `k`
  carries that `judge`, `auth` is unchanged, and every succession at `k` has
  already recorded `AUTHORITY_MISMATCH` for the same reason. A fork at `k`
  is already hard; this rule fixes which findings follow it, and it keeps
  authority on the line the key in office wrote, so that no key that has
  handed the seat on moves it by publishing a second succession at a
  position it no longer holds.
- **`ROOT_MISMATCH(seq, actual, entry_id)`**, hard, `seq` always 0, `actual`
  the entry's own `judge`: an entry at `seq` 0 whose `judge` is not the
  root. Position 0 is tested by this finding alone and never records
  `AUTHORITY_MISMATCH`, whose subject is the succession state position 0
  cannot yet have. It is tested over every `seq`-0 ledger entry, so a
  lineage key opening a competing root under the audited seat is a hard
  fault. Absence of any `seq`-0 entry convicts nobody: where the ledger
  holds an entry at some later position, the missing position 0 is the first
  `SEQ_GAP`, and where the ledger holds no entry at all the walk records
  nothing and the report's entry count carries the whole of the answer.

**[C] 8.3 Hard and soft, and the closed vocabulary.** A finding is hard iff
its entire witness is in hand: two lineage-signed entries that contradict
each other, one lineage-signed entry that contradicts the audited root, a
walked-state contradiction reached while the walk was certain, or a byte
string that a counted anchor record of a lineage key that had not retired at
it carried and that is no entry of this ledger (§8.4). Absence never
convicts. `PREV_MISMATCH`, `ROOT_MISMATCH`, `EQUIVOCATION`, and
`CARRIED_MALFORMED` (§8.4) are hard; `SEQ_GAP` is soft; `AUTHORITY_MISMATCH`
is hard iff `certain`. These six names are the closed vocabulary of the
`findings` list of §8.7 item 3. `MISSING`, `EXCLUDED`, `ANCHORED`,
`UNKNOWN_TYPE`, `MALFORMED`, `UNPROVEN`, `VOID`, and `DISCARDED` name lists
of §8.7, are outside the partition, are never hard, and are never findings
of that list; the four labels of §8.7 are the closed label set; `counted`,
`UNPROVEN`, and `VOID` are the closed anchor verdicts (§9.3), `UNPROVEN`
naming a verdict, a list, and a label alike, and `VOID` a verdict and a
list.

**[C] 8.4 Equivocation, and the carried record.** `EQUIVOCATION(seq, a, b)`,
hard: two ledger entries with different `entry_id` that share a `seq`, or
share a non-null `prev`, where `a` and `b` are their `entry_id`s in bytewise
ascending order and `seq` is the smaller of their two `seq` values. One
finding is recorded for each such unordered pair, whatever number of grounds
the pair meets, so three entries colliding at one position record three
findings. Byte-identical copies are one entry and never equivocate. Which
lineage key signed each side is irrelevant. A fork therefore cannot be
manufactured by any key outside the lineage, nor by a lineage key's own
records at which it has retired (§7.4, §8.1), the records it sent in its
handing off's own second that carry what it signed at a position after its
retiring seq included, and can always be manufactured by any lineage key at
a record at which it has not retired: `BROKEN_CHAIN` says that some key of
this seat's lineage equivocated, and never which one is at fault. A key's
reach ends at its retirement in every audit whose anchor set holds a record
carrying any succession or migration it signed (§7.4), which is why a seat
carries each succession and migration on every chain of its basis (§5.7),
and custody of signatures is part of the judge's whole burden: a `sig` a
judge's tooling produced over bytes the judge chose not to publish is a
complete second entry from the moment it leaves the judge's hands, so a
producer signs a position once and publishes exactly what it signed. A
reader who wants to know which side of a fork was written first prices the
anchors (§9.5), which is the only evidence of order this grammar carries.

`CARRIED_MALFORMED(seq, token, entry_id)`, hard: a byte string carried
(§8.1) by at least one anchor record that has verdict `counted` and that the
trim of §8.1 did not discard, where the byte string either fails §4.3 or
passes it with a `judge` outside the lineage. `entry_id` is the byte
string's `entry_id` (§2.1), which is the `hash` of every record that carries
it (§9.2); `seq` is the entry's `seq` where the bytes pass §4.3 and 0 where
they do not; `token` is the §10 token the decision order of §4.3 names where
the bytes fail it, and the word `FOREIGN` where they pass it, a word outside
§10 that names the second ground. One finding is recorded per such byte
string, whatever number of records carry it, the two grounds excluding each
other. The finding is hard because its entire witness is in hand: the
record's `sender` is a lineage key, recovered from the transaction's own
signature, and the bytes in the record's `entry` are bytes that key's
transaction published under that signature, so a lineage key that publishes
junk, or a stranger's entry, as the bytes it commits to is charged with its
own act, where a stranger who drops the same bytes beside the ledger
convicts nobody (§8.1). The byte string keeps every listing the rest of this
section gives it: a carried byte string that fails §4.3 stands in
`MALFORMED` (§8.7 item 8) and its `hash` in `MISSING` (§8.5), and a carried
entry whose `judge` is outside the lineage stands in `EXCLUDED` (item 6);
the finding is what the carrying adds. A carried byte string that is a
ledger entry records nothing here: it is a ledger entry like any other, and
whether the ledger reads whole with it is for §8.2, this section's first
paragraph, and §8.5 to say, a seat that leaves it out of every later
publication having published it already, on a chain, in its own transaction.
A record the trim of §8.1 discarded, and a record whose `verdict` is
`UNPROVEN` or `VOID`, carries its bytes into the lineage computation of
§8.1, makes no ledger entry (§8.6), and grounds no finding: the first is an
act of a key outside the lineage or of a key at a record at which it has
retired, and the record stands in `DISCARDED` with its bytes (§8.7 item 11);
the second is a commitment §9.3 does not count, and the record stands in the
`UNPROVEN` or `VOID` list with its bytes (items 9 and 10).

**[C] 8.5 Reconciliation.** Let `have` be the set of `entry_id`s of ledger
entries, and let the **counted anchor set** be the `hash` values of the
anchor records whose `verdict` is `counted`, as trimmed by §8.1.

- **`MISSING(hash)`**: every `hash` in the counted anchor set that is not in
  `have`. No ledger entry has that `entry_id`: no counted undiscarded record
  carried bytes under it (§9.1: a log whose data decoded to nothing, or to
  bytes whose `sha256` is another word), or the bytes carried under it are
  no entry (`MALFORMED`, §8.7 item 8), or are an entry whose `judge` is
  outside the lineage (`EXCLUDED`, item 6), or ride only in records that did
  not count or that §8.1 discarded (§8.6). The commitment is on a chain, and
  what a seat can still do is carry the bytes in a later counted anchor of a
  lineage key that has not retired at it, where, passing §4.3 with a `judge`
  in the lineage, they are a ledger entry (§8.6), or shrink the lineage by
  withholding a succession, which §8.1 answers with the `DISCARDED` list.

§8.7's `ANCHORED` names, for every ledger entry, the counted undiscarded
records that committed to it, so that the existence bounds of §9.5 are
computable from a report and never only from the anchor set that produced
it. The test matches exact `entry_id`s and nothing else. `have` holds the
`entry_id`s of ledger entries and nothing else, so a hash whose bytes the
chain carries under a signature outside the lineage is `MISSING` while those
bytes appear in `EXCLUDED` (§8.7); any lineage key can create such a row by
a record at which it has not retired, and the seat clears it only by a
succession that admits that signer to the lineage, which is for good. A
`MISSING` row that names no transaction is an accusation no reader can
re-derive from a chain, so the anchoring transactions travel with it (§8.7),
and a reader who doubts one re-scans the declared basis for that
transaction. An anchor at a later position bounds when earlier entries
existed (§9.5), and does not put their `entry_id`s in the anchor set.

**[C] 8.6 Verdicts and the ledger.** A record's verdict (§9.3) is a fact
about one anchor and never about the bytes it carries. A byte string that no
counted undiscarded record carries, carried only by records whose verdict is
`UNPROVEN` or `VOID`, by records the trim of §8.1 discarded, or by both, is
no ledger entry: the commitment the chain shows did not count, and the bytes
stand beside their record in the `UNPROVEN`, `VOID`, or `DISCARDED` list of
§8.7, in hand and readable, and in no walk. The same bytes, where they pass
§4.3 with a `judge` in the lineage (§7.4), carried also by a counted
undiscarded record are a ledger entry, at the position their own `seq`
names, whatever was published between: two records of one `entry_id` are two
anchors (§9.2), and the bound §9.5 computes is the earliest counted one. A
succession such bytes spell nonetheless extends the lineage where §7.4
admits it (§8.1): lineage is decided by signatures, which no verdict
touches, and it is what lets a seat whose key carries a designator hand the
seat on and publish the old key's entries again from the new one (§5.7).
After the trimming of §8.1, every hash of a counted undiscarded record lands
in exactly one of two places, `have` or `MISSING`.

**[C] 8.7 Report and label.** The report carries these twelve contents,
thirteen members since item 1 carries two; the numbering names them and fixes each list's own order, and the order of
the members inside the report object is §3.4 item 5's. Where a sort key is
an `int`, the comparison is numeric ascending; where it is a string, the
comparison is the bytewise order of §1, and each list names which of its
keys are strings:

1. the root, and the declared basis (§9.4) as a value nested in the report,
   which §3.4 therefore spells canonically and §9.4 gives one spelling per
   scan;
2. the number of ledger entries;
3. the findings of §8.2 and §8.4, sorted by `(position, finding name,
   entry_id, second entry_id)`, where a finding's `position` is the `seq`
   argument the finding carries, `PREV_MISMATCH`, `AUTHORITY_MISMATCH`, and
   `ROOT_MISMATCH` each carrying exactly one, `SEQ_GAP`'s `position` is its
   `actual`, `CARRIED_MALFORMED`'s `position` is its `seq`, `EQUIVOCATION`'s
   `position` is its `seq`, which is the smaller of its two entries' `seq`
   values even where its `a` is the `entry_id` of the entry at the larger,
   `EQUIVOCATION`'s `entry_id` for this sort is its `a` and its `second
   entry_id` is its `b`, every other finding sorts with an empty `second
   entry_id`, which orders before every hex32 under §1's bytewise order,
   finding names compare bytewise, and each distinct finding, a distinct
   name carrying distinct arguments, appears exactly once, the four sort
   keys already separating every pair the walk and §8.4 can record, each
   name being recorded at most once per visited entry, `EQUIVOCATION` once
   per unordered pair, and `CARRIED_MALFORMED` once per carried byte string;
4. `MISSING`, one row per hash carrying the `hash` and, in ascending
   `(chainId, blockNumber, tx)` order, the `(chainId, blockNumber, tx,
   sender)` of every counted anchor record undiscarded by §8.1 that
   committed to it, one element per record, `tx` comparing bytewise, the
   rows sorted bytewise by `hash`;
5. `ANCHORED`, one row per ledger entry, carrying the `entry_id` and, in
   ascending `(chainId, blockNumber, tx)` order, the `(chainId, blockNumber,
   blockTimestamp, tx, sender)` of every counted anchor record undiscarded
   by §8.1 that committed to it, that is, whose `hash` is the entry's
   `entry_id`, whether or not it carries the bytes, `tx` comparing bytewise,
   the rows sorted bytewise by `entry_id`. It is informational and no label
   turns on it. With `MISSING` it closes the reconciliation on the face of
   the report: the `hash` of every counted anchor record undiscarded by §8.1
   appears in exactly one of `ANCHORED` and `MISSING`, and every ledger
   entry in `ANCHORED`. A reader asking what this seat had committed to by a
   block of some chain reads this list and applies §9.5;
6. `EXCLUDED(seq, judge, entry_id)` for every carried entry whose `judge` is
   outside the lineage, whatever record carried it (§8.1), byte-identical
   copies having collapsed to one entry under §8.1, sorted by `(seq,
   entry_id)`, `entry_id` comparing bytewise;
7. `UNKNOWN_TYPE(seq, entryType, entry_id)`, one row for every ledger entry
   whose `entryType` is none of the eight of §6, byte-identical copies
   having collapsed to one entry under §8.1, sorted by `(seq, entryType,
   entry_id)`, `entryType` comparing bytewise. It is informational and no
   label turns on it. The list is over the eight types this grammar knows
   and never over the types a particular verifier's family laws taught it to
   read, so it stays a function of the two inputs alone; a reader that knows
   a family law strikes from it the types that law names and is left with
   the entries nobody read. This is the list §6.8 requires of any claim of
   silence;
8. `MALFORMED(entry_id, token)`, one row for every carried byte string that
   fails §4.3, carrying the §10 token its decision order names,
   byte-identical copies having collapsed to one, sorted bytewise by
   `entry_id`. It is informational and no label turns on it; it exists so
   that a `MISSING` row over bytes the chain carries that are no entry is
   distinguishable from one over a log that carried none;
9. the `UNPROVEN` list: every anchor record undiscarded by §8.1 whose
   `verdict` is `UNPROVEN` (§9.3), one row per record carrying `(hash,
   chainId, blockNumber, tx, sender)` and the member `entry` beside them
   exactly where the record carries one, sorted by the first four in that
   order, `hash` and `tx` comparing bytewise; the label of item 12 turns on
   it;
10. the `VOID` list: every anchor record undiscarded by §8.1 whose `verdict`
    is `VOID` (§9.3), in the same row form and order, `entry` beside the
    five exactly where the record carries one; it is informational and no
    label turns on it;
11. the `DISCARDED` list: every anchor record the trim of §8.1
    discarded, one row per record carrying its seven required members and,
    where the record carries one, its `entry`, sorted by `(sender, chainId,
    blockNumber, tx, hash)`, `sender`, `tx`, and `hash` comparing bytewise;
    it is informational and no label turns on it, and it exists so that a
    record the reader's own basis produced is visible whatever the lineage
    made of its sender (§8.1);
12. one **label** from the closed set, decided in this order:
    1. `BROKEN_CHAIN` if any finding is hard;
    2. else `UNPROVEN` if the `UNPROVEN` list carries a record whose `hash`
       is not the `hash` of an undiscarded record with verdict `counted`, so
       that a reader whose node could not answer for a boundary never reads
       that silence as the seat's;
    3. else `GAPS` if any `SEQ_GAP` was recorded or `MISSING` is non-empty;
    4. else `COMPLETE`.

The report is a canonical value of §3 whose root is an object with exactly
the members `root` (hex20), `basis` (the declared basis), `entries` (int),
`findings`, `missing`, `anchored`, `excluded`, `unknown_type`, `malformed`,
`unproven`, `void`, `discarded`, and `label`, the lists in the orders items
3 through 11 fix. A finding is an
object with the members `name`, `position` (as item 3 defines it),
`entry_id`, `hard` (`true` or `false`), and exactly the arguments §8.2 and
§8.4 write for that name under the names those sections write: `SEQ_GAP`
carries `expected` and `actual`; `PREV_MISMATCH` carries `seq`;
`AUTHORITY_MISMATCH` carries `seq` and `certain`; `ROOT_MISMATCH` carries
`seq` and `actual`; `EQUIVOCATION` carries `seq`, `a`, and `b`, and its
`entry_id` member is its `a`; `CARRIED_MALFORMED` carries `seq` and
`token`. A `MISSING` row is `{hash, anchors}` with
`anchors` a list of `{chainId, blockNumber, tx, sender}`; an `ANCHORED` row
is `{entry_id, anchors}` with `anchors` a list of `{chainId, blockNumber,
blockTimestamp, tx, sender}`; an `EXCLUDED` row is `{seq, judge, entry_id}`; an `UNKNOWN_TYPE` row
is `{seq, entryType, entry_id}`; a `MALFORMED` row is `{entry_id, token}`;
an `UNPROVEN` or `VOID` row is `{hash, chainId, blockNumber, tx, sender}`
with the member `entry` beside them exactly where the record carries one,
spelled as §9.2 spells an `entry` in a report;
a `DISCARDED` row is `{chainId, blockNumber, blockTimestamp, tx, sender,
hash, verdict}` with the member `entry` beside them exactly where the
record carries one, spelled the same way. Where
the well-formedness test of §9.4 refuses the inputs, the output is the
canonical object `{"ok":false,"reason":"NO_LABEL"}` and nothing else; a
failure of a scan (§9.4) produces no report and no output of this section.

`EXCLUDED` is informational and no label turns on it: anyone may publish
such bytes, so it convicts nobody, and it exists so that a truncation is
visible; the one exception is bytes a counted anchor record of a lineage key
that had not retired at it carried, which §8.4 records as
`CARRIED_MALFORMED` beside the row, the row itself moving nothing. A reader
who sees a run of excluded entries whose `seq` continues past the ledger's
last position knows to ask for the succession entry that would admit them,
which is the one entry whose absence a walk cannot see, or, where a ledger
succession names their judge, for a record carrying that succession at which
its signer had not retired, and a reader who sees a `DISCARDED` row under a
sender the ledger's successions never name asks the same question of the
anchor set; a `DISCARDED` row at which its sender had retired is a retired
key's record and asks nothing. `COMPLETE` asserts only that the inputs held
no fault: with zero ledger entries it says that nothing in the inputs was
this root's, and under a basis that covers no chain it says that nothing was
looked for, which is why the entry count and the basis both travel with the
label.

## 9. Anchoring

An anchor is a commitment, published on a blockchain from a lineage key, to
a 32-byte `entry_id`, carrying the bytes it commits to where its log's data
carries them (§9.1). It is the one way an entry comes to exist for a reader
(§8.1): the chain carries every entry, and a ledger is read out of the
chain. A reader with no chain in reach reads it out of proof kits (§9.6)
that carry the same records; a chain whose nodes have let receipts expire
(Ethereum's history expiry) is read the same way, every header staying,
which is the trust a kit reduces to.

**[X] 9.1 The form.** Throughout this section, **the transaction's sender**
is the address a verifier recovers from the transaction's own secp256k1
signature, and **the transaction's calldata** is the top-level transaction's
`data` field, the bytes the sender signed, and never any inner call's
calldata. A transaction that creates a contract carries init code in that
field and no calldata: the containment test below has no field to run
against, so a log emitted during a creation transaction is not an anchor
whatever its topics hold, and a lineage key's deployments carry no anchor
and open no surface. A transaction that carries no such signature (a system,
deposit, or protocol-originated transaction, and any transaction validated
by account code rather than by recovery) has no sender in this grammar: it
anchors nothing, and a log it carries is not an anchor whatever its topics
hold. A transaction has a sender exactly where the chain's own rules
validate it by recovering a secp256k1 signature; a verifier that cannot
tell whether a transaction is such a transaction, an envelope of a type it
cannot decode among them, has not scanned the basis (§9.4). A transaction
whose envelope names a chain other than the one the log sits on is not an
anchor. A transaction whose signature names no chain (a
legacy transaction outside EIP-155) is not an anchor under this grammar,
because the same signed bytes are a valid transaction on every chain and no
reader can say which chain the sender addressed. A receipt that carries no
status field carries no status 1, so no transaction of such a receipt is an
anchor under this grammar; this grammar reads a status byte and never a
post-transaction state root.

- **The form.** A log in the receipt of a transaction with status 1, whose
  `address` field, the account whose storage context executed the emitting
  instruction, is an emitter that some `chains` object of the basis (§9.4)
  admits, by listing its address in `registries` or by listing there a class
  item whose `codeHash` is the `keccak256` of the code the emitter's account
  holds in the state at the end of the log's block, that object's `chainId`
  being the chain the log sits on and its range holding the log's block, so
  that a reader declaring two windows of one chain declares each window's
  registries for that window alone (a log emitted from within a
  `delegatecall` or `callcode` frame is emitted by the account that made
  that call, and a log emitted from an account carrying an EIP-7702
  designator is emitted by that account itself; in every case the emitter is
  the account whose storage the executing code addressed, and never the
  account whose code ran), carrying exactly three topics: topic 0 byte-equal
  to `keccak256("Anchored(address,bytes32,bytes)")`, topic 1 the
  transaction's sender's 20 bytes preceded by 12 zero bytes, and topic 2 the
  anchored hash. Topic 1 is the transaction's sender and never the emitting
  call's `msg.sender`. The 32 bytes of topic 2 must also appear as 32
  consecutive bytes of the transaction's calldata, lying wholly within it
  and beginning at an offset that is either a multiple of 32 or 4 plus a
  multiple of 32 counted from the first byte of the calldata, so that the
  hash a sender is charged with is a hash the sender's own signed bytes
  carried at a word boundary; a log whose topic 2 appears nowhere in the
  calldata at such an offset is not an anchor, whatever address emitted it
  and whatever the basis declares. Containment binds the hash to the
  sender's own signed bytes and never to the sender's intention: every
  argument of an ordinary ABI call sits at such an offset, so a declared
  contract may charge a lineage key with any word that key passed it,
  including one a counterparty chose. The producer obligation below is
  therefore a rule about contracts and never only about transactions: a seat
  anchors through a contract whose emitting paths it has read and whose code
  cannot change under it, and a seat that calls a declared venue with
  arguments it did not choose knows that each of them is a word the venue
  may echo. A reader who declines that exposure lists in `registries` only
  addresses, or code hashes, whose code it has read, and bounds `fromBlock`
  and `toBlock` to the window over which the code it read stood. A log
  carrying fewer or more than three topics is not an anchor whatever its
  topics hold. Its data is read once the log is an anchor and decides
  nothing about whether it is one: where the data is the ABI encoding of one
  `bytes` value, that is, 32 bytes spelling the integer 32, 32 bytes
  spelling a length `L`, `L` bytes, and the fewest zero bytes that bring the
  whole to a multiple of 32, and the `sha256` of those `L` bytes is
  byte-equal to topic 2, the record of §9.2 carries those `L` bytes as its
  `entry`; where the data is anything else, the log is an anchor of topic 2
  that carries nothing and no byte of it is read, §8.5 deciding where its
  hash stands. The declaration this form names is `event Anchored(address
  indexed sender, bytes32 indexed hash, bytes entry)`; the topic 0 preimage
  records no indexing, so a declaration that leaves `hash` unindexed
  produces a log of two topics and a declaration that indexes `entry` one of
  four, and neither is an anchor. The declared registry list bounds which
  logs are read, never which hashes a lineage key is charged with. Any
  contract may emit the form: a registry whose `anchorCarried(bytes32 hash,
  bytes entry)` and `anchorCarriedMany(bytes32[] hashes, bytes[] entries)`
  emit one such log per element of `hashes` and none for an empty array (the
  reference registry refuses a call whose `entry` does not hash to its
  `hash`, holds no state, and has no owner, and nothing about its code
  enters this test), or a venue whose ruling call emits it for the ruling's
  `entry_id` and bytes. A venue anchors in its own ruling transaction only
  when a lineage key of the seat that has not retired (§7.4) is that
  transaction's sender, the transaction's calldata carries the hash at such
  an offset, and the venue emits the form itself; a venue that reaches a
  registry through an inner call (whose log names the venue, since
  `msg.sender` there is the venue), a venue that derives the hash from
  storage or from a computation, a venue reached through a batcher or a
  packed payload anchor nothing in that transaction; a transaction sent by
  anyone but a lineage key that has not retired, a party, a relayer, a smart
  account, and a retired key among them, puts no hash into this seat's
  reconciliation whatever venue it reached, §8.1 discarding its record and
  §8.7 item 11 carrying it where the basis's `senders` reach that sender,
  and no record arising where they do not (§9.4); and the ruling is anchored
  from a lineage key's own transaction like a ruling of a venue that never
  heard of this grammar. This is the whole price of reading one log, one
  calldata, and, where the basis admits by class, one code hash, and never
  the contract's conduct. The form's exposure is in the reader's hands: a
  lineage key sends no transaction to any contract that emits it except to
  anchor, a seat that cannot tell which contracts do so anchors through
  addresses it deployed or read, and a reader who names an address in
  `registries`, or a class item that an emitter's code matches, charges this
  seat with every 32-byte word of the seat's own signed calldata that
  address, or any contract of that code, ever echoed into topic 2, whatever
  the emitter meant by it, the basis in the report being where that choice
  is visible. A reverted transaction emits no log and anchors nothing; a log
  with the same signature from an emitter no `chains` object admits for its
  own chain and block is not an anchor. What the form adds is that the bytes
  a lineage key committed to travel in the transaction that committed to
  them, which §8.1 reads and §8.4 charges, so that a seat cannot commit to
  an entry and keep its text.

The grammar has no deployment dependency on any one contract: the reference
registry is a convenience, and any contract a lineage key calls that emits
the form for a word of its own calldata, and the bytes behind it, anchors.

**[X] 9.2 Anchor record.** A verifier represents each anchor as `(chainId,
blockNumber, blockTimestamp, tx, sender, hash, verdict, entry)`, where
`blockTimestamp` is the timestamp of the block header that includes the
anchor, `sender` is the transaction's sender, and `verdict` is `counted`,
`UNPROVEN`, or `VOID` under §9.3. Each member carries a stated form:
`chainId`, `blockNumber`, and `blockTimestamp` are `int`s of §3.1; `tx` and
`hash` are hex32; `sender` is hex20; `verdict` is one of the three tokens of
§9.3; `entry`, present where the log's data carried the bytes (§9.1) and
absent where it carried none, is a byte string whose `sha256` is the
record's `hash`, written in a report as the string `0x` followed by the
lowercase hexadecimal digits of its bytes, the empty byte string written
`0x`. A record that lacks one of the seven members other than `entry`, or
whose member carries any other form, or whose `entry` is present and is not
a byte string whose `sha256` is its `hash`, is not a `galeed/1` anchor
record, and §9.4's no-label outcome follows; every comparison and sort over
these members is over these spellings, and two records agree on `entry`
where both lack it or both carry equal bytes, this one member being compared
on its bytes and never on a transport's spelling of them. Two records with
equal `hash` on different chains, or in different transactions, are two
anchors of one `entry_id`; where one chain includes one transaction hash in
two blocks, the two inclusions are two anchors of one hash. On each chain
the counted record undiscarded by §8.1 with the smallest `blockTimestamp`
gives the entry's existence bound relative to that chain (§9.5), and
`blockNumber` orders anchors inside one chain and never across chains.
Chains are not ranked by the grammar; how much a given chain's block order
is worth is for whoever weighs the evidence, and the one place the grammar
sets records of two chains side by side is a key's retirement (§7.4), which
reads every chain's `blockTimestamp` alike.

**[X] 9.3 Codeless at the anchor.** An anchor counts only if the sending
address has no code in the state at the end of the block before the anchor's
block and no code in the state at the end of the anchor's block; where the
anchor's block is block 0 of its chain, the state before it is the chain's
pre-genesis state, in which every address is absent and therefore carries no
code, and this sentence consults that boundary and decides it in the
anchor's favour, so that only the second boundary requires a query to a
chain and both boundaries count as consulted for the verdicts below. An
address absent from the state trie has no code. An account carrying an
EIP-7702 delegation designator has code. Both boundaries are objective facts
about two block states, so the verdict cannot be changed after the fact: a
transaction that installs a designator on its own sender carries no anchor
that counts while the designator stands at the end of its block, its record
then being `VOID`, and a judge who authorizes a delegation voids every
anchor sent from that key in every block at whose end the designator stands
and in the block after each such block, and voids none sent before. §9.1's
topic 1 already requires an anchor's log to name the transaction's own
sender, so under this grammar's own form no delegate lets a third party
originate an anchor under the judge's name; the reason this test stands
beside that closure is that an address carrying code is an address whose
transactions this grammar cannot read as one key's own act under any future
the EVM may take, and the grammar declines to distinguish today's delegate
semantics from tomorrow's. The cost is stated plainly: a third party holding
an authorization the judge signed once can land a designator in the block of
the judge's anchor and void it, and a seat that cannot rule out such an
authorization anchors on a chain the authorization does not reach, and where
an authorization of chain id 0 leaves no such chain, hands the seat to a
fresh key (§6.6), which the designator does not reach, a designator that
stands voiding every anchor its own key sends on that chain until the block
after the one in which it is gone, and the old key retiring at the earliest
block time of a record carrying any succession or migration it signed
(§7.4); a re-anchor after the designator is cleared buys a bound at the
later block and never the bound the voided anchor carried. When either state
cannot be consulted, the verifier's verdict for the anchor is `UNPROVEN`:
the record enters the anchor set carrying that verdict, takes no part in the
reconciliation of §8.5, and is carried in the `UNPROVEN` list of §8.7 where
§8.1 does not discard it, so that an anchor found and not proven is never
silently dropped. Where both states were consulted and the sending address
carried code at either boundary, the verdict is `VOID`: the record enters
the anchor set carrying that verdict, takes no part in the reconciliation of
§8.5, and is carried in the `VOID` list of §8.7 where §8.1 does not discard
it, so that a transaction shaped like an anchor is visible in every report
whatever became of its sender's code, and a judge who delegates in the block
of their own anchor buys no deniability for the bytes the chain shows. Where
both states were consulted and the address carried no code at either
boundary, the verdict is `counted`. A verifier consults the two states
through an endpoint that serves the state of past blocks, and a seat anchors
on a chain whose state readers can consult; a verdict of `UNPROVEN` is the
reader's reach and never the seat's act, which is why §8.7 labels it apart
from `GAPS`.

**9.4 Scan basis.** **[C]** An anchor set is always relative to a declared
basis, itself a canonical value (§3) that is an object with exactly the
member `chains`, an array of objects each with exactly the members `chainId`
(int), `fromBlock` (int), `toBlock` (int), `registries` (an array whose
elements are hex20 addresses or **class items**, objects with exactly the
member `codeHash` (hex32): the emitters whose logs of §9.1's form are
trusted on that chain, registries proper and every venue the reader lists by
address, and every contract whose code hash equals a class item's word, a
venue standard that deploys one instance per case being listed once by its
code), and `senders` (array of hex20, the sender addresses scanned). Every
member named here must carry the form named for it, and `fromBlock` ≤
`toBlock`; `chains` is ordered by `(chainId, fromBlock)` strictly ascending,
two objects of one `chainId` having ranges that do not overlap, and two
objects of one `chainId` that are adjacent, the earlier object's `toBlock`
plus 1 being the later object's `fromBlock`, differing in `registries` or in
`senders`, so that a reader whose node serves two windows of one chain
declares two objects and one coverage still has one spelling, the `int` keys
of the ordering comparing numerically as §8.7's do, a tie being refused by
the overlap rule; `senders` is bytewise ascending and carries no repeated
element, and `registries` lists its hex20 elements first, bytewise
ascending, then its class items, bytewise ascending by `codeHash`, no
element repeated in either part, so that one scan has one spelling and two
reports declare the same scan exactly when their bases are byte-equal. A
basis failing any of these is not a `galeed/1` basis. The form, arity,
ordering, and uniqueness rules of this paragraph are decided from the
basis's own bytes and are class [C]; the completeness rule below, which
consults a chain, is class [X]. No basis names a chain whose id is at or
above 2^53, `chainId` being an `int` of §3.1, and §6.9 reaches no object of
this section. An anchor on such a chain is outside every basis and enters no
anchor set: a verifier that meets one omits it and the input stands, a seat
that sits before a venue there anchors its rulings on a chain this grammar
can name, and a future law that must reach those ids widens §3.1 and this
section together. "Itself a canonical value" means a value of §3's universe;
the basis's own spelling inside a transport is that transport's business,
and §8.7 carries it re-canonicalized. `fromBlock` and `toBlock` bound an
inclusive range. **[X]** A conforming anchor set carries a record for every
log of §9.1's form emitted by an emitter a `chains` object admits, by
address or by class, the class decided on the code hash of the emitter's
account in the state at the end of the log's block, in a block of that
object's range, whose transaction sender is in that object's `senders`, one
record per distinct hash such logs of one transaction carry, since two such
logs carrying one hash agree on the seven members §9.2 names other than
`entry` and are one record, that record carrying an `entry` where any of
those logs carried the bytes (§9.1), and no other record. A verifier that
did not obtain every log the completeness rule enumerates has not scanned
this basis and produces no `galeed/1` anchor set under it: a range it could
not read, a range that reaches past the head of the chain it read (whatever
that range's `registries` list), a chain that did not answer, and a signed
envelope the verifier cannot decode (§9.1) are each a failure of the scan
and never an empty result; the verifier narrows the basis to what it read
and reports under that narrower basis, or produces no report at all.
Producing no report is a fact about the verifier's reach and carries no
output of this law; the `NO_LABEL` object of §8.7 is the output of this
section's well-formedness test and of nothing else in this law; a program
under a measuring contract whose reader refuses a file's bytes as an audit
input answers with the same object, as that contract states, which stands on
§12.1's compared surface, and a path it cannot open is that contract's
misuse and no output of this law. A failure of the scan is an absence of an
answer, and never an answer of absence. `UNPROVEN` is the verdict of a
record that exists and whose §9.3 states could not be consulted, and never
the mark of a log that was never fetched. The basis pins block numbers and
never block hashes; which depth of block a reader is willing to name in
`toBlock`, so that the range it read stays the range it will re-read, is the
reader's affair and no predicate of this grammar. **[C]** §6.9 reaches no
object of this section: an object bearing a member beyond its list, or two
`chains` objects of one `chainId` whose ranges overlap or which are adjacent
with equal `registries` and `senders`, is not a `galeed/1` basis; two
`chains` objects of one `chainId` over two windows are ordinary. `senders`
bounds the scan: a log whose transaction sender is outside `senders` yields
no record even when that sender is a lineage key, and §8.1 does not
re-filter by `senders`. A basis whose `senders` omit an address of the
audited lineage therefore says nothing about that address's anchors, and a
`COMPLETE` label under it carries that silence exactly as one under a basis
that covers no chain does (§8.7); a reader who wants the silence closed
compares `senders` against the root and every `to` in the ledger's
successions. **[C]** The anchor set holds at most one record per `(chainId,
blockNumber, tx, hash)`; two records that agree on the eight members §9.2
names, `entry` agreeing as §9.2 says, are one record, whatever members
either carries beyond them. A record may carry members beyond the eight: no
predicate of this grammar reads them, they enter no report row, and they
never make two records that agree on the eight into two records; the arity
closure of this section reaches the basis and its nested objects alone, a
class item among them. Every record lies within the basis's reach: some one
`chains` object of its `chainId` has both a range holding its `blockNumber`
and `senders` holding its `sender`; a record the basis does not reach is a
record no scan of that basis produces, and the input has no label. The test
reads the basis's declarations and the record's own members and reads
`registries` not at all: it refuses a record whose chain, block, and sender
no declared window holds together, and it never vouches that a record it
admits is genuine, which is what §9.4's completeness rule and §9.6's kits
are for. An input carrying two records that agree on `chainId`,
`blockNumber`, `tx`, and `hash` and disagree on `blockTimestamp`, `sender`,
`verdict`, or `entry`, one carrying an `entry` the other lacks included, or
a record that lacks one of the seven required members §9.2 names or any of
whose members fails the form §9.2 names, an `entry` whose `sha256` is not
the record's `hash` included, or an audit whose root is not hex20, or whose
declared basis is not a `galeed/1` basis under this section, is not a
`galeed/1` audit input, and a verifier handed one returns no label. This
test is decided on the inputs as supplied and before every trimming of §8.1,
so a record the trim of §8.1 would discard still spoils the input; the test
and its no-label outcome are decided from the two inputs of §8 alone and are
therefore inside the freeze of §12. **[X]** Two verifiers whose bases are
byte-equal, run against chains that agree on the blocks their bases name,
and that answer for both account states §9.3 consults, produce byte-equal
anchor sets, because the completeness rule above fixes them. **[C]** The
basis is part of every report (§8.7); a label with an undeclared basis is
not a `galeed/1` audit, and two verifiers with byte-equal bases and
byte-equal anchor sets produce one report.

**[C] 9.5 Time.** An anchor of the entry at `seq` `N` on one chain is an upper
bound, relative to that chain, on when that entry, and every entry reachable
from it by following `prev` through ledger entries, existed,
because each `prev` commits to its predecessor's exact bytes and a hash of
unwritten bytes cannot exist. The same argument reaches the digests those
entries carry, on the terms of the function each names: an anchor bounds,
relative to that chain, the existence of the bytes of every 32-byte digest
a reachable entry names, a `terms`, a `manifest`, an `evm` subject's `id`
and `codeHash`, and a `hash` subject's `sha256` among them, wherever the
document that names the field states a function under which a digest of
unwritten bytes is no more possible than a `prev` of them. This law fixes
`sha256` for `prev`, for every `entry_id` a body names, an `amanah` and a
`subject` among them (§2.1, §6.3, §6.4, §6.5), and for a `hash` subject's
`sha256` (§6.3), and fixes `keccak256` for a `codeHash`, in a `venues`
element and in an `evm` subject alike (§6.2, §6.3); it fixes the function of
no other field, the fields §11 resolves against a document being the rest,
so the bound over the digests this law fixes is this grammar's and the
bound over the rest is the naming document's; what either is worth before
a venue is §11's, and the grammar states the bound alone. The grammar
states one bound per chain and
never a minimum across chains, because §9.2 ranks no chain and a reader who
admits a chain whose timestamps it does not trust would otherwise hand that
chain the bound for the whole ledger; a reader who wants a single bound
takes the minimum over the chains that reader is willing to weigh, and says
which those were. The bound is computed from the `ANCHORED` list of §8.7 and
the ledger entries alone, so it is class [C] and a reader's computation over
a report.
Reachability stops at the first position where no ledger entry matches the
`prev`, so an anchor bounds one line of a fork and never the other, and
bounds nothing across a gap. Density is a cost dial that buys resolution; it
changes no predicate in §4, §7, or §8. Transitivity is a statement about
time and never about set membership (§8.5).

**[X] 9.6 Proof kits.** A self-contained proof of one anchor has these
parts: the chain id the anchor is claimed on, the 32-byte hash claimed, and
the emitting address as the receipt's log carries it, with the emitter's
account proof against the anchor header's `stateRoot` where the recipient's
basis admits emitters by class, so that a recipient can decide §9.1's
admitted-emitter test against a basis of the recipient's own; the anchor's
block header and the header of the block before it, that second header and
its account proof omitted where the anchor's block is block 0; the anchor
transaction with a Merkle proof against the anchor header's
`transactionsRoot`; the anchor transaction's receipt with a Merkle proof
against the anchor header's `receiptsRoot`, since §9.1 requires status 1
and only the receipt carries it; and two account proofs of the sending
address, one against each header's `stateRoot`, each showing either an
account whose code hash is `keccak256` of the empty byte string or, by
exclusion, no account at that address at all, an address absent from the
state trie having no code and satisfying §9.3 on that boundary. An account
proof that shows an account carrying code is a part of the same form and
shows the boundary consulted and decided against the anchor, so a complete
kit whose proofs show code establishes a `VOID` anchor in the sense of §9.3
exactly as one whose proofs show none establishes a `counted` one; what a
kit never shows is a boundary it carries no proof of. A verifier binds both
headers to block hashes it trusts independently and to each other, the
second header's block hash being the anchor header's `parentHash`, recovers
the sender from the transaction, requires it to equal the log's topic 1,
checks both account proofs, verifies the receipt's inclusion at the same
trie index as the transaction's, so that the receipt whose status it reads
is the receipt of the transaction it tested, and applies every test of §9.1
to the kit's transaction and receipt: the status, the emitting address, the
topic 0, the topic count of three, the data's decoding to one `bytes` value
whose `sha256` is topic 2, the bytes being a part of the kit (a kit that
names none establishes an anchor carrying nothing, whose hash §8.5
places), topic 1 against the recovered sender, the calldata containment
of topic 2, and the claimed hash as that log's topic 2. A kit's transaction
must be signed for the chain the kit names, so that a kit cannot be
relabelled to another chain; a transaction whose signature names no chain
establishes no anchor (§9.1). The kit shows nothing about the emitting
contract beyond its address and, where the recipient's basis admits
emitters by class, the account proof that fixes its code hash at the
anchor's block; it carries no code, because §9.1 reads the log, the
calldata, and that one word, and never the contract's conduct. Such a kit
reduces the chain-facing trust to two 32-byte block hashes and outlives any
history expiry. A kit convinces its recipient of one anchor, of its bytes,
and of the verdict §9.3 gives it. A set of kits, one per record a basis
enumerates, is that basis's anchor set for a recipient who verified every
kit (§9.4), and is how a ledger is audited with no chain in reach or after
a chain's nodes let its receipts expire; a recipient persuaded by a kit of
an anchor its own basis does not enumerate widens the basis and scans it,
or reports under the basis it read. A kit missing an account proof of a
boundary that requires a query leaves that state unconsulted and shows its
anchor to be `UNPROVEN` in the sense of §9.3, the pre-genesis boundary of a
block-0 anchor requiring none and its omission being the form this section
names; a kit missing any other part, a kit whose headers are not parent and
child, and a kit whose two proofs stand at two indices show nothing at
all.

## 10. Rejection tokens

Every rejection of a byte string offered as an entry carries exactly one
token from this closed set, decided by the order of §3.5 and §4.3. Refusals
that are not rejections of one byte string carry no token: a refusal for
size (§11), and the no-label outcome a malformed audit input produces
(§9.4). A verifier that rejects for the wrong reason is distinguishable
from one that rejects correctly.

| token | meaning |
|---|---|
| `E_UTF8` | bytes are not valid UTF-8 (§1) |
| `E_JSON` | bytes are not one JSON text, carry a byte order mark outside a string, or carry a surrogate escape |
| `E_NUMBER` | a value position of numeric shape is not an integer in [0, 2^53 − 1] written per §3.2 |
| `E_DEPTH` | container nesting exceeds 128 |
| `E_DUP_KEY` | an object repeats a key after decoding |
| `E_KEY_CHARSET` | a key is empty or carries a scalar value outside U+0020 through U+007E |
| `E_VALUE_CHARSET` | a string value outside every prose subtree carries a scalar value outside U+0020 through U+007E |
| `E_NOT_CANONICAL` | the bytes differ from the canonical bytes of their value |
| `E_ENVELOPE` | the root is not an object |
| `E_ENVELOPE_MISSING` | one of the seven keys is absent |
| `E_ENVELOPE_CLOSED` | a key beyond the seven is present |
| `E_SPEC` | `spec` is not the string `galeed/1` |
| `E_ENTRYTYPE` | `entryType` is not a token |
| `E_JUDGE` | `judge` is not hex20 |
| `E_SEQ` | `seq` is not an int |
| `E_PREV` | `prev` is neither `null` nor hex32 |
| `E_PREV_SEQ` | `prev` is `null` while `seq` is not 0, or `prev` is not `null` while `seq` is 0 |
| `E_BODY` | `body` is not an object |
| `E_SIG_FORM` | `sig` is not hex65 |
| `E_GENESIS_PLACE` | `genesis` off position 0, or position 0 not `genesis` |
| `E_BODY_FIELD` | a required field is absent, a present field has the wrong form, or §6.4's one relation (`clause` present iff `category` is `unlawful-subject`) fails |
| `E_SIG_V` | `v` is not 27 or 28 |
| `E_SIG_RANGE` | `r` or `s` is 0 or ≥ n |
| `E_SIG_HIGH_S` | `s` exceeds (n − 1) / 2 |
| `E_SIG_RECOVER` | public key recovery fails |
| `E_SIG_SIGNER` | the recovered address is not `judge` |

Six further closed vocabularies exist beside this one: the six finding
names (§8.3), the four labels (§8.7), the three anchor verdicts `counted`,
`UNPROVEN`, and `VOID` (§9.3), the two `category` values of §6.4, the four
reading verdicts of §11, and the `token` of a `CARRIED_MALFORMED` finding,
a token of this section or the word `FOREIGN` (§8.4), a word that names the
second ground of that finding and not the reading verdict of §11 that
shares its spelling. Each names a state of a set of entries, of a chain
record, of a field, or of a reference; a rejection token names a state of
one byte string, is carried in no label and in no finding other than
`CARRIED_MALFORMED`, and reaches a report there and in the `MALFORMED` rows
of §8.7 item 8, which record why bytes the chain carries are not entries.

## 11. Outside the grammar

The following are decided by readers, venues, family laws, and the parties
to a case, never by this grammar, and no sentence above should be read as
deciding them:

- **The chains' clocks.** Retirement (§7.4) orders records on different
  chains by their block times, so it is as good as the clocks of the chains
  a basis admits: a chain whose block times run behind the others' lets a
  record there date a handing off earlier than the seat did, and a chain
  whose block times a producer chooses lets the holder of a key that handed
  off date a record before its handing off. Which chains a reader admits
  into a basis, and what margin a seat keeps between its last entries and
  its handing off, are the reader's and the seat's (§5.7).
- **Money and effect.** No entry moves value and no entry executes. A
  ruling's effect is whatever the venue named in its subject does with the
  ruling's hash and its own transaction; the ledger records that the seat
  ruled, and the chain records that the venue acted, and neither record
  stands in for the other.
- **Reference resolution.** Whether an `amanah` or `subject` reference names
  an existing entry: a reader resolves it against a declared **resolution
  basis**, the ledgers the reader has audited, each named by its root and
  the basis of its audit input, and reports one of four verdicts.
  `RESOLVED`: this ledger holds an entry with that `entry_id` and, where the
  row that carried the reference names a type, the entry carries that type.
  `MISMATCHED`: this ledger holds it under another type. `FOREIGN`: this
  ledger does not hold it and some other ledger of the basis does, where
  **this ledger** is the ledger under whose root the reader is reading and
  is named beside the verdict; a byte string that is a ledger entry of two
  roots at once (§7.5) carries its references once under each root and is
  resolved once per root, and where the target itself is a ledger entry of
  two roots, the verdict under the root being read wins and `FOREIGN` is not
  reported. `DANGLING`: no ledger of the basis holds it. Two readers with
  different bases lawfully report `FOREIGN` and `DANGLING` for one
  reference, and the bases say why. The four verdicts answer resolution and
  nothing else. `terms`, `manifest`, `txRefs`, an `evm` subject's `id`, and
  a `hash` subject carry no verdict of this set: each names a transaction,
  an object, or a document that is never an entry of any ledger of this
  grammar. A document is resolved by being shown it and recomputing its
  digest under the function named by the document that named the field, the
  venue's for `terms` and the evidence family law's for `manifest`; this
  grammar fixes neither and tests only the hex32 form, and a field whose
  naming document states no function is a digest no reader can check, which
  is that document's failure. The grammar accepts the entry under every
  outcome.
- **Covenant.** Which `amanah` version a seat offers, what its windows,
  benches, guild pointers, and evidence regimes say, and how a venue freezes
  a version into terms: the family laws, read from `quotes` (the registry
  law), from the members an `amanah` body carries beyond §6.2, and from the
  registry documents those laws define.
- **Venues.** Whether a subject's `contract` and `codeHash` name a venue the
  seat declared, what a `decision` means, and whether a venue executed the
  ruling: the venue's own contract and the family laws that read it. A venue
  may be any contract on any chain; the grammar has no list.
- **Evidence.** What a manifest contains, how bytes are encrypted to a
  `recipient`, and what `produced` should have meant in a given case: the
  evidence family law and the seat's covenant.
- **Dispatch, rosters, verifiers.** Dispatch notes and passes, guild rosters
  and their resolution, and the identity of any address that keeps no
  ledger: family laws of this seat; where one of them adds an entry type
  here, §6.8 admits it.
- **Second rails.** A seat may publish elsewhere a fact this ledger already
  carries: a key change recorded in the succession book of `galeed.roster/1`
  §4, a covenant version named by its `entry_id` in a registry document, an
  encryption recipient placed in metadata. This grammar reads entries under
  a root and knows no other rail: which record a reader acts on, whether a
  seat is obliged to keep the two agreeing, and what follows when they
  disagree, are the family laws' and the readers'. Authority to sign entries
  is §7.3's and is decided by this ledger alone; nothing published on
  another rail moves it, and the registry law reads the succession book as
  the way to an address and never as a key's authority (`galeed.roster/1`
  §9).
- **Precedence and supersession.** Whether a later entry amends, vacates, or
  supersedes an earlier one, which of two rulings on one `subjectRef`
  governs, and whether `seq` order is precedence order: the venue's own
  contract and the covenant it froze into terms. This grammar relates two
  entries by `seq`, by `prev`, and by nothing else; §8.4's equivocation is a
  finding about position and never about subject, and an `annotation` with a
  `subject` changes no earlier entry's standing.
- **Office in time, and liveness.** When an entry was written, whether a key
  was in office at a moment rather than at a position, and whether a seat
  that has stopped writing has ended: §7.3 walks positions and §7.6 reads no
  clock, so the grammar answers none of the three; the one time it compares
  is a key's retirement, at and after which that key's own records at which
  it has retired count for nothing in an audit that holds a record carrying
  any succession or migration it signed (§7.4). An anchor bounds existence
  from above on the chain that carries it (§9.5) and bounds nothing else; a
  seat's liveness is a fact readers establish outside these bytes.
- **Anchoring as existence.** Whether a venue acts on a ruling it has not
  seen carried, what weight a party gives an `UNPROVEN` or a `VOID` record,
  and which chains, registries, senders, and block depth a reader's basis
  should cover: the venue's own rules, the covenant it froze into terms, and
  the reader who publishes the basis. This grammar decides what an anchor is
  (§9.1), what a record of one holds (§9.2), which carried bytes are entries
  (§8.1, §8.6), and what a report says about the set a reader declared
  (§8.5, §8.7).
- **Weighing.** What a ruling is worth, how predictable a seat's rulings
  are, and whether a resignation was honest: readers, and never a score. No
  statistic over this ledger is normative, and neither the chain nor the
  ledger carries one.
- **Glyph similarity.** Prose subtrees carry every Unicode scalar value,
  bidirectional controls and invisible characters included, so two prose
  strings that look alike may differ in bytes and two that differ in bytes
  may look alike. Which bytes a reader is shown is a display question; the
  grammar decides identity by bytes and declines to chase appearance.
- **Size and transport.** No entry has a size ceiling in this grammar; caps
  belong to storage and transport layers and are declared there. A verifier
  may refuse an input it cannot hold, and a refusal for size is not a
  rejection under §10: it names no token, decides nothing about the bytes,
  and is a failure of the scan in the sense of §9.4: a verifier that could
  not hold every log the completeness rule enumerates has not scanned the
  basis and produces no report under it. The freeze criterion of §12.1 is
  over inputs a verifier accepted into memory, and conformance under §12.2
  is agreement on the inputs the measuring contract obliges a verifier to
  accept.
- **Interpretation.** Indexers, statistics, and viewers compete freely; none
  is normative.

## 12. Freeze and evolution

**12.1 The frozen core is the criterion.** The reference implementation of
this grammar freezes only when an independently written implementation,
produced from this text alone, agrees with it on every input of a generated
corpus (the canonical bytes of every input that passes §3.5 tests 1
through 5, `entry_id`, every §3.5 and §4.3 token, every §8 finding, list, and
label, and the `NO_LABEL` object in a report's place), across at least one hundred thousand fuzz
samples, with zero panics, hangs, nondeterministic answers, or wrong
accepts, and with every closed boundary of classes [E] and [C] in this text
witnessed on both sides by at least one corpus input under every seed the
corpus is generated with, the boundary list being a precondition on the
corpus and never a property claimed of it afterwards, and published beside
the corpus as a working instrument. A closed boundary of class [E] or [C]
is a boundary the corpus can witness on the surface §12.2 compares: the
canonical bytes, the `entry_id`, the rejection token, and the report of
§8.7 with its lists and its label, or the `NO_LABEL` object §9.4's test
returns in a report's place. A [C] passage whose output is a reader's
computation over a report and never a member of one (§9.5) states no such
boundary and enters no precondition; the forms §9.2 names are read by the
well-formedness test of §9.4 and are boundaries of class [C] whatever the
mark on §9.2, since that test is decided from the two inputs of §8 alone;
a rule with no reachable failing side is witnessed once and states no
boundary in this sense. The closed boundaries of class [X] are
outside the corpus and outside the freeze; §9.4's completeness rule and
§9.6's kits settle a disagreement about what a chain shows. After the
freeze, the core binds and this text may be rewritten freely as a textbook.

**12.2 Conformance is a test run.** Once §12.5 names a release digest,
agreement with the frozen core on the corpus is the test anyone can run in
seconds, and it is evidence of conformance and never its definition:
conformance is agreement on every input, and an input outside the corpus on
which a verifier differs from the core is a non-conformance the corpus did
not catch and a reason to grow it.

**12.3 Forks are visible.** `galeed/2` is a new spec id in the envelope and
a new domain literal; signatures cannot leak across (§5.6); archives under
`galeed/1` remain valid under `galeed/1` forever and are never re-signed. A
migration keeps the key and is signed on both sides by it: under `galeed/1`
the old chain's last entry is a `migration` (§6.6a) signed by the key in
office, which closes the ledger (§7.3) and retires that key at the earliest
block time of a record carrying any succession or migration it signed
(§7.4), and the new chain's genesis is signed by the same key; that entry is
the whole of what this law can say about a migration. The new spec's genesis
**continues** this ledger when it is signed by the key that signed the
`migration` and names this ledger's root and the `migration`'s `entry_id`. A
ledger so continued is the successor's history and one seat's record with
it: it is audited under this law as it closed and grows past the `migration`
no further, since no key holds authority after it (§7.3) and its key's later
anchors count for nothing (§8.1); an entry of the successor that names an
entry of this ledger, a ruling's `amanah` above all, names it as it would
name an entry of its own, so that a case seated under a covenant of this
ledger is ruled in the successor under that covenant. The same holds between
the successor and any ledger that later continues it, each closed ledger
being the history of the next. A law that succeeds this one states this
paragraph in its own text, and the family laws written for it read a judge's
ledger together with the ledgers it continues. A law that succeeds this one
keeps the `entry_id` of §2.1, the `sha256` of an entry's bytes, and counts a
log in the form of §9.1 as an anchor as this law does, so that a venue that
tests that digest and emits that form, and cannot change, takes and anchors
the rulings of a seat that has moved to it. A new-spec chain whose genesis
names no `galeed/1` `migration` inherits nothing; the old archive's silence
is the whole of the answer. Anchors carry no spec namespace: the form of §9
commits to 32 bytes and carries bytes, and says nothing about which law's
`entry_id` those bytes are; the key that signed the `migration` retires at
the earliest block time of a record carrying any succession or migration it
signed (§7.4), so every new-spec entry it anchors at a later block time
stands, in an audit whose anchor set holds a record carrying the
`migration`, in `DISCARDED` and, failing §4.3 (at `E_SPEC` where it keeps
this envelope, §7.5), in `MALFORMED`, both informational, and in no
`MISSING` row and no finding (§8.1, §8.4). A seat carries the `migration` on
every chain of its basis before it anchors under the new law there, and on
each chain anchors under the new law only at a block time later than that of
the earliest record there that carries the `migration` (§5.7), so a reader
auditing the migrated ledger under the seat's declared basis, to whatever
block on each chain, reads no anchor under the new law into it, and the
label is the one the old ledger's own records earn under that reading.

**12.4 Other ledgers on one key.** Anchors carry no spec namespace: every
hash a lineage key anchors by a record at which it has not retired enters
the reconciliation of every ledger whose lineage holds that key, so a hash
anchored under any other law by a key of this lineage, which no `galeed/1`
byte string can discharge, stands in this ledger's `MISSING` list for good,
and where the anchor carried its bytes also in `MALFORMED` and as
`CARRIED_MALFORMED` (§8.4). A key is therefore a seat under one ledger law
at a time; §7.5 states the obligation this puts on a succession, and §6.6a
the entry that closes a ledger before its key sits under another law.

**12.5 The core's identity.** Naming a release digest here is the freeze,
and it is the only act that performs it. The release digest of the
`galeed/1` core is

    0x5f24cd212705b0cbfd00e5fe0c720ca059b662276c7eaac9a7907e621c50e65d

named on 2026-10-30: the pure Python implementation in
`galeed-conformance/impl-py/` (five source files: `gl1.py`, `glcanon.py`,
`glcrypto.py`, `glentry.py`, `glaudit.py`; the language's standard library
and nothing else), whose release digest
`galeed-conformance/criterion-digest.sh` computes; the corpus, the harness
contract, and the review record beside it say how it converged. From the
naming the core binds and this text is its textbook; a dispute about what
`galeed/1` says on some input is settled by running the core on it. A new
core, or a change to this one, is a new digest named by the same act.
