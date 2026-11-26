// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {MockFeed, MockUSD} from "./mocks/Mocks.sol";

/// Standard s3: the litigation fee priced once at init from a reference unit
/// through two feeds, rounded up, clamped min then max, pinned; a stale or
/// silent feed refuses the case; the cap in the token.
contract EravonPricing is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE_PAYOUT = address(0x4000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah");
    MockUSD internal usd;
    MockFeed internal feedTok; // the token priced in a base with 8 decimals
    MockFeed internal feedUsd; // the unit priced in the same base

    function setUp() public {
        setUpBase();
        usd = new MockUSD();
        feedTok = new MockFeed(8, 2000e8, vm.getBlockTimestamp()); // one token = 2000 base
        feedUsd = new MockFeed(8, 1e8, vm.getBlockTimestamp()); // one unit = 1 base
    }

    function pricedQ(uint256 feeFixedUsd, uint256 feeMinUsd, uint256 feeMaxUsd, uint256 capUsd, uint32 maxStale)
        internal
        view
        returns (Q memory q)
    {
        q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.feeUnit = address(usd);
        q.feedToken = address(feedTok);
        q.feedUnit = address(feedUsd);
        q.maxStale = maxStale;
        q.feeFixed = feeFixedUsd;
        q.feeBps = 0;
        q.feeMin = feeMinUsd;
        q.feeMax = feeMaxUsd;
        q.amountCap = capUsd;
    }

    function openPriced(Q memory q, uint256 amount_) internal returns (Eravon inst, Eravon.OrderTerms memory t) {
        bytes memory quote = encodeQuote(q);
        t = namedTerms(JUDGE, AMANAH, quote, amount_);
        fundDeposit(amount_ * 10); // more than any deposit here
        inst = openNamed(t, quote);
    }

    function test_priced_convertsPinsAndPullsTheDeposit() public {
        // fee fixed 50 USD; one token = 2000 USD, so 50 USD = 0.025 token; cap 1,000,000 USD = 500 token
        Q memory q = pricedQ(50e6, 0, 0, 1_000_000e6, 3600);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        fundDeposit(1 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 1 ether);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Priced(address(usd), 0.025 ether, 500 ether, 2000e8, 1e8, uint64(vm.getBlockTimestamp()));
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        (uint256 lf, uint256 cap, uint256 pT, uint256 pU, uint64 at) = inst.feeQuoted();
        assertEq(lf, 0.025 ether);
        assertEq(cap, 500 ether);
        assertEq(pT, 2000e8);
        assertEq(pU, 1e8);
        assertEq(at, uint64(vm.getBlockTimestamp()));
        // the deposit is P + the priced lf
        assertEq(depositHeld(inst), 0.01 ether + 0.025 ether);
        assertEq(premiumHeld(inst), 0.01 ether);
        // the pinned number does not move with the feed afterwards
        feedTok.set(1000e8, vm.getBlockTimestamp());
        (uint256 lf2,,,,) = inst.feeQuoted();
        assertEq(lf2, 0.025 ether);
    }

    function test_priced_roundsUpAndClampsInOrder() public {
        feedTok.set(3e8, vm.getBlockTimestamp()); // one token = 3 USD
        (Eravon inst,) = openPriced(pricedQ(1e6, 0, 0, 0, 3600), 1 ether); // 1 USD = 1/3 token, rounded up
        (uint256 lf,,,,) = inst.feeQuoted();
        assertEq(lf, 333333333333333334);
        // feeMin above feeMax: raised to min, then lowered to max, yielding max
        (Eravon inst2,) = openPriced(pricedQ(0, 9e6, 6e6, 0, 3600), 1 ether);
        (uint256 lf2,,,,) = inst2.feeQuoted();
        assertEq(lf2, 2 ether);
    }

    function test_priced_staleOrSilentFeedRefusesInit() public {
        Q memory q = pricedQ(50e6, 0, 0, 0, 600);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        fundDeposit(1 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 1 ether);
        feedUsd.set(1e8, vm.getBlockTimestamp() - 601);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.RateStale.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        feedUsd.set(0, vm.getBlockTimestamp());
        vm.prank(SELLER);
        vm.expectRevert(Eravon.RateStale.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        feedUsd.set(-5, vm.getBlockTimestamp());
        vm.prank(SELLER);
        vm.expectRevert(Eravon.RateStale.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        // a feed that is no contract
        feedUsd.set(1e8, vm.getBlockTimestamp());
        vm.etch(address(feedTok), "");
        vm.prank(SELLER);
        vm.expectRevert(Eravon.RateStale.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        // nothing landed, the deposit was never pulled; the case opens once the feed answers
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.None));
        assertEq(tok.balanceOf(SELLER), 1 ether);
        vm.etch(address(feedTok), address(feedUsd).code);
        feedTok.set(2000e8, vm.getBlockTimestamp());
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
    }

    function test_priced_staleIsReadAfterTheSignatures() public {
        Q memory q = pricedQ(50e6, 0, 0, 0, 600);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 1 ether);
        feedUsd.set(1e8, vm.getBlockTimestamp() - 601);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.init(t, quote, sellerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
    }

    function test_priced_capInTokenRefusesOverAmount() public {
        // cap 1000 USD = 0.5 token; amount 1 token is over it
        Q memory q = pricedQ(50e6, 0, 0, 1000e6, 3600);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        fundDeposit(1 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 1 ether);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.OverCap.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        // at the cap it opens
        (Eravon inst2,) = openPriced(q, 0.5 ether);
        assertEq(uint8(phaseOf(inst2)), uint8(Eravon.Phase.Funded));
        // a cap in the token itself, no unit
        Q memory q2 = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q2.amountCap = 10 ether;
        bytes memory quote2 = encodeQuote(q2);
        Eravon.OrderTerms memory t2 = namedTerms(JUDGE, AMANAH, quote2, 10 ether + 1);
        fundDeposit(100 ether);
        Eravon inst3 = deployProxy(t2);
        tok.mint(address(inst3), 10 ether + 1);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.OverCap.selector);
        inst3.init(t2, quote2, buyerSig(t2), sellerSig(t2), new address[](0), new bytes32[](0));
    }

    function test_priced_zeroFeedSideCountsAsOne() public {
        Q memory q = pricedQ(50e6, 0, 0, 0, 3600);
        q.feedUnit = address(0);
        (Eravon inst,) = openPriced(q, 1 ether);
        // 50e6 unit-units * 1 * 10^8 * 10^18 / (2000e8 * 10^0 * 10^6) = 0.025 token
        (uint256 lf,,, uint256 pU, uint64 at) = inst.feeQuoted();
        assertEq(lf, 0.025 ether);
        assertEq(pU, 1);
        assertEq(at, uint64(vm.getBlockTimestamp()), "the token feed's clock");
    }

    function test_unpriced_quoteInTheToken() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        fundDeposit(q, 1 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 1 ether);
        vm.recordLogs();
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        VmSafe.Log[] memory logs = vm.getRecordedLogs();
        for (uint256 i = 0; i < logs.length; i++) {
            assertTrue(logs[i].topics[0] != Eravon.Priced.selector, "no Priced without a unit");
        }
        (uint256 lf, uint256 cap, uint256 pT, uint256 pU, uint64 at) = inst.feeQuoted();
        assertEq(lf, 5 ether, "10% of 1 raised to feeMin 5");
        assertEq(cap, 0);
        assertEq(pT, 0);
        assertEq(pU, 0);
        assertEq(at, 0);
    }
}

import {VmSafe} from "forge-std/Vm.sol";
