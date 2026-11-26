# erl convergence harness: command-line contract for eravon.link/1

The measuring interface shared by every independent implementation of
`eravon.link/1` (docs/eravon-link-v1.md). It defines nothing about the
law. Every implementation embeds a conforming `eravon.terms/1` core; the
criterion here (`eravon-conformance/link-py/`) embeds
`eravon-conformance/terms-py/`.

Output, exit codes, misuse, the sixteen-mebibyte bound, and the fault
object are as HARNESS-TERMS.md fixes them. A fault or a
verdict written under `inner` carries no `ok` member, at every depth
(HARNESS-TERMS.md, the fault object); the `document` of `resolve-link`'s
`LISTING` and `SETTLE` answers is the accepted answer as `check` or
`settle-check` renders it, its `"ok":true` included, the one `ok` inside
a verdict answer. `E_LISTING_SIG`'s `index` is the candidate index of the
law's §2.3 numbering, judges first; every other `index` of this law is an
element index as HARNESS-TERMS.md says, except that `BIND_JUDGES` and
`BIND_GUILDS` carry the listing's own count where the lengths differ
(law §7.1). Every address, digest, and byte
string an answer carries is written as `galeed/1` §1's hex functions write
it: `buyerKey` and `instance` as hex20, `salt`, `caseId`, and `digest` as
hex32, and the decoded bytes of `decode` and `receipt-decode` as `0x`
followed by their lowercase hexadecimal digits. `inner` nests through the terms
law's faults where a token of this law carries one (`E_LISTING_SIG`
carries `index` and the signature fault; `E_LISTING_TERMS_DOC` carries
the terms document's fault, with its `index` where that fault has one;
`E_LISTING_BIND` carries a verdict of §7 written as a fault object whose
`token` is the verdict name, with `index` where the verdict carries one;
`E_LINK_DOC`, `E_RECEIPT_DOC`, `E_PACK_LINK`, `E_PACK_LISTING`,
`E_PACK_RECEIPT` carry the document fault; `E_PACK_ORDER` carries a
verdict of §4.4 written as a fault object whose `token` is the verdict
name, with `inner` where it carries a fault; the attachment tokens
`E_PACK_ATTACHMENT`, `E_PACK_ATTACHMENT_HASH`, `E_PACK_ATTACHMENT_UNNAMED`
carry the element's `index`). The count `attachments` of `pack-check`'s
accepted answer is a member of the answer and of no fault. A verdict answer is
`{"verdict":"NAME"}` with the members its command below lists:
`"index":k` where §7 gives the verdict one, `"inner":F` where it carries
a fault, the values §4.4 and §8.3 give `RECEIPT_BOUND`, `RECEIPT_BY_CALL`,
`SETTLE_BOUND` and `SETTLE_BY_CALL`, and the document's answer under
`document` where §3.3 gives `LISTING` or `SETTLE`.

## Arguments

- `<listing.json>`, `<link.json>`, `<document.json>`, `<receipt.json>`,
  `<pack.json>`, `<settle.json>`, `<terms.md>`, `<link.txt>`,
  `<text.txt>`: files of any bytes; every readable file receives an
  answer.
- `<records.json>`: bytes that read, as the `galeed/1` harness's reader
  reads a JSON input (`galeed-conformance/HARNESS.md`, `gl1 audit`), to an
  object with exactly the members `chainId` (an int), `posted`,
  `consented`, `revoked`, `pulled`, `opened`, `events`, `declarations` and
  `accessions`, each an array. Every element is an object with exactly
  `blockNumber` and `logIndex` (ints) and its record's members as the
  law's §6.1 names them: `posted` `poster`, `key`, `hash`, `document`;
  `consented` `who`, `digest`; `revoked` `seller`, `termsHash`; `pulled`
  `seller`, `fund`, `token`, `instance`, `amount`; `opened` `emitter`,
  `caseId`, `seller`, `buyer`, `judge`, `amount`, `termsHash`; `events`
  `emitter`, `name`; `declarations` `from`, `to` (the book's `Declared`);
  `accessions` `holder`, `to`, `root` (the book's `Acceded`). Addresses
  are hex20 and digests hex32 as `galeed/1` §1 spells them (`poster`, `who`,
  `seller`, `fund`, `token`, `instance`, `emitter`, `buyer`, `judge`,
  `from`, `to`, `holder` and `root` hex20; `key`, `hash`, `digest`,
  `termsHash` and `caseId` hex32), `amount` a
  `dec256` as `eravon.terms/1` §1 spells one, `document` the two bytes
  `0x` followed by an even number of lowercase hexadecimal digits (the
  posted bytes), and `name` any string. No two elements of the whole
  object share a position `(blockNumber, logIndex)`, and the elements of a
  list stand in any order. `chainId` names the chain the records were read
  on and is read for its form alone, as every `hash` and every `root` is
  (law §1, §6.1): no predicate reads their values. Any other input is
  misuse. This contract
  states the members and their forms itself, citing the `galeed/1` harness
  for the reader alone.
- `<sellerKey>`, `<address>`: a hex20 as `galeed/1` §1 spells it;
  `<termsHash>`: a hex32; any other spelling is misuse.

Executable name: `erl`. Commands:

## `erl check <listing.json>`

§2.5 over the file's bytes. Accepted:
`{"byCall":[{"candidate":<int>,"digest":"0x<64>"}...],"candidates":<int>,"ok":true,"termsHash":"0x<64>"}`,
`termsHash` being the listing's `terms.hash` and `byCall` the by-call
candidates of §2.5, each by its candidate index (§2.3's numbering, as
`E_LISTING_SIG`'s `index`) with its offer's digest of §2.4 under the
logic's domain, ascending by candidate, an empty array where there is
none. Rejected: the fault object.

## `erl bind <listing.json> <terms.md>`

§7.1. Output `{"verdict":"BOUND"}` or `{"verdict":"NAME"}` with `index`
for `BIND_JUDGES` and `BIND_GUILDS` and `inner` for `LISTING_INVALID` and
`TERMS_INVALID`.

## `erl link-check <link.json>`

§3.1 over the file's bytes. Accepted:
`{"chainId":<int>,"hash":"0x<64>","ok":true,"tx":"0x<64>"}`. Rejected:
the fault object.

## `erl encode <link.json>`

§3.2: §3.1 then the text. Accepted: `{"link":"eravon://open?e=...","ok":true}`.
Rejected: the fault object.

## `erl decode <link.txt>`

§3.2 over the file's bytes as the link text. Accepted:
`{"link":"0x<hex of the link bytes>","ok":true}`, lowercase digits.
Rejected: the fault object (`E_LINK_PREFIX`, `E_LINK_B64`, `E_LINK_DOC`
carrying the link fault).

## `erl resolve-link <link.json> <document.json>`

§3.3 over the link's bytes and the fetched document's bytes. Resolved:
`{"document":A,"verdict":"LISTING"}` with `A` the listing's accepted
answer as `check` renders it, or `{"document":A,"verdict":"SETTLE"}` with
`A` the settlement offer's accepted answer as `settle-check` renders it.
Otherwise `{"verdict":"NAME"}` with `inner` where the verdict carries a
fault (`LINK_INVALID`, `LINK_LISTING`, `LINK_SETTLE`).

## `erl receipt-check <receipt.json>`

§4.2. Accepted:
`{"candidate":<int>,"form":"secp256k1"|"call","ok":true,"openBy":<int>,"prev":"0x<40>"}`,
`call` where `sig` is the empty string. Rejected: the fault object.

## `erl receipt-encode <receipt.json>`

§4.3: §4.2 then the text. Accepted: `{"ok":true,"text":"eravon-receipt:..."}`.
Rejected: the fault object.

## `erl receipt-decode <text.txt>`

§4.3 over the file's bytes. Accepted:
`{"ok":true,"receipt":"0x<hex of the receipt bytes>"}`. Rejected: the
fault object (`E_RECEIPT_PREFIX`, `E_RECEIPT_B64`, `E_RECEIPT_DOC`
carrying the receipt fault).

## `erl bind-receipt <listing.json> <receipt.json>`

§4.4. Bound:
`{"caseId":..,"digest":..,"form":"secp256k1","instance":..,"salt":..,"verdict":"RECEIPT_BOUND"}`;
by call:
`{"caseId":..,"digest":..,"form":"call","instance":..,"salt":..,"verdict":"RECEIPT_BY_CALL"}`.
Otherwise `{"verdict":"NAME"}` with `inner` where the verdict carries a
fault (`LISTING_INVALID`, `RECEIPT_INVALID`, `RECEIPT_ORDER`,
`RECEIPT_SIG`).

## `erl pack-check <pack.json>`

§5.2. Accepted:
`{"attachments":<int>,"buyerKey":..,"caseId":..,"form":..,"instance":..,"ok":true}`,
`form` as `bind-receipt` renders it (`call` for a by-call receipt) and
`attachments` how many attachments the pack carries (0 where the member is
absent). Rejected: the fault object.

## `erl settle-check <settle.json>`

§8.2. Accepted:
`{"buyerBps":<int>,"expiry":<int>,"instance":"0x<40>","ok":true}`.
Rejected: the fault object.

## `erl bind-settle <pack.json> <settle.json>`

§8.3. Bound: `{"digest":"0x<64>","verdict":"SETTLE_BOUND"}`, `digest`
being the `Settle` action digest; by call:
`{"digest":"0x<64>","verdict":"SETTLE_BY_CALL"}`. Otherwise
`{"verdict":"NAME"}` with `inner` where the verdict carries a fault
(`PACK_INVALID`, `SETTLE_INVALID`, `SETTLE_SIG`).

## `erl place <records.json> <sellerKey> <termsHash>`

§6.2 over the record set and the pair. Output
`{"ok":true,"position":{"blockNumber":<int>,"logIndex":<int>,"source":"self"|"opened"|"revoked"}}`,
or `{"ok":true,"position":null}` where the pair is no document of the
record set.

## `erl pay-check <listing.json> <records.json>`

§6.4. Output `{"verdict":"PAYABLE"|"SELLER_KEY_RETIRED"|"UNPLACED"}`, or
`{"inner":F,"verdict":"LISTING_INVALID"}` with `F` the listing's fault.

## `erl history <records.json> <address>`

§6.5. Output
`{"attempts":[…],"cases":[…],"consents":[…],"documents":[…],"keys":[…],"ok":true}`:
`keys` the record's keys as hex20 in record order; each document
`{"cases":[…],"position":P,"postings":[…],"revoked":[…],"sellerKey":..,"source":"self"|"opened"|"revoked","standing":"stands"|"attempt","termsHash":..}`
with `P` written as `place` writes a position and `source` the same word,
`postings` the `posted` elements that draw the document, each with one more
member `standing`, and `revoked` its `revoked` elements; each case
`{"buyer":..,"caseId":..,"instance":..,"position":{"blockNumber":<int>,"logIndex":<int>},"records":[…]}`,
`instance` the `opened` element's `emitter` and `records` the elements of
§6.5's list; `consents` the `consented` elements; the top-level `cases`
written as a document's case; `attempts` the documents that are attempts,
written as in `documents`. Every element is written as it stands in
`<records.json>`, and every list in §6.5's order.
