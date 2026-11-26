# gpp convergence harness: command-line contract for galeed.papers/1

The measuring interface shared by every independent implementation of
`galeed.papers/1` (docs/galeed-papers-v1.md). It defines nothing about the
law. Every implementation embeds a conforming `galeed/1` core for canonical form, signing, audit,
`keccak256` and the reader of `galeed.roster/1` §1; the criterion here (`galeed-conformance/papers-py/`)
embeds `galeed-conformance/impl-py/`, the `galeed/1` core whose digest the law's §13.4 carries from the naming; an independent implementation may embed that core or its own, and one
`galeed/1` core may serve a `gro`, a `gpp` and a `gev` alike.

All output is one canonical JSON value (galeed/1 §3.4 form) on stdout, no
trailing newline. Exit 0 whenever the program produced its JSON answer (a
rejection is an answer); exit 2 with empty stdout for every outcome in
which the command produced no JSON answer: harness misuse, and a refusal
above the sixteen-mebibyte bound. Harness misuse is what HARNESS.md makes
it for `gl1`: a path the command cannot read, absent or otherwise; bad
arguments; an argument beyond a command's own list; no command or an
unknown command. A command reads every path its arguments name and tests every
constraint this contract places on a named file before it decides
anything, so an unreadable path and a file outside its stated form are
each misuse whatever the other inputs would have decided, except where
the command's own line names an answer for that fault (the door's audit
input, which answers `LEDGER_INVALID`). A command's
argument list is the token sequence its line below spells: positional
arguments in their places, each option token taking as its value the next
separate token where its line below spells a value and taking none where
its line spells none, options standing in any relative order after the
positional arguments, no option repeated; any other token sequence, a repeated
option, a joined `--now=<int>`, or an option before a positional argument
among them, is misuse. `--now` outside `galeed/1` §3.2's `int` as that
section spells it (decimal digits, no sign, no leading zero, the single
digit `0` included, in [0, 2^53 − 1]) is a bad argument. The diagnostic an
exit 2 writes to stderr is the implementation's own; only the exit code
and the empty stdout are part of this contract. No command exits with any
code but 0 and 2. Sixteen mebibytes is the bound on each file a command
reads; no corpus file exceeds it, no candidate refuses a file of sixteen
mebibytes or less for its size, and above that bound it may answer or
refuse.

Executable name: `gpp`. Commands:

## Documents

- `gpp note-check <path>`: §4.2 over the file's bytes.
  Accepted: `{"doc_id":"0x<64>","ok":true}`. Rejected:
  `{"ok":false,"token":"E_..."}`.
- `gpp pass-check <path>`: §5.2, same shapes.
- `gpp report-check <path>`: §6.2, same shapes.
- `gpp refusal-check <path>`: §7.2, same shapes, plus `"index":k` exactly
  when the token is one of `E_REFUSAL_PHOTO`, `E_REFUSAL_DUP_PHOTO`,
  `E_REFUSAL_PHOTO_ORDER` (the tokens §7.2 says carry an index);
  `E_REFUSAL_PHOTOS` carries none.
- `gpp service-check <path>`: §7a.2, same shapes, plus `"index":k` exactly
  when the token is one of `E_SERVICE_ADDRESS`, `E_SERVICE_DUP`,
  `E_SERVICE_ORDER`; `E_SERVICE_SERVES` carries none. No signature is
  read: a file carrying the four members and a `sig` fails the member set
  (`E_DOC_CLOSED`).
- `gpp sign <privkey-hex> <path> <domain>`: `<path>` holds the canonical
  bytes of the document without `sig` (any spelling that passes `galeed/1`
  §3.5 tests 1 through 5 is re-canonicalized). The command reads no member of the file and removes
  none: a file carrying a top-level `sig` is signed with it inside the
  preimage, exactly as `gl1 sign` does, and stripping it before signing is
  the caller's own step. `<privkey-hex>`, `<path>`, and misuse are as
  HARNESS.md states them for `gl1 sign`: `<privkey-hex>` is sixty-four
  hexadecimal digits of either case, with or without a leading `0x` or
  `0X`, naming a scalar in [1, n − 1], and any other argument is harness
  misuse; a `<path>` whose bytes fail `galeed/1` §3.5 tests 1 through 5,
  and one whose value is not an object, are each harness misuse (exit 2, no
  JSON); an object of any member count is signed. `<domain>` is exactly one
  of `galeed.note/1`, `galeed.pass/1`, `galeed.report/1`,
  `galeed.refusal/1`; any other domain is misuse, since §3.2 closes this law's list and the
  surface measured here is this law's. Output
  `{"digest":..,"presig":..,"sig":..,"signer":..}` as in HARNESS.md.
- `<current>`: the law's §1 `current`, spelled as a hex20 (`galeed/1`
  §1's spelling) or as the literal `unresolved`; any other spelling, an
  uppercase digit or a checksummed address among them, is misuse.
- `gpp pair <note-path> <report-path> <current>`: §6.4. Output
  `{"verdict":"<name>"}`; when `NOTE_INVALID` or `REPORT_INVALID` also
  `"token"` with the document token.
- `gpp pair-refusal <note-path> <pass-path> <refusal-path> <current>`: §7.4. Output
  `{"verdict":"<name>"}`; when `NOTE_INVALID`, `PASS_INVALID`, or
  `REFUSAL_INVALID` also `"token"` with the document token, plus
  `"index":k` exactly when that token carries an index (the three photo
  tokens of §7.2).

## Service

- `<logs.json>`: the registry logs set in HARNESS-ROSTER.md's `<logs.json>`
  form, read the same way: the root is an object whose `publications`,
  `statuses`, `declarations`, and `accessions` members are each present and
  an array; members beyond the four are ignored. Every element of
  `publications` is an object whose `emitter`, `blockNumber`, `logIndex`,
  `document`, and `word` are each present and, respectively, hex20, int,
  int, the transport hex spelling of the log's `document` bytes
  (HARNESS-ROSTER.md's spelling: `0x` and an even number of hexadecimal
  digits of either case), and hex32; every element of `statuses` is an
  object whose `emitter`, `blockNumber`, `logIndex`, and `active` are each
  present and, respectively, hex20, int, int, and a `bool`; every element
  of `declarations` is an object whose `emitter`, `blockNumber`,
  `logIndex`, and `to` are each present and, respectively, hex20, int, int,
  and hex20; every element of `accessions` is an object whose `emitter`,
  `blockNumber`, `logIndex`, `holder`, and `root` are each present and,
  respectively, hex20, int, int, hex20, and hex20; members beyond those are
  ignored. No two elements of the four arrays together share
  `(blockNumber, logIndex)`. The order of the arrays carries nothing. The
  file is read by the reader of `galeed.roster/1` §1, as HARNESS-ROSTER.md
  states, and any violation is misuse. The reading of §7a.3 uses the
  publications alone; the other three arrays are read for form and nothing
  else.
- `gpp service-of <logs.json> <address>`: §7a.3's `serviceOf` over the
  logs set and the address, `<address>` hex20 (any other spelling is
  misuse). Output `{"serves":[...],"verdict":"SERVICE"}` with the record's
  `serves` array as read; `{"verdict":"SERVICE_NONE"}`;
  `{"token":"E_...","verdict":"SERVICE_UNREAD"}` with the candidate's
  token (§7a.2's, or `galeed.roster/1` §2.1's `E_HASH`, `E_JSON`, or
  `E_KIND`), plus
  `"index":k` exactly when that token carries an index;
  `{"verdict":"SERVICE_MISMATCH"}`.
- `gpp serving <logs.json> <verifier> <seat>`: §7a.3's `serving`, both
  addresses hex20 (any other spelling is misuse). Output
  `{"serving":true}` or `{"serving":false}`.

## Payload

- `gpp qr-encode <pass-path> <note-path>`: §8.1. Output
  `{"payload":"galeed-pass:..."}`; on failure
  `{"ok":false,"token":"E_PAYLOAD_PASS"|"E_PAYLOAD_NOTE"|"E_PAYLOAD_CAP"}`
  with `"inner":"E_..."` for the two document tokens.
- `gpp qr-decode <path>`: §8.2 over the file's bytes as the payload. Output
  `{"note":"0x<hex of note bytes>","ok":true,"pass":"0x<hex of pass bytes>"}`,
  the two byte strings in HARNESS.md's transport hex spelling (`0x` then
  an even number of lowercase hexadecimal digits), or
  `{"ok":false,"token":"E_..."}` with `"count":n` for `E_PAYLOAD_SEGMENTS`,
  `"index":k` for `E_PAYLOAD_B64`, and `"inner":"E_..."` for
  `E_PAYLOAD_PASS` and `E_PAYLOAD_NOTE`.

## Door

- `gpp door <note-path> [<pass-path>] [--now <int>] [--audit <audit-input.json>] [--bench <bench.json> | --bench-unresolved] [--current <current>]`:
  §10. The first token after `door` is the note path, whatever its bytes.
  The second positional argument is present iff a token follows the note
  path and does not begin with `--`; omitting it passes `null` for the
  pass (a pass file whose name begins with `--` is named through a path
  such as `./--x`), and a token in that place beginning with `--`, the
  bare `--` included, is read as an option, an unknown one being misuse. Output the §10.4 result object exactly:
  `{"basis":<basis or null>,"bench":<null|"unresolved"|[...]>,"checks":{"anchor":{"state":..,"token":..},"bench":{..},"clock":{..},"ledger":{..},"note":{..},"pass":{..}},"current":<hex20 or "unresolved">,"now":<int or null>,"root":<hex20 or null>,"verdict":"GREEN"|"PARTIAL"|"FAIL"}`,
  each `token` being `null`, a string, or a two-element array of the check's
  token and the carried document token. Omitting `--now` passes `null`;
  omitting `--audit` passes `null`; omitting both bench options passes
  `null`; `--current` takes a `<current>` as its value, and omitting it
  passes `unresolved`; `--bench-unresolved` passes `unresolved` and takes no value, so a token
  after it that is no option is an argument beyond the list; giving both
  bench options, or either twice, is misuse. `<bench.json>` holds a JSON array
  every element of which is `hex20`, bytewise ascending with no repeated
  element (§1), read by the reader HARNESS.md's `gl1 audit` describes; a
  file that reader refuses, a root value that is not an array, an element
  outside `hex20`, or an array out of order or with a repeat is harness
  misuse. `<audit-input.json>`
  is read as a file first (an unreadable path is misuse) and handed to the
  core whole: a file that is no audit input, by the reader's refusal, by a
  root value that is not an object, by a member absent or outside its
  form, or by `galeed/1` §9.4, is the law's invalid input and answers
  `LEDGER_INVALID`, never exit 2.

Audit input files use the shape of HARNESS.md (root, anchors, basis).
