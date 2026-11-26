// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {Eravon} from "../src/Eravon.sol";
import {EravonBoard} from "../src/EravonBoard.sol";
import {EravonPanelBook} from "../src/EravonPanelBook.sol";
import {EravonVerifier} from "../src/EravonVerifier.sol";
import {MockERC20} from "./replay/MockERC20.sol";
import {GaleedSuccessionBook} from "./replay/GaleedSuccessionBook.sol";

/// Shared fixture: the public deployer etched at its mainnet address, the
/// family's succession book etched at the address the code names, the
/// logic, the verifier, the board and the chain's panel book deployed
/// through the deployer under their salts, terms and quote builders, the two
/// signatures, the seller's deposit allowance, and the two-step deployment.
abstract contract EravonBase is Test {
    address internal constant DEPLOYER = 0x4e59b44847b379578588920cA78FbF26c0B4956C;
    // Arachnid's deterministic deployment proxy, runtime code
    bytes internal constant DEPLOYER_CODE =
        hex"7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe03601600081602082378035828234f58015156039578182fd5b8082525050506014600cf3";

    uint256 internal constant SELLER_PK = 0xA11CE;
    uint256 internal constant BUYER_PK = 0xB0B;
    address internal SELLER;
    address internal BUYER;
    address internal constant SELLER_PAYOUT = address(0x5E11E7);
    address internal constant BUYER_PAYOUT = address(0xB0B7);

    /// The family's succession book at the address the standard's code names (Shape, s9).
    address internal constant SUCCESSION = 0x3dB7c505dd25361F1A66f6c5Bd96b48df27f668c;

    Eravon internal logic;
    EravonVerifier internal verifier;
    EravonBoard internal board;
    EravonPanelBook internal pbook;
    GaleedSuccessionBook internal sbook;
    MockERC20 internal tok;
    uint256 internal nonceCounter;

    function setUpBase() internal {
        vm.warp(1_800_000_000);
        vm.etch(DEPLOYER, DEPLOYER_CODE);
        vm.etch(SUCCESSION, type(GaleedSuccessionBook).runtimeCode);
        sbook = GaleedSuccessionBook(SUCCESSION);
        logic = Eravon(deployThrough(keccak256("eravon/1"), type(Eravon).creationCode));
        verifier = EravonVerifier(deployThrough(keccak256("eravon/1/verify"), type(EravonVerifier).creationCode));
        board = EravonBoard(deployThrough(keccak256("eravon/1/board"), type(EravonBoard).creationCode));
        pbook = EravonPanelBook(deployThrough(keccak256("eravon/1/panel"), type(EravonPanelBook).creationCode));
        require(address(pbook) == logic.panelBook(), "book address");
        tok = new MockERC20();
        SELLER = vm.addr(SELLER_PK);
        BUYER = vm.addr(BUYER_PK);
    }

    /// CREATE2 through the public deployer: calldata is salt || initcode.
    function deployThrough(bytes32 salt, bytes memory initcode) internal returns (address a) {
        (bool ok, bytes memory ret) = DEPLOYER.call(abi.encodePacked(salt, initcode));
        require(ok && ret.length == 20, "deployer");
        a = address(bytes20(ret));
        require(a.code.length != 0, "no code");
    }

    function proxyInitcode() internal view returns (bytes memory) {
        return abi.encodePacked(
            hex"3d602d80600a3d3981f3363d3d373d3d3d363d73", address(logic), hex"5af43d82803e903d91602b57fd5bf3"
        );
    }

    function create2Address(bytes32 salt, bytes32 initHash) internal pure returns (address) {
        return address(uint160(uint256(keccak256(abi.encodePacked(hex"ff", DEPLOYER, salt, initHash)))));
    }

    // ---- quotes ----

    struct Q {
        address token;
        uint16 premiumBps;
        address feeUnit;
        address feedToken;
        address feedUnit;
        uint32 maxStale;
        uint256 feeFixed;
        uint16 feeBps;
        uint256 feeMin;
        uint256 feeMax;
        uint256 amountCap;
        uint32 rulingWindow;
        uint32 offerWindow;
        uint8 lapseDefault;
        bool autoAccept;
    }

    /// The judge's quote (s3); the second argument is ignored, a quote carrying no payout (terms/1 s3.3).
    function defaultQuote(address token_, address, bool auto_) internal pure returns (Q memory q) {
        q.token = token_;
        q.premiumBps = 100; // 1%
        q.feeFixed = 0;
        q.feeBps = 1000; // 10%
        q.feeMin = 5 ether;
        q.feeMax = 500 ether;
        q.amountCap = 0;
        q.rulingWindow = 14 days;
        q.offerWindow = 3 days;
        q.lapseDefault = 2;
        q.autoAccept = auto_;
    }

    /// The fifteen words in quoteType() order, no tuple wrapping: two static halves concatenated.
    function encodeQuote(Q memory q) internal pure returns (bytes memory) {
        bytes memory a = abi.encode(q.token, q.premiumBps, q.feeUnit, q.feedToken, q.feedUnit, q.maxStale, q.feeFixed);
        bytes memory b = abi.encode(q.feeBps, q.feeMin, q.feeMax, q.amountCap, q.rulingWindow, q.offerWindow, q.lapseDefault, q.autoAccept);
        return abi.encodePacked(a, b);
    }

    /// terms/1 s6.1 over a quote in the settlement token: lf, then P = A * premiumBps / 10000, then D = P + lf.
    function lfOf(Q memory q, uint256 amount_) internal pure returns (uint256 lf) {
        lf = q.feeFixed + (amount_ * q.feeBps) / 10000;
        if (q.feeMin != 0 && lf < q.feeMin) lf = q.feeMin;
        if (q.feeMax != 0 && lf > q.feeMax) lf = q.feeMax;
    }

    function premiumOf(Q memory q, uint256 amount_) internal pure returns (uint256) {
        return (amount_ * q.premiumBps) / 10000;
    }

    function depositOf(Q memory q, uint256 amount_) internal pure returns (uint256) {
        return premiumOf(q, amount_) + lfOf(q, amount_);
    }

    // ---- terms ----

    function threeRungs() internal pure returns (Eravon.Rung[] memory l) {
        l = new Eravon.Rung[](3);
        l[0] = Eravon.Rung(0, 10000, 0);
        l[1] = Eravon.Rung(5000, 5000, 2);
        l[2] = Eravon.Rung(10000, 0, 1);
    }

    /// A named-track order for `judge` under `quote`: T1 7 days, T2 5 days, openWindow 1 day,
    /// buyerMayOpen true, openBy one day out, expiry 30 days out.
    function namedTerms(address judge, bytes32 amanah, bytes memory quote, uint256 amount_)
        internal
        returns (Eravon.OrderTerms memory t)
    {
        t.offer.sellerKey = SELLER;
        t.offer.judge = judge;
        t.offer.guild = address(0);
        t.offer.sellerPayout = SELLER_PAYOUT;
        t.offer.sellerFund = SELLER;
        t.offer.token = address(tok);
        t.offer.amount = amount_;
        t.offer.deliveryWindow = 7 days;
        t.offer.confirmWindow = 5 days;
        t.offer.openWindow = 1 days;
        t.offer.buyerMayOpen = true;
        t.offer.ladder = threeRungs();
        t.offer.termsHash = keccak256("terms document");
        t.offer.amanahHash = amanah;
        t.offer.quoteHash = keccak256(quote);
        t.offer.nonce = ++nonceCounter;
        t.offer.expiry = vm.getBlockTimestamp() + 30 days;
        t.buyerKey = BUYER;
        t.buyerPayout = BUYER_PAYOUT;
        t.openBy = uint64(vm.getBlockTimestamp() + 1 days);
    }

    function guildTerms(address g, bytes32 amanah, bytes memory quote, uint256 amount_)
        internal
        returns (Eravon.OrderTerms memory t)
    {
        t = namedTerms(address(0), amanah, quote, amount_);
        t.offer.guild = g;
    }

    // ---- hashing and signing (the same words the contract and terms/1 use) ----

    bytes32 internal constant TH_DOMAIN =
        keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)");
    bytes32 internal constant TH_OUTCOME = keccak256("Outcome(uint16 buyerBps,uint16 sellerBps,uint8 feeBearer)");
    bytes32 internal constant TH_OFFER = keccak256(
        "Offer(address sellerKey,address judge,address guild,address sellerPayout,address sellerFund,address token,uint256 amount,uint32 deliveryWindow,uint32 confirmWindow,uint32 openWindow,bool buyerMayOpen,Outcome[] ladder,bytes32 termsHash,bytes32 amanahHash,bytes32 quoteHash,uint256 nonce,uint256 expiry)Outcome(uint16 buyerBps,uint16 sellerBps,uint8 feeBearer)"
    );
    bytes32 internal constant TH_ORDER = keccak256(
        "OrderTerms(Offer offer,address buyerKey,address buyerPayout,uint64 openBy,address prev)Offer(address sellerKey,address judge,address guild,address sellerPayout,address sellerFund,address token,uint256 amount,uint32 deliveryWindow,uint32 confirmWindow,uint32 openWindow,bool buyerMayOpen,Outcome[] ladder,bytes32 termsHash,bytes32 amanahHash,bytes32 quoteHash,uint256 nonce,uint256 expiry)Outcome(uint16 buyerBps,uint16 sellerBps,uint8 feeBearer)"
    );
    bytes32 internal constant TH_CLAIM = keccak256("ClaimDelivery(bytes32 manifest)");
    bytes32 internal constant TH_CONFIRM = keccak256("Confirm()");
    bytes32 internal constant TH_DISPUTE = keccak256("Dispute()");
    bytes32 internal constant TH_REFUND = keccak256("Refund()");
    bytes32 internal constant TH_SETTLE = keccak256("Settle(uint16 buyerBps,uint256 expiry)");
    bytes32 internal constant TH_CANCEL = keccak256("Cancel()");

    function domainOf(address verifying) internal view returns (bytes32) {
        return keccak256(abi.encode(TH_DOMAIN, keccak256("ERAVON"), keccak256("1"), block.chainid, verifying));
    }

    function digestOf(address verifying, bytes32 structHash) internal view returns (bytes32) {
        return keccak256(abi.encodePacked(hex"1901", domainOf(verifying), structHash));
    }

    /// hashStruct(Offer), computed here so that no external call sits between a prank or an expectRevert and its target.
    function offerHash(Eravon.OrderTerms memory t) internal pure returns (bytes32) {
        Eravon.Offer memory o = t.offer;
        bytes32[] memory rungs = new bytes32[](o.ladder.length);
        for (uint256 i = 0; i < rungs.length; i++) {
            rungs[i] = keccak256(abi.encode(TH_OUTCOME, uint256(o.ladder[i].buyerBps), uint256(o.ladder[i].sellerBps), uint256(o.ladder[i].feeBearer)));
        }
        bytes memory head = abi.encode(TH_OFFER, o.sellerKey, o.judge, o.guild, o.sellerPayout, o.sellerFund, o.token, o.amount, uint256(o.deliveryWindow), uint256(o.confirmWindow), uint256(o.openWindow));
        bytes memory tail = abi.encode(
            o.buyerMayOpen ? uint256(1) : uint256(0), keccak256(abi.encodePacked(rungs)), o.termsHash, o.amanahHash, o.quoteHash, o.nonce, o.expiry
        );
        return keccak256(abi.encodePacked(head, tail));
    }

    function orderHash(Eravon.OrderTerms memory t) internal pure returns (bytes32) {
        return keccak256(abi.encode(TH_ORDER, offerHash(t), t.buyerKey, t.buyerPayout, uint256(t.openBy), t.prev));
    }

    function instanceOf(Eravon.OrderTerms memory t) internal view returns (address) {
        return create2Address(orderHash(t), keccak256(proxyInitcode()));
    }

    function sign(uint256 pk, bytes32 digest) internal pure returns (bytes memory) {
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(pk, digest);
        return abi.encodePacked(r, s, v);
    }

    function sellerSig(Eravon.OrderTerms memory t) internal view returns (bytes memory) {
        return sign(SELLER_PK, digestOf(address(logic), offerHash(t)));
    }

    function buyerSig(Eravon.OrderTerms memory t) internal view returns (bytes memory) {
        return sign(BUYER_PK, digestOf(instanceOf(t), orderHash(t)));
    }

    function actionDigest(address instance, bytes32 structHash) internal view returns (bytes32) {
        return digestOf(instance, structHash);
    }

    function cancelSig(Eravon.OrderTerms memory t) internal view returns (bytes memory) {
        return sign(BUYER_PK, digestOf(instanceOf(t), keccak256(abi.encode(TH_CANCEL))));
    }

    // ---- deployment ----

    /// Step one of s1: the proxy at the order's address, not yet initialised.
    function deployProxy(Eravon.OrderTerms memory t) internal returns (Eravon inst) {
        address a = deployThrough(orderHash(t), proxyInitcode());
        require(a == instanceOf(t), "address");
        inst = Eravon(a);
    }

    /// s7a: the seller's deposit as an allowance to the logic, the key funded for it.
    function fundDeposit(uint256 d) internal {
        tok.mint(SELLER, d);
        uint256 have = tok.allowance(SELLER, address(logic));
        vm.prank(SELLER);
        tok.approve(address(logic), have + d);
    }

    function fundDeposit(Q memory q, uint256 amount_) internal {
        fundDeposit(depositOf(q, amount_));
    }

    /// Funds the address and runs both steps on the named track, the seller opening.
    function openNamed(Eravon.OrderTerms memory t, bytes memory quote) internal returns (Eravon inst) {
        inst = deployProxy(t);
        tok.mint(address(inst), t.offer.amount);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
    }

    function openGuild(
        Eravon.OrderTerms memory t,
        bytes memory quote,
        address[] memory members,
        bytes32[] memory words
    ) internal returns (Eravon inst) {
        inst = deployProxy(t);
        tok.mint(address(inst), t.offer.amount);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, words);
        drawNext(inst, members, words);
    }

    /// The guild track's draw (standard s7): from the block after the arming, by anyone with the frozen panel (the
    /// base sends it from the seller).
    function drawNext(Eravon inst, address[] memory members, bytes32[] memory words) internal {
        vm.roll(vm.getBlockNumber() + 1);
        vm.prank(SELLER);
        inst.redraw(members, words);
    }

    /// The chain's panel book (standard s7), deployed once in setUpBase; the guild argument is the sender's to name.
    function deployBook(address) internal view returns (EravonPanelBook) {
        return pbook;
    }

    function phaseOf(Eravon inst) internal view returns (Eravon.Phase) {
        return inst.status().phase;
    }

    function depositHeld(Eravon inst) internal view returns (uint256) {
        return inst.status().deposit;
    }

    function premiumHeld(Eravon inst) internal view returns (uint256) {
        return inst.status().premium;
    }

    /// The guild engages a panel on the chain's book from its current key, no router; the book files it under
    /// the guild's root, a block before any case that freezes it.
    function engagePanel(address g, address[] memory members, bytes32[] memory words, uint64 expires) internal {
        bytes32 key = logic.PROXY_CODEHASH();
        vm.prank(g);
        pbook.engage(key, members, words, bytes32(0), expires, address(0));
        vm.roll(vm.getBlockNumber() + 1);
    }

    /// A key's record moves on the succession book: the current key declares `to`, `to` accedes (galeed.roster/1 s4).
    function moveKey(address from, address to) internal {
        vm.prank(from);
        sbook.declare(to);
        vm.prank(to);
        sbook.accede(from);
    }

    /// The verifier reads the funding registry from its caller's logic (an instance's LOGIC()); a test that
    /// calls the verifier directly answers as an instance of this logic.
    function LOGIC() external view returns (address) {
        return address(logic);
    }

    // ---- the parties' moves ----

    function claim(Eravon inst, bytes32 manifest) internal {
        vm.prank(SELLER);
        inst.claimDelivery(manifest);
    }

    /// The buyer disputes, the fee pulled from the buyer's key.
    function raise(Eravon inst) internal {
        (uint256 lf,,,,) = inst.feeQuoted();
        tok.mint(BUYER, lf);
        vm.prank(BUYER);
        tok.approve(address(inst), lf);
        vm.prank(BUYER);
        inst.dispute();
    }
}
