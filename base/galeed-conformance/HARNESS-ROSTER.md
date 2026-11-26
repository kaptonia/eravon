# gro convergence harness: command-line contract for galeed.roster/1

The measuring interface shared by every independent implementation of the
registry law (docs/galeed-roster-v1.md). It defines nothing about the law.
The criterion here (`galeed-conformance/roster-py/`) embeds `galeed-conformance/impl-py/`, the `galeed/1` core whose digest the law's §10.4 carries from the naming, for `keccak256`
and `sha256`, its
implementation of the law's §1 reader, the canonical form of its answers, and the basis
rules of that law's §9.4; an independent implementation may embed that core
or its own, and one `galeed/1` core may serve a `gro`, a `gpp` and a `gev`
alike. An implementation that takes the basis rules from a `galeed/1`
core's `audit` command hands it an audit input whose `root` is any hex20,
whose `anchors` is empty, and whose `basis` is the value under test: the
core answers `NO_LABEL` exactly when the basis fails those rules.

All output is one canonical JSON value (`galeed/1` §3.4 form) on stdout, no
trailing newline; every ascending order named below, and the member order
of every answer, is bytewise over UTF-8 bytes, as the law's §1 fixes. Exit 0
whenever the program produced its JSON answer (a refusal of a guild is an
answer); exit 2 with empty stdout for every outcome in which the command
produced no JSON answer: harness misuse, and a refusal above the
sixteen-mebibyte bound. Harness misuse is what HARNESS.md makes it for
`gl1`: a path the command cannot read, absent or otherwise; bad arguments;
an argument beyond a command's own list; no command or an unknown command;
and, in this contract, a file outside the form its command names below. A
command reads every path its arguments name and tests every constraint this
contract places on a named file before it decides anything, so an
unreadable path and a file outside its stated form are each misuse whatever
the other inputs would have decided; every such outcome shares exit code 2
and empty stdout, so this rule fixes what must be tested and never an order
among the tests. A command's argument list is the token sequence its line
below spells, positional arguments only; any other token sequence is
misuse. The diagnostic an exit 2 writes to stderr is the implementation's
own; only the exit code and the empty stdout are part of this contract. No
command exits with any code but 0 and 2. Sixteen mebibytes is the bound on
each file a command reads; no corpus file exceeds it, no candidate refuses
a file of sixteen mebibytes or less for its size, and above that bound it
may answer or refuse, which is the one place two conforming candidates may
differ.

## Arguments and files

- `<chainId>` is an `int` spelled as `galeed/1` §3.2 spells one
  (decimal digits, no sign, no leading zero, the single digit `0`
  included, in [0, 2^53 − 1]); any other spelling, the empty string, `+1`,
  `1.0`, `-1`, a leading zero, and a value at or above 2^53 among them, is
  misuse.
- `<address>`, `<contract>`, and `<token>` are each a `hex20` as `galeed/1`
  §1 spells it, and `<codeHash>` a `hex32`; any other spelling, an
  uppercase digit, an uppercase `0X` prefix, or a checksummed address
  among them, is misuse.
- `<kind>` is exactly one of the five words `roster`, `guildcard`,
  `judge`, `guildamanah`, `verifier`, naming the reserved prefix
  `galeed.<kind>/` of the law's §1; any other token is misuse.
- **the reader** for every file below is the law's §1 reader, which is the
  one HARNESS.md's `gl1 audit` describes: RFC 8259 with five restrictions
  (valid UTF-8; every number an integer in [0, 2^53 − 1] in §3.2's
  spelling; no repeated member name after escape decoding; no lone
  surrogate escape, a pair read as one scalar value; nesting depth at most
  128). A file the reader refuses is misuse. Every form named below is
  decided on the string the reader decoded and never on the escapes the
  file spelled it with.
- **transport hex**: a string of the two lowercase bytes `0x` followed by
  an even number of hexadecimal digits of either case, `0x` alone spelling
  the empty byte string; this is HARNESS.md's spelling of a byte string and
  none of §1's forms. A string outside it is misuse where a member below
  names it.
- `<logs.json>`: the root is an object whose `publications`, `statuses`,
  `declarations`, and `accessions` members are each present and an array;
  members beyond the four are ignored. Every element of `publications` is
  an object whose `emitter`, `blockNumber`, `logIndex`, `document`, and
  `word` are each present and, respectively, hex20, int, int, the transport
  hex spelling of the log's `document` bytes, and hex32; every element of
  `statuses` is an object whose `emitter`, `blockNumber`, `logIndex`, and
  `active` are each present and, respectively, hex20, int, int, and a
  `bool`; every element of `declarations` (the law's §4.1, one `Declared`
  log) is an object whose `emitter`, `blockNumber`, `logIndex`, and `to` are
  each present and, respectively, hex20, int, int, and hex20; every element
  of `accessions` (one `Acceded` log) is an object whose `emitter`,
  `blockNumber`, `logIndex`, `holder`, and `root` are each present and,
  respectively, hex20, int, int, hex20, and hex20; members beyond those are
  ignored. No two elements of the four arrays together share
  `(blockNumber, logIndex)`. The order of the arrays carries nothing. Any
  violation is misuse.
- `<publication.json>`: the root is one object of the `publications`
  element form above, all five members present and in form, `emitter`,
  `blockNumber` and `logIndex` included though `decode` does not read them;
  any violation is misuse.
- `<bench.json>`: bytes the reader refuses are misuse like any other file;
  every value the reader yields, `true`, `0`, `{}`, and `null` included, is
  decided by the law's §5.5, and none of them is misuse. The file spells
  `B` present; the `B` absent half of §5.5's first row has no spelling here
  and shares `null`'s answer.
- `<quotes.json>`: the root is an object, the `quotes` value of an `amanah`
  body or of a guild covenant as the law's §5.10 reads it; a root that is
  not an object is misuse, and every member of the object is decided by
  §5.10, none being misuse.
- `<query.json>`: the root is an object whose `pointers` member is present
  and an array of strings, and whose `allow` and `deny` members, where
  present, are arrays of hex20; only the absent key means the empty array,
  and a member present with any other value, `null` included, is a
  violation; members beyond the three are ignored. Any violation is misuse.

Executable name: `gro`. Commands:

## `gro decode <publication.json>`

§2.1 over the one publication. Output one of:

- `{"ok":false,"token":"E_..."}` with the token §2.1, §3.2, §3a.2, §3b.2,
  or §3c.2 names;
- `{"kind":"<string>","known":false,"ok":true}` for an unknown kind, `kind`
  being the string as read, and no other member;
- `{"kind":"galeed.roster/1","known":true,"ok":true,"roster":R}` where `R`
  is an object with `guild`, `chainId`, `issued`, `judges` (the address of
  each element, §3.3, in document order, never reordered), `unknownFields`
  (the root's keys beyond §3.1's table, bytewise ascending), and, exactly
  where the document carries it, `bench` as
  `{"protocols":[...],"verifiers":[...]}`: `protocols` as written, and
  `verifiers` one object per element in document order, `{"addr":...}` for
  a string element and `{"addr":...,"name":...,"protocols":[...]}` for a
  verifier object, its three named members as written and no other;
- `{"guildcard":C,"kind":"galeed.guildcard/1","known":true,"ok":true}`
  where `C` is an object with `guild`, `issued`, `unknownFields` (the root's
  keys beyond §3c.1's table, bytewise ascending), and `name`, `home`, and
  `note_md` exactly where the document carries them;
- `{"judge":J,"kind":"galeed.judge/1","known":true,"ok":true}` where `J` is
  an object with `judge`, `issued`, `amanah` as `{"entry_id":...}` (that
  one member, whatever else the document's `amanah` object carries),
  `ledger` as `{"basis":B,"root":...}` (those two members, `B` the basis as
  read, re-canonicalized), `unknownFields` (the root's keys beyond §3a.1's
  table, bytewise ascending), and `name`, `home`, and `note_md` exactly
  where the document carries them;
- `{"guildamanah":G,"kind":"galeed.guildamanah/1","known":true,"ok":true}`
  where `G` is an object with `guild`, `chainId`, `issued`, `version`,
  `text_md`, `sha256` (the `sha256` of the UTF-8 encoding of `text_md`'s
  scalar values, as hex32, §3b.3), `unknownFields` (the root's keys beyond
  §3b.1's table, bytewise ascending), and, exactly where the document
  carries them, `venues` (one object per element carrying `chainId`,
  `codeHash`, `contract` as written and no other member) and `quotes` (the
  object as written, every member kept).

## `gro publication <logs.json> <address> <kind>`

§1's `candidateOf(address, kind)` and §5.1's `active` over `<address>`
itself: no record of the succession book is followed by this command. Output
`{"active":<bool>,"candidate":C}` where `C` is `null` where there is no
candidate, else `{"blockNumber":<int>,"decoded":D,"logIndex":<int>}` with
`D` the `decode` answer of that publication in the shape above.

## `gro resolve-identity <logs.json> <chainId> <address>`

§5.3. Output `{"attempts":[...],"final":"0x...","ok":true,"trail":[...]}`,
each attempt `{"blockNumber":<int>,"logIndex":<int>,"to":"0x..."}` in the
order of §4.2's replay. `<chainId>` is read for its spelling; §5.3 reads no
chain.

## `gro current-at <logs.json> <address> <blockNumber> <logIndex>`

§5.2's `currentAt(address, (blockNumber, logIndex))`: the current key of the
address's record by the replay of the declarations and accessions of the
logs set before that position, the records at it and after it in §4.2's
order (`blockNumber` compared first) unread.
`<blockNumber>` and `<logIndex>` are `int` spellings of `galeed/1` §3.2 (any
other spelling is misuse); the position need not be one a record of the set
holds. Output `{"current":"0x..."}`.

## `gro resolve-guild <logs.json> <chainId> <address>`

§5.4. Output, where the guild resolves,
`{"bench":[...],"card":K,"covenant":V,"guild":"0x...","issued":<int>,"judges":[...],"ok":true,"protocols":[...],"trail":[...],"unknownFields":[...],"verifiers":[...]}`;
`issued` is the roster's `issued` as written; `card` is always present,
the `gro guild-card` answer at the final address `G`; `covenant` is always
present: `null` where §5.9's
`candidateOf` finds no candidate, `{"ok":false,"token":"E_..."}` where §5.9 stops,
with the stop's token, else the `covenant` object of `gro covenant` below;
`guild` is the final address `G` of step 1, byte-equal to the roster's
`guild` by step 5; `unknownFields` is bytewise ascending; each judges row
is `{"final":"0x...","listed":"0x...","trail":[...]}`; each verifiers row
is the same shape with `name` and `protocols` added exactly where the
element is a verifier object; `bench` is the resolved array of §5.4 step
7, bytewise ascending without repeats, and is a different thing from the
roster's `bench` object that `decode` shows. Where the guild does not
resolve, `{"ok":false,"token":"E_..."}`.

## `gro judge <logs.json> <address>`

§5.7's `judgeOf` over `<address>` itself. Output one of
`{"verdict":"JUDGE_NONE"}`; `{"token":"E_...","verdict":"JUDGE_UNREAD"}`
with the candidate's token (a token of §2.1 or §3a.2, or `E_UNKNOWN_KIND`);
`{"verdict":"JUDGE_MISMATCH"}`;
`{"blockNumber":<int>,"document":J,"verdict":"JUDGE","word":"0x..."}` with
`J` the judge object `decode` shows and `word` the publication's word.

## `gro guild-card <logs.json> <address>`

§5.11's `guildCardOf` over `<address>` itself. Output one of
`{"verdict":"CARD_NONE"}`; `{"token":"E_...","verdict":"CARD_UNREAD"}`
with the candidate's token (a token of §2.1 or §3c.2, or `E_UNKNOWN_KIND`);
`{"verdict":"CARD_MISMATCH"}`;
`{"blockNumber":<int>,"document":C,"verdict":"CARD","word":"0x..."}` with
`C` the guild card object `decode` shows and `word` the publication's word.

## `gro resolve-judge <logs.json> <chainId> <address>`

§5.8. Output
`{"active":<bool>,"final":"0x...","judge":D,"ok":true,"trail":[...]}` with
`D` the `gro judge` answer at the final address. `<chainId>` is read for
its spelling; §5.8 reads no chain. A fourth argument is beyond the list and
is misuse.

## `gro covenant <logs.json> <chainId> <address>`

§5.9's `covenantOf` over `<address>` as the final address. Output
`{"covenant":null,"ok":true}` where there is no candidate;
`{"ok":false,"token":"E_..."}` where the reading stops (a token of §2.1 or
§3b.2, `E_UNKNOWN_KIND`, `E_GUILDAMANAH_GUILD_MISMATCH`, or
`E_GUILDAMANAH_CHAIN_MISMATCH`); else
`{"covenant":{"blockNumber":<int>,"sha256":"0x...","version":<int>,"word":"0x..."},"ok":true}`
with `venues` and `quotes` added inside `covenant` exactly where the
document carries them, spelled as `decode` spells them.

## `gro quote <quotes.json> <chainId> <contract> <codeHash> [<token>]`

§5.10's `quoteOf` over the file's object as `Q`, `<token>` being `null`
where the argument is absent. Output `{"key":"...","quote":"0x...","verdict":"QUOTE"}`,
`{"key":"...","verdict":"E_QUOTE_FORM"}`, or `{"verdict":"QUOTE_NONE"}`,
`key` being the key of `Q` the order reached.

## `gro bench <logs.json> <chainId> <bench.json>`

§5.5 over the file's value as `B`. Output `{"bench":V}` where `V` is
`null`, the string `unresolved`, or an array of hex20 bytewise ascending
without repeats.

## `gro candidates <logs.json> <chainId> <query.json>`

§5.6. Output `{"candidates":[...],"perPointer":[...]}`; `candidates` is the
list in construction order; `perPointer` has one object per pointer in
order: the `resolve-guild` answer for that pointer's address with a
`pointer` member carrying the string as read (so a guild that does not
resolve is `{"ok":false,"pointer":"...","token":"E_..."}`), or `{"ok":false,"pointer":"...","token":"E_POINTER"}` where the string is
no pointer, or `{"ok":false,"pointer":"...","token":"E_POINTER_CHAIN"}`
where its chain id differs from `<chainId>`.
