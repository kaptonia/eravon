// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {FixedRouter, RevertRouter} from "./replay/ReplayRouters.sol";

/// Standard s4a and s7: a sale delivered in stages, each stage its own order and instance, chained by the orders'
/// `prev`. `init` opens a later stage only once the stage at `prev` is an instance of this standard with the same
/// keys that ended leaving the buyer no share (BadPrev, StageOpen); `cancel` frees a later stage's money only once
/// the stage at `prev` ended leaving the buyer a share (StageHeld); stages cancel in order; a `prev` that is no
/// instance opens nothing and is cancelled by nobody; the expiry branch runs whatever the stage before did; and
/// where a guild names no router the draw seats the series judge first, the judge who last heard a stage of the
/// sale, handed on at init through `stage()`, a Lapsed stage carrying none forward; an account that carries the
/// proxy's code but proves no address is no instance.
contract EravonStages is EravonBase {
    address internal constant J = address(0x1DD9E0001);
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("guild covenant");
    uint256 internal constant A = 100 ether;
    Q internal q;
    bytes internal quote;

    function setUp() public {
        setUpBase();
        q = defaultQuote(address(tok), address(0), true);
        quote = encodeQuote(q);
    }

    /// A named-track stage whose order names `prev`.
    function _stage(address prev) internal returns (Eravon.OrderTerms memory t) {
        t = namedTerms(J, AMANAH, quote, A);
        t.prev = prev;
    }

    function _deployFunded(Eravon.OrderTerms memory t) internal returns (Eravon inst) {
        fundDeposit(q, A);
        inst = deployProxy(t);
        tok.mint(address(inst), A);
    }

    function _init(Eravon inst, Eravon.OrderTerms memory t) internal returns (bool ok, bytes memory ret) {
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        (ok, ret) = address(inst).call(abi.encodeCall(Eravon.init, (t, quote, bs, ss, new address[](0), new bytes32[](0))));
    }

    function _cancel(Eravon inst, Eravon.OrderTerms memory t) internal returns (bool ok, bytes memory ret) {
        vm.prank(BUYER);
        (ok, ret) = address(inst).call(abi.encodeCall(Eravon.cancel, (t, bytes(""))));
    }

    /// The proxy's runtime: its initcode (s1) less the ten-byte prefix that returns it.
    function proxyRuntime() internal view returns (bytes memory r) {
        bytes memory ic = proxyInitcode();
        r = new bytes(ic.length - 10);
        for (uint256 i = 0; i < r.length; i++) {
            r[i] = ic[i + 10];
        }
    }

    function _sel(bytes memory ret) internal pure returns (bytes4 s) {
        if (ret.length >= 4) s = bytes4(ret);
    }

    /// Stage 1 opened and released (confirmed): the buyer's share is zero.
    function _releasedStage() internal returns (Eravon inst, Eravon.OrderTerms memory t) {
        t = _stage(address(0));
        fundDeposit(q, A);
        inst = openNamed(t, quote);
        claim(inst, keccak256("m"));
        vm.prank(BUYER);
        inst.confirm();
        (,, bool ended, uint16 bps, address j,) = inst.stage();
        assertTrue(ended, "ended");
        assertEq(bps, 0, "buyerBps 0 at Released");
        assertEq(j, J, "the last seat's judge");
    }

    // ---- the view ----

    function test_stage_viewBeforeAnyCase() public {
        Eravon.OrderTerms memory t = _stage(address(0));
        Eravon inst = deployProxy(t);
        (address sk, address bk, bool ended, uint16 bps, address j,) = inst.stage();
        assertEq(sk, address(0));
        assertEq(bk, address(0));
        assertFalse(ended);
        assertEq(bps, 0);
        assertEq(j, address(0));
    }

    function test_stage_viewInFlightAndAtEveryEnd() public {
        // in flight: ended false, bps 0
        Eravon.OrderTerms memory t = _stage(address(0));
        fundDeposit(q, A);
        Eravon inst = openNamed(t, quote);
        (address sk, address bk, bool ended, uint16 bps,,) = inst.stage();
        assertEq(sk, SELLER);
        assertEq(bk, BUYER);
        assertFalse(ended);
        assertEq(bps, 0);
        // Refunded by the seller: 10000
        vm.prank(SELLER);
        inst.refund();
        (,, ended, bps,,) = inst.stage();
        assertTrue(ended);
        assertEq(bps, 10000);
        // cancel: 10000
        t = _stage(address(0));
        inst = deployProxy(t);
        vm.prank(BUYER);
        inst.cancel(t, "");
        (,, ended, bps,,) = inst.stage();
        assertTrue(ended);
        assertEq(bps, 10000);
        // the expiry branch: 10000, the keys kept
        t = _stage(address(0));
        inst = deployProxy(t);
        tok.mint(address(inst), A);
        vm.warp(uint256(t.openBy) + 1);
        vm.prank(SELLER);
        inst.init(t, quote, "", "", new address[](0), new bytes32[](0));
        (sk, bk, ended, bps,,) = inst.stage();
        assertEq(sk, SELLER);
        assertTrue(ended);
        assertEq(bps, 10000);
        vm.warp(vm.getBlockTimestamp() - 2 days);
        // Settled at a share: the signed buyerBps
        t = _stage(address(0));
        fundDeposit(q, A);
        inst = openNamed(t, quote);
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes memory sb = sign(BUYER_PK, digestOf(address(inst), keccak256(abi.encode(TH_SETTLE, uint256(2500), exp))));
        bytes memory ss = sign(SELLER_PK, digestOf(address(inst), keccak256(abi.encode(TH_SETTLE, uint256(2500), exp))));
        vm.prank(SELLER);
        inst.settle(2500, exp, sb, ss);
        (,, ended, bps,,) = inst.stage();
        assertTrue(ended);
        assertEq(bps, 2500);
    }

    // ---- init's gate ----

    function test_stage_opensAfterTheStageBeforeIsPaid() public {
        (Eravon inst1,) = _releasedStage();
        Eravon.OrderTerms memory t2 = _stage(address(inst1));
        Eravon inst2 = _deployFunded(t2);
        (bool ok,) = _init(inst2, t2);
        assertTrue(ok, "stage 2 opens");
        assertEq(uint8(inst2.status().phase), uint8(Eravon.Phase.Funded));
    }

    function test_stage_refusedWhileTheStageBeforeIsInFlightOrFailed() public {
        Eravon.OrderTerms memory t1 = _stage(address(0));
        fundDeposit(q, A);
        Eravon inst1 = openNamed(t1, quote);
        Eravon.OrderTerms memory t2 = _stage(address(inst1));
        Eravon inst2 = _deployFunded(t2);
        (bool ok, bytes memory ret) = _init(inst2, t2);
        assertFalse(ok);
        assertEq(_sel(ret), Eravon.StageOpen.selector, "in flight: StageOpen");
        // the stage before fails the seller: a refund leaves the buyer the whole share
        vm.prank(SELLER);
        inst1.refund();
        (ok, ret) = _init(inst2, t2);
        assertFalse(ok);
        assertEq(_sel(ret), Eravon.StageOpen.selector, "failed: StageOpen");
        // and the buyer may now cancel stage 2
        (ok,) = _cancel(inst2, t2);
        assertTrue(ok, "cancel after the stage before failed");
        assertEq(uint8(inst2.status().phase), uint8(Eravon.Phase.Refunded));
    }

    function test_stage_badPrev() public {
        // a prev that is no instance: init BadPrev, cancel StageHeld, the expiry branch refunds
        Eravon.OrderTerms memory t = _stage(address(0xBAD));
        Eravon inst = _deployFunded(t);
        (bool ok, bytes memory ret) = _init(inst, t);
        assertFalse(ok);
        assertEq(_sel(ret), Eravon.BadPrev.selector, "no instance: BadPrev");
        (ok, ret) = _cancel(inst, t);
        assertFalse(ok);
        assertEq(_sel(ret), Eravon.StageHeld.selector, "no instance: StageHeld");
        uint256 b0 = tok.balanceOf(BUYER_PAYOUT);
        vm.warp(uint256(t.openBy) + 1);
        vm.prank(BUYER);
        inst.init(t, quote, "", "", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(BUYER_PAYOUT) - b0, A, "the expiry branch returns the money");
        vm.warp(vm.getBlockTimestamp() - 2 days);
        // a prev that is an unopened instance of this standard: no keys yet, BadPrev; cancel StageHeld
        Eravon.OrderTerms memory t0 = _stage(address(0));
        Eravon inst0 = deployProxy(t0);
        Eravon.OrderTerms memory t1 = _stage(address(inst0));
        Eravon inst1 = _deployFunded(t1);
        (ok, ret) = _init(inst1, t1);
        assertFalse(ok);
        assertEq(_sel(ret), Eravon.BadPrev.selector, "unopened: BadPrev");
        (ok, ret) = _cancel(inst1, t1);
        assertFalse(ok);
        assertEq(_sel(ret), Eravon.StageHeld.selector, "unopened: StageHeld");
        // a prev whose keys differ (another buyer): BadPrev
        (Eravon instR,) = _releasedStage();
        Eravon.OrderTerms memory tb = _stage(address(instR));
        tb.buyerKey = vm.addr(0xB2);
        Eravon instb = _deployFunded(tb);
        bytes memory bsig = sign(0xB2, digestOf(instanceOf(tb), orderHash(tb)));
        bytes memory ssig = sellerSig(tb);
        vm.prank(SELLER);
        (ok, ret) = address(instb).call(abi.encodeCall(Eravon.init, (tb, quote, bsig, ssig, new address[](0), new bytes32[](0))));
        assertFalse(ok);
        assertEq(_sel(ret), Eravon.BadPrev.selector, "another buyer: BadPrev");
        // a prev that is this very instance: no keys, BadPrev and StageHeld
        Eravon.OrderTerms memory ts = _stage(address(0));
        ts.prev = instanceOf(ts); // a fixed point is unreachable: computing the address changes it; use a near miss
        Eravon insts = deployProxy(ts);
        (ok, ret) = _init(insts, ts);
        assertFalse(ok);
        assertEq(_sel(ret), Eravon.BadPrev.selector, "self or no instance: BadPrev");
    }

    function test_stage_expiryBranchRunsWhateverTheStageBeforeDid() public {
        Eravon.OrderTerms memory t1 = _stage(address(0));
        fundDeposit(q, A);
        Eravon inst1 = openNamed(t1, quote);
        Eravon.OrderTerms memory t2 = _stage(address(inst1));
        Eravon inst2 = deployProxy(t2);
        tok.mint(address(inst2), A);
        uint256 b0 = tok.balanceOf(BUYER_PAYOUT);
        vm.warp(uint256(t2.openBy) + 1);
        vm.prank(BUYER);
        inst2.init(t2, quote, "", "", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(BUYER_PAYOUT) - b0, A, "openBy passed: the money comes back while stage 1 is in flight");
    }

    // ---- the terminal table, row by row ----

    function test_stage_settledAtZeroAndLapsedAtZeroOpenTheNext() public {
        // Settled at buyerBps 0
        Eravon.OrderTerms memory t1 = _stage(address(0));
        fundDeposit(q, A);
        Eravon inst1 = openNamed(t1, quote);
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 sh = keccak256(abi.encode(TH_SETTLE, uint256(0), exp));
        vm.prank(SELLER);
        inst1.settle(0, exp, sign(BUYER_PK, digestOf(address(inst1), sh)), sign(SELLER_PK, digestOf(address(inst1), sh)));
        Eravon.OrderTerms memory t2 = _stage(address(inst1));
        Eravon inst2 = _deployFunded(t2);
        (bool ok,) = _init(inst2, t2);
        assertTrue(ok, "Settled at 0 opens the next stage");
        // Lapsed at lapseDefault 0: the judge silent through T3
        Q memory q0 = q;
        q0.lapseDefault = 0;
        bytes memory quote0 = encodeQuote(q0);
        Eravon.OrderTerms memory t3 = namedTerms(J, AMANAH, quote0, A);
        fundDeposit(q0, A);
        Eravon inst3 = deployProxy(t3);
        tok.mint(address(inst3), A);
        vm.prank(SELLER);
        inst3.init(t3, quote0, buyerSig(t3), sellerSig(t3), new address[](0), new bytes32[](0));
        claim(inst3, keccak256("m"));
        raise(inst3);
        vm.warp(vm.getBlockTimestamp() + q0.rulingWindow + 1);
        inst3.advance(orderHash(t3), 0);
        assertEq(uint8(inst3.status().phase), uint8(Eravon.Phase.Lapsed));
        (,, bool ended, uint16 bps, address j,) = inst3.stage();
        assertTrue(ended);
        assertEq(bps, 0, "lapseDefault 0: the buyer's share is zero");
        assertEq(j, address(0), "a Lapsed stage carries no judge forward");
        Eravon.OrderTerms memory t4 = namedTerms(J, AMANAH, quote0, A);
        t4.prev = address(inst3);
        fundDeposit(q0, A);
        Eravon inst4 = deployProxy(t4);
        tok.mint(address(inst4), A);
        vm.prank(SELLER);
        inst4.init(t4, quote0, buyerSig(t4), sellerSig(t4), new address[](0), new bytes32[](0));
        assertEq(uint8(inst4.status().phase), uint8(Eravon.Phase.Funded), "Lapsed at 0 opens the next stage");
    }

    function test_stage_ruledAtTheFirstRungOpensAndAtAHigherRungFrees() public {
        for (uint64 rung = 0; rung < 2; rung++) {
            Eravon.OrderTerms memory t1 = _stage(address(0));
            fundDeposit(q, A);
            Eravon inst1 = openNamed(t1, quote);
            claim(inst1, keccak256("m"));
            raise(inst1);
            bytes32 id1 = orderHash(t1);
            bytes32 entryId = sha256("entry"); // a precompile call before the prank, not after it
            vm.prank(J, J);
            inst1.rule(id1, 0, entryId, rung, "entry", "");
            Eravon.OrderTerms memory t2 = _stage(address(inst1));
            Eravon inst2 = _deployFunded(t2);
            (bool ok, bytes memory ret) = _init(inst2, t2);
            if (rung == 0) {
                assertTrue(ok, "ruled at the first rung: the next opens");
            } else {
                assertFalse(ok);
                assertEq(_sel(ret), Eravon.StageOpen.selector, "ruled with a share to the buyer: StageOpen");
                (ok,) = _cancel(inst2, t2);
                assertTrue(ok, "and the buyer cancels");
            }
        }
    }

    function test_stage_resignedFrees() public {
        // a guild stage with no usable draw ends Resigned with form 3: the buyer's whole share
        address[] memory m = new address[](1);
        m[0] = SELLER;
        bytes32[] memory w = new bytes32[](1);
        w[0] = AMANAH;
        RevertRouter r = new RevertRouter();
        bytes32 key = logic.PROXY_CODEHASH();
        vm.prank(GUILD, GUILD);
        pbook.engage(key, m, w, keccak256("roster"), uint64(vm.getBlockTimestamp() + 30 days), address(r));
        vm.roll(vm.getBlockNumber() + 1);
        Eravon.OrderTerms memory t1 = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst1 = deployProxy(t1);
        tok.mint(address(inst1), A);
        vm.prank(SELLER);
        inst1.init(t1, quote, buyerSig(t1), sellerSig(t1), m, w);
        drawNext(inst1, m, w); // the router reverts: the case ends Resigned with form 3
        assertEq(uint8(inst1.status().phase), uint8(Eravon.Phase.Resigned));
        Eravon.OrderTerms memory t2 = _stage(address(inst1));
        Eravon inst2 = _deployFunded(t2);
        (bool ok, bytes memory ret) = _init(inst2, t2);
        assertFalse(ok);
        assertEq(_sel(ret), Eravon.StageOpen.selector);
        (ok,) = _cancel(inst2, t2);
        assertTrue(ok, "Resigned frees the next stage");
    }

    // ---- cancel in order ----

    function test_stage_chainCancelsInOrder() public {
        Eravon.OrderTerms memory t1 = _stage(address(0));
        fundDeposit(q, A);
        Eravon inst1 = openNamed(t1, quote);
        Eravon.OrderTerms memory t2 = _stage(address(inst1));
        Eravon inst2 = _deployFunded(t2);
        Eravon.OrderTerms memory t3 = _stage(address(inst2));
        Eravon inst3 = _deployFunded(t3);
        // nothing moves while stage 1 is in flight
        (bool ok, bytes memory ret) = _cancel(inst2, t2);
        assertFalse(ok);
        assertEq(_sel(ret), Eravon.StageHeld.selector, "stage 2 held while stage 1 is in flight");
        (ok, ret) = _cancel(inst3, t3);
        assertFalse(ok);
        assertEq(_sel(ret), Eravon.StageHeld.selector, "stage 3 held: stage 2 is not ended");
        // stage 1 fails the seller
        vm.prank(SELLER);
        inst1.refund();
        (ok, ret) = _cancel(inst3, t3);
        assertFalse(ok);
        assertEq(_sel(ret), Eravon.StageHeld.selector, "stage 3 still held: stage 2 is not ended");
        uint256 b0 = tok.balanceOf(BUYER_PAYOUT);
        (ok,) = _cancel(inst2, t2);
        assertTrue(ok, "stage 2 cancels");
        (ok,) = _cancel(inst3, t3);
        assertTrue(ok, "then stage 3");
        assertEq(tok.balanceOf(BUYER_PAYOUT) - b0, 2 * A, "both stages' money back");
        // and a later stage never opens over the cancelled one
        Eravon.OrderTerms memory t4 = _stage(address(inst3));
        Eravon inst4 = _deployFunded(t4);
        (ok, ret) = _init(inst4, t4);
        assertFalse(ok);
        assertEq(_sel(ret), Eravon.StageOpen.selector);
    }

    // ---- the draw without a router: the stage before's judge first ----

    function _guildStage(address prev, address[] memory m, bytes32[] memory w) internal returns (Eravon inst, Eravon.OrderTerms memory t) {
        t = guildTerms(GUILD, AMANAH, quote, A);
        t.prev = prev;
        fundDeposit(q, A);
        inst = deployProxy(t);
        tok.mint(address(inst), A);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), m, w);
    }

    function _panel(uint256 n) internal pure returns (address[] memory m, bytes32[] memory w) {
        m = new address[](n);
        w = new bytes32[](n);
        for (uint256 i = 0; i < n; i++) {
            m[i] = address(uint160(0x001000000000000000000000000000000000000B00) + uint160(i + 1));
            w[i] = AMANAH;
        }
    }

    function test_stage_drawSeatsTheStageBeforesJudge() public {
        // seats offered, not auto-accepted, so that the stage before's judge may decline the offer
        q = defaultQuote(address(tok), address(0), false);
        quote = encodeQuote(q);
        (address[] memory m, bytes32[] memory w) = _panel(6);
        engagePanel(GUILD, m, w, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst1, Eravon.OrderTerms memory t1) = _guildStage(address(0), m, w);
        drawNext(inst1, m, w);
        address j1 = inst1.seatOf(orderHash(t1), 0).judge;
        assertTrue(j1 != address(0), "stage 1 seated");
        bytes32 id1 = orderHash(t1);
        vm.prank(j1, j1);
        inst1.accept(id1, 0);
        // stage 1 released by the buyer's confirmation
        claim(inst1, keccak256("m"));
        vm.prank(BUYER);
        inst1.confirm();
        // stage 2 under the same panel: the draw seats j1 whatever the seed
        (Eravon inst2, Eravon.OrderTerms memory t2) = _guildStage(address(inst1), m, w);
        drawNext(inst2, m, w);
        assertEq(inst2.seatOf(orderHash(t2), 0).judge, j1, "the stage before's judge is seated first");
        // j1 declines: the next seat is the pool's pick, never j1 again
        bytes32 id2 = orderHash(t2);
        bytes32 declined = sha256("declined");
        vm.prank(j1, j1);
        inst2.resign(id2, 0, declined, "declined");
        drawNext(inst2, m, w);
        address j2 = inst2.seatOf(orderHash(t2), 1).judge;
        assertTrue(j2 != j1 && j2 != address(0), "a declined judge is not seated again");
        vm.prank(j2, j2);
        inst2.accept(id2, 1);
        // stage 3 after stage 2 is released: j2 (the last seat's judge) is seated
        claim(inst2, keccak256("m"));
        vm.prank(BUYER);
        inst2.confirm();
        (Eravon inst3, Eravon.OrderTerms memory t3) = _guildStage(address(inst2), m, w);
        drawNext(inst3, m, w);
        assertEq(inst3.seatOf(orderHash(t3), 0).judge, j2, "the last seat's judge, not the declined one");
    }

    function test_stage_drawFallsBackWhereTheJudgeLeftThePanelOrLapsed() public {
        (address[] memory m, bytes32[] memory w) = _panel(6);
        engagePanel(GUILD, m, w, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst1, Eravon.OrderTerms memory t1) = _guildStage(address(0), m, w);
        drawNext(inst1, m, w);
        address j1 = inst1.seatOf(orderHash(t1), 0).judge;
        claim(inst1, keccak256("m"));
        vm.prank(BUYER);
        inst1.confirm();
        // the guild re-engages without j1: the pick follows the model over the new pool
        address[] memory m2 = new address[](5);
        bytes32[] memory w2 = new bytes32[](5);
        uint256 k;
        for (uint256 i = 0; i < 6; i++) {
            if (m[i] == j1) continue;
            m2[k] = m[i];
            w2[k] = AMANAH;
            k++;
        }
        engagePanel(GUILD, m2, w2, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst2, Eravon.OrderTerms memory t2) = _guildStage(address(inst1), m2, w2);
        uint64 armed = inst2.status().drawBlock;
        drawNext(inst2, m2, w2);
        bytes32 seed = blockhash(armed);
        uint256 pick = uint256(keccak256(abi.encodePacked(orderHash(t2), seed, uint64(0)))) % 5;
        assertEq(inst2.seatOf(orderHash(t2), 0).judge, m2[pick], "j1 off the panel: the pool's pick");
        // a Lapsed stage carries no judge: stage 3 under the first panel again takes the pool's pick
        engagePanel(GUILD, m, w, uint64(vm.getBlockTimestamp() + 30 days));
        Q memory q0 = q;
        q0.lapseDefault = 0;
        bytes memory quote0 = encodeQuote(q0);
        Eravon.OrderTerms memory tL = guildTerms(GUILD, AMANAH, quote0, A);
        fundDeposit(q0, A);
        Eravon instL = deployProxy(tL);
        tok.mint(address(instL), A);
        vm.prank(SELLER);
        instL.init(tL, quote0, buyerSig(tL), sellerSig(tL), m, w);
        drawNext(instL, m, w);
        assertTrue(instL.seatOf(orderHash(tL), 0).judge != address(0), "seated");
        claim(instL, keccak256("m"));
        raise(instL);
        vm.warp(vm.getBlockTimestamp() + q0.rulingWindow + 1);
        instL.advance(orderHash(tL), 0);
        assertEq(uint8(instL.status().phase), uint8(Eravon.Phase.Lapsed));
        (,,,, address none,) = instL.stage();
        assertEq(none, address(0), "Lapsed: no judge carried");
        Eravon.OrderTerms memory t3 = guildTerms(GUILD, AMANAH, quote0, A);
        t3.prev = address(instL);
        fundDeposit(q0, A);
        Eravon inst3 = deployProxy(t3);
        tok.mint(address(inst3), A);
        vm.prank(SELLER);
        inst3.init(t3, quote0, buyerSig(t3), sellerSig(t3), m, w);
        armed = inst3.status().drawBlock;
        drawNext(inst3, m, w);
        seed = blockhash(armed);
        pick = uint256(keccak256(abi.encodePacked(orderHash(t3), seed, uint64(0)))) % 6;
        assertEq(inst3.seatOf(orderHash(t3), 0).judge, m[pick], "after a Lapsed stage: the pool's pick");
    }

    function test_stage_routerIsAskedWhereNamed() public {
        (address[] memory m, bytes32[] memory w) = _panel(4);
        FixedRouter r = new FixedRouter(2);
        bytes32 key = logic.PROXY_CODEHASH();
        vm.prank(GUILD, GUILD);
        pbook.engage(key, m, w, keccak256("roster"), uint64(vm.getBlockTimestamp() + 30 days), address(r));
        vm.roll(vm.getBlockNumber() + 1);
        (Eravon inst1, Eravon.OrderTerms memory t1) = _guildStage(address(0), m, w);
        drawNext(inst1, m, w);
        assertEq(inst1.seatOf(orderHash(t1), 0).judge, m[2]);
        claim(inst1, keccak256("m"));
        vm.prank(BUYER);
        inst1.confirm();
        FixedRouter r2 = new FixedRouter(3);
        vm.prank(GUILD, GUILD);
        pbook.engage(key, m, w, keccak256("roster"), uint64(vm.getBlockTimestamp() + 30 days), address(r2));
        vm.roll(vm.getBlockNumber() + 1);
        (Eravon inst2, Eravon.OrderTerms memory t2) = _guildStage(address(inst1), m, w);
        drawNext(inst2, m, w);
        assertEq(inst2.seatOf(orderHash(t2), 0).judge, m[3], "a router named: the router draws, the stage before's judge is no preference");
    }

    // ---- the gate's reads are bounded ----

    function test_stage_anotherCodeHashIsBadPrev() public {
        // an account that carries another code than the proxy's is no instance, whatever it answers (BadPrev)
        Eravon.OrderTerms memory t = _stage(address(verifier));
        Eravon inst = _deployFunded(t);
        (bool ok, bytes memory ret) = _init(inst, t);
        assertFalse(ok);
        assertEq(_sel(ret), Eravon.BadPrev.selector, "another contract's code: BadPrev");
    }

    // ---- the sixth word: the salt the instance proved its address under ----

    function test_stage_sixthWordIsTheSalt() public {
        Eravon.OrderTerms memory t = _stage(address(0));
        Eravon inst = deployProxy(t);
        (,,,,, bytes32 c0) = inst.stage();
        assertEq(c0, bytes32(0), "no case, no cancel, no expiry branch: zero");
        tok.mint(address(inst), A);
        fundDeposit(q, A);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        (,,,,, bytes32 c1) = inst.stage();
        assertEq(c1, orderHash(t), "after init: the order's salt");
        // the expiry branch records it too
        Eravon.OrderTerms memory te = _stage(address(0));
        Eravon inste = deployProxy(te);
        tok.mint(address(inste), A);
        vm.warp(uint256(te.openBy) + 1);
        vm.prank(BUYER);
        inste.init(te, quote, "", "", new address[](0), new bytes32[](0));
        (,, bool ended, uint16 bps,, bytes32 c2) = inste.stage();
        assertTrue(ended);
        assertEq(bps, 10000);
        assertEq(c2, orderHash(te), "after the expiry branch: the order's salt");
    }

    // ---- an account that carries the proxy's code but proves no address ----

    function test_stage_aForgedStorageAccountIsNoInstance() public {
        // the forger writes what a paid stage would hold (phase Released, share 0, the two keys, any salt) and
        // returns the proxy's runtime, so it carries proxyCodeHash; it is no instance under s1
        Eravon.OrderTerms memory t = _stage(address(0));
        uint256[] memory slots = new uint256[](4);
        uint256[] memory vals = new uint256[](4);
        slots[0] = 0;
        vals[0] = uint256(uint8(Eravon.Phase.Released)) | (uint256(1) << 8); // _phase, _stageBps = 0 + 1
        slots[1] = 2;
        vals[1] = uint256(uint160(t.offer.sellerKey));
        slots[2] = 3;
        vals[2] = uint256(uint160(t.buyerKey));
        slots[3] = 15;
        vals[3] = uint256(keccak256("any salt"));
        address forged = address(new StageForger(proxyRuntime(), slots, vals));
        assertEq(forged.codehash, logic.PROXY_CODEHASH(), "the forger carries the proxy's code hash");
        (address fsk,, bool fended, uint16 fbps,,) = Eravon(forged).stage();
        assertEq(fsk, t.offer.sellerKey);
        assertTrue(fended && fbps == 0, "it answers as a paid stage");
        (bool ok,,,,,) = verifier.stageOf(forged, logic.PROXY_CODEHASH());
        assertFalse(ok, "stageOf: no instance");
        Eravon.OrderTerms memory t2 = _stage(forged);
        Eravon inst2 = _deployFunded(t2);
        (bool o, bytes memory ret) = _init(inst2, t2);
        assertFalse(o);
        assertEq(_sel(ret), Eravon.BadPrev.selector, "init over a forged stage: BadPrev");
        // forged as a failed stage (share 10000): cancel is StageHeld all the same
        vals[0] = uint256(uint8(Eravon.Phase.Refunded)) | (uint256(10001) << 8);
        address forgedF = address(new StageForger(proxyRuntime(), slots, vals));
        Eravon.OrderTerms memory t3 = _stage(forgedF);
        Eravon inst3 = _deployFunded(t3);
        (o, ret) = _cancel(inst3, t3);
        assertFalse(o);
        assertEq(_sel(ret), Eravon.StageHeld.selector, "cancel over a forged stage: StageHeld");
    }

    // ---- cancel's faults in the order of s4: the address proves itself before the stage is read ----

    function test_stage_cancelWrongAddressBeforeStageHeld() public {
        Eravon.OrderTerms memory t1 = _stage(address(0));
        fundDeposit(q, A);
        Eravon inst1 = openNamed(t1, quote); // in flight: stage 2 is held
        Eravon.OrderTerms memory t2 = _stage(address(inst1));
        Eravon inst2 = _deployFunded(t2);
        (bool ok, bytes memory ret) = _cancel(inst2, t2);
        assertEq(_sel(ret), Eravon.StageHeld.selector, "the instance's own order: StageHeld");
        // `t2` is a memory struct: the two edits below are undone after each call
        t2.openBy += 1; // not this instance's order
        vm.prank(BUYER);
        (ok, ret) = address(inst2).call(abi.encodeCall(Eravon.cancel, (t2, bytes(""))));
        assertFalse(ok);
        assertEq(_sel(ret), Eravon.WrongAddress.selector, "a wrong-address cancel: WrongAddress before StageHeld");
        t2.openBy -= 1;
        address bk = t2.buyerKey;
        t2.buyerKey = address(0);
        vm.prank(BUYER);
        (ok, ret) = address(inst2).call(abi.encodeCall(Eravon.cancel, (t2, bytes(""))));
        assertEq(_sel(ret), Eravon.WrongAddress.selector, "a zero buyerKey is another order: WrongAddress first");
        t2.buyerKey = bk;
    }

    // ---- the stage before unopened: the buyer runs its expiry branch, then cancels the next ----

    function test_stage_expiryBranchOfTheStageBeforeFreesTheNext() public {
        Eravon.OrderTerms memory t1 = _stage(address(0));
        Eravon inst1 = deployProxy(t1);
        tok.mint(address(inst1), A);
        Eravon.OrderTerms memory t2 = _stage(address(inst1));
        Eravon inst2 = _deployFunded(t2);
        (bool ok, bytes memory ret) = _cancel(inst2, t2);
        assertFalse(ok);
        assertEq(_sel(ret), Eravon.StageHeld.selector, "stage 1 unopened: held");
        vm.warp(uint256(t1.openBy) + 1);
        vm.prank(BUYER);
        inst1.init(t1, quote, "", "", new address[](0), new bytes32[](0));
        (,, bool ended, uint16 bps,, bytes32 c) = inst1.stage();
        assertTrue(ended && bps == 10000 && c == orderHash(t1), "the expiry branch ended stage 1 with the whole share");
        uint256 b0 = tok.balanceOf(BUYER_PAYOUT);
        (ok,) = _cancel(inst2, t2);
        assertTrue(ok, "stage 2 cancels");
        assertEq(tok.balanceOf(BUYER_PAYOUT) - b0, A);
    }

    // ---- the series judge: handed on through a stage that opened no served seat, and past a resigned one ----

    function test_stage_seriesJudgeCarriesThroughAnUnservedStage() public {
        q = defaultQuote(address(tok), address(0), false);
        quote = encodeQuote(q);
        (address[] memory m, bytes32[] memory w) = _panel(6);
        engagePanel(GUILD, m, w, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst1, Eravon.OrderTerms memory t1) = _guildStage(address(0), m, w);
        drawNext(inst1, m, w);
        bytes32 id1 = orderHash(t1);
        address j1 = inst1.seatOf(id1, 0).judge;
        vm.prank(j1, j1);
        inst1.accept(id1, 0);
        claim(inst1, keccak256("m"));
        vm.prank(BUYER);
        inst1.confirm();
        (,,,, address s1,) = inst1.stage();
        assertEq(s1, j1, "stage 1: the judge who heard it");
        // stage 2: j1 is offered first and never accepts; the parties settle at 0 before any acceptance
        (Eravon inst2, Eravon.OrderTerms memory t2) = _guildStage(address(inst1), m, w);
        drawNext(inst2, m, w);
        bytes32 id2 = orderHash(t2);
        assertEq(inst2.seatOf(id2, 0).judge, j1, "offered to the series judge first");
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 sh = keccak256(abi.encode(TH_SETTLE, uint256(0), exp));
        vm.prank(SELLER);
        inst2.settle(0, exp, sign(BUYER_PK, digestOf(address(inst2), sh)), sign(SELLER_PK, digestOf(address(inst2), sh)));
        (,, bool ended, uint16 bps, address s2,) = inst2.stage();
        assertTrue(ended && bps == 0);
        assertEq(s2, j1, "no seat served: the inherited judge is handed on");
        // stage 3 seats j1 first again
        (Eravon inst3, Eravon.OrderTerms memory t3) = _guildStage(address(inst2), m, w);
        drawNext(inst3, m, w);
        assertEq(inst3.seatOf(orderHash(t3), 0).judge, j1, "stage 3: the series judge");
    }

    function test_stage_aResignedJudgeIsNotTheSeriesJudge() public {
        q = defaultQuote(address(tok), address(0), false);
        quote = encodeQuote(q);
        (address[] memory m, bytes32[] memory w) = _panel(6);
        engagePanel(GUILD, m, w, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst1, Eravon.OrderTerms memory t1) = _guildStage(address(0), m, w);
        drawNext(inst1, m, w);
        bytes32 id1 = orderHash(t1);
        address j1 = inst1.seatOf(id1, 0).judge;
        vm.prank(j1, j1);
        inst1.accept(id1, 0);
        claim(inst1, keccak256("m"));
        vm.prank(BUYER);
        inst1.confirm();
        // stage 2: j1 declines; j2 is drawn, accepts, and resigns after the claim; the buyer confirms
        (Eravon inst2, Eravon.OrderTerms memory t2) = _guildStage(address(inst1), m, w);
        drawNext(inst2, m, w);
        bytes32 id2 = orderHash(t2);
        bytes32 declined = sha256("declined");
        vm.prank(j1, j1);
        inst2.resign(id2, 0, declined, "declined");
        drawNext(inst2, m, w);
        address j2 = inst2.seatOf(id2, 1).judge;
        assertTrue(j2 != j1);
        vm.prank(j2, j2);
        inst2.accept(id2, 1);
        claim(inst2, keccak256("m"));
        bytes32 left = sha256("left");
        vm.prank(j2, j2);
        inst2.resign(id2, 1, left, "left");
        vm.prank(BUYER);
        inst2.confirm();
        (,, bool ended, uint16 bps, address s2,) = inst2.stage();
        assertTrue(ended && bps == 0, "Released after the resignation");
        assertEq(s2, j1, "the resigned judge is not handed on; the inherited one is");
        (Eravon inst3, Eravon.OrderTerms memory t3) = _guildStage(address(inst2), m, w);
        drawNext(inst3, m, w);
        assertEq(inst3.seatOf(orderHash(t3), 0).judge, j1, "stage 3 seats the series judge, not the one who left");
    }

    // ---- the served-seat test's two halves and the Lapsed zero on a later stage (FUZZ, stages round 2) ----

    /// The stage's last seat is offered to a judge other than the series judge and never accepted; the stage ends at 0;
    /// its `stage()` hands on the inherited judge, not the unaccepted one.
    function test_stage_anUnacceptedSeatOfAnotherJudgeHandsOnTheInherited() public {
        q = defaultQuote(address(tok), address(0), false);
        quote = encodeQuote(q);
        (address[] memory m, bytes32[] memory w) = _panel(6);
        engagePanel(GUILD, m, w, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst1, Eravon.OrderTerms memory t1) = _guildStage(address(0), m, w);
        drawNext(inst1, m, w);
        bytes32 id1 = orderHash(t1);
        address j1 = inst1.seatOf(id1, 0).judge;
        vm.prank(j1, j1);
        inst1.accept(id1, 0);
        claim(inst1, keccak256("m"));
        vm.prank(BUYER);
        inst1.confirm();
        // stage 2: j1 declines; j2 is drawn and never accepts; the parties settle at 0
        (Eravon inst2, Eravon.OrderTerms memory t2) = _guildStage(address(inst1), m, w);
        drawNext(inst2, m, w);
        bytes32 id2 = orderHash(t2);
        assertEq(inst2.seatOf(id2, 0).judge, j1);
        bytes32 declined = sha256("declined");
        vm.prank(j1, j1);
        inst2.resign(id2, 0, declined, "declined");
        drawNext(inst2, m, w);
        address j2 = inst2.seatOf(id2, 1).judge;
        assertTrue(j2 != j1 && j2 != address(0));
        assertEq(inst2.seatOf(id2, 1).accepted, 0);
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 sh = keccak256(abi.encode(TH_SETTLE, uint256(0), exp));
        vm.prank(SELLER);
        inst2.settle(0, exp, sign(BUYER_PK, digestOf(address(inst2), sh)), sign(SELLER_PK, digestOf(address(inst2), sh)));
        (,, bool ended, uint16 bps, address s2,) = inst2.stage();
        assertTrue(ended && bps == 0);
        assertEq(s2, j1, "the unaccepted seat's judge is not handed on; the inherited one is");
        (Eravon inst3, Eravon.OrderTerms memory t3) = _guildStage(address(inst2), m, w);
        drawNext(inst3, m, w);
        assertEq(inst3.seatOf(orderHash(t3), 0).judge, j1, "stage 3 seats the series judge first");
    }

    /// The Lapsed zero on a later stage: the stage before handed on J; this stage lapses by its judge's silence
    /// (the seat state 4); its `stage()` answers no judge, where without the Lapsed test it would answer the
    /// inherited J.
    function test_stage_aLaterStageThatLapsedHandsOnNoJudge() public {
        (Eravon inst1,) = _releasedStage();
        Q memory q0 = q;
        q0.lapseDefault = 0;
        bytes memory quote0 = encodeQuote(q0);
        Eravon.OrderTerms memory t2 = namedTerms(J, AMANAH, quote0, A);
        t2.prev = address(inst1);
        fundDeposit(q0, A);
        Eravon inst2 = deployProxy(t2);
        tok.mint(address(inst2), A);
        vm.prank(SELLER);
        inst2.init(t2, quote0, buyerSig(t2), sellerSig(t2), new address[](0), new bytes32[](0));
        claim(inst2, keccak256("m"));
        raise(inst2);
        vm.warp(vm.getBlockTimestamp() + q0.rulingWindow + 1);
        inst2.advance(orderHash(t2), 0);
        assertEq(uint8(inst2.status().phase), uint8(Eravon.Phase.Lapsed));
        assertEq(inst2.seatOf(orderHash(t2), 0).state, 4, "the seat lapsed");
        (,, bool ended, uint16 bps, address j,) = inst2.stage();
        assertTrue(ended && bps == 0);
        assertEq(j, address(0), "a Lapsed later stage hands on no judge, not the inherited one");
    }

    /// A served seat that ended by a ruling (state 2) hands its judge on, at the first rung and at a higher one
    /// (FUZZ, stages round 3).
    function test_stage_aRuledStageHandsItsJudgeOn() public {
        for (uint64 rung = 0; rung < 2; rung++) {
            Eravon.OrderTerms memory t1 = _stage(address(0));
            fundDeposit(q, A);
            Eravon inst1 = openNamed(t1, quote);
            claim(inst1, keccak256("m"));
            raise(inst1);
            bytes32 id1 = orderHash(t1);
            bytes32 entryId = sha256("entry");
            vm.prank(J, J);
            inst1.rule(id1, 0, entryId, rung, "entry", "");
            assertEq(inst1.seatOf(id1, 0).state, 2, "the seat ruled");
            (,, bool ended,, address j,) = inst1.stage();
            assertTrue(ended);
            assertEq(j, J, "a ruled stage hands its judge on");
        }
    }

    /// The series judge under a stranger's draw (s7: open to any sender; ADV, open-draw round 1): eight later stages
    /// of one served stage, each drawn by a stranger, every one seats the judge who heard the stage before.
    function test_stage_aStrangersDrawSeatsTheSeriesJudge() public {
        address stranger = address(0x3000000000000000000000000000000000000077);
        q = defaultQuote(address(tok), address(0), false);
        quote = encodeQuote(q);
        (address[] memory m, bytes32[] memory w) = _panel(6);
        engagePanel(GUILD, m, w, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst1, Eravon.OrderTerms memory t1) = _guildStage(address(0), m, w);
        drawNext(inst1, m, w);
        bytes32 id1 = orderHash(t1);
        address j1 = inst1.seatOf(id1, 0).judge;
        vm.prank(j1, j1);
        inst1.accept(id1, 0);
        claim(inst1, keccak256("m"));
        vm.prank(BUYER);
        inst1.confirm();
        for (uint256 k = 0; k < 8; k++) {
            (Eravon inst2, Eravon.OrderTerms memory t2) = _guildStage(address(inst1), m, w);
            vm.roll(vm.getBlockNumber() + 1);
            vm.prank(stranger);
            inst2.redraw(m, w);
            assertEq(inst2.seatOf(orderHash(t2), 0).judge, j1, "a stranger's draw seats the series judge");
        }
    }
}

/// An account that writes chosen storage and then carries the proxy's runtime: the proxy's code hash with storage
/// no order wrote. Under s1 it proves no address, so the standard reads it as no instance.
contract StageForger {
    constructor(bytes memory runtime, uint256[] memory slots, uint256[] memory vals) {
        for (uint256 i = 0; i < slots.length; i++) {
            uint256 sl = slots[i];
            uint256 v = vals[i];
            assembly {
                sstore(sl, v)
            }
        }
        assembly {
            return(add(runtime, 32), mload(runtime))
        }
    }
}
