// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {IGaleedVenue} from "../src/IGaleedVenue.sol";
import {Forwarder} from "./replay/VenueReplay.sol";
import {Company} from "./mocks/Mocks.sol";
/// Standard s5, s6, s8 on the named track: every row of the terminal table
/// with its Closed event and the balances behind it, the four clocks and the
/// last second of each, the resignation forms and what follows each, the
/// state guards on every party action. A = 100, P = 1, lf = 10, D = 11.
contract EravonLifecycle is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE_PAYOUT = address(0x4000000000000000000000000000000000000001);
    address internal constant STRANGER = address(0x3000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah");
    bytes32 internal constant MANIFEST = keccak256("manifest");
    bytes internal constant ENTRY = "judgment entry bytes";
    bytes32 internal E;
    uint256 internal constant A = 100 ether;

    Q internal qAuto;
    Q internal qManual;
    bytes internal quoteAuto;
    bytes internal quoteManual;

    function setUp() public {
        setUpBase();
        E = sha256(ENTRY);
        qAuto = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        qManual = defaultQuote(address(tok), JUDGE_PAYOUT, false);
        quoteAuto = encodeQuote(qAuto);
        quoteManual = encodeQuote(qManual);
    }

    // ---- helpers ----

    function openAuto() internal returns (Eravon inst, Eravon.OrderTerms memory t) {
        t = namedTerms(JUDGE, AMANAH, quoteAuto, A);
        fundDeposit(qAuto, A);
        inst = openNamed(t, quoteAuto);
    }

    function openManual() internal returns (Eravon inst, Eravon.OrderTerms memory t) {
        t = namedTerms(JUDGE, AMANAH, quoteManual, A);
        fundDeposit(qManual, A);
        inst = openNamed(t, quoteManual);
    }

    function expectClosed(Eravon inst, uint8 term, uint256 b, uint256 s, uint256 j, uint256 bond) internal {
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Closed(term, b, s, j, bond);
    }

    function judgeClaimable(Eravon inst) internal view returns (uint256 c) {
        (, c) = inst.accountOf(JUDGE, address(tok));
    }

    function seat(Eravon inst, bytes32 id, uint64 n) internal view returns (IGaleedVenue.Seat memory) {
        return inst.seatOf(id, n);
    }

    function settleSig(Eravon inst, uint256 pk, uint16 bps, uint256 exp) internal view returns (bytes memory) {
        return sign(pk, digestOf(address(inst), keccak256(abi.encode(TH_SETTLE, uint256(bps), exp))));
    }

    function ruleAs(Eravon inst, bytes32 id, uint64 rung) internal {
        vm.prank(JUDGE, JUDGE);
        inst.rule(id, 0, E, rung, ENTRY, "");
    }

    function resignAs(Eravon inst, bytes32 id) internal {
        vm.prank(JUDGE, JUDGE);
        inst.resign(id, 0, E, ENTRY);
    }

    /// The three balances and the judge's ledger after a terminal.
    function assertMoney(Eravon inst, uint256 buyerP, uint256 sellerP, uint256 sellerK, uint256 judgeC) internal view {
        assertEq(tok.balanceOf(BUYER_PAYOUT), buyerP, "buyerPayout");
        assertEq(tok.balanceOf(SELLER_PAYOUT), sellerP, "sellerPayout");
        assertEq(tok.balanceOf(SELLER), sellerK, "sellerKey");
        assertEq(judgeClaimable(inst), judgeC, "judge claimable");
        assertEq(tok.balanceOf(address(inst)), judgeC, "the instance keeps the judge's credit alone");
        Eravon.CaseStatus memory _st19 = inst.status();
        uint256 unpaid = _st19.unpaid;
        assertEq(unpaid, judgeC, "unpaid is the judge's credit");
    }

    // ---- the table: Released ----

    function test_released_confirm() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        bytes32 id = orderHash(t);
        claim(inst, MANIFEST);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Claimed));
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Confirmed();
        expectClosed(inst, 1, 0, 110 ether, 1 ether, 0);
        vm.prank(BUYER);
        inst.confirm();
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released));
        assertMoney(inst, 0, 100 ether, 10 ether, 1 ether);
        assertEq(seat(inst, id, 0).state, 5, "closed");
        // the judge's fee leaves to the payout the judge names at its first withdrawal
        (address p,) = inst.accountOf(JUDGE, address(tok));
        assertEq(p, address(0), "no payout until the judge sets one");
        vm.prank(JUDGE);
        vm.expectRevert(Eravon.NoPayout.selector);
        inst.withdraw(address(tok), JUDGE, address(0));
        vm.prank(JUDGE);
        inst.withdraw(address(tok), JUDGE, JUDGE_PAYOUT);
        (p,) = inst.accountOf(JUDGE, address(tok));
        assertEq(p, JUDGE_PAYOUT);
        assertEq(tok.balanceOf(JUDGE_PAYOUT), 1 ether);
        assertEq(judgeClaimable(inst), 0);
        assertEq(tok.balanceOf(address(inst)), 0);
    }

    function test_released_confirmBySig() public {
        (Eravon inst,) = openAuto();
        claim(inst, MANIFEST);
        bytes memory sig = sign(BUYER_PK, digestOf(address(inst), keccak256(abi.encode(TH_CONFIRM))));
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.confirmBySig(sig);
        vm.prank(SELLER);
        inst.confirmBySig(sig);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released));
    }

    function test_released_T2() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        bytes32 id = orderHash(t);
        claim(inst, MANIFEST);
        uint64 end = seat(inst, id, 0).windowEnd;
        assertEq(end, uint64(vm.getBlockTimestamp()) + 5 days, "T2 is confirmWindow from the claim");
        vm.warp(end);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Claimed), "the last second is still inside");
        vm.warp(end + 1);
        expectClosed(inst, 1, 0, 110 ether, 1 ether, 0);
        vm.prank(STRANGER);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released));
        assertEq(seat(inst, id, 0).state, 5);
        assertMoney(inst, 0, 100 ether, 10 ether, 1 ether);
    }

    // ---- the table: Refunded ----

    function test_refunded_T1() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        bytes32 id = orderHash(t);
        IGaleedVenue.Seat memory s = seat(inst, id, 0);
        assertEq(s.accepted, uint64(vm.getBlockTimestamp()), "autoAccept carried the consent");
        assertEq(s.windowEnd, uint64(vm.getBlockTimestamp()) + 7 days, "T1");
        vm.warp(s.windowEnd + 1);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Refunded(0);
        expectClosed(inst, 0, 100 ether, 11 ether, 0, 0);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded));
        assertEq(seat(inst, id, 0).state, 5);
        assertMoney(inst, 100 ether, 0, 11 ether, 0);
    }

    function test_refunded_offerClock() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openManual();
        bytes32 id = orderHash(t);
        IGaleedVenue.Seat memory s = seat(inst, id, 0);
        assertEq(s.accepted, 0);
        assertEq(s.windowEnd, uint64(vm.getBlockTimestamp()) + 3 days, "the offer clock");
        vm.warp(s.windowEnd + 1);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Refunded(1);
        expectClosed(inst, 0, 100 ether, 11 ether, 0, 0);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded));
        assertEq(seat(inst, id, 0).state, 5);
        assertMoney(inst, 100 ether, 0, 11 ether, 0);
    }

    function test_refunded_sellerRefund_eachOpenState() public {
        // Funded
        (Eravon inst,) = openAuto();
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Refunded(3);
        expectClosed(inst, 0, 100 ether, 11 ether, 0, 0);
        vm.prank(SELLER);
        inst.refund();
        assertMoney(inst, 100 ether, 0, 11 ether, 0);
        // Claimed under a seated seat, by signature sent by the buyer: P to the judge, as T2's lapse would give it
        (Eravon inst2,) = openAuto();
        claim(inst2, MANIFEST);
        bytes memory rs = sign(SELLER_PK, digestOf(address(inst2), keccak256(abi.encode(TH_REFUND))));
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst2.refundBySig(rs);
        expectClosed(inst2, 0, 100 ether, 10 ether, 1 ether, 0);
        vm.prank(BUYER);
        inst2.refundBySig(rs);
        assertEq(uint8(phaseOf(inst2)), uint8(Eravon.Phase.Refunded));
        assertEq(judgeClaimable(inst2), 1 ether, "P to the judge: delivery was claimed under a seated seat");
        // Disputed, within T3: both forms are refused and nothing moves (s8)
        (Eravon inst3, Eravon.OrderTerms memory t3) = openAuto();
        claim(inst3, MANIFEST);
        raise(inst3);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst3.refund();
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst3.refundBySig(sign(SELLER_PK, digestOf(address(inst3), keccak256(abi.encode(TH_REFUND)))));
        assertEq(uint8(phaseOf(inst3)), uint8(Eravon.Phase.Disputed));
        assertEq(seat(inst3, orderHash(t3), 0).state, 1);
        assertEq(judgeClaimable(inst3), 0, "nothing to the judge");
    }

    function test_refunded_sellerRefund_inDisputePastT3_isThePlainRefund() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        claim(inst, MANIFEST);
        raise(inst);
        vm.warp(seat(inst, orderHash(t), 0).windowEnd + 1);
        // T3 ran out and nobody advanced: the refund is the plain one, the fee back to the buyer, P to the seller
        expectClosed(inst, 0, 110 ether, 11 ether, 0, 0);
        vm.prank(SELLER);
        inst.refund();
        assertEq(seat(inst, orderHash(t), 0).state, 5);
        assertMoney(inst, 110 ether, 0, 11 ether, 0);
    }

    // ---- the table: Settled ----

    function test_settled_seated_claimedAndDisputed() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        claim(inst, MANIFEST);
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Settled(3000);
        expectClosed(inst, 2, 30 ether, 80 ether, 1 ether, 0);
        vm.prank(BUYER);
        inst.settle(3000, exp, settleSig(inst, BUYER_PK, 3000, exp), settleSig(inst, SELLER_PK, 3000, exp));
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Settled));
        assertEq(seat(inst, orderHash(t), 0).state, 5);
        assertMoney(inst, 30 ether, 70 ether, 10 ether, 1 ether);
        // Disputed: the fee returns to the buyer, P still to the judge
        (Eravon inst2, Eravon.OrderTerms memory t2) = openAuto();
        claim(inst2, MANIFEST);
        raise(inst2);
        expectClosed(inst2, 2, 40 ether, 80 ether, 1 ether, 0);
        vm.prank(SELLER);
        inst2.settle(3000, exp, settleSig(inst2, BUYER_PK, 3000, exp), settleSig(inst2, SELLER_PK, 3000, exp));
        assertEq(seat(inst2, orderHash(t2), 0).state, 5);
        assertEq(tok.balanceOf(BUYER_PAYOUT), 70 ether);
        // Funded: open too
        (Eravon inst3,) = openAuto();
        expectClosed(inst3, 2, 100 ether, 10 ether, 1 ether, 0);
        vm.prank(SELLER);
        inst3.settle(10000, exp, settleSig(inst3, BUYER_PK, 10000, exp), settleSig(inst3, SELLER_PK, 10000, exp));
    }

    function test_settled_consentForms() public {
        (Eravon inst,) = openAuto();
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes memory sB = settleSig(inst, BUYER_PK, 5000, exp);
        bytes memory sS = settleSig(inst, SELLER_PK, 5000, exp);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BpsOver.selector);
        inst.settle(10001, exp, sB, sS);
        vm.warp(exp + 1);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.SettleExpired.selector);
        inst.settle(5000, exp, sB, sS);
        vm.warp(exp - 1);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.settle(5000, exp, settleSig(inst, BUYER_PK, 4000, exp), sS);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.settle(5000, exp, sB, settleSig(inst, SELLER_PK, 5000, exp + 1));
        // a stranger is no party, whatever it carries
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.settle(5000, exp, sB, sS);
        // the buyer's own call carries its consent; the seller signed
        vm.prank(BUYER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.settle(5000, exp, "", "");
        // the seller's registry entry on the logic
        bytes32 d = digestOf(address(inst), keccak256(abi.encode(TH_SETTLE, uint256(5000), exp)));
        vm.prank(SELLER);
        logic.consent(SELLER, d);
        assertTrue(logic.consented(SELLER, d));
        vm.prank(BUYER);
        inst.settle(5000, exp, "", "");
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Settled));
        // a company buyer settles by its own call
        Company co = new Company();
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quoteAuto, A);
        t.buyerKey = address(co);
        Eravon inst2 = deployProxy(t);
        tok.mint(address(inst2), A);
        fundDeposit(qAuto, A);
        co.exec(address(inst2), abi.encodeCall(inst2.init, (t, quoteAuto, "", sellerSig(t), new address[](0), new bytes32[](0))));
        co.exec(address(inst2), abi.encodeCall(inst2.settle, (2500, exp, "", settleSig(inst2, SELLER_PK, 2500, exp))));
        assertEq(uint8(phaseOf(inst2)), uint8(Eravon.Phase.Settled));
    }

    // ---- the table: Ruled ----

    function test_ruled_eachRung() public {
        uint16[3] memory buyerBps = [uint16(0), 5000, 10000];
        uint256[3] memory b = [uint256(0), 55 ether, 110 ether];
        uint256[3] memory s = [uint256(110 ether), 55 ether, 0];
        uint256[3] memory bond = [uint256(0), 5 ether, 10 ether];
        uint256[3] memory sellerP = [uint256(100 ether), 50 ether, 0];
        uint256[3] memory sellerK = [uint256(10 ether), 5 ether, 0];
        for (uint64 r = 0; r < 3; r++) {
            (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
            bytes32 id = orderHash(t);
            claim(inst, MANIFEST);
            raise(inst);
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Disputed));
            assertEq(seat(inst, id, 0).state, 1);
            vm.expectEmit(true, true, true, true, address(inst));
            emit IGaleedVenue.Anchored(JUDGE, E, ENTRY);
            vm.expectEmit(true, true, true, true, address(inst));
            emit Eravon.Ruled(r, E);
            expectClosed(inst, 3, b[r], s[r], 11 ether, bond[r]);
            ruleAs(inst, id, r);
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Ruled));
            assertEq(seat(inst, id, 0).state, 2, "ruled");
            assertEq(judgeClaimable(inst), 11 ether, "P and the buyer's fee");
            assertEq(tok.balanceOf(SELLER_PAYOUT), sellerP[r]);
            assertEq(tok.balanceOf(SELLER), sellerK[r]);
            assertEq(tok.balanceOf(BUYER_PAYOUT), b[r]);
            assertEq(buyerBps[r], t.offer.ladder[r].buyerBps);
            // reset the payouts for the next rung
            uint256 bb = tok.balanceOf(BUYER_PAYOUT);
            uint256 sb = tok.balanceOf(SELLER_PAYOUT);
            uint256 kb = tok.balanceOf(SELLER);
            vm.prank(BUYER_PAYOUT);
            tok.transfer(address(0xdead), bb);
            vm.prank(SELLER_PAYOUT);
            tok.transfer(address(0xdead), sb);
            vm.prank(SELLER);
            tok.transfer(address(0xdead), kb);
        }
    }

    function test_ruled_guards() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        bytes32 id = orderHash(t);
        claim(inst, MANIFEST);
        // not disputed yet
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.NotOpen.selector);
        inst.rule(id, 0, E, 1, ENTRY, "");
        raise(inst);
        // not the judge, not direct
        vm.prank(STRANGER, STRANGER);
        vm.expectRevert(Eravon.NotDirect.selector);
        inst.rule(id, 0, E, 1, ENTRY, "");
        Forwarder f = new Forwarder();
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.NotDirect.selector);
        f.forward(address(inst), abi.encodeCall(inst.rule, (id, 0, E, 1, ENTRY, "")));
        vm.prank(JUDGE, STRANGER);
        vm.expectRevert(Eravon.NotDirect.selector);
        inst.rule(id, 0, E, 1, ENTRY, "");
        // rung, extra, entry
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.RungOutOfRange.selector);
        inst.rule(id, 0, E, 3, ENTRY, "");
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.ExtraNotEmpty.selector);
        inst.rule(id, 0, E, 1, ENTRY, hex"00");
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.EntryMismatch.selector);
        inst.rule(id, 0, E, 1, "other bytes", "");
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.EntryMismatch.selector);
        inst.rule(id, 0, bytes32(0), 1, "", "");
        // unknown seat and case
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.UnknownSeat.selector);
        inst.rule(id, 1, E, 1, ENTRY, "");
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.UnknownSeat.selector);
        inst.rule(keccak256("other"), 0, E, 1, ENTRY, "");
        // the last second of T3 is taken, the next refused
        uint64 end = seat(inst, id, 0).windowEnd;
        assertEq(end, uint64(vm.getBlockTimestamp()) + 14 days, "T3 is rulingWindow from the dispute");
        vm.warp(end + 1);
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.WindowClosed.selector);
        inst.rule(id, 0, E, 1, ENTRY, "");
        vm.warp(end);
        ruleAs(inst, id, 1);
        // a second ruling
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.NotOpen.selector);
        inst.rule(id, 0, E, 1, ENTRY, "");
    }

    // ---- the table: Lapsed ----

    function test_lapsed_T3_lapseDefault2() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        bytes32 id = orderHash(t);
        claim(inst, MANIFEST);
        raise(inst);
        uint64 end = seat(inst, id, 0).windowEnd;
        vm.warp(end + 1);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Lapsed(10000);
        expectClosed(inst, 5, 110 ether, 11 ether, 0, 0);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Lapsed));
        assertEq(seat(inst, id, 0).state, 4, "lapsed");
        assertMoney(inst, 110 ether, 0, 11 ether, 0);
    }

    function test_lapsed_T3_lapseDefault0() public {
        Q memory q0 = qAuto;
        q0.lapseDefault = 0;
        bytes memory quote0 = encodeQuote(q0);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote0, A);
        fundDeposit(q0, A);
        Eravon inst = openNamed(t, quote0);
        bytes32 id = orderHash(t);
        claim(inst, MANIFEST);
        raise(inst);
        vm.warp(seat(inst, id, 0).windowEnd + 1);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Lapsed(0);
        expectClosed(inst, 5, 10 ether, 111 ether, 0, 0);
        inst.advance(id, 0);
        assertMoney(inst, 10 ether, 100 ether, 11 ether, 0);
    }

    function test_lapsed_T3_lapseDefault1() public {
        Q memory q1 = qAuto;
        q1.lapseDefault = 1;
        bytes memory quote1 = encodeQuote(q1);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote1, A);
        fundDeposit(q1, A);
        Eravon inst = openNamed(t, quote1);
        bytes32 id = orderHash(t);
        claim(inst, MANIFEST);
        raise(inst);
        vm.warp(seat(inst, id, 0).windowEnd + 1);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Lapsed(5000);
        expectClosed(inst, 5, 60 ether, 61 ether, 0, 0);
        inst.advance(id, 0);
        assertMoney(inst, 60 ether, 50 ether, 11 ether, 0);
    }

    // ---- resignation forms ----

    function test_resigned_declined() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openManual();
        bytes32 id = orderHash(t);
        vm.expectEmit(true, true, true, true, address(inst));
        emit IGaleedVenue.Anchored(JUDGE, E, ENTRY);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Resigned(E, 0);
        expectClosed(inst, 4, 100 ether, 11 ether, 0, 0);
        resignAs(inst, id);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned));
        assertEq(seat(inst, id, 0).state, 3);
        assertMoney(inst, 100 ether, 0, 11 ether, 0);
    }

    function test_resigned_declineOfAStaleOfferStaysOpen() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openManual();
        bytes32 id = orderHash(t);
        vm.warp(seat(inst, id, 0).windowEnd + 1);
        // past the offer clock an unaccepted seat may still decline; the clock's outcome is the same money
        expectClosed(inst, 4, 100 ether, 11 ether, 0, 0);
        resignAs(inst, id);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned));
    }

    function test_resigned_funded_form1() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        bytes32 id = orderHash(t);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Resigned(E, 1);
        expectClosed(inst, 4, 100 ether, 11 ether, 0, 0);
        resignAs(inst, id);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned));
        assertMoney(inst, 100 ether, 0, 11 ether, 0);
    }

    function test_resigned_claimed_thenConfirm() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        bytes32 id = orderHash(t);
        claim(inst, MANIFEST);
        uint64 end = seat(inst, id, 0).windowEnd;
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Resigned(E, 1);
        resignAs(inst, id);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Claimed), "the case runs on");
        IGaleedVenue.Seat memory s = seat(inst, id, 0);
        assertEq(s.state, 3, "resigned");
        assertEq(s.windowEnd, end, "T2 kept");
        Eravon.CaseStatus memory _st20 = inst.status();
        bool pending = _st20.pendingRedraw;
        assertFalse(pending, "no redraw on the named track");
        // P back to the seller on release
        expectClosed(inst, 1, 0, 111 ether, 0, 0);
        vm.prank(BUYER);
        inst.confirm();
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released));
        assertEq(seat(inst, id, 0).state, 3, "the seat stays resigned");
        assertMoney(inst, 0, 100 ether, 11 ether, 0);
    }

    function test_resigned_claimed_thenT2() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        bytes32 id = orderHash(t);
        claim(inst, MANIFEST);
        resignAs(inst, id);
        uint64 end = seat(inst, id, 0).windowEnd;
        vm.warp(end);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Claimed));
        vm.warp(end + 1);
        expectClosed(inst, 1, 0, 111 ether, 0, 0);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released));
        assertEq(seat(inst, id, 0).state, 3);
        assertMoney(inst, 0, 100 ether, 11 ether, 0);
    }

    function test_resigned_claimed_thenDispute() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        bytes32 id = orderHash(t);
        claim(inst, MANIFEST);
        resignAs(inst, id);
        // no fee is pulled: the buyer holds nothing and grants nothing
        assertEq(tok.balanceOf(BUYER), 0);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Disputed();
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Lapsed(10000);
        expectClosed(inst, 5, 100 ether, 11 ether, 0, 0);
        vm.prank(BUYER);
        inst.dispute();
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Lapsed));
        assertEq(seat(inst, id, 0).state, 3, "the seat stays resigned");
        assertMoney(inst, 100 ether, 0, 11 ether, 0);
        // the dispute by signature takes the same path
        (Eravon inst2, Eravon.OrderTerms memory t2) = openAuto();
        claim(inst2, MANIFEST);
        resignAs(inst2, orderHash(t2));
        expectClosed(inst2, 5, 100 ether, 11 ether, 0, 0);
        vm.prank(SELLER);
        inst2.disputeBySig(sign(BUYER_PK, digestOf(address(inst2), keccak256(abi.encode(TH_DISPUTE)))));
        // past T2 the dispute is refused and only the clock remains
        (Eravon inst3, Eravon.OrderTerms memory t3) = openAuto();
        claim(inst3, MANIFEST);
        resignAs(inst3, orderHash(t3));
        vm.warp(seat(inst3, orderHash(t3), 0).windowEnd + 1);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.WindowClosed.selector);
        inst3.dispute();
    }

    function test_resigned_claimed_thenSettleOrRefund() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        claim(inst, MANIFEST);
        resignAs(inst, orderHash(t));
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        expectClosed(inst, 2, 30 ether, 81 ether, 0, 0);
        vm.prank(SELLER);
        inst.settle(3000, exp, settleSig(inst, BUYER_PK, 3000, exp), settleSig(inst, SELLER_PK, 3000, exp));
        assertEq(seat(inst, orderHash(t), 0).state, 3);
        assertMoney(inst, 30 ether, 70 ether, 11 ether, 0);
        (Eravon inst2, Eravon.OrderTerms memory t2) = openAuto();
        claim(inst2, MANIFEST);
        resignAs(inst2, orderHash(t2));
        expectClosed(inst2, 0, 100 ether, 11 ether, 0, 0);
        vm.prank(SELLER);
        inst2.refund();
        assertEq(seat(inst2, orderHash(t2), 0).state, 3);
    }

    function test_resigned_disputed_form2() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        bytes32 id = orderHash(t);
        claim(inst, MANIFEST);
        raise(inst);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Resigned(E, 2);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Lapsed(10000);
        expectClosed(inst, 5, 110 ether, 11 ether, 0, 0);
        resignAs(inst, id);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Lapsed));
        assertEq(seat(inst, id, 0).state, 3);
        assertMoney(inst, 110 ether, 0, 11 ether, 0);
    }

    function test_resign_guards() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        bytes32 id = orderHash(t);
        vm.prank(STRANGER, STRANGER);
        vm.expectRevert(Eravon.NotDirect.selector);
        inst.resign(id, 0, E, ENTRY);
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.EntryMismatch.selector);
        inst.resign(id, 0, E, "other");
        // an accepted seat past its window has only the clock's outcome
        vm.warp(seat(inst, id, 0).windowEnd + 1);
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.WindowClosed.selector);
        inst.resign(id, 0, E, ENTRY);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded));
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.NotOpen.selector);
        inst.resign(id, 0, E, ENTRY);
    }

    // ---- clocks and the last second ----

    function test_clocks_claimLastSecond() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        bytes32 id = orderHash(t);
        uint64 end = seat(inst, id, 0).windowEnd;
        vm.warp(end + 1);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WindowClosed.selector);
        inst.claimDelivery(MANIFEST);
        vm.warp(end);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.DeliveryClaimed(MANIFEST);
        vm.expectEmit(true, true, true, true, address(inst));
        emit IGaleedVenue.Committed(id, SELLER, MANIFEST);
        claim(inst, MANIFEST);
        IGaleedVenue.Seat memory s = seat(inst, id, 0);
        assertEq(s.windowStart, end);
        assertEq(s.windowEnd, end + 5 days);
        Eravon.CaseStatus memory _st21 = inst.status();
        uint64 claimedAt = _st21.claimedAt;
        assertEq(claimedAt, end);
    }

    function test_clocks_disputeLastSecond() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        bytes32 id = orderHash(t);
        claim(inst, MANIFEST);
        uint64 end = seat(inst, id, 0).windowEnd;
        tok.mint(BUYER, 10 ether);
        vm.prank(BUYER);
        tok.approve(address(inst), 10 ether);
        vm.warp(end + 1);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.WindowClosed.selector);
        inst.dispute();
        vm.warp(end);
        vm.prank(BUYER);
        inst.dispute();
        IGaleedVenue.Seat memory s = seat(inst, id, 0);
        assertEq(s.state, 1);
        assertEq(s.windowEnd, end + 14 days);
        Eravon.CaseStatus memory _st22 = inst.status();
        uint256 lfHeld = _st22.lfHeld;
        uint64 disputedAt = _st22.disputedAt;
        assertEq(lfHeld, 10 ether);
        assertEq(disputedAt, end);
    }

    function test_clocks_advanceNothingDue() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        bytes32 id = orderHash(t);
        bytes32 before = keccak256(abi.encode(seat(inst, id, 0)));
        inst.advance(id, 0);
        inst.advance(id, 7);
        inst.advance(keccak256("other"), 0);
        assertEq(keccak256(abi.encode(seat(inst, id, 0))), before, "nothing changed");
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
        // after the terminal nothing is due either
        claim(inst, MANIFEST);
        vm.prank(BUYER);
        inst.confirm();
        vm.warp(vm.getBlockTimestamp() + 365 days);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released));
    }

    function test_clocks_acceptOnTheManualTrack() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openManual();
        bytes32 id = orderHash(t);
        vm.prank(STRANGER, STRANGER);
        vm.expectRevert(Eravon.NotDirect.selector);
        inst.accept(id, 0);
        // the seller cannot claim before the judge accepts
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotOpen.selector);
        inst.claimDelivery(MANIFEST);
        uint64 end = seat(inst, id, 0).windowEnd;
        vm.warp(end + 1);
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.WindowClosed.selector);
        inst.accept(id, 0);
        vm.warp(end);
        vm.prank(JUDGE, JUDGE);
        inst.accept(id, 0);
        IGaleedVenue.Seat memory s = seat(inst, id, 0);
        assertEq(s.accepted, end);
        assertEq(s.windowStart, end);
        assertEq(s.windowEnd, end + 7 days, "T1 from the acceptance");
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.NotOpen.selector);
        inst.accept(id, 0);
        // the auto track's seat is accepted at init and refuses a second acceptance
        (Eravon inst2, Eravon.OrderTerms memory t2) = openAuto();
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.NotOpen.selector);
        inst2.accept(orderHash(t2), 0);
    }

    // ---- state guards on the party actions ----

    function test_guards_partyActions() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        bytes32 id = orderHash(t);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.confirm();
        vm.prank(BUYER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.dispute();
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotSeller.selector);
        inst.claimDelivery(MANIFEST);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.ZeroManifest.selector);
        inst.claimDelivery(bytes32(0));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.ZeroManifest.selector);
        inst.claimDeliveryBySig(bytes32(0), new bytes(65));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.claimDeliveryBySig(MANIFEST, sign(BUYER_PK, digestOf(address(inst), keccak256(abi.encode(TH_CLAIM, MANIFEST)))));
        vm.prank(SELLER);
        inst.claimDeliveryBySig(MANIFEST, sign(SELLER_PK, digestOf(address(inst), keccak256(abi.encode(TH_CLAIM, MANIFEST)))));
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Claimed));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.claimDelivery(MANIFEST);
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotBuyer.selector);
        inst.confirm();
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotBuyer.selector);
        inst.dispute();
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotSeller.selector);
        inst.refund();
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.confirmBySig(sign(SELLER_PK, digestOf(address(inst), keccak256(abi.encode(TH_CONFIRM)))));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.refundBySig(sign(BUYER_PK, digestOf(address(inst), keccak256(abi.encode(TH_REFUND)))));
        // commit from anyone while open
        vm.expectEmit(true, true, true, true, address(inst));
        emit IGaleedVenue.Committed(id, STRANGER, keccak256("evidence"));
        vm.prank(STRANGER);
        inst.commit(id, keccak256("evidence"));
        vm.expectRevert(Eravon.NotOpen.selector);
        inst.commit(keccak256("other"), keccak256("evidence"));
        vm.prank(BUYER);
        inst.confirm();
        // after the terminal every action is refused
        vm.prank(BUYER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.confirm();
        vm.prank(BUYER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.dispute();
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.refund();
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.claimDelivery(MANIFEST);
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.settle(5000, exp, settleSig(inst, BUYER_PK, 5000, exp), settleSig(inst, SELLER_PK, 5000, exp));
        vm.expectRevert(Eravon.NotOpen.selector);
        inst.commit(id, keccak256("late evidence"));
        vm.prank(JUDGE, JUDGE);
        vm.expectRevert(Eravon.NotOpen.selector);
        inst.resign(id, 0, E, ENTRY);
    }

    // ---- the buyer's fee ----

    function test_fee_sentAheadAndPulled() public {
        // sent ahead in full: no allowance needed
        (Eravon inst,) = openAuto();
        claim(inst, MANIFEST);
        tok.mint(address(inst), 10 ether);
        vm.prank(BUYER);
        inst.dispute();
        Eravon.CaseStatus memory _st23 = inst.status();
        uint256 lfHeld = _st23.lfHeld;
        assertEq(lfHeld, 10 ether);
        // sent ahead in part: the rest pulled
        (Eravon inst2,) = openAuto();
        claim(inst2, MANIFEST);
        tok.mint(address(inst2), 4 ether);
        tok.mint(BUYER, 6 ether);
        vm.prank(BUYER);
        tok.approve(address(inst2), 6 ether);
        vm.prank(BUYER);
        inst2.dispute();
        assertEq(tok.balanceOf(BUYER), 0, "exactly the rest was pulled");
        // more than the fee sent ahead: the excess is the buyer's stray, returned at the terminal
        (Eravon inst3, Eravon.OrderTerms memory t3) = openAuto();
        claim(inst3, MANIFEST);
        tok.mint(address(inst3), 13 ether);
        vm.prank(BUYER);
        inst3.dispute();
        vm.warp(seat(inst3, orderHash(t3), 0).windowEnd + 1);
        vm.prank(SELLER);
        inst3.refund();
        assertEq(tok.balanceOf(BUYER_PAYOUT), 113 ether + 0, "A, the fee, and the 3 stray");
    }

    function test_fee_shortRefusesTheDispute() public {
        (Eravon inst,) = openAuto();
        claim(inst, MANIFEST);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.TransferFailed.selector);
        inst.dispute();
        tok.mint(BUYER, 9 ether);
        vm.prank(BUYER);
        tok.approve(address(inst), 10 ether);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.TransferFailed.selector);
        inst.dispute();
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Claimed), "nothing moved");
        tok.mint(BUYER, 1 ether);
        vm.prank(BUYER);
        inst.dispute();
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Disputed));
    }

    // ---- strays, sweep, withdraw, setPayout ----

    function test_strays_returnToTheBuyerAtTheTerminal() public {
        (Eravon inst,) = openAuto();
        tok.mint(address(inst), 7 ether); // while Funded
        claim(inst, MANIFEST);
        expectClosed(inst, 1, 7 ether, 110 ether, 1 ether, 0);
        vm.prank(BUYER);
        inst.confirm();
        assertEq(tok.balanceOf(BUYER_PAYOUT), 7 ether);
        // after the terminal: sweep credits later inflows to the buyer's payout and pushes them when it is named
        tok.mint(address(inst), 2 ether);
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.sweep(BUYER_PAYOUT, address(tok));
        vm.prank(SELLER);
        inst.sweep(STRANGER, address(tok));
        assertEq(inst.owed(BUYER_PAYOUT, address(tok)), 2 ether, "credited");
        assertEq(tok.balanceOf(BUYER_PAYOUT), 7 ether);
        vm.prank(BUYER);
        inst.sweep(BUYER_PAYOUT, address(tok));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 9 ether);
        assertEq(tok.balanceOf(address(inst)), 1 ether, "the judge's credit stays");
        // sweep pushes the judge's credit to the payout the judge registered; the seated judge may sweep too
        vm.prank(SELLER);
        inst.sweep(JUDGE, address(tok));
        assertEq(tok.balanceOf(address(inst)), 1 ether, "no payout registered: the credit stays");
        vm.prank(JUDGE);
        inst.setPayout(JUDGE, JUDGE_PAYOUT);
        vm.prank(JUDGE);
        inst.sweep(JUDGE, address(tok));
        assertEq(tok.balanceOf(JUDGE_PAYOUT), 1 ether);
        assertEq(tok.balanceOf(address(inst)), 0);
    }

    function test_foreignTokenIsTheBuyersStray() public {
        (Eravon inst,) = openAuto();
        // a second token at the instance
        address other = address(new MockOther());
        MockOther(other).mint(address(inst), 5 ether);
        vm.prank(SELLER);
        inst.sweep(address(0), other);
        assertEq(MockOther(other).balanceOf(BUYER_PAYOUT), 5 ether);
        // the case is untouched
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
        assertEq(tok.balanceOf(address(inst)), 111 ether);
    }

    /// venue/1 s3, standard s8: no payout at init, neither the order's nor the quote's; the judge's current key
    /// registers one, once, by setPayout or by the first withdraw that names one.
    function test_setPayoutAndWithdraw() public {
        Q memory q = qAuto;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = openNamed(t, quote);
        (address p0,) = inst.accountOf(JUDGE, address(tok));
        assertEq(p0, address(0), "nothing registered at init");
        claim(inst, MANIFEST);
        vm.prank(BUYER);
        inst.confirm();
        assertEq(judgeClaimable(inst), 1 ether);
        vm.prank(JUDGE);
        vm.expectRevert(Eravon.NoPayout.selector);
        inst.withdraw(address(tok), JUDGE, address(0));
        vm.prank(BUYER);
        inst.sweep(JUDGE, address(tok));
        assertEq(judgeClaimable(inst), 1 ether, "sweep leaves a seat without a payout alone");
        // only the judge's current key registers, nonzero, once
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotCurrent.selector);
        inst.setPayout(JUDGE, address(0x7A1));
        vm.prank(JUDGE);
        vm.expectRevert(Eravon.ZeroPayout.selector);
        inst.setPayout(JUDGE, address(0));
        vm.prank(JUDGE);
        inst.setPayout(JUDGE, address(0x7A1));
        vm.prank(JUDGE);
        vm.expectRevert(Eravon.PayoutSet.selector);
        inst.setPayout(JUDGE, address(0x7A2));
        // withdraw names no other payout
        vm.prank(JUDGE);
        vm.expectRevert(Eravon.PayoutSet.selector);
        inst.withdraw(address(tok), JUDGE, address(0x7A2));
        vm.prank(JUDGE);
        inst.withdraw(address(tok), JUDGE, address(0x7A1));
        assertEq(tok.balanceOf(address(0x7A1)), 1 ether);
        // withdraw with nothing claimable is a no-op
        vm.prank(JUDGE);
        inst.withdraw(address(tok), JUDGE, address(0));
        // on the logic both refuse
        vm.prank(JUDGE);
        vm.expectRevert(Eravon.NotInstance.selector);
        logic.setPayout(JUDGE, address(0x7A1));
        vm.prank(JUDGE);
        vm.expectRevert(Eravon.NotInstance.selector);
        logic.withdraw(address(tok), JUDGE, address(0x7A1));
        // a second case: the first withdraw that names a payout registers it, from the judge's current key alone
        (Eravon inst2, Eravon.OrderTerms memory t2) = openAuto();
        claim(inst2, MANIFEST);
        vm.prank(BUYER);
        inst2.confirm();
        address moved = address(0x1000000000000000000000000000000000000077);
        moveKey(JUDGE, moved);
        vm.prank(JUDGE);
        vm.expectRevert(Eravon.NotCurrent.selector);
        inst2.withdraw(address(tok), JUDGE, address(0x7A3));
        vm.prank(moved);
        inst2.withdraw(address(tok), JUDGE, address(0x7A3));
        assertEq(tok.balanceOf(address(0x7A3)), 1 ether, "the seat's ledger is the recorded address's, the current key draws it");
        (address p2,) = inst2.accountOf(JUDGE, address(tok));
        assertEq(p2, address(0x7A3));
        assertEq(inst2.seatOf(orderHash(t2), 0).judge, JUDGE, "the seat keeps the recorded address");
    }

    // ---- seatOf ----

    function test_seatOf() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        bytes32 id = orderHash(t);
        IGaleedVenue.Seat memory s = seat(inst, id, 0);
        assertEq(s.judge, JUDGE);
        assertEq(s.amanahHash, AMANAH);
        assertEq(s.termsHash, t.offer.termsHash);
        assertEq(s.quoteHash, keccak256(quoteAuto));
        assertEq(s.rungs, 3);
        assertEq(s.rungMin, 0);
        assertEq(s.rungMax, 2);
        assertEq(s.token, address(tok));
        assertEq(s.amount, A);
        assertEq(s.parties.length, 5);
        assertEq(s.parties[0], BUYER);
        assertEq(s.parties[1], SELLER);
        assertEq(s.parties[2], SELLER);
        assertEq(s.parties[3], BUYER_PAYOUT);
        assertEq(s.parties[4], SELLER_PAYOUT);
        vm.expectRevert(Eravon.UnknownSeat.selector);
        inst.seatOf(id, 1);
        vm.expectRevert(Eravon.UnknownSeat.selector);
        inst.seatOf(keccak256("x"), 0);
        // an uninitialised instance has no seat
        Eravon.OrderTerms memory t2 = namedTerms(JUDGE, AMANAH, quoteAuto, A);
        Eravon inst2 = deployProxy(t2);
        vm.expectRevert(Eravon.UnknownSeat.selector);
        inst2.seatOf(orderHash(t2), 0);
        assertEq(inst.quoteType(), QUOTE_TYPE_EXPECTED);
        assertEq(inst.extraType(), "");
    }

    string internal constant QUOTE_TYPE_EXPECTED =
        "(address token,uint16 premiumBps,address feeUnit,address feedToken,address feedUnit,uint32 maxStale,uint256 feeFixed,uint16 feeBps,uint256 feeMin,uint256 feeMax,uint256 amountCap,uint32 rulingWindow,uint32 offerWindow,uint8 lapseDefault,bool autoAccept)";
}

import {MockERC20} from "./replay/MockERC20.sol";

contract MockOther is MockERC20 {}
