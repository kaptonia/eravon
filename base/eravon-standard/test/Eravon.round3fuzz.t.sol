// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {EravonVerifier} from "../src/EravonVerifier.sol";
import {IPriceFeed} from "../src/interfaces.sol";
import {RawFeed, DecToken} from "./Eravon.fuzz.t.sol";
import {R3GasFeed} from "./Eravon.round3.t.sol";

/// The verifier's internal arithmetic, decoders and feed read, exposed (the deployed verifier is untouched).
contract R3Harness is EravonVerifier {
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

/// Round 3, seat FUZZ: stateless properties, each against a model built apart from the verifier's own code:
/// `_mulDivUp` by the characterisation of the rounded-up quotient (no division in the model, only 512-bit
/// products from 128-bit limbs), `_mulPow10` by its bound, the pricing under the scaled-price rule by the same
/// characterisation, `_readFeed` and `_decimals` against the EVM's own answer under READ_GAS for feeds that burn
/// and bomb, `_termsFault` at the bounds with several guards broken, and `verifySeller` against `ecrecover` with
/// the low-s rule.
contract EravonRound3Fuzz is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE_PAYOUT = address(0x4000000000000000000000000000000000000001);
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah");
    uint256 internal constant MAX = type(uint256).max;
    uint256 internal constant M128 = type(uint128).max;
    uint256 internal constant SECP_N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141;
    uint256 internal constant MAX_INT53 = 9007199254740991;
    uint256 internal constant READ_GAS = 200_000;
    R3Harness internal hx;

    function setUp() public {
        setUpBase();
        hx = new R3Harness();
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
    // 3. the feed read and decimals() under READ_GAS, against the EVM's own answer
    // ==================================================================

    /// A feed whose two reads burn random gas (to past READ_GAS) and bomb random padding (to past what READ_GAS
    /// pays for), with an answer at its edges: `_readFeed` never reverts, and it is fresh exactly where both
    /// calls, each made here with exactly READ_GAS, succeed with the round's 160 bytes and a decimals word, the
    /// answer is positive below 2^128, updatedAt is in [now - maxStale, now] below 2^64, and decimals is at most
    /// 36; `_decimals` likewise for a reference unit.
    function testFuzz_readFeedUnderReadGas(uint256 s0, uint256 s1, int256 answer, uint256 upd) public {
        R3GasFeed f = new R3GasFeed();
        uint256 nowT = vm.getBlockTimestamp();
        uint32 maxStale = uint32(1 + s0 % 2 days);
        uint256 k = (s1 >> 200) % 6;
        if (k == 0) answer = int256(1 + (uint256(answer) % 1e30));
        else if (k == 1) answer = int256(uint256(1 << 128) - 1);
        else if (k == 2) answer = int256(uint256(1 << 128));
        else if (k == 3) answer = 0;
        uint256 kt = (s1 >> 210) % 5;
        if (kt == 0) upd = nowT;
        else if (kt == 1) upd = nowT - maxStale;
        else if (kt == 2) upd = nowT - maxStale - 1;
        else if (kt == 3) upd = nowT + 1;
        uint256 dec = (s1 >> 220) % 5 == 0 ? (s1 >> 224) % 40 : 8;
        f.set(answer, upd, dec);
        uint256 br = (s0 >> 32) % 3 == 0 ? (s0 >> 40) % 400_000 : 0;
        uint256 bd = (s0 >> 64) % 3 == 0 ? (s0 >> 72) % 400_000 : 0;
        uint256 pr = (s0 >> 104) % 3 == 0 ? (s0 >> 112) % 12_000 : 0;
        uint256 pd = (s0 >> 136) % 3 == 0 ? (s0 >> 144) % 12_000 : 0;
        if ((s1 >> 232) % 7 == 0) br = type(uint256).max;
        f.setGas(br, bd, pr, pd);
        if ((s1 >> 240) % 9 == 0) f.setCut(true, (s1 >> 244) % 192);
        // the EVM's own answer to each read under exactly READ_GAS
        (bool c1, bytes memory r1) = address(f).staticcall{gas: READ_GAS}(abi.encodeWithSelector(IPriceFeed.latestRoundData.selector));
        (bool c2, bytes memory r2) = address(f).staticcall{gas: READ_GAS}(abi.encodeWithSelector(IPriceFeed.decimals.selector));
        bool fresh = c1 && r1.length >= 160;
        if (fresh) {
            uint256 aw;
            uint256 tw;
            assembly ("memory-safe") {
                aw := mload(add(r1, 64))
                tw := mload(add(r1, 128))
            }
            fresh = aw != 0 && aw < (1 << 128) && tw < (1 << 64) && tw <= nowT && tw + maxStale >= nowT;
        }
        if (fresh) {
            fresh = c2 && r2.length >= 32;
            if (fresh) {
                uint256 dw;
                assembly ("memory-safe") {
                    dw := mload(add(r2, 32))
                }
                fresh = dw <= 36;
            }
        }
        try hx.readFeedX(address(f), maxStale) returns (uint256 price, uint8 d_, uint64 at, bool ok) {
            assertEq(ok, fresh, "fresh exactly where the EVM's own reads under READ_GAS say");
            if (ok) {
                assertEq(price, uint256(answer));
                assertEq(d_, dec);
                assertEq(at, uint64(upd));
            }
        } catch {
            assertTrue(false, "the feed read reverted");
        }
        bool decOk = c2 && r2.length >= 32 && abi.decode(r2, (uint256)) <= 36;
        try hx.decimalsX(address(f)) returns (uint8 d2, bool ok2) {
            assertEq(ok2, decOk, "decimals() as the EVM's own read says");
            if (ok2) assertEq(d2, dec);
        } catch {
            assertTrue(false, "decimals() read reverted");
        }
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
