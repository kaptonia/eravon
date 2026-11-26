# ERAVON peer binaries · for programs

What a program needs to call each binary ERAVON ships besides `eravon`: verbs, inputs, exits, refusals and answer keys, one item a line.
`<binary> schema` answers that binary's verbs, flags, exit codes and refusal names as one JSON value; where this page and that answer differ, the answer holds.
`eravon` itself is in `CLI-SCHEMA.md`.

## 1 · Binaries

- `eravon-app`: the desktop app; `window` opens it, the other verbs answer headless readings of the interface and lay out the macOS bundle.
- `era`: the standard (`eravon.standard/1`): ABI, deployment and action calldata, chain reads, pre-send gates, digests, transactions, company mode, proposals.
- `ert`: the terms (`eravon.terms/1`): terms documents, quotes, offers, orders, actions, signatures, fees.
- `erl`: the link (`eravon.link/1`): listings, links, receipts, case packs, settlement offers, their transports and bindings, record history.
- `erd`: the dossier (`eravon.dossier/1`): offline dossier checks.
- `erj`: the roster: guild resolution, candidate sets, pool candidates, panel hashes, the candidate gate.
- `erv`: the evidence (`courtevidence/1`): evidence sheets, file digests, recipients, folder listings.
- `erk`: recheck: recomputes a dossier's items from the chain.
- `erq`: candidates: the client's checks at the act and the pool form's checks over the live chain.
- `erx`: scan: case scans by key, order rebuilds, event history, judge ledgers, board posts, order numbers.
- `erc`: the chain (`eravon.chain/1`): endpoint tables, multi-endpoint reads, polling, gas facts, nonces, sending, receipts.
- `ers`: the store and vault: the data root, records, settings, bundles, the writer lock.
- `erz`: the ZIKARON bridge: read-only discovery of a ZIKARON machine directory and its disclosure kits.
- `eravon-literals-check`: checks that the product's literals do not collide with the galeed and ZIKARON families.
- `eravon-galeed-leg`: the galeed layer (`galeed/1`): canonical form and entry envelope checks.

## 2 · Common rules

1. stdout is one canonical JSON value (members in byte order, no whitespace), no trailing newline.
2. Success carries `ok: true` plus the verb's keys unless its line says otherwise; a refusal carries `ok: false`, `token` (the refusal name), `detail` (a sentence), a lower layer's name passed through unchanged.
3. Misuse (exit 2) writes zero bytes to stdout; the first stderr line is `<REASON> <subject>`: `E_ARGS` (a missing, repeated, unknown or malformed argument, a verb not in the book), `E_UNREADABLE` (a named file does not read as what it should), `E_KEY` (a private key that is not one).
4. The subject never repeats a typed word: the program name, a missing flag as `--name`, or a typed word by place and length (`#3 (12 bytes)`; the verb is `#1`); later stderr lines are for people.
5. A flag given twice is misuse unless its book type is `array`; verbs whose answer depends on time take it from `--now`.
6. Each secret flag (`--passcode`, `--new-passcode`, `--file-passcode`, `--again`, `--phrase`, `--key`) has a file form `--<flag>-file <path>`: owner-only, at most 4096 bytes of UTF-8, one trailing line end dropped; both forms together is misuse.
7. Exit codes of `eravon-app`, `era`, `erk`, `erq`, `erx`, `erc`, `ers`, `erz`, `eravon-literals-check`: `0` affirmed, `1` denied, `2` misuse, `3` partial, `4` unanswered.
8. A refusal exits by the class of `downToken`, else `token`: names in the book's `exitPartial` (`E_V_UNCONFIRMED`, `E_QUEUED`) exit 3, names in `exitUnanswered` (chain, store, vault, app, ZIKARON and network names) exit 4, all others 1; a non-empty `txs` makes a 1 a 3.
9. `ert`, `erl`, `erd`, `erj`, `erv`, `eravon-galeed-leg` (legs of frozen criteria) have two codes: every answer, a refusal included, exits 0 and its `ok` says which; misuse exits 2.
10. `ert`, `erl`, `erd`, `erj`, `erv` take positional arguments in the book's `args` order (a wrong count is `E_ARGS` naming the program); their refusals carry `ok: false`, `token` and, where known, `index`, `field`, `inner` (the carried lower refusal), no `detail`.
11. `schema` (every binary): `v`, `contract`, `exits`, `exitPartial` and `exitUnanswered` (five-code binaries), `secretFlags`, `trailingNewline`, `refusal`, `commonTokens` (names any verb can give), `verbs`, `register`, `ok`.
12. `register` rows: `name`, `args` (`name`, `type`, `required`), `tokens` (the verb's own refusal names), `carries` (closed refusal sets passed through), `opensWindow`.
13. An endpoint address that does not read is `E_CHAIN_USAGE`, `detail` starting `scheme`, `port`, `host` or `shape`, never the address; one with a path or query is said as scheme, host and port then `/…`; a proxy that does not read starts `shape` or `credentials`.
14. A read asks every endpoint and the answers must agree by method (the head the lowest, pending count and `latest` block the highest, a gas estimate the highest), else `E_CHAIN_DISAGREE` (`behind` for heights over 64 blocks apart, `not-yet` for a transaction still split after retries); `agreement` is `single` under two answers, `left` lists endpoints passed over.
15. A node error's `detail` starts `funds`, `nonce-used`, `pending`, `underpriced`, `gas-too-low`, `reverted` (`E_CHAIN_EXECUTION`) or `rate-limited`, `no-method`, `auth`, `wrong-chain`, `coded`, `other` (`E_CHAIN_ENDPOINT`); `E_CHAIN_TRANSPORT`'s starts `name`, `connect`, `handshake`, `certificate`, `late`, `overlong`, `stream`, `closed` or `thread`.
16. `erk`, `erq`, `erx`, `eravon-app` keep what this machine settled of the chain under an existing machine directory (`cli-read/`, `checked-blocks/`); deleting those folders clears it.

## 3 · `eravon-app`

No verb is misuse, except a macOS LaunchServices launch or a Windows start with no terminal, which open the window; headless verbs need no display.
- `form`: interface form constants as settled and as read back, frame readings; `floorOk`, `form`, `frames`, `installed`.
- `skin`: light and dark color tables, inks, tints, button role keys, fonts; `fonts`, `inks`, `keys`, `looks`, `tints`.
- `i18n`: key sets of the Chinese and English word tables and their self-checks; `--lang` adds that language's words; `byKind`, `checker`, `count`, `keysEn`, `keysZh`, `keysetChecker`, `onlyEn`, `onlyZh`, `wording`, `words`.
- `drive`: runs the shell headless from a step script; needs `--data`, `--script`; the interface state blocks, `first`, `steps`, `trace`, `tracePath`, `write`, `door`.
- `page-frames`: a drive script, then each `--page` painted at `--width` and `--look` (`here` the state as left); needs `--data`, `--script`, `--page`; `look`, `pages`, `width`.
- `trace`: reads back the trace file the last run with `ERAVON_TRACE` set left; needs `--data`; `bytes`, `path`, `steps`.
- `bundle`: lays out a macOS `.app` at `--at` from this executable (or `--from`) with `eravon` beside it, `--data` written in; needs `--at`; `app`, `bundleId`, `cli`, `executable`, `launch`, `plist`, `scheme`, `urlDelivery`.
- `window`: opens the window on `--data` (else the bundle's launch file; on Windows `%LOCALAPPDATA%\ERAVON`), `--lang`, `--url` an `eravon://` link; prints nothing.
- `drive` answer: `write` per kind (`chain`, `local`, `vault`) why it cannot write now (blank when it can) and `writer` (`writer`, `reader`, `other-machine`, `mark-unread`); `door` the waiting request's verb (`held`) and flag names (`flags`); a step's refusal is said in its page block (`said`, `offer`: `name`) while the run answers `ok: true`; the book's `drive` row lists every step (`steps`) and page (`pages`).
- Refusals: `E_APP_IO` 4 (a file or folder the verb needs does not read or write; `bundle` without `eravon` beside it); `E_APP_SCRIPT` 1 (`drive`, `page-frames`: the script does not read, or a step runs past the budget); `E_APP_NO_DATA` 1 (`window`: no data root given or remembered).

## 4 · `era`

Writes no files, sends nothing; reads are chain-id guarded; `--chain` and `--logic` default to the standard table's.
- `sol`: the protocol ABI surface; `actions`, `aggregate3`, `bookEvents`, `events`, `exported`, `guildOnly`, `init`, `judgeFace`, `judgeOnly`, `views`.
- `address`: instance address and digests of an order file; needs `--order`; `caseId`, `chainId`, `instance`, `logic`, `offerHash`, `orderDigest`, `sellerDigest`.
- `deploy-logic`: the deployer call for the logic contract; needs `--initcode`; `call`.
- `deploy-verifier`: the deployer call for the verifier; needs `--initcode`; `call`, `verifier`.
- `deploy-board`: the deployer call for the board; needs `--initcode`; `board`, `call`.
- `deploy-instance`: the deployer call for the case instance; needs `--order`; `call`, `instance`.
- `init`: `init` calldata, with `--quote`, `--sig-buyer`, `--sig-seller`, `--member`, `--amanah`; needs `--order`; `call`, `instance`.
- `expire`: the expiry-branch `init` call, refused while `--now` is not past the offer expiry; needs `--now`, `--order`; `call`, `expiry`, `instance`.
- `merged`: the deployments a case needs as one call (`--verifier-initcode`, `--board-initcode` first), `init` beside it; from `--order`, or `--terms` with `--candidate`, `--buyer-key`, `--buyer-payout`, `--open-by`; `call`, `init`, `instance`, `steps`.
- `cancel`: the cancel call for an order (given as in `merged`) with `--sig-buyer`; `call`, `instance`.
- `revoke`: the call revoking a seller's terms; needs `--seller`, `--terms-hash`; `call`.
- `accept-funding`: the seller's accept-funding call; needs `--seller`; `call`.
- `post`: a board post call, `--board` or the standard board; needs `--key`, `--document`; `call`, `hash`.
- `action`: a party action call, `--window-end` with `--now` checking the window; needs `--instance`, `--do` (`claim-delivery`, `confirm`, `dispute`, `refund`, `settle`, `commit`, `advance`, `redraw`, `sweep`); `call`, `verb`.
- `bysig`: a signature-relayed action call; needs `--instance`, `--do` (`claim-delivery`, `confirm`, `dispute`, `refund`), `--sig`; `call`, `verb`.
- `read`: a guarded read; needs `--endpoint`, `--what` (`code`, `verifier`, `board`, `status`, `revoked`, `funds`, `check`, `fee`, `seat`, `account`, `hash-offer`, `panel-book`, `book-latest`, `price`, `balance`, `consented`, `settle-gate`); `agreement` and that read's keys.
- `precheck`: one pre-send gate; needs `--what` (`sig`, `consent`, `sender`, `window`, `price`, `verifier`, `deposit`, `funds`, `allowance`, `late`); `gate` and that gate's keys.
- `digest`: an action's EIP-712 digest under the instance domain (needs `--instance`), or with `--do offer`, `--terms`, `--candidate` the offer digest; needs `--do`; `digest`, `message`.
- `decode-log`: decodes an event log; needs `--data`; `args`, `event`.
- `decode-init`: decodes `init` calldata; needs `--data`; `amount`, `buyerKey`, `buyerPayout`, `guild`, `judge`, `members`, `offerHash`, `quoteBytes`, `sellerKey`, `termsHash`, `token`.
- `tx`: an EIP-1559 transaction, with `--sig` also the raw one; needs `--chain`, `--nonce`, `--priority-fee`, `--max-fee`, `--gas-limit`, `--to`; `digest`, `payload`, `raw`, `txHash`, `type`.
- `approve`: ERC-20 `approve` calldata; needs `--token`, `--spender`; amount as `--amount` or `--amount-text`; `call`.
- `permit`: the EIP-2612 permit digest, with `--sig` the call; needs `--token`, `--owner`, `--spender`, `--deadline`, `--domain`, `--nonce`; `call`, `digest`.
- `company`: whether company mode opens for `--company`; needs `--endpoint`, `--company`; `identity`, `open`, `shut`, `views`.
- `time-check`: the company check, then windows and deadlines against the company calendar; needs `--endpoint`, `--company`, `--now`; `identity`, `open`, `shut`, `time`, `views`.
- `proposal-read`: reads a proposal file; needs `--file`; `calls`, `contribution`, `digest`, `documents`, `structural`.
- `proposal-pay`: a proposal call for the company wallet's `pay`; needs `--wallet`, `--token`, `--to`; `call`, `documents`.
- `proposal-consent`: a proposal call for `consent(party, digest)`; needs `--logic`, `--party`, `--digest`, `--original`, `--kind`; `call`, `documents`.
- Refusals (may add `gate`): `E_A1_FORM` 1 (an input does not read as its form); `E_A1_ABI` 1 (ABI encoding or decoding failed); `E_A1_PRECHECK` 1 (`expire`, `merged`, `action`, `precheck`: a gate says the chain would reject it); `E_A1_CODEHASH` 1 (`read`: the code read is not the code the standard expects).

## 5 · `ert`

Positional: `terms-file`, `candidate`, `buyer-key`, `buyer-payout`, `open-by`, `prev`, `privkey`; `action-name` is `ClaimDelivery <manifest>`, `Confirm`, `Dispute`, `Refund`, `Cancel` or `Settle <buyerBps> <expiry>`.
- `check`: a terms document; `terms-file`; `termsHash`.
- `quote`: decodes a quote; `quote-hex`; `quote`, `quoteHash`.
- `offer`: one candidate's offer; `terms-file candidate`; `digest`, `offer`, `structHash`.
- `order`: the order derived from it; `terms-file candidate buyer-key buyer-payout open-by prev`; `caseId`, `digest`, `instance`, `offerDigest`, `salt`.
- `action`: an action's digest; the `order` arguments, then `action-name`; `digest`, `instance`, `structHash`.
- `sign-offer`: signs the offer; `privkey terms-file candidate`; `digest`, `sig`, `signer`.
- `sign-order`: signs the order as buyer; `privkey terms-file candidate buyer-payout open-by prev`; `buyerKey`, `digest`, `instance`, `sig`.
- `sign-action`: signs an action; `privkey` then the `action` arguments; `digest`, `instance`, `sig`, `signer`.
- `verify-offer`: checks the seller's signature; `terms-file candidate seller-sig`; `form`, `signer?`.
- `verify-order`: checks the buyer's signature; the `order` arguments, then `sig-file`; `form`, `instance`.
- `verify-action`: checks an action signature; the `action` arguments, then `party` (`buyer` or `seller`), `sig-file`; `form`.
- `fee`: the litigation fee for an amount; `quote-hex amount`; `lf` (`null` when unpriced), `premium`, `priced`.
- Refusals (exit 0): `E_QUOTE_LEN`, `E_QUOTE_FIELD`, `E_QUOTE_FEED` (`quote`, `fee`: the quote's length, a field, its feed); `E_FEE_OVERFLOW` (`fee`); `E_ACTION_PARTY`, `E_SIG_FORM` (`verify-action`: not that party's action; not a signature form).

## 6 · `erl`

Positional files, except `seller-key`, `terms-hash`, `address`. A `verdict` answer has no `ok`: the verdict name says the result.
- `check`: a listing; `listing-file`; `byCall` (each `candidate`, `digest`), `candidates`, `termsHash`.
- `bind`: a listing against its terms; `listing-file terms-file`; `verdict` (`BOUND` or a refusal verdict), `index?`, `inner?`.
- `link-check`: a link document; `link-file`; `chainId`, `hash`, `tx`.
- `encode`: a link to its text; `link-file`; `link`.
- `decode`: link text to bytes; `link-text-file`; `link`.
- `resolve-link`: a link against the document it names; `link-file document-file`; `verdict` (`LISTING` or `SETTLE`), `document`.
- `receipt-check`: a receipt; `receipt-file`; `candidate`, `form`, `openBy`, `prev`.
- `receipt-encode`: a receipt to its text; `receipt-file`; `text`.
- `receipt-decode`: receipt text to bytes; `receipt-text-file`; `receipt`.
- `bind-receipt`: a receipt against its listing; `listing-file receipt-file`; `verdict`, `caseId`, `digest`, `form`, `instance`, `salt`.
- `pack-check`: a case pack; `pack-file`; `attachments`, `buyerKey`, `caseId`, `form`, `instance`.
- `settle-check`: a settlement offer; `settle-file`; `buyerBps`, `expiry`, `instance`.
- `bind-settle`: a settlement offer against its pack; `pack-file settle-file`; `verdict`, `digest`.
- `place`: where the seller placed the terms in its records; `records-file seller-key terms-hash`; `position` (`null` when not placed).
- `pay-check`: whether a listing may be paid; `listing-file records-file`; `verdict` (`PAYABLE`, `SELLER_KEY_RETIRED`, `UNPLACED`, `LISTING_INVALID`).
- `history`: an address's record history; `records-file address`; `attempts`, `cases`, `consents`, `documents`, `keys`.
- Refusals (exit 0): `E_LINK_DOC` (`decode`), `E_RECEIPT_DOC` (`receipt-decode`): the text is not a link or receipt document.

## 7 · `erd`

- `check`: a dossier folder; `dossier-dir`; `attachments`, `buyerConsent`, `candidate`, `caseId`, `entryId` (`null` when none), `instance`, `sellerConsent`, `termsHash`; refusals by the dossier set (`E_DOSSIER_*`, `E_TRANSACTION`), exit 0.

## 8 · `erj`

Lists (`pointers`, `allow`, `parties`, `members`, `hashes`) are comma-separated, empty for none; refusals carry `token` only, exit 0.
- `guild`: resolves a guild from roster logs; `logs-file guild-addr`; `card`, `guild`, `issued`, `judges`.
- `candidate-set`: the candidate set; `logs-file pointers allow parties`; `candidates`, `count`.
- `pool`: a pool candidate; `logs-file book-file pointer guild-terms-file token parties now`; `amanahHash`, `expires`, `guild`, `members`, `panelHash`, `quote`, `root`, `router`, `seq`.
- `panel-hash`: a panel hash; `members hashes`; `panelHash`.
- `gate`: the candidate gate over a gate file; `gate-file`; `candidates` (each `candidate`, `note`, `state`), `choosable`.
- Refusals: `pool` by `E_POOL_POINTER`, `E_POOL_ROSTER`, `E_POOL_ROSTER_EMPTY`, `E_POOL_NO_GUILDTERMS`, `E_POOL_TERMS_DIGEST`, `E_POOL_NO_QUOTE`, `E_POOL_QUOTE_FORM`, `E_POOL_NO_PANEL`, `E_POOL_PANEL_HASH`, `E_POOL_PANEL_EXPIRED`, `E_POOL_PANEL_WORD`, `E_POOL_PANEL_EMPTY`; `guild` by the roster set.

## 9 · `erv`

- `check`: an evidence sheet; `sheet-file`; `manifest`; refused by the `courtevidence/1` sheet set, `index` naming the file (exit 0).
- `digest`: any file; `any-file`; `bytes`, `sha256` (no `ok`).
- `recipient-check`: a recipient's form; `recipient`; `ok` only; refused `E_RECIPIENT`.
- `listing`: a folder listed as a sheet; `listing-dir subject-file party media-type`; `files`, `manifest`, `sheet`; refused as `check`.

## 10 · `erk`

- `check`: a dossier recomputed from the chain, one row per item; needs `--dossier` and `--endpoint` or `--table`; option `--registry`; `caseId`, `counts`, `instance`, `items` (names in the book's `closed.item`, states `pass`, `mismatch`, `unread`), `registries`.
- Refusals: `E_R1_DOSSIER` 1 (the dossier checker refused, its name inside), `E_R1_FORM` 1 (an input does not read as its form).

## 11 · `erq`

- `act`: every check of the act on an order; needs `--endpoint`, `--chain`, `--logic`, `--order`, `--registry`; `deposit`, `head`, `lf`, `premium`, `quote`, `seat` (`track` `named` with `judge`, `label`, `narrow`, or `guild` with `guild`, `panelHash`, `pool`, `seq`).
- `pool`: the pool form's checks on a guild pointer; needs `--endpoint`, `--chain`, `--logic`, `--registry`, `--pointer`, `--guild-terms-file`, `--token`; option `--party`; `amanahHash`, `expires`, `guild`, `head`, `members`, `panelHash`, `quote`, `root`, `router`, `seq`.
- Refusals add `gate` (`form`, `chain`, `registry`, `pool` or the act's gate): `E_ACT_FORM` 1 (`act`: the order file does not read); the act's and the pool form's `E_ACT_*`, `E_POOL_*`, `E_QUOTE_FORM` 1.

## 12 · `erx`

Reads the chain only; every verb needs `--endpoint` (`rebuild`: `--endpoint` or `--table`).
- `opened`: `Opened` cases by a party's key; needs `--key`, `--side`; `agreement`, `cases` (each `block`, `buyer`, `caseId`, `instance`, `seller`, `tx`), `count`, `foreign`.
- `rebuild`: an order rebuilt from its opening transaction; needs `--tx`, `--case-id`; `amount`, `buyer`, `caseId`, `logic`, `members`, `past`, `path`, `receipt`, `seller`, `state`, `termsHash`, `token`, `why`.
- `history`: an instance's events in chain order; needs `--instance`; `count`, `moments` (each `block`, `event`, `logIndex`, `tx`).
- `ledger`: a judge ledger read and audited; needs `--root`, `--basis-file`; `discarded`, `entries`, `head`, `label`, `lineage`, `narrow`, `notAnchors`, `records`, `root`, `unchecked`.
- `board-key`: board posts under a key; needs `--key`; `count`, `posts` (each `block`, `document`, `hash`, `key`, `logIndex`, `poster`, `tx`).
- `board-tx`: one transaction's board posts matching `--hash`; needs `--tx`, `--hash`; `count`, `posts`.
- `order-no`: a party's cases numbered by month; needs `--key`, `--side`; `count`, `orders` (each `caseId`, `month`, `no`, `seq`).
- Refusals: `E_A3_FORM` 1 (an input does not read as its form); `E_A3_REBUILD` 1 (`rebuild`: the rebuilt terms give another case id than the chain's).

## 13 · `erc`

Endpoints from `--endpoint` or `--table`; reads go to every endpoint, `send` and `receipt` to one; only `send` and `poll --once --state` write; refusals are the `eravon.chain/1` set's (`send`, `receipt` without `E_CHAIN_DISAGREE`), with `class` `transport`, `endpoint` or `execution`.
- `window`: the window rule offline (an action while `now <= end`, a stamp once `now > end`); needs `--end`, `--now`; `action`, `stamp`.
- `endpoints-check`: the endpoint list or table, as accepted; `count`, `endpoints`.
- `chain-id`: the chain id, refused when not `--chain`; `agreement`, `chainId`, `endpoints`.
- `balance`: native balance; needs `--address`; `agreement`, `balance`, `endpoints`.
- `call`: an `eth_call`; needs `--to`, `--data`; `agreement`, `data`, `endpoints`.
- `trace`: the non-reverted call to `--to` with `--selector` in a transaction; needs `--tx`, `--to`, `--selector`; `agreement`, `endpoints`, `from`, `input`, `state`, `support`, `why`.
- `logs`: an address's logs over a range; needs `--address`; `agreement`, `count`, `endpoints`.
- `clock`: latest block number and time; `agreement`, `block`, `endpoints`, `timestamp`.
- `poll`: watches a balance (and `--logs`) over `--blocks`; `--once` with `--state` takes one tick and writes the baseline; needs `--address`; `blocks`, `changed`, `changedAtBlock`, `changedFrom`, `changedTo`, `idleTicks`, `logs`, `outcomes`, `state`, `stateBlock`.
- `gas`: gas estimate, gas price, recent base fees; needs `--to`; `agreement`, `baseFees`, `endpoints`, `estimatedGas`, `gasPrice`.
- `nonce`: transaction count, mined or pending (`--at`); needs `--address`; `address`, `agreement`, `at`, `endpoint`, `endpoints`, `nonce`.
- `send`: broadcasts a signed raw transaction; needs `--raw`; `endpoint`, `txHash` (the endpoint's own hash).
- `receipt`: a receipt once (`--wait 0`) or polled until `--wait` runs out; needs `--tx`; `block`, `contract`, `found`, `gasUsed`, `status`, `txHash`.

## 14 · `ers`

Every verb needs `--data`; `--passcode` unlocks the vault first (counted); writing verbs also need `--writer`, `--now` and take the writer lock.
- `init`: creates the data root when absent; `data`, `identities`.
- `identity-init`: one identity's directories; needs `--address`; `address`, `boxes`.
- `put`: writes a record, `--field key=value`, `--part`; needs `--address`, `--id`, `--kind`; `record`.
- `delete`: removes a record of that kind; needs `--address`, `--kind`, `--id`; `deleted`, `kind`.
- `get`: one record; needs `--address`, `--box`, `--id`; `fields`, `logical`, `partNames`, `record`.
- `part`: one part of a record, `--out` writing its bytes; needs `--address`, `--box`, `--id`, `--name`; `bytes`, `name`.
- `open-file`: any local data file by logical coordinate or disk path, `--out` writing its bytes; needs `--path`; `bytes`, `disk`, `kind`, `logical`.
- `list`: one box's records, newest first; needs `--address`, `--box`; `box`, `count`, `rows`.
- `identities`: the identity addresses; `count`, `identities`.
- `name`: the on-disk name for a kind and logical name; needs `--kind`, `--logical`; `kind`, `token`.
- `settings-put`: global settings (`--active-seller`, `--active-buyer`, `--language`, `--seller-ready`, `--buyer-ready`, `--past-cases`); `lastWriter`.
- `settings-get`: global settings; `activeBuyer`, `activeSeller`, `buyerReady`, `language`, `lastWriter`, `sellerReady`.
- `identity-settings-put`: `--payout`, `--chain` (a known network's id, applied first), `--endpoint` (that network's nodes); needs `--address`; `address`.
- `identity-settings-get`: one identity's settings; needs `--address`; `company`, `companyMode`, `companyPast`, `endpoints` (the home network's JSON-RPC nodes), `payout`, `timeCheck`.
- `candidates-put`: replaces the candidate table with `--entry kind:address:label:time`; needs `--address`; `count`.
- `candidates-get`: the candidate table, newest first; needs `--address`; `count`, `entries`.
- `export`: the whole data root to one bundle; needs `--out`; `bundleBytes`, `dirs`, `files`.
- `import`: a bundle into an empty data root, marked a read-only mirror; needs `--bundle`; `files`.
- `hold`: takes the writer lock until stdin ends; `held`, `released`, `writer`.
- `lock-status`: whether another instance holds the lock now, writing nothing; `held`, `pid`, `since`, `writer` (the last three for people only).
- `take-over`: marks a data root whose writer mark names another machine, or does not read, as this machine's; `mode`.
- Refusals (any verb): `E_STORE_IO` 4 (a read, write or directory sync failed); `E_STORE_NO_KEY` 1 (the vault is locked); `E_STORE_SEALED` 1 (a sealed file does not read; `detail` starts `not-sealed`, `truncated`, `other-kind`, `newer-version`, `swapped`, `unopenable`).
- Refusals (writing verbs): `E_STORE_LOCKED` 4 (another instance here holds the lock); `E_STORE_OTHER_MACHINE` 1 (the writer mark names another machine or does not read; `take-over` writes here); `E_STORE_MACHINE` 4 (this machine's mark cannot be had); `E_STORE_UNREAD` 1 (a file that does not read is never written over).
- Refusals by the verb's book row, all 1: `E_STORE_MISSING`, `E_STORE_FORM`, `E_STORE_VERSION`, `E_STORE_KIND`, `E_STORE_NAME`, `E_STORE_TIME`, `E_STORE_ROOT`, `E_STORE_CAP`, `E_STORE_ARCHIVE`, `E_STORE_EXISTS`.

## 15 · `erz`

Read-only: writes nothing, starts no process; ZIKARON not there is an answer with `absent` set.
- `status`: whether a ZIKARON machine directory is found, where, through which layer, how many kits; needs `--home`; `absent`, `at`, `connected`, `count`, `detail`, `layer`.
- `kits`: the disclosure kits; needs `--home`; `absent`, `at`, `connected`, `detail`, `kits`, `layer`.
- `pack`: a kit plus plain files (`--file`, `--media-type`, `--party`, `--subject`, `--note`) as manifest items; needs `--home`, `--kit`; `at`, `files`, `fromKit`, `kitId`, `link`, `manifest`, `manifestBytes`, `prefix`.
- Refusals: `E_ZK_IO` 4 (a path that is there does not read); `E_ZK_INDEX_FORM`, `E_ZK_INDEX_SHAPE` 1 (the kit index's form or shape); `pack`: `E_ZK_NO_SUCH_KIT` 1 (no kit of that id), `E_ZK_PACK` 1 (sealed to one recipient, does not unpack, or no manifest items).

## 16 · `eravon-literals-check`

- (no verb): `--galeed <path> --zikaron <path>`; `v`, `ok`, `families` (each `name`, `source`, `count`), `collisions` (each `literal`, `families`), `literals`, `company`, `network`, `constants`; exit 1 with the same answer and `ok: false` when any collision.

## 17 · `eravon-galeed-leg`

- `selftest`: the leg's own checks; `v`, `op`, `ok`, `checked`; a failed leg answers `ok: false`, `token` (`E_LEG_CANON`, `E_LEG_KECCAK`, `E_LEG_SHA256`, `E_LEG_SIGN`, `E_LEG_ENVELOPE`, `E_LEG_CARRIED`), `detail`.
- `canon`: canonical form of a case file; needs `--case`; `v`, `op`, `bytes`, `token` (`ok` or the refusal name), `canonSha256` when `ok`; no `ok` key.
- `entry`: entry envelope of a case file; needs `--case`; `v`, `op`, `bytes`, `token` (`ok` or the refusal name), `entryId` when `ok`; no `ok` key.
