# ert convergence harness: command-line contract for eravon.terms/1

The measuring interface shared by every independent implementation of
`eravon.terms/1` (docs/eravon-terms-v1.md). It defines nothing about the
law. Every implementation carries its own implementation of the `galeed/1`
forms, canonical form, `keccak256` and secp256k1 the law reads (law
§10.4); the criterion here (`eravon-conformance/terms-py/`) carries them as
that section names them (`ertcanon.py`, `ertcrypto.py`, `ertlaw.py`).

All output is one canonical JSON value (`galeed/1` §3.4 form) on stdout, no
trailing newline. Exit 0 whenever the program produced its JSON answer (a
rejection is an answer); exit 2 with empty stdout for every outcome in
which the command produced no JSON answer: harness misuse, and a refusal
above the sixteen-mebibyte bound. Harness misuse is what
`galeed-conformance/HARNESS.md` makes it for `gl1`: a path the command cannot read, absent or otherwise; bad
arguments; an argument beyond a command's own list; no command or an
unknown command; and, in this contract, an argument outside the form its
line below names. A command reads every path its arguments name and tests
every constraint this contract places on a named file or argument before
it decides anything, so an unreadable path and an argument outside its
stated form are each misuse whatever the other inputs would have decided.
A command's argument list is the token sequence its line below spells,
positional arguments only; any other token sequence is misuse. The
diagnostic an exit 2 writes to stderr is the implementation's own; only
the exit code and the empty stdout are part of this contract. No command
exits with any code but 0 and 2. Sixteen mebibytes is the bound on each
file a command reads; no corpus file exceeds it, no candidate refuses a
file of sixteen mebibytes or less for its size, and above that bound it
may answer or refuse.

## Arguments

- `<c>`, `<openBy>`, `<buyerBps>`, `<expiry>`: an `int` spelled as
  `galeed/1` §3.2 spells one (decimal digits, no sign, no leading zero,
  the single digit `0` included, in [0, 2^53 − 1]); any other spelling,
  the empty string included, is misuse. `<openBy>` is the order's
  `openBy` of the law's §4.4, an `int` here, so that its form half
  (`E_ORDER_OPENBY`) is unreachable on this surface.
- `<amount>` (for `fee`): a `dec256` as the law's §1 spells it (decimal
  digits, no sign, no leading zero, `0` included, at most 2^256 − 1); any
  other spelling is misuse.
- `<buyerKey>`, `<buyerPayout>`, `<prev>`: a `hex20` as `galeed/1` §1 spells
  it; any other spelling (uppercase digits, a checksummed address, a wrong
  length) is misuse. The law's own tests (nonzero, distinctness) then answer
  in JSON; `<prev>` is the order's `prev` of the law's §4.4, zero admitted,
  so that `E_ORDER_PREV` (the law's test on its form, which has no other
  half) is unreachable on this surface.
- `<privkey>`: sixty-four hexadecimal digits of either case, with or
  without a leading `0x` or `0X`, naming a scalar in [1, n − 1] of
  secp256k1; any other argument is misuse.
- `<sig>` (for `verify-offer`) and `<manifest>` (for `ClaimDelivery`):
  any argument string; the law's §5.1 and §4.5 decide it, `E_SIG_FORM`
  and `E_ACTION_MANIFEST` included, so no spelling is misuse; the empty
  string handed as `<sig>` is the law's §5.4 second form.
- `<hexbytes>` (for `quote` and `fee`): transport hex, the two lowercase
  bytes `0x` followed by an even number of hexadecimal digits of either
  case, `0x` alone spelling the empty byte string; anything else is
  misuse. The law's own length test (§3.2) then answers in JSON.
- `<name>`: exactly one of `ClaimDelivery`, `Confirm`, `Dispute`,
  `Refund`, `Settle`, `Cancel`, followed by that message's arguments as
  §4.5 lists them (one `<manifest>`; none; none; none; `<buyerBps>`
  `<expiry>`; none); another name, or a wrong count, is misuse. `<name>`
  is read before the tokens after it are assigned positions, so those
  tokens are that message's arguments in §4.5's order and then, on
  `verify-action`, `<party>` and `<sig.json>`; a token sequence of any
  other length is misuse.
- `<party>`: exactly `buyer` or `seller`; else misuse.
- `<terms.md>`: a file of any bytes; every readable file receives an
  answer.
- `<sig.json>`: a file read by the reader `galeed-conformance/HARNESS.md`'s `gl1
  audit` describes (RFC 8259 with five restrictions); a file the reader refuses
  is misuse; the root value, of any form, is the signature value the law
  decides, and never misuse: on `verify-order` and for the buyer on
  `verify-action` by §5.2, for the seller on `verify-action` by §5.1 (on
  either, a root that is no string answers `E_SIG_FORM`, an object among
  them, §5.3), and a root that is the empty string by §5.4 on either
  command (the second form, `form: call`, on `verify-order` and for
  `Settle` on `verify-action`;
  `E_SIG_FORM` for the other five messages under the party §4.5 admits
  for each, `E_ACTION_PARTY` under the other).

## The fault object

Every rejection is `{"ok":false,"token":"E_..."}` with, where the law
says the token carries a value, `"index":k` (an element index counted from 0) or `"field":k` (a quote word index).
`E_QUOTE_FIELD` carries `field`; `E_TERMS_DUP_JUDGE`,
`E_TERMS_DUP_GUILD`, `E_LADDER_ORDER`, and every token of this law whose name is or
begins with `E_RUNG`, `E_JUDGE`, `E_GUILD` carry `index`; no other token
of this law carries anything, and no token of this law carries another
token. Where a token of another law of this family carries a fault or a
verdict under `inner`, the carried object is written without an `ok`
member, `{"token":...}` with the members it carries, nested as deep as
the carrying goes; `ok` appears once, at the top of every answer but a
verdict answer (`{"verdict":…}`, HARNESS-LINK.md): a fault answer, an
accepted answer, and the §5.4 answers of the `verify-*` commands, which
are no acceptance; never in a carried fault or verdict; a verdict answer has no
`ok` of its own, and an accepted answer a verdict carries under `document`
(HARNESS-LINK.md, `resolve-link`) is written as that document's command
writes it, its `ok` included.

Executable name: `ert`. Commands:

## `ert check <terms.md>`

§2.4 over the file's bytes. Accepted: `{"ok":true,"termsHash":"0x<64>"}`.
Rejected: the fault object.

## `ert quote <hexbytes>`

§3.2 over the argument's bytes. Accepted:
`{"ok":true,"quote":Q,"quoteHash":"0x<64>"}` where `Q` is an object with
the fifteen members of §3.1 by name: the four addresses as `hex20`;
`premiumBps`, `maxStale`, `feeBps`, `rulingWindow`, `offerWindow`,
`lapseDefault` as ints; `feeFixed`, `feeMin`, `feeMax`, `amountCap` as
`dec256` strings; `autoAccept` as a bool. Rejected: the fault object.

## `ert offer <terms.md> <c>`

§2.4 then §4.3. Accepted:
`{"digest":"0x<64>","offer":O,"ok":true,"structHash":"0x<64>"}` where
`O` carries the seventeen members of §4.1 by name: addresses as `hex20`,
digests as `hex32`, `amount` as a `dec256` string, `deliveryWindow`,
`confirmWindow`, `openWindow`, `nonce`, `expiry` as ints, `buyerMayOpen`
as a bool, `ladder` as an array of
`{"buyerBps":..,"feeBearer":..,"sellerBps":..}`; `digest` is
`offerDigest` of §4.2. Rejected: the fault object of §2.4 or §4.3.

## `ert order <terms.md> <c> <buyerKey> <buyerPayout> <openBy> <prev>`

§2.4, §4.3, then §4.4. Accepted:
`{"caseId":..,"digest":..,"instance":"0x<40>","offerDigest":..,"ok":true,"salt":..}`,
`digest` being `orderDigest`. Rejected: the fault object.

## `ert action <terms.md> <c> <buyerKey> <buyerPayout> <openBy> <prev> <name> [<arg>...]`

§2.4, §4.3, §4.4, then §4.5. Accepted:
`{"digest":"0x<64>","instance":"0x<40>","ok":true,"structHash":"0x<64>"}`,
`digest` being `actionDigest`. Rejected: the fault object.

## `ert sign-offer <privkey> <terms.md> <c>`

Producer side. §2.4, §4.3, then a secp256k1 signature over `offerDigest`
with RFC 6979 (HMAC-SHA256 over the 32-byte digest) and low-s by
negation, as `galeed/1` §5.7 fixes them:
`{"digest":..,"ok":true,"sig":"0x<130>","signer":"0x<40>"}`. The signer
is the key's address, whatever the document's `sellerKey`. Rejected: the
fault object.

## `ert sign-order <privkey> <terms.md> <c> <buyerPayout> <openBy> <prev>`

Producer side. `buyerKey` is the key's address; §2.4, §4.3, §4.4, then a
signature over `orderDigest` as above:
`{"buyerKey":..,"digest":..,"instance":..,"ok":true,"sig":"0x<130>"}`.
Rejected: the fault object (`E_ORDER_BUYER` where the key's address is
the seller's or the judge's, among them).

## `ert sign-action <privkey> <terms.md> <c> <buyerKey> <buyerPayout> <openBy> <prev> <name> [<arg>...]`

Producer side. §2.4, §4.3, §4.4, §4.5, then a secp256k1 signature over
`actionDigest` as above, by whatever key is given (the party rule of §4.5
is a verifier's test and no signer's):
`{"digest":..,"instance":"0x<40>","ok":true,"sig":"0x<130>","signer":"0x<40>"}`.
Rejected: the fault object.

## `ert verify-offer <terms.md> <c> <sig>`

§2.4, §4.3, then §5.1 over `offerDigest`, the argument, and the
document's `sellerKey`. Accepted:
`{"form":"secp256k1","ok":true,"signer":"0x<40>"}`; over the empty
string, the law's §5.4 answer `{"form":"call","ok":true}`, which names no
signer and certifies no signature. Rejected: the fault object.

## `ert verify-order <terms.md> <c> <buyerKey> <buyerPayout> <openBy> <prev> <sig.json>`

§2.4, §4.3, §4.4, then §5.2, or §5.4 where the root is the empty string.
Accepted: `{"form":"secp256k1","instance":..,"ok":true}`; over the empty
string, the law's §5.4 answer `{"form":"call","instance":..,"ok":true}`,
which is no acceptance and certifies no signature. Rejected: the fault
object.

## `ert verify-action <terms.md> <c> <buyerKey> <buyerPayout> <openBy> <prev> <name> [<arg>...] <party> <sig.json>`

§2.4, §4.3, §4.4, §4.5 (its argument tests, then its party rule), then
§5.1 for the seller over the document's `sellerKey` or §5.2 for the buyer
over the `<buyerKey>` argument, each over `actionDigest` (a root value
that is no string answering `E_SIG_FORM`); a root that is the empty
string is the law's §5.4 answer for `Settle` under either party and
`E_SIG_FORM` for the other five messages under the party §4.5 admits for
each, the party rule answering `E_ACTION_PARTY` first under the other.
Accepted: `{"form":"secp256k1","ok":true}`; for `Settle` over
the empty string, the law's §5.4 answer `{"form":"call","ok":true}`,
which is no acceptance and certifies no signature. Rejected: the fault
object.

## `ert fee <hexbytes> <amount>`

§3.2 over the first argument's bytes, then §6.1 over the decoded quote
and the amount. Accepted:
`{"lf":<dec256 or null>,"ok":true,"premium":<dec256>,"priced":<bool>}`,
`lf` being `null` exactly when `priced` is `false`. Rejected: the fault
object (a quote token, or `E_FEE_OVERFLOW`). The document carries no
quote, so the fee is read from the quote bytes a client fetched and the
amount the document names (law §6.2).
