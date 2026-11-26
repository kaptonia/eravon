// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {VmSafe} from "forge-std/Vm.sol";
import {EravonBase} from "../EravonBase.sol";
import {Eravon} from "../../src/Eravon.sol";
import {EravonVerifier} from "../../src/EravonVerifier.sol";
import {EravonPanelBook} from "../../src/EravonPanelBook.sol";
import {MockERC20} from "../replay/MockERC20.sol";
import {Company, MockUSD} from "../mocks/Mocks.sol";
import {R4ShapedToken, R4ShapedFeed, R4NoCopy, R4SideSwap, R4SellerWallet, IR4Inst} from "./R4Mocks.sol";

/// Round 4, seat FUZZ. The regression half over every round-3 finding (ADV3-1 to ADV3-4, SEC3-I1 to I4,
/// VULN3-I-1 to I-3, the fuzz seat's R3-L1 and R3-I2 to I4), each round-3 proof ported to the code of
/// ecf085ed and shown to fail or to answer as the revised text says; and the fresh hunt over what round 3
/// changed: the refund refused in a dispute within T3 in both forms, the signed refund testing its signature
/// first, the verifier's fixed-size reads, the push that copies no answer, every answer length at every read,
/// push and pull, and the stale Refund() signature in every phase.
contract EravonRound4 is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE_PAYOUT = address(0x4000000000000000000000000000000000000001);
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    address internal constant STRANGER = address(0x3000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah");
    bytes32 internal constant ROSTER = keccak256("roster");
    bytes32 internal constant MANIFEST = keccak256("manifest");
    bytes internal constant ENTRY = "judgment entry bytes";
    uint256 internal constant TX_GAS = 16_777_216;
    uint256 internal constant PUSH_GAS = 1_000_000;
    bytes32 internal E;

    function setUp() public {
        setUpBase();
        E = sha256(ENTRY);
    }

    // ------------------------------------------------------------------
    // fixtures
    // ------------------------------------------------------------------

    function _open(Q memory q, uint256 amount) internal returns (Eravon inst, Eravon.OrderTerms memory t, bytes memory quote) {
        quote = encodeQuote(q);
        t = namedTerms(JUDGE, AMANAH, quote, amount);
        fundDeposit(depositOf(q, amount));
        inst = openNamed(t, quote);
    }

    function _settleSigs(Eravon inst, uint16 bps, uint256 exp) internal view returns (bytes memory sb, bytes memory ss) {
        bytes32 d = digestOf(address(inst), keccak256(abi.encode(TH_SETTLE, uint256(bps), exp)));
        sb = sign(BUYER_PK, d);
        ss = sign(SELLER_PK, d);
    }

    function _refundSig(Eravon inst) internal view returns (bytes memory) {
        return sign(SELLER_PK, digestOf(address(inst), keccak256(abi.encode(TH_REFUND))));
    }

    function _members(uint256 n) internal pure returns (address[] memory m, bytes32[] memory w) {
        m = new address[](n);
        w = new bytes32[](n);
        for (uint256 i = 0; i < n; i++) {
            m[i] = address(uint160(0x001000000000000000000000000000000000000B01 + i));
            w[i] = AMANAH;
        }
    }

    function _engage(uint256 n) internal returns (address[] memory m, bytes32[] memory w) {
        EravonPanelBook book = deployBook(GUILD);
        (m, w) = _members(n);
        bytes32 key = logic.PROXY_CODEHASH();
        uint64 exp = uint64(vm.getBlockTimestamp() + 3650 days);
        vm.prank(GUILD, GUILD);
        book.engage(key, m, w, ROSTER, exp, address(0));
        vm.roll(vm.getBlockNumber() + 1);
    }

    /// A guild case armed at init (no draw made), under a quote with the given offer window and consent switch.
    function _guildCase(uint256 n, uint32 offerWindow, bool auto_)
        internal
        returns (Eravon inst, Eravon.OrderTerms memory t, address[] memory m, bytes32[] memory w)
    {
        (m, w) = _engage(n);
        Q memory q = defaultQuote(address(tok), address(0), auto_);
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
        Eravon.CaseStatus memory _st36 = inst.status();
        pending = _st36.pendingRedraw;
        deadline = _st36.redrawDeadline;
        drawBlock = _st36.drawBlock;
    }

    function _seats(Eravon inst) internal view returns (uint256 n) {
        Eravon.CaseStatus memory _st37 = inst.status();
        n = _st37.seats;
    }

    function _judgeClaim(Eravon inst, address j, address tk) internal view returns (uint256 c) {
        (, c) = inst.accountOf(j, tk);
    }

    function _closed(VmSafe.Log[] memory logs, address emitter)
        internal
        pure
        returns (bool found, uint8 term, uint256 b, uint256 s, uint256 j, uint256 bond)
    {
        for (uint256 i = 0; i < logs.length; i++) {
            if (logs[i].emitter == emitter && logs[i].topics.length != 0 && logs[i].topics[0] == Eravon.Closed.selector) {
                (term, b, s, j, bond) = abi.decode(logs[i].data, (uint8, uint256, uint256, uint256, uint256));
                return (true, term, b, s, j, bond);
            }
        }
    }

    /// Everything a refund could move on a named case in `tk`: the phase, the instance's and the five payees'
    /// balances, the parties' credits, the judge's claimable, the ledger and the held sums.
    function _digest(Eravon inst, Eravon.OrderTerms memory t, address tk) internal view returns (bytes32) {
        Eravon.CaseStatus memory _st38 = inst.status();
        Eravon.Phase ph = _st38.phase;
        uint256 seats = _st38.seats;
        uint256 prem = _st38.premium;
        uint256 dep = _st38.deposit;
        uint256 lfh = _st38.lfHeld;
        uint256 unpaid = _st38.unpaid;
        bool pending = _st38.pendingRedraw;
        bytes memory a = abi.encode(
            ph,
            seats,
            prem,
            dep,
            lfh,
            unpaid,
            pending,
            _bal(tk, address(inst)),
            _bal(tk, t.buyerPayout),
            _bal(tk, t.offer.sellerPayout),
            _bal(tk, t.offer.sellerKey),
            _bal(tk, JUDGE_PAYOUT)
        );
        bytes memory b = abi.encode(
            inst.owed(t.buyerPayout, tk),
            inst.owed(t.offer.sellerPayout, tk),
            inst.owed(t.offer.sellerKey, tk),
            _judgeClaim(inst, t.offer.judge, tk)
        );
        return keccak256(abi.encodePacked(a, b));
    }

    function _bal(address tk, address who) internal view returns (uint256) {
        (bool ok, bytes memory r) = tk.staticcall(abi.encodeWithSignature("plainBalance(address)", who));
        if (ok && r.length == 32) return abi.decode(r, (uint256));
        return MockERC20(tk).balanceOf(who);
    }

    // ==================================================================
    // Regression: ADV3-1 (low): a refund sent while Claimed executed as the surrender behind a dispute
    // ==================================================================

    /// ADV3-1's proof (R3Race.test_FINDING_ADV3_1_disputeOrderedBeforeTheSellersRefund), ported: the seller sends
    /// refund() while Claimed and the buyer's dispute is ordered first. The surrender is gone: the refund now
    /// reverts WrongPhase throughout T3 (at its first second, its middle and its last), the signed form too, and
    /// nothing moves; the seller leaves the dispute by settle (Settle(10000) costs it P, the surrender cost it
    /// P + lf) or by the ruling; one second past T3 its refund is the plain refund, the judge paid nothing.
    function test_REG_ADV3_1_aDisputeOrderedAheadOfTheRefundLeavesItRefused() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        uint256 a = 100 ether;
        uint256 p = premiumOf(q, a);
        uint256 lf = lfOf(q, a);
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, a);
        bytes32 id = orderHash(t);
        claim(inst, MANIFEST);
        tok.mint(BUYER, lf);
        vm.prank(BUYER);
        tok.approve(address(inst), lf);
        uint256 s0 = vm.snapshotState();
        // what the seller sent: the refund of the Claimed phase (A to the buyer, P to the seated judge as T2's
        // lapse would give it since R3-L1, the seller's LF back)
        vm.prank(SELLER);
        inst.refund();
        assertEq(tok.balanceOf(SELLER), lf, "the seller's LF back");
        assertEq(tok.balanceOf(BUYER_PAYOUT), a, "the buyer whole");
        assertEq(_judgeClaim(inst, JUDGE, address(tok)), p, "P to the seated judge");
        vm.revertToState(s0);
        // the same refund ordered behind the buyer's dispute
        vm.prank(BUYER);
        inst.dispute();
        bytes memory sg = _refundSig(inst);
        uint64 start = inst.seatOf(id, 0).windowStart;
        uint64 end = inst.seatOf(id, 0).windowEnd;
        uint256[3] memory at = [uint256(start), (uint256(start) + end) / 2, uint256(end)];
        for (uint256 k = 0; k < 3; k++) {
            vm.warp(at[k]);
            bytes32 before = _digest(inst, t, address(tok));
            vm.prank(SELLER);
            vm.expectRevert(Eravon.WrongPhase.selector);
            inst.refund();
            vm.prank(BUYER);
            vm.expectRevert(Eravon.WrongPhase.selector);
            inst.refundBySig(sg);
            assertEq(_digest(inst, t, address(tok)), before, "nothing moves");
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Disputed));
        }
        // the seller's way out inside T3: a settlement at 10000 returns the buyer A and its fee and costs the
        // seller P (the surrender took P + lf)
        uint256 s1 = vm.snapshotState();
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        (bytes memory sb, bytes memory ss) = _settleSigs(inst, 10000, exp);
        vm.prank(SELLER);
        inst.settle(10000, exp, sb, ss);
        assertEq(tok.balanceOf(SELLER), lf, "the settlement costs the seller P alone");
        assertEq(tok.balanceOf(BUYER_PAYOUT), a + lf, "the buyer whole, its fee back");
        vm.revertToState(s1);
        // one second past T3: the plain refund, nothing to the judge
        vm.warp(uint256(end) + 1);
        vm.recordLogs();
        vm.prank(SELLER);
        inst.refund();
        (bool found, uint8 term, uint256 b, uint256 s, uint256 j, uint256 bond) = _closed(vm.getRecordedLogs(), address(inst));
        assertTrue(found);
        assertEq(term, 0);
        assertEq(b, a + lf, "A and the fee to the buyer");
        assertEq(s, p + lf, "P and the LF to the seller");
        assertEq(j, 0, "nothing to the judge past T3");
        assertEq(bond, 0);
        assertEq(_judgeClaim(inst, JUDGE, address(tok)), 0);
    }

    /// ADV3-1's exposure (R3Race.test_ADV3_1_exposureIsClaimedOnly), ported: at T2's last second a dispute is still
    /// taken, and the refund ordered behind it is refused instead of surrendering; the signed form fails the
    /// same way (R3Race.test_ADV3_1_theSignedFormFailsSafe_theDirectFormDoesNot: now both forms fail safe).
    function test_REG_ADV3_1_atT2sLastSecondBothFormsFailSafe() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        uint256 a = 100 ether;
        uint256 lf = lfOf(q, a);
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, a);
        claim(inst, MANIFEST);
        vm.warp(inst.seatOf(orderHash(t), 0).windowEnd);
        tok.mint(BUYER, lf);
        vm.prank(BUYER);
        tok.approve(address(inst), lf);
        vm.prank(BUYER);
        inst.dispute();
        bytes32 before = _digest(inst, t, address(tok));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.refund();
        bytes memory sg = _refundSig(inst);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.refundBySig(sg);
        assertEq(_digest(inst, t, address(tok)), before);
        assertEq(tok.balanceOf(SELLER), 0, "the deposit stays held, not surrendered");
        assertEq(_judgeClaim(inst, JUDGE, address(tok)), 0);
    }

    // ==================================================================
    // Regression: R3-L1 (low, this seat): a refund plus a side payment completed the trade without P
    // ==================================================================

    /// R3-L1's proof (test_R3_L1_aRefundAndASideSwapCompleteTheTradeWithoutP), ported: at T2's last second and
    /// one past it the seller (a contract) refunds and collects the price off the instance in one transaction.
    /// The judge is now paid P as the release and a settlement pay it, and the two parties together end exactly
    /// where the release leaves them: the route strips nothing.
    function test_REG_R3_L1_aRefundAndASideSwapNoLongerStripP() public {
        uint256 a = 100 ether;
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.premiumBps = 300;
        uint256 p = premiumOf(q, a);
        uint256 lf = lfOf(q, a);
        bytes memory quote = encodeQuote(q);
        R4SellerWallet wallet = new R4SellerWallet();
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, a);
        t.offer.sellerKey = address(wallet);
        t.offer.sellerFund = address(wallet);
        Eravon inst = deployProxy(t);
        bytes32 id = orderHash(t);
        tok.mint(address(inst), a);
        tok.mint(address(wallet), p + lf);
        wallet.exec(address(tok), abi.encodeWithSignature("approve(address,uint256)", address(logic), p + lf));
        wallet.exec(address(inst), abi.encodeCall(Eravon.init, (t, quote, buyerSig(t), bytes(""), new address[](0), new bytes32[](0))));
        wallet.exec(address(inst), abi.encodeCall(Eravon.claimDelivery, (MANIFEST)));
        uint64 end = inst.seatOf(id, 0).windowEnd;
        uint256 price = a - p / 2;
        for (uint256 k = 0; k < 2; k++) {
            uint256 s0 = vm.snapshotState();
            vm.warp(uint256(end) + k);
            // the release, for the reference
            uint256 s1 = vm.snapshotState();
            vm.prank(BUYER);
            inst.confirm();
            assertEq(_judgeClaim(inst, JUDGE, address(tok)), p, "the release pays the judge P");
            uint256 relSeller = tok.balanceOf(SELLER_PAYOUT) + tok.balanceOf(address(wallet));
            uint256 relBuyer = tok.balanceOf(BUYER_PAYOUT);
            vm.revertToState(s1);
            R4SideSwap swap = new R4SideSwap(IR4Inst(address(inst)), address(tok), BUYER_PAYOUT, SELLER_PAYOUT, price);
            vm.prank(BUYER_PAYOUT);
            tok.approve(address(swap), price);
            wallet.refundAndCollect(IR4Inst(address(inst)), swap);
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded));
            assertEq(_judgeClaim(inst, JUDGE, address(tok)), p, "the judge is paid P by the refund too");
            uint256 sel = tok.balanceOf(SELLER_PAYOUT) + tok.balanceOf(address(wallet));
            uint256 buy = tok.balanceOf(BUYER_PAYOUT);
            assertEq(sel + buy, relSeller + relBuyer, "the two parties together end where the release leaves them");
            assertEq(sel, price + lf, "the seller: the side price and its LF");
            vm.revertToState(s0);
        }
    }

    // ==================================================================
    // Regression: R3-I2, ADV3-3, SEC3-I2, VULN3-I-1 (informational): refundBySig tested the phase first
    // ==================================================================

    /// R3-I2's proof (test_R3_I2_refundBySigTestsThePhaseBeforeTheSignatureWhileDisputed) and VULN3-I-1's, ported:
    /// refundBySig now tests the signature before the phase, so a wrong, empty or foreign signature answers
    /// BadSellerSig in every phase, a dispute included, as s10's general rule says with no exception; the
    /// seller's valid signature answers WrongPhase throughout the dispute and at every terminal.
    function test_REG_R3_I2_refundBySigTestsTheSignatureFirstInEveryPhase() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, 100 ether);
        bytes32 id = orderHash(t);
        bytes memory valid = _refundSig(inst);
        bytes[5] memory bad = [
            new bytes(65),
            bytes(""),
            new bytes(7),
            sign(BUYER_PK, digestOf(address(inst), keccak256(abi.encode(TH_REFUND)))),
            sign(SELLER_PK, digestOf(address(logic), keccak256(abi.encode(TH_REFUND))))
        ];
        claim(inst, MANIFEST);
        raise(inst);
        for (uint256 k = 0; k < bad.length; k++) {
            vm.prank(BUYER);
            vm.expectRevert(Eravon.BadSellerSig.selector);
            inst.refundBySig(bad[k]);
        }
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotSeller.selector);
        inst.refund();
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.refundBySig(valid);
        // past T3 the valid signature is still refused (throughout a dispute), the bad ones by their signature
        vm.warp(inst.seatOf(id, 0).windowEnd + 1);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.refundBySig(valid);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.refundBySig(bad[0]);
        // a terminal
        vm.warp(inst.seatOf(id, 0).windowEnd);
        vm.prank(JUDGE, JUDGE);
        inst.rule(id, 0, E, 0, ENTRY, "");
        for (uint256 k = 0; k < bad.length; k++) {
            vm.prank(SELLER);
            vm.expectRevert(Eravon.BadSellerSig.selector);
            inst.refundBySig(bad[k]);
        }
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.refundBySig(valid);
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotSeller.selector);
        inst.refund();
    }

    // ==================================================================
    // Regression: R3-I3, ADV3-2, SEC3-I1, VULN3-I-2 (informational): advance on a chain that serves no hash
    // ==================================================================

    /// R3-I3's proof (test_R3_I3_onAChainServingNoHashAdvanceWaitsWhereRedrawCloses), SEC3-I1's and VULN3-I-2's:
    /// the code is unchanged and s7 now states it: on a chain that serves no hash redraw closes at once (from the
    /// first drawable block), advance waits for one of its two clocks: past the deadline from the second block on,
    /// or more than 256 blocks; it never closes in the arming block or the one after it.
    function test_REG_R3_I3_advanceOnANoHashChainIsTheTextsRule() public {
        (Eravon inst, Eravon.OrderTerms memory t, address[] memory m, bytes32[] memory w) = _guildCase(3, 3 days, true);
        bytes32 id = orderHash(t);
        (, uint64 deadline, uint64 drawBlock) = _armed(inst);
        uint256 s0 = vm.snapshotState();
        uint256[6] memory gaps = [uint256(1), 2, 100, 255, 256, 257];
        for (uint256 k = 0; k < gaps.length; k++) {
            vm.revertToState(s0);
            vm.roll(uint256(drawBlock) + gaps[k]);
            vm.setBlockhash(drawBlock, bytes32(0));
            uint256 s1 = vm.snapshotState();
            inst.advance(id, 0);
            (bool pending,,) = _armed(inst);
            assertEq(pending, gaps[k] <= 256, "advance closes by the block count alone");
            vm.revertToState(s1);
            vm.prank(SELLER);
            inst.redraw(m, w);
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned), "redraw closes on the lost seed at once");
        }
        // past the deadline: advance closes from the second block on, never in the first
        vm.revertToState(s0);
        vm.roll(uint256(drawBlock) + 1);
        vm.warp(uint256(deadline) + 1);
        inst.advance(id, 0);
        (bool p1,,) = _armed(inst);
        assertTrue(p1, "the block after the arming: advance closes nothing");
        vm.roll(uint256(drawBlock) + 2);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned), "past the deadline from the second block");
    }

    // ==================================================================
    // Regression: R3-I4 (informational, this seat): return bombs raised init's gas sevenfold
    // ==================================================================

    /// R3-I4's proof (test_R3_returnBombFeedsPriceUnderTheCap), ported: two feeds, their decimals, the unit's and
    /// the token's decimals each answer the largest bomb READ_GAS pays for. Round 3 measured init at 865,643 gas
    /// plain and 6,036,135 bombing. The verifier now copies 160 or 32 bytes: the bombing init costs the plain init
    /// plus exactly the six callees' own memory (priced apart by a reader that copies nothing), and prices the same.
    function test_REG_R3_I4_returnBombsCostTheCallerNothing() public {
        R4ShapedToken st = new R4ShapedToken();
        st.setStandard(logic.PROXY_CODEHASH(), address(logic), address(verifier));
        R4ShapedToken unit = new R4ShapedToken();
        unit.setStandard(logic.PROXY_CODEHASH(), address(logic), address(verifier));
        R4ShapedFeed fT = new R4ShapedFeed();
        R4ShapedFeed fU = new R4ShapedFeed();
        R4NoCopy nc = new R4NoCopy();
        st.mark(address(nc), true);
        unit.mark(address(nc), true);
        fT.set(2000e8, vm.getBlockTimestamp(), 8);
        fU.set(1e8, vm.getBlockTimestamp(), 8);
        uint256 bomb = 240_000;
        uint256[2] memory used;
        uint256[2] memory callee;
        uint256[2] memory lfs;
        Q memory q = defaultQuote(address(st), JUDGE_PAYOUT, true);
        q.feeUnit = address(unit);
        q.feedToken = address(fT);
        q.feedUnit = address(fU);
        q.maxStale = 1 days;
        q.feeFixed = 1;
        q.feeMin = 5;
        q.feeMax = 500;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t.offer.token = address(st);
        uint256 s0 = vm.snapshotState();
        for (uint256 k = 0; k < 2; k++) {
            // the callees' own cost, priced by a reader that copies nothing, then the verifier's, from one state
            vm.revertToState(s0);
            _bombAll(k == 0 ? 0 : bomb, fT, fU, st, unit);
            callee[k] = _sixReads(nc, fT, fU, st, unit);
            vm.revertToState(s0);
            _bombAll(k == 0 ? 0 : bomb, fT, fU, st, unit);
            used[k] = _checkGas(nc, t, quote);
            EravonVerifier.Checked memory c = verifier.check{gas: TX_GAS}(t, quote, "", "", new address[](0), new bytes32[](0));
            assertFalse(c.stale, "a bomb within READ_GAS answers");
            lfs[k] = c.lf;
        }
        assertEq(lfs[0], lfs[1], "priced the same");
        emit log_named_uint("check, plain answers, gas", used[0]);
        emit log_named_uint("check, six bombing reads of 240,000 bytes, gas", used[1]);
        emit log_named_uint("the six callees' own extra gas", callee[1] - callee[0]);
        // the caller's own cost is the same within the call overhead of six reads
        uint256 extra = used[1] - used[0];
        assertEq(extra, callee[1] - callee[0], "the caller pays nothing for the answers' length");
    }

    function _bombAll(uint256 bomb, R4ShapedFeed fT, R4ShapedFeed fU, R4ShapedToken st, R4ShapedToken unit) internal {
        fT.setLens(bomb == 0 ? 160 : bomb, bomb == 0 ? 32 : bomb);
        fU.setLens(bomb == 0 ? 160 : bomb, bomb == 0 ? 32 : bomb);
        st.shape(3, bomb == 0 ? 32 : bomb, 0);
        unit.shape(3, bomb == 0 ? 32 : bomb, 0);
    }

    function _sixReads(R4NoCopy nc, R4ShapedFeed fT, R4ShapedFeed fU, R4ShapedToken st, R4ShapedToken unit)
        internal
        returns (uint256 sum)
    {
        (uint256 c1,) = nc.gasOf(address(fT), abi.encodeWithSignature("latestRoundData()"), true);
        (uint256 c2,) = nc.gasOf(address(fT), abi.encodeWithSignature("decimals()"), true);
        (uint256 c3,) = nc.gasOf(address(fU), abi.encodeWithSignature("latestRoundData()"), true);
        (uint256 c4,) = nc.gasOf(address(fU), abi.encodeWithSignature("decimals()"), true);
        (uint256 c5,) = nc.gasOf(address(st), abi.encodeWithSignature("decimals()"), true);
        (uint256 c6,) = nc.gasOf(address(unit), abi.encodeWithSignature("decimals()"), true);
        sum = c1 + c2 + c3 + c4 + c5 + c6;
    }

    // ==================================================================
    // Regression: ADV3-4 (informational, text): the draw's arming calls, its closes, its window
    // ==================================================================

    /// ADV3-4's proofs (R3Draw.test_R9_firstDrawableBlock_everyArming, test_R9_effectiveWindowIsTheShorter,
    /// Adv9Draw.test_T9_noReArm_staleSeedAndDeadline), against s7 and s5 as revised: (a) the three calls that arm
    /// a draw (init, a decline, the advance that runs out an offer clock) each refuse a draw in their own block
    /// and admit one in the next; (b) past the deadline from the second block redraw is NotPendingRedraw and
    /// advance closes; (c) the window is the earlier of the deadline and block drawBlock + 256.
    function test_REG_ADV3_4_theArmingCallsTheClosesAndTheWindow() public {
        (Eravon inst, Eravon.OrderTerms memory t, address[] memory m, bytes32[] memory w) = _guildCase(3, 1 days, false);
        bytes32 id = orderHash(t);
        // (a) init armed: refused in its block, drawn in the next
        vm.prank(SELLER);
        vm.expectRevert(Eravon.DrawPending.selector);
        inst.redraw(m, w);
        vm.roll(vm.getBlockNumber() + 1);
        vm.prank(SELLER);
        inst.redraw(m, w);
        assertEq(_seats(inst), 1);
        // a decline arms
        address j0 = inst.seatOf(id, 0).judge;
        uint256 s0 = vm.snapshotState();
        vm.prank(j0, j0);
        inst.resign(id, 0, E, ENTRY);
        (bool pending,, uint64 db) = _armed(inst);
        assertTrue(pending && db == vm.getBlockNumber(), "a decline arms in its block");
        vm.prank(SELLER);
        vm.expectRevert(Eravon.DrawPending.selector);
        inst.redraw(m, w);
        vm.roll(vm.getBlockNumber() + 1);
        vm.prank(SELLER);
        inst.redraw(m, w);
        assertEq(_seats(inst), 2);
        vm.revertToState(s0);
        // the advance that runs out an offer clock arms
        vm.warp(inst.seatOf(id, 0).windowEnd + 1);
        vm.roll(vm.getBlockNumber() + 1);
        inst.advance(id, 0);
        (pending,, db) = _armed(inst);
        assertTrue(pending && db == vm.getBlockNumber(), "the expired offer's advance arms in its block");
        vm.prank(SELLER);
        vm.expectRevert(Eravon.DrawPending.selector);
        inst.redraw(m, w);
        (, uint64 deadline,) = _armed(inst);
        // (b) past the deadline, from the second block
        uint256 s1 = vm.snapshotState();
        vm.roll(uint256(db) + 2);
        vm.warp(uint256(deadline) + 1);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotPendingRedraw.selector);
        inst.redraw(m, w);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned));
        vm.revertToState(s1);
        // (c) inside the deadline, past block drawBlock + 256: the seed is gone, both doors close
        vm.roll(uint256(db) + 257);
        assertLt(vm.getBlockTimestamp(), deadline);
        uint256 s2 = vm.snapshotState();
        vm.prank(SELLER);
        inst.redraw(m, w);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned), "redraw closes on the lost seed");
        vm.revertToState(s2);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned), "advance closes past 256 blocks");
        vm.revertToState(s2);
        // and at block drawBlock + 256 the seed is still served: the draw seats
        vm.roll(uint256(db) + 256);
        vm.prank(SELLER);
        inst.redraw(m, w);
        assertEq(_seats(inst), 2, "drawn at the 256th block");
    }

    // ==================================================================
    // Regression: SEC3-I3 (informational, liveness and text): the draw's effective window
    // ==================================================================

    /// SEC3-I3's proofs (test_SEC3_I3_oneBlockWithoutADrawEndsAShortWindowCase and the 256-block one): the code is
    /// unchanged (round 2 chose closing over re-arming) and s3 and terms s3.3 now carry the client rule (a quote
    /// whose offerWindow is below 256 block intervals is no guild candidate). Under a 12-second window one block
    /// without a draw ends the case; under 3 days, 257 two-second blocks do; the money is the refund's either way.
    function test_REG_SEC3_I3_aDrawNotMadeInTimeEndsTheCaseWithTheRefundsMoney() public {
        (Eravon i1, Eravon.OrderTerms memory t1, address[] memory m, bytes32[] memory w) = _guildCase(3, 12, true);
        (, , uint64 db1) = _armed(i1);
        vm.roll(uint256(db1) + 2);
        vm.warp(vm.getBlockTimestamp() + 24);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotPendingRedraw.selector);
        i1.redraw(m, w);
        vm.recordLogs();
        i1.advance(orderHash(t1), 0);
        (bool found, uint8 term, uint256 b,, uint256 j,) = _closed(vm.getRecordedLogs(), address(i1));
        assertTrue(found && term == 4 && b == 100 ether && j == 0, "Resigned form 3, the amount to the buyer");
        (Eravon i2, Eravon.OrderTerms memory t2,,) = _guildCase(3, 3 days, true);
        (, , uint64 db2) = _armed(i2);
        vm.roll(uint256(db2) + 257);
        vm.warp(vm.getBlockTimestamp() + 257 * 2);
        i2.advance(orderHash(t2), 0);
        assertEq(uint8(phaseOf(i2)), uint8(Eravon.Phase.Resigned));
    }

    // ==================================================================
    // Regression: VULN3-I-3 (informational, text): a computable BLOCKHASH gives the draw to whoever times the arming
    // ==================================================================

    /// VULN3-I-3's proof (test_fresh_aPredictableHashGivesTheDrawToWhoeverTimesTheArming): the code is unchanged and
    /// s7 now names the actor (where the hash is computable in advance, whoever times the arming transaction).
    /// With BLOCKHASH a public function of the block number, the seller computes each coming block's member and
    /// sends init in the first block whose hash names its target; the draw in the next block seats it.
    function test_REG_VULN3_I3_aComputableHashGivesTheDrawToTheArmingsTimer() public {
        (address[] memory m, bytes32[] memory w) = _engage(4);
        Q memory q = defaultQuote(address(tok), address(0), true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, 100 ether);
        bytes32 id = orderHash(t);
        uint256 b = vm.getBlockNumber();
        // the target: a member the current block's hash does not name, so that the seller has to wait for its block
        address target = m[(uint256(keccak256(abi.encodePacked(id, keccak256(abi.encode(block.chainid, b)), uint64(0)))) + 1) % 4];
        uint256 tries;
        while (true) {
            bytes32 h = keccak256(abi.encode(block.chainid, b));
            if (m[uint256(keccak256(abi.encodePacked(id, h, uint64(0)))) % 4] == target) break;
            b++;
            tries++;
        }
        fundDeposit(q, 100 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        vm.roll(b);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), m, w);
        vm.roll(b + 1);
        vm.setBlockhash(b, keccak256(abi.encode(block.chainid, b)));
        vm.prank(SELLER);
        inst.redraw(m, w);
        assertEq(inst.seatOf(id, 0).judge, target, "the arming's timer chose the member");
        emit log_named_uint("blocks the seller waited", tries);
        assertGt(tries, 0, "the seller waited for the block that names its target");
    }

    // ==================================================================
    // Fresh: the refund in a dispute, at every second of T3 and past it, both forms
    // ==================================================================

    /// s5, s8, s9: at any second of T3 (fuzzed over the whole window, its first and last seconds included) both
    /// forms revert WrongPhase and move nothing, whatever strays sit in the instance; one second or more past T3
    /// the signed form is still WrongPhase and the direct one is the plain refund: A, the strays and the fee to the
    /// buyer, P and the LF to the seller, nothing to the judge; at a random ladder and premium.
    function testFuzz_R4_aRefundInADisputeAtEverySecond(uint256 off, uint256 past, uint256 strays, uint16 prem, uint256 rw) public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.premiumBps = uint16(bound(prem, 0, 10000));
        q.rulingWindow = uint32(bound(rw, 0, 60 days));
        uint256 a = 100 ether;
        uint256 p = premiumOf(q, a);
        uint256 lf = lfOf(q, a);
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, a);
        bytes32 id = orderHash(t);
        claim(inst, MANIFEST);
        uint256 st_ = bound(strays, 0, 1e24);
        if (st_ != 0) tok.mint(address(inst), st_); // sent ahead: the fee comes from it, the rest stays stray
        if (st_ < lf) {
            tok.mint(BUYER, lf - st_);
            vm.prank(BUYER);
            tok.approve(address(inst), lf - st_);
        }
        vm.prank(BUYER);
        inst.dispute();
        uint256 stray = st_ > lf ? st_ - lf : 0;
        uint64 start = inst.seatOf(id, 0).windowStart;
        uint64 end = inst.seatOf(id, 0).windowEnd;
        bytes memory sg = _refundSig(inst);
        uint256 inT3 = uint256(start) + bound(off, 0, uint256(end) - start);
        vm.warp(inT3);
        bytes32 before = _digest(inst, t, address(tok));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.refund();
        vm.prank(BUYER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.refundBySig(sg);
        assertEq(_digest(inst, t, address(tok)), before, "nothing moves inside T3");
        vm.warp(uint256(end) + 1 + bound(past, 0, 365 days));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.refundBySig(sg);
        vm.recordLogs();
        vm.prank(SELLER);
        inst.refund();
        (bool found, uint8 term, uint256 b, uint256 s, uint256 j, uint256 bond) = _closed(vm.getRecordedLogs(), address(inst));
        assertTrue(found && term == 0 && bond == 0);
        assertEq(b, a + stray + lf, "A, the strays and the fee to the buyer");
        assertEq(s, p + lf, "P and the LF to the seller");
        assertEq(j, 0, "nothing to the judge");
        assertEq(_judgeClaim(inst, JUDGE, address(tok)), 0);
        assertEq(inst.seatOf(id, 0).state, 5, "the seat closed");
    }

    // ==================================================================
    // Fresh: a stale Refund() signature submitted in every phase
    // ==================================================================

    struct Probe {
        bool ok;
        bytes4 err;
    }

    function _trySig(Eravon inst, bytes memory sig) internal returns (Probe memory r) {
        return _trySigFrom(BUYER, inst, sig);
    }

    function _trySigFrom(address who, Eravon inst, bytes memory sig) internal returns (Probe memory r) {
        vm.prank(who);
        (bool ok, bytes memory ret) = address(inst).call{gas: TX_GAS}(abi.encodeCall(Eravon.refundBySig, (sig)));
        r.ok = ok;
        if (!ok && ret.length >= 4) r.err = bytes4(ret);
    }

    /// The seller's valid Refund() signature (one signed long before, in another phase) and five bad ones, submitted
    /// by the buyer in every phase a case can stand in: before any case (NotParty: no party key is stored, s10),
    /// after a cancel (WrongPhase: the keys are kept), Funded unaccepted, Funded in T1, a guild draw armed, a guild offer, Claimed seated,
    /// Claimed after a resignation (the valid one lands as the seller's own refund would, to the wei), Disputed
    /// inside and past T3 and every terminal (WrongPhase); the bad ones BadSellerSig everywhere, before the phase.
    function test_R4_aStaleRefundSignatureInEveryPhase() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, false);
        bytes memory quote = encodeQuote(q);
        uint256 a = 100 ether;
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, a);
        Eravon inst = deployProxy(t);
        bytes32 id = orderHash(t);
        bytes memory valid = _refundSig(inst);
        // before any case: no party key, so every sender and every signature is refused NotParty
        _expectSig(inst, valid, false, Eravon.NotParty.selector, "None");
        tok.mint(address(inst), a);
        fundDeposit(q, a);
        uint256 s0 = vm.snapshotState();
        vm.prank(BUYER);
        inst.cancel(t, "");
        _expectSig(inst, valid, false, Eravon.WrongPhase.selector, "after a cancel");
        vm.revertToState(s0);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        _expectLands(inst, t, valid, "Funded, offer unaccepted");
        vm.prank(JUDGE, JUDGE);
        inst.accept(id, 0);
        _expectLands(inst, t, valid, "Funded, T1");
        claim(inst, MANIFEST);
        _expectLands(inst, t, valid, "Claimed, seated");
        uint256 s1 = vm.snapshotState();
        vm.prank(JUDGE, JUDGE);
        inst.resign(id, 0, E, ENTRY);
        _expectLands(inst, t, valid, "Claimed, resigned");
        vm.revertToState(s1);
        raise(inst);
        _expectSig(inst, valid, true, Eravon.WrongPhase.selector, "Disputed, T3");
        uint256 s2 = vm.snapshotState();
        vm.warp(inst.seatOf(id, 0).windowEnd + 1);
        _expectSig(inst, valid, true, Eravon.WrongPhase.selector, "Disputed, past T3");
        vm.revertToState(s2);
        // the terminals
        uint256 s3 = vm.snapshotState();
        vm.prank(JUDGE, JUDGE);
        inst.rule(id, 0, E, 1, ENTRY, "");
        _expectSig(inst, valid, true, Eravon.WrongPhase.selector, "Ruled");
        vm.revertToState(s3);
        vm.warp(inst.seatOf(id, 0).windowEnd + 1);
        inst.advance(id, 0);
        _expectSig(inst, valid, true, Eravon.WrongPhase.selector, "Lapsed");
        vm.revertToState(s3);
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        (bytes memory sb, bytes memory ss) = _settleSigs(inst, 3000, exp);
        vm.prank(SELLER);
        inst.settle(3000, exp, sb, ss);
        _expectSig(inst, valid, true, Eravon.WrongPhase.selector, "Settled");
        vm.revertToState(s1);
        vm.prank(BUYER);
        inst.confirm();
        _expectSig(inst, valid, true, Eravon.WrongPhase.selector, "Released");
        vm.revertToState(s1);
        vm.prank(SELLER);
        inst.refund();
        _expectSig(inst, valid, true, Eravon.WrongPhase.selector, "Refunded");
        vm.revertToState(s0);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        vm.prank(JUDGE, JUDGE);
        inst.resign(id, 0, E, ENTRY);
        _expectSig(inst, valid, true, Eravon.WrongPhase.selector, "Resigned");
        // the guild track: a draw armed, then an offer
        (Eravon g, Eravon.OrderTerms memory tg, address[] memory m, bytes32[] memory w) = _guildCase(3, 1 days, false);
        bytes memory gv = _refundSig(g);
        _expectLands(g, tg, gv, "a draw armed");
        vm.roll(vm.getBlockNumber() + 1);
        vm.prank(SELLER);
        g.redraw(m, w);
        _expectLands(g, tg, gv, "a guild offer");
    }

    /// The valid signature refused with `want` (or the bad ones refused by their signature), nothing moving.
    function _expectSig(Eravon inst, bytes memory valid, bool caseExists, bytes4 want, string memory where) internal {
        bytes[5] memory bad = [
            new bytes(65),
            bytes(""),
            new bytes(64),
            sign(BUYER_PK, digestOf(address(inst), keccak256(abi.encode(TH_REFUND)))),
            sign(SELLER_PK, digestOf(address(0xBEEF), keccak256(abi.encode(TH_REFUND))))
        ];
        uint256 b0 = tok.balanceOf(address(inst));
        Probe memory r = _trySigFrom(STRANGER, inst, valid);
        assertTrue(!r.ok && r.err == Eravon.NotParty.selector, string(abi.encodePacked("a stranger: ", where)));
        r = _trySig(inst, valid);
        assertTrue(!r.ok && r.err == want, string(abi.encodePacked("the valid signature: ", where)));
        bytes4 badWant = want == Eravon.NotParty.selector ? want : Eravon.BadSellerSig.selector;
        for (uint256 k = 0; k < bad.length; k++) {
            r = _trySig(inst, bad[k]);
            assertTrue(!r.ok && r.err == badWant, string(abi.encodePacked("a bad signature: ", where)));
        }
        assertEq(tok.balanceOf(address(inst)), b0, "nothing moves");
        if (!caseExists) assertEq(uint8(phaseOf(inst)) == 0 || uint8(phaseOf(inst)) == 4, true, "no live case");
    }

    /// Outside a dispute the valid signature lands exactly as the seller's own refund does (the same Closed, the
    /// same balances and claimable), and the bad ones are refused by their signature.
    function _expectLands(Eravon inst, Eravon.OrderTerms memory t, bytes memory valid, string memory where) internal {
        uint256 s = vm.snapshotState();
        vm.recordLogs();
        vm.prank(SELLER);
        inst.refund();
        (, uint8 t0, uint256 b0, uint256 s0, uint256 j0,) = _closed(vm.getRecordedLogs(), address(inst));
        bytes32 d0 = _digest(inst, t, address(tok));
        vm.revertToState(s);
        bytes[2] memory bad = [new bytes(65), sign(BUYER_PK, digestOf(address(inst), keccak256(abi.encode(TH_REFUND))))];
        for (uint256 k = 0; k < bad.length; k++) {
            Probe memory r0 = _trySig(inst, bad[k]);
            assertTrue(!r0.ok && r0.err == Eravon.BadSellerSig.selector, string(abi.encodePacked("a bad signature: ", where)));
        }
        vm.recordLogs();
        Probe memory r = _trySig(inst, valid);
        assertTrue(r.ok, string(abi.encodePacked("the valid signature lands: ", where)));
        (, uint8 t1, uint256 b1, uint256 s1, uint256 j1,) = _closed(vm.getRecordedLogs(), address(inst));
        assertTrue(t0 == t1 && b0 == b1 && s0 == s1 && j0 == j1, string(abi.encodePacked("the same Closed: ", where)));
        assertEq(_digest(inst, t, address(tok)), d0, string(abi.encodePacked("the same outcome: ", where)));
        vm.revertToState(s);
    }

    /// A seller that is a contract has no key that signs: every Refund() blob answers BadSellerSig in every phase.
    function test_R4_aCompanySellersRefundBySigIsAlwaysBadSellerSig() public {
        Company co = new Company();
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 100 ether);
        t.offer.sellerKey = address(co);
        t.offer.sellerFund = address(co);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        uint256 d = depositOf(q, 100 ether);
        tok.mint(address(co), d);
        co.exec(address(tok), abi.encodeCall(MockERC20.approve, (address(logic), d)));
        co.exec(address(inst), abi.encodeCall(Eravon.init, (t, quote, buyerSig(t), bytes(""), new address[](0), new bytes32[](0))));
        bytes memory any_ = _refundSig(inst);
        Probe memory r = _trySig(inst, any_);
        assertTrue(!r.ok && r.err == Eravon.BadSellerSig.selector);
        co.exec(address(inst), abi.encodeCall(Eravon.claimDelivery, (MANIFEST)));
        raise(inst);
        r = _trySig(inst, any_);
        assertTrue(!r.ok && r.err == Eravon.BadSellerSig.selector, "the signature first, while Disputed too");
    }

    // ==================================================================
    // Fresh: every answer length at every push
    // ==================================================================

    struct Run {
        bool ok;
        bytes4 err;
        uint256 gas;
        bytes32 outcome;
        uint256 transfers;
        uint256 maxEntry;
    }

    function _shapedCase(R4ShapedToken st, Q memory q, uint256 a, bool open_)
        internal
        returns (Eravon inst, Eravon.OrderTerms memory t, bytes memory quote)
    {
        quote = encodeQuote(q);
        t = namedTerms(JUDGE, AMANAH, quote, a);
        t.offer.token = address(st);
        uint256 d = depositOf(q, a);
        st.mint(SELLER, d);
        vm.prank(SELLER);
        st.approve(address(logic), d);
        inst = deployProxy(t);
        st.mint(address(inst), a);
        if (open_) {
            bytes memory bs = buyerSig(t);
            bytes memory ss = sellerSig(t);
            vm.prank(SELLER);
            inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        }
    }

    function _run(R4ShapedToken st, Eravon inst, Eravon.OrderTerms memory t, address from, bytes memory data)
        internal
        returns (Run memory r)
    {
        st.resetMeter();
        vm.prank(from);
        uint256 g0 = gasleft();
        (bool ok, bytes memory ret) = address(inst).call{gas: TX_GAS}(data);
        r.gas = g0 - gasleft();
        r.ok = ok;
        if (!ok && ret.length >= 4) r.err = bytes4(ret);
        r.outcome = _digest(inst, t, address(st));
        r.transfers = st.transfers();
        r.maxEntry = st.maxEntryGas();
    }

    /// The callee's own gas for one shaped transfer (a reader that copies nothing), in a snapshot of its own.
    function _transferCost(R4ShapedToken st, R4NoCopy nc) internal returns (uint256 c) {
        uint256 s = vm.snapshotState();
        (c,) = nc.gasOf(address(st), abi.encodeWithSignature("transfer(address,uint256)", address(0xCAFE), uint256(0)), false);
        vm.revertToState(s);
    }

    /// s8: a push copies back no return data and counts by the balance that left. At the three pushes of a
    /// terminal (a settlement), at cancel's, at the expiry branch's, and at sweep's and withdraw's, the token's
    /// transfer answers 0, 31, 32, 33, 159, 160, 161, 4,096 or 600,000 bytes whose first word is true, false or
    /// garbage, the balance moved each time: the outcome is the same to the wei, the push of the standard's own is
    /// handed at most PUSH_GAS (sweep and withdraw all the caller's gas), and the caller's gas grows by exactly
    /// the callees' own cost of their answers, nothing of its own.
    function test_R4_everyAnswerLengthAtEveryPush() public {
        uint256[9] memory lens = [uint256(0), 31, 32, 33, 159, 160, 161, 4096, 600_000];
        for (uint256 site = 0; site < 4; site++) {
            uint256 s0 = vm.snapshotState();
            R4ShapedToken st = new R4ShapedToken();
            st.setStandard(logic.PROXY_CODEHASH(), address(logic), address(verifier));
            R4NoCopy nc = new R4NoCopy();
            st.mark(address(nc), true);
            Q memory q = defaultQuote(address(st), JUDGE_PAYOUT, true);
            (Eravon inst, Eravon.OrderTerms memory t,) = _shapedCase(st, q, 100 ether, site != 1 && site != 2);
            address from = SELLER;
            bytes memory data;
            if (site == 0) {
                // a settlement in Claimed: three pushes (buyer's payout, seller's payout, seller's key)
                vm.prank(SELLER);
                inst.claimDelivery(MANIFEST);
                uint256 exp = vm.getBlockTimestamp() + 1 days;
                (bytes memory sb, bytes memory ss) = _settleSigs(inst, 4000, exp);
                data = abi.encodeCall(Eravon.settle, (uint16(4000), exp, sb, ss));
            } else if (site == 1) {
                from = BUYER;
                data = abi.encodeCall(Eravon.cancel, (t, bytes("")));
            } else if (site == 2) {
                vm.warp(t.openBy + 1);
                data = abi.encodeCall(Eravon.init, (t, bytes(""), bytes(""), bytes(""), new address[](0), new bytes32[](0)));
            } else {
                // a release whose pushes fail (the payouts refuse), then sweep by a party and the judge's withdraw
                vm.prank(SELLER);
                inst.claimDelivery(MANIFEST);
                st.setRefuses(SELLER_PAYOUT, true);
                vm.prank(BUYER);
                inst.confirm();
                st.setRefuses(SELLER_PAYOUT, false);
                data = abi.encodeCall(Eravon.sweep, (SELLER_PAYOUT, address(st)));
            }
            uint256 s1 = vm.snapshotState();
            Run memory base;
            uint256 baseCost;
            for (uint256 k = 0; k < lens.length; k++) {
                for (uint8 val = 0; val < 3; val++) {
                    vm.revertToState(s1);
                    st.shape(0, lens[k], val);
                    uint256 cost = _transferCost(st, nc);
                    Run memory r = _run(st, inst, t, from, data);
                    assertTrue(r.ok, "the action lands whatever the push answers");
                    if (k == 2 && val == 0) {
                        base = r;
                        baseCost = cost;
                    }
                    if (site != 3) assertLe(r.maxEntry, PUSH_GAS, "a push of the standard's own under the allowance");
                    else assertGt(r.maxEntry, PUSH_GAS, "sweep hands the token the caller's gas");
                    if (k > 2 || (k == 2 && val != 0)) {
                        assertEq(r.outcome, base.outcome, "the same outcome whatever the answer");
                        assertEq(r.transfers, base.transfers, "the same pushes");
                        int256 own = int256(r.gas) - int256(base.gas) - int256(r.transfers) * (int256(cost) - int256(baseCost));
                        assertEq(own, 0, "the caller pays nothing for the answer's length");
                    }
                }
                vm.revertToState(s1);
            }
            // the judge's withdraw, on the release's credit
            if (site == 3) {
                vm.revertToState(s1);
                uint256 s2 = vm.snapshotState();
                Run memory wb;
                uint256 wc;
                for (uint256 k = 0; k < lens.length; k++) {
                    vm.revertToState(s2);
                    st.shape(0, lens[k], uint8(k % 3));
                    uint256 cost = _transferCost(st, nc);
                    Run memory r = _run(st, inst, t, JUDGE, abi.encodeCall(Eravon.withdraw, (address(st), JUDGE, JUDGE_PAYOUT)));
                    assertTrue(r.ok, "withdraw lands whatever the push answers");
                    if (k == 0) {
                        wb = r;
                        wc = cost;
                    } else {
                        assertEq(r.outcome, wb.outcome);
                        int256 own = int256(r.gas) - int256(wb.gas) - int256(r.transfers) * (int256(cost) - int256(wc));
                        assertEq(own, 0, "withdraw's caller pays nothing for the answer's length");
                    }
                }
            }
            vm.revertToState(s0);
        }
    }

    /// s8: a push counts by the balance that left, whatever the token answered: a token that moves the balance and
    /// answers false, or nothing, has paid (the credit is debited); one that answers true and moves nothing has not
    /// (the credit stays owed for sweep).
    function test_R4_aPushThatMovesAndAnswersFalseOrNothingHasPaid() public {
        for (uint256 m = 0; m < 3; m++) {
            uint256 s0 = vm.snapshotState();
            R4ShapedToken st = new R4ShapedToken();
            st.setStandard(logic.PROXY_CODEHASH(), address(logic), address(verifier));
            Q memory q = defaultQuote(address(st), JUDGE_PAYOUT, true);
            (Eravon inst,,) = _shapedCase(st, q, 100 ether, true);
            if (m == 0) st.shape(0, 32, 1); // moves, answers false
            if (m == 1) st.shape(0, 0, 0); // moves, answers nothing
            if (m == 2) {
                st.shape(0, 32, 2); // refuses, answers a garbage word (nonzero: "true" to a lenient reader)
                st.setRefuses(BUYER_PAYOUT, true);
            }
            vm.prank(SELLER);
            inst.refund();
            if (m < 2) {
                assertEq(st.plainBalance(BUYER_PAYOUT), 100 ether, "delivered");
                assertEq(inst.owed(BUYER_PAYOUT, address(st)), 0, "debited by what left");
            } else {
                assertEq(st.plainBalance(BUYER_PAYOUT), 0, "nothing delivered");
                assertEq(inst.owed(BUYER_PAYOUT, address(st)), 100 ether, "the credit stays owed");
                st.setRefuses(BUYER_PAYOUT, false);
                vm.prank(SELLER);
                inst.sweep(BUYER_PAYOUT, address(st));
                assertEq(st.plainBalance(BUYER_PAYOUT), 100 ether, "sweep delivers it");
            }
            vm.revertToState(s0);
        }
    }

    // ==================================================================
    // Fresh: every answer length at every pricing read
    // ==================================================================

    /// s3: each of the six reads the pricing makes (feedToken's round and decimals(), feedUnit's round and
    /// decimals(), the settlement token's and the unit's decimals()) answers, one at a time, every length around
    /// its floor (0, 31, 32, 33, 159, 160, 161 bytes), 4,096 bytes, and a bomb READ_GAS pays for: below the floor
    /// (160 for a round, 32 for a decimals()) the read is stale and init reverts RateStale; at or above it the
    /// verifier prices exactly as with the plain answer, and the caller's gas grows by the callee's own cost alone.
    function test_R4_everyAnswerLengthAtEveryPricingRead() public {
        R4ShapedToken st = new R4ShapedToken();
        st.setStandard(logic.PROXY_CODEHASH(), address(logic), address(verifier));
        R4ShapedToken unit = new R4ShapedToken();
        unit.setStandard(logic.PROXY_CODEHASH(), address(logic), address(verifier));
        R4ShapedFeed fT = new R4ShapedFeed();
        R4ShapedFeed fU = new R4ShapedFeed();
        R4NoCopy nc = new R4NoCopy();
        st.mark(address(nc), true);
        unit.mark(address(nc), true);
        fT.set(2000e8, vm.getBlockTimestamp(), 8);
        fU.set(1e8, vm.getBlockTimestamp(), 8);
        Q memory q = defaultQuote(address(st), JUDGE_PAYOUT, true);
        q.feeUnit = address(unit);
        q.feedToken = address(fT);
        q.feedUnit = address(fU);
        q.maxStale = 1 days;
        q.feeFixed = 1e6;
        q.feeMin = 5e6;
        q.feeMax = 500e6;
        q.amountCap = 1e40;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t.offer.token = address(st);
        EravonVerifier.Checked memory plain = verifier.check(t, quote, "", "", new address[](0), new bytes32[](0));
        assertFalse(plain.stale);
        uint256 gPlain = _checkGas(nc, t, quote);
        uint256[9] memory lens = [uint256(0), 31, 32, 33, 159, 160, 161, 4096, 240_000];
        for (uint256 site = 0; site < 6; site++) {
            for (uint256 k = 0; k < lens.length; k++) {
                uint256 s = vm.snapshotState();
                uint256 floor_ = (site == 0 || site == 2) ? 160 : 32;
                (address who, bytes memory call_) = _shapeRead(site, lens[k], fT, fU, st, unit);
                (uint256 cPlain,) = (0, 0);
                {
                    uint256 s2 = vm.snapshotState();
                    _shapeRead(site, floor_, fT, fU, st, unit);
                    (cPlain,) = nc.gasOf(who, call_, true);
                    vm.revertToState(s2);
                    _shapeRead(site, lens[k], fT, fU, st, unit);
                }
                (uint256 cNow,) = nc.gasOf(who, call_, true);
                EravonVerifier.Checked memory c = verifier.check(t, quote, "", "", new address[](0), new bytes32[](0));
                if (lens[k] < floor_) {
                    assertTrue(c.stale, "below the floor: stale");
                } else {
                    assertFalse(c.stale, "at or above the floor: priced");
                    assertEq(c.lf, plain.lf);
                    assertEq(c.cap, plain.cap);
                    assertEq(c.priceToken, plain.priceToken);
                    assertEq(c.priceUnit, plain.priceUnit);
                    assertEq(c.rateAt, plain.rateAt);
                    uint256 g = _checkGas(nc, t, quote);
                    // a round at its floor is the plain read itself; the caller pays the callee's extra alone
                    int256 own = int256(g) - int256(gPlain) - (int256(cNow) - int256(cPlain));
                    assertEq(own, 0, "the caller pays nothing for the answer's length");
                }
                vm.revertToState(s);
            }
        }
        // and through init: a short round is RateStale by name, after the consents
        uint256 s3 = vm.snapshotState();
        fT.setLens(159, 32);
        uint256 d = premiumOf(q, 1 ether) + plain.lf;
        st.mint(SELLER, d);
        vm.prank(SELLER);
        st.approve(address(logic), d);
        Eravon inst = deployProxy(t);
        st.mint(address(inst), 1 ether);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.RateStale.selector);
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        fT.setLens(240_000, 240_000);
        vm.prank(SELLER);
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        (uint256 lf,,,,) = inst.feeQuoted();
        assertEq(lf, plain.lf, "a bombing feed prices the case as the plain one");
        vm.revertToState(s3);
    }

    /// The verifier's own gas for `check`, through a reader that copies nothing back (so the test's own memory
    /// is no part of the measure).
    function _checkGas(R4NoCopy nc, Eravon.OrderTerms memory t, bytes memory quote) internal returns (uint256 used) {
        (used,) = nc.gasOf(
            address(verifier), abi.encodeCall(EravonVerifier.check, (t, quote, "", "", new address[](0), new bytes32[](0))), true
        );
    }

    function _shapeRead(uint256 site, uint256 len, R4ShapedFeed fT, R4ShapedFeed fU, R4ShapedToken st, R4ShapedToken unit)
        internal
        returns (address who, bytes memory call_)
    {
        if (site == 0) {
            fT.setLens(len, 32);
            return (address(fT), abi.encodeWithSignature("latestRoundData()"));
        }
        if (site == 1) {
            fT.setLens(160, len);
            return (address(fT), abi.encodeWithSignature("decimals()"));
        }
        if (site == 2) {
            fU.setLens(len, 32);
            return (address(fU), abi.encodeWithSignature("latestRoundData()"));
        }
        if (site == 3) {
            fU.setLens(160, len);
            return (address(fU), abi.encodeWithSignature("decimals()"));
        }
        if (site == 4) {
            st.shape(3, len, 0);
            return (address(st), abi.encodeWithSignature("decimals()"));
        }
        unit.shape(3, len, 0);
        return (address(unit), abi.encodeWithSignature("decimals()"));
    }

    /// s3: a read whose bomb is larger than READ_GAS pays for runs out inside the callee and reads as no answer:
    /// RateStale by name, under the transaction cap, at each of the six reads.
    function test_R4_aBombPastReadGasIsNoAnswer() public {
        R4ShapedToken st = new R4ShapedToken();
        st.setStandard(logic.PROXY_CODEHASH(), address(logic), address(verifier));
        R4ShapedToken unit = new R4ShapedToken();
        unit.setStandard(logic.PROXY_CODEHASH(), address(logic), address(verifier));
        R4ShapedFeed fT = new R4ShapedFeed();
        R4ShapedFeed fU = new R4ShapedFeed();
        fT.set(2000e8, vm.getBlockTimestamp(), 8);
        fU.set(1e8, vm.getBlockTimestamp(), 8);
        Q memory q = defaultQuote(address(st), JUDGE_PAYOUT, true);
        q.feeUnit = address(unit);
        q.feedToken = address(fT);
        q.feedUnit = address(fU);
        q.maxStale = 1 days;
        q.feeMin = 5e6;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t.offer.token = address(st);
        for (uint256 site = 0; site < 6; site++) {
            uint256 s = vm.snapshotState();
            _shapeRead(site, 400_000, fT, fU, st, unit);
            EravonVerifier.Checked memory c = verifier.check{gas: TX_GAS}(t, quote, "", "", new address[](0), new bytes32[](0));
            assertTrue(c.stale, "a bomb past READ_GAS reads as no answer");
            vm.revertToState(s);
        }
    }

    // ==================================================================
    // Fresh: every answer length at the two pulls
    // ==================================================================

    /// s4 step 7, s8: the deposit's pull (the logic's transferFrom at init) and the fee's pull (the instance's at a
    /// dispute) succeed only on no data or exactly one 32-byte word equal to 1, as the text says, the balance moved
    /// either way: 31, 33, 64, 159, 160 or 161 bytes, a false or garbage word, are TransferFailed by name, and
    /// nothing lands.
    function test_R4_everyAnswerLengthAtThePulls() public {
        uint256[10] memory lens = [uint256(0), 31, 32, 32, 32, 33, 64, 159, 160, 161];
        uint8[10] memory vals = [uint8(0), 0, 0, 1, 2, 0, 0, 0, 0, 0];
        for (uint256 k = 0; k < lens.length; k++) {
            bool good = lens[k] == 0 || (lens[k] == 32 && vals[k] == 0);
            uint256 s0 = vm.snapshotState();
            R4ShapedToken st = new R4ShapedToken();
            st.setStandard(logic.PROXY_CODEHASH(), address(logic), address(verifier));
            Q memory q = defaultQuote(address(st), JUDGE_PAYOUT, true);
            (Eravon inst, Eravon.OrderTerms memory t, bytes memory quote) = _shapedCase(st, q, 100 ether, false);
            st.shape(1, lens[k], vals[k]);
            bytes memory bs = buyerSig(t);
            bytes memory ss = sellerSig(t);
            vm.prank(SELLER);
            (bool ok, bytes memory ret) =
                address(inst).call{gas: TX_GAS}(abi.encodeCall(Eravon.init, (t, quote, bs, ss, new address[](0), new bytes32[](0))));
            if (good) {
                assertTrue(ok, "init's pull: no data or one word equal to 1");
            } else {
                assertFalse(ok);
                assertEq(bytes4(ret), Eravon.TransferFailed.selector, "init's pull: TransferFailed by name");
                assertEq(uint8(phaseOf(inst)), 0, "nothing lands");
                st.shape(1, 32, 0);
                vm.prank(SELLER);
                inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
                st.shape(1, lens[k], vals[k]);
            }
            vm.prank(SELLER);
            inst.claimDelivery(MANIFEST);
            uint256 lf = lfOf(q, 100 ether);
            st.mint(BUYER, lf);
            vm.prank(BUYER);
            st.approve(address(inst), lf);
            vm.prank(BUYER);
            (ok, ret) = address(inst).call{gas: TX_GAS}(abi.encodeCall(Eravon.dispute, ()));
            if (good) {
                assertTrue(ok, "the fee's pull: no data or one word equal to 1");
                assertEq(uint8(phaseOf(inst)), 3);
            } else {
                assertFalse(ok);
                assertEq(bytes4(ret), Eravon.TransferFailed.selector, "the fee's pull: TransferFailed by name");
                assertEq(uint8(phaseOf(inst)), 2, "still Claimed");
            }
            vm.revertToState(s0);
        }
    }

    // ==================================================================
    // Fresh: every answer length at the balance reads
    // ==================================================================

    /// The instance's balance reads (`_balance`): an answer of 32 bytes or more is read by its first word, so 33,
    /// 159, 160, 161 and 4,096 bytes give the same outcome as 32; 0 or 31 bytes read as no balance (init is
    /// Underfunded; a terminal counts no push and leaves every credit owed). Measured here: the read copies the
    /// whole answer back (Solidity's staticcall), so the caller pays its length, quadratically: a refund that costs
    /// 294,767 gas at 32 bytes costs 1,465,282 at 128 KiB, 9,704,642 at 384 KiB, and at 512 KiB it no longer lands
    /// under the transaction cap (out of gas, no name). See this seat's FUZZ4-I1.
    function test_R4_everyAnswerLengthAtTheBalanceReads() public {
        uint256[7] memory same = [uint256(32), 33, 159, 160, 161, 4096, 65536];
        R4ShapedToken st = new R4ShapedToken();
        st.setStandard(logic.PROXY_CODEHASH(), address(logic), address(verifier));
        R4NoCopy nc = new R4NoCopy();
        st.mark(address(nc), true);
        Q memory q = defaultQuote(address(st), JUDGE_PAYOUT, true);
        (Eravon inst, Eravon.OrderTerms memory t,) = _shapedCase(st, q, 100 ether, true);
        uint256 s0 = vm.snapshotState();
        Run memory base;
        for (uint256 k = 0; k < same.length; k++) {
            vm.revertToState(s0);
            st.shape(2, same[k], 0);
            Run memory r = _run(st, inst, t, SELLER, abi.encodeCall(Eravon.refund, ()));
            assertTrue(r.ok);
            if (k == 0) base = r;
            else assertEq(r.outcome, base.outcome, "the same outcome at 32 bytes or more");
        }
        // short answers: no balance, the pushes count nothing, every credit stays owed
        vm.revertToState(s0);
        st.shape(2, 31, 0);
        Run memory rs = _run(st, inst, t, SELLER, abi.encodeCall(Eravon.refund, ()));
        assertTrue(rs.ok);
        assertEq(st.plainBalance(BUYER_PAYOUT), 100 ether, "the token moved the amount");
        assertEq(inst.owed(BUYER_PAYOUT, address(st)), 100 ether, "but the instance counted nothing: the credit stays owed");
        // the measurement of FUZZ4-I1: the caller pays the copy
        uint256[4] memory big = [uint256(131072), 262144, 393216, 524288];
        for (uint256 k = 0; k < big.length; k++) {
            vm.revertToState(s0);
            st.shape(2, 32, 0);
            (uint256 c32,) = nc.gasOf(address(st), abi.encodeWithSignature("balanceOf(address)", address(inst)), true);
            st.shape(2, big[k], 0);
            (uint256 cBig,) = nc.gasOf(address(st), abi.encodeWithSignature("balanceOf(address)", address(inst)), true);
            Run memory r = _run(st, inst, t, SELLER, abi.encodeCall(Eravon.refund, ()));
            emit log_named_uint("balanceOf answer, bytes", big[k]);
            emit log_named_uint("  the refund lands (1) or runs out under the cap (0)", r.ok ? 1 : 0);
            emit log_named_uint("  gas", r.gas);
            emit log_named_uint("  the five reads' callee extra", 5 * (cBig - c32));
            if (r.ok) {
                assertEq(r.outcome, base.outcome);
                assertGt(r.gas - base.gas, 5 * (cBig - c32), "the caller pays beyond the callees' own gas");
            } else {
                assertEq(r.err, bytes4(0), "out of gas, no name");
                assertEq(uint8(phaseOf(inst)), 1, "the case cannot close");
            }
        }
    }

    /// FUZZ4-I1's comparison: the same refund under a token whose balanceOf spends every unit of gas it is handed
    /// (the class the copy belongs to: the settlement token's own reads are trusted whatever they cost): each read
    /// leaves the caller a sixty-fourth, and the refund runs out under the transaction cap, no name, as at a
    /// 512 KiB answer; with the plain token it lands.
    function test_R4_aBalanceReadThatBurnsItsGasFreezesTheCaseToo() public {
        R4ShapedToken st = new R4ShapedToken();
        st.setStandard(logic.PROXY_CODEHASH(), address(logic), address(verifier));
        Q memory q = defaultQuote(address(st), JUDGE_PAYOUT, true);
        (Eravon inst, Eravon.OrderTerms memory t,) = _shapedCase(st, q, 100 ether, true);
        uint256 s0 = vm.snapshotState();
        st.setBurnBalance(true);
        Run memory r = _run(st, inst, t, SELLER, abi.encodeCall(Eravon.refund, ()));
        assertFalse(r.ok, "the refund cannot land");
        assertEq(r.err, bytes4(0), "out of gas, no name");
        assertEq(uint8(phaseOf(inst)), 1);
        vm.revertToState(s0);
        Run memory p = _run(st, inst, t, SELLER, abi.encodeCall(Eravon.refund, ()));
        assertTrue(p.ok, "the plain token's refund lands");
    }

    /// FUZZ4-I1, the pulls: the logic's pull and the instance's fee pull copy the token's whole transferFrom answer
    /// (Solidity's call), so a long answer, refused by name (TransferFailed) as the text says, costs the caller its
    /// length; past about 2 MiB the call runs out under the transaction cap, no name, instead.
    function test_R4_aPullsLongAnswerIsRefusedByNameUntilItRunsOut() public {
        uint256[4] memory lens = [uint256(64), 524_288, 1_048_576, 2_097_152];
        for (uint256 k = 0; k < lens.length; k++) {
            uint256 s0 = vm.snapshotState();
            R4ShapedToken st = new R4ShapedToken();
            st.setStandard(logic.PROXY_CODEHASH(), address(logic), address(verifier));
            Q memory q = defaultQuote(address(st), JUDGE_PAYOUT, true);
            (Eravon inst, Eravon.OrderTerms memory t, bytes memory quote) = _shapedCase(st, q, 100 ether, false);
            st.shape(1, lens[k], 0);
            bytes memory bs = buyerSig(t);
            bytes memory ss = sellerSig(t);
            vm.prank(SELLER);
            uint256 g0 = gasleft();
            (bool ok, bytes memory ret) =
                address(inst).call{gas: TX_GAS}(abi.encodeCall(Eravon.init, (t, quote, bs, ss, new address[](0), new bytes32[](0))));
            uint256 used = g0 - gasleft();
            assertFalse(ok);
            emit log_named_uint("transferFrom answer, bytes", lens[k]);
            emit log_named_uint("  init's gas", used);
            emit log_named_uint("  refused by name (1) or out of gas (0)", ret.length == 4 ? 1 : 0);
            if (k < 2) assertEq(bytes4(ret), Eravon.TransferFailed.selector, "TransferFailed by name");
            if (k == 3) assertEq(ret.length, 0, "out of gas under the cap, no name");
            vm.revertToState(s0);
        }
    }
}
