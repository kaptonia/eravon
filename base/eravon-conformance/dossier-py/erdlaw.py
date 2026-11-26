"""eravon.dossier/1 sections 2 and 3: the index and the files.

Built on the eravon.link/1 core in ../link-py and, through it, the
eravon.terms/1 core, and on erdentry, this core's own implementation of
galeed/1 s4.3; every predicate below is the law's own.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "..", "link-py"))
sys.path.insert(0, os.path.join(HERE, "..", "terms-py"))

from ertcanon import Fault, Obj, accept, is_hex, is_int  # noqa: E402
from ertcrypto import sha256                             # noqa: E402
from erdentry import validate                            # noqa: E402
import ertlaw as T                                  # noqa: E402
from ertlaw import DocFault, hx                     # noqa: E402
import erllaw as L                                  # noqa: E402
from erllaw import Verdict                          # noqa: E402

V_DOSSIER = "eravon.dossier/1"
INDEX_NAME = "dossier.json"
KINDS = frozenset(("post", "fund", "deploy", "init", "accept", "delivery", "confirm", "dispute", "commit",
                   "settle", "rule", "resign", "refund", "expire", "cancel", "revoke", "advance", "redraw",
                   "setPayout", "withdraw", "sweep", "consent", "fundBind"))
TERMINALS = frozenset(("Refunded", "Released", "Settled", "Ruled", "Resigned", "Lapsed"))
REQUIRED = ("v", "chainId", "instance", "caseId", "logic", "proxyCodeHash", "posted", "files", "seat", "transactions")
ALL = REQUIRED + ("terminal", "compliance")
FILE_KEYS = ("listing", "receipt", "entry")


def is_file_name(v):
    return (isinstance(v, str) and len(v) > 0 and "/" not in v and v != "." and v != ".."
            and all(0x20 <= ord(ch) <= 0x7e for ch in v))


def _file_ref(v):
    if not isinstance(v, Obj):
        return None
    d = v.dict()
    if set(d.keys()) != {"name", "sha256"}:
        return None
    if not is_file_name(d["name"]) or not is_hex(d["sha256"], 32):
        return None
    return d


def _check_posted(v):
    if not isinstance(v, Obj):
        raise DocFault("E_DOSSIER_POSTED")
    d = v.dict()
    if set(d.keys()) != {"tx", "hash"} or not is_hex(d["tx"], 32) or not is_hex(d["hash"], 32):
        raise DocFault("E_DOSSIER_POSTED")
    return d


def _check_files(v):
    """s2.1's files object: returns (refs by key, attachment refs in order)."""
    if not isinstance(v, Obj):
        raise DocFault("E_DOSSIER_FILES")
    d = v.dict()
    for k in ("listing", "receipt"):
        if k not in d:
            raise DocFault("E_DOSSIER_FILES")
    for k in d:
        if k not in FILE_KEYS + ("attachments",):
            raise DocFault("E_DOSSIER_FILES")
    out = {}
    for k in FILE_KEYS:
        if k in d:
            r = _file_ref(d[k])
            if r is None:
                raise DocFault("E_DOSSIER_FILES")
            out[k] = r
    atts = []
    if "attachments" in d:
        if not isinstance(d["attachments"], list):
            raise DocFault("E_DOSSIER_FILES")
        for el in d["attachments"]:
            r = _file_ref(el)
            if r is None:
                raise DocFault("E_DOSSIER_FILES")
            atts.append(r)
    return out, atts


def _check_seat(v):
    if not isinstance(v, Obj):
        raise DocFault("E_DOSSIER_SEAT")
    d = v.dict()
    keys = set(d.keys())
    if not {"amanahHash", "quoteHash", "seat"} <= keys or keys - {"judge", "amanahHash", "quoteHash", "seat", "entryId"}:
        raise DocFault("E_DOSSIER_SEAT")
    if "judge" in d and not T.is_nonzero_hex20(d["judge"]):
        raise DocFault("E_DOSSIER_SEAT")
    if not T.is_nonzero_hex32(d["amanahHash"]) or not is_hex(d["quoteHash"], 32):
        raise DocFault("E_DOSSIER_SEAT")
    if not is_int(d["seat"]) or d["seat"] > 2:
        raise DocFault("E_DOSSIER_SEAT")
    if "entryId" in d and not is_hex(d["entryId"], 32):
        raise DocFault("E_DOSSIER_SEAT")
    if "judge" not in d and (d["seat"] != 0 or "entryId" in d):
        raise DocFault("E_DOSSIER_SEAT")
    return d


def _check_transaction(el, i):
    if not isinstance(el, Obj):
        raise DocFault("E_TRANSACTION", i)
    d = el.dict()
    keys = set(d.keys())
    if not {"kind", "tx", "block", "timestamp"} <= keys or keys - {"kind", "tx", "block", "timestamp", "note_md"}:
        raise DocFault("E_TRANSACTION", i)
    if not (isinstance(d["kind"], str) and d["kind"] in KINDS):
        raise DocFault("E_TRANSACTION", i)
    if not is_hex(d["tx"], 32) or not is_int(d["block"]) or not is_int(d["timestamp"]):
        raise DocFault("E_TRANSACTION", i)
    if "note_md" in d and not isinstance(d["note_md"], str):
        raise DocFault("E_TRANSACTION", i)
    return d


def _check_terminal(v):
    if not isinstance(v, Obj):
        raise DocFault("E_DOSSIER_TERMINAL")
    d = v.dict()
    if set(d.keys()) != {"name", "buyerPaid", "sellerPaid", "judgePaid", "bondToBuyer"}:
        raise DocFault("E_DOSSIER_TERMINAL")
    if not (isinstance(d["name"], str) and d["name"] in TERMINALS):
        raise DocFault("E_DOSSIER_TERMINAL")
    for k in ("buyerPaid", "sellerPaid", "judgePaid", "bondToBuyer"):
        if not T.is_dec256(d[k]):
            raise DocFault("E_DOSSIER_TERMINAL")
    return d


def _check_compliance(v):
    if not isinstance(v, Obj):
        raise DocFault("E_DOSSIER_COMPLIANCE")
    d = v.dict()
    if set(d.keys()) - {"buyer_md", "seller_md", "serial", "conversions"}:
        raise DocFault("E_DOSSIER_COMPLIANCE")
    for k in ("buyer_md", "seller_md"):
        if k in d and not isinstance(d[k], str):
            raise DocFault("E_DOSSIER_COMPLIANCE")
    if "serial" in d and not L.is_skeleton_nonempty(d["serial"]):
        raise DocFault("E_DOSSIER_COMPLIANCE")
    if "conversions" in d:
        cs = d["conversions"]
        if not isinstance(cs, list):
            raise DocFault("E_DOSSIER_COMPLIANCE")
        for i, el in enumerate(cs):
            if not isinstance(el, Obj):
                raise DocFault("E_DOSSIER_COMPLIANCE", i)
            ed = el.dict()
            if set(ed.keys()) != {"tx", "at", "unit", "rate", "source"}:
                raise DocFault("E_DOSSIER_COMPLIANCE", i)
            if not is_hex(ed["tx"], 32) or not is_int(ed["at"]):
                raise DocFault("E_DOSSIER_COMPLIANCE", i)
            for k in ("unit", "rate", "source"):
                if not L.is_skeleton_nonempty(ed[k]):
                    raise DocFault("E_DOSSIER_COMPLIANCE", i)
    return d


class Index:
    __slots__ = ("d", "posted", "files", "attachments", "seat", "terminal")


def check_index(b):
    """s2.6.  Returns an Index; raises DocFault."""
    try:
        v = accept(b)
    except Fault as f:
        raise DocFault(f.token)
    if not isinstance(v, Obj):
        raise DocFault("E_DOSSIER")
    d = v.dict()
    T._members(d, REQUIRED, ALL, "E_DOSSIER_MISSING", "E_DOSSIER_CLOSED")
    if not (isinstance(d["v"], str) and d["v"] == V_DOSSIER):
        raise DocFault("E_DOSSIER_V")
    if not is_int(d["chainId"]):
        raise DocFault("E_DOSSIER_CHAIN")
    if not is_hex(d["instance"], 20):
        raise DocFault("E_DOSSIER_INSTANCE")
    if not is_hex(d["caseId"], 32):
        raise DocFault("E_DOSSIER_CASE")
    if not is_hex(d["logic"], 20):
        raise DocFault("E_DOSSIER_LOGIC")
    if not is_hex(d["proxyCodeHash"], 32):
        raise DocFault("E_DOSSIER_CODEHASH")
    posted = _check_posted(d["posted"])
    files, atts = _check_files(d["files"])
    seat = _check_seat(d["seat"])
    if not isinstance(d["transactions"], list):
        raise DocFault("E_DOSSIER_TRANSACTIONS")
    for i, el in enumerate(d["transactions"]):
        _check_transaction(el, i)
    terminal = None
    if "terminal" in d:
        terminal = _check_terminal(d["terminal"])
        if terminal["name"] == "Ruled" and ("entryId" not in seat or "judge" not in seat):
            raise DocFault("E_DOSSIER_RULED")
    if "compliance" in d:
        _check_compliance(d["compliance"])
    ix = Index()
    ix.d = d
    ix.posted = posted
    ix.files = files
    ix.attachments = atts
    ix.seat = seat
    ix.terminal = terminal
    return ix


def check_dossier(directory, read):
    """s3.1 over a directory; `read(name)` returns the bytes of a named file
    or None.  Returns the answer; raises DocFault."""
    ib = read(INDEX_NAME)
    if ib is None:
        raise DocFault("E_DOSSIER_FILE", key="index")
    ix = check_index(ib)
    data = {}
    refs = [(k, ix.files[k]) for k in FILE_KEYS if k in ix.files]
    refs += [("attachment:%d" % i, r) for i, r in enumerate(ix.attachments)]
    for k, ref in refs:
        fb = read(ref["name"])
        if fb is None:
            raise DocFault("E_DOSSIER_FILE", key=k)
        if hx(sha256(fb)) != ref["sha256"]:
            raise DocFault("E_DOSSIER_DIGEST", key=k)
        data[k] = fb
    try:
        s = L.check_listing(data["listing"])
    except DocFault as f:
        raise DocFault("E_DOSSIER_LISTING", inner=f)
    if hx(sha256(data["listing"])) != ix.posted["hash"]:
        raise DocFault("E_DOSSIER_HASH")
    try:
        r = L.check_receipt(data["receipt"])
    except DocFault as f:
        raise DocFault("E_DOSSIER_RECEIPT", inner=f)
    try:
        bd = L.bind_receipt_checked(s, r)
    except Verdict as v:
        raise DocFault("E_DOSSIER_ORDER", inner=v.as_fault())
    d = ix.d
    if d["chainId"] != s.d["chainId"]:
        raise DocFault("E_DOSSIER_CHAIN_MISMATCH")
    if d["logic"] != s.d["logic"]:
        raise DocFault("E_DOSSIER_LOGIC_MISMATCH")
    if d["proxyCodeHash"] != s.d["proxyCodeHash"]:
        raise DocFault("E_DOSSIER_CODEHASH_MISMATCH")
    if d["instance"] != hx(bd.instance):
        raise DocFault("E_DOSSIER_INSTANCE_MISMATCH")
    if d["caseId"] != hx(bd.salt):
        raise DocFault("E_DOSSIER_CASE_MISMATCH")
    c = bd.candidate
    seat = ix.seat
    if c < len(s.judges):
        cand = s.judges[c]
        ok = (seat.get("judge") == cand["judge"] and seat["seat"] == 0 and seat["amanahHash"] == cand["amanahHash"]
              and seat["quoteHash"] == cand["quoteHash"])
    else:
        cand = s.guilds[c - len(s.judges)]
        ok = seat["amanahHash"] == cand["amanahHash"] and seat["quoteHash"] == cand["quoteHash"]
    if not ok:
        raise DocFault("E_DOSSIER_SEAT_MISMATCH")
    if ("entryId" in seat) != ("entry" in data):
        raise DocFault("E_DOSSIER_ENTRY")
    entry_id = None
    if "entryId" in seat:
        eb = data["entry"]
        if hx(sha256(eb)) != seat["entryId"]:
            raise DocFault("E_DOSSIER_ENTRY_DIGEST")
        try:
            e = validate(eb)
        except Fault as f:
            raise DocFault("E_DOSSIER_ENTRY_FORM", inner=DocFault(f.token))
        if e.etype not in ("judgment", "resignation"):
            raise DocFault("E_DOSSIER_ENTRY_TYPE")
        body = e.body.dict()
        sr = body["subjectRef"].dict()
        seat_ok = (("seat" in sr and is_int(sr["seat"]) and sr["seat"] == seat["seat"])
                   or (e.etype == "resignation" and "seat" not in sr))
        if not (sr["kind"] == "evm" and sr["chainId"] == d["chainId"] and sr["contract"] == d["instance"]
                and sr["codeHash"] == d["proxyCodeHash"] and sr["id"] == d["caseId"] and seat_ok):
            raise DocFault("E_DOSSIER_SUBJECT")
        entry_id = seat["entryId"]
    for i, ref in enumerate(ix.attachments):
        if ref["sha256"] not in L.named_hashes(s):
            raise DocFault("E_DOSSIER_ATTACHMENT", i)
    seller_consent = "call" if c in [i for i, _ in s.by_call] else "secp256k1"
    return Obj([("attachments", len(ix.attachments)), ("buyerConsent", bd.form), ("candidate", c),
                ("caseId", hx(bd.salt)), ("entryId", entry_id), ("instance", hx(bd.instance)), ("ok", True),
                ("sellerConsent", seller_consent), ("termsHash", s.d["terms"]["hash"])])
