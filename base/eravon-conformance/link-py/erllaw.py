"""eravon.link/1 sections 1 through 8: the listing, the link, the receipt,
the case pack, identity over a chain's records (the succession book's replay
of s1 among them), the binding of a listing to its document, and the
settlement offer.

Built on the eravon.terms/1 core in ../terms-py; every predicate below is
the law's own.
"""
import base64
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "..", "terms-py"))

from ertcanon import Fault, Obj, accept, canon, is_hex, is_int      # noqa: E402
from ertcrypto import sha256                                        # noqa: E402
import ertcrypto as C                                               # noqa: E402
import ertlaw as T                                                  # noqa: E402
from ertlaw import DocFault, hx, hexb, ZERO20                       # noqa: E402

V_LISTING = "eravon.listing/1"
V_LINK = "eravon.link/1"
V_RECEIPT = "eravon.receipt/1"
V_PACK = "eravon.pack/1"
V_SETTLE = "eravon.settle/1"
OPEN_PREFIX = b"eravon://open?e="
RECEIPT_PREFIX = b"eravon-receipt:"
_B64 = frozenset(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_")


class Verdict(Exception):
    """A value of a closed vocabulary (bind, bindReceipt, resolveLink,
    bindSettle), carried with an optional index, an optional nested fault,
    and extra answer members."""

    def __init__(self, name, index=None, inner=None, extra=None):
        Exception.__init__(self, name)
        self.name = name
        self.index = index
        self.inner = inner
        self.extra = extra or []

    def obj(self):
        pairs = [("verdict", self.name)]
        if self.index is not None:
            pairs.append(("index", self.index))
        if self.inner is not None:
            pairs.append(("inner", self.inner.obj(ok=False)))
        pairs.extend(self.extra)
        return Obj(pairs)

    def as_fault(self):
        """The verdict as a nested fault object (for E_PACK_ORDER and the like)."""
        return DocFault(self.name, index=self.index, inner=self.inner)


def is_skeleton_nonempty(v):
    return isinstance(v, str) and len(v) > 0


def _utf8(s):
    return s.encode("utf-8")


# --------------------------------------------------------------------------
# s2 the listing
# --------------------------------------------------------------------------

LISTING_KEYS = ("v", "chainId", "token", "logic", "proxyCodeHash", "terms", "order", "judges", "guilds")
ORDER_KEYS = ("sellerKey", "sellerPayout", "sellerFund", "amount", "deliveryWindow", "confirmWindow", "openWindow",
              "buyerMayOpen", "ladder", "nonce", "expiry")


class Listing:
    __slots__ = ("raw", "d", "terms_bytes", "terms", "order", "ladder", "judges", "guilds", "offers", "by_call")


def _check_terms_member(v):
    if not isinstance(v, Obj):
        raise DocFault("E_LISTING_TERMS")
    d = v.dict()
    if set(d.keys()) != {"hash", "inline_md"}:
        raise DocFault("E_LISTING_TERMS")
    if not T.is_nonzero_hex32(d["hash"]) or not isinstance(d["inline_md"], str):
        raise DocFault("E_LISTING_TERMS")
    tb = _utf8(d["inline_md"])
    if hx(sha256(tb)) != d["hash"]:
        raise DocFault("E_LISTING_TERMS_HASH")
    try:
        t = T.check_terms(tb)
    except DocFault as f:
        raise DocFault("E_LISTING_TERMS_DOC", inner=f)
    return d, tb, t


def _check_order(v):
    if not isinstance(v, Obj):
        raise DocFault("E_LISTING_ORDER_MISSING")
    d = v.dict()
    T._members(d, ORDER_KEYS, ORDER_KEYS, "E_LISTING_ORDER_MISSING", "E_LISTING_ORDER_CLOSED")
    if not T.is_nonzero_hex20(d["sellerKey"]):
        raise DocFault("E_LISTING_SELLER")
    if not T.is_nonzero_hex20(d["sellerPayout"]):
        raise DocFault("E_LISTING_PAYOUT")
    if not T.is_nonzero_hex20(d["sellerFund"]):
        raise DocFault("E_LISTING_FUND")
    if not T.is_dec256(d["amount"]) or int(d["amount"]) == 0:
        raise DocFault("E_LISTING_AMOUNT")
    if not T.is_u32(d["deliveryWindow"]):
        raise DocFault("E_LISTING_DELIVERY_WINDOW")
    if not T.is_u32(d["confirmWindow"]):
        raise DocFault("E_LISTING_CONFIRM_WINDOW")
    if not T.is_u32(d["openWindow"]):
        raise DocFault("E_LISTING_OPENWINDOW")
    if not T.is_bool(d["buyerMayOpen"]):
        raise DocFault("E_LISTING_BUYEROPEN")
    ld = d["ladder"]
    if not isinstance(ld, list) or not (2 <= len(ld) <= 16):
        raise DocFault("E_LISTING_LADDER")
    ladder = [T.check_rung(el, i, allowed=("buyerBps", "sellerBps", "feeBearer")) for i, el in enumerate(ld)]
    T.check_ladder_rules(ladder)
    if not is_int(d["nonce"]):
        raise DocFault("E_LISTING_NONCE")
    if not is_int(d["expiry"]):
        raise DocFault("E_LISTING_EXPIRY")
    return d, ladder


def _check_listing_judge(el, i, seller_key, chain_id):
    d = T.check_judge(el, i, seller_key, chain_id,
                      required=("judge", "amanahHash", "quoteHash", "sig"),
                      allowed=("judge", "amanahHash", "quoteHash", "source", "sig"))
    if not (is_hex(d["sig"], 65) or T.is_by_call(d["sig"])):
        raise DocFault("E_JUDGE_SIG", i)
    return d


def _check_listing_guild(el, i, seller_key, chain_id):
    d = T.check_guild(el, i, seller_key, chain_id,
                      required=("pointer", "amanahHash", "quoteHash", "seq", "panelHash", "sig"),
                      allowed=("pointer", "amanahHash", "quoteHash", "seq", "panelHash", "sig"))
    if not is_int(d["seq"]):
        raise DocFault("E_GUILD_SEQ", i)
    if not T.is_nonzero_hex32(d["panelHash"]):
        raise DocFault("E_GUILD_PANEL", i)
    if not (is_hex(d["sig"], 65) or T.is_by_call(d["sig"])):
        raise DocFault("E_GUILD_SIG", i)
    return d


def listing_offer(s, c):
    """s2.4: the offer value for candidate c of a checked listing; None when
    c is out of range."""
    cf = T.candidate_fields(s.judges, s.guilds, c)
    if cf is None:
        return None
    judge, guild, amanah, qh = cf
    o = s.order
    return T.build_offer(o["sellerKey"], judge, guild, o["sellerPayout"], o["sellerFund"], s.d["token"], int(o["amount"]),
                         o["deliveryWindow"], o["confirmWindow"], o["openWindow"], o["buyerMayOpen"], s.ladder,
                         s.d["terms"]["hash"], amanah, qh, o["nonce"], o["expiry"])


def check_listing(b):
    """s2.5.  Returns a Listing; raises DocFault."""
    try:
        v = accept(b)
    except Fault as f:
        raise DocFault(f.token)
    if not isinstance(v, Obj):
        raise DocFault("E_LISTING")
    d = v.dict()
    T._members(d, LISTING_KEYS, LISTING_KEYS, "E_LISTING_MISSING", "E_LISTING_CLOSED")
    if not (isinstance(d["v"], str) and d["v"] == V_LISTING):
        raise DocFault("E_LISTING_V")
    if not is_int(d["chainId"]):
        raise DocFault("E_LISTING_CHAIN")
    if not T.is_nonzero_hex20(d["token"]):
        raise DocFault("E_LISTING_TOKEN")
    if not T.is_nonzero_hex20(d["logic"]):
        raise DocFault("E_LISTING_LOGIC")
    if not is_hex(d["proxyCodeHash"], 32) or d["proxyCodeHash"] != hx(C.proxy_codehash(hexb(d["logic"]))):
        raise DocFault("E_LISTING_CODEHASH")
    terms, tb, t = _check_terms_member(d["terms"])
    order, ladder = _check_order(d["order"])
    judges, guilds = T.check_candidates(
        d["judges"], d["guilds"], order["sellerKey"], d["chainId"],
        judge_fn=_check_listing_judge, guild_fn=_check_listing_guild,
        dup_judge="E_LISTING_DUP_JUDGE", dup_guild="E_LISTING_DUP_GUILD",
        none_tok="E_LISTING_NO_CANDIDATE", judges_tok="E_LISTING_JUDGES", guilds_tok="E_LISTING_GUILDS")
    s = Listing()
    s.raw = b
    s.d = d
    s.d["terms"] = terms
    s.terms_bytes = tb
    s.terms = t
    s.order = order
    s.ladder = ladder
    s.judges = judges
    s.guilds = guilds
    try:
        bind_compare(s, t)
    except Verdict as vv:
        raise DocFault("E_LISTING_BIND", inner=vv.as_fault())
    s.offers = []
    s.by_call = []
    for c in range(len(judges) + len(guilds)):
        off = listing_offer(s, c)
        dg = T.offer_digest(d["chainId"], d["logic"], off)
        sig = judges[c]["sig"] if c < len(judges) else guilds[c - len(judges)]["sig"]
        try:
            form, _ = T.verify_offer_sig(dg, sig, order["sellerKey"])
        except DocFault as f:
            raise DocFault("E_LISTING_SIG", c, inner=f)
        if form == "call":
            s.by_call.append((c, hx(dg)))
        s.offers.append(off)
    return s


def listing_answer(s):
    by_call = [Obj([("candidate", c), ("digest", dg)]) for c, dg in s.by_call]
    return Obj([("byCall", by_call), ("candidates", len(s.judges) + len(s.guilds)), ("ok", True),
                ("termsHash", s.d["terms"]["hash"])])


# --------------------------------------------------------------------------
# s7 binding a listing to its document
# --------------------------------------------------------------------------

def bind(sb, tb):
    """s7.1 over two byte strings.  Returns (listing, terms); raises Verdict."""
    try:
        s = check_listing(sb)
    except DocFault as f:
        raise Verdict("LISTING_INVALID", inner=f)
    try:
        t = T.check_terms(tb)
    except DocFault as f:
        raise Verdict("TERMS_INVALID", inner=f)
    if hx(t.hash) != s.d["terms"]["hash"]:
        raise Verdict("BIND_HASH")
    bind_compare(s, t)
    return s, t


def bind_compare(s, t):
    """s7.1 from BIND_CHAIN on: the listing's members against the document's."""
    if s.d["chainId"] != t.d["chainId"]:
        raise Verdict("BIND_CHAIN")
    if s.d["token"] != t.d["token"]:
        raise Verdict("BIND_TOKEN")
    if s.d["logic"] != t.d["logic"]:
        raise Verdict("BIND_LOGIC")
    o = s.order
    td = t.d
    same = (o["sellerKey"] == td["sellerKey"] and o["sellerPayout"] == td["sellerPayout"]
            and o["sellerFund"] == td["sellerFund"]
            and int(o["amount"]) == int(td["amount"]) and o["deliveryWindow"] == td["deliveryWindow"]
            and o["confirmWindow"] == td["confirmWindow"] and o["openWindow"] == td["openWindow"]
            and o["buyerMayOpen"] == td["buyerMayOpen"]
            and [(r["buyerBps"], r["sellerBps"], r["feeBearer"]) for r in s.ladder]
            == [(r["buyerBps"], r["sellerBps"], r["feeBearer"]) for r in t.ladder]
            and o["nonce"] == td["nonce"] and o["expiry"] == td["expiry"])
    if not same:
        raise Verdict("BIND_ORDER")
    if len(s.judges) != len(t.judges):
        raise Verdict("BIND_JUDGES", len(s.judges))
    for i, (a, b) in enumerate(zip(s.judges, t.judges)):
        if (a["judge"], a["amanahHash"], a["quoteHash"], a.get("source")) != (b["judge"], b["amanahHash"], b["quoteHash"], b.get("source")):
            raise Verdict("BIND_JUDGES", i)
    if len(s.guilds) != len(t.guilds):
        raise Verdict("BIND_GUILDS", len(s.guilds))
    for i, (a, b) in enumerate(zip(s.guilds, t.guilds)):
        if (a["pointer"], a["amanahHash"], a["quoteHash"]) != (b["pointer"], b["amanahHash"], b["quoteHash"]):
            raise Verdict("BIND_GUILDS", i)


# --------------------------------------------------------------------------
# s3 the link
# --------------------------------------------------------------------------

LINK_KEYS = ("v", "chainId", "tx", "hash")


def check_link(b):
    """s3.1.  Returns the dict; raises DocFault."""
    try:
        v = accept(b)
    except Fault as f:
        raise DocFault(f.token)
    if not isinstance(v, Obj):
        raise DocFault("E_LINK")
    d = v.dict()
    T._members(d, LINK_KEYS, LINK_KEYS, "E_LINK_MISSING", "E_LINK_CLOSED")
    if not (isinstance(d["v"], str) and d["v"] == V_LINK):
        raise DocFault("E_LINK_V")
    if not is_int(d["chainId"]):
        raise DocFault("E_LINK_CHAIN")
    if not is_hex(d["tx"], 32):
        raise DocFault("E_LINK_TX")
    if not is_hex(d["hash"], 32):
        raise DocFault("E_LINK_HASH")
    return d


def link_answer(d):
    return Obj([("chainId", d["chainId"]), ("hash", d["hash"]), ("ok", True), ("tx", d["tx"])])


def b64e(b):
    return base64.urlsafe_b64encode(b).rstrip(b"=")


def b64d(seg):
    """Canonical base64url without padding, or None."""
    if len(seg) == 0 or len(seg) % 4 == 1:
        return None
    for c in seg:
        if c not in _B64:
            return None
    raw = base64.urlsafe_b64decode(seg + b"=" * (-len(seg) % 4))
    if b64e(raw) != seg:
        return None
    return raw


def decode_transport(t, prefix, prefix_tok, b64_tok, doc_tok, check):
    if not t.startswith(prefix):
        raise DocFault(prefix_tok)
    raw = b64d(t[len(prefix):])
    if raw is None:
        raise DocFault(b64_tok)
    try:
        check(raw)
    except DocFault as f:
        raise DocFault(doc_tok, inner=f)
    return raw


def _kind_of(b):
    """The `v` of a canonical object, or None where s3.3's LINK_KIND test fails."""
    try:
        v = accept(b)
    except Fault:
        return None
    if not isinstance(v, Obj):
        return None
    d = v.dict()
    if "v" not in d or not isinstance(d["v"], str):
        return None
    return d["v"]


def resolve_link(lb, db):
    """s3.3 over a link byte string and a fetched document's bytes.  Returns
    (kind, answer Obj); raises Verdict."""
    try:
        ln = check_link(lb)
    except DocFault as f:
        raise Verdict("LINK_INVALID", inner=f)
    if hx(sha256(db)) != ln["hash"]:
        raise Verdict("LINK_HASH")
    kind = _kind_of(db)
    if kind is None:
        raise Verdict("LINK_KIND")
    if kind == V_LISTING:
        try:
            s = check_listing(db)
        except DocFault as f:
            raise Verdict("LINK_LISTING", inner=f)
        if s.d["chainId"] != ln["chainId"]:
            raise Verdict("LINK_CHAIN")
        return "LISTING", listing_answer(s)
    if kind == V_SETTLE:
        try:
            st = check_settle(db)
        except DocFault as f:
            raise Verdict("LINK_SETTLE", inner=f)
        if st["chainId"] != ln["chainId"]:
            raise Verdict("LINK_CHAIN")
        return "SETTLE", settle_answer(st)
    raise Verdict("LINK_UNKNOWN")


# --------------------------------------------------------------------------
# s4 the receipt
# --------------------------------------------------------------------------

RECEIPT_REQUIRED = ("v", "termsHash", "candidate", "buyerKey", "buyerPayout", "openBy", "prev", "sig")
RECEIPT_ALL = RECEIPT_REQUIRED + ("seq",)


def check_receipt(b):
    """s4.2.  Returns the dict; raises DocFault."""
    try:
        v = accept(b)
    except Fault as f:
        raise DocFault(f.token)
    if not isinstance(v, Obj):
        raise DocFault("E_RECEIPT")
    d = v.dict()
    T._members(d, RECEIPT_REQUIRED, RECEIPT_ALL, "E_RECEIPT_MISSING", "E_RECEIPT_CLOSED")
    if not (isinstance(d["v"], str) and d["v"] == V_RECEIPT):
        raise DocFault("E_RECEIPT_V")
    if not T.is_nonzero_hex32(d["termsHash"]):
        raise DocFault("E_RECEIPT_TERMS")
    if not is_int(d["candidate"]):
        raise DocFault("E_RECEIPT_CANDIDATE")
    if not T.is_nonzero_hex20(d["buyerKey"]):
        raise DocFault("E_RECEIPT_BUYER")
    if not T.is_nonzero_hex20(d["buyerPayout"]):
        raise DocFault("E_RECEIPT_PAYOUT")
    if not is_int(d["openBy"]):
        raise DocFault("E_RECEIPT_OPENBY")
    if not is_hex(d["prev"], 20):
        raise DocFault("E_RECEIPT_PREV")
    s = d["sig"]
    if not (is_hex(s, 65) or T.is_by_call(s)):
        raise DocFault("E_RECEIPT_SIG")
    if "seq" in d and not is_int(d["seq"]):
        raise DocFault("E_RECEIPT_SEQ")
    return d


def receipt_form(r):
    return "call" if T.is_by_call(r["sig"]) else "secp256k1"


class Bound:
    __slots__ = ("salt", "instance", "digest", "offer", "candidate", "form")


def bind_receipt(sb, rb):
    """s4.4 over two byte strings.  Returns a Bound; raises Verdict."""
    try:
        s = check_listing(sb)
    except DocFault as f:
        raise Verdict("LISTING_INVALID", inner=f)
    try:
        r = check_receipt(rb)
    except DocFault as f:
        raise Verdict("RECEIPT_INVALID", inner=f)
    return bind_receipt_checked(s, r)


def bind_receipt_checked(s, r):
    if r["termsHash"] != s.d["terms"]["hash"]:
        raise Verdict("RECEIPT_TERMS_MISMATCH")
    c = r["candidate"]
    n = len(s.judges) + len(s.guilds)
    if c >= n:
        raise Verdict("RECEIPT_CANDIDATE_RANGE")
    is_guild = c >= len(s.judges)
    if is_guild != ("seq" in r):
        raise Verdict("RECEIPT_SEQ")
    off = s.offers[c]
    try:
        od = T.order_of(off, s.d["chainId"], s.d["logic"], r["buyerKey"], r["buyerPayout"], r["openBy"], r["prev"])
    except DocFault as f:
        raise Verdict("RECEIPT_ORDER", inner=f)
    try:
        form = T.verify_order_sig(od.digest, r["sig"], r["buyerKey"])
    except DocFault as f:
        raise Verdict("RECEIPT_SIG", inner=f)
    b = Bound()
    b.salt = od.salt
    b.instance = od.instance
    b.digest = od.digest
    b.offer = off
    b.candidate = c
    b.form = form
    return b


def bound_answer(b):
    """RECEIPT_BOUND, or RECEIPT_BY_CALL where the buyer's slot was empty (s4.4)."""
    return Obj([("caseId", hx(b.salt)), ("digest", hx(b.digest)), ("form", b.form),
                ("instance", hx(b.instance)), ("salt", hx(b.salt)),
                ("verdict", "RECEIPT_BY_CALL" if b.form == "call" else "RECEIPT_BOUND")])


# --------------------------------------------------------------------------
# s5 the case pack
# --------------------------------------------------------------------------

PACK_REQUIRED = ("v", "chainId", "instance", "caseId", "link", "listing", "receipt")
PACK_ALL = PACK_REQUIRED + ("attachments",)


class Pack:
    __slots__ = ("d", "link", "listing", "receipt", "bound", "attachments")


def _check_attachment(el, i, named):
    if not isinstance(el, Obj):
        raise DocFault("E_PACK_ATTACHMENT", i)
    d = el.dict()
    if set(d.keys()) != {"sha256", "text_md"} or not is_hex(d["sha256"], 32) or not isinstance(d["text_md"], str):
        raise DocFault("E_PACK_ATTACHMENT", i)
    if hx(sha256(_utf8(d["text_md"]))) != d["sha256"]:
        raise DocFault("E_PACK_ATTACHMENT_HASH", i)
    if d["sha256"] not in named:
        raise DocFault("E_PACK_ATTACHMENT_UNNAMED", i)
    return d


def named_hashes(s):
    """s5.3: the goodsHash and deliveryHash the listing's document carries, as the set of their spellings."""
    td = s.terms.d
    return {v for v in (td.get("goodsHash"), td.get("deliveryHash")) if v is not None}


def check_pack(b):
    """s5.2.  Returns a Pack; raises DocFault."""
    try:
        v = accept(b)
    except Fault as f:
        raise DocFault(f.token)
    if not isinstance(v, Obj):
        raise DocFault("E_PACK")
    d = v.dict()
    T._members(d, PACK_REQUIRED, PACK_ALL, "E_PACK_MISSING", "E_PACK_CLOSED")
    if not (isinstance(d["v"], str) and d["v"] == V_PACK):
        raise DocFault("E_PACK_V")
    if not is_int(d["chainId"]):
        raise DocFault("E_PACK_CHAIN")
    if not is_hex(d["instance"], 20):
        raise DocFault("E_PACK_INSTANCE")
    if not is_hex(d["caseId"], 32):
        raise DocFault("E_PACK_CASE")
    if not isinstance(d["link"], Obj):
        raise DocFault("E_PACK_LINK", inner=DocFault("E_LINK"))
    try:
        ln = check_link(canon(d["link"]))
    except DocFault as f:
        raise DocFault("E_PACK_LINK", inner=f)
    if not isinstance(d["listing"], Obj):
        raise DocFault("E_PACK_LISTING", inner=DocFault("E_LISTING"))
    sb = canon(d["listing"])
    try:
        s = check_listing(sb)
    except DocFault as f:
        raise DocFault("E_PACK_LISTING", inner=f)
    if hx(sha256(sb)) != ln["hash"]:
        raise DocFault("E_PACK_HASH")
    if not isinstance(d["receipt"], Obj):
        raise DocFault("E_PACK_RECEIPT", inner=DocFault("E_RECEIPT"))
    try:
        r = check_receipt(canon(d["receipt"]))
    except DocFault as f:
        raise DocFault("E_PACK_RECEIPT", inner=f)
    try:
        bd = bind_receipt_checked(s, r)
    except Verdict as vv:
        raise DocFault("E_PACK_ORDER", inner=vv.as_fault())
    if d["chainId"] != s.d["chainId"] or d["chainId"] != ln["chainId"]:
        raise DocFault("E_PACK_CHAIN_MISMATCH")
    if d["instance"] != hx(bd.instance):
        raise DocFault("E_PACK_INSTANCE_MISMATCH")
    if d["caseId"] != hx(bd.salt):
        raise DocFault("E_PACK_CASE_MISMATCH")
    attachments = []
    if "attachments" in d:
        if not isinstance(d["attachments"], list):
            raise DocFault("E_PACK_ATTACHMENTS")
        attachments = [_check_attachment(el, i, named_hashes(s)) for i, el in enumerate(d["attachments"])]
    pk = Pack()
    pk.d = d
    pk.link = ln
    pk.listing = s
    pk.receipt = r
    pk.bound = bd
    pk.attachments = attachments
    return pk


# --------------------------------------------------------------------------
# s1 the book's replay and currentAt; s6 identity over a record set
# --------------------------------------------------------------------------

class Records:
    """A record set of one chain (s6.1): each kind a list of dicts carrying
    `blockNumber`, `logIndex` and the record's members, as HARNESS-LINK.md's
    <records.json> spells them; `pos` is (blockNumber, logIndex)."""
    KINDS = ("posted", "consented", "revoked", "pulled", "opened", "events", "declarations", "accessions")

    def __init__(self, chain_id, kinds):
        self.chain_id = chain_id
        self.k = kinds
        self.book = sorted([("d", r) for r in kinds["declarations"]] + [("a", r) for r in kinds["accessions"]],
                           key=lambda x: pos(x[1]))


def pos(r):
    return (r["blockNumber"], r["logIndex"])


class Replay:
    """s1: root, head, pending, entered, walked in ascending position."""

    def __init__(self):
        self.root = {}
        self.head = {}
        self.pending = {}
        self.entered = set()
        self.acceded = []   # (position, holder, to, root) of every accession that counted

    def root_of(self, a):
        return self.root.get(a, a)

    def current(self, a):
        r = self.root_of(a)
        return self.head.get(r, r)

    def step(self, kind, r):
        if kind == "d":
            f, t = r["from"], r["to"]
            if self.current(f) == f and t != ZERO20 and t != f and t not in self.entered:
                self.pending[f] = t
                self.entered.add(f)
        else:
            h, t = r["holder"], r["to"]
            if t not in self.entered and self.pending.get(h) == t and self.current(h) == h:
                rt = self.root_of(h)
                self.root[t] = rt
                self.head[rt] = t
                del self.pending[h]
                self.entered.add(t)
                self.acceded.append((pos(r), h, t, rt))


def replay(rs, before=None):
    """The replay over every book record of `rs`, or over those before `before`."""
    rp = Replay()
    for kind, r in rs.book:
        if before is not None and pos(r) >= before:
            break
        rp.step(kind, r)
    return rp


def current_at(rs, a, p):
    """s1 currentAt(a, p): the current key of `a`, the records at `p` and after it unread."""
    return replay(rs, p).current(a)


def keys_of(rs, a):
    """s1: the keys of a's record, its root and then each key in the order it acceded."""
    rp = replay(rs)
    rt = rp.root_of(a)
    return [rt] + [t for (_p, _h, t, r) in rp.acceded if r == rt]


def stands(rs, k, p):
    """s6.3: a thing k did at p stands where currentAt(k, p) is k."""
    return current_at(rs, k, p) == k


def own_posting(r):
    """s6.2: the (sellerKey, termsHash) a posting of its own draws, or None
    (a repost, or bytes that fail s2.5)."""
    try:
        s = check_listing(hexb(r["document"]))
    except DocFault:
        return None
    sk, th = s.order["sellerKey"], s.d["terms"]["hash"]
    if r["key"] != th or r["poster"] != sk:
        return None
    return (sk, th)


class Document:
    __slots__ = ("seller", "terms", "postings", "opened", "revoked", "position", "source", "standing")


def documents(rs):
    """s6.2 and s6.3: every document of the record set, by (sellerKey, termsHash)."""
    docs = {}

    def get(key):
        if key not in docs:
            d = Document()
            d.seller, d.terms = key
            d.postings, d.opened, d.revoked = [], [], []
            docs[key] = d
        return docs[key]
    for r in rs.k["posted"]:
        key = own_posting(r)
        if key is not None:
            get(key).postings.append(r)
    for r in rs.k["opened"]:
        get((r["seller"], r["termsHash"])).opened.append(r)
    for r in rs.k["revoked"]:
        get((r["seller"], r["termsHash"])).revoked.append(r)
    for d in docs.values():
        for lst in (d.postings, d.opened, d.revoked):
            lst.sort(key=pos)
        if d.postings:
            first = d.postings[0]
            d.position, d.source = pos(first), "self"
            d.standing = "stands" if stands(rs, first["poster"], pos(first)) else "attempt"
        elif d.opened:
            d.position, d.source, d.standing = pos(d.opened[0]), "opened", "stands"
        else:
            d.position, d.source, d.standing = pos(d.revoked[0]), "revoked", "stands"
    return docs


def place(rs, seller, terms_hash):
    """s6.2: the document's position and source, or None."""
    d = documents(rs).get((seller, terms_hash))
    return None if d is None else (d.position, d.source)


def pay_check(sb, rs):
    """s6.4 over a listing byte string and a record set.  Returns the verdict
    name; raises Verdict for LISTING_INVALID."""
    try:
        s = check_listing(sb)
    except DocFault as f:
        raise Verdict("LISTING_INVALID", inner=f)
    sk, th = s.order["sellerKey"], s.d["terms"]["hash"]
    own = sorted((r for r in rs.k["posted"] if hexb(r["document"]) == sb and r["key"] == th and r["poster"] == sk),
                 key=pos)
    if not own:
        return "UNPLACED"
    return "PAYABLE" if stands(rs, own[0]["poster"], pos(own[0])) else "SELLER_KEY_RETIRED"


def history(rs, a):
    """s6.5.  Returns (keys, documents, consents, cases, attempts), each
    document a Document, each case an opened record."""
    keys = keys_of(rs, a)
    ks = set(keys)
    docs = sorted((d for d in documents(rs).values() if d.seller in ks), key=lambda d: (d.position, d.terms))
    consents = sorted((r for r in rs.k["consented"] if r["who"] in ks), key=pos)
    cases = sorted((r for r in rs.k["opened"] if r["buyer"] in ks), key=pos)
    attempts = [d for d in docs if d.standing == "attempt"]
    return keys, docs, consents, cases, attempts


def case_records(rs, opened):
    """s6.5: every record whose emitter is the case's instance, and every pulled
    whose instance is it, in ascending position."""
    inst = opened["emitter"]
    out = [r for r in rs.k["opened"] + rs.k["events"] if r["emitter"] == inst]
    out += [r for r in rs.k["pulled"] if r["instance"] == inst]
    return sorted(out, key=pos)


# --------------------------------------------------------------------------
# s8 the settlement offer
# --------------------------------------------------------------------------

SETTLE_KEYS = ("v", "chainId", "instance", "buyerBps", "expiry", "sig")


def check_settle(b):
    """s8.2.  Returns the dict; raises DocFault."""
    try:
        v = accept(b)
    except Fault as f:
        raise DocFault(f.token)
    if not isinstance(v, Obj):
        raise DocFault("E_SETTLE")
    d = v.dict()
    T._members(d, SETTLE_KEYS, SETTLE_KEYS, "E_SETTLE_MISSING", "E_SETTLE_CLOSED")
    if not (isinstance(d["v"], str) and d["v"] == V_SETTLE):
        raise DocFault("E_SETTLE_V")
    if not is_int(d["chainId"]):
        raise DocFault("E_SETTLE_CHAIN")
    if not T.is_nonzero_hex20(d["instance"]):
        raise DocFault("E_SETTLE_INSTANCE")
    if not (is_int(d["buyerBps"]) and d["buyerBps"] <= 10000):
        raise DocFault("E_SETTLE_BPS")
    if not is_int(d["expiry"]):
        raise DocFault("E_SETTLE_EXPIRY")
    if not (is_hex(d["sig"], 65) or T.is_by_call(d["sig"])):
        raise DocFault("E_SETTLE_SIG")
    return d


def settle_answer(st):
    return Obj([("buyerBps", st["buyerBps"]), ("expiry", st["expiry"]), ("instance", st["instance"]), ("ok", True)])


def bind_settle(kb, sb):
    """s8.3 over two byte strings.  Returns (action digest, form), the form
    "call" where the seller's slot was empty; raises Verdict."""
    try:
        pk = check_pack(kb)
    except DocFault as f:
        raise Verdict("PACK_INVALID", inner=f)
    try:
        st = check_settle(sb)
    except DocFault as f:
        raise Verdict("SETTLE_INVALID", inner=f)
    if st["chainId"] != pk.d["chainId"]:
        raise Verdict("SETTLE_CHAIN")
    if st["instance"] != pk.d["instance"]:
        raise Verdict("SETTLE_INSTANCE")
    sh = T.action_struct_hash("Settle", [st["buyerBps"], st["expiry"]])
    dg = T.action_digest(pk.d["chainId"], hexb(pk.d["instance"]), sh)
    try:
        form, _ = T.verify_offer_sig(dg, st["sig"], pk.listing.order["sellerKey"])
    except DocFault as f:
        raise Verdict("SETTLE_SIG", inner=f)
    return dg, form
