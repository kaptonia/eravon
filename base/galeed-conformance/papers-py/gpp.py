#!/usr/bin/env python3
"""gpp: the galeed.papers/1 criterion in pure Python 3, over the frozen
galeed/1 core in ../impl-py.  Command-line contract: HARNESS-PAPERS.md."""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "..", "impl-py"))

from glcanon import Fault, Obj, canon, parse15                    # noqa: E402
from glcrypto import N, address_of, eip191_digest, message, pubkey, sha256, sign_digest  # noqa: E402
from glentry import is_hex                                         # noqa: E402
import glaudit                                                     # noqa: E402
import gppdoc                                                      # noqa: E402
from gppdoc import DocFault, PayloadFault                          # noqa: E402
import gppdoor                                                     # noqa: E402


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
        die("gpp: cannot read %s: %s" % (path, exc))


def cmd_doc_check(argv, fn, name):
    if len(argv) != 1:
        die("usage: gpp %s <path>" % name)
    b = read_file(argv[0])
    try:
        fn(b)
    except DocFault as f:
        pairs = [("ok", False), ("token", f.token)]
        if f.index is not None:
            pairs.append(("index", f.index))
        emit(Obj(pairs))
        return
    emit(Obj([("doc_id", gppdoc.doc_id(b)), ("ok", True)]))


def _transport_hex(v):
    if not isinstance(v, str) or len(v) < 2 or v[0] != "0" or v[1] != "x":
        return None
    body = v[2:]
    if len(body) % 2:
        return None
    for c in body:
        if c not in "0123456789abcdefABCDEF":
            return None
    return bytes.fromhex(body)


def _publication_of(v):
    if not isinstance(v, Obj):
        return None
    d = v.dict()
    for k in ("emitter", "blockNumber", "logIndex", "document", "word"):
        if k not in d:
            return None
    if not is_hex(d["emitter"], 20) or type(d["blockNumber"]) is not int or type(d["logIndex"]) is not int:
        return None
    doc = _transport_hex(d["document"])
    if doc is None or not is_hex(d["word"], 32):
        return None
    return {"emitter": d["emitter"], "blockNumber": d["blockNumber"], "logIndex": d["logIndex"],
            "document": doc, "word": d["word"]}


def _status_of(v):
    if not isinstance(v, Obj):
        return None
    d = v.dict()
    for k in ("emitter", "blockNumber", "logIndex", "active"):
        if k not in d:
            return None
    if not is_hex(d["emitter"], 20) or type(d["blockNumber"]) is not int or type(d["logIndex"]) is not int \
            or type(d["active"]) is not bool:
        return None
    return (d["blockNumber"], d["logIndex"])


def _book_record_of(v, members):
    """A `declarations` or `accessions` element: emitter, blockNumber,
    logIndex, and the hex20 members named; its (blockNumber, logIndex)."""
    if not isinstance(v, Obj):
        return None
    d = v.dict()
    for k in ("emitter", "blockNumber", "logIndex") + members:
        if k not in d:
            return None
    if not is_hex(d["emitter"], 20) or type(d["blockNumber"]) is not int or type(d["logIndex"]) is not int:
        return None
    if not all(is_hex(d[k], 20) for k in members):
        return None
    return (d["blockNumber"], d["logIndex"])


def _load_logs(path):
    """HARNESS-ROSTER's <logs.json> form, read as HARNESS-PAPERS states it: the publications."""
    b = read_file(path)
    try:
        v = glaudit.load_input(b)
    except glaudit.NoLabel:
        die("gpp: %s is not readable by the reader" % path)
    if not isinstance(v, Obj):
        die("gpp: logs root is not an object")
    d = v.dict()
    if "publications" not in d or not isinstance(d["publications"], list):
        die("gpp: logs.publications is not an array")
    if "statuses" not in d or not isinstance(d["statuses"], list):
        die("gpp: logs.statuses is not an array")
    pubs, seen = [], set()
    for e in d["publications"]:
        p = _publication_of(e)
        if p is None:
            die("gpp: a publication is outside its form")
        k = (p["blockNumber"], p["logIndex"])
        if k in seen:
            die("gpp: two records share (blockNumber, logIndex)")
        seen.add(k)
        pubs.append(p)
    for e in d["statuses"]:
        k = _status_of(e)
        if k is None:
            die("gpp: a status is outside its form")
        if k in seen:
            die("gpp: two records share (blockNumber, logIndex)")
        seen.add(k)
    for name, members in (("declarations", ("to",)), ("accessions", ("holder", "root"))):
        if name not in d or not isinstance(d[name], list):
            die("gpp: logs.%s is not an array" % name)
        for e in d[name]:
            k = _book_record_of(e, members)
            if k is None:
                die("gpp: a book record is outside its form")
            if k in seen:
                die("gpp: two records share (blockNumber, logIndex)")
            seen.add(k)
    return pubs


def cmd_service_of(argv):
    if len(argv) != 2:
        die("usage: gpp service-of <logs.json> <address>")
    pubs = _load_logs(argv[0])
    if not is_hex(argv[1], 20):
        die("gpp: address is not hex20")
    r, x = gppdoc.service_of(argv[1], pubs)
    if r == "SERVICE":
        emit(Obj([("serves", list(x)), ("verdict", r)]))
    elif r == "SERVICE_UNREAD":
        pairs = []
        if x.index is not None:
            pairs.append(("index", x.index))
        pairs += [("token", x.token), ("verdict", r)]
        emit(Obj(pairs))
    else:
        emit(Obj([("verdict", r)]))


def cmd_serving(argv):
    if len(argv) != 3:
        die("usage: gpp serving <logs.json> <verifier> <seat>")
    pubs = _load_logs(argv[0])
    if not is_hex(argv[1], 20):
        die("gpp: verifier is not hex20")
    if not is_hex(argv[2], 20):
        die("gpp: seat is not hex20")
    emit(Obj([("serving", gppdoc.serving(argv[1], argv[2], pubs))]))


def cmd_sign(argv):
    if len(argv) != 3:
        die("usage: gpp sign <privkey-hex> <path> <domain>")
    key = argv[0]
    if key.startswith("0x") or key.startswith("0X"):
        key = key[2:]
    if len(key) != 64 or any(c not in "0123456789abcdefABCDEF" for c in key):
        die("gpp: bad private key")
    priv = int(key, 16)
    if not (1 <= priv <= N - 1):
        die("gpp: private key out of range")
    domain = argv[2]
    if domain not in gppdoc.DOMAINS:
        die("gpp: domain is one of the four literals of s3.2")
    b = read_file(argv[1])
    try:
        v = parse15(b)
    except Fault as f:
        die("gpp: preimage file is not a readable value (%s)" % f.token)
    if not isinstance(v, Obj):
        die("gpp: preimage is not an object")
    pre = sha256(canon(v))
    digest = eip191_digest(message(domain, pre))
    r, s, vv = sign_digest(priv, digest)
    sig = r.to_bytes(32, "big") + s.to_bytes(32, "big") + bytes([vv])
    emit(Obj([("digest", "0x" + digest.hex()), ("presig", "0x" + pre.hex()),
              ("sig", "0x" + sig.hex()), ("signer", "0x" + address_of(pubkey(priv)).hex())]))


def _emit_pair(verdict, token, index=None):
    pairs = [("verdict", verdict)]
    if token is not None:
        pairs.append(("token", token))
    if index is not None:
        pairs.append(("index", index))
    emit(Obj(pairs))


def _parse_current(s):
    # s1: a hex20, or the literal `unresolved`
    if s != "unresolved" and not is_hex(s, 20):
        die("gpp: <current> is a hex20 or the word unresolved")
    return s


def cmd_pair(argv):
    if len(argv) != 3:
        die("usage: gpp pair <note-path> <report-path> <current>")
    n = read_file(argv[0])
    r = read_file(argv[1])
    c = _parse_current(argv[2])
    _emit_pair(*gppdoc.pair(n, r, c))


def cmd_pair_refusal(argv):
    if len(argv) != 4:
        die("usage: gpp pair-refusal <note-path> <pass-path> <refusal-path> <current>")
    n = read_file(argv[0])
    p = read_file(argv[1])
    f = read_file(argv[2])
    c = _parse_current(argv[3])
    _emit_pair(*gppdoc.pair_refusal(n, p, f, c))


def _emit_payload_fault(f):
    pairs = [("ok", False), ("token", f.token)]
    if f.inner is not None:
        pairs.append(("inner", f.inner))
    if f.index is not None:
        pairs.append(("index", f.index))
    if f.count is not None:
        pairs.append(("count", f.count))
    emit(Obj(pairs))


def cmd_qr_encode(argv):
    if len(argv) != 2:
        die("usage: gpp qr-encode <pass-path> <note-path>")
    p = read_file(argv[0])
    n = read_file(argv[1])
    try:
        payload = gppdoc.payload_encode(p, n)
    except PayloadFault as f:
        _emit_payload_fault(f)
        return
    emit(Obj([("payload", payload.decode("ascii"))]))


def cmd_qr_decode(argv):
    if len(argv) != 1:
        die("usage: gpp qr-decode <path>")
    try:
        p, n = gppdoc.payload_decode(read_file(argv[0]))
    except PayloadFault as f:
        _emit_payload_fault(f)
        return
    emit(Obj([("note", "0x" + n.hex()), ("ok", True), ("pass", "0x" + p.hex())]))


def _parse_now(s):
    # galeed/1 s3.2 spelling: decimal digits, no sign, no leading zero, <= 2^53-1
    if not s.isascii() or not s.isdigit():
        die("gpp: --now is an int of s3.2")
    if len(s) > 1 and s[0] == "0":
        die("gpp: --now has a leading zero")
    v = int(s)
    if v > (1 << 53) - 1:
        die("gpp: --now above 2^53-1")
    return v


def _load_bench(path):
    raw = read_file(path)
    try:
        v = glaudit.load_input(raw)
    except glaudit.NoLabel:
        die("gpp: bench file is unreadable")
    if not isinstance(v, list):
        die("gpp: bench root is not an array")
    for x in v:
        if not is_hex(x, 20):
            die("gpp: bench element outside hex20")
    for i in range(1, len(v)):
        if v[i].encode("utf-8") <= v[i - 1].encode("utf-8"):
            die("gpp: bench is bytewise ascending without repeats")
    return v


def cmd_door(argv):
    usage = "usage: gpp door <note-path> [<pass-path>] [--now <int>] [--audit <path>] [--bench <path> | --bench-unresolved] [--current <hex20|unresolved>]"
    if len(argv) < 1:
        die(usage)
    note_b = read_file(argv[0])
    rest = argv[1:]
    pass_b = None
    if rest and not rest[0].startswith("--"):
        pass_b = read_file(rest[0])
        rest = rest[1:]
    now = None
    audit_raw = None
    bench = None
    current = "unresolved"
    seen = set()
    i = 0
    while i < len(rest):
        opt = rest[i]
        if opt in ("--bench", "--bench-unresolved") and ("--bench" in seen or "--bench-unresolved" in seen):
            die("gpp: bench given twice")
        if opt == "--bench-unresolved":
            seen.add(opt)
            bench = "unresolved"
            i += 1
            continue
        if opt not in ("--now", "--audit", "--bench", "--current") or opt in seen or i + 1 >= len(rest):
            die("gpp: bad arguments")
        seen.add(opt)
        val = rest[i + 1]
        if opt == "--now":
            now = _parse_now(val)
        elif opt == "--audit":
            audit_raw = read_file(val)
        elif opt == "--current":
            current = _parse_current(val)
        else:
            bench = _load_bench(val)
        i += 2
    emit(gppdoor.door(note_b, pass_b, now, audit_raw, bench, current))


def main(argv):
    if len(argv) < 2:
        die("usage: gpp {note-check|pass-check|report-check|refusal-check|sign|pair|pair-refusal|qr-encode|qr-decode|door} ...")
    cmd, rest = argv[1], argv[2:]
    if cmd == "note-check":
        cmd_doc_check(rest, gppdoc.check_note, cmd)
    elif cmd == "pass-check":
        cmd_doc_check(rest, gppdoc.check_pass, cmd)
    elif cmd == "report-check":
        cmd_doc_check(rest, gppdoc.check_report, cmd)
    elif cmd == "refusal-check":
        cmd_doc_check(rest, gppdoc.check_refusal, cmd)
    elif cmd == "service-check":
        cmd_doc_check(rest, gppdoc.check_service, cmd)
    elif cmd == "service-of":
        cmd_service_of(rest)
    elif cmd == "serving":
        cmd_serving(rest)
    elif cmd == "sign":
        cmd_sign(rest)
    elif cmd == "pair":
        cmd_pair(rest)
    elif cmd == "pair-refusal":
        cmd_pair_refusal(rest)
    elif cmd == "qr-encode":
        cmd_qr_encode(rest)
    elif cmd == "qr-decode":
        cmd_qr_decode(rest)
    elif cmd == "door":
        cmd_door(rest)
    else:
        die("gpp: unknown command %r" % cmd)
    return 0


if __name__ == "__main__":
    sys.setrecursionlimit(10000)
    sys.exit(main(sys.argv))
