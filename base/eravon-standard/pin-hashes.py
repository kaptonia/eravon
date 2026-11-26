#!/usr/bin/env python3
"""Tooling: writes the panel book's and the verifier's initcode and runtime
code hashes, from the pinned compile, into src/Eravon.sol's pinned block
(standard Shape, s13), so that the logic's initcode embeds neither contract.
Run inside eravon/standard after any change to EravonVerifier.sol,
EravonPanelBook.sol, IGaleedVenue.sol or interfaces.sol. identity.sh
recomputes the four values, and every test that opens a case proves them:
the logic derives the book's and the verifier's addresses from the two
initcode hashes and refuses a book or a verifier whose code hash differs."""
import json, os, re, subprocess, sys
HERE = os.path.dirname(os.path.abspath(__file__))
subprocess.run(["forge", "build", "--silent", "--skip", "test", "--skip", "script"], cwd=HERE, check=True, env=dict(os.environ, FOUNDRY_TEST="/nonexistent"))
def keccak(hexstr):
    return subprocess.run(["cast", "keccak", hexstr], capture_output=True, text=True, check=True).stdout.strip()
def art(name):
    return json.load(open(os.path.join(HERE, "out", name + ".sol", name + ".json")))
vals = {}
for key, name in (("PANEL", "EravonPanelBook"), ("VERIFIER", "EravonVerifier")):
    a = art(name)
    vals[key + "_INITCODE_HASH"] = keccak(a["bytecode"]["object"])
    vals[key + "_CODEHASH"] = keccak(a["deployedBytecode"]["object"])
p = os.path.join(HERE, "src", "Eravon.sol")
s = open(p).read()
for k, v in vals.items():
    s, n = re.subn(r"(bytes32 internal constant %s = )0x[0-9a-fA-F]{64};" % k, r"\g<1>%s;" % v, s)
    assert n == 1, k
open(p, "w").write(s)
for k, v in vals.items():
    print(k, v)
