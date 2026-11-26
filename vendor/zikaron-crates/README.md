# ZIKARON 核心 crate 冻结快照

来源:zikaron-desk 仓提交 `c07358e1`(2026-11-19),2026-11-19 以 `git archive` 抽出 `crates/zikaron` 整个 crate(包名 `zikaron`,`zikaron/2` 核;含 `entry`、`v2::entry` 与它们所依的 `cryptox`、`hexfmt`、`json`、`tokens`,以及 `trace`、`audit`、`v2::audit`、二进制 `zk1`、`zk2` 与两枚测试档),以及 `.cargo/config.toml`(`zikaron` 编译期以 `env!` 读的 `ZIKARON_TRACE_CAP_BYTES` 由它的 `[env]` 给)、`rust-toolchain.toml`、`Cargo.lock`;本案不改一字,改动只经来家侧再取重锁。来家的语料不入本家基底:这份快照的判据由来家自己的收敛闸守。本目录的 `Cargo.toml` 与本说明是本家写的接线档。

本快照不是根工作区成员(根 `Cargo.toml` 的 `exclude = ["vendor"]`);本家的 crate 要引它时以路径依赖引,依赖键名 `zikaron-core`(码里 `zikaron_core`),与本家核 `eravon` 分开。cargo 编 `vendor/` 下被引的 crate 时只读根工作区的配置,故根 `.cargo/config.toml` 另给一份 `ZIKARON_TRACE_CAP_BYTES`,值照本快照那一份。

单独编:把本目录拷到别处(本目录自带的 `Cargo.lock` 是来家整个工作区那一份,在此处起 cargo 会把它改写),在拷贝里 `cargo build --release -p zikaron`。

怎样重建:`git -C <zikaron-desk> archive c07358e1 crates/zikaron .cargo/config.toml rust-toolchain.toml Cargo.lock | tar -x -C <空目录>`,再放回本家写的这两档;`vendor/zikaron-crates` 与 `base/zikaron-crates` 两份逐字节同,逐档摘要记在 `BASE-LOCK.json` 的 `files`(`zikaron-crates/*` 各行),来源记在 `zikaronCrates` 节。
