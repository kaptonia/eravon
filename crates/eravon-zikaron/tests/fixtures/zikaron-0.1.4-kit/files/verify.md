# 怎么核这一包

1. 逐档算 sha256,对 `manifest.json` 里 `files` 与 `proofs` 各行的 `sha256`;
2. `entries/` 下每一档的 sha256 即它的档名(去 `.zk1`),也即 `entries` 表里那一行;
3. 每一枚条目按 zikaron/1 §4.3 判十三步;
4. 清单外的档一件也不该有。

本束:条目 2 枚 · 档 3 件 · 证明 0 件。

出包时已由 zikaron.kit/1 的 `verify_kit` 自验,KIT_OK 才落盘(不变式 9)。

# How to verify this kit

1. Compute each file's sha256 and compare it with the `sha256` of its row under `files` and `proofs` in `manifest.json`;
2. Each file under `entries/` is named by its own sha256 (less `.zk1`), which is also its row in the `entries` table;
3. Judge each entry by the thirteen steps of zikaron/1 §4.3;
4. There should be no file outside the manifest.

This kit: 2 entries · 3 files · 0 proofs.

Before it was written, the kit verified itself with `verify_kit` of zikaron.kit/1, and it was written only on KIT_OK (invariant 9).
