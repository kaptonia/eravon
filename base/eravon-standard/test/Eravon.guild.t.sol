// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {EravonPanelBook} from "../src/EravonPanelBook.sol";
import {IGaleedVenue} from "../src/IGaleedVenue.sol";
import {Forwarder} from "./replay/VenueReplay.sol";

/// Standard s7: the chain's panel book, the freezing at init (the guild's
/// root, the latest record before init's block that is unexpired), the draw
/// by anyone, the redraw after a declined or expired offer,
/// exhaustion, and the rule that no seat accepted is ever redrawn.
contract EravonGuild is EravonBase {
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    address internal constant J1 = address(uint160(0x001000000000000000000000000000000000000A01));
    address internal constant J2 = address(uint160(0x001000000000000000000000000000000000000A02));
    address internal constant J3 = address(uint160(0x001000000000000000000000000000000000000A03));
    address internal constant STRANGER = address(0x3000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("guild covenant");
    bytes32 internal constant ROSTER = keccak256("roster");
    bytes32 internal constant MANIFEST = keccak256("manifest");
    bytes internal constant ENTRY = "resignation entry";
    bytes32 internal E;
    uint256 internal constant A = 100 ether;

    EravonPanelBook internal book;
    Q internal q;
    bytes internal quote;
    address[] internal members;
    bytes32[] internal words;
    bytes32 internal T;

    function setUp() public {
        setUpBase();
        E = sha256(ENTRY);
        q = defaultQuote(address(tok), address(0), false);
        quote = encodeQuote(q);
        book = deployBook(GUILD);
        members.push(J1);
        members.push(J2);
        members.push(J3);
        words.push(AMANAH);
        words.push(AMANAH);
        words.push(AMANAH);
        T = logic.PROXY_CODEHASH();
    }

    /// The guild records a panel; a case freezes only a record of an earlier block, so the block moves on.
    function engage(address[] memory m, bytes32[] memory w, uint64 exp) internal {
        vm.prank(GUILD, GUILD);
        book.engage(T, m, w, ROSTER, exp, address(0));
        vm.roll(vm.getBlockNumber() + 1);
    }

    function open() internal returns (Eravon inst, Eravon.OrderTerms memory t) {
        t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        inst = openGuild(t, quote, members, words);
    }

    function judgeOf(Eravon inst, bytes32 id, uint64 n) internal view returns (address) {
        return inst.seatOf(id, n).judge;
    }

    function isMember(address who) internal view returns (bool) {
        for (uint256 i = 0; i < members.length; i++) {
            if (members[i] == who) return true;
        }
        return false;
    }

    function acceptSeat(Eravon inst, bytes32 id, uint64 n) internal {
        address j = judgeOf(inst, id, n);
        vm.prank(j, j);
        inst.accept(id, n);
    }

    function declineSeat(Eravon inst, bytes32 id, uint64 n) internal {
        address j = judgeOf(inst, id, n);
        vm.prank(j, j);
        inst.resign(id, n, E, ENTRY);
    }

    function pending(Eravon inst) internal view returns (bool p, uint64 deadline) {
        Eravon.CaseStatus memory _st1 = inst.status();
        p = _st1.pendingRedraw;
        deadline = _st1.redrawDeadline;
    }

    // ---- the freeze ----

    function test_freeze_drawsFromTheLatestRecord() public {
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        bytes32 ph = keccak256(abi.encode(members, words));
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.PanelFrozen(GUILD, GUILD, 0, ph, address(0));
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, words);
        bytes32 id = orderHash(t);
        Eravon.CaseStatus memory _st2 = inst.status();
        uint256 seats0 = _st2.seats;
        bool armed = _st2.pendingRedraw;
        uint64 drawBlock = _st2.drawBlock;
        uint256 index = _st2.panelIndex;
        bytes32 h = _st2.panelHash;
        uint256 size = _st2.panelSize;
        assertEq(seats0, 0, "no seat before the draw");
        assertTrue(armed, "the draw is armed");
        assertEq(drawBlock, uint64(vm.getBlockNumber()), "seeded by this block's hash");
        assertEq(index, 0);
        assertEq(h, ph);
        assertEq(size, 3);
        drawNext(inst, members, words);
        IGaleedVenue.Seat memory s = inst.seatOf(id, 0);
        assertTrue(isMember(s.judge), "the seat is a member");
        assertEq(s.accepted, 0, "an offer, not a consent");
        assertEq(s.windowEnd, uint64(vm.getBlockTimestamp()) + 3 days, "the offer clock");
        assertEq(s.amanahHash, AMANAH);
        assertEq(s.termsHash, t.offer.termsHash);
        // the Opened event names the drawn judge
    }

    function test_freeze_refusals() public {
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        // no record yet: NoPanel
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NoPanel.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, words);
        // a record made in init's own block never reaches the case
        vm.prank(GUILD, GUILD);
        book.engage(T, members, words, ROSTER, uint64(vm.getBlockTimestamp() + 30 days), address(0));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NoPanel.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, words);
        vm.roll(vm.getBlockNumber() + 1);
        // a panel that is not the record the case freezes
        address[] memory two = new address[](2);
        two[0] = J1;
        two[1] = J2;
        bytes32[] memory w2 = new bytes32[](2);
        w2[0] = AMANAH;
        w2[1] = AMANAH;
        vm.prank(SELLER);
        vm.expectRevert(Eravon.PanelMismatch.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), two, w2);
        // a word that is not the order's
        bytes32[] memory other = new bytes32[](3);
        other[0] = keccak256("other");
        other[1] = other[0];
        other[2] = other[0];
        engage(members, other, uint64(vm.getBlockTimestamp() + 30 days));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.PanelWord.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, other);
        // a panel of the parties alone has no pool
        address[] memory parties = new address[](2);
        parties[0] = BUYER;
        parties[1] = SELLER;
        engage(parties, w2, uint64(vm.getBlockTimestamp() + 30 days));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NoMembers.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), parties, w2);
        // the money stayed, and leaves by the expiry branch later, sent by a party
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.None));
        vm.warp(t.offer.expiry + 1);
        vm.expectRevert(Eravon.NotParty.selector);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        vm.prank(BUYER);
        inst.init(t, "", "", "", new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(BUYER_PAYOUT), A);
    }

    /// s7: the latest record recorded before init's block that is unexpired at it; an expired latest record gives
    /// way to an older one still unexpired, and with every record expired no case opens.
    function test_freeze_expiredLatestGivesWayToAnOlder() public {
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        address[] memory two = new address[](2);
        two[0] = J1;
        two[1] = J2;
        bytes32[] memory w2 = new bytes32[](2);
        w2[0] = AMANAH;
        w2[1] = AMANAH;
        uint64 exp = uint64(vm.getBlockTimestamp() + 12 hours);
        engage(two, w2, exp);
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        // at the second `expires` names the latest record still serves
        vm.warp(exp);
        uint256 snap = vm.snapshotState();
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), two, w2);
        assertEq(inst.status().panelIndex, 1, "the latest record, at its expires second");
        vm.revertToState(snap);
        // one second past it the older record is the case's
        vm.warp(exp + 1);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.PanelMismatch.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), two, w2);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, words);
        assertEq(inst.status().panelIndex, 0, "the older record, unexpired");
        // a guild whose every record expired opens no case
        address g3 = address(0x6000000000000000000000000000000000000003);
        uint64 exp3 = uint64(vm.getBlockTimestamp() + 1 hours);
        vm.prank(g3, g3);
        book.engage(T, members, words, ROSTER, exp3, address(0));
        vm.roll(vm.getBlockNumber() + 1);
        vm.warp(exp3 + 1);
        Eravon.OrderTerms memory t3 = guildTerms(g3, AMANAH, quote, A);
        t3.offer.expiry = vm.getBlockTimestamp() + 30 days;
        t3.openBy = uint64(vm.getBlockTimestamp() + 1 days);
        fundDeposit(q, A);
        Eravon inst3 = deployProxy(t3);
        tok.mint(address(inst3), A);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NoPanel.selector);
        inst3.init(t3, quote, buyerSig(t3), sellerSig(t3), members, words);
    }

    function test_freeze_bookMissingOrImpostor() public {
        address g2 = address(0x6000000000000000000000000000000000000002);
        Eravon.OrderTerms memory t = guildTerms(g2, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        // a guild that recorded nothing on the chain's book
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NoPanel.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, words);
        // no code at the book's address, and code there that is not the book's
        bytes memory code = address(book).code;
        vm.etch(address(book), hex"");
        vm.prank(SELLER);
        vm.expectRevert(Eravon.PanelBookMissing.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, words);
        vm.etch(address(book), hex"6000");
        vm.prank(SELLER);
        vm.expectRevert(Eravon.PanelBookMissing.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, words);
        vm.etch(address(book), code);
        // the instance's own engage is no panel face
        vm.expectRevert(Eravon.NoPanelFace.selector);
        inst.engage(T, members, words, ROSTER, uint64(vm.getBlockTimestamp() + 1), address(0));
        assertEq(inst.panelInitCodeHash(), logic.panelInitCodeHash());
        assertEq(inst.panelBook(), address(book));
    }

    function test_freeze_partiesInThePanelAreNotDrawn() public {
        // a panel whose members include the buyer: the pool is the others
        address[] memory m = new address[](3);
        m[0] = BUYER < J1 ? BUYER : J1;
        m[1] = BUYER < J1 ? J1 : BUYER;
        m[2] = J2;
        engage(m, words, uint64(vm.getBlockTimestamp() + 30 days));
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = openGuild(t, quote, m, words);
        Eravon.CaseStatus memory _st3 = inst.status();
        uint256 size = _st3.panelSize;
        assertEq(size, 2, "the buyer is not in the pool");
        bytes32 id = orderHash(t);
        address j0 = judgeOf(inst, id, 0);
        assertTrue(j0 == J1 || j0 == J2);
        declineSeat(inst, id, 0);
        drawNext(inst, m, words);
        address j1 = judgeOf(inst, id, 1);
        assertTrue(j1 != j0 && (j1 == J1 || j1 == J2), "the other member");
        declineSeat(inst, id, 1);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned), "two seats exhaust a pool of two");
    }

    // ---- decline, expiry, redraw, exhaustion ----

    function test_decline_thenRedraw() public {
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst, Eravon.OrderTerms memory t) = open();
        bytes32 id = orderHash(t);
        address j0 = judgeOf(inst, id, 0);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Resigned(E, 0);
        declineSeat(inst, id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded), "the case stays open");
        (bool p, uint64 deadline) = pending(inst);
        assertTrue(p);
        assertEq(deadline, uint64(vm.getBlockTimestamp()) + 3 days, "offerWindow from the decline");
        // the wrong panel
        address[] memory two = new address[](2);
        two[0] = J1;
        two[1] = J2;
        bytes32[] memory w2 = new bytes32[](2);
        w2[0] = AMANAH;
        w2[1] = AMANAH;
        vm.prank(SELLER);
        vm.expectRevert(Eravon.PanelMismatch.selector);
        inst.redraw(two, w2);
        // the block of the arming cannot draw
        vm.prank(SELLER);
        vm.expectRevert(Eravon.DrawPending.selector);
        inst.redraw(members, words);
        // anyone with the frozen panel, from the next block: a stranger's draw seats the member the seed names, the
        // same one the guild's would (s7: the answer is the seed's and the panel's, whoever sends it)
        vm.roll(vm.getBlockNumber() + 1);
        uint256 snap = vm.snapshotState();
        vm.expectEmit(false, true, false, true, address(inst));
        emit IGaleedVenue.Seated(address(0), id, 1);
        vm.prank(STRANGER);
        inst.redraw(members, words);
        address byStranger = judgeOf(inst, id, 1);
        vm.revertToStateAndDelete(snap);
        vm.expectEmit(false, true, false, true, address(inst));
        emit IGaleedVenue.Seated(address(0), id, 1);
        vm.prank(GUILD);
        inst.redraw(members, words);
        assertEq(judgeOf(inst, id, 1), byStranger, "the same member whoever lands the draw");
        (p,) = pending(inst);
        assertFalse(p);
        address j1 = judgeOf(inst, id, 1);
        assertTrue(j1 != j0 && isMember(j1), "a member not seated before");
        Eravon.CaseStatus memory _st4 = inst.status();
        uint256 seats = _st4.seats;
        assertEq(seats, 2);
        // a second redraw with nothing pending
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotPendingRedraw.selector);
        inst.redraw(members, words);
        // the declined seat is closed to every act
        vm.prank(j0, j0);
        vm.expectRevert(Eravon.NotOpen.selector);
        inst.accept(id, 0);
        // the new seat accepts and the case runs as any other
        acceptSeat(inst, id, 1);
        claim(inst, MANIFEST);
        vm.prank(BUYER);
        inst.confirm();
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Released));
        (, uint256 c) = inst.accountOf(j1, address(tok));
        assertEq(c, 1 ether, "P to the judge who finished");
        (, uint256 c0) = inst.accountOf(j0, address(tok));
        assertEq(c0, 0);
    }

    function test_expiredOffer_thenRedraw() public {
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst, Eravon.OrderTerms memory t) = open();
        bytes32 id = orderHash(t);
        uint64 end = inst.seatOf(id, 0).windowEnd;
        vm.warp(end + 1);
        inst.advance(id, 0);
        assertEq(inst.seatOf(id, 0).state, 5, "closed");
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
        (bool p, uint64 deadline) = pending(inst);
        assertTrue(p);
        assertEq(deadline, uint64(vm.getBlockTimestamp()) + 3 days);
        drawNext(inst, members, words);
        assertEq(inst.seatOf(id, 1).windowEnd, uint64(vm.getBlockTimestamp()) + 3 days, "a fresh offer clock");
    }

    function test_nobodyRedraws_theCaseCloses() public {
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst, Eravon.OrderTerms memory t) = open();
        bytes32 id = orderHash(t);
        declineSeat(inst, id, 0);
        (, uint64 deadline) = pending(inst);
        vm.warp(deadline);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded), "the deadline's own second is inside");
        vm.warp(deadline + 1);
        // the block after the arming still draws whatever the clock; the deadline governs from the second on
        vm.roll(vm.getBlockNumber() + 2);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotPendingRedraw.selector);
        inst.redraw(members, words);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Resigned(bytes32(0), 3);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Closed(4, 100 ether, 11 ether, 0, 0);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 100 ether);
        assertEq(tok.balanceOf(SELLER), 11 ether);
    }

    function test_exhaustion_threeSeats() public {
        address[] memory five = new address[](5);
        five[0] = J1;
        five[1] = J2;
        five[2] = J3;
        five[3] = address(uint160(0x001000000000000000000000000000000000000A04));
        five[4] = address(uint160(0x001000000000000000000000000000000000000A05));
        bytes32[] memory w5 = new bytes32[](5);
        for (uint256 i = 0; i < 5; i++) {
            w5[i] = AMANAH;
        }
        engage(five, w5, uint64(vm.getBlockTimestamp() + 30 days));
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = openGuild(t, quote, five, w5);
        bytes32 id = orderHash(t);
        declineSeat(inst, id, 0);
        drawNext(inst, five, w5);
        declineSeat(inst, id, 1);
        drawNext(inst, five, w5);
        // the third decline ends the case in the same transaction: two Resigned logs
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Resigned(E, 0);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Resigned(bytes32(0), 3);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Closed(4, 100 ether, 11 ether, 0, 0);
        declineSeat(inst, id, 2);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned));
        (bool p,) = pending(inst);
        assertFalse(p);
        address a = judgeOf(inst, id, 0);
        address b = judgeOf(inst, id, 1);
        address c = judgeOf(inst, id, 2);
        assertTrue(a != b && b != c && a != c, "three distinct members");
    }

    function test_exhaustion_poolOfTwo() public {
        address[] memory two = new address[](2);
        two[0] = J1;
        two[1] = J2;
        bytes32[] memory w2 = new bytes32[](2);
        w2[0] = AMANAH;
        w2[1] = AMANAH;
        engage(two, w2, uint64(vm.getBlockTimestamp() + 30 days));
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = openGuild(t, quote, two, w2);
        bytes32 id = orderHash(t);
        // first by expiry, second by decline
        vm.warp(inst.seatOf(id, 0).windowEnd + 1);
        inst.advance(id, 0);
        drawNext(inst, two, w2);
        declineSeat(inst, id, 1);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned));
    }

    // ---- the draw's own block, deadline and window ----

    function test_draw_firstSeatWaitsForTheNextBlock() public {
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        bytes32 id = orderHash(t);
        // Opened names no judge on the guild track
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.DrawArmed(uint64(vm.getBlockNumber()), uint64(vm.getBlockTimestamp()) + 3 days);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Opened(id, SELLER, BUYER, address(0), A, t.offer.termsHash);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, words);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
        vm.expectRevert(Eravon.UnknownSeat.selector);
        inst.seatOf(id, 0);
        vm.prank(J1, J1);
        vm.expectRevert(Eravon.UnknownSeat.selector);
        inst.accept(id, 0);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.DrawPending.selector);
        inst.redraw(members, words);
        // the next block draws; the seat's clock starts at the draw
        vm.roll(vm.getBlockNumber() + 1);
        vm.warp(vm.getBlockTimestamp() + 1 hours);
        vm.expectEmit(false, true, false, true, address(inst));
        emit IGaleedVenue.Seated(address(0), id, 0);
        vm.prank(GUILD);
        inst.redraw(members, words);
        IGaleedVenue.Seat memory s = inst.seatOf(id, 0);
        assertTrue(isMember(s.judge));
        assertEq(s.windowStart, uint64(vm.getBlockTimestamp()));
        assertEq(s.windowEnd, uint64(vm.getBlockTimestamp()) + 3 days);
        (bool p,) = pending(inst);
        assertFalse(p);
    }

    function test_draw_seedIsTheArmedBlocksHash() public {
        // two cases armed in the same block with the same panel draw by their own ids: the seed is shared, the
        // id is not; and the drawn member is the one the formula names
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst, Eravon.OrderTerms memory t) = open();
        bytes32 id = orderHash(t);
        Eravon.CaseStatus memory _st5 = inst.status();
        uint64 drawBlock = _st5.drawBlock;
        bytes32 seed = blockhash(drawBlock);
        uint256 pick = uint256(keccak256(abi.encodePacked(id, seed, uint64(0)))) % 3;
        assertEq(judgeOf(inst, id, 0), members[pick], "the formula's member");
    }

    function test_draw_nobodyDrawsTheFirstSeat() public {
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        bytes32 id = orderHash(t);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, words);
        (, uint64 deadline) = pending(inst);
        vm.roll(vm.getBlockNumber() + 1);
        vm.warp(deadline);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded), "the deadline's own second is inside");
        vm.warp(deadline + 1);
        vm.roll(vm.getBlockNumber() + 1);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotPendingRedraw.selector);
        inst.redraw(members, words);
        // the clock runs from any seat index, an unopened one included
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Resigned(bytes32(0), 3);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Closed(4, 100 ether, 11 ether, 0, 0);
        inst.advance(id, 7);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned));
        assertEq(tok.balanceOf(BUYER_PAYOUT), 100 ether);
        assertEq(tok.balanceOf(SELLER), 11 ether);
    }

    function test_draw_staleSeedClosesTheCase() public {
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, words);
        // 256 blocks on the hash is still served and the draw lands
        uint256 armed = vm.getBlockNumber();
        uint256 snap = vm.snapshotState();
        vm.roll(armed + 256);
        vm.prank(SELLER);
        inst.redraw(members, words);
        assertTrue(isMember(judgeOf(inst, orderHash(t), 0)), "the last served block draws");
        vm.revertToState(snap);
        // 257 blocks on the hash is gone: the draw closes the case, a seed nobody drew on buys no second one
        vm.roll(armed + 257);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Resigned(bytes32(0), 3);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Closed(4, 100 ether, 11 ether, 0, 0);
        vm.prank(SELLER);
        inst.redraw(members, words);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned));
        (bool p,) = pending(inst);
        assertFalse(p);
        assertEq(tok.balanceOf(BUYER_PAYOUT), 100 ether);
        assertEq(tok.balanceOf(SELLER), 11 ether);
    }

    function test_draw_staleSeedClosedByAdvance() public {
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        bytes32 id = orderHash(t);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, words);
        uint256 armed = vm.getBlockNumber();
        vm.roll(armed + 256);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded), "the seed still served, the deadline not reached");
        vm.roll(armed + 257);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Resigned(bytes32(0), 3);
        inst.advance(id, 3);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned), "advance closes a draw whose seed is gone");
    }

    function test_draw_firstDrawableBlockDrawsWhateverTheClock() public {
        // a guild quote with offerWindow 0 under autoAccept: the block after the arming still draws
        Q memory q0 = defaultQuote(address(tok), address(0), true);
        q0.offerWindow = 0;
        bytes memory quote0 = encodeQuote(q0);
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote0, A);
        fundDeposit(q0, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        bytes32 id = orderHash(t);
        vm.prank(SELLER);
        inst.init(t, quote0, buyerSig(t), sellerSig(t), members, words);
        (, uint64 deadline) = pending(inst);
        assertEq(deadline, uint64(vm.getBlockTimestamp()), "the deadline is the arming second");
        uint256 snap = vm.snapshotState();
        vm.roll(vm.getBlockNumber() + 1);
        vm.warp(vm.getBlockTimestamp() + 12);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded), "advance closes nothing in the first drawable block");
        vm.prank(SELLER);
        inst.redraw(members, words);
        assertTrue(isMember(judgeOf(inst, id, 0)), "drawn past the deadline in the first drawable block");
        assertEq(inst.seatOf(id, 0).accepted, uint64(vm.getBlockTimestamp()), "consent carried");
        vm.revertToState(snap);
        vm.roll(vm.getBlockNumber() + 2);
        vm.warp(vm.getBlockTimestamp() + 24);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotPendingRedraw.selector);
        inst.redraw(members, words);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned), "from the second block on the deadline governs");
    }

    function test_draw_settleAndRefundBeforeAnySeat() public {
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        // settle with no seat: P back to the seller
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, words);
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 d = keccak256(abi.encode(TH_SETTLE, uint256(5000), exp));
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Closed(2, 50 ether, 61 ether, 0, 0);
        vm.prank(BUYER);
        inst.settle(5000, exp, sign(BUYER_PK, digestOf(address(inst), d)), sign(SELLER_PK, digestOf(address(inst), d)));
        (bool p,) = pending(inst);
        assertFalse(p);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotPendingRedraw.selector);
        inst.redraw(members, words);
        // refund with no seat
        Eravon.OrderTerms memory t2 = guildTerms(GUILD, AMANAH, quote, A);
        t2.offer.nonce = 77;
        fundDeposit(q, A);
        Eravon inst2 = deployProxy(t2);
        tok.mint(address(inst2), A);
        vm.prank(SELLER);
        inst2.init(t2, quote, buyerSig(t2), sellerSig(t2), members, words);
        vm.expectEmit(true, true, true, true, address(inst2));
        emit Eravon.Closed(0, 100 ether, 11 ether, 0, 0);
        vm.prank(SELLER);
        inst2.refund();
        assertEq(uint8(phaseOf(inst2)), uint8(Eravon.Phase.Refunded));
    }

    // ---- no redraw after acceptance ----

    function test_acceptedSeat_isNeverRedrawn() public {
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        // resignation while Funded ends the case
        (Eravon inst, Eravon.OrderTerms memory t) = open();
        bytes32 id = orderHash(t);
        acceptSeat(inst, id, 0);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Resigned(E, 1);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Closed(4, 100 ether, 11 ether, 0, 0);
        declineSeat(inst, id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned));
        (bool p,) = pending(inst);
        assertFalse(p);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotPendingRedraw.selector);
        inst.redraw(members, words);
        // resignation while Claimed: the case runs on with no seat, P back to the seller
        (Eravon inst2, Eravon.OrderTerms memory t2) = open();
        bytes32 id2 = orderHash(t2);
        acceptSeat(inst2, id2, 0);
        claim(inst2, MANIFEST);
        declineSeat(inst2, id2, 0);
        assertEq(uint8(phaseOf(inst2)), uint8(Eravon.Phase.Claimed));
        (p,) = pending(inst2);
        assertFalse(p);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotPendingRedraw.selector);
        inst2.redraw(members, words);
        vm.expectEmit(true, true, true, true, address(inst2));
        emit Eravon.Closed(1, 0, 111 ether, 0, 0);
        vm.prank(BUYER);
        inst2.confirm();
        // T1 expiry on an accepted seat refunds, no redraw
        (Eravon inst3, Eravon.OrderTerms memory t3) = open();
        bytes32 id3 = orderHash(t3);
        acceptSeat(inst3, id3, 0);
        vm.warp(inst3.seatOf(id3, 0).windowEnd + 1);
        vm.expectEmit(true, true, true, true, address(inst3));
        emit Eravon.Refunded(0);
        inst3.advance(id3, 0);
        assertEq(uint8(phaseOf(inst3)), uint8(Eravon.Phase.Refunded));
    }

    function test_frozenPanelServesToTheEnd() public {
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst, Eravon.OrderTerms memory t) = open();
        bytes32 id = orderHash(t);
        // the guild records another panel afterwards
        address[] memory two = new address[](2);
        two[0] = J1;
        two[1] = J2;
        bytes32[] memory w2 = new bytes32[](2);
        w2[0] = AMANAH;
        w2[1] = AMANAH;
        engage(two, w2, uint64(vm.getBlockTimestamp() + 30 days));
        assertEq(book.count(GUILD, T), 2);
        declineSeat(inst, id, 0);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.PanelMismatch.selector);
        inst.redraw(two, w2);
        drawNext(inst, members, words);
        assertTrue(isMember(judgeOf(inst, id, 1)));
        // the record's later expiry does not touch the case either
        vm.warp(vm.getBlockTimestamp() + 60 days);
        declineSeat(inst, id, 1);
        drawNext(inst, members, words);
        Eravon.CaseStatus memory _st6 = inst.status();
        uint256 seats = _st6.seats;
        assertEq(seats, 3);
    }

    function test_sellerRefundAndSettleClearAPendingRedraw() public {
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst, Eravon.OrderTerms memory t) = open();
        bytes32 id = orderHash(t);
        declineSeat(inst, id, 0);
        vm.prank(SELLER);
        inst.refund();
        (bool p,) = pending(inst);
        assertFalse(p);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded));
        (Eravon inst2, Eravon.OrderTerms memory t2) = open();
        declineSeat(inst2, orderHash(t2), 0);
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 d = keccak256(abi.encode(TH_SETTLE, uint256(5000), exp));
        // P back to the seller: the seat resigned
        vm.expectEmit(true, true, true, true, address(inst2));
        emit Eravon.Closed(2, 50 ether, 61 ether, 0, 0);
        vm.prank(BUYER);
        inst2.settle(5000, exp, sign(BUYER_PK, digestOf(address(inst2), d)), sign(SELLER_PK, digestOf(address(inst2), d)));
    }

    function test_guildJudgeRegistersItsOwnPayout() public {
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst, Eravon.OrderTerms memory t) = open();
        bytes32 id = orderHash(t);
        address j = judgeOf(inst, id, 0);
        acceptSeat(inst, id, 0);
        claim(inst, MANIFEST);
        raise(inst);
        vm.prank(j, j);
        inst.rule(id, 0, E, 1, ENTRY, "");
        (address p, uint256 c) = inst.accountOf(j, address(tok));
        assertEq(p, address(0), "no quoted payout on the guild track");
        assertEq(c, 11 ether);
        vm.prank(j);
        vm.expectRevert(Eravon.NoPayout.selector);
        inst.withdraw(address(tok), j, address(0));
        vm.prank(j);
        inst.setPayout(j, address(0x7A9));
        vm.prank(j);
        inst.sweep(j, address(tok));
        assertEq(tok.balanceOf(address(0x7A9)), 11 ether);
    }

    // ---- the book ----

    function test_book_rejectionsAndViews() public {
        uint64 future = uint64(vm.getBlockTimestamp() + 1);
        bytes32[] memory w2 = new bytes32[](2);
        w2[0] = AMANAH;
        w2[1] = AMANAH;
        address[] memory two = new address[](2);
        two[0] = J1;
        two[1] = J2;
        // the six rejections, in order, after the current-key test
        vm.expectRevert(EravonPanelBook.LengthMismatch.selector);
        book.engage(T, two, words, ROSTER, future, address(0));
        vm.expectRevert(EravonPanelBook.EmptyMembers.selector);
        book.engage(T, new address[](0), new bytes32[](0), ROSTER, future, address(0));
        bytes32[] memory zero = new bytes32[](2);
        zero[0] = AMANAH;
        vm.expectRevert(EravonPanelBook.ZeroWord.selector);
        book.engage(T, two, zero, ROSTER, future, address(0));
        bytes32[] memory differ = new bytes32[](2);
        differ[0] = AMANAH;
        differ[1] = keccak256("other");
        vm.expectRevert(EravonPanelBook.WordsDiffer.selector);
        book.engage(T, two, differ, ROSTER, future, address(0));
        vm.expectRevert(EravonPanelBook.ExpiresNotInFuture.selector);
        book.engage(T, two, w2, ROSTER, uint64(vm.getBlockTimestamp()), address(0));
        vm.expectRevert(EravonPanelBook.RouterNoCode.selector);
        book.engage(T, two, w2, ROSTER, future, address(0xdead));
        vm.expectRevert(EravonPanelBook.NoSuchIndex.selector);
        book.panelAt(GUILD, T, 0);
        assertEq(book.count(GUILD, T), 0);
        // a panel may list a member twice, in any order: the book records it as sent
        address[] memory dup = new address[](2);
        dup[0] = J2;
        dup[1] = J2;
        vm.prank(GUILD, GUILD);
        book.engage(T, dup, w2, ROSTER, future, address(0));
        // a record under (root, termsHash) with its hash, roster, expiry, block and router
        vm.expectEmit(true, true, true, true, address(book));
        emit EravonPanelBook.Engaged(GUILD, T, 1, GUILD, two, w2, ROSTER, future, address(0), bytes32(0));
        vm.prank(GUILD, GUILD);
        book.engage(T, two, w2, ROSTER, future, address(0));
        (bytes32 ph, bytes32 rh, uint64 exp, uint64 recorded, address router, bytes32 routerHash) = book.panelAt(GUILD, T, 1);
        assertEq(ph, keccak256(abi.encode(two, w2)));
        assertEq(rh, ROSTER);
        assertEq(exp, future);
        assertEq(recorded, uint64(vm.getBlockNumber()));
        assertEq(router, address(0));
        assertEq(routerHash, bytes32(0));
        vm.prank(STRANGER);
        book.engage(T, two, w2, ROSTER, future, address(0));
        assertEq(book.count(GUILD, T), 2, "a stranger's record is under its own root");
        assertEq(book.count(STRANGER, T), 1);
        // a record under another termsHash is not the standard's
        vm.prank(GUILD, GUILD);
        book.engage(keccak256("other venue"), two, w2, ROSTER, future, address(0));
        assertEq(book.count(GUILD, T), 2);
        // a router with code is recorded with its code hash
        vm.prank(GUILD, GUILD);
        book.engage(T, two, w2, ROSTER, future, address(tok));
        (,,,, router, routerHash) = book.panelAt(GUILD, T, 2);
        assertEq(router, address(tok));
        assertEq(routerHash, address(tok).codehash);
        // the guild moves its key: the former key's engage is refused, the current key's filed under the root
        address g2 = address(0x6000000000000000000000000000000000000009);
        moveKey(GUILD, g2);
        vm.prank(GUILD, GUILD);
        vm.expectRevert(EravonPanelBook.NotCurrent.selector);
        book.engage(T, two, w2, ROSTER, future, address(0));
        vm.prank(g2, g2);
        book.engage(T, two, w2, ROSTER, future, address(0));
        assertEq(book.count(GUILD, T), 4, "filed under the guild's root");
        assertEq(book.count(g2, T), 0);
        // the book names the instances' succession book and route gas, and stands where the logic designates it
        assertEq(book.successionBook(), SUCCESSION);
        assertEq(book.routeGas(), logic.routeGas());
        assertEq(address(book), logic.panelBook());
        assertEq(keccak256(type(EravonPanelBook).creationCode), logic.panelInitCodeHash());
    }

    function test_acceptGuards() public {
        engage(members, words, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst, Eravon.OrderTerms memory t) = open();
        bytes32 id = orderHash(t);
        address j = judgeOf(inst, id, 0);
        address other = j == J1 ? J2 : J1;
        vm.prank(other, other);
        vm.expectRevert(Eravon.NotDirect.selector);
        inst.accept(id, 0);
        Forwarder f = new Forwarder();
        vm.prank(j, j);
        vm.expectRevert(Eravon.NotDirect.selector);
        f.forward(address(inst), abi.encodeCall(inst.accept, (id, 0)));
        vm.prank(j, j);
        vm.expectRevert(Eravon.UnknownSeat.selector);
        inst.accept(id, 1);
        acceptSeat(inst, id, 0);
        assertEq(inst.seatOf(id, 0).accepted, uint64(vm.getBlockTimestamp()));
    }
}
