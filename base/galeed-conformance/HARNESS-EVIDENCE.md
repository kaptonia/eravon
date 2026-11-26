# gev convergence harness: command-line contract for courtevidence/1

The measuring interface shared by every independent implementation of the
evidence law (docs/courtevidence-v1.md). It defines nothing about the law.
Every implementation embeds a conforming `galeed/1` core for
`canonical(b)`, the `subjectRef` form, and the audit; the criterion here
(`galeed-conformance/evidence-py/`) embeds `galeed-conformance/impl-py/`, the `galeed/1` core whose digest the law's §8.4 carries from the naming; an independent
implementation may embed
that core or its own, and one `galeed/1` core may serve a `gro`, a `gpp` and
a `gev` alike.

All output is one canonical JSON value (`galeed/1` §3.4 form) on stdout, no
trailing newline. Exit 0 whenever the program produced its JSON answer (a
rejection is an answer); exit 2 with empty stdout for every outcome in
which the command produced no JSON answer: harness misuse, and a refusal
above the sixteen-mebibyte bound. Harness misuse is what HARNESS.md makes
it for `gl1`: a path the command cannot read, absent or otherwise; bad
arguments; an argument beyond a command's own list; no command or an
unknown command; `check`, `digest`, and `pick-recipient` impose no form under which a
readable file is misuse, and every file they can read receives an answer,
the sixteen-mebibyte bound below excepted. A
command reads every path its arguments name and tests every constraint
this contract places on a named file before it decides anything, so an
unreadable path and a file outside its stated form are each misuse
whatever the other inputs would have decided. A command's argument list is
the token sequence its line below spells, positional arguments only; any
other token sequence is misuse. The diagnostic an exit 2 writes to stderr
is the implementation's own; only the exit code and the empty stdout are
part of this contract. No command exits with any code but 0 and 2. Sixteen
mebibytes is the bound on each file a command reads; no corpus file
exceeds it, no candidate refuses a file of sixteen mebibytes or less for
its size, and above that bound it may answer or refuse.

Executable name: `gev`. Commands:

## `gev check <path>`

§2.3 over the file's bytes. Accepted: `{"manifest":"0x<64>","ok":true}`,
the digest `manifest(b)`. Rejected: `{"ok":false,"token":"E_..."}`, plus
`"index":k` exactly when the token is one of the eight file-item tokens
(`E_FILE`, `E_FILE_MISSING`, `E_FILE_CLOSED`, `E_FILE_NAME`,
`E_FILE_SHA256`, `E_FILE_BYTES`, `E_FILE_MEDIATYPE`, `E_FILE_ZIKARON`), `k`
the index of the
faulting element counted from 0. `"note_md":null` at the root answers
`E_MANIFEST_NOTE` and inside a `hash` subject `E_MANIFEST_SUBJECT`, a
present `null` being a member present (law §1).

## `gev digest <path>`

The file-item view of any byte string, for producers listing a file:
`{"bytes":<int>,"sha256":"0x<64>"}`, the length of the file's bytes and
their `sha256`. Every file the command can read answers, whatever its
bytes; a file above the bound may be refused, and the bound keeps `bytes`
inside `galeed/1` §3.1's universe.

## `gev recipient-check <recipient>`

§3.2 over the argument's bytes as the operating system delivered them, an
argument being a byte string the operating system can deliver (one without
a NUL byte); a candidate whose runtime decodes its arguments tests the
decoded string, which refuses the same arguments, every byte of a recipient
being ASCII. Output `{"ok":true}` or `{"ok":false,"token":"E_RECIPIENT"}`.
The answer is the form and never usability: a recipient §3.3 forbids
encrypting to, a small-order point among them, answers `ok` here.

## `gev pick-recipient <audit.json>`

§4 over the audit input the file holds. `<audit.json>` is a `galeed/1`
audit input in the shape HARNESS.md's `gl1 audit` describes (`root`,
`anchors`, `basis`), read as a file first (an unreadable path is misuse)
and handed to the core whole: a file that is no audit input, by the
reader's refusal, by a root value that is not an object, by a member absent
or outside its form, or by `galeed/1` §9.4, is the law's `unread` and
answers `{"pick":"unread"}`, never exit 2. Output one of:

- `{"pick":"unread"}`;
- `{"pick":"none"}`;
- `{"binding":B,"pick":"one"}`;
- `{"list":[B, ...],"pick":"candidates","reason":"fork"|"gap"|"broken"}`,
  the list in §4.1's order;

where a binding `B` is
`{"effective":<int>,"entry_id":"0x<64>","recipient":"<string>","recipientForm":<bool>,"seq":<int>}`.
