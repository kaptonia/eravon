#!/usr/bin/env python3
"""ert: the eravon.terms/1 criterion in pure Python 3, its galeed/1 forms
and arithmetic carried in ertcanon and ertcrypto.  Command-line contract:
HARNESS-TERMS.md."""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from ertcanon import NoLabel, Obj, canon, is_hex, load_input     # noqa: E402
from ertcrypto import N, address_of, pubkey, sign_digest         # noqa: E402
import ertcrypto as C                                            # noqa: E402
import ertlaw as L                                               # noqa: E402
from ertlaw import DocFault, hx                                  # noqa: E402


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
        die("ert: cannot read %s: %s" % (path, exc))


def fault_obj(f):
    return f.obj()


def parse_int_arg(s):
    if not L.is_int_spelling(s):
        die("ert: an int of galeed/1 s3.2")
    return int(s)


def parse_hex20_arg(s):
    if not is_hex(s, 20):
        die("ert: a hex20")
    return s


def parse_privkey(key):
    if key.startswith("0x") or key.startswith("0X"):
        key = key[2:]
    if len(key) != 64 or any(c not in "0123456789abcdefABCDEF" for c in key):
        die("ert: bad private key")
    priv = int(key, 16)
    return priv


def parse_transport_hex(s):
    if len(s) < 2 or s[0] != "0" or s[1] != "x" or len(s) % 2:
        die("ert: transport hex")
    body = s[2:]
    if any(c not in "0123456789abcdefABCDEF" for c in body):
        die("ert: transport hex")
    return bytes.fromhex(body)


def load_json_value(path):
    raw = read_file(path)
    try:
        return load_input(raw)
    except NoLabel:
        die("ert: %s is not a readable value" % path)


# --------------------------------------------------------------------------

def cmd_check(argv):
    if len(argv) != 1:
        die("usage: ert check <terms.md>")
    b = read_file(argv[0])
    try:
        t = L.check_terms(b)
    except DocFault as f:
        emit(fault_obj(f))
        return
    emit(Obj([("ok", True), ("termsHash", hx(t.hash))]))


def cmd_quote(argv):
    if len(argv) != 1:
        die("usage: ert quote <hexbytes>")
    q = parse_transport_hex(argv[0])
    try:
        d = L.decode_quote(q)
    except DocFault as f:
        emit(fault_obj(f))
        return
    emit(Obj([("ok", True), ("quote", L.quote_answer(d)), ("quoteHash", hx(C.keccak256(q)))]))


def _terms_offer(path, c):
    """Read, check, derive; emit the fault and return None, or (t, offer)."""
    b = read_file(path)
    try:
        t = L.check_terms(b)
        o = L.offer_of(t, c)
    except DocFault as f:
        emit(fault_obj(f))
        return None
    return t, o


def cmd_offer(argv):
    if len(argv) != 2:
        die("usage: ert offer <terms.md> <c>")
    c = parse_int_arg(argv[1])
    r = _terms_offer(argv[0], c)
    if r is None:
        return
    t, o = r
    emit(L.offer_answer(o, t.d["chainId"], t.d["logic"]))


def _order_args(argv):
    """<buyerKey> <buyerPayout> <openBy> <prev> in that order; misuse otherwise."""
    bk = parse_hex20_arg(argv[0])
    bp = parse_hex20_arg(argv[1])
    ob = parse_int_arg(argv[2])
    pv = parse_hex20_arg(argv[3])
    return bk, bp, ob, pv


def cmd_order(argv):
    if len(argv) != 6:
        die("usage: ert order <terms.md> <c> <buyerKey> <buyerPayout> <openBy> <prev>")
    c = parse_int_arg(argv[1])
    bk, bp, ob, pv = _order_args(argv[2:6])
    r = _terms_offer(argv[0], c)
    if r is None:
        return
    t, o = r
    try:
        od = L.order_of(o, t.d["chainId"], t.d["logic"], bk, bp, ob, pv)
    except DocFault as f:
        emit(fault_obj(f))
        return
    emit(L.order_answer(od))


def _sig_hex(priv, digest32):
    r, s, v = sign_digest(priv, digest32)
    return hx(r.to_bytes(32, "big") + s.to_bytes(32, "big") + bytes([v]))


def cmd_sign_offer(argv):
    if len(argv) != 3:
        die("usage: ert sign-offer <privkey> <terms.md> <c>")
    priv = parse_privkey(argv[0])
    if not (1 <= priv <= N - 1):
        die("ert: private key out of range")
    c = parse_int_arg(argv[2])
    r = _terms_offer(argv[1], c)
    if r is None:
        return
    t, o = r
    dg = L.offer_digest(t.d["chainId"], t.d["logic"], o)
    emit(Obj([("digest", hx(dg)), ("ok", True), ("sig", _sig_hex(priv, dg)), ("signer", hx(address_of(pubkey(priv))))]))


def cmd_sign_order(argv):
    if len(argv) != 6:
        die("usage: ert sign-order <privkey> <terms.md> <c> <buyerPayout> <openBy> <prev>")
    priv = parse_privkey(argv[0])
    if not (1 <= priv <= N - 1):
        die("ert: private key out of range")
    c = parse_int_arg(argv[2])
    bp = parse_hex20_arg(argv[3])
    ob = parse_int_arg(argv[4])
    pv = parse_hex20_arg(argv[5])
    r = _terms_offer(argv[1], c)
    if r is None:
        return
    t, o = r
    bk = hx(address_of(pubkey(priv)))
    try:
        od = L.order_of(o, t.d["chainId"], t.d["logic"], bk, bp, ob, pv)
    except DocFault as f:
        emit(fault_obj(f))
        return
    emit(Obj([("buyerKey", bk), ("digest", hx(od.digest)), ("instance", hx(od.instance)), ("ok", True),
              ("sig", _sig_hex(priv, od.digest))]))


# --------------------------------------------------------------------------
# s4.5 actions

def _parse_action(argv):
    """argv begins with <name> and its arguments; returns (name, args, rest)."""
    if not argv or argv[0] not in L.ACTIONS:
        die("ert: an action is one of " + ", ".join(L.ACTIONS))
    name = argv[0]
    n = L.ACTION_ARGS[name]
    if len(argv) < 1 + n:
        die("ert: %s takes %d argument(s)" % (name, n))
    raw = argv[1:1 + n]
    if name == "ClaimDelivery":
        args = [raw[0]]
    elif name == "Settle":
        args = [parse_int_arg(raw[0]), parse_int_arg(raw[1])]
    else:
        args = []
    return name, args, argv[1 + n:]


def _order_and_action(path, c, bk, bp, ob, pv, name, args):
    """Derive the order and the action digest; emit the fault and return None."""
    r = _terms_offer(path, c)
    if r is None:
        return None
    t, o = r
    try:
        od = L.order_of(o, t.d["chainId"], t.d["logic"], bk, bp, ob, pv)
        sh = L.action_struct_hash(name, args)
    except DocFault as f:
        emit(fault_obj(f))
        return None
    return t, o, od, sh, L.action_digest(t.d["chainId"], od.instance, sh)


def cmd_action(argv):
    if len(argv) < 7:
        die("usage: ert action <terms.md> <c> <buyerKey> <buyerPayout> <openBy> <prev> <name> [<arg>...]")
    c = parse_int_arg(argv[1])
    bk, bp, ob, pv = _order_args(argv[2:6])
    name, args, rest = _parse_action(argv[6:])
    if rest:
        die("ert: an argument beyond the list")
    r = _order_and_action(argv[0], c, bk, bp, ob, pv, name, args)
    if r is None:
        return
    t, o, od, sh, dg = r
    emit(Obj([("digest", hx(dg)), ("instance", hx(od.instance)), ("ok", True), ("structHash", hx(sh))]))


def cmd_sign_action(argv):
    if len(argv) < 8:
        die("usage: ert sign-action <privkey> <terms.md> <c> <buyerKey> <buyerPayout> <openBy> <prev> <name> [<arg>...]")
    priv = parse_privkey(argv[0])
    if not (1 <= priv <= N - 1):
        die("ert: private key out of range")
    c = parse_int_arg(argv[2])
    bk, bp, ob, pv = _order_args(argv[3:7])
    name, args, rest = _parse_action(argv[7:])
    if rest:
        die("ert: an argument beyond the list")
    r = _order_and_action(argv[1], c, bk, bp, ob, pv, name, args)
    if r is None:
        return
    t, o, od, sh, dg = r
    emit(Obj([("digest", hx(dg)), ("instance", hx(od.instance)), ("ok", True), ("sig", _sig_hex(priv, dg)),
              ("signer", hx(address_of(pubkey(priv))))]))


def cmd_verify_action(argv):
    if len(argv) < 9:
        die("usage: ert verify-action <terms.md> <c> <buyerKey> <buyerPayout> <openBy> <prev> <name> [<arg>...] <party> <sig.json>")
    c = parse_int_arg(argv[1])
    bk, bp, ob, pv = _order_args(argv[2:6])
    name, args, rest = _parse_action(argv[6:])
    if len(rest) != 2:
        die("ert: <party> <sig.json> follow the action")
    party = rest[0]
    if party not in ("buyer", "seller"):
        die("ert: party is buyer or seller")
    sig = load_json_value(rest[1])
    r = _order_and_action(argv[0], c, bk, bp, ob, pv, name, args)
    if r is None:
        return
    t, o, od, sh, dg = r
    try:
        form = L.verify_action_sig(name, party, dg, sig, t.d["sellerKey"], bk)
    except DocFault as f:
        emit(fault_obj(f))
        return
    emit(Obj([("form", form), ("ok", True)]))


def cmd_verify_offer(argv):
    if len(argv) != 3:
        die("usage: ert verify-offer <terms.md> <c> <sig>")
    c = parse_int_arg(argv[1])
    r = _terms_offer(argv[0], c)
    if r is None:
        return
    t, o = r
    dg = L.offer_digest(t.d["chainId"], t.d["logic"], o)
    try:
        form, signer = L.verify_offer_sig(dg, argv[2], t.d["sellerKey"])
    except DocFault as f:
        emit(fault_obj(f))
        return
    if form == "call":
        emit(Obj([("form", form), ("ok", True)]))
        return
    emit(Obj([("form", form), ("ok", True), ("signer", signer)]))


def cmd_verify_order(argv):
    if len(argv) != 7:
        die("usage: ert verify-order <terms.md> <c> <buyerKey> <buyerPayout> <openBy> <prev> <sig.json>")
    c = parse_int_arg(argv[1])
    bk, bp, ob, pv = _order_args(argv[2:6])
    sig = load_json_value(argv[6])
    r = _terms_offer(argv[0], c)
    if r is None:
        return
    t, o = r
    try:
        od = L.order_of(o, t.d["chainId"], t.d["logic"], bk, bp, ob, pv)
        form = L.verify_order_sig(od.digest, sig, bk)
    except DocFault as f:
        emit(fault_obj(f))
        return
    emit(Obj([("form", form), ("instance", hx(od.instance)), ("ok", True)]))


def cmd_fee(argv):
    if len(argv) != 2:
        die("usage: ert fee <quote-hexbytes> <amount>")
    q = parse_transport_hex(argv[0])
    if not L.is_dec256(argv[1]):
        die("ert: amount a dec256")
    amount = int(argv[1])
    try:
        d = L.decode_quote(q)
        premium, priced, lf = L.fee_of(d, amount)
    except DocFault as f:
        emit(fault_obj(f))
        return
    emit(Obj([("lf", None if lf is None else str(lf)), ("ok", True), ("premium", str(premium)),
              ("priced", priced)]))


COMMANDS = {
    "check": cmd_check, "quote": cmd_quote, "offer": cmd_offer, "order": cmd_order,
    "sign-offer": cmd_sign_offer, "sign-order": cmd_sign_order,
    "verify-offer": cmd_verify_offer, "verify-order": cmd_verify_order, "fee": cmd_fee,
    "action": cmd_action, "sign-action": cmd_sign_action,
    "verify-action": cmd_verify_action,
}


def main(argv):
    if len(argv) < 2 or argv[1] not in COMMANDS:
        die("usage: ert {%s} ..." % "|".join(COMMANDS))
    COMMANDS[argv[1]](argv[2:])
    return 0


if __name__ == "__main__":
    sys.setrecursionlimit(10000)
    sys.exit(main(sys.argv))
