#!/usr/bin/env python3
"""EIP-712 vectors for the contract from the eravon.terms/1 criterion: nine terms documents built with the
corpus generator's helpers under known quote bytes (one with a sellerFund apart from sellerKey), then
`ert offer|order|action` over each candidate, chainId 1.
test/Eravon.vectors.t.sol asserts every value against the contract and drives live cases with signatures over
the criterion's struct hashes. Tooling, run from anywhere:

    python3 eravon/standard/test/vectors/gen-vectors.py [<dir for the documents>]

The documents are written to the directory named (a fresh temporary one otherwise); the vectors land beside
this script as terms-eip712.json."""
import json, os, subprocess, sys, tempfile
HERE = os.path.dirname(os.path.abspath(__file__))
C = os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "eravon-conformance"))
OUT = os.path.join(HERE, "terms-eip712.json")
DOCS = sys.argv[1] if len(sys.argv) > 1 else tempfile.mkdtemp(prefix="eravon-vectors-")
sys.path.insert(0, C)
import gencommon as G
ERT = ["python3", os.path.join(C, "terms-py", "ert.py")]
G.seed(1)

def ert(*args):
    r = subprocess.run(ERT + list(args), capture_output=True, cwd=DOCS)
    assert r.returncode == 0, (args, r.stderr)
    return json.loads(r.stdout)

seller_priv = "0x%064x" % G.key("seller")
buyer_priv = "0x%064x" % G.key("buyer")
bk, bp = G.addr("buyer"), G.addr("buyer-payout")
OPEN_BY = 1850000000
EXPIRY = 1900000000
tok = G.addr("token")
judge = G.addr("judge")
guild = G.addr("guild")

def J(q, amanah=1):
    return {"judge": judge, "amanahHash": G.H32(amanah), "quoteHash": G.qhash(q)}

def Gd(q):
    return {"pointer": "galeed:guild:1:" + guild, "amanahHash": G.H32(2), "quoteHash": G.qhash(q)}

q_auto = G.quote_bytes(tok)
q_manual = G.quote_bytes(tok, autoAccept=0, lapseDefault=0, rulingWindow=86400 * 3, offerWindow=3600)
q_guild = G.quote_bytes(tok, autoAccept=0)
q_priced = G.quote_bytes(tok, feeUnit=G.addr("usd"), feedToken=G.addr("feed-token"), feedUnit=G.addr("feed-unit"))
q_nomin = G.quote_bytes(tok, feeFixed=0, feeMin=0, feeMax=0, premiumBps=0)
q_full = G.quote_bytes(tok, premiumBps=10000, feeBps=10000)
ladder16 = [{"buyerBps": i * 10000 // 15, "sellerBps": 10000 - i * 10000 // 15, "feeBearer": i % 3} for i in range(16)]
ladder2 = [{"buyerBps": 0, "sellerBps": 10000, "feeBearer": 0}, {"buyerBps": 10000, "sellerBps": 0, "feeBearer": 1}]

docs = [
    ("v1_default.md", G.base_block(judges=[J(q_auto)], guilds=[Gd(q_guild)], expiry=EXPIRY), {0: q_auto, 1: q_guild}),
    ("v2_manual_two_rungs.md", G.base_block(judges=[J(q_manual, 3)], ladder=ladder2, buyer_may_open=False, expiry=EXPIRY, delivery=3600, confirm=3600, open_window=600, nonce=1), {0: q_manual}),
    ("v3_priced.md", G.base_block(judges=[J(q_priced)], expiry=EXPIRY, amount="250000000000000000000"), {0: q_priced}),
    ("v4_max_amount.md", G.base_block(judges=[J(q_auto)], expiry=EXPIRY, amount=str((2**256 - 1) // 10000), nonce=2**53 - 1), {0: q_auto}),
    ("v5_sixteen_rungs.md", G.base_block(judges=[J(q_auto)], ladder=ladder16, expiry=EXPIRY, delivery=2**32 - 1, confirm=2**32 - 1, open_window=2**32 - 1, nonce=0), {0: q_auto}),
    ("v6_goods_hash.md", G.base_block(judges=[J(q_nomin)], expiry=EXPIRY, goods_md=Ellipsis, goodsHash=G.text_hash("hashed goods"), delivery_md=Ellipsis, deliveryHash=G.text_hash("hashed delivery"), amount="1"), {0: q_nomin}),
    ("v7_full_premium.md", G.base_block(judges=[J(q_full)], expiry=EXPIRY, amount="123456789"), {0: q_full}),
    ("v8_two_judges.md", G.base_block(judges=[J(q_auto), {"judge": G.addr("judge2"), "amanahHash": G.H32(4), "quoteHash": G.qhash(q_manual)}], expiry=EXPIRY, nonce=99), {0: q_auto, 1: q_manual}),
    ("v9_company_fund.md", G.base_block(judges=[J(q_auto)], expiry=EXPIRY, sellerFund=G.addr("company-wallet"), nonce=7), {0: q_auto}),
]
out = {"_": "EIP-712 vectors from the eravon.terms/1 criterion: ert offer|order|action over nine terms documents, one with a sellerFund apart from sellerKey (chainId 1, domain version 1, logic " + G.addr("logic") + "); the quote bytes are the ones each candidate's quoteHash names; the two private keys are the corpus generator's test keys (seed 1). test/Eravon.vectors.t.sol asserts every value against the contract.",
       "sellerPriv": seller_priv, "buyerPriv": buyer_priv, "vectors": []}
for name, blk, quotes in docs:
    body = G.cbytes(blk)
    open(os.path.join(DOCS, name), "wb").write(G.terms_doc(body))
    chk = ert("check", name)
    assert chk.get("ok"), (name, chk)
    for c, qhex in quotes.items():
        o = ert("offer", name, str(c))
        assert o.get("ok"), (name, c, o)
        od = ert("order", name, str(c), bk, bp, str(OPEN_BY), G.ZERO20)
        assert od.get("ok"), od
        acts = {}
        for an, extra in (("ClaimDelivery", ["0x" + "11" * 32]), ("Confirm", []), ("Dispute", []), ("Refund", []), ("Settle", ["5000", "1860000000"]), ("Cancel", [])):
            a = ert("action", name, str(c), bk, bp, str(OPEN_BY), G.ZERO20, an, *extra)
            assert a.get("ok"), a
            acts[an] = {"structHash": a["structHash"], "digest": a["digest"]}
        # the same order as a later stage: prev nonzero, another salt, instance and digest, the actions under it (terms s4.4)
        PREV = G.addr("stage-before")
        odp = ert("order", name, str(c), bk, bp, str(OPEN_BY), PREV)
        assert odp.get("ok"), odp
        ap = ert("action", name, str(c), bk, bp, str(OPEN_BY), PREV, "Confirm")
        assert ap.get("ok"), ap
        later = {"prev": PREV, "salt": odp["salt"], "instance": odp["instance"], "orderDigest": odp["digest"], "confirmDigest": ap["digest"]}
        out["vectors"].append({"doc": name, "c": c, "logic": blk["logic"], "chainId": blk["chainId"], "offer": o["offer"], "quote": qhex, "later": later,
                               "offerStructHash": o["structHash"], "offerDigest": o["digest"], "buyerKey": bk, "buyerPayout": bp,
                               "openBy": OPEN_BY, "prev": G.ZERO20, "salt": od["salt"], "instance": od["instance"], "orderDigest": od["digest"],
                               "actions": acts, "named": o["offer"]["guild"] == G.ZERO20})
json.dump(out, open(OUT, "w"), indent=1, sort_keys=True)
print("vectors:", len(out["vectors"]), "named:", sum(1 for v in out["vectors"] if v["named"]))
