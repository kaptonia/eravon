# Changelog

[中文](CHANGELOG.md) | English

## 0.1.0

The first public release of ERAVON. A seller publishes products and gives out payment links from the desktop app; a buyer opens a link in the same app, checks it step by step and pays. Every order's payment is held by its own instance of the ERAVON standard contract until the order ends, and the judge or guild named in the terms rules a dispute. The contract has no owner, and no server or account sits between the parties. This release supports Ethereum mainnet and the Sepolia testnet, with packages for macOS, Linux and Windows.

### Seller

- **Products and payment links**: the editor has five tabs, "Basics", "Delivery", "Refund", "Judges" and "Confirm". The goods description and the delivery notes are written in a Markdown editor, and the product page and the images' SHA-256 are written at the end of the text and frozen with it; the delivery and confirmation windows are given in days, hours and minutes; refund tiers run from 0% to 100%, up to 16 of them. "Publish" writes the terms document, signs an offer for each chosen candidate with the signing key and posts this version on the on-chain board in one transaction, which brings the `eravon://` payment link and its QR code; "Publish only" makes the link first and leaves the board post for when you choose.
- **New versions and taking off sale**: a change to the terms publishes a new version with a new link, and older links take orders until they expire. After "Take off sale on chain", payments cannot open a case and the buyer can take the money back, while orders already opened go on as before. The app re-signs on its own when a link is about to expire, a candidate changes or the payout address changes.
- **Stages**: a sale delivered in stages is one product per stage, in one series, published in order; a later stage's opening window and expiry follow from the stage before.
- **Judges and guilds**: add a judge by address and a guild by pointer (`galeed:guild:1:0x…`), each with a label; every time one is opened its judge document or roster is read from the chain again. The details give the overview, the covenant, the quote (judge fee, litigation fee, acceptance window, ruling window, resignation terms, per-order cap) and the records or members; every row states its status, and only a "Selectable" candidate can be chosen for a product.
- **Opening settings**: "Open on payment", "Opening window" and "Let the buyer open, paying the gas", each applying to versions published after the change.
- **Orders**: the orders table combines the orders recorded on this machine with those found on chain, in "In progress" and "Done", searchable by order number, buyer, item and date. Pasting the buyer's receipt finds the order; "Sync" also scans the chain for orders not yet recorded and payments that arrived. The order page has the Overview, Delivery, Dispute, Records and Fees tabs, and "Papers frozen with the case" gives the full terms, the goods description, the delivery notes and every digest.
- **Delivery**: drop files or a folder and the app computes their manifest (`courtevidence/1`); only the manifest goes on chain, and a physical delivery can carry just a note with the carrier and tracking number. Once the confirmation window has passed, either side can release the payment with "Settle at deadline". When an order ends released or ruled with a share for you, the Delivery tab lists the three values for registering the licence in ZIKARON Desk.
- **Disputes, settlements and refunds**: submit evidence (a ZIKARON disclosure kit may go with it), with only the evidence list's hash on chain; settle in two steps, the offer posted on the board in one transaction and the buyer's signature pasted back; "Issue a refund" shows what each side gets before sending; the judge's ruling and resignation entries are shown in full.
- **Evidence delivery**: every file the manifest lists, and the manifest itself, is sealed into an `.age` file for the recipient key the judge published in its own ledger, for you to hand to the judge by your own channel; when no single recipient key can be picked, the app says why and never sends in the clear.
- **Complaining about a judge**: the seat's current key signs an `eravon.complaint/1` file, its destination read from the order itself; the page shows the guild's card, and you send the file.
- **Seller wallet**: the built-in vault (the trade key is the vault, with no registration) or an outside vault. One deposit card per settlement token gives the deposit it stands now, the balance, the allowance to the contract and the registration, and counts how many waiting orders it covers; "Adjust allowance" sends from the trade key, or, with an outside vault, lists the transactions the vault must sign with their QR codes. Each case's judge fee and deposit are pulled from the vault when it opens.
- **Export**: the case dossier (terms, signatures, transactions and ruling, every transaction checked against the chain and the format self-checked before writing) and the statement (read from the chain, with a compliance page when the terms carry a compliance block).
- **Home and notifications**: four overview cards, the open orders and the activity feed; notifications list only what needs handling, and a system notification comes once per deadline as it approaches.
- **Company mode**: turned on from the command line, every action that would send a transaction exports a proposal for the company to execute instead.

### Buyer

- **Opening a payment link**: a link clicked, pasted or scanned opens straight onto "Pay". The app reads the version the link names from the board and checks it step by step: link content, terms, terms match, succession book, posted by the seller, not revoked, judge details, quotes, standard, price source, seller registration and offer key current, with the balance when it cannot be read. "Whole history" gives the seller key's whole record on chain.
- **Checking the texts**: when the terms carry only the goods description's hash, drop in the text the seller handed over to check it, and a match is kept with the order; a goods description you hold takes a dropped image to check against its digest.
- **Choosing a judge and paying**: every candidate shows its ruling window, judge fee, litigation fee and seller deposit, and a guild seat its covenant. "Pay" on the confirmation card signs the order, keeps its receipt and order file, and pays from the current key; where the terms let the buyer open, the case opens right after. With a short balance the order is already signed and can be paid from another wallet by its QR code on the order page.
- **Before the case opens**: the overview card follows the budget address: while payment is due it gives the QR code and what has arrived against what is due; where the terms allow, "Open the case myself"; before opening, cancelling returns whatever arrived in full; an order not opened in time, or a version taken off sale, offers "Refund unopened order".
- **After the case opens**: "Confirm receipt" releases the payment to the seller; "Hand out confirmation" only signs, spending no gas, for the seller or anyone to submit; "Request refund", within the confirmation window, advances the litigation fee and the judge rules over every tier of the terms; in a dispute, submit evidence and read the seller's settlement offer from the board, then sign it or accept by call; "Settle at deadline" once a deadline has passed; "Withdraw" recovers overpaid or stray tokens after the order ends.
- **Import**: drop an order file and the order is rebuilt from the chain on this machine.
- **Wallet**: the current key's settlement tokens and ETH, a receiving QR code, sending ("All" sets aside the transfer's fee for ETH) and the transaction list; after a key change, money paid to a former key is listed under "On former keys", and "Move to current key" brings it over.

### Identity, keys and security

- **First-run wizard**: set the passcode, create an identity (generate twelve recovery words, import existing words or restore a whole-machine backup), choose the network, choose Seller or Buyer, give the payout address and export a whole-machine backup. A payment link that arrives before any identity exists waits, and the app goes back to it when the wizard ends.
- **Two seats**: the seller and the buyer each have their own keys, switched at the top of the sidebar; a seat can hold several keys and switch between them.
- **Passcode and locking**: the passcode is eight letters or digits, and one too easy to guess is refused with the rule it breaks; the app locks after 15 idle minutes by default; after five wrong passcodes in a row only recovery remains. A forgotten passcode is recovered with the primary identity's recovery words, its key file or a whole-machine backup.
- **Key rotation**: with "Keep identity" on, the app first lists the transactions to send, the cases moving to the new key and the orders and products to register; confirmed, the old key declares and the new key accedes, and the identity and its data stay where they are; the new key's twelve words are shown once.
- **Backups**: a whole-machine backup is encrypted with a backup password and can be exported, viewed and restored; a key file brings back the primary identity only. When the key store cannot be read, this machine's data can be moved aside to start over, with nothing deleted.

### Networks

- **Ethereum mainnet and Sepolia**: each home chooses its network when it is made and can switch later in Settings > Network; the app reads, scans and lists the orders of the network you are on only, and refuses by name an endpoint table that answers another chain.
- **Nodes**: the factory endpoints of every network are filled in on first start; RPC nodes are added and removed one at a time; "REST API mode" adds services such as Etherscan or Blockscout; chain readings are compared across the nodes, the card saying "Agreed" or "Single node"; "Most questions per node" bounds the questions each node is asked per read.
- **Proxy**: follow the system (its automatic configuration script included), off, or a custom proxy (HTTP, HTTPS, SOCKS5 or SOCKS4, with a user name and password if needed); no password is written in the clear.
- **Past contracts**: after the contract changes generation, "Show past contracts" in Settings > Network lists the cases on the past contract, read and written under their own contract.

### Command line and AI apps

- **The `eravon` command line**: every seller and buyer action is a verb named with its seat, every verb prints exactly one JSON answer, and the exit codes 0 to 4 each have their meaning; `eravon contract` gives every verb, flag and refusal. The full reference is `CLI-SCHEMA.md`.
- **Through the desktop**: with `--desktop <data folder>` the running desktop app does the verb with its own unlocked key; a request that signs or sends first raises "The command line asks you to sign" in the window, for you to approve or decline.
- **With a credential**: once "With a credential" is turned on with a validity in Settings > Command line and AI control, the command line and AI apps write this machine's data without the passcode while the desktop does not have that home open; each program brings a program key of its own (the environment variable `ERAVON_CLI_PASS`), and each key can be taken back alone.
- **AI apps**: `eravon mcp` offers these verbs through the Model Context Protocol (MCP), and "Connect an AI app" copies the configuration; a passcode, a private key or a recovery phrase is never a parameter.
- **Enable the eravon command**: one switch puts the command line that ships with the app on the terminal's command path.

### Interface

- Chinese and English, switched at any time; the time zone is UTC or follows the system; the theme is light, dark or follows the system.
- Reminders arrive as system notifications; with the app already open, an `eravon://` link clicked again goes to the open one.
- With ZIKARON linked, its disclosure kits are listed in the evidence sheets of both seats, to submit as evidence.

### Platforms

- macOS 11 or later on Apple silicon: `.dmg` and `.pkg`.
- Linux x86_64 with glibc 2.31 or later: `.deb` and AppImage.
- Windows 10 and 11 on x86_64: a zip to unpack and run.
- Every package carries the third-party licences. The macOS app is self-signed and not notarized by Apple, and the Linux and Windows packages are not signed; the README's "Install" section says what to do.

### Known limits

- Ethereum mainnet and Sepolia only; no L2.
- The desktop app has no judge seat.
- Company mode is not listed in the desktop settings for now; `seller-settings-company` and `buyer-settings-company` on the command line turn it on and off.
- The app carries no file for you: delivered texts, evidence originals, sealed packages and complaint files are handed over by each party through its own channel.
- A series of stages takes one candidate, and the buyer cannot open a later stage; a later stage posted after a key rotation can be paid by nobody, so publish every stage of a series before rotating the key.
- A case on a past contract lacks settling by call, cancelling, the expiry refund, the draw and opening, and a past contract's listing opens no new case.
- The command line does not change the four settings only the window needs: auto-lock, the refresh period, the language and the appearance.
- In the AppImage the command line is not exposed, and neither "Enable the eravon command" nor "Connect an AI app" is available; run straight from the dmg, both wait until the app is in Applications.
