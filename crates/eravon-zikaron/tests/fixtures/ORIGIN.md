# Origin of the fixtures

`kits-index-sample.json` is the sample disclosure kit index ZIKARON gives in its description of kits and the index,
copied byte for byte: the compact
canonical form, members sorted by name, the first row in the older shape (no `link`, no `anchoredOn`), the second in the
newer one (with `link` and `anchoredOn`). Its digests and paths are made up, as the note says. The index form literal it
carries, `zikaron.kits-index/1`, is the one this crate compares against.

`zikaron-0.1.4-kit.zkit` (one file) and `zikaron-0.1.4-kit/` (a folder) are one record package (`kitId`
`0x760a7a38d1ce237cc4ef3ecb376fde1c80ae8fb9c6a789380543b1136f96b08f`) written by another writer: ZIKARON 0.1.4's released
command line (`/Applications/ZIKARON.app/Contents/MacOS/zikaron`, version 0.1.4), run on 2026-10-23 in a temporary root
of its own on the volume (`HOME` and `TMPDIR` there, no node, no chain, no network): `keygen`; `init`; `history --file
交付说明.txt`; `kit-export --file notes/delivery.txt=交付说明.txt --file b.txt=b.txt --note "A4 外来字节夹具"` (the default
`--as file`), and the same with `--as folder`.
