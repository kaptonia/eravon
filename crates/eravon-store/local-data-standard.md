# Local data standard (ERAVON)

The family's standard for files an app seals on its own machine, as ERAVON does it: the family's envelope byte for byte,
with ERAVON's own magic and literals. This text says only what ERAVON's code does today; the code is
`crates/eravon-seal/src/envelope.rs` (the envelope, one implementation for the desktop and the command line),
`crates/eravon-seal/src/keybox.rs` (the keys) and `crates/eravon-vault/src/local.rs` (which files, where); the test
vectors are `crates/eravon-seal/tests/local_standard.rs`.

## 1 · A file's identity is its owner and its place

- **Owner.** A machine-directory file is the machine's (`machine`); a home file is its home's (`home:<number>`, 32
  lowercase hex digits drawn when the home is made and kept in the home's sealed label); the label itself is
  `home-label`, opened before the home's number is known.
- **Place.** The file's kind and its logical name inside its home or the machine directory (`settings.json`,
  `candidates.json`, `products/<id>/record.json`, …). Never its path or its name on disk: names on disk are keyed tokens,
  so moving the data folder, renaming everything under a new master key or restoring onto another machine leaves every
  identity as it was.
- **The digest.** 16 bytes: 8 from the owner (`SHA-256("eravon-local/owner" ‖ 0 ‖ owner)[..8]`), 8 from the place, keyed
  (`HMAC-SHA256(K, "eravon-local/place" ‖ 0 ‖ logical name)[..8]`, `K = HKDF-SHA256(D, info "eravon-local/place-key")`).
  A logical name can carry a public id (an entry's, a grant's); keyed, the heads on a locked disk cannot be matched
  against them.

## 2 · The envelope

| bytes | what |
|---|---|
| 15 | magic `eravon-local/2\n` (each product its own, in its family literal register) |
| 1 | algorithm: `2` = XChaCha20-Poly1305 with the place half keyed (a later algorithm takes another number and keeps the magic) |
| 1 + n | kind tag length, kind tag (ASCII, `eravon/local/<kind>`) |
| 2 | the kind's format version, big-endian (every kind is at 1) |
| 16 | identity digest (§1) |
| 24 | nonce, fresh for every seal |
| rest | ciphertext and 16-byte tag, under the local data key D = `HKDF-SHA256(M, info "eravon/local/v1")`, M the key vault's master key |

- The additional data is the whole head, nonce included: any byte of the head changed, or the file put in another place
  or home, does not open.
- The plaintext carries the owner and the logical name again: `u16 length ‖ owner text ‖ u16 length ‖ logical name ‖ the
  file's own bytes` (lengths big-endian). Once opened they are checked against what was asked for.
- D exists only while the key vault is open: locked, every read and write of a sealed file is refused (`LOCKED`).

## 3 · Reading

- One envelope, read as it is. A file that does not read is refused with one member (`LOCAL_SEAL`). Its evidence's first
  word names one of six reasons, a closed table:

| reason | when |
|---|---|
| `not-sealed` | not this envelope: no magic (a plain file, another product's), the first envelope an earlier version wrote (magic `eravon-local/1\n`), or the unkeyed algorithm 1 |
| `truncated` | the magic is there but the head, or the ciphertext's tag, is cut short |
| `other-kind` | sealed as another kind, or a kind tag this version does not know |
| `newer-version` | a format version or an algorithm past this version's |
| `swapped` | another file: the owner or place is not the one asked for |
| `unopenable` | another key, or a byte altered |

## 4 · Writing

- Only this envelope is written. There is no migration step: nothing is sealed again in bulk, and a file in any earlier
  shape (the first envelope, algorithm 1, a key vault or identity register of an earlier shape) is refused whole by name,
  never read as this one, and left on disk as it was.
- A file that is there but does not read is never written over: the write is refused by name and the file left as it was;
  only "not there" counts as not there.
- Writing lands through the one atomic write (a temporary name beside the target, owner-only, synced, renamed, the
  directory synced).
- Plaintext opened from an envelope is wiped once copied out.
- Left plain, and never read through the envelope: what is read before the key vault opens (the machine directory's
  pointer, the key vault `keys-<account>.json`, sealed inside by its own scheme, and its lock, the plain machine settings
  `machine.json` holding only appearance and language, the machine mark, a data root's lock file `.lock` and writer mark
  `.writer-mark`), and exports meant for other people (record bundles, key files, backups). Every other file is an
  envelope; the rest of the machine settings is the sealed `machine-<account>.ev-sealed`.

## 5 · Known limits

- An older copy of the same file put back in its place is not told.
- Two whole homes swapped, labels included, are not told by the envelope (opening a home checks its label's owner
  against the opening identity).
- There is no way back: a file this version writes does not read in a version before it (refused by name there, the file
  untouched).

## 6 · Test vectors

Local data key D: bytes `00 01 … 1f`. Nonce: bytes `40 41 … 57`. Home number: `0123456789abcdef0123456789abcdef`. Each
vector gives the owner, kind, logical name, plaintext, the identity digest and the whole envelope in hex;
`crates/eravon-seal/tests/local_standard.rs` holds them and checks them byte for byte, with two more tests for §3's
reasons.

### `global-settings`

- owner: `machine`
- kind: `eravon/local/global-settings`
- logical name: `settings.json`
- plaintext: `{"language":"zh"}`
- digest: `f79f610ff2e8a2aea07bc3a56d972d65`
- envelope:

```
657261766f6e2d6c6f63616c2f320a021c657261766f6e2f6c6f63616c2f676c6f62616c2d73657474696e67730001f79f610ff2e8a2aea07bc3a56d972d65404142434445464748494a4b4c4d4e4f5051525354555657d43e6811b3881078eaf48acdcae811fbfcdddeea792a3cf411139124676356f1719c205477e8ec3282935842d603b0361cd577cbd7a5d8adee
```

### `candidates`

- owner: `home:0123456789abcdef0123456789abcdef`
- kind: `eravon/local/candidates`
- logical name: `candidates.json`
- plaintext: `{"entries":[]}`
- digest: `3c43354f3dba5970b9cde35dc0cb8a12`
- envelope:

```
657261766f6e2d6c6f63616c2f320a0217657261766f6e2f6c6f63616c2f63616e6469646174657300013c43354f3dba5970b9cde35dc0cb8a12404142434445464748494a4b4c4d4e4f5051525354555657d41c6d1fbd854326bec6b48a9aaa52aaabdbcfa7773c35aa5b03ce713c3214a82f98600d31f7e210f0d195226eb3be90ef0e0ec605f6dbf2cdde6d9a67c025312909c22949ce15218c0c93f773f7b6515f4e0eaf97cb
```

### `product`

- owner: `home:0123456789abcdef0123456789abcdef`
- kind: `eravon/local/product`
- logical name: `products/p1/record.json`
- plaintext: `{"id":"p1"}`
- digest: `3c43354f3dba59706b3bde0f2186278e`
- envelope:

```
657261766f6e2d6c6f63616c2f320a0214657261766f6e2f6c6f63616c2f70726f6475637400013c43354f3dba59706b3bde0f2186278e404142434445464748494a4b4c4d4e4f5051525354555657d41c6d1fbd854326bec6b48a9aaa52aaabdbcfa7773c35aa5b03ce713c3214a82f98600d31f7e210e8c286236eafb985e8440dd940f7d1ffd98e6cda79c1233a21099116368967b8b421cd1174c98151f44a9628b9409eb602970c
```
