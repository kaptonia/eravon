# ERAVON

中文 | [English](README.en.md)

版本 0.1.0

ERAVON 让卖家经付款链接出售商品：货款在链上托管到订单完结，条款里写明的一位判官或一个判官行会在发生争议时裁决。卖家在桌面客户端里发布商品、跟进订单；买家在同一个桌面客户端里打开付款链接，逐项核验后付款。每笔订单由它自己的一个 ERAVON 标准合约实例结算，这个合约没有所有者，也没有任何特权地址；买卖双方之间不经任何服务器或账户。

命令行 `eravon` 提供与窗口相同的卖家与买家操作，一个动作一个动词（`seller-<动词>`、`buyer-<动词>`），每个动词在标准输出上答一个 JSON。带 `--desktop <数据目录>` 时，动词交给正在运行、持着那个目录的桌面客户端去做，要签字或发送的请求先在窗口里弹出，由你同意；在应用的「设置 > 命令行与 AI 控制」里打开「经凭据操作」之后，命令行与 AI 应用可以在桌面没开着这一家时，凭各自的程序钥写本机数据。`eravon mcp` 让 AI 应用经模型上下文协议（MCP）调用这些动词；口令、私钥与助记词从不作为参数，替人跑腿的程序也从不持有钱包或私钥。

## 下载

0.1.0 的安装包（[发布页](https://github.com/kaptonia/eravon/releases/tag/v0.1.0)）：

| 平台 | 下载 |
|---|---|
| macOS（Apple 芯片，macOS 11 起） | [`ERAVON-0.1.0-macos-arm64.dmg`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/ERAVON-0.1.0-macos-arm64.dmg) · [`.pkg` 安装包](https://github.com/kaptonia/eravon/releases/download/v0.1.0/ERAVON-0.1.0-macos-arm64.pkg) |
| Windows 10 / 11（x86_64） | [`ERAVON-0.1.0-windows-x86_64.zip`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/ERAVON-0.1.0-windows-x86_64.zip) |
| Linux（x86_64，glibc 2.31 起） | [`eravon-app_0.1.0_amd64.deb`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/eravon-app_0.1.0_amd64.deb) · [`AppImage`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/ERAVON-0.1.0-x86_64.AppImage) |
| 源码 | [`eravon-0.1.0-src.tar.gz`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/eravon-0.1.0-src.tar.gz) |

下载后先用 [`SHA256SUMS.txt`](https://github.com/kaptonia/eravon/releases/download/v0.1.0/SHA256SUMS.txt) 核对 SHA-256，再按下文「安装」一节安装。

## 平台

- Apple 芯片的 macOS（提供 `.dmg` 与 `.pkg` 安装包）
- x86_64 的 Linux（提供 `.deb` 与 AppImage 安装包）
- x86_64 的 Windows 10 与 11（提供 zip 包，解压即用）

每次推到 `main` 和每个合并请求，都在 macOS、Linux 与 Windows 上构建并跑 `cargo test --workspace`（`.github/workflows/test.yml`）；在 Windows 实机上走过之前，Windows 一格还不要求通过。

## 网络

一次构建认得已知网络表里的两个网络，以太坊主网与 Sepolia 测试网，两者一视同仁，不上 L2。ERAVON 标准合约的逻辑合约 `Eravon` 经公共的无钥 CREATE2 部署器部署，两个网络上地址相同；每笔订单是它的一个 EIP-1167 最小代理实例（运行时 codeHash `0x9fcd87aa6d3f7b07234d4eba0cd8277506802498b41c377764ef05b3afb5583f`）。

| 网络 | 链号 | 标准合约（逻辑） | 起始区块 | 判官登记簿 |
|---|---|---|---|---|
| 以太坊主网（默认） | 1 | `0x63c2C2a4b21Ad15d68F2A42e746feDB1D8470875` | 26496124 | `0x5da2894fc5260832330305f50f948401b3c904d8` |
| Sepolia 测试网 | 11155111 | `0x63c2C2a4b21Ad15d68F2A42e746feDB1D8470875` | 12010682 | `0x5da2894fc5260832330305f50f948401b3c904d8` |

每个家记着自己所在的网络：建家时选定（首次启动向导的「网络」一步，新建与导入身份的单上，命令行 `seller-vault-init` 与 `buyer-vault-init` 的 `--chain`；不选即表上头一行，以太坊主网），之后在「设置 > 网络」里切换。家读所在网络的合约、判官登记簿与节点；节点表答的是别的链，即具名拒绝。不论家在哪个网络，都只有一份安装、一个数据目录（`ERAVON`）。

首次启动时，每个网络都已填好出厂节点（每网两家，运营方各不相同）；「设置 > 网络」里可以逐台增删 RPC 节点，打开「REST API 模式」添加 Etherscan、Blockscout 一类的服务，并设置代理。

在主网上发布商品、付款与开案都要花真实的 ETH 付 gas，结算用真实的代币。内置的结算币：主网为 USDT、USDC、USDS、DAI、PYUSD、FDUSD、EURC、USDe、WETH、WBTC；Sepolia 为 USDC、EURC、USDT、WETH、RPT。想不花真钱先试用，建家时选「Sepolia 测试网」。

## 安装

0.1.0 提供 macOS 与 Linux 安装包和 Windows 的 zip 包，每个包都附带第三方组件许可（`THIRD-PARTY-LICENSES.txt`）。

**macOS。** 从 `.dmg` 安装 `ERAVON.app`（拖进「应用程序」），或用 `.pkg` 安装包安装。应用是自签的，未经 Apple 公证。从浏览器下载的文件带有 macOS 的隔离标记，系统会拒绝打开；在较新的 macOS 上，「系统设置 > 隐私与安全性」里的「仍要打开」也不一定有效。先用 `shasum -a 256 <文件>` 核对 SHA-256，再在「终端」里去掉隔离标记：

- 用 dmg：打开 dmg 之前先运行下面这条，再打开 dmg，把 `ERAVON.app` 拖进「应用程序」：

  ```
  xattr -d com.apple.quarantine ~/Downloads/ERAVON-0.1.0-macos-arm64.dmg
  ```

  如果已经拖进去了，改为运行：

  ```
  xattr -dr com.apple.quarantine /Applications/ERAVON.app
  ```

- 用 pkg：先运行下面这条，再双击安装：

  ```
  xattr -d com.apple.quarantine ~/Downloads/ERAVON-0.1.0-macos-arm64.pkg
  ```

命令里的路径按你实际下载的位置改。

要求 macOS 11 或更新。命令行在 `ERAVON.app/Contents/MacOS/eravon`；用 pkg 安装时，另装一份到 `/usr/local/bin/eravon`；用 dmg 安装时，先把应用拖进「应用程序」，再在应用的「设置 > 命令行与 AI 控制」里打开「启用 eravon 命令」（它在 `/usr/local/bin` 放一个链接，系统可能要管理员确认），即可在终端里直接敲 `eravon`。`eravon://` 链接由 `ERAVON.app` 登记，点击即交给它。本机数据在 `~/Library/Application Support/ERAVON/`。

**Linux。** x86_64 用 `.deb` 或 AppImage。`.deb` 用 `sudo apt install ./eravon-app_0.1.0_amd64.deb` 安装，把窗口程序 `eravon-app` 与命令行 `eravon` 装进 `/usr/bin`，应用菜单里即有 ERAVON。AppImage 不用安装：`chmod +x ERAVON-0.1.0-x86_64.AppImage` 之后运行 `./ERAVON-0.1.0-x86_64.AppImage window` 打开窗口；命令行在包内但不直接暴露，「启用 eravon 命令」与「接入 AI 应用」在 AppImage 里也不可用；系统没有 FUSE 时运行 `./ERAVON-0.1.0-x86_64.AppImage --appimage-extract-and-run window`。两者都要求 glibc 2.31 或更新，包没有签名，先用 `sha256sum <文件>` 核对。系统文件对话框经由桌面门户，所以要装 `xdg-desktop-portal` 和一个后端，如 `xdg-desktop-portal-gtk`（`.deb` 把两者列为推荐依赖）。`eravon://` 链接由客户端每次启动时为当前用户登记。本机数据在 `~/.local/share/ERAVON/`。

**Windows。** x86_64 的 Windows 10 或 11 用 `ERAVON-0.1.0-windows-x86_64.zip`：先在 PowerShell 里用 `Get-FileHash <文件> -Algorithm SHA256` 核对 SHA-256，解压到任意文件夹，双击 `eravon-app.exe` 即开，命令行是同一文件夹里的 `eravon.exe`（在终端里运行）。不用安装，也不需另装运行库。程序没有签名，首次打开时 Windows 会提示「Windows 已保护你的电脑」，按「更多信息」再按「仍要运行」。Windows 11 开着「智能应用控制」时，未签名的程序会被直接拦下、不能单独放行，要先在「Windows 安全中心 › 应用和浏览器控制 › 智能应用控制设置」里关掉它。`eravon://` 链接由客户端每次启动时为当前用户登记。本机数据默认在 `%LOCALAPPDATA%\ERAVON\`，删掉解压的文件夹不会删掉它。要在任何终端里直接敲 `eravon`，在应用的「设置 > 命令行与 AI 控制」里打开「启用 eravon 命令」，它把命令行所在的文件夹加进本用户的 `PATH`。

用户手册：[中文](docs/manual.zh.md)、[English](docs/manual.en.md)。

各版本的更新见 [`CHANGELOG.md`](CHANGELOG.md)。

命令行的输出形状、退出码、拒因和参数见 [`CLI-SCHEMA.md`](CLI-SCHEMA.md)。

## 从源码构建

Rust 1.97.1（由 `rust-toolchain.toml` 钉定，rustup 会自动取用），2024 edition。

同一份源码在 macOS、Linux 与 Windows 上构建。在 Linux 上先装构建依赖（Debian / Ubuntu）：

```
sudo apt install build-essential pkg-config libxkbcommon-dev libxkbcommon-x11-0 \
  libwayland-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev \
  libgl1-mesa-dev libegl1-mesa-dev xdg-desktop-portal xdg-desktop-portal-gtk
```

即 xkbcommon、Wayland、X11（连同它的 cursor、randr 与 input 扩展）、GL 与 EGL，以及桌面门户和它的一个后端。

然后构建：

```
cargo build --release
```

会产出两个程序：

| 程序 | 是什么 |
|---|---|
| `target/release/eravon-app` | 桌面客户端（在 macOS 上打包为 `ERAVON.app`，在 Linux 上装为 `eravon-app`，在 Windows 上为 `eravon-app.exe`） |
| `target/release/eravon` | 命令行 |

构建还会产出几个小程序：各库的检查入口 `era`、`ert`、`erl`、`erd`、`erj`、`erv`、`erk`、`erx`、`erc`、`ers`、`erz`、`erq`、`evf`，`eravon-literals-check` 与 `eravon-galeed-leg`，以及打包工具 `eravon-pack`。

从源码运行：

- 桌面客户端：`target/release/eravon-app window` 打开窗口（`--data <目录>` 指定数据目录，不给即用系统应用数据里的 `ERAVON` 目录；`--lang zh|en` 指定界面语言）。在 macOS 上，`eravon-app bundle --at <目录>` 排出一个登记 `eravon://` 链接的 `ERAVON.app`。
- 命令行：`target/release/eravon seller-<动词> ...` 与 `target/release/eravon buyer-<动词> ...`，每个动词在标准输出上答一个规范 JSON；`eravon contract` 打印机器可读的动词表。

## 测试

```
cargo test --workspace
```

## 打包

| 脚本 | 产出 |
|---|---|
| `packaging/macos/build.sh [--identity NAME]` | 装在 `.dmg` 里的 `ERAVON.app` 和一个 `.pkg`（macOS） |
| `packaging/linux/build.sh` | 在 Linux 上本机构建：x86_64 出 `.deb` 和 AppImage；aarch64 只出 `.deb`（AppImage 一步报错停下） |
| `packaging/linux/cross-build.sh` | 在别的主机上用 zig 和 `cargo-zigbuild` 交叉打出同样的 Linux x86_64 包（glibc 2.31 起） |
| `cargo run --release --locked -p eravon-pack -- windows --out dist` | Windows x86_64 包的文件夹：窗口程序、命令行、许可（在 Windows 上用 MSVC 工具链构建；`.github/workflows/package-windows.yml` 即调这一条） |

macOS 与 Linux 的包落在 `dist/`，各附一份 `SHA256SUMS-<系统>-<架构>.txt`；Windows 那一条命令只排出文件夹 `dist/ERAVON-<版本>-windows-x86_64/`，不打 zip，也不出校验和文件（发布页上的 zip 由这个文件夹压成）。每个包都带第三方许可声明（`THIRD-PARTY-LICENSES.txt`，按目标平台从 `Cargo.lock` 现生），不带构建机的用户名、属主、主机或系统版本。

## 目录

| 路径 | 内容 |
|---|---|
| `crates/app` | 桌面客户端 `eravon-app`：卖家席与买家席、金库与钥、商品与付款链接、订单、争议、导出、钱包与设置 |
| `crates/eravon-cli` | 命令行 `eravon`：每个卖家与买家操作一个动词，以及 `contract` 与 `mcp` |
| `crates/eravon-terms`、`eravon-link`、`eravon-dossier` | 三份文书规范的核验：条款文档与要约、链接载荷与回执、案件卷宗；各带一个检查程序（`ert`、`erl`、`erd`） |
| `crates/eravon-standard`、`eravon-chain`、`eravon-scan`、`eravon-send`、`eravon-net` | 链访问：标准合约的唯一适配层、节点表与多节点一致读、链上扫描、交易的发送、唯一的传输层（HTTP、HTTPS 与代理） |
| `crates/eravon-store`、`eravon-vault`、`eravon-seal`、`eravon-glue`、`eravon-os` | 存储与钥：密钥库、口令与恢复、本机文件的加封、落盘的原语、随操作系统而异的几样 |
| `crates/eravon-goods`、`eravon-cands`、`eravon-payways`、`eravon-watch`、`eravon-orders`、`eravon-deliver`、`eravon-dispute`、`eravon-pay`、`eravon-import`、`eravon-after`、`eravon-wallet`、`eravon-home`、`eravon-export`、`eravon-statement`、`eravon-recheck` | 各产品区，一区一个 crate：商品与付款链接、判官与行会、付款方式与开案设置、到款与部署、订单、交付、争议、买家付款、导入、售后、钱包、首页与通知、卷宗导出、对账单、卷宗的链上复核 |
| `crates/eravon-galeed`、`eravon-galeed-scan`、`eravon-roster`、`eravon-evidence`、`eravon-face`、`eravon-zikaron` | 家族库之上的薄层：`galeed/1` 与判官账本、行会名录与候选池、证据清单、身份的面，以及只读的 ZIKARON 披露包 |
| `crates/eravon`、`eravon-literals`、`eravon-qr` | 共用的底层（加密、十六进制、JSON）、常量与已知网络表、二维码的生成与回读 |
| `crates/eravon-ui` | 组件库与外观 |
| `crates/eravon-pack` | 打包件：第三方许可声明生成器、让包里不带构建机信息的几步、Windows 包的排布、AppImage 运行时的核对 |
| `vendor/` | 产品所依的 galeed 与 ZIKARON 家族库的冻结快照 |
| `base/` | 产品实现的规范（`base/docs/`）、标准合约的源码及其钉定的代码摘要、一致性语料 |
| `packaging/` | macOS 与 Linux 的打包脚本与图标 |
| `docs/` | 用户手册（中英两份）、投诉文件的格式（`complaint.md`）与工程说明 |
| `CLI-SCHEMA.md` | 命令行的输出形状、退出码、拒因与参数 |
| `BASE-LOCK.json` | 基底锁：产品据以构建的冻结基底的各项摘要 |
| `FAMILY.md`、`family-components.json` | 与姊妹应用共用的组件及其版本 |

没有任何 crate 按路径依赖 `base/`。

## 规范

| 规范 | 文件 |
|---|---|
| `eravon.terms/1`（条款文档、要约、订单、签名、费用） | `base/docs/eravon-terms-v1.md` |
| `eravon.link/1`（载荷、回执、案件包、和解要约） | `base/docs/eravon-link-v1.md` |
| `eravon.dossier/1`（案件卷宗） | `base/docs/eravon-dossier-v1.md` |
| ERAVON 标准（链上合约，作为一处 `galeed.venue/1` 场所） | `base/docs/eravon-standard-v1.md` |
| `galeed/1`、`galeed.roster/1`、`galeed.venue/1`、`galeed.papers/1` | `base/docs/galeed-v1.md`、`base/docs/galeed-roster-v1.md`、`base/docs/galeed-venue-v1.md`、`base/docs/galeed-papers-v1.md` |
| `courtevidence/1`（证据清单） | `base/docs/courtevidence-v1.md` |
| `eravon.complaint/1`（投诉文件） | `docs/complaint.md` |

## 许可

MIT，见 [`LICENSE`](LICENSE)。随附的字体保留各自的许可：Inter 与 JetBrains Mono（SIL Open Font License 1.1，`crates/eravon-ui/fonts/OFL-Inter.txt`、`OFL-JetBrainsMono.txt`），Noto Sans SC（SIL Open Font License 1.1，`OFL-NotoSansSC.txt`，在 macOS 以外的系统上内嵌）。Rust 依赖与其他第三方组件的许可全文随每个安装包附带，也放在源码根目录的 `THIRD-PARTY-LICENSES.txt`；应用的「设置 > 关于」里同样列出。
