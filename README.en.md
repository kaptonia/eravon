# Eravon

[中文](README.md) | English

Version 0.1.0

Eravon lets a seller sell goods through payment links whose funds are held on chain until the order completes, with a
judge or a guild of judges named in the terms to rule a dispute. The seller publishes products and follows orders in
the desktop app; the buyer opens the payment link in the same desktop app, checks it step by step and pays. Every order
is settled by its own instance of the Eravon standard contract, which has no owner and no privileged address; no server
or account sits between the parties.

The command line `eravon` offers the same seller and buyer actions as the window, one verb per action
(`seller-<verb>`, `buyer-<verb>`), each answering one JSON on stdout. With `--desktop <data folder>` a verb is done by
the running desktop app that holds that folder, and a request that signs or sends first comes up in the window for you
to approve; once "With a credential" is on in the app's Settings > Command line and AI control, the command line and AI
apps can write this machine's data while the desktop does not have that home open, each with a program key of its own.
`eravon mcp` lets AI apps call these verbs through the Model Context Protocol (MCP); a passcode, a private key or a
recovery phrase is never a parameter, and a program running errands never holds a wallet or a key.

## Download

The 0.1.0 packages ([release page](https://github.com/kaptonia/eravon/releases/tag/v0.1.0)):

| Platform | Download |
|---|---|
| macOS (Apple silicon, macOS 11 or later) | [`ERAVON-0.1.0-macos-arm64.dmg`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/ERAVON-0.1.0-macos-arm64.dmg) · [`.pkg` installer](https://github.com/kaptonia/eravon/releases/download/v0.1.0/ERAVON-0.1.0-macos-arm64.pkg) |
| Windows 10 / 11 (x86_64) | [`ERAVON-0.1.0-windows-x86_64.zip`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/ERAVON-0.1.0-windows-x86_64.zip) |
| Linux (x86_64, glibc 2.31 or later) | [`eravon-app_0.1.0_amd64.deb`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/eravon-app_0.1.0_amd64.deb) · [`AppImage`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/ERAVON-0.1.0-x86_64.AppImage) |
| Source | [`eravon-0.1.0-src.tar.gz`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/eravon-0.1.0-src.tar.gz) |

Check the SHA-256 against [`SHA256SUMS.txt`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/SHA256SUMS.txt) first, then install as described under "Install" below.

## Platforms

- macOS on Apple silicon (`.dmg` and `.pkg` release packages)
- Linux x86_64 (`.deb` and AppImage release packages)
- Windows 10 and 11 on x86_64 (a zip package: unpack and run)

Every push to `main` and every pull request builds and runs `cargo test --workspace` on macOS, Linux and Windows
(`.github/workflows/test.yml`); until it has been run on a Windows machine, the Windows cell is not yet required to pass.

## Networks

One build knows the two networks of the known network table, Ethereum mainnet and the Sepolia testnet, and treats them
alike; there is no L2. The logic contract of the Eravon standard, `Eravon`, is deployed through the public keyless
CREATE2 deployer and has the same address on both; every order is one EIP-1167 minimal proxy instance of it (runtime
codeHash `0x9fcd87aa6d3f7b07234d4eba0cd8277506802498b41c377764ef05b3afb5583f`).

| Network | Chain id | Standard contract (logic) | From block | Judge registry |
|---|---|---|---|---|
| Ethereum mainnet (default) | 1 | `0x63c2C2a4b21Ad15d68F2A42e746feDB1D8470875` | 26496124 | `0x5da2894fc5260832330305f50f948401b3c904d8` |
| Sepolia testnet | 11155111 | `0x63c2C2a4b21Ad15d68F2A42e746feDB1D8470875` | 12010682 | `0x5da2894fc5260832330305f50f948401b3c904d8` |

Each home keeps the network it is on: chosen when the home is made (the first-run wizard's Network step, the
new-identity and import sheets, `--chain` on the command line's `seller-vault-init` and `buyer-vault-init`; the head row
of the table, mainnet, when none is chosen) and switched later in Settings > Network. A home reads its network's
contract, judge registry and nodes, and refuses by name an endpoint table that answers another chain. There is one
install and one data folder (`ERAVON`) whichever networks its homes are on.

On first start every network's list already holds its factory endpoints (two each, run by different operators);
Settings > Network adds and removes RPC nodes one at a time, adds services such as Etherscan or Blockscout under
"REST API mode", and sets the proxy.

On mainnet, publishing, paying and opening cases spend real ETH for gas, and settlement is in real tokens. The built-in
settlement tokens are USDT, USDC, USDS, DAI, PYUSD, FDUSD, EURC, USDe, WETH and WBTC on mainnet, and USDC, EURC, USDT,
WETH and RPT on Sepolia. To try the app out without spending real funds, choose "Sepolia testnet" when you make a home.

## Install

The 0.1.0 release provides macOS and Linux packages and a Windows zip; each carries the third-party licences
(`THIRD-PARTY-LICENSES.txt`).

**macOS.** Install `ERAVON.app` from the `.dmg` (drag it into Applications) or with the `.pkg` installer. The app is
self-signed and is not notarized by Apple. Files downloaded in a browser carry macOS's quarantine flag, and macOS refuses
to open them; on recent macOS, "Open Anyway" in System Settings > Privacy & Security does not always work either. Check
the SHA-256 with `shasum -a 256 <file>` first, then remove the quarantine flag in Terminal:

- With the dmg: before opening the dmg, run the command below, then open the dmg and drag `ERAVON.app` to Applications:

  ```
  xattr -d com.apple.quarantine ~/Downloads/ERAVON-0.1.0-macos-arm64.dmg
  ```

  If you have already dragged the app in, run this instead:

  ```
  xattr -dr com.apple.quarantine /Applications/ERAVON.app
  ```

- With the pkg: run the command below, then double-click the pkg to install:

  ```
  xattr -d com.apple.quarantine ~/Downloads/ERAVON-0.1.0-macos-arm64.pkg
  ```

Change the paths to wherever you downloaded the files.

macOS 11 or later is required. The command line is `ERAVON.app/Contents/MacOS/eravon`; the pkg also installs a copy at
`/usr/local/bin/eravon`. After installing from the dmg, drag the app into Applications first, then turn on "Enable the
eravon command" in the app's Settings > Command line and AI control (it places a link in `/usr/local/bin`, and the
system may ask for an administrator) to type `eravon` in a terminal. `eravon://` links are registered by `ERAVON.app`
and open it when clicked. Machine data lives in `~/Library/Application Support/ERAVON/`.

**Linux.** On x86_64, use the `.deb` or the AppImage. Install the `.deb` with
`sudo apt install ./eravon-app_0.1.0_amd64.deb`, which puts the window program `eravon-app` and the command line
`eravon` in `/usr/bin` and ERAVON in the applications menu. The AppImage installs nothing: make it executable with
`chmod +x ERAVON-0.1.0-x86_64.AppImage` and open the window with `./ERAVON-0.1.0-x86_64.AppImage window`; the command
line is inside but not exposed, and neither "Enable the eravon command" nor "Connect an AI app" is available in it;
without FUSE, run `./ERAVON-0.1.0-x86_64.AppImage --appimage-extract-and-run window`. Both need glibc 2.31 or later; the
packages are not signed, so check them with `sha256sum <file>` first. The system file dialog goes through the desktop
portal, so `xdg-desktop-portal` and a backend such as `xdg-desktop-portal-gtk` should be installed (the `.deb`
recommends both). `eravon://` links are registered for the current user each time the app starts. Machine data lives
in `~/.local/share/ERAVON/`.

**Windows.** On Windows 10 or 11 (x86_64) use `ERAVON-0.1.0-windows-x86_64.zip`: check the SHA-256 first in PowerShell
with `Get-FileHash <file> -Algorithm SHA256`, unpack it into any folder and double-click `eravon-app.exe`; the command
line is `eravon.exe` in the same folder (run it in a terminal). Nothing is installed and no runtime is needed. The
programs are not signed: on first start Windows says "Windows protected your PC"; click "More info", then "Run anyway".
On Windows 11 with Smart App Control on, unsigned programs are blocked outright and cannot be allowed one by one; turn it
off first in Windows Security › App & browser control › Smart App Control settings. `eravon://` links are registered for
the current user each time the app starts. Machine data lives in `%LOCALAPPDATA%\ERAVON\` by default; deleting the
unpacked folder does not delete it. To type `eravon` in any terminal, turn on "Enable the eravon command" in the app's
Settings > Command line and AI control, which adds the command line's folder to this user's `PATH`.

The user manual: [English](docs/manual.en.md), [中文](docs/manual.zh.md).

What changed in each version is in [`CHANGELOG.en.md`](CHANGELOG.en.md).

The command line's output shapes, exit codes, refusal reasons and flags are in [`CLI-SCHEMA.md`](CLI-SCHEMA.md).

## Build from source

Rust 1.97.1 (pinned by `rust-toolchain.toml`; rustup picks it up by itself), 2024 edition.

The same source builds on macOS, Linux and Windows. On Linux, install the build dependencies first (Debian / Ubuntu):

```
sudo apt install build-essential pkg-config libxkbcommon-dev libxkbcommon-x11-0 \
  libwayland-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev \
  libgl1-mesa-dev libegl1-mesa-dev xdg-desktop-portal xdg-desktop-portal-gtk
```

That is xkbcommon, Wayland, X11 (with its cursor, randr and input extensions), GL and EGL, and the desktop portal with
one backend.

Then build:

```
cargo build --release
```

This produces two binaries:

| Binary | What it is |
|---|---|
| `target/release/eravon-app` | The desktop app (packaged as `ERAVON.app` on macOS, installed as `eravon-app` on Linux, `eravon-app.exe` on Windows) |
| `target/release/eravon` | The command line |

The build also produces a few small programs: the libraries' check entry points `era`, `ert`, `erl`, `erd`, `erj`,
`erv`, `erk`, `erx`, `erc`, `ers`, `erz`, `erq` and `evf`, `eravon-literals-check` and `eravon-galeed-leg`, and the
packaging tool `eravon-pack`.

Running from source:

- Desktop app: `target/release/eravon-app window` opens the window (`--data <dir>` chooses the data folder, the
  `ERAVON` folder in the system's application data when it is left out; `--lang zh|en` the language). On macOS,
  `eravon-app bundle --at <dir>` lays out an `ERAVON.app` that registers the `eravon://` links.
- Command line: `target/release/eravon seller-<verb> ...` and `target/release/eravon buyer-<verb> ...`; every verb
  answers one canonical JSON on stdout. `eravon contract` prints the machine-readable verb book.

## Test

```
cargo test --workspace
```

## Packaging

| Script | Output |
|---|---|
| `packaging/macos/build.sh [--identity NAME]` | `ERAVON.app` in a `.dmg` and a `.pkg` (macOS) |
| `packaging/linux/build.sh` | Built natively on Linux: `.deb` and AppImage on x86_64; only the `.deb` on aarch64 (the AppImage step stops with an error) |
| `packaging/linux/cross-build.sh` | The same Linux x86_64 packages from another host, with zig and `cargo-zigbuild` (glibc 2.31 or later) |
| `cargo run --release --locked -p eravon-pack -- windows --out dist` | The Windows x86_64 package folder: window binary, command line, licences (built on Windows with the MSVC toolchain; `.github/workflows/package-windows.yml` runs exactly this) |

The macOS and Linux packages land in `dist/`, each with a `SHA256SUMS-<os>-<arch>.txt`; the Windows command lays out
the folder `dist/ERAVON-<version>-windows-x86_64/` only, with no zip and no checksum file (the zip on the release page
is made from that folder). Every package carries the third-party notices (`THIRD-PARTY-LICENSES.txt`, made from
`Cargo.lock` for the target) and nothing of the building machine's user name, owners, host or system version.

## Layout

| Path | Contents |
|---|---|
| `crates/app` | The desktop app `eravon-app`: seller and buyer seats, vault and keys, products and payment links, orders, disputes, exports, wallet and settings |
| `crates/eravon-cli` | The command line `eravon`: one verb per seller and buyer action, with `contract` and `mcp` |
| `crates/eravon-terms`, `eravon-link`, `eravon-dossier` | Checks of the three document specifications: terms documents and offers, link payloads and receipts, case dossiers; each with a check program (`ert`, `erl`, `erd`) |
| `crates/eravon-standard`, `eravon-chain`, `eravon-scan`, `eravon-send`, `eravon-net` | Chain access: the one adapter to the standard contract, endpoint tables and reads agreed across nodes, chain scans, sending transactions, the one transport (HTTP, HTTPS and proxies) |
| `crates/eravon-store`, `eravon-vault`, `eravon-seal`, `eravon-glue`, `eravon-os` | Storage and keys: the key store, passcode and recovery, sealing local files, storage primitives, what differs by operating system |
| `crates/eravon-goods`, `eravon-cands`, `eravon-payways`, `eravon-watch`, `eravon-orders`, `eravon-deliver`, `eravon-dispute`, `eravon-pay`, `eravon-import`, `eravon-after`, `eravon-wallet`, `eravon-home`, `eravon-export`, `eravon-statement`, `eravon-recheck` | One crate per product area: products and payment links, judges and guilds, payment methods and opening settings, arrival watch and deployment, orders, delivery, disputes, buyer payment, import, after-sale actions, wallet, home and notifications, dossier export, statements, the dossier's chain re-check |
| `crates/eravon-galeed`, `eravon-galeed-scan`, `eravon-roster`, `eravon-evidence`, `eravon-face`, `eravon-zikaron` | Thin layers over the family libraries: `galeed/1` and judge ledgers, guild rosters and candidate pools, evidence manifests, an identity's faces, and ZIKARON disclosure kits read only |
| `crates/eravon`, `eravon-literals`, `eravon-qr` | Shared basics (cryptography, hex, JSON), constants and the known network table, QR codes made and read back |
| `crates/eravon-ui` | The widget library and skin |
| `crates/eravon-pack` | Packaging pieces: the third-party notices generator, the steps that keep the building machine out of a package, laying out the Windows package, checking the AppImage runtime |
| `vendor/` | Frozen snapshots of the galeed and ZIKARON family libraries the product builds on |
| `base/` | The specifications the product implements (`base/docs/`), the standard contract's source with its pinned code digests, conformance corpora |
| `packaging/` | Packaging scripts for macOS and Linux, and the icons |
| `docs/` | The user manual in English and Chinese, the complaint file format (`complaint.md`) and engineering notes |
| `CLI-SCHEMA.md` | Output shapes, exit codes, refusals and flags of the command line |
| `BASE-LOCK.json` | The base lock: digests of the frozen base the product was built against |
| `FAMILY.md`, `family-components.json` | Components shared with the sister apps, and their versions |

No crate depends on `base/` by path.

## Specifications

| Specification | File |
|---|---|
| `eravon.terms/1` (terms documents, offers, orders, signatures, fees) | `base/docs/eravon-terms-v1.md` |
| `eravon.link/1` (payloads, receipts, case packs, settlement offers) | `base/docs/eravon-link-v1.md` |
| `eravon.dossier/1` (dossiers) | `base/docs/eravon-dossier-v1.md` |
| Eravon standard (the on-chain contracts as a `galeed.venue/1` venue) | `base/docs/eravon-standard-v1.md` |
| `galeed/1`, `galeed.roster/1`, `galeed.venue/1`, `galeed.papers/1` | `base/docs/galeed-v1.md`, `base/docs/galeed-roster-v1.md`, `base/docs/galeed-venue-v1.md`, `base/docs/galeed-papers-v1.md` |
| `courtevidence/1` (evidence manifests) | `base/docs/courtevidence-v1.md` |
| `eravon.complaint/1` (complaint files) | `docs/complaint.md` |

## License

MIT. See [`LICENSE`](LICENSE). The bundled fonts keep their own licences: Inter and JetBrains Mono (SIL Open Font
License 1.1, `crates/eravon-ui/fonts/OFL-Inter.txt` and `OFL-JetBrainsMono.txt`), and Noto Sans SC (SIL Open Font
License 1.1, `OFL-NotoSansSC.txt`, embedded on systems other than macOS). The full licence texts of the Rust
dependencies and other third-party parts ship inside every package and as `THIRD-PARTY-LICENSES.txt` at the root of the
source; the app's Settings > About lists them too.
