"""galeed.papers/1 sections 9 and 10: the ledger as read, and the door check,
over the frozen galeed/1 core's audit."""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "impl-py"))

import glaudit                                   # noqa: E402
from glaudit import NoLabel                      # noqa: E402
from glcanon import Fault, Obj                   # noqa: E402
from glentry import validate                     # noqa: E402
from gppdoc import DocFault, check_note, check_pass, doc_id  # noqa: E402

DISPATCH_TYPE = "galeed.dispatch/1"


class View:
    __slots__ = ("root", "basis", "label", "ledger", "by_eid", "lineage", "disputed")


def _plain(v):
    if isinstance(v, Obj):
        return dict((k, _plain(x)) for k, x in v.pairs)
    if isinstance(v, list):
        return [_plain(x) for x in v]
    return v


def load_view(raw):
    """s9.0: the report and the view.  Raises NoLabel for an invalid input,
    transport refusals included."""
    parsed = glaudit.load_input(raw)
    (root, anchors, basis) = glaudit._read_inputs(parsed)
    report = glaudit.audit(raw)
    rd = report.dict()
    # s9.0: the ledger is the set of ledger entries galeed/1 s8.1 decides, the
    # carried byte strings s4.3 accepts whose judge is in the lineage and which
    # a counted undiscarded record carries; the report's ANCHORED rows name
    # exactly those entries
    seen = set()
    entries = []
    for a in anchors:
        if a.entry is None or a.entry in seen:
            continue
        seen.add(a.entry)
        try:
            entries.append(validate(a.entry))
        except Fault:
            pass
    lineage = glaudit._lineage(root, entries, anchors)
    anchored_eids = frozenset(a.dict()["entry_id"] for a in rd["anchored"])
    ledger = [e for e in entries if e.eid in anchored_eids]
    v = View()
    v.root = root
    v.basis = basis
    v.label = rd["label"]
    v.ledger = ledger
    v.by_eid = dict((e.eid, e) for e in ledger)
    v.lineage = frozenset(lineage)
    v.disputed = frozenset(f.dict()["entry_id"] for f in rd["findings"] if f.dict()["name"] == "AUTHORITY_MISMATCH")
    return v


def covering(view):
    """s9.2: on every chain named, the objects begin at block 0 and are
    contiguous; every object has registries and every lineage sender."""
    bd = _plain(view.basis)
    chains = bd["chains"]
    if not chains:
        return False
    expect = {}
    for c in chains:
        cid = c["chainId"]
        if c["fromBlock"] != expect.get(cid, 0):
            return False
        expect[cid] = c["toBlock"] + 1
        if not c["registries"]:
            return False
        if not view.lineage <= set(c["senders"]):
            return False
    return True


def dispatches(view, note_bytes):
    """s9.1."""
    want = doc_id(note_bytes)
    out = []
    for e in view.ledger:
        if e.etype != DISPATCH_TYPE:
            continue
        nh = e.body.dict().get("noteHash")
        if isinstance(nh, str) and nh == want:
            out.append(e)
    return out


def _c(state, token=None):
    return {"state": state, "token": token}


def door(note_bytes, pass_bytes, now, audit_raw, bench, current):
    """s10.  `pass_bytes` may be None; `bench` is None, the string
    'unresolved', or a list of hex20; `current` is a hex20 or the string
    'unresolved'.  Returns the result as an Obj."""
    checks = {}
    # the echoes are decided by the inputs and by no check (s10.4)
    view = None
    if audit_raw is not None:
        try:
            view = load_view(audit_raw)
        except NoLabel:
            view = None
    basis = view.basis if view is not None else None
    root = view.root if view is not None else None
    note = None
    try:
        note = check_note(note_bytes)
        checks["note"] = _c("PASS")
    except DocFault as f:
        checks["note"] = _c("FAIL", ["BAD_NOTE", f.token])
    if note is None:
        for k in ("pass", "clock", "ledger", "anchor", "bench"):
            checks[k] = _c("SKIPPED", "NO_NOTE")
        return _result(checks, now, basis, bench, root, current)

    pas = None
    if pass_bytes is None:
        checks["pass"] = _c("SKIPPED", "NO_PASS")
    else:
        try:
            pas = check_pass(pass_bytes)
        except DocFault as f:
            checks["pass"] = _c("FAIL", ["BAD_PASS", f.token])
        if pas is not None:
            if pas["noteHash"] != doc_id(note_bytes):
                checks["pass"] = _c("FAIL", "PASS_NOTE_MISMATCH")
                pas = None
            elif current == "unresolved":
                checks["pass"] = _c("UNKNOWN", "VERIFIER_UNRESOLVED")
                pas = None
            elif pas["verifier"] != current:
                checks["pass"] = _c("FAIL", "PASS_VERIFIER_MISMATCH")
                pas = None
            else:
                checks["pass"] = _c("PASS")

    if now is None:
        checks["clock"] = _c("UNKNOWN", "NO_CLOCK")
    elif pas is None:
        if now > note["deadline"]:
            checks["clock"] = _c("FAIL", "NOTE_EXPIRED")
        else:
            checks["clock"] = _c("SKIPPED", "NO_PASS")
    elif now < pas["window"]["from"]:
        checks["clock"] = _c("FAIL", "NOT_YET")
    elif now > pas["window"]["to"]:
        checks["clock"] = _c("FAIL", "PASS_EXPIRED")
    elif now > note["deadline"]:
        checks["clock"] = _c("FAIL", "NOTE_EXPIRED")
    else:
        checks["clock"] = _c("PASS")

    if audit_raw is None:
        checks["ledger"] = _c("UNKNOWN", "NO_LEDGER")
    elif view is None:
        checks["ledger"] = _c("UNKNOWN", "LEDGER_INVALID")
    elif view.label == "BROKEN_CHAIN":
        checks["ledger"] = _c("FAIL", "BROKEN_LEDGER")
    elif note["judge"] not in view.lineage:
        checks["ledger"] = _c("UNKNOWN", "NOT_IN_LINEAGE")
    else:
        checks["ledger"] = _c("PASS")

    if checks["ledger"]["state"] != "PASS":
        checks["anchor"] = _c("UNKNOWN", "LEDGER_UNREAD")
    else:
        D = dispatches(view, note_bytes)
        cov = covering(view)
        if not D:
            if not view.ledger:
                checks["anchor"] = _c("UNKNOWN", "NOT_DISPATCHED")
            elif not cov:
                checks["anchor"] = _c("UNKNOWN", "NOT_DISPATCHED")
            elif view.label == "COMPLETE":
                checks["anchor"] = _c("FAIL", "NOT_DISPATCHED")
            else:
                checks["anchor"] = _c("UNKNOWN", "NOT_DISPATCHED")
        elif any(e.eid not in view.disputed for e in D):
            checks["anchor"] = _c("PASS")
        else:
            checks["anchor"] = _c("UNKNOWN", "DISPATCH_DISPUTED")

    if bench is None:
        checks["bench"] = _c("SKIPPED", "NO_BENCH")
    elif bench == "unresolved":
        checks["bench"] = _c("UNKNOWN", "BENCH_UNRESOLVED")
    elif note["verifier"] in bench:
        checks["bench"] = _c("PASS")
    else:
        checks["bench"] = _c("FAIL", "NOT_ON_BENCH")
    return _result(checks, now, basis, bench, root, current)


def _result(checks, now, basis, bench, root, current):
    states = [c["state"] for c in checks.values()]
    if "FAIL" in states:
        verdict = "FAIL"
    elif "UNKNOWN" in states or any(c["state"] == "SKIPPED" and c["token"] != "NO_BENCH" for c in checks.values()):
        verdict = "PARTIAL"
    else:
        verdict = "GREEN"
    co = Obj([(k, Obj([("state", c["state"]), ("token", c["token"])])) for k, c in sorted(checks.items())])
    bench_v = bench if bench is None or isinstance(bench, str) else list(bench)
    return Obj([("basis", basis), ("bench", bench_v), ("checks", co), ("current", current),
                ("now", now), ("root", root), ("verdict", verdict)])
