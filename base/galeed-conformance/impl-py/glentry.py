"""galeed/1 sections 4, 5 and 6: envelope, signing, bodies."""

from glcanon import Fault, Obj, accept, canon
from glcrypto import (N, HALF_N, address_of, eip191_digest, message, recover,
                      sha256)

SEVEN = ("body", "entryType", "judge", "prev", "seq", "sig", "spec")
SEVEN_SET = frozenset(SEVEN)

DOMAIN_ENTRY = "galeed/1"
KNOWN_TYPES = frozenset(("genesis", "amanah", "judgment", "resignation", "annotation",
                         "succession", "migration", "keybinding"))
CATEGORIES = frozenset(("unlawful-subject", "other"))

_HEXL = frozenset("0123456789abcdef")


# --------------------------------------------------------------------------
# s1 forms
# --------------------------------------------------------------------------

def is_hex(v, nbytes):
    if not isinstance(v, str):
        return False
    if len(v) != 2 + 2 * nbytes:
        return False
    if v[0] != "0" or v[1] != "x":
        return False
    for c in v[2:]:
        if c not in _HEXL:
            return False
    return True


def is_token(v):
    if not isinstance(v, str) or len(v) == 0:
        return False
    for ch in v:
        o = ord(ch)
        if o < 0x21 or o > 0x7E:
            return False
    return True


def is_skeleton_nonempty(v):
    if not isinstance(v, str) or len(v) == 0:
        return False
    for ch in v:
        o = ord(ch)
        if o < 0x20 or o > 0x7E:
            return False
    return True


def is_int(v):
    return type(v) is int


def is_bool(v):
    return v is True or v is False


def is_prose(v):
    return isinstance(v, str)


def hexb(v):
    """The bytes of a hexN string."""
    return bytes.fromhex(v[2:])


# --------------------------------------------------------------------------
# s6 body tables
# --------------------------------------------------------------------------

def _bad():
    raise Fault("E_BODY_FIELD")


def _check_subject_ref(v):
    """s6.3: an object with a token `kind`; a known kind forces its table."""
    if not isinstance(v, Obj):
        _bad()
    d = v.dict()
    if "kind" not in d or not is_token(d["kind"]):
        _bad()
    kind = d["kind"]
    if kind == "evm":
        if "chainId" not in d or not is_int(d["chainId"]):
            _bad()
        if "contract" not in d or not is_hex(d["contract"], 20):
            _bad()
        if "codeHash" not in d or not is_hex(d["codeHash"], 32):
            _bad()
        if "id" not in d or not is_hex(d["id"], 32):
            _bad()
    elif kind == "hash":
        if "sha256" not in d or not is_hex(d["sha256"], 32):
            _bad()
        if "note_md" in d and not is_prose(d["note_md"]):
            _bad()


def _check_body(etype, body):
    d = body.dict()

    def req(k, test):
        if k not in d or not test(d[k]):
            _bad()

    def opt(k, test):
        if k in d and not test(d[k]):
            _bad()

    if etype == "genesis":
        req("statement_md", is_prose)
    elif etype == "amanah":
        req("version", is_int)
        req("text_md", is_prose)
        if "venues" in d:
            vs = d["venues"]
            if not isinstance(vs, list):
                _bad()
            for el in vs:
                if not isinstance(el, Obj):
                    _bad()
                ed = el.dict()
                if "chainId" not in ed or not is_int(ed["chainId"]):
                    _bad()
                if "contract" not in ed or not is_hex(ed["contract"], 20):
                    _bad()
                if "codeHash" not in ed or not is_hex(ed["codeHash"], 32):
                    _bad()
        opt("quotes", lambda v: isinstance(v, Obj))
    elif etype == "judgment":
        if "subjectRef" not in d:
            _bad()
        _check_subject_ref(d["subjectRef"])
        req("amanah", lambda v: is_hex(v, 32))
        req("verdict_md", is_prose)
        opt("decision", lambda v: isinstance(v, Obj))
        if "evidence" in d:
            ev = d["evidence"]
            if not isinstance(ev, list):
                _bad()
            for el in ev:
                if not isinstance(el, Obj):
                    _bad()
                ed = el.dict()
                if "manifest" not in ed or not is_hex(ed["manifest"], 32):
                    _bad()
                if "produced" not in ed or not is_bool(ed["produced"]):
                    _bad()
        if "txRefs" in d:
            tr = d["txRefs"]
            if not isinstance(tr, list):
                _bad()
            for x in tr:
                if not is_hex(x, 32):
                    _bad()
        opt("terms", lambda v: is_hex(v, 32))
    elif etype == "resignation":
        if "subjectRef" not in d:
            _bad()
        _check_subject_ref(d["subjectRef"])
        req("amanah", lambda v: is_hex(v, 32))
        req("category", lambda v: isinstance(v, str) and v in CATEGORIES)
        if (d["category"] == "unlawful-subject") != ("clause" in d):
            _bad()
        opt("clause", is_skeleton_nonempty)
        req("statement_md", is_prose)
    elif etype == "annotation":
        opt("subject", lambda v: is_hex(v, 32))
        req("note_md", is_prose)
    elif etype == "succession":
        req("to", lambda v: is_hex(v, 20))
        req("kind", is_token)
        req("effective", is_int)
        req("statement_md", is_prose)
    elif etype == "migration":
        req("spec", is_token)
        req("statement_md", is_prose)
    elif etype == "keybinding":
        req("recipient", is_token)
        req("effective", is_int)
    # s6.8: an unknown type has only the s6.9 test, already met by step 9.


# --------------------------------------------------------------------------
# s5 signature
# --------------------------------------------------------------------------

def split_sig(sighex):
    raw = hexb(sighex)
    r = int.from_bytes(raw[0:32], "big")
    s = int.from_bytes(raw[32:64], "big")
    v = raw[64]
    return r, s, v


def check_sig(digest, sighex, expected_addr_hex):
    """s5.4 and s5.5.  Raises Fault."""
    r, s, v = split_sig(sighex)
    if v != 27 and v != 28:
        raise Fault("E_SIG_V")
    if not (1 <= r <= N - 1) or not (1 <= s <= N - 1):
        raise Fault("E_SIG_RANGE")
    if s > HALF_N:
        raise Fault("E_SIG_HIGH_S")
    pt = recover(digest, r, s, v - 27)
    if pt is None:
        raise Fault("E_SIG_RECOVER")
    if "0x" + address_of(pt).hex() != expected_addr_hex:
        raise Fault("E_SIG_SIGNER")
    return True


def entry_digest(b6):
    return eip191_digest(message(DOMAIN_ENTRY, sha256(b6)))


# --------------------------------------------------------------------------
# s4.3 the thirteen steps
# --------------------------------------------------------------------------

class Entry:
    __slots__ = ("raw", "value", "eid", "eidb", "etype", "judge", "seq",
                 "prev", "body")

    def __init__(self, raw, value, eid, etype, judge, seq, prev, body):
        self.raw = raw
        self.value = value
        self.eid = eid
        self.eidb = bytes.fromhex(eid[2:])
        self.etype = etype
        self.judge = judge
        self.seq = seq
        self.prev = prev
        self.body = body


def validate(b):
    """Decide s4.3 over the byte string `b`.  Raises Fault, or returns Entry."""
    v = accept(b)                                                   # 1
    if not isinstance(v, Obj):                                      # 2
        raise Fault("E_ENVELOPE")
    d = v.dict()
    for k in SEVEN:                                                 # 3
        if k not in d:
            raise Fault("E_ENVELOPE_MISSING")
    for k in d:
        if k not in SEVEN_SET:
            raise Fault("E_ENVELOPE_CLOSED")
    if not (isinstance(d["spec"], str) and d["spec"] == "galeed/1"):  # 4
        raise Fault("E_SPEC")
    if not is_token(d["entryType"]):                                # 5
        raise Fault("E_ENTRYTYPE")
    if not is_hex(d["judge"], 20):                                  # 6
        raise Fault("E_JUDGE")
    if not is_int(d["seq"]):                                        # 7
        raise Fault("E_SEQ")
    prev = d["prev"]
    if not (prev is None or is_hex(prev, 32)):                      # 8
        raise Fault("E_PREV")
    if (prev is None) != (d["seq"] == 0):
        raise Fault("E_PREV_SEQ")
    if not isinstance(d["body"], Obj):                              # 9
        raise Fault("E_BODY")
    if not is_hex(d["sig"], 65):                                    # 10
        raise Fault("E_SIG_FORM")
    if (d["entryType"] == "genesis") != (d["seq"] == 0):            # 11
        raise Fault("E_GENESIS_PLACE")
    _check_body(d["entryType"], d["body"])                          # 12
    b6 = canon(Obj([(k, val) for k, val in v.pairs if k != "sig"]))  # 13
    check_sig(entry_digest(b6), d["sig"], d["judge"])
    return Entry(b, v, "0x" + sha256(b).hex(), d["entryType"], d["judge"],
                 d["seq"], prev, d["body"])
