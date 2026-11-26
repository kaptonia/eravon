"""courtevidence/1 (docs/courtevidence-v1.md): the manifest (s2), the
recipient form (s3.2), and the pick (s4), over the frozen galeed/1 core."""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "impl-py"))

from glcanon import Fault, Obj, accept                     # noqa: E402
from glcrypto import sha256                                # noqa: E402
from glentry import (is_hex, is_int, is_prose,             # noqa: E402
                     is_skeleton_nonempty, validate)
import glentry                                             # noqa: E402
import glaudit                                             # noqa: E402

SPEC = "courtevidence/1"
MANIFEST_KEYS = ("spec", "subjectRef", "party", "files")
FILE_KEYS = ("name", "sha256", "bytes", "mediaType")
ZIKARON_KEYS = frozenset(("root", "chainId", "tx", "entry"))
FILE_TOKENS = frozenset(("E_FILE", "E_FILE_MISSING", "E_FILE_CLOSED", "E_FILE_NAME",
                         "E_FILE_SHA256", "E_FILE_BYTES", "E_FILE_MEDIATYPE", "E_FILE_ZIKARON"))
_HEXLOW = frozenset("0123456789abcdef")


class ManifestFault(Exception):
    def __init__(self, token, index=None):
        super().__init__(token)
        self.token = token
        self.index = index


# --------------------------------------------------------------------------
# s2 the manifest
# --------------------------------------------------------------------------

def _subject_ref_ok(v):
    """galeed/1 s6.3's subjectRef row, decided by the frozen core (s8.4)."""
    try:
        glentry._check_subject_ref(v)
    except Fault:
        return False
    return True


def _entry_spelling_ok(v):
    """s2.2: 0x followed by an even number of lowercase hexadecimal digits."""
    if not isinstance(v, str) or len(v) < 2 or v[0] != "0" or v[1] != "x":
        return False
    body = v[2:]
    return len(body) % 2 == 0 and all(c in _HEXLOW for c in body)


def _zikaron_ok(v):
    """s2.2's zikaron row: an object with exactly the four members, each in form."""
    if not isinstance(v, Obj):
        return False
    d = v.dict()
    if frozenset(d.keys()) != ZIKARON_KEYS:
        return False
    return (is_hex(d["root"], 20) and is_int(d["chainId"]) and is_hex(d["tx"], 32)
            and _entry_spelling_ok(d["entry"]))


def check(b):
    """s2.3.  Returns the parsed value; raises Fault (galeed/1 s3) or
    ManifestFault."""
    v = accept(b)
    if not isinstance(v, Obj):
        raise ManifestFault("E_MANIFEST")
    d = v.dict()
    for k in MANIFEST_KEYS:
        if k not in d:
            raise ManifestFault("E_MANIFEST_MISSING")
    for k in d:
        if k not in MANIFEST_KEYS and k != "note_md":
            raise ManifestFault("E_MANIFEST_CLOSED")
    if not (isinstance(d["spec"], str) and d["spec"] == SPEC):
        raise ManifestFault("E_MANIFEST_SPEC")
    if not _subject_ref_ok(d["subjectRef"]):
        raise ManifestFault("E_MANIFEST_SUBJECT")
    if not is_hex(d["party"], 20):
        raise ManifestFault("E_MANIFEST_PARTY")
    if not isinstance(d["files"], list):
        raise ManifestFault("E_MANIFEST_FILES")
    for i, e in enumerate(d["files"]):
        if not isinstance(e, Obj):
            raise ManifestFault("E_FILE", i)
        ed = e.dict()
        for k in FILE_KEYS:
            if k not in ed:
                raise ManifestFault("E_FILE_MISSING", i)
        for k in ed:
            if k not in FILE_KEYS and k != "zikaron":
                raise ManifestFault("E_FILE_CLOSED", i)
        if not is_skeleton_nonempty(ed["name"]):
            raise ManifestFault("E_FILE_NAME", i)
        if not is_hex(ed["sha256"], 32):
            raise ManifestFault("E_FILE_SHA256", i)
        if not is_int(ed["bytes"]):
            raise ManifestFault("E_FILE_BYTES", i)
        if not is_skeleton_nonempty(ed["mediaType"]):
            raise ManifestFault("E_FILE_MEDIATYPE", i)
        if "zikaron" in ed and not _zikaron_ok(ed["zikaron"]):
            raise ManifestFault("E_FILE_ZIKARON", i)
    if "note_md" in d and not is_prose(d["note_md"]):
        raise ManifestFault("E_MANIFEST_NOTE")
    return v


def check_answer(b):
    try:
        check(b)
    except Fault as f:
        return Obj([("ok", False), ("token", f.token)])
    except ManifestFault as f:
        pairs = [("ok", False), ("token", f.token)]
        if f.index is not None:
            pairs.append(("index", f.index))
        return Obj(pairs)
    return Obj([("manifest", "0x" + sha256(b).hex()), ("ok", True)])


# --------------------------------------------------------------------------
# s3.2 the recipient form (Bech32, BIP-173, hrp "age", 32 data bytes)
# --------------------------------------------------------------------------

BECH32 = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"


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


def recipient_ok(s):
    if not isinstance(s, str) or len(s) != 62 or s[:4] != "age1":
        return False
    data = s[4:]
    if any(c not in BECH32 for c in data):
        return False
    hrp = "age"
    values = [ord(c) >> 5 for c in hrp] + [0] + [ord(c) & 31 for c in hrp] \
        + [BECH32.index(c) for c in data]
    if _polymod(values) != 1:
        return False
    # 52 groups of 5 bits = 260 bits: 32 bytes and 4 padding bits, which are zero
    groups = [BECH32.index(c) for c in data[:52]]
    acc = 0
    for g in groups:
        acc = (acc << 5) | g
    return (acc & 0xF) == 0


# --------------------------------------------------------------------------
# s4 the pick
# --------------------------------------------------------------------------

def pick(audit_raw):
    """s4.  `audit_raw` is the bytes of a galeed/1 audit input file.  Returns
    the answer Obj; `unread` where the core yields no report."""
    try:
        report = glaudit.audit(audit_raw).dict()
    except glaudit.NoLabel:
        return Obj([("pick", "unread")])
    (_root, anchors, _basis) = glaudit._read_inputs(glaudit.load_input(audit_raw))
    anchored = set(r.dict()["entry_id"] for r in report["anchored"])
    bindings = []
    seen = set()
    for a in anchors:
        b = a.entry
        if b is None or b in seen:
            continue
        seen.add(b)
        try:
            e = validate(b)
        except Fault:
            continue
        if e.eid not in anchored or e.etype != "keybinding":
            continue
        bd = e.body.dict()
        bindings.append((e.seq, e.eid, bd["recipient"], bd["effective"]))
    if not bindings:
        return Obj([("pick", "none")])
    bindings.sort(key=lambda t: (-t[0], t[1].encode("utf-8")))
    top = bindings[0][0]
    rows = [Obj([("effective", eff), ("entry_id", eid), ("recipient", rc),
                 ("recipientForm", recipient_ok(rc)), ("seq", sq)])
            for sq, eid, rc, eff in bindings]
    findings = [f.dict() for f in report["findings"]]
    if any(f["name"] == "EQUIVOCATION" and f["position"] == top for f in findings):
        return Obj([("list", rows), ("pick", "candidates"), ("reason", "fork")])
    # s4.2 rule 3: a hole is a SEQ_GAP finding, or a missing hash or an
    # unproven row's hash that is the entry_id of no ledger entry, no
    # MALFORMED row and no EXCLUDED row (bytes the scan never read as the
    # seat's act)
    read = set(anchored)
    read.update(m.dict()["entry_id"] for m in report["malformed"])
    read.update(x.dict()["entry_id"] for x in report["excluded"])
    hole = (any(f["name"] == "SEQ_GAP" for f in findings)
            or any(m.dict()["hash"] not in read for m in report["missing"])
            or any(u.dict()["hash"] not in read for u in report["unproven"]))
    if hole:
        return Obj([("list", rows), ("pick", "candidates"), ("reason", "gap")])
    if any(f["hard"] for f in findings):
        return Obj([("list", rows), ("pick", "candidates"), ("reason", "broken")])
    return Obj([("binding", rows[0]), ("pick", "one")])
