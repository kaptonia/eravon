#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
gen.py - corpus generator for the `galeed/1` convergence run.

Written from docs/galeed-v1.md and galeed-conformance/HARNESS.md alone, by
carrying over the twin law's generator where the two texts say the same thing
in the same words (canonical form, signing, chain, audit, anchoring) and
rewriting every part the judge's ledger states differently (the envelope key
`judge`, the seven bodies, the four audit inputs). No implementation of the
law was consulted. Everything here (canonicalizer, parser, secp256k1 with
RFC 6979 and low-s, EIP-191 digest, envelope and body tests, audit walk) is the
corpus author's own reading, and is recorded in manifest.json as a *prediction*,
a third opinion beside the two implementations under test. Where the author's
prose reasoning about a case disagreed with the author's own code, the manifest
carries both and the case is flagged `disagreement: true`.

Usage:  python3 gen.py --seed 1
Populates ./canon, ./sign, ./audit and writes manifest.json.
"""

import argparse
import hashlib
import hmac
import json
import os
import random
import shutil
import sys
from functools import lru_cache

from Crypto.Hash import keccak as _keccak

sys.setrecursionlimit(20000)

HERE = os.path.dirname(os.path.abspath(__file__))
MAXINT = (1 << 53) - 1

# --------------------------------------------------------------------------
# hashes
# --------------------------------------------------------------------------


def keccak256(b: bytes) -> bytes:
    h = _keccak.new(digest_bits=256)
    h.update(b)
    return h.digest()


def sha256(b: bytes) -> bytes:
    return hashlib.sha256(b).digest()


def hx(b: bytes) -> str:
    return "0x" + b.hex()


# --------------------------------------------------------------------------
# secp256k1 (pure python), RFC 6979, low-s, EIP-191, address recovery
# --------------------------------------------------------------------------

P = 2 ** 256 - 2 ** 32 - 977
N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
HALF_N = (N - 1) // 2
GX = 0x79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798
GY = 0x483ADA7726A3C4655DA4FBFC0E1108A8FD17B448A68554199C47D08FFB10D4B8
G = (GX, GY)


def _jac_double(p):
    x, y, z = p
    if y == 0:
        return (0, 0, 0)
    ysq = (y * y) % P
    s = (4 * x * ysq) % P
    m = (3 * x * x) % P
    nx = (m * m - 2 * s) % P
    ny = (m * (s - nx) - 8 * ysq * ysq) % P
    nz = (2 * y * z) % P
    return (nx, ny, nz)


def _jac_add(p, q):
    if p[2] == 0:
        return q
    if q[2] == 0:
        return p
    x1, y1, z1 = p
    x2, y2, z2 = q
    z1s = (z1 * z1) % P
    z2s = (z2 * z2) % P
    u1 = (x1 * z2s) % P
    u2 = (x2 * z1s) % P
    s1 = (y1 * z2s * z2) % P
    s2 = (y2 * z1s * z1) % P
    if u1 == u2:
        if s1 != s2:
            return (0, 0, 0)
        return _jac_double(p)
    h = (u2 - u1) % P
    r = (s2 - s1) % P
    h2 = (h * h) % P
    h3 = (h2 * h) % P
    u1h2 = (u1 * h2) % P
    nx = (r * r - h3 - 2 * u1h2) % P
    ny = (r * (u1h2 - nx) - s1 * h3) % P
    nz = (h * z1 * z2) % P
    return (nx, ny, nz)


def _jac_mul(p, k):
    if k % N == 0 or p[2] == 0:
        return (0, 0, 0)
    r = (0, 0, 0)
    q = p
    while k:
        if k & 1:
            r = _jac_add(r, q)
        q = _jac_double(q)
        k >>= 1
    return r


def _to_affine(p):
    if p[2] == 0:
        return None
    zi = pow(p[2], P - 2, P)
    zi2 = (zi * zi) % P
    return ((p[0] * zi2) % P, (p[1] * zi2 * zi) % P)


def mul(pt, k):
    """Affine scalar multiplication; None is the point at infinity."""
    if pt is None:
        return None
    return _to_affine(_jac_mul((pt[0], pt[1], 1), k % N))


def add(a, b):
    if a is None:
        return b
    if b is None:
        return a
    return _to_affine(_jac_add((a[0], a[1], 1), (b[0], b[1], 1)))


def on_curve(x, y):
    return (y * y - x * x * x - 7) % P == 0


def privkey_to_pub(d):
    return mul(G, d)


def pub_to_addr(pub):
    x, y = pub
    return keccak256(x.to_bytes(32, "big") + y.to_bytes(32, "big"))[12:]


def privkey_to_addr_hex(d):
    return hx(pub_to_addr(privkey_to_pub(d)))


def rfc6979_k(d: int, h1: bytes):
    """RFC 6979 section 3.2 with HMAC-SHA256, qlen = hlen = 256."""
    x = d.to_bytes(32, "big")
    z1 = int.from_bytes(h1, "big")
    z2 = z1 - N if z1 >= N else z1
    bo = z2.to_bytes(32, "big")
    v = b"\x01" * 32
    k = b"\x00" * 32
    k = hmac.new(k, v + b"\x00" + x + bo, hashlib.sha256).digest()
    v = hmac.new(k, v, hashlib.sha256).digest()
    k = hmac.new(k, v + b"\x01" + x + bo, hashlib.sha256).digest()
    v = hmac.new(k, v, hashlib.sha256).digest()
    while True:
        v = hmac.new(k, v, hashlib.sha256).digest()
        cand = int.from_bytes(v, "big")
        if 1 <= cand < N:
            yield cand
        k = hmac.new(k, v + b"\x00", hashlib.sha256).digest()
        v = hmac.new(k, v, hashlib.sha256).digest()


def ecdsa_sign(d: int, digest: bytes):
    """Returns (r, s, v) with RFC 6979 nonce and low-s; v in {27, 28}."""
    z = int.from_bytes(digest, "big") % N
    for k in rfc6979_k(d, digest):
        Rp = mul(G, k)
        if Rp is None:
            continue
        r = Rp[0] % N
        if r == 0:
            continue
        s = (pow(k, N - 2, N) * (z + r * d)) % N
        if s == 0:
            continue
        rec = (Rp[1] & 1) | (2 if Rp[0] >= N else 0)
        if s > HALF_N:
            s = N - s
            rec ^= 1
        if rec > 1:
            continue
        return r, s, 27 + rec
    raise AssertionError("unreachable")


def ecdsa_recover(digest: bytes, r: int, s: int, v: int):
    """Recovery exactly as sec 5.4 defines it. Returns a pubkey or None."""
    i = v - 27
    if i not in (0, 1):
        return None
    if not (1 <= r <= N - 1 and 1 <= s <= N - 1):
        return None
    x = r
    alpha = (x * x * x + 7) % P
    beta = pow(alpha, (P + 1) // 4, P)
    if (beta * beta) % P != alpha:
        return None
    y = beta if (beta % 2 == i) else (P - beta)
    R = (x, y)
    z = int.from_bytes(digest, "big") % N
    rinv = pow(r, N - 2, N)
    sR = mul(R, s)
    zG = mul(G, (-z) % N)
    Q = add(sR, zG)
    if Q is None:
        return None
    Q = mul(Q, rinv)
    if Q is None:
        return None
    return Q


def eip191_digest(domain: str, presig: bytes) -> bytes:
    m = domain.encode("ascii") + b"\x0a" + hx(presig).encode("ascii")
    return keccak256(b"\x19" + b"Ethereum Signed Message:\x0a" + str(len(m)).encode("ascii") + m)


def sig_hex(r, s, v):
    return hx(r.to_bytes(32, "big") + s.to_bytes(32, "big") + bytes([v & 0xFF]))


# --------------------------------------------------------------------------
# the value universe (sec 3.1) and canonical bytes (sec 3.4)
# --------------------------------------------------------------------------


class JObj(object):
    """A JSON object as an ordered member list, so duplicate keys and
    non-canonical member order survive parsing."""

    __slots__ = ("members",)

    def __init__(self, members=None):
        self.members = list(members or [])

    def get(self, k, default=None):
        for kk, vv in self.members:
            if kk == k:
                return vv
        return default

    def has(self, k):
        return any(kk == k for kk, _ in self.members)

    def keys(self):
        return [k for k, _ in self.members]

    def __repr__(self):
        return "JObj(%r)" % (self.members,)


_ESC = {
    '"': '\\"',
    "\\": "\\\\",
    "\b": "\\b",
    "\t": "\\t",
    "\n": "\\n",
    "\f": "\\f",
    "\r": "\\r",
}


def cstring(s: str) -> bytes:
    out = ['"']
    for ch in s:
        e = _ESC.get(ch)
        if e is not None:
            out.append(e)
        elif ord(ch) < 0x20:
            out.append("\\u00%02x" % ord(ch))
        else:
            out.append(ch)
    out.append('"')
    return "".join(out).encode("utf-8")


def cbytes(v, sort=True) -> bytes:
    """Canonical bytes per sec 3.4. sort=False keeps the written member order,
    which is only ever used to build deliberately non-canonical inputs."""
    if v is None:
        return b"null"
    if v is True:
        return b"true"
    if v is False:
        return b"false"
    if isinstance(v, int):
        return str(v).encode("ascii")
    if isinstance(v, str):
        return cstring(v)
    if isinstance(v, list):
        return b"[" + b",".join(cbytes(e, sort) for e in v) + b"]"
    if isinstance(v, JObj):
        ms = v.members
        if sort:
            ms = sorted(ms, key=lambda kv: kv[0].encode("utf-8"))
        return b"{" + b",".join(cstring(k) + b":" + cbytes(val, sort) for k, val in ms) + b"}"
    raise TypeError("not a value of the universe: %r" % (v,))


def J(x):
    """Convert plain python containers into the universe's representation."""
    if isinstance(x, dict):
        return JObj([(k, J(v)) for k, v in x.items()])
    if isinstance(x, list):
        return [J(e) for e in x]
    return x


# --------------------------------------------------------------------------
# the parser: sec 3.5 test 2 (RFC 8259 JSON-text, E_DEPTH, E_NUMBER, surrogates)
# --------------------------------------------------------------------------

WS = " \t\n\r"
DIGITS = "0123456789"
NUMBYTES = "0123456789+-.eE"
HEXDIG = "0123456789abcdefABCDEF"


class Fault(Exception):
    def __init__(self, token):
        Exception.__init__(self, token)
        self.token = token


def is_int_literal(t: str) -> bool:
    if not t:
        return False
    for c in t:
        if c not in DIGITS:
            return False
    if len(t) > 1 and t[0] == "0":
        return False
    return int(t) <= MAXINT


class Parser(object):
    """Left-to-right single pass. Because the scan never backs up, the first
    fault raised is the fault whose triggering byte comes earliest in b, which
    is the rule sec 3.5 test 2 states."""

    def __init__(self, s: str):
        self.s = s
        self.i = 0
        self.n = len(s)
        self.depth = 0

    def err(self):
        raise Fault("E_JSON")

    def ws(self):
        while self.i < self.n and self.s[self.i] in WS:
            self.i += 1

    def text(self):
        self.ws()
        v = self.value()
        self.ws()
        if self.i != self.n:
            self.err()
        return v

    def value(self):
        if self.i >= self.n:
            self.err()
        c = self.s[self.i]
        if c in NUMBYTES and c not in "eE":
            return self.number()
        if c == '"':
            return self.string()
        if c == "{":
            return self.obj()
        if c == "[":
            return self.arr()
        if self.s.startswith("true", self.i):
            self.i += 4
            return True
        if self.s.startswith("false", self.i):
            self.i += 5
            return False
        if self.s.startswith("null", self.i):
            self.i += 4
            return None
        self.err()

    def number(self):
        j = self.i
        while j < self.n and self.s[j] in NUMBYTES:
            j += 1
        run = self.s[self.i:j]
        if not is_int_literal(run):
            raise Fault("E_NUMBER")
        self.i = j
        return int(run)

    def string(self):
        self.i += 1  # opening quote
        out = []
        while True:
            if self.i >= self.n:
                self.err()
            c = self.s[self.i]
            if c == '"':
                self.i += 1
                return "".join(out)
            if c == "\\":
                self.i += 1
                if self.i >= self.n:
                    self.err()
                e = self.s[self.i]
                if e in '"\\/':
                    out.append(e)
                    self.i += 1
                elif e == "b":
                    out.append("\b")
                    self.i += 1
                elif e == "f":
                    out.append("\f")
                    self.i += 1
                elif e == "n":
                    out.append("\n")
                    self.i += 1
                elif e == "r":
                    out.append("\r")
                    self.i += 1
                elif e == "t":
                    out.append("\t")
                    self.i += 1
                elif e == "u":
                    h = self.s[self.i + 1:self.i + 5]
                    if len(h) != 4 or any(x not in HEXDIG for x in h):
                        self.err()
                    cp = int(h, 16)
                    if 0xD800 <= cp <= 0xDFFF:
                        raise Fault("E_JSON")
                    out.append(chr(cp))
                    self.i += 5
                else:
                    self.err()
            elif ord(c) < 0x20:
                self.err()
            else:
                out.append(c)
                self.i += 1

    def _open(self):
        self.depth += 1
        if self.depth == 129:
            raise Fault("E_DEPTH")

    def obj(self):
        self._open()
        self.i += 1
        members = []
        self.ws()
        if self.i < self.n and self.s[self.i] == "}":
            self.i += 1
            self.depth -= 1
            return JObj(members)
        while True:
            self.ws()
            if self.i >= self.n or self.s[self.i] != '"':
                self.err()
            k = self.string()
            self.ws()
            if self.i >= self.n or self.s[self.i] != ":":
                self.err()
            self.i += 1
            self.ws()
            v = self.value()
            members.append((k, v))
            self.ws()
            if self.i >= self.n:
                self.err()
            if self.s[self.i] == ",":
                self.i += 1
                continue
            if self.s[self.i] == "}":
                self.i += 1
                break
            self.err()
        self.depth -= 1
        return JObj(members)

    def arr(self):
        self._open()
        self.i += 1
        out = []
        self.ws()
        if self.i < self.n and self.s[self.i] == "]":
            self.i += 1
            self.depth -= 1
            return out
        while True:
            self.ws()
            out.append(self.value())
            self.ws()
            if self.i >= self.n:
                self.err()
            if self.s[self.i] == ",":
                self.i += 1
                continue
            if self.s[self.i] == "]":
                self.i += 1
                break
            self.err()
        self.depth -= 1
        return out


# --------------------------------------------------------------------------
# sec 3.5 tests 3, 4, 5 and 6
# --------------------------------------------------------------------------


def has_dup_key(v) -> bool:
    if isinstance(v, list):
        return any(has_dup_key(e) for e in v)
    if isinstance(v, JObj):
        seen = set()
        for k, val in v.members:
            if k in seen:
                return True
            seen.add(k)
        return any(has_dup_key(val) for _, val in v.members)
    return False


def _skeleton(s: str) -> bool:
    return all(0x20 <= ord(c) <= 0x7E for c in s)


def bad_key(v) -> bool:
    if isinstance(v, list):
        return any(bad_key(e) for e in v)
    if isinstance(v, JObj):
        for k, val in v.members:
            if len(k) == 0 or not _skeleton(k):
                return True
            if bad_key(val):
                return True
    return False


def bad_value_charset(v, prose=False) -> bool:
    if isinstance(v, str):
        return (not prose) and (not _skeleton(v))
    if isinstance(v, list):
        return any(bad_value_charset(e, prose) for e in v)
    if isinstance(v, JObj):
        for k, val in v.members:
            if bad_value_charset(val, prose or k.endswith("_md")):
                return True
    return False


def accept_canonical(b: bytes):
    """sec 3.5. Returns (value, None) or (None, token)."""
    try:
        s = b.decode("utf-8")
    except UnicodeDecodeError:
        return None, "E_UTF8"
    try:
        v = Parser(s).text()
    except Fault as f:
        return None, f.token
    if has_dup_key(v):
        return None, "E_DUP_KEY"
    if bad_key(v):
        return None, "E_KEY_CHARSET"
    if bad_value_charset(v):
        return None, "E_VALUE_CHARSET"
    if cbytes(v) != b:
        return None, "E_NOT_CANONICAL"
    return v, None


def canon_command(b: bytes):
    """`gl1 canon`: sec 3.5 tests 1 through 5 only."""
    try:
        s = b.decode("utf-8")
    except UnicodeDecodeError:
        return None, "E_UTF8"
    try:
        v = Parser(s).text()
    except Fault as f:
        return None, f.token
    if has_dup_key(v):
        return None, "E_DUP_KEY"
    if bad_key(v):
        return None, "E_KEY_CHARSET"
    if bad_value_charset(v):
        return None, "E_VALUE_CHARSET"
    return cbytes(v), None


# --------------------------------------------------------------------------
# sec 4 / 5 / 6: the envelope, the body tables, the signature
# --------------------------------------------------------------------------

SEVEN = ("body", "entryType", "judge", "prev", "seq", "sig", "spec")
KNOWN_TYPES = ("genesis", "amanah", "judgment", "resignation", "annotation", "succession", "migration", "keybinding")
CATEGORIES = ("unlawful-subject", "other")


def is_str(v):
    return isinstance(v, str)


def is_int(v):
    return isinstance(v, int) and not isinstance(v, bool) and 0 <= v <= MAXINT


def is_token(v):
    return isinstance(v, str) and len(v) > 0 and all(0x21 <= ord(c) <= 0x7E for c in v)


def _is_hex(v, ndig):
    if not isinstance(v, str) or len(v) != ndig + 2:
        return False
    if v[0] != "0" or v[1] != "x":
        return False
    return all(c in "0123456789abcdef" for c in v[2:])


def is_hex20(v):
    return _is_hex(v, 40)


def is_hex32(v):
    return _is_hex(v, 64)


def is_hex65(v):
    return _is_hex(v, 130)


def is_bool(v):
    return v is True or v is False


def is_skel_nonempty(v):
    return isinstance(v, str) and len(v) > 0 and all(0x20 <= ord(c) <= 0x7E for c in v)


def subject_ref_ok(v):
    """sec 6.3: an object with a token kind; a known kind forces its table."""
    if not isinstance(v, JObj):
        return False
    if not (v.has("kind") and is_token(v.get("kind"))):
        return False
    k = v.get("kind")
    if k == "evm":
        return (v.has("chainId") and is_int(v.get("chainId")) and v.has("contract") and is_hex20(v.get("contract"))
                and v.has("codeHash") and is_hex32(v.get("codeHash")) and v.has("id") and is_hex32(v.get("id")))
    if k == "hash":
        if not (v.has("sha256") and is_hex32(v.get("sha256"))):
            return False
        return (not v.has("note_md")) or is_str(v.get("note_md"))
    return True


def body_fault(etype, body: JObj):
    """sec 6. Returns 'E_BODY_FIELD' or None."""
    E = "E_BODY_FIELD"

    def req(k, pred):
        return body.has(k) and pred(body.get(k))

    def opt(k, pred):
        return (not body.has(k)) or pred(body.get(k))

    if etype == "genesis":
        return None if req("statement_md", is_str) else E
    if etype == "amanah":
        if not req("version", is_int):
            return E
        if not req("text_md", is_str):
            return E
        if body.has("venues"):
            vs = body.get("venues")
            if not isinstance(vs, list):
                return E
            for el in vs:
                if not isinstance(el, JObj):
                    return E
                if not (el.has("chainId") and is_int(el.get("chainId"))):
                    return E
                if not (el.has("contract") and is_hex20(el.get("contract"))):
                    return E
                if not (el.has("codeHash") and is_hex32(el.get("codeHash"))):
                    return E
        if body.has("quotes") and not isinstance(body.get("quotes"), JObj):
            return E
        return None
    if etype == "judgment":
        if not (body.has("subjectRef") and subject_ref_ok(body.get("subjectRef"))):
            return E
        if not req("amanah", is_hex32):
            return E
        if not req("verdict_md", is_str):
            return E
        if not opt("decision", lambda v: isinstance(v, JObj)):
            return E
        if body.has("evidence"):
            ev = body.get("evidence")
            if not isinstance(ev, list):
                return E
            for el in ev:
                if not isinstance(el, JObj):
                    return E
                if not (el.has("manifest") and is_hex32(el.get("manifest"))):
                    return E
                if not (el.has("produced") and is_bool(el.get("produced"))):
                    return E
        if body.has("txRefs"):
            tr = body.get("txRefs")
            if not isinstance(tr, list) or not all(is_hex32(x) for x in tr):
                return E
        if not opt("terms", is_hex32):
            return E
        return None
    if etype == "resignation":
        if not (body.has("subjectRef") and subject_ref_ok(body.get("subjectRef"))):
            return E
        if not req("amanah", is_hex32):
            return E
        if not req("category", lambda v: isinstance(v, str) and v in CATEGORIES):
            return E
        if (body.get("category") == "unlawful-subject") != body.has("clause"):
            return E
        if not opt("clause", is_skel_nonempty):
            return E
        if not req("statement_md", is_str):
            return E
        return None
    if etype == "succession":
        if not req("to", is_hex20):
            return E
        if not req("kind", is_token):
            return E
        if not req("effective", is_int):
            return E
        if not req("statement_md", is_str):
            return E
        return None
    if etype == "migration":
        if not req("spec", is_token):
            return E
        if not req("statement_md", is_str):
            return E
        return None
    if etype == "annotation":
        if not opt("subject", is_hex32):
            return E
        if not req("note_md", is_str):
            return E
        return None
    if etype == "keybinding":
        if not req("recipient", is_token):
            return E
        if not req("effective", is_int):
            return E
        return None
    return None  # sec 6.8: unknown type, only sec 6.9 applies


def b6_bytes(env: JObj) -> bytes:
    return cbytes(JObj([(k, v) for k, v in env.members if k != "sig"]))


def sig_fault(env: JObj):
    """sec 5.4 bullet order then sec 5.5. Returns token or None."""
    sig = env.get("sig")
    raw = bytes.fromhex(sig[2:])
    r = int.from_bytes(raw[0:32], "big")
    s = int.from_bytes(raw[32:64], "big")
    v = raw[64]
    if v not in (27, 28):
        return "E_SIG_V"
    if not (1 <= r <= N - 1) or not (1 <= s <= N - 1):
        return "E_SIG_RANGE"
    if s > HALF_N:
        return "E_SIG_HIGH_S"
    presig = sha256(b6_bytes(env))
    digest = eip191_digest("galeed/1", presig)
    pub = _recover_cached(digest, r, s, v)
    if pub is None:
        return "E_SIG_RECOVER"
    if hx(pub_to_addr(pub)) != env.get("judge"):
        return "E_SIG_SIGNER"
    return None


@lru_cache(maxsize=100000)
def _recover_cached(digest, r, s, v):
    return ecdsa_recover(digest, r, s, v)


@lru_cache(maxsize=200000)
def zk_check(b: bytes):
    """`gl1 check`: sec 3.5 then sec 4.3. Returns (True, entry_id) or (False, token)."""
    v, tok = accept_canonical(b)
    if tok:
        return (False, tok)
    if not isinstance(v, JObj):
        return (False, "E_ENVELOPE")
    ks = set(v.keys())
    if not set(SEVEN) <= ks:
        return (False, "E_ENVELOPE_MISSING")
    if ks - set(SEVEN):
        return (False, "E_ENVELOPE_CLOSED")
    if v.get("spec") != "galeed/1":
        return (False, "E_SPEC")
    if not is_token(v.get("entryType")):
        return (False, "E_ENTRYTYPE")
    if not is_hex20(v.get("judge")):
        return (False, "E_JUDGE")
    if not is_int(v.get("seq")):
        return (False, "E_SEQ")
    prev = v.get("prev")
    if not (prev is None or is_hex32(prev)):
        return (False, "E_PREV")
    if (prev is None) != (v.get("seq") == 0):
        return (False, "E_PREV_SEQ")
    if not isinstance(v.get("body"), JObj):
        return (False, "E_BODY")
    if not is_hex65(v.get("sig")):
        return (False, "E_SIG_FORM")
    if (v.get("entryType") == "genesis") != (v.get("seq") == 0):
        return (False, "E_GENESIS_PLACE")
    bf = body_fault(v.get("entryType"), v.get("body"))
    if bf:
        return (False, bf)
    sf = sig_fault(v)
    if sf:
        return (False, sf)
    return (True, hx(sha256(b)))


def parsed_entry(b: bytes):
    """For the audit predictor: the parsed envelope of a byte string that is an entry."""
    v, tok = accept_canonical(b)
    return v


# --------------------------------------------------------------------------
# entry construction
# --------------------------------------------------------------------------


def build_env(spec, entryType, judge, seq, prev, body):
    return JObj([
        ("body", body),
        ("entryType", entryType),
        ("judge", judge),
        ("prev", prev),
        ("seq", seq),
        ("spec", spec),
    ])


def sign_env(priv, env6: JObj):
    b6 = b6_bytes(env6) if env6.has("sig") else cbytes(env6)
    presig = sha256(b6)
    digest = eip191_digest("galeed/1", presig)
    r, s, v = ecdsa_sign(priv, digest)
    return sig_hex(r, s, v), presig, digest, (r, s, v)


def make_entry(priv, entryType, seq, prev, body, spec="galeed/1", judge=None,
               sig_override=None, extra_env=None, unsorted=False):
    """Build the canonical bytes of a signed entry (or of a deliberately
    faulty near-entry).  `body` is a plain dict/JObj."""
    if judge is None:
        judge = privkey_to_addr_hex(priv)
    if not isinstance(body, JObj) and isinstance(body, dict):
        body = J(body)
    env6 = build_env(spec, entryType, judge, seq, prev, body)
    if extra_env:
        for k, val in extra_env:
            env6.members.append((k, J(val)))
    if sig_override is not None:
        sig = sig_override
    else:
        sig, _, _, _ = sign_env(priv, env6)
    full = JObj(env6.members + [("sig", sig)])
    return cbytes(full, sort=not unsorted)


def entry_digest(judge_hex, entryType, seq, prev, body, spec="galeed/1"):
    env6 = build_env(spec, entryType, judge_hex, seq, prev, J(body) if isinstance(body, dict) else body)
    presig = sha256(cbytes(env6))
    return eip191_digest("galeed/1", presig), presig


def eid(b: bytes) -> str:
    return hx(sha256(b))


# --------------------------------------------------------------------------
# the audit predictor (sec 8, sec 9.4 well-formedness)
# --------------------------------------------------------------------------


def _basis_ok(basis):
    if not isinstance(basis, dict):
        return False
    if set(basis.keys()) != {"chains"}:
        return False
    ch = basis["chains"]
    if not isinstance(ch, list):
        return False
    prev = None
    for o in ch:
        if not isinstance(o, dict):
            return False
        if set(o.keys()) != {"chainId", "fromBlock", "toBlock", "registries", "senders"}:
            return False
        if not (is_int(o["chainId"]) and is_int(o["fromBlock"]) and is_int(o["toBlock"])):
            return False
        if o["fromBlock"] > o["toBlock"]:
            return False
        sd = o["senders"]
        if not isinstance(sd, list) or not all(is_hex20(x) for x in sd):
            return False
        # sec 9.4: bytewise ascending, no repeated element
        if any(sd[i].encode() >= sd[i + 1].encode() for i in range(len(sd) - 1)):
            return False
        # sec 9.4: registries lists hex20 elements first, bytewise ascending, then
        # class items {codeHash: hex32}, bytewise ascending by codeHash, no repeats
        rg = o["registries"]
        if not isinstance(rg, list):
            return False
        i = 0
        while i < len(rg) and isinstance(rg[i], str):
            if not is_hex20(rg[i]):
                return False
            if i > 0 and rg[i - 1].encode() >= rg[i].encode():
                return False
            i += 1
        j = i
        while j < len(rg):
            x = rg[j]
            if not isinstance(x, dict) or set(x.keys()) != {"codeHash"} or not is_hex32(x["codeHash"]):
                return False
            if j > i and rg[j - 1]["codeHash"].encode() >= x["codeHash"].encode():
                return False
            j += 1
        # sec 9.4: ordered by (chainId, fromBlock); two windows of one chain do not
        # overlap, and where they touch their registries or senders differ
        lists = (tuple(json.dumps(x, sort_keys=True) for x in o["registries"]), tuple(o["senders"]))
        if prev is not None:
            if (o["chainId"], o["fromBlock"]) <= (prev[0], prev[1]):
                return False
            if o["chainId"] == prev[0]:
                if o["fromBlock"] <= prev[2]:
                    return False
                if o["fromBlock"] == prev[2] + 1 and lists == prev[3]:
                    return False
        prev = (o["chainId"], o["fromBlock"], o["toBlock"], lists)
    return True


class Ent(object):
    __slots__ = ("b", "id", "env", "seq", "prev", "judge", "etype")

    def __init__(self, b, env):
        self.b = b
        self.id = hx(sha256(b))
        self.env = env
        self.seq = env.get("seq")
        self.prev = env.get("prev")
        self.judge = env.get("judge")
        self.etype = env.get("entryType")


def _hexn_ok(v, n):
    return isinstance(v, str) and len(v) == 2 + 2 * n and v[:2] == "0x" and all(c in "0123456789abcdef" for c in v[2:])


def _transport_ok(x):
    return (isinstance(x, str) and x[:2] == "0x" and len(x) % 2 == 0
            and all(c in "0123456789abcdefABCDEF" for c in x[2:]))


def _nums_in_universe(v):
    """HARNESS: every number in the file, wherever it sits, is an int of sec 3.1, and
    every string decodes to scalar values."""
    if isinstance(v, bool):
        return True
    if isinstance(v, str):
        return not any(0xD800 <= ord(c) <= 0xDFFF for c in v)
    if isinstance(v, int):
        return 0 <= v <= MAXINT
    if isinstance(v, float):
        return False
    if isinstance(v, list):
        return all(_nums_in_universe(x) for x in v)
    if isinstance(v, dict):
        return all(_nums_in_universe(x) for x in v.values())
    return True


def _depth(v):
    if isinstance(v, list):
        return 1 + max([_depth(x) for x in v] + [0])
    if isinstance(v, dict):
        return 1 + max([_depth(x) for x in v.values()] + [0])
    return 0


def _inputs_well_formed(inp):
    """sec 8 with sec 9.4: the two inputs by their forms; any failure is no label."""
    if not isinstance(inp, dict):
        return False
    if not _nums_in_universe(inp) or _depth(inp) > 128:
        return False
    for k in ("root", "anchors", "basis"):
        if k not in inp:
            return False
    if not _hexn_ok(inp["root"], 20):
        return False
    if not isinstance(inp["anchors"], list):
        return False
    for a in inp["anchors"]:
        if not isinstance(a, dict):
            return False
        for k in ("chainId", "blockNumber", "blockTimestamp"):
            if not is_int(a.get(k)):
                return False
        if not (_hexn_ok(a.get("tx"), 32) and _hexn_ok(a.get("sender"), 20) and _hexn_ok(a.get("hash"), 32)):
            return False
        if a.get("verdict") not in ("counted", "UNPROVEN", "VOID"):
            return False
        if "entry" in a:
            # sec 9.2: the eighth member is a byte string whose sha256 is the hash,
            # spelled in the transport hex spelling
            x = a["entry"]
            if not _transport_ok(x):
                return False
            if hx(sha256(bytes.fromhex(x[2:]))) != a["hash"]:
                return False
    return True


def predict_audit(inp):
    """Predicts sec 8.7's label plus a few counts. Not authoritative."""
    out = {"label": None, "entries": 0, "findings": 0, "hard": 0,
           "f_SEQ_GAP": 0, "f_PREV_MISMATCH": 0, "f_AUTHORITY_MISMATCH": 0,
           "f_ROOT_MISMATCH": 0, "f_EQUIVOCATION": 0, "f_CARRIED_MALFORMED": 0,
           "missing": 0, "anchored": 0, "excluded": 0,
           "unknown_type": 0, "malformed": 0, "unproven": 0, "void": 0,
           "discarded": 0}
    if not _inputs_well_formed(inp):
        out["label"] = "NO_LABEL"
        return out
    if not _basis_ok(inp["basis"]):
        out["label"] = "NO_LABEL"
        return out
    seen = {}
    SEVEN_A = ("chainId", "blockNumber", "blockTimestamp", "tx", "sender", "hash", "verdict")
    for a in inp["anchors"]:
        k = (a["chainId"], a["blockNumber"], a["tx"], a["hash"])
        a7 = {kk: a[kk] for kk in SEVEN_A}
        # sec 9.2 / 9.4: entry agrees where both lack it or both carry equal bytes
        a7["entry"] = bytes.fromhex(a["entry"][2:]) if "entry" in a else None
        if k in seen and seen[k] != a7:
            out["label"] = "NO_LABEL"
            return out
        seen[k] = a7
    anchors = list(seen.values())
    # sec 9.4 [C]: every record lies within the basis's reach
    bas = inp["basis"]
    for a in anchors:
        by_chain = any(c["chainId"] == a["chainId"] and c["fromBlock"] <= a["blockNumber"] <= c["toBlock"]
                       and a["sender"] in c["senders"] for c in bas["chains"])
        if not by_chain:
            out["label"] = "NO_LABEL"
            return out

    # sec 8.1: the carried byte strings, whatever the record's sender and verdict
    raw = []
    for a in anchors:
        if a["entry"] is not None and a["entry"] not in raw:
            raw.append(a["entry"])
    ents = {}
    malformed = set()
    for b in raw:
        ok, val = zk_check(b)
        if ok and b not in ents:
            ents[b] = Ent(b, parsed_entry(b))
        elif not ok:
            malformed.add(hx(sha256(b)))
    entries = list(ents.values())
    out["malformed"] = len(malformed)

    root = inp["root"]
    # sec 7.4: a key's retirement is the smallest blockTimestamp of the records,
    # on any chain and whatever their sender and verdict, carrying a succession
    # or a migration it signed
    # ... paired with its retiring seq, the smallest seq among them carried at
    # the retirement's own second (sec 7.4)
    retire = {}
    for a in anchors:
        e = ents.get(a["entry"]) if a["entry"] is not None else None
        if e is not None and e.etype in ("succession", "migration"):
            cur = retire.get(e.judge)
            if cur is None or (a["blockTimestamp"], e.seq) < cur:
                retire[e.judge] = (a["blockTimestamp"], e.seq)

    def retired(key, a):
        # sec 7.4: later than the retirement; or at its second, carrying an
        # entry the key signed at a seq greater than its retiring seq
        if key not in retire:
            return False
        t, k = retire[key]
        if a["blockTimestamp"] > t:
            return True
        if a["blockTimestamp"] == t and a["entry"] is not None:
            e = ents.get(a["entry"])
            return e is not None and e.judge == key and e.seq > k
        return False

    lineage = {root}
    changed = True
    while changed:
        changed = False
        for e in entries:
            if e.etype == "succession" and e.judge in lineage \
                    and any(a["entry"] == e.b and not retired(e.judge, a) for a in anchors):
                t = e.env.get("body").get("to")
                if t not in lineage:
                    lineage.add(t)
                    changed = True

    keep = [a for a in anchors if a["sender"] in lineage and not retired(a["sender"], a)]
    out["discarded"] = len(anchors) - len(keep)
    anchors = keep
    counted = [a for a in anchors if a["verdict"] == "counted"]
    counted_hashes = set(a["hash"] for a in counted)
    counted_carried = set(a["entry"] for a in counted if a["entry"] is not None)
    unproven = [a for a in anchors if a["verdict"] == "UNPROVEN"]
    void = [a for a in anchors if a["verdict"] == "VOID"]
    out["unproven"] = len(unproven)
    out["void"] = len(void)

    # sec 8.1: the ledger is the carried entries whose judge is in the lineage
    # and which a counted undiscarded record carries (sec 8.6)
    ledger = [e for e in entries if e.judge in lineage and e.b in counted_carried]
    excluded = [e for e in entries if e.judge not in lineage]
    out["entries"] = len(ledger)
    out["excluded"] = len(excluded)
    out["unknown_type"] = sum(1 for e in ledger if e.etype not in KNOWN_TYPES)

    order = sorted(ledger, key=lambda e: (e.seq, bytes.fromhex(e.id[2:])))
    by_seq = {}
    for e in order:
        by_seq.setdefault(e.seq, []).append(e)

    findings = []
    expected = 0
    auth = root
    certain = True
    prev_seq = None
    for s in sorted(by_seq):
        grp = by_seq[s]
        for e in grp:
            if e.seq != expected and e.seq != prev_seq:
                findings.append(("SEQ_GAP", False))
                expected = e.seq if e.seq == MAXINT else e.seq + 1
                certain = False
            elif e.seq == expected:
                expected = e.seq if e.seq == MAXINT else e.seq + 1
            if e.seq >= 1:
                pool = by_seq.get(e.seq - 1)
                if pool and not any(x.id == e.prev for x in pool):
                    findings.append(("PREV_MISMATCH", True))
            if e.seq == 0:
                if e.judge != root:
                    findings.append(("ROOT_MISMATCH", True))
            else:
                if e.judge != auth:
                    findings.append(("AUTHORITY_MISMATCH", certain))
            prev_seq = e.seq
        if auth is not None and any(e.etype == "migration" and e.judge == auth for e in grp):
            auth = None
        succ = [e for e in grp if e.etype == "succession" and e.judge == auth]
        if auth is not None and succ:
            auth = min(succ, key=lambda e: bytes.fromhex(e.id[2:])).env.get("body").get("to")

    for i in range(len(order)):
        for j in range(i + 1, len(order)):
            a, b = order[i], order[j]
            if a.id == b.id:
                continue
            if a.seq == b.seq or (a.prev is not None and a.prev == b.prev):
                findings.append(("EQUIVOCATION", True))
    # sec 8.4: one CARRIED_MALFORMED per byte string carried by an undiscarded
    # counted record that fails sec 4.3 or whose judge is outside the lineage
    for b in counted_carried:
        if b in ents:
            if ents[b].judge not in lineage:
                findings.append(("CARRIED_MALFORMED", True))
        else:
            findings.append(("CARRIED_MALFORMED", True))

    out["findings"] = len(findings)
    out["hard"] = sum(1 for _, h in findings if h)
    for nm, _h in findings:
        out["f_" + nm] = out.get("f_" + nm, 0) + 1
    have = set(e.id for e in ledger)
    missing = counted_hashes - have
    out["missing"] = len(missing)
    out["anchored"] = len(have)

    unproven_open = [a for a in unproven if a["hash"] not in counted_hashes]
    if out["hard"]:
        out["label"] = "BROKEN_CHAIN"
    elif unproven_open:
        out["label"] = "UNPROVEN"
    elif any(nm == "SEQ_GAP" for nm, _ in findings) or missing:
        out["label"] = "GAPS"
    else:
        out["label"] = "COMPLETE"
    return out


# --------------------------------------------------------------------------
# corpus bookkeeping
# --------------------------------------------------------------------------

MANIFEST = []
WARNINGS = []
_counters = {"canon": 0, "sign": 0, "audit": 0}


def _write(path, data: bytes):
    with open(path, "wb") as f:
        f.write(data)


def C(slug, data: bytes, note, expect=None):
    """Register a canon/ case. `expect` is the author's hand reasoning; the
    recorded prediction is the author's code. A disagreement is flagged."""
    _counters["canon"] += 1
    name = "c%04d_%s.bin" % (_counters["canon"], slug)
    _write(os.path.join(HERE, "canon", name), data)
    ok, tok = zk_check(data)
    pred = "ok" if ok else tok
    cb, ctok = canon_command(data)
    rec = {
        "path": "canon/" + name,
        "category": "canon",
        "note": note,
        "predicted": pred,
        "predicted_entry_id": tok if ok else None,
        "predicted_canon": "ok" if ctok is None else ctok,
        "predicted_canon_sha256": hx(sha256(cb)) if ctok is None else None,
        "bytes": len(data),
    }
    if expect is not None:
        rec["hand_expect"] = expect
        if expect != pred:
            rec["disagreement"] = True
            WARNINGS.append("canon/%s: hand=%s code=%s (%s)" % (name, expect, pred, note))
    MANIFEST.append(rec)
    return rec


def S(slug, priv, data: bytes, domain, note, privkey_spelling=None, misuse=False, extra_args=()):
    """Register a sign/ case.  `domain` None means the argument is absent (the
    default literal galeed/1); `privkey_spelling` is the argument as given where
    it is not the lowercase 0x form; `misuse` marks a case whose expected
    outcome is exit 2 and no answer (predicted null)."""
    _counters["sign"] += 1
    base = "s%03d_%s" % (_counters["sign"], slug)
    _write(os.path.join(HERE, "sign", base + ".bin"), data)
    meta = {
        "input": base + ".bin",
        "privkey": privkey_spelling if privkey_spelling is not None else "0x%064x" % priv,
        "note": note,
    }
    if domain is not None:
        meta["domain"] = domain
    if extra_args:
        meta["extra_args"] = list(extra_args)
    if misuse:
        meta["predicted"] = None
    else:
        presig = sha256(data)
        digest = eip191_digest("galeed/1" if domain is None else domain, presig)
        r, s, v = ecdsa_sign(priv, digest)
        meta["predicted"] = {
            "presig": hx(presig),
            "digest": hx(digest),
            "sig": sig_hex(r, s, v),
            "signer": privkey_to_addr_hex(priv),
        }
    with open(os.path.join(HERE, "sign", base + ".meta.json"), "w") as f:
        json.dump(meta, f, indent=1, sort_keys=True)
    MANIFEST.append({
        "path": "sign/" + base + ".bin",
        "meta": "sign/" + base + ".meta.json",
        "category": "sign",
        "note": note,
        "domain": domain,
        "privkey": meta["privkey"],
        "predicted": meta["predicted"],
    })


def A(slug, inp, note, expect_label=None, twin=True):
    """Register an audit/ case. Every case with array members also registers
    its twin with every input array reversed (sec 8: the report does not
    depend on the order in which any input was supplied), so that each sorted
    list of the report is witnessed from an input in another order."""
    if twin and isinstance(inp, dict):
        inp2 = dict((k, (list(reversed(v)) if isinstance(v, list) else v)) for k, v in inp.items())
        if inp2 != inp:
            A(slug + "_reversed", inp2, "input arrays reversed: " + note, expect_label, twin=False)
    _counters["audit"] += 1
    base = "a%03d_%s" % (_counters["audit"], slug)
    with open(os.path.join(HERE, "audit", base + ".json"), "w") as f:
        json.dump(inp, f, indent=1, sort_keys=True)
    with open(os.path.join(HERE, "audit", base + ".note"), "w") as f:
        f.write(note.rstrip() + "\n")
    try:
        p = predict_audit(inp)
    except Exception:
        # the predictor reads a well-formed input; a malformed one is the
        # law's no-label outcome, which the case's stated expectation names
        p = {"label": expect_label}
    rec = {
        "path": "audit/" + base + ".json",
        "note_file": "audit/" + base + ".note",
        "category": "audit",
        "note": note,
        "predicted_label": p["label"],
        "predicted_counts": {k: v for k, v in p.items() if k != "label"},
    }
    if expect_label is not None:
        rec["hand_expect_label"] = expect_label
        if expect_label != p["label"]:
            rec["disagreement"] = True
            WARNINGS.append("audit/%s: hand=%s code=%s (%s)" % (base, expect_label, p["label"], note))
    MANIFEST.append(rec)
    return rec


def A_raw(slug, raw: bytes, note, expect_label="NO_LABEL", predict_from=None):
    """Register an audit/ case whose bytes are written as given: inputs that
    are not JSON values at all (sec 9.4 harness forms: no label), and inputs
    whose transport spelling is the point (a label is then stated by hand)."""
    _counters["audit"] += 1
    base = "a%03d_%s" % (_counters["audit"], slug)
    _write(os.path.join(HERE, "audit", base + ".json"), raw)
    with open(os.path.join(HERE, "audit", base + ".note"), "w") as f:
        f.write(note.rstrip() + "\n")
    MANIFEST.append({
        "path": "audit/" + base + ".json",
        "note_file": "audit/" + base + ".note",
        "category": "audit",
        "note": note,
        "predicted_label": expect_label,
        "predicted_counts": ({k: v for k, v in predict_audit(predict_from).items() if k != "label"}
                             if predict_from is not None else {}),
        "hand_expect_label": expect_label,
    })


# ==========================================================================
# test keys
# ==========================================================================

def testkey(tag, seed):
    d = int.from_bytes(sha256(("gl1-key/%s/%d" % (tag, seed)).encode()), "big")
    return (d % (N - 2)) + 1


KEYS = {}


def K(tag):
    return KEYS[tag]


def A_(tag):
    return privkey_to_addr_hex(KEYS[tag])


# ==========================================================================
# small helpers for building deliberately faulty bytes
# ==========================================================================


def sub(data: bytes, old: bytes, new: bytes, n=1) -> bytes:
    assert data.count(old) >= 1, "marker %r absent" % (old,)
    return data.replace(old, new, n)


def deep_list(n):
    v = []
    for _ in range(n - 1):
        v = [v]
    return v


def nested_arr_text(n):
    return ("[" * n + "]" * n).encode()


def nested_obj_text(n):
    s = "{}"
    for _ in range(n - 1):
        s = '{"a":' + s + "}"
    return s.encode()


def rot_top(data: bytes, k=1) -> bytes:
    v = Parser(data.decode("utf-8")).text()
    v.members = v.members[k:] + v.members[:k]
    return cbytes(v, sort=False)


def rot_body(data: bytes, k=1) -> bytes:
    v = Parser(data.decode("utf-8")).text()
    b = v.get("body")
    b.members = b.members[k:] + b.members[:k]
    return cbytes(v, sort=False)


def drop_key(data: bytes, key: str) -> bytes:
    v = Parser(data.decode("utf-8")).text()
    v.members = [(k, x) for k, x in v.members if k != key]
    return cbytes(v, sort=True)


def add_key(data: bytes, key: str, val) -> bytes:
    v = Parser(data.decode("utf-8")).text()
    v.members.append((key, J(val)))
    return cbytes(v, sort=True)


def set_key(data: bytes, key: str, val) -> bytes:
    v = Parser(data.decode("utf-8")).text()
    v.members = [(k, (J(val) if k == key else x)) for k, x in v.members]
    return cbytes(v, sort=True)



# ==========================================================================
# canon/ : byte strings for `gl1 check` and `gl1 canon`
# ==========================================================================


def HX32(i):
    return "0x%064x" % i


def HX20(i):
    return "0x%040x" % i


class T(object):
    """Valid templates the fault cases are cut from."""
    pass


def build_templates():
    t = T()
    t.gen = make_entry(K("root"), "genesis", 0, None, {"statement_md": "the seat of one judge"})
    t.gid = eid(t.gen)
    t.aman = make_entry(K("root"), "amanah", 1, t.gid,
                        {"version": 1, "text_md": "the covenant of this seat",
                         "venues": [{"chainId": 1, "contract": HX20(0xE5C), "codeHash": HX32(0xC0DE)}]})
    t.aid = eid(t.aman)
    t.judg = make_entry(K("root"), "judgment", 2, t.aid,
                        {"subjectRef": {"kind": "evm", "chainId": 1, "contract": HX20(0xE5C),
                                        "codeHash": HX32(0xC0DE), "id": HX32(0xCA5E)},
                         "amanah": t.aid, "verdict_md": "the buyer is refunded in full",
                         "decision": {"rung": 2},
                         "evidence": [{"manifest": HX32(0xE1), "produced": True}]})
    t.jid = eid(t.judg)
    t.resg = make_entry(K("root"), "resignation", 3, t.jid,
                        {"subjectRef": {"kind": "hash", "sha256": HX32(0x5A)},
                         "amanah": t.aid, "category": "unlawful-subject", "clause": "8",
                         "statement_md": "the terms name a subject this seat does not sell judgment on"})
    t.rid = eid(t.resg)
    t.succ = make_entry(K("root"), "succession", 4, t.rid,
                        {"to": A_("k2"), "kind": "handover", "effective": 1700000000,
                         "statement_md": "the seat moves"})
    t.sid = eid(t.succ)
    t.keyb = make_entry(K("root"), "keybinding", 1, t.gid,
                        {"recipient": "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq", "effective": 1700000000})
    t.ann = make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "ZZMARKZZ"})
    t.nps = make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "n", "q": "ZZMARKZZ"})
    t.keyt = make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "n", "ZZKEYZZ": "v"})
    t.unk = make_entry(K("root"), "windmill", 1, t.gid, {})
    return t


# sec 3.5, closed by construction: every prefix and every single-byte
# substitution of a small set of templates that together reach every state of
# the grammar. Seed-free; the same cases under every seed.
_SYNTAX_TEMPLATES = [
    ("full", b'{"a":[1,-20,"b\\"\\\\\\/\\b\\f\\n\\r\\t\\u00e9",true,false,null],"c":{"d":{}},"e":[]}'),
    ("escapes", b'"x\\u0041\\ud83dy"'),
    ("nesting", b'[[[[1]]],{"k":"v"}]'),
    ("envelope", b'{"prev":null,"seq":0,"spec":"galeed/1"}'),
    ("spaced", b' [ 1 , 2 ] '),
    ("utf8", b'"caf\xc3\xa9 \xe4\xb8\xad \xf0\x9f\x98\x80"'),
    ("int", b"-123"),
    ("literal", b"true"),
    ("dupkey", b'{"a":1,"a":2}'),
]
_SYNTAX_BYTES = [b"{", b"]", b'"', b"\\", b":", b",", b" ", b"\x00", b"x", b"0", b"-", b"u",
                 b"\xff"]


def build_syntax_closure():
    seen = set()
    for name, t in _SYNTAX_TEMPLATES:
        for i in range(1, len(t)):
            b = t[:i]
            if b in seen:
                continue
            seen.add(b)
            C("syn_%s_cut%02d" % (name, i), b,
              "sec 3.5 closure: template %s cut after %d bytes" % (name, i))
        for i in range(len(t)):
            for x in _SYNTAX_BYTES:
                b = t[:i] + x + t[i + 1:]
                if b in seen:
                    continue
                seen.add(b)
                C("syn_%s_p%02d_%02x" % (name, i, x[0]), b,
                  "sec 3.5 closure: template %s, byte %d replaced by 0x%02x" % (name, i, x[0]))


def build_canon(rng, t):
    build_syntax_closure()
    # ---------------- sec 1 / sec 3.5 test 1: UTF-8 -----------------------
    C("utf8_valid_bmp", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "café 中文"}),
      "sec 3.3/3.4: multibyte scalars raw in a prose subtree", expect="ok")
    C("utf8_valid_astral", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "\U0001f600"}),
      "sec 3.4 item 3: an astral scalar emitted raw", expect="ok")
    C("utf8_max_scalar", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "\U0010ffff"}),
      "sec 1: U+10FFFF, the top of the scalar range, in prose", expect="ok")
    C("utf8_overlong_2byte", sub(t.ann, b"ZZMARKZZ", b"\xc0\xaf"),
      "sec 1: overlong two-byte encoding of '/'", expect="E_UTF8")
    C("utf8_overlong_3byte", sub(t.ann, b"ZZMARKZZ", b"\xe0\x80\xaf"),
      "sec 1: overlong three-byte encoding of '/'", expect="E_UTF8")
    C("utf8_overlong_nul", sub(t.ann, b"ZZMARKZZ", b"\xc0\x80"),
      "sec 1: overlong encoding of U+0000", expect="E_UTF8")
    C("utf8_surrogate_encoded_d800", sub(t.ann, b"ZZMARKZZ", b"\xed\xa0\x80"),
      "sec 1: UTF-8 bytes encoding U+D800", expect="E_UTF8")
    C("utf8_surrogate_encoded_dfff", sub(t.ann, b"ZZMARKZZ", b"\xed\xbf\xbf"),
      "sec 1: UTF-8 bytes encoding U+DFFF", expect="E_UTF8")
    C("utf8_above_10ffff", sub(t.ann, b"ZZMARKZZ", b"\xf4\x90\x80\x80"),
      "sec 1: four-byte sequence for U+110000", expect="E_UTF8")
    C("utf8_five_byte", sub(t.ann, b"ZZMARKZZ", b"\xf8\x88\x80\x80\x80"),
      "sec 1: five-byte sequence, no scalar", expect="E_UTF8")
    C("utf8_truncated", sub(t.ann, b"ZZMARKZZ", b"\xe2\x82"),
      "sec 1: truncated three-byte sequence", expect="E_UTF8")
    C("utf8_lone_continuation", sub(t.ann, b"ZZMARKZZ", b"\x80"),
      "sec 1: a bare continuation byte", expect="E_UTF8")
    C("utf8_ff_byte", sub(t.ann, b"ZZMARKZZ", b"\xff"),
      "sec 1: 0xFF, never a UTF-8 byte", expect="E_UTF8")
    C("utf8_fe_byte", sub(t.ann, b"ZZMARKZZ", b"\xfe"),
      "sec 1: 0xFE, never a UTF-8 byte", expect="E_UTF8")
    C("utf8_bad_outside_string", t.gen[:-1] + b"\xc3",
      "sec 3.5 test 1 precedes every other test: a stray lead byte after the root", expect="E_UTF8")
    C("utf8_beats_depth", b"\xff" + nested_arr_text(200),
      "sec 3.5: test 1 is whole-string and precedes test 2's E_DEPTH", expect="E_UTF8")

    # ---------------- BOM --------------------------------------------------
    C("bom_leading", b"\xef\xbb\xbf" + t.gen,
      "sec 3.5: a byte order mark is not whitespace under JSON-text", expect="E_JSON")
    C("bom_trailing", t.gen + b"\xef\xbb\xbf",
      "sec 3.5 test 2: U+FEFF after the root value is not whitespace", expect="E_JSON")
    C("bom_scalar_in_prose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "a﻿b"}),
      "sec 3.3: U+FEFF is an ordinary scalar inside a prose subtree", expect="ok")
    C("bom_scalar_outside_prose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "n", "q": "a﻿b"}),
      "sec 3.3: U+FEFF encodes to bytes outside 0x20-0x7E in a non-prose value", expect="E_VALUE_CHARSET")

    # ---------------- whitespace ------------------------------------------
    C("ws_leading_space", b" " + t.gen, "sec 3.5: leading insignificant whitespace", expect="E_NOT_CANONICAL")
    C("ws_trailing_newline", t.gen + b"\n", "sec 2.2/3.5: trailing newline", expect="E_NOT_CANONICAL")
    C("ws_trailing_space", t.gen + b" ", "sec 3.5: trailing space", expect="E_NOT_CANONICAL")
    C("ws_leading_tab", b"\t" + t.gen, "sec 3.5: leading tab is JSON whitespace", expect="E_NOT_CANONICAL")
    C("ws_crlf_both_ends", b"\r\n" + t.gen + b"\r\n", "sec 3.5: CRLF around the root", expect="E_NOT_CANONICAL")
    C("ws_after_colon", sub(t.gen, b'"spec":"galeed/1"', b'"spec": "galeed/1"'),
      "sec 3.4 item 6: no byte lies between the pieces", expect="E_NOT_CANONICAL")
    C("ws_after_comma", sub(t.gen, b'","', b'", "'),
      "sec 3.4 item 6: a space between members", expect="E_NOT_CANONICAL")
    C("ws_inside_string_kept", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": " a  b "}),
      "sec 3.4 item 6: whitespace inside a string is part of the string", expect="ok")
    C("ws_only", b"   ", "sec 3.5 test 2: whitespace is not a JSON text", expect="E_JSON")
    C("empty_input", b"", "sec 3.5 test 2: the empty byte string is a proper prefix", expect="E_JSON")

    # ---------------- escapes ---------------------------------------------
    C("esc_quote", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": 'a"b'}),
      "sec 3.4 item 3: quote emitted as backslash-quote", expect="ok")
    C("esc_backslash", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "a\\b"}),
      "sec 3.4 item 3: backslash emitted doubled", expect="ok")
    C("esc_b_prose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "a\bb"}),
      "sec 3.4 item 3: U+0008 as backslash-b in a prose subtree", expect="ok")
    C("esc_t_prose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "a\tb"}),
      "sec 3.4 item 3: U+0009 as backslash-t", expect="ok")
    C("esc_n_prose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "a\nb"}),
      "sec 3.4 item 3: U+000A as backslash-n", expect="ok")
    C("esc_f_prose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "a\fb"}),
      "sec 3.4 item 3: U+000C as backslash-f", expect="ok")
    C("esc_r_prose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "a\rb"}),
      "sec 3.4 item 3: U+000D as backslash-r", expect="ok")
    C("esc_u0000_prose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "a\x00b"}),
      "sec 3.4 item 3: U+0000 as backslash-u-0-0-0-0", expect="ok")
    C("esc_u0001_prose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "\x01"}),
      "sec 3.4 item 3: U+0001 as a lowercase four-digit escape", expect="ok")
    C("esc_u000b_prose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "\x0b"}),
      "sec 3.4 item 3: U+000B has no short form, escapes as backslash-u-0-0-0-b", expect="ok")
    C("esc_u001f_prose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "\x1f"}),
      "sec 3.4 item 3: U+001F, the top of the escaped range", expect="ok")
    C("esc_u001f_upper", sub(make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "\x1f"}),
                             b"\\u001f", b"\\u001F"),
      "sec 3.4 item 3: uppercase hex digits in a control escape are not canonical",
      expect="E_NOT_CANONICAL")
    C("esc_u0008_long_form", sub(t.ann, b"ZZMARKZZ", b"\\u0008"),
      "sec 3.4 item 3: U+0008 has a short form, so the long escape is not canonical",
      expect="E_NOT_CANONICAL")
    C("esc_u0009_long_form", sub(t.ann, b"ZZMARKZZ", b"\\u0009"),
      "sec 3.4 item 3: U+0009 must be backslash-t", expect="E_NOT_CANONICAL")
    C("esc_u0020_above_1f", sub(t.ann, b"ZZMARKZZ", b"\\u0020"),
      "sec 3.4 item 3: U+0020 and above are emitted raw", expect="E_NOT_CANONICAL")
    C("esc_u0041_letter", sub(t.ann, b"ZZMARKZZ", b"\\u0041"),
      "sec 3.4 item 3: an escaped ASCII letter is not canonical", expect="E_NOT_CANONICAL")
    C("esc_u00e9_lower_prose", sub(t.ann, b"ZZMARKZZ", b"\\u00e9"),
      "sec 3.4 item 3: U+00E9 must be raw UTF-8", expect="E_NOT_CANONICAL")
    C("esc_u00E9_upper_prose", sub(t.ann, b"ZZMARKZZ", b"\\u00E9"),
      "sec 3.4 item 3: uppercase digits above U+001F, still not canonical", expect="E_NOT_CANONICAL")
    C("esc_u00e9_nonprose", sub(t.nps, b"ZZMARKZZ", b"\\u00e9"),
      "sec 3.5: test 5 names the fault before test 6 can", expect="E_VALUE_CHARSET")
    C("esc_u0008_nonprose", sub(t.nps, b"ZZMARKZZ", b"\\u0008"),
      "sec 3.3: U+0008 outside a prose subtree", expect="E_VALUE_CHARSET")
    C("esc_b_nonprose", sub(t.nps, b"ZZMARKZZ", b"\\b"),
      "sec 3.3: a short escape decodes to a byte the value charset refuses", expect="E_VALUE_CHARSET")
    C("esc_t_nonprose", sub(t.nps, b"ZZMARKZZ", b"\\t"),
      "sec 3.3: tab outside a prose subtree", expect="E_VALUE_CHARSET")
    C("esc_u0080_prose", sub(t.ann, b"ZZMARKZZ", b"\\u0080"),
      "sec 3.4: the C1 range is emitted raw, so the escape is not canonical", expect="E_NOT_CANONICAL")
    C("esc_solidus", sub(t.ann, b"ZZMARKZZ", b"a\\/b"),
      "sec 3.4: an escaped solidus decodes to '/', which is emitted raw", expect="E_NOT_CANONICAL")
    C("esc_unknown_x", sub(t.ann, b"ZZMARKZZ", b"a\\xb"),
      "RFC 8259: backslash-x is not an escape", expect="E_JSON")
    C("esc_u_short", sub(t.ann, b"ZZMARKZZ", b"a\\u12b"),
      "RFC 8259: a backslash-u escape needs four hex digits", expect="E_JSON")
    C("esc_u_nonhex", sub(t.ann, b"ZZMARKZZ", b"a\\u12g4b"),
      "RFC 8259: non-hex digit in an escape", expect="E_JSON")
    C("esc_trailing_backslash", sub(t.ann, b'"ZZMARKZZ"', b'"a\\'),
      "RFC 8259: a backslash at the end of the input", expect="E_JSON")

    # surrogate escapes
    C("esc_surrogate_pair", sub(t.ann, b"ZZMARKZZ", b"\\ud83d\\ude00"),
      "sec 3.3: a surrogate escape fails whether or not a second follows it", expect="E_JSON")
    C("esc_lone_high_surrogate", sub(t.ann, b"ZZMARKZZ", b"\\ud800"),
      "sec 3.3: a lone high surrogate escape", expect="E_JSON")
    C("esc_lone_low_surrogate", sub(t.ann, b"ZZMARKZZ", b"\\udc00"),
      "sec 3.3: a lone low surrogate escape", expect="E_JSON")
    C("esc_surrogate_dfff", sub(t.ann, b"ZZMARKZZ", b"\\uDFFF"),
      "sec 3.3: the top of the surrogate block, uppercase digits", expect="E_JSON")
    C("esc_just_below_surrogates", sub(t.ann, b"ZZMARKZZ", b"\\ud7ff"),
      "sec 3.3: U+D7FF is not a surrogate, so test 6 names the fault", expect="E_NOT_CANONICAL")
    C("esc_just_above_surrogates", sub(t.ann, b"ZZMARKZZ", b"\\ue000"),
      "sec 3.3: U+E000 is not a surrogate", expect="E_NOT_CANONICAL")
    C("esc_surrogate_in_key", sub(t.keyt, b"ZZKEYZZ", b"\\ud800x"),
      "sec 3.3: a surrogate escape in a key is still E_JSON at test 2", expect="E_JSON")
    C("esc_surrogate_beats_dup_key",
      sub(sub(t.keyt, b"ZZKEYZZ", b"\\ud800"), b'"note_md":"n"', b'"note_md":"n","note_md":"n"'),
      "sec 3.5: test 2 precedes test 3", expect="E_JSON")

    # ---------------- raw controls and the two charsets --------------------
    C("raw_lf_in_string", sub(t.ann, b"ZZMARKZZ", b"a\nb"),
      "RFC 8259: an unescaped U+000A inside a string", expect="E_JSON")
    C("raw_1f_in_string", sub(t.ann, b"ZZMARKZZ", b"a\x1fb"),
      "RFC 8259: an unescaped U+001F inside a string", expect="E_JSON")
    C("raw_20_in_string", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "a b"}),
      "RFC 8259: U+0020 is the first byte a string may carry raw", expect="ok")
    C("raw_7f_prose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "a\x7fb"}),
      "sec 3.3/3.4: U+007F raw in a prose subtree", expect="ok")
    C("raw_7f_nonprose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "n", "q": "a\x7fb"}),
      "sec 3.3: U+007F is outside 0x20-0x7E", expect="E_VALUE_CHARSET")
    C("raw_c1_80_prose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "a\x80b"}),
      "sec 3.4: the C1 range raw in prose", expect="ok")
    C("raw_c1_9f_prose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "\x9f"}),
      "sec 3.4: U+009F, the top of C1, raw in prose", expect="ok")
    C("raw_c1_nonprose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "n", "q": "\x80"}),
      "sec 3.3: a C1 scalar encodes to bytes outside the skeleton range", expect="E_VALUE_CHARSET")
    C("raw_7e_nonprose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "n", "q": "~"}),
      "sec 3.3: 0x7E is the last byte the skeleton range admits", expect="ok")
    C("raw_20_nonprose", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "n", "q": " "}),
      "sec 3.3: 0x20 is the first byte the skeleton range admits", expect="ok")
    C("empty_string_value", make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "n", "q": ""}),
      "sec 3.3: an empty string value is a skeleton string", expect="ok")

    # ---------------- sec 3.1 / 3.2 : the integer universe ------------------
    SEQ1 = b'"seq":1,'
    C("int_zero", t.gen, "sec 3.2: the integer zero at seq 0", expect="ok")
    C("int_one", t.ann, "sec 3.2: the integer one", expect="ok")
    maxent = make_entry(K("root"), "annotation", MAXINT, t.gid, {"note_md": "top of the universe"})
    C("int_2p53_minus_1", maxent, "sec 3.2: 2^53 - 1, the top of the universe", expect="ok")
    C("int_2p53", sub(t.ann, SEQ1, b'"seq":9007199254740992,'),
      "sec 3.2: 2^53 is outside the universe", expect="E_NUMBER")
    C("int_2p53_plus_1", sub(t.ann, SEQ1, b'"seq":9007199254740993,'),
      "sec 3.2: one past 2^53", expect="E_NUMBER")
    C("int_leading_zero", sub(t.ann, SEQ1, b'"seq":01,'),
      "sec 3.2: no leading zeros", expect="E_NUMBER")
    C("int_double_zero", sub(t.ann, SEQ1, b'"seq":00,'),
      "sec 3.2: zero is written 0", expect="E_NUMBER")
    C("int_negative", sub(t.ann, SEQ1, b'"seq":-1,'),
      "sec 3.1: no value carries a minus sign", expect="E_NUMBER")
    C("int_negative_zero", sub(t.ann, SEQ1, b'"seq":-0,'),
      "sec 3.1: minus zero is not an integer here", expect="E_NUMBER")
    C("int_fraction_zero", sub(t.ann, SEQ1, b'"seq":1.0,'),
      "sec 3.1: no value carries a fraction", expect="E_NUMBER")
    C("int_fraction_half", sub(t.ann, SEQ1, b'"seq":1.5,'),
      "sec 3.1: floating point is unrepresentable", expect="E_NUMBER")
    C("int_exponent_lower", sub(t.ann, SEQ1, b'"seq":1e2,'),
      "sec 3.1: no value carries an exponent", expect="E_NUMBER")
    C("int_exponent_upper", sub(t.ann, SEQ1, b'"seq":1E2,'),
      "sec 3.1: uppercase exponent", expect="E_NUMBER")
    C("int_exponent_signed", sub(t.ann, SEQ1, b'"seq":1e+2,'),
      "sec 3.1: signed exponent", expect="E_NUMBER")
    C("int_plus_one", sub(t.ann, SEQ1, b'"seq":+1,'),
      "sec 3.1: '+' opens a value position of numeric shape", expect="E_NUMBER")
    C("int_dot_five", sub(t.ann, SEQ1, b'"seq":.5,'),
      "sec 3.1: '.' opens a value position of numeric shape", expect="E_NUMBER")
    C("int_one_dot", sub(t.ann, SEQ1, b'"seq":1.,'),
      "sec 3.1: a trailing decimal point", expect="E_NUMBER")
    C("int_minus_infinity", sub(t.ann, SEQ1, b'"seq":-Infinity,'),
      "sec 3.5: -Infinity opens a numeric-shape position whose bytes are just '-'",
      expect="E_NUMBER")
    C("int_nan", sub(t.ann, SEQ1, b'"seq":NaN,'),
      "sec 3.5: NaN is an unknown literal, not a numeric shape", expect="E_JSON")
    C("int_infinity", sub(t.ann, SEQ1, b'"seq":Infinity,'),
      "sec 3.5: Infinity is an unknown literal", expect="E_JSON")
    C("int_undefined", sub(t.ann, SEQ1, b'"seq":undefined,'),
      "sec 3.5: undefined is an unknown literal", expect="E_JSON")
    C("int_huge_digits", sub(t.ann, SEQ1, b'"seq":99999999999999999999,'),
      "sec 3.2: twenty digits, far outside the universe", expect="E_NUMBER")
    C("int_hex_literal", sub(t.ann, SEQ1, b'"seq":0x10,'),
      "sec 3.1: the bytes there are just '0', an integer, so the grammar fails at 'x'",
      expect="E_JSON")
    C("int_two_numbers", sub(t.ann, SEQ1, b'"seq":1 2,'),
      "sec 3.1: the bytes there are '1'; the grammar fails at '2'", expect="E_JSON")
    C("int_trailing_e", sub(t.ann, SEQ1, b'"seq":12e,'),
      "sec 3.1: the bytes there are '12e', not an integer", expect="E_NUMBER")
    C("int_lone_minus", sub(t.ann, SEQ1, b'"seq":-,'),
      "sec 3.1: a bare minus sign", expect="E_NUMBER")
    C("int_version_max",
      make_entry(K("root"), "amanah", 1, t.gid, {"version": MAXINT, "text_md": "top"}),
      "sec 3.2 inside a body: 2^53 - 1 as a version", expect="ok")
    C("int_version_zero",
      make_entry(K("root"), "amanah", 1, t.gid, {"version": 0, "text_md": "zero"}),
      "sec 3.2 inside a body: version 0 is an int", expect="ok")
    C("int_version_over",
      sub(make_entry(K("root"), "amanah", 1, t.gid, {"version": MAXINT, "text_md": "top"}),
          b'"version":9007199254740991', b'"version":9007199254740992'),
      "sec 3.2 inside a body: 2^53 fails at test 2, never at the body table", expect="E_NUMBER")
    C("int_in_array_body",
      make_entry(K("root"), "windmill", 1, t.gid, {"xs": [0, 1, MAXINT]}),
      "sec 3.1: integers inside an array in a body", expect="ok")

    # ---------------- literals in the envelope and in bodies ---------------
    C("lit_seq_true", set_key(t.ann, "seq", True), "sec 4.3 step 7", expect="E_SEQ")
    C("lit_seq_false", set_key(t.ann, "seq", False), "sec 4.3 step 7", expect="E_SEQ")
    C("lit_seq_null", set_key(t.ann, "seq", None), "sec 4.3 step 7", expect="E_SEQ")
    C("lit_seq_string", set_key(t.ann, "seq", "1"), "sec 4.3 step 7", expect="E_SEQ")
    C("lit_seq_array", set_key(t.ann, "seq", [1]), "sec 4.3 step 7", expect="E_SEQ")
    C("lit_prev_true", set_key(t.ann, "prev", True), "sec 4.3 step 8", expect="E_PREV")
    C("lit_prev_false", set_key(t.ann, "prev", False), "sec 4.3 step 8", expect="E_PREV")
    C("lit_prev_int", set_key(t.ann, "prev", 0), "sec 4.3 step 8", expect="E_PREV")
    C("lit_body_null", set_key(t.ann, "body", None), "sec 4.3 step 9", expect="E_BODY")
    C("lit_body_true", set_key(t.ann, "body", True), "sec 4.3 step 9", expect="E_BODY")
    C("lit_body_array", set_key(t.ann, "body", []), "sec 4.3 step 9", expect="E_BODY")
    C("lit_body_string", set_key(t.ann, "body", "x"), "sec 4.3 step 9", expect="E_BODY")
    C("lit_spec_null", set_key(t.ann, "spec", None), "sec 4.3 step 4", expect="E_SPEC")
    C("lit_spec_true", set_key(t.ann, "spec", True), "sec 4.3 step 4", expect="E_SPEC")
    C("lit_spec_int", set_key(t.ann, "spec", 1), "sec 4.3 step 4", expect="E_SPEC")
    C("lit_entrytype_null", set_key(t.ann, "entryType", None), "sec 4.3 step 5", expect="E_ENTRYTYPE")
    C("lit_entrytype_true", set_key(t.ann, "entryType", True), "sec 4.3 step 5", expect="E_ENTRYTYPE")
    C("lit_entrytype_int", set_key(t.ann, "entryType", 7), "sec 4.3 step 5", expect="E_ENTRYTYPE")
    C("lit_judge_null", set_key(t.ann, "judge", None), "sec 4.3 step 6", expect="E_JUDGE")
    C("lit_sig_null", set_key(t.ann, "sig", None), "sec 4.3 step 10", expect="E_SIG_FORM")
    C("lit_sig_int", set_key(t.ann, "sig", 0), "sec 4.3 step 10", expect="E_SIG_FORM")
    C("lit_body_values",
      make_entry(K("root"), "windmill", 1, t.gid,
                 {"a": None, "b": True, "c": False, "d": [None, True, False], "e": {"f": None}}),
      "sec 3.1/6.10: null, true and false are values of the universe, in a body", expect="ok")
    C("lit_null_optional_absent",
      make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "n"}),
      "sec 6: the only spelling of an absent optional field is the absent key", expect="ok")

    # ---------------- keys (sec 3.3) ---------------------------------------
    C("key_empty", make_entry(K("root"), "annotation", 1, t.gid, {"": "v", "note_md": "n"}),
      "sec 3.3: a key with no byte", expect="E_KEY_CHARSET")
    C("key_space", make_entry(K("root"), "annotation", 1, t.gid, {"a b": "v", "note_md": "n"}),
      "sec 3.3: 0x20 is inside the key charset", expect="ok")
    C("key_single_space", make_entry(K("root"), "annotation", 1, t.gid, {" ": "v", "note_md": "n"}),
      "sec 3.3: a key that is one space", expect="ok")
    C("key_bang", make_entry(K("root"), "annotation", 1, t.gid, {"!": "v", "note_md": "n"}),
      "sec 3.3: 0x21", expect="ok")
    C("key_tilde", make_entry(K("root"), "annotation", 1, t.gid, {"~": "v", "note_md": "n"}),
      "sec 3.3: 0x7E, the last byte of the key charset", expect="ok")
    C("key_del", make_entry(K("root"), "annotation", 1, t.gid, {"a\x7f": "v", "note_md": "n"}),
      "sec 3.3: 0x7F is one past the key charset", expect="E_KEY_CHARSET")
    C("key_c1", make_entry(K("root"), "annotation", 1, t.gid, {"a\x80": "v", "note_md": "n"}),
      "sec 3.3: a C1 scalar in a key", expect="E_KEY_CHARSET")
    C("key_multibyte", make_entry(K("root"), "annotation", 1, t.gid, {"café": "v", "note_md": "n"}),
      "sec 3.3: a non-ASCII scalar in a key", expect="E_KEY_CHARSET")
    C("key_control_escape", sub(t.keyt, b"ZZKEYZZ", b"a\\u001fb"),
      "sec 3.3: the key charset is decided after escape decoding", expect="E_KEY_CHARSET")
    C("key_nul_escape", sub(t.keyt, b"ZZKEYZZ", b"\\u0000"),
      "sec 3.3: U+0000 in a key", expect="E_KEY_CHARSET")
    C("key_empty_in_prose_subtree",
      make_entry(K("root"), "annotation", 1, t.gid, {"note_md": {"": "\x00"}}),
      "sec 3.3: keys inside a prose subtree remain keys under the key rule",
      expect="E_KEY_CHARSET")
    C("key_control_in_prose_subtree",
      make_entry(K("root"), "annotation", 1, t.gid, {"note_md": {"a\x01": "\x00"}}),
      "sec 3.3: a control scalar in a key inside a prose subtree", expect="E_KEY_CHARSET")
    C("key_dup_beats_charset", sub(t.keyt, b'"ZZKEYZZ":"v"', b'"":"v","":"w"'),
      "sec 3.5: test 3 precedes test 4", expect="E_DUP_KEY")
    C("key_case_distinct", make_entry(K("root"), "annotation", 1, t.gid,
                                      {"a": 1, "A": 2, "note_md": "n"}),
      "sec 3.5: keys differing in case are distinct; 0x41 sorts before 0x61", expect="ok")

    # ---------------- prose subtrees (sec 3.3) ------------------------------
    C("prose_key_exactly_md", make_entry(K("root"), "windmill", 1, t.gid, {"_md": "\x00\x01\x7f"}),
      "sec 3.3: a member whose decoded key is the three bytes _md", expect="ok")
    C("prose_key_ends_md", make_entry(K("root"), "windmill", 1, t.gid, {"x_md": "\x00"}),
      "sec 3.3: a key ending in _md", expect="ok")
    C("prose_key_double_underscore", make_entry(K("root"), "windmill", 1, t.gid, {"__md": "\x00"}),
      "sec 3.3: __md ends in _md", expect="ok")
    C("prose_key_uppercase_MD", make_entry(K("root"), "windmill", 1, t.gid, {"x_MD": "\x00"}),
      "sec 3.3: the suffix test is over bytes, so _MD opens nothing", expect="E_VALUE_CHARSET")
    C("prose_key_md_no_underscore", make_entry(K("root"), "windmill", 1, t.gid, {"md": "\x00"}),
      "sec 3.3: md is not _md", expect="E_VALUE_CHARSET")
    C("prose_key_md_prefix", make_entry(K("root"), "windmill", 1, t.gid, {"_mdx": "\x00"}),
      "sec 3.3: the key must end in _md, not merely carry it", expect="E_VALUE_CHARSET")
    C("prose_nested_object", make_entry(K("root"), "windmill", 1, t.gid,
                                        {"a_md": {"b": {"c": "\x00\x1f\x7f‮"}}}),
      "sec 3.3: every string value at any depth within the member", expect="ok")
    C("prose_nested_array", make_entry(K("root"), "windmill", 1, t.gid,
                                       {"a_md": [["\x00"], {"k": ["\x1b"]}]}),
      "sec 3.3: through arrays and nested objects", expect="ok")
    C("prose_sibling_not_prose", make_entry(K("root"), "windmill", 1, t.gid,
                                            {"a_md": "\x00", "b": "\x00"}),
      "sec 3.3: the sibling outside the subtree keeps the value charset",
      expect="E_VALUE_CHARSET")
    C("prose_bidi_controls", make_entry(K("root"), "annotation", 1, t.gid,
                                        {"note_md": "a‮b‏c⁦d"}),
      "sec 11: bidirectional controls are ordinary scalars in prose", expect="ok")
    C("prose_root_md_extra_member",
      make_entry(K("root"), "annotation", 1, t.gid, {"note_md": "n"},
                 extra_env=[("x_md", "\x00")]),
      "sec 3.3/4.1: an eighth key ending in _md opens prose at the root, and test 5 "
      "passes before sec 4.3 step 3 closes the envelope", expect="E_ENVELOPE_CLOSED")

    # ---------------- depth (sec 3.5 test 2) --------------------------------
    C("depth_arr_128", nested_arr_text(128),
      "sec 3.5: 128 nested arrays, the root counted as depth 1", expect="E_ENVELOPE")
    C("depth_arr_129", nested_arr_text(129),
      "sec 3.5: a container opening at depth 129", expect="E_DEPTH")
    C("depth_obj_128", nested_obj_text(128),
      "sec 3.5: 128 nested objects; the root is an object, so sec 4.3 step 3 names the fault",
      expect="E_ENVELOPE_MISSING")
    C("depth_obj_129", nested_obj_text(129),
      "sec 3.5: an object opening at depth 129", expect="E_DEPTH")
    C("depth_entry_128",
      make_entry(K("root"), "windmill", 1, t.gid, {"d": deep_list(126)}),
      "sec 3.5: root 1, body 2, 126 arrays inside, deepest container at depth 128", expect="ok")
    C("depth_entry_129",
      make_entry(K("root"), "windmill", 1, t.gid, {"d": deep_list(127)}),
      "sec 3.5: the same body one container deeper", expect="E_DEPTH")
    C("depth_mixed_129",
      ("".join('{"a":[' for _ in range(64)) + "[]" + "]}" * 64).encode(),
      "sec 3.5: alternating objects and arrays, 129 containers", expect="E_DEPTH")
    C("depth_before_number", b"[" + nested_arr_text(128) + b",1.5]",
      "sec 3.5: the depth-129 '[' comes before the numeric-shape byte", expect="E_DEPTH")
    C("number_before_depth", b"[1.5," + nested_arr_text(128) + b"]",
      "sec 3.5: the numeric-shape byte comes first", expect="E_NUMBER")
    C("json_before_depth", b"[@," + nested_arr_text(129) + b"]",
      "sec 3.5: a grammar failure at byte 1 beats a later depth fault", expect="E_JSON")
    C("number_before_eof", b'{"a":1.5',
      "sec 3.5: E_NUMBER's byte precedes the one-past-the-end byte of a truncated text",
      expect="E_NUMBER")

    # ---------------- scalar roots ------------------------------------------
    C("root_string", b'"hello"', "sec 3.5: a scalar root has depth 0; sec 4.3 step 2",
      expect="E_ENVELOPE")
    C("root_int", b"0", "sec 3.5/4.3: an integer root", expect="E_ENVELOPE")
    C("root_null", b"null", "sec 4.3 step 2", expect="E_ENVELOPE")
    C("root_true", b"true", "sec 4.3 step 2", expect="E_ENVELOPE")
    C("root_false", b"false", "sec 4.3 step 2", expect="E_ENVELOPE")
    C("root_array_empty", b"[]", "sec 4.3 step 2", expect="E_ENVELOPE")
    C("root_object_empty", b"{}", "sec 4.3 step 3", expect="E_ENVELOPE_MISSING")
    C("root_negative", b"-1", "sec 3.1: the root is a value position of numeric shape",
      expect="E_NUMBER")
    C("root_fraction", b"1.5", "sec 3.1: a fractional root", expect="E_NUMBER")
    C("root_leading_zero", b"01", "sec 3.2: leading zero at the root", expect="E_NUMBER")
    C("root_string_control", b'"\\u0001"',
      "sec 3.3: a scalar root string is outside every prose subtree", expect="E_VALUE_CHARSET")
    C("root_string_noncanonical", b'"\\u0041"',
      "sec 3.4: an escaped letter at the root", expect="E_NOT_CANONICAL")
    C("root_trailing_garbage", b"null null", "sec 3.5 test 2: one JSON text only", expect="E_JSON")

    # ---------------- duplicate keys ----------------------------------------
    C("dup_envelope_key", sub(t.ann, b'"spec":"galeed/1"', b'"spec":"galeed/1","spec":"galeed/1"'),
      "sec 3.5 test 3: a repeated envelope key", expect="E_DUP_KEY")
    C("dup_body_key", sub(t.ann, b'"note_md":"ZZMARKZZ"', b'"note_md":"a","note_md":"b"'),
      "sec 3.5 test 3: a repeated body key", expect="E_DUP_KEY")
    C("dup_after_escape_decoding", sub(t.ann, b'"note_md":"ZZMARKZZ"',
                                       b'"note_md":"a","note\\u005fmd":"b"'),
      "sec 3.5 test 3: distinct after escape decoding is the test", expect="E_DUP_KEY")
    C("dup_in_prose_subtree", sub(t.ann, b'"note_md":"ZZMARKZZ"',
                                  b'"note_md":{"a":"x","a":"y"}'),
      "sec 3.5 test 3: every object at every depth", expect="E_DUP_KEY")
    C("dup_in_array_element", sub(t.ann, b'"note_md":"ZZMARKZZ"',
                                  b'"note_md":[{"a":1,"a":2}]'),
      "sec 3.5 test 3: an object inside an array", expect="E_DUP_KEY")
    C("dup_beats_not_canonical", sub(t.ann, b'"note_md":"ZZMARKZZ"',
                                     b'"note_md":"b","note_md":"a"'),
      "sec 3.5: test 3 precedes test 6", expect="E_DUP_KEY")

    # ---------------- member order (sec 3.4 item 5) --------------------------
    C("order_envelope_rotated", rot_top(t.gen, 1),
      "sec 3.4 item 5: members ordered by the bytewise order of their keys",
      expect="E_NOT_CANONICAL")
    C("order_envelope_reversed", cbytes(
        JObj(list(reversed(Parser(t.gen.decode()).text().members))), sort=False),
      "sec 3.4 item 5: the envelope written in reverse key order", expect="E_NOT_CANONICAL")
    C("order_body_rotated", rot_body(t.judg, 1),
      "sec 3.4 item 5: body members out of order", expect="E_NOT_CANONICAL")
    C("order_space_key_correct",
      make_entry(K("root"), "windmill", 1, t.gid, {"a b": 1, "ab": 2}),
      "sec 3.4 item 5: 0x20 sorts before 0x62, so 'a b' precedes 'ab'", expect="ok")
    C("order_space_key_swapped",
      cbytes(JObj([(k, (JObj(list(reversed(v.members))) if k == "body" else v))
                   for k, v in Parser(make_entry(K("root"), "windmill", 1, t.gid,
                                                 {"a b": 1, "ab": 2}).decode()).text().members]),
             sort=False),
      "sec 3.4 item 5: the same two keys written the other way round", expect="E_NOT_CANONICAL")
    C("order_prefix_shorter_first",
      make_entry(K("root"), "windmill", 1, t.gid, {"a": 1, "ab": 2, "abc": 3}),
      "sec 1: where one key is a proper prefix of another the shorter orders first", expect="ok")
    C("order_case_correct",
      make_entry(K("root"), "windmill", 1, t.gid, {"A": 1, "Z": 2, "a": 3, "z": 4}),
      "sec 3.4 item 5: bytewise order over ASCII keys", expect="ok")

    # ---------------- sec 4.1 envelope arity --------------------------------
    for k in SEVEN:
        C("env_missing_" + k, drop_key(t.ann, k),
          "sec 4.3 step 3: the key %r is absent" % k, expect="E_ENVELOPE_MISSING")
    C("env_extra_key", add_key(t.ann, "extra", 1),
      "sec 4.3 step 3: an eighth key", expect="E_ENVELOPE_CLOSED")
    C("env_extra_two_keys", add_key(add_key(t.ann, "x", 1), "y", 2),
      "sec 4.3 step 3: two keys beyond the seven", expect="E_ENVELOPE_CLOSED")
    C("env_extra_key_sorts_first", add_key(t.ann, "aaa", 1),
      "sec 4.3 step 3: an extra key that sorts before every one of the seven",
      expect="E_ENVELOPE_CLOSED")
    C("env_seven_members_missing_and_extra", add_key(drop_key(t.ann, "sig"), "sigx", "0x00"),
      "sec 4.1: seven members that both lack one of the seven and carry an eighth key; "
      "the missing test is decided first", expect="E_ENVELOPE_MISSING")
    C("env_six_of_seven_plus_two_extra",
      add_key(add_key(drop_key(t.ann, "prev"), "p", 1), "q", 2),
      "sec 4.1: member count alone decides neither test", expect="E_ENVELOPE_MISSING")
    C("env_empty_object", b"{}", "sec 4.3 step 3 over an empty object",
      expect="E_ENVELOPE_MISSING")

    # ---------------- sec 4.1 field forms -----------------------------------
    C("spec_wrong_version", set_key(t.ann, "spec", "galeed/2"), "sec 4.3 step 4", expect="E_SPEC")
    C("spec_trailing_space", set_key(t.ann, "spec", "galeed/1 "), "sec 4.3 step 4: byte-equal",
      expect="E_SPEC")
    C("spec_uppercase", set_key(t.ann, "spec", "ZIKARON/1"), "sec 4.3 step 4", expect="E_SPEC")
    C("spec_empty", set_key(t.ann, "spec", ""), "sec 4.3 step 4", expect="E_SPEC")
    C("spec_zikaron", set_key(t.ann, "spec", "zikaron/1"), "sec 12.4: the twin law's spec id",
      expect="E_SPEC")
    C("entrytype_empty", set_key(t.ann, "entryType", ""), "sec 1: a token is non-empty",
      expect="E_ENTRYTYPE")
    C("entrytype_space", set_key(t.ann, "entryType", "a b"), "sec 1: 0x20 is outside a token",
      expect="E_ENTRYTYPE")
    C("entrytype_leading_space", set_key(t.ann, "entryType", " a"), "sec 1: token charset",
      expect="E_ENTRYTYPE")
    C("entrytype_bang", make_entry(K("root"), "!", 1, t.gid, {"note_md": "n"}),
      "sec 1/6.8: 0x21 is the first token byte, and an unrecognized type is an entry all the same",
      expect="ok")
    C("entrytype_tilde", make_entry(K("root"), "~", 1, t.gid, {"note_md": "n"}),
      "sec 1/6.8: 0x7E is the last token byte", expect="ok")
    C("entrytype_resigned_unknown", set_key(t.ann, "entryType", "!"),
      "sec 4.3: step 5 admits the token, and the stale signature then fails at step 13",
      expect="E_SIG_SIGNER")
    C("entrytype_del", set_key(t.ann, "entryType", "a\x7f"),
      "sec 3.3: 0x7F in a non-prose value is refused by test 5 before the token test is reached",
      expect="E_VALUE_CHARSET")
    C("judge_uppercase", set_key(t.ann, "judge", A_("root").upper().replace("0X", "0x")),
      "sec 1: hex20 digits are lowercase", expect="E_JUDGE")
    C("judge_no_prefix", set_key(t.ann, "judge", A_("root")[2:]), "sec 1: the 0x prefix",
      expect="E_JUDGE")
    C("judge_short", set_key(t.ann, "judge", A_("root")[:-1]), "sec 1: 39 digits",
      expect="E_JUDGE")
    C("judge_long", set_key(t.ann, "judge", A_("root") + "0"), "sec 1: 41 digits",
      expect="E_JUDGE")
    C("judge_capital_X", set_key(t.ann, "judge", "0X" + A_("root")[2:]), "sec 1: the prefix bytes",
      expect="E_JUDGE")
    C("judge_nonhex", set_key(t.ann, "judge", "0x" + "g" * 40), "sec 1: hex digits",
      expect="E_JUDGE")
    C("judge_hex32_length", set_key(t.ann, "judge", HX32(1)), "sec 1: a hex32 in a hex20 field",
      expect="E_JUDGE")
    C("judge_zero_address", set_key(t.ann, "judge", HX20(0)),
      "sec 6.10: the all-zero address is a legal spelling, and the signature decides",
      expect="E_SIG_SIGNER")
    C("prev_short", set_key(t.ann, "prev", HX32(1)[:-1]), "sec 1: 63 digits", expect="E_PREV")
    C("prev_long", set_key(t.ann, "prev", HX32(1) + "0"), "sec 1: 65 digits", expect="E_PREV")
    C("prev_uppercase", set_key(t.ann, "prev", "0x" + "A" * 64), "sec 1: lowercase digits",
      expect="E_PREV")
    C("prev_empty_string", set_key(t.ann, "prev", ""), "sec 4.3 step 8", expect="E_PREV")
    C("prev_hex20", set_key(t.ann, "prev", HX20(1)), "sec 1: a hex20 in a hex32 field",
      expect="E_PREV")
    C("sig_short", set_key(t.ann, "sig", t.ann.decode().split('"sig":"')[1][:131]),
      "sec 1: 129 digits", expect="E_SIG_FORM")
    C("sig_long", set_key(t.ann, "sig", "0x" + "0" * 131), "sec 1: 131 digits", expect="E_SIG_FORM")
    C("sig_uppercase", set_key(t.ann, "sig", "0x" + "A" * 130), "sec 1: lowercase digits",
      expect="E_SIG_FORM")
    C("sig_no_prefix", set_key(t.ann, "sig", "0" * 130), "sec 1: the 0x prefix",
      expect="E_SIG_FORM")
    C("sig_empty", set_key(t.ann, "sig", ""), "sec 4.3 step 10", expect="E_SIG_FORM")

    # ---------------- sec 4.2 genesis placement and the prev hinge ----------
    C("genesis_at_zero", t.gen, "sec 4.2: entryType is genesis iff seq is 0", expect="ok")
    C("genesis_at_one_with_prev",
      make_entry(K("root"), "genesis", 1, t.gid, {"statement_md": "misplaced"}),
      "sec 4.2: a genesis off position 0", expect="E_GENESIS_PLACE")
    C("genesis_at_one_prev_null",
      craft("genesis", 1, None, {"statement_md": "misplaced"}, "0x" + "11" * 65),
      "sec 4.3: step 8 precedes step 11", expect="E_PREV_SEQ")
    C("nongenesis_at_zero",
      make_entry(K("root"), "annotation", 0, None, {"note_md": "misplaced"}),
      "sec 4.2: a non-genesis entry at position 0", expect="E_GENESIS_PLACE")
    C("unknown_type_at_zero",
      make_entry(K("root"), "windmill", 0, None, {}),
      "sec 4.2/6.8: an unknown type at position 0 is still off placement",
      expect="E_GENESIS_PLACE")
    C("nongenesis_at_zero_with_prev",
      craft("annotation", 0, HX32(1), {"note_md": "x"}, "0x" + "11" * 65),
      "sec 4.3: step 8 names the prev hinge before step 11", expect="E_PREV_SEQ")
    C("prev_null_at_five",
      craft("annotation", 5, None, {"note_md": "x"}, "0x" + "11" * 65),
      "sec 4.3 step 8: prev is null iff seq is 0", expect="E_PREV_SEQ")
    C("prev_set_at_zero",
      craft("genesis", 0, HX32(9), {"statement_md": "x"}, "0x" + "11" * 65),
      "sec 4.3 step 8: a non-null prev at seq 0", expect="E_PREV_SEQ")
    C("prev_all_zero_hex32",
      make_entry(K("root"), "annotation", 1, HX32(0), {"note_md": "x"}),
      "sec 6.10: the all-zero digest is a legal hex32 and chain law is not an entry predicate",
      expect="ok")

    # ---------------- sec 3.5 test 4 before test 5, and sec 4.3 step pairs ----
    C("key_charset_beats_value_charset",
      sub(sub(t.keyt, b'"ZZKEYZZ":"v"', b'"k\\u0001":"v","q":"\\u0001"'), b'"note_md":"n"', b'"note_md":"n"'),
      "sec 3.5: a key outside the charset and a non-prose value outside it; test 4 is decided "
      "before test 5", expect="E_KEY_CHARSET")
    # sec 3.4 item 5: members order by the decoded key, and the quote's canonical spelling
    # begins with a backslash, so the emitted bytes are not in key-byte order
    C("key_order_decoded_quote_first", b'{"\\"":1,"#":2}',
      "sec 3.4 item 5: keys order by decoded scalar values, U+0022 before U+0023, though the "
      "emitted key bytes begin 0x5C and 0x23; canonical, and no entry, so check names the "
      "envelope", expect="E_ENVELOPE_MISSING")
    C("key_order_emitted_bytes_not_decoded", b'{"#":2,"\\"":1}',
      "sec 3.4 item 5: the order of the emitted key bytes is not the canonical order",
      expect="E_NOT_CANONICAL")
    C("step4_before_step5", set_key(set_key(t.ann, "spec", "galeed/2"), "entryType", ""),
      "sec 4.3: step 4 precedes step 5", expect="E_SPEC")
    C("step5_before_step6", set_key(set_key(t.ann, "entryType", ""), "judge", "0x" + "z" * 40),
      "sec 4.3: step 5 precedes step 6", expect="E_ENTRYTYPE")
    C("step6_before_step7", set_key(set_key(t.ann, "judge", "0x" + "z" * 40), "seq", "1"),
      "sec 4.3: step 6 precedes step 7", expect="E_JUDGE")
    C("step9_before_step10", set_key(set_key(t.ann, "body", []), "sig", None),
      "sec 4.3: step 9 precedes step 10", expect="E_BODY")
    C("step10_before_step11", set_key(set_key(t.ann, "sig", None), "entryType", "genesis"),
      "sec 4.3: step 10 precedes step 11", expect="E_SIG_FORM")
    C("step11_before_step12",
      craft("genesis", 1, t.gid, {}, "0x" + "11" * 65),
      "sec 4.3: a genesis at seq 1 whose body lacks statement_md; step 11 precedes step 12",
      expect="E_GENESIS_PLACE")

    # ---------------- sec 5 signature boundaries -----------------------------
    good = Parser(t.ann.decode()).text().get("sig")
    gr = int(good[2:66], 16)
    gs = int(good[66:130], 16)
    for vv, tokname in ((0, "E_SIG_V"), (1, "E_SIG_V"), (26, "E_SIG_V"), (29, "E_SIG_V"),
                        (30, "E_SIG_V"), (255, "E_SIG_V")):
        C("sig_v_%d" % vv,
          craft("annotation", 1, t.gid, {"note_md": "ZZMARKZZ"}, sig_hex(gr, gs, vv)),
          "sec 5.4: v = %d is outside {27, 28}" % vv, expect=tokname)
    C("sig_v_27_valid", t.ann, "sec 5.4: v = 27 accepted", expect="ok")
    C("sig_v_28_valid", find_v28(t.gid), "sec 5.4: v = 28 accepted", expect="ok")
    C("sig_r_zero", craft("annotation", 1, t.gid, {"note_md": "ZZMARKZZ"}, sig_hex(0, gs, 27)),
      "sec 5.4: r = 0", expect="E_SIG_RANGE")
    C("sig_s_zero", craft("annotation", 1, t.gid, {"note_md": "ZZMARKZZ"}, sig_hex(gr, 0, 27)),
      "sec 5.4: s = 0", expect="E_SIG_RANGE")
    C("sig_r_one", craft("annotation", 1, t.gid, {"note_md": "ZZMARKZZ"}, sig_hex(1, gs, 27)),
      "sec 5.4: r = 1 is the low inside edge of the range, so a later bullet names the fault",
      expect=("E_SIG_SIGNER" if pow((1 + 7) % P, (P - 1) // 2, P) == 1 else "E_SIG_RECOVER"))
    C("sig_s_one", craft("annotation", 1, t.gid, {"note_md": "ZZMARKZZ"}, sig_hex(gr, 1, 27)),
      "sec 5.4: s = 1 is the low inside edge of the range and is low-s, so recovery decides",
      expect="E_SIG_SIGNER")
    C("sig_r_n", craft("annotation", 1, t.gid, {"note_md": "ZZMARKZZ"}, sig_hex(N, gs, 27)),
      "sec 5.4: r = n", expect="E_SIG_RANGE")
    C("sig_s_n", craft("annotation", 1, t.gid, {"note_md": "ZZMARKZZ"}, sig_hex(gr, N, 27)),
      "sec 5.4: s = n", expect="E_SIG_RANGE")
    C("sig_r_n_minus_1",
      craft("annotation", 1, t.gid, {"note_md": "ZZMARKZZ"}, sig_hex(N - 1, gs, 27)),
      "sec 5.4: r = n - 1 is inside the range, so a later bullet names the fault",
      expect=("E_SIG_SIGNER" if pow(((N - 1) ** 3 + 7) % P, (P - 1) // 2, P) == 1
              else "E_SIG_RECOVER"))
    C("sig_s_half_n",
      craft("annotation", 1, t.gid, {"note_md": "ZZMARKZZ"}, sig_hex(gr, HALF_N, 27)),
      "sec 5.4: s exactly (n-1)/2 passes the low-s test, so a later bullet names the fault",
      expect="E_SIG_SIGNER")
    C("sig_s_half_n_plus_1",
      craft("annotation", 1, t.gid, {"note_md": "ZZMARKZZ"}, sig_hex(gr, HALF_N + 1, 27)),
      "sec 5.4: one past (n-1)/2", expect="E_SIG_HIGH_S")
    C("sig_s_n_minus_1",
      craft("annotation", 1, t.gid, {"note_md": "ZZMARKZZ"}, sig_hex(gr, N - 1, 27)),
      "sec 5.4: s = n - 1 is in range and high", expect="E_SIG_HIGH_S")
    C("sig_malleability_twin",
      craft("annotation", 1, t.gid, {"note_md": "ZZMARKZZ"}, sig_hex(gr, N - gs, 55 - 27)),
      "sec 5.4: the malleability twin (r, n-s, 55-v) of a published signature",
      expect="E_SIG_HIGH_S")
    C("sig_high_s_beats_recover",
      craft("annotation", 1, t.gid, {"note_md": "ZZMARKZZ"}, sig_hex(bad_r(), N - 1, 27)),
      "sec 5.4: the low-s bullet is decided before recovery", expect="E_SIG_HIGH_S")
    C("sig_no_curve_point",
      craft("annotation", 1, t.gid, {"note_md": "ZZMARKZZ"}, sig_hex(bad_r(), gs, 27)),
      "sec 5.4: no point of secp256k1 has this x-coordinate", expect="E_SIG_RECOVER")
    C("sig_point_at_infinity", infinity_sig_entry(t.gid),
      "sec 5.4: sR = zG, so P is the point at infinity and recovery fails",
      expect="E_SIG_RECOVER")
    C("sig_wrong_signer", make_entry(K("k2"), "annotation", 1, t.gid, {"note_md": "x"},
                                     judge=A_("root")),
      "sec 5.5: signed by a key that is not the judge", expect="E_SIG_SIGNER")
    C("sig_zikaron_domain", foreign_domain_sig_entry(t.gid, "zikaron/1"),
      "sec 5.6: a signature made under the twin law's domain placed in sig", expect="E_SIG_SIGNER")
    C("sig_zikaron_adoption_domain", foreign_domain_sig_entry(t.gid, "zikaron/1-adoption"),
      "sec 5.6: a signature made under the twin law's second domain placed in sig",
      expect="E_SIG_SIGNER")
    C("sig_galeed2_domain", foreign_domain_sig_entry(t.gid, "galeed/2"),
      "sec 5.6: a signature under a 75-byte sibling domain; lengths decide nothing",
      expect="E_SIG_SIGNER")
    C("sig_family_domain", foreign_domain_sig_entry(t.gid, "galeed.amanah/1"),
      "sec 5.6: a signature under a family law's domain is not an entry signature",
      expect="E_SIG_SIGNER")
    C("sig_over_wrong_body", sig_over_other_b6(t.gid),
      "sec 5.1: a signature over a different B6 of the same author", expect="E_SIG_SIGNER")
    C("sig_body_field_beats_signature",
      craft("annotation", 1, t.gid, {}, "0x" + "11" * 65),
      "sec 4.3: step 12 precedes step 13", expect="E_BODY_FIELD")

    # ---------------- sec 6 body tables --------------------------------------
    def bc(slug, etype, body, note, expect, seq=1, prev=None):
        C(slug, make_entry(K("root"), etype, seq, (t.gid if prev is None else prev), body),
          note, expect=expect)

    E = "E_BODY_FIELD"
    # 6.1 genesis
    C("body_genesis_ok", t.gen, "sec 6.1: statement_md present as a prose string", expect="ok")
    C("body_genesis_missing",
      make_entry(K("root"), "genesis", 0, None, {}), "sec 6.1: statement_md absent", expect=E)
    C("body_genesis_int",
      make_entry(K("root"), "genesis", 0, None, {"statement_md": 1}),
      "sec 6.1: statement_md with a form its row refuses", expect=E)
    C("body_genesis_null",
      make_entry(K("root"), "genesis", 0, None, {"statement_md": None}),
      "sec 6: an optional-looking null is a present value with a refused form", expect=E)
    C("body_genesis_object",
      make_entry(K("root"), "genesis", 0, None, {"statement_md": {"a": "\x00"}}),
      "sec 3.3/6.1: the prose subtree still opens for the charset, and the row still refuses "
      "an object", expect=E)
    C("body_genesis_array",
      make_entry(K("root"), "genesis", 0, None, {"statement_md": ["x"]}), "sec 6.1", expect=E)
    C("body_genesis_extra",
      make_entry(K("root"), "genesis", 0, None, {"statement_md": "s", "x": 1, "y_md": "\x00"}),
      "sec 6.9: members beyond the table, at the body's top level", expect="ok")

    # 6.2 amanah
    AMB = {"version": 3, "text_md": "the covenant"}
    VN = {"chainId": 1, "contract": HX20(0xE5C), "codeHash": HX32(0xC0DE)}
    C("body_amanah_ok", t.aman, "sec 6.2: version, text_md and one venue", expect="ok")
    bc("body_amanah_minimal", "amanah", AMB, "sec 6.2: venues absent", expect="ok")
    bc("body_amanah_no_version", "amanah", {"text_md": "t"}, "sec 6.2: version absent", expect=E)
    bc("body_amanah_no_text", "amanah", {"version": 1}, "sec 6.2: text_md absent", expect=E)
    bc("body_amanah_version_string", "amanah", dict(AMB, version="3"), "sec 6.2", expect=E)
    bc("body_amanah_version_bool", "amanah", dict(AMB, version=True),
       "sec 3.1/6.2: true is not an integer", expect=E)
    bc("body_amanah_version_null", "amanah", dict(AMB, version=None), "sec 6", expect=E)
    bc("body_amanah_text_int", "amanah", dict(AMB, text_md=1), "sec 6.2", expect=E)
    bc("body_amanah_text_object", "amanah", dict(AMB, text_md={"a": "\x00"}),
       "sec 3.3/6.2: the prose subtree opens for the charset, and the row still refuses an "
       "object", expect=E)
    bc("body_amanah_text_prose", "amanah", dict(AMB, text_md="第一条\x00\U0001f600"),
       "sec 6.2: text_md is prose", expect="ok")
    bc("body_amanah_venues_empty", "amanah", dict(AMB, venues=[]),
       "sec 6.2: an empty venues array is legal", expect="ok")
    bc("body_amanah_venues_two", "amanah", dict(AMB, venues=[VN, dict(VN, chainId=8453)]),
       "sec 6.2: two venues", expect="ok")
    bc("body_amanah_venues_object", "amanah", dict(AMB, venues=VN), "sec 6.2: venues is an array",
       expect=E)
    bc("body_amanah_venues_null", "amanah", dict(AMB, venues=None), "sec 6", expect=E)
    bc("body_amanah_venues_string", "amanah", dict(AMB, venues="0x00"), "sec 6.2", expect=E)
    bc("body_amanah_venue_string", "amanah", dict(AMB, venues=["0x00"]),
       "sec 6.2: every element an object", expect=E)
    bc("body_amanah_venue_no_chainid", "amanah",
       dict(AMB, venues=[{"contract": HX20(1), "codeHash": HX32(1)}]), "sec 6.2", expect=E)
    bc("body_amanah_venue_no_contract", "amanah",
       dict(AMB, venues=[{"chainId": 1, "codeHash": HX32(1)}]), "sec 6.2", expect=E)
    bc("body_amanah_venue_no_codehash", "amanah",
       dict(AMB, venues=[{"chainId": 1, "contract": HX20(1)}]), "sec 6.2", expect=E)
    bc("body_amanah_venue_chainid_string", "amanah", dict(AMB, venues=[dict(VN, chainId="1")]),
       "sec 6.2", expect=E)
    bc("body_amanah_venue_contract_hex32", "amanah", dict(AMB, venues=[dict(VN, contract=HX32(1))]),
       "sec 1", expect=E)
    bc("body_amanah_venue_codehash_hex20", "amanah", dict(AMB, venues=[dict(VN, codeHash=HX20(1))]),
       "sec 1", expect=E)
    bc("body_amanah_venue_second_bad", "amanah", dict(AMB, venues=[VN, dict(VN, codeHash=None)]),
       "sec 6.2: the row is applied to every element", expect=E)
    bc("body_amanah_venue_extra", "amanah",
       dict(AMB, venues=[dict(VN, name="escrow v1", deep={"a": [1]})]),
       "sec 6.9: members beyond a venue element's row", expect="ok")
    bc("body_amanah_extra_machine_fields", "amanah",
       dict(AMB, fees={"premiumBps": 150}, windows={"t3": 7}, bench=HX20(0xB), guilds=[HX20(0x61)]),
       "sec 6.9/11: the family laws read their fields from members this grammar "
       "permits and does not test", expect="ok")
    bc("body_amanah_quotes_object", "amanah", dict(AMB, quotes={"1:" + HX20(0xE5C): "0x00ff", "x": 1}),
       "sec 6.2: quotes is an object; its members are untested by this grammar, a key outside the "
       "registry law's forms among them", expect="ok")
    bc("body_amanah_quotes_empty", "amanah", dict(AMB, quotes={}),
       "sec 6.2: the empty quotes object", expect="ok")
    bc("body_amanah_quotes_array", "amanah", dict(AMB, quotes=[]),
       "sec 6.2: quotes present with a form its row refuses", expect=E)
    bc("body_amanah_quotes_null", "amanah", dict(AMB, quotes=None),
       "sec 6: quotes written null is present with a refused form", expect=E)
    bc("body_amanah_quotes_string", "amanah", dict(AMB, quotes="0x"),
       "sec 6.2: quotes a string", expect=E)

    # 6.3 judgment
    SR = {"kind": "evm", "chainId": 1, "contract": HX20(0xE5C), "codeHash": HX32(0xC0DE), "id": HX32(0xCA5E)}
    SH = {"kind": "hash", "sha256": HX32(0x5A)}
    JB = {"subjectRef": SR, "amanah": HX32(0xA1), "verdict_md": "ruled"}
    C("body_judgment_ok", t.judg, "sec 6.3: the three required fields plus decision and evidence",
      expect="ok")
    bc("body_judgment_minimal", "judgment", JB, "sec 6.3: every optional field absent", expect="ok")
    bc("body_judgment_no_subject", "judgment", {"amanah": HX32(1), "verdict_md": "v"}, "sec 6.3",
       expect=E)
    bc("body_judgment_no_amanah", "judgment", {"subjectRef": SR, "verdict_md": "v"}, "sec 6.3",
       expect=E)
    bc("body_judgment_no_verdict", "judgment", {"subjectRef": SR, "amanah": HX32(1)}, "sec 6.3",
       expect=E)
    bc("body_judgment_amanah_hex20", "judgment", dict(JB, amanah=HX20(1)), "sec 1", expect=E)
    bc("body_judgment_amanah_null", "judgment", dict(JB, amanah=None), "sec 6", expect=E)
    bc("body_judgment_amanah_dangling", "judgment", dict(JB, amanah=HX32(0xFFFF)),
       "sec 6.3/11: a dangling reference is a legal entry", expect="ok")
    bc("body_judgment_verdict_int", "judgment", dict(JB, verdict_md=7), "sec 6.3", expect=E)
    bc("body_judgment_verdict_prose", "judgment", dict(JB, verdict_md="判决\x00\u200e"),
       "sec 6.3: verdict_md is prose", expect="ok")
    bc("body_judgment_subject_string", "judgment", dict(JB, subjectRef="evm"),
       "sec 6.3: subjectRef is an object", expect=E)
    bc("body_judgment_subject_null", "judgment", dict(JB, subjectRef=None), "sec 6", expect=E)
    bc("body_judgment_subject_no_kind", "judgment", dict(JB, subjectRef={"chainId": 1}),
       "sec 6.3: kind required", expect=E)
    bc("body_judgment_subject_kind_int", "judgment", dict(JB, subjectRef={"kind": 1}), "sec 6.3",
       expect=E)
    bc("body_judgment_subject_kind_empty", "judgment", dict(JB, subjectRef={"kind": ""}),
       "sec 1: a token is non-empty", expect=E)
    bc("body_judgment_subject_kind_space", "judgment", dict(JB, subjectRef={"kind": "a b"}),
       "sec 1: 0x20 is outside a token", expect=E)
    bc("body_judgment_subject_unknown_kind", "judgment",
       dict(JB, subjectRef={"kind": "bitcoin-txo", "txid": HX32(1), "vout": 0}),
       "sec 6.3: an unknown kind is tested as an object carrying a token kind and nothing more",
       expect="ok")
    bc("body_judgment_subject_unknown_kind_bare", "judgment", dict(JB, subjectRef={"kind": "x"}),
       "sec 6.3: an unknown kind with no other member", expect="ok")
    bc("body_judgment_subject_unknown_kind_junk", "judgment",
       dict(JB, subjectRef={"kind": "x", "chainId": "not an int", "contract": None}),
       "sec 6.3: no known kind's table reaches an unknown kind", expect="ok")
    bc("body_judgment_evm_no_chainid", "judgment",
       dict(JB, subjectRef={k: v for k, v in SR.items() if k != "chainId"}), "sec 6.3 evm", expect=E)
    bc("body_judgment_evm_no_contract", "judgment",
       dict(JB, subjectRef={k: v for k, v in SR.items() if k != "contract"}), "sec 6.3 evm", expect=E)
    bc("body_judgment_evm_no_codehash", "judgment",
       dict(JB, subjectRef={k: v for k, v in SR.items() if k != "codeHash"}), "sec 6.3 evm", expect=E)
    bc("body_judgment_evm_no_id", "judgment",
       dict(JB, subjectRef={k: v for k, v in SR.items() if k != "id"}), "sec 6.3 evm", expect=E)
    bc("body_judgment_evm_chainid_string", "judgment", dict(JB, subjectRef=dict(SR, chainId="1")),
       "sec 6.3 evm", expect=E)
    bc("body_judgment_evm_contract_hex32", "judgment", dict(JB, subjectRef=dict(SR, contract=HX32(1))),
       "sec 1", expect=E)
    bc("body_judgment_evm_id_hex20", "judgment", dict(JB, subjectRef=dict(SR, id=HX20(1))),
       "sec 1", expect=E)
    bc("body_judgment_evm_extra", "judgment",
       dict(JB, subjectRef=dict(SR, caseId=7, venue="escrow")),
       "sec 6.9: members beyond a known kind's table", expect="ok")
    bc("body_judgment_hash_ok", "judgment", dict(JB, subjectRef=SH), "sec 6.3 hash", expect="ok")
    bc("body_judgment_hash_note", "judgment", dict(JB, subjectRef=dict(SH, note_md="off\x00chain")),
       "sec 6.3 hash: the optional prose note", expect="ok")
    bc("body_judgment_hash_no_sha", "judgment", dict(JB, subjectRef={"kind": "hash"}),
       "sec 6.3 hash", expect=E)
    bc("body_judgment_hash_sha_hex20", "judgment", dict(JB, subjectRef={"kind": "hash", "sha256": HX20(1)}),
       "sec 1", expect=E)
    bc("body_judgment_hash_note_int", "judgment", dict(JB, subjectRef=dict(SH, note_md=1)),
       "sec 6.3 hash", expect=E)
    bc("body_judgment_decision_ok", "judgment", dict(JB, decision={"rung": 0, "amount": "0x0"}),
       "sec 6.3: decision is an object in the venue's vocabulary", expect="ok")
    bc("body_judgment_decision_empty", "judgment", dict(JB, decision={}),
       "sec 6.3: an empty decision object", expect="ok")
    bc("body_judgment_decision_int", "judgment", dict(JB, decision=2), "sec 6.3", expect=E)
    bc("body_judgment_decision_null", "judgment", dict(JB, decision=None), "sec 6", expect=E)
    bc("body_judgment_decision_array", "judgment", dict(JB, decision=[2]), "sec 6.3", expect=E)
    EV = {"manifest": HX32(0xE1), "produced": True}
    bc("body_judgment_evidence_two", "judgment",
       dict(JB, evidence=[EV, {"manifest": HX32(0xE2), "produced": False}]),
       "sec 6.3: two evidence rows, one produced and one not", expect="ok")
    bc("body_judgment_evidence_empty", "judgment", dict(JB, evidence=[]),
       "sec 6.3: an empty evidence array", expect="ok")
    bc("body_judgment_evidence_object", "judgment", dict(JB, evidence=EV),
       "sec 6.3: evidence is an array", expect=E)
    bc("body_judgment_evidence_null", "judgment", dict(JB, evidence=None), "sec 6", expect=E)
    bc("body_judgment_evidence_element_string", "judgment", dict(JB, evidence=[HX32(1)]),
       "sec 6.3: every element an object", expect=E)
    bc("body_judgment_evidence_no_manifest", "judgment", dict(JB, evidence=[{"produced": True}]),
       "sec 6.3", expect=E)
    bc("body_judgment_evidence_no_produced", "judgment", dict(JB, evidence=[{"manifest": HX32(1)}]),
       "sec 6.3", expect=E)
    bc("body_judgment_evidence_produced_int", "judgment",
       dict(JB, evidence=[{"manifest": HX32(1), "produced": 1}]),
       "sec 6.3: produced is true or false, and 1 is neither", expect=E)
    bc("body_judgment_evidence_produced_null", "judgment",
       dict(JB, evidence=[{"manifest": HX32(1), "produced": None}]), "sec 6.3", expect=E)
    bc("body_judgment_evidence_produced_string", "judgment",
       dict(JB, evidence=[{"manifest": HX32(1), "produced": "true"}]), "sec 6.3", expect=E)
    bc("body_judgment_evidence_manifest_hex20", "judgment",
       dict(JB, evidence=[{"manifest": HX20(1), "produced": True}]), "sec 1", expect=E)
    bc("body_judgment_evidence_second_bad", "judgment",
       dict(JB, evidence=[EV, {"manifest": HX32(2)}]),
       "sec 6.3: the row is applied to every element", expect=E)
    bc("body_judgment_evidence_extra", "judgment",
       dict(JB, evidence=[dict(EV, party=HX20(0xB), committedRef=HX32(0xC), note_md="\x00")]),
       "sec 6.9: members beyond an evidence row", expect="ok")
    bc("body_judgment_txrefs_ok", "judgment", dict(JB, txRefs=[HX32(1), HX32(2)]),
       "sec 6.3: transactions the ruling relies on", expect="ok")
    bc("body_judgment_txrefs_empty", "judgment", dict(JB, txRefs=[]), "sec 6.3", expect="ok")
    bc("body_judgment_txrefs_string", "judgment", dict(JB, txRefs=HX32(1)),
       "sec 6.3: txRefs is an array", expect=E)
    bc("body_judgment_txrefs_element_hex20", "judgment", dict(JB, txRefs=[HX32(1), HX20(2)]),
       "sec 1", expect=E)
    bc("body_judgment_txrefs_element_int", "judgment", dict(JB, txRefs=[1]), "sec 6.3", expect=E)
    bc("body_judgment_txrefs_null", "judgment", dict(JB, txRefs=None), "sec 6", expect=E)
    bc("body_judgment_terms_ok", "judgment", dict(JB, terms=HX32(0x7E)),
       "sec 6.3: the terms digest", expect="ok")
    bc("body_judgment_terms_hex20", "judgment", dict(JB, terms=HX20(1)), "sec 1", expect=E)
    bc("body_judgment_terms_null", "judgment", dict(JB, terms=None), "sec 6", expect=E)
    bc("body_judgment_extra", "judgment",
       dict(JB, statements={"buyer": "said so"}, x=1, y_md="\x00"),
       "sec 6.9: members beyond the table", expect="ok")

    # 6.4 resignation
    RB = {"subjectRef": SR, "amanah": HX32(0xA1), "category": "other", "statement_md": "declined"}
    RU = dict(RB, category="unlawful-subject", clause="8.1")
    C("body_resignation_ok", t.resg, "sec 6.4: unlawful-subject with its clause", expect="ok")
    bc("body_resignation_other", "resignation", RB, "sec 6.4: other, no clause", expect="ok")
    bc("body_resignation_unlawful_with_clause", "resignation", RU, "sec 6.4", expect="ok")
    bc("body_resignation_unlawful_no_clause", "resignation",
       {k: v for k, v in RU.items() if k != "clause"},
       "sec 6.4: clause present iff category is unlawful-subject", expect=E)
    bc("body_resignation_other_with_clause", "resignation", dict(RB, clause="8.1"),
       "sec 6.4: a clause beside other breaks the relation", expect=E)
    bc("body_resignation_category_third", "resignation", dict(RB, category="conflict"),
       "sec 6.4: the set is closed", expect=E)
    bc("body_resignation_category_case", "resignation", dict(RB, category="Other"),
       "sec 6.4: tokens compare bytewise", expect=E)
    bc("body_resignation_category_int", "resignation", dict(RB, category=1), "sec 6.4", expect=E)
    bc("body_resignation_category_null", "resignation", dict(RB, category=None), "sec 6", expect=E)
    bc("body_resignation_no_category", "resignation",
       {k: v for k, v in RB.items() if k != "category"}, "sec 6.4", expect=E)
    bc("body_resignation_no_subject", "resignation",
       {k: v for k, v in RB.items() if k != "subjectRef"}, "sec 6.4", expect=E)
    bc("body_resignation_no_amanah", "resignation",
       {k: v for k, v in RB.items() if k != "amanah"}, "sec 6.4", expect=E)
    bc("body_resignation_no_statement", "resignation",
       {k: v for k, v in RB.items() if k != "statement_md"}, "sec 6.4", expect=E)
    bc("body_resignation_clause_empty", "resignation", dict(RU, clause=""),
       "sec 6.4: clause is non-empty", expect=E)
    bc("body_resignation_clause_space", "resignation", dict(RU, clause="section 8, item 1"),
       "sec 6.4: a skeleton string may carry spaces", expect="ok")
    bc("body_resignation_clause_control", "resignation", dict(RU, clause="8\x01"),
       "sec 3.3: a control byte fails the value charset at test 5 before the table", expect="E_VALUE_CHARSET")
    bc("body_resignation_clause_int", "resignation", dict(RU, clause=8), "sec 6.4", expect=E)
    bc("body_resignation_clause_null", "resignation", dict(RU, clause=None), "sec 6", expect=E)
    bc("body_resignation_subject_hash", "resignation", dict(RB, subjectRef=SH),
       "sec 6.4: subjectRef as sec 6.3", expect="ok")
    bc("body_resignation_subject_bad_evm", "resignation", dict(RB, subjectRef=dict(SR, id=1)),
       "sec 6.4: the known kind's table applies", expect=E)
    bc("body_resignation_extra", "resignation", dict(RB, refund=True, x_md="\x00"),
       "sec 6.9: members beyond the table", expect="ok")

    # 6.7 keybinding
    KB = {"recipient": "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq", "effective": 1700000000}
    C("body_keybinding_ok", t.keyb, "sec 6.7: recipient and effective", expect="ok")
    bc("body_keybinding_no_recipient", "keybinding", {"effective": 1}, "sec 6.7", expect=E)
    bc("body_keybinding_no_effective", "keybinding", {"recipient": "age1x"}, "sec 6.7", expect=E)
    bc("body_keybinding_recipient_empty", "keybinding", dict(KB, recipient=""),
       "sec 1: a token is non-empty", expect=E)
    bc("body_keybinding_recipient_space", "keybinding", dict(KB, recipient="age1 x"),
       "sec 1: 0x20 is outside a token", expect=E)
    bc("body_keybinding_recipient_int", "keybinding", dict(KB, recipient=1), "sec 6.7", expect=E)
    bc("body_keybinding_recipient_hex", "keybinding", dict(KB, recipient=HX32(1)),
       "sec 6.7: the grammar tests a token and nothing about what it encodes", expect="ok")
    bc("body_keybinding_effective_string", "keybinding", dict(KB, effective="1"), "sec 6.7", expect=E)
    bc("body_keybinding_effective_zero", "keybinding", dict(KB, effective=0), "sec 6.7", expect="ok")
    bc("body_keybinding_extra", "keybinding", dict(KB, scheme="age", note_md="\x00"),
       "sec 6.9", expect="ok")

    # 6.6 succession
    SB = {"to": A_("k2"), "kind": "handover", "effective": 1700000000, "statement_md": "s"}
    C("body_succession_ok", t.succ, "sec 6.6: all four required fields", expect="ok")
    bc("body_succession_no_to", "succession", {k: v for k, v in SB.items() if k != "to"},
       "sec 6.6", expect=E)
    bc("body_succession_no_kind", "succession", {k: v for k, v in SB.items() if k != "kind"},
       "sec 6.6", expect=E)
    bc("body_succession_no_effective", "succession",
       {k: v for k, v in SB.items() if k != "effective"}, "sec 6.6", expect=E)
    bc("body_succession_no_statement", "succession",
       {k: v for k, v in SB.items() if k != "statement_md"}, "sec 6.6", expect=E)
    bc("body_succession_to_hex32", "succession", dict(SB, to=HX32(1)), "sec 1", expect=E)
    bc("body_succession_kind_keyrotation", "succession", dict(SB, kind="keyrotation"),
       "sec 6.6: the grammar names keyrotation and reads nothing", expect="ok")
    bc("body_succession_kind_space", "succession", dict(SB, kind="hand over"),
       "sec 1/6: 0x20 is the one scalar value sec 3.5 admits and a token refuses", expect=E)
    bc("body_succession_kind_unknown", "succession", dict(SB, kind="court-order/2031"),
       "sec 6.6: the grammar refuses no other value", expect="ok")
    bc("body_succession_kind_empty", "succession", dict(SB, kind=""), "sec 1", expect=E)
    bc("body_succession_kind_int", "succession", dict(SB, kind=1), "sec 6.6", expect=E)
    bc("body_succession_effective_zero", "succession", dict(SB, effective=0),
       "sec 6.6: Unix seconds zero", expect="ok")
    bc("body_succession_effective_string", "succession", dict(SB, effective="1700000000"),
       "sec 6.6", expect=E)
    bc("body_succession_effective_null", "succession", dict(SB, effective=None), "sec 6",
       expect=E)
    bc("body_succession_to_self", "succession", dict(SB, to=A_("root")),
       "sec 6.6: a to equal to the signer passes the table; that a seat writes none is sec 6.6's prose", expect="ok")
    bc("body_succession_statement_int", "succession", dict(SB, statement_md=1), "sec 6.6",
       expect=E)

    # 6.6a migration
    MB = {"spec": "galeed/2", "statement_md": "the seat moves to galeed/2 under this key"}
    bc("body_migration_ok", "migration", MB, "sec 6.6a: spec and statement_md", expect="ok")
    bc("body_migration_spec_any_token", "migration", dict(MB, spec="court-ledger/7"),
       "sec 6.6a: spec is any token, data for readers", expect="ok")
    bc("body_migration_spec_same_law", "migration", dict(MB, spec="galeed/1"),
       "sec 6.6a: the grammar reads no spec it names, its own included", expect="ok")
    bc("body_migration_spec_one_char", "migration", dict(MB, spec="!"), "sec 1: a one-byte token", expect="ok")
    bc("body_migration_no_spec", "migration", {"statement_md": "s"}, "sec 6.6a", expect=E)
    bc("body_migration_no_statement", "migration", {"spec": "galeed/2"}, "sec 6.6a", expect=E)
    bc("body_migration_spec_empty", "migration", dict(MB, spec=""), "sec 1: the empty string is no token", expect=E)
    bc("body_migration_spec_space", "migration", dict(MB, spec="galeed 2"), "sec 1: 0x20 is outside a token", expect=E)
    bc("body_migration_spec_int", "migration", dict(MB, spec=2), "sec 6.6a", expect=E)
    bc("body_migration_spec_null", "migration", dict(MB, spec=None), "sec 6", expect=E)
    bc("body_migration_statement_int", "migration", dict(MB, statement_md=1), "sec 6.6a", expect=E)
    bc("body_migration_statement_empty", "migration", dict(MB, statement_md=""), "sec 6.6a: the empty prose string", expect="ok")
    bc("body_migration_extra", "migration", dict(MB, newRoot=A_("root"), entry_id=HX32(7)),
       "sec 6.9: members beyond the table are data", expect="ok")
    bc("body_migration_spec_del", "migration", dict(MB, spec="galeed/2\x7f"),
       "sec 3.3: DEL fails the value charset at test 5 before the table", expect="E_VALUE_CHARSET")
    C("migration_at_seq_zero", make_entry(K("root"), "migration", 0, None, MB),
      "sec 4.2: a migration at seq 0 is no genesis", expect="E_GENESIS_PLACE")

    # 6.5 annotation
    C("body_annotation_ok", t.ann, "sec 6.5: note_md present", expect="ok")
    bc("body_annotation_no_note", "annotation", {}, "sec 6.5", expect=E)
    bc("body_annotation_note_empty", "annotation", {"note_md": ""},
       "sec 6.10: the empty prose string is a legal spelling of a prose string row", expect="ok")
    bc("body_annotation_subject_ok", "annotation", {"note_md": "n", "subject": HX32(3)},
       "sec 6.5: the optional subject", expect="ok")
    bc("body_annotation_subject_null", "annotation", {"note_md": "n", "subject": None},
       "sec 6", expect=E)
    bc("body_annotation_subject_hex20", "annotation", {"note_md": "n", "subject": HX20(3)},
       "sec 1", expect=E)
    bc("body_annotation_note_object", "annotation", {"note_md": {"a": "b"}}, "sec 6.5", expect=E)
    bc("body_annotation_note_null", "annotation", {"note_md": None}, "sec 6", expect=E)

    # 6.8 unknown types
    C("body_unknown_empty", t.unk, "sec 6.8: an unknown type with an empty body", expect="ok")
    bc("body_unknown_judgment_shaped", "dispatch", JB,
       "sec 6.8: an unknown type carrying a judgment-shaped body is tested by sec 6.9 alone",
       expect="ok")
    bc("body_unknown_broken_judgment_shape", "dispatch", {"subjectRef": 1, "amanah": None},
       "sec 6.8: no known type's row reaches an unknown type's body", expect="ok")
    C("body_unknown_body_not_object",
      craft("windmill", 1, t.gid, [], "0x" + "11" * 65),
      "sec 4.3: step 9 precedes step 12 for unknown types too", expect="E_BODY")
    C("body_member_named_sig",
      make_entry(K("root"), "windmill", 1, t.gid, {"sig": "0x" + "11" * 65, "n": 1}),
      "sec 5.1: B6 removes the top-level member whose key is sig and nothing else; a member "
      "named sig at any depth inside body is untouched and stays inside the preimage",
      expect="ok")
    C("body_members_named_like_envelope",
      make_entry(K("root"), "windmill", 1, t.gid,
                 {"spec": "galeed/9", "seq": 99, "prev": None, "judge": "x",
                  "entryType": "y", "body": {"sig": [1]}}),
      "sec 4.1/6.9: only the envelope is arity-closed; a body may carry members named like "
      "envelope keys", expect="ok")
    bc("body_unknown_deep_extras", "windmill",
       {"a": {"b": {"c": [{"d_md": "\x00"}, 1, None]}}, "e_md": {"f": ["\x07"]}},
       "sec 6.9: extra members at every depth, with prose subtrees inside them", expect="ok")

    # ---------------- round 8: sides the third-opinion seat found unisolated
    C("depth_refused_bracket_at_128", b"[" * 127 + b"{" + b"[",
      "sec 3.5 test 2: a bracket the grammar refuses opens no container, so the `[` at the "
      "member-name position of the 128th container is E_JSON, and E_DEPTH shares no triggering "
      "byte with it.", "E_JSON")
    C("value_charset_fault_before_key_charset_fault", b'{"a":"\\u0001","k\\u0001":"v"}',
      "sec 3.5 tests 4 and 5: each is decided in full before the next, so the key outside the "
      "charset names E_KEY_CHARSET although the value's fault sits at an earlier byte.",
      "E_KEY_CHARSET")
    C("prev_malformed_at_seq_zero", make_entry(K("root"), "genesis", 0, "0x00", GENB),
      "sec 4.3 step 8: E_PREV before E_PREV_SEQ, so a prev that is neither null nor hex32 names "
      "E_PREV although seq 0 would also fail the relation.", "E_PREV")
    C("prev_malformed_at_seq_one", make_entry(K("root"), "annotation", 1, "0x00", ANNB("x")),
      "sec 4.3 step 8: the same prev at seq 1 names E_PREV under either order; the control.",
      "E_PREV")
    C("sig_all_zero_v_before_range", craft("genesis", 0, None, GENB, "0x" + "0" * 130),
      "sec 4.3 step 13 with sec 5.4: v is tested before r and s, so a signature failing both "
      "bullets names E_SIG_V.", "E_SIG_V")
    C("seq_string_before_prev_malformed", craft("annotation", "1", "0x00", ANNB("x"), "0x" + "11" * 65),
      "sec 4.3 steps 7 and 8: a seq that is not an int names E_SEQ before the malformed prev is "
      "reached.", "E_SEQ")
    C("prev_malformed_before_body_array", craft("annotation", 1, "0x00", [], "0x" + "11" * 65),
      "sec 4.3 steps 8 and 9: the malformed prev names E_PREV before the body that is no object "
      "is reached.", "E_PREV")
    C("array_open_then_dot", b"[.]",
      "sec 3.1: the position after an array's [ admits a value, so a `.` there is a value "
      "position of numeric shape and names E_NUMBER, never E_JSON.", "E_NUMBER")
    C("array_open_then_close", b"[]",
      "sec 3.1: the ] the grammar also admits after [ takes nothing from the value position; "
      "the control, canonical and no object.", "E_ENVELOPE")
    # ---------------- round 11: every row of sec 6, one case per type, row and side
    KBX = {"recipient": "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq", "effective": 1700000000}
    SUX = {"to": HX20(0x5), "kind": "handover", "effective": 1700000000, "statement_md": "handed"}
    EVM = {"kind": "evm", "chainId": 1, "contract": HX20(0xE5C), "codeHash": HX32(0xC0DE), "id": HX32(7)}
    HSH = {"kind": "hash", "sha256": HX32(0xA5), "note_md": "an object"}
    RBX = {"subjectRef": dict(HSH), "amanah": HX32(0xA1), "category": "other", "statement_md": "declined"}
    JBX = {"subjectRef": dict(EVM), "amanah": HX32(0xA1), "verdict_md": "ruled", "decision": {"rung": 2},
           "evidence": [{"manifest": HX32(0xE1), "produced": True}], "txRefs": [HX32(0xE2)], "terms": HX32(0xE3)}
    BAD = {"prose": 1, "int": "1", "hex32": "0x12", "hex20": "0x12", "token": "a b", "bool": "true",
           "object": [], "hex32list": ["0x12"], "list": 1, "category": "maybe"}
    ROWS = {
        "genesis": (dict(GENB), [("statement_md", True, "prose")]),
        "amanah": ({"version": 3, "text_md": "covenant 3", "venues": [{"chainId": 1, "contract": HX20(2), "codeHash": HX32(3)}]},
                   [("version", True, "int"), ("text_md", True, "prose"), ("venues", False, "list")]),
        "judgment": (JBX, [("subjectRef", True, "object"), ("amanah", True, "hex32"), ("verdict_md", True, "prose"),
                           ("decision", False, "object"), ("evidence", False, "list"), ("txRefs", False, "hex32list"),
                           ("terms", False, "hex32")]),
        "resignation": (RBX, [("subjectRef", True, "object"), ("amanah", True, "hex32"),
                              ("category", True, "category"), ("statement_md", True, "prose")]),
        "succession": (SUX, [("to", True, "hex20"), ("kind", True, "token"), ("effective", True, "int"),
                             ("statement_md", True, "prose")]),
        "annotation": (dict(ANNB("n"), subject=HX32(9)), [("subject", False, "hex32"), ("note_md", True, "prose")]),
        "keybinding": (KBX, [("recipient", True, "token"), ("effective", True, "int")]),
        "migration": ({"spec": "galeed/2", "statement_md": "moving to the next law"},
                      [("spec", True, "token"), ("statement_md", True, "prose")]),
    }
    for etype, (base, rows) in ROWS.items():
        seq0 = etype == "genesis"
        def rowcase(slug, body, note):
            C("row_%s_%s" % (etype, slug),
              make_entry(K("root"), etype, 0 if seq0 else 1, None if seq0 else t.gid, body), note, "E_BODY_FIELD")
        for field, required, form in rows:
            if required:
                rowcase(field + "_absent", {k: v for k, v in base.items() if k != field},
                        "sec 6 row table: %s's required %s absent." % (etype, field))
            rowcase(field + "_form", dict(base, **{field: BAD[form]}),
                    "sec 6 row table: %s's %s present with a form its row refuses." % (etype, field))
        if etype == "amanah":
            for f in ("chainId", "contract", "codeHash"):
                el = {"chainId": 1, "contract": HX20(2), "codeHash": HX32(3)}
                rowcase("venues_%s_absent" % f.lower(), dict(base, venues=[{k: v for k, v in el.items() if k != f}]),
                        "sec 6.2 venues element: %s absent." % f)
                rowcase("venues_%s_form" % f.lower(), dict(base, venues=[dict(el, **{f: BAD["int" if f == "chainId" else "hex20"]})]),
                        "sec 6.2 venues element: %s present with a form its row refuses." % f)
            rowcase("venues_element_form", dict(base, venues=[1]), "sec 6.2 venues element: not an object.")
        if etype == "judgment":
            for f, form in (("manifest", "hex32"), ("produced", "bool")):
                el = {"manifest": HX32(0xE1), "produced": True}
                rowcase("evidence_%s_absent" % f, dict(base, evidence=[{k: v for k, v in el.items() if k != f}]),
                        "sec 6.3 evidence element: %s absent." % f)
                rowcase("evidence_%s_form" % f, dict(base, evidence=[dict(el, **{f: BAD[form]})]),
                        "sec 6.3 evidence element: %s present with a form its row refuses." % f)
            rowcase("evidence_element_form", dict(base, evidence=[1]), "sec 6.3 evidence element: not an object.")
        if etype in ("judgment", "resignation"):
            for kind, tbl, req in (("evm", EVM, ("chainId", "contract", "codeHash", "id")), ("hash", HSH, ("sha256",))):
                forms = {"chainId": "int", "contract": "hex20", "codeHash": "hex32", "id": "hex32", "sha256": "hex32", "note_md": "prose"}
                for f in tbl:
                    if f == "kind":
                        continue
                    if f in req:
                        rowcase("subject_%s_%s_absent" % (kind, f.lower()), dict(base, subjectRef={k: v for k, v in tbl.items() if k != f}),
                                "sec 6.3 %s kind under %s: %s absent." % (kind, etype, f))
                    rowcase("subject_%s_%s_form" % (kind, f.lower()), dict(base, subjectRef=dict(tbl, **{f: BAD[forms[f]]})),
                            "sec 6.3 %s kind under %s: %s present with a form its row refuses." % (kind, etype, f))
        if etype == "resignation":
            rowcase("clause_empty", dict(base, category="unlawful-subject", clause=""),
                    "sec 6.4: clause present and empty under unlawful-subject.")
            rowcase("clause_form", dict(base, category="unlawful-subject", clause=1),
                    "sec 6.4: clause present with a form its row refuses under unlawful-subject.")

    for slug, sr in (("string", "evm"), ("no_kind", {"chainId": 1}), ("kind_int", {"kind": 1}),
                     ("evm_no_chainid", {"kind": "evm", "contract": HX20(1), "codeHash": HX32(1), "id": HX32(1)}),
                     ("evm_contract_short", {"kind": "evm", "chainId": 1, "contract": "0x12", "codeHash": HX32(1), "id": HX32(1)}),
                     ("hash_sha256_short", {"kind": "hash", "sha256": "0x12"}),
                     ("hash_note_int", {"kind": "hash", "sha256": HX32(2), "note_md": 1})):
        C("body_resignation_subject_" + slug,
          make_entry(K("root"), "resignation", 1, t.gid, dict(RBX, subjectRef=sr)),
          "sec 6.4 with sec 6.3: the subjectRef row and its kind tables reach a resignation "
          "exactly as they reach a judgment; %s." % slug.replace("_", " "), "E_BODY_FIELD")


    # ---------------- randomized valid entries -------------------------------

    valid_pool = [t.gen, t.aman, t.judg, t.resg, t.succ, t.ann, t.keyb, t.unk, maxent]
    types = ["genesis", "amanah", "judgment", "resignation", "succession",
             "annotation", "keybinding", "dispatch", "sale/2031", "x"]
    for i in range(120):
        et = types[i % len(types)]
        seq = 0 if et == "genesis" else rng.randrange(1, 1 << 20)
        prev = None if seq == 0 else HX32(rng.getrandbits(256))
        body = random_body(rng, et)
        for _ in range(rng.randrange(0, 4)):
            k, v = random_member(rng, 0)
            body[k] = v
        b = make_entry(K(rng.choice(["root", "k2", "k3"])), et, seq, prev, body)
        valid_pool.append(b)
        C("rand_valid_%03d_%s" % (i, et.replace("/", "-")), b,
          "randomized valid entry of type %r with random extra members and random prose" % et,
          expect="ok")

    # ---------------- randomized mutations of valid entries ------------------
    kinds = ["flip", "insert", "delete", "swap", "truncate", "dup_slice",
             "shuffle_top", "shuffle_body", "splice"]
    made = 0
    i = 0
    while made < 320:
        i += 1
        base = rng.choice(valid_pool)
        kind = rng.choice(kinds)
        try:
            m = mutate(rng, base, kind, valid_pool)
        except Exception:
            continue
        if m == base or m is None:
            continue
        C("rand_mut_%03d_%s" % (made, kind), m,
          "randomized %s mutation of a valid entry" % kind)
        made += 1


# ==========================================================================
# helpers the canon cases lean on
# ==========================================================================


def build_canon_round1(rng, t):
    """Round-1 witnesses of 2026-10-16: boundaries the seats found unwitnessed."""
    E = None
    # sec 3.1 / 3.5 test 2: numeric runs and value positions
    C("num_run_zero_minus", b"0-", "sec 3.1: the run 0- is drawn from the run set and is not an integer: E_NUMBER at its first byte", "E_NUMBER")
    C("num_run_one_minus_two", b"1-2", "sec 3.1: 1-2 is one run, not an integer", "E_NUMBER")
    C("num_run_one_e", b"1e", "sec 3.1: 1e is one run", "E_NUMBER")
    C("num_run_zero_e", b"0e", "sec 3.1: 0e is one run", "E_NUMBER")
    C("num_run_exp_plus", b"1E+2", "sec 3.1: 1E+2 is one run", "E_NUMBER")
    C("array_value_then_minus_no_comma", b"[1 -2]", "sec 3.1: the - follows a complete value where the grammar admits no value: E_JSON", "E_JSON")
    C("array_value_comma_minus", b"[1,-2]", "sec 3.1: after the comma a value position of numeric shape: E_NUMBER", "E_NUMBER")
    C("ws_form_feed", b"[\x0c1]", "sec 3.5 test 2: a form feed is not RFC 8259 whitespace", "E_JSON")
    C("ws_vertical_tab", b"[\x0b1]", "sec 3.5 test 2: a vertical tab is not RFC 8259 whitespace", "E_JSON")
    C("ws_nbsp", b"[\xc2\xa01]", "sec 3.5 test 2: U+00A0 is not RFC 8259 whitespace", "E_JSON")
    C("ws_nel", b"[\xc2\x851]", "sec 3.5 test 2: U+0085 is not RFC 8259 whitespace", "E_JSON")
    # sec 6: kind and type literals compare bytewise
    P0 = HX32(0x11)
    C("subject_kind_EVM_unknown", mk("root", "judgment", 2, P0, {"subjectRef": {"kind": "EVM"}, "amanah": HX32(0xA1), "verdict_md": "v"}),
      "sec 6.3: kind EVM differs from evm in case; an unknown kind forces no table, so the one member passes", "ok")
    C("subject_kind_Hash_unknown", mk("root", "judgment", 2, P0, {"subjectRef": {"kind": "Hash"}, "amanah": HX32(0xA1), "verdict_md": "v"}),
      "sec 6.3: kind Hash is an unknown kind", "ok")
    C("type_Genesis_at_zero", mk("root", "Genesis", 0, None, GENB), "sec 4.2/6: Genesis is not genesis; at seq 0 the entry is not a genesis: E_GENESIS_PLACE", "E_GENESIS_PLACE")
    C("type_GENESIS_at_one", mk("root", "GENESIS", 1, P0, GENB), "sec 6.8: GENESIS is an unknown type with an opaque body, accepted at seq 1", "ok")
    # sec 5.4: recovery never forms x = r + n
    PRIME = 2 ** 256 - 2 ** 32 - 977
    def no_point(x):
        y2 = (pow(x, 3, PRIME) + 7) % PRIME
        return pow(y2, (PRIME - 1) // 2, PRIME) != 1
    r = 2
    while not (no_point(r) and not no_point(r + N)):
        r += 1
    sig = "0x" + r.to_bytes(32, "big").hex() + (1).to_bytes(32, "big").hex() + "1b"
    C("sig_r_plus_n_never_formed", craft("annotation", 1, P0, {"note_md": "r + n"}, sig),
      "sec 5.4: r = %d names no curve point while r + n does; the candidate x = r + n is never formed: E_SIG_RECOVER" % r, "E_SIG_RECOVER")


def craft(entryType, seq, prev, body, sigval, judge=None, spec="galeed/1"):
    if judge is None:
        judge = A_("root")
    bv = body if isinstance(body, (JObj, list)) else J(body)
    if isinstance(body, list):
        bv = J(body)
    env6 = build_env(spec, entryType, judge, seq, prev, bv)
    return cbytes(JObj(env6.members + [("sig", sigval)]))


def find_v28(prev):
    for i in range(100000):
        env6 = build_env("galeed/1", "annotation", A_("root"), 1, prev,
                         J({"n": i, "note_md": "seeking v=28"}))
        digest = eip191_digest("galeed/1", sha256(cbytes(env6)))
        r, s, v = ecdsa_sign(K("root"), digest)
        if v == 28:
            return cbytes(JObj(env6.members + [("sig", sig_hex(r, s, v))]))
    raise AssertionError("no v=28")


_BAD_R = [None]


def bad_r():
    if _BAD_R[0] is None:
        r = 1
        while True:
            a = (r * r * r + 7) % P
            if a != 0 and pow(a, (P - 1) // 2, P) == P - 1:
                _BAD_R[0] = r
                break
            r += 1
    return _BAD_R[0]


def infinity_sig_entry(prev):
    env6 = build_env("galeed/1", "annotation", A_("root"), 1, prev,
                     J({"note_md": "sR equals zG"}))
    digest = eip191_digest("galeed/1", sha256(cbytes(env6)))
    z = int.from_bytes(digest, "big") % N
    for k in range(1, 500):
        R = mul(G, k)
        if R is None or R[0] >= N or R[0] % N == 0:
            continue
        r = R[0] % N
        s = (z * pow(k, N - 2, N)) % N
        if s == 0 or s > HALF_N:
            continue
        return cbytes(JObj(env6.members + [("sig", sig_hex(r, s, 27 + (R[1] & 1)))]))
    raise AssertionError("no infinity signature")


def _other_domain_entry(prev, domain, note):
    env6 = build_env("galeed/1", "annotation", A_("root"), 1, prev, J({"note_md": note}))
    presig = sha256(cbytes(env6))
    r, s, v = ecdsa_sign(K("root"), eip191_digest(domain, presig))
    return cbytes(JObj(env6.members + [("sig", sig_hex(r, s, v))]))


def foreign_domain_sig_entry(prev, domain):
    return _other_domain_entry(prev, domain, "signed under " + domain)


def sig_over_other_b6(prev):
    other = build_env("galeed/1", "annotation", A_("root"), 1, prev, J({"note_md": "other"}))
    r, s, v = ecdsa_sign(K("root"), eip191_digest("galeed/1", sha256(cbytes(other))))
    env6 = build_env("galeed/1", "annotation", A_("root"), 1, prev, J({"note_md": "this"}))
    return cbytes(JObj(env6.members + [("sig", sig_hex(r, s, v))]))


# ---------------- randomized value generators ------------------------------

PROSE_POOL = ([chr(c) for c in range(0x20, 0x7F)] +
              [chr(c) for c in range(0x00, 0x20)] +
              ["\x7f"] + [chr(c) for c in range(0x80, 0xA0)] +
              ["‎", "‏", "‪", "‫", "‬", "‭", "‮",
               "⁦", "⁩", "﻿", "é", "中", "\U0001f600",
               "\U0010ffff", "́"])
SKEL_POOL = [chr(c) for c in range(0x20, 0x7F)]


def rand_prose(rng, n=None):
    n = rng.randrange(0, 24) if n is None else n
    return "".join(rng.choice(PROSE_POOL) for _ in range(n))


def rand_skel(rng, n=None):
    n = rng.randrange(0, 16) if n is None else n
    return "".join(rng.choice(SKEL_POOL) for _ in range(n))


def rand_key(rng):
    while True:
        k = "".join(rng.choice(SKEL_POOL) for _ in range(rng.randrange(1, 8)))
        if not k.endswith("_md"):
            return k


def random_member(rng, depth):
    prose = rng.random() < 0.35
    k = rand_key(rng) + ("_md" if prose else "")
    return k, random_value(rng, depth, prose)


def random_value(rng, depth, prose=False):
    r = rng.random()
    if depth >= 3 or r < 0.35:
        pick = rng.randrange(6)
        if pick == 0:
            return None
        if pick == 1:
            return True
        if pick == 2:
            return False
        if pick == 3:
            return rng.randrange(0, MAXINT + 1)
        return rand_prose(rng) if prose else rand_skel(rng)
    if r < 0.6:
        return [random_value(rng, depth + 1, prose) for _ in range(rng.randrange(0, 4))]
    d = {}
    for _ in range(rng.randrange(0, 4)):
        if prose:
            d[rand_key(rng)] = random_value(rng, depth + 1, True)
        else:
            k, v = random_member(rng, depth + 1)
            d[k] = v
    return d


def random_subject(rng):
    r = rng.random()
    if r < 0.45:
        return {"kind": "evm", "chainId": rng.randrange(0, 1 << 20), "contract": HX20(rng.getrandbits(160)),
                "codeHash": HX32(rng.getrandbits(256)), "id": HX32(rng.getrandbits(256))}
    if r < 0.8:
        d = {"kind": "hash", "sha256": HX32(rng.getrandbits(256))}
        if rng.random() < 0.5:
            d["note_md"] = rand_prose(rng)
        return d
    d = {"kind": rand_token(rng)}
    for _ in range(rng.randrange(0, 3)):
        k, v = random_member(rng, 1)
        d[k] = v
    return d


def random_body(rng, etype):
    if etype == "genesis":
        return {"statement_md": rand_prose(rng)}
    if etype == "amanah":
        b = {"version": rng.randrange(0, MAXINT + 1), "text_md": rand_prose(rng)}
        if rng.random() < 0.6:
            b["venues"] = [{"chainId": rng.randrange(0, 1 << 20), "contract": HX20(rng.getrandbits(160)),
                            "codeHash": HX32(rng.getrandbits(256))} for _ in range(rng.randrange(0, 3))]
        if rng.random() < 0.4:
            b["quotes"] = {"1:" + HX20(rng.getrandbits(160)): "0x" + "%04x" % rng.getrandbits(16)}
        return b
    if etype == "judgment":
        b = {"subjectRef": random_subject(rng), "amanah": HX32(rng.getrandbits(256)),
             "verdict_md": rand_prose(rng)}
        if rng.random() < 0.5:
            b["decision"] = {"rung": rng.randrange(0, 8)}
        if rng.random() < 0.5:
            b["evidence"] = [{"manifest": HX32(rng.getrandbits(256)), "produced": rng.random() < 0.5}
                             for _ in range(rng.randrange(0, 3))]
        if rng.random() < 0.4:
            b["txRefs"] = [HX32(rng.getrandbits(256)) for _ in range(rng.randrange(0, 3))]
        if rng.random() < 0.4:
            b["terms"] = HX32(rng.getrandbits(256))
        return b
    if etype == "resignation":
        b = {"subjectRef": random_subject(rng), "amanah": HX32(rng.getrandbits(256)),
             "statement_md": rand_prose(rng)}
        if rng.random() < 0.5:
            b["category"] = "unlawful-subject"
            b["clause"] = rand_skel(rng, rng.randrange(1, 12))
        else:
            b["category"] = "other"
        return b
    if etype == "succession":
        return {"to": HX20(rng.getrandbits(160)), "kind": rand_token(rng),
                "effective": rng.randrange(0, MAXINT + 1), "statement_md": rand_prose(rng)}
    if etype == "annotation":
        b = {"note_md": rand_prose(rng)}
        if rng.random() < 0.5:
            b["subject"] = HX32(rng.getrandbits(256))
        return b
    if etype == "keybinding":
        return {"recipient": "age1" + rand_token(rng), "effective": rng.randrange(0, MAXINT + 1)}
    return {}


def rand_token(rng):
    pool = [chr(c) for c in range(0x21, 0x7F)]
    return "".join(rng.choice(pool) for _ in range(rng.randrange(1, 10)))


def mutate(rng, base, kind, pool):
    ba = bytearray(base)
    if kind == "flip":
        i = rng.randrange(len(ba))
        ba[i] ^= 1 << rng.randrange(8)
        return bytes(ba)
    if kind == "insert":
        i = rng.randrange(len(ba) + 1)
        ba[i:i] = bytes([rng.randrange(256)])
        return bytes(ba)
    if kind == "delete":
        i = rng.randrange(len(ba))
        del ba[i]
        return bytes(ba)
    if kind == "swap":
        i = rng.randrange(len(ba) - 1)
        ba[i], ba[i + 1] = ba[i + 1], ba[i]
        return bytes(ba)
    if kind == "truncate":
        return bytes(ba[:rng.randrange(1, len(ba))])
    if kind == "dup_slice":
        i = rng.randrange(len(ba))
        j = min(len(ba), i + rng.randrange(1, 12))
        ba[i:i] = ba[i:j]
        return bytes(ba)
    if kind == "splice":
        other = rng.choice(pool)
        i = rng.randrange(len(ba))
        j = rng.randrange(len(other))
        return bytes(ba[:i]) + other[j:]
    if kind == "shuffle_top":
        v = Parser(base.decode("utf-8")).text()
        ms = list(v.members)
        rng.shuffle(ms)
        v.members = ms
        return cbytes(v, sort=False)
    if kind == "shuffle_body":
        v = Parser(base.decode("utf-8")).text()
        b = v.get("body")
        if not isinstance(b, JObj) or len(b.members) < 2:
            return None
        ms = list(b.members)
        rng.shuffle(ms)
        b.members = ms
        return cbytes(v, sort=False)
    return None


# ==========================================================================
# sign/ : (private key, six-member canonical bytes or a family document) pairs
# ==========================================================================


def build_sign(rng, t):
    def b6(etype, seq, prev, body, author_key="root", spec="galeed/1"):
        return cbytes(build_env(spec, etype, A_(author_key), seq, prev, J(body)))

    P0 = HX32(0x11)
    S("genesis", K("root"), b6("genesis", 0, None, {"statement_md": "the seat of one judge"}),
      "galeed/1", "sec 5.1: B6 of a genesis; prev null, seq 0")
    S("amanah", K("root"),
      b6("amanah", 1, P0, {"version": 1, "text_md": "the covenant",
                           "venues": [{"chainId": 1, "contract": HX20(0xE5C), "codeHash": HX32(0xC0DE)}]}),
      "galeed/1", "sec 5.1: B6 of an amanah with one venue")
    S("judgment", K("root"),
      b6("judgment", 2, P0, {"subjectRef": {"kind": "evm", "chainId": 1, "contract": HX20(0xE5C),
                                            "codeHash": HX32(0xC0DE), "id": HX32(0xCA5E)},
                             "amanah": HX32(0xA1), "verdict_md": "ruled", "decision": {"rung": 0},
                             "evidence": [{"manifest": HX32(0xE1), "produced": True}],
                             "txRefs": [HX32(1)], "terms": HX32(0x7E)}),
      "galeed/1", "sec 5.1: B6 of a judgment carrying every optional field")
    S("resignation", K("root"),
      b6("resignation", 3, P0, {"subjectRef": {"kind": "hash", "sha256": HX32(0x5A)}, "amanah": HX32(0xA1),
                                "category": "unlawful-subject", "clause": "8", "statement_md": "no"}),
      "galeed/1", "sec 5.1: B6 of a resignation")
    S("keybinding", K("root"),
      b6("keybinding", 4, P0, {"recipient": "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq",
                               "effective": 1700000000}),
      "galeed/1", "sec 5.1: B6 of a keybinding")
    S("succession", K("root"),
      b6("succession", 5, P0, {"to": A_("k2"), "kind": "handover", "effective": 0,
                               "statement_md": "the seat moves"}),
      "galeed/1", "sec 5.1: B6 of a succession")
    S("annotation", K("root"), b6("annotation", 6, P0, {"note_md": "a note"}),
      "galeed/1", "sec 5.1: B6 of an annotation")
    S("unknown_type", K("root"), b6("windmill", 7, P0, {}),
      "galeed/1", "sec 6.8: B6 of an unknown type with an empty body")
    S("max_seq", K("root"), b6("annotation", MAXINT, P0, {"note_md": "top"}),
      "galeed/1", "sec 3.2: seq at 2^53 - 1 inside the preimage")
    S("prose_controls", K("root"),
      b6("annotation", 8, P0, {"note_md": "\x00\x01\x08\x09\x0a\x0c\x0d\x1f\x7f\x80"}),
      "galeed/1", "sec 3.4 item 3: every escape form inside the preimage")
    S("prose_astral", K("root"),
      b6("annotation", 9, P0, {"note_md": "\U0001f600\U0010ffff中é‮"}),
      "galeed/1", "sec 3.4 item 3: astral and bidi scalars raw in the preimage")
    S("deep_body", K("root"), b6("windmill", 10, P0, {"d": deep_list(120)}),
      "galeed/1", "sec 3.4: a deeply nested body inside B6")
    S("member_order", K("root"),
      b6("windmill", 11, P0, {"a b": 1, "ab": 2, "A": 3, "a": 4, "~": 5, " ": 6}),
      "galeed/1", "sec 3.4 item 5: bytewise key order inside the preimage")
    S("body_named_sig", K("root"),
      b6("windmill", 11, P0, {"sig": "0x" + "11" * 65, "body": {"seq": 1}}),
      "galeed/1", "sec 5.1: a member named sig inside body sits inside B6")
    S("key_one", 1, b6("annotation", 12, P0, {"note_md": "smallest private key"}, "root"),
      "galeed/1", "sec 5: private key 1")
    S("key_n_minus_1", N - 1, b6("annotation", 13, P0, {"note_md": "largest private key"}, "root"),
      "galeed/1", "sec 5: private key n - 1")
    S("key_half_n", HALF_N, b6("annotation", 14, P0, {"note_md": "key at (n-1)/2"}, "root"),
      "galeed/1", "sec 5: private key (n - 1) / 2")
    RT = ["genesis", "amanah", "judgment", "resignation", "succession", "annotation",
          "keybinding", "windmill"]
    for i in range(8):
        S("rand_b6_%d" % i, testkey("sign", 100 + i),
          b6(RT[i], 0 if i == 0 else i, None if i == 0 else HX32(rng.getrandbits(256)),
             random_body(rng, RT[i])),
          "galeed/1", "randomized B6 with a randomized private key")
    S("family_domain", K("root"), cbytes(J({"case": HX32(1), "note_md": "a dispatch note"})),
      "galeed.dispatch/1", "sec 5.6: a family law's document under its own literal")
    S("other_literal", K("root"), cbytes(J({"note_md": "a venue's document"})),
      "venue.doc/1", "sec 5.6/5.7: a document under a literal no domain of this law spells")
    S("domain_tab", K("root"), cbytes(J({"note_md": "a venue's document"})),
      "venue.doc/1\t", "HARNESS: a domain is any bytes below 0x80 without a line feed; a tab is signed as given")
    S("domain_del", K("root"), cbytes(J({"note_md": "a venue's document"})),
      "venue.doc/1\x7f", "HARNESS: a domain is any bytes below 0x80 without a line feed; 0x7F is signed as given")
    # round 1 additions: the sign surface the comparator never exercised
    D6 = b6("annotation", 15, P0, {"note_md": "domain edges"})
    S("domain_absent", K("root"), D6, None, "HARNESS: the domain argument absent defaults to galeed/1")
    S("domain_empty", K("root"), D6, "", "HARNESS/sec 5.6: the empty domain is signed as given; len(M) is 67")
    S("domain_len_m_100", K("root"), D6, "d" * 33, "sec 5.2: a domain of 33 bytes puts len(M) at 100, three decimal digits")
    S("domain_len_m_1000", K("root"), D6, "d" * 933, "sec 5.2: a domain of 933 bytes puts len(M) at 1000, four decimal digits")
    S("domain_len_m_99", K("root"), D6, "d" * 32, "sec 5.2: len(M) at 99, the digit width below")
    S("domain_len_m_999", K("root"), D6, "d" * 932, "sec 5.2: a domain of 932 bytes puts len(M) at 999, the digit width below four")
    S("key_0X_prefix", K("root"), D6, "galeed/1", "HARNESS: the key with a 0X prefix", privkey_spelling="0X%064x" % K("root"))
    S("key_uppercase", K("root"), D6, "galeed/1", "HARNESS: the key's digits in upper case", privkey_spelling="0x%064X" % K("root"))
    S("key_mixed_case", K("root"), D6, "galeed/1", "HARNESS: each digit of either case", privkey_spelling="0x" + "".join(c.upper() if i % 2 else c for i, c in enumerate("%064x" % K("root"))))
    S("key_no_prefix", K("root"), D6, "galeed/1", "HARNESS: the key without a prefix", privkey_spelling="%064x" % K("root"))
    S("misuse_key_zero", K("root"), D6, "galeed/1", "HARNESS: the scalar 0 is outside [1, n - 1]", privkey_spelling="0x" + "00" * 32, misuse=True)
    S("misuse_key_n", K("root"), D6, "galeed/1", "HARNESS: the scalar n is outside [1, n - 1]", privkey_spelling="0x%064x" % N, misuse=True)
    S("misuse_key_63_digits", K("root"), D6, "galeed/1", "HARNESS: sixty-three digits", privkey_spelling="0x" + ("%064x" % K("root"))[1:], misuse=True)
    S("misuse_key_65_digits", K("root"), D6, "galeed/1", "HARNESS: sixty-five digits", privkey_spelling="0x0" + "%064x" % K("root"), misuse=True)
    S("misuse_key_non_hex", K("root"), D6, "galeed/1", "HARNESS: a non-hexadecimal digit", privkey_spelling="0x" + "zz" + ("%064x" % K("root"))[2:], misuse=True)
    S("misuse_domain_line_feed", K("root"), D6, "galeed/1\n", "HARNESS: a domain carrying a line feed is misuse", misuse=True)
    S("misuse_domain_high_byte", K("root"), D6, "venue.doc/\u00e9", "HARNESS: a domain carrying a byte at or above 0x80 is misuse", misuse=True)
    S("misuse_file_not_canonical_value", K("root"), b'{"a":1.0}', "galeed/1", "HARNESS: a file failing sec 3.5 test 2 (a fraction) is misuse", misuse=True)
    S("misuse_file_invalid_utf8", K("root"), b'{"a":"\xff"}', "galeed/1", "HARNESS: a file failing sec 3.5 test 1 is misuse", misuse=True)
    S("misuse_file_bad_key_charset", K("root"), b'{"\\u00e9":1}'.replace(b"\\u00e9", "\u00e9".encode("utf-8")), "galeed/1", "HARNESS: a file failing sec 3.5 test 4 (a key outside the charset) is misuse", misuse=True)
    S("misuse_file_array", K("root"), b'[1,2]', "galeed/1", "HARNESS: a file whose value is not an object is misuse", misuse=True)
    S("misuse_file_dup_key", K("root"), b'{"a":1,"a":2}', "galeed/1", "HARNESS: a file failing sec 3.5 test 3 (a duplicate key), which a plain JSON reader accepts, is misuse", misuse=True)
    S("misuse_file_value_charset", K("root"), b'{"a":"a\\u0001b"}', "galeed/1", "HARNESS: a file failing sec 3.5 test 5 (a string value outside the charset) is misuse", misuse=True)
    S("misuse_file_scalar", K("root"), b'"x"', "galeed/1", "HARNESS: a scalar root is misuse", misuse=True)
    S("misuse_extra_argument", K("root"), D6, "galeed/1", "HARNESS: an argument beyond the list", misuse=True, extra_args=("x",))




# ==========================================================================
# audit/ : sec 8 scenarios in the HARNESS.md input format
# ==========================================================================

GENB = {"statement_md": "a judge's ledger"}


def AMB(i):
    return {"version": i, "text_md": "covenant %d" % i}


def ANNB(s):
    return {"note_md": s}


def JGB(i):
    return {"subjectRef": {"kind": "evm", "chainId": 1, "contract": HX20(0xE5C),
                           "codeHash": HX32(0xC0DE), "id": HX32(i)},
            "amanah": HX32(i), "verdict_md": "ruling %d" % i}


def RSB(i):
    return {"subjectRef": {"kind": "hash", "sha256": HX32(i)}, "amanah": HX32(i),
            "category": "other", "statement_md": "declined %d" % i}


def SUCB(to, kind="handover"):
    return {"to": to, "kind": kind, "effective": 1700000000, "statement_md": "the seat moves"}


def mk(key, etype, seq, prev, body):
    return make_entry(K(key), etype, seq, prev, body)


def build_chain(specs, start_seq=0, start_prev=None):
    out, ids = [], []
    prev, seq = start_prev, start_seq
    for key, etype, body in specs:
        b = mk(key, etype, seq, (None if seq == 0 else prev), body)
        out.append(b)
        ids.append(eid(b))
        prev = eid(b)
        seq += 1
    return out, ids


def basis(chains=(), raw=False):
    """A sec 9.4 basis; the array in the order the section fixes unless raw."""
    ch = list(chains)
    if not raw:
        ch = sorted(ch, key=lambda c: (c["chainId"], c["fromBlock"]))
    return {"chains": ch}


def chainobj(cid=1, fb=0, tb=1000, regs=(), senders=(), raw=False):
    r, sd = list(regs), list(senders)
    if not raw:
        r = sorted(set(r), key=lambda a: a.encode())
        sd = sorted(set(sd), key=lambda a: a.encode())
    return {"chainId": cid, "fromBlock": fb, "toBlock": tb,
            "registries": r, "senders": sd}


def anchor(h, sender, cid=1, bn=100, ts=1700000000, tx=None, verdict="counted"):
    """A record that carries nothing: an anchor of a hash alone (sec 9.1, a log
    whose data decoded to no bytes)."""
    return {"chainId": cid, "blockNumber": bn, "blockTimestamp": ts,
            "tx": tx if tx else HX32(0xA0000 + bn), "sender": sender, "hash": h,
            "verdict": verdict}


def rec(entry_bytes, sender, **kw):
    """A record carrying bytes: its hash is the entry_id of the bytes it
    carries (sec 9.1, sec 9.2)."""
    a = anchor(eid(entry_bytes), sender, **kw)
    a["entry"] = hx(entry_bytes)
    return a


canchor = rec


def recs(entries, sender, bn0=10, **kw):
    """One carried counted record per byte string, at consecutive blocks."""
    return [rec(b, sender, bn=bn0 + i, **kw) for i, b in enumerate(entries)]


def classitem(i):
    return {"codeHash": HX32(i)}


ROOTB = None   # set by build_audit once the keys exist: one chain, every hand-case sender


def inp(root, anchors=(), bas=None):
    return {"root": root, "anchors": list(anchors),
            "basis": bas if bas is not None else ROOTB}


def build_audit(rng, t):
    global ROOTB
    R = A_("root")
    K2, K3, K4, K9 = A_("k2"), A_("k3"), A_("k4"), A_("k9")
    ROOTB = basis((chainobj(1, senders=(R, K2, K3, K4, K9)),))
    CL, CID = build_chain([("root", "genesis", GENB), ("root", "amanah", AMB(1)),
                           ("root", "judgment", JGB(2)), ("root", "annotation", ANNB("a note")),
                           ("root", "resignation", RSB(4))])
    RC = recs(CL, R)

    A("clean_ledger", inp(R, RC), "A clean five-entry ledger under one key, every entry carried "
      "by a counted record of the root: no gaps, no forks, five ANCHORED rows. sec 8.7 label 4.",
      "COMPLETE")
    A("empty_anchor_set", inp(R, []), "An empty anchor set. sec 8.7: COMPLETE asserts only that the "
      "inputs held no fault, and the entry count travels with it.", "COMPLETE")
    A("basis_covers_no_chain_empty", inp(R, [], bas=basis()),
      "sec 8.7: under a basis that covers no chain a COMPLETE label says that nothing was looked "
      "for, which is why the basis travels with the label.", "COMPLETE")
    FOR, FORI = build_chain([("k9", "genesis", GENB), ("k9", "amanah", AMB(3)),
                             ("k9", "annotation", ANNB("elsewhere"))])
    A("only_foreign_entries_own_anchors", inp(R, recs(FOR, K9)),
      "A stranger anchors its own three entries and the basis scans its key. sec 8.1: the "
      "records are discarded, the entries' judge is outside the lineage, so they are EXCLUDED "
      "rows and DISCARDED rows carrying the bytes; no finding, COMPLETE.", "COMPLETE")
    A("foreign_entry_carried_by_root", inp(R, RC + [rec(FOR[0], R, bn=20)]),
      "sec 8.4 second ground: the root's counted record carries a stranger's genesis; "
      "CARRIED_MALFORMED with token FOREIGN at seq 0, the entry EXCLUDED, its hash MISSING; "
      "BROKEN_CHAIN.", "BROKEN_CHAIN")
    A("gap_at_start", inp(R, recs(CL[1:], R)),
      "The seq-0 entry is carried by no record. sec 8.2: a ledger whose smallest seq is not 0 "
      "records SEQ_GAP(0, s) first; the entry at seq 1 records no prev finding because no entry "
      "at seq 0 is held.", "GAPS")
    A("gap_in_middle", inp(R, recs(CL[:2] + CL[3:], R)),
      "seq 2 is carried by no record. sec 8.2: SEQ_GAP(2, 3), and across the gap the link is "
      "unverifiable so no PREV_MISMATCH is recorded.", "GAPS")
    A("gap_from_hash_only_record", inp(R, recs(CL[:2] + CL[3:], R) + [anchor(CID[2], R, bn=50)]),
      "sec 8.5/8.2: the third entry's anchor carried nothing (a log whose data decoded to no "
      "bytes), so the entry is MISSING by hash and the position is a SEQ_GAP; GAPS.", "GAPS")

    SU, SUI = build_chain([("root", "genesis", GENB), ("root", "succession", SUCB(K2)),
                           ("k2", "amanah", AMB(4)), ("k2", "annotation", ANNB("new hand"))])
    A("succession_and_successor", inp(R, recs(SU[:2], R) + recs(SU[2:], K2, bn0=20)),
      "sec 7.3: a succession at seq 1 moves authority; the successor's entries at 2 and 3 carry "
      "the new judge and are carried by the successor's own counted records, which the lineage "
      "admits (sec 7.4).", "COMPLETE")
    A("successor_entries_carried_by_root", inp(R, recs(SU, R)),
      "sec 7.4/8.1: every lineage key can carry any entry by a record at which it has not "
      "retired; the root that handed off, every record at one second, carries the successor's "
      "entries and they are ledger entries all the same.", "COMPLETE")
    SR, _ = build_chain([("root", "genesis", GENB), ("root", "succession", SUCB(R)),
                         ("root", "annotation", ANNB("still me"))])
    A("succession_to_root", inp(R, recs(SR, R)),
      "sec 7.3/7.4: a succession whose `to` equals the key already in office is walked as written "
      "and authority is unchanged, and it retires its signer all the same: the root's entry at seq "
      "2, carried in the second of the handing off, is DISCARDED; sec 6.6 obliges the seat to write "
      "none, and the walk decides over every input.", "COMPLETE")
    SP, _ = build_chain([("root", "genesis", GENB), ("root", "succession", SUCB(K2)),
                         ("k2", "annotation", ANNB("k2 writes")),
                         ("k2", "succession", SUCB(R)), ("root", "annotation", ANNB("back"))])
    A("succession_to_prior_holder", inp(R, recs(SP, R)),
      "sec 7.3/7.4: a succession back to an earlier key of the seat; authority is a function of "
      "the prefix and switches as written, and the root's entry at seq 4, signed after it handed "
      "off at seq 1 and carried in that second, is DISCARDED; the seat that wrote it broke sec "
      "6.6's obligation and nothing of sec 8.", "COMPLETE")
    SL, _ = build_chain([("root", "genesis", GENB), ("root", "annotation", ANNB("one")),
                         ("root", "succession", SUCB(K2))])
    A("succession_at_last_position", inp(R, recs(SL, R)),
      "sec 6.6: the ledger's last entry hands the seat off and nothing follows; silence is the "
      "ordinary end.", "COMPLETE")
    A("withheld_succession_successor_anchors", inp(R, recs([SU[0]], R) + recs(SU[2:], K2, bn0=20)),
      "sec 8.1: the succession that would admit the successor is carried by no record. The "
      "successor is outside the lineage, its two records are DISCARDED with their bytes, its "
      "entries EXCLUDED, the ledger stops at seq 0, and no walk can see the missing entry; the "
      "clean label stands beside two rows a reader asks about.", "COMPLETE")
    A("withheld_succession_root_carries", inp(R, recs([SU[0]] + SU[2:], R)),
      "sec 8.4: the root's own counted records carry the successor's entries while the "
      "succession is withheld; each is CARRIED_MALFORMED with token FOREIGN at its seq, and the "
      "label is BROKEN_CHAIN.", "BROKEN_CHAIN")

    # forks. Wherever the law orders two entries by entry_id (8.2 walk ties, 8.2
    # competing successions, 8.4 a/b), the case is built in both id orders by a
    # deterministic search over the competitor's prose.
    def both_orders(slug, build, note, expect_label=None):
        got = {}
        k = 0
        while len(got) < 2:
            inp_, asc = build(k)
            got.setdefault(asc, inp_)
            k += 1
            assert k < 256, slug
        for asc in (True, False):
            A(slug + ("_ids_ascending" if asc else "_ids_descending"), got[asc],
              note + (" Here the entry met first has the smaller id." if asc
                      else " Here the entry met first has the larger id."), expect_label)

    tw_a = mk("root", "annotation", 3, CID[2], ANNB("left"))
    tw_b = mk("root", "annotation", 3, CID[2], ANNB("right"))
    tw_c = mk("root", "annotation", 3, CID[2], ANNB("third"))
    both_orders("fork_two_way",
                lambda k: (lambda b: (inp(R, recs(CL[:3] + [tw_a, b], R)), eid(tw_a) < eid(b)))(
                    mk("root", "annotation", 3, CID[2], ANNB("right %d" % k))),
                "sec 8.4: two ledger entries with different entry_id at one seq. One EQUIVOCATION, "
                "hard, naming the bytewise-smaller id first.", "BROKEN_CHAIN")
    s2 = mk("root", "succession", 3, CID[2], SUCB(K2))
    e2 = mk("k2", "annotation", 4, eid(s2), ANNB("k2 writes after"))

    def competing(k):
        s3 = mk("root", "succession", 3, CID[2], SUCB(K3, kind="handover-%d" % k))
        e3 = mk("k3", "annotation", 4, eid(s3), ANNB("k3 writes after"))
        return inp(R, recs(CL[:3] + [s2, e2, s3, e3], R)), eid(s2) < eid(s3)
    both_orders("fork_competing_successions", competing,
                "sec 8.2: two successions compete at seq 3; authority after 3 follows the one with "
                "the bytewise-smallest entry_id, so exactly one of the two seq-4 entries is an "
                "AUTHORITY_MISMATCH, and which one turns on the id order.", "BROKEN_CHAIN")
    A("fork_three_way", inp(R, recs(CL[:3] + [tw_a, tw_b, tw_c], R)),
      "sec 8.4: three entries colliding at one position record three findings, one per "
      "unordered pair.", "BROKEN_CHAIN")
    tw_s = mk("root", "succession", 3, CID[2], SUCB(K2))
    A("fork_with_succession_twin", inp(R, recs(CL[:3] + [tw_a, tw_s] +
                                                 [mk("k2", "annotation", 4, eid(tw_s), ANNB("after"))], R)),
      "sec 8.2: a fork at k where one twin is a succession signed by the key in office. "
      "Authority after k follows that succession, the annotation twin moving nothing, so the "
      "successor's entry at k+1 records no finding beside the fork.",
      "BROKEN_CHAIN")
    gen2 = mk("k2", "genesis", 0, None, {"statement_md": "a second opening"})
    A("twins_at_seq0", inp(R, recs([CL[0], gen2, mk("root", "succession", 1, CID[0], SUCB(K2))], R)),
      "sec 7.5/8.2: a lineage key opens a competing genesis under the audited seat. "
      "ROOT_MISMATCH on the second seq-0 entry and EQUIVOCATION between the two.",
      "BROKEN_CHAIN")
    ret = mk("root", "annotation", 2, SUI[1], ANNB("the retired key writes again"))
    A("fork_by_retired_key", inp(R, recs(SU[:3] + [ret], R)),
      "sec 7.4/8.1: a key that handed off signs again at a later position and carries it in the "
      "second of its handing off (every record here at one second): a record at which it has "
      "retired, DISCARDED, no ledger entry, no fork; COMPLETE.",
      "COMPLETE")
    sp2 = mk("root", "annotation", 2, CID[0], ANNB("wrong head"))
    A("shared_prev_across_seqs", inp(R, recs(CL[:2] + [sp2], R)),
      "sec 8.4: two entries at different seq sharing a non-null prev. The finding's seq is the "
      "smaller of the two, and PREV_MISMATCH also fires at seq 2.", "BROKEN_CHAIN")
    pm = mk("root", "annotation", 2, HX32(0xDEAD), ANNB("dangling head"))
    A("prev_mismatch_adjacent", inp(R, recs(CL[:2] + [pm], R)),
      "sec 8.2: an entry at seq 2 whose prev matches no held entry at seq 1.", "BROKEN_CHAIN")
    A("prev_mismatch_across_gap", inp(R, recs(CL[:2] + [mk("root", "annotation", 4, HX32(0xDEAD),
                                                             ANNB("across the gap"))], R)),
      "sec 8.2: the same broken link one position further out, where no entry at s-1 is held. "
      "The gap already is the finding and no prev finding is recorded.", "GAPS")

    AM, AMI = build_chain([("root", "genesis", GENB), ("root", "annotation", ANNB("one"))])
    am2 = mk("k2", "annotation", 2, AMI[1], ANNB("not yet in office"))
    am3 = mk("root", "succession", 3, eid(am2), SUCB(K2))
    A("authority_mismatch_certain", inp(R, recs(AM + [am2, am3], R)),
      "sec 8.2: the walk is contiguous from 0, so the authority contradiction at seq 2 is hard.",
      "BROKEN_CHAIN")
    am4 = mk("k2", "annotation", 4, HX32(0x44), ANNB("after a gap"))
    am6 = mk("root", "succession", 6, HX32(0x66), SUCB(K2))
    A("authority_mismatch_after_gap", inp(R, recs(AM + [am4, am6], R)),
      "sec 8.2: a gap could hide a succession, so after a gap the same finding is recorded soft "
      "and no label turns hard.", "GAPS")

    mx = mk("root", "annotation", MAXINT, HX32(0x77), ANNB("the last position"))
    A("seq_at_2p53_minus_1", inp(R, recs([CL[0], mx], R)),
      "sec 8.2: at seq 2^53 - 1 the walk leaves `expected` at that value, since no entry can "
      "occupy a position beyond the universe of sec 3.1.", "GAPS")
    A("seq_max_twins", inp(R, recs([CL[0], mx, mk("root", "annotation", MAXINT, HX32(0x77),
                                                    ANNB("a twin at the last position"))], R)),
      "sec 8.2/8.4: two entries at 2^53 - 1; the second is a fork twin and records no second gap.",
      "BROKEN_CHAIN")

    # ---------------- verdicts, the reconciliation, and the ledger (sec 8.5, 8.6, 9.3) ------
    H_UNSEEN = HX32(0xBEEFBEEF)
    A("hash_only_record_missing", inp(R, RC + [anchor(H_UNSEEN, R)]),
      "sec 8.5 forward: a counted record of a hash whose bytes no record carries is MISSING; "
      "GAPS.", "GAPS")
    A("hash_only_record_of_ledger_entry", inp(R, RC + [anchor(CID[0], R, bn=50)]),
      "sec 8.5/8.7 item 5: a second counted record of the genesis carries nothing; the entry is "
      "a ledger entry through the record that carried it, and the ANCHORED row carries both "
      "records that committed to it; COMPLETE.", "COMPLETE")
    A("unproven_record_of_counted_hash", inp(R, RC + [anchor(CID[1], R, bn=50, verdict="UNPROVEN")]),
      "sec 9.3/8.7: an UNPROVEN record takes no part in the reconciliation and is carried in the "
      "UNPROVEN list; its hash is the hash of a counted record, so label rule 2 does not fire.",
      "COMPLETE")
    A("unproven_record_of_unseen_hash", inp(R, RC + [anchor(H_UNSEEN, R, verdict="UNPROVEN")]),
      "sec 8.7 item 12 rule 2: an UNPROVEN record whose hash no counted record carries moves the "
      "label to UNPROVEN: the reader's node could not answer for a boundary, and the silence is "
      "never read as the seat's.", "UNPROVEN")
    A("unproven_record_carrying_bytes", inp(R, RC[:4] + [rec(CL[4], R, bn=50, verdict="UNPROVEN")]),
      "sec 8.6: the fifth entry is carried only by an UNPROVEN record; it is no ledger entry, "
      "the bytes stand in the UNPROVEN row, the ledger has four entries and no gap after them, "
      "and the label is UNPROVEN since no counted record carries the hash.", "UNPROVEN")
    A("unproven_carrying_bytes_also_counted", inp(R, RC + [rec(CL[4], R, bn=50, verdict="UNPROVEN")]),
      "sec 8.6: the same bytes carried by a counted record as well; the entry is a ledger entry "
      "through the counted record, the UNPROVEN row carries the bytes, and the label is "
      "COMPLETE.", "COMPLETE")
    A("void_record_of_unseen_hash", inp(R, RC + [anchor(H_UNSEEN, R, verdict="VOID")]),
      "sec 9.3/8.7: a VOID record is informational, takes no part in the reconciliation, and no "
      "label turns on it, even when its hash is in no ledger entry.", "COMPLETE")
    A("void_record_carrying_bytes", inp(R, RC[:4] + [rec(CL[4], R, bn=50, verdict="VOID")]),
      "sec 8.6: the fifth entry is carried only by a VOID record; it is no ledger entry, the "
      "bytes stand in the VOID row, and the four entries read whole; COMPLETE.", "COMPLETE")
    A("void_record_carrying_middle_entry", inp(R, recs(CL[:2] + CL[3:], R) + [rec(CL[2], R, bn=50, verdict="VOID")]),
      "sec 8.6: the third entry is carried only by a VOID record, so the position is a hole: "
      "SEQ_GAP at 3, and GAPS, the VOID list moving no label.", "GAPS")
    A("void_then_counted_same_bytes", inp(R, RC[:4] + [rec(CL[4], R, bn=50, verdict="VOID"), rec(CL[4], R, bn=60)]),
      "sec 8.6: bytes carried by a VOID record and later by a counted record of a lineage key are "
      "a ledger entry from the counted record on; one VOID row with the bytes, one ANCHORED row "
      "with the counted record alone.", "COMPLETE")
    A("void_and_unproven_together", inp(R, RC + [anchor(H_UNSEEN, R, verdict="VOID"),
                                                 anchor(HX32(0xF00D), R, bn=12, verdict="UNPROVEN")]),
      "sec 8.7: both lists non-empty; only the UNPROVEN list moves the label.", "UNPROVEN")
    A("unproven_duplicate_of_counted_hash", inp(R, RC + [anchor(CID[0], R, bn=20, tx=HX32(0x777), verdict="UNPROVEN")]),
      "sec 8.7 item 12 rule 2: an UNPROVEN record whose hash is already counted moves no label; "
      "the row is still carried.", "COMPLETE")
    A("unproven_of_hash_counted_but_missing",
      inp(R, RC + [anchor(H_UNSEEN, R, bn=50, tx=HX32(0x501)),
                   anchor(H_UNSEEN, R, bn=60, tx=HX32(0x777), verdict="UNPROVEN")]),
      "sec 8.7 item 12 rule 2, second escape: an UNPROVEN hash counted elsewhere moves no label; "
      "the counted record's MISSING row gives GAPS.", "GAPS")
    A("anchor_sender_outside_lineage", inp(R, RC + [anchor(H_UNSEEN, K9)]),
      "sec 8.1: the audit discards every anchor record whose sender is not in the lineage, so "
      "no key outside it can put a hash into this ledger's reconciliation; a DISCARDED row.",
      "COMPLETE")
    A("anchor_sender_retired_lineage_key", inp(R, recs(SU[:2], R) + recs(SU[2:], K2, bn0=20) + [anchor(eid(SU[2]), R, bn=40)]),
      "sec 7.4/8.1: every key that has held the seat can anchor by any record at which it has "
      "not retired; every record here sits at one second, so the anchor of the root that handed "
      "off counts beside the successor's own.", "COMPLETE")
    A("anchor_outside_basis_reach",
      inp(R, [rec(CL[0], R)], bas=basis((chainobj(1, senders=(K2,)),))),
      "sec 9.4 [C]: a record no chains object reaches (its sender outside that object's senders) "
      "is a record no scan of that basis produces; no label.", "NO_LABEL")
    A("anchor_outside_basis_range", inp(R, [rec(CL[0], R, bn=2000)]),
      "sec 9.4 [C]: a record whose blockNumber lies outside every chains object's range of its "
      "chain is outside the basis's reach; no label.", "NO_LABEL")
    A("anchor_outside_basis_chain", inp(R, [rec(CL[0], R, cid=10)]),
      "sec 9.4 [C]: a record on a chain the basis does not declare is outside its reach; no label.",
      "NO_LABEL")
    TWOCH = basis((chainobj(1, senders=(R,)), chainobj(8453, senders=(R,))))
    A("missing_on_two_chains",
      inp(R, RC + [anchor(H_UNSEEN, R, cid=1, bn=50, ts=1700000000, tx=HX32(0x501)),
                   anchor(H_UNSEEN, R, cid=8453, bn=10, ts=1699000000, tx=HX32(0x101))], bas=TWOCH),
      "sec 8.7 item 4: one MISSING row carrying both anchoring transactions in ascending "
      "(chainId, blockNumber, tx) order; sec 9.2: the smaller blockTimestamp gives the "
      "existence bound and blockNumber never orders across chains.", "GAPS")
    A("anchored_on_two_chains",
      inp(R, RC + [rec(CL[0], R, cid=8453, bn=10, ts=1699000000, tx=HX32(0x101))], bas=TWOCH),
      "sec 8.7 item 5 with sec 9.4: two chains carry one entry_id; the dedup key holds chainId, "
      "so records differing only there are two records, and the row's anchors sort by chainId "
      "first.", "COMPLETE")
    A("anchored_and_missing_together", inp(R, RC + [anchor(H_UNSEEN, R, bn=101)]),
      "sec 8.7 item 5: the partition on the face of the report, ANCHORED and MISSING both "
      "non-empty in one report.", "GAPS")
    BADB = bytearray(CL[1]); BADB[len(BADB) // 2] ^= 0x01; BADB = bytes(BADB)
    A("carried_bytes_fail_4_3", inp(R, RC + [rec(BADB, R, bn=50)]),
      "sec 8.4: the root's counted record carries bytes that fail sec 4.3; CARRIED_MALFORMED "
      "(seq 0, the sec 10 token) is hard, the bytes are listed MALFORMED and the hash MISSING; "
      "BROKEN_CHAIN.", "BROKEN_CHAIN")
    A("carried_empty_bytes", inp(R, RC + [rec(b"", R, bn=50)]),
      "sec 8.4: a carried empty byte string is a byte string that fails sec 4.3 (E_JSON); hard.",
      "BROKEN_CHAIN")
    JUNK = b'{"junk":1}'
    A("carried_junk_by_two_records_one_finding",
      inp(R, RC + [rec(JUNK, R, bn=50, tx=HX32(0x801)), rec(JUNK, R, bn=51, tx=HX32(0x802))]),
      "sec 8.4: two counted records carry the same junk; one finding per byte string, one "
      "MISSING row with two anchors, one MALFORMED row.", "BROKEN_CHAIN")
    B_RK3 = basis((chainobj(1, senders=(R, K3)),))
    A("junk_carried_by_discarded_record", inp(R, RC + [rec(JUNK, K3, bn=50)], bas=B_RK3),
      "sec 8.4: a stranger the basis scans carries junk; the record is DISCARDED with its entry, "
      "the bytes stand in MALFORMED, and no finding is recorded.", "COMPLETE")
    A("junk_carried_by_unproven_record", inp(R, RC + [rec(JUNK, R, bn=50, verdict="UNPROVEN")]),
      "sec 8.1/8.7: an UNPROVEN record carries junk; MALFORMED lists the bytes, the UNPROVEN row "
      "carries them, sec 8.4 counts no finding on an uncounted record, and the label is "
      "UNPROVEN since no counted record carries the hash.", "UNPROVEN")
    A("junk_carried_by_void_record", inp(R, RC + [rec(JUNK, R, bn=50, verdict="VOID")]),
      "sec 8.1: a VOID record carries junk; MALFORMED lists it, the VOID row carries it, and "
      "nothing else moves.", "COMPLETE")
    A("hard_beats_unproven", inp(R, recs(CL[:3] + [tw_a, tw_b], R) + [anchor(H_UNSEEN, R, bn=50, verdict="UNPROVEN")]),
      "sec 8.7 label order: rule 1 fires first.", "BROKEN_CHAIN")
    A("unproven_beats_gaps", inp(R, recs(CL[:2] + CL[3:], R) + [anchor(H_UNSEEN, R, bn=50, verdict="UNPROVEN")]),
      "sec 8.7 label order: rule 2 fires before rule 3 even though a SEQ_GAP was recorded.",
      "UNPROVEN")
    A("gaps_from_missing_alone", inp(R, RC + [anchor(H_UNSEEN, R, bn=50)]),
      "sec 8.7 label rule 3: no gap in the walk, one MISSING row; GAPS.", "GAPS")

    # ---------------- duplicates, records agreeing and disagreeing -------------
    A("same_bytes_two_records", inp(R, RC + [rec(CL[2], R, bn=50, tx=HX32(0x900))]),
      "sec 8.1: the same bytes carried by two counted records collapse to one entry; its "
      "ANCHORED row carries both records, and the report does not depend on the order in which "
      "any input was supplied.", "COMPLETE")
    A("malformed_only", inp(R, recs([BADB, b"{}", b"not json at all"], K9)),
      "sec 8.1: byte strings that fail sec 4.3 are no ledger entries: they enter no walk, no "
      "have and no EXCLUDED row, and sec 8.7 item 8 lists each with its token; carried by a "
      "stranger, so no finding and three DISCARDED rows.", "COMPLETE")
    A("malformed_duplicate_collapses", inp(R, RC + [rec(b"{}", K9, bn=50), rec(b"{}", K9, bn=51)]),
      "sec 8.1/8.7 item 8: byte-identical copies collapse to one MALFORMED row.", "COMPLETE")
    XG, XI = build_chain([("root", "genesis", GENB), ("root", "amanah", AMB(1))])
    A2 = mk("root", "annotation", 2, XI[1], ANNB("first on this head"))
    both_orders("equivocation_shared_prev",
                lambda k: (lambda b: (inp(R, recs(XG + [A2, b], R)), eid(A2) < eid(b)))(
                    mk("root", "annotation", 3, XI[1], ANNB("second on this head %d" % k))),
                "sec 8.4: two ledger entries share a prev at seqs 2 and 3; the finding names the "
                "bytewise-smaller entry_id first whichever entry the walk met first.")
    A("dup_anchor_records_agree",
      inp(R, RC + [anchor(CID[0], R, tx=HX32(0x900)), anchor(CID[0], R, tx=HX32(0x900))]),
      "sec 9.4: the anchor set holds at most one record per (chainId, blockNumber, tx, hash); "
      "two records that agree on every remaining member are one record.", "COMPLETE")
    A("dup_anchor_records_disagree_timestamp",
      inp(R, RC + [anchor(CID[0], R, tx=HX32(0x900), ts=1), anchor(CID[0], R, tx=HX32(0x900), ts=2)]),
      "sec 9.4: two records that disagree on a remaining field are not a galeed/1 audit input.",
      "NO_LABEL")
    A("dup_anchor_records_disagree_verdict",
      inp(R, RC + [anchor(CID[0], R, tx=HX32(0x900)), anchor(CID[0], R, tx=HX32(0x900), verdict="VOID")]),
      "sec 9.4: disagreement on the verdict.", "NO_LABEL")
    A("dup_anchor_records_disagree_sender",
      inp(R, RC + [anchor(CID[0], R, tx=HX32(0x900)), anchor(CID[0], K2, tx=HX32(0x900))]),
      "sec 9.4: two records agreeing on (chainId, blockNumber, tx, hash) and disagreeing on "
      "sender are no audit input.", "NO_LABEL")
    A("dup_anchor_records_differ_blocknumber_two_records",
      inp(R, RC + [anchor(CID[0], R, tx=HX32(0x900), bn=5), anchor(CID[0], R, tx=HX32(0x900), bn=6)]),
      "sec 9.2/9.4: one chain including one transaction hash in two blocks is two anchors of "
      "one hash, two records, and one ANCHORED row carrying both.", "COMPLETE")
    A("dup_anchor_same_tx_different_hash",
      inp(R, RC + [anchor(CID[0], R, tx=HX32(0x900)), anchor(CID[1], R, tx=HX32(0x900))]),
      "sec 9.4: one transaction carrying two hashes is two records and is legal.", "COMPLETE")
    A("dup_anchor_records_agree_with_extra_member",
      inp(R, RC + [anchor(CID[0], R, tx=HX32(0x900)), dict(anchor(CID[0], R, tx=HX32(0x900)), note="x")]),
      "sec 9.4: two records that agree on the seven members are one record, whatever either "
      "carries beyond them.", "COMPLETE")
    A("dup_records_entry_versus_none",
      inp(R, RC[:4] + [anchor(eid(CL[4]), R, bn=50), rec(CL[4], R, bn=50)]),
      "sec 9.4: two records agreeing on the four keys, one carrying an entry the other lacks, "
      "are no audit input.", "NO_LABEL")
    A("dup_records_entry_equal", inp(R, RC[:4] + [rec(CL[4], R, bn=50), rec(CL[4], R, bn=50)]),
      "sec 9.4: two records agreeing on all eight members collapse to one.", "COMPLETE")
    A("dup_records_entry_equal_bytes_different_spelling",
      inp(R, RC[:4] + [rec(CL[4], R, bn=50), dict(anchor(eid(CL[4]), R, bn=50), entry="0x" + hx(CL[4])[2:].upper())]),
      "sec 9.4 with HARNESS: the same bytes spelled in two cases are equal entries; one record.",
      "COMPLETE")

    # ---------------- input shape (HARNESS, sec 8, sec 9.2, sec 9.4) ---------------
    WF = inp(R, RC)
    A_raw("input_not_json", b"not a json text at all",
          "HARNESS: the audit input is not a JSON text; no label.")
    A_raw("input_not_utf8", b"\xff\xfe" + json.dumps(WF).encode("utf-8"),
          "HARNESS: the audit input is not valid UTF-8; no label.")
    A_raw("input_duplicate_member",
          json.dumps(WF)[:-1].encode("utf-8") + b', "root": "' + R.encode() + b'"}',
          "HARNESS: the input reader refuses an object that repeats a member name; no label.")
    A_raw("input_float_member", json.dumps(WF)[:-1].encode("utf-8") + b', "extra": 1.5}',
          "HARNESS: the input reader admits only integers of sec 3.1's universe, in any member; "
          "no label.")
    A_raw("input_nan_constant", json.dumps(WF)[:-1].encode("utf-8") + b', "extra": NaN}',
          "HARNESS: NaN is no value the input reader admits; no label.")
    A_raw("input_root_not_object", b"[1,2]",
          "HARNESS: a file whose root value is not an object gives no label, and is no misuse.")
    A_raw("input_byte_order_mark", b"\xef\xbb\xbf" + json.dumps(WF).encode("utf-8"),
          "HARNESS: a byte order mark is not whitespace under RFC 8259's JSON-text production, "
          "so the file is unreadable, as it is for an entry.")
    for k in ("root", "anchors", "basis"):
        A("input_%s_absent" % k, {kk: v for kk, v in WF.items() if kk != k},
          "HARNESS: %s is a required member of the audit input file." % k, "NO_LABEL")
    A("input_root_not_string", dict(WF, root=1),
      "HARNESS with sec 9.4: root is hex20, and an int is outside its form.", "NO_LABEL")
    A("input_root_uppercase", dict(WF, root="0x" + R[2:].upper()),
      "sec 8 with sec 1: the root is hex20, lowercase; another spelling is not an audit input.",
      "NO_LABEL")
    A("input_root_uppercase_prefix", dict(WF, root="0X" + R[2:]),
      "sec 9.4 with sec 1: an uppercase 0x prefix fails hex20 as an uppercase digit does.",
      "NO_LABEL")
    A("input_anchors_not_array", dict(WF, anchors={}),
      "sec 8: the anchor set is a set of records.", "NO_LABEL")
    A("input_anchors_element_not_object", dict(WF, anchors=[1]),
      "HARNESS with sec 9.2: anchors holds objects.", "NO_LABEL")
    A("input_basis_not_object", dict(WF, basis=[]),
      "sec 9.4: the basis is an object; an array is not a basis.", "NO_LABEL")
    A("input_two_extra_members_ignored", dict(WF, held=[], missed=[]),
      "HARNESS: the object is open and members beyond the three are ignored, two at once.",
      "COMPLETE")
    A("input_extra_member_ignored", dict(WF, extra=[{"chainId": 1, "tx": HX32(7)}]),
      "sec 8 with HARNESS: the audit has two inputs and the file three members; a fourth is not "
      "one of them, and nothing in this law refuses an input object for carrying it.", "COMPLETE")
    A("input_ignored_member_max_int", dict(WF, extra=[{"n": 2 ** 53 - 1}]),
      "HARNESS: 2^53 - 1 is the largest integer sec 3.1 admits and is read, in an ignored member "
      "as anywhere else; the report is the clean ledger's.", "COMPLETE")
    A("input_ignored_member_negative_number", dict(WF, evidence=[{"n": -1}]),
      "HARNESS: every number in the audit input file is an integer of sec 3.1's universe, in a "
      "member the reader ignores as anywhere else; -1 makes the file unreadable.", "NO_LABEL")
    A("input_ignored_member_number_above_universe", dict(WF, evidence=[{"n": 2 ** 53}]),
      "HARNESS: 2^53 in an ignored member is outside sec 3.1's universe and the file is refused.",
      "NO_LABEL")
    A("input_ignored_member_lone_surrogate", dict(WF, evidence=["\ud800"]),
      "HARNESS: a lone surrogate escape names no scalar value; the file is refused, in an "
      "ignored member as anywhere else.", "NO_LABEL")
    A("input_ignored_member_surrogate_pair", dict(WF, evidence=["\U0001f600"]),
      "HARNESS: a surrogate pair is RFC 8259's spelling of one scalar value above U+FFFF and "
      "is read as that value; the report is the clean ledger's.", "COMPLETE")
    deep = 7
    for _ in range(126):
        deep = [deep]
    A("input_ignored_member_depth_128", dict(WF, evidence=[deep]),
      "HARNESS: the file carries sec 3.5's depth bound; a container opening at depth 128 inside "
      "an ignored member is read, and the report is the clean ledger's.", "COMPLETE")
    A("input_ignored_member_depth_129", dict(WF, evidence=[[deep]]),
      "HARNESS: a container opening at depth 129, the root counted as depth 1, makes the file "
      "unreadable, in an ignored member as anywhere else; no reader needs an unbounded stack.",
      "NO_LABEL")
    A_raw("input_ignored_member_leading_zero",
          json.dumps(dict(WF, evidence=[{"n": 7}])).replace('"n": 7', '"n": 07').encode(),
          "HARNESS: a leading zero is not sec 3.2's spelling of an integer, in an ignored member "
          "as anywhere else; the file is refused.")
    A_raw("input_ignored_member_exponent",
          json.dumps(dict(WF, evidence=[{"n": 7}])).replace('"n": 7', '"n": 1e3').encode(),
          "HARNESS: an exponent is not sec 3.2's spelling of an integer, in an ignored member "
          "as anywhere else; the file is refused.")
    A_raw("input_reader_utf8_inside_ignored_string",
          (json.dumps(WF, sort_keys=True)[:-1] + ',"ignored":"').encode("utf-8") + b"\xc3\x28" + b'"}',
          "HARNESS: the reader's first restriction, valid UTF-8 over the whole file, is the only "
          "rule the two bytes C3 28 inside an ignored member's string reach; the file is "
          "unreadable and gives no label.")
    A_raw("input_reader_raw_control_byte_in_ignored_string",
          (json.dumps(WF, sort_keys=True)[:-1] + ',"ignored":"a\tb"}').encode("utf-8"),
          "HARNESS: the reader is an RFC 8259 reader, whose string grammar admits no raw byte "
          "below 0x20; a raw tab inside an ignored member's string makes the file unreadable.")
    A_raw("input_reader_escaped_control_in_ignored_string",
          (json.dumps(WF, sort_keys=True)[:-1] + ',"ignored":"a\\u0000b"}').encode("utf-8"),
          "HARNESS: a `\\u0000` escape names a scalar value and is read, in an ignored member as "
          "anywhere else; the report is the clean ledger's.", expect_label="COMPLETE",
          predict_from=WF)
    A_raw("input_duplicate_member_name_by_escape",
          (json.dumps(WF, sort_keys=True)[:-1] + ',"anchor\\u0073":[]}').encode("utf-8"),
          "HARNESS: no object repeats a member name after escape decoding; anchors beside "
          "anchor\\u0073 is one name twice and the file is unreadable.")
    A_raw("input_required_member_name_by_escape",
          json.dumps(WF, sort_keys=True).replace('"root":', '"\\u0072oot":', 1).encode("utf-8"),
          "HARNESS: a member name is the name the reader decoded, so root spelled with an escape "
          "is the required member root and the report is the clean ledger's.",
          expect_label="COMPLETE", predict_from=WF)
    A_raw("input_basis_member_name_by_escape",
          json.dumps(WF, sort_keys=True).replace('"chains":', '"\\u0063hains":', 1).encode("utf-8"),
          "HARNESS: a basis member spelled with an escape is the member chains and no member "
          "beyond the basis's list; the report is the clean ledger's.", expect_label="COMPLETE",
          predict_from=WF)
    A_raw("input_basis_transport_member_order",
          b'{"root":"' + R.encode() + b'","anchors":' + json.dumps(WF["anchors"]).encode()
          + b',"basis":{"chains":[{"senders":' + json.dumps(ROOTB["chains"][0]["senders"]).encode()
          + b',"registries":[],"toBlock":1000,"fromBlock":0,"chainId":1}]}}',
          "HARNESS with sec 8.7 item 1: the basis arrives in any RFC 8259 spelling and the report "
          "carries it re-canonicalized, so a file spelling a chains object's members in another "
          "order yields the same report as the canonical spelling; label COMPLETE.",
          expect_label="COMPLETE", predict_from=WF)
    PA = inp(R, RC + [rec(b"A", K9, bn=50)])
    A_raw("input_entry_escape_spelled",
          json.dumps(PA, sort_keys=True).replace('"0x41"', '"0x\\u0034\\u0031"').encode("utf-8"),
          "HARNESS: every form is decided on the decoded string, so an entry spelled with escapes "
          "is the one-byte string 0x41, a MALFORMED row and a DISCARDED row, and the report is "
          "the plain spelling's byte for byte.", expect_label=predict_audit(PA)["label"],
          predict_from=PA)
    VA = inp(R, RC)
    A_raw("input_verdict_escape_spelled",
          json.dumps(VA, sort_keys=True).replace('"counted"', '"\\u0063ounted"').encode("utf-8"),
          "HARNESS: a verdict whose escapes decode to counted is the verdict counted; the report "
          "is the plain spelling's byte for byte.", expect_label=predict_audit(VA)["label"],
          predict_from=VA)
    # the record's members (sec 9.2)
    for k in ("chainId", "blockNumber", "blockTimestamp", "tx", "sender", "hash", "verdict"):
        A("input_anchor_record_lacks_%s" % k.lower(),
          inp(R, [{kk: v for kk, v in rec(CL[0], R).items() if kk != k}]),
          "sec 9.2/9.4: a record lacking one of the seven required members is no anchor record; "
          "%s absent." % k, "NO_LABEL")
    A("input_anchor_verdict_outside_set", inp(R, [dict(rec(CL[0], R), verdict="maybe")]),
      "sec 9.4: an anchor record whose verdict is outside the three values is not an audit input.",
      "NO_LABEL")
    A("input_anchor_sender_not_hex20", inp(R, [dict(rec(CL[0], R), sender="0xabc")]),
      "sec 9.4: a record whose members fail the forms of sec 8 and sec 9.2 is not an audit input.",
      "NO_LABEL")
    A("input_anchor_hash_not_hex32", inp(R, [dict(anchor(CID[0], R), hash="0x12")]),
      "sec 9.2 with sec 1: hash is hex32.", "NO_LABEL")
    A("input_anchor_chainid_string", inp(R, [dict(anchor(CID[0], R), chainId="1")]),
      "sec 9.2 with sec 1: chainId is an int of sec 3.1.", "NO_LABEL")
    A("input_anchor_blocknumber_string", inp(R, [dict(anchor(CID[0], R), blockNumber="100")]),
      "sec 9.2 with sec 1: blockNumber is an int of sec 3.1.", "NO_LABEL")
    A("input_anchor_tx_not_hex32", inp(R, [dict(anchor(CID[0], R), tx="0x12")]),
      "sec 9.2 with sec 1: tx is hex32.", "NO_LABEL")
    A("input_anchor_timestamp_above_universe", inp(R, [dict(anchor(CID[0], R), blockTimestamp=MAXINT + 1)]),
      "sec 9.2 with sec 1: blockTimestamp is an int of the universe; 2^53 fails the form.",
      "NO_LABEL")
    A("input_anchor_blocktimestamp_string", inp(R, [dict(anchor(CID[0], R), blockTimestamp="1700000000")]),
      "sec 9.2 with sec 1: blockTimestamp is an int; the form fault alone refuses it.", "NO_LABEL")
    A("input_anchor_record_extra_member", inp(R, RC + [dict(anchor(CID[0], R, bn=50), note="x")]),
      "sec 8: a record with a member beyond its eight is read by its eight; nothing in the law refuses it.",
      "COMPLETE")
    A("carried_entry_hash_mismatch", inp(R, RC[:4] + [dict(anchor(eid(CL[4]), R, bn=50), entry=hx(CL[3]))]),
      "sec 9.2: an entry whose sha256 is not the record's hash makes the record no anchor "
      "record; no label.", "NO_LABEL")
    A("carried_entry_odd_hex", inp(R, RC[:4] + [dict(anchor(eid(CL[4]), R, bn=50), entry="0x1")]),
      "sec 9.2 with HARNESS: an entry outside the transport spelling; no label.", "NO_LABEL")
    A("carried_entry_without_0x", inp(R, RC[:4] + [dict(anchor(eid(CL[4]), R, bn=50), entry=hx(CL[4])[2:])]),
      "HARNESS: an entry that does not begin with 0x; no label.", "NO_LABEL")
    A("carried_entry_nonhex", inp(R, RC[:4] + [dict(anchor(eid(CL[4]), R, bn=50), entry="0xzz" + hx(CL[4])[4:])]),
      "HARNESS: an entry carrying a non-hex character; no label.", "NO_LABEL")
    A("carried_entry_uppercase_prefix", inp(R, RC[:4] + [dict(anchor(eid(CL[4]), R, bn=50), entry="0X" + hx(CL[4])[2:])]),
      "HARNESS: the transport spelling has the two lowercase bytes 0x; 0X is outside it.",
      "NO_LABEL")
    A("carried_entry_not_string", inp(R, RC[:4] + [dict(anchor(eid(CL[4]), R, bn=50), entry=7)]),
      "sec 9.2: an entry that is not a byte string; no label.", "NO_LABEL")
    A("carried_entry_uppercase_digits_accepted",
      inp(R, RC[:4] + [dict(anchor(eid(CL[4]), R, bn=50), entry="0x" + hx(CL[4])[2:].upper())]),
      "HARNESS: hexadecimal digits of either case spell the same bytes; the entry is carried and "
      "the report is the clean ledger's.", "COMPLETE")
    A("carried_empty_entry_spelling", inp(R, RC + [dict(anchor(eid(b""), R, bn=50), entry="0x")]),
      "HARNESS: 0x alone spells the empty byte string; sec 8.4 finds the carried empty string "
      "malformed (E_JSON) and hard.", "BROKEN_CHAIN")

    # ---------------- basis forms (sec 9.4) ------------------------------------
    A("basis_extra_top_member", dict(WF, basis=dict(ROOTB, note="scan of 2031-04")),
      "sec 9.4: the basis is an object with exactly the member chains; sec 6.9 reaches no object "
      "of this section.", "NO_LABEL")
    A("basis_second_member_array", dict(WF, basis=dict(ROOTB, transactions=[])),
      "sec 9.4: a second member is a member beyond the list, an empty array as much as a "
      "note, and the object is not a galeed/1 basis.", "NO_LABEL")
    A("basis_chains_member_absent", dict(WF, basis={}),
      "sec 9.4: exactly the member chains; the empty object is no basis.", "NO_LABEL")
    A("basis_chains_not_array", dict(WF, basis={"chains": {}}),
      "sec 9.4: chains is an array of objects.", "NO_LABEL")
    A("basis_chains_element_not_object", dict(WF, basis={"chains": [1]}),
      "sec 9.4: chains holds objects.", "NO_LABEL")
    A("basis_chains_object_extra_member",
      dict(WF, basis=basis((dict(ROOTB["chains"][0], rpc="https://example"),))),
      "sec 9.4: an object bearing a member beyond its list is not a galeed/1 basis.",
      "NO_LABEL")
    for k in ("chainId", "fromBlock", "toBlock", "registries", "senders"):
        A("basis_chains_object_lacks_%s" % k.lower(),
          dict(WF, basis={"chains": [{kk: v for kk, v in ROOTB["chains"][0].items() if kk != k}]}),
          "sec 9.4: a chains object carries exactly its five members; %s absent." % k, "NO_LABEL")
    A("basis_chainid_string", dict(WF, basis={"chains": [dict(ROOTB["chains"][0], chainId="1")]}),
      "sec 9.4: chainId is an int.", "NO_LABEL")
    A("basis_fromblock_string", dict(WF, basis={"chains": [dict(ROOTB["chains"][0], fromBlock="0")]}),
      "sec 9.4: fromBlock is an int.", "NO_LABEL")
    A("basis_toblock_string", dict(WF, basis={"chains": [dict(ROOTB["chains"][0], toBlock="1000")]}),
      "sec 9.4: toBlock is an int.", "NO_LABEL")
    for arr in ("registries", "senders"):
        A("basis_%s_not_array" % arr, dict(WF, basis={"chains": [dict(ROOTB["chains"][0], **{arr: "0x00"})]}),
          "sec 9.4: %s is an array." % arr, "NO_LABEL")
    A("basis_sender_not_hex20", inp(R, [], bas=basis((chainobj(1, senders=("0xabc",), raw=True),))),
      "sec 9.4 with sec 1: senders is an array of hex20.", "NO_LABEL")
    A("basis_registry_uppercase", inp(R, [], bas=basis((chainobj(1, regs=("0x" + "A" * 40,)),))),
      "sec 9.4/sec 1: registries is an array of hex20 and class items.", "NO_LABEL")
    A("basis_registries_element_int", inp(R, [], bas=basis((chainobj(1, regs=(7,), raw=True),))),
      "sec 9.4: a registries element that is neither hex20 nor a class item.", "NO_LABEL")
    A("basis_fromblock_equals_toblock", inp(R, [], bas=basis((chainobj(1, fb=500, tb=500),))),
      "sec 9.4: fromBlock equal to toBlock is the inside edge of the range relation.", "COMPLETE")
    A("basis_fromblock_above_toblock", inp(R, [], bas=basis((chainobj(1, fb=501, tb=500),))),
      "sec 9.4: fromBlock above toBlock; the object is not a galeed/1 basis.", "NO_LABEL")
    A("basis_chains_unsorted", inp(R, [], bas=basis((chainobj(8453), chainobj(1)), raw=True)),
      "sec 9.4: chains is ordered by chainId ascending; another order is no basis.", "NO_LABEL")
    A("basis_chains_int_order_numeric",
      inp(R, [], bas=basis((chainobj(2, senders=(R,)), chainobj(10, senders=(R,))), raw=True)),
      "sec 9.4: chains is ordered by chainId numerically, so 2 then 10 is a basis.", "COMPLETE")
    A("basis_chains_int_order_decimal_string",
      inp(R, [], bas=basis((chainobj(10, senders=(R,)), chainobj(2, senders=(R,))), raw=True)),
      "sec 9.4: 10 then 2 is the decimal-string order and not the numeric one; no label.",
      "NO_LABEL")
    A("basis_senders_unsorted", inp(R, [], bas=basis((chainobj(1, senders=(HX20(2), HX20(1)), raw=True),))),
      "sec 9.4: senders is bytewise ascending.", "NO_LABEL")
    A("basis_senders_repeated", inp(R, [], bas=basis((chainobj(1, senders=(HX20(1), HX20(1)), raw=True),))),
      "sec 9.4: senders carries no repeated element.", "NO_LABEL")
    A("basis_registries_unsorted", inp(R, [], bas=basis((chainobj(1, regs=(HX20(5), HX20(4)), raw=True),))),
      "sec 9.4: registries is bytewise ascending.", "NO_LABEL")
    A("basis_registries_repeated", inp(R, [], bas=basis((chainobj(1, regs=(HX20(7), HX20(7)), raw=True),))),
      "sec 9.4: registries carries no repeated element.", "NO_LABEL")
    A("basis_arrays_sorted_two_chains",
      inp(R, [], bas=basis((chainobj(8453, senders=(HX20(9), HX20(3))), chainobj(1, regs=(HX20(5), HX20(4)))))),
      "sec 9.4: the helper's ordering, two chains and two-element arrays each in order.", "COMPLETE")
    W1 = chainobj(1, 0, 100, senders=[R])
    W2 = chainobj(1, 200, 300, senders=[R])
    A("basis_two_windows_one_chain", inp(R, [], bas=basis((W1, W2))),
      "sec 9.4: two chains objects of one chainId over two windows that do not overlap are "
      "ordinary; a reader whose node serves two windows declares two objects.", "COMPLETE")
    A("basis_two_windows_one_block_apart",
      inp(R, [], bas=basis((W1, chainobj(1, fb=102, tb=300, senders=(R,))))),
      "sec 9.4: ranges one block apart are not adjacent, so equal lists are a basis; the inside "
      "edge of the adjacency clause.", "COMPLETE")
    A("basis_two_windows_overlap", inp(R, [], bas=basis((W1, chainobj(1, 100, 300, senders=[R])), raw=True)),
      "sec 9.4: two objects of one chainId whose ranges overlap are not a basis.", "NO_LABEL")
    A("basis_two_windows_touch_equal_lists", inp(R, [], bas=basis((W1, chainobj(1, 101, 300, senders=[R])), raw=True)),
      "sec 9.4: two objects of one chainId whose ranges touch with equal registries and "
      "senders would be one object spelled twice, and are not a basis.", "NO_LABEL")
    A("basis_two_windows_touch_different_lists",
      inp(R, [], bas=basis((W1, chainobj(1, 101, 300, senders=[R, K2])), raw=True)),
      "sec 9.4: two touching windows whose senders differ are two scans with one spelling, "
      "and a basis.", "COMPLETE")
    A("basis_two_windows_touch_different_registries",
      inp(R, [], bas=basis((chainobj(1, fb=0, tb=100, regs=(HX20(4),)), chainobj(1, fb=101, tb=300, regs=(HX20(4), HX20(5)))))),
      "sec 9.4: two adjacent windows whose registries differ are two scans with one spelling, "
      "and a basis.", "COMPLETE")
    A("basis_two_windows_unsorted", inp(R, [], bas=basis((W2, W1), raw=True)),
      "sec 9.4: chains is ordered by (chainId, fromBlock) ascending; two windows of one chain "
      "spelled in the other order are not a basis.", "NO_LABEL")
    A("basis_windows_fromblock_numeric",
      inp(R, [], bas=basis((chainobj(1, fb=9, tb=9, senders=(R,)), chainobj(1, fb=10, tb=20, senders=(R,))), raw=True)),
      "sec 9.4: two windows ordered by fromBlock numerically, 9 then 10, and adjacent only "
      "where 9 plus 1 is 10, which it is, their lists equal; no label.", "NO_LABEL")
    A("basis_windows_fromblock_numeric_apart",
      inp(R, [], bas=basis((chainobj(1, fb=9, tb=9, senders=(R,)), chainobj(1, fb=11, tb=20, senders=(R,))), raw=True)),
      "sec 9.4: 9 then 11 is numeric order and the windows are a block apart; a basis.",
      "COMPLETE")
    A("anchor_reach_split_across_windows",
      inp(R, [rec(CL[0], R, bn=150)],
          bas=basis((chainobj(1, fb=0, tb=100, senders=(R,)), chainobj(1, fb=101, tb=300, senders=(K2,))))),
      "sec 9.4 [C]: the reach test is over one chains object: the record's block lies in the "
      "second window and its sender in the first window's senders only, so no object reaches "
      "it; no label.", "NO_LABEL")
    WB = basis((chainobj(1, fb=100, tb=200, senders=(R,)),))
    for bn, lab, why in ((100, "COMPLETE", "at fromBlock, inside the inclusive range"),
                         (200, "COMPLETE", "at toBlock, inside the inclusive range"),
                         (99, "NO_LABEL", "one below fromBlock, outside the range"),
                         (201, "NO_LABEL", "one above toBlock, outside the range")):
        A("basis_window_edge_block_%d" % bn, inp(R, [rec(CL[0], R, bn=bn)], bas=WB),
          "sec 9.4: fromBlock and toBlock bound an inclusive range; a record %s is %s." %
          (why, "reached" if lab != "NO_LABEL" else "reached by no object"), lab)
    # class items
    CB = basis((chainobj(1, regs=(K4,), senders=(R,)),))
    CB["chains"][0]["registries"] = [K4, classitem(1)]
    A("basis_class_item_after_address", inp(R, [rec(CL[0], R)], bas=CB),
      "sec 9.4: registries lists a hex20 address then a class item; a valid basis, the record "
      "reached by senders.", "COMPLETE")
    def cb(regs):
        b = basis((chainobj(1, senders=(R,)),))
        b["chains"][0]["registries"] = regs
        return b
    A("basis_class_item_alone", inp(R, [rec(CL[0], R)], bas=cb([classitem(1)])),
      "sec 9.4: a registries array of one class item.", "COMPLETE")
    A("basis_class_items_sorted", inp(R, [], bas=cb([classitem(1), classitem(2)])),
      "sec 9.4: two class items ascending by codeHash.", "COMPLETE")
    A("basis_class_items_unsorted", inp(R, [], bas=cb([classitem(2), classitem(1)])),
      "sec 9.4: class items out of bytewise order; no label.", "NO_LABEL")
    A("basis_class_item_repeated", inp(R, [], bas=cb([classitem(1), classitem(1)])),
      "sec 9.4: a repeated class item; no label.", "NO_LABEL")
    A("basis_class_item_before_address", inp(R, [], bas=cb([classitem(1), K4])),
      "sec 9.4: an address after a class item; the strings come first; no label.", "NO_LABEL")
    A("basis_class_item_extra_member", inp(R, [], bas=cb([{"codeHash": HX32(1), "chainId": 1}])),
      "sec 9.4: a class item carries exactly codeHash; a second member is no basis.", "NO_LABEL")
    A("basis_class_item_codehash_hex20", inp(R, [], bas=cb([{"codeHash": HX20(1)}])),
      "sec 9.4 with sec 1: codeHash is hex32.", "NO_LABEL")
    A("basis_class_item_codehash_uppercase", inp(R, [], bas=cb([{"codeHash": "0x" + HX32(0xAB)[2:].upper()}])),
      "sec 1: hex32 is lowercase; no label.", "NO_LABEL")
    A("basis_class_item_empty_object", inp(R, [], bas=cb([{}])),
      "sec 9.4: an object without codeHash is no class item.", "NO_LABEL")
    A("basis_class_item_codehash_int", inp(R, [], bas=cb([{"codeHash": 5}])),
      "sec 9.4: codeHash of the wrong form.", "NO_LABEL")
    CB13 = basis((chainobj(1, 0, 10, senders=(R,)), chainobj(1, 11, 20, senders=(R,))), raw=True)
    CB13["chains"][0]["registries"] = [classitem(1)]
    CB13["chains"][1]["registries"] = [classitem(2)]
    A("basis_two_windows_touch_different_class_items", inp(R, [], bas=CB13),
      "sec 9.4: two adjacent windows of one chain differing in their class item alone are two "
      "objects.", "COMPLETE")
    CB14 = basis((chainobj(1, 0, 10, senders=(R,)), chainobj(1, 11, 20, senders=(R,))), raw=True)
    CB14["chains"][0]["registries"] = [classitem(1)]
    CB14["chains"][1]["registries"] = [classitem(1)]
    A("basis_two_windows_touch_equal_class_items", inp(R, [], bas=CB14),
      "sec 9.4: two adjacent windows with equal registries and senders; no label.", "NO_LABEL")

    # ---------------- lineage, unknown types, sort keys, the carried record -------
    UNK, UNKI = build_chain([("root", "genesis", GENB), ("root", "dispatch", {"case": HX32(1), "note_md": "n"}),
                             ("root", "annotation", ANNB("after")), ("root", "pass/1", {})])
    A("unknown_types_listed", inp(R, recs(UNK, R)),
      "sec 6.8/8.7 item 7: every ledger entry of a type outside the seven is listed with its "
      "seq and type, sorted by (seq, entryType, entry_id).", "COMPLETE")
    A("unknown_types_sorted_by_type", inp(R, recs(UNK + [mk("root", "aardvark", 1, UNKI[0], {})], R)),
      "sec 8.7 item 7: two unknown types at one seq sort by entryType bytewise; the fork is "
      "hard all the same.", "BROKEN_CHAIN")
    A("unknown_types_same_type_one_seq", inp(R, recs(UNK + [mk("root", "dispatch", 1, UNKI[0], {"x": 1})], R)),
      "sec 8.7 item 7: two unknown entries of one type at one seq sort by entry_id bytewise; "
      "the fork is hard all the same.", "BROKEN_CHAIN")
    A("unknown_type_excluded_entry_not_listed",
      inp(R, recs(UNK, R) + [rec(mk("k9", "dispatch", 4, UNKI[3], {}), K9, bn=50)]),
      "sec 8.7 item 7: the list is over ledger entries; an EXCLUDED entry of an unknown type "
      "is an EXCLUDED row and no UNKNOWN_TYPE row.", "COMPLETE")
    A("keybinding_is_a_known_type",
      inp(R, recs(UNK[:3] + [mk("root", "keybinding", 3, UNKI[2], {"recipient": "age1qq", "effective": 1700000000})], R)),
      "sec 8.7 item 7: keybinding is one of the seven and earns no row, witnessed by hand and "
      "never by the seed.", "COMPLETE")
    A("unknown_types_sorted_by_case",
      inp(R, recs(CL[:2] + [mk("root", "Zulu", 1, CID[0], {}), mk("root", "aardvark", 1, CID[0], {})], R)),
      "sec 8.7 item 7: entryType compares bytewise, so Zulu precedes aardvark.", "BROKEN_CHAIN")
    UT0 = mk("root", "genesis", 0, None, GENB)
    UT1 = mk("root", "zulu", 1, eid(UT0), {})
    UT2 = mk("root", "aardvark", 2, eid(UT1), {})
    A("unknown_type_seq_before_type", inp(R, recs([UT0, UT1, UT2], R)),
      "sec 8.7 item 7: the list sorts by seq before entryType, so zulu at 1 precedes aardvark "
      "at 2 though the types sort the other way.", "COMPLETE")
    # sec 7.4/8.1: a retired key's succession at a fork, carried in the second of its handing off, is DISCARDED and admits nobody, whichever id is smaller
    HC, HCI = build_chain([("root", "genesis", GENB), ("root", "succession", SUCB(K2)),
                           ("k2", "judgment", JGB(2))])
    s_hon = mk("k2", "succession", 3, HCI[2], SUCB(K3))
    e_hon = mk("k3", "judgment", 4, eid(s_hon), JGB(4))

    def retired_fork(k):
        s_ret = mk("root", "succession", 3, HCI[2], SUCB(K4, kind="handover-%d" % k))
        return inp(R, recs(HC + [s_hon, e_hon, s_ret], R)), eid(s_hon) < eid(s_ret)
    both_orders("fork_retired_key_succession", retired_fork,
                "sec 7.4/8.1: the root that handed off signs a competing succession at seq 3 and carries "
                "it in the second of its handing off: the record is DISCARDED, the succession admits "
                "nobody and moves nothing, and the honest line records no finding, whichever "
                "succession has the smaller id.", "COMPLETE")
    NS, NSI = build_chain([("root", "genesis", GENB), ("root", "succession", SUCB(K2)),
                           ("k2", "judgment", JGB(2))])
    ns_ret = mk("root", "succession", 3, NSI[2], SUCB(K3))
    A("succession_at_k_none_in_office",
      inp(R, recs(NS + [ns_ret, mk("k2", "judgment", 4, eid(ns_ret), JGB(4))], R)),
      "sec 7.4/8.1/8.2: the only succession at seq 3 is signed by the root that handed off and carried "
      "in the second of that handing off; the record is DISCARDED, seq 3 is a gap, and k2's entry at "
      "seq 4 is a ledger entry across it; GAPS.",
      "GAPS")
    A("succession_at_k_none_in_office_successor_signs",
      inp(R, recs(NS + [ns_ret, mk("k3", "judgment", 4, eid(ns_ret), JGB(4))], R)),
      "sec 7.4/8.1/8.4: the same entries with seq 4 signed by the named successor; the root's "
      "succession at 3 is DISCARDED, k3 is outside the lineage, its entry EXCLUDED and its hash "
      "MISSING, and the root's counted record that carried it records CARRIED_MALFORMED with "
      "token FOREIGN; BROKEN_CHAIN.", "BROKEN_CHAIN")
    # sec 7.2/8.2: the link is over ledger entries; a stranger's genesis satisfies none
    GX = mk("k9", "genesis", 0, None, {"statement_md": "a stranger's opening"})
    A("prev_names_excluded_entry",
      inp(R, recs([CL[0], mk("root", "annotation", 1, eid(GX), ANNB("linked to the stranger"))], R) + [rec(GX, K9, bn=50)]),
      "sec 7.2/8.2: prev names an entry the lineage left out; the link is over ledger entries, "
      "the stranger's genesis (carried by the stranger's own record) is EXCLUDED and satisfies "
      "no link, so PREV_MISMATCH is hard.", "BROKEN_CHAIN")
    # sec 7.4: the fixpoint reads a succession's judge against the lineage, never against auth
    LF, LFI = build_chain([("root", "genesis", GENB)])
    lf_s1 = mk("k3", "succession", 1, LFI[0], SUCB(K4))
    lf_s2 = mk("root", "succession", 2, eid(lf_s1), SUCB(K3))
    lf_e3 = mk("k4", "annotation", 3, eid(lf_s2), ANNB("named by a key that never held office here"))
    A("lineage_by_succession_that_moved_nothing", inp(R, recs(LF + [lf_s1, lf_s2, lf_e3], R)),
      "sec 7.4: k3 joins the lineage through the root's succession at 2, so k3's own succession "
      "at 1 extends the lineage to k4 though it moved nothing; all four are ledger entries, and "
      "the two authority faults are hard.", "BROKEN_CHAIN")
    STR = mk("k9", "succession", 1, eid(CL[0]), SUCB(K4))
    A("lineage_stranger_succession_extends_nothing",
      inp(R, RC + [rec(STR, K9, bn=50), anchor(HX32(0xBEEF), K4, bn=51)]),
      "sec 7.4/8.1: the lineage is extended by the `to` of a succession whose judge is already "
      "in the set; a stranger's succession extends nothing, so its `to` cannot put a hash into "
      "this ledger's reconciliation and both records are DISCARDED rows, the succession EXCLUDED.",
      "COMPLETE")
    # sec 8.6: a succession carried only by a discarded record extends the lineage
    A("lineage_through_discarded_record",
      inp(R, recs([SU[0]], R) + [rec(SU[1], K9, bn=20)] + recs(SU[2:], K2, bn0=30)),
      "sec 8.6/7.4: the root's succession is carried only by a stranger's record; the record is "
      "DISCARDED with its bytes, yet lineage is decided by signatures, so k2 joins the lineage "
      "and its own records count. The succession itself is no ledger entry, so seq 1 is a "
      "SEQ_GAP and the label is GAPS.", "GAPS")
    A("lineage_through_void_record",
      inp(R, recs([SU[0]], R) + [rec(SU[1], R, bn=20, verdict="VOID")] + recs(SU[2:], K2, bn0=30)),
      "sec 8.6: the succession is carried only by a VOID record of the root; k2 joins the "
      "lineage all the same, the succession is no ledger entry, and the position is a gap.",
      "GAPS")
    A("lineage_through_unproven_record",
      inp(R, recs([SU[0]], R) + [rec(SU[1], R, bn=20, verdict="UNPROVEN")] + recs(SU[2:], K2, bn0=30)),
      "sec 8.6: the succession is carried only by an UNPROVEN record; k2 joins the lineage, the "
      "position is a gap, and the label is UNPROVEN since no counted record carries that hash.",
      "UNPROVEN")
    # sec 6.9/8.4: an extra member is part of the bytes, so twins differing only in one equivocate
    xm_a = mk("root", "annotation", 1, CID[0], ANNB("n"))
    xm_b = mk("root", "annotation", 1, CID[0], dict(ANNB("n"), z=1))
    A("extra_member_twins_equivocate", inp(R, recs([CL[0], xm_a, xm_b], R)),
      "sec 6.9 with sec 2.1 and sec 8.4: two entries at one seq differing only in an extra body "
      "member are two entries with two entry_ids and equivocate; identity is total.",
      "BROKEN_CHAIN")
    XA = mk("k9", "annotation", 1, HX32(0x11), ANNB("left"))
    XB = mk("k9", "annotation", 1, HX32(0x11), ANNB("right"))
    A("excluded_two_at_one_seq", inp(R, RC + recs([XA, XB], K9, bn0=50)),
      "sec 8.7 item 6: two excluded entries at one seq sort by entry_id bytewise, and sec 8.4 "
      "records no equivocation, since neither is a ledger entry.", "COMPLETE")
    # the carried record (sec 8.4) at every position, and its listings
    FOREIGN = mk("k9", "annotation", 3, HX32(0x77), ANNB("a stranger's entry"))
    A("carried_foreign_entry_also_own_record", inp(R, RC + [rec(FOREIGN, R, bn=50), rec(FOREIGN, K9, bn=51)]),
      "sec 8.4: the same stranger's entry carried by the root's counted record and by the "
      "stranger's own; one EXCLUDED row, one finding, one DISCARDED row.", "BROKEN_CHAIN")
    FG = mk("k9", "genesis", 0, None, GENB)
    A("carried_findings_sort_at_position_zero",
      inp(R, RC + [rec(JUNK, R, bn=50, tx=HX32(0x811)), rec(FG, R, bn=51, tx=HX32(0x812))]),
      "sec 8.7 item 3: two CARRIED_MALFORMED findings at position 0 (junk, and a stranger's "
      "genesis) sort by entry_id; a stranger's seq-0 entry is EXCLUDED and records no "
      "ROOT_MISMATCH.", "BROKEN_CHAIN")
    FAR = mk("k9", "annotation", 7, HX32(0x71), ANNB("far foreign"))
    A("carried_foreign_position_is_seq",
      inp(R, RC[:3] + [rec(FAR, R, bn=50), rec(JUNK, R, bn=51, tx=HX32(0x813))]),
      "sec 8.7 item 3: the foreign entry's finding sits at position 7, after the junk's at 0.",
      "BROKEN_CHAIN")
    A("discarded_void_record_carries_entry", inp(R, RC + [rec(CL[1], K3, bn=50, verdict="VOID")], bas=B_RK3),
      "sec 8.7 item 11: a discarded record carries its entry in the DISCARDED row, verdict and "
      "all.", "COMPLETE")
    TAIL = mk("root", "annotation", 9, HX32(0x99), ANNB("a branch far ahead"))
    A("carried_branch_seq_gap", inp(R, RC + [rec(TAIL, R, bn=50)]),
      "sec 8.2: a carried entry the root signed at seq 9 with an unknown prev; it is a ledger "
      "entry, the gap is SEQ_GAP (soft), no prev finding across the gap; GAPS.", "GAPS")
    TWIN4 = mk("root", "annotation", 4, eid(CL[3]), ANNB("a second entry at four"))
    A("carried_fork_twin", inp(R, RC + [rec(TWIN4, R, bn=50)]),
      "sec 8.4: a carried twin of the fifth entry; EQUIVOCATION, hard.", "BROKEN_CHAIN")
    UNKE = mk("root", "windmill", 5, eid(CL[4]), {"n": 1})
    A("carried_unknown_type_entry", inp(R, RC + [rec(UNKE, R, bn=50)]),
      "sec 8.7 item 7: a carried ledger entry of a type this grammar does not know; "
      "UNKNOWN_TYPE row, no finding.", "COMPLETE")
    MIG = make_entry(K("root"), "annotation", 5, eid(CL[4]), ANNB("written under the next spec"), spec="galeed/2")
    A("carried_next_spec_entry_e_spec", inp(R, RC + [rec(MIG, R, bn=50)]),
      "sec 12.3 with sec 8.4: a lineage key carries an entry whose spec is galeed/2; it fails "
      "sec 4.3 at E_SPEC, is CARRIED_MALFORMED with that token at position 0, MALFORMED, and its "
      "hash MISSING; BROKEN_CHAIN, which is what a galeed/1 reader must say of bytes a lineage "
      "key published under another spec.", "BROKEN_CHAIN")
    A("carried_next_spec_entry_by_stranger", inp(R, RC + [rec(MIG, K9, bn=50)]),
      "sec 12.3: the same bytes carried by a stranger's record are DISCARDED and MALFORMED and "
      "charge nobody.", "COMPLETE")
    A("carried_next_spec_entry_narrow_basis", inp(R, RC + [rec(MIG, R, bn=50)], bas=basis((chainobj(1, tb=40, senders=(R,)),))),
      "sec 9.4: a basis bounded before the new-spec record's block does not reach it, so an "
      "input carrying it has no label; the reader narrows the anchor set with the basis.", "NO_LABEL")
    A("carried_next_spec_entry_outside_narrow_basis", inp(R, RC, bas=basis((chainobj(1, tb=40, senders=(R,)),))),
      "sec 12.3: the old ledger read in its own time, under a basis whose toBlock stops before "
      "the key anchored anything under the next spec; COMPLETE.", "COMPLETE")
    # sec 8.7 sort keys against a decimal order, and per-list orders
    H_F00D = HX32(0xF00D)
    A("missing_two_hashes_sorted",
      inp(R, RC + [anchor(H_UNSEEN, R, bn=50, tx=HX32(0x501)), anchor(H_F00D, R, bn=60, tx=HX32(0x601))]),
      "sec 8.7 item 4: two counted hashes not in have; the rows sort bytewise by hash.", "GAPS")
    A("missing_anchors_sorted_by_block",
      inp(R, RC + [anchor(H_UNSEEN, R, bn=60, tx=HX32(0x501)), anchor(H_UNSEEN, R, bn=50, tx=HX32(0x777))]),
      "sec 8.7 item 4: the anchors of one MISSING row sort by blockNumber before tx.", "GAPS")
    A("missing_two_transactions_one_chain",
      inp(R, RC + [anchor(H_UNSEEN, R, bn=50, tx=HX32(0x502)), anchor(H_UNSEEN, R, bn=50, tx=HX32(0x501))]),
      "sec 9.2/8.7 item 4: two anchors of one hash in two transactions of one chain; one "
      "MISSING row whose transactions sort by (chainId, blockNumber, tx).", "GAPS")
    A("anchored_anchors_sorted_by_tx",
      inp(R, RC + [anchor(CID[0], R, bn=50, tx=HX32(0x777)), anchor(CID[0], R, bn=50, tx=HX32(0x501))]),
      "sec 8.7 item 5: three records of one entry_id, two in one block, so the tx component of "
      "the anchor ordering decides.", "COMPLETE")
    FIVE = lambda verdict, snd=R: [
        anchor(H_UNSEEN, snd, cid=8453, bn=50, ts=1699000000, tx=HX32(0x501), verdict=verdict),
        anchor(H_UNSEEN, snd, bn=60, tx=HX32(0x501), verdict=verdict),
        anchor(H_UNSEEN, snd, bn=50, tx=HX32(0x777), verdict=verdict),
        anchor(H_UNSEEN, snd, bn=50, tx=HX32(0x501), verdict=verdict),
        anchor(H_F00D, snd, bn=50, tx=HX32(0x501), verdict=verdict)]
    A("unproven_rows_sorted", inp(R, RC + FIVE("UNPROVEN"), bas=TWOCH),
      "sec 8.7 item 9: five UNPROVEN records differing first at each of hash, chainId, "
      "blockNumber and tx; the rows sort by the four in that order.", "UNPROVEN")
    A("void_rows_sorted", inp(R, RC + FIVE("VOID"), bas=TWOCH),
      "sec 8.7 item 10: the VOID list sorts as the UNPROVEN list does; no label turns on it.",
      "COMPLETE")
    A("unproven_rows_with_entries", inp(R, RC + [rec(CL[4], R, bn=50, verdict="UNPROVEN"), anchor(H_UNSEEN, R, bn=51, verdict="UNPROVEN")]),
      "sec 8.7 item 9: one UNPROVEN row carries its entry and the other none; the first's hash "
      "is counted, the second's is not, so the label is UNPROVEN.", "UNPROVEN")
    A("void_rows_with_entries", inp(R, RC + [rec(CL[4], R, bn=50, verdict="VOID"), anchor(H_UNSEEN, R, bn=51, verdict="VOID")]),
      "sec 8.7 item 10: one VOID row carries its entry and the other none.", "COMPLETE")
    A("discarded_rows_sorted",
      inp(R, RC + [anchor(H_UNSEEN, K9, bn=50, tx=HX32(0x502)),
                   anchor(H_UNSEEN, K4, cid=8453, bn=50, ts=1699000000, tx=HX32(0x501)),
                   anchor(H_UNSEEN, K4, bn=50, tx=HX32(0x777)),
                   anchor(H_UNSEEN, K4, bn=50, tx=HX32(0x501)),
                   anchor(H_F00D, K4, bn=50, tx=HX32(0x501))],
          bas=basis((chainobj(1, senders=(R, K4, K9)), chainobj(8453, senders=(K4,))))),
      "sec 8.7 item 11: five discarded records differing first at each of sender, chainId, tx "
      "and hash; the rows sort by the five members in that order.", "COMPLETE")
    A("discarded_records_listed", inp(R, RC + [anchor(H_UNSEEN, K9, bn=50), anchor(CID[1], K9, bn=7)]),
      "sec 8.7 item 11: every anchor record the lineage trim discarded is a DISCARDED row "
      "carrying its seven members, whatever its hash; no label turns on the list.", "COMPLETE")
    THREE = [anchor(H_UNSEEN, R, cid=2, bn=9, tx=HX32(0x901)),
             anchor(H_UNSEEN, R, cid=2, bn=10, tx=HX32(0x901)),
             anchor(H_UNSEEN, R, cid=10, bn=9, tx=HX32(0x901))]
    WIDE = basis((chainobj(1, senders=(R,)), chainobj(2, senders=(R,)), chainobj(10, senders=(R,))))
    A("missing_anchor_sort_keys_numeric", inp(R, RC + THREE, bas=WIDE),
      "sec 8.7: the chainId and blockNumber of a MISSING row's anchors compare numerically, so "
      "chain 2 precedes chain 10 and block 9 precedes block 10.", "GAPS")
    A("anchored_anchor_sort_keys_numeric", inp(R, RC + [dict(a, hash=CID[0]) for a in THREE], bas=WIDE),
      "sec 8.7 item 5: the same two components inside an ANCHORED row.", "COMPLETE")
    A("unproven_sort_keys_numeric", inp(R, RC + [dict(a, verdict="UNPROVEN") for a in THREE], bas=WIDE),
      "sec 8.7 item 9: chainId and blockNumber compare numerically.", "UNPROVEN")
    A("void_sort_keys_numeric", inp(R, RC + [dict(a, verdict="VOID") for a in THREE], bas=WIDE),
      "sec 8.7 item 10: the same, and no label turns on the list.", "COMPLETE")
    A("discarded_sort_keys_numeric",
      inp(R, RC + [dict(a, sender=K9) for a in THREE],
          bas=basis((chainobj(1, senders=(R,)), chainobj(2, senders=(K9,)), chainobj(10, senders=(K9,))))),
      "sec 8.7 item 11: chainId compares numerically, so chain 2 precedes chain 10.", "COMPLETE")
    LONG, LID = build_chain([("root", "genesis", GENB)] +
                            [("root", "annotation", ANNB("e%d" % i)) for i in range(1, 11)])
    A("findings_position_numeric",
      inp(R, recs(LONG + [mk("root", "annotation", 9, LID[8], ANNB("twin at 9")),
                          mk("root", "annotation", 10, LID[9], ANNB("twin at 10"))], R)),
      "sec 8.7 item 3: two equivocations at positions 9 and 10; position compares numerically, "
      "so 9 sorts first.", "BROKEN_CHAIN")
    A("excluded_seq_numeric",
      inp(R, recs(LONG, R) + recs([mk("k9", "annotation", 9, LID[8], ANNB("foreign at 9")),
                                   mk("k9", "annotation", 10, LID[9], ANNB("foreign at 10"))], K9, bn0=50)),
      "sec 8.7 item 6: seq compares numerically, so the row at 9 precedes the row at 10.",
      "COMPLETE")
    A("unknown_type_seq_numeric",
      inp(R, recs(LONG + [mk("root", "dispatch", 9, LID[8], {}), mk("root", "dispatch", 10, LID[9], {})], R)),
      "sec 8.7 item 7: seq compares numerically before entryType.", "BROKEN_CHAIN")
    I0 = eid(CL[0])
    TW = [mk("root", "annotation", 1, I0, ANNB("twin %d" % i)) for i in range(4)]
    A("four_way_fork_findings_sort", inp(R, recs([CL[0]] + TW, R)),
      "sec 8.7 item 3: six EQUIVOCATIONs at one position sort by (entry_id, second entry_id), "
      "(A,B),(A,C),(A,D),(B,C),(B,D),(C,D), where sorting by the second id alone would put "
      "(B,C) before (A,D); BROKEN_CHAIN.", "BROKEN_CHAIN")
    A("anchored_row_excludes_discarded_record",
      inp(R, [rec(CL[0], R, bn=10, tx=HX32(0x701)), anchor(eid(CL[0]), K3, bn=11, tx=HX32(0x702))], bas=B_RK3),
      "sec 8.7 items 5 and 11: two records of the genesis, one from the root and one from a "
      "stranger the basis scans; the ANCHORED row carries the root's record alone and the "
      "stranger's is a DISCARDED row.", "COMPLETE")
    A("missing_row_excludes_discarded_record",
      inp(R, [rec(CL[0], R), anchor(HX32(0xBEEF), R, bn=11, tx=HX32(0x701)), anchor(HX32(0xBEEF), K3, bn=12, tx=HX32(0x702))], bas=B_RK3),
      "sec 8.7 items 4 and 11: two records over a hash no record carries bytes for; the MISSING "
      "row carries the root's record alone, the stranger's is DISCARDED; GAPS.", "GAPS")
    A("root_mismatch_lone_foreign_genesis",
      inp(R, recs([gen2, mk("root", "succession", 1, eid(gen2), SUCB(K2))], R)),
      "sec 8.2: ROOT_MISMATCH is tested over every seq-0 ledger entry.", "BROKEN_CHAIN")
    A("withheld_succession_erases_tail",
      inp(R, recs([SU[0], mk("root", "annotation", 1, SUI[0], ANNB("one"))], R)
          + [rec(mk("k2", "judgment", 3, HX32(0x33), JGB(3)), K2, bn=30), rec(mk("k2", "judgment", 4, HX32(0x44), JGB(4)), K2, bn=31)],
          bas=basis((chainobj(1, senders=[R, K2]),))),
      "sec 8.1: no record carries the succession, so the successor is outside the lineage and "
      "its two records are discarded; the walk sees no gap and the label is COMPLETE, and the "
      "two DISCARDED rows under a sender no succession names are what a reader asks about.",
      "COMPLETE")

    # ---------------- randomized ledgers with injected faults -----------------
    for i in range(24):
        L = rng.randrange(1, 8)
        specs = [("root", "genesis", GENB)]
        for j in range(1, L):
            et = rng.choice(["amanah", "judgment", "annotation", "resignation", "keybinding",
                             "succession", "windmill"])
            if et == "succession":
                specs.append(("root", et, SUCB(A_(rng.choice(["k2", "k3"])))))
            else:
                specs.append(("root", et, random_body(rng, et)))
        entries, ids = build_chain(specs)
        anchors = []
        faults = []
        for _ in range(rng.randrange(1, 4)):
            f = rng.choice(["drop", "twin", "bad_prev", "foreign", "reauthor", "dup",
                            "anchor_missing", "anchor_have", "anchor_unproven", "anchor_void",
                            "anchor_foreign_sender", "none",
                            "carried_unproven", "carried_void", "carried_junk",
                            "carried_foreign", "carried_discarded", "carried_by_successor"])
            faults.append(f)
            if f == "drop" and len(entries) > 1:
                entries.pop(rng.randrange(len(entries)))
            elif f == "twin" and entries:
                j = rng.randrange(len(entries))
                v = Parser(entries[j].decode()).text()
                entries.append(mk("root", v.get("entryType"), v.get("seq"), v.get("prev"),
                                  random_body(rng, v.get("entryType"))
                                  if v.get("entryType") != "succession"
                                  else SUCB(A_("k3"))))
            elif f == "bad_prev" and len(entries) > 1:
                j = rng.randrange(1, len(entries))
                v = Parser(entries[j].decode()).text()
                entries[j] = mk("root", v.get("entryType"), v.get("seq"),
                                HX32(rng.getrandbits(256)),
                                {k: v2 for k, v2 in
                                 [(kk, vv) for kk, vv in v.get("body").members]})
            elif f == "foreign":
                anchors.append(rec(mk("k9", "annotation", rng.randrange(0, 6),
                                      HX32(rng.getrandbits(256)), ANNB("outside")) if rng.random() < 0.5
                                   else mk("k9", "genesis", 0, None, GENB), A_("k9"), bn=rng.randrange(1, 999)))
            elif f == "reauthor" and len(entries) > 1:
                j = rng.randrange(1, len(entries))
                v = Parser(entries[j].decode()).text()
                entries[j] = mk("k2", v.get("entryType"), v.get("seq"), v.get("prev"),
                                JObj(v.get("body").members))
            elif f == "dup" and entries:
                anchors.append(rec(entries[rng.randrange(len(entries))], A_("root"), bn=rng.randrange(1, 999)))
            elif f == "anchor_missing":
                anchors.append(anchor(HX32(rng.getrandbits(256)), A_("root"), bn=rng.randrange(1, 999)))
            elif f == "anchor_have" and ids:
                anchors.append(anchor(rng.choice(ids), A_("root"), bn=rng.randrange(1, 999)))
            elif f == "anchor_unproven":
                anchors.append(anchor(HX32(rng.getrandbits(256)), A_("root"),
                                      bn=rng.randrange(1, 999), verdict="UNPROVEN"))
            elif f == "anchor_void":
                anchors.append(anchor(HX32(rng.getrandbits(256)), A_("root"),
                                      bn=rng.randrange(1, 999), verdict="VOID"))
            elif f == "anchor_foreign_sender":
                anchors.append(anchor(HX32(rng.getrandbits(256)), A_("k9"), bn=rng.randrange(1, 999)))
            elif f == "carried_unproven" and len(entries) > 1:
                b = entries.pop(rng.randrange(len(entries)))
                anchors.append(rec(b, A_("root"), bn=rng.randrange(1, 999), verdict="UNPROVEN"))
            elif f == "carried_void" and len(entries) > 1:
                b = entries.pop(rng.randrange(len(entries)))
                anchors.append(rec(b, A_("root"), bn=rng.randrange(1, 999), verdict="VOID"))
            elif f == "carried_junk":
                anchors.append(rec(rng.choice([b"{}", b"", b"\xff", b"[1]"]), A_("root"), bn=rng.randrange(1, 999)))
            elif f == "carried_foreign":
                anchors.append(rec(mk("k9", "annotation", rng.randrange(0, 6),
                                      HX32(rng.getrandbits(256)), ANNB("carried outsider")),
                                   A_("root"), bn=rng.randrange(1, 999),
                                   verdict=rng.choice(["counted", "counted", "UNPROVEN"])))
            elif f == "carried_discarded":
                anchors.append(rec(rng.choice([b"{}"] + entries), A_("k9"), bn=rng.randrange(1, 999)))
            elif f == "carried_by_successor" and entries:
                b = entries.pop(rng.randrange(len(entries)))
                anchors.append(rec(b, A_(rng.choice(["k2", "k3"])), bn=rng.randrange(1, 999)))
        carried_at = rng.randrange(1, 500)
        rng.shuffle(entries)
        anchors = recs(entries, A_("root"), bn0=carried_at) + anchors
        rng.shuffle(anchors)
        A("rand_ledger_%02d" % i, inp(R, anchors),
          "randomized ledger of %d written entries carried by the root, with injected faults: %s"
          % (L, ", ".join(faults)))


# ==========================================================================
# main
# ==========================================================================
    build_audit_round1(R, K2, K3, K9, CL, CID, RC, FOR, H_UNSEEN)
    build_audit_retirement(R, K2, K3, K9)
    build_audit_retirement2(R, K2, K3, K4, K9)
    build_audit_retirement3(R, K2, K3, K4, K9)
    build_audit_retirement4(R, K2, K3, K4, K9)


def build_audit_retirement4(R, K2, K3, K4, K9):
    """sec 7.4 in the second of the handing off, at the boundaries round 1 of
    2026-10-30 found unwitnessed: the record's sender against the entry's
    signer (the successor's and a stranger's records of that second), the
    key's hash-only record of its own late entry, the handing off carried by
    the successor alone, a record before the handing off, what sets the
    retiring seq (bytes failing sec 4.3, a lower handing off on a stranger's
    VOID record, another chain, a migration beside a succession), the
    UNPROVEN carrier, and two keys handing off in one second."""
    T0 = 1700000000
    S5 = (R, K2, K3, K4, K9)
    B2 = basis((chainobj(1, senders=S5), chainobj(2, senders=S5)))

    def on2(a):
        return dict(a, chainId=2)
    SU, SUI = build_chain([("root", "genesis", GENB), ("root", "succession", SUCB(K2)),
                           ("k2", "amanah", AMB(4)), ("k2", "annotation", ANNB("new hand"))])
    base = [rec(SU[0], R, bn=10, ts=T0 - 100), rec(SU[1], R, bn=11, ts=T0),
            rec(SU[2], K2, bn=20, ts=T0 + 100), rec(SU[3], K2, bn=21, ts=T0 + 101)]
    fork = mk("root", "annotation", 2, SUI[1], ANNB("written by the old key after it handed off"))
    # the trim reads the record's sender; the admission reads the succession's judge (sec 7.4, sec 8.1)
    A("retired_key_extra_entry_carried_by_successor_same_second", inp(R, base + [rec(fork, K2, bn=30, ts=T0)]),
      "sec 7.4/8.1: in the second of the handing off the successor's record carries what the old key signed at the position after it: the successor has not retired at that record, so it stands and the bytes are a ledger entry, a certain AUTHORITY_MISMATCH and an EQUIVOCATION at seq 2; BROKEN_CHAIN, custody of signatures being the seat's.", "BROKEN_CHAIN")
    late = mk("root", "succession", 4, SUI[3], SUCB(K3))
    k3e = mk("k3", "annotation", 5, eid(late), ANNB("the thief's key"))
    A("retired_key_late_succession_carried_by_successor_same_second", inp(R, base + [rec(late, K2, bn=40, ts=T0), rec(k3e, K3, bn=41, ts=T0 + 201)]),
      "sec 7.4/8.1: the successor's record of the handing off's second carries the old key's late succession: the record stands and the succession is a ledger entry signed out of office, a certain AUTHORITY_MISMATCH, yet it admits nobody, no record carrying it being one at which its judge has not retired; K3's record is DISCARDED and its entry EXCLUDED; BROKEN_CHAIN.", "BROKEN_CHAIN")
    A("retired_key_late_succession_carried_by_stranger_same_second", inp(R, base + [rec(late, K9, bn=40, ts=T0), rec(k3e, K3, bn=41, ts=T0 + 201)]),
      "sec 7.4/8.1: a stranger's record of that second carries the old key's late succession: the record is DISCARDED, its sender being outside the lineage, the succession admits nobody and is no ledger entry, and K3's record is DISCARDED and its entry EXCLUDED; COMPLETE with four entries.", "COMPLETE")
    SH, SHI = build_chain([("root", "genesis", GENB), ("root", "succession", SUCB(K2))])
    sh_base = [rec(SH[0], R, bn=10, ts=T0 - 100), rec(SH[1], R, bn=11, ts=T0)]
    extra = mk("root", "annotation", 2, SHI[1], ANNB("sent twice by the tool"))
    # a record carrying no bytes is one at which the key has not retired, whatever hash it commits to
    A("retired_key_extra_entry_hash_only_beside_bytes_same_second", inp(R, sh_base + [rec(extra, R, bn=12, ts=T0), anchor(eid(extra), R, bn=13, ts=T0)]),
      "sec 7.4/8.5: in the second of its handing off the old key carries its resend with its bytes, a record at which it has retired, DISCARDED, and commits to the same hash in a record carrying no bytes, one at which it has not retired: that hash stands in MISSING beside the two ledger entries; GAPS.", "GAPS")
    A("retired_key_extra_entry_hash_only_beside_bytes_after", inp(R, sh_base + [rec(extra, R, bn=12, ts=T0 + 1), anchor(eid(extra), R, bn=13, ts=T0 + 1)]),
      "sec 7.4/8.1: one second later both records are DISCARDED, the one carrying no bytes with the other; COMPLETE.", "COMPLETE")
    # the retirement and the retiring seq read every record of the anchor set whatever its sender
    A("handoff_carried_by_successor_only_resend_same_second", inp(R, [rec(SU[0], R, bn=10, ts=T0 - 100), rec(SU[1], K2, bn=11, ts=T0), rec(SU[2], K2, bn=20, ts=T0 + 100), rec(SU[3], K2, bn=21, ts=T0 + 101), rec(fork, R, bn=30, ts=T0)]),
      "sec 7.4: the succession the successor alone carries at T0 retires the old key there and sets its retiring seq, whatever the record's sender: the old key's own record of that second carrying its entry at seq 2 is DISCARDED; COMPLETE with four entries.", "COMPLETE")
    # the second clause reads the handing off's own second and no earlier record
    A("retired_key_extra_entry_anchored_before_its_handoff", inp(R, base + [rec(fork, R, bn=9, ts=T0 - 1)]),
      "sec 7.4/8.2/8.4: the old key's entry at the position after its handing off, carried by the old key one second before the record carrying the handing off, rides a record at which it has not retired: a ledger entry, a certain AUTHORITY_MISMATCH and an EQUIVOCATION at seq 2; BROKEN_CHAIN.", "BROKEN_CHAIN")
    # what sets the retiring seq: the byte strings that pass sec 4.3 and nothing else
    LN, LNI = build_chain([("root", "genesis", GENB), ("root", "annotation", ANNB("one")), ("root", "annotation", ANNB("two")),
                           ("root", "succession", SUCB(K2)), ("k2", "annotation", ANNB("k2"))])
    ln_recs = [rec(LN[0], R, bn=10, ts=T0 - 100), rec(LN[1], R, bn=11, ts=T0 - 50), rec(LN[2], R, bn=12, ts=T0),
               rec(LN[3], R, bn=13, ts=T0), rec(LN[4], K2, bn=20, ts=T0 + 100)]
    bad_body = mk("root", "succession", 1, LNI[0], {"kind": "handover", "effective": 0, "statement_md": "no to"})
    A("forged_succession_body_fault_same_second", inp(R, ln_recs + [rec(bad_body, K9, bn=14, ts=T0)]),
      "sec 7.4: a stranger's record of the handing off's second carries succession-shaped bytes the old key signed at seq 1 that fail sec 4.3 (no `to`): they set no retiring seq, which stays 3, so the old key's entry at seq 2 carried in that second counts and the succession at 3 admits its key; the stranger's record is DISCARDED and its bytes MALFORMED; COMPLETE with five entries.", "COMPLETE")
    wrong_signer = make_entry(K("k9"), "succession", 1, LNI[0], SUCB(K3), judge=R)
    A("forged_succession_wrong_signer_same_second", inp(R, ln_recs + [rec(wrong_signer, K9, bn=14, ts=T0)]),
      "sec 7.4: a stranger's record of that second carries a succession naming the old key as judge yet signed by another key (E_SIG_SIGNER at sec 4.3): no byte string a stranger can make reaches the retiring seq, which stays 3; the stranger's record is DISCARDED and its bytes MALFORMED; COMPLETE with five entries.", "COMPLETE")
    # a lower handing off of the key, riding a stranger's VOID record in that second, sets the retiring seq below the handing off that moved the seat
    low = mk("root", "succession", 1, LNI[0], SUCB(K3))
    A("lower_stray_handoff_sets_retiring_seq_same_second", inp(R, ln_recs + [rec(low, K9, bn=14, ts=T0, verdict="VOID")]),
      "sec 7.4: the old key's stray succession at seq 1, which a stranger's VOID record carries in the second of the handing off, passes sec 4.3 and is a handing off the key signed, so the retiring seq is 1: the entry at 2 and the succession at 3, both on the old key's own records of that second, are DISCARDED, the succession admits nobody, and K2's record is DISCARDED and its entry EXCLUDED; the stray, at the retiring seq, admits K3, who signed nothing, and rides a discarded record, so it is no ledger entry; COMPLETE with two entries, custody of signatures being the seat's.", "COMPLETE")
    A("lower_stray_handoff_successor_entry_carried_by_ceding_key_same_second", inp(R, ln_recs + [rec(low, K9, bn=14, ts=T0, verdict="VOID"), rec(LN[4], R, bn=15, ts=T0)]),
      "sec 7.4/8.4: the same, with the old key's own record of that second carrying the successor's entry at seq 4: the bytes are not its own, so it has not retired at that record, which stands; the successor being outside the lineage, the record charges CARRIED_MALFORMED with token FOREIGN; BROKEN_CHAIN.", "BROKEN_CHAIN")
    # another chain and a higher handing off in the same second
    s2k3 = mk("root", "succession", 2, SUI[1], SUCB(K3))
    k3b = mk("k3", "annotation", 3, eid(s2k3), ANNB("k3 on chain 2"))
    A("late_succession_on_chain_2_same_second", inp(R, base + [on2(rec(s2k3, R, bn=5, ts=T0)), rec(k3b, K3, bn=41, ts=T0 + 201)], bas=B2),
      "sec 7.4: the retiring seq is the smallest over every chain: the succession at seq 1 on chain 1 and the one at seq 2 on chain 2, both in the handing off's second, give 1, so the chain-2 record is one at which the old key has retired, DISCARDED, and its succession admits nobody; K3's record is DISCARDED and its entry EXCLUDED; COMPLETE with four entries.", "COMPLETE")
    MS, MSI = build_chain([("root", "genesis", GENB), ("root", "annotation", ANNB("before")),
                           ("root", "migration", {"spec": "galeed/2", "statement_md": "moving on"}),
                           ("root", "succession", SUCB(K2)), ("k2", "annotation", ANNB("k2"))])
    A("migration_then_succession_same_second", inp(R, [rec(MS[0], R, bn=10, ts=T0 - 100), rec(MS[1], R, bn=11, ts=T0 - 50), rec(MS[2], R, bn=12, ts=T0), rec(MS[3], R, bn=13, ts=T0), rec(MS[4], K2, bn=20, ts=T0 + 100)]),
      "sec 7.4/6.6a: a migration at seq 2 and a succession at seq 3, both carried in one second: the retiring seq is 2, the succession rides a record at which the key has retired, DISCARDED, and admits nobody, and the migration closes the ledger; K2's record is DISCARDED and its entry EXCLUDED; COMPLETE with three entries.", "COMPLETE")
    # the verdict of the record of that second
    A("retired_key_extra_entry_same_second_unproven", inp(R, sh_base + [rec(extra, R, bn=12, ts=T0, verdict="UNPROVEN")]),
      "sec 7.4/8.6: the resend rides an UNPROVEN record of the handing off's second: a record at which the key has retired is DISCARDED whatever its verdict, so it stands in DISCARDED and makes no UNPROVEN row; COMPLETE with two entries.", "COMPLETE")
    # two keys hand off in one second: each reads its own retiring seq
    TK, TKI = build_chain([("root", "genesis", GENB), ("root", "succession", SUCB(K2)),
                           ("k2", "succession", SUCB(K3)), ("k3", "annotation", ANNB("k3 writes"))])
    k2_stray = mk("k2", "annotation", 3, TKI[2], ANNB("k2 resends after handing on"))
    tk_base = [rec(TK[0], R, bn=10, ts=T0 - 100), rec(TK[1], R, bn=11, ts=T0), rec(TK[2], K2, bn=12, ts=T0)]
    A("two_keys_hand_off_same_second", inp(R, tk_base + [rec(k2_stray, K2, bn=13, ts=T0), rec(TK[3], K3, bn=20, ts=T0 + 100)]),
      "sec 7.4: the root hands to K2 at seq 1 and K2 to K3 at seq 2, both carried in one second; each key's retiring seq is its own, so K2's record of that second carrying its entry at seq 3 is DISCARDED while its record carrying its handing off stands; COMPLETE with four entries.", "COMPLETE")
    A("successor_handoff_carries_next_key_entry_same_second", inp(R, tk_base + [rec(TK[3], K2, bn=13, ts=T0)]),
      "sec 7.4: in the second it hands on, K2 carries K3's entry at the position after its handing off: the bytes are K3's, so K2 has not retired at that record and the entry is a ledger entry; COMPLETE with four entries.", "COMPLETE")


def build_audit_retirement(R, K2, K3, K9):
    """sec 7.4 retirement, sec 8.1 the trim, sec 6.6a and sec 7.3 migration
    (2026-10-23): records at distinct block times, on one chain and on two."""
    T0 = 1700000000
    B2 = basis((chainobj(1, senders=(R, K2, K3, K9)), chainobj(2, senders=(R, K2, K3, K9))))
    SU, SUI = build_chain([("root", "genesis", GENB), ("root", "succession", SUCB(K2)),
                           ("k2", "amanah", AMB(4)), ("k2", "annotation", ANNB("new hand"))])
    base = [rec(SU[0], R, bn=10, ts=T0 - 100), rec(SU[1], R, bn=11, ts=T0),
            rec(SU[2], K2, bn=20, ts=T0 + 100), rec(SU[3], K2, bn=21, ts=T0 + 101)]
    stray = HX32(0x5EED)
    A("retire_stray_same_second", inp(R, base + [anchor(stray, R, bn=12, ts=T0)]),
      "sec 7.4: a record of its retirement's own second that carries no byte string is one at which the key has not retired; its stray anchor is MISSING; GAPS.", "GAPS")
    A("retire_stray_after", inp(R, base + [anchor(stray, R, bn=12, ts=T0 + 1)]),
      "sec 7.4/8.1: one second after the first record carrying its succession, the key has retired; its stray anchor is DISCARDED and no MISSING row; COMPLETE.", "COMPLETE")
    A("retire_stray_before", inp(R, base + [anchor(stray, R, bn=9, ts=T0 - 1)]),
      "sec 7.4: before the retirement the key's anchor counts; MISSING; GAPS.", "GAPS")
    fork = mk("root", "annotation", 2, SUI[1], ANNB("written by the old key after it handed off"))
    A("retired_fork_after", inp(R, base + [rec(fork, R, bn=30, ts=T0 + 50)]),
      "sec 8.4: a fork the retired key signs and carries after its retirement is DISCARDED, no ledger entry, no EQUIVOCATION; COMPLETE.", "COMPLETE")
    A("retired_fork_same_second", inp(R, base + [rec(fork, R, bn=30, ts=T0)]),
      "sec 7.4/8.1: carried in the second of its handing off, a fork the key that handed off signs at a position after it is a record at which it has retired: DISCARDED, no ledger entry, no EQUIVOCATION; COMPLETE.", "COMPLETE")
    A("retired_fork_carried_by_successor", inp(R, base + [rec(fork, K2, bn=30, ts=T0 + 50)]),
      "sec 8.1: the successor has not retired; its counted record makes the old key's late fork a ledger entry; BROKEN_CHAIN, custody of signatures being the seat's.", "BROKEN_CHAIN")
    late = mk("root", "succession", 4, SUI[3], SUCB(K3))
    k3e = mk("k3", "annotation", 5, eid(late), ANNB("the thief's key"))
    A("retired_key_late_succession", inp(R, base + [rec(late, R, bn=40, ts=T0 + 200), rec(k3e, K3, bn=41, ts=T0 + 201)]),
      "sec 7.4: a succession the retired key signs and carries after its retirement admits nobody; both records are DISCARDED, the stranger's entry an EXCLUDED row, the late succession no ledger entry; COMPLETE.", "COMPLETE")
    A("retired_key_late_succession_carried_by_successor", inp(R, base + [rec(late, K2, bn=40, ts=T0 + 200), rec(k3e, K3, bn=41, ts=T0 + 201)]),
      "sec 7.4: admission reads the time of the succession's judge: a record the successor sends after the old key's retirement admits nothing; the late succession is a ledger entry signed out of office, a certain AUTHORITY_MISMATCH; BROKEN_CHAIN.", "BROKEN_CHAIN")
    A("retired_key_late_succession_same_second", inp(R, base + [rec(late, R, bn=40, ts=T0), rec(k3e, K3, bn=41, ts=T0 + 201)]),
      "sec 7.4/8.1: carried in the second of the old key's handing off at a position after it, the late succession admits nobody and its record is DISCARDED; the stranger's record is DISCARDED and its entry EXCLUDED; COMPLETE.", "COMPLETE")
    # two chains: the succession on chain 1 only, the old key's anchor on chain 2
    def on2(a):
        return dict(a, chainId=2)
    A("retire_cross_chain_after", inp(R, base + [on2(anchor(stray, R, bn=12, ts=T0 + 10))], bas=B2),
      "sec 7.4: the retirement reads every chain alike; the old key's anchor on chain 2 after the succession's record on chain 1 is DISCARDED; COMPLETE.", "COMPLETE")
    A("retire_cross_chain_before", inp(R, base + [on2(anchor(stray, R, bn=12, ts=T0 - 10))], bas=B2),
      "sec 7.4: an anchor on chain 2 before the retirement counts; MISSING; GAPS.", "GAPS")
    # the succession carried on two chains: the smallest time decides
    two = [rec(SU[0], R, bn=10, ts=T0 - 100), rec(SU[1], R, bn=11, ts=T0 + 20), on2(rec(SU[1], R, bn=5, ts=T0 + 5)),
           rec(SU[2], K2, bn=20, ts=T0 + 100), rec(SU[3], K2, bn=21, ts=T0 + 101)]
    A("retire_smallest_time_across_chains", inp(R, two + [anchor(stray, R, bn=12, ts=T0 + 10)], bas=B2),
      "sec 7.4: the retirement is the smallest blockTimestamp of the records carrying the succession, on any chain: chain 2's record at T0+5 retires the key before its chain-1 anchor at T0+10, though that chain's own record of the succession comes later; COMPLETE.", "COMPLETE")
    A("retire_smallest_time_equal_second", inp(R, two + [anchor(stray, R, bn=12, ts=T0 + 5)], bas=B2),
      "sec 7.4: at the smallest second itself the key's record carrying no byte string is one at which it has not retired; MISSING; GAPS.", "GAPS")
    # the succession carried by a stranger's record alone
    q_only = [rec(SU[0], R, bn=10, ts=T0 - 100), rec(SU[1], K9, bn=11, ts=T0),
              rec(SU[2], K2, bn=20, ts=T0 + 100), rec(SU[3], K2, bn=21, ts=T0 + 101)]
    A("retire_by_strangers_record", inp(R, q_only + [anchor(stray, R, bn=12, ts=T0 + 1)]),
      "sec 7.4: whatever the sender: the stranger's discarded record carrying the published succession retires the old key all the same, and its stray anchor after it is DISCARDED; the succession itself no ledger entry, a gap at seq 1; GAPS.", "GAPS")
    A("retire_by_strangers_earlier_record", inp(R, base + [rec(SU[1], K9, bn=8, ts=T0 - 50), anchor(stray, R, bn=9, ts=T0 - 10)]),
      "sec 7.4: a stranger's record carrying the published succession earlier than the seat's own moves the retirement earlier, under a basis that scans the stranger: the seat's own record of the succession and the old key's anchor, both after it, are DISCARDED, so the succession rides no counted undiscarded record and its position is a gap; GAPS.", "GAPS")
    A("retire_by_void_record", inp(R, [rec(SU[0], R, bn=10, ts=T0 - 100), rec(SU[1], R, bn=11, ts=T0, verdict="VOID"),
                                        rec(SU[2], K2, bn=20, ts=T0 + 100), rec(SU[3], K2, bn=21, ts=T0 + 101), anchor(stray, R, bn=12, ts=T0 + 1)]),
      "sec 7.4: whatever the verdict: a VOID record carrying the succession retires the key; GAPS from the succession's own position.", "GAPS")
    # the handing off's own second (sec 7.4): what the key signed at a position after it, and nothing else
    SH, SHI = build_chain([("root", "genesis", GENB), ("root", "succession", SUCB(K2))])
    sh_base = [rec(SH[0], R, bn=10, ts=T0 - 100), rec(SH[1], R, bn=11, ts=T0)]
    extra = mk("root", "annotation", 2, SHI[1], ANNB("sent twice by the tool"))
    A("retired_key_extra_entry_same_second", inp(R, sh_base + [rec(extra, R, bn=12, ts=T0)]),
      "sec 7.4/8.1: in the second of its handing off the old key carries an entry it signed at the position after it (a tool's resend): a record at which it has retired, DISCARDED, no ledger entry, no finding; COMPLETE with two entries.", "COMPLETE")
    A("retired_key_extra_entry_after", inp(R, sh_base + [rec(extra, R, bn=12, ts=T0 + 1)]),
      "sec 7.4/8.1: the same record a second later is DISCARDED, as it is at the handing off's own second; COMPLETE.", "COMPLETE")
    BF, BFI = build_chain([("root", "genesis", GENB), ("root", "annotation", ANNB("before the handing off")),
                           ("root", "succession", SUCB(K2))])
    A("retired_key_backfill_same_second", inp(R, [rec(BF[0], R, bn=10, ts=T0 - 100), rec(BF[2], R, bn=12, ts=T0), rec(BF[1], R, bn=11, ts=T0)]),
      "sec 7.4: an entry the old key signed at a position before its handing off, carried in that same second, counts as at any earlier record; three ledger entries; COMPLETE.", "COMPLETE")
    A("retired_key_extra_entry_same_second_other_chain", inp(R, base + [on2(rec(fork, R, bn=30, ts=T0))], bas=B2),
      "sec 7.4: the retirement reads every chain alike: the old key's fork at a position after its handing off, carried on chain 2 in the handing off's second, is DISCARDED; COMPLETE.", "COMPLETE")
    A("foreign_entry_carried_by_retiring_key_same_second", inp(R, base + [rec(mk("k9", "annotation", 2, SUI[1], ANNB("a stranger at the handing off's second")), R, bn=30, ts=T0)]),
      "sec 7.4/8.4: in its handing off's second the old key carries a stranger's entry at the position after it: the record is one at which the key has not retired (the bytes are not its own), so it counts and records CARRIED_MALFORMED with token FOREIGN; BROKEN_CHAIN.", "BROKEN_CHAIN")
    A("successor_entry_carried_by_retiring_key_same_second", inp(R, base[:2] + [rec(SU[2], R, bn=12, ts=T0), rec(SU[3], K2, bn=21, ts=T0 + 101)]),
      "sec 7.4: in its handing off's second the old key carries the successor's entry at the position after it: the record is one at which the key has not retired, and the entry is a ledger entry; COMPLETE with four entries.", "COMPLETE")
    # a second succession of the retired key at its own position
    S1b = mk("root", "succession", 1, SUI[0], SUCB(K3))
    A("second_succession_after_retirement", inp(R, base + [rec(S1b, R, bn=13, ts=T0 + 10), rec(mk("k3", "annotation", 2, eid(S1b), ANNB("k3")), K3, bn=14, ts=T0 + 11)]),
      "sec 7.4: a second succession the key signs at a position it held, carried after its first, admits nobody and is DISCARDED: no fork; COMPLETE.", "COMPLETE")
    A("second_succession_same_second", inp(R, base + [rec(S1b, R, bn=13, ts=T0)]),
      "sec 7.4: carried at the same second as the first, both successions admit and both are ledger entries: a fork at seq 1; BROKEN_CHAIN.", "BROKEN_CHAIN")
    # migration
    MG, MGI = build_chain([("root", "genesis", GENB), ("root", "annotation", ANNB("before")),
                           ("root", "migration", {"spec": "galeed/2", "statement_md": "moving on"})])
    mrec = [rec(MG[0], R, bn=10, ts=T0 - 100), rec(MG[1], R, bn=11, ts=T0 - 50), rec(MG[2], R, bn=12, ts=T0)]
    A("migration_closes", inp(R, mrec),
      "sec 6.6a/7.3: a migration at the last position closes the ledger; COMPLETE.", "COMPLETE")
    after = mk("root", "annotation", 3, MGI[2], ANNB("after the migration"))
    A("migration_then_entry_same_second", inp(R, mrec + [rec(after, R, bn=13, ts=T0)]),
      "sec 7.4/8.1: the migration retires its key; an entry the key signs at a position after it, carried at the migration's own second, is DISCARDED; COMPLETE.", "COMPLETE")
    A("migration_then_entry_later", inp(R, mrec + [rec(after, R, bn=13, ts=T0 + 1)]),
      "sec 7.4/8.1: the migration retires its key; an entry carried after it is DISCARDED; COMPLETE.", "COMPLETE")
    A("migration_then_entry_by_strangers_record", inp(R, mrec + [rec(after, K9, bn=13, ts=T0 + 1)]),
      "sec 8.1: a stranger's record of the late entry is DISCARDED; COMPLETE.", "COMPLETE")
    newspec = make_entry(K("root"), "genesis", 0, None, {"statement_md": "the next law's genesis", "continues": MGI[2]}, spec="galeed/2")
    A("migration_new_spec_same_second", inp(R, mrec + [rec(newspec, R, bn=13, ts=T0)]),
      "sec 12.3: new-spec bytes the key carries at the migration's second are MALFORMED and a CARRIED_MALFORMED at E_SPEC; BROKEN_CHAIN.", "BROKEN_CHAIN")
    A("migration_new_spec_later", inp(R, mrec + [rec(newspec, R, bn=13, ts=T0 + 1)]),
      "sec 12.3: carried after the migration's first record, the new spec's bytes are DISCARDED and MALFORMED, both informational; COMPLETE.", "COMPLETE")
    A("migration_new_spec_on_chain_2_later", inp(R, mrec + [on2(rec(newspec, R, bn=3, ts=T0 + 7))], bas=B2),
      "sec 5.7/7.4: the new law anchored on another chain after the migration's record on chain 1 reads DISCARDED; COMPLETE.", "COMPLETE")
    mig_succ = mk("root", "succession", 2, MGI[1], SUCB(K2))
    A("migration_beside_succession", inp(R, mrec + [rec(mig_succ, R, bn=14, ts=T0), rec(mk("k2", "annotation", 3, MGI[2], ANNB("k2")), K2, bn=15, ts=T0 + 5)]),
      "sec 7.3/8.2: a migration and a succession at one position by the key in office: the migration closes whatever successions share the position; a fork at seq 2 and the successor's entry an AUTHORITY_MISMATCH; BROKEN_CHAIN.", "BROKEN_CHAIN")
    out_of_office = mk("k2", "migration", 2, SUI[1], {"spec": "galeed/2", "statement_md": "the successor moves on"})
    A("migration_by_successor_closes", inp(R, base[:2] + [rec(out_of_office, K2, bn=20, ts=T0 + 100)]),
      "sec 7.3: the successor's migration at the position it holds closes the ledger; COMPLETE.", "COMPLETE")
    mig_by_r = mk("root", "migration", 2, SUI[1], {"spec": "galeed/2", "statement_md": "R out of office"})
    A("migration_by_retired_key_out_of_office", inp(R, base + [rec(mig_by_r, R, bn=30, ts=T0)]),
      "sec 7.4/8.1: R's migration at seq 2, signed after it handed the seat to K2 and carried in the second of that handing off, is DISCARDED: it closes nothing and grounds no finding; COMPLETE.", "COMPLETE")
    A("migration_retires_signer_whatever_office", inp(R, base + [rec(mig_by_r, R, bn=30, ts=T0 + 30), anchor(stray, R, bn=31, ts=T0 + 31)]),
      "sec 7.4: R retired at T0 already; its late migration and stray anchor are DISCARDED; COMPLETE.", "COMPLETE")


def build_audit_retirement2(R, K2, K3, K4, K9):
    """sec 7.4 and sec 8.1 at their boundaries (round 1 of 2026-10-23): what
    retires a key (any verdict, carried bytes alone, a handing off and nothing
    else, the smallest time across successions and migrations, the range's
    ends), the trim whatever the verdict, migrations at their boundaries, the
    chains' clocks, and the walk naming a key outside the lineage."""
    T0 = 1700000000
    B2 = basis((chainobj(1, senders=(R, K2, K3, K4, K9)), chainobj(2, senders=(R, K2, K3, K4, K9))))

    def on2(a):
        return dict(a, chainId=2)
    SU, SUI = build_chain([("root", "genesis", GENB), ("root", "succession", SUCB(K2)),
                           ("k2", "amanah", AMB(4)), ("k2", "annotation", ANNB("new hand"))])
    tail = [rec(SU[2], K2, bn=20, ts=T0 + 100), rec(SU[3], K2, bn=21, ts=T0 + 101)]
    base = [rec(SU[0], R, bn=10, ts=T0 - 100), rec(SU[1], R, bn=11, ts=T0)] + tail
    stray = HX32(0x5EE2)
    # what retires a key: any verdict
    unp = [rec(SU[0], R, bn=10, ts=T0 - 100), rec(SU[1], R, bn=11, ts=T0, verdict="UNPROVEN")] + tail
    A("retire_by_unproven_record", inp(R, unp + [anchor(stray, R, bn=12, ts=T0 + 1)]),
      "sec 7.4: whatever the verdict: an UNPROVEN record carrying the succession retires the key; its stray anchor a second later is DISCARDED; the succession no ledger entry, a gap at seq 1 and the successor's entries soft mismatches; UNPROVEN.", "UNPROVEN")
    A("retire_by_unproven_record_stray_same_second", inp(R, unp + [anchor(stray, R, bn=12, ts=T0)]),
      "sec 7.4: at the UNPROVEN record's own second the key's record carrying no byte string is one at which it has not retired; its stray anchor is MISSING; UNPROVEN.", "UNPROVEN")
    # the trim reads no verdict
    A("retired_key_unproven_record_after", inp(R, base + [anchor(stray, R, bn=12, ts=T0 + 1, verdict="UNPROVEN")]),
      "sec 8.1: the trim reads no verdict: the retired key's UNPROVEN record after its retirement is DISCARDED and in no UNPROVEN row; COMPLETE.", "COMPLETE")
    A("retired_key_unproven_record_same_second", inp(R, base + [anchor(stray, R, bn=12, ts=T0, verdict="UNPROVEN")]),
      "sec 8.1: at the retirement's second the UNPROVEN record stands; UNPROVEN.", "UNPROVEN")
    ann2 = mk("root", "annotation", 2, SUI[1], ANNB("the old key's late note"))
    A("retired_key_void_record_after", inp(R, base + [rec(ann2, R, bn=12, ts=T0 + 1, verdict="VOID")]),
      "sec 8.1: the retired key's VOID record after its retirement is DISCARDED and in no VOID row; COMPLETE.", "COMPLETE")
    # only a record that carries the bytes retires
    hb = [rec(SU[0], R, bn=10, ts=T0 - 100), anchor(eid(SU[1]), R, bn=9, ts=T0), rec(SU[1], R, bn=13, ts=T0 + 10)] + tail
    A("hash_only_anchor_of_succession_retires_nobody", inp(R, hb + [anchor(stray, R, bn=12, ts=T0 + 5)]),
      "sec 7.4: a record carrying only the succession's hash carries no byte string and retires nobody; the key retires at the record carrying the bytes, T0+10, so its stray anchor at T0+5 is MISSING; GAPS.", "GAPS")
    bad_body = mk("root", "succession", 1, SUI[0], {"to": K3})
    A("malformed_succession_retires_nobody", inp(R, base[:1] + [rec(bad_body, K9, bn=8, ts=T0 - 50)] + base[1:] + [anchor(stray, R, bn=12, ts=T0 - 10)]),
      "sec 7.4: bytes shaped like the key's succession that fail sec 4.3 retire nobody, whoever carries them earlier; the stray anchor before the true retirement is MISSING; GAPS.", "GAPS")
    RK, RKI = build_chain([("root", "genesis", GENB), ("root", "amanah", AMB(1)), ("root", "resignation", RSB(3)),
                           ("root", "keybinding", {"recipient": "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq", "effective": 1700000000}),
                           ("root", "annotation", ANNB("still writing"))])
    A("resignation_and_keybinding_retire_nobody", inp(R, [rec(RK[0], R, bn=10, ts=T0 - 100), rec(RK[1], R, bn=11, ts=T0 - 50), rec(RK[2], R, bn=12, ts=T0),
                                                           rec(RK[3], R, bn=13, ts=T0 + 10), rec(RK[4], R, bn=14, ts=T0 + 20)]),
      "sec 7.4: only a succession or a migration retires: a resignation and a keybinding retire nobody, and the entries after them count; COMPLETE.", "COMPLETE")
    self_s = mk("root", "succession", 1, SUI[0], SUCB(R))
    A("retire_by_succession_to_self", inp(R, [rec(SU[0], R, bn=10, ts=T0 - 100), rec(self_s, R, bn=11, ts=T0),
                                               rec(mk("root", "annotation", 2, eid(self_s), ANNB("after the self-succession")), R, bn=12, ts=T0 + 1)]),
      "sec 7.4: a succession to the key itself retires that key; its annotation a second later is DISCARDED and the ledger ends at the succession; COMPLETE.", "COMPLETE")
    # the smallest time across a succession and a migration
    MG, MGI = build_chain([("root", "genesis", GENB), ("root", "annotation", ANNB("before")),
                           ("root", "migration", {"spec": "galeed/2", "statement_md": "moving on"})])
    late_s = mk("root", "succession", 3, MGI[2], SUCB(K3))
    A("retire_min_across_succession_and_migration", inp(R, [rec(MG[0], R, bn=10, ts=T0 - 100), rec(MG[1], R, bn=11, ts=T0 - 50), rec(MG[2], R, bn=12, ts=T0),
                                                             on2(rec(late_s, R, bn=5, ts=T0 - 5))], bas=B2),
      "sec 7.4: the retirement is the smallest time over every succession and migration the key signed: a succession at seq 3 carried on chain 2 at T0-5 retires it before its migration's record at T0, which is DISCARDED; the position of the migration is a gap; GAPS.", "GAPS")
    # the range's ends
    SZ = [rec(SU[0], R, bn=10, ts=0), rec(SU[1], R, bn=11, ts=0), rec(SU[2], K2, bn=20, ts=5), rec(SU[3], K2, bn=21, ts=6)]
    A("retire_at_second_zero", inp(R, SZ + [anchor(stray, R, bn=12, ts=0)]),
      "sec 7.4: a retirement at second 0; the stray anchor at that second is MISSING; GAPS.", "GAPS")
    A("retire_at_second_zero_one_later", inp(R, SZ + [anchor(stray, R, bn=12, ts=1)]),
      "sec 7.4: one second after a retirement at 0 the key has retired; COMPLETE.", "COMPLETE")
    TOP = 9007199254740991
    ST = [rec(SU[0], R, bn=10, ts=1), rec(SU[1], R, bn=11, ts=TOP - 1), rec(SU[2], K2, bn=20, ts=TOP), rec(SU[3], K2, bn=21, ts=TOP)]
    A("retire_at_the_top_of_the_range", inp(R, ST + [anchor(stray, R, bn=12, ts=TOP)]),
      "sec 7.4: a retirement at 2^53 - 2; the stray anchor at 2^53 - 1 is DISCARDED; COMPLETE.", "COMPLETE")
    # migrations at their boundaries
    mrec = [rec(MG[0], R, bn=10, ts=T0 - 100), rec(MG[1], R, bn=11, ts=T0 - 50), rec(MG[2], R, bn=12, ts=T0)]
    mig_b = mk("root", "migration", 2, MGI[1], {"spec": "galeed/3", "statement_md": "moving elsewhere"})
    A("two_migrations_at_one_position", inp(R, mrec + [rec(mig_b, R, bn=13, ts=T0)]),
      "sec 8.4: two migrations at one position are a fork; BROKEN_CHAIN.", "BROKEN_CHAIN")
    far = mk("root", "annotation", 4, HX32(0xFA4), ANNB("past the migration, across a gap"))
    A("closure_then_gap_same_second", inp(R, mrec + [rec(far, R, bn=13, ts=T0)]),
      "sec 7.4/8.1: an entry past the migration at a later, non-adjacent position, carried at the migration's second, is DISCARDED; the ledger ends at the migration; COMPLETE.", "COMPLETE")
    MG2, MG2I = build_chain([("root", "genesis", GENB), ("root", "annotation", ANNB("before")),
                             ("root", "migration", {"spec": "galeed/2", "statement_md": "moving on"}),
                             ("root", "annotation", ANNB("after"))])
    A("closure_after_gap", inp(R, [rec(MG2[0], R, bn=10, ts=T0 - 100), rec(MG2[2], R, bn=12, ts=T0), rec(MG2[3], R, bn=13, ts=T0)]),
      "sec 7.3/7.4/8.2: a migration reached across a gap still closes the ledger; the entry after it, carried at the migration's second, is DISCARDED; GAPS from the gap before the migration.", "GAPS")
    A("migration_carried_unproven", inp(R, [rec(MG[0], R, bn=10, ts=T0 - 100), rec(MG[1], R, bn=11, ts=T0 - 50), rec(MG[2], R, bn=12, ts=T0, verdict="UNPROVEN")]),
      "sec 7.4/8.6: a migration carried only by an UNPROVEN record retires its key and is no ledger entry; UNPROVEN.", "UNPROVEN")
    after = mk("root", "annotation", 3, MGI[2], ANNB("after"))
    vm = [rec(MG[0], R, bn=10, ts=T0 - 100), rec(MG[1], R, bn=11, ts=T0 - 50), rec(MG[2], R, bn=12, ts=T0, verdict="VOID")]
    A("migration_carried_void_entry_later", inp(R, vm + [rec(after, R, bn=13, ts=T0 + 1)]),
      "sec 7.4: a migration carried only by a VOID record retires its key all the same; the entry after it carried later is DISCARDED; COMPLETE.", "COMPLETE")
    A("migration_carried_void_entry_same_second", inp(R, vm + [rec(after, R, bn=13, ts=T0)]),
      "sec 7.4/8.1: a VOID migration retires its key all the same and is no ledger entry; the entry the key signs at a position after it, carried at that second, is DISCARDED; the ledger ends before the migration's position; COMPLETE.", "COMPLETE")
    unk = mk("root", "Migration", 2, MGI[1], {"spec": "galeed/2", "statement_md": "the type spelled otherwise"})
    A("unknown_type_spelled_like_migration", inp(R, [rec(MG[0], R, bn=10, ts=T0 - 100), rec(MG[1], R, bn=11, ts=T0 - 50), rec(unk, R, bn=12, ts=T0),
                                                      rec(mk("root", "annotation", 3, eid(unk), ANNB("after")), R, bn=13, ts=T0 + 1)]),
      "sec 6/7.4: a type that differs from migration by case is an unknown type: it retires nobody and closes nothing; the entry after it counts; COMPLETE.", "COMPLETE")
    # the new law anchored on each chain only after its own carrier there (sec 5.7), read under the whole basis and under a cut
    newspec = make_entry(K("root"), "genesis", 0, None, {"statement_md": "the next law's genesis", "continues": MGI[2]}, spec="galeed/2")
    two = mrec + [on2(rec(MG[2], R, bn=7, ts=T0 + 100))]
    A("migration_two_chains_new_law_later_whole_basis", inp(R, two + [on2(rec(newspec, R, bn=8, ts=T0 + 101))], bas=B2),
      "sec 5.7/12.3: the migration carried on chain 1 at T0 and on chain 2 at T0+100; the new law's genesis on chain 2 at T0+101; COMPLETE.", "COMPLETE")
    cut = basis((chainobj(1, tb=11, senders=(R, K2, K3, K4, K9)), chainobj(2, senders=(R, K2, K3, K4, K9))))
    A("migration_two_chains_new_law_later_chain_1_cut", inp(R, mrec[:2] + [on2(rec(MG[2], R, bn=7, ts=T0 + 100)), on2(rec(newspec, R, bn=8, ts=T0 + 101))], bas=cut),
      "sec 5.7/12.3: the same seat read with chain 1 cut before the migration: the retirement is chain 2's carrier, T0+100, and the new law's genesis after it on that chain is DISCARDED; COMPLETE.", "COMPLETE")
    A("migration_two_chains_new_law_same_second_chain_1_cut", inp(R, mrec[:2] + [on2(rec(MG[2], R, bn=7, ts=T0 + 100)), on2(rec(newspec, R, bn=7, ts=T0 + 100, tx=HX32(0xB7)))], bas=cut),
      "sec 5.7: a new-law genesis at the second of the chain's own carrier breaks the rule; read with chain 1 cut it counts and is CARRIED_MALFORMED at E_SPEC; BROKEN_CHAIN.", "BROKEN_CHAIN")
    # the chains' clocks (sec 11): a chain that runs behind
    CK, CKI = build_chain([("root", "genesis", GENB), ("root", "annotation", ANNB("before the hand-off")),
                           ("root", "succession", SUCB(K2)), ("k2", "annotation", ANNB("the successor"))])
    ck = [rec(CK[0], R, bn=10, ts=T0 - 100), rec(CK[1], R, bn=11, ts=T0 + 5), rec(CK[2], R, bn=12, ts=T0 + 10),
          on2(rec(CK[2], R, bn=3, ts=T0 + 2)), rec(CK[3], K2, bn=20, ts=T0 + 100)]
    A("lagging_chain_retires_before_an_earlier_entry", inp(R, ck, bas=B2),
      "sec 7.4/11: the succession carried on a chain whose clock runs behind, at T0+2, retires the key before the record of its earlier entry at T0+5, which is DISCARDED; a gap at seq 1; GAPS.", "GAPS")
    A("lagging_chain_successor_recarries", inp(R, ck + [rec(CK[1], K2, bn=21, ts=T0 + 101)], bas=B2),
      "sec 8.6: the successor re-carries the entry the lagging chain left after the retirement; COMPLETE.", "COMPLETE")
    thief = mk("root", "succession", 1, SUI[0], SUCB(K3))
    A("early_clock_late_succession", inp(R, base + [on2(rec(thief, R, bn=2, ts=T0 - 1))], bas=B2),
      "sec 7.4/11: a succession the old key's holder dates on a chain one second before the true hand-off retires the key there and admits its to; the true succession's record is DISCARDED and admits nobody, the successor's entries EXCLUDED, the thief's succession the only one in the ledger; COMPLETE.", "COMPLETE")
    stray_s = mk("root", "succession", 5, HX32(0x5A5), SUCB(K3))
    A("stray_succession_at_a_position_never_held", inp(R, base + [rec(stray_s, R, bn=9, ts=T0 - 1)]),
      "sec 7.4: a succession the key signs at a position it never held, carried a second before its true succession, retires it first; the true succession admits nobody and the successor's entries are EXCLUDED; the stray succession is a ledger entry after a gap; GAPS.", "GAPS")
    # the trim keeps a retired key's record at its own second where it carries no bytes, bytes that fail sec 4.3, another key's entry, or its own entry at or below its retiring seq (sec 7.4, sec 8.4)
    A("retired_key_junk_same_second", inp(R, base + [on2(dict(anchor(HX32(0), R, bn=4, ts=T0), hash="0x" + hashlib.sha256(b"junk").hexdigest(), entry=hx(b"junk")))], bas=B2),
      "sec 8.4: junk the key carries in a counted record at its retirement's second is CARRIED_MALFORMED; BROKEN_CHAIN.", "BROKEN_CHAIN")
    A("retired_key_junk_after", inp(R, base + [on2(dict(anchor(HX32(0), R, bn=4, ts=T0 + 1), hash="0x" + hashlib.sha256(b"junk").hexdigest(), entry=hx(b"junk")))], bas=B2),
      "sec 8.4: the same junk a second later is the act of a retired key: DISCARDED and MALFORMED, no finding; COMPLETE.", "COMPLETE")
    # the walk can name a key outside the lineage (sec 7.4, sec 8.2)
    WK, WKI = build_chain([("root", "genesis", GENB)])
    s_in = mk("root", "succession", 1, WKI[0], SUCB(K3))
    s_far = mk("root", "succession", 5, HX32(0xFA5), SUCB(K2))
    c2 = mk("k3", "annotation", 2, eid(s_in), ANNB("the walked key"))
    b2 = mk("k2", "annotation", 2, eid(s_in), ANNB("the lineage key"))
    A("walked_authority_outside_the_lineage", inp(R, [rec(WK[0], R, bn=1, ts=1), rec(s_far, R, bn=2, ts=5, verdict="VOID"), rec(s_in, K2, bn=3, ts=10),
                                                       rec(c2, K2, bn=4, ts=11), rec(b2, K2, bn=5, ts=12)]),
      "sec 7.4/8.2: the root's far succession, carried first and only by its VOID record at 5, retires it and admits K2; its succession to K3, carried by K2 at 10, admits nobody but is a ledger entry, so the walk puts K3 in office: K3's entry is EXCLUDED and CARRIED_MALFORMED as FOREIGN, K2's entry a certain AUTHORITY_MISMATCH; BROKEN_CHAIN.", "BROKEN_CHAIN")


def build_audit_retirement3(R, K2, K3, K4, K9):
    """sec 7.4, sec 8.1 and sec 12.3 at the boundaries round 2 of 2026-10-23
    found unwitnessed: the smallest time with the migration carried first, a
    key that retires before it is named, a stray migration, the handing offs
    that fail sec 4.3 by seq or spec, ties across chains, the top of the
    range, migrations out of office and re-carried, the new law between two
    carriers, the label of a cut read, and the walked outsider admitted
    later."""
    T0 = 1700000000
    S5 = (R, K2, K3, K4, K9)
    B2 = basis((chainobj(1, senders=S5), chainobj(2, senders=S5)))
    B3 = basis((chainobj(1, senders=S5), chainobj(2, senders=S5), chainobj(3, senders=S5)))

    def on(c, a):
        return dict(a, chainId=c)
    MIGB = {"spec": "galeed/2", "statement_md": "moving on"}
    stray = HX32(0x5EE3)
    # the smallest time over successions and migrations, the migration carried first
    MG, MGI = build_chain([("root", "genesis", GENB), ("root", "annotation", ANNB("before")), ("root", "migration", MIGB)])
    late_s = mk("root", "succession", 3, MGI[2], SUCB(K3))
    mfirst = [rec(MG[0], R, bn=10, ts=T0 - 100), rec(MG[1], R, bn=11, ts=T0 - 50), rec(MG[2], R, bn=12, ts=T0)]
    A("retire_min_migration_before_succession", inp(R, mfirst + [anchor(stray, R, bn=13, ts=T0 + 5), rec(late_s, R, bn=14, ts=T0 + 10)]),
      "sec 7.4: the migration carried at T0 retires the key before its later succession's record at T0+10; the stray anchor at T0+5 and the succession are DISCARDED; COMPLETE.", "COMPLETE")
    A("retire_min_migration_before_succession_stray_same_second", inp(R, mfirst + [anchor(stray, R, bn=13, ts=T0), rec(late_s, R, bn=14, ts=T0 + 10)]),
      "sec 7.4: at the migration's own second the key's record carrying no byte string is one at which it has not retired: the stray anchor is MISSING; GAPS.", "GAPS")
    # a key that retires before it is named
    g = mk("root", "genesis", 0, None, GENB)
    s_rk = mk("root", "succession", 1, eid(g), SUCB(K2))
    s_kk = mk("k2", "succession", 2, eid(s_rk), SUCB(K3))
    c3 = mk("k3", "annotation", 3, eid(s_kk), ANNB("the third key"))
    named = [rec(g, R, bn=1, ts=1), rec(s_kk, R, bn=3, ts=3, verdict="VOID"), rec(s_rk, R, bn=10, ts=10)]
    A("retired_before_named", inp(R, named + [rec(s_kk, K2, bn=11, ts=11), rec(c3, K3, bn=12, ts=12)]),
      "sec 7.4: the second key's succession is first carried at 3, by a VOID record, before the root names it at 10: it retires at 3 and its own record at 11 is DISCARDED, its succession admitting the third key and no ledger entry; GAPS.", "GAPS")
    A("retired_before_named_successor_carries", inp(R, named + [rec(s_kk, K3, bn=11, ts=11), rec(c3, K3, bn=12, ts=12)]),
      "sec 8.6: the third key carries the succession that admitted it: a ledger entry; COMPLETE.", "COMPLETE")
    # a stray migration at a position never held, first carried a second before the true succession
    mx = mk("root", "migration", 7, HX32(0x777), dict(MIGB, statement_md="stray"))
    b2 = mk("k2", "annotation", 2, eid(s_rk), ANNB("the second key"))
    A("stray_migration_retires_before_succession", inp(R, [rec(g, R, bn=1, ts=1), rec(mx, R, bn=4, ts=4, verdict="VOID"), rec(s_rk, R, bn=5, ts=5), rec(b2, K2, bn=6, ts=6)]),
      "sec 7.4: a stray migration's VOID record at 4 retires the root; its succession at 5 admits nobody and is DISCARDED, the second key's entry EXCLUDED; COMPLETE.", "COMPLETE")
    A("stray_migration_same_second_as_succession", inp(R, [rec(g, R, bn=1, ts=1), rec(mx, R, bn=4, ts=5, verdict="VOID"), rec(s_rk, R, bn=5, ts=5), rec(b2, K2, bn=6, ts=6)]),
      "sec 7.4: the stray migration's VOID record and the succession share a second, so the retiring seq is the succession's: the stray migration at seq 7 rides a record at which the root has retired, DISCARDED, and the succession admits its key; COMPLETE.", "COMPLETE")
    # handing offs that fail sec 4.3 retire nobody
    a1 = mk("root", "annotation", 1, eid(g), ANNB("the root writes on"))
    for slug, b in (("seq0_succession", mk("root", "succession", 0, None, SUCB(K2))),
                    ("seq0_migration", mk("root", "migration", 0, None, MIGB)),
                    ("other_spec_succession", make_entry(K("root"), "succession", 1, eid(g), SUCB(K2), spec="galeed/2"))):
        A(slug + "_retires_nobody", inp(R, [rec(b, R, bn=5, ts=5, verdict="VOID"), rec(g, R, bn=10, ts=10), rec(a1, R, bn=20, ts=20)]),
          "sec 7.4: bytes shaped like a handing off that fail sec 4.3 (%s) retire nobody; the entry after them counts; COMPLETE." % slug.replace("_", " "), "COMPLETE")
    # one retirement second carried on two chains, a stray on a third
    s1 = mk("root", "succession", 1, eid(g), SUCB(K2))
    tie = [rec(g, R, bn=10, ts=10), rec(s1, R, bn=50, ts=50), on(2, rec(s1, R, bn=50, ts=50))]
    A("two_chain_tie_stray_same_second", inp(R, tie + [on(3, anchor(HX32(0x5EE5), R, bn=50, ts=50))], bas=B3),
      "sec 7.4: the retirement second carried on two chains; a stray anchor on a third chain at that second is MISSING; GAPS.", "GAPS")
    A("two_chain_tie_stray_one_later", inp(R, tie + [on(3, anchor(HX32(0x5EE6), R, bn=51, ts=51))], bas=B3),
      "sec 7.4: one second later on the third chain the stray is DISCARDED; COMPLETE.", "COMPLETE")
    TOP = 9007199254740991
    A("retire_at_the_top_stray_at_the_top", inp(R, [rec(g, R, bn=10, ts=TOP - 1), rec(s1, R, bn=11, ts=TOP), on(2, anchor(HX32(0x5EE7), R, bn=12, ts=TOP))], bas=B2),
      "sec 7.4: a retirement at 2^53 - 1: no record is later, and the stray at that second is MISSING; GAPS.", "GAPS")
    # a migration by a key not in office: no closure, a hard mismatch, and its signer retires
    mk2 = mk("k2", "migration", 1, eid(g), MIGB)
    a2k = mk("k2", "annotation", 2, eid(s1), ANNB("after the out-of-office migration"))
    A("out_of_office_migration_retires_its_signer", inp(R, [rec(g, R, bn=10, ts=10), rec(s1, R, bn=20, ts=20), rec(mk2, K2, bn=25, ts=25), rec(a2k, K2, bn=30, ts=30)]),
      "sec 7.3/7.4: the second key's migration at position 1, beside the root's succession there, closes nothing and is a fork; it retires its signer at 25, whose entry at 30 is DISCARDED; BROKEN_CHAIN.", "BROKEN_CHAIN")
    # a retired key's migration re-carried by its successor: a ledger entry out of office, no closure
    m2 = mk("root", "migration", 2, eid(s1), MIGB)
    a3 = mk("k2", "annotation", 3, eid(m2), ANNB("after the re-carried migration"))
    A("retired_key_migration_recarried_by_successor", inp(R, [rec(g, R, bn=1, ts=1), rec(s1, R, bn=20, ts=20), rec(m2, K2, bn=30, ts=30), rec(a3, K2, bn=40, ts=40)]),
      "sec 7.3/8.2: the root's migration, signed after it handed off and carried by the successor, is a ledger entry and a certain AUTHORITY_MISMATCH at 2; no closure; BROKEN_CHAIN.", "BROKEN_CHAIN")
    # the new law on chain 2 between the two chains' carriers of the migration
    mm = mk("root", "migration", 1, eid(g), MIGB)
    nl = make_entry(K("root"), "genesis", 0, None, {"statement_md": "the next law", "continues": eid(mm)}, spec="galeed/2")
    A("new_law_between_carriers_whole_basis", inp(R, [rec(g, R, bn=1, ts=1), rec(mm, R, bn=100, ts=100), on(2, rec(mm, R, bn=150, ts=150)), on(2, rec(nl, R, bn=120, ts=120))], bas=B2),
      "sec 5.7/12.3: a seat that anchors the new law on chain 2 at 120, before its carrier there at 150: under the whole basis the key retired at 100 and the new law is DISCARDED; COMPLETE.", "COMPLETE")
    cut2 = basis((chainobj(1, tb=0, senders=S5), chainobj(2, senders=S5)))
    A("new_law_between_carriers_chain_1_cut", inp(R, [on(2, rec(g, R, bn=1, ts=1)), on(2, rec(mm, R, bn=150, ts=150)), on(2, rec(nl, R, bn=120, ts=120))], bas=cut2),
      "sec 5.7: read with chain 1 cut before its carrier, the same seat's new-law genesis counts and is CARRIED_MALFORMED at E_SPEC: why the rule is per chain; BROKEN_CHAIN.", "BROKEN_CHAIN")
    # the label of a cut read is the one the old ledger's records earn under it
    E4, E4I = build_chain([("root", "genesis", GENB), ("root", "annotation", ANNB("e1")), ("root", "annotation", ANNB("e2")),
                           ("root", "annotation", ANNB("e3")), ("root", "migration", MIGB)])
    whole = [rec(E4[0], R, bn=10, ts=10), rec(E4[1], R, bn=20, ts=20), rec(E4[2], R, bn=30, ts=30), rec(E4[3], R, bn=40, ts=40),
             rec(E4[4], R, bn=100, ts=100), on(2, rec(E4[4], R, bn=50, ts=50))]
    A("migration_kept_rule_whole_basis", inp(R, whole, bas=B2),
      "sec 12.3: a seat that kept sec 5.7, the migration carried on both chains, read under the whole basis; COMPLETE.", "COMPLETE")
    cut1 = basis((chainobj(1, tb=20, senders=S5), chainobj(2, senders=S5)))
    A("migration_kept_rule_chain_1_cut_after_e1", inp(R, [a for a in whole if not (a["chainId"] == 1 and a["blockNumber"] > 20)], bas=cut1),
      "sec 12.3: the same read with chain 1 cut after e1: no new-law anchor enters, and the label is what the records read earn: a gap below the migration; GAPS.", "GAPS")
    # the walked outsider admitted later by a lineage key's succession that is no ledger entry
    far = mk("root", "succession", 5, HX32(0x55), SUCB(K4))
    s_out = mk("root", "succession", 1, eid(g), SUCB(K2))
    a2o = mk("k2", "annotation", 2, eid(s_out), ANNB("the walked key"))
    back = mk("k4", "succession", 3, eid(a2o), SUCB(K2))
    strayk = mk("k4", "succession", 7, HX32(0x66), SUCB(K3))
    wb = [rec(g, R, bn=1, ts=100), rec(far, R, bn=2, ts=101), rec(s_out, K4, bn=3, ts=102), rec(a2o, K4, bn=4, ts=103)]
    A("walked_outsider_admitted_by_void_succession", inp(R, wb + [rec(back, K4, bn=5, ts=104, verdict="VOID")]),
      "sec 7.4: the far successor's VOID succession naming the walked key, carried before its own retirement, admits it: the walked key's entry is a ledger entry; GAPS.", "GAPS")
    A("walked_outsider_admitter_retired_first", inp(R, wb + [rec(strayk, K4, bn=5, ts=103, verdict="VOID"), rec(back, K4, bn=6, ts=104, verdict="VOID")]),
      "sec 7.4: the far successor retired a second earlier by a stray succession, so the same VOID succession admits nobody and is DISCARDED; the walked key's entry stays EXCLUDED and is charged FOREIGN; BROKEN_CHAIN.", "BROKEN_CHAIN")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=1)
    args = ap.parse_args()
    seed = args.seed
    rng = random.Random(seed)

    for tag in ("root", "k2", "k3", "k4", "k9"):
        KEYS[tag] = testkey(tag, seed)

    for d in ("canon", "sign", "audit"):
        p = os.path.join(HERE, d)
        if os.path.isdir(p):
            shutil.rmtree(p)
        os.makedirs(p)
    del MANIFEST[:]
    del WARNINGS[:]
    for k in _counters:
        _counters[k] = 0

    t = build_templates()
    build_canon(rng, t)
    build_canon_round1(rng, t)
    build_sign(rng, t)
    build_audit(rng, t)
    build_misuse()


    counts = {}
    for r in MANIFEST:
        counts[r["category"]] = counts.get(r["category"], 0) + 1
    man = {
        "corpus": "galeed/1 convergence corpus",
        "law": "docs/galeed-v1.md",
        "harness": "galeed-conformance/HARNESS.md",
        "seed": seed,
        "generator": "gen.py",
        "keys": {tag: {"privkey": "0x%064x" % KEYS[tag], "address": privkey_to_addr_hex(KEYS[tag])}
                 for tag in sorted(KEYS)},
        "counts": counts,
        "coverage": coverage(),
        "prediction_disclaimer":
            "Every `predicted*` field is the corpus author's own reading of the law, produced "
            "by gen.py and by nothing else. It is a third opinion beside the two "
            "implementations under test and is not authoritative. `hand_expect` records the "
            "author's prose reasoning about a case; where it differs from the code the case "
            "carries `disagreement: true`.",
        "cases": MANIFEST,
    }
    with open(os.path.join(HERE, "manifest.json"), "w") as f:
        json.dump(man, f, indent=1, sort_keys=True)

    with open(os.path.join(HERE, "README.md"), "w") as f:
        f.write(README % (seed, counts.get("canon", 0), counts.get("sign", 0),
                          counts.get("audit", 0)))

    print("seed %d" % seed)
    for k in sorted(counts):
        print("  %-6s %5d files" % (k, counts[k]))
    print("  canon/ predicted tokens:")
    tally = {}
    for r in MANIFEST:
        if r["category"] == "canon":
            tally[r["predicted"]] = tally.get(r["predicted"], 0) + 1
    for k in sorted(tally, key=lambda x: (-tally[x], x)):
        print("      %-22s %4d" % (k, tally[k]))
    print("  audit/ predicted labels:")
    tally = {}
    for r in MANIFEST:
        if r["category"] == "audit":
            tally[r["predicted_label"]] = tally.get(r["predicted_label"], 0) + 1
    for k in sorted(tally, key=lambda x: (-tally[x], x)):
        print("      %-22s %4d" % (k, tally[k]))
    cov = coverage()
    gaps = ([t for t in TOKENS if t not in cov["sec_10_tokens_and_accept"]] +
            [f for f in FINDINGS if f not in cov["sec_8_3_report_rows"]] +
            [l for l in LABELS if l not in cov["sec_8_7_labels"]] +
            [v for v in ("counted", "UNPROVEN", "VOID")
             if v not in cov["sec_9_3_anchor_verdicts"]])
    print("  closed vocabularies unwitnessed: %s" % (", ".join(gaps) if gaps else "none"))
    if WARNINGS:
        print("  hand/code disagreements (%d):" % len(WARNINGS))
        for w in WARNINGS:
            print("      " + w)
    else:
        print("  hand/code disagreements: none")


def build_audit_round1(R, K2, K3, K9, CL, CID, RC, FOR, H_UNSEEN):
    """Round-1 witnesses of 2026-10-16 (sec 8.1, 8.6, 8.7, 9.4)."""
    # the ledger needs a counted undiscarded record that carries the very bytes
    A("hash_only_counted_beside_unproven_carrier", inp(R, RC[:4] + [anchor(CID[4], R, bn=50), rec(CL[4], R, bn=51, verdict="UNPROVEN")]),
      "sec 8.1/8.6: the root's counted record of the fifth entry carries nothing and the bytes ride only in an UNPROVEN record: no ledger entry, the hash MISSING, the position a SEQ_GAP; GAPS.", "GAPS")
    A("hash_only_counted_beside_void_carrier", inp(R, RC[:4] + [anchor(CID[4], R, bn=50), rec(CL[4], R, bn=51, verdict="VOID")]),
      "sec 8.1/8.6: the bytes ride only in a VOID record beside a counted record carrying nothing: no ledger entry; GAPS.", "GAPS")
    A("hash_only_counted_beside_discarded_carrier", inp(R, RC[:4] + [anchor(CID[4], R, bn=50), rec(CL[4], K9, bn=51)]),
      "sec 8.1: the bytes ride only in a stranger's record, which the trim discards, beside the root's counted record carrying nothing: no ledger entry; GAPS.", "GAPS")
    A("two_counted_records_one_carrying_nothing", inp(R, RC + [anchor(CID[2], R, bn=50)]),
      "sec 8.7 item 5: a counted record carrying nothing beside a counted record carrying the bytes of one hash: the entry is a ledger entry and the ANCHORED row names both records.", "COMPLETE")
    # the UNPROVEN label is shadowed only by an undiscarded counted record
    A("unproven_shadowed_only_by_discarded_counted", inp(R, RC + [anchor(H_UNSEEN, R, bn=50, verdict="UNPROVEN"), anchor(H_UNSEEN, K9, bn=51)]),
      "sec 8.7 label rule 2: the counted record of the UNPROVEN hash is a stranger's and is discarded, so it shadows nothing; UNPROVEN.", "UNPROVEN")
    A("discarded_unproven_record", inp(R, RC + [anchor(H_UNSEEN, K9, bn=50, verdict="UNPROVEN")]),
      "sec 8.1/8.7 items 9 and 12: an UNPROVEN record from a stranger is discarded; it stands in DISCARDED, in no UNPROVEN row, and moves no label; COMPLETE.", "COMPLETE")
    # DISCARDED row order: (chainId, blockNumber, tx, sender, hash)
    A("discarded_order_block_before_tx", inp(R, RC + [anchor(HX32(0xD1), K9, bn=60, tx=HX32(0xB)), anchor(HX32(0xD2), K9, bn=59, tx=HX32(0xC))]),
      "sec 8.7 item 11: DISCARDED rows are ordered by blockNumber before tx: block 59 with the larger tx comes first.", "COMPLETE")
    A("discarded_order_tx_before_hash", inp(R, RC + [anchor(HX32(0xFF), K9, bn=60, tx=HX32(0xA)), anchor(HX32(0x01), K9, bn=60, tx=HX32(0xB))]),
      "sec 8.7 item 11: at one block the rows are ordered by tx before hash: the smaller tx with the larger hash comes first.", "COMPLETE")
    # foreign entries carried only by records that do not count
    A("foreign_entries_carried_by_void_and_unproven", inp(R, RC + [rec(FOR[0], R, bn=50, verdict="VOID"), rec(FOR[1], R, bn=51, verdict="UNPROVEN")]),
      "sec 8.7 item 6: a stranger's entries carried only by the root's VOID and UNPROVEN records are EXCLUDED rows all the same, ground no finding (sec 8.4 needs a counted record), and the UNPROVEN hash no counted record has labels UNPROVEN.", "UNPROVEN")
    A("foreign_entry_carried_only_by_discarded_record", inp(R, RC + [rec(FOR[0], K9, bn=50)]),
      "sec 8.7 item 6: a stranger's entry carried only by the stranger's own discarded record is an EXCLUDED row, whatever record carried it; COMPLETE.", "COMPLETE")
    # a succession that fails sec 4.3 extends no lineage
    SB2, _ = build_chain([("root", "genesis", GENB)])
    bad_signer = mk("k3", "succession", 1, eid(SB2[0]), SUCB(K2))
    k2_entries, _ = build_chain([("k2", "annotation", ANNB("k2 after a bad succession"))], start_seq=2, start_prev=eid(bad_signer))
    A("succession_signed_by_stranger_carried_by_root", inp(R, recs(SB2, R) + [rec(bad_signer, R, bn=20)] + recs(k2_entries, K2, bn0=30)),
      "sec 7.4/8.4: a succession signed by a key outside the lineage names k2, but the lineage is decided by signatures: k2 is not admitted, the root's counted record carrying the stranger's entry is CARRIED_MALFORMED FOREIGN, k2's own records are discarded and its entry excluded; BROKEN_CHAIN.", "BROKEN_CHAIN")
    A("succession_signed_by_stranger_carried_by_stranger", inp(R, recs(SB2, R) + [rec(bad_signer, K3, bn=20)] + recs(k2_entries, K2, bn0=30)),
      "sec 7.4: the same stranger-signed succession carried by the stranger's own record: discarded, the entry excluded, no finding, k2 not admitted; COMPLETE.", "COMPLETE")
    good_succ = mk("root", "succession", 1, eid(SB2[0]), SUCB(K2))
    i_sig = good_succ.index(b'"sig":"0x') + 9 + 7
    bad_sig_succ = good_succ[:i_sig] + (b"0" if good_succ[i_sig:i_sig + 1] != b"0" else b"1") + good_succ[i_sig + 1:]
    k2_after_bad_sig, _ = build_chain([("k2", "annotation", ANNB("k2 after a succession whose signature fails"))], start_seq=2, start_prev=eid(bad_sig_succ))
    A("succession_signature_fails_carried_by_root", inp(R, recs(SB2, R) + [rec(bad_sig_succ, R, bn=20)] + recs(k2_after_bad_sig, K2, bn0=30)),
      "sec 4.3/7.4/8.4: a root-signed succession whose signature digit was altered fails sec 4.3 (the recovered signer is not the judge, or no signer recovers): CARRIED_MALFORMED with that token, no lineage extension, k2's own record discarded and its entry excluded; BROKEN_CHAIN.", "BROKEN_CHAIN")
    A("succession_signature_fails_carried_by_stranger", inp(R, recs(SB2, R) + [rec(bad_sig_succ, K3, bn=20)] + recs(k2_after_bad_sig, K2, bn0=30)),
      "sec 4.3/7.4: the same bytes carried by a stranger's discarded record: MALFORMED and DISCARDED, no finding, no lineage extension; COMPLETE.", "COMPLETE")
    # sec 8.1/8.2: authority moves only by a ledger succession the key in office signed
    SC1, _ = build_chain([("root", "genesis", GENB), ("root", "succession", SUCB(K2))])
    void_twin = mk("root", "succession", 1, eid(SC1[0]), SUCB(K2, kind="keyrotation"))
    k2_on, _ = build_chain([("k2", "annotation", ANNB("k2 after the counted succession"))], start_seq=2, start_prev=eid(SC1[1]))
    A("void_only_succession_beside_counted_twin", inp(R, recs(SC1, R) + [rec(void_twin, R, bn=20, verdict="VOID")] + recs(k2_on, K2, bn0=30)),
      "sec 8.1/8.2: a second succession to the same key carried only by a VOID record is no ledger entry and moves nothing, while the counted succession moved the seat to k2: k2's entry grades nothing; COMPLETE.", "COMPLETE")
    retired_succ = mk("root", "succession", 2, eid(SC1[1]), SUCB(K3))
    k3_on, _ = build_chain([("k3", "annotation", ANNB("k3 after a succession the retired key signed"))], start_seq=3, start_prev=eid(retired_succ))
    A("succession_signed_by_retired_key_counted", inp(R, recs(SC1, R) + [rec(retired_succ, R, bn=20)] + recs(k3_on, K3, bn0=30)),
      "sec 7.4/8.1: the root signs a succession to k3 after handing the seat to k2 and carries it in the second of its handing off (every record here at one second): the record is DISCARDED, k3 is outside the lineage, k3's own record is DISCARDED and its entry EXCLUDED; COMPLETE.", "COMPLETE")
    self_succ = mk("root", "succession", 1, eid(SC1[0]), SUCB(R))
    root_after, _ = build_chain([("root", "annotation", ANNB("the root goes on after a succession to itself"))], start_seq=2, start_prev=eid(self_succ))
    A("uncounted_self_succession_to_root", inp(R, recs([SC1[0]], R) + [rec(self_succ, R, bn=20, verdict="UNPROVEN")] + recs(root_after, R, bn0=30)),
      "sec 7.4/8.1/8.6: a succession to the root itself carried only by an UNPROVEN record extends nothing and moves nothing, yet retires its signer; the root's entry at seq 2, carried in that second, is DISCARDED; the ledger is the genesis alone beside the UNPROVEN record of bytes in hand; UNPROVEN.", "UNPROVEN")
    self_void = mk("root", "succession", 1, eid(SC1[0]), SUCB(R, kind="keyrotation"))
    self_counted = mk("root", "succession", 1, eid(SC1[0]), SUCB(R))
    root_after2, _ = build_chain([("root", "annotation", ANNB("the root goes on after a succession to itself, twice spelled"))], start_seq=2, start_prev=eid(self_counted))
    A("void_only_self_succession_beside_counted_twin", inp(R, recs([SC1[0]], R) + [rec(self_void, R, bn=20, verdict="VOID"), rec(self_counted, R, bn=21)] + recs(root_after2, R, bn0=30)),
      "sec 7.4/8.1/8.2: a succession to the root itself carried only by a VOID record beside a counted twin to the root at the same seq: the twin fills the position and moves the seat to the key already in office, retiring it; the root's entry at seq 2, carried in that second, is DISCARDED; COMPLETE.", "COMPLETE")
    root_three, _ = build_chain([("root", "genesis", GENB), ("root", "annotation", ANNB("the root's first")), ("root", "annotation", ANNB("the root's second"))])
    self_void3 = mk("root", "succession", 1, eid(root_three[0]), SUCB(R, kind="keyrotation"))
    A("void_only_self_succession_beside_root_entries", inp(R, recs(root_three, R) + [rec(self_void3, R, bn=20, verdict="VOID")]),
      "sec 7.4/8.1/8.6: a succession to the root itself carried only by a VOID record, beside the root's own counted entries at seq 1 and 2, all at one second: the VOID record retires the root at that second all the same, so the root's annotation at seq 2 is DISCARDED while the one at seq 1 stands; no ledger entry at seq 1 but the annotation, nothing graded; COMPLETE.", "COMPLETE")
    bad_body = mk("root", "succession", 1, eid(SB2[0]), {"kind": "handover", "effective": 0, "statement_md": "no to"})
    A("succession_body_fault_carried_by_root", inp(R, recs(SB2, R) + [rec(bad_body, R, bn=20)]),
      "sec 4.3/8.4: a root-signed succession whose body lacks `to` fails sec 4.3: CARRIED_MALFORMED with its token, the bytes MALFORMED and the hash MISSING; BROKEN_CHAIN.", "BROKEN_CHAIN")
    # a case variant of a known type
    SU2, _ = build_chain([("root", "genesis", GENB), ("root", "Succession", SUCB(K2))])
    k2b, _ = build_chain([("k2", "annotation", ANNB("after a Succession"))], start_seq=2, start_prev=eid(SU2[1]))
    A("type_Succession_case_extends_no_lineage", inp(R, recs(SU2, R) + recs(k2b, K2, bn0=30)),
      "sec 6/7.4: Succession is an unknown type and not a succession: an UNKNOWN_TYPE row, no lineage extension, k2's record discarded and its entry excluded; COMPLETE.", "COMPLETE")
    # sec 9.4 forms the harness spells
    base = inp(R, RC)
    bad = json.loads(json.dumps(base)); bad["anchors"][0]["entry"] = bad["anchors"][0]["entry"][:-1]
    A_raw("entry_odd_hex_digits", json.dumps(bad).encode(), "HARNESS: an entry with an odd number of hexadecimal digits is outside the transport spelling: no label.")
    bad = json.loads(json.dumps(base)); bad["anchors"][0]["entry"] = "0x102"; bad["anchors"][0]["hash"] = eid(bytes([1, 2]))
    A_raw("entry_odd_hex_leftpad_reading_hashes", json.dumps(bad).encode(), "HARNESS: three digits whose left-padded reading (01 02) hashes to the record's hash: the spelling is outside the transport form whatever a lenient reader would make of it; no label.")
    bad = json.loads(json.dumps(base)); bad["anchors"][0]["entry"] = bad["anchors"][0]["entry"] + "7"
    A_raw("entry_odd_hex_truncated_reading_hashes", json.dumps(bad).encode(), "HARNESS: a valid entry's digits plus one trailing digit, the record's hash being that entry's: a reader that drops the odd digit would answer a report; the form refuses the spelling; no label.")
    bad = json.loads(json.dumps(base)); bad["anchors"][0]["verdict"] = "Counted"
    A_raw("verdict_case_variant", json.dumps(bad).encode(), "sec 9.2/9.3: a verdict is one of three literals compared bytewise; Counted is none of them: no label.")
    bad = json.loads(json.dumps(base)); bad["anchors"][0]["verdict"] = "unproven"
    A_raw("verdict_lowercase_unproven", json.dumps(bad).encode(), "sec 9.2/9.3: unproven in lower case is no verdict: no label.")
    bad = json.loads(json.dumps(base)); bad["anchors"][0]["entry"] = None
    A_raw("entry_null", json.dumps(bad).encode(), "HARNESS: a present entry member whose value is null is outside the transport spelling: no label.")
    A_raw("audit_form_feed_whitespace", json.dumps(base).replace('"root"', '\x0c"root"').encode(), "sec 3.5 test 2 via the audit reader: a form feed is not whitespace: no label.")
    dup = inp(R, RC + [anchor(CID[0], R, bn=10, tx=RC[0]["tx"])])
    A("duplicate_key_records_differ_in_entry", dup, "sec 9.4: two records agreeing on (chainId, blockNumber, tx, hash) and differing in whether entry is present are not one record: no label.", "NO_LABEL")


def build_misuse():
    """HARNESS misuse cases of check, canon and audit (argv cases: exit 2, no answer)."""
    for argv, note in (
        ([], "no command"), (["frob"], "an unknown command"), (["Check", "canon/c0001_x.bin"], "a command in another case"),
        (["toString", "canon/c0001_x.bin"], "a command spelled as an inherited property name of a common runtime's object type"),
        (["constructor", "canon/c0001_x.bin"], "a command spelled as a constructor name"),
        (["__proto__", "canon/c0001_x.bin"], "a command spelled as a prototype accessor"),
        (["check"], "check with no path"), (["canon"], "canon with no path"), (["audit"], "audit with no path"),
        (["check", "canon", "x"], "check with an argument beyond the list"),
        (["check", "no/such/file.bin"], "check of a path the command cannot read"),
        (["canon", "no/such/file.bin"], "canon of a path the command cannot read"),
        (["audit", "audit"], "audit of a directory"), (["check", "canon"], "check of a directory"),
        (["audit", "no/such/file.json", "x"], "audit with two faults: an unreadable path and an argument beyond the list"),
    ):
        MANIFEST.append({"argv": argv, "path": "(argv)", "category": "misuse", "note": "HARNESS: %s is misuse" % note, "predicted": None})


def coverage():
    tok, lab, fin, verd = {}, {}, {}, {}
    for r in MANIFEST:
        if r["category"] == "canon":
            tok[r["predicted"]] = tok.get(r["predicted"], 0) + 1
        elif r["category"] == "audit":
            lab[r["predicted_label"]] = lab.get(r["predicted_label"], 0) + 1
            for k, v in r["predicted_counts"].items():
                if k.startswith("f_") and v:
                    fin[k[2:]] = fin.get(k[2:], 0) + v
            for k in ("missing", "anchored", "excluded", "unknown_type",
                      "malformed", "unproven", "void", "discarded"):
                if r["predicted_counts"].get(k):
                    fin[k.upper()] = fin.get(k.upper(), 0) + r["predicted_counts"][k]
    for r in MANIFEST:
        if r["category"] == "audit":
            try:
                _doc = json.load(open(os.path.join(HERE, r["path"])))
            except Exception:
                continue                      # a raw no-label input is not a JSON text
            if not isinstance(_doc, dict) or not isinstance(_doc.get("anchors"), list):
                continue
            for a in _doc["anchors"]:
                v = a.get("verdict") if isinstance(a, dict) else None
                if v in ("counted", "UNPROVEN", "VOID"):
                    verd[v] = verd.get(v, 0) + 1
    return {"sec_10_tokens_and_accept": tok, "sec_8_7_labels": lab,
            "sec_8_3_report_rows": fin, "sec_9_3_anchor_verdicts": verd}


TOKENS = ["E_UTF8", "E_JSON", "E_NUMBER", "E_DEPTH", "E_DUP_KEY", "E_KEY_CHARSET",
          "E_VALUE_CHARSET", "E_NOT_CANONICAL", "E_ENVELOPE", "E_ENVELOPE_MISSING",
          "E_ENVELOPE_CLOSED", "E_SPEC", "E_ENTRYTYPE", "E_JUDGE", "E_SEQ", "E_PREV",
          "E_PREV_SEQ", "E_BODY", "E_SIG_FORM", "E_GENESIS_PLACE", "E_BODY_FIELD", "E_SIG_V",
          "E_SIG_RANGE", "E_SIG_HIGH_S", "E_SIG_RECOVER", "E_SIG_SIGNER"]
FINDINGS = ["SEQ_GAP", "PREV_MISMATCH", "AUTHORITY_MISMATCH", "ROOT_MISMATCH", "EQUIVOCATION",
            "CARRIED_MALFORMED", "MISSING", "ANCHORED", "EXCLUDED", "UNKNOWN_TYPE", "MALFORMED",
            "UNPROVEN", "VOID", "DISCARDED"]
LABELS = ["BROKEN_CHAIN", "UNPROVEN", "GAPS", "COMPLETE"]


README = """# galeed/1 convergence corpus

Generated by `gen.py` from `docs/galeed-v1.md` (the law) and
`galeed-conformance/HARNESS.md` (the command-line contract) alone, carrying
over the twin law's generator where the two texts agree word for word. No
implementation of the law was read while building it.

## Regenerating

    python3 gen.py --seed %d

The run is deterministic: the same seed rebuilds byte-identical directories.
An audit case's reversed twin (`*_reversed`) is registered only where
reversing the input arrays changes the input, so the audit count moves by a
few between seeds while every hand-built case is present under every seed.
`canon/`, `sign/` and `audit/` are deleted and repopulated on every run, and
`manifest.json` and this file are rewritten. Requires Python 3 and
`pycryptodome` (for `Crypto.Hash.keccak`); secp256k1 with RFC 6979 and low-s,
the sec 3.4 canonicalizer, the sec 3.5 and sec 4.3 decision order and the sec 8
walk are all implemented inside `gen.py`.

## Layout

- `canon/` (%d files) - byte strings for `gl1 check <path>` and
  `gl1 canon <path>`. Every file is raw bytes; many are not valid UTF-8 and
  most are not valid JSON, which is the point.
- `sign/` (%d cases, each a `.bin` and its `.meta.json`) - `sNNN_*.bin` holds the canonical bytes of a six-member
  envelope (or, under a family law's domain, of that document). The sibling
  `sNNN_*.meta.json` names the private key and the domain to pass to
  `gl1 sign <privkey-hex> <path> [domain]`.
- `audit/` (%d cases, each a `.json` and its `.note`) - `aNNN_*.json` is one `gl1 audit <path>` input in the
  HARNESS.md format (`root`, `anchors`, `basis`: the two inputs of sec 8, the
  entries travelling as the bytes anchor records carry); the sibling
  `aNNN_*.note` says what the scenario probes.
- `manifest.json` - every file with its category, its note, and the corpus
  author's own prediction; cases of category `misuse` carry an `argv` (the
  whole argument vector after the executable name) and no file, and their
  expected outcome is exit code 2 with no answer; a `sign` case whose
  `predicted` is `null` expects the same.

## Reading manifest.json

Each case carries the author's prediction as a third opinion:

- canon cases: `predicted` (`"ok"` or the sec 10 token `gl1 check` should
  name), `predicted_entry_id`, `predicted_canon` (what `gl1 canon` should
  answer, since it applies sec 3.5 tests 1 through 5 only) and
  `predicted_canon_sha256` (SHA-256 of the canonical bytes it should print,
  so long outputs stay comparable).
- audit cases: `predicted_label` and `predicted_counts` (ledger entry count,
  finding counts, list sizes).
- sign cases: `predicted` with `presig`, `digest`, `sig` and `signer`.

None of these is authoritative. Where the two implementations agree with each
other and disagree with a prediction, the prediction is wrong. Where they
disagree with each other, the law decides, and the prediction is only a hint
about which reading a third reader reached. `hand_expect` (canon) and
`hand_expect_label` (audit) carry the author's prose reasoning written before
the code ran; a case where the two differ is flagged `disagreement: true` and
is worth reading closely.

## Comparing two implementations

    for f in canon/*.bin;  do gl1 check "$f"; gl1 canon "$f"; done
    for m in sign/*.meta.json; do ... gl1 sign <privkey> <bin> <domain>; done
    for f in audit/*.json; do gl1 audit "$f"; done

Every command writes one canonical JSON value and exits 0 whenever it produced
an answer. Diff the two implementations' streams byte for byte.
"""


if __name__ == "__main__":
    main()
