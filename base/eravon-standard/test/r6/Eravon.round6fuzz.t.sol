// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {EravonBase} from "../EravonBase.sol";
import {EravonVerifier} from "../../src/EravonVerifier.sol";
import {IGaleedVenue} from "../../src/IGaleedVenue.sol";

/// The terminal split as the logic computed it before the 2026-10-18 move (round 5's `_finish`, Court ebcd2eb5,
/// lines 1109-1127), word for word, the strays added where that code added them.
contract R6OldSplit {
    function old(
        uint256 a,
        uint16 buyerBps,
        uint256 stray,
        uint256 premium,
        uint256 deposit,
        uint256 lfHeld,
        bool pToJudge,
        bool lfToJudge,
        uint8 bondTo
    ) external pure returns (uint256 buyerPaid, uint256 sellerA, uint256 judgePaid, uint256 toSellerKey, uint256 bondToBuyer) {
        uint256 buyerA = (a * buyerBps) / 10000;
        sellerA = a - buyerA;
        judgePaid = 0;
        toSellerKey = 0;
        buyerPaid = buyerA + stray;
        if (pToJudge) judgePaid += premium;
        else toSellerKey += premium;
        if (lfHeld != 0) {
            if (lfToJudge) judgePaid += lfHeld;
            else buyerPaid += lfHeld;
        }
        bondToBuyer = 0;
        uint256 sellerLf = deposit - premium;
        if (bondTo == 1) bondToBuyer = sellerLf;
        else if (bondTo == 2) bondToBuyer = sellerLf / 2;
        buyerPaid += bondToBuyer;
        toSellerKey += sellerLf - bondToBuyer;
    }
}

/// Round 6, stateless: the verifier's `split` (standard s8 on the verifier since 2026-10-18) against the old in-logic
/// arithmetic over every input, reverts included, and against s8's terminal table row by row.
contract EravonRound6Fuzz is EravonBase {
    R6OldSplit internal o;

    function setUp() public {
        setUpBase();
        o = new R6OldSplit();
    }

    struct Out {
        bool ok;
        uint256 b;
        uint256 s;
        uint256 j;
        uint256 f;
        uint256 bond;
    }

    function _new(uint256 a, uint16 bps, uint256 stray, uint256 p, uint256 d, uint256 lfh, bool pj, bool lj, uint8 bt)
        internal
        view
        returns (Out memory r)
    {
        try verifier.split(a, bps, p, d, lfh, pj, lj, bt, 0) returns (uint256 b, uint256 s, uint256 j, uint256 f, uint256 bond, IGaleedVenue.Outcome memory) {
            // the instance adds the strays after the call, checked
            unchecked {
                if (b + stray < b) return r;
            }
            r = Out(true, b + stray, s, j, f, bond);
        } catch {}
    }

    function _old(uint256 a, uint16 bps, uint256 stray, uint256 p, uint256 d, uint256 lfh, bool pj, bool lj, uint8 bt)
        internal
        view
        returns (Out memory r)
    {
        try o.old(a, bps, stray, p, d, lfh, pj, lj, bt) returns (uint256 b, uint256 s, uint256 j, uint256 f, uint256 bond) {
            r = Out(true, b, s, j, f, bond);
        } catch {}
    }

    /// Every input, the unreachable ones included (an amount past guard 6, bps past 10000, P above D, sums that
    /// overflow, every bondTo byte): the two computations succeed together and agree to the wei, or fail together,
    /// save the one input the outcome adds a refusal for (a held fee below the bond on a ruling).
    function testFuzz_R6_splitIsTheOldArithmetic(
        uint256 a,
        uint16 bps,
        uint256 stray,
        uint256 p,
        uint256 d,
        uint256 lfh,
        uint256 flags
    ) public view {
        // half the runs in the range an instance reaches, so that success is exercised as often as failure
        if (flags & 1 == 0) {
            a = bound(a, 0, type(uint256).max / 10000);
            bps = uint16(bound(bps, 0, 10000));
            p = (a * uint256(uint16(bound(p, 0, 10000)))) / 10000;
            d = p + bound(d, 0, type(uint256).max / 4 - p);
            lfh = bound(lfh, 0, type(uint256).max / 4);
            stray = bound(stray, 0, type(uint256).max / 4);
        }
        bool pj = (flags >> 1) & 1 == 1;
        bool lj = (flags >> 2) & 1 == 1;
        uint8 bt = (flags >> 3) % 4 == 3 ? uint8(flags >> 8) : uint8((flags >> 3) % 4);
        Out memory n = _new(a, bps, stray, p, d, lfh, pj, lj, bt);
        Out memory x = _old(a, bps, stray, p, d, lfh, pj, lj, bt);
        // the outcome's rulingFeeFrom splits the fee the judge takes between the two sides (venue/1 s2.10), so the
        // new split refuses a held fee below the bond the seller's LF pays the buyer; an instance holds lfHeld =
        // D - P on every ruling, so that input lies outside every state a case reaches
        uint256 bond = d < p ? 0 : (bt == 1 ? d - p : (bt == 2 ? (d - p) / 2 : 0));
        if (x.ok && lj && lfh < bond) {
            assertFalse(n.ok, "the new split refuses a held fee below the bond");
            return;
        }
        assertEq(n.ok, x.ok, "succeed or fail together");
        assertEq(n.b, x.b, "buyerPaid");
        assertEq(n.s, x.s, "sellerA");
        assertEq(n.j, x.j, "judgePaid");
        assertEq(n.f, x.f, "to sellerFund (the old toSellerKey)");
        assertEq(n.bond, x.bond, "bondToBuyer");
    }

    // ---- s8's table, row by row ----

    /// The rows of s8's table that a case reaches, with the flags the instance hands `split` for each (the call
    /// sites of `_finish`): 0 Refunded cause 0 or 1, 1 Refunded cause 3 while Claimed under a seated seat, 2
    /// Refunded cause 3 otherwise, 3 Released seated, 4 Released after the seat resigned, 5 Settled with P to the
    /// judge, 6 Settled otherwise, 7 to 9 Ruled at a rung of feeBearer 0, 1, 2, 10 Resigned, 11 Lapsed.
    function _flags(uint8 row) internal pure returns (bool pj, bool lj, uint8 bt) {
        if (row == 1 || row == 3 || row == 5) pj = true;
        if (row >= 7 && row <= 9) {
            pj = true;
            lj = true;
            bt = row - 7;
        }
    }

    /// What the text's table gives each side for that row, written from the table and not from the code: the
    /// buyer's share of A is floor(A x bps / 10000) and the seller's the rest; P to the judge where the P column says
    /// judge and back to sellerFund otherwise; the buyer's fee to the judge on Ruled and back to the buyer wherever it
    /// was pulled otherwise; the seller's LF back to sellerFund except on Ruled, where feeBearer 1 sends it to the
    /// buyer, 2 sends floor(LF / 2) to the buyer and the rest back, 0 returns it; the strays to the buyer.
    function _table(uint8 row, uint256 a, uint16 bps, uint256 p, uint256 lf, uint256 fee, uint256 stray)
        internal
        pure
        returns (Out memory w)
    {
        bool pJudge = row == 1 || row == 3 || row == 5 || (row >= 7 && row <= 9);
        bool ruled = row >= 7 && row <= 9;
        uint256 bond = row == 8 ? lf : (row == 9 ? lf / 2 : 0);
        uint256 ba = (a * bps) / 10000;
        w.ok = true;
        w.b = ba + stray + (ruled ? 0 : fee) + bond;
        w.s = a - ba;
        w.j = (pJudge ? p : 0) + (ruled ? fee : 0);
        w.f = (pJudge ? 0 : p) + (lf - bond);
        w.bond = bond;
    }

    /// Each row, over the amounts a case reaches (A within guard 6, P = floor(A x premiumBps / 10000), LF any sum that
    /// leaves D = P + LF in 256 bits, the buyer's fee held or not, strays): the verifier's split plus the strays is the
    /// table's row, and the old arithmetic is the same.
    function testFuzz_R6_splitEveryRowOfTheTable(uint8 row, uint256 a, uint16 prem, uint256 lf, uint16 bps, uint256 flags, uint256 stray)
        public
        view
    {
        row = uint8(bound(row, 0, 11));
        a = bound(a, 1, type(uint256).max / 10000);
        prem = uint16(bound(prem, 0, 10000));
        uint256 p = (a * prem) / 10000;
        lf = bound(lf, 0, (type(uint256).max - p) / 4);
        stray = bound(stray, 0, type(uint256).max / 4);
        // the rows' buyerBps: 10000 at a refund or a resignation, 0 at a release, any signed or rung value otherwise
        if (row <= 2 || row == 10) bps = 10000;
        else if (row == 3 || row == 4) bps = 0;
        else bps = uint16(bound(bps, 0, 10000));
        // the buyer's fee is held where a dispute was raised: always on Ruled, never at a release, a T1 or offer-clock
        // refund, a refund while Claimed under a seated seat, or a resignation; either way otherwise
        bool held = (flags & 1 == 1);
        if (row >= 7 && row <= 9) held = true;
        if (row == 0 || row == 1 || row == 3 || row == 4 || row == 10) held = false;
        uint256 fee = held ? lf : 0;
        (bool pj, bool lj, uint8 bt) = _flags(row);
        Out memory n = _new(a, bps, stray, p, p + lf, fee, pj, lj, bt);
        Out memory x = _old(a, bps, stray, p, p + lf, fee, pj, lj, bt);
        Out memory w = _table(row, a, bps, p, lf, fee, stray);
        assertTrue(n.ok && x.ok, "a reachable row succeeds");
        assertEq(n.b, w.b, "buyer");
        assertEq(n.s, w.s, "seller's share of A");
        assertEq(n.j, w.j, "judge");
        assertEq(n.f, w.f, "back to sellerFund");
        assertEq(n.bond, w.bond, "bondToBuyer");
        assertEq(x.b, w.b);
        assertEq(x.s, w.s);
        assertEq(x.j, w.j);
        assertEq(x.f, w.f);
        assertEq(x.bond, w.bond);
        // nothing created or lost: what the instance held for the case is what the five columns pay
        assertEq(n.b + n.s + n.j + n.f, a + p + lf + fee + stray, "the split pays exactly A, D, the fee and the strays");
    }
}
