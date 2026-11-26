# The complaint file, `eravon.complaint/1`

A party to a case (the buyer or the seller) writes one complaint file about the case's seated judge from the order page
("Complain about the judge"). The party sends the file to the judge's guild by any way the guild's card gives; the guild's
receiving page reads it. ERAVON writes the file and sends nothing.

## Members

One JSON object, UTF-8:

| Member | Value |
|---|---|
| `v` | `eravon.complaint/1` |
| `chainId` | the chain the case is on (integer) |
| `instance` | the case instance's address (lowercase hex) |
| `caseId` | the case id, 32 bytes (lowercase hex), as the instance's `stage()` reads it |
| `logic` | the logic contract the case was made under (lowercase hex) |
| `judge` | the case's seated judge, the one complained of (lowercase hex) |
| `guild` | the judge's guild as a pointer, `galeed:guild:<chainId>:0x…` |
| `complainant` | an object: `address` (lowercase hex), the key that signs; `role`, `buyer` or `seller` |
| `text` | what the party complains of; not empty |
| `createdAt` | when the file was made (unix seconds) |
| `attachments` | always written, `[]` with none; each an object: `kind` (`dispatch`, `pass`, `audit-input` or `other`), `name` (the file's own name), `sha256` (0x and 64 lowercase hex digits of the bytes), `base64` (the bytes, RFC 4648 §4 standard alphabet `+/`, padded with `=`) |
| `sig` | the complainant's EIP-191 `personal_sign` over the canonical bytes of every other member: 65 bytes as 0x hex, `v` 27 or 28 |

## Canonical bytes

The bytes `sig` covers are the object without `sig`, written as: every object's keys (nested ones too) in byte order; no
blank; strings as raw UTF-8, escaping only `"`, `\` and the control characters U+0000 to U+001F (`\b \t \n \f \r` short,
the others lowercase `\u00xx`; U+007F as it is); integers as their digits. The file itself is the canonical bytes of the
whole object, `sig` included, so one complaint is one file digest. The text and the names are free text: a reader takes
the file as open JSON (RFC 8259) and checks it against its canonical bytes.

## Where it goes

The guild is read from the case itself, never typed: a case drawn from a guild's pool goes to that guild; a judge seated by
name goes to a guild whose current roster on the registry lists the judge, the party picking one where there are several.
Either way the guild is the one `galeed.roster/1` §5.4 resolves: at the address its record moved to on the succession
book, and never a guild that retired; a roster lists the judge where it lists any key of the judge's record (a judge who
rotated keeps its guilds). A judge in no guild has nowhere to go and no file is written. The file is signed only by a
key the order page itself sends with: a company's order and an order read only through another party's consent sign
none.

## Who signs

The complainant is the party's role key as it stands when the file is written: the seat's current key, the new key after a
rotation. The guild finds the party by the key that party's record has moved to on the succession book. A party whose role
is a company contract signs no file from the window (the key on the order is not a key the window holds).
