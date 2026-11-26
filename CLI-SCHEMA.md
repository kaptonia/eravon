# eravon CLI · for programs

What a program needs to call `eravon`: verbs, flags, exit codes, refusal reasons and answer keys, one item a line.
`eravon contract` answers the verbs, each verb's flags and must-give cells, the exit codes and every refusal
reason as one JSON value, read from the same tables the command line runs by; where this page and that answer
differ, the answer holds. The product's other binaries are in `CLI-SCHEMA-PEERS.md`.

## 1 · General rules

1. stdout is one canonical JSON value (no whitespace, members in byte order), no trailing newline, one value per run; `mcp` aside.
2. Misuse (exit 2) writes zero bytes to stdout; its first stderr line is `<REASON> <subject>`, the subject running to the line's end.
3. Misuse reasons: `E_ARGS` (a flag missing, repeated, unknown or malformed, a verb not known, two forms of one value), `E_UNREADABLE` (a named file does not read as it should), `E_KEY` (a private key that is not one).
4. A misuse subject names a flag of the table as `--name`; any other word (a value, a path, an unknown flag or verb) by its place and length, `#3 (12 bytes)` (the verb is `#1`), a word read from a file by its length alone, `(12 bytes)`.
5. Lines for people go to stderr, in Chinese when the first set of `LC_ALL`, `LC_MESSAGES`, `LANG`, `LANGUAGE` starts `zh`, English otherwise; only the first misuse line is fixed.
6. A refusal a lower layer gave is `E_V_DOWN`, with that layer in `layer` and its own name, unchanged, in `downToken`.
7. Deadlines and windows are judged by `--now` (Unix seconds) and by chain time.
8. Secrets (`--passcode`, `--new-passcode`, `--file-passcode`, `--again`, `--phrase`, `--key`) each have a file form `--<flag>-file`: readable by its owner only, at most 4096 bytes of UTF-8, one trailing line end dropped; giving both is misuse.
9. An endpoint address is never repeated: one that does not read is named by its place in the table and its length; one that reads is said by scheme, host and port only.
10. One value per flag: a flag given twice is misuse unless it is repeatable.
11. Every flag is a long flag taking a value (`true` and `false` are values); each verb's flag list is closed; a flag outside it is misuse.
12. `mcp` speaks the Model Context Protocol on stdin and stdout, one canonical JSON message a line, for as long as its input stays open; it takes `--rest-keys` and `--passcode-file` only.

## 2 · Exit codes

- `0` affirmed: answered yes (`ok: true`); a fact that is "no" (`matches: false`, `found: false`) is still an answer.
- `1` denied: answered no: the bytes, the law or the local state said no.
- `2` misuse: malformed arguments, an unreadable path, a flag the verb does not take; stdout empty.
- `3` partial: answered, neither yes nor no: the action went part way and whether it took is unknown.
- `4` unanswered: could not answer: out of reach, disagreeing, unreadable, locked by another.
- A refusal exits by `downToken` when it passes one through, else by `reason`; through the desktop, by the name it answers.
- A refusal carrying `txs` already sent exits 3, unless its name exits 4.
- Exit 3: `E_V_UNCONFIRMED`, `E_QUEUED`. Every name not listed for 3 or 4 exits 1.
- Exit 4: `E_CHAIN_TRANSPORT`, `E_CHAIN_ENDPOINT`, `E_CHAIN_DISAGREE`, `E_CHAIN_WRONG`, `E_CHAIN_ECHO`, `E_CHAIN_OVER_ASKS`, `E_CHAIN_UNREAD`, `E_STORE_IO`, `E_STORE_LOCKED`, `E_STORE_MACHINE`, `E_VAULT_IO`, `E_V_IO`, `E_APP_IO`, `E_ZK_IO`, `E_QUOTE_FEED`, `E_UNREACHABLE`, `E_TX_NOT_YET`, `E_DESKTOP`, `E_OUTCOME_LOST`, `E_NO_ENDPOINT`, `E_DISAGREE`, `E_SCAN_REFUSED`, `E_CHAIN_SHAPE`, `E_NODE_TLS`, `E_PROXY_UNREACHABLE`, `E_PROXY_CREDENTIALS_MISSING`, `E_PROXY_CREDENTIALS_REFUSED`, `E_NODE_TIMEOUT`, `E_ANSWER_TOO_LONG`, `E_ANSWER_NOT_JSON`, `E_RATE_LIMITED`, `E_METHOD_MISSING`, `E_NODE_AUTH`, `E_SERVICE_UNSERVED`, `E_SERVICE_KEY_MISSING`, `E_WRONG_CHAIN`, `E_NODE_REFUSED`.

## 3 · Answer shapes

- Affirmed: `{"ok":true, <the verb's keys>}`, exit 0.
- Refused: `{"ok":false,"reason":"E_…","detail":"…", <context members>}`; the exit code (1, 3, 4) tells denied, partial and unanswered apart.
- Context members a refusal can carry: `layer`, `downToken`, `rule`, `gate`, `txHash`, `txs`, `nonce`, `door`, `owner`, `count`, `tx`.
- Every seller and buyer verb can refuse `E_V_IO`, `E_V_MISSING`, `E_V_EXISTS`, `E_V_STATE`, `E_V_ARG`, `E_V_DOWN`, `E_V_NOT_YET`; a verb the door takes also `E_DESKTOP` and the desktop's own reasons; one that needs a secret, `E_ON_DESKTOP`.
- Through the desktop (`--desktop`): the desktop's answer is the verb's own answer, the same object; the command line exits by it.

## 4 · Refusal reasons

Each line: reason, exit, when. Run directly, a lower layer's name (the lines from `E_APP_FILE_MISSING` on) arrives as the `downToken` of `E_V_DOWN`; through the desktop it is the `reason`; the exit is the same either way.

- `E_ARGS` 2: malformed arguments: a flag missing, repeated, unknown or malformed, a verb not in the table, two forms of one value given together.
- `E_UNREADABLE` 2: a file named does not read, or does not read as what it should be.
- `E_KEY` 2: a private key that is not a scalar in [1, n−1].
- `E_UNREACHABLE` 4: no endpoint answered, the send failed or the wait ran out; a signed broadcast no endpoint took (`tx`); run again, it asks by hash first.
- `E_TX_STATUS` 1: included in a block with status other than 1 (`tx`, `state`).
- `E_TX_NOT_YET` 4: sent, and the wait ran out before inclusion; fate unknown (`tx`, `count` seconds waited); run again, it asks by that hash first.
- `E_TX_VOID` 1: what was sent will never be included: its nonce went to another transaction; nothing new sent (`tx`).
- `E_GAS_REFUSED` 1: the node refused the gas estimate for the call itself, or estimated it past 200,000; nothing broadcast.
- `E_DESKTOP` 4: `--desktop` got no answer; `detail` one of `NOT_OPEN`, `CLOSED`, `PATH_TOO_LONG`, `NOT_YOU`, `BROKEN`, `NO_MACHINE`, `ALONE`.
- `E_ON_DESKTOP` 1: `--desktop`: the action needs a secret the open vault does not stand in for; done on the desktop only; nothing done.
- `E_QUEUED` 3: `--desktop`: the desktop's command line sending is queue only, or the `--wait` seconds passed; nothing signed or sent, the request waits (`count`).
- `E_DOOR_UNREAD` 1: `--desktop`: the desktop could not read the request (its form, its size, a verb its door does not take) (`token`, `detail`).
- `E_IN_FLIGHT` 1: `--desktop`: the desktop is already doing what was asked; nothing new started (`token`, `detail`).
- `E_OUTCOME_LOST` 4: `--desktop`: the desktop began it and lost the outcome before answering; whether it was done is not told (`token`, `detail`).
- `E_REKEYING` 1: `--desktop`: the desktop is changing its vault's key; nothing written or sent until it ends (`token`, `detail`).
- `E_GOODS_QR_OVER` 1: `seller-product-qr`: the payment link is past the QR byte budget; nothing written; the `downToken` of `E_V_DOWN`.
- `E_GOODS_QR_SELFCHECK` 1: `seller-product-qr`: the code drawn does not read back to the link; nothing written; the `downToken` of `E_V_DOWN`.
- `E_V_ARG` 1: a flag value malformed in a way only the verb can judge.
- `E_V_DOWN` by `downToken`: a lower layer refused (`layer`, `downToken` its own name, `detail`; `rule` for a passcode rule).
- `E_V_EXISTS` 1: what the verb would create already exists.
- `E_V_IO` 4: the disk could not be read or written (interface, path, system's words in `detail`).
- `E_V_MISSING` 1: the record named does not exist.
- `E_V_NOT_YET` 1: this door is not available here (`door`, `owner`).
- `E_V_PRECHECK` 1: a local precheck stopped the call before anything was signed (`gate`).
- `E_V_REVERTED` 1: sent, and the chain reverted it (`txHash`).
- `E_V_SENT_VOID` 1: an earlier send from this machine can never be included (`txHash`, `nonce`); nothing sent; marked void.
- `E_V_STATE` 1: the local state does not allow this step (no active key, the seat not ready, the vault locked, a table full).
- `E_V_UNCONFIRMED` 3: sent, and whether the chain took it cannot be told yet; not to be sent again before it is checked (`txHash`; `txs` for a series).
- `E_APP_FILE_MISSING` 1: a file named is not on disk.
- `E_APP_DENIED` 1: no permission to read or write that place; through the desktop, also a queued request the person declined.
- `E_APP_TRACE_FULL` 1: the trace file is full; no more marks recorded.
- `E_CLI_PATH_TAKEN` 1: something else stands where the command line goes; left as it is.
- `E_CLI_PATH_CANCELLED` 1: the system's administrator dialog was cancelled; nothing changed.
- `E_CLI_PATH_NOT_ALLOWED` 1: the system did not allow changing where the command line goes; nothing changed.
- `E_CLI_PATH_UNSUPPORTED` 1: an app opened from its disk image cannot be put on the terminal's path.
- `E_APP_CLI_ANCHOR_GREY` 1: the desktop's command line sending choice is greyed out (the command line does not write through the desktop); not changed.
- `E_CANDS_LABEL` 1: a candidate given with no label.
- `E_CANDS_ADDRESS` 1: a judge address that is not `0x` and forty hex digits.
- `E_CANDS_POINTER` 1: a guild pointer that does not read.
- `E_CANDS_CHAIN` 1: the guild is on another chain.
- `E_CANDS_EXISTS` 1: already on the candidate table.
- `E_CANDS_MISSING` 1: not on the candidate table.
- `E_CANDS_KIND` 1: a candidate row that is neither a judge nor a guild row.
- `E_CHAIN_TRANSPORT` 4: a node not reached; `detail` starts `name`, `connect`, `handshake`, `certificate`, `late`, `overlong`, `stream`, `closed` or `thread`.
- `E_CHAIN_ENDPOINT` 4: a node answered wrongly (an error not about the call, an HTTP status, an answer that does not read).
- `E_CHAIN_EXECUTION` 1: the chain refused the call itself (the node's words in `detail`).
- `E_CHAIN_DISAGREE` 4: the nodes' answers differ (`detail` starts `behind` or `not-yet` where so); never a majority.
- `E_CHAIN_WRONG` 4: the nodes answer a chain other than the expected one.
- `E_CHAIN_USAGE` 1: an endpoint, Base URL or proxy that does not read; `detail` starts `scheme`, `port`, `host`, `shape` or `credentials`.
- `E_CHAIN_UNREAD` 4: the reading needed has not been read yet.
- `E_CHAIN_ECHO` 4: a node took a broadcast and answered a transaction hash other than the signed bytes'.
- `E_V_ON_DESKTOP` 1: the action is done on the desktop only.
- `E_V_DESKTOP` 1: the desktop's door gave no answer.
- `E_PAYWAYS_CHANNEL` 1: a payment way not known.
- `E_PAYWAYS_ID_FORM` 1: a payment way identifier that is malformed.
- `E_PAYWAYS_MIRROR_SCHEME` 1: a mirror address not on `https`.
- `E_PAYWAYS_MIRROR_FORM` 1: a mirror address that is malformed.
- `E_PAYWAYS_MIRROR_DUP` 1: a mirror listed twice.
- `E_PAYWAYS_TEST_EMPTY` 1: no identifier to test.
- `E_PAYWAYS_PAYLOAD` 1: an issued link fails its check.
- `E_PAYWAYS_RETURN` 1: a return address that is not `http` or `https`.
- `E_R1_DOSSIER` 1: the dossier fails its check.
- `E_R1_FORM` 1: the chain's answer to a recheck is malformed.
- `E_A3_FORM` 1: the scan read something malformed.
- `E_A3_REBUILD` 1: the rebuilt case id does not match the order.
- `E_A1_FORM` 1: a value for the standard's calls is not in its form.
- `E_A1_ABI` 1: the contract's answer does not read.
- `E_A1_CODEHASH` 1: the contract is not the standard one.
- `E_A1_PRECHECK` 1: the standard's local precheck says the call will be refused.
- `E_STORE_ROOT` 1: the data folder cannot be used.
- `E_STORE_NAME` 1: a name in the store is malformed.
- `E_STORE_KIND` 1: a record of the wrong kind.
- `E_STORE_VERSION` 1: a file's format version is not known.
- `E_STORE_FORM` 1: a file is broken, or under the product's earlier envelope head.
- `E_STORE_MISSING` 1: no such entry in the store.
- `E_STORE_EXISTS` 1: the target already holds data.
- `E_STORE_IO` 4: a disk read, write or directory sync failed (interface, path, system's words in `detail`); a write not taken as done.
- `E_STORE_LOCKED` 4: another instance on this machine holds the data root, the desktop among them.
- `E_STORE_ARCHIVE` 1: a data bundle is broken.
- `E_STORE_MIRROR` 1: a read-only copy; written on the original machine only.
- `E_STORE_TIME` 1: a time out of range.
- `E_STORE_CAP` 1: a table is full.
- `E_STORE_NO_KEY` 1: sealed local data and its key not at hand.
- `E_STORE_SEALED` 1: a sealed local file does not read; `detail` starts `not-sealed`, `truncated`, `other-kind`, `newer-version`, `swapped` or `unopenable`.
- `E_STORE_UNREAD` 1: a file there does not read, so it is not written over.
- `E_STORE_STALE` 1: things changed meanwhile, so it was not done.
- `E_STORE_OTHER_MACHINE` 1: the data root's writer mark names another machine or does not read; nothing written.
- `E_STORE_MACHINE` 4: this machine's mark is not to be had.
- `E_VAULT_RULE` 1: a new passcode breaks a rule (`rule`: the rule's word).
- `E_VAULT_PASSCODE` 1: a wrong passcode.
- `E_VAULT_DEAD` 1: locked out after five wrong passcodes.
- `E_VAULT_LOCKED` 1: the vault is locked.
- `E_VAULT_MNEMONIC` 1: the recovery words are not right.
- `E_VAULT_SEALED` 1: a key file that does not open: wrong passphrase or a changed file.
- `E_VAULT_FORM` 1: the key store file has a problem.
- `E_VAULT_KDF_BOUND` 1: a vault asking more than the limits allow.
- `E_VAULT_PATH` 1: a role not in the derivation table.
- `E_VAULT_IO` 4: a vault file write failed.
- `E_VAULT_KEYFILE_BROKEN` 1: a key file whose stated address is not its key's.
- `E_VAULT_EXISTS` 1: the key is already here (`detail` names its label on this machine).
- `E_VAULT_NO_IDENTITY` 1: no such identity; for `*-wallet-former-move`, `--former` not a former key of this seat.
- `E_VAULT_MIGRATE` 1: the old vault could not be taken in.
- `E_VAULT_ADDRESS` 1: a payout address that is malformed.
- `E_VAULT_NOT_BACKUP` 1: not this app's machine backup.
- `E_VAULT_BACKUP_PASSWORD` 1: a wrong backup password or a changed file.
- `E_VAULT_BACKUP_VERSION` 1: a backup newer than this app.
- `E_VAULT_KEYSTORE_PARAMS` 1: a key file outside the bounds (scrypt memory up to 2^30 bytes, work up to 2^32, p up to 16; pbkdf2 c 1 to 2^24).
- `E_VAULT_HEAD_UNREAD` 1: the key store does not read.
- `E_VAULT_NAME_KEY` 1: the name key does not open.
- `E_VAULT_PASSWORD_LONG` 1: a key file passphrase over 1024 bytes.
- `E_VAULT_BACKUP_NOT_LANDED` 1: a backup did not land: its read-back is not compliant or states another address.
- `E_VAULT_KDF_FLOOR` 1: a vault file's parameters below the floor; refused before any derivation, not counted.
- `E_VAULT_BACKUP_SHAPE` 1: a backup's header is broken.
- `E_ZK_INDEX_FORM` 1: the ZIKARON kit index is in a form not known.
- `E_ZK_INDEX_SHAPE` 1: the ZIKARON kit index does not read.
- `E_ZK_NO_SUCH_KIT` 1: no such kit in ZIKARON.
- `E_ZK_PACK` 1: the kit cannot become manifest items.
- `E_ZK_IO` 4: a disk read or write for ZIKARON failed.
- `E_ZK_ARG` 1: an argument given to ZIKARON is wrong.
- `E_P6_BY_CALL_KEY` 1: by call, a `buyerKey` that is not the current key; the order confirms by signature.
- `E_P6_FORM` 1: a rung out of range.
- `E_P6_NOTHING` 1: `buyer-sweep`: nothing of the token belongs to the buyer on the instance; nothing to take back, nothing sent.
- `E_P6_NOT_COMPANY` 1: the order is not the company's.
- `E_P6_NOT_OPEN` 1: the order's case is not open yet.
- `E_P6_NO_PACK` 1: the order file is not imported.
- `E_P6_NO_PAYOUT` 1: `buyer-sweep`: no refund address reads (neither the chain nor an order file holds one); nothing sent.
- `E_P6_NO_TERMS` 1: the order file has no terms.
- `E_P6_OPENED` 1: the case is open, so no refund on expiry.
- `E_P6_PULL_KEY` 1: the order needs a transfer to it first (the transfer-first route stands).
- `E_P6_READONLY` 1: an order this seat holds read only.
- `E_P6_SETTLE_EXPIRED` 1: `buyer-settle-sign`, `buyer-settle`: the settlement offer has expired; nothing signed or sent.
- `E_P6_SETTLE_FORM` 1: a buyer's share out of range, or a settlement link naming a listing.
- `E_P6_SETTLE_GATE` 1: `buyer-settle-sign`, `buyer-settle`: the seller's settlement registration does not read as held; nothing signed or sent.
- `E_P6_SHORT` 1: `buyer-dispute`: the key the case fee leaves from holds less than is owed (shortfall and address in `detail`); nothing sent.
- `E_P6_STALE` 1: the order changed meanwhile; this step not done.
- `E_P4_NOT_OPENED` 1: an order imported before its case opened.
- `E_P4_FOREIGN_KEY` 1: the order's buyer key is not on this machine.
- `E_P4_NO_ENDPOINT` 1: no nodes set up for importing.
- `E_P4_UNTRACED` 1: the case cannot be traced.
- `E_P3_OPEN_BY_PINNED` 1: `buyer-order-sign`: the order's budget address is taken at its pinned `openBy`.
- `E_P3_COMPANY_TAKEN` 1: `buyer-order-sign --export-proposal`: a case is already open at the company order's budget address; nothing exported.
- `E_P3_NO_ENDPOINT` 1: no nodes set up for paying.
- `E_P3_STANDARD_CHAIN` 1: the link's network does not match.
- `E_P3_STANDARD_LOGIC` 1: the logic contract is not the standard one.
- `E_P3_STANDARD_NO_CODE` 1: the standard logic address holds no code on the chain.
- `E_P7_AMOUNT` 1: an amount of zero; for `*-wallet-former-move`, nothing that moves.
- `E_P7_DECIMALS` 1: the token's decimals do not read.
- `E_P7_KEEP` 1: on chain, but the receipt was not recorded.
- `E_P7_OVER` 1: more than the balance.
- `E_P7_RECEIPT` 1: a receipt missing fields.
- `E_P7_STALE` 1: the wallet changed meanwhile; this step not done.
- `E_POOL_NO_GUILD_TERMS` 1: the guild gives no covenant.
- `E_POOL_NO_PANEL` 1: the guild's book holds no panel yet.
- `E_POOL_TERMS_DIGEST` 1: the guild's covenant does not match.
- `E_CANDS_NO_REGISTRY` 1: no judge registry set up.
- `E_CANDS_UNREACHABLE` 1: the judge registry not reached.
- `E_CANDS_NO_DOCUMENT` 1: no details found for the judge.
- `E_CANDS_UNVERIFIED` 1: the judge's details fail their check.
- `E_CANDS_RETIRED` 1: the judge has retired.
- `E_VAULT_PARTY_KEY` 1: a key from outside: only keys this app made enter a seat; nothing added.
- `E_ZK_NOT_CONNECTED` 1: ZIKARON is not connected.
- `E_ZK_POINTER_STALE` 1: the ZIKARON folder has moved.
- `E_ZK_NO_KITS` 1: ZIKARON holds no kits yet.
- `E_<code>` 1 or 4: through the desktop, a cause of the desktop's own is `E_` plus its code (e.g. `E_LOCKED`, `E_NO_HOME`, `E_DOOR_SHUT`, `E_INSUFFICIENT_FUNDS`), 4 for a network refusal, else 1; `contract` lists them all.
- `E_LEDGER`, `E_KIT`, `E_ALREADY_ROOTED`, `E_GRANT_FILE` 1: through the desktop only, the desktop's code of the same spelling (`detail` its evidence).
- `E_RANDOM`, `E_ENTRY`, `E_TIP_ABSENT`, `E_ENTRY_ABSENT`, `E_TIP_FORKED`, `E_SCAN`, `E_ENDPOINTS_DISAGREE`, `E_BADGE`, `E_FRAGMENT`, `E_NOT_EMPTY`, `E_RETRACTION`, `E_WOULD_BREAK`, `E_V_FORM`: listed by `contract`; no verb of this product answers them.

Cases under an earlier generation of the standard:

- A case made under an earlier generation of the standard is written only while the machine-wide switch is on (settings **Show past contracts**, `settings-past-cases --on true|false` on either seat, through the desktop too; off by default).
- Switch off, the case is not listed and a verb that would write it is refused before anything is signed or sent: `E_V_DOWN` `downToken` `E_A1_PAST_CONTRACT`, exit 1; a listing of an earlier generation opens no new case whatever the switch: `downToken` `E_A1_PAST_LISTING`, exit 1.

## 5 · Verbs

Each line: verb, what it does, the success keys (`?` marks a key present only under some conditions). `<seat>` is `seller` or `buyer`. Flags per verb: `eravon contract`.

Both seats (`seller-<verb>` and `buyer-<verb>`):

- `<seat>-vault-init`: creates the vault with a primary identity from the twelve words in `--phrase`, its home on `--chain`'s network; `address`, `identities`, `kdfLogN`, `kdfMs`, `seat`, `trouble`?. With a vault already there, `buyer-vault-init` lays the buyer seat out as `buyer-seat-set` does (`address`, `endpointCount`, `seat`).
- `<seat>-vault-import`: seller: adds a key this client makes as a new identity of the seat (`address`, `endpoints`, `endpointsFrom`, `identities`, `kind`, `trouble`?); buyer: as `buyer-vault-init`. A key handed in (`--key`) is refused on either seat (`E_VAULT_PARTY_KEY`).
- `<seat>-vault-derive`: the address and derivation path derived for `--role`; no private key; `address`, `path`, `role`.
- `<seat>-vault-lock`: zeroes the master key held by this process; the door state after locking; `door`, `open`, `wrongCount`.
- `<seat>-vault-unlock`: checks the passcode and unlocks the vault; `door`, `open`, `opened`, `wrongCount`.
- `<seat>-vault-passcode`: changes the vault passcode (`--new-passcode`, repeated in `--again`) after checking the current one; `changed`, `wrongCount`.
- `<seat>-vault-recover`: recovers the primary identity from `--phrase` and sets a new passcode; `recovered`, `wrongCount`.
- `<seat>-vault-list`: the vault's identities and whether it is locked; `count`, `identities`?, `locked`, `wrongCount`. The buyer answers no `wrongCount`.
- `<seat>-vault-accounts`: this seat's accounts, each identity by its key in office, and the active key; `accounts`, `current`, `seat`.
- `<seat>-vault-backup`: writes an identity's private key as a keystore V3 file into `--out` under `--file-passcode`, read back; `address`, `bytes`, `file`, `landed`.
- `<seat>-vault-import-words`: imports a mnemonic (`--phrase`) as a secondary identity of this seat; `address`, `endpoints`, `endpointsFrom`, `kind`, `label`, `trouble`?.
- `<seat>-vault-new-key`: makes a new mnemonic key as a secondary identity; its twelve words appear in this answer only; `address`, `endpoints`, `endpointsFrom`, `label`, `phrase`, `seat`, `trouble`?.
- `<seat>-vault-phrase`: an identity's twelve words, after a fresh passcode check; `address`, `phrase`, `words`.
- `<seat>-vault-rebuild`: reopens a vault whose passcode is lost with the primary's key file (`--file`, `--file-passcode`) under `--new-passcode`; `address`, `endpoints`, `endpointsFrom`, `otherKeysCounted`, `otherKeysLost`, `primary`, `trouble`?.
- `<seat>-vault-remove`: removes a non-primary key (`--address`) after a passcode check; its identity folder stays on disk; `current`, `homeKept`, `removed`.
- `<seat>-vault-reset`: makes a new identity (new words, or `--phrase`) and sets it as primary; the passcode stays; `address`, `endpoints`, `endpointsFrom`, `identities`, `phrase`, `trouble`?.
- `<seat>-vault-restore`: always refused (`E_VAULT_PARTY_KEY`): a backup key file (`--file`) enters no seat; nothing added.
- `<seat>-vault-switch`: switches this seat's active key to `--address`; `current`, `home`, `seat`.
- `<seat>-vault-rotate`: rotates this seat's key: lists the rotation, or with `--send true` runs it; `--continue false` only makes a new key; listing: `continue`, `old`, `unfinished`, and while a change goes on `cases`, `casesUnread`, `chains`, `deposits`, `fund`, `handed`, `historyLinked`, `offers`, `orders`, `owed`, `owedOther`, `refunds`, `refused`, `stranded`, `terms`, `txs`, `txsAtMost`, `unswept`; with `--send true`: `backupAgain`, `chains`, `continue`, `funds`, `left`, `letExpire`, `new`, `old`, `owed`, `owedOther`, `sent`, `swept`, `unswept`, `phrase`?, `leftOn`?; with `--export-proposal`: `accede`, `declare` (each company members), `new`, `old`, `state`.
- `<seat>-payout-set`: sets a key's payout address (`--address`, repeated in `--again`, EIP-55 checked) for terms signed afterwards; `key`, `payout`, `resignsPending`.
- `<seat>-payout-show`: the stored payout address of a key; `key`, `payout`.
- `<seat>-home-stand`: whether this data root is writable: mirror and staleness, writer lock, writer mark; `data`, `heldBy`, `heldPid`, `markUnread`, `mirrorCutAt`, `mirrorFrom`, `otherMachine`, `stale`, `staleSays`, `writable`.
- `<seat>-history`: a record's whole history on this chain, read off the chain; `--who` names it, else this seat's key; `chainId`, `history` (`attempts`, `cases`, `consents`, `documents`, `keys`), `who`. The buyer reads a seller's, a judge's or a guild's record before paying.
- `<seat>-deploy`: opens the case of order `--id`: the instance deployment, then `init`; sent with `--send true`; seller: `agreement`, `balance`, `caseId`, `funded`, `instance`, `need`, `deployTx`, `expireTx`, `gap`, `gapNote`, `id`, `note`, `notice`, `say`, `state`, `verdict`, `sent`, `txs`, send members; buyer: `caseId`, `instance`, `initTx`, `txs`?, `init`?, send members; company members with `--export-proposal`. The seller opens through its watch round; the buyer opens only where the terms let the buyer open (`buyerMayOpen`) and refuses a company's order (`E_V_COMPANY_PROPOSES`).
- `<seat>-expire`: for order `--id` past `min(expiry, openBy)` or revoked, never opened: returns the money to the buyer's payout; `caseId`, `expiry`, `instance`, `openBy`, `initTx`, `txs`?, `init`?, send members. The buyer adds `notKept`? where its order list could not record the end.
- `<seat>-settle`: builds or sends `settle` with both settlement signatures; `action`, `instance`, send members; buyer adds `needsSeller`. The seller checks the buyer's signature (`--sig-buyer`) against the instance; the buyer submits a seller's offer (`--settle`), by signature or with `--by-call true`.
- `<seat>-commit`: builds or sends `commit` with an evidence manifest hash (`--manifest`, or computed from `--manifest-file`); `action`, `instance`, send members, company members; buyer adds `needsSeller`.
- `<seat>-sweep`: builds or sends `sweep`: tokens that reached the instance after the case ended; `action`, `instance`, send members; buyer adds `needsSeller`. The seller pays `--token` to `--who`, both required; the buyer pays the case's refund address, `--who` if given must be it (`E_P6_SWEEP_WHO`).
- `<seat>-advance`: builds or sends `advance`, stamping an expired seat (`--case`, `--seat`) so the case moves on; `action`, `instance`, send members; buyer adds `needsSeller`.
- `<seat>-redraw`: builds or sends the guild track `redraw` with the panel the case froze; `--member`, `--amanah` must match it; `action`, `instance`, send members; buyer adds `needsSeller`.
- `<seat>-complaint-open`: where a complaint about the seated judge of order `--instance` goes: the guilds, each with its card; `complaint` (`caseId`, `guilds` (each `address`, `card`, `pointer`; not read: `refused`), `instance`, `judge`, `said`). The buyer also refuses an order it holds read-only (`E_P6_READONLY`).
- `<seat>-complaint-write`: signs the complaint `--text` about order `--instance`'s judge and writes one `eravon.complaint/1` file into `--out`; nothing sent; `complaint` (as `complaint-open`, with `written`), `file`.
- `<seat>-vault-rename`: renames one of this seat's keys (`--address`) to `--label`, taken as given; `address`, `label`.
- `<seat>-seat-set`: lays this seat out for a key (`--address`, else the current one) and makes it current; `address`, `endpointCount`, `seat`.
- `<seat>-backup-export`: writes a whole-machine backup into `--out` under `--file-passcode`, typed twice, at least eight characters; `count`, `file`.
- `<seat>-backup-restore`: restores a whole-machine backup over this machine; refused while the desktop holds the data root (`E_STORE_LOCKED`); `count`, `file`.
- `<seat>-settings-endpoints`: replaces the key's nodes on its home's network with `--nodes`; other networks' nodes stay; `address`, `endpointCount`.
- `<seat>-settings-network`: puts the key's home on the known network `--chain` names (mainnet or Sepolia); `address`, `chainId`, `endpointCount`.
- `<seat>-settings-zikaron`: sets the ZIKARON machine folder (`--zikaron-dir`; empty: found automatically); `address`, `dir`.
- `<seat>-settings-zikaron-link`: turns the ZIKARON link on or off (`--on`); `address`, `on`.
- `<seat>-settings-zone`: sets the key's time zone (`--zone`); `address`, `zone`.
- `<seat>-settings-cli-path`: puts this command line on the path, or takes it off (`--on`); `on`, `state`.
- `<seat>-settings-proxy`: sets this machine's way out (`--way`): the system's, none, or a proxy address; `way`.
- `<seat>-settings-token-add`: adds `--token` to the machine's table, decimals read on chain, else `--decimals`; `decimals`, `decimalsSource`, `label`, `token`.
- `<seat>-settings-token-remove`: removes `--token` from the machine's table; refused while a product or draft settles in it; `label`, `token`.
- `<seat>-settings-token-decimals`: sets `--decimals` for a token whose decimals are hand-typed or pending; `decimals`, `token`.
- `<seat>-settings-company`: turns company mode on for `--company` once the company reads on chain, or off (`--on`); `address`, `agreement`?, `calendarBound`?, `company`?, `on`.
- `<seat>-settings-time-check`: turns the time verification on or off (`--on`); `address`, `on`.
- `<seat>-settings-past-cases`: turns past-generation cases on or off (`--on`) for the whole machine, both seats; `on`.
- `<seat>-note-handle`: marks a reminder (`--reminder`) handled; `address`, `reminder`.
- `<seat>-export`: the export page: an order's case dossier or statement (`--tab`) into `--out`; `compliance`?, `counts`?, `files`, `instance`, `report`?, `selfcheck`?, `shape`?, `statement`?, `tab`.
- `<seat>-seal`: seals an evidence manifest this seat kept (`--manifest`, `--manifest-file`) to the seated judge, into `--out`; nothing sent; `dir`, `files`, `instance`, `judge` (`address`, `name`, `pick`: `state`, with `recipient` and `seq` for `one`, `count` and `reason` for `candidates`), `manifest`.
- `<seat>-direct-check`: checks a direct send (`--tx`) whose end is not known: its receipt, else the void rule; `block`?, `state`, `tx`.
- `<seat>-direct-give-up`: gives a direct send (`--tx`) up, only when it can never be included (`E_WATCH_NOT_VOID` otherwise); `state`, `tx`.
- `<seat>-wallet-send`: transfers `--token` from this seat's key to `--to`, after the payout-address and funds checks; `amount`, `balance`, `to`, `token`, `amountText`?, `amountDecimals`?, `balanceText`?, `balanceDecimals`?, send members. The seller also needs `--to-again`, repeating `--to`.
- `<seat>-wallet-former-move`: moves what a former key (`--former`) holds to the key in office: the `--token`, then its ETH; `address`, `from`, `moves`, `sent`, `moved`?, `paid`?, `why`?, `endpointCount`, `endpointSource`.

Seller only:

- `seller-terms-build`: assembles a terms document from a block file (`--block`), checks and stores it, optionally writes `--out`; answers fees; `amount`, `bytes`, `candidates`, `fees`, `feesWhy`?, `id`, `termsHash`.
- `seller-terms-hash`: answers the terms hash of a terms document file (`--file`); `termsHash`.
- `seller-terms-check`: validates a terms document and answers its key facts; `amount`, `chainId`, `expiry`, `guilds`, `judges`, `ladder`, `nonce`, `sellerKey`, `termsHash`.
- `seller-offer-sign`: signs the offer digest for every candidate of a terms document; `--export-proposal` writes a company proposal instead; `count`, `fees`, `feesWhy`?, `id`, `signatures`, company members.
- `seller-link-encode`: validates a listing file (`--file`) and encodes it as link text; `bytes`, `link`.
- `seller-link-decode`: decodes link text (`--link`) and answers the posting chain, transaction and document hash; `chainId`, `hash`, `tx`.
- `seller-receipt-paste`: binds a buyer receipt (`--receipt`) to its listing and terms, derives budget address and case id, records the order; `budget`, `buyerKey`, `candidate`, `caseId`, `id`, `instance`, `openBy`, `verdict`.
- `seller-watch`: reads whether an order's budget address holds the full amount: `--id` one order, `--scan` all; `--send` opens funded ones; `agreement`, `balance`, `caseId`, `funded`, `instance`, `need`, `priceAge`, `priceAnswer`, `priceDecimals`, `priceUpdatedAt`, `priceFeed`, `priceFeedDoor`, `priceFeedOwner`, `deployTx`, `expireTx`, `gap`, `gapNote`, `id`, `note`, `notice`, `say`, `state`, `verdict`, `sent`, `txs`, `enrolled`, `foreign`, `marked`, `failed`, `ticked`, `sends`, send members.
- `seller-claim`: builds the delivery manifest (`--manifest-dir`, `--note`), optionally a ZIKARON kit (`--kit`), then builds or sends `claimDelivery`; `action`, `handOver`?, `instance`, `kitFiles`, `kitId`, `kitLink`, `manifest`, send members, company members.
- `seller-refund`: builds or sends the seller's unilateral `refund`; `action`, `instance`, send members, company members.
- `seller-settle-offer`: signs a settlement offer (`--buyer-bps`, `--expiry`) and posts it on the board under the case id; full refund refused; `bytes`, `digest`, `sig`, `post`, company members.
- `seller-confirm-bysig`: checks the buyer's confirm signature (`--sig`) against the instance and relays it as `confirmBySig`; `action`, `instance`, send members.
- `seller-dossier-export`: writes an order's case dossier into `--out`, chain comparison and self-check first; one not whole is refused; as `seller-export`.
- `seller-statement-export`: writes an order's statement and its compliance page into `--out`; `compliance`?, `files`, `instance`, `statement`, `tab`.
- `seller-zikaron-status`: probes, read-only, for the ZIKARON machine folder (`--machine` overrides) and counts its kits; absence is an answer; `absent`, `at`, `connected`, `count`, `layer`.
- `seller-zikaron-kits`: lists the ZIKARON disclosure kits, read-only, found as `seller-zikaron-status` finds them; absence is an answer; `absent`, `at`, `connected`, `kits`, `layer`.
- `seller-product-new`: saves a product as `seller-product-save` does; as `seller-product-save`.
- `seller-product-edit`: saves a product as `seller-product-save` does; as `seller-product-save`.
- `seller-product-save`: saves a product's draft from `--draft-file` or disk, fields changed by `--title`, `--price`, `--note`; answers fees; signs nothing; `companyRefused`, `companyRefusedDetail`, `createdAt`, `draft`, `excluded`, `fees`, `id`, `link`, `linkState`, `linkStateDetail`, `missing`, `pending`, `series`, `termsHash`, `updatedAt`, `version`.
- `seller-product-publish`: saves, generates terms, pre-signs offers and builds the listing; `--send` posts it on the board at once; `companyRefused`, `companyRefusedDetail`, `createdAt`, `draft`, `excluded`, `fees`, `id`, `link`, `linkState`, `linkStateDetail`, `missing`, `pending`, `postedHash`, `postedTx`, `sent`, `why`, `txs`, `termsHash`, `updatedAt`, `version`.
- `seller-product-list`: lists stored products on the home's chain (or `--chain`), optionally filtered by `--draft`; `count`, `products`.
- `seller-product-show`: shows one stored product record (`--id`); `product`.
- `seller-product-live`: reads back a product's current draft field by field, including fields buyers do not see; `cands`, `deliveryNote`, `deliveryPrivate`, `description`, `draft`, `goodsPrivate`, `id`, `ladder`, `name`, `price`, `t1`, `t1h`, `t1m`, `t2`, `t2h`, `t2m`, `token`.
- `seller-product-link`: builds a product's listing from its terms and signatures and reads it back; `--out` writes it; `bytes`, `id`, `termsHash`.
- `seller-product-qr`: writes the QR code PNG of a product's payment link to `--out` when it fits and reads back; `budget`, `bytes`, `id`, `out`, `px`, `version`.
- `seller-product-resign`: rebuilds a product's terms from `--block` and signs the offers over it in one step; `offers`, `terms`.
- `seller-candidate-add`: adds a judge (by address) or a guild (by pointer) with a label to the candidate table; `candidate`, `count`, `kind`.
- `seller-candidate-list`: lists the candidate table and each entry read again on chain, with pill and state; `--registry` names the registries; `count`, `candidates`, `rows`.
- `seller-candidate-show`: shows one candidate row and its detail as read on chain; `addedAt`, `address`, `detail`, `kind`, `label`.
- `seller-candidate-remove`: removes one candidate from the table; `count`, `removed`.
- `seller-order-list`: lists orders from the local archive joined with a chain scan, filtered by `--tab`, searched by `--q`; `agreement`, `count`, `foreign`, `rows`.
- `seller-order-show`: shows one order: overview, history, moments, fees, terms, the live status row and the order card's tabs; `agreement`, `detail`, `fee`, `handOver`?, `history`, `instance`, `moments`, `order`, `status`, `terms`, `said`?.
- `seller-payways-set`: saves the case-opening settings in one edit: `--auto-open`, `--open-window`, `--fund`, among others; `address`, `saved`.
- `seller-fund-check`: reads the vault's state for this seller key on `--token`: binding, balance, allowance, the deposit it stands; `agreement`, `allowance`, `available`, `balance`, `bound`, `fund`, `logic`, `token`, `deposit`?, `orders`?, `act`?, `hints`.
- `seller-fund-setup`: builds or sends the vault's allowance (`--amount`) and `acceptFunding`; `--export-proposal` writes two company proposals; `agreement`, `allowance`, `available`, `balance`, `bound`, `fund`, `calls`, `sent`, `why`, `txs`.
- `seller-candidate-rename`: renames a candidate; `candidate`, `label`.
- `seller-product-listed`: puts a product on this machine's list or takes it off (`--on`); `id`, `listed`.
- `seller-product-post`: posts a live version's listing on the board (`post(termsHash, listing)`) and answers the link; fees shown are checked first; `expiry`, `id`, `link`, `postedHash`, `postedTx`, `sent`, `why`, `txs`.
- `seller-product-revoke`: revokes a version's terms on chain (`revoke(termsHash)`) and reads it back; a company's version is refused; `id`, `revoked`, `sent`, `why`, `txs`.
- `seller-product-delete`: deletes a draft; a published product is refused; `removed`.
- `seller-product-export-proposal`: writes a product's company proposal into `--out`; `file`, `id`, `timeShort`?.
- `seller-pending-bump`: raises the fees of an order's pending transaction, at its nonce; `instance`, `note`, send members.
- `seller-pending-void`: gives up an order's pending transaction, only when it can never be included; `instance`, `note`.
- `seller-order-label`: labels a buyer on this seat's orders; `buyer`, `label`.
- `seller-deliver-save`: saves an order's delivered originals into `--out`, each checked against its sha256; `files`, `instance`.

Buyer only:

- `buyer-open`: opens a payment link (`--link`): reads its board post, checks terms and candidates, stores the listing, answers the offer facts; `chainId`, `candidates`, `expiry`, `guilds`, `judges`, `kind`, `logic`, `amount`, `sellerKey`, `sellerPayout`, `openWindow`, `termsHash`, `token`, `stored`, `storedWhy`, `gate`, `choosable`, `acts`?, `openBy`?, `segments`?, `standing`?, `page`?.
- `buyer-choose`: picks a candidate (`--candidate`) from a link and hands over its quote hash and offer signature; `candidate`, `offerSig`, `quoteHash`, `sellerKey`.
- `buyer-order-sign`: signs the order for the chosen candidate (`--candidate`), writes its receipt and case pack; `--export-proposal` writes a company proposal instead; `budget`, `caseId`, `depositShort`, `id`, `instance`, `openBy`, `orderDigest`, `prev`, `receiptText`, `segmentsMissing`, `sig`, company members.
- `buyer-pay`: transfers the settlement token to the order's budget address (`--to`, `--id`) after a funds check; sends with `--send true`; `balance`, `amount`, `to`, `token`, `amountText`?, `amountDecimals`?, `balanceText`?, `balanceDecimals`?, send members, company members.
- `buyer-import`: imports a case pack (`--pack`) under this seat's key; `--tx` rebuilds the order from on-chain calldata; `caseId`, `id`, `instance`, `rebuilt`.
- `buyer-list`: lists this seat's orders from one chain scan plus local orders not yet on chain; `--tab` and `--q` filter; `agreement`, `cases`, `count`, `foreign`, `pending`, `rows`.
- `buyer-show`: shows one order (`--instance`): live status, quoted fees, event history and its row on the orders page; `agreement`, `detail`, `feeQuoted`, `history`, `instance`, `row`, `status`.
- `buyer-confirm`: builds or sends `confirm`; with `--passcode` also signs a `confirmBySig` for the seller to submit; `action`, `instance`, `needsSeller`, `bySig`?, send members, company members.
- `buyer-cancel`: cancels the order `--id` before its case opens: the deployment where needed, then `cancel`; the budget returns to the buyer; `caseId`, `instance`, `initTx`, `txs`, send members.
- `buyer-attach`: drops in a hashed segment's private text (`--segment`, `--file`) for the listing `--link` names; `--id` rewrites that order's pack; `hash`, `segment`, `stored`, `pack`?.
- `buyer-offers`: lists the settlement offers posted on the board under the case of `--instance`, each with whether it still runs; `caseId`, `count`, `instance`, `offers`.
- `buyer-dispute`: requests the refund while the case awaits confirmation: the litigation fee owed goes to the instance, then `dispute`; `action`, `fee`, `feeTx`, `dispute`, `instance`, `needsSeller`, `bySig`?, `pull`, `txs`, send members, company members.
- `buyer-settle-sign`: countersigns a settlement offer (`--settle`, a file or board link) and answers the settlement with both signatures; `buyerBps`, `digest`, `expiry`, `instance`, `sigBuyer`, `sigSeller`, company members.
- `buyer-withdraw-to`: transfers the settlement token from the buyer key to `--to`, the whole balance without `--amount`, after a funds check; `amount`, `balance`, `to`, `token`, `amountText`?, `amountDecimals`?, `balanceText`?, `balanceDecimals`?, send members.
- `buyer-wallet-balance`: reads the buyer key's balance of `--token`, or its native balance; without `--address` also lists movable former keys; `address`, `agreement`, `balance`?, `formers`?, `native`?, `token`?.
- `buyer-wallet-tx`: reads one transaction by hash (`--tx`): its block, sender, recipient and value; `agreement`, `blockNumber`, `from`, `hash`, `to`, `value`.

No seat:

- `contract`: the verbs, each verb's flags and must-give cells, the exit codes and every refusal reason as one value (section 10).
- `mcp`: the Model Context Protocol on stdin and stdout (section 13).

## 6 · What every seat verb takes

- Every seller and buyer verb takes `--data`, `--now`, `--writer`, `--endpoint`, `--table`, `--roots`, `--proxy`, `--rest-keys`, `--chain`, `--logic`, and all but `seller-vault-rebuild` and `buyer-vault-rebuild` take `--passcode`.
- Every verb that sends a transaction also takes `--send`, `--wait`, `--gas-limit`, `--max-fee`, `--priority-fee`; a verb that only signs takes `--wait` alone.
- Every seller and buyer verb must be given `--data` and `--now`; with `--desktop`, nothing else is needed.

## 7 · Answer keys

- `ok`: whether affirmed.
- `reason`: the refusal reason.
- `detail`: the refusal's words for people; never branched on.
- `layer`, `downToken`: with `E_V_DOWN`, the lower layer that refused and its own code, which decides the exit.
- `rule`: with `downToken` `E_VAULT_RULE`, the passcode rule not met.
- `gate`: with `E_V_PRECHECK`, the local precheck that stopped the call before anything was signed.
- `txHash`: in a refusal, the transaction sent (`E_V_REVERTED`, `E_V_UNCONFIRMED`, `E_V_SENT_VOID`).
- `nonce`: with `E_V_SENT_VOID`, the nonce of the send that can never be included.
- `txs`: in a refusal, the transactions of a series already sent (each `chain`, `tx`); in an answer, the hashes sent, in order.
- `door`, `owner`: with `E_V_NOT_YET`, the door not available here and who provides it.
- `count`, `tx`: with `E_TX_NOT_YET`, the seconds waited and the transaction out; `tx` alone with `E_UNREACHABLE` after a send.
- Send members, on a verb that builds a transaction: `tx` (the call: `data`, `to`, `value`), `sent` (whether it went out), `why` (why it did not).
- When sent, also: `txHash`, `endpoint` (the node it went through), `nonce`, `gasLimit`, `feeSource` (`given`, `chain`, `fallback`), `receipt`, `recordOpen`? (the send's local record left open: name and words).
- `receipt`: `block`, `gasUsed`, `status`, `waited`; `{"waited":false}` with `--wait 0`; an affirmed answer's `status` is 1.
- `endpointCount`, `endpointSource`: how many nodes were read and where the table came from: `inline` (`--endpoint`), `file` (`--table`), `identity` (the seat's own); most chain-reading verbs answer them.
- Company members, on a verb run with `--export-proposal`: `company` (`address`, `agreement`, `calendarBound`, `ledger`, `wallet`), `face` (the proposal's face lines), `proposal`, `time`; beside them `sent` `false` and `why`.
- `proposal`: the proposal written: `calls`, `digest`, `documents` (`kind`, `name`, `sha256`), `file`, `proposalHash`, `structural`.
- `time`: the time check: `bound` (the calendar bound, seconds), `enough`, `gate` (whether the check is on), `rows`.
- `agreement`: `agreed` when two or more nodes gave the reading, `single` when one did.
- `action`: the contract function built or sent (`confirm`, `dispute`, `commit`, …).
- `instance`, `caseId`: the case's instance address (the budget address before a case opens) and its case id.
- `initTx`, `init`: the second call of a two-call send (`data`, `to`), and its send members once sent.
- `amountText`, `amountDecimals`, `balanceText`, `balanceDecimals`: with `--units-text`, the whole-token form and the decimals used; `null` where the decimals are unknown.
- `id`: the local order or record id.

## 8 · Flags

Each line: flag, value. A secret flag's file form (`--<flag>-file`) stands for the flag.

- `--address`: an account address (hex20): the identity the verb acts as or names; absent: the seat's active key.
- `--advance`: `buyer-dispute` export form: `wallet` or `personal`; absent: `wallet`.
- `--again`: the new value typed a second time, which must match; a secret, file form `--again-file`.
- `--again-file`: a file holding `--again`'s value.
- `--amanah`: hex32, repeatable: a guild roster's frozen terms hashes; absent: the frozen panel read from the instance.
- `--amount`: an amount in the token's smallest units; absent: the whole balance (`withdraw-to`, `wallet-send`), else refused.
- `--amount-text`: the same amount in whole tokens, converted by the token's decimals; with `--amount`, misuse.
- `--attach`: a complaint attachment `kind:path` (`dispatch`, `pass`, `audit-input`, `other`), repeatable.
- `--auto-open`: `true` or `false`: the desktop's automatic rounds open the case once the budget address holds the full amount; absent: the saved setting.
- `--block`: path to the terms block file.
- `--buyer`: a buyer's key address (hex20): whose orders the label names.
- `--buyer-bps`: the buyer's share in basis points.
- `--buyer-key`: the buyer's key address (hex20); absent: from the stored order.
- `--buyer-may-open`: `true` or `false`: whether the terms let the buyer open the case; absent: the saved setting.
- `--buyer-payout`: the buyer's payout address (hex20); absent: from the stored order.
- `--by-call`: `true`: submit a settlement by call, sent by the buyer key; absent: by signature.
- `--candidate`: repeatable: a candidate's index on the link, or a judge address or guild pointer for the candidate table.
- `--case`: the case id (hex32) or the case's index on the instance.
- `--chain`: chain id; `settings-network`: `1` mainnet or `11155111` Sepolia; the run keeps to it throughout; absent: the network of the run's home.
- `--company`: the company address (hex20); absent: the identity's setting.
- `--continue`: `true` or `false`: a key rotation hands the record on, or makes a new key only; absent: `false`.
- `--data`: path to the data root folder; every seller and buyer verb must be given it.
- `--decimals`: a token's decimals typed by hand, 0 to 77, for a contract that answers no `decimals()`; absent: read on chain.
- `--desktop`: path to the data root the desktop holds: the running desktop does the verb, which then takes its home flags only.
- `--draft`: `true` or `false`: filter products by draft state; absent: every product.
- `--draft-file`: path to a product draft file; absent: the draft on disk.
- `--endpoint`: an RPC endpoint URL, repeatable, never echoed; absent: `--table`, else the identity's stored endpoints.
- `--expiry`: an expiry time, Unix seconds.
- `--expiry-word`: `seller-settle`: the expiry as one 32-byte word, hex.
- `--export-proposal`: path of a company proposal file written instead of signing or sending; absent: the verb signs or sends itself.
- `--file`: path to the file the verb reads: a terms document, listing, segment text, backup key file or key pack.
- `--file-passcode`: a backup key file's passcode; a secret, file form `--file-passcode-file`.
- `--file-passcode-file`: a file holding `--file-passcode`'s value.
- `--former`: a former key of the seat (hex20), as `wallet-balance` lists it under `formers`.
- `--from`: `order-list`, `buyer-list`: the search's start time, seconds; absent: no lower bound.
- `--from-block`: first block of a chain scan; absent: the identity's start block.
- `--fund`: the seller's vault address (hex20, the terms' `sellerFund`); absent: the saved setting.
- `--fund-own`: `true` or `false`: the seller key is its own vault; absent: the saved setting.
- `--gas-limit`: the transaction's gas limit; absent: one and a half times the agreed estimate, rounded up.
- `--guild`: the guild a complaint goes to (hex20), where several guilds list the judge; absent: the only one, else refused.
- `--guild-panel`: the guild's panel hash (hex32), one value.
- `--guild-seq`: the guild's roster sequence; absent: 0.
- `--guild-sig`: the guild's signature (hex65), one value.
- `--home`: path of the ZIKARON home (`claim`, `commit`, `seal`, `zikaron-status`, `zikaron-kits`).
- `--id`: a local record id (order, product, terms); absent: a new id where the verb makes one, else refused.
- `--instance`: the case instance address (hex20).
- `--judge-sig`: the judge's signature (hex65), one value.
- `--key`: a private key, 64 hex digits (`0x` allowed), in [1, n−1]; a secret, file form `--key-file`; malformed: `E_KEY`.
- `--key-file`: a file holding the private key as `--key` takes it; white space at either end dropped.
- `--kind`: `judge` or `guild`, for the candidate table.
- `--kit`: path to a ZIKARON disclosure kit; absent: no kit.
- `--label`: a label for the key or candidate; absent: none.
- `--leave-coins`: `true`: a key rotation goes on while the seller key holds tokens, leaving them on the former key; absent: `false`.
- `--let-expire`: repeatable: a paid order's id a key rotation leaves to run out to its refund at `openBy`.
- `--link`: a payment link: `eravon://open?e=…` text or a file holding the link document; `seller-link-decode` takes the text only.
- `--listing`: path to a listing file (`eravon.listing/1`).
- `--logic`: the logic contract address (hex20); absent: from the standard table.
- `--machine`: path to a ZIKARON machine folder; absent: the settings' folder, else found as ZIKARON finds it.
- `--manifest`: an evidence manifest hash (hex32); absent: computed from `--manifest-file`.
- `--manifest-dir`: path to the folder the delivery manifest is built from; absent: `seller-claim` takes no files, others refuse.
- `--manifest-file`: path to an evidence manifest file; absent: `--manifest`.
- `--max-fee`: max fee per gas, wei; absent: from the head block's base fee and recent tips.
- `--media-type`: the delivery's media type; absent: none.
- `--member`: a roster member address (hex20), repeatable; absent: the frozen panel read from the instance.
- `--new-passcode`: the new vault passcode; a secret, file form `--new-passcode-file`.
- `--new-passcode-file`: a file holding `--new-passcode`'s value.
- `--nodes`: `settings-endpoints`: the endpoint table as typed, node addresses separated by spaces or commas.
- `--note`: prose; absent: empty.
- `--now`: the current time, Unix seconds; every seller and buyer verb must be given it.
- `--on`: `true` or `false`: a switch turned on or off.
- `--open-by`: the order's `openBy`, seconds, later than now; absent: `min(expiry, now + openWindow)` where the verb computes it.
- `--open-window`: the opening window in seconds (uint32), counted from the buyer's signature; absent: the saved setting.
- `--out`: output path, a file (`vault-backup`: a folder); absent: nothing written where the verb allows it, else refused.
- `--out-pack`: path to write the case pack to; absent: not written.
- `--out-receipt`: path to write the receipt to; absent: not written.
- `--pack`: a file holding a case pack.
- `--passcode`: the vault passcode; a secret, file form `--passcode-file`; absent: the vault is not opened, refused where needed.
- `--passcode-file`: a file holding `--passcode`'s value; `mcp`: handed on to each tool whose verb opens the vault.
- `--payout`: a refund address (hex20, EIP-55 checked in mixed case); absent: the buyer key.
- `--phrase`: twelve recovery words; a secret, file form `--phrase-file`.
- `--phrase-file`: a file holding `--phrase`'s value.
- `--price`: a price in the token's smallest units; absent: unchanged.
- `--price-feed`: a price feed address (hex20); absent: no price reading.
- `--priority-fee`: priority fee per gas, wei; absent: from recent tips.
- `--proxy`: `system`, `none`, or `scheme://[user[:password]@]host:port` (`http`, `https`, `socks5`, `socks5h`, `socks4`, `socks4a`); credentials never echoed; absent: `system`.
- `--pull`: `true`: top up the litigation fee from the buyer key by permit or approve; absent: `false`.
- `--q`: search text; absent: no search.
- `--receipt`: a receipt: `eravon-receipt:…` text or a file holding it.
- `--registry`: a candidate registry address (hex20), repeatable; absent: the literal registry slot.
- `--reminder`: a reminder's key, as the home and alerts pages list it.
- `--rest-keys`: a file of `<service> <key>` lines (`etherscan`, `blockscout`, `etherscan-like`); read once, never said back; `mcp` hands it on.
- `--revoke`: repeatable: a product's id a key rotation takes down instead of registering its offers.
- `--role`: a derivation role.
- `--roots`: path to a PEM file of one root certificate, trusted beside the compiled-in table; absent: that table only.
- `--scan`: `true` or `false`: run the startup scan over stored orders; absent: `false`.
- `--seat`: a seat name on the case.
- `--segment`: which hashed segment: `goods` or `delivery`.
- `--send`: `true` or `false`: sign, send and wait for the receipt; absent: `false`, the transaction built and answered, not sent.
- `--series-id`: `new`, `none`, or the id (hex32) of one of this seat's series; another id is refused; absent: unchanged.
- `--settle`: a settlement offer: a file holding it, or `eravon://open?e=…` link text naming the offer posted on the board.
- `--sig`: a signature (hex65).
- `--sig-buyer`: the buyer's signature (hex65).
- `--sig-seller`: the seller's signature (hex65).
- `--stage`: the product's stage in its series, from 1; absent: unchanged.
- `--stages`: the number of stages of its series; absent: unchanged.
- `--t1`: the delivery window's days; absent: 0.
- `--t1h`: the delivery window's hours, 0 to 23; absent: 0.
- `--t1m`: the delivery window's minutes, 0 to 59; absent: 0.
- `--t2`: the confirmation window's days; absent: 0.
- `--t2h`: the confirmation window's hours, 0 to 23; absent: 0.
- `--t2m`: the confirmation window's minutes, 0 to 59; absent: 0.
- `--tab`: `active` or `done` (order lists); `dossier` or `statement` (exports); absent: every order, an export refuses.
- `--table`: path to an endpoint table file, one value; absent: the identity's stored endpoints.
- `--terms`: path to a terms document, or a stored terms id.
- `--text`: what a complaint says.
- `--time-check`: `on` or `off`: refuse a deadline shortfall against the company's objection window; absent: the identity's setting.
- `--title`: prose; absent: unchanged.
- `--to`: a recipient address (hex20, EIP-55 checked in mixed case); `order-list`, `buyer-list`: the end time, seconds, absent no upper bound.
- `--to-again`: `seller-wallet-send`: `--to` typed a second time.
- `--to-block`: last block of a chain scan; absent: the head.
- `--token`: a token address (hex20); absent: the native coin (`wallet-balance`) or the order's token.
- `--tx`: a transaction hash (hex32).
- `--units-text`: `true`: amounts are also answered in whole tokens; absent: smallest units only.
- `--wait`: seconds to wait for the receipt (absent: 60); through a queue-only desktop, for the person's decision (absent: `E_QUEUED` at once).
- `--way`: this machine's way out: `system`, `none`, or a proxy address as `--proxy` takes; answered without its password.
- `--who`: an address (hex20) the verb acts for or pays to; absent: the seat's active key (`buyer-sweep`: the case's refund address).
- `--writer`: the writer name recorded in the store; absent: `eravon`.
- `--zikaron-dir`: the ZIKARON machine folder; empty or absent: found automatically.
- `--zone`: a time zone by its written name.

## 9 · Environment variables

- `ERAVON_TIMEOUT_SECS`: one node exchange's total wall time, seconds (default 30; `0` none).
- `ERAVON_TIMEOUT_MS`: the same in milliseconds; when set and readable, overrides the seconds.
- `ERAVON_MAX_ANSWER_BYTES`: largest node answer in bytes (default 67108864; `0` none).
- A number variable that does not read as a whole number is ignored; the default holds.
- `ERAVON_TRACE`: a file to append trace marks to; stdout and exit unchanged (default unset).
- `ERAVON_CLI_PASS`: the program key that opens a sealed home with the credential, read only when one is opened (default unset).
- `https_proxy`, `http_proxy`, `all_proxy`, `no_proxy` (lower or upper case): followed under `--proxy system`; a read or send by the home's own nodes without `--proxy` uses this machine's proxy setting instead.
- `LC_ALL`, `LC_MESSAGES`, `LANG`, `LANGUAGE`: the language of stderr lines (default English).

## 10 · `contract`

- Takes no flags; answers exit 0: `ok`, `exits` (`code`, `name`), `reasons`, `standsFor` (`flag`, `for`), `verbs` (`name`, `flags`, `homeFlags?`, `required`: one array per cell).

## 11 · `--desktop`: through the desktop

- `--desktop <data root>`: the running desktop that holds that data root as its writer does the verb with its own open vault, nodes and settings; the command line holds no secret and writes nothing itself.
- The door: a local socket readable by its owner only (Windows: a named pipe only this user reaches), open while the desktop is unlocked, holds the data root as its writer, "Through the desktop app" is on and "With a credential" is off; no token; a caller running as another user is refused.
- Verbs: every verb with `homeFlags` in `contract` takes `--desktop`; `contract`, `mcp`, `seller-vault-lock`, `seller-vault-list`, `seller-home-stand`, `seller-terms-hash`, `seller-terms-check`, `seller-link-encode`, `seller-link-decode`, `buyer-vault-lock`, `buyer-vault-list`, `buyer-home-stand`, `buyer-choose` do not (misuse).
- Flags: `--desktop` and the verb's `homeFlags` only (its own flags less `--data`, `--passcode`, `--rest-keys` and every secret); misuse is judged here before the desktop is asked (exit 2); the data root must be a folder; a path flag is made absolute before it goes.
- `E_ON_DESKTOP` 1, nothing done, for each seat's `vault-init`, `vault-import`, `vault-unlock`, `vault-passcode`, `vault-recover`, `vault-backup`, `vault-import-words`, `vault-new-key`, `vault-phrase`, `vault-rebuild`, `vault-remove`, `vault-reset`, `vault-restore`, `vault-switch`, `vault-rename`, `payout-set`, `backup-export`, `backup-restore`.
- Answers: the verb's own answer and exit, as run directly, less every `phrase` member.
- `E_DESKTOP` 4, `detail`: `NOT_OPEN` no door (desktop locked or not running, not this data root's writer, switch off), `CLOSED` it locked, let the data root go or quit first, `PATH_TOO_LONG` the door's place is too long for this system, `NOT_YOU` another user keeps it, `BROKEN` broke off or an answer not read, `NO_MACHINE` no machine folder, `ALONE` "With a credential" is on.
- A data root another machine writes has no door: `E_V_DOWN`, `downToken` `E_STORE_OTHER_MACHINE`, exit 1, nothing written.
- A refusal of the desktop is the command line's name of the same cause, else `E_` and the desktop's code; `token` the desktop's code where it differs from the name, `detail` its evidence; it exits as that name exits run directly (section 4); a code this version does not know is `E_DESKTOP` `BROKEN`.
- The door's own causes (`E_DOOR_UNREAD`, `E_IN_FLIGHT`, `E_OUTCOME_LOST`, `E_REKEYING`, `E_LOCKED`, `E_NO_HOME`, `E_DOOR_SHUT`): section 4.
- `--chain` naming a known network other than the desktop's: `E_V_DOWN`, `layer` `desk`, `downToken` `E_DOOR_OTHER_CHAIN`, exit 1, nothing read or sent; not for `settings-network`, `settings-endpoints`, `vault-rotate`.
- Requests are done one at a time, in the order they reach the door.
- A request that signs or sends (`--send true`; always `seller-offer-sign`, `seller-product-publish`, `seller-product-resign`, `seller-settle-offer`, `buyer-order-sign`, `buyer-confirm`, `buyer-dispute`, `buyer-settle-sign`, each seat's `complaint-write`, and `buyer-settle` unless `--by-call true`) follows the desktop's command-line send setting; one that only reads runs at once.
- *Send automatically* (the default): it runs at once; *Queue only*: it waits on the desktop for the person to approve or decline it on a card.
- Queue only, no `--wait`: `E_QUEUED` 3 at once, `count` how many wait; with `--wait <seconds>` (more than zero): approved, the verb's answer; declined, `E_APP_DENIED` 1; the seconds passed, `E_QUEUED` 3; while an answer is held open the door takes no other request.
- A request held open when the desktop locks, lets the data root go or switches it: `E_DESKTOP` `CLOSED`, nothing run; one already answered `E_QUEUED` waits through a lock of the same data root and is dropped when the desktop lets that data root go, switches or quits.
- The send setting is changed on the desktop only, and is in force only while "Through the desktop app" is on and "With a credential" off; otherwise every road sends automatically.

## 12 · With a credential

- Two switches in Settings > Command line and AI control; "With a credential" outranks "Through the desktop app"; one way at a time.
- Both off: the command line opens a vault with `--passcode` only; the door is closed.
- "Through the desktop app" on: writes go through the door while the desktop is unlocked (section 11).
- "With a credential" on: the command line opens the vault alone, the desktop open or not, but not on a data root the desktop holds (`E_V_DOWN`, `downToken` `E_CREDENTIAL_DESKTOP_HOLDS`, exit 1); the door is shut (`E_DESKTOP` `ALONE`).
- The credential: a file in this user's machine folder, readable by its owner only, written on the desktop past the passcode with a validity or none; one row per program key, no key in it.
- A program key: `evp_` and 64 hex digits, added on the desktop (a note of at most 40 characters, at most 16 keys), shown once, kept nowhere, deleted one by one.
- The program brings its key in `ERAVON_CLI_PASS`; no passcode.
- Every seat verb given `--data` and no `--passcode` opens the vault with the credential and runs as with the passcode, signing with the seat's key in the vault; `--passcode` given is taken as given.
- A write takes the data root's writer lock; a data root another writer holds is refused, nothing written.
- Order of refusals, each exit 1, nothing written: `E_CREDENTIAL_MISSING`; `E_CREDENTIAL_EXPIRED` (removed); `E_CREDENTIAL_SHAPE` (removed); `E_CREDENTIAL_PASS_MISSING` (`ERAVON_CLI_PASS` unset or empty); `E_CREDENTIAL_PASS_UNKNOWN` (`detail` `FORM` or `NONE`, never the key); `E_CREDENTIAL_SHAPE` for a row holding another master key (removed); then the writer lock.
- Expiry is judged against the verb's `--now`; turning the switch off, or a new master key on the desktop, removes the file.
- "Through the desktop app" on with *Queue only*: a verb that signs or sends and opens the vault itself (`--passcode`, a passcode file, `mcp`'s key road) is refused once the vault opens: `E_V_DOWN`, `downToken` `E_CLI_QUEUE_ONLY`, exit 1, nothing signed or sent.
- Writing with the credential, or both switches off, every signature and send goes as sending automatically.
- A transaction sent straight from the command line keeps a sealed record in the machine folder; the same command run again with the same key asks the nodes about it before signing anything new.

## 13 · `mcp`

- `eravon mcp`: JSON-RPC 2.0 on stdin and stdout, one message a line; versions `2025-11-25`, `2025-06-18` (another asked is answered `2025-11-25`); capability `tools` (`listChanged` false); `serverInfo` name `eravon`; stdin closed, it exits 0.
- Methods: `initialize`, `notifications/initialized`, `ping`, `tools/list`, `tools/call`, `notifications/cancelled`; others `-32601`; not JSON `-32700`; malformed, or a number not a whole within 9007199254740991, depth past 128, a member twice: `-32600` with `data` (`token`, `at`); unknown tool or `arguments` not an object: `-32602`.
- Flags: `--rest-keys` (handed to each call whose verb takes it and that does not go through the door) and `--passcode-file` (the key road); any other is misuse.
- A call runs the tool's verb as this program in its own process: each parameter as the flag of that name, a repeatable one once per value, stdin empty, `mcp`'s environment, no deadline; calls are done one at a time in order.
- Parameters: a string, a repeatable one a list of strings; a path is an absolute path on this machine.
- Result: text the process's stdout; `structuredContent` `{"answer":…,"exit":…}`; exits 0, 1, 3, 4 are answers; `isError` for exit 2 (text its stderr lines) and for a parameter refused before running (`E_ARGS` and its flag, `exit` 2); no exit code (a crash, not started): `isError`, stderr text, no `structuredContent`.
- Seat tools that take `--desktop` name the data root by `desktop` (required); while this machine has the credential the call is `--data <desktop>` run here with it; otherwise `--desktop <desktop>` with the verb's `homeFlags` only, another parameter refused by name.
- Key road `eravon mcp --passcode-file <file>`: each such call is `--data <desktop> --passcode-file <file>` run here, before the credential and the desktop.
- `contract` takes nothing; the nine other tools that take no `desktop` take their verb's flags less `passcode`, `writer`, `proxy`, `roots`, `rest-keys`.
- Annotations: `readOnlyHint` true on tools that write, sign and send nothing; `destructiveHint`, `openWorldHint` on the tools that can send.
- Tools: one per verb, named as the verb, except `mcp`; each seat's `vault-` verbs other than `vault-derive`, `vault-accounts`, `vault-list`; each seat's `payout-set`, `backup-export`, `backup-restore`, `direct-check`, `direct-give-up`; `seller-pending-bump`; `buyer-import`. `tools/list` answers them.
- No tool takes a passcode, a key or a recovery phrase.
- Cancelling: a call not started is dropped; a read being done is stopped and not answered; a write runs to its end and is answered.
- Configuration from "Connect an AI app": `{"mcpServers":{"eravon":{"args":["mcp"],"command":"<full path of eravon>"}}}`; with "With a credential" on, the copy first adds a program key and carries `"env":{"ERAVON_CLI_PASS":"evp_<64 hex digits>"}`.
