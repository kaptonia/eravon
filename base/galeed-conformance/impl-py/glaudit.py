"""galeed/1 sections 7, 8 and 9.4: the audit."""

import json

from glcanon import MAX_INT, Fault, Obj
from glcrypto import sha256
from glentry import KNOWN_TYPES, is_hex, is_int, validate

MAXSEQ = MAX_INT


class NoLabel(Exception):
    pass


def need(cond):
    if not cond:
        raise NoLabel()


# --------------------------------------------------------------------------
# Lenient reading of the harness's audit input file
# --------------------------------------------------------------------------

def _reject(*_a, **_k):
    raise NoLabel()


def _parse_int(sv):
    # HARNESS: every number in the file is an integer of s3.1's universe
    if sv[:1] == "-" or int(sv) > MAX_INT:
        raise NoLabel()
    return int(sv)


def _pairs_hook(pairs):
    keys = [k for k, _ in pairs]
    if len(set(keys)) != len(keys):
        raise NoLabel()
    return Obj(pairs)


def _depth_ok(text):
    # HARNESS: a container opening at depth 129 makes the file unreadable
    depth, in_str, esc = 0, False, False
    for ch in text:
        if in_str:
            if esc:
                esc = False
            elif ch == "\\":
                esc = True
            elif ch == '"':
                in_str = False
        elif ch == '"':
            in_str = True
        elif ch in "[{":
            depth += 1
            if depth > 128:
                return False
        elif ch in "]}":
            depth -= 1
    return True


def _no_lone_surrogate(v):
    # HARNESS: every string of the file decodes to scalar values
    if isinstance(v, str):
        need(not any(0xD800 <= ord(c) <= 0xDFFF for c in v))
    elif isinstance(v, list):
        for x in v:
            _no_lone_surrogate(x)
    elif isinstance(v, Obj):
        for k, x in v.dict().items():
            _no_lone_surrogate(k)
            _no_lone_surrogate(x)


def load_input(raw):
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError:
        raise NoLabel()
    need(_depth_ok(text))
    try:
        v = json.loads(text, object_pairs_hook=_pairs_hook,
                          parse_float=_reject, parse_constant=_reject,
                          parse_int=_parse_int)
    except NoLabel:
        raise
    except Exception:
        raise NoLabel()
    _no_lone_surrogate(v)
    return v


def _hexbytes(v):
    """A `0x<hex>` blob of any length (harness encoding, not a law field)."""
    if not isinstance(v, str) or len(v) < 2 or v[0] != "0" or v[1] != "x":
        raise NoLabel()
    body = v[2:]
    if len(body) % 2:
        raise NoLabel()
    for c in body:
        if c not in "0123456789abcdefABCDEF":
            raise NoLabel()
    return bytes.fromhex(body)


def _uint(v):
    need(is_int(v) and 0 <= v <= MAX_INT)
    return v


def _hex20(v):
    need(is_hex(v, 20))
    return v


def _hex32(v):
    need(is_hex(v, 32))
    return v


# --------------------------------------------------------------------------
# s9.4 basis validity
# --------------------------------------------------------------------------

_CHAIN_KEYS = frozenset(("chainId", "fromBlock", "toBlock", "registries",
                         "senders"))
_CLASS_KEYS = frozenset(("codeHash",))
_BASIS_KEYS = frozenset(("chains",))


def _exact(o, keys):
    need(isinstance(o, Obj))
    d = o.dict()
    need(frozenset(d.keys()) == keys)
    return d


def check_basis(basis):
    """Raise NoLabel unless `basis` is a galeed/1 basis."""
    d = _exact(basis, _BASIS_KEYS)
    chains = d["chains"]
    need(isinstance(chains, list))
    prev = None
    for c in chains:
        cd = _exact(c, _CHAIN_KEYS)
        cid = _uint(cd["chainId"])
        fb = _uint(cd["fromBlock"])
        tb = _uint(cd["toBlock"])
        need(fb <= tb)
        lists = {}
        need(isinstance(cd["senders"], list))
        prev_a = None
        for a in cd["senders"]:
            _hex20(a)
            # s9.4: bytewise ascending, no repeated element
            need(prev_a is None or prev_a.encode("ascii") < a.encode("ascii"))
            prev_a = a
        lists["senders"] = tuple(cd["senders"])
        # s9.4: registries lists its hex20 elements first, bytewise ascending,
        # then its class items {codeHash: hex32}, bytewise ascending by
        # codeHash, no element repeated in either part
        need(isinstance(cd["registries"], list))
        regs = []
        prev_a = None
        prev_c = None
        for a in cd["registries"]:
            if isinstance(a, str):
                need(prev_c is None)
                _hex20(a)
                need(prev_a is None or prev_a.encode("ascii") < a.encode("ascii"))
                prev_a = a
                regs.append(("a", a))
            else:
                h = _exact(a, _CLASS_KEYS)["codeHash"]
                _hex32(h)
                need(prev_c is None or prev_c.encode("ascii") < h.encode("ascii"))
                prev_c = h
                regs.append(("c", h))
        lists["registries"] = tuple(regs)
        # s9.4: chains ordered by (chainId, fromBlock) ascending; two objects of
        # one chainId have ranges that do not overlap and, where they touch,
        # registries or senders that differ
        if prev is not None:
            need((cid, fb) > (prev[0], prev[1]))
            if cid == prev[0]:
                need(fb > prev[2])
                if fb == prev[2] + 1:
                    need(lists["registries"] != prev[3] or lists["senders"] != prev[4])
        prev = (cid, fb, tb, lists["registries"], lists["senders"])


def _reaches(basis, rec):
    """s9.4 [C]: some one chains object holds the record's chain, block, and sender."""
    for c in basis.dict()["chains"]:
        cd = c.dict()
        if (cd["chainId"] == rec.chainId and cd["fromBlock"] <= rec.blockNumber <= cd["toBlock"]
                and rec.sender in cd["senders"]):
            return True
    return False


# --------------------------------------------------------------------------
# s8 the audit
# --------------------------------------------------------------------------

class Anchor:
    __slots__ = ("chainId", "blockNumber", "blockTimestamp", "tx", "sender",
                 "hash", "verdict", "entry")


VERDICTS = ("counted", "UNPROVEN", "VOID")


def _read_inputs(v):
    need(isinstance(v, Obj))
    d = v.dict()
    for k in ("root", "anchors", "basis"):
        need(k in d)
    root = _hex20(d["root"])

    need(isinstance(d["anchors"], list))
    anchors = {}
    for a in d["anchors"]:
        need(isinstance(a, Obj))
        ad = a.dict()
        for k in ("chainId", "blockNumber", "blockTimestamp", "tx", "sender",
                  "hash", "verdict"):
            need(k in ad)
        rec = Anchor()
        rec.chainId = _uint(ad["chainId"])
        rec.blockNumber = _uint(ad["blockNumber"])
        rec.blockTimestamp = _uint(ad["blockTimestamp"])
        rec.tx = _hex32(ad["tx"])
        rec.sender = _hex20(ad["sender"])
        rec.hash = _hex32(ad["hash"])
        need(isinstance(ad["verdict"], str) and ad["verdict"] in VERDICTS)
        rec.verdict = ad["verdict"]
        # s9.2: the eighth member, present where the log's data carried the
        # bytes; a byte string whose sha256 is the record's hash, or the
        # record is no anchor record
        if "entry" in ad:
            rec.entry = _hexbytes(ad["entry"])
            need("0x" + sha256(rec.entry).hex() == rec.hash)
        else:
            rec.entry = None
        key = (rec.chainId, rec.blockNumber, rec.tx, rec.hash)
        prior = anchors.get(key)
        if prior is not None:
            # s9.4: at most one record per (chainId, blockNumber, tx, hash);
            # two that disagree on a remaining member, entry among them, are
            # no audit input.
            need(prior.blockTimestamp == rec.blockTimestamp
                 and prior.sender == rec.sender
                 and prior.verdict == rec.verdict
                 and prior.entry == rec.entry)
        else:
            anchors[key] = rec

    check_basis(d["basis"])
    for rec in anchors.values():
        need(_reaches(d["basis"], rec))
    return (root, list(anchors.values()), d["basis"])


def retirements(entries, records):
    """s7.4: each key's retirement, the smallest blockTimestamp of the records
    of the anchor set, on any chain and whatever their sender and verdict,
    that carry a succession or a migration the key signed, paired with its
    retiring seq, the smallest seq among the successions and migrations it
    signed that a record at its retirement carries."""
    by_bytes = dict((e.raw, e) for e in entries)
    out = {}
    for a in records:
        e = by_bytes.get(a.entry) if a.entry is not None else None
        if e is not None and e.etype in ("succession", "migration"):
            cur = out.get(e.judge)
            if cur is None or (a.blockTimestamp, e.seq) < cur:
                out[e.judge] = (a.blockTimestamp, e.seq)
    return out


def retired_at(retire, key, rec, by_bytes):
    """s7.4: the key has retired at the record: the record's blockTimestamp
    is later than the key's retirement, or equals it and the record carries
    a byte string that passes s4.3, signed by the key, at a seq greater than
    the retiring seq."""
    cur = retire.get(key)
    if cur is None:
        return False
    t, k = cur
    if rec.blockTimestamp > t:
        return True
    if rec.blockTimestamp == t and rec.entry is not None:
        e = by_bytes.get(rec.entry)
        return e is not None and e.judge == key and e.seq > k
    return False


def _lineage(root, entries, records):
    """s7.4: the least fixpoint seeded with the root, extended by the `to` of
    every carried succession whose judge is in the set and that some record
    carries at which its judge has not retired."""
    retire = retirements(entries, records)
    by_bytes = dict((e.raw, e) for e in entries)
    carriers = {}
    for a in records:
        if a.entry is not None:
            carriers.setdefault(a.entry, []).append(a)
    admitted = [e for e in entries if e.etype == "succession"
                and any(not retired_at(retire, e.judge, a, by_bytes) for a in carriers.get(e.raw, ()))]
    lin = {root}
    changed = True
    while changed:
        changed = False
        for e in admitted:
            if e.judge in lin:
                to = e.body.dict()["to"]
                if to not in lin:
                    lin.add(to)
                    changed = True
    return lin


def _finding(name, position, entry_id, extra, hard, second=""):
    """A finding as (sort key..., row).  s8.7 item 3 sorts by
    (position, finding name, entry_id, second entry_id), the empty second
    entry_id ordering before every hex32."""
    pairs = [("name", name), ("position", position), ("entry_id", entry_id),
             ("hard", hard)]
    pairs.extend(extra)
    return (position, name.encode("ascii"), entry_id.encode("ascii"),
            second.encode("ascii"), Obj(pairs))


def _walk(root, ledger):
    order = sorted(ledger, key=lambda e: (e.seq, e.eidb))
    by_seq = {}
    for e in ledger:
        by_seq.setdefault(e.seq, []).append(e)
    findings = []
    expected = 0
    auth = root
    certain = True
    prev_seq = None
    any_gap = False
    i = 0
    n = len(order)
    while i < n:
        s = order[i].seq
        j = i
        while j < n and order[j].seq == s:
            j += 1
        group = order[i:j]
        for e in group:
            # SEQ_GAP
            if e.seq == expected:
                expected = e.seq if e.seq == MAXSEQ else e.seq + 1
            elif prev_seq is not None and e.seq == prev_seq:
                pass                                     # fork twin
            else:
                findings.append(_finding(
                    "SEQ_GAP", e.seq, e.eid,
                    [("expected", expected), ("actual", e.seq)], False))
                any_gap = True
                expected = e.seq if e.seq == MAXSEQ else e.seq + 1
                certain = False
            prev_seq = e.seq
            # PREV_MISMATCH
            if e.seq >= 1:
                pred = by_seq.get(e.seq - 1)
                if pred and not any(x.eid == e.prev for x in pred):
                    findings.append(_finding(
                        "PREV_MISMATCH", e.seq, e.eid, [("seq", e.seq)], True))
            # AUTHORITY_MISMATCH
            if e.seq >= 1 and e.judge != auth:
                findings.append(_finding(
                    "AUTHORITY_MISMATCH", e.seq, e.eid,
                    [("seq", e.seq), ("certain", certain)], certain))
            # ROOT_MISMATCH
            if e.seq == 0 and e.judge != root:
                findings.append(_finding(
                    "ROOT_MISMATCH", 0, e.eid,
                    [("seq", 0), ("actual", e.judge)], True))
        # s8.2: a migration signed by the key in office closes the ledger;
        # else only a succession signed by the key in office moves authority
        if auth is not None and any(e.etype == "migration" and e.judge == auth for e in group):
            auth = None
        succs = [e for e in group if e.etype == "succession" and e.judge == auth]
        if auth is not None and succs:
            auth = min(succs, key=lambda e: e.eidb).body.dict()["to"]
        i = j
    return findings, any_gap


def _equivocation(ledger):
    out = []
    m = len(ledger)
    for i in range(m):
        a = ledger[i]
        for j in range(i + 1, m):
            b = ledger[j]
            if a.eid == b.eid:
                continue
            if a.seq == b.seq or (a.prev is not None and a.prev == b.prev):
                if a.eidb < b.eidb:
                    x, y = a, b
                else:
                    x, y = b, a
                seq = a.seq if a.seq < b.seq else b.seq
                out.append(_finding("EQUIVOCATION", seq, x.eid,
                                    [("seq", seq), ("a", x.eid), ("b", y.eid)],
                                    True, second=y.eid))
    return out


def _record_rows(recs):
    """An UNPROVEN, VOID row: the five members and the entry where carried."""
    recs = sorted(recs, key=lambda a: (a.hash, a.chainId, a.blockNumber, a.tx))
    return [Obj([("hash", a.hash), ("chainId", a.chainId),
                 ("blockNumber", a.blockNumber), ("tx", a.tx),
                 ("sender", a.sender)]
                + ([("entry", "0x" + a.entry.hex())] if a.entry is not None else []))
            for a in recs]


def audit(raw):
    (root, records, basis) = _read_inputs(load_input(raw))

    # s8.1: the carried byte strings, whatever the record's sender and verdict
    carried = []
    seen = set()
    for a in records:
        if a.entry is not None and a.entry not in seen:
            seen.add(a.entry)
            carried.append(a.entry)
    entries = []
    by_bytes = {}
    malformed = {}
    for b in carried:
        try:
            e = validate(b)
            entries.append(e)
            by_bytes[b] = e
        except Fault as f:
            malformed["0x" + sha256(b).hex()] = f.token
    # s7.4 over every carried byte string that passes s4.3, read with the
    # records that carry it
    lineage = _lineage(root, entries, records)
    retire = retirements(entries, records)
    # the trim: a sender outside the lineage, or a record at which its sender
    # has retired
    def kept(a):
        return a.sender in lineage and not retired_at(retire, a.sender, a, by_bytes)
    discarded = [a for a in records if not kept(a)]
    anchors = [a for a in records if kept(a)]
    counted = [a for a in anchors if a.verdict == "counted"]
    counted_hashes = set(a.hash for a in counted)
    counted_carried = set(a.entry for a in counted if a.entry is not None)
    # s8.1: the ledger is the carried entries whose judge is in the lineage
    # and which a counted undiscarded record carries (s8.6)
    ledger = [e for e in entries if e.judge in lineage and e.raw in counted_carried]
    excluded = [e for e in entries if e.judge not in lineage]

    # s8.2 and s8.4
    findings, any_gap = _walk(root, ledger)
    findings.extend(_equivocation(ledger))
    # s8.4: the carried record; one finding per byte string carried by an
    # undiscarded counted record that is no entry of this ledger
    carried_counted = {}
    for a in counted:
        if a.entry is not None:
            carried_counted[a.entry] = a.hash
    for b, h in carried_counted.items():
        e = by_bytes.get(b)
        if e is None:
            findings.append(_finding("CARRIED_MALFORMED", 0, h,
                                     [("seq", 0), ("token", malformed[h])], True))
        elif e.judge not in lineage:
            findings.append(_finding("CARRIED_MALFORMED", e.seq, h,
                                     [("seq", e.seq), ("token", "FOREIGN")], True))
    dedup = {}
    for f in findings:
        dedup[(f[0], f[1], f[2], f[3])] = f
    findings = sorted(dedup.values(), key=lambda f: (f[0], f[1], f[2], f[3]))
    any_hard = any(f[4].dict()["hard"] for f in findings)

    # s8.5 reconciliation
    have = set(e.eid for e in ledger)
    missing = []
    for h in sorted(h for h in counted_hashes if h not in have):
        recs = sorted((a for a in counted if a.hash == h),
                      key=lambda a: (a.chainId, a.blockNumber, a.tx))
        missing.append(Obj([
            ("hash", h),
            ("anchors", [Obj([("chainId", a.chainId), ("blockNumber", a.blockNumber),
                              ("tx", a.tx), ("sender", a.sender)])
                         for a in recs]),
        ]))
    # s8.7 item 5: every ledger entry, with the counted records that committed to it
    anchored = []
    for eid_ in sorted(have):
        recs = sorted((a for a in counted if a.hash == eid_),
                      key=lambda a: (a.chainId, a.blockNumber, a.tx))
        anchored.append(Obj([
            ("entry_id", eid_),
            ("anchors", [Obj([("chainId", a.chainId), ("blockNumber", a.blockNumber),
                              ("blockTimestamp", a.blockTimestamp), ("tx", a.tx),
                              ("sender", a.sender)])
                         for a in recs]),
        ]))
    # s8.7 items 6, 7 and 8
    excluded_rows = [Obj([("seq", e.seq), ("judge", e.judge),
                          ("entry_id", e.eid)])
                     for e in sorted(excluded, key=lambda e: (e.seq, e.eidb))]
    unknown_rows = [Obj([("seq", e.seq), ("entryType", e.etype), ("entry_id", e.eid)])
                    for e in sorted((e for e in ledger if e.etype not in KNOWN_TYPES),
                                    key=lambda e: (e.seq, e.etype.encode("ascii"), e.eidb))]
    malformed_rows = [Obj([("entry_id", h), ("token", malformed[h])])
                      for h in sorted(malformed)]

    # s8.7 items 9 and 10
    unproven_rows = _record_rows([a for a in anchors if a.verdict == "UNPROVEN"])
    void_rows = _record_rows([a for a in anchors if a.verdict == "VOID"])

    # s8.7 item 11: every record the trim discarded, its seven members
    discarded.sort(key=lambda a: (a.sender, a.chainId, a.blockNumber, a.tx, a.hash))
    discarded_rows = [Obj([("chainId", a.chainId), ("blockNumber", a.blockNumber),
                           ("blockTimestamp", a.blockTimestamp), ("tx", a.tx),
                           ("sender", a.sender), ("hash", a.hash), ("verdict", a.verdict)]
                          + ([("entry", "0x" + a.entry.hex())] if a.entry is not None else []))
                      for a in discarded]

    # s8.7 item 12: an UNPROVEN record whose hash no undiscarded counted
    # record carries moves the label
    unproven_open = any(a.hash not in counted_hashes
                        for a in anchors if a.verdict == "UNPROVEN")
    if any_hard:
        label = "BROKEN_CHAIN"
    elif unproven_open:
        label = "UNPROVEN"
    elif any_gap or missing:
        label = "GAPS"
    else:
        label = "COMPLETE"

    return Obj([
        ("root", root),
        ("basis", basis),
        ("entries", len(ledger)),
        ("findings", [f[4] for f in findings]),
        ("missing", missing),
        ("anchored", anchored),
        ("excluded", excluded_rows),
        ("unknown_type", unknown_rows),
        ("malformed", malformed_rows),
        ("unproven", unproven_rows),
        ("void", void_rows),
        ("discarded", discarded_rows),
        ("label", label),
    ])
