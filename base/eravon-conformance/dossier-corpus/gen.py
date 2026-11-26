#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
gen.py - corpus generator for the `eravon.dossier/1` convergence run.

Written from docs/eravon-dossier-v1.md, the two document laws and
HARNESS-DOSSIER.md.  Every prediction is written by hand from the texts (or
`null` where the case is harness misuse); the digests an accepted answer
carries are computed by ../gencommon.py's own code, a third opinion beside
the criterion and the candidates, never authoritative.

Usage:  python3 gen.py --seed 1
Populates ./dossiers (one directory per case) and writes manifest.json.
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

C = None
OB = G.OPEN_BY


def sha(b):
    return hx(hashlib.sha256(b).digest())


def up(h):
    return h.upper().replace("0X", "0x")


def copy(d):
    return json.loads(json.dumps(d))


def main_case():
    tok = addr("token")
    gj = {"judge": addr("judge"), "amanahHash": H32(), "quoteHash": G.qhash(G.quote_bytes(tok))}
    gg = {"pointer": "galeed:guild:1:" + addr("guild"), "amanahHash": H32(), "quoteHash": G.qhash(G.quote_bytes(tok, autoAccept=0))}
    blk = G.base_block(judges=[gj], guilds=[gg])
    tb = G.terms_doc(cbytes(blk))
    listing = G.listing_of(blk, tb)
    r, ov = G.receipt_of(listing, blk, 0)
    rg, ovg = G.receipt_of(listing, blk, 1, buyer_tag="buyer2")
    return blk, tb, listing, r, ov, rg, ovg


def judgment(judge_tag, seq, prev, chain_id, instance, codehash, case_id, amanah, rung=2, seat=0):
    body = {"subjectRef": {"kind": "evm", "chainId": chain_id, "contract": instance, "codeHash": codehash, "id": case_id, "seat": seat},
            "amanah": amanah, "verdict_md": "全额退款。", "decision": {"rung": rung},
            "evidence": [{"manifest": H32(), "produced": True}]}
    return G.entry(judge_tag, seq, prev, "judgment", body)


def resignation(judge_tag, seq, prev, chain_id, instance, codehash, case_id, amanah, seat=0, with_seat=True):
    sr = {"kind": "evm", "chainId": chain_id, "contract": instance, "codeHash": codehash, "id": case_id}
    if with_seat:
        sr["seat"] = seat
    body = {"subjectRef": sr, "amanah": amanah, "category": "other", "statement_md": "辞任。"}
    return G.entry(judge_tag, seq, prev, "resignation", body)


ALL_KINDS = ("post", "fund", "deploy", "init", "accept", "delivery", "confirm", "dispute", "commit", "settle", "rule",
             "resign", "refund", "expire", "cancel", "revoke", "advance", "redraw", "setPayout", "withdraw", "sweep", "consent",
             "fundBind")


def txrows(n=4):
    kinds = ["post", "fund", "deploy", "init", "accept", "delivery", "dispute", "commit", "rule", "setPayout", "withdraw",
             "sweep", "advance", "redraw", "expire", "refund", "resign", "settle", "confirm", "cancel", "revoke", "consent",
             "fundBind"]
    return [{"kind": kinds[i % len(kinds)], "tx": H32(), "block": 20000000 + i, "timestamp": 1750000000 + 12 * i} for i in range(n)]


def posted_of(listing_bytes, tx=None):
    return {"tx": tx or H32(), "hash": sha(listing_bytes)}


def index_of(listing, lb, ov, r, files, entry_id=None, terminal=None, compliance=None, seat=None, posted=None, **over):
    c = r["candidate"]
    if c < len(listing["judges"]):
        cand = listing["judges"][c]
        seat_d = {"judge": cand["judge"], "amanahHash": cand["amanahHash"], "quoteHash": cand["quoteHash"], "seat": 0}
    else:
        cand = listing["guilds"][c - len(listing["judges"])]
        seat_d = {"judge": addr("drawn"), "amanahHash": cand["amanahHash"], "quoteHash": cand["quoteHash"], "seat": 1}
    if seat:
        seat_d.update(seat)
    if entry_id is not None:
        seat_d["entryId"] = entry_id
    d = {"v": "eravon.dossier/1", "chainId": listing["chainId"], "instance": ov["instance"], "caseId": ov["salt"],
         "logic": listing["logic"], "proxyCodeHash": listing["proxyCodeHash"], "posted": posted or posted_of(lb),
         "files": files, "seat": seat_d, "transactions": txrows()}
    if terminal is not None:
        d["terminal"] = terminal
    if compliance is not None:
        d["compliance"] = compliance
    d.update(over)
    return d


def write_dossier(slug, index_bytes, files):
    """files: {name: bytes}; index bytes may be None to omit dossier.json."""
    p = C.dir("dossiers", slug)
    if index_bytes is not None:
        with open(os.path.join(HERE, p, "dossier.json"), "wb") as f:
            f.write(index_bytes)
    for name, b in files.items():
        os.makedirs(os.path.dirname(os.path.join(HERE, p, name)), exist_ok=True)
        if isinstance(b, tuple) and b[0] == "link":
            os.symlink(b[1], os.path.join(HERE, p, name))
            continue
        if isinstance(b, tuple) and b[0] == "dir":
            os.makedirs(os.path.join(HERE, p, name), exist_ok=True)
            with open(os.path.join(HERE, p, name, "keep"), "wb") as f:  # a file inside, so that git carries the directory
                f.write(b"")
            continue
        with open(os.path.join(HERE, p, name), "wb") as f:
            f.write(b)
    return p


def D(slug, index, files, note, predicted):
    ib = cbytes(index) if isinstance(index, dict) else index
    p = write_dossier(slug, ib, files)
    C.case("check", [p], note, predicted, path=p)
    return p


def refs(files, attachments=None):
    out = {k: {"name": n, "sha256": sha(b)} for k, (n, b) in files.items()}
    if attachments is not None:
        out["attachments"] = [{"name": n, "sha256": sha(b)} for n, b in attachments]
    return out


def okd(index, r, listing, entry_id=None, buyer="secp256k1", seller="secp256k1", n=0):
    return {"attachments": n, "buyerConsent": buyer, "candidate": r["candidate"], "caseId": index["caseId"], "entryId": entry_id,
            "instance": index["instance"], "ok": True, "sellerConsent": seller, "termsHash": listing["terms"]["hash"]}


def gen():
    blk, tb, listing, r, ov, rg, ovg = main_case()
    lb = cbytes(listing)
    rb = cbytes(r)
    rgb = cbytes(rg)
    ch = listing["proxyCodeHash"]
    amanah = listing["judges"][0]["amanahHash"]
    ej = judgment("judge", 3, H32(), 1, ov["instance"], ch, ov["salt"], amanah)
    eid = sha(ej)
    base_files = {"listing": ("listing.json", lb), "receipt": ("receipt.json", rb)}
    fs = lambda extra=None: {**{n: b for n, b in base_files.values()}, **(extra or {})}  # noqa: E731
    term = {"name": "Ruled", "buyerPaid": "1000000000", "sellerPaid": "0", "judgePaid": "160000000", "bondToBuyer": "0"}

    ix = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", ej)}), entry_id=eid, terminal=term,
                  compliance={"buyer_md": "买方:某公司", "seller_md": "Studio Ltd", "serial": "2026-0001",
                              "conversions": [{"tx": H32(), "at": 1750000000, "unit": "USD", "rate": "1.0003", "source": "https://rate.example"}]})
    D("ok_ruled", ix, fs({"entry.bin": ej}), "s3.1: a whole dossier of a ruled case with an entry and a compliance page", okd(ix, r, listing, eid))
    ix = index_of(listing, lb, ov, r, refs(base_files))
    D("ok_open", ix, fs(), "s3.1: an open case, no terminal, no entry", okd(ix, r, listing))
    term2 = {"name": "Released", "buyerPaid": "0", "sellerPaid": "990000000", "judgePaid": "10000000", "bondToBuyer": "0"}
    ix = index_of(listing, lb, ov, r, refs(base_files), terminal=term2, transactions=[])
    D("ok_released_no_rows", ix, fs(), "s2.3: transactions may be empty; a Released terminal needs no entry", okd(ix, r, listing))
    er = resignation("judge", 4, H32(), 1, ov["instance"], ch, ov["salt"], amanah)
    eidr = sha(er)
    term3 = {"name": "Resigned", "buyerPaid": "1000000000", "sellerPaid": "0", "judgePaid": "0", "bondToBuyer": "0"}
    ix = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("resign.bin", er)}), entry_id=eidr, terminal=term3)
    D("ok_resigned_entry", ix, fs({"resign.bin": er}), "s3.1: a resignation entry under another file name", okd(ix, r, listing, eidr))
    ern = resignation("judge", 4, H32(), 1, ov["instance"], ch, ov["salt"], amanah, with_seat=False)
    ix = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("resign.bin", ern)}), entry_id=sha(ern), terminal=term3)
    D("ok_resignation_no_seat", ix, fs({"resign.bin": ern}), "s3.1: a resignation's subjectRef may omit seat", okd(ix, r, listing, sha(ern)))
    ix = index_of(listing, lb, ov, r, refs(base_files), terminal=term3)
    D("ok_resigned_no_entry", ix, fs(), "s2.4: Resigned needs no entry (a pool exhaustion has none)", okd(ix, r, listing))
    term4 = {"name": "Lapsed", "buyerPaid": "1000000000", "sellerPaid": "160000000", "judgePaid": "0", "bondToBuyer": "0"}
    ix = index_of(listing, lb, ov, r, refs(base_files), terminal=term4)
    D("ok_lapsed", ix, fs(), "s2.4: a Lapsed terminal, the deposit back to the seller", okd(ix, r, listing))
    gfiles = {"listing": ("listing.json", lb), "receipt": ("receipt.json", rgb)}
    eg = judgment("drawn", 1, H32(), 1, ovg["instance"], ch, ovg["salt"], listing["guilds"][0]["amanahHash"], seat=1)
    ix = index_of(listing, lb, ovg, rg, refs({**gfiles, "entry": ("entry.bin", eg)}), entry_id=sha(eg), terminal=term)
    D("ok_guild_drawn", ix, {"listing.json": lb, "receipt.json": rgb, "entry.bin": eg}, "s3.1: a pool case: the seat's judge is whoever was drawn, seat 1", okd(ix, rg, listing, sha(eg)))
    ix = index_of(listing, lb, ovg, rg, refs(gfiles), seat={"seat": 2, "judge": addr("other-drawn")})
    D("ok_guild_seat_2", ix, {"listing.json": lb, "receipt.json": rgb}, "s2.2: a pool seat may be 2", okd(ix, rg, listing))
    ix = index_of(listing, lb, ov, r, refs(base_files), compliance={})
    D("ok_compliance_empty", ix, fs(), "s2.5: every compliance member is optional", okd(ix, r, listing))
    rc = dict(r, sig="")
    rcb = cbytes(rc)
    cfiles = {"listing": ("listing.json", lb), "receipt": ("receipt.json", rcb)}
    ix = index_of(listing, lb, ov, rc, refs(cfiles))
    D("ok_buyer_by_call", ix, {"listing.json": lb, "receipt.json": rcb}, "s3.1: a by-call receipt binds RECEIPT_BY_CALL; buyerConsent is call", okd(ix, rc, listing, buyer="call"))
    # a later stage (terms s4.4, s5.3): the receipt's prev the instance before; instance and caseId under it
    rl, ovl = G.receipt_of(listing, blk, 0, prev=addr("stage-before"))
    rlb = cbytes(rl)
    lfiles = {"listing": ("listing.json", lb), "receipt": ("receipt.json", rlb)}
    ixl = index_of(listing, lb, ovl, rl, refs(lfiles))
    D("ok_later_stage", ixl, {"listing.json": lb, "receipt.json": rlb}, "s3.1, link s4.4: a later stage's dossier under a named judge, the receipt's prev the instance before", okd(ixl, rl, listing))
    rlg, ovlg = G.receipt_of(listing, blk, 1, buyer_tag="buyer2", prev=addr("stage-before"))
    rlgb = cbytes(rlg)
    ixlg = index_of(listing, lb, ovlg, rlg, refs({"listing": ("listing.json", lb), "receipt": ("receipt.json", rlgb)}))
    D("ok_later_stage_pool", ixlg, {"listing.json": lb, "receipt.json": rlgb}, "s3.1, link s4.4: a later stage's dossier under a guild's pool", okd(ixlg, rlg, listing))
    rlc = dict(rl, sig="")
    rlcb = cbytes(rlc)
    ixlc = index_of(listing, lb, ovl, rlc, refs({"listing": ("listing.json", lb), "receipt": ("receipt.json", rlcb)}))
    D("ok_later_stage_by_call", ixlc, {"listing.json": lb, "receipt.json": rlcb}, "s3.1, link s4.4: a later stage's dossier, the buyer consenting by call", okd(ixlc, rlc, listing, buyer="call"))
    D("later_stage_instance_of_first", dict(ixl, instance=ov["instance"]), {"listing.json": lb, "receipt.json": rlb}, "s3.1: the instance test reads the instance of the receipt's order, prev among its members, not the first stage's", fault("E_DOSSIER_INSTANCE_MISMATCH"))
    D("later_stage_case_of_first", dict(ixl, caseId=ov["salt"]), {"listing.json": lb, "receipt.json": rlb}, "s3.1: the case test reads the salt the receipt's prev derives", fault("E_DOSSIER_CASE_MISMATCH"))
    rlu = dict(rl, prev=addr("another-stage"))
    rlub = cbytes(rlu)
    ixlu = index_of(listing, lb, ovl, rlu, refs({"listing": ("listing.json", lb), "receipt": ("receipt.json", rlub)}))
    D("later_stage_prev_unsigned", ixlu, {"listing.json": lb, "receipt.json": rlub}, "s3.1: a receipt whose prev differs from the signed order's: E_DOSSIER_ORDER carrying RECEIPT_SIG", fault("E_DOSSIER_ORDER", inner=inner("RECEIPT_SIG", inner_=inner("E_SIG_SIGNER"))))
    rlf = dict(rl, prev="0x12")
    rlfb = cbytes(rlf)
    ixlf = index_of(listing, lb, ovl, rlf, refs({"listing": ("listing.json", lb), "receipt": ("receipt.json", rlfb)}))
    D("later_stage_prev_form", ixlf, {"listing.json": lb, "receipt.json": rlfb}, "s3.1: E_DOSSIER_RECEIPT carrying E_RECEIPT_PREV", fault("E_DOSSIER_RECEIPT", inner=inner("E_RECEIPT_PREV")))
    rla = dict(rl, prev="0x12", sig="0x")
    rlab = cbytes(rla)
    ixla = index_of(listing, lb, ovl, rla, refs({"listing": ("listing.json", lb), "receipt": ("receipt.json", rlab)}))
    D("later_stage_adj_prev_before_sig", ixla, {"listing.json": lb, "receipt.json": rlab}, "s3.1, link s4.2 order: prev before sig inside E_DOSSIER_RECEIPT", fault("E_DOSSIER_RECEIPT", inner=inner("E_RECEIPT_PREV")))
    lis_c = copy(listing); lis_c["judges"][0]["sig"] = ""
    lcb = cbytes(lis_c)
    sfiles = {"listing": ("listing.json", lcb), "receipt": ("receipt.json", rb)}
    ix = index_of(lis_c, lcb, ov, r, refs(sfiles))
    D("ok_seller_by_call", ix, {"listing.json": lcb, "receipt.json": rb}, "s3.1: the candidate the receipt names is by call; sellerConsent is call", okd(ix, r, listing, seller="call"))
    bfiles = {"listing": ("listing.json", lcb), "receipt": ("receipt.json", rcb)}
    ix = index_of(lis_c, lcb, ov, rc, refs(bfiles))
    D("ok_both_by_call", ix, {"listing.json": lcb, "receipt.json": rcb}, "s3.1: two companies; both forms are call", okd(ix, rc, listing, buyer="call", seller="call"))
    lis_g = copy(listing); lis_g["guilds"][0]["sig"] = ""
    lgb = cbytes(lis_g)
    gfiles2 = {"listing": ("listing.json", lgb), "receipt": ("receipt.json", rgb)}
    ix = index_of(lis_g, lgb, ovg, rg, refs(gfiles2))
    D("ok_guild_seller_by_call", ix, {"listing.json": lgb, "receipt.json": rgb}, "s3.1: a pool candidate by call; the seller's form follows the candidate the receipt names", okd(ix, rg, listing, seller="call"))
    jfiles = {"listing": ("listing.json", lcb), "receipt": ("receipt.json", rgb)}
    ix = index_of(lis_c, lcb, ovg, rg, refs(jfiles))
    D("ok_other_candidate_by_call", ix, {"listing.json": lcb, "receipt.json": rgb}, "s3.1: a by-call candidate the receipt does not name leaves sellerConsent secp256k1", okd(ix, rg, listing))
    rows = txrows(3); rows[0]["kind"] = "consent"; rows[0]["note_md"] = "the buyer's registry entry"
    ix2 = dict(index_of(listing, lb, ov, rc, refs(cfiles)), transactions=rows)
    D("ok_transaction_consent", ix2, {"listing.json": lb, "receipt.json": rcb}, "s2.3: consent is a kind", okd(ix2, rc, listing, buyer="call"))
    ixk = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", ej)}), entry_id=eid, terminal=term,
                   transactions=[{"kind": k, "tx": H32(), "block": 20000000 + i, "timestamp": 1750000000 + 12 * i} for i, k in enumerate(sorted(ALL_KINDS))])
    D("ok_every_kind", ixk, fs({"entry.bin": ej}), "s2.3: one row of each of the twenty-three kinds", okd(ixk, r, listing, eid))
    # a company seller: the document's sellerFund is the company's wallet; a fundBind row before the init's
    blkf = copy(blk); blkf["sellerFund"] = addr("company-wallet")
    tbf = G.terms_doc(cbytes(blkf))
    lisf = G.listing_of(blkf, tbf)
    lfb = cbytes(lisf)
    rf, ovf = G.receipt_of(lisf, blkf, 0)
    rfb = cbytes(rf)
    rows = txrows(4)
    rows[0]["kind"] = "fundBind"; rows[0]["note_md"] = "the wallet's bond to the seller"
    rows[1]["kind"] = "fund"; rows[2]["kind"] = "deploy"; rows[3]["kind"] = "init"
    ixf = index_of(lisf, lfb, ovf, rf, refs({"listing": ("listing.json", lfb), "receipt": ("receipt.json", rfb)}), transactions=rows)
    D("ok_company_fund", ixf, {"listing.json": lfb, "receipt.json": rfb}, "s2.3, s5.1: a company seller, sellerFund the wallet; a fundBind row, then fund, deploy, init", okd(ixf, rf, lisf))
    rows = txrows(3)
    rows[0]["kind"] = "fund"; rows[1]["kind"] = "deploy"; rows[2]["kind"] = "init"; rows[2]["tx"] = rows[1]["tx"]
    ixf2 = index_of(lisf, lfb, ovf, rf, refs({"listing": ("listing.json", lfb), "receipt": ("receipt.json", rfb)}), transactions=rows)
    D("ok_company_no_bind_row", ixf2, {"listing.json": lfb, "receipt.json": rfb}, "s6: a company seller's dossier owes no fundBind row; deploy and init in one transaction", okd(ixf2, rf, lisf))
    rows = [{"kind": "post", "tx": ZERO32, "block": 0, "timestamp": 0, "note_md": ""}]
    ixz = index_of(listing, lb, ov, r, refs(base_files), transactions=rows,
                   compliance={"buyer_md": "", "seller_md": "", "conversions": [{"tx": ZERO32, "at": 0, "unit": "USD", "rate": "1", "source": "s"}]})
    D("ok_zero_edges", ixz, fs(), "s2.3, s2.5: a row's tx zero, block 0, timestamp 0, note_md empty; buyer_md and seller_md empty; a conversion's tx zero and at 0: each form admits its zero", okd(ixz, r, listing))
    ixc = index_of(listing, lb, ov, rc, refs(cfiles))
    D("by_call_instance_mismatch", dict(ixc, instance=addr("x")), {"listing.json": lb, "receipt.json": rcb}, "s3.1: the instance test reads the value RECEIPT_BY_CALL yields", fault("E_DOSSIER_INSTANCE_MISMATCH"))
    D("by_call_case_mismatch", dict(ixc, caseId=H32()), {"listing.json": lb, "receipt.json": rcb}, "s3.1: the case test reads the salt RECEIPT_BY_CALL yields", fault("E_DOSSIER_CASE_MISMATCH"))
    odd = {"listing": ("a b~.json", lb), "receipt": ("r", rb)}
    ix = index_of(listing, lb, ov, r, refs(odd))
    D("ok_file_names", ix, {"a b~.json": lb, "r": rb}, "s1: file names are skeleton strings without /", okd(ix, r, listing))
    dots = {"listing": ("...", lb), "receipt": (".listing", rb)}
    ix = index_of(listing, lb, ov, r, refs(dots))
    D("ok_file_name_dots", ix, {"...": lb, ".listing": rb}, "s1: only . and .. are refused; ... and a leading dot are file names", okd(ix, r, listing))
    # attachments: a document with hashed segments
    blk_h = G.base_block(judges=[blk["judges"][0]], goods=..., goodsHash=G.text_hash(G.GOODS), delivery_text=..., deliveryHash=G.text_hash(G.DELIVERY))
    tb_h = G.terms_doc(cbytes(blk_h))
    lis_h = G.listing_of(blk_h, tb_h)
    lhb = cbytes(lis_h)
    r_h, ov_h = G.receipt_of(lis_h, blk_h, 0)
    rhb = cbytes(r_h)
    hfiles = {"listing": ("listing.json", lhb), "receipt": ("receipt.json", rhb)}
    gt = G.GOODS.encode("utf-8")
    dt = G.DELIVERY.encode("utf-8")
    ix = index_of(lis_h, lhb, ov_h, r_h, refs(hfiles, [("goods.md", gt)]))
    D("ok_attachment", ix, {"listing.json": lhb, "receipt.json": rhb, "goods.md": gt}, "s3.1: one attachment, the goods text the document names", okd(ix, r_h, lis_h, n=1))
    ix = index_of(lis_h, lhb, ov_h, r_h, refs(hfiles, [("goods.md", gt), ("delivery.md", dt)]))
    D("ok_attachments_two", ix, {"listing.json": lhb, "receipt.json": rhb, "goods.md": gt, "delivery.md": dt}, "s3.1: both segments' texts", okd(ix, r_h, lis_h, n=2))
    ix = index_of(lis_h, lhb, ov_h, r_h, refs(hfiles, []))
    D("ok_attachments_empty", ix, {"listing.json": lhb, "receipt.json": rhb}, "s2.1: attachments may be an empty array", okd(ix, r_h, lis_h))
    ix = index_of(lis_h, lhb, ov_h, r_h, refs(hfiles, [("goods.md", gt), ("goods.md", gt)]))
    D("ok_attachment_twice", ix, {"listing.json": lhb, "receipt.json": rhb, "goods.md": gt}, "s1: two references may name one file", okd(ix, r_h, lis_h, n=2))
    ix = index_of(lis_h, lhb, ov_h, r_h, refs(hfiles, [("goods.md", gt)]))
    D("attachment_file_absent", ix, {"listing.json": lhb, "receipt.json": rhb}, "s3.1: E_DOSSIER_FILE with the attachment's key", fault("E_DOSSIER_FILE", key="attachment:0"))
    D("attachment_digest", ix, {"listing.json": lhb, "receipt.json": rhb, "goods.md": gt + b" "}, "s3.1: E_DOSSIER_DIGEST with the attachment's key", fault("E_DOSSIER_DIGEST", key="attachment:0"))
    ix = index_of(lis_h, lhb, ov_h, r_h, refs(hfiles, [("goods.md", gt), ("other.md", b"other")]))
    D("attachment_second_absent", ix, {"listing.json": lhb, "receipt.json": rhb, "goods.md": gt}, "s3.1: the second attachment's key", fault("E_DOSSIER_FILE", key="attachment:1"))
    ix = index_of(lis_h, lhb, ov_h, r_h, refs(hfiles, [("other.md", b"a text the document does not name")]))
    D("attachment_unnamed", ix, {"listing.json": lhb, "receipt.json": rhb, "other.md": b"a text the document does not name"}, "s3.1: E_DOSSIER_ATTACHMENT, a hash the document does not carry", fault("E_DOSSIER_ATTACHMENT", 0))
    ix = index_of(lis_h, lhb, ov_h, r_h, refs(hfiles, [("goods.md", gt), ("other.md", b"x")]))
    D("attachment_unnamed_second", ix, {"listing.json": lhb, "receipt.json": rhb, "goods.md": gt, "other.md": b"x"}, "s3.1: the first unnamed attachment's index", fault("E_DOSSIER_ATTACHMENT", 1))
    ix = index_of(listing, lb, ov, r, refs(base_files, [("goods.md", gt)]))
    D("attachment_unnamed_clear_document", ix, fs({"goods.md": gt}), "s3.1: a document that carries the text in the clear names no hash", fault("E_DOSSIER_ATTACHMENT", 0))
    note_t = b"a note"
    blk_p = G.base_block(judges=[dict(blk["judges"][0], amanahHash=G.text_hash("covenant text"))], goods=G.GOODS + " " + sha(note_t))
    tb_p = G.terms_doc(cbytes(blk_p))
    lis_p = G.listing_of(blk_p, tb_p)
    lpb = cbytes(lis_p)
    r_p, ov_p = G.receipt_of(lis_p, blk_p, 0)
    rpb = cbytes(r_p)
    pfiles = {"listing": ("listing.json", lpb), "receipt": ("receipt.json", rpb)}
    ix = index_of(lis_p, lpb, ov_p, r_p, refs(pfiles, [("note.md", note_t)]))
    D("attachment_prose_digest", ix, {"listing.json": lpb, "receipt.json": rpb, "note.md": note_t}, "s3.1: a text whose digest the prose spells is named by no goodsHash or deliveryHash", fault("E_DOSSIER_ATTACHMENT", 0))
    cov_t = "covenant text".encode("utf-8")
    ix = index_of(lis_p, lpb, ov_p, r_p, refs(pfiles, [("covenant.md", cov_t)]))
    D("attachment_covenant_text", ix, {"listing.json": lpb, "receipt.json": rpb, "covenant.md": cov_t}, "s3.1: a text hashing to a candidate's amanahHash is no attachment", fault("E_DOSSIER_ATTACHMENT", 0))
    tt = lis_h["terms"]["inline_md"].encode("utf-8")
    ixt = index_of(lis_h, lhb, ov_h, r_h, refs(hfiles, [("terms.md", tt)]))
    D("attachment_terms_text", ixt, {"listing.json": lhb, "receipt.json": rhb, "terms.md": tt}, "s3.1: the terms text hashes to terms.hash, which is neither goodsHash nor deliveryHash", fault("E_DOSSIER_ATTACHMENT", 0))
    ixe = index_of(lis_h, lhb, ov_h, r_h, refs({**hfiles, "entry": ("entry.bin", ej)}, [("other.md", b"x")]), entry_id=eid)
    D("attachment_after_entry", ixe, {"listing.json": lhb, "receipt.json": rhb, "other.md": b"x", "entry.bin": ej}, "s3.1 order: the entry's subject before the attachments", fault("E_DOSSIER_SUBJECT"))

    # ---- the index, s2.6
    D("no_index", None, fs(), "HARNESS: dossier.json absent", fault("E_DOSSIER_FILE", key="index"))
    D("index_dir", None, fs({"dossier.json": ("dir", None)}), "s1: an entry that is not a regular file is unreadable", fault("E_DOSSIER_FILE", key="index"))
    D("index_json", b"{", fs(), "galeed/1 s3.5 test 2", fault("E_JSON"))
    ix = index_of(listing, lb, ov, r, refs(base_files))
    D("index_pretty", json.dumps(ix, indent=1, sort_keys=True, ensure_ascii=False).encode("utf-8"), fs(), "galeed/1 s3.5 test 6", fault("E_NOT_CANONICAL"))
    D("canon_root_pretty", b"[ ]", fs(), "galeed/1 s3 order: canonical before the root (an index that is neither)", fault("E_NOT_CANONICAL"))
    D("root_array", b"[]", fs(), "s2.6: root an object", fault("E_DOSSIER"))
    d = dict(ix); del d["seat"]
    D("missing", d, fs(), "s2.6: a required member absent", fault("E_DOSSIER_MISSING"))
    d = dict(ix); del d["posted"]
    D("missing_posted", d, fs(), "s2.6: posted is required", fault("E_DOSSIER_MISSING"))
    for key_, slug_ in (("v", "missing_v"), ("chainId", "missing_chain"), ("instance", "missing_instance"), ("caseId", "missing_case"),
                        ("logic", "missing_logic"), ("proxyCodeHash", "missing_codehash"), ("files", "missing_files")):
        d = dict(ix); del d[key_]
        D(slug_, d, fs(), "s2.6: %s is required" % key_, fault("E_DOSSIER_MISSING"))
    # s7.1: every required member of every table of s2 absent alone, and every table with a member beyond its list
    for m in ("tx", "hash"):
        p_ = dict(ix["posted"]); del p_[m]
        D("posted_missing_" + m, dict(ix, posted=p_), fs(), "s2.1: posted's %s required" % m, fault("E_DOSSIER_POSTED"))
    for m in ("name", "sha256"):
        f = refs(base_files); del f["listing"][m]
        D("files_ref_missing_" + m, dict(ix, files=f), fs(), "s2.1: a file reference's %s required" % m, fault("E_DOSSIER_FILES"))
    f = refs(base_files); f["listing"]["x"] = 1
    D("files_ref_extra", dict(ix, files=f), fs(), "s2.1: a file reference closed", fault("E_DOSSIER_FILES"))
    for m in ("name", "sha256"):
        f = refs(base_files, [("goods.md", b"g")]); del f["attachments"][0][m]
        D("files_attachment_missing_" + m, dict(ix, files=f), fs({"goods.md": b"g"}), "s2.1: an attachment reference's %s required" % m, fault("E_DOSSIER_FILES"))
    f = refs(base_files, [("goods.md", b"g")]); f["attachments"][0]["x"] = 1
    D("files_attachment_extra", dict(ix, files=f), fs({"goods.md": b"g"}), "s2.1: an attachment reference closed", fault("E_DOSSIER_FILES"))
    for m in ("amanahHash", "seat"):
        s_ = dict(ix["seat"]); del s_[m]
        D("seat_missing_" + m.lower(), dict(ix, seat=s_), fs(), "s2.2: %s required" % m, fault("E_DOSSIER_SEAT"))
    for m in ("kind", "block", "timestamp"):
        rows = txrows(1); del rows[0][m]
        D("transaction_missing_" + m, dict(ix, transactions=rows), fs(), "s2.3: %s required, index 0" % m, fault("E_TRANSACTION", 0))
    for m in ("name", "buyerPaid", "sellerPaid", "judgePaid", "bondToBuyer"):
        t_ = dict(term2); del t_[m]
        D("terminal_missing_" + m.lower(), dict(ix, terminal=t_), fs(), "s2.4: %s required" % m, fault("E_DOSSIER_TERMINAL"))
    convm = {"tx": H32(), "at": 1, "unit": "USD", "rate": "1", "source": "x"}
    for m in ("tx", "at", "unit", "rate", "source"):
        c_ = dict(convm); del c_[m]
        D("compliance_conversion_missing_" + m, dict(ix, compliance={"conversions": [c_]}), fs(), "s2.5: a conversion's %s required, index 0" % m, fault("E_DOSSIER_COMPLIANCE", 0))
    d = dict(ix); d["x"] = 1
    D("closed", d, fs(), "s2.6: a member beyond the twelve", fault("E_DOSSIER_CLOSED"))
    d = dict(ix); d["zz_md"] = "é"
    D("closed_md_key", d, fs(), "s1: an unlisted _md key is prose under galeed/1 s3.3, then a member beyond the list", fault("E_DOSSIER_CLOSED"))
    D("index_utf8", b"\xff" + cbytes(ix), fs(), "galeed/1 s3.5 test 1: E_UTF8", fault("E_UTF8"))
    D("index_depth", cbytes(ix)[:-1] + b',"x":' + b"[" * 128 + b"]" * 128 + b"}", fs(), "galeed/1 s3.5 test 2: E_DEPTH", fault("E_DEPTH"))
    D("index_number", cbytes(ix).replace(b'"chainId":1', b'"chainId":1.0'), fs(), "galeed/1 s3.5 test 2: E_NUMBER", fault("E_NUMBER"))
    D("index_dup_key", cbytes(ix).replace(b'"chainId":1', b'"chainId":1,"chainId":1'), fs(), "galeed/1 s3.5 test 3: E_DUP_KEY", fault("E_DUP_KEY"))
    D("index_key_charset", cbytes(dict(ix, compliance={"é": 1})), fs(), "galeed/1 s3.5 test 4: a key outside the charset; compliance is no prose subtree, and every key of the index is a key", fault("E_KEY_CHARSET"))
    D("index_value_charset", dict(ix, v="é"), fs(), "galeed/1 s3.3: a skeleton string outside a prose subtree", fault("E_VALUE_CHARSET"))
    D("v", dict(ix, v="x"), fs(), "s2.6: v", fault("E_DOSSIER_V"))
    D("chain", dict(ix, chainId="1"), fs(), "s2.6: chainId an int", fault("E_DOSSIER_CHAIN"))
    D("instance", dict(ix, instance="0x1"), fs(), "s2.6: instance hex20", fault("E_DOSSIER_INSTANCE"))
    D("case", dict(ix, caseId=1), fs(), "s2.6: caseId hex32", fault("E_DOSSIER_CASE"))
    D("logic", dict(ix, logic=None), fs(), "s2.6: logic hex20", fault("E_DOSSIER_LOGIC"))
    D("codehash", dict(ix, proxyCodeHash="0x"), fs(), "s2.6: proxyCodeHash hex32", fault("E_DOSSIER_CODEHASH"))
    D("v_case", dict(ix, v="ERAVON.DOSSIER/1"), fs(), "s2.6: v byte for byte", fault("E_DOSSIER_V"))
    D("chain_bool", dict(ix, chainId=True), fs(), "s2.6: chainId an int; true is no int", fault("E_DOSSIER_CHAIN"))
    D("instance_upper", dict(ix, instance=up(ix["instance"])), fs(), "s1 hex20 is lowercase: instance", fault("E_DOSSIER_INSTANCE"))
    D("case_upper", dict(ix, caseId=up(ix["caseId"])), fs(), "s1 hex32 is lowercase: caseId", fault("E_DOSSIER_CASE"))
    D("logic_upper", dict(ix, logic=up(ix["logic"])), fs(), "s1 hex20 is lowercase: logic", fault("E_DOSSIER_LOGIC"))
    D("codehash_upper", dict(ix, proxyCodeHash=up(ix["proxyCodeHash"])), fs(), "s1 hex32 is lowercase: proxyCodeHash", fault("E_DOSSIER_CODEHASH"))
    D("posted_tx_upper", dict(ix, posted=dict(ix["posted"], tx=up(ix["posted"]["tx"]))), fs(), "s1 hex32 is lowercase: posted.tx", fault("E_DOSSIER_POSTED"))
    D("posted_hash_upper", dict(ix, posted=dict(ix["posted"], hash=up(ix["posted"]["hash"]))), fs(), "s1 hex32 is lowercase: posted.hash", fault("E_DOSSIER_POSTED"))
    D("posted_zero", dict(ix, posted={"tx": ZERO32, "hash": ZERO32}), fs(), "s2.1: a zero tx and hash pass the form; s3.1 then finds the listing is not the file posted.hash names", fault("E_DOSSIER_HASH"))
    f = refs(base_files); f["listing"]["sha256"] = up(f["listing"]["sha256"])
    D("files_sha_upper", dict(ix, files=f), fs(), "s1 hex32 is lowercase: a reference's sha256", fault("E_DOSSIER_FILES"))
    D("posted_string", dict(ix, posted="x"), fs(), "s2.1: posted an object", fault("E_DOSSIER_POSTED"))
    D("posted_extra", dict(ix, posted=dict(ix["posted"], block=1)), fs(), "s2.1: posted closed", fault("E_DOSSIER_POSTED"))
    D("posted_tx_short", dict(ix, posted=dict(ix["posted"], tx="0x1")), fs(), "s2.1: tx hex32", fault("E_DOSSIER_POSTED"))
    D("posted_hash_int", dict(ix, posted=dict(ix["posted"], hash=1)), fs(), "s2.1: hash hex32", fault("E_DOSSIER_POSTED"))
    f = refs(base_files); del f["receipt"]
    D("files_missing", dict(ix, files=f), fs(), "s2.1: receipt required", fault("E_DOSSIER_FILES"))
    f = refs(base_files); del f["listing"]
    D("files_missing_listing", dict(ix, files=f), fs(), "s2.1: listing required", fault("E_DOSSIER_FILES"))
    f = refs(base_files); f["terms"] = f["listing"]
    D("files_closed_terms", dict(ix, files=f), fs(), "s2.1: no terms key; the document stands in the listing", fault("E_DOSSIER_FILES"))
    f = refs(base_files); f["payload"] = f["listing"]
    D("files_closed_payload", dict(ix, files=f), fs(), "s2.1: no payload key", fault("E_DOSSIER_FILES"))
    f = refs(base_files); f["listing"] = {"name": "listing.json"}
    D("files_ref_missing", dict(ix, files=f), fs(), "s2.1: a file reference has name and sha256", fault("E_DOSSIER_FILES"))
    f = refs(base_files); f["listing"]["name"] = "sub/listing.json"
    D("files_name_slash", dict(ix, files=f), fs(), "s1: a file name carries no /", fault("E_DOSSIER_FILES"))
    f = refs(base_files); f["listing"]["name"] = ".."
    D("files_name_dotdot", dict(ix, files=f), fs(), "s1: .. is no file name", fault("E_DOSSIER_FILES"))
    f = refs(base_files); f["listing"]["name"] = ""
    D("files_name_empty", dict(ix, files=f), fs(), "s1: a file name is non-empty", fault("E_DOSSIER_FILES"))
    f = refs(base_files); f["listing"]["sha256"] = "0x1"
    D("files_sha_short", dict(ix, files=f), fs(), "s2.1: sha256 hex32", fault("E_DOSSIER_FILES"))
    D("files_string", dict(ix, files="x"), fs(), "s2.6: files an object", fault("E_DOSSIER_FILES"))
    f = refs(base_files); f["receipt"] = "receipt.json"
    D("files_ref_string", dict(ix, files=f), fs(), "s2.1: a file reference an object", fault("E_DOSSIER_FILES"))
    f = refs(base_files); f["attachments"] = {}
    D("files_attachments_object", dict(ix, files=f), fs(), "s2.1: attachments an array", fault("E_DOSSIER_FILES"))
    f = refs(base_files); f["attachments"] = ["goods.md"]
    D("files_attachment_string", dict(ix, files=f), fs(), "s2.1: an attachment reference an object", fault("E_DOSSIER_FILES"))
    f = refs(base_files); f["attachments"] = [{"name": "goods.md", "sha256": "0x1"}]
    D("files_attachment_sha_short", dict(ix, files=f), fs(), "s2.1: an attachment reference's sha256 hex32", fault("E_DOSSIER_FILES"))
    D("seat_string", dict(ix, seat="x"), fs(), "s2.6: seat an object", fault("E_DOSSIER_SEAT"))
    s = dict(ix["seat"]); del s["quoteHash"]
    D("seat_missing", dict(ix, seat=s), fs(), "s2.2: quoteHash required", fault("E_DOSSIER_SEAT"))
    D("seat_extra", dict(ix, seat=dict(ix["seat"], x=1)), fs(), "s2.2: seat closed", fault("E_DOSSIER_SEAT"))
    D("seat_judge_zero", dict(ix, seat=dict(ix["seat"], judge=ZERO20)), fs(), "s2.2: judge nonzero", fault("E_DOSSIER_SEAT"))
    D("seat_3", dict(ix, seat=dict(ix["seat"], seat=3)), fs(), "s2.2: seat at most 2", fault("E_DOSSIER_SEAT"))
    D("seat_amanah_zero", dict(ix, seat=dict(ix["seat"], amanahHash=ZERO32)), fs(), "s2.2: amanahHash nonzero", fault("E_DOSSIER_SEAT"))
    D("seat_entry_short", dict(ix, seat=dict(ix["seat"], entryId="0x1")), fs(), "s2.2: entryId hex32", fault("E_DOSSIER_SEAT"))
    D("seat_number_bool", dict(ix, seat=dict(ix["seat"], seat=True)), fs(), "s2.2: seat an int; true is no int", fault("E_DOSSIER_SEAT"))
    D("seat_judge_upper", dict(ix, seat=dict(ix["seat"], judge=up(ix["seat"]["judge"]))), fs(), "s1 hex20 is lowercase: judge", fault("E_DOSSIER_SEAT"))
    D("seat_amanah_upper", dict(ix, seat=dict(ix["seat"], amanahHash=up(ix["seat"]["amanahHash"]))), fs(), "s1 hex32 is lowercase: amanahHash", fault("E_DOSSIER_SEAT"))
    D("seat_quote_upper", dict(ix, seat=dict(ix["seat"], quoteHash=up(ix["seat"]["quoteHash"]))), fs(), "s1 hex32 is lowercase: quoteHash", fault("E_DOSSIER_SEAT"))
    D("seat_entry_upper", dict(ix, seat=dict(ix["seat"], entryId=up(eid))), fs(), "s1 hex32 is lowercase: entryId", fault("E_DOSSIER_SEAT"))
    D("transactions_object", dict(ix, transactions={}), fs(), "s2.6: transactions an array", fault("E_DOSSIER_TRANSACTIONS"))
    rows = txrows(2); rows[1] = "x"
    D("transaction_string", dict(ix, transactions=rows), fs(), "s2.3: a row not an object, index 1", fault("E_TRANSACTION", 1))
    rows = txrows(2); del rows[0]["tx"]
    D("transaction_missing", dict(ix, transactions=rows), fs(), "s2.3: tx required", fault("E_TRANSACTION", 0))
    rows = txrows(2); rows[1]["x"] = 1
    D("transaction_closed", dict(ix, transactions=rows), fs(), "s2.3: a row closed", fault("E_TRANSACTION", 1))
    rows = txrows(1); rows[0]["kind"] = "payment"
    D("transaction_kind", dict(ix, transactions=rows), fs(), "s2.3: kind one of the twenty-three", fault("E_TRANSACTION", 0))
    rows = txrows(1); rows[0]["kind"] = "bond"
    D("transaction_kind_bond", dict(ix, transactions=rows), fs(), "s2.3: bond is no kind of this law", fault("E_TRANSACTION", 0))
    rows = txrows(1); rows[0]["kind"] = "Fund"
    D("transaction_kind_case", dict(ix, transactions=rows), fs(), "s2.3: kinds compare byte for byte", fault("E_TRANSACTION", 0))
    rows = txrows(1); rows[0]["block"] = "1"
    D("transaction_block", dict(ix, transactions=rows), fs(), "s2.3: block an int", fault("E_TRANSACTION", 0))
    rows = txrows(1); rows[0]["block"] = True
    D("transaction_block_bool", dict(ix, transactions=rows), fs(), "s2.3: block an int; true is no int", fault("E_TRANSACTION", 0))
    rows = txrows(1); rows[0]["timestamp"] = True
    D("transaction_timestamp_bool", dict(ix, transactions=rows), fs(), "s2.3: timestamp an int; true is no int", fault("E_TRANSACTION", 0))
    rows = txrows(1); rows[0]["tx"] = up(rows[0]["tx"])
    D("transaction_tx_upper", dict(ix, transactions=rows), fs(), "s1 hex32 is lowercase: tx", fault("E_TRANSACTION", 0))
    rows = txrows(1); rows[0]["note_md"] = 1
    D("transaction_note", dict(ix, transactions=rows), fs(), "s2.3: note_md a string", fault("E_TRANSACTION", 0))
    rows = txrows(3); rows[0]["kind"] = "cancel"; rows[0]["note_md"] = "买家\n撤单"
    ix2 = dict(ix, transactions=rows)
    D("ok_transaction_note", ix2, fs(), "s2.3: cancel is a kind; note_md is prose", okd(ix2, r, listing))
    D("terminal_string", dict(ix, terminal="Ruled"), fs(), "s2.4: terminal an object", fault("E_DOSSIER_TERMINAL"))
    D("terminal_name", dict(ix, terminal=dict(term2, name="Closed")), fs(), "s2.4: name one of the six", fault("E_DOSSIER_TERMINAL"))
    D("terminal_name_case", dict(ix, terminal=dict(term2, name="released")), fs(), "s2.4: name byte for byte", fault("E_DOSSIER_TERMINAL"))
    D("terminal_missing", dict(ix, terminal={"name": "Released"}), fs(), "s2.4: the four amounts required", fault("E_DOSSIER_TERMINAL"))
    D("terminal_amount_number", dict(ix, terminal=dict(term2, buyerPaid=0)), fs(), "s2.4: amounts are dec256 strings", fault("E_DOSSIER_TERMINAL"))
    D("terminal_extra", dict(ix, terminal=dict(term2, rung=1)), fs(), "s2.4: terminal closed", fault("E_DOSSIER_TERMINAL"))
    D("ruled_without_entry", dict(ix, terminal=term), fs(), "s2.4: Ruled requires entryId", fault("E_DOSSIER_RULED"))
    D("compliance_string", dict(ix, compliance="x"), fs(), "s2.5: compliance an object", fault("E_DOSSIER_COMPLIANCE"))
    D("compliance_closed", dict(ix, compliance={"tax": 1}), fs(), "s2.5: compliance closed", fault("E_DOSSIER_COMPLIANCE"))
    D("compliance_closed_before_rows", dict(ix, compliance={"tax": 1, "conversions": [{}]}), fs(), "s2.5: no member beyond the list is decided before the rows, so no index", fault("E_DOSSIER_COMPLIANCE"))
    D("compliance_serial_before_conversion", dict(ix, compliance={"serial": "", "conversions": [{}]}), fs(), "s2.5 order: the rows in the order written, serial's before conversions', so no index", fault("E_DOSSIER_COMPLIANCE"))
    D("compliance_buyer_int", dict(ix, compliance={"buyer_md": 1}), fs(), "s2.5: buyer_md a string", fault("E_DOSSIER_COMPLIANCE"))
    D("compliance_serial_empty", dict(ix, compliance={"serial": ""}), fs(), "s2.5: serial non-empty", fault("E_DOSSIER_COMPLIANCE"))
    D("compliance_conversions_object", dict(ix, compliance={"conversions": {}}), fs(), "s2.5: conversions an array", fault("E_DOSSIER_COMPLIANCE"))
    D("compliance_seller_before_conversion", dict(ix, compliance={"seller_md": 1, "conversions": ["x"]}), fs(), "s2.5 order: seller_md's row before the conversions', so no index", fault("E_DOSSIER_COMPLIANCE"))
    D("compliance_buyer_before_conversion", dict(ix, compliance={"buyer_md": 1, "conversions": ["x"]}), fs(), "s2.5 order: buyer_md's row before the conversions', so no index", fault("E_DOSSIER_COMPLIANCE"))
    convb = {"tx": H32(), "at": True, "unit": "USD", "rate": "1", "source": "x"}
    D("compliance_conversion_at_bool", dict(ix, compliance={"conversions": [convb]}), fs(), "s2.5: at an int; true is no int, index 0", fault("E_DOSSIER_COMPLIANCE", 0))
    convu = {"tx": up(H32()), "at": 1, "unit": "USD", "rate": "1", "source": "x"}
    D("compliance_conversion_tx_upper", dict(ix, compliance={"conversions": [convu]}), fs(), "s1 hex32 is lowercase: a conversion's tx, index 0", fault("E_DOSSIER_COMPLIANCE", 0))
    ixce = index_of(listing, lb, ov, r, refs(base_files), compliance={"conversions": []})
    D("ok_conversions_empty", ixce, fs(), "s2.5: conversions may be an empty array", okd(ixce, r, listing))
    conv = {"tx": H32(), "at": 1, "unit": "USD", "rate": "1", "source": "x"}
    D("compliance_conversion_missing", dict(ix, compliance={"conversions": [conv, {"tx": H32()}]}), fs(), "s2.5: a conversion's members, index 1", fault("E_DOSSIER_COMPLIANCE", 1))
    D("compliance_conversion_element_before_element", dict(ix, compliance={"conversions": [dict(conv, rate=""), "x"]}), fs(), "s2.5 order: conversion 0's rows before conversion 1's form", fault("E_DOSSIER_COMPLIANCE", 0))
    D("compliance_conversion_rate_empty", dict(ix, compliance={"conversions": [dict(conv, rate="")]}), fs(), "s2.5: rate non-empty, index 0", fault("E_DOSSIER_COMPLIANCE", 0))
    D("compliance_conversion_tx", dict(ix, compliance={"conversions": [dict(conv, tx="0x1")]}), fs(), "s2.5: tx hex32, index 0", fault("E_DOSSIER_COMPLIANCE", 0))
    D("compliance_conversion_string", dict(ix, compliance={"conversions": ["x"]}), fs(), "s2.5: a conversion an object", fault("E_DOSSIER_COMPLIANCE", 0))
    d = dict(ix); del d["seat"]; d["x"] = 1
    D("order_missing_before_closed", d, fs(), "s2.6 order", fault("E_DOSSIER_MISSING"))
    d = dict(ix, v="x"); d["x"] = 1
    D("order_closed_before_v", d, fs(), "s2.6 order", fault("E_DOSSIER_CLOSED"))
    D("order_v_before_chain", dict(ix, v="x", chainId="1"), fs(), "s2.6 order", fault("E_DOSSIER_V"))
    D("order_chain_before_instance", dict(ix, chainId="1", instance="0x1"), fs(), "s2.6 order", fault("E_DOSSIER_CHAIN"))
    D("order_instance_before_case", dict(ix, instance="0x1", caseId="0x1"), fs(), "s2.6 order", fault("E_DOSSIER_INSTANCE"))
    D("order_case_before_logic", dict(ix, caseId="0x1", logic="0x1"), fs(), "s2.6 order", fault("E_DOSSIER_CASE"))
    D("order_logic_before_codehash", dict(ix, logic="0x1", proxyCodeHash="0x1"), fs(), "s2.6 order", fault("E_DOSSIER_LOGIC"))
    D("order_codehash_before_posted", dict(ix, proxyCodeHash="0x1", posted="x"), fs(), "s2.6 order", fault("E_DOSSIER_CODEHASH"))
    D("order_posted_before_files", dict(ix, posted="x", files="y"), fs(), "s2.6 order", fault("E_DOSSIER_POSTED"))
    D("order_files_before_seat", dict(ix, files="x", seat="y"), fs(), "s2.6 order", fault("E_DOSSIER_FILES"))
    D("order_seat_before_transactions", dict(ix, seat="x", transactions={}), fs(), "s2.6 order", fault("E_DOSSIER_SEAT"))
    D("order_transactions_before_terminal", dict(ix, transactions={}, terminal="x"), fs(), "s2.6 order", fault("E_DOSSIER_TRANSACTIONS"))
    D("order_terminal_before_compliance", dict(ix, terminal="x", compliance="y"), fs(), "s2.6 order", fault("E_DOSSIER_TERMINAL"))
    D("order_ruled_before_compliance", dict(ix, terminal=term, compliance="y"), fs(), "s2.6 order: the Ruled rule before compliance", fault("E_DOSSIER_RULED"))
    D("order_index_before_files", dict(ix, compliance="y"), {"listing.json": lb}, "s2.6 before s3.1: the index's fault before any file is read", fault("E_DOSSIER_COMPLIANCE"))

    # ---- the files, s3.1
    D("file_listing_absent", ix, {"receipt.json": rb}, "s3.1: E_DOSSIER_FILE listing", fault("E_DOSSIER_FILE", key="listing"))
    D("file_receipt_absent", ix, {"listing.json": lb}, "s3.1: E_DOSSIER_FILE receipt", fault("E_DOSSIER_FILE", key="receipt"))
    ixe = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", ej)}), entry_id=eid)
    D("file_entry_absent", ixe, fs(), "s3.1: E_DOSSIER_FILE entry", fault("E_DOSSIER_FILE", key="entry"))
    D("file_listing_digest", ix, fs({"listing.json": lb + b" "}), "s3.1: E_DOSSIER_DIGEST listing", fault("E_DOSSIER_DIGEST", key="listing"))
    fz = refs(base_files); fz["listing"]["sha256"] = ZERO32
    D("files_sha_zero", dict(ix, files=fz), fs(), "s2.6: a file's sha256 hex32, zero among them; s3.1's digest test then fails with the key", fault("E_DOSSIER_DIGEST", key="listing"))
    D("file_receipt_digest", ix, fs({"receipt.json": rb + b" "}), "s3.1: E_DOSSIER_DIGEST receipt", fault("E_DOSSIER_DIGEST", key="receipt"))
    D("file_entry_digest", ixe, fs({"entry.bin": ej + b" "}), "s3.1: E_DOSSIER_DIGEST entry", fault("E_DOSSIER_DIGEST", key="entry"))
    D("file_listing_before_receipt", ix, {}, "s3.1 order: listing before receipt", fault("E_DOSSIER_FILE", key="listing"))
    D("file_listing_digest_before_receipt_absent", ix, {"listing.json": lb + b" "}, "s3.1 order: both tests of one key before the next key", fault("E_DOSSIER_DIGEST", key="listing"))
    D("file_receipt_before_entry", ixe, {"listing.json": lb, "entry.bin": ej}, "s3.1 order: receipt before entry", fault("E_DOSSIER_FILE", key="receipt"))
    D("file_entry_before_documents", ixe, fs({"entry.bin": ej + b" ", "listing.json": b"["}), "s3.1 order: every key's digest before any document test; listing.json's digest fails first", fault("E_DOSSIER_DIGEST", key="listing"))
    ixa = index_of(lis_h, lhb, ov_h, r_h, refs(hfiles, [("goods.md", gt)]))
    D("file_entry_before_attachment", dict(ixa, files=dict(ixa["files"], entry={"name": "entry.bin", "sha256": eid}), seat=dict(ixa["seat"], entryId=eid)), {"listing.json": lhb, "receipt.json": rhb, "goods.md": gt}, "s3.1 order: entry before the attachments", fault("E_DOSSIER_FILE", key="entry"))
    D("file_attachment_before_documents", ixa, {"listing.json": b"[", "receipt.json": rhb}, "s3.1 order: the attachment's file before the listing's document test", fault("E_DOSSIER_DIGEST", key="listing"))
    D("ok_extra_file", ix, fs({"notes.txt": b"anything", "dossier.json.bak": b"x"}), "s1: a file the index does not name is no fault", okd(ix, r, listing))
    f = refs(base_files); f["receipt"] = {"name": "listing.json", "sha256": sha(lb)}
    D("two_references_one_file", dict(ix, files=f), fs(), "s1: two references may name one file; the receipt test then reads the listing's bytes", fault("E_DOSSIER_RECEIPT", inner=inner("E_RECEIPT_MISSING")))
    f = refs(base_files); f["listing"] = {"name": "dossier.json", "sha256": H32()}
    D("reference_names_index", dict(ix, files=f), fs(), "s1: a reference may name dossier.json, read like any file; its digest fails", fault("E_DOSSIER_DIGEST", key="listing"))
    D("link_dangling", ix, fs({"listing.json": ("link", "nowhere")}), "s1: a symbolic link resolving to nothing is unreadable", fault("E_DOSSIER_FILE", key="listing"))
    D("link_loop", ix, {"receipt.json": rb, "listing.json": ("link", "loop.json"), "loop.json": ("link", "listing.json")}, "s1: a loop of two links resolves to no regular file", fault("E_DOSSIER_FILE", key="listing"))
    D("link_to_file", ix, {"receipt.json": rb, "real.json": lb, "listing.json": ("link", "real.json")}, "s1: a symbolic link resolving to a regular file counts as that file", okd(ix, r, listing))
    D("name_case", ix, {"Listing.json": lb, "receipt.json": rb}, "s1: names match byte for byte; Listing.json is not listing.json", fault("E_DOSSIER_FILE", key="listing"))
    ixn = index_of(listing, lb, ov, r, refs(base_files))
    D("link_to_link", ixn, {"receipt.json": rb, "real.json": lb, "mid.json": ("link", "real.json"), "listing.json": ("link", "mid.json")}, "s1: a link through a link resolves to the regular file", okd(ixn, r, listing))
    D("index_link_to_file", None, {"real_index.json": cbytes(ixn), "dossier.json": ("link", "real_index.json"), "listing.json": lb, "receipt.json": rb}, "s1: dossier.json itself may be a link to a regular file", okd(ixn, r, listing))
    D("index_name_case", None, {"Dossier.json": cbytes(ixn), "listing.json": lb, "receipt.json": rb}, "s1: dossier.json matched byte for byte; Dossier.json is no index", fault("E_DOSSIER_FILE", key="index"))
    f = refs(base_files); f["listing"]["name"] = "."
    D("files_name_dot", dict(ixn, files=f), fs(), "s1: . is no file name", fault("E_DOSSIER_FILES"))
    D("seat_judge_short", dict(ixn, seat=dict(ixn["seat"], judge="0x1")), fs(), "s2.2: judge hex20", fault("E_DOSSIER_SEAT"))
    D("seat_quote_short", dict(ixn, seat=dict(ixn["seat"], quoteHash="0x1")), fs(), "s2.2: quoteHash hex32", fault("E_DOSSIER_SEAT"))
    D("seat_number_string", dict(ixn, seat=dict(ixn["seat"], seat="0")), fs(), "s2.2: seat an int", fault("E_DOSSIER_SEAT"))
    sn = dict(ixn["seat"]); del sn["judge"]
    D("seat_no_judge_named", dict(ixn, seat=sn), fs(), "s2.2 and s3.1: a named candidate's seat carries judge", fault("E_DOSSIER_SEAT_MISMATCH"))
    ixg0 = index_of(listing, lb, ovg, rg, refs(gfiles), seat={"seat": 0})
    del ixg0["seat"]["judge"]
    D("ok_guild_seat_no_judge", ixg0, {"listing.json": lb, "receipt.json": rgb}, "s2.2: a pool case nobody was seated on carries no judge, seat 0", okd(ixg0, rg, listing))
    # s2.3: the rows related to nothing: no order among them, none to another, to the seat, the terminal, posted or the listing
    tx_x, tx_y, tx_z = H32(), H32(), H32()
    b0, t0 = 20000000, 1750000000
    rows = [{"kind": "init", "tx": tx_x, "block": b0 + 5, "timestamp": t0 + 60},
            {"kind": "fund", "tx": tx_x, "block": b0 + 5, "timestamp": t0 + 60},
            {"kind": "fundBind", "tx": tx_y, "block": b0 + 9, "timestamp": t0 + 12},
            {"kind": "fund", "tx": tx_z, "block": b0 + 9, "timestamp": t0 + 999},
            {"kind": "rule", "tx": H32(), "block": b0 + 4, "timestamp": t0 + 48},
            {"kind": "accept", "tx": H32(), "block": b0 + 3, "timestamp": t0 + 36},
            {"kind": "settle", "tx": H32(), "block": b0 + 2, "timestamp": t0 + 24},
            {"kind": "cancel", "tx": H32(), "block": b0 + 1, "timestamp": t0 + 12},
            {"kind": "resign", "tx": H32(), "block": b0 + 1, "timestamp": t0 + 12},
            {"kind": "post", "tx": H32(), "block": 1, "timestamp": t0},
            {"kind": "init", "tx": tx_x, "block": b0 + 7, "timestamp": t0 + 84}]
    ixu = index_of(listing, lb, ovg, rg, refs(gfiles), seat={"seat": 0}, transactions=rows)
    del ixu["seat"]["judge"]
    D("ok_rows_unrelated", ixu, {"listing.json": lb, "receipt.json": rgb}, "s2.3: no predicate orders the rows or relates them to each other, the seat, the terminal or posted: kinds and txs repeated, blocks and timestamps out of order, rule and accept and resign beside a seat with no judge and no terminal", okd(ixu, rg, listing))
    ixgq = index_of(listing, lb, ovg, rg, refs(gfiles), seat={"quoteHash": H32()})
    D("guild_seat_quote_mismatch", ixgq, {"listing.json": lb, "receipt.json": rgb}, "s3.1: a pool seat's quoteHash is the candidate's", fault("E_DOSSIER_SEAT_MISMATCH"))
    rows = txrows(1); rows[0]["tx"] = "0x12"
    D("transaction_tx_short", dict(ixn, transactions=rows), fs(), "s2.3: tx hex32", fault("E_TRANSACTION", 0))
    rows = txrows(1); rows[0]["timestamp"] = "1750000000"
    D("transaction_timestamp_string", dict(ixn, transactions=rows), fs(), "s2.3: timestamp an int", fault("E_TRANSACTION", 0))
    D("order_transaction_before_terminal", dict(ixn, transactions=["x"], terminal="Ruled"), fs(), "s2.6 order: the rows before the terminal", fault("E_TRANSACTION", 0))
    term_n = {"name": "Ruled", "buyerPaid": 1, "sellerPaid": "0", "judgePaid": "0", "bondToBuyer": "0"}
    D("order_terminal_before_ruled", dict(ixn, terminal=term_n), fs(), "s2.6 order: the terminal's form before its Ruled rule", fault("E_DOSSIER_TERMINAL"))
    term_s = {"name": "Released", "buyerPaid": "0", "sellerPaid": "990000000", "judgePaid": "10000000", "bondToBuyer": "0"}
    D("terminal_amount_leading_zero", dict(ixn, terminal=dict(term_s, buyerPaid="01")), fs(), "s2.4: dec256 has no leading zero", fault("E_DOSSIER_TERMINAL"))
    D("terminal_amount_over", dict(ixn, terminal=dict(term_s, buyerPaid=str(2 ** 256))), fs(), "s2.4: dec256 at most 2^256 - 1", fault("E_DOSSIER_TERMINAL"))
    D("compliance_seller_int", dict(ixn, compliance={"seller_md": 1}), fs(), "s2.5: seller_md a string", fault("E_DOSSIER_COMPLIANCE"))
    D("compliance_serial_int", dict(ixn, compliance={"serial": 7}), fs(), "s2.5: serial a string", fault("E_DOSSIER_COMPLIANCE"))
    conv_n = {"tx": H32(), "at": 1750000000, "unit": "USD", "rate": "1.0", "source": "https://rate.example"}
    D("compliance_conversion_at", dict(ixn, compliance={"conversions": [dict(conv_n, at="1")]}), fs(), "s2.5: at an int, index 0", fault("E_DOSSIER_COMPLIANCE", 0))
    D("compliance_conversion_unit_empty", dict(ixn, compliance={"conversions": [dict(conv_n, unit="")]}), fs(), "s2.5: unit non-empty, index 0", fault("E_DOSSIER_COMPLIANCE", 0))
    D("compliance_conversion_extra", dict(ixn, compliance={"conversions": [dict(conv_n, note="x")]}), fs(), "s2.5: a conversion closed, index 0", fault("E_DOSSIER_COMPLIANCE", 0))
    body_k = {"subjectRef": {"kind": "evm2", "chainId": 1, "contract": ov["instance"], "codeHash": ch, "id": ov["salt"], "seat": 0}, "amanah": amanah, "verdict_md": "x", "decision": {"rung": 2}, "evidence": []}
    ek = G.entry("judge", 3, H32(), "judgment", body_k)
    ixk2 = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", ek)}), entry_id=sha(ek), terminal=term)
    D("subject_kind_other", ixk2, fs({"entry.bin": ek}), "s3.1: subjectRef of kind evm", fault("E_DOSSIER_SUBJECT"))
    ers1 = resignation("judge", 4, H32(), 1, ov["instance"], ch, ov["salt"], amanah, seat=1)
    ixr1 = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("resign.bin", ers1)}), entry_id=sha(ers1), terminal=term3)
    D("subject_resignation_seat_other", ixr1, fs({"resign.bin": ers1}), "s3.1: a resignation's seat, where present, equals the index's", fault("E_DOSSIER_SUBJECT"))
    etrue = judgment("judge", 3, H32(), 1, ov["instance"], ch, ov["salt"], amanah, seat=True)
    ixtrue = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", etrue)}), entry_id=sha(etrue), terminal=term)
    D("subject_seat_bool", ixtrue, fs({"entry.bin": etrue}), "s3.1: seat an int; true is no int", fault("E_DOSSIER_SUBJECT"))
    egt = judgment("drawn", 1, H32(), 1, ovg["instance"], ch, ovg["salt"], listing["guilds"][0]["amanahHash"], seat=True)
    ixgt = index_of(listing, lb, ovg, rg, refs({**gfiles, "entry": ("entry.bin", egt)}), entry_id=sha(egt), terminal=term)
    D("subject_seat_bool_pool", ixgt, {"listing.json": lb, "receipt.json": rgb, "entry.bin": egt}, "s3.1: a pool seat 1 against a judgment's seat true; true is no int, whatever a loose comparison makes of 1", fault("E_DOSSIER_SUBJECT"))
    bad = copy(listing); bad["order"]["nonce"] = "7"
    bb = cbytes(bad)
    f = refs({"listing": ("listing.json", bb), "receipt": ("receipt.json", rb)})
    D("listing_invalid", dict(ix, files=f, posted=posted_of(bb)), {"listing.json": bb, "receipt.json": rb}, "s3.1: E_DOSSIER_LISTING carrying the token", fault("E_DOSSIER_LISTING", inner=inner("E_LISTING_NONCE")))
    bad = copy(listing); bad["judges"][0]["sig"] = bad["judges"][0]["sig"][:-2] + "1d"
    bb = cbytes(bad)
    f = refs({"listing": ("listing.json", bb), "receipt": ("receipt.json", rb)})
    D("listing_invalid_sig", dict(ix, files=f, posted=posted_of(bb)), {"listing.json": bb, "receipt.json": rb}, "s3.1: the carried fault keeps its index and inner", fault("E_DOSSIER_LISTING", inner=inner("E_LISTING_SIG", 0, inner_=inner("E_SIG_V"))))
    D("hash", dict(ix, posted=dict(ix["posted"], hash=H32())), fs(), "s3.1: E_DOSSIER_HASH, the listing file is not the one posted.hash names", fault("E_DOSSIER_HASH"))
    bad = copy(listing); bad["guilds"][0]["seq"] = 4
    bb = cbytes(bad)
    f = refs({"listing": ("listing.json", bb), "receipt": ("receipt.json", rb)})
    D("hash_other_seq", dict(ix, files=f), {"listing.json": bb, "receipt.json": rb}, "s3.1: a listing whose seq differs is other bytes than the posted ones", fault("E_DOSSIER_HASH"))
    f = refs({"listing": ("listing.json", lb), "receipt": ("receipt.json", b"[]")})
    D("receipt_invalid", dict(ix, files=f), {"listing.json": lb, "receipt.json": b"[]"}, "s3.1: E_DOSSIER_RECEIPT carrying the token", fault("E_DOSSIER_RECEIPT", inner=inner("E_RECEIPT")))
    rbad = cbytes(dict(r, termsHash=H32()))
    f = refs({"listing": ("listing.json", lb), "receipt": ("receipt.json", rbad)})
    D("order_terms_mismatch", dict(ix, files=f), {"listing.json": lb, "receipt.json": rbad}, "s3.1: E_DOSSIER_ORDER carrying RECEIPT_TERMS_MISMATCH", fault("E_DOSSIER_ORDER", inner=inner("RECEIPT_TERMS_MISMATCH")))
    rbad = cbytes(dict(r, buyerPayout=addr("other")))
    f = refs({"listing": ("listing.json", lb), "receipt": ("receipt.json", rbad)})
    D("order_sig", dict(ix, files=f), {"listing.json": lb, "receipt.json": rbad}, "s3.1: E_DOSSIER_ORDER carrying RECEIPT_SIG with its inner", fault("E_DOSSIER_ORDER", inner=inner("RECEIPT_SIG", inner_=inner("E_SIG_SIGNER"))))
    rbad = cbytes(dict(r, candidate=5))
    f = refs({"listing": ("listing.json", lb), "receipt": ("receipt.json", rbad)})
    D("order_range", dict(ix, files=f), {"listing.json": lb, "receipt.json": rbad}, "s3.1: E_DOSSIER_ORDER carrying RECEIPT_CANDIDATE_RANGE", fault("E_DOSSIER_ORDER", inner=inner("RECEIPT_CANDIDATE_RANGE")))
    rbad = cbytes(dict(r, buyerKey=addr("seller")))
    f = refs({"listing": ("listing.json", lb), "receipt": ("receipt.json", rbad)})
    D("order_buyer", dict(ix, files=f), {"listing.json": lb, "receipt.json": rbad}, "s3.1: E_DOSSIER_ORDER carrying RECEIPT_ORDER", fault("E_DOSSIER_ORDER", inner=inner("RECEIPT_ORDER", inner_=inner("E_ORDER_BUYER"))))
    D("chain_mismatch", dict(ix, chainId=5), fs(), "s3.1: E_DOSSIER_CHAIN_MISMATCH", fault("E_DOSSIER_CHAIN_MISMATCH"))
    D("logic_mismatch", dict(ix, logic=addr("x")), fs(), "s3.1: E_DOSSIER_LOGIC_MISMATCH", fault("E_DOSSIER_LOGIC_MISMATCH"))
    D("codehash_mismatch", dict(ix, proxyCodeHash=H32()), fs(), "s3.1: E_DOSSIER_CODEHASH_MISMATCH", fault("E_DOSSIER_CODEHASH_MISMATCH"))
    D("instance_mismatch", dict(ix, instance=addr("x")), fs(), "s3.1: E_DOSSIER_INSTANCE_MISMATCH", fault("E_DOSSIER_INSTANCE_MISMATCH"))
    D("instance_zero", dict(ix, instance=ZERO20), fs(), "s2.6 admits a zero instance; s3.1's instance test then fails", fault("E_DOSSIER_INSTANCE_MISMATCH"))
    D("case_mismatch", dict(ix, caseId=H32()), fs(), "s3.1: E_DOSSIER_CASE_MISMATCH", fault("E_DOSSIER_CASE_MISMATCH"))
    D("chain_zero", dict(ix, chainId=0), fs(), "s2.6: chainId an int, 0 among them; s3.1's chain test then fails", fault("E_DOSSIER_CHAIN_MISMATCH"))
    D("logic_zero", dict(ix, logic=ZERO20), fs(), "s2.6: logic hex20, zero among them; s3.1's logic test then fails", fault("E_DOSSIER_LOGIC_MISMATCH"))
    D("codehash_zero", dict(ix, proxyCodeHash=ZERO32), fs(), "s2.6: proxyCodeHash hex32, zero among them; s3.1's code hash test then fails", fault("E_DOSSIER_CODEHASH_MISMATCH"))
    D("case_zero", dict(ix, caseId=ZERO32), fs(), "s2.6: caseId hex32, zero among them; s3.1's case test then fails", fault("E_DOSSIER_CASE_MISMATCH"))
    D("seat_judge_mismatch", dict(ix, seat=dict(ix["seat"], judge=addr("other"))), fs(), "s3.1: a named candidate's judge differs", fault("E_DOSSIER_SEAT_MISMATCH"))
    D("seat_number_mismatch", dict(ix, seat=dict(ix["seat"], seat=1)), fs(), "s3.1: a named candidate's seat is 0", fault("E_DOSSIER_SEAT_MISMATCH"))
    D("seat_amanah_mismatch", dict(ix, seat=dict(ix["seat"], amanahHash=H32())), fs(), "s3.1: amanahHash differs", fault("E_DOSSIER_SEAT_MISMATCH"))
    D("seat_quote_mismatch", dict(ix, seat=dict(ix["seat"], quoteHash=H32())), fs(), "s3.1: quoteHash differs from the candidate's", fault("E_DOSSIER_SEAT_MISMATCH"))
    D("seat_quote_zero", dict(ix, seat=dict(ix["seat"], quoteHash=ZERO32)), fs(), "s3.1: the seat's quoteHash form admits zero; the comparison refuses it", fault("E_DOSSIER_SEAT_MISMATCH"))
    ixg = index_of(listing, lb, ovg, rg, refs(gfiles))
    D("guild_seat_amanah_mismatch", dict(ixg, seat=dict(ixg["seat"], amanahHash=H32())), {"listing.json": lb, "receipt.json": rgb}, "s3.1: a pool seat's amanahHash is compared", fault("E_DOSSIER_SEAT_MISMATCH"))
    D("ok_guild_seat_any_judge", dict(ixg, seat=dict(ixg["seat"], judge=addr("anyone"), seat=0)), {"listing.json": lb, "receipt.json": rgb}, "s3.1: a pool seat's judge and number are not compared here", okd(ixg, rg, listing))
    D("entry_without_file", dict(ix, seat=dict(ix["seat"], entryId=eid)), fs(), "s3.1: entryId present iff files.entry", fault("E_DOSSIER_ENTRY"))
    D("file_without_entry", dict(ix, files=refs({**base_files, "entry": ("entry.bin", ej)})), fs({"entry.bin": ej}), "s3.1: files.entry without entryId", fault("E_DOSSIER_ENTRY"))
    D("entry_digest", dict(ixe, seat=dict(ixe["seat"], entryId=H32())), fs({"entry.bin": ej}), "s3.1: E_DOSSIER_ENTRY_DIGEST", fault("E_DOSSIER_ENTRY_DIGEST"))
    D("entry_zero", dict(ixe, seat=dict(ixe["seat"], entryId=ZERO32)), fs({"entry.bin": ej}), "s2.2: entryId is a hex32, zero included in its form; s3.1's digest test then fails it", fault("E_DOSSIER_ENTRY_DIGEST"))
    junk = b"{}"
    ixj = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", junk)}), entry_id=sha(junk))
    D("entry_form", ixj, fs({"entry.bin": junk}), "s3.1: E_DOSSIER_ENTRY_FORM carrying the galeed/1 token", fault("E_DOSSIER_ENTRY_FORM", inner=inner("E_ENVELOPE_MISSING")))
    ann = G.entry("judge", 5, H32(), "annotation", {"note_md": "x"})
    ixan = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", ann)}), entry_id=sha(ann))
    D("entry_type", ixan, fs({"entry.bin": ann}), "s3.1: E_DOSSIER_ENTRY_TYPE, an annotation", fault("E_DOSSIER_ENTRY_TYPE"))
    mig = G.entry("judge", 5, H32(), "migration", {"spec": "galeed/2", "statement_md": "x"})
    ixmg = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", mig)}), entry_id=sha(mig))
    D("entry_type_migration", ixmg, fs({"entry.bin": mig}), "s3.1: E_DOSSIER_ENTRY_TYPE, a migration, an entry type of galeed/1 s4.3 as it stands", fault("E_DOSSIER_ENTRY_TYPE"))
    # ---- the entry test this core carries (s1 entry(b), s7.1): galeed/1 s4.3 from step 2 on, token by token,
    # E_BODY_FIELD under each type of galeed/1 s6, and adjacent steps in their order
    from glentry import entry_digest
    from glcrypto import N as N_SECP
    P_SECP = 2 ** 256 - 2 ** 32 - 977

    def jbody(**over):
        b = {"subjectRef": {"kind": "evm", "chainId": 1, "contract": ov["instance"], "codeHash": ch, "id": ov["salt"], "seat": 0},
             "amanah": amanah, "verdict_md": "x", "decision": {"rung": 1}}
        b.update(over)
        return b

    def ENV(seq, prev, etype, body, spec="galeed/1", judge=None, signer="judge", extra=None, sig=None):
        env = {"body": body, "entryType": etype, "judge": judge or addr("judge"), "prev": prev, "seq": seq, "spec": spec}
        dg = entry_digest(cbytes(env))
        env["sig"] = G.sign(key(signer), dg) if sig is None else sig(G.sign(key(signer), dg))
        if extra:
            env.update(extra)
        return cbytes(env)

    def EF(slug, eb, note, tok):
        ixf = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", eb)}), entry_id=sha(eb))
        D(slug, ixf, fs({"entry.bin": eb}), "s3.1, s7.1: E_DOSSIER_ENTRY_FORM carrying " + tok + "; " + note,
          fault("E_DOSSIER_ENTRY_FORM", inner=inner(tok)))

    def parts(sg):
        b = bytes.fromhex(sg[2:])
        return b[:32], int.from_bytes(b[32:64], "big"), b[64]
    hp = H32()
    EF("entry_form_envelope", cbytes([]), "galeed/1 s4.3 step 2: the root is an object", "E_ENVELOPE")
    EF("entry_form_closed", ENV(3, hp, "judgment", jbody(), extra={"note": "x"}), "step 3: an eighth member", "E_ENVELOPE_CLOSED")
    EF("entry_form_spec", ENV(3, hp, "judgment", jbody(), spec="galeed/2"), "step 4: spec byte-equal to galeed/1", "E_SPEC")
    EF("entry_form_entrytype", ENV(3, hp, "two words", jbody()), "step 5: entryType a token", "E_ENTRYTYPE")
    EF("entry_form_judge_upper", ENV(3, hp, "judgment", jbody(), judge=addr("judge").upper().replace("0X", "0x")), "step 6: judge hex20, lowercase", "E_JUDGE")
    EF("entry_form_seq", ENV("3", hp, "judgment", jbody()), "step 7: seq an int", "E_SEQ")
    EF("entry_form_prev", ENV(3, "0x12", "judgment", jbody()), "step 8: prev null or hex32", "E_PREV")
    EF("entry_form_prev_seq", ENV(3, None, "judgment", jbody()), "step 8: prev null iff seq 0", "E_PREV_SEQ")
    EF("entry_form_body", ENV(3, hp, "judgment", []), "step 9: body an object", "E_BODY")
    EF("entry_form_sig_form", ENV(3, hp, "judgment", jbody(), sig=lambda g: "0x12"), "step 10: sig hex65", "E_SIG_FORM")
    EF("entry_form_genesis_place", ENV(3, hp, "genesis", {"statement_md": "x"}), "step 11: a genesis off position 0", "E_GENESIS_PLACE")
    # step 12 under each type of galeed/1 s6
    EF("entry_form_body_genesis", ENV(0, None, "genesis", {}), "step 12, s6.1: statement_md required", "E_BODY_FIELD")
    EF("entry_form_body_amanah", ENV(3, hp, "amanah", {"version": 1, "statement_md": "x"}), "step 12, s6.2: text_md required", "E_BODY_FIELD")
    EF("entry_form_body_judgment", ENV(3, hp, "judgment", jbody(verdict_md=1)), "step 12, s6.3: verdict_md a prose string", "E_BODY_FIELD")
    sr = {k: v for k, v in jbody()["subjectRef"].items() if k != "kind"}
    EF("entry_form_subject_kind", ENV(3, hp, "judgment", jbody(subjectRef=sr)), "step 12, s6.3: subjectRef's kind required", "E_BODY_FIELD")
    rb_ = {"subjectRef": jbody()["subjectRef"], "amanah": amanah, "category": "other", "clause": "c1", "statement_md": "x"}
    EF("entry_form_body_resignation", ENV(4, hp, "resignation", rb_), "step 12, s6.4: clause present iff unlawful-subject", "E_BODY_FIELD")
    EF("entry_form_body_annotation", ENV(5, hp, "annotation", {}), "step 12, s6.5: note_md required", "E_BODY_FIELD")
    EF("entry_form_body_succession", ENV(5, hp, "succession", {"to": addr("judge2"), "kind": "handover", "effective": 1}), "step 12, s6.6: statement_md required", "E_BODY_FIELD")
    EF("entry_form_body_migration", ENV(5, hp, "migration", {"statement_md": "x"}), "step 12, s6.6a: spec required", "E_BODY_FIELD")
    EF("entry_form_migration_spec_space", ENV(5, hp, "migration", {"spec": "galeed 2", "statement_md": "x"}), "step 12, s6.6a: spec a token", "E_BODY_FIELD")
    EF("entry_form_body_keybinding", ENV(5, hp, "keybinding", {"recipient": "age1x"}), "step 12, s6.7: effective required", "E_BODY_FIELD")
    # step 13, s5.4 in its bullet order and s5.5

    def lowv(g):
        rr, ss, vv = parts(g)
        return "0x" + (rr + ss.to_bytes(32, "big") + bytes([vv - 27])).hex()

    def rzero(g):
        rr, ss, vv = parts(g)
        return "0x" + (bytes(32) + ss.to_bytes(32, "big") + bytes([vv])).hex()

    def highs(g):
        rr, ss, vv = parts(g)
        return "0x" + (rr + (N_SECP - ss).to_bytes(32, "big") + bytes([55 - vv])).hex()
    xr = 1
    while pow((xr ** 3 + 7) % P_SECP, (P_SECP - 1) // 2, P_SECP) == 1:
        xr += 1

    def norecover(g):
        rr, ss, vv = parts(g)
        return "0x" + (xr.to_bytes(32, "big") + ss.to_bytes(32, "big") + bytes([vv])).hex()
    EF("entry_form_sig_v", ENV(3, hp, "judgment", jbody(), sig=lowv), "step 13, s5.4: v 27 or 28", "E_SIG_V")
    EF("entry_form_sig_range", ENV(3, hp, "judgment", jbody(), sig=rzero), "step 13, s5.4: r in range", "E_SIG_RANGE")
    EF("entry_form_high_s", ENV(3, hp, "judgment", jbody(), sig=highs), "step 13, s5.4: s at most n/2", "E_SIG_HIGH_S")
    EF("entry_form_sig_recover", ENV(3, hp, "judgment", jbody(), sig=norecover), "step 13, s5.4: r the x of no curve point", "E_SIG_RECOVER")
    EF("entry_form_signer", ENV(3, hp, "judgment", jbody(), signer="other"), "step 13, s5.5: the signature recovers to another address", "E_SIG_SIGNER")
    # adjacent steps: each later fault waits for the earlier one
    EF("entry_form_sigform_before_place", ENV(3, hp, "genesis", {"statement_md": "x"}, sig=lambda g: "0x12"), "steps 10 before 11", "E_SIG_FORM")
    EF("entry_form_place_before_body", ENV(3, hp, "genesis", {}), "steps 11 before 12", "E_GENESIS_PLACE")
    EF("entry_form_body_before_sig", ENV(3, hp, "judgment", jbody(verdict_md=1), signer="other"), "steps 12 before 13", "E_BODY_FIELD")
    # the adjacent pairs the galeed/1 check set leaves unwitnessed (s7.1)
    EF("entry_form_closed_before_spec", ENV(3, hp, "judgment", jbody(), spec="galeed/2", extra={"note": "x"}), "step 3's closed test before step 4", "E_ENVELOPE_CLOSED")
    EF("entry_form_prevseq_before_body", ENV(3, None, "judgment", []), "step 8's prev and seq coupling before step 9", "E_PREV_SEQ")
    # ---- the entry test measured by galeed/1's own witnesses (s7.1): every input of the galeed/1 corpus's check set
    # that passes galeed/1 s3.5 as the entry file of an otherwise whole dossier, predicted as that corpus predicts it
    # (a token of s4.3 carried; an accepted entry going on to s3.1's type and subject tests), and one input failing
    # s3.5's sixth test for the step-1 token
    seen_nc = False
    for gp, gb, gnote, gcanon, gcheck in G.galeed_canon_inputs():
        if gcanon != "ok":
            continue
        if gcheck == "E_NOT_CANONICAL":
            if not seen_nc:
                seen_nc = True
                EF("entry_not_canonical", gb, "galeed/1 corpus %s (%s): the step-1 token" % (gp, gnote), "E_NOT_CANONICAL")
            continue
        if gcheck != "ok":
            EF("gl1_" + gp, gb, "galeed/1 corpus %s (%s)" % (gp, gnote), gcheck)
            continue
        ge = json.loads(gb.decode("utf-8"))
        if ge["entryType"] not in ("judgment", "resignation"):
            gpred = fault("E_DOSSIER_ENTRY_TYPE")
        else:
            gsr = ge["body"]["subjectRef"]
            assert not (gsr.get("kind") == "evm" and gsr.get("chainId") == 1 and gsr.get("contract") == ov["instance"]
                        and gsr.get("codeHash") == ch and gsr.get("id") == ov["salt"]), gp
            gpred = fault("E_DOSSIER_SUBJECT")
        ixg = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", gb)}), entry_id=sha(gb))
        D("gl1_" + gp, ixg, fs({"entry.bin": gb}), "galeed/1 corpus %s (%s): an accepted entry goes on to s3.1's type and subject tests" % (gp, gnote), gpred)
    eo = judgment("judge", 3, H32(), 1, addr("other-instance"), ch, ov["salt"], amanah)
    ixo = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", eo)}), entry_id=sha(eo))
    D("subject_contract", ixo, fs({"entry.bin": eo}), "s3.1: E_DOSSIER_SUBJECT, another contract", fault("E_DOSSIER_SUBJECT"))
    eo = judgment("judge", 3, H32(), 1, ov["instance"], ch, ov["salt"], amanah, seat=1)
    ixo = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", eo)}), entry_id=sha(eo))
    D("subject_seat", ixo, fs({"entry.bin": eo}), "s3.1: E_DOSSIER_SUBJECT, a judgment's seat differs", fault("E_DOSSIER_SUBJECT"))
    eo = judgment("judge", 3, H32(), 5, ov["instance"], ch, ov["salt"], amanah)
    ixo = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", eo)}), entry_id=sha(eo))
    D("subject_chain", ixo, fs({"entry.bin": eo}), "s3.1: E_DOSSIER_SUBJECT, another chain", fault("E_DOSSIER_SUBJECT"))
    eo = judgment("judge", 3, H32(), 1, ov["instance"], H32(), ov["salt"], amanah)
    ixo = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", eo)}), entry_id=sha(eo))
    D("subject_codehash", ixo, fs({"entry.bin": eo}), "s3.1: E_DOSSIER_SUBJECT, another code hash", fault("E_DOSSIER_SUBJECT"))
    eo = judgment("judge", 3, H32(), 1, ov["instance"], ch, H32(), amanah)
    ixo = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", eo)}), entry_id=sha(eo))
    D("subject_id", ixo, fs({"entry.bin": eo}), "s3.1: E_DOSSIER_SUBJECT, another case id", fault("E_DOSSIER_SUBJECT"))
    body = {"subjectRef": {"kind": "evm", "chainId": 1, "contract": ov["instance"], "codeHash": ch, "id": ov["salt"]},
            "amanah": amanah, "verdict_md": "x", "decision": {"rung": 1}, "evidence": []}
    ens = G.entry("judge", 3, H32(), "judgment", body)
    ixns = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", ens)}), entry_id=sha(ens))
    D("subject_judgment_no_seat", ixns, fs({"entry.bin": ens}), "s3.1: a judgment's subjectRef carries seat", fault("E_DOSSIER_SUBJECT"))
    body["subjectRef"]["seat"] = "0"
    ess = G.entry("judge", 3, H32(), "judgment", body)
    ixss = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", ess)}), entry_id=sha(ess))
    D("subject_seat_string", ixss, fs({"entry.bin": ess}), "s3.1: seat an int", fault("E_DOSSIER_SUBJECT"))
    eok = judgment("stranger", 3, H32(), 1, ov["instance"], ch, ov["salt"], H32(), rung=1)
    ixs = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", eok)}), entry_id=sha(eok))
    D("ok_entry_other_judge", ixs, fs({"entry.bin": eok}), "s3.1: whose key signed the entry and which word it names are s5.1's; the files pass", okd(ixs, r, listing, sha(eok)))
    # adjacency through s3.1
    bad = copy(listing); bad["order"]["nonce"] = "7"
    bb = cbytes(bad)
    f = refs({"listing": ("listing.json", bb), "receipt": ("receipt.json", b"[]")})
    D("adj_listing_before_hash", dict(ix, files=f), {"listing.json": bb, "receipt.json": b"[]"}, "s3.1 order: the listing's document test before the posted hash", fault("E_DOSSIER_LISTING", inner=inner("E_LISTING_NONCE")))
    f = refs({"listing": ("listing.json", lb), "receipt": ("receipt.json", b"[]")})
    D("adj_hash_before_receipt", dict(ix, files=f, posted=dict(ix["posted"], hash=H32())), {"listing.json": lb, "receipt.json": b"[]"}, "s3.1 order: the posted hash before the receipt", fault("E_DOSSIER_HASH"))
    rbad = cbytes(dict(r, termsHash=H32()))
    f = refs({"listing": ("listing.json", lb), "receipt": ("receipt.json", b"[]")})
    D("adj_receipt_before_order", dict(ix, files=f, chainId=5), {"listing.json": lb, "receipt.json": b"[]"}, "s3.1 order: the receipt before its binding", fault("E_DOSSIER_RECEIPT", inner=inner("E_RECEIPT")))
    f = refs({"listing": ("listing.json", lb), "receipt": ("receipt.json", rbad)})
    D("adj_order_before_chain", dict(ix, files=f, chainId=5), {"listing.json": lb, "receipt.json": rbad}, "s3.1 order: the binding before the chain", fault("E_DOSSIER_ORDER", inner=inner("RECEIPT_TERMS_MISMATCH")))
    D("adj_chain_before_logic", dict(ix, chainId=5, logic=addr("x")), fs(), "s3.1 order", fault("E_DOSSIER_CHAIN_MISMATCH"))
    D("adj_logic_before_codehash", dict(ix, logic=addr("x"), proxyCodeHash=H32()), fs(), "s3.1 order", fault("E_DOSSIER_LOGIC_MISMATCH"))
    D("adj_codehash_before_instance", dict(ix, proxyCodeHash=H32(), instance=addr("x")), fs(), "s3.1 order", fault("E_DOSSIER_CODEHASH_MISMATCH"))
    D("adj_instance_before_case", dict(ix, instance=addr("x"), caseId=H32()), fs(), "s3.1 order", fault("E_DOSSIER_INSTANCE_MISMATCH"))
    D("adj_case_before_seat", dict(ix, caseId=H32(), seat=dict(ix["seat"], judge=addr("other"))), fs(), "s3.1 order", fault("E_DOSSIER_CASE_MISMATCH"))
    D("adj_seat_before_entry", dict(ix, seat=dict(ix["seat"], judge=addr("other"), entryId=eid)), fs(), "s3.1 order", fault("E_DOSSIER_SEAT_MISMATCH"))
    D("adj_entry_before_digest", dict(ixe, seat=dict(ixe["seat"], entryId=H32()), files=refs(base_files)), fs(), "s3.1 order: the presence rule before the digest", fault("E_DOSSIER_ENTRY"))
    D("adj_digest_before_form", dict(ixj, seat=dict(ixj["seat"], entryId=H32())), fs({"entry.bin": junk}), "s3.1 order: the entry's digest before its form", fault("E_DOSSIER_ENTRY_DIGEST"))
    D("adj_form_before_type", ixj, fs({"entry.bin": junk}), "s3.1 order: form before type (a form fault)", fault("E_DOSSIER_ENTRY_FORM", inner=inner("E_ENVELOPE_MISSING")))
    ann2 = G.entry("judge", 5, H32(), "annotation", {"note_md": "x"})
    ixa2 = index_of(listing, lb, ov, r, refs({**base_files, "entry": ("entry.bin", ann2)}), entry_id=sha(ann2), terminal=term)
    D("adj_type_before_subject", ixa2, fs({"entry.bin": ann2}), "s3.1 order: type before subject", fault("E_DOSSIER_ENTRY_TYPE"))
    # ---- round 2: the boundaries the README claimed on one side alone, and the seat's presence rules
    term_r = {"name": "Refunded", "buyerPaid": "1000000000", "sellerPaid": "160000000", "judgePaid": "0", "bondToBuyer": "0"}
    ixr = index_of(listing, lb, ov, r, refs(base_files), terminal=term_r)
    D("ok_refunded", ixr, fs(), "s2.4: a Refunded terminal, the deposit back to the seller", okd(ixr, r, listing))
    term_st = {"name": "Settled", "buyerPaid": "500000000", "sellerPaid": "650000000", "judgePaid": "10000000", "bondToBuyer": "0"}
    ixs = index_of(listing, lb, ov, r, refs(base_files), terminal=term_st)
    D("ok_settled", ixs, fs(), "s2.4: a Settled terminal, P to the judge", okd(ixs, r, listing))
    term_m = {"name": "Released", "buyerPaid": str(2 ** 256 - 1), "sellerPaid": "0", "judgePaid": "0", "bondToBuyer": "0"}
    ixm = index_of(listing, lb, ov, r, refs(base_files), terminal=term_m)
    D("ok_terminal_amount_max", ixm, fs(), "s1: dec256 at 2^256 - 1 passes", okd(ixm, r, listing))
    d = dict(ixn); del d["transactions"]
    D("missing_transactions", d, fs(), "s2.1: transactions is required", fault("E_DOSSIER_MISSING"))
    D("compliance_conversion_source_empty", dict(ixn, compliance={"conversions": [dict(conv_n, source="")]}), fs(), "s2.5: source non-empty, index 0", fault("E_DOSSIER_COMPLIANCE", 0))
    D("compliance_conversion_rate_int", dict(ixn, compliance={"conversions": [dict(conv_n, rate=1)]}), fs(), "s2.5: rate a string, index 0", fault("E_DOSSIER_COMPLIANCE", 0))
    f = refs({**base_files, "entry": ("entry.bin", ej)})
    D("file_entry_unread_without_entry_id", dict(ixn, files=f), fs(), "s3.1 order: every key's file is read before any document test; an entry the seat does not name is read all the same", fault("E_DOSSIER_FILE", key="entry"))
    rows = txrows(2); rows[0]["tx"] = "0x12"; rows[1]["tx"] = "0x34"
    D("transaction_two_faults", dict(ixn, transactions=rows), fs(), "s2.3: the first faulting row's index", fault("E_TRANSACTION", 0))
    ixa2a = index_of(lis_h, lhb, ov_h, r_h, refs(hfiles, [("goods.md", gt), ("goods2.md", gt)]))
    D("attachment_two_absent", ixa2a, {"listing.json": lhb, "receipt.json": rhb}, "s3.1 order: the first absent attachment's key", fault("E_DOSSIER_FILE", key="attachment:0"))
    ixa2u = index_of(lis_h, lhb, ov_h, r_h, refs(hfiles, [("a.md", b"aaa"), ("b.md", b"bbb")]))
    D("attachment_two_unnamed", ixa2u, {"listing.json": lhb, "receipt.json": rhb, "a.md": b"aaa", "b.md": b"bbb"}, "s3.1: the first unnamed attachment's index", fault("E_DOSSIER_ATTACHMENT", 0))
    D("seat_amanah_short", dict(ixn, seat=dict(ixn["seat"], amanahHash="0x1")), fs(), "s2.2: amanahHash hex32", fault("E_DOSSIER_SEAT"))
    ixge = index_of(listing, lb, ovg, rg, refs({**gfiles, "entry": ("entry.bin", eg)}), entry_id=sha(eg), terminal=term, seat={"seat": 0})
    del ixge["seat"]["judge"]
    D("seat_no_judge_with_entry", ixge, {"listing.json": lb, "receipt.json": rgb, "entry.bin": eg}, "s2.2: where judge is absent, entryId is absent (nobody anchored); seat 0 passes the other half, so the entryId alone refuses", fault("E_DOSSIER_SEAT"))
    ixg1 = index_of(listing, lb, ovg, rg, refs(gfiles), seat={"seat": 1})
    del ixg1["seat"]["judge"]
    D("seat_no_judge_seat_1", ixg1, {"listing.json": lb, "receipt.json": rgb}, "s2.2: where judge is absent, seat is 0", fault("E_DOSSIER_SEAT"))
    ixgr = index_of(listing, lb, ovg, rg, refs(gfiles), seat={"seat": 0}, terminal=term)
    del ixgr["seat"]["judge"]
    D("ruled_without_judge", ixgr, {"listing.json": lb, "receipt.json": rgb}, "s2.4: Ruled requires the seat's judge and entryId", fault("E_DOSSIER_RULED"))
    C.case("check", ["dossiers/absent"], "HARNESS: a path that is no directory", None)
    pfile = write_dossier("not_a_dir", None, {"f": b"x"})
    C.case("check", [pfile + "/f"], "HARNESS: a regular file as <dir> is misuse", None, path=pfile + "/f")
    C.case("check", ["dossiers/d001_ok_ruled", "x"], "HARNESS: an argument beyond the list", None, path="dossiers/d001_ok_ruled")
    C.case("check", [], "HARNESS: no path", None, path="(none)")
    C.argv_case([], "HARNESS: no command", None)
    C.argv_case(["bogus", "dossiers/d001_ok_ruled"], "HARNESS: an unknown command", None, path="dossiers/d001_ok_ruled")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=1)
    a = ap.parse_args()
    G.seed(a.seed)
    global C
    C = G.Corpus(HERE, ["dossiers"])
    gen()
    n = C.finish(a.seed)
    print("dossier corpus: %d cases" % n)


if __name__ == "__main__":
    main()
