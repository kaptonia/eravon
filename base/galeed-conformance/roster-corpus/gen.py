#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
gen.py - corpus generator for the `galeed.roster/1` convergence run.

Written from docs/galeed-roster-v1.md and galeed-conformance/HARNESS-ROSTER.md.
Every case carries a `predicted` answer written by hand from the text (or
`null` where the case is harness misuse and the expected outcome is exit 2
with no output); the prediction is a third opinion beside the criterion and
the implementations under test, never authoritative. keccak256 is taken from
the galeed/1 core (`../impl-py/glcrypto.py`), the only thing borrowed from it;
sha256 is the standard library's.

Usage:  python3 gen.py --seed 1
Populates ./pubs, ./logs, ./bench, ./query, ./quotes and writes manifest.json.
The logs set carries the succession book's declarations and accessions
(s4) beside the registry's publications and statuses.
"""
import argparse
import hashlib
import json
import os
import random
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.dont_write_bytecode = True  # the criteria's trees stay as committed
sys.path.insert(0, os.path.join(HERE, "..", "impl-py"))
from glcrypto import keccak256  # noqa: E402

KR, KC, KJ, KG = "galeed.roster/1", "galeed.guildcard/1", "galeed.judge/1", "galeed.guildamanah/1"
ROSTER_TABLE = ("kind", "guild", "chainId", "issued", "judges", "bench")
CARD_TABLE = ("kind", "guild", "issued", "name", "home", "note_md")
MANIFEST = []
RNG = random.Random(1)
ADDR = {}
_n = {"pubs": 0, "logs": 0, "bench": 0, "query": 0, "quotes": 0}


def A(tag):
    """A deterministic lowercase hex20 for a tag, drawn from the seed."""
    if tag not in ADDR:
        ADDR[tag] = "0x" + "".join("%02x" % RNG.randrange(256) for _ in range(20))
    return ADDR[tag]


def H32():
    return "0x" + "".join("%02x" % RNG.randrange(256) for _ in range(32))


def hx(b):
    return "0x" + b.hex()


def sha(s):
    return "0x" + hashlib.sha256(s.encode("utf-8")).hexdigest()


def jbytes(d):
    """Document bytes: compact JSON, member order as given, non-ASCII raw."""
    return json.dumps(d, ensure_ascii=False, separators=(",", ":")).encode("utf-8")


def pub(emitter, block, idx, doc_bytes, word=None):
    """A publication element in the harness's file form."""
    w = hx(keccak256(doc_bytes)) if word is None else word
    return {"emitter": emitter, "blockNumber": block, "logIndex": idx,
            "document": hx(doc_bytes), "word": w}


def status(emitter, block, idx, active):
    return {"emitter": emitter, "blockNumber": block, "logIndex": idx, "active": active}


def write(cat, slug, data):
    _n[cat] += 1
    name = "%s%03d_%s.json" % (cat[0] if cat != "quotes" else "t", _n[cat], slug)
    path = os.path.join(cat, name)
    with open(os.path.join(HERE, path), "wb") as f:
        f.write(data if isinstance(data, bytes) else json.dumps(data, ensure_ascii=False).encode("utf-8"))
    return path


def case(cmd, args, note, predicted, path=None, absent=None):
    e = {"cmd": cmd, "args": args, "path": path or args[0], "note": note, "predicted": predicted}
    if absent:
        e["absent"] = absent  # the corpus paths the case names that no file carries
    MANIFEST.append(e)


def argv_case(argv, note, predicted, path):
    MANIFEST.append({"argv": argv, "path": path, "note": note, "predicted": predicted})


BASIS = {"chains": [{"chainId": 1, "fromBlock": 0, "toBlock": 1000, "registries": [], "senders": []}]}


def roster(guild, chain=1, issued=10, judges=None, **kw):
    d = {"kind": KR, "guild": guild, "chainId": chain, "issued": issued, "judges": judges or []}
    d.update(kw)
    return d


def card(guild, issued=10, **kw):
    d = {"kind": KC, "guild": guild, "issued": issued}
    d.update(kw)
    return d


def jdoc(judge, entry_id=None, root=None, bas=None, issued=10, **kw):
    d = {"kind": KJ, "judge": judge, "issued": issued, "amanah": {"entry_id": entry_id or H32()},
         "ledger": {"root": root or judge, "basis": BASIS if bas is None else bas}}
    d.update(kw)
    return d


def gdoc(guild, chain=1, issued=10, version=1, text="the guild's covenant", **kw):
    d = {"kind": KG, "guild": guild, "chainId": chain, "issued": issued, "version": version, "text_md": text}
    d.update(kw)
    return d


def roster_answer(r, unknown=()):
    """The decode answer of a roster; every root member beyond s3.1's table
    (expires, name, and terms among them since 2026-10-23) is an unknown member."""
    extra = set(unknown) | set(k for k in r if k not in ROSTER_TABLE)
    out = {"guild": r["guild"], "chainId": r["chainId"], "issued": r["issued"],
           "judges": [e if isinstance(e, str) else e["addr"] for e in r["judges"]],
           "unknownFields": sorted(extra, key=lambda k: k.encode("utf-8"))}
    if "bench" in r:
        out["bench"] = {"protocols": list(r["bench"]["protocols"]),
                        "verifiers": [vview(e) for e in r["bench"]["verifiers"]]}
    return {"kind": KR, "known": True, "ok": True, "roster": out}


def judge_view(j, unknown=()):
    out = {"judge": j["judge"], "issued": j["issued"], "amanah": {"entry_id": j["amanah"]["entry_id"]},
           "ledger": {"basis": j["ledger"]["basis"], "root": j["ledger"]["root"]},
           "unknownFields": sorted(unknown, key=lambda k: k.encode("utf-8"))}
    for k in ("name", "home", "note_md"):
        if k in j:
            out[k] = j[k]
    return out


def judge_answer(j, unknown=()):
    return {"judge": judge_view(j, unknown), "kind": KJ, "known": True, "ok": True}


def covenant_view(g, unknown=()):
    out = {"guild": g["guild"], "chainId": g["chainId"], "issued": g["issued"], "version": g["version"],
           "text_md": g["text_md"], "sha256": sha(g["text_md"]),
           "unknownFields": sorted(unknown, key=lambda k: k.encode("utf-8"))}
    if "venues" in g:
        out["venues"] = [{"chainId": e["chainId"], "codeHash": e["codeHash"], "contract": e["contract"]} for e in g["venues"]]
    if "quotes" in g:
        out["quotes"] = g["quotes"]
    return out


def covenant_answer(g, unknown=()):
    return {"guildamanah": covenant_view(g, unknown), "kind": KG, "known": True, "ok": True}


def cov_resolved(g, p):
    """The covenant member of a guild answer / the `gro covenant` answer."""
    out = {"blockNumber": p["blockNumber"], "sha256": sha(g["text_md"]), "version": g["version"], "word": p["word"]}
    if "venues" in g:
        out["venues"] = covenant_view(g)["venues"]
    if "quotes" in g:
        out["quotes"] = g["quotes"]
    return out


def vview(e):
    """The decode answer's view of a verifiers element (HARNESS-ROSTER)."""
    if isinstance(e, str):
        return {"addr": e}
    return {"addr": e["addr"], "name": e["name"], "protocols": list(e["protocols"])}


def vrow(e, final=None, trail=(), token=None):
    """A verifiers row of a resolve-guild answer (s5.4 step 8)."""
    addr = e if isinstance(e, str) else e["addr"]
    row = {"listed": addr}
    if not isinstance(e, str):
        row["name"] = e["name"]
        row["protocols"] = list(e["protocols"])
    if token is None:
        row["final"] = final
    else:
        row["token"] = token
    row["trail"] = list(trail)
    return row


def card_view(c):
    out = {"guild": c["guild"], "issued": c["issued"],
           "unknownFields": sorted((k for k in c if k not in CARD_TABLE), key=lambda k: k.encode("utf-8"))}
    for k in ("name", "home", "note_md"):
        if k in c:
            out[k] = c[k]
    return out


def card_answer(c):
    return {"guildcard": card_view(c), "kind": KC, "known": True, "ok": True}


def fault(token):
    return {"ok": False, "token": token}


# ==========================================================================
# decode
# ==========================================================================

def gen_decode():
    g, j1, j2, v1, v2 = A("g"), A("j1"), A("j2"), A("v1"), A("v2")

    def D(slug, p, note, predicted):
        path = write("pubs", slug, p)
        case("decode", [path], note, predicted)

    # ---- accepted documents
    r = roster(g)
    D("roster_minimal", pub(g, 5, 0, jbytes(r)), "the four required members and an empty judges array", roster_answer(r))
    r = roster(g, judges=[j1, {"addr": j2, "since": 7, "role": "x"}],
               bench={"verifiers": [v1, {"addr": v2, "name": "v two", "protocols": [H32()], "tag": 1}], "protocols": [H32(), H32()], "extra": True})
    r["extra"] = "kept"
    r["alpha"] = [1, 2]
    D("roster_full", pub(g, 5, 0, jbytes(r)), "every optional member, both element forms, extension members at root, in bench and in a verifier object", roster_answer(r, ("extra", "alpha")))
    r = roster(g, expires=99, name="第一版 guild", terms="terms\nwith newline", judges=[j1])
    D("roster_expires_name_terms_unknown", pub(g, 5, 0, jbytes(r)), "s3.1: expires, name, and terms are no members of the table; a roster that writes them carries them as members beyond it, unread", roster_answer(r))
    s = {"kind": "galeed.succession/1", "from": j1, "to": j2, "chainId": 1, "issued": 1}
    D("succession_record_unknown_kind", pub(j1, 5, 0, jbytes(s)), "s2.1 step 4: galeed.succession/1 is no kind of this law; an unknown kind, nothing else read", {"kind": "galeed.succession/1", "known": False, "ok": True})
    D("succession_prefix_not_reserved", pub(j1, 5, 0, jbytes({"kind": "galeed.succession/2"})), "s1: galeed.succession/ is no reserved prefix; an unknown kind", {"kind": "galeed.succession/2", "known": False, "ok": True})
    D("unknown_kind_v2", pub(g, 5, 0, jbytes({"kind": "galeed.roster/2", "guild": g})), "s2.1 step 4: an unknown kind is no fault and nothing else is read", {"kind": "galeed.roster/2", "known": False, "ok": True})
    D("unknown_kind_empty", pub(g, 5, 0, jbytes({"kind": ""})), "the empty string is an unknown kind", {"kind": "", "known": False, "ok": True})
    D("unknown_kind_verifier", pub(g, 5, 0, jbytes({"kind": "galeed.verifier/1", "verifier": g, "serves": [], "at": 1})), "a service record is a kind of another law: unknown here, its members not read", {"kind": "galeed.verifier/1", "known": False, "ok": True})
    D("unknown_kind_other_family", pub(g, 5, 0, jbytes({"kind": "foo/1", "x": 1})), "a kind under no reserved prefix is an unknown kind", {"kind": "foo/1", "known": False, "ok": True})
    D("kind_absent", pub(g, 5, 0, jbytes({"guild": g})), "s2.1 step 3", fault("E_KIND"))
    D("kind_int", pub(g, 5, 0, jbytes({"kind": 1})), "s2.1 step 3: kind not a string", fault("E_KIND"))
    D("kind_null", pub(g, 5, 0, jbytes({"kind": None})), "s2.1 step 3: kind null", fault("E_KIND"))
    D("kind_case", pub(g, 5, 0, jbytes({"kind": "Galeed.Roster/1", "guild": g})), "kind compares bytewise: a case variant is an unknown kind", {"kind": "Galeed.Roster/1", "known": False, "ok": True})
    D("empty_object", pub(g, 5, 0, jbytes({})), "an object without kind", fault("E_KIND"))
    for slug, k in (("kind_bare_prefix", "galeed.roster/"), ("kind_version_zero", "galeed.roster/0"), ("kind_version_leading_zero", "galeed.roster/01"), ("kind_trailing_space", "galeed.roster/1 "), ("kind_no_slash", "galeed.roster")):
        D(slug, pub(g, 5, 0, jbytes({"kind": k, "guild": g})), "s1: a kind string other than the literal decodes as an unknown kind; whether it is of the roster kind (under the prefix) or of another kind is read by s5", {"kind": k, "known": False, "ok": True})


    # ---- hash
    doc = jbytes(roster(g))
    D("hash_mismatch", pub(g, 5, 0, doc, word=H32()), "s2.1 step 1", fault("E_HASH"))
    D("hash_of_other_bytes", pub(g, 5, 0, doc, word=hx(keccak256(doc + b"\n"))), "s2.1 step 1: the word is over the document bytes exactly", fault("E_HASH"))
    D("hash_before_json", pub(g, 5, 0, b"not json", word=H32()), "step 1 is decided before step 2", fault("E_HASH"))
    D("hash_empty_document", pub(g, 5, 0, b""), "the empty document: step 1 passes for its keccak, step 2 fails: nothing to read", fault("E_JSON"))
    D("hash_uppercase_word", pub(g, 5, 0, doc, word=hx(keccak256(doc)).upper().replace("0X", "0x")), "HARNESS: word outside hex32 is misuse", None)

    # ---- the reader
    D("json_invalid_utf8", pub(g, 5, 0, b'{"kind":"\xff"}'), "the reader: invalid UTF-8", fault("E_JSON"))
    D("json_root_array", pub(g, 5, 0, b'[]'), "s2.1 step 2: root not an object", fault("E_JSON"))
    D("json_root_string", pub(g, 5, 0, b'"galeed.roster/1"'), "s2.1 step 2: root a string", fault("E_JSON"))
    D("json_dup_key", pub(g, 5, 0, b'{"kind":"galeed.roster/1","kind":"galeed.roster/1"}'), "the reader: a repeated member name", fault("E_JSON"))
    D("json_dup_key_escaped", pub(g, 5, 0, b'{"kind":"galeed.roster/1","k\\u0069nd":"x"}'), "the reader: repeated after escape decoding", fault("E_JSON"))
    D("json_float", pub(g, 5, 0, b'{"kind":"galeed.roster/1","guild":"%s","chainId":1.0,"issued":1,"judges":[]}' % g.encode()), "the reader: a fraction", fault("E_JSON"))
    D("json_negative", pub(g, 5, 0, b'{"kind":"galeed.roster/1","guild":"%s","chainId":-1,"issued":1,"judges":[]}' % g.encode()), "the reader: a sign", fault("E_JSON"))
    D("json_leading_zero", pub(g, 5, 0, b'{"kind":"galeed.roster/1","guild":"%s","chainId":01,"issued":1,"judges":[]}' % g.encode()), "the reader: a leading zero", fault("E_JSON"))
    D("json_two_pow_53", pub(g, 5, 0, b'{"kind":"galeed.roster/1","guild":"%s","chainId":1,"issued":9007199254740992,"judges":[]}' % g.encode()), "the reader: 2^53 is outside the universe", fault("E_JSON"))
    r = roster(g, issued=9007199254740991)
    D("json_max_int", pub(g, 5, 0, jbytes(r)), "the reader: 2^53 - 1 is inside", roster_answer(r))
    D("json_exponent", pub(g, 5, 0, b'{"kind":"galeed.roster/1","guild":"%s","chainId":1e0,"issued":1,"judges":[]}' % g.encode()), "the reader: an exponent", fault("E_JSON"))
    D("json_lone_surrogate", pub(g, 5, 0, b'{"kind":"galeed.roster/1","guild":"%s","chainId":1,"issued":1,"judges":[],"name":"\\ud800"}' % g.encode()), "the reader: a lone surrogate escape", fault("E_JSON"))
    r = roster(g, name="\U0001F600 pair")
    D("json_surrogate_pair", pub(g, 5, 0, b'{"kind":"galeed.roster/1","guild":"%s","chainId":1,"issued":10,"judges":[],"name":"\\ud83d\\ude00 pair"}' % g.encode()), "the reader: a pair reads as one scalar value; the answer spells it raw", roster_answer(r))
    r = roster(g, name="tab\there \x01 ctl")
    D("json_prose_controls", pub(g, 5, 0, b'{"kind":"galeed.roster/1","guild":"%s","chainId":1,"issued":10,"judges":[],"name":"tab\\there \\u0001 ctl"}' % g.encode()), "the reader places no charset on values: control scalars in a name are read", roster_answer(r))
    deep = "[" * 128 + "]" * 128
    D("json_depth_129", pub(g, 5, 0, b'{"kind":"galeed.roster/1","guild":"%s","chainId":1,"issued":10,"judges":[],"x":%s}' % (g.encode(), deep.encode())), "the reader: the root object is depth 1, so 128 nested arrays open depth 129", fault("E_JSON"))
    deep127 = "[" * 127 + "]" * 127
    D("json_depth_128", pub(g, 5, 0, b'{"kind":"galeed.roster/1","guild":"%s","chainId":1,"issued":10,"judges":[],"x":%s}' % (g.encode(), deep127.encode())), "the reader: depth 128 is readable; x is an extension member", roster_answer(roster(g), ("x",)))
    D("json_bom", pub(g, 5, 0, b'\xef\xbb\xbf' + doc), "the reader: a byte order mark is unreadable", fault("E_JSON"))
    D("json_nan", pub(g, 5, 0, b'{"kind":"galeed.roster/1","guild":"%s","chainId":NaN,"issued":1,"judges":[]}' % g.encode()), "the reader: NaN", fault("E_JSON"))
    D("json_trailing_garbage", pub(g, 5, 0, doc + b"x"), "the reader: not one JSON text", fault("E_JSON"))
    pretty = json.dumps(roster(g), indent=2).encode("utf-8") + b"\n"
    D("json_pretty_printed", pub(g, 5, 0, pretty), "the reader does not require canonical form: whitespace and a trailing newline are read", roster_answer(roster(g)))
    D("json_ws_around", pub(g, 5, 0, b"  " + doc + b"\r\n"), "the reader: RFC 8259 whitespace around the text", roster_answer(roster(g)))
    r = roster(g, name="café 中")
    D("json_raw_non_ascii", pub(g, 5, 0, jbytes(r)), "the document bytes are raw UTF-8; no transport encoding stands between the chain and the reader", roster_answer(r))

    # ---- the reader: more faults
    D("json_utf8_overlong", pub(g, 5, 0, b'{"kind":"\xc0\x80"}'), "the reader: an overlong UTF-8 sequence", fault("E_JSON"))
    D("json_utf8_encoded_surrogate", pub(g, 5, 0, b'{"kind":"\xed\xa0\x80"}'), "the reader: an encoded surrogate", fault("E_JSON"))
    D("json_utf8_above_max", pub(g, 5, 0, b'{"kind":"\xf4\x90\x80\x80"}'), "the reader: a sequence above U+10FFFF", fault("E_JSON"))
    D("json_utf8_truncated", pub(g, 5, 0, b'{"kind":"\xe4\xb8"}'), "the reader: a sequence truncated at the end of a string", fault("E_JSON"))
    D("json_high_surrogate_then_letter", pub(g, 5, 0, b'{"kind":"\\ud800A"}'), "the reader: a high surrogate escape not followed by a low one", fault("E_JSON"))
    D("json_lone_low_surrogate", pub(g, 5, 0, b'{"kind":"\\udc00"}'), "the reader: a lone low surrogate escape", fault("E_JSON"))
    D("json_two_high_surrogates", pub(g, 5, 0, b'{"kind":"\\ud800\\ud800"}'), "the reader: two high surrogate escapes in a row", fault("E_JSON"))
    D("json_negative_zero", pub(g, 5, 0, b'{"kind":"galeed.roster/1","guild":"%s","chainId":-0,"issued":1,"judges":[]}' % g.encode()), "the reader: -0 carries a sign", fault("E_JSON"))
    D("json_exponent_upper", pub(g, 5, 0, b'{"kind":"galeed.roster/1","guild":"%s","chainId":1E2,"issued":1,"judges":[]}' % g.encode()), "the reader: an uppercase exponent", fault("E_JSON"))
    D("json_eighteen_digits", pub(g, 5, 0, b'{"kind":"galeed.roster/1","guild":"%s","chainId":1,"issued":100000000000000000,"judges":[]}' % g.encode()), "the reader: eighteen digits are above 2^53 - 1", fault("E_JSON"))

    # ---- the judge document's basis at galeed/1 s9.4's edges
    def CH(cid=1, fb=0, tb=10, regs=(), snd=(), **kw):
        d = {"chainId": cid, "fromBlock": fb, "toBlock": tb, "registries": list(regs), "senders": list(snd)}
        d.update(kw)
        return d
    HC = "0x" + "ab" * 32
    j_lo, j_hi = sorted([j1, j2], key=lambda s: s.encode())
    for slug, chains, ok, note in (
            ("basis_from_equals_to", [CH(fb=5, tb=5)], True, "fromBlock equal to toBlock is a window of one block"),
            ("basis_touching_differ_senders", [CH(tb=10), CH(fb=11, tb=20, snd=[j1])], True, "touching windows that differ in senders"),
            ("basis_touching_differ_registries", [CH(tb=10), CH(fb=11, tb=20, regs=[v1])], True, "touching windows that differ in registries"),
            ("basis_touching_equal_lists", [CH(tb=10), CH(fb=11, tb=20)], False, "touching windows with equal lists"),
            ("basis_gap_one_block_equal_lists", [CH(tb=10), CH(fb=12, tb=20)], True, "a gap of one block between equal windows"),
            ("basis_max_ints", [CH(cid=9007199254740991, fb=0, tb=9007199254740991)], True, "chainId and toBlock at 2^53 - 1"),
            ("basis_registries_address_then_class", [CH(regs=[v1, {"codeHash": HC}])], True, "an address item then a class item"),
            ("basis_overlap_one_block", [CH(tb=10), CH(fb=10, tb=20, snd=[j1])], False, "windows overlapping at one block"),
            ("basis_nested_window", [CH(tb=20), CH(fb=5, tb=10, snd=[j1])], False, "a window inside another overlaps it"),
            ("basis_class_codehash_short", [CH(regs=[{"codeHash": "0x12"}])], False, "a class item whose codeHash is not hex32"),
            ("basis_class_codehash_upper", [CH(regs=[{"codeHash": "0x" + "AB" * 32}])], False, "a class item whose codeHash carries upper-case digits is not hex32"),
            ("basis_class_codehash_hex20", [CH(regs=[{"codeHash": v1}])], False, "a class item whose codeHash is a hex20 is not hex32"),
            ("basis_class_codehash_int", [CH(regs=[{"codeHash": 7}])], False, "a class item whose codeHash is an int is not hex32"),
            ("basis_registries_not_array", [CH(registries={"codeHash": HC})], False, "registries that is not an array"),
            ("basis_senders_not_array", [CH(senders=j1)], False, "senders that is not an array"),
            ("basis_sender_not_hex20", [CH(snd=["0x12"])], False, "a senders element outside hex20"),
            ("basis_equal_windows_two_chains", [CH(cid=1, tb=10), CH(cid=2, fb=0, tb=10)], True, "equal windows on two chains are two chains and need not differ"),
            ("basis_equal_from", [CH(tb=10), CH(fb=0, tb=20, snd=[j1])], False, "two windows with equal fromBlock"),
            ("basis_chains_out_of_order", [CH(cid=2), CH(cid=1)], False, "chains out of order by chainId"),
            ("basis_duplicate_sender", [CH(snd=[j1, j1])], False, "a repeated sender"),
            ("basis_senders_out_of_order", [CH(snd=[j_hi, j_lo])], False, "senders out of bytewise order"),
            ("basis_class_before_address", [CH(regs=[{"codeHash": HC}, v1])], False, "a class item before an address item"),
            ("basis_class_out_of_order", [CH(regs=[{"codeHash": "0x" + "ff" * 32}, {"codeHash": "0x" + "00" * 32}])], False, "class items out of order"),
            ("basis_class_duplicate", [CH(regs=[{"codeHash": HC}, {"codeHash": HC}])], False, "a repeated class item"),
            ("basis_registry_hex32_string", [CH(regs=["0x" + "ab" * 32])], False, "a hex32 string is no registry item"),
            ("basis_class_extra_member", [CH(regs=[{"codeHash": HC, "x": 1}])], False, "a class item with a member beyond codeHash"),
            ("basis_chain_extra_member", [CH(x=1)], False, "a chains object with a member beyond the five"),
            ("basis_chain_missing_senders", [{"chainId": 1, "fromBlock": 0, "toBlock": 10, "registries": []}], False, "a chains object without senders"),
            ("basis_from_above_to", [CH(fb=11, tb=10)], False, "fromBlock above toBlock"),
            ("basis_block_not_int", [CH(fb="0")], False, "a block number that is no int")):
        jd = jdoc(j1, bas={"chains": chains})
        D(slug, pub(j1, 5, 0, jbytes(jd)), "galeed/1 s9.4 via s3a.1: " + note, judge_answer(jd) if ok else fault("E_JUDGE_LEDGER"))
    jd = jdoc(j1, bas={"chains": []})
    D("basis_empty_chains", pub(j1, 5, 0, jbytes(jd)), "galeed/1 s9.4 via s3a.1: a basis whose chains is empty is a basis", judge_answer(jd))

    # ---- the roster table, in decision order

    D("roster_guild_absent", pub(g, 5, 0, jbytes({"kind": KR, "chainId": 1, "issued": 1, "judges": []})), "s3.2: guild absent", fault("E_ROSTER_GUILD"))
    D("roster_guild_checksummed", pub(g, 5, 0, jbytes(roster("0x" + "".join(c.upper() if i % 3 == 0 else c for i, c in enumerate(g[2:]))))), "s1: no case normalization; a checksummed address is outside hex20", fault("E_ROSTER_GUILD"))
    D("roster_guild_short", pub(g, 5, 0, jbytes(roster(g[:-2]))), "s3.2: guild outside hex20", fault("E_ROSTER_GUILD"))
    D("roster_guild_int", pub(g, 5, 0, jbytes(roster(5))), "s3.2: guild not a string", fault("E_ROSTER_GUILD"))
    D("roster_guild_no_prefix", pub(g, 5, 0, jbytes(roster(g[2:] + "ab"))), "s3.2: 42 characters without 0x", fault("E_ROSTER_GUILD"))
    D("roster_chain_absent", pub(g, 5, 0, jbytes({"kind": KR, "guild": g, "issued": 1, "judges": []})), "s3.2: chainId absent", fault("E_ROSTER_CHAIN"))
    D("roster_chain_string", pub(g, 5, 0, jbytes(roster(g, chain="1"))), "s3.2: chainId a string", fault("E_ROSTER_CHAIN"))
    D("roster_issued_absent", pub(g, 5, 0, jbytes({"kind": KR, "guild": g, "chainId": 1, "judges": []})), "s3.2: issued absent", fault("E_ROSTER_ISSUED"))
    D("roster_issued_bool", pub(g, 5, 0, jbytes(roster(g, issued=True))), "s3.2: issued a bool", fault("E_ROSTER_ISSUED"))
    r = roster(g, expires=None)
    D("roster_expires_null", pub(g, 5, 0, jbytes(r)), "s3.1: expires is no member of the table; written null it is a member beyond the table, untested", roster_answer(r))
    r = roster(g, expires="99")
    D("roster_expires_string", pub(g, 5, 0, jbytes(r)), "s3.1: expires a string, untested", roster_answer(r))
    r = roster(g, name=3)
    D("roster_name_int", pub(g, 5, 0, jbytes(r)), "s3.1: name is no member of the table; an int there is untested", roster_answer(r))
    r = roster(g, name="")
    D("roster_name_empty", pub(g, 5, 0, jbytes(r)), "s3.1: name beyond the table", roster_answer(r))
    r = roster(g, terms=["a"])
    D("roster_terms_array", pub(g, 5, 0, jbytes(r)), "s3.1: terms is no member of the table; an array there is untested", roster_answer(r))
    D("roster_judges_absent", pub(g, 5, 0, jbytes({"kind": KR, "guild": g, "chainId": 1, "issued": 1})), "s3.2: judges absent", fault("E_ROSTER_JUDGES"))
    D("roster_judges_object", pub(g, 5, 0, jbytes({"kind": KR, "guild": g, "chainId": 1, "issued": 1, "judges": {}})), "s3.2: judges not an array", fault("E_ROSTER_JUDGES"))
    D("roster_judges_element_int", pub(g, 5, 0, jbytes(roster(g, judges=[j1, 7]))), "s3.2: an element outside both forms", fault("E_ROSTER_JUDGES"))
    D("roster_judges_element_no_addr", pub(g, 5, 0, jbytes(roster(g, judges=[{"since": 1}]))), "s3.1: an object element needs addr", fault("E_ROSTER_JUDGES"))
    D("roster_judges_element_addr_upper", pub(g, 5, 0, jbytes(roster(g, judges=[{"addr": j1.upper().replace("0X", "0x")}]))), "s3.1: addr outside hex20", fault("E_ROSTER_JUDGES"))
    D("roster_judges_element_null", pub(g, 5, 0, jbytes(roster(g, judges=[None]))), "s3.2: a null element", fault("E_ROSTER_JUDGES"))
    r = roster(g, judges=[j1, j1, {"addr": j1}])
    D("roster_judges_repeated", pub(g, 5, 0, jbytes(r)), "s3.1: a repeated address is one element per occurrence", roster_answer(r))
    r = roster(g, judges=[{"addr": j1, "since": "not an int", "x": None}])
    D("roster_judges_since_untested", pub(g, 5, 0, jbytes(r)), "s3.1: members of an element object beyond addr are untested", roster_answer(r))
    D("roster_bench_null", pub(g, 5, 0, jbytes(roster(g, bench=None))), "s3.2: bench written null", fault("E_ROSTER_BENCH"))
    D("roster_bench_array", pub(g, 5, 0, jbytes(roster(g, bench=[v1]))), "s3.2: bench an array", fault("E_ROSTER_BENCH"))
    D("roster_bench_no_protocols", pub(g, 5, 0, jbytes(roster(g, bench={"verifiers": []}))), "s3.1: protocols absent", fault("E_ROSTER_BENCH"))
    D("roster_bench_no_verifiers", pub(g, 5, 0, jbytes(roster(g, bench={"protocols": []}))), "s3.1: verifiers absent", fault("E_ROSTER_BENCH"))
    D("roster_bench_protocol_hex20", pub(g, 5, 0, jbytes(roster(g, bench={"verifiers": [], "protocols": [v1]}))), "s3.1: a protocol outside hex32", fault("E_ROSTER_BENCH"))
    D("roster_bench_verifier_no_name", pub(g, 5, 0, jbytes(roster(g, bench={"verifiers": [{"addr": v1, "protocols": []}], "protocols": []}))), "s3.1: a verifier object needs name", fault("E_ROSTER_BENCH"))
    D("roster_bench_verifier_name_int", pub(g, 5, 0, jbytes(roster(g, bench={"verifiers": [{"addr": v1, "name": 1, "protocols": []}], "protocols": []}))), "s3.1: name a string", fault("E_ROSTER_BENCH"))
    D("roster_bench_verifier_protocols_absent", pub(g, 5, 0, jbytes(roster(g, bench={"verifiers": [{"addr": v1, "name": "v"}], "protocols": []}))), "s3.1: a verifier object needs protocols", fault("E_ROSTER_BENCH"))
    D("roster_bench_verifier_protocol_bad", pub(g, 5, 0, jbytes(roster(g, bench={"verifiers": [{"addr": v1, "name": "v", "protocols": ["0x12"]}], "protocols": []}))), "s3.1: a verifier's protocol outside hex32", fault("E_ROSTER_BENCH"))
    D("roster_bench_verifier_upper", pub(g, 5, 0, jbytes(roster(g, bench={"verifiers": [v1.upper().replace("0X", "0x")], "protocols": []}))), "s3.1: a verifier string outside hex20", fault("E_ROSTER_BENCH"))
    D("roster_bench_verifier_int", pub(g, 5, 0, jbytes(roster(g, bench={"verifiers": [1], "protocols": []}))), "s3.1: a verifier outside both forms", fault("E_ROSTER_BENCH"))
    D("roster_bench_verifier_object_no_addr", pub(g, 5, 0, jbytes(roster(g, bench={"verifiers": [{"name": "v", "protocols": []}], "protocols": []}))), "s3.1: a verifier object needs addr", fault("E_ROSTER_BENCH"))
    D("roster_bench_verifier_object_addr_upper", pub(g, 5, 0, jbytes(roster(g, bench={"verifiers": [{"addr": v1.upper().replace("0X", "0x"), "name": "v", "protocols": []}], "protocols": []}))), "s3.1: a verifier object's addr outside hex20", fault("E_ROSTER_BENCH"))
    D("roster_bench_verifier_object_addr_int", pub(g, 5, 0, jbytes(roster(g, bench={"verifiers": [{"addr": 7, "name": "v", "protocols": []}], "protocols": []}))), "s3.1: a verifier object's addr not a string", fault("E_ROSTER_BENCH"))
    r = roster(g, bench={"verifiers": [], "protocols": [], "note": "kept"})
    D("roster_bench_extra_member", pub(g, 5, 0, jbytes(r)), "s3.1: members of bench beyond the two are permitted", roster_answer(r))
    r = roster(g, bench={"verifiers": [], "protocols": []})
    D("roster_bench_empty", pub(g, 5, 0, jbytes(r)), "s3.1: both arrays may be empty", roster_answer(r))
    r = roster(g, pool={"version": 1, "sha256": H32()})
    D("roster_pool_member_is_extension", pub(g, 5, 0, jbytes(r)), "s3.1: a member outside the table, whatever a tool means by it, is an extension member and not read; a pool is offered by a covenant document (s3b) and never by a roster member", roster_answer(r, ("pool",)))
    D("roster_order_guild_before_chain", pub(g, 5, 0, jbytes({"kind": KR, "guild": 1, "chainId": "1", "issued": 1, "judges": []})), "s3.2 order: guild is decided before chainId", fault("E_ROSTER_GUILD"))
    D("roster_order_judges_before_bench", pub(g, 5, 0, jbytes(roster(g, judges=[1], bench=None))), "s3.2 order: judges before bench", fault("E_ROSTER_JUDGES"))
    r = roster(g, expires="x", name=1)
    D("roster_expires_and_name_untested", pub(g, 5, 0, jbytes(r)), "s3.2: no step reads expires or name", roster_answer(r))
    D("roster_order_chain_before_issued", pub(g, 5, 0, jbytes(roster(g, chain="1", issued="1"))), "s3.2 order: chainId before issued", fault("E_ROSTER_CHAIN"))
    D("roster_order_issued_then_judges", pub(g, 5, 0, jbytes(roster(g, issued="1", expires="1"))), "s3.2 order: issued is decided; expires is no step", fault("E_ROSTER_ISSUED"))
    D("roster_order_issued_before_judges", pub(g, 5, 0, jbytes(roster(g, issued="1", judges=1))), "s3.2 order: issued before judges", fault("E_ROSTER_ISSUED"))
    D("roster_terms_beside_bad_judges", pub(g, 5, 0, jbytes(roster(g, terms=2, judges=1))), "s3.2: terms is no step; judges decides", fault("E_ROSTER_JUDGES"))
    r = roster(g, judges=[j2, j1])
    D("roster_judges_document_order", pub(g, 5, 0, jbytes(r)), "HARNESS: judges in document order, never reordered", roster_answer(r))
    r = roster(g, bench={"verifiers": [v2, {"addr": v1, "name": "实验室 A", "protocols": [H32()]}], "protocols": []})
    D("roster_bench_verifier_views", pub(g, 5, 0, jbytes(r)), "HARNESS: a string element shows addr alone, a verifier object its three members; name is any string", roster_answer(r))
    r = roster(g, bench={"verifiers": [{"addr": v1, "name": "tab\tin name", "protocols": []}], "protocols": []})
    D("roster_bench_verifier_name_any_scalar", pub(g, 5, 0, jbytes(r)), "s3.1: a verifier name's scalar values are unrestricted", roster_answer(r))
    D("json_escaped_kind_and_key", pub(g, 5, 0, b'{"k\\u0069nd":"galeed.ro\\u0073ter/1","gu\\u0069ld":"%s","chainId":1,"issued":10,"judges":[]}' % g.encode()), "s1: every form is decided on the decoded string; escapes in kind and in a member name", roster_answer(roster(g)))
    D("json_escape_upper_digits", pub(g, 5, 0, b'{"kind":"galeed.roster\\u002F1","guild":"%s","chainId":1,"issued":10,"judges":[]}' % g.encode()), "s1: a \\u escape's hexadecimal digits are of either case; the solidus spelled with an uppercase F", roster_answer(roster(g)))
    r = roster(g); r["é"] = 1
    D("json_non_ascii_key", pub(g, 5, 0, jbytes(r)), "s1: the reader places no charset on keys; a non-ASCII key is an extension member", roster_answer(roster(g), ("é",)))
    r = roster(g); r["！"] = 1; r["\U0001f600"] = 2
    D("json_key_order_utf8_bytes", pub(g, 5, 0, jbytes(r)), "s1: bytewise ascending over UTF-8 bytes puts U+FF01 (EF BC 81) before U+1F600 (F0 9F 98 80); UTF-16 order would reverse them", roster_answer(roster(g), ("！", "\U0001f600")))


    # ---- the guild card table (s3c)
    c = card(g, name="Guild Ones", home="https://guild.example", note_md="自述\n第二行")
    D("guildcard_full", pub(g, 5, 0, jbytes(c)), "s3c.1: every member", card_answer(c))
    c = card(g)
    D("guildcard_minimal", pub(g, 5, 0, jbytes(c)), "s3c.1: the required members alone", card_answer(c))
    c = card(g, name="")
    D("guildcard_name_empty", pub(g, 5, 0, jbytes(c)), "s3c.1: name any string, the empty string included", card_answer(c))
    c = card(g, home="not a url at all \u0001")
    D("guildcard_home_any_string", pub(g, 5, 0, jbytes(c)), "s3c.1: home's scalar values unrestricted", card_answer(c))
    c = card(g, issued=9007199254740991)
    D("guildcard_issued_max", pub(g, 5, 0, jbytes(c)), "s3c.1: issued at 2^53 - 1", card_answer(c))
    c = card(g, amanah={"entry_id": H32()}, ledger={"root": g}, expires=5, chainId=1)
    D("guildcard_judge_members_unknown", pub(g, 5, 0, jbytes(c)), "s3c.3: amanah and ledger are a judge document's and no card's; a card that writes them, or expires or chainId, carries them beyond the table", card_answer(c))
    D("guildcard_guild_absent", pub(g, 5, 0, jbytes({"kind": KC, "issued": 1})), "s3c.2: guild absent", fault("E_GUILDCARD_GUILD"))
    D("guildcard_guild_upper", pub(g, 5, 0, jbytes(card(g.upper().replace("0X", "0x")))), "s3c.2: guild outside hex20", fault("E_GUILDCARD_GUILD"))
    D("guildcard_guild_null", pub(g, 5, 0, jbytes(card(None))), "s3c.2: guild written null", fault("E_GUILDCARD_GUILD"))
    D("guildcard_issued_absent", pub(g, 5, 0, jbytes({"kind": KC, "guild": g})), "s3c.2: issued absent", fault("E_GUILDCARD_ISSUED"))
    D("guildcard_issued_string", pub(g, 5, 0, jbytes(card(g, issued="10"))), "s3c.2: issued a string", fault("E_GUILDCARD_ISSUED"))
    D("guildcard_name_int", pub(g, 5, 0, jbytes(card(g, name=1))), "s3c.2: name a string", fault("E_GUILDCARD_NAME"))
    D("guildcard_home_null", pub(g, 5, 0, jbytes(card(g, home=None))), "s3c.2: home written null is present", fault("E_GUILDCARD_HOME"))
    D("guildcard_note_array", pub(g, 5, 0, jbytes(card(g, note_md=["x"]))), "s3c.2: note_md a string", fault("E_GUILDCARD_NOTE"))
    D("guildcard_order_guild_before_issued", pub(g, 5, 0, jbytes(card(1, issued="x"))), "s3c.2 order: guild before issued", fault("E_GUILDCARD_GUILD"))
    D("guildcard_order_issued_before_name", pub(g, 5, 0, jbytes(card(g, issued="x", name=1))), "s3c.2 order: issued before name", fault("E_GUILDCARD_ISSUED"))
    D("guildcard_order_name_before_home", pub(g, 5, 0, jbytes(card(g, name=1, home=1))), "s3c.2 order: name before home", fault("E_GUILDCARD_NAME"))
    D("guildcard_order_home_before_note", pub(g, 5, 0, jbytes(card(g, home=1, note_md=1))), "s3c.2 order: home before note_md", fault("E_GUILDCARD_HOME"))
    D("unknown_kind_guildcard_v2", pub(g, 5, 0, jbytes({"kind": "galeed.guildcard/2", "guild": g})), "s2.1 step 4: a later card kind is an unknown kind to this reader", {"kind": "galeed.guildcard/2", "known": False, "ok": True})

    # ---- the judge document table (s3a)
    jd = jdoc(j1, name="Judge One", home="https://j1.example", note_md="自述\n第二行")
    D("judge_full", pub(j1, 5, 0, jbytes(jd)), "s3a.1: every member, the basis a galeed/1 basis", judge_answer(jd))
    jd = jdoc(j1)
    D("judge_minimal", pub(j1, 5, 0, jbytes(jd)), "s3a.1: the required members alone", judge_answer(jd))
    jd = jdoc(j1); jd["amanah"]["anchors"] = [1]; jd["ledger"]["note"] = "x"; jd["tracks"] = ["a"]
    D("judge_extra_members", pub(j1, 5, 0, jbytes(jd)), "s3a.1: members beyond the table at the root, inside amanah and inside ledger are permitted; the root's are named, the nested ones not carried", judge_answer(jdoc(j1, entry_id=jd["amanah"]["entry_id"]), ("tracks",)))
    jd = jdoc(j1, root=j2)
    D("judge_root_differs_from_judge", pub(j1, 5, 0, jbytes(jd)), "s3a.1: ledger.root is the root the ledger is audited under, which a moved seat's current key is not; the table tests the form alone", judge_answer(jd))
    two = {"chains": [{"chainId": 1, "fromBlock": 0, "toBlock": 10, "registries": [A("reg")], "senders": [j1]},
                      {"chainId": 8453, "fromBlock": 5, "toBlock": 50, "registries": [{"codeHash": H32()}], "senders": [j1, j2] if j1 < j2 else [j2, j1]}]}
    jd = jdoc(j1, bas=two)
    D("judge_basis_two_chains", pub(j1, 5, 0, jbytes(jd)), "s3a.1: a basis of two chains with a class item is a galeed/1 basis", judge_answer(jd))
    D("judge_judge_absent", pub(j1, 5, 0, jbytes({"kind": KJ, "issued": 1, "amanah": {"entry_id": H32()}, "ledger": {"root": j1, "basis": BASIS}})), "s3a.2: judge absent", fault("E_JUDGE_JUDGE"))
    D("judge_judge_upper", pub(j1, 5, 0, jbytes(jdoc(j1.upper().replace("0X", "0x")))), "s3a.2: judge outside hex20", fault("E_JUDGE_JUDGE"))
    D("judge_issued_absent", pub(j1, 5, 0, jbytes({"kind": KJ, "judge": j1, "amanah": {"entry_id": H32()}, "ledger": {"root": j1, "basis": BASIS}})), "s3a.2: issued absent", fault("E_JUDGE_ISSUED"))
    D("judge_issued_string", pub(j1, 5, 0, jbytes(jdoc(j1, issued="10"))), "s3a.2: issued a string", fault("E_JUDGE_ISSUED"))
    D("judge_amanah_absent", pub(j1, 5, 0, jbytes({"kind": KJ, "judge": j1, "issued": 1, "ledger": {"root": j1, "basis": BASIS}})), "s3a.2: amanah absent", fault("E_JUDGE_AMANAH"))
    D("judge_amanah_string", pub(j1, 5, 0, jbytes(jdoc(j1, amanah=H32()))), "s3a.2: amanah not an object (the entry_id alone is not the member)", fault("E_JUDGE_AMANAH"))
    D("judge_amanah_no_entry_id", pub(j1, 5, 0, jbytes(jdoc(j1, amanah={"version": 3}))), "s3a.2: entry_id absent", fault("E_JUDGE_AMANAH"))
    D("judge_amanah_entry_id_hex20", pub(j1, 5, 0, jbytes(jdoc(j1, amanah={"entry_id": j1}))), "s3a.2: entry_id outside hex32", fault("E_JUDGE_AMANAH"))
    D("judge_amanah_entry_id_upper", pub(j1, 5, 0, jbytes(jdoc(j1, amanah={"entry_id": H32().upper().replace("0X", "0x")}))), "s3a.2: entry_id lowercase", fault("E_JUDGE_AMANAH"))
    D("judge_ledger_absent", pub(j1, 5, 0, jbytes({"kind": KJ, "judge": j1, "issued": 1, "amanah": {"entry_id": H32()}})), "s3a.2: ledger absent", fault("E_JUDGE_LEDGER"))
    D("judge_ledger_array", pub(j1, 5, 0, jbytes(jdoc(j1, ledger=[]))), "s3a.2: ledger not an object", fault("E_JUDGE_LEDGER"))
    D("judge_ledger_no_root", pub(j1, 5, 0, jbytes(jdoc(j1, ledger={"basis": BASIS}))), "s3a.2: root absent", fault("E_JUDGE_LEDGER"))
    D("judge_ledger_root_short", pub(j1, 5, 0, jbytes(jdoc(j1, ledger={"root": "0x12", "basis": BASIS}))), "s3a.2: root outside hex20", fault("E_JUDGE_LEDGER"))
    D("judge_ledger_no_basis", pub(j1, 5, 0, jbytes(jdoc(j1, ledger={"root": j1}))), "s3a.2: basis absent", fault("E_JUDGE_LEDGER"))
    D("judge_basis_second_member", pub(j1, 5, 0, jbytes(jdoc(j1, bas={"chains": [], "transactions": []}))), "s3a.1: a basis with a member beyond chains is no galeed/1 basis", fault("E_JUDGE_LEDGER"))
    D("judge_basis_chains_missing", pub(j1, 5, 0, jbytes(jdoc(j1, bas={}))), "s3a.1: the empty object is no basis", fault("E_JUDGE_LEDGER"))
    D("judge_basis_senders_unsorted", pub(j1, 5, 0, jbytes(jdoc(j1, bas={"chains": [{"chainId": 1, "fromBlock": 0, "toBlock": 1, "registries": [], "senders": ["0x" + "bb" * 20, "0x" + "aa" * 20]}]}))), "s3a.1 with galeed/1 s9.4: senders out of bytewise order", fault("E_JUDGE_LEDGER"))
    D("judge_basis_range", pub(j1, 5, 0, jbytes(jdoc(j1, bas={"chains": [{"chainId": 1, "fromBlock": 2, "toBlock": 1, "registries": [], "senders": []}]}))), "s3a.1 with galeed/1 s9.4: fromBlock above toBlock", fault("E_JUDGE_LEDGER"))
    D("judge_basis_windows_touch_equal", pub(j1, 5, 0, jbytes(jdoc(j1, bas={"chains": [{"chainId": 1, "fromBlock": 0, "toBlock": 1, "registries": [], "senders": []}, {"chainId": 1, "fromBlock": 2, "toBlock": 3, "registries": [], "senders": []}]}))), "s3a.1 with galeed/1 s9.4: two adjacent windows with equal lists", fault("E_JUDGE_LEDGER"))
    D("judge_basis_registry_upper", pub(j1, 5, 0, jbytes(jdoc(j1, bas={"chains": [{"chainId": 1, "fromBlock": 0, "toBlock": 1, "registries": ["0x" + "AA" * 20], "senders": []}]}))), "s3a.1 with galeed/1 s9.4: a registry outside hex20", fault("E_JUDGE_LEDGER"))
    D("judge_basis_class_item_extra", pub(j1, 5, 0, jbytes(jdoc(j1, bas={"chains": [{"chainId": 1, "fromBlock": 0, "toBlock": 1, "registries": [{"codeHash": H32(), "x": 1}], "senders": []}]}))), "s3a.1 with galeed/1 s9.4: a class item with a second member", fault("E_JUDGE_LEDGER"))
    D("judge_basis_not_object", pub(j1, 5, 0, jbytes(jdoc(j1, bas=[]))), "s3a.1: a basis that is not an object", fault("E_JUDGE_LEDGER"))
    D("judge_name_int", pub(j1, 5, 0, jbytes(jdoc(j1, name=1))), "s3a.2: name a string", fault("E_JUDGE_NAME"))
    D("judge_home_null", pub(j1, 5, 0, jbytes(jdoc(j1, home=None))), "s3a.2: home written null is present", fault("E_JUDGE_HOME"))
    D("judge_note_array", pub(j1, 5, 0, jbytes(jdoc(j1, note_md=["x"]))), "s3a.2: note_md a string", fault("E_JUDGE_NOTE"))
    D("judge_order_judge_before_issued", pub(j1, 5, 0, jbytes(jdoc(1, issued="x"))), "s3a.2 order: judge before issued", fault("E_JUDGE_JUDGE"))
    D("judge_order_issued_before_amanah", pub(j1, 5, 0, jbytes(jdoc(j1, issued="x", amanah=1))), "s3a.2 order: issued before amanah", fault("E_JUDGE_ISSUED"))
    D("judge_order_amanah_before_ledger", pub(j1, 5, 0, jbytes(jdoc(j1, amanah=1, ledger=1))), "s3a.2 order: amanah before ledger", fault("E_JUDGE_AMANAH"))
    D("judge_order_ledger_before_name", pub(j1, 5, 0, jbytes(jdoc(j1, ledger=1, name=1))), "s3a.2 order: ledger before name", fault("E_JUDGE_LEDGER"))
    D("judge_order_name_before_home", pub(j1, 5, 0, jbytes(jdoc(j1, name=1, home=1))), "s3a.2 order: name before home", fault("E_JUDGE_NAME"))
    D("judge_order_home_before_note", pub(j1, 5, 0, jbytes(jdoc(j1, home=1, note_md=1))), "s3a.2 order: home before note_md", fault("E_JUDGE_HOME"))
    jd = jdoc(j1, name="")
    D("judge_name_empty", pub(j1, 5, 0, jbytes(jd)), "s3a.1: name any string, the empty string included", judge_answer(jd))
    D("unknown_kind_judge_v2", pub(j1, 5, 0, jbytes({"kind": "galeed.judge/2", "judge": j1})), "s2.1 step 4: a later judge kind is an unknown kind to this reader", {"kind": "galeed.judge/2", "known": False, "ok": True})

    # ---- the guild covenant table (s3b)
    gd = gdoc(g, version=3, text="第三版 判法\n全文", venues=[{"chainId": 1, "contract": A("venue"), "codeHash": H32(), "name": "escrow"}],
              quotes={"1:" + A("venue"): "0x00ff", "1:" + H32() + ":" + A("tok"): "0x", "nonsense": 5})
    gd["extra"] = True
    D("guildamanah_full", pub(g, 5, 0, jbytes(gd)), "s3b.1: every member; venues carried by their three named members, quotes as written whatever its members, sha256 of the text", covenant_answer(gd, ("extra",)))
    gd = gdoc(g)
    D("guildamanah_minimal", pub(g, 5, 0, jbytes(gd)), "s3b.1: the required members alone", covenant_answer(gd))
    gd = gdoc(g, text="")
    D("guildamanah_text_empty", pub(g, 5, 0, jbytes(gd)), "s3b.1: text_md any string; the empty text has a sha256", covenant_answer(gd))
    gd = gdoc(g, text="\U0001f600 é")
    D("guildamanah_text_non_ascii_sha", pub(g, 5, 0, jbytes(gd)), "s3b.3: sha256 is over the UTF-8 encoding of the scalar values", covenant_answer(gd))
    gd = gdoc(g, venues=[])
    D("guildamanah_venues_empty", pub(g, 5, 0, jbytes(gd)), "s3b.1: the empty venues array", covenant_answer(gd))
    gd = gdoc(g, quotes={})
    D("guildamanah_quotes_empty", pub(g, 5, 0, jbytes(gd)), "s3b.1: the empty quotes object", covenant_answer(gd))
    D("guildamanah_guild_absent", pub(g, 5, 0, jbytes({"kind": KG, "chainId": 1, "issued": 1, "version": 1, "text_md": "t"})), "s3b.2: guild absent", fault("E_GUILDAMANAH_GUILD"))
    D("guildamanah_guild_upper", pub(g, 5, 0, jbytes(gdoc(g.upper().replace("0X", "0x")))), "s3b.2: guild outside hex20", fault("E_GUILDAMANAH_GUILD"))
    D("guildamanah_chain_string", pub(g, 5, 0, jbytes(gdoc(g, chain="1"))), "s3b.2: chainId a string", fault("E_GUILDAMANAH_CHAIN"))
    D("guildamanah_issued_absent", pub(g, 5, 0, jbytes({"kind": KG, "guild": g, "chainId": 1, "version": 1, "text_md": "t"})), "s3b.2: issued absent", fault("E_GUILDAMANAH_ISSUED"))
    D("guildamanah_version_string", pub(g, 5, 0, jbytes(gdoc(g, version="1"))), "s3b.2: version an int", fault("E_GUILDAMANAH_VERSION"))
    D("guildamanah_version_absent", pub(g, 5, 0, jbytes({"kind": KG, "guild": g, "chainId": 1, "issued": 1, "text_md": "t"})), "s3b.2: version absent", fault("E_GUILDAMANAH_VERSION"))
    D("guildamanah_text_absent", pub(g, 5, 0, jbytes({"kind": KG, "guild": g, "chainId": 1, "issued": 1, "version": 1})), "s3b.2: text_md absent", fault("E_GUILDAMANAH_TEXT"))
    D("guildamanah_text_int", pub(g, 5, 0, jbytes(gdoc(g, text=1))), "s3b.2: text_md a string", fault("E_GUILDAMANAH_TEXT"))
    D("guildamanah_venues_object", pub(g, 5, 0, jbytes(gdoc(g, venues={}))), "s3b.2: venues an array", fault("E_GUILDAMANAH_VENUES"))
    D("guildamanah_venue_no_codehash", pub(g, 5, 0, jbytes(gdoc(g, venues=[{"chainId": 1, "contract": A("venue")}]))), "s3b.1: a venue element needs codeHash", fault("E_GUILDAMANAH_VENUES"))
    D("guildamanah_venue_contract_hex32", pub(g, 5, 0, jbytes(gdoc(g, venues=[{"chainId": 1, "contract": H32(), "codeHash": H32()}]))), "s3b.1: contract hex20", fault("E_GUILDAMANAH_VENUES"))
    D("guildamanah_venue_string", pub(g, 5, 0, jbytes(gdoc(g, venues=[A("venue")]))), "s3b.1: every element an object", fault("E_GUILDAMANAH_VENUES"))
    D("guildamanah_venue_no_chainid", pub(g, 5, 0, jbytes(gdoc(g, venues=[{"contract": A("venue"), "codeHash": H32()}]))), "s3b.1: a venue element needs chainId", fault("E_GUILDAMANAH_VENUES"))
    D("guildamanah_venue_chainid_string", pub(g, 5, 0, jbytes(gdoc(g, venues=[{"chainId": "1", "contract": A("venue"), "codeHash": H32()}]))), "s3b.1: a venue element's chainId an int", fault("E_GUILDAMANAH_VENUES"))
    D("guildamanah_quotes_array", pub(g, 5, 0, jbytes(gdoc(g, quotes=[]))), "s3b.2: quotes an object", fault("E_GUILDAMANAH_QUOTES"))
    D("guildamanah_quotes_null", pub(g, 5, 0, jbytes(gdoc(g, quotes=None))), "s3b.2: quotes written null is present", fault("E_GUILDAMANAH_QUOTES"))
    D("guildamanah_order_guild_before_chain", pub(g, 5, 0, jbytes(gdoc(1, chain="1"))), "s3b.2 order: guild before chainId", fault("E_GUILDAMANAH_GUILD"))
    D("guildamanah_order_chain_before_issued", pub(g, 5, 0, jbytes(gdoc(g, chain="1", issued="1"))), "s3b.2 order: chainId before issued", fault("E_GUILDAMANAH_CHAIN"))
    D("guildamanah_order_issued_before_version", pub(g, 5, 0, jbytes(gdoc(g, issued="1", version="1"))), "s3b.2 order: issued before version", fault("E_GUILDAMANAH_ISSUED"))
    D("guildamanah_order_version_before_text", pub(g, 5, 0, jbytes(gdoc(g, version="1", text=1))), "s3b.2 order: version before text_md", fault("E_GUILDAMANAH_VERSION"))
    D("guildamanah_order_text_before_venues", pub(g, 5, 0, jbytes(gdoc(g, text=1, venues=1))), "s3b.2 order: text_md before venues", fault("E_GUILDAMANAH_TEXT"))
    D("guildamanah_order_venues_before_quotes", pub(g, 5, 0, jbytes(gdoc(g, venues=1, quotes=1))), "s3b.2 order: venues before quotes", fault("E_GUILDAMANAH_VENUES"))
    D("unknown_kind_guildamanah_v2", pub(g, 5, 0, jbytes({"kind": "galeed.guildamanah/2", "guild": g})), "s2.1 step 4: a later covenant kind is an unknown kind to this reader", {"kind": "galeed.guildamanah/2", "known": False, "ok": True})

    # ---- publication file form (misuse)
    p = pub(g, 5, 0, doc); p["logIndex"] = "0"
    D("misuse_logindex_string", p, "HARNESS: logIndex not an int", None)
    p = pub(g, 5, 0, doc); del p["emitter"]
    D("misuse_emitter_absent", p, "HARNESS: emitter absent, though decode does not read it", None)
    p = pub(g, 5, 0, doc); del p["blockNumber"]
    D("misuse_block_absent", p, "HARNESS: blockNumber absent", None)
    case("decode", [], "HARNESS: a missing argument", None, path="(none)")
    p = pub(g, 5, 0, doc)
    p["emitter"] = g.upper().replace("0X", "0x")
    D("misuse_emitter_upper", p, "HARNESS: emitter outside hex20", None)
    p = pub(g, 5, 0, doc)
    p["blockNumber"] = "5"
    D("misuse_block_string", p, "HARNESS: blockNumber not an int", None)
    p = pub(g, 5, 0, doc)
    p["document"] = doc.decode("utf-8")
    D("misuse_document_not_transport", p, "HARNESS: document is the transport hex spelling of the bytes, never the text", None)
    p = pub(g, 5, 0, doc)
    p["document"] = p["document"] + "a"
    D("misuse_document_odd_digits", p, "HARNESS: an odd number of digits", None)
    p = pub(g, 5, 0, doc)
    p["document"] = "0X" + p["document"][2:]
    D("misuse_document_0X", p, "HARNESS: the transport prefix is the two lowercase bytes 0x", None)
    p = pub(g, 5, 0, doc)
    del p["word"]
    D("misuse_word_absent", p, "HARNESS: word absent", None)
    p = pub(g, 5, 0, doc)
    p["document"] = p["document"].upper().replace("0X", "0x")
    D("document_transport_upper_digits", p, "HARNESS: transport digits of either case", roster_answer(roster(g)))
    p = pub(g, 5, 0, doc)
    p["ignored"] = {"anything": 1}
    D("publication_extra_member", p, "HARNESS: members beyond the five are ignored", roster_answer(roster(g)))
    D("misuse_root_array", [pub(g, 5, 0, doc)], "HARNESS: the root is one object", None)
    D("misuse_unreadable_file", b"{", "HARNESS: a file the reader refuses", None)
    case("decode", ["pubs/absent-file.json"], "HARNESS: a path the command cannot read", None, absent=["pubs/absent-file.json"])
    case("decode", ["pubs/p001_roster_minimal.json", "extra"], "HARNESS: an argument beyond the list", None)
    argv_case([], "HARNESS: no command", None, "(none)")
    argv_case(["frobnicate", "pubs/p001_roster_minimal.json"], "HARNESS: an unknown command", None, "(none)")


# ==========================================================================
# the shared logs set for resolution
# ==========================================================================

GUILD_TEXT = "the guild's covenant, version two"
GUILD_QUOTES = None   # filled in build_logs
BOOK = {}             # tag -> the declaration or accession record, for predictions
ZERO = "0x" + "00" * 20


def build_logs():
    """One logs set on chain 1 that every resolution case reads: the
    registry's publications and statuses, and the succession book's
    declarations and accessions (s4); the notes below name the addresses by
    tag."""
    global GUILD_QUOTES
    pubs, stats, decls, accs = [], [], [], []
    idx = [0]

    def nxt():
        idx[0] += 1
        return idx[0]

    def P(emitter, block, doc, **kw):
        p = pub(emitter, block, nxt(), jbytes(doc) if not isinstance(doc, bytes) else doc, **kw)
        pubs.append(p)
        return p

    def S(emitter, block, active):
        stats.append(status(emitter, block, nxt(), active))

    def DEC(tag, emitter, block, to):
        d = {"emitter": emitter, "blockNumber": block, "logIndex": nxt(), "to": to}
        decls.append(d)
        BOOK[tag] = d
        return d

    def ACC(tag, emitter, block, holder, root):
        a = {"emitter": emitter, "blockNumber": block, "logIndex": nxt(), "holder": holder, "root": root}
        accs.append(a)
        BOOK[tag] = a
        return a

    def MOVE(tag, old, new, block, root):
        DEC(tag + ".d", old, block, new)
        ACC(tag + ".a", new, block, old, root)

    g1, j1, j2, j3, j9, v1, v2, v3 = A("g1"), A("j1"), A("j2"), A("j3"), A("j9"), A("v1"), A("v2"), A("v3")
    # g1: two versions; the later carries the bench, extension members, and the three members the table no longer has
    P(g1, 5, roster(g1, judges=[j1, j3], name="first"))
    r2 = roster(g1, issued=10, judges=[j1], name="第二版", terms="t", bench={"verifiers": [v2, {"addr": v1, "name": "v", "protocols": [H32()]}, v3], "protocols": [H32()]})
    r2["zeta"] = 1
    r2["alpha"] = 2
    P(g1, 7, r2)
    # j1 -> j2 at block 1; a later declaration from j1, no longer current, binds nothing
    MOVE("j1", j1, j2, 1, j1)
    DEC("j1.late", j1, 2, j9)
    # j3: a declaration to j9 that j9 never takes up
    DEC("j3", j3, 1, j9)
    # v1: an accession with no declaration behind it counts for nothing
    w9 = A("w9")
    ACC("v1", w9, 1, v1, v1)
    # v3: a declaration to the emitter itself binds nothing
    DEC("v3", v3, 1, v3)
    # c1 -> c2, then c2 names c1, which has entered: binds nothing
    c1, c2 = A("c1"), A("c2")
    MOVE("c1", c1, c2, 1, c1)
    DEC("c2.back", c2, 2, c1)
    # t1 -> t2 -> t3, then t3 names t1: binds nothing
    t1, t2, t3 = A("t1"), A("t2"), A("t3")
    MOVE("t1", t1, t2, 1, t1)
    MOVE("t2", t2, t3, 2, t1)
    DEC("t3.back", t3, 3, t1)
    # h0 -> h1 -> ... -> h65: sixty-five hops from h0, no bound
    hs = [A("h%d" % i) for i in range(66)]
    for i in range(65):
        MOVE("h%d" % i, hs[i], hs[i + 1], i + 1, hs[0])
    # k1: a declaration replaced, a stale accession, the binding one, a late declaration; and a judge document that fails its table
    k1, k2, k3 = A("k1"), A("k2"), A("k3")
    DEC("k1.first", k1, 1, k3)
    DEC("k1.second", k1, 2, k2)
    ACC("k3.stale", k3, 3, k1, k1)
    ACC("k2", k2, 3, k1, k1)
    DEC("k1.late", k1, 4, k3)
    P(k1, 3, {"kind": KJ, "x": 1})
    # m1: the accession sits in a later block than the declaration it takes up (the file order is shuffled)
    m1, m2 = A("m1"), A("m2")
    DEC("m1", m1, 3, m2)
    ACC("m2", m2, 4, m1, m1)
    # n1: an accession before the declaration in the same block, by logIndex: it counts for nothing
    n1, n2 = A("n1"), A("n2")
    ACC("n2.early", n2, 5, n1, n1)
    DEC("n1", n1, 5, n2)
    # o1: a declaration with a smaller logIndex but a later block than the accession: block first
    o1, o2 = A("o1"), A("o2")
    DEC("o1", o1, 6, o2)
    ACC("o2.early", o2, 5, o1, o1)
    # p1 -> p2 whose accession log names another root: the replay reads no root member
    p1, p2 = A("p1"), A("p2")
    DEC("p1", p1, 1, p2)
    ACC("p2", p2, 2, p1, A("lie"))
    # z1 declares to the zero address, binds nothing and does not enter; z0 then hands its record to z1
    z0, z1 = A("z0"), A("z1")
    DEC("z1.zero", z1, 1, ZERO)
    MOVE("z0", z0, z1, 2, z0)
    # e1 enters by declaring; f1 then names e1: binds nothing
    e1, e9, f1 = A("e1"), A("e9"), A("f1")
    DEC("e1", e1, 1, e9)
    DEC("f1", f1, 2, e1)
    # q1 names q2; q2 enters by declaring for its own record; q2's accession of q1 counts for nothing
    q1, q2, q9 = A("q1"), A("q2"), A("q9")
    DEC("q1", q1, 1, q2)
    DEC("q2", q2, 2, q9)
    ACC("q2.late", q2, 3, q1, q1)
    # s1 names s3; a stranger sx names s3 too; s3 takes up s1's record
    s1, s3, sx = A("s1"), A("s3"), A("sx")
    DEC("s1", s1, 1, s3)
    DEC("sx", sx, 2, s3)
    ACC("s3", s3, 3, s1, s1)
    # u1 names u2, then u3; u3 takes it up; u2's accession after it counts for nothing
    u1, u2, u3 = A("u1"), A("u2"), A("u3")
    DEC("u1.first", u1, 1, u2)
    DEC("u1.second", u1, 2, u3)
    ACC("u3", u3, 3, u1, u1)
    ACC("u2.late", u2, 4, u1, u1)
    # gr: retired guild (status false latest); gs: retired then resumed; gn: status only, no publication
    gr, gs, gn = A("gr"), A("gs"), A("gn")
    P(gr, 1, roster(gr, judges=[j1])); S(gr, 2, False)
    P(gs, 1, roster(gs, judges=[j1])); S(gs, 2, False); S(gs, 3, True)
    S(gn, 1, False)
    # gh: last roster has a hash mismatch; gu: last roster kind is a later version; gx: a roster then a former succession record (another kind)
    gh, gu, gx = A("gh"), A("gu"), A("gx")
    P(gh, 1, roster(gh, judges=[j1]), word=H32())
    P(gu, 1, roster(gu, judges=[j1])); P(gu, 2, {"kind": "galeed.roster/2", "guild": gu})
    P(gx, 1, roster(gx, judges=[j1])); P(gx, 2, {"kind": "galeed.succession/1", "from": gx, "to": j9, "chainId": 1, "issued": 1})
    # gm: moved guild: gm -> gm2 which publishes its own roster; gm's own card stays at gm; gq: moved to gq2 which published nothing
    gm, gm2, gq, gq2 = A("gm"), A("gm2"), A("gq"), A("gq2")
    P(gm, 1, roster(gm, judges=[j3])); P(gm, 1, card(gm, name="gm before it moved")); MOVE("gm", gm, gm2, 2, gm); P(gm2, 3, roster(gm2, judges=[j3, j1]))
    P(gq, 1, roster(gq, judges=[j3])); MOVE("gq", gq, gq2, 2, gq)
    # gw: roster whose guild member names another address; gc: chainId 2; ge: a roster that writes expires; gi: issued far ahead
    gw, gc, ge, gi = A("gw"), A("gc"), A("ge"), A("gi")
    P(gw, 1, roster(g1, judges=[j1]))
    P(gc, 1, roster(gc, chain=2, judges=[j1]))
    P(ge, 1, roster(ge, expires=50, judges=[j1]))
    P(gi, 1, roster(gi, issued=9007199254740991, judges=[j1]))
    # gb: judges and a bench whose addresses moved, a repeated address, a duplicate through a trail
    gb = A("gb")
    P(gb, 1, roster(gb, judges=[c1, j1, hs[0]], bench={"verifiers": [v3, j1, j2, {"addr": j1, "name": "again", "protocols": []}], "protocols": [H32(), H32()]}))
    # gz: a roster with no bench member; gd: retired at the successor, active at the origin; go: origin retired, successor active
    gz, gd, gd2, go, go2 = A("gz"), A("gd"), A("gd2"), A("go"), A("go2")
    P(gz, 1, roster(gz, judges=[]))
    MOVE("gd", gd, gd2, 1, gd); P(gd2, 2, roster(gd2, judges=[j1])); S(gd2, 3, False)
    MOVE("go", go, go2, 1, go); S(go, 2, False); P(go2, 3, roster(go2, judges=[j1]))
    # gl: retired and mismatching: the order of s5.4 decides
    gl, gp, gk = A("gl"), A("gp"), A("gk")
    P(gl, 1, roster(g1, chain=2, judges=[])); S(gl, 2, False)
    P(gp, 1, roster(g1, chain=2, judges=[]))
    P(gk, 1, roster(gk, chain=2, judges=[]))
    # ga: two publications in one block
    ga = A("ga")
    P(ga, 4, roster(ga, judges=[j1], name="a")); P(ga, 4, roster(ga, judges=[j2], name="b"))
    # gt: two statuses in one block, false then true by logIndex
    gt = A("gt")
    P(gt, 1, roster(gt, judges=[j1])); S(gt, 5, False); S(gt, 5, True)
    # cs: a self-declaration (binds nothing) and a false status
    cs = A("cs")
    DEC("cs", cs, 1, cs); S(cs, 2, False); P(cs, 1, roster(cs, judges=[j1]))
    # a0 -> a1 -> a2, attempts at a0 (a replaced declaration, a late one), at a1 (a late one), and a2's pending one, interleaved by block
    a0, a1, a2, x1, x2, x3, x4 = A("a0"), A("a1"), A("a2"), A("x1"), A("x2"), A("x3"), A("x4")
    DEC("a0.x1", a0, 2, x1)
    MOVE("a0", a0, a1, 3, a0)
    MOVE("a1", a1, a2, 5, a0)
    DEC("a1.x3", a1, 7, x3)
    DEC("a0.x2", a0, 8, x2)
    DEC("a2.x4", a2, 9, x4)
    # gg: a pool guild: a roster and a covenant, the covenant republished once
    gg = A("gg")
    GUILD_QUOTES = {"1:" + v1: "0x0001", "1:" + ("0x" + "cd" * 32) + ":" + v2: "0x", "1": "0x0f"}
    P(gg, 1, roster(gg, judges=[j1, j3]))
    P(gg, 2, gdoc(gg, version=1, text="the guild's covenant, version one"))
    P(gg, 3, gdoc(gg, version=2, text=GUILD_TEXT, venues=[{"chainId": 1, "contract": v1, "codeHash": "0x" + "cd" * 32}], quotes=GUILD_QUOTES))
    # gg2: a covenant alone, no roster; gg3: covenant whose guild is another address; gg4: covenant for chain 2;
    # gg5: a later covenant version kind; gg6: the last covenant fails its hash; gg7: a roster whose covenant fails its table
    gg2, gg3, gg4, gg5, gg6, gg7 = A("gg2"), A("gg3"), A("gg4"), A("gg5"), A("gg6"), A("gg7")
    P(gg2, 1, gdoc(gg2, version=0, text="pool text"))
    P(gg3, 1, roster(gg3, judges=[j1])); P(gg3, 2, gdoc(gg, version=1))
    P(gg4, 1, roster(gg4, judges=[j1])); P(gg4, 2, gdoc(gg4, chain=2, version=1))
    P(gg5, 1, roster(gg5, judges=[j1])); P(gg5, 2, gdoc(gg5, version=1)); P(gg5, 3, {"kind": "galeed.guildamanah/2", "guild": gg5})
    P(gg6, 1, roster(gg6, judges=[j1])); P(gg6, 2, gdoc(gg6, version=1)); P(gg6, 3, gdoc(gg6, version=2), word=H32())
    P(gg7, 1, roster(gg7, judges=[j1])); P(gg7, 2, gdoc(gg7, version="x"))
    # gv: a guild whose bench carries verifier objects, one of which moved, and protocols out of order
    gv = A("gv")
    P(gv, 1, roster(gv, judges=[j1], bench={"verifiers": [{"addr": v1, "name": "Bureau A", "protocols": [H32()]}, {"addr": c1, "name": "Moved", "protocols": []}, j1], "protocols": ["0xff" + "ff" * 31, "0x11" + "11" * 31]}))
    # gj: a guild whose last publication of all is a judge document; the roster is read all the same
    gj = A("gj")
    P(gj, 1, roster(gj, judges=[j1])); P(gj, 2, jdoc(gj))
    # gf: a guild whose last publication is a kind under no reserved prefix
    gf = A("gf")
    P(gf, 1, roster(gf, judges=[j1])); P(gf, 2, {"kind": "foo/1", "x": 1})
    # gy: a guild whose last publication is unreadable bytes (of every kind), after a good roster and a good card
    gy = A("gy")
    P(gy, 1, roster(gy, judges=[j1])); P(gy, 1, card(gy)); P(gy, 2, b"not json")
    # gvv: a guild whose last publication is a verifier service record (another reserved prefix, another kind)
    gvv = A("gvv")
    P(gvv, 1, roster(gvv, judges=[j1])); P(gvv, 2, {"kind": "galeed.verifier/1", "verifier": gvv, "serves": [], "at": 1})
    # guild cards: gcard a roster, a card, a later card; gcm a card naming another guild; gct a card failing its table;
    # gcv a later card version; gcr a card then a roster (each reader reads its own kind)
    gcard, gcm, gct, gcv, gcr = A("gcard"), A("gcm"), A("gct"), A("gcv"), A("gcr")
    P(gcard, 1, roster(gcard, judges=[j1]))
    P(gcard, 2, card(gcard, name="first card"))
    cfull = card(gcard, issued=20, name="Guild Card", home="https://gcard.example", note_md="我们的行会")
    cfull["motto"] = "kept"
    P(gcard, 3, cfull)
    P(gcm, 1, roster(gcm, judges=[j1])); P(gcm, 2, card(gc))
    P(gct, 1, roster(gct, judges=[j1])); P(gct, 2, card(gct, issued="x"))
    P(gcv, 1, roster(gcv, judges=[j1])); P(gcv, 2, card(gcv)); P(gcv, 3, {"kind": "galeed.guildcard/2", "guild": gcv})
    P(gcr, 1, card(gcr, name="card first")); P(gcr, 2, roster(gcr, judges=[j1]))
    # judges: jd1 publishes a document; jd2's document names another judge; jd3 moved to jd3b which publishes its own document;
    # jd4: a good document then a later one that fails its table; jd5: a document then a later judge kind;
    # jd6: a document then an unreadable publication; jd7: retired judge with a document; jd8: moved to jd8b, which published nothing;
    # jd9: a document from an address that also publishes a roster (both read, each by its kind)
    jd1, jd2, jd3, jd3b, jd4, jd5, jd6, jd7, jd8, jd8b, jd9 = [A("jd%s" % x) for x in ("1", "2", "3", "3b", "4", "5", "6", "7", "8", "8b", "9")]
    P(jd1, 2, jdoc(jd1, name="Judge One", note_md="自述"))
    P(jd2, 2, jdoc(jd1))
    P(jd3, 2, jdoc(jd3)); MOVE("jd3", jd3, jd3b, 3, jd3); P(jd3b, 4, jdoc(jd3b, root=jd3))
    P(jd4, 2, jdoc(jd4)); P(jd4, 3, jdoc(jd4, ledger={"root": jd4}))
    P(jd5, 2, jdoc(jd5)); P(jd5, 3, {"kind": "galeed.judge/2", "judge": jd5})
    P(jd6, 2, jdoc(jd6)); P(jd6, 3, b"\xff")
    P(jd7, 2, jdoc(jd7)); S(jd7, 3, False)
    P(jd8, 2, jdoc(jd8)); MOVE("jd8", jd8, jd8b, 3, jd8)
    P(jd9, 2, roster(jd9, judges=[j1])); P(jd9, 3, jdoc(jd9)); P(jd9, 4, roster(jd9, judges=[j2]))
    # gcx: a covenant whose guild and chain both mismatch (guild is decided first)
    gcx = A("gcx")
    P(gcx, 1, roster(gcx, judges=[j1])); P(gcx, 2, gdoc(gg, chain=2, version=1))
    # gqo: a covenant whose quotes keys and nested member names carry astral and BMP scalars (UTF-8 order on the answer surface)
    gqo = A("gqo")
    P(gqo, 1, roster(gqo, judges=[j1])); P(gqo, 2, gdoc(gqo, version=1, quotes={"\U0001F600": "0x", "！": "0x", "1": {"nested": [1, "\u0001"], "\U0001F600": None, "￿": 2}, "￿": "0x"}))
    # gk5: the last publication's kind is not a string: E_KIND, of every kind
    gk5 = A("gk5")
    P(gk5, 1, roster(gk5, judges=[j1])); P(gk5, 2, jdoc(gk5)); P(gk5, 3, gdoc(gk5, version=1)); P(gk5, 4, {"kind": 5})
    # gyr: unreadable bytes before the roster (the covenant and card candidates stop at step 2, the guild resolves); ghr: a wrong word before the roster
    gyr, ghr = A("gyr"), A("ghr")
    P(gyr, 1, b"not json"); P(gyr, 2, roster(gyr, judges=[j1]))
    P(ghr, 1, roster(ghr, judges=[]), word=H32()); P(ghr, 2, roster(ghr, judges=[j1]))
    # gbp: the bare prefix as kind (of the roster kind, another version); gns: the prefix without its slash (another kind); gts: a trailing space
    gbp, gns, gts = A("gbp"), A("gns"), A("gts")
    P(gbp, 1, roster(gbp, judges=[j1])); P(gbp, 2, {"kind": "galeed.roster/", "guild": gbp})
    P(gns, 1, roster(gns, judges=[j1])); P(gns, 2, {"kind": "galeed.roster", "x": 1})
    P(gts, 1, roster(gts, judges=[j1])); P(gts, 2, {"kind": "galeed.roster/1 ", "guild": gts})
    # jdE: a judge document whose basis has no chains; jdG: windows one block apart with equal lists; jdH: a document then a wrong-word publication
    jdE, jdG, jdH = A("jdE"), A("jdG"), A("jdH")
    P(jdE, 2, jdoc(jdE, bas={"chains": []}))
    P(jdG, 2, jdoc(jdG, bas={"chains": [{"chainId": 1, "fromBlock": 0, "toBlock": 10, "registries": [], "senders": []}, {"chainId": 1, "fromBlock": 12, "toBlock": 20, "registries": [], "senders": []}]}))
    P(jdH, 2, jdoc(jdH)); P(jdH, 3, jdoc(jdH), word=H32())
    # gbig: a roster at block and log index 2^53 - 1; bbig: a declaration and an accession at the top of the range
    gbig, bbig, bbig2 = A("gbig"), A("bbig"), A("bbig2")
    pubs.append(pub(gbig, 9007199254740991, 9007199254740991, jbytes(roster(gbig, judges=[j1]))))
    d = {"emitter": bbig, "blockNumber": 9007199254740991, "logIndex": 9007199254740989, "to": bbig2}
    decls.append(d); BOOK["bbig"] = d
    a = {"emitter": bbig2, "blockNumber": 9007199254740991, "logIndex": 9007199254740990, "holder": bbig, "root": bbig}
    accs.append(a); BOOK["bbig2"] = a
    # gnest: a judges element object whose extra members nest containers
    gnest = A("gnest")
    P(gnest, 1, roster(gnest, judges=[{"addr": j1, "meta": {"a": [1, {"b": None}], "c": {}}}]))
    # dm0 -> dm1 -> dm2: gdm lists the middle key dm1; gdo lists the root dm0 and the final key dm2
    dm0, dm1, dm2, gdm, gdo = A("dm0"), A("dm1"), A("dm2"), A("gdm"), A("gdo")
    MOVE("dm0", dm0, dm1, 1, dm0)
    MOVE("dm1", dm1, dm2, 2, dm0)
    P(gdm, 1, roster(gdm, judges=[dm1], bench={"verifiers": [dm1], "protocols": []}))
    P(gdo, 1, roster(gdo, judges=[dm0, dm2]))
    # a record rooted at the zero address: zero declares zr1 and zr1 accedes
    zr1 = A("zr1")
    MOVE("zr", ZERO, zr1, 1, ZERO)
    # two holders name one key; the key takes up the second; its later accession of the first counts for nothing
    th1, th2, tk = A("th1"), A("th2"), A("tk")
    DEC("th1", th1, 1, tk)
    DEC("th2", th2, 2, tk)
    ACC("tk", tk, 3, th2, th2)
    ACC("tk.late", tk, 4, th1, th1)
    # a binding declaration, then a non-binding one from the same current key: the pending key stays and accedes
    bn0, bn1 = A("bn0"), A("bn1")
    DEC("bn0", bn0, 1, bn1)
    DEC("bn0.zero", bn0, 2, ZERO)
    ACC("bn1", bn1, 3, bn0, bn0)
    # sv: a self-declaration does not enter; sy names sv and sv accedes
    sv, sy = A("sv"), A("sy")
    DEC("sv.self", sv, 1, sv)
    DEC("sy", sy, 2, sv)
    ACC("sv", sv, 3, sy, sy)
    # et: a declaration naming a key that entered keeps the emitter's earlier pending key
    ee, ef, eh, ek = A("ee"), A("ef"), A("eh"), A("ek")
    DEC("ee", ee, 1, ef)
    DEC("eh.k", eh, 2, ek)
    DEC("eh.e", eh, 3, ee)
    ACC("ek", ek, 4, eh, eh)
    # ex: a declaration naming an entered key does not make its emitter enter; the emitter accedes elsewhere
    ex2, ef2, ey2 = A("ex2"), A("ef2"), A("ey2")
    ee2 = A("ee2")
    DEC("ee2", ee2, 1, ef2)
    DEC("ex2.e", ex2, 2, ee2)
    DEC("ey2", ey2, 3, ex2)
    ACC("ex2", ex2, 4, ey2, ey2)
    # na: an acceder named again: it entered by acceding, and a second holder's declaration binds nothing
    nh, nk, nj = A("nh"), A("nk"), A("nj")
    MOVE("nh", nh, nk, 1, nh)
    DEC("nj", nj, 3, nk)
    ACC("nk.again", nk, 4, nj, nj)
    # lk: a key that left declares and its named key accedes: both count for nothing
    lh, lk, lx = A("lh"), A("lk"), A("lx")
    MOVE("lh", lh, lk, 1, lh)
    DEC("lh.x", lh, 2, lx)
    ACC("lx", lx, 3, lh, lh)
    # gcmv: a moved guild whose final address publishes a card naming the origin, the origin's own card valid
    gcmv, gcmv2 = A("gcmv"), A("gcmv2")
    P(gcmv, 1, card(gcmv, name="origin card"))
    MOVE("gcmv", gcmv, gcmv2, 2, gcmv)
    P(gcmv2, 3, roster(gcmv2, judges=[j1]))
    P(gcmv2, 3, card(gcmv, name="names the origin"))
    # gco: a guild that publishes a covenant, moves, and its successor publishes its own covenant
    gco, gco2 = A("gco"), A("gco2")
    P(gco, 1, gdoc(gco, version=1, text="the origin's covenant"))
    MOVE("gco", gco, gco2, 2, gco)
    P(gco2, 4, gdoc(gco2, version=1, text="the successor's covenant"))
    # jm1 -> jm1b with a false status at the origin; jm2 -> jm2b with a false status at the final key
    jm1, jm1b, jm2, jm2b = A("jm1"), A("jm1b"), A("jm2"), A("jm2b")
    MOVE("jm1", jm1, jm1b, 1, jm1); P(jm1b, 2, jdoc(jm1b, root=jm1)); S(jm1, 3, False)
    MOVE("jm2", jm2, jm2b, 1, jm2); P(jm2b, 2, jdoc(jm2b, root=jm2)); S(jm2b, 3, False)
    # gk7: a kind that is not a string before the roster: the card and the covenant stop, the guild resolves
    gk7 = A("gk7")
    P(gk7, 1, {"kind": 7}); P(gk7, 2, roster(gk7, judges=[j1]))
    # pk0 -> pk1, then pk1 declares pk2, which never accedes; gpk lists pk1 as judge and as verifier
    pk0, pk1, pk2, gpk = A("pk0"), A("pk1"), A("pk2"), A("gpk")
    MOVE("pk0", pk0, pk1, 1, pk0)
    DEC("pk1", pk1, 2, pk2)
    P(gpk, 3, roster(gpk, judges=[pk1], bench={"verifiers": [pk1], "protocols": []}))
    # gla lists la0, whose record moves to la1 after the roster
    la0, la1, gla = A("la0"), A("la1"), A("gla")
    P(gla, 1, roster(gla, judges=[la0]))
    MOVE("la0", la0, la1, 2, la0)
    # rd0 declares rd1, then rd2, which replaces it and accedes; grd lists rd0
    rd0, rd1, rd2, grd = A("rd0"), A("rd1"), A("rd2"), A("grd")
    DEC("rd0.first", rd0, 1, rd1)
    DEC("rd0", rd0, 2, rd2)
    ACC("rd2", rd2, 3, rd0, rd0)
    P(grd, 4, roster(grd, judges=[rd0]))
    # sk0 declares sk1 twice in one block, the second replacing the first, and sk1 accedes
    sk0, sk1 = A("sk0"), A("sk1")
    DEC("sk0.first", sk0, 1, sk1)
    DEC("sk0", sk0, 1, sk1)
    ACC("sk1", sk1, 2, sk0, sk0)
    # gzr lists zr1, the final key of the record rooted at the zero address
    gzr = A("gzr")
    P(gzr, 2, roster(gzr, judges=[zr1]))
    # gs0 -> gs1: the guild moved, and its successor's roster lists the guild's own root and j1
    gs0, gs1 = A("gs0"), A("gs1")
    MOVE("gs0", gs0, gs1, 1, gs0)
    P(gs1, 2, roster(gs1, judges=[gs0, j1]))
    RNG.shuffle(pubs)
    RNG.shuffle(stats)
    RNG.shuffle(decls)
    RNG.shuffle(accs)
    return {"publications": pubs, "statuses": stats, "declarations": decls, "accessions": accs}


def judge_answer_at(p):
    """s5.7's JUDGE answer for the judge publication `p`."""
    return {"blockNumber": p["blockNumber"], "document": judge_view(_doc_of(p)), "verdict": "JUDGE", "word": p["word"]}


def _doc_of(p):
    return json.loads(bytes.fromhex(p["document"][2:]).decode("utf-8"))


def at(tag):
    """An attempt row for the declaration recorded under `tag`."""
    d = BOOK[tag]
    return {"blockNumber": d["blockNumber"], "logIndex": d["logIndex"], "to": d["to"]}


def gen_resolution():
    logs = build_logs()
    lp = write("logs", "chain1", logs)
    g1, j1, j2, j3, j9, v1, v2, v3 = A("g1"), A("j1"), A("j2"), A("j3"), A("j9"), A("v1"), A("v2"), A("v3")
    w9, c1, c2, t1, t2, t3 = A("w9"), A("c1"), A("c2"), A("t1"), A("t2"), A("t3")
    hs = [A("h%d" % i) for i in range(66)]
    k1, k2, k3, m1, m2, n1, n2, o1, o2, p1, p2 = [A(x) for x in ("k1", "k2", "k3", "m1", "m2", "n1", "n2", "o1", "o2", "p1", "p2")]
    z0, z1, e1, e9, f1, q1, q2, q9, s1, s3, sx, u1, u2, u3 = [A(x) for x in ("z0", "z1", "e1", "e9", "f1", "q1", "q2", "q9", "s1", "s3", "sx", "u1", "u2", "u3")]
    gr, gs, gn, gh, gu, gx = A("gr"), A("gs"), A("gn"), A("gh"), A("gu"), A("gx")
    gm, gm2, gq, gq2, gw, gc, ge, gi = A("gm"), A("gm2"), A("gq"), A("gq2"), A("gw"), A("gc"), A("ge"), A("gi")
    gb, gz, gd, gd2, go, go2, gl, gp, gk, ga = A("gb"), A("gz"), A("gd"), A("gd2"), A("go"), A("go2"), A("gl"), A("gp"), A("gk"), A("ga")
    gt, cs, a0, a1, a2, gv, gg = A("gt"), A("cs"), A("a0"), A("a1"), A("a2"), A("gv"), A("gg")
    gg2, gg3, gg4, gg5, gg6, gg7, gj, gf, gy, gvv = A("gg2"), A("gg3"), A("gg4"), A("gg5"), A("gg6"), A("gg7"), A("gj"), A("gf"), A("gy"), A("gvv")
    gcard, gcm, gct, gcv, gcr = A("gcard"), A("gcm"), A("gct"), A("gcv"), A("gcr")
    jd1, jd2, jd3, jd3b, jd4, jd5, jd6, jd7, jd8, jd8b, jd9 = [A("jd%s" % x) for x in ("1", "2", "3", "3b", "4", "5", "6", "7", "8", "8b", "9")]
    gcx, gqo, gk5, gyr, ghr, gbp, gns, gts, jdE, jdG, jdH, gbig, gnest = [A(x) for x in ("gcx", "gqo", "gk5", "gyr", "ghr", "gbp", "gns", "gts", "jdE", "jdG", "jdH", "gbig", "gnest")]
    bbig, bbig2 = A("bbig"), A("bbig2")

    def find(emitter, block):
        return [p for p in logs["publications"] if p["emitter"] == emitter and p["blockNumber"] == block]

    def bw(xs):
        return sorted(set(xs), key=lambda s: s.encode("utf-8"))

    def resolved(final, trail=(), attempts=()):
        return {"attempts": list(attempts), "final": final, "ok": True, "trail": list(trail)}

    def cand(p, decoded):
        return {"blockNumber": p["blockNumber"], "decoded": decoded, "logIndex": p["logIndex"]}

    NONE = {"verdict": "CARD_NONE"}

    def card_of(p):
        return {"blockNumber": p["blockNumber"], "document": card_view(_doc_of(p)), "verdict": "CARD", "word": p["word"]}

    # ---- publication, by kind
    pj = find(g1, 7)[0]
    r2full = _doc_of(pj)
    ans = roster_answer(r2full)
    case("publication", [lp, g1, "roster"], "s1/s5.1: the later of two rosters is the candidate; no status means active",
         {"active": True, "candidate": cand(pj, ans)})
    case("publication", [lp, gr, "roster"], "s5.1: latest status false", {"active": False, "candidate": cand(find(gr, 1)[0], roster_answer(roster(gr, judges=[j1])))})
    case("publication", [lp, gs, "roster"], "s5.1: false then true resumes", {"active": True, "candidate": cand(find(gs, 1)[0], roster_answer(roster(gs, judges=[j1])))})
    case("publication", [lp, gn, "roster"], "s5.1: a status without any publication", {"active": False, "candidate": None})
    case("publication", [lp, A("nobody"), "roster"], "s5.1: an address the logs never name", {"active": True, "candidate": None})
    case("publication", [lp, gh, "roster"], "s1: a publication whose hash fails is of every kind and is the roster candidate, reported with its decoding fault", {"active": True, "candidate": cand(find(gh, 1)[0], fault("E_HASH"))})
    pc3 = find(gcard, 3)[0]
    case("publication", [lp, gcard, "guildcard"], "s1: candidateOf for the card kind is the last card, by (blockNumber, logIndex)", {"active": True, "candidate": cand(pc3, card_answer(_doc_of(pc3)))})
    case("publication", [lp, gcard, "roster"], "s1: the same address asked for its roster candidate", {"active": True, "candidate": cand(find(gcard, 1)[0], roster_answer(roster(gcard, judges=[j1])))})
    case("publication", [lp, gx, "roster"], "s1: a former succession record's kind sits under no reserved prefix and is of another kind; the roster is the candidate", {"active": True, "candidate": cand(find(gx, 1)[0], roster_answer(roster(gx, judges=[j1])))})
    pa = max(find(ga, 4), key=lambda p: p["logIndex"])
    case("publication", [lp, ga, "roster"], "s5.1: two publications in one block are ordered by logIndex", {"active": True, "candidate": cand(pa, roster_answer(roster(ga, judges=[j2], name="b")))})
    case("publication", [lp, gj, "roster"], "s1: a judge document from the guild's address is of another kind; the roster is the roster candidate", {"active": True, "candidate": cand(find(gj, 1)[0], roster_answer(roster(gj, judges=[j1])))})
    case("publication", [lp, gj, "judge"], "s1: the same address asked for its judge candidate", {"active": True, "candidate": cand(find(gj, 2)[0], judge_answer(_doc_of(find(gj, 2)[0])))})
    case("publication", [lp, gj, "guildamanah"], "s1: the same address has no covenant", {"active": True, "candidate": None})
    case("publication", [lp, gj, "guildcard"], "s1: the same address has no card", {"active": True, "candidate": None})
    case("publication", [lp, gf, "roster"], "s1: a kind under no reserved prefix is of another kind for every reader", {"active": True, "candidate": cand(find(gf, 1)[0], roster_answer(roster(gf, judges=[j1])))})
    case("publication", [lp, gy, "roster"], "s1: unreadable bytes are of every kind, so the last publication is the roster candidate and decodes to nothing", {"active": True, "candidate": cand(find(gy, 2)[0], fault("E_JSON"))})
    case("publication", [lp, gy, "verifier"], "s1: unreadable bytes are the candidate of the verifier kind too", {"active": True, "candidate": cand(find(gy, 2)[0], fault("E_JSON"))})
    case("publication", [lp, gy, "guildcard"], "s1: and of the card kind", {"active": True, "candidate": cand(find(gy, 2)[0], fault("E_JSON"))})
    case("publication", [lp, gvv, "roster"], "s1: a verifier service record is of another kind (another reserved prefix)", {"active": True, "candidate": cand(find(gvv, 1)[0], roster_answer(roster(gvv, judges=[j1])))})
    case("publication", [lp, gvv, "verifier"], "s1: the verifier kind's candidate decodes here to an unknown kind, this law reading no service record", {"active": True, "candidate": cand(find(gvv, 2)[0], {"kind": "galeed.verifier/1", "known": False, "ok": True})})
    case("publication", [lp, gu, "roster"], "s1: a later roster version is of the roster kind and is the candidate", {"active": True, "candidate": cand(find(gu, 2)[0], {"kind": "galeed.roster/2", "known": False, "ok": True})})
    case("publication", [lp, k1, "judge"], "s1: a judge document that fails its table is the judge candidate", {"active": True, "candidate": cand(find(k1, 3)[0], fault("E_JUDGE_JUDGE"))})
    case("publication", [lp, gm, "roster"], "s5.1: publication reads the address it is given; the moved guild's own roster", {"active": True, "candidate": cand(find(gm, 1)[0] if _doc_of(find(gm, 1)[0])["kind"] == KR else find(gm, 1)[1], roster_answer(roster(gm, judges=[j3])))})
    case("publication", [lp, g1.upper().replace("0X", "0x"), "roster"], "HARNESS: address outside hex20", None)
    case("publication", [lp, g1], "HARNESS: a missing argument", None)
    case("publication", [lp, g1, "pass"], "HARNESS: a kind outside the five words", None)
    case("publication", [lp, g1, "Roster"], "HARNESS: the kind word compares exactly", None)
    case("publication", [lp, g1, "succession"], "HARNESS: succession is no kind word of this law", None)

    # ---- resolve-identity: the replay of s4.2
    case("resolve-identity", [lp, "1", g1], "s5.3: an address the book never met is its own final, empty trail", resolved(g1))
    case("resolve-identity", [lp, "1", j1], "s4.2: a declaration and the named key's accession move the record; a later declaration from the key that left binds nothing and is an attempt",
         resolved(j2, [j1], [at("j1.late")]))
    case("resolve-identity", [lp, "1", j2], "s5.3: the trail runs from the address asked; the current key's trail is empty", resolved(j2))
    case("resolve-identity", [lp, "1", j3], "s4.2: a declaration no accession took up moves nothing; it is an attempt", resolved(j3, [], [at("j3")]))
    case("resolve-identity", [lp, "1", j9], "s4.2: a key named by declarations it never took up is its own final", resolved(j9))
    case("resolve-identity", [lp, "1", v1], "s4.2: an accession with no declaration pending counts for nothing", resolved(v1))
    case("resolve-identity", [lp, "1", w9], "s4.2: the would-be acceder entered nothing", resolved(w9))
    case("resolve-identity", [lp, "1", v3], "s4.2: a declaration to the emitter itself binds nothing; an attempt", resolved(v3, [], [at("v3")]))
    case("resolve-identity", [lp, "1", c1], "s4.2: a declaration naming a key that has entered binds nothing, so no record returns to a key it held", resolved(c2, [c1], [at("c2.back")]))
    case("resolve-identity", [lp, "1", c2], "s5.3: the same record asked at its current key", resolved(c2, [], [at("c2.back")]))
    case("resolve-identity", [lp, "1", t2], "s5.3: asked in the middle of a record, the trail runs from that key", resolved(t3, [t2], [at("t3.back")]))
    case("resolve-identity", [lp, "1", t1], "s5.3: asked at the root", resolved(t3, [t1, t2], [at("t3.back")]))
    case("resolve-identity", [lp, "1", hs[0]], "s5.3: sixty-five hops resolve; the walk needs no bound", resolved(hs[65], hs[0:65]))
    case("resolve-identity", [lp, "1", hs[1]], "s5.3: sixty-four hops", resolved(hs[65], hs[1:65]))
    case("resolve-identity", [lp, "1", hs[64]], "s5.3: one hop", resolved(hs[65], [hs[64]]))
    case("resolve-identity", [lp, "1", hs[65]], "s5.3: the end of the record is its own final", resolved(hs[65]))
    case("resolve-identity", [lp, "1", k1], "s4.2: declaring again replaces the pending key; the replaced declaration and the late one are attempts; the stale accession counts for nothing",
         resolved(k2, [k1], [at("k1.first"), at("k1.late")]))
    case("resolve-identity", [lp, "1", k3], "s4.2: the key whose declaration was replaced never entered", resolved(k3))
    case("resolve-identity", [lp, "1", m1], "s4.2: records are walked by (blockNumber, logIndex), never by file order", resolved(m2, [m1]))
    case("resolve-identity", [lp, "1", n1], "s4.2: an accession before the declaration in the same block, by logIndex, counts for nothing; the declaration is pending", resolved(n1, [], [at("n1")]))
    case("resolve-identity", [lp, "1", o1], "s4.2: blockNumber is compared first: an accession in an earlier block precedes the declaration whatever their logIndex", resolved(o1, [], [at("o1")]))
    case("resolve-identity", [lp, "1", p1], "s4.1: the root member of an accession is read for its form and by no step of the replay", resolved(p2, [p1]))
    case("resolve-identity", [lp, "1", z0], "s4.2: a declaration to the zero address binds nothing and the emitter does not enter, so it can take up a record later",
         resolved(z1, [z0], [at("z1.zero")]))
    case("resolve-identity", [lp, "1", f1], "s4.2: a declaration naming a key that entered by declaring binds nothing", resolved(f1, [], [at("f1")]))
    case("resolve-identity", [lp, "1", e1], "s4.2: a key that declared is its own final with its pending declaration", resolved(e1, [], [at("e1")]))
    case("resolve-identity", [lp, "1", q1], "s4.2: an accession by a key that has entered counts for nothing", resolved(q1, [], [at("q1")]))
    case("resolve-identity", [lp, "1", q2], "s4.2: that key's own record", resolved(q2, [], [at("q2")]))
    case("resolve-identity", [lp, "1", s1], "s4.3: a stranger naming the new key does not make it enter; the new key takes up the record", resolved(s3, [s1]))
    case("resolve-identity", [lp, "1", sx], "s5.3: the stranger's declaration stays its own attempt", resolved(sx, [], [at("sx")]))
    case("resolve-identity", [lp, "1", u1], "s4.2: a replaced key's accession after the record moved counts for nothing", resolved(u3, [u1], [at("u1.first")]))
    case("resolve-identity", [lp, "1", u2], "s4.2: the replaced key never entered", resolved(u2))
    case("resolve-identity", [lp, "1", a0], "s5.3: attempts of every key of the trail and of the final key, in walk order across keys, the final key's pending declaration included; the declarations accessions took up are not attempts",
         resolved(a2, [a0, a1], [at("a0.x1"), at("a1.x3"), at("a0.x2"), at("a2.x4")]))
    case("resolve-identity", [lp, "1", a1], "s5.3: asked in the middle, the attempts of the keys from there on", resolved(a2, [a1], [at("a1.x3"), at("a2.x4")]))
    case("resolve-identity", [lp, "1", a2], "s5.3: asked at the current key", resolved(a2, [], [at("a2.x4")]))
    case("resolve-identity", [lp, "1", gm], "s5.3: a guild that moved", resolved(gm2, [gm]))
    case("resolve-identity", [lp, "1", bbig], "the reader: book records at block and log index near 2^53 - 1", resolved(bbig2, [bbig]))
    pk0, pk1, rd0, rd2, sk0, sk1 = A("pk0"), A("pk1"), A("rd0"), A("rd2"), A("sk0"), A("sk1")
    case("resolve-identity", [lp, "1", pk1], "s5.3: the final key's pending declaration is an attempt, its named key no key of the record", resolved(pk1, [], [at("pk1")]))
    case("resolve-identity", [lp, "1", pk0], "s5.3: asked at the root of that record", resolved(pk1, [pk0], [at("pk1")]))
    case("resolve-identity", [lp, "1", rd0], "s4.2: a declaration replaced by a later one is an attempt; the later one's key accedes", resolved(rd2, [rd0], [at("rd0.first")]))
    case("resolve-identity", [lp, "1", sk0], "s5.3: two declarations name one key, the second replacing the first; the accession takes up the one that last made the key pending and the first is an attempt",
         resolved(sk1, [sk0], [at("sk0.first")]))
    case("resolve-identity", [lp, "0", j1], "HARNESS: chainId is read for its spelling; the book's records are the logs set's whatever the chain named", resolved(j2, [j1], [at("j1.late")]))
    case("resolve-identity", [lp, "01", j1], "HARNESS: chainId with a leading zero", None)
    case("resolve-identity", [lp, "1", j1[:-1]], "HARNESS: address outside hex20", None)
    case("resolve-identity", [lp, "9007199254740992", j1], "HARNESS: chainId above 2^53 - 1", None)

    # ---- resolve-guild
    v1obj = r2full["bench"]["verifiers"][1]
    ans_g1 = {"bench": bw([v2, v1, v3]), "card": NONE, "covenant": None, "guild": g1, "issued": 10,
              "judges": [{"final": j2, "listed": j1, "trail": [j1]}], "ok": True,
              "protocols": list(r2full["bench"]["protocols"]), "trail": [], "unknownFields": ["alpha", "name", "terms", "zeta"],
              "verifiers": [vrow(v2, final=v2), vrow(v1obj, final=v1), vrow(v3, final=v3)]}

    def plain(guild, judges_rows, **kw):
        d = {"bench": [], "card": NONE, "covenant": None, "guild": guild, "issued": 10, "judges": judges_rows, "ok": True,
             "protocols": [], "trail": [], "unknownFields": [], "verifiers": []}
        d.update(kw)
        return d
    J1ROW = {"final": j2, "listed": j1, "trail": [j1]}
    case("resolve-guild", [lp, "1", g1], "s5.4: the later version supersedes; the bench holds every key of each verifier's record, bytewise, without repeats; name, terms and the extension members are members beyond the table; no card, no covenant", ans_g1)
    case("resolve-guild", [lp, "1", gr], "s5.4 step 2: retired", fault("E_RETIRED"))
    case("resolve-guild", [lp, "1", gs], "s5.1: resumed", plain(gs, [J1ROW]))
    case("resolve-guild", [lp, "1", gn], "s5.4 order: retired is decided before no document", fault("E_RETIRED"))
    case("resolve-guild", [lp, "1", A("nobody")], "s5.4 step 3: no publication", fault("E_NO_DOCUMENT"))
    case("resolve-guild", [lp, "1", gh], "s5.4 step 4: the roster candidate fails decoding", fault("E_HASH"))
    case("resolve-guild", [lp, "1", gu], "s5.4 step 4: the last roster is of a later version of the kind; the older roster is not read", fault("E_UNKNOWN_KIND"))
    case("resolve-guild", [lp, "1", gx], "s1: a former succession record from the guild's address is of another kind; the roster before it is the candidate and resolves", plain(gx, [J1ROW]))
    case("resolve-guild", [lp, "1", gj], "s1/s9: a judge document from the guild's address is of another kind and moves nothing; the roster resolves", plain(gj, [J1ROW]))
    case("resolve-guild", [lp, "1", gf], "s1: a kind under no reserved prefix is of another kind; the roster resolves", plain(gf, [J1ROW]))
    case("resolve-guild", [lp, "1", gy], "s1: unreadable bytes are of every kind and are the roster candidate; the guild is declined, no walk back to the older roster", fault("E_JSON"))
    case("resolve-guild", [lp, "1", gvv], "s1: a verifier's service record from the guild's address is of another kind; the roster resolves", plain(gvv, [J1ROW]))
    case("resolve-guild", [lp, "1", gm], "s5.4 step 1: the guild moved; the successor's roster is read, the trail reported, and the card read at the successor, which published none", plain(gm2, [{"final": j3, "listed": j3, "trail": []}, J1ROW], trail=[gm]))
    case("resolve-guild", [lp, "1", gq], "s5.4 step 3: the successor published nothing", fault("E_NO_DOCUMENT"))
    case("resolve-guild", [lp, "1", gw], "s5.4 step 5: the roster names another guild", fault("E_GUILD_MISMATCH"))
    case("resolve-guild", [lp, "1", gc], "s5.4 step 5: the roster names another chain", fault("E_CHAIN_MISMATCH"))
    case("resolve-guild", [lp, "1", ge], "s5.4: no step reads a time; a roster that writes expires resolves and carries it beyond the table", plain(ge, [J1ROW], unknownFields=["expires"]))
    case("resolve-guild", [lp, "1", gi], "s5.4 step 8: issued is carried as written", plain(gi, [J1ROW], issued=9007199254740991))
    rgb = _doc_of(find(gb, 1)[0])
    ans_gb = plain(gb, [{"final": c2, "listed": c1, "trail": [c1]}, J1ROW, {"final": hs[65], "listed": hs[0], "trail": hs[0:65]}],
                   bench=bw([v3, j1, j2]), protocols=list(rgb["bench"]["protocols"]),
                   verifiers=[vrow(v3, final=v3), vrow(j1, final=j2, trail=[j1]), vrow(j2, final=j2), vrow(rgb["bench"]["verifiers"][3], final=j2, trail=[j1])])
    case("resolve-guild", [lp, "1", gb], "s5.4 steps 6 and 7: every row resolves; the bench dedups an address reached twice; the long row carries its sixty-five trail keys", ans_gb)
    case("resolve-guild", [lp, "1", gz], "s5.4 step 7: no bench member resolves to the empty bench and empty protocols", plain(gz, []))
    case("resolve-guild", [lp, "1", gd], "s5.4 step 2: standing is read at the final address", fault("E_RETIRED"))
    case("resolve-guild", [lp, "1", go], "s5.4 step 2: a retired origin does not retire the successor", plain(go2, [J1ROW], trail=[go]))
    case("resolve-guild", [lp, "1", gl], "s5.4 order: retired before mismatch", fault("E_RETIRED"))
    case("resolve-guild", [lp, "1", gp], "s5.4 order: guild mismatch before chain mismatch", fault("E_GUILD_MISMATCH"))
    case("resolve-guild", [lp, "1", gk], "s5.4 step 5: chain mismatch", fault("E_CHAIN_MISMATCH"))
    case("resolve-guild", [lp, "1", c1], "s5.4 step 1: the guild's record moved to an address that published no roster", fault("E_NO_DOCUMENT"))
    case("resolve-guild", [lp, "2", gc], "chain 2: gc's roster names chain 2; the book's records of the logs set are read as they are", plain(gc, [J1ROW]))
    case("resolve-guild", [lp, "1", ga], "s5.1: two publications in one block; the higher logIndex is the candidate", plain(ga, [{"final": j2, "listed": j2, "trail": []}], unknownFields=["name"]))
    case("publication", [lp, gt, "roster"], "s5.1: two statuses in one block; logIndex breaks the tie", {"active": True, "candidate": cand(find(gt, 1)[0], roster_answer(roster(gt, judges=[j1])))})
    case("resolve-guild", [lp, "1", gt], "s5.1: resumed within one block", plain(gt, [J1ROW]))
    case("resolve-guild", [lp, "1", cs], "s5.4 step 1 never stops; step 2's standing decides", fault("E_RETIRED"))
    rgv = _doc_of(find(gv, 1)[0])
    case("resolve-guild", [lp, "1", gv], "s5.4 step 7: verifier rows carry name and protocols as written; a moved verifier contributes its final and its trail; the guild's protocols are as written, never sorted",
         plain(gv, [J1ROW], bench=bw([v1, c1, c2, j1, j2]), protocols=["0xff" + "ff" * 31, "0x11" + "11" * 31],
               verifiers=[vrow(rgv["bench"]["verifiers"][0], final=v1), vrow(rgv["bench"]["verifiers"][1], final=c2, trail=[c1]), vrow(j1, final=j2, trail=[j1])]))
    pgg3 = find(gg, 3)[0]
    cov_gg = cov_resolved(_doc_of(pgg3), pgg3)
    case("resolve-guild", [lp, "1", gg], "s5.4 step 8: the covenant is the guild's current one (the later of two), with its word, version, text digest, venues and quotes; a pool guild resolves like any other",
         plain(gg, [J1ROW, {"final": j3, "listed": j3, "trail": []}], covenant=cov_gg))
    case("resolve-guild", [lp, "1", gg2], "s5.4 step 3: a covenant alone is no roster; no document", fault("E_NO_DOCUMENT"))
    case("resolve-guild", [lp, "1", gg3], "s5.9: the covenant names another guild; the guild resolves and its covenant member is the stop", plain(gg3, [J1ROW], covenant=fault("E_GUILDAMANAH_GUILD_MISMATCH")))
    case("resolve-guild", [lp, "1", gg4], "s5.9: the covenant is for chain 2; the stop", plain(gg4, [J1ROW], covenant=fault("E_GUILDAMANAH_CHAIN_MISMATCH")))
    case("resolve-guild", [lp, "1", gg5], "s5.9: the last covenant is of a later version of the kind; the stop, no walk back", plain(gg5, [J1ROW], covenant=fault("E_UNKNOWN_KIND")))
    case("resolve-guild", [lp, "1", gg6], "s1: the last covenant fails its hash, and bytes that fail step 1 are of every kind, so they are the roster candidate too; the whole guild is declined at E_HASH, fail-closed", fault("E_HASH"))
    case("resolve-guild", [lp, "1", gg7], "s5.9: the covenant fails its table; the stop with that token", plain(gg7, [J1ROW], covenant=fault("E_GUILDAMANAH_VERSION")))
    case("resolve-guild", [lp, "2", gg4], "chain 2: gg4's roster names chain 1, so the guild is refused there before its covenant is read", fault("E_CHAIN_MISMATCH"))
    case("resolve-guild", [lp, "1", gcard], "s5.4 step 8 with s5.11: the card is the guild's last, with its word and block; the card's members beyond its table carried by key", plain(gcard, [J1ROW], card=card_of(find(gcard, 3)[0])))
    case("resolve-guild", [lp, "1", gcm], "s5.11: a card naming another guild: CARD_MISMATCH; the guild resolves", plain(gcm, [J1ROW], card={"verdict": "CARD_MISMATCH"}))
    case("resolve-guild", [lp, "1", gct], "s5.11: the last card fails its table: CARD_UNREAD with the token, no walk back", plain(gct, [J1ROW], card={"token": "E_GUILDCARD_ISSUED", "verdict": "CARD_UNREAD"}))
    case("resolve-guild", [lp, "1", gcv], "s5.11: the last card is of a later version: CARD_UNREAD with E_UNKNOWN_KIND", plain(gcv, [J1ROW], card={"token": "E_UNKNOWN_KIND", "verdict": "CARD_UNREAD"}))
    pcr = find(gcr, 1)[0]
    case("resolve-guild", [lp, "1", gcr], "s1: a card before the roster; each reading takes its own kind", plain(gcr, [J1ROW], card=card_of(pcr)))
    case("resolve-guild", [lp, "1", gyr], "s5.4 step 8: unreadable bytes before the roster are the covenant and card candidates of every kind; the guild resolves, its covenant member the stop E_JSON and its card CARD_UNREAD",
         plain(gyr, [J1ROW], covenant=fault("E_JSON"), card={"token": "E_JSON", "verdict": "CARD_UNREAD"}))
    case("resolve-guild", [lp, "1", ghr], "s5.4 step 8: a wrong-word publication before the roster: the covenant member is the stop E_HASH and the card CARD_UNREAD E_HASH",
         plain(ghr, [J1ROW], covenant=fault("E_HASH"), card={"token": "E_HASH", "verdict": "CARD_UNREAD"}))
    case("resolve-guild", [lp, "1", g1, "100"], "HARNESS: a fourth argument is beyond the list", None)
    case("resolve-guild", [lp, "1"], "HARNESS: a missing argument", None)
    case("resolve-guild", [lp, "x", g1], "HARNESS: chainId not an int spelling", None)
    case("resolve-identity", [lp, "1", g1, "x"], "HARNESS: an argument beyond the list", None)
    case("resolve-identity", [lp, "1"], "HARNESS: a missing argument", None)
    case("publication", [lp, g1, "roster", "x"], "HARNESS: an argument beyond the list", None)
    case("publication", [lp, "0X" + g1[2:], "roster"], "HARNESS: an uppercase 0X prefix", None)

    # ---- guild-card
    case("guild-card", [lp, gcard], "s5.11: the last card of the address", card_of(find(gcard, 3)[0]))
    case("guild-card", [lp, g1], "s5.11: no card", NONE)
    case("guild-card", [lp, gcm], "s5.11: a card naming another guild", {"verdict": "CARD_MISMATCH"})
    case("guild-card", [lp, gct], "s5.11: a card failing its table", {"token": "E_GUILDCARD_ISSUED", "verdict": "CARD_UNREAD"})
    case("guild-card", [lp, gcv], "s5.11: a later card kind", {"token": "E_UNKNOWN_KIND", "verdict": "CARD_UNREAD"})
    case("guild-card", [lp, gy], "s1/s5.11: unreadable bytes after a good card are the card candidate", {"token": "E_JSON", "verdict": "CARD_UNREAD"})
    pgm_card = [p for p in find(gm, 1) if _doc_of(p)["kind"] == KC][0]
    case("guild-card", [lp, gm], "s5.11: guildCardOf reads the address it is given and follows no record of the book", card_of(pgm_card))
    case("guild-card", [lp, gm2], "s5.11: the successor published no card", NONE)
    case("guild-card", [lp, gk5], "s2.1 step 3: a kind that is not a string is of every kind; CARD_UNREAD with E_KIND", {"token": "E_KIND", "verdict": "CARD_UNREAD"})
    case("guild-card", [lp], "HARNESS: a missing argument", None)
    case("guild-card", [lp, gcard, "x"], "HARNESS: an argument beyond the list", None)
    case("guild-card", [lp, gcard.upper().replace("0X", "0x")], "HARNESS: address outside hex20", None)

    # ---- judge, resolve-judge
    pjd1 = find(jd1, 2)[0]
    J1 = {"blockNumber": 2, "document": judge_view(_doc_of(pjd1)), "verdict": "JUDGE", "word": pjd1["word"]}
    case("judge", [lp, jd1], "s5.7: a judge's document, read at its own address", J1)
    case("judge", [lp, A("nobody")], "s5.7: no publication", {"verdict": "JUDGE_NONE"})
    case("judge", [lp, jd2], "s5.7: the document's judge is another address", {"verdict": "JUDGE_MISMATCH"})
    case("judge", [lp, jd4], "s5.7: the last judge document fails its table; the whole reading is declined, no walk back", {"token": "E_JUDGE_LEDGER", "verdict": "JUDGE_UNREAD"})
    case("judge", [lp, jd5], "s5.7: the last publication of the judge kind is a later version; unread with E_UNKNOWN_KIND", {"token": "E_UNKNOWN_KIND", "verdict": "JUDGE_UNREAD"})
    case("judge", [lp, jd6], "s1/s5.7: unreadable bytes are of every kind, so they are the candidate; unread at step 2", {"token": "E_JSON", "verdict": "JUDGE_UNREAD"})
    case("judge", [lp, k1], "s5.7: a document that fails at judge", {"token": "E_JUDGE_JUDGE", "verdict": "JUDGE_UNREAD"})
    case("judge", [lp, gx], "s5.7: a former succession record is of another kind; no document", {"verdict": "JUDGE_NONE"})
    case("judge", [lp, jd3], "s5.7: judgeOf follows no record; the moved judge's own document is read at its old address", {"blockNumber": 2, "document": judge_view(_doc_of(find(jd3, 2)[0])), "verdict": "JUDGE", "word": find(jd3, 2)[0]["word"]})
    pjd9 = find(jd9, 3)[0]
    case("judge", [lp, jd9], "s1: an address with rosters and a judge document; the judge reader selects the document whatever came after it", {"blockNumber": 3, "document": judge_view(_doc_of(pjd9)), "verdict": "JUDGE", "word": pjd9["word"]})
    case("resolve-guild", [lp, "1", jd9], "s1: the same address asked as a guild resolves its last roster", plain(jd9, [{"final": j2, "listed": j2, "trail": []}]))
    case("judge", [lp, jd7], "s5.7: standing is not read here; a retired judge's document reads", {"blockNumber": 2, "document": judge_view(_doc_of(find(jd7, 2)[0])), "verdict": "JUDGE", "word": find(jd7, 2)[0]["word"]})
    case("judge", [lp], "HARNESS: a missing argument", None)
    case("judge", [lp, jd1, "x"], "HARNESS: an argument beyond the list", None)
    case("judge", [lp, jd1.upper().replace("0X", "0x")], "HARNESS: address outside hex20", None)
    case("resolve-judge", [lp, "1", jd1], "s5.8: a judge that never moved", {"active": True, "final": jd1, "judge": J1, "ok": True, "trail": []})
    pjd3b = find(jd3b, 4)[0]
    case("resolve-judge", [lp, "1", jd3], "s5.8: the judge moved; the document at the final address, the trail reported", {"active": True, "final": jd3b, "judge": {"blockNumber": 4, "document": judge_view(_doc_of(pjd3b)), "verdict": "JUDGE", "word": pjd3b["word"]}, "ok": True, "trail": [jd3]})
    case("resolve-judge", [lp, "1", jd7], "s5.8: active read at the final address; the document reads whatever the standing", {"active": False, "final": jd7, "judge": {"blockNumber": 2, "document": judge_view(_doc_of(find(jd7, 2)[0])), "verdict": "JUDGE", "word": find(jd7, 2)[0]["word"]}, "ok": True, "trail": []})
    case("resolve-judge", [lp, "1", jd8], "s5.8: the judge moved to a key that published no document: JUDGE_NONE at the final address, the old document unread", {"active": True, "final": jd8b, "judge": {"verdict": "JUDGE_NONE"}, "ok": True, "trail": [jd8]})
    case("resolve-judge", [lp, "1", hs[0]], "s5.8: a long record resolves", {"active": True, "final": hs[65], "judge": {"verdict": "JUDGE_NONE"}, "ok": True, "trail": hs[0:65]})
    case("resolve-judge", [lp, "1", A("nobody")], "s5.8: an address with nothing resolves to itself with JUDGE_NONE", {"active": True, "final": A("nobody"), "judge": {"verdict": "JUDGE_NONE"}, "ok": True, "trail": []})
    case("resolve-judge", [lp, "2", jd3], "s5.8 on chain 2: the book's records of the logs set are read as they are", {"active": True, "final": jd3b, "judge": {"blockNumber": 4, "document": judge_view(_doc_of(pjd3b)), "verdict": "JUDGE", "word": pjd3b["word"]}, "ok": True, "trail": [jd3]})
    case("resolve-judge", [lp, "1", "100", jd1], "HARNESS: a fourth argument is beyond the list", None)
    case("resolve-judge", [lp, "01", jd1], "HARNESS: chainId with a leading zero", None)
    case("resolve-judge", [lp, "1"], "HARNESS: a missing argument", None)
    case("resolve-judge", [lp, jd1, "1"], "HARNESS: the arguments in another order (an address where an int is due)", None)

    # ---- covenant
    case("covenant", [lp, "1", gg], "s5.9: the guild's current covenant, the later of two", {"covenant": cov_gg, "ok": True})
    pgg2 = find(gg2, 1)[0]
    case("covenant", [lp, "1", gg2], "s5.9: a covenant with the required members alone, published from an address without a roster", {"covenant": cov_resolved(_doc_of(pgg2), pgg2), "ok": True})
    case("covenant", [lp, "1", g1], "s5.9: no covenant published", {"covenant": None, "ok": True})
    case("covenant", [lp, "1", gg3], "s5.9: guild mismatch", fault("E_GUILDAMANAH_GUILD_MISMATCH"))
    case("covenant", [lp, "1", gg4], "s5.9: chain mismatch", fault("E_GUILDAMANAH_CHAIN_MISMATCH"))
    case("covenant", [lp, "2", gg4], "s5.9 on chain 2: the covenant names chain 2 and reads", {"covenant": cov_resolved(_doc_of(find(gg4, 2)[0]), find(gg4, 2)[0]), "ok": True})
    case("covenant", [lp, "1", gg5], "s5.9: a later version of the kind", fault("E_UNKNOWN_KIND"))
    case("covenant", [lp, "1", gg6], "s5.9: the last covenant fails its hash", fault("E_HASH"))
    case("covenant", [lp, "1", gg7], "s5.9: the table's token", fault("E_GUILDAMANAH_VERSION"))
    case("covenant", [lp, "1", gy], "s1: unreadable bytes are the candidate of every kind; a stop", fault("E_JSON"))
    case("covenant", [lp, "1", gm], "s5.9: covenantOf follows no record; the moved guild's own address has no covenant", {"covenant": None, "ok": True})
    case("covenant", [lp, "1", gcard], "s1: a card is of another kind than a covenant", {"covenant": None, "ok": True})
    case("covenant", [lp, "1"], "HARNESS: a missing argument", None)
    case("covenant", [lp, "1", gg, "x"], "HARNESS: an argument beyond the list", None)
    case("covenant", [lp, "x", gg], "HARNESS: chainId not an int spelling", None)

    # ---- quote
    def T(slug, Q, args, note, predicted):
        path = write("quotes", slug, Q)
        case("quote", [path] + args, note, predicted, path=path)
    VEN, COD, TOK = A("venue"), "0x" + "cd" * 32, A("tok")
    full = {"1:%s:%s" % (VEN, TOK): "0x01", "1:" + VEN: "0x02", "1:%s:%s" % (COD, TOK): "0x03", "1:" + COD: "0x04", "1": "0x05", "2:" + VEN: "0x06"}
    T("address_token_first", full, ["1", VEN, COD, TOK], "s5.10: the five keys in order; the address and token key is first", {"key": "1:%s:%s" % (VEN, TOK), "quote": "0x01", "verdict": "QUOTE"})
    T("address_without_token", full, ["1", VEN, COD], "s5.10: with no token the token keys are skipped; the address key", {"key": "1:" + VEN, "quote": "0x02", "verdict": "QUOTE"})
    no_addr = {k: v for k, v in full.items() if VEN not in k}
    T("code_token", no_addr, ["1", VEN, COD, TOK], "s5.10: no address key; the code and token key", {"key": "1:%s:%s" % (COD, TOK), "quote": "0x03", "verdict": "QUOTE"})
    T("code_without_token", no_addr, ["1", VEN, COD], "s5.10: no address key, no token; the code key", {"key": "1:" + COD, "quote": "0x04", "verdict": "QUOTE"})
    T("chain_alone", {"1": "0x05", "2:" + VEN: "0x06"}, ["1", VEN, COD, TOK], "s5.10: the chain key alone", {"key": "1", "quote": "0x05", "verdict": "QUOTE"})
    T("none", {"2:" + VEN: "0x06"}, ["1", VEN, COD, TOK], "s5.10: none of the five keys", {"verdict": "QUOTE_NONE"})
    T("empty_object", {}, ["1", VEN, COD], "s5.10: the empty quotes object", {"verdict": "QUOTE_NONE"})
    T("empty_quote_present", {"1:" + VEN: "0x"}, ["1", VEN, COD], "s7: a quote of zero bytes, spelled 0x, is a quote present", {"key": "1:" + VEN, "quote": "0x", "verdict": "QUOTE"})
    T("form_fault_uppercase", {"1:" + VEN: "0xAB"}, ["1", VEN, COD], "s5.10: the value under the first key reached is not a quote spelling", {"key": "1:" + VEN, "verdict": "E_QUOTE_FORM"})
    T("form_fault_odd", {"1:" + VEN: "0xabc"}, ["1", VEN, COD], "s5.10: an odd number of digits", {"key": "1:" + VEN, "verdict": "E_QUOTE_FORM"})
    T("form_fault_int", {"1:" + VEN: 5}, ["1", VEN, COD], "s5.10: a value that is no string", {"key": "1:" + VEN, "verdict": "E_QUOTE_FORM"})
    T("form_fault_no_prefix", {"1:" + VEN: "abcd"}, ["1", VEN, COD], "s5.10: no 0x", {"key": "1:" + VEN, "verdict": "E_QUOTE_FORM"})
    T("form_fault_shadows_later_good", {"1:%s:%s" % (VEN, TOK): "zz", "1:" + VEN: "0x02"}, ["1", VEN, COD, TOK], "s5.10: the first key reached decides; a bad value there is E_QUOTE_FORM, never a fall-through", {"key": "1:%s:%s" % (VEN, TOK), "verdict": "E_QUOTE_FORM"})
    T("token_key_skipped_without_token", {"1:%s:%s" % (VEN, TOK): "0x01"}, ["1", VEN, COD], "s5.10: a key naming a token is skipped where token is null; none", {"verdict": "QUOTE_NONE"})
    T("other_chain_only", {"2:" + VEN: "0x06", "2": "0x07"}, ["1", VEN, COD], "s5.10: keys of another chain are data", {"verdict": "QUOTE_NONE"})
    T("untested_keys_data", {"nonsense": 5, "1:" + VEN + ":" + TOK + ":x": 1, "01:" + VEN: "0x", "1:" + VEN: "0x02"}, ["1", VEN, COD], "s5.10: keys the order never reaches are data, whatever their shape", {"key": "1:" + VEN, "quote": "0x02", "verdict": "QUOTE"})
    T("chain_key_spelling", {"01": "0x05", "1": "0x08"}, ["1", VEN, COD], "s5.10: the chain key is the int's s3.2 spelling; 01 is another key", {"key": "1", "quote": "0x08", "verdict": "QUOTE"})
    T("large_chain", {"9007199254740991:" + VEN: "0x09"}, ["9007199254740991", VEN, COD], "s5.10: the largest chain id", {"key": "9007199254740991:" + VEN, "quote": "0x09", "verdict": "QUOTE"})
    T("case_of_key_address", {"1:" + VEN.upper().replace("0X", "0x"): "0x02"}, ["1", VEN, COD], "s5.10: keys compare bytewise; a checksummed address is another key", {"verdict": "QUOTE_NONE"})
    T("misuse_root_array", [], ["1", VEN, COD], "HARNESS: quotes root not an object", None)
    T("misuse_unreadable", b"{", ["1", VEN, COD], "HARNESS: a file the reader refuses", None)
    T("misuse_token_upper", {"1": "0x"}, ["1", VEN, COD, TOK.upper().replace("0X", "0x")], "HARNESS: token outside hex20", None)
    T("misuse_codehash_hex20", {"1": "0x"}, ["1", VEN, VEN], "HARNESS: codeHash outside hex32", None)
    T("misuse_too_many", {"1": "0x"}, ["1", VEN, COD, TOK, "x"], "HARNESS: an argument beyond the list", None)
    T("misuse_too_few", {"1": "0x"}, ["1", VEN], "HARNESS: a missing argument", None)
    T("misuse_chain_spelling", {"1": "0x"}, ["+1", VEN, COD], "HARNESS: chainId not an int spelling", None)
    T("guild_quotes_from_logs", GUILD_QUOTES, ["1", v1, "0x" + "cd" * 32, v2], "s5.10 over the pool guild's quotes of the logs set: the address key precedes the code and token key", {"key": "1:" + v1, "quote": "0x0001", "verdict": "QUOTE"})
    T("guild_quotes_code_token", {k: v for k, v in GUILD_QUOTES.items() if v1 not in k or ":" + v2 in k}, ["1", A("venue2"), "0x" + "cd" * 32, v2], "s5.10: the code and token key where the address key is absent", {"key": "1:" + ("0x" + "cd" * 32) + ":" + v2, "quote": "0x", "verdict": "QUOTE"})

    # ---- bench
    def B(slug, value, note, predicted, chain="1"):
        path = write("bench", slug, value)
        case("bench", [lp, chain, path], note, {"bench": predicted}, path=path)

    B("null", None, "s5.5: null means no bench question", None)
    B("pointer_ok", "galeed:guild:1:" + g1, "s5.5: a pointer to a resolving guild yields its bench", bw([v2, v1, v3]))
    B("pointer_no_bench_member", "galeed:guild:1:" + gz, "s5.5: a roster without bench yields the empty array, a resolution", [])
    B("pointer_other_chain", "galeed:guild:2:" + g1, "s5.5: a pointer naming another chain than the scan", "unresolved")
    B("pointer_retired", "galeed:guild:1:" + gr, "s5.5: a guild that does not resolve", "unresolved")
    B("pointer_no_document", "galeed:guild:1:" + A("nobody"), "s5.5: a pointer to nothing", "unresolved")
    B("pointer_checksummed", "galeed:guild:1:" + g1[:10].upper().replace("0X", "0x") + g1[10:], "s6.1: not a pointer", "unresolved")
    B("pointer_leading_zero_chain", "galeed:guild:01:" + g1, "s6.1: chain id with a leading zero is no pointer", "unresolved")
    B("pointer_prefix_case", "GALEED:guild:1:" + g1, "s6.1: the prefix compares bytewise", "unresolved")
    B("pointer_trailing_space", "galeed:guild:1:" + g1 + " ", "s6.1: surrounding whitespace", "unresolved")
    B("pointer_chain_zero", "galeed:guild:0:" + g1, "s6.1: chain 0 is a pointer; it names another chain than the scan", "unresolved", chain="1")
    B("pointer_moved_to_nothing", "galeed:guild:1:" + c1, "s5.5: the guild's record moved to an address with no roster", "unresolved")
    B("list_ok", [j1, v3, j3], "s5.5: a list resolves each element, every element contributing every key of its record", bw([j1, j2, v3, j3]))
    B("list_empty", [], "s5.5: the empty list is the empty bench", [])
    B("list_bad_element", [j1, "0x12"], "s5.5: an element outside hex20 makes the list unresolved", "unresolved")
    B("list_element_int", [1], "s5.5: an element that is no string", "unresolved")
    B("list_repeated", [j1, j1, j2], "s5.5: repeats collapse", bw([j1, j2]))
    B("list_long_record", [hs[0]], "s5.5: an element whose record is long contributes all sixty-six keys", bw(hs))
    B("list_mid_record", [t2], "s5.5: an element in the middle of a record contributes every key of its record, the keys before it included", bw([t1, t2, t3]))
    pk0, pk1, pk2, gpk, la0, la1, gla = A("pk0"), A("pk1"), A("pk2"), A("gpk"), A("la0"), A("la1"), A("gla")
    B("list_key_with_pending", [pk1], "s5.3, s5.5: a key's pending key is no key of its record", bw([pk0, pk1]))
    B("list_pending_key", [pk2], "s5.5: a pending key that never acceded is its own record", [pk2])
    B("list_key_moved_after_roster", [la0], "s5.5: the record is read after every record of the logs set, a key that acceded later included", bw([la0, la1]))
    B("pointer_verifier_with_pending", "galeed:guild:1:" + gpk, "s5.4 step 7: the bench holds no pending key", bw([pk0, pk1]))
    B("value_number", 5, "s5.5: any other value is unresolved", "unresolved")
    B("value_object", {"pointer": "galeed:guild:1:" + g1}, "s5.5: an object is unresolved", "unresolved")
    B("value_true", True, "s5.5: a bool is unresolved", "unresolved")
    B("pointer_roster_writes_expires", "galeed:guild:1:" + ge, "s5.5: no row reads a time; a roster that writes expires resolves", [])
    B("pointer_moved_guild", "galeed:guild:1:" + gm, "s5.5: a moved guild resolves through its successor", [])
    B("pointer_guild_with_verifier_objects", "galeed:guild:1:" + gv, "s5.5: the bench is the set of every key of the verifier rows' records", bw([v1, c1, c2, j1, j2]))
    B("pointer_chain_above_range", "galeed:guild:9007199254740992:" + g1, "s6.1: a chain id at or above 2^53 makes the string no pointer", "unresolved")
    B("pointer_guild_covenant_stop", "galeed:guild:1:" + gg3, "s5.5: a guild whose covenant stops still resolves; the bench is the roster's", [])
    B("pointer_guild_card_unread", "galeed:guild:1:" + gct, "s5.5: a guild whose card is unread still resolves", [])
    path = write("bench", "unreadable", b"{")
    case("bench", [lp, "1", path], "HARNESS: a bench file the reader refuses", None, path=path)
    case("bench", [lp, "1"], "HARNESS: a missing argument", None, path="(none)")
    case("bench", [lp, "1", "bench/b001_null.json", "x"], "HARNESS: an argument beyond the list", None, path="bench/b001_null.json")
    case("bench", [lp, "1", "100", "bench/b001_null.json"], "HARNESS: an argument beyond the list; the command reads no clock", None, path="bench/b001_null.json")

    # ---- candidates
    def Q(slug, q, note, predicted, chain="1"):
        path = write("query", slug, q)
        case("candidates", [lp, chain, path], note, predicted, path=path)

    def guild_row(pointer, answer):
        d = {"pointer": pointer}
        d.update(answer)
        return d

    row_g1 = guild_row("galeed:guild:1:" + g1, ans_g1)
    ans_gm = plain(gm2, [{"final": j3, "listed": j3, "trail": []}, J1ROW], trail=[gm])
    row_gm = guild_row("galeed:guild:1:" + gm, ans_gm)
    Q("union_dedup", {"pointers": ["galeed:guild:1:" + g1, "galeed:guild:1:" + gm]}, "s5.6: pointer order then roster order, an address appended once",
      {"candidates": [j2, j3], "perPointer": [row_g1, row_gm]})
    Q("deny_by_trail", {"pointers": ["galeed:guild:1:" + gm], "deny": [j1]}, "s5.6: denying an old key denies its successor",
      {"candidates": [j3], "perPointer": [row_gm]})
    Q("deny_by_final", {"pointers": ["galeed:guild:1:" + gm], "deny": [j2]}, "s5.6: deny by the final address",
      {"candidates": [j3], "perPointer": [row_gm]})
    Q("allow_after_deny", {"pointers": ["galeed:guild:1:" + gm], "allow": [j2, j9, j9], "deny": [j2]}, "s5.6: allow is applied after deny and repeats collapse",
      {"candidates": [j3, j2, j9], "perPointer": [row_gm]})
    Q("allow_only", {"pointers": [], "allow": [j9]}, "s5.6: no pointer at all", {"candidates": [j9], "perPointer": []})
    pk0, pk1, pk2, gpk = A("pk0"), A("pk1"), A("pk2"), A("gpk")
    la0, la1, gla, rd0, rd1, rd2, grd = A("la0"), A("la1"), A("gla"), A("rd0"), A("rd1"), A("rd2"), A("grd")
    zr1, gzr, gs0, gs1 = A("zr1"), A("gzr"), A("gs0"), A("gs1")
    row_gpk = guild_row("galeed:guild:1:" + gpk, plain(gpk, [{"final": pk1, "listed": pk1, "trail": []}], bench=bw([pk0, pk1]), verifiers=[vrow(pk1, final=pk1)]))
    Q("deny_pending_key", {"pointers": ["galeed:guild:1:" + gpk], "deny": [pk2]}, "s5.6: a judge's pending key is no key of its record, and denying it denies nothing",
      {"candidates": [pk1], "perPointer": [row_gpk]})
    Q("deny_root_of_listed_final", {"pointers": ["galeed:guild:1:" + gpk], "deny": [pk0]}, "s5.6: denying the root denies the judge listed at its final key",
      {"candidates": [], "perPointer": [row_gpk]})
    row_gla = guild_row("galeed:guild:1:" + gla, plain(gla, [{"final": la1, "listed": la0, "trail": [la0]}]))
    Q("deny_key_acceded_after_roster", {"pointers": ["galeed:guild:1:" + gla], "deny": [la1]}, "s5.6: a key that acceded after the roster is a key of the record",
      {"candidates": [], "perPointer": [row_gla]})
    row_grd = guild_row("galeed:guild:1:" + grd, plain(grd, [{"final": rd2, "listed": rd0, "trail": [rd0]}]))
    Q("deny_replaced_declaration_target", {"pointers": ["galeed:guild:1:" + grd], "deny": [rd1]}, "s5.6: the key of a replaced declaration never entered the record; denying it denies nothing",
      {"candidates": [rd2], "perPointer": [row_grd]})
    row_gzr = guild_row("galeed:guild:1:" + gzr, plain(gzr, [{"final": zr1, "listed": zr1, "trail": []}]))
    Q("deny_zero_root", {"pointers": ["galeed:guild:1:" + gzr], "deny": [ZERO]}, "s5.6: the zero address as the root of a judge's record is a key like any other",
      {"candidates": [], "perPointer": [row_gzr]})
    row_gs = guild_row("galeed:guild:1:" + gs0, plain(gs1, [{"final": gs1, "listed": gs0, "trail": [gs0]}, J1ROW], trail=[gs0]))
    Q("guild_own_row_contributes", {"pointers": ["galeed:guild:1:" + gs0]}, "s5.6: a row whose record is the guild's own contributes its final like any other",
      {"candidates": [gs1, j2], "perPointer": [row_gs]})
    Q("deny_guild_own_key", {"pointers": ["galeed:guild:1:" + gs0], "deny": [gs1]}, "s5.6: a row whose record is the guild's own is a row like any other; the guild still resolves",
      {"candidates": [j2], "perPointer": [row_gs]})
    Q("bad_pointer", {"pointers": ["galeed:guild:1:" + g1, "not a pointer"]}, "s5.6: a string that is no pointer contributes nothing and is reported",
      {"candidates": [j2], "perPointer": [row_g1, {"ok": False, "pointer": "not a pointer", "token": "E_POINTER"}]})
    Q("chain_mismatch_pointer", {"pointers": ["galeed:guild:2:" + g1]}, "s5.6: a pointer for another chain",
      {"candidates": [], "perPointer": [{"ok": False, "pointer": "galeed:guild:2:" + g1, "token": "E_POINTER_CHAIN"}]})
    pieces = ["galeed:guild:1", "galeed:guild::" + g1, "galeed:guild:a:" + g1, "galeed:guild:9007199254740992:" + g1, "galeed:guild:1:" + g1[:-1], "galeed:guild:9007199254740991:" + g1, "galeed:guild:1:" + g1 + ":1", "galeed:guild:1:" + g1.upper().replace("0X", "0x"), " galeed:guild:1:" + g1]
    Q("pointer_pieces", {"pointers": pieces}, "s6.1: the missing colon, an empty chain id, a non-digit chain id, a chain id at 2^53, a short address, a chain id at 2^53 - 1 (a pointer, for another chain), a trailing field, an uppercase address, leading whitespace",
      {"candidates": [], "perPointer": [{"ok": False, "pointer": s, "token": ("E_POINTER_CHAIN" if s.startswith("galeed:guild:9007199254740991:") else "E_POINTER")} for s in pieces]})
    Q("deny_guild_own_trail", {"pointers": ["galeed:guild:1:" + gm], "deny": [gm]}, "s5.6: deny is read against judges rows alone and suppresses no guild, whatever the guild's own trail carries",
      {"candidates": [j3, j2], "perPointer": [row_gm]})
    Q("misuse_allow_null", {"pointers": [], "allow": None}, "HARNESS: allow present with the value null is a violation", None)
    Q("misuse_pointers_absent_only", {}, "HARNESS: pointers required even with nothing else", None)
    case("candidates", [lp, "1"], "HARNESS: a missing argument", None, path="(none)")
    case("candidates", [lp, "1", "query/q001_union_dedup.json", "x"], "HARNESS: an argument beyond the list", None, path="query/q001_union_dedup.json")
    Q("unresolved_guild", {"pointers": ["galeed:guild:1:" + gr, "galeed:guild:1:" + gm]}, "s5.6: a refused guild contributes nothing and its answer is reported",
      {"candidates": [j3, j2], "perPointer": [guild_row("galeed:guild:1:" + gr, fault("E_RETIRED")), row_gm]})
    Q("duplicate_pointer", {"pointers": ["galeed:guild:1:" + gm, "galeed:guild:1:" + gm]}, "s5.6: a repeated pointer is reported twice and contributes once",
      {"candidates": [j3, j2], "perPointer": [row_gm, row_gm]})
    Q("moved_rows", {"pointers": ["galeed:guild:1:" + gb]}, "s5.6: every row names its final address",
      {"candidates": [c2, j2, hs[65]], "perPointer": [guild_row("galeed:guild:1:" + gb, ans_gb)]})
    Q("deny_mid_trail", {"pointers": ["galeed:guild:1:" + gb], "deny": [hs[30]]}, "s5.6: deny reaches any key of a row's record, one in the middle of a long record included",
      {"candidates": [c2, j2], "perPointer": [guild_row("galeed:guild:1:" + gb, ans_gb)]})
    Q("moved_guild_pointer", {"pointers": ["galeed:guild:1:" + c1]}, "s5.6: a pointer whose guild moved to an address with no roster",
      {"candidates": [], "perPointer": [guild_row("galeed:guild:1:" + c1, fault("E_NO_DOCUMENT"))]})
    Q("pool_guild_candidates", {"pointers": ["galeed:guild:1:" + gg]}, "s5.6: a pool guild's judges are candidates like any other's; the covenant travels in the per-pointer answer",
      {"candidates": [j2, j3], "perPointer": [guild_row("galeed:guild:1:" + gg, plain(gg, [J1ROW, {"final": j3, "listed": j3, "trail": []}], covenant=cov_gg))]})
    Q("card_guild_candidates", {"pointers": ["galeed:guild:1:" + gcard]}, "s5.6: a guild's card travels in the per-pointer answer and moves no candidate",
      {"candidates": [j2], "perPointer": [guild_row("galeed:guild:1:" + gcard, plain(gcard, [J1ROW], card=card_of(find(gcard, 3)[0])))]})
    Q("misuse_pointers_absent", {"allow": [j1]}, "HARNESS: pointers required", None)
    Q("misuse_pointer_int", {"pointers": [1]}, "HARNESS: pointers are strings", None)
    Q("misuse_allow_upper", {"pointers": [], "allow": [j1.upper().replace("0X", "0x")]}, "HARNESS: allow elements are hex20", None)
    Q("misuse_deny_string", {"pointers": [], "deny": j1}, "HARNESS: deny an array", None)
    Q("misuse_root_array", [], "HARNESS: query root an object", None)
    q = {"pointers": ["galeed:guild:1:" + gm], "other": 1}
    Q("extra_member_ignored", q, "HARNESS: members beyond the three are ignored", {"candidates": [j3, j2], "perPointer": [row_gm]})
    case("candidates", [lp, "1", "100", "query/q001_union_dedup.json"], "HARNESS: a clock argument is no argument of this command", None, path="query/q001_union_dedup.json")

    # ---- logs file misuse
    def L(slug, mutate, cmd, args, note, predicted=None):
        bad = json.loads(json.dumps(logs))
        mutate(bad)
        p = write("logs", slug, bad)
        case(cmd, [p] + args, note, predicted, path=p)

    def dup(b):
        b["publications"][0]["logIndex"] = b["publications"][1]["logIndex"]
        b["publications"][0]["blockNumber"] = b["publications"][1]["blockNumber"]
    L("dup_pair", dup, "publication", [g1, "roster"], "HARNESS: two records share (blockNumber, logIndex)")

    def dup_across(b):
        b["statuses"][0]["logIndex"] = b["publications"][0]["logIndex"]
        b["statuses"][0]["blockNumber"] = b["publications"][0]["blockNumber"]
    L("dup_pair_across", dup_across, "resolve-identity", ["1", g1], "HARNESS: a status and a publication share the pair")

    def dup_book(b):
        b["declarations"][0]["logIndex"] = b["accessions"][0]["logIndex"]
        b["declarations"][0]["blockNumber"] = b["accessions"][0]["blockNumber"]
    L("dup_pair_book", dup_book, "resolve-identity", ["1", g1], "HARNESS: a declaration and an accession share the pair")

    def dup_decl_pub(b):
        b["declarations"][0]["logIndex"] = b["publications"][0]["logIndex"]
        b["declarations"][0]["blockNumber"] = b["publications"][0]["blockNumber"]
    L("dup_pair_declaration_publication", dup_decl_pub, "judge", [jd1], "HARNESS: a declaration and a publication share the pair")
    def dup_ab(src, si, dst, di):
        def f(b):
            b[src][si]["logIndex"] = b[dst][di]["logIndex"]
            b[src][si]["blockNumber"] = b[dst][di]["blockNumber"]
        return f
    L("dup_pair_publication_accession", dup_ab("accessions", 0, "publications", 0), "resolve-identity", ["1", g1], "HARNESS: a publication and an accession share the pair")
    L("dup_pair_status_declaration", dup_ab("declarations", 0, "statuses", 0), "resolve-identity", ["1", g1], "HARNESS: a status and a declaration share the pair")
    L("dup_pair_status_accession", dup_ab("accessions", 0, "statuses", 0), "resolve-identity", ["1", g1], "HARNESS: a status and an accession share the pair")
    L("dup_pair_statuses", dup_ab("statuses", 0, "statuses", 1), "resolve-identity", ["1", g1], "HARNESS: two statuses share the pair")
    L("dup_pair_declarations", dup_ab("declarations", 0, "declarations", 1), "resolve-identity", ["1", g1], "HARNESS: two declarations share the pair")
    L("dup_pair_accessions", dup_ab("accessions", 0, "accessions", 1), "resolve-identity", ["1", g1], "HARNESS: two accessions share the pair")
    L("publications_object", lambda b: b.__setitem__("publications", {}), "resolve-identity", ["1", g1], "HARNESS: publications an array")
    L("statuses_null", lambda b: b.__setitem__("statuses", None), "resolve-identity", ["1", g1], "HARNESS: statuses an array")
    L("accessions_string", lambda b: b.__setitem__("accessions", "x"), "resolve-identity", ["1", g1], "HARNESS: accessions an array")
    L("accession_not_object", lambda b: b["accessions"].__setitem__(0, [1]), "resolve-identity", ["1", g1], "HARNESS: an accession is an object")
    L("declaration_not_object", lambda b: b["declarations"].__setitem__(0, 5), "current-at", [g1, "1", "0"], "HARNESS: a declaration is an object")
    L("no_statuses", lambda b: b.pop("statuses"), "resolve-guild", ["1", g1], "HARNESS: statuses required")
    L("no_publications", lambda b: b.pop("publications"), "resolve-guild", ["1", g1], "HARNESS: publications required")
    L("no_declarations", lambda b: b.pop("declarations"), "resolve-identity", ["1", g1], "HARNESS: declarations required")
    L("no_accessions", lambda b: b.pop("accessions"), "judge", [jd1], "HARNESS: accessions required")
    L("declarations_object", lambda b: b.__setitem__("declarations", {}), "resolve-guild", ["1", g1], "HARNESS: declarations an array")

    L("pub_no_document", lambda b: b["publications"][0].pop("document"), "resolve-guild", ["1", g1], "HARNESS: a publication without document")
    L("status_not_object", lambda b: b["statuses"].__setitem__(0, 3), "resolve-guild", ["1", g1], "HARNESS: a status is an object")
    L("status_no_blocknumber", lambda b: b["statuses"][0].pop("blockNumber"), "resolve-guild", ["1", g1], "HARNESS: a status without blockNumber")
    L("status_blocknumber_string", lambda b: b["statuses"][0].__setitem__("blockNumber", "3"), "resolve-guild", ["1", g1], "HARNESS: a status blockNumber not an int")
    rootarr = write("logs", "root_array", [])
    case("resolve-identity", [rootarr, "1", g1], "HARNESS: a logs root that is not an object", None, path=rootarr)
    L("status_emitter_short", lambda b: b["statuses"][0].__setitem__("emitter", b["statuses"][0]["emitter"][:-1]), "resolve-guild", ["1", g1], "HARNESS: a status emitter outside hex20")
    L("status_logindex_bool", lambda b: b["statuses"][0].__setitem__("logIndex", True), "resolve-guild", ["1", g1], "HARNESS: a status logIndex not an int")
    L("pub_logindex_string", lambda b: b["publications"][0].__setitem__("logIndex", "3"), "resolve-guild", ["1", g1], "HARNESS: a publication logIndex not an int")
    L("active_int", lambda b: b["statuses"][0].__setitem__("active", 1), "resolve-guild", ["1", g1], "HARNESS: active is a bool")
    L("emitter_short", lambda b: b["publications"][0].__setitem__("emitter", b["publications"][0]["emitter"][:-1]), "resolve-guild", ["1", g1], "HARNESS: emitter outside hex20")
    L("document_not_hex", lambda b: b["publications"][0].__setitem__("document", "0xzz"), "judge", [jd1], "HARNESS: a publication's document outside the transport spelling")
    L("word_absent", lambda b: b["publications"][0].pop("word"), "covenant", ["1", gg], "HARNESS: a publication without its word")
    L("declaration_to_short", lambda b: b["declarations"][0].__setitem__("to", "0x12"), "resolve-identity", ["1", g1], "HARNESS: a declaration's to outside hex20")
    L("declaration_to_absent", lambda b: b["declarations"][0].pop("to"), "guild-card", [g1], "HARNESS: a declaration without to")
    L("declaration_emitter_upper", lambda b: b["declarations"][0].__setitem__("emitter", b["declarations"][0]["emitter"].upper().replace("0X", "0x")), "resolve-judge", ["1", jd1], "HARNESS: a declaration's emitter outside hex20")
    L("declaration_block_string", lambda b: b["declarations"][0].__setitem__("blockNumber", "1"), "resolve-identity", ["1", g1], "HARNESS: a declaration's blockNumber not an int")
    L("accession_root_absent", lambda b: b["accessions"][0].pop("root"), "resolve-identity", ["1", g1], "HARNESS: an accession without root, though the replay does not read it")
    L("accession_holder_int", lambda b: b["accessions"][0].__setitem__("holder", 7), "bench", ["1", "bench/b001_null.json"], "HARNESS: an accession's holder outside hex20")
    L("accession_logindex_negative", lambda b: b["accessions"][0].__setitem__("logIndex", -1), "candidates", ["1", "query/q001_union_dedup.json"], "HARNESS: an accession's logIndex outside the reader's numbers")
    ok = json.loads(json.dumps(logs))
    ok["scannedAt"] = 123
    ok["publications"][0]["txHash"] = H32()
    ok["declarations"][0]["txHash"] = H32()
    ok["accessions"][0]["gas"] = 1
    p = write("logs", "extra_members", ok)
    case("resolve-identity", [p, "1", j1], "HARNESS: members beyond the named ones are ignored", resolved(j2, [j1], [at("j1.late")]), path=p)
    p = write("logs", "empty", {"publications": [], "statuses": [], "declarations": [], "accessions": []})
    case("resolve-guild", [p, "1", g1], "an empty logs set: no document", fault("E_NO_DOCUMENT"), path=p)
    case("resolve-identity", [p, "1", g1], "an empty logs set: every address is its own final", resolved(g1), path=p)
    case("judge", [p, jd1], "an empty logs set: no document", {"verdict": "JUDGE_NONE"}, path=p)
    case("covenant", [p, "1", gg], "an empty logs set: no covenant", {"covenant": None, "ok": True}, path=p)
    case("guild-card", [p, gcard], "an empty logs set: no card", NONE, path=p)
    p = write("logs", "two_array_form", {"publications": [], "statuses": []})
    case("resolve-identity", [p, "1", g1], "HARNESS: a logs file without the book's two arrays is outside the form", None, path=p)
    # a book replayed from the declarations and accessions alone: only declarations
    only = {"publications": [], "statuses": [], "declarations": [{"emitter": j1, "blockNumber": 1, "logIndex": 0, "to": j2}], "accessions": []}
    p = write("logs", "only_declaration", only)
    case("resolve-identity", [p, "1", j1], "s4.2: a declaration alone moves nothing", resolved(j1, [], [{"blockNumber": 1, "logIndex": 0, "to": j2}]), path=p)
    case("resolve-identity", [p, "1", j2], "s4.2: the named key alone is its own final", resolved(j2), path=p)
    only["accessions"].append({"emitter": j2, "blockNumber": 1, "logIndex": 1, "holder": j1, "root": j1})
    p = write("logs", "declaration_and_accession", only)
    case("resolve-identity", [p, "1", j1], "s4.2: a declaration and its accession", resolved(j2, [j1]), path=p)

    # ---- the covenant's quotes, kinds, the top of the range
    case("covenant", [lp, "1", gcx], "s5.9 order: guild mismatch is decided before chain mismatch", fault("E_GUILDAMANAH_GUILD_MISMATCH"))
    pqo = find(gqo, 2)[0]
    case("covenant", [lp, "1", gqo], "s3b.1/s10.1: quotes is kept whole; keys and nested names of any scalar value come out in UTF-8 byte order (U+FF01 before U+FFFF before U+1F600)", {"covenant": cov_resolved(_doc_of(pqo), pqo), "ok": True})
    case("resolve-guild", [lp, "1", gqo], "s5.4 step 8: the covenant's quotes reach the guild answer whole", plain(gqo, [J1ROW], covenant=cov_resolved(_doc_of(pqo), pqo)))
    case("resolve-guild", [lp, "1", gk5], "s2.1 step 3: a kind that is not a string is of every kind; E_KIND", fault("E_KIND"))
    case("judge", [lp, gk5], "s2.1 step 3 as the judge candidate: JUDGE_UNREAD with E_KIND", {"token": "E_KIND", "verdict": "JUDGE_UNREAD"})
    case("covenant", [lp, "1", gk5], "s2.1 step 3 as the covenant candidate: a stop with E_KIND", fault("E_KIND"))
    Q("covenant_stop_from_unreadable_bytes", {"pointers": ["galeed:guild:1:" + gyr]}, "s5.6: a guild whose covenant member is a decoding stop still yields candidates",
      {"candidates": [j2], "perPointer": [guild_row("galeed:guild:1:" + gyr, plain(gyr, [J1ROW], covenant=fault("E_JSON"), card={"token": "E_JSON", "verdict": "CARD_UNREAD"}))]})
    case("resolve-guild", [lp, "1", gbp], "s1: the bare prefix is a kind under the prefix of another version; E_UNKNOWN_KIND", fault("E_UNKNOWN_KIND"))
    case("resolve-guild", [lp, "1", gns], "s1: the prefix without its slash is a kind under no reserved prefix, another kind; the roster stands", plain(gns, [J1ROW]))
    case("resolve-guild", [lp, "1", gts], "s1: the literal with a trailing space is another version; E_UNKNOWN_KIND", fault("E_UNKNOWN_KIND"))
    case("publication", [lp, gbp, "roster"], "s1: the bare prefix is of the roster kind and is the candidate", {"active": True, "candidate": cand(find(gbp, 2)[0], {"kind": "galeed.roster/", "known": False, "ok": True})})
    case("publication", [lp, gns, "roster"], "s1: the prefix without its slash is of another kind; the roster is the candidate", {"active": True, "candidate": cand(find(gns, 1)[0], roster_answer(roster(gns, judges=[j1])))})
    pE = find(jdE, 2)[0]
    case("judge", [lp, jdE], "galeed/1 s9.4: a basis with no chains is a basis; the document reads", {"blockNumber": 2, "document": judge_view(_doc_of(pE)), "verdict": "JUDGE", "word": pE["word"]})
    pG = find(jdG, 2)[0]
    case("judge", [lp, jdG], "galeed/1 s9.4: two windows of one chain a block apart with equal lists do not touch and need not differ", {"blockNumber": 2, "document": judge_view(_doc_of(pG)), "verdict": "JUDGE", "word": pG["word"]})
    case("judge", [lp, jdH], "s5.7: the last judge document fails its hash; JUDGE_UNREAD with E_HASH, no walk back", {"token": "E_HASH", "verdict": "JUDGE_UNREAD"})
    pbig = [p for p in logs["publications"] if p["emitter"] == gbig][0]
    case("publication", [lp, gbig, "roster"], "the reader: blockNumber and logIndex at 2^53 - 1", {"active": True, "candidate": cand(pbig, roster_answer(roster(gbig, judges=[j1])))})
    case("resolve-guild", [lp, "1", gbig], "a roster at block 2^53 - 1 resolves", plain(gbig, [J1ROW]))
    case("resolve-guild", [lp, "1", gnest], "s3.1: a judges element's extra members may nest containers; untested", plain(gnest, [J1ROW]))

    # ---- the record (s5.3), deny and the bench by record (s5.5, s5.6), the replay's boundaries (s4.2)
    dm0, dm1, dm2, gdm, gdo, zr1 = A("dm0"), A("dm1"), A("dm2"), A("gdm"), A("gdo"), A("zr1")
    th1, th2, tk, bn0, bn1, sv, sy = A("th1"), A("th2"), A("tk"), A("bn0"), A("bn1"), A("sv"), A("sy")
    ee, eh, ek, ex2, ey2, nh, nk, nj, lh, lk, lx = [A(x) for x in ("ee", "eh", "ek", "ex2", "ey2", "nh", "nk", "nj", "lh", "lk", "lx")]
    gcmv, gcmv2, gco, gco2, jm1, jm1b, jm2, jm2b, gk7 = [A(x) for x in ("gcmv", "gcmv2", "gco", "gco2", "jm1", "jm1b", "jm2", "jm2b", "gk7")]
    DMROW = {"final": dm2, "listed": dm1, "trail": [dm1]}
    ans_gdm = plain(gdm, [DMROW], bench=bw([dm0, dm1, dm2]), verifiers=[vrow(dm1, final=dm2, trail=[dm1])])
    case("resolve-identity", [lp, "1", dm1], "s5.3: asked in the middle, the trail runs from the key asked; the keys before it are the root's trail", resolved(dm2, [dm1]))
    case("resolve-guild", [lp, "1", gdm], "s5.4 step 7: the bench is every key of the record of the listed verifier, the keys before the listed address included", ans_gdm)
    Q("deny_predecessor_of_listed", {"pointers": ["galeed:guild:1:" + gdm], "deny": [dm0]}, "s5.6: deny reaches a key the record held before the listed address",
      {"candidates": [], "perPointer": [guild_row("galeed:guild:1:" + gdm, ans_gdm)]})
    Q("deny_successor_of_listed", {"pointers": ["galeed:guild:1:" + gdm], "deny": [dm2]}, "s5.6: deny reaches the record's final key",
      {"candidates": [], "perPointer": [guild_row("galeed:guild:1:" + gdm, ans_gdm)]})
    Q("listed_middle_not_denied", {"pointers": ["galeed:guild:1:" + gdm], "deny": [j1]}, "s5.6: a key of no row's record denies nothing",
      {"candidates": [dm2], "perPointer": [guild_row("galeed:guild:1:" + gdm, ans_gdm)]})
    ans_gdo = plain(gdo, [{"final": dm2, "listed": dm0, "trail": [dm0, dm1]}, {"final": dm2, "listed": dm2, "trail": []}])
    Q("old_and_successor_listed", {"pointers": ["galeed:guild:1:" + gdo]}, "s5.6: two rows of one record name one final address, appended once",
      {"candidates": [dm2], "perPointer": [guild_row("galeed:guild:1:" + gdo, ans_gdo)]})
    Q("old_and_successor_denied", {"pointers": ["galeed:guild:1:" + gdo], "deny": [dm1]}, "s5.6: denying a middle key suppresses both rows of the record",
      {"candidates": [], "perPointer": [guild_row("galeed:guild:1:" + gdo, ans_gdo)]})
    B("list_middle_with_root", [dm1, dm0], "s5.5: a middle key named with its root contributes the record once", bw([dm0, dm1, dm2]))
    B("list_final_key", [dm2], "s5.5: the final key contributes every key of its record", bw([dm0, dm1, dm2]))
    B("pointer_middle_verifier", "galeed:guild:1:" + gdm, "s5.5: a guild's bench of a listed middle key holds the keys before it", bw([dm0, dm1, dm2]))
    case("resolve-identity", [lp, "1", ZERO], "s4.2: the zero address is a key like any other as an emitter: a record rooted at it moves", resolved(zr1, [ZERO]))
    case("resolve-identity", [lp, "1", zr1], "s5.3: the record rooted at zero, asked at its final key", resolved(zr1))
    B("list_zero_root", [ZERO], "s5.5: the zero address's record", bw([ZERO, zr1]))
    case("resolve-identity", [lp, "1", th1], "s4.2: two holders name one key; the key takes up the second, and the first's declaration stays an attempt for ever",
         resolved(th1, [], [at("th1")]))
    case("resolve-identity", [lp, "1", th2], "s4.2: the second holder's record moved to the key; its later accession of the first counts for nothing", resolved(tk, [th2]))
    case("resolve-identity", [lp, "1", bn0], "s4.2: a non-binding declaration from the current key leaves its pending key, which then accedes", resolved(bn1, [bn0], [at("bn0.zero")]))
    case("resolve-identity", [lp, "1", sy], "s4.2: a self-declaration does not make its emitter enter, so it can take up a record later", resolved(sv, [sy], [at("sv.self")]))
    case("resolve-identity", [lp, "1", eh], "s4.2: a declaration naming a key that entered binds nothing and leaves the emitter's pending key, which accedes", resolved(ek, [eh], [at("eh.e")]))
    case("resolve-identity", [lp, "1", ey2], "s4.2: a declaration naming an entered key does not make its emitter enter; the emitter takes up a record later", resolved(ex2, [ey2], [at("ex2.e")]))
    case("resolve-identity", [lp, "1", nj], "s4.2: a key that entered by acceding is never named again; the second holder keeps its record", resolved(nj, [], [at("nj")]))
    case("resolve-identity", [lp, "1", nh], "s4.2: the first holder's record, its acceder never joined to another", resolved(nk, [nh]))
    case("resolve-identity", [lp, "1", nk], "s4.2: asked at the acceder", resolved(nk))
    case("resolve-identity", [lp, "1", lh], "s4.2: a key that left declares and its named key accedes: both count for nothing", resolved(lk, [lh], [at("lh.x")]))
    case("resolve-identity", [lp, "1", lx], "s4.2: the key the left key named is its own final", resolved(lx))
    pcm = [p for p in find(gcmv, 1) if _doc_of(p)["kind"] == KC][0]
    case("guild-card", [lp, gcmv], "s5.11: the origin's own card reads at the origin", card_of(pcm))
    case("guild-card", [lp, gcmv2], "s5.11: the final address's card names the origin: CARD_MISMATCH", {"verdict": "CARD_MISMATCH"})
    case("resolve-guild", [lp, "1", gcmv], "s5.4 step 8: the card is read at the final address, whatever the origin's card", plain(gcmv2, [J1ROW], card={"verdict": "CARD_MISMATCH"}, trail=[gcmv]))
    pco = find(gco, 1)[0]
    case("covenant", [lp, "1", gco], "s5.9: covenantOf follows no record: the origin's own covenant, never the successor's", {"covenant": cov_resolved(_doc_of(pco), pco), "ok": True})
    case("resolve-judge", [lp, "1", jm1], "s5.8: a false status at the origin binds no reader; active is read at the final key", {"active": True, "final": jm1b, "judge": judge_answer_at(find(jm1b, 2)[0]), "ok": True, "trail": [jm1]})
    case("resolve-judge", [lp, "1", jm2], "s5.8: a false status at the final key", {"active": False, "final": jm2b, "judge": judge_answer_at(find(jm2b, 2)[0]), "ok": True, "trail": [jm2]})
    case("resolve-guild", [lp, "1", gk7], "s2.1 step 3: a kind that is not a string, before the roster, stops the card and the covenant; the guild resolves",
         plain(gk7, [J1ROW], card={"token": "E_KIND", "verdict": "CARD_UNREAD"}, covenant=fault("E_KIND")))

    # ---- current-at (s5.2)
    acc0, acc1 = BOOK["a0.a"], BOOK["a1.a"]
    def CA(args, note, predicted):
        case("current-at", [lp] + args, note, predicted)
    CA([a0, str(acc0["blockNumber"]), str(acc0["logIndex"])], "s5.2: at the accession's own position the accession is unread", {"current": a0})
    CA([a0, str(acc0["blockNumber"]), str(acc0["logIndex"] + 1)], "s5.2: one position past it the record has moved", {"current": a1})
    CA([a2, str(acc1["blockNumber"]), str(acc1["logIndex"])], "s5.2: a key asked before it acceded is its own record's current key there", {"current": a2})
    CA([a0, str(acc1["blockNumber"]), str(acc1["logIndex"])], "s5.2: the root asked before the second accession: the key that held the record then", {"current": a1})
    CA([a1, "9007199254740991", "9007199254740991"], "s5.2: at the top of the range, every record below the greatest position is read", {"current": a2})
    CA([a0, "0", "0"], "s5.2: before every record, every address is its own current key", {"current": a0})
    CA([A("nobody"), "5", "0"], "s5.2: an address the book never met", {"current": A("nobody")})
    dec0 = BOOK["a0.d"]
    CA([a0, str(dec0["blockNumber"]), str(dec0["logIndex"])], "s5.2: at a declaration's own position the record is where it was; a declaration moves no record", {"current": a0})
    CA([a0, str(acc0["blockNumber"] + 1), "0"], "s5.2: blockNumber is compared first: the next block at log index 0 reads the accession", {"current": a1})
    CA([a0, str(acc0["blockNumber"] - 1), str(acc0["logIndex"] + 50)], "s5.2: blockNumber is compared first: the block before, at a greater log index, does not", {"current": a0})
    CA([a0, str(acc0["blockNumber"]), "0"], "s5.2: the records later in the position's own block are unread", {"current": a0})
    CA([a0, str(acc1["blockNumber"]), "0"], "s5.2: the second accession's block at log index 0: the key that held the record then", {"current": a1})
    bacc = BOOK["bbig2"]
    CA([bbig, str(bacc["blockNumber"]), str(bacc["logIndex"])], "s5.2: near the top of the range, a record at the position asked is unread", {"current": bbig})
    CA([bbig, "9007199254740991", "9007199254740991"], "s5.2: at the greatest position every record below it is read", {"current": bbig2})
    th1, th2, tk = A("th1"), A("th2"), A("tk")
    CA([th1, "9", "0"], "s5.2: a holder whose pending key took up another holder's record keeps its own", {"current": th1})
    CA([th2, "9", "0"], "s5.2: the other holder's record moved to the key", {"current": tk})
    CA([a0, "03", "0"], "HARNESS: a blockNumber with a leading zero", None)
    CA([a0, "3", "00"], "HARNESS: a logIndex with a leading zero", None)
    CA([a0, "3", "-1"], "HARNESS: a logIndex with a sign", None)
    CA([a0, "3", "+1"], "HARNESS: a logIndex with a plus sign", None)
    CA([a0, "3", "1.0"], "HARNESS: a logIndex with a fraction", None)
    CA([a0, "3", "0x1"], "HARNESS: a logIndex in hexadecimal", None)
    CA([a0, "3", ""], "HARNESS: an empty logIndex", None)
    CA([a0, "3", "9007199254740992"], "HARNESS: a logIndex above 2^53 - 1", None)
    CA([a0, "9007199254740992", "0"], "HARNESS: a blockNumber above 2^53 - 1", None)
    CA([a0, "-1", "0"], "HARNESS: a blockNumber with a sign", None)
    CA([a0, "+3", "0"], "HARNESS: a blockNumber with a plus sign", None)
    CA([a0, "3.0", "0"], "HARNESS: a blockNumber with a fraction", None)
    CA([a0, "0x3", "0"], "HARNESS: a blockNumber in hexadecimal", None)
    # a book record at the greatest position, read by current and by no current-at (its own logs set: l001 holds a publication there)
    tx_, ty_ = A("top_x"), A("top_y")
    MAX = 9007199254740991
    toplogs = {"publications": [], "statuses": [],
               "declarations": [{"emitter": tx_, "blockNumber": MAX, "logIndex": MAX - 1, "to": ty_}],
               "accessions": [{"emitter": ty_, "blockNumber": MAX, "logIndex": MAX, "holder": tx_, "root": tx_}]}
    tp = write("logs", "top_position", toplogs)
    case("resolve-identity", [tp, "1", tx_], "s5.2, s5.3: current reads a record at the greatest position", resolved(ty_, [tx_]))
    case("current-at", [tp, tx_, str(MAX), str(MAX)], "s5.2: every current-at leaves a record at the greatest position unread", {"current": tx_})
    case("current-at", [tp, ty_, str(MAX), str(MAX)], "s5.2: the acceding key asked at the greatest position is its own record's current key there", {"current": ty_})
    bp = write("bench", "list_record_moved_at_greatest_position", [tx_])
    case("bench", [tp, "1", bp], "s5.5: the record read after every record, one at the greatest position included", {"bench": bw([tx_, ty_])}, path=bp)
    # a holder that declares one key twice: the accession takes up the second, the one that bound last; the first,
    # a stranger's naming of the same key and the holder's later declaration stay attempts
    rh, rk, rs, rz = A("redecl_h"), A("redecl_k"), A("redecl_s"), A("redecl_z")
    rdlogs = {"publications": [], "statuses": [],
              "declarations": [{"emitter": rh, "blockNumber": 1, "logIndex": 0, "to": rk},
                               {"emitter": rh, "blockNumber": 2, "logIndex": 0, "to": rk},
                               {"emitter": rs, "blockNumber": 2, "logIndex": 1, "to": rk},
                               {"emitter": rh, "blockNumber": 4, "logIndex": 0, "to": rz}],
              "accessions": [{"emitter": rk, "blockNumber": 3, "logIndex": 0, "holder": rh, "root": rh}]}
    rd = write("logs", "redeclared_twice", rdlogs)
    case("resolve-identity", [rd, "1", rh], "s4.2, s5.3: the accession takes up the holder's last binding declaration; the one it replaced and the late one stay attempts",
         resolved(rk, [rh], [{"blockNumber": 1, "logIndex": 0, "to": rk}, {"blockNumber": 4, "logIndex": 0, "to": rz}]))
    case("resolve-identity", [rd, "1", rs], "s4.2, s5.3: a stranger that named the same key keeps its own record and its declaration as an attempt",
         resolved(rs, [], [{"blockNumber": 2, "logIndex": 1, "to": rk}]))
    CA([a0, "", "0"], "HARNESS: an empty blockNumber", None)
    CA(["0x" + a0[2:].upper(), "3", "0"], "HARNESS: an address in uppercase", None)
    case("current-at", ["logs/no_such_file.json", a0, "3", "0"], "HARNESS: a logs file the reader cannot read", None, absent=["logs/no_such_file.json"])
    CA([a0, "3"], "HARNESS: a missing argument", None)
    CA([a0, "3", "0", "x"], "HARNESS: an argument beyond the list", None)
    CA([a0[:-1], "3", "0"], "HARNESS: an address outside hex20", None)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=1)
    a = ap.parse_args()
    RNG.seed(a.seed)
    for cat in _n:
        d = os.path.join(HERE, cat)
        shutil.rmtree(d, ignore_errors=True)
        os.makedirs(d)
    gen_decode()
    gen_resolution()
    with open(os.path.join(HERE, "manifest.json"), "w", encoding="utf-8") as f:
        json.dump({"seed": a.seed, "cases": MANIFEST}, f, ensure_ascii=False, indent=1)
    print("cases:", len(MANIFEST))


if __name__ == "__main__":
    main()
