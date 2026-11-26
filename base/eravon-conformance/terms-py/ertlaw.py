"""eravon.terms/1 sections 2 through 6: the document, the quote, the offer
and the order, the signatures, the fee.

Built on ertcanon (the forms and canonical form of galeed/1 s1 and s3) and
ertcrypto (keccak256, secp256k1, typed data, CREATE2), this core's own files
(s10.4); every predicate below is the law's own.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from ertcanon import Fault, Obj, accept, is_hex, is_int, split_sig       # noqa: E402
from ertcrypto import N, HALF_N, address_of, recover, sha256, keccak256  # noqa: E402
import ertcrypto as C                                                    # noqa: E402

VERSION = "eravon.terms/1"
OPEN_FENCE = b"```eravon.terms/1"
CLOSE_FENCE = b"```"
MAX_U32 = (1 << 32) - 1
MAX_U256 = (1 << 256) - 1
MAX_INT = (1 << 53) - 1
ZERO20 = "0x" + "00" * 20
ZERO32 = "0x" + "00" * 32
_DIGITS = frozenset("0123456789")


class DocFault(Exception):
    """One token of s7, with the values the harness carries beside it."""

    def __init__(self, token, index=None, inner=None, field=None, key=None):
        Exception.__init__(self, token)
        self.token = token
        self.index = index
        self.inner = inner      # a nested DocFault, or None
        self.field = field
        self.key = key

    def obj(self, ok=True):
        """The fault as a canonical-form value (HARNESS: the fault object)."""
        pairs = [("ok", False)] if ok else []
        pairs.append(("token", self.token))
        if self.index is not None:
            pairs.append(("index", self.index))
        if self.field is not None:
            pairs.append(("field", self.field))
        if self.key is not None:
            pairs.append(("key", self.key))
        if self.inner is not None:
            pairs.append(("inner", self.inner.obj(ok=False)))
        return Obj(pairs)


# --------------------------------------------------------------------------
# s1 forms
# --------------------------------------------------------------------------

def hexb(v):
    return bytes.fromhex(v[2:])


def hx(b):
    return "0x" + b.hex()


def is_dec256(v):
    if not isinstance(v, str) or len(v) == 0:
        return False
    for c in v:
        if c not in _DIGITS:
            return False
    if len(v) > 1 and v[0] == "0":
        return False
    return int(v) <= MAX_U256


def is_int_spelling(v):
    """galeed/1 s3.2's spelling of an int, as a string."""
    if not isinstance(v, str) or len(v) == 0:
        return False
    for c in v:
        if c not in _DIGITS:
            return False
    if len(v) > 1 and v[0] == "0":
        return False
    return int(v) <= MAX_INT


def is_u32(v):
    return is_int(v) and v <= MAX_U32


def is_bool(v):
    return isinstance(v, bool)


def is_nonzero_hex20(v):
    return is_hex(v, 20) and v != ZERO20


def is_nonzero_hex32(v):
    return is_hex(v, 32) and v != ZERO32


def parse_pointer(v):
    """(chainId, hex20 address) or None."""
    if not isinstance(v, str):
        return None
    pre = "galeed:guild:"
    if not v.startswith(pre):
        return None
    rest = v[len(pre):]
    i = rest.find(":")
    if i < 0:
        return None
    cid, addr = rest[:i], rest[i + 1:]
    if not is_int_spelling(cid) or not is_hex(addr, 20):
        return None
    return (int(cid), addr)


# --------------------------------------------------------------------------
# s3 the quote
# --------------------------------------------------------------------------

QUOTE_FIELDS = ("token", "premiumBps", "feeUnit", "feedToken", "feedUnit",
                "maxStale", "feeFixed", "feeBps", "feeMin", "feeMax", "amountCap",
                "rulingWindow", "offerWindow", "lapseDefault", "autoAccept")


def _is_addr_word(w):
    return w[:12] == bytes(12)


def decode_quote(q):
    """s3.2.  Returns a dict of the fifteen fields (addresses as hex20
    strings, numbers as ints, autoAccept as bool); raises DocFault with
    E_QUOTE_LEN, E_QUOTE_FIELD (field index) or E_QUOTE_FEED."""
    if len(q) != 480:
        raise DocFault("E_QUOTE_LEN")
    words = [q[32 * i:32 * i + 32] for i in range(15)]
    ints = [int.from_bytes(w, "big") for w in words]
    # word order: w0 w1 w2 w3 ... each faulting word names its own index
    order = [(0, lambda: _is_addr_word(words[0]) and ints[0] != 0),
             (1, lambda: ints[1] <= 10000),
             (2, lambda: _is_addr_word(words[2])),
             (3, lambda: _is_addr_word(words[3])),
             (4, lambda: _is_addr_word(words[4])),
             (5, lambda: ints[5] <= MAX_U32),
             (6, lambda: True),
             (7, lambda: ints[7] <= 10000),
             (8, lambda: True),
             (9, lambda: True),
             (10, lambda: True),
             (11, lambda: ints[11] <= MAX_U32),
             (12, lambda: ints[12] <= MAX_U32),
             (13, lambda: ints[13] <= 2),
             (14, lambda: ints[14] <= 1)]
    for i, test in order:
        if not test():
            raise DocFault("E_QUOTE_FIELD", field=i)
    if ints[2] == 0 and (ints[3] != 0 or ints[4] != 0):
        raise DocFault("E_QUOTE_FEED")
    d = {}
    for i, name in enumerate(QUOTE_FIELDS):
        if i in (0, 2, 3, 4):
            d[name] = hx(words[i][12:])
        elif i == 14:
            d[name] = ints[i] == 1
        else:
            d[name] = ints[i]
    return d


def quote_answer(d):
    """The decoded quote as a canonical-form value."""
    pairs = []
    for name in QUOTE_FIELDS:
        v = d[name]
        if name in ("feeFixed", "feeMin", "feeMax", "amountCap"):
            v = str(v)
        pairs.append((name, v))
    return Obj(pairs)


# --------------------------------------------------------------------------
# s2 the document
# --------------------------------------------------------------------------

REQUIRED = ("v", "chainId", "logic", "sellerKey", "sellerPayout", "sellerFund", "token", "amount", "deliveryWindow",
            "confirmWindow", "openWindow", "buyerMayOpen", "ladder", "judges", "guilds", "nonce", "expiry")
ALL = REQUIRED + ("goods_md", "goodsHash", "delivery_md", "deliveryHash", "compliance_md", "series")
ACTIONS = ("ClaimDelivery", "Confirm", "Dispute", "Refund", "Settle", "Cancel")


def _members(d, required, allowed, missing_tok, closed_tok, index=None):
    for k in required:
        if k not in d:
            raise DocFault(missing_tok, index)
    for k in d:
        if k not in allowed:
            raise DocFault(closed_tok, index)


def machine_block(b):
    """s2.2: the block bytes, or DocFault."""
    try:
        b.decode("utf-8")
    except UnicodeDecodeError:
        raise DocFault("E_TERMS_UTF8")
    lines = b.split(b"\n")
    opens = [i for i, ln in enumerate(lines) if ln == OPEN_FENCE]
    if len(opens) == 0:
        raise DocFault("E_TERMS_BLOCK")
    if len(opens) > 1:
        raise DocFault("E_TERMS_FENCES")
    o = opens[0]
    close = None
    for j in range(o + 1, len(lines)):
        if lines[j] == CLOSE_FENCE:
            close = j
            break
    if close is None:
        raise DocFault("E_TERMS_UNCLOSED")
    for ln in lines[:o] + lines[close + 1:]:
        if ln != b"":
            raise DocFault("E_TERMS_OUTSIDE")
    return b"\n".join(lines[o + 1:close])


def check_rung(el, i, allowed=("buyerBps", "sellerBps", "feeBearer", "note_md")):
    if not isinstance(el, Obj):
        raise DocFault("E_RUNG", i)
    d = el.dict()
    _members(d, ("buyerBps", "sellerBps", "feeBearer"), allowed,
             "E_RUNG_MISSING", "E_RUNG_CLOSED", i)
    if not is_int(d["buyerBps"]) or d["buyerBps"] > 10000:
        raise DocFault("E_RUNG_BUYER", i)
    if not is_int(d["sellerBps"]):
        raise DocFault("E_RUNG_SELLER", i)
    if not is_int(d["feeBearer"]) or d["feeBearer"] > 2:
        raise DocFault("E_RUNG_BEARER", i)
    if "note_md" in d and not isinstance(d["note_md"], str):
        raise DocFault("E_RUNG_NOTE", i)
    if d["buyerBps"] + d["sellerBps"] != 10000:
        raise DocFault("E_RUNG_SUM", i)
    return d


def check_ladder_rules(rungs):
    """The three ladder-wide rules over a list of rung dicts."""
    if rungs[0]["buyerBps"] != 0:
        raise DocFault("E_LADDER_FIRST")
    if rungs[-1]["buyerBps"] != 10000:
        raise DocFault("E_LADDER_LAST")
    for i in range(1, len(rungs)):
        if rungs[i]["buyerBps"] < rungs[i - 1]["buyerBps"]:
            raise DocFault("E_LADDER_ORDER", i)


def check_judge(el, i, seller_key, chain_id, required=("judge", "amanahHash", "quoteHash"),
                allowed=("judge", "amanahHash", "quoteHash", "source")):
    if not isinstance(el, Obj):
        raise DocFault("E_JUDGE", i)
    d = el.dict()
    _members(d, required, allowed, "E_JUDGE_MISSING", "E_JUDGE_CLOSED", i)
    if not is_nonzero_hex20(d["judge"]) or d["judge"] == seller_key:
        raise DocFault("E_JUDGE_ADDRESS", i)
    if not is_nonzero_hex32(d["amanahHash"]):
        raise DocFault("E_JUDGE_AMANAH", i)
    if not is_nonzero_hex32(d["quoteHash"]):
        raise DocFault("E_JUDGE_QUOTE", i)
    if "source" in d:
        p = parse_pointer(d["source"])
        if p is None or p[0] != chain_id:
            raise DocFault("E_JUDGE_SOURCE", i)
    return d


def check_guild(el, i, seller_key, chain_id, required=("pointer", "amanahHash", "quoteHash"),
                allowed=("pointer", "amanahHash", "quoteHash")):
    if not isinstance(el, Obj):
        raise DocFault("E_GUILD", i)
    d = el.dict()
    _members(d, required, allowed, "E_GUILD_MISSING", "E_GUILD_CLOSED", i)
    p = parse_pointer(d["pointer"])
    if p is None or p[1] == ZERO20 or p[1] == seller_key or p[0] != chain_id:
        raise DocFault("E_GUILD_POINTER", i)
    if not is_nonzero_hex32(d["amanahHash"]):
        raise DocFault("E_GUILD_AMANAH", i)
    if not is_nonzero_hex32(d["quoteHash"]):
        raise DocFault("E_GUILD_QUOTE", i)
    d["_address"] = p[1]
    return d


def check_candidates(judges, guilds, seller_key, chain_id, judge_fn=check_judge, guild_fn=check_guild,
                     dup_judge="E_TERMS_DUP_JUDGE", dup_guild="E_TERMS_DUP_GUILD",
                     none_tok="E_TERMS_NO_CANDIDATE", judges_tok="E_TERMS_JUDGES", guilds_tok="E_TERMS_GUILDS"):
    if not isinstance(judges, list):
        raise DocFault(judges_tok)
    js = [judge_fn(el, i, seller_key, chain_id) for i, el in enumerate(judges)]
    seen = set()
    for i, j in enumerate(js):
        if j["judge"] in seen:
            raise DocFault(dup_judge, i)
        seen.add(j["judge"])
    if not isinstance(guilds, list):
        raise DocFault(guilds_tok)
    gs = [guild_fn(el, i, seller_key, chain_id) for i, el in enumerate(guilds)]
    seen = set()
    for i, g in enumerate(gs):
        if g["_address"] in seen:
            raise DocFault(dup_guild, i)
        seen.add(g["_address"])
    if len(js) + len(gs) == 0:
        raise DocFault(none_tok)
    return js, gs


def _exactly_one(d, text_key, hash_key, tok):
    """s2.3's pair rule: exactly one of the two present, the text a string,
    the hash a hex32."""
    has_t = text_key in d
    has_h = hash_key in d
    if has_t == has_h:
        raise DocFault(tok)
    if has_t and not isinstance(d[text_key], str):
        raise DocFault(tok)
    if has_h and not is_hex(d[hash_key], 32):
        raise DocFault(tok)


class Terms:
    __slots__ = ("raw", "hash", "d", "ladder", "judges", "guilds")


def _series_ok(v):
    """s2.3 series: exactly id (nonzero hex32), stage (int >= 1), stages (int >= stage)."""
    if not isinstance(v, Obj):
        return False
    s = v.dict()
    if set(s) != {"id", "stage", "stages"}:
        return False
    return (is_nonzero_hex32(s["id"]) and is_int(s["stage"]) and s["stage"] >= 1
            and is_int(s["stages"]) and s["stages"] >= s["stage"])


def check_terms(b):
    """s2.4.  Returns a Terms; raises DocFault."""
    block = machine_block(b)
    try:
        v = accept(block)
    except Fault as f:
        raise DocFault(f.token)
    if not isinstance(v, Obj):
        raise DocFault("E_TERMS")
    d = v.dict()
    _members(d, REQUIRED, ALL, "E_TERMS_MISSING", "E_TERMS_CLOSED")
    if not (isinstance(d["v"], str) and d["v"] == VERSION):
        raise DocFault("E_TERMS_V")
    if not is_int(d["chainId"]):
        raise DocFault("E_TERMS_CHAIN")
    if not is_nonzero_hex20(d["logic"]):
        raise DocFault("E_TERMS_LOGIC")
    if not is_nonzero_hex20(d["sellerKey"]):
        raise DocFault("E_TERMS_SELLER")
    if not is_nonzero_hex20(d["sellerPayout"]):
        raise DocFault("E_TERMS_PAYOUT")
    if not is_nonzero_hex20(d["sellerFund"]):
        raise DocFault("E_TERMS_FUND")
    if not is_nonzero_hex20(d["token"]):
        raise DocFault("E_TERMS_TOKEN")
    if not is_dec256(d["amount"]) or int(d["amount"]) == 0 or int(d["amount"]) > MAX_U256 // 10000:
        raise DocFault("E_TERMS_AMOUNT")
    if not is_u32(d["deliveryWindow"]):
        raise DocFault("E_TERMS_DELIVERY_WINDOW")
    if not is_u32(d["confirmWindow"]):
        raise DocFault("E_TERMS_CONFIRM_WINDOW")
    if not is_u32(d["openWindow"]):
        raise DocFault("E_TERMS_OPENWINDOW")
    if not is_bool(d["buyerMayOpen"]):
        raise DocFault("E_TERMS_BUYEROPEN")
    _exactly_one(d, "goods_md", "goodsHash", "E_TERMS_GOODS")
    _exactly_one(d, "delivery_md", "deliveryHash", "E_TERMS_DELIVERY")
    ld = d["ladder"]
    if not isinstance(ld, list) or not (2 <= len(ld) <= 16):
        raise DocFault("E_TERMS_LADDER")
    ladder = [check_rung(el, i) for i, el in enumerate(ld)]
    check_ladder_rules(ladder)
    judges, guilds = check_candidates(d["judges"], d["guilds"], d["sellerKey"], d["chainId"])
    if not is_int(d["nonce"]):
        raise DocFault("E_TERMS_NONCE")
    if not is_int(d["expiry"]):
        raise DocFault("E_TERMS_EXPIRY")
    if "compliance_md" in d and not isinstance(d["compliance_md"], Obj):
        raise DocFault("E_TERMS_COMPLIANCE")
    if "series" in d and not _series_ok(d["series"]):
        raise DocFault("E_TERMS_SERIES")
    t = Terms()
    t.raw = b
    t.hash = sha256(b)
    t.d = d
    t.ladder = ladder
    t.judges = judges
    t.guilds = guilds
    return t


# --------------------------------------------------------------------------
# s4 the offer and the order
# --------------------------------------------------------------------------

def candidate_fields(judges, guilds, c):
    """(judge hex20, guild hex20, amanahHash hex32, quoteHash hex32) for
    candidate number c, or None when out of range."""
    if c < len(judges):
        j = judges[c]
        return (j["judge"], ZERO20, j["amanahHash"], j["quoteHash"])
    c -= len(judges)
    if c < len(guilds):
        g = guilds[c]
        return (ZERO20, g["_address"], g["amanahHash"], g["quoteHash"])
    return None


def build_offer(seller_key, judge, guild, seller_payout, seller_fund, token, amount, delivery, confirm, open_window,
                buyer_may_open, ladder, terms_hash, amanah_hash, quote_hash, nonce, expiry):
    """An offer value with hex strings, ints and a bool, as the answer carries it."""
    return {"sellerKey": seller_key, "judge": judge, "guild": guild, "sellerPayout": seller_payout,
            "sellerFund": seller_fund, "token": token, "amount": amount, "deliveryWindow": delivery, "confirmWindow": confirm,
            "openWindow": open_window, "buyerMayOpen": buyer_may_open,
            "ladder": [(r["buyerBps"], r["sellerBps"], r["feeBearer"]) for r in ladder],
            "termsHash": terms_hash, "amanahHash": amanah_hash, "quoteHash": quote_hash,
            "nonce": nonce, "expiry": expiry}


def offer_of(t, c):
    """s4.3.  Returns the offer dict."""
    cf = candidate_fields(t.judges, t.guilds, c)
    if cf is None:
        raise DocFault("E_OFFER_CANDIDATE")
    judge, guild, amanah, qh = cf
    d = t.d
    return build_offer(d["sellerKey"], judge, guild, d["sellerPayout"], d["sellerFund"], d["token"], int(d["amount"]),
                       d["deliveryWindow"], d["confirmWindow"], d["openWindow"], d["buyerMayOpen"], t.ladder,
                       hx(t.hash), amanah, qh, d["nonce"], d["expiry"])


def _obytes(o):
    """The offer with bytes values for hashing."""
    return {"sellerKey": hexb(o["sellerKey"]), "judge": hexb(o["judge"]), "guild": hexb(o["guild"]),
            "sellerPayout": hexb(o["sellerPayout"]), "sellerFund": hexb(o["sellerFund"]), "token": hexb(o["token"]),
            "amount": o["amount"],
            "deliveryWindow": o["deliveryWindow"], "confirmWindow": o["confirmWindow"],
            "openWindow": o["openWindow"], "buyerMayOpen": o["buyerMayOpen"],
            "ladder": o["ladder"], "termsHash": hexb(o["termsHash"]), "amanahHash": hexb(o["amanahHash"]),
            "quoteHash": hexb(o["quoteHash"]), "nonce": o["nonce"], "expiry": o["expiry"]}


def offer_struct_hash(o):
    return C.hash_offer(_obytes(o))


def offer_digest(chain_id, logic_hex, o):
    return C.digest(chain_id, hexb(logic_hex), offer_struct_hash(o))


def offer_answer(o, chain_id, logic_hex):
    sh = offer_struct_hash(o)
    dg = C.digest(chain_id, hexb(logic_hex), sh)
    ov = Obj([("amanahHash", o["amanahHash"]), ("amount", str(o["amount"])),
              ("buyerMayOpen", o["buyerMayOpen"]), ("confirmWindow", o["confirmWindow"]),
              ("deliveryWindow", o["deliveryWindow"]), ("expiry", o["expiry"]), ("guild", o["guild"]),
              ("judge", o["judge"]),
              ("ladder", [Obj([("buyerBps", r[0]), ("feeBearer", r[2]), ("sellerBps", r[1])]) for r in o["ladder"]]),
              ("nonce", o["nonce"]), ("openWindow", o["openWindow"]), ("quoteHash", o["quoteHash"]),
              ("sellerFund", o["sellerFund"]), ("sellerKey", o["sellerKey"]), ("sellerPayout", o["sellerPayout"]),
              ("termsHash", o["termsHash"]),
              ("token", o["token"])])
    return Obj([("digest", hx(dg)), ("offer", ov), ("ok", True), ("structHash", hx(sh))])


class Order:
    __slots__ = ("salt", "instance", "digest", "offer_digest")


def order_of(o, chain_id, logic_hex, buyer_key, buyer_payout, open_by, prev):
    """s4.4 over an offer dict, two strings, a value and a string.  Returns an Order."""
    if not is_nonzero_hex20(buyer_key) or buyer_key == o["sellerKey"] or buyer_key == o["judge"] or buyer_key == o["guild"]:
        raise DocFault("E_ORDER_BUYER")
    if not is_nonzero_hex20(buyer_payout):
        raise DocFault("E_ORDER_PAYOUT")
    if not is_int(open_by):
        raise DocFault("E_ORDER_OPENBY")
    if not is_hex(prev, 20):
        raise DocFault("E_ORDER_PREV")
    sh = offer_struct_hash(o)
    salt = C.hash_order(sh, hexb(buyer_key), hexb(buyer_payout), open_by, hexb(prev))
    logic = hexb(logic_hex)
    instance = C.create2(C.DEPLOYER, salt, keccak256(C.initcode(logic)))
    r = Order()
    r.salt = salt
    r.instance = instance
    r.digest = C.digest(chain_id, instance, salt)
    r.offer_digest = C.digest(chain_id, logic, sh)
    return r


def order_answer(r):
    return Obj([("caseId", hx(r.salt)), ("digest", hx(r.digest)), ("instance", hx(r.instance)),
                ("offerDigest", hx(r.offer_digest)), ("ok", True), ("salt", hx(r.salt))])


# --------------------------------------------------------------------------
# s5 signatures
# --------------------------------------------------------------------------

def verify_secp(digest32, sig, expected_hex20):
    """s5.1 / s5.2 over a hex65 value.  Returns the signer hex20; raises DocFault."""
    if not is_hex(sig, 65):
        raise DocFault("E_SIG_FORM")
    r, s, v = split_sig(sig)
    if v != 27 and v != 28:
        raise DocFault("E_SIG_V")
    if not (1 <= r <= N - 1) or not (1 <= s <= N - 1):
        raise DocFault("E_SIG_RANGE")
    if s > HALF_N:
        raise DocFault("E_SIG_HIGH_S")
    pt = recover(digest32, r, s, v - 27)
    if pt is None:
        raise DocFault("E_SIG_RECOVER")
    signer = hx(address_of(pt))
    if signer != expected_hex20:
        raise DocFault("E_SIG_SIGNER")
    return signer


def is_by_call(v):
    """s5.4: the empty string, the second form; nothing else is."""
    return isinstance(v, str) and v == ""


def verify_offer_sig(digest32, sig, seller_hex20):
    """s5.1 with s5.4: ("call", None) on the empty string, else
    ("secp256k1", signer); raises DocFault."""
    if is_by_call(sig):
        return "call", None
    return "secp256k1", verify_secp(digest32, sig, seller_hex20)


def verify_order_sig(digest32, v, buyer_key):
    """s5.2 / s5.3 / s5.4 over a signature value: the empty string, or s5.1
    over any other value, one that is no string answering E_SIG_FORM.
    Returns the form."""
    if is_by_call(v):
        return "call"
    if not isinstance(v, str):
        raise DocFault("E_SIG_FORM")
    verify_secp(digest32, v, buyer_key)
    return "secp256k1"


# --------------------------------------------------------------------------
# s6 fees
# --------------------------------------------------------------------------

def fee_of(quote, amount):
    """s6.1.  Returns (premium, priced, lf-or-None); raises E_FEE_OVERFLOW."""
    premium = amount * quote["premiumBps"] // 10000
    if quote["feeUnit"] != ZERO20:
        return premium, False, None
    lf = quote["feeFixed"] + amount * quote["feeBps"] // 10000
    if lf > MAX_U256:
        raise DocFault("E_FEE_OVERFLOW")
    if quote["feeMin"] != 0 and lf < quote["feeMin"]:
        lf = quote["feeMin"]
    if quote["feeMax"] != 0 and lf > quote["feeMax"]:
        lf = quote["feeMax"]
    if premium + lf > MAX_U256:
        raise DocFault("E_FEE_OVERFLOW")
    return premium, True, lf


# --------------------------------------------------------------------------
# s4.5 action messages
# --------------------------------------------------------------------------

ACTION_TYPES = {
    "ClaimDelivery": b"ClaimDelivery(bytes32 manifest)",
    "Confirm": b"Confirm()",
    "Dispute": b"Dispute()",
    "Refund": b"Refund()",
    "Settle": b"Settle(uint16 buyerBps,uint256 expiry)",
    "Cancel": b"Cancel()",
}
ACTION_ARGS = {"ClaimDelivery": 1, "Confirm": 0, "Dispute": 0, "Refund": 0, "Settle": 2, "Cancel": 0}
ACTION_PARTIES = {"ClaimDelivery": ("seller",), "Confirm": ("buyer",), "Dispute": ("buyer",),
                  "Refund": ("seller",), "Settle": ("buyer", "seller"), "Cancel": ("buyer",)}


def action_struct_hash(name, args):
    """s4.5 over a name of the six and its arguments (strings from the
    harness or values).  Returns the struct hash; raises DocFault."""
    th = keccak256(ACTION_TYPES[name])
    if name == "ClaimDelivery":
        m = args[0]
        if not is_nonzero_hex32(m):
            raise DocFault("E_ACTION_MANIFEST")
        return keccak256(th + hexb(m))
    if name == "Settle":
        bps, exp = args
        if not (is_int(bps) and bps <= 10000):
            raise DocFault("E_ACTION_BPS")
        return keccak256(th + C.word(bps) + C.word(exp))
    return keccak256(th)


def action_digest(chain_id, instance20, struct_hash):
    return C.digest(chain_id, instance20, struct_hash)


def action_party_ok(name, party):
    return party in ACTION_PARTIES[name]


def verify_action_sig(name, party, digest32, v, seller_key, buyer_key):
    """s4.5 with s5: the party rule first, then the party's form over the
    action digest; the empty string is s5.4's answer for Settle alone and
    E_SIG_FORM for the other five.  Returns the form; raises DocFault."""
    if not action_party_ok(name, party):
        raise DocFault("E_ACTION_PARTY")
    if is_by_call(v):
        if name == "Settle":
            return "call"
        raise DocFault("E_SIG_FORM")
    if party == "seller":
        if not isinstance(v, str):
            raise DocFault("E_SIG_FORM")
        verify_secp(digest32, v, seller_key)
        return "secp256k1"
    return verify_order_sig(digest32, v, buyer_key)
