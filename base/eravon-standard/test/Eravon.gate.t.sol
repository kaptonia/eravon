// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {Forwarder} from "./replay/VenueReplay.sol";
import {HostileToken, Company, Impostor} from "./mocks/Mocks.sol";

/// Standard s4, the gate: init's order of decision, the expiry branch, the
/// caller rule, every guard, the two consents, the deposit; cancel; the
/// logic's three registries (consent, revocation, pull).
contract EravonGate is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE_PAYOUT = address(0x4000000000000000000000000000000000000001);
    address internal constant STRANGER = address(0x3000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah");
    Q internal qAuto;
    bytes internal quoteAuto;

    function setUp() public {
        setUpBase();
        qAuto = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        quoteAuto = encodeQuote(qAuto);
    }

    function terms(uint256 amount_) internal returns (Eravon.OrderTerms memory t) {
        t = namedTerms(JUDGE, AMANAH, quoteAuto, amount_);
    }

    function prepared(Eravon.OrderTerms memory t) internal returns (Eravon inst) {
        inst = deployProxy(t);
        tok.mint(address(inst), t.offer.amount);
        fundDeposit(qAuto, t.offer.amount);
    }

    function initAs(address who, Eravon inst, Eravon.OrderTerms memory t) internal {
        vm.prank(who);
        inst.init(t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
    }

    // ---- order of decision ----

    function test_init_opensAndPullsTheDeposit() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon inst = prepared(t);
        uint256 d = depositOf(qAuto, 100 ether);
        assertEq(d, 11 ether, "D = P + lf");
        vm.expectEmit(true, true, true, true, address(logic));
        emit Eravon.Pulled(SELLER, SELLER, address(tok), address(inst), d);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Opened(orderHash(t), SELLER, BUYER, JUDGE, 100 ether, t.offer.termsHash);
        initAs(SELLER, inst, t);
        assertEq(tok.balanceOf(address(inst)), 111 ether, "A + D held");
        assertEq(tok.balanceOf(SELLER), 0, "the deposit left the seller's key");
        assertEq(depositHeld(inst), d);
        assertEq(premiumHeld(inst), 1 ether);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
    }

    function test_init_alreadyInitialized() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon inst = prepared(t);
        initAs(SELLER, inst, t);
        fundDeposit(qAuto, 100 ether);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.AlreadyInitialized.selector);
        inst.init(t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
    }

    function test_init_wrongAddress() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon.OrderTerms memory other = terms(100 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongAddress.selector);
        inst.init(other, quoteAuto, buyerSig(other), sellerSig(other), new address[](0), new bytes32[](0));
        // an order differing in openBy alone is another instance
        Eravon.OrderTerms memory t2 = t;
        t2.openBy = t.openBy + 1;
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongAddress.selector);
        inst.init(t2, quoteAuto, buyerSig(t2), sellerSig(t2), new address[](0), new bytes32[](0));
    }

    function test_init_wrongAddressBeforeTheExpiryBranch() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon.OrderTerms memory other = terms(100 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        vm.warp(t.offer.expiry + 1);
        vm.expectRevert(Eravon.WrongAddress.selector);
        inst.init(other, "", "", "", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(address(inst)), 100 ether, "nothing moved on a foreign order");
    }

    // ---- the expiry branch ----

    function test_expiryBranch_expiry() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 70 ether);
        vm.warp(t.offer.expiry + 1);
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Refunded(2);
        vm.prank(BUYER);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 70 ether, "the whole balance to the buyer");
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.None), "no case");
        // a later deposit leaves by the same branch again
        tok.mint(address(inst), 5 ether);
        vm.prank(SELLER);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 75 ether);
    }

    function test_expiryBranch_openBy() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        vm.warp(uint256(t.openBy) + 1);
        assertTrue(vm.getBlockTimestamp() < t.offer.expiry, "the offer still stands; the order's clock ran out");
        vm.prank(SELLER);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 100 ether);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.None));
        // at openBy itself the case still opens
        Eravon.OrderTerms memory t2 = terms(100 ether);
        Eravon inst2 = prepared(t2);
        vm.warp(uint256(t2.openBy));
        initAs(SELLER, inst2, t2);
        assertEq(uint8(phaseOf(inst2)), uint8(Eravon.Phase.Funded));
    }

    function test_expiryBranch_revoked() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon inst = prepared(t);
        vm.prank(SELLER);
        logic.revoke(SELLER, t.offer.termsHash);
        assertTrue(logic.revoked(SELLER, t.offer.termsHash));
        vm.prank(BUYER);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 100 ether, "the buyer's money leaves at once");
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.None));
        // the deposit was never pulled
        assertEq(tok.balanceOf(SELLER), depositOf(qAuto, 100 ether));
        // another seller's revocation of a termsHash means nothing to this seller's offers
        Eravon.OrderTerms memory t2 = terms(100 ether);
        t2.offer.termsHash = keccak256("another document");
        Eravon inst2 = prepared(t2);
        vm.prank(STRANGER);
        logic.revoke(STRANGER, t2.offer.termsHash);
        initAs(SELLER, inst2, t2);
        assertEq(uint8(phaseOf(inst2)), uint8(Eravon.Phase.Funded));
    }

    function test_expiryBranch_readsNoSignatureNoQuoteNoFeed() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        t.offer.termsHash = bytes32(0); // guard 13 would fail
        t.offer.quoteHash = bytes32(0); // guard 15 would fail
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 7 ether);
        vm.warp(t.offer.expiry + 1);
        vm.prank(BUYER);
        inst.init(t, hex"00", hex"01", hex"02", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 7 ether);
    }

    function test_expiryBranch_beforeTheCallerRule() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        t.offer.buyerMayOpen = false;
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        vm.warp(uint256(t.openBy) + 1);
        vm.prank(BUYER);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 100 ether, "the buyer gets its money back without the seller");
    }

    function test_revokeAfterInitTouchesNoOpenCase() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon inst = prepared(t);
        initAs(SELLER, inst, t);
        vm.prank(SELLER);
        logic.revoke(SELLER, t.offer.termsHash);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
        claim(inst, keccak256("m"));
        vm.prank(BUYER);
        inst.confirm();
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released));
    }

    // ---- the caller rule ----

    function test_callerRule_buyerMayOpenFalse() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        t.offer.buyerMayOpen = false;
        Eravon inst = prepared(t);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.NotSeller.selector);
        inst.init(t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotSeller.selector);
        inst.init(t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        // a forwarder's address is not the seller
        Forwarder f = new Forwarder();
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotSeller.selector);
        f.forward(address(inst), abi.encodeCall(inst.init, (t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0))));
        initAs(SELLER, inst, t);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
    }

    function test_callerRule_buyerMayOpenTrue() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon inst = prepared(t);
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.init(t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        // a forwarder's address is no party
        Forwarder f = new Forwarder();
        vm.prank(BUYER);
        vm.expectRevert(Eravon.NotParty.selector);
        f.forward(address(inst), abi.encodeCall(inst.init, (t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0))));
        initAs(BUYER, inst, t);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded), "the buyer opens with both signatures");
    }

    function test_callerRule_beforeTheGuards() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        t.offer.buyerMayOpen = false;
        t.offer.amount = 0;
        Eravon inst = deployProxy(t);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.NotSeller.selector);
        inst.init(t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        vm.prank(SELLER);
        vm.expectRevert(abi.encodeWithSelector(Eravon.BadTerms.selector, 6));
        inst.init(t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
    }

    // ---- the guards ----

    function _guard(Eravon.OrderTerms memory t, uint8 n) internal {
        Eravon inst = deployProxy(t);
        vm.prank(SELLER);
        vm.expectRevert(abi.encodeWithSelector(Eravon.BadTerms.selector, n));
        inst.init(t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
    }

    function test_guards_eachRefused() public {
        Eravon.OrderTerms memory t;
        t = terms(1 ether); t.buyerKey = SELLER; _guard(t, 1);
        t = terms(1 ether); t.offer.guild = address(0xBEEF); _guard(t, 2);
        t = terms(1 ether); t.offer.judge = address(0); _guard(t, 2);
        t = terms(1 ether); t.offer.judge = BUYER; _guard(t, 3);
        t = terms(1 ether); t.offer.judge = SELLER; _guard(t, 3);
        t = terms(1 ether); t.buyerPayout = address(0); _guard(t, 4);
        t = terms(1 ether); t.offer.sellerPayout = address(0); _guard(t, 4);
        t = terms(1 ether); t.offer.sellerFund = address(0); _guard(t, 4);
        t = terms(1 ether); t.offer.token = address(0); _guard(t, 5);
        t = terms(1 ether); t.offer.amount = 0; _guard(t, 6);
        t = terms(1 ether); t.offer.ladder = new Eravon.Rung[](1); t.offer.ladder[0] = Eravon.Rung(10000, 0, 0); _guard(t, 7);
        t = terms(1 ether); t.offer.ladder[1] = Eravon.Rung(5000, 4000, 0); _guard(t, 8);
        t = terms(1 ether); t.offer.ladder[1] = Eravon.Rung(5000, 5000, 3); _guard(t, 9);
        t = terms(1 ether); t.offer.ladder = new Eravon.Rung[](4); t.offer.ladder[0] = Eravon.Rung(0, 10000, 0); t.offer.ladder[1] = Eravon.Rung(6000, 4000, 0); t.offer.ladder[2] = Eravon.Rung(3000, 7000, 0); t.offer.ladder[3] = Eravon.Rung(10000, 0, 0); _guard(t, 10);
        t = terms(1 ether); t.offer.ladder[0] = Eravon.Rung(1, 9999, 0); t.offer.ladder[1] = Eravon.Rung(0, 10000, 0); _guard(t, 11);
        t = terms(1 ether); t.offer.ladder[1] = Eravon.Rung(10000, 0, 0); t.offer.ladder[2] = Eravon.Rung(9999, 1, 0); _guard(t, 12);
        t = terms(1 ether); t.offer.judge = address(0); t.offer.guild = SELLER; _guard(t, 3);
        t = terms(1 ether); t.offer.judge = address(0); t.offer.guild = BUYER; _guard(t, 3);
        t = terms(1 ether); t.offer.amount = type(uint256).max / 10000 + 1; _guard(t, 6);
        t = terms(1 ether); t.offer.nonce = 2 ** 53; _guard(t, 16);
        t = terms(1 ether); t.offer.expiry = 2 ** 53; _guard(t, 16);
        t = terms(1 ether); t.openBy = uint64(2 ** 53); _guard(t, 16);
        t = terms(1 ether); t.offer.ladder[0] = Eravon.Rung(1, 9999, 0); t.offer.ladder[1] = Eravon.Rung(5000, 5000, 0); _guard(t, 11);
        t = terms(1 ether); t.offer.ladder[2] = Eravon.Rung(9999, 1, 0); _guard(t, 12);
        t = terms(1 ether); t.offer.termsHash = bytes32(0); _guard(t, 13);
        t = terms(1 ether); t.offer.amanahHash = bytes32(0); _guard(t, 14);
        t = terms(1 ether); t.offer.quoteHash = bytes32(0); _guard(t, 15);
        // 20: a panel handed to a named-track init
        t = terms(1 ether);
        Eravon inst = prepared(t);
        address[] memory m = new address[](1); m[0] = JUDGE;
        bytes32[] memory w = new bytes32[](1); w[0] = AMANAH;
        vm.prank(SELLER);
        vm.expectRevert(abi.encodeWithSelector(Eravon.BadTerms.selector, 20));
        inst.init(t, quoteAuto, buyerSig(t), sellerSig(t), m, w);
    }

    function test_quote_refusals() public {
        Eravon.OrderTerms memory t = terms(1 ether);
        Eravon inst = prepared(t);
        bytes memory short = new bytes(479);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadQuote.selector);
        inst.init(t, short, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        bytes memory long_ = bytes.concat(quoteAuto, hex"00");
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadQuote.selector);
        inst.init(t, long_, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        Q memory q1 = qAuto; q1.lapseDefault = 3;
        bytes memory lapse1 = encodeQuote(q1);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadQuote.selector);
        inst.init(t, lapse1, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        Q memory q2 = qAuto; q2.feeBps = 10001;
        bytes memory bps = encodeQuote(q2);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadQuote.selector);
        inst.init(t, bps, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        // a decoding quote that is not the order's
        Q memory q3 = qAuto; q3.premiumBps = 200;
        bytes memory other = encodeQuote(q3);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.QuoteMismatch.selector);
        inst.init(t, other, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        // the quote's token is not the order's: the order names the quote (hash) but the token differs
        Q memory q4 = qAuto; q4.token = address(0xBEEF);
        bytes memory otherTok = encodeQuote(q4);
        Eravon.OrderTerms memory t4 = namedTerms(JUDGE, AMANAH, otherTok, 1 ether);
        Eravon inst4 = deployProxy(t4);
        tok.mint(address(inst4), 1 ether);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.QuoteMismatch.selector);
        inst4.init(t4, otherTok, buyerSig(t4), sellerSig(t4), new address[](0), new bytes32[](0));
    }

    // ---- the consents ----

    function test_consents_signatures() public {
        Eravon.OrderTerms memory t = terms(1 ether);
        Eravon inst = prepared(t);
        bytes memory wrongS = sign(BUYER_PK, digestOf(address(logic), offerHash(t)));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quoteAuto, buyerSig(t), wrongS, new address[](0), new bytes32[](0));
        bytes memory wrongB = sign(SELLER_PK, digestOf(instanceOf(t), orderHash(t)));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.init(t, quoteAuto, wrongB, sellerSig(t), new address[](0), new bytes32[](0));
        // the seller's signature under another logic's domain is no signature here
        bytes memory otherDomain = sign(SELLER_PK, digestOf(address(0xD0D0), offerHash(t)));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quoteAuto, buyerSig(t), otherDomain, new address[](0), new bytes32[](0));
        // the seller's consent by its own call, the buyer's by signature
        vm.prank(SELLER);
        inst.init(t, quoteAuto, buyerSig(t), "", new address[](0), new bytes32[](0));
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
    }

    function test_consents_byCall() public {
        // the buyer opens with its own call, the seller's slot empty too: refused without the registry
        Eravon.OrderTerms memory t = terms(1 ether);
        Eravon inst = prepared(t);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quoteAuto, "", "", new address[](0), new bytes32[](0));
        // the seller registers the offer digest on the logic; the buyer's own call opens
        vm.prank(SELLER);
        logic.consent(SELLER, digestOf(address(logic), offerHash(t)));
        vm.prank(BUYER);
        inst.init(t, quoteAuto, "", "", new address[](0), new bytes32[](0));
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
        // the seller with both slots empty and only the seller registered: the buyer's slot fails
        Eravon.OrderTerms memory t2 = terms(1 ether);
        Eravon inst2 = prepared(t2);
        vm.prank(SELLER);
        logic.consent(SELLER, digestOf(address(logic), offerHash(t2)));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst2.init(t2, quoteAuto, "", "", new address[](0), new bytes32[](0));
        vm.prank(BUYER);
        logic.consent(BUYER, digestOf(instanceOf(t2), orderHash(t2)));
        vm.prank(SELLER);
        inst2.init(t2, quoteAuto, "", "", new address[](0), new bytes32[](0));
        assertEq(uint8(phaseOf(inst2)), uint8(Eravon.Phase.Funded), "both consents by registry, the seller's call opens");
    }

    function test_consents_companyBuyer() public {
        Company co = new Company();
        Eravon.OrderTerms memory t = terms(1 ether);
        t.buyerKey = address(co);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 1 ether);
        fundDeposit(qAuto, 1 ether);
        // the company opens by its own call; the seller signed
        co.exec(address(inst), abi.encodeCall(inst.init, (t, quoteAuto, "", sellerSig(t), new address[](0), new bytes32[](0))));
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
        // a registry entry through an instance is refused
        vm.expectRevert(Eravon.NotRegistry.selector);
        inst.consent(SELLER, keccak256("x"));
        assertFalse(inst.consented(SELLER, keccak256("x")));
    }

    function test_consents_zeroKeyConsentsToNothing() public {
        Eravon.OrderTerms memory t = terms(1 ether);
        t.buyerKey = address(0);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 1 ether);
        vm.prank(SELLER);
        vm.expectRevert(abi.encodeWithSelector(Eravon.BadTerms.selector, 1));
        inst.init(t, quoteAuto, "", "", new address[](0), new bytes32[](0));
    }

    // ---- balance, pricing, deposit ----

    function test_underfundedAndSurplus() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether - 1);
        fundDeposit(qAuto, 100 ether);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.Underfunded.selector);
        inst.init(t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        tok.mint(address(inst), 1 + 3 ether);
        initAs(SELLER, inst, t);
        assertEq(inst.owed(BUYER_PAYOUT, address(tok)), 3 ether, "the surplus credited to the buyer's payout at init");
        vm.prank(SELLER);
        inst.sweep(BUYER_PAYOUT, address(tok));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 3 ether);
        assertEq(tok.balanceOf(address(inst)), 111 ether);
    }

    function test_depositShort_allowance() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        tok.mint(SELLER, 11 ether);
        vm.prank(SELLER);
        tok.approve(address(logic), 11 ether - 1);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.TransferFailed.selector);
        inst.init(t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.None), "nothing landed");
        assertEq(tok.balanceOf(address(inst)), 100 ether, "the buyer's money stays, refundable by the clock");
    }

    function test_depositShort_balance() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        tok.mint(SELLER, 11 ether - 1);
        vm.prank(SELLER);
        tok.approve(address(logic), type(uint256).max);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.TransferFailed.selector);
        inst.init(t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
    }

    function test_depositShort_feeOnTransferToken() public {
        HostileToken hostile = new HostileToken();
        hostile.setMode(HostileToken.Mode.FeeOnTransfer);
        Q memory q = defaultQuote(address(hostile), JUDGE_PAYOUT, true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 100 ether);
        t.offer.token = address(hostile);
        Eravon inst = deployProxy(t);
        hostile.mint(address(inst), 100 ether);
        hostile.mint(SELLER, 11 ether);
        vm.prank(SELLER);
        hostile.approve(address(logic), 11 ether);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.DepositShort.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
    }

    // ---- cancel ----

    function test_cancel_byTheBuyer() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Refunded(4);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Closed(0, 100 ether, 0, 0, 0);
        vm.prank(BUYER);
        inst.cancel(t, "");
        assertEq(tok.balanceOf(BUYER_PAYOUT), 100 ether);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded));
        // a later stray leaves by sweep to the buyer's payout
        tok.mint(address(inst), 2 ether);
        vm.prank(SELLER);
        inst.sweep(BUYER_PAYOUT, address(tok));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 102 ether);
    }

    function test_cancel_bySignatureFromAParty() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.cancel(t, cancelSig(t));
        vm.prank(SELLER);
        inst.cancel(t, cancelSig(t));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 100 ether);
    }

    function test_cancel_refusals() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotBuyer.selector);
        inst.cancel(t, "");
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotBuyer.selector);
        inst.cancel(t, "");
        // the signed form from no party's key, decided before the signature
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.cancel(t, cancelSig(t));
        bytes memory wrong = sign(SELLER_PK, digestOf(instanceOf(t), keccak256(abi.encode(TH_CANCEL))));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.cancel(t, wrong);
        // a Cancel signed for another instance
        Eravon.OrderTerms memory t2 = terms(100 ether);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.cancel(t, cancelSig(t2));
        // a Confirm signature is no Cancel
        bytes memory confirmSig = sign(BUYER_PK, digestOf(instanceOf(t), keccak256(abi.encode(TH_CONFIRM))));
        vm.prank(BUYER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.cancel(t, confirmSig);
        // wrong instance
        vm.prank(BUYER);
        vm.expectRevert(Eravon.WrongAddress.selector);
        inst.cancel(t2, "");
        assertEq(tok.balanceOf(address(inst)), 100 ether, "nothing moved");
        // a cancel on an empty instance costs nothing and closes it
        Eravon inst2 = deployProxy(t2);
        vm.prank(BUYER);
        inst2.cancel(t2, "");
        assertEq(uint8(phaseOf(inst2)), uint8(Eravon.Phase.Refunded));
    }

    function test_cancel_pastExpiryStillRefunds() public {
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        vm.warp(t.offer.expiry + 1);
        vm.prank(BUYER);
        inst.cancel(t, "");
        assertEq(tok.balanceOf(BUYER_PAYOUT), 100 ether);
    }

    function test_cancel_byACompanyBuyer() public {
        Company co = new Company();
        Eravon.OrderTerms memory t = terms(100 ether);
        t.buyerKey = address(co);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        co.exec(address(inst), abi.encodeCall(inst.cancel, (t, "")));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 100 ether);
    }

    // ---- the logic's registries and the deposit face ----

    function test_pull_onlyInstanceToItself() public {
        tok.mint(SELLER, 50 ether);
        vm.prank(SELLER);
        tok.approve(address(logic), 50 ether);
        // an account that is no proxy of the logic
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotInstance.selector);
        logic.pull(SELLER, SELLER, address(tok), 1 ether);
        Impostor imp = new Impostor();
        vm.expectRevert(Eravon.NotInstance.selector);
        imp.tryPull(logic, SELLER, address(tok), 1 ether);
        // a proxy of another logic has another code hash
        Eravon logic2 = Eravon(deployThrough(keccak256("eravon/other"), type(Eravon).creationCode));
        bytes memory otherInit = abi.encodePacked(hex"3d602d80600a3d3981f3363d3d373d3d3d363d73", address(logic2), hex"5af43d82803e903d91602b57fd5bf3");
        address otherProxy = deployThrough(keccak256("x"), otherInit);
        vm.prank(otherProxy);
        vm.expectRevert(Eravon.NotInstance.selector);
        logic.pull(SELLER, SELLER, address(tok), 1 ether);
        // through an instance the function is the registry's alone
        Eravon.OrderTerms memory t = terms(1 ether);
        Eravon inst = deployProxy(t);
        vm.expectRevert(Eravon.NotRegistry.selector);
        inst.pull(SELLER, SELLER, address(tok), 1 ether);
        vm.expectRevert(Eravon.NotRegistry.selector);
        inst.revoke(SELLER, keccak256("x"));
        assertFalse(inst.revoked(SELLER, keccak256("x")));
        assertEq(tok.balanceOf(SELLER), 50 ether, "nothing left the seller");
        // a genuine proxy pulls to itself alone: Pulled names the caller as the receiver
        vm.prank(address(inst));
        vm.expectEmit(true, true, true, true, address(logic));
        emit Eravon.Pulled(SELLER, SELLER, address(tok), address(inst), 3 ether);
        logic.pull(SELLER, SELLER, address(tok), 3 ether);
        assertEq(tok.balanceOf(address(inst)), 3 ether);
    }

    function test_revoke_registry() public {
        vm.prank(SELLER);
        vm.expectEmit(true, true, true, true, address(logic));
        emit Eravon.Revoked(SELLER, keccak256("doc"));
        logic.revoke(SELLER, keccak256("doc"));
        assertTrue(logic.revoked(SELLER, keccak256("doc")));
        assertFalse(logic.revoked(STRANGER, keccak256("doc")));
        assertFalse(logic.revoked(SELLER, keccak256("other")));
    }

    function test_logicHoldsNoMoney() public {
        // nothing of the logic receives tokens: a direct transfer to it is a stray nobody can pull back through it
        Eravon.OrderTerms memory t = terms(100 ether);
        Eravon inst = prepared(t);
        initAs(SELLER, inst, t);
        assertEq(tok.balanceOf(address(logic)), 0, "the deposit went to the instance, never through the logic");
    }
}
