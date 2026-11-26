#!/usr/bin/env python3
"""gev: the courtevidence/1 criterion in pure Python 3, over the frozen
galeed/1 core in ../impl-py.  Command-line contract: HARNESS-EVIDENCE.md."""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "..", "impl-py"))

from glcanon import Obj, canon                                    # noqa: E402
from glcrypto import sha256                                       # noqa: E402
import gevlaw                                                     # noqa: E402

BOUND = 16 * 1024 * 1024


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
        die("gev: cannot read %s: %s" % (path, exc))
    if len(b) > BOUND:
        die("gev: %s is above the sixteen-mebibyte bound" % path)
    return b


def cmd_check(argv):
    if len(argv) != 1:
        die("usage: gev check <path>")
    emit(gevlaw.check_answer(read_file(argv[0])))


def cmd_digest(argv):
    if len(argv) != 1:
        die("usage: gev digest <path>")
    b = read_file(argv[0])
    emit(Obj([("bytes", len(b)), ("sha256", "0x" + sha256(b).hex())]))


def cmd_recipient(argv):
    if len(argv) != 1:
        die("usage: gev recipient-check <recipient>")
    if gevlaw.recipient_ok(argv[0]):
        emit(Obj([("ok", True)]))
    else:
        emit(Obj([("ok", False), ("token", "E_RECIPIENT")]))


def cmd_pick(argv):
    if len(argv) != 1:
        die("usage: gev pick-recipient <audit.json>")
    emit(gevlaw.pick(read_file(argv[0])))


COMMANDS = {
    "check": cmd_check,
    "digest": cmd_digest,
    "recipient-check": cmd_recipient,
    "pick-recipient": cmd_pick,
}


def main(argv):
    if len(argv) < 2 or argv[1] not in COMMANDS:
        die("usage: gev {check|digest|recipient-check|pick-recipient} ...")
    COMMANDS[argv[1]](argv[2:])
    return 0


if __name__ == "__main__":
    sys.setrecursionlimit(10000)
    sys.exit(main(sys.argv))
