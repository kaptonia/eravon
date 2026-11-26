#!/usr/bin/env python3
"""gl1: a galeed/1 implementation in pure Python 3.

Command-line contract per the convergence harness:
    gl1 check <path>
    gl1 canon <path>
    gl1 sign  <privkey-hex> <path> [domain]
    gl1 audit <path>
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from glcanon import Fault, Obj, canon, parse15                    # noqa: E402
from glcrypto import (address_of, eip191_digest, message, pubkey,  # noqa: E402
                      sha256, sign_digest)
from glentry import DOMAIN_ENTRY, validate                        # noqa: E402
import glaudit                                                    # noqa: E402


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
        die("gl1: cannot read %s: %s" % (path, exc))


def cmd_check(argv):
    if len(argv) != 1:
        die("usage: gl1 check <path>")
    b = read_file(argv[0])
    try:
        validate(b)
    except Fault as f:
        emit(Obj([("ok", False), ("token", f.token)]))
        return
    emit(Obj([("entry_id", "0x" + sha256(b).hex()), ("ok", True)]))


def cmd_canon(argv):
    if len(argv) != 1:
        die("usage: gl1 canon <path>")
    b = read_file(argv[0])
    try:
        v = parse15(b)
    except Fault as f:
        emit(Obj([("ok", False), ("token", f.token)]))
        return
    emit(Obj([("canon", "0x" + canon(v).hex()), ("ok", True)]))


def cmd_sign(argv):
    if len(argv) not in (2, 3):
        die("usage: gl1 sign <privkey-hex> <path> [domain]")
    # HARNESS: sixty-four hexadecimal digits of either case, with or without
    # a leading 0x, naming a scalar in [1, n - 1]; anything else is misuse
    key = argv[0]
    if key.startswith("0x") or key.startswith("0X"):
        key = key[2:]
    if len(key) != 64 or any(c not in "0123456789abcdefABCDEF" for c in key):
        die("gl1: bad private key")
    priv = int(key, 16)
    from glcrypto import N
    if not (1 <= priv <= N - 1):
        die("gl1: private key out of range")
    domain = argv[2] if len(argv) == 3 else DOMAIN_ENTRY
    # HARNESS: a present literal is signed as given, the empty one included,
    # where it is ASCII and carries no line feed (s5.6); anything else is misuse
    if not domain.isascii() or "\n" in domain:
        die("gl1: a domain literal is ASCII and carries no line feed")
    b = read_file(argv[1])
    try:
        v = parse15(b)
    except Fault as f:
        die("gl1: preimage file is not a readable value (%s)" % f.token)
    if not isinstance(v, Obj):
        die("gl1: preimage is not an object")
    pre = sha256(canon(v))
    digest = eip191_digest(message(domain, pre))
    r, s, vv = sign_digest(priv, digest)
    sig = r.to_bytes(32, "big") + s.to_bytes(32, "big") + bytes([vv])
    emit(Obj([
        ("digest", "0x" + digest.hex()),
        ("presig", "0x" + pre.hex()),
        ("sig", "0x" + sig.hex()),
        ("signer", "0x" + address_of(pubkey(priv)).hex()),
    ]))


def cmd_audit(argv):
    if len(argv) != 1:
        die("usage: gl1 audit <path>")
    raw = read_file(argv[0])
    try:
        rep = glaudit.audit(raw)
    except glaudit.NoLabel:
        emit(Obj([("ok", False), ("reason", "NO_LABEL")]))
        return
    emit(rep)


def main(argv):
    if len(argv) < 2:
        die("usage: gl1 {check|canon|sign|audit} ...")
    cmd = argv[1]
    rest = argv[2:]
    if cmd == "check":
        cmd_check(rest)
    elif cmd == "canon":
        cmd_canon(rest)
    elif cmd == "sign":
        cmd_sign(rest)
    elif cmd == "audit":
        cmd_audit(rest)
    else:
        die("gl1: unknown command %r" % cmd)
    return 0


if __name__ == "__main__":
    sys.setrecursionlimit(10000)
    sys.exit(main(sys.argv))
