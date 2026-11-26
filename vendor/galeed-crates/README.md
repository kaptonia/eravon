# B9 galeed 三 crate 冻结快照

来源:galeed-desk 仓提交 `27208ce1`(2026-11-04,GALEED 已封的产品提交,含 `galeed/1` §7.4 退任同秒那一笔 `37d752bd`),2026-11-19 以 `git archive` 抽出 `crates/galeed`、`crates/galeed-roster`、`crates/galeed-evidence` 三 crate 加它们依赖的 `crates/galeed-os`(`galeed` 经路径依赖取系统熵;零依赖),以及 `.cargo/config.toml`(`galeed` 编译期以 `env!` 读的 `GALEED_DESK_TRACE_CAP_BYTES` 由它的 `[env]` 给)、`rust-toolchain.toml`、`Cargo.lock`、`FAMILY-LITERALS.md`;本案不改一字,改动只经 Desk 侧封块后重取重锁。本目录的 `Cargo.toml` 与本说明是本家写的接线档。

重建夹具:`cargo build --release -p galeed -p galeed-roster -p galeed-evidence`,得 `target/release/gl1`、`gro`、`gev`(HARNESS.md、HARNESS-ROSTER.md、HARNESS-EVIDENCE.md 契约)。

入锁闸:三夹具对 ../galeed-conformance 的 corpus、roster-corpus、evidence-corpus seed 1 至 4 零分歧零预测偏差,结果见 BASE-LOCK.json 的 galeedCrates 节。
