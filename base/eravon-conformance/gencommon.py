#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Shared helpers of the three ERAVON corpus generators.

Borrowed from the GALEED base's galeed/1 core (../galeed-conformance/impl-py,
settled law, never this law's predicate): canonical bytes, keccak256,
sha256, secp256k1 keys and RFC 6979 signing, and the galeed/1 entry digest.
Everything
that is a prediction of one of the three laws (typed data, CREATE2, the
document orders, the tokens, the action messages) is written here or in
the generators from the texts alone, as a third opinion beside the
criterion and the candidates.
"""
import base64
import hashlib
import os
import random
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "galeed-conformance", "impl-py"))

from glcanon import Obj, canon                                   # noqa: E402
from glcrypto import keccak256, sha256, address_of, pubkey, sign_digest, N  # noqa: E402

N_ORDER = N
RNG = random.Random(1)
SEED = 1
KEYS = {}


def seed(n):
    global RNG, SEED
    RNG = random.Random(n)
    SEED = n
    KEYS.clear()


def J(x):
    if isinstance(x, dict):
        return Obj([(k, J(v)) for k, v in x.items()])
    if isinstance(x, list):
        return [J(e) for e in x]
    return x


def cbytes(x):
    return canon(J(x))


def cbytes_unsorted(pairs):
    """Canonical bytes with the member order as written (a deliberately
    non-canonical spelling)."""
    out = b"{"
    first = True
    for k, v in pairs:
        if not first:
            out += b","
        first = False
        out += canon(k) + b":" + canon(J(v))
    return out + b"}"


def hx(b):
    return "0x" + b.hex()


def key(tag):
    if tag not in KEYS:
        KEYS[tag] = RNG.randrange(1, N_ORDER - 1)
    return KEYS[tag]


def addr(tag):
    return hx(address_of(pubkey(key(tag))))


def privhex(tag):
    return "0x%064x" % key(tag)


def H32(i=None):
    return "0x%064x" % (RNG.getrandbits(256) if i is None else i)


def H20():
    return "0x%040x" % RNG.getrandbits(160)


def sig_hex(r, s, v):
    return hx(r.to_bytes(32, "big") + s.to_bytes(32, "big") + bytes([v]))


def sign(priv, digest32):
    r, s, v = sign_digest(priv, digest32)
    return sig_hex(r, s, v)


# --------------------------------------------------------------------------
# s1 and s4.2 of eravon.terms/1, this generator's own reading
# --------------------------------------------------------------------------

DEPLOYER = "0x4e59b44847b379578588920ca78fbf26c0b4956c"
ZERO20 = "0x" + "00" * 20
ZERO32 = "0x" + "00" * 32


def w(x):
    if isinstance(x, bool):
        return (1 if x else 0).to_bytes(32, "big")
    if isinstance(x, int):
        return x.to_bytes(32, "big")
    b = bytes.fromhex(x[2:]) if isinstance(x, str) else x
    return bytes(32 - len(b)) + b


TYPE_OUTCOME = "Outcome(uint16 buyerBps,uint16 sellerBps,uint8 feeBearer)"
TYPE_OFFER_PRIMARY = ("Offer(address sellerKey,address judge,address guild,address sellerPayout,address sellerFund,address token,"
                      "uint256 amount,uint32 deliveryWindow,uint32 confirmWindow,uint32 openWindow,bool buyerMayOpen,"
                      "Outcome[] ladder,bytes32 termsHash,bytes32 amanahHash,bytes32 quoteHash,uint256 nonce,"
                      "uint256 expiry)")
TYPE_OFFER = TYPE_OFFER_PRIMARY + TYPE_OUTCOME
TYPE_ORDER = "OrderTerms(Offer offer,address buyerKey,address buyerPayout,uint64 openBy,address prev)" + TYPE_OFFER
TYPE_DOMAIN = "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"


def th(s):
    return keccak256(s.encode("ascii"))


def hash_rung(r):
    return keccak256(th(TYPE_OUTCOME) + w(r["buyerBps"]) + w(r["sellerBps"]) + w(r["feeBearer"]))


def hash_offer(o):
    ladder = keccak256(b"".join(hash_rung(r) for r in o["ladder"]))
    return keccak256(th(TYPE_OFFER) + w(o["sellerKey"]) + w(o["judge"]) + w(o["guild"]) + w(o["sellerPayout"])
                     + w(o["sellerFund"]) + w(o["token"]) + w(int(o["amount"])) + w(o["deliveryWindow"]) + w(o["confirmWindow"])
                     + w(o["openWindow"]) + w(o["buyerMayOpen"]) + ladder + w(o["termsHash"]) + w(o["amanahHash"])
                     + w(o["quoteHash"]) + w(o["nonce"]) + w(o["expiry"]))


def hash_order(offer_hash, buyer_key, buyer_payout, open_by, prev=ZERO20):
    return keccak256(th(TYPE_ORDER) + offer_hash + w(buyer_key) + w(buyer_payout) + w(open_by) + w(prev))


def domain(chain_id, verifying):
    return keccak256(th(TYPE_DOMAIN) + keccak256(b"ERAVON") + keccak256(b"1") + w(chain_id) + w(verifying))


def digest(chain_id, verifying, struct_hash):
    return keccak256(b"\x19\x01" + domain(chain_id, verifying) + struct_hash)


def initcode(logic):
    return (bytes.fromhex("3d602d80600a3d3981f3363d3d373d3d3d363d73") + bytes.fromhex(logic[2:])
            + bytes.fromhex("5af43d82803e903d91602b57fd5bf3"))


def proxy_codehash(logic):
    return hx(keccak256(initcode(logic)[10:]))


def instance_of(logic, salt):
    return hx(keccak256(b"\xff" + bytes.fromhex(DEPLOYER[2:]) + salt + keccak256(initcode(logic)))[12:])


# --------------------------------------------------------------------------
# quotes
# --------------------------------------------------------------------------

def quote_bytes(token, premiumBps=100, feeUnit=ZERO20, feedToken=ZERO20, feedUnit=ZERO20,
                maxStale=3600, feeFixed=50000000, feeBps=1000, feeMin=50000000, feeMax=500000000, amountCap=0,
                rulingWindow=1209600, offerWindow=86400, lapseDefault=2, autoAccept=1):
    words = [w(token), w(premiumBps), w(feeUnit), w(feedToken), w(feedUnit), w(maxStale), w(feeFixed),
             w(feeBps), w(feeMin), w(feeMax), w(amountCap), w(rulingWindow), w(offerWindow), w(lapseDefault),
             w(autoAccept)]
    return hx(b"".join(words))


def quote_words(qhex, **over):
    """Rewrite words of a quote by index (values as 32-byte ints)."""
    b = bytearray(bytes.fromhex(qhex[2:]))
    for k, v in over.items():
        i = int(k[1:])
        b[32 * i:32 * i + 32] = v.to_bytes(32, "big")
    return hx(bytes(b))


def quote_decoded(qhex):
    """The fifteen fields as the harness's answer object spells them."""
    b = bytes.fromhex(qhex[2:])
    ws = [int.from_bytes(b[32 * i:32 * i + 32], "big") for i in range(15)]
    names = ("token", "premiumBps", "feeUnit", "feedToken", "feedUnit", "maxStale", "feeFixed",
             "feeBps", "feeMin", "feeMax", "amountCap", "rulingWindow", "offerWindow", "lapseDefault", "autoAccept")
    d = {}
    for i, n in enumerate(names):
        if i in (0, 2, 3, 4):
            d[n] = "0x%040x" % ws[i]
        elif i in (6, 8, 9, 10):
            d[n] = str(ws[i])
        elif i == 14:
            d[n] = ws[i] == 1
        else:
            d[n] = ws[i]
    return d


# --------------------------------------------------------------------------
# terms documents
# --------------------------------------------------------------------------

OPEN = b"```eravon.terms/1"
CLOSE = b"```"


def terms_doc(block_bytes, before=b"", after=b"\n"):
    """The fences and the block; s2.2 admits empty lines alone outside them."""
    return before + OPEN + b"\n" + block_bytes + b"\n" + CLOSE + after


def qhash(qhex):
    """keccak256 of a quote's bytes, the quoteHash a document carries."""
    return hx(keccak256(bytes.fromhex(qhex[2:])))


GOODS = "One illustration, 2000 × 3000 px, PNG, 草稿先行。"
DELIVERY = "A download link posted within the delivery window; delivered when the link opens."


def base_block(seller="seller", logic=None, token=None, judges=None, guilds=None, ladder=None,
               chain_id=1, nonce=7, expiry=1800000000, amount="1000000000", delivery=604800, confirm=259200,
               open_window=86400, buyer_may_open=True, goods=None, delivery_text=None, **kw):
    logic = logic or addr("logic")
    token = token or addr("token")
    if judges is None:
        judges = [{"judge": addr("judge"), "amanahHash": H32(), "quoteHash": qhash(quote_bytes(token))}]
    if ladder is None:
        ladder = [{"buyerBps": 0, "sellerBps": 10000, "feeBearer": 0, "note_md": "驳回"},
                  {"buyerBps": 5000, "sellerBps": 5000, "feeBearer": 2},
                  {"buyerBps": 10000, "sellerBps": 0, "feeBearer": 1}]
    d = {"v": "eravon.terms/1", "chainId": chain_id, "logic": logic, "sellerKey": addr(seller),
         "sellerPayout": addr(seller + "-payout"), "sellerFund": addr(seller), "token": token, "amount": amount,
         "deliveryWindow": delivery,
         "confirmWindow": confirm, "openWindow": open_window, "buyerMayOpen": buyer_may_open,
         "goods_md": GOODS if goods is None else goods,
         "delivery_md": DELIVERY if delivery_text is None else delivery_text, "ladder": ladder,
         "judges": judges, "guilds": guilds if guilds is not None else [], "nonce": nonce, "expiry": expiry}
    d.update(kw)
    for k in [k for k, v in d.items() if v is Ellipsis]:
        del d[k]
    return d


def text_hash(text):
    """sha256 of a text's UTF-8 bytes as hex32: a goodsHash or deliveryHash."""
    return hx(hashlib.sha256(text.encode("utf-8")).digest())


def rung_tuple(r):
    return {"buyerBps": r["buyerBps"], "sellerBps": r["sellerBps"], "feeBearer": r["feeBearer"]}


def candidates(block):
    """(judge, guild, amanahHash, quoteHash) per candidate number, judges first."""
    out = []
    for j in block["judges"]:
        out.append((j["judge"], ZERO20, j["amanahHash"], j["quoteHash"]))
    for g in block["guilds"]:
        out.append((ZERO20, g["pointer"][g["pointer"].rindex(":") + 1:], g["amanahHash"], g["quoteHash"]))
    return out


def offer_of(block, terms_hash, c):
    """The offer value for candidate c, as the answer spells it."""
    judge, guild, amanah, qh = candidates(block)[c]
    return {"sellerKey": block["sellerKey"], "judge": judge, "guild": guild, "sellerPayout": block["sellerPayout"],
            "sellerFund": block["sellerFund"], "token": block["token"], "amount": block["amount"], "deliveryWindow": block["deliveryWindow"],
            "confirmWindow": block["confirmWindow"], "openWindow": block["openWindow"],
            "buyerMayOpen": block["buyerMayOpen"], "ladder": [rung_tuple(r) for r in block["ladder"]],
            "termsHash": terms_hash, "amanahHash": amanah, "quoteHash": qh, "nonce": block["nonce"],
            "expiry": block["expiry"]}


def offer_answer(block, o):
    sh = hash_offer(o)
    return {"digest": hx(digest(block["chainId"], block["logic"], sh)), "offer": o, "ok": True,
            "structHash": hx(sh)}


OPEN_BY = 1700086400


def order_values(block, o, buyer_key, buyer_payout, open_by=OPEN_BY, prev=ZERO20):
    sh = hash_offer(o)
    salt = hash_order(sh, buyer_key, buyer_payout, open_by, prev)
    inst = instance_of(block["logic"], salt)
    return {"salt": hx(salt), "instance": inst, "digest": hx(digest(block["chainId"], inst, salt)),
            "offerDigest": hx(digest(block["chainId"], block["logic"], sh))}


def b64url(b):
    return base64.urlsafe_b64encode(b).rstrip(b"=").decode("ascii")


def galeed_canon_inputs():
    """The galeed/1 corpus's check set (galeed-conformance/corpus/canon/, at that corpus's committed seed), read as
    data: (stem, bytes, note, predicted_canon, predicted) per input in path order. The terms corpus carries each as
    the block of a document and the dossier corpus each that passes galeed/1 s3.5 as an entry file, so that the
    canonical form and the entry test the cores carry are measured by that law's own witnesses (eravon.terms/1
    s10.1, eravon.dossier/1 s7.1). No input carries a fence line, which the terms document's s2.2 would read."""
    import json
    root = os.path.join(os.path.dirname(os.path.realpath(__file__)), "..", "galeed-conformance", "corpus")
    m = json.load(open(os.path.join(root, "manifest.json"), encoding="utf-8"))
    out = []
    for c in m["cases"]:
        if c.get("category") != "canon":
            continue
        b = open(os.path.join(root, c["path"]), "rb").read()
        for ln in b.split(b"\n"):
            assert ln not in (OPEN, CLOSE), c["path"]
        stem = os.path.basename(c["path"]).rsplit(".", 1)[0]
        out.append((stem, b, c["note"], c["predicted_canon"], c["predicted"]))
    out.sort(key=lambda x: x[0])
    return out


# --------------------------------------------------------------------------
# s4.5 action messages and the settlement document
# --------------------------------------------------------------------------

ACTION_TYPES = {"ClaimDelivery": "ClaimDelivery(bytes32 manifest)", "Confirm": "Confirm()",
                "Dispute": "Dispute()", "Refund": "Refund()",
                "Settle": "Settle(uint16 buyerBps,uint256 expiry)", "Cancel": "Cancel()"}


def action_hash(name, *args):
    """The struct hash of an action message (arguments as ints or hex32)."""
    t = th(ACTION_TYPES[name])
    return keccak256(t + b"".join(w(a) for a in args))


def action_digest(chain_id, instance, name, *args):
    return digest(chain_id, instance, action_hash(name, *args))


def settle_of(chain_id, instance, buyer_bps, expiry, seller_tag="seller"):
    dg = action_digest(chain_id, instance, "Settle", buyer_bps, expiry)
    return {"v": "eravon.settle/1", "chainId": chain_id, "instance": instance, "buyerBps": buyer_bps,
            "expiry": expiry, "sig": sign(key(seller_tag), dg)}, dg


# --------------------------------------------------------------------------
# manifest writing
# --------------------------------------------------------------------------

class Corpus:
    def __init__(self, root, dirs):
        self.root = root
        self.dirs = dirs
        self.n = {d: 0 for d in dirs}
        self.manifest = []
        self.notes = set()
        for d in dirs:
            p = os.path.join(root, d)
            if os.path.isdir(p):
                for f in os.listdir(p):
                    fp = os.path.join(p, f)
                    if os.path.isfile(fp):
                        os.remove(fp)
                    else:
                        import shutil
                        shutil.rmtree(fp)
            os.makedirs(p, exist_ok=True)

    def write(self, cat, slug, data, ext=".json"):
        self.n[cat] += 1
        name = "%s%03d_%s%s" % (cat[0], self.n[cat], slug, ext)
        path = os.path.join(cat, name)
        with open(os.path.join(self.root, path), "wb") as f:
            f.write(data)
        return path

    def dir(self, cat, slug):
        self.n[cat] += 1
        name = "%s%03d_%s" % (cat[0], self.n[cat], slug)
        path = os.path.join(cat, name)
        os.makedirs(os.path.join(self.root, path), exist_ok=True)
        return path

    def case(self, cmd, args, note, predicted, path=None, slug=None):
        rec = {"cmd": cmd, "args": args, "path": path or (args[-1] if args else "(none)"),
               "note": note, "predicted": predicted}
        if slug is not None:
            rec["slug"] = slug
        self.manifest.append(rec)

    def argv_case(self, argv, note, predicted, path="(none)"):
        self.manifest.append({"argv": argv, "path": path, "note": note, "predicted": predicted})

    def finish(self, seed_n):
        import json
        with open(os.path.join(self.root, "manifest.json"), "w") as f:
            json.dump({"seed": seed_n, "cases": self.manifest}, f, indent=1, sort_keys=True, ensure_ascii=False)
        return len(self.manifest)


def fault(token, index=None, field=None, key=None, inner=None, ok=True):
    d = {"ok": False} if ok else {}
    d["token"] = token
    if index is not None:
        d["index"] = index
    if field is not None:
        d["field"] = field
    if key is not None:
        d["key"] = key
    if inner is not None:
        d["inner"] = inner
    return d


def inner(token, index=None, field=None, key=None, inner_=None):
    return fault(token, index, field, key, inner_, ok=False)


# --------------------------------------------------------------------------
# listings, links, receipts, packs (eravon.link/1), this generator's own reading
# --------------------------------------------------------------------------

def listing_of(block, terms_bytes, seller_tag="seller", seq=3, panel_hash=None, **extra):
    """A listing of a terms document whose block is `block`, the document
    inside it in full, every candidate's offer signed by the seller's key."""
    th = hx(hashlib.sha256(terms_bytes).digest())
    order = {"sellerKey": block["sellerKey"], "sellerPayout": block["sellerPayout"], "sellerFund": block["sellerFund"],
             "amount": block["amount"],
             "deliveryWindow": block["deliveryWindow"], "confirmWindow": block["confirmWindow"],
             "openWindow": block["openWindow"], "buyerMayOpen": block["buyerMayOpen"],
             "ladder": [rung_tuple(r) for r in block["ladder"]], "nonce": block["nonce"], "expiry": block["expiry"]}
    judges = []
    guilds = []
    for c, (judge, guild, amanah, qh) in enumerate(candidates(block)):
        o = offer_of(block, th, c)
        sg = sign(key(seller_tag), digest(block["chainId"], block["logic"], hash_offer(o)))
        if c < len(block["judges"]):
            j = dict(block["judges"][c])
            j["sig"] = sg
            judges.append(j)
        else:
            g = dict(block["guilds"][c - len(block["judges"])])
            g.update({"seq": seq, "panelHash": panel_hash or H32(), "sig": sg})
            guilds.append(g)
    p = {"v": "eravon.listing/1", "chainId": block["chainId"], "token": block["token"], "logic": block["logic"],
         "proxyCodeHash": proxy_codehash(block["logic"]),
         "terms": {"hash": th, "inline_md": terms_bytes.decode("utf-8")}, "order": order, "judges": judges,
         "guilds": guilds}
    p.update(extra)
    return p


def link_of(chain_id, tx, hash_):
    return {"v": "eravon.link/1", "chainId": chain_id, "tx": tx, "hash": hash_}


def link_for(listing, tx=None):
    """A link pointing at a listing's canonical bytes."""
    return link_of(listing["chainId"], tx or H32(), hx(hashlib.sha256(cbytes(listing)).digest()))


def receipt_of(listing, block, c, buyer_tag="buyer", payout=None, open_by=OPEN_BY, prev=ZERO20):
    """A receipt for candidate c of a listing, signed by the buyer."""
    th = listing["terms"]["hash"]
    o = offer_of(block, th, c)
    bp = payout or addr(buyer_tag + "-payout")
    bk = addr(buyer_tag)
    ov = order_values(block, o, bk, bp, open_by, prev)
    sg = sign(key(buyer_tag), bytes.fromhex(ov["digest"][2:]))
    r = {"v": "eravon.receipt/1", "termsHash": th, "candidate": c, "buyerKey": bk, "buyerPayout": bp,
         "openBy": open_by, "prev": prev, "sig": sg}
    if c >= len(listing["judges"]):
        r["seq"] = listing["guilds"][c - len(listing["judges"])]["seq"]
    return r, ov


def attachment_of(text):
    return {"sha256": text_hash(text), "text_md": text}


def pack_of(listing, receipt, ov, link=None, attachments=None):
    p = {"v": "eravon.pack/1", "chainId": listing["chainId"], "instance": ov["instance"], "caseId": ov["salt"],
         "link": link or link_for(listing), "listing": listing, "receipt": receipt}
    if attachments is not None:
        p["attachments"] = attachments
    return p


# --------------------------------------------------------------------------
# galeed/1 entries (frozen law; built with the frozen core's primitives)
# --------------------------------------------------------------------------

def entry(judge_tag, seq, prev, entry_type, body):
    """A signed galeed/1 entry as canonical bytes."""
    from glentry import entry_digest
    env = {"body": body, "entryType": entry_type, "judge": addr(judge_tag), "prev": prev, "seq": seq,
           "spec": "galeed/1"}
    b6 = cbytes(env)
    dg = entry_digest(b6)
    env["sig"] = sign(key(judge_tag), dg)
    return cbytes(env)
