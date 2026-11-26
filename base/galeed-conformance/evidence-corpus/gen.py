#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
gen.py - corpus generator for the `courtevidence/1` convergence run.

Written from docs/courtevidence-v1.md and galeed-conformance/HARNESS-EVIDENCE.md.
The canonical form, the signing primitives and the entry builder are borrowed
from ../corpus/gen.py (the galeed/1 generator's own reading, never the frozen
core); every prediction of this law is written by hand from the text (or
`null` where the case is harness misuse and the expected outcome is exit 2
with no output), a third opinion beside the criterion and the
implementations under test, never authoritative.

Usage:  python3 gen.py --seed 1
Populates ./manifests, ./files, ./audits and writes manifest.json.
"""
import argparse
import hashlib
import json
import os
import random
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
if not os.path.isfile(os.path.join(HERE, "..", "corpus", "gen.py")):
    sys.exit("this generator borrows ../corpus/gen.py, the galeed/1 generator, which is not beside it")
sys.dont_write_bytecode = True  # the criteria's trees stay as committed
sys.path.insert(0, os.path.join(HERE, "..", "corpus"))
import gen as G  # noqa: E402

J, JObj, cbytes = G.J, G.JObj, G.cbytes
eid = G.eid
N_ORDER = 0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141
SPEC = "courtevidence/1"
MANIFEST = []
_n = {"manifests": 0, "files": 0, "audits": 0}
RNG = random.Random(1)
KEYS = {}
BECH32 = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"


def key(tag):
    if tag not in KEYS:
        KEYS[tag] = RNG.randrange(1, N_ORDER - 1)
    return KEYS[tag]


def addr(tag):
    return G.privkey_to_addr_hex(key(tag))


def H32():
    return "0x%064x" % RNG.getrandbits(256)


def sha(b):
    return "0x" + hashlib.sha256(b).hexdigest()


# --------------------------------------------------------------------------
# Bech32 (BIP-173) for age recipients
# --------------------------------------------------------------------------

def _polymod(values):
    gen = (0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3)
    chk = 1
    for v in values:
        b = chk >> 25
        chk = ((chk & 0x1ffffff) << 5) ^ v
        for i in range(5):
            if (b >> i) & 1:
                chk ^= gen[i]
    return chk


def _hrp_expand(hrp):
    return [ord(c) >> 5 for c in hrp] + [0] + [ord(c) & 31 for c in hrp]


def bech32_encode(hrp, data5):
    poly = _polymod(_hrp_expand(hrp) + data5 + [0] * 6) ^ 1
    chk = [(poly >> 5 * (5 - i)) & 31 for i in range(6)]
    return hrp + "1" + "".join(BECH32[d] for d in data5 + chk)


def to5(b):
    acc, bits, out = 0, 0, []
    for byte in b:
        acc = (acc << 8) | byte
        bits += 8
        while bits >= 5:
            bits -= 5
            out.append((acc >> bits) & 31)
    if bits:
        out.append((acc << (5 - bits)) & 31)
    return out


def recipient(tag):
    """A valid age X25519 recipient spelling over 32 seeded bytes."""
    raw = bytes(RNG.randrange(256) for _ in range(32))
    return bech32_encode("age", to5(raw))


# --------------------------------------------------------------------------
# writing
# --------------------------------------------------------------------------

def write(cat, slug, data, ext=".json"):
    _n[cat] += 1
    name = "%s%03d_%s%s" % (cat[0], _n[cat], slug, ext)
    path = os.path.join(cat, name)
    with open(os.path.join(HERE, path), "wb") as f:
        f.write(data)
    return path


def case(cmd, args, note, predicted, path=None, absent=None):
    e = {"cmd": cmd, "args": args, "path": path or args[-1], "note": note, "predicted": predicted}
    if absent:
        e["absent"] = absent  # the corpus paths the case names that no file carries
    MANIFEST.append(e)


def argv_case(argv, note, predicted, path):
    MANIFEST.append({"argv": argv, "path": path, "note": note, "predicted": predicted})


def fault(token, index=None):
    d = {"ok": False, "token": token}
    if index is not None:
        d["index"] = index
    return d


def manifest_obj(**kw):
    d = {"spec": SPEC, "subjectRef": {"kind": "hash", "sha256": H32()}, "party": addr("party"),
         "files": [{"name": "findings.txt", "sha256": H32(), "bytes": 42, "mediaType": "text/plain"},
                   {"name": "photo-1.jpg", "sha256": H32(), "bytes": 1048576, "mediaType": "image/jpeg"}]}
    d.update(kw)
    return d


def canon(d):
    return cbytes(J(d))


# ==========================================================================
# check
# ==========================================================================

def gen_check():
    def M(slug, b, note, predicted):
        path = write("manifests", slug, b)
        case("check", [path], note, predicted)

    def ok(b):
        return {"manifest": sha(b), "ok": True}

    b = canon(manifest_obj(note_md="勘验附注\n第二行"))
    M("ok_full", b, "a manifest with note_md carrying any scalar value", ok(b))
    b = canon(manifest_obj())
    M("ok_minimal", b, "the four required members", ok(b))
    b = canon(manifest_obj(files=[]))
    M("ok_empty_files", b, "s2.1: the empty array is permitted", ok(b))
    b = canon(manifest_obj(subjectRef={"kind": "evm", "chainId": 1, "contract": addr("venue"), "codeHash": H32(), "id": H32()}))
    M("ok_subject_evm", b, "s1: an evm subject with its four members", ok(b))
    b = canon(manifest_obj(subjectRef={"kind": "evm", "chainId": 1, "contract": addr("venue"), "codeHash": H32(), "id": H32(), "seat": 0}))
    M("ok_subject_evm_extra", b, "galeed/1 s6.3: members beyond the kind's table are permitted", ok(b))
    b = canon(manifest_obj(subjectRef={"kind": "custom", "whatever": [1, 2, {"deep": True}]}))
    M("ok_subject_unknown_kind", b, "galeed/1 s6.3: an unknown kind is tested for object and token alone", ok(b))
    b = canon(manifest_obj(subjectRef={"kind": "hash", "sha256": H32(), "note_md": "任意"}))
    M("ok_subject_hash_note", b, "the hash kind's optional note_md is prose", ok(b))
    b = canon(manifest_obj(note_md=""))
    M("ok_note_empty", b, "note_md may be the empty string", ok(b))
    b = canon(manifest_obj(files=[{"name": "a b.txt", "sha256": H32(), "bytes": 0, "mediaType": "text/plain; charset=utf-8"}]))
    M("ok_name_space_bytes_zero", b, "a skeleton string may carry U+0020; bytes may be 0", ok(b))
    b = canon(manifest_obj(files=[{"name": "~", "sha256": H32(), "bytes": 9007199254740991, "mediaType": "x"}]))
    M("ok_max_bytes_tilde", b, "bytes at 2^53 - 1; U+007E is inside the skeleton charset", ok(b))
    f = {"name": "same", "sha256": H32(), "bytes": 1, "mediaType": "x"}
    b = canon(manifest_obj(files=[f, dict(f)]))
    M("ok_repeated_items", b, "s2.3: two items may repeat name and sha256", ok(b))

    # ---- canonical(b) first
    d = manifest_obj()
    M("canon_pretty", json.dumps(d, indent=1, sort_keys=True).encode("utf-8"), "galeed/1 s3.5 test 6", fault("E_NOT_CANONICAL"))
    M("canon_trailing_newline", canon(d) + b"\n", "galeed/1 s3.5: a trailing newline", fault("E_NOT_CANONICAL"))
    M("canon_unsorted", cbytes(J(d), sort=False), "galeed/1 s3.4 item 5: member order", fault("E_NOT_CANONICAL"))
    M("canon_dup_key", b'{"files":[],"party":"%s","party":"%s","spec":"courtevidence/1","subjectRef":{"kind":"hash","sha256":"%s"}}' % (addr("party").encode(), addr("party").encode(), H32().encode()), "galeed/1 s3.5 test 3", fault("E_DUP_KEY"))
    M("canon_utf8", canon(d).replace(b"findings", b"find\xffngs"), "galeed/1 s3.5 test 1", fault("E_UTF8"))
    M("canon_value_charset_name", canon(manifest_obj(files=[{"name": "café", "sha256": H32(), "bytes": 1, "mediaType": "x"}])), "galeed/1 s3.3: a name is a skeleton string before s2.2 reads it", fault("E_VALUE_CHARSET"))
    M("canon_value_charset_tab", canon(manifest_obj(files=[{"name": "a\tb", "sha256": H32(), "bytes": 1, "mediaType": "x"}])), "galeed/1 s3.3: U+0009 outside a prose subtree", fault("E_VALUE_CHARSET"))
    M("canon_value_charset_del", canon(manifest_obj(files=[{"name": "a\x7fb", "sha256": H32(), "bytes": 1, "mediaType": "x"}])), "galeed/1 s3.3: U+007F outside a prose subtree", fault("E_VALUE_CHARSET"))
    M("canon_key_charset", canon({"spec": SPEC, "subjectRef": {"kind": "hash", "sha256": H32()}, "party": addr("party"), "files": [], "é": 1}), "galeed/1 s3.5 test 4", fault("E_KEY_CHARSET"))
    M("canon_number", canon(d).replace(b'"bytes":42', b'"bytes":42.0'), "galeed/1 s3.5 test 2: E_NUMBER", fault("E_NUMBER"))
    M("canon_negative_bytes", canon(d).replace(b'"bytes":42', b'"bytes":-1'), "galeed/1 s3.2: a sign fails before s2.2 reads bytes", fault("E_NUMBER"))
    M("canon_json", b'{"spec":', "galeed/1 s3.5 test 2: E_JSON", fault("E_JSON"))
    M("canon_empty", b"", "galeed/1 s3.5 test 2 over the empty byte string", fault("E_JSON"))
    M("canon_depth", b'{"files":[],"party":"%s","spec":"courtevidence/1","subjectRef":{"kind":"hash","sha256":"%s"},"x":%s}' % (addr("party").encode(), H32().encode(), ("[" * 128 + "]" * 128).encode()), "galeed/1 s3.5 test 2: E_DEPTH", fault("E_DEPTH"))

    # ---- s2.3 in order
    M("root_array", b"[]", "s2.3: root not an object", fault("E_MANIFEST"))
    M("root_string", b'"courtevidence/1"', "s2.3: root a string", fault("E_MANIFEST"))
    d = manifest_obj(); del d["party"]
    M("missing_party", canon(d), "s2.3: a required member absent", fault("E_MANIFEST_MISSING"))
    d = manifest_obj(); del d["files"]; d["extra"] = 1
    M("missing_and_extra", canon(d), "s2.3 order: missing before closed", fault("E_MANIFEST_MISSING"))
    d = manifest_obj(); d["extra"] = None
    M("closed", canon(d), "s2.3: a member beyond the five", fault("E_MANIFEST_CLOSED"))
    d = manifest_obj(); d["note"] = "not note_md"
    M("closed_note", canon(d), "s2.3: note is not note_md", fault("E_MANIFEST_CLOSED"))
    M("spec_v2", canon(manifest_obj(spec="courtevidence/2")), "s2.3: spec", fault("E_MANIFEST_SPEC"))
    M("spec_int", canon(manifest_obj(spec=1)), "s2.3: spec not a string", fault("E_MANIFEST_SPEC"))
    M("spec_and_party", canon(manifest_obj(spec="x", party="y")), "s2.3 order: spec before party", fault("E_MANIFEST_SPEC"))
    M("subject_evm_missing_id", canon(manifest_obj(subjectRef={"kind": "evm", "chainId": 1, "contract": addr("venue"), "codeHash": H32()})), "s2.3: a known kind forces its table", fault("E_MANIFEST_SUBJECT"))
    EVM = {"kind": "evm", "chainId": 1, "contract": addr("venue"), "codeHash": H32(), "id": H32()}
    for slug, chg in (("chain_string", {"chainId": "1"}), ("contract_hex32", {"contract": H32()}), ("codehash_hex20", {"codeHash": addr("venue")}), ("id_int", {"id": 5})):
        M("subject_evm_" + slug, canon(manifest_obj(subjectRef=dict(EVM, **chg))), "galeed/1 s6.3 via s2.3: an evm member outside its form", fault("E_MANIFEST_SUBJECT"))
    for slug, key_ in (("missing_chain", "chainId"), ("missing_contract", "contract"), ("missing_codehash", "codeHash")):
        d_ = dict(EVM); del d_[key_]
        M("subject_evm_" + slug, canon(manifest_obj(subjectRef=d_)), "galeed/1 s6.3 via s2.3: an evm member absent", fault("E_MANIFEST_SUBJECT"))
    b_evm = canon(manifest_obj(subjectRef={"kind": "EVM"}))
    M("subject_kind_evm_upper", b_evm, "galeed/1 s6.3: kind compares bytewise; EVM is an unknown kind and forces no table", ok(b_evm))
    d_ = manifest_obj(); d_["x_md"] = "ctl \u0001 here"
    M("root_extra_md_control", canon(d_), "s1: an extra _md member at the root opens a prose subtree; canonical passes and the closure refuses, E_MANIFEST_CLOSED and never E_VALUE_CHARSET", fault("E_MANIFEST_CLOSED"))
    f_ = {"name": "a", "sha256": H32(), "bytes": 1, "mediaType": "x", "name_md": "ctl \u0001"}
    M("file_extra_md_control", canon(manifest_obj(files=[f_])), "s1: an extra _md member in a file item opens a prose subtree; E_FILE_CLOSED at index 0", fault("E_FILE_CLOSED", 0))
    z_ = {"root": addr("zik"), "chainId": 1, "tx": H32(), "entry": "0x", "note_md": "ctl \u0001"}
    M("zikaron_extra_md_control", canon(manifest_obj(files=[{"name": "a", "sha256": H32(), "bytes": 1, "mediaType": "x", "zikaron": z_}])), "s1: an extra _md member in a zikaron object opens a prose subtree; E_FILE_ZIKARON at index 0", fault("E_FILE_ZIKARON", 0))

    M("subject_kind_space", canon(manifest_obj(subjectRef={"kind": "has h"})), "s2.3: kind is a token, no U+0020", fault("E_MANIFEST_SUBJECT"))
    M("subject_kind_empty", canon(manifest_obj(subjectRef={"kind": ""})), "s2.3: a token is non-empty", fault("E_MANIFEST_SUBJECT"))
    M("subject_not_object", canon(manifest_obj(subjectRef=H32())), "s2.3: subjectRef not an object", fault("E_MANIFEST_SUBJECT"))
    M("subject_hash_note_int", canon(manifest_obj(subjectRef={"kind": "hash", "sha256": H32(), "note_md": 1})), "s2.3: the hash kind's note_md must be a string", fault("E_MANIFEST_SUBJECT"))
    M("subject_hash_upper", canon(manifest_obj(subjectRef={"kind": "hash", "sha256": H32().upper().replace("0X", "0x")})), "s2.3: sha256 outside hex32", fault("E_MANIFEST_SUBJECT"))
    M("party_upper", canon(manifest_obj(party=addr("party").upper().replace("0X", "0x"))), "s2.3: party outside hex20", fault("E_MANIFEST_PARTY"))
    M("party_short", canon(manifest_obj(party="0x12")), "s2.3: party short", fault("E_MANIFEST_PARTY"))
    M("party_int", canon(manifest_obj(party=0)), "s2.3: party not a string", fault("E_MANIFEST_PARTY"))
    M("files_object", canon(manifest_obj(files={})), "s2.3: files not an array", fault("E_MANIFEST_FILES"))
    M("files_null", canon(manifest_obj(files=None)), "s2.3: files null", fault("E_MANIFEST_FILES"))
    M("file_string", canon(manifest_obj(files=["a"])), "s2.3: an element not an object, index 0", fault("E_FILE", 0))
    good = {"name": "a", "sha256": H32(), "bytes": 1, "mediaType": "x"}
    M("file_missing_bytes_index_1", canon(manifest_obj(files=[good, {"name": "b", "sha256": H32(), "mediaType": "x"}])), "s2.3: the second element lacks bytes", fault("E_FILE_MISSING", 1))
    M("file_closed", canon(manifest_obj(files=[dict(good, extra=True)])), "s2.3: a member beyond the four", fault("E_FILE_CLOSED", 0))
    bad = dict(good); del bad["name"]; bad["extra"] = 1
    M("file_missing_and_closed", canon(manifest_obj(files=[bad])), "s2.3 order: missing before closed within an item", fault("E_FILE_MISSING", 0))
    M("file_name_empty", canon(manifest_obj(files=[dict(good, name="")])), "s2.2: name non-empty", fault("E_FILE_NAME", 0))
    M("file_name_int", canon(manifest_obj(files=[dict(good, name=1)])), "s2.2: name a string", fault("E_FILE_NAME", 0))
    M("file_sha_short", canon(manifest_obj(files=[dict(good, sha256="0x12")])), "s2.2: sha256 hex32", fault("E_FILE_SHA256", 0))
    M("file_sha_upper", canon(manifest_obj(files=[dict(good, sha256=H32().upper().replace("0X", "0x"))])), "s2.2: sha256 lowercase", fault("E_FILE_SHA256", 0))
    M("file_bytes_string", canon(manifest_obj(files=[dict(good, bytes="1")])), "s2.2: bytes an int", fault("E_FILE_BYTES", 0))
    M("file_bytes_bool", canon(manifest_obj(files=[dict(good, bytes=True)])), "s2.2: bytes not a bool", fault("E_FILE_BYTES", 0))
    M("file_mediatype_empty", canon(manifest_obj(files=[dict(good, mediaType="")])), "s2.2: mediaType non-empty", fault("E_FILE_MEDIATYPE", 0))
    M("file_mediatype_null", canon(manifest_obj(files=[dict(good, mediaType=None)])), "s2.2: mediaType a string", fault("E_FILE_MEDIATYPE", 0))
    M("file_order_name_before_sha", canon(manifest_obj(files=[dict(good, name="", sha256="0x1")])), "s2.3 order within an item", fault("E_FILE_NAME", 0))
    M("file_order_first_faulting_element", canon(manifest_obj(files=[good, dict(good, name=""), "x"])), "s2.3: the first faulting element names the fault", fault("E_FILE_NAME", 1))
    M("file_order_index_2", canon(manifest_obj(files=[good, good, dict(good, bytes="x")])), "s2.3: index counted from 0", fault("E_FILE_BYTES", 2))
    M("note_int", canon(manifest_obj(note_md=1)), "s2.3: note_md a string", fault("E_MANIFEST_NOTE"))
    M("note_null", canon(manifest_obj(note_md=None)), "s2.3: note_md written null is present", fault("E_MANIFEST_NOTE"))
    M("note_after_files", canon(manifest_obj(files=[dict(good, name="")], note_md=1)), "s2.3 order: files before note_md", fault("E_FILE_NAME", 0))
    d = manifest_obj(spec="x"); d["extra"] = 1
    M("order_closed_before_spec", canon(d), "s2.3 order: closed before spec", fault("E_MANIFEST_CLOSED"))
    M("order_subject_before_party", canon(manifest_obj(subjectRef=1, party="y")), "s2.3 order: subjectRef before party", fault("E_MANIFEST_SUBJECT"))
    M("order_party_before_files", canon(manifest_obj(party="y", files={})), "s2.3 order: party before files", fault("E_MANIFEST_PARTY"))
    M("file_order_sha_before_bytes", canon(manifest_obj(files=[{"bytes": "x", "mediaType": "x", "name": "a", "sha256": "0x1"}])), "s2.3 order within an item: sha256 before bytes", fault("E_FILE_SHA256", 0))
    M("file_order_bytes_before_mediatype", canon(manifest_obj(files=[{"bytes": "x", "mediaType": 1, "name": "a", "sha256": H32()}])), "s2.3 order within an item: bytes before mediaType", fault("E_FILE_BYTES", 0))
    M("file_order_closed_before_name", canon(manifest_obj(files=[dict(good, name="", extra=1)])), "s2.3 order within an item: closed before name", fault("E_FILE_CLOSED", 0))
    M("subject_hash_note_null", canon(manifest_obj(subjectRef={"kind": "hash", "sha256": H32(), "note_md": None})), "galeed/1 s6: note_md written null inside a hash subject is present with a refused form", fault("E_MANIFEST_SUBJECT"))
    b = canon(manifest_obj(files=[{"name": "a", "sha256": H32(), "bytes": 0, "mediaType": "x"}]))
    M("ok_bytes_zero_any_digest", b, "s2.2: no predicate relates bytes to sha256", ok(b))
    b = canon(manifest_obj(subjectRef={"kind": "custom", "x_md": "control \u0001 and tab\t", "deep": {"y_md": ["\u0000"]}}))
    M("ok_subject_prose_subtree", b, "s1: an extra member of a subjectRef ending in _md opens a prose subtree", ok(b))
    M("subject_extra_non_prose_control", canon(manifest_obj(subjectRef={"kind": "custom", "x": "control \u0001"})), "galeed/1 s3.3: a skeleton value with a control scalar fails at canonical(b)", fault("E_VALUE_CHARSET"))
    ZK = {"root": addr("author"), "chainId": 1, "tx": H32(), "entry": "0x" + "7b22737065632231" }
    good_z = dict(good, zikaron=ZK)
    b = canon(manifest_obj(files=[good_z]))
    M("ok_zikaron_full", b, "s2.2: the optional fifth member with its four members in form", ok(b))
    b = canon(manifest_obj(files=[dict(good, zikaron=dict(ZK, entry="0x"))]))
    M("ok_zikaron_entry_empty", b, "s2.2: 0x alone spells the empty byte string", ok(b))
    b = canon(manifest_obj(files=[good, good_z, good]))
    M("ok_zikaron_some_items", b, "s2.2: the member is per item; items with and without it list side by side", ok(b))
    b = canon(manifest_obj(files=[dict(good, zikaron=dict(ZK, chainId=0))]))
    M("ok_zikaron_chain_zero", b, "s2.2: chainId is an int; zero is one", ok(b))
    M("zikaron_null", canon(manifest_obj(files=[dict(good, zikaron=None)])), "s2.3: zikaron written null is present with a refused form", fault("E_FILE_ZIKARON", 0))
    M("zikaron_array", canon(manifest_obj(files=[dict(good, zikaron=[ZK["root"]])])), "s2.3: zikaron not an object", fault("E_FILE_ZIKARON", 0))
    M("zikaron_missing_tx", canon(manifest_obj(files=[dict(good, zikaron={k: v for k, v in ZK.items() if k != "tx"})])), "s2.2: a member of the four absent", fault("E_FILE_ZIKARON", 0))
    M("zikaron_extra_member", canon(manifest_obj(files=[dict(good, zikaron=dict(ZK, note="x"))])), "s2.2: a member beyond the four", fault("E_FILE_ZIKARON", 0))
    M("zikaron_root_short", canon(manifest_obj(files=[dict(good, zikaron=dict(ZK, root="0x12"))])), "s2.2: root hex20", fault("E_FILE_ZIKARON", 0))
    M("zikaron_root_upper", canon(manifest_obj(files=[dict(good, zikaron=dict(ZK, root="0x" + ZK["root"][2:].upper()))])), "s2.2: root lowercase", fault("E_FILE_ZIKARON", 0))
    M("zikaron_chain_string", canon(manifest_obj(files=[dict(good, zikaron=dict(ZK, chainId="1"))])), "s2.2: chainId an int", fault("E_FILE_ZIKARON", 0))
    M("zikaron_chain_bool", canon(manifest_obj(files=[dict(good, zikaron=dict(ZK, chainId=True))])), "s2.2: chainId not a bool", fault("E_FILE_ZIKARON", 0))
    M("zikaron_tx_hex20", canon(manifest_obj(files=[dict(good, zikaron=dict(ZK, tx=addr("author")))])), "s2.2: tx hex32", fault("E_FILE_ZIKARON", 0))
    M("zikaron_entry_odd", canon(manifest_obj(files=[dict(good, zikaron=dict(ZK, entry="0x7b2"))])), "s2.2: an even number of digits", fault("E_FILE_ZIKARON", 0))
    M("zikaron_entry_upper", canon(manifest_obj(files=[dict(good, zikaron=dict(ZK, entry="0x7B22"))])), "s2.2: lowercase digits", fault("E_FILE_ZIKARON", 0))
    M("zikaron_entry_no_prefix", canon(manifest_obj(files=[dict(good, zikaron=dict(ZK, entry="7b22"))])), "s2.2: the 0x prefix", fault("E_FILE_ZIKARON", 0))
    M("zikaron_entry_int", canon(manifest_obj(files=[dict(good, zikaron=dict(ZK, entry=1))])), "s2.2: entry a string", fault("E_FILE_ZIKARON", 0))
    M("zikaron_index_1", canon(manifest_obj(files=[good, dict(good, zikaron=[])])), "s2.3: the index names the faulting item", fault("E_FILE_ZIKARON", 1))
    M("file_closed_sixth_member", canon(manifest_obj(files=[dict(good_z, extra=1)])), "s2.3: a member beyond the five", fault("E_FILE_CLOSED", 0))
    M("file_order_mediatype_before_zikaron", canon(manifest_obj(files=[dict(good, mediaType="", zikaron=[])])), "s2.3 order within an item: mediaType before zikaron", fault("E_FILE_MEDIATYPE", 0))
    M("file_order_zikaron_before_note", canon(manifest_obj(files=[dict(good, zikaron=[])], note_md=1)), "s2.3 order: a file item's zikaron before note_md", fault("E_FILE_ZIKARON", 0))
    M("file_order_closed_before_zikaron", canon(manifest_obj(files=[dict(good, zikaron=[], extra=1)])), "s2.3 order within an item: closed before zikaron", fault("E_FILE_CLOSED", 0))
    case("check", ["manifests/absent.json"], "HARNESS: a path the command cannot read", None, absent=["manifests/absent.json"])
    case("check", ["manifests/m001_ok_full.json", "x"], "HARNESS: an argument beyond the list", None)
    case("check", [], "HARNESS: no path", None, path="(none)")


# ==========================================================================
# digest
# ==========================================================================

def gen_digest():
    for slug, data in (("text", "勘验所见:三箱货物,封条完整。\n".encode("utf-8")), ("empty", b""), ("binary", bytes(range(256)) * 3)):
        path = write("files", slug, data, ext=".bin")
        case("digest", [path], "s2.2: the file-item view of a byte string", {"bytes": len(data), "sha256": sha(data)})
    case("digest", ["files/absent.bin"], "HARNESS: a path the command cannot read", None, absent=["files/absent.bin"])
    case("digest", ["files/f001_text.bin", "x"], "HARNESS: an argument beyond the list", None)
    case("digest", [], "HARNESS: a missing argument", None, path="(none)")
    argv_case([], "HARNESS: no command", None, "(none)")
    argv_case(["frobnicate", "files/f001_text.bin"], "HARNESS: an unknown command", None, "(none)")


# ==========================================================================
# recipient-check
# ==========================================================================

def gen_recipient():
    def R(r, note, okv):
        case("recipient-check", [r], note, {"ok": True} if okv else fault("E_RECIPIENT"), path="(argument) " + r)

    good = recipient("r1")
    R(good, "s3.2: a well formed recipient", True)
    R(good[:-1] + ("q" if good[-1] != "q" else "p"), "s3.2: the checksum does not verify", False)
    R(good.upper(), "s3.2: uppercase", False)
    R(good[:4] + good[5:], "s3.2: sixty-one characters", False)
    R(good + "q", "s3.2: sixty-three characters", False)
    R("aeg1" + good[4:], "s3.2: the human-readable part is age", False)
    R("age" + good[4:] + "q", "s3.2: no separator at position 4", False)
    R(good[:10] + "b" + good[11:], "s3.2: a character outside the Bech32 alphabet", False)
    raw = bytes(RNG.randrange(256) for _ in range(32))
    d5 = to5(raw)
    d5[-1] |= 1
    R(bech32_encode("age", d5), "s3.2: the four padding bits are not zero, though the checksum verifies", False)
    R("", "s3.2: the empty string", False)
    R("age1", "s3.2: the prefix alone", False)
    R("age1" + "q" * 58, "s3.2: a data part whose checksum fails", False)
    R("age1" + good[4:52] + "Q" + good[53:], "s3.2: mixed case", False)
    R(bech32_encode("age", to5(bytes(32))), "s3.2: the all-zero key spells a recipient", True)
    R(bech32_encode("age", to5(bytes([255] * 32))), "s3.2: the all-ones key spells a recipient", True)
    raw = bytes(RNG.randrange(256) for _ in range(32))
    d5 = to5(raw)
    poly = _polymod(_hrp_expand("age") + d5 + [0] * 6) ^ 0x2bc830a3
    chk = [(poly >> 5 * (5 - i)) & 31 for i in range(6)]
    R("age1" + "".join(BECH32[x] for x in d5 + chk), "s3.2: the Bech32m spelling of a key (BIP-350's constant) is refused", False)
    R(good[:30] + "\u00e9" + good[31:], "s3.2: a scalar value above U+007E fails for that reason alone", False)
    case("recipient-check", [good, "x"], "HARNESS: an argument beyond the list", None, path="(argument)")
    case("recipient-check", [], "HARNESS: no argument", None, path="(argument)")


# ==========================================================================
# pick-recipient
# ==========================================================================

def gen_pick():
    root = addr("judge")
    k = key("judge")

    def ent(etype, seq, prev, body, kk=None, judge=None):
        return G.make_entry(kk or k, etype, seq, prev, body, judge=judge)

    def kb(seq, prev, rc, eff=1700000000, extra=None, kk=None):
        body = {"recipient": rc, "effective": eff}
        if extra:
            body.update(extra)
        return ent("keybinding", seq, prev, body, kk=kk)

    def rec(b, sender, bn, verdict="counted", ts=None):
        return {"chainId": 1, "blockNumber": bn, "blockTimestamp": 1700000000 + bn if ts is None else ts,
                "tx": "0x%064x" % (0xA0000 + bn), "sender": sender, "hash": eid(b),
                "verdict": verdict, "entry": "0x" + b.hex()}

    def basis(senders):
        return {"chains": [{"chainId": 1, "fromBlock": 0, "toBlock": 1000, "registries": [],
                            "senders": sorted(set(senders), key=lambda a: a.encode())}]}

    def audit(entries, root_arg=None, carrier=None, extra=(), senders=(), flat=False):
        """`flat`: every entry's record in one second, so that no key has
        retired at any of them (galeed/1 s7.4)."""
        return {"root": root_arg or root,
                "anchors": [rec(b, carrier or root, 10 + i, ts=1700000000 if flat else None) for i, b in enumerate(entries)] + list(extra),
                "basis": basis([root, addr("succ"), addr("other")] + list(senders))}

    def audit_case(slug, entries, note, predicted, root_arg=None, carrier=None, extra=(), raw=None, flat=False):
        inp = raw if raw is not None else json.dumps(audit(entries, root_arg, carrier, extra, flat=flat)).encode("utf-8")
        path = write("audits", slug, inp)
        case("pick-recipient", [path], note, predicted)

    set_case = lambda slug, entries, note, predicted, root_arg=None: audit_case(slug, entries, note, predicted, root_arg)

    def B(b, rc, eff, sq):
        return {"effective": eff, "entry_id": eid(b), "recipient": rc, "recipientForm": is_recipient(rc), "seq": sq}

    def is_recipient(rc):
        # the generator's own reading of s3.2
        if len(rc) != 62 or rc[:4] != "age1" or any(c not in BECH32 for c in rc[4:]):
            return False
        vals = _hrp_expand("age") + [BECH32.index(c) for c in rc[4:]]
        if _polymod(vals) != 1:
            return False
        acc = 0
        for c in rc[4:56]:
            acc = (acc << 5) | BECH32.index(c)
        return (acc & 0xF) == 0

    r1, r2, r3 = recipient("a"), recipient("b"), recipient("c")
    g0 = ent("genesis", 0, None, {"statement_md": "a judge's ledger"})
    k1 = kb(1, eid(g0), r1, 1700000000)
    k2 = kb(2, eid(k1), r2, 1700000100)
    set_case("one", [g0, k1, k2], "s4.2 item 5: the single binding at the greatest seq", {"binding": B(k2, r2, 1700000100, 2), "pick": "one"})
    set_case("one_unordered_records", [k2, g0, k1], "the anchor set is a set; the order of the records carries nothing", {"binding": B(k2, r2, 1700000100, 2), "pick": "one"})
    a1 = ent("annotation", 1, eid(g0), {"note_md": "no binding here"})
    set_case("none", [g0, a1], "s4.2 item 1: no keybinding", {"pick": "none"})
    set_case("none_empty", [], "s4.2 item 1: an empty anchor set", {"pick": "none"})
    k2g = kb(2, H32(), r2, 1700000100)
    set_case("gap", [g0, k2g], "s4.2 item 3: position 1 is missing", {"list": [B(k2g, r2, 1700000100, 2)], "pick": "candidates", "reason": "gap"})
    k1b = kb(1, eid(g0), r2, 1700000001)
    rows = sorted([B(k1, r1, 1700000000, 1), B(k1b, r2, 1700000001, 1)], key=lambda r: r["entry_id"].encode())
    set_case("fork", [g0, k1, k1b], "s4.2 item 2: two bindings at the top position; the list is ordered by entry_id at equal seq", {"list": rows, "pick": "candidates", "reason": "fork"})
    set_case("fork_with_annotation", [g0, k1, a1], "s4.2 item 2: an equivocation at the top position with a non-binding", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "fork"})
    kbad = kb(1, H32(), r1)
    set_case("broken_prev", [g0, kbad], "s4.2 item 4: PREV_MISMATCH is hard and no gap or fork is at play", {"list": [B(kbad, r1, 1700000000, 1)], "pick": "candidates", "reason": "broken"})
    kx = kb(1, eid(g0), r1, kk=key("other"))
    audit_case("excluded_other_judge", [g0], "s4.1: an entry outside the lineage, carried by its own signer, is no binding", {"pick": "none"}, extra=[rec(kx, addr("other"), 50)])
    audit_case("excluded_other_judge_carried_by_root", [g0, kx], "s4.1: the root's counted record carries a stranger's binding: CARRIED_MALFORMED is hard, and the stranger's entry is no binding, so none", {"pick": "none"})
    audit_case("malformed_ignored", [g0, k1], "s4.1: a malformed byte string carried by a stranger is no binding and grounds no finding", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"}, extra=[rec(b"not an entry", addr("other"), 50)])
    audit_case("malformed_carried_by_root_broken", [g0, k1], "s4.2 item 4: malformed bytes carried by the root's own counted record are CARRIED_MALFORMED, hard: broken", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "broken"}, extra=[rec(b"not an entry", root, 50)])
    set_case("duplicates_collapse", [g0, k1, k1, k1], "s4.1: byte-identical copies carried by three records collapse, so no fork", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"})
    audit_case("binding_carried_by_void_record", [g0, k1], "s4.1 with galeed/1 s8.6: a later binding carried only by a VOID record is no ledger entry; the pick is relative to the scan and names the earlier binding", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"}, extra=[rec(k2, root, 50, verdict="VOID")])
    audit_case("binding_carried_by_unproven_record", [g0, k1], "s4.2 rule 3: a binding carried only by an UNPROVEN record whose hash no counted record has is a hole (the label is UNPROVEN); the pick is withheld", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "gap"}, extra=[rec(k2, root, 50, verdict="UNPROVEN")])
    audit_case("binding_hash_only_gap", [g0, k1], "s4.2 rule 3: a counted record of the next binding carries nothing; the hash is MISSING, a hole, and the pick is withheld", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "gap"}, extra=[{"chainId": 1, "blockNumber": 50, "blockTimestamp": 1700000050, "tx": "0x%064x" % 0xA0050, "sender": root, "hash": eid(k2), "verdict": "counted"}])
    audit_case("unproven_record_hash_also_counted_no_hole", [g0, k1, k2], "s4.2 rule 3: an UNPROVEN record whose hash a counted record carries as a ledger entry is no hole; one", {"binding": B(k2, r2, 1700000100, 2), "pick": "one"}, extra=[rec(k2, root, 50, verdict="UNPROVEN")])
    audit_case("unproven_record_of_stranger_discarded_no_hole", [g0, k1], "s4.2 rule 3: an UNPROVEN record from a stranger is discarded and stands in no unproven row; no hole, one", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"}, extra=[rec(k2, addr("other"), 50, verdict="UNPROVEN")])
    audit_case("missing_hash_above_top_is_hole", [g0, k1], "s4.2 rule 3: a counted anchor of nothing at any position is a hole; the pick is withheld whatever the hash was", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "gap"}, extra=[{"chainId": 1, "blockNumber": 50, "blockTimestamp": 1700000050, "tx": "0x%064x" % 0xA0050, "sender": root, "hash": H32(), "verdict": "counted"}])
    k4 = kb(4, H32(), r3, 1700000400)
    set_case("candidates_order", [g0, k1, k2, k4], "s4.1: seq descending; a gap at 3", {"list": [B(k4, r3, 1700000400, 4), B(k2, r2, 1700000100, 2), B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "gap"})
    kold = kb(1, eid(g0), "age1old", 5)
    set_case("recipient_form_false", [g0, kold], "s4.1: a recipient outside s3.2 is carried with recipientForm false; the pick is not withheld", {"binding": B(kold, "age1old", 5, 1), "pick": "one"})
    s1 = ent("succession", 1, eid(g0), {"to": addr("succ"), "kind": "keyrotation", "effective": 1700000000, "statement_md": "rotated"})
    k2s = kb(2, eid(s1), r2, 1700000100, kk=key("succ"))
    audit_case("lineage_successor", [g0, s1, k2s], "s4.1: a binding signed by the successor key is in the lineage, carried in the second the root handed off", {"binding": B(k2s, r2, 1700000100, 2), "pick": "one"}, flat=True)
    set_case("successor_binding_carried_by_retired_root", [g0, s1, k2s], "s4.1 with galeed/1 s7.4: the successor's binding carried by the root's record after the root retired is DISCARDED and no ledger entry; no binding", {"pick": "none"})
    audit_case("lineage_successor_own_anchor", [g0, s1], "s4.1: the successor's binding carried by the successor's own record", {"binding": B(k2s, r2, 1700000100, 2), "pick": "one"}, extra=[rec(k2s, addr("succ"), 50)])
    audit_case("root_is_current_key", [g0, s1], "s4.1: the same ledger read under the successor's key as root; the root's records are discarded there, the genesis and the succession fall outside that lineage, position 2 is the first held, and the pick is withheld for the gap", {"list": [B(k2s, r2, 1700000100, 2)], "pick": "candidates", "reason": "gap"}, root_arg=addr("succ"), extra=[rec(k2s, addr("succ"), 50)])
    k2a = kb(2, H32(), r1, 1)
    k2b = kb(2, H32(), r2, 2)
    rows = sorted([B(k2a, r1, 1, 2), B(k2b, r2, 2, 2)], key=lambda r: r["entry_id"].encode())
    set_case("fork_over_gap", [g0, k2a, k2b], "s4.2 order: fork at the top is decided before a gap", {"list": rows, "pick": "candidates", "reason": "fork"})
    set_case("equivocation_below_top", [g0, k1, a1, k2], "s4.2: an equivocation below the top is a hard finding, not a fork; no gap: broken", {"list": [B(k2, r2, 1700000100, 2), B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "broken"})
    k3g = kb(3, H32(), r3, 1700000300)
    set_case("equivocation_below_top_with_gap", [g0, k1, a1, k3g], "s4.2 order: gap before broken", {"list": [B(k3g, r3, 1700000300, 3), B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "gap"})
    ke = kb(1, eid(g0), r1, 7, extra={"note_md": "extra members are data"})
    set_case("extra_body_members", [g0, ke], "galeed/1 s6.9: extra body members change nothing; effective is carried", {"binding": B(ke, r1, 7, 1), "pick": "one"})
    set_case("genesis_not_carried", [k1], "the genesis is carried by no record: a gap at 0", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "gap"})
    UNREAD = {"pick": "unread"}
    audit_case("unread_root_upper", [g0, k1], "s4.1: a root outside hex20 is no audit input; unread", UNREAD, root_arg=root.upper().replace("0X", "0x"))
    audit_case("unread_root_array", [], "s4.1: a file whose root value is not an object is no audit input; unread", UNREAD, raw=b'[]')
    audit_case("unread_unreadable", [], "s4.1: a file the reader refuses; unread, never misuse", UNREAD, raw=b"[")
    bad = audit([g0, k1]); bad["anchors"][0]["entry"] = "0X" + bad["anchors"][0]["entry"][2:]
    audit_case("unread_entry_0X", [], "s4.1 with galeed/1 s9.2: an entry outside the transport spelling; unread", UNREAD, raw=json.dumps(bad).encode())
    bad = audit([g0, k1]); bad["basis"] = {"chains": [], "transactions": []}
    audit_case("unread_basis_member", [], "s4.1 with galeed/1 s9.4: a basis with a member beyond its list; unread", UNREAD, raw=json.dumps(bad).encode())
    bad = audit([g0, k1]); del bad["basis"]
    audit_case("unread_basis_absent", [], "s4.1: a required member absent; unread", UNREAD, raw=json.dumps(bad).encode())
    bad = audit([g0, k1]); bad["anchors"][1]["blockNumber"] = 5000
    audit_case("unread_record_unreached", [], "s4.1 with galeed/1 s9.4: a record no chains object reaches; unread", UNREAD, raw=json.dumps(bad).encode())
    up = audit([g0, k1]); up["anchors"][0]["entry"] = "0x" + up["anchors"][0]["entry"][2:].upper()
    audit_case("upper_digits_ok", [], "HARNESS: transport digits of either case", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"}, raw=json.dumps(up).encode())
    esc = lambda h: "".join("\\u%04x" % ord(c) if i % 3 == 0 else c for i, c in enumerate(h))
    es = json.dumps(audit([g0, k1])).replace('"0x' + g0.hex() + '"', '"' + esc("0x" + g0.hex()) + '"')
    audit_case("escaped_entry", [], "HARNESS: the transport spelling is decided on the decoded string; escapes in an entry", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"}, raw=es.encode("ascii"))
    audit_case("empty_anchor_set_none", [], "s4.2 item 1: a valid audit input with no record holds no binding", {"pick": "none"})
    audit_case("basis_covers_no_chain_none", [], "s4.2 item 1: a basis that covers no chain and no record", {"pick": "none"}, raw=json.dumps({"root": root, "anchors": [], "basis": {"chains": []}}).encode())
    a2a = ent("annotation", 2, eid(k1), {"note_md": "two"})
    a2b = ent("annotation", 2, eid(k1), {"note_md": "two again"})
    set_case("equivocation_above_top", [g0, k1, a2a, a2b], "s4.2: top is the greatest seq among the bindings; an equivocation above it is a hard finding, not a fork", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "broken"})
    # a shared prev between seq 2 and seq 3 carries position 2 (galeed/1 s8.7 item 3)
    j2 = ent("judgment", 2, eid(k1), {"subjectRef": {"kind": "hash", "sha256": H32()}, "amanah": H32(), "verdict_md": "v"})
    k3a = kb(3, eid(j2), r3, 1700000300)
    a3 = ent("annotation", 3, eid(j2), {"note_md": "three"})
    a3b = ent("annotation", 2, eid(k1), {"note_md": "shares k1 as prev with j2"})
    set_case("shared_prev_same_seq_at_top", [g0, k1, j2, k3a, a3], "s4.2: k3 at seq 3 and a3 at seq 3 share a seq: position 3 equals top, fork", {"list": [B(k3a, r3, 1700000300, 3), B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "fork"})
    k2p = kb(2, eid(k1), r2, 1700000100)
    j3 = ent("judgment", 3, eid(k1), {"subjectRef": {"kind": "hash", "sha256": H32()}, "amanah": H32(), "verdict_md": "prev is k1, not k2"})
    set_case("shared_prev_position_is_smaller_seq", [g0, k1, k2p, j3], "s4.2: k2 at seq 2 and j3 at seq 3 share prev k1; the equivocation's position is 2, which is top: fork", {"list": [B(k2p, r2, 1700000100, 2), B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "fork"})
    k3q = kb(3, eid(k2p), r3, 1700000300)
    j4 = ent("judgment", 4, eid(k2p), {"subjectRef": {"kind": "hash", "sha256": H32()}, "amanah": H32(), "verdict_md": "prev is k2, as is k3's"})
    set_case("shared_prev_below_top_after_bindings", [g0, k1, k2p, k3q, j4], "s4.2: k3 at 3 and j4 at 4 share prev k2; position 3 equals top 3: fork", {"list": [B(k3q, r3, 1700000300, 3), B(k2p, r2, 1700000100, 2), B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "fork"})
    k4r = kb(4, eid(k3q), r1, 1700000400)
    j5 = ent("judgment", 5, eid(k2p), {"subjectRef": {"kind": "hash", "sha256": H32()}, "amanah": H32(), "verdict_md": "prev is k2, as is k3's"})
    set_case("shared_prev_pair_straddling_top", [g0, k1, k2p, k3q, k4r, j5], "s4.2: k3 (seq 3) and j5 (seq 5) share prev k2, position 3, while top is 4: not a fork; hard, so broken", {"list": [B(k4r, r1, 1700000400, 4), B(k3q, r3, 1700000300, 3), B(k2p, r2, 1700000100, 2), B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "broken"})
    ja1 = ent("judgment", 1, eid(g0), {"subjectRef": {"kind": "hash", "sha256": H32()}, "amanah": H32(), "verdict_md": "prev is genesis"})
    kg2 = kb(2, eid(g0), r2, 1700000100)
    set_case("shared_prev_upper_entry_is_top_binding", [g0, ja1, kg2], "s4.2: judgment at 1 and keybinding at 2 share prev genesis; position 1, top 2: not a fork; PREV_MISMATCH at 2 is hard, so broken", {"list": [B(kg2, r2, 1700000100, 2)], "pick": "candidates", "reason": "broken"})
    # both orders of an equal-seq fork, searched over effective so that each order appears under every seed
    def fork_pair(want_first_effective_small):
        eff = 1
        while True:
            ka = kb(1, eid(g0), r1, eff)
            kb_ = kb(1, eid(g0), r2, eff + 1)
            if (eid(ka).encode() < eid(kb_).encode()) == want_first_effective_small:
                return ka, kb_, eff
            eff += 2
    ka, kb1, eff = fork_pair(True)
    set_case("fork_order_small_effective_first", [g0, ka, kb1], "s4.1: at equal seq the list is by entry_id bytewise; here the binding with the smaller effective sorts first", {"list": [B(ka, r1, eff, 1), B(kb1, r2, eff + 1, 1)], "pick": "candidates", "reason": "fork"})
    ka, kb1, eff = fork_pair(False)
    set_case("fork_order_large_effective_first", [g0, ka, kb1], "s4.1: the same fork with the other entry_id order; effective decides nothing", {"list": [B(kb1, r2, eff + 1, 1), B(ka, r1, eff, 1)], "pick": "candidates", "reason": "fork"})
    # ---- round 1 additions (2026-10-16 revision)
    k2ret = kb(2, eid(s1), r2, 1700000100)
    audit_case("retired_key_binding_certain_broken", [g0, s1, k2ret], "s4.1 with galeed/1 s7.4: the root key binds again at the position after it handed the seat on, carried in that same second: the record is DISCARDED and no binding stands; none", {"pick": "none"}, flat=True)
    audit_case("root_binding_carried_by_successor_same_second", [g0, s1], "s4.2 rule 4 with galeed/1 s7.4, s8.1: the root key binds again at the position after it handed the seat on, and the successor's record carries that binding in the same second: the successor has not retired at its record, so the binding is a ledger entry signed out of office, a certain AUTHORITY_MISMATCH, hard; broken", {"list": [B(k2ret, r2, 1700000100, 2)], "pick": "candidates", "reason": "broken"}, flat=True, extra=[rec(k2ret, addr("succ"), 12, ts=1700000000)])
    set_case("retired_key_binding_after_retirement", [g0, s1, k2ret], "s4.1 with galeed/1 s7.4: the root key's binding carried after its retirement is DISCARDED; no binding", {"pick": "none"})
    k3ret = kb(3, H32(), r3, 1700000300)
    audit_case("retired_key_binding_after_hole_gap", [g0, s1, k3ret], "s4.1 with galeed/1 s7.4: the handing key's binding after a hole, at a position after its handing off and carried in that second, is DISCARDED; no binding stands; none", {"pick": "none"}, flat=True)
    gX = ent("genesis", 0, None, {"statement_md": "opened by the successor"}, kk=key("succ"))
    sR = ent("succession", 1, eid(gX), {"to": addr("succ"), "kind": "keyrotation", "effective": 1700000000, "statement_md": "the root hands on"})
    kR = kb(2, eid(sR), r2, 1700000100, kk=key("succ"))
    audit_case("root_mismatch_alone_broken", [gX, sR, kR], "s4.2 rule 4: a genesis signed by the successor under the root's audit is ROOT_MISMATCH, hard and alone; broken", {"list": [B(kR, r2, 1700000100, 2)], "pick": "candidates", "reason": "broken"}, flat=True)
    a3h = ent("annotation", 3, H32(), {"note_md": "above the top binding, after a hole"})
    set_case("hole_above_top", [g0, k1, a3h], "s4.2 rule 3: a hole above the top binding withholds the pick like any other", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "gap"})
    kbadtop = kb(2, eid(k1), "age1old", 6)
    set_case("recipient_form_false_at_top_above_valid", [g0, k1, kbadtop], "s4.2: a top binding whose recipient fails s3.2 is the pick all the same; no fall-through to the earlier valid one", {"binding": B(kbadtop, "age1old", 6, 2), "pick": "one"})
    set_case("ledger_binding_beside_root_carried_stranger_entry", [g0, k1, kx], "s4.2 rule 4: the root carries a stranger's binding beside its own: CARRIED_MALFORMED FOREIGN is hard; broken with the ledger's one binding", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "broken"})
    kq = kb(1, eid(g0), 'age1"q\\x', 8)
    set_case("recipient_with_quote_and_backslash", [g0, kq], "s3.2/s4.1: a recipient token carrying a quotation mark and a backslash; the answer escapes them as galeed/1 s3.4 spells a string", {"binding": B(kq, 'age1"q\\x', 8, 1), "pick": "one"})
    kmax = kb(9007199254740991, H32(), r1, 9)
    set_case("binding_at_max_seq", [g0, kmax], "a binding at seq 2^53 - 1 after a hole; gap", {"list": [B(kmax, r1, 9, 9007199254740991)], "pick": "candidates", "reason": "gap"})
    k5 = kb(5, H32(), r3, 1700000500)
    set_case("shared_prev_pair_then_gap", [g0, k1, k2p, j3, k5], "s4.2: k2 (seq 2) and j3 (seq 3) share prev k1 at position 2 while top is 5: no fork; the hole at 4 answers gap before the hard finding", {"list": [B(k5, r3, 1700000500, 5), B(k2p, r2, 1700000100, 2), B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "gap"})
    # s4.2 rule 3: the sides of the hole rule
    def hash_only(h, bn, verdict="counted"):
        return {"chainId": 1, "blockNumber": bn, "blockTimestamp": 1700000000 + bn, "tx": "0x%064x" % (0xA0000 + bn), "sender": root, "hash": h, "verdict": verdict}
    junk = b"not an entry"
    audit_case("unproven_junk_no_hole", [g0, k1], "s4.2 rule 3: the root's UNPROVEN record carries junk; its hash is a MALFORMED row, bytes the scan read, so no hole; one", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"}, extra=[rec(junk, root, 50, verdict="UNPROVEN")])
    audit_case("unproven_stranger_entry_no_hole", [g0, k1], "s4.2 rule 3: the root's UNPROVEN record carries a stranger's binding; its hash is an EXCLUDED row, bytes the scan read, so no hole; one", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"}, extra=[rec(kx, root, 50, verdict="UNPROVEN")])
    audit_case("missing_bytes_only_in_void_carrier_hole", [g0, k1], "s4.2 rule 3: the counted record of the next binding carries nothing and its bytes ride only in a VOID record: read only in a record that did not count, a hole; gap", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "gap"}, extra=[hash_only(eid(k2), 50), rec(k2, root, 51, verdict="VOID")])
    audit_case("missing_bytes_only_in_discarded_carrier_hole", [g0, k1], "s4.2 rule 3: the counted record carries nothing and the bytes ride only in a stranger's discarded record: a hole; gap", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "gap"}, extra=[hash_only(eid(k2), 50), rec(k2, addr("other"), 51)])
    audit_case("missing_malformed_bytes_in_discarded_carrier_no_hole", [g0, k1], "s4.2 rule 3: the root's counted record carries nothing under a hash whose bytes a stranger's discarded record carries and which are no entry: MISSING and MALFORMED, GAPS, yet no hole and no hard finding; one", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"}, extra=[hash_only(eid(junk), 50), rec(junk, addr("other"), 51)])
    audit_case("missing_malformed_bytes_in_void_carrier_no_hole", [g0, k1], "s4.2 rule 3: the same with the root's VOID record as the carrier of the junk: MISSING and MALFORMED, no hole; one", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"}, extra=[hash_only(eid(junk), 50), rec(junk, root, 51, verdict="VOID")])
    audit_case("missing_foreign_entry_in_discarded_carrier_no_hole", [g0, k1], "s4.2 rule 3: the counted record carries nothing under a stranger's binding that only the stranger's discarded record carries: MISSING and EXCLUDED, no hole; one", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"}, extra=[hash_only(eid(kx), 50), rec(kx, addr("other"), 51)])
    audit_case("top_binding_relayed_by_stranger_only", [g0, k1], "s4.1 with galeed/1 s8.1: the judge's later binding carried only by a stranger's counted record, which the trim of galeed/1 s8.1 discards: no ledger entry, no commitment of a lineage key, no hole; COMPLETE and one naming the earlier binding", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"}, extra=[rec(k2, addr("other"), 50)])
    audit_case("void_record_carrying_nothing_no_hole", [g0, k1], "s4.2 rule 3: a VOID record of the root carrying nothing under the later binding's hash is no commitment the report lists as missing or unproven: no hole; one", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"}, extra=[hash_only(eid(k2), 50, verdict="VOID")])
    audit_case("successor_root_binding_relayed_by_root_only", [g0, s1, k2s], "s4.1: under the successor as root, the successor's binding was sent only from the root's key, so every record is discarded: an empty ledger, COMPLETE; none", {"pick": "none"}, root_arg=addr("succ"))
    audit_case("successor_root_signed_nothing", [g0, s1], "s4.1: the ledger read under the successor as root, the root's records discarded and the successor having signed nothing: an empty ledger, COMPLETE; none", {"pick": "none"}, root_arg=addr("succ"))
    a2s = ent("annotation", 2, eid(s1), {"note_md": "the successor annotates"}, kk=key("succ"))
    audit_case("successor_root_signed_only_annotation", [g0, s1], "s4.1: under the successor as root the successor signed an annotation and no binding: positions 0 and 1 fall outside the lineage, GAPS, and rule 1 answers none before rule 3", {"pick": "none"}, root_arg=addr("succ"), extra=[rec(a2s, addr("succ"), 50)])
    audit_case("void_bytes_beside_unproven_nobytes_gap", [g0, k1], "s4.2 rule 3: the later binding's bytes ride only in a VOID record while an UNPROVEN record of the root commits to its hash carrying nothing: the unproven row's hash is no ledger entry, a hole; gap", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "gap"}, extra=[rec(k2, root, 50, verdict="VOID"), hash_only(eid(k2), 51, verdict="UNPROVEN")])
    audit_case("middle_binding_void_only_gap", [g0], "s4.2 rule 3: the binding at seq 1 rides only in a VOID record while seq 2 is counted; position 1 is unfilled, a SEQ_GAP, a hole; gap", {"list": [B(k2, r2, 1700000100, 2)], "pick": "candidates", "reason": "gap"}, extra=[rec(k1, root, 50, verdict="VOID"), rec(k2, root, 51)])
    a3v = ent("annotation", 3, eid(k2), {"note_md": "above a binding only a VOID record carries"})
    audit_case("void_only_later_binding_below_counted_annotation_gap", [g0, k1], "s4.2 rule 3: the later binding at seq 2 rides only in a VOID record while a counted annotation at seq 3 sits above it: position 2 is unfilled, a SEQ_GAP, limb 1's hole; gap", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "gap"}, extra=[rec(k2, root, 50, verdict="VOID"), rec(a3v, root, 51)])
    # ---- round 1 additions (2026-10-23 revision): retirement across chains, migration, a retired key's commitments
    S1 = addr("succ")

    def at(b, sender, bn, ts, chain=1, verdict="counted"):
        return dict(rec(b, sender, bn, verdict=verdict, ts=ts), chainId=chain)

    def nothing_at(h, sender, bn, ts, verdict="counted"):
        return {"chainId": 1, "blockNumber": bn, "blockTimestamp": ts, "tx": "0x%064x" % (0xA0000 + bn), "sender": sender, "hash": h, "verdict": verdict}

    def two_chain_case(slug, records, note, predicted):
        bas = {"chains": [{"chainId": c, "fromBlock": 0, "toBlock": 1000, "registries": [],
                           "senders": sorted({root, S1, addr("other")}, key=lambda a: a.encode())} for c in (1, 5)]}
        audit_case(slug, [], note, predicted, raw=json.dumps({"root": root, "anchors": records, "basis": bas}).encode("utf-8"))

    b2r = kb(2, eid(k1), r2, 1700000100)
    s3 = ent("succession", 3, eid(b2r), {"to": S1, "kind": "keyrotation", "effective": 1700000000, "statement_md": "rotated"})
    skew = [at(g0, root, 10, 900), at(k1, root, 11, 910), at(b2r, root, 12, 1000), at(s3, root, 13, 1010)]
    two_chain_case("skew_succession_void_on_another_chain_earlier", skew + [at(s3, root, 3, 995, chain=5, verdict="VOID")],
                   "s4.1 with galeed/1 s7.4: the succession carried by the root's VOID record on another chain at 995 retires it there for every chain; its binding at 1000 and its succession at 1010 are DISCARDED, and the pick falls back to the binding below them with no hole", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"})
    two_chain_case("skew_succession_counted_on_another_chain_earlier", skew + [at(s3, root, 3, 995, chain=5)],
                   "s4.2 rule 3: the same succession counted on the other chain at 995 is a ledger entry; the binding at 1000 is DISCARDED and its position a hole; gap", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "gap"})
    two_chain_case("skew_successor_reanchors_binding", skew + [at(s3, root, 3, 995, chain=5, verdict="VOID"), at(b2r, S1, 20, 1100)],
                   "s4.1 with galeed/1 s8.6: the successor re-anchors the binding the skew left after the retirement; one", {"binding": B(b2r, r2, 1700000100, 2), "pick": "one"})
    mig = ent("migration", 2, eid(k1), {"spec": "galeed/2", "statement_md": "moving to the next law"})
    mg = [at(g0, root, 10, 900), at(k1, root, 11, 910), at(mig, root, 12, 1000)]
    two_chain_case("migration_closed_ledger_one", mg, "s4.1/s6: a ledger closed by a migration answers with its last binding; one", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"})
    b3m = kb(3, eid(mig), r3, 1700000300)
    two_chain_case("migration_binding_above_same_second", mg + [at(b3m, root, 13, 1000)],
                   "s4.1 with galeed/1 s7.4: a binding above the migration, carried at the migration's own second, is DISCARDED; one names the last binding below the migration", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"})
    two_chain_case("migration_binding_above_later", mg + [at(b3m, root, 13, 1001)],
                   "s4.1 with galeed/1 s7.4: the binding above the migration carried a second later is DISCARDED; one names the last binding below the migration", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"})
    nl = G.make_entry(k, "genesis", 0, None, {"statement_md": "the next law's genesis", "continues": eid(mig)}, spec="galeed/2")
    two_chain_case("migration_new_law_genesis_same_second", mg + [at(nl, root, 14, 1000)],
                   "s4.2 rule 4 with galeed/1 s8.4: new-law bytes the key carries at the migration's second are CARRIED_MALFORMED; broken", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "broken"})
    two_chain_case("migration_new_law_genesis_later", mg + [at(nl, root, 14, 1001)],
                   "s4.2 rule 3 gloss: new-law bytes carried a second later by the retired key are DISCARDED and MALFORMED, no finding and no hole; one", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"})
    s1r = ent("succession", 1, eid(g0), {"to": S1, "kind": "keyrotation", "effective": 1700000000, "statement_md": "rotated"})
    k2r = kb(2, eid(s1r), r2, 1700000100, kk=key("succ"))
    k3h = kb(3, eid(k2r), r3, 1700000300, kk=key("succ"))
    rr = [at(g0, root, 10, 900), at(s1r, root, 11, 1000), at(k2r, S1, 20, 1100)]
    two_chain_case("retired_root_unproven_commitment_after", rr + [at(k3h, root, 30, 1001, verdict="UNPROVEN")],
                   "s4.2 rule 3: the retired root's UNPROVEN record of a later binding, a second after its retirement, is DISCARDED and in no unproven row: no hole; one", {"binding": B(k2r, r2, 1700000100, 2), "pick": "one"})
    two_chain_case("retired_root_unproven_commitment_same_second", rr + [at(k3h, root, 30, 1000, verdict="UNPROVEN")],
                   "s4.2 rule 3: at the retirement's second the root's UNPROVEN record of a hash no ledger entry has is a hole; gap", {"list": [B(k2r, r2, 1700000100, 2)], "pick": "candidates", "reason": "gap"})
    two_chain_case("retired_root_hash_only_commitment_after", rr + [nothing_at(eid(k3h), root, 30, 1001)],
                   "s4.2 rule 3: the retired root's counted record carrying only a later binding's hash, after its retirement, is DISCARDED: no MISSING row, no hole; one", {"binding": B(k2r, r2, 1700000100, 2), "pick": "one"})
    two_chain_case("retired_root_hash_only_commitment_same_second", rr + [nothing_at(eid(k3h), root, 30, 1000)],
                   "s4.2 rule 3: at the retirement's second the root's commitment to the hash is MISSING, a hole; gap", {"list": [B(k2r, r2, 1700000100, 2)], "pick": "candidates", "reason": "gap"})
    # round 2: the retirement read whatever the record's sender and verdict, and the retired key's records beside a commitment
    two_chain_case("skew_succession_strangers_record_on_another_chain_earlier", skew + [at(s3, addr("other"), 3, 995, chain=5)],
                   "s4.1 with galeed/1 s7.4: a stranger's record on another chain at 995 carrying the root's succession retires the root whatever its sender; the binding at 1000 and the succession at 1010 are DISCARDED; one names the binding below", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"})
    two_chain_case("skew_succession_unproven_on_another_chain_earlier", skew + [at(s3, root, 3, 995, chain=5, verdict="UNPROVEN")],
                   "s4.1, s4.2 rule 3 with galeed/1 s7.4: the root's UNPROVEN record on another chain at 995 retires it whatever its verdict; the binding and the succession are DISCARDED, and the UNPROVEN row's hash is no ledger entry, a hole; gap", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "gap"})
    mig3 = ent("migration", 3, eid(b2r), {"spec": "galeed/2", "statement_md": "moving to the next law"})
    two_chain_case("migration_first_carried_void_on_another_chain_earlier", [at(g0, root, 10, 900), at(k1, root, 11, 910), at(b2r, root, 12, 1000), at(mig3, root, 13, 1010), at(mig3, root, 3, 995, chain=5, verdict="VOID")],
                   "s6 with galeed/1 s7.4: the migration's first record, VOID on another chain at 995, retires the root before its last binding was carried; that binding is DISCARDED, no key can re-anchor it, and one names the binding below", {"binding": B(k1, r1, 1700000000, 1), "pick": "one"})
    for when, ts, hole in (("after", 1150, True), ("same_second", 1000, False)):
        two_chain_case("missing_bytes_only_in_retired_root_record_" + when, rr + [nothing_at(eid(k3h), S1, 40, 1200), at(k3h, root, 35, ts)],
                       "s4.2 rule 3: the successor's counted record commits to its later binding's hash alone, and the bytes ride only in the root's record at %d, %s" % (ts, "which the trim of galeed/1 s8.1 discarded: a hole; gap" if hole else "the root's retirement second, where the record counts: the binding is a ledger entry; one"),
                       {"list": [B(k2r, r2, 1700000100, 2)], "pick": "candidates", "reason": "gap"} if hole else {"binding": B(k3h, r3, 1700000300, 3), "pick": "one"})
        two_chain_case("unproven_nobytes_bytes_only_in_retired_root_record_" + when, rr + [nothing_at(eid(k3h), S1, 40, 1200, verdict="UNPROVEN"), at(k3h, root, 35, ts)],
                       "s4.2 rule 3: the successor's UNPROVEN record commits to the hash alone and the bytes ride only in the root's record at %d; %s" % (ts, "discarded, a hole; gap" if hole else "at the retirement's second the binding is a ledger entry; one"),
                       {"list": [B(k2r, r2, 1700000100, 2)], "pick": "candidates", "reason": "gap"} if hole else {"binding": B(k3h, r3, 1700000300, 3), "pick": "one"})
        for slug, b, what in (("malformed_bytes", junk, "bytes that are no entry"), ("foreign_entry", kx, "a stranger's binding")):
            two_chain_case("missing_%s_in_retired_root_record_%s" % (slug, when), rr + [nothing_at(eid(b), S1, 40, 1200), at(b, root, 35, ts)],
                           "s4.2 rule 3: the successor commits to the hash of %s that ride only in the root's record at %d; %s" % (what, ts, "discarded: MALFORMED or EXCLUDED, no hole and no finding; one" if hole else "counted at the retirement's second: CARRIED_MALFORMED, a hard finding; broken"),
                           {"binding": B(k2r, r2, 1700000100, 2), "pick": "one"} if hole else {"list": [B(k2r, r2, 1700000100, 2)], "pick": "candidates", "reason": "broken"})
    two_chain_case("void_and_discarded_carriers_of_a_later_binding", rr + [at(k3h, S1, 40, 1200, verdict="VOID"), at(k3h, root, 41, 1150)],
                   "s4.2 rule 3: a later binding whose bytes only the successor's VOID record and the retired root's discarded record carry, its hash committed to by no other record, is no act of the ledger and no hole; one", {"binding": B(k2r, r2, 1700000100, 2), "pick": "one"})
    b2late = kb(2, eid(k1), r2, 1700000100)
    s3b = ent("succession", 3, eid(b2late), {"to": S1, "kind": "keyrotation", "effective": 1700000000, "statement_md": "rotated"})
    own_late = [at(g0, root, 10, 900), at(k1, root, 11, 910), at(s3b, root, 13, 1010), at(b2late, root, 14, 1011)]
    two_chain_case("root_binding_anchored_after_its_retirement", own_late,
                   "s4.2 rule 3 with galeed/1 s7.4: the root's own binding at a position it held, anchored only after its retirement, is DISCARDED and its position a hole; gap", {"list": [B(k1, r1, 1700000000, 1)], "pick": "candidates", "reason": "gap"})
    two_chain_case("root_binding_reanchored_by_successor", own_late + [at(b2late, S1, 20, 1100)],
                   "s4.1 with galeed/1 s8.6: the successor re-anchors that binding; one", {"binding": B(b2late, r2, 1700000100, 2), "pick": "one"})
    case("recipient-check", ["-h"], "HARNESS: -h is a recipient argument, not an option; it fails the form", {"ok": False, "token": "E_RECIPIENT"}, path="(argument)")
    bdd = canon(manifest_obj())
    with open(os.path.join(HERE, "--"), "wb") as f:
        f.write(bdd)
    case("check", ["--"], "HARNESS: a file named -- is a path like any other", {"manifest": sha(bdd), "ok": True}, path="--")
    case("pick-recipient", ["audits/a001_one.json", "x"], "HARNESS: an argument beyond the list", None)

    case("pick-recipient", [], "HARNESS: a missing argument", None, path="(none)")
    case("pick-recipient", ["audits/absent.json"], "HARNESS: a path the command cannot read", None, absent=["audits/absent.json"])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=1)
    a = ap.parse_args()
    RNG.seed(a.seed)
    for cat in _n:
        d = os.path.join(HERE, cat)
        shutil.rmtree(d, ignore_errors=True)
        os.makedirs(d)
    gen_check()
    gen_digest()
    gen_recipient()
    gen_pick()
    with open(os.path.join(HERE, "manifest.json"), "w", encoding="utf-8") as f:
        json.dump({"seed": a.seed, "cases": MANIFEST}, f, ensure_ascii=False, indent=1)
    print("cases:", len(MANIFEST))


if __name__ == "__main__":
    main()
