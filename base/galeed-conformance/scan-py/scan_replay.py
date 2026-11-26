#!/usr/bin/env python3
"""galeed/1 chain-facing scan (docs/galeed-v1.md section 9), replayed against a
recorded JSON-RPC transcript in place of a live chain.

    python3 scan_replay.py <fixture.json>

Prints the canonical-JSON (section 3.4) audit-input fragment

    {"anchors":[...],"basis":<basis>}

and exits 0 (a record carries its `entry` where the log's data carried the
bytes, section 9.1); prints {"ok":false,"reason":"NO_LABEL"} and exits 0
when the declared basis is not a galeed/1 basis (section 9.4); exits 2
when the scan fails (section 9.4): the recording cannot answer a call the
scan must make (other than eth_getCode for a sender's boundaries, whose
failure is the section 9.3 verdict UNPROVEN), a chain answers eth_getLogs
with no array, a declared range reaches past the chain's head, or a
transaction's envelope is of a type the scan cannot decode.

Written from the text of docs/galeed-v1.md alone: one anchor form, the
log of section 9.1 whose data carries the entry.
"""

import json
import sys

MAX_INT = (1 << 53) - 1


class Loud(Exception):
    """A failure the scan cannot absorb: exit 2."""


# ============================ canonical JSON (3.4) ============================

_SHORT = {0x08: b"\\b", 0x09: b"\\t", 0x0A: b"\\n", 0x0C: b"\\f", 0x0D: b"\\r"}


def _cstring(s):
    out = bytearray(b'"')
    for ch in s:
        c = ord(ch)
        if ch == '"':
            out += b'\\"'
        elif ch == "\\":
            out += b"\\\\"
        elif c in _SHORT:
            out += _SHORT[c]
        elif c < 0x20:
            out += ("\\u%04x" % c).encode("ascii")
        else:
            out += ch.encode("utf-8")
    out += b'"'
    return bytes(out)


def _emit(v, out):
    if v is None:
        out += b"null"
    elif v is True:
        out += b"true"
    elif v is False:
        out += b"false"
    elif isinstance(v, int):
        if v < 0 or v > MAX_INT:
            raise Loud("integer outside the 3.1 universe: %r" % (v,))
        out += str(v).encode("ascii")
    elif isinstance(v, str):
        out += _cstring(v)
    elif isinstance(v, (list, tuple)):
        out += b"["
        for i, e in enumerate(v):
            if i:
                out += b","
            _emit(e, out)
        out += b"]"
    elif isinstance(v, dict):
        out += b"{"
        for i, k in enumerate(sorted(v.keys(), key=lambda x: x.encode("utf-8"))):
            if i:
                out += b","
            out += _cstring(k)
            out += b":"
            _emit(v[k], out)
        out += b"}"
    else:
        raise Loud("value outside the 3.1 universe: %r" % (v,))


def canon(v):
    out = bytearray()
    _emit(v, out)
    return bytes(out)


# ================================= keccak-256 =================================

try:
    from Crypto.Hash import keccak as _pycrypto_keccak

    def keccak256(data):
        h = _pycrypto_keccak.new(digest_bits=256)
        h.update(data)
        return h.digest()

except Exception:  # pragma: no cover - fallback, no third-party code needed
    _MASK = (1 << 64) - 1
    _RC = [
        0x0000000000000001, 0x0000000000008082, 0x800000000000808A,
        0x8000000080008000, 0x000000000000808B, 0x0000000080000001,
        0x8000000080008081, 0x8000000000008009, 0x000000000000008A,
        0x0000000000000088, 0x0000000080008009, 0x000000008000000A,
        0x000000008000808B, 0x800000000000008B, 0x8000000000008089,
        0x8000000000008003, 0x8000000000008002, 0x8000000000000080,
        0x000000000000800A, 0x800000008000000A, 0x8000000080008081,
        0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
    ]
    _R = [
        [0, 36, 3, 41, 18],
        [1, 44, 10, 45, 2],
        [62, 6, 43, 15, 61],
        [28, 55, 25, 21, 56],
        [27, 20, 39, 8, 14],
    ]

    def _rol(x, n):
        n %= 64
        if n == 0:
            return x
        return ((x << n) | (x >> (64 - n))) & _MASK

    def _keccak_f(A):
        for rnd in range(24):
            C = [A[x][0] ^ A[x][1] ^ A[x][2] ^ A[x][3] ^ A[x][4] for x in range(5)]
            D = [C[(x - 1) % 5] ^ _rol(C[(x + 1) % 5], 1) for x in range(5)]
            for x in range(5):
                for y in range(5):
                    A[x][y] ^= D[x]
            B = [[0] * 5 for _ in range(5)]
            for x in range(5):
                for y in range(5):
                    B[y][(2 * x + 3 * y) % 5] = _rol(A[x][y], _R[x][y])
            for x in range(5):
                for y in range(5):
                    A[x][y] = B[x][y] ^ ((~B[(x + 1) % 5][y] & _MASK) & B[(x + 2) % 5][y])
            A[0][0] ^= _RC[rnd]
        return A

    def keccak256(data):
        rate = 136
        pad = bytearray(data)
        pad.append(0x01)
        while len(pad) % rate != 0:
            pad.append(0x00)
        pad[-1] |= 0x80
        A = [[0] * 5 for _ in range(5)]
        for off in range(0, len(pad), rate):
            block = pad[off:off + rate]
            for i in range(rate // 8):
                lane = int.from_bytes(block[i * 8:i * 8 + 8], "little")
                A[i % 5][i // 5] ^= lane
            _keccak_f(A)
        out = bytearray()
        for i in range(4):
            out += A[i % 5][i // 5].to_bytes(8, "little")
        return bytes(out)


# the one form of 9.1: three topics under this topic 0, the entry bytes
# ABI-encoded in the data (or anything else, which carries nothing)
ANCHORED_TOPIC0 = "0x" + keccak256(b"Anchored(address,bytes32,bytes)").hex()

try:
    import hashlib

    def sha256(data):
        return hashlib.sha256(data).digest()
except Exception:  # pragma: no cover
    sha256 = None


def carried_entry(data, h):
    """9.1: the entry bytes the log's data carries, or None where the data is
    not the ABI encoding of one `bytes` value whose sha256 is the anchored
    hash (the log is then an anchor that carries nothing)."""
    if len(data) < 64 or len(data) % 32 != 0:
        return None
    if int.from_bytes(data[0:32], "big") != 32:
        return None
    n = int.from_bytes(data[32:64], "big")
    padded = 64 + ((n + 31) // 32) * 32
    if len(data) != padded:
        return None
    body = data[64:64 + n]
    if any(b != 0 for b in data[64 + n:]):
        return None
    if sha256(body) != hexbytes(h):
        return None
    return body


# ================================ hex helpers =================================

_HEXDIGITS = set("0123456789abcdef")


def is_hexn(s, nbytes):
    """Section 1: '0x' then exactly 2*nbytes lowercase hex digits."""
    if not isinstance(s, str) or isinstance(s, bool):
        return False
    if len(s) != 2 + 2 * nbytes or not s.startswith("0x"):
        return False
    return all(c in _HEXDIGITS for c in s[2:])


def is_int(v):
    return isinstance(v, int) and not isinstance(v, bool) and 0 <= v <= MAX_INT


def lc(s):
    """Normalize a hex string the chain answered with: lowercase."""
    if not isinstance(s, str):
        raise Loud("expected a hex string from the chain, got %r" % (s,))
    return s.lower()


def hexbytes(s):
    """Decode an '0x'-prefixed byte string as the chain spells it."""
    if not isinstance(s, str):
        raise Loud("expected hex bytes from the chain, got %r" % (s,))
    t = s[2:] if s[:2].lower() == "0x" else s
    if len(t) % 2:
        raise Loud("odd-length hex from the chain: %r" % (s,))
    try:
        return bytes.fromhex(t)
    except ValueError:
        raise Loud("non-hex bytes from the chain: %r" % (s,))


def qty(s):
    """Read a hex quantity the chain answered with as an integer."""
    if isinstance(s, int) and not isinstance(s, bool):
        return s
    if not isinstance(s, str):
        raise Loud("expected a hex quantity from the chain, got %r" % (s,))
    try:
        return int(s, 16)
    except ValueError:
        raise Loud("bad hex quantity from the chain: %r" % (s,))


def hexq(n):
    """Write a block number as a JSON-RPC hex quantity: minimal, lowercase."""
    return hex(n)


# ============================== recorded transport ============================

class RpcError(Exception):
    def __init__(self, code, message):
        Exception.__init__(self, "%s (%s)" % (message, code))
        self.code = code
        self.message = message


class Missing(Exception):
    def __init__(self, chain_id, method, params):
        Exception.__init__(
            self, "no recorded exchange: chain %s %s %s" % (chain_id, method, json.dumps(params))
        )


def jeq(a, b):
    """JSON-value equality; object key order irrelevant."""
    if isinstance(a, bool) or isinstance(b, bool):
        return isinstance(a, bool) and isinstance(b, bool) and a == b
    if a is None or b is None:
        return a is None and b is None
    if isinstance(a, str) or isinstance(b, str):
        return isinstance(a, str) and isinstance(b, str) and a == b
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return a == b
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(jeq(x, y) for x, y in zip(a, b))
    if isinstance(a, dict) and isinstance(b, dict):
        return set(a.keys()) == set(b.keys()) and all(jeq(a[k], b[k]) for k in a)
    return False


class Recording(object):
    def __init__(self, rpc):
        if rpc is None:
            rpc = {}
        if not isinstance(rpc, dict):
            raise Loud("fixture member 'rpc' is not an object")
        self.rpc = rpc

    def call(self, chain_id, method, params):
        entries = self.rpc.get(str(chain_id))
        if not isinstance(entries, list):
            raise Missing(chain_id, method, params)
        for ex in entries:
            if not isinstance(ex, dict):
                continue
            if ex.get("method") != method:
                continue
            if not jeq(ex.get("params"), params):
                continue
            if "error" in ex and ex["error"] is not None:
                err = ex["error"]
                raise RpcError(err.get("code"), err.get("message"))
            return ex.get("result")
        raise Missing(chain_id, method, params)


# =========================== sender recovery (9.1) ===========================
#
# Section 9.1: the transaction's sender is the address a verifier recovers from
# the transaction's own secp256k1 signature.  The scanner therefore never
# reads the node's `from` field: it re-encodes the transaction the node
# answered with, checks that the signed bytes hash to the hash it asked for,
# and recovers the signer from (r, s, parity) over the signing payload.  A
# transaction that carries no signature this grammar reads (a system or
# deposit transaction, a typed transaction of a kind this scanner does not
# encode, a legacy transaction whose signature names no chain) has no sender
# and anchors nothing.

_P = 2 ** 256 - 2 ** 32 - 977
_N = 0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141
_GX = 0x79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798
_GY = 0x483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8
_INF = (0, 0, 0)
_G = (_GX, _GY, 1)


def _jdouble(p):
    X, Y, Z = p
    if Z == 0 or Y == 0:
        return _INF
    S = (4 * X * Y * Y) % _P
    M = (3 * X * X) % _P
    X3 = (M * M - 2 * S) % _P
    Y3 = (M * (S - X3) - 8 * Y * Y * Y * Y) % _P
    Z3 = (2 * Y * Z) % _P
    return (X3, Y3, Z3)


def _jadd(p, q):
    if p[2] == 0:
        return q
    if q[2] == 0:
        return p
    X1, Y1, Z1 = p
    X2, Y2, Z2 = q
    Z1Z1 = Z1 * Z1 % _P
    Z2Z2 = Z2 * Z2 % _P
    U1 = X1 * Z2Z2 % _P
    U2 = X2 * Z1Z1 % _P
    S1 = Y1 * Z2Z2 * Z2 % _P
    S2 = Y2 * Z1Z1 * Z1 % _P
    if U1 == U2:
        if S1 != S2:
            return _INF
        return _jdouble(p)
    H = (U2 - U1) % _P
    R = (S2 - S1) % _P
    HH = H * H % _P
    HHH = HH * H % _P
    V = U1 * HH % _P
    X3 = (R * R - HHH - 2 * V) % _P
    Y3 = (R * (V - X3) - S1 * HHH) % _P
    Z3 = H * Z1 * Z2 % _P
    return (X3, Y3, Z3)


def _jmul(k, p):
    k %= _N
    r = _INF
    while k:
        if k & 1:
            r = _jadd(r, p)
        p = _jdouble(p)
        k >>= 1
    return r


def _affine(p):
    X, Y, Z = p
    if Z == 0:
        return None
    zi = pow(Z, _P - 2, _P)
    zi2 = zi * zi % _P
    return (X * zi2 % _P, Y * zi2 * zi % _P)


def recover_address(digest, r, s, parity):
    """The 20-byte address whose key signed `digest` with (r, s, parity),
    or None when no key did."""
    if not (1 <= r < _N and 1 <= s < _N) or parity not in (0, 1):
        return None
    x = r
    y2 = (pow(x, 3, _P) + 7) % _P
    y = pow(y2, (_P + 1) // 4, _P)
    if y * y % _P != y2:
        return None
    if (y & 1) != parity:
        y = _P - y
    z = int.from_bytes(digest, "big") % _N
    ri = pow(r, _N - 2, _N)
    q = _affine(_jadd(_jmul((-z * ri) % _N, _G), _jmul(s * ri % _N, (x, y, 1))))
    if q is None:
        return None
    return keccak256(q[0].to_bytes(32, "big") + q[1].to_bytes(32, "big"))[-20:]


def _rlp_len(n, base):
    if n < 56:
        return bytes([base + n])
    b = n.to_bytes((n.bit_length() + 7) // 8, "big")
    return bytes([base + 55 + len(b)]) + b


def rlp(item):
    """RLP of a byte string or a (nested) list of them."""
    if isinstance(item, bytes):
        if len(item) == 1 and item[0] < 0x80:
            return item
        return _rlp_len(len(item), 0x80) + item
    body = b"".join(rlp(x) for x in item)
    return _rlp_len(len(body), 0xc0) + body


def _rq(v):
    """A JSON-RPC quantity as RLP's minimal big-endian bytes."""
    n = qty(v)
    return b"" if n == 0 else n.to_bytes((n.bit_length() + 7) // 8, "big")


def _rb(v):
    """A JSON-RPC byte field; null (a creation's `to`) is the empty string."""
    return b"" if v is None else hexbytes(v)


def _access_list(al):
    out = []
    for e in (al or []):
        if not isinstance(e, dict):
            raise Loud("access list entry is not an object")
        out.append([hexbytes(e.get("address")),
                    [hexbytes(k) for k in (e.get("storageKeys") or [])]])
    return out


def _authorization_list(al):
    out = []
    for e in (al or []):
        if not isinstance(e, dict):
            raise Loud("authorization list entry is not an object")
        parity = e.get("yParity", e.get("v"))
        out.append([_rq(e.get("chainId")), hexbytes(e.get("address")), _rq(e.get("nonce")),
                    _rq(parity), _rq(e.get("r")), _rq(e.get("s"))])
    return out


def tx_sender(txo, tx_hash, chain_id):
    """Section 9.1's sender of a transaction the chain answered with: the
    hex20 recovered from its signature, or None when the transaction carries
    no signature this grammar reads or its envelope names another chain than
    the one the log sits on.  A signed envelope of a type this scan cannot
    decode is a transaction the verifier cannot tell the sender of: the scan
    is loud (9.1, 9.4).  The signed encoding must hash to the hash the
    transaction was asked for by; a node whose answer does not is answering
    about some other bytes, and the scan is loud."""
    if not isinstance(txo, dict):
        raise Loud("transaction %s is not an object" % tx_hash)
    ttype = qty(txo.get("type", "0x0"))
    r = qty(txo.get("r", "0x0"))
    s = qty(txo.get("s", "0x0"))
    if r == 0 and s == 0:
        return None
    if ttype == 0:
        v = qty(txo.get("v", "0x0"))
        chain = (v - 35) // 2 if v >= 35 else None
        parity = (v - 35) % 2 if v >= 35 else None
        body = [_rq(txo.get("nonce")), _rq(txo.get("gasPrice")), _rq(txo.get("gas")),
                _rb(txo.get("to")), _rq(txo.get("value")), _rb(txo.get("input", "0x"))]
        preimage = rlp(body + [_rq(chain), b"", b""]) if chain is not None else None
        raw = rlp(body + [_rq(v), _rq(r), _rq(s)])
    elif ttype in (1, 2, 3, 4):
        parity = qty(txo.get("yParity", txo.get("v", "0x0")))
        chain = qty(txo.get("chainId", "0x0"))
        if ttype == 1:
            fields = [_rq(txo.get("chainId")), _rq(txo.get("nonce")), _rq(txo.get("gasPrice")),
                      _rq(txo.get("gas")), _rb(txo.get("to")), _rq(txo.get("value")),
                      _rb(txo.get("input", "0x")), _access_list(txo.get("accessList"))]
        else:
            fields = [_rq(txo.get("chainId")), _rq(txo.get("nonce")),
                      _rq(txo.get("maxPriorityFeePerGas")), _rq(txo.get("maxFeePerGas")),
                      _rq(txo.get("gas")), _rb(txo.get("to")), _rq(txo.get("value")),
                      _rb(txo.get("input", "0x")), _access_list(txo.get("accessList"))]
            if ttype == 3:
                fields += [_rq(txo.get("maxFeePerBlobGas")),
                           [hexbytes(h) for h in (txo.get("blobVersionedHashes") or [])]]
            elif ttype == 4:
                fields += [_authorization_list(txo.get("authorizationList"))]
        preimage = bytes([ttype]) + rlp(fields)
        raw = bytes([ttype]) + rlp(fields + [_rq(parity), _rq(r), _rq(s)])
    else:
        raise Loud("transaction %s: a signed envelope of type %d this scan cannot decode; the basis is not scanned (9.1, 9.4)"
                   % (tx_hash, ttype))
    # the envelope the node answered with must be the transaction asked for;
    # every reading of its fields comes after this test
    if keccak256(raw) != hexbytes(tx_hash):
        raise Loud("transaction %s: the chain's answer does not hash to it" % tx_hash)
    if ttype == 0 and chain is None:
        # v of 27 or 28 is a signature that names no chain (9.1); anything
        # below 35 is no EIP-155 signature
        return None
    if ttype != 0 and parity not in (0, 1):
        return None
    if chain != chain_id:
        # 9.1: the envelope names another chain than the one the log sits on
        return None
    addr = recover_address(keccak256(preimage), r, s, parity)
    if addr is None:
        return None
    return "0x" + addr.hex()


# ============================= basis validation (9.4) =========================

def valid_basis(b):
    """Section 9.4, class [C]: exactly the member chains, each object
    arity-closed, every member of its named form; `chains` ordered by
    (chainId, fromBlock) strictly ascending with fromBlock <= toBlock, two
    objects of one chainId neither overlapping nor adjacent with equal
    registries and senders; `registries` (addresses first, then class items)
    and `senders` bytewise ascending without repeats."""
    if not isinstance(b, dict):
        return False
    if set(b.keys()) != {"chains"}:
        return False

    chains = b["chains"]
    if not isinstance(chains, list):
        return False
    prev = None
    for c in chains:
        if not isinstance(c, dict):
            return False
        if set(c.keys()) != {"chainId", "fromBlock", "toBlock", "registries", "senders"}:
            return False
        if not (is_int(c["chainId"]) and is_int(c["fromBlock"]) and is_int(c["toBlock"])):
            return False
        if c["fromBlock"] > c["toBlock"]:
            return False
        v = c["senders"]
        if not isinstance(v, list):
            return False
        for a in v:
            if not is_hexn(a, 20):
                return False
        if any(v[i] >= v[i + 1] for i in range(len(v) - 1)):
            return False
        # 9.4: registries lists hex20 addresses first, bytewise ascending, then
        # class items {codeHash: hex32}, ascending by codeHash, no repeats
        rg = c["registries"]
        if not isinstance(rg, list):
            return False
        i = 0
        while i < len(rg) and isinstance(rg[i], str):
            if not is_hexn(rg[i], 20):
                return False
            if i > 0 and rg[i - 1] >= rg[i]:
                return False
            i += 1
        j = i
        while j < len(rg):
            x = rg[j]
            if not isinstance(x, dict) or set(x.keys()) != {"codeHash"} or not is_hexn(x["codeHash"], 32):
                return False
            if j > i and rg[j - 1]["codeHash"] >= x["codeHash"]:
                return False
            j += 1
        if prev is not None:
            if (prev["chainId"], prev["fromBlock"]) >= (c["chainId"], c["fromBlock"]):
                return False
            if prev["chainId"] == c["chainId"]:
                if prev["toBlock"] >= c["fromBlock"]:
                    return False
                if (prev["toBlock"] + 1 == c["fromBlock"]
                        and prev["registries"] == c["registries"]
                        and prev["senders"] == c["senders"]):
                    return False
        prev = c
    return True


# =================================== scan =====================================

def word_offsets(n):
    """Offsets a 32-byte window may begin at inside n bytes of calldata: a
    multiple of 32, or 4 plus a multiple of 32, lying wholly within (9.1)."""
    offs = set()
    o = 0
    while o + 32 <= n:
        offs.add(o)
        o += 32
    o = 4
    while o + 32 <= n:
        offs.add(o)
        o += 32
    return sorted(offs)


def carried_in_calldata(calldata, needle):
    return any(calldata[o:o + 32] == needle for o in word_offsets(len(calldata)))


class Scanner(object):
    def __init__(self, basis, rec):
        self.basis = basis
        self.rec = rec
        self.tx_cache = {}
        self.receipt_cache = {}
        self.block_cache = {}
        self.code_cache = {}
        self.head_cache = {}


    # ------------------------------- transport -------------------------------

    def get_tx(self, chain_id, tx):
        key = (chain_id, tx)
        if key not in self.tx_cache:
            self.tx_cache[key] = self.rec.call(chain_id, "eth_getTransactionByHash", [tx])
        return self.tx_cache[key]

    def get_receipt(self, chain_id, tx):
        key = (chain_id, tx)
        if key not in self.receipt_cache:
            self.receipt_cache[key] = self.rec.call(chain_id, "eth_getTransactionReceipt", [tx])
        return self.receipt_cache[key]

    def head(self, chain_id):
        """The chain's head as it answered eth_blockNumber (9.4: a range past
        the head is a failure of the scan)."""
        if chain_id not in self.head_cache:
            self.head_cache[chain_id] = qty(self.rec.call(chain_id, "eth_blockNumber", []))
        return self.head_cache[chain_id]

    def block_timestamp(self, chain_id, number):
        key = (chain_id, number)
        if key not in self.block_cache:
            blk = self.rec.call(chain_id, "eth_getBlockByNumber", [hexq(number), False])
            if not isinstance(blk, dict) or "timestamp" not in blk:
                raise Loud("no block header for chain %s block %d" % (chain_id, number))
            self.block_cache[key] = qty(blk["timestamp"])
        return self.block_cache[key]

    def has_code(self, chain_id, address, number):
        """Tri-state: True (code), False (no code), None (could not consult)."""
        key = (chain_id, address, number)
        if key not in self.code_cache:
            try:
                code = self.rec.call(chain_id, "eth_getCode", [address, hexq(number)])
            except (Missing, RpcError):
                self.code_cache[key] = None
            else:
                if code is None:
                    self.code_cache[key] = None
                else:
                    self.code_cache[key] = len(hexbytes(code)) > 0
        return self.code_cache[key]

    # ------------------------------ 9.3 verdict ------------------------------

    def verdict(self, chain_id, sender, number):
        if number == 0:
            states = [self.has_code(chain_id, sender, 0)]
        else:
            states = [
                self.has_code(chain_id, sender, number - 1),
                self.has_code(chain_id, sender, number),
            ]
        if any(s is None for s in states):
            return "UNPROVEN"
        if any(s for s in states):
            return "VOID"
        return "counted"

    # ------------------------ emitter admission (9.1, 9.4) -------------------


    def code_hash(self, chain_id, address, number):
        """keccak256 of the code an account holds in the state at the end of
        block `number`, or None where the recording could not answer."""
        key = ("codehash", chain_id, address, number)
        if key not in self.code_cache:
            try:
                code = self.rec.call(chain_id, "eth_getCode", [address, hexq(number)])
            except (Missing, RpcError):
                self.code_cache[key] = None
            else:
                self.code_cache[key] = None if code is None else "0x" + keccak256(hexbytes(code)).hex()
        return self.code_cache[key]

    def scan_registries(self, out):
        for c in self.basis["chains"]:
            chain_id = c["chainId"]
            if c["toBlock"] > self.head(chain_id):
                raise Loud("chain %s: the declared range reaches past the head %d; the basis is not scanned (9.4)"
                           % (chain_id, self.head(chain_id)))
            addresses = [a for a in c["registries"] if isinstance(a, str)]
            classes = set(a["codeHash"] for a in c["registries"] if not isinstance(a, str))
            if not addresses and not classes:
                # No admitted emitter on this chain, so no log of the form
                # can qualify; the scan surface is empty.
                continue
            declared = set(addresses)
            senders = set(c["senders"])
            start = c["fromBlock"]
            while start <= c["toBlock"]:
                end = min(start + 1999, c["toBlock"])
                if classes:
                    # class items: no address filter, the lineage keys as the
                    # sender topic, the form's topic 0; the emitter's code is
                    # checked log by log
                    logs = self.rec.call(chain_id, "eth_getLogs", [{
                        "fromBlock": hexq(start),
                        "toBlock": hexq(end),
                        "topics": [[ANCHORED_TOPIC0],
                                   ["0x" + "00" * 12 + s[2:] for s in sorted(senders)]],
                    }])
                else:
                    logs = self.rec.call(chain_id, "eth_getLogs", [{
                        "fromBlock": hexq(start),
                        "toBlock": hexq(end),
                        "address": addresses,
                        "topics": [[ANCHORED_TOPIC0]],
                    }])
                if not isinstance(logs, list):
                    raise Loud("eth_getLogs did not answer with an array on chain %s" % chain_id)
                for log in logs:
                    self.consider_log(chain_id, log, declared, classes, senders,
                                      c["fromBlock"], c["toBlock"], out)
                start = end + 1

    def consider_log(self, chain_id, log, declared, classes, senders, lo, hi, out):
        if not isinstance(log, dict):
            raise Loud("eth_getLogs answered with a non-object log on chain %s" % chain_id)
        topics = log.get("topics")
        if not isinstance(topics, list) or len(topics) != 3:
            return
        topics = [lc(t) for t in topics]
        if topics[0] != ANCHORED_TOPIC0:
            return
        data = hexbytes(log.get("data", "0x"))
        number = qty(log.get("blockNumber"))
        emitter = lc(log.get("address", ""))
        if emitter not in declared and not classes:
            return
        if number < lo or number > hi:
            return
        tx = lc(log.get("transactionHash"))

        txo = self.get_tx(chain_id, tx)
        if not isinstance(txo, dict):
            raise Loud("no transaction %s on chain %s" % (tx, chain_id))
        if txo.get("to") is None:
            return  # a creation carries init code and no calldata (9.1)
        sender = tx_sender(txo, tx, chain_id)
        if sender is None or sender not in senders:
            return
        if topics[1] != "0x" + "00" * 12 + sender[2:]:
            return

        rcpt = self.get_receipt(chain_id, tx)
        if not isinstance(rcpt, dict):
            raise Loud("no receipt for %s on chain %s" % (tx, chain_id))
        status = rcpt.get("status")
        if status is None or qty(status) != 1:
            return  # 9.1: a receipt carrying no status 1 anchors nothing

        calldata = hexbytes(txo.get("input", "0x"))
        h = topics[2]
        if not carried_in_calldata(calldata, hexbytes(h)):
            return

        if emitter not in declared:
            # 9.1: an emitter admitted by class, on the code hash of its account
            # in the state at the end of the log's block; read last, once every
            # test the log and its transaction decide has passed
            ch = self.code_hash(chain_id, emitter, number)
            if ch is None:
                raise Loud("cannot read the code of emitter %s at block %d on chain %s"
                           % (emitter, number, chain_id))
            if ch not in classes:
                return


        # 9.1: the data is read once the log is an anchor; bytes that decode
        # to the hash are the record's entry, anything else carries nothing
        self.record(out, chain_id, number, tx, sender, h, carried_entry(data, h))

    # --------------------------------- records -------------------------------

    def record(self, out, chain_id, number, tx, sender, h, entry=None):
        key = (chain_id, number, tx, h)
        if key in out:
            # at most one record per (chainId, blockNumber, tx, hash) (9.4); the
            # record carries an entry where any of the logs keyed to it did
            if entry is not None and "entry" not in out[key]:
                out[key]["entry"] = "0x" + entry.hex()
            return
        out[key] = {
            "chainId": chain_id,
            "blockNumber": number,
            "blockTimestamp": self.block_timestamp(chain_id, number),
            "tx": tx,
            "sender": sender,
            "hash": h,
            "verdict": self.verdict(chain_id, sender, number),
        }
        if entry is not None:
            out[key]["entry"] = "0x" + entry.hex()


# =================================== main =====================================

def run(fixture):
    basis = fixture.get("basis")
    if not valid_basis(basis):
        return {"ok": False, "reason": "NO_LABEL"}

    rec = Recording(fixture.get("rpc"))
    sc = Scanner(basis, rec)

    anchors = {}
    sc.scan_registries(anchors)

    return {
        "anchors": [anchors[k] for k in sorted(anchors.keys())],
        "basis": basis,
    }


def try_run(fixture):
    """(True, answer) where the scan answers; (False, message) where it fails
    (exit 2): what a fixture whose `expected` is null asserts."""
    try:
        return True, run(fixture)
    except (Missing, RpcError, Loud) as exc:
        return False, str(exc)


def main(argv):
    if len(argv) != 2:
        sys.stderr.write("usage: scan_replay.py <fixture.json>\n")
        return 2
    try:
        with open(argv[1], "rb") as fh:
            fixture = json.loads(fh.read().decode("utf-8"))
    except Exception as exc:
        sys.stderr.write("cannot read fixture: %s\n" % exc)
        return 2
    if not isinstance(fixture, dict):
        sys.stderr.write("fixture is not a JSON object\n")
        return 2
    try:
        answer = run(fixture)
    except Missing as exc:
        sys.stderr.write("%s\n" % exc)
        return 2
    except RpcError as exc:
        sys.stderr.write("recorded RPC error: %s\n" % exc)
        return 2
    except Loud as exc:
        sys.stderr.write("%s\n" % exc)
        return 2
    sys.stdout.buffer.write(canon(answer))
    sys.stdout.buffer.flush()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
