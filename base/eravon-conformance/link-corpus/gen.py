#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
gen.py - corpus generator for the `eravon.link/1` convergence run.

Written from docs/eravon-link-v1.md, docs/eravon-terms-v1.md and
HARNESS-LINK.md.  Every prediction is written by hand from the texts (or
`null` where the case is harness misuse); the digests an accepted answer
carries are computed by ../gencommon.py's own typed-data and CREATE2 code,
a third opinion beside the criterion and the candidates, never
authoritative.

Usage:  python3 gen.py --seed 1
Populates ./listings, ./links, ./docs, ./receipts, ./packs, ./texts,
./terms, ./settles and writes manifest.json.
"""
import argparse
import base64
import hashlib
import json
import os
import random
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, ".."))
import gencommon as G  # noqa: E402
from gencommon import J, cbytes, hx, addr, key, H32, H20, fault, inner, ZERO20, ZERO32  # noqa: E402

C = None
OB = G.OPEN_BY
P_PRETTY = None
P_ROOT_STRING = None


def sha(b):
    return hx(hashlib.sha256(b).digest())


def b64(b):
    return base64.urlsafe_b64encode(b).rstrip(b"=")


def W(cat, slug, data, ext=".json"):
    return C.write(cat, slug, data, ext=ext)


def up(h):
    return h.upper().replace("0X", "0x")


def copy(d):
    return json.loads(json.dumps(d))


def S(slug, b, note, predicted, cmd="check"):
    p = W("listings", slug, b)
    C.case(cmd, [p], note, predicted, path=p)
    return p


B64_ALPHABET = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"


def b64_mutations(cmd, prefix, body, slug, tok, sec, ok):
    """s3.2's base64 rule as the receipt transport applies it too (s4.3), from one table: the empty
    remainder, padding, a length of 1 mod 4 and nonzero trailing bits, each a text under `cmd`
    answering the transport's own token; `ok` is the accepted answer for the body where padding is a no-op."""
    t = W("texts", slug + "_empty", prefix, ext=".txt")
    C.case(cmd, [t], sec + ": an empty segment", fault(tok), path=t)
    t = W("texts", slug + "_padded", prefix + b64(body) + b"=", ext=".txt")
    C.case(cmd, [t], sec + ": a padding character is outside the alphabet", fault(tok), path=t)
    seg = b64(body)
    t = W("texts", slug + "_len_mod4", prefix + seg + b"A" * ((1 - len(seg)) % 4), ext=".txt")
    C.case(cmd, [t], sec + ": a length of 1 mod 4", fault(tok), path=t)
    seg = bytearray(seg)
    idx = B64_ALPHABET.index(seg[-1])
    rem = len(seg) % 4
    if rem == 2:
        seg[-1] = B64_ALPHABET[idx | 0x0f] if (idx & 0x0f) == 0 else B64_ALPHABET[idx ^ 0x01]
        text = prefix + bytes(seg)
    elif rem == 3:
        seg[-1] = B64_ALPHABET[idx ^ 0x01]
        text = prefix + bytes(seg)
    else:
        text = prefix + b64(body + b"\x01")[:-1] + b"B"
    t = W("texts", slug + "_trailing_bits", text, ext=".txt")
    C.case(cmd, [t], "s1: base64url is canonical, unused trailing bits zero", fault(tok), path=t)
    t = W("texts", slug + "_newline", prefix + b64(body) + b"\n", ext=".txt")
    C.case(cmd, [t], sec + ": a byte outside the alphabet, a trailing line feed", fault(tok), path=t)
    t = W("texts", slug + "_space_prefix", b" " + prefix + b64(body), ext=".txt")
    C.case(cmd, [t], sec + ": the prefix begins the text", fault(tok.replace("_B64", "_PREFIX")), path=t)


def listing_offer_digest(listing, c):
    """The generator's own reading of link s2.4 and terms s4.2: candidate c's
    offer digest under the logic's domain, from the listing's own members."""
    o = listing["order"]
    els = listing["judges"] + listing["guilds"]
    el = els[c]
    if c < len(listing["judges"]):
        judge, guild = el["judge"], ZERO20
    else:
        judge, guild = ZERO20, el["pointer"][el["pointer"].rindex(":") + 1:]
    off = {"sellerKey": o["sellerKey"], "judge": judge, "guild": guild, "sellerPayout": o["sellerPayout"],
           "sellerFund": o["sellerFund"], "token": listing["token"], "amount": o["amount"], "deliveryWindow": o["deliveryWindow"],
           "confirmWindow": o["confirmWindow"], "openWindow": o["openWindow"], "buyerMayOpen": o["buyerMayOpen"],
           "ladder": o["ladder"], "termsHash": listing["terms"]["hash"], "amanahHash": el["amanahHash"],
           "quoteHash": el["quoteHash"], "nonce": o["nonce"], "expiry": o["expiry"]}
    return hx(G.digest(listing["chainId"], listing["logic"], G.hash_offer(off)))


def oks(listing):
    by_call = [{"candidate": c, "digest": listing_offer_digest(listing, c)}
               for c, el in enumerate(listing["judges"] + listing["guilds"]) if el.get("sig") == ""]
    return {"byCall": by_call, "candidates": len(listing["judges"]) + len(listing["guilds"]), "ok": True,
            "termsHash": listing["terms"]["hash"]}


def resign(d, tag="seller"):
    """Re-sign every candidate of a listing under the listing's own values, by the key `tag` names."""
    for c, el in enumerate(d["judges"] + d["guilds"]):
        el["sig"] = G.sign(key(tag), bytes.fromhex(listing_offer_digest(d, c)[2:]))
    return d


def main_terms():
    """The terms document most cases share."""
    tok = addr("token")
    gj = {"judge": addr("judge"), "amanahHash": H32(), "quoteHash": G.qhash(G.quote_bytes(tok))}
    gj2 = {"judge": addr("judge2"), "amanahHash": H32(), "quoteHash": G.qhash(G.quote_bytes(tok, amountCap=1500000000)), "source": "galeed:guild:1:" + addr("guild")}
    gg = {"pointer": "galeed:guild:1:" + addr("guild"), "amanahHash": H32(), "quoteHash": G.qhash(G.quote_bytes(tok, autoAccept=0))}
    blk = G.base_block(judges=[gj, gj2], guilds=[gg])
    tb = G.terms_doc(cbytes(blk))
    return blk, tb


# ==========================================================================
# the listing: check, bind
# ==========================================================================

def gen_listing(blk, tb):
    s0 = G.listing_of(blk, tb)
    b = cbytes(s0)
    S("ok_full", b, "s2.5: two judges (one with a source) and a guild, the document inline, every offer signed", oks(s0))
    sid = H32()
    for st in (1, 2):
        blk_s = G.base_block(judges=[blk["judges"][0]], nonce=20 + st, series={"id": sid, "stage": st, "stages": 2})
        tb_s = G.terms_doc(cbytes(blk_s))
        s = G.listing_of(blk_s, tb_s)
        S("ok_series_stage_%d" % st, cbytes(s), "s2.6, s10: stage %d of a two-stage series; the documents share one series.id" % st, oks(s))
    blk_sb = G.base_block(judges=[blk["judges"][0]], series={"id": sid, "stage": 2, "stages": 1})
    tb_sb = G.terms_doc(cbytes(blk_sb))
    S("terms_doc_series", cbytes(G.listing_of(blk_sb, tb_sb)), "s2.5: E_LISTING_TERMS_DOC carrying E_TERMS_SERIES", fault("E_LISTING_TERMS_DOC", inner=inner("E_TERMS_SERIES")))
    # ---- galeed/1 s3 on this surface (s11.1): every input of the galeed/1 corpus's check set as a listing file,
    # predicted as that corpus predicts it (no test of this law decides before canonical(b)), a canonical value then
    # s2.5's root (E_LISTING) or members (E_LISTING_MISSING)
    for gp, gb, gnote, gcanon, gcheck in G.galeed_canon_inputs():
        if gcanon != "ok":
            gpred = fault(gcanon)
        elif gcheck == "E_NOT_CANONICAL":
            gpred = fault("E_NOT_CANONICAL")
        else:
            gpred = fault("E_LISTING_MISSING" if isinstance(json.loads(gb.decode("utf-8")), dict) else "E_LISTING")
        S("gl1_" + gp, gb, "galeed/1 corpus %s as a listing (%s); gl1 check: %s" % (gp, gnote, gcheck), gpred)
    blk_j = dict(blk, judges=[blk["judges"][0]], guilds=[])
    tb_j = G.terms_doc(cbytes(blk_j))
    s = G.listing_of(blk_j, tb_j)
    S("ok_one_judge", cbytes(s), "s2.5: one candidate", oks(s))
    blk_gg = dict(blk, judges=[], guilds=[blk["guilds"][0]])
    tb_gg = G.terms_doc(cbytes(blk_gg))
    s = G.listing_of(blk_gg, tb_gg)
    S("ok_guild_only", cbytes(s), "s2.5: a guild alone", oks(s))
    blk_h = G.base_block(judges=[blk["judges"][0]], goods=..., goodsHash=G.text_hash(G.GOODS))
    tb_h = G.terms_doc(cbytes(blk_h))
    s = G.listing_of(blk_h, tb_h)
    S("ok_hashed_goods", cbytes(s), "s2.6: a document whose goods segment is a hash passes; the text travels apart", oks(s))
    blk_hh = G.base_block(judges=[blk["judges"][0]], goods=..., goodsHash=G.text_hash(G.GOODS), delivery_text=..., deliveryHash=G.text_hash(G.DELIVERY))
    tb_hh = G.terms_doc(cbytes(blk_hh))
    s = G.listing_of(blk_hh, tb_hh)
    S("ok_both_hashed", cbytes(s), "s2.6: both segments as hashes", oks(s))
    blk_f = G.base_block(judges=[blk["judges"][0]], buyer_may_open=False, open_window=0)
    tb_f = G.terms_doc(cbytes(blk_f))
    s = G.listing_of(blk_f, tb_f)
    S("ok_buyer_may_open_false", cbytes(s), "s2.2: buyerMayOpen false and openWindow 0 in the order object", oks(s))
    blk_c = G.base_block(chain_id=5, judges=[dict(blk["judges"][0], source="galeed:guild:5:" + addr("guild"))], guilds=[dict(blk["guilds"][0], pointer="galeed:guild:5:" + addr("guild"))])
    tb_c = G.terms_doc(cbytes(blk_c))
    s = G.listing_of(blk_c, tb_c)
    S("ok_chain_5", cbytes(s), "s2.5: a chain-5 listing of a chain-5 document", oks(s))
    blk_w = G.base_block(judges=[blk["judges"][0]], delivery=2 ** 32 - 1, confirm=2 ** 32 - 1, open_window=2 ** 32 - 1)
    tb_w = G.terms_doc(cbytes(blk_w))
    s = G.listing_of(blk_w, tb_w)
    S("ok_windows_max", cbytes(s), "s2.2: the three windows at 2^32 - 1 pass", oks(s))
    ladder16 = [{"buyerBps": i * 10000 // 15, "sellerBps": 10000 - i * 10000 // 15, "feeBearer": i % 3} for i in range(16)]
    ladder16[0]["buyerBps"], ladder16[0]["sellerBps"] = 0, 10000
    ladder16[15]["buyerBps"], ladder16[15]["sellerBps"] = 10000, 0
    blk_16 = G.base_block(judges=[blk["judges"][0]], ladder=ladder16)
    tb_16 = G.terms_doc(cbytes(blk_16))
    s = G.listing_of(blk_16, tb_16)
    S("ok_16_rungs", cbytes(s), "s2.2: sixteen rungs pass", oks(s))

    base = lambda: G.listing_of(blk, tb)  # noqa: E731
    global P_PRETTY
    P_PRETTY = S("canon_pretty", json.dumps(base(), indent=1, sort_keys=True, ensure_ascii=False).encode("utf-8"), "galeed/1 s3.5 test 6", fault("E_NOT_CANONICAL"))
    S("canon_json", b"{", "galeed/1 s3.5 test 2", fault("E_JSON"))
    S("canon_value_charset", cbytes(dict(base(), v="eravon.listing/1é")), "galeed/1 s3.3: v is a skeleton string", fault("E_VALUE_CHARSET"))
    S("root_array", b"[]", "s2.5: root not an object", fault("E_LISTING"))
    d = base(); del d["order"]
    S("missing", cbytes(d), "s2.5: a required member absent", fault("E_LISTING_MISSING"))
    d = base(); d["mirrors"] = []
    S("closed_extra_array", cbytes(d), "s2.5: an array member beyond the nine", fault("E_LISTING_CLOSED"))
    d = base(); d["pageHash"] = H32()
    S("closed_extra_hash", cbytes(d), "s2.5: a digest member beyond the nine", fault("E_LISTING_CLOSED"))
    d = base(); del d["judges"]; d["extra"] = 1
    S("missing_before_closed", cbytes(d), "s2.5 order", fault("E_LISTING_MISSING"))
    S("v_link", cbytes(dict(base(), v="eravon.link/1")), "s2.5: a listing's v is eravon.listing/1, not the link's", fault("E_LISTING_V"))
    S("chain_string", cbytes(dict(base(), chainId="1")), "s2.5: chainId an int", fault("E_LISTING_CHAIN"))
    S("token_zero", cbytes(dict(base(), token=ZERO20)), "s2.5: token nonzero", fault("E_LISTING_TOKEN"))
    S("logic_zero", cbytes(dict(base(), logic=ZERO20)), "s2.5: logic nonzero", fault("E_LISTING_LOGIC"))
    S("token_upper", cbytes(dict(base(), token=addr("token").upper().replace("0X", "0x"))), "s2.5: token hex20, lowercase", fault("E_LISTING_TOKEN"))
    S("logic_short", cbytes(dict(base(), logic="0x1")), "s2.5: logic hex20", fault("E_LISTING_LOGIC"))
    S("codehash_wrong", cbytes(dict(base(), proxyCodeHash=H32())), "s2.5: proxyCodeHash equals proxyCodeHash(logic)", fault("E_LISTING_CODEHASH"))
    S("codehash_short", cbytes(dict(base(), proxyCodeHash="0x12")), "s2.5: proxyCodeHash hex32", fault("E_LISTING_CODEHASH"))
    S("codehash_for_other_logic", cbytes(dict(base(), logic=addr("logic2"))), "s2.5: the code hash of another logic", fault("E_LISTING_CODEHASH"))
    S("adj_logic_zero_before_codehash_form", cbytes(dict(base(), logic=ZERO20, proxyCodeHash="0x12")), "s2.5 order: logic's rule before proxyCodeHash's form", fault("E_LISTING_LOGIC"))
    d = base(); d["terms"] = "x"
    S("terms_string", cbytes(d), "s2.5: terms an object", fault("E_LISTING_TERMS"))
    d = base(); d["terms"] = {"hash": d["terms"]["hash"]}
    S("terms_missing_inline", cbytes(d), "s2.1: terms has exactly hash and inline_md; the text is required", fault("E_LISTING_TERMS"))
    d = base(); d["terms"]["uri"] = "https://x"
    S("terms_closed_uri", cbytes(d), "s2.1: no uri member; a listing carries the text, not a place to fetch it", fault("E_LISTING_TERMS"))
    d = base(); d["terms"]["hash"] = "0x12"
    S("terms_hash_short", cbytes(d), "s2.5: hash hex32", fault("E_LISTING_TERMS"))
    d = base(); d["terms"]["inline_md"] = 5
    S("terms_inline_int", cbytes(d), "s2.1: inline_md a string", fault("E_LISTING_TERMS"))
    d = base(); d["terms"]["hash"] = ZERO32
    S("terms_hash_zero", cbytes(d), "s2.1: hash nonzero (terms/1 s4.1's guard)", fault("E_LISTING_TERMS"))
    d = base(); d["terms"]["inline_md"] = "not the text"
    S("terms_hash_mismatch", cbytes(d), "s2.5: E_LISTING_TERMS_HASH", fault("E_LISTING_TERMS_HASH"))
    d = base(); d["terms"]["inline_md"] = tb.decode("utf-8") + "\n\n"
    S("terms_hash_newline", cbytes(d), "s2.5: one byte more is another digest", fault("E_LISTING_TERMS_HASH"))
    d = base(); d["terms"]["hash"] = sha(tb[:-1]); d["terms"]["inline_md"] = tb[:-1].decode("utf-8")
    S("ok_no_trailing_newline_text", cbytes(resign(d)), "s2.1: the text without its last line feed is another document; hash and text agree, every candidate re-signed", oks(d))
    nb = b"# Terms\n\nno fence\n"
    d = base(); d["terms"] = {"hash": sha(nb), "inline_md": nb.decode("utf-8")}
    S("terms_doc_no_block", cbytes(d), "s2.5: E_LISTING_TERMS_DOC carrying the terms fault", fault("E_LISTING_TERMS_DOC", inner=inner("E_TERMS_BLOCK")))
    blk_bad = G.base_block(judges=[blk["judges"][0]], ladder=[{"buyerBps": 0, "sellerBps": 10000, "feeBearer": 0}, {"buyerBps": 10001, "sellerBps": 0, "feeBearer": 1}])
    tbb = G.terms_doc(cbytes(blk_bad))
    d = base(); d["terms"] = {"hash": sha(tbb), "inline_md": tbb.decode("utf-8")}
    S("terms_doc_rung_index", cbytes(d), "s2.5: the carried terms fault keeps its index", fault("E_LISTING_TERMS_DOC", inner=inner("E_RUNG_BUYER", 1)))
    ob = b"x\n" + tb
    d = base(); d["terms"] = {"hash": sha(ob), "inline_md": ob.decode("utf-8")}
    S("terms_doc_outside", cbytes(d), "terms/1 s2.2: text outside the fences refuses the document", fault("E_LISTING_TERMS_DOC", inner=inner("E_TERMS_OUTSIDE")))
    d = base(); d["order"] = "x"
    S("order_string", cbytes(d), "s2.5: order an object", fault("E_LISTING_ORDER_MISSING"))
    d = base(); del d["order"]["openWindow"]
    S("order_missing", cbytes(d), "s2.5: an order member absent (openWindow)", fault("E_LISTING_ORDER_MISSING"))
    d = base(); d["order"]["milestone"] = 0
    S("order_closed_extra", cbytes(d), "s2.2: a member beyond the eleven", fault("E_LISTING_ORDER_CLOSED"))
    d = base(); d["order"]["sellerKey"] = ZERO20
    S("order_seller_zero", cbytes(d), "s2.2: sellerKey nonzero", fault("E_LISTING_SELLER"))
    d = base(); d["order"]["sellerPayout"] = "0x1"
    S("order_payout", cbytes(d), "s2.2: sellerPayout hex20", fault("E_LISTING_PAYOUT"))
    d = base(); d["order"]["amount"] = "0"
    S("order_amount_zero", cbytes(d), "s2.2: amount nonzero", fault("E_LISTING_AMOUNT"))
    d = base(); d["order"]["amount"] = 5
    S("order_amount_number", cbytes(d), "s2.2: amount a dec256 string", fault("E_LISTING_AMOUNT"))
    d = base(); d["order"]["sellerKey"] = d["order"]["sellerKey"].upper().replace("0X", "0x")
    S("order_seller_upper", cbytes(d), "s2.2: sellerKey hex20, lowercase", fault("E_LISTING_SELLER"))
    d = base(); d["order"]["sellerPayout"] = ZERO20
    S("order_payout_zero", cbytes(d), "s2.2: sellerPayout nonzero", fault("E_LISTING_PAYOUT"))
    d = base(); d["order"]["sellerFund"] = "0x1"
    S("order_fund", cbytes(d), "s2.2: sellerFund hex20", fault("E_LISTING_FUND"))
    d = base(); d["order"]["sellerFund"] = ZERO20
    S("order_fund_zero", cbytes(d), "s2.2: sellerFund nonzero", fault("E_LISTING_FUND"))
    blk_cf = G.base_block(judges=[blk["judges"][0]], sellerFund=addr("company-wallet"))
    tb_cf = G.terms_doc(cbytes(blk_cf))
    s_cf = G.listing_of(blk_cf, tb_cf)
    S("ok_company_fund", cbytes(s_cf), "s2.2: sellerFund an address other than sellerKey (a company's wallet) binds and passes", oks(s_cf))
    d = G.listing_of(blk_cf, tb_cf); resign(d, "company-wallet")
    S("sig_by_fund", cbytes(d), "s2.5: the candidate signed by the fund's key; verifyOffer reads sellerKey alone", fault("E_LISTING_SIG", 0, inner=inner("E_SIG_SIGNER")))
    d = base(); d["order"]["amount"] = "0" + d["order"]["amount"]
    S("order_amount_leading_zero", cbytes(d), "s1 dec256: no leading zero; an integer compare would pass this", fault("E_LISTING_AMOUNT"))
    d = base(); d["order"]["amount"] = str(2 ** 256)
    S("order_amount_2p256", cbytes(d), "s1 dec256: at most 2^256 - 1", fault("E_LISTING_AMOUNT"))
    d = base(); d["order"]["amount"] = str(2 ** 256 - 1)
    S("order_amount_max", cbytes(d), "s1 dec256 at 2^256 - 1 passes the form; terms/1 s2.3's bound reaches it through the binding alone", fault("E_LISTING_BIND", inner=inner("BIND_ORDER")))
    d = base(); d["order"]["deliveryWindow"] = "86400"
    S("order_delivery_string", cbytes(d), "s2.2: deliveryWindow an int", fault("E_LISTING_DELIVERY_WINDOW"))
    d = base(); d["order"]["confirmWindow"] = 2 ** 32
    S("order_confirm_2p32", cbytes(d), "s2.2: confirmWindow at most 2^32 - 1", fault("E_LISTING_CONFIRM_WINDOW"))
    d = base(); d["order"]["openWindow"] = 2 ** 32
    S("order_openwindow_2p32", cbytes(d), "s2.2: openWindow at most 2^32 - 1", fault("E_LISTING_OPENWINDOW"))
    d = base(); d["order"]["ladder"] = [{"buyerBps": i * 10000 // 16, "sellerBps": 10000 - i * 10000 // 16, "feeBearer": 0} for i in range(17)]
    S("order_ladder_17", cbytes(d), "s2.2: at most sixteen rungs", fault("E_LISTING_LADDER"))
    d = base(); d["order"]["deliveryWindow"] = 2 ** 32
    S("order_delivery", cbytes(d), "s2.2: deliveryWindow u32", fault("E_LISTING_DELIVERY_WINDOW"))
    d = base(); d["order"]["confirmWindow"] = None
    S("order_confirm", cbytes(d), "s2.2: confirmWindow u32", fault("E_LISTING_CONFIRM_WINDOW"))
    d = base(); d["order"]["openWindow"] = "86400"
    S("order_openwindow", cbytes(d), "s2.2: openWindow u32", fault("E_LISTING_OPENWINDOW"))
    d = base(); d["order"]["buyerMayOpen"] = 1
    S("order_buyeropen_int", cbytes(d), "s2.2: buyerMayOpen a bool", fault("E_LISTING_BUYEROPEN"))
    d = base(); d["order"]["ladder"] = d["order"]["ladder"][:1]
    S("order_ladder_one", cbytes(d), "s2.2: two to sixteen rungs", fault("E_LISTING_LADDER"))
    d = base(); d["order"]["ladder"][1]["note_md"] = "x"
    S("order_rung_note", cbytes(d), "s2.2: note_md is beyond the list here", fault("E_RUNG_CLOSED", 1))
    d = base(); d["order"]["ladder"][1]["buyerBps"] = 10001
    S("order_rung_buyer", cbytes(d), "terms/1 s2.5: buyerBps at most 10000, with its index", fault("E_RUNG_BUYER", 1))
    d = base(); d["order"]["ladder"][0]["buyerBps"] = 1; d["order"]["ladder"][0]["sellerBps"] = 9999
    S("order_ladder_first", cbytes(d), "terms/1 s2.5: the first rung", fault("E_LADDER_FIRST"))
    d = base(); d["order"]["ladder"][-1]["buyerBps"] = 9999; d["order"]["ladder"][-1]["sellerBps"] = 1
    S("order_ladder_last", cbytes(d), "terms/1 s2.5: the last rung", fault("E_LADDER_LAST"))
    d = base(); d["order"]["ladder"][1]["buyerBps"] = 9000; d["order"]["ladder"][1]["sellerBps"] = 1000
    d["order"]["ladder"].insert(2, {"buyerBps": 8000, "sellerBps": 2000, "feeBearer": 2})
    S("order_ladder_order", cbytes(d), "terms/1 s2.5: no rung below its predecessor", fault("E_LADDER_ORDER", 2))
    d = base(); d["order"]["nonce"] = "7"
    S("order_nonce", cbytes(d), "s2.2: nonce an int", fault("E_LISTING_NONCE"))
    d = base(); d["order"]["expiry"] = []
    S("order_expiry", cbytes(d), "s2.2: expiry an int", fault("E_LISTING_EXPIRY"))
    for slug, muts, tok in (
            ("adj_order_missing_before_closed", {"nonce": None, "x": 1}, "E_LISTING_ORDER_MISSING"),
            ("adj_order_closed_before_seller", {"x": 1, "sellerKey": ZERO20}, "E_LISTING_ORDER_CLOSED"),
            ("adj_order_seller_before_payout", {"sellerKey": ZERO20, "sellerPayout": "0x1"}, "E_LISTING_SELLER"),
            ("adj_order_payout_before_amount", {"sellerPayout": "0x1", "amount": "0"}, "E_LISTING_PAYOUT"),
            ("adj_order_payout_before_fund", {"sellerPayout": "0x1", "sellerFund": "0x1"}, "E_LISTING_PAYOUT"),
            ("adj_order_fund_before_amount", {"sellerFund": "0x1", "amount": "0"}, "E_LISTING_FUND"),
            ("adj_order_amount_before_delivery", {"amount": "0", "deliveryWindow": 2 ** 32}, "E_LISTING_AMOUNT"),
            ("adj_order_delivery_before_confirm", {"deliveryWindow": 2 ** 32, "confirmWindow": "x"}, "E_LISTING_DELIVERY_WINDOW"),
            ("adj_order_confirm_before_openwindow", {"confirmWindow": "x", "openWindow": "y"}, "E_LISTING_CONFIRM_WINDOW"),
            ("adj_order_openwindow_before_buyeropen", {"openWindow": "y", "buyerMayOpen": 0}, "E_LISTING_OPENWINDOW"),
            ("adj_order_buyeropen_before_ladder", {"buyerMayOpen": 0, "ladder": []}, "E_LISTING_BUYEROPEN"),
            ("adj_order_ladder_before_nonce", {"ladder": [], "nonce": "7"}, "E_LISTING_LADDER"),
            ("adj_order_nonce_before_expiry", {"nonce": "7", "expiry": "x"}, "E_LISTING_NONCE")):
        d = base()
        for k_, v_ in muts.items():
            if v_ is None:
                del d["order"][k_]
            else:
                d["order"][k_] = v_
        S(slug, cbytes(d), "s2.5 order inside the order object", fault(tok))
    d = base(); d["judges"] = {}
    S("judges_object", cbytes(d), "s2.5: judges an array", fault("E_LISTING_JUDGES"))
    d = base(); del d["judges"][0]["sig"]
    S("judge_missing_sig", cbytes(d), "s2.3: sig required", fault("E_JUDGE_MISSING", 0))
    d = base(); d["judges"][0]["sig"] = "0x12"
    S("judge_sig_form", cbytes(d), "s2.5: E_JUDGE_SIG", fault("E_JUDGE_SIG", 0))
    d = base(); d["judges"][0]["extra"] = 1
    S("judge_closed", cbytes(d), "s2.3: a member beyond the five", fault("E_JUDGE_CLOSED", 0))
    d = base(); d["judges"][0]["quote"] = G.quote_bytes(addr("token"))
    S("judge_closed_quote", cbytes(d), "terms/1 s2.5: the quote bytes travel in no document", fault("E_JUDGE_CLOSED", 0))
    d = base(); d["judges"][1]["judge"] = d["judges"][0]["judge"]
    S("judge_dup", cbytes(d), "s2.5: E_LISTING_DUP_JUDGE", fault("E_LISTING_DUP_JUDGE", 1))
    d = base(); d["judges"][1]["quoteHash"] = ZERO32
    S("judge_quote_zero", cbytes(d), "terms/1 s2.5: quoteHash nonzero, with its index", fault("E_JUDGE_QUOTE", 1))
    d = base(); d["judges"][0] = "x"
    S("judge_string", cbytes(d), "terms/1 s2.5: a judge element an object, E_JUDGE with its index", fault("E_JUDGE", 0))
    d = base(); d["judges"][0]["judge"] = ZERO20
    S("judge_zero", cbytes(d), "terms/1 s2.5: a zero judge address", fault("E_JUDGE_ADDRESS", 0))
    d = base(); d["order"]["sellerKey"] = d["judges"][0]["judge"]
    S("judge_is_listing_seller", cbytes(d), "s2.3: the inherited table reads the listing's own sellerKey, which judge 0 equals here", fault("E_JUDGE_ADDRESS", 0))
    d = base(); d["chainId"] = 5
    S("chain_5_listing_chain_1_pointers", cbytes(d), "s2.3: the inherited table reads the listing's own chainId; the judge's source pointer names chain 1", fault("E_JUDGE_SOURCE", 1))
    d = base(); d["chainId"] = 5; d["judges"][1]["source"] = "galeed:guild:5:" + addr("guild"); d["guilds"][0]["pointer"] = "galeed:guild:5:" + addr("guild")
    S("chain_5_listing_chain_5_pointers", cbytes(d), "s2.3 and s7: pointers of the listing's chain pass the tables; the binding then finds the document's chain differs", fault("E_LISTING_BIND", inner=inner("BIND_CHAIN")))
    d = base(); d["guilds"][0] = 1
    S("guild_string", cbytes(d), "terms/1 s2.5: a guild element an object, E_GUILD with its index", fault("E_GUILD", 0))
    d = base(); d["guilds"][0]["pointer"] = "galeed:guild:1:" + ZERO20
    S("guild_pointer_zero", cbytes(d), "terms/1 s2.5: a zero pointer address", fault("E_GUILD_POINTER", 0))
    d = base(); d["order"]["sellerKey"] = d["guilds"][0]["pointer"].split(":")[-1]
    S("guild_pointer_is_listing_seller", cbytes(d), "s2.3: the inherited guild table reads the listing's own sellerKey, which the pointer's address equals here", fault("E_GUILD_POINTER", 0))
    d = base(); d["chainId"] = 5; d["judges"][1]["source"] = "galeed:guild:5:" + addr("guild")
    S("guild_pointer_other_chain", cbytes(d), "s2.3: the inherited guild table reads the listing's own chainId; the judge's source names chain 5 and passes, the guild's pointer names chain 1", fault("E_GUILD_POINTER", 0))
    d = base(); d["guilds"][0]["amanahHash"] = ZERO32
    S("guild_amanah_zero", cbytes(d), "terms/1 s2.5: a guild's amanahHash nonzero", fault("E_GUILD_AMANAH", 0))
    d = base(); d["order"]["ladder"][1] = "x"
    S("order_rung_string", cbytes(d), "terms/1 s2.5: a rung an object, E_RUNG with its index", fault("E_RUNG", 1))
    d = base(); d["order"]["ladder"][1]["sellerBps"] = "5000"
    S("order_rung_seller", cbytes(d), "terms/1 s2.5: sellerBps an int", fault("E_RUNG_SELLER", 1))
    d = base(); d["order"]["ladder"][1]["sellerBps"] = d["order"]["ladder"][1]["sellerBps"] - 1
    S("order_rung_sum", cbytes(d), "terms/1 s2.5: buyerBps + sellerBps = 10000", fault("E_RUNG_SUM", 1))
    for kind_, raw_, tok_ in (("utf8", b"\xff", "E_UTF8"), ("number", b'{"a":1.5}', "E_NUMBER"), ("depth", b"[" * 129 + b"]" * 129, "E_DEPTH"), ("dup_key", b'{"a":1,"a":2}', "E_DUP_KEY"), ("key_charset", b'{"\xc3\xa9":1}', "E_KEY_CHARSET")):
        S("canon_" + kind_, raw_, "galeed/1 s3: " + tok_ + " on a listing", fault(tok_))
    d = base(); d["guilds"] = None
    S("guilds_null", cbytes(d), "s2.5: guilds an array", fault("E_LISTING_GUILDS"))
    d = base(); del d["guilds"][0]["seq"]
    S("guild_missing_seq", cbytes(d), "s2.3: seq required", fault("E_GUILD_MISSING", 0))
    d = base(); d["guilds"][0]["uri"] = "https://g"
    S("guild_closed_uri", cbytes(d), "s2.3: no uri member on a guild element", fault("E_GUILD_CLOSED", 0))
    d = base(); d["guilds"][0]["seq"] = "3"
    S("guild_seq_string", cbytes(d), "s2.5: E_GUILD_SEQ", fault("E_GUILD_SEQ", 0))
    d = base(); d["guilds"][0]["panelHash"] = ZERO32
    S("guild_panel_zero", cbytes(d), "s2.5: panelHash nonzero", fault("E_GUILD_PANEL", 0))
    d = base(); d["guilds"][0]["panelHash"] = "0x1"
    S("guild_panel_short", cbytes(d), "s2.5: panelHash hex32; it is compared with nothing afterwards, so its form is the whole test", fault("E_GUILD_PANEL", 0))
    d = base(); d["guilds"][0]["sig"] = "0x00"
    S("guild_sig_form", cbytes(d), "s2.5: E_GUILD_SIG", fault("E_GUILD_SIG", 0))
    d = base(); d["guilds"][0]["seq"] = "3"; d["guilds"][0]["panelHash"] = ZERO32
    S("guild_order_seq_before_panel", cbytes(d), "s2.5 order within a guild: seq before panelHash", fault("E_GUILD_SEQ", 0))
    d = base(); d["guilds"][0]["panelHash"] = ZERO32; d["guilds"][0]["sig"] = "0x00"
    S("guild_order_panel_before_sig", cbytes(d), "s2.5 order within a guild: panelHash before sig", fault("E_GUILD_PANEL", 0))
    d = base(); d["guilds"][0]["pointer"] = "galeed:guild:2:" + addr("guild"); d["guilds"][0]["seq"] = "x"
    S("guild_order_inherited_before_added", cbytes(d), "s2.3: the inherited rows before the added members", fault("E_GUILD_POINTER", 0))
    d = base(); del d["judges"][0]["sig"]; d["judges"][0]["extra"] = 1
    S("judge_order_missing_before_closed", cbytes(d), "s2.3 order: presence before closure, the added sig among the required members", fault("E_JUDGE_MISSING", 0))
    d = base(); del d["guilds"][0]["seq"]; d["guilds"][0]["uri"] = "https://g"
    S("guild_order_missing_before_closed", cbytes(d), "s2.3 order: presence before closure on a guild element", fault("E_GUILD_MISSING", 0))
    d = base(); d["judges"][0]["extra"] = 1; d["judges"][0]["judge"] = ZERO20
    S("judge_order_closed_before_rows", cbytes(d), "s2.3 order: closure before the inherited rows", fault("E_JUDGE_CLOSED", 0))
    d = base(); d["guilds"][0]["amanahHash"] = ZERO32; d["guilds"][0]["quoteHash"] = "0x1"
    S("guild_order_amanah_before_quote", cbytes(d), "terms/1 s2.5 row order within a guild: amanahHash before quoteHash", fault("E_GUILD_AMANAH", 0))
    d = base(); d["judges"][1]["source"] = "x"; d["judges"][1]["sig"] = "0x00"
    S("judge_order_source_before_sig", cbytes(d), "s2.3: the inherited source row before the added sig", fault("E_JUDGE_SOURCE", 1))
    d = base(); d["guilds"].append(dict(d["guilds"][0], amanahHash=H32()))
    S("guild_dup", cbytes(d), "s2.5: E_LISTING_DUP_GUILD", fault("E_LISTING_DUP_GUILD", 1))
    d = base(); d["judges"].append(dict(d["judges"][0], amanahHash=H32()))
    S("judge_dup_address_other_members", cbytes(d), "s2.5: the address repeats though other members differ", fault("E_LISTING_DUP_JUDGE", 2))
    d = base(); d["judges"] = []; d["guilds"] = []
    S("no_candidate", cbytes(d), "s2.5: E_LISTING_NO_CANDIDATE", fault("E_LISTING_NO_CANDIDATE"))

    # ---- s2.5's binding to the carried document (s7 from BIND_CHAIN on)
    d = G.listing_of(blk_j, tb_j); d["chainId"] = 5
    S("bind_chain", cbytes(d), "s2.5: the listing's chainId is not the document's, E_LISTING_BIND carrying BIND_CHAIN", fault("E_LISTING_BIND", inner=inner("BIND_CHAIN")))
    d = base(); d["token"] = addr("token2")
    S("bind_token", cbytes(d), "s7.1: the token differs", fault("E_LISTING_BIND", inner=inner("BIND_TOKEN")))
    d = base(); d["logic"] = addr("logic2"); d["proxyCodeHash"] = G.proxy_codehash(addr("logic2"))
    S("bind_logic", cbytes(d), "s7.1: another logic with its own code hash", fault("E_LISTING_BIND", inner=inner("BIND_LOGIC")))
    d = base(); d["order"]["amount"] = "999"
    S("bind_order_amount", cbytes(d), "s7.1: the order's amount differs from the document's", fault("E_LISTING_BIND", inner=inner("BIND_ORDER")))
    d = base(); d["order"]["buyerMayOpen"] = False
    S("bind_order_buyeropen", cbytes(d), "s7.1: buyerMayOpen differs", fault("E_LISTING_BIND", inner=inner("BIND_ORDER")))
    d = base(); d["order"]["amount"] = str((2 ** 256 - 1) // 10000 + 1)
    S("order_amount_over_cap", cbytes(d), "s2.2: the listing's amount row carries no bound; terms/1 s2.3's bound reaches it through the binding, BIND_ORDER", fault("E_LISTING_BIND", inner=inner("BIND_ORDER")))
    d = base(); d["order"]["amount"] = str((2 ** 256 - 1) // 10000 + 1); d["order"]["expiry"] = "x"
    S("order_amount_over_cap_expiry", cbytes(d), "s2.5 order: the order object's own rows before the binding; the amount over the bound is no row of this law", fault("E_LISTING_EXPIRY"))
    # s7.1's order, each adjacent pair of comparisons differing
    d = G.listing_of(blk_j, tb_j); d["chainId"] = 5; d["token"] = addr("token2")
    S("bind_pair_chain_token", cbytes(d), "s7.1 order: BIND_CHAIN before BIND_TOKEN", fault("E_LISTING_BIND", inner=inner("BIND_CHAIN")))
    d = base(); d["token"] = addr("token2"); d["logic"] = addr("logic2"); d["proxyCodeHash"] = G.proxy_codehash(addr("logic2"))
    S("bind_pair_token_logic", cbytes(d), "s7.1 order: BIND_TOKEN before BIND_LOGIC", fault("E_LISTING_BIND", inner=inner("BIND_TOKEN")))
    d = base(); d["logic"] = addr("logic2"); d["proxyCodeHash"] = G.proxy_codehash(addr("logic2")); d["order"]["nonce"] = 8
    S("bind_pair_logic_order", cbytes(d), "s7.1 order: BIND_LOGIC before BIND_ORDER", fault("E_LISTING_BIND", inner=inner("BIND_LOGIC")))
    d = base(); d["order"]["nonce"] = 8; d["judges"][1]["amanahHash"] = H32()
    S("bind_pair_order_judges", cbytes(d), "s7.1 order: BIND_ORDER before BIND_JUDGES", fault("E_LISTING_BIND", inner=inner("BIND_ORDER")))
    d = base(); d["judges"][1]["amanahHash"] = H32(); d["guilds"][0]["quoteHash"] = H32()
    S("bind_pair_judges_guilds", cbytes(d), "s7.1 order: BIND_JUDGES before BIND_GUILDS, the judge's index carried", fault("E_LISTING_BIND", inner=inner("BIND_JUDGES", 1)))
    # s7.1: the listing's own count where the lengths differ, not the first differing index
    d = base(); d["judges"].append(dict(d["judges"][0], judge=addr("judge3"), amanahHash=H32()))
    S("bind_judges_three_vs_two", cbytes(d), "s7.1: three judges against the document's two carry 3, the listing's own count", fault("E_LISTING_BIND", inner=inner("BIND_JUDGES", 3)))
    d = base(); d["judges"] = [dict(d["judges"][0], amanahHash=H32())]
    S("bind_judges_one_differing_vs_two", cbytes(d), "s7.1: one judge, differing, against two carries 1, the listing's own count, whatever the elements", fault("E_LISTING_BIND", inner=inner("BIND_JUDGES", 1)))
    d = base(); d["guilds"].append(dict(d["guilds"][0], pointer="galeed:guild:1:" + addr("guild2"), amanahHash=H32()))
    S("bind_guilds_two_vs_one", cbytes(d), "s7.1: two guilds against the document's one carry 2", fault("E_LISTING_BIND", inner=inner("BIND_GUILDS", 2)))
    d = base(); d["order"]["openWindow"] = 1
    S("bind_order_openwindow", cbytes(d), "s7.1: openWindow differs", fault("E_LISTING_BIND", inner=inner("BIND_ORDER")))
    d = base(); d["order"]["ladder"][1]["feeBearer"] = 0
    S("bind_order_ladder", cbytes(d), "s7.1: a rung's feeBearer differs", fault("E_LISTING_BIND", inner=inner("BIND_ORDER")))
    d = base(); d["order"]["ladder"][1] = dict(d["order"]["ladder"][1], buyerBps=4000, sellerBps=6000); resign(d)
    S("bind_order_ladder_bps", cbytes(d), "s7.1: a rung's two shares differ, its feeBearer the same; re-signed", fault("E_LISTING_BIND", inner=inner("BIND_ORDER")))
    blk_l = G.base_block(judges=[blk["judges"][0]], ladder=[{"buyerBps": 0, "sellerBps": 10000, "feeBearer": 0},
                                                           {"buyerBps": 10000, "sellerBps": 0, "feeBearer": 1},
                                                           {"buyerBps": 10000, "sellerBps": 0, "feeBearer": 2}])
    tb_l = G.terms_doc(cbytes(blk_l))
    d = G.listing_of(blk_l, tb_l); d["order"]["ladder"] = d["order"]["ladder"][:2]; resign(d)
    S("bind_order_ladder_prefix", cbytes(d), "s7.1: the listing's ladder a prefix of the document's, every rung it has equal; re-signed", fault("E_LISTING_BIND", inner=inner("BIND_ORDER")))
    d = base(); d["order"]["nonce"] = 8
    S("bind_order_nonce", cbytes(d), "s7.1: the nonce differs; the comparison names it before any signature", fault("E_LISTING_BIND", inner=inner("BIND_ORDER")))
    d = base(); d["judges"] = d["judges"][:1]
    S("bind_judges_length", cbytes(d), "s7.1: BIND_JUDGES carrying the listing's own count", fault("E_LISTING_BIND", inner=inner("BIND_JUDGES", 1)))
    d = base(); d["judges"][1]["amanahHash"] = H32()
    S("bind_judges_amanah", cbytes(d), "s7.1: a judge's amanahHash differs, index 1", fault("E_LISTING_BIND", inner=inner("BIND_JUDGES", 1)))
    d = base(); del d["judges"][1]["source"]
    S("bind_judges_source_absent", cbytes(d), "s7.1: a source present on one side only", fault("E_LISTING_BIND", inner=inner("BIND_JUDGES", 1)))
    d = base(); d["judges"][0]["source"] = "galeed:guild:1:" + addr("guild")
    S("bind_judges_source_added", cbytes(d), "s7.1: a source on the listing's side alone", fault("E_LISTING_BIND", inner=inner("BIND_JUDGES", 0)))
    d = base(); d["guilds"] = []
    S("bind_guilds_length", cbytes(d), "s7.1: BIND_GUILDS carrying 0", fault("E_LISTING_BIND", inner=inner("BIND_GUILDS", 0)))
    d = base(); d["guilds"][0]["quoteHash"] = H32()
    S("bind_guilds_quote", cbytes(d), "s7.1: a guild's quoteHash differs", fault("E_LISTING_BIND", inner=inner("BIND_GUILDS", 0)))
    # s7.1: each member of the three comparison lists not witnessed alone above, the listing re-signed over its
    # own values (resign), so that a reader skipping the member accepts the listing outright
    d = base(); d["order"]["sellerKey"] = addr("seller2"); resign(d, "seller2")
    S("bind_order_seller", cbytes(d), "s7.1: the order's sellerKey differs, the listing signed by the key it names", fault("E_LISTING_BIND", inner=inner("BIND_ORDER")))
    d = base(); d["order"]["sellerPayout"] = addr("payout2"); resign(d)
    S("bind_order_payout", cbytes(d), "s7.1: sellerPayout differs", fault("E_LISTING_BIND", inner=inner("BIND_ORDER")))
    d = base(); d["order"]["sellerFund"] = addr("company-wallet"); resign(d)
    S("bind_order_fund", cbytes(d), "s7.1: sellerFund differs, a company's wallet against the document's sellerKey", fault("E_LISTING_BIND", inner=inner("BIND_ORDER")))
    d = base(); d["order"]["deliveryWindow"] += 1; resign(d)
    S("bind_order_delivery", cbytes(d), "s7.1: deliveryWindow differs", fault("E_LISTING_BIND", inner=inner("BIND_ORDER")))
    d = base(); d["order"]["confirmWindow"] += 1; resign(d)
    S("bind_order_confirm", cbytes(d), "s7.1: confirmWindow differs", fault("E_LISTING_BIND", inner=inner("BIND_ORDER")))
    d = base(); d["order"]["expiry"] += 1; resign(d)
    S("bind_order_expiry", cbytes(d), "s7.1: expiry differs", fault("E_LISTING_BIND", inner=inner("BIND_ORDER")))
    d = base(); d["judges"][0]["quoteHash"] = H32(); resign(d)
    S("bind_judges_quote", cbytes(d), "s7.1: a judge's quoteHash differs, index 0", fault("E_LISTING_BIND", inner=inner("BIND_JUDGES", 0)))
    d = base(); d["guilds"][0]["pointer"] = "galeed:guild:1:" + addr("guild2"); resign(d)
    S("bind_guilds_pointer", cbytes(d), "s7.1: a guild's pointer differs, another guild on the same chain", fault("E_LISTING_BIND", inner=inner("BIND_GUILDS", 0)))
    d = base(); d["guilds"][0]["amanahHash"] = H32(); resign(d)
    S("bind_guilds_amanah", cbytes(d), "s7.1: a guild's amanahHash differs", fault("E_LISTING_BIND", inner=inner("BIND_GUILDS", 0)))
    # the guild array's index rule, as the judge array's above, over a document with two guilds
    gg2 = {"pointer": "galeed:guild:1:" + addr("guild2"), "amanahHash": H32(), "quoteHash": G.qhash(G.quote_bytes(addr("token"), autoAccept=0, feeBps=300))}
    blk_g2 = dict(blk, guilds=blk["guilds"] + [gg2])
    tb_g2 = G.terms_doc(cbytes(blk_g2))
    d = G.listing_of(blk_g2, tb_g2)
    S("ok_two_guilds", cbytes(d), "s2.5: a listing with two guilds binds and passes", oks(d))
    d = G.listing_of(blk_g2, tb_g2); d["guilds"] = [dict(d["guilds"][0], quoteHash=H32())]; resign(d)
    S("bind_guilds_one_differing_vs_two", cbytes(d), "s7.1: one guild, differing, against two carries 1, the listing's own count, whatever the elements", fault("E_LISTING_BIND", inner=inner("BIND_GUILDS", 1)))
    d = G.listing_of(blk_g2, tb_g2); d["guilds"][1]["quoteHash"] = H32(); resign(d)
    S("bind_guilds_second", cbytes(d), "s7.1: the second guild differs, index 1", fault("E_LISTING_BIND", inner=inner("BIND_GUILDS", 1)))
    d = G.listing_of(blk_g2, tb_g2); d["guilds"][0]["quoteHash"] = H32(); d["guilds"][1]["amanahHash"] = H32(); resign(d)
    S("bind_guilds_first_differing", cbytes(d), "s7.1: both guilds differ; the first differing index", fault("E_LISTING_BIND", inner=inner("BIND_GUILDS", 0)))
    d = base(); d["guilds"][0]["seq"] = 9; d["guilds"][0]["panelHash"] = H32()
    S("ok_seq_not_compared", cbytes(d), "s7.1: seq and panelHash are the listing's own and outside the offer", oks(d))
    d = base(); d["judges"][1]["judge"] = addr("judge3"); d["judges"][0]["judge"] = addr("judge2")
    S("bind_judges_first_differing", cbytes(d), "s7.1: the first differing index", fault("E_LISTING_BIND", inner=inner("BIND_JUDGES", 0)))

    # ---- signatures
    d = base(); d["judges"][0]["sig"] = d["judges"][1]["sig"]
    S("sig_other_offer", cbytes(d), "s2.5: a signature over another candidate's offer", fault("E_LISTING_SIG", 0, inner=inner("E_SIG_SIGNER")))
    d = base(); d["judges"][0]["sig"] = G.sign(key("stranger"), bytes.fromhex(listing_offer_digest(d, 0)[2:]))
    S("sig_stranger", cbytes(d), "s2.5: a stranger's signature", fault("E_LISTING_SIG", 0, inner=inner("E_SIG_SIGNER")))
    d = base(); d["guilds"][0]["sig"] = d["guilds"][0]["sig"][:-2] + "1d"
    S("sig_v", cbytes(d), "s2.5: the signature fault carried, guild at candidate 2", fault("E_LISTING_SIG", 2, inner=inner("E_SIG_V")))
    d = base(); d["judges"][0]["sig"] = "0x" + "00" * 64 + "1b"
    S("sig_range", cbytes(d), "s2.5: E_SIG_RANGE carried", fault("E_LISTING_SIG", 0, inner=inner("E_SIG_RANGE")))
    d = base(); d["judges"][0]["sig"] = G.sig_hex(G.N_ORDER - 1, 1, 27)
    S("sig_recover", cbytes(d), "s2.5: E_SIG_RECOVER carried", fault("E_LISTING_SIG", 0, inner=inner("E_SIG_RECOVER")))
    d = base(); rsv = G.sign_digest(key("seller"), bytes.fromhex(listing_offer_digest(d, 0)[2:])); d["judges"][0]["sig"] = G.sig_hex(rsv[0], G.N_ORDER - rsv[1], 55 - rsv[2])
    S("sig_high_s", cbytes(d), "s2.5: E_SIG_HIGH_S carried (the seller's own twin)", fault("E_LISTING_SIG", 0, inner=inner("E_SIG_HIGH_S")))
    d = base(); d["judges"][1]["source"] = "galeed:guild:1:" + addr("guild2")
    S("bind_source_changed", cbytes(d), "s7.1: source is compared with the document, though it is outside the offer", fault("E_LISTING_BIND", inner=inner("BIND_JUDGES", 1)))
    d = base(); d["judges"][0]["sig"] = ""
    S("ok_judge_by_call", cbytes(d), "s2.5: an empty slot is a by-call candidate, OFFER_BY_CALL; the listing passes and lists it", oks(d))
    d = base(); d["guilds"][0]["sig"] = ""
    S("ok_guild_by_call", cbytes(d), "s2.3: a guild element's slot may be empty too; candidate 2", oks(d))
    d = base(); d["judges"][0]["sig"] = ""; d["judges"][1]["sig"] = ""; d["guilds"][0]["sig"] = ""
    S("ok_all_by_call", cbytes(d), "s2.5: a seller with no key offers every candidate by call", oks(d))
    d = base(); d["judges"][0]["sig"] = ""; d["guilds"][0]["sig"] = ""
    S("ok_two_by_call", cbytes(d), "s2.5: a list of two by-call candidates, ascending", oks(d))
    d = base(); d["judges"][0]["sig"] = "0x"
    S("judge_sig_zero_bytes", cbytes(d), "s2.3: 0x is neither hex65 nor the empty string", fault("E_JUDGE_SIG", 0))
    d = base(); d["guilds"][0]["sig"] = "0x"
    S("guild_sig_zero_bytes", cbytes(d), "s2.3: 0x on a guild element", fault("E_GUILD_SIG", 0))
    d = base(); d["judges"][0]["sig"] = ""; d["judges"][1]["sig"] = d["guilds"][0]["sig"]
    S("by_call_then_bad_sig", cbytes(d), "s2.5: candidate 0 by call refuses nothing; candidate 1's wrong signature is named", fault("E_LISTING_SIG", 1, inner=inner("E_SIG_SIGNER")))
    d = base(); d["judges"][0]["sig"] = d["guilds"][0]["sig"]; d["judges"][1]["sig"] = "0x" + "00" * 64 + "1b"
    S("sig_first_candidate_named", cbytes(d), "s2.5: candidate 0's fault before candidate 1's", fault("E_LISTING_SIG", 0, inner=inner("E_SIG_SIGNER")))
    for slug, muts, tok in (
            ("adj_closed_before_v", [("extra", 1), ("v", "x")], fault("E_LISTING_CLOSED")),
            ("adj_v_before_chain", [("v", "x"), ("chainId", "1")], fault("E_LISTING_V")),
            ("adj_chain_before_token", [("chainId", "1"), ("token", ZERO20)], fault("E_LISTING_CHAIN")),
            ("adj_token_before_logic", [("token", ZERO20), ("logic", ZERO20)], fault("E_LISTING_TOKEN")),
            ("adj_logic_before_codehash", [("logic", ZERO20), ("proxyCodeHash", H32())], fault("E_LISTING_LOGIC")),
            ("adj_codehash_before_terms", [("proxyCodeHash", H32()), ("terms", "x")], fault("E_LISTING_CODEHASH"))):
        d = base()
        for k_, v_ in muts:
            d[k_] = v_
        S(slug, cbytes(d), "s2.5 order", tok)
    d = base(); d["terms"]["inline_md"] = "wrong"; d["order"] = "x"
    S("adj_terms_hash_before_order", cbytes(d), "s2.5 order: the terms hash before the order object", fault("E_LISTING_TERMS_HASH"))
    d = base(); d["terms"] = {"hash": "0x1", "inline_md": "wrong"}
    S("adj_terms_form_before_hash", cbytes(d), "s2.5 order: the form before the hash", fault("E_LISTING_TERMS"))
    d = base(); d["terms"] = {"hash": sha(nb), "inline_md": nb.decode("utf-8")}; d["order"] = "x"
    S("adj_terms_doc_before_order", cbytes(d), "s2.5 order: the document test before the order object", fault("E_LISTING_TERMS_DOC", inner=inner("E_TERMS_BLOCK")))
    d = base(); d["order"]["expiry"] = "x"; d["judges"] = {}
    S("adj_order_expiry_before_judges", cbytes(d), "s2.5 order: the order's expiry before judges", fault("E_LISTING_EXPIRY"))
    d = base(); d["judges"] = {}; d["guilds"] = {}
    S("adj_judges_before_guilds", cbytes(d), "s2.5 order: the judges form before the guilds form", fault("E_LISTING_JUDGES"))
    d = base(); d["judges"].append(dict(d["judges"][0], amanahHash=ZERO32))
    S("adj_judge_element_before_repeat", cbytes(d), "s2.5 order: a judge's table before the judge repeat", fault("E_JUDGE_AMANAH", 2))
    d = base(); d["judges"].append(dict(d["judges"][0], amanahHash=H32())); d["guilds"] = "x"
    S("adj_dup_judge_before_guilds", cbytes(d), "s2.5 order: the judge repeat before guilds", fault("E_LISTING_DUP_JUDGE", 2))
    d = base(); d["guilds"].append(dict(d["guilds"][0], panelHash=ZERO32))
    S("adj_guild_element_before_repeat", cbytes(d), "s2.5 order: a guild's table before the guild repeat", fault("E_GUILD_PANEL", 1))
    d = base(); d["guilds"].append(dict(d["guilds"][0], amanahHash=H32())); d["token"] = addr("token2")
    S("adj_dup_guild_before_bind", cbytes(d), "s2.5 order: the guild repeat before the binding", fault("E_LISTING_DUP_GUILD", 1))
    d = base(); d["judges"] = []; d["guilds"] = []; d["token"] = addr("token2")
    S("adj_no_candidate_before_bind", cbytes(d), "s2.5 order: the non-empty test before the binding", fault("E_LISTING_NO_CANDIDATE"))
    d = base(); d["token"] = addr("token2"); d["judges"][0]["sig"] = "0x" + "00" * 64 + "1b"
    S("adj_bind_before_sig", cbytes(d), "s2.5 order: the binding before any signature", fault("E_LISTING_BIND", inner=inner("BIND_TOKEN")))
    d = base(); d["order"]["ladder"] = [{"buyerBps": 0, "sellerBps": 10000, "feeBearer": 0}, {"buyerBps": 9999, "sellerBps": 1, "feeBearer": 1}]; d["order"]["nonce"] = "x"
    S("adj_order_ladder_rules_before_nonce", cbytes(d), "s2.5 order: the ladder-wide rules before nonce", fault("E_LADDER_LAST"))
    r0_ = {"buyerBps": 0, "feeBearer": 0, "sellerBps": 10000}
    d = base(); d["order"]["ladder"] = ["x"]
    S("ladder_one_bad_rung", cbytes(d), "s2.5 order: the ladder's count before any rung", fault("E_LISTING_LADDER"))
    d = base(); d["order"]["ladder"][1] = {"buyerBps": 5000, "sellerBps": 5000, "x": 1}
    S("rung_missing_before_closed", cbytes(d), "s2.5 order within a rung: missing before closed", fault("E_RUNG_MISSING", 1))
    d = base(); d["order"]["ladder"][1] = {"buyerBps": 10001, "feeBearer": 2, "sellerBps": "5000"}
    S("rung_buyer_before_seller", cbytes(d), "s2.5 order within a rung: buyer before seller", fault("E_RUNG_BUYER", 1))
    d = base(); d["order"]["ladder"][1] = {"buyerBps": 5000, "feeBearer": 3, "sellerBps": 4000}
    S("rung_bearer_before_sum", cbytes(d), "s2.5 order within a rung: bearer before the sum", fault("E_RUNG_BEARER", 1))
    d = base(); d["order"]["ladder"] = [{"buyerBps": 100, "feeBearer": 0, "sellerBps": 9900}, {"buyerBps": 9000, "feeBearer": 1, "sellerBps": 1000}]
    S("ladder_first_before_last", cbytes(d), "s2.5 order: the first rung's rule before the last's", fault("E_LADDER_FIRST"))
    d = base(); d["judges"][0]["judge"] = "0x1"; d["judges"][0]["amanahHash"] = ZERO32
    S("judge_order_address_before_amanah", cbytes(d), "s2.3 order within a judge: address before amanah", fault("E_JUDGE_ADDRESS", 0))
    d = base(); d["judges"][0]["amanahHash"] = ZERO32; d["judges"][0]["quoteHash"] = "abc"
    S("judge_order_amanah_before_quote", cbytes(d), "s2.3 order within a judge: amanah before quoteHash", fault("E_JUDGE_AMANAH", 0))
    d = base(); d["judges"][1]["quoteHash"] = "abc"; d["judges"][1]["source"] = 1
    S("judge_order_quote_before_source", cbytes(d), "s2.3 order within a judge: quoteHash before source", fault("E_JUDGE_QUOTE", 1))
    d = base(); d["guilds"][0]["pointer"] = "galeed:guild:1"; d["guilds"][0]["amanahHash"] = ZERO32
    S("guild_order_pointer_before_amanah", cbytes(d), "s2.3 order within a guild: pointer before amanah", fault("E_GUILD_POINTER", 0))
    d = base(); d["guilds"][0]["quoteHash"] = "abc"; d["guilds"][0]["seq"] = "x"
    S("guild_order_quote_before_seq", cbytes(d), "s2.3 order within a guild: quoteHash before seq", fault("E_GUILD_QUOTE", 0))
    d = base(); d["judges"][0]["sig"] = "0x00"; d["judges"][1]["judge"] = ZERO20
    S("judge_sig_before_next_element", cbytes(d), "s2.3 order: a judge's sig row before the next element's table", fault("E_JUDGE_SIG", 0))
    gg2 = {"pointer": "galeed:guild:1:" + addr("guild2"), "amanahHash": H32(), "quoteHash": G.qhash(G.quote_bytes(addr("token"), autoAccept=0))}
    blk_2g = dict(blk, guilds=[blk["guilds"][0], gg2])
    tb_2g = G.terms_doc(cbytes(blk_2g))
    d = G.listing_of(blk_2g, tb_2g); d["guilds"][0]["sig"] = "0x00"; d["guilds"][1]["amanahHash"] = ZERO32
    S("guild_sig_before_next_element", cbytes(d), "s2.3 order: a guild's sig row before the next guild element's table", fault("E_GUILD_SIG", 0))
    d = base(); d["order"]["ladder"][0] = {"buyerBps": 0, "sellerBps": 9999, "feeBearer": 0}; d["order"]["ladder"][1] = "x"
    S("rung_sum_before_next_rung", cbytes(d), "s2.5 order: every row of rung 0 before rung 1's form", fault("E_RUNG_SUM", 0))
    d = base(); d["order"]["ladder"][1] = {"buyerBps": 10001, "sellerBps": 0, "feeBearer": 0, "x": 1}
    S("rung_closed_before_buyer", cbytes(d), "s2.5 order within a rung: closure before buyerBps", fault("E_RUNG_CLOSED", 1))
    d = base(); d["order"]["ladder"][1] = {"buyerBps": 5000, "sellerBps": "5000", "feeBearer": 3}
    S("rung_seller_before_bearer", cbytes(d), "s2.5 order within a rung: sellerBps before feeBearer", fault("E_RUNG_SELLER", 1))
    d = base(); d["order"]["ladder"] = [{"buyerBps": 0, "sellerBps": 10000, "feeBearer": 0}, {"buyerBps": 9000, "sellerBps": 1000, "feeBearer": 0}, {"buyerBps": 8000, "sellerBps": 2000, "feeBearer": 0}, {"buyerBps": 9999, "sellerBps": 1, "feeBearer": 0}]
    S("ladder_last_before_order", cbytes(d), "s2.5 order: the last rung's rule before the order rule", fault("E_LADDER_LAST"))
    d = base(); d["guilds"][0]["x"] = 1; d["guilds"][0]["pointer"] = "galeed:guild:1"
    S("guild_order_closed_before_rows", cbytes(d), "s2.3 order: a guild element's closure before its rows", fault("E_GUILD_CLOSED", 0))
    d = base(); js = d["judges"]; d["judges"] = [js[0], copy(js[0]), dict(copy(js[1]), amanahHash=ZERO32)]
    S("judge_dup_before_later_row", cbytes(d), "s2.5 order: the repeat test runs once every element has passed; judge 1 repeats judge 0 and judge 2 fails a row", fault("E_JUDGE_AMANAH", 2))
    d = base(); g0 = d["guilds"][0]; d["guilds"] = [g0, copy(g0), dict(copy(g0), amanahHash=ZERO32)]
    S("guild_dup_before_later_row", cbytes(d), "s2.5 order: the repeat test runs once every element has passed; guild 1 repeats guild 0 and guild 2 fails a row", fault("E_GUILD_AMANAH", 2))
    S("codehash_upper", cbytes(dict(base(), proxyCodeHash=base()["proxyCodeHash"].upper().replace("0X", "0x"))), "s1 hex32 is lowercase: proxyCodeHash", fault("E_LISTING_CODEHASH"))
    d = base(); d["terms"]["hash"] = d["terms"]["hash"].upper().replace("0X", "0x")
    S("terms_hash_upper", cbytes(d), "s1 hex32 is lowercase: terms.hash", fault("E_LISTING_TERMS"))
    d = base(); d["guilds"][0]["panelHash"] = d["guilds"][0]["panelHash"].upper().replace("0X", "0x")
    S("guild_panel_upper", cbytes(d), "s1 hex32 is lowercase: panelHash", fault("E_GUILD_PANEL", 0))
    S("logic_upper", cbytes(dict(base(), logic=up(base()["logic"]))), "s1 hex20 is lowercase: logic", fault("E_LISTING_LOGIC"))
    d = base(); d["order"]["sellerPayout"] = up(d["order"]["sellerPayout"])
    S("payout_upper", cbytes(d), "s1 hex20 is lowercase: order.sellerPayout", fault("E_LISTING_PAYOUT"))
    d = base(); d["order"]["sellerFund"] = up(d["order"]["sellerFund"])
    S("fund_upper", cbytes(d), "s1 hex20 is lowercase: order.sellerFund", fault("E_LISTING_FUND"))
    d = base(); d["judges"][0]["judge"] = up(d["judges"][0]["judge"])
    S("judge_address_upper", cbytes(d), "s1 hex20 is lowercase: a judge's address, with its index", fault("E_JUDGE_ADDRESS", 0))
    d = base(); d["judges"][0]["amanahHash"] = up(d["judges"][0]["amanahHash"])
    S("judge_amanah_upper", cbytes(d), "s1 hex32 is lowercase: a judge's amanahHash", fault("E_JUDGE_AMANAH", 0))
    d = base(); d["judges"][0]["quoteHash"] = up(d["judges"][0]["quoteHash"])
    S("judge_quote_upper", cbytes(d), "s1 hex32 is lowercase: a judge's quoteHash", fault("E_JUDGE_QUOTE", 0))
    d = base(); d["judges"][0]["sig"] = up(d["judges"][0]["sig"])
    S("judge_sig_upper", cbytes(d), "s1 hex65 is lowercase: a judge's sig", fault("E_JUDGE_SIG", 0))
    d = base(); d["guilds"][0]["amanahHash"] = up(d["guilds"][0]["amanahHash"])
    S("guild_amanah_upper", cbytes(d), "s1 hex32 is lowercase: a guild's amanahHash", fault("E_GUILD_AMANAH", 0))
    d = base(); d["guilds"][0]["quoteHash"] = up(d["guilds"][0]["quoteHash"])
    S("guild_quote_upper", cbytes(d), "s1 hex32 is lowercase: a guild's quoteHash", fault("E_GUILD_QUOTE", 0))
    d = base(); d["guilds"][0]["sig"] = up(d["guilds"][0]["sig"])
    S("guild_sig_upper", cbytes(d), "s1 hex65 is lowercase: a guild's sig", fault("E_GUILD_SIG", 0))
    d = base(); del d["order"]
    S("canon_pretty_before_missing", json.dumps(d, indent=1, sort_keys=True, ensure_ascii=False).encode("utf-8"), "galeed/1 s3 order: canonical before presence", fault("E_NOT_CANONICAL"))
    C.case("check", ["listings/absent.json"], "HARNESS: an unreadable path", None)
    C.case("check", ["listings/l001_ok_full.json", "x"], "HARNESS: an argument beyond the list", None)
    C.case("check", [], "HARNESS: no path", None, path="(none)")

    # ---- bind (s7.1 as a standalone predicate)
    path_ok = "listings/l001_ok_full.json"
    tp = W("terms", "ok", tb, ext=".md")
    C.case("bind", [path_ok, tp], "s7.1: BOUND", {"verdict": "BOUND"}, path=tp)
    tp2 = W("terms", "other_nonce", G.terms_doc(cbytes(dict(blk, nonce=8))), ext=".md")
    C.case("bind", [path_ok, tp2], "s7.1: BIND_HASH where the document handed in is not the one the listing carries", {"verdict": "BIND_HASH"}, path=tp2)
    tp3 = W("terms", "no_block", nb, ext=".md")
    C.case("bind", [path_ok, tp3], "s7.1: TERMS_INVALID carrying the token", {"verdict": "TERMS_INVALID", "inner": inner("E_TERMS_BLOCK")}, path=tp3)
    C.case("bind", [P_PRETTY, tp], "s7.1: LISTING_INVALID carrying the token", {"verdict": "LISTING_INVALID", "inner": inner("E_NOT_CANONICAL")}, path=tp)
    C.case("bind", [P_PRETTY, tp3], "s7.1 order: the listing before the document", {"verdict": "LISTING_INVALID", "inner": inner("E_NOT_CANONICAL")}, path=tp3)
    C.case("bind", [path_ok], "HARNESS: an argument short", None, path=path_ok)
    C.case("bind", [path_ok, "terms/absent.md"], "HARNESS: an unreadable path", None, path=path_ok)
    return s0, path_ok


# ==========================================================================
# the link: link-check, encode, decode, resolve-link
# ==========================================================================

def gen_link(blk, tb, s0, path_ok):
    sb = cbytes(s0)
    ln = G.link_for(s0, tx=H32())
    lb = cbytes(ln)
    lp = W("links", "ok", lb)
    okl = {"chainId": 1, "hash": ln["hash"], "ok": True, "tx": ln["tx"]}
    C.case("link-check", [lp], "s3.1: a link", okl, path=lp)
    ln5 = G.link_of(11155111, H32(), H32())
    p = W("links", "ok_other_chain", cbytes(ln5))
    C.case("link-check", [p], "s3.1: any chainId; the hash need point at nothing yet", {"chainId": 11155111, "hash": ln5["hash"], "ok": True, "tx": ln5["tx"]}, path=p)
    p = W("links", "ok_zero_hash", cbytes(G.link_of(1, ZERO32, ZERO32)))
    C.case("link-check", [p], "s3.1: tx and hash are hex32, zero included", {"chainId": 1, "hash": ZERO32, "ok": True, "tx": ZERO32}, path=p)
    p = W("links", "canon_pretty", json.dumps(ln, indent=1, sort_keys=True).encode("utf-8"))
    C.case("link-check", [p], "galeed/1 s3.5 test 6", fault("E_NOT_CANONICAL"), path=p)
    p = W("links", "root_array", b"[]")
    C.case("link-check", [p], "s3.1: root an object", fault("E_LINK"), path=p)
    d = dict(ln); del d["tx"]
    p = W("links", "missing", cbytes(d))
    C.case("link-check", [p], "s3.1: a required member absent", fault("E_LINK_MISSING"), path=p)
    p = W("links", "closed", cbytes(dict(ln, uri="https://x")))
    C.case("link-check", [p], "s3.1: a member beyond the four", fault("E_LINK_CLOSED"), path=p)
    d = dict(ln); del d["hash"]; d["x"] = 1
    p = W("links", "missing_before_closed", cbytes(d))
    C.case("link-check", [p], "s3.1 order", fault("E_LINK_MISSING"), path=p)
    d = dict(ln); del d["tx"]
    p = W("links", "canon_pretty_before_missing", json.dumps(d, indent=1, sort_keys=True).encode("utf-8"))
    C.case("link-check", [p], "galeed/1 s3 order: canonical before presence", fault("E_NOT_CANONICAL"), path=p)
    p = W("links", "v_listing", cbytes(dict(ln, v="eravon.listing/1")))
    C.case("link-check", [p], "s3.1: v is eravon.link/1", fault("E_LINK_V"), path=p)
    p = W("links", "chain_string", cbytes(dict(ln, chainId="1")))
    C.case("link-check", [p], "s3.1: chainId an int", fault("E_LINK_CHAIN"), path=p)
    p = W("links", "tx_short", cbytes(dict(ln, tx="0x12")))
    C.case("link-check", [p], "s3.1: tx hex32", fault("E_LINK_TX"), path=p)
    p = W("links", "tx_upper", cbytes(dict(ln, tx="0x" + "AB" * 32)))
    C.case("link-check", [p], "s1: hex32 is lowercase", fault("E_LINK_TX"), path=p)
    p = W("links", "hash_int", cbytes(dict(ln, hash=5)))
    C.case("link-check", [p], "s3.1: hash hex32", fault("E_LINK_HASH"), path=p)
    for slug, muts, tok in (("adj_closed_before_v", {"x": 1, "v": "y"}, "E_LINK_CLOSED"), ("adj_v_before_chain", {"v": "y", "chainId": "1"}, "E_LINK_V"),
                            ("adj_chain_before_tx", {"chainId": "1", "tx": "0x1"}, "E_LINK_CHAIN"), ("adj_tx_before_hash", {"tx": "0x1", "hash": "0x1"}, "E_LINK_TX")):
        p = W("links", slug, cbytes(dict(ln, **muts)))
        C.case("link-check", [p], "s3.1 order", fault(tok), path=p)
    C.case("link-check", ["links/absent.json"], "HARNESS: an unreadable path", None)
    C.case("link-check", [], "HARNESS: no path", None, path="(none)")

    # ---- transport
    text = b"eravon://open?e=" + b64(lb)
    C.case("encode", [lp], "s3.2: the link text", {"link": text.decode("ascii"), "ok": True}, path=lp)
    C.case("encode", ["links/l006_missing.json"], "s3.2: the link's fault", fault("E_LINK_MISSING"), path="links/l006_missing.json")
    for cid, slug, res in ((10, "ok_len_mod4_2", 1), (100, "ok_len_mod4_3", 2)):
        lx = cbytes(G.link_of(cid, H32(), H32()))
        assert len(lx) % 3 == res
        px = W("links", slug, lx)
        C.case("encode", [px], "s3.2: the link text at a segment length %d mod 4, no padding" % (2 if res == 1 else 3), {"link": (b"eravon://open?e=" + b64(lx)).decode("ascii"), "ok": True}, path=px)
    p = W("links", "hash_upper", cbytes(dict(ln, hash=ln["hash"].upper().replace("0X", "0x"))))
    C.case("link-check", [p], "s1: hex32 is lowercase; hash in uppercase", fault("E_LINK_HASH"), path=p)
    t = W("texts", "link_ok", text, ext=".txt")
    C.case("decode", [t], "s3.2: the text decodes to the link bytes", {"link": hx(lb), "ok": True}, path=t)
    t = W("texts", "link_fragment_prefix", b"#e=" + b64(lb), ext=".txt")
    C.case("decode", [t], "s3.2: #e= is no prefix of this law", fault("E_LINK_PREFIX"), path=t)
    t = W("texts", "link_no_prefix", b64(lb), ext=".txt")
    C.case("decode", [t], "s3.2: E_LINK_PREFIX", fault("E_LINK_PREFIX"), path=t)
    t = W("texts", "link_prefix_case", b"ERAVON://open?e=" + b64(lb), ext=".txt")
    C.case("decode", [t], "s3.2: the prefix byte for byte", fault("E_LINK_PREFIX"), path=t)
    t = W("texts", "link_empty_file", b"", ext=".txt")
    C.case("decode", [t], "s3.2: the empty file has no prefix", fault("E_LINK_PREFIX"), path=t)
    b64_mutations("decode", b"eravon://open?e=", lb, "link", "E_LINK_B64", "s3.2", {"link": hx(lb), "ok": True})
    t = W("texts", "link_doc_fault", b"eravon://open?e=" + b64(cbytes(dict(ln, v="x"))), ext=".txt")
    C.case("decode", [t], "s3.2: E_LINK_DOC carrying the link's fault", fault("E_LINK_DOC", inner=inner("E_LINK_V")), path=t)
    t = W("texts", "link_doc_listing", b"eravon://open?e=" + b64(sb), ext=".txt")
    C.case("decode", [t], "s3.2: a listing's bytes under the link prefix are no link (tx and hash absent)", fault("E_LINK_DOC", inner=inner("E_LINK_MISSING")), path=t)
    # the alphabet witness: a link's canonical bytes are ASCII and never encode to _, so the two URL
    # characters are witnessed on bytes that pass the alphabet rule and fail the document test
    junk = bytes([0xfb, 0xef, 0xbe, 0xff, 0xff, 0xff])
    segu = b64(junk); assert segu == b"----____"
    t = W("texts", "link_url_alphabet", b"eravon://open?e=" + segu, ext=".txt")
    C.case("decode", [t], "link_url_alphabet, s1: - and _ are the alphabet's two URL characters; the bytes pass the alphabet and fail the document test", fault("E_LINK_DOC", inner=inner("E_UTF8")), path=t)
    segs = segu.replace(b"-", b"+").replace(b"_", b"/")
    t = W("texts", "link_std_alphabet", b"eravon://open?e=" + segs, ext=".txt")
    C.case("decode", [t], "link_std_alphabet, s1: + and / are outside the alphabet", fault("E_LINK_B64"), path=t)
    # the base64url rule at the other residues: a link whose bytes are 1 mod 3 encodes to a length 2 mod 4
    # and passes; one that is 2 mod 3 encodes to 3 mod 4, where the trailing-bits rule bites on two bits
    lr2 = lr3 = None
    for cid in (1, 11, 111, 1111):
        cand = cbytes(G.link_of(cid, H32(), H32()))
        if len(cand) % 3 == 1 and lr2 is None: lr2 = cand
        if len(cand) % 3 == 2 and lr3 is None: lr3 = cand
    assert lr2 is not None and lr3 is not None
    t = W("texts", "link_ok_len_mod4_2", b"eravon://open?e=" + b64(lr2), ext=".txt")
    C.case("decode", [t], "s1: a segment of length 2 mod 4 decodes where its trailing bits are zero", {"link": hx(lr2), "ok": True}, path=t)
    t = W("texts", "link_ok_len_mod4_3", b"eravon://open?e=" + b64(lr3), ext=".txt")
    C.case("decode", [t], "s1: a segment of length 3 mod 4 decodes where its two trailing bits are zero", {"link": hx(lr3), "ok": True}, path=t)
    seg3 = bytearray(b64(lr3)); assert len(seg3) % 4 == 3
    seg3[-1] = B64_ALPHABET[B64_ALPHABET.index(seg3[-1]) ^ 0x01]
    t = W("texts", "link_trailing_bits_len_mod4_3", b"eravon://open?e=" + bytes(seg3), ext=".txt")
    C.case("decode", [t], "s1: at length 3 mod 4 the last character's two low bits are the unused ones; set, the text is refused", fault("E_LINK_B64"), path=t)
    C.case("decode", ["texts/absent.txt"], "HARNESS: an unreadable path", None)
    C.case("decode", [t, t], "HARNESS: an argument beyond the list", None, path=t)
    C.case("encode", [], "HARNESS: no path", None, path="(none)")

    # ---- resolve-link
    dp = W("docs", "listing", sb)
    C.case("resolve-link", [lp, dp], "s3.3: a listing resolves, LISTING with the listing's answer", {"document": oks(s0), "verdict": "LISTING"}, path=dp)
    st, sdg = G.settle_of(1, addr("instance"), 5000, 1800000000)
    stb = cbytes(st)
    lns = G.link_for(st)
    lsp = W("links", "settle", cbytes(lns))
    dsp = W("docs", "settle", stb)
    C.case("resolve-link", [lsp, dsp], "s3.3: a settlement offer resolves, SETTLE with its answer", {"document": {"buyerBps": 5000, "expiry": 1800000000, "instance": addr("instance"), "ok": True}, "verdict": "SETTLE"}, path=dsp)
    C.case("resolve-link", ["links/l006_missing.json", dp], "s3.3: LINK_INVALID carrying the token", {"verdict": "LINK_INVALID", "inner": inner("E_LINK_MISSING")}, path=dp)
    C.case("resolve-link", [lp, dsp], "s3.3: LINK_HASH, the bytes are not the ones the link names", {"verdict": "LINK_HASH"}, path=dsp)
    junk = b"{"
    lj = W("links", "junk", cbytes(G.link_of(1, H32(), sha(junk))))
    dj = W("docs", "junk", junk)
    C.case("resolve-link", [lj, dj], "s3.3: LINK_KIND, the bytes are no canonical object", {"verdict": "LINK_KIND"}, path=dj)
    arr = b"[]"
    la = W("links", "array", cbytes(G.link_of(1, H32(), sha(arr))))
    da = W("docs", "array", arr)
    C.case("resolve-link", [la, da], "s3.3: LINK_KIND, the root is no object", {"verdict": "LINK_KIND"}, path=da)
    nov = cbytes({"chainId": 1})
    lnv = W("links", "no_v", cbytes(G.link_of(1, H32(), sha(nov))))
    dnv = W("docs", "no_v", nov)
    C.case("resolve-link", [lnv, dnv], "s3.3: LINK_KIND, no v", {"verdict": "LINK_KIND"}, path=dnv)
    vint = cbytes({"v": 1})
    lvi = W("links", "v_int", cbytes(G.link_of(1, H32(), sha(vint))))
    dvi = W("docs", "v_int", vint)
    C.case("resolve-link", [lvi, dvi], "s3.3: LINK_KIND, v no string", {"verdict": "LINK_KIND"}, path=dvi)
    bad = copy(s0); bad["order"]["nonce"] = "7"
    badb = cbytes(bad)
    lbad = W("links", "bad_listing", cbytes(G.link_for(bad)))
    dbad = W("docs", "bad_listing", badb)
    C.case("resolve-link", [lbad, dbad], "s3.3: LINK_LISTING carrying the listing's token", {"verdict": "LINK_LISTING", "inner": inner("E_LISTING_NONCE")}, path=dbad)
    bad2 = copy(s0); bad2["judges"][0]["sig"] = bad2["judges"][1]["sig"]
    lbad2 = W("links", "bad_listing_sig", cbytes(G.link_for(bad2)))
    dbad2 = W("docs", "bad_listing_sig", cbytes(bad2))
    C.case("resolve-link", [lbad2, dbad2], "s3.3: LINK_LISTING carrying an indexed fault with its inner", {"verdict": "LINK_LISTING", "inner": inner("E_LISTING_SIG", 0, inner_=inner("E_SIG_SIGNER"))}, path=dbad2)
    l5 = W("links", "chain_5_for_listing", cbytes(G.link_of(5, H32(), sha(sb))))
    C.case("resolve-link", [l5, dp], "s3.3: LINK_CHAIN, a chain-1 listing under a chain-5 link", {"verdict": "LINK_CHAIN"}, path=dp)
    l5s = W("links", "chain_5_for_settle", cbytes(G.link_of(5, H32(), sha(stb))))
    C.case("resolve-link", [l5s, dsp], "s3.3: LINK_CHAIN on a settlement offer", {"verdict": "LINK_CHAIN"}, path=dsp)
    bads = dict(st, buyerBps=10001)
    lbs = W("links", "bad_settle", cbytes(G.link_for(bads)))
    dbs = W("docs", "bad_settle", cbytes(bads))
    C.case("resolve-link", [lbs, dbs], "s3.3: LINK_SETTLE carrying the token", {"verdict": "LINK_SETTLE", "inner": inner("E_SETTLE_BPS")}, path=dbs)
    stb_bad5 = cbytes(dict(st, buyerBps=10001))
    lb5s = W("links", "settle_bad_chain5", cbytes(G.link_of(5, H32(), sha(stb_bad5))))
    db5s = W("docs", "settle_bad_chain5", stb_bad5)
    C.case("resolve-link", [lb5s, db5s], "s3.3 order: LINK_SETTLE before LINK_CHAIN", {"verdict": "LINK_SETTLE", "inner": inner("E_SETTLE_BPS")}, path=db5s)
    for kind_, raw_, tok_ in (("utf8", b"\xff", "E_UTF8"), ("number", b'{"a":1.5}', "E_NUMBER"), ("depth", b"[" * 129 + b"]" * 129, "E_DEPTH"), ("dup_key", b'{"a":1,"a":2}', "E_DUP_KEY"), ("key_charset", b'{"\xc3\xa9":1}', "E_KEY_CHARSET"), ("json", b"{", "E_JSON"), ("value_charset", cbytes(dict(ln, v=ln["v"] + "é")), "E_VALUE_CHARSET")):
        pc = W("links", "canon_" + kind_, raw_)
        C.case("link-check", [pc], "galeed/1 s3: " + tok_ + " on a link", fault(tok_), path=pc)
    bad5 = copy(s0); bad5["chainId"] = 5; bad5["order"]["nonce"] = "7"
    lb5 = W("links", "bad_listing_chain_5", cbytes(G.link_of(1, H32(), sha(cbytes(bad5)))))
    db5 = W("docs", "bad_listing_chain_5", cbytes(bad5))
    C.case("resolve-link", [lb5, db5], "s3.3 order: LINK_LISTING before LINK_CHAIN", {"verdict": "LINK_LISTING", "inner": inner("E_LISTING_NONCE")}, path=db5)
    r_doc, _ = G.receipt_of(s0, blk, 0)
    rdb = cbytes(r_doc)
    lr = W("links", "receipt_doc", cbytes(G.link_of(1, H32(), sha(rdb))))
    dr = W("docs", "receipt", rdb)
    C.case("resolve-link", [lr, dr], "s3.3: LINK_UNKNOWN, a receipt is no document a link points at", {"verdict": "LINK_UNKNOWN"}, path=dr)
    v2 = cbytes({"v": "eravon.listing/2"})
    lv2 = W("links", "v2", cbytes(G.link_of(1, H32(), sha(v2))))
    dv2 = W("docs", "v2", v2)
    C.case("resolve-link", [lv2, dv2], "s3.3: LINK_UNKNOWN, a later version's literal", {"verdict": "LINK_UNKNOWN"}, path=dv2)
    C.case("resolve-link", ["links/l006_missing.json", dj], "s3.3 order: the link before the hash", {"verdict": "LINK_INVALID", "inner": inner("E_LINK_MISSING")}, path=dj)
    C.case("resolve-link", [lp, dj], "s3.3 order: the hash before the kind", {"verdict": "LINK_HASH"}, path=dj)
    C.case("resolve-link", [lp], "HARNESS: an argument short", None, path=lp)
    C.case("resolve-link", [lp, "docs/absent.json"], "HARNESS: an unreadable path", None, path=lp)
    return ln, lp


# ==========================================================================
# receipts
# ==========================================================================

def gen_receipt(blk, tb, s0, path_ok):
    r, ov = G.receipt_of(s0, blk, 0)
    rb = cbytes(r)
    rp = W("receipts", "ok_secp", rb)
    C.case("receipt-check", [rp], "s4.2: a secp256k1 receipt", {"candidate": 0, "form": "secp256k1", "ok": True, "openBy": OB, "prev": ZERO20}, path=rp)
    rg, ovg = G.receipt_of(s0, blk, 2)
    rgp = W("receipts", "ok_guild", cbytes(rg))
    C.case("receipt-check", [rgp], "s4.2: a pool candidate's receipt carries seq", {"candidate": 2, "form": "secp256k1", "ok": True, "openBy": OB, "prev": ZERO20}, path=rgp)
    rk, ovk = G.receipt_of(s0, blk, 1)
    rkp = W("receipts", "ok_second_judge", cbytes(rk))
    C.case("receipt-check", [rkp], "s4.2: the second named candidate's receipt", {"candidate": 1, "form": "secp256k1", "ok": True, "openBy": OB, "prev": ZERO20}, path=rkp)
    rc = dict(r, sig="")
    rcp = W("receipts", "ok_by_call", cbytes(rc))
    C.case("receipt-check", [rcp], "s4.2: the empty string is the buyer's claim to consent by call", {"candidate": 0, "form": "call", "ok": True, "openBy": OB, "prev": ZERO20}, path=rcp)
    r0, ov0 = G.receipt_of(s0, blk, 0, open_by=0)
    r0p = W("receipts", "ok_openby_zero", cbytes(r0))
    C.case("receipt-check", [r0p], "s4.1: openBy is an int, zero included", {"candidate": 0, "form": "secp256k1", "ok": True, "openBy": 0, "prev": ZERO20}, path=r0p)
    p = W("receipts", "canon_pretty", json.dumps(r, indent=1, sort_keys=True).encode("utf-8"))
    C.case("receipt-check", [p], "galeed/1 s3.5 test 6", fault("E_NOT_CANONICAL"), path=p)
    for kind_, raw_, tok_ in (("utf8", b"\xff", "E_UTF8"), ("number", b'{"a":1.5}', "E_NUMBER"), ("depth", b"[" * 129 + b"]" * 129, "E_DEPTH"), ("dup_key", b'{"a":1,"a":2}', "E_DUP_KEY"), ("key_charset", b'{"\xc3\xa9":1}', "E_KEY_CHARSET"), ("json", b"{", "E_JSON")):
        pc = W("receipts", "canon_" + kind_, raw_)
        C.case("receipt-check", [pc], "galeed/1 s3: " + tok_ + " on a receipt", fault(tok_), path=pc)
    pc = W("receipts", "canon_value_charset", cbytes(dict(r, sig="\u00e9")))
    C.case("receipt-check", [pc], "galeed/1 s3.3: E_VALUE_CHARSET on a receipt, a value outside the skeleton charset", fault("E_VALUE_CHARSET"), path=pc)
    p = W("receipts", "root_string", b'"x"')
    global P_ROOT_STRING
    P_ROOT_STRING = p
    C.case("receipt-check", [p], "s4.2: root an object", fault("E_RECEIPT"), path=p)
    d = dict(r); del d["openBy"]
    p = W("receipts", "missing_openby", cbytes(d))
    C.case("receipt-check", [p], "s4.2: openBy is required", fault("E_RECEIPT_MISSING"), path=p)
    d = dict(r); d["milestone"] = 0
    p = W("receipts", "closed_extra", cbytes(d))
    C.case("receipt-check", [p], "s4.1: a member beyond the eight", fault("E_RECEIPT_CLOSED"), path=p)
    d = dict(r); del d["sig"]; d["x"] = 1
    p = W("receipts", "missing_before_closed", cbytes(d))
    C.case("receipt-check", [p], "s4.2 order", fault("E_RECEIPT_MISSING"), path=p)
    p = W("receipts", "v", cbytes(dict(r, v="eravon.receipt/2")))
    C.case("receipt-check", [p], "s4.2: v", fault("E_RECEIPT_V"), path=p)
    p = W("receipts", "terms_zero", cbytes(dict(r, termsHash=ZERO32)))
    C.case("receipt-check", [p], "s4.1: termsHash nonzero", fault("E_RECEIPT_TERMS"), path=p)
    p = W("receipts", "terms_short", cbytes(dict(r, termsHash="0x1")))
    C.case("receipt-check", [p], "s4.1: termsHash hex32", fault("E_RECEIPT_TERMS"), path=p)
    p = W("receipts", "candidate_string", cbytes(dict(r, candidate="0")))
    C.case("receipt-check", [p], "s4.2: candidate an int", fault("E_RECEIPT_CANDIDATE"), path=p)
    p = W("receipts", "buyer_zero", cbytes(dict(r, buyerKey=ZERO20)))
    C.case("receipt-check", [p], "s4.1: buyerKey nonzero", fault("E_RECEIPT_BUYER"), path=p)
    p = W("receipts", "buyer_upper", cbytes(dict(r, buyerKey=r["buyerKey"].upper().replace("0X", "0x"))))
    C.case("receipt-check", [p], "s4.1: buyerKey hex20, lowercase", fault("E_RECEIPT_BUYER"), path=p)
    p = W("receipts", "sig_upper", cbytes(dict(r, sig=r["sig"].upper().replace("0X", "0x"))))
    C.case("receipt-check", [p], "s1: hex65 is lowercase", fault("E_RECEIPT_SIG"), path=p)
    p = W("receipts", "payout_upper", cbytes(dict(r, buyerPayout=r["buyerPayout"].upper().replace("0X", "0x"))))
    C.case("receipt-check", [p], "s1: hex20 is lowercase; buyerPayout in uppercase", fault("E_RECEIPT_PAYOUT"), path=p)
    p = W("receipts", "terms_upper", cbytes(dict(r, termsHash=up(r["termsHash"]))))
    C.case("receipt-check", [p], "s1 hex32 is lowercase: termsHash", fault("E_RECEIPT_TERMS"), path=p)
    d = dict(r); del d["openBy"]
    p = W("receipts", "canon_pretty_before_missing", json.dumps(d, indent=1, sort_keys=True).encode("utf-8"))
    C.case("receipt-check", [p], "galeed/1 s3 order: canonical before presence", fault("E_NOT_CANONICAL"), path=p)
    p = W("receipts", "payout_short", cbytes(dict(r, buyerPayout="0x1")))
    C.case("receipt-check", [p], "s4.1: buyerPayout hex20", fault("E_RECEIPT_PAYOUT"), path=p)
    p = W("receipts", "openby_string", cbytes(dict(r, openBy="1700086400")))
    C.case("receipt-check", [p], "s4.2: openBy an int", fault("E_RECEIPT_OPENBY"), path=p)
    rpv, ovpv = G.receipt_of(s0, blk, 0, prev=addr("stage-before"))
    rpvp = W("receipts", "prev_nonzero", cbytes(rpv))
    C.case("receipt-check", [rpvp], "s4.1: a later stage's receipt names the instance before in prev", {"candidate": 0, "form": "secp256k1", "ok": True, "openBy": OB, "prev": addr("stage-before")}, path=rpvp)
    d = dict(r); del d["prev"]
    p = W("receipts", "prev_missing", cbytes(d))
    C.case("receipt-check", [p], "s4.2: prev is required", fault("E_RECEIPT_MISSING"), path=p)
    p = W("receipts", "prev_short", cbytes(dict(r, prev="0x12")))
    C.case("receipt-check", [p], "s4.2: prev a hex20", fault("E_RECEIPT_PREV"), path=p)
    p = W("receipts", "prev_upper", cbytes(dict(r, prev=up(addr("stage-before")))))
    C.case("receipt-check", [p], "s4.2: prev a hex20, lowercase; zero admitted", fault("E_RECEIPT_PREV"), path=p)
    p = W("receipts", "prev_int", cbytes(dict(r, prev=0)))
    C.case("receipt-check", [p], "s4.2: prev a string", fault("E_RECEIPT_PREV"), path=p)
    # s1, s4.2: every hex member of the receipt under every break of its form (as the records lists are, form_<list>_<member>_<break>)
    for mem, tok, base in (("buyerKey", "E_RECEIPT_BUYER", r["buyerKey"]), ("buyerPayout", "E_RECEIPT_PAYOUT", r["buyerPayout"]),
                           ("termsHash", "E_RECEIPT_TERMS", r["termsHash"]), ("prev", "E_RECEIPT_PREV", addr("stage-before"))):
        for brk, val in (("upper", "0x" + base[2:].upper()), ("short", base[:-2]), ("long", base + "00"), ("noprefix", base[2:])):
            p = W("receipts", "form_receipt_%s_%s" % (mem.lower(), brk), cbytes(dict(r, **{mem: val})))
            C.case("receipt-check", [p], "s1, s4.2: %s under the break %s" % (mem, brk), fault(tok), path=p)

    p = W("receipts", "openby_bool", cbytes(dict(r, openBy=True)))
    C.case("receipt-check", [p], "s4.2: a bool is no int", fault("E_RECEIPT_OPENBY"), path=p)
    p = W("receipts", "sig_short", cbytes(dict(r, sig="0x12")))
    C.case("receipt-check", [p], "s4.2: sig a hex65 or the empty string", fault("E_RECEIPT_SIG"), path=p)
    p = W("receipts", "sig_zero_bytes", cbytes(dict(r, sig="0x")))
    C.case("receipt-check", [p], "s4.1: 0x is no form", fault("E_RECEIPT_SIG"), path=p)
    p = W("receipts", "sig_int", cbytes(dict(r, sig=1)))
    C.case("receipt-check", [p], "s4.2: sig of no form", fault("E_RECEIPT_SIG"), path=p)
    p = W("receipts", "sig_object", cbytes(dict(r, sig={"r": "0x" + "11" * 32, "s": "0x" + "22" * 32})))
    C.case("receipt-check", [p], "s4.1: an object is no form of sig (eravon.terms/1 s5.3)", fault("E_RECEIPT_SIG"), path=p)
    p = W("receipts", "seq_string", cbytes(dict(rg, seq="3")))
    C.case("receipt-check", [p], "s4.2: seq an int", fault("E_RECEIPT_SEQ"), path=p)
    p = W("receipts", "ok_seq_on_judge_form", cbytes(dict(r, seq=3)))
    C.case("receipt-check", [p], "s4.2: seq on a judge candidate passes the form; s4.4 refuses it", {"candidate": 0, "form": "secp256k1", "ok": True, "openBy": OB, "prev": ZERO20}, path=p)
    for slug, muts, tok in (("adj_closed_before_v", {"x": 1, "v": "y"}, "E_RECEIPT_CLOSED"), ("adj_v_before_terms", {"v": "y", "termsHash": "0x1"}, "E_RECEIPT_V"),
                            ("adj_terms_before_candidate", {"termsHash": "0x1", "candidate": "0"}, "E_RECEIPT_TERMS"),
                            ("adj_candidate_before_buyer", {"candidate": "0", "buyerKey": ZERO20}, "E_RECEIPT_CANDIDATE"),
                            ("adj_buyer_before_payout", {"buyerKey": ZERO20, "buyerPayout": ZERO20}, "E_RECEIPT_BUYER"),
                            ("adj_payout_before_openby", {"buyerPayout": ZERO20, "openBy": "x"}, "E_RECEIPT_PAYOUT"),
                            ("adj_openby_before_prev", {"openBy": "x", "prev": "0x12"}, "E_RECEIPT_OPENBY"),
                            ("adj_prev_before_sig", {"prev": "0x12", "sig": "0x"}, "E_RECEIPT_PREV"),
                            ("adj_sig_before_seq", {"sig": "0x", "seq": "x"}, "E_RECEIPT_SIG")):
        p = W("receipts", slug, cbytes(dict(r, **muts)))
        C.case("receipt-check", [p], "s4.2 order", fault(tok), path=p)
    C.case("receipt-check", ["receipts/absent.json"], "HARNESS: an unreadable path", None)
    C.case("receipt-check", [], "HARNESS: no path", None, path="(none)")

    # ---- transport
    text = b"eravon-receipt:" + b64(rb)
    C.case("receipt-encode", [rp], "s4.3: the receipt text", {"ok": True, "text": text.decode("ascii")}, path=rp)
    C.case("receipt-encode", [P_ROOT_STRING], "s4.3: the receipt's fault", fault("E_RECEIPT"), path=P_ROOT_STRING)
    t = W("texts", "receipt_ok", text, ext=".txt")
    C.case("receipt-decode", [t], "s4.3: the text decodes to the receipt bytes", {"ok": True, "receipt": hx(rb)}, path=t)
    t = W("texts", "receipt_link_prefix", b"eravon://open?e=" + b64(rb), ext=".txt")
    C.case("receipt-decode", [t], "s4.3: the link prefix is not the receipt's", fault("E_RECEIPT_PREFIX"), path=t)
    b64_mutations("receipt-decode", b"eravon-receipt:", rb, "receipt", "E_RECEIPT_B64", "s4.3", {"ok": True, "receipt": hx(rb)})
    t = W("texts", "receipt_doc_fault", b"eravon-receipt:" + b64(cbytes(dict(r, openBy="x"))), ext=".txt")
    C.case("receipt-decode", [t], "s4.3: E_RECEIPT_DOC carrying the fault", fault("E_RECEIPT_DOC", inner=inner("E_RECEIPT_OPENBY")), path=t)
    t = W("texts", "receipt_doc_link", b"eravon-receipt:" + b64(cbytes(G.link_for(s0))), ext=".txt")
    C.case("receipt-decode", [t], "s4.3: a link's bytes under the receipt prefix", fault("E_RECEIPT_DOC", inner=inner("E_RECEIPT_MISSING")), path=t)
    junk = bytes([0xfb, 0xef, 0xbe, 0xff, 0xff, 0xff])
    segu = b64(junk)
    t = W("texts", "receipt_url_alphabet", b"eravon-receipt:" + segu, ext=".txt")
    C.case("receipt-decode", [t], "receipt_url_alphabet, s1: - and _ are the alphabet's two URL characters; the bytes pass the alphabet and fail the document test", fault("E_RECEIPT_DOC", inner=inner("E_UTF8")), path=t)
    segs = segu.replace(b"-", b"+").replace(b"_", b"/")
    t = W("texts", "receipt_std_alphabet", b"eravon-receipt:" + segs, ext=".txt")
    C.case("receipt-decode", [t], "receipt_std_alphabet, s1: + and / are outside the alphabet", fault("E_RECEIPT_B64"), path=t)
    rr2 = rr3 = None
    for ob_ in (OB, 170008640, 17000864, 1700086):
        cand = cbytes(G.receipt_of(s0, blk, 0, open_by=ob_)[0])
        if len(cand) % 3 == 1 and rr2 is None: rr2 = cand
        if len(cand) % 3 == 0 and rr3 is None: rr3 = cand
    assert rr2 is not None and rr3 is not None
    t = W("texts", "receipt_ok_len_mod4_2", b"eravon-receipt:" + b64(rr2), ext=".txt")
    C.case("receipt-decode", [t], "s1: a receipt segment of length 2 mod 4 decodes", {"ok": True, "receipt": hx(rr2)}, path=t)
    pr2 = W("receipts", "ok_len_mod4_2", rr2)
    C.case("receipt-encode", [pr2], "s4.3: the receipt text at a segment length 2 mod 4, no padding", {"ok": True, "text": (b"eravon-receipt:" + b64(rr2)).decode("ascii")}, path=pr2)
    rseg = bytearray(b64(rr3 + b"\x01")[:-1]) + b"B"
    t = W("texts", "receipt_trailing_bits_len_mod4_2", b"eravon-receipt:" + bytes(rseg), ext=".txt")
    C.case("receipt-decode", [t], "s1: at length 2 mod 4 the last character's four low bits are the unused ones; B sets one", fault("E_RECEIPT_B64"), path=t)
    C.case("receipt-decode", ["texts/absent.txt"], "HARNESS: an unreadable path", None)
    C.case("receipt-encode", [], "HARNESS: no path", None, path="(none)")

    # ---- bind-receipt
    def bound(ov, form="secp256k1"):
        return {"caseId": ov["salt"], "digest": ov["digest"], "form": form, "instance": ov["instance"], "salt": ov["salt"], "verdict": "RECEIPT_BOUND"}

    def by_call(ov):
        return {"caseId": ov["salt"], "digest": ov["digest"], "form": "call", "instance": ov["instance"], "salt": ov["salt"], "verdict": "RECEIPT_BY_CALL"}
    C.case("bind-receipt", [path_ok, rp], "s4.4: RECEIPT_BOUND with the four values", bound(ov), path=rp)
    C.case("bind-receipt", [path_ok, rgp], "s4.4: a pool candidate's receipt binds; seq is compared with nothing", bound(ovg), path=rgp)
    C.case("bind-receipt", [path_ok, rkp], "s4.4: the second named candidate's receipt binds", bound(ovk), path=rkp)
    C.case("bind-receipt", [path_ok, rcp], "s4.4: RECEIPT_BY_CALL with the same four values", by_call(ov), path=rcp)
    C.case("bind-receipt", [path_ok, r0p], "s4.4: openBy 0 binds; another openBy is another case", bound(ov0), path=r0p)
    C.case("bind-receipt", [path_ok, rpvp], "s4.4: a nonzero prev binds to another salt, instance and digest", bound(ovpv), path=rpvp)
    p = W("receipts", "sig_other_prev", cbytes(dict(r, prev=addr("stage-before"))))
    C.case("bind-receipt", [path_ok, p], "s4.4: another prev is another digest; the signature recovers another address", {"verdict": "RECEIPT_SIG", "inner": inner("E_SIG_SIGNER")}, path=p)

    rgs = dict(rg, seq=9)
    p = W("receipts", "ok_guild_other_seq", cbytes(rgs))
    C.case("bind-receipt", [path_ok, p], "s4.4: the buyer's seq may differ from the seller's", bound(ovg), path=p)
    C.case("bind-receipt", [P_PRETTY, rp], "s4.4: LISTING_INVALID carrying the token", {"verdict": "LISTING_INVALID", "inner": inner("E_NOT_CANONICAL")}, path=rp)
    C.case("bind-receipt", [path_ok, P_ROOT_STRING], "s4.4: RECEIPT_INVALID carrying the token", {"verdict": "RECEIPT_INVALID", "inner": inner("E_RECEIPT")}, path=P_ROOT_STRING)
    C.case("bind-receipt", [P_PRETTY, P_ROOT_STRING], "s4.4 order: the listing before the receipt", {"verdict": "LISTING_INVALID", "inner": inner("E_NOT_CANONICAL")}, path=P_ROOT_STRING)
    p = W("receipts", "terms_mismatch", cbytes(dict(r, termsHash=H32())))
    C.case("bind-receipt", [path_ok, p], "s4.4: RECEIPT_TERMS_MISMATCH", {"verdict": "RECEIPT_TERMS_MISMATCH"}, path=p)
    p = W("receipts", "receipt_invalid_before_terms_mismatch", cbytes(dict(r, termsHash=H32(), openBy="x")))
    C.case("bind-receipt", [path_ok, p], "s4.4 order: RECEIPT_INVALID before RECEIPT_TERMS_MISMATCH", {"verdict": "RECEIPT_INVALID", "inner": inner("E_RECEIPT_OPENBY")}, path=p)
    p = W("receipts", "candidate_range", cbytes(dict(r, candidate=3)))
    C.case("bind-receipt", [path_ok, p], "s4.4: RECEIPT_CANDIDATE_RANGE (three candidates)", {"verdict": "RECEIPT_CANDIDATE_RANGE"}, path=p)
    p = W("receipts", "seq_on_judge", cbytes(dict(r, seq=3)))
    C.case("bind-receipt", [path_ok, p], "s4.4: RECEIPT_SEQ, seq on a judge candidate", {"verdict": "RECEIPT_SEQ"}, path=p)
    d = dict(rg); del d["seq"]
    p = W("receipts", "seq_absent_on_guild", cbytes(d))
    C.case("bind-receipt", [path_ok, p], "s4.4: RECEIPT_SEQ, seq absent on a pool candidate", {"verdict": "RECEIPT_SEQ"}, path=p)
    p = W("receipts", "order_buyer_seller", cbytes(dict(r, buyerKey=addr("seller"))))
    C.case("bind-receipt", [path_ok, p], "s4.4: RECEIPT_ORDER carrying E_ORDER_BUYER (the seller's key)", {"verdict": "RECEIPT_ORDER", "inner": inner("E_ORDER_BUYER")}, path=p)
    p = W("receipts", "order_buyer_judge", cbytes(dict(r, buyerKey=addr("judge"))))
    C.case("bind-receipt", [path_ok, p], "s4.4: RECEIPT_ORDER, the candidate judge's key", {"verdict": "RECEIPT_ORDER", "inner": inner("E_ORDER_BUYER")}, path=p)
    rj, ovj = G.receipt_of(s0, blk, 2, buyer_tag="judge")
    p = W("receipts", "ok_judge_key_on_guild", cbytes(rj))
    C.case("bind-receipt", [path_ok, p], "s4.4: the named judge's key is no party to the pool candidate's order", bound(ovj), path=p)
    p = W("receipts", "sig_other_payout", cbytes(dict(r, buyerPayout=addr("other"))))
    C.case("bind-receipt", [path_ok, p], "s4.4: RECEIPT_SIG carrying E_SIG_SIGNER (another payout is another order)", {"verdict": "RECEIPT_SIG", "inner": inner("E_SIG_SIGNER")}, path=p)
    p = W("receipts", "sig_other_openby", cbytes(dict(r, openBy=OB + 1)))
    C.case("bind-receipt", [path_ok, p], "s4.4: another openBy is another digest", {"verdict": "RECEIPT_SIG", "inner": inner("E_SIG_SIGNER")}, path=p)
    p = W("receipts", "sig_other_candidate", cbytes(dict(r, candidate=1)))
    C.case("bind-receipt", [path_ok, p], "s4.4: the signature was over candidate 0's order", {"verdict": "RECEIPT_SIG", "inner": inner("E_SIG_SIGNER")}, path=p)
    rsv = G.sign_digest(key("buyer"), bytes.fromhex(ov["digest"][2:]))
    p = W("receipts", "sig_high_s", cbytes(dict(r, sig=G.sig_hex(rsv[0], G.N_ORDER - rsv[1], 55 - rsv[2]))))
    C.case("bind-receipt", [path_ok, p], "s4.4: RECEIPT_SIG carrying E_SIG_HIGH_S", {"verdict": "RECEIPT_SIG", "inner": inner("E_SIG_HIGH_S")}, path=p)
    p = W("receipts", "sig_v", cbytes(dict(r, sig=r["sig"][:-2] + "1d")))
    C.case("bind-receipt", [path_ok, p], "s4.4: E_SIG_V carried", {"verdict": "RECEIPT_SIG", "inner": inner("E_SIG_V")}, path=p)
    p = W("receipts", "sig_range", cbytes(dict(r, sig="0x" + "00" * 64 + "1b")))
    C.case("bind-receipt", [path_ok, p], "s4.4: E_SIG_RANGE carried", {"verdict": "RECEIPT_SIG", "inner": inner("E_SIG_RANGE")}, path=p)
    p = W("receipts", "sig_recover", cbytes(dict(r, sig=G.sig_hex(G.N_ORDER - 1, 1, 27))))
    C.case("bind-receipt", [path_ok, p], "s4.4: E_SIG_RECOVER carried", {"verdict": "RECEIPT_SIG", "inner": inner("E_SIG_RECOVER")}, path=p)
    p = W("receipts", "adj_terms_before_range", cbytes(dict(r, termsHash=H32(), candidate=9)))
    C.case("bind-receipt", [path_ok, p], "s4.4 order: terms before the range", {"verdict": "RECEIPT_TERMS_MISMATCH"}, path=p)
    p = W("receipts", "adj_range_before_seq", cbytes(dict(r, candidate=9, seq=1)))
    C.case("bind-receipt", [path_ok, p], "s4.4 order: the range before seq", {"verdict": "RECEIPT_CANDIDATE_RANGE"}, path=p)
    p = W("receipts", "adj_seq_before_order", cbytes(dict(r, seq=1, buyerKey=addr("seller"))))
    C.case("bind-receipt", [path_ok, p], "s4.4 order: seq before the order", {"verdict": "RECEIPT_SEQ"}, path=p)
    p = W("receipts", "adj_order_before_sig", cbytes(dict(r, buyerKey=addr("seller"), sig="0x" + "00" * 64 + "1b")))
    C.case("bind-receipt", [path_ok, p], "s4.4 order: the order before the signature", {"verdict": "RECEIPT_ORDER", "inner": inner("E_ORDER_BUYER")}, path=p)
    p = W("receipts", "by_call_terms_mismatch", cbytes(dict(rc, termsHash=H32())))
    C.case("bind-receipt", [path_ok, p], "s4.4: a by-call receipt is tested like any other before its slot", {"verdict": "RECEIPT_TERMS_MISMATCH"}, path=p)
    p = W("receipts", "by_call_seller_key", cbytes(dict(rc, buyerKey=addr("seller"))))
    C.case("bind-receipt", [path_ok, p], "s4.4: RECEIPT_ORDER before the by-call answer", {"verdict": "RECEIPT_ORDER", "inner": inner("E_ORDER_BUYER")}, path=p)
    ovc = G.order_values(blk, G.offer_of(blk, s0["terms"]["hash"], 0), addr("company"), r["buyerPayout"], OB)
    p = W("receipts", "by_call_company", cbytes(dict(rc, buyerKey=addr("company"))))
    C.case("bind-receipt", [path_ok, p], "s4.4: any nonzero buyerKey claims the call", by_call(ovc), path=p)
    # the listing by call and the receipt by call
    s_bc = copy(s0); s_bc["judges"][0]["sig"] = ""
    sbcp = W("listings", "ok_by_call_for_receipts", cbytes(s_bc))
    C.case("check", [sbcp], "s2.5: the listing the by-call receipt cases read", oks(s_bc), path=sbcp)
    C.case("bind-receipt", [sbcp, rcp], "s4.4: a by-call candidate and a by-call buyer, two companies", by_call(ov), path=rcp)
    C.case("bind-receipt", [sbcp, rp], "s4.4: a by-call candidate with a signing buyer", bound(ov), path=rp)
    C.case("bind-receipt", [path_ok], "HARNESS: an argument short", None, path=path_ok)
    C.case("bind-receipt", [path_ok, rp, rp], "HARNESS: an argument beyond the list", None, path=rp)
    return r, ov, rk, ovk, rg, ovg, rc


# ==========================================================================
# packs
# ==========================================================================

def gen_pack(blk, tb, s0, path_ok, r, ov, rk, ovk, rg, ovg, rc):
    ln = G.link_for(s0)
    pk = G.pack_of(s0, r, ov, link=ln)
    pb = cbytes(pk)
    pp = W("packs", "ok", pb)

    def okk(ov, r, form="secp256k1", n=0):
        return {"attachments": n, "buyerKey": r["buyerKey"], "caseId": ov["salt"], "form": form, "instance": ov["instance"], "ok": True}
    C.case("pack-check", [pp], "s5.2: a whole pack", okk(ov, r), path=pp)
    p = W("packs", "closed_key", cbytes(dict(pk, key={"address": r["buyerKey"][2:], "version": 3})))
    C.case("pack-check", [p], "s5.1: a pack carries no key member; key is beyond the eight", fault("E_PACK_CLOSED"), path=p)
    p = W("packs", "case_upper", cbytes(dict(pk, caseId=up(pk["caseId"]))))
    C.case("pack-check", [p], "s1 hex32 is lowercase: caseId", fault("E_PACK_CASE"), path=p)
    p = W("packs", "instance_upper", cbytes(dict(pk, instance=up(pk["instance"]))))
    C.case("pack-check", [p], "s1: hex20 is lowercase; instance in uppercase", fault("E_PACK_INSTANCE"), path=p)
    pkk = G.pack_of(s0, rk, ovk, link=ln)
    p = W("packs", "receipt_sig_object", cbytes(dict(pkk, receipt=dict(rk, sig={"r": "0x" + "11" * 32}))))
    C.case("pack-check", [p], "s5.2: E_PACK_RECEIPT carrying E_RECEIPT_SIG, an object in the receipt's sig", fault("E_PACK_RECEIPT", inner=inner("E_RECEIPT_SIG")), path=p)
    p = W("packs", "ok_second_judge", cbytes(pkk))
    C.case("pack-check", [p], "s5.2: the second named candidate's pack", okk(ovk, rk), path=p)
    pkg = G.pack_of(s0, rg, ovg, link=ln)
    p = W("packs", "ok_guild", cbytes(pkg))
    C.case("pack-check", [p], "s5.2: a pool candidate's pack", okk(ovg, rg), path=p)
    pkc = G.pack_of(s0, rc, ov, link=ln)
    p = W("packs", "ok_by_call", cbytes(pkc))
    C.case("pack-check", [p], "s5.2: a by-call receipt's pack", okk(ov, rc, "call"), path=p)
    p = W("packs", "ok_attachments_empty", cbytes(G.pack_of(s0, r, ov, link=ln, attachments=[])))
    C.case("pack-check", [p], "s5.1: attachments may be an empty array", okk(ov, r), path=p)
    # a document with hashed segments and the pack that carries the texts
    blk_h = G.base_block(judges=[blk["judges"][0]], goods=..., goodsHash=G.text_hash(G.GOODS), delivery_text=..., deliveryHash=G.text_hash(G.DELIVERY))
    tb_h = G.terms_doc(cbytes(blk_h))
    s_h = G.listing_of(blk_h, tb_h)
    r_h, ov_h = G.receipt_of(s_h, blk_h, 0)
    ln_h = G.link_for(s_h)
    ag = G.attachment_of(G.GOODS)
    ad = G.attachment_of(G.DELIVERY)
    p = W("packs", "ok_attachment_goods", cbytes(G.pack_of(s_h, r_h, ov_h, link=ln_h, attachments=[ag])))
    C.case("pack-check", [p], "s5.3: the goods text, its hash named in the document", okk(ov_h, r_h, n=1), path=p)
    p = W("packs", "ok_attachments_both", cbytes(G.pack_of(s_h, r_h, ov_h, link=ln_h, attachments=[ag, ad])))
    C.case("pack-check", [p], "s5.3: both segments' texts", okk(ov_h, r_h, n=2), path=p)
    p = W("packs", "ok_attachments_twice", cbytes(G.pack_of(s_h, r_h, ov_h, link=ln_h, attachments=[ag, ag])))
    C.case("pack-check", [p], "s5.3: two attachments may carry one text", okk(ov_h, r_h, n=2), path=p)
    p = W("packs", "ok_attachment_in_clear_document", cbytes(G.pack_of(s0, r, ov, link=ln, attachments=[G.attachment_of("x" * 3)] if False else [])))
    C.case("pack-check", [p], "s5.3: no attachment where the document names no hash", okk(ov, r), path=p)
    p = W("packs", "attachments_object", cbytes(G.pack_of(s_h, r_h, ov_h, link=ln_h, attachments={})))
    C.case("pack-check", [p], "s5.2: attachments an array", fault("E_PACK_ATTACHMENTS"), path=p)
    p = W("packs", "attachment_string", cbytes(G.pack_of(s_h, r_h, ov_h, link=ln_h, attachments=[ag, "x"])))
    C.case("pack-check", [p], "s5.3: an attachment an object, index 1", fault("E_PACK_ATTACHMENT", 1), path=p)
    p = W("packs", "attachment_missing", cbytes(G.pack_of(s_h, r_h, ov_h, link=ln_h, attachments=[{"sha256": ag["sha256"]}])))
    C.case("pack-check", [p], "s5.3: exactly sha256 and text_md", fault("E_PACK_ATTACHMENT", 0), path=p)
    p = W("packs", "attachment_extra", cbytes(G.pack_of(s_h, r_h, ov_h, link=ln_h, attachments=[dict(ag, name="goods")])))
    C.case("pack-check", [p], "s5.3: a member beyond the two", fault("E_PACK_ATTACHMENT", 0), path=p)
    p = W("packs", "attachment_sha_short", cbytes(G.pack_of(s_h, r_h, ov_h, link=ln_h, attachments=[dict(ag, sha256="0x1")])))
    C.case("pack-check", [p], "s5.3: sha256 hex32", fault("E_PACK_ATTACHMENT", 0), path=p)
    p = W("packs", "attachment_text_int", cbytes(G.pack_of(s_h, r_h, ov_h, link=ln_h, attachments=[dict(ag, text_md=1)])))
    C.case("pack-check", [p], "s5.3: text_md a string", fault("E_PACK_ATTACHMENT", 0), path=p)
    p = W("packs", "attachment_hash", cbytes(G.pack_of(s_h, r_h, ov_h, link=ln_h, attachments=[dict(ag, text_md=G.GOODS + " ")])))
    C.case("pack-check", [p], "s5.3: E_PACK_ATTACHMENT_HASH, the text does not hash to sha256", fault("E_PACK_ATTACHMENT_HASH", 0), path=p)
    p = W("packs", "attachment_unnamed", cbytes(G.pack_of(s_h, r_h, ov_h, link=ln_h, attachments=[G.attachment_of("a text the document does not name")])))
    C.case("pack-check", [p], "s5.3: E_PACK_ATTACHMENT_UNNAMED, a text whose hash the document does not carry", fault("E_PACK_ATTACHMENT_UNNAMED", 0), path=p)
    p = W("packs", "attachment_unnamed_clear_document", cbytes(G.pack_of(s0, r, ov, link=ln, attachments=[G.attachment_of(G.GOODS)])))
    C.case("pack-check", [p], "s5.3: the goods text's hash is not in a document that carries the text in the clear", fault("E_PACK_ATTACHMENT_UNNAMED", 0), path=p)
    p = W("packs", "attachment_second_faults", cbytes(G.pack_of(s_h, r_h, ov_h, link=ln_h, attachments=[ag, dict(ad, text_md="y")])))
    C.case("pack-check", [p], "s5.3: the first faulting element, index 1", fault("E_PACK_ATTACHMENT_HASH", 1), path=p)
    p = W("packs", "attachment_unnamed_before_next_form", cbytes(G.pack_of(s_h, r_h, ov_h, link=ln_h, attachments=[G.attachment_of("a text the document does not name"), "x"])))
    C.case("pack-check", [p], "s5.2 order: every test of element 0, its naming included, before element 1's form", fault("E_PACK_ATTACHMENT_UNNAMED", 0), path=p)
    p = W("packs", "attachment_hash_before_unnamed", cbytes(G.pack_of(s_h, r_h, ov_h, link=ln_h, attachments=[{"sha256": H32(), "text_md": "z"}])))
    C.case("pack-check", [p], "s5.3 order: the hash before the naming", fault("E_PACK_ATTACHMENT_HASH", 0), path=p)
    p = W("packs", "attachment_form_before_hash", cbytes(G.pack_of(s_h, r_h, ov_h, link=ln_h, attachments=[{"sha256": "0x1", "text_md": "z"}])))
    C.case("pack-check", [p], "s5.3 order: the form before the hash", fault("E_PACK_ATTACHMENT", 0), path=p)
    # s5.3: the digest spelled in the document's prose, and a judge's amanahHash that is a text's hash, name no attachment
    note_ = "a note"
    blk_p = G.base_block(judges=[dict(blk["judges"][0], amanahHash=G.text_hash("covenant text"))], goods=G.GOODS + " " + sha(note_.encode("utf-8")))
    tb_p = G.terms_doc(cbytes(blk_p))
    s_p = G.listing_of(blk_p, tb_p)
    r_p, ov_p = G.receipt_of(s_p, blk_p, 0)
    ln_p = G.link_for(s_p)
    p = W("packs", "attachment_prose_digest", cbytes(G.pack_of(s_p, r_p, ov_p, link=ln_p, attachments=[G.attachment_of(note_)])))
    C.case("pack-check", [p], "s5.3: a text whose digest the prose spells is named by no goodsHash or deliveryHash", fault("E_PACK_ATTACHMENT_UNNAMED", 0), path=p)
    p = W("packs", "attachment_covenant_text", cbytes(G.pack_of(s_p, r_p, ov_p, link=ln_p, attachments=[G.attachment_of("covenant text")])))
    C.case("pack-check", [p], "s5.3: a text hashing to a candidate's amanahHash is no attachment", fault("E_PACK_ATTACHMENT_UNNAMED", 0), path=p)
    p = W("packs", "case_mismatch_before_attachments", cbytes(dict(pk, caseId=H32(), attachments={})))
    C.case("pack-check", [p], "s5.2 order: the mismatches before attachments", fault("E_PACK_CASE_MISMATCH"), path=p)
    for kind_, raw_, tok_ in (("utf8", b"\xff", "E_UTF8"), ("number", b'{"a":1.5}', "E_NUMBER"), ("depth", b"[" * 129 + b"]" * 129, "E_DEPTH"), ("dup_key", b'{"a":1,"a":2}', "E_DUP_KEY"), ("key_charset", b'{"\xc3\xa9":1}', "E_KEY_CHARSET"), ("json", b"{", "E_JSON"), ("value_charset", cbytes(dict(pk, v=pk["v"] + "é")), "E_VALUE_CHARSET")):
        pc = W("packs", "canon_" + kind_, raw_)
        C.case("pack-check", [pc], "galeed/1 s3: " + tok_ + " on a pack", fault(tok_), path=pc)

    p = W("packs", "canon_pretty", json.dumps(pk, indent=1, sort_keys=True, ensure_ascii=False).encode("utf-8"))
    C.case("pack-check", [p], "galeed/1 s3.5 test 6", fault("E_NOT_CANONICAL"), path=p)
    p = W("packs", "root_array", b"[]")
    C.case("pack-check", [p], "s5.2: root an object", fault("E_PACK"), path=p)
    d = dict(pk); del d["link"]
    p_missing = W("packs", "missing", cbytes(d))
    C.case("pack-check", [p_missing], "s5.2: a required member absent", fault("E_PACK_MISSING"), path=p_missing)
    d = dict(pk); del d["link"]
    p = W("packs", "canon_pretty_before_missing", json.dumps(d, indent=1, sort_keys=True, ensure_ascii=False).encode("utf-8"))
    C.case("pack-check", [p], "galeed/1 s3 order: canonical before presence", fault("E_NOT_CANONICAL"), path=p)
    d = dict(pk); d["terms_md"] = tb.decode("utf-8")
    p = W("packs", "closed_terms_md", cbytes(d))
    C.case("pack-check", [p], "s5.1: no terms_md member; the document stands in the listing", fault("E_PACK_CLOSED"), path=p)
    d = dict(pk); d["payload"] = {}
    p = W("packs", "closed_extra", cbytes(d))
    C.case("pack-check", [p], "s5.1: a member beyond the eight", fault("E_PACK_CLOSED"), path=p)
    d = dict(pk); del d["receipt"]; d["x"] = 1
    p = W("packs", "missing_before_closed", cbytes(d))
    C.case("pack-check", [p], "s5.2 order", fault("E_PACK_MISSING"), path=p)
    p = W("packs", "v", cbytes(dict(pk, v="x")))
    C.case("pack-check", [p], "s5.2: v", fault("E_PACK_V"), path=p)
    p = W("packs", "chain_string", cbytes(dict(pk, chainId="1")))
    C.case("pack-check", [p], "s5.2: chainId an int", fault("E_PACK_CHAIN"), path=p)
    p = W("packs", "instance_short", cbytes(dict(pk, instance="0x1")))
    C.case("pack-check", [p], "s5.2: instance hex20", fault("E_PACK_INSTANCE"), path=p)
    p = W("packs", "case_short", cbytes(dict(pk, caseId="0x1")))
    C.case("pack-check", [p], "s5.2: caseId hex32", fault("E_PACK_CASE"), path=p)
    p = W("packs", "link_string", cbytes(dict(pk, link="x")))
    C.case("pack-check", [p], "s5.2: a link that is no object carries E_LINK", fault("E_PACK_LINK", inner=inner("E_LINK")), path=p)
    p = W("packs", "link_invalid", cbytes(dict(pk, link=dict(ln, tx="0x1"))))
    C.case("pack-check", [p], "s5.2: E_PACK_LINK carrying the link's fault", fault("E_PACK_LINK", inner=inner("E_LINK_TX")), path=p)
    p = W("packs", "listing_string", cbytes(dict(pk, listing="x")))
    C.case("pack-check", [p], "s5.2: a listing that is no object carries E_LISTING", fault("E_PACK_LISTING", inner=inner("E_LISTING")), path=p)
    bad = copy(s0); bad["order"]["nonce"] = "7"
    p = W("packs", "listing_invalid", cbytes(dict(pk, listing=bad)))
    C.case("pack-check", [p], "s5.2: E_PACK_LISTING carrying the listing's fault", fault("E_PACK_LISTING", inner=inner("E_LISTING_NONCE")), path=p)
    bad = copy(s0); bad["judges"][1]["sig"] = bad["judges"][0]["sig"]
    p = W("packs", "listing_invalid_sig", cbytes(dict(pk, listing=bad)))
    C.case("pack-check", [p], "s5.2: the carried fault keeps its index and inner", fault("E_PACK_LISTING", inner=inner("E_LISTING_SIG", 1, inner_=inner("E_SIG_SIGNER"))), path=p)
    p = W("packs", "hash", cbytes(dict(pk, link=dict(ln, hash=H32()))))
    C.case("pack-check", [p], "s5.2: E_PACK_HASH, the listing's bytes are not the ones the link names", fault("E_PACK_HASH"), path=p)
    bad = copy(s0); bad["guilds"][0]["seq"] = 4
    p = W("packs", "hash_other_seq", cbytes(dict(pk, listing=bad)))
    C.case("pack-check", [p], "s5.2: a listing whose seq differs is other bytes; the link names the posted ones", fault("E_PACK_HASH"), path=p)
    p = W("packs", "receipt_string", cbytes(dict(pk, receipt="x")))
    C.case("pack-check", [p], "s5.2: a receipt that is no object carries E_RECEIPT", fault("E_PACK_RECEIPT", inner=inner("E_RECEIPT")), path=p)
    p = W("packs", "receipt_invalid", cbytes(dict(pk, receipt=dict(r, openBy="x"))))
    C.case("pack-check", [p], "s5.2: E_PACK_RECEIPT carrying the receipt's fault", fault("E_PACK_RECEIPT", inner=inner("E_RECEIPT_OPENBY")), path=p)
    p = W("packs", "order_terms_mismatch", cbytes(dict(pk, receipt=dict(r, termsHash=H32()))))
    C.case("pack-check", [p], "s5.2: E_PACK_ORDER carrying RECEIPT_TERMS_MISMATCH", fault("E_PACK_ORDER", inner=inner("RECEIPT_TERMS_MISMATCH")), path=p)
    p = W("packs", "order_sig", cbytes(dict(pk, receipt=dict(r, buyerPayout=addr("other")))))
    C.case("pack-check", [p], "s5.2: E_PACK_ORDER carrying RECEIPT_SIG with its inner", fault("E_PACK_ORDER", inner=inner("RECEIPT_SIG", inner_=inner("E_SIG_SIGNER"))), path=p)
    p = W("packs", "order_buyer", cbytes(dict(pk, receipt=dict(r, buyerKey=addr("seller")))))
    C.case("pack-check", [p], "s5.2: E_PACK_ORDER carrying RECEIPT_ORDER", fault("E_PACK_ORDER", inner=inner("RECEIPT_ORDER", inner_=inner("E_ORDER_BUYER"))), path=p)
    p = W("packs", "chain_mismatch", cbytes(dict(pk, chainId=5)))
    C.case("pack-check", [p], "s5.2: E_PACK_CHAIN_MISMATCH against the listing's", fault("E_PACK_CHAIN_MISMATCH"), path=p)
    p = W("packs", "chain_mismatch_link", cbytes(dict(pk, link=dict(ln, chainId=5))))
    C.case("pack-check", [p], "s5.2: E_PACK_CHAIN_MISMATCH against the link's", fault("E_PACK_CHAIN_MISMATCH"), path=p)
    p = W("packs", "instance_mismatch", cbytes(dict(pk, instance=addr("x"))))
    C.case("pack-check", [p], "s5.2: E_PACK_INSTANCE_MISMATCH", fault("E_PACK_INSTANCE_MISMATCH"), path=p)
    p = W("packs", "case_mismatch", cbytes(dict(pk, caseId=H32())))
    C.case("pack-check", [p], "s5.2: E_PACK_CASE_MISMATCH", fault("E_PACK_CASE_MISMATCH"), path=p)
    for slug, muts, tok in (("adj_closed_before_v", {"x": 1, "v": "y"}, "E_PACK_CLOSED"), ("adj_v_before_chain", {"v": "y", "chainId": "1"}, "E_PACK_V"),
                            ("adj_chain_before_instance", {"chainId": "1", "instance": "0x1"}, "E_PACK_CHAIN"),
                            ("adj_instance_before_case", {"instance": "0x1", "caseId": "0x1"}, "E_PACK_INSTANCE"),
                            ("adj_case_before_link", {"caseId": "0x1", "link": "x"}, "E_PACK_CASE"),
                            ("adj_link_before_listing", {"link": "x", "listing": "y"}, "E_PACK_LINK"),
                            ("adj_hash_before_receipt", {"link": dict(ln, hash=H32()), "receipt": "x"}, "E_PACK_HASH"),
                            ("adj_receipt_before_chain_mismatch", {"receipt": "x", "chainId": 5}, "E_PACK_RECEIPT"),
                            ("adj_chain_mismatch_before_instance", {"chainId": 5, "instance": addr("x")}, "E_PACK_CHAIN_MISMATCH"),
                            ("adj_instance_before_case_mismatch", {"instance": addr("x"), "caseId": H32()}, "E_PACK_INSTANCE_MISMATCH"),
                            ("adj_case_mismatch_before_attachments", {"caseId": H32(), "attachments": "x"}, "E_PACK_CASE_MISMATCH")):
        d = dict(pk, **muts)
        p = W("packs", slug, cbytes(d))
        tok_f = fault(tok) if tok not in ("E_PACK_LINK", "E_PACK_RECEIPT") else fault(tok, inner=inner("E_LINK" if tok == "E_PACK_LINK" else "E_RECEIPT"))
        C.case("pack-check", [p], "s5.2 order", tok_f, path=p)
    bad = copy(s0); bad["order"]["nonce"] = "7"
    p = W("packs", "adj_listing_before_hash", cbytes(dict(pk, listing=bad, link=dict(ln, hash=H32()))))
    C.case("pack-check", [p], "s5.2 order: the listing before the hash", fault("E_PACK_LISTING", inner=inner("E_LISTING_NONCE")), path=p)
    p = W("packs", "adj_order_before_chain_mismatch", cbytes(dict(pk, receipt=dict(r, termsHash=H32()), chainId=5)))
    C.case("pack-check", [p], "s5.2 order: the receipt's binding before the chain", fault("E_PACK_ORDER", inner=inner("RECEIPT_TERMS_MISMATCH")), path=p)
    C.case("pack-check", ["packs/absent.json"], "HARNESS: an unreadable path", None)
    C.case("pack-check", [], "HARNESS: no path", None, path="(none)")

    return pk, pp


# ==========================================================================
# the settlement offer
# ==========================================================================

def gen_settle(blk, tb, s0, path_ok, r, ov, pk, pp, rc):
    st, sdg = G.settle_of(1, ov["instance"], 5000, 1800000000)
    sb = cbytes(st)
    sp = W("settles", "ok", sb)
    C.case("settle-check", [sp], "s8.2: a settlement offer", {"buyerBps": 5000, "expiry": 1800000000, "instance": ov["instance"], "ok": True}, path=sp)
    st0, _ = G.settle_of(1, ov["instance"], 0, 0)
    p = W("settles", "ok_zero", cbytes(st0))
    C.case("settle-check", [p], "s8.2: buyerBps 0 and expiry 0", {"buyerBps": 0, "expiry": 0, "instance": ov["instance"], "ok": True}, path=p)
    stc = dict(st, sig="")
    scp = W("settles", "ok_by_call", cbytes(stc))
    C.case("settle-check", [scp], "s8.2: the empty string is the seller's claim to consent by call", {"buyerBps": 5000, "expiry": 1800000000, "instance": ov["instance"], "ok": True}, path=scp)
    p = W("settles", "root_array", b"[]")
    C.case("settle-check", [p], "s8.2: root an object", fault("E_SETTLE"), path=p)
    p = W("settles", "canon_pretty", json.dumps(st, indent=1, sort_keys=True).encode("utf-8"))
    C.case("settle-check", [p], "galeed/1 s3.5 test 6", fault("E_NOT_CANONICAL"), path=p)
    for kind_, raw_, tok_ in (("utf8", b"\xff", "E_UTF8"), ("number", b'{"a":1.5}', "E_NUMBER"), ("depth", b"[" * 129 + b"]" * 129, "E_DEPTH"), ("dup_key", b'{"a":1,"a":2}', "E_DUP_KEY"), ("key_charset", b'{"\xc3\xa9":1}', "E_KEY_CHARSET"), ("json", b"{", "E_JSON"), ("value_charset", cbytes(dict(st, v=st["v"] + "é")), "E_VALUE_CHARSET")):
        pc = W("settles", "canon_" + kind_, raw_)
        C.case("settle-check", [pc], "galeed/1 s3: " + tok_ + " on a settlement offer", fault(tok_), path=pc)
    d = dict(st); del d["expiry"]
    p = W("settles", "missing", cbytes(d))
    C.case("settle-check", [p], "s8.2: a required member absent", fault("E_SETTLE_MISSING"), path=p)
    p = W("settles", "closed", cbytes(dict(st, x=1)))
    C.case("settle-check", [p], "s8.2: a member beyond the six", fault("E_SETTLE_CLOSED"), path=p)
    p = W("settles", "v", cbytes(dict(st, v="eravon.settle/2")))
    C.case("settle-check", [p], "s8.2: v", fault("E_SETTLE_V"), path=p)
    p = W("settles", "chain", cbytes(dict(st, chainId="1")))
    C.case("settle-check", [p], "s8.2: chainId an int", fault("E_SETTLE_CHAIN"), path=p)
    p = W("settles", "instance_zero", cbytes(dict(st, instance=ZERO20)))
    C.case("settle-check", [p], "s8.1: instance nonzero", fault("E_SETTLE_INSTANCE"), path=p)
    d = dict(st); del d["expiry"]; d["x"] = 1
    p = W("settles", "missing_before_closed", cbytes(d))
    C.case("settle-check", [p], "s8.2 order: presence before closure", fault("E_SETTLE_MISSING"), path=p)
    p = W("settles", "instance_upper", cbytes(dict(st, instance=st["instance"].upper().replace("0X", "0x"))))
    C.case("settle-check", [p], "s1: hex20 is lowercase; instance in uppercase", fault("E_SETTLE_INSTANCE"), path=p)
    p = W("settles", "sig_upper", cbytes(dict(st, sig=up(st["sig"]))))
    C.case("settle-check", [p], "s1 hex65 is lowercase: sig", fault("E_SETTLE_SIG"), path=p)
    d = dict(st); del d["expiry"]
    p = W("settles", "canon_pretty_before_missing", json.dumps(d, indent=1, sort_keys=True).encode("utf-8"))
    C.case("settle-check", [p], "galeed/1 s3 order: canonical before presence", fault("E_NOT_CANONICAL"), path=p)
    p = W("settles", "instance_short", cbytes(dict(st, instance="0x1")))
    C.case("settle-check", [p], "s8.1: instance hex20", fault("E_SETTLE_INSTANCE"), path=p)
    stm, _ = G.settle_of(1, ov["instance"], 10000, 1800000000)
    p = W("settles", "ok_bps_max", cbytes(stm))
    C.case("settle-check", [p], "s8.1: buyerBps 10000 passes", {"buyerBps": 10000, "expiry": 1800000000, "instance": ov["instance"], "ok": True}, path=p)
    p = W("settles", "bps_over", cbytes(dict(st, buyerBps=10001)))
    C.case("settle-check", [p], "s8.1: buyerBps at most 10000", fault("E_SETTLE_BPS"), path=p)
    p = W("settles", "bps_string", cbytes(dict(st, buyerBps="5000")))
    C.case("settle-check", [p], "s8.2: buyerBps an int", fault("E_SETTLE_BPS"), path=p)
    p = W("settles", "expiry", cbytes(dict(st, expiry=None)))
    C.case("settle-check", [p], "s8.2: expiry an int", fault("E_SETTLE_EXPIRY"), path=p)
    p = W("settles", "sig_short", cbytes(dict(st, sig="0x12")))
    C.case("settle-check", [p], "s8.2: sig hex65 or the empty string", fault("E_SETTLE_SIG"), path=p)
    p = W("settles", "sig_zero_bytes", cbytes(dict(st, sig="0x")))
    C.case("settle-check", [p], "s8.1: 0x is no form", fault("E_SETTLE_SIG"), path=p)
    p = W("settles", "sig_object", cbytes(dict(st, sig={"x": 1})))
    C.case("settle-check", [p], "s8.1: the seller's slot admits no object", fault("E_SETTLE_SIG"), path=p)
    for slug, muts, tok in (("adj_closed_before_v", {"x": 1, "v": "y"}, "E_SETTLE_CLOSED"), ("adj_v_before_chain", {"v": "y", "chainId": "1"}, "E_SETTLE_V"),
                            ("adj_chain_before_instance", {"chainId": "1", "instance": ZERO20}, "E_SETTLE_CHAIN"),
                            ("adj_instance_before_bps", {"instance": ZERO20, "buyerBps": 10001}, "E_SETTLE_INSTANCE"),
                            ("adj_bps_before_expiry", {"buyerBps": 10001, "expiry": "x"}, "E_SETTLE_BPS"),
                            ("adj_expiry_before_sig", {"expiry": "x", "sig": "0x"}, "E_SETTLE_EXPIRY")):
        p = W("settles", slug, cbytes(dict(st, **muts)))
        C.case("settle-check", [p], "s8.2 order", fault(tok), path=p)
    C.case("settle-check", ["settles/absent.json"], "HARNESS: an unreadable path", None)
    C.case("settle-check", [], "HARNESS: no path", None, path="(none)")

    # ---- bind-settle
    C.case("bind-settle", [pp, sp], "s8.3: SETTLE_BOUND with the Settle digest", {"digest": hx(sdg), "verdict": "SETTLE_BOUND"}, path=sp)
    C.case("bind-settle", [pp, scp], "s8.3: SETTLE_BY_CALL with the digest", {"digest": hx(sdg), "verdict": "SETTLE_BY_CALL"}, path=scp)
    # a later stage (terms s4.4): the receipt's prev nonzero, the pack's instance and caseId under it, a settlement over it
    rpv_, ovpv_ = G.receipt_of(s0, blk, 0, prev=addr("stage-before"))
    ppv = W("packs", "ok_later_stage", cbytes(G.pack_of(s0, rpv_, ovpv_, link=G.link_for(s0))))
    C.case("pack-check", [ppv], "s5.2, terms s4.4: a later stage's pack, its instance and caseId the ones the receipt's prev derives", {"attachments": 0, "buyerKey": rpv_["buyerKey"], "caseId": ovpv_["salt"], "form": "secp256k1", "instance": ovpv_["instance"], "ok": True}, path=ppv)
    stl, sdgl = G.settle_of(1, ovpv_["instance"], 5000, 1800000000)
    spl = W("settles", "ok_later_stage", cbytes(stl))
    C.case("bind-settle", [ppv, spl], "s8.3, terms s4.4: SETTLE_BOUND over a later stage's pack, the digest under that stage's instance", {"digest": hx(sdgl), "verdict": "SETTLE_BOUND"}, path=spl)
    C.case("bind-settle", ["packs/p040_root_array.json" if False else "packs/absent_placeholder", sp], "placeholder", None, path=sp)
    C.manifest.pop()
    bad_pack = W("packs", "settle_invalid_pack", b"[]")
    C.case("bind-settle", [bad_pack, sp], "s8.3: PACK_INVALID carrying the token", {"verdict": "PACK_INVALID", "inner": inner("E_PACK")}, path=sp)
    C.case("bind-settle", [pp, "settles/s004_root_array.json"], "s8.3: SETTLE_INVALID carrying the token", {"verdict": "SETTLE_INVALID", "inner": inner("E_SETTLE")}, path="settles/s004_root_array.json")
    C.case("bind-settle", [bad_pack, "settles/s004_root_array.json"], "s8.3 order: the pack before the offer", {"verdict": "PACK_INVALID", "inner": inner("E_PACK")}, path="settles/s004_root_array.json")
    st5, _ = G.settle_of(5, ov["instance"], 5000, 1800000000)
    p = W("settles", "chain_5", cbytes(st5))
    C.case("bind-settle", [pp, p], "s8.3: SETTLE_CHAIN", {"verdict": "SETTLE_CHAIN"}, path=p)
    p = W("settles", "settle_invalid_before_chain", cbytes(dict(st5, buyerBps=10001)))
    C.case("bind-settle", [pp, p], "s8.3 order: SETTLE_INVALID before SETTLE_CHAIN", {"verdict": "SETTLE_INVALID", "inner": inner("E_SETTLE_BPS")}, path=p)
    sto, _ = G.settle_of(1, addr("other-instance"), 5000, 1800000000)
    p = W("settles", "other_instance", cbytes(sto))
    C.case("bind-settle", [pp, p], "s8.3: SETTLE_INSTANCE", {"verdict": "SETTLE_INSTANCE"}, path=p)
    sts, _ = G.settle_of(1, ov["instance"], 5000, 1800000000, seller_tag="stranger")
    p = W("settles", "sig_stranger", cbytes(sts))
    C.case("bind-settle", [pp, p], "s8.3: SETTLE_SIG carrying E_SIG_SIGNER", {"verdict": "SETTLE_SIG", "inner": inner("E_SIG_SIGNER")}, path=p)
    blk_cf = G.base_block(judges=[blk["judges"][0]], sellerFund=addr("company-wallet"))
    tb_cf = G.terms_doc(cbytes(blk_cf))
    s_cf = G.listing_of(blk_cf, tb_cf)
    r_cf, ov_cf = G.receipt_of(s_cf, blk_cf, 0)
    pcf = W("packs", "company_fund", cbytes(G.pack_of(s_cf, r_cf, ov_cf)))
    stf, _ = G.settle_of(1, ov_cf["instance"], 5000, 1800000000, seller_tag="company-wallet")
    p = W("settles", "sig_by_fund", cbytes(stf))
    C.case("bind-settle", [pcf, p], "s8.3: SETTLE_SIG carrying E_SIG_SIGNER, the offer signed by the key of the pack's sellerFund", {"verdict": "SETTLE_SIG", "inner": inner("E_SIG_SIGNER")}, path=p)
    stk, skdg = G.settle_of(1, ov_cf["instance"], 5000, 1800000000)
    p = W("settles", "ok_company_fund", cbytes(stk))
    C.case("bind-settle", [pcf, p], "s8.3: SETTLE_BOUND under the same pack, the seller's key signing", {"digest": hx(skdg), "verdict": "SETTLE_BOUND"}, path=p)
    stn, _ = G.settle_of(1, ov["instance"], 5000, 1800000000, seller_tag="seller-new")
    p = W("settles", "sig_by_moved_key", cbytes(stn))
    C.case("bind-settle", [pp, p], "s8.3: SETTLE_SIG carrying E_SIG_SIGNER, the offer signed by the key the seller's record moved to; the predicate verifies against the listing's sellerKey (s10 Rotation)", {"verdict": "SETTLE_SIG", "inner": inner("E_SIG_SIGNER")}, path=p)
    C.case("bind-settle", [pp, scp], "s8.3: SETTLE_BY_CALL, the slot a moved seller leaves empty and registers from its current key under the former (s10 Rotation)", {"digest": hx(sdg), "verdict": "SETTLE_BY_CALL"}, path=scp)
    p = W("settles", "sig_other_bps", cbytes(dict(st, buyerBps=4000)))
    C.case("bind-settle", [pp, p], "s8.3: a signature over another buyerBps", {"verdict": "SETTLE_SIG", "inner": inner("E_SIG_SIGNER")}, path=p)
    p = W("settles", "sig_other_expiry", cbytes(dict(st, expiry=1800000001)))
    C.case("bind-settle", [pp, p], "s8.3: a signature over another expiry", {"verdict": "SETTLE_SIG", "inner": inner("E_SIG_SIGNER")}, path=p)
    p = W("settles", "sig_v", cbytes(dict(st, sig=st["sig"][:-2] + "1d")))
    C.case("bind-settle", [pp, p], "s8.3: E_SIG_V carried", {"verdict": "SETTLE_SIG", "inner": inner("E_SIG_V")}, path=p)
    rsv = G.sign_digest(key("seller"), sdg)
    p = W("settles", "sig_high_s", cbytes(dict(st, sig=G.sig_hex(rsv[0], G.N_ORDER - rsv[1], 55 - rsv[2]))))
    C.case("bind-settle", [pp, p], "s8.3: E_SIG_HIGH_S carried", {"verdict": "SETTLE_SIG", "inner": inner("E_SIG_HIGH_S")}, path=p)
    p = W("settles", "sig_range", cbytes(dict(st, sig="0x" + "00" * 64 + "1b")))
    C.case("bind-settle", [pp, p], "s8.3: E_SIG_RANGE carried", {"verdict": "SETTLE_SIG", "inner": inner("E_SIG_RANGE")}, path=p)
    p = W("settles", "sig_recover", cbytes(dict(st, sig=G.sig_hex(G.N_ORDER - 1, 1, 27))))
    C.case("bind-settle", [pp, p], "s8.3: E_SIG_RECOVER carried", {"verdict": "SETTLE_SIG", "inner": inner("E_SIG_RECOVER")}, path=p)
    p = W("settles", "adj_chain_before_instance_bind", cbytes(dict(st5, instance=addr("other-instance"))))
    C.case("bind-settle", [pp, p], "s8.3 order: the chain before the instance", {"verdict": "SETTLE_CHAIN"}, path=p)
    p = W("settles", "adj_instance_before_sig", cbytes(dict(sto, sig="0x" + "00" * 64 + "1b")))
    C.case("bind-settle", [pp, p], "s8.3 order: the instance before the signature", {"verdict": "SETTLE_INSTANCE"}, path=p)
    p = W("settles", "by_call_other_instance", cbytes(dict(stc, instance=addr("other-instance"))))
    C.case("bind-settle", [pp, p], "s8.3: a by-call offer is tested like any other before its slot", {"verdict": "SETTLE_INSTANCE"}, path=p)
    pkc = G.pack_of(s0, rc, ov, link=G.link_for(s0))
    pcp = W("packs", "by_call_for_settle", cbytes(pkc))
    C.case("bind-settle", [pcp, scp], "s8.3: a by-call buyer and a by-call seller may still close a case", {"digest": hx(sdg), "verdict": "SETTLE_BY_CALL"}, path=scp)
    C.case("bind-settle", [pp], "HARNESS: an argument short", None, path=pp)
    C.case("bind-settle", [pp, sp, sp], "HARNESS: an argument beyond the list", None, path=sp)
    C.case("bind-settle", [pp, "settles/absent.json"], "HARNESS: an unreadable path", None, path=pp)


# ==========================================================================
# s6 identity: place, pay-check, history over a record set
# ==========================================================================

KINDS = ("posted", "consented", "revoked", "pulled", "opened", "events", "declarations", "accessions")


def P_(bn, li):
    return {"blockNumber": bn, "logIndex": li}


def posted(bn, li, poster, b, key_=None):
    d = json.loads(b) if key_ is None else None
    return dict(P_(bn, li), poster=poster, key=key_ or d["terms"]["hash"], hash=sha(b), document=hx(b))


def decl(bn, li, f, t):
    return dict(P_(bn, li), **{"from": f, "to": t})


def acc(bn, li, h, t, root):
    return dict(P_(bn, li), holder=h, to=t, root=root)


def opened(bn, li, inst, case_id, seller, buyer, judge, amount, th):
    return dict(P_(bn, li), emitter=inst, caseId=case_id, seller=seller, buyer=buyer, judge=judge, amount=amount, termsHash=th)


def event(bn, li, inst, name):
    return dict(P_(bn, li), emitter=inst, name=name)


def pulled(bn, li, seller, fund, token, inst, amount):
    return dict(P_(bn, li), seller=seller, fund=fund, token=token, instance=inst, amount=amount)


def consented(bn, li, who, dg):
    return dict(P_(bn, li), who=who, digest=dg)


def revoked(bn, li, seller, th):
    return dict(P_(bn, li), seller=seller, termsHash=th)


def recset(**kinds):
    d = {"chainId": 1}
    for k in KINDS:
        d[k] = kinds.get(k, [])
    return d


def RW(slug, rec, order=None):
    """A record set written with every list of two or more elements out of position order, by seed
    (HARNESS-LINK: the elements of a list stand in any order); order="asc" writes every list in ascending
    position instead, so that a reader that reverses its lists meets one case at every seed."""
    r = random.Random("%d:records:%s" % (G.SEED, slug))
    out = dict(rec)
    for k in KINDS:
        xs = list(rec.get(k, []))
        if len(xs) >= 2:
            if order == "asc":
                out[k] = sorted(xs, key=lambda x: (x["blockNumber"], x["logIndex"]))
                continue
            ys = xs[:]
            r.shuffle(ys)
            if ys == sorted(xs, key=lambda x: (x["blockNumber"], x["logIndex"])):
                ys = ys[::-1]
            out[k] = ys
    return W("records", slug, cbytes(out))


class Model:
    """The generator's own reading of link s1 (the book's replay, currentAt, the keys of a record)
    and s6.2 to s6.5, over a record set this generator built; `valid` maps the bytes of every
    listing it built that passes s2.5 to its (sellerKey, termsHash)."""

    def __init__(self, rec, valid):
        self.rec = rec
        self.valid = valid
        self.book = sorted([("d", x) for x in rec["declarations"]] + [("a", x) for x in rec["accessions"]],
                           key=lambda kx: (kx[1]["blockNumber"], kx[1]["logIndex"]))

    @staticmethod
    def pos(x):
        return (x["blockNumber"], x["logIndex"])

    def run(self, before=None):
        root, head, pend, ent, accs = {}, {}, {}, set(), []

        def cur(a):
            r = root.get(a, a)
            return head.get(r, r)
        for kind, x in self.book:
            if before is not None and self.pos(x) >= before:
                break
            if kind == "d":
                if cur(x["from"]) == x["from"] and x["to"] not in (ZERO20, x["from"]) and x["to"] not in ent:
                    pend[x["from"]] = x["to"]
                    ent.add(x["from"])
            elif x["to"] not in ent and pend.get(x["holder"]) == x["to"] and cur(x["holder"]) == x["holder"]:
                r = root.get(x["holder"], x["holder"])
                root[x["to"]] = r
                head[r] = x["to"]
                del pend[x["holder"]]
                ent.add(x["to"])
                accs.append((r, x["to"]))
        return root, cur, accs

    def current_at(self, a, p):
        return self.run(p)[1](a)

    def keys(self, a):
        root, _cur, accs = self.run()
        r = root.get(a, a)
        return [r] + [t for rr, t in accs if rr == r]

    def own(self, x):
        v = self.valid.get(bytes.fromhex(x["document"][2:]))
        if v is None or x["key"] != v[1] or x["poster"] != v[0]:
            return None
        return v

    def documents(self):
        docs = {}
        for kind, keyf in (("posted", None), ("opened", ("seller", "termsHash")), ("revoked", ("seller", "termsHash"))):
            for x in self.rec[kind]:
                k = self.own(x) if kind == "posted" else (x[keyf[0]], x[keyf[1]])
                if k is None:
                    continue
                docs.setdefault(k, {"posted": [], "opened": [], "revoked": []})[kind].append(x)
        out = {}
        for k, d in docs.items():
            for lst in d.values():
                lst.sort(key=self.pos)
            if d["posted"]:
                f = d["posted"][0]
                p, src = self.pos(f), "self"
                st = "stands" if self.current_at(f["poster"], p) == f["poster"] else "attempt"
            elif d["opened"]:
                p, src, st = self.pos(d["opened"][0]), "opened", "stands"
            else:
                p, src, st = self.pos(d["revoked"][0]), "revoked", "stands"
            out[k] = dict(d, position=p, source=src, standing=st)
        return out

    def place(self, seller, th):
        d = self.documents().get((seller, th))
        return {"ok": True, "position": None if d is None else {"blockNumber": d["position"][0], "logIndex": d["position"][1], "source": d["source"]}}

    def pay(self, b):
        v = self.valid.get(b)
        assert v is not None
        own = sorted((x for x in self.rec["posted"] if bytes.fromhex(x["document"][2:]) == b and self.own(x) is not None), key=self.pos)
        if not own:
            return {"verdict": "UNPLACED"}
        f = own[0]
        return {"verdict": "PAYABLE" if self.current_at(f["poster"], self.pos(f)) == f["poster"] else "SELLER_KEY_RETIRED"}

    def case(self, o):
        inst = o["emitter"]
        recs = [x for x in self.rec["opened"] + self.rec["events"] if x["emitter"] == inst] + [x for x in self.rec["pulled"] if x["instance"] == inst]
        recs.sort(key=self.pos)
        return {"buyer": o["buyer"], "caseId": o["caseId"], "instance": inst, "position": P_(*self.pos(o)), "records": recs}

    def doc(self, k, d):
        posts = [dict(x, standing="stands" if self.current_at(x["poster"], self.pos(x)) == x["poster"] else "attempt") for x in d["posted"]]
        return {"cases": [self.case(o) for o in d["opened"]],
                "position": {"blockNumber": d["position"][0], "logIndex": d["position"][1], "source": d["source"]},
                "postings": posts, "revoked": d["revoked"], "sellerKey": k[0], "source": d["source"], "standing": d["standing"], "termsHash": k[1]}

    def history(self, a):
        ks = self.keys(a)
        docs = sorted(((k, d) for k, d in self.documents().items() if k[0] in ks), key=lambda kd: (kd[1]["position"], kd[0][1]))
        return {"attempts": [self.doc(k, d) for k, d in docs if d["standing"] == "attempt"],
                "cases": [self.case(o) for o in sorted((o for o in self.rec["opened"] if o["buyer"] in ks), key=self.pos)],
                "consents": sorted((x for x in self.rec["consented"] if x["who"] in ks), key=self.pos),
                "documents": [self.doc(k, d) for k, d in docs], "keys": ks, "ok": True}


def gen_identity(blk, s0):
    S1, S2, S3, K2, R0 = addr("seller"), addr("seller2"), addr("seller3"), addr("k2"), addr("seller0")
    X, B, J_ = addr("stranger"), addr("buyer"), addr("judge")
    LA = cbytes(s0)
    thA = s0["terms"]["hash"]
    d2 = copy(s0); d2["guilds"][0]["seq"] = 7
    LA2 = cbytes(d2)                                   # another listing of document A: the seller's reading of the book differs
    valid = {LA: (S1, thA), LA2: (S1, thA)}

    def listing(seller_tag, nonce):
        b_ = G.base_block(seller=seller_tag, judges=[blk["judges"][0]], nonce=nonce)
        t_ = G.terms_doc(cbytes(b_))
        lst = G.listing_of(b_, t_, seller_tag=seller_tag)
        lb = cbytes(lst)
        valid[lb] = (addr(seller_tag), lst["terms"]["hash"])
        return lb, lst["terms"]["hash"]
    LC, thC = listing("seller", 11)
    LD, thD = listing("seller3", 12)
    LF, thF = listing("k2", 13)
    LE, thEE = listing("seller2", 14)
    ple = W("listings", "identity_e", LE)
    thE = H32()                                        # a document never posted, opened by call
    thR = H32()                                        # a document only revoked
    byc = copy(s0)
    for el in byc["judges"] + byc["guilds"]:
        el["sig"] = ""
    LBC = cbytes(byc)                                  # every candidate claimed by call: a listing anyone could write naming S1
    valid[LBC] = (S1, thA)
    pl_ = W("listings", "identity_a", LA)
    pl2 = W("listings", "identity_a_other_seq", LA2)
    plc = W("listings", "identity_by_call_forged", LBC)
    C.case("check", [plc], "s2.5: a listing whose every candidate is claimed by call needs no signature to pass", oks(byc), path=plc)
    I1, I2, I3 = addr("instance1"), addr("instance2"), addr("instance3")
    cid1, cid2, cid3 = H32(), H32(), H32()
    tok = addr("token")

    def twin(rec):
        return any(len(rec.get(k, [])) >= 2 for k in KINDS)

    def PL(slug, rec, seller, th, note):
        pred = Model(rec, valid).place(seller, th)
        p = RW(slug, rec)
        C.case("place", [p, seller, th], note, pred, path=p)
        if twin(rec):
            p2 = RW(slug + "_asc", rec, "asc")
            C.case("place", [p2, seller, th], note + "; every list written in ascending position", pred, path=p2)

    def PC(slug, rec, lpath, lb, note):
        pred = Model(rec, valid).pay(lb)
        p = RW(slug, rec)
        C.case("pay-check", [lpath, p], note, pred, path=p)
        if twin(rec):
            p2 = RW(slug + "_asc", rec, "asc")
            C.case("pay-check", [lpath, p2], note + "; every list written in ascending position", pred, path=p2)

    def HI(slug, rec, a, note):
        pred = Model(rec, valid).history(a)
        p = RW(slug, rec)
        C.case("history", [p, a], note, pred, path=p)
        if twin(rec):
            p2 = RW(slug + "_asc", rec, "asc")
            C.case("history", [p2, a], note + "; every list written in ascending position", pred, path=p2)
        return p

    # ---- s6.2 position: three sources, none, the order of the sources, reposts and refusals
    full = recset(posted=[posted(10, 0, S1, LA)], opened=[opened(20, 1, I1, cid1, S1, B, J_, "1000000000", thA)], revoked=[revoked(30, 0, S1, thA)])
    PL("place_self", full, S1, thA, "s6.2: the seller's own posting places the document (self) before any opened or revoked")
    PL("place_opened", recset(opened=[opened(20, 1, I1, cid1, S1, B, J_, "1000000000", thA)], revoked=[revoked(30, 0, S1, thA)]), S1, thA, "s6.2: no own posting, the least opened places it (opened)")
    PL("place_revoked", recset(revoked=[revoked(30, 0, S1, thA), revoked(25, 4, S1, thA)]), S1, thA, "s6.2: only revocations, the least places it (revoked)")
    PL("place_none", recset(), S1, thA, "s6.2: a pair drawn from no record is no document")
    PL("place_other_terms", full, S1, H32(), "s6.2: another termsHash under the same seller is no document")
    PL("place_opened_first_self_wins", recset(posted=[posted(10, 0, S1, LA)], opened=[opened(5, 0, I1, cid1, S1, B, J_, "1000000000", thA)]), S1, thA, "s6.2: the first source that exists, self, though an opened is earlier")
    PL("place_least_posting", recset(posted=[posted(12, 3, S1, LA), posted(10, 7, S1, LA2)]), S1, thA, "s6.2: the least position among the seller's own postings of the document")
    PL("place_repost", recset(posted=[posted(10, 0, X, LA)]), S1, thA, "s6.2: a posting from another poster is a repost and places nothing")
    PL("place_forged_by_call", recset(posted=[posted(10, 0, X, LBC)]), S1, thA, "s6.2: a third party's posting of a by-call listing naming the seller places nothing")
    PL("place_document_invalid", recset(posted=[posted(10, 0, S1, b'{"v":"eravon.listing/1"}', key_=thA)]), S1, thA, "s6.2: bytes that fail s2.5 draw no document")
    PL("place_key_mismatch", recset(posted=[posted(10, 0, S1, LA, key_=H32())]), S1, thA, "s6.2: a posting whose key is not the listing's terms.hash is no posting of its own")
    PL("place_repost_then_open", recset(posted=[posted(10, 0, X, LA)], opened=[opened(20, 1, I1, cid1, S1, B, J_, "1000000000", thA)]), S1, thA, "s6.2: a repost before an opened leaves the opened to place it")

    # ---- s6.4 the check before paying
    mv = [decl(5, 0, S1, S2)], [acc(6, 0, S1, S2, S1)]
    PC("pay_payable", recset(posted=[posted(10, 0, S1, LA)]), pl_, LA, "s6.4: posted by its seller while current, PAYABLE")
    PC("pay_unplaced", recset(), pl_, LA, "s6.4: no posting of its own, UNPLACED")
    PC("pay_unplaced_opened_revoked", recset(opened=[opened(20, 1, I1, cid1, S1, B, J_, "1000000000", thA)], revoked=[revoked(30, 0, S1, thA)]), pl_, LA, "s6.4: a document placed by opened or revoked alone leaves the listing UNPLACED")
    PC("pay_retired", recset(posted=[posted(10, 0, S1, LA)], declarations=mv[0], accessions=mv[1]), pl_, LA, "s6.4: first posted after its key handed the record on, SELLER_KEY_RETIRED")
    PC("pay_pending", recset(posted=[posted(10, 0, S1, LA)], declarations=mv[0]), pl_, LA, "s6.4: a declaration no accession took up moves nothing, PAYABLE")
    PC("pay_before_accession", recset(posted=[posted(10, 0, S1, LA)], declarations=[decl(11, 0, S1, S2)], accessions=[acc(12, 0, S1, S2, S1)]), pl_, LA, "s6.4: posted before the accession, payable after it")
    PC("pay_posted_twice", recset(posted=[posted(10, 0, S1, LA), posted(13, 0, S1, LA)], declarations=[decl(11, 0, S1, S2)], accessions=[acc(12, 0, S1, S2, S1)]), pl_, LA, "s6.4: posted before and again after the accession; the least posting decides, PAYABLE")
    sib = recset(posted=[posted(10, 0, S1, LA), posted(13, 0, S1, LA2)], declarations=[decl(11, 0, S1, S2)], accessions=[acc(12, 0, S1, S2, S1)])
    PC("pay_sibling_after", sib, pl2, LA2, "s6.4: another listing of the same document, first posted after the accession, SELLER_KEY_RETIRED")
    PC("pay_sibling_first", sib, pl_, LA, "s6.4: the verdict is the listing's own; the earlier listing of the document stays PAYABLE")
    PC("pay_third_party", recset(posted=[posted(10, 0, X, LA)]), pl_, LA, "s6.4: a listing its seller never posted is unplaced, whoever posted it")
    PC("pay_forged_by_call", recset(posted=[posted(10, 0, X, LBC)]), plc, LBC, "s6.4: a by-call listing a third party posted naming the seller, UNPLACED")
    PC("pay_order_unplaced_before_retired", recset(posted=[posted(10, 0, X, LA)], declarations=mv[0], accessions=mv[1]), pl_, LA, "s6.4 order: no own posting answers UNPLACED before any standing is read")
    PC("pay_same_block_before", recset(posted=[posted(9, 1, S1, LA)], declarations=[decl(5, 0, S1, S2)], accessions=[acc(9, 2, S1, S2, S1)]), pl_, LA, "s1 position: the posting at logIndex 1 precedes the accession at logIndex 2 of the same block, PAYABLE")
    PC("pay_same_block_after", recset(posted=[posted(9, 3, S1, LA)], declarations=[decl(5, 0, S1, S2)], accessions=[acc(9, 2, S1, S2, S1)]), pl_, LA, "s1 position: the posting at logIndex 3 follows the accession at logIndex 2, SELLER_KEY_RETIRED")
    PC("pay_block_before_index", recset(posted=[posted(8, 9, S1, LA)], declarations=[decl(5, 0, S1, S2)], accessions=[acc(9, 0, S1, S2, S1)]), pl_, LA, "s1 position: blockNumber is compared first; block 8 at logIndex 9 precedes block 9 at logIndex 0")
    PC("pay_acceded_key", recset(posted=[posted(10, 0, S1, LA)], declarations=[decl(3, 0, R0, S1)], accessions=[acc(4, 0, R0, S1, R0)]), pl_, LA, "s1: a key that acceded is its record's current key; its posting stands, PAYABLE")
    PC("pay_accession_unmatched", recset(posted=[posted(10, 0, S1, LA)], accessions=[acc(6, 0, S1, S2, S1)]), pl_, LA, "s1: an accession no declaration made pending counts for nothing, PAYABLE")
    PC("pay_declaration_to_self", recset(posted=[posted(10, 0, S1, LA)], declarations=[decl(5, 0, S1, S1)], accessions=[acc(6, 0, S1, S1, S1)]), pl_, LA, "s1: a declaration naming its own key binds nothing")
    PC("pay_declaration_to_zero", recset(posted=[posted(10, 0, S1, LA)], declarations=[decl(5, 0, S1, ZERO20)], accessions=[acc(6, 0, S1, ZERO20, S1)]), pl_, LA, "s1: a declaration naming the zero address binds nothing")
    PC("pay_declaration_to_entered", recset(posted=[posted(10, 0, S1, LA)], declarations=[decl(3, 0, S2, S3), decl(5, 0, S1, S2)], accessions=[acc(6, 0, S1, S2, S1)]), pl_, LA, "s1: a key that entered by declaring is named by no declaration that binds; S1 keeps its record, PAYABLE")
    PC("pay_redeclared", recset(posted=[posted(10, 0, S1, LA)], declarations=[decl(4, 0, S1, S3), decl(5, 0, S1, S2)], accessions=[acc(6, 0, S1, S3, S1)]), pl_, LA, "s1: a later declaration replaces the pending key; the earlier key's accession counts for nothing, PAYABLE")
    PC("pay_redeclared_taken", recset(posted=[posted(10, 0, S1, LA)], declarations=[decl(4, 0, S1, S3), decl(5, 0, S1, S2)], accessions=[acc(6, 0, S1, S2, S1)]), pl_, LA, "s1: the pending key in place accedes, SELLER_KEY_RETIRED")
    pinv = W("listings", "identity_invalid", json.dumps(s0, indent=1, sort_keys=True).encode("utf-8"))
    p = RW("pay_listing_invalid", recset(posted=[posted(10, 0, S1, LA)]))
    C.case("pay-check", [pinv, p], "s6.4: LISTING_INVALID carrying the token", {"verdict": "LISTING_INVALID", "inner": inner("E_NOT_CANONICAL")}, path=p)
    p = RW("pay_listing_invalid_empty", recset())
    C.case("pay-check", [pinv, p], "s6.4 order: LISTING_INVALID before UNPLACED", {"verdict": "LISTING_INVALID", "inner": inner("E_NOT_CANONICAL")}, path=p)

    # ---- s6.5 history
    HI("history_one_key", recset(posted=[posted(10, 0, S1, LA), posted(11, 0, X, LA)],
                                 opened=[opened(20, 0, I1, cid1, S1, B, J_, "1000000000", thA)],
                                 events=[event(20, 1, I1, "Seated"), event(22, 0, I2, "Seated")],
                                 pulled=[pulled(20, 2, S1, S1, tok, I1, "100")],
                                 consented=[consented(15, 0, S1, H32()), consented(16, 0, X, H32())],
                                 revoked=[revoked(30, 0, S1, thR)]),
       S1, "s6.5: one key; a document with its posting and its case, a document only revoked, the key's consents; the repost and the stranger's consent not listed")
    three = recset(posted=[posted(10, 0, S1, LA), posted(13, 0, S1, LC), posted(16, 0, S3, LD), posted(18, 0, X, LD)],
                   declarations=[decl(11, 0, S1, S2), decl(14, 0, S2, S3)],
                   accessions=[acc(12, 0, S1, S2, S1), acc(15, 0, S2, S3, S1)],
                   opened=[opened(17, 0, I3, cid3, S1, B, J_, "1000000000", thE)],
                   events=[event(17, 1, I3, "Seated")])
    for who, tag in ((S1, "first"), (S2, "second"), (S3, "third")):
        HI("history_three_keys_" + tag, three, who, "s6.5: a record of three keys read from its %s key; a document its first key posted after handing the record on is an attempt; one only opened stands" % tag)
    acceded = recset(posted=[posted(5, 0, K2, LF), posted(10, 0, S1, LA)],
                     consented=[consented(6, 0, K2, H32())],
                     declarations=[decl(20, 0, S1, K2)], accessions=[acc(21, 0, S1, K2, S1)],
                     opened=[opened(25, 0, I2, cid2, S1, K2, J_, "1000000000", thA)])
    for who, tag in ((S1, "root"), (K2, "acceded")):
        HI("history_acceded_" + tag, acceded, who, "s6.5: a key that acceded brings its earlier acts into the record (read from its %s key); a case where a key of the record is the buyer" % tag)
    HI("history_declaration_not_current", recset(declarations=[decl(3, 0, S1, S2), decl(5, 0, S1, S3)], accessions=[acc(4, 0, S1, S2, S1), acc(6, 0, S1, S3, S1)]),
       S1, "s1: a declaration by a key no longer current binds nothing, so the second accession counts for nothing; the record keeps two keys")
    HI("history_empty", recset(), addr("nobody"), "s6.5: an address with no records: its own record of one key, every list empty")
    HI("history_buyer", recset(opened=[opened(20, 0, I1, cid1, S2, S1, J_, "5", thR), opened(21, 0, I2, cid2, S2, X, J_, "5", thR)],
                               events=[event(20, 1, I1, "Opened")]),
       S1, "s6.5: cases where the key is the buyer, written as a document's case; another buyer's case not listed")

    # ---- s1 the replay, each clause where it alone decides the answer
    PC("pay_declaration_to_entered_keeps_pending", recset(posted=[posted(10, 0, S1, LA)], declarations=[decl(3, 0, S1, S3), decl(4, 0, S2, X), decl(5, 0, S1, S2)], accessions=[acc(6, 0, S1, S3, S1)]),
       pl_, LA, "s1: a declaration naming a key that entered binds nothing and leaves the pending key in place; that key accedes, SELLER_KEY_RETIRED")
    HI("history_declaration_to_self_first", recset(declarations=[decl(3, 0, S1, S1), decl(4, 0, S2, S1)], accessions=[acc(5, 0, S2, S1, S2)]),
       S1, "s1: a declaration naming its own key binds nothing and its key does not enter; named by another key, it accedes")
    PC("pay_accession_to_entered", recset(posted=[posted(10, 0, S1, LA)], declarations=[decl(3, 0, S1, X), decl(4, 0, X, addr("stranger2"))], accessions=[acc(5, 0, S1, X, S1)]),
       pl_, LA, "s1: an accession whose key entered after it was declared counts for nothing, PAYABLE")
    PC("pay_accession_enters", recset(posted=[posted(10, 0, S1, LA)], declarations=[decl(3, 0, S2, X), decl(5, 0, S1, X)], accessions=[acc(4, 0, S2, X, S2), acc(6, 0, S1, X, S1)]),
       pl_, LA, "s1: a key that acceded has entered, so a later declaration naming it binds nothing, PAYABLE")
    PC("pay_no_merger", recset(posted=[posted(10, 0, S2, LE)], declarations=[decl(3, 0, S1, X), decl(4, 0, S2, X)], accessions=[acc(5, 0, S1, X, S1), acc(6, 0, S2, X, S2)]),
       ple, LE, "s1: a key two holders declared accedes to the first; its second accession counts for nothing, and no two records merge, PAYABLE")
    PC("pay_accession_root_unread", recset(posted=[posted(10, 0, S1, LA)], declarations=[decl(5, 0, S1, S2)], accessions=[acc(6, 0, S1, S2, X)]),
       pl_, LA, "s1, s6.1: an accession's root is read by no step; the record moves, SELLER_KEY_RETIRED")
    PC("pay_same_block_book_order", recset(posted=[posted(10, 0, S1, LA)], declarations=[decl(5, 1, S1, S2)], accessions=[acc(5, 0, S1, S2, S1)]),
       pl_, LA, "s1 position: in one block the accession at logIndex 0 precedes the declaration at logIndex 1 and counts for nothing, PAYABLE")
    PC("pay_book_unsorted", recset(posted=[posted(10, 0, S1, LA)], declarations=[decl(5, 0, S1, S2), decl(3, 0, S1, S3)], accessions=[acc(6, 0, S1, S2, S1)]),
       pl_, LA, "s1: the replay walks the records in ascending position whatever order the lists give, SELLER_KEY_RETIRED")
    # ---- s6.2's own posting under place, pay-check and history
    bad = copy(s0)
    sg = bad["judges"][0]["sig"]
    bad["judges"][0]["sig"] = sg[:-2] + ("00" if sg[-2:] != "00" else "01")
    LBAD = cbytes(bad)
    PL("place_own_bad_sig", recset(posted=[posted(10, 0, S1, LBAD, key_=thA)]), S1, thA,
       "s6.2: the seller's own posting of bytes that fail s2.5 at a signature draws no document")
    PC("pay_key_mismatch", recset(posted=[posted(10, 0, S1, LA, key_=H32())]), pl_, LA,
       "s6.4: the seller's posting under a key other than terms.hash is no posting of its own, UNPLACED")
    HI("history_key_mismatch", recset(posted=[posted(10, 0, S1, LA, key_=H32())], opened=[opened(20, 1, I1, cid1, S1, B, J_, "1000000000", thA)]),
       S1, "s6.5: a posting under another key is none of the document's postings")
    PL("place_hash_unread", recset(posted=[dict(posted(10, 0, S1, LA), hash=H32())]), S1, thA,
       "s6.1: a posting's hash is read by no predicate; the seller's posting places the document")
    PC("pay_chain_unread", dict(recset(posted=[posted(10, 0, S1, LA)]), chainId=5), pl_, LA,
       "s6.1: no predicate compares the listing's chainId with the records' chain, PAYABLE")
    # ---- s6.2, s6.3 and s6.5's orders and standings
    PL("place_least_opened", recset(opened=[opened(25, 0, I2, cid2, S1, B, J_, "5", thA), opened(20, 1, I1, cid1, S1, B, J_, "1000000000", thA)]), S1, thA,
       "s6.2: the least position of an opened that draws the document, whatever the list's order")
    HI("history_posted_across_move", recset(posted=[posted(10, 0, S1, LA), posted(13, 0, S1, LA2)], declarations=[decl(11, 0, S1, S2)], accessions=[acc(12, 0, S1, S2, S1)]),
       S1, "s6.3, s6.5: a document posted while its key was current and again after the hand-on stands, its later posting an attempt")
    I4 = addr("instance4")
    HI("history_lists_order", recset(posted=[posted(30, 0, S1, LC)],
                                     opened=[opened(25, 2, I2, cid2, S1, B, J_, "5", thA), opened(20, 2, I1, cid1, S1, B, J_, "1000000000", thA),
                                             opened(50, 0, I3, cid3, S3, S1, J_, "5", thR), opened(45, 0, I4, H32(), S3, S1, J_, "5", thR)],
                                     pulled=[pulled(25, 0, S1, S1, tok, I2, "5"), pulled(20, 0, S1, S1, tok, I1, "100")],
                                     events=[event(25, 1, I2, "Seated"), event(20, 1, I1, "Seated")],
                                     revoked=[revoked(41, 0, S1, thA), revoked(40, 0, S1, thA)],
                                     consented=[consented(15, 0, S1, H32()), consented(14, 0, S1, H32())]),
       S1, "s6.5: every list in ascending position: two documents, the one an opened places before the one a posting places; two cases of one document, each with init's Pulled, Seated and Opened; two revocations; two consents; two buyer's cases")
    HI("history_cases_unsorted", recset(opened=[opened(25, 2, I2, cid2, S1, B, J_, "5", thA), opened(20, 2, I1, cid1, S1, B, J_, "1000000000", thA)]),
       S1, "s6.5: a document's cases in ascending position, the list given out of order")
    HI("history_buyer_cases_unsorted", recset(opened=[opened(50, 0, I3, cid3, S3, S1, J_, "5", thR), opened(45, 0, I4, H32(), S3, S1, J_, "5", thR)]),
       S1, "s6.5: the buyer's cases in ascending position, the list given out of order")
    HI("history_event_name_any", recset(opened=[opened(20, 0, I1, cid1, S2, S1, J_, "5", thR)], events=[event(20, 1, I1, "\u00e9\u0000")]),
       S1, "HARNESS <records.json>: an event's name is any string")
    # ---- s1 position and s6.3 where one source alone decides
    PC("pay_same_block_rotation", recset(posted=[posted(10, 0, S1, LA)], declarations=[decl(5, 0, S1, S2)], accessions=[acc(5, 1, S1, S2, S1)]),
       pl_, LA, "s1 position: a declaration at logIndex 0 and its accession at logIndex 1 of one block; the accession counts, SELLER_KEY_RETIRED")
    HI("history_same_block_rotation", recset(posted=[posted(10, 0, S1, LA)], declarations=[decl(5, 0, S1, S2)], accessions=[acc(5, 1, S1, S2, S1)]),
       S1, "s1 position: the declaration and its accession in one block, the declaration first; the record has two keys and the later posting is an attempt")
    HI("history_revoked_after_move", recset(declarations=[decl(5, 0, S1, S2)], accessions=[acc(6, 0, S1, S2, S1)], revoked=[revoked(20, 0, S1, thR)]),
       S1, "s6.3: a document drawn from a revoked alone stands, the revoked sent after the record moved; no attempt")
    PL("place_revoked_before_opened", recset(opened=[opened(20, 1, I1, cid1, S1, B, J_, "1000000000", thA)], revoked=[revoked(15, 0, S1, thA)]), S1, thA,
       "s6.2: an opened places the document before a revoked, though the revoked stands earlier")
    # ---- s6.5's orders against a reader that sorts by value or reverses its lists
    dA, dB = sorted((H32(), H32()))
    HI("history_consents_against_digest", recset(consented=[consented(15, 0, S1, dB), consented(16, 0, S1, dA)]),
       S1, "s6.5: consents in ascending position, their digests in the opposite order")
    ka, kb, kc = sorted((S1, S2, S3), reverse=True)
    HI("history_keys_against_address", recset(declarations=[decl(3, 0, ka, kb), decl(5, 0, kb, kc)], accessions=[acc(4, 0, ka, kb, ka), acc(6, 0, kb, kc, ka)]),
       kc, "s1, s6.5: the keys of a record in the order they acceded, their addresses in the opposite order")
    firstL, secondL = (LA, LC) if thA > thC else (LC, LA)
    HI("history_two_attempts", recset(posted=[posted(10, 0, S1, firstL), posted(11, 0, S1, secondL)], declarations=[decl(5, 0, S1, S2)], accessions=[acc(6, 0, S1, S2, S1)]),
       S1, "s6.5: two attempts in ascending position, their termsHash in the opposite order")
    cA, cB = sorted((H32(), H32()))
    iA, iB = sorted((addr("instance5"), addr("instance6")))
    HI("history_cases_against_values", recset(posted=[posted(10, 0, S1, LA)], opened=[opened(20, 0, iB, cB, S1, B, J_, "5", thA), opened(25, 0, iA, cA, S1, B, J_, "5", thA)]),
       S1, "s6.5: a document's cases in ascending position, their caseId and instance in the opposite order")
    HI("history_buyer_cases_against_values", recset(opened=[opened(20, 0, iB, cB, S3, S1, J_, "5", thR), opened(25, 0, iA, cA, S3, S1, J_, "5", thR)]),
       S1, "s6.5: the buyer's cases in ascending position, their caseId and instance in the opposite order")
    HI("history_case_records_order", recset(posted=[posted(10, 0, S1, LA)],
                                            opened=[opened(20, 0, I1, cid1, S1, B, J_, "1000000000", thA)],
                                            events=[event(20, 1, I1, "Seated"), event(21, 0, I1, "Accepted")],
                                            pulled=[pulled(20, 2, S1, S1, tok, I1, "5"), pulled(22, 0, S1, S1, tok, I1, "7")]),
       S1, "s6.5: a case's records of three kinds, two of each kind the record set holds, merged in ascending position whatever order the lists give")
    firstP, secondP = (LA, LA2) if sha(LA) > sha(LA2) else (LA2, LA)
    HI("history_postings_against_hash", recset(posted=[posted(10, 0, S1, firstP), posted(11, 0, S1, secondP)]),
       S1, "s6.5: a document's two own postings in ascending position, their hashes in the opposite order")

    # ---- HARNESS: <records.json> misuse and the commands' arguments
    good = RW("ok_empty", recset())
    C.case("place", [good, S1, thA], "HARNESS: an empty record set is a record set", {"ok": True, "position": None}, path=good)

    def MIS(slug, raw, note):
        p = W("records", slug, raw)
        C.case("history", [p, S1], "HARNESS <records.json>: " + note, None, path=p)
        return p
    base = recset(posted=[posted(10, 0, S1, LA)])
    MIS("root_array", b"[]", "the root is an object")
    p_unread = MIS("unreadable", b"{", "a file the reader refuses")
    d = dict(base); del d["events"]; MIS("missing_list", cbytes(d), "every list present")
    MIS("extra_member", cbytes(dict(base, logs=[])), "no member beyond chainId and the eight lists")
    MIS("chain_string", cbytes(dict(base, chainId="1")), "chainId an int")
    MIS("list_object", cbytes(dict(base, events={})), "each list an array")
    MIS("element_string", cbytes(dict(base, events=["x"])), "an element is an object")
    MIS("element_missing", cbytes(dict(base, posted=[{k: v for k, v in base["posted"][0].items() if k != "hash"}])), "an element carries its members")
    MIS("element_extra", cbytes(dict(base, posted=[dict(base["posted"][0], x=1)])), "an element carries no member beyond its list")
    MIS("element_no_position", cbytes(dict(base, events=[{"emitter": I1, "name": "x"}])), "blockNumber and logIndex present")
    MIS("position_string", cbytes(dict(base, events=[dict(event(1, 0, I1, "x"), blockNumber="1")])), "blockNumber an int")
    # every member of every list once outside its form (HARNESS <records.json>: the members and their forms)
    FORMS = {"posted": (("poster", "hex20"), ("key", "hex32"), ("hash", "hex32")),
             "consented": (("who", "hex20"), ("digest", "hex32")),
             "revoked": (("seller", "hex20"), ("termsHash", "hex32")),
             "pulled": (("seller", "hex20"), ("fund", "hex20"), ("token", "hex20"), ("instance", "hex20"), ("amount", "dec256")),
             "opened": (("emitter", "hex20"), ("caseId", "hex32"), ("seller", "hex20"), ("buyer", "hex20"), ("judge", "hex20"), ("amount", "dec256"), ("termsHash", "hex32")),
             "events": (("emitter", "hex20"),),
             "declarations": (("from", "hex20"), ("to", "hex20")),
             "accessions": (("holder", "hex20"), ("to", "hex20"), ("root", "hex20"))}
    SAMPLE = {"posted": posted(1, 0, S1, LA), "consented": consented(1, 0, S1, H32()), "revoked": revoked(1, 0, S1, thA),
              "pulled": pulled(1, 0, S1, S1, tok, I1, "5"), "opened": opened(1, 0, I1, cid1, S1, B, J_, "5", thA),
              "events": event(1, 0, I1, "x"), "declarations": decl(1, 0, S1, S2), "accessions": acc(1, 0, S1, S2, S1)}
    BREAKS = {"hex20": (("upper", up), ("short", lambda v: v[:-2]), ("long", lambda v: v + "00"), ("noprefix", lambda v: v[2:])),
              "hex32": (("upper", up), ("short", lambda v: v[:-2]), ("long", lambda v: v + "00"), ("noprefix", lambda v: v[2:])),
              "dec256": (("leading_zero", lambda v: "01"), ("sign", lambda v: "-1"), ("exp", lambda v: "1e3"), ("empty", lambda v: ""))}
    for kind_, members_ in FORMS.items():
        for name_, form_ in members_:
            for brk_, fn_ in BREAKS[form_]:
                el = dict(SAMPLE[kind_])
                el[name_] = fn_(el[name_])
                MIS("form_%s_%s_%s" % (kind_, name_, brk_), cbytes(recset(**{kind_: [el]})), "%s.%s a %s: %s" % (kind_, name_, form_, brk_))
    MIS("duplicate_member", b'{"accessions":[],"chainId":1,"chainId":1,"consented":[],"declarations":[],"events":[],"opened":[],"posted":[],"pulled":[],"revoked":[]}', "no member twice (the reader)")
    MIS("logindex_negative", cbytes(recset(events=[dict(event(1, 0, I1, "x"), logIndex=-1)])), "logIndex an int as galeed/1 s3.2 spells one, no sign")
    MIS("document_odd", cbytes(dict(base, posted=[dict(base["posted"][0], document="0xabc")])), "document 0x and an even number of lowercase digits")
    MIS("document_upper", cbytes(dict(base, posted=[dict(base["posted"][0], document=up(base["posted"][0]["document"]))])), "document digits lowercase")
    MIS("name_int", cbytes(dict(base, events=[dict(event(1, 0, I1, "x"), name=1)])), "an event's name a string")
    MIS("shared_position", cbytes(dict(base, events=[event(10, 0, I1, "x")])), "no two elements of the whole object share a position")
    C.case("history", ["records/absent.json", S1], "HARNESS: an unreadable path", None)
    C.case("history", [good, up(S1)], "HARNESS: <address> a hex20, lowercase", None, path=good)
    C.case("history", [good], "HARNESS: an argument short", None, path=good)
    C.case("history", [good, S1, S1], "HARNESS: an argument beyond the list", None, path=good)
    C.case("place", [good, S1, "0x12"], "HARNESS: <termsHash> a hex32", None, path=good)
    C.case("place", [good, S1, up(thA)], "HARNESS: <termsHash> a hex32, lowercase", None, path=good)
    C.case("place", [good, S1[:-2], thA], "HARNESS: <sellerKey> a hex20, twenty bytes", None, path=good)
    C.case("place", [good, "0x12", thA], "HARNESS: <sellerKey> a hex20", None, path=good)
    C.case("place", [good, S1], "HARNESS: an argument short", None, path=good)
    C.case("pay-check", [pl_], "HARNESS: an argument short", None, path=pl_)
    C.case("pay-check", [pl_, "records/absent.json"], "HARNESS: an unreadable records path", None, path=pl_)
    C.case("pay-check", [pinv, p_unread], "HARNESS before s6.4: the records file is misuse whatever the listing", None, path=pinv)
    C.case("pay-check", ["listings/absent.json", good], "HARNESS: an unreadable listing path", None, path=good)


def gen_misuse():
    C.argv_case([], "HARNESS: no command", None)
    C.argv_case(["bogus"], "HARNESS: an unknown command", None)
    C.argv_case(["check-listing", "listings/l001_ok_full.json"], "HARNESS: an unknown command with a path", None, path="listings/l001_ok_full.json")
    # every command once with an argument beyond its list, once short, once with an unreadable path
    L_, K_, R_, P_, T_, X_ = "listings/l001_ok_full.json", "links/l001_ok.json", "receipts/r001_ok_secp.json", "packs/p001_ok.json", "texts/t001_link_ok.txt", "settles/s001_ok.json"
    for cmd_, args_ in (("link-check", [K_]), ("resolve-link", [K_, L_]), ("receipt-check", [R_]), ("pack-check", [P_]), ("settle-check", [X_]),
                        ("bind", [L_, L_]), ("encode", [K_]), ("receipt-encode", [R_]), ("receipt-decode", [T_])):
        C.argv_case([cmd_] + args_ + ["x"], "HARNESS: " + cmd_ + " with an argument beyond the list", None, path=args_[0])
    for cmd_ in ("decode", "receipt-decode", "resolve-link", "bind-settle"):
        C.argv_case([cmd_], "HARNESS: " + cmd_ + " with no argument", None)
    C.argv_case(["encode", "links/absent.json"], "HARNESS: encode on an unreadable path", None)
    C.argv_case(["receipt-encode", "receipts/absent.json"], "HARNESS: receipt-encode on an unreadable path", None)
    C.argv_case(["bind-receipt", L_, "receipts/absent.json"], "HARNESS: bind-receipt on an unreadable receipt path", None, path=L_)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=1)
    a = ap.parse_args()
    G.seed(a.seed)
    global C
    C = G.Corpus(HERE, ["listings", "links", "docs", "receipts", "packs", "texts", "terms", "settles", "records"])
    blk, tb = main_terms()
    s0, path_ok = gen_listing(blk, tb)
    gen_link(blk, tb, s0, path_ok)
    r, ov, rk, ovk, rg, ovg, rc = gen_receipt(blk, tb, s0, path_ok)
    pk, pp = gen_pack(blk, tb, s0, path_ok, r, ov, rk, ovk, rg, ovg, rc)
    gen_settle(blk, tb, s0, path_ok, r, ov, pk, pp, rc)
    gen_identity(blk, s0)
    gen_misuse()
    n = C.finish(a.seed)
    print("link corpus: %d cases" % n)


if __name__ == "__main__":
    main()
