#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
gen.py - corpus generator for the `eravon.terms/1` convergence run.

Written from docs/eravon-terms-v1.md and HARNESS-TERMS.md.  Every prediction
is written by hand from the text (or `null` where the case is harness misuse
and the expected outcome is exit 2 with no output); the digests an accepted
answer carries are computed by ../gencommon.py's own typed-data and CREATE2
code, a third opinion beside the criterion and the candidates, never
authoritative.

Usage:  python3 gen.py --seed 1
Populates ./docs, ./sigs and writes manifest.json.
"""
import argparse
import hashlib
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, ".."))
import gencommon as G  # noqa: E402
from gencommon import J, cbytes, hx, addr, key, H32, H20, fault, inner, ZERO20, ZERO32  # noqa: E402

C = None  # the Corpus
NO_BLOCK = None
DERIV = None
OB = G.OPEN_BY


def sha(b):
    return hx(hashlib.sha256(b).digest())


def doc(block, **kw):
    return G.terms_doc(cbytes(block), **kw)


def D(slug, b, note, predicted, cmd="check", extra=None):
    path = C.write("docs", slug, b, ext=".md")
    C.case(cmd, [path] + (extra or []), note, predicted, path=path)
    return path


def ok(b):
    return {"ok": True, "termsHash": sha(b)}


def gj_of(tok, **over):
    j = {"judge": addr("judge"), "amanahHash": H32(), "quoteHash": G.qhash(G.quote_bytes(tok))}
    j.update(over)
    return j


def gg_of(tok, **over):
    g = {"pointer": "galeed:guild:1:" + addr("guild"), "amanahHash": H32(), "quoteHash": G.qhash(G.quote_bytes(tok))}
    g.update(over)
    return g


# ==========================================================================
# check
# ==========================================================================

def gen_check():
    tok = addr("token")
    b = doc(G.base_block())
    D("ok_full", b, "both segments as text, three rungs with a note, one judge, buyerMayOpen true", ok(b))
    blk = G.base_block(amount="1", delivery=0, confirm=0, open_window=0, buyer_may_open=False,
                       ladder=[{"buyerBps": 0, "sellerBps": 10000, "feeBearer": 0}, {"buyerBps": 10000, "sellerBps": 0, "feeBearer": 1}])
    b = doc(blk)
    D("ok_minimal", b, "amount 1, the three windows 0, buyerMayOpen false, two rungs, one judge", ok(b))
    blk = G.base_block(judges=[], guilds=[gg_of(tok)])
    b = doc(blk)
    D("ok_guild_only", b, "s2.4: judges may be empty where guilds is not", ok(b))
    blk = G.base_block(guilds=[gg_of(tok)], compliance_md={"seller": {"name": "工作室有限公司", "vat": "GB123"}, "law": "England\nand Wales ✓"})
    blk["judges"][0]["source"] = "galeed:guild:1:" + addr("guild")
    b = doc(blk)
    D("ok_source_and_compliance", b, "a judge with a source pointer, a guild, a compliance_md object of any members whose strings are prose", ok(b))
    blk_s = G.base_block(guilds=[gg_of(tok)]); blk_s["judges"][0]["source"] = "galeed:guild:1:" + ZERO20
    b = doc(blk_s)
    D("ok_source_zero_address", b, "s2.5: source's address is untested; the zero address passes where the guild row's pointer would not", ok(b))
    blk_s = G.base_block(guilds=[gg_of(tok)]); blk_s["judges"][0]["source"] = "galeed:guild:1:" + addr("seller")
    b = doc(blk_s)
    D("ok_source_seller_address", b, "s2.5: source's address is untested; the seller's key passes where the guild row's pointer would not", ok(b))
    blk_s = G.base_block(guilds=[gg_of(tok)]); blk_s["judges"][0]["source"] = "galeed:guild:1:" + addr("guild").upper().replace("0X", "0x")
    D("judge_source_upper", doc(blk_s), "s2.5: source is a pointer of roster/1 s6.1, its address lowercase", fault("E_JUDGE_SOURCE", 0))
    blk = G.base_block(amount=str((2 ** 256 - 1) // 10000), delivery=2 ** 32 - 1, confirm=2 ** 32 - 1, open_window=2 ** 32 - 1)
    b = doc(blk)
    D("ok_max_amount_and_windows", b, "s2.3: amount at its bound (2^256 - 1) // 10000, the three windows at 2^32 - 1", ok(b))
    D("amount_over_bound", doc(G.base_block(amount=str((2 ** 256 - 1) // 10000 + 1))), "s2.3: amount above (2^256 - 1) // 10000", fault("E_TERMS_AMOUNT"))
    ladder = [{"buyerBps": i * 10000 // 15, "sellerBps": 10000 - i * 10000 // 15, "feeBearer": i % 3} for i in range(16)]
    ladder[0]["buyerBps"], ladder[0]["sellerBps"] = 0, 10000
    ladder[15]["buyerBps"], ladder[15]["sellerBps"] = 10000, 0
    b = doc(G.base_block(ladder=ladder))
    D("ok_16_rungs", b, "s2.5: sixteen rungs, non-decreasing, first 0, last 10000", ok(b))
    ladder = [{"buyerBps": 0, "sellerBps": 10000, "feeBearer": 0}, {"buyerBps": 0, "sellerBps": 10000, "feeBearer": 2}, {"buyerBps": 10000, "sellerBps": 0, "feeBearer": 1}]
    b = doc(G.base_block(ladder=ladder))
    D("ok_equal_rungs", b, "s2.5: a rung equal to its predecessor is not below it", ok(b))
    b = doc(G.base_block(goods=..., goodsHash=G.text_hash(G.GOODS)))
    D("ok_goods_hash", b, "s2.3: the goods segment carried as a hash", ok(b))
    b = doc(G.base_block(delivery_text=..., deliveryHash=G.text_hash(G.DELIVERY)))
    D("ok_delivery_hash", b, "s2.3: the delivery segment carried as a hash", ok(b))
    b = doc(G.base_block(goods=..., goodsHash=G.text_hash(G.GOODS), delivery_text=..., deliveryHash=G.text_hash(G.DELIVERY)))
    D("ok_both_hashes", b, "s2.3: both segments as hashes", ok(b))
    b = doc(G.base_block(goods=..., goodsHash=ZERO32))
    D("ok_goods_hash_zero", b, "s2.3: goodsHash is a hex32, zero included; it is data for the parties", ok(b))
    b = doc(G.base_block(goods="", delivery_text=""))
    D("ok_empty_segments", b, "s2.3: a prose string may be empty", ok(b))
    b = doc(G.base_block(), after=b"")
    D("ok_no_trailing_newline", b, "s2.2: the closing fence may be the last line", ok(b))
    b = doc(G.base_block(), before=b"\n\n", after=b"\n\n\n")
    D("ok_blank_lines_outside", b, "s2.2: empty lines before the opening fence and after the closing fence", ok(b))
    b = doc(G.base_block(nonce=0, expiry=0))
    D("ok_zero_nonce_expiry", b, "s2.3: nonce and expiry are ints, zero included", ok(b))
    b = doc(G.base_block(chain_id=11155111))
    D("ok_other_chain", b, "s2.3: chainId is any int; no rule here binds it to mainnet", ok(b))
    b = doc(G.base_block(goods="# 商品\n\n插画一幅 ✓ 2000×3000", compliance_md={"a_md": {"deep": ["é", {"x": "ß"}]}}))
    D("ok_prose_unicode", b, "s1: prose strings carry any scalar; a nested prose subtree's strings too", ok(b))

    # ---- s2.2 and s2.4 before canonical
    D("utf8", doc(G.base_block(), before=b"\xff\n"), "s2.4: E_TERMS_UTF8 before any fence test", fault("E_TERMS_UTF8"))
    D("utf8_in_block", G.terms_doc(cbytes(G.base_block()).replace(b'"v"', b'"\xffv"')), "s2.4: an invalid byte inside the block is E_TERMS_UTF8, the whole document being tested first", fault("E_TERMS_UTF8"))
    global NO_BLOCK
    NO_BLOCK = D("no_block", b"# Terms\n\nno fence at all\n", "s2.4: E_TERMS_BLOCK", fault("E_TERMS_BLOCK"))
    D("crlf", G.OPEN + b"\r\n" + cbytes(G.base_block()) + b"\r\n```\r\n", "s2.2: CRLF endings leave no opening fence", fault("E_TERMS_BLOCK"))
    D("fence_with_space", b"```eravon.terms/1 \n" + cbytes(G.base_block()) + b"\n```\n", "s2.2: the opening fence is the exact 17 bytes", fault("E_TERMS_BLOCK"))
    D("fence_indented", b" ```eravon.terms/1\n" + cbytes(G.base_block()) + b"\n```\n", "s2.2: an indented fence is no fence", fault("E_TERMS_BLOCK"))
    D("two_openings", doc(G.base_block(), after=b"\n" + G.OPEN + b"\n{}\n```\n"), "s2.4: E_TERMS_FENCES", fault("E_TERMS_FENCES"))
    D("two_openings_one_block", doc(G.base_block(), before=G.OPEN + b"\n"), "s2.4 order: the fence count before the block's canonical form", fault("E_TERMS_FENCES"))
    _b = doc(G.base_block(), before=G.OPEN + b"\n"); _i = _b.rfind(b"```"); _b = _b[:_i] + _b[_i + 3:]
    D("two_openings_unclosed", _b, "s2.4 order: the fence count before the closing fence", fault("E_TERMS_FENCES"))
    D("utf8_no_block", b"# Terms\n\nno fence at all\n\xff\n", "s2.4 order: UTF-8 before any fence test, on bytes with no fence", fault("E_TERMS_UTF8"))
    D("utf8_two_openings", doc(G.base_block(), before=b"\xff\n", after=b"\n" + G.OPEN + b"\n{}\n```\n"), "s2.4 order: UTF-8 before the fence count", fault("E_TERMS_UTF8"))
    D("utf8_unclosed", b"\xff\n" + G.OPEN + b"\n" + cbytes(G.base_block()) + b"\n", "s2.4 order: UTF-8 before the closing fence", fault("E_TERMS_UTF8"))
    D("unclosed", G.OPEN + b"\n" + cbytes(G.base_block()) + b"\n", "s2.4: E_TERMS_UNCLOSED", fault("E_TERMS_UNCLOSED"))
    D("close_with_space", G.OPEN + b"\n" + cbytes(G.base_block()) + b"\n``` \n", "s2.2: the closing fence is the exact 3 bytes", fault("E_TERMS_UNCLOSED"))
    D("close_before_open", b"```\n" + G.OPEN + b"\n" + cbytes(G.base_block()) + b"\n", "s2.2: a closing fence before the opening does not close it", fault("E_TERMS_UNCLOSED"))
    D("unclosed_before_outside", b"# T\n" + G.OPEN + b"\n" + cbytes(G.base_block()) + b"\n", "s2.4 order: the closing fence before the outside lines", fault("E_TERMS_UNCLOSED"))
    D("outside_before", doc(G.base_block(), before=b"# Terms\n\n"), "s2.4: a non-empty line before the opening fence, E_TERMS_OUTSIDE", fault("E_TERMS_OUTSIDE"))
    D("outside_after", doc(G.base_block(), after=b"\n\nLicence: non-exclusive.\n"), "s2.4: a non-empty line after the closing fence", fault("E_TERMS_OUTSIDE"))
    D("outside_space_line", doc(G.base_block(), before=b" \n"), "s2.2: a line of one space is not empty", fault("E_TERMS_OUTSIDE"))
    D("outside_cr_line", doc(G.base_block(), before=b"\r\n"), "s2.2: a carriage return is a byte of its line, so the line is not empty", fault("E_TERMS_OUTSIDE"))
    D("outside_second_closing", doc(G.base_block(), after=b"\n```\n"), "s2.2: a second closing fence after the first is a non-empty line outside", fault("E_TERMS_OUTSIDE"))
    D("outside_near_fence", doc(G.base_block(), after=b"\n" + G.OPEN[:-1] + b"\n"), "s2.2: a line that is a proper prefix of the opening fence is no fence and is outside", fault("E_TERMS_OUTSIDE"))
    D("outside_before_canonical", G.terms_doc(b"[ 1 ]", before=b"x\n"), "s2.4 order: the outside lines before the block's canonical form", fault("E_TERMS_OUTSIDE"))
    D("outside_before_root", G.terms_doc(b"[]", after=b"\nx\n"), "s2.4 order: the outside lines before the root test", fault("E_TERMS_OUTSIDE"))

    # ---- canonical(block)
    blk = G.base_block()
    D("canon_empty_block", G.OPEN + b"\n```\n", "s2.2: a block of no lines is the empty byte string, E_JSON", fault("E_JSON"))
    D("canon_two_lines", G.OPEN + b"\n" + cbytes(blk) + b"\n\n```\n", "s2.2: a block of two lines carries a line feed, E_NOT_CANONICAL", fault("E_NOT_CANONICAL"))
    D("canon_pretty", G.terms_doc(json.dumps(blk, indent=1, sort_keys=True, ensure_ascii=False).encode("utf-8")), "galeed/1 s3.5 test 6", fault("E_NOT_CANONICAL"))
    D("canon_root_pretty", G.terms_doc(b"[ 1 ]"), "galeed/1 s3 order: canonical before the root (a block that is neither)", fault("E_NOT_CANONICAL"))
    D("canon_unsorted", G.terms_doc(G.cbytes_unsorted(list(blk.items())[::-1])), "galeed/1 s3.4 item 5: member order", fault("E_NOT_CANONICAL"))
    D("canon_dup_key", G.terms_doc(cbytes(blk).replace(b'"nonce":7', b'"nonce":7,"nonce":8')), "galeed/1 s3.5 test 3", fault("E_DUP_KEY"))
    D("canon_key_charset", G.terms_doc(cbytes(dict(blk, compliance_md={"é": 1}))), "galeed/1 s3.5 test 4: keys stay keys inside a prose subtree", fault("E_KEY_CHARSET"))
    D("closed_md_bare", G.terms_doc(cbytes(dict(blk, _md="é"))), "s1: a key that is exactly _md opens a prose subtree too (galeed/1 s3.3), then is a member beyond the list", fault("E_TERMS_CLOSED"))
    D("canon_value_charset", G.terms_doc(cbytes(dict(blk, goods_md="x", delivery_md="y", expiry=1, nonce=1)).replace(b'"v":"eravon.terms/1"', '"v":"eravon.terms/1é"'.encode("utf-8"))), "galeed/1 s3.3: a skeleton value outside a prose subtree", fault("E_VALUE_CHARSET"))
    D("canon_value_charset_hash", doc(G.base_block(goods=..., goodsHash="0x" + "é" * 32)), "galeed/1 s3.3: goodsHash is a skeleton string; a scalar above U+007E in it is E_VALUE_CHARSET before any form test", fault("E_VALUE_CHARSET"))
    D("canon_number", G.terms_doc(cbytes(blk).replace(b'"nonce":7', b'"nonce":7.0')), "galeed/1 s3.5 test 2: E_NUMBER", fault("E_NUMBER"))
    D("canon_json", G.terms_doc(b'{"v":'), "galeed/1 s3.5 test 2: E_JSON", fault("E_JSON"))
    D("canon_depth", G.terms_doc(cbytes(blk)[:-1] + b',"x":' + b"[" * 128 + b"]" * 128 + b"}"), "galeed/1 s3.5 test 2: E_DEPTH", fault("E_DEPTH"))
    # ---- galeed/1 s3 as this core carries it (s10.1, s10.4), measured by that law's own witnesses: every input of
    # the galeed/1 corpus's check set as the block of a document, predicted from that corpus's own prediction (s2.4:
    # the whole document's UTF-8 first, E_TERMS_UTF8; then canonical(block) with that corpus's token; then the root,
    # E_TERMS, or the seventeen required members, E_TERMS_MISSING)
    for gp, gb, gnote, gcanon, gcheck in G.galeed_canon_inputs():
        if gcanon == "E_UTF8":
            gpred = fault("E_TERMS_UTF8")
        elif gcanon != "ok":
            gpred = fault(gcanon)
        elif gcheck == "E_NOT_CANONICAL":
            gpred = fault("E_NOT_CANONICAL")
        else:
            gpred = fault("E_TERMS_MISSING" if isinstance(json.loads(gb.decode("utf-8")), dict) else "E_TERMS")
        D("gl1_" + gp, G.terms_doc(gb), "galeed/1 corpus %s as the block (%s); gl1 check: %s" % (gp, gnote, gcheck), gpred)
    # ---- what the galeed/1 corpus reaches on no surface: an object's trailing comma (RFC 8259, s3.5 test 2)
    D("canon_trailing_comma_object", G.terms_doc(cbytes(blk)[:-1] + b",}"), "galeed/1 s3.5 test 2: a comma before an object's closing brace, E_JSON", fault("E_JSON"))
    # ---- s2.3 series: present and absent, each refusal, the boundary stages == stage, the adjacent pair
    sid = H32()
    bs = doc(G.base_block(series={"id": sid, "stage": 1, "stages": 3}))
    D("ok_series", bs, "s2.3: a series member of id, stage 1 and stages 3 passes", ok(bs))
    bs2 = doc(G.base_block(series={"id": sid, "stage": 3, "stages": 3}))
    D("ok_series_last_stage", bs2, "s2.3: stages equal to stage passes (the last stage)", ok(bs2))
    D("series_array", doc(G.base_block(series=[sid, 1, 3])), "s2.4: series an object", fault("E_TERMS_SERIES"))
    D("series_id_zero", doc(G.base_block(series={"id": ZERO32, "stage": 1, "stages": 3})), "s2.4: id nonzero", fault("E_TERMS_SERIES"))
    D("series_id_short", doc(G.base_block(series={"id": "0x12", "stage": 1, "stages": 3})), "s2.4: id a hex32", fault("E_TERMS_SERIES"))
    D("series_stage_zero", doc(G.base_block(series={"id": sid, "stage": 0, "stages": 3})), "s2.4: stage at least 1", fault("E_TERMS_SERIES"))
    D("series_stage_string", doc(G.base_block(series={"id": sid, "stage": "1", "stages": 3})), "s2.4: stage an int", fault("E_TERMS_SERIES"))
    D("series_stages_below_stage", doc(G.base_block(series={"id": sid, "stage": 3, "stages": 2})), "s2.4: stages at least stage (one below)", fault("E_TERMS_SERIES"))
    D("series_missing_member", doc(G.base_block(series={"id": sid, "stage": 1})), "s2.4: exactly the three members (one missing)", fault("E_TERMS_SERIES"))
    D("series_extra_member", doc(G.base_block(series={"id": sid, "stage": 1, "stages": 3, "note_md": "x"})), "s2.4: exactly the three members (one beyond)", fault("E_TERMS_SERIES"))
    D("order_compliance_before_series", doc(G.base_block(compliance_md=[], series=[])), "s2.4 order: compliance_md before series", fault("E_TERMS_COMPLIANCE"))
    D("series_missing_id", doc(G.base_block(series={"stage": 1, "stages": 3})), "s2.4: exactly the three members (id missing)", fault("E_TERMS_SERIES"))
    D("series_missing_stage", doc(G.base_block(series={"id": sid, "stages": 3})), "s2.4: exactly the three members (stage missing)", fault("E_TERMS_SERIES"))
    D("series_empty", doc(G.base_block(series={})), "s2.4: exactly the three members (none)", fault("E_TERMS_SERIES"))
    D("series_null", doc(G.base_block(series=None)), "s2.4, s1: a key present with null is a member present; series an object", fault("E_TERMS_SERIES"))
    D("series_stage_bool", doc(G.base_block(series={"id": sid, "stage": True, "stages": 3})), "s2.4: stage an int (a bool is none)", fault("E_TERMS_SERIES"))
    D("series_stages_string", doc(G.base_block(series={"id": sid, "stage": 1, "stages": "3"})), "s2.4: stages an int", fault("E_TERMS_SERIES"))
    D("series_stages_bool", doc(G.base_block(series={"id": sid, "stage": 1, "stages": True})), "s2.4: stages an int (a bool is none)", fault("E_TERMS_SERIES"))
    D("series_id_hex20", doc(G.base_block(series={"id": addr("x"), "stage": 1, "stages": 3})), "s2.4: id a hex32 (a hex20 is none)", fault("E_TERMS_SERIES"))
    D("series_id_upper", doc(G.base_block(series={"id": "0x" + sid[2:].upper(), "stage": 1, "stages": 3})), "s2.4, s1: id a hex32, lowercase", fault("E_TERMS_SERIES"))

    # ---- galeed/1 s1's hex forms on each refusal that section names, on a hex20 and a hex32 a predicate reads
    sp = addr("seller-payout")
    D("payout_prefix_upper", doc(G.base_block(sellerPayout="0X" + sp[2:])), "galeed/1 s1: an uppercase prefix fails the form, E_TERMS_PAYOUT", fault("E_TERMS_PAYOUT"))
    D("payout_prefix_other", doc(G.base_block(sellerPayout="1x" + sp[2:])), "galeed/1 s1: another prefix fails the form", fault("E_TERMS_PAYOUT"))
    D("payout_prefix_absent", doc(G.base_block(sellerPayout=sp[2:])), "galeed/1 s1: forty digits with no prefix fail the form", fault("E_TERMS_PAYOUT"))
    gh = G.text_hash(G.GOODS)
    D("goodshash_prefix_upper", doc(G.base_block(goods=..., goodsHash="0X" + gh[2:])), "galeed/1 s1: an uppercase prefix on a hex32, E_TERMS_GOODS", fault("E_TERMS_GOODS"))
    D("goodshash_prefix_absent", doc(G.base_block(goods=..., goodsHash=gh[2:])), "galeed/1 s1: a hex32 with no prefix", fault("E_TERMS_GOODS"))

    # ---- s2.4 in order
    D("root_array", G.terms_doc(b"[]"), "s2.4: root not an object", fault("E_TERMS"))
    D("root_string", G.terms_doc(b'"eravon.terms/1"'), "s2.4: root a string", fault("E_TERMS"))
    d = G.base_block(); del d["amount"]
    D("missing_amount", doc(d), "s2.4: a required member absent", fault("E_TERMS_MISSING"))
    d = G.base_block(); del d["buyerMayOpen"]
    D("missing_buyeropen", doc(d), "s2.4: buyerMayOpen is required", fault("E_TERMS_MISSING"))
    d = G.base_block(); del d["nonce"]; d["extra"] = 1
    D("missing_and_extra", doc(d), "s2.4 order: missing before closed", fault("E_TERMS_MISSING"))
    d = G.base_block(); d["extra"] = None
    D("closed", doc(d), "s2.4: a member beyond the table's keys", fault("E_TERMS_CLOSED"))
    d = G.base_block(); d["milestones"] = []
    D("closed_milestones", doc(d), "s2.3: no milestones member; it is beyond the list", fault("E_TERMS_CLOSED"))
    d = G.base_block(); d["grantHash"] = H32()
    D("closed_grant", doc(d), "s2.3: no grantHash member", fault("E_TERMS_CLOSED"))
    d = G.base_block(); d["compliance"] = {}
    D("closed_compliance_without_suffix", doc(d), "s2.3: the member is compliance_md; compliance is beyond the list", fault("E_TERMS_CLOSED"))
    d = G.base_block(v="x"); d["extra"] = 1
    D("closed_before_v", doc(d), "s2.4 order: closed before v", fault("E_TERMS_CLOSED"))
    D("v_2", doc(G.base_block(v="eravon.terms/2")), "s2.4: v", fault("E_TERMS_V"))
    D("v_int", doc(G.base_block(v=1)), "s2.4: v not a string", fault("E_TERMS_V"))
    D("chain_string", doc(G.base_block(chain_id="1")), "s2.4: chainId not an int", fault("E_TERMS_CHAIN"))
    D("chain_bool", doc(G.base_block(chain_id=True)), "s2.4: chainId a bool", fault("E_TERMS_CHAIN"))
    D("logic_zero", doc(G.base_block(logic=ZERO20)), "s2.4: logic zero", fault("E_TERMS_LOGIC"))
    D("logic_upper", doc(G.base_block(logic=addr("logic").upper().replace("0X", "0x"))), "s2.4: logic outside hex20", fault("E_TERMS_LOGIC"))
    D("seller_zero", doc(G.base_block(sellerKey=ZERO20)), "s2.4: sellerKey zero", fault("E_TERMS_SELLER"))
    D("seller_short", doc(G.base_block(sellerKey="0x12")), "s2.4: sellerKey short", fault("E_TERMS_SELLER"))
    D("payout_zero", doc(G.base_block(sellerPayout=ZERO20)), "s2.4: sellerPayout zero", fault("E_TERMS_PAYOUT"))
    D("payout_upper", doc(G.base_block(sellerPayout=G.base_block()["sellerPayout"].upper().replace("0X", "0x"))), "s2.4: sellerPayout outside hex20", fault("E_TERMS_PAYOUT"))
    D("fund_zero", doc(G.base_block(sellerFund=ZERO20)), "s2.4: sellerFund zero", fault("E_TERMS_FUND"))
    D("fund_upper", doc(G.base_block(sellerFund=G.base_block()["sellerFund"].upper().replace("0X", "0x"))), "s2.4: sellerFund outside hex20", fault("E_TERMS_FUND"))
    D("fund_short", doc(G.base_block(sellerFund="0x12")), "s2.4: sellerFund hex20", fault("E_TERMS_FUND"))
    d = G.base_block(); del d["sellerFund"]
    D("missing_fund", doc(d), "s2.4: sellerFund is required", fault("E_TERMS_MISSING"))
    bf = G.base_block(sellerFund=addr("company-wallet"))
    D("ok_fund_other", doc(bf), "s2.3: sellerFund need not be sellerKey (a company's wallet)", ok(doc(bf)))
    D("token_zero", doc(G.base_block(token=ZERO20, judges=[gj_of(tok)])), "s2.4: token zero", fault("E_TERMS_TOKEN"))
    D("token_int", doc(G.base_block(token=5, judges=[gj_of(tok)])), "s2.4: token not a string", fault("E_TERMS_TOKEN"))
    D("amount_zero", doc(G.base_block(amount="0")), "s2.4: amount nonzero", fault("E_TERMS_AMOUNT"))
    D("amount_leading_zero", doc(G.base_block(amount="05")), "s1: dec256 has no leading zero", fault("E_TERMS_AMOUNT"))
    D("amount_number", doc(G.base_block(amount=5)), "s1: dec256 is a string", fault("E_TERMS_AMOUNT"))
    D("amount_too_big", doc(G.base_block(amount=str(2 ** 256))), "s1: dec256 at most 2^256 - 1", fault("E_TERMS_AMOUNT"))
    D("amount_empty", doc(G.base_block(amount="")), "s1: dec256 non-empty", fault("E_TERMS_AMOUNT"))
    D("amount_hex", doc(G.base_block(amount="0x5")), "s1: dec256 is decimal digits", fault("E_TERMS_AMOUNT"))
    D("delivery_u32", doc(G.base_block(delivery=2 ** 32)), "s2.4: deliveryWindow u32", fault("E_TERMS_DELIVERY_WINDOW"))
    D("delivery_string", doc(G.base_block(delivery="1")), "s2.4: deliveryWindow an int", fault("E_TERMS_DELIVERY_WINDOW"))
    D("confirm_u32", doc(G.base_block(confirm=2 ** 32)), "s2.4: confirmWindow u32", fault("E_TERMS_CONFIRM_WINDOW"))
    D("confirm_bool", doc(G.base_block(confirm=True)), "s2.4: confirmWindow an int (a bool is none)", fault("E_TERMS_CONFIRM_WINDOW"))
    D("openwindow_u32", doc(G.base_block(open_window=2 ** 32)), "s2.4: openWindow u32", fault("E_TERMS_OPENWINDOW"))
    D("openwindow_string", doc(G.base_block(open_window="86400")), "s2.4: openWindow an int", fault("E_TERMS_OPENWINDOW"))
    D("buyeropen_int", doc(G.base_block(buyer_may_open=1)), "s2.4: buyerMayOpen a bool (1 is none)", fault("E_TERMS_BUYEROPEN"))
    D("buyeropen_string", doc(G.base_block(buyer_may_open="true")), "s2.4: buyerMayOpen a bool", fault("E_TERMS_BUYEROPEN"))
    D("buyeropen_null", doc(G.base_block(buyer_may_open=None)), "s2.4: buyerMayOpen written null is present and no bool", fault("E_TERMS_BUYEROPEN"))
    D("goods_both", doc(G.base_block(goodsHash=G.text_hash(G.GOODS))), "s2.3: goods_md and goodsHash both present", fault("E_TERMS_GOODS"))
    D("goods_neither", doc(G.base_block(goods=...)), "s2.3: neither goods_md nor goodsHash", fault("E_TERMS_GOODS"))
    D("goods_md_int", doc(G.base_block(goods=1)), "s2.3: goods_md a string", fault("E_TERMS_GOODS"))
    D("goods_md_null", doc(G.base_block(goods_md=None)), "galeed/1 s6: a present null is a member present, and no string", fault("E_TERMS_GOODS"))
    D("goodshash_short", doc(G.base_block(goods=..., goodsHash="0x12")), "s2.3: goodsHash hex32", fault("E_TERMS_GOODS"))
    D("goodshash_upper", doc(G.base_block(goods=..., goodsHash="0x" + "AB" * 32)), "s1: hex32 is lowercase", fault("E_TERMS_GOODS"))
    D("goodshash_int", doc(G.base_block(goods=..., goodsHash=5)), "s2.3: goodsHash a string", fault("E_TERMS_GOODS"))
    D("delivery_both", doc(G.base_block(deliveryHash=G.text_hash(G.DELIVERY))), "s2.3: delivery_md and deliveryHash both present", fault("E_TERMS_DELIVERY"))
    D("delivery_neither", doc(G.base_block(delivery_text=...)), "s2.3: neither delivery_md nor deliveryHash", fault("E_TERMS_DELIVERY"))
    D("delivery_md_array", doc(G.base_block(delivery_text=["x"])), "s2.3: delivery_md a string", fault("E_TERMS_DELIVERY"))
    D("deliveryhash_short", doc(G.base_block(delivery_text=..., deliveryHash="0x00")), "s2.3: deliveryHash hex32", fault("E_TERMS_DELIVERY"))
    D("goods_before_delivery", doc(G.base_block(goods=..., delivery_text=...)), "s2.4 order: the goods pair before the delivery pair", fault("E_TERMS_GOODS"))
    r0 = {"buyerBps": 0, "sellerBps": 10000, "feeBearer": 0}
    r1 = {"buyerBps": 10000, "sellerBps": 0, "feeBearer": 1}
    D("ladder_one", doc(G.base_block(ladder=[r1])), "s2.4: two to sixteen rungs", fault("E_TERMS_LADDER"))
    D("ladder_17", doc(G.base_block(ladder=[r0] + [{"buyerBps": 5000, "sellerBps": 5000, "feeBearer": 2}] * 15 + [r1])), "s2.4: seventeen rungs", fault("E_TERMS_LADDER"))
    D("ladder_object", doc(G.base_block(ladder={})), "s2.4: ladder not an array", fault("E_TERMS_LADDER"))
    D("ladder_one_bad_rung", doc(G.base_block(ladder=["x"])), "s2.4 order: the ladder's count before any rung's table", fault("E_TERMS_LADDER"))
    D("ladder_17_bad_rung", doc(G.base_block(ladder=[r0] * 16 + ["x"])), "s2.4 order: seventeen rungs before the seventeenth's form", fault("E_TERMS_LADDER"))
    D("rung_string", doc(G.base_block(ladder=[r0, "x"])), "s2.5: a rung not an object", fault("E_RUNG", 1))
    D("rung_missing", doc(G.base_block(ladder=[{"buyerBps": 0, "sellerBps": 10000}, r1])), "s2.5: feeBearer absent", fault("E_RUNG_MISSING", 0))
    D("rung_closed", doc(G.base_block(ladder=[dict(r0, x=1), r1])), "s2.5: a member beyond the four", fault("E_RUNG_CLOSED", 0))
    D("rung_buyer_over", doc(G.base_block(ladder=[r0, {"buyerBps": 10001, "sellerBps": 0, "feeBearer": 1}])), "s2.5: buyerBps at most 10000", fault("E_RUNG_BUYER", 1))
    D("rung_buyer_string", doc(G.base_block(ladder=[dict(r0, buyerBps="0"), r1])), "s2.5: buyerBps an int", fault("E_RUNG_BUYER", 0))
    D("rung_seller_string", doc(G.base_block(ladder=[dict(r0, sellerBps="10000"), r1])), "s2.5: sellerBps an int", fault("E_RUNG_SELLER", 0))
    D("rung_bearer_3", doc(G.base_block(ladder=[r0, dict(r1, feeBearer=3)])), "s2.5: feeBearer at most 2", fault("E_RUNG_BEARER", 1))
    D("rung_bearer_string", doc(G.base_block(ladder=[r0, dict(r1, feeBearer="2")])), "s2.5: feeBearer an int", fault("E_RUNG_BEARER", 1))
    D("rung_note_int", doc(G.base_block(ladder=[dict(r0, note_md=1), r1])), "s2.5: note_md a string", fault("E_RUNG_NOTE", 0))
    D("rung_sum", doc(G.base_block(ladder=[r0, {"buyerBps": 10000, "sellerBps": 1, "feeBearer": 1}])), "s2.5: buyerBps + sellerBps == 10000", fault("E_RUNG_SUM", 1))
    D("rung_order_buyer_before_sum", doc(G.base_block(ladder=[r0, {"buyerBps": 10001, "sellerBps": 1, "feeBearer": 1}])), "s2.5 order: buyer form before the sum", fault("E_RUNG_BUYER", 1))
    D("ladder_first", doc(G.base_block(ladder=[{"buyerBps": 1, "sellerBps": 9999, "feeBearer": 0}, r1])), "s2.5: the first rung's buyerBps is 0", fault("E_LADDER_FIRST"))
    D("ladder_last", doc(G.base_block(ladder=[r0, {"buyerBps": 9999, "sellerBps": 1, "feeBearer": 1}])), "s2.5: the last rung's buyerBps is 10000", fault("E_LADDER_LAST"))
    D("ladder_order", doc(G.base_block(ladder=[r0, {"buyerBps": 6000, "sellerBps": 4000, "feeBearer": 2}, {"buyerBps": 5000, "sellerBps": 5000, "feeBearer": 2}, r1])), "s2.5: no rung below its predecessor, index of the first that is", fault("E_LADDER_ORDER", 2))
    D("ladder_first_before_last", doc(G.base_block(ladder=[{"buyerBps": 1, "sellerBps": 9999, "feeBearer": 0}, {"buyerBps": 9999, "sellerBps": 1, "feeBearer": 1}])), "s2.5 order: first before last", fault("E_LADDER_FIRST"))
    D("ladder_rung_before_first", doc(G.base_block(ladder=[{"buyerBps": 1, "sellerBps": 9999, "feeBearer": 0}, dict(r1, feeBearer=9)])), "s2.4 order: each rung's table before the ladder-wide rules", fault("E_RUNG_BEARER", 1))
    gj = gj_of(tok)
    D("judges_object", doc(G.base_block(judges={})), "s2.4: judges not an array", fault("E_TERMS_JUDGES"))
    D("judge_string", doc(G.base_block(judges=[gj, "x"])), "s2.5: a judge not an object", fault("E_JUDGE", 1))
    D("judge_missing", doc(G.base_block(judges=[{"judge": addr("judge"), "quoteHash": gj["quoteHash"]}])), "s2.5: amanahHash absent", fault("E_JUDGE_MISSING", 0))
    D("judge_missing_quotehash", doc(G.base_block(judges=[{"judge": addr("judge"), "amanahHash": H32()}])), "s2.5: quoteHash is required", fault("E_JUDGE_MISSING", 0))
    D("judge_closed", doc(G.base_block(judges=[dict(gj, sig="0x00")])), "s2.5: a member beyond the four (sig is the link law's)", fault("E_JUDGE_CLOSED", 0))
    D("judge_closed_quote", doc(G.base_block(judges=[dict(gj, quote=G.quote_bytes(tok))])), "s2.5: the quote bytes travel in no document; quote is beyond the list", fault("E_JUDGE_CLOSED", 0))
    D("judge_zero", doc(G.base_block(judges=[dict(gj, judge=ZERO20)])), "s2.5: judge nonzero", fault("E_JUDGE_ADDRESS", 0))
    D("judge_is_seller", doc(G.base_block(judges=[dict(gj, judge=addr("seller"))])), "s2.5: judge not the seller", fault("E_JUDGE_ADDRESS", 0))
    D("judge_amanah_zero", doc(G.base_block(judges=[dict(gj, amanahHash=ZERO32)])), "s2.5: amanahHash nonzero", fault("E_JUDGE_AMANAH", 0))
    D("judge_amanah_short", doc(G.base_block(judges=[dict(gj, amanahHash="0x12")])), "s2.5: amanahHash hex32", fault("E_JUDGE_AMANAH", 0))
    D("judge_quote_zero", doc(G.base_block(judges=[dict(gj, quoteHash=ZERO32)])), "s2.5: quoteHash nonzero", fault("E_JUDGE_QUOTE", 0))
    D("judge_quote_short", doc(G.base_block(judges=[dict(gj, quoteHash="0xabcd")])), "s2.5: quoteHash hex32, no inner", fault("E_JUDGE_QUOTE", 0))
    D("judge_quote_upper", doc(G.base_block(judges=[dict(gj, quoteHash="0x" + "AB" * 32)])), "s1: hex32 is lowercase", fault("E_JUDGE_QUOTE", 0))
    D("judge_quote_int", doc(G.base_block(judges=[dict(gj, quoteHash=7)])), "s2.5: quoteHash a string", fault("E_JUDGE_QUOTE", 0))
    D("judge_source_bad", doc(G.base_block(judges=[dict(gj, source="galeed:guild:01:" + addr("guild"))])), "s2.5: source a pointer (leading zero chain)", fault("E_JUDGE_SOURCE", 0))
    D("judge_source_int", doc(G.base_block(judges=[dict(gj, source=1)])), "s2.5: source a string", fault("E_JUDGE_SOURCE", 0))
    D("judge_source_other_chain", doc(G.base_block(judges=[dict(gj, source="galeed:guild:5:" + addr("guild"))])), "s2.5: the source pointer's chainId equals the document's", fault("E_JUDGE_SOURCE", 0))
    b = doc(G.base_block(chain_id=5, judges=[dict(gj, source="galeed:guild:5:" + addr("guild"))]))
    D("ok_source_chain_5", b, "s2.5: a chain-5 document with a chain-5 pointer", ok(b))
    D("judge_dup", doc(G.base_block(judges=[gj, dict(gj, amanahHash=H32())])), "s2.4: no judge repeated, index of the repeat", fault("E_TERMS_DUP_JUDGE", 1))
    D("judge_dup_after_tables", doc(G.base_block(judges=[gj, gj, dict(gj, judge="0x1")])), "s2.4 order: every element's table before the repeat test", fault("E_JUDGE_ADDRESS", 2))
    gg = gg_of(tok)
    D("guilds_object", doc(G.base_block(guilds={})), "s2.4: guilds not an array", fault("E_TERMS_GUILDS"))
    D("guild_string", doc(G.base_block(guilds=["x"])), "s2.5: a guild not an object", fault("E_GUILD", 0))
    D("guild_missing", doc(G.base_block(guilds=[{"pointer": gg["pointer"], "quoteHash": gg["quoteHash"]}])), "s2.5: amanahHash absent", fault("E_GUILD_MISSING", 0))
    D("guild_closed", doc(G.base_block(guilds=[dict(gg, source="x")])), "s2.5: a member beyond the three", fault("E_GUILD_CLOSED", 0))
    D("guild_closed_quote", doc(G.base_block(guilds=[dict(gg, quote=G.quote_bytes(tok))])), "s2.5: quote is beyond the list", fault("E_GUILD_CLOSED", 0))
    D("guild_pointer_zero", doc(G.base_block(guilds=[dict(gg, pointer="galeed:guild:1:" + ZERO20)])), "s2.5: the pointer's address nonzero", fault("E_GUILD_POINTER", 0))
    D("guild_pointer_no_colon", doc(G.base_block(guilds=[dict(gg, pointer="galeed:guild:1")])), "roster/1 s6.1: no second colon", fault("E_GUILD_POINTER", 0))
    D("guild_pointer_prefix", doc(G.base_block(guilds=[dict(gg, pointer="galeed:judge:1:" + addr("guild"))])), "roster/1 s6.1: the prefix", fault("E_GUILD_POINTER", 0))
    D("guild_pointer_upper", doc(G.base_block(guilds=[dict(gg, pointer="galeed:guild:1:" + addr("guild").upper().replace("0X", "0x"))])), "roster/1 s6.1: uppercase digits", fault("E_GUILD_POINTER", 0))
    D("guild_pointer_chain_big", doc(G.base_block(guilds=[dict(gg, pointer="galeed:guild:%d:%s" % (2 ** 53, addr("guild")))])), "roster/1 s6.1: chain id at or above 2^53", fault("E_GUILD_POINTER", 0))
    D("guild_pointer_space", doc(G.base_block(guilds=[dict(gg, pointer=" " + gg["pointer"])])), "roster/1 s6.1: surrounding whitespace", fault("E_GUILD_POINTER", 0))
    D("guild_pointer_empty_chain", doc(G.base_block(guilds=[dict(gg, pointer="galeed:guild::" + addr("guild"))])), "roster/1 s6.1: an empty chain id", fault("E_GUILD_POINTER", 0))
    D("guild_pointer_other_chain", doc(G.base_block(guilds=[dict(gg, pointer="galeed:guild:5:" + addr("guild"))])), "s2.5: the pointer's chainId equals the document's", fault("E_GUILD_POINTER", 0))
    D("guild_amanah_zero", doc(G.base_block(guilds=[dict(gg, amanahHash=ZERO32)])), "s2.5: amanahHash nonzero", fault("E_GUILD_AMANAH", 0))
    D("guild_amanah_short", doc(G.base_block(guilds=[dict(gg, amanahHash="0x12")])), "s2.5: amanahHash hex32", fault("E_GUILD_AMANAH", 0))
    D("guild_pointer_seller", doc(G.base_block(guilds=[dict(gg, pointer="galeed:guild:1:" + addr("seller"))])), "s2.5: the pointer's address is not the seller's key", fault("E_GUILD_POINTER", 0))
    D("guild_quote_zero", doc(G.base_block(guilds=[dict(gg, quoteHash=ZERO32)])), "s2.5: quoteHash nonzero", fault("E_GUILD_QUOTE", 0))
    D("guild_quote_short", doc(G.base_block(guilds=[dict(gg, quoteHash="0x" + "00" * 31 + "1")])), "s2.5: quoteHash hex32 (sixty-three digits)", fault("E_GUILD_QUOTE", 0))
    D("guild_quote_null", doc(G.base_block(guilds=[dict(gg, quoteHash=None)])), "s2.5: quoteHash a string", fault("E_GUILD_QUOTE", 0))
    D("guild_dup", doc(G.base_block(guilds=[gg, dict(gg, amanahHash=H32())])), "s2.4: no guild address repeated", fault("E_TERMS_DUP_GUILD", 1))
    D("guild_dup_after_tables", doc(G.base_block(guilds=[gg, gg, dict(gg, amanahHash=ZERO32)])), "s2.4 order: every guild's table before the guild repeat test", fault("E_GUILD_AMANAH", 2))
    b = doc(G.base_block(judges=[dict(gj, judge=addr("guild"))], guilds=[gg]))
    D("judge_and_guild_same_address", b, "s2.4: a judge and a guild may share an address; the two lists are tested apart", ok(b))
    b = doc(G.base_block(judges=[gj, dict(gj, judge=addr("judge2"), quoteHash=gj["quoteHash"])]))
    D("two_judges_same_quote_hash", b, "s2.5: two candidates may name the same quote hash", ok(b))
    D("no_candidate", doc(G.base_block(judges=[])), "s2.4: E_TERMS_NO_CANDIDATE", fault("E_TERMS_NO_CANDIDATE"))
    D("no_candidate_before_nonce", doc(G.base_block(judges=[], nonce="7")), "s2.4 order: candidates before nonce", fault("E_TERMS_NO_CANDIDATE"))
    D("nonce_string", doc(G.base_block(nonce="7")), "s2.4: nonce an int", fault("E_TERMS_NONCE"))
    D("expiry_null", doc(G.base_block(expiry=None)), "s2.4: expiry an int", fault("E_TERMS_EXPIRY"))
    D("compliance_string", doc(G.base_block(compliance_md="x")), "s2.4: compliance_md an object", fault("E_TERMS_COMPLIANCE"))
    D("compliance_null", doc(G.base_block(compliance_md=None)), "s2.4: compliance_md written null is present", fault("E_TERMS_COMPLIANCE"))
    # the stage bridges of s2.4, each adjacent pair
    D("order_v_before_chain", doc(G.base_block(v="x", chain_id="1")), "s2.4 order: v before chainId", fault("E_TERMS_V"))
    D("order_chain_before_logic", doc(G.base_block(chain_id="1", logic="x")), "s2.4 order", fault("E_TERMS_CHAIN"))
    D("order_logic_before_seller", doc(G.base_block(logic="x", sellerKey="y")), "s2.4 order", fault("E_TERMS_LOGIC"))
    D("order_seller_before_payout", doc(G.base_block(sellerKey="x", sellerPayout="y")), "s2.4 order", fault("E_TERMS_SELLER"))
    D("order_payout_before_token", doc(G.base_block(sellerPayout="x", token="y")), "s2.4 order", fault("E_TERMS_PAYOUT"))
    D("order_payout_before_fund", doc(G.base_block(sellerPayout="x", sellerFund="y")), "s2.4 order", fault("E_TERMS_PAYOUT"))
    D("order_fund_before_token", doc(G.base_block(sellerFund="x", token="y")), "s2.4 order", fault("E_TERMS_FUND"))
    D("order_token_before_amount", doc(G.base_block(token="x", amount="0")), "s2.4 order", fault("E_TERMS_TOKEN"))
    D("order_amount_before_delivery", doc(G.base_block(amount="0", delivery="x")), "s2.4 order", fault("E_TERMS_AMOUNT"))
    D("order_delivery_before_confirm", doc(G.base_block(delivery="x", confirm="y")), "s2.4 order", fault("E_TERMS_DELIVERY_WINDOW"))
    D("order_confirm_before_openwindow", doc(G.base_block(confirm="y", open_window="z")), "s2.4 order", fault("E_TERMS_CONFIRM_WINDOW"))
    D("order_openwindow_before_buyeropen", doc(G.base_block(open_window="z", buyer_may_open=0)), "s2.4 order", fault("E_TERMS_OPENWINDOW"))
    D("order_buyeropen_before_goods", doc(G.base_block(buyer_may_open=0, goods=...)), "s2.4 order", fault("E_TERMS_BUYEROPEN"))
    D("order_delivery_pair_before_ladder", doc(G.base_block(delivery_text=..., ladder=[])), "s2.4 order: the delivery pair before the ladder", fault("E_TERMS_DELIVERY"))
    D("order_ladder_before_judges", doc(G.base_block(ladder=[], judges={})), "s2.4 order", fault("E_TERMS_LADDER"))
    D("order_judges_before_guilds", doc(G.base_block(judges={}, guilds={})), "s2.4 order", fault("E_TERMS_JUDGES"))
    D("order_guilds_before_candidates", doc(G.base_block(judges=[], guilds={})), "s2.4 order: guilds form before the non-empty test", fault("E_TERMS_GUILDS"))
    D("order_nonce_before_expiry", doc(G.base_block(nonce="7", expiry="x")), "s2.4 order", fault("E_TERMS_NONCE"))
    D("order_expiry_before_compliance", doc(G.base_block(expiry="x", compliance_md="z")), "s2.4 order", fault("E_TERMS_EXPIRY"))
    D("order_ladder_rules_before_judges", doc(G.base_block(ladder=[{"buyerBps": 1, "sellerBps": 9999, "feeBearer": 0}, r1], judges={})), "s2.4 order: the ladder-wide rules before judges", fault("E_LADDER_FIRST"))
    D("order_judge_repeat_before_guilds", doc(G.base_block(judges=[gj, dict(gj, amanahHash=H32())], guilds={})), "s2.4 order: the judge repeat before guilds", fault("E_TERMS_DUP_JUDGE", 1))
    D("order_guild_element_before_repeat", doc(G.base_block(guilds=[gg, dict(gg, amanahHash=ZERO32)])), "s2.4 order: a guild's table before the guild repeat", fault("E_GUILD_AMANAH", 1))
    D("order_guild_repeat_before_nonce", doc(G.base_block(guilds=[gg, dict(gg, amanahHash=H32())], nonce="x")), "s2.4 order: the guild repeat before nonce", fault("E_TERMS_DUP_GUILD", 1))
    D("order_guilds_before_nonce", doc(G.base_block(guilds={}, nonce="x")), "s2.4 order: the guilds form before nonce", fault("E_TERMS_GUILDS"))
    D("rung_order_buyer_before_seller", doc(G.base_block(ladder=[dict(r0, buyerBps="0", sellerBps="10000"), r1])), "s2.5 order within a rung: buyer before seller", fault("E_RUNG_BUYER", 0))
    D("rung_order_seller_before_bearer", doc(G.base_block(ladder=[dict(r0, sellerBps="10000", feeBearer=3), r1])), "s2.5 order within a rung: seller before bearer", fault("E_RUNG_SELLER", 0))
    D("rung_order_bearer_before_note", doc(G.base_block(ladder=[dict(r0, feeBearer=3, note_md=1), r1])), "s2.5 order within a rung: bearer before note", fault("E_RUNG_BEARER", 0))
    D("rung_order_note_before_sum", doc(G.base_block(ladder=[dict(r0, sellerBps=1, note_md=1), r1])), "s2.5 order within a rung: note before the sum", fault("E_RUNG_NOTE", 0))
    D("judge_order_address_before_amanah", doc(G.base_block(judges=[dict(gj, judge=ZERO20, amanahHash=ZERO32)])), "s2.5 order within a judge: address before amanah", fault("E_JUDGE_ADDRESS", 0))
    D("judge_order_amanah_before_quote", doc(G.base_block(judges=[dict(gj, amanahHash=ZERO32, quoteHash="abc")])), "s2.5 order within a judge: amanah before quoteHash", fault("E_JUDGE_AMANAH", 0))
    D("judge_order_quote_before_source", doc(G.base_block(judges=[dict(gj, quoteHash="abc", source=1)])), "s2.5 order within a judge: quoteHash before source", fault("E_JUDGE_QUOTE", 0))
    D("guild_order_pointer_before_amanah", doc(G.base_block(guilds=[dict(gg, pointer="galeed:guild:1", amanahHash=ZERO32)])), "s2.5 order within a guild: pointer before amanah", fault("E_GUILD_POINTER", 0))
    D("guild_order_amanah_before_quote", doc(G.base_block(guilds=[dict(gg, amanahHash=ZERO32, quoteHash="abc")])), "s2.5 order within a guild: amanah before quoteHash", fault("E_GUILD_AMANAH", 0))
    D("rung_order_missing_before_closed", doc(G.base_block(ladder=[{"buyerBps": 0, "sellerBps": 10000, "x": 1}, r1])), "s2.5 order within a rung: missing before closed", fault("E_RUNG_MISSING", 0))
    D("rung_order_closed_before_buyer", doc(G.base_block(ladder=[dict(r0, x=1, buyerBps=10001), r1])), "s2.5 order within a rung: closed before the first row", fault("E_RUNG_CLOSED", 0))
    D("judge_order_missing_before_closed", doc(G.base_block(judges=[{"judge": addr("judge"), "quoteHash": gj["quoteHash"], "x": 1}])), "s2.5 order within a judge: missing before closed", fault("E_JUDGE_MISSING", 0))
    D("judge_order_closed_before_address", doc(G.base_block(judges=[dict(gj, x=1, judge=ZERO20)])), "s2.5 order within a judge: closed before the first row", fault("E_JUDGE_CLOSED", 0))
    D("guild_order_missing_before_closed", doc(G.base_block(guilds=[{"pointer": gg["pointer"], "amanahHash": gg["amanahHash"], "x": 1}])), "s2.5 order within a guild: missing before closed", fault("E_GUILD_MISSING", 0))
    D("guild_order_closed_before_pointer", doc(G.base_block(guilds=[dict(gg, x=1, pointer="galeed:guild:1:" + ZERO20)])), "s2.5 order within a guild: closed before the first row", fault("E_GUILD_CLOSED", 0))
    D("ladder_last_before_order", doc(G.base_block(ladder=[r0, {"buyerBps": 6000, "sellerBps": 4000, "feeBearer": 2}, {"buyerBps": 5000, "sellerBps": 5000, "feeBearer": 2}])), "s2.5 order: the last rung's rule before the order rule", fault("E_LADDER_LAST"))
    D("rung_order_element_before_element", doc(G.base_block(ladder=[dict(r0, note_md=1), {"buyerBps": 10001, "sellerBps": 0, "feeBearer": 1}])), "s2.4 order: rung 0's table before rung 1's", fault("E_RUNG_NOTE", 0))
    D("judge_order_element_before_element", doc(G.base_block(judges=[dict(gj, source="galeed:guild:5:" + addr("guild")), dict(gj, judge=addr("judge2"), amanahHash=ZERO32)])), "s2.4 order: judge 0's table before judge 1's", fault("E_JUDGE_SOURCE", 0))
    D("guild_order_element_before_element", doc(G.base_block(guilds=[dict(gg, quoteHash=ZERO32), dict(gg, pointer="galeed:guild:1:" + ZERO20)])), "s2.4 order: guild 0's table before guild 1's", fault("E_GUILD_QUOTE", 0))
    D("ladder_order_first_of_two", doc(G.base_block(ladder=[r0, {"buyerBps": 6000, "sellerBps": 4000, "feeBearer": 2}, {"buyerBps": 5000, "sellerBps": 5000, "feeBearer": 2}, {"buyerBps": 7000, "sellerBps": 3000, "feeBearer": 2}, {"buyerBps": 6500, "sellerBps": 3500, "feeBearer": 2}, r1])), "s2.5: two descents; the first is named", fault("E_LADDER_ORDER", 2))
    _j1 = dict(gj, judge=addr("judge2"), amanahHash=H32())
    D("judge_dup_first_of_two", doc(G.base_block(judges=[gj, _j1, _j1, gj])), "s2.4: two repeats; the first is named", fault("E_TERMS_DUP_JUDGE", 2))
    _g1 = dict(gg, pointer="galeed:guild:1:0x" + "00" * 18 + "1234")
    D("guild_dup_first_of_two", doc(G.base_block(guilds=[gg, _g1, _g1, gg])), "s2.4: two repeats; the first is named", fault("E_TERMS_DUP_GUILD", 2))
    C.case("check", ["docs/absent.md"], "HARNESS: a path the command cannot read", None)
    C.case("check", ["docs/d001_ok_full.md", "x"], "HARNESS: an argument beyond the list", None)
    C.case("check", [], "HARNESS: no path", None, path="(none)")


# ==========================================================================
# quote
# ==========================================================================

def gen_quote():
    tok = addr("token")

    def Q(slug, qhex, note, predicted):
        C.argv_case(["quote", qhex], slug + ": " + note, predicted, path="quote:" + slug)

    def okq(qhex):
        return {"ok": True, "quote": G.quote_decoded(qhex), "quoteHash": hx(G.keccak256(bytes.fromhex(qhex[2:])))}

    q = G.quote_bytes(tok)
    Q("ok_direct", q, "s3.2: a quote in the settlement token, lapseDefault 2", okq(q))
    q = G.quote_bytes(tok, feeUnit=addr("usd"), feedToken=addr("feedT"), feedUnit=addr("feedU"), amountCap=10 ** 9, feeFixed=2 ** 256 - 1, feeMin=0, feeMax=0)
    Q("ok_reference", q, "s3.2: feeUnit with feeds; a uint256 word is any word", okq(q))
    q = G.quote_bytes(tok, premiumBps=10000, feeBps=10000, maxStale=2 ** 32 - 1, rulingWindow=2 ** 32 - 1, offerWindow=2 ** 32 - 1, lapseDefault=0, autoAccept=0)
    Q("ok_bounds", q, "s3.2: every bounded word at its bound, lapseDefault 0", okq(q))
    q = G.quote_bytes(tok, feeUnit=addr("usd"), feedToken=addr("feedT"))
    Q("ok_one_feed", q, "s3.2: one feed zero beside a nonzero feeUnit", okq(q))
    q = G.quote_bytes(tok, lapseDefault=1)
    Q("ok_lapse_half", q, "s3.2: lapseDefault 1, the half share", okq(q))
    Q("ok_upper_transport", q.upper().replace("0X", "0x"), "HARNESS: transport hex digits of either case", okq(q))
    Q("len_479", "0x" + "00" * 479, "s3.2: one byte short of 480", fault("E_QUOTE_LEN"))
    Q("len_481", q + "00", "s3.2: one byte beyond 480", fault("E_QUOTE_LEN"))
    Q("len_512", q + "00" * 32, "s3.2: sixteen words are no quote", fault("E_QUOTE_LEN"))
    Q("len_0", "0x", "s3.2: the empty byte string", fault("E_QUOTE_LEN"))
    for a_, b_, va, vb in ((0, 1, 0, 10001), (1, 2, 10001, 2 ** 160), (2, 3, 2 ** 160, 2 ** 160), (3, 4, 2 ** 160, 2 ** 160),
                           (4, 5, 2 ** 160, 2 ** 32), (5, 7, 2 ** 32, 10001), (7, 11, 10001, 2 ** 32), (11, 12, 2 ** 32, 2 ** 32),
                           (12, 13, 2 ** 32, 3), (13, 14, 3, 2)):
        Q("order_w%d_before_w%d" % (a_, b_), G.quote_words(q, **{"w%d" % a_: va, "w%d" % b_: vb}), "s3.2 order: word %d before word %d" % (a_, b_), fault("E_QUOTE_FIELD", field=a_))
    base = G.quote_bytes(tok)
    Q("w0_zero", G.quote_words(base, w0=0), "s3.2: w0 nonzero", fault("E_QUOTE_FIELD", field=0))
    Q("w0_not_address", G.quote_words(base, w0=1 << 160), "s3.2: an address word has 12 zero bytes", fault("E_QUOTE_FIELD", field=0))
    Q("w1_over", G.quote_words(base, w1=10001), "s3.2: premiumBps at most 10000", fault("E_QUOTE_FIELD", field=1))
    Q("w2_not_address", G.quote_words(base, w2=1 << 255), "s3.2: feeUnit an address word", fault("E_QUOTE_FIELD", field=2))
    Q("w3_not_address", G.quote_words(base, w2=int(addr("usd")[2:], 16), w3=1 << 160), "s3.2: feedToken an address word", fault("E_QUOTE_FIELD", field=3))
    Q("w4_not_address", G.quote_words(base, w2=int(addr("usd")[2:], 16), w4=1 << 160), "s3.2: feedUnit an address word", fault("E_QUOTE_FIELD", field=4))
    Q("w5_over", G.quote_words(base, w5=2 ** 32), "s3.2: maxStale u32", fault("E_QUOTE_FIELD", field=5))
    Q("w7_over", G.quote_words(base, w7=10001), "s3.2: feeBps at most 10000", fault("E_QUOTE_FIELD", field=7))
    Q("w11_over", G.quote_words(base, w11=2 ** 32), "s3.2: rulingWindow u32", fault("E_QUOTE_FIELD", field=11))
    Q("w12_over", G.quote_words(base, w12=2 ** 32), "s3.2: offerWindow u32", fault("E_QUOTE_FIELD", field=12))
    Q("w13_three", G.quote_words(base, w13=3), "s3.2: lapseDefault 0, 1 or 2; 3 is refused", fault("E_QUOTE_FIELD", field=13))
    Q("w14_over", G.quote_words(base, w14=2), "s3.2: autoAccept 0 or 1", fault("E_QUOTE_FIELD", field=14))
    # each word read whole as a big-endian integer (s3.1): a bounded word with bits past its ABI type's width, the free words at the top
    Q("w1_high_bits", G.quote_words(base, w1=2 ** 16 + 5), "s3.1, s3.2: premiumBps read whole; bits past 16 are no uint16", fault("E_QUOTE_FIELD", field=1))
    Q("w7_high_bits", G.quote_words(base, w7=2 ** 16), "s3.1, s3.2: feeBps read whole; bits past 16 are no uint16", fault("E_QUOTE_FIELD", field=7))
    Q("w13_high_bits", G.quote_words(base, w13=2 ** 8), "s3.1, s3.2: lapseDefault read whole; bits past 8 are no uint8", fault("E_QUOTE_FIELD", field=13))
    Q("w14_high_bits", G.quote_words(base, w14=2 ** 8), "s3.1, s3.2: autoAccept read whole; bits past 8 are no bool", fault("E_QUOTE_FIELD", field=14))
    qt = G.quote_bytes(tok, feeFixed=2 ** 256 - 1, feeMin=2 ** 256 - 1, feeMax=2 ** 256 - 1, amountCap=2 ** 256 - 1)
    Q("ok_words_top", qt, "s3.2: feeFixed, feeMin, feeMax and amountCap each at 2^256 - 1, any word", okq(qt))
    Q("order_w1_before_w13", G.quote_words(base, w1=10001, w13=3), "s3.2 order: the first faulting word", fault("E_QUOTE_FIELD", field=1))
    Q("order_field_before_feed", G.quote_words(base, w3=int(addr("feedT")[2:], 16), w14=2), "s3.2 order: every word before the feed rule", fault("E_QUOTE_FIELD", field=14))
    Q("feed_token", G.quote_words(base, w3=int(addr("feedT")[2:], 16)), "s3.2: E_QUOTE_FEED (feedToken)", fault("E_QUOTE_FEED"))
    Q("feed_unit", G.quote_words(base, w4=int(addr("feedU")[2:], 16)), "s3.2: E_QUOTE_FEED (feedUnit)", fault("E_QUOTE_FEED"))
    C.argv_case(["quote", "0xabc"], "HARNESS: transport hex has an even number of digits", None, path="quote:misuse_odd")
    C.argv_case(["quote", "0xzz"], "HARNESS: transport hex digits only", None, path="quote:misuse_digits")
    C.argv_case(["quote", "abcd"], "HARNESS: transport hex begins 0x", None, path="quote:misuse_prefix")
    C.argv_case(["quote", "0Xabcd"], "HARNESS: the prefix is lowercase", None, path="quote:misuse_0X")
    C.argv_case(["quote"], "HARNESS: no argument", None, path="quote:misuse_none")
    C.argv_case(["quote", q, q], "HARNESS: an argument beyond the list", None, path="quote:misuse_two")


# ==========================================================================
# offer, order, fee
# ==========================================================================

def gen_derivations():
    tok = addr("token")
    gj = gj_of(tok)
    gj2 = gj_of(tok, judge=addr("judge2"), quoteHash=G.qhash(G.quote_bytes(tok, feeUnit=addr("usd"))))
    gg = gg_of(tok, quoteHash=G.qhash(G.quote_bytes(tok, autoAccept=0, feeMin=0, feeMax=0, feeFixed=0, feeBps=250)))
    blk = G.base_block(judges=[gj, gj2], guilds=[gg])
    b = doc(blk)
    path = D("derivations", b, "the document the derivation cases read", ok(b))
    global DERIV
    DERIV = path
    th = sha(b)

    def off(c):
        return G.offer_of(blk, th, c)

    C.case("offer", [path, "0"], "s4.3: the first judge candidate", G.offer_answer(blk, off(0)), path=path)
    C.case("offer", [path, "1"], "s4.3: the second judge candidate", G.offer_answer(blk, off(1)), path=path)
    C.case("offer", [path, "2"], "s4.3: the guild candidate, judge zero", G.offer_answer(blk, off(2)), path=path)
    C.case("offer", [path, "3"], "s4.3: E_OFFER_CANDIDATE", fault("E_OFFER_CANDIDATE"), path=path)
    C.case("offer", [path, "9007199254740991"], "s4.3: E_OFFER_CANDIDATE at the int bound", fault("E_OFFER_CANDIDATE"), path=path)
    C.case("offer", [NO_BLOCK, "0"], "s4.3: the document's fault first", fault("E_TERMS_BLOCK"), path=NO_BLOCK)
    C.case("offer", [path, "01"], "HARNESS: c with a leading zero", None, path=path)
    C.case("offer", [path, ""], "HARNESS: an empty c", None, path=path)
    C.case("offer", [path, "-1"], "HARNESS: c with a sign", None, path=path)
    C.case("offer", [path, "9007199254740992"], "HARNESS: c above 2^53 - 1", None, path=path)
    C.case("offer", [path], "HARNESS: an argument short", None, path=path)
    C.case("offer", [path, "0", "0"], "HARNESS: an argument beyond the list", None, path=path)
    blk2 = G.base_block(buyer_may_open=False, open_window=0, goods=..., goodsHash=G.text_hash(G.GOODS))
    b2 = doc(blk2)
    p2 = D("offer_flags", b2, "the document for the buyerMayOpen-false, openWindow-0, hashed-goods offer", ok(b2))
    C.case("offer", [p2, "0"], "s4.3: buyerMayOpen false and openWindow 0 reach the offer; the hashed segment is not in it", G.offer_answer(blk2, G.offer_of(blk2, sha(b2), 0)), path=p2)
    # a document whose sellerFund is another address than sellerKey (a company's wallet): the derivations read it
    blkf = G.base_block(sellerFund=addr("company-wallet"), judges=[gj, gj2], guilds=[gg])
    bf = doc(blkf)
    pf = D("derivations_fund_other", bf, "the document with a sellerFund apart from sellerKey the derivation cases read", ok(bf))
    of_ = G.offer_of(blkf, sha(bf), 0)
    C.case("offer", [pf, "0"], "offer_fund_other, s4.3: every member from the document, sellerFund apart from sellerKey", G.offer_answer(blkf, of_), path=pf)

    bk = addr("buyer")
    bp = addr("buyer-payout")
    o = off(0)
    ob = str(OB)
    ov = G.order_values(blk, o, bk, bp, OB)
    C.case("order", [path, "0", bk, bp, ob, ZERO20], "s4.4: the order, its salt, instance, and digest", dict(ov, caseId=ov["salt"], ok=True), path=path)
    o2 = off(2)
    ov2 = G.order_values(blk, o2, bk, bp, OB)
    C.case("order", [path, "2", bk, bp, ob, ZERO20], "s4.4: a guild candidate's order", dict(ov2, caseId=ov2["salt"], ok=True), path=path)
    ov3 = G.order_values(blk, o, bk, bk, OB)
    C.case("order", [path, "0", bk, bk, ob, ZERO20], "s4.4: buyerPayout may equal buyerKey", dict(ov3, caseId=ov3["salt"], ok=True), path=path)
    ov4 = G.order_values(blk, o, bk, addr("seller"), OB)
    C.case("order", [path, "0", bk, addr("seller"), ob, ZERO20], "s4.4: buyerPayout may be any nonzero address, the seller's included", dict(ov4, caseId=ov4["salt"], ok=True), path=path)
    ov6 = G.order_values(blk, o, bk, bp, 0)
    C.case("order", [path, "0", bk, bp, "0", ZERO20], "s4.4: openBy 0 is an int; another openBy is another salt", dict(ov6, caseId=ov6["salt"], ok=True), path=path)
    ov7 = G.order_values(blk, o, bk, bp, 2 ** 53 - 1)
    C.case("order", [path, "0", bk, bp, str(2 ** 53 - 1), ZERO20], "s4.4: openBy at the int bound", dict(ov7, caseId=ov7["salt"], ok=True), path=path)
    C.case("order", [path, "0", ZERO20, bp, ob, ZERO20], "s4.4: buyerKey zero", fault("E_ORDER_BUYER"), path=path)
    C.case("order", [path, "0", addr("seller"), bp, ob, ZERO20], "s4.4: buyerKey the seller's", fault("E_ORDER_BUYER"), path=path)
    C.case("order", [path, "0", addr("judge"), bp, ob, ZERO20], "s4.4: buyerKey the judge's", fault("E_ORDER_BUYER"), path=path)
    ov5 = G.order_values(blk, o2, addr("judge"), bp, OB)
    C.case("order", [path, "2", addr("judge"), bp, ob, ZERO20], "s4.4: on a guild candidate the judge is zero, so a buyer at the named judge's address is not refused", dict(ov5, caseId=ov5["salt"], ok=True), path=path)
    C.case("order", [path, "2", addr("guild"), bp, ob, ZERO20], "order_buyer_is_guild, s4.4: buyerKey byte-equal to the guild's address", fault("E_ORDER_BUYER"), path=path)
    ov8 = G.order_values(blk, off(1), addr("judge"), bp, OB)
    C.case("order", [path, "1", addr("judge"), bp, ob, ZERO20], "s4.4: the first judge's address is no party to the second judge's offer", dict(ov8, caseId=ov8["salt"], ok=True), path=path)
    ovf = G.order_values(blkf, of_, bk, bp, OB)
    C.case("order", [pf, "0", bk, bp, ob, ZERO20], "order_fund_other, s4.4: the order over an offer whose sellerFund is another address; its salt and instance carry it", dict(ovf, caseId=ovf["salt"], ok=True), path=pf)
    ovbf = G.order_values(blkf, of_, addr("company-wallet"), bp, OB)
    C.case("order", [pf, "0", addr("company-wallet"), bp, ob, ZERO20], "order_buyer_is_fund, s4.4: buyerKey excludes the seller's key, the judge and the guild, not the seller's fund", dict(ovbf, caseId=ovbf["salt"], ok=True), path=pf)
    odf = G.digest(blkf["chainId"], blkf["logic"], G.hash_offer(of_))
    C.case("verify-offer", [pf, "0", G.sign(key("seller"), odf)], "verify_offer_fund_other, s5.1: the seller's key signs an offer whose sellerFund is another address; the fund signs nothing", {"form": "secp256k1", "ok": True, "signer": addr("seller")}, path=pf)
    # the fund's own key signing: no signature of the fund's is the seller's (s5.1 verifies against sellerKey)
    C.case("verify-offer", [pf, "0", G.sign(key("company-wallet"), odf)], "verify_offer_by_fund, s5.1: the fund's key signs the offer that names it; the signature recovers another address than sellerKey", fault("E_SIG_SIGNER"), path=pf)
    rdg = G.action_digest(blkf["chainId"], ovf["instance"], "Refund")
    prf = C.write("sigs", "act_refund_by_fund", json.dumps(G.sign(key("company-wallet"), rdg)).encode("utf-8"))
    C.case("verify-action", [pf, "0", bk, bp, ob, ZERO20, "Refund", "seller", prf], "act_refund_by_fund, s4.5/s5.1: a Refund the fund's key signed, under the seller", fault("E_SIG_SIGNER"), path=prf)
    prs = C.write("sigs", "act_refund_seller_fund_other", json.dumps(G.sign(key("seller"), rdg)).encode("utf-8"))
    C.case("verify-action", [pf, "0", bk, bp, ob, ZERO20, "Refund", "seller", prs], "act_refund_seller_fund_other, s4.5/s5.1: the seller's key signs the Refund of an order whose sellerFund is another address", {"form": "secp256k1", "ok": True}, path=prs)
    # every int member at the top of its range: chainId, nonce and expiry at 2^53 - 1, a source and a pointer on that chain
    TOP = 2 ** 53 - 1
    blkt = G.base_block(chain_id=TOP, nonce=TOP, expiry=TOP,
                        judges=[gj_of(tok, source="galeed:guild:%d:%s" % (TOP, addr("guild")))],
                        guilds=[gg_of(tok, pointer="galeed:guild:%d:%s" % (TOP, addr("guild")))])
    bt = doc(blkt)
    pt = D("ok_int_top", bt, "s1, s2.3: chainId, nonce and expiry at 2^53 - 1, a judge's source and a guild's pointer on that chain", ok(bt))
    for c_ in (0, 1):
        C.case("offer", [pt, str(c_)], "ok_int_top, s4.3: the offer at the int bound, candidate %d" % c_, G.offer_answer(blkt, G.offer_of(blkt, sha(bt), c_)), path=pt)
    ovt = G.order_values(blkt, G.offer_of(blkt, sha(bt), 0), bk, bp, OB)
    C.case("order", [pt, "0", bk, bp, ob, ZERO20], "ok_int_top, s4.4: the order under a domain whose chainId is 2^53 - 1", dict(ovt, caseId=ovt["salt"], ok=True), path=pt)
    C.case("order", [path, "0", bk, ZERO20, ob, ZERO20], "s4.4: buyerPayout zero", fault("E_ORDER_PAYOUT"), path=path)
    PREV1 = addr("stage-before")
    ovp = G.order_values(blk, o, bk, bp, OB, PREV1)
    C.case("order", [path, "0", bk, bp, ob, PREV1], "order_prev_nonzero, s4.4: a nonzero prev, the instance of the stage before, is part of the salt; another salt, instance and digest", dict(ovp, caseId=ovp["salt"], ok=True), path=path)
    C.case("order", [path, "0", bk, bp, ob, "0X" + PREV1[2:]], "HARNESS: prev outside hex20 (an uppercase prefix)", None, path=path)
    C.case("order", [path, "0", bk, bp, ob, "0x12"], "HARNESS: prev short", None, path=path)
    C.case("order", [path, "0", bk, bp, ob], "HARNESS: an argument short (no prev)", None, path=path)
    C.case("order", [path, "0", bk, bp, ob, "0x" + PREV1[2:].upper()], "HARNESS: prev with uppercase digits", None, path=path)
    C.case("order", [path, "0", bk, bp, ob, PREV1[2:]], "HARNESS: prev without its prefix", None, path=path)
    C.case("order", [path, "0", bk, bp, ob, ""], "HARNESS: prev the empty string (zero is spelt in full)", None, path=path)

    C.case("order", [path, "0", ZERO20, ZERO20, ob, ZERO20], "s4.4 order: buyer before payout", fault("E_ORDER_BUYER"), path=path)
    C.case("order", [path, "3", bk, bp, ob, ZERO20], "s4.4: the offer's fault first", fault("E_OFFER_CANDIDATE"), path=path)
    C.case("order", [path, "0", bk.upper().replace("0X", "0x"), bp, ob, ZERO20], "HARNESS: buyerKey outside hex20", None, path=path)
    C.case("order", [path, "0", bk, "0x12", ob, ZERO20], "HARNESS: buyerPayout short", None, path=path)
    C.case("order", [path, "0", bk, bp, "01700086400", ZERO20], "HARNESS: openBy with a leading zero", None, path=path)
    C.case("order", [path, "0", bk, bp, "-5", ZERO20], "HARNESS: openBy with a sign", None, path=path)
    C.case("order", [path, "0", bk, bp, "9007199254740992", ZERO20], "HARNESS: openBy above 2^53 - 1", None, path=path)
    C.case("order", [path, "0", bk, bp], "HARNESS: an argument short (no openBy)", None, path=path)
    C.case("order", [path, "0", bk, bp, ob, ZERO20, "x"], "HARNESS: an argument beyond the list", None, path=path)

    # ---- fee over quote bytes and an amount
    qd = G.quote_bytes(tok)
    A = "1000000000"
    C.case("fee", [], "s6.1: lf = fixed + amount*bps/10000 inside the bounds; premium", {"lf": str(50000000 + 1000000000 * 1000 // 10000), "ok": True, "premium": str(1000000000 * 100 // 10000), "priced": True}, path="fee:direct")
    C.manifest[-1]["args"] = [qd, A]
    def F(slug, qhex, amount, note, predicted):
        C.argv_case(["fee", qhex, amount], slug + ": " + note, predicted, path="fee:" + slug)
    C.manifest.pop()
    F("direct", qd, A, "s6.1: lf = fixed + amount*bps/10000 inside the bounds; premium", {"lf": str(50000000 + 1000000000 * 1000 // 10000), "ok": True, "premium": str(1000000000 * 100 // 10000), "priced": True})
    F("reference_unit", G.quote_bytes(tok, feeUnit=addr("usd")), A, "s6.1: feeUnit nonzero, priced at init", {"lf": None, "ok": True, "premium": str(1000000000 * 100 // 10000), "priced": False})
    F("no_bounds", G.quote_bytes(tok, feeMin=0, feeMax=0, feeFixed=0, feeBps=250), A, "s6.1: no bounds, bps alone", {"lf": str(1000000000 * 250 // 10000), "ok": True, "premium": str(1000000000 * 100 // 10000), "priced": True})
    F("min", G.quote_bytes(tok, feeFixed=0, feeBps=100, feeMin=7, feeMax=0, premiumBps=3), "10", "s6.1: lf raised to feeMin; premium floors to 0", {"lf": "7", "ok": True, "premium": "0", "priced": True})
    F("max", G.quote_bytes(tok, feeFixed=5, feeBps=10000, feeMin=0, feeMax=9), "10000", "s6.1: lf lowered to feeMax", {"lf": "9", "ok": True, "premium": "100", "priced": True})
    F("min_above_max", G.quote_bytes(tok, feeFixed=0, feeBps=1, feeMin=50, feeMax=20), "10000", "s6.1: raised to feeMin, then lowered to feeMax", {"lf": "20", "ok": True, "premium": "100", "priced": True})
    F("overflow", G.quote_bytes(tok, feeFixed=2 ** 256 - 1, feeBps=1, feeMin=0, feeMax=0), str(2 ** 256 - 1), "s6.1: E_FEE_OVERFLOW", fault("E_FEE_OVERFLOW"))
    F("max_no_overflow", G.quote_bytes(tok, feeFixed=2 ** 256 - 1, feeBps=0, feeMin=0, feeMax=5), str(2 ** 256 - 1), "s6.1: feeFixed at the top with feeBps 0 does not overflow; feeMax lowers it", {"lf": "5", "ok": True, "premium": str((2 ** 256 - 1) * 100 // 10000), "priced": True})
    F("amount_zero", qd, "0", "s6.1: an amount of 0 is a uint256; lf is feeFixed raised to feeMin", {"lf": str(50000000), "ok": True, "premium": "0", "priced": True})
    F("amount_max", G.quote_bytes(tok, premiumBps=10000, feeBps=0, feeFixed=0, feeMin=0, feeMax=0), str(2 ** 256 - 1), "s6.1: premium of the whole amount at 10000 bps; the fee's amount is any uint256", {"lf": "0", "ok": True, "premium": str(2 ** 256 - 1), "priced": True})
    F("sum_overflow", G.quote_bytes(tok, premiumBps=10000, feeBps=0, feeFixed=1, feeMin=0, feeMax=0), str(2 ** 256 - 1), "s6.1: premium + lf above 2^256 - 1 is E_FEE_OVERFLOW", fault("E_FEE_OVERFLOW"))
    F("sum_at_bound", G.quote_bytes(tok, premiumBps=10000, feeBps=0, feeFixed=1, feeMin=0, feeMax=0), str(2 ** 256 - 2), "s6.1: premium + lf exactly 2^256 - 1 fits", {"lf": "1", "ok": True, "premium": str(2 ** 256 - 2), "priced": True})
    F("overflow_before_max", G.quote_bytes(tok, premiumBps=100, feeFixed=2 ** 256 - 1, feeBps=1, feeMin=0, feeMax=5), "10000", "s6.1 order: the first sum is decided before the bounds; a feeMax that would have lowered it does not save it", fault("E_FEE_OVERFLOW"))
    F("sum_after_min", G.quote_bytes(tok, premiumBps=10000, feeFixed=0, feeBps=0, feeMin=2, feeMax=0), str(2 ** 256 - 2), "s6.1 order: premium + lf is decided after the bounds; the raise to feeMin makes it overflow", fault("E_FEE_OVERFLOW"))
    F("unit_nonzero_no_sum", G.quote_bytes(tok, feeUnit=addr("usd"), premiumBps=10000, feeBps=10000, feeFixed=2 ** 256 - 1, feeMin=0, feeMax=0), str(2 ** 256 - 1), "s6.1: with feeUnit nonzero no sum of the section is formed and nothing faults; premium alone, priced false", {"lf": None, "ok": True, "premium": str(2 ** 256 - 1), "priced": False})
    F("min_top", G.quote_bytes(tok, premiumBps=0, feeFixed=0, feeBps=0, feeMin=2 ** 256 - 1, feeMax=0), "10000", "s6.1: feeMin at 2^256 - 1 raises lf to it; premium 0, the sum fits", {"lf": str(2 ** 256 - 1), "ok": True, "premium": "0", "priced": True})
    F("min_top_sum", G.quote_bytes(tok, premiumBps=1, feeFixed=0, feeBps=0, feeMin=2 ** 256 - 1, feeMax=0), "10000", "s6.1: feeMin at 2^256 - 1 with a premium of 1, premium + lf overflows", fault("E_FEE_OVERFLOW"))
    F("max_top", G.quote_bytes(tok, premiumBps=0, feeFixed=2 ** 255, feeBps=0, feeMin=0, feeMax=2 ** 256 - 1), "10000", "s6.1: feeMax at 2^256 - 1 lowers nothing; lf is feeFixed at 2^255, both words read whole", {"lf": str(2 ** 255), "ok": True, "premium": "0", "priced": True})
    F("max_high_lowers", G.quote_bytes(tok, premiumBps=0, feeFixed=2 ** 201, feeBps=0, feeMin=0, feeMax=2 ** 200), "10000", "s6.1: feeMax at 2^200 lowers lf from 2^201, both words past 128 bits", {"lf": str(2 ** 200), "ok": True, "premium": "0", "priced": True})
    F("quote_len_first", "0x" + "00" * 479, A, "s3.2 first: a quote the decoder refuses", fault("E_QUOTE_LEN"))
    F("quote_field_first", G.quote_words(qd, w13=3), A, "s3.2 first: lapseDefault 3", fault("E_QUOTE_FIELD", field=13))
    F("quote_feed_first", G.quote_words(qd, w4=int(addr("feedU")[2:], 16)), str(2 ** 256 - 1), "s3.2 order: the feed rule before the fee's overflow", fault("E_QUOTE_FEED"))
    C.argv_case(["fee", qd, "01"], "HARNESS: amount with a leading zero", None, path="fee:misuse_leading_zero")
    C.argv_case(["fee", qd, str(2 ** 256)], "HARNESS: amount above 2^256 - 1", None, path="fee:misuse_too_big")
    C.argv_case(["fee", qd, "1e9"], "HARNESS: amount not decimal digits", None, path="fee:misuse_exp")
    C.argv_case(["fee", qd, ""], "HARNESS: an empty amount", None, path="fee:misuse_empty")
    C.argv_case(["fee", "0xabc", A], "HARNESS: quote not transport hex", None, path="fee:misuse_hex")
    C.argv_case(["fee", qd], "HARNESS: an argument short", None, path="fee:misuse_short")
    C.argv_case(["fee", path, "0"], "HARNESS: a path is no transport hex", None, path="fee:misuse_path")


# ==========================================================================
# signatures
# ==========================================================================

def gen_signatures():
    tok = addr("token")
    gj = gj_of(tok)
    gg = gg_of(tok)
    blk = G.base_block(judges=[gj], guilds=[gg])
    b = doc(blk)
    path = D("signatures", b, "the document the signature cases read", ok(b))
    th = sha(b)
    o = G.offer_of(blk, th, 0)
    og = G.offer_of(blk, th, 1)
    od = G.digest(1, blk["logic"], G.hash_offer(o))
    ogd = G.digest(1, blk["logic"], G.hash_offer(og))
    sk = key("seller")
    sig = G.sign(sk, od)
    sigg = G.sign(sk, ogd)
    ob = str(OB)
    C.case("sign-offer", [G.privhex("seller"), path, "0"], "s5.5: the seller's deterministic signature", {"digest": hx(od), "ok": True, "sig": sig, "signer": addr("seller")}, path=path)
    C.case("sign-offer", [G.privhex("seller"), path, "1"], "s5.5: the guild candidate's offer", {"digest": hx(ogd), "ok": True, "sig": sigg, "signer": addr("seller")}, path=path)
    C.case("sign-offer", [G.privhex("buyer"), path, "0"], "sign-offer signs with any key; the signer is that key's address", {"digest": hx(od), "ok": True, "sig": G.sign(key("buyer"), od), "signer": addr("buyer")}, path=path)
    C.case("sign-offer", [G.privhex("seller")[2:].upper(), path, "0"], "HARNESS: the key without 0x and in uppercase", {"digest": hx(od), "ok": True, "sig": sig, "signer": addr("seller")}, path=path)
    C.case("sign-offer", ["0X" + G.privhex("seller")[2:], path, "0"], "privkey_0X_prefix, HARNESS: the 0X prefix", {"digest": hx(od), "ok": True, "sig": sig, "signer": addr("seller")}, path=path)
    C.case("sign-offer", ["0x" + "".join(ch.upper() if i % 2 else ch for i, ch in enumerate(G.privhex("seller")[2:])), path, "0"], "privkey_mixed_case, HARNESS: digits of either case, mixed", {"digest": hx(od), "ok": True, "sig": sig, "signer": addr("seller")}, path=path)
    C.case("sign-offer", ["0x" + "g" + G.privhex("seller")[3:], path, "0"], "privkey_non_hex_digit, HARNESS: a non-hex digit is misuse", None, path=path)
    C.case("sign-offer", [G.privhex("seller"), path, "2"], "s4.3: the derivation's fault", fault("E_OFFER_CANDIDATE"), path=path)
    C.case("sign-offer", ["0x" + "00" * 32, path, "0"], "HARNESS: a zero key is out of range", None, path=path)
    C.case("sign-offer", ["0x%064x" % G.N_ORDER, path, "0"], "HARNESS: n is out of range", None, path=path)
    C.case("sign-offer", ["0x12", path, "0"], "HARNESS: a short key", None, path=path)
    C.case("sign-offer", [G.privhex("seller"), path], "HARNESS: an argument short", None, path=path)

    C.case("verify-offer", [path, "0", sig], "s5.1: the seller's signature verifies", {"form": "secp256k1", "ok": True, "signer": addr("seller")}, path=path)
    C.case("verify-offer", [NO_BLOCK, "0", sig], "s2.4 first: the document's fault", fault("E_TERMS_BLOCK"), path=NO_BLOCK)
    C.case("verify-offer", [path, "2", sig], "s4.3 first: the derivation's fault", fault("E_OFFER_CANDIDATE"), path=path)
    C.case("verify-offer", [path, "1", sigg], "s5.1: on the guild candidate's offer", {"form": "secp256k1", "ok": True, "signer": addr("seller")}, path=path)
    C.case("verify-offer", [path, "1", sig], "s5.1: a signature over another offer recovers another address", fault("E_SIG_SIGNER"), path=path)
    C.case("verify-offer", [path, "0", G.sign(key("buyer"), od)], "s5.1: another key", fault("E_SIG_SIGNER"), path=path)
    C.case("verify-offer", [path, "0", "0x1234"], "s5.1: E_SIG_FORM", fault("E_SIG_FORM"), path=path)
    C.case("verify-offer", [path, "0", sig.upper().replace("0X", "0x")], "s5.1: hex65 is lowercase", fault("E_SIG_FORM"), path=path)
    C.case("verify-offer", [path, "0", sig[:-2] + "1d"], "galeed/1 s5.4: v outside {27, 28}", fault("E_SIG_V"), path=path)
    C.case("verify-offer", [path, "0", "0x" + "00" * 64 + "1b"], "galeed/1 s5.4: r and s in range", fault("E_SIG_RANGE"), path=path)
    r_, s_, v_ = G.sign_digest(sk, od)
    high = G.sig_hex(r_, G.N_ORDER - s_, 55 - v_)
    C.case("verify-offer", [path, "0", high], "galeed/1 s5.4: the malleability twin is high-s", fault("E_SIG_HIGH_S"), path=path)
    C.case("verify-offer", [path, "0", G.sig_hex(G.N_ORDER - 1, 1, 27)], "galeed/1 s5.4: an r with no point recovers nothing", fault("E_SIG_RECOVER"), path=path)
    C.case("verify-offer", [path, "0", G.sig_hex(5, 5, 27)], "galeed/1 s5.4: x = 5 is no point of the curve", fault("E_SIG_RECOVER"), path=path)
    k_ = 0x1234567
    Rx, Ry = G.pubkey(k_)
    z_ = int.from_bytes(od, "big") % G.N_ORDER
    s_inf = z_ * pow(k_, -1, G.N_ORDER) % G.N_ORDER
    r_inf, v_inf = Rx % G.N_ORDER, 27 + (Ry & 1)
    if s_inf > G.N_ORDER // 2:
        s_inf, v_inf = G.N_ORDER - s_inf, 55 - v_inf
    C.case("verify-offer", [path, "0", G.sig_hex(r_inf, s_inf, v_inf)], "verify_offer_infinity, galeed/1 s5.4: R = kG and s = z/k recover the point at infinity", fault("E_SIG_RECOVER"), path=path)
    GX = 0x79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798
    C.case("verify-offer", [path, "0", G.sig_hex(GX, 5, 27)], "galeed/1 s5.4: a wrong signature over a real point recovers a stranger", fault("E_SIG_SIGNER"), path=path)
    C.case("verify-offer", [path, "0", sig[:-2] + "00"], "v_zero, galeed/1 s5.4: v 0 is outside {27, 28}, whatever a library would normalise", fault("E_SIG_V"), path=path)
    C.case("verify-offer", [path, "0", sig[:-2] + "01"], "v_one, galeed/1 s5.4: v 1 is outside {27, 28}", fault("E_SIG_V"), path=path)
    C.case("verify-offer", [path, "0", sig[:-2] + "1a"], "v_26, galeed/1 s5.4: v 26 is outside {27, 28}", fault("E_SIG_V"), path=path)
    C.case("verify-offer", [path, "0", G.sig_hex(G.N_ORDER, 1, 27)], "r_n, galeed/1 s5.4: r = n beside an s in range", fault("E_SIG_RANGE"), path=path)
    C.case("verify-offer", [path, "0", G.sig_hex(0, 1, 27)], "r_zero_s_in_range, galeed/1 s5.4: r = 0 beside an s in range", fault("E_SIG_RANGE"), path=path)
    C.case("verify-offer", [path, "0", G.sig_hex(GX, 0, 27)], "s_zero_r_in_range, galeed/1 s5.4: s = 0 beside an r in range", fault("E_SIG_RANGE"), path=path)
    C.case("verify-offer", [path, "0", G.sig_hex(GX, (G.N_ORDER - 1) // 2, 27)], "s_half, galeed/1 s5.4: s = (n - 1)/2 passes the low-s test; the real point recovers a stranger", fault("E_SIG_SIGNER"), path=path)
    C.case("verify-offer", [path, "0", G.sig_hex(GX, (G.N_ORDER - 1) // 2 + 1, 27)], "s_half_plus_one, galeed/1 s5.4: s = (n - 1)/2 + 1 is high", fault("E_SIG_HIGH_S"), path=path)
    C.case("verify-offer", [path, "0", sig[:-2] + "1d" + "00"], "s5.1: a 66-byte value fails the form", fault("E_SIG_FORM"), path=path)
    C.case("verify-offer", [path, "0", G.sig_hex(0, 1, 29)], "sig_order_v_before_range, galeed/1 s5.4 order: v before the range", fault("E_SIG_V"), path=path)
    C.case("verify-offer", [path, "0", G.sig_hex(GX, G.N_ORDER, 27)], "sig_order_range_before_high_s, galeed/1 s5.4 order: the range before high-s", fault("E_SIG_RANGE"), path=path)
    C.case("verify-offer", [path, "0", G.sig_hex(0, G.N_ORDER - 1, 27)], "sig_order_r_zero_before_high_s, galeed/1 s5.4 order: r = 0 fails the range before an s in range and high is tested", fault("E_SIG_RANGE"), path=path)
    C.case("verify-offer", [path, "0", G.sig_hex(G.N_ORDER, G.N_ORDER - 1, 27)], "sig_order_r_n_before_high_s, galeed/1 s5.4 order: r = n fails the range before an s in range and high is tested", fault("E_SIG_RANGE"), path=path)
    C.case("verify-offer", [path, "0", "0X" + sig[2:]], "sig_prefix_upper, galeed/1 s1: a hex65 with an uppercase prefix fails the form", fault("E_SIG_FORM"), path=path)
    C.case("verify-offer", [path, "0", sig[2:]], "sig_prefix_absent, galeed/1 s1: a hex65 with no prefix fails the form", fault("E_SIG_FORM"), path=path)
    C.case("verify-offer", [path, "0", G.sig_hex(5, G.N_ORDER - 1, 27)], "sig_order_high_s_before_recover, galeed/1 s5.4 order: high-s before recovery", fault("E_SIG_HIGH_S"), path=path)
    C.case("verify-offer", [path, "0", ""], "s5.4: the empty string is the second form, SIG_BY_CALL, no signer named", {"form": "call", "ok": True}, path=path)
    C.case("verify-offer", [path, "1", ""], "s5.4: the second form on the guild candidate's offer", {"form": "call", "ok": True}, path=path)
    C.case("verify-offer", [path, "0", "0x"], "s5.4: a string spelling zero bytes is no second form", fault("E_SIG_FORM"), path=path)
    C.case("verify-offer", [NO_BLOCK, "0", ""], "s2.4 first: the document's fault before the second form", fault("E_TERMS_BLOCK"), path=NO_BLOCK)
    C.case("verify-offer", [path, "2", ""], "s4.3 first: the derivation's fault before the second form", fault("E_OFFER_CANDIDATE"), path=path)
    C.case("verify-offer", [path, "0"], "HARNESS: an argument short", None, path=path)
    C.case("verify-offer", [path, "0", sig, sig], "HARNESS: an argument beyond the list", None, path=path)

    bk = addr("buyer")
    bp = addr("buyer-payout")
    ov = G.order_values(blk, o, bk, bp, OB)
    odg = bytes.fromhex(ov["digest"][2:])
    bsig = G.sign(key("buyer"), odg)
    C.case("sign-order", [G.privhex("buyer"), path, "0", bp, ob, ZERO20], "s5.5: the buyer's deterministic signature; buyerKey is the key's address", {"buyerKey": bk, "digest": ov["digest"], "instance": ov["instance"], "ok": True, "sig": bsig}, path=path)
    C.case("sign-order", [G.privhex("seller"), path, "0", bp, ob, ZERO20], "s4.4: the seller's key as buyer is refused", fault("E_ORDER_BUYER"), path=path)
    C.case("sign-order", [G.privhex("judge"), path, "0", bp, ob, ZERO20], "s4.4: the judge's key as buyer is refused", fault("E_ORDER_BUYER"), path=path)
    ovg = G.order_values(blk, og, addr("judge"), bp, OB)
    C.case("sign-order", [G.privhex("judge"), path, "1", bp, ob, ZERO20], "s4.4: on the guild candidate the named judge is no party", {"buyerKey": addr("judge"), "digest": ovg["digest"], "instance": ovg["instance"], "ok": True, "sig": G.sign(key("judge"), bytes.fromhex(ovg["digest"][2:]))}, path=path)
    C.case("sign-order", [G.privhex("buyer"), path, "0", ZERO20, ob, ZERO20], "s4.4: buyerPayout zero", fault("E_ORDER_PAYOUT"), path=path)
    ovp = G.order_values(blk, o, bk, bp, OB, addr("stage-before"))
    C.case("sign-order", [G.privhex("buyer"), path, "0", bp, ob, addr("stage-before")], "sign_order_prev, s4.4, s5.5: the buyer signs the order of a later stage, prev the instance before", {"buyerKey": bk, "digest": ovp["digest"], "instance": ovp["instance"], "ok": True, "sig": G.sign(key("buyer"), bytes.fromhex(ovp["digest"][2:]))}, path=path)
    C.case("sign-order", [G.privhex("buyer"), path, "0", bp, ob, addr("stage-before")[2:]], "HARNESS: prev without its prefix", None, path=path)

    ov0 = G.order_values(blk, o, bk, bp, 0)
    C.case("sign-order", [G.privhex("buyer"), path, "0", bp, "0", ZERO20], "s4.4: another openBy is another digest", {"buyerKey": bk, "digest": ov0["digest"], "instance": ov0["instance"], "ok": True, "sig": G.sign(key("buyer"), bytes.fromhex(ov0["digest"][2:]))}, path=path)
    C.case("sign-order", [G.privhex("buyer"), path, "0", bp], "HARNESS: an argument short (no openBy)", None, path=path)
    C.case("sign-order", [G.privhex("buyer"), path, "0", bp, "x", ZERO20], "HARNESS: openBy not an int", None, path=path)

    def S(slug, value, note, predicted, bkey=bk, bpay=bp, c="0", openby=ob, prev_=ZERO20):
        p = C.write("sigs", slug, json.dumps(value, ensure_ascii=False).encode("utf-8"))
        C.case("verify-order", [path, c, bkey, bpay, openby, prev_, p], note, predicted, path=p)

    S("secp_ok", bsig, "s5.2: the buyer's secp256k1 signature", {"form": "secp256k1", "instance": ov["instance"], "ok": True})
    S("secp_other_payout", bsig, "s5.2: another payout is another order", fault("E_SIG_SIGNER"), bpay=addr("other-payout"))
    S("secp_other_openby", bsig, "s5.2: another openBy is another order", fault("E_SIG_SIGNER"), openby=str(OB + 1))
    S("secp_other_prev", bsig, "s5.2, s4.4: another prev is another order", fault("E_SIG_SIGNER"), prev_=addr("stage-before"))
    ovp2 = G.order_values(blk, o, bk, bp, OB, addr("stage-before"))
    S("secp_prev_ok", G.sign(key("buyer"), bytes.fromhex(ovp2["digest"][2:])), "s5.2: the buyer's signature over a later stage's order", {"form": "secp256k1", "instance": ovp2["instance"], "ok": True}, prev_=addr("stage-before"))
    S("secp_prev_misuse", G.sign(key("buyer"), bytes.fromhex(ovp2["digest"][2:])), "HARNESS: prev of twenty-one bytes", None, prev_=addr("stage-before") + "00")

    S("secp_wrong_key", G.sign(key("stranger"), odg), "s5.2: a stranger's signature", fault("E_SIG_SIGNER"))
    S("secp_form", "0x12", "s5.2: a string outside hex65", fault("E_SIG_FORM"))
    S("secp_int", 5, "s5.2: a value that is no string", fault("E_SIG_FORM"))
    S("secp_null", None, "s5.2: null", fault("E_SIG_FORM"))
    S("secp_array", [bsig], "s5.2: an array", fault("E_SIG_FORM"))
    S("secp_high_s", (lambda rsv: G.sig_hex(rsv[0], G.N_ORDER - rsv[1], 55 - rsv[2]))(G.sign_digest(key("buyer"), odg)), "s5.2: high-s", fault("E_SIG_HIGH_S"))
    S("secp_buyer_zero", bsig, "s4.4 first: buyerKey zero", fault("E_ORDER_BUYER"), bkey=ZERO20)
    S("secp_offer_fault", bsig, "s4.3 first: E_OFFER_CANDIDATE", fault("E_OFFER_CANDIDATE"), c="2")
    S("by_call", "", "s5.4: the empty string is the second form, SIG_BY_CALL", {"form": "call", "instance": ov["instance"], "ok": True})
    S("by_call_zero_bytes", "0x", "s5.4: a string spelling zero bytes is E_SIG_FORM", fault("E_SIG_FORM"))
    S("by_call_buyer_zero", "", "s4.4 first: buyerKey zero before the second form", fault("E_ORDER_BUYER"), bkey=ZERO20)
    S("by_call_offer_fault", "", "s4.3 first: E_OFFER_CANDIDATE before the second form", fault("E_OFFER_CANDIDATE"), c="2")
    S("by_call_payout_zero", "", "s4.4 first: buyerPayout zero before the second form", fault("E_ORDER_PAYOUT"), bpay=ZERO20)
    ovs = G.order_values(blk, o, addr("company"), bp, OB)
    S("by_call_any_key", "", "s5.4: the form names no key and reads no code; any nonzero buyerKey claims it", {"form": "call", "instance": ovs["instance"], "ok": True}, bkey=addr("company"))

    C.case("sign-order", ["0x" + "00" * 32, path, "0", bp, ob, ZERO20], "HARNESS: a zero key is out of range", None, path=path)
    nm1_addr = hx(G.address_of(G.pubkey(G.N_ORDER - 1)))
    ovn = G.order_values(blk, o, nm1_addr, bp, OB)
    C.case("sign-order", ["0x%064x" % (G.N_ORDER - 1), path, "0", bp, ob, ZERO20], "privkey_n_minus_1, HARNESS: n - 1 is the last secp256k1 key", {"buyerKey": nm1_addr, "digest": ovn["digest"], "instance": ovn["instance"], "ok": True, "sig": G.sign(G.N_ORDER - 1, bytes.fromhex(ovn["digest"][2:]))}, path=path)
    C.case("sign-order", [G.privhex("buyer"), path, "5", bp, ob, ZERO20], "s4.3: the derivation's fault", fault("E_OFFER_CANDIDATE"), path=path)
    S("object_form", {"r": "0x" + "11" * 32, "s": "0x" + "22" * 32}, "s5.2 and s5.3: an object is no signature of either party", fault("E_SIG_FORM"))
    # ---- HARNESS <sig.json>: the reader (galeed-conformance/HARNESS.md, gl1 audit: RFC 8259 with five restrictions)
    # refuses, each restriction once, and reads three neighbours it admits (the value then E_SIG_FORM)

    def R(slug, raw, note, predicted):
        p = C.write("sigs", slug, raw)
        C.case("verify-order", [path, "0", bk, bp, ob, ZERO20, p], note, predicted, path=p)
    R("reader_dup_key", b'{"a":1,"a":2}', "HARNESS <sig.json>: a member twice is refused by the reader", None)
    R("reader_fraction", b"1.5", "HARNESS <sig.json>: a fraction is refused by the reader", None)
    R("reader_negative", b"-1", "HARNESS <sig.json>: a negative number is refused by the reader", None)
    R("reader_two_pow_53", b"9007199254740992", "HARNESS <sig.json>: 2^53 is refused by the reader", None)
    R("reader_nan", b"NaN", "HARNESS <sig.json>: NaN is no JSON value", None)
    R("reader_depth_129", b"[" * 129 + b"]" * 129, "HARNESS <sig.json>: depth 129 is refused by the reader", None)
    R("reader_lone_surrogate", b'"\\ud800"', "HARNESS <sig.json>: a lone surrogate is refused by the reader", None)
    R("reader_bom", b'\xef\xbb\xbf"0x12"', "HARNESS <sig.json>: a byte order mark is refused by the reader", None)
    R("reader_surrogate_pair", b'"\\ud83d\\ude00"', "HARNESS <sig.json>: a surrogate pair reads; the string is no hex65", fault("E_SIG_FORM"))
    R("reader_two_pow_53_minus_one", b"9007199254740991", "HARNESS <sig.json>: 2^53 - 1 reads; an int is no signature", fault("E_SIG_FORM"))
    R("reader_depth_128", b"[" * 128 + b"]" * 128, "HARNESS <sig.json>: depth 128 reads; an array is no signature", fault("E_SIG_FORM"))

    # ---- s4.5 actions
    def A(name, *args):
        return G.action_hash(name, *args)

    def AD(name, *args):
        return G.action_digest(1, ov["instance"], name, *args)
    man = H32()
    for name, argv_, args in (("Confirm", [], ()), ("Refund", [], ()), ("Dispute", [], ()), ("Cancel", [], ()), ("ClaimDelivery", [man], (man,)), ("Settle", ["5000", "1800000000"], (5000, 1800000000))):
        C.case("action", [path, "0", bk, bp, ob, ZERO20, name] + argv_, "s4.5: %s's struct hash and digest" % name, {"digest": hx(AD(name, *args)), "instance": ov["instance"], "ok": True, "structHash": hx(A(name, *args))}, path=path)
    C.case("action", [path, "0", addr("seller"), bp, ob, ZERO20, "Settle", "10001", "5"], "action_order_before_args, s4.4 order: the order's token before s4.5's argument tests", fault("E_ORDER_BUYER"), path=path)
    p_ = C.write("sigs", "verify_action_order_before_args", json.dumps("0x" + "00" * 65).encode("utf-8"))
    C.case("verify-action", [path, "0", addr("seller"), bp, ob, ZERO20, "Settle", "10001", "5", "buyer", p_], "verify_action_order_before_args, s4.4 order: the order's token before s4.5's argument tests", fault("E_ORDER_BUYER"), path=p_)
    C.case("action", [path, "0", bk, bp, ob, ZERO20, "ClaimDelivery", ZERO32], "s4.5: E_ACTION_MANIFEST (zero)", fault("E_ACTION_MANIFEST"), path=path)
    C.case("action", [path, "0", bk, bp, ob, ZERO20, "ClaimDelivery", "0x12"], "s4.5: E_ACTION_MANIFEST (form)", fault("E_ACTION_MANIFEST"), path=path)
    C.case("action", [path, "0", bk, bp, ob, ZERO20, "Settle", "10001", "1"], "s4.5: E_ACTION_BPS", fault("E_ACTION_BPS"), path=path)
    C.case("action", [path, "0", bk, bp, ob, ZERO20, "Settle", "10000", "0"], "s4.5: buyerBps 10000 and expiry 0 admitted", {"digest": hx(AD("Settle", 10000, 0)), "instance": ov["instance"], "ok": True, "structHash": hx(A("Settle", 10000, 0))}, path=path)
    C.case("action", [path, "0", bk, bp, ob, ZERO20, "Settle", "5000", str(2 ** 53 - 1)], "s4.5: Settle's expiry at the int bound", {"digest": hx(AD("Settle", 5000, 2 ** 53 - 1)), "instance": ov["instance"], "ok": True, "structHash": hx(A("Settle", 5000, 2 ** 53 - 1))}, path=path)
    C.case("action", [path, "0", bk, bp, ob, ZERO20, "Settle", "5000", str(2 ** 53)], "HARNESS: Settle's expiry above 2^53 - 1 is no int", None, path=path)
    C.case("action", [path, "0", bk, bp, ob, ZERO20, "Settle", "5000", "01"], "HARNESS: Settle's expiry with a leading zero is no int", None, path=path)
    C.case("action", [path, "0", ZERO20, bp, ob, ZERO20, "Confirm"], "s4.4 first: E_ORDER_BUYER", fault("E_ORDER_BUYER"), path=path)
    C.case("action", [path, "2", bk, bp, ob, ZERO20, "Confirm"], "s4.3 first: E_OFFER_CANDIDATE", fault("E_OFFER_CANDIDATE"), path=path)
    ovx = G.order_values(blk, o, bk, bp, 0)
    C.case("action", [path, "0", bk, bp, "0", ZERO20, "Cancel"], "s4.5: the digest is under the instance the order with this openBy derives", {"digest": hx(G.action_digest(1, ovx["instance"], "Cancel")), "instance": ovx["instance"], "ok": True, "structHash": hx(A("Cancel"))}, path=path)
    C.case("action", [path, "0", bk, bp, ob, ZERO20, "Bogus"], "HARNESS: an unknown action", None, path=path)
    C.case("action", [path, "0", bk, bp, ob, ZERO20, "Dispute", "1"], "HARNESS: Dispute takes no argument; a rung is an argument beyond the list", None, path=path)
    C.case("action", [path, "0", bk, bp, ob, ZERO20, "Cancel", "0"], "HARNESS: Cancel takes no argument", None, path=path)
    C.case("action", [path, "0", bk, bp, ob, ZERO20, "Settle", "5000"], "HARNESS: an argument short", None, path=path)
    C.case("action", [path, "0", bk, bp, ob, ZERO20, "ClaimDelivery"], "HARNESS: an argument short", None, path=path)
    C.case("action", [path, "0", bk, bp, ob, ZERO20, "Settle", "x", "1"], "HARNESS: buyerBps not an int", None, path=path)
    C.case("action", [path, "0", bk, bp, ob, ZERO20, "Confirm", "x"], "HARNESS: an argument beyond the list", None, path=path)
    C.case("action", [path, "0", bk, bp, "Confirm", ZERO20], "HARNESS: an argument short (no openBy; Confirm lands in its place and is no int)", None, path=path)
    C.case("action", [path, "0", bk, bp, ob, ZERO20], "HARNESS: no action", None, path=path)
    # a later stage: the action digest is under the instance the order with this prev derives (s4.4, s4.5)
    pv1 = addr("stage-before")
    ovp1 = G.order_values(blk, o, bk, bp, OB, pv1)
    adgp = G.action_digest(1, ovp1["instance"], "Confirm")
    C.case("action", [path, "0", bk, bp, ob, pv1, "Confirm"], "act_confirm_prev, s4.5, s4.4: a later stage's Confirm is under the instance its prev derives", {"digest": hx(adgp), "instance": ovp1["instance"], "ok": True, "structHash": hx(A("Confirm"))}, path=path)
    C.case("action", [path, "0", bk, bp, ob, "0x" + pv1[2:].upper(), "Confirm"], "HARNESS: prev with uppercase digits", None, path=path)

    # sign-action, verify-action
    adg = AD("Confirm")
    C.case("sign-action", [G.privhex("buyer"), path, "0", bk, bp, ob, ZERO20, "Confirm"], "s5.5: the buyer signs Confirm", {"digest": hx(adg), "instance": ov["instance"], "ok": True, "sig": G.sign(key("buyer"), adg), "signer": bk}, path=path)
    cdg = AD("Cancel")
    C.case("sign-action", [G.privhex("buyer"), path, "0", bk, bp, ob, ZERO20, "Cancel"], "s5.5: the buyer signs Cancel", {"digest": hx(cdg), "instance": ov["instance"], "ok": True, "sig": G.sign(key("buyer"), cdg), "signer": bk}, path=path)
    ddg = AD("Dispute")
    C.case("sign-action", [G.privhex("buyer"), path, "0", bk, bp, ob, ZERO20, "Dispute"], "s5.5: the buyer signs Dispute, which carries no rung", {"digest": hx(ddg), "instance": ov["instance"], "ok": True, "sig": G.sign(key("buyer"), ddg), "signer": bk}, path=path)
    sdg = AD("Settle", 5000, 1800000000)
    C.case("sign-action", [G.privhex("seller"), path, "0", bk, bp, ob, ZERO20, "Settle", "5000", "1800000000"], "s5.5: the seller signs Settle", {"digest": hx(sdg), "instance": ov["instance"], "ok": True, "sig": G.sign(key("seller"), sdg), "signer": addr("seller")}, path=path)
    C.case("sign-action", [G.privhex("seller"), path, "0", bk, bp, ob, ZERO20, "Confirm"], "sign-action signs by any key; the party rule is a verifier's", {"digest": hx(adg), "instance": ov["instance"], "ok": True, "sig": G.sign(key("seller"), adg), "signer": addr("seller")}, path=path)
    C.case("sign-action", [G.privhex("seller"), path, "0", bk, bp, ob, ZERO20, "ClaimDelivery", ZERO32], "s4.5: the argument test's fault", fault("E_ACTION_MANIFEST"), path=path)
    C.case("sign-action", [G.privhex("seller"), path, "0", bk, bp, ob, ZERO20, "Refund", "x"], "HARNESS: an argument beyond the list", None, path=path)
    C.case("sign-action", [G.privhex("seller"), path, "0", bk, bp, ob, ZERO20], "HARNESS: no action", None, path=path)
    C.case("sign-action", [G.privhex("buyer"), path, "0", bk, bp, ob, pv1, "Confirm"], "sign_action_prev, s5.5, s4.4: the buyer signs a later stage's Confirm, under that stage's instance", {"digest": hx(adgp), "instance": ovp1["instance"], "ok": True, "sig": G.sign(key("buyer"), adgp), "signer": bk}, path=path)
    C.case("sign-action", [G.privhex("buyer"), path, "0", bk, bp, ob, "", "Confirm"], "HARNESS: prev the empty string", None, path=path)

    def V(slug, name, argv_, party, value, note, predicted, bkey=bk, bpay=bp, c="0", openby=ob, prev_=ZERO20):
        p = C.write("sigs", slug, json.dumps(value, ensure_ascii=False).encode("utf-8"))
        C.case("verify-action", [path, c, bkey, bpay, openby, prev_, name] + argv_ + [party, p], note, predicted, path=p)

    V("act_confirm_buyer_ok", "Confirm", [], "buyer", G.sign(key("buyer"), adg), "s4.5/s5.2: Confirm under the buyer", {"form": "secp256k1", "ok": True})
    V("act_cancel_buyer_ok", "Cancel", [], "buyer", G.sign(key("buyer"), cdg), "s4.5/s5.2: Cancel under the buyer", {"form": "secp256k1", "ok": True})
    V("act_dispute_buyer_ok", "Dispute", [], "buyer", G.sign(key("buyer"), ddg), "s4.5/s5.2: Dispute under the buyer", {"form": "secp256k1", "ok": True})
    V("act_claim_seller_ok", "ClaimDelivery", [man], "seller", G.sign(key("seller"), AD("ClaimDelivery", man)), "s4.5/s5.1: ClaimDelivery under the seller", {"form": "secp256k1", "ok": True})
    V("act_refund_seller_ok", "Refund", [], "seller", G.sign(key("seller"), AD("Refund")), "s4.5/s5.1: Refund under the seller", {"form": "secp256k1", "ok": True})
    V("act_settle_seller_ok", "Settle", ["5000", "1800000000"], "seller", G.sign(key("seller"), sdg), "s4.5/s5.1: Settle under the seller", {"form": "secp256k1", "ok": True})
    V("act_settle_buyer_ok", "Settle", ["5000", "1800000000"], "buyer", G.sign(key("buyer"), sdg), "s4.5/s5.2: Settle under the buyer", {"form": "secp256k1", "ok": True})
    V("act_settle_seller_by_call", "Settle", ["5000", "1800000000"], "seller", "", "s5.4: Settle admits the second form under the seller", {"form": "call", "ok": True})
    V("act_settle_buyer_by_call", "Settle", ["5000", "1800000000"], "buyer", "", "s5.4: Settle admits the second form under the buyer", {"form": "call", "ok": True})
    V("act_confirm_by_call", "Confirm", [], "buyer", "", "s5.4: Confirm admits no empty slot", fault("E_SIG_FORM"))
    V("act_cancel_by_call", "Cancel", [], "buyer", "", "s5.4: Cancel admits no empty slot; a contract buyer cancels by calling", fault("E_SIG_FORM"))
    V("act_dispute_by_call", "Dispute", [], "buyer", "", "s5.4: Dispute admits no empty slot", fault("E_SIG_FORM"))
    V("act_claim_by_call", "ClaimDelivery", [man], "seller", "", "s5.4: ClaimDelivery admits no empty slot", fault("E_SIG_FORM"))
    V("act_confirm_prev_ok", "Confirm", [], "buyer", G.sign(key("buyer"), adgp), "s4.5, s4.4: Confirm under the buyer on a later stage, signed under that stage's instance", {"form": "secp256k1", "ok": True}, prev_=pv1)
    V("act_confirm_other_prev", "Confirm", [], "buyer", G.sign(key("buyer"), adg), "s4.5, s4.4: a Confirm signed under the first stage's instance verifies under no other prev", fault("E_SIG_SIGNER"), prev_=pv1)
    V("act_confirm_prev_misuse", "Confirm", [], "buyer", G.sign(key("buyer"), adg), "HARNESS: prev of nineteen bytes", None, prev_=pv1[:-2])
    V("act_refund_by_call", "Refund", [], "seller", "", "s5.4: Refund admits no empty slot", fault("E_SIG_FORM"))
    V("act_confirm_seller_party", "Confirm", [], "seller", G.sign(key("seller"), adg), "s4.5: Confirm does not admit the seller", fault("E_ACTION_PARTY"))
    V("act_cancel_seller_party", "Cancel", [], "seller", G.sign(key("seller"), cdg), "s4.5: Cancel presented as the seller's is E_ACTION_PARTY", fault("E_ACTION_PARTY"))
    V("act_dispute_seller_party", "Dispute", [], "seller", G.sign(key("seller"), ddg), "s4.5: Dispute does not admit the seller", fault("E_ACTION_PARTY"))
    V("act_claim_buyer_party", "ClaimDelivery", [man], "buyer", G.sign(key("buyer"), AD("ClaimDelivery", man)), "s4.5: ClaimDelivery does not admit the buyer", fault("E_ACTION_PARTY"))
    V("act_refund_buyer_party", "Refund", [], "buyer", G.sign(key("buyer"), AD("Refund")), "s4.5: Refund does not admit the buyer", fault("E_ACTION_PARTY"))
    V("act_cancel_seller_by_call_party", "Cancel", [], "seller", "", "s4.5 order: the party rule before the slot's form", fault("E_ACTION_PARTY"))
    V("act_confirm_seller_by_call_party", "Confirm", [], "seller", "", "s4.5 order: the party rule before the slot's form", fault("E_ACTION_PARTY"))
    V("act_claim_zero_buyer_manifest_first", "ClaimDelivery", [ZERO32], "buyer", "", "s4.5 order: the argument test before the party rule", fault("E_ACTION_MANIFEST"))
    V("act_settle_bps_before_party", "Settle", ["10001", "1"], "buyer", "", "s4.5 order: the argument test before the party rule and the slot", fault("E_ACTION_BPS"))
    V("act_confirm_wrong_key", "Confirm", [], "buyer", G.sign(key("stranger"), adg), "s5.2: a stranger's signature", fault("E_SIG_SIGNER"))
    V("act_confirm_other_openby", "Confirm", [], "buyer", G.sign(key("buyer"), adg), "s4.5: another openBy is another instance and another digest", fault("E_SIG_SIGNER"), openby="0")
    V("act_claim_seller_wrong_digest", "ClaimDelivery", [man], "seller", G.sign(key("seller"), AD("ClaimDelivery", H32())), "s5.1: a signature over another manifest", fault("E_SIG_SIGNER"))
    V("act_claim_seller_object", "ClaimDelivery", [man], "seller", {"x": 1}, "s5.1: the seller's form is a string; an object is E_SIG_FORM", fault("E_SIG_FORM"))
    V("act_settle_seller_object_form", "Settle", ["5000", "1800000000"], "seller", {"r": "0x" + "11" * 32}, "s5.3: an object is no signature of the seller; E_SIG_FORM", fault("E_SIG_FORM"))
    V("act_settle_buyer_object_form", "Settle", ["5000", "1800000000"], "buyer", {"r": "0x" + "11" * 32}, "s5.2 and s5.3: an object is no signature of the buyer; E_SIG_FORM", fault("E_SIG_FORM"))
    V("act_confirm_buyer_int", "Confirm", [], "buyer", 7, "s5.2: a value that is no string", fault("E_SIG_FORM"))
    V("act_settle_zero_bytes", "Settle", ["5000", "1800000000"], "seller", "0x", "s5.4: 0x is no second form", fault("E_SIG_FORM"))
    V("act_order_fault_first", "Confirm", [], "buyer", G.sign(key("buyer"), adg), "s4.4 first: E_ORDER_BUYER", fault("E_ORDER_BUYER"), bkey=ZERO20)
    V("act_offer_fault_first", "Confirm", [], "buyer", G.sign(key("buyer"), adg), "s4.3 first: E_OFFER_CANDIDATE", fault("E_OFFER_CANDIDATE"), c="2")
    p_bad = C.write("sigs", "unreadable_sig", b"{")
    C.case("verify-action", [path, "0", bk, bp, ob, ZERO20, "Confirm", "buyer", p_bad], "HARNESS: a sig.json the reader refuses is misuse", None, path=p_bad)
    C.case("verify-action", [path, "0", bk, bp, ob, ZERO20, "Confirm", "owner", p_], "HARNESS: party is buyer or seller", None, path=p_)
    C.case("verify-action", [path, "0", bk, bp, ob, ZERO20, "Dispute", "1", "buyer", p_], "HARNESS: Dispute takes no argument, so the token sequence is one too long", None, path=p_)
    C.case("verify-action", [path, "0", bk, bp, ob, ZERO20, "Confirm", p_], "HARNESS: an argument short", None, path=p_)
    C.case("verify-order", [path, "0", bk, bp, ob, ZERO20, "sigs/absent.json"], "HARNESS: a path the command cannot read", None)
    C.case("verify-order", [path, "0", bk, bp, p_, ZERO20], "HARNESS: an argument short (no openBy)", None, path=p_)
    C.case("verify-order", [path, "0", bk, bp, ob, ZERO20, p_, "x"], "HARNESS: an argument beyond the list", None, path=p_)
    C.argv_case([], "HARNESS: no command", None)
    C.argv_case(["bogus"], "HARNESS: an unknown command", None)


def gen_misuse():
    """HARNESS: an unreadable path on every command that reads one, and an argument beyond the list on the four commands
    that had no such case; the other arguments are well formed so that the path alone is the boundary."""
    bk, bp, ob = addr("buyer"), addr("buyer-payout"), "1800000000"
    good, absent = "docs/d001_ok_full.md", "docs/absent.md"
    sig = "0x" + "11" * 64 + "1b"
    priv = G.privhex("buyer")
    for cmd, argv in (("offer", [absent, "0"]), ("order", [absent, "0", bk, bp, ob]), ("action", [absent, "0", bk, bp, ob, "Confirm"]),
                      ("sign-offer", [priv, absent, "0"]), ("sign-order", [priv, absent, "0", bp, ob]),
                      ("sign-action", [priv, absent, "0", bk, bp, ob, "Confirm"]),
                      ("verify-offer", [absent, "0", sig]),
                      ("verify-action", [absent, "0", bk, bp, ob, "Confirm", "buyer", "sigs/absent.json"])):
        C.case(cmd, argv, "HARNESS: a path the command cannot read", None)
    qd = G.quote_bytes(addr("token"))
    C.argv_case(["fee", qd, "5", "x"], "HARNESS: an argument beyond the list", None, path="fee:misuse_beyond")
    C.case("sign-offer", [priv, good, "0", "x"], "HARNESS: an argument beyond the list", None, path=good)
    C.case("sign-order", [priv, good, "0", bp, ob, ZERO20, "x"], "HARNESS: an argument beyond the list", None, path=good)
    # misuse answers first, before the document is decided: the document here fails s2.4
    C.case("offer", [NO_BLOCK, "01"], "HARNESS before s2.4: c with a leading zero on a document that has no block", None, path=NO_BLOCK)
    C.case("verify-order", [NO_BLOCK, "0", bk, bp, ob, ZERO20, "sigs/absent.json"], "HARNESS before s2.4: an unreadable sig path on a document that has no block", None, path=NO_BLOCK)
    psig = C.write("sigs", "misuse_before_decision", json.dumps(sig).encode("utf-8"))
    C.case("verify-action", [NO_BLOCK, "0", bk, bp, ob, ZERO20, "Confirm", "judge", psig], "HARNESS before s2.4: a party outside buyer/seller on a document that has no block", None, path=psig)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=1)
    a = ap.parse_args()
    G.seed(a.seed)
    global C
    C = G.Corpus(HERE, ["docs", "sigs"])
    gen_check()
    gen_quote()
    gen_derivations()
    gen_signatures()
    gen_misuse()
    n = C.finish(a.seed)
    print("terms corpus: %d cases" % n)


if __name__ == "__main__":
    main()
