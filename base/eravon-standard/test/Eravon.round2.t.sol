// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {EravonVerifier} from "../src/EravonVerifier.sol";
import {EravonPanelBook} from "../src/EravonPanelBook.sol";
import {RawFeed, DecToken} from "./Eravon.fuzz.t.sol";
import {BytesFeed} from "./Eravon.arith.t.sol";
import {MockUSD} from "./mocks/Mocks.sol";

/// A plain settlement token one of whose receivers runs a hook that burns every unit of gas it is given.
contract BurnHookToken {
    uint8 public decimals = 18;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;
    address public burner;

    function setBurner(address b) external {
        burner = b;
    }

    function mint(address to, uint256 v) external {
        balanceOf[to] += v;
    }

    function approve(address s, uint256 v) external returns (bool) {
        allowance[msg.sender][s] = v;
        return true;
    }

    function transfer(address to, uint256 v) external returns (bool) {
        return _move(msg.sender, to, v);
    }

    function transferFrom(address from, address to, uint256 v) external returns (bool) {
        allowance[from][msg.sender] -= v;
        return _move(from, to, v);
    }

    function _move(address from, address to, uint256 v) internal returns (bool) {
        if (to == burner && burner != address(0)) {
            assembly ("memory-safe") {
                invalid()
            }
        }
        balanceOf[from] -= v;
        balanceOf[to] += v;
        return true;
    }
}

/// A Chainlink-shaped feed whose latestRoundData burns every unit of gas it is given.
contract GasBurnFeed {
    function decimals() external pure returns (uint8) {
        return 8;
    }

    function latestRoundData() external pure returns (uint80, int256, uint256, uint256, uint80) {
        assembly ("memory-safe") {
            invalid()
        }
    }
}

/// A token whose balanceOf reverts with its own reason.
contract RevertingBalanceToken {
    function balanceOf(address) external pure returns (uint256) {
        revert("no balance today");
    }
}

/// A token whose balanceOf answers until its first transferFrom, and reverts from then on.
contract FlakyBalanceToken {
    mapping(address => uint256) internal bal;
    mapping(address => mapping(address => uint256)) public allowance;
    bool public mute;

    function mint(address to, uint256 v) external {
        bal[to] += v;
    }

    function balanceOf(address a) external view returns (uint256) {
        require(!mute, "mute");
        return bal[a];
    }

    function approve(address s, uint256 v) external returns (bool) {
        allowance[msg.sender][s] = v;
        return true;
    }

    function transfer(address to, uint256 v) external returns (bool) {
        bal[msg.sender] -= v;
        bal[to] += v;
        return true;
    }

    function transferFrom(address from, address to, uint256 v) external returns (bool) {
        allowance[from][msg.sender] -= v;
        bal[from] -= v;
        bal[to] += v;
        mute = true;
        return true;
    }
}

/// Round 2, seat FUZZ: the regression half over round 1's F1 to F6 (each round-1 proof ported to the revised
/// code, the same scenario now answering as the text says), and this round's findings R2-1 to R2-8, each
/// as the rule that now holds.
contract EravonRound2 is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE_PAYOUT = address(0x4000000000000000000000000000000000000001);
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    address internal constant STRANGER = address(0x3000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah");
    bytes32 internal constant ROSTER = keccak256("roster");
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

    // ==================================================================
    // regression: F1, the amount past (2^256 - 1) / 10000
    // ==================================================================

    /// Round 1's test_F1_hugeAmountOpensThenRefundsRevert: A = 2^255 under a quote whose premium and fee
    /// rate are zero opened, and then the refund, T1 and every settlement above 1 bps panicked. Now guard 6
    /// refuses the order at the gate, and the largest amount it admits runs those terminals.
    function test_regression_F1_hugeAmountIsRefusedAndTheBoundRunsTheRefunds() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.premiumBps = 0;
        q.feeBps = 0;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 2 ** 255);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 2 ** 255);
        fundDeposit(depositOf(q, 1));
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        vm.expectRevert(abi.encodeWithSelector(Eravon.BadTerms.selector, uint8(6)));
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        // the buyer's money is not locked: the buyer cancels
        vm.prank(BUYER);
        inst.cancel(t, "");
        assertEq(tok.balanceOf(BUYER_PAYOUT), 2 ** 255);
        // the bound itself: the refund, T1 and a settlement at 2 bps all land
        uint256 a = type(uint256).max / 10000;
        (Eravon inst2, Eravon.OrderTerms memory t2,) = _open(q, a);
        bytes32 id = orderHash(t2);
        uint256 s0 = vm.snapshotState();
        vm.prank(SELLER);
        inst2.refund();
        assertEq(tok.balanceOf(BUYER_PAYOUT), 2 ** 255 + a);
        vm.revertToState(s0);
        vm.warp(vm.getBlockTimestamp() + 7 days + 1);
        inst2.advance(id, 0);
        assertEq(uint8(phaseOf(inst2)), uint8(Eravon.Phase.Refunded));
        vm.revertToState(s0);
        _settle(inst2, 2);
        assertEq(tok.balanceOf(SELLER_PAYOUT), a - (a * 2) / 10000);
    }

    /// Round 1's test_F1_hugeAmountDeclineAndLastRungRevert: A = (2^256 - 1) / 10000 + 1 opened under an
    /// offer clock and the decline and the offer clock's end panicked. Now refused; at the bound both land.
    function test_regression_F1_onePastTheBoundIsRefusedAndTheDeclineLands() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, false);
        q.premiumBps = 0;
        q.feeBps = 0;
        bytes memory quote = encodeQuote(q);
        uint256 a = type(uint256).max / 10000;
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, a + 1);
        EravonVerifier.Checked memory c = verifier.check(t, quote, "", "", new address[](0), new bytes32[](0));
        assertEq(c.termsFault, 6);
        (Eravon inst, Eravon.OrderTerms memory t2,) = _open(q, a);
        bytes32 id = orderHash(t2);
        uint256 s0 = vm.snapshotState();
        vm.prank(JUDGE, JUDGE);
        inst.resign(id, 0, E, ENTRY);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned));
        assertEq(tok.balanceOf(BUYER_PAYOUT), a);
        vm.revertToState(s0);
        vm.warp(vm.getBlockTimestamp() + 3 days + 1);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded));
        assertEq(tok.balanceOf(BUYER_PAYOUT), a);
    }

    // ==================================================================
    // regression: F2, pricing anomalies answered by name in s4 step 5's order
    // ==================================================================

    /// Round 1's test_F2_feedRoundIdPast80BitsRevertsBare: a round id of 2^80 reverted with no data, before
    /// the consents. Now the round words are not read: a stranger meets BadSellerSig and the seller opens.
    function test_regression_F2_roundIdPast80BitsPrices() public {
        DecToken unit = new DecToken(6);
        RawFeed fT = new RawFeed();
        RawFeed fU = new RawFeed();
        fT.set(8, 2 ** 80, 2000e8, vm.getBlockTimestamp(), 0);
        fU.set(8, 1, 1e8, vm.getBlockTimestamp(), 0);
        Q memory q = _unitQuote(address(tok), address(unit), address(fT), address(fU));
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 1 ether);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quote, "", "", new address[](0), new bytes32[](0));
        fundDeposit(premiumOf(q, 1 ether) + 1e17);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        (uint256 lf,,,,) = inst.feeQuoted();
        assertEq(lf, 1e17);
    }

    /// Round 1's test_F2_admittedDecimalsOverflowWithAPanic: a 36-decimal token under 18-decimal feeds
    /// panicked on pU x 10^54. Now the common power cancels and the product is full width: the stranger meets
    /// the consents, the seller opens at lf = 6e39.
    function test_regression_F2_admittedDecimalsPriceExactly() public {
        DecToken settle = new DecToken(36);
        DecToken unit = new DecToken(8);
        RawFeed fT = new RawFeed();
        RawFeed fU = new RawFeed();
        fT.set(18, 1, 1e18, vm.getBlockTimestamp(), 0);
        fU.set(18, 1, 120000e18, vm.getBlockTimestamp(), 0);
        Q memory q = _unitQuote(address(settle), address(unit), address(fT), address(fU));
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t.offer.token = address(settle);
        Eravon inst = deployProxy(t);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quote, "", "", new address[](0), new bytes32[](0));
        uint256 p = premiumOf(q, 1 ether);
        settle.mint(address(inst), 1 ether);
        settle.mint(SELLER, p + 6e39);
        vm.prank(SELLER);
        settle.approve(address(logic), p + 6e39);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        (uint256 lf,,,,) = inst.feeQuoted();
        assertEq(lf, 6e39);
    }

    /// Round 1's test_F2_feeOverflowPrecedesTheConsentsAndTheBalance: feeFixed = 2^256 - 1 panicked before
    /// the consents. Now: BadSellerSig, then Underfunded, then FeeOverflow, in s4 step 5's order.
    function test_regression_F2_feeOverflowAfterTheConsentsAndTheBalance() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.feeFixed = type(uint256).max;
        q.feeBps = 1;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        Eravon inst = deployProxy(t);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quote, "", "", new address[](0), new bytes32[](0));
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.Underfunded.selector);
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        tok.mint(address(inst), 1 ether);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.FeeOverflow.selector);
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
    }

    /// Round 1's test_F2_pricingPanicPreemptsTheTermsGuards: the same overflow panicked ahead of BadTerms(13).
    /// Now the terms guard answers first.
    function test_regression_F2_termsGuardBeforeThePricing() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.feeFixed = type(uint256).max;
        q.feeBps = 1;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t.offer.termsHash = bytes32(0);
        Eravon inst = deployProxy(t);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        vm.expectRevert(abi.encodeWithSelector(Eravon.BadTerms.selector, uint8(13)));
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
    }

    // ==================================================================
    // regression: F4, P at a settlement
    // ==================================================================

    /// Round 1's test_F4_unacceptedOfferCollectsPAtASettlement: P went to a judge that never took the seat.
    /// Now P returns to the seller's key.
    function test_regression_F4_unacceptedOfferEarnsNothingAtASettlement() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, false);
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, 100 ether);
        bytes32 id = orderHash(t);
        vm.warp(vm.getBlockTimestamp() + 3 days + 1);
        uint256 before = tok.balanceOf(SELLER);
        _settle(inst, 5000);
        (, uint256 c) = inst.accountOf(JUDGE, address(tok));
        assertEq(c, 0, "nothing to a judge that never took the seat");
        assertEq(tok.balanceOf(SELLER) - before, depositOf(q, 100 ether), "P and the seller's LF back to the key");
        assertEq(inst.seatOf(id, 0).state, 5);
    }

    /// Round 1's test_F4_silentJudgePastT3CollectsPAtASettlement: P went to a judge whose ruling window had
    /// run out. Now P returns to the seller's key.
    function test_regression_F4_silentJudgePastT3EarnsNothingAtASettlement() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, 100 ether);
        claim(inst, MANIFEST);
        raise(inst);
        vm.warp(vm.getBlockTimestamp() + 14 days + 1);
        uint256 before = tok.balanceOf(SELLER);
        _settle(inst, 5000);
        (, uint256 c) = inst.accountOf(JUDGE, address(tok));
        assertEq(c, 0, "nothing to the judge whose window ran out");
        assertEq(tok.balanceOf(SELLER) - before, depositOf(q, 100 ether));
        assertEq(inst.seatOf(orderHash(t), 0).state, 5);
    }

    // ==================================================================
    // regression: F5 and F6, the two text fixes
    // ==================================================================

    /// Round 1's test_F5_fundedStrayBecomesTheFee: the code took a Funded stray as the fee where s8 read
    /// "while Claimed". The code is unchanged and s8 now says the dispute takes the fee from whatever flowed
    /// in unrecorded since init, Funded and Claimed alike; the same scenario is now the text's.
    function test_regression_F5_aFundedStrayPaysTheFeeAsTheTextSays() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, 100 ether);
        (uint256 lf,,,,) = inst.feeQuoted();
        tok.mint(address(inst), lf);
        claim(inst, MANIFEST);
        vm.prank(BUYER);
        inst.dispute();
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Disputed));
        Eravon.CaseStatus memory _st27 = inst.status();
        uint256 lfHeld = _st27.lfHeld;
        assertEq(lfHeld, lf);
        // nothing beyond the fee flowed in, so nothing stray returns at the ruling
        vm.prank(JUDGE, JUDGE);
        inst.rule(orderHash(t), 0, E, 0, ENTRY, "");
        assertEq(tok.balanceOf(BUYER_PAYOUT), 0, "rung 0, bearer 0: the buyer is owed nothing");
    }

    /// Round 1's test_F6_afterTheExpiryBranchSweepMovesNoCaseToken: the code left the case token sent after
    /// the branch to the branch alone where s4 step 3 read "or by sweep". The code is unchanged and step 3 now
    /// says the settlement token leaves by the branch again and any other token by sweep.
    function test_regression_F6_afterTheBranchTheCaseTokenLeavesByTheBranchAsTheTextSays() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 1 ether);
        vm.warp(t.openBy + 1);
        vm.prank(BUYER);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        tok.mint(address(inst), 2 ether);
        vm.prank(SELLER);
        inst.sweep(BUYER_PAYOUT, address(tok));
        assertEq(tok.balanceOf(address(inst)), 2 ether);
        vm.prank(BUYER);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(address(inst)), 0);
        assertEq(tok.balanceOf(BUYER_PAYOUT), 3 ether);
        DecToken other = new DecToken(18);
        other.mint(address(inst), 5 ether);
        vm.prank(SELLER);
        inst.sweep(STRANGER, address(other));
        assertEq(other.balanceOf(BUYER_PAYOUT), 5 ether);
    }

    // ==================================================================
    // R2-1: withholding the guild's draw buys no second seed
    // ==================================================================

    function _guildCase(uint256 n, uint32 offerWindow, bool autoAccept)
        internal
        returns (Eravon inst, Eravon.OrderTerms memory t, address[] memory m, bytes32[] memory w)
    {
        EravonPanelBook book = deployBook(GUILD);
        m = new address[](n);
        w = new bytes32[](n);
        for (uint256 i = 0; i < n; i++) {
            m[i] = address(uint160(0x001000000000000000000000000000000000000B01 + i));
            w[i] = AMANAH;
        }
        bytes32 key = logic.PROXY_CODEHASH();
        vm.prank(GUILD, GUILD);
        book.engage(key, m, w, ROSTER, uint64(vm.getBlockTimestamp() + 3650 days), address(0));
        vm.roll(vm.getBlockNumber() + 1);
        Q memory q = defaultQuote(address(tok), address(0), autoAccept);
        q.offerWindow = offerWindow;
        bytes memory quote = encodeQuote(q);
        t = guildTerms(GUILD, AMANAH, quote, 100 ether);
        fundDeposit(q, 100 ether);
        inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), m, w);
    }

    function _armed(Eravon inst) internal view returns (bool pending, uint64 deadline, uint64 drawBlock) {
        Eravon.CaseStatus memory _st28 = inst.status();
        pending = _st28.pendingRedraw;
        deadline = _st28.redrawDeadline;
        drawBlock = _st28.drawBlock;
    }

    function _seats(Eravon inst) internal view returns (uint256 seats) {
        Eravon.CaseStatus memory _st29 = inst.status();
        seats = _st29.seats;
    }

    function _deposit(Eravon inst) internal view returns (uint256 d) {
        Eravon.CaseStatus memory _st30 = inst.status();
        d = _st30.deposit;
    }

    /// The member s7's formula seats for the first seat, from a block hash anyone reads from the next block on.
    function _member(bytes32 id, bytes32 seed, address[] memory m) internal pure returns (address) {
        return m[uint256(keccak256(abi.encodePacked(id, seed, uint64(0)))) % m.length];
    }

    /// The seller opens a guild case and, in the block after the arming, reads from the armed block's hash
    /// the member a draw would seat. Where it is the member it wants, it draws; where it is not, it draws
    /// nothing and lets 256 blocks pass, well inside the three days of the deadline. The arming has not
    /// moved (no call re-arms), and the next call, a redraw or an advance by anyone, finds the seed gone and
    /// closes the case as Resigned form 3: the amount back to the buyer, the deposit back to the seller,
    /// nobody seated. Over an eight-member pool the withholder seats the first seed's member or nobody.
    function test_R2_1_withholdingTheDrawBuysNoSecondSeed() public {
        uint256 n = 8;
        uint256 seatedTargets = 0;
        for (uint256 target = 0; target < n; target++) {
            uint256 snap = vm.snapshotState();
            (Eravon inst, Eravon.OrderTerms memory t, address[] memory m, bytes32[] memory w) = _guildCase(n, 3 days, false);
            bytes32 id = orderHash(t);
            (, uint64 deadline, uint64 drawBlock) = _armed(inst);
            uint256 t0 = vm.getBlockTimestamp();
            // the block after the arming: the seed is public, and with it the member
            vm.roll(uint256(drawBlock) + 1);
            vm.warp(t0 + 12);
            if (_member(id, blockhash(drawBlock), m) == m[target]) {
                vm.prank(SELLER);
                inst.redraw(m, w);
                assertEq(inst.seatOf(id, 0).judge, m[target], "the first seed's member is seated");
                seatedTargets++;
                vm.revertToState(snap);
                continue;
            }
            // withheld: 257 blocks on the armed block's hash is no longer served
            vm.roll(uint256(drawBlock) + 257);
            vm.warp(t0 + 12 * 257);
            assertLe(vm.getBlockTimestamp(), deadline, "well inside the deadline");
            (bool pending, uint64 d2, uint64 b2) = _armed(inst);
            assertTrue(pending);
            assertEq(d2, deadline, "the deadline unchanged");
            assertEq(b2, drawBlock, "the armed block unchanged: nothing re-arms");
            uint256 dep = _deposit(inst);
            vm.expectEmit(true, true, true, true, address(inst));
            emit Eravon.Resigned(bytes32(0), 3);
            vm.expectEmit(true, true, true, true, address(inst));
            emit Eravon.Closed(4, 100 ether, dep, 0, 0);
            if (target % 2 == 0) {
                vm.prank(SELLER);
                inst.redraw(m, w);
            } else {
                vm.prank(STRANGER);
                inst.advance(id, 0);
            }
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned), "a lost seed closes the case");
            assertEq(_seats(inst), 0, "nobody seated");
            (pending,,) = _armed(inst);
            assertFalse(pending);
            assertEq(tok.balanceOf(BUYER_PAYOUT), 100 ether, "the amount back to the buyer");
            vm.prank(SELLER);
            vm.expectRevert(Eravon.NotPendingRedraw.selector);
            inst.redraw(m, w);
            vm.revertToState(snap);
        }
        assertEq(seatedTargets, 1, "the withholder seats the first seed's member or nobody");
    }

    /// The same case where anyone else draws in the block after the arming: the first seed's member is seated.
    function test_R2_1_aPromptDrawSeatsTheFirstSeedsMember() public {
        (Eravon inst, Eravon.OrderTerms memory t, address[] memory m, bytes32[] memory w) = _guildCase(8, 3 days, false);
        (,, uint64 drawBlock) = _armed(inst);
        vm.roll(uint256(drawBlock) + 1);
        address want = _member(orderHash(t), blockhash(drawBlock), m);
        vm.prank(BUYER);
        inst.redraw(m, w);
        assertEq(inst.seatOf(orderHash(t), 0).judge, want);
    }

    // ==================================================================
    // R2-2: every push of the standard's own runs under PUSH_GAS; sweep hands the token all its gas
    // ==================================================================

    /// s8: "each best-effort push (a terminal's, cancel's, the expiry branch's) runs under a gas allowance of
    /// 1,000,000". A buyer's payout whose hook burns every unit of gas it is given: cancel, the seller's
    /// refund (a terminal of _finish) and the expiry branch each spend about the allowance of 8,000,000 sent,
    /// land, and leave the credit owed; sweep, whose caller hands the token all the gas it brings, gives the
    /// hook 63/64 of it, lands, and leaves the credit owed as well.
    function test_R2_2_cancelAndTheExpiryBranchPushUnderTheAllowance() public {
        BurnHookToken bt = new BurnHookToken();
        bt.setBurner(BUYER_PAYOUT);
        Q memory q = defaultQuote(address(bt), JUDGE_PAYOUT, true);
        bytes memory quote = encodeQuote(q);
        // cancel
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t.offer.token = address(bt);
        Eravon inst = deployProxy(t);
        bt.mint(address(inst), 1 ether);
        vm.prank(BUYER);
        uint256 g0 = gasleft();
        inst.cancel{gas: 8_000_000}(t, "");
        uint256 usedCancel = g0 - gasleft();
        emit log_named_uint("gas spent by cancel under the hook, 8,000,000 sent", usedCancel);
        assertLt(usedCancel, 1_500_000, "cancel's push stopped at the allowance");
        assertEq(inst.owed(BUYER_PAYOUT, address(bt)), 1 ether, "the credit stays owed");
        // a terminal of _finish
        Eravon.OrderTerms memory t2 = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t2.offer.token = address(bt);
        Eravon inst2 = deployProxy(t2);
        bt.mint(address(inst2), 1 ether);
        uint256 d = depositOf(q, 1 ether);
        bt.mint(SELLER, d);
        vm.prank(SELLER);
        bt.approve(address(logic), d);
        bytes memory bs = buyerSig(t2);
        bytes memory ss = sellerSig(t2);
        vm.prank(SELLER);
        inst2.init(t2, quote, bs, ss, new address[](0), new bytes32[](0));
        vm.prank(SELLER);
        uint256 g1 = gasleft();
        inst2.refund{gas: 8_000_000}();
        uint256 usedRefund = g1 - gasleft();
        emit log_named_uint("gas spent by the seller's refund under the hook, 8,000,000 sent", usedRefund);
        assertLt(usedRefund, 1_500_000, "the terminal's push stopped at the allowance");
        assertEq(inst2.owed(BUYER_PAYOUT, address(bt)), 1 ether);
        // the expiry branch
        Eravon.OrderTerms memory t3 = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t3.offer.token = address(bt);
        Eravon inst3 = deployProxy(t3);
        bt.mint(address(inst3), 1 ether);
        vm.warp(t3.openBy + 1);
        vm.prank(BUYER);
        uint256 g2 = gasleft();
        inst3.init{gas: 8_000_000}(t3, "", "", "", new address[](0), new bytes32[](0));
        uint256 usedBranch = g2 - gasleft();
        emit log_named_uint("gas spent by the expiry branch under the hook, 8,000,000 sent", usedBranch);
        assertLt(usedBranch, 1_500_000, "the branch's push stopped at the allowance");
        assertEq(inst3.owed(BUYER_PAYOUT, address(bt)), 1 ether);
        // sweep hands the token all the gas its caller brings; the credit stays owed where the hook takes it
        vm.prank(BUYER);
        uint256 g3 = gasleft();
        inst.sweep{gas: 8_000_000}(BUYER_PAYOUT, address(bt));
        uint256 usedSweep = g3 - gasleft();
        emit log_named_uint("gas spent by sweep under the hook, 8,000,000 sent", usedSweep);
        assertGt(usedSweep, 7_500_000, "sweep hands the token all the gas, far past the allowance");
        assertEq(inst.owed(BUYER_PAYOUT, address(bt)), 1 ether);
        // the hook lifted, sweep delivers
        bt.setBurner(address(0));
        vm.prank(SELLER);
        inst.sweep(BUYER_PAYOUT, address(bt));
        assertEq(bt.balanceOf(BUYER_PAYOUT), 1 ether);
        assertEq(inst.owed(BUYER_PAYOUT, address(bt)), 0);
    }

    // ==================================================================
    // R2-3: init's balance reads on a token without code, or that refuses balanceOf
    // ==================================================================

    /// s4 step 5: both balance reads go through the read that answers zero where the token does not answer.
    /// On an order whose token has no code at this address's chain, or whose balanceOf reverts, init with
    /// both consents in hand is Underfunded; on a token whose balanceOf stops answering once the deposit
    /// moved, the measure after the pull is DepositShort. Never a bare revert or the token's own reason; the
    /// buyer's cancel lands on the same orders.
    function test_R2_3_initOnATokenWithoutCodeIsUnderfunded() public {
        address noCode = address(uint160(uint256(keccak256("no code at this chain"))));
        Q memory q = defaultQuote(noCode, JUDGE_PAYOUT, true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t.offer.token = noCode;
        Eravon inst = deployProxy(t);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.Underfunded.selector);
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        vm.prank(BUYER);
        inst.cancel(t, "");
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded));
        // balanceOf reverts with the token's own reason: Underfunded all the same
        RevertingBalanceToken rb = new RevertingBalanceToken();
        q = defaultQuote(address(rb), JUDGE_PAYOUT, true);
        quote = encodeQuote(q);
        t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t.offer.token = address(rb);
        inst = deployProxy(t);
        bs = buyerSig(t);
        ss = sellerSig(t);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.Underfunded.selector);
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        vm.prank(BUYER);
        inst.cancel(t, "");
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded));
        // balanceOf answers until the deposit moved, then stops: the second read is DepositShort
        FlakyBalanceToken fb = new FlakyBalanceToken();
        q = defaultQuote(address(fb), JUDGE_PAYOUT, true);
        quote = encodeQuote(q);
        t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t.offer.token = address(fb);
        inst = deployProxy(t);
        fb.mint(address(inst), 1 ether);
        uint256 dep = depositOf(q, 1 ether);
        fb.mint(SELLER, dep);
        vm.prank(SELLER);
        fb.approve(address(logic), dep);
        bs = buyerSig(t);
        ss = sellerSig(t);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.DepositShort.selector);
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
    }

    // ==================================================================
    // R2-4: a latestRoundData answer shorter than its five words is stale
    // ==================================================================

    /// s3: a feed is stale where latestRoundData answers fewer than 160 bytes (its five words); 32, 64, 128
    /// and 159 bytes are stale, 160 prices.
    function test_R2_4_aRoundAnswerShorterThan160BytesIsStale() public {
        DecToken unit = new DecToken(6);
        uint256[5] memory lens = [uint256(32), 64, 128, 159, 160];
        for (uint256 k = 0; k < 5; k++) {
            BytesFeed f = new BytesFeed();
            bytes memory r = new bytes(lens[k]);
            // words: roundId 1, answer 2000e8, startedAt, updatedAt now, answeredInRound
            bytes memory full = abi.encode(uint256(1), uint256(2000e8), vm.getBlockTimestamp(), vm.getBlockTimestamp(), uint256(1));
            for (uint256 i = 0; i < lens[k]; i++) {
                r[i] = full[i];
            }
            f.set(r, abi.encode(uint256(8)), false, false);
            Q memory q = _unitQuote(address(tok), address(unit), address(f), address(0));
            bytes memory quote = encodeQuote(q);
            Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
            EravonVerifier.Checked memory c = verifier.check(t, quote, "", "", new address[](0), new bytes32[](0));
            assertEq(c.stale, lens[k] < 160, "stale exactly below 160 bytes");
        }
    }

    // ==================================================================
    // R2-5: a settlement past T2 pays the judge the P the clock pays it
    // ==================================================================

    /// Claimed, the seat seated, T2 run out and nobody advanced: advance (anyone) releases with P to the
    /// judge, and a settlement at 1 bps closes the same trade with P to the judge as well (s8: a seated seat
    /// while the case is Claimed, at any time); the seller's key receives its LF alone. Where the seat
    /// resigned during T2, both the release and the settlement send P back to the seller's key.
    function test_R2_5_aSettlementPastT2PaysTheJudgeAsTheClockWould() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.premiumBps = 300;
        uint256 a = 100 ether;
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, a);
        bytes32 id = orderHash(t);
        uint256 p = premiumOf(q, a);
        uint256 lf = lfOf(q, a);
        claim(inst, MANIFEST);
        uint256 s1 = vm.snapshotState();
        vm.warp(uint256(inst.seatOf(id, 0).windowEnd) + 1);
        uint256 s0 = vm.snapshotState();
        inst.advance(id, 0);
        (, uint256 jc) = inst.accountOf(JUDGE, address(tok));
        assertEq(jc, p, "the clock releases with P to the judge");
        assertEq(tok.balanceOf(SELLER_PAYOUT), a);
        vm.revertToState(s0);
        uint256 keyBefore = tok.balanceOf(SELLER);
        _settle(inst, 1);
        (, jc) = inst.accountOf(JUDGE, address(tok));
        assertEq(jc, p, "the settlement past T2 pays the judge P");
        assertEq(tok.balanceOf(SELLER_PAYOUT), a - a / 10000, "the seller's share less one bps");
        assertEq(tok.balanceOf(BUYER_PAYOUT), a / 10000, "the buyer's one bps");
        assertEq(tok.balanceOf(SELLER) - keyBefore, lf, "the seller's LF alone back to the key");
        assertEq(inst.seatOf(id, 0).state, 5);
        // the seat resigned during T2: the release and the settlement both send P back
        vm.revertToState(s1);
        vm.prank(JUDGE, JUDGE);
        inst.resign(id, 0, E, ENTRY);
        vm.warp(uint256(inst.seatOf(id, 0).windowEnd) + 1);
        uint256 s2 = vm.snapshotState();
        keyBefore = tok.balanceOf(SELLER);
        inst.advance(id, 0);
        (, jc) = inst.accountOf(JUDGE, address(tok));
        assertEq(jc, 0, "the release under a resigned seat pays the judge nothing");
        assertEq(tok.balanceOf(SELLER) - keyBefore, p + lf);
        vm.revertToState(s2);
        keyBefore = tok.balanceOf(SELLER);
        _settle(inst, 1);
        (, jc) = inst.accountOf(JUDGE, address(tok));
        assertEq(jc, 0, "the settlement under a resigned seat pays the judge nothing");
        assertEq(tok.balanceOf(SELLER) - keyBefore, p + lf);
    }

    // ==================================================================
    // R2-6: a feed that burns its gas reads as stale
    // ==================================================================

    /// s3: every read the pricing makes is handed 200,000 gas, and a read that runs out is one that did not
    /// answer. Both feeds burning: the verifier answers stale under 1,000,000 gas, and init answers RateStale
    /// under the transaction cap and under 2,000,000 alike; one feed burning, or a feed whose decimals()
    /// burns, the same.
    function test_R2_6_gasBurningFeedsAreStaleByName() public {
        DecToken unit = new DecToken(6);
        GasBurnFeed f = new GasBurnFeed();
        GasBurnFeed f2 = new GasBurnFeed();
        BytesFeed fd = new BytesFeed();
        fd.set(abi.encode(uint256(1), uint256(2000e8), vm.getBlockTimestamp(), vm.getBlockTimestamp(), uint256(1)), abi.encode(uint256(8)), false, false);
        fd.setBurns(false, true);
        address[2][3] memory feeds = [[address(f), address(f2)], [address(f), address(0)], [address(fd), address(0)]];
        bytes memory stale = abi.encodeWithSelector(Eravon.RateStale.selector);
        for (uint256 k = 0; k < 3; k++) {
            Q memory q = _unitQuote(address(tok), address(unit), feeds[k][0], feeds[k][1]);
            bytes memory quote = encodeQuote(q);
            Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
            EravonVerifier.Checked memory c = verifier.check{gas: 1_000_000}(t, quote, "", "", new address[](0), new bytes32[](0));
            assertTrue(c.stale, "stale under a modest allowance");
            Eravon inst = deployProxy(t);
            tok.mint(address(inst), 1 ether);
            fundDeposit(premiumOf(q, 1 ether) + 1e17);
            bytes memory bs = buyerSig(t);
            bytes memory ss = sellerSig(t);
            uint256[2] memory gases = [uint256(16_777_216), 2_000_000];
            for (uint256 g = 0; g < 2; g++) {
                vm.prank(SELLER);
                (bool ok, bytes memory ret) =
                    address(inst).call{gas: gases[g]}(abi.encodeCall(Eravon.init, (t, quote, bs, ss, new address[](0), new bytes32[](0))));
                assertFalse(ok);
                assertEq(ret, stale, "RateStale, by name");
            }
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.None), "nothing lands");
        }
    }

    // ==================================================================
    // R2-7 and ADV3-1: no refund during a dispute within T3, in either form
    // ==================================================================

    /// s9: refundBySig is refused (WrongPhase, after the signature) while Disputed, so a Refund() the seller
    /// signed while Funded, handed to the judge in the evidence, cannot pick the refund over the lapse or the
    /// ruling; the seller's own call is refused at or before T3's end and is the plain refund past it. Outside a
    /// dispute the same signature refunds as the seller's call would.
    function test_R2_7_aSignedRefundIsRefusedDuringADispute() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        uint256 a = 100 ether;
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, a);
        bytes32 id = orderHash(t);
        bytes memory refundSig = sign(SELLER_PK, digestOf(address(inst), keccak256(abi.encode(TH_REFUND))));
        uint256 lf = lfOf(q, a);
        uint256 p = premiumOf(q, a);
        claim(inst, MANIFEST);
        uint256 s0 = vm.snapshotState();
        raise(inst);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.refundBySig(refundSig);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Disputed), "nothing moved");
        (, uint256 jc) = inst.accountOf(JUDGE, address(tok));
        assertEq(jc, 0);
        // the seller's own call within T3 is refused too, nothing moving
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.refund();
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Disputed), "nothing moved");
        assertEq(inst.seatOf(id, 0).state, 1);
        // past T3 the seller's own call is the plain refund: A and the fee back to the buyer, P and D to the seller
        vm.warp(vm.getBlockTimestamp() + 14 days + 1);
        uint256 keyBefore0 = tok.balanceOf(SELLER);
        vm.prank(SELLER);
        inst.refund();
        (, jc) = inst.accountOf(JUDGE, address(tok));
        assertEq(jc, 0, "nothing to the judge at a refund");
        assertEq(tok.balanceOf(BUYER_PAYOUT), a + lf, "A and the buyer's own fee back");
        assertEq(tok.balanceOf(SELLER) - keyBefore0, p + lf, "P and the deposit back to the seller");
        assertEq(inst.seatOf(id, 0).state, 5);
        // outside a dispute (Claimed, the seat seated) the same signature refunds: A to the buyer, the deposit
        // less P to the seller, P to the judge as T2's lapse would give it
        vm.revertToState(s0);
        uint256 keyBefore = tok.balanceOf(SELLER);
        vm.prank(BUYER);
        inst.refundBySig(refundSig);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded));
        (, jc) = inst.accountOf(JUDGE, address(tok));
        assertEq(jc, p, "P to the seated judge");
        assertEq(tok.balanceOf(BUYER_PAYOUT), a);
        assertEq(tok.balanceOf(SELLER) - keyBefore, lf);
    }

    /// s8: whatever the last rung's feeBearer, a refund during the dispute is refused within T3 and, past it, pays
    /// the ladder nothing: A and the buyer's fee to the buyer, P and the deposit to the seller, nothing to the judge.
    function test_R2_7_theRefundIsRefusedInADisputeWhateverTheLastRungsBearer() public {
        for (uint8 fb = 0; fb < 3; fb++) {
            uint256 snap = vm.snapshotState();
            Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
            bytes memory quote = encodeQuote(q);
            uint256 a = 100 ether;
            Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, a);
            Eravon.Rung[] memory l = new Eravon.Rung[](2);
            l[0] = Eravon.Rung(0, 10000, 0);
            l[1] = Eravon.Rung(10000, 0, fb);
            t.offer.ladder = l;
            fundDeposit(depositOf(q, a));
            Eravon inst = openNamed(t, quote);
            uint256 lf = lfOf(q, a);
            uint256 p = premiumOf(q, a);
            claim(inst, MANIFEST);
            raise(inst);
            vm.prank(SELLER);
            vm.expectRevert(Eravon.WrongPhase.selector);
            inst.refund();
            vm.warp(vm.getBlockTimestamp() + 14 days + 1);
            vm.expectEmit(true, true, true, true, address(inst));
            emit Eravon.Closed(0, a + lf, p + lf, 0, 0);
            vm.prank(SELLER);
            inst.refund();
            vm.revertToState(snap);
        }
    }

    // ==================================================================
    // R2-8: the block after the arming draws whatever the clock
    // ==================================================================

    /// s7: a guild quote whose offerWindow is shorter than a block (0 or 11 seconds, autoAccept set): the
    /// block after the arming, twelve seconds on and past the deadline, still draws, and the member is seated
    /// with its consent carried; advance closes nothing in that block. From the second block after the arming
    /// the deadline governs: redraw is NotPendingRedraw and advance closes the case as Resigned form 3.
    function test_R2_8_aGuildQuoteWithAShortOfferWindowSeatsInTheNextBlock() public {
        uint32[2] memory windows = [uint32(0), 11];
        for (uint256 k = 0; k < 2; k++) {
            (Eravon inst, Eravon.OrderTerms memory t, address[] memory m, bytes32[] memory w) = _guildCase(3, windows[k], true);
            bytes32 id = orderHash(t);
            (, uint64 deadline,) = _armed(inst);
            assertEq(deadline, uint64(vm.getBlockTimestamp()) + windows[k]);
            uint256 snap = vm.snapshotState();
            vm.roll(vm.getBlockNumber() + 1);
            vm.warp(vm.getBlockTimestamp() + 12);
            assertGt(vm.getBlockTimestamp(), deadline, "the next block is past the deadline");
            inst.advance(id, 0);
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded), "advance closes nothing in the first drawable block");
            vm.prank(SELLER);
            inst.redraw(m, w);
            assertEq(_seats(inst), 1, "drawn in the first drawable block");
            assertEq(inst.seatOf(id, 0).accepted, uint64(vm.getBlockTimestamp()), "consent carried");
            vm.revertToState(snap);
            vm.roll(vm.getBlockNumber() + 2);
            vm.warp(vm.getBlockTimestamp() + 24);
            vm.prank(SELLER);
            vm.expectRevert(Eravon.NotPendingRedraw.selector);
            inst.redraw(m, w);
            inst.advance(id, 0);
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned), "from the second block the deadline governs");
            assertEq(_seats(inst), 0);
        }
    }
}
