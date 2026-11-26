// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test, Vm} from "forge-std/Test.sol";
import {VmSafe} from "forge-std/Vm.sol";
import {IGaleedVenue, RouteView} from "../../src/IGaleedVenue.sol";
import {MockERC20} from "./MockERC20.sol";
import {GaleedSuccessionBook} from "./GaleedSuccessionBook.sol";
import {
    FixedRouter, FirstEligibleRouter, FirstEligibleRouterB, RevertRouter, HogRouter, EchoRouter, GasProbeRouter, GasGateRouter,
    SecondFailsRouter, SecondOutOfRangeRouter, LongReturnRouter, ShortReturnRouter, WriterRouter, FieldsRouter, FieldsRouterB,
    NthFailsRouter, NthOutOfRangeRouter, ParityRouter
} from "./ReplayRouters.sol";

/// A contract that forwards a call to the venue, so that "from J through a
/// contract" can be tested: msg.sender at the venue is this contract.
contract Forwarder {
    function forward(address target, bytes calldata data) external {
        (bool ok, bytes memory ret) = target.call(data);
        if (!ok) {
            assembly { revert(add(ret, 32), mload(ret)) }
        }
    }
}

/// A contract whose constructor sends one call and keeps whether it was
/// taken: during construction its address carries no code and is not
/// tx.origin, so a venue that tests code alone takes it (s2.9).
contract ConstructorCall {
    bool public taken;
    constructor(address target, bytes memory data) {
        (taken, ) = target.call(data);
    }
}

/// galeed.venue/1 §8: the conformance replay, as an abstract Foundry test.
/// A venue completes it by implementing the hooks below in a concrete test
/// contract; the replay reads only the interface. Every assertion names the
/// spec line it tests. The harness is tooling: it measures, it never binds.
abstract contract VenueReplay is Test {
    // ---- fixed actors and words ----
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE3 = address(0x1000000000000000000000000000000000000003);
    address internal constant JUDGE4 = address(0x1000000000000000000000000000000000000004);
    address internal constant JUDGE5 = address(0x1000000000000000000000000000000000000005);
    address internal constant GUILD = address(0x2000000000000000000000000000000000000001);
    address internal constant STRANGER = address(0x3000000000000000000000000000000000000001);
    address internal constant IMPOSTOR = address(0x3000000000000000000000000000000000000002);
    address internal constant PAYOUT = address(0x4000000000000000000000000000000000000001);
    address internal constant PAYOUT2 = address(0x4000000000000000000000000000000000000002);
    address internal constant Q1 = address(0x4000000000000000000000000000000000000003);
    address internal constant Q2 = address(0x4000000000000000000000000000000000000004);
    // two recorded addresses with no payout, whose keys step 13 moves
    address internal constant MOVER = address(0x6000000000000000000000000000000000000001);
    address internal constant MOVER2 = address(0x6000000000000000000000000000000000000002);
    bytes32 internal constant AMANAH = keccak256("amanah word of the judge");
    bytes32 internal constant ROSTER = keccak256("roster document");
    bytes32 internal constant M1 = keccak256("manifest one");
    bytes32 internal constant M2 = keccak256("manifest two");
    bytes32 internal constant M3 = keccak256("manifest three");
    bytes32 internal constant M0 = keccak256("manifest before acceptance");
    bytes32 internal constant M4 = keccak256("manifest through a contract");
    bytes32 internal constant M5 = keccak256("manifest after a redraw");
    bytes internal constant N6 = "resignation during a dispute";
    bytes32 internal E6;
    // the entry bytes the rulings and resignations carry (s2.9, s2.10), and
    // their entry_ids: sha256 of the bytes, as galeed/1 s2.1 has it
    bytes internal constant N1 = "judgment entry";
    bytes internal constant N3 = "resignation entry";
    bytes internal constant N5 = "declined offer entry";
    bytes internal constant N7 = "judgment entry from the moved key";
    bytes internal constant N8 = "resignation entry from the moved key";
    bytes internal constant N9 = "declined offer entry from the moved key";
    bytes internal constant NX = "bytes of some other entry";
    bytes32 internal E1;
    bytes32 internal E3;
    bytes32 internal E5;
    bytes32 internal E7;
    bytes32 internal E8;
    bytes32 internal E9;

    bytes32 internal constant SEATED_SIG = keccak256("Seated(address,bytes32,uint64)");
    bytes32 internal constant COMMITTED_SIG = keccak256("Committed(bytes32,address,bytes32)");
    bytes32 internal constant ANCHORED_SIG = keccak256("Anchored(address,bytes32,bytes)");

    MockERC20 internal token;
    IGaleedVenue internal venue;
    Forwarder internal fwd;
    /// the succession book (galeed.roster/1 s4) the harness deploys before the
    /// venue; a venue that completes the hooks names it as `successionBook()`
    GaleedSuccessionBook internal succession;

    // the entryIds and entry bytes of every accepted rule and resign, for step 12
    bytes32[] internal anchoredWords;
    bytes[] internal anchoredEntries;

    /// the number of roles `partyRoles()` names (s2.12)
    uint256 internal roles;

    // ---------------------------------------------------------------------
    // hooks: what the venue supplies
    // ---------------------------------------------------------------------

    /// Deploy the venue. `token` and `succession` are deployed already; a
    /// venue that settles in `token` may read it, and the venue names
    /// `succession` as its succession book. The judge's quote, if any, is the
    /// venue's own.
    function deployVenue() internal virtual returns (IGaleedVenue);

    /// Whether the venue draws judges from an engaged panel (§4). The harness
    /// engages `[JUDGE]` with `[AMANAH]` under `panelTerms()` before any case.
    function venueDraws() internal view virtual returns (bool);

    /// The termsHash the venue's draw path reads a panel under.
    function panelTerms() internal view virtual returns (bytes32) { return keccak256("terms"); }

    /// The venue contract that holds a case: `venue` for a venue that holds
    /// every case in one deployment; for a standard that deploys one venue
    /// per case (one instance per case, one `codeHash`), the instance that
    /// holds `caseId`. Every per-case call and read of the replay goes
    /// through it, and a log counts as the venue's when its emitter is any
    /// venue this hook has named.
    function venueOf(bytes32) internal view virtual returns (IGaleedVenue) { return venue; }

    /// Where the panel face lives (s4): address(0) when the venue holds it
    /// itself; the panel book the venue's standard names otherwise, in which
    /// case the guild's `engage` goes to the book and the venue's own reverts.
    function panelBook() internal view virtual returns (address) { return address(0); }

    /// Whether the seating `openCase()` opens waits for the judge's `accept`
    /// (§2.7) or carries the judge's consent from the Seated block. Consent is
    /// per seating: the replay reads `accepted` on every seat it opens and
    /// takes the branch that seat is in; this hook only says which branch the
    /// venue's ordinary creation path lands in.
    function seatsBeforeAcceptance() internal view virtual returns (bool);

    /// A venue whose ordinary path opens an offer but which also opens
    /// seatings with consent carried (a redraw, a reopened seating) may
    /// return such a seating here so that the consent branch of step 4a is
    /// measured too; (0, 0) means none.
    function openCarriedCase() internal virtual returns (bytes32 caseId, uint64 seat) { return (bytes32(0), 0); }

    /// Whether an open offer runs a clock (windowEnd nonzero while unaccepted).
    function offerClockRuns() internal view virtual returns (bool) { return seatsBeforeAcceptance(); }

    /// Whether the call that closes an offer expired unaccepted opens a new seating.
    function reassignsAfterExpiredOffer() internal view virtual returns (bool) { return false; }

    /// Whether the call that lapses a seat opens a new seating.
    function redrawsAfterLapse() internal view virtual returns (bool) { return false; }

    /// Create a case by the venue's own path and bring it to the point where
    /// JUDGE is seated (in flight). Returns the (caseId, seat) the path opened.
    /// Where the venue draws and no usable panel exists, the path reverts.
    function openCase() internal virtual returns (bytes32 caseId, uint64 seat);

    /// Where the venue draws: whether `openCaseNaming` can open a case whose
    /// terms name the guild by another address of its record.
    function opensCaseNaming() internal view virtual returns (bool) { return false; }

    /// `openCase()` with the terms naming the guild by `guild`.
    function openCaseNaming(address) internal virtual returns (bytes32 caseId, uint64 seat) { return (bytes32(0), 0); }

    /// Bring an accepted seat to `disputed` by the venue's own path, in the
    /// form the venue's documentation names for the dispute `outcomeOf` takes
    /// as made (s2.12, s7 item 17); a no-op where the seat is disputed already.
    function dispute(bytes32 caseId, uint64 seat) internal virtual;

    /// An address the venue counts as a party of the case.
    function party(bytes32 caseId) internal view virtual returns (address);

    /// The `extra` bytes a permitted ruling carries: `abi.encode` of the
    /// values `extraType()` names, empty where that string is empty.
    function rulingExtra(bytes32, uint64) internal view virtual returns (bytes memory) { return ""; }

    /// The token in which JUDGE's fees accrue; the harness token by default.
    function feeToken() internal view virtual returns (address) { return address(token); }

    /// Optional (s2.12): the addresses that stand in role `role` of
    /// `partyRoles()` on the case, as the venue's documentation states them;
    /// none leaves that role's receipts unmeasured.
    function roleAddresses(bytes32, uint256) internal view virtual returns (address[] memory) { return new address[](0); }

    /// Optional (s2.12): the venue's own path by which `who` takes what the
    /// venue holds for it on the case; nothing where the venue transfers.
    function collect(bytes32, address) internal virtual {}

    /// Optional (s2.5): the quote bytes the creation path of `openCase()`
    /// copied into the case, where the venue decodes a quote for its seat.
    function quoteOf(bytes32) internal view virtual returns (bool known, bytes memory quote) { return (false, ""); }

    /// Optional (s2.5, s2.12): whether the seating's ladder is determined. A
    /// seating that stands on a relationship before any claim names a class
    /// reports `rungs` zero and its `outcomeOf` MAY revert; the replay then
    /// reads no outcome on it until the dispute determines the ladder.
    function ladderDetermined(bytes32, uint64) internal view virtual returns (bool) { return true; }

    /// Where the venue's book is a constant, the address its code names: the
    /// replay places the family's book there and moves every key on it. Zero,
    /// the default: the replay deploys the book and hands its address over.
    function bookAt() internal view virtual returns (address) { return address(0); }

    // ---------------------------------------------------------------------

    function setUp() public virtual {
        vm.warp(1_800_000_000);
        E1 = sha256(N1);
        E3 = sha256(N3);
        E5 = sha256(N5);
        E7 = sha256(N7);
        E8 = sha256(N8);
        E9 = sha256(N9);
        E6 = sha256(N6);
        token = new MockERC20();
        succession = new GaleedSuccessionBook();
        if (bookAt() != address(0)) {
            // the family's book, byte for byte, at the venue's constant
            vm.etch(bookAt(), address(succession).code);
            succession = GaleedSuccessionBook(bookAt());
        }
        fwd = new Forwarder();
        // recording starts before the venue exists, so that a Seated a
        // per-case standard emits while deploying its first case is seen
        vm.recordLogs();
        venue = deployVenue();
        _venues.push(address(venue));
    }

    address[] internal _venues;

    /// `openCase()` followed by the registration of the case's venue.
    function _open() internal returns (bytes32 caseId, uint64 seat) {
        (caseId, seat) = openCase();
        _note(venueOf(caseId));
    }

    function _note(IGaleedVenue v) internal {
        for (uint256 i = 0; i < _venues.length; i++) if (_venues[i] == address(v)) return;
        _venues.push(address(v));
        // s8.1 for every venue the replay meets: code, constant type strings
        assertTrue(address(v).codehash != bytes32(0), "s8.1 code");
        assertEq(keccak256(bytes(v.quoteType())), keccak256(bytes(venue.quoteType())), "s8.1 quoteType constant across the standard");
        assertEq(keccak256(bytes(v.extraType())), keccak256(bytes(venue.extraType())), "s8.1 extraType constant across the standard");
        assertEq(keccak256(bytes(v.partyRoles())), keccak256(bytes(venue.partyRoles())), "s8.1 partyRoles constant across the standard");
        assertEq(v.successionBook(), venue.successionBook(), "s1 successionBook constant across the standard");
        assertEq(v.routeGas(), venue.routeGas(), "s4 routeGas constant across the standard");
        _readsNoState(address(v), "a venue of the standard");
    }

    /// s1, s4, s7.16: successionBook() and routeGas() are fixed by the code, a
    /// constant or an immutable: reading them reads no state of any account
    /// (no storage, no balance, no code of another account, and nothing an
    /// account whose code is a delegation designator runs), the venue's own or
    /// one it calls, and they answer alike whatever the balances, the clock
    /// and the block.
    function _readsNoState(address v, string memory what) internal {
        vm.startStateDiffRecording();
        address b0 = IGaleedVenue(v).successionBook();
        uint64 g0 = IGaleedVenue(v).routeGas();
        Vm.AccountAccess[] memory acc = vm.stopAndReturnStateDiff();
        string memory m = string.concat("s1, s7.16 successionBook() and routeGas() read no state of any account: ", what);
        for (uint256 i = 0; i < acc.length; i++) {
            assertEq(acc[i].storageAccesses.length, 0, m);
            VmSafe.AccountAccessKind k = acc[i].kind;
            assertTrue(k != VmSafe.AccountAccessKind.Balance && k != VmSafe.AccountAccessKind.Extcodesize
                && k != VmSafe.AccountAccessKind.Extcodehash && k != VmSafe.AccountAccessKind.Extcodecopy, m);
            bytes memory c = acc[i].account.code;
            assertFalse(c.length == 23 && c[0] == 0xef && c[1] == 0x01 && c[2] == 0x00, m);
        }
        // the same answers whatever the balances, the clock and the block
        (uint256 t0, uint256 n0, uint256 r0, address cb0, uint256 f0) =
            (vm.getBlockTimestamp(), vm.getBlockNumber(), block.prevrandao, block.coinbase, block.basefee);
        uint256 snap = vm.snapshotState();
        vm.deal(v, v.balance + 1);
        for (uint256 i = 0; i < acc.length; i++) vm.deal(acc[i].account, acc[i].account.balance + 1);
        vm.warp(t0 + 100 * 365 days);
        vm.roll(n0 + 100_000_000);
        vm.prevrandao(bytes32(r0 ^ 0x5ec4));
        vm.coinbase(address(0x5ec4));
        vm.fee(f0 + 7 gwei);
        string memory m2 = string.concat("s1, s4 successionBook() and routeGas() answer alike whatever the balances, the clock and the block: ", what);
        assertEq(IGaleedVenue(v).successionBook(), b0, m2);
        assertEq(IGaleedVenue(v).routeGas(), g0, m2);
        vm.revertToStateAndDelete(snap);
        vm.warp(t0);
        vm.roll(n0);
        vm.prevrandao(bytes32(r0));
        vm.coinbase(cb0);
        vm.fee(f0);
    }

    /// `openCase()` as an external call, so that a creation the venue must
    /// refuse can be caught.
    function openForReplay() external returns (bytes32 caseId, uint64 seat) {
        return _open();
    }

    /// `openCaseNaming()` as an external call.
    function openNamingForReplay(address guild) external returns (bytes32 caseId, uint64 seat) {
        (caseId, seat) = openCaseNaming(guild);
        _note(venueOf(caseId));
    }

    function _isVenue(address a) internal view returns (bool) {
        for (uint256 i = 0; i < _venues.length; i++) if (_venues[i] == a) return true;
        return false;
    }

    /// the replay's book, the one step 1 finds the venue names
    function _bk() internal view returns (GaleedSuccessionBook) { return succession; }

    function _roll() internal { vm.roll(vm.getBlockNumber() + 1); }

    function test_replay() public {
        // 1. immutable code, constant type strings, well-formed roles
        assertTrue(address(venue).codehash != bytes32(0), "s8.1 code");
        string memory q1 = venue.quoteType();
        string memory x1 = venue.extraType();
        string memory r1 = venue.partyRoles();
        address bk = venue.successionBook();
        uint64 rg = venue.routeGas();
        _roll();
        assertEq(keccak256(bytes(q1)), keccak256(bytes(venue.quoteType())), "s8.1 quoteType constant");
        assertEq(keccak256(bytes(x1)), keccak256(bytes(venue.extraType())), "s8.1 extraType constant");
        assertEq(keccak256(bytes(r1)), keccak256(bytes(venue.partyRoles())), "s8.1 partyRoles constant");
        bool formed;
        (formed, roles) = _roleWords(r1);
        assertTrue(formed, "s2.12 partyRoles is distinct lowercase words of letters and digits, each beginning with a letter, joined by single commas");
        assertTrue(bk.code.length != 0, "s1 successionBook names a contract");
        assertEq(bk.codehash, address(succession).codehash, "s7.16 successionBook carries the code of galeed.roster/1 s4 the replay deployed");
        assertEq(bk, address(succession), "s1 successionBook names the chain's book, the one the replay deployed and moves every key on");
        assertEq(venue.successionBook(), bk, "s1 successionBook constant");
        assertEq(venue.routeGas(), rg, "s4 routeGas constant");
        // s1, s7.16: both are fixed by the code, so the code at another address reads them alike
        address twin = address(0xfee1000000000000000000000000000000000001);
        vm.etch(twin, address(venue).code);
        assertEq(IGaleedVenue(twin).successionBook(), bk, "s1 successionBook is fixed by the code");
        assertEq(IGaleedVenue(twin).routeGas(), rg, "s4 routeGas is fixed by the code");
        _readsNoState(address(venue), "the venue");

        // 3. the panel face (before step 2, so that the first case can exist before any registration)
        bytes32 T = panelTerms();
        address book = panelBook();
        _panelFace(T, book);
        // s4: a case freezes a panel recorded before its creation block
        _roll();

        // 4. first case: seated
        (bytes32 C, uint64 s) = _open();

        // 2. no payout is registered at the case's creation; registration is one-shot, from the key's current key
        (address p0, ) = venue.accountOf(JUDGE, feeToken());
        assertEq(p0, address(0), "s3 no payout is registered at a case's creation");
        vm.prank(STRANGER, STRANGER);
        _drain(); vm.expectRevert();
        venue.setPayout(JUDGE, STRANGER); vm.getRecordedLogs();
        vm.prank(JUDGE, JUDGE);
        venue.setPayout(JUDGE, PAYOUT);
        (address p, ) = venue.accountOf(JUDGE, feeToken());
        assertEq(p, PAYOUT, "s8.2 payout");
        vm.prank(JUDGE, JUDGE);
        _drain(); vm.expectRevert();
        venue.setPayout(JUDGE, PAYOUT2); vm.getRecordedLogs();
        (address pz, ) = venue.accountOf(STRANGER, feeToken());
        assertEq(pz, address(0), "s3 unset payout reads zero");

        assertEq(_seatedCount(C, s, JUDGE), 1, "s8.4 exactly one Seated");
        IGaleedVenue.Seat memory st = venueOf(C).seatOf(C, s);
        assertEq(st.judge, JUDGE, "s8.4 judge");
        assertEq(st.amanahHash, AMANAH, "s8.4 amanahHash");
        assertTrue(st.termsHash != bytes32(0), "s8.4 termsHash nonzero");
        assertTrue(_inFlight(st.state), "s8.4 in flight");
        if (bytes(q1).length == 0) assertEq(st.quoteHash, bytes32(0), "s2.5 no quote, zero quoteHash");
        {
            (bool qk, bytes memory qb) = quoteOf(C);
            if (qk) assertEq(st.quoteHash, keccak256(qb), "s2.5 the seat freezes keccak256 of the quote bytes it decoded");
        }
        // s2.8: commit is taken from the moment the case exists, an open offer included
        vm.prank(STRANGER, STRANGER);
        venueOf(C).commit(C, M0);
        assertTrue(_committed(C, M0, STRANGER), "s2.8 commit is taken on the seat as opened, before any acceptance");
        _drain(); vm.expectRevert();
        venueOf(C).seatOf(keccak256("no such case"), 0); vm.getRecordedLogs();
        _drain(); vm.expectRevert();
        venueOf(C).advanceOf(keccak256("no such case"), 0); vm.getRecordedLogs();
        _drain(); vm.expectRevert();
        venueOf(C).seatOf(C, s + 1000); vm.getRecordedLogs();
        _drain(); vm.expectRevert();
        venueOf(C).advanceOf(C, s + 1000); vm.getRecordedLogs();
        vm.prank(STRANGER, STRANGER);
        venueOf(C).advance(keccak256("no such case"), 0);
        vm.prank(STRANGER, STRANGER);
        venueOf(C).advance(C, type(uint64).max);
        assertEq(venueOf(C).seatOf(C, s).state, st.state, "s2.11 advance on an unseated pair changes nothing");
        // s2.12: outcomeOf answers on a seat in flight before acceptance and dispute
        bytes memory extra = rulingExtra(C, s);
        _outcomesInRange(C, s, extra, "s2.12 on a seat in flight before acceptance");
        _keepPredictions(C, s, extra);

        // 4a. acceptance, by the seat's own accepted value
        uint256 anchoredBefore = _anchoredCount();
        assertEq(st.accepted == 0, seatsBeforeAcceptance(), "s8.4a the hook names the branch the seat is in");
        if (st.accepted == 0) assertEq(st.windowEnd != 0, offerClockRuns(), "s8.9a offerClockRuns() names whether the offer runs a clock");
        if (st.accepted == 0) {
            assertEq(st.state, 0, "s8.4a seated");
            vm.prank(STRANGER, STRANGER); _drain(); vm.expectRevert();
            venueOf(C).accept(C, s); vm.getRecordedLogs();
            vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
            fwd.forward(address(venueOf(C)), abi.encodeCall(IGaleedVenue.accept, (C, s))); vm.getRecordedLogs();
            _withCode(JUDGE);
            vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
            venueOf(C).accept(C, s); vm.getRecordedLogs();
            _withoutCode(JUDGE);
            Snap memory a0 = _snap(venueOf(C), C, s);
            vm.prank(JUDGE, JUDGE);
            venueOf(C).accept(C, s);
            _creditsCounted(a0, _snap(venueOf(C), C, s), "s7.18 what the acceptance credits is counted on the seating");
            st = venueOf(C).seatOf(C, s);
            assertEq(st.accepted, uint64(vm.getBlockTimestamp()), "s8.4a accepted now");
            assertTrue(_inFlight(st.state), "s8.4a still in flight");
            assertEq(_anchoredCount(), anchoredBefore, "s8.4a no Anchored");
            vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
            venueOf(C).accept(C, s); vm.getRecordedLogs();
            _outcomesInRange(C, s, extra, "s2.12 on an accepted seat before any dispute");
        } else {
            assertTrue(st.accepted != 0, "s8.4a consent carried: accepted nonzero");
            vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
            venueOf(C).accept(C, s); vm.getRecordedLogs();
        }
        (bytes32 Cc, uint64 sc) = openCarriedCase();
        if (Cc != bytes32(0)) _note(venueOf(Cc));
        if (Cc != bytes32(0)) {
            IGaleedVenue.Seat memory cst = venueOf(Cc).seatOf(Cc, sc);
            assertTrue(cst.accepted != 0, "s8.4a carried seating: accepted nonzero");
            assertTrue(_inFlight(cst.state), "s8.4a carried seating in flight");
            vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
            venueOf(Cc).accept(Cc, sc); vm.getRecordedLogs();
        }

        // 5. commitments from a party and a stranger
        vm.prank(party(C), party(C));
        venueOf(C).commit(C, M1);
        vm.prank(STRANGER, STRANGER);
        venueOf(C).commit(C, M2);
        assertTrue(_committed(C, M1, party(C)) && _committed(C, M2, STRANGER), "s8.5 committed set, each naming its sender");
        // s2.8: from any address, a contract included, never gated on identity; `by` is the sender
        vm.prank(STRANGER, STRANGER);
        fwd.forward(address(venueOf(C)), abi.encodeCall(IGaleedVenue.commit, (C, M4)));
        assertTrue(_committed(C, M4, address(fwd)), "s2.8 a commitment sent through a contract is taken and names the contract as by");

        // 6. disputed with a live window
        dispute(C, s);
        st = venueOf(C).seatOf(C, s);
        assertEq(st.state, 1, "s8.6 disputed");
        assertTrue(st.windowEnd != 0 && st.windowEnd >= vm.getBlockTimestamp(), "s8.6 window live");
        assertTrue(st.accepted != 0, "s2.7 disputed implies accepted");
        if (st.rungs > 0) assertTrue(st.rungMin <= st.rungMax && st.rungMax < st.rungs, "s7.11 rung range");
        vm.prank(STRANGER, STRANGER);
        venueOf(C).commit(C, M3);
        assertTrue(_committed(C, M3, STRANGER), "s2.8 commit is taken while a seat is disputed");

        // 7. ruling: the three conditions, the range, one Anchored; 15. what outcomeOf reported is what the ruling does
        // s2.9: where rungs is zero the venue ignores the rung, so the ruling carries one that is not zero
        uint64 rung = st.rungs == 0 ? 7 : st.rungMin;
        extra = rulingExtra(C, s);
        _outcomesInRange(C, s, extra, "s2.12 on a disputed seat");
        vm.prank(STRANGER, STRANGER); _drain(); vm.expectRevert();
        venueOf(C).rule(C, s, E1, rung, N1, extra); vm.getRecordedLogs();
        vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
        fwd.forward(address(venueOf(C)), abi.encodeCall(IGaleedVenue.rule, (C, s, E1, rung, N1, extra))); vm.getRecordedLogs();
        _withCode(JUDGE);
        vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
        venueOf(C).rule(C, s, E1, rung, N1, extra); vm.getRecordedLogs();
        _withoutCode(JUDGE);
        if (st.rungs > 0) {
            vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
            venueOf(C).rule(C, s, E1, st.rungMax + 1, N1, extra); vm.getRecordedLogs();
            if (st.rungMin > 0) {
                vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
                venueOf(C).rule(C, s, E1, st.rungMin - 1, N1, extra); vm.getRecordedLogs();
            }
        }
        if (bytes(x1).length == 0) {
            assertEq(extra.length, 0, "s2.9 empty extraType: the hook supplies no extra");
            vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
            venueOf(C).rule(C, s, E1, rung, N1, hex"01"); vm.getRecordedLogs();
        } else {
            assertTrue(extra.length != 0, "s2.9 named extraType: the hook supplies the encoding");
            vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
            venueOf(C).rule(C, s, E1, rung, N1, ""); vm.getRecordedLogs();
            vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
            venueOf(C).rule(C, s, E1, rung, N1, abi.encodePacked(extra, hex"00")); vm.getRecordedLogs();
            bytes[] memory dirty = _dirtyExtras(x1, extra);
            for (uint256 q = 0; q < dirty.length; q++) {
                vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
                venueOf(C).rule(C, s, E1, rung, N1, dirty[q]); vm.getRecordedLogs();
            }
        }
        // s2.9: the entry bytes must hash to entryId under sha256
        vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
        venueOf(C).rule(C, s, E1, rung, NX, extra); vm.getRecordedLogs();
        vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
        venueOf(C).rule(C, s, E1, rung, "", extra); vm.getRecordedLogs();
        _comparePredictions(C, s, extra);
        // s2.3: a ruling at the window's last second is never refused for time
        vm.warp(st.windowEnd);
        IGaleedVenue.Seat memory pre = venueOf(C).seatOf(C, s);
        _everyRung(C, s, extra);
        IGaleedVenue.Outcome memory o = venueOf(C).outcomeOf(C, s, rung, extra);
        _shape(o, st.amount, "s8.15 the ruling's outcome");
        assertEq(o.state, 2, "s8.15 outcomeOf reports ruled");
        uint256[] memory rb = _received(C, st.token);
        Snap memory sn = _snap(venueOf(C), C, s);
        anchoredBefore = _anchoredCount();
        vm.prank(JUDGE, JUDGE);
        venueOf(C).rule(C, s, E1, rung, N1, extra);
        _sharesPaid(C, st.token, rb, o, "s8.15 the ruling");
        anchoredWords.push(E1);
        anchoredEntries.push(N1);
        assertEq(_anchoredCount(), anchoredBefore + 1, "s8.7 exactly one Anchored");
        assertTrue(_anchoredIs(anchoredBefore, JUDGE, E1, N1), "s8.7 Anchored(J, E, entry)");
        assertEq(venueOf(C).seatOf(C, s).state, o.state, "s8.7 ruled, as outcomeOf reported");
        _moved(sn, _snap(venueOf(C), C, s), o, "s8.15 the ruling");
        _kept(pre, venueOf(C).seatOf(C, s), "s2.3 the ruled seat keeps its window, accepted and range");
        assertTrue(_same(o, venueOf(C).outcomeOf(C, s, rung, extra)), "s8.15 outcomeOf on the ruled seat reports what it reported before");

        // 8. second case: lapse under advance; 15. what advanceOf reported is what the advance does
        (bytes32 C2, uint64 s2) = _open();
        if (venueOf(C2).seatOf(C2, s2).accepted == 0) { vm.prank(JUDGE, JUDGE); venueOf(C2).accept(C2, s2); }
        dispute(C2, s2);
        st = venueOf(C2).seatOf(C2, s2);
        vm.warp(st.windowEnd);
        _nothingDue(C2, s2, "s2.11 nothing due at the window's last second");
        // s2.12: on a seat in flight an advance does what advanceOf reports, whichever of the venue's clocks moves
        _advanceAsReported(C2, s2, "s2.11 an advance at the window's last second");
        assertEq(venueOf(C2).seatOf(C2, s2).state, 1, "s2.11 at the window's last second advance leaves the seat disputed");
        vm.warp(uint256(st.windowEnd) + 1);
        assertEq(venueOf(C2).seatOf(C2, s2).state, 1, "s2.6 past its window and before advance the seat still reads disputed");
        _outcomesInRange(C2, s2, rulingExtra(C2, s2), "s2.12 on a disputed seat past its window");
        IGaleedVenue.Seat memory pre2 = venueOf(C2).seatOf(C2, s2);
        _advanceAsReported(C2, s2, "s8.15 the lapse");
        assertEq(venueOf(C2).seatOf(C2, s2).state, 4, "s8.8 lapsed");
        _kept(pre2, venueOf(C2).seatOf(C2, s2), "s2.3 the lapsed seat keeps its window, accepted and range");
        _nothingDue(C2, s2, "s2.12 nothing due on a lapsed seat");
        // a drawing venue's redraw after a lapse is measured at step 14, against the panel the case froze
        if (redrawsAfterLapse() && !venueDraws()) {
            uint64 sNew = _latestSeat(C2);
            assertTrue(sNew != s2, "s8.8 redraw opens a new ordinal");
            assertTrue(_inFlight(venueOf(C2).seatOf(C2, sNew).state), "s8.8 redrawn seat in flight");
        } else if (!venueDraws()) {
            assertEq(_latestSeat(C2), s2, "s8.8 the venue's hook says it does not redraw after a lapse, and no seat follows");
        }

        // 9. third case: resignation keeps the window and accepted
        (bytes32 C3, uint64 s3) = _open();
        if (venueOf(C3).seatOf(C3, s3).accepted == 0) { vm.prank(JUDGE, JUDGE); venueOf(C3).accept(C3, s3); }
        IGaleedVenue.Seat memory before = venueOf(C3).seatOf(C3, s3);
        anchoredBefore = _anchoredCount();
        vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
        venueOf(C3).resign(C3, s3, E3, NX); vm.getRecordedLogs();
        vm.prank(JUDGE, JUDGE);
        venueOf(C3).resign(C3, s3, E3, N3);
        anchoredWords.push(E3);
        anchoredEntries.push(N3);
        assertEq(_anchoredCount(), anchoredBefore + 1, "s8.9 one Anchored");
        assertTrue(_anchoredIs(anchoredBefore, JUDGE, E3, N3), "s8.9 Anchored(J, E3, entry)");
        st = venueOf(C3).seatOf(C3, s3);
        assertEq(st.state, 3, "s8.9 resigned");
        assertEq(st.windowStart, before.windowStart, "s8.9 window kept");
        assertEq(st.windowEnd, before.windowEnd, "s8.9 window kept");
        assertEq(st.accepted, before.accepted, "s8.9 accepted kept");
        _kept(before, st, "s2.3 the resigned seat keeps its window, accepted and range");
        _nothingDue(C3, s3, "s2.12 nothing due on a resigned seat");
        vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
        venueOf(C3).resign(C3, s3, E3, N3); vm.getRecordedLogs();

        // 9b. a resignation during a dispute, at the ruling window's last second, is never refused for time
        {
            (bytes32 Cd, uint64 sd) = _open();
            if (venueOf(Cd).seatOf(Cd, sd).accepted == 0) { vm.prank(JUDGE, JUDGE); venueOf(Cd).accept(Cd, sd); }
            dispute(Cd, sd);
            IGaleedVenue.Seat memory bd = venueOf(Cd).seatOf(Cd, sd);
            assertEq(bd.state, 1, "s8.9b disputed");
            vm.warp(bd.windowEnd);
            anchoredBefore = _anchoredCount();
            vm.prank(JUDGE, JUDGE);
            venueOf(Cd).resign(Cd, sd, E6, N6);
            anchoredWords.push(E6);
            anchoredEntries.push(N6);
            assertEq(_anchoredCount(), anchoredBefore + 1, "s8.9b one Anchored");
            assertTrue(_anchoredIs(anchoredBefore, JUDGE, E6, N6), "s8.9b Anchored(J, E6, entry)");
            assertEq(venueOf(Cd).seatOf(Cd, sd).state, 3, "s8.9b resigned during the dispute");
            _kept(bd, venueOf(Cd).seatOf(Cd, sd), "s2.10 the seat resigned during a dispute keeps its window, accepted and range");
        }

        // 9a. offers: expiry and decline
        if (seatsBeforeAcceptance()) {
            if (offerClockRuns()) {
                (bytes32 C4, uint64 s4) = _open();
                st = venueOf(C4).seatOf(C4, s4);
                assertEq(st.accepted, 0, "s8.9a unaccepted");
                assertTrue(st.windowEnd != 0, "s8.9a offer clock");
                // s2.7, s2.11: at the offer's last second accept is taken and nothing is due
                vm.warp(st.windowEnd);
                _nothingDue(C4, s4, "s2.11 nothing due at an offer's last second");
                _drain();
                {
                    uint256 sn = vm.snapshotState();
                    vm.prank(JUDGE, JUDGE);
                    venueOf(C4).accept(C4, s4);
                    assertEq(venueOf(C4).seatOf(C4, s4).accepted, st.windowEnd, "s2.7 accept is taken at the offer's last second");
                    vm.revertToStateAndDelete(sn);
                    vm.getRecordedLogs();
                }
                _advanceAsReported(C4, s4, "s2.11 an advance at the offer's last second");
                assertEq(venueOf(C4).seatOf(C4, s4).state, 0, "s2.11 at the offer's last second advance leaves the offer open");
                vm.warp(uint256(st.windowEnd) + 1);
                vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
                venueOf(C4).accept(C4, s4); vm.getRecordedLogs();
                _outcomesInRange(C4, s4, rulingExtra(C4, s4), "s2.12 on an expired offer before advance");
                IGaleedVenue.Seat memory pre4 = venueOf(C4).seatOf(C4, s4);
                _advanceAsReported(C4, s4, "s2.12 the expired offer");
                assertEq(venueOf(C4).seatOf(C4, s4).state, 5, "s8.9a expired offer is closed, never lapsed");
                _kept(pre4, venueOf(C4).seatOf(C4, s4), "s2.3 the closed offer keeps its window, accepted and range");
                _nothingDue(C4, s4, "s2.12 nothing due on a closed seat");
                // a drawing venue's redraw is measured at step 14, against the panel the case froze
                if (reassignsAfterExpiredOffer() && !venueDraws()) {
                    uint64 sNew = _latestSeat(C4);
                    assertTrue(sNew != s4, "s8.9a reassignment opens a new ordinal");
                } else if (!venueDraws()) {
                    assertEq(_latestSeat(C4), s4, "s8.9a the venue's hook says it does not reassign, and no seat follows");
                }
            }
            (bytes32 C5, uint64 s5) = _open();
            IGaleedVenue.Seat memory pre5 = venueOf(C5).seatOf(C5, s5);
            anchoredBefore = _anchoredCount();
            vm.prank(JUDGE, JUDGE);
            venueOf(C5).resign(C5, s5, E5, N5);
            _kept(pre5, venueOf(C5).seatOf(C5, s5), "s2.7, s2.10 the declined offer keeps its window, accepted and range");
            anchoredWords.push(E5);
            anchoredEntries.push(N5);
            assertEq(_anchoredCount(), anchoredBefore + 1, "s8.9a decline: one Anchored");
            assertTrue(_anchoredIs(anchoredBefore, JUDGE, E5, N5), "s8.9a Anchored(J, E5, entry)");
            st = venueOf(C5).seatOf(C5, s5);
            assertEq(st.state, 3, "s8.9a declined offer is resigned");
            assertEq(st.accepted, 0, "s8.9a declined offer keeps accepted zero");
            vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
            venueOf(C5).accept(C5, s5); vm.getRecordedLogs();
        }

        // 10. advance with nothing due; 15. advanceOf reports the present state and zero amounts
        (bytes32 C6, uint64 s6) = _open();
        uint8 s6state = venueOf(C6).seatOf(C6, s6).state;
        _nothingDue(C6, s6, "s8.15 nothing due on a seat in flight");
        _advanceAsReported(C6, s6, "s8.15 an advance with nothing due on a seat in flight");
        assertEq(venueOf(C6).seatOf(C6, s6).state, s6state, "s8.10 nothing due: state unchanged");
        _nothingDue(C, s, "s8.15 nothing due on a ruled seat");
        _advanceAsReported(C, s, "s8.15 an advance with nothing due on a ruled seat");
        assertEq(venueOf(C).seatOf(C, s).state, 2, "s8.10 terminal seat: no revert, no change");

        // s2.12: an accepted seat whose own clock runs out under advance, where the venue runs one
        (bytes32 Cr, uint64 sr) = _open();
        if (venueOf(Cr).seatOf(Cr, sr).accepted == 0) { vm.prank(JUDGE, JUDGE); venueOf(Cr).accept(Cr, sr); }
        st = venueOf(Cr).seatOf(Cr, sr);
        if (st.state == 0 && st.windowEnd != 0) {
            vm.warp(st.windowEnd);
            _nothingDue(Cr, sr, "s2.11 nothing due at the last second of an accepted seat's clock");
            _advanceAsReported(Cr, sr, "s2.11 an advance at the last second of an accepted seat's clock");
            assertEq(venueOf(Cr).seatOf(Cr, sr).state, 0, "s2.11 at the last second of its clock the accepted seat stays");
            vm.warp(uint256(st.windowEnd) + 1);
            IGaleedVenue.Seat memory prer = venueOf(Cr).seatOf(Cr, sr);
            _advanceAsReported(Cr, sr, "s2.12 an accepted seat's clock");
            assertEq(venueOf(Cr).seatOf(Cr, sr).state, 5, "s2.6 a seated seat past its window, no ruling due, is closed and never lapsed");
            _kept(prer, venueOf(Cr).seatOf(Cr, sr), "s2.3 the closed seat keeps its window, accepted and range");
        }

        // 11. withdrawal
        (, uint256 claimable) = venueOf(C).accountOf(JUDGE, feeToken());
        uint256 balBefore = MockERC20(feeToken()).balanceOf(PAYOUT);
        vm.prank(JUDGE, JUDGE); _drain(); vm.expectRevert();
        venueOf(C).withdraw(feeToken(), JUDGE, PAYOUT2); vm.getRecordedLogs();
        vm.prank(STRANGER, STRANGER); _drain(); vm.expectRevert();
        venueOf(C).withdraw(feeToken(), JUDGE, address(0)); vm.getRecordedLogs();
        vm.prank(JUDGE, JUDGE);
        venueOf(C).withdraw(feeToken(), JUDGE, address(0));
        assertEq(MockERC20(feeToken()).balanceOf(PAYOUT) - balBefore, claimable, "s8.11 claimable moved to payout");
        (, uint256 after_) = venueOf(C).accountOf(JUDGE, feeToken());
        assertEq(after_, 0, "s8.11 nothing left claimable");
        vm.prank(STRANGER, STRANGER); _drain(); vm.expectRevert();
        venueOf(C).withdraw(feeToken(), STRANGER, address(0)); vm.getRecordedLogs();
        _creditScan();

        _movedKey();
        _creditScan();
        if (venueDraws()) _routers(T, book);

        // 15. over the whole replay no credited field falls, and every credit to J is counted on J's seatings
        _creditScan();
        _creditAccount();
        _wholeReplay();

        // 12. every Anchored is an accepted rule or resign, over the whole replay
        _drain();
        uint256 n = 0;
        for (uint256 i = 0; i < _seen.length; i++) {
            Vm.Log memory l = _seen[i];
            if (!_isVenue(l.emitter) || l.topics.length == 0 || l.topics[0] != ANCHORED_SIG) continue;
            n++;
            assertEq(l.topics.length, 3, "s8.12 three topics");
            address sr = _bk().root(address(uint160(uint256(l.topics[1]))));
            assertTrue(sr == JUDGE || _resigner[sr], "s8.12 the sender is a key of the record of the judge whose call it was");
            bytes memory carried = abi.decode(l.data, (bytes));
            assertEq(sha256(carried), l.topics[2], "s8.12 the data decodes to bytes whose sha256 is the hash");
            bool known = false;
            for (uint256 j = 0; j < anchoredWords.length; j++) {
                if (anchoredWords[j] == l.topics[2] && keccak256(anchoredEntries[j]) == keccak256(carried)) known = true;
            }
            assertTrue(known, "s8.12 hash and bytes are the arguments of an accepted call");
        }
        assertEq(n, anchoredWords.length, "s8.12 Anchored count equals accepted rule and resign calls");
        _solvent("at the end of the replay");
    }

    /// `collect` as an external call, so that a collection the venue cannot pay is caught.
    function collectForReplay(bytes32 C, address who) external {
        require(msg.sender == address(this));
        collect(C, who);
    }

    /// s2.12, s3: every sum an action moves to a role beyond its share goes back to the address
    /// that put it in or to the party the venue held it for, so the venue's books balance: once
    /// every seat in flight has ended (by its judge's resignation, or under `advance` once its
    /// window has passed) and every party has taken what the venue holds for it, what the venue
    /// still holds covers what every judge it seated can claim. A sum moved to a role out of
    /// another case's money, or out of nobody's, shows up here as a collection the venue cannot
    /// pay or a claimable it does not hold.
    function _solvent(string memory what) internal {
        string memory m = string.concat("s2.12, s3 once every case has ended and every party has taken what the venue holds for it, the venue holds what every judge can claim: ", what);
        _register();
        // a redraw an end opens is a new entry, ended on the next pass
        uint256 from = 0;
        for (uint256 pass = 0; pass < 16 && from < _credits.length; pass++) {
            uint256 upTo = _credits.length;
            for (uint256 i = from; i < upTo; i++) {
                (address v, bytes32 c, uint64 s) = (_credits[i].v, _credits[i].c, _credits[i].s);
                for (uint256 t = 0; t < 4; t++) {
                    IGaleedVenue.Seat memory st = IGaleedVenue(v).seatOf(c, s);
                    if (!_inFlight(st.state)) break;
                    if (st.windowEnd != 0 && vm.getBlockTimestamp() > st.windowEnd) {
                        vm.prank(STRANGER, STRANGER);
                        IGaleedVenue(v).advance(c, s);
                    } else {
                        address k = _bk().current(st.judge);
                        bytes memory en = abi.encodePacked("resignation entry of the solvency sweep ", i, t);
                        bytes memory rs = abi.encodeCall(IGaleedVenue.resign, (c, s, sha256(en), en));
                        vm.prank(k, k);
                        (bool ok, ) = v.call(rs);
                        if (!ok) break;
                    }
                }
                assertFalse(_inFlight(IGaleedVenue(v).seatOf(c, s).state),
                    string.concat("s2.10, s2.11 a seat in flight ends by its judge's resignation up to its window's end, past it under advance: ", what));
            }
            _register();
            from = upTo;
        }
        uint256 n = _credits.length;
        address[] memory vs = new address[](n);
        bytes32[] memory cs = new bytes32[](n);
        address[] memory js = new address[](n);
        for (uint256 i = 0; i < n; i++) {
            (vs[i], cs[i]) = (_credits[i].v, _credits[i].c);
            js[i] = IGaleedVenue(vs[i]).seatOf(cs[i], _credits[i].s).judge;
            bool earlier = false;
            for (uint256 q = 0; q < i && !earlier; q++) earlier = cs[q] == cs[i] && vs[q] == vs[i];
            if (earlier) continue;
            for (uint256 r = 0; r < roles; r++) {
                address[] memory xs = roleAddresses(cs[i], r);
                for (uint256 x = 0; x < xs.length; x++) {
                    try this.collectForReplay(cs[i], xs[x]) {} catch { fail(m); }
                }
            }
        }
        address[2] memory tks = [feeToken(), address(token)];
        for (uint256 k = 0; k < _venues.length; k++) {
            for (uint256 w = 0; w < 2; w++) {
                if (w == 1 && tks[1] == tks[0]) continue;
                uint256 owed = 0;
                for (uint256 i = 0; i < n; i++) {
                    if (vs[i] != _venues[k]) continue;
                    bool earlier = false;
                    for (uint256 q = 0; q < i && !earlier; q++) earlier = vs[q] == vs[i] && js[q] == js[i];
                    if (earlier) continue;
                    (, uint256 cl) = IGaleedVenue(vs[i]).accountOf(js[i], tks[w]);
                    owed += cl;
                }
                assertGe(MockERC20(tks[w]).balanceOf(_venues[k]), owed, m);
            }
        }
    }

    // ---------------------------------------------------------------------
    // step 3: the panel face
    // ---------------------------------------------------------------------

    function _panelFace(bytes32 T, address book) internal {
        address[] memory members = new address[](1); members[0] = JUDGE;
        bytes32[] memory words = new bytes32[](1); words[0] = AMANAH;
        uint64 future = uint64(vm.getBlockTimestamp() + 3650 days);
        if (venueDraws()) {
            // the contract that holds the face: the venue, or the book its standard names
            IGaleedVenue face = book == address(0) ? venue : IGaleedVenue(book);
            // s4: a stranger's panel under the same terms, recorded before the guild's, is never drawn from (step 4 seats JUDGE)
            address[] memory imp = new address[](1); imp[0] = IMPOSTOR;
            _engage(face, STRANGER, T, imp, future, address(0), R_NONE, 0);
            bytes32[] memory two = new bytes32[](2); two[0] = AMANAH; two[1] = AMANAH;
            vm.prank(GUILD, GUILD); _drain(); vm.expectRevert();
            face.engage(T, members, two, ROSTER, future, address(0)); vm.getRecordedLogs();
            address[] memory none = new address[](0); bytes32[] memory noneW = new bytes32[](0);
            vm.prank(GUILD, GUILD); _drain(); vm.expectRevert();
            face.engage(T, none, noneW, ROSTER, future, address(0)); vm.getRecordedLogs();
            bytes32[] memory zero = new bytes32[](1); zero[0] = bytes32(0);
            vm.prank(GUILD, GUILD); _drain(); vm.expectRevert();
            face.engage(T, members, zero, ROSTER, future, address(0)); vm.getRecordedLogs();
            // s4 fifth rejection: two members whose words differ (the guild's covenant word is one word for the whole panel)
            address[] memory pair = new address[](2); pair[0] = JUDGE; pair[1] = address(uint160(JUDGE) + 1);
            bytes32[] memory differ = new bytes32[](2); differ[0] = AMANAH; differ[1] = bytes32(uint256(AMANAH) ^ 1);
            vm.prank(GUILD, GUILD); _drain(); vm.expectRevert();
            face.engage(T, pair, differ, ROSTER, future, address(0)); vm.getRecordedLogs();
            // s4: two members under one word is a panel the face accepts
            _engage(face, GUILD, T, pair, future, address(0), R_NONE, 0);
            // then [J], the guild's latest panel, from which every draw rule seats J at step 4
            _engage(face, GUILD, T, members, future, address(0), R_NONE, 0);
            // s4, s7.15: a stranger's panel recorded after the guild's is never drawn from either (step 4 seats JUDGE)
            _engage(face, STRANGER, T, imp, future, address(0), R_NONE, 0);
            // s4: an expires not after the block's timestamp
            vm.prank(GUILD, GUILD); _drain(); vm.expectRevert();
            face.engage(T, members, words, ROSTER, uint64(vm.getBlockTimestamp()), address(0)); vm.getRecordedLogs();
            // s4 sixth rejection: a nonzero router that carries no code
            vm.prank(GUILD, GUILD); _drain(); vm.expectRevert();
            face.engage(T, members, words, ROSTER, future, address(0xdead)); vm.getRecordedLogs();
            if (book != address(0)) {
                // s7.15 third branch: the venue's own engage reverts
                vm.prank(GUILD, GUILD); _drain(); vm.expectRevert();
                venue.engage(T, members, words, ROSTER, future, address(0)); vm.getRecordedLogs();
            }
        } else {
            vm.prank(GUILD, GUILD); _drain(); vm.expectRevert();
            venue.engage(T, members, words, ROSTER, future, address(0)); vm.getRecordedLogs();
        }
    }

    // ---------------------------------------------------------------------
    // step 13: the key moves before every call §1 gates by the current key
    // ---------------------------------------------------------------------

    uint160 internal _freshKeys = uint160(0x7000000000000000000000000000000000000000);

    /// Moves the record of `rec` to a fresh key: the key current now declares
    /// it and it accedes (galeed.roster/1 s4.2).
    function _rekey(address rec) internal returns (address old, address cur) {
        return _rekeyTo(rec, address(++_freshKeys));
    }

    /// every key each record held, its root first, in order
    mapping(address => address[]) internal _keys;

    function _rekeyTo(address rec, address next) internal returns (address old, address cur) {
        GaleedSuccessionBook bk = _bk();
        old = bk.current(rec);
        cur = next;
        if (_keys[rec].length == 0) _keys[rec].push(old);
        _keys[rec].push(next);
        vm.prank(old, old);
        bk.declare(cur);
        vm.prank(cur, cur);
        bk.accede(old);
        assertEq(bk.current(rec), cur, "s8.13 the book moved the record's key");
    }

    uint8 internal constant G_KEY = 0;      // engage: the sender is its record's current key (a contract may be a guild)
    uint8 internal constant G_ACCOUNT = 1;  // setPayout, withdraw: the sender is the named key's current key
    uint8 internal constant G_DIRECT = 2;   // accept, rule, resign: the three conditions of s2.9

    /// Moves the record's key (twice before the record's first gated call, so
    /// that it holds a root, a key between and the current key), then sends
    /// `data` to `target` from every key the record held before, and, by
    /// `gate`, from the current key through a contract and from the current
    /// key carrying code, each refused, and from the current key, taken.
    /// Returns the current key.
    function _gated(address target, address rec, bytes memory data, uint8 gate, string memory what) internal returns (address cur) {
        address old;
        if (_keys[rec].length < 2) _rekey(rec);
        (old, cur) = _rekey(rec);
        bool ok;
        _drain();
        vm.prank(old, old);
        (ok, ) = target.call(data);
        assertFalse(ok, string.concat("s1 refused from the key current a moment earlier: ", what));
        vm.getRecordedLogs();
        for (uint256 i = 0; i + 1 < _keys[rec].length; i++) {
            address k = _keys[rec][i];
            _drain();
            vm.prank(k, k);
            (ok, ) = target.call(data);
            assertFalse(ok, string.concat("s1 refused from every key the record held before, its root and every key between: ", what));
            vm.getRecordedLogs();
        }
        if (gate != G_KEY) {
            _drain();
            vm.prank(cur, cur);
            (ok, ) = address(fwd).call(abi.encodeCall(Forwarder.forward, (target, data)));
            assertFalse(ok, string.concat("s1 refused from the current key through a contract: ", what));
            vm.getRecordedLogs();
        }
        if (gate == G_DIRECT) {
            _withCode(cur);
            _drain();
            vm.prank(cur, cur);
            (ok, ) = target.call(data);
            _withoutCode(cur);
            assertFalse(ok, string.concat("s2.9 refused from the current key carrying code: ", what));
            vm.getRecordedLogs();
            // the key moves to the address of a contract whose constructor sends the call
            bytes memory init = abi.encodePacked(type(ConstructorCall).creationCode, abi.encode(target, data));
            bytes32 salt = bytes32(uint256(_freshKeys));
            address born = vm.computeCreate2Address(salt, keccak256(init), address(this));
            (, cur) = _rekeyTo(rec, born);
            // the harness's prank of the accession counted a nonce there; a key that never sent is at zero
            vm.setNonceUnsafe(born, 0);
            address made;
            _drain();
            assembly { made := create2(0, add(init, 0x20), mload(init), salt) }
            // the constructor's call was refused: what its frame emitted no chain keeps
            vm.getRecordedLogs();
            assertEq(made, born, "the constructor call is born at the current key");
            assertFalse(ConstructorCall(made).taken(), string.concat("s2.9 refused from the current key that is not tx.origin: ", what));
            (, cur) = _rekey(rec);
        }
        vm.prank(cur, cur);
        (ok, ) = target.call(data);
        assertTrue(ok, string.concat("s1 taken from the current key: ", what));
    }

    function _movedKey() internal {
        // three seats opened before the first move: an offer, one to resign, one to rule
        (bytes32 Co, uint64 so) = _open();
        (bytes32 Ca, uint64 sa) = _open();
        (bytes32 Cd, uint64 sd) = _open();
        vm.prank(STRANGER, STRANGER); _drain(); vm.expectRevert();
        venue.setPayout(MOVER, STRANGER); vm.getRecordedLogs();
        IGaleedVenue va = venueOf(Ca);
        IGaleedVenue vd = venueOf(Cd);
        if (va.seatOf(Ca, sa).accepted == 0) _gated(address(va), JUDGE, abi.encodeCall(IGaleedVenue.accept, (Ca, sa)), G_DIRECT, "accept");
        if (vd.seatOf(Cd, sd).accepted == 0) _gated(address(vd), JUDGE, abi.encodeCall(IGaleedVenue.accept, (Cd, sd)), G_DIRECT, "accept");
        assertEq(vd.seatOf(Cd, sd).judge, JUDGE, "s1 the address recorded stays the one seated");
        dispute(Cd, sd);
        IGaleedVenue.Seat memory st = vd.seatOf(Cd, sd);
        bytes memory extra = rulingExtra(Cd, sd);
        uint256 before = _anchoredCount();
        // s8.15: the ruling the moved key sends is read against outcomeOf like every other ruling
        IGaleedVenue.Outcome memory o7 = vd.outcomeOf(Cd, sd, st.rungMin, extra);
        _shape(o7, st.amount, "s8.15 the moved key's ruling");
        uint256[] memory rb7 = _received(Cd, st.token);
        Snap memory sn7 = _snap(vd, Cd, sd);
        address cur = _gated(address(vd), JUDGE, abi.encodeCall(IGaleedVenue.rule, (Cd, sd, E7, st.rungMin, N7, extra)), G_DIRECT, "rule");
        _kept(st, vd.seatOf(Cd, sd), "s2.3 the seat the moved key ruled keeps its window, accepted and range");
        _moved(sn7, _snap(vd, Cd, sd), o7, "s8.15 the moved key's ruling");
        _sharesPaid(Cd, st.token, rb7, o7, "s8.15 the moved key's ruling");
        assertTrue(_same(o7, vd.outcomeOf(Cd, sd, st.rungMin, extra)), "s8.15 outcomeOf on the seat the moved key ruled reports what it reported before");
        anchoredWords.push(E7);
        anchoredEntries.push(N7);
        assertEq(_anchoredCount(), before + 1, "s8.13 one Anchored");
        assertTrue(_anchoredIs(before, cur, E7, N7), "s8.13 Anchored(current key, E7, entry)");
        assertEq(vd.seatOf(Cd, sd).state, 2, "s8.13 ruled by the current key");
        before = _anchoredCount();
        IGaleedVenue.Seat memory prea = va.seatOf(Ca, sa);
        cur = _gated(address(va), JUDGE, abi.encodeCall(IGaleedVenue.resign, (Ca, sa, E8, N8)), G_DIRECT, "resign");
        _kept(prea, va.seatOf(Ca, sa), "s2.10 the seat the moved key resigned keeps its window, accepted and range");
        anchoredWords.push(E8);
        anchoredEntries.push(N8);
        assertTrue(_anchoredIs(before, cur, E8, N8), "s8.13 Anchored(current key, E8, entry)");
        assertEq(va.seatOf(Ca, sa).state, 3, "s8.13 resigned by the current key");
        IGaleedVenue vo = venueOf(Co);
        if (vo.seatOf(Co, so).accepted == 0) {
            before = _anchoredCount();
            IGaleedVenue.Seat memory preo = vo.seatOf(Co, so);
            cur = _gated(address(vo), JUDGE, abi.encodeCall(IGaleedVenue.resign, (Co, so, E9, N9)), G_DIRECT, "decline");
            _kept(preo, vo.seatOf(Co, so), "s2.10 the offer the moved key declined keeps its window, accepted and range");
            anchoredWords.push(E9);
            anchoredEntries.push(N9);
            assertTrue(_anchoredIs(before, cur, E9, N9), "s8.13 Anchored(current key, E9, entry)");
            assertEq(vo.seatOf(Co, so).state, 3, "s8.13 declined by the current key");
            assertEq(vo.seatOf(Co, so).accepted, 0, "s8.13 a declined offer keeps accepted zero");
        }
        // J's account, its payout registered at step 2
        (, uint256 owed) = vd.accountOf(JUDGE, feeToken());
        uint256 balBefore = MockERC20(feeToken()).balanceOf(PAYOUT);
        _gated(address(vd), JUDGE, abi.encodeCall(IGaleedVenue.withdraw, (feeToken(), JUDGE, PAYOUT)), G_ACCOUNT, "withdraw");
        assertEq(MockERC20(feeToken()).balanceOf(PAYOUT) - balBefore, owed, "s8.13 the current key withdraws the seat's claimable to its payout");
        (address old, ) = _rekey(JUDGE);
        vm.prank(old, old); _drain(); vm.expectRevert();
        vd.setPayout(JUDGE, PAYOUT2); vm.getRecordedLogs();
        // keys with no payout: accountOf reports zero until the current key registers one
        (address m0, ) = venue.accountOf(MOVER, feeToken());
        assertEq(m0, address(0), "s3 no payout before registration");
        // s3: a payout may carry code
        _gated(address(venue), MOVER, abi.encodeCall(IGaleedVenue.setPayout, (MOVER, address(fwd))), G_ACCOUNT, "setPayout on a key with no payout, to a contract");
        (address m1, ) = venue.accountOf(MOVER, feeToken());
        assertEq(m1, address(fwd), "s8.13 the current key registered the payout");
        _gated(address(venue), MOVER2, abi.encodeCall(IGaleedVenue.withdraw, (feeToken(), MOVER2, Q2)), G_ACCOUNT, "withdraw registering a payout");
        (address m2, ) = venue.accountOf(MOVER2, feeToken());
        assertEq(m2, Q2, "s8.13 the current key's withdraw registered the payout");
    }

    // ---------------------------------------------------------------------
    // step 14: the panel each case freezes, and the router's call
    // ---------------------------------------------------------------------

    uint8 internal constant R_NONE = 0;   // no router: the venue's own rule, a member of the panel
    uint8 internal constant R_FIXED = 1;  // members[k] where it is neither seated nor offered
    uint8 internal constant R_FIRST = 2;  // the first member neither seated nor offered
    uint8 internal constant R_FAIL = 3;   // no usable panel at any draw
    uint8 internal constant R_SECOND = 4; // the first eligible member below ordinal k (1 where k is 0), none from it on
    uint8 internal constant R_PARITY = 5; // members[ordinal % 2] where it is neither seated nor offered

    /// The replay's own record of the panels it engaged (s4), in the order recorded.
    struct RPanel {
        address root;
        bytes32 terms;
        address[] members;
        uint64 expires;
        uint256 recorded;
        uint8 kind;
        uint256 k;
        bytes32 word;
    }

    RPanel[] internal _rec;
    /// the record index plus one of the panel each case froze
    mapping(bytes32 => uint256) internal _frozenOf;
    bytes32 internal _terms;

    /// at step 14 every panel the replay engages carries a word of its own,
    /// so that the word a seat freezes names the panel it was drawn from
    bool internal _distinct;

    function _word() internal view returns (bytes32) {
        return _distinct ? keccak256(abi.encode(AMANAH, _rec.length)) : AMANAH;
    }

    function _engage(IGaleedVenue face, address rec, bytes32 terms, address[] memory members, uint64 expires, address router, uint8 kind, uint256 k) internal {
        address key = _bk().current(rec);
        bytes32 w = _word();
        bytes32[] memory words = new bytes32[](members.length);
        for (uint256 i = 0; i < members.length; i++) words[i] = w;
        vm.prank(key, key);
        face.engage(terms, members, words, ROSTER, expires, router);
        _record(_bk().root(rec), terms, members, expires, kind, k, w);
    }

    function _record(address root, bytes32 terms, address[] memory members, uint64 expires, uint8 kind, uint256 k, bytes32 w) internal {
        RPanel storage p = _rec.push();
        p.root = root;
        p.terms = terms;
        for (uint256 i = 0; i < members.length; i++) p.members.push(members[i]);
        p.expires = expires;
        p.recorded = vm.getBlockNumber();
        p.kind = kind;
        p.k = k;
        p.word = w;
    }

    /// §4: the latest of the panels recorded under the named guild's root and
    /// the terms before this block that are unexpired now.
    function _frozenFor(address named) internal view returns (bool, uint256) {
        address root = _bk().root(named);
        uint256 n = vm.getBlockNumber();
        uint256 t = vm.getBlockTimestamp();
        for (uint256 i = _rec.length; i > 0; i--) {
            RPanel storage p = _rec[i - 1];
            if (p.root == root && p.terms == _terms && p.recorded < n && p.expires >= t) return (true, i - 1);
        }
        return (false, 0);
    }

    function _in(address a, address[] memory xs) internal pure returns (bool) {
        for (uint256 i = 0; i < xs.length; i++) if (xs[i] == a) return true;
        return false;
    }

    /// What a draw from panel `pi` seats with `taken` seated or offered on the
    /// case: whether a member is seated, and which one (zero where the venue's
    /// own rule picks any eligible member).
    function _expect(uint256 pi, address[] memory taken, uint64 ordinal) internal view returns (bool usable, address member) {
        RPanel storage p = _rec[pi];
        if (p.kind == R_FAIL) return (false, address(0));
        if (p.kind == R_SECOND && ordinal >= (p.k == 0 ? 1 : p.k)) return (false, address(0));
        if (p.kind == R_PARITY) {
            address pm = p.members[ordinal % 2];
            if (!_in(pm, taken)) return (true, pm);
            return (false, address(0));
        }
        if (p.kind == R_FIXED) {
            if (p.k < p.members.length && !_in(p.members[p.k], taken)) return (true, p.members[p.k]);
            return (false, address(0));
        }
        for (uint256 i = 0; i < p.members.length; i++) {
            if (!_in(p.members[i], taken)) return (true, p.kind == R_NONE ? address(0) : p.members[i]);
        }
        return (false, address(0));
    }

    function _judgesOf(bytes32 C, uint64 upTo) internal view returns (address[] memory js) {
        js = new address[](uint256(upTo) + 1);
        for (uint64 i = 0; i <= upTo; i++) js[i] = venueOf(C).seatOf(C, i).judge;
    }

    /// Opens a case whose terms name the guild by `named`, against the
    /// replay's prediction of the panel it freezes and the member it seats.
    function _openExpect(address named, string memory what) internal returns (bytes32 C, uint64 s, bool ok) {
        (bool has, uint256 pi) = _frozenFor(named);
        bool usable;
        address m;
        if (has) (usable, m) = _expect(pi, new address[](0), 0);
        _drain();
        if (named == GUILD) {
            try this.openForReplay() returns (bytes32 c, uint64 x) { C = c; s = x; ok = true; } catch { vm.getRecordedLogs(); }
        } else {
            try this.openNamingForReplay(named) returns (bytes32 c, uint64 x) { C = c; s = x; ok = true; } catch { vm.getRecordedLogs(); }
        }
        if (!usable) {
            assertFalse(ok, string.concat("s4 no usable panel, the creation is refused: ", what));
            return (C, s, ok);
        }
        assertTrue(ok, string.concat("s4 the frozen panel seats: ", what));
        address j = venueOf(C).seatOf(C, s).judge;
        assertEq(venueOf(C).seatOf(C, s).amanahHash, _rec[pi].word, string.concat("s4 the seat carries the frozen panel's word: ", what));
        if (m == address(0)) assertTrue(_in(j, _rec[pi].members), string.concat("s4 a member of the frozen panel: ", what));
        else assertEq(j, m, string.concat("s4 the member the frozen panel's router names: ", what));
        _frozenOf[C] = pi + 1;
    }

    /// The seat a redraw opened, accepted by its judge's current key where it
    /// waits for one and disputed by the venue's own path, ruled on a snapshot
    /// at every rung of its range: each ruling does what outcomeOf reported.
    function _ruleRedrawn(bytes32 C, uint64 s, string memory what) internal {
        _drain();
        uint256 snap0 = vm.snapshotState();
        IGaleedVenue v = venueOf(C);
        address k = _bk().current(v.seatOf(C, s).judge);
        if (v.seatOf(C, s).accepted == 0) {
            _refuse(STRANGER, address(v), abi.encodeCall(IGaleedVenue.accept, (C, s)), string.concat("s2.7 a stranger's accept on a redrawn seat is refused: ", what));
            vm.prank(k, k);
            v.accept(C, s);
        }
        dispute(C, s);
        IGaleedVenue.Seat memory st = v.seatOf(C, s);
        assertEq(st.state, 1, string.concat("s8.6 the redrawn seat disputed: ", what));
        bytes memory extra = rulingExtra(C, s);
        {
            // s2.9, s2.10: the three conditions hold on a redrawn seat as on the first
            uint64 r0 = st.rungs == 0 ? 7 : st.rungMin;
            bytes memory rd = abi.encodeCall(IGaleedVenue.rule, (C, s, E1, r0, N1, extra));
            _refuse(STRANGER, address(v), rd, string.concat("s2.9 a stranger's rule on a redrawn seat is refused: ", what));
            _refuse(STRANGER, address(v), abi.encodeCall(IGaleedVenue.resign, (C, s, E3, N3)), string.concat("s2.10 a stranger's resign on a redrawn seat is refused: ", what));
            _refuse(k, address(fwd), abi.encodeCall(Forwarder.forward, (address(v), rd)), string.concat("s2.9 the redrawn judge's rule through a contract is refused: ", what));
            _withCode(k);
            _refuse(k, address(v), rd, string.concat("s2.9 the redrawn judge's rule carrying code is refused: ", what));
            _withoutCode(k);
        }
        // s2.3: at the ruling window's last second
        if (st.windowEnd > vm.getBlockTimestamp()) vm.warp(st.windowEnd);
        uint64 top = st.rungs == 0 ? st.rungMin : st.rungMax;
        for (uint64 r = st.rungMin; r <= top && r < st.rungMin + 16; r++) {
            uint256 snap = vm.snapshotState();
            _ruleOne(C, s, r, extra, k, what);
            vm.revertToStateAndDelete(snap);
        }
        vm.revertToStateAndDelete(snap0);
        // what the snapshots emitted belongs to no state the replay keeps
        vm.getRecordedLogs();
    }

    function _ruleOne(bytes32 C, uint64 s, uint64 r, bytes memory extra, address k, string memory what) internal {
        IGaleedVenue v = venueOf(C);
        IGaleedVenue.Seat memory st = v.seatOf(C, s);
        IGaleedVenue.Outcome memory o = v.outcomeOf(C, s, r, extra);
        _shape(o, st.amount, what);
        uint256[] memory rb = _received(C, st.token);
        Snap memory a = _snap(v, C, s);
        uint256 a0 = _anchoredCount();
        vm.prank(k, k);
        v.rule(C, s, E1, r, N1, extra);
        assertEq(_anchoredCount(), a0 + 1, string.concat("s2.9 a redrawn seat's ruling emits one Anchored: ", what));
        assertTrue(_anchoredIs(a0, k, E1, N1), string.concat("s2.9 Anchored(the redrawn judge's key, E, entry): ", what));
        assertEq(v.seatOf(C, s).state, o.state, string.concat("s2.12 the state outcomeOf reported: ", what));
        _moved(a, _snap(v, C, s), o, what);
        _sharesPaid(C, st.token, rb, o, what);
        _solvent(what);
    }

    /// The sweep's gas limits, placed by what the path used with ample gas:
    /// from a little below it to where the router could be handed all of
    /// routeGas() even had it used none of it, and a margin past that.
    function _window(uint256 used) internal view returns (uint256 lo, uint256 hi) {
        uint256 rg = venue.routeGas();
        lo = used > 90_000 ? used - 60_000 : 30_000;
        hi = used + rg + rg / 63 + 60_000;
    }

    /// A router that answers the first eligible member only where its entry
    /// gas is what exactly routeGas() leaves there, within 16: its entry cost
    /// is read from a calibration copy before it is deployed.
    function _exactProbe() internal returns (address) {
        return _exact(false);
    }

    /// The same, answering off routeGas() by reverting at once (`gate`) or by naming the panel's last member.
    function _exact(bool gate) internal returns (address) {
        uint256 rg = venue.routeGas();
        address cal = gate ? address(new GasGateRouter(rg, 600)) : address(new GasProbeRouter(rg, 600));
        address[] memory m = new address[](3); m[0] = JUDGE; m[1] = JUDGE3; m[2] = JUDGE4;
        RouteView memory v;
        v.members = m;
        v.seated = new address[](0);
        v.parties = new address[](0);
        // the largest call gas at which the calibration copy still answers 0 is routeGas() plus its entry cost
        uint256 lo = rg; uint256 hi = rg + 600;
        while (hi - lo > 1) {
            uint256 mid = (lo + hi) / 2;
            (bool ok, bytes memory r) = cal.staticcall{gas: mid}(abi.encodeCall(GasProbeRouter.route, (v)));
            if (ok && abi.decode(r, (uint256)) == 0) lo = mid; else hi = mid;
        }
        uint256 entryCost = lo - rg;
        return gate ? address(new GasGateRouter(rg - entryCost + 8, 16)) : address(new GasProbeRouter(rg - entryCost + 8, 16));
    }

    /// The smallest gas limit at which `data` sent to `target` is taken, between `lo` (not) and `hi` (taken).
    function _takenAt(address target, bytes memory data, uint256 lo, uint256 hi) internal returns (uint256) {
        while (hi - lo > 1) {
            uint256 mid = (lo + hi) / 2;
            uint256 snap = vm.snapshotState();
            (bool ok, ) = target.call{gas: mid}(data);
            vm.revertToStateAndDelete(snap);
            if (ok) hi = mid; else lo = mid;
        }
        return hi;
    }

    /// At every gas limit of the sweep, a creation either reverts whole or
    /// seats `expected`, and at some limit it seats.
    function _sweepOpen(address expected) internal {
        _drain();
        uint256 snap0 = vm.snapshotState();
        uint256 b = gasleft();
        this.openForReplay();
        uint256 used = b - gasleft();
        vm.revertToStateAndDelete(snap0);
        (uint256 lo, uint256 hi) = _window(used);
        uint256 landed = 0;
        for (uint256 g = lo; g <= hi; g += 5_000) {
            uint256 snap = vm.snapshotState();
            try this.openForReplay{gas: g}() returns (bytes32 c, uint64 x) {
                landed++;
                assertEq(venueOf(c).seatOf(c, x).judge, expected, "s4 at every gas limit the creation reverts whole or the router answers under exactly routeGas()");
            } catch {}
            vm.revertToStateAndDelete(snap);
        }
        assertGt(landed, 0, "s4 the creation sweep seats at some gas limit");
        uint256 th = _takenAt(address(this), abi.encodeCall(this.openForReplay, ()), lo - 1, hi);
        for (uint256 g = th > 50 ? th - 50 : 0; g <= th + 400; g++) {
            uint256 snap = vm.snapshotState();
            try this.openForReplay{gas: g}() returns (bytes32 c, uint64 x) {
                assertEq(venueOf(c).seatOf(c, x).judge, expected, "s4 at every gas limit by the threshold the creation reverts whole or the router answers under exactly routeGas()");
            } catch {}
            vm.revertToStateAndDelete(snap);
        }
        // what the sweep emitted belongs to no state the replay keeps
        vm.getRecordedLogs();
    }

    function _routers(bytes32 T, address book) internal {
        // the root a case freezes is measured only through a case naming the guild by another address
        assertTrue(opensCaseNaming(), "s8.14 a drawing venue supplies openCaseNaming(), its terms naming a guild address");
        _terms = T;
        _distinct = true;
        IGaleedVenue face = book == address(0) ? venue : IGaleedVenue(book);
        GaleedSuccessionBook bk = _bk();
        if (book != address(0)) {
            assertEq(IGaleedVenue(book).successionBook(), address(bk), "s4 the panel book names the venue's succession book");
            assertEq(IGaleedVenue(book).routeGas(), venue.routeGas(), "s4 the panel book names the venue's route gas");
            _readsNoState(book, "the panel book");
        }
        uint64 far = uint64(vm.getBlockTimestamp() + 3650 days);
        address[] memory duo = new address[](2); duo[0] = JUDGE; duo[1] = JUDGE3;
        address[] memory trio = new address[](3); trio[0] = JUDGE; trio[1] = JUDGE3; trio[2] = JUDGE4;
        address[] memory imp = new address[](1); imp[0] = IMPOSTOR;

        // a panel recorded in the creation block is not frozen into the case
        _engage(face, GUILD, T, imp, far, address(0), R_NONE, 0);
        _openExpect(GUILD, "a panel of the creation block is not drawn");
        _roll();

        // a router whose route returns a member's index seats that member
        // (every router is deployed before the prank, which a creation would consume)
        address f1 = address(new FixedRouter(1));
        _engage(face, GUILD, T, duo, far, f1, R_FIXED, 1);
        _roll();
        _openExpect(GUILD, "the router's index seats that member");

        // routers that leave the case with no usable panel: one that reverts, an index outside
        // members, one that spends its gas, two words of return data, 31 bytes of return data,
        // and one that writes state, which a call by staticcall cannot do
        address[6] memory bad = [
            address(new RevertRouter()), address(new FixedRouter(2)), address(new HogRouter(venue.routeGas())),
            address(new LongReturnRouter()), address(new ShortReturnRouter()), address(new WriterRouter())
        ];
        for (uint256 i = 0; i < bad.length; i++) {
            _engage(face, GUILD, T, duo, far, bad[i], R_FAIL, 0);
            _roll();
            _openExpect(GUILD, "a failing router leaves the case with no usable panel");
        }
        // a router whose code hash changed after the panel recorded it
        FixedRouter f0 = new FixedRouter(0);
        _engage(face, GUILD, T, duo, far, address(f0), R_FAIL, 0);
        _roll();
        vm.etch(address(f0), f1.code);
        _openExpect(GUILD, "a router code hash that differs leaves the case with no usable panel");

        // a router that reads the view seats only where the fields are the venue's
        address echo = address(new EchoRouter(duo));
        _engage(face, GUILD, T, duo, far, echo, R_FIRST, 0);
        _roll();
        _openExpect(GUILD, "the view carries the venue, the chain, the time, the members and the seated");
        // a panel that lists a member twice reaches the router as recorded
        address[] memory twice = new address[](3); twice[0] = JUDGE; twice[1] = JUDGE; twice[2] = JUDGE3;
        address echo2 = address(new EchoRouter(twice));
        _engage(face, GUILD, T, twice, far, echo2, R_FIRST, 0);
        _roll();
        _openExpect(GUILD, "a panel listing a member twice reaches the router as recorded");

        // a router whose index is the hash of the view's case, terms, token, amount, rungs and parties
        address[] memory sixteen = new address[](16);
        for (uint256 i = 0; i < 16; i++) sixteen[i] = address(uint160(0x1100000000000000000000000000000000000000) + uint160(i));
        address fields = address(new FieldsRouter());
        _engage(face, GUILD, T, sixteen, far, fields, R_NONE, 0);
        _roll();
        for (uint256 n = 0; n < 4; n++) {
            (bytes32 Cf, uint64 sf, ) = _openExpect(GUILD, "a router reading the view's fields");
            IGaleedVenue.Seat memory sv = venueOf(Cf).seatOf(Cf, sf);
            uint256 want = uint256(keccak256(abi.encode(Cf, sv.termsHash, sv.token, sv.amount, sv.rungs, sv.parties))) % 16;
            assertEq(sv.judge, sixteen[want], "s4 the view carries the caseId, the seat's termsHash, token, amount and rungs, and the parties as seatOf reports them");
        }

        // a router that answers by the gas it receives
        address probe = _exactProbe();
        _engage(face, GUILD, T, trio, far, probe, R_FIRST, 0);
        _roll();
        _sweepOpen(JUDGE);
        _openExpect(GUILD, "the router receives exactly routeGas()");

        // a panel under other terms recorded after the guild's is never read under these
        _engage(face, GUILD, keccak256("other terms"), imp, far, address(0), R_NONE, 0);
        _roll();
        _openExpect(GUILD, "a panel under other terms is not read");

        // expiry: a panel at its expires second, then one second past it, when an older panel is unexpired
        uint64 soon = uint64(vm.getBlockTimestamp() + 1000);
        address[] memory pe = new address[](2); pe[0] = JUDGE4; pe[1] = JUDGE5;
        _engage(face, GUILD, T, pe, soon, address(0), R_NONE, 0);
        _roll();
        vm.warp(soon);
        _openExpect(GUILD, "a panel is unexpired at its expires second");
        vm.warp(uint256(soon) + 1);
        _openExpect(GUILD, "the latest panel expired, the latest unexpired one serves");

        // s8.14: every call that can reach a draw is sent once and read; each that drew is measured on its own path
        _base = NO_PATH;
        for (uint8 p = 0; p < N_PATHS; p++) {
            (bool possible, bool drew) = _observe(face, T, p);
            if (possible && p == PATH_OFFER) assertEq(drew, reassignsAfterExpiredOffer(), "s8.14 reassignsAfterExpiredOffer() names what the venue does");
            if (possible && p == PATH_LAPSE) assertEq(drew, redrawsAfterLapse(), "s8.14 redrawsAfterLapse() names what the venue does");
            if (!drew) continue;
            if (_base == NO_PATH && p < PATH_TERM_OFFER) _base = p;
            _redrawSuite(face, T, p);
        }

        // the guild moves its key: the key it left engages nothing, and the current key's
        // panels are recorded under the root the terms' address resolves to
        address[] memory j3 = new address[](1); j3[0] = JUDGE3;
        bytes32[] memory w1 = new bytes32[](1); w1[0] = AMANAH;
        far = uint64(vm.getBlockTimestamp() + 3650 days);
        w1[0] = _word();
        address gcur = _gated(address(face), GUILD, abi.encodeCall(IGaleedVenue.engage, (T, j3, w1, ROSTER, far, address(0))), G_KEY, "engage");
        _record(bk.root(GUILD), T, j3, far, R_NONE, 0, w1[0]);
        // the guild's current key calls a contract that engages: the face takes the call from the
        // contract, whose own root files it, and no case naming the guild draws from it
        {
            address[] memory imp2 = new address[](1); imp2[0] = IMPOSTOR;
            bytes32[] memory ws2 = new bytes32[](1); ws2[0] = keccak256(abi.encode("a contract the guild's key called", _rec.length));
            _drain();
            vm.prank(gcur, gcur);
            (bool okc, ) = address(fwd).call(abi.encodeCall(Forwarder.forward,
                (address(face), abi.encodeCall(IGaleedVenue.engage, (T, imp2, ws2, ROSTER, far, address(0))))));
            if (okc) _record(bk.root(address(fwd)), T, imp2, far, R_NONE, 0, ws2[0]);
            else vm.getRecordedLogs();
        }
        _roll();
        _openExpect(GUILD, "panels are recorded under the guild's root");
        if (opensCaseNaming()) _openExpect(bk.current(GUILD), "a guild named by its current key freezes its root");
    }

    uint8 internal constant PATH_OFFER = 0;         // a stranger's advance on an offer past its window
    uint8 internal constant PATH_LAPSE = 1;         // a stranger's advance on a disputed seat past its ruling window
    uint8 internal constant PATH_DECLINE = 2;       // the judge's resign on an offer
    uint8 internal constant PATH_RESIGN = 3;        // the judge's resign on an accepted seat
    uint8 internal constant PATH_RESIGN_DISPUTED = 4; // the judge's resign on a disputed seat
    uint8 internal constant PATH_TERM_OFFER = 5;    // a stranger's advance naming a seat that left flight; the later seat an offer past its window
    uint8 internal constant PATH_TERM_SEATED = 6;   // the same; the later seat accepted, its own clock past
    uint8 internal constant PATH_TERM_DISPUTED = 7; // the same; the later seat disputed past its ruling window
    uint8 internal constant N_PATHS = 8;
    uint8 internal constant NO_PATH = 255;
    /// the first path on which the venue drew again: how a terminal path reaches its later seat
    uint8 internal _base;
    uint256 internal _resigns;
    /// the records whose judges the redraw measure had resign
    mapping(address => bool) internal _resigner;

    function _term(uint8 path) internal pure returns (bool) { return path >= PATH_TERM_OFFER && path < N_PATHS; }
    function _byJudge(uint8 path) internal pure returns (bool) { return path == PATH_DECLINE || path == PATH_RESIGN || path == PATH_RESIGN_DISPUTED; }

    function _pathName(uint8 path) internal pure returns (string memory) {
        string[8] memory n = ["after an expired offer: ", "after a lapse: ", "after a decline: ", "after a resignation: ", "after a resignation during a dispute: ",
            "through a seat that left flight, the later an expired offer: ", "through a seat that left flight, the later seat's own clock: ",
            "through a seat that left flight, the later a lapse: "];
        return n[path];
    }

    /// The call that makes the redraw on `path`, sent on seat `s` with `gas`
    /// (zero: all), from a stranger or from the judge's current key.
    function _fireData(bytes32 C, uint64 s, uint8 path) internal returns (address from, bytes memory data) {
        if (_byJudge(path)) {
            from = _bk().current(venueOf(C).seatOf(C, s).judge);
            bytes memory e = abi.encodePacked("resignation entry of the redraw measure ", _resigns);
            data = abi.encodeCall(IGaleedVenue.resign, (C, s, sha256(e), e));
        } else {
            from = STRANGER;
            data = abi.encodeCall(IGaleedVenue.advance, (C, s));
        }
    }

    function _fire(bytes32 C, uint64 s, uint8 path) internal {
        (address from, bytes memory data) = _fireData(C, s, path);
        uint256 before = _anchoredCount();
        vm.prank(from, from);
        (bool ok, ) = address(venueOf(C)).call(data);
        assertTrue(ok, string.concat("s2.10, s2.11 the call is taken: ", _pathName(path)));
        if (_byJudge(path)) {
            _resigner[_bk().root(from)] = true;
            bytes memory e = abi.encodePacked("resignation entry of the redraw measure ", _resigns++);
            anchoredWords.push(sha256(e));
            anchoredEntries.push(e);
            assertEq(_anchoredCount(), before + 1, "s2.10 one Anchored");
        }
    }

    /// The seat whose clock or exit the call moves.
    function _due(uint64 s, uint8 path) internal pure returns (uint64) { return _term(path) ? s + 1 : s; }

    /// What the moved seat must read once the call is taken.
    function _endOk(uint8 path, uint8 st) internal pure returns (bool) {
        if (path == PATH_OFFER || path == PATH_TERM_OFFER || path == PATH_TERM_SEATED) return st == 5;
        if (path == PATH_LAPSE) return st == 4;
        if (path == PATH_TERM_DISPUTED) return st == 4 || st == 5;
        return st == 3;
    }

    /// Readies case C's seat s for the call on `path`. A terminal path first
    /// draws again on the base path and readies the later seat.
    function _ready(bytes32 C, uint64 s, uint8 path) internal returns (bool) {
        IGaleedVenue v = venueOf(C);
        IGaleedVenue.Seat memory st = v.seatOf(C, s);
        if (path == PATH_OFFER) return st.accepted == 0 && st.windowEnd != 0;
        if (path == PATH_DECLINE) return st.accepted == 0 && _inFlight(st.state);
        if (_term(path)) {
            if (_base == NO_PATH || !_ready(C, s, _base)) return false;
            if (!_byJudge(_base)) _pastWindow(C, s);
            uint64 n0 = _latestSeat(C);
            _fire(C, s, _base);
            if (_latestSeat(C) != s + 1 || n0 != s) return false;
            uint64 t = s + 1;
            IGaleedVenue.Seat memory nt = v.seatOf(C, t);
            if (path == PATH_TERM_OFFER) return nt.accepted == 0 && nt.windowEnd != 0;
            if (nt.accepted == 0) {
                address k = _bk().current(nt.judge);
                vm.prank(k, k);
                v.accept(C, t);
            }
            if (path == PATH_TERM_SEATED) return v.seatOf(C, t).state == 0 && v.seatOf(C, t).windowEnd != 0;
            dispute(C, t);
            return v.seatOf(C, t).state == 1;
        }
        if (st.accepted == 0) {
            address k = _bk().current(st.judge);
            vm.prank(k, k);
            v.accept(C, s);
        }
        if (path == PATH_RESIGN) return _inFlight(v.seatOf(C, s).state);
        dispute(C, s);
        return v.seatOf(C, s).state == 1;
    }

    function _pastWindow(bytes32 C, uint64 s) internal {
        uint256 e = venueOf(C).seatOf(C, s).windowEnd;
        if (vm.getBlockTimestamp() <= e) vm.warp(e + 1);
    }

    /// The call on `path`, against the prediction of the draw from the panel
    /// the case froze; the moved seat leaves flight as the path requires.
    function _fireExpect(bytes32 C, uint64 s, uint8 path, bool mustMove, string memory what) internal {
        uint64 d = _due(s, path);
        (bool usable, address m) = _expect(_frozenOf[C] - 1, _judgesOf(C, d), d + 1);
        IGaleedVenue.Seat memory pd = venueOf(C).seatOf(C, d);
        if (_byJudge(path)) {
            IGaleedVenue.Seat memory pre = venueOf(C).seatOf(C, s);
            Snap memory a = _snap(venueOf(C), C, s);
            _fire(C, s, path);
            _creditsCounted(a, _snap(venueOf(C), C, s), string.concat("s7.18 what the resignation credits is counted: ", what));
            assertEq(venueOf(C).seatOf(C, s).accepted, pre.accepted, string.concat("s2.10 accepted kept: ", what));
        } else if (_term(path)) {
            // the named seat left flight: its view reports nothing due, and the call is measured by conservation
            _nothingDue(C, s, string.concat("s2.12 nothing due on the named seat: ", what));
            _advanceAsReported(C, s, what);
        } else {
            _advanceAsReported(C, s, what);
        }
        uint64 latest = _latestSeat(C);
        if (_term(path) && !mustMove && _inFlight(venueOf(C).seatOf(C, d).state)) {
            // s2.11: the call left the venue's other clock alone, which is the venue's
            assertEq(latest, d, string.concat("s2.11 nothing moved and nothing was drawn: ", what));
            return;
        }
        assertTrue(_endOk(path, venueOf(C).seatOf(C, d).state), string.concat("s2.6, s2.11 the moved seat leaves flight as the path requires: ", what));
        _kept(pd, venueOf(C).seatOf(C, d), string.concat("s2.3 the seat that left flight keeps its window, accepted and range: ", what));
        if (!mustMove) {
            // the observed call: whether the venue draws again here is the venue's rule; a seat that follows is checked
            if (latest == d) return;
            assertEq(latest, d + 1, string.concat("s4 one seat follows: ", what));
            m = address(0);
        } else if (!usable) {
            assertEq(latest, d, string.concat("s4 no usable panel, no seat follows: ", what));
            return;
        }
        assertEq(latest, d + 1, string.concat("s4 the redraw opens the next ordinal: ", what));
        _commitInFlight(C, what);
        address j = venueOf(C).seatOf(C, latest).judge;
        assertEq(venueOf(C).seatOf(C, latest).amanahHash, _rec[_frozenOf[C] - 1].word, string.concat("s4 the redrawn seat carries the frozen panel's word: ", what));
        if (m == address(0)) {
            assertTrue(_in(j, _rec[_frozenOf[C] - 1].members) && !_in(j, _judgesOf(C, d)), string.concat("s4 a member of the frozen panel neither seated nor offered: ", what));
        } else {
            assertEq(j, m, string.concat("s4 the member the frozen panel's router names: ", what));
        }
        _ruleRedrawn(C, latest, string.concat("s2.12 a redrawn seat's ruling: ", what));
    }

    /// At every gas limit of the sweep, the call on `path` either reverts
    /// whole or moves the seat out of flight and seats `expected` next, and at
    /// some limit it does.
    function _sweepFire(bytes32 C, uint64 s, uint8 path, address expected) internal {
        _drain();
        IGaleedVenue v = venueOf(C);
        uint64 d = _due(s, path);
        (address from, bytes memory data) = _fireData(C, s, path);
        uint256 snap0 = vm.snapshotState();
        vm.prank(from, from);
        uint256 b = gasleft();
        (bool ok0, ) = address(v).call(data);
        uint256 used = b - gasleft();
        assertTrue(ok0, "the call is taken with ample gas");
        vm.revertToStateAndDelete(snap0);
        (uint256 lo, uint256 hi) = _window(used);
        uint256 landed = 0;
        for (uint256 g = lo; g <= hi; g += 5_000) {
            uint256 snap = vm.snapshotState();
            vm.prank(from, from);
            (bool ok, ) = address(v).call{gas: g}(data);
            if (ok && _term(path) && _inFlight(v.seatOf(C, d).state)) {
                // s2.11: a call naming a seat that left flight may leave the other clock alone
                try v.seatOf(C, d + 1) returns (IGaleedVenue.Seat memory) { fail("s4 a seat was drawn while the moved seat stayed in flight"); } catch {}
            } else if (ok) {
                landed++;
                assertTrue(_endOk(path, v.seatOf(C, d).state), "s2.11 a call taken on the path moves the seat out of flight");
                try v.seatOf(C, d + 1) returns (IGaleedVenue.Seat memory nx) {
                    assertEq(nx.judge, expected, "s4 at every gas limit the call reverts whole or the router answers under exactly routeGas()");
                } catch {
                    fail("s4 at a gas limit the call moved the seat and drew nobody");
                }
            }
            vm.revertToStateAndDelete(snap);
        }
        assertGt(landed, 0, "s4 the sweep moves the seat at some gas limit");
        // at the least gas limit the call is taken, and the limits just above it, one unit apart
        uint256 tlo = lo - 1;
        uint256 thi = hi;
        while (thi - tlo > 1) {
            uint256 mid = (tlo + thi) / 2;
            uint256 snap = vm.snapshotState();
            vm.prank(from, from);
            (bool okm, ) = address(v).call{gas: mid}(data);
            vm.revertToStateAndDelete(snap);
            if (okm) thi = mid; else tlo = mid;
        }
        for (uint256 g = thi > 50 ? thi - 50 : 0; g <= thi + 400; g++) {
            uint256 snap = vm.snapshotState();
            vm.prank(from, from);
            (bool ok, ) = address(v).call{gas: g}(data);
            if (ok && !(_term(path) && _inFlight(v.seatOf(C, d).state))) {
                assertTrue(_endOk(path, v.seatOf(C, d).state), "s2.11 a call taken by the threshold moves the seat out of flight");
                try v.seatOf(C, d + 1) returns (IGaleedVenue.Seat memory nx) {
                    assertEq(nx.judge, expected, "s4 at every gas limit by the threshold the call reverts whole or the router answers under exactly routeGas()");
                } catch {
                    fail("s4 at a gas limit by the threshold the call moved the seat and drew nobody");
                }
            }
            vm.revertToStateAndDelete(snap);
        }
        vm.getRecordedLogs();
    }

    /// Sends the call on `path` once on a case drawn from a three-member
    /// panel and reads whether a Seated followed. `possible` is false where
    /// the venue's own path never reaches the call (an offer that carries
    /// consent, a terminal path at a venue that never drew again).
    function _observe(IGaleedVenue face, bytes32 T, uint8 path) internal returns (bool possible, bool drew) {
        (bool p1, bool d1) = _observeOne(face, T, path, false);
        // s4: the call again where an unlawful draw differs from a lawful one: the frozen panel has
        // expired and a later panel, of a member and a word of its own, is recorded under the guild's root
        (bool p2, bool d2) = _observeOne(face, T, path, true);
        return (p1 || p2, d1 || d2);
    }

    function _observeOne(IGaleedVenue face, bytes32 T, uint8 path, bool adverse) internal returns (bool possible, bool drew) {
        if (_term(path) && _base == NO_PATH) return (false, false);
        string memory what = string.concat(_pathName(path), adverse ? "the observed call, the frozen panel expired under a later panel" : "the observed call");
        address[] memory trio = new address[](3); trio[0] = JUDGE3; trio[1] = JUDGE4; trio[2] = JUDGE5;
        uint64 soon = uint64(vm.getBlockTimestamp() + 1000);
        _engage(face, GUILD, T, trio, adverse ? soon : uint64(vm.getBlockTimestamp() + 3650 days), address(0), R_NONE, 0);
        _roll();
        (bytes32 C, uint64 s, bool ok) = _openExpect(GUILD, what);
        if (!ok || !_ready(C, s, path)) return (false, false);
        uint64 d = _due(s, path);
        if (adverse) {
            address[] memory imp = new address[](1); imp[0] = IMPOSTOR;
            _engage(face, GUILD, T, imp, uint64(vm.getBlockTimestamp() + 3650 days), address(0), R_NONE, 0);
            _roll();
            if (vm.getBlockTimestamp() <= soon) vm.warp(uint256(soon) + 1);
        }
        if (!_byJudge(path)) _pastWindow(C, d);
        _fireExpect(C, s, path, false, what);
        drew = _latestSeat(C) > d;
        if (drew && !_term(path)) {
            // the same call on the seat the redraw opened: a case's later redraws read the frozen panel too
            uint64 s1 = d + 1;
            if (_ready(C, s1, path)) {
                if (!_byJudge(path)) _pastWindow(C, s1);
                _fireExpect(C, s1, path, false, string.concat(what, ", on the seat a redraw opened"));
            }
        }
        return (true, drew);
    }

    /// s4 at every redraw on `path`: the panel the case froze serves after it
    /// expired and over a panel recorded later; the router's call at every
    /// gas limit; a router naming a member seated on the case, an index
    /// outside members, failing, or whose code changed after the creation
    /// leaves no usable panel; the redraw reads the panel under the root the
    /// case froze after the named address joined another record.
    function _redrawSuite(IGaleedVenue face, bytes32 T, uint8 path) internal {
        _redrawExpiryAndGas(face, T, path);
        _redrawRouters(face, T, path);
        if (opensCaseNaming()) _redrawRoot(face, T, path);
    }

    function _trio() internal pure returns (address[] memory trio) {
        trio = new address[](3); trio[0] = JUDGE; trio[1] = JUDGE3; trio[2] = JUDGE4;
    }

    function _redrawExpiryAndGas(IGaleedVenue face, bytes32 T, uint8 path) internal {
        string memory at = _pathName(path);
        address[] memory pe = new address[](3); pe[0] = JUDGE4; pe[1] = JUDGE5; pe[2] = JUDGE3;
        uint64 soon = uint64(vm.getBlockTimestamp() + 1000);
        _engage(face, GUILD, T, pe, soon, address(0), R_NONE, 0);
        _roll();
        vm.warp(soon);
        (bytes32 Cq, uint64 sq, bool okq) = _openExpect(GUILD, string.concat(at, "a panel at its expires second"));
        okq = okq && _ready(Cq, sq, path);
        address probe = _exactProbe();
        // four members, so that the probe's answer off routeGas() (the last) is never the member a redraw seats
        address[] memory quad = new address[](4); quad[0] = JUDGE; quad[1] = JUDGE3; quad[2] = JUDGE4; quad[3] = JUDGE5;
        _engage(face, GUILD, T, quad, uint64(vm.getBlockTimestamp() + 3650 days), probe, R_FIRST, 0);
        _roll();
        (bytes32 Cp, uint64 sp, bool okp) = _openExpect(GUILD, string.concat(at, "the router that answers by its gas, first draw"));
        okp = okp && _ready(Cp, sp, path);
        if (!_byJudge(path)) {
            if (okq) _pastWindow(Cq, _due(sq, path));
            if (okp) _pastWindow(Cp, _due(sp, path));
        }
        if (okq) _fireExpect(Cq, sq, path, true, string.concat(at, "the redraw reads the frozen panel after it expired, and no panel recorded later"));
        if (okp) {
            _sweepFire(Cp, sp, path, _term(path) ? JUDGE4 : JUDGE3);
            _fireExpect(Cp, sp, path, true, string.concat(at, "the redraw's router receives exactly routeGas()"));
        }
        // the call lands by two paths, the one that seats a member and the one a draw that finds no member
        // takes, each from its own least gas limit; a router that fails at once off routeGas() turns a call
        // short of it onto the second, which the probe above, answering, never reaches; so the same sweep
        // runs under such a router, and every limit from 50 below the second path's least limit to 400 above
        (bool nm, uint256 l0) = _noMemberLanding(face, T, path, quad);
        address gate = _exact(true);
        _engage(face, GUILD, T, quad, uint64(vm.getBlockTimestamp() + 3650 days), gate, R_FIRST, 0);
        _roll();
        (bytes32 Cg, uint64 sg, bool okg) = _openExpect(GUILD, string.concat(at, "the router that fails off routeGas(), first draw"));
        okg = okg && _ready(Cg, sg, path);
        if (okg && !_byJudge(path)) _pastWindow(Cg, _due(sg, path));
        if (okg) {
            _sweepFire(Cg, sg, path, _term(path) ? JUDGE4 : JUDGE3);
            if (nm) _scanFrom(Cg, sg, path, l0, _term(path) ? JUDGE4 : JUDGE3);
            _fireExpect(Cg, sg, path, true, string.concat(at, "the redraw's router that fails off routeGas() receives exactly routeGas()"));
        }
    }

    /// The least gas limit at which the call on `path` lands where its draw finds no member: a case over
    /// `members` whose router answers at the first draw and fails at the one the call makes, whatever its gas.
    function _noMemberLanding(IGaleedVenue face, bytes32 T, uint8 path, address[] memory members) internal returns (bool, uint256) {
        bool tm = _term(path);
        string memory at = _pathName(path);
        address r = tm ? address(new NthFailsRouter(2)) : address(new SecondFailsRouter());
        _engage(face, GUILD, T, members, uint64(vm.getBlockTimestamp() + 3650 days), r, R_SECOND, tm ? 2 : 0);
        _roll();
        (bytes32 C, uint64 s, bool ok) = _openExpect(GUILD, string.concat(at, "the router that fails at the redraw, first draw"));
        if (!(ok && _ready(C, s, path))) return (false, 0);
        if (!_byJudge(path)) _pastWindow(C, _due(s, path));
        _drain();
        (address from, bytes memory data) = _fireData(C, s, path);
        uint256 lo = 1_000;
        uint256 hi = 30_000_000;
        while (hi - lo > 1) {
            uint256 mid = (lo + hi) / 2;
            uint256 snap = vm.snapshotState();
            vm.prank(from, from);
            (bool okm, ) = address(venueOf(C)).call{gas: mid}(data);
            vm.revertToStateAndDelete(snap);
            if (okm) hi = mid; else lo = mid;
        }
        vm.getRecordedLogs();
        _fireExpect(C, s, path, true, string.concat(at, "a router failing at the redraw, whatever its gas, leaves no usable panel"));
        return (true, hi);
    }

    /// At every gas limit from 50 below `l0` to 400 above, the call on `path` reverts whole or moves the
    /// seat out of flight and seats `expected` next.
    function _scanFrom(bytes32 C, uint64 s, uint8 path, uint256 l0, address expected) internal {
        _drain();
        IGaleedVenue v = venueOf(C);
        uint64 d = _due(s, path);
        (address from, bytes memory data) = _fireData(C, s, path);
        for (uint256 g = l0 > 50 ? l0 - 50 : 0; g <= l0 + 400; g++) {
            uint256 snap = vm.snapshotState();
            vm.prank(from, from);
            (bool ok, ) = address(v).call{gas: g}(data);
            if (ok && !(_term(path) && _inFlight(v.seatOf(C, d).state))) {
                try v.seatOf(C, d + 1) returns (IGaleedVenue.Seat memory nx) {
                    assertEq(nx.judge, expected, "s4 at every gas limit by the no-member path's least limit the call reverts whole or the router answers under exactly routeGas()");
                } catch {
                    fail("s4 at a gas limit by the no-member path's least limit the call moved the seat and drew nobody");
                }
            }
            vm.revertToStateAndDelete(snap);
        }
        vm.getRecordedLogs();
    }

    function _badRouter(uint8 path, uint256 i, address fe) internal returns (address r, uint8 kind, uint256 k) {
        bool tm = _term(path);
        if (i == 0) return tm ? (address(new ParityRouter()), R_PARITY, 0) : (address(new FixedRouter(0)), R_FIXED, 0);
        if (i == 1) return tm ? (address(new NthOutOfRangeRouter(2)), R_SECOND, 2) : (address(new SecondOutOfRangeRouter()), R_SECOND, 0);
        if (i == 2) return tm ? (address(new NthFailsRouter(2)), R_SECOND, 2) : (address(new SecondFailsRouter()), R_SECOND, 0);
        return (fe, R_SECOND, tm ? 1 : 0);
    }

    function _redrawRouters(IGaleedVenue face, bytes32 T, uint8 path) internal {
        address fe = address(new FirstEligibleRouter());
        address feB = address(new FirstEligibleRouterB());
        string[4] memory whats = [
            "a router naming a member seated on the case leaves no usable panel",
            "a router naming an index outside members at a redraw leaves no usable panel",
            "a router failing at a redraw leaves no usable panel",
            "a router whose code changed after the creation leaves no usable panel"
        ];
        for (uint256 i = 0; i < 4; i++) {
            (address r, uint8 kind, uint256 k) = _badRouter(path, i, fe);
            _engage(face, GUILD, T, _trio(), uint64(vm.getBlockTimestamp() + 3650 days), r, kind, k);
            _roll();
            (bytes32 Cx, uint64 sx, bool okx) = _openExpect(GUILD, string.concat(_pathName(path), "the first draw"));
            if (!(okx && _ready(Cx, sx, path))) continue;
            if (r == fe) vm.etch(fe, feB.code);
            if (!_byJudge(path)) _pastWindow(Cx, _due(sx, path));
            _fireExpect(Cx, sx, path, true, string.concat(_pathName(path), whats[i]));
        }
        _redrawView(face, T, path);
    }

    /// s4: the view at a redraw carries what the venue holds, as at the
    /// creation: the venue, the chain, the time, the members and the seated,
    /// and the case's fields as seatOf reports them for the seat drawn.
    function _redrawView(IGaleedVenue face, bytes32 T, uint8 path) internal {
        string memory at = _pathName(path);
        uint64 far = uint64(vm.getBlockTimestamp() + 3650 days);
        address[] memory trio = _trio();
        _engage(face, GUILD, T, trio, far, address(new EchoRouter(trio)), R_FIRST, 0);
        _roll();
        (bytes32 Ce, uint64 se, bool oke) = _openExpect(GUILD, string.concat(at, "the first draw under the router that reads the view"));
        if (oke && _ready(Ce, se, path)) {
            if (!_byJudge(path)) _pastWindow(Ce, _due(se, path));
            _fireExpect(Ce, se, path, true, string.concat(at, "the redraw's view carries the venue, the chain, the time, the members and the seated"));
        }
        address[] memory sixteen = new address[](16);
        for (uint256 i = 0; i < 16; i++) sixteen[i] = address(uint160(0x1100000000000000000000000000000000000000) + uint160(i));
        _engage(face, GUILD, T, sixteen, far, address(new FieldsRouterB()), R_NONE, 0);
        _roll();
        (bytes32 Cf, uint64 sf, bool okf) = _openExpect(GUILD, string.concat(at, "the first draw under the router that reads the view's fields"));
        if (!(okf && _ready(Cf, sf, path))) return;
        uint64 d = _due(sf, path);
        if (!_byJudge(path)) _pastWindow(Cf, d);
        _viewIsTheVenues(Cf, sf, path, sixteen, at);
        _fireExpect(Cf, sf, path, true, string.concat(at, "the redraw under the router that reads the view's fields"));
        IGaleedVenue.Seat memory sv = venueOf(Cf).seatOf(Cf, d + 1);
        address[] memory taken = _judgesOf(Cf, d);
        uint256 h = uint256(keccak256(abi.encode(Cf, sv.termsHash, sv.token, sv.amount, sv.rungs, sv.parties))) % 16;
        uint256 k = 0;
        while (_in(sixteen[(h + k) % 16], taken)) k++;
        assertEq(sv.judge, sixteen[(h + k) % 16],
            string.concat(at, "s4 the redraw's view carries the caseId, the seat's termsHash, token, amount and rungs, and the parties as seatOf reports them"));
    }

    /// s4: the RouteView the venue hands the router at the redraw on `path`, read from the call itself
    /// (on a snapshot), field for field: what the venue holds, the fields as seatOf reports them for the seat drawn.
    function _viewIsTheVenues(bytes32 C, uint64 s, uint8 path, address[] memory members, string memory at) internal {
        _drain();
        uint256 sn = vm.snapshotState();
        IGaleedVenue v = venueOf(C);
        uint64 d = _due(s, path);
        address[] memory taken = _judgesOf(C, d);
        address router = address(0);
        (address from, bytes memory data) = _fireData(C, s, path);
        vm.startStateDiffRecording();
        vm.prank(from, from);
        (bool ok, ) = address(v).call(data);
        Vm.AccountAccess[] memory acc = vm.stopAndReturnStateDiff();
        assertTrue(ok, string.concat("s4 the call is taken: ", at));
        IGaleedVenue.Seat memory sv = v.seatOf(C, d + 1);
        uint256 seen = 0;
        for (uint256 i = 0; i < acc.length; i++) {
            bytes memory cd = acc[i].data;
            if (acc[i].accessor != address(v) || cd.length < 4 || bytes4(cd) != bytes4(keccak256("route((bytes32,uint64,bytes32,address,uint256,uint64,address[],address[],address[],bytes32,uint64,uint256,address))"))) continue;
            if (router == address(0)) router = acc[i].account;
            bytes memory args = new bytes(cd.length - 4);
            for (uint256 k = 0; k < args.length; k++) args[k] = cd[k + 4];
            RouteView memory rv = abi.decode(args, (RouteView));
            if (rv.seat != d + 1) continue;
            seen++;
            assertEq(rv.caseId, C, string.concat("s4 the redraw's view: caseId: ", at));
            assertEq(rv.termsHash, sv.termsHash, string.concat("s4 the redraw's view: the seat's termsHash: ", at));
            assertEq(rv.token, sv.token, string.concat("s4 the redraw's view: token: ", at));
            assertEq(rv.amount, sv.amount, string.concat("s4 the redraw's view: amount: ", at));
            assertEq(rv.rungs, sv.rungs, string.concat("s4 the redraw's view: rungs: ", at));
            assertEq(keccak256(abi.encode(rv.parties)), keccak256(abi.encode(sv.parties)), string.concat("s4 the redraw's view: the parties as seatOf reports them: ", at));
            assertEq(keccak256(abi.encode(rv.members)), keccak256(abi.encode(members)), string.concat("s4 the redraw's view: the panel's members: ", at));
            assertEq(keccak256(abi.encode(rv.seated)), keccak256(abi.encode(taken)), string.concat("s4 the redraw's view: every judge seated or offered: ", at));
            assertEq(rv.blockTime, uint64(vm.getBlockTimestamp()), string.concat("s4 the redraw's view: the block's time: ", at));
            assertEq(rv.chainId, block.chainid, string.concat("s4 the redraw's view: the chain: ", at));
            assertEq(rv.venue, address(v), string.concat("s4 the redraw's view: the venue: ", at));
        }
        assertGt(seen, 0, string.concat("s4 the redraw calls the panel's router: ", at));
        vm.revertToStateAndDelete(sn);
        vm.getRecordedLogs();
    }

    function _redrawRoot(IGaleedVenue face, bytes32 T, uint8 path) internal {
        string memory at = _pathName(path);
        // the terms name an address that is its own root; after the case froze it, it joins another record
        address X = address(uint160(0x2000000000000000000000000000000000000100) + path);
        address H = address(uint160(0x2000000000000000000000000000000000000200) + path);
        address[] memory imp = new address[](1); imp[0] = IMPOSTOR;
        uint64 far = uint64(vm.getBlockTimestamp() + 3650 days);
        _engage(face, X, T, _trio(), far, address(0), R_NONE, 0);
        _roll();
        (bytes32 Cr, uint64 sr, bool okr) = _openExpect(X, string.concat(at, "a guild named by an address that is its own root"));
        okr = okr && _ready(Cr, sr, path);
        GaleedSuccessionBook bk = _bk();
        vm.prank(H, H);
        bk.declare(X);
        vm.prank(X, X);
        bk.accede(H);
        assertEq(bk.root(X), H, "the named address now has another root");
        _engage(face, H, T, imp, far, address(0), R_NONE, 0);
        _roll();
        if (okr) {
            if (!_byJudge(path)) _pastWindow(Cr, _due(sr, path));
            _fireExpect(Cr, sr, path, true, string.concat(at, "the redraw reads the panel under the root the case froze"));
        }
    }

    // ---------------------------------------------------------------------
    // step 15: outcomes
    // ---------------------------------------------------------------------

    /// s2.12: the words of `r` and whether they are well formed.
    function _roleWords(string memory r) internal pure returns (bool ok, uint256 n) {
        bytes memory b = bytes(r);
        if (b.length == 0) return (true, 0);
        bool start = true;
        for (uint256 i = 0; i < b.length; i++) {
            bytes1 c = b[i];
            bool lower = c >= 0x61 && c <= 0x7a;
            bool digit = c >= 0x30 && c <= 0x39;
            if (start) {
                if (!lower) return (false, 0);
                start = false;
                n++;
            } else if (c == 0x2c) {
                start = true;
            } else if (!lower && !digit) {
                return (false, 0);
            }
        }
        if (start) return (false, 0);
        // s2.12: the words are distinct
        bytes32[] memory ws = new bytes32[](n);
        uint256 w = 0;
        uint256 from = 0;
        for (uint256 i = 0; i <= b.length; i++) {
            if (i == b.length || b[i] == 0x2c) {
                bytes memory x = new bytes(i - from);
                for (uint256 k = from; k < i; k++) x[k - from] = b[k];
                ws[w] = keccak256(x);
                for (uint256 k = 0; k < w; k++) if (ws[k] == ws[w]) return (false, 0);
                w++;
                from = i + 1;
            }
        }
        return (true, n);
    }

    function _shape(IGaleedVenue.Outcome memory o, uint256 amount, string memory what) internal view {
        assertEq(o.shares.length, roles, string.concat("s2.12 one share per role: ", what));
        assertEq(o.rulingFeeFrom.length, roles, string.concat("s2.12 one ruling-fee part per role: ", what));
        uint256 sum = 0;
        for (uint256 i = 0; i < o.rulingFeeFrom.length; i++) sum += o.rulingFeeFrom[i];
        assertLe(sum, o.rulingFee, string.concat("s2.12 the parts borne are no more than the ruling fee: ", what));
        sum = 0;
        for (uint256 i = 0; i < o.shares.length; i++) sum += o.shares[i];
        assertLe(sum, amount, string.concat("s2.12 the shares together are no more than the seat's amount: ", what));
    }

    /// s2.3: a terminal seat keeps the window, the accepted value and the range in force at the transition.
    /// s2.8: a seat of the case is in flight again, so a stranger's commitment is taken.
    function _commitInFlight(bytes32 C, string memory what) internal {
        vm.prank(STRANGER, STRANGER);
        venueOf(C).commit(C, M5);
        assertTrue(_committed(C, M5, STRANGER), string.concat("s2.8 commit is taken while the redrawn seat is in flight: ", what));
    }

    function _kept(IGaleedVenue.Seat memory a, IGaleedVenue.Seat memory b, string memory what) internal pure {
        assertEq(b.windowStart, a.windowStart, what);
        assertEq(b.windowEnd, a.windowEnd, what);
        assertEq(b.accepted, a.accepted, what);
        assertEq(b.rungMin, a.rungMin, what);
        assertEq(b.rungMax, a.rungMax, what);
    }

    /// What `outcomeOf` reported for each rung before acceptance, with the
    /// amount, the range and the time it was read at.
    mapping(uint64 => bytes32) internal _pred;
    uint256 internal _predAmount;
    uint64 internal _predMin;
    uint64 internal _predMax;
    uint256 internal _predTime;

    function _keepPredictions(bytes32 C, uint64 s, bytes memory extra) internal {
        IGaleedVenue.Seat memory st = venueOf(C).seatOf(C, s);
        if (st.rungs == 0 && !ladderDetermined(C, s)) { _predTime = type(uint256).max; return; }
        (_predAmount, _predMin, _predMax, _predTime) = (st.amount, st.rungMin, st.rungMax, vm.getBlockTimestamp());
        for (uint64 r = st.rungMin; r <= st.rungMax && r < st.rungMin + 64; r++) {
            _pred[r] = keccak256(abi.encode(venueOf(C).outcomeOf(C, s, r, extra)));
        }
    }

    /// s2.12: outcomeOf on a seat before acceptance takes the acceptance and
    /// the dispute as made, so where the amount, the range and the time are
    /// unchanged the disputed seat reports the same ruling for every rung.
    function _comparePredictions(bytes32 C, uint64 s, bytes memory extra) internal view {
        IGaleedVenue.Seat memory st = venueOf(C).seatOf(C, s);
        if (st.amount != _predAmount || st.rungMin != _predMin || st.rungMax != _predMax || vm.getBlockTimestamp() != _predTime) return;
        for (uint64 r = st.rungMin; r <= st.rungMax && r < st.rungMin + 64; r++) {
            assertEq(keccak256(abi.encode(venueOf(C).outcomeOf(C, s, r, extra))), _pred[r],
                "s2.12 outcomeOf before acceptance reports the ruling the disputed seat then reports");
        }
    }

    /// s2.12: each rung of the range, ruled on a snapshot, does what outcomeOf reported for it.
    function _everyRung(bytes32 C, uint64 s, bytes memory extra) internal {
        IGaleedVenue.Seat memory st = venueOf(C).seatOf(C, s);
        if (st.rungs == 0) return;
        _drain();
        for (uint64 r = st.rungMin; r <= st.rungMax && r < st.rungMin + 64; r++) {
            uint256 snap = vm.snapshotState();
            IGaleedVenue.Outcome memory o = venueOf(C).outcomeOf(C, s, r, extra);
            uint256[] memory rb = _received(C, st.token);
            Snap memory a = _snap(venueOf(C), C, s);
            vm.prank(JUDGE, JUDGE);
            venueOf(C).rule(C, s, E1, r, N1, extra);
            assertEq(venueOf(C).seatOf(C, s).state, o.state, "s2.12 each rung of the range: the state outcomeOf reported");
            _moved(a, _snap(venueOf(C), C, s), o, "s2.12 each rung of the range");
            _sharesPaid(C, st.token, rb, o, "s2.12 each rung of the range");
            _solvent("after each rung of the range");
            vm.revertToStateAndDelete(snap);
            // what the snapshot emitted belongs to no state the replay keeps
            vm.getRecordedLogs();
        }
    }

    /// Everything transferred in `tk` into the addresses standing in each role
    /// over the replay, after the venue's own path for them to take what it holds.
    function _received(bytes32 C, address tk) internal returns (uint256[] memory b) {
        b = new uint256[](roles);
        for (uint256 r = 0; r < roles; r++) {
            address[] memory xs = roleAddresses(C, r);
            for (uint256 i = 0; i < xs.length; i++) {
                (, uint256 pc) = venueOf(C).accountOf(xs[i], tk);
                assertEq(pc, 0, "s3 no party money passes through the economic face, read before the party collects");
                collect(C, xs[i]);
            }
        }
        _drain();
        for (uint256 r = 0; r < roles; r++) {
            address[] memory xs = roleAddresses(C, r);
            for (uint256 i = 0; i < xs.length; i++) {
                // read from the token's state, which a reverted frame leaves as it was; the recorder's
                // Transfer logs only where the token keeps no such count
                try MockERC20(tk).received(xs[i]) returns (uint256 got) { b[r] += got; } catch { b[r] += _inflow[tk][xs[i]]; }
            }
        }
    }

    /// s2.12: every measured role received, transferred to it or held for it,
    /// at least its element of `shares`: what flowed into its addresses, the
    /// venue's own path for taking what it holds for them included; what the
    /// action moves besides (a refund, or a charge the role pays) is outside
    /// the Outcome, and a charge is no receipt.
    function _sharesPaid(bytes32 C, address tk, uint256[] memory before, IGaleedVenue.Outcome memory o, string memory what) internal {
        uint256[] memory after_ = _received(C, tk);
        for (uint256 r = 0; r < roles; r++) {
            if (roleAddresses(C, r).length == 0) continue;
            assertGe(after_[r] - before[r], o.shares[r], string.concat("s2.12 each role receives its share: ", what));
        }
    }

    function _same(IGaleedVenue.Outcome memory a, IGaleedVenue.Outcome memory b) internal pure returns (bool) {
        return keccak256(abi.encode(a)) == keccak256(abi.encode(b));
    }

    /// s2.12: on a seat in flight with a ladder, outcomeOf answers for every rung in
    /// range and reverts outside it, and reverts for an extra of the wrong shape.
    function _outcomesInRange(bytes32 C, uint64 s, bytes memory extra, string memory what) internal {
        IGaleedVenue v = venueOf(C);
        IGaleedVenue.Seat memory st = v.seatOf(C, s);
        if (st.rungs == 0 && !ladderDetermined(C, s)) return;
        if (st.rungs > 0) {
            for (uint64 r = st.rungMin; r <= st.rungMax && r < st.rungMin + 64; r++) {
                IGaleedVenue.Outcome memory o = v.outcomeOf(C, s, r, extra);
                _shape(o, st.amount, what);
                assertEq(o.state, 2, string.concat("s2.12 a ruling's outcome is ruled: ", what));
                _sameFromEveryCaller(address(v), abi.encodeCall(IGaleedVenue.outcomeOf, (C, s, r, extra)), what);
            }
            _drain(); vm.expectRevert();
            v.outcomeOf(C, s, st.rungMax + 1, extra); vm.getRecordedLogs();
            if (st.rungMin > 0) {
                _drain(); vm.expectRevert();
                v.outcomeOf(C, s, st.rungMin - 1, extra); vm.getRecordedLogs();
            }
        } else {
            // any rung where rungs is zero
            for (uint64 r = 0; r < 8; r += 7) {
                IGaleedVenue.Outcome memory o = v.outcomeOf(C, s, r, extra);
                _shape(o, st.amount, what);
                assertEq(o.state, 2, string.concat("s2.12 where rungs is zero outcomeOf answers for any rung: ", what));
            }
        }
        if (bytes(v.extraType()).length == 0) {
            _drain(); vm.expectRevert();
            v.outcomeOf(C, s, st.rungMin, hex"01"); vm.getRecordedLogs();
        } else {
            _drain(); vm.expectRevert();
            v.outcomeOf(C, s, st.rungMin, ""); vm.getRecordedLogs();
            _drain(); vm.expectRevert();
            v.outcomeOf(C, s, st.rungMin, abi.encodePacked(extra, hex"00")); vm.getRecordedLogs();
            bytes[] memory dirty = _dirtyExtras(v.extraType(), extra);
            for (uint256 q = 0; q < dirty.length; q++) {
                _drain(); vm.expectRevert();
                v.outcomeOf(C, s, st.rungMin, dirty[q]); vm.getRecordedLogs();
            }
        }
    }

    /// s2.9: `extra` of the right length that is no encoding of the values
    /// `extraType()` names: for every word whose type is narrower than a word,
    /// the encoding with a bit outside its type there. A static array's elements
    /// and a static tuple's members are encoded in place, one word each; a
    /// dynamic type, or a token this reader does not know, yields none.
    function _dirtyExtras(string memory t, bytes memory extra) internal pure returns (bytes[] memory outs) {
        bytes memory b = bytes(t);
        if (b.length < 2 || b[0] != "(" || b[b.length - 1] != ")") return new bytes[](0);
        bytes[] memory tmp = new bytes[](extra.length / 32 + 1);
        uint256 n = 0;
        uint256 word = 0;
        uint256 i = 1;
        uint256 end = b.length - 1;
        while (i < end) {
            while (i < end && (b[i] == " " || b[i] == "," || b[i] == "(" || b[i] == ")")) i++;
            if (i >= end) break;
            uint256 j = i;
            while (j < end && b[j] != " " && b[j] != "," && b[j] != "[" && b[j] != "(" && b[j] != ")") j++;
            bytes memory ty = new bytes(j - i);
            for (uint256 k = 0; k < j - i; k++) ty[k] = b[i + k];
            uint256 count = 1;
            while (j < end && b[j] == "[") {
                uint256 k = j + 1;
                uint256 dim = 0;
                while (k < end && b[k] != "]") {
                    if (b[k] < "0" || b[k] > "9") return new bytes[](0);
                    dim = dim * 10 + uint8(b[k]) - 48;
                    k++;
                }
                if (k == j + 1 || k >= end) return new bytes[](0);
                count *= dim;
                j = k + 1;
            }
            // the field's name, if any
            while (j < end && b[j] == " ") j++;
            if (j < end && b[j] != "," && b[j] != ")") {
                while (j < end && b[j] != "," && b[j] != ")") j++;
            }
            (bool narrow, uint256 flip) = _flipOf(ty);
            if (!narrow && !_wordType(ty)) return new bytes[](0);
            for (uint256 c = 0; c < count; c++) {
                if (narrow && extra.length >= 32 * (word + 1) && n < tmp.length) {
                    bytes memory d = bytes.concat(extra);
                    uint256 w;
                    assembly { w := mload(add(add(d, 0x20), mul(word, 0x20))) }
                    w ^= flip;
                    assembly { mstore(add(add(d, 0x20), mul(word, 0x20)), w) }
                    tmp[n++] = d;
                }
                word++;
            }
            i = j;
        }
        outs = new bytes[](n);
        for (uint256 k = 0; k < n; k++) outs[k] = tmp[k];
    }

    /// a static elementary type one word wide that has no bit outside it
    function _wordType(bytes memory ty) internal pure returns (bool) {
        bytes32 h = keccak256(ty);
        return h == keccak256("uint256") || h == keccak256("int256") || h == keccak256("bytes32")
            || h == keccak256("uint") || h == keccak256("int");
    }

    function _flipOf(bytes memory ty) internal pure returns (bool, uint256) {
        bytes32 h = keccak256(ty);
        if (h == keccak256("bool")) return (true, 2);
        if (h == keccak256("address")) return (true, uint256(1) << 255);
        for (uint256 n = 8; n < 256; n += 8) {
            if (h == keccak256(bytes(string.concat("uint", vm.toString(n))))) return (true, uint256(1) << n);
            if (h == keccak256(bytes(string.concat("int", vm.toString(n))))) return (true, uint256(1) << 255 | uint256(1) << n);
        }
        for (uint256 n = 1; n < 32; n++) {
            if (h == keccak256(bytes(string.concat("bytes", vm.toString(n))))) return (true, 1);
        }
        return (false, 0);
    }

    /// The credited fields of a seat, its judge's claimable, and what the judge's payout holds.
    /// The credited fields of a seat, what the venue has credited on the
    /// judge's other seatings in the seat's token, the judge's claimable, and
    /// what the judge's payout holds.
    struct Snap {
        uint256 standby;
        uint256 ruling;
        uint256 others;
        uint256 claimable;
        uint256 paid;
    }

    function _snap(IGaleedVenue v, bytes32 C, uint64 s) internal returns (Snap memory n) {
        _register();
        IGaleedVenue.Seat memory st = v.seatOf(C, s);
        n.standby = st.standbyCredited;
        n.ruling = st.rulingCredited;
        for (uint256 i = 0; i < _credits.length; i++) {
            Seen storage e = _credits[i];
            if (e.v != address(v) || (e.c == C && e.s == s)) continue;
            IGaleedVenue.Seat memory ot = v.seatOf(e.c, e.s);
            if (ot.judge == st.judge && ot.token == st.token) n.others += ot.standbyCredited + ot.rulingCredited;
        }
        (address p, uint256 cl) = v.accountOf(st.judge, st.token);
        n.claimable = cl;
        n.paid = p == address(0) ? 0 : MockERC20(st.token).balanceOf(p);
    }

    /// s8.15: the credited fields rose by exactly the reported fees, and the
    /// claimable by their sum and what the transaction added to the judge's
    /// other seatings, less what the venue pushed to the payout.
    function _moved(Snap memory a, Snap memory b, IGaleedVenue.Outcome memory o, string memory what) internal pure {
        assertGe(b.standby, a.standby, string.concat("s2.3 standbyCredited never falls: ", what));
        assertGe(b.ruling, a.ruling, string.concat("s2.3 rulingCredited never falls: ", what));
        assertGe(b.others, a.others, string.concat("s2.3 no credited field of the judge's other seatings falls: ", what));
        assertEq(b.standby - a.standby, o.standbyFee, string.concat("s8.15 standbyCredited rises by the reported standbyFee: ", what));
        assertEq(b.ruling - a.ruling, o.rulingFee, string.concat("s8.15 rulingCredited rises by the reported rulingFee: ", what));
        assertEq(b.claimable + (b.paid - a.paid), a.claimable + o.standbyFee + o.rulingFee + (b.others - a.others),
            string.concat("s8.15 the claimable rises by the fees and the other seatings' credits, less what was pushed: ", what));
    }

    /// s7.18: whatever a transaction credits the seat's judge is counted on its seatings.
    function _creditsCounted(Snap memory a, Snap memory b, string memory what) internal pure {
        assertGe(b.standby, a.standby, what);
        assertGe(b.ruling, a.ruling, what);
        assertGe(b.others, a.others, what);
        assertEq(b.claimable + (b.paid - a.paid), a.claimable + (b.standby - a.standby) + (b.ruling - a.ruling) + (b.others - a.others), what);
    }

    /// s2.3, s2.12: a view answers by its arguments and the state
    /// alone, so a key that carries no code, the zero address (an eth_call's
    /// default sender) and a contract read one answer, and so does a read with
    /// an eth_call's gas under a chain's coinbase, randomness, base fee and gas price.
    function _sameFromEveryCaller(address v, bytes memory call, string memory what) internal {
        string memory m = string.concat("s2.12 a view answers alike whoever calls it: ", what);
        (bool ok0, bytes memory r0) = v.staticcall(call);
        address[3] memory callers = [JUDGE, address(0), address(fwd)];
        for (uint256 i = 0; i < 3; i++) {
            vm.prank(callers[i], callers[i]);
            (bool ok, bytes memory r) = v.staticcall(call);
            assertEq(ok, ok0, m);
            assertEq(keccak256(r), keccak256(r0), m);
        }
        (address cb0, uint256 r0n, uint256 f0) = (block.coinbase, block.prevrandao, block.basefee);
        uint256 g0 = tx.gasprice;
        vm.coinbase(address(0x00000000000000000000000000000000c0FFEE));
        vm.prevrandao(bytes32(r0n ^ 0x5ec4));
        vm.fee(f0 + 7 gwei);
        vm.txGasPrice(g0 + 8 gwei);
        vm.prank(JUDGE, JUDGE);
        (bool okc, bytes memory rc) = v.staticcall{gas: 30_000_000}(call);
        vm.coinbase(cb0);
        vm.prevrandao(bytes32(r0n));
        vm.fee(f0);
        vm.txGasPrice(g0);
        assertEq(okc, ok0, m);
        assertEq(keccak256(rc), keccak256(r0), m);
    }

    /// `advance` by a stranger, against what `advanceOf` reported on the state it found.
    function _advanceAsReported(bytes32 C, uint64 s, string memory what) internal {
        IGaleedVenue v = venueOf(C);
        _sameFromEveryCaller(address(v), abi.encodeCall(IGaleedVenue.advanceOf, (C, s)), what);
        _sameFromEveryCaller(address(v), abi.encodeCall(IGaleedVenue.seatOf, (C, s)), what);
        IGaleedVenue.Outcome memory o = v.advanceOf(C, s);
        IGaleedVenue.Seat memory st = v.seatOf(C, s);
        _shape(o, st.amount, what);
        // s2.3: only a ruling the seat gave credits a ruling fee
        assertEq(o.rulingFee, 0, string.concat("s2.3 an advance credits no ruling fee: ", what));
        uint256[] memory rb = _received(C, st.token);
        Snap memory a = _snap(v, C, s);
        vm.prank(STRANGER, STRANGER);
        v.advance(C, s);
        assertEq(v.seatOf(C, s).state, o.state, string.concat("s8.15 the state advanceOf reported: ", what));
        if (_inFlight(st.state)) {
            _moved(a, _snap(v, C, s), o, what);
        } else {
            // s2.12: on a seat that left flight the report is zero, and what the call
            // does on the venue's other clocks is outside it, counted on the seatings
            _creditsCounted(a, _snap(v, C, s), string.concat("s7.18 what an advance on a seat that left flight credits is counted on the judge's seatings: ", what));
        }
        _sharesPaid(C, st.token, rb, o, what);
    }

    function _nothingDue(bytes32 C, uint64 s, string memory what) internal view {
        IGaleedVenue v = venueOf(C);
        IGaleedVenue.Outcome memory o = v.advanceOf(C, s);
        _shape(o, v.seatOf(C, s).amount, what);
        assertEq(o.state, v.seatOf(C, s).state, string.concat("s8.15 the present state: ", what));
        assertEq(o.standbyFee + o.rulingFee, 0, string.concat("s8.15 zero amounts: ", what));
        for (uint256 i = 0; i < o.shares.length; i++) {
            assertEq(o.shares[i] + o.rulingFeeFrom[i], 0, string.concat("s8.15 zero amounts: ", what));
        }
    }

    /// Every seat the venues seated, with the credited fields last seen.
    struct Seen {
        address v;
        bytes32 c;
        uint64 s;
        uint256 standby;
        uint256 ruling;
    }

    Seen[] internal _credits;
    mapping(bytes32 => uint256) internal _creditAt;

    /// the position in `_seen` up to which `_register` has read
    uint256 internal _registered;

    /// Adds every seat a venue has Seated since the last call to `_credits`.
    function _register() internal {
        _drain();
        for (uint256 i = _registered; i < _seen.length; i++) {
            Vm.Log memory l = _seen[i];
            if (!_isVenue(l.emitter) || l.topics.length != 3 || l.topics[0] != SEATED_SIG) continue;
            uint64 sv = abi.decode(l.data, (uint64));
            bytes32 key = keccak256(abi.encode(l.emitter, l.topics[2], sv));
            if (_creditAt[key] == 0) {
                _credits.push(Seen(l.emitter, l.topics[2], sv, 0, 0));
                _creditAt[key] = _credits.length;
                assertEq(IGaleedVenue(l.emitter).seatOf(l.topics[2], sv).judge, address(uint160(uint256(l.topics[1]))),
                    "s7.2 every Seated carries the seat's judge as its first topic");
                // s4, whatever call made the seat: the frozen panel's word, a member of it, one no earlier seat holds
                bytes32 cc = l.topics[2];
                if (_frozenOf[cc] != 0) {
                    RPanel storage fp = _rec[_frozenOf[cc] - 1];
                    IGaleedVenue.Seat memory ns = IGaleedVenue(l.emitter).seatOf(cc, sv);
                    assertEq(ns.amanahHash, fp.word, "s4 every seat of a drawn case carries its frozen panel's word");
                    assertTrue(_in(ns.judge, fp.members), "s4 every seat of a drawn case is a member of its frozen panel");
                    for (uint64 q = 0; q < sv; q++) {
                        assertTrue(IGaleedVenue(l.emitter).seatOf(cc, q).judge != ns.judge, "s4 no draw seats a member seated or offered on the case");
                    }
                }
            }
        }
        _registered = _seen.length;
    }

    /// s8.15: no seat's credited field falls between two reads.
    function _creditScan() internal {
        _register();
        for (uint256 i = 0; i < _credits.length; i++) {
            Seen storage e = _credits[i];
            IGaleedVenue.Seat memory st = IGaleedVenue(e.v).seatOf(e.c, e.s);
            assertGe(st.standbyCredited, e.standby, "s8.15 no standbyCredited falls over the replay");
            assertGe(st.rulingCredited, e.ruling, "s8.15 no rulingCredited falls over the replay");
            e.standby = st.standbyCredited;
            e.ruling = st.rulingCredited;
            if (st.state == 4) assertTrue(st.accepted != 0, "s2.6, s2.7 lapsed only on a seat whose ruling was due");
            if (!_inFlight(st.state)) _nothingDue(e.c, e.s, "s2.12 nothing due on a seat that left flight");
        }
    }

    /// s7.18: for every judge the replay seated, every amount credited in the
    /// fee token is counted on its seatings: their credited fields sum to what
    /// its payout received and what it can still claim, where every seating
    /// of the judge settles in the fee token. Only J ever withdrew, to P.
    function _creditAccount() internal view {
        for (uint256 i = 0; i < _credits.length; i++) {
            address j = IGaleedVenue(_credits[i].v).seatOf(_credits[i].c, _credits[i].s).judge;
            bool earlier = false;
            for (uint256 k = 0; k < i; k++) {
                if (IGaleedVenue(_credits[k].v).seatOf(_credits[k].c, _credits[k].s).judge == j) earlier = true;
            }
            if (earlier) continue;
            uint256 credited = 0;
            bool other = false;
            for (uint256 k = 0; k < _credits.length; k++) {
                IGaleedVenue.Seat memory st = IGaleedVenue(_credits[k].v).seatOf(_credits[k].c, _credits[k].s);
                if (st.judge != j) continue;
                if (st.token != feeToken()) other = true;
                credited += st.standbyCredited + st.rulingCredited;
            }
            if (other) continue;
            uint256 claimable = 0;
            for (uint256 k = 0; k < _venues.length; k++) {
                (, uint256 cl) = IGaleedVenue(_venues[k]).accountOf(j, feeToken());
                claimable += cl;
            }
            uint256 paid = j == JUDGE ? MockERC20(feeToken()).balanceOf(PAYOUT) : 0;
            assertEq(credited, paid + claimable, "s7.18 every amount credited to a judge is counted on its seatings");
        }
    }

    mapping(bytes32 => uint256) internal _seatedTimes;

    /// Over the whole replay: every (venue, caseId, seat) is Seated once
    /// (s2.2); a seat that gave no ruling carries no ruling fee (s2.3); no
    /// party's money is claimable through the economic face (s3).
    function _wholeReplay() internal {
        _drain();
        for (uint256 i = 0; i < _seen.length; i++) {
            Vm.Log memory l = _seen[i];
            if (!_isVenue(l.emitter) || l.topics.length != 3 || l.topics[0] != SEATED_SIG) continue;
            bytes32 key = keccak256(abi.encode(l.emitter, l.topics[2], abi.decode(l.data, (uint64))));
            _seatedTimes[key] += 1;
            assertEq(_seatedTimes[key], 1, "s2.2 Seated exactly once per (caseId, seat)");
        }
        for (uint256 i = 0; i < _credits.length; i++) {
            Seen storage e = _credits[i];
            IGaleedVenue.Seat memory st = IGaleedVenue(e.v).seatOf(e.c, e.s);
            if (st.state != 2) assertEq(st.rulingCredited, 0, "s2.3 a seat that gave no ruling carries no ruling fee");
            // s3: only JUDGE ever called setPayout or withdraw; no other judge has a payout
            if (st.judge != JUDGE) {
                (address pj, ) = IGaleedVenue(e.v).accountOf(st.judge, feeToken());
                assertEq(pj, address(0), "s3 no payout is registered by any path but setPayout and withdraw");
            }
            for (uint256 k = 0; k < st.parties.length; k++) {
                (, uint256 cl) = IGaleedVenue(e.v).accountOf(st.parties[k], st.token);
                assertEq(cl, 0, "s3 no party money passes through the economic face");
            }
        }
    }

    // ---------------------------------------------------------------------
    // log readers: every fact here is read from the interface's own events
    // ---------------------------------------------------------------------

    Vm.Log[] internal _seen;

    /// Drains the recorder into `_seen` without losing earlier logs, then
    /// re-arms it. Foundry's getRecordedLogs clears the buffer.
    function _drain() internal {
        Vm.Log[] memory logs = vm.getRecordedLogs();
        for (uint256 i = 0; i < logs.length; i++) {
            _seen.push(logs[i]);
            Vm.Log memory l = logs[i];
            // every ERC-20 transfer into an address, for what a role receives (s2.12)
            if (l.topics.length == 3 && l.topics[0] == TRANSFER_SIG && l.topics[1] != l.topics[2] && l.data.length == 32) {
                _inflow[l.emitter][address(uint160(uint256(l.topics[2])))] += abi.decode(l.data, (uint256));
            }
        }
    }

    bytes32 internal constant TRANSFER_SIG = keccak256("Transfer(address,address,uint256)");
    /// token => address => everything transferred into the address over the replay
    mapping(address => mapping(address => uint256)) internal _inflow;

    function _seatedCount(bytes32 caseId, uint64 seat, address judge) internal returns (uint256 n) {
        _drain();
        for (uint256 i = 0; i < _seen.length; i++) {
            Vm.Log memory l = _seen[i];
            if (!_isVenue(l.emitter) || l.topics.length != 3 || l.topics[0] != SEATED_SIG) continue;
            if (address(uint160(uint256(l.topics[1]))) != judge || l.topics[2] != caseId) continue;
            if (abi.decode(l.data, (uint64)) == seat) n++;
        }
    }

    function _latestSeat(bytes32 caseId) internal returns (uint64 latest) {
        _drain();
        bool any_;
        for (uint256 i = 0; i < _seen.length; i++) {
            Vm.Log memory l = _seen[i];
            if (!_isVenue(l.emitter) || l.topics.length != 3 || l.topics[0] != SEATED_SIG || l.topics[2] != caseId) continue;
            uint64 sv = abi.decode(l.data, (uint64));
            if (!any_ || sv > latest) { latest = sv; any_ = true; }
        }
        assertTrue(any_, "no Seated for the case");
    }

    function _committed(bytes32 caseId, bytes32 manifest, address by) internal returns (bool) {
        _drain();
        for (uint256 i = 0; i < _seen.length; i++) {
            Vm.Log memory l = _seen[i];
            if (!_isVenue(l.emitter) || l.topics.length != 3 || l.topics[0] != COMMITTED_SIG || l.topics[1] != caseId) continue;
            if (l.topics[2] != bytes32(uint256(uint160(by)))) continue;
            if (abi.decode(l.data, (bytes32)) == manifest) return true;
        }
        return false;
    }

    function _anchoredCount() internal returns (uint256 n) {
        _drain();
        for (uint256 i = 0; i < _seen.length; i++) {
            Vm.Log memory l = _seen[i];
            if (_isVenue(l.emitter) && l.topics.length > 0 && l.topics[0] == ANCHORED_SIG) n++;
        }
    }

    /// The Anchored log at ordinal `k` (zero-based, in emission order): three
    /// topics, the sender, the hash, and data that is the ABI encoding of the
    /// entry bytes (the form of galeed/1 s9.1).
    function _anchoredIs(uint256 k, address sender, bytes32 hash, bytes memory entry) internal returns (bool) {
        _drain();
        uint256 n = 0;
        for (uint256 i = 0; i < _seen.length; i++) {
            Vm.Log memory l = _seen[i];
            if (!_isVenue(l.emitter) || l.topics.length == 0 || l.topics[0] != ANCHORED_SIG) continue;
            if (n == k) {
                return l.topics.length == 3
                    && keccak256(l.data) == keccak256(abi.encode(entry))
                    && address(uint160(uint256(l.topics[1]))) == sender && l.topics[2] == hash;
            }
            n++;
        }
        return false;
    }

    function _inFlight(uint8 state) internal pure returns (bool) { return state == 0 || state == 1; }

    /// `data` from `from` to `target` is refused. A refused call emits nothing on chain;
    /// forge records what its frames emitted, so the kept logs are drained first and the
    /// refused call's discarded.
    function _refuse(address from, address target, bytes memory data, string memory what) internal {
        _drain();
        vm.prank(from, from);
        (bool ok, ) = target.call(data);
        assertFalse(ok, what);
        vm.getRecordedLogs();
    }

    function _withCode(address who) internal { vm.etch(who, abi.encodePacked(hex"ef0100", address(fwd))); }
    function _withoutCode(address who) internal { vm.etch(who, ""); }
}
