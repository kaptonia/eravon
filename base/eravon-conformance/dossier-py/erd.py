#!/usr/bin/env python3
"""erd: the eravon.dossier/1 criterion in pure Python 3, over the
eravon.link/1 and eravon.terms/1 cores beside it and its own galeed/1 s4.3.
Command-line contract: HARNESS-DOSSIER.md."""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "..", "link-py"))
sys.path.insert(0, os.path.join(HERE, "..", "terms-py"))

from ertcanon import canon          # noqa: E402
from ertlaw import DocFault         # noqa: E402
import erdlaw as L                  # noqa: E402


def emit(v):
    sys.stdout.buffer.write(canon(v))
    sys.stdout.buffer.flush()


def die(msg):
    sys.stderr.write(msg + "\n")
    sys.exit(2)


def cmd_check(argv):
    if len(argv) != 1:
        die("usage: erd check <dir>")
    directory = argv[0]
    if not os.path.isdir(directory):
        die("erd: %s is not a directory" % directory)

    # s1: a file the index names is the directory entry byte-equal to it; the
    # host's own name folding never reaches an answer
    try:
        entries = set(os.listdir(os.fsencode(directory)))
    except OSError:
        die("erd: %s cannot be listed" % directory)

    def read(name):
        if name.encode("utf-8") not in entries:
            return None
        path = os.path.join(directory, name)
        if not os.path.isfile(path):
            return None
        try:
            with open(path, "rb") as fh:
                return fh.read()
        except OSError:
            return None

    try:
        ans = L.check_dossier(directory, read)
    except DocFault as f:
        emit(f.obj())
        return
    emit(ans)


COMMANDS = {"check": cmd_check}


def main(argv):
    if len(argv) < 2 or argv[1] not in COMMANDS:
        die("usage: erd {%s} ..." % "|".join(COMMANDS))
    COMMANDS[argv[1]](argv[2:])
    return 0


if __name__ == "__main__":
    sys.setrecursionlimit(10000)
    sys.exit(main(sys.argv))
