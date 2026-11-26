"""galeed.roster/1 (docs/galeed-roster-v1.md): decoding a publication (s2,
s3, s3a, s3b, s3c), the succession book's replay (s4), and resolution (s5,
s6), over the frozen galeed/1 core for keccak256, the reader, the forms, and
the basis well-formedness test."""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "impl-py"))

from glcanon import Obj                                   # noqa: E402
from glcrypto import keccak256, sha256                    # noqa: E402
from glentry import is_hex, is_int                        # noqa: E402
import glaudit                                            # noqa: E402

KIND_ROSTER = "galeed.roster/1"
KIND_GUILDCARD = "galeed.guildcard/1"
KIND_JUDGE = "galeed.judge/1"
KIND_GUILDAMANAH = "galeed.guildamanah/1"
# s1: the reserved prefixes, by the kind name a reader asks candidateOf for
PREFIXES = {"roster": "galeed.roster/", "guildcard": "galeed.guildcard/",
            "judge": "galeed.judge/", "guildamanah": "galeed.guildamanah/",
            "verifier": "galeed.verifier/"}
POINTER_PREFIX = "galeed:guild:"
ZERO = "0x" + "00" * 20

ROSTER_KEYS = frozenset(("kind", "guild", "chainId", "issued", "judges", "bench"))
GUILDCARD_KEYS = frozenset(("kind", "guild", "issued", "name", "home", "note_md"))
JUDGE_KEYS = frozenset(("kind", "judge", "issued", "amanah", "ledger", "name", "home", "note_md"))
GUILDAMANAH_KEYS = frozenset(("kind", "guild", "chainId", "issued", "version", "text_md",
                              "venues", "quotes"))
_HEXLOW = frozenset("0123456789abcdef")


class Fault(Exception):
    def __init__(self, token):
        super().__init__(token)
        self.token = token


def _int_spelling(s):
    return (isinstance(s, str) and s != "" and all(c in "0123456789" for c in s)
            and not (len(s) > 1 and s[0] == "0") and len(s) <= 16 and int(s) <= glaudit.MAX_INT)


def _is_str(v):
    return isinstance(v, str)


def read_document(b):
    """The reader of s1 (HARNESS.md's audit-input reader); None when unreadable."""
    try:
        return glaudit.load_input(b)
    except glaudit.NoLabel:
        return None


# --------------------------------------------------------------------------
# s5.10 quotes
# --------------------------------------------------------------------------

def venue_key_ok(k):
    """s5.10: <chainId>:<hex20|hex32> with an optional trailing :<hex20> token."""
    parts = k.split(":")
    if len(parts) not in (2, 3) or not _int_spelling(parts[0]):
        return False
    if not (is_hex(parts[1], 20) or is_hex(parts[1], 32)):
        return False
    return len(parts) == 2 or is_hex(parts[2], 20)


def quote_spelling_ok(v):
    """s5.10: 0x followed by an even number of lowercase hexadecimal digits."""
    if not isinstance(v, str) or len(v) < 2 or v[0] != "0" or v[1] != "x":
        return False
    body = v[2:]
    return len(body) % 2 == 0 and all(c in _HEXLOW for c in body)


def quote_of(Q, chain_id, contract, code_hash, token):
    """s5.10.  Q is an Obj (the quotes object); token is a hex20 or None.
    Returns ("QUOTE", key, value) | ("E_QUOTE_FORM", key, None) | ("QUOTE_NONE", None, None)."""
    d = Q.dict()
    c = str(chain_id)
    keys = []
    if token is not None:
        keys.append("%s:%s:%s" % (c, contract, token))
    keys.append("%s:%s" % (c, contract))
    if token is not None:
        keys.append("%s:%s:%s" % (c, code_hash, token))
    keys.append("%s:%s" % (c, code_hash))
    keys.append(c)
    for k in keys:
        if k in d:
            v = d[k]
            if quote_spelling_ok(v):
                return ("QUOTE", k, v)
            return ("E_QUOTE_FORM", k, None)
    return ("QUOTE_NONE", None, None)


def quote_answer(Q, chain_id, contract, code_hash, token):
    r, k, v = quote_of(Q, chain_id, contract, code_hash, token)
    if r == "QUOTE":
        return Obj([("key", k), ("quote", v), ("verdict", r)])
    if r == "E_QUOTE_FORM":
        return Obj([("key", k), ("verdict", r)])
    return Obj([("verdict", r)])


# --------------------------------------------------------------------------
# s3, s3a, s3b, s3c: the tables
# --------------------------------------------------------------------------

def element_address(e):
    """s3.3: the address of a judges or verifiers element, or None."""
    if isinstance(e, str):
        return e if is_hex(e, 20) else None
    if isinstance(e, Obj):
        d = e.dict()
        a = d.get("addr")
        if "addr" in d and is_hex(a, 20):
            return a
    return None


def _verifier_view(e):
    """s3.3: the element view the harness's decode shows, or None where the
    element is outside its two forms."""
    if isinstance(e, str):
        return {"addr": e} if is_hex(e, 20) else None
    if isinstance(e, Obj):
        d = e.dict()
        if "addr" not in d or not is_hex(d["addr"], 20):
            return None
        if "name" not in d or not _is_str(d["name"]):
            return None
        if "protocols" not in d or not isinstance(d["protocols"], list):
            return None
        if not all(is_hex(p, 32) for p in d["protocols"]):
            return None
        return {"addr": d["addr"], "name": d["name"], "protocols": list(d["protocols"])}
    return None


def _roster(d):
    """s3.2 over the object's members; a dict of the read roster."""
    if "guild" not in d or not is_hex(d["guild"], 20):
        raise Fault("E_ROSTER_GUILD")
    if "chainId" not in d or not is_int(d["chainId"]):
        raise Fault("E_ROSTER_CHAIN")
    if "issued" not in d or not is_int(d["issued"]):
        raise Fault("E_ROSTER_ISSUED")
    if "judges" not in d or not isinstance(d["judges"], list):
        raise Fault("E_ROSTER_JUDGES")
    judges = []
    for e in d["judges"]:
        a = element_address(e)
        if a is None:
            raise Fault("E_ROSTER_JUDGES")
        judges.append(a)
    bench = None
    if "bench" in d:
        b = d["bench"]
        if not isinstance(b, Obj):
            raise Fault("E_ROSTER_BENCH")
        bd = b.dict()
        if "verifiers" not in bd or not isinstance(bd["verifiers"], list):
            raise Fault("E_ROSTER_BENCH")
        if "protocols" not in bd or not isinstance(bd["protocols"], list):
            raise Fault("E_ROSTER_BENCH")
        views = []
        for e in bd["verifiers"]:
            v = _verifier_view(e)
            if v is None:
                raise Fault("E_ROSTER_BENCH")
            views.append(v)
        for p in bd["protocols"]:
            if not is_hex(p, 32):
                raise Fault("E_ROSTER_BENCH")
        bench = {"verifiers": views, "protocols": list(bd["protocols"])}
    r = {"guild": d["guild"], "chainId": d["chainId"], "issued": d["issued"],
         "judges": judges, "bench": bench,
         "unknownFields": sorted((k for k in d if k not in ROSTER_KEYS),
                                 key=lambda k: k.encode("utf-8"))}
    return r


def _guild_card(d):
    """s3c.2 over the object's members; a dict of the read guild card."""
    if "guild" not in d or not is_hex(d["guild"], 20):
        raise Fault("E_GUILDCARD_GUILD")
    if "issued" not in d or not is_int(d["issued"]):
        raise Fault("E_GUILDCARD_ISSUED")
    for k, tok in (("name", "E_GUILDCARD_NAME"), ("home", "E_GUILDCARD_HOME"),
                   ("note_md", "E_GUILDCARD_NOTE")):
        if k in d and not _is_str(d[k]):
            raise Fault(tok)
    c = {"guild": d["guild"], "issued": d["issued"],
         "unknownFields": sorted((k for k in d if k not in GUILDCARD_KEYS),
                                 key=lambda k: k.encode("utf-8"))}
    for k in ("name", "home", "note_md"):
        if k in d:
            c[k] = d[k]
    return c


def _basis_ok(v):
    """s3a.1: a galeed/1 s9.4 basis, by the embedded core's well-formedness test."""
    try:
        glaudit.check_basis(v)
    except glaudit.NoLabel:
        return False
    return True


def _judge_doc(d):
    """s3a.2 over the object's members; a dict of the read judge document."""
    if "judge" not in d or not is_hex(d["judge"], 20):
        raise Fault("E_JUDGE_JUDGE")
    if "issued" not in d or not is_int(d["issued"]):
        raise Fault("E_JUDGE_ISSUED")
    am = d.get("amanah")
    if "amanah" not in d or not isinstance(am, Obj) or "entry_id" not in am.dict() \
            or not is_hex(am.dict()["entry_id"], 32):
        raise Fault("E_JUDGE_AMANAH")
    lg = d.get("ledger")
    if "ledger" not in d or not isinstance(lg, Obj):
        raise Fault("E_JUDGE_LEDGER")
    ld = lg.dict()
    if "root" not in ld or not is_hex(ld["root"], 20):
        raise Fault("E_JUDGE_LEDGER")
    if "basis" not in ld or not _basis_ok(ld["basis"]):
        raise Fault("E_JUDGE_LEDGER")
    for k, tok in (("name", "E_JUDGE_NAME"), ("home", "E_JUDGE_HOME"), ("note_md", "E_JUDGE_NOTE")):
        if k in d and not _is_str(d[k]):
            raise Fault(tok)
    j = {"judge": d["judge"], "issued": d["issued"],
         "amanah": Obj([("entry_id", am.dict()["entry_id"])]),
         "ledger": Obj([("basis", ld["basis"]), ("root", ld["root"])]),
         "unknownFields": sorted((k for k in d if k not in JUDGE_KEYS),
                                 key=lambda k: k.encode("utf-8"))}
    for k in ("name", "home", "note_md"):
        if k in d:
            j[k] = d[k]
    return j


def _venues_ok(vs):
    if not isinstance(vs, list):
        return False
    for el in vs:
        if not isinstance(el, Obj):
            return False
        ed = el.dict()
        if "chainId" not in ed or not is_int(ed["chainId"]):
            return False
        if "contract" not in ed or not is_hex(ed["contract"], 20):
            return False
        if "codeHash" not in ed or not is_hex(ed["codeHash"], 32):
            return False
    return True


def _guild_covenant(d):
    """s3b.2 over the object's members; a dict of the read guild covenant."""
    if "guild" not in d or not is_hex(d["guild"], 20):
        raise Fault("E_GUILDAMANAH_GUILD")
    if "chainId" not in d or not is_int(d["chainId"]):
        raise Fault("E_GUILDAMANAH_CHAIN")
    if "issued" not in d or not is_int(d["issued"]):
        raise Fault("E_GUILDAMANAH_ISSUED")
    if "version" not in d or not is_int(d["version"]):
        raise Fault("E_GUILDAMANAH_VERSION")
    if "text_md" not in d or not _is_str(d["text_md"]):
        raise Fault("E_GUILDAMANAH_TEXT")
    if "venues" in d and not _venues_ok(d["venues"]):
        raise Fault("E_GUILDAMANAH_VENUES")
    if "quotes" in d and not isinstance(d["quotes"], Obj):
        raise Fault("E_GUILDAMANAH_QUOTES")
    g = {"guild": d["guild"], "chainId": d["chainId"], "issued": d["issued"],
         "version": d["version"], "text_md": d["text_md"],
         "sha256": "0x" + sha256(d["text_md"].encode("utf-8")).hex(),
         "unknownFields": sorted((k for k in d if k not in GUILDAMANAH_KEYS),
                                 key=lambda k: k.encode("utf-8"))}
    if "venues" in d:
        g["venues"] = [Obj([("chainId", el.dict()["chainId"]), ("codeHash", el.dict()["codeHash"]),
                            ("contract", el.dict()["contract"])]) for el in d["venues"]]
    if "quotes" in d:
        g["quotes"] = d["quotes"]
    return g


# --------------------------------------------------------------------------
# s2.1 decoding
# --------------------------------------------------------------------------

def decode(pub):
    """s2.1.  Returns ("roster", R) | ("judge", J) | ("guildamanah", G) |
    ("guildcard", C) | ("unknown", kind); raises Fault."""
    doc = pub["document"]
    if "0x" + keccak256(doc).hex() != pub["word"]:
        raise Fault("E_HASH")
    v = read_document(doc)
    if v is None or not isinstance(v, Obj):
        raise Fault("E_JSON")
    d = v.dict()
    if "kind" not in d or not _is_str(d["kind"]):
        raise Fault("E_KIND")
    kind = d["kind"]
    if kind == KIND_ROSTER:
        return ("roster", _roster(d))
    if kind == KIND_JUDGE:
        return ("judge", _judge_doc(d))
    if kind == KIND_GUILDAMANAH:
        return ("guildamanah", _guild_covenant(d))
    if kind == KIND_GUILDCARD:
        return ("guildcard", _guild_card(d))
    return ("unknown", kind)


def kind_string(pub):
    """s1 (of a kind): the decoded `kind` string where steps 1 to 3 pass and
    kind is a string, else None (the publication is of every kind)."""
    doc = pub["document"]
    if "0x" + keccak256(doc).hex() != pub["word"]:
        return None
    v = read_document(doc)
    if v is None or not isinstance(v, Obj):
        return None
    k = v.dict().get("kind")
    if "kind" not in v.dict() or not _is_str(k):
        return None
    return k


def of_kind(pub, K):
    """s1: of kind K (the name of a reserved prefix)."""
    k = kind_string(pub)
    return k is None or k.startswith(PREFIXES[K])


def _roster_view(x):
    pairs = [("guild", x["guild"]), ("chainId", x["chainId"]), ("issued", x["issued"]),
             ("judges", list(x["judges"])), ("unknownFields", list(x["unknownFields"]))]
    if x["bench"] is not None:
        views = []
        for v in x["bench"]["verifiers"]:
            vp = [("addr", v["addr"])]
            if "name" in v:
                vp += [("name", v["name"]), ("protocols", list(v["protocols"]))]
            views.append(Obj(vp))
        pairs.append(("bench", Obj([("protocols", list(x["bench"]["protocols"])),
                                    ("verifiers", views)])))
    return Obj(pairs)


def _judge_view(x):
    pairs = [("judge", x["judge"]), ("issued", x["issued"]), ("amanah", x["amanah"]),
             ("ledger", x["ledger"]), ("unknownFields", list(x["unknownFields"]))]
    for k in ("name", "home", "note_md"):
        if k in x:
            pairs.append((k, x[k]))
    return Obj(pairs)


def _card_view(x):
    pairs = [("guild", x["guild"]), ("issued", x["issued"]),
             ("unknownFields", list(x["unknownFields"]))]
    for k in ("name", "home", "note_md"):
        if k in x:
            pairs.append((k, x[k]))
    return Obj(pairs)


def _covenant_view(x):
    pairs = [("guild", x["guild"]), ("chainId", x["chainId"]), ("issued", x["issued"]),
             ("version", x["version"]), ("text_md", x["text_md"]), ("sha256", x["sha256"]),
             ("unknownFields", list(x["unknownFields"]))]
    for k in ("venues", "quotes"):
        if k in x:
            pairs.append((k, x[k]))
    return Obj(pairs)


def decode_answer(pub):
    """The `gro decode` answer object for a publication."""
    try:
        what, x = decode(pub)
    except Fault as f:
        return Obj([("ok", False), ("token", f.token)])
    if what == "unknown":
        return Obj([("kind", x), ("known", False), ("ok", True)])
    if what == "roster":
        return Obj([("kind", KIND_ROSTER), ("known", True), ("ok", True), ("roster", _roster_view(x))])
    if what == "judge":
        return Obj([("judge", _judge_view(x)), ("kind", KIND_JUDGE), ("known", True), ("ok", True)])
    if what == "guildamanah":
        return Obj([("guildamanah", _covenant_view(x)), ("kind", KIND_GUILDAMANAH), ("known", True), ("ok", True)])
    return Obj([("guildcard", _card_view(x)), ("kind", KIND_GUILDCARD), ("known", True), ("ok", True)])


# --------------------------------------------------------------------------
# s5 resolution
# --------------------------------------------------------------------------

class Logs:
    """A logs set: publications, statuses, declarations, and accessions as
    dicts (see gro.py for the file form).  Publications carry emitter,
    blockNumber, logIndex, document (bytes), word (hex32); statuses carry
    emitter, blockNumber, logIndex, active; declarations carry emitter,
    blockNumber, logIndex, to; accessions carry emitter, blockNumber,
    logIndex, holder, root."""

    def __init__(self, publications, statuses, declarations=(), accessions=()):
        self.publications = list(publications)
        self.statuses = list(statuses)
        self.declarations = list(declarations)
        self.accessions = list(accessions)
        self._decoded = {}
        self._kinds = {}
        self._book = None

    def book(self):
        if self._book is None:
            self._book = Book(self.declarations, self.accessions)
        return self._book

    def pubs(self, a):
        """s5.1: publications of `a` ascending by (blockNumber, logIndex)."""
        ps = [p for p in self.publications if p["emitter"] == a]
        ps.sort(key=lambda p: (p["blockNumber"], p["logIndex"]))
        return ps

    def candidate_of(self, a, K):
        """s1: the last publication of `a` that is of kind K, or None."""
        last = None
        for p in self.pubs(a):
            key = (p["blockNumber"], p["logIndex"])
            if key not in self._kinds:
                self._kinds[key] = kind_string(p)
            k = self._kinds[key]
            if k is None or k.startswith(PREFIXES[K]):
                last = p
        return last

    def active(self, a):
        ss = [s for s in self.statuses if s["emitter"] == a]
        if not ss:
            return True
        return max(ss, key=lambda s: (s["blockNumber"], s["logIndex"]))["active"]

    def decoded(self, p):
        key = (p["blockNumber"], p["logIndex"])
        if key not in self._decoded:
            try:
                self._decoded[key] = decode(p)
            except Fault as f:
                self._decoded[key] = ("fault", f.token)
        return self._decoded[key]


class Book:
    """s4.2: the replay of the declarations and accessions of a logs set."""

    def __init__(self, declarations, accessions):
        recs = [("d", x) for x in declarations] + [("a", x) for x in accessions]
        recs.sort(key=lambda r: (r[1]["blockNumber"], r[1]["logIndex"]))
        self.root_of = {}       # address -> its root, where not itself
        self.head = {}          # root -> its head, where not the root
        self.pending = {}       # address -> the binding declaration pending at it
        self.entered = set()
        self.keys = {}          # root -> its keys, the root first, then each in the order it acceded
        self.taken = set()      # (blockNumber, logIndex) of the declarations an accession took up
        self.decls = []         # every declaration, in walk order
        for kind, x in recs:
            if kind == "d":
                self.decls.append(x)
                e, to = x["emitter"], x["to"]
                if self.current(e) == e and to != ZERO and to != e and to not in self.entered:
                    self.pending[e] = x
                    self.entered.add(e)
            else:
                e, h = x["emitter"], x["holder"]
                pd = self.pending.get(h)
                if e not in self.entered and pd is not None and pd["to"] == e \
                        and self.current(h) == h:
                    r = self.root(h)
                    self.root_of[e] = r
                    self.head[r] = e
                    self.keys.setdefault(r, [r]).append(e)
                    self.taken.add((pd["blockNumber"], pd["logIndex"]))
                    del self.pending[h]
                    self.entered.add(e)

    def root(self, a):
        return self.root_of.get(a, a)

    def current(self, a):
        r = self.root(a)
        return self.head.get(r, r)


def current_at(logs, a, position):
    """s5.2: the current key of a's record at `position`, a (blockNumber,
    logIndex) pair, the records at it and after it unread."""
    before = lambda x: (x["blockNumber"], x["logIndex"]) < position
    return Book([x for x in logs.declarations if before(x)],
                [x for x in logs.accessions if before(x)]).current(a)


def resolve_identity(logs, a):
    """s5.3.  Returns (final, trail, attempts)."""
    bk = logs.book()
    final = bk.current(a)
    keys = bk.keys.get(bk.root(a), [a])
    trail = keys[keys.index(a):keys.index(final)]
    record = set(trail) | {final}
    attempts = [{"blockNumber": x["blockNumber"], "logIndex": x["logIndex"], "to": x["to"]}
                for x in bk.decls
                if x["emitter"] in record and (x["blockNumber"], x["logIndex"]) not in bk.taken]
    return final, trail, attempts


def record_keys(logs, a):
    """s5.3: the keys of a's record, its root first, then each in the order it acceded."""
    bk = logs.book()
    r = bk.root(a)
    return list(bk.keys.get(r, [r]))


def identity_answer(logs, a):
    final, trail, attempts = resolve_identity(logs, a)
    return Obj([("attempts", [Obj([("blockNumber", t["blockNumber"]), ("logIndex", t["logIndex"]),
                                   ("to", t["to"])]) for t in attempts]),
                ("final", final), ("ok", True), ("trail", list(trail))])


def _bytewise_set(addrs):
    return sorted(set(addrs), key=lambda s: s.encode("utf-8"))


def covenant_of(logs, chain, G):
    """s5.9.  Returns None (no covenant), ("stop", token), or ("covenant", Obj)."""
    p = logs.candidate_of(G, "guildamanah")
    if p is None:
        return None
    what, x = logs.decoded(p)
    if what == "fault":
        return ("stop", x)
    if what != "guildamanah":
        # of this kind by prefix and not this version
        return ("stop", "E_UNKNOWN_KIND")
    if x["guild"] != G:
        return ("stop", "E_GUILDAMANAH_GUILD_MISMATCH")
    if x["chainId"] != chain:
        return ("stop", "E_GUILDAMANAH_CHAIN_MISMATCH")
    pairs = [("blockNumber", p["blockNumber"]), ("sha256", x["sha256"]), ("version", x["version"]),
             ("word", p["word"])]
    for k in ("venues", "quotes"):
        if k in x:
            pairs.append((k, x[k]))
    return ("covenant", Obj(pairs))


def covenant_answer(logs, chain, G):
    r = covenant_of(logs, chain, G)
    if r is None:
        return Obj([("covenant", None), ("ok", True)])
    if r[0] == "stop":
        return Obj([("ok", False), ("token", r[1])])
    return Obj([("covenant", r[1]), ("ok", True)])


def guild_card_of(logs, G):
    """s5.11.  Returns the answer Obj."""
    p = logs.candidate_of(G, "guildcard")
    if p is None:
        return Obj([("verdict", "CARD_NONE")])
    what, x = logs.decoded(p)
    if what == "fault":
        return Obj([("token", x), ("verdict", "CARD_UNREAD")])
    if what != "guildcard":
        return Obj([("token", "E_UNKNOWN_KIND"), ("verdict", "CARD_UNREAD")])
    if x["guild"] != G:
        return Obj([("verdict", "CARD_MISMATCH")])
    return Obj([("blockNumber", p["blockNumber"]), ("document", _card_view(x)),
                ("verdict", "CARD"), ("word", p["word"])])


def resolve_guild(logs, chain, g):
    """s5.4.  Returns the answer Obj."""
    G, trail, _ = resolve_identity(logs, g)
    if not logs.active(G):
        return Obj([("ok", False), ("token", "E_RETIRED")])
    p = logs.candidate_of(G, "roster")
    if p is None:
        return Obj([("ok", False), ("token", "E_NO_DOCUMENT")])
    what, x = logs.decoded(p)
    if what == "fault":
        return Obj([("ok", False), ("token", x)])
    if what != "roster":
        return Obj([("ok", False), ("token", "E_UNKNOWN_KIND")])
    R = x
    if R["guild"] != G:
        return Obj([("ok", False), ("token", "E_GUILD_MISMATCH")])
    if R["chainId"] != chain:
        return Obj([("ok", False), ("token", "E_CHAIN_MISMATCH")])
    rows = []
    for listed in R["judges"]:
        f, t, _ = resolve_identity(logs, listed)
        rows.append(Obj([("final", f), ("listed", listed), ("trail", list(t))]))
    bench = []
    protocols = []
    vrows = []
    if R["bench"] is not None:
        addrs = []
        for v in R["bench"]["verifiers"]:
            extra = []
            if "name" in v:
                extra = [("name", v["name"]), ("protocols", list(v["protocols"]))]
            f, t, _ = resolve_identity(logs, v["addr"])
            addrs.extend(record_keys(logs, v["addr"]))
            vrows.append(Obj([("final", f), ("listed", v["addr"])] + extra + [("trail", list(t))]))
        bench = _bytewise_set(addrs)
        protocols = list(R["bench"]["protocols"])
    cov = covenant_of(logs, chain, G)
    if cov is None:
        covenant = None
    elif cov[0] == "stop":
        covenant = Obj([("ok", False), ("token", cov[1])])
    else:
        covenant = cov[1]
    return Obj([("bench", bench), ("card", guild_card_of(logs, G)), ("covenant", covenant),
                ("guild", G), ("issued", R["issued"]), ("judges", rows), ("ok", True),
                ("protocols", protocols), ("trail", list(trail)),
                ("unknownFields", list(R["unknownFields"])), ("verifiers", vrows)])


def judge_of(logs, a):
    """s5.7.  Returns the answer Obj."""
    p = logs.candidate_of(a, "judge")
    if p is None:
        return Obj([("verdict", "JUDGE_NONE")])
    what, x = logs.decoded(p)
    if what == "fault":
        return Obj([("token", x), ("verdict", "JUDGE_UNREAD")])
    if what != "judge":
        return Obj([("token", "E_UNKNOWN_KIND"), ("verdict", "JUDGE_UNREAD")])
    if x["judge"] != a:
        return Obj([("verdict", "JUDGE_MISMATCH")])
    return Obj([("blockNumber", p["blockNumber"]), ("document", _judge_view(x)),
                ("verdict", "JUDGE"), ("word", p["word"])])


def resolve_judge(logs, chain, a):
    """s5.8.  Returns the answer Obj."""
    J, trail, _ = resolve_identity(logs, a)
    return Obj([("active", logs.active(J)), ("final", J), ("judge", judge_of(logs, J)),
                ("ok", True), ("trail", list(trail))])


# --------------------------------------------------------------------------
# s6 the pointer
# --------------------------------------------------------------------------

def parse_pointer(s):
    """s6.1: (chainId, address) or None."""
    if not isinstance(s, str) or not s.startswith(POINTER_PREFIX):
        return None
    rest = s[len(POINTER_PREFIX):]
    i = rest.find(":")
    if i < 0:
        return None
    num, addr = rest[:i], rest[i + 1:]
    if not num or any(c not in "0123456789" for c in num):
        return None
    if len(num) > 1 and num[0] == "0":
        return None
    if len(num) > 16 or int(num) > glaudit.MAX_INT:
        return None
    if not is_hex(addr, 20):
        return None
    return int(num), addr


# --------------------------------------------------------------------------
# s5.5 the bench, s5.6 candidates
# --------------------------------------------------------------------------

def bench_value(logs, chain, B):
    """s5.5.  Returns None, "unresolved", or a list."""
    if B is None:
        return None
    if isinstance(B, str):
        ptr = parse_pointer(B)
        if ptr is None or ptr[0] != chain:
            return "unresolved"
        ans = resolve_guild(logs, chain, ptr[1])
        d = ans.dict()
        if not d["ok"]:
            return "unresolved"
        return list(d["bench"])
    if isinstance(B, list):
        if not all(is_hex(e, 20) for e in B):
            return "unresolved"
        addrs = []
        for e in B:
            addrs.extend(record_keys(logs, e))
        return _bytewise_set(addrs)
    return "unresolved"


def candidates(logs, chain, pointers, allow, deny):
    """s5.6.  Returns the answer Obj."""
    per = []
    lst = []
    denyset = set(deny)
    for ptr in pointers:
        parsed = parse_pointer(ptr)
        if parsed is None:
            per.append(Obj([("ok", False), ("pointer", ptr), ("token", "E_POINTER")]))
            continue
        if parsed[0] != chain:
            per.append(Obj([("ok", False), ("pointer", ptr), ("token", "E_POINTER_CHAIN")]))
            continue
        ans = resolve_guild(logs, chain, parsed[1])
        d = ans.dict()
        per.append(Obj([("pointer", ptr)] + list(ans.pairs)))
        if not d["ok"]:
            continue
        for row in d["judges"]:
            rd = row.dict()
            if any(k in denyset for k in record_keys(logs, rd["listed"])):
                continue
            if rd["final"] not in lst:
                lst.append(rd["final"])
    for a in allow:
        if a not in lst:
            lst.append(a)
    return Obj([("candidates", lst), ("perPointer", per)])
