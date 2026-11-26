#!/usr/bin/env python3
"""gro: the galeed.roster/1 criterion in pure Python 3, over the frozen
galeed/1 core in ../impl-py.  Command-line contract: HARNESS-ROSTER.md."""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "..", "impl-py"))

from glcanon import Obj, canon                                    # noqa: E402
from glentry import is_hex, is_int, is_bool                       # noqa: E402
import glaudit                                                    # noqa: E402
import grolaw                                                     # noqa: E402

BOUND = 16 * 1024 * 1024
KINDS = ("roster", "guildcard", "judge", "guildamanah", "verifier")


def emit(v):
    sys.stdout.buffer.write(canon(v))
    sys.stdout.buffer.flush()


def die(msg):
    sys.stderr.write(msg + "\n")
    sys.exit(2)


def read_file(path):
    try:
        with open(path, "rb") as fh:
            b = fh.read(BOUND + 1)
    except OSError as exc:
        die("gro: cannot read %s: %s" % (path, exc))
    if len(b) > BOUND:
        die("gro: %s is above the sixteen-mebibyte bound" % path)
    return b


def read_json(path):
    try:
        return glaudit.load_input(read_file(path))
    except glaudit.NoLabel:
        die("gro: %s is not readable by the reader" % path)


def parse_int_arg(s, what):
    if not s or any(c not in "0123456789" for c in s) or (len(s) > 1 and s[0] == "0") \
            or len(s) > 16 or int(s) > glaudit.MAX_INT:
        die("gro: %s is not an int spelling" % what)
    return int(s)


def parse_addr_arg(s):
    if not is_hex(s, 20):
        die("gro: address is not hex20")
    return s


def transport_hex(v):
    if not isinstance(v, str) or len(v) < 2 or v[0] != "0" or v[1] != "x":
        return None
    body = v[2:]
    if len(body) % 2:
        return None
    for c in body:
        if c not in "0123456789abcdefABCDEF":
            return None
    return bytes.fromhex(body)


def publication_of(v):
    if not isinstance(v, Obj):
        return None
    d = v.dict()
    if "emitter" not in d or not is_hex(d["emitter"], 20):
        return None
    if "blockNumber" not in d or not is_int(d["blockNumber"]):
        return None
    if "logIndex" not in d or not is_int(d["logIndex"]):
        return None
    if "document" not in d:
        return None
    doc = transport_hex(d["document"])
    if doc is None:
        return None
    if "word" not in d or not is_hex(d["word"], 32):
        return None
    return {"emitter": d["emitter"], "blockNumber": d["blockNumber"],
            "logIndex": d["logIndex"], "document": doc, "word": d["word"]}


def status_of(v):
    if not isinstance(v, Obj):
        return None
    d = v.dict()
    if "emitter" not in d or not is_hex(d["emitter"], 20):
        return None
    if "blockNumber" not in d or not is_int(d["blockNumber"]):
        return None
    if "logIndex" not in d or not is_int(d["logIndex"]):
        return None
    if "active" not in d or not is_bool(d["active"]):
        return None
    return {"emitter": d["emitter"], "blockNumber": d["blockNumber"],
            "logIndex": d["logIndex"], "active": d["active"]}


def declaration_of(v):
    if not isinstance(v, Obj):
        return None
    d = v.dict()
    for k in ("emitter", "blockNumber", "logIndex", "to"):
        if k not in d:
            return None
    if not is_hex(d["emitter"], 20) or not is_int(d["blockNumber"]) \
            or not is_int(d["logIndex"]) or not is_hex(d["to"], 20):
        return None
    return {"emitter": d["emitter"], "blockNumber": d["blockNumber"],
            "logIndex": d["logIndex"], "to": d["to"]}


def accession_of(v):
    if not isinstance(v, Obj):
        return None
    d = v.dict()
    for k in ("emitter", "blockNumber", "logIndex", "holder", "root"):
        if k not in d:
            return None
    if not is_hex(d["emitter"], 20) or not is_int(d["blockNumber"]) \
            or not is_int(d["logIndex"]) or not is_hex(d["holder"], 20) \
            or not is_hex(d["root"], 20):
        return None
    return {"emitter": d["emitter"], "blockNumber": d["blockNumber"],
            "logIndex": d["logIndex"], "holder": d["holder"], "root": d["root"]}


def load_logs(path):
    v = read_json(path)
    if not isinstance(v, Obj):
        die("gro: logs root is not an object")
    d = v.dict()
    if "publications" not in d or not isinstance(d["publications"], list):
        die("gro: logs.publications is not an array")
    if "statuses" not in d or not isinstance(d["statuses"], list):
        die("gro: logs.statuses is not an array")
    if "declarations" not in d or not isinstance(d["declarations"], list):
        die("gro: logs.declarations is not an array")
    if "accessions" not in d or not isinstance(d["accessions"], list):
        die("gro: logs.accessions is not an array")
    pubs, stats, decls, accs, seen = [], [], [], [], set()
    for e in d["publications"]:
        p = publication_of(e)
        if p is None:
            die("gro: a publication is outside its form")
        key = (p["blockNumber"], p["logIndex"])
        if key in seen:
            die("gro: two records share (blockNumber, logIndex)")
        seen.add(key)
        pubs.append(p)
    for e in d["statuses"]:
        s = status_of(e)
        if s is None:
            die("gro: a status is outside its form")
        key = (s["blockNumber"], s["logIndex"])
        if key in seen:
            die("gro: two records share (blockNumber, logIndex)")
        seen.add(key)
        stats.append(s)
    for e in d["declarations"]:
        x = declaration_of(e)
        if x is None:
            die("gro: a declaration is outside its form")
        key = (x["blockNumber"], x["logIndex"])
        if key in seen:
            die("gro: two records share (blockNumber, logIndex)")
        seen.add(key)
        decls.append(x)
    for e in d["accessions"]:
        x = accession_of(e)
        if x is None:
            die("gro: an accession is outside its form")
        key = (x["blockNumber"], x["logIndex"])
        if key in seen:
            die("gro: two records share (blockNumber, logIndex)")
        seen.add(key)
        accs.append(x)
    return grolaw.Logs(pubs, stats, decls, accs)


def cmd_decode(argv):
    if len(argv) != 1:
        die("usage: gro decode <publication.json>")
    p = publication_of(read_json(argv[0]))
    if p is None:
        die("gro: publication is outside its form")
    emit(grolaw.decode_answer(p))


def cmd_publication(argv):
    if len(argv) != 3:
        die("usage: gro publication <logs.json> <address> <kind>")
    logs = load_logs(argv[0])
    a = parse_addr_arg(argv[1])
    if argv[2] not in KINDS:
        die("gro: kind is one of roster, guildcard, judge, guildamanah, verifier")
    p = logs.candidate_of(a, argv[2])
    cand = None
    if p is not None:
        cand = Obj([("blockNumber", p["blockNumber"]), ("decoded", grolaw.decode_answer(p)),
                    ("logIndex", p["logIndex"])])
    emit(Obj([("active", logs.active(a)), ("candidate", cand)]))


def cmd_identity(argv):
    if len(argv) != 3:
        die("usage: gro resolve-identity <logs.json> <chainId> <address>")
    logs = load_logs(argv[0])
    parse_int_arg(argv[1], "chainId")
    a = parse_addr_arg(argv[2])
    emit(grolaw.identity_answer(logs, a))


def cmd_current_at(argv):
    if len(argv) != 4:
        die("usage: gro current-at <logs.json> <address> <blockNumber> <logIndex>")
    logs = load_logs(argv[0])
    a = parse_addr_arg(argv[1])
    position = (parse_int_arg(argv[2], "blockNumber"), parse_int_arg(argv[3], "logIndex"))
    emit(Obj([("current", grolaw.current_at(logs, a, position))]))


def cmd_guild(argv):
    if len(argv) != 3:
        die("usage: gro resolve-guild <logs.json> <chainId> <address>")
    logs = load_logs(argv[0])
    chain = parse_int_arg(argv[1], "chainId")
    a = parse_addr_arg(argv[2])
    emit(grolaw.resolve_guild(logs, chain, a))


def cmd_judge(argv):
    if len(argv) != 2:
        die("usage: gro judge <logs.json> <address>")
    logs = load_logs(argv[0])
    a = parse_addr_arg(argv[1])
    emit(grolaw.judge_of(logs, a))


def cmd_guild_card(argv):
    if len(argv) != 2:
        die("usage: gro guild-card <logs.json> <address>")
    logs = load_logs(argv[0])
    a = parse_addr_arg(argv[1])
    emit(grolaw.guild_card_of(logs, a))


def cmd_resolve_judge(argv):
    if len(argv) != 3:
        die("usage: gro resolve-judge <logs.json> <chainId> <address>")
    logs = load_logs(argv[0])
    chain = parse_int_arg(argv[1], "chainId")
    a = parse_addr_arg(argv[2])
    emit(grolaw.resolve_judge(logs, chain, a))


def cmd_covenant(argv):
    if len(argv) != 3:
        die("usage: gro covenant <logs.json> <chainId> <address>")
    logs = load_logs(argv[0])
    chain = parse_int_arg(argv[1], "chainId")
    a = parse_addr_arg(argv[2])
    emit(grolaw.covenant_answer(logs, chain, a))


def cmd_quote(argv):
    if len(argv) not in (4, 5):
        die("usage: gro quote <quotes.json> <chainId> <contract> <codeHash> [<token>]")
    Q = read_json(argv[0])
    if not isinstance(Q, Obj):
        die("gro: quotes root is not an object")
    chain = parse_int_arg(argv[1], "chainId")
    contract = parse_addr_arg(argv[2])
    if not is_hex(argv[3], 32):
        die("gro: codeHash is not hex32")
    token = None
    if len(argv) == 5:
        token = parse_addr_arg(argv[4])
    emit(grolaw.quote_answer(Q, chain, contract, argv[3], token))


def cmd_bench(argv):
    if len(argv) != 3:
        die("usage: gro bench <logs.json> <chainId> <bench.json>")
    logs = load_logs(argv[0])
    chain = parse_int_arg(argv[1], "chainId")
    B = read_json(argv[2])
    emit(Obj([("bench", grolaw.bench_value(logs, chain, B))]))


def cmd_candidates(argv):
    if len(argv) != 3:
        die("usage: gro candidates <logs.json> <chainId> <query.json>")
    logs = load_logs(argv[0])
    chain = parse_int_arg(argv[1], "chainId")
    q = read_json(argv[2])
    if not isinstance(q, Obj):
        die("gro: query root is not an object")
    d = q.dict()
    if "pointers" not in d or not isinstance(d["pointers"], list) \
            or not all(isinstance(x, str) for x in d["pointers"]):
        die("gro: query.pointers is not an array of strings")
    lists = {}
    for k in ("allow", "deny"):
        if k not in d:
            lists[k] = []
            continue
        if not isinstance(d[k], list) or not all(is_hex(x, 20) for x in d[k]):
            die("gro: query.%s is not an array of hex20" % k)
        lists[k] = list(d[k])
    emit(grolaw.candidates(logs, chain, list(d["pointers"]), lists["allow"], lists["deny"]))


COMMANDS = {
    "decode": cmd_decode,
    "publication": cmd_publication,
    "resolve-identity": cmd_identity,
    "current-at": cmd_current_at,
    "resolve-guild": cmd_guild,
    "judge": cmd_judge,
    "guild-card": cmd_guild_card,
    "resolve-judge": cmd_resolve_judge,
    "covenant": cmd_covenant,
    "quote": cmd_quote,
    "bench": cmd_bench,
    "candidates": cmd_candidates,
}


def main(argv):
    if len(argv) < 2 or argv[1] not in COMMANDS:
        die("usage: gro {decode|publication|resolve-identity|current-at|resolve-guild|judge|guild-card|resolve-judge|covenant|quote|bench|candidates} ...")
    COMMANDS[argv[1]](argv[2:])
    return 0


if __name__ == "__main__":
    sys.setrecursionlimit(10000)
    sys.exit(main(sys.argv))
