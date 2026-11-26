// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Vm} from "forge-std/Vm.sol";
import {EravonBase} from "../EravonBase.sol";
import {Eravon} from "../../src/Eravon.sol";
import {EravonVerifier} from "../../src/EravonVerifier.sol";
import {EravonPanelBook} from "../../src/EravonPanelBook.sol";
import {IGaleedVenue, IGaleedRouter, RouteView} from "../../src/IGaleedVenue.sol";
import {FixedRouter} from "../replay/ReplayRouters.sol";
import {GaleedSuccessionBook} from "../replay/GaleedSuccessionBook.sol";
import {MockERC20} from "../replay/MockERC20.sol";
import {ExactGasRouter} from "../fuzz24/Eravon.fuzz24.t.sol";

/// The sentences the round-1 fold of the revision of 2026-10-24 wrote into the standard (FUZZ, round 2 of that
/// generation), each measured on the compiled bytes, and fresh stateless angles.

/// A router whose answer turns on one field of the block or the transaction (standard s7: a router's answer may turn
/// on anything the block and the transaction carry beyond the seed).
contract EnvRouter {
    uint8 public mode;

    function set(uint8 m) external {
        mode = m;
    }

    function route(RouteView calldata v) external view returns (uint256) {
        return uint256(keccak256(abi.encode(envWord(mode)))) % v.members.length;
    }

    function envWord(uint8 md) public view returns (uint256 x) {
        if (md == 0) x = block.timestamp;
        else if (md == 1) x = block.number;
        else if (md == 2) x = block.prevrandao;
        else if (md == 3) x = uint160(address(block.coinbase));
        else if (md == 4) x = block.basefee;
        else if (md == 5) x = uint160(tx.origin);
        else x = tx.gasprice;
    }
}

/// Forwards a call from its own address (an intermediary, s9).
contract Forwarder {
    function fwd(address to, bytes calldata data) external returns (bool ok, bytes memory ret) {
        (ok, ret) = to.call(data);
    }
}

abstract contract R2Base is EravonBase {
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("guild covenant");
    uint256 internal constant A = 100 ether;
    bytes32 internal T;
    Q internal q;
    bytes internal quote;

    function _base() internal {
        setUpBase();
        T = logic.PROXY_CODEHASH();
        q = defaultQuote(address(tok), address(0), false);
        quote = encodeQuote(q);
    }

    function _members(uint256 n, uint256 tag) internal pure returns (address[] memory m) {
        m = new address[](n);
        for (uint256 i = 0; i < n; i++) m[i] = address(uint160(0x0010000000000000000000000000000000000000) + uint160(tag << 16) + uint160(i + 1));
    }

    function _words(uint256 n) internal pure returns (bytes32[] memory w) {
        w = new bytes32[](n);
        for (uint256 i = 0; i < n; i++) w[i] = AMANAH;
    }

    function _engage(address[] memory m, address router) internal returns (bytes32[] memory w) {
        w = _words(m.length);
        vm.prank(GUILD, GUILD);
        pbook.engage(T, m, w, keccak256("roster"), uint64(vm.getBlockTimestamp() + 30 days), router);
        vm.roll(vm.getBlockNumber() + 1);
    }

    function _open(address[] memory m, bytes32[] memory w) internal returns (Eravon inst, Eravon.OrderTerms memory t) {
        t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        inst = deployProxy(t);
        tok.mint(address(inst), A);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), m, w);
    }

    function _state(Eravon inst) internal view returns (bytes32) {
        Eravon.CaseStatus memory st = inst.status();
        return keccak256(abi.encode(st, tok.balanceOf(address(inst)), tok.balanceOf(BUYER_PAYOUT), tok.balanceOf(SELLER)));
    }

    function _sel(bytes memory ret) internal pure returns (bytes4 s) {
        if (ret.length >= 4) s = bytes4(ret);
    }
}

// =====================================================================================================================
// 1. RouterGas: the exact threshold the fold wrote (s7), read at the comparison in the bytecode, and exactly routeGas()
// =====================================================================================================================

contract EravonR2RouterGas is R2Base {
    uint256 internal constant TH = 1_000_000 + uint256(1_000_000) / 63 + 20_000; // 1,035,873

    Eravon internal inst;
    address[] internal mem;
    bytes32[] internal wd;
    bytes internal data;
    ExactGasRouter internal r;
    uint256 internal lowest;
    bytes32 internal before;

    function setUp() public {
        _base();
        mem = _members(3, 1);
        r = new ExactGasRouter();
        RouteView memory v;
        v.members = mem;
        v.parties = new address[](5);
        v.seated = new address[](0);
        (bool ok, bytes memory ret) = address(r).staticcall{gas: logic.routeGas()}(abi.encodeCall(ExactGasRouter.route, (v)));
        require(ok && ret.length == 32, "calibration");
        r.setExpect(abi.decode(ret, (uint256)));
        wd = _engage(mem, address(r));
        (inst,) = _open(mem, wd);
        vm.roll(vm.getBlockNumber() + 1);
        data = abi.encodeCall(Eravon.redraw, (mem, wd));
        before = _state(inst);
        uint256 lo = 30_000;
        uint256 hi = 20_000_000;
        while (hi - lo > 1) {
            uint256 mid = (lo + hi) / 2;
            uint256 snap = vm.snapshotState();
            vm.prank(SELLER);
            (bool ok2,) = address(inst).call{gas: mid}(data);
            vm.revertToStateAndDelete(snap);
            if (ok2) hi = mid;
            else lo = mid;
        }
        lowest = hi;
    }

    /// The compare against the threshold in the trace: the gas value it reads and whether the STATICCALL to the
    /// router followed with exactly routeGas() as its gas argument.
    function _read(Vm.DebugStep[] memory steps) internal view returns (bool found, uint256 left, bool called, uint256 cmp) {
        for (uint256 i = 0; i < steps.length; i++) {
            Vm.DebugStep memory s = steps[i];
            if (!found && (s.opcode == 0x10 || s.opcode == 0x11) && s.stack.length >= 2) {
                if (s.stack[1] == TH) {
                    (found, left, cmp) = (true, s.stack[0], s.opcode);
                } else if (s.stack[0] == TH) {
                    (found, left, cmp) = (true, s.stack[1], s.opcode);
                }
            }
            if (s.opcode == 0xfa && s.stack.length >= 2 && s.stack[1] == uint256(uint160(address(r)))) {
                assertEq(s.stack[0], 1_000_000, "the STATICCALL's gas argument is routeGas()");
                called = true;
            }
        }
    }

    /// Every limit from 40 below the lowest that lands to 8 above it, traced: the comparison reads the gas left
    /// against exactly routeGas() + floor(routeGas() / 63) + 20,000; the call reverts RouterGas where the value
    /// read is not above it and lands, with the router handed exactly routeGas(), where it is.
    function _has(bytes memory code, bytes memory pat) internal pure returns (bool) {
        for (uint256 i = 0; i + pat.length <= code.length; i++) {
            bool eq = true;
            for (uint256 j = 0; j < pat.length && eq; j++) if (code[i + j] != pat[j]) eq = false;
            if (eq) return true;
        }
        return false;
    }

    /// The comparison as compiled (PUSH3 0x0fce61, GAS, GT, ISZERO: on to the revert where the gas read is not above
    /// the threshold); then, where forge runs a tracer (-vvv), the trace below. Without one, the bytecode reading alone.
    function test_R2_RouterGasThresholdIsExactAtTheCheck() public {
        assertTrue(_has(address(logic).code, hex"620fce615a1115"), "PUSH3 1035873 GAS GT ISZERO in the logic");
        bool traced = true;
        try vm.startDebugTraceRecording() {
            vm.stopAndReturnDebugTraceRecording();
        } catch {
            traced = false;
        }
        if (!traced) return;
        assertGt(lowest, TH, "the lowest landing limit exceeds the threshold");
        bool sawAt;
        bool sawAbove;
        for (uint256 g = lowest - 40; g <= lowest + 8; g++) {
            uint256 snap = vm.snapshotState();
            vm.startDebugTraceRecording();
            vm.prank(SELLER);
            (bool ok, bytes memory ret) = address(inst).call{gas: g}(data);
            Vm.DebugStep[] memory steps = vm.stopAndReturnDebugTraceRecording();
            (bool found, uint256 left, bool called, uint256 cmp) = _read(steps);
            if (found) {
                assertTrue(cmp == 0x10 || cmp == 0x11, "LT or GT");
                if (left <= TH) {
                    assertFalse(ok, "at or below the threshold nothing lands");
                    assertEq(_sel(ret), Eravon.RouterGas.selector, "RouterGas");
                    assertFalse(called, "the router is not called");
                    if (left == TH) sawAt = true;
                } else {
                    assertTrue(called, "above it the router is called");
                    assertTrue(ok, "above the threshold the frugal router's draw lands");
                    if (left == TH + 1) sawAbove = true;
                }
            } else {
                assertFalse(ok, "out of gas before the check");
            }
            if (ok) {
                assertEq(uint8(inst.status().phase), uint8(Eravon.Phase.Funded), "seated, the router handed exactly routeGas()");
                assertEq(inst.status().seats, 1, "one seat");
            } else {
                assertEq(_state(inst), before, "nothing moved");
            }
            vm.revertToStateAndDelete(snap);
        }
        assertTrue(sawAt, "a limit read exactly the threshold and met RouterGas");
        assertTrue(sawAbove, "a limit read one above it and landed");
        emit log_named_uint("lowest landing limit", lowest);
        emit log_named_uint("threshold", TH);
    }

    /// At any gas limit, the draw lands with the router handed exactly routeGas() (the calibrated router answers
    /// members[0] only then, and an index outside the panel otherwise, which would end the case), or refuses with
    /// nothing moved, by RouterGas or by running out.
    function testFuzz_R2_aLandedDrawHandsTheRouterExactlyRouteGas(uint256 g) public {
        g = bound(g, lowest - 60_000, lowest + 3_000_000);
        vm.prank(SELLER);
        (bool ok, bytes memory ret) = address(inst).call{gas: g}(data);
        if (ok) {
            assertEq(uint8(inst.status().phase), uint8(Eravon.Phase.Funded), "seated");
            assertEq(inst.status().seats, 1, "one seat");
            assertGe(g, lowest, "nothing lands below the lowest limit");
        } else {
            assertLt(g, lowest, "everything from the lowest limit on lands");
            assertTrue(ret.length == 0 || _sel(ret) == Eravon.RouterGas.selector, "RouterGas or out of gas");
            assertEq(_state(inst), before, "nothing moved");
        }
    }
}

// =====================================================================================================================
// 2. A panel's cost: init's read and a draw without a router grow with the square of the panel's length (s7)
// =====================================================================================================================

contract EravonR2PanelCost is R2Base {
    function setUp() public {
        _base();
    }

    /// A seed whose pick is the pool's `want`-th member at seat 0.
    function _seedFor(bytes32 id, uint256 n, uint256 want) internal pure returns (bytes32 s) {
        for (uint256 i = 1; ; i++) {
            s = keccak256(abi.encode("seed", i));
            if (uint256(keccak256(abi.encodePacked(id, s, uint64(0)))) % n == want) return s;
        }
    }

    function _measure(uint256 n, address router) internal returns (uint256 gInit, uint256 gDraw) {
        (gInit, gDraw,) = _measure3(n, router);
    }

    /// init's gas, then a draw's at the first pick and at the last.
    function _measure3(uint256 n, address router) internal returns (uint256 gInit, uint256 gFirst, uint256 gLast) {
        uint256 snap = vm.snapshotState();
        address[] memory m = _members(n, n);
        bytes32[] memory w = _engage(m, router);
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        uint256 g0 = gasleft();
        inst.init(t, quote, bs, ss, m, w);
        gInit = g0 - gasleft();
        uint256 armed = vm.getBlockNumber();
        vm.roll(armed + 1);
        bytes32 id = orderHash(t);
        for (uint256 k = 0; k < 2; k++) {
            uint256 s2 = vm.snapshotState();
            if (router == address(0)) vm.setBlockhash(armed, _seedFor(id, n, k == 0 ? 0 : n - 1));
            vm.prank(SELLER);
            g0 = gasleft();
            inst.redraw(m, w);
            uint256 used = g0 - gasleft();
            assertEq(inst.status().seats, 1, "seated");
            if (router == address(0)) assertEq(inst.seatOf(id, 0).judge, m[k == 0 ? 0 : n - 1], "the pick");
            if (k == 0) gFirst = used;
            else gLast = used;
            vm.revertToStateAndDelete(s2);
        }
        vm.revertToStateAndDelete(snap);
    }

    function test_R2_panelCostGrowsWithTheSquareOfItsLength() public {
        uint256[6] memory ns = [uint256(25), 50, 100, 200, 400, 500];
        uint256[6] memory iN;
        uint256[6] memory dF;
        uint256[6] memory dL;
        uint256[6] memory iR;
        uint256[6] memory dR;
        FixedRouter fr = new FixedRouter(0);
        for (uint256 k = 0; k < 6; k++) {
            (iN[k], dF[k], dL[k]) = _measure3(ns[k], address(0));
            (iR[k], dR[k],) = _measure3(ns[k], address(fr));
            string memory nn = string.concat("n ", vm.toString(ns[k]));
            emit log_named_uint(string.concat(nn, " init, no router"), iN[k]);
            emit log_named_uint(string.concat(nn, " draw, no router, first pick"), dF[k]);
            emit log_named_uint(string.concat(nn, " draw, no router, last pick"), dL[k]);
            emit log_named_uint(string.concat(nn, " init, router"), iR[k]);
            emit log_named_uint(string.concat(nn, " draw, router"), dR[k]);
        }
        // doubling n from 100 to 200 to 400: a quadratic's increments grow fourfold, a linear's twofold
        for (uint256 k = 3; k < 5; k++) {
            uint256[5] memory r5;
            r5[0] = ((iN[k] - iN[k - 1]) * 100) / (iN[k - 1] - iN[k - 2]);
            r5[1] = ((dF[k] - dF[k - 1]) * 100) / (dF[k - 1] - dF[k - 2]);
            r5[2] = ((dL[k] - dL[k - 1]) * 100) / (dL[k - 1] - dL[k - 2]);
            r5[3] = ((iR[k] - iR[k - 1]) * 100) / (iR[k - 1] - iR[k - 2]);
            r5[4] = ((dR[k] - dR[k - 1]) * 100) / (dR[k - 1] - dR[k - 2]);
            string memory nn = string.concat("x100 increment ratio to n ", vm.toString(ns[k]));
            emit log_named_uint(string.concat(nn, " init, no router"), r5[0]);
            emit log_named_uint(string.concat(nn, " draw, no router, first pick"), r5[1]);
            emit log_named_uint(string.concat(nn, " draw, no router, last pick"), r5[2]);
            emit log_named_uint(string.concat(nn, " init, router"), r5[3]);
            emit log_named_uint(string.concat(nn, " draw, router"), r5[4]);
            assertGt(r5[0], 330, "init without a router: quadratic");
            assertGt(r5[1], 330, "a draw without a router, first pick: quadratic");
            assertGt(r5[2], 330, "a draw without a router, last pick: quadratic");
            assertGt(r5[3], 330, "init under a router: the pool is counted, quadratic");
            assertLt(r5[4], 250, "a draw under a router: linear");
        }
    }
}

// =====================================================================================================================
// 3. A router may read the block and the transaction beyond the seed (s7)
// =====================================================================================================================

contract EravonR2RouterInputs is R2Base {
    Eravon internal inst;
    address[] internal mem;
    bytes32[] internal wd;
    EnvRouter internal er;
    uint256 internal t0;
    uint256 internal n0;
    bytes32 internal id;

    function setUp() public {
        _base();
        mem = _members(8, 3);
        er = new EnvRouter();
        wd = _engage(mem, address(er));
        Eravon.OrderTerms memory t;
        (inst, t) = _open(mem, wd);
        id = orderHash(t);
        t0 = vm.getBlockTimestamp();
        n0 = vm.getBlockNumber();
    }

    function _drawWith(uint8 md, uint256 k) internal returns (address seated, uint256 x) {
        uint256 snap = vm.snapshotState();
        er.set(md);
        vm.roll(n0 + 1 + (md == 1 ? k : 0));
        vm.warp(t0 + 12 + (md == 0 ? k : 0));
        if (md == 2) vm.prevrandao(bytes32(uint256(keccak256(abi.encode("r", k)))));
        if (md == 3) vm.coinbase(address(uint160(0xC0FFEE00 + k)));
        if (md == 4) vm.fee(7 + k);
        if (md == 6) vm.txGasPrice(9 + k);
        address origin = md == 5 ? address(uint160(0x0A1100 + k)) : SELLER;
        x = er.envWord(md);
        if (md == 5) x = uint160(origin);
        vm.prank(SELLER, origin);
        inst.redraw(mem, wd);
        assertEq(uint8(inst.status().phase), uint8(Eravon.Phase.Funded), "seated");
        seated = inst.seatOf(id, 0).judge;
        vm.revertToStateAndDelete(snap);
    }

    function test_R2_aRoutersAnswerTurnsOnTheBlockAndTheTransaction() public {
        string[7] memory names = ["timestamp", "number", "prevrandao", "coinbase", "basefee", "tx.origin", "gasprice"];
        for (uint8 md = 0; md < 7; md++) {
            address first;
            bool varied;
            for (uint256 k = 0; k < 24 && !varied; k++) {
                (address s, uint256 x) = _drawWith(md, k);
                assertEq(s, mem[uint256(keccak256(abi.encode(x))) % mem.length], string.concat("the member the field names: ", names[md]));
                if (k == 0) first = s;
                else if (s != first) varied = true;
            }
            assertTrue(varied, string.concat("whoever sets it chooses among the members: ", names[md]));
        }
    }
}

// =====================================================================================================================
// 4. withdraw: the folded payout rule (s8), every branch, under a moved key
// =====================================================================================================================

contract EravonR2Withdraw is R2Base {
    address internal constant J = address(0x1DD9E0001);
    address internal constant J2 = address(0x1DD9E0002);
    address internal constant R = address(0x1DD9E00AA);
    Eravon internal inst;
    bytes32 internal id;
    uint256 internal p;

    function setUp() public {
        _base();
        Q memory qa = defaultQuote(address(tok), address(0), true);
        q = qa;
        quote = encodeQuote(qa);
        Eravon.OrderTerms memory t = namedTerms(J, AMANAH, quote, A);
        fundDeposit(q, A);
        inst = openNamed(t, quote);
        id = orderHash(t);
        p = premiumOf(q, A);
    }

    function _release() internal {
        claim(inst, keccak256("manifest"));
        vm.prank(BUYER);
        inst.confirm();
        (, uint256 c) = inst.accountOf(J, address(tok));
        assertEq(c, p, "P claimable");
    }

    /// kind 0: none registered, a zero payout; 1: none, a nonzero payout; 2: registered, a zero payout; 3: registered,
    /// the registered one; 4: registered, another nonzero one; 5: none, a nonzero payout, nothing claimable.
    function testFuzz_R2_withdrawRegistersRefusesOrPays(uint8 kind, address other, bool moved) public {
        kind = uint8(bound(kind, 0, 5));
        vm.assume(other != address(0) && other != R && other.code.length == 0 && uint160(other) > 0x100);
        vm.assume(other != address(inst) && other != BUYER_PAYOUT && other != SELLER_PAYOUT && other != SELLER);
        if (kind != 5) _release();
        if (kind >= 2 && kind <= 4) {
            vm.prank(J);
            inst.setPayout(J, R);
        }
        address key = J;
        if (moved) {
            moveKey(J, J2);
            key = J2;
            vm.prank(J);
            vm.expectRevert(Eravon.NotCurrent.selector);
            inst.withdraw(address(tok), J, kind == 1 || kind == 5 ? other : R);
        }
        address arg = kind == 0 || kind == 2 ? address(0) : (kind == 3 ? R : other);
        uint256 rb = tok.balanceOf(R);
        uint256 ob = tok.balanceOf(other);
        if (kind == 0) {
            vm.prank(key);
            vm.expectRevert(Eravon.NoPayout.selector);
            inst.withdraw(address(tok), J, arg);
            (address reg,) = inst.accountOf(J, address(tok));
            assertEq(reg, address(0), "nothing registered");
            return;
        }
        if (kind == 4) {
            vm.prank(key);
            vm.expectRevert(Eravon.PayoutSet.selector);
            inst.withdraw(address(tok), J, arg);
            (address reg,) = inst.accountOf(J, address(tok));
            assertEq(reg, R, "the registration stands");
            return;
        }
        vm.prank(key);
        inst.withdraw(address(tok), J, arg);
        (address reg2, uint256 c2) = inst.accountOf(J, address(tok));
        assertEq(c2, 0, "nothing left claimable");
        if (kind == 1 || kind == 5) {
            assertEq(reg2, other, "the named payout registered");
            assertEq(tok.balanceOf(other) - ob, kind == 1 ? p : 0, "paid to it");
        } else {
            assertEq(reg2, R, "the registered payout, unchanged");
            assertEq(tok.balanceOf(R) - rb, p, "paid to the registered payout");
        }
        // after registration a zero payout and the registered one change nothing, another is refused
        vm.prank(key);
        inst.withdraw(address(tok), J, address(0));
        vm.prank(key);
        inst.withdraw(address(tok), J, reg2);
        vm.prank(key);
        vm.expectRevert(Eravon.PayoutSet.selector);
        inst.withdraw(address(tok), J, address(uint160(reg2) ^ 1));
        (address reg3,) = inst.accountOf(J, address(tok));
        assertEq(reg3, reg2, "registered once");
    }

    /// A payout that is the instance itself: the push delivers nothing (s8: TransferFailed), and nothing moves.
    function test_R2_withdrawToTheInstanceItselfIsTransferFailed() public {
        _release();
        vm.prank(J);
        vm.expectRevert(Eravon.TransferFailed.selector);
        inst.withdraw(address(tok), J, address(inst));
        (address reg, uint256 c) = inst.accountOf(J, address(tok));
        assertEq(reg, address(0), "the registration rolled back with the refusal");
        assertEq(c, p, "still claimable");
    }
}

// =====================================================================================================================
// 5. The party gates the fold listed (s9): every sender but a party's current key meets NotParty first
// =====================================================================================================================

contract EravonR2Gates is R2Base {
    address internal constant J = address(0x1DD9E0001);
    address internal constant S2 = address(0x5E2);
    address internal constant B2 = address(0xB02);
    bytes internal JUNK;
    address internal constant VM_ADDR = 0x7109709ECfa91a80626fF3989D68f67F5b1DD12D;
    address internal constant CONSOLE_ADDR = 0x000000000000000000636F6e736F6c652e6c6f67;

    function setUp() public {
        _base();
        JUNK = abi.encodePacked(bytes32(uint256(1)), bytes32(uint256(1)), uint8(27));
    }

    function _bad(address who, bool sellerMoved, bool buyerMoved) internal view returns (bool) {
        address cs = sellerMoved ? S2 : SELLER;
        address cb = buyerMoved ? B2 : BUYER;
        return who != cs && who != cb;
    }

    function _named(bool autoAccept) internal returns (Eravon inst, Eravon.OrderTerms memory t) {
        q = defaultQuote(address(tok), address(0), autoAccept);
        quote = encodeQuote(q);
        t = namedTerms(J, AMANAH, quote, A);
        fundDeposit(q, A);
        inst = openNamed(t, quote);
    }

    function _expect(address who, address target, bytes memory cd, bytes4 want, string memory what) internal {
        vm.prank(who);
        (bool ok, bytes memory ret) = target.call(cd);
        assertFalse(ok, what);
        assertEq(_sel(ret), want, what);
    }

    /// The four signed forms, settle and sweep: a sender other than either party's current key meets NotParty with
    /// a junk signature (the sender is tested before the signature); after a move the former key meets it too; the
    /// current key with the junk signature meets the signature's own fault.
    function testFuzz_R2_signedFormsSettleAndSweepTestTheSenderFirst(address who, uint8 form, bool moveS, bool moveB) public {
        form = uint8(bound(form, 0, 5));
        (Eravon inst,) = _named(true);
        if (form == 1 || form == 2) claim(inst, keccak256("manifest"));
        if (moveS) moveKey(SELLER, S2);
        if (moveB) moveKey(BUYER, B2);
        bytes memory cd;
        bytes4 sigFault;
        if (form == 0) (cd, sigFault) = (abi.encodeCall(Eravon.claimDeliveryBySig, (keccak256("m"), JUNK)), Eravon.BadSellerSig.selector);
        if (form == 1) (cd, sigFault) = (abi.encodeCall(Eravon.confirmBySig, (JUNK)), Eravon.BadBuyerSig.selector);
        if (form == 2) (cd, sigFault) = (abi.encodeCall(Eravon.disputeBySig, (JUNK)), Eravon.BadBuyerSig.selector);
        if (form == 3) (cd, sigFault) = (abi.encodeCall(Eravon.refundBySig, (JUNK)), Eravon.BadSellerSig.selector);
        if (form == 4) {
            cd = abi.encodeCall(Eravon.settle, (uint16(5000), vm.getBlockTimestamp() + 1 days, JUNK, JUNK));
            sigFault = Eravon.BadSellerSig.selector;
        }
        if (form == 5) cd = abi.encodeCall(Eravon.sweep, (BUYER_PAYOUT, address(tok)));
        vm.assume(who != J && who != VM_ADDR && who != CONSOLE_ADDR);
        if (_bad(who, moveS, moveB)) {
            _expect(who, address(inst), cd, Eravon.NotParty.selector, "a stranger: NotParty");
        }
        if (moveS) _expect(SELLER, address(inst), cd, Eravon.NotParty.selector, "the seller's former key: NotParty");
        if (moveB) _expect(BUYER, address(inst), cd, Eravon.NotParty.selector, "the buyer's former key: NotParty");
        address cur = moveS ? S2 : SELLER;
        if (form == 5) {
            vm.prank(cur);
            (bool ok,) = address(inst).call(cd);
            assertTrue(ok, "a party's current key sweeps");
        } else {
            _expect(cur, address(inst), cd, sigFault, "the current key: the signature's fault");
        }
    }

    /// sweep by a seated judge: the judge's current key passes the gate, its former key meets NotParty.
    function test_R2_sweepAdmitsTheSeatedJudgesCurrentKeyAlone() public {
        (Eravon inst,) = _named(true);
        bytes memory cd = abi.encodeCall(Eravon.sweep, (BUYER_PAYOUT, address(tok)));
        vm.prank(J);
        (bool ok,) = address(inst).call(cd);
        assertTrue(ok, "the seated judge");
        moveKey(J, address(0x1DD9E0002));
        _expect(J, address(inst), cd, Eravon.NotParty.selector, "the judge's former key");
        vm.prank(address(0x1DD9E0002));
        (ok,) = address(inst).call(cd);
        assertTrue(ok, "the judge's current key");
    }

    /// redraw is open to any sender (s7, s9): a stranger, the root's former key and the root's current key all reach
    /// the panel test (junk members: PanelMismatch); NotParty comes from no draw.
    function testFuzz_R2_redrawIsOpenToAnySender(address who, bool moveRoot) public {
        vm.assume(who != VM_ADDR && who != CONSOLE_ADDR);
        address[] memory m = _members(4, 9);
        bytes32[] memory w = _engage(m, address(0));
        (Eravon inst,) = _open(m, w);
        vm.roll(vm.getBlockNumber() + 1);
        address g2 = address(0x6000000000000000000000000000000000000002);
        if (moveRoot) moveKey(GUILD, g2);
        address[] memory junk = new address[](1);
        bytes32[] memory jw = new bytes32[](1);
        bytes memory cd = abi.encodeCall(Eravon.redraw, (junk, jw));
        address rootCur = moveRoot ? g2 : GUILD;
        _expect(who, address(inst), cd, Eravon.PanelMismatch.selector, "anyone reaches the panel test");
        if (moveRoot) _expect(GUILD, address(inst), cd, Eravon.PanelMismatch.selector, "the root's former key too");
        _expect(rootCur, address(inst), cd, Eravon.PanelMismatch.selector, "the root's current key reaches the panel test");
        _expect(BUYER, address(inst), cd, Eravon.PanelMismatch.selector, "the buyer reaches the panel test");
    }

    /// The expiry branch and init's caller rule: a stranger meets NotParty on the expiry branch, and on the open path
    /// NotParty where the order admits the buyer and NotSeller where it does not.
    function testFuzz_R2_initsCallerRule(address who, bool expired, bool buyerMayOpen) public {
        vm.assume(who != SELLER && who != BUYER && who != VM_ADDR && who != CONSOLE_ADDR);
        q = defaultQuote(address(tok), address(0), true);
        quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(J, AMANAH, quote, A);
        t.offer.buyerMayOpen = buyerMayOpen;
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        if (expired) vm.warp(uint256(t.openBy) + 1);
        bytes memory cd = abi.encodeCall(Eravon.init, (t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0)));
        bytes4 want = expired ? Eravon.NotParty.selector : (buyerMayOpen ? Eravon.NotParty.selector : Eravon.NotSeller.selector);
        _expect(who, address(inst), cd, want, "a stranger at init");
        if (!expired && !buyerMayOpen) _expect(BUYER, address(inst), cd, Eravon.NotSeller.selector, "the buyer where the order does not admit it");
    }

    /// cancel's signed form: a stranger with a junk signature meets NotParty (the sender before the signature); a
    /// party with the junk signature meets BadBuyerSig; the direct form is the buyer's current key alone (NotBuyer).
    function testFuzz_R2_cancelTestsTheSenderBeforeTheSignature(address who) public {
        vm.assume(who != SELLER && who != BUYER && who != VM_ADDR && who != CONSOLE_ADDR);
        q = defaultQuote(address(tok), address(0), true);
        quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(J, AMANAH, quote, A);
        Eravon inst = deployProxy(t);
        _expect(who, address(inst), abi.encodeCall(Eravon.cancel, (t, JUNK)), Eravon.NotParty.selector, "a stranger, signed form");
        _expect(SELLER, address(inst), abi.encodeCall(Eravon.cancel, (t, JUNK)), Eravon.BadBuyerSig.selector, "the seller, junk signature");
        _expect(who, address(inst), abi.encodeCall(Eravon.cancel, (t, bytes(""))), Eravon.NotBuyer.selector, "a stranger, direct form");
        _expect(SELLER, address(inst), abi.encodeCall(Eravon.cancel, (t, bytes(""))), Eravon.NotBuyer.selector, "the seller, direct form");
        vm.prank(SELLER);
        inst.cancel(t, cancelSig(t));
        assertEq(uint8(inst.status().phase), uint8(Eravon.Phase.Refunded), "the seller submits the buyer's signature");
    }

    /// revoke and consent read their caller as the s9 rule reads it: an intermediary is the caller, whatever the
    /// transaction's origin; the current key records under the party's name, the former key writes nothing.
    function test_R2_revokeAndConsentReadTheCallerNotTheOrigin() public {
        Forwarder f = new Forwarder();
        bytes32 h = keccak256("doc");
        bytes32 d = keccak256("digest");
        bytes memory cdr = abi.encodeCall(Eravon.revoke, (SELLER, h));
        bytes memory cdc = abi.encodeCall(Eravon.consent, (SELLER, d));
        vm.prank(SELLER, SELLER);
        (bool ok, bytes memory ret) = f.fwd(address(logic), cdr);
        assertFalse(ok, "revoke through an intermediary, the origin the seller's key");
        assertEq(_sel(ret), Eravon.NotCurrent.selector, "NotCurrent");
        vm.prank(SELLER, SELLER);
        (ok, ret) = f.fwd(address(logic), cdc);
        assertFalse(ok, "consent through an intermediary, the origin the seller's key");
        assertEq(_sel(ret), Eravon.NotCurrent.selector, "NotCurrent");
        assertFalse(logic.revoked(SELLER, h), "nothing revoked");
        assertFalse(logic.consented(SELLER, d), "nothing consented");
        moveKey(SELLER, S2);
        _expect(SELLER, address(logic), abi.encodeCall(Eravon.revoke, (SELLER, h)), Eravon.NotCurrent.selector, "former key revoke");
        _expect(SELLER, address(logic), abi.encodeCall(Eravon.consent, (SELLER, d)), Eravon.NotCurrent.selector, "former key consent");
        vm.prank(S2, S2);
        logic.revoke(SELLER, h);
        vm.prank(S2, S2);
        logic.consent(SELLER, d);
        assertTrue(logic.revoked(SELLER, h), "the successor revokes under the same name");
        assertTrue(logic.consented(SELLER, d), "the successor consents under the same name");
        assertFalse(logic.revoked(S2, h), "nothing under the successor's own address");
    }
}

// =====================================================================================================================
// 6. The succession book's address (Shape): the deployer's CREATE2 address of the book's initcode; before it stands,
//    every gate that reads it reverts; anyone deploys it through the deployer; then the gates read it
// =====================================================================================================================

contract EravonR2Book is EravonBase {
    function test_R2_theBookAddressIsTheDeployersCreate2AndAnyoneDeploysIt() public {
        bytes memory init = type(GaleedSuccessionBook).creationCode;
        assertEq(keccak256(init), 0x711aec6ad1a23808653588aeae574e048be00bbefd513646ed6785e4e465a397, "the book's initcode hash");
        assertEq(keccak256(type(GaleedSuccessionBook).runtimeCode), 0xdb3293b42187b8b7270e62d5eaafee45af02c9caf3a0c8fe376cb4195f2f6aca, "codeHash");
        assertEq(create2Address(keccak256("galeed/succession-book"), keccak256(init)), SUCCESSION, "the constant address");
        // a chain without the book: the deployer, the logic, the verifier and the panel book, no book
        vm.warp(1_800_000_000);
        vm.etch(DEPLOYER, DEPLOYER_CODE);
        logic = Eravon(deployThrough(keccak256("eravon/1"), type(Eravon).creationCode));
        verifier = EravonVerifier(deployThrough(keccak256("eravon/1/verify"), type(EravonVerifier).creationCode));
        pbook = EravonPanelBook(deployThrough(keccak256("eravon/1/panel"), type(EravonPanelBook).creationCode));
        tok = new MockERC20();
        SELLER = vm.addr(SELLER_PK);
        BUYER = vm.addr(BUYER_PK);
        assertEq(SUCCESSION.code.length, 0, "no book");
        assertEq(logic.successionBook(), SUCCESSION);
        // every gate that reads the book reverts: the logic's registries, the book's engage, an instance's gates
        vm.prank(SELLER);
        (bool ok,) = address(logic).call(abi.encodeCall(Eravon.revoke, (SELLER, keccak256("d"))));
        assertFalse(ok, "revoke reverts");
        vm.prank(SELLER);
        (ok,) = address(logic).call(abi.encodeCall(Eravon.consent, (SELLER, keccak256("d"))));
        assertFalse(ok, "consent reverts");
        address[] memory m = new address[](1);
        m[0] = address(0xA1);
        bytes32[] memory w = new bytes32[](1);
        w[0] = keccak256("w");
        vm.prank(address(0x6001));
        (ok,) = address(pbook).call(abi.encodeCall(EravonPanelBook.engage, (bytes32(0), m, w, bytes32(0), uint64(block.timestamp + 1 days), address(0))));
        assertFalse(ok, "engage reverts");
        Q memory qq = defaultQuote(address(tok), address(0), true);
        bytes memory quote = encodeQuote(qq);
        Eravon.OrderTerms memory t = namedTerms(address(0x1DD9E0001), keccak256("a"), quote, 1 ether);
        Eravon inst = deployProxy(t);
        vm.prank(SELLER);
        (ok,) = address(inst).call(abi.encodeCall(Eravon.init, (t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0))));
        assertFalse(ok, "init reverts");
        vm.prank(SELLER);
        (ok,) = address(inst).call(abi.encodeCall(Eravon.cancel, (t, cancelSig(t))));
        assertFalse(ok, "cancel's signed form reverts");
        // anyone deploys the book through the deployer, at the constant address, with the codeHash
        vm.prank(address(0xD0D0));
        address b = deployThrough(keccak256("galeed/succession-book"), init);
        assertEq(b, SUCCESSION, "deployed at the constant");
        assertEq(b.codehash, 0xdb3293b42187b8b7270e62d5eaafee45af02c9caf3a0c8fe376cb4195f2f6aca, "the codeHash");
        sbook = GaleedSuccessionBook(b);
        vm.prank(SELLER);
        logic.revoke(SELLER, keccak256("d"));
        assertTrue(logic.revoked(SELLER, keccak256("d")), "the gate reads it");
        vm.prank(address(0x6001));
        pbook.engage(bytes32(0), m, w, bytes32(0), uint64(block.timestamp + 1 days), address(0));
        tok.mint(SELLER, 100 ether);
        vm.prank(SELLER);
        tok.approve(address(logic), type(uint256).max);
        tok.mint(address(inst), 1 ether);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        assertEq(uint8(inst.status().phase), uint8(Eravon.Phase.Funded), "init reads it");
    }
}

// =====================================================================================================================
// 7. Stateless differentials on the verifier: the draw without a router (the pool rule, zero excluded), the router's
//    calldata, and the freeze of the latest earlier unexpired record (s7, CODEHASH: the verifier checks the book's code)
// =====================================================================================================================

contract EravonR2Verifier is R2Base {
    function setUp() public {
        _base();
    }

    function _alphabet(uint256 k) internal view returns (address) {
        k = k % 8;
        if (k == 0) return address(0);
        if (k == 1) return SELLER;
        if (k == 2) return BUYER;
        return address(uint160(0xA00 + k));
    }

    function testFuzz_R2_theDrawWithoutARouterIsThePoolRule(uint256 seedM, uint256 seedS, bytes32 caseId, bytes32 seed, uint64 seat) public view {
        vm.assume(seed != bytes32(0));
        uint256 n = 1 + (seedM % 12);
        address[] memory m = new address[](n);
        for (uint256 i = 0; i < n; i++) m[i] = _alphabet(uint256(keccak256(abi.encode(seedM, i))));
        uint256 ns = seedS % 4;
        address[] memory s = new address[](ns);
        for (uint256 i = 0; i < ns; i++) s[i] = _alphabet(uint256(keccak256(abi.encode(seedS, i))));
        bytes32[] memory w = _words(n);
        uint256[11] memory wv;
        wv[0] = uint256(caseId);
        wv[1] = seat;
        wv[6] = uint256(seed);
        wv[7] = uint256(keccak256(abi.encode(m, w)));
        wv[8] = 1;
        address[5] memory parties = [BUYER, SELLER, SELLER, BUYER_PAYOUT, SELLER_PAYOUT];
        (address got, bytes memory data) = verifier.draw(wv, parties, m, w, s);
        assertEq(data.length, 0, "no router calldata");
        // the model: distinct nonzero members, neither party's key, never seated, in their first place's order
        address[] memory pool = new address[](n);
        uint256 k = 0;
        for (uint256 i = 0; i < n; i++) {
            address a = m[i];
            if (a == address(0) || a == SELLER || a == BUYER) continue;
            bool dup = false;
            for (uint256 j = 0; j < i; j++) if (m[j] == a) dup = true;
            for (uint256 j = 0; j < ns; j++) if (s[j] == a) dup = true;
            if (!dup) pool[k++] = a;
        }
        address want = k == 0 ? address(0) : pool[uint256(keccak256(abi.encodePacked(caseId, seed, seat))) % k];
        assertEq(got, want, "the model's member");
        assertTrue(got != address(0) || k == 0, "zero only where the pool is spent");
    }

    function testFuzz_R2_theRoutersCalldataIsTheView(bytes32 caseId, bytes32 seed, uint64 seat, uint256 amount, uint64 rungs, address router) public view {
        vm.assume(seed != bytes32(0) && router != address(0));
        address[] memory m = _members(5, 7);
        bytes32[] memory w = _words(5);
        address[] memory s = new address[](1);
        s[0] = m[2];
        uint256[11] memory wv = [uint256(caseId), seat, uint256(keccak256("terms")), uint256(uint160(address(tok))), amount, rungs, uint256(seed), uint256(keccak256(abi.encode(m, w))), 1, uint256(uint160(router)), 0];
        address[5] memory parties = [BUYER, SELLER, address(0xF0), BUYER_PAYOUT, SELLER_PAYOUT];
        (address got, bytes memory data) = verifier.draw(wv, parties, m, w, s);
        assertEq(got, address(0), "no member: the router draws");
        RouteView memory v;
        v.caseId = caseId;
        v.seat = seat;
        v.termsHash = keccak256("terms");
        v.token = address(tok);
        v.amount = amount;
        v.rungs = rungs;
        v.parties = new address[](5);
        for (uint256 i = 0; i < 5; i++) v.parties[i] = parties[i];
        v.members = m;
        v.seated = s;
        v.seed = seed;
        v.blockTime = uint64(block.timestamp);
        v.chainId = block.chainid;
        v.venue = address(this);
        assertEq(keccak256(data), keccak256(abi.encodeCall(IGaleedRouter.route, (v))), "route(RouteView), field by field");
    }

    /// Records engaged at drawn blocks with drawn expiries; init freezes the latest one recorded before its block and
    /// unexpired at its second (expires >= now), and refuses NoPanel where there is none.
    function testFuzz_R2_initFreezesTheLatestEarlierUnexpiredRecord(uint256 seedR, uint8 k) public {
        k = uint8(bound(k, 1, 6));
        address[] memory m = _members(3, 5);
        bytes32[] memory w = _words(3);
        uint64[] memory rec = new uint64[](k);
        uint64[] memory exp = new uint64[](k);
        uint256 initAt = vm.getBlockTimestamp() + 1000;
        for (uint256 i = 0; i < k; i++) {
            uint256 x = uint256(keccak256(abi.encode(seedR, i)));
            if (i != 0 && x % 3 == 0) {
                // the same block as the record before
            } else {
                vm.roll(vm.getBlockNumber() + 1 + (x % 2));
            }
            vm.warp(vm.getBlockTimestamp() + (x >> 8) % 3);
            uint64 e = uint64(initAt + 3 - ((x >> 16) % 7)); // around init's second: expired, exactly at it, or after
            if (e <= vm.getBlockTimestamp()) e = uint64(vm.getBlockTimestamp() + 1);
            vm.prank(GUILD, GUILD);
            pbook.engage(T, m, w, bytes32(i), e, address(0));
            rec[i] = uint64(vm.getBlockNumber());
            exp[i] = e;
        }
        if (uint256(keccak256(abi.encode(seedR, "same"))) % 3 != 0) vm.roll(vm.getBlockNumber() + 1);
        vm.warp(initAt);
        bool found;
        uint256 want;
        for (uint256 i = k; i != 0; i--) {
            if (rec[i - 1] < vm.getBlockNumber() && exp[i - 1] >= initAt) {
                (found, want) = (true, i - 1);
                break;
            }
        }
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        t.offer.expiry = initAt + 30 days;
        t.openBy = uint64(initAt + 1 days);
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        (bool ok, bytes memory ret) = address(inst).call(abi.encodeCall(Eravon.init, (t, quote, bs, ss, m, w)));
        if (!found) {
            assertFalse(ok, "no record: refused");
            assertEq(_sel(ret), Eravon.NoPanel.selector, "NoPanel");
        } else {
            assertTrue(ok, "frozen");
            assertEq(inst.status().panelIndex, want, "the latest earlier unexpired record");
        }
    }

    /// CODEHASH: the verifier checks the book's code hash before it reads the book.
    function test_R2_theVerifierChecksTheBooksCodeBeforeReadingIt() public {
        address[] memory m = _members(3, 6);
        bytes32[] memory w = _engage(m, address(0));
        vm.etch(address(pbook), hex"6000");
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.PanelBookMissing.selector);
        inst.init(t, quote, bs, ss, m, w);
    }
}

// =====================================================================================================================
// 8. What lies outside the Outcome (s8): the part of the seller's LF a feeBearer sends the buyer is no part of amount
//    and rulingFeeFrom books it as the seller's
// =====================================================================================================================

contract EravonR2Outcome is R2Base {
    address internal constant J = address(0x1DD9E0001);

    function testFuzz_R2_theBondLiesOutsideTheSharesAndIsTheSellersPartOfTheFee(uint8 rung, uint256 amount) public {
        _base();
        amount = bound(amount, 1 ether, 1_000_000 ether);
        rung = uint8(bound(rung, 0, 2));
        q = defaultQuote(address(tok), address(0), true);
        quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(J, AMANAH, quote, amount);
        fundDeposit(q, amount);
        Eravon inst = openNamed(t, quote);
        bytes32 id = orderHash(t);
        claim(inst, keccak256("manifest"));
        raise(inst);
        uint256 lf = lfOf(q, amount);
        IGaleedVenue.Outcome memory o = inst.outcomeOf(id, 0, rung, "");
        uint8 fb = threeRungs()[rung].feeBearer;
        uint256 bond = fb == 1 ? lf : (fb == 2 ? lf / 2 : 0);
        assertEq(o.shares[0] + o.shares[1], amount, "the shares are amount and nothing else");
        assertEq(o.rulingFee, lf, "the ruling fee is the buyer's fee");
        assertEq(o.rulingFeeFrom[1], bond, "the seller's part of the fee is the bond");
        assertEq(o.rulingFeeFrom[0] + o.rulingFeeFrom[1], o.rulingFee, "the fee borne in all");
        uint256 b0 = tok.balanceOf(BUYER_PAYOUT);
        vm.recordLogs();
        bytes32 eid = sha256("entry");
        vm.prank(J, J);
        inst.rule(id, 0, eid, rung, "entry", "");
        Vm.Log[] memory logs = vm.getRecordedLogs();
        uint256 bondLogged = type(uint256).max;
        uint256 buyerLogged;
        for (uint256 i = 0; i < logs.length; i++) {
            if (logs[i].topics[0] == Eravon.Closed.selector) {
                (, buyerLogged,,, bondLogged) = abi.decode(logs[i].data, (uint8, uint256, uint256, uint256, uint256));
            }
        }
        assertEq(bondLogged, bond, "Closed's bondToBuyer");
        assertEq(buyerLogged, o.shares[0] + bond, "the buyer: its share and the bond, outside the share");
        assertEq(tok.balanceOf(BUYER_PAYOUT) - b0, o.shares[0] + bond, "paid");
    }
}
