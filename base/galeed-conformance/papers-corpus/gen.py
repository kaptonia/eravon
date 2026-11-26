#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
gen.py - corpus generator for the `galeed.papers/1` convergence run.

Written from docs/galeed-papers-v1.md and galeed-conformance/HARNESS-PAPERS.md
alone. The canonical form, the signing primitives, the entry builder and the
audit label predictor are borrowed from ../corpus/gen.py (the galeed/1
generator's own reading, never the frozen core), and keccak256 from
../impl-py/glcrypto.py; everything of the papers law
(document orders, pairing, payload, the ledger as read, the door) is this
generator's own reading and is recorded in manifest.json as a *prediction*, a
third opinion beside the implementations under test.

Usage:  python3 gen.py --seed 1
Populates ./docs, ./sign, ./pair, ./qr, ./door, ./service and writes manifest.json.
"""
import argparse
import base64
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

J, JObj, cbytes, hx = G.J, G.JObj, G.cbytes, G.hx
sha256, eid = G.sha256, G.eid
N_ORDER = 0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141

KN, KP, KR, KF = "galeed.note/1", "galeed.pass/1", "galeed.report/1", "galeed.refusal/1"
KS = "galeed.verifier/1"
sys.path.insert(0, os.path.join(HERE, "..", "impl-py"))
from glcrypto import keccak256  # noqa: E402
CAP = 2953
PREFIX = b"galeed-pass:"
REG = "0x" + "11" * 20
MAXT = 2 ** 53 - 1

MANIFEST = []
_n = {"docs": 0, "sign": 0, "pair": 0, "qr": 0, "door": 0, "service": 0}
RNG = random.Random(1)
KEYS = {}
TAB = chr(9)
LF = chr(10)
DEL = chr(127)


def key(tag):
    if tag not in KEYS:
        KEYS[tag] = RNG.randrange(1, N_ORDER - 1)
    return KEYS[tag]


def addr(tag):
    return G.privkey_to_addr_hex(key(tag))


def H32(i=None):
    return "0x%064x" % (RNG.getrandbits(256) if i is None else i)


def sign_doc(priv, d, domain, sort=True):
    """Canonical bytes of the object with `sig` added under `domain`."""
    o = J(d) if isinstance(d, dict) else d
    presig = sha256(cbytes(o))
    dg = G.eip191_digest(domain, presig)
    r, s, v = G.ecdsa_sign(priv, dg)
    o2 = JObj(list(o.members) + [("sig", G.sig_hex(r, s, v))])
    return cbytes(o2, sort=sort)


def sub(data, old, new, n=1):
    assert data.count(old) >= 1, "marker %r absent" % (old,)
    return data.replace(old, new, n)


def write(cat, slug, data):
    _n[cat] += 1
    name = "%s%03d_%s" % (cat[0], _n[cat], slug)
    path = os.path.join(cat, name + (".txt" if cat == "qr" else ".json"))
    with open(os.path.join(HERE, path), "wb") as f:
        f.write(data)
    return path


def _note_verifier(path):
    """The `verifier` the note at `path` names, where its bytes read to an
    object carrying a hex20 one; the default verifier otherwise."""
    try:
        d = json.loads(open(os.path.join(HERE, path), "rb").read().decode("utf-8"))
        v = d.get("verifier") if isinstance(d, dict) else None
        if isinstance(v, str) and len(v) == 42 and v.startswith("0x") and all(c in "0123456789abcdef" for c in v[2:]):
            return v
    except Exception:
        pass
    return addr("ver")


def case(cmd, args, note, predicted, path=None, absent=None):
    """Every door, pair and pair-refusal case that is no misuse carries the
    caller's `current` (s1): the note's verifier unless the case names one."""
    path = path or args[-1]
    if predicted is not None and cmd == "door" and "--current" not in args and "|nocurrent" not in path:
        v = _note_verifier(args[0])
        args = list(args) + ["--current", v]
        predicted = dict(predicted, current=v)
    elif predicted is not None and cmd == "pair" and len(args) == 2:
        args = list(args) + [_note_verifier(args[0])]
    elif predicted is not None and cmd == "pair-refusal" and len(args) == 3:
        args = list(args) + [_note_verifier(args[0])]
    e = {"cmd": cmd, "args": args, "path": path, "note": note, "predicted": predicted}
    if absent:
        e["absent"] = absent  # the corpus paths the case names that no file carries
    MANIFEST.append(e)


# --------------------------------------------------------------------------
# document builders
# --------------------------------------------------------------------------

def note_obj(**kw):
    d = {"kind": KN, "judge": addr("judge"), "case": {"kind": "hash", "sha256": H32()},
         "verifier": addr("ver"), "protocol": H32(), "scope_md": "check the goods in warehouse 7",
         "issued": 1000, "deadline": 2000}
    d.update(kw)
    return d


def pass_obj(nb, **kw):
    d = {"kind": KP, "verifier": addr("ver"), "noteHash": eid(nb),
         "window": {"from": 1200, "to": 1300}, "orderRef": "SF-1"}
    d.update(kw)
    return d


def report_obj(nb, protocol, **kw):
    d = {"kind": KR, "verifier": addr("ver"), "noteHash": eid(nb), "protocol": protocol,
         "manifest": H32(), "at": 1500, "note_md": ""}
    d.update(kw)
    return d


def refusal_obj(nb, pb, **kw):
    d = {"kind": KF, "verifier": addr("ver"), "noteHash": eid(nb), "passHash": eid(pb), "at": 1250,
         "place_md": "front gate", "orderRef": "", "photos": []}
    d.update(kw)
    return d


def NOTE(**kw):
    return sign_doc(key("judge"), note_obj(**kw), KN)


def PASS(nb, **kw):
    return sign_doc(key("ver"), pass_obj(nb, **kw), KP)


def REPORT(nb, proto, **kw):
    return sign_doc(key("ver"), report_obj(nb, proto, **kw), KR)


def REFUSAL(nb, pb, **kw):
    return sign_doc(key("ver"), refusal_obj(nb, pb, **kw), KF)


def ok(b):
    return {"doc_id": eid(b), "ok": True}


def bad(token, index=None):
    d = {"ok": False, "token": token}
    if index is not None:
        d["index"] = index
    return d


def D(cmd, slug, data, note, pred):
    p = write("docs", slug, data)
    case(cmd, [p], note, pred, p)
    return p


# --------------------------------------------------------------------------
# documents: every closed boundary on both sides
# --------------------------------------------------------------------------

def build_docs():
    nb = NOTE()
    D("note-check", "note_ok", nb, "a well formed note", ok(nb))
    # parent's canonical-form tokens, one per adopted token
    D("note-check", "note_bad_utf8", nb[:-1] + b"\xff", "s3 token first", bad("E_UTF8"))
    D("note-check", "note_not_canonical_ws", sub(nb, b'{"case"', b'{ "case"'), "a space: not canonical", bad("E_NOT_CANONICAL"))
    D("note-check", "note_unsorted", sign_doc(key("judge"), JObj(list(reversed(J(note_obj()).members))), KN, sort=False), "members out of order", bad("E_NOT_CANONICAL"))
    D("note-check", "note_number_float", sub(nb, b'"issued":1000', b'"issued":1000.0'), "a fraction fails s3.2 at the parser", bad("E_NUMBER"))
    D("note-check", "note_dup_key", sub(nb, b'"kind":"hash"', b'"kind":"hash","kind":"hash"'), "a repeated member name inside case", bad("E_DUP_KEY"))
    D("note-check", "note_key_charset", NOTE(case={"kind": "hash", "sha256": H32(), "bad" + TAB + "key": 1}), "a key carrying a tab fails s3.3 (a space is inside the key charset)", bad("E_KEY_CHARSET"))
    b = NOTE(case={"kind": "hash", "sha256": H32(), "a key": 1}); D("note-check", "note_key_space_ok", b, "a key carrying a space is a skeleton string", ok(b))
    deep = {"kind": "hash", "sha256": H32()}
    node = deep
    for _ in range(127):
        node["d"] = {}
        node = node["d"]
    D("note-check", "note_depth", NOTE(case=deep), "a container opening at depth 129", bad("E_DEPTH"))
    D("note-check", "note_value_charset_deep", NOTE(case={"kind": "hash", "sha256": H32(), "x": {"deep": "a" + TAB + "b"}}), "a tab below a non-prose key, at depth: s3.3 subtree rule", bad("E_VALUE_CHARSET"))
    b = NOTE(case={"kind": "hash", "sha256": H32(), "extra_md": {"deep": "a" + TAB + "b"}}); D("note-check", "note_prose_subtree_ok", b, "a tab below a _md key at depth is prose", ok(b))
    D("note-check", "root_array", b"[]", "root is not an object", bad("E_DOC"))
    D("note-check", "root_string", b'"galeed.note/1"', "root is a string", bad("E_DOC"))
    # member set
    D("note-check", "note_missing_scope", sign_doc(key("judge"), {k: v for k, v in note_obj().items() if k != "scope_md"}, KN), "absent key", bad("E_DOC_MISSING"))
    D("note-check", "note_extra_member", sign_doc(key("judge"), dict(note_obj(), extra=1), KN), "foreign key", bad("E_DOC_CLOSED"))
    D("note-check", "note_missing_and_extra", sign_doc(key("judge"), dict({k: v for k, v in note_obj().items() if k != "judge"}, extra=1), KN), "absent before foreign", bad("E_DOC_MISSING"))
    D("note-check", "note_no_sig", cbytes(J(note_obj())), "sig absent is a missing member", bad("E_DOC_MISSING"))
    D("note-check", "note_sig_twice", sub(nb, b'"sig":"0x', b'"sig":"0x00","sig":"0x'), "two top-level sig keys fail at the parser", bad("E_DUP_KEY"))
    b = NOTE(case={"kind": "hash", "sha256": H32(), "sig": "0x00"}); D("note-check", "note_nested_sig_ok", b, "a sig inside case is untouched by s3.1", ok(b))
    # kind
    D("note-check", "note_kind_pass", sign_doc(key("judge"), note_obj(kind=KP), KN), "kind of another paper", bad("E_KIND"))
    D("note-check", "note_kind_v2", sign_doc(key("judge"), note_obj(kind="galeed.note/2"), KN), "a later version", bad("E_KIND"))
    D("note-check", "note_kind_int", sign_doc(key("judge"), note_obj(kind=1), KN), "kind not a string", bad("E_KIND"))
    D("note-check", "note_kind_then_judge", sign_doc(key("judge"), note_obj(kind=KP, judge="0x00"), KN), "kind decided before judge", bad("E_KIND"))
    # judge
    D("note-check", "note_judge_upper", sign_doc(key("judge"), note_obj(judge="0x" + addr("judge")[2:].upper()), KN), "uppercase digits", bad("E_NOTE_JUDGE"))
    D("note-check", "note_judge_short", sign_doc(key("judge"), note_obj(judge=addr("judge")[:-2]), KN), "38 digits", bad("E_NOTE_JUDGE"))
    D("note-check", "note_judge_then_case", sign_doc(key("judge"), note_obj(judge="0x", case=[]), KN), "judge before case", bad("E_NOTE_JUDGE"))
    # case
    for slug, c, exp, why in [
        ("case_evm_ok", {"kind": "evm", "chainId": 1, "contract": addr("c"), "codeHash": H32(), "id": H32()}, None, "evm table"),
        ("case_evm_extra_ok", {"kind": "evm", "chainId": 1, "contract": addr("c"), "codeHash": H32(), "id": H32(), "seat": 3}, None, "extra member permitted"),
        ("case_hash_note_ok", {"kind": "hash", "sha256": H32(), "note_md": "a parcel"}, None, "hash with note_md"),
        ("case_unknown_kind_ok", {"kind": "licence", "anything": None}, None, "unknown kind tested for nothing further"),
        ("case_kind_upper_ok", {"kind": "EVM", "chainId": "x"}, None, "EVM is not evm: an unknown kind"),
        ("case_kind_missing", {"sha256": H32()}, "E_NOTE_CASE", "kind absent"),
        ("case_kind_space", {"kind": "a b"}, "E_NOTE_CASE", "kind not a token"),
        ("case_kind_empty", {"kind": ""}, "E_NOTE_CASE", "kind the empty string"),
        ("case_not_object", "evm", "E_NOTE_CASE", "case a string"),
        ("case_null", None, "E_NOTE_CASE", "case null"),
        ("case_evm_no_id", {"kind": "evm", "chainId": 1, "contract": addr("c"), "codeHash": H32()}, "E_NOTE_CASE", "evm missing id"),
        ("case_evm_chain_str", {"kind": "evm", "chainId": "1", "contract": addr("c"), "codeHash": H32(), "id": H32()}, "E_NOTE_CASE", "chainId a string"),
        ("case_evm_contract_32", {"kind": "evm", "chainId": 1, "contract": H32(), "codeHash": H32(), "id": H32()}, "E_NOTE_CASE", "contract hex32"),
        ("case_evm_codehash_20", {"kind": "evm", "chainId": 1, "contract": addr("c"), "codeHash": addr("c"), "id": H32()}, "E_NOTE_CASE", "codeHash hex20"),
        ("case_hash_bad_digest", {"kind": "hash", "sha256": "0xab"}, "E_NOTE_CASE", "sha256 too short"),
        ("case_hash_no_digest", {"kind": "hash"}, "E_NOTE_CASE", "sha256 absent"),
        ("case_hash_note_int", {"kind": "hash", "sha256": H32(), "note_md": 5}, "E_NOTE_CASE", "note_md not a string"),
        ("case_hash_note_null", {"kind": "hash", "sha256": H32(), "note_md": None}, "E_NOTE_CASE", "note_md present as null is present and refused"),
    ]:
        b = NOTE(case=c)
        D("note-check", "note_" + slug, b, why, ok(b) if exp is None else bad(exp))
    D("note-check", "note_case_then_verifier", sign_doc(key("judge"), note_obj(case=5, verifier=5), KN), "case before verifier", bad("E_NOTE_CASE"))
    # verifier, protocol, scope, issued, deadline, window
    D("note-check", "note_verifier_bad", NOTE(verifier="0x1234"), "verifier not hex20", bad("E_NOTE_VERIFIER"))
    D("note-check", "note_protocol_20", NOTE(protocol=addr("c")), "protocol is hex20 not hex32", bad("E_NOTE_PROTOCOL"))
    D("note-check", "note_scope_null", NOTE(scope_md=None), "scope not a string", bad("E_NOTE_SCOPE"))
    b = NOTE(scope_md=""); D("note-check", "note_scope_empty_ok", b, "empty scope permitted", ok(b))
    b = NOTE(scope_md="仓库 x " + TAB); D("note-check", "note_scope_unicode_ok", b, "prose is unrestricted, a tab included", ok(b))
    D("note-check", "note_issued_str", NOTE(issued="1000"), "issued a string", bad("E_NOTE_ISSUED"))
    D("note-check", "note_deadline_bool", NOTE(deadline=True), "deadline a bool", bad("E_NOTE_DEADLINE"))
    D("note-check", "note_issued_after_deadline", NOTE(issued=2001, deadline=2000), "issued > deadline", bad("E_NOTE_WINDOW"))
    b = NOTE(issued=2000, deadline=2000); D("note-check", "note_issued_eq_deadline_ok", b, "equal permitted", ok(b))
    b = NOTE(issued=0, deadline=(1 << 53) - 1); D("note-check", "note_extreme_ints_ok", b, "universe bounds", ok(b))
    D("note-check", "note_issued_then_deadline", NOTE(issued="x", deadline="y"), "issued decided before deadline", bad("E_NOTE_ISSUED"))
    # signature
    good = note_obj()
    D("note-check", "note_sig_short", cbytes(J(dict(good, sig="0x" + "00" * 64))), "64 bytes", bad("E_SIG_FORM"))
    D("note-check", "note_sig_upper", sub(nb, b'"sig":"0x', b'"sig":"0X'), "0X prefix fails hex65", bad("E_SIG_FORM"))
    presig = sha256(cbytes(J(good))); dg = G.eip191_digest(KN, presig); r, s, v = G.ecdsa_sign(key("judge"), dg)
    D("note-check", "note_sig_v29", cbytes(J(dict(good, sig=G.sig_hex(r, s, 29)))), "v outside 27/28", bad("E_SIG_V"))
    D("note-check", "note_sig_r_zero", cbytes(J(dict(good, sig=G.sig_hex(0, s, v)))), "r = 0", bad("E_SIG_RANGE"))
    D("note-check", "note_sig_s_n", cbytes(J(dict(good, sig=G.sig_hex(r, N_ORDER, v)))), "s = n", bad("E_SIG_RANGE"))
    D("note-check", "note_sig_r_ge_n", cbytes(J(dict(good, sig=G.sig_hex(N_ORDER + 5, s, v)))), "r above the group order fails the range test before recovery", bad("E_SIG_RANGE"))
    D("note-check", "note_sig_high_s", cbytes(J(dict(good, sig=G.sig_hex(r, N_ORDER - s, 27 if v == 28 else 28)))), "high s", bad("E_SIG_HIGH_S"))
    half = (N_ORDER - 1) // 2
    D("note-check", "note_sig_s_half_plus_one", cbytes(J(dict(good, sig=G.sig_hex(r, half + 1, v)))), "s one above (n-1)/2 is high", bad("E_SIG_HIGH_S"))
    D("note-check", "note_sig_no_point", cbytes(J(dict(good, sig=G.sig_hex(5, s, v)))), "r = 5 is no x-coordinate on the curve: no point recovers", bad("E_SIG_RECOVER"))
    D("note-check", "note_sig_wrong_key", sign_doc(key("ver"), good, KN), "signed by the verifier's key", bad("E_SIG_SIGNER"))
    D("note-check", "note_sig_wrong_domain", sign_doc(key("judge"), good, KP), "signed under the pass domain", bad("E_SIG_SIGNER"))
    D("note-check", "note_sig_entry_domain", sign_doc(key("judge"), good, "galeed/1"), "signed under the ledger domain", bad("E_SIG_SIGNER"))
    D("note-check", "note_sig_over_other_bytes", cbytes(J(dict(note_obj(scope_md="changed"), sig=json.loads(nb)["sig"]))), "body changed after signing", bad("E_SIG_SIGNER"))

    # ---- pass ----
    pb = PASS(nb)
    D("pass-check", "pass_ok", pb, "a well formed pass", ok(pb))
    p = write("docs", "pass_as_note", pb)
    case("note-check", [p], "a pass offered as a note", bad("E_DOC_MISSING"), p)
    D("pass-check", "note_as_pass", nb, "a note offered as a pass", bad("E_DOC_MISSING"))
    D("pass-check", "pass_root_array", b"[1]", "root is not an object", bad("E_DOC"))
    D("pass-check", "pass_extra", sign_doc(key("ver"), dict(pass_obj(nb), x=1), KP), "foreign key", bad("E_DOC_CLOSED"))
    D("pass-check", "pass_kind_note", PASS(nb, kind=KN), "note kind on a pass", bad("E_KIND"))
    D("pass-check", "pass_kind_null", PASS(nb, kind=None), "kind null", bad("E_KIND"))
    D("pass-check", "pass_verifier_bad", PASS(nb, verifier="0x00"), "verifier form", bad("E_PASS_VERIFIER"))
    D("pass-check", "pass_notehash_bad", PASS(nb, noteHash="0x" + "a" * 63), "63 digits", bad("E_PASS_NOTE"))
    D("pass-check", "pass_window_array", PASS(nb, window=[1200, 1300]), "window an array", bad("E_PASS_WINDOW"))
    D("pass-check", "pass_window_extra", PASS(nb, window={"from": 1, "to": 2, "tz": 0}), "window with a third member", bad("E_PASS_WINDOW"))
    D("pass-check", "pass_window_missing_to", PASS(nb, window={"from": 1}), "window without to", bad("E_PASS_WINDOW"))
    D("pass-check", "pass_window_from_str", PASS(nb, window={"from": "1", "to": 2}), "from a string", bad("E_PASS_WINDOW"))
    D("pass-check", "pass_window_reversed", PASS(nb, window={"from": 1301, "to": 1300}), "from > to", bad("E_PASS_WINDOW_ORDER"))
    b = PASS(nb, window={"from": 1300, "to": 1300}); D("pass-check", "pass_window_point_ok", b, "from = to permitted", ok(b))
    D("pass-check", "pass_window_reversed_then_order", PASS(nb, window={"from": 9, "to": 1}, orderRef=7), "window order before orderRef", bad("E_PASS_WINDOW_ORDER"))
    D("pass-check", "pass_order_tab", PASS(nb, orderRef="SF" + TAB + "1"), "a tab in a non-prose value is refused by s3.3 at canonical(b)", bad("E_VALUE_CHARSET"))
    D("pass-check", "pass_order_unicode", PASS(nb, orderRef="单号"), "non-ASCII in a non-prose value: s3.3", bad("E_VALUE_CHARSET"))
    D("pass-check", "pass_order_del", PASS(nb, orderRef=DEL), "U+007F in a non-prose value: s3.3", bad("E_VALUE_CHARSET"))
    D("pass-check", "pass_order_int", PASS(nb, orderRef=7), "not a string", bad("E_PASS_ORDER"))
    b = PASS(nb, orderRef="with space ~"); D("pass-check", "pass_order_space_ok", b, "space and tilde are inside", ok(b))
    b = PASS(nb, orderRef=""); D("pass-check", "pass_order_empty_ok", b, "empty permitted", ok(b))
    D("pass-check", "pass_sig_judge_key", sign_doc(key("judge"), pass_obj(nb), KP), "signed by the judge", bad("E_SIG_SIGNER"))
    D("pass-check", "pass_sig_note_domain", sign_doc(key("ver"), pass_obj(nb), KN), "signed under the note domain", bad("E_SIG_SIGNER"))
    D("pass-check", "pass_sig_form", cbytes(J(dict(pass_obj(nb), sig="0x"))), "empty sig", bad("E_SIG_FORM"))
    pg = pass_obj(nb); presig = sha256(cbytes(J(pg))); r, s, v = G.ecdsa_sign(key("ver"), G.eip191_digest(KP, presig))
    D("pass-check", "pass_sig_high_s", cbytes(J(dict(pg, sig=G.sig_hex(r, N_ORDER - s, 27 if v == 28 else 28)))), "high s on a pass", bad("E_SIG_HIGH_S"))

    # ---- report ----
    proto = json.loads(nb)["protocol"]
    rb = REPORT(nb, proto)
    D("report-check", "report_ok", rb, "a well formed report", ok(rb))
    D("report-check", "report_root_null", b"null", "root null", bad("E_DOC"))
    D("report-check", "report_missing_manifest", sign_doc(key("ver"), {k: v for k, v in report_obj(nb, proto).items() if k != "manifest"}, KR), "absent key", bad("E_DOC_MISSING"))
    D("report-check", "report_missing_at", sign_doc(key("ver"), {k: v for k, v in report_obj(nb, proto).items() if k != "at"}, KR), "at absent", bad("E_DOC_MISSING"))
    D("report-check", "report_extra", sign_doc(key("ver"), dict(report_obj(nb, proto), late=True), KR), "foreign key", bad("E_DOC_CLOSED"))
    D("report-check", "report_kind_refusal", REPORT(nb, proto, kind=KF), "refusal kind", bad("E_KIND"))
    D("report-check", "report_kind_int", REPORT(nb, proto, kind=0), "kind an int", bad("E_KIND"))
    D("report-check", "report_verifier_bad", REPORT(nb, proto, verifier=None), "verifier null", bad("E_REPORT_VERIFIER"))
    D("report-check", "report_note_bad", REPORT(nb, proto, noteHash=addr("c")), "noteHash hex20", bad("E_REPORT_NOTE"))
    D("report-check", "report_protocol_bad", REPORT(nb, 1), "protocol int", bad("E_REPORT_PROTOCOL"))
    D("report-check", "report_manifest_bad", REPORT(nb, proto, manifest="0x" + "G" * 64), "manifest with a non-hex digit", bad("E_REPORT_MANIFEST"))
    D("report-check", "report_at_str", REPORT(nb, proto, at="1500"), "at a string", bad("E_REPORT_AT"))
    D("report-check", "report_at_then_text", REPORT(nb, proto, at=None, note_md=None), "at decided before note_md", bad("E_REPORT_AT"))
    D("report-check", "report_text_bad", REPORT(nb, proto, note_md=[]), "note_md an array", bad("E_REPORT_TEXT"))
    b = REPORT(nb, proto, note_md="findings: all sealed"); D("report-check", "report_text_ok", b, "prose", ok(b))
    D("report-check", "report_sig_form", cbytes(J(dict(report_obj(nb, proto), sig="0x" + "ab" * 66))), "66 bytes", bad("E_SIG_FORM"))
    D("report-check", "report_sig_wrong_domain", sign_doc(key("ver"), report_obj(nb, proto), KF), "signed under the refusal domain", bad("E_SIG_SIGNER"))
    D("report-check", "report_sig_judge", sign_doc(key("judge"), report_obj(nb, proto), KR), "signed by the judge", bad("E_SIG_SIGNER"))

    # ---- refusal ----
    fb = REFUSAL(nb, pb)
    D("refusal-check", "refusal_ok_empty_photos", fb, "no photos", ok(fb))
    ph = sorted([H32(), H32(), H32()])
    b = REFUSAL(nb, pb, photos=ph); D("refusal-check", "refusal_ok_three_photos", b, "sorted distinct photos", ok(b))
    b = REFUSAL(nb, pb, photos=[ph[0]]); D("refusal-check", "refusal_ok_one_photo", b, "one photo", ok(b))
    D("refusal-check", "refusal_root_int", b"7", "root an int", bad("E_DOC"))
    D("refusal-check", "refusal_missing_at", sign_doc(key("ver"), {k: v for k, v in refusal_obj(nb, pb).items() if k != "at"}, KF), "absent key", bad("E_DOC_MISSING"))
    D("refusal-check", "refusal_extra", sign_doc(key("ver"), dict(refusal_obj(nb, pb), witness=addr("c")), KF), "foreign key", bad("E_DOC_CLOSED"))
    D("refusal-check", "refusal_kind_report", REFUSAL(nb, pb, kind=KR), "report kind", bad("E_KIND"))
    D("refusal-check", "refusal_kind_bool", REFUSAL(nb, pb, kind=True), "kind a bool", bad("E_KIND"))
    D("refusal-check", "refusal_verifier_bad", REFUSAL(nb, pb, verifier=""), "verifier empty", bad("E_REFUSAL_VERIFIER"))
    D("refusal-check", "refusal_note_bad", REFUSAL(nb, pb, noteHash=0), "noteHash int", bad("E_REFUSAL_NOTE"))
    D("refusal-check", "refusal_pass_bad", REFUSAL(nb, pb, passHash="0x" + "0" * 65), "65 digits", bad("E_REFUSAL_PASS"))
    D("refusal-check", "refusal_at_str", REFUSAL(nb, pb, at="1250"), "at a string", bad("E_REFUSAL_AT"))
    D("refusal-check", "refusal_place_bad", REFUSAL(nb, pb, place_md=False), "place a bool", bad("E_REFUSAL_PLACE"))
    D("refusal-check", "refusal_order_charset", REFUSAL(nb, pb, orderRef=LF), "line feed in orderRef: s3.3 first", bad("E_VALUE_CHARSET"))
    D("refusal-check", "refusal_order_bad", REFUSAL(nb, pb, orderRef=None), "orderRef null", bad("E_REFUSAL_ORDER"))
    D("refusal-check", "refusal_photos_obj", REFUSAL(nb, pb, photos={}), "photos an object: no index", bad("E_REFUSAL_PHOTOS"))
    D("refusal-check", "refusal_photos_str", REFUSAL(nb, pb, photos=ph[0]), "photos a string: no index", bad("E_REFUSAL_PHOTOS"))
    D("refusal-check", "refusal_photos_elem_0", REFUSAL(nb, pb, photos=[7]), "element 0 malformed", bad("E_REFUSAL_PHOTO", 0))
    D("refusal-check", "refusal_photos_elem_1", REFUSAL(nb, pb, photos=[ph[0], "0xzz"]), "second element malformed", bad("E_REFUSAL_PHOTO", 1))
    D("refusal-check", "refusal_photos_elem_first_of_two_bad", REFUSAL(nb, pb, photos=[7, "0xzz"]), "first offending index", bad("E_REFUSAL_PHOTO", 0))
    D("refusal-check", "refusal_photos_dup", REFUSAL(nb, pb, photos=[ph[0], ph[1], ph[0]]), "repeat at 2, unsorted too: dup decided first", bad("E_REFUSAL_DUP_PHOTO", 2))
    D("refusal-check", "refusal_photos_dup_adjacent", REFUSAL(nb, pb, photos=[ph[0], ph[0]]), "adjacent repeat", bad("E_REFUSAL_DUP_PHOTO", 1))
    D("refusal-check", "refusal_photos_unsorted", REFUSAL(nb, pb, photos=[ph[1], ph[0], ph[2]]), "element 1 orders before 0", bad("E_REFUSAL_PHOTO_ORDER", 1))
    D("refusal-check", "refusal_photos_unsorted_late", REFUSAL(nb, pb, photos=[ph[0], ph[2], ph[1]]), "element 2 orders before 1", bad("E_REFUSAL_PHOTO_ORDER", 2))
    D("refusal-check", "refusal_photos_form_before_dup", REFUSAL(nb, pb, photos=[ph[0], ph[0], "x"]), "form decided before distinctness", bad("E_REFUSAL_PHOTO", 2))
    D("refusal-check", "refusal_sig_form", cbytes(J(dict(refusal_obj(nb, pb), sig=5))), "sig an int", bad("E_SIG_FORM"))
    D("refusal-check", "refusal_sig_judge", sign_doc(key("judge"), refusal_obj(nb, pb), KF), "signed by the judge", bad("E_SIG_SIGNER"))
    D("refusal-check", "refusal_sig_report_domain", sign_doc(key("ver"), refusal_obj(nb, pb), KR), "signed under the report domain", bad("E_SIG_SIGNER"))
    h1, h2 = H32(), H32()
    lo32, hi32 = sorted([h1, h2])
    D("refusal-check", "refusal_dup_photo_abba", REFUSAL(nb, pb, photos=[lo32, hi32, hi32, lo32]), "the least index whose element equals an earlier element: 2", bad("E_REFUSAL_DUP_PHOTO", 2))
    # round two: adjacencies of the orders, and the parent's tokens per document
    D("note-check", "note_canonical_before_member", sign_doc(key("judge"), JObj(list(reversed(J(note_obj(judge="0x")).members))), KN, sort=False), "out of order and a bad judge: canonical first", bad("E_NOT_CANONICAL"))
    D("note-check", "note_closed_then_kind", sign_doc(key("judge"), dict(note_obj(kind=KP), extra=1), KN), "foreign key before kind", bad("E_DOC_CLOSED"))
    D("note-check", "note_verifier_then_protocol", NOTE(verifier=1, protocol=2), "verifier before protocol", bad("E_NOTE_VERIFIER"))
    D("note-check", "note_protocol_then_scope", NOTE(protocol=2, scope_md=3), "protocol before scope", bad("E_NOTE_PROTOCOL"))
    D("note-check", "note_scope_then_issued", NOTE(scope_md=3, issued="x"), "scope before issued", bad("E_NOTE_SCOPE"))
    D("note-check", "note_window_then_sig_form", cbytes(J(dict(note_obj(issued=5, deadline=4), sig="0x"))), "window before sig form", bad("E_NOTE_WINDOW"))
    D("note-check", "note_sig_v_then_range", cbytes(J(dict(good, sig=G.sig_hex(0, s, 29)))), "v before range", bad("E_SIG_V"))
    D("note-check", "note_sig_range_then_high_s", cbytes(J(dict(good, sig=G.sig_hex(0, N_ORDER - s, v)))), "range before high s", bad("E_SIG_RANGE"))
    D("note-check", "note_sig_high_s_then_recover", cbytes(J(dict(good, sig=G.sig_hex(5, N_ORDER - s, v)))), "high s before recovery", bad("E_SIG_HIGH_S"))
    D("note-check", "note_sig_s_half_ok_low", cbytes(J(dict(good, sig=G.sig_hex(r, half, v)))), "s = (n-1)/2 passes low-s and fails the signer", bad("E_SIG_SIGNER"))
    D("note-check", "note_sig_r_n", cbytes(J(dict(good, sig=G.sig_hex(N_ORDER, s, v)))), "r = n", bad("E_SIG_RANGE"))
    D("note-check", "note_sig_s_zero", cbytes(J(dict(good, sig=G.sig_hex(r, 0, v)))), "s = 0", bad("E_SIG_RANGE"))
    D("note-check", "note_sig_r_n_minus_1", cbytes(J(dict(good, sig=G.sig_hex(N_ORDER - 1, s, v)))), "r = n - 1 is in range and no point", bad("E_SIG_RECOVER"))
    D("pass-check", "pass_kind_then_verifier", PASS(nb, kind=1, verifier=1), "kind before verifier", bad("E_KIND"))
    D("pass-check", "pass_window_to_str", PASS(nb, window={"from": 1, "to": "2"}), "to a string", bad("E_PASS_WINDOW"))
    D("pass-check", "pass_window_int", PASS(nb, window=7), "window an int", bad("E_PASS_WINDOW"))
    D("pass-check", "pass_sig_v29", cbytes(J(dict(pg, sig=G.sig_hex(r, s, 29)))), "v outside 27/28 on a pass", bad("E_SIG_V"))
    D("pass-check", "pass_truncated", pb[:-5], "pass bytes cut: E_JSON", bad("E_JSON"))
    rg = report_obj(nb, proto); rpre = sha256(cbytes(J(rg))); rr, rs, rv = G.ecdsa_sign(key("ver"), G.eip191_digest(KR, rpre))
    D("report-check", "report_not_canonical", sub(rb, b'{"at"', b'{ "at"'), "a space in a report", bad("E_NOT_CANONICAL"))
    D("report-check", "report_dup_key", sub(rb, b'"at":1500', b'"at":1500,"at":1500'), "a repeated member in a report", bad("E_DUP_KEY"))
    D("report-check", "report_truncated", rb[:-7], "report bytes cut: E_JSON", bad("E_JSON"))
    D("report-check", "report_missing_and_extra", sign_doc(key("ver"), dict({k: v for k, v in rg.items() if k != "at"}, extra=1), KR), "absent before foreign, on a report", bad("E_DOC_MISSING"))
    D("report-check", "report_note_then_protocol", REPORT(nb, 1, noteHash=1), "noteHash before protocol", bad("E_REPORT_NOTE"))
    D("report-check", "report_sig_r_zero", cbytes(J(dict(rg, sig=G.sig_hex(0, rs, rv)))), "r = 0 on a report", bad("E_SIG_RANGE"))
    fg = refusal_obj(nb, pb); fpre = sha256(cbytes(J(fg))); fr, fs, fv = G.ecdsa_sign(key("ver"), G.eip191_digest(KF, fpre))
    D("refusal-check", "refusal_truncated", fb[:-3], "refusal bytes cut: E_JSON", bad("E_JSON"))
    D("refusal-check", "refusal_missing_and_extra", sign_doc(key("ver"), dict({k: v for k, v in fg.items() if k != "photos"}, extra=1), KF), "absent before foreign, on a refusal", bad("E_DOC_MISSING"))
    D("refusal-check", "refusal_at_then_place", REFUSAL(nb, pb, at="x", place_md=1), "at before place", bad("E_REFUSAL_AT"))
    D("refusal-check", "refusal_sig_r5", cbytes(J(dict(fg, sig=G.sig_hex(5, fs, fv)))), "no point on a refusal", bad("E_SIG_RECOVER"))
    D("refusal-check", "refusal_sig_high_s", cbytes(J(dict(fg, sig=G.sig_hex(fr, N_ORDER - fs, 27 if fv == 28 else 28)))), "high s on a refusal", bad("E_SIG_HIGH_S"))
    # every consecutive pair of the three remaining orders, one two-fault document each (s13.1)
    build_adjacencies(nb, pb, proto)
    return nb, pb, rb, fb, proto


def build_adjacencies(nb, pb, proto):
    """s13.1: for each order of s5.2, s6.2, s7.2, every consecutive pair
    (A, B) a single document can fail both of, one document each; the
    answer is A.  A step is (token, mutation); mutations compose unless the
    pair names its own."""
    def sig_of(obj, priv, domain):
        return G.ecdsa_sign(priv, G.eip191_digest(domain, sha256(cbytes(J(obj)))))
    def drop(k):
        return lambda o: dict((x, y) for x, y in o.items() if x != k)
    SIG = [
        ("E_SIG_FORM", lambda o, rsv: "0x"),
        ("E_SIG_V", lambda o, rsv: G.sig_hex(rsv[0], rsv[1], 29)),
        ("E_SIG_RANGE", lambda o, rsv: G.sig_hex(0, rsv[1], rsv[2])),
        ("E_SIG_HIGH_S", lambda o, rsv: G.sig_hex(rsv[0], N_ORDER - rsv[1], 27 if rsv[2] == 28 else 28)),
        ("E_SIG_RECOVER", lambda o, rsv: G.sig_hex(5, rsv[1], rsv[2])),
    ]
    SIGPAIR = {
        ("E_SIG_V", "E_SIG_RANGE"): lambda rsv: G.sig_hex(0, rsv[1], 29),
        ("E_SIG_RANGE", "E_SIG_HIGH_S"): lambda rsv: G.sig_hex(0, N_ORDER - rsv[1], rsv[2]),
        ("E_SIG_HIGH_S", "E_SIG_RECOVER"): lambda rsv: G.sig_hex(5, N_ORDER - rsv[1], rsv[2]),
    }
    orders = {
        "pass-check": (lambda: pass_obj(nb), key("ver"), KP, [
            ("E_DOC_MISSING", drop("window")),
            ("E_DOC_CLOSED", lambda o: dict(o, extra=1)),
            ("E_KIND", lambda o: dict(o, kind=1)),
            ("E_PASS_VERIFIER", lambda o: dict(o, verifier=1)),
            ("E_PASS_NOTE", lambda o: dict(o, noteHash=1)),
            ("E_PASS_WINDOW", lambda o: dict(o, window=1)),
            ("E_PASS_WINDOW_ORDER", lambda o: dict(o, window={"from": 2, "to": 1})),
            ("E_PASS_ORDER", lambda o: dict(o, orderRef=1)),
        ], {("E_PASS_WINDOW", "E_PASS_WINDOW_ORDER"): lambda o: dict(o, window={"from": 2, "to": 1, "x": 0})}, {}),
        "report-check": (lambda: report_obj(nb, proto), key("ver"), KR, [
            ("E_DOC_MISSING", drop("manifest")),
            ("E_DOC_CLOSED", lambda o: dict(o, extra=1)),
            ("E_KIND", lambda o: dict(o, kind=1)),
            ("E_REPORT_VERIFIER", lambda o: dict(o, verifier=1)),
            ("E_REPORT_NOTE", lambda o: dict(o, noteHash=1)),
            ("E_REPORT_PROTOCOL", lambda o: dict(o, protocol=1)),
            ("E_REPORT_MANIFEST", lambda o: dict(o, manifest=1)),
            ("E_REPORT_AT", lambda o: dict(o, at="x")),
            ("E_REPORT_TEXT", lambda o: dict(o, note_md=1)),
        ], {}, {}),
        "refusal-check": (lambda: refusal_obj(nb, pb), key("ver"), KF, [
            ("E_DOC_MISSING", drop("photos")),
            ("E_DOC_CLOSED", lambda o: dict(o, extra=1)),
            ("E_KIND", lambda o: dict(o, kind=1)),
            ("E_REFUSAL_VERIFIER", lambda o: dict(o, verifier=1)),
            ("E_REFUSAL_NOTE", lambda o: dict(o, noteHash=1)),
            ("E_REFUSAL_PASS", lambda o: dict(o, passHash=1)),
            ("E_REFUSAL_AT", lambda o: dict(o, at="x")),
            ("E_REFUSAL_PLACE", lambda o: dict(o, place_md=1)),
            ("E_REFUSAL_ORDER", lambda o: dict(o, orderRef=1)),
            ("E_REFUSAL_PHOTOS", lambda o: dict(o, photos=1)),
            ("E_REFUSAL_PHOTO", lambda o: dict(o, photos=["x", H32(1)])),
            ("E_REFUSAL_DUP_PHOTO", lambda o: dict(o, photos=[H32(1), H32(1)])),
            ("E_REFUSAL_PHOTO_ORDER", lambda o: dict(o, photos=[H32(2), H32(1)])),
        ], {("E_REFUSAL_PHOTO", "E_REFUSAL_DUP_PHOTO"): lambda o: dict(o, photos=["x", H32(1), H32(1)]),
            ("E_REFUSAL_DUP_PHOTO", "E_REFUSAL_PHOTO_ORDER"): lambda o: dict(o, photos=[H32(2), H32(1), H32(1)])},
           {"E_REFUSAL_PHOTO": 0, "E_REFUSAL_DUP_PHOTO": 1, "E_REFUSAL_PHOTO_ORDER": 1,
            ("E_REFUSAL_DUP_PHOTO", "E_REFUSAL_PHOTO_ORDER"): 2}),
    }
    for cmd, (mk, priv, domain, steps, own, idx) in orders.items():
        full = steps + [(t, None) for t, _ in SIG]
        for i in range(len(full) - 1):
            (a, ma), (b, mb) = full[i], full[i + 1]
            if (a, b) in (("E_REFUSAL_PHOTOS", "E_REFUSAL_PHOTO"), ("E_SIG_FORM", "E_SIG_V")):
                continue  # no one document fails both
            obj = mk()
            if (a, b) in own:
                obj = own[(a, b)](obj)
            else:
                if ma is not None:
                    obj = ma(obj)
                if mb is not None:
                    obj = mb(obj)
            if b.startswith("E_SIG_"):
                rsv = sig_of(obj, priv, domain)
                sig = SIGPAIR[(a, b)](rsv) if a.startswith("E_SIG_") else dict(SIG)[b](obj, rsv)
                data = cbytes(J(dict(obj, sig=sig)))
            else:
                data = sign_doc(priv, obj, domain)
            slug = "adj_%s_%s_then_%s" % (cmd.split("-")[0], a.lower(), b.lower())
            k = idx.get((a, b), idx.get(a))
            D(cmd, slug, data, "order adjacency: %s decided before %s" % (a, b), bad(a, k))


# --------------------------------------------------------------------------
# sign: one case per domain, plus the preimage rules
# --------------------------------------------------------------------------

def build_sign(nb, pb, proto):
    def S(slug, priv, obj, domain, note):
        b = cbytes(J(obj))
        p = write("sign", slug, b)
        presig = sha256(b)
        dg = G.eip191_digest(domain, presig)
        r, s, v = G.ecdsa_sign(priv, dg)
        pred = {"digest": hx(dg), "presig": hx(presig), "sig": G.sig_hex(r, s, v), "signer": G.privkey_to_addr_hex(priv)}
        case("sign", ["0x%064x" % priv, p, domain], note, pred, p)
    S("note", key("judge"), note_obj(), KN, "a note preimage under the note domain")
    S("pass", key("ver"), pass_obj(nb), KP, "a pass preimage under the pass domain")
    S("report", key("ver"), report_obj(nb, proto), KR, "a report preimage under the report domain")
    S("refusal", key("ver"), refusal_obj(nb, pb), KF, "a refusal preimage under the refusal domain")
    S("note_with_sig_inside", key("judge"), dict(note_obj(), sig="0x00"), KN, "a top-level sig is signed inside the preimage")
    S("loose_json", key("ver"), pass_obj(nb), KP, "the file is re-canonicalized")
    _canon_copy = MANIFEST[-1]["args"][1]
    MANIFEST[-1]["args"][1] = write("sign", "loose_json_bytes", json.dumps(pass_obj(nb), indent=2).encode())
    os.remove(os.path.join(HERE, _canon_copy))
    MANIFEST[-1]["path"] = MANIFEST[-1]["args"][1]
    for slug, spell in [("key_0X_lower", "0X%064x"), ("key_0x_upper", "0x%064X"), ("key_bare_upper", "%064X")]:
        S("spelling_" + slug, key("judge"), note_obj(), KN, "the private key spelled " + slug)
        MANIFEST[-1]["args"][0] = spell % key("judge")
    p = write("sign", "misuse_domain", cbytes(J(note_obj())))
    case("sign", ["0x%064x" % key("judge"), p, "galeed/1"], "the ledger domain is outside the four: misuse", None, p)
    case("sign", ["0x%064x" % key("judge"), p, "galeed.note/2"], "a later literal: misuse", None, p + "|v2")
    case("sign", ["0x" + "0" * 64, p, KN], "zero scalar: misuse", None, p + "|zero")
    case("sign", ["0x%064x" % N_ORDER, p, KN], "the group order: misuse", None, p + "|n")
    for slug, raw, why in (("dup_key", b'{"kind":"galeed.note/1","a":1,"a":2}', "a duplicate key"),
                           ("key_charset", b'{"kind":"galeed.note/1","k\\u0001":1}', "a key outside U+0020 through U+007E"),
                           ("value_charset", b'{"kind":"galeed.note/1","v":"a\\u0001b"}', "a string value outside the charset"),
                           ("invalid_utf8", b'{"kind":"galeed.note/1","v":"\xff"}', "bytes that are not UTF-8")):
        pm = write("sign", "misuse_preimage_" + slug, raw)
        case("sign", ["0x%064x" % key("judge"), pm, KN], "HARNESS: a preimage failing galeed/1 s3.5 (%s), which a plain JSON reader accepts or a reader of bytes refuses: misuse" % why, None, pm)
    _b = cbytes(J(note_obj())); _pre = sha256(_b); _dg = G.eip191_digest(KN, _pre); _r, _s, _v = G.ecdsa_sign(N_ORDER - 1, _dg)
    pn1 = write("sign", "key_n_minus_1", _b)
    case("sign", ["0x%064x" % (N_ORDER - 1), pn1, KN], "n minus one signs", {"digest": hx(_dg), "presig": hx(_pre), "sig": G.sig_hex(_r, _s, _v), "signer": G.privkey_to_addr_hex(N_ORDER - 1)}, pn1)


# --------------------------------------------------------------------------
# pairing
# --------------------------------------------------------------------------

def build_pair(nb, pb, rb, fb, proto):
    def P(slug, n, r, note, pred):
        pn = write("pair", slug + "_note", n); pr = write("pair", slug + "_report", r)
        case("pair", [pn, pr], note, pred, pr)
    P("paired", nb, rb, "the report answers the note", {"verdict": "PAIRED"})
    P("note_invalid", nb[:-1], rb, "note bytes truncated", {"verdict": "NOTE_INVALID", "token": "E_JSON"})
    P("report_invalid", nb, REPORT(nb, proto, manifest=1), "report manifest malformed", {"verdict": "REPORT_INVALID", "token": "E_REPORT_MANIFEST"})
    P("both_invalid", b"{}", b"[]", "note decided first", {"verdict": "NOTE_INVALID", "token": "E_DOC_MISSING"})
    P("note_mismatch", NOTE(scope_md="another note"), rb, "report names a different note", {"verdict": "REPORT_NOTE_MISMATCH"})
    other = sign_doc(key("ver2"), report_obj(nb, proto, verifier=addr("ver2")), KR)
    P("verifier_mismatch", nb, other, "signed by a verifier the note did not name", {"verdict": "REPORT_VERIFIER_MISMATCH"})
    P("protocol_mismatch", nb, REPORT(nb, H32()), "another protocol", {"verdict": "REPORT_PROTOCOL_MISMATCH"})
    P("note_mismatch_before_verifier", NOTE(scope_md="x"), other, "noteHash decided before verifier", {"verdict": "REPORT_NOTE_MISMATCH"})
    P("verifier_before_protocol", nb, sign_doc(key("ver2"), report_obj(nb, H32(), verifier=addr("ver2")), KR), "verifier decided before protocol", {"verdict": "REPORT_VERIFIER_MISMATCH"})
    P("report_invalid_before_mismatch", nb, REPORT(NOTE(scope_md="z"), proto, manifest=1), "malformed and naming another note: invalid first", {"verdict": "REPORT_INVALID", "token": "E_REPORT_MANIFEST"})

    def R(slug, n, p, f, note, pred):
        pn = write("pair", "refusal_" + slug + "_note", n); pp = write("pair", "refusal_" + slug + "_pass", p); pf = write("pair", "refusal_" + slug + "_refusal", f)
        case("pair-refusal", [pn, pp, pf], note, pred, pf)
    R("paired", nb, pb, fb, "the record names its note and pass", {"verdict": "REFUSAL_PAIRED"})
    R("note_invalid", b"{", pb, fb, "note unreadable", {"verdict": "NOTE_INVALID", "token": "E_JSON"})
    R("pass_invalid", nb, PASS(nb, verifier=""), fb, "pass fails its order", {"verdict": "PASS_INVALID", "token": "E_PASS_VERIFIER"})
    R("refusal_invalid", nb, pb, REFUSAL(nb, pb, at="x"), "record fails its order", {"verdict": "REFUSAL_INVALID", "token": "E_REFUSAL_AT"})
    R("pass_then_refusal", nb, b"[]", b"[]", "pass decided before the record", {"verdict": "PASS_INVALID", "token": "E_DOC"})
    other_note = NOTE(scope_md="other")
    R("note_mismatch", other_note, pb, fb, "record names another note", {"verdict": "REFUSAL_NOTE_MISMATCH"})
    R("pass_mismatch", nb, PASS(nb, orderRef="SF-2"), fb, "record names another pass", {"verdict": "REFUSAL_PASS_MISMATCH"})
    fo = sign_doc(key("ver2"), refusal_obj(nb, pb, verifier=addr("ver2")), KF)
    R("verifier_mismatch", nb, pb, fo, "record signed by a verifier the note did not name", {"verdict": "REFUSAL_VERIFIER_MISMATCH"})
    R("note_before_pass_mismatch", other_note, PASS(nb, orderRef="SF-2"), fb, "noteHash decided before passHash", {"verdict": "REFUSAL_NOTE_MISMATCH"})
    R("pass_before_verifier_mismatch", nb, PASS(nb, orderRef="SF-2"), fo, "passHash decided before verifier", {"verdict": "REFUSAL_PASS_MISMATCH"})
    R("note_before_pass_invalid", b"{", b"[", fb, "note and pass both broken: note first", {"verdict": "NOTE_INVALID", "token": "E_JSON"})
    R("refusal_invalid_before_mismatch", nb, pb, REFUSAL(other_note, pb, at="x"), "malformed and naming another note: invalid first", {"verdict": "REFUSAL_INVALID", "token": "E_REFUSAL_AT"})
    unpaired_pass = PASS(other_note)
    R("pass_wraps_other_note_ok", nb, unpaired_pass, REFUSAL(nb, unpaired_pass), "the pairing does not ask whether the pass wraps the note", {"verdict": "REFUSAL_PAIRED"})


# --------------------------------------------------------------------------
# payload
# --------------------------------------------------------------------------
    h1, h2 = H32(), H32()
    lo32, hi32 = sorted([h1, h2])
    R("refusal_invalid_with_index", nb, pb, REFUSAL(nb, pb, photos=[lo32, hi32, hi32, lo32]), "a refusal whose token carries an index: the pairing carries it too", {"verdict": "REFUSAL_INVALID", "token": "E_REFUSAL_DUP_PHOTO", "index": 2})
    R("refusal_invalid_photo_index", nb, pb, REFUSAL(nb, pb, photos=[lo32, "0xzz"]), "the photo form token through the pairing, with its index", {"verdict": "REFUSAL_INVALID", "token": "E_REFUSAL_PHOTO", "index": 1})
    R("refusal_invalid_photo_order_index", nb, pb, REFUSAL(nb, pb, photos=[hi32, lo32]), "the photo order token through the pairing, with its index", {"verdict": "REFUSAL_INVALID", "token": "E_REFUSAL_PHOTO_ORDER", "index": 1})



def b64(b):
    return base64.urlsafe_b64encode(b).rstrip(b"=")


def build_qr(nb, pb):
    payload = PREFIX + b64(pb) + b"." + b64(nb)
    pp = write("qr", "encode_pass", pb); pn = write("qr", "encode_note", nb)
    case("qr-encode", [pp, pn], "encode a good pair", {"payload": payload.decode()}, pn)
    case("qr-encode", [pn, pp], "arguments swapped: the note is no pass", {"ok": False, "token": "E_PAYLOAD_PASS", "inner": "E_DOC_MISSING"}, pp)
    badp = write("qr", "encode_bad_pass", pb[:-3])
    case("qr-encode", [badp, pn], "pass bytes truncated", {"ok": False, "token": "E_PAYLOAD_PASS", "inner": "E_JSON"}, badp)
    badn = write("qr", "encode_bad_note", NOTE(issued=5, deadline=4))
    case("qr-encode", [pp, badn], "note fails its own order", {"ok": False, "token": "E_PAYLOAD_NOTE", "inner": "E_NOTE_WINDOW"}, badn)
    base_len = len(PREFIX) + len(b64(pb)) + 1

    def payload_len(k):
        return base_len + len(b64(NOTE(scope_md="x" * k)))
    lo, hi = 0, 4000
    while lo < hi:
        mid = (lo + hi + 1) // 2
        if payload_len(mid) <= CAP:
            lo = mid
        else:
            hi = mid - 1
    n_cap = NOTE(scope_md="x" * lo)
    at_cap = PREFIX + b64(pb) + b"." + b64(n_cap)
    assert len(at_cap) <= CAP < payload_len(lo + 1)
    pcap = write("qr", "encode_note_at_cap", n_cap)
    case("qr-encode", [pp, pcap], "payload of %d bytes, at or under the cap" % len(at_cap), {"payload": at_cap.decode()}, pcap)
    n_over = NOTE(scope_md="x" * (lo + 1))
    pover = write("qr", "encode_note_over_cap", n_over)
    over = PREFIX + b64(pb) + b"." + b64(n_over); assert len(over) > CAP
    case("qr-encode", [pp, pover], "payload of %d bytes" % len(over), {"ok": False, "token": "E_PAYLOAD_CAP"}, pover)
    # encode order collisions: a bad document beside an over-cap payload
    n_over_bad = NOTE(scope_md="x" * (lo + 1), issued=9, deadline=1)
    pob = write("qr", "encode_bad_note_over_cap", n_over_bad)
    case("qr-encode", [pp, pob], "note invalid and over the cap: the document decides first", {"ok": False, "token": "E_PAYLOAD_NOTE", "inner": "E_NOTE_WINDOW"}, pob)
    case("qr-encode", [badp, pover], "pass invalid and over the cap: the pass decides first", {"ok": False, "token": "E_PAYLOAD_PASS", "inner": "E_JSON"}, pover + "|badpass")

    def Q(slug, t, note, pred):
        p = write("qr", "decode_" + slug, t)
        case("qr-decode", [p], note, pred, p)
    Q("ok", payload, "a good payload", {"note": hx(nb), "ok": True, "pass": hx(pb)})
    Q("at_cap_ok", at_cap, "at the cap", {"note": hx(n_cap), "ok": True, "pass": hx(pb)})
    Q("over_cap", over, "over the cap", {"ok": False, "token": "E_PAYLOAD_CAP"})
    Q("prefix_case", b"Galeed-pass:" + payload[12:], "prefix case", {"ok": False, "token": "E_PAYLOAD_PREFIX"})
    Q("prefix_zikaron", b"zikaron-grant:" + payload[12:], "the twin's prefix", {"ok": False, "token": "E_PAYLOAD_PREFIX"})
    Q("prefix_missing_colon", b"galeed-pass" + payload[12:], "eleven bytes then no colon", {"ok": False, "token": "E_PAYLOAD_PREFIX"})
    Q("prefix_short_1", b"g", "one byte", {"ok": False, "token": "E_PAYLOAD_PREFIX"})
    Q("prefix_short_11", b"galeed-pass", "eleven bytes and nothing more", {"ok": False, "token": "E_PAYLOAD_PREFIX"})
    Q("prefix_nonascii", b"galeed-pass\xc2\xa0:" + payload[12:], "a non-ASCII byte at the colon", {"ok": False, "token": "E_PAYLOAD_PREFIX"})
    Q("empty", b"", "empty text", {"ok": False, "token": "E_PAYLOAD_PREFIX"})
    Q("prefix_only", PREFIX, "prefix and nothing: zero dots", {"ok": False, "token": "E_PAYLOAD_SEGMENTS", "count": 0})
    Q("prefix_before_cap", b"x" * (CAP + 10), "wrong prefix and over the cap: prefix first", {"ok": False, "token": "E_PAYLOAD_PREFIX"})
    Q("cap_before_segments", PREFIX + b"a" * CAP, "over the cap with zero dots: cap first", {"ok": False, "token": "E_PAYLOAD_CAP"})
    Q("two_dots", payload + b".", "trailing dot: two dots", {"ok": False, "token": "E_PAYLOAD_SEGMENTS", "count": 2})
    Q("three_dots", PREFIX + b"..." , "three dots", {"ok": False, "token": "E_PAYLOAD_SEGMENTS", "count": 3})
    Q("no_dot", PREFIX + b64(pb) + b64(nb), "segments joined without a dot", {"ok": False, "token": "E_PAYLOAD_SEGMENTS", "count": 0})
    Q("empty_first", PREFIX + b"." + b64(nb), "empty pass segment", {"ok": False, "token": "E_PAYLOAD_B64", "index": 0})
    Q("empty_second", PREFIX + b64(pb) + b".", "empty note segment", {"ok": False, "token": "E_PAYLOAD_B64", "index": 1})
    Q("bad_char", PREFIX + b64(pb) + b"." + b64(nb).replace(b"e", b"+", 1), "standard alphabet char", {"ok": False, "token": "E_PAYLOAD_B64", "index": 1})
    Q("nonascii_in_segment", PREFIX + b64(pb) + b"." + b64(nb)[:-1] + b"\xc3", "a byte above 0x7F inside a segment", {"ok": False, "token": "E_PAYLOAD_B64", "index": 1})
    Q("padding_char", PREFIX + b64(pb) + b"=." + b64(nb), "a padding character", {"ok": False, "token": "E_PAYLOAD_B64", "index": 0})
    # base64 residue classes: documents of byte length 0, 1, 2 mod 3 give segments of length 0, 2, 3 mod 4
    docs_by_res = {}
    k = 0
    while len(docs_by_res) < 3 and k < 6:
        n_k = NOTE(scope_md="r" * k)
        docs_by_res.setdefault(len(n_k) % 3, n_k)
        k += 1
    for res, n_k in sorted(docs_by_res.items()):
        seg = b64(n_k)
        assert len(seg) % 4 == {0: 0, 1: 2, 2: 3}[res]
        Q("residue_%d_ok" % res, PREFIX + b64(pb) + b"." + seg, "note segment of length %d mod 4 decodes" % (len(seg) % 4), {"note": hx(n_k), "ok": True, "pass": hx(pb)})
        if res != 0:
            # set an unused trailing bit: the last character's low bits must be zero
            last = seg[-1:]
            idx = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_".index(last)
            bump = 1 if res == 2 else 4   # two unused bits for res 2 (length 3 mod 4), four for res 1 (length 2 mod 4)
            alt = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"[idx ^ bump:idx ^ bump + 1]
            Q("residue_%d_trailing_bits" % res, PREFIX + b64(pb) + b"." + seg[:-1] + alt, "unused trailing bits set on a segment of length %d mod 4" % (len(seg) % 4), {"ok": False, "token": "E_PAYLOAD_B64", "index": 1})
    segn = b64(nb)
    while len(segn) % 4 != 1:
        segn += b"A"
    Q("len_mod4_1", PREFIX + b64(pb) + b"." + segn, "length 1 mod 4", {"ok": False, "token": "E_PAYLOAD_B64", "index": 1})
    segp0 = b64(pb)
    if len(segp0) % 4 != 0:
        idx0 = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_".index(segp0[-1:])
        bump0 = 1 if len(segp0) % 4 == 3 else 4
        alt0 = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"[idx0 ^ bump0:idx0 ^ bump0 + 1]
        Q("seg0_trailing_bits", PREFIX + segp0[:-1] + alt0 + b"." + b64(nb), "unused trailing bits set on segment 0", {"ok": False, "token": "E_PAYLOAD_B64", "index": 0})
    segp = b64(pb)
    while len(segp) % 4 != 1:
        segp += b"A"
    Q("len_mod4_1_first", PREFIX + segp + b"." + b64(nb), "length 1 mod 4 on segment 0", {"ok": False, "token": "E_PAYLOAD_B64", "index": 0})
    Q("first_seg_note", PREFIX + b64(nb) + b"." + b64(pb), "segments swapped", {"ok": False, "token": "E_PAYLOAD_PASS", "inner": "E_DOC_MISSING"})
    Q("pass_bad_sig", PREFIX + b64(sign_doc(key("judge"), pass_obj(nb), KP)) + b"." + b64(nb), "pass signed by the judge", {"ok": False, "token": "E_PAYLOAD_PASS", "inner": "E_SIG_SIGNER"})
    Q("pass_not_canonical", PREFIX + b64(sub(pb, b'{"kind"', b'{ "kind"')) + b"." + b64(nb), "a non-canonical pass inside a good segment", {"ok": False, "token": "E_PAYLOAD_PASS", "inner": "E_NOT_CANONICAL"})
    Q("note_bad", PREFIX + b64(pb) + b"." + b64(NOTE(verifier="0x")), "note fails its order", {"ok": False, "token": "E_PAYLOAD_NOTE", "inner": "E_NOTE_VERIFIER"})
    Q("b64_before_doc", PREFIX + b"!!!!" + b"." + b64(nb[:-1]), "segment 0 base64 decided before segment 1", {"ok": False, "token": "E_PAYLOAD_B64", "index": 0})
    Q("pass_doc_before_note_b64", PREFIX + b64(pb[:-1]) + b"." + b"!!!!", "segment 0's document decided before segment 1's base64", {"ok": False, "token": "E_PAYLOAD_PASS", "inner": "E_JSON"})
    other_note = NOTE(scope_md="unrelated")
    Q("unpaired_ok", PREFIX + b64(pb) + b"." + b64(other_note), "decodes; the door decides pairing", {"note": hx(other_note), "ok": True, "pass": hx(pb)})


# --------------------------------------------------------------------------
# door
# --------------------------------------------------------------------------

def ledger(judge_tag, extra_entries, dispatch_note=None, extra_type=None):
    """A chain: genesis, then a dispatch of `dispatch_note` (if given), then extras."""
    k = key(judge_tag)
    ents = []
    g = G.make_entry(k, "genesis", 0, None, {"statement_md": "a judge's ledger"})
    ents.append(g)
    prev, seq = eid(g), 1
    if dispatch_note is not None:
        body = {"noteHash": eid(dispatch_note)} if extra_type is None else extra_type
        d = G.make_entry(k, "galeed.dispatch/1", seq, prev, body)
        ents.append(d); prev, seq = eid(d), seq + 1
    for etype, body in extra_entries:
        e = G.make_entry(k, etype, seq, prev, body)
        ents.append(e); prev, seq = eid(e), seq + 1
    return ents


def anc(h, sender, bn=100, ts=1500, verdict="counted", tx=None):
    """A record carrying nothing."""
    return {"chainId": 1, "blockNumber": bn, "blockTimestamp": ts, "tx": tx or H32(0xA0000 + bn),
            "sender": sender, "hash": h, "verdict": verdict}


def carried(e, sender, **kw):
    """A record carrying the bytes e (galeed/1 s9.1, s9.2)."""
    return dict(anc(eid(e), sender, **kw), entry=hx(e))


def carry(ents, sender, bn0=1):
    return [carried(e, sender, bn=bn0 + i) for i, e in enumerate(ents)]


def basis(senders, registries=(REG,)):
    sd = sorted(set(senders), key=lambda a: a.encode())
    return {"chains": [{"chainId": 1, "fromBlock": 0, "toBlock": 1000, "registries": sorted(registries), "senders": sd}]}


def audit_input(root, ents, anchors, senders=None, registries=(REG,), bas=None, carrier=None):
    """The two inputs of galeed/1 s8: the entries travel as the bytes counted
    records of `carrier` (the root unless given) carry, at blocks 1 onward;
    `anchors` are further records."""
    return {"root": root, "anchors": carry(ents, carrier or root) + list(anchors),
            "basis": bas if bas is not None else basis(senders or [root], registries)}


def C_(state, token=None):
    return {"state": state, "token": token}


PASSC = C_("PASS")


def result(note, pas, clock, ledger_, anchor, bench, now, basis_, bench_echo, root=None, current=None):
    checks = {"anchor": anchor, "bench": bench, "clock": clock, "ledger": ledger_, "note": note, "pass": pas}
    states = [c["state"] for c in checks.values()]
    if "FAIL" in states:
        v = "FAIL"
    elif "UNKNOWN" in states or any(c["state"] == "SKIPPED" and c["token"] != "NO_BENCH" for c in checks.values()):
        v = "PARTIAL"
    else:
        v = "GREEN"
    out = {"basis": basis_, "bench": bench_echo, "checks": checks, "now": now, "root": root, "verdict": v}
    if current is not None:
        out["current"] = current
    return out


def build_door(nb, pb):
    J_ = addr("judge"); V = addr("ver")
    _root_of = {}
    wn = write("door", "note", nb); wp = write("door", "pass", pb)
    Lg = ledger("judge", [], nb)
    good = audit_input(J_, Lg, [])
    GB = good["basis"]
    _root_of[json.dumps(GB, sort_keys=True)] = J_
    wa = write("door", "audit_good", json.dumps(good).encode())
    bench_on = sorted([addr("x1"), V], key=lambda a: a.encode())
    wb = write("door", "bench_on", json.dumps(bench_on).encode())
    wboff = write("door", "bench_off", json.dumps([addr("x1")]).encode())
    wbempty = write("door", "bench_empty", b"[]")
    NOB = C_("SKIPPED", "NO_BENCH")

    def DOOR(slug, args, note, pred):
        if pred is not None and pred.get("basis") is not None and pred.get("root") is None:
            pred["root"] = _root_of.get(json.dumps(pred["basis"], sort_keys=True), J_)
        case("door", args, note, pred, args[0] + "|" + slug)

    DOOR("green", [wn, wp, "--now", "1250", "--audit", wa, "--bench", wb], "every check passes",
         result(PASSC, PASSC, PASSC, PASSC, PASSC, PASSC, 1250, GB, bench_on))
    DOOR("green_no_bench", [wn, wp, "--now", "1250", "--audit", wa], "bench skipped is outside the verdict",
         result(PASSC, PASSC, PASSC, PASSC, PASSC, NOB, 1250, GB, None))
    DOOR("bench_off", [wn, wp, "--now", "1250", "--audit", wa, "--bench", wboff], "verifier not on the bench",
         result(PASSC, PASSC, PASSC, PASSC, PASSC, C_("FAIL", "NOT_ON_BENCH"), 1250, GB, [addr("x1")]))
    DOOR("bench_empty", [wn, wp, "--now", "1250", "--audit", wa, "--bench", wbempty], "an empty bench is a resolved bench nobody sits on",
         result(PASSC, PASSC, PASSC, PASSC, PASSC, C_("FAIL", "NOT_ON_BENCH"), 1250, GB, []))
    DOOR("bench_unresolved", [wn, wp, "--now", "1250", "--audit", wa, "--bench-unresolved"], "the caller could not resolve the bench",
         result(PASSC, PASSC, PASSC, PASSC, PASSC, C_("UNKNOWN", "BENCH_UNRESOLVED"), 1250, GB, "unresolved"))
    DOOR("option_order", [wn, wp, "--bench", wb, "--audit", wa, "--now", "1250"], "options in another order",
         result(PASSC, PASSC, PASSC, PASSC, PASSC, PASSC, 1250, GB, bench_on))
    DOOR("offline", [wn, wp], "no clock, no ledger", result(PASSC, PASSC, C_("UNKNOWN", "NO_CLOCK"), C_("UNKNOWN", "NO_LEDGER"), C_("UNKNOWN", "LEDGER_UNREAD"), NOB, None, None, None))
    DOOR("clock_only", [wn, wp, "--now", "1250"], "clock passes, ledger unknown", result(PASSC, PASSC, PASSC, C_("UNKNOWN", "NO_LEDGER"), C_("UNKNOWN", "LEDGER_UNREAD"), NOB, 1250, None, None))
    DOOR("note_only", [wn, "--now", "1250", "--audit", wa, "--bench", wb], "no pass: the inbox reading; partial at best",
         result(PASSC, C_("SKIPPED", "NO_PASS"), C_("SKIPPED", "NO_PASS"), PASSC, PASSC, PASSC, 1250, GB, bench_on))
    DOOR("note_only_expired", [wn, "--now", "2001", "--audit", wa], "no pass and a clock past the deadline: the note's own fact is tested",
         result(PASSC, C_("SKIPPED", "NO_PASS"), C_("FAIL", "NOTE_EXPIRED"), PASSC, PASSC, NOB, 2001, GB, None))
    DOOR("note_only_bare", [wn], "a note alone with nothing else: no clock is decided before no pass", result(PASSC, C_("SKIPPED", "NO_PASS"), C_("UNKNOWN", "NO_CLOCK"), C_("UNKNOWN", "NO_LEDGER"), C_("UNKNOWN", "LEDGER_UNREAD"), NOB, None, None, None))
    # note failures
    wbadn = write("door", "note_bad_sig", sign_doc(key("ver"), note_obj(), KN))
    SK = C_("SKIPPED", "NO_NOTE")
    DOOR("bad_note", [wbadn, wp, "--now", "1250", "--audit", wa, "--bench", wb], "note signed by the wrong key: everything else skipped, the audit still echoed",
         result(C_("FAIL", ["BAD_NOTE", "E_SIG_SIGNER"]), SK, SK, SK, SK, SK, 1250, GB, bench_on))
    DOOR("swapped", [wp, wn, "--now", "1250"], "pass offered as note", result(C_("FAIL", ["BAD_NOTE", "E_DOC_MISSING"]), SK, SK, SK, SK, SK, 1250, None, None))
    DOOR("bad_note_window", [write("door", "note_window", NOTE(issued=3, deadline=2)), wp], "a token of this law's own carried beside BAD_NOTE",
         result(C_("FAIL", ["BAD_NOTE", "E_NOTE_WINDOW"]), SK, SK, SK, SK, SK, None, None, None))
    # pass failures
    wbadp = write("door", "pass_bad", pb[:-2])
    DOOR("bad_pass", [wn, wbadp, "--now", "1250", "--audit", wa, "--bench", wb], "pass bytes broken; clock skipped, ledger, anchor and bench decided",
         result(PASSC, C_("FAIL", ["BAD_PASS", "E_JSON"]), C_("SKIPPED", "NO_PASS"), PASSC, PASSC, PASSC, 1250, GB, bench_on))
    other_note = NOTE(scope_md="another")
    wpo = write("door", "pass_other_note", PASS(other_note))
    DOOR("pass_note_mismatch", [wn, wpo, "--now", "1250", "--audit", wa], "pass wraps another note",
         result(PASSC, C_("FAIL", "PASS_NOTE_MISMATCH"), C_("SKIPPED", "NO_PASS"), PASSC, PASSC, NOB, 1250, GB, None))
    wpv = write("door", "pass_other_verifier", sign_doc(key("ver2"), pass_obj(nb, verifier=addr("ver2")), KP))
    DOOR("pass_verifier_mismatch", [wn, wpv, "--now", "1250"], "a pass by a verifier the note did not name",
         result(PASSC, C_("FAIL", "PASS_VERIFIER_MISMATCH"), C_("SKIPPED", "NO_PASS"), C_("UNKNOWN", "NO_LEDGER"), C_("UNKNOWN", "LEDGER_UNREAD"), NOB, 1250, None, None))
    # clock boundaries (window 1200..1300, deadline 2000)
    for now, st, tok, why in [(1199, "FAIL", "NOT_YET", "one second early"), (1200, "PASS", None, "window start inclusive"),
                              (1300, "PASS", None, "window end inclusive"), (1301, "FAIL", "PASS_EXPIRED", "one second late"),
                              (0, "FAIL", "NOT_YET", "epoch"), ((1 << 53) - 1, "FAIL", "PASS_EXPIRED", "the largest now")]:
        DOOR("clock_%d" % now, [wn, wp, "--now", str(now), "--audit", wa], why, result(PASSC, PASSC, C_(st, tok), PASSC, PASSC, NOB, now, GB, None))
    wpl = write("door", "pass_beyond_deadline", PASS(nb, window={"from": 1900, "to": 2100}))
    for now, st, tok, why in [(2000, "PASS", None, "deadline inclusive"), (2001, "FAIL", "NOTE_EXPIRED", "past the note's deadline inside the pass window"),
                              (2200, "FAIL", "PASS_EXPIRED", "past both: the pass window decides first")]:
        DOOR("deadline_%d" % now, [wn, wpl, "--now", str(now), "--audit", wa], why, result(PASSC, PASSC, C_(st, tok), PASSC, PASSC, NOB, now, GB, None))

    # ledger and anchor outcomes
    def AUD(slug, inp, ledger_c, anchor_c, why, now="1250", bas=None):
        w = write("door", "audit_" + slug, inp if isinstance(inp, bytes) else json.dumps(inp).encode())
        b_ = None if isinstance(inp, bytes) else (inp.get("basis") if bas is None else bas)
        r_ = None if isinstance(inp, bytes) else inp.get("root")
        if ledger_c["state"] in ("UNKNOWN",) and ledger_c["token"] in ("NO_LEDGER", "LEDGER_INVALID"):
            b_ = None; r_ = None
        DOOR("anchor_" + slug, [wn, wp, "--now", now, "--audit", w], why, result(PASSC, PASSC, PASSC, ledger_c, anchor_c, NOB, int(now), b_, None, r_))
    LI = C_("UNKNOWN", "LEDGER_INVALID"); LU = C_("UNKNOWN", "LEDGER_UNREAD")
    ND_U = C_("UNKNOWN", "NOT_DISPATCHED"); ND_F = C_("FAIL", "NOT_DISPATCHED"); DD = C_("UNKNOWN", "DISPATCH_DISPUTED")
    AUD("invalid_json", b"{not json", LI, LU, "audit input unreadable: invalid, never misuse")
    AUD("invalid_bom", b"\xef\xbb\xbf" + json.dumps(good).encode(), LI, LU, "a byte order mark makes the file unreadable")
    AUD("invalid_root_array", b"[]", LI, LU, "root not an object")
    badbasis = dict(good); badbasis["basis"] = {"chains": [], "transactions": []}
    AUD("invalid_basis", badbasis, LI, LU, "a basis with a second member is no galeed/1 basis: no label")
    badentry = dict(good); badentry["anchors"] = [dict(good["anchors"][0], entry="0X" + good["anchors"][0]["entry"][2:])]
    AUD("invalid_entry_element", badentry, LI, LU, "an entry spelled 0X is outside the transport form: no label")
    unreach = dict(good); unreach["basis"] = basis([addr("x1")])
    AUD("invalid_record_unreached", unreach, LI, LU, "a record no chains object reaches (the judge is not among the senders): no label")
    AUD("not_in_lineage", audit_input(addr("ver"), ledger("ver", []), []), C_("UNKNOWN", "NOT_IN_LINEAGE"), LU, "audited under the verifier's own ledger, label COMPLETE: the basis never named the judge's key, so unknown")
    AUD("not_in_lineage_gaps", audit_input(addr("ver"), ledger("ver", []), [anc(H32(), addr("ver"))]), C_("UNKNOWN", "NOT_IN_LINEAGE"), LU, "a stranger's ledger short of COMPLETE: unknown all the same")
    L = ledger("judge", [], nb)
    AUD("dispatch_carried", audit_input(J_, L, []), PASSC, PASSC, "the dispatch is carried by a counted record of the judge: a ledger entry, PASS")
    AUD("dispatch_carried_not_covering", audit_input(J_, L, [], registries=()), PASSC, PASSC, "a dispatch in hand passes whatever the basis covers")
    AUD("dispatch_hash_only_gaps", audit_input(J_, L[:1], [anc(eid(L[1]), J_, bn=50)]), PASSC, ND_U, "the dispatch's record carries nothing: the hash is MISSING, the entry is no ledger entry, label GAPS, so unknown")
    AUD("not_dispatched_complete", audit_input(J_, ledger("judge", []), []), PASSC, ND_F, "no dispatch entry, covering basis, label COMPLETE")
    AUD("not_dispatched_not_covering", audit_input(J_, ledger("judge", []), [], registries=()), PASSC, ND_U, "no dispatch entry and a basis that looked nowhere")
    AUD("not_dispatched_gaps", audit_input(J_, ledger("judge", []), [anc(H32(), J_)]), PASSC, ND_U, "no dispatch entry, label GAPS")
    nc = {"root": J_, "anchors": [], "basis": {"chains": []}}
    AUD("empty_ledger_empty_chains", nc, PASSC, ND_U, "a basis with no chains and no record: the ledger is empty, a reader holding nothing convicts nobody")
    AUD("dispatch_carried_unproven", audit_input(J_, L[:1], [carried(L[1], J_, bn=50, verdict="UNPROVEN")]), PASSC, ND_U, "the dispatch is carried only by an UNPROVEN record: no ledger entry, label UNPROVEN, so unknown")
    AUD("dispatch_carried_void", audit_input(J_, L[:1], [carried(L[1], J_, bn=50, verdict="VOID")]), PASSC, ND_F, "the dispatch is carried only by a VOID record: no ledger entry, the ledger reads COMPLETE without it, covering, FAIL")
    AUD("dispatch_carried_by_stranger", audit_input(J_, L[:1], [carried(L[1], addr("x1"), bn=50)], senders=[J_, addr("x1")]), PASSC, ND_F, "a stranger's record carries the dispatch: the record is discarded, the entry is no ledger entry, COMPLETE, FAIL")
    AUD("dispatch_carried_void_and_counted", audit_input(J_, L, [carried(L[1], J_, bn=50, verdict="VOID")]), PASSC, PASSC, "the dispatch carried by a VOID record and by a counted one: a ledger entry through the second, PASS")
    # succession
    succ = key("succ"); S = addr("succ")
    Ls = ledger("judge", [("succession", {"to": S, "kind": "keyrotation", "effective": 1700000000, "statement_md": "rotated"})])
    d2 = G.make_entry(succ, "galeed.dispatch/1", 2, eid(Ls[1]), {"noteHash": eid(nb)})
    AUD("dispatch_by_successor", audit_input(J_, Ls, [carried(d2, S, bn=50)], senders=[J_, S]), PASSC, PASSC, "the successor's dispatch carried by the successor's own record counts")
    AUD("dispatch_by_successor_carried_by_root", audit_input(J_, Ls + [d2], [], senders=[J_, S]), PASSC, PASSC, "the root that handed off carries the successor's dispatch at its retirement's own second: every lineage key can carry any entry")
    AUD("not_dispatched_senders_short", audit_input(J_, Ls, [], senders=[J_]), PASSC, ND_U, "no dispatch, and the basis names no successor sender: not covering, unknown")
    AUD("not_dispatched_senders_full", audit_input(J_, Ls, [], senders=[J_, S]), PASSC, ND_F, "no dispatch under a basis naming both keys: covering, COMPLETE, FAIL")
    ns = sign_doc(succ, note_obj(judge=S), KN)
    wns = write("door", "note_by_successor", ns)
    ps = PASS(ns)
    wps = write("door", "pass_for_successor_note", ps)
    d2s = G.make_entry(succ, "galeed.dispatch/1", 2, eid(Ls[1]), {"noteHash": eid(ns)})
    ws = write("door", "audit_successor_note", json.dumps(audit_input(J_, Ls, [carried(d2s, S, bn=50)], senders=[J_, S])).encode())
    case("door", [wns, wps, "--now", "1250", "--audit", ws], "a note signed by the successor key, audited under the original root: in the lineage",
         result(PASSC, PASSC, PASSC, PASSC, PASSC, NOB, 1250, basis([J_, S]), None, J_), wns + "|successor_note_original_root")
    ws2 = write("door", "audit_successor_note_rooted_at_successor", json.dumps(audit_input(S, [], [carried(d2s, S, bn=50)], senders=[S])).encode())
    case("door", [wns, wps, "--now", "1250", "--audit", ws2], "audited under the successor as root: a fragment with a gap, the dispatch carried: PASS under GAPS",
         result(PASSC, PASSC, PASSC, PASSC, PASSC, NOB, 1250, basis([S]), None, S), wns + "|successor_note_successor_root")
    # strangers and wrong forms
    Lf = ledger("judge", [])
    df = G.make_entry(key("x1"), "galeed.dispatch/1", 1, eid(Lf[0]), {"noteHash": eid(nb)}, judge=addr("x1"))
    AUD("dispatch_by_stranger", audit_input(J_, Lf, [carried(df, addr("x1"), bn=50)], senders=[J_, addr("x1")]), PASSC, ND_F, "a stranger's entry carried by the stranger is no ledger entry; COMPLETE, FAIL")
    AUD("dispatch_by_stranger_carried_by_judge", audit_input(J_, Lf + [df], []), C_("FAIL", "BROKEN_LEDGER"), LU, "a stranger's entry carried by the judge's own counted record is CARRIED_MALFORMED: BROKEN_CHAIN")
    AUD("carried_succession_unproven_dispatch_by_successor", audit_input(J_, [Ls[0]], [carried(Ls[1], J_, bn=50, verdict="UNPROVEN"), carried(d2, S, bn=51)], senders=[J_, S]), PASSC, DD, "the succession is carried only by an UNPROVEN record: the successor joins the lineage, the succession is no ledger entry, so its position is a gap and the successor's dispatch is disputed; label UNPROVEN")
    wsc = write("door", "audit_successor_note_carried_succession", json.dumps(audit_input(J_, [Ls[0]], [carried(Ls[1], J_, bn=50), carried(d2s, S, bn=51)], senders=[J_, S])).encode())
    case("door", [wns, wps, "--now", "1250", "--audit", wsc], "the succession reaches the audit as a carried entry at a later block than the genesis: the successor's note is in the lineage and its dispatch clean",
         result(PASSC, PASSC, PASSC, PASSC, PASSC, NOB, 1250, basis([J_, S]), None, J_), wns + "|successor_note_carried_succession")
    wsx = write("door", "audit_successor_note_succession_carried_by_stranger", json.dumps(audit_input(J_, [Ls[0]], [carried(Ls[1], addr("x1"), bn=50), carried(d2s, S, bn=51)], senders=[J_, S, addr("x1")])).encode())
    case("door", [wns, wps, "--now", "1250", "--audit", wsx], "the succession is carried by a stranger's record: the record is discarded, the successor is in the lineage by signatures, the succession is no ledger entry, so the position is a gap and the successor's dispatch is disputed",
         result(PASSC, PASSC, PASSC, PASSC, DD, NOB, 1250, basis([J_, S, addr("x1")]), None, J_), wns + "|successor_note_succession_carried_by_stranger")

    Lw = ledger("judge", [], nb, extra_type={"noteHash": "0x" + eid(nb)[2:].upper()})
    AUD("dispatch_notehash_upper", audit_input(J_, Lw, []), PASSC, ND_F, "noteHash not byte-equal")
    Lo = ledger("judge", [], nb, extra_type={"note": eid(nb)})
    AUD("dispatch_wrong_member", audit_input(J_, Lo, []), PASSC, ND_F, "body carries no noteHash")
    Lx = ledger("judge", [], nb, extra_type={"noteHash": eid(nb), "scope": "extra"})
    AUD("dispatch_extra_member_ok", audit_input(J_, Lx, []), PASSC, PASSC, "an extra body member is data")
    Lt = ledger("judge", [("galeed.dispatch/2", {"noteHash": eid(nb)})])
    AUD("dispatch_v2_type", audit_input(J_, Lt, []), PASSC, ND_F, "a later dispatch type is not this law's")
    Lm = ledger("judge", [("galeed.dispatch/1", {"noteHash": eid(nb)})], nb)
    AUD("two_dispatches_carried", audit_input(J_, Lm, []), PASSC, PASSC, "two dispatches of one note; either serves")
    # findings on the dispatch
    g0 = ledger("judge", [])[0]
    dgap = G.make_entry(key("judge"), "galeed.dispatch/1", 2, eid(g0), {"noteHash": eid(nb)})
    AUD("dispatch_after_gap", audit_input(J_, [g0, dgap], []), PASSC, PASSC, "a dispatch across a gap: a SEQ_GAP says nothing about the entry, PASS under GAPS")
    Lr = ledger("judge", [("succession", {"to": S, "kind": "handover", "effective": 1700000000, "statement_md": "handed over"})])
    dret = G.make_entry(key("judge"), "galeed.dispatch/1", 3, H32(), {"noteHash": eid(nb)})
    AUD("retired_key_after_gap", audit_input(J_, Lr + [dret], [], senders=[J_, S]), PASSC, C_("FAIL", "NOT_DISPATCHED"), "galeed/1 s7.4: the dispatch of the key that handed off, at a position after its handing off and carried in that same second, rides a record at which the key has retired: DISCARDED, no ledger entry; COMPLETE, so the absent dispatch convicts")
    # broken chain and malformed bytes
    Le = ledger("judge", [], nb)
    twin = G.make_entry(key("judge"), "annotation", 1, eid(Le[0]), {"note_md": "twin"})
    AUD("broken_chain", audit_input(J_, Le + [twin], []), C_("FAIL", "BROKEN_LEDGER"), LU, "two entries at seq 1: BROKEN_CHAIN, even with the dispatch carried")
    twin_d = G.make_entry(key("judge"), "galeed.dispatch/1", 1, eid(Le[0]), {"noteHash": eid(nb), "x": 1})
    AUD("two_dispatches_one_seq", audit_input(J_, Le + [twin_d], []), C_("FAIL", "BROKEN_LEDGER"), LU, "two dispatches of one note at one seq are an equivocation")
    mal = Le[1][:-4]
    AUD("malformed_dispatch_carried_by_judge", audit_input(J_, [Le[0]], [carried(mal, J_, bn=50)]), C_("FAIL", "BROKEN_LEDGER"), LU, "the judge's counted record carries malformed bytes: CARRIED_MALFORMED, hard")
    AUD("malformed_dispatch_carried_by_stranger", audit_input(J_, [Le[0]], [carried(mal, addr("x1"), bn=50)], senders=[J_, addr("x1")]), PASSC, ND_F, "a stranger carries malformed bytes: MALFORMED and DISCARDED, no finding, COMPLETE, no dispatch")
    # round two: empty record, block range, two chains, a key that signed nothing, disputed beside clean
    AUD("empty_anchor_set", audit_input(J_, [], []), PASSC, ND_U, "a reader holding nothing convicts nobody")
    fb9 = audit_input(J_, L, [], carrier=J_); fb9["anchors"] = carry(L, J_, bn0=950); fb9["basis"]["chains"][0]["fromBlock"] = 900
    AUD("dispatch_from_block_900", fb9, PASSC, PASSC, "a scan from block 900 that found the dispatch: PASS, coverage unread")
    nd9 = audit_input(J_, ledger("judge", []), []); nd9["anchors"] = carry(ledger("judge", []), J_, bn0=950); nd9["basis"]["chains"][0]["fromBlock"] = 900
    AUD("not_dispatched_from_block_900", nd9, PASSC, ND_U, "a scan from block 900 did not look where the dispatch would be: not covering, unknown")
    nd1 = audit_input(J_, ledger("judge", []), []); nd1["basis"]["chains"][0]["fromBlock"] = 1
    AUD("not_dispatched_from_block_1", nd1, PASSC, ND_U, "fromBlock 1 is not covering")
    two_ok = audit_input(J_, ledger("judge", []), []); two_ok["basis"]["chains"].append({"chainId": 2, "fromBlock": 0, "toBlock": 50, "registries": [REG], "senders": [J_]})
    AUD("two_chains_both_covering", two_ok, PASSC, ND_F, "two chains objects, both covering, no dispatch: FAIL under COMPLETE")
    tw = audit_input(J_, ledger("judge", []), []); tw["basis"]["chains"] = [{"chainId": 1, "fromBlock": 0, "toBlock": 500, "registries": [REG], "senders": [J_]}, {"chainId": 1, "fromBlock": 501, "toBlock": 1000, "registries": [REG, "0x" + "22" * 20], "senders": [J_]}]
    AUD("one_chain_two_windows_covering", tw, PASSC, ND_F, "one chain in two contiguous windows from block 0: covering, FAIL under COMPLETE")
    twh = audit_input(J_, ledger("judge", []), []); twh["basis"]["chains"] = [{"chainId": 1, "fromBlock": 0, "toBlock": 500, "registries": [REG], "senders": [J_]}, {"chainId": 1, "fromBlock": 600, "toBlock": 1000, "registries": [REG], "senders": [J_]}]
    AUD("one_chain_two_windows_hole", twh, PASSC, ND_U, "a hole between the windows: not covering")
    twn = audit_input(J_, ledger("judge", []), []); twn["anchors"] = carry(ledger("judge", []), J_, bn0=150); twn["basis"]["chains"] = [{"chainId": 1, "fromBlock": 100, "toBlock": 500, "registries": [REG], "senders": [J_]}, {"chainId": 1, "fromBlock": 501, "toBlock": 1000, "registries": [REG, "0x" + "22" * 20], "senders": [J_]}]
    AUD("one_chain_two_windows_late_start", twn, PASSC, ND_U, "windows contiguous but starting at 100: not covering")
    tb = audit_input(J_, ledger("judge", []), []); tb["basis"]["chains"][0]["toBlock"] = 50
    AUD("to_block_below_the_anchor", tb, PASSC, ND_F, "toBlock 50 with no dispatch in the input: covering by the rule, FAIL under COMPLETE; the reach is the basis's")
    two_second_from = audit_input(J_, ledger("judge", []), []); two_second_from["basis"]["chains"].append({"chainId": 2, "fromBlock": 7, "toBlock": 50, "registries": [REG], "senders": [J_]})
    AUD("two_chains_second_from_block_7", two_second_from, PASSC, ND_U, "fromBlock read on the second object: not covering")
    two_second_senders = audit_input(J_, Ls, [], senders=[J_, S]); two_second_senders["basis"]["chains"].append({"chainId": 2, "fromBlock": 0, "toBlock": 50, "registries": [REG], "senders": [J_]})
    AUD("two_chains_second_senders_short", two_second_senders, PASSC, ND_U, "senders read on the second object: the successor is missing there")
    two_union = audit_input(J_, Ls, [], senders=[J_]); two_union["basis"]["chains"].append({"chainId": 2, "fromBlock": 0, "toBlock": 50, "registries": [REG], "senders": [S]})
    AUD("two_chains_senders_union_not_enough", two_union, PASSC, ND_U, "each object names one lineage key; the union names both and the rule asks each object")
    stranger_unproven = audit_input(J_, L[:1], [carried(L[1], addr("x1"), bn=50, verdict="UNPROVEN")], senders=[J_, addr("x1")])
    AUD("unproven_by_stranger_discarded", stranger_unproven, PASSC, ND_F, "an UNPROVEN record from a stranger is discarded by the trim of galeed/1 s8.1: no label moves, no dispatch, FAIL under COMPLETE")
    AUD("discarded_beside_green", audit_input(J_, L, [anc(H32(), addr("x1"), bn=101)], senders=[J_, addr("x1")]), PASSC, PASSC, "a stranger's anchor is discarded beside a carried dispatch: GREEN")
    Lprefix = ledger("judge", [("succession", {"to": S, "kind": "keyrotation", "effective": 1700000000, "statement_md": "rotated"})])
    d2p = G.make_entry(succ, "galeed.dispatch/1", 2, eid(Lprefix[1]), {"noteHash": eid(ns)})
    wpre = write("door", "audit_prefix_truncation", json.dumps(audit_input(J_, [Lprefix[0]], [], senders=[J_, S])).encode())
    case("door", [wns, wps, "--now", "1250", "--audit", wpre], "the scan found the genesis alone: COMPLETE, and the successor's note reads unknown",
         result(PASSC, PASSC, PASSC, C_("UNKNOWN", "NOT_IN_LINEAGE"), LU, NOB, 1250, basis([J_, S]), None, J_), wns + "|prefix_truncation")
    whole = write("door", "audit_succession_withheld", json.dumps(audit_input(J_, [Lprefix[0]], [carried(d2p, S, bn=50)], senders=[J_, S])).encode())
    case("door", [wns, wps, "--now", "1250", "--audit", whole], "the succession withheld: the successor's record is discarded and its dispatch no ledger entry, label COMPLETE, unknown",
         result(PASSC, PASSC, PASSC, C_("UNKNOWN", "NOT_IN_LINEAGE"), LU, NOB, 1250, basis([J_, S]), None, J_), wns + "|succession_withheld")
    stranger_all = audit_input(J_, ledger("ver", []), [], senders=[J_, addr("ver")], carrier=addr("ver"))
    AUD("empty_ledger_under_stranger_records", stranger_all, PASSC, ND_U, "a stranger's whole ledger carried by the stranger under this judge's root: every record discarded, every entry excluded, the ledger empty")
    Lthree = ledger("judge", [("succession", {"to": S, "kind": "keyrotation", "effective": 1700000000, "statement_md": "one"})])
    s2 = G.make_entry(succ, "succession", 2, eid(Lthree[1]), {"to": addr("succ2"), "kind": "keyrotation", "effective": 1700000001, "statement_md": "two"})
    d3 = G.make_entry(key("succ2"), "galeed.dispatch/1", 3, eid(s2), {"noteHash": eid(nb)})
    AUD("lineage_of_three", audit_input(J_, Lthree + [s2], [carried(d3, addr("succ2"), bn=50)], senders=[J_, S, addr("succ2")]), PASSC, PASSC, "two successions: a lineage of three keys, the third carries the dispatch")
    AUD("lineage_of_three_not_dispatched_senders_short", audit_input(J_, Lthree + [s2], [], senders=[J_, S]), PASSC, ND_U, "no dispatch and the third key missing from senders: not covering")
    rev3 = {"root": J_, "anchors": [carried(d3, addr("succ2"), bn=52), carried(s2, S, bn=51), carried(Lthree[1], J_, bn=50), carried(Lthree[0], J_, bn=49)], "basis": basis([J_, S, addr("succ2")])}
    AUD("two_successions_reverse_order", rev3, PASSC, PASSC, "s9.0: two successions listed after the dispatch and before the genesis; the lineage is a fixpoint over the records, so a single pass in the listed order admits nobody while the fixpoint admits the third key: GREEN")
    Lmany = ledger("judge", [("galeed.dispatch/1", {"noteHash": eid(nb)}), ("galeed.dispatch/1", {"noteHash": eid(nb)})], nb)
    AUD("three_dispatches_carried", audit_input(J_, Lmany, []), PASSC, PASSC, "three dispatches of one note: any one serves")
    for slug, body in [("int", {"noteHash": 5}), ("null", {"noteHash": None}), ("array", {"noteHash": [eid(nb)]}), ("nested", {"x": {"noteHash": eid(nb)}})]:
        Lb = ledger("judge", [], nb, extra_type=body)
        AUD("dispatch_notehash_" + slug, audit_input(J_, Lb, []), PASSC, ND_F, "noteHash present as %s: no dispatch" % slug)
    two_bad = audit_input(J_, ledger("judge", []), []); two_bad["basis"]["chains"].append({"chainId": 2, "fromBlock": 0, "toBlock": 50, "registries": [], "senders": [J_]})
    AUD("two_chains_one_not_covering", two_bad, PASSC, ND_U, "every chains object must cover: the second has no registries")
    Lsn = ledger("judge", [("succession", {"to": S, "kind": "keyrotation", "effective": 1700000000, "statement_md": "rotated"})])
    AUD("lineage_key_signed_nothing_senders_short", audit_input(J_, Lsn, [], senders=[J_]), PASSC, ND_U, "the successor signed nothing and is in the lineage all the same; senders lack it: not covering")
    AUD("lineage_key_signed_nothing_senders_full", audit_input(J_, Lsn, [], senders=[J_, S]), PASSC, ND_F, "senders name the silent successor: covering, COMPLETE, no dispatch")
    dclean = G.make_entry(key("judge"), "galeed.dispatch/1", 1, eid(Lr[0]), {"noteHash": eid(nb)})
    Lr2 = [Lr[0], dclean, G.make_entry(key("judge"), "succession", 2, eid(dclean), {"to": S, "kind": "handover", "effective": 1700000000, "statement_md": "handed over"})]
    dret2 = G.make_entry(key("judge"), "galeed.dispatch/1", 4, H32(), {"noteHash": eid(nb)})
    AUD("disputed_beside_clean", audit_input(J_, Lr2, [carried(dret2, S, bn=40)], senders=[J_, S]), PASSC, PASSC, "s10.2 check 5: two dispatches in the ledger, the ceding key's at 4 re-carried by the successor in the second of the handing off (galeed/1 s7.4, s8.1: the successor has not retired, so its record stands and the bytes are a ledger entry) and disputed by a soft AUTHORITY_MISMATCH after the gap, and the clean one at 1: the clean one serves")
    AUD("disputed_alone", audit_input(J_, Lr2[:1] + [Lr2[2]] + [dret2], [], senders=[J_, S]), PASSC, C_("UNKNOWN", "NOT_DISPATCHED"), "seq 1 missing so the ledger reads GAPS; the dispatch at 4 of the key that handed off, carried in the second of its handing off, is DISCARDED, and under GAPS the absent dispatch is unknown")
    AUD("disputed_alone_successor_carrier", audit_input(J_, Lr2[:1] + [Lr2[2]], [carried(dret2, S, bn=40)], senders=[J_, S]), PASSC, C_("UNKNOWN", "DISPATCH_DISPUTED"), "galeed/1 s7.4, s8.1: the successor has not retired, so its record of the handing off's second makes the ceding key's dispatch at 4 a ledger entry; seq 1 missing, the walk is uncertain and the AUTHORITY_MISMATCH soft: every dispatch disputed")
    AUD("retired_key_late_anchor_of_held_position", audit_input(J_, [Lr2[0], Lr2[2]], [carried(dclean, J_, bn=9)], senders=[J_, S]), PASSC, PASSC, "s12: the key that handed off anchors, at its retirement's own second, its dispatch at the position it held (seq 1, before its hand-off at 2): it has not retired at that record, so the entry is graded clean, COMPLETE, GREEN")
    dconf = G.make_entry(key("judge"), "galeed.dispatch/1", 1, eid(Lr2[0]), {"noteHash": H32()})
    AUD("retired_key_late_conflicting_entry_at_held_position", audit_input(J_, Lr2, [carried(dconf, J_, bn=9)], senders=[J_, S]), C_("FAIL", "BROKEN_LEDGER"), LU, "s12: at its retirement's own second, the key that handed off anchors a second, different dispatch at the position it held: two ledger entries at seq 1, an EQUIVOCATION, hard; BROKEN_CHAIN, the ledger unread")
    L3 = ledger("judge", [("annotation", {"note_md": "two"}), ("annotation", {"note_md": "three"})], nb)
    AUD("later_entry_missing", audit_input(J_, [L3[0], L3[1], L3[3]], []), PASSC, PASSC, "an entry after the dispatch is carried by no record: GAPS, the dispatch itself a ledger entry, PASS")
    wsb = write("door", "audit_broken_stranger_note", json.dumps(audit_input(J_, Le + [twin], [])).encode())
    case("door", [wns, wps, "--now", "1250", "--audit", wsb], "a forked ledger read against a note its lineage never held: BROKEN_LEDGER decided first",
         result(PASSC, PASSC, PASSC, C_("FAIL", "BROKEN_LEDGER"), LU, NOB, 1250, basis([J_]), None, J_), wns + "|broken_before_lineage")
    # check 2 and 3 adjacencies
    wboth = write("door", "pass_both_mismatch", sign_doc(key("ver2"), pass_obj(other_note, verifier=addr("ver2")), KP))
    DOOR("pass_both_mismatch", [wn, wboth, "--now", "1250"], "another note and another verifier: noteHash decided first",
         result(PASSC, C_("FAIL", "PASS_NOTE_MISMATCH"), C_("SKIPPED", "NO_PASS"), C_("UNKNOWN", "NO_LEDGER"), C_("UNKNOWN", "LEDGER_UNREAD"), NOB, 1250, None, None))
    wbadsig_other = write("door", "pass_badsig_other_note", sign_doc(key("judge"), pass_obj(other_note), KP))
    DOOR("bad_pass_before_note_mismatch", [wn, wbadsig_other, "--now", "1250"], "a pass signed by the wrong key that also names another note: BAD_PASS decided first",
         result(PASSC, C_("FAIL", ["BAD_PASS", "E_SIG_SIGNER"]), C_("SKIPPED", "NO_PASS"), C_("UNKNOWN", "NO_LEDGER"), C_("UNKNOWN", "LEDGER_UNREAD"), NOB, 1250, None, None))
    DOOR("bad_pass_past_deadline", [wn, wbadp, "--now", "2001", "--audit", wa], "a broken pass and a clock past the deadline: check 3 tests the note's deadline",
         result(PASSC, C_("FAIL", ["BAD_PASS", "E_JSON"]), C_("FAIL", "NOTE_EXPIRED"), PASSC, PASSC, NOB, 2001, GB, None))
    DOOR("note_only_at_deadline", [wn, "--now", "2000", "--audit", wa], "no pass, now at the deadline exactly: inclusive",
         result(PASSC, C_("SKIPPED", "NO_PASS"), C_("SKIPPED", "NO_PASS"), PASSC, PASSC, NOB, 2000, GB, None))
    wbench_ver2 = write("door", "bench_ver2_only", json.dumps([addr("ver2")]).encode())
    DOOR("bench_reads_the_note_verifier", [wn, wpv, "--now", "1250", "--audit", wa, "--bench", wbench_ver2], "the bench holds the pass's verifier and not the note's: NOT_ON_BENCH",
         result(PASSC, C_("FAIL", "PASS_VERIFIER_MISMATCH"), C_("SKIPPED", "NO_PASS"), PASSC, PASSC, C_("FAIL", "NOT_ON_BENCH"), 1250, GB, [addr("ver2")]))
    wlate = write("door", "pass_window_after_deadline", PASS(nb, window={"from": 2500, "to": 2600}))
    DOOR("not_yet_before_note_expired", [wn, wlate, "--now", "2400", "--audit", wa], "a window that opens after the deadline: NOT_YET decided before NOTE_EXPIRED",
         result(PASSC, PASSC, C_("FAIL", "NOT_YET"), PASSC, PASSC, NOB, 2400, GB, None))
    # misuse
    wbad = write("door", "bench_bad", b'["0x1"]')
    wbobj = write("door", "bench_obj", b'{"a":1}')
    case("door", [wn, wp, "--bench", wbad], "bench element outside hex20: misuse", None, wn + "|bench_misuse")
    case("door", [wn, wp, "--bench", wbobj], "bench root an object: misuse", None, wn + "|bench_obj_misuse")
    wbun = write("door", "bench_unreadable", b"{not json")
    case("door", [wn, wp, "--bench", wbun], "a bench file the reader refuses: misuse", None, wn + "|bench_unreadable")
    wbus = write("door", "bench_unsorted", json.dumps(sorted(bench_on, key=lambda a: a.encode(), reverse=True)).encode())
    case("door", [wn, wp, "--bench", wbus], "a bench out of bytewise order: misuse", None, wn + "|bench_unsorted")
    wbdu = write("door", "bench_dup", json.dumps([V, V]).encode())
    case("door", [wn, wp, "--bench", wbdu], "a bench with a repeat: misuse", None, wn + "|bench_dup")
    case("door", [wn, wp, "--bench", "door/no_such_bench.json"], "a missing bench path: misuse", None, wn + "|bench_missing_path", absent=["door/no_such_bench.json"])
    case("door", [wn, wp, "--foo", "1"], "an unknown option: misuse", None, wn + "|unknown_option")
    case("note-check", ["docs/no_such_note.json"], "an absent positional path: misuse", None, wn + "|note_missing_path", absent=["docs/no_such_note.json"])
    MANIFEST.append({"argv": [], "path": wn + "|no_command", "note": "gpp with no command at all: misuse", "predicted": None})
    case("door", [wbadn, wp, "--bench", wbad], "a failing note does not excuse a bad bench file: files are read first", None, wn + "|bench_misuse_bad_note")
    case("door", [wbadn, wp, "--audit", "door/no_such_file.json"], "an unreadable audit path is misuse even when the note fails", None, wn + "|audit_missing_bad_note", absent=["door/no_such_file.json"])
    case("door", [wn, wp, "--now", "01"], "leading zero: misuse", None, wn + "|now_misuse")
    case("door", [wn, wp, "--now", "+1"], "a sign: misuse", None, wn + "|now_sign")
    case("door", [wn, wp, "--now", "9007199254740992"], "2^53: misuse", None, wn + "|now_too_big")
    case("door", [wn, wp, "--now", "1250", "--now", "1250"], "repeated option: misuse", None, wn + "|dup_option")
    case("door", [wn, wp, "--now=1250"], "joined option: misuse", None, wn + "|joined_option")
    case("door", [wn, wp, "--now"], "option without value: misuse", None, wn + "|dangling_option")
    case("door", [wn, wp, "--bench", wb, "--bench-unresolved"], "both bench options: misuse", None, wn + "|both_bench")
    case("door", [wn, wp, "--bench-unresolved", "--bench-unresolved"], "unresolved twice: misuse", None, wn + "|unresolved_twice")
    case("door", [wn, "--now", "1250", wp], "an option before a positional: the pass path is a bad argument", None, wn + "|option_before_positional")
    case("door", [wn, wp, "--bench-unresolved", wb], "a token after --bench-unresolved: beyond the list", None, wn + "|unresolved_with_value")
    case("door", [wn, wp, "--audit", wa, "--audit", wa], "--audit twice: misuse", None, wn + "|audit_twice")
    case("door", [wn, wp, "--bench", wb, "--bench", wb], "--bench twice: misuse", None, wn + "|bench_twice")
    case("pass-check", [wp, wn], "two arguments to pass-check: misuse", None, wn + "|pass_two_args")
    case("qr-decode", [wn, wp], "two arguments to qr-decode: misuse", None, wn + "|decode_two_args")
    case("pair", [wn, wn, wn], "three arguments to pair: misuse", None, wn + "|pair_three_args")
    # argument-count misuse, one per command, and a duplicated record
    case("note-check", [wn, wp], "two arguments: misuse", None, wn + "|two_args")
    case("pass-check", [], "no argument: misuse", None, wn + "|pass_no_arg")
    case("pair", [wn], "one argument: misuse", None, wn + "|pair_one_arg")
    case("pair-refusal", [wn, wp], "two arguments: misuse", None, wn + "|pair_refusal_two_args")
    case("qr-encode", [wp], "one argument: misuse", None, wn + "|encode_one_arg")
    case("qr-decode", [], "no argument: misuse", None, wn + "|decode_no_arg")
    case("door", [], "no argument: misuse", None, wn + "|door_no_arg")
    case("sign", ["0x%064x" % key("judge"), wn], "two arguments: misuse", None, wn + "|sign_two_args")
    case("sign", ["abc", wn, KN], "a key of three digits: misuse", None, wn + "|sign_short_key")
    _b = open(os.path.join(HERE, wn), "rb").read(); _o = J(json.loads(_b)); _pre = sha256(cbytes(_o)); _dg = G.eip191_digest(KN, _pre); _r, _s, _v = G.ecdsa_sign(key("judge"), _dg)
    case("sign", ["%064x" % key("judge"), wn, KN], "a key without 0x is accepted, and a file carrying sig is signed with it inside", {"digest": hx(_dg), "presig": hx(_pre), "sig": G.sig_hex(_r, _s, _v), "signer": addr("judge")}, wn + "|sign_bare_key")
    case("sign", ["0x%064x" % key("judge"), write("door", "sign_bad_preimage", b"{"), KN], "an unreadable preimage: misuse", None, wn + "|sign_bad_preimage")
    case("sign", ["0x%064x" % key("judge"), write("door", "sign_array_preimage", b"[]"), KN], "a preimage that is no object: misuse", None, wn + "|sign_array_preimage")
    case("scan", [wn], "an unknown command: misuse", None, wn + "|unknown_command")
    dup = audit_input(J_, Lg, [carried(Lg[1], J_, bn=50)])
    AUD("duplicate_record", dup, PASSC, PASSC, "the dispatch bytes carried by two records collapse to one entry")
    # ---- round 1 additions (2026-10-16 revision)
    # the harness: the first token after door is the note path whatever its bytes; an option value may begin with --
    with open(os.path.join(HERE, "--note-dashes.json"), "wb") as f:
        f.write(nb)
    with open(os.path.join(HERE, "--now"), "wb") as f:
        f.write(json.dumps(good).encode())
    DOOR("note_path_begins_with_dashes", ["--note-dashes.json", "--now", "1250", "--audit", wa], "HARNESS: the first token after door is the note path whatever its bytes",
         result(PASSC, C_("SKIPPED", "NO_PASS"), C_("SKIPPED", "NO_PASS"), PASSC, PASSC, NOB, 1250, GB, None))
    DOOR("option_value_begins_with_dashes", [wn, wp, "--audit", "--now", "--now", "1250"], "HARNESS: an option takes the next token as its value whatever its bytes",
         result(PASSC, PASSC, PASSC, PASSC, PASSC, NOB, 1250, GB, None))
    case("door", [wn, wp, "--"], "HARNESS: a bare -- in the pass place is an unknown option: misuse", None, wn + "|bare_double_dash")
    # s9.2: a registries array of class items alone covers
    cls = {"chains": [{"chainId": 1, "fromBlock": 0, "toBlock": 1000, "registries": [{"codeHash": "0x" + "cd" * 32}], "senders": [J_]}]}
    AUD("class_item_only_registry", audit_input(J_, ledger("judge", []), [], bas=cls), PASSC, ND_F, "s9.2: a registries array holding a class item alone is non-empty; covering, FAIL under COMPLETE")
    # s12: a stray counted anchor carrying nothing beside a carried dispatch: GAPS, PASS
    AUD("stray_anchor_no_bytes_dispatch_carried", audit_input(J_, L, [anc(H32(), J_, bn=60)]), PASSC, PASSC, "s12: a counted anchor carrying no bytes is a MISSING row, label GAPS, and the carried dispatch passes")
    # s10.2 check 5: PASS under label UNPROVEN
    AUD("dispatch_carried_label_unproven", audit_input(J_, L, [anc(H32(), J_, bn=60, verdict="UNPROVEN")]), PASSC, PASSC, "s10.2 check 5: a carried dispatch passes under label UNPROVEN; the label is read only against COMPLETE")
    # s12 read short: senders omit the sitting key but the carried succession admits it: check 4 passes, check 5 not covering
    womit = write("door", "audit_senders_omit_sitting_key", json.dumps(audit_input(J_, Ls, [], senders=[J_])).encode())
    case("door", [wns, wps, "--now", "1250", "--audit", womit], "s12: the basis omits the sitting key, but the succession the root carried admits it to the lineage: check 4 passes and check 5 answers not covering",
         result(PASSC, PASSC, PASSC, PASSC, ND_U, NOB, 1250, basis([J_]), None, J_), wns + "|senders_omit_sitting_key")
    womitd = write("door", "audit_senders_omit_sitting_key_dispatched", json.dumps(audit_input(J_, Ls + [d2s], [], senders=[J_])).encode())
    case("door", [wns, wps, "--now", "1250", "--audit", womitd], "s12/s10.2: the basis omits the sitting key and is not covering, yet the root carried the successor's dispatch: a dispatch in hand is a witness, check 5 passes, GREEN",
         result(PASSC, PASSC, PASSC, PASSC, PASSC, NOB, 1250, basis([J_]), None, J_), wns + "|senders_omit_sitting_key_dispatched")
    # s9.0: a succession that fails s4.3 extends no lineage (carried by an UNPROVEN record so that no hard finding masks check 4)
    i_sig = Ls[1].index(b'"sig":"0x') + 9 + 7
    bad_sig = Ls[1][:i_sig] + (b"0" if Ls[1][i_sig:i_sig + 1] != b"0" else b"1") + Ls[1][i_sig + 1:]
    wbs = write("door", "audit_succession_bad_signature_unproven", json.dumps(audit_input(J_, [Ls[0]], [carried(bad_sig, J_, bn=50, verdict="UNPROVEN")], senders=[J_, S])).encode())
    case("door", [wns, wps, "--now", "1250", "--audit", wbs], "s9.0: a succession whose signature fails s4.3 admits nobody; the successor's note is not in the lineage",
         result(PASSC, PASSC, PASSC, C_("UNKNOWN", "NOT_IN_LINEAGE"), LU, NOB, 1250, basis([J_, S]), None, J_), wns + "|succession_bad_signature")
    bad_body = G.make_entry(key("judge"), "succession", 1, eid(Ls[0]), {"kind": "keyrotation", "effective": 1700000000, "statement_md": "no to"})
    wbb = write("door", "audit_succession_bad_body_unproven", json.dumps(audit_input(J_, [Ls[0]], [carried(bad_body, J_, bn=50, verdict="UNPROVEN")], senders=[J_, S])).encode())
    case("door", [wns, wps, "--now", "1250", "--audit", wbb], "s9.0: a succession whose body fails s4.3 (no to) admits nobody",
         result(PASSC, PASSC, PASSC, C_("UNKNOWN", "NOT_IN_LINEAGE"), LU, NOB, 1250, basis([J_, S]), None, J_), wns + "|succession_bad_body")
    stranger_succ = G.make_entry(key("x1"), "succession", 1, eid(Ls[0]), {"to": S, "kind": "keyrotation", "effective": 1700000000, "statement_md": "a stranger names the successor"}, judge=addr("x1"))
    wss = write("door", "audit_stranger_succession_unproven", json.dumps(audit_input(J_, [Ls[0]], [carried(stranger_succ, J_, bn=50, verdict="UNPROVEN")], senders=[J_, S])).encode())
    case("door", [wns, wps, "--now", "1250", "--audit", wss], "s9.0: a succession signed by a key outside the lineage admits nobody, whoever carried it",
         result(PASSC, PASSC, PASSC, C_("UNKNOWN", "NOT_IN_LINEAGE"), LU, NOB, 1250, basis([J_, S]), None, J_), wns + "|stranger_succession")
    # s9.0: the lineage is a fixpoint, whatever the order of the records
    rev = {"root": J_, "anchors": [carried(d2s, S, bn=51), carried(Ls[1], J_, bn=50), carried(Ls[0], J_, bn=49)], "basis": basis([J_, S])}
    wrev = write("door", "audit_records_reverse_order", json.dumps(rev).encode())
    case("door", [wns, wps, "--now", "1250", "--audit", wrev], "s9.0: the records listed with the successor's dispatch first; the lineage is a fixpoint and the order carries nothing",
         result(PASSC, PASSC, PASSC, PASSC, PASSC, NOB, 1250, basis([J_, S]), None, J_), wns + "|records_reverse_order")
    # s9.0: a counted record carrying nothing beside a non-counted carrier of the same bytes: no ledger entry
    AUD("hash_only_counted_beside_unproven_carrier", audit_input(J_, L[:1], [anc(eid(L[1]), J_, bn=50), carried(L[1], J_, bn=51, verdict="UNPROVEN")]), PASSC, ND_U, "s9.0: the counted record carries nothing and the carrier is UNPROVEN: no ledger entry, MISSING, GAPS, unknown")
    AUD("hash_only_counted_beside_stranger_carrier", audit_input(J_, L[:1], [anc(eid(L[1]), J_, bn=50), carried(L[1], addr("x1"), bn=51)], senders=[J_, addr("x1")]), PASSC, ND_U, "s9.0: the counted record carries nothing and the carrier is a stranger's discarded record: no ledger entry, GAPS, unknown")
    # s9.2: one chain in two windows differing in senders, the second short
    tws = audit_input(J_, Ls, [], senders=[J_, S]); tws["basis"]["chains"] = [{"chainId": 1, "fromBlock": 0, "toBlock": 500, "registries": [REG], "senders": sorted([J_, S], key=lambda a: a.encode())}, {"chainId": 1, "fromBlock": 501, "toBlock": 1000, "registries": [REG], "senders": [J_]}]
    AUD("one_chain_second_window_senders_short", tws, PASSC, ND_U, "s9.2: every object of the chain must name every lineage key; the second window lacks the successor")
    # s7.3 of galeed/1: a return to an earlier key is a lawful succession; the note's judge is in the lineage
    gS = G.make_entry(succ, "genesis", 0, None, {"statement_md": "the successor opened this ledger"})
    sS = G.make_entry(succ, "succession", 1, eid(gS), {"to": J_, "kind": "handover", "effective": 1700000000, "statement_md": "handed to the judge"})
    dS = G.make_entry(key("judge"), "galeed.dispatch/1", 2, eid(sS), {"noteHash": eid(nb)})
    wret = write("door", "audit_return_to_earlier_key", json.dumps(audit_input(S, [gS, sS, dS], [], senders=[S, J_])).encode())
    _root_of[json.dumps(basis([S, J_]), sort_keys=True)] = S
    DOOR("anchor_return_to_earlier_key", [wn, wp, "--now", "1250", "--audit", wret], "a ledger rooted at the successor that handed the seat to the judge: the judge is in the lineage and the dispatch is a ledger entry",
         result(PASSC, PASSC, PASSC, PASSC, PASSC, NOB, 1250, basis([S, J_]), None, S))
    # galeed/1 s7.4: a retired key's dispatch with no gap before it, carried in the second of its handing off, is DISCARDED and no ledger entry
    dret_c = G.make_entry(key("judge"), "galeed.dispatch/1", 2, eid(Lr[1]), {"noteHash": eid(nb)})
    AUD("retired_key_dispatch_certain", audit_input(J_, Lr + [dret_c], [], senders=[J_, S]), PASSC, C_("FAIL", "NOT_DISPATCHED"), "galeed/1 s7.4: the dispatch of the key that handed off, right after the succession and carried in that second, is DISCARDED; COMPLETE, so the absent dispatch convicts")



# --------------------------------------------------------------------------
# service record (s7a): every closed boundary on both sides, and the reading
# --------------------------------------------------------------------------

def svc_obj(**kw):
    d = {"kind": KS, "verifier": addr("ver"), "serves": [addr("judge")], "at": 1500}
    d.update(kw)
    return d


def SVC(**kw):
    return cbytes(J(svc_obj(**kw)))


def pub(emitter, block, idx, doc_bytes, word=None):
    """A publication element in the harness's file form: the document bytes
    in the transport hex spelling and the word."""
    w = hx(keccak256(doc_bytes)) if word is None else word
    return {"emitter": emitter, "blockNumber": block, "logIndex": idx,
            "document": hx(doc_bytes), "word": w}


def LOGS(pubs, statuses=None, root=None, declarations=None, accessions=None):
    v = {"publications": pubs, "statuses": statuses or [], "declarations": declarations or [],
         "accessions": accessions or []} if root is None else root
    return json.dumps(v, ensure_ascii=False).encode("utf-8")


def S(cmd, slug, data, args, note, pred):
    p = write("service", slug, data)
    case(cmd, [p] + args, note, pred, p)
    return p


def build_service():
    v, j, g, other = addr("ver"), addr("judge"), addr("guild"), addr("other")
    lo, hi = sorted([j, g], key=lambda a: a.encode("utf-8"))
    sb = SVC()
    D("service-check", "service_ok", sb, "a well formed service record, one seat", ok(sb))
    b = SVC(serves=[]); D("service-check", "service_empty_serves_ok", b, "an empty serves stands in nobody's service and is a record", ok(b))
    b = SVC(serves=[lo, hi]); D("service-check", "service_two_sorted_ok", b, "two seats, bytewise ascending", ok(b))
    b = SVC(at=0); D("service-check", "service_at_zero_ok", b, "at is data; zero is an int", ok(b))
    # parent's canonical-form tokens
    D("service-check", "service_bad_utf8", sb[:-1] + b"\xff", "s3 token first", bad("E_UTF8"))
    D("service-check", "service_not_canonical_ws", sub(sb, b'{"at"', b'{ "at"'), "a space: not canonical", bad("E_NOT_CANONICAL"))
    D("service-check", "service_unsorted", cbytes(JObj(list(reversed(J(svc_obj()).members))), sort=False), "members out of order", bad("E_NOT_CANONICAL"))
    D("service-check", "service_number_float", sub(sb, b'"at":1500', b'"at":1500.0'), "a fraction fails s3.2 at the parser", bad("E_NUMBER"))
    D("service-check", "service_root_array", b"[]", "root is not an object", bad("E_DOC"))
    # member set
    D("service-check", "service_missing_at", cbytes(J({k: x for k, x in svc_obj().items() if k != "at"})), "absent key", bad("E_DOC_MISSING"))
    D("service-check", "service_extra_member", cbytes(J(dict(svc_obj(), note_md="hello"))), "foreign key", bad("E_DOC_CLOSED"))
    D("service-check", "service_with_sig", cbytes(J(dict(svc_obj(), sig="0x" + "00" * 65))), "no signature is read: a sig member is a foreign key", bad("E_DOC_CLOSED"))
    D("service-check", "service_signed_like_a_paper", sign_doc(key("ver"), svc_obj(), KF), "a service record signed as a paper is a record with a foreign member", bad("E_DOC_CLOSED"))
    D("service-check", "service_missing_and_extra", cbytes(J(dict({k: x for k, x in svc_obj().items() if k != "serves"}, extra=1))), "absent before foreign", bad("E_DOC_MISSING"))
    # kind
    D("service-check", "service_kind_note", SVC(kind=KN), "kind of a paper", bad("E_KIND"))
    D("service-check", "service_kind_v2", SVC(kind="galeed.verifier/2"), "a later version", bad("E_KIND"))
    D("service-check", "service_kind_int", SVC(kind=1), "kind not a string", bad("E_KIND"))
    D("service-check", "service_kind_then_verifier", SVC(kind=KN, verifier="0x"), "kind decided before verifier", bad("E_KIND"))
    # verifier
    D("service-check", "service_verifier_upper", SVC(verifier="0x" + v[2:].upper()), "uppercase digits", bad("E_SERVICE_VERIFIER"))
    D("service-check", "service_verifier_short", SVC(verifier=v[:-2]), "38 digits", bad("E_SERVICE_VERIFIER"))
    D("service-check", "service_verifier_int", SVC(verifier=5), "not a string", bad("E_SERVICE_VERIFIER"))
    D("service-check", "service_verifier_then_serves", SVC(verifier="0x", serves=1), "verifier before serves", bad("E_SERVICE_VERIFIER"))
    b = SVC(verifier=other); D("service-check", "service_verifier_other_ok", b, "the check reads no publisher: any hex20 passes s7a.2", ok(b))
    # serves
    D("service-check", "service_serves_string", SVC(serves=j), "serves not an array", bad("E_SERVICE_SERVES"))
    D("service-check", "service_serves_object", SVC(serves={"a": j}), "serves an object", bad("E_SERVICE_SERVES"))
    D("service-check", "service_serves_then_at", SVC(serves=1, at="x"), "serves before at", bad("E_SERVICE_SERVES"))
    D("service-check", "service_address_bad_first", SVC(serves=["0x" + "ab" * 32]), "a hex32 element at index 0", bad("E_SERVICE_ADDRESS", 0))
    D("service-check", "service_address_bad_second", SVC(serves=[lo, "0xzz"]), "the first element that is not hex20, at index 1", bad("E_SERVICE_ADDRESS", 1))
    D("service-check", "service_address_upper", SVC(serves=["0x" + j[2:].upper()]), "uppercase digits are not hex20", bad("E_SERVICE_ADDRESS", 0))
    D("service-check", "service_address_before_dup", SVC(serves=[j, j, "0x"]), "every element tested for form before any repeat: index 2, not the repeat at 1", bad("E_SERVICE_ADDRESS", 2))
    D("service-check", "service_dup", SVC(serves=[j, j]), "a repeat at index 1", bad("E_SERVICE_DUP", 1))
    D("service-check", "service_dup_third", SVC(serves=[lo, hi, lo]), "the first repeat, at index 2", bad("E_SERVICE_DUP", 2))
    D("service-check", "service_dup_before_order", SVC(serves=[hi, lo, hi]), "repeats tested before order: the repeat at 2 wins over the descent at 1", bad("E_SERVICE_DUP", 2))
    D("service-check", "service_dup_abba", SVC(serves=[lo, hi, hi, lo]), "the least index whose element equals an earlier element: 2, not the second occurrence of the first element that recurs", bad("E_SERVICE_DUP", 2))
    D("service-check", "service_order", SVC(serves=[hi, lo]), "descending at index 1", bad("E_SERVICE_ORDER", 1))
    D("service-check", "service_order_third", SVC(serves=[lo, hi, lo[:-1] + ("0" if lo[-1] != "0" else "1")]) if False else SVC(serves=[lo, hi, "0x" + "00" * 20]), "ascending then a descent at index 2", bad("E_SERVICE_ORDER", 2))
    D("service-check", "service_order_then_at", SVC(serves=[hi, lo], at="x"), "order before at", bad("E_SERVICE_ORDER", 1))
    # at
    D("service-check", "service_at_string", SVC(at="1500"), "at a string", bad("E_SERVICE_AT"))
    D("service-check", "service_at_bool", SVC(at=True), "at a bool", bad("E_SERVICE_AT"))
    D("service-check", "service_at_negative", sub(sb, b'"at":1500', b'"at":-1'), "a negative int fails at the parser", bad("E_NUMBER"))
    # the reading (s7a.3)
    good = SVC()
    good2 = SVC(serves=[lo, hi], at=1600)
    bad_dup = SVC(serves=[j, j], at=1700)
    mism = SVC(verifier=other, at=1800)
    ws = sub(good, b'{"at"', b'{ "at"')
    roster = cbytes(J({"kind": "galeed.roster/1", "guild": v, "chainId": 1, "issued": 10, "judges": [j]}))
    nokind = cbytes(J({"verifier": v, "serves": [j], "at": 1}))
    kindint = cbytes(J({"kind": 7, "verifier": v, "serves": [j], "at": 1}))
    arr = b"[1,2]"
    def P(slug, pubs, note, pred, statuses=None):
        return S("service-of", slug, LOGS(pubs, statuses), [v], note, pred)
    SV = lambda sv: {"serves": sv, "verdict": "SERVICE"}
    P("none_empty_logs", [], "no publication at all", {"verdict": "SERVICE_NONE"})
    P("none_other_emitter", [pub(other, 5, 0, good)], "a record of this verifier published from another address is not this verifier's", {"verdict": "SERVICE_NONE"})
    P("one_record", [pub(v, 5, 0, good)], "one good record", SV([j]))
    P("latest_wins", [pub(v, 5, 0, good), pub(v, 6, 0, good2)], "the last record of the kind", SV([lo, hi]))
    P("latest_wins_listed_backwards", [pub(v, 6, 0, good2), pub(v, 5, 0, good)], "the file's order carries nothing", SV([lo, hi]))
    P("same_block_log_index", [pub(v, 5, 3, good2), pub(v, 5, 7, good)], "same block: the higher logIndex is later", SV([j]))
    P("block_before_log_index", [pub(v, 5, 9, good2), pub(v, 6, 0, good)], "blockNumber compared first: block 6 index 0 is after block 5 index 9", SV([j]))
    P("latest_unread_dup", [pub(v, 5, 0, good), pub(v, 6, 0, bad_dup)], "the latest record fails s7a.2: the whole list is declined, no fallback", {"index": 1, "token": "E_SERVICE_DUP", "verdict": "SERVICE_UNREAD"})
    P("latest_unread_not_canonical", [pub(v, 5, 0, good), pub(v, 6, 0, ws)], "the latest reads to an object of the kind but its bytes are not canonical: unread with the parent's token", {"token": "E_NOT_CANONICAL", "verdict": "SERVICE_UNREAD"})
    P("older_unread_latest_good", [pub(v, 5, 0, bad_dup), pub(v, 6, 0, good)], "an older bad record is not the candidate", SV([j]))
    P("mismatch", [pub(v, 5, 0, mism)], "the record names another verifier", {"verdict": "SERVICE_MISMATCH"})
    P("mismatch_latest_over_good", [pub(v, 5, 0, good), pub(v, 6, 0, mism)], "the latest mismatches, the older good one is not read", {"verdict": "SERVICE_MISMATCH"})
    P("unread_before_mismatch", [pub(v, 5, 0, SVC(verifier=other, serves=[j, j]))], "s7a.2 is decided before the verifier is compared", {"index": 1, "token": "E_SERVICE_DUP", "verdict": "SERVICE_UNREAD"})
    P("roster_after_record_skipped", [pub(v, 5, 0, good), pub(v, 6, 0, roster)], "a later publication of another kind is not a candidate", SV([j]))
    P("roster_only", [pub(v, 5, 0, roster)], "an address that publishes only rosters has no record", {"verdict": "SERVICE_NONE"})
    U = lambda tok, idx=None: ({"index": idx} if idx is not None else {}) | {"token": tok, "verdict": "SERVICE_UNREAD"}
    P("bad_hash_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, good2, word=hx(keccak256(b"x")))], "step 1 fails on the last publication: it is of every kind, so it is the candidate and the list is declined", U("E_HASH"))
    P("bad_hash_only", [pub(v, 5, 0, good, word=hx(keccak256(b"x")))], "step 1 fails on the only publication: the candidate, unread", U("E_HASH"))
    P("bad_hash_older_latest_good", [pub(v, 5, 0, good2, word=hx(keccak256(b"x"))), pub(v, 6, 0, good)], "an older unreadable publication is not the candidate", SV([j]))
    P("roster_bad_hash_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, roster, word=hx(keccak256(b"x")))], "roster bytes under a wrong word: step 1 fails before the kind is read, so the publication is of every kind and declines the list", U("E_HASH"))
    P("roster_dup_key_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, b'{"kind":"galeed.roster/1","a":1,"a":2}')], "roster bytes the reader refuses (a repeated member name) are of every kind: the candidate, unread at step 2 with E_JSON", U("E_JSON"))
    P("roster_float_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, b'{"kind":"galeed.roster/1","issued":10.0}')], "roster bytes with a fraction: the reader refuses them; the candidate, unread at step 2", U("E_JSON"))
    P("roster_bom_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, b'\xef\xbb\xbf{"kind":"galeed.roster/1"}')], "roster bytes behind a byte order mark: unreadable, so the candidate, unread at step 2", U("E_JSON"))
    P("roster_depth_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, b'{"kind":"galeed.roster/1","x":' + b'{"d":' * 128 + b'1' + b'}' * 129)], "roster bytes opening a container at depth 129: the reader refuses them, the candidate, unread at step 2", U("E_JSON"))
    P("roster_non_ascii_skipped", [pub(v, 5, 0, good), pub(v, 6, 0, json.dumps({"kind": "galeed.roster/1", "x": "\u00e9"}, ensure_ascii=False, separators=(",", ":")).encode("utf-8"))], "roster bytes the reader reads and the parent's charset would refuse: read to another kind, passed over; canonical form classifies nothing", SV([j]))
    P("roster_spacing_skipped", [pub(v, 5, 0, good), pub(v, 6, 0, b'{"kind": "galeed.roster/1"}')], "roster bytes with spacing: readable, another kind, passed over", SV([j]))


    P("json_array_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, arr)], "galeed.roster/1 s2.1 step 2 reads to an array, not an object: of every kind, the candidate, unread at step 2", U("E_JSON"))
    P("no_kind_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, nokind)], "an object with no kind is of every kind: the candidate, unread at galeed.roster/1 s2.1 step 3", U("E_KIND"))
    P("kind_int_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, kindint)], "a kind that is not a string: the candidate, refused at galeed.roster/1 s2.1 step 3", U("E_KIND"))
    P("later_version_unread", [pub(v, 5, 0, good), pub(v, 6, 0, SVC(kind="galeed.verifier/2"))], "a record of a later version of this kind is the candidate and fails at kind: the list is declined, as galeed.roster/1 s10.3 has it", U("E_KIND"))
    P("later_version_extra_member_unread", [pub(v, 5, 0, good), pub(v, 6, 0, cbytes(J(dict(svc_obj(kind="galeed.verifier/2"), x=1))))], "a later version with a fifth member: s7a.2's order decides the member set before kind", U("E_DOC_CLOSED"))
    P("family_kind_other_members_unread", [pub(v, 5, 0, good), pub(v, 6, 0, cbytes(J({"kind": "galeed.verifier/profile", "name": "x"})))], "a kind under the family prefix with another member set: the candidate, unread at the member set, never at kind", U("E_DOC_MISSING"))

    P("family_prefix_alone_unread", [pub(v, 5, 0, good), pub(v, 6, 0, SVC(kind="galeed.verifier/"))], "the family prefix alone is of this family and fails at kind", U("E_KIND"))
    P("other_family_skipped", [pub(v, 5, 0, good), pub(v, 6, 0, SVC(kind="galeed.verifierx/1"))], "a kind outside the family is another kind and is skipped", SV([j]))
    P("succession_skipped", [pub(v, 5, 0, good), pub(v, 6, 0, cbytes(J({"kind": "galeed.succession/1", "from": v, "to": other, "chainId": 1})))], "a succession record is another kind", SV([j]))
    P("escaped_kind_candidate", [pub(v, 5, 0, good), pub(v, 6, 0, sub(good2, b'"kind":"galeed.verifier/1"', b'"kind":"galeed.verifier\\/1"'))], "the kind is tested on the decoded string: an escaped slash still reads as this kind, and the bytes are not canonical", U("E_NOT_CANONICAL"))
    P("bad_utf8_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, good2[:-1] + b"\xff")], "bytes the reader refuses are of every kind: the candidate, unread at step 2", U("E_JSON"))
    P("dup_key_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, sub(good2, b'"at":1600', b'"at":1600,"at":1600'))], "a repeated member name: the reader refuses it, so it is the candidate, unread at step 2", U("E_JSON"))
    P("float_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, sub(good2, b'"at":1600', b'"at":1600.0'))], "a fraction: the reader refuses it; the candidate, unread at step 2", U("E_JSON"))
    P("abba_dup_index", [pub(v, 5, 0, SVC(serves=[lo, hi, hi, lo]))], "the least index whose element equals an earlier element: 2", U("E_SERVICE_DUP", 2))
    P("verifier_token_through_reading", [pub(v, 5, 0, SVC(verifier="0x"))], "s7a.2's verifier token passes through the reading unchanged", U("E_SERVICE_VERIFIER"))
    P("serves_token_through_reading", [pub(v, 5, 0, SVC(serves=j))], "the serves token, carrying no index", U("E_SERVICE_SERVES"))
    P("address_token_through_reading", [pub(v, 5, 0, SVC(serves=[lo, "0xzz"]))], "the address token with its index", U("E_SERVICE_ADDRESS", 1))
    P("order_token_through_reading", [pub(v, 5, 0, SVC(serves=[hi, lo]))], "the order token with its index", U("E_SERVICE_ORDER", 1))
    P("at_token_through_reading", [pub(v, 5, 0, SVC(at="x"))], "the at token", U("E_SERVICE_AT"))
    # G3: another kind passed over with a faulty or mismatched candidate behind it
    P("unread_record_under_roster", [pub(v, 5, 0, bad_dup), pub(v, 6, 0, roster)], "a later roster is passed over and the record behind it is the candidate: unread, no walk back to anything older", U("E_SERVICE_DUP", 1))
    P("mismatch_under_roster", [pub(v, 5, 0, mism), pub(v, 6, 0, roster)], "a later roster passed over, a mismatched record behind it", {"verdict": "SERVICE_MISMATCH"})

    # the reader seam, both sides (galeed.roster/1 s1 reads it, s7a.2's canonical(b) decides it)
    P("negative_int_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, sub(good2, b'"at":1600', b'"at":-1'))], "a negative int: the reader refuses it, the candidate, unread at step 2", U("E_JSON"))
    P("surrogate_escape_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, sub(good2, b'"at":1600', b'"at":1600,"x":"\\ud83d\\ude00"'))], "a surrogate pair escape: the reader reads one scalar, the parent's s3.3 refuses it", U("E_JSON"))
    deep = b'"x":' + b'{"d":' * 128 + b"1" + b"}" * 128
    P("depth_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, sub(good2, b'"at":1600', b'"at":1600,' + deep))], "a container opening at depth 129 inside an extra member: the reader refuses the bytes at step 2", U("E_JSON"))
    P("key_charset_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, sub(good2, b'"at":1600', b'"at":1600,"bad\\tkey":1'))], "a key carrying an escaped tab: the reader reads it, the parent's charset refuses it", U("E_KEY_CHARSET"))
    P("value_charset_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, sub(good2, b'"at":1600', b'"at":1600,"x":"a\\tb"'))], "an escaped tab below a non-prose key", U("E_VALUE_CHARSET"))
    P("extra_member_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, cbytes(J(dict(svc_obj(at=1600), x=1))))], "canonical, this kind, one foreign member", U("E_DOC_CLOSED"))
    # step 2's byte classes, both sides
    # another emitter interleaved; successions not followed
    P("other_emitter_interleaved", [pub(v, 5, 0, good), pub(other, 6, 0, SVC(verifier=other, serves=[g]))], "a later record of the kind from another address is not v's", SV([j]))
    succ = cbytes(J({"kind": "galeed.succession/1", "from": v, "to": other, "chainId": 1}))
    P("succession_not_followed", [pub(v, 5, 0, good), pub(v, 6, 0, succ), pub(other, 7, 0, SVC(verifier=other, serves=[g]))], "a succession is another kind and is not followed: v's own record is read", SV([j]))
    S("service-of", "successor_has_own_record", LOGS([pub(v, 5, 0, good), pub(v, 6, 0, succ)]), [other], "the successor's address, asked directly, has no record of its own", {"verdict": "SERVICE_NONE"})
    # serving over more than one publication
    lg5 = write("service", "serving_removed_logs", LOGS([pub(v, 5, 0, good2), pub(v, 6, 0, SVC(serves=[hi], at=1700))]))
    case("serving", [lg5, v, lo], "a seat removed by a later record", {"serving": False}, lg5)
    case("serving", [lg5, v, hi], "a seat kept by the later record", {"serving": True}, lg5)
    lg6 = write("service", "serving_older_under_unread_logs", LOGS([pub(v, 5, 0, good2), pub(v, 6, 0, bad_dup)]))
    case("serving", [lg6, v, lo], "a seat listed in an older good record under an unread latest: false, no fallback", {"serving": False}, lg6)
    lg7 = write("service", "serving_older_under_mismatch_logs", LOGS([pub(v, 5, 0, good2), pub(v, 6, 0, mism)]))
    case("serving", [lg7, v, lo], "a seat listed in an older good record under a mismatched latest: false", {"serving": False}, lg7)
    lg8 = write("service", "serving_roster_latest_logs", LOGS([pub(v, 5, 0, good2), pub(v, 6, 0, roster)]))
    case("serving", [lg8, v, lo], "a later publication of another kind is skipped: the record still serves", {"serving": True}, lg8)
    # s7a.2 adjacencies at the corpus's own standard
    D("service-check", "adj_canonical_before_verifier", cbytes(JObj(list(reversed(J(svc_obj(verifier="0x")).members))), sort=False), "members out of order and a bad verifier: canonical first", bad("E_NOT_CANONICAL"))
    D("service-check", "adj_closed_before_kind", cbytes(J(dict(svc_obj(kind=KN), x=1))), "a paper's kind and a foreign member: the member set first", bad("E_DOC_CLOSED"))
    D("service-check", "adj_missing_before_sig_closed", cbytes(J(dict({k: x for k, x in svc_obj().items() if k != "serves"}, sig="0x" + "00" * 65))), "sig present and serves absent: missing before foreign", bad("E_DOC_MISSING"))
    D("service-check", "service_missing_kind", cbytes(J({k: x for k, x in svc_obj().items() if k != "kind"})), "kind absent", bad("E_DOC_MISSING"))
    D("service-check", "service_missing_verifier", cbytes(J({k: x for k, x in svc_obj().items() if k != "verifier"})), "verifier absent", bad("E_DOC_MISSING"))
    # transport hex of either case; logs reader misuse forms
    bp = pub(v, 5, 0, good); bp["document"] = "0x" + bp["document"][2:].upper()
    P("document_transport_upper_read", [bp], "the transport hex spelling takes digits of either case", SV([j]))
    S("service-of", "misuse_logs_dup_key", LOGS([pub(v, 5, 0, good)])[:-1] + b',"statuses":[]}', [v], "HARNESS: a repeated member name in the logs file is refused by the reader", None)
    S("service-of", "misuse_logs_leading_zero", LOGS([dict(pub(v, 5, 0, good), blockNumber=5)]).replace(b'"blockNumber": 5', b'"blockNumber": 05'), [v], "HARNESS: an int with a leading zero is refused by the reader", None)
    # sign refuses the new literal as a domain
    sp = write("service", "sign_service_misuse", cbytes(J(svc_obj())))
    case("sign", ["0x%064x" % key("ver"), sp, KS], "galeed.verifier/1 is no signing domain: s3.2 closes the four", None, sp)
    P("empty_serves_record", [pub(v, 5, 0, SVC(serves=[]))], "an empty list is SERVICE with nobody", SV([]))
    P("not_canonical_only", [pub(v, 5, 0, ws)], "a sole record that reads to the kind but is not canonical: unread", {"token": "E_NOT_CANONICAL", "verdict": "SERVICE_UNREAD"})
    P("statuses_ignored", [pub(v, 5, 0, good)], "statuses are read for form and nothing else", SV([j]), statuses=[{"emitter": v, "blockNumber": 7, "logIndex": 0, "active": False}])
    # serving
    lg = write("service", "serving_logs", LOGS([pub(v, 5, 0, good2)]))
    case("serving", [lg, v, lo], "a listed seat", {"serving": True}, lg)
    case("serving", [lg, v, hi], "the other listed seat", {"serving": True}, lg)
    case("serving", [lg, v, other], "a seat not listed", {"serving": False}, lg)
    case("serving", [lg, other, lo], "a verifier with no record", {"serving": False}, lg)
    lg2 = write("service", "serving_unread_logs", LOGS([pub(v, 5, 0, bad_dup)]))
    case("serving", [lg2, v, j], "an unread record serves nobody, whatever it lists", {"serving": False}, lg2)
    lg3 = write("service", "serving_mismatch_logs", LOGS([pub(v, 5, 0, mism)]))
    case("serving", [lg3, v, j], "a mismatched record serves nobody", {"serving": False}, lg3)
    # misuse
    S("service-of", "misuse_root_array", LOGS([], root=[]), [v], "HARNESS: logs root is not an object", None)
    S("service-of", "misuse_no_publications", LOGS([], root={"statuses": [], "declarations": [], "accessions": []}), [v], "HARNESS: publications absent", None)
    S("service-of", "misuse_no_statuses", LOGS([], root={"publications": [], "declarations": [], "accessions": []}), [v], "HARNESS: statuses absent", None)
    bp = pub(v, 5, 0, good); bp["emitter"] = "0x" + v[2:].upper()
    S("service-of", "misuse_emitter_upper", LOGS([bp]), [v], "HARNESS: a publication outside its form", None)
    bp = pub(v, 5, 0, good); bp["document"] = good.decode("utf-8")
    S("service-of", "misuse_document_not_transport", LOGS([bp]), [v], "HARNESS: document is the transport hex spelling, never the text", None)
    bp = pub(v, 5, 0, good); del bp["word"]
    S("service-of", "misuse_no_word", LOGS([bp]), [v], "HARNESS: word absent", None)
    S("service-of", "misuse_shared_position", LOGS([pub(v, 5, 0, good), pub(other, 5, 0, good)]), [v], "HARNESS: two records share (blockNumber, logIndex)", None)
    S("service-of", "misuse_status_bad", LOGS([pub(v, 5, 0, good)], statuses=[{"emitter": v, "blockNumber": 6, "logIndex": 0, "active": 1}]), [v], "HARNESS: a status outside its form", None)
    S("service-of", "misuse_status_shares_position", LOGS([pub(v, 5, 0, good)], statuses=[{"emitter": v, "blockNumber": 5, "logIndex": 0, "active": True}]), [v], "HARNESS: a status sharing a publication's position", None)
    st5 = {"emitter": v, "blockNumber": 6, "logIndex": 0, "active": True}
    dc6 = {"emitter": v, "blockNumber": 6, "logIndex": 0, "to": other}
    ac6 = {"emitter": other, "blockNumber": 6, "logIndex": 0, "holder": v, "root": v}
    for slug, kw in (("status_status", {"statuses": [st5, dict(st5, emitter=other)]}),
                     ("status_declaration", {"statuses": [st5], "declarations": [dc6]}),
                     ("status_accession", {"statuses": [st5], "accessions": [ac6]}),
                     ("declaration_declaration", {"declarations": [dc6, dict(dc6, emitter=other, to=v)]}),
                     ("accession_accession", {"accessions": [ac6, dict(ac6, emitter=addr("x2"))]})):
        S("service-of", "misuse_shared_position_" + slug, LOGS([pub(v, 5, 0, good)], **kw), [v], "HARNESS: two elements of the four arrays share (blockNumber, logIndex): " + slug.replace("_", " and "), None)
    S("service-of", "misuse_address_upper", LOGS([pub(v, 5, 0, good)]), ["0x" + v[2:].upper()], "HARNESS: address not hex20", None)
    S("service-of", "misuse_address_short", LOGS([pub(v, 5, 0, good)]), [v[:-2]], "HARNESS: address not hex20", None)
    S("service-of", "misuse_extra_arg", LOGS([pub(v, 5, 0, good)]), [v, v], "HARNESS: an argument beyond the command's list", None)
    S("service-of", "misuse_unreadable_json", b"{", [v], "HARNESS: the reader refuses the file", None)
    bp = pub(v, 5, 0, good); bp["document"] = bp["document"] + "a"
    S("service-of", "misuse_document_odd_hex", LOGS([bp]), [v], "HARNESS: transport hex of odd length", None)
    bp = pub(v, 5, 0, good); bp["document"] = "0x" + "zz" + bp["document"][2:]
    S("service-of", "misuse_document_not_hex", LOGS([bp]), [v], "HARNESS: transport hex with a non-hex digit", None)
    S("service-of", "misuse_publication_not_object", LOGS([[]]), [v], "HARNESS: a publication element that is not an object", None)
    S("service-of", "misuse_status_not_object", LOGS([pub(v, 5, 0, good)], statuses=[1]), [v], "HARNESS: a status element that is not an object", None)
    S("service-of", "misuse_status_missing_member", LOGS([pub(v, 5, 0, good)], statuses=[{"emitter": v, "blockNumber": 6, "logIndex": 0}]), [v], "HARNESS: a status with active absent", None)
    P("unreadable_json_latest_unread", [pub(v, 5, 0, good), pub(v, 6, 0, b"{")], "bytes the reader refuses are the candidate: truncated JSON, unread at step 2", U("E_JSON"))
    lg4 = write("service", "serving_misuse_logs", LOGS([pub(v, 5, 0, good)]))
    case("serving", [lg4, v], "HARNESS: a missing argument", None, lg4)
    case("serving", [lg4, v, "0x" + lo[2:].upper()], "HARNESS: seat not hex20", None, lg4)
    case("serving", [lg4, "0x" + v[2:].upper(), lo], "HARNESS: verifier not hex20", None, lg4)


# --------------------------------------------------------------------------
# the caller's current key of the note's verifier (s1 current; s6.4, s7.4,
# s10.2 check 2), and the retired key's anchors (s12)
# --------------------------------------------------------------------------

def build_current(nb, pb, rb, fb, proto):
    J_ = addr("judge"); V = addr("ver"); V2 = addr("ver2")
    NOB = C_("SKIPPED", "NO_BENCH"); SP = C_("SKIPPED", "NO_PASS"); LU2 = C_("UNKNOWN", "LEDGER_UNREAD")
    UR = C_("UNKNOWN", "VERIFIER_UNRESOLVED"); PVM = C_("FAIL", "PASS_VERIFIER_MISMATCH")
    wn = write("door", "cur_note", nb); wp = write("door", "cur_pass", pb)
    pv2 = sign_doc(key("ver2"), pass_obj(nb, verifier=V2), KP)
    wpv2 = write("door", "cur_pass_by_moved_key", pv2)
    Lg = ledger("judge", [], nb)
    good = audit_input(J_, Lg, [])
    GB = good["basis"]
    wa = write("door", "cur_audit_good", json.dumps(good).encode())
    bv = write("door", "cur_bench_note_verifier", json.dumps([V]).encode())
    bv2 = write("door", "cur_bench_moved_key", json.dumps([V2]).encode())

    def CUR(slug, args, note, pred):
        case("door", args, note, pred, args[0] + "|" + slug)

    CUR("current_unresolved", [wn, wp, "--now", "1250", "--audit", wa, "--current", "unresolved"],
        "s10.2 check 2: current unresolved: UNKNOWN with VERIFIER_UNRESOLVED; check 3 tests the deadline alone",
        result(PASSC, UR, SP, PASSC, PASSC, NOB, 1250, GB, None, J_, current="unresolved"))
    CUR("current_omitted|nocurrent", [wn, wp, "--now", "1250", "--audit", wa],
        "HARNESS: omitting --current passes unresolved",
        result(PASSC, UR, SP, PASSC, PASSC, NOB, 1250, GB, None, J_, current="unresolved"))
    CUR("current_moved_key", [wn, wpv2, "--now", "1250", "--audit", wa, "--current", V2],
        "s10.2 check 2: the pass is signed by the note verifier's current key: PASS",
        result(PASSC, PASSC, PASSC, PASSC, PASSC, NOB, 1250, GB, None, J_, current=V2))
    CUR("current_moved_key_old_pass", [wn, wp, "--now", "1250", "--audit", wa, "--current", V2],
        "s10.2 check 2: a pass signed by the key the verifier left: PASS_VERIFIER_MISMATCH",
        result(PASSC, PVM, SP, PASSC, PASSC, NOB, 1250, GB, None, J_, current=V2))
    CUR("current_names_note_verifier_moved_pass", [wn, wpv2, "--now", "1250", "--audit", wa, "--current", V],
        "s10.2 check 2: the pass is compared with current and never with the note's verifier",
        result(PASSC, PVM, SP, PASSC, PASSC, NOB, 1250, GB, None, J_, current=V))
    CUR("current_moved_key_bench_reads_note", [wn, wpv2, "--now", "1250", "--audit", wa, "--bench", bv, "--current", V2],
        "s4.1: check 6 looks the note's verifier itself up on the bench",
        result(PASSC, PASSC, PASSC, PASSC, PASSC, PASSC, 1250, GB, [V], J_, current=V2))
    CUR("current_moved_key_bench_holds_only_new_key", [wn, wpv2, "--now", "1250", "--audit", wa, "--bench", bv2, "--current", V2],
        "s4.1: a bench that holds the current key and not the note's verifier: NOT_ON_BENCH (a bench resolved whole on the book current is read from never holds the one key without the other; this one is read from another book)",
        result(PASSC, PASSC, PASSC, PASSC, PASSC, C_("FAIL", "NOT_ON_BENCH"), 1250, GB, [V2], J_, current=V2))
    wpo = write("door", "cur_pass_other_note", PASS(NOTE(scope_md="another note for current")))
    CUR("current_note_mismatch_first", [wn, wpo, "--now", "1250", "--current", "unresolved"],
        "s10.2 check 2 order: the pass's noteHash is decided before current",
        result(PASSC, C_("FAIL", "PASS_NOTE_MISMATCH"), SP, C_("UNKNOWN", "NO_LEDGER"), C_("UNKNOWN", "LEDGER_UNREAD"), NOB, 1250, None, None, current="unresolved"))
    wbp = write("door", "cur_pass_bad", PASS(nb, verifier=""))
    CUR("current_bad_pass_first", [wn, wbp, "--current", "unresolved"],
        "s10.2 check 2 order: a pass that fails s5.2 is decided before current",
        result(PASSC, C_("FAIL", ["BAD_PASS", "E_PASS_VERIFIER"]), C_("UNKNOWN", "NO_CLOCK"), C_("UNKNOWN", "NO_LEDGER"), C_("UNKNOWN", "LEDGER_UNREAD"), NOB, None, None, None, current="unresolved"))
    CUR("current_no_pass", [wn, "--now", "1250", "--audit", wa, "--current", "unresolved"],
        "s10.2 check 2: no pass: SKIPPED, whatever current is",
        result(PASSC, SP, SP, PASSC, PASSC, NOB, 1250, GB, None, J_, current="unresolved"))
    wbn = write("door", "cur_note_bad", sign_doc(key("ver"), note_obj(), KN))
    SK = C_("SKIPPED", "NO_NOTE")
    CUR("current_bad_note", [wbn, wp, "--current", V2], "s10.4: current echoes the input whatever check 1 decided",
        result(C_("FAIL", ["BAD_NOTE", "E_SIG_SIGNER"]), SK, SK, SK, SK, SK, None, None, None, current=V2))
    CUR("current_offline", [wn, wp, "--current", V], "s10.5: a door with no ledger reach and no clock decides check 2 in full where it holds a current",
        result(PASSC, PASSC, C_("UNKNOWN", "NO_CLOCK"), C_("UNKNOWN", "NO_LEDGER"), C_("UNKNOWN", "LEDGER_UNREAD"), NOB, None, None, None, current=V))
    CUR("current_option_first", [wn, wp, "--current", V, "--now", "1250", "--audit", wa],
        "HARNESS: --current in any place among the options",
        result(PASSC, PASSC, PASSC, PASSC, PASSC, NOB, 1250, GB, None, J_, current=V))
    for slug, val in (("upper", "0x" + V[2:].upper()), ("short", V[:-2]), ("word_case", "Unresolved"), ("empty", ""), ("no_prefix", V[2:])):
        case("door", [wn, wp, "--current", val], "HARNESS: --current is a hex20 or the word unresolved", None, wn + "|current_misuse_" + slug)
    case("door", [wn, wp, "--current", V, "--current", V], "HARNESS: --current repeated", None, wn + "|current_misuse_repeat")
    case("door", ["--current", V, "--now", "1250"], "HARNESS: --current as the first token after door is the note path, a path that cannot be read", None, "(none)|current_as_note_path")
    case("door", [wn, wp, "--current"], "HARNESS: --current without its value", None, wn + "|current_misuse_no_value")
    case("door", [wn, wp, "--current=" + V], "HARNESS: a joined option", None, wn + "|current_misuse_joined")

    # pairing against the current key
    pn = write("pair", "cur_note", nb); pr = write("pair", "cur_report", rb)
    rv2 = sign_doc(key("ver2"), report_obj(nb, proto, verifier=V2), KR)
    prv2 = write("pair", "cur_report_by_moved_key", rv2)
    case("pair", [pn, pr, "unresolved"], "s6.4: current unresolved", {"verdict": "REPORT_VERIFIER_UNRESOLVED"}, pr + "|unresolved")
    case("pair", [pn, prv2, V2], "s6.4: a report signed by the note verifier's current key pairs", {"verdict": "PAIRED"}, prv2 + "|moved")
    case("pair", [pn, pr, V2], "s6.4: a report signed by the key the verifier left", {"verdict": "REPORT_VERIFIER_MISMATCH"}, pr + "|old_key")
    case("pair", [pn, prv2, V], "s6.4: the report is compared with current and never with the note's verifier", {"verdict": "REPORT_VERIFIER_MISMATCH"}, prv2 + "|note_verifier")
    prn = write("pair", "cur_report_other_note", REPORT(NOTE(scope_md="other note, pairing"), proto))
    case("pair", [pn, prn, "unresolved"], "s6.4 order: noteHash before current", {"verdict": "REPORT_NOTE_MISMATCH"}, prn + "|unresolved")
    pri = write("pair", "cur_report_invalid", REPORT(nb, proto, manifest=1))
    case("pair", [pn, pri, "unresolved"], "s6.4 order: an invalid report before current", {"verdict": "REPORT_INVALID", "token": "E_REPORT_MANIFEST"}, pri + "|unresolved")
    prp = write("pair", "cur_report_other_protocol_moved", sign_doc(key("ver2"), report_obj(nb, H32(), verifier=V2), KR))
    case("pair", [pn, prp, V2], "s6.4 order: current decided, then protocol", {"verdict": "REPORT_PROTOCOL_MISMATCH"}, prp + "|moved")
    case("pair", [pn, prp, "unresolved"], "s6.4 order: current before protocol", {"verdict": "REPORT_VERIFIER_UNRESOLVED"}, prp + "|unresolved")
    case("pair", [pn, pr, "0x" + V[2:].upper()], "HARNESS: <current> outside hex20", None, pr + "|misuse_upper")
    case("pair", [pn, pr, "UNRESOLVED"], "HARNESS: the word compares exactly", None, pr + "|misuse_word")
    case("pair", [pn, pr, V, V], "HARNESS: an argument beyond the list", None, pr + "|misuse_extra")
    fn = write("pair", "cur_refusal_note", nb); fp = write("pair", "cur_refusal_pass", pb); ff = write("pair", "cur_refusal", fb)
    fv2 = sign_doc(key("ver2"), refusal_obj(nb, pb, verifier=V2), KF)
    ffv2 = write("pair", "cur_refusal_by_moved_key", fv2)
    case("pair-refusal", [fn, fp, ff, "unresolved"], "s7.4: current unresolved", {"verdict": "REFUSAL_VERIFIER_UNRESOLVED"}, ff + "|unresolved")
    case("pair-refusal", [fn, fp, ffv2, V2], "s7.4: a record signed by the note verifier's current key pairs", {"verdict": "REFUSAL_PAIRED"}, ffv2 + "|moved")
    case("pair-refusal", [fn, fp, ff, V2], "s7.4: a record signed by the key the verifier left", {"verdict": "REFUSAL_VERIFIER_MISMATCH"}, ff + "|old_key")
    fpo = write("pair", "cur_refusal_other_pass", PASS(nb, orderRef="SF-cur"))
    case("pair-refusal", [fn, fpo, ff, "unresolved"], "s7.4 order: passHash before current", {"verdict": "REFUSAL_PASS_MISMATCH"}, fpo + "|unresolved")
    case("pair-refusal", [fn, fp, ff, "unresolved "], "HARNESS: a trailing space is no spelling of the word", None, ff + "|misuse_space")
    case("pair-refusal", [fn, fp, ff], "HARNESS: <current> missing", None, ff + "|misuse_missing")

    # s12: the retired key's anchors count for nothing after its retirement
    Ssk = key("succ"); Sa = addr("succ")
    Lret = ledger("judge", [("succession", {"to": Sa, "kind": "keyrotation", "effective": 1700000000, "statement_md": "rotated"})])
    def RET(slug, anchors, ledger_c, anchor_c, why, ents=None):
        inp = {"root": J_, "anchors": [carried(e, J_, bn=1 + i, ts=1400) for i, e in enumerate(ents or Lret)] + anchors,
               "basis": basis([J_, Sa])}
        w = write("door", "ret_" + slug, json.dumps(inp).encode())
        case("door", [wn, wp, "--now", "1250", "--audit", w, "--current", V], why,
             result(PASSC, PASSC, PASSC, ledger_c, anchor_c, NOB, 1250, basis([J_, Sa]), None, J_, current=V), wn + "|ret_" + slug)
    RET("stray_after_retirement", [anc(H32(), J_, bn=60, ts=1401)], PASSC, C_("FAIL", "NOT_DISPATCHED"),
        "s12, galeed/1 s7.4 and s8.1: the retired key's stray anchor after its retirement stands in DISCARDED; COMPLETE, so the absent dispatch convicts")
    RET("stray_at_retirement", [anc(H32(), J_, bn=60, ts=1400)], PASSC, C_("UNKNOWN", "NOT_DISPATCHED"),
        "galeed/1 s7.4: a record of its retirement's own second that carries no byte string is one at which the key has not retired; the stray anchor is MISSING and the label GAPS")
    dret = G.make_entry(key("judge"), "galeed.dispatch/1", 2, eid(Lret[1]), {"noteHash": eid(nb)})
    RET("retired_dispatch_after_retirement", [carried(dret, J_, bn=61, ts=1500)], PASSC, C_("FAIL", "NOT_DISPATCHED"),
        "s12: the retired key's dispatch at a position after it handed off, carried after its retirement, is no ledger entry")
    RET("retired_dispatch_at_retirement", [carried(dret, J_, bn=61, ts=1400)], PASSC, C_("FAIL", "NOT_DISPATCHED"),
        "galeed/1 s7.4: carried at its retirement's second at a position after its handing off, the retired key's dispatch is DISCARDED as it would be a second later; no ledger entry")
    dsucc = G.make_entry(Ssk, "galeed.dispatch/1", 2, eid(Lret[1]), {"noteHash": eid(nb)})
    RET("successor_dispatch_after_retirement", [carried(dsucc, Sa, bn=62, ts=1600)], PASSC, PASSC,
        "galeed/1 s8.1: the successor has not retired; its dispatch counts whenever it is carried")
    RET("retired_key_carries_successor_dispatch_late", [carried(dsucc, J_, bn=62, ts=1600)], PASSC, C_("FAIL", "NOT_DISPATCHED"),
        "galeed/1 s8.1: the retired key's record carrying the successor's dispatch after the retirement is discarded; the dispatch rides no counted undiscarded record")
    Lmig = ledger("judge", [("migration", {"spec": "galeed/2", "statement_md": "moving to the next law"})], nb)
    RET("dispatch_before_migration", [], PASSC, PASSC,
        "galeed/1 s6.6a: a ledger that closed by migration still holds the entries before it", ents=Lmig)
    dlate = G.make_entry(key("judge"), "galeed.dispatch/1", 3, eid(Lmig[2]), {"noteHash": eid(NOTE(scope_md="after migration"))})
    RET("entry_after_migration_same_second", [carried(dlate, J_, bn=63, ts=1400)], PASSC, PASSC,
        "galeed/1 s7.4: the migration retires its key; its entry past the migration, carried at the migration's own second, is DISCARDED and the closed ledger reads as it closed", ents=Lmig)
    RET("entry_after_migration_later", [carried(dlate, J_, bn=63, ts=1401)], PASSC, PASSC,
        "galeed/1 s7.4: carried after the migration's first record, the key's entry is discarded and the closed ledger reads as it closed", ents=Lmig)

    # s12, galeed/1 s7.4 across chains: a key retires at the earliest record carrying its handing off, on any chain, whatever the sender
    def XR(slug, recs, ledger_c, anchor_c, why, senders, nbp=None):
        sd = sorted(set(senders), key=lambda a: a.encode())
        bas = {"chains": [{"chainId": c, "fromBlock": 0, "toBlock": 1000, "registries": [REG], "senders": sd} for c in (1, 2)]}
        inp = {"root": J_, "anchors": recs, "basis": bas}
        w = write("door", "xr_" + slug, json.dumps(inp).encode())
        n_, p_ = (wn, wp) if nbp is None else nbp
        case("door", [n_, p_, "--now", "1250", "--audit", w, "--current", V], why,
             result(PASSC, PASSC, PASSC, ledger_c, anchor_c, NOB, 1250, bas, None, J_, current=V), n_ + "|xr_" + slug)

    def on2(rec):
        return dict(rec, chainId=2)
    Lx = ledger("judge", [("succession", {"to": Sa, "kind": "keyrotation", "effective": 1700000000, "statement_md": "rotated"})], nb)
    for slug, t, anchor_c, why in (
            ("dispatch_before_retirement", 1399, PASSC, "galeed/1 s7.4: the succession is first carried on chain 2 at 1400; the dispatch's record on chain 1 a second earlier counts: GREEN"),
            ("dispatch_at_retirement", 1400, PASSC, "galeed/1 s7.4: the dispatch's record on chain 1 at the second of the succession's earliest record on chain 2 counts: GREEN"),
            ("dispatch_after_retirement", 1401, C_("UNKNOWN", "NOT_DISPATCHED"), "galeed/1 s7.4, s8.1: the succession first carried on chain 2 at 1400 retires the key there for chain 1 as well; the dispatch's record at 1401 is discarded, the gap at its position makes GAPS, and no dispatch is in the ledger")):
        XR(slug, [carried(Lx[0], J_, bn=1, ts=1000), carried(Lx[1], J_, bn=2, ts=t), carried(Lx[2], J_, bn=3, ts=1500), on2(carried(Lx[2], J_, bn=5, ts=1400))],
           PASSC, anchor_c, why, [J_, Sa])
    X1 = addr("x1")
    XR("stranger_early_carrier_retires_key", [carried(Lx[0], J_, bn=1, ts=1000), carried(Lx[1], J_, bn=2, ts=1350), carried(Lx[2], J_, bn=3, ts=1400), on2(carried(Lx[2], X1, bn=5, ts=1300))],
       PASSC, C_("FAIL", "NOT_DISPATCHED"), "galeed/1 s7.4: a stranger's discarded record of the succession on chain 2 at 1300 retires the key from that second; the key's own records of the dispatch and the succession are discarded, the ledger is the genesis alone, COMPLETE, and the absent dispatch convicts", [J_, Sa, X1])
    XR("successor_recarries_held_dispatch", [carried(Lx[0], J_, bn=1, ts=1000), carried(Lx[1], J_, bn=2, ts=1401), carried(Lx[2], J_, bn=3, ts=1400), carried(Lx[1], Sa, bn=6, ts=1500)],
       PASSC, PASSC, "galeed/1 s8.6: the retired key's dispatch record is discarded and the successor's counted record carries the same bytes: the dispatch is a ledger entry, COMPLETE, GREEN", [J_, Sa])
    XR("genesis_anchored_after_retirement", [carried(Lx[0], J_, bn=7, ts=1401), carried(Lx[1], J_, bn=2, ts=1300), carried(Lx[2], J_, bn=3, ts=1400)],
       PASSC, PASSC, "galeed/1 s8.1: the genesis's own record after the retirement is discarded; the gap at position 0 makes GAPS and the dispatch carried before still passes", [J_, Sa])
    # the lineage admits a succession only by a record at which its judge has not retired
    K2 = addr("k2"); K3 = addr("k3")
    nk2 = sign_doc(key("k2"), note_obj(judge=K2, scope_md="a note by the key a late fork names"), KN)
    pk2 = PASS(nk2)
    wnk2, wpk2 = write("door", "note_k2", nk2), write("door", "pass_k2", pk2)
    fork = G.make_entry(key("judge"), "succession", 1, eid(Lret[0]), {"to": K2, "kind": "keyrotation", "effective": 1700000000, "statement_md": "a second hand-off"})
    XR("late_fork_admits_nobody", [carried(Lret[0], J_, bn=1, ts=1000), carried(Lret[1], J_, bn=2, ts=1400), carried(fork, J_, bn=3, ts=1401)],
       C_("UNKNOWN", "NOT_IN_LINEAGE"), LU2, "galeed/1 s7.4: a second succession the key signed, carried only after its retirement, admits nobody; the note of the key it names is not in the lineage", [J_, Sa], nbp=(wnk2, wpk2))
    XR("fork_at_retirement_second", [carried(Lret[0], J_, bn=1, ts=1000), carried(Lret[1], J_, bn=2, ts=1400), carried(fork, J_, bn=3, ts=1400)],
       C_("FAIL", "BROKEN_LEDGER"), LU2, "galeed/1 s7.4, s8.3: carried at the retirement's second, the second succession admits its key and is an equivocation: BROKEN_CHAIN", [J_, Sa, K2], nbp=(wnk2, wpk2))
    nk3 = sign_doc(key("k3"), note_obj(judge=K3, scope_md="a note by the key named after a migration"), KN)
    pk3 = PASS(nk3)
    wnk3, wpk3 = write("door", "note_k3", nk3), write("door", "pass_k3", pk3)
    Lm2 = ledger("judge", [("migration", {"spec": "galeed/2", "statement_md": "moving to the next law"})])
    late_s = G.make_entry(key("judge"), "succession", 2, eid(Lm2[1]), {"to": K3, "kind": "keyrotation", "effective": 1700000000, "statement_md": "after the migration"})
    XR("succession_after_migration_late", [carried(Lm2[0], J_, bn=1, ts=1000), carried(Lm2[1], J_, bn=2, ts=1400), carried(late_s, J_, bn=3, ts=1401)],
       C_("UNKNOWN", "NOT_IN_LINEAGE"), LU2, "galeed/1 s7.4: a migration retires its key; a succession it signs after, carried later, admits nobody", [J_, K3], nbp=(wnk3, wpk3))
    XR("succession_after_migration_same_second", [carried(Lm2[0], J_, bn=1, ts=1000), carried(Lm2[1], J_, bn=2, ts=1400), carried(late_s, J_, bn=3, ts=1400)],
       C_("UNKNOWN", "NOT_IN_LINEAGE"), LU2, "galeed/1 s7.4: carried at the migration's second at a position after it, the succession the key signs admits nobody and is DISCARDED, as it would be a second later", [J_, K3], nbp=(wnk3, wpk3))

    # s9.0: the papers lineage's own retirement, read from the carried bytes of a handing off, whatever the record's sender and verdict
    nS = sign_doc(Ssk, note_obj(judge=Sa, scope_md="a note by the successor"), KN)
    wnS, wpS = write("door", "note_succ", nS), write("door", "pass_succ", PASS(nS))
    sRS = Lret[1]
    dS = G.make_entry(Ssk, "galeed.dispatch/1", 2, eid(sRS), {"noteHash": eid(nS)})
    mR = G.make_entry(key("judge"), "migration", 2, eid(sRS), {"spec": "galeed/2", "statement_md": "moving to the next law"})
    base = [carried(Lret[0], J_, bn=1, ts=1000), carried(sRS, J_, bn=3, ts=1400), carried(dS, Sa, bn=4, ts=1450)]
    XR("lineage_hashonly_early_record_of_succession", base + [on2(anc(eid(sRS), J_, bn=12, ts=1300))], PASSC, PASSC,
       "s9.0, galeed/1 s7.4: a record carrying the succession's hash alone at 1300 retires nobody; the succession carried at 1400 admits the successor, whose note opens", [J_, Sa], nbp=(wnS, wpS))
    XR("lineage_hashonly_early_record_of_migration", base + [on2(anc(eid(mR), X1, bn=12, ts=1300)), on2(carried(mR, X1, bn=19, ts=1500))], PASSC, PASSC,
       "s9.0, galeed/1 s7.4: the migration's hash alone at 1300 retires nobody, its bytes arriving at 1500; the succession at 1400 admits the successor", [J_, Sa, X1], nbp=(wnS, wpS))
    XR("lineage_migration_bytes_early", base + [on2(carried(mR, X1, bn=12, ts=1300))], C_("UNKNOWN", "NOT_IN_LINEAGE"), LU2,
       "s9.0, galeed/1 s7.4: the migration's bytes, carried by a stranger at 1300, retire the root; the succession carried at 1400 admits nobody", [J_, Sa, X1], nbp=(wnS, wpS))
    for v_, art in (("UNPROVEN", "an"), ("VOID", "a")):
        XR("lineage_key_admitted_only_by_%s_record" % v_, [carried(Lret[0], J_, bn=1, ts=1000), carried(sRS, J_, bn=3, ts=1400, verdict=v_), carried(dS, Sa, bn=4, ts=1450)],
           PASSC, C_("UNKNOWN", "DISPATCH_DISPUTED"),
           "s9.0, galeed/1 s7.4: a succession carried only by %s %s record admits its key: the successor's note is in the lineage, its dispatch disputed" % (art, v_), [J_, Sa], nbp=(wnS, wpS))
        XR("lineage_%s_migration_retires_before_succession" % v_, [carried(Lret[0], J_, bn=1, ts=1000), carried(mR, X1, bn=2, ts=1200, verdict=v_), carried(sRS, J_, bn=3, ts=1300)],
           C_("UNKNOWN", "NOT_IN_LINEAGE"), LU2,
           "s9.0, galeed/1 s7.4: a stranger's %s record of the root's migration at 1200 retires the root; its succession carried at 1300 admits nobody" % v_, [J_, Sa, X1], nbp=(wnS, wpS))
    # a return to the root: the root retired at its first handing off for good
    sSR = G.make_entry(Ssk, "succession", 2, eid(sRS), {"to": J_, "kind": "keyrotation", "effective": 1700000000, "statement_md": "back to the root"})
    dR3 = G.make_entry(key("judge"), "galeed.dispatch/1", 3, eid(sSR), {"noteHash": eid(nb)})
    for sender, t, anchor_c, why in ((J_, 1500, C_("FAIL", "NOT_DISPATCHED"), "galeed/1 s7.4: a key retires at its first handing off for good; the root's own dispatch after the return is discarded, COMPLETE, and the absent dispatch convicts"),
                                     (Sa, 1400, PASSC, "galeed/1 s8.6: the same dispatch carried by the successor at its own retirement's second is a ledger entry: GREEN")):
        XR("return_to_root_dispatch_by_" + ("root" if sender == J_ else "successor"), [carried(Lret[0], J_, bn=1, ts=1000), carried(sRS, J_, bn=2, ts=1300), carried(sSR, Sa, bn=3, ts=1400), carried(dR3, sender, bn=4, ts=t)],
           PASSC, anchor_c, why, [J_, Sa])
    # the key's own uncounted early carrier of its succession on another chain retires it
    for v_, ledger_c, anchor_c in (("UNPROVEN", PASSC, C_("UNKNOWN", "NOT_DISPATCHED")), ("VOID", PASSC, C_("FAIL", "NOT_DISPATCHED"))):
        XR("own_%s_carrier_on_another_chain_early" % v_, [carried(Lx[0], J_, bn=1, ts=10), carried(Lx[1], J_, bn=40, ts=80), carried(Lx[2], J_, bn=50, ts=100), on2(carried(Lx[2], J_, bn=6, ts=60, verdict=v_))],
           ledger_c, anchor_c, "galeed/1 s7.4: the key's own %s record of its succession on chain 2 at 60 retires it there; its dispatch at 80 is discarded" % v_, [J_, Sa])
    # the walked authority outside the lineage, at the door: a far succession retires the root first, the line succession is carried later by a lineage key
    sfar = G.make_entry(key("judge"), "succession", 5, H32(), {"to": K2, "kind": "keyrotation", "effective": 1700000000, "statement_md": "a far hand-off"})
    s13 = G.make_entry(key("judge"), "succession", 1, eid(Lret[0]), {"to": K3, "kind": "keyrotation", "effective": 1700000000, "statement_md": "the line hand-off"})
    walk = [carried(Lret[0], J_, bn=1, ts=1), carried(sfar, J_, bn=2, ts=5, verdict="VOID"), carried(s13, K2, bn=5, ts=10)]
    d3 = G.make_entry(key("k3"), "galeed.dispatch/1", 2, eid(s13), {"noteHash": eid(nk3)})
    XR("walked_outsider_own_dispatch", walk + [carried(d3, K3, bn=6, ts=12)], C_("UNKNOWN", "NOT_IN_LINEAGE"), LU2,
       "galeed/1 s7.4, s8.2: the line succession carried after the root retired moves the walk and admits nobody; the key it puts in office is outside the lineage, whose note no door opens", [J_, K2, K3], nbp=(wnk3, wpk3))
    XR("walked_lineage_key_no_dispatch", walk, PASSC, C_("FAIL", "NOT_DISPATCHED"),
       "galeed/1 s7.4: the far succession's key is in the lineage, admitted at the root's retirement second; its note is read, COMPLETE, and no dispatch is there", [J_, K2, K3], nbp=(wnk2, wpk2))
    # the ends of the clock
    for slug, ts_s, ts_d, anchor_c in (("retirement_at_zero_dispatch_at_zero", 0, 0, PASSC), ("retirement_at_zero_dispatch_at_one", 0, 1, C_("UNKNOWN", "NOT_DISPATCHED")),
                                       ("retirement_at_max_dispatch_at_max", MAXT, MAXT, PASSC), ("retirement_below_max_dispatch_at_max", MAXT - 1, MAXT, C_("UNKNOWN", "NOT_DISPATCHED"))):
        XR(slug, [carried(Lx[0], J_, bn=1, ts=0), carried(Lx[1], J_, bn=2, ts=ts_d), carried(Lx[2], J_, bn=3, ts=ts_s)], PASSC, anchor_c,
           "galeed/1 s7.4, s8.1: retirement at %d; the dispatch at %d %s" % (ts_s, ts_d, "counts" if ts_d <= ts_s else "is discarded"), [J_, Sa])


def build_service_book():
    """HARNESS-PAPERS <logs.json>: the book's two arrays, read for form."""
    v = addr("svc_book")
    good = SVC(verifier=v)
    SVCV = {"serves": [addr("judge")], "verdict": "SERVICE"}

    def B(slug, decls, accs, note, pred):
        S("service-of", slug, LOGS([pub(v, 5, 0, good)], declarations=decls, accessions=accs), [v], note, pred)
    B("book_arrays_read_for_form", [{"emitter": v, "blockNumber": 6, "logIndex": 0, "to": addr("x1")}],
      [{"emitter": addr("x1"), "blockNumber": 6, "logIndex": 1, "holder": v, "root": v}],
      "s7a.3 reads the publications alone: a move of the verifier's key on the book changes no answer of serviceOf", SVCV)
    B("misuse_declaration_to_bad", [{"emitter": v, "blockNumber": 6, "logIndex": 0, "to": "0x12"}], [], "HARNESS: a declaration outside its form", None)
    B("misuse_accession_no_root", [], [{"emitter": v, "blockNumber": 6, "logIndex": 0, "holder": v}], "HARNESS: an accession without root", None)
    B("misuse_book_shares_position", [{"emitter": v, "blockNumber": 5, "logIndex": 0, "to": addr("x1")}], [], "HARNESS: a declaration sharing a publication's position", None)
    S("service-of", "misuse_no_declarations", LOGS([], root={"publications": [], "statuses": [], "accessions": []}), [v], "HARNESS: declarations absent", None)
    S("service-of", "misuse_no_accessions", LOGS([], root={"publications": [], "statuses": [], "declarations": []}), [v], "HARNESS: accessions absent", None)
    B("misuse_declaration_not_object", [1], [], "HARNESS: a declaration element that is not an object", None)
    B("misuse_accession_shares_publication", [], [{"emitter": addr("x1"), "blockNumber": 5, "logIndex": 0, "holder": v, "root": v}], "HARNESS: an accession sharing a publication's position", None)
    B("misuse_declaration_shares_accession", [{"emitter": v, "blockNumber": 6, "logIndex": 0, "to": addr("x1")}], [{"emitter": addr("x1"), "blockNumber": 6, "logIndex": 0, "holder": v, "root": v}], "HARNESS: a declaration and an accession sharing a position", None)
    B("misuse_accession_emitter_bad", [], [{"emitter": "0x12", "blockNumber": 6, "logIndex": 0, "holder": v, "root": v}], "HARNESS: an accession whose emitter is outside hex20", None)
    S("service-of", "misuse_two_array_form", LOGS([], root={"publications": [], "statuses": []}), [v], "HARNESS: the form without the book's arrays", None)


# --------------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=1)
    a = ap.parse_args()
    RNG.seed(a.seed)
    for cat in ("docs", "sign", "pair", "qr", "door", "service"):
        shutil.rmtree(os.path.join(HERE, cat), ignore_errors=True)
        os.makedirs(os.path.join(HERE, cat))
    nb, pb, rb, fb, proto = build_docs()
    build_sign(nb, pb, proto)
    build_pair(nb, pb, rb, fb, proto)
    build_qr(nb, pb)
    build_door(nb, pb)
    build_current(nb, pb, rb, fb, proto)
    build_service()
    build_service_book()
    with open(os.path.join(HERE, "manifest.json"), "w") as f:
        json.dump({"seed": a.seed, "cases": MANIFEST}, f, indent=1, ensure_ascii=False)
    print("cases:", len(MANIFEST), dict(_n))
    census()


def census():
    """s13.1: every literal s11 of the law spells, witnessed by some prediction.
    PAYLOAD_OK is spelled on no interface (HARNESS fixes qr-decode's success
    as ok:true) and is excepted by name."""
    import re as _re
    d = HERE
    law = None
    while True:
        cand = os.path.join(d, "docs", "galeed-papers-v1.md")
        if os.path.exists(cand):
            law = open(cand, encoding="utf-8").read()
            break
        up = os.path.dirname(d)
        if up == d:
            break
        d = up
    if law is None:
        print("s11 literals: the law is not on any ancestor path of this corpus, census skipped")
        return
    sec = law[law.index("## 11. Tokens and vocabularies"):law.index("## 12. Outside the grammar")]
    literals = set(_re.findall(r"`([A-Z][A-Z0-9_]+)`", sec)) - {"PAYLOAD_OK"}
    seen = set()
    def walk(v):
        if isinstance(v, dict):
            for x in v.values(): walk(x)
        elif isinstance(v, list):
            for x in v: walk(x)
        elif isinstance(v, str):
            seen.add(v)
    for c in MANIFEST:
        walk(c.get("predicted"))
    missing = sorted(literals - seen)
    print("s11 literals:", len(literals), "unwitnessed:", missing if missing else "none")


if __name__ == "__main__":
    main()
