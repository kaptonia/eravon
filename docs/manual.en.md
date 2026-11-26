# ERAVON User Manual

This manual covers version 0.1.0 of the ERAVON desktop app (`eravon-app`) for sellers and buyers and, briefly, of the command line (`eravon`). Every deal is made in the desktop app: the seller publishes products, the buyer opens the payment link and pays, and neither goes through a web page.

## 1. What ERAVON is

ERAVON runs escrowed orders on Ethereum. Each order is a contract instance that holds the buyer's payment until the order ends. The terms of every order name a judge, or a guild from which a judge is drawn by lot, and that judge rules if the buyer and the seller disagree.

The roles:

- **Seller**. Sets up products in the desktop app. On publishing, the app writes the product's terms document, signs an offer for each judge or guild the seller chose, posts this version on the on-chain board, and gives the payment link (`eravon://open?e=<payload>`, format `eravon.link/1`) with its QR code. The seller sends the link to buyers, then follows each order: delivers, answers disputes, and settles or refunds. Each case's judge fee and deposit are pulled from the seller's vault when it opens.
- **Buyer**. Hands the payment link to the desktop app (clicked, pasted or scanned). The app reads this version from the board and checks it step by step; the buyer chooses a judge, signs the order, pays to the order's own address (the budget address), and the case opens. After delivery the buyer confirms receipt, or requests a refund within the confirmation window, and the judge rules over every tier of the terms.
- *Judge or guild.* Named in the terms and chosen by the buyer on paying. In a dispute the judge rules within the ruling window. The desktop app has no judge seat.

### 1.1 Facts about this release

- **Chain**. Ethereum mainnet and Sepolia, both alike, never an L2. A home's network is chosen when the home is made (the first run's **Network** step, the new-identity and import sheets; with none chosen, the head row of the table, Ethereum mainnet) and switched later in Settings > Network; the about page's **Network** row names it; each network keeps its own endpoint table, and the app reads, scans and lists the orders of the network you are on only. New products, payments, openings and every new transaction go through that network's current contract; cases on a past contract are hidden until **Show past contracts** is turned on in Settings > Network, and are then acted on under their own contract (section 5.5). An endpoint table that answers another chain is refused by name.
- *Judge registry.* Judge documents and guild rosters are read through the network's judge registry. It stands at one address on Ethereum mainnet and on Sepolia (`0x5da2894fc5260832330305f50f948401b3c904d8`).
- *Platform.* The desktop app is built from the same source for each system it supports (macOS, Linux and Windows). File choosers use the system's own file panel where the system has one (where it has none, drop the file onto the drop zone); on a Linux Wayland desktop the window takes dropped files too, and on Linux a place that takes a file or a folder first asks which. Reminders arrive as system notifications. `eravon://` links are registered by the app bundle on macOS; on Linux and Windows the app registers the scheme for the current user at each start. When the app is already open, a link clicked again goes to the open one, and no second window opens.

## 2. Building and starting

Version 0.1.0 is released as packages on the [release page](https://github.com/kaptonia/eravon/releases/tag/v0.1.0), with [`SHA256SUMS.txt`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/SHA256SUMS.txt) to check them; the README's Install section says how to install each:

- macOS 11 or later on Apple silicon: [`ERAVON-0.1.0-macos-arm64.dmg`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/ERAVON-0.1.0-macos-arm64.dmg) or [`ERAVON-0.1.0-macos-arm64.pkg`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/ERAVON-0.1.0-macos-arm64.pkg). The window opens from `ERAVON.app`; the command line is `ERAVON.app/Contents/MacOS/eravon`, and the pkg also installs `/usr/local/bin/eravon`. Data lives in `~/Library/Application Support/ERAVON/`.
- Linux x86_64 with glibc 2.31 or later: [`eravon-app_0.1.0_amd64.deb`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/eravon-app_0.1.0_amd64.deb) (the window from the applications menu, `eravon-app` and `eravon` in `/usr/bin`) or [`ERAVON-0.1.0-x86_64.AppImage`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/ERAVON-0.1.0-x86_64.AppImage) (run it with `window`). Data lives in `~/.local/share/ERAVON/`.
- Windows 10 and 11 on x86_64: [`ERAVON-0.1.0-windows-x86_64.zip`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/ERAVON-0.1.0-windows-x86_64.zip); unpack it and double-click `eravon-app.exe`, the command line being `eravon.exe` beside it. Data lives in `%LOCALAPPDATA%\ERAVON\`.
- The source: [`eravon-0.1.0-src.tar.gz`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/eravon-0.1.0-src.tar.gz), built as below.

### 2.1 Building

From the repository root:

```
cargo build --release
```

The Rust toolchain is pinned in `rust-toolchain.toml`; rustup picks it up automatically. The two programs this manual covers are placed in `target/release`:

- `eravon-app`: the desktop app.
- `eravon`: the command line.

### 2.2 Starting the desktop app

```
target/release/eravon-app window [--data <data folder>] [--lang en] [--url <link>]
```

- `--data` is the folder that holds your vault, keys, settings, orders and activity log; without it the app uses its `ERAVON` folder in the system's application data, as an installed app does. Use the same folder every time.
- `--lang` picks the interface language at start: `zh` (the default) or `en`.
- `--url` hands the app an `eravon://open?e=` link at start (a payment link or a settlement offer). When the app is already open on the same data folder, the link goes to the open one and this start ends.

Run from a terminal with no arguments, `eravon-app` reports a usage error and exits with code 2; from a terminal the window opens only through `window`.

*App bundle (macOS).* To launch ERAVON from Finder and let `eravon://` links open it, lay out an app bundle:

```
target/release/eravon-app bundle --at <folder> --data <data folder>
```

This writes `ERAVON.app` into `<folder>`. The bundle is unsigned and meant for local use. It remembers the data folder, so launching it needs no arguments. Clicking an `eravon://open?e=` link launches the bundle and hands it the link.

## 3. First run and security

### 3.1 The first-run wizard

When the data folder has no vault yet, the app opens the first-run wizard.

1. **Set a passcode**. Eight letters or digits (case counts), entered twice. A passcode of one character eight times is refused; an all-digit passcode that runs in sequence (such as `12345678`) or reads as a date is refused too, and the app says which rule failed.
2. **Create an identity**. Optionally give a **Label** first, then choose one way:
   - **Generate recovery words**: the app generates twelve recovery words. Press **Make sure no one can see your screen, then click to show**, copy the words in order onto paper, press **I have written them down**, then type the three words the app asks for and press **Confirm and create**.
   - **Import an existing key**: enter the **Recovery phrase** (all twelve words can be pasted into the first box). Only keys this app made are taken; a bare private key or a key file is not. Words alone restore only what can be read on chain.
   - **Restore from backup**: drop or choose a machine backup and enter the backup password. This replaces identities and data on this machine.
3. **Network**. Choose the network this seat's home is on (**Ethereum mainnet** or **Sepolia testnet**; the head row of the table starts chosen) and press **Next**; you can switch it later in Settings > Network.
4. **I am**. Choose **Seller** or **Buyer**; each choice has a line under it. You can switch at any time.
5. *Seller: Payout address.* The address that receives your sales. Press **Later** to skip; you can add it on the **Payment methods** page.
6. **Export backup**. Writes this machine's backup (the card shows its **Contents**: how many keys and records). Press **Do it later** to export it later from Settings > Local data.

A payment link that arrives before any identity exists (clicked, given by `--url` or pasted) waits: the top of the wizard says a link is waiting, **I am** starts on **Buyer**, and the import step can wait; when the wizard ends, the app goes back to the link. A link that arrives while the app is locked opens after you unlock.

**Finish** on the last step ends the wizard and opens the main window; unfinished steps can be continued in Settings > About.

### 3.2 Seats and keys

The app has two seats, **Seller** and **Buyer**, switched with the segment at the top of the left sidebar. Each seat has its own keys. The first time you open the other seat, a short wizard adds a key for it (identity step, then the network and that seat's step).

The identity chip at the top left shows the current key. Click it to switch keys in this seat, or to reach **New identity**, **Import key** and **Identity key**.

**Settings** > **Identity key** opens with the current identity card (name, seat and kind, **Copy address**; the card lists the **Key address**, **Gas balance** and **Backup status**), followed by **Passcode** (**Change passcode**, **Auto-lock**, **Idle time**), **Export** (**Export key file**, **Show recovery phrase**), **Identity** (**Primary**, **Set as primary**, **Switch identity**, **New identity**, **Import key**, **Rename**) and **Details**, with **Delete identity** at the foot (it deletes the current key; the primary identity cannot be deleted, and the passcode is asked first). **Switch identity** opens a sheet with one card per key in this seat; open a card to see its address in place and press **Switch**. **New identity** generates a new twelve-word key (**Generate 12 words**, then **Make sure no one can see your screen, then click to show**, **I have written them down**, type the three words asked for and press **Confirm and create**). **Import key** adds a key from its **Recovery phrase** and gives it a **Label**; a word not in the word list turns red at once. **Rename** changes only this key's name on this machine; its address and key stay the same. A key added this way is not protected by the passcode; only its twelve words can recover it.

**Rotate key** (Settings › Identity key): turn on **Keep identity** and it first lists what the rotation does (**Transactions to send**, **Cases moving to the new key**, **Paid orders to register**, **Offers to register**, **Re-signed to take deposits from the new key**); each paid order not yet opened can be switched to **Let it refund instead**, each product on sale to **Take it down instead**; where both seats share one set of words, the list and its count take in the other seat's orders and products too, and a seat that does not read is named. Confirmed, the old key declares and the new key accedes on the network this seat's home is on, and the new key's twelve words are shown once. The identity and its data stay where they are: the identity menu keeps as many rows, the seat's key in office becomes the new key, both seats together; the new twelve words are this identity's words (the primary identity recovers with them, and the old words open it no more). An identity of an imported single key changes to a new single key and is shown no words; its **Backup status** goes back to not backed up, and the card says to export its key file again or make a whole-machine backup again. What the old key signed and nobody submitted is void after the accession; the old key's history links to the new key publicly, for good. A chain the rotation stopped on shows **Resume rotation**. While the seller key holds any token this is said first: send it out in the wallet, or press **Change anyway**, the tokens then staying on the former key, moved back afterwards from **On former keys** in the wallet with **Move to current key**; a paid order not opened whose deposit was signed on the old key refunds at its deadline. A rotation left part way goes on. A whole-machine backup made part way through, restored on another machine and finished there, says that the new twelve words are not on that machine: the identity's words and recovery stay the old ones, so finish the rotation on the original machine and back it up again. Without keeping the identity it adds a new key as an identity of its own and switches to it, unrelated to the old one.

### 3.3 Passcode, locking and recovery

- **Lock** at the bottom of the sidebar (or Cmd+L; Ctrl+L on systems other than macOS) locks the app at once. Locking stops a payment in progress before its next transaction.
- **Auto-lock** (Settings > Identity and keys > Passcode) is on by default and locks after 15 minutes idle. **Idle time** offers 1, 5, 15, 30 or 60 minutes.
- **Enter passcode** appears at start and after locking. The check starts as the eighth character lands: the cells empty at once and take no keys while it runs. One line under the cells says **Checking**, and a refusal appears in the same place until you type again. After five wrong passcodes in a row the app shows **Locked**, and only recovery remains.
- **Forgot passcode** opens **Recover**; which routes it offers follows the primary identity's kind (read from the vault's header before unlocking):
  - A recovery-phrase primary: **Recovery phrase** first, typing the primary key's twelve words and then a new passcode twice; folded under it, **Restore from a key file** with a key file either of its seats exported and its file passcode.
  - An imported-key primary: **Restore from a key file**, with the key file it exported and its file passcode.
  - Either way, **Restore from backup** too: drop the machine backup and enter the backup password; this replaces identities and data on this machine. With no primary identity, only this route remains.
- **Change passcode** (Settings > Identity and keys > Passcode) asks for the current passcode and the new one twice.
- When the key store cannot be read, the lock screen has one way on: **Move this machine's data aside and start over**. Its confirming sheet says which folder moves (renamed beside it, nothing deleted), which data folder stays where it is, and **The way back**; **Move aside and start over** then runs first-run setup again, where a whole-machine backup or the recovery phrase restores. While another window or the command line is writing this machine's data, the sheet refuses by name (`MACHINE_HELD`) and moves nothing.

### 3.4 Backups and key files

In Settings > Identity and keys > Export:

- **Show recovery phrase** shows the primary key's twelve words after you enter the passcode.
- **Export key file** writes an encrypted key file to the folder under **Saved to** (your Documents folder until you press **Choose folder** at the sheet's lower left). Enter the passcode, then a file passcode of at least 8 characters twice. A key file brings back the primary identity only: when the passcode is forgotten, **Restore from a key file** takes the primary identity's file alone, and a file exported from another identity has nowhere to go; keep those with a whole-machine backup.
In Settings > **Local data** (which also shows the data folder):

- **Whole-machine backup**: **Export backup** writes a backup of identities and local data to the folder under **Saved to** (where the last backup went, else your Documents folder; **Choose folder** at the sheet's lower left changes it), protected by a backup password of at least 8 characters (a strength bar rates it); while it exports, the sheet stays open with a line at its foot naming the step (**Collecting**, **Encryption**, **Writing**, **Reading back**), closes once the backup is written, and takes neither back nor Esc meanwhile. **View backup** opens a backup with its backup password and lists only when it was made and what it holds, replacing nothing. **Restore from backup** replaces identities and data on this machine with a backup; anything not in the backup is lost. **Last backup** shows when you last backed up and how many changes have been made since. Recovery words alone restore only on-chain data, so keep a current backup.

Settings > **Command line and AI control** (terminal and AI agent automation) has two groups:

- **Terminal** > **Enable the eravon command** puts the `eravon` command line that ships with the app on your terminal's command path, so typing `eravon` in a terminal runs it (on macOS the system may ask for an administrator). Turning it off removes only what the switch made; if another install already occupies that place, the switch says so and leaves it alone.
- **Terminal** > **Connect an AI app** > **Copy config** copies the configuration that has an AI app start `eravon mcp` (`CLI-SCHEMA.md` section 13); paste it into the AI app's settings. With **With a credential** off the copy holds no program key. With it on, **Copy config** first asks for the passcode and adds a program key for that copy (its note **Connect an AI app** and the moment), then copies the configuration with the key in its `env`, as `"env":{"ERAVON_CLI_PASS":"evp_…"}` (the whole of it in `CLI-SCHEMA.md` section 13); a key not added copies nothing. A configuration without a key has its writes answered `E_CREDENTIAL_PASS_MISSING` while this machine writes alone. Where the command line cannot be reached from outside the app (a disk image opened in place, an AppImage), the key is grey.
- **Automation level** > **Through the desktop app** (works while the desktop is unlocked): while it is on, a verb given `--desktop <data folder>` is done by this window (section 8); while it is off, the answer is `E_DESKTOP` (`NOT_OPEN`, exit 4). The switch is kept on this machine and stays as it is after a restart.
- **Automation level** > **With a credential**: asks for the passcode and a validity (days, hours, minutes, or **Does not expire**), then lets the command line and AI apps write this machine's data without the passcode, until it expires or you turn it off (`CLI-SCHEMA.md` section 12); they write only while the desktop does not have this home open, and a write while it does is answered `E_CREDENTIAL_DESKTOP_HOLDS`: quit the desktop first, or turn **With a credential** off and write through the desktop. While it is on, **Through the desktop app** is greyed out and a verb given `--desktop` is answered `E_DESKTOP` (`ALONE`): leave out `--desktop`.
- **Program keys** (once **With a credential** is on): each program the command line runs for brings a key of its own. **Add a program key** asks for a note and the passcode and shows the key this once (copy it then; it is never shown again nor kept on disk); give it to that program in the environment variable `ERAVON_CLI_PASS`. Each row shows its note and when it was added; **Delete** takes that one key back at once and leaves the others. Turning **With a credential** off, or a new master key taking effect, removes the credential with every program key in it. An earlier version's credential (one key for every program) is no longer taken: unlocking removes it, turns **With a credential** off and says so once, and the command line answers `E_CREDENTIAL_SHAPE` and removes it; turn it on again and add program keys.

## 4. The main window

- *Sidebar.* Seller: **Home**, **Orders**, **Pending**, **Products**, **Judges and guilds**, **Export**, **Wallet**, **Payment methods**, **Notifications**. Buyer: **Home**, **Orders**, **Pending**, **Payment link**, **Wallet**, **Notifications**, **Export**. **Settings** and **Lock** sit at the bottom. **Notifications** shows a count of items needing action.
- *Status line.* The bottom of the sidebar says one thing at a time, the first that holds: a long action you started is in progress (its name); **Syncing…**; **No nodes set up**, **Node error** or **Nodes on another chain** (the nodes answer a chain other than the network you are on; the hint says to switch them back to that network's nodes); **Sync failed**; when the last sync was read (hour and minute on the same day, the month and day too on another, in the time zone chosen in Settings › General; past ten minutes the line turns amber); **Not synced yet**. Whether the nodes agreed and the block number are in Settings › Network. The key beside it refreshes; home and alerts are read again too.
- *Toolbar.* A back arrow appears on sub-pages, a forward arrow after going back. Page actions sit on the right; a detail page (an order, a product, the editor) carries its title and its keys in its own large title row.
- *Messages.* Results appear as a short message at the top, one at a time; an error takes two lines, what happened and then what to do next; a copy says only **Copied**. **Show details** shows the raw error, **Hide** closes it.
- *Shortcuts.* Cmd+1 to Cmd+6 open the first six sidebar items, Cmd+[ and Cmd+] go back and forward, Cmd+, opens Settings, Cmd+L locks, Esc closes sheets. Here Cmd (⌘) is the Command key on macOS and the Ctrl key on other systems.
- *Greyed-out actions.* When an action cannot run, a banner says why: **Locked; unlock to edit**, **Add a key in Settings › Identity and keys first**, **Add nodes in Settings › Network first**, **Node settings are invalid; fix them in Settings › Network**, or **Not synced yet; refresh first**. When the nodes are the cause, the banner ends with a **Network** key that opens Settings › Network.
- *Dropping an order file.* On the buyer seat, dropping an order file anywhere on the window opens **Payment link** > **Import** with that file selected (except while the pay page has a text page open, waiting for that text to be dropped in, section 6.1).
- *Links.* An `eravon://open?e=` link handed to the app (by `--url`, a click, or Cmd+V with no input field focused) lands on its page: a payment link on **Payment link** > **Pay**; a link to a settlement offer on the Dispute tab of the buyer's order, with the offer read. If another action is running, the link opens when it finishes.
- *QR codes.* The QR codes for payment links, receiving addresses, payments due and receipts are all drawn in one place; content too long for a code says **This content does not fit a QR code; copy it instead**.

### 4.1 Evidence kits (ZIKARON)

ZIKARON is a separate application that keeps evidence with a trusted time chain. When it is linked, its disclosure kits are listed in the evidence sheets of both seats, so you can submit a kit as evidence. Settings > **ZIKARON**:

- **Link ZIKARON** turns the link on or off (on by default). When off, nothing is probed and evidence sheets list no kits.
- **Machine directory** shows where ZIKARON was found (**Auto-discover**, or a folder set by hand); **Change** opens **Change directory**, where **Choose folder** at the sheet's lower left picks one and **Save** keeps it.
- **Details** says how it was found (**Manual**, **Pointer file** or **Default folder**), how many kits there are, or why it is not linked.

## 5. Seller

### 5.1 Settings to fill in first

- **Settings** > **Network**. First launch fills the list silently with every network's factory endpoints (two or more for each, run by different operators); the **RPC nodes** card lists them one per row, a key at each row's end removing one, and the cell under the card adds one (`https://…`, or `chainId=URL` for another network), one already listed not taken twice. The **Most questions per node** row takes a whole number from 1 to 1000 (256 by default); **Set limit** keeps it with the node list, the command line and AI apps read the same number, and a number outside that is not kept. The **REST API mode** switch is off by default; on, a **REST API** card appears: fill **Base URL** (starting with `https://`, no query string, such as Etherscan V2's `https://api.etherscan.io/v2/api`) and **API Key** to add a row; the key is kept only in this machine's sealed settings and goes with the whole-machine backup, the row showing its last four characters; a keys file named by the environment variable `ERAVON_REST_KEYS` when the desktop app starts (one `service key` per line) gives the key to a row whose settings hold none, and a service it gives a key for is added with **API Key** left empty; the command line takes the same file as `--rest-keys <file>` (`eravon mcp` hands it on to the commands it runs); the file is never written into the settings or a backup, and its keys are never shown; one service counts once and takes one row. Questions a REST service cannot answer (logs without an address, fee history, a balance at a given block) are not asked of it; those readings come from the other nodes. With the switch off, the rows stay and are not asked. The card shows the number of **Nodes**, the **Contract version** and the **Chain reading** (**Agreed**, **Single node** or **Not synced yet**); the details under it list the nodes verbatim, the **Contract** address and the **Block**. **Auto refresh** is under Settings > Notifications: every 60 seconds when the app is new; type the interval in seconds in the row's cell, an empty cell or 0 for **Manual only**, and press Return or **Save**; **Saved** stands beside the cell once it is saved. The interval governs only the timed refresh; unlocking, switching keys or nodes, your own actions and the refresh key still read the chain, and readings older than ten minutes are still marked old. Chain reads and transactions go only through this list, and its nodes must be on the chain of the network you are on. The contract version on the card and the contract address in the details are the current contract of the network you are on; it cannot change within a version. **Proxy** is one of three: **System** (the system's proxy settings, its automatic configuration script included), **Off**, or **Custom proxy**. A custom proxy takes its cells: **Type** (HTTP, HTTPS, SOCKS5, SOCKS4), **Host**, **Port**, **User name** and password (SOCKS4 carries a **User ID** and no password); a whole proxy address may be pasted into the host cell and is split into the cells when you leave it. Once saved, the address is shown without its password, the password cell is emptied and says **Password set**; saving the same proxy with the same user name and no password keeps the saved one; no password is written in the clear.
- **Payment methods** page > **Crypto wallet payment** > **Payout address**: shows the current key's payout address (shortened, with copy) or **Not set**. Press **Set address** when none is set, **Change** when one is; enter the address twice, then **Save**. Products on sale are re-signed after a change.
- **Settings** > **Identity key**: the identity card's **Gas balance** gives the signing key's ETH, which pays transaction fees, with when it was read (the last sync's reading); **Not read** until it has been read.
- **Settings** > **About** > **Run wizard again** opens this seat's wizard at the first step not yet done (the finished ones are checked and can be looked back at); finishing it writes only what that step leaves, such as the receive address.

### 5.2 Payment methods and opening settings

The **Payment methods** page holds only **Crypto wallet payment** (always on, **Default**): buyers pay to the order's own address. The card holds the **Payout address** (see 5.1).

**Settings** > **Opening settings** (seller seat only) has three items; a change applies to versions published after it, and links already out stay as they are:

- **Open on payment**: once the payment arrives, the app opens the case at unlock, and when you press **Sync**. Off, it opens nothing: the order says **Paid, waiting to open**, and **Open the case now** on the order page opens it.
- **Opening window**: days, hours and minutes; if no case opens within it after paying, the buyer can take everything back.
- **Let the buyer open, paying the gas**: after paying, the buyer can open the case. While the deposit is not below the price and this is on, the wallet page warns that any buyer opening pulls this deposit from the vault.

Each item is followed by **Save**.

### 5.3 Judges and guilds

The **Judges and guilds** page has two segments, **Judges** and **Guilds**.

- **Add judge** takes the judge's **Address** and a **Label**. **Add guild** takes the guild's **Pointer** (`galeed:guild:1:0x…`) and a **Label**.
- Each row shows the label, its address (shortened, so two entries of one name are told apart), a one-line summary (rules version and intake for a judge; members and mode for a guild) and a status: **Selectable**, **No quote**, **Order window too short** (a pool quote whose order window is shorter than this chain needs; it cannot be picked), **Needs re-signing**, **Malformed**, **Unreachable**, **Not verified** or **Retired**. See section 1.1 about the judge registry.
- Click a row for details. A judge has the tabs **Overview**, **Covenant**, **Quotes** and **Records**; a guild has **Overview**, **Covenant**, **Quotes** and **Members**. The overview shows, as the document writes them, its **Name**, **Home page** and description, and the full **Address** it was read at, marked **Has code · contract** or **No code · key**; a judge that moved its address lists each hop under **Addresses followed**, and the last one is read; succession declarations on its record that have not taken effect or were replaced, the one its current key has pending included, are listed under **Succession attempts** (as they stand for you to judge). Members beyond the document's table are listed as written; this app reads none of them. A guild's overview also gives the roster's **The guild's words**. A guild's **Members** tab lists each address the roster gives and the address its succession leads to (a member's name, where its document has one, before the address, never in its place), marked the same way; each member's home page, description, succession attempts and the roster element's members beyond `addr` are listed as written in that member's fold. Quotes show the **Judge fee**, **Litigation fee**, **Acceptance window**, **Ruling window**, **Resignation terms** (opening a page that says, case by case, where the money goes when the judge does not act), **Per-order cap** and **Acceptance**, with an example amount.
- **Edit label** then **Save label** changes the label. **Remove this** then **Remove** takes the candidate off your list; it is no longer offered for your products.
- When a candidate changes, it shows **Needs re-signing**, and links that use it are re-signed.

### 5.4 Products and payment links

The **Products** page lists products with name, status, judges and price, and has a search box. Statuses: **Draft**, **Unlisted**, **To sign** and **On sale**, the same words in the list and on the product page.

**New product** opens the editor with five tabs: **Basics**, **Delivery**, **Refund**, **Judges** and **Confirm**. **Cancel**, **Save** and **Publish** stand at the right of the page's title row; **Publish** is there on every tab and lights only on the **Confirm** tab once the terms are complete and the fees are shown; unlit, its hover names what is still missing.

- **Basics**: **Name**, **Text**, **Product page** (**optional**), **Images** (**optional**; **Add image** records only the image's name and SHA-256), **Price**, **Settlement token** (picked from the tokens of this chain registered under Settings > **Tokens**). The text is written in the Markdown editor: its toolbar has **Heading**, **Bold**, **Italic**, **List**, **Numbered**, **Quote**, **Code** and **Import text** (which replaces the whole text with a text file); the character count stands under it, with the **Text digest** under **Show details**. The product page and the images are written at the end of the text and frozen with it (how, at the end of this section).
- **Delivery**: **Delivery window** and **Confirmation window**, each in days, hours (0 to 23) and minutes (0 to 59); an empty cell is 0, and each window must be more than zero. **Delivery notes**, in the Markdown editor.
- **Refund**: **Refund tiers**, each with a name and the buyer's refund share, marked **Case fee on the buyer** or **Case fee on the seller** beside it. The first tier (0%) and the last tier (100%) are fixed; **Add tier** inserts tiers between them, up to 16 in total, and shares must increase. The case fee in a dispute is borne by the buyer at the first tier and by the seller at every other tier.
- **Judges**: two cards, **Judges** and **Guilds**; switch on the ones to offer. Each row gives its resignation terms. Only selectable candidates can be switched on.
- **Confirm**: **Basics**, **Delivery**, **Refund** and **Judges** summed up, each with **Revise** to go back to its tab; under them, the **Fees** each candidate charges at this price and the **Resignation terms**. Once saved with something missing, the top of the editor says **Draft · Still needed:** and what is missing.

**Save** stores the draft and signs nothing. **Publish** writes the terms document, signs an offer for each chosen candidate with your signing key, and posts this version on the on-chain board in one transaction (its fee paid from the signing key); the payment link comes with the post. Once the publish lands, the window turns to the product page, where the on-sale mark, the payment link and the post state are. If the app is locked when this happens, the product shows **To sign** and is signed after you unlock; signed and not yet posted, the product page says **This version is not on the board yet: the payment link comes with the post**, with **Post on the board**. Leaving the editor with unsaved changes asks **Save product?** (**Save**, **Discard**, **Keep editing**). A change publishes a new version with a new link; the current version and its link stay valid until the new version is published. **Publish only** signs this version and makes its listing and payment link without the board post; the post stays on the product page under **Post on the board**, for you to send when you choose.

The product page opens with the product's name, price and status, with **Edit** at the right; then come **Basics** (**Price**, **Delivery window**, **Confirmation window**, **Refund tiers**, **Judges**), **Description**, **Resignation terms** and the **Payment link** section, and at the foot the key that ends this version: **Delete draft** for a draft, **Take off sale on chain** while on sale, **List for sale** for a version unlisted here, **Publish a new version** once taken off sale on chain.

- **Description** has one row each for the **Goods description** and the **Delivery notes** (**Filled in** or **Not filled in** on a draft; **Full text on chain** or **Digest on chain only** once published); click a row for that text's page. When the text names a product page, a **Product page** row follows, opening in the browser, with a copy key at its end. The goods description's page lists each image's name and SHA-256; drop an image on it (or press **Choose a file**) and it is checked there and then.
- The link state: **Link open**, **Unlisted** or **No link** with the reason; under it, a posted version says it is on the board and gives the transaction.
- **Copy link** copies the payment link (`eravon://open?e=…`). **QR code** shows it as a QR code; At the card's lower left, **Save image** saves a PNG into a folder you choose (nothing already there under that name is overwritten), and **Copy image** puts the image on the clipboard.
- **Details** lists the link and the older links with their expiry dates. Links already sent take orders until they expire.
- **Take off sale on chain** opens a confirmation card: this version's item and price, **Payments after it** **Cannot open a case; the buyer can take the money back**, and orders already **Case opened** **Go on as before**. Confirming sends the revocation from the seller key, which pays the gas; after it the foot offers only **Publish a new version**, and selling again means changing the terms and publishing a new version.
- **Delete draft** then **Delete** deletes a draft. This cannot be undone.

The app re-signs products automatically when an order was pre-signed, a link is about to expire, a candidate changed, or the payout address changed (**Re-signed after unlock:** lists the reasons while locked).

*How the product page and the image digests are written.* The editor's two fields write them at the very end of the goods text as one fenced block, hashed and frozen with the text; the seller's product page, the buyer's pay page and the goods text frozen with a case in the dossier all read it the one same way, and the block is not drawn as part of the text. Writing it into the text by hand takes the same form:

````
```eravon.goods-refs/1
page https://shop.example/item
image <SHA-256 in 64 lowercase hex digits> Front view.png
```
````

- the block stands at the very end of the text, and the text has only one;
- `page <address>`: at most one line; the address is one run of non-blank characters, shown as written, and whether it opens is the app's open gate (only `http` and `https`);
- `image <SHA-256> <name>`: none or more lines; the digest exactly 64 lowercase hex digits, the name the whole rest of the line after one space (spaces and any script kept), not empty;
- blank lines inside the block are allowed. A block that breaks any rule is plain text: the text shows as the seller wrote it, carrying no goods page and no pictures, and nothing is refused; the description may hold anything (a bare hash too).

*Stages*: a sale delivered in stages is one product per stage: for the first stage pick **New series** under **Series id** in **Basics** (the app makes the id), for each later one pick that series, then fill in its own **Stage** and **Stages** (picking a series fills in its stages and next stage); the goods list says which stage of how many each one is. A series takes one candidate, the same on every stage; a later stage can be written only once the stage before is written and published, its opening window and expiry follow from that stage's (after the latest second it can still end), and the buyer cannot open it. Publish every stage of a series before you rotate your key: a later stage posted after the rotation can be paid by nobody.

### 5.5 Receiving orders

The **Orders** page has the segments **In progress** and **Done**, a search row (**Order no., buyer or item**, **Start date**, **End date**) and the columns **Order no.**, **Buyer**, **Item**, **Due**, **Amount** and **Status**. The list combines the orders recorded on this machine with those found on chain. An order whose seller deposit cannot stand the opening reads "To deploy · Seller deposit short", marked **Seller deposit short** in amber.

- **Paste receipt** (toolbar): paste the receipt the buyer copied to you and press **Paste**. The app checks it against your product records and opens the order.
- Click an order to open it. The detail page has the tabs **Overview**, **Delivery**, **Dispute** (shown once a dispute exists), **Records** and **Fees**.
- **Overview** opens with an overview card: the state and its deadline, with only the one key this moment asks for most at its right (**Deliver** when delivery is due, **Submit evidence** in a dispute, **Export** once ended, **Open the case now** when paid and waiting to open, no key otherwise), and the **Amount**, **Judge** and **Placed** under it. Below it, **Basics**: **Item**, **Status**, **Buyer** (press it to give the buyer a label on this machine, then **Save**), **Goods details** (**Frozen with the case**; it opens **Papers frozen with the case** at its **Terms** tab) and **Judge** (before the case opens, the candidate this order went to; after, it opens **This case's judge**). **Status** is listed only before the case opens. Then **Progress** (a shortfall, notes, a transaction in flight with its two keys, and the buyer signature cell) and the **Actions** group (the other keys, such as **Open the case now**, **Export** and **Deliver**; **Settle at deadline** once a deadline has passed), then **Details** (order number, amount in full, case ID, contract, buyer address, judge address and token contract). Where the chain's reading says this order takes a refund, **Issue a refund** stands at the foot.
- **Papers frozen with the case** is a full page with four tabs: **Terms** (**Key terms** and the full text), **Goods description**, **Delivery notes** and **Digests**; **Digests** lists each text's digest, **This case's covenant fingerprint**, **Seller's offer signature**, **Board posting transaction**, **Case ID** and **Contract**, each row with copy, and **Export all papers** opens the Export page. What this machine does not hold is said as such, never filled in.
- With an order open on the orders page, **Sync** at the lower left also reads that order once more from the chain. When the buyer's payment has fully arrived at the order's address, the app is unlocked and **Open on payment** is on, this round sends the transaction that opens the case, paid from your signing key, and the judge fee and deposit are pulled from the vault; off, it sends nothing until you press **Open the case now**. If the opening window passes with no case, the buyer can take the payment back.
- **Settle at deadline** (shown only once a deadline has passed): moves the order on by its terms. After the judge resigned and the confirmation window passed, you release the payment here.
- **Sync** at the lower left also scans: it searches the chain for orders you have not recorded (for example cases the buyer opened), rebuilds them from the opening transaction, and catches up on payments that arrived. The periodic sync only reads and sends nothing.
- If the buyer signed a confirmation and sent you the signature, paste it into **Buyer signature** in the **Progress** section and press **Verify and confirm**.
- Order status words follow the order's life: awaiting payment, awaiting deployment, awaiting acceptance by the judge, awaiting delivery, awaiting confirmation, confirmed and awaiting the other side's submission, in dispute, awaiting a redraw, completed, ruled, settled, refunded, lapsed, expired, cancelled and unlisted.
- *Cases on a past contract.* After the contract changes generation, cases on the past contract are not listed; turn on **Show past contracts** in Settings > Network (on the command line and from AI apps, `eravon <seat>-settings-past-cases --on true`) and they list in the orders table marked **Past contract**, read and written under their own contract. Keys for steps that generation lacks (settling by call, cancelling, the expiry refund, the draw, opening) are greyed, their hover saying **The past contract has no such step**; a past contract's listing opens no new case. While the switch is off, every write to such a case is refused with **A case on a past contract: turn on Show past contracts in settings to act on it**, nothing sent.

### 5.6 Deposit and delivery

Each case's judge fee and deposit are pulled from your vault, within its allowance, when the case opens (section 5.12); the order page no longer takes a deposit of its own. The **Deposit** row on the **Fees** tab shows the deposit this case holds.

- **Deliver** (available once the judge accepts): drop a file or folder on the sheet, or press **Choose a file or folder** at its lower left; the app computes a manifest of the files and says how many this delivery lists. For a physical delivery, drop the shipping proof or just write a note (carrier and tracking number). The **Note (optional)** is committed with the manifest and the buyer does not see it, so send the tracking number separately; the **Delivery method (this machine only)** stays on this machine. Then press **Deliver**. Only the manifest goes on chain; hand the texts to the buyer apart. The confirmation window starts; once it has passed, either side can release the payment with **Settle at deadline**.
- The **Delivery** tab shows **Delivered at**, the note and the files (click a file for its size and sha256), with the transaction and its block under **Details**. **Save a copy** on the **Save local originals** row copies the delivered files to a folder in Downloads; a delivery of a note alone has no such row.
- After an order ends released or ruled with a share for you, the **Delivery** tab lists three values for registering the licence in ZIKARON Desk: **Grantee**, **Work digest** and **Terms digest**, each with **Copy**.

### 5.7 Disputes, evidence, settlements and refunds

When the buyer requests a refund, the **Dispute** tab appears. Each key appears only while the contract takes that move at that moment; none is offered once the case is closed.

While a judge is seated on the order, its actions include **Complain about the judge**: write what you complain of, attach what you hold, pick a folder, and **Generate complaint file** signs an `eravon.complaint/1` file with this seat's current key (the format is in `docs/complaint.md`). Where it goes is read from the order itself: a guild seat goes to the guild the deal drew from, a judge seated by name to a guild whose roster on the registry lists the judge. The sheet shows the guild's card; you send the file by a way the card gives, and the app sends nothing. A judge in no guild gets no file.

- **Claim** shows when it was **Raised**, **Ruling due** and the **Buyer's litigation fee paid in**. The buyer's request names no tier: the judge rules over every tier of the terms.
- **Evidence** lists items from both sides. Your own items can be opened with **Open original**.
- The Dispute tab's **Actions** group has **Submit evidence** and **Settle**, and **Issue a refund** once the ruling window has passed.
- **Submit evidence**: choose a disclosure kit under **Kits on this machine** (or **Don't pick**), and/or drop files or a folder (**Choose a file or folder** at the lower left); add a **Description (optional)** for the judge and press **Submit evidence**. Only the evidence list's hash goes on chain; hand the originals to the judge and the other side apart.
- **Settle** works in two steps.
  1. **Step 1 of 2 · Make an offer**: enter the **Buyer share (%)** (0 to 100, up to two decimals) and how long it is **Valid for (days)** (**1 day**, **3 days**, **7 days** or **Enter your own**, 1 to 7), then **Make offer**. The offer is posted on the board in one transaction (keyed by this case's ID), its fee paid from the signing key. A share of 100% goes the full-refund way instead.
  2. **Step 2 of 2 · Wait for the buyer to sign**: the card gives the offer's link (`eravon://open?e=…`) with **Copy link**. The buyer's app can read it from the board by the case ID, or open the link you send. When the buyer returns a signature, paste it under the **Settle proposal** line on the Dispute tab and press **Verify and settle**; the buyer may also submit the settlement on chain.
- **Issue a refund** (on the Dispute tab and at the bottom of the Overview) opens **Confirm refund** with the **Order amount**, **Judge fee (from the buyer's share)**, **To buyer**, **Bond back** and, in a dispute, **Of which, fee back**. **Confirm refund** sends the refund. The order ends; this cannot be undone.
- **Entries** shows the judge's **Ruling** or **Resignation** entries with their full text.

#### Evidence delivery

Originals go to the judge encrypted to the recipient key the judge published in its own ledger; the channel is the party's own, and this window sends nothing and relays nothing.

- On the seller's Overview the **Judge** row opens the full **This case's judge** page (Overview / Covenant / Quote): the seated judge's latest judge document read live by address (a member drawn from a guild opens even when it is not on your candidate table, with the guild that drew them): name, home page, the **Judge's self-description**, the **Covenant** this case is bound to in full and the quote frozen when the case opened; with no judge seated yet, it says so. On the buyer's side the **Judge** row in an order's details opens the same page.
- The **Evidence delivery** card on the Dispute tab says whether the **Recipient key** is picked. The key is picked from the judge's ledger alone: when no single key can be picked, or the judge published none, it says why, exports nothing and never sends in the clear. Press **Read the judge and recipient key** when it has not been read.
- After **Submit evidence** succeeds (and after **Deliver** on the Delivery tab), press **Export sealed package** and pick a folder: every file the manifest lists and the manifest itself are each sealed into an `.age` file named as in the manifest (a disclosure kit's `zikaron-kit/` path kept). An original changed since it was submitted is refused and nothing is written; a case with more than one seated judge is refused by name.
- Hand the package to the judge by your own channel; the judge opens it with its own key and checks each item against the manifest. The buyer's order page has the same places and takes the same path.

### 5.8 Record and fees

- **Records** lists every on-chain event of the order with its time; click a row for the transaction hash.
- **Fees** lists the **Order amount**, **Judge fee** (estimated until the order ends), **Deposit** and **Litigation fee** (paid / due), and after the order ends the **Split** among buyer, seller, judge and the deposit returned to the buyer.

### 5.9 Export

The **Export** page (both seats) has two tabs, **Case dossier** and **Statement**.

1. **Choose order** opens a large sheet (keyword on top, from and to dates below, one row per order); an order missing a document on this machine is greyed out, with **Missing locally:** and what is missing at its right. The chosen order stands in the **Order** section, with **Change** beside it to pick another.
2. **Export** opens a sheet: **Save to** starts at the folder of the last export, **Choose folder** at the sheet's lower left changes it, and confirming exports.

- **Case dossier**: terms, signatures, transactions and ruling. Before writing, every transaction row is checked against the chain and the dossier format is self-checked; a failed self-check refuses the export. The result card leads with **Dossier exported; every row matches the chain** or **Dossier exported; the rows below differ or were not recomputed**, with the three counts; under it come **Terms and signatures**, **Transactions** (how many), **Final state**, **Board posting transaction**, **Attachments** (how many), **Ruling**, **Compliance page**, **Chain check** (one line for each row that is a **Mismatch** or **Not recomputed**) and **Self-check**; the paths written are under the result card's **Details**.
- **Statement**: read from the chain. The result lists the **Final state**, **Order amount**, **Judge fee** (**From the deposit**), **Deposit**, **Litigation fee** and **Fee source**, then each party in turn: **Buyer receives**, **Seller receives**, **Judge receives** and **Seller's litigation fee to buyer** (**Set when the case ends** until it closes), and the number of **Transactions**. When the terms carry a compliance block, a compliance page with legal identities and conversions (with source and time) is written too.

A finished order's **Export** button opens this page with the order selected.

### 5.10 Home and alerts

- **Home** shows four cards (**In progress** with the number to handle, **Income this month**, **Products** on sale, **Judges and guilds** choosable), the open orders, your selectable judges and guilds, and the **Activity** feed (click a row for its transaction hash).
- **Notifications** lists only what needs handling: delivery due, a buyer refund request, a redraw due, funds arrived, a seller deposit short, a stale price feed, the licence reminder, a payment link expiring or needing re-signing, and a backup that is behind (no machine backup ever made counts too). Click a row to go where it is handled (the order, Products, or Settings > Local data); **Mark handled** takes it off the list; with nothing left it says **Nothing needs your attention**. As a deadline approaches, the app sends a system notification once per deadline.

### 5.11 Company mode

Company mode lets you act for a company: each action that would send a transaction exports a proposal for the company to execute. It is off by default; the desktop settings do not list it for now, and the command line `seller-settings-company` and `buyer-settings-company` turn it on or off; once on, every page goes the company way described below.

- Turning it on takes `--company` (the company address) and checks the company's contracts on several nodes first, saying what failed if it cannot.
- **Time check** (on by default, `seller-settings-time-check` and `buyer-settings-time-check`): when on, an export whose deadline is shorter than the company's objection window is refused; when off, the app asks **Export anyway**.
- With company mode on, a product version is not signed. The product page shows **Company proposal**: **Export proposal** writes one registration proposal per candidate to your Downloads folder (registration cannot be undone), and the link shows **Awaiting company approval** until **Check registry** sees every registration.
- In orders, the delivery, refund, evidence and settlement sheets export a proposal (**Export proposal**) for the company to execute. A settlement offer goes on the board inside that proposal (the consent's registration and the post land together or not at all); this seat posts nothing for it.
- Where the terms do not let the buyer open the case, the company opens it too: the order page gives **Export the opening proposal**, whose call is the company's own `init` on the deployed instance, with the order's listing and receipt attached as originals (an undeployed instance is deployed first from this seat's key, which anyone may send; a guild order never opens by proposal). After the export the page lists the proposal's path and the **Originals attached**.

### 5.12 Seller wallet

The seller seat's **Wallet** holds the deposit's vault, the payout and the trade key:

- **Use the built-in vault**: on by default, the trade key is the vault: deposits stay on that key, with no registration. While on, the card opens into the **Built-in vault**: one row per token it holds; **Transfer in** gives the address and its QR code; **Send** takes the token and amount, and the **To address** twice (**To address again**, typed by hand). Turned off, the outside vault comes back: press **Set the vault** and enter the **Vault address**. Once the vault changes, the products on sale are to be re-signed.
- **Deposit**: each case's judge fee and deposit are pulled from the vault. The deposit card says one line, **The vault holds deposits in advance**. One card per settlement token of your published products: **Ready**, **Not registered** or **No allowance**, with **Deposit it stands now** (the smaller of the balance and the allowance), **Vault**, **Balance**, **Allowance to the contract** and **Registration**. Each case pulls the judge's fee and the bond from the vault when it opens; what is not paid to the judge or the buyer at the close goes back to the vault, and the allowance does not rise with it. The allowance is shared by every seller this vault registered; to withdraw, set the allowance to 0. With orders waiting for their case, the card says how many it stands (counted one after another, each by its own deposit; short, it says how many of how many waiting) and lists **Each waiting order's deposit**; an order whose deposit is not below its price, while the buyer may open, gets a hint of its own. After a new generation of the standard, a hint says the vault must allow and register with the new contract again.
- **Adjust allowance**: enter the allowance for the contract (whole tokens; the unit stands beside the field) and press **Adjust allowance**. The built-in vault sends it from the trade key, and says how much is short where the fee is not covered. With an outside vault the app works out the transactions the vault must send (the allowance, then the registration), each with a QR code and **Open in the vault's wallet**, signed by the vault's own wallet; after sending them, press **Read the vault again after sending**, and the chain's reading decides.
- **Receive**: the payout address that receives your sales, with **Set address** or **Change**; never the trade key.
- **Trade key**: the current key and its ETH. With the built-in vault off and tokens still on the key, each is listed with **Send**. Send the tokens out before changing the key.

## 6. Buyer

### 6.1 Paying in the desktop app

On the buyer seat, open **Payment link**. It has two tabs, **Pay** and **Import**. A payment link clicked, pasted or scanned opens straight onto **Pay**.

**Pay**

1. Paste the link (`eravon://open?e=…`) under **Paste a payment link** and press **Open link**. The app reads the version the link names from the board.
2. **Checks** lists each check with **Passed** or **Failed** and the reason: **Link content**, **Terms**, **Terms match**, **Succession book**, **Posted by the seller**, **Not revoked**, **Judge details**, **Quotes**, **Standard**, **Price source**, **Seller registration**, **Offer key current**, and **Balance** if the balance cannot be read. **Posted by the seller** fails when the seller did not post this listing, or posted it only after its key handed its record on; a candidate failing **Offer key current** is hidden (the terms let the buyer open, and the offer was signed by a key the seller handed on, with nothing in the registry). A candidate whose seller deposit is not in place can still be chosen; its row says the case opens once the deposit is in place. A guild candidate whose panel is too long for its opening or its draw to fit one transaction is refused by name before signing (`E_ACT_GAS`).
   Under **Basics**, **Whole history** opens a page on the seller's record on the chain (loading while it is read; what stopped it if it cannot be read): **Confirmed keys**, **Awaiting accession** (a key declared and not yet taken up), **Documents**, **Attempts** (done after a key handed its record on) and **Cases as a party**. The other side on the orders page, and each candidate on the payment link page, have their own **Whole history** row.
3. **Basics** shows the **Item** (**Private goods** when the terms carry only the description's hash), **Amount**, **Delivery window** (**Counted from the judge's acceptance**), **Confirmation window**, **Paying from** (your current key), its **Balance** and the **Seller** (its short address). If the balance is short, a banner gives the address to top up and the shortfall.
4. **Description** has one row each for the **Goods description** and the **Delivery notes**, saying **Full text on chain**, **Checked** or **Not held**; when the goods description names a product page, a **Product page** row follows. Click a row for that text's page: where you do not hold the text, drop in the text file the seller handed over (or press **Choose a file**) to check its hash, and a match is kept with the order, a mismatch said; a goods description you hold takes a dropped image to check against its digest. A missing text does not stop the signature, and the signing card says which text this order goes without.
5. **Refund tiers** gives each tier's **Buyer gets**, **Seller gets** and who bears the case fee; the key terms follow, with **Full terms** folded under them.
6. Under **Judge**, pick one candidate: each row gives its ruling window, the **Judge fee**, the litigation fee (paid up front only if you request a refund) and the **Seller deposit**; a candidate that cannot be chosen is grey and says why. A candidate quoting in a reference currency is marked **priced at opening** in its row, and under the candidates the page says once **Priced in a reference currency: the litigation fee and the deposit are converted from the price feed and fixed when the case opens**. For a guild seat, **Guild covenant** follows: the quote and the covenant text as read on chain, with **Signing this seat accepts this covenant; its text and its quote are read from the chain**. Then **Resignation terms** lists each candidate's acceptance window and ruling window; click a row for that candidate's resignation terms.
7. **Refund address** defaults to your current key's address; enter another and press **Use this address** to change it.
A later stage of a series: sign them in order, the stage before first; while the stage before has not ended, the page says before you pay that this stage opens only when that one ends with no share to you, and that its money comes back if that stage refunds you or this one expires, and says it again once paid. A stage that already has an order gets no second one: the page names that order, to carry on with on the orders page.

8. **Pay** opens a confirmation card (item, amount, windows, chosen judge, paying from, the judge fee, the litigation fee and the seller's deposit; for a guild seat the covenant can be unfolded). Pressing **Pay** there first signs the order and keeps its receipt and order file, then runs the checks before opening; when your current key holds enough of the token it pays from there, and where the terms let the buyer open, the case opens right after. The result shows **Paid and opened**, or what stopped it (for example a short balance, a short seller deposit, stale price data, or a version taken off sale). With a short balance the order is already signed: pay it from another wallet in **Orders** (section 6.2). If the order's address is already taken, the app signs again a second later; where the offer's expiry fixes the order, it refuses and you ask the seller for another version.

**Import**

- **Order file**: drop or choose the order file and press **Import**. The order's buyer key must be on this machine; an order consented by call lists read-only.
- Order files come only from you: drop one on **Drop the order file**, or click that box to choose one (the dialog opens in your Downloads folder).

With company mode on (section 5.11), **Pay** is replaced by a proposal: choose **Registered form** (pay and register; you open the order after the company executes) or **Direct** (payment and opening in one proposal), press **Export proposal**, and copy the **Receipt for the seller**; the proposal's originals carry the listing, the terms and the receipt. For the registered form press **Opening proposal** after the company has executed: it exports the opening proposal (deployment and opening called by the company), and the company's execution opens the case.

### 6.2 Buyer orders and after-sale actions

**Orders** has **In progress** and **Done** segments, a search row (**Order no., seller or item**, **Start date**, **End date**) and the columns **Seller**, **Item**, **Due**, **Amount**, **Fee shortfall** and **Status**.

Open an order for the tabs **Overview**, **Delivery**, **Dispute**, **Records** and **Fees**. **Overview** shows state, deadline, seller, amount, judge, placed date, key and whether the terms are saved (**Terms saved** or **Terms not saved**). Cases on a past contract follow the Past cases switch on the buyer seat too (section 5.5).

Before the case opens, the overview starts with a card that follows the budget address's reading:

- **To pay**: the **Pay** section gives a QR code for what is still owed (any wallet scans it, or **Open in wallet**), **Copy address**, and what has arrived against what is due.
- **Awaiting opening**: the **Open case** section gives the amount **Received** and the deadline for the opening. Where the terms let the buyer open, **Open the case myself** sends the deployment and the opening from here, and opening locks the deposit and fees into this case. An order already opened is said so and the card closes; a seller deposit short is said as **The seller's deposit is short, so this order's case can't open now**: wait for the seller to top it up, or cancel for a refund.
- **Not opened in time** (the opening window passed) or **Unlisted, can be taken back** (this version was taken off sale): the **Refund unopened order** section gives the amount received, and **Refund unopened order** returns it to the refund address, then says how much went back, where, and the transaction.
- The first two give **Cancel order** / **Cancel and refund**: before the case opens you can cancel, and whatever arrived goes back in full. The basics also list **Refund to** and the **Opening window** (refunded in full if not opened by then).
- With the order file on this machine, the basics give **Copy receipt** and **Receipt QR code** (hand it to the seller, who knows the order by it), before the case opens and after.

Once the case is open, the actions depend on the state:

- **Settle at deadline** (shown only once a deadline has passed, then first on the overview card): moves the order on or settles it by its terms; the confirm card lists what the buyer, the seller and the judge each receive. An order awaiting a draw gives it by the draw's blocks. When the delivery window passes with nothing delivered, press **Settle at deadline** first, then **Withdraw** the refund.
- **Confirm receipt** (awaiting confirmation): shows what goes **To the seller** and the **Judge fee**; **Release payment** pays the seller. This cannot be undone.
- **Hand out confirmation** (awaiting confirmation, an order whose key is on this computer): signs only, sending no transaction and spending no gas; the signed call goes to the seller, or anyone, to submit. Handing it out is consent to release: it is recorded on the order at once, the state reads **Confirmed, to submit**, and no refund request is offered from then on.
- **Request refund** (awaiting confirmation, within the confirmation window only): no tier to choose. The card gives the **Judge**, the **Case fee to advance** (paid now), the **Ruling range** (every tier of the terms), **If the judge does not rule** (the tier an unruled case lapses at, read from the quote the case was opened with) and **Apply by** (after that the price is released to the seller); press **Prepay and apply**, the fee goes from your current key into this order, and the request follows. The judge rules within the ruling window.
- **Submit evidence** (in dispute): on the sheet pick a disclosure kit and/or drop files or a folder, or press **Choose a file or folder** at the lower left; one line on the sheet names what was chosen (**None selected** until then); then press **Submit evidence**. Only the list's hash goes on chain; hand the originals over apart.
- **Accept settlement** (in dispute): press **Read from the board**, and the app reads the newest offer the seller posted under this case's ID; or paste the seller's `eravon://open?e=…` link and press **Read offer**. It shows **Buyer share**, **Valid until** and **Seller consent** (signed with the offer, or registered on the contract). Press **Sign and accept**, or **Accept by call**.
- **Withdraw** (after the order ends): enter a token address (empty for the settlement token), press **Check** to see what is **Withdrawable**, then **Withdraw**, which then says how much went where, and the transaction. This recovers overpaid or stray tokens.
- **Export** (finished orders) opens the Export page (section 5.9).

**Fees** shows the amount, the judge fee, after the case ends **Buyer receives**, **Seller receives**, **Judge receives** and **Seller's litigation fee to buyer**, and the network fees of the transactions this order sent.

### 6.3 Wallet

The buyer seat's **Wallet** works with the current key:

- The **Balance** overview card names the current key, with **Receive** and **Send** at its right; it lists each settlement token held and ETH (for fees), and **Details** shows the address and the exact ETH amount.
- After a key change, money paid to a former key (a refund, an expiry or a settlement of an order signed before) is listed under **On former keys**, one row per key; **Move to current key** moves it over, each token's fee paid by the current key (ETH less its own fee). ETH short of its own transfer fee is not listed. Without the window, the same move is `buyer-wallet-former-move --former <former key> --token <token> --send true`.
- **Receive** opens a sheet with the address, a QR code and **Copy address**.
- **Send**: choose the **Tokens**, enter the **Amount** and the **To address**, then **Send**. **All** fills in what you **Can send now** (for ETH, **This transfer's fee is set aside**).
- **Transactions** lists your transactions with time, order, type and amount. Click one for its status (**Confirmed**, **Reverted** or **To confirm**), network fee, hash and sender, with **Copy hash**.

### 6.4 Buyer home and alerts

**Home** shows **In progress** (with the number to handle), **Balance** and **Done**, your orders with a **Whose turn** column (**You**, **Seller**, **Judge** or **Anyone**), and the activity feed. **Notifications** lists only what needs handling: payment due, opening due, confirmation due, an expiry refund available, funds to withdraw, a settlement offer received, and orders whose to-do could not be read; click a row to go where it is handled, and with nothing left it says **Nothing needs your attention**. As a deadline approaches, the app sends a system notification once per deadline.

## 7. Languages, appearance and data

- **Language**. Settings > Language and time > **Language** switches between 中文 and English at once and saves the choice. The window starts in the language given by `--lang` (Chinese when omitted).
- **Time zone**. Settings > Language and time > **Time zone**: **UTC** or **Follow the system** (as a new app starts). Every moment the app shows (order details, home activity, alerts, the last backup, the last sync) is written in the zone chosen, and the order list's date filter cuts days in it; the line under the choice shows the moment now. When this machine's zone cannot be read, moments show in UTC and the page says so. Signed documents, answers and exported files keep UTC.
- **Appearance**. Settings > **Appearance** > **Theme**: **Light**, **Dark** or **Follow the system**. The choice is saved and applied at the next start once you unlock.
- *Data.* Everything the app keeps (vault, keys, identities, settings, product records, orders and the activity log) lives in the data folder given at start (Settings > **Local data** says where). A whole user folder copied to another disk or another user reads and writes the copy, never the original, and says once where the data came from the first time it opens. Saved originals and company proposals go to your Downloads folder by default, QR images to the folder you choose; key files and machine backups go to the **Saved to** folder chosen on their sheets.

## 8. Command line

The `eravon` program offers the same seller and buyer operations from a terminal, each a flat verb named with its seat: `eravon seller-<verb> [--flag value ...]` and `eravon buyer-<verb> [--flag value ...]`, and `eravon contract` gives the full verb list. Everything the desktop changes has a verb, except four things only the window needs (auto-lock, refresh period, language, appearance). Every verb prints exactly one JSON answer, and a refusal is an answer too; exit code 0 means yes, 1 a refusal, 2 misuse (no JSON then), 3 a series stopped part way (the transactions already sent listed in `txs`), and 4 that the chain, the network or the disk could not answer, a transaction sent and not in a block when the wait ran out included (`E_TX_NOT_YET`, with its hash `tx` and the seconds waited `count`; the same command run again asks about it by that hash first). With `--desktop <data folder>` a verb is done by the running desktop that holds that folder (Settings > Command line and AI control > **Through the desktop app** must be on): relative paths are made whole before they go, and a refusal exits as the same name exits run directly; a data folder another machine writes is not asked of the desktop and is answered as run directly (`E_STORE_OTHER_MACHINE`). While Past cases is off, a case on a past contract is refused by name to the command line's writing verbs too (`E_A1_PAST_CONTRACT`). The full reference is in `CLI-SCHEMA.md`.

While this window is unlocked and holds the data folder, a command line verb that writes the data folder or sends (`--send true`) is done by this window with its own unlocked key, so the command line needs no passcode. A request that signs or sends (an offer, a settlement, a delivery, a refund, a confirmation, a dispute or a sweep, and anything given `--send true`) first raises **The command line asks you to sign** in this window (a request that only reads, scans or forms its calls raises nothing and is done at once): it lists the **Action** and the **Arguments it came with**, and says **Decline a request you do not recognise: an agent only runs errands, whether to sign is yours**; only **Approve and sign** signs and sends, and **Decline this request** signs nothing and the command line gets a refusal. The **Pending** page lists direct sends whose end is not known (**Check it**, **Give it up**: only once it can never be included) and orders with a transaction out (click to open the order). Verbs that change the passcode or the vault are always done in this window.

*A note for agents.* A program running errands for someone (an AI agent included) that meets an `eravon://open?e=` link or an `eravon.link/1` payload hands it to the local app (`eravon-app window --url <link>`, or a command line verb through `--desktop`); the app checks it and the user approves on the desktop. An agent never holds a wallet or a key.
