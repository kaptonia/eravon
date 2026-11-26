#!/usr/bin/env python3
"""Produce the chain-facing fixtures that galeed-core/fixtures holds: the
form of galeed/1 s9.1 recorded from an anvil node against the GaleedRegistry
and the rogue contracts, hand-written recordings for boundaries no node
produces (an anchor in block 0, a legacy signature naming no chain, a
signature-less transaction, a log inside a creation, two windows of one
chain, class items, the data refusals), and the basis forms of s9.4 that need
no recording.

    scenarios.py            writes each fixture whose file is absent
    scenarios.py hand       the hand-written and basis fixtures only (no anvil)
    GL_REGEN_FIXTURES=1 ... rewrites them all

Needs anvil, forge and cast on PATH. Every recorded scenario asserts the
facts its fragment must show before the fixture is written; the fragment
itself is the scanner's output, and the written fixture is replayed once
more through scan_replay.py. A fixture whose `expected` is null records a
scan that fails (exit 2), and the replay must fail on it."""

import hashlib
import json
import os
import socket
import subprocess
import sys
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import scan_replay as S  # noqa: E402
import record as R  # noqa: E402

FIX = os.path.normpath(os.path.join(HERE, "..", "..", "galeed-core", "fixtures"))
ROGUE = os.path.join(HERE, "rogue")
REGISTRY_ROOT = os.path.normpath(os.path.join(HERE, "..", "..", "galeed-core", "contracts"))
REGEN = os.environ.get("GL_REGEN_FIXTURES") == "1"
CHAIN = 31337
CHAIN2 = 7777
NO_LABEL = '{"ok":false,"reason":"NO_LABEL"}'

# anvil's deterministic accounts: 0 relays, 2 is the audited sender, 3 a stranger
RELAYER_KEY = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
SENDER_KEY = "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"
STRANGER_KEY = "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6"
SENDER = "0x3c44cdddb6a900fa2b585dd299e03d12fa4293bc"
STRANGER = "0x90f79bf6eb2c4f870365e785982e1f101e93b906"
RELAYER = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266"
E1 = b'{"body":{"statement_md":"the seat of one judge"},"entryType":"genesis","judge":"0x3c44cdddb6a900fa2b585dd299e03d12fa4293bc","prev":null,"seq":0,"sig":"0x00","spec":"galeed/1"}'
E2 = b'{"an entry":"two"}'
E3 = b'{"an entry":"three"}'


def sha(b):
    return "0x" + hashlib.sha256(b).hexdigest()


H1, H2, H3 = sha(E1), sha(E2), sha(E3)


def free_port():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    p = s.getsockname()[1]
    s.close()
    return p


def rpc(url, method, params):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    req = urllib.request.Request(url, data=body, headers={"content-type": "application/json"})
    with urllib.request.urlopen(req, timeout=30) as resp:
        r = json.loads(resp.read().decode())
    if r.get("error"):
        raise RuntimeError("%s: %s" % (method, r["error"]))
    return r["result"]


def sh(args, check=True):
    p = subprocess.run(args, capture_output=True, text=True)
    if check and p.returncode != 0:
        raise RuntimeError("%s\n%s\n%s" % (" ".join(args), p.stdout, p.stderr))
    return p.stdout


def wait_receipt(url, tx):
    for _ in range(100):
        r = rpc(url, "eth_getTransactionReceipt", [tx])
        if r is not None:
            return r
        time.sleep(0.1)
    raise RuntimeError("no receipt for %s" % tx)


def deploy(url, key, name, root=ROGUE, src="src/Rogue.sol"):
    out = sh(["forge", "create", "--root", root, "%s:%s" % (src, name), "--rpc-url", url,
              "--private-key", key, "--broadcast", "--json"])
    return json.loads(out.strip())["deployedTo"].lower()


def send(url, key, to, sig=None, args=(), data=None, auth=None, gas=None, wait=True):
    cmd = ["cast", "send", "--rpc-url", url, "--private-key", key, "--async", to]
    if sig is not None:
        cmd += [sig] + list(args)
    elif data is not None:
        cmd += [data]
    if auth is not None:
        cmd += ["--auth", auth]
    if gas is not None:
        cmd += ["--gas-limit", str(gas)]
    tx = sh(cmd).strip().splitlines()[-1].strip()
    if not wait:
        return tx.lower(), None
    rcpt = wait_receipt(url, tx)
    return tx.lower(), rcpt


def write_fixture(name, fixture, note):
    path = os.path.join(FIX, name + ".json")
    if os.path.exists(path) and not REGEN:
        print("  kept   ", name)
        return
    ok, got = S.try_run(fixture)
    if fixture["expected"] is None:
        if ok:
            raise RuntimeError("%s: the fixture expects a failing scan and the replay answered" % name)
    elif not ok or S.canon(got).decode("utf-8") != fixture["expected"]:
        raise RuntimeError("%s: the fixture does not replay to its own fragment" % name)
    with open(path, "w") as fh:
        json.dump(fixture, fh, indent=2, sort_keys=True)
        fh.write("\n")
    with open(os.path.join(FIX, name + ".note"), "w") as fh:
        fh.write(note.rstrip() + "\n")
    print("  wrote  ", name)


def anchors_of(fixture):
    return json.loads(fixture["expected"])["anchors"]


def chain_basis(lo, hi, registries, senders, cid=CHAIN):
    return {"chains": [{"chainId": cid, "fromBlock": lo, "toBlock": hi,
                        "registries": registries, "senders": sorted(senders)}]}


def rec(url, basis, urls=None):
    return R.record({"basis": basis, "rpc": urls or {str(CHAIN): url}})


def blk(rcpt):
    return S.qty(rcpt["blockNumber"])


def hexs(b):
    return "0x" + b.hex()


# ------------------------------------------------------------ recorded scenarios

def recorded(url, url2):
    registry = deploy(url, RELAYER_KEY, "GaleedRegistry", root=REGISTRY_ROOT, src="src/GaleedRegistry.sol")
    registry_b = deploy(url, RELAYER_KEY, "GaleedRegistry", root=REGISTRY_ROOT, src="src/GaleedRegistry.sol")
    rogue = deploy(url, RELAYER_KEY, "Rogue")
    forwarder = deploy(url, RELAYER_KEY, "Forwarder")
    reverter = deploy(url, RELAYER_KEY, "Reverter")

    # 01: the form, one record carrying its entry
    tx, r = send(url, SENDER_KEY, registry, "anchorCarried(bytes32,bytes)", [H1, hexs(E1)])
    assert S.qty(r["status"]) == 1 and len(r["logs"]) == 1
    b1 = blk(r)
    fx = rec(url, chain_basis(b1, b1, [registry], [SENDER]))
    assert anchors_of(fx) == [{"blockNumber": b1, "blockTimestamp": anchors_of(fx)[0]["blockTimestamp"], "chainId": CHAIN,
                               "entry": hexs(E1), "hash": H1, "sender": SENDER, "tx": tx, "verdict": "counted"}]
    write_fixture("01_carried_anchor", fx,
                  "9.1, 9.2, 9.3: the form. A declared registry emits Anchored(sender, hash, entry) in a status-1 transaction from a sender the basis declares; topic 2 sits in the calldata at offset 4 and the data is the ABI encoding of bytes whose sha256 is topic 2. One record, verdict counted, carrying the entry.")
    fx01 = fx

    # 02: anchorCarriedMany with three entries: three records sharing one transaction
    tx, r = send(url, SENDER_KEY, registry, "anchorCarriedMany(bytes32[],bytes[])",
                 ["[%s,%s,%s]" % (H1, H2, H3), "[%s,%s,%s]" % (hexs(E1), hexs(E2), hexs(E3))])
    assert S.qty(r["status"]) == 1 and len(r["logs"]) == 3
    b = blk(r)
    fx = rec(url, chain_basis(b, b, [registry], [SENDER]))
    a = anchors_of(fx)
    assert sorted(x["hash"] for x in a) == sorted([H1, H2, H3]) and all("entry" in x for x in a) and len(set(x["tx"] for x in a)) == 1
    write_fixture("02_anchor_many_three", fx,
                  "9.1, 9.4: anchorCarriedMany with three entries emits one log per element, each hash at 4 plus a multiple of 32 of the calldata; three records sharing one transaction, each carrying its entry.")

    # 03, 04: a stranger's anchor, declared and not declared among the senders
    tx, r = send(url, STRANGER_KEY, registry, "anchorCarried(bytes32,bytes)", [H2, hexs(E2)])
    b = blk(r)
    fx = rec(url, chain_basis(b, b, [registry], [STRANGER]))
    a = anchors_of(fx)
    assert len(a) == 1 and a[0]["sender"] == STRANGER and a[0]["entry"] == hexs(E2)
    write_fixture("03_foreign_sender_declared", fx,
                  "9.4: a sender outside the audited lineage whose address the basis names. The scan records it with its entry; section 8.1 is what later discards it and lists it with its bytes.")
    fx = rec(url, chain_basis(b, b, [registry], [SENDER]))
    assert anchors_of(fx) == []
    write_fixture("04_foreign_sender_absent", fx,
                  "9.4: the same chain under a basis whose senders omit that address. No record, and the basis carries that silence.")

    # 05: the registry refuses bytes that do not hash to the word; status 0, no log
    tx, r = send(url, SENDER_KEY, registry, "anchorCarried(bytes32,bytes)", [hexs(S.keccak256(E1)), hexs(E1)], gas=200000)
    assert S.qty(r["status"]) == 0 and len(r["logs"]) == 0
    b = blk(r)
    fx = rec(url, chain_basis(b, b, [registry], [SENDER]))
    assert anchors_of(fx) == []
    write_fixture("05_reverted_mismatch", fx,
                  "9.1: the reference registry refuses a call whose entry does not hash (sha256) to its hash; the transaction reverts, status 0, no log, no record.")

    # 06: two identical registries, one declared
    tx, r = send(url, SENDER_KEY, registry_b, "anchorCarried(bytes32,bytes)", [H3, hexs(E3)])
    b = blk(r)
    tx2, r2 = send(url, SENDER_KEY, registry, "anchorCarried(bytes32,bytes)", [H3, hexs(E3)])
    b2 = blk(r2)
    fx = rec(url, chain_basis(b, b2, [registry], [SENDER]))
    a = anchors_of(fx)
    assert len(a) == 1 and a[0]["tx"] == tx2
    write_fixture("06_undeclared_emitter", fx,
                  "9.1: two registries of one code, one declared. The undeclared contract's well-formed log is never read; the declared one's is a record.")

    # 07: a hash derived from the argument, nowhere in the calldata
    X = "0x" + "ab" * 32
    tx, r = send(url, SENDER_KEY, rogue, "derived(bytes32,bytes)", [X, hexs(E1)])
    assert S.qty(r["status"]) == 1 and len(r["logs"]) == 1
    b = blk(r)
    fx = rec(url, chain_basis(b, b, [rogue], [SENDER]))
    assert anchors_of(fx) == []
    write_fixture("07_derived_topic_not_in_calldata", fx,
                  "9.1: a declared contract emits the form with topic 2 derived from its argument (keccak256 of the encoded argument), which appears nowhere in the calldata at a word boundary; no anchor, whatever the data carries.")

    # 08: a delegated sender (EIP-7702 designator in both states): VOID
    auth = sh(["cast", "wallet", "sign-auth", reverter, "--private-key", SENDER_KEY, "--rpc-url", url]).strip().splitlines()[-1].strip()
    _t0, r0 = send(url, RELAYER_KEY, RELAYER, auth=auth, gas=100000)
    assert S.qty(r0["status"]) == 1
    code = rpc(url, "eth_getCode", [SENDER, "latest"])
    assert code.lower().startswith("0xef0100"), code
    tx, r = send(url, SENDER_KEY, registry, "anchorCarried(bytes32,bytes)", [H2, hexs(E2)], gas=100000)
    assert S.qty(r["status"]) == 1 and len(r["logs"]) == 1
    b = blk(r)
    fx = rec(url, chain_basis(b, b, [registry], [SENDER]))
    a = anchors_of(fx)
    assert len(a) == 1 and a[0]["verdict"] == "VOID" and a[0]["entry"] == hexs(E2)
    write_fixture("08_delegation_void", fx,
                  "9.3, 5.7: the sender installed an EIP-7702 delegation designator before anchoring. The log is the form and the record carries the entry, but the sending address has code at both boundaries: verdict VOID, and 8.6 makes the bytes no ledger entry.")

    # 09: the designator cleared in earlier blocks: counted
    clear = sh(["cast", "wallet", "sign-auth", "0x0000000000000000000000000000000000000000", "--private-key", SENDER_KEY, "--rpc-url", url]).strip().splitlines()[-1].strip()
    _t1, r1 = send(url, RELAYER_KEY, RELAYER, auth=clear, gas=100000)
    assert S.qty(r1["status"]) == 1
    assert rpc(url, "eth_getCode", [SENDER, "latest"]) in ("0x", "0x0", None)
    rpc(url, "anvil_mine", ["0x1"])
    tx, r = send(url, SENDER_KEY, registry, "anchorCarried(bytes32,bytes)", [H3, hexs(E3)])
    b = blk(r)
    fx = rec(url, chain_basis(b, b, [registry], [SENDER]))
    a = anchors_of(fx)
    assert len(a) == 1 and a[0]["verdict"] == "counted"
    write_fixture("09_delegation_revoked_counted", fx,
                  "9.3: the designator installed and cleared (an authorization naming the zero address, the one authorization 5.7 allows) in earlier blocks; both boundaries codeless, verdict counted.")

    # 10: codeless before, the designator installed inside the anchor's own block by the
    # anchoring transaction itself (a type-4 transaction carrying the sender's own authorization)
    nonce = S.qty(rpc(url, "eth_getTransactionCount", [SENDER, "pending"]))
    auth2 = sh(["cast", "wallet", "sign-auth", reverter, "--private-key", SENDER_KEY, "--rpc-url", url,
                "--nonce", str(nonce + 1)]).strip().splitlines()[-1].strip()
    txa, ra = send(url, SENDER_KEY, registry, "anchorCarried(bytes32,bytes)", [H1, hexs(E1)], auth=auth2, gas=200000)
    assert S.qty(ra["status"]) == 1 and len(ra["logs"]) == 1
    assert rpc(url, "eth_getCode", [SENDER, "latest"]).lower().startswith("0xef0100")
    b = blk(ra)
    fx = rec(url, chain_basis(b, b, [registry], [SENDER]))
    a = anchors_of(fx)
    assert len(a) == 1 and a[0]["verdict"] == "VOID", a
    write_fixture("10_delegation_in_anchor_block", fx,
                  "9.3, 5.7: codeless at the end of the block before, the designator installed inside the anchor's own block by the anchoring transaction itself (a type-4 transaction carrying the sender's own authorization). Both states consulted, the second shows code: VOID; a transaction that installs a designator on its own sender carries no anchor that counts, its record being VOID.")
    # clear again for the scenarios below
    clear2 = sh(["cast", "wallet", "sign-auth", "0x0000000000000000000000000000000000000000", "--private-key", SENDER_KEY, "--rpc-url", url]).strip().splitlines()[-1].strip()
    _t2, r2 = send(url, RELAYER_KEY, RELAYER, auth=clear2, gas=100000)
    assert S.qty(r2["status"]) == 1

    # 11: the node does not serve the state: UNPROVEN (fixture 01's recording with eth_getCode refused)
    fx = json.loads(json.dumps(fx01))
    for ex in fx["rpc"][str(CHAIN)]:
        if ex["method"] == "eth_getCode":
            ex.pop("result", None)
            ex["error"] = {"code": -32000, "message": "missing trie node: state not available"}
    fx["expected"] = S.canon(S.run(fx)).decode("utf-8")
    a = anchors_of(fx)
    assert len(a) == 1 and a[0]["verdict"] == "UNPROVEN" and a[0]["entry"] == hexs(E1)
    write_fixture("11_state_not_served_unproven", fx,
                  "9.3: the endpoint refuses eth_getCode for the sender's boundaries. The record exists and carries its entry, the verdict is UNPROVEN, and the recording carries the node's error verbatim; under a root equal to the sender, 8.7 labels the audit UNPROVEN and 8.6 makes the bytes no ledger entry.")

    # 12: one hash anchored on two chains
    registry2 = deploy(url2, RELAYER_KEY, "GaleedRegistry", root=REGISTRY_ROOT, src="src/GaleedRegistry.sol")
    txc, rc = send(url2, SENDER_KEY, registry2, "anchorCarried(bytes32,bytes)", [H1, hexs(E1)])
    bc = blk(rc)
    tx, r = send(url, SENDER_KEY, registry, "anchorCarried(bytes32,bytes)", [H1, hexs(E1)])
    b = blk(r)
    basis = {"chains": [{"chainId": CHAIN2, "fromBlock": bc, "toBlock": bc, "registries": [registry2], "senders": [SENDER]},
                        {"chainId": CHAIN, "fromBlock": b, "toBlock": b, "registries": [registry], "senders": [SENDER]}]}
    fx = R.record({"basis": basis, "rpc": {str(CHAIN): url, str(CHAIN2): url2}})
    a = anchors_of(fx)
    assert [x["chainId"] for x in a] == [CHAIN2, CHAIN] and all(x["hash"] == H1 and "entry" in x for x in a)
    write_fixture("12_two_chains_same_hash", fx,
                  "9.2, 9.4: one entry anchored on chain 31337 and on chain 7777. Two records, one exchange log per chainId; the basis lists the chains in ascending order as 9.4 requires, and 9.5 bounds the entry once per chain.")

    # 13: a contract calls the registry: topic 1 names the contract
    tx, r = send(url, SENDER_KEY, forwarder, "forward(address,bytes32,bytes)", [registry, H2, hexs(E2)])
    assert S.qty(r["status"]) == 1 and len(r["logs"]) == 1
    b = blk(r)
    fx = rec(url, chain_basis(b, b, [registry], [SENDER]))
    assert anchors_of(fx) == []
    write_fixture("13_contract_caller_topic1", fx,
                  "9.1: a contract calls the declared registry, so topic 1 is the contract and never the transaction's sender. The hash does sit in the calldata; the log is still no anchor.")

    # 14: a non-canonical anchorCarriedMany encoding: the element off every word boundary
    sel = S.keccak256(b"anchorCarriedMany(bytes32[],bytes[])")[:4]
    def word(n):
        return n.to_bytes(32, "big")
    # head: two offsets; the hashes array placed at 0x45 (69), five bytes past the second head word
    hashes_at = 0x45
    hashes_blob = word(1) + bytes.fromhex(H2[2:])
    entries_at = hashes_at + len(hashes_blob)
    entries_blob = word(1) + word(0x20) + word(len(E2)) + E2 + b"\x00" * ((-len(E2)) % 32)
    body = word(hashes_at) + word(entries_at) + b"\x00" * 5 + hashes_blob + entries_blob
    data = "0x" + (sel + body).hex()
    tx, r = send(url, SENDER_KEY, registry, data=data, gas=300000)
    if S.qty(r["status"]) == 1 and len(r["logs"]) == 1:
        b = blk(r)
        fx = rec(url, chain_basis(b, b, [registry], [SENDER]))
        assert anchors_of(fx) == []
        write_fixture("14_anchor_many_skewed_offset", fx,
                      "9.1: a non-canonical anchorCarriedMany encoding places the hash at calldata offset 105, which is neither a multiple of 32 nor 4 plus one. The registry emits the form with the entry; the law declines to read the hash there, and the log is no anchor.")
    else:
        print("  skipped 14_anchor_many_skewed_offset: the compiler refused the skewed encoding (status %s)" % r["status"])

    # 15-19: rogue logs from a declared registry
    for name, fn, args, sentence, expect in (
        ("15_rogue_two_topics", "twoTopics(bytes32)", [H1],
         "9.1: a log from a declared registry carrying two topics under the form's topic 0 (the hash travels in the data) is not an anchor, whatever its topics hold.", 0),
        ("16_rogue_old_form_three_topics", "oldForm(bytes32)", [H1],
         "9.1: three topics and no data under the two-argument declaration's topic 0, the form an earlier registry emitted. It is not the form of this grammar: no anchor, and a reader's basis that admits the emitter records nothing for it.", 0),
        ("17_rogue_four_topics", "fourTopics(bytes32)", [H1],
         "9.1: a log from a declared registry carrying four topics is not an anchor.", 0),
        ("18_rogue_raw_data", "rawData(bytes32,bytes)", [H1, hexs(E1)],
         "9.1: three well-formed topics whose data is the raw entry bytes and not the ABI encoding of one bytes value: an anchor of topic 2 that carries nothing; one record without entry, and 8.5 lists the hash MISSING.", 1),
        ("19_rogue_wrong_bytes", "wrongBytes(bytes32,bytes)", [H1, hexs(E2)],
         "9.1: three well-formed topics whose data encodes bytes whose sha256 is not topic 2: an anchor that carries nothing; one record without entry.", 1),
    ):
        tx, r = send(url, SENDER_KEY, rogue, fn, args)
        assert S.qty(r["status"]) == 1 and len(r["logs"]) == 1, name
        b = blk(r)
        fx = rec(url, chain_basis(b, b, [rogue], [SENDER]))
        a = anchors_of(fx)
        assert len(a) == expect, (name, a)
        if expect:
            assert "entry" not in a[0] and a[0]["hash"] == H1
        write_fixture(name, fx, sentence)

    # 20: one transaction emitting one hash twice; one record per (chainId, blockNumber, tx, hash)
    tx, r = send(url, SENDER_KEY, registry, "anchorCarriedMany(bytes32[],bytes[])", ["[%s,%s]" % (H1, H1), "[%s,%s]" % (hexs(E1), hexs(E1))])
    assert S.qty(r["status"]) == 1 and len(r["logs"]) == 2
    b = blk(r)
    fx = rec(url, chain_basis(b, b, [registry], [SENDER]))
    a = anchors_of(fx)
    assert len(a) == 1 and a[0]["hash"] == H1 and a[0]["entry"] == hexs(E1)
    write_fixture("20_anchor_many_same_hash_twice", fx,
                  "9.4: anchorCarriedMany with one entry twice emits two logs and yields one record, since a record is at most one per (chainId, blockNumber, tx, hash), carrying the entry.")

    # 21: a chain declared with no registry has no surface
    fx = rec(url, chain_basis(b, b, [], [SENDER]))
    assert anchors_of(fx) == []
    assert not any(ex["method"] == "eth_getLogs" for ex in fx["rpc"][str(CHAIN)])
    write_fixture("21_chain_without_registries", fx,
                  "9.1/9.4: the declared registry list bounds which logs are read; a chains object with an empty registries array reads no log at all, and the recording holds no eth_getLogs exchange.")


# ------------------------------------------------------ hand-written recordings
#
# Section 9.1 reads the sender from the transaction's own signature, so a
# hand-written transaction object must carry a real signature whose signed
# bytes hash to the transaction's hash.  These helpers sign with the same
# secp256k1 code impl-py uses and encode with the scanner's own RLP.

sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "impl-py")))
import glcrypto as ZC  # noqa: E402

KEY_A = int("0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d", 16)  # anvil 1
KEY_B = int(SENDER_KEY, 16)  # anvil 2
REG = "0x5fbdb2315678afecb367f032d93f642f64180aa3"
GAS = "0x30d40"


def addr_of(priv):
    return "0x" + ZC.address_of(ZC.pubkey(priv)).hex()


def hq(n):
    return hex(n)


def block_hash(n):
    return "0x" + S.keccak256(b"hand block " + str(n).encode("ascii")).hex()


def hand_tx(priv, nonce, to, data, block, kind="1559", protected=True, index=0, chain=None):
    """A signed transaction object as a node answers it, with the hash the
    signed bytes carry.  `kind` is "1559", "2930", "4844", "7702" or "legacy";
    a legacy transaction with protected=False signs without EIP-155 and names
    no chain."""
    data_b = S.hexbytes(data)
    to_b = b"" if to is None else S.hexbytes(to)
    CH = CHAIN if chain is None else chain
    if kind == "legacy":
        body = [S._rq(hq(nonce)), S._rq("0x1"), S._rq(GAS), to_b, b"", data_b]
        if protected:
            r, s, v = ZC.sign_digest(priv, S.keccak256(S.rlp(body + [S._rq(hq(CH)), b"", b""])))
            v = CH * 2 + 35 + (v - 27)
        else:
            r, s, v = ZC.sign_digest(priv, S.keccak256(S.rlp(body)))
        raw = S.rlp(body + [S._rq(hq(v)), S._rq(hq(r)), S._rq(hq(s))])
        txo = {"type": "0x0", "nonce": hq(nonce), "gasPrice": "0x1", "gas": GAS, "to": to,
               "value": "0x0", "input": data, "v": hq(v), "r": hq(r), "s": hq(s)}
        if protected:
            txo["chainId"] = hq(CH)
    else:
        ttype = {"2930": 1, "1559": 2, "4844": 3, "7702": 4}[kind]
        al_addr = "0x" + "77" * 20
        al_key = "0x" + "01" * 32
        access = [[S.hexbytes(al_addr), [S.hexbytes(al_key)]]]
        if ttype == 1:
            fields = [S._rq(hq(CH)), S._rq(hq(nonce)), S._rq("0x1"), S._rq(GAS), to_b, b"", data_b, access]
            txo = {"type": "0x1", "chainId": hq(CH), "nonce": hq(nonce), "gasPrice": "0x1", "gas": GAS,
                   "to": to, "value": "0x0", "input": data,
                   "accessList": [{"address": al_addr, "storageKeys": [al_key]}]}
        else:
            fields = [S._rq(hq(CH)), S._rq(hq(nonce)), S._rq("0x1"), S._rq("0x2"), S._rq(GAS),
                      to_b, b"", data_b, access]
            txo = {"type": hq(ttype), "chainId": hq(CH), "nonce": hq(nonce), "maxPriorityFeePerGas": "0x1",
                   "maxFeePerGas": "0x2", "gas": GAS, "to": to, "value": "0x0", "input": data,
                   "accessList": [{"address": al_addr, "storageKeys": [al_key]}]}
            if ttype == 3:
                blob = "0x01" + "ab" * 31
                fields += [S._rq("0x1"), [S.hexbytes(blob)]]
                txo.update({"maxFeePerBlobGas": "0x1", "blobVersionedHashes": [blob]})
            elif ttype == 4:
                # the authorization is another account's (the other test key), so
                # that the sender's own code stays as the codeless answers say
                auth_key = KEY_A if priv != KEY_A else KEY_B
                auth_pre = b"\x05" + S.rlp([S._rq(hq(CH)), S.hexbytes(al_addr), S._rq("0x0")])
                ar, as_, av = ZC.sign_digest(auth_key, S.keccak256(auth_pre))
                fields += [[[S._rq(hq(CH)), S.hexbytes(al_addr), S._rq("0x0"),
                             S._rq(hq(av - 27)), S._rq(hq(ar)), S._rq(hq(as_))]]]
                txo["authorizationList"] = [{"chainId": hq(CH), "address": al_addr, "nonce": "0x0",
                                             "yParity": hq(av - 27), "r": hq(ar), "s": hq(as_)}]
        r, s, v = ZC.sign_digest(priv, S.keccak256(bytes([ttype]) + S.rlp(fields)))
        parity = v - 27
        raw = bytes([ttype]) + S.rlp(fields + [S._rq(hq(parity)), S._rq(hq(r)), S._rq(hq(s))])
        txo.update({"yParity": hq(parity), "v": hq(parity), "r": hq(r), "s": hq(s)})
    h = "0x" + S.keccak256(raw).hex()
    txo.update({"hash": h, "from": addr_of(priv), "blockHash": block_hash(block),
                "blockNumber": hq(block), "transactionIndex": hq(index)})
    return txo


def unsigned_tx(claimed_from, to, data, block, index=0):
    """A transaction with no signature (a system or deposit transaction as a
    node might answer it): type 0x7e, r = s = v = 0, and a `from` the node
    asserts.  Its hash is arbitrary, since no signed bytes exist."""
    h = "0x" + S.keccak256(b"unsigned " + data.encode("ascii")).hex()
    return {"type": "0x7e", "nonce": "0x0", "gas": GAS, "to": to, "value": "0x0",
            "input": data, "v": "0x0", "r": "0x0", "s": "0x0", "hash": h, "from": claimed_from,
            "blockHash": block_hash(block), "blockNumber": hq(block), "transactionIndex": hq(index)}


def abi_bytes(b):
    """The ABI encoding of one `bytes` value: offset word, length word, the
    bytes, zero padding to a multiple of 32 (9.1)."""
    pad = (-len(b)) % 32
    return "0x" + (32).to_bytes(32, "big").hex() + len(b).to_bytes(32, "big").hex() + b.hex() + "00" * pad


def carried_log(emitter, sender, h, txo, data, log_index=0):
    """A log of the form: topic 0 of the declaration, the sender and the hash
    as topics 1 and 2, `data` as given."""
    return {"address": emitter, "topics": [S.ANCHORED_TOPIC0, "0x" + "00" * 12 + sender[2:], h],
            "data": data, "blockNumber": txo["blockNumber"], "blockHash": txo["blockHash"],
            "transactionHash": txo["hash"], "transactionIndex": txo["transactionIndex"],
            "logIndex": hq(log_index), "removed": False}


def receipt_of(txo, status=1, logs=(), contract=None):
    return {"blockHash": txo["blockHash"], "blockNumber": txo["blockNumber"],
            "contractAddress": contract, "cumulativeGasUsed": "0x5208", "effectiveGasPrice": "0x1",
            "from": txo["from"], "gasUsed": "0x5208", "logs": list(logs), "logsBloom": "0x" + "00" * 256,
            "status": hq(status), "to": txo["to"], "transactionHash": txo["hash"],
            "transactionIndex": txo["transactionIndex"], "type": txo["type"]}


def hand_rpc(windows=(), txs=(), receipts=(), blocks=(), codeless=(), class_windows=(), codes=(), head=None, chain=None):
    """The recorded exchanges a hand fixture needs.  `windows` are
    (fromBlock, toBlock, registries, logs) in the chunks the scanner asks
    for, queried by address under the form's topic 0; `class_windows` are
    (fromBlock, toBlock, senders, logs), the query a basis with class items
    makes (no address filter, the senders as topic 1); `codeless` are
    (address, block) pairs that answer with no code; `codes` are (address,
    block, code) triples that answer with code; `head` is what eth_blockNumber
    answers, the largest toBlock queried where not given (9.4)."""
    cid = CHAIN if chain is None else chain
    if head is None:
        head = max([hi for _lo, hi, _r, _l in windows] + [hi for _lo, hi, _s, _l in class_windows] + [0])
    ex = [{"method": "eth_chainId", "params": [], "result": hq(cid)},
          {"method": "eth_blockNumber", "params": [], "result": hq(head)}]
    for lo, hi, regs, logs in windows:
        ex.append({"method": "eth_getLogs",
                   "params": [{"fromBlock": hq(lo), "toBlock": hq(hi), "address": list(regs),
                               "topics": [[S.ANCHORED_TOPIC0]]}],
                   "result": list(logs)})
    for lo, hi, senders, logs in class_windows:
        ex.append({"method": "eth_getLogs",
                   "params": [{"fromBlock": hq(lo), "toBlock": hq(hi),
                               "topics": [[S.ANCHORED_TOPIC0],
                                          ["0x" + "00" * 12 + s[2:] for s in sorted(senders)]]}],
                   "result": list(logs)})
    for a, n, code in codes:
        ex.append({"method": "eth_getCode", "params": [a, hq(n)], "result": code})
    for txo in txs:
        ex.append({"method": "eth_getTransactionByHash", "params": [txo["hash"]], "result": txo})
    for rc in receipts:
        ex.append({"method": "eth_getTransactionReceipt", "params": [rc["transactionHash"]], "result": rc})
    for n in blocks:
        ex.append({"method": "eth_getBlockByNumber", "params": [hq(n), False],
                   "result": {"number": hq(n), "timestamp": hq(0x6a000000 + n)}})
    for a, n in codeless:
        ex.append({"method": "eth_getCode", "params": [a, hq(n)], "result": "0x"})
    return {str(cid): ex}


def hand_fixture(basis, rpc):
    fixture = {"basis": basis, "rpc": rpc}
    fixture["expected"] = S.canon(S.run(fixture)).decode("utf-8")
    return fixture


def failing_fixture(basis, rpc):
    """A fixture of a scan that fails: `expected` null; the construction asserts the failure."""
    fixture = {"basis": basis, "rpc": rpc, "expected": None}
    ok, msg = S.try_run(fixture)
    assert not ok, "the scan was expected to fail and answered"
    return fixture


def hand_written():
    A = addr_of(KEY_A)
    B = addr_of(KEY_B)
    ENTRY = b'{"an entry":"the bytes a venue carried"}'
    HE = sha(ENTRY)
    call = "0x" + "00" * 4 + HE[2:]
    good = abi_bytes(ENTRY)
    win = {"chains": [{"chainId": CHAIN, "fromBlock": 0, "toBlock": 3, "registries": [REG], "senders": [A]}]}

    def record_of(txo, bn, extra=None, sender=A, h=HE):
        d = {"blockNumber": bn, "blockTimestamp": 0x6a000000 + bn, "chainId": CHAIN, "hash": h,
             "sender": sender, "tx": txo["hash"], "verdict": "counted"}
        if extra is not None:
            d["entry"] = extra
        return d

    # 22: 9.4, two adjacent windows of one chain with differing senders.
    t1 = hand_tx(KEY_A, 0, REG, call, 2)
    t2 = hand_tx(KEY_B, 0, REG, call, 4)
    t3 = hand_tx(KEY_A, 1, REG, call, 4, index=1)
    l1 = carried_log(REG, A, HE, t1, good)
    l2 = carried_log(REG, B, HE, t2, good)
    l3 = carried_log(REG, A, HE, t3, good, log_index=1)
    basis = {"chains": [
        {"chainId": CHAIN, "fromBlock": 0, "toBlock": 2, "registries": [REG], "senders": [A]},
        {"chainId": CHAIN, "fromBlock": 3, "toBlock": 5, "registries": [REG], "senders": [B]},
    ]}
    fx = hand_fixture(basis, hand_rpc(windows=[(0, 2, [REG], [l1]), (3, 5, [REG], [l2, l3])],
                                      txs=[t1, t2, t3], receipts=[receipt_of(t1, logs=[l1]), receipt_of(t2, logs=[l2]), receipt_of(t3, logs=[l3])],
                                      blocks=[2, 4], codeless=[(A, 1), (A, 2), (B, 3), (B, 4)]))
    assert [(a["blockNumber"], a["sender"], a["entry"]) for a in anchors_of(fx)] == [(2, A, hexs(ENTRY)), (4, B, hexs(ENTRY))]
    write_fixture("22_two_windows_adjacent_senders_differ", fx,
                  "9.4: two chains objects of one chain over adjacent windows (0..2 and 3..5) whose senders differ form a basis; each window's senders bound that window alone, so the first sender's anchor in block 2 and the second sender's in block 4 are records, each carrying the entry, and the first sender's anchor in block 4 is outside the basis.")

    # 23: 9.1, a transaction without a signature has no sender.
    u = unsigned_tx(A, REG, call, 2)
    lu = carried_log(REG, A, HE, u, good)
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [lu])], txs=[u], receipts=[receipt_of(u, logs=[lu])],
                                    blocks=[2], codeless=[(A, 1), (A, 2)]))
    assert anchors_of(fx) == []
    write_fixture("23_unsigned_tx_no_sender", fx,
                  "9.1: a transaction that carries no secp256k1 signature (a system or deposit transaction; r and s are zero and the node asserts a from field) has no sender in this grammar. A log of the form it carries is not an anchor whatever its topics hold.")

    # 24: 9.1, a legacy transaction outside EIP-155 names no chain.
    txo = hand_tx(KEY_A, 0, REG, call, 2, kind="legacy", protected=False)
    lg = carried_log(REG, A, HE, txo, good)
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [lg])], txs=[txo], receipts=[receipt_of(txo, logs=[lg])],
                                    blocks=[2], codeless=[(A, 1), (A, 2)]))
    assert anchors_of(fx) == []
    write_fixture("24_legacy_unprotected_no_anchor", fx,
                  "9.1: a transaction whose legacy signature carries v = 27 or 28 names no chain, so it is not an anchor under this grammar; the node's from field is never read.")

    # 25: 9.1, a signed transaction of a type this scanner cannot decode: the scan fails.
    u = unsigned_tx(A, REG, call, 2)
    u.update({"type": "0x10", "r": "0x" + "11" * 32, "s": "0x" + "22" * 32, "v": "0x0"})
    lu = carried_log(REG, A, HE, u, good)
    fx = failing_fixture(win, hand_rpc(windows=[(0, 3, [REG], [lu])], txs=[u], receipts=[receipt_of(u, logs=[lu])],
                                       blocks=[2], codeless=[(A, 1), (A, 2)]))
    write_fixture("25_unknown_tx_type_scan_fails", fx,
                  "9.1, 9.4: a transaction of a type this scanner cannot decode (0x10) that carries a signature (r and s nonzero) is one the verifier cannot tell the sender of; the verifier has not scanned the basis, and the replay fails (exit 2). The fixture's expected is null.")

    # 26-28: the sender of a typed transaction of types 1, 3 and 4 is recovered over its own payload.
    for name, kind, note in [
        ("26_type1_access_list", "2930", "an EIP-2930 transaction (type 1) with an access list"),
        ("27_type3_blob", "4844", "an EIP-4844 transaction (type 3) with a blob versioned hash"),
        ("28_type4_authorization", "7702", "an EIP-7702 transaction (type 4) carrying another account's authorization (so that the sender's own code stays as the codeless answers say; the list itself decides nothing)"),
    ]:
        txo = hand_tx(KEY_B, 3, REG, call, 6, kind=kind)
        lg = carried_log(REG, B, HE, txo, good)
        wb = {"chains": [{"chainId": CHAIN, "fromBlock": 0, "toBlock": 6, "registries": [REG], "senders": [B]}]}
        fx = hand_fixture(wb, hand_rpc(windows=[(0, 6, [REG], [lg])], txs=[txo], receipts=[receipt_of(txo, logs=[lg])],
                                       blocks=[6], codeless=[(B, 5), (B, 6)]))
        assert anchors_of(fx) == [record_of(txo, 6, hexs(ENTRY), sender=B)]
        write_fixture(name, fx, "9.1: the sender of %s is recovered over that type's signing payload; the registry log it carries is a record with its entry, and the chain's answers to the codeless test at both boundaries give the verdict counted." % note)

    # 29: 9.3, block 0 has one boundary.
    txo = hand_tx(KEY_A, 0, REG, call, 0, kind="legacy")
    lg = carried_log(REG, A, HE, txo, good)
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [lg])], txs=[txo], receipts=[receipt_of(txo, logs=[lg])],
                                    blocks=[0], codeless=[(A, 0)]))
    assert anchors_of(fx) == [record_of(txo, 0, hexs(ENTRY))]
    write_fixture("29_anchor_in_block_zero", fx,
                  "9.3: block 0 has one boundary, so a codeless sender there is counted on that single state. Hand-written recording (a legacy EIP-155 transaction signed with a test key): no node includes a transaction in its genesis block.")

    # 30: 9.1, a log of the form emitted during a creation.
    init = HE + "0x60016000f3"[2:]
    txo = hand_tx(KEY_A, 0, None, init, 2)
    lg = carried_log(REG, A, HE, txo, good)
    rc = receipt_of(txo, logs=[lg], contract="0x" + "ee" * 20)
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [lg])], txs=[txo], receipts=[rc],
                                    blocks=[2], codeless=[(A, 1), (A, 2)]))
    assert anchors_of(fx) == []
    write_fixture("30_creation_log_not_anchor", fx,
                  "9.1: a log of the form emitted during a contract creation (the transaction has no recipient) is not an anchor whatever its topics hold: the transaction carries init code and no calldata, so the containment test has no field to run against. The log names the sender, a word the init code carries at offset 0, and the entry's bytes, and is still refused.")

    # 31-33: class items admit emitters by the code hash of their account
    X = "0x" + "ce" * 20
    CODE = "0x6001600055"
    K = "0x" + S.keccak256(S.hexbytes(CODE)).hex()
    OTHER = "0x" + S.keccak256(b"other code").hex()
    cls = {"chains": [{"chainId": CHAIN, "fromBlock": 0, "toBlock": 3, "registries": [{"codeHash": K}], "senders": [A]}]}
    tx60 = hand_tx(KEY_A, 0, X, call, 2)
    lg = carried_log(X, A, HE, tx60, good)
    fx = hand_fixture(cls, hand_rpc(class_windows=[(0, 3, [A], [lg])], txs=[tx60], receipts=[receipt_of(tx60, logs=[lg])],
                                    blocks=[2], codeless=[(A, 1), (A, 2)], codes=[(X, 2, CODE)]))
    assert anchors_of(fx) == [record_of(tx60, 2, hexs(ENTRY))]
    write_fixture("31_class_item_admits_emitter", fx,
                  "9.1, 9.4: a basis whose registries holds a class item and no address; the scan asks for the form's logs from the declared senders without an address filter, reads the emitter's code at the end of the log's block, and admits the emitter whose code hash equals the class item's word. The venue's log is a record with its entry.")
    cls2 = {"chains": [{"chainId": CHAIN, "fromBlock": 0, "toBlock": 3, "registries": [{"codeHash": OTHER}], "senders": [A]}]}
    fx = hand_fixture(cls2, hand_rpc(class_windows=[(0, 3, [A], [lg])], txs=[tx60], receipts=[receipt_of(tx60, logs=[lg])],
                                     blocks=[2], codeless=[(A, 1), (A, 2)], codes=[(X, 2, CODE)]))
    assert anchors_of(fx) == []
    write_fixture("32_class_item_code_differs", fx,
                  "9.1, 9.4: the same log under a class item naming another code hash; the emitter is admitted by neither address nor class, and the log is not an anchor.")
    mixed = {"chains": [{"chainId": CHAIN, "fromBlock": 0, "toBlock": 3, "registries": [REG, {"codeHash": K}], "senders": [A]}]}
    txr = hand_tx(KEY_A, 1, REG, call, 3)
    lr = carried_log(REG, A, HE, txr, good)
    fx = hand_fixture(mixed, hand_rpc(class_windows=[(0, 3, [A], [lg, lr])], txs=[tx60, txr],
                                      receipts=[receipt_of(tx60, logs=[lg]), receipt_of(txr, logs=[lr])],
                                      blocks=[2, 3], codeless=[(A, 1), (A, 2), (A, 3)], codes=[(X, 2, CODE)]))
    assert [(a["blockNumber"], a["hash"], "entry" in a) for a in anchors_of(fx)] == [(2, HE, True), (3, HE, True)]
    write_fixture("33_class_item_and_address_mixed", fx,
                  "9.4: registries listing an address and a class item; the scan queries by sender topic, admits the registry's log by its address without reading its code and the venue's by its code, one record each.")

    # 34: two logs of one transaction carrying one hash, one with the bytes and one without: one record with the entry
    tx63 = hand_tx(KEY_A, 2, REG, call, 3, index=1)
    l63a = carried_log(REG, A, HE, tx63, "0x" + ENTRY.hex())
    l63b = carried_log(REG, A, HE, tx63, good, log_index=1)
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [l63a, l63b])], txs=[tx63],
                                    receipts=[receipt_of(tx63, logs=[l63a, l63b])], blocks=[3], codeless=[(A, 2), (A, 3)]))
    a = anchors_of(fx)
    assert len(a) == 1 and a[0].get("entry") == hexs(ENTRY), a
    write_fixture("34_two_logs_one_hash_one_record", fx,
                  "9.4: two logs of one transaction carrying one hash, one whose data carries nothing and one whose data carries the bytes, are one record, and that record carries the entry since one of the logs carried the bytes.")

    # 35-39: the refusals of the form's data, each an anchor carrying nothing
    txo = hand_tx(KEY_A, 0, REG, call, 2)
    bad_offset = "0x" + (64).to_bytes(32, "big").hex() + good[66:]
    bad_length = "0x" + good[2:66] + (len(ENTRY) + 40).to_bytes(32, "big").hex() + good[130:]
    bad_pad = good[:-2] + "01"
    for name, data, why in (
        ("35_data_hash_mismatch", abi_bytes(b"some other bytes"), "bytes whose sha256 is not topic 2"),
        ("36_data_not_encoded", "0x" + ENTRY.hex(), "the raw bytes and not the ABI encoding of one bytes value"),
        ("37_data_offset_not_32", bad_offset, "an offset word other than 32"),
        ("38_data_length_mismatch", bad_length, "a length word that does not match the data's length"),
        ("39_data_padding_nonzero", bad_pad, "padding bytes that are not zero"),
    ):
        lg = carried_log(REG, A, HE, txo, data)
        fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [lg])], txs=[txo], receipts=[receipt_of(txo, logs=[lg])],
                                        blocks=[2], codeless=[(A, 1), (A, 2)]))
        a = anchors_of(fx)
        assert len(a) == 1 and "entry" not in a[0], (name, a)
        write_fixture(name, fx, "9.1: a log of the form whose data is %s; the log is an anchor of topic 2 that carries nothing, one record without entry, and 8.5 places the hash." % why)

    # ---- round 1 of 2026-10-16: the sentences of 9.1 to 9.4 no recording witnessed
    # 58: a receipt without a status field
    txo = hand_tx(KEY_A, 0, REG, call, 2)
    lg = carried_log(REG, A, HE, txo, good)
    rc = receipt_of(txo, logs=[lg]); del rc["status"]
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [lg])], txs=[txo], receipts=[rc], blocks=[2], codeless=[(A, 1), (A, 2)]))
    assert anchors_of(fx) == []
    write_fixture("58_receipt_without_status", fx,
                  "9.1: a receipt that carries no status field carries no status 1, so no transaction of such a receipt is an anchor; the scan answers with no record and does not fail.")
    # 59: status 0 beside a returned log
    rc = receipt_of(txo, status=0, logs=[lg])
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [lg])], txs=[txo], receipts=[rc], blocks=[2], codeless=[(A, 1), (A, 2)]))
    assert anchors_of(fx) == []
    write_fixture("59_status_zero_with_log", fx,
                  "9.1: the node returns a log of the form from a transaction whose receipt carries status 0; the log is no anchor.")
    # 60: the hash at calldata offset 0 and at offset 32
    t60a = hand_tx(KEY_A, 0, REG, HE, 2)
    t60b = hand_tx(KEY_A, 1, REG, "0x" + "00" * 32 + HE[2:], 2, index=1)
    la = carried_log(REG, A, HE, t60a, good)
    lb = carried_log(REG, A, HE, t60b, good, log_index=1)
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [la, lb])], txs=[t60a, t60b],
                                    receipts=[receipt_of(t60a, logs=[la]), receipt_of(t60b, logs=[lb])], blocks=[2], codeless=[(A, 1), (A, 2)]))
    assert [a["tx"] for a in anchors_of(fx)] == sorted([t60a["hash"], t60b["hash"]]) and all("entry" in a for a in anchors_of(fx))
    write_fixture("60_hash_at_offsets_zero_and_32", fx,
                  "9.1: the containment test admits a 32-byte window beginning at a multiple of 32 as well as at 4 plus one: calldata that is the hash alone (offset 0) and calldata of one zero word then the hash (offset 32) each carry the hash; two records with their entries.")
    # 61: the hash straddling the end of the calldata
    t61 = hand_tx(KEY_A, 0, REG, "0x" + "00" * 4 + HE[2:-2], 2)
    l61 = carried_log(REG, A, HE, t61, good)
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [l61])], txs=[t61], receipts=[receipt_of(t61, logs=[l61])], blocks=[2], codeless=[(A, 1), (A, 2)]))
    assert anchors_of(fx) == []
    write_fixture("61_hash_not_wholly_within_calldata", fx,
                  "9.1: the calldata ends one byte before the end of the window where the hash would lie; a window must lie wholly within the calldata, so the log is no anchor.")
    # 62: a returned log under another topic 0
    l62 = dict(lg); l62["topics"] = ["0x" + S.keccak256(b"Other(address,bytes32,bytes)").hex()] + lg["topics"][1:]
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [l62, lg])], txs=[txo], receipts=[receipt_of(txo, logs=[l62, lg])], blocks=[2], codeless=[(A, 1), (A, 2)]))
    assert len(anchors_of(fx)) == 1
    write_fixture("62_returned_log_other_topic0", fx,
                  "9.1: a node that returns a log under another topic 0 beside the form's log; the first is no anchor whatever its other topics hold, the second is a record.")
    # 63: a lying from field beside a valid signature
    t63 = dict(txo); t63["from"] = B
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [lg])], txs=[t63], receipts=[receipt_of(txo, logs=[lg])], blocks=[2], codeless=[(A, 1), (A, 2)]))
    assert [a["sender"] for a in anchors_of(fx)] == [A]
    write_fixture("63_lying_from_field", fx,
                  "9.1: the sender is the address recovered from the transaction's own signature; a node whose from field names another address changes nothing, and the record names the recovered sender.")
    # 64: the empty entry
    HZ = sha(b"")
    t64 = hand_tx(KEY_A, 0, REG, "0x" + "00" * 4 + HZ[2:], 2)
    l64 = carried_log(REG, A, HZ, t64, abi_bytes(b""))
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [l64])], txs=[t64], receipts=[receipt_of(t64, logs=[l64])], blocks=[2], codeless=[(A, 1), (A, 2)]))
    assert anchors_of(fx) == [record_of(t64, 2, "0x", h=HZ)]
    write_fixture("64_empty_entry", fx,
                  "9.1: the data encodes the empty byte string (an offset word of 32 and a length word of 0) and topic 2 is sha256 of nothing; the record carries the empty entry, spelled 0x.")
    # 65: one transaction hash in two blocks (a reorganised node answering both)
    l65a = carried_log(REG, A, HE, txo, good)
    l65b = dict(l65a); l65b["blockNumber"] = hq(3); l65b["blockHash"] = block_hash(3)
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [l65a, l65b])], txs=[txo], receipts=[receipt_of(txo, logs=[l65a])], blocks=[2, 3], codeless=[(A, 1), (A, 2), (A, 3)]))
    assert [a["blockNumber"] for a in anchors_of(fx)] == [2, 3]
    write_fixture("65_one_tx_hash_in_two_blocks", fx,
                  "9.2, 9.4: a node that returns the form's log for one transaction hash in two blocks yields two records, since a record is one per (chainId, blockNumber, tx, hash) and the block is read from the log.")
    # 66, 67: the two boundaries of 9.3 one at a time
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [lg])], txs=[txo], receipts=[receipt_of(txo, logs=[lg])], blocks=[2], codeless=[(A, 2)], codes=[(A, 1, "0xef0100" + "11" * 20)]))
    assert [a["verdict"] for a in anchors_of(fx)] == ["VOID"]
    write_fixture("66_void_on_first_boundary_alone", fx,
                  "9.3: code at the end of the previous block and none at the end of the log's own block; either boundary with code voids the record: VOID.")
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [lg])], txs=[txo], receipts=[receipt_of(txo, logs=[lg])], blocks=[2], codeless=[(A, 1)]))
    assert [a["verdict"] for a in anchors_of(fx)] == ["UNPROVEN"]
    write_fixture("67_unproven_one_boundary_unanswered", fx,
                  "9.3: the node answers the first boundary codeless and does not answer the second; a verdict needs both, so the record is UNPROVEN.")
    # 68, 69, 70: failures of the scan
    rpc68 = hand_rpc(windows=[(0, 3, [REG], [lg])], txs=[txo], receipts=[receipt_of(txo, logs=[lg])], blocks=[2], codeless=[(A, 1), (A, 2)])
    for ex in rpc68[str(CHAIN)]:
        if ex["method"] == "eth_getLogs":
            ex.pop("result", None); ex["error"] = {"code": -32005, "message": "query returned more than 10000 results"}
    write_fixture("68_getlogs_error_scan_fails", failing_fixture(win, rpc68),
                  "9.4: the chain answers eth_getLogs with an error; the verifier did not obtain every log the completeness rule enumerates, so the basis is not scanned and the replay fails (exit 2). The fixture's expected is null.")
    rpc69 = hand_rpc(windows=[(0, 3, [REG], [lg])], txs=[txo], receipts=[receipt_of(txo, logs=[lg])], blocks=[2], codeless=[(A, 1), (A, 2)])
    for ex in rpc69[str(CHAIN)]:
        if ex["method"] == "eth_getLogs":
            ex["result"] = None
    write_fixture("69_getlogs_null_scan_fails", failing_fixture(win, rpc69),
                  "9.4: the chain answers eth_getLogs with null, which is no array of logs; a chain that did not answer is a failure of the scan and never an empty result. The fixture's expected is null.")
    rpc70 = hand_rpc(windows=[(0, 3, [REG], [lg])], txs=[txo], receipts=[receipt_of(txo, logs=[lg])], blocks=[2], codeless=[(A, 1), (A, 2)], head=2)
    write_fixture("70_range_past_head_scan_fails", failing_fixture(win, rpc70),
                  "9.4: the declared range reaches block 3 while the chain's head (eth_blockNumber) is 2; a range that reaches past the head is a failure of the scan. The fixture's expected is null.")
    # 71: two adjacent windows of one chain differing in registries alone
    REG2 = "0x" + "77" * 20
    t71 = hand_tx(KEY_A, 1, REG2, call, 4, index=1)
    l71 = carried_log(REG2, A, HE, t71, good, log_index=1)
    basis71 = {"chains": [
        {"chainId": CHAIN, "fromBlock": 0, "toBlock": 2, "registries": [REG], "senders": [A]},
        {"chainId": CHAIN, "fromBlock": 3, "toBlock": 5, "registries": [REG2], "senders": [A]},
    ]}
    fx = hand_fixture(basis71, hand_rpc(windows=[(0, 2, [REG], [lg]), (3, 5, [REG2], [l71])], txs=[txo, t71],
                                        receipts=[receipt_of(txo, logs=[lg]), receipt_of(t71, logs=[l71])], blocks=[2, 4], codeless=[(A, 1), (A, 2), (A, 3), (A, 4)]))
    assert [(a["blockNumber"], a["tx"]) for a in anchors_of(fx)] == [(2, txo["hash"]), (4, t71["hash"])]
    write_fixture("71_two_windows_adjacent_registries_differ", fx,
                  "9.4: two chains objects of one chain over adjacent windows whose registries differ and whose senders agree form a basis; each window reads its own registry, one record each.")
    # 72: a class emitter whose code differs across blocks
    X2 = "0x" + "cf" * 20
    CODE = "0x6001600055"
    K = "0x" + S.keccak256(S.hexbytes(CODE)).hex()
    cls = {"chains": [{"chainId": CHAIN, "fromBlock": 0, "toBlock": 3, "registries": [{"codeHash": K}], "senders": [A]}]}
    t72a = hand_tx(KEY_A, 0, X2, call, 2)
    t72b = hand_tx(KEY_A, 1, X2, call, 3)
    l72a = carried_log(X2, A, HE, t72a, good)
    l72b = carried_log(X2, A, HE, t72b, good)
    fx = hand_fixture(cls, hand_rpc(class_windows=[(0, 3, [A], [l72a, l72b])], txs=[t72a, t72b],
                                    receipts=[receipt_of(t72a, logs=[l72a]), receipt_of(t72b, logs=[l72b])], blocks=[2, 3],
                                    codeless=[(A, 1), (A, 2), (A, 3)], codes=[(X2, 2, CODE), (X2, 3, "0x6002600055")]))
    assert [a["blockNumber"] for a in anchors_of(fx)] == [2]
    write_fixture("72_class_emitter_code_differs_across_blocks", fx,
                  "9.1, 9.4: an emitter admitted by class is read on the code hash of its account at the end of each log's block; the block-2 log is admitted and the block-3 log, after the code changed, is not.")
    # 73: a properly signed galeed/1 entry carried: the fragment yields a ledger entry
    import subprocess
    gl1 = os.path.normpath(os.path.join(HERE, "..", "impl-py", "gl1.py"))
    env6 = {"body": {"statement_md": "a seat that publishes its entries"}, "entryType": "genesis", "judge": A, "prev": None, "seq": 0, "spec": "galeed/1"}
    tmp = os.path.join(HERE, ".b6.tmp")
    with open(tmp, "wb") as fh:
        fh.write(S.canon(env6))
    signed = json.loads(subprocess.run([sys.executable, gl1, "sign", "0x%064x" % KEY_A, tmp], capture_output=True, check=True).stdout)
    os.remove(tmp)
    ENTRY73 = S.canon(dict(env6, sig=signed["sig"]))
    H73 = sha(ENTRY73)
    t73 = hand_tx(KEY_A, 0, REG, "0x" + "00" * 4 + H73[2:], 2)
    l73 = carried_log(REG, A, H73, t73, abi_bytes(ENTRY73))
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [l73])], txs=[t73], receipts=[receipt_of(t73, logs=[l73])], blocks=[2], codeless=[(A, 1), (A, 2)]))
    assert anchors_of(fx) == [record_of(t73, 2, hexs(ENTRY73), h=H73)]
    frag = json.loads(fx["expected"]); frag["root"] = A
    with open(tmp, "wb") as fh:
        fh.write(json.dumps(frag).encode())
    rep = json.loads(subprocess.run([sys.executable, gl1, "audit", tmp], capture_output=True, check=True).stdout)
    os.remove(tmp)
    assert rep["label"] == "COMPLETE" and rep["entries"] == 1, rep
    write_fixture("73_signed_genesis_carried", fx,
                  "9.1 to 8.1: a genesis entry signed by the sender's key, carried by the form; the record carries bytes that pass 4.3, and the fragment fed to the audit with the sender as root yields a ledger of one entry, COMPLETE.")
    # 74, 75: envelopes naming another chain
    t74 = hand_tx(KEY_A, 0, REG, call, 2, kind="legacy", chain=1)
    l74 = carried_log(REG, A, HE, t74, good)
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [l74])], txs=[t74], receipts=[receipt_of(t74, logs=[l74])], blocks=[2], codeless=[(A, 1), (A, 2)]))
    assert anchors_of(fx) == []
    write_fixture("74_legacy_signed_for_other_chain", fx,
                  "9.1: a legacy EIP-155 transaction whose signature names chain 1 inside a block of chain 31337; a transaction whose envelope names another chain than the one the log sits on is not an anchor.")
    t75 = hand_tx(KEY_A, 0, REG, call, 2, chain=1)
    l75 = carried_log(REG, A, HE, t75, good)
    fx = hand_fixture(win, hand_rpc(windows=[(0, 3, [REG], [l75])], txs=[t75], receipts=[receipt_of(t75, logs=[l75])], blocks=[2], codeless=[(A, 1), (A, 2)]))
    assert anchors_of(fx) == []
    write_fixture("75_typed_signed_for_other_chain", fx,
                  "9.1: a type-2 transaction whose envelope names chain 1 inside a block of chain 31337 is not an anchor.")
    # 76: a node answer whose envelope fields do not hash to the transaction: the scan fails
    t76 = dict(txo); t76["chainId"] = hq(1)
    l76 = carried_log(REG, A, HE, t76, good)
    write_fixture("76_altered_envelope_scan_fails", failing_fixture(win, hand_rpc(windows=[(0, 3, [REG], [l76])], txs=[t76], receipts=[receipt_of(txo, logs=[l76])], blocks=[2], codeless=[(A, 1), (A, 2)])),
                  "9.1, 9.4: the node answers a transaction object whose chainId was altered, so the signed bytes no longer hash to the transaction's hash; the node is answering about other bytes, the verifier cannot tell the sender, and the scan fails before any field of the envelope decides anything. The fixture's expected is null.")



# ---------------------------------------------------------- basis forms (9.4)


def basis_forms():
    good_chain = {"chainId": 1, "fromBlock": 0, "toBlock": 10, "registries": [], "senders": []}
    base = lambda **kw: dict({"chains": []}, **kw)  # noqa: E731
    cases = [
        ("40_basis_not_object", [1], "the basis is not an object"),
        ("41_basis_second_member", {"chains": [], "transactions": []}, "the basis carries a member beyond chains"),
        ("42_basis_chains_absent", {}, "the basis lacks its member"),
        ("43_basis_chains_not_array", base(chains={}), "chains is not an array"),
        ("44_basis_chain_not_object", base(chains=[1]), "a chains element is not an object"),
        ("45_basis_chain_member_set", base(chains=[dict(good_chain, extra=1)]), "a chains element carries a member outside its five"),
        ("46_basis_chain_block_form", base(chains=[dict(good_chain, fromBlock="0x0")]), "fromBlock is not an int"),
        ("47_basis_chain_range", base(chains=[dict(good_chain, fromBlock=11)]), "fromBlock exceeds toBlock"),
        ("48_basis_registries_not_array", base(chains=[dict(good_chain, registries="0x00")]), "registries is not an array"),
        ("49_basis_registry_form", base(chains=[dict(good_chain, registries=["0x00"])]), "a registry is neither hex20 nor a class item"),
        ("50_basis_chains_duplicate", base(chains=[good_chain, dict(good_chain)]), "two chains objects of one chainId cover one range (equal objects overlap, and are not ordered by (chainId, fromBlock))"),
        ("51_basis_registries_unsorted", base(chains=[dict(good_chain, registries=["0x" + "bb" * 20, "0x" + "aa" * 20])]), "registries is not bytewise ascending"),
        ("52_basis_senders_repeated", base(chains=[dict(good_chain, senders=["0x" + "aa" * 20, "0x" + "aa" * 20])]), "senders repeats an element"),
        ("53_basis_chains_unordered", base(chains=[dict(good_chain, chainId=2), good_chain]), "chains is not ascending by (chainId, fromBlock)"),
        ("54_basis_chains_overlap", base(chains=[good_chain, dict(good_chain, fromBlock=5, toBlock=20)]), "two chains objects of one chainId have overlapping ranges"),
        ("55_basis_chains_adjacent_equal", base(chains=[good_chain, dict(good_chain, fromBlock=11, toBlock=20)]), "two chains objects of one chainId are adjacent with equal registries and senders, which one object would spell"),
        ("56_basis_class_item_form", base(chains=[dict(good_chain, registries=[{"codeHash": "0x" + "aa" * 20}])]), "a class item's codeHash is not hex32"),
        ("57_basis_class_item_before_address", base(chains=[dict(good_chain, registries=[{"codeHash": "0x" + "aa" * 32}, "0x" + "bb" * 20])]), "an address after a class item; the strings come first"),
    ]
    for name, basis, why in cases:
        fixture = {"basis": basis, "expected": NO_LABEL, "rpc": {}}
        assert S.canon(S.run(fixture)).decode("utf-8") == NO_LABEL, name
        write_fixture(name, fixture, "9.4: %s; the scan has no label and consults no node." % why)


def replay_all():
    bad = 0
    names = [n for n in sorted(os.listdir(FIX)) if n.endswith(".json")]
    for n in names:
        with open(os.path.join(FIX, n)) as fh:
            fx = json.load(fh)
        ok, got = S.try_run(fx)
        if fx["expected"] is None:
            if ok:
                bad += 1
                print("  REPLAY DIFFERS (expected a failing scan)", n)
        elif not ok or S.canon(got).decode("utf-8") != fx["expected"]:
            bad += 1
            print("  REPLAY DIFFERS", n)
    print("replayed %d fixtures, %d differ" % (len(names), bad))
    return bad


def start_anvil(port, chain_id):
    p = subprocess.Popen(["anvil", "--port", str(port), "--chain-id", str(chain_id), "--hardfork", "prague", "--silent"],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    url = "http://127.0.0.1:%d" % port
    for _ in range(100):
        try:
            if S.qty(rpc(url, "eth_chainId", [])) == chain_id:
                return p, url
        except Exception:
            time.sleep(0.1)
    p.terminate()
    raise RuntimeError("anvil did not come up")


def main():
    os.makedirs(FIX, exist_ok=True)
    if sys.argv[1:] == ["hand"]:
        hand_written()
        basis_forms()
        return 1 if replay_all() else 0
    a1, url = start_anvil(free_port(), CHAIN)
    a2, url2 = start_anvil(free_port(), CHAIN2)
    try:
        recorded(url, url2)
    finally:
        a1.terminate(); a1.wait()
        a2.terminate(); a2.wait()
    hand_written()
    basis_forms()
    return 1 if replay_all() else 0


if __name__ == "__main__":
    sys.exit(main())
