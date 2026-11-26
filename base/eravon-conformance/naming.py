#!/usr/bin/env python3
"""The naming act for the three ERAVON document laws, in the order their
cores nest: eravon.terms/1 §10.5, then eravon.link/1 §11.4 and §11.5, then
eravon.dossier/1 §7.4 and §7.5. Each step computes the digest with the
law's own script and writes it into the text; the scripts carry their own
guards (the embedded upstream digest against the pinning section, the
printed digest against the naming section). Tooling: run once, on the
naming day.

    python3 eravon-conformance/naming.py <date> [--laws terms,link,dossier] <fuzz-note> ...

One fuzz note per named law, in the order named; each note is followed by
the record of the fuzzing run for the core named on 2026-10-19, bound to
that core's digest (CARRIED below). With --laws the act
starts at a lower law whose upstream is already named (a revision in
place of link alone names link and dossier); a pinning section already
quoting its upstream digest is left as it stands.
"""
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DOCS = os.path.join(HERE, "..", "docs")


# The fuzzing records of the namings of 2026-10-19, carried beside each new digest and bound to the core each
# measured (the draft of 2026-10-24, items terms-33 and link-45, trimmed as the draft trimmed them).
CARRIED = {
    "terms": ("The fuzzing of the deferral over the core named on 2026-10-19 "
              "(`0x5cb4a7f34c2a5b46b290614165c32a5ceb95b928b7f355ca3e467d0143b1945e`) was run across the five rounds of the "
              "revision of 2026-10-17 and the two of 2026-10-18 and 2026-10-19 by the three implementation seats' "
              "differential probes against that core: over §2.2's fences and bytes (49 fence shapes, 4,000 grammar-fuzzed "
              "texts and 1,320 byte mutations in round 1 by the Node seat, 810 fence cases and 1,200 byte mutations by the "
              "Python seat, and the structural and byte mutation of every document kind every seat repeated at fresh seeds "
              "in rounds 2 to 7) and over `dec256` (the amount at ⌊(2^256 − 1)/10000⌋ and beyond, leading zeros and every "
              "spelling, through every command that reads one), zero disagreements with that core; beside that digest "
              "stood the three seats' 1,634,961 differential inputs over the seven rounds across the three laws (Rust "
              "656,230, Node 580,430, Python 398,301)."),
    "link": ("The fuzzing of the deferral over the core named on 2026-10-19 "
             "(`0x29ff5e4651ee99cdc94521157ef96ad1e943473bea645cc24a569fb4429c80ef`) was run across the five rounds of the "
             "revision of 2026-10-17 and the two of 2026-10-18 and 2026-10-19: the two transports' prefix and base64url "
             "scans (`decode` and `receipt-decode` at every residue modulo 4 and at eight lengths with every last character, "
             "stray bytes at the end and in the middle, eight prefix spellings, the standard and the URL alphabets, padding "
             "and trailing bits, `encode` and `receipt-encode` over documents whose bytes carry both URL characters, by the "
             "Rust, Node and Python seats at seeds 1 to 6), zero disagreements with that core; the three seats' 1,634,961 "
             "differential inputs over the seven rounds are counted at `eravon.terms/1` §10.5."),
    "dossier": ("The fuzzing this law defers with the two below it was run, for the cores named on 2026-10-19 "
                "(`0x0a6b5b321985f2f22f4f66752695b86fe0e23b00260bacf4d2a654db6f92932e` here), across the five rounds of the "
                "revision of 2026-10-17 and the two of 2026-10-18 and 2026-10-19 over their scans, as their §10.5 and §11.5 "
                "record; this law's own surface, §1's file names and the one path the reader opens, the three seats probed "
                "at the host's limits (36 spellings of `<dir>`, the path-length limit straddled from 1,020 to 1,026 bytes "
                "under every spelling, link chains of 15 to 33 and the loop, a link whose target differs in case alone, "
                "entries that are devices), zero disagreements with that core."),
}


def digest(script):
    r = subprocess.run(["sh", os.path.join(HERE, script)], capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit("%s failed: %s" % (script, r.stderr))
    return r.stdout.strip().split("\n")[-1]


def edit(path, old, new, optional=False):
    """Whitespace-insensitive unique replacement (the laws are re-wrapped from time to time)."""
    s = open(path, encoding="utf-8").read()
    pat = re.compile(r"\s+".join(re.escape(tok) for tok in old.split()))
    ms = list(pat.finditer(s))
    if len(ms) == 0 and optional:
        print("naming: an optional anchor matched nothing in %s: %s" % (os.path.basename(path), old[:60]))
        return False
    assert len(ms) == 1, (path, old[:60], len(ms))
    m = ms[0]
    open(path, "w", encoding="utf-8").write(s[:m.start()] + new + s[m.end():])
    return True


def name_terms(date, fz):
    law = os.path.join(DOCS, "eravon-terms-v1.md")
    d = digest("criterion-digest-terms.sh")
    edit(law,
         "and it is the only act that performs it. No release digest is named yet; until one is, the reference implementation in `eravon-conformance/terms-py/` (`ert.py`, `ertlaw.py`, `ertcrypto.py` and `ertcanon.py`, which carry §10.4's implementation as that section names; the language's standard library, nothing else) is the candidate whose release digest `criterion-digest-terms.sh` computes.",
         "and it is the only act that performs it. The release digest of the\nreference implementation in `eravon-conformance/terms-py/` (`ert.py`,\n`ertlaw.py`, `ertcrypto.py` and `ertcanon.py`, which carry §10.4's\nimplementation as that section names; the language's standard library,\nnothing else),\nas `criterion-digest-terms.sh` computes it, is\n\n`%s`\n\nnamed on %s; the review record beside this text\n(docs/eravon-terms-v1-review.md) records how it converged. %s %s" % (d, date, fz, CARRIED["terms"]))
    edit(law,
         "on every input; until §10.5 names one, this text is the criterion. Evolution goes through `eravon.terms/2`.",
         "on every input. Evolution goes through `eravon.terms/2`.", optional=True)
    return d


def name_link(date, fz):
    law = os.path.join(DOCS, "eravon-link-v1.md")
    d_terms = digest("criterion-digest-terms.sh")
    edit(law,
         "This law's core embeds the `eravon.terms/1` core of the release digest quoted here in full once it exists, the one that law's §10.5 named when this section was pinned;",
         "This law's core embeds the `eravon.terms/1` core of release digest\n\n`%s`\n\n(the one that law's §10.5 named when this section was pinned);" % d_terms, optional=True)
    edit(law,
         "digest §11.4 names once `eravon.terms/1` §10.5 has named one, §11.4 quoting that digest in full, and agrees with the core §11.5 of this text names on every input; until each is named, this text and `eravon.terms/1` are the criterion. Evolution goes through `eravon.link/2`.",
         "digest §11.4 names and agrees with the core §11.5 of this text names on\nevery input. Evolution goes through `eravon.link/2`.", optional=True)
    d = digest("criterion-digest-link.sh")
    edit(law,
         "and it is the only act that performs it. No release digest is named yet; until one is, the reference implementation in `eravon-conformance/link-py/` (`erl.py`, `erllaw.py`; the language's standard library and the `eravon.terms/1` core, nothing else) is the candidate whose release digest `criterion-digest-link.sh` computes.",
         "and it is the only act that performs it. The release digest of the\nreference implementation in `eravon-conformance/link-py/` (`erl.py`,\n`erllaw.py`; the language's standard library and the `eravon.terms/1`\ncore, nothing else), as `criterion-digest-link.sh` computes it, is\n\n`%s`\n\nnamed on %s; the review record beside this text\n(docs/eravon-link-v1-review.md) records how it converged. %s %s" % (d, date, fz, CARRIED["link"]))
    return d


def name_dossier(date, fz):
    law = os.path.join(DOCS, "eravon-dossier-v1.md")
    d_link = digest("criterion-digest-link.sh")
    edit(law,
         "This law's core embeds the `eravon.link/1` core of the release digest quoted here in full once it exists, the one that law's §11.5 named when this section was pinned, and through it the `eravon.terms/1` core that law names;",
         "This law's core embeds the `eravon.link/1` core of release digest\n\n`%s`\n\n(the one that law's §11.5 named when this section was pinned), and\nthrough it the `eravon.terms/1` core that law names;" % d_link, optional=True)
    edit(law,
         "digest §7.4 names once `eravon.link/1` §11.5 has named one, §7.4 quoting that digest in full, and agrees with the core §7.5 of this text names on every input; until each is named, this text and the two laws below it are the criterion. Evolution goes through `eravon.dossier/2`.",
         "digest §7.4 names and agrees with the core §7.5 of this text names on\nevery input. Evolution goes through `eravon.dossier/2`.", optional=True)
    d = digest("criterion-digest-dossier.sh")
    edit(law,
         "and it is the only act that performs it. No release digest is named yet; until one is, the reference implementation in `eravon-conformance/dossier-py/` (`erd.py`, `erdlaw.py` and `erdentry.py`, the last carrying `galeed/1` §4.3 (§7.4); the language's standard library and the two cores, nothing else) is the candidate whose release digest `criterion-digest-dossier.sh` computes.",
         "and it is the only act that performs it. The release digest of the\nreference implementation in `eravon-conformance/dossier-py/` (`erd.py`,\n`erdlaw.py` and `erdentry.py`, the last carrying `galeed/1` §4.3 (§7.4);\nthe language's standard library and the two cores, nothing else), as\n`criterion-digest-dossier.sh` computes it, is\n\n`%s`\n\nnamed on %s; the review record beside this text\n(docs/eravon-dossier-v1-review.md) records how it converged. %s %s" % (d, date, fz, CARRIED["dossier"]))
    return d


NAMERS = {"terms": name_terms, "link": name_link, "dossier": name_dossier}


def main():
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    date = args.pop(0)
    laws = ["terms", "link", "dossier"]
    if args and args[0] == "--laws":
        args.pop(0)
        laws = args.pop(0).split(",")
    if len(args) != len(laws):
        sys.exit("one fuzz note per law named: %s" % ", ".join(laws))
    for law, fz in zip(laws, args):
        print(law, NAMERS[law](date, fz))


if __name__ == "__main__":
    main()
