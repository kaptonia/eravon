// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {IGaleedVenue, RouteView} from "../src/IGaleedVenue.sol";
import {firstEligible, GasProbeRouter, GasGateRouter, FirstEligibleRouterB} from "./replay/ReplayRouters.sol";

/// A router whose answer the test sets between draws (the draw reaches it by
/// staticcall, which reads the mode and writes nothing): 0 the first member
/// neither seated nor offered, 1 a revert, 2 the index `at`, 3 the index the
/// whole view hashes to, 4 two words, 5 31 bytes, 6 every unit of gas it is
/// handed, 7 a storage write (refused under staticcall), 8 the index the
/// view's `blockTime` hashes to (a router that reads the block).
contract ModeRouter {
    uint8 public mode;
    uint256 public at;
    uint256 public writes;

    function set(uint8 m, uint256 a) external {
        mode = m;
        at = a;
    }

    function route(RouteView calldata v) external returns (uint256) {
        uint8 m = mode;
        if (m == 0) return firstEligible(v);
        if (m == 1) revert("refused");
        if (m == 2) return at;
        if (m == 3) return uint256(keccak256(abi.encode(v))) % v.members.length;
        if (m == 4) {
            assembly {
                mstore(0, 0)
                mstore(32, 0)
                return(0, 64)
            }
        }
        if (m == 5) {
            assembly {
                mstore(0, 0)
                return(0, 31)
            }
        }
        if (m == 6) {
            uint256 i;
            while (gasleft() > 0) i++;
            return i;
        }
        if (m == 8) return uint256(v.blockTime) % v.members.length;
        writes += 1;
        return firstEligible(v);
    }
}

/// Standard s7, the guild draw under a router, on each of the three paths that
/// arm a draw (init's first draw, the redraw after a decline, the redraw after
/// an offer clock ran out): the view a router is handed, field by field; the
/// seat for the member the router names; every way a router can fail (a
/// revert, an answer of another length, an index outside the panel or naming
/// a member a seat holds, a write, the gas run out, a code hash that changed
/// since the engagement) ending the case as Resigned with form 3, the amount
/// back to the buyer and the deposit to the seller; `RouterGas` where the gas
/// left cannot give the router `routeGas()`; and the gas sweep the replay's
/// step 14 runs over a creation, run here over each path's `redraw` (s12).
contract EravonRouter is EravonBase {
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("guild covenant");
    bytes32 internal constant ROSTER = keccak256("roster");
    bytes internal constant ENTRY = "resignation entry";
    uint256 internal constant A = 100 ether;
    bytes32 internal T;
    Q internal q;
    bytes internal quote;
    address[] internal members;
    bytes32[] internal words;

    function setUp() public {
        setUpBase();
        T = logic.PROXY_CODEHASH();
        q = defaultQuote(address(tok), address(0), false);
        quote = encodeQuote(q);
        for (uint256 i = 0; i < 4; i++) {
            members.push(address(uint160(0x001000000000000000000000000000000000000A01 + i)));
            words.push(AMANAH);
        }
    }

    // ---- the case ----

    /// A guild case under a panel naming `router`; the first draw armed, not yet made.
    function _case(address router) internal returns (Eravon inst, bytes32 id, Eravon.OrderTerms memory t) {
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

    /// Brings the case to the draw of path `p`, from the block that may draw: 0 the first draw, 1 the draw after
    /// seat 0 is declined, 2 the draw after seat 0's offer clock ran out (seat 0 drawn by mode 0).
    function _toPath(Eravon inst, bytes32 id, ModeRouter mr, uint8 p) internal {
        if (p != 0) {
            if (address(mr) != address(0)) mr.set(0, 0);
            vm.roll(vm.getBlockNumber() + 1);
            vm.prank(SELLER);
            inst.redraw(members, words);
            address j = inst.seatOf(id, 0).judge;
            if (p == 1) {
                bytes32 e = sha256(ENTRY);
                vm.prank(j, j);
                inst.resign(id, 0, e, ENTRY);
            } else {
                vm.warp(uint256(inst.seatOf(id, 0).windowEnd) + 1);
                inst.advance(id, 0);
            }
            assertTrue(inst.status().pendingRedraw, "a draw armed");
        }
        vm.roll(vm.getBlockNumber() + 1);
    }

    function _seated(Eravon inst, bytes32 id) internal view returns (address[] memory s) {
        uint256 n = inst.status().seats;
        s = new address[](n);
        for (uint256 i = 0; i < n; i++) s[i] = inst.seatOf(id, uint64(i)).judge;
    }

    /// The view s7 hands the router at this draw, built from the order, the frozen panel and the chain.
    function _view(Eravon inst, bytes32 id, Eravon.OrderTerms memory t) internal view returns (RouteView memory v) {
        Eravon.CaseStatus memory st = inst.status();
        v.caseId = id;
        v.seat = uint64(st.seats);
        v.termsHash = t.offer.termsHash;
        v.token = t.offer.token;
        v.amount = t.offer.amount;
        v.rungs = uint64(t.offer.ladder.length);
        v.parties = new address[](5);
        v.parties[0] = t.buyerKey;
        v.parties[1] = t.offer.sellerKey;
        v.parties[2] = t.offer.sellerFund;
        v.parties[3] = t.buyerPayout;
        v.parties[4] = t.offer.sellerPayout;
        v.members = members;
        v.seated = _seated(inst, id);
        v.seed = blockhash(st.drawBlock);
        v.blockTime = uint64(vm.getBlockTimestamp());
        v.chainId = block.chainid;
        v.venue = address(inst);
    }

    function _redraw(Eravon inst) internal {
        vm.prank(SELLER);
        inst.redraw(members, words);
    }

    /// The case ended for want of a usable panel: Resigned with form 3, no seat opened, the amount back to the
    /// buyer's payout and the deposit back to the seller's fund.
    function _assertForm3(Eravon inst, uint256 seatsBefore, uint256 buyer0, uint256 seller0, string memory what) internal view {
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned), string.concat("Resigned: ", what));
        assertEq(inst.status().seats, seatsBefore, string.concat("no seat opened: ", what));
        assertEq(tok.balanceOf(BUYER_PAYOUT) - buyer0, A, string.concat("the amount to the buyer: ", what));
        assertEq(tok.balanceOf(SELLER) - seller0, depositOf(q, A), string.concat("the deposit to the seller: ", what));
    }

    // ---- the view and the member ----

    function test_router_theViewFieldByFieldOnEveryPath() public {
        for (uint8 p = 0; p < 3; p++) {
            uint256 s0 = vm.snapshotState();
            ModeRouter mr = new ModeRouter();
            (Eravon inst, bytes32 id, Eravon.OrderTerms memory t) = _case(address(mr));
            _toPath(inst, id, mr, p);
            mr.set(3, 0);
            RouteView memory v = _view(inst, id, t);
            assertTrue(v.seed != bytes32(0), "the armed block's hash is served");
            uint256 idx = uint256(keccak256(abi.encode(v))) % members.length;
            bool taken = false;
            for (uint256 k = 0; k < v.seated.length; k++) if (v.seated[k] == members[idx]) taken = true;
            uint256 b0 = tok.balanceOf(BUYER_PAYOUT);
            uint256 sl0 = tok.balanceOf(SELLER);
            _redraw(inst);
            if (taken) {
                _assertForm3(inst, v.seat, b0, sl0, "the view's index names a seated member");
            } else {
                assertEq(inst.seatOf(id, v.seat).judge, members[idx], "the member the whole view names");
                assertEq(inst.seatOf(id, v.seat).amanahHash, AMANAH);
            }
            vm.revertToStateAndDelete(s0);
        }
    }

    function test_router_seatsTheMemberItNamesWhoeverItIs() public {
        // the router may name any member no seat holds, a party's key included (s7: the guild answers for its panel)
        members[3] = BUYER;
        ModeRouter mr = new ModeRouter();
        (Eravon inst, bytes32 id,) = _case(address(mr));
        _toPath(inst, id, mr, 0);
        mr.set(2, 3);
        _redraw(inst);
        assertEq(inst.seatOf(id, 0).judge, BUYER, "members[3], the buyer's key");
    }

    function test_router_aStrangerLandsTheDraw() public {
        // s7: redraw is open to any sender; the router's answer, not the sender, names the member, and the stranger
        // pays the gas
        ModeRouter mr = new ModeRouter();
        (Eravon inst, bytes32 id,) = _case(address(mr));
        _toPath(inst, id, mr, 0);
        mr.set(2, 2);
        vm.prank(address(0xBEEF));
        inst.redraw(members, words);
        assertEq(inst.seatOf(id, 0).judge, members[2], "members[2], landed by a stranger");
    }

    function test_router_theLeverIsTheBlockNotTheSender() public {
        // s7: without a router the block a stranger lands the draw in changes nothing; under a router that reads
        // `blockTime` the member follows the landing block's timestamp (the view's `blockTime` is that block's,
        // not the arming block's), so the choice is the block's, whoever sends
        for (uint8 r = 0; r < 2; r++) {
            uint256 s0 = vm.snapshotState();
            ModeRouter mr = r == 0 ? ModeRouter(address(0)) : new ModeRouter();
            (Eravon inst, bytes32 id,) = _case(address(mr));
            _toPath(inst, id, mr, 0);
            if (r == 1) mr.set(8, 0);
            uint256 b0 = inst.status().drawBlock;
            uint256 t0 = vm.getBlockTimestamp();
            address[2] memory got;
            for (uint256 k = 0; k < 2; k++) {
                uint256 s1 = vm.snapshotState();
                vm.roll(b0 + 1 + k);
                vm.warp(t0 + 12 + 13 * k);
                vm.prank(address(0xBEEF));
                inst.redraw(members, words);
                got[k] = inst.seatOf(id, 0).judge;
                if (r == 1) assertEq(got[k], members[(t0 + 12 + 13 * k) % members.length], "the landing block's timestamp");
                vm.revertToStateAndDelete(s1);
            }
            if (r == 0) assertEq(got[0], got[1], "no router: the block does not matter");
            else assertTrue(got[0] != got[1], "a blockTime router: the block chooses");
            vm.revertToStateAndDelete(s0);
        }
    }

    // ---- every failure ends the case with form 3 ----

    function _failsOnEveryPath(uint8 mode, uint256 at, bool replaceCode, string memory what) internal {
        for (uint8 p = 0; p < 3; p++) {
            uint256 s0 = vm.snapshotState();
            ModeRouter mr = new ModeRouter();
            (Eravon inst, bytes32 id,) = _case(address(mr));
            _toPath(inst, id, mr, p);
            mr.set(mode, at);
            if (replaceCode) {
                mr.set(0, 0);
                vm.etch(address(mr), vm.getDeployedCode("ReplayRouters.sol:FirstEligibleRouterB"));
            }
            uint256 seats = inst.status().seats;
            uint256 b0 = tok.balanceOf(BUYER_PAYOUT);
            uint256 sl0 = tok.balanceOf(SELLER);
            vm.expectEmit(true, true, true, true, address(inst));
            emit Eravon.Resigned(bytes32(0), 3);
            _redraw(inst);
            _assertForm3(inst, seats, b0, sl0, what);
            vm.revertToStateAndDelete(s0);
        }
    }

    function test_router_aRevertEndsTheCase() public {
        _failsOnEveryPath(1, 0, false, "a revert");
    }

    function test_router_anIndexOutsideThePanelEndsTheCase() public {
        _failsOnEveryPath(2, 4, false, "index 4 of four");
        _failsOnEveryPath(2, type(uint256).max - 1, false, "a huge index");
    }

    function test_router_aSeatedMemberEndsTheCase() public {
        // paths 1 and 2 only have a seated member; path 0 names members[0], which no seat holds yet
        for (uint8 p = 1; p < 3; p++) {
            uint256 s0 = vm.snapshotState();
            ModeRouter mr = new ModeRouter();
            (Eravon inst, bytes32 id,) = _case(address(mr));
            _toPath(inst, id, mr, p);
            address j0 = inst.seatOf(id, 0).judge;
            for (uint256 i = 0; i < members.length; i++) if (members[i] == j0) mr.set(2, i);
            uint256 b0 = tok.balanceOf(BUYER_PAYOUT);
            uint256 sl0 = tok.balanceOf(SELLER);
            _redraw(inst);
            _assertForm3(inst, 1, b0, sl0, "the member seat 0 holds");
            vm.revertToStateAndDelete(s0);
        }
    }

    function test_router_anAnswerOfAnotherLengthEndsTheCase() public {
        _failsOnEveryPath(4, 0, false, "two words");
        _failsOnEveryPath(5, 0, false, "31 bytes");
    }

    function test_router_aWriteEndsTheCase() public {
        _failsOnEveryPath(7, 0, false, "a write under staticcall");
    }

    function test_router_runningOutOfItsGasEndsTheCase() public {
        _failsOnEveryPath(6, 0, false, "every unit of routeGas spent");
    }

    function test_router_aCodeHashChangedSinceTheEngagementEndsTheCase() public {
        _failsOnEveryPath(0, 0, true, "the code replaced");
    }

    // ---- the gas ----

    /// The smallest gas limit at which the seller's `redraw` lands, between `lo` (not) and `hi` (lands).
    function _landsAt(Eravon inst, uint256 lo, uint256 hi) internal returns (uint256) {
        bytes memory data = abi.encodeCall(Eravon.redraw, (members, words));
        while (hi - lo > 1) {
            uint256 mid = (lo + hi) / 2;
            uint256 snap = vm.snapshotState();
            vm.prank(SELLER);
            (bool ok,) = address(inst).call{gas: mid}(data);
            vm.revertToStateAndDelete(snap);
            if (ok) hi = mid;
            else lo = mid;
        }
        return hi;
    }

    /// A probe that answers the first eligible member only where its entry gas is what exactly routeGas() leaves
    /// there, within 16 (its entry cost read from a calibration copy), off it the panel's last member, or (`gate`) a
    /// revert at once.
    function _exact(bool gate) internal returns (address) {
        uint256 rg = logic.routeGas();
        address cal = address(new GasProbeRouter(rg, 600));
        RouteView memory v;
        v.members = members;
        v.seated = new address[](0);
        v.parties = new address[](0);
        uint256 lo = rg;
        uint256 hi = rg + 600;
        while (hi - lo > 1) {
            uint256 mid = (lo + hi) / 2;
            (bool ok, bytes memory r) = cal.staticcall{gas: mid}(abi.encodeCall(GasProbeRouter.route, (v)));
            if (ok && abi.decode(r, (uint256)) == 0) lo = mid;
            else hi = mid;
        }
        uint256 entry = lo - rg;
        return gate ? address(new GasGateRouter(rg - entry + 8, 16)) : address(new GasProbeRouter(rg - entry + 8, 16));
    }

    function _state(Eravon inst, bytes32 id) internal view returns (bytes32) {
        Eravon.CaseStatus memory st = inst.status();
        bytes32 h = keccak256(abi.encode(st));
        if (st.seats != 0) h = keccak256(abi.encode(h, inst.seatOf(id, uint64(st.seats - 1))));
        return keccak256(abi.encode(h, tok.balanceOf(address(inst)), tok.balanceOf(BUYER_PAYOUT), tok.balanceOf(SELLER)));
    }

    /// Every limit from 50 below the lowest that lands to 400 above it: the redraw reverts with every effect it
    /// had, or lands with the router answered under routeGas() (the first eligible member seated).
    function _sweep(Eravon inst, bytes32 id, uint256 from, string memory what) internal {
        bytes memory data = abi.encodeCall(Eravon.redraw, (members, words));
        address want = members[_firstFree(inst, id)];
        bytes32 before = _state(inst, id);
        uint256 seats = inst.status().seats;
        for (uint256 g = from - 50; g <= from + 400; g++) {
            uint256 snap = vm.snapshotState();
            vm.prank(SELLER);
            (bool ok,) = address(inst).call{gas: g}(data);
            if (ok) {
                assertEq(inst.status().seats, seats + 1, string.concat("a landed draw opens the seat: ", what));
                assertEq(inst.seatOf(id, uint64(seats)).judge, want, string.concat("the router answered under routeGas(): ", what));
            } else {
                assertEq(_state(inst, id), before, string.concat("a refused draw leaves every effect undone: ", what));
            }
            vm.revertToStateAndDelete(snap);
        }
    }

    function _firstFree(Eravon inst, bytes32 id) internal view returns (uint256) {
        address[] memory s = _seated(inst, id);
        for (uint256 i = 0; i < members.length; i++) {
            bool taken = false;
            for (uint256 k = 0; k < s.length; k++) if (s[k] == members[i]) taken = true;
            if (!taken) return i;
        }
        revert("no member");
    }

    function test_router_gasSweepOverEveryPath() public {
        for (uint8 p = 0; p < 3; p++) {
            uint256 s0 = vm.snapshotState();
            // the path's seat 0 is drawn by a first-eligible router where the path needs one: the probe answers it
            address probe = _exact(false);
            (Eravon inst, bytes32 id,) = _case(probe);
            _toPathWithProbe(inst, id, p);
            uint256 at = _landsAt(inst, 30_000, 20_000_000);
            _sweep(inst, id, at, string.concat("the probe, path ", vm.toString(p)));
            vm.revertToStateAndDelete(s0);
        }
    }

    /// Under a router that reverts off routeGas(), every limit around the least at which a redraw whose router
    /// fails lands: a draw that would end the case for want of a member there is a router called short.
    function test_router_gasSweepAroundTheFailingDraw() public {
        for (uint8 p = 0; p < 3; p++) {
            uint256 s0 = vm.snapshotState();
            ModeRouter mr = new ModeRouter();
            (Eravon inst, bytes32 id,) = _case(address(mr));
            _toPath(inst, id, mr, p);
            mr.set(1, 0);
            uint256 failAt = _landsAt(inst, 30_000, 20_000_000);
            vm.revertToStateAndDelete(s0);
            s0 = vm.snapshotState();
            address gate = _exact(true);
            (inst, id,) = _case(gate);
            _toPathWithProbe(inst, id, p);
            _sweep(inst, id, failAt, string.concat("the gate around the failing draw, path ", vm.toString(p)));
            uint256 at = _landsAt(inst, 30_000, 20_000_000);
            _sweep(inst, id, at, string.concat("the gate, path ", vm.toString(p)));
            vm.revertToStateAndDelete(s0);
        }
    }

    /// `_toPath` for a probe router: seat 0 is drawn with the gas a plain call carries, so the probe answers it.
    function _toPathWithProbe(Eravon inst, bytes32 id, uint8 p) internal {
        _toPath(inst, id, ModeRouter(address(0)), p);
    }

    function test_router_RouterGasWhereTheGasLeftCannotGiveRouteGas() public {
        ModeRouter mr = new ModeRouter();
        (Eravon inst, bytes32 id,) = _case(address(mr));
        _toPath(inst, id, mr, 0);
        uint256 at = _landsAt(inst, 30_000, 20_000_000);
        assertGt(at, logic.routeGas(), "a draw needs more than routeGas()");
        bytes memory data = abi.encodeCall(Eravon.redraw, (members, words));
        bool seen = false;
        bytes32 before = _state(inst, id);
        for (uint256 g = at - 1; g + 200_000 > at && !seen; g -= 97) {
            vm.prank(SELLER);
            (bool ok, bytes memory ret) = address(inst).call{gas: g}(data);
            assertFalse(ok, "below the lowest limit nothing lands");
            if (ret.length == 4 && bytes4(ret) == Eravon.RouterGas.selector) seen = true;
            assertEq(_state(inst, id), before, "nothing moved");
        }
        assertTrue(seen, "RouterGas below the limit");
    }
}
