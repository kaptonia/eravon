// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {HostileToken} from "./mocks/Mocks.sol";

/// Standard s8 against a token that misbehaves: re-entrancy from inside every
/// transfer the instance makes, fee on transfer, false or empty or dirty
/// answers, reverts on zero, a blocked payout; and the residual across the
/// ladder under fuzzed amounts and premiums.
contract EravonTokens is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE_PAYOUT = address(0x4000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah");
    bytes32 internal constant MANIFEST = keccak256("manifest");
    bytes internal constant ENTRY = "judgment entry bytes";
    bytes32 internal E;
    uint256 internal constant A = 100 ether;

    HostileToken internal hos;
    Q internal q;
    bytes internal quote;

    function setUp() public {
        setUpBase();
        E = sha256(ENTRY);
        hos = new HostileToken();
        q = defaultQuote(address(hos), JUDGE_PAYOUT, true);
        quote = encodeQuote(q);
    }

    function hostileTerms(uint256 amount_) internal returns (Eravon.OrderTerms memory t) {
        t = namedTerms(JUDGE, AMANAH, quote, amount_);
        t.offer.token = address(hos);
    }

    function fundHostileDeposit(uint256 d) internal {
        hos.mint(SELLER, d);
        vm.prank(SELLER);
        hos.approve(address(logic), type(uint256).max);
    }

    /// Deploys, funds the amount and the deposit, and opens as the seller.
    function openHostile(uint256 amount_) internal returns (Eravon inst, Eravon.OrderTerms memory t) {
        t = hostileTerms(amount_);
        inst = deployProxy(t);
        hos.mint(address(inst), amount_);
        fundHostileDeposit(depositOf(q, amount_));
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
    }

    function raiseHostile(Eravon inst) internal {
        (uint256 lf,,,,) = inst.feeQuoted();
        hos.mint(BUYER, lf);
        vm.prank(BUYER);
        hos.approve(address(inst), lf);
        vm.prank(BUYER);
        inst.dispute();
    }

    // ---- re-entrancy ----

    /// Every guarded entry point, as calldata a hook replays from inside a transfer.
    function _probes(Eravon.OrderTerms memory t, bytes32 id) internal view returns (bytes[] memory p) {
        bytes memory sig65 = new bytes(65);
        address[] memory noAddrs = new address[](0);
        bytes32[] memory noWords = new bytes32[](0);
        p = new bytes[](18);
        p[0] = abi.encodeCall(Eravon.init, (t, quote, sig65, sig65, noAddrs, noWords));
        p[1] = abi.encodeCall(Eravon.cancel, (t, ""));
        p[2] = abi.encodeCall(Eravon.redraw, (noAddrs, noWords));
        p[3] = abi.encodeCall(Eravon.accept, (id, 0));
        p[4] = abi.encodeCall(Eravon.rule, (id, 0, E, 1, ENTRY, ""));
        p[5] = abi.encodeCall(Eravon.resign, (id, 0, E, ENTRY));
        p[6] = abi.encodeCall(Eravon.advance, (id, 0));
        p[7] = abi.encodeCall(Eravon.claimDelivery, (MANIFEST));
        p[8] = abi.encodeCall(Eravon.claimDeliveryBySig, (MANIFEST, sig65));
        p[9] = abi.encodeCall(Eravon.confirm, ());
        p[10] = abi.encodeCall(Eravon.confirmBySig, (sig65));
        p[11] = abi.encodeCall(Eravon.dispute, ());
        p[12] = abi.encodeCall(Eravon.disputeBySig, (sig65));
        p[13] = abi.encodeCall(Eravon.refund, ());
        p[14] = abi.encodeCall(Eravon.refundBySig, (sig65));
        p[15] = abi.encodeCall(Eravon.settle, (5000, vm.getBlockTimestamp() + 1 days, sig65, sig65));
        p[16] = abi.encodeCall(Eravon.withdraw, (address(hos), JUDGE, address(0)));
        p[17] = abi.encodeCall(Eravon.sweep, (BUYER_PAYOUT, address(hos)));
    }

    /// The hook fires once per move; the results are whole rounds of the
    /// eighteen probes, every one of them refused by the lock.
    function assertAllReentered(uint256 n) internal view {
        uint256 got = hos.resultsLength();
        assertTrue(got >= n && got % n == 0, "whole rounds of probes ran");
        for (uint256 i = 0; i < got; i++) {
            assertEq(hos.results(i), Eravon.Reentered.selector, "a probe got through the lock");
        }
    }

    function test_reentrancy_duringTheDepositPull() public {
        Eravon.OrderTerms memory t = hostileTerms(A);
        Eravon inst = deployProxy(t);
        hos.mint(address(inst), A);
        fundHostileDeposit(11 ether);
        hos.arm(address(inst), _probes(t, orderHash(t)));
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        assertAllReentered(18);
        assertEq(depositHeld(inst), 11 ether, "the deposit landed");
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
    }

    function test_reentrancy_theLogicDuringThePull() public {
        // the token's hook aims at the logic: pull from the token's address is no instance's call
        Eravon.OrderTerms memory t = hostileTerms(A);
        Eravon inst = deployProxy(t);
        hos.mint(address(inst), A);
        fundHostileDeposit(11 ether);
        bytes[] memory p = new bytes[](3);
        p[0] = abi.encodeCall(Eravon.pull, (SELLER, SELLER, address(hos), 1 ether));
        p[1] = abi.encodeCall(Eravon.revoke, (address(hos), t.offer.termsHash));
        p[2] = abi.encodeCall(Eravon.consent, (address(hos), keccak256("x")));
        hos.arm(address(logic), p);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        assertEq(hos.results(0), Eravon.Reentered.selector, "pull is under the logic's own lock while it runs");
        assertEq(hos.results(1), bytes4(0), "revoke writes under the token's own key: harmless");
        assertEq(hos.results(2), bytes4(0), "consent likewise");
        assertTrue(logic.revoked(address(hos), t.offer.termsHash));
        assertFalse(logic.revoked(SELLER, t.offer.termsHash));
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
    }

    function test_reentrancy_duringTheFeePull() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openHostile(A);
        claim(inst, MANIFEST);
        hos.mint(BUYER, 10 ether);
        vm.prank(BUYER);
        hos.approve(address(inst), 10 ether);
        hos.arm(address(inst), _probes(t, orderHash(t)));
        vm.prank(BUYER);
        inst.dispute();
        assertAllReentered(18);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Disputed));
        Eravon.CaseStatus memory _st35 = inst.status();
        uint256 lfHeld = _st35.lfHeld;
        assertEq(lfHeld, 10 ether);
    }

    function test_reentrancy_duringTheTerminalPush() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openHostile(A);
        claim(inst, MANIFEST);
        hos.arm(address(inst), _probes(t, orderHash(t)));
        vm.prank(BUYER);
        inst.confirm();
        assertAllReentered(18);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released));
        assertEq(hos.balanceOf(SELLER_PAYOUT), 100 ether);
        assertEq(hos.balanceOf(SELLER), 10 ether, "the seller's LF back to the key");
        assertEq(hos.balanceOf(address(inst)), 1 ether, "the judge's credit alone");
    }

    function test_reentrancy_duringTheRulingPush() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openHostile(A);
        bytes32 id = orderHash(t);
        claim(inst, MANIFEST);
        raiseHostile(inst);
        hos.arm(address(inst), _probes(t, id));
        vm.prank(JUDGE, JUDGE);
        inst.rule(id, 0, E, 1, ENTRY, "");
        assertAllReentered(18);
        assertEq(inst.seatOf(id, 0).state, 2);
        assertEq(hos.balanceOf(BUYER_PAYOUT), 55 ether, "half of A and half the seller's LF");
        assertEq(hos.balanceOf(SELLER_PAYOUT), 50 ether);
        assertEq(hos.balanceOf(SELLER), 5 ether);
        assertEq(hos.balanceOf(address(inst)), 11 ether, "P and the buyer's fee for the judge");
    }

    function test_reentrancy_duringSweepAndWithdraw() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openHostile(A);
        claim(inst, MANIFEST);
        vm.prank(BUYER);
        inst.confirm();
        hos.mint(address(inst), 3 ether);
        hos.arm(address(inst), _probes(t, orderHash(t)));
        vm.prank(SELLER);
        inst.sweep(BUYER_PAYOUT, address(hos));
        assertAllReentered(18);
        assertEq(hos.balanceOf(BUYER_PAYOUT), 3 ether);
        hos.arm(address(inst), _probes(t, orderHash(t)));
        vm.prank(JUDGE);
        inst.withdraw(address(hos), JUDGE, JUDGE_PAYOUT);
        assertAllReentered(18);
        assertEq(hos.balanceOf(JUDGE_PAYOUT), 1 ether);
    }

    function test_theUnguardedEntryPointsHoldNothing() public {
        Eravon.OrderTerms memory t = hostileTerms(A);
        Eravon inst = deployProxy(t);
        hos.mint(address(inst), A);
        fundHostileDeposit(11 ether);
        bytes[] memory p = new bytes[](3);
        p[0] = abi.encodeCall(Eravon.commit, (orderHash(t), keccak256("from inside a transfer")));
        p[1] = abi.encodeCall(Eravon.setPayout, (address(hos), address(0xBADBEEF)));
        p[2] = abi.encodeCall(Eravon.engage, (bytes32(0), new address[](0), new bytes32[](0), bytes32(0), 0, address(0)));
        hos.arm(address(inst), p);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        assertEq(hos.results(0), Eravon.NotOpen.selector, "no case yet while the deposit is pulled");
        assertEq(hos.results(1), bytes4(0), "setPayout writes under the token's own key");
        assertEq(hos.results(2), Eravon.NoPanelFace.selector);
        (address p0,) = inst.accountOf(address(hos), address(hos));
        assertEq(p0, address(0xBADBEEF));
        (address pj,) = inst.accountOf(JUDGE, address(hos));
        assertEq(pj, address(0), "the judge's account is untouched");
    }

    // ---- fee on transfer ----

    function test_feeOnTransfer_depositAndFeeAreMeasured() public {
        hos.setMode(HostileToken.Mode.FeeOnTransfer);
        Eravon.OrderTerms memory t = hostileTerms(A);
        Eravon inst = deployProxy(t);
        hos.mint(address(inst), A);
        fundHostileDeposit(20 ether);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.DepositShort.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.None));
        // the case opens under a plain token; the fee pulled later under a fee-taking one is short
        hos.setMode(HostileToken.Mode.Plain);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        claim(inst, MANIFEST);
        hos.setMode(HostileToken.Mode.FeeOnTransfer);
        hos.mint(BUYER, 10 ether);
        vm.prank(BUYER);
        hos.approve(address(inst), 10 ether);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.Underfunded.selector);
        inst.dispute();
        // sent ahead in full, the fee is measured by the balance and the dispute lands
        hos.mint(address(inst), 10 ether);
        vm.prank(BUYER);
        inst.dispute();
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Disputed));
    }

    // ---- false, nothing, dirty, zero ----

    function test_tokenReturningFalseIsRefusedOnTheWayInAndKeepsTheCreditOnTheWayOut() public {
        hos.setMode(HostileToken.Mode.ReturnsFalse);
        Eravon.OrderTerms memory t = hostileTerms(A);
        Eravon inst = deployProxy(t);
        hos.mint(address(inst), A);
        fundHostileDeposit(11 ether);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.TransferFailed.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        hos.setMode(HostileToken.Mode.Plain);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        claim(inst, MANIFEST);
        hos.setMode(HostileToken.Mode.ReturnsFalse);
        vm.prank(BUYER);
        inst.confirm();
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released), "the terminal is reached");
        assertEq(hos.balanceOf(SELLER_PAYOUT), 0, "nothing moved");
        assertEq(inst.owed(SELLER_PAYOUT, address(hos)), 100 ether, "the credit is kept");
        assertEq(inst.owed(SELLER, address(hos)), 10 ether);
        hos.setMode(HostileToken.Mode.Plain);
        vm.prank(SELLER);
        inst.sweep(SELLER_PAYOUT, address(hos));
        vm.prank(SELLER);
        inst.sweep(SELLER, address(hos));
        assertEq(hos.balanceOf(SELLER_PAYOUT), 100 ether);
        assertEq(hos.balanceOf(SELLER), 10 ether);
        assertEq(inst.owed(SELLER_PAYOUT, address(hos)), 0);
    }

    function test_tokenReturningNothingRunsTheWholeLifecycle() public {
        hos.setMode(HostileToken.Mode.ReturnsNothing);
        (Eravon inst, Eravon.OrderTerms memory t) = openHostile(A);
        claim(inst, MANIFEST);
        raiseHostile(inst);
        vm.prank(JUDGE, JUDGE);
        inst.rule(orderHash(t), 0, E, 2, ENTRY, "");
        assertEq(hos.balanceOf(BUYER_PAYOUT), 110 ether, "A and the seller's LF");
        assertEq(hos.balanceOf(address(inst)), 11 ether);
        vm.prank(JUDGE);
        inst.withdraw(address(hos), JUDGE, JUDGE_PAYOUT);
        assertEq(hos.balanceOf(JUDGE_PAYOUT), 11 ether);
    }

    function test_tokenWithADirtyBooleanWord() public {
        hos.setMode(HostileToken.Mode.DirtyBool);
        // the pull demands a clean true: refused
        Eravon.OrderTerms memory t = hostileTerms(A);
        Eravon inst = deployProxy(t);
        hos.mint(address(inst), A);
        fundHostileDeposit(11 ether);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.TransferFailed.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        hos.setMode(HostileToken.Mode.Plain);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        // a push counts by the balance that left, whatever the word
        claim(inst, MANIFEST);
        hos.setMode(HostileToken.Mode.DirtyBool);
        vm.prank(BUYER);
        inst.confirm();
        assertEq(hos.balanceOf(SELLER_PAYOUT), 100 ether);
        assertEq(inst.owed(SELLER_PAYOUT, address(hos)), 0, "paid, so no credit kept");
    }

    function test_tokenRevertingOnZeroIsNeverAskedForOne() public {
        hos.setMode(HostileToken.Mode.RevertsOnZero);
        // Released: the buyer's share is zero and no transfer of zero is attempted
        (Eravon inst,) = openHostile(A);
        claim(inst, MANIFEST);
        vm.prank(BUYER);
        inst.confirm();
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released));
        // Ruled at the last rung: the seller's share and its LF are zero
        (Eravon inst2, Eravon.OrderTerms memory t2) = openHostile(A);
        claim(inst2, MANIFEST);
        raiseHostile(inst2);
        vm.prank(JUDGE, JUDGE);
        inst2.rule(orderHash(t2), 0, E, 2, ENTRY, "");
        assertEq(uint8(phaseOf(inst2)), uint8(Eravon.Phase.Ruled));
        // a zero premium: nothing for the judge, no zero push
        Q memory q0 = q;
        q0.premiumBps = 0;
        bytes memory quote0 = encodeQuote(q0);
        Eravon.OrderTerms memory t3 = namedTerms(JUDGE, AMANAH, quote0, A);
        t3.offer.token = address(hos);
        Eravon inst3 = deployProxy(t3);
        hos.mint(address(inst3), A);
        fundHostileDeposit(10 ether);
        vm.prank(SELLER);
        inst3.init(t3, quote0, buyerSig(t3), sellerSig(t3), new address[](0), new bytes32[](0));
        claim(inst3, MANIFEST);
        vm.prank(BUYER);
        inst3.confirm();
        (, uint256 c) = inst3.accountOf(JUDGE, address(hos));
        assertEq(c, 0);
        // sweep and withdraw with nothing owed ask for nothing
        vm.prank(SELLER);
        inst3.sweep(BUYER_PAYOUT, address(hos));
        vm.prank(JUDGE);
        inst3.withdraw(address(hos), JUDGE, JUDGE_PAYOUT);
    }

    function test_blockedPayoutDoesNotStopTheCase() public {
        (Eravon inst,) = openHostile(A);
        claim(inst, MANIFEST);
        hos.setMode(HostileToken.Mode.BlocksOne);
        hos.block_(SELLER_PAYOUT);
        vm.prank(BUYER);
        inst.confirm();
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released));
        assertEq(inst.owed(SELLER_PAYOUT, address(hos)), 100 ether, "the blocked credit is kept");
        assertEq(hos.balanceOf(SELLER), 10 ether, "the other push went through");
        hos.block_(address(0));
        vm.prank(SELLER);
        inst.sweep(SELLER_PAYOUT, address(hos));
        assertEq(hos.balanceOf(SELLER_PAYOUT), 100 ether);
    }

    // ---- the residual ----

    /// Across the ladder, every terminal leaves the instance holding the
    /// judge's credit alone, and the sums paid are A + D + the fee.
    function testFuzz_zeroResidualAcrossTheLadder(uint16 premium, uint256 amount_, uint8 rung) public {
        premium = uint16(bound(premium, 0, 10000));
        amount_ = bound(amount_, 1, 1e30);
        rung = uint8(bound(rung, 0, 2));
        Q memory qf = q;
        qf.premiumBps = premium;
        bytes memory quotef = encodeQuote(qf);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quotef, amount_);
        t.offer.token = address(hos);
        Eravon inst = deployProxy(t);
        hos.mint(address(inst), amount_);
        uint256 d = depositOf(qf, amount_);
        fundHostileDeposit(d);
        vm.prank(SELLER);
        inst.init(t, quotef, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        assertEq(depositHeld(inst), d);
        claim(inst, MANIFEST);
        raiseHostile(inst);
        uint256 lf = lfOf(qf, amount_);
        vm.prank(JUDGE, JUDGE);
        inst.rule(orderHash(t), 0, E, rung, ENTRY, "");
        (, uint256 jc) = inst.accountOf(JUDGE, address(hos));
        assertEq(jc, premiumOf(qf, amount_) + lf, "P and the fee to the judge");
        assertEq(hos.balanceOf(address(inst)), jc, "nothing else left");
        uint256 paid = hos.balanceOf(BUYER_PAYOUT) + hos.balanceOf(SELLER_PAYOUT) + hos.balanceOf(SELLER) + jc;
        assertEq(paid, amount_ + d + lf, "everything that came in went out");
        uint256 buyerA = (amount_ * t.offer.ladder[rung].buyerBps) / 10000;
        uint256 bond = rung == 2 ? lf : (rung == 1 ? lf / 2 : 0);
        assertEq(hos.balanceOf(BUYER_PAYOUT), buyerA + bond);
        assertEq(hos.balanceOf(SELLER_PAYOUT), amount_ - buyerA);
        assertEq(hos.balanceOf(SELLER), lf - bond);
    }
}
