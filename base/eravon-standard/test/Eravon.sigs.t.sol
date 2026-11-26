// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {Forwarder} from "./replay/VenueReplay.sol";

/// Standard s9 under attack: every signature replayed across instances and
/// across actions, each message bound to one party, malleated s and a v
/// outside 27 and 28, a buyer signature of another length, and the offer
/// signature's reuse by design.
contract EravonSigs is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE_PAYOUT = address(0x4000000000000000000000000000000000000001);
    address internal constant STRANGER = address(0x3000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah");
    bytes32 internal constant MANIFEST = keccak256("manifest");
    uint256 internal constant SECP_N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141;
    uint256 internal constant A = 100 ether;
    Q internal qAuto;
    bytes internal quoteAuto;

    function setUp() public {
        setUpBase();
        qAuto = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        quoteAuto = encodeQuote(qAuto);
    }

    function openAuto() internal returns (Eravon inst, Eravon.OrderTerms memory t) {
        t = namedTerms(JUDGE, AMANAH, quoteAuto, A);
        fundDeposit(qAuto, A);
        inst = openNamed(t, quoteAuto);
    }

    function actionSig(uint256 pk, Eravon inst, bytes32 structHash) internal view returns (bytes memory) {
        return sign(pk, digestOf(address(inst), structHash));
    }

    function confirmHash() internal pure returns (bytes32) {
        return keccak256(abi.encode(TH_CONFIRM));
    }

    function disputeHash() internal pure returns (bytes32) {
        return keccak256(abi.encode(TH_DISPUTE));
    }

    function refundHash() internal pure returns (bytes32) {
        return keccak256(abi.encode(TH_REFUND));
    }

    function claimHash(bytes32 m) internal pure returns (bytes32) {
        return keccak256(abi.encode(TH_CLAIM, m));
    }

    function settleHash(uint16 bps, uint256 exp) internal pure returns (bytes32) {
        return keccak256(abi.encode(TH_SETTLE, uint256(bps), exp));
    }

    // ---- replay across instances ----

    function test_orderSignatureBoundToOneInstance() public {
        Eravon.OrderTerms memory t1 = namedTerms(JUDGE, AMANAH, quoteAuto, A);
        Eravon.OrderTerms memory t2 = namedTerms(JUDGE, AMANAH, quoteAuto, A);
        fundDeposit(qAuto, A);
        Eravon i2 = deployProxy(t2);
        tok.mint(address(i2), A);
        // the buyer's signature over the first order presented at the second
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        i2.init(t2, quoteAuto, buyerSig(t1), sellerSig(t2), new address[](0), new bytes32[](0));
        // the buyer's digest of the first instance signed under the second's address is another digest
        bytes memory wrongDomain = sign(BUYER_PK, digestOf(instanceOf(t1), orderHash(t2)));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        i2.init(t2, quoteAuto, wrongDomain, sellerSig(t2), new address[](0), new bytes32[](0));
        // the registry's consent is bound the same way: the first order's digest opens nothing at the second
        vm.prank(BUYER);
        logic.consent(BUYER, digestOf(instanceOf(t1), orderHash(t1)));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        i2.init(t2, quoteAuto, "", sellerSig(t2), new address[](0), new bytes32[](0));
        vm.prank(SELLER);
        i2.init(t2, quoteAuto, buyerSig(t2), sellerSig(t2), new address[](0), new bytes32[](0));
        assertEq(uint8(phaseOf(i2)), uint8(Eravon.Phase.Funded));
    }

    function test_actionSignaturesBoundToOneInstance() public {
        (Eravon a,) = openAuto();
        (Eravon b,) = openAuto();
        claim(a, MANIFEST);
        claim(b, MANIFEST);
        bytes memory confirmA = actionSig(BUYER_PK, a, confirmHash());
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        b.confirmBySig(confirmA);
        bytes memory refundA = actionSig(SELLER_PK, a, refundHash());
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        b.refundBySig(refundA);
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes memory sB = actionSig(BUYER_PK, a, settleHash(5000, exp));
        bytes memory sS = actionSig(SELLER_PK, a, settleHash(5000, exp));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        b.settle(5000, exp, sB, sS);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        b.settle(5000, exp, sB, actionSig(SELLER_PK, b, settleHash(5000, exp)));
        // at their own instance they land
        vm.prank(SELLER);
        a.settle(5000, exp, sB, sS);
        vm.prank(SELLER);
        b.confirmBySig(actionSig(BUYER_PK, b, confirmHash()));
        assertEq(uint8(phaseOf(a)), uint8(Eravon.Phase.Settled));
        assertEq(uint8(phaseOf(b)), uint8(Eravon.Phase.Released));
    }

    // ---- replay across actions and parties ----

    function test_eachSignatureIsOneActionOfOneParty() public {
        (Eravon inst, Eravon.OrderTerms memory t) = openAuto();
        // the seller's claim signature is not a refund, nor the buyer's confirmation
        bytes memory c = actionSig(SELLER_PK, inst, claimHash(MANIFEST));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.refundBySig(c);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.claimDeliveryBySig(keccak256("other manifest"), c);
        vm.prank(SELLER);
        inst.claimDeliveryBySig(MANIFEST, c);
        // the buyer's confirmation is not a dispute, and not a seller's refund
        bytes memory conf = actionSig(BUYER_PK, inst, confirmHash());
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.disputeBySig(conf);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.refundBySig(conf);
        // the seller signing the buyer's message, and the buyer the seller's
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.confirmBySig(actionSig(SELLER_PK, inst, confirmHash()));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.refundBySig(actionSig(BUYER_PK, inst, refundHash()));
        // the order's signature is no action
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.confirmBySig(buyerSig(t));
        // a Cancel signature after init opens nothing and confirms nothing
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.confirmBySig(cancelSig(t));
        vm.prank(BUYER);
        vm.expectRevert(Eravon.AlreadyInitialized.selector);
        inst.cancel(t, cancelSig(t));
        // the settlement needs both keys and the same values
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes memory sB = actionSig(BUYER_PK, inst, settleHash(5000, exp));
        bytes memory sS = actionSig(SELLER_PK, inst, settleHash(5000, exp));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.settle(5000, exp, sB, sB);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.settle(5000, exp, sS, sS);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.settle(4999, exp, sB, sS);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.settle(4999, exp, sB, actionSig(SELLER_PK, inst, settleHash(4999, exp)));
        vm.prank(SELLER);
        inst.confirmBySig(conf);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released));
    }

    // ---- malleability ----

    function test_highSAndBadVRefused() public {
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quoteAuto, A);
        fundDeposit(qAuto, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        bytes32 d = digestOf(address(logic), offerHash(t));
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(SELLER_PK, d);
        bytes memory highS = abi.encodePacked(r, bytes32(SECP_N - uint256(s)), uint8(v == 27 ? 28 : 27));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quoteAuto, buyerSig(t), highS, new address[](0), new bytes32[](0));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quoteAuto, buyerSig(t), abi.encodePacked(r, s, uint8(v - 27)), new address[](0), new bytes32[](0));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quoteAuto, buyerSig(t), abi.encodePacked(r, s, uint8(29)), new address[](0), new bytes32[](0));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quoteAuto, buyerSig(t), abi.encodePacked(bytes32(0), s, v), new address[](0), new bytes32[](0));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quoteAuto, buyerSig(t), abi.encodePacked(r, bytes32(0), v), new address[](0), new bytes32[](0));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quoteAuto, buyerSig(t), abi.encodePacked(r, s), new address[](0), new bytes32[](0));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quoteAuto, buyerSig(t), abi.encodePacked(r, s, v, uint8(0)), new address[](0), new bytes32[](0));
        // the buyer's half on the order
        bytes32 db = digestOf(address(inst), orderHash(t));
        (uint8 vb, bytes32 rb, bytes32 sb) = vm.sign(BUYER_PK, db);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.init(t, quoteAuto, abi.encodePacked(rb, bytes32(SECP_N - uint256(sb)), uint8(vb == 27 ? 28 : 27)), sellerSig(t), new address[](0), new bytes32[](0));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.init(t, quoteAuto, abi.encodePacked(rb, sb, uint8(1)), sellerSig(t), new address[](0), new bytes32[](0));
        vm.prank(SELLER);
        inst.init(t, quoteAuto, abi.encodePacked(rb, sb, vb), abi.encodePacked(r, s, v), new address[](0), new bytes32[](0));
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
    }

    function test_buyerHighSOnAnActionAndOtherLengths() public {
        (Eravon inst,) = openAuto();
        claim(inst, MANIFEST);
        bytes32 d = digestOf(address(inst), confirmHash());
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(BUYER_PK, d);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.confirmBySig(abi.encodePacked(r, bytes32(SECP_N - uint256(s)), uint8(v == 27 ? 28 : 27)));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.confirmBySig(abi.encodePacked(r, s, uint8(1)));
        // a length other than 65 is no signature of the one form (s9)
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.confirmBySig(abi.encodePacked(r, s));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.confirmBySig(abi.encodePacked(r, s, v, uint8(0)));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.confirmBySig("");
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Claimed), "nothing moved");
        vm.prank(SELLER);
        inst.confirmBySig(abi.encodePacked(r, s, v));
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released));
    }

    // ---- the offer signature, reusable by design ----

    function test_oneOfferTwoBuyersTwoInstances() public {
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quoteAuto, A);
        bytes memory sigS = sellerSig(t);
        uint256 pk2 = 0xB0B2;
        Eravon.OrderTerms memory second = namedTerms(JUDGE, AMANAH, quoteAuto, A);
        second.offer = t.offer;
        second.buyerKey = vm.addr(pk2);
        second.buyerPayout = address(0xB0B72);
        assertTrue(instanceOf(t) != instanceOf(second), "two orders, two addresses");
        assertEq(offerHash(t), offerHash(second), "one offer behind both");
        fundDeposit(qAuto, A);
        fundDeposit(qAuto, A);
        Eravon first = deployProxy(t);
        tok.mint(address(first), A);
        vm.prank(SELLER);
        first.init(t, quoteAuto, buyerSig(t), sigS, new address[](0), new bytes32[](0));
        Eravon other = deployProxy(second);
        tok.mint(address(other), A);
        vm.prank(SELLER);
        other.init(second, quoteAuto, sign(pk2, digestOf(address(other), orderHash(second))), sigS, new address[](0), new bytes32[](0));
        assertEq(uint8(phaseOf(first)), uint8(Eravon.Phase.Funded));
        assertEq(uint8(phaseOf(other)), uint8(Eravon.Phase.Funded));
        // one buyer's key over another's order is refused
        Eravon.OrderTerms memory third = namedTerms(JUDGE, AMANAH, quoteAuto, A);
        third.offer = t.offer;
        third.buyerKey = vm.addr(pk2);
        third.buyerPayout = address(0xB0B73);
        Eravon i3 = deployProxy(third);
        tok.mint(address(i3), A);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        i3.init(third, quoteAuto, sign(BUYER_PK, digestOf(address(i3), orderHash(third))), sigS, new address[](0), new bytes32[](0));
        // the seller's registry consent to the offer serves every order under it, like the signature
        Eravon.OrderTerms memory fourth = namedTerms(JUDGE, AMANAH, quoteAuto, A);
        Eravon.OrderTerms memory fifth = namedTerms(JUDGE, AMANAH, quoteAuto, A);
        fifth.offer = fourth.offer;
        fifth.buyerKey = vm.addr(pk2);
        vm.prank(SELLER);
        logic.consent(SELLER, digestOf(address(logic), offerHash(fourth)));
        fundDeposit(qAuto, A);
        fundDeposit(qAuto, A);
        Eravon i4 = deployProxy(fourth);
        tok.mint(address(i4), A);
        vm.prank(BUYER);
        i4.init(fourth, quoteAuto, "", "", new address[](0), new bytes32[](0));
        Eravon i5 = deployProxy(fifth);
        tok.mint(address(i5), A);
        vm.prank(vm.addr(pk2));
        i5.init(fifth, quoteAuto, "", "", new address[](0), new bytes32[](0));
        assertEq(uint8(phaseOf(i5)), uint8(Eravon.Phase.Funded));
        // and not another offer
        Eravon.OrderTerms memory sixth = namedTerms(JUDGE, AMANAH, quoteAuto, A);
        Eravon i6 = deployProxy(sixth);
        tok.mint(address(i6), A);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        i6.init(sixth, quoteAuto, "", "", new address[](0), new bytes32[](0));
    }

    // ---- front-running ----

    function test_frontRunningTheDeploymentAndTheInitGainsNothing() public {
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quoteAuto, A);
        fundDeposit(qAuto, A);
        // a stranger deploys the proxy; it cannot send the parties' signatures (no party's key), and the case a
        // party opens is the one they signed
        vm.prank(STRANGER);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.init(t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        vm.prank(BUYER);
        inst.init(t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        assertEq(inst.seatOf(orderHash(t), 0).judge, JUDGE);
        assertEq(inst.seatOf(orderHash(t), 0).parties[1], SELLER);
        // altered terms at the same address are refused
        Eravon.OrderTerms memory t2 = namedTerms(JUDGE, AMANAH, quoteAuto, A);
        Eravon inst2 = deployProxy(t2);
        tok.mint(address(inst2), A);
        Eravon.OrderTerms memory altered = t2;
        altered.offer.sellerPayout = STRANGER;
        vm.prank(BUYER);
        vm.expectRevert(Eravon.WrongAddress.selector);
        inst2.init(altered, quoteAuto, buyerSig(altered), sign(SELLER_PK, digestOf(address(logic), offerHash(altered))), new address[](0), new bytes32[](0));
        // a relay is not the seller when the seller alone may open
        Eravon.OrderTerms memory t3 = namedTerms(JUDGE, AMANAH, quoteAuto, A);
        t3.offer.buyerMayOpen = false;
        Eravon inst3 = deployProxy(t3);
        tok.mint(address(inst3), A);
        Forwarder f = new Forwarder();
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotSeller.selector);
        f.forward(address(inst3), abi.encodeCall(inst3.init, (t3, quoteAuto, buyerSig(t3), sellerSig(t3), new address[](0), new bytes32[](0))));
    }
}
