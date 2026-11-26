// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "../EravonBase.sol";
import {Eravon} from "../../src/Eravon.sol";
import {EravonVerifier} from "../../src/EravonVerifier.sol";
import {IPriceFeed} from "../../src/interfaces.sol";
import {RawFeed, DecToken} from "../Eravon.fuzz.t.sol";
import {BytesFeed} from "../Eravon.arith.t.sol";
import {R4ShapedFeed} from "./R4Mocks.sol";

/// The verifier's internal arithmetic, decoders and feed read, exposed (the deployed verifier is untouched).
contract R4Harness is EravonVerifier {
    function mulDivUpX(uint256 x, uint256 y, uint256 d) external pure returns (uint256, bool) {
        return _mulDivUp(x, y, d);
    }

    function mulPow10X(uint256 x, uint256 k) external pure returns (uint256, bool) {
        return _mulPow10(x, k);
    }

    function readFeedX(address feed, uint32 maxStale) external view returns (uint256, uint8, uint64, bool) {
        return _readFeed(feed, maxStale);
    }

    function decimalsX(address token) external view returns (uint8, bool) {
        return _decimals(token);
    }

    function termsFaultX(Eravon.OrderTerms calldata t) external pure returns (uint8) {
        return _termsFault(t);
    }

}

/// Round 4, seat FUZZ (round 3's stateless campaign ported to ecf085ed and extended to the fixed-size reads):
/// stateless properties, each against a model built apart from the verifier's own code:
/// `_mulDivUp` by the characterisation of the rounded-up quotient (no division in the model, only 512-bit
/// products from 128-bit limbs), `_mulPow10` by its bound, the pricing under the scaled-price rule by the same
/// characterisation, `_readFeed` and `_decimals` against the EVM's own answer under READ_GAS for feeds that burn
/// and bomb, `_termsFault` at the bounds with several guards broken,
/// and `verifySeller` against `ecrecover` with the low-s rule.
contract EravonRound4Fuzz is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE_PAYOUT = address(0x4000000000000000000000000000000000000001);
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah");
    uint256 internal constant MAX = type(uint256).max;
    uint256 internal constant M128 = type(uint128).max;
    uint256 internal constant SECP_N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141;
    uint256 internal constant MAX_INT53 = 9007199254740991;
    uint256 internal constant READ_GAS = 200_000;
    R4Harness internal hx;

    function setUp() public {
        setUpBase();
        hx = new R4Harness();
    }

    // ------------------------------------------------------------------
    // 512-bit products from 128-bit limbs, and their order
    // ------------------------------------------------------------------

    function _mul512(uint256 x, uint256 y) internal pure returns (uint256 hi, uint256 lo) {
        uint256 xl = x & M128;
        uint256 xh = x >> 128;
        uint256 yl = y & M128;
        uint256 yh = y >> 128;
        uint256 ll = xl * yl;
        uint256 lh = xl * yh;
        uint256 hl = xh * yl;
        uint256 hh = xh * yh;
        unchecked {
            uint256 mid = lh + hl;
            uint256 midCarry = mid < lh ? 1 : 0;
            lo = ll + (mid << 128);
            uint256 c1 = lo < ll ? 1 : 0;
            hi = hh + (mid >> 128) + (midCarry << 128) + c1;
        }
    }

    function _lt512(uint256 ah, uint256 al, uint256 bh, uint256 bl) internal pure returns (bool) {
        return ah < bh || (ah == bh && al < bl);
    }

    /// The rounded-up quotient's characterisation: ceil(x y / d) = z exactly where (z - 1) d < x y <= z d, and it
    /// passes 2^256 - 1 exactly where (2^256 - 1) d < x y.
    function _checkCeil(uint256 x, uint256 y, uint256 d, uint256 z, bool ok) internal pure returns (bool) {
        (uint256 ph, uint256 pl) = _mul512(x, y);
        (uint256 mh, uint256 ml) = _mul512(MAX, d);
        bool over = _lt512(mh, ml, ph, pl);
        if (over) return !ok;
        if (!ok) return false;
        (uint256 zh, uint256 zl) = _mul512(z, d);
        if (_lt512(zh, zl, ph, pl)) return false; // z d < x y
        if (z == 0) return ph == 0 && pl == 0;
        (uint256 wh, uint256 wl) = _mul512(z - 1, d);
        return _lt512(wh, wl, ph, pl); // (z - 1) d < x y
    }

    function _mag(uint256 w, uint256 s) internal pure returns (uint256) {
        return w >> (s % 257 == 256 ? 255 : s % 256);
    }

    // ==================================================================
    // 1. _mulDivUp and _mulPow10
    // ==================================================================

    function testFuzz_mulDivUpIsTheCeiling(uint256 x, uint256 y, uint256 d, uint256 s) public view {
        x = _mag(x, s);
        y = _mag(y, s >> 9);
        d = _mag(d, s >> 18);
        uint256 k = (s >> 27) % 12;
        if (k == 0) x = MAX;
        else if (k == 1) y = MAX;
        else if (k == 2) d = MAX;
        else if (k == 3) d = 1;
        else if (k == 4) {
            // the floor lands on 2^256 - 1 with a remainder, or just under it
            d = (1 << 255) + 1 + ((d >> 3) % (1 << 250));
            x = d + 1;
            y = (s >> 40) % 2 == 0 ? MAX - 1 : MAX - 2;
        } else if (k == 5) {
            // an exact multiple: x y = d q
            d = 1 + (d >> 129);
            x = d;
            y = y >> 1;
        } else if (k == 6) {
            x = 0;
        }
        if (d == 0) d = 1 + (s >> 48) % 13;
        (uint256 z, bool ok) = hx.mulDivUpX(x, y, d);
        assertTrue(_checkCeil(x, y, d, z, ok), "ceil(x y / d), or its overflow");
        if (!ok) assertEq(z, 0);
    }

    function _pow10(uint256 k) internal pure returns (uint256 p) {
        p = 1;
        for (uint256 i = 0; i < k; i++) {
            p *= 10;
        }
    }

    /// x * 10^k fits exactly where x is zero, or k <= 77 and x <= floor((2^256 - 1) / 10^k).
    function testFuzz_mulPow10IsItsBound(uint256 x, uint256 k, uint256 s) public view {
        k = k % 101;
        uint256 kind = s % 5;
        uint256 lim = k <= 77 ? MAX / _pow10(k) : 0;
        if (kind == 0) x = lim;
        else if (kind == 1 && lim != MAX) x = lim + 1;
        else if (kind == 2) x = 0;
        else x = _mag(x, s >> 8);
        (uint256 got, bool ok) = hx.mulPow10X(x, k);
        bool fits = x == 0 || (k <= 77 && x <= lim);
        assertEq(ok, fits, "fits exactly where the bound says");
        if (fits) assertEq(got, x == 0 ? 0 : x * _pow10(k));
        else assertEq(got, 0);
    }

    // ==================================================================
    // 2. the pricing under the scaled-price rule, by the same characterisation
    // ==================================================================

    struct P {
        uint256 decT;
        uint256 decU;
        uint256 fdT;
        uint256 fdU;
        uint256 pT;
        uint256 pU;
        uint256[4] x; // feeFixed, feeMin, feeMax, amountCap in the unit
        uint16 bps;
        uint256 amount;
    }

    function _p(uint256 s0, uint256 s1, uint256 s2, uint256 s3) internal pure returns (P memory p) {
        p.decT = s0 % 37;
        p.decU = (s0 >> 8) % 37;
        p.fdT = (s0 >> 16) % 37;
        p.fdU = (s0 >> 24) % 37;
        if ((s0 >> 32) % 3 != 0) {
            // the realistic corner: 6, 8 or 18 decimals
            uint256[3] memory r = [uint256(6), 8, 18];
            p.decT = r[(s0 >> 36) % 3];
            p.decU = r[(s0 >> 40) % 3];
            p.fdT = 8;
            p.fdU = 8;
        }
        p.pT = 1 + _mag(s1, s0 >> 48) % M128;
        p.pU = 1 + _mag(s1 >> 7 | s2 << 249, s0 >> 60) % M128;
        if ((s0 >> 72) % 9 == 0) p.pT = M128;
        if ((s0 >> 80) % 9 == 0) p.pU = M128;
        for (uint256 i = 0; i < 4; i++) {
            uint256 w = uint256(keccak256(abi.encode(s2, i)));
            uint256 k = (s3 >> (8 * i)) % 6;
            p.x[i] = k == 0 ? 0 : (k == 1 ? w : (k == 2 ? MAX - (w % 997) : _mag(w, s3 >> (64 + 9 * i))));
        }
        if ((s3 >> 120) % 5 == 0) {
            p.x[0] = 0;
            p.x[1] = 0;
            p.x[2] = 0;
            p.x[3] = 0;
        }
        p.bps = uint16((s3 >> 130) % 10001);
        p.amount = 1 + _mag(uint256(keccak256(abi.encode(s2, "a"))), s3 >> 150) % (MAX / 10000);
    }

    /// standard s3: where any of the four amounts is nonzero the scaled price (pU or pT times the power of ten
    /// left after the common one cancels) must fit 256 bits, and every converted amount, rounded up, too, else
    /// FeeOverflow; the conversion's value is checked by the characterisation; then terms/1 s6.1.
    function testFuzz_pricingIsTheScaledPriceRule(uint256 s0, uint256 s1, uint256 s2, uint256 s3) public {
        P memory p = _p(s0, s1, s2, s3);
        DecToken settle = new DecToken(p.decT);
        DecToken unit = new DecToken(p.decU);
        RawFeed fT = new RawFeed();
        RawFeed fU = new RawFeed();
        fT.set(p.fdT, 1, int256(p.pT), vm.getBlockTimestamp(), 0);
        fU.set(p.fdU, 1, int256(p.pU), vm.getBlockTimestamp(), 0);
        Q memory q = defaultQuote(address(settle), JUDGE_PAYOUT, true);
        q.feeUnit = address(unit);
        q.feedToken = address(fT);
        q.feedUnit = address(fU);
        q.maxStale = 1 days;
        q.feeFixed = p.x[0];
        q.feeBps = p.bps;
        q.feeMin = p.x[1];
        q.feeMax = p.x[2];
        q.amountCap = p.x[3];
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, p.amount);
        t.offer.token = address(settle);
        EravonVerifier.Checked memory c = verifier.check(t, quote, "", "", new address[](0), new bytes32[](0));
        assertFalse(c.stale, "fresh feeds");
        bool any = p.x[0] != 0 || p.x[1] != 0 || p.x[2] != 0 || p.x[3] != 0;
        uint256[4] memory y = p.x;
        if (any) {
            uint256 a_ = p.fdT + p.decT;
            uint256 b_ = p.fdU + p.decU;
            uint256 e = a_ >= b_ ? a_ - b_ : b_ - a_;
            uint256 base = a_ >= b_ ? p.pU : p.pT;
            bool scaledFits = e <= 77 && base <= MAX / _pow10(e);
            if (!scaledFits) {
                assertTrue(c.overflow, "the scaled price passes 2^256 - 1: FeeOverflow");
                return;
            }
            uint256 num = a_ >= b_ ? base * _pow10(e) : p.pU;
            uint256 den = a_ >= b_ ? p.pT : base * _pow10(e);
            for (uint256 i = 0; i < 4; i++) {
                if (p.x[i] == 0) continue;
                (uint256 ph, uint256 pl) = _mul512(p.x[i], num);
                (uint256 mh, uint256 ml) = _mul512(MAX, den);
                if (_lt512(mh, ml, ph, pl)) {
                    assertTrue(c.overflow, "a converted amount passes 2^256 - 1: FeeOverflow");
                    return;
                }
            }
            // the converted values are fixed only through lf and the cap: take the cap and lf's bounds from them
            (uint256 cz, bool cok) = hx.mulDivUpX(p.x[3], num, den);
            if (p.x[3] != 0) assertTrue(_checkCeil(p.x[3], num, den, cz, cok) && cok);
            for (uint256 i = 0; i < 3; i++) {
                if (p.x[i] == 0) continue;
                (uint256 z, bool zok) = hx.mulDivUpX(p.x[i], num, den);
                assertTrue(_checkCeil(p.x[i], num, den, z, zok) && zok, "the converted amount is the ceiling");
                y[i] = z;
            }
            y[3] = p.x[3] == 0 ? 0 : cz;
        }
        uint256 part = (p.amount * p.bps) / 10000;
        if (y[0] > MAX - part) {
            assertTrue(c.overflow, "feeFixed + A x feeBps passes 2^256 - 1");
            return;
        }
        assertFalse(c.overflow);
        uint256 lf = y[0] + part;
        if (y[1] != 0 && lf < y[1]) lf = y[1];
        if (y[2] != 0 && lf > y[2]) lf = y[2];
        assertEq(c.lf, lf, "lf from the converted amounts");
        assertEq(c.cap, y[3], "the cap converted");
        assertEq(c.priceToken, p.pT);
        assertEq(c.priceUnit, p.pU);
    }

    // ==================================================================
    // 3. the feed read and decimals() over raw answers of every length, under READ_GAS
    // ==================================================================

    /// The words of a raw answer, read as the text reads them: word i of the first 32 (i + 1) bytes, or nothing.
    function _wordAt(bytes memory raw, uint256 i) internal pure returns (uint256 w) {
        assembly ("memory-safe") {
            w := mload(add(add(raw, 32), mul(i, 32)))
        }
    }

    function _rawRound(uint256 s, uint256 L, int256 answer, uint256 upd) internal pure returns (bytes memory raw) {
        raw = new bytes(L);
        for (uint256 i = 0; i < L; i++) {
            raw[i] = bytes1(uint8(uint256(keccak256(abi.encode(s, i)))));
        }
        uint256[5] memory w = [uint256(1), uint256(answer), upd, upd, 1];
        for (uint256 i = 0; i < 5; i++) {
            if (32 * (i + 1) <= L && (s >> (100 + i)) % 8 != 0) {
                uint256 v = w[i];
                assembly ("memory-safe") {
                    mstore(add(add(raw, 32), mul(i, 32)), v)
                }
            }
        }
    }

    /// latestRoundData() and decimals() answer raw bytes of any length to 400 (random bytes, the round's words
    /// set where they fit, three in four times), or revert, or burn their allowance; the verifier's read never
    /// reverts and is fresh exactly where the text says: the round's call succeeds with at least 160 bytes whose
    /// second word is positive below 2^128 and whose fourth is in [now - maxStale, now] below 2^64, and the
    /// decimals() call succeeds with at least 32 bytes whose first word is at most 36; bytes past the floor are
    /// never read.
    function testFuzz_readFeedOverRawAnswersOfEveryLength(uint256 s0, uint256 s1, int256 answer, uint256 upd) public {
        BytesFeed f = new BytesFeed();
        uint256 nowT = vm.getBlockTimestamp();
        uint32 maxStale = uint32(1 + s0 % 2 days);
        // half the answers valid, so that the length alone decides as often as it can
        uint256 k = (s1 >> 200) % 10;
        if (k < 5) answer = int256(1 + (uint256(answer) % 1e30));
        else if (k == 5) answer = int256(uint256(1 << 128) - 1);
        else if (k == 6) answer = int256(uint256(1 << 128));
        else if (k == 7) answer = 0;
        uint256 kt = (s1 >> 210) % 8;
        if (kt < 3) upd = nowT - (upd % (uint256(maxStale) + 1));
        else if (kt == 3) upd = nowT - maxStale;
        else if (kt == 4) upd = nowT - maxStale - 1;
        else if (kt == 5) upd = nowT + 1;
        // the lengths: half of them at and around the floor (128 to 161), the rest anywhere to 400
        uint256 L = (s0 >> 40) % 2 == 0 ? 128 + (s0 >> 48) % 34 : (s0 >> 48) % 401;
        uint256 LD = (s0 >> 64) % 2 == 0 ? (s0 >> 72) % 97 : [uint256(0), 1, 31, 32, 33, 64][(s0 >> 80) % 6];
        bytes memory raw = _rawRound(s1, L, answer, upd);
        bytes memory dec = new bytes(LD);
        // a short or long decimals() answer of zeros (what a contract with nothing to say may answer) one time in
        // three, random bytes otherwise
        if ((s0 >> 90) % 3 != 0) {
            for (uint256 i = 0; i < LD; i++) {
                dec[i] = bytes1(uint8(uint256(keccak256(abi.encode(s1, "d", i)))));
            }
        }
        uint256 dv = (s1 >> 220) % 5 == 0 ? (s1 >> 224) % 40 : 8;
        if (LD >= 32 && (s1 >> 230) % 4 != 0 && (s0 >> 90) % 3 != 0) {
            assembly ("memory-safe") {
                mstore(add(dec, 32), dv)
            }
        }
        bool rr = (s1 >> 236) % 13 == 0;
        bool dr = (s1 >> 240) % 13 == 0;
        f.set(raw, dec, rr, dr);
        f.setBurns((s1 >> 244) % 17 == 0, (s1 >> 248) % 17 == 0);
        // the model, from the raw bytes alone
        bool fresh = !rr && (s1 >> 244) % 17 != 0 && L >= 160;
        if (fresh) {
            uint256 aw = _wordAt(raw, 1);
            uint256 tw = _wordAt(raw, 3);
            fresh = aw != 0 && aw >> 255 == 0 && aw < (1 << 128) && tw < (1 << 64) && tw <= nowT && tw + maxStale >= nowT;
        }
        bool decOk = !dr && (s1 >> 248) % 17 != 0 && LD >= 32 && _wordAt(dec, 0) <= 36;
        try hx.readFeedX(address(f), maxStale) returns (uint256 price, uint8 d_, uint64 at, bool ok) {
            assertEq(ok, fresh && decOk, "fresh exactly where the raw answers say");
            if (ok) {
                assertEq(price, _wordAt(raw, 1));
                assertEq(d_, _wordAt(dec, 0));
                assertEq(at, uint64(_wordAt(raw, 3)));
            }
        } catch {
            assertTrue(false, "the feed read reverted");
        }
        try hx.decimalsX(address(f)) returns (uint8 d2, bool ok2) {
            assertEq(ok2, decOk, "decimals() as the raw answer says");
            if (ok2) assertEq(d2, _wordAt(dec, 0));
        } catch {
            assertTrue(false, "decimals() read reverted");
        }
    }

    /// The same reads against bombs: a round or a decimals() that answers its words and then runs on to a random
    /// length up to 400,000 bytes, after burning random gas: the read is fresh exactly where the call made here
    /// with exactly READ_GAS (copying nothing) succeeds, and its answer is the first words', whatever the length.
    function testFuzz_readFeedUnderBombsAndBurns(uint256 s0, uint256 s1) public {
        R4ShapedFeed f = new R4ShapedFeed();
        uint256 nowT = vm.getBlockTimestamp();
        f.set(int256(1 + s1 % 1e30), nowT - (s1 >> 100) % 3600, 8 + (s1 >> 110) % 29);
        uint256 lr = (s0 % 2 == 0) ? 160 + (s0 >> 8) % 400_000 : [uint256(160), 161, 4096, 240_000, 300_000][(s0 >> 8) % 5];
        uint256 ld = (s0 >> 40) % 2 == 0 ? 32 + (s0 >> 48) % 400_000 : [uint256(32), 33, 4096, 240_000, 300_000][(s0 >> 48) % 5];
        f.setLens(lr, ld);
        uint256 br = (s0 >> 80) % 3 == 0 ? (s0 >> 88) % 250_000 : 0;
        uint256 bd = (s0 >> 120) % 3 == 0 ? (s0 >> 128) % 250_000 : 0;
        f.setBurns(br, bd);
        bool c1;
        bool c2;
        uint256 g = READ_GAS;
        bytes memory d1 = abi.encodeWithSelector(IPriceFeed.latestRoundData.selector);
        bytes memory d2 = abi.encodeWithSelector(IPriceFeed.decimals.selector);
        assembly ("memory-safe") {
            c1 := staticcall(g, f, add(d1, 32), mload(d1), 0, 0)
            c2 := staticcall(g, f, add(d2, 32), mload(d2), 0, 0)
        }
        (uint256 price, uint8 dd, uint64 at, bool ok) = hx.readFeedX(address(f), 1 days);
        assertEq(ok, c1 && c2, "fresh exactly where both calls under READ_GAS succeed");
        if (ok) {
            assertEq(price, uint256(f.answer()));
            assertEq(dd, f.dec());
            assertEq(at, f.at());
        }
        (uint8 d3, bool ok3) = hx.decimalsX(address(f));
        assertEq(ok3, c2);
        if (ok3) assertEq(d3, f.dec());
    }

    // ==================================================================
    // 4. _termsFault at the bounds, several guards broken at once
    // ==================================================================

    function _faultModel(Eravon.OrderTerms memory t) internal pure returns (uint8) {
        Eravon.Offer memory o = t.offer;
        if (o.sellerKey == address(0) || t.buyerKey == address(0) || o.sellerKey == t.buyerKey) return 1;
        if ((o.judge != address(0)) == (o.guild != address(0))) return 2;
        if (o.judge != address(0) && (o.judge == o.sellerKey || o.judge == t.buyerKey)) return 3;
        if (o.guild != address(0) && (o.guild == o.sellerKey || o.guild == t.buyerKey)) return 3;
        if (o.sellerPayout == address(0) || o.sellerFund == address(0) || t.buyerPayout == address(0)) return 4;
        if (o.token == address(0)) return 5;
        if (o.amount == 0 || o.amount > MAX / 10000) return 6;
        uint256 n = o.ladder.length;
        if (n < 2 || n > 16) return 7;
        for (uint256 i = 0; i < n; i++) {
            if (uint256(o.ladder[i].buyerBps) + o.ladder[i].sellerBps != 10000) return 8;
            if (o.ladder[i].feeBearer > 2) return 9;
        }
        if (o.ladder[0].buyerBps != 0) return 11;
        if (o.ladder[n - 1].buyerBps != 10000) return 12;
        for (uint256 i = 1; i < n; i++) {
            if (o.ladder[i].buyerBps < o.ladder[i - 1].buyerBps) return 10;
        }
        if (o.termsHash == bytes32(0)) return 13;
        if (o.amanahHash == bytes32(0)) return 14;
        if (o.quoteHash == bytes32(0)) return 15;
        if (o.nonce > MAX_INT53 || o.expiry > MAX_INT53 || t.openBy > MAX_INT53) return 16;
        return 0;
    }

    function _edge(uint256 b, uint256 s) internal pure returns (uint256) {
        uint256 k = s % 3;
        return k == 0 ? b - 1 : (k == 1 ? b : b + 1);
    }

    /// Each guard broken with probability one in five, independently, the integers drawn one below, at and one
    /// above their bounds, the ladder's sizes 1, 2, 16 and 17, a rung's sum at 9999 and 10001, a bearer at 2 and
    /// 3, the order broken by one bps; the verifier names the table's first failing row.
    function testFuzz_termsFaultIsTheTablesFirstRow(uint256 s, uint256 w) public view {
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
        o.ladder = new Eravon.Rung[](n);
        for (uint256 i = 0; i < n; i++) {
            uint16 b = i == n - 1 ? 10000 : uint16((10000 * i) / (n - 1));
            o.ladder[i] = Eravon.Rung(b, 10000 - b, uint8((w >> (8 + i)) % 3));
        }
        o.termsHash = keccak256("t");
        o.amanahHash = keccak256("a");
        o.quoteHash = keccak256("q");
        o.nonce = MAX_INT53 - (w >> 40) % 3;
        o.expiry = MAX_INT53 - (w >> 42) % 2;
        t.openBy = uint64(MAX_INT53 - (w >> 44) % 2);
        uint256 h = uint256(keccak256(abi.encode(s, "break")));
        if (h % 5 == 0) o.sellerKey = (h >> 3) % 2 == 0 ? address(0) : BUYER;
        if ((h >> 5) % 5 == 0) {
            if ((h >> 8) % 2 == 0) o.guild = GUILD; // both nonzero
            else o.judge = address(0); // neither
        }
        if ((h >> 10) % 5 == 0) o.judge = (h >> 13) % 2 == 0 ? BUYER : SELLER;
        if ((h >> 15) % 5 == 0) {
            o.judge = address(0);
            o.guild = (h >> 18) % 2 == 0 ? SELLER : BUYER;
        }
        if ((h >> 20) % 5 == 0) t.buyerPayout = address(0);
        if ((h >> 25) % 5 == 0) o.token = address(0);
        if ((h >> 30) % 5 == 0) o.amount = (h >> 33) % 2 == 0 ? 0 : _edge(MAX / 10000, h >> 34);
        if ((h >> 40) % 5 == 0) {
            uint256[4] memory sizes = [uint256(0), 1, 16, 17];
            uint256 m = sizes[(h >> 43) % 4];
            Eravon.Rung[] memory l = new Eravon.Rung[](m);
            for (uint256 i = 0; i < m; i++) {
                uint16 b = m < 2 ? 0 : (i == m - 1 ? 10000 : uint16((10000 * i) / (m - 1)));
                l[i] = Eravon.Rung(b, 10000 - b, 0);
            }
            o.ladder = l;
        }
        uint256 L = o.ladder.length;
        if (L != 0 && (h >> 50) % 5 == 0) {
            uint256 at = (h >> 53) % L;
            unchecked {
                o.ladder[at].sellerBps += (h >> 57) % 2 == 0 ? 1 : 0xffff; // the sum one past or one short (mod 2^16)
            }
        }
        if (L != 0 && (h >> 60) % 5 == 0) o.ladder[(h >> 63) % L].feeBearer = 3;
        if (L >= 2 && (h >> 70) % 5 == 0) {
            o.ladder[0].buyerBps = 1;
            o.ladder[0].sellerBps = 9999;
        }
        if (L >= 2 && (h >> 75) % 5 == 0) {
            o.ladder[L - 1].buyerBps = 9999;
            o.ladder[L - 1].sellerBps = 1;
        }
        if (L >= 3 && (h >> 80) % 5 == 0 && o.ladder[2].buyerBps < 10000) {
            // rung 1 above rung 2 by one bps
            o.ladder[1].buyerBps = o.ladder[2].buyerBps + 1;
            o.ladder[1].sellerBps = 10000 - o.ladder[1].buyerBps;
        }
        if ((h >> 90) % 5 == 0) o.termsHash = 0;
        if ((h >> 95) % 5 == 0) o.amanahHash = 0;
        if ((h >> 100) % 5 == 0) o.quoteHash = 0;
        if ((h >> 105) % 5 == 0) o.nonce = MAX_INT53 + 1;
        if ((h >> 110) % 5 == 0) o.expiry = MAX_INT53 + 1;
        if ((h >> 115) % 5 == 0) t.openBy = uint64(MAX_INT53 + 1);
        t.offer = o;
        assertEq(hx.termsFaultX(t), _faultModel(t), "the first failing row of s4's table");
    }

    // ==================================================================
    // 6. verifySeller against ecrecover with the low-s rule
    // ==================================================================

    /// A real signature by a random key, its high-s twin, its v flipped, cut or lengthened, r or s at zero, n or
    /// n/2 + 1, or random words; the key the right one, another, or zero: verifySeller passes exactly where the
    /// signature is 65 bytes, v is 27 or 28, 0 < r < n, 0 < s <= n/2, and ecrecover gives the key, nonzero.
    function testFuzz_verifySellerIsLowSEcrecover(uint256 pk, bytes32 d, uint256 mode, bytes32 rr, bytes32 ss) public view {
        pk = 1 + pk % (SECP_N - 1);
        address signer = vm.addr(pk);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(pk, d);
        uint256 m = mode % 12;
        if (m == 1) {
            s = bytes32(SECP_N - uint256(s));
            v = v == 27 ? 28 : 27;
        } else if (m == 2) {
            v = v == 27 ? 28 : 27;
        } else if (m == 3) {
            r = 0;
        } else if (m == 4) {
            s = 0;
        } else if (m == 5) {
            r = bytes32(SECP_N);
        } else if (m == 6) {
            s = bytes32((SECP_N >> 1) + 1);
        } else if (m == 7) {
            r = rr;
            s = ss;
            v = uint8(27 + (mode >> 8) % 2);
        } else if (m == 8) {
            v = uint8((mode >> 8) % 256);
        }
        bytes memory sig = abi.encodePacked(r, s, v);
        if (m == 9) sig = abi.encodePacked(r, s);
        if (m == 10) sig = abi.encodePacked(r, s, v, uint8(0));
        uint256 ks = (mode >> 16) % 4;
        address key = ks == 0 ? address(0) : (ks == 1 ? address(uint160(uint256(keccak256(abi.encode(mode))))) : signer);
        bool pass = sig.length == 65 && (v == 27 || v == 28) && uint256(r) != 0 && uint256(r) < SECP_N && uint256(s) != 0
            && uint256(s) <= (SECP_N >> 1);
        if (pass) {
            address rec = ecrecover(d, v, r, s);
            pass = rec != address(0) && rec == key;
        }
        try verifier.verifySeller(d, sig, key) {
            assertTrue(pass, "accepted a signature the rule refuses");
        } catch (bytes memory err) {
            assertFalse(pass, "refused a signature the rule accepts");
            assertEq(err.length, 4);
            assertEq(bytes4(err), Eravon.BadSellerSig.selector);
        }
    }
}
