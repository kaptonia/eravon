// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {VenueReplay} from "./replay/VenueReplay.sol";
import {IGaleedVenue} from "../src/IGaleedVenue.sol";
import {Eravon} from "../src/Eravon.sol";
import {EravonPanelBook} from "../src/EravonPanelBook.sol";
import {EravonBase} from "./EravonBase.sol";
import {GaleedSuccessionBook} from "./replay/GaleedSuccessionBook.sol";

/// What the two tracks share: one instance per case (`venueOf`), the family's
/// succession book at the constant the code names (`bookAt`), the seller's
/// deposits by an allowance to the logic (s7a, s12), the dispute as s8 takes
/// it for `outcomeOf` (the seller's claim, then the buyer's `dispute` with the
/// fee pulled), the two roles of `partyRoles()` (s8: the buyer role is
/// `buyerKey` and `buyerPayout`, the seller role `sellerKey`, `sellerFund` and
/// `sellerPayout`), and `sweep` as the path by which a role's address takes what
/// the instance holds for it.
abstract contract EravonReplayBase is VenueReplay, EravonBase {
    mapping(bytes32 => address) internal instanceByCase;
    mapping(bytes32 => bytes) internal quoteByCase;
    /// the addresses of the two roles, as each case's order names them
    struct Roles {
        address buyerKey;
        address buyerPayout;
        address sellerKey;
        address sellerPayout;
        address sellerFund;
    }

    mapping(bytes32 => Roles) internal rolesByCase;

    function setUp() public virtual override {
        setUpBase();
        // the harness token is the settlement token; the base's own token is unused here
        VenueReplay.setUp();
    }

    function bookAt() internal pure override returns (address) {
        return SUCCESSION;
    }

    function venueOf(bytes32 id) internal view override returns (IGaleedVenue) {
        return IGaleedVenue(instanceByCase[id]);
    }

    function seatsBeforeAcceptance() internal pure virtual override returns (bool) {
        return true;
    }

    function offerClockRuns() internal pure override returns (bool) {
        return true;
    }

    /// The seller's allowance to the logic for every deposit of the replay, from the order's sellerFund.
    function _allowDeposits(address fund) internal {
        token.mint(fund, 1_000_000 ether);
        vm.prank(fund);
        token.approve(address(logic), type(uint256).max);
    }

    function _deadlines(Eravon.OrderTerms memory t) internal view {
        t.offer.token = address(token);
        t.offer.expiry = vm.getBlockTimestamp() + 3650 days;
        t.openBy = uint64(vm.getBlockTimestamp() + 3650 days);
    }

    /// The instance of `t`, deployed once; the buyer's amount sent to it.
    function _instance(Eravon.OrderTerms memory t, bytes memory quote) internal returns (bytes32 id, Eravon inst) {
        id = orderHash(t);
        address a = instanceByCase[id];
        if (a == address(0)) {
            inst = deployProxy(t);
            instanceByCase[id] = address(inst);
        } else {
            inst = Eravon(a);
        }
        quoteByCase[id] = quote;
        rolesByCase[id] = Roles(t.buyerKey, t.buyerPayout, t.offer.sellerKey, t.offer.sellerPayout, t.offer.sellerFund);
        token.mint(address(inst), t.offer.amount);
    }

    /// Accepted seat to Disputed: the seller claims, the buyer disputes with the fee pulled from its key.
    function dispute(bytes32 id, uint64 s) internal override {
        Eravon inst = Eravon(instanceByCase[id]);
        if (inst.seatOf(id, s).state == 1) return;
        vm.prank(_sbookCurrent(SELLER));
        inst.claimDelivery(keccak256("delivery manifest"));
        (uint256 lf,,,,) = inst.feeQuoted();
        address b = _sbookCurrent(BUYER);
        token.mint(b, lf);
        vm.prank(b);
        token.approve(address(inst), lf);
        vm.prank(b);
        inst.dispute();
    }

    function party(bytes32) internal view override returns (address) {
        return _sbookCurrent(BUYER);
    }

    function quoteOf(bytes32 id) internal view override returns (bool, bytes memory) {
        return (true, quoteByCase[id]);
    }

    function roleAddresses(bytes32 id, uint256 role) internal view override returns (address[] memory xs) {
        Roles storage r = rolesByCase[id];
        if (role == 0) {
            xs = new address[](2);
            xs[0] = r.buyerKey;
            xs[1] = r.buyerPayout;
        } else {
            bool same = r.sellerFund == r.sellerKey;
            xs = new address[](same ? 2 : 3);
            xs[0] = r.sellerKey;
            xs[1] = r.sellerPayout;
            if (!same) xs[2] = r.sellerFund;
        }
    }

    /// What the instance holds for an address of a role, a party's `sweep` pushes to it.
    function collect(bytes32 id, address who) internal override {
        vm.prank(_sbookCurrent(BUYER));
        Eravon(instanceByCase[id]).sweep(who, address(token));
    }

    function _sbookCurrent(address a) internal view returns (address) {
        return succession.current(a);
    }
}

/// galeed.venue/1 s8 replayed against ERAVON on the named track: a judge the
/// parties named, an open offer with a clock, and, through openCarriedCase,
/// the same judge's quote with autoAccept set.
contract EravonNamedReplay is EravonReplayBase {
    bytes internal quoteOffer;
    bytes internal quoteCarried;
    Eravon.OrderTerms internal firstTerms;
    bool internal firstOpened;

    function deployVenue() internal virtual override returns (IGaleedVenue) {
        Q memory q = defaultQuote(address(token), address(0), false);
        quoteOffer = encodeQuote(q);
        q.autoAccept = true;
        quoteCarried = encodeQuote(q);
        _allowDeposits(SELLER);
        // the first case's instance, prepared: the replay reads it as `venue` before the first case opens
        firstTerms = _order(quoteOffer);
        Eravon inst = deployProxy(firstTerms);
        instanceByCase[orderHash(firstTerms)] = address(inst);
        return IGaleedVenue(address(inst));
    }

    function _order(bytes memory quote) internal virtual returns (Eravon.OrderTerms memory t) {
        t = namedTerms(JUDGE, AMANAH, quote, 100 ether);
        _deadlines(t);
    }

    function venueDraws() internal pure override returns (bool) {
        return false;
    }

    function _open(Eravon.OrderTerms memory t, bytes memory quote) internal returns (bytes32 id) {
        Eravon inst;
        (id, inst) = _instance(t, quote);
        vm.prank(_sbookCurrent(SELLER));
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
    }

    function openCase() internal override returns (bytes32 id, uint64 seat) {
        Eravon.OrderTerms memory t;
        if (!firstOpened) {
            firstOpened = true;
            t = firstTerms;
        } else {
            t = _order(quoteOffer);
        }
        id = _open(t, quoteOffer);
        seat = 0;
    }

    function openCarriedCase() internal override returns (bytes32 id, uint64 seat) {
        id = _open(_order(quoteCarried), quoteCarried);
        seat = 0;
    }
}

/// The named track once more for a company seller (s7a, s12): the order's
/// `sellerFund` is the company's wallet, an address other than `sellerKey`,
/// bound to the seller by `acceptFunding` before any case opens; every deposit
/// is pulled from the wallet and every returned deposit lands there.
contract EravonCompanyReplay is EravonNamedReplay {
    address internal constant WALLET = address(0xC0A1E7);

    function deployVenue() internal override returns (IGaleedVenue) {
        _allowDeposits(WALLET);
        vm.prank(WALLET);
        logic.acceptFunding(SELLER);
        return super.deployVenue();
    }

    function _order(bytes memory quote) internal override returns (Eravon.OrderTerms memory t) {
        t = super._order(quote);
        t.offer.sellerFund = WALLET;
    }

    function test_companySeller_depositsLeaveAndReturnAtTheWallet() public {
        uint256 before = token.balanceOf(WALLET);
        uint256 keyBefore = token.balanceOf(SELLER);
        (bytes32 id,) = openCase();
        Eravon inst = Eravon(address(venueOf(id)));
        uint256 d = token.balanceOf(address(inst)) - 100 ether;
        assertTrue(d != 0, "a deposit was pulled");
        assertEq(token.balanceOf(WALLET), before - d, "pulled from the wallet");
        assertEq(token.balanceOf(SELLER), keyBefore, "nothing from the key");
        vm.prank(SELLER);
        inst.refund();
        assertEq(token.balanceOf(WALLET), before, "returned to the wallet");
    }
}

/// The same replay on the guild track: the judge drawn from the chain's panel
/// book under the guild's root, an offer clock under the guild's quote. The
/// order names the guild's covenant word, so the path reads the panel the
/// case will freeze (s7: the latest record under the root and proxyCodeHash
/// recorded before init's block and unexpired) from the replay's own record
/// of the panels it engaged, puts that panel's word in the order, opens the
/// case with the panel in calldata, and draws seat 0 in the next block by the
/// seller's `redraw`. A case whose draw seats nobody (a router that fails or
/// names no member) is a path that reaches no `Seated`: the hook reverts and
/// takes the case back, as a creation refused.
contract EravonGuildReplay is EravonReplayBase {
    bytes internal quoteGuild;
    Eravon.OrderTerms internal firstTerms;
    bool internal firstOpened;

    function deployVenue() internal override returns (IGaleedVenue) {
        Q memory q = defaultQuote(address(token), address(0), false);
        quoteGuild = encodeQuote(q);
        _allowDeposits(SELLER);
        firstTerms = _order(GUILD, AMANAH);
        Eravon inst = deployProxy(firstTerms);
        instanceByCase[orderHash(firstTerms)] = address(inst);
        return IGaleedVenue(address(inst));
    }

    function _order(address g, bytes32 word) internal returns (Eravon.OrderTerms memory t) {
        t = guildTerms(g, word, quoteGuild, 100 ether);
        _deadlines(t);
    }

    function venueDraws() internal pure override returns (bool) {
        return true;
    }

    function panelTerms() internal view override returns (bytes32) {
        return logic.PROXY_CODEHASH();
    }

    function panelBook() internal view override returns (address) {
        return address(pbook);
    }

    function opensCaseNaming() internal pure override returns (bool) {
        return true;
    }

    function openCase() internal override returns (bytes32 id, uint64 seat) {
        return _openGuild(GUILD);
    }

    function openCaseNaming(address g) internal override returns (bytes32 id, uint64 seat) {
        return _openGuild(g);
    }

    /// The record s7 freezes for a case naming `named`, read from the replay's own record of its panels.
    function _latest(address named) internal view returns (bool, uint256) {
        address root = succession.root(named);
        bytes32 T = logic.PROXY_CODEHASH();
        for (uint256 i = _rec.length; i > 0; i--) {
            RPanel storage p = _rec[i - 1];
            if (p.root == root && p.terms == T && p.recorded < vm.getBlockNumber() && p.expires >= vm.getBlockTimestamp()) {
                return (true, i - 1);
            }
        }
        return (false, 0);
    }

    function _openGuild(address named) internal returns (bytes32 id, uint64 seat) {
        (bool has, uint256 pi) = _latest(named);
        address[] memory m;
        bytes32[] memory w;
        bytes32 word = AMANAH;
        if (has) {
            m = _rec[pi].members;
            word = _rec[pi].word;
            w = new bytes32[](m.length);
            for (uint256 i = 0; i < m.length; i++) w[i] = word;
        }
        Eravon.OrderTerms memory t;
        if (!firstOpened && named == GUILD && word == AMANAH) {
            firstOpened = true;
            t = firstTerms;
        } else {
            t = _order(named, word);
        }
        Eravon inst;
        (id, inst) = _instance(t, quoteGuild);
        address sk = _sbookCurrent(SELLER);
        vm.prank(sk);
        inst.init(t, quoteGuild, buyerSig(t), sellerSig(t), m, w);
        // the draw, in the block after the arming; a failed draw leaves the block where the creation found it
        uint256 n = vm.getBlockNumber();
        vm.roll(n + 1);
        vm.prank(sk);
        try inst.redraw(m, w) {}
        catch (bytes memory err) {
            vm.roll(n);
            assembly ("memory-safe") {
                revert(add(err, 32), mload(err))
            }
        }
        if (inst.status().seats == 0 || !_inFlight(inst.seatOf(id, 0).state)) {
            vm.roll(n);
            revert("the draw seated nobody");
        }
        seat = 0;
    }
}

/// Beside the two tracks (standard s12): the buyer's cancellation before
/// `init` and never after, the book's fifth rejection on its own, and the
/// party gates under moved keys (EravonKeys below).
contract EravonReplayExtras is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah word of the judge");

    function setUp() public {
        setUpBase();
    }

    function test_cancel_beforeInitOnly() public {
        Q memory q = defaultQuote(address(tok), address(0), true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 100 ether);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        // the buyer cancels by its own call: the whole balance to the buyer's payout, the case closed
        vm.prank(BUYER);
        inst.cancel(t, "");
        assertEq(tok.balanceOf(BUYER_PAYOUT), 100 ether, "refunded");
        assertEq(uint256(phaseOf(inst)), uint256(Eravon.Phase.Refunded), "closed");
        // init after it is refused
        fundDeposit(q, 100 ether);
        tok.mint(address(inst), 100 ether);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.AlreadyInitialized.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        // a second case: init first, then cancel is refused
        Eravon.OrderTerms memory t2 = namedTerms(JUDGE, AMANAH, quote, 100 ether);
        Eravon inst2 = openNamed(t2, quote);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.AlreadyInitialized.selector);
        inst2.cancel(t2, "");
        // the signed form sent by the seller, on a third case; a stranger is no party
        Eravon.OrderTerms memory t3 = namedTerms(JUDGE, AMANAH, quote, 7 ether);
        Eravon inst3 = deployProxy(t3);
        tok.mint(address(inst3), 7 ether);
        bytes memory cs = cancelSig(t3);
        vm.prank(address(0xCAFE));
        vm.expectRevert(Eravon.NotParty.selector);
        inst3.cancel(t3, cs);
        vm.prank(SELLER);
        inst3.cancel(t3, cs);
        assertEq(tok.balanceOf(BUYER_PAYOUT), 107 ether, "refunded by signature");
    }

    function test_book_fifthRejection() public {
        address g = address(0x2000000000000000000000000000000000000001);
        address[] memory pair = new address[](2);
        pair[0] = JUDGE;
        pair[1] = address(uint160(JUDGE) + 1);
        bytes32[] memory differ = new bytes32[](2);
        differ[0] = AMANAH;
        differ[1] = bytes32(uint256(AMANAH) ^ 1);
        bytes32 T = logic.PROXY_CODEHASH();
        uint64 future = uint64(vm.getBlockTimestamp() + 1 days);
        vm.prank(g);
        vm.expectRevert(EravonPanelBook.WordsDiffer.selector);
        pbook.engage(T, pair, differ, keccak256("roster"), future, address(0));
        bytes32[] memory same = new bytes32[](2);
        same[0] = AMANAH;
        same[1] = AMANAH;
        vm.prank(g);
        pbook.engage(T, pair, same, keccak256("roster"), future, address(0));
        assertEq(pbook.count(g, T), 1, "one record");
    }
}

/// Standard s9 and s11 item 16, the party gates under moved keys: once a
/// party's record has moved on the succession book the code names, every
/// party gate admits the record's current key alone, and the key that handed
/// the record on acts, signs and registers for nothing; `parties` keeps the
/// recorded addresses, every payout and the fee's source stay the recorded
/// ones, and no book but `successionBook()`'s is read.
contract EravonKeys is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE_PAYOUT = address(0x4000000000000000000000000000000000000001);
    address internal constant STRANGER = address(0x3000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah");
    bytes32 internal constant MANIFEST = keccak256("manifest");
    uint256 internal constant A = 100 ether;
    uint256 internal constant S2_PK = 0x5E22;
    uint256 internal constant B2_PK = 0xB22;
    address internal S2;
    address internal B2;
    Q internal qa;
    bytes internal quote;

    function setUp() public {
        setUpBase();
        S2 = vm.addr(S2_PK);
        B2 = vm.addr(B2_PK);
        qa = defaultQuote(address(tok), address(0), true);
        quote = encodeQuote(qa);
    }

    function _order() internal returns (Eravon.OrderTerms memory t, Eravon inst) {
        t = namedTerms(JUDGE, AMANAH, quote, A);
        fundDeposit(qa, A);
        inst = deployProxy(t);
        tok.mint(address(inst), A);
    }

    function _open() internal returns (Eravon.OrderTerms memory t, Eravon inst, bytes32 id) {
        (t, inst) = _order();
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        id = orderHash(t);
    }

    function _act(uint256 pk, Eravon inst, bytes32 structHash) internal view returns (bytes memory) {
        return sign(pk, digestOf(address(inst), structHash));
    }

    function _moveBoth() internal {
        moveKey(SELLER, S2);
        moveKey(BUYER, B2);
        assertEq(sbook.current(SELLER), S2);
        assertEq(sbook.current(BUYER), B2);
    }

    // ---- the gate ----

    function test_keys_initTakesTheCurrentKeysConsentsAndCall() public {
        (Eravon.OrderTerms memory t, Eravon inst) = _order();
        _moveBoth();
        bytes memory oldS = sellerSig(t);
        bytes memory oldB = buyerSig(t);
        bytes memory newS = sign(S2_PK, digestOf(address(logic), offerHash(t)));
        bytes memory newB = sign(B2_PK, digestOf(address(inst), orderHash(t)));
        // the call: the old keys are no party (buyerMayOpen), a stranger neither
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.init(t, quote, newB, newS, new address[](0), new bytes32[](0));
        vm.prank(BUYER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.init(t, quote, newB, newS, new address[](0), new bytes32[](0));
        // the consents: a signature by a key that handed its record on is no consent
        vm.prank(S2);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quote, newB, oldS, new address[](0), new bytes32[](0));
        vm.prank(S2);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.init(t, quote, oldB, newS, new address[](0), new bytes32[](0));
        // the current keys open; parties keeps the recorded addresses
        vm.prank(B2);
        inst.init(t, quote, newB, newS, new address[](0), new bytes32[](0));
        bytes32 id = orderHash(t);
        address[] memory p = inst.seatOf(id, 0).parties;
        assertEq(p[0], BUYER, "the recorded buyerKey");
        assertEq(p[1], SELLER, "the recorded sellerKey");
        assertEq(p[2], SELLER, "the recorded sellerFund");
        assertEq(p[3], BUYER_PAYOUT);
        assertEq(p[4], SELLER_PAYOUT);
        assertEq(depositHeld(inst), depositOf(qa, A), "the deposit pulled from the recorded sellerFund");
    }

    function test_keys_theSellerAloneUnderBuyerMayOpenFalse() public {
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, A);
        t.offer.buyerMayOpen = false;
        fundDeposit(qa, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        moveKey(SELLER, S2);
        bytes memory newS = sign(S2_PK, digestOf(address(logic), offerHash(t)));
        bytes memory b = buyerSig(t);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotSeller.selector);
        inst.init(t, quote, b, newS, new address[](0), new bytes32[](0));
        vm.prank(S2);
        inst.init(t, quote, b, newS, new address[](0), new bytes32[](0));
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
    }

    function test_keys_theExpiryBranchAndCancel() public {
        (Eravon.OrderTerms memory t, Eravon inst) = _order();
        _moveBoth();
        // cancel: the direct form from the buyer's current key alone
        vm.prank(BUYER);
        vm.expectRevert(Eravon.NotBuyer.selector);
        inst.cancel(t, "");
        // the signed form: sent by a current key, signed by the buyer's current key
        bytes memory oldSig = cancelSig(t);
        bytes memory newSig = sign(B2_PK, digestOf(address(inst), keccak256(abi.encode(TH_CANCEL))));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.cancel(t, newSig);
        vm.prank(S2);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.cancel(t, oldSig);
        uint256 s0 = vm.snapshotState();
        vm.prank(S2);
        inst.cancel(t, newSig);
        assertEq(tok.balanceOf(BUYER_PAYOUT), A, "to the recorded buyerPayout");
        vm.revertToStateAndDelete(s0);
        vm.prank(B2);
        inst.cancel(t, "");
        assertEq(tok.balanceOf(BUYER_PAYOUT), A);
        // the expiry branch on another order: the old keys are no party
        (Eravon.OrderTerms memory t2, Eravon inst2) = _order();
        vm.warp(t2.offer.expiry + 1);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst2.init(t2, "", "", "", new address[](0), new bytes32[](0));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst2.init(t2, "", "", "", new address[](0), new bytes32[](0));
        vm.prank(S2);
        inst2.init(t2, "", "", "", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 2 * A);
    }

    // ---- the parties' actions ----

    function test_keys_directForms() public {
        (, Eravon inst, bytes32 id) = _open();
        _moveBoth();
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotSeller.selector);
        inst.claimDelivery(MANIFEST);
        uint256 s0 = vm.snapshotState();
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotSeller.selector);
        inst.refund();
        vm.prank(S2);
        inst.refund();
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded));
        vm.revertToStateAndDelete(s0);
        vm.prank(S2);
        inst.claimDelivery(MANIFEST);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.NotBuyer.selector);
        inst.confirm();
        uint256 s1 = vm.snapshotState();
        vm.prank(B2);
        inst.confirm();
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released));
        assertEq(tok.balanceOf(SELLER_PAYOUT), A, "to the recorded sellerPayout");
        vm.revertToStateAndDelete(s1);
        // the dispute fee leaves the recorded buyerKey's address, whatever key acts for it
        (uint256 lf,,,,) = inst.feeQuoted();
        tok.mint(BUYER, lf);
        vm.prank(BUYER);
        tok.approve(address(inst), lf);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.NotBuyer.selector);
        inst.dispute();
        vm.prank(B2);
        inst.dispute();
        assertEq(tok.balanceOf(BUYER), 0, "the fee from the recorded buyerKey");
        assertEq(inst.seatOf(id, 0).state, 1);
    }

    function test_keys_signedForms() public {
        (, Eravon inst,) = _open();
        // a signature made before the move is refused after it
        bytes memory before = _act(SELLER_PK, inst, keccak256(abi.encode(TH_CLAIM, MANIFEST)));
        _moveBoth();
        vm.prank(S2);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.claimDeliveryBySig(MANIFEST, before);
        bytes memory claimNew = _act(S2_PK, inst, keccak256(abi.encode(TH_CLAIM, MANIFEST)));
        // sent by a key that handed its record on: no party
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.claimDeliveryBySig(MANIFEST, claimNew);
        vm.prank(B2);
        inst.claimDeliveryBySig(MANIFEST, claimNew);
        bytes memory confirmOld = _act(BUYER_PK, inst, keccak256(abi.encode(TH_CONFIRM)));
        bytes memory confirmNew = _act(B2_PK, inst, keccak256(abi.encode(TH_CONFIRM)));
        vm.prank(S2);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.confirmBySig(confirmOld);
        vm.prank(BUYER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.confirmBySig(confirmNew);
        vm.prank(S2);
        inst.confirmBySig(confirmNew);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released));
    }

    function test_keys_settleAndTheConsentRegistry() public {
        (, Eravon inst,) = _open();
        _moveBoth();
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 sh = keccak256(abi.encode(TH_SETTLE, uint256(4000), exp));
        bytes32 d = digestOf(address(inst), sh);
        bytes memory bOld = sign(BUYER_PK, d);
        bytes memory sNew = sign(S2_PK, d);
        bytes memory bNew = sign(B2_PK, d);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.settle(4000, exp, bNew, sNew);
        vm.prank(S2);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.settle(4000, exp, bOld, sNew);
        // the registry: the named party's current key alone registers
        vm.prank(BUYER);
        vm.expectRevert(Eravon.NotCurrent.selector);
        logic.consent(BUYER, d);
        vm.prank(B2);
        vm.expectRevert(Eravon.NotCurrent.selector);
        logic.consent(SELLER, d);
        vm.prank(B2);
        logic.consent(BUYER, d);
        assertTrue(logic.consented(BUYER, d), "registered under the recorded party");
        // the buyer's slot empty, its consent the registry's; the seller's own call carries the seller's
        vm.prank(S2);
        inst.settle(4000, exp, "", "");
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Settled));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 40 ether);
        assertEq(tok.balanceOf(SELLER_PAYOUT), 60 ether);
        // a signed settlement on a second case lands under the current keys' signatures
        (Eravon.OrderTerms memory t2, Eravon inst2) = _order();
        bytes memory os2 = sign(S2_PK, digestOf(address(logic), offerHash(t2)));
        bytes memory ob2 = sign(B2_PK, digestOf(address(inst2), orderHash(t2)));
        vm.prank(S2);
        inst2.init(t2, quote, ob2, os2, new address[](0), new bytes32[](0));
        bytes32 d2 = digestOf(address(inst2), sh);
        bytes memory b2 = sign(B2_PK, d2);
        bytes memory s2 = sign(S2_PK, d2);
        vm.prank(B2);
        inst2.settle(4000, exp, b2, s2);
        assertEq(uint8(phaseOf(inst2)), uint8(Eravon.Phase.Settled));
    }

    function test_keys_revokeFromTheCurrentKeyAlone() public {
        (Eravon.OrderTerms memory t, Eravon inst) = _order();
        moveKey(SELLER, S2);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotCurrent.selector);
        logic.revoke(SELLER, t.offer.termsHash);
        vm.prank(S2);
        logic.revoke(SELLER, t.offer.termsHash);
        assertTrue(logic.revoked(SELLER, t.offer.termsHash), "under the recorded seller");
        // the gate is closed: init runs the expiry branch
        vm.prank(BUYER);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(BUYER_PAYOUT), A);
    }

    function test_keys_sweep() public {
        (, Eravon inst,) = _open();
        _moveBoth();
        vm.prank(S2);
        inst.claimDelivery(MANIFEST);
        vm.prank(B2);
        inst.confirm();
        tok.mint(address(inst), 3 ether);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.sweep(BUYER_PAYOUT, address(tok));
        vm.prank(BUYER);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.sweep(BUYER_PAYOUT, address(tok));
        vm.prank(B2);
        inst.sweep(BUYER_PAYOUT, address(tok));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 3 ether, "to the recorded buyerPayout");
    }

    // ---- the judge's economic face ----

    function test_keys_theJudgesPayoutAndWithdraw() public {
        (, Eravon inst,) = _open();
        vm.prank(SELLER);
        inst.claimDelivery(MANIFEST);
        vm.prank(BUYER);
        inst.confirm();
        address J2 = address(0x1000000000000000000000000000000000000002);
        moveKey(JUDGE, J2);
        vm.prank(JUDGE);
        vm.expectRevert(Eravon.NotCurrent.selector);
        inst.setPayout(JUDGE, JUDGE_PAYOUT);
        vm.prank(JUDGE);
        vm.expectRevert(Eravon.NotCurrent.selector);
        inst.withdraw(address(tok), JUDGE, JUDGE_PAYOUT);
        vm.prank(J2);
        inst.withdraw(address(tok), JUDGE, JUDGE_PAYOUT);
        (address p, uint256 c) = inst.accountOf(JUDGE, address(tok));
        assertEq(p, JUDGE_PAYOUT, "registered under the recorded judge");
        assertEq(c, 0);
        assertEq(tok.balanceOf(JUDGE_PAYOUT), premiumOf(qa, A));
    }

    // ---- the book the code names, and no other ----

    function test_keys_noBookButTheOneTheCodeNames() public {
        (Eravon.OrderTerms memory t, Eravon inst) = _order();
        // the seller's record moves on another copy of the book: nothing changes for the standard
        GaleedSuccessionBook other = new GaleedSuccessionBook();
        vm.prank(SELLER);
        other.declare(S2);
        vm.prank(S2);
        other.accede(SELLER);
        assertEq(other.current(SELLER), S2);
        assertEq(inst.successionBook(), SUCCESSION);
        bytes memory newS = sign(S2_PK, digestOf(address(logic), offerHash(t)));
        bytes memory b = buyerSig(t);
        vm.prank(S2);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.init(t, quote, b, newS, new address[](0), new bytes32[](0));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, quote, b, newS, new address[](0), new bytes32[](0));
        bytes memory s = sellerSig(t);
        vm.prank(SELLER);
        inst.init(t, quote, b, s, new address[](0), new bytes32[](0));
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
        // a stranger stays a stranger whatever book it moves on
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotSeller.selector);
        inst.claimDelivery(MANIFEST);
    }
}
