// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {EravonVerifier} from "../src/EravonVerifier.sol";
import {EravonBoard} from "../src/EravonBoard.sol";

/// Standard s1 and s13: the verifier designated by code at the derived
/// address, every quote-decoding refusal, the consent views; and s7b, the board.
contract EravonVerifierAndBoard is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE_PAYOUT = address(0x4000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah");
    Q internal qAuto;
    bytes internal quoteAuto;

    function setUp() public {
        setUpBase();
        qAuto = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        quoteAuto = encodeQuote(qAuto);
    }

    function test_verifier_atTheDerivedAddressWithThePinnedCode() public {
        assertEq(address(verifier), create2Address(keccak256("eravon/1/verify"), keccak256(type(EravonVerifier).creationCode)));
        assertEq(address(verifier).codehash, keccak256(vm.getDeployedCode("EravonVerifier.sol:EravonVerifier")));
        assertEq(address(board).codehash, keccak256(vm.getDeployedCode("EravonBoard.sol:EravonBoard")));
        assertEq(address(board), create2Address(keccak256("eravon/1/board"), keccak256(type(EravonBoard).creationCode)));
        assertEq(address(logic), create2Address(keccak256("eravon/1"), keccak256(type(Eravon).creationCode)));
        assertEq(logic.LOGIC(), address(logic));
        // the proxy's runtime code hash, the one the registry admits
        bytes memory runtime = abi.encodePacked(hex"363d3d373d3d3d363d73", address(logic), hex"5af43d82803e903d91602b57fd5bf3");
        assertEq(logic.PROXY_CODEHASH(), keccak256(runtime));
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quoteAuto, 1 ether);
        Eravon inst = deployProxy(t);
        assertEq(address(inst).codehash, logic.PROXY_CODEHASH());
        assertEq(address(inst).code, runtime);
    }

    function test_verifier_missingOrImpostorRefusesInit() public {
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quoteAuto, 1 ether);
        fundDeposit(qAuto, 1 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 1 ether);
        bytes memory code = address(verifier).code;
        vm.etch(address(verifier), "");
        vm.prank(SELLER);
        vm.expectRevert(Eravon.VerifierMissing.selector);
        inst.init(t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        vm.etch(address(verifier), hex"6000");
        vm.prank(SELLER);
        vm.expectRevert(Eravon.VerifierMissing.selector);
        inst.init(t, quoteAuto, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        // the expiry branch and cancel prove the address through the verifier too: nothing moves without it
        Eravon.OrderTerms memory t2 = namedTerms(JUDGE, AMANAH, quoteAuto, 1 ether);
        Eravon inst2 = deployProxy(t2);
        tok.mint(address(inst2), 1 ether);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.VerifierMissing.selector);
        inst2.cancel(t2, "");
        vm.warp(t2.offer.expiry + 1);
        vm.expectRevert(Eravon.VerifierMissing.selector);
        inst2.init(t2, "", "", "", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(address(inst2)), 1 ether, "the money waits for the verifier");
        // the verifier back at its address: the branch runs
        vm.etch(address(verifier), code);
        vm.prank(BUYER);
        inst2.init(t2, "", "", "", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 1 ether);
    }

    function test_verifier_hashOfferAgreesWithTheFixture() public {
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quoteAuto, 1 ether);
        assertEq(verifier.hashOffer(t.offer), offerHash(t));
        t.offer.buyerMayOpen = false;
        assertEq(verifier.hashOffer(t.offer), offerHash(t));
        t.offer.openWindow = 77;
        assertEq(verifier.hashOffer(t.offer), offerHash(t));
        // openBy is in the order, not the offer
        bytes32 oh = verifier.hashOffer(t.offer);
        bytes32 salt = orderHash(t);
        t.openBy = t.openBy + 1;
        assertEq(verifier.hashOffer(t.offer), oh);
        assertTrue(orderHash(t) != salt);
    }

    function _bad(bytes memory quote) internal {
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 1 ether);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadQuote.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
    }

    function _withWord(uint256 i, uint256 w) internal view returns (bytes memory q) {
        q = quoteAuto;
        assembly {
            mstore(add(add(q, 32), mul(i, 32)), w)
        }
    }

    function test_verifier_quoteDecodingRefusals() public {
        _bad(new bytes(479));
        _bad(abi.encodePacked(quoteAuto, hex"00"));
        _bad(_withWord(0, 0)); // token zero
        _bad(_withWord(0, uint256(uint160(address(tok))) | (1 << 160))); // dirty address word
        _bad(_withWord(2, 1 << 160));
        _bad(_withWord(3, 1 << 160));
        _bad(_withWord(4, 1 << 160));
        _bad(_withWord(1, 10001)); // premiumBps
        _bad(_withWord(7, 10001)); // feeBps
        _bad(_withWord(5, uint256(type(uint32).max) + 1)); // maxStale
        _bad(_withWord(11, uint256(type(uint32).max) + 1)); // rulingWindow
        _bad(_withWord(12, uint256(type(uint32).max) + 1)); // offerWindow
        _bad(_withWord(13, 3)); // lapseDefault
        _bad(_withWord(14, 2)); // autoAccept
        _bad(_withWord(3, uint256(uint160(address(0xFEED))))); // a feed with no unit
        _bad(_withWord(4, uint256(uint160(address(0xFEED)))));
        // the accepted edges: lapseDefault 0, 1 and 2 (the default quote's)
        for (uint256 v = 0; v < 2; v++) {
            Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, _withWord(13, v), 1 ether);
            fundDeposit(qAuto, 1 ether);
            Eravon inst = openNamed(t, _withWord(13, v));
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
        }
    }

    function test_verifier_consentViews() public {
        bytes32 d = keccak256("digest");
        vm.expectRevert(Eravon.BadSellerSig.selector);
        verifier.consents(address(logic), SELLER, address(0), d, "", BUYER, d, "");
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        verifier.consents(address(logic), SELLER, SELLER, d, "", address(0), d, "");
        // sender as the seller carries the seller's consent; the buyer's slot then fails
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        verifier.consents(address(logic), SELLER, SELLER, d, "", BUYER, d, "");
        verifier.consents(address(logic), BUYER, SELLER, d, sign(SELLER_PK, d), BUYER, d, "");
        verifier.consents(address(logic), address(0xCAFE), SELLER, d, sign(SELLER_PK, d), BUYER, d, sign(BUYER_PK, d));
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        verifier.verifyBuyer(d, "", BUYER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        verifier.verifyBuyer(d, sign(SELLER_PK, d), BUYER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        verifier.verifyBuyer(d, sign(BUYER_PK, d), address(0));
        verifier.verifyBuyer(d, sign(BUYER_PK, d), BUYER);
        vm.expectRevert();
        verifier.verifyBuyer(d, new bytes(64), BUYER);
    }

    // ---- the board ----

    function test_board_postEmitsAndStoresNothing() public {
        bytes memory doc = "```eravon.listing/1\n{}\n```\n";
        vm.expectEmit(true, true, true, true, address(board));
        emit EravonBoard.Posted(address(this), keccak256("terms"), sha256(doc), doc);
        board.post(keccak256("terms"), doc);
        // any key, any bytes, from anyone, twice
        vm.prank(address(0xBEEF));
        board.post(bytes32(0), "");
        vm.expectEmit(true, true, true, true, address(board));
        emit EravonBoard.Posted(address(0xBEEF), bytes32(0), sha256(""), "");
        vm.prank(address(0xBEEF));
        board.post(bytes32(0), "");
        bytes memory big = new bytes(20000);
        big[0] = 0x7b;
        vm.expectEmit(true, true, true, true, address(board));
        emit EravonBoard.Posted(address(this), keccak256("case"), sha256(big), big);
        board.post(keccak256("case"), big);
        // no storage: a slot read answers zero after every post
        assertEq(vm.load(address(board), bytes32(0)), bytes32(0));
        assertEq(vm.load(address(board), keccak256("terms")), bytes32(0));
    }
}
