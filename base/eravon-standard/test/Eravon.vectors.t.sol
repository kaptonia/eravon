// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {MockERC20} from "./replay/MockERC20.sol";

/// The contract against the eravon.terms/1 criterion: every EIP-712 value the
/// two share (hashStruct(Offer), the salt and case id, the instance address,
/// the order digest, the six action struct hashes and their digests) is read
/// from test/vectors/terms-eip712.json, which `ert offer|order|action`
/// produced over eight terms documents, and asserted here. The values the
/// contract computes internally are proved by driving live cases with
/// signatures over the criterion's struct hashes: a case opens only if the
/// contract's salt is the criterion's, and a signed action lands only if its
/// struct hash is.
contract EravonVectors is EravonBase {
    string internal json;
    uint256 internal sellerPriv;
    uint256 internal buyerPriv;
    bytes32 internal constant MANIFEST = 0x1111111111111111111111111111111111111111111111111111111111111111;

    function setUp() public {
        setUpBase();
        vm.chainId(1);
        json = vm.readFile("test/vectors/terms-eip712.json");
        sellerPriv = vm.parseJsonUint(json, ".sellerPriv");
        buyerPriv = vm.parseJsonUint(json, ".buyerPriv");
    }

    function _p(uint256 i, string memory tail) internal pure returns (string memory) {
        return string(abi.encodePacked(".vectors[", vm.toString(i), "]", tail));
    }

    function _offer(uint256 i) internal view returns (Eravon.OrderTerms memory t) {
        string memory o = _p(i, ".offer");
        t.offer.sellerKey = vm.parseJsonAddress(json, string(abi.encodePacked(o, ".sellerKey")));
        t.offer.judge = vm.parseJsonAddress(json, string(abi.encodePacked(o, ".judge")));
        t.offer.guild = vm.parseJsonAddress(json, string(abi.encodePacked(o, ".guild")));
        t.offer.sellerPayout = vm.parseJsonAddress(json, string(abi.encodePacked(o, ".sellerPayout")));
        t.offer.sellerFund = vm.parseJsonAddress(json, string(abi.encodePacked(o, ".sellerFund")));
        t.offer.token = vm.parseJsonAddress(json, string(abi.encodePacked(o, ".token")));
        t.offer.amount = vm.parseUint(vm.parseJsonString(json, string(abi.encodePacked(o, ".amount"))));
        t.offer.deliveryWindow = uint32(vm.parseJsonUint(json, string(abi.encodePacked(o, ".deliveryWindow"))));
        t.offer.confirmWindow = uint32(vm.parseJsonUint(json, string(abi.encodePacked(o, ".confirmWindow"))));
        t.offer.openWindow = uint32(vm.parseJsonUint(json, string(abi.encodePacked(o, ".openWindow"))));
        t.offer.buyerMayOpen = vm.parseJsonBool(json, string(abi.encodePacked(o, ".buyerMayOpen")));
        uint256 n = 0;
        while (vm.keyExistsJson(json, string(abi.encodePacked(o, ".ladder[", vm.toString(n), "].buyerBps")))) n++;
        t.offer.ladder = new Eravon.Rung[](n);
        for (uint256 j = 0; j < n; j++) {
            string memory r = string(abi.encodePacked(o, ".ladder[", vm.toString(j), "]"));
            t.offer.ladder[j] = Eravon.Rung(
                uint16(vm.parseJsonUint(json, string(abi.encodePacked(r, ".buyerBps")))),
                uint16(vm.parseJsonUint(json, string(abi.encodePacked(r, ".sellerBps")))),
                uint8(vm.parseJsonUint(json, string(abi.encodePacked(r, ".feeBearer"))))
            );
        }
        t.offer.termsHash = vm.parseJsonBytes32(json, string(abi.encodePacked(o, ".termsHash")));
        t.offer.amanahHash = vm.parseJsonBytes32(json, string(abi.encodePacked(o, ".amanahHash")));
        t.offer.quoteHash = vm.parseJsonBytes32(json, string(abi.encodePacked(o, ".quoteHash")));
        t.offer.nonce = vm.parseJsonUint(json, string(abi.encodePacked(o, ".nonce")));
        t.offer.expiry = vm.parseJsonUint(json, string(abi.encodePacked(o, ".expiry")));
        t.buyerKey = vm.parseJsonAddress(json, _p(i, ".buyerKey"));
        t.buyerPayout = vm.parseJsonAddress(json, _p(i, ".buyerPayout"));
        t.openBy = uint64(vm.parseJsonUint(json, _p(i, ".openBy")));
        t.prev = vm.parseJsonAddress(json, _p(i, ".prev"));
    }

    function _word(bytes memory b, uint256 i) internal pure returns (uint256 w) {
        assembly {
            w := mload(add(add(b, 32), mul(i, 32)))
        }
    }

    function _action(uint256 i, string memory name) internal view returns (bytes32 sh, bytes32 d) {
        sh = vm.parseJsonBytes32(json, _p(i, string(abi.encodePacked(".actions.", name, ".structHash"))));
        d = vm.parseJsonBytes32(json, _p(i, string(abi.encodePacked(".actions.", name, ".digest"))));
    }

    function test_vectors() public {
        uint256 i = 0;
        uint256 live = 0;
        while (vm.keyExistsJson(json, _p(i, ".salt"))) {
            Eravon.OrderTerms memory t = _offer(i);
            bytes32 structHash = vm.parseJsonBytes32(json, _p(i, ".offerStructHash"));
            bytes32 salt = vm.parseJsonBytes32(json, _p(i, ".salt"));
            address vLogic = vm.parseJsonAddress(json, _p(i, ".logic"));
            address vInstance = vm.parseJsonAddress(json, _p(i, ".instance"));
            bytes memory quote = vm.parseJsonBytes(json, _p(i, ".quote"));
            assertEq(keccak256(quote), t.offer.quoteHash, "the quote bytes are the candidate's");
            // the contract's hashStruct(Offer) is the criterion's
            assertEq(verifier.hashOffer(t.offer), structHash, "hashStruct(Offer)");
            // the fixture's order hash, domain and initcode agree with the criterion
            assertEq(orderHash(t), salt, "salt");
            assertEq(digestOf(vInstance, salt), vm.parseJsonBytes32(json, _p(i, ".orderDigest")), "order digest");
            bytes memory vInit = abi.encodePacked(hex"3d602d80600a3d3981f3363d3d373d3d3d363d73", vLogic, hex"5af43d82803e903d91602b57fd5bf3");
            assertEq(create2Address(salt, keccak256(vInit)), vInstance, "CREATE2 instance");
            assertEq(digestOf(vLogic, structHash), vm.parseJsonBytes32(json, _p(i, ".offerDigest")), "offer digest");
            // the same order as a later stage (terms/1 s4.4): prev in the salt, another instance, the actions under it
            {
                address prev0 = t.prev;
                t.prev = vm.parseJsonAddress(json, _p(i, ".later.prev"));
                bytes32 saltL = vm.parseJsonBytes32(json, _p(i, ".later.salt"));
                address instL = vm.parseJsonAddress(json, _p(i, ".later.instance"));
                assertEq(orderHash(t), saltL, "later stage: salt");
                (bytes32 vs,) = verifier.salt(t);
                assertEq(vs, saltL, "later stage: the verifier's salt");
                assertEq(create2Address(saltL, keccak256(vInit)), instL, "later stage: CREATE2 instance");
                assertEq(digestOf(instL, saltL), vm.parseJsonBytes32(json, _p(i, ".later.orderDigest")), "later stage: order digest");
                assertEq(digestOf(instL, keccak256(abi.encode(TH_CONFIRM))), vm.parseJsonBytes32(json, _p(i, ".later.confirmDigest")), "later stage: Confirm digest");
                assertTrue(saltL != salt && instL != vInstance, "a later stage is another order and another instance");
                t.prev = prev0;
            }
            // the six action struct hashes and their digests under the instance's domain
            _checkAction(i, vInstance, "ClaimDelivery", keccak256(abi.encode(TH_CLAIM, MANIFEST)));
            _checkAction(i, vInstance, "Confirm", keccak256(abi.encode(TH_CONFIRM)));
            _checkAction(i, vInstance, "Dispute", keccak256(abi.encode(TH_DISPUTE)));
            _checkAction(i, vInstance, "Refund", keccak256(abi.encode(TH_REFUND)));
            _checkAction(i, vInstance, "Settle", keccak256(abi.encode(TH_SETTLE, uint256(5000), uint256(1860000000))));
            _checkAction(i, vInstance, "Cancel", keccak256(abi.encode(TH_CANCEL)));
            bool priced = _word(quote, 2) != 0;
            if (vm.parseJsonBool(json, _p(i, ".named")) && t.offer.amount < type(uint128).max && !priced) {
                _live(i, t, structHash, salt, quote);
                live++;
            }
            i++;
        }
        assertTrue(i >= 11 && live >= 7, "vector file read");
    }

    function _checkAction(uint256 i, address vInstance, string memory name, bytes32 expected) internal view {
        (bytes32 sh, bytes32 d) = _action(i, name);
        assertEq(sh, expected, string(abi.encodePacked(name, " struct hash")));
        assertEq(digestOf(vInstance, sh), d, string(abi.encodePacked(name, " digest")));
    }

    uint256 internal _fundBefore;
    uint256 internal _keyBefore;

    /// The deposit from the order's sellerFund: the key, or an address bound to the seller on the logic (s7a).
    function _fundFrom(MockERC20 token_, address fund, address seller, uint256 d) internal {
        token_.mint(fund, d);
        vm.prank(fund);
        token_.approve(address(logic), d);
        if (fund != seller) {
            vm.prank(fund);
            logic.acceptFunding(seller);
        }
        _fundBefore = token_.balanceOf(fund);
        _keyBefore = token_.balanceOf(seller);
    }

    function _checkFunded(MockERC20 token_, address fund, address seller, uint256 d) internal view {
        assertEq(_fundBefore - token_.balanceOf(fund), d, "the deposit left sellerFund");
        if (fund != seller) assertEq(token_.balanceOf(seller), _keyBefore, "the key paid nothing");
    }

    /// Opens the vector's case under this test's logic with signatures over the
    /// criterion's struct hashes, then lands signed actions by vector index.
    function _live(uint256 i, Eravon.OrderTerms memory t, bytes32 structHash, bytes32 salt, bytes memory quote) internal {
        if (t.offer.token.code.length == 0) vm.etch(t.offer.token, address(tok).code);
        MockERC20 token_ = MockERC20(t.offer.token);
        address seller = vm.addr(sellerPriv);
        assertEq(seller, t.offer.sellerKey, "the vector's seller key");
        assertEq(vm.addr(buyerPriv), t.buyerKey, "the vector's buyer key");
        Eravon inst = deployProxy(t);
        token_.mint(address(inst), t.offer.amount);
        uint256 path = i % 4;
        if (path == 0) {
            // Cancel() before any case, by signature sent by the seller (a stranger is no party)
            (bytes32 sh,) = _action(i, "Cancel");
            uint256 before = token_.balanceOf(t.buyerPayout);
            bytes memory cs = sign(buyerPriv, digestOf(address(inst), sh));
            vm.prank(address(0xCAFE));
            vm.expectRevert(Eravon.NotParty.selector);
            inst.cancel(t, cs);
            vm.prank(seller);
            inst.cancel(t, cs);
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded), "Cancel()");
            assertEq(token_.balanceOf(t.buyerPayout) - before, t.offer.amount);
            return;
        }
        // the deposit from the quote's words: P + lf
        uint256 lf = _word(quote, 6) + (t.offer.amount * _word(quote, 7)) / 10000;
        if (_word(quote, 8) != 0 && lf < _word(quote, 8)) lf = _word(quote, 8);
        if (_word(quote, 9) != 0 && lf > _word(quote, 9)) lf = _word(quote, 9);
        uint256 d = (t.offer.amount * _word(quote, 1)) / 10000 + lf;
        _fundFrom(token_, t.offer.sellerFund, seller, d);
        bytes memory sigS = sign(sellerPriv, digestOf(address(logic), structHash));
        bytes memory sigB = sign(buyerPriv, digestOf(address(inst), salt));
        vm.prank(t.offer.buyerMayOpen ? t.buyerKey : seller);
        inst.init(t, quote, sigB, sigS, new address[](0), new bytes32[](0));
        // the contract's case id is the criterion's salt
        assertEq(inst.seatOf(salt, 0).judge, t.offer.judge, "caseId is the salt");
        assertEq(depositHeld(inst), d, "the deposit the quote names");
        _checkFunded(token_, t.offer.sellerFund, seller, d);
        if (_word(quote, 14) != 1) {
            vm.prank(t.offer.judge, t.offer.judge);
            inst.accept(salt, 0);
        }
        if (path == 1) {
            (bytes32 sh,) = _action(i, "Refund");
            vm.prank(seller);
            inst.refundBySig(sign(sellerPriv, digestOf(address(inst), sh)));
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded), "Refund()");
            return;
        }
        (bytes32 shC,) = _action(i, "ClaimDelivery");
        vm.prank(seller);
        inst.claimDeliveryBySig(MANIFEST, sign(sellerPriv, digestOf(address(inst), shC)));
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Claimed), "ClaimDelivery(bytes32)");
        if (path == 2) {
            (bytes32 sh,) = _action(i, "Confirm");
            vm.prank(seller);
            inst.confirmBySig(sign(buyerPriv, digestOf(address(inst), sh)));
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released), "Confirm()");
            return;
        }
        // the fee sent ahead, the dispute by signature, then a settlement under both signatures
        token_.mint(address(inst), lf);
        (bytes32 shD,) = _action(i, "Dispute");
        vm.prank(seller);
        inst.disputeBySig(sign(buyerPriv, digestOf(address(inst), shD)));
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Disputed), "Dispute()");
        (bytes32 shS,) = _action(i, "Settle");
        vm.prank(seller);
        inst.settle(5000, 1860000000, sign(buyerPriv, digestOf(address(inst), shS)), sign(sellerPriv, digestOf(address(inst), shS)));
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Settled), "Settle(uint16,uint256)");
    }
}
