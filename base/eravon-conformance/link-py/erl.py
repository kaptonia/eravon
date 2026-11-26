#!/usr/bin/env python3
"""erl: the eravon.link/1 criterion in pure Python 3, over the eravon.terms/1
core in ../terms-py.  Command-line contract: HARNESS-LINK.md."""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "..", "terms-py"))

from ertcanon import NoLabel, Obj, canon, is_hex, is_int, load_input  # noqa: E402
import ertlaw as T                                 # noqa: E402
from ertlaw import DocFault, hx                    # noqa: E402
import erllaw as L                                 # noqa: E402
from erllaw import Verdict                         # noqa: E402


def emit(v):
    sys.stdout.buffer.write(canon(v))
    sys.stdout.buffer.flush()


def die(msg):
    sys.stderr.write(msg + "\n")
    sys.exit(2)


def read_file(path):
    try:
        with open(path, "rb") as fh:
            return fh.read()
    except OSError as exc:
        die("erl: cannot read %s: %s" % (path, exc))


def parse_int_arg(s):
    if not T.is_int_spelling(s):
        die("erl: an int of galeed/1 s3.2")
    return int(s)


# --------------------------------------------------------------------------

def cmd_check(argv):
    if len(argv) != 1:
        die("usage: erl check <listing.json>")
    try:
        s = L.check_listing(read_file(argv[0]))
    except DocFault as f:
        emit(f.obj())
        return
    emit(L.listing_answer(s))


def cmd_bind(argv):
    if len(argv) != 2:
        die("usage: erl bind <listing.json> <terms.md>")
    sb = read_file(argv[0])
    tb = read_file(argv[1])
    try:
        L.bind(sb, tb)
    except Verdict as v:
        emit(v.obj())
        return
    emit(Obj([("verdict", "BOUND")]))


def cmd_link_check(argv):
    if len(argv) != 1:
        die("usage: erl link-check <link.json>")
    try:
        ln = L.check_link(read_file(argv[0]))
    except DocFault as f:
        emit(f.obj())
        return
    emit(L.link_answer(ln))


def cmd_encode(argv):
    if len(argv) != 1:
        die("usage: erl encode <link.json>")
    b = read_file(argv[0])
    try:
        L.check_link(b)
    except DocFault as f:
        emit(f.obj())
        return
    emit(Obj([("link", (L.OPEN_PREFIX + L.b64e(b)).decode("ascii")), ("ok", True)]))


def cmd_decode(argv):
    if len(argv) != 1:
        die("usage: erl decode <link.txt>")
    try:
        raw = L.decode_transport(read_file(argv[0]), L.OPEN_PREFIX, "E_LINK_PREFIX", "E_LINK_B64",
                                 "E_LINK_DOC", L.check_link)
    except DocFault as f:
        emit(f.obj())
        return
    emit(Obj([("ok", True), ("link", hx(raw))]))


def cmd_resolve_link(argv):
    if len(argv) != 2:
        die("usage: erl resolve-link <link.json> <document.json>")
    lb = read_file(argv[0])
    db = read_file(argv[1])
    try:
        kind, ans = L.resolve_link(lb, db)
    except Verdict as v:
        emit(v.obj())
        return
    emit(Obj([("document", ans), ("verdict", kind)]))


def cmd_receipt_check(argv):
    if len(argv) != 1:
        die("usage: erl receipt-check <receipt.json>")
    try:
        r = L.check_receipt(read_file(argv[0]))
    except DocFault as f:
        emit(f.obj())
        return
    emit(Obj([("candidate", r["candidate"]), ("form", L.receipt_form(r)), ("ok", True), ("openBy", r["openBy"]), ("prev", r["prev"])]))


def cmd_receipt_encode(argv):
    if len(argv) != 1:
        die("usage: erl receipt-encode <receipt.json>")
    b = read_file(argv[0])
    try:
        L.check_receipt(b)
    except DocFault as f:
        emit(f.obj())
        return
    emit(Obj([("ok", True), ("text", (L.RECEIPT_PREFIX + L.b64e(b)).decode("ascii"))]))


def cmd_receipt_decode(argv):
    if len(argv) != 1:
        die("usage: erl receipt-decode <text.txt>")
    try:
        raw = L.decode_transport(read_file(argv[0]), L.RECEIPT_PREFIX, "E_RECEIPT_PREFIX", "E_RECEIPT_B64",
                                 "E_RECEIPT_DOC", L.check_receipt)
    except DocFault as f:
        emit(f.obj())
        return
    emit(Obj([("ok", True), ("receipt", hx(raw))]))


def cmd_bind_receipt(argv):
    if len(argv) != 2:
        die("usage: erl bind-receipt <listing.json> <receipt.json>")
    sb = read_file(argv[0])
    rb = read_file(argv[1])
    try:
        b = L.bind_receipt(sb, rb)
    except Verdict as v:
        emit(v.obj())
        return
    emit(L.bound_answer(b))


def cmd_pack_check(argv):
    if len(argv) != 1:
        die("usage: erl pack-check <pack.json>")
    try:
        pk = L.check_pack(read_file(argv[0]))
    except DocFault as f:
        emit(f.obj())
        return
    emit(Obj([("attachments", len(pk.attachments)), ("buyerKey", pk.receipt["buyerKey"]), ("caseId", hx(pk.bound.salt)),
              ("form", pk.bound.form), ("instance", hx(pk.bound.instance)), ("ok", True)]))


def cmd_settle_check(argv):
    if len(argv) != 1:
        die("usage: erl settle-check <settle.json>")
    try:
        st = L.check_settle(read_file(argv[0]))
    except DocFault as f:
        emit(f.obj())
        return
    emit(L.settle_answer(st))


def cmd_bind_settle(argv):
    if len(argv) != 2:
        die("usage: erl bind-settle <pack.json> <settle.json>")
    kb = read_file(argv[0])
    sb = read_file(argv[1])
    try:
        dg, form = L.bind_settle(kb, sb)
    except Verdict as v:
        emit(v.obj())
        return
    emit(Obj([("digest", hx(dg)), ("verdict", "SETTLE_BY_CALL" if form == "call" else "SETTLE_BOUND")]))


# --------------------------------------------------------------------------
# s6 identity: <records.json> (HARNESS-LINK.md) and the three commands

_FIELDS = {
    "posted": (("poster", "a"), ("key", "d"), ("hash", "d"), ("document", "b")),
    "consented": (("who", "a"), ("digest", "d")),
    "revoked": (("seller", "a"), ("termsHash", "d")),
    "pulled": (("seller", "a"), ("fund", "a"), ("token", "a"), ("instance", "a"), ("amount", "n")),
    "opened": (("emitter", "a"), ("caseId", "d"), ("seller", "a"), ("buyer", "a"), ("judge", "a"), ("amount", "n"),
               ("termsHash", "d")),
    "events": (("emitter", "a"), ("name", "s")),
    "declarations": (("from", "a"), ("to", "a")),
    "accessions": (("holder", "a"), ("to", "a"), ("root", "a")),
}
_HEXL = frozenset("0123456789abcdef")


def _form_ok(kind, v):
    if kind == "a":
        return is_hex(v, 20)
    if kind == "d":
        return is_hex(v, 32)
    if kind == "n":
        return T.is_dec256(v)
    if kind == "b":
        return isinstance(v, str) and len(v) >= 2 and v[:2] == "0x" and len(v) % 2 == 0 and all(c in _HEXL for c in v[2:])
    return isinstance(v, str)


def read_records(path):
    """<records.json>: misuse (exit 2) on any other shape."""
    try:
        v = load_input(read_file(path))
    except NoLabel:
        die("erl: %s is not a readable value" % path)
    if not isinstance(v, Obj):
        die("erl: records: an object")
    d = v.dict()
    if set(d.keys()) != {"chainId"} | set(_FIELDS):
        die("erl: records: exactly chainId and the eight lists")
    if not is_int(d["chainId"]):
        die("erl: records: chainId an int")
    kinds, seen = {}, set()
    for kind, fields in _FIELDS.items():
        if not isinstance(d[kind], list):
            die("erl: records: %s a list" % kind)
        out = []
        for el in d[kind]:
            if not isinstance(el, Obj):
                die("erl: records: an element is an object")
            e = el.dict()
            if set(e.keys()) != {"blockNumber", "logIndex"} | {f for f, _ in fields}:
                die("erl: records: the members of a %s element" % kind)
            if not (is_int(e["blockNumber"]) and is_int(e["logIndex"])):
                die("erl: records: blockNumber and logIndex ints")
            for f, k in fields:
                if not _form_ok(k, e[f]):
                    die("erl: records: the form of %s.%s" % (kind, f))
            p = (e["blockNumber"], e["logIndex"])
            if p in seen:
                die("erl: records: two elements share a position")
            seen.add(p)
            out.append(e)
        kinds[kind] = out
    return L.Records(d["chainId"], kinds)


def _rec(e):
    return Obj(sorted(e.items()))


def _posobj(p, source=None):
    pairs = [("blockNumber", p[0]), ("logIndex", p[1])]
    if source is not None:
        pairs.append(("source", source))
    return Obj(pairs)


def _case(rs, o):
    return Obj([("buyer", o["buyer"]), ("caseId", o["caseId"]), ("instance", o["emitter"]),
                ("position", _posobj(L.pos(o))), ("records", [_rec(r) for r in L.case_records(rs, o)])])


def _doc(rs, d):
    postings = [Obj(sorted(list(r.items()) + [("standing", "stands" if L.stands(rs, r["poster"], L.pos(r)) else "attempt")]))
                for r in d.postings]
    return Obj([("cases", [_case(rs, o) for o in d.opened]), ("position", _posobj(d.position, d.source)),
                ("postings", postings), ("revoked", [_rec(r) for r in d.revoked]), ("sellerKey", d.seller),
                ("source", d.source), ("standing", d.standing), ("termsHash", d.terms)])


def cmd_place(argv):
    if len(argv) != 3:
        die("usage: erl place <records.json> <sellerKey> <termsHash>")
    rs = read_records(argv[0])
    if not is_hex(argv[1], 20) or not is_hex(argv[2], 32):
        die("erl: place: a hex20 and a hex32")
    pl = L.place(rs, argv[1], argv[2])
    emit(Obj([("ok", True), ("position", None if pl is None else _posobj(pl[0], pl[1]))]))


def cmd_pay_check(argv):
    if len(argv) != 2:
        die("usage: erl pay-check <listing.json> <records.json>")
    sb = read_file(argv[0])
    rs = read_records(argv[1])
    try:
        verdict = L.pay_check(sb, rs)
    except Verdict as v:
        emit(v.obj())
        return
    emit(Obj([("verdict", verdict)]))


def cmd_history(argv):
    if len(argv) != 2:
        die("usage: erl history <records.json> <address>")
    rs = read_records(argv[0])
    if not is_hex(argv[1], 20):
        die("erl: history: a hex20")
    keys, docs, consents, cases, attempts = L.history(rs, argv[1])
    emit(Obj([("attempts", [_doc(rs, d) for d in attempts]), ("cases", [_case(rs, o) for o in cases]),
              ("consents", [_rec(r) for r in consents]), ("documents", [_doc(rs, d) for d in docs]),
              ("keys", keys), ("ok", True)]))


COMMANDS = {
    "check": cmd_check, "bind": cmd_bind,
    "link-check": cmd_link_check, "encode": cmd_encode, "decode": cmd_decode, "resolve-link": cmd_resolve_link,
    "receipt-check": cmd_receipt_check, "receipt-encode": cmd_receipt_encode,
    "receipt-decode": cmd_receipt_decode, "bind-receipt": cmd_bind_receipt,
    "pack-check": cmd_pack_check,
    "settle-check": cmd_settle_check, "bind-settle": cmd_bind_settle,
    "place": cmd_place, "pay-check": cmd_pay_check, "history": cmd_history,
}


def main(argv):
    if len(argv) < 2 or argv[1] not in COMMANDS:
        die("usage: erl {%s} ..." % "|".join(COMMANDS))
    COMMANDS[argv[1]](argv[2:])
    return 0


if __name__ == "__main__":
    sys.setrecursionlimit(10000)
    sys.exit(main(sys.argv))
