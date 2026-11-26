// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Vm} from "forge-std/Vm.sol";
import {EravonBase} from "../EravonBase.sol";
import {Eravon} from "../../src/Eravon.sol";
import {RouteView} from "../../src/IGaleedVenue.sol";
import {ModeRouter} from "../Eravon.router.t.sol";
import {EchoRouter, FirstEligibleRouter} from "../replay/ReplayRouters.sol";

/// Sender against sender (FUZZ, open-draw round 1). On one armed case and one block, every sender class lands the
/// same `redraw` from one snapshot; the whole post-state (the call's answer, every seat, the case status, the
/// sixth-word stage view, the balances of every account the case can pay, the sender's own balance change, and
/// every event the call emits) is fingerprinted and must be one across the classes (standard s7: the answer is
/// the seed's and the panel's, whoever sends it). Without a router, with routers of the safe class (one that
/// reads nothing but the view: firstEligible, a fixed index, the hash of the whole view, the echo router that
/// tests the view is the venue's own), and with failing routers (revert, two words, 31 bytes, gas hog, write).
contract EravonR27SenderDiff is EravonBase {
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("guild covenant");
    bytes32 internal constant ROSTER = keccak256("roster");
    bytes internal constant ENTRY = "resignation entry";
    uint256 internal constant A = 100 ether;
    bytes32 internal T;
    address[] internal members;
    bytes32[] internal words;
    address[] internal watch;

    function setUp() public {
        setUpBase();
        T = logic.PROXY_CODEHASH();
    }

    struct P {
        uint8 rk; // 0 none, 1 ModeRouter, 2 EchoRouter, 3 FirstEligibleRouter
        uint8 mode; // ModeRouter's mode at the measured draw
        uint256 at;
        uint8 path; // 0 first draw, 1 after a decline, 2 after an offer clock ran out
        uint256 roll; // blocks after the arming
        uint256 warp; // seconds after the arming
        bool auto_;
        uint8 n; // panel size 3..6
        uint8 partyIn; // 0 none, 1 buyer in the panel, 2 seller in the panel
    }

    function _case(P memory p, address router) internal returns (Eravon inst, bytes32 id, Eravon.OrderTerms memory t) {
        Q memory q = defaultQuote(address(tok), address(0), p.auto_);
        bytes memory quote = encodeQuote(q);
        delete members;
        delete words;
        for (uint256 i = 0; i < p.n; i++) {
            members.push(address(uint160(0x001000000000000000000000000000000000000A01 + i)));
            words.push(AMANAH);
        }
        if (p.partyIn == 1) members[p.n - 1] = BUYER;
        if (p.partyIn == 2) members[0] = SELLER;
        if (p.rk == 2) router = address(new EchoRouter(members));
        vm.prank(GUILD, GUILD);
        pbook.engage(T, members, words, ROSTER, uint64(vm.getBlockTimestamp() + 30 days), router);
        vm.roll(vm.getBlockNumber() + 1);
        t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        inst = deployProxy(t);
        tok.mint(address(inst), A);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, words);
        id = orderHash(t);
    }

    function _toPath(Eravon inst, bytes32 id, ModeRouter mr, uint8 path) internal returns (bool) {
        if (path == 0) return inst.status().pendingRedraw;
        if (address(mr) != address(0)) mr.set(0, 0);
        vm.roll(vm.getBlockNumber() + 1);
        vm.prank(SELLER);
        inst.redraw(members, words);
        if (inst.status().seats == 0) return false;
        address j = inst.seatOf(id, 0).judge;
        if (path == 1) {
            bytes32 e = sha256(ENTRY);
            vm.prank(j, j);
            (bool ok,) = address(inst).call(abi.encodeCall(Eravon.resign, (id, 0, e, ENTRY)));
            if (!ok) return false;
        } else {
            if (inst.seatOf(id, 0).accepted != 0) return false;
            vm.warp(uint256(inst.seatOf(id, 0).windowEnd) + 1);
            inst.advance(id, 0);
        }
        return inst.status().pendingRedraw;
    }

    function _fingerprint(Eravon inst, bytes32 id, address sender, bool ok, bytes memory ret, uint256 senderBal0)
        internal
        returns (bytes32 fp)
    {
        Vm.Log[] memory logs = vm.getRecordedLogs();
        bytes32 lh;
        for (uint256 i = 0; i < logs.length; i++) {
            lh = keccak256(abi.encode(lh, logs[i].emitter, logs[i].topics, logs[i].data));
        }
        Eravon.CaseStatus memory st = inst.status();
        bytes32 sh = keccak256(abi.encode(st));
        for (uint256 s = 0; s < st.seats; s++) sh = keccak256(abi.encode(sh, inst.seatOf(id, uint64(s))));
        (address a, address b, bool c, uint16 d, address e, bytes32 f) = inst.stage();
        sh = keccak256(abi.encode(sh, a, b, c, d, e, f));
        bytes32 bh;
        for (uint256 i = 0; i < watch.length; i++) {
            bh = keccak256(abi.encode(bh, watch[i], tok.balanceOf(watch[i])));
        }
        bool watched;
        for (uint256 i = 0; i < watch.length; i++) if (watch[i] == sender) watched = true;
        // a sender the case can pay is in the balances above; any other sender's balance must not move
        int256 senderDelta = watched ? int256(0) : int256(tok.balanceOf(sender)) - int256(senderBal0);
        fp = keccak256(abi.encode(ok, ret, lh, sh, bh, senderDelta));
    }

    function _land(Eravon inst, bytes32 id, address sender, address origin) internal returns (bytes32) {
        uint256 b0 = tok.balanceOf(sender);
        vm.recordLogs();
        vm.prank(sender, origin);
        (bool ok, bytes memory ret) = address(inst).call(abi.encodeCall(Eravon.redraw, (members, words)));
        return _fingerprint(inst, id, sender, ok, ret, b0);
    }

    function testFuzz_R27_theDrawIsOneWhoeverSendsIt(
        uint8 rk,
        uint8 mode,
        uint256 at,
        uint8 path,
        uint256 roll,
        uint256 warp,
        bool auto_,
        uint8 n,
        uint8 partyIn,
        address stranger,
        address origin
    ) public {
        P memory p;
        p.rk = rk % 4;
        p.mode = mode % 8;
        p.at = at % 8;
        p.path = path % 3;
        p.roll = 1 + roll % 300;
        p.warp = warp % 5 days;
        p.auto_ = auto_;
        p.n = uint8(3 + n % 4);
        p.partyIn = partyIn % 3;
        vm.assume(stranger != address(0) && uint160(stranger) > 0xffff);
        vm.assume(origin != address(0));
        _run(p, stranger, origin);
    }

    /// The grid: every router kind and mode, every path, the block after the arming, a later block, the last block
    /// that still serves the seed and a block that no longer does, inside and past the deadline, with and without
    /// autoAccept; every outcome class must occur (seated, Resigned form 3, NotPendingRedraw) and the senders agree.
    function test_R27_theGridOfDraws() public {
        uint256[4] memory rolls = [uint256(1), 2, 256, 300];
        uint256[2] memory warps = [uint256(0), 4 days];
        uint256[4] memory got;
        for (uint8 r = 0; r < 11; r++) {
            for (uint8 path = 0; path < 3; path++) {
                for (uint256 x = 0; x < 4; x++) {
                    for (uint256 y = 0; y < 2; y++) {
                        for (uint256 z = 0; z < 2; z++) {
                            P memory p;
                            p.rk = r == 0 ? 0 : (r <= 8 ? 1 : (r == 9 ? 2 : 3));
                            p.mode = r >= 1 && r <= 8 ? r - 1 : 0;
                            p.at = 1;
                            p.path = path;
                            p.roll = rolls[x];
                            p.warp = warps[y];
                            p.auto_ = z == 1;
                            p.n = uint8(3 + (r + path) % 4);
                            p.partyIn = uint8((r + x) % 3);
                            uint256 s0 = vm.snapshotState();
                            got[_run(p, address(0xBEEF00), address(0x0919))]++;
                            vm.revertToState(s0);
                        }
                    }
                }
            }
        }
        emit log_named_uint("not armed on the path", got[0]);
        emit log_named_uint("refused (NotPendingRedraw or other)", got[1]);
        emit log_named_uint("seated", got[2]);
        emit log_named_uint("Resigned form 3", got[3]);
        assertGt(got[1], 0, "refusals occur");
        assertGt(got[2], 0, "seats occur");
        assertGt(got[3], 0, "form 3 occurs");
    }

    /// 0 no draw armed on the path, 1 the draw refused, 2 a seat opened, 3 the case ended Resigned.
    function _run(P memory p, address stranger, address origin) internal returns (uint8 cls) {
        ModeRouter mr;
        address router;
        if (p.rk == 1) {
            mr = new ModeRouter();
            router = address(mr);
        } else if (p.rk == 3) {
            router = address(new FirstEligibleRouter());
        }
        (Eravon inst, bytes32 id, Eravon.OrderTerms memory t) = _case(p, router);
        if (!_toPath(inst, id, mr, p.path)) return 0;
        if (address(mr) != address(0)) mr.set(p.mode, p.at);
        // one block for every sender: the arming block + roll, the arming time + warp
        Eravon.CaseStatus memory st = inst.status();
        vm.roll(uint256(st.drawBlock) + p.roll);
        vm.warp(vm.getBlockTimestamp() + p.warp);
        delete watch;
        watch.push(address(inst));
        watch.push(BUYER_PAYOUT);
        watch.push(SELLER_PAYOUT);
        watch.push(SELLER);
        watch.push(BUYER);
        watch.push(GUILD);
        watch.push(t.offer.sellerFund);
        for (uint256 i = 0; i < members.length; i++) watch.push(members[i]);

        address[12] memory who;
        who[0] = SELLER;
        who[1] = BUYER;
        who[2] = GUILD;
        who[3] = stranger;
        who[4] = members[1];
        who[5] = st.seats == 0 ? members[2] : inst.seatOf(id, 0).judge;
        who[6] = address(inst);
        who[7] = address(logic);
        who[8] = address(verifier);
        who[9] = router == address(0) ? address(pbook) : router;
        who[10] = BUYER_PAYOUT;
        who[11] = address(tok);
        uint256 seats0 = st.seats;
        uint256 snap = vm.snapshotState();
        bytes32 first = _land(inst, id, who[0], who[0]);
        Eravon.CaseStatus memory st1 = inst.status();
        if (st1.pendingRedraw == st.pendingRedraw && st1.seats == seats0 && st1.phase == st.phase) cls = 1;
        else if (st1.seats > seats0) cls = 2;
        else cls = 3;
        vm.revertToState(snap);
        for (uint256 k = 1; k < 12; k++) {
            bytes32 fp = _land(inst, id, who[k], k == 3 ? origin : who[k]);
            assertEq(fp, first, string.concat("the post-state differs for sender class ", vm.toString(k)));
            vm.revertToState(snap);
        }
        // the origin alone differs: a party sends with a stranger's origin
        assertEq(_land(inst, id, who[1], origin), first, "the origin changes the draw");
        vm.revertToState(snap);
    }
}
