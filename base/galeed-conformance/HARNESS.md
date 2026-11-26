# gl1 convergence harness: command-line contract

This file fixes the *measuring interface* shared by every independent
implementation of `galeed/1` in this convergence run. It defines nothing
about the law; every rule comes from docs/galeed-v1.md alone. Two
implementations that follow this contract can be run on the same inputs
and their outputs compared byte for byte.

All output is written to stdout as **canonical JSON in the law's own
canonical form (§3.4)**: no whitespace, members sorted bytewise, integers
plain, strings escaped per §3.4. One JSON value per invocation, no trailing
newline. Exit code 0 whenever the program produced its JSON answer (a
rejection is an answer); exit code 2 for harness misuse (a path the
command cannot read, absent, a directory, or otherwise; bad arguments; an argument beyond
a command's own list) and for every other outcome in which the command
produced no JSON answer, a refusal above the sixteen-mebibyte bound among them. No command exits with any code but 0 and 2. The diagnostic an exit 2 writes to stderr is the implementation's own; only the exit code and the empty stdout are part of
this contract. Sixteen mebibytes is every command's bound: no corpus file of
any command exceeds it, no candidate refuses an input of sixteen mebibytes or less for
its size whatever the command, and a candidate that cannot hold an input
above it refuses that input under §11 the same way: exit code 2 and no JSON. A `<path>` argument is handed to the file system as the
operating system delivered it, whatever its bytes; every path the corpus and
`compare.py` name is valid UTF-8, so a candidate whose runtime decodes its
arguments meets this contract, and a path whose bytes are not valid UTF-8 is
outside the measuring interface.

Executable name: `gl1` (Rust: `target/release/gl1` or `cargo run --release --`;
Python: `python3 gl1.py`). Commands:

## `gl1 check <path>`
Reads the file's bytes and applies §3.5 and §4.3 in full.
- Accepted: `{"entry_id":"0x<64 hex>","ok":true}`
- Rejected: `{"ok":false,"token":"E_..."}` with the single token the law's
  decision order names.

## `gl1 canon <path>`
Reads the file's bytes, applies §3.5 tests 1 through 5 (everything except
roundtrip identity), and on success writes the canonical bytes of the parsed
value wrapped as `{"canon":"0x<hex of canonical bytes>","ok":true}`, the
digits lowercase as §1's hex functions write theirs. On
failure `{"ok":false,"token":"E_..."}`. (This lets a corpus be built from
loosely written JSON and lets two implementations be compared on §3.4 alone.)

## `gl1 sign <privkey-hex> <path> [domain]`
`<path>` holds the canonical bytes of a six-member envelope (an entry
without `sig`; any spelling that passes §3.5 tests 1 through 5 is
re-canonicalized, and the value is signed as given: no member is removed, so a file carrying a top-level `sig`
is signed with it inside the preimage). `domain` defaults to `galeed/1`
when the argument is absent; when it is present it is signed as given, the
empty string included, where every byte of it is below 0x80 and none is a line feed, which is what
this contract requires of a literal it will sign, §5.6 itself binding a
literal only to carry no line feed and to differ from every other literal of
the family, with `decimal(len(M))` recomputed from the message, for the
family laws of this seat; any literal outside that shape, one carrying a line feed or a byte
above 0x7F among them, is harness misuse. The domain is tested on its bytes
as the operating system delivered them; a candidate whose runtime decodes
its arguments tests the decoded string's bytes, which refuses the same
arguments, since a line feed decodes to a line feed and every other argument outside
this shape carries a byte at or above 0x80 however it is decoded. The command reads no member of the file
and derives no object from it. `<privkey-hex>` is sixty-four hexadecimal digits, each of either
case, with or without a leading `0x` or `0X`, this argument being a
transport spelling and none of §1's forms, naming a scalar in [1, n − 1];
any other argument, a scalar outside that range included, is harness
misuse. Computes §5.1 through §5.5 with RFC 6979 (HMAC-SHA256 over the §5.3
digest) and low-s by negation, as §5.7 fixes them, and writes
`{"digest":"0x<64 hex>","presig":"0x<64 hex>","sig":"0x<130 hex>","signer":"0x<40 hex>"}`.
A `<path>` whose bytes fail §3.5 tests 1 through 5, and one whose value is
not an object, are each harness misuse: exit code 2 and no JSON answer; an
object of any member count is signed as given. (Producer side; used to build corpora and to
cross-check that two implementations produce byte-identical signatures.)

## `gl1 audit <path>`
`<path>` holds a JSON object with members (the three are required; the
object is open, and a member beyond them is ignored):
- `root`: hex20
- `anchors`: array of objects carrying at least `{chainId:int,
  blockNumber:int, blockTimestamp:int, tx:hex32, sender:hex20, hash:hex32,
  verdict:"counted"|"UNPROVEN"|"VOID"}` and, where the record carries bytes
  (§9.1), `entry`, spelled in the **transport hex** spelling: the two
  lowercase bytes `0x` followed by an even number of hexadecimal digits of
  either case, encoding one byte string, the empty byte string written
  `0x`; this transport encoding is not one of §1's hex forms, and the case
  rule it carries reaches its digits alone, an element spelled `0X` being
  outside the form; a present `entry` member of any other value, `null`
  included, is outside the form. The object is open and a member beyond those eight is
  ignored (§9.4), while the basis and its nested objects are arity-closed;
  the array transports a set, two records that agree on the eight
  collapsing to one under §9.4, `entry` agreeing where both lack it or both
  carry equal bytes
- `basis`: the §9.4 object, in any RFC 8259 spelling of its value; a
  `registries` element is a hex20 string or a class item, an object with
  exactly the member `codeHash` (hex32), the strings first in bytewise
  ascending order and the class items after them in bytewise ascending
  order of `codeHash`, no element repeated in either part (§9.4)

The file is read by an RFC 8259 reader with five restrictions and no
others: the bytes are valid UTF-8; every number, at any depth and in any member,
is written as §3.2 writes an integer and lies in [0, 2^53 − 1], so a sign,
a fraction, an exponent, a leading zero, a value at or above 2^53, or a
literal such as `NaN` each make the file unreadable; no object repeats a member name after escape decoding; every string, a member name included, decodes to
scalar values, a `\u` escape
naming a lone surrogate being refused while a surrogate pair is read as
the one scalar value RFC 8259 says it spells; and a container opening at
nesting depth 129, the root counted as depth 1, makes the file unreadable,
so no reader needs an unbounded stack. A byte order mark is not whitespace
under RFC 8259's `JSON-text` production, so a file beginning `EF BB BF` is
unreadable, as it is for an entry (§3.5). No corpus file exceeds sixteen
mebibytes, and above that bound a candidate may answer or refuse, a refusal
being exit code 2 with no JSON, which `compare.py` never exercises. A file the reader refuses, a file whose root value is not an object, a
required member absent or outside its form, or inputs that are not a
`galeed/1` audit input under §9.4 (a malformed basis, two records agreeing on `(chainId, blockNumber, tx,
hash)` and disagreeing on another of the eight members, one carrying an
`entry` the other lacks included, a record lacking one
of the seven required members, a record whose `entry` is outside the
transport hex spelling or whose `sha256` is not the record's `hash`, a
record no `chains` object of the declared basis reaches, or a root outside
its form) all give
`{"ok":false,"reason":"NO_LABEL"}`; misuse is a path the command cannot read or bad arguments alone, an
argument beyond a command's own list among them, and a refusal above the
sixteen-mebibyte bound takes misuse's exit code and is no fault.
§3.3's charsets are entry predicates and do not reach this file; §3.5's
depth bound does, as the fifth restriction says. Every form this section
names is decided on the string the reader decoded and never on the escapes
the file spelled it with, so an `entry` whose escapes decode to `0x41`
is a byte string of one byte and a `verdict` whose escapes decode to
`counted` is the verdict `counted`; the spelling is the transport's business and the
form is the value's. A member name is likewise the name the reader decoded,
so a member spelled `"\u0072oot"` is the member `root`, and a basis whose
member is spelled `"\u0063hains"` carries `chains` and no member beyond its
list.

Output: the §8.7 report as one canonical JSON object with exactly these
members (empty arrays where a list is empty):
- `root`: hex20
- `basis`: the basis object as given, re-canonicalized
- `entries`: int, the number of ledger entries
- `findings`: array in §8.7 item 3 order; each finding is an object with
  `name` (one of the six), `position` (int), `entry_id` (hex32), and the
  finding's own members: `SEQ_GAP` has `expected` and `actual` (ints);
  `PREV_MISMATCH` has `seq`; `AUTHORITY_MISMATCH` has `seq` and `certain`
  (bool); `ROOT_MISMATCH` has `seq` (0) and `actual` (hex20, the entry's
  own `judge`); `EQUIVOCATION` has `seq`, `a`, `b` (hex32), and its
  `entry_id` member is its `a`; `CARRIED_MALFORMED` has `seq` (int, 0
  where the bytes are no entry) and `token` (string: a §10 token, or
  `FOREIGN`), and its `entry_id` is the carried byte string's `entry_id`.
  Every finding also carries `hard` (bool).
- `missing`: array of `{hash:hex32, anchors:[{chainId:int, blockNumber:int,
  tx:hex32, sender:hex20}, ...]}` in §8.7 item 4 order
- `anchored`: array of `{entry_id:hex32, anchors:[{chainId:int,
  blockNumber:int, blockTimestamp:int, tx:hex32, sender:hex20}, ...]}` in
  item 5 order
- `excluded`: array of `{seq:int, judge:hex20, entry_id:hex32}` in item 6 order
- `unknown_type`: array of `{seq:int, entryType:string, entry_id:hex32}` in
  item 7 order
- `malformed`: array of `{entry_id:hex32, token:string}` in item 8 order
- `unproven`: array of `{hash:hex32, chainId:int, blockNumber:int, tx:hex32,
  sender:hex20}` with `entry` (the string `0x` followed by the lowercase
  hexadecimal digits of the bytes) beside them exactly where the record
  carried one, in item 9 order
- `void`: array in the same row form, item 10 order
- `discarded`: array of `{chainId:int, blockNumber:int, blockTimestamp:int,
  tx:hex32, sender:hex20, hash:hex32, verdict:string}`, the seven members of
  every anchor record the trim of §8.1 discarded, with `entry`
  spelled the same way beside them exactly where the record carried one, in
  item 11 order
- `label`: one of `BROKEN_CHAIN`, `UNPROVEN`, `GAPS`, `COMPLETE`

Where the inputs give no label (above), the output is
`{"ok":false,"reason":"NO_LABEL"}` and nothing else.

## Test key
For corpus construction any secp256k1 private key may be used; a corpus
file may carry the private key it was built with so that both
implementations can re-sign and compare.
