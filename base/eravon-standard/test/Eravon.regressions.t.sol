// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {EravonVerifier} from "../src/EravonVerifier.sol";
import {IPriceFeed} from "../src/interfaces.sol";
import {RawFeed, DecToken} from "./Eravon.fuzz.t.sol";
import {HostileToken, MockUSD} from "./mocks/Mocks.sol";

/// A price feed whose answers are valid in their first words and run on for `extra` bytes more (the round's
/// five words, then zeros; `decimals()` likewise); the verifier copies a fixed size, so the length costs the
/// caller nothing.
contract BombFeed {
    int256 internal answer;
    uint256 internal extra;

    constructor(int256 a, uint256 e) {
        answer = a;
        extra = e;
    }

    fallback() external {
        bool round = msg.sig == IPriceFeed.latestRoundData.selector;
        int256 a = answer;
        uint256 e = extra;
        assembly ("memory-safe") {
            let p := mload(0x40)
            switch round
            case 1 {
                mstore(p, 1)
                mstore(add(p, 32), a)
                mstore(add(p, 64), timestamp())
                mstore(add(p, 96), timestamp())
                mstore(add(p, 128), 1)
                return(p, add(160, e))
            }
            default {
                mstore(p, 8)
                return(p, add(32, e))
            }
        }
    }
}

/// A token whose `transfer` moves the balance and answers with `extra` bytes of return data beyond the bool.
contract BombToken {
    mapping(address => uint256) public balanceOf;
    uint256 internal extra;
    uint8 public constant decimals = 18;

    constructor(uint256 e) {
        extra = e;
    }

    function mint(address to, uint256 v) external {
        balanceOf[to] += v;
    }

    function transfer(address to, uint256 v) external returns (bool) {
        require(balanceOf[msg.sender] >= v, "balance");
        balanceOf[msg.sender] -= v;
        balanceOf[to] += v;
        uint256 e = extra;
        assembly ("memory-safe") {
            let p := mload(0x40)
            mstore(p, 1)
            return(p, add(32, e))
        }
    }
}

/// The audit rounds' findings F1 to F10, each as the rule that now holds: the amount bound (F1), the pricing
/// that answers every anomaly by name after the terms guards and the consents (F2), P paid at a settlement
/// only to an accepted seat within its window (F4), the fee
/// taken from whatever flowed in unrecorded since init (F5), the exits after the expiry branch (F6), the
/// credit of what is not yet owed (F7), the fixed-size copies of a feed's answer (F8) and of a push's (F9), and
/// P to the judge at a refund while Claimed under a seated seat (F10).
contract EravonRegressions is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE_PAYOUT = address(0x4000000000000000000000000000000000000001);
    address internal constant STRANGER = address(0x3000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah");
    bytes32 internal constant MANIFEST = keccak256("manifest");
    bytes internal constant ENTRY = "judgment entry bytes";
    bytes32 internal E;

    function setUp() public {
        setUpBase();
        E = sha256(ENTRY);
    }

    function _open(Q memory q, uint256 amount) internal returns (Eravon inst, Eravon.OrderTerms memory t, bytes memory quote) {
        quote = encodeQuote(q);
        t = namedTerms(JUDGE, AMANAH, quote, amount);
        fundDeposit(depositOf(q, amount));
        inst = openNamed(t, quote);
    }

    function _settle(Eravon inst, uint16 bps) internal {
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 d = digestOf(address(inst), keccak256(abi.encode(TH_SETTLE, uint256(bps), exp)));
        vm.prank(SELLER);
        inst.settle(bps, exp, sign(BUYER_PK, d), sign(SELLER_PK, d));
    }

    function _accept(Eravon inst, bytes32 id) internal {
        vm.prank(JUDGE, JUDGE);
        inst.accept(id, 0);
    }

    // ------------------------------------------------------------------
    // F1: the amount is at most (2^256 - 1) / 10000 (terms guard 6), so that
    // every share A x bps / 10000 and P are in range at every terminal.
    // ------------------------------------------------------------------

    function test_F1_amountPastTheBoundIsRefused() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.premiumBps = 0;
        q.feeBps = 0;
        bytes memory quote = encodeQuote(q);
        uint256[3] memory bad = [type(uint256).max / 10000 + 1, 2 ** 255, type(uint256).max];
        for (uint256 k = 0; k < 3; k++) {
            Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, bad[k]);
            EravonVerifier.Checked memory c = verifier.check(t, quote, "", "", new address[](0), new bytes32[](0));
            assertEq(c.termsFault, 6, "guard 6");
            Eravon inst = deployProxy(t);
            bytes memory bs = buyerSig(t);
            bytes memory ss = sellerSig(t);
            vm.prank(SELLER);
            vm.expectRevert(abi.encodeWithSelector(Eravon.BadTerms.selector, 6));
            inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        }
    }

    function test_F1_amountAtTheBoundRunsEveryTerminal() public {
        uint256 a = type(uint256).max / 10000;
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, false); // the judge's word awaited
        q.premiumBps = 10000; // P = A: the product A x 10000 at its largest
        q.feeBps = 10000; // A x feeBps as well, lf then lowered to feeMax
        uint256 p = premiumOf(q, a);
        uint256 lf = lfOf(q, a);
        assertEq(p, a);
        assertEq(lf, q.feeMax);
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, a);
        bytes32 id = orderHash(t);
        uint256 s0 = vm.snapshotState();
        // the decline: Resigned at 10000
        vm.prank(JUDGE, JUDGE);
        inst.resign(id, 0, E, ENTRY);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned));
        assertEq(tok.balanceOf(BUYER_PAYOUT), a);
        assertEq(tok.balanceOf(SELLER), p + lf);
        vm.revertToState(s0);
        // the offer clock: Refunded(1)
        vm.warp(vm.getBlockTimestamp() + 3 days + 1);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded));
        assertEq(tok.balanceOf(BUYER_PAYOUT), a);
        vm.revertToState(s0);
        _accept(inst, id);
        uint256 s1 = vm.snapshotState();
        // the seller's refund
        vm.prank(SELLER);
        inst.refund();
        assertEq(tok.balanceOf(BUYER_PAYOUT), a);
        assertEq(tok.balanceOf(SELLER), p + lf);
        vm.revertToState(s1);
        // T1
        vm.warp(vm.getBlockTimestamp() + 7 days + 1);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded));
        assertEq(tok.balanceOf(BUYER_PAYOUT), a);
        vm.revertToState(s1);
        // settlements at 1 bps and at 10000
        _settle(inst, 1);
        assertEq(tok.balanceOf(BUYER_PAYOUT), a / 10000);
        assertEq(tok.balanceOf(SELLER_PAYOUT), a - a / 10000);
        vm.revertToState(s1);
        _settle(inst, 10000);
        assertEq(tok.balanceOf(BUYER_PAYOUT), a);
        vm.revertToState(s1);
        // disputed: the ruling at the last rung, the lapse to it, the refund refused within T3 and plain past it
        claim(inst, MANIFEST);
        raise(inst);
        uint256 s2 = vm.snapshotState();
        vm.prank(JUDGE, JUDGE);
        inst.rule(id, 0, E, 2, ENTRY, "");
        assertEq(tok.balanceOf(BUYER_PAYOUT), a + lf, "A and the seller's LF (bearer 1) to the buyer");
        (, uint256 jc) = inst.accountOf(JUDGE, address(tok));
        assertEq(jc, p + lf);
        vm.revertToState(s2);
        vm.warp(vm.getBlockTimestamp() + 14 days + 1);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Lapsed));
        assertEq(tok.balanceOf(BUYER_PAYOUT), a + lf);
        vm.revertToState(s2);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.refund();
        vm.warp(vm.getBlockTimestamp() + 14 days + 1);
        vm.prank(SELLER);
        inst.refund();
        assertEq(tok.balanceOf(BUYER_PAYOUT), a + lf, "past T3 the plain refund: A and the fee back to the buyer");
        (, jc) = inst.accountOf(JUDGE, address(tok));
        assertEq(jc, 0, "nothing to the judge at a refund");
    }

    // ------------------------------------------------------------------
    // F2: every pricing anomaly is an answer of the verifier, never a revert
    // in it; the logic refuses by name, after the terms guards, the quote,
    // the consents and the balance (standard s4 step 5's order).
    // ------------------------------------------------------------------

    function _unitQuote(address settleToken, address unit, address feedT, address feedU) internal pure returns (Q memory q) {
        q = defaultQuote(settleToken, JUDGE_PAYOUT, true);
        q.feeUnit = unit;
        q.feedToken = feedT;
        q.feedUnit = feedU;
        q.maxStale = 1 days;
        q.feeFixed = 0;
        q.feeMin = 5e6;
        q.feeMax = 500e6;
    }

    function test_F2_feedRoundWordsPast80BitsAreNotRead() public {
        DecToken unit = new DecToken(6);
        RawFeed fT = new RawFeed();
        RawFeed fU = new RawFeed();
        fT.set(8, 2 ** 80, 2000e8, vm.getBlockTimestamp(), 0); // a round id no uint80 holds
        fU.set(8, 2 ** 255, 1e8, vm.getBlockTimestamp(), 0);
        Q memory q = _unitQuote(address(tok), address(unit), address(fT), address(fU));
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 1 ether);
        // the consents decide before the pricing, as for any quote
        vm.prank(BUYER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quote, "", "", new address[](0), new bytes32[](0));
        // 5e6 and 500e6 of the unit at 2000.0 a token: 2.5e15 and 2.5e17; lf = 1e17 between them
        uint256 lf = 1e17;
        fundDeposit(premiumOf(q, 1 ether) + lf);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        (uint256 lfQ, uint256 capQ, uint256 pT, uint256 pU, uint64 at) = inst.feeQuoted();
        assertEq(lfQ, lf);
        assertEq(capQ, 0);
        assertEq(pT, 2000e8);
        assertEq(pU, 1e8);
        assertEq(at, uint64(vm.getBlockTimestamp()));
    }

    function test_F2_admittedDecimalsPriceWithoutOverflow() public {
        DecToken settle = new DecToken(36); // decimals up to 36 are admitted
        DecToken unit = new DecToken(8);
        RawFeed fT = new RawFeed();
        RawFeed fU = new RawFeed();
        fT.set(18, 1, 1e18, vm.getBlockTimestamp(), 0); // the token at 1.0, an 18-decimal feed
        fU.set(18, 1, 120000e18, vm.getBlockTimestamp(), 0); // the unit at 120000.0
        Q memory q = _unitQuote(address(settle), address(unit), address(fT), address(fU));
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t.offer.token = address(settle);
        // ceil(5e6 x 1.2e23 x 10^54 / (1e18 x 10^26)) = 6e39: the common power of ten cancelled, nothing overflows
        uint256 lf = 6e39;
        uint256 p = premiumOf(q, 1 ether);
        Eravon inst = deployProxy(t);
        settle.mint(address(inst), 1 ether);
        settle.mint(SELLER, p + lf);
        vm.prank(SELLER);
        settle.approve(address(logic), p + lf);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        (uint256 lfQ,,,,) = inst.feeQuoted();
        assertEq(lfQ, lf);
        assertEq(depositHeld(inst), p + lf);
    }

    function test_F2_feeOverflowIsNamedAfterTheConsentsAndTheBalance() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.feeFixed = type(uint256).max; // "any word" (terms/1 s3.2); the sum is E_FEE_OVERFLOW (s6.1)
        q.feeBps = 1;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        EravonVerifier.Checked memory c = verifier.check(t, quote, "", "", new address[](0), new bytes32[](0));
        assertTrue(c.overflow, "the verifier reports the overflow");
        assertEq(c.termsFault, 0);
        assertFalse(c.quoteBad);
        Eravon inst = deployProxy(t);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        // a stranger with no signature: the consents
        vm.prank(BUYER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quote, "", "", new address[](0), new bytes32[](0));
        // the consents hold, the address is unfunded: the balance
        vm.prank(SELLER);
        vm.expectRevert(Eravon.Underfunded.selector);
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        // funded: the pricing, by name
        tok.mint(address(inst), 1 ether);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.FeeOverflow.selector);
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
    }

    function test_F2_termsGuardsPrecedeThePricing() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.feeFixed = type(uint256).max;
        q.feeBps = 1;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t.offer.termsHash = bytes32(0); // BadTerms(13), the first fault s4 step 5 decides
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 1 ether);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        vm.expectRevert(abi.encodeWithSelector(Eravon.BadTerms.selector, 13));
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
    }

    function test_F2_conversionOrDepositPastTheWordIsFeeOverflow() public {
        // a converted absolute past 2^256 - 1: 2^255 of the unit at four tokens a unit
        DecToken unit = new DecToken(18);
        RawFeed fT = new RawFeed();
        RawFeed fU = new RawFeed();
        fT.set(8, 1, 1e8, vm.getBlockTimestamp(), 0);
        fU.set(8, 1, 4e8, vm.getBlockTimestamp(), 0);
        Q memory q = _unitQuote(address(tok), address(unit), address(fT), address(fU));
        q.feeFixed = 2 ** 255;
        q.feeMin = 0;
        q.feeMax = 0;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 1 ether);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.FeeOverflow.selector);
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        // the deposit P + lf past 2^256 - 1, each in range
        Q memory q2 = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q2.premiumBps = 10000;
        q2.feeFixed = type(uint256).max - 1 ether;
        q2.feeBps = 0;
        q2.feeMin = 0;
        q2.feeMax = 0;
        bytes memory quote2 = encodeQuote(q2);
        Eravon.OrderTerms memory t2 = namedTerms(JUDGE, AMANAH, quote2, 2 ether);
        Eravon inst2 = deployProxy(t2);
        tok.mint(address(inst2), 2 ether);
        bytes memory bs2 = buyerSig(t2);
        bytes memory ss2 = sellerSig(t2);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.FeeOverflow.selector);
        inst2.init(t2, quote2, bs2, ss2, new address[](0), new bytes32[](0));
    }

    // ------------------------------------------------------------------
    // F4: a settlement pays P to the judge only where the last seat was
    // accepted and its window still runs; otherwise P goes back to the seller.
    // ------------------------------------------------------------------

    /// Settles at 5000 and answers the judge's claimable and what the seller's key received.
    function _settleSplit(Eravon inst) internal returns (uint256 judgeClaim, uint256 sellerKeyGot) {
        uint256 before = tok.balanceOf(SELLER);
        _settle(inst, 5000);
        (, judgeClaim) = inst.accountOf(JUDGE, address(tok));
        sellerKeyGot = tok.balanceOf(SELLER) - before;
    }

    function test_F4_settlePaysPOnlyToAnAcceptedSeatWithinItsWindow() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, false); // the judge's word awaited
        uint256 a = 100 ether;
        uint256 p = premiumOf(q, a);
        uint256 lf = lfOf(q, a);
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, a);
        bytes32 id = orderHash(t);
        uint256 s0 = vm.snapshotState();
        (uint256 jc, uint256 sk) = _settleSplit(inst);
        assertEq(jc, 0, "an offer never accepted earns nothing");
        assertEq(sk, p + lf);
        vm.revertToState(s0);
        vm.warp(vm.getBlockTimestamp() + 3 days + 1);
        (jc, sk) = _settleSplit(inst);
        assertEq(jc, 0, "nor one whose offer clock ran out");
        assertEq(sk, p + lf);
        vm.revertToState(s0);
        _accept(inst, id);
        uint64 end = inst.seatOf(id, 0).windowEnd;
        uint256 s1 = vm.snapshotState();
        vm.warp(end);
        (jc, sk) = _settleSplit(inst);
        assertEq(jc, p, "accepted, T1's last second");
        assertEq(sk, lf);
        vm.revertToState(s1);
        vm.warp(uint256(end) + 1);
        (jc, sk) = _settleSplit(inst);
        assertEq(jc, 0, "accepted, T1 run out with nobody advancing");
        assertEq(sk, p + lf);
        vm.revertToState(s1);
        claim(inst, MANIFEST);
        end = inst.seatOf(id, 0).windowEnd;
        uint256 s2 = vm.snapshotState();
        vm.warp(end);
        (jc,) = _settleSplit(inst);
        assertEq(jc, p, "claimed, T2's last second");
        vm.revertToState(s2);
        vm.warp(uint256(end) + 1);
        (jc,) = _settleSplit(inst);
        assertEq(jc, p, "claimed, T2 run out: the late settlement pays the judge as T2's lapse would");
        vm.revertToState(s2);
        raise(inst);
        end = inst.seatOf(id, 0).windowEnd;
        uint256 s3 = vm.snapshotState();
        vm.warp(end);
        (jc,) = _settleSplit(inst);
        assertEq(jc, p, "disputed, T3's last second");
        vm.revertToState(s3);
        vm.warp(uint256(end) + 1);
        (jc, sk) = _settleSplit(inst);
        assertEq(jc, 0, "disputed, the judge silent past T3");
        assertEq(sk, p + lf);
        assertEq(inst.seatOf(id, 0).state, 5);
    }

    // ------------------------------------------------------------------
    // F5: the dispute takes the fee from whatever flowed in unrecorded since
    // init, Funded and Claimed alike; what flowed in beyond it is the buyer's
    // stray, returned at the terminal.
    // ------------------------------------------------------------------

    function test_F5_aStraySentWhileFundedPaysTheFee() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        uint256 a = 100 ether;
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, a);
        (uint256 lf,,,,) = inst.feeQuoted();
        uint256 extra = 3 ether;
        tok.mint(address(inst), lf + extra); // while Funded
        claim(inst, MANIFEST);
        vm.prank(BUYER);
        inst.dispute(); // no allowance, nothing sent while Claimed
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Disputed), "the inflow since init paid the fee");
        Eravon.CaseStatus memory _st24 = inst.status();
        uint256 lfHeld = _st24.lfHeld;
        assertEq(lfHeld, lf);
        // the ruling at rung 1 (5000, half each): the buyer's half of A, half the seller's LF, and the stray beyond the fee
        vm.prank(JUDGE, JUDGE);
        inst.rule(orderHash(t), 0, E, 1, ENTRY, "");
        assertEq(tok.balanceOf(BUYER_PAYOUT), a / 2 + lf / 2 + extra);
    }

    // ------------------------------------------------------------------
    // F6: after the expiry branch the payout and the token are kept: the case
    // token sent later leaves by the branch again, another token by sweep.
    // ------------------------------------------------------------------

    function test_F6_afterTheExpiryBranchTheCaseTokenLeavesByTheBranchAndOthersBySweep() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 1 ether);
        vm.warp(t.openBy + 1);
        vm.prank(BUYER);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 1 ether, "the branch refunded");
        tok.mint(address(inst), 2 ether);
        vm.prank(SELLER);
        inst.sweep(BUYER_PAYOUT, address(tok));
        assertEq(tok.balanceOf(address(inst)), 2 ether, "sweep leaves the case token to the branch");
        vm.prank(BUYER);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(address(inst)), 0, "the branch again moves it");
        assertEq(tok.balanceOf(BUYER_PAYOUT), 3 ether);
        DecToken other = new DecToken(18);
        other.mint(address(inst), 5 ether);
        vm.prank(BUYER);
        inst.sweep(STRANGER, address(other));
        assertEq(other.balanceOf(BUYER_PAYOUT), 5 ether, "another token by sweep, to the buyer's payout");
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.None), "no case");
    }

    // ------------------------------------------------------------------
    // F7 (the fold's own finding): the expiry branch and cancel credit the
    // balance beyond what is owed already, so a run whose push failed is
    // never counted twice and the credit stays payable.
    // ------------------------------------------------------------------

    /// Round 3 (reader B): the verifier copies a feed's round to 160 bytes and a decimals() to 32, so feeds
    /// answering 200 KiB more cost the caller nothing beyond the reads' allowances, and the case opens priced.
    function test_F8_aFeedsLongAnswerCostsNothingBeyondTheAllowance() public {
        MockUSD usd = new MockUSD();
        BombFeed fa = new BombFeed(2000e8, 200_000);
        BombFeed fb = new BombFeed(1e8, 200_000);
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.feeUnit = address(usd);
        q.feedToken = address(fa);
        q.feedUnit = address(fb);
        q.maxStale = 3600;
        q.feeFixed = 50e6;
        q.feeBps = 0;
        q.feeMin = 0;
        q.feeMax = 0;
        q.amountCap = 1_000_000e6;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        fundDeposit(10 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 1 ether);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        uint256 g0 = gasleft();
        vm.prank(SELLER);
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        uint256 used = g0 - gasleft();
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded), "priced and open");
        assertLt(used, 2_000_000, "the answers' length is not the caller's to pay");
    }

    /// Round 3 (reader B): a push copies back no return data, so a token answering a transfer with 400 KiB costs
    /// the caller nothing beyond the push's allowance; the push is counted by the balance that left.
    function test_F9_aLongAnswerToAPushCostsNothingBeyondTheAllowance() public {
        BombToken bt = new BombToken(400_000);
        Q memory q = defaultQuote(address(bt), JUDGE_PAYOUT, true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t.offer.token = address(bt);
        Eravon inst = deployProxy(t);
        bt.mint(address(inst), 1 ether);
        vm.warp(t.openBy + 1);
        uint256 g0 = gasleft();
        vm.prank(BUYER);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        uint256 used = g0 - gasleft();
        assertEq(bt.balanceOf(BUYER_PAYOUT), 1 ether, "delivered, counted by the balance that left");
        assertEq(inst.owed(BUYER_PAYOUT, address(bt)), 0, "nothing left owed");
        assertLt(used, 700_000, "the answer's length is not the caller's to pay");
    }

    /// Round 3 (the fuzz seat's R3-L1): a refund while Claimed under a seated seat pays P to the judge, as T2's
    /// lapse would, so that a refund and a payment off the instance cannot complete the trade with P stripped;
    /// in Funded, and in Claimed after the seat resigned, P returns to the seller as the clock's own lapse would.
    function test_F10_aRefundWhileClaimedUnderASeatedSeatPaysTheJudgeP() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        uint256 a = 100 ether;
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, a);
        uint256 p = premiumOf(q, a);
        uint256 lf = lfOf(q, a);
        uint256 s0 = vm.snapshotState();
        // Funded: P back to the seller
        vm.prank(SELLER);
        inst.refund();
        (, uint256 jc) = inst.accountOf(JUDGE, address(tok));
        assertEq(jc, 0, "nothing to the judge while Funded");
        vm.revertToState(s0);
        // Claimed, the seat seated: P to the judge
        claim(inst, MANIFEST);
        uint256 s1 = vm.snapshotState();
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Closed(0, a, lf, p, 0);
        vm.prank(SELLER);
        inst.refund();
        (, jc) = inst.accountOf(JUDGE, address(tok));
        assertEq(jc, p, "P to the judge as T2's lapse would give it");
        assertEq(tok.balanceOf(BUYER_PAYOUT), a);
        vm.revertToState(s1);
        // Claimed after the seat resigned: P back to the seller
        vm.prank(JUDGE, JUDGE);
        inst.resign(orderHash(t), 0, E, ENTRY);
        vm.prank(SELLER);
        inst.refund();
        (, jc) = inst.accountOf(JUDGE, address(tok));
        assertEq(jc, 0, "nothing to a judge who resigned");
    }

    function test_F7_theExpiryBranchAndCancelCreditOnlyWhatIsNotOwed() public {
        HostileToken hos = new HostileToken();
        hos.setMode(HostileToken.Mode.BlocksOne);
        hos.block_(BUYER_PAYOUT);
        Q memory q = defaultQuote(address(hos), JUDGE_PAYOUT, true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t.offer.token = address(hos);
        Eravon inst = deployProxy(t);
        hos.mint(address(inst), 1 ether);
        vm.warp(t.openBy + 1);
        // the push to a blocked payout fails and the credit stays
        vm.prank(BUYER);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        assertEq(inst.owed(BUYER_PAYOUT, address(hos)), 1 ether, "credited once");
        assertEq(hos.balanceOf(address(inst)), 1 ether);
        // the branch again: nothing beyond the owed sum is there, so nothing is credited
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Refunded(2);
        vm.prank(BUYER);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        assertEq(inst.owed(BUYER_PAYOUT, address(hos)), 1 ether, "still credited once");
        // a later inflow is credited on top, once
        hos.mint(address(inst), 0.5 ether);
        vm.prank(BUYER);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        assertEq(inst.owed(BUYER_PAYOUT, address(hos)), 1.5 ether);
        // cancel after the branch: Closed reports what this call credited, nothing
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Closed(0, 0, 0, 0, 0);
        vm.prank(BUYER);
        inst.cancel(t, "");
        assertEq(inst.owed(BUYER_PAYOUT, address(hos)), 1.5 ether);
        Eravon.CaseStatus memory _st25 = inst.status();
        uint256 unpaid = _st25.unpaid;
        assertEq(unpaid, 1.5 ether, "the ledger equals the balance");
        // unblocked, sweep pays the whole credit and the ledger closes
        hos.block_(address(0));
        vm.prank(SELLER);
        inst.sweep(BUYER_PAYOUT, address(hos));
        assertEq(hos.balanceOf(BUYER_PAYOUT), 1.5 ether);
        assertEq(hos.balanceOf(address(inst)), 0);
        assertEq(inst.owed(BUYER_PAYOUT, address(hos)), 0);
        Eravon.CaseStatus memory _st26 = inst.status();
        unpaid = _st26.unpaid;
        assertEq(unpaid, 0);
    }
}
