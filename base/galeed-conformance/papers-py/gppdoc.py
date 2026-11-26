"""galeed.papers/1 sections 2 through 8: documents, signing, pairing, the service record and its reading, payload.

Built on the frozen galeed/1 core in ../impl-py (canonical form, signing
primitives); every predicate below is the law's own.
"""
import base64
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "impl-py"))

from glcanon import Fault, Obj, accept, canon          # noqa: E402
from glcrypto import eip191_digest, keccak256, message, sha256    # noqa: E402
import glaudit                                          # noqa: E402
from glentry import check_sig, is_hex, is_int, is_token, is_prose  # noqa: E402

KIND_NOTE = "galeed.note/1"
KIND_PASS = "galeed.pass/1"
KIND_REPORT = "galeed.report/1"
KIND_REFUSAL = "galeed.refusal/1"
KIND_SERVICE = "galeed.verifier/1"
DOMAINS = (KIND_NOTE, KIND_PASS, KIND_REPORT, KIND_REFUSAL)
PAYLOAD_CAP = 2953
PAYLOAD_PREFIX = b"galeed-pass:"


class DocFault(Exception):
    def __init__(self, token, index=None):
        Exception.__init__(self, token)
        self.token = token
        self.index = index


def _members(d, keys):
    for k in keys:
        if k not in d:
            raise DocFault("E_DOC_MISSING")
    for k in d:
        if k not in keys:
            raise DocFault("E_DOC_CLOSED")


def _kind(d, lit):
    v = d["kind"]
    if not (isinstance(v, str) and v == lit):
        raise DocFault("E_KIND")


def _signature(v, d, domain, signer_key):
    if not is_hex(d["sig"], 65):
        raise DocFault("E_SIG_FORM")
    b = canon(Obj([(k, val) for k, val in v.pairs if k != "sig"]))
    digest = eip191_digest(message(domain, sha256(b)))
    try:
        check_sig(digest, d["sig"], d[signer_key])
    except Fault as f:
        raise DocFault(f.token)


def _open(b):
    try:
        v = accept(b)
    except Fault as f:
        raise DocFault(f.token)
    if not isinstance(v, Obj):
        raise DocFault("E_DOC")
    return v, v.dict()


def _case(v):
    if not isinstance(v, Obj):
        return False
    d = v.dict()
    if "kind" not in d or not is_token(d["kind"]):
        return False
    kind = d["kind"]
    if kind == "evm":
        return ("chainId" in d and is_int(d["chainId"])
                and "contract" in d and is_hex(d["contract"], 20)
                and "codeHash" in d and is_hex(d["codeHash"], 32)
                and "id" in d and is_hex(d["id"], 32))
    if kind == "hash":
        if "sha256" not in d or not is_hex(d["sha256"], 32):
            return False
        if "note_md" in d and not is_prose(d["note_md"]):
            return False
    return True


NOTE_KEYS = ("kind", "judge", "case", "verifier", "protocol", "scope_md", "issued", "deadline", "sig")
PASS_KEYS = ("kind", "verifier", "noteHash", "window", "orderRef", "sig")
REPORT_KEYS = ("kind", "verifier", "noteHash", "protocol", "manifest", "at", "note_md", "sig")
REFUSAL_KEYS = ("kind", "verifier", "noteHash", "passHash", "at", "place_md", "orderRef", "photos", "sig")
SERVICE_KEYS = ("kind", "verifier", "serves", "at")


def check_note(b):
    """s4.2.  Returns the parsed dict; raises DocFault."""
    v, d = _open(b)
    _members(d, NOTE_KEYS)
    _kind(d, KIND_NOTE)
    if not is_hex(d["judge"], 20):
        raise DocFault("E_NOTE_JUDGE")
    if not _case(d["case"]):
        raise DocFault("E_NOTE_CASE")
    if not is_hex(d["verifier"], 20):
        raise DocFault("E_NOTE_VERIFIER")
    if not is_hex(d["protocol"], 32):
        raise DocFault("E_NOTE_PROTOCOL")
    if not isinstance(d["scope_md"], str):
        raise DocFault("E_NOTE_SCOPE")
    if not is_int(d["issued"]):
        raise DocFault("E_NOTE_ISSUED")
    if not is_int(d["deadline"]):
        raise DocFault("E_NOTE_DEADLINE")
    if not d["issued"] <= d["deadline"]:
        raise DocFault("E_NOTE_WINDOW")
    _signature(v, d, KIND_NOTE, "judge")
    return d


def check_pass(b):
    """s5.2."""
    v, d = _open(b)
    _members(d, PASS_KEYS)
    _kind(d, KIND_PASS)
    if not is_hex(d["verifier"], 20):
        raise DocFault("E_PASS_VERIFIER")
    if not is_hex(d["noteHash"], 32):
        raise DocFault("E_PASS_NOTE")
    w = d["window"]
    if not isinstance(w, Obj):
        raise DocFault("E_PASS_WINDOW")
    wd = w.dict()
    if set(wd.keys()) != {"from", "to"} or not is_int(wd["from"]) or not is_int(wd["to"]):
        raise DocFault("E_PASS_WINDOW")
    if not wd["from"] <= wd["to"]:
        raise DocFault("E_PASS_WINDOW_ORDER")
    if not isinstance(d["orderRef"], str):
        raise DocFault("E_PASS_ORDER")
    _signature(v, d, KIND_PASS, "verifier")
    d["window"] = wd
    return d


def check_report(b):
    """s6.2."""
    v, d = _open(b)
    _members(d, REPORT_KEYS)
    _kind(d, KIND_REPORT)
    if not is_hex(d["verifier"], 20):
        raise DocFault("E_REPORT_VERIFIER")
    if not is_hex(d["noteHash"], 32):
        raise DocFault("E_REPORT_NOTE")
    if not is_hex(d["protocol"], 32):
        raise DocFault("E_REPORT_PROTOCOL")
    if not is_hex(d["manifest"], 32):
        raise DocFault("E_REPORT_MANIFEST")
    if not is_int(d["at"]):
        raise DocFault("E_REPORT_AT")
    if not isinstance(d["note_md"], str):
        raise DocFault("E_REPORT_TEXT")
    _signature(v, d, KIND_REPORT, "verifier")
    return d


def check_refusal(b):
    """s7.2."""
    v, d = _open(b)
    _members(d, REFUSAL_KEYS)
    _kind(d, KIND_REFUSAL)
    if not is_hex(d["verifier"], 20):
        raise DocFault("E_REFUSAL_VERIFIER")
    if not is_hex(d["noteHash"], 32):
        raise DocFault("E_REFUSAL_NOTE")
    if not is_hex(d["passHash"], 32):
        raise DocFault("E_REFUSAL_PASS")
    if not is_int(d["at"]):
        raise DocFault("E_REFUSAL_AT")
    if not isinstance(d["place_md"], str):
        raise DocFault("E_REFUSAL_PLACE")
    if not isinstance(d["orderRef"], str):
        raise DocFault("E_REFUSAL_ORDER")
    ph = d["photos"]
    if not isinstance(ph, list):
        raise DocFault("E_REFUSAL_PHOTOS")
    for i, x in enumerate(ph):
        if not is_hex(x, 32):
            raise DocFault("E_REFUSAL_PHOTO", i)
    seen = set()
    for i, x in enumerate(ph):
        if x in seen:
            raise DocFault("E_REFUSAL_DUP_PHOTO", i)
        seen.add(x)
    for i in range(1, len(ph)):
        if ph[i].encode("utf-8") < ph[i - 1].encode("utf-8"):
            raise DocFault("E_REFUSAL_PHOTO_ORDER", i)
    _signature(v, d, KIND_REFUSAL, "verifier")
    return d


def check_service(b):
    """s7a.2.  No signature is read; s3 does not run."""
    v, d = _open(b)
    _members(d, SERVICE_KEYS)
    _kind(d, KIND_SERVICE)
    if not is_hex(d["verifier"], 20):
        raise DocFault("E_SERVICE_VERIFIER")
    sv = d["serves"]
    if not isinstance(sv, list):
        raise DocFault("E_SERVICE_SERVES")
    for i, x in enumerate(sv):
        if not is_hex(x, 20):
            raise DocFault("E_SERVICE_ADDRESS", i)
    seen = set()
    for i, x in enumerate(sv):
        if x in seen:
            raise DocFault("E_SERVICE_DUP", i)
        seen.add(x)
    for i in range(1, len(sv)):
        if sv[i].encode("utf-8") < sv[i - 1].encode("utf-8"):
            raise DocFault("E_SERVICE_ORDER", i)
    if not is_int(d["at"]):
        raise DocFault("E_SERVICE_AT")
    return d


# s7a.3 reading a verifier's record -----------------------------------------

KIND_FAMILY = "galeed.verifier/"


def _publication_bytes(pub):
    """galeed.roster/1 s2.1 steps 1 through 3 over one publication (a dict with
    `document` bytes and `word` hex32): (document bytes, None), or (None, token)."""
    doc = pub["document"]
    if "0x" + keccak256(doc).hex() != pub["word"]:
        return None, "E_HASH"
    return doc, None


def _kind_string(doc):
    """galeed.roster/1 s1: the decoded `kind` where s2.1 steps 2 and 3 pass,
    else None (a publication a reader cannot place is of every kind)."""
    try:
        v = glaudit.load_input(doc)
    except glaudit.NoLabel:
        return None
    if not isinstance(v, Obj):
        return None
    d = v.dict()
    k = d.get("kind")
    if "kind" not in d or not isinstance(k, str):
        return None
    return k


def _of_this_kind(pub):
    """galeed.roster/1 s1: of kind verifier, that is, steps 1 to 3 fail or the
    kind begins with the reserved prefix."""
    doc, tok = _publication_bytes(pub)
    if tok is not None:
        return True
    k = _kind_string(doc)
    return k is None or k.startswith(KIND_FAMILY)


def service_of(verifier, publications):
    """s7a.3.  `publications`: dicts carrying emitter, blockNumber, logIndex,
    document (bytes), word (hex32), the logs set as galeed.roster/1 s1 reads it.
    Returns ("SERVICE", serves) | ("SERVICE_NONE", None) |
    ("SERVICE_UNREAD", DocFault) | ("SERVICE_MISMATCH", None)."""
    ps = [p for p in publications if p["emitter"] == verifier]
    ps.sort(key=lambda p: (p["blockNumber"], p["logIndex"]))
    cand = None
    for p in ps:
        if _of_this_kind(p):
            cand = p
    if cand is None:
        return ("SERVICE_NONE", None)
    doc, tok = _publication_bytes(cand)
    if tok is not None:
        return ("SERVICE_UNREAD", DocFault(tok))
    v = _kind_string(doc)
    if v is None:
        # galeed.roster/1 s2.1 steps 2 and 3: the reader's refusal, a root that
        # is not an object, or a kind that is absent or not a string
        try:
            x = glaudit.load_input(doc)
        except glaudit.NoLabel:
            return ("SERVICE_UNREAD", DocFault("E_JSON"))
        if not isinstance(x, Obj):
            return ("SERVICE_UNREAD", DocFault("E_JSON"))
        return ("SERVICE_UNREAD", DocFault("E_KIND"))
    try:
        d = check_service(doc)
    except DocFault as f:
        return ("SERVICE_UNREAD", f)
    if d["verifier"] != verifier:
        return ("SERVICE_MISMATCH", None)
    return ("SERVICE", d["serves"])


def serving(verifier, seat, publications):
    """s7a.3: true iff serviceOf is SERVICE and `seat` is among its serves."""
    r, sv = service_of(verifier, publications)
    return r == "SERVICE" and seat in sv


def doc_id(b):
    return "0x" + sha256(b).hex()


# s6.4 and s7.4 pairing ----------------------------------------------------

def pair(note_bytes, report_bytes, current):
    """Returns (verdict, token-or-None); `current` is a hex20 or 'unresolved'."""
    try:
        n = check_note(note_bytes)
    except DocFault as f:
        return ("NOTE_INVALID", f.token)
    try:
        r = check_report(report_bytes)
    except DocFault as f:
        return ("REPORT_INVALID", f.token)
    if r["noteHash"] != doc_id(note_bytes):
        return ("REPORT_NOTE_MISMATCH", None)
    if current == "unresolved":
        return ("REPORT_VERIFIER_UNRESOLVED", None)
    if r["verifier"] != current:
        return ("REPORT_VERIFIER_MISMATCH", None)
    if r["protocol"] != n["protocol"]:
        return ("REPORT_PROTOCOL_MISMATCH", None)
    return ("PAIRED", None)


def pair_refusal(note_bytes, pass_bytes, refusal_bytes, current):
    try:
        n = check_note(note_bytes)
    except DocFault as f:
        return ("NOTE_INVALID", f.token)
    try:
        check_pass(pass_bytes)
    except DocFault as f:
        return ("PASS_INVALID", f.token)
    try:
        r = check_refusal(refusal_bytes)
    except DocFault as f:
        return ("REFUSAL_INVALID", f.token, f.index)
    if r["noteHash"] != doc_id(note_bytes):
        return ("REFUSAL_NOTE_MISMATCH", None)
    if r["passHash"] != doc_id(pass_bytes):
        return ("REFUSAL_PASS_MISMATCH", None)
    if current == "unresolved":
        return ("REFUSAL_VERIFIER_UNRESOLVED", None)
    if r["verifier"] != current:
        return ("REFUSAL_VERIFIER_MISMATCH", None)
    return ("REFUSAL_PAIRED", None)


# s8 payload ----------------------------------------------------------------

_B64 = frozenset(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_")


def _b64e(b):
    return base64.urlsafe_b64encode(b).rstrip(b"=")


def _b64d(seg):
    """Canonical base64url without padding, or None."""
    if len(seg) == 0 or len(seg) % 4 == 1:
        return None
    for c in seg:
        if c not in _B64:
            return None
    padded = seg + b"=" * (-len(seg) % 4)
    raw = base64.urlsafe_b64decode(padded)
    if _b64e(raw) != seg:
        return None
    return raw


class PayloadFault(Exception):
    def __init__(self, token, inner=None, index=None, count=None):
        Exception.__init__(self, token)
        self.token = token
        self.inner = inner
        self.index = index
        self.count = count


def payload_encode(pass_bytes, note_bytes):
    try:
        check_pass(pass_bytes)
    except DocFault as f:
        raise PayloadFault("E_PAYLOAD_PASS", inner=f.token)
    try:
        check_note(note_bytes)
    except DocFault as f:
        raise PayloadFault("E_PAYLOAD_NOTE", inner=f.token)
    payload = PAYLOAD_PREFIX + _b64e(pass_bytes) + b"." + _b64e(note_bytes)
    if len(payload) > PAYLOAD_CAP:
        raise PayloadFault("E_PAYLOAD_CAP")
    return payload


def payload_decode(t):
    if not t.startswith(PAYLOAD_PREFIX):
        raise PayloadFault("E_PAYLOAD_PREFIX")
    if len(t) > PAYLOAD_CAP:
        raise PayloadFault("E_PAYLOAD_CAP")
    rest = t[len(PAYLOAD_PREFIX):]
    dots = rest.count(b".")
    if dots != 1:
        raise PayloadFault("E_PAYLOAD_SEGMENTS", count=dots)
    segs = rest.split(b".")
    out = []
    for k, seg in enumerate(segs):
        raw = _b64d(seg)
        if raw is None:
            raise PayloadFault("E_PAYLOAD_B64", index=k)
        try:
            if k == 0:
                check_pass(raw)
            else:
                check_note(raw)
        except DocFault as f:
            raise PayloadFault("E_PAYLOAD_PASS" if k == 0 else "E_PAYLOAD_NOTE", inner=f.token)
        out.append(raw)
    return out[0], out[1]
