// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "../EravonBase.sol";
import {Eravon} from "../../src/Eravon.sol";
import {EravonVerifier} from "../../src/EravonVerifier.sol";
import {R5Token, R5Feed} from "../mocks/InvMocks.sol";

/// The verifier's internal decoders and pricing, exposed (the deployed verifier is untouched).
contract R5Harness is EravonVerifier {
    function termsFaultX(Eravon.OrderTerms calldata t) external pure returns (uint8) {
        return _termsFault(t);
    }

    function priceX(uint256[15] memory q, uint256 amount, address token) external view returns (Checked memory c) {
        c.q = q;
        _price(c, amount, token);
    }
}

/// Round 5, seat FUZZ: stateless properties against models built apart from the verifier's code: `_termsFault` with
/// two to eight of s4's guards broken by construction, the expected fault being the table's first broken row (not a
/// re-evaluation of the guards); and s3's pricing under odd decimals() against a model that converts by long
/// division.
contract EravonRound5Fuzz is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    uint256 internal constant MAX = type(uint256).max;
    uint256 internal constant MAX_INT53 = 9007199254740991;
    R5Harness internal hx;

    function setUp() public {
        setUpBase();
        hx = new R5Harness();
    }

    // ==================================================================
    // 1. _termsFault: several guards broken by construction
    // ==================================================================

    /// The table's rows in the order s4 runs them, rows 8 and 9 standing for the rung loop.
    function _pos(uint8 row) internal pure returns (uint256) {
        uint8[16] memory order = [1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12, 10, 13, 14, 15, 16];
        for (uint256 k = 0; k < 16; k++) {
            if (order[k] == row) return k;
        }
        return 99;
    }

    function _ladderOf(uint256 n, uint256 w) internal pure returns (Eravon.Rung[] memory l) {
        l = new Eravon.Rung[](n);
        for (uint256 i = 0; i < n; i++) {
            uint16 b = n == 1 ? 0 : (i == n - 1 ? 10000 : uint16((10000 * i) / (n - 1)));
            l[i] = Eravon.Rung(b, 10000 - b, uint8((w >> (8 + i)) % 3));
        }
    }

    /// Two to eight rows of s4's table broken, each by a change that breaks its own row and no row before it; the
    /// verifier names the first broken row in the table's order, rows 8 and 9 by the lowest rung broken (8 before 9
    /// on one rung).
    function testFuzz_R5_termsFaultIsTheFirstBrokenRow(uint256 s, uint256 w) public view {
        Eravon.OrderTerms memory t;
        Eravon.Offer memory o = t.offer;
        o.sellerKey = SELLER;
        o.sellerFund = SELLER;
        t.buyerKey = BUYER;
        o.judge = JUDGE;
        o.sellerPayout = address(0x5E11E7);
        t.buyerPayout = address(0xB0B7);
        o.token = address(tok);
        o.amount = 1 ether;
        uint256 n = 2 + (w % 15);
        o.ladder = _ladderOf(n, w);
        o.termsHash = keccak256("t");
        o.amanahHash = keccak256("a");
        o.quoteHash = keccak256("q");
        o.nonce = MAX_INT53;
        o.expiry = MAX_INT53;
        t.openBy = uint64(MAX_INT53);
        // which rows to break: k of the sixteen, k from 2 to 8
        uint256 h = uint256(keccak256(abi.encode(s, "rows")));
        uint256 k = 2 + h % 7;
        bool[17] memory br;
        uint8[16] memory rows = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        for (uint256 x = 0; x < k; x++) {
            br[rows[uint256(keccak256(abi.encode(h, x))) % 16]] = true;
        }
        // row 7: a ladder of 1 or 17 rungs (otherwise sound); the rung rows then act on that ladder where it has rungs
        if (br[7]) {
            n = (h >> 8) % 2 == 0 ? 1 : 17;
            o.ladder = _ladderOf(n, w);
        }
        uint256 r8 = n == 0 ? 0 : (h >> 16) % n;
        uint256 r9 = n == 0 ? 0 : (h >> 24) % n;
        if (n < 2) {
            // a one-rung ladder: the rung rows are not reached, row 7 comes first
            br[8] = br[9] = br[10] = br[11] = br[12] = false;
        }
        if (n < 4) br[10] = false; // an interior rung to move needs four rungs
        if (br[1]) o.sellerKey = (h >> 32) % 2 == 0 ? address(0) : BUYER;
        if (br[2]) o.guild = GUILD; // both set
        if (br[3]) o.judge = BUYER; // the buyer's own key
        if (br[4]) {
            if ((h >> 34) % 2 == 0) t.buyerPayout = address(0);
            else o.sellerPayout = address(0);
        }
        if (br[5]) o.token = address(0);
        if (br[6]) o.amount = (h >> 36) % 2 == 0 ? 0 : MAX / 10000 + 1;
        if (br[11]) {
            o.ladder[0].buyerBps = 1;
            o.ladder[0].sellerBps = 9999;
        }
        if (br[12]) {
            o.ladder[n - 1].buyerBps = 9999;
            o.ladder[n - 1].sellerBps = 1;
        }
        uint256 j10 = n >= 4 ? 2 + (h >> 40) % (n - 3) : 0; // in [2, n - 2]
        if (br[10]) {
            o.ladder[j10].buyerBps = o.ladder[j10 - 1].buyerBps - 1;
            o.ladder[j10].sellerBps = 10000 - o.ladder[j10].buyerBps;
        }
        if (br[8]) o.ladder[r8].sellerBps += 1;
        if (br[9]) o.ladder[r9].feeBearer = 3;
        if (br[13]) o.termsHash = 0;
        if (br[14]) o.amanahHash = 0;
        if (br[15]) o.quoteHash = 0;
        if (br[16]) {
            uint256 which = (h >> 48) % 3;
            if (which == 0) o.nonce = MAX_INT53 + 1;
            else if (which == 1) o.expiry = MAX_INT53 + 1;
            else t.openBy = uint64(MAX_INT53 + 1);
        }
        t.offer = o;
        // the expectation, from the rows broken alone
        uint8 want = 0;
        uint256 best = 99;
        for (uint8 row = 1; row <= 16; row++) {
            if (!br[row] || row == 8 || row == 9) continue;
            if (_pos(row) < best) {
                best = _pos(row);
                want = row;
            }
        }
        if ((br[8] || br[9]) && _pos(8) < best) {
            // the rung loop: the lowest rung broken, 8 before 9 on that rung
            uint256 r = MAX;
            uint8 rr;
            if (br[8]) {
                r = r8;
                rr = 8;
            }
            if (br[9] && r9 < r) {
                r = r9;
                rr = 9;
            }
            want = rr;
        }
        assertEq(hx.termsFaultX(t), want, "the first broken row of s4's table");
    }

    // ==================================================================
    // 3. s3's pricing under odd decimals(), against a long-division model
    // ==================================================================

    function _m512(uint256 x, uint256 y) internal pure returns (uint256 hi, uint256 lo) {
        uint256 M = type(uint128).max;
        uint256 p00 = (x & M) * (y & M);
        uint256 p01 = (x & M) * (y >> 128);
        uint256 p10 = (x >> 128) * (y & M);
        uint256 p11 = (x >> 128) * (y >> 128);
        uint256 mid = (p00 >> 128) + (p01 & M) + (p10 & M);
        lo = (p00 & M) | ((mid & M) << 128);
        hi = p11 + (p01 >> 128) + (p10 >> 128) + (mid >> 128);
    }

    function _ceil(uint256 x, uint256 y, uint256 d) internal pure returns (bool, uint256) {
        (uint256 hi, uint256 lo) = _m512(x, y);
        uint256 r;
        uint256 qhi;
        uint256 qlo;
        unchecked {
            for (uint256 k = 512; k > 0; k--) {
                uint256 bit = k - 1 >= 256 ? (hi >> (k - 257)) & 1 : (lo >> (k - 1)) & 1;
                bool carry = r >> 255 == 1;
                r = (r << 1) | bit;
                if (carry || r >= d) {
                    r -= d;
                    if (k - 1 >= 256) qhi |= uint256(1) << (k - 257);
                    else qlo |= uint256(1) << (k - 1);
                }
            }
        }
        if (qhi != 0) return (false, 0);
        if (r != 0) {
            if (qlo == MAX) return (false, 0);
            qlo++;
        }
        return (true, qlo);
    }

    function _p10(uint256 x, uint256 k) internal pure returns (bool, uint256) {
        if (k > 77) return (false, 0);
        uint256 p = 1;
        for (uint256 n = 0; n < k; n++) {
            p *= 10;
        }
        if (x > MAX / p) return (false, 0);
        return (true, x * p);
    }

    struct Case {
        uint256 dT;
        uint256 dU;
        uint256 decT;
        uint256 decU;
        uint256 lT;
        uint256 lU;
        uint256 lTok;
        uint256 lUnit;
        uint256 pT;
        uint256 pU;
        uint256[4] v; // fixed, min, max, cap
        uint256 bps;
        uint256 amount;
    }

    function _decs(uint256 h) internal pure returns (uint256) {
        uint256[8] memory odd = [uint256(37), 64, 72, 255, 256, 1 << 255, MAX, 100];
        if (h % 10 == 0) return odd[(h >> 8) % 8];
        return (h >> 16) % 37;
    }

    function _len(uint256 h) internal pure returns (uint256) {
        uint256[6] memory l = [uint256(0), 1, 31, 33, 64, 96];
        if (h % 12 == 0) return l[(h >> 8) % 6];
        return 32;
    }

    function _amt(uint256 h) internal pure returns (uint256) {
        uint256 k = h % 8;
        if (k == 0) return 0;
        if (k == 1) return 1;
        if (k == 2) return MAX;
        if (k == 3) return MAX >> ((h >> 8) % 256);
        return (h >> 16) % 1e30;
    }

    /// Every decimals() at 0 to 36 and past it, each answered in 0 to 96 bytes, prices from 1 to 2^128 - 1 and past
    /// it, absolute amounts from 0 to 2^256 - 1: the verifier's stale, overflow, lf and cap are the model's.
    function testFuzz_R5_pricingUnderOddDecimals(uint256 s0, uint256 s1, uint256 s2) public {
        Case memory k;
        k.dT = _decs(s0);
        k.dU = _decs(s0 >> 32);
        k.decT = _decs(s0 >> 64);
        k.decU = _decs(s0 >> 96);
        k.lT = _len(s0 >> 128);
        k.lU = _len(s0 >> 144);
        k.lTok = _len(s0 >> 160);
        k.lUnit = _len(s0 >> 176);
        k.pT = s1 % 7 == 0 ? (s1 >> 8) % 3 : (s1 >> 16) % (uint256(1) << ((s1 >> 8) % 129));
        k.pU = s2 % 7 == 0 ? uint256(1) << 128 : (s2 >> 16) % (uint256(1) << ((s2 >> 8) % 129));
        if (k.pT == 0 && s1 % 5 != 0) k.pT = 1;
        if (k.pU == 0 && s2 % 5 != 0) k.pU = 1;
        uint256 h = uint256(keccak256(abi.encode(s0, s1, s2)));
        for (uint256 x = 0; x < 4; x++) {
            k.v[x] = _amt(h >> (x * 40));
        }
        k.bps = (h >> 200) % 10001;
        k.amount = 1 + (h >> 210) % (MAX / 10000);
        R5Token tkn = new R5Token();
        R5Feed fT = new R5Feed();
        R5Feed fU = new R5Feed();
        R5Feed unit = new R5Feed();
        uint256 at = vm.getBlockTimestamp() - 1;
        fT.set(int256(k.pT), int256(k.pT), at, k.dT, k.dT, k.lT, 0);
        fU.set(int256(k.pU), int256(k.pU), at, k.dU, k.dU, k.lU, 0);
        tkn.setDec(k.decT, k.decT, k.lTok, false);
        unit.set(1, 1, 0, k.decU, k.decU, k.lUnit, 0);
        uint256[15] memory q;
        q[0] = uint256(uint160(address(tkn)));
        q[2] = uint256(uint160(address(unit)));
        q[3] = uint256(uint160(address(fT)));
        q[4] = uint256(uint160(address(fU)));
        q[5] = type(uint32).max;
        q[6] = k.v[0];
        q[7] = k.bps;
        q[8] = k.v[1];
        q[9] = k.v[2];
        q[10] = k.v[3];
        EravonVerifier.Checked memory c = hx.priceX(q, k.amount, address(tkn));
        // the model
        bool stale = k.lT < 32 || k.lU < 32 || k.lTok < 32 || k.lUnit < 32 || k.dT > 36 || k.dU > 36 || k.decT > 36 || k.decU > 36
            || k.pT == 0 || k.pT >= (1 << 128) || k.pU == 0 || k.pU >= (1 << 128);
        assertEq(c.stale, stale, "stale is the model's");
        if (stale) return;
        bool over;
        uint256[4] memory v = k.v;
        if (v[0] != 0 || v[1] != 0 || v[2] != 0 || v[3] != 0) {
            uint256 a_ = k.dT + k.decT;
            uint256 b_ = k.dU + k.decU;
            uint256 num = k.pU;
            uint256 den = k.pT;
            bool ok = true;
            if (a_ >= b_) (ok, num) = _p10(k.pU, a_ - b_);
            else (ok, den) = _p10(k.pT, b_ - a_);
            over = !ok;
            for (uint256 x = 0; x < 4 && !over; x++) {
                if (v[x] == 0) continue;
                (ok, v[x]) = _ceil(v[x], num, den);
                over = !ok;
            }
        }
        uint256 share = (k.amount * k.bps) / 10000;
        if (!over && v[0] > MAX - share) over = true;
        assertEq(c.overflow, over, "overflow is the model's");
        if (over) return;
        uint256 lf = v[0] + share;
        if (v[1] != 0 && lf < v[1]) lf = v[1];
        if (v[2] != 0 && lf > v[2]) lf = v[2];
        assertEq(c.lf, lf, "lf is the model's");
        assertEq(c.cap, v[3], "the cap is the model's");
        assertEq(c.priceToken, k.pT, "priceToken");
        assertEq(c.priceUnit, k.pU, "priceUnit");
        assertEq(c.rateAt, at, "rateAt");
    }
}
