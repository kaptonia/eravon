// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {EravonVerifier} from "../src/EravonVerifier.sol";
import {IPriceFeed} from "../src/interfaces.sol";
import {RawFeed, DecToken} from "./Eravon.fuzz.t.sol";

/// The verifier's internal arithmetic and decoders, exposed (the logic reaches them through `check`,
/// `verifyBuyer` and `verifySeller`; the deployed verifier is untouched).
contract ArithHarness is EravonVerifier {
    function mulDivUpX(uint256 x, uint256 y, uint256 d) external pure returns (uint256, bool) {
        return _mulDivUp(x, y, d);
    }

    function mulPow10X(uint256 x, uint256 k) external pure returns (uint256, bool) {
        return _mulPow10(x, k);
    }

    function readFeedX(address feed, uint32 maxStale) external view returns (uint256, uint8, uint64, bool) {
        return _readFeed(feed, maxStale);
    }

}

/// A feed whose two answers are raw bytes set by the test: latestRoundData() and decimals() return
/// exactly the stored bytes, revert, or burn every unit of gas they are handed.
contract BytesFeed {
    bytes internal round;
    bytes internal dec;
    bool internal roundReverts;
    bool internal decReverts;
    bool internal roundBurns;
    bool internal decBurns;

    function set(bytes calldata r, bytes calldata d, bool rr, bool dr) external {
        round = r;
        dec = d;
        roundReverts = rr;
        decReverts = dr;
    }

    function setBurns(bool rb, bool db) external {
        roundBurns = rb;
        decBurns = db;
    }

    fallback(bytes calldata input) external returns (bytes memory) {
        if (input.length >= 4 && bytes4(input[:4]) == IPriceFeed.latestRoundData.selector) {
            if (roundBurns) _burn();
            if (roundReverts) revert();
            return round;
        }
        if (input.length >= 4 && bytes4(input[:4]) == IPriceFeed.decimals.selector) {
            if (decBurns) _burn();
            if (decReverts) revert();
            return dec;
        }
        revert();
    }

    function _burn() internal pure {
        assembly ("memory-safe") {
            invalid()
        }
    }
}

/// Round 2, seat FUZZ: stateless properties of the arithmetic and the decoders the round-1 fold changed,
/// each against a model written from the text and computed apart from the verifier's own code: the
/// 512-bit mulDiv rounding up and the power of ten (64-bit limbs and bitwise long division), the
/// conversion under the text's two overflow rules with every decimals word 0 to 36 and prices to 2^128,
/// the feed read over raw answers of every length and reads that burn their gas allowance, the terms
/// guards at their bounds, the seller's recovery against
/// ecrecover, and the refund during and after a dispute with strays at random ladders.
contract EravonArithFuzz is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE_PAYOUT = address(0x4000000000000000000000000000000000000001);
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    address internal constant STRANGER = address(0x3000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah");
    bytes32 internal constant MANIFEST = keccak256("manifest");
    bytes internal constant ENTRY = "judgment entry bytes";
    uint256 internal constant SECP_N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141;
    uint256 internal constant MAX_INT53 = 9007199254740991;
    uint256 internal constant M64 = type(uint64).max;
    uint256 internal constant MAX = type(uint256).max;
    ArithHarness internal ah;
    bytes32 internal E;

    function setUp() public {
        setUpBase();
        ah = new ArithHarness();
        E = sha256(ENTRY);
    }

    // ==================================================================
    // the 512-bit model: 64-bit limbs and bitwise long division
    // ==================================================================

    function _limbs(uint256 x) internal pure returns (uint256[8] memory r) {
        for (uint256 i = 0; i < 4; i++) {
            r[i] = (x >> (64 * i)) & M64;
        }
    }

    /// x * y exactly, as (hi, lo).
    function _mul512(uint256 x, uint256 y) internal pure returns (uint256 hi, uint256 lo) {
        uint256[8] memory a = _limbs(x);
        uint256[8] memory b = _limbs(y);
        uint256[8] memory r;
        for (uint256 i = 0; i < 4; i++) {
            uint256 carry = 0;
            for (uint256 j = 0; j < 4; j++) {
                uint256 t = a[i] * b[j] + r[i + j] + carry;
                r[i + j] = t & M64;
                carry = t >> 64;
            }
            r[i + 4] = carry;
        }
        for (uint256 i = 0; i < 4; i++) {
            lo |= r[i] << (64 * i);
            hi |= r[i + 4] << (64 * i);
        }
    }

    /// (hi, lo) / d bit by bit: the quotient as (qhi, qlo) and the remainder.
    function _divmod512(uint256 hi, uint256 lo, uint256 d) internal pure returns (uint256 qhi, uint256 qlo, uint256 r) {
        for (uint256 i = 0; i < 512; i++) {
            uint256 bit = i < 256 ? (hi >> (255 - i)) & 1 : (lo >> (511 - i)) & 1;
            bool top = (r >> 255) == 1;
            uint256 qb = 0;
            unchecked {
                r = (r << 1) | bit;
                if (top || r >= d) {
                    r -= d;
                    qb = 1;
                }
            }
            qhi = (qhi << 1) | (qlo >> 255);
            qlo = (qlo << 1) | qb;
        }
    }

    /// ceil(x * y / d), or the overflow past 2^256 - 1.
    function _mulDivUpModel(uint256 x, uint256 y, uint256 d) internal pure returns (bool ovf, uint256 z) {
        (uint256 hi, uint256 lo) = _mul512(x, y);
        (uint256 qhi, uint256 qlo, uint256 r) = _divmod512(hi, lo, d);
        if (qhi != 0) return (true, 0);
        if (r != 0) {
            if (qlo == MAX) return (true, 0);
            qlo += 1;
        }
        return (false, qlo);
    }

    /// x * 10^k exactly, or the overflow.
    function _pow10Model(uint256 x, uint256 k) internal pure returns (bool ovf, uint256 z) {
        if (x == 0) return (false, 0);
        z = x;
        for (uint256 i = 0; i < k; i++) {
            (uint256 hi, uint256 lo) = _mul512(z, 10);
            if (hi != 0) return (true, 0);
            z = lo;
        }
    }

    function _mag(uint256 w, uint256 s) internal pure returns (uint256) {
        return w >> (s % 257 == 256 ? 255 : s % 256);
    }

    // ==================================================================
    // 1. _mulDivUp and _mulPow10
    // ==================================================================

    /// The verifier's 512-bit mulDiv rounding up agrees with the model for random x, y, d of every
    /// magnitude, with exact multiples, with results at 2^256 - 1 and one past it after the rounding.
    function testFuzz_mulDivUpIsThe512BitModel(uint256 x, uint256 y, uint256 d, uint256 s) public view {
        uint256 k = s % 16;
        x = _mag(x, s >> 8);
        y = _mag(y, s >> 16);
        d = _mag(d, s >> 24);
        if (k == 0) {
            x = MAX;
        } else if (k == 1) {
            y = MAX;
        } else if (k == 2) {
            d = MAX;
        } else if (k == 3) {
            d = 1;
        } else if (k == 4 && d != 0) {
            // an exact multiple: x = d * q
            uint256 q = y >> 128;
            (uint256 hi, uint256 lo) = _mul512(d, q);
            if (hi == 0) x = lo;
        } else if (k == 5) {
            // floor at 2^256 - 1 with a remainder: rounding up overflows
            d = (1 << 255) + 1 + (d >> 2) % ((1 << 254) - 4);
            x = d + 1;
            y = MAX - 1;
        } else if (k == 6) {
            // floor at 2^256 - 2 with a remainder: rounds up to 2^256 - 1
            d = (1 << 255) + 1 + (d >> 2) % ((1 << 254) - 4);
            x = d + 1;
            y = MAX - 2;
        } else if (k == 7) {
            // x * y exactly d * (2^256 - 1)
            x = d;
            y = MAX;
        } else if (k == 8) {
            x = 0;
        }
        if (d == 0) d = 1 + (s >> 32) % 7;
        (bool ovf, uint256 z) = _mulDivUpModel(x, y, d);
        (uint256 got, bool ok) = ah.mulDivUpX(x, y, d);
        assertEq(!ok, ovf, "the overflow as the model's");
        if (ok) assertEq(got, z, "ceil(x * y / d)");
        else assertEq(got, 0);
    }

    function test_mulDivUpEdges() public view {
        (uint256 z, bool ok) = ah.mulDivUpX(MAX, MAX, MAX);
        assertTrue(ok);
        assertEq(z, MAX);
        (z, ok) = ah.mulDivUpX(MAX, 2, 2);
        assertTrue(ok);
        assertEq(z, MAX);
        (, ok) = ah.mulDivUpX(MAX, 3, 2);
        assertFalse(ok, "floor past 2^256 - 1");
        uint256 d = (1 << 255) + 12345;
        (, ok) = ah.mulDivUpX(d + 1, MAX - 1, d);
        assertFalse(ok, "the rounding up passes 2^256 - 1");
        (z, ok) = ah.mulDivUpX(d + 1, MAX - 2, d);
        assertTrue(ok);
        assertEq(z, MAX, "the rounding up lands on 2^256 - 1");
        (z, ok) = ah.mulDivUpX(0, MAX, 7);
        assertTrue(ok);
        assertEq(z, 0);
        (z, ok) = ah.mulDivUpX(1, 1, MAX);
        assertTrue(ok);
        assertEq(z, 1, "anything above zero rounds up to one");
    }

    /// x * 10^k with its overflow, for every x magnitude and k up to 100.
    function testFuzz_mulPow10IsTheModel(uint256 x, uint256 k, uint256 s) public view {
        x = _mag(x, s);
        k = k % 101;
        if (s >> 200 == 0) x = 0;
        (bool ovf, uint256 z) = _pow10Model(x, k);
        (uint256 got, bool ok) = ah.mulPow10X(x, k);
        assertEq(!ok, ovf, "the overflow as the model's");
        if (ok) assertEq(got, z);
    }

    // ==================================================================
    // 2. the conversion: decimals 0 to 36, prices to 2^128, the text's two overflow rules
    // ==================================================================

    struct Conv {
        uint256 decT;
        uint256 decU;
        uint256 fdT;
        uint256 fdU;
        uint256 pT;
        uint256 pU;
        uint256 fixed_;
        uint256 min_;
        uint256 max_;
        uint256 cap;
        uint16 bps;
        uint256 amount;
    }

    struct ConvModel {
        bool overflow;
        uint256 lf;
        uint256 cap;
    }

    /// standard s3: where every absolute amount is zero nothing is scaled; otherwise the scaled price, pU or
    /// pT times the power of ten left after the common one cancels, and every converted amount, rounded up,
    /// must fit 256 bits, else FeeOverflow; then terms/1 s6.1's sum and bounds.
    function _convModel(Conv memory c) internal pure returns (ConvModel memory m) {
        uint256 fixed_ = c.fixed_;
        uint256 min_ = c.min_;
        uint256 max_ = c.max_;
        uint256 cap = c.cap;
        if (fixed_ != 0 || min_ != 0 || max_ != 0 || cap != 0) {
            uint256 a = c.fdT + c.decT;
            uint256 b = c.fdU + c.decU;
            uint256 num = c.pU;
            uint256 den = c.pT;
            bool o;
            if (a >= b) (o, num) = _pow10Model(c.pU, a - b);
            else (o, den) = _pow10Model(c.pT, b - a);
            if (o) {
                m.overflow = true;
                return m;
            }
            bool o1;
            bool o2;
            bool o3;
            bool o4;
            if (fixed_ != 0) (o1, fixed_) = _mulDivUpModel(fixed_, num, den);
            if (min_ != 0) (o2, min_) = _mulDivUpModel(min_, num, den);
            if (max_ != 0) (o3, max_) = _mulDivUpModel(max_, num, den);
            if (cap != 0) (o4, cap) = _mulDivUpModel(cap, num, den);
            if (o1 || o2 || o3 || o4) {
                m.overflow = true;
                return m;
            }
        }
        uint256 part = (c.amount * c.bps) / 10000;
        if (fixed_ > MAX - part) {
            m.overflow = true;
            return m;
        }
        uint256 lf = fixed_ + part;
        if (min_ != 0 && lf < min_) lf = min_;
        if (max_ != 0 && lf > max_) lf = max_;
        m.lf = lf;
        m.cap = cap;
    }

    function _absolute(uint256 w, uint256 s) internal pure returns (uint256) {
        uint256 k = s % 6;
        if (k == 0) return 0;
        if (k == 1) return w; // any word
        if (k == 2) return MAX - (w % 1000);
        return _mag(w, s >> 8);
    }

    function _conv(uint256 s0, uint256 s1, uint256 s2, uint256 s3) internal pure returns (Conv memory c) {
        c.decT = s0 % 37;
        c.decU = (s0 >> 8) % 37;
        c.fdT = (s0 >> 16) % 37;
        c.fdU = (s0 >> 24) % 37;
        c.pT = 1 + (_mag(s1, s0 >> 32) % ((1 << 128) - 1));
        c.pU = 1 + (_mag(s1 >> 128 | s2 << 128, s0 >> 40) % ((1 << 128) - 1));
        if ((s0 >> 48) % 8 == 0) c.pT = (1 << 128) - 1;
        if ((s0 >> 56) % 8 == 0) c.pU = (1 << 128) - 1;
        c.fixed_ = _absolute(uint256(keccak256(abi.encode(s2, 1))), s3);
        c.min_ = _absolute(uint256(keccak256(abi.encode(s2, 2))), s3 >> 8);
        c.max_ = _absolute(uint256(keccak256(abi.encode(s2, 3))), s3 >> 16);
        c.cap = _absolute(uint256(keccak256(abi.encode(s2, 4))), s3 >> 24);
        if ((s3 >> 32) % 6 == 0) {
            c.fixed_ = 0;
            c.min_ = 0;
            c.max_ = 0;
            c.cap = 0;
        }
        c.bps = uint16((s3 >> 40) % 10001);
        c.amount = 1 + _mag(uint256(keccak256(abi.encode(s2, 5))), s3 >> 56) % (MAX / 10000);
        if ((s3 >> 120) % 4 == 0 && c.fixed_ != 0) {
            // feeFixed at the last unit that converts, or one past it
            uint256 a = c.fdT + c.decT;
            uint256 b = c.fdU + c.decU;
            bool o1;
            uint256 num = c.pU;
            uint256 den = c.pT;
            if (a >= b) (o1, num) = _pow10Model(c.pU, a - b);
            else (o1, den) = _pow10Model(c.pT, b - a);
            if (!o1) {
                (uint256 hi, uint256 lo) = _mul512(MAX, den);
                (uint256 qhi, uint256 qlo,) = _divmod512(hi, lo, num);
                if (qhi == 0 && qlo != 0) c.fixed_ = (s3 >> 128) % 2 == 0 || qlo == MAX ? qlo : qlo + 1;
            }
        }
    }

    function _convSetup(Conv memory c) internal returns (Eravon.OrderTerms memory t, bytes memory quote) {
        DecToken settle = new DecToken(c.decT);
        DecToken unit = new DecToken(c.decU);
        RawFeed fT = new RawFeed();
        RawFeed fU = new RawFeed();
        fT.set(c.fdT, 1, int256(c.pT), vm.getBlockTimestamp(), 0);
        fU.set(c.fdU, 1, int256(c.pU), vm.getBlockTimestamp(), 0);
        Q memory q = defaultQuote(address(settle), JUDGE_PAYOUT, true);
        q.feeUnit = address(unit);
        q.feedToken = address(fT);
        q.feedUnit = address(fU);
        q.maxStale = 1 days;
        q.feeFixed = c.fixed_;
        q.feeBps = c.bps;
        q.feeMin = c.min_;
        q.feeMax = c.max_;
        q.amountCap = c.cap;
        quote = encodeQuote(q);
        t = namedTerms(JUDGE, AMANAH, quote, c.amount);
        t.offer.token = address(settle);
    }

    /// The conversion over decimals words 0 to 36 on all four sides and prices to 2^128 - 1 agrees with the
    /// text's model: the overflow exactly where the scaled price or a converted amount passes 2^256 - 1 or
    /// the sum does, otherwise lf and the cap to the unit.
    function testFuzz_conversionIsTheTextsModel(uint256 s0, uint256 s1, uint256 s2, uint256 s3) public {
        Conv memory c = _conv(s0, s1, s2, s3);
        ConvModel memory m = _convModel(c);
        (Eravon.OrderTerms memory t, bytes memory quote) = _convSetup(c);
        try verifier.check(t, quote, "", "", new address[](0), new bytes32[](0)) returns (EravonVerifier.Checked memory got) {
            assertFalse(got.stale, "fresh feeds");
            assertEq(got.overflow, m.overflow, "the overflow as the text's model");
            if (m.overflow) return;
            assertEq(got.lf, m.lf, "lf converted and bounded");
            assertEq(got.cap, m.cap, "the cap converted");
            assertEq(got.priceToken, c.pT);
            assertEq(got.priceUnit, c.pU);
        } catch {
            assertTrue(false, "the verifier's pricing reverted");
        }
    }

    // ==================================================================
    // 3. the feed read over raw answers
    // ==================================================================

    function _answerWord(uint256 s) internal pure returns (uint256) {
        uint256 k = s % 10;
        if (k == 0) return 0;
        if (k == 1) return 1;
        if (k == 2) return (1 << 128) - 1;
        if (k == 3) return 1 << 128;
        if (k == 4) return MAX; // -1
        if (k == 5) return 1 << 255; // int256 min
        if (k == 6) return (1 << 255) - 1;
        if (k == 7) return s >> 128;
        return s;
    }

    function _timeWord(uint256 s, uint256 nowT, uint32 maxStale) internal pure returns (uint256) {
        uint256 k = s % 11;
        if (k == 0) return nowT;
        if (k == 1) return nowT + 1;
        if (k == 2) return nowT >= maxStale ? nowT - maxStale : 0;
        if (k == 3) return nowT >= uint256(maxStale) + 1 ? nowT - maxStale - 1 : 0;
        if (k == 4) return (1 << 64) - 1;
        if (k == 5) return 1 << 64;
        if (k == 6) return 0;
        if (k == 7) return s;
        if (k == 8) return s | (1 << 200);
        return nowT - (s >> 8) % (nowT < uint256(maxStale) + 2 ? nowT : uint256(maxStale) + 2);
    }

    function _decWord(uint256 s) internal pure returns (uint256) {
        uint256 k = s % 6;
        if (k == 0) return 36;
        if (k == 1) return 37;
        if (k == 2) return s >> 8;
        if (k == 3) return 255;
        return (s >> 8) % 40;
    }

    function _rawLen(uint256 s, uint256[13] memory lens) internal pure returns (uint256) {
        if (s % 4 == 0) return (s >> 8) % 300;
        return lens[(s >> 8) % 13];
    }

    /// The model of standard s3's read of one feed, from its raw answers.
    function _feedModel(bytes memory r, bool rr, bytes memory d, bool dr, uint32 maxStale, uint256 nowT)
        internal
        pure
        returns (uint256 price, uint8 dec, uint64 at, bool ok)
    {
        if (rr || r.length < 160) return (0, 0, 0, false);
        uint256 answer;
        uint256 upd;
        assembly ("memory-safe") {
            answer := mload(add(r, 64))
            upd := mload(add(r, 128))
        }
        if (answer == 0 || answer >= (1 << 128)) return (0, 0, 0, false); // not positive (as int256) or past 2^128
        if (upd >= (1 << 64) || upd > nowT || upd + maxStale < nowT) return (0, 0, 0, false);
        if (dr || d.length < 32) return (0, 0, 0, false);
        uint256 x;
        assembly ("memory-safe") {
            x := mload(add(d, 32))
        }
        if (x > 36) return (0, 0, 0, false);
        return (answer, uint8(x), uint64(upd), true);
    }

    /// A feed's two raw answers: latestRoundData of every length around 160 bytes, its answer and updatedAt
    /// words at their type edges, decimals short, long, or past 36, either call reverting now and then, or
    /// burning every unit of gas it is handed.
    struct RawCase {
        bytes r;
        bytes d;
        bool rr;
        bool dr;
        bool rb; // latestRoundData burns its allowance
        bool db; // decimals() burns its allowance
        uint32 maxStale;
    }

    function _rawCase(uint256 s0, uint256 s1, uint256 s2, uint256 s3, uint32 maxStale, uint256 nowT) internal pure returns (RawCase memory rc) {
        if (s0 % 5 == 0) maxStale = uint32(s0 >> 8) % 3;
        uint256[13] memory lens = [uint256(0), 31, 32, 64, 96, 128, 159, 160, 161, 191, 192, 224, 256];
        uint256 rl = _rawLen(s1, lens);
        if ((s1 >> 200) % 3 == 0) rl = 160 + (s1 >> 210) % 64;
        bytes memory r = new bytes(rl);
        for (uint256 i = 0; i < rl; i++) {
            r[i] = bytes1(uint8(uint256(keccak256(abi.encode(s2, i)))));
        }
        if (rl >= 96) {
            uint256 aw = _answerWord(s2);
            assembly ("memory-safe") {
                mstore(add(r, 64), aw)
            }
        }
        if (rl >= 160) {
            uint256 tw = _timeWord(s3, nowT, maxStale);
            assembly ("memory-safe") {
                mstore(add(r, 128), tw)
            }
        }
        uint256 dl = s3 % 3 == 0 ? (s3 >> 8) % 70 : 32;
        bytes memory d = new bytes(dl);
        if (dl >= 32) {
            uint256 dw = _decWord(s3 >> 16);
            assembly ("memory-safe") {
                mstore(add(d, 32), dw)
            }
        } else {
            for (uint256 i = 0; i < dl; i++) {
                d[i] = bytes1(uint8(s3 >> (i % 30)));
            }
        }
        rc.r = r;
        rc.d = d;
        rc.rr = (s0 >> 16) % 16 == 0;
        rc.dr = (s0 >> 24) % 16 == 0;
        rc.rb = (s0 >> 40) % 16 == 0;
        rc.db = (s0 >> 48) % 16 == 0;
        rc.maxStale = maxStale;
    }

    /// _readFeed over raw answers of every length, the answer and updatedAt words at their type edges and
    /// the decimals answer short, long, or past 36, and either read burning its gas: never a revert, always
    /// the model's verdict and values (a read that runs out of its 200,000 gas is one that did not answer),
    /// and the whole pricing under 2,000,000 gas.
    function testFuzz_readFeedOverRawAnswers(uint256 s0, uint256 s1, uint256 s2, uint256 s3, uint32 maxStale) public {
        uint256 nowT = vm.getBlockTimestamp();
        RawCase memory rc = _rawCase(s0, s1, s2, s3, maxStale, nowT);
        maxStale = rc.maxStale;
        bytes memory r = rc.r;
        bytes memory d = rc.d;
        // a read that burns its allowance answers as one that reverted
        bool rr = rc.rr || rc.rb;
        bool dr = rc.dr || rc.db;
        BytesFeed f = new BytesFeed();
        f.set(r, d, rc.rr, rc.dr);
        f.setBurns(rc.rb, rc.db);
        address feed = (s0 >> 32) % 20 == 0 ? address(uint160(uint256(keccak256(abi.encode(s0, "no code"))))) : address(f);
        (uint256 mp, uint8 md, uint64 mat, bool mok) =
            feed == address(f) ? _feedModel(r, rr, d, dr, maxStale, nowT) : (uint256(0), uint8(0), uint64(0), false);
        try ah.readFeedX{gas: 1_000_000}(feed, maxStale) returns (uint256 p, uint8 dd, uint64 at, bool ok) {
            assertEq(ok, mok, "fresh or stale as the model's");
            assertEq(p, mp);
            assertEq(dd, md);
            assertEq(at, mat);
        } catch {
            assertTrue(false, "the feed read reverted");
        }
        // and through the whole pricing: the feed's verdict decides RateStale
        DecToken unit = new DecToken(6);
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.feeUnit = address(unit);
        q.feedToken = feed;
        q.maxStale = maxStale;
        q.feeMin = 5e6;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        try verifier.check{gas: 2_000_000}(t, quote, "", "", new address[](0), new bytes32[](0)) returns (EravonVerifier.Checked memory c) {
            assertEq(c.stale, !mok, "RateStale exactly where the feed is not ok");
        } catch {
            assertTrue(false, "check reverted");
        }
    }

    // ==================================================================
    // 5. the terms guards at their bounds, several broken at once
    // ==================================================================

    /// standard s4's table, row by row in its order.
    function _tableModel(Eravon.OrderTerms memory t) internal pure returns (uint8) {
        Eravon.Offer memory o = t.offer;
        if (o.sellerKey == address(0) || t.buyerKey == address(0) || o.sellerKey == t.buyerKey) return 1;
        if ((o.judge != address(0)) == (o.guild != address(0))) return 2;
        address seat = o.judge != address(0) ? o.judge : o.guild;
        if (seat == o.sellerKey || seat == t.buyerKey) return 3;
        if (o.sellerPayout == address(0) || t.buyerPayout == address(0)) return 4;
        if (o.token == address(0)) return 5;
        if (o.amount == 0 || o.amount > MAX / 10000) return 6;
        if (o.ladder.length < 2 || o.ladder.length > 16) return 7;
        for (uint256 i = 0; i < o.ladder.length; i++) {
            if (uint256(o.ladder[i].buyerBps) + o.ladder[i].sellerBps != 10000) return 8;
            if (o.ladder[i].feeBearer > 2) return 9;
        }
        if (o.ladder[0].buyerBps != 0) return 11;
        if (o.ladder[o.ladder.length - 1].buyerBps != 10000) return 12;
        for (uint256 i = 1; i < o.ladder.length; i++) {
            if (o.ladder[i].buyerBps < o.ladder[i - 1].buyerBps) return 10;
        }
        if (o.termsHash == bytes32(0)) return 13;
        if (o.amanahHash == bytes32(0)) return 14;
        if (o.quoteHash == bytes32(0)) return 15;
        if (o.nonce > MAX_INT53 || o.expiry > MAX_INT53 || t.openBy > MAX_INT53) return 16;
        return 0;
    }

    function _near(uint256 bound_, uint256 s) internal pure returns (uint256) {
        uint256 k = s % 3;
        return k == 0 ? bound_ - 1 : (k == 1 ? bound_ : bound_ + 1);
    }

    /// Each guard broken with probability one in four, independently, the amount and the three ints drawn
    /// at their bound, one below, one above; the guild or the judge equal to a party; the verifier names the
    /// table's first failing row, and init refuses with it, before any quote or signature is read.
    function testFuzz_guardsAtTheBoundsSeveralBroken(uint256 s, uint256 w) public {
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, abi.encode(w), 1 ether);
        t.offer.termsHash = keccak256("terms");
        if ((s >> 1) & 1 == 1) {
            t.offer.judge = address(0);
            t.offer.guild = GUILD;
        }
        if ((s >> 2) % 4 == 0) {
            uint256 v = (s >> 4) % 3;
            if (v == 0) t.offer.sellerKey = address(0);
            else if (v == 1) t.buyerKey = address(0);
            else t.buyerKey = t.offer.sellerKey;
        }
        if ((s >> 6) % 4 == 0) {
            if ((s >> 8) & 1 == 0) (t.offer.judge, t.offer.guild) = (address(0), address(0));
            else (t.offer.judge, t.offer.guild) = (JUDGE, GUILD);
        }
        if ((s >> 9) % 4 == 0) {
            address party = (s >> 11) & 1 == 0 ? t.offer.sellerKey : t.buyerKey;
            if (t.offer.guild != address(0)) t.offer.guild = party;
            else t.offer.judge = party;
        }
        if ((s >> 12) % 4 == 0) {
            if ((s >> 14) & 1 == 0) t.offer.sellerPayout = address(0);
            else t.buyerPayout = address(0);
        }
        if ((s >> 15) % 4 == 0) t.offer.token = address(0);
        // the amount: at the bound, below it, above it, zero, or ordinary
        uint256 am = (s >> 17) % 5;
        if (am <= 2) t.offer.amount = _near(MAX / 10000, s >> 19);
        else if (am == 3) t.offer.amount = (s >> 21) % 4 == 0 ? 0 : MAX - (w % 1000);
        if ((s >> 23) % 4 == 0) {
            uint256 n = (s >> 25) % 2 == 0 ? (s >> 26) % 2 : 17 + (s >> 27) % 4;
            Eravon.Rung[] memory l = new Eravon.Rung[](n);
            for (uint256 i = 0; i < n; i++) {
                uint16 b = uint16(n < 2 ? 0 : (10000 * i) / (n - 1));
                l[i] = Eravon.Rung(b, 10000 - b, 0);
            }
            t.offer.ladder = l;
        }
        uint256 n2 = t.offer.ladder.length;
        if (n2 >= 2) {
            if ((s >> 30) % 6 == 0) t.offer.ladder[(s >> 33) % n2].sellerBps += 1;
            if ((s >> 36) % 6 == 0) t.offer.ladder[(s >> 39) % n2].feeBearer = uint8(3 + (s >> 42) % 200);
            if ((s >> 50) % 6 == 0) t.offer.ladder[0] = Eravon.Rung(1, 9999, 0);
            if ((s >> 53) % 6 == 0) t.offer.ladder[n2 - 1] = Eravon.Rung(9999, 1, 0);
            if ((s >> 56) % 6 == 0 && n2 >= 3) {
                t.offer.ladder[1] = Eravon.Rung(6000, 4000, 0);
                t.offer.ladder[2] = Eravon.Rung(5999, 4001, 0);
            }
        }
        if ((s >> 60) % 4 == 0) t.offer.termsHash = bytes32(0);
        if ((s >> 62) % 4 == 0) t.offer.amanahHash = bytes32(0);
        if ((s >> 64) % 4 == 0) t.offer.quoteHash = bytes32(0);
        // the three ints at 2^53 - 1, one below, one above
        if ((s >> 66) % 3 == 0) t.offer.nonce = _near(MAX_INT53, s >> 68);
        if ((s >> 70) % 3 == 0) t.offer.expiry = _near(MAX_INT53, s >> 72);
        if ((s >> 74) % 3 == 0) t.openBy = uint64(_near(MAX_INT53, s >> 76));
        t.offer.buyerMayOpen = true;
        uint8 want = _tableModel(t);
        EravonVerifier.Checked memory c = verifier.check(t, new bytes(480), "", "", new address[](0), new bytes32[](0));
        assertEq(c.termsFault, want, "the first row of the table that fails");
        if (want == 0) return;
        Eravon inst = deployProxy(t);
        // the caller rule comes before the guards: a party opens, and with no party key nobody can
        address caller = t.offer.sellerKey != address(0) ? t.offer.sellerKey : t.buyerKey;
        if (caller == address(0)) {
            vm.prank(STRANGER);
            vm.expectRevert(Eravon.NotParty.selector);
        } else {
            vm.prank(caller);
            vm.expectRevert(abi.encodeWithSelector(Eravon.BadTerms.selector, want));
        }
        inst.init(t, new bytes(480), "", "", new address[](0), new bytes32[](0));
    }

    /// An order at every bound at once opens: the amount at (2^256 - 1) / 10000 and nonce, expiry and openBy
    /// at 2^53 - 1; one past any of them is refused by its row.
    function test_orderAtEveryBoundOpens() public {
        uint256 a = MAX / 10000;
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.premiumBps = 10000;
        q.feeBps = 10000;
        q.feeMax = 0;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, a);
        t.offer.nonce = MAX_INT53;
        t.offer.expiry = MAX_INT53;
        t.openBy = uint64(MAX_INT53);
        uint256 p = premiumOf(q, a);
        uint256 lf = lfOf(q, a);
        assertEq(p, a);
        assertEq(lf, a, "lf = A x 10000 / 10000, unbounded");
        fundDeposit(p + lf);
        Eravon inst = openNamed(t, quote);
        assertEq(depositHeld(inst), 2 * a, "D = P + lf = 2A");
        // every share at the bound is exact: a settlement at 3333 bps
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 d = digestOf(address(inst), keccak256(abi.encode(TH_SETTLE, uint256(3333), exp)));
        vm.prank(SELLER);
        inst.settle(3333, exp, sign(BUYER_PK, d), sign(SELLER_PK, d));
        assertEq(tok.balanceOf(BUYER_PAYOUT), (a * 3333) / 10000);
        assertEq(tok.balanceOf(SELLER_PAYOUT), a - (a * 3333) / 10000);
        (, uint256 jc) = inst.accountOf(JUDGE, address(tok));
        assertEq(jc, p, "P to the accepted seat within its window");
        assertEq(tok.balanceOf(SELLER), lf);
        // one past each bound
        for (uint256 k = 0; k < 4; k++) {
            Eravon.OrderTerms memory u = namedTerms(JUDGE, AMANAH, quote, a);
            if (k == 0) u.offer.amount = a + 1;
            if (k == 1) u.offer.nonce = MAX_INT53 + 1;
            if (k == 2) u.offer.expiry = MAX_INT53 + 1;
            if (k == 3) u.openBy = uint64(MAX_INT53 + 1);
            EravonVerifier.Checked memory c = verifier.check(u, quote, "", "", new address[](0), new bytes32[](0));
            assertEq(c.termsFault, k == 0 ? 6 : 16);
        }
    }

    // ==================================================================
    // 6. the seller's recovery against ecrecover
    // ==================================================================

    function _edgeScalar(uint256 w, uint256 s) internal pure returns (uint256) {
        uint256 k = s % 9;
        if (k == 0) return 0;
        if (k == 1) return 1;
        if (k == 2) return SECP_N - 1;
        if (k == 3) return SECP_N;
        if (k == 4) return SECP_N / 2;
        if (k == 5) return SECP_N / 2 + 1;
        if (k == 6) return MAX;
        return w;
    }

    /// verifySeller admits a 65-byte signature exactly where v is 27 or 28, r in [1, n - 1], s in
    /// [1, n / 2] and ecrecover answers the key, nonzero; low s and the two v values tested at their edges.
    function testFuzz_verifySellerIsLowSEcrecover(bytes32 d, uint256 r, uint256 s, uint8 v, uint256 mode, uint256 pk) public view {
        if (mode % 3 == 0) {
            pk = bound(pk, 1, SECP_N - 1);
            (uint8 v_, bytes32 r_, bytes32 s_) = vm.sign(pk, d);
            r = uint256(r_);
            s = uint256(s_);
            v = v_;
            if ((mode >> 8) % 2 == 0 && s > SECP_N / 2) {
                s = SECP_N - s;
                v = v == 27 ? 28 : 27;
            }
        } else {
            r = _edgeScalar(r, mode >> 16);
            s = _edgeScalar(s, mode >> 24);
            if ((mode >> 32) % 3 != 0) v = 27 + uint8((mode >> 40) % 2);
        }
        bytes memory sig = abi.encodePacked(r, s, v);
        address who = address(0);
        if ((v == 27 || v == 28) && r != 0 && r < SECP_N && s != 0 && s <= SECP_N / 2) who = ecrecover(d, v, bytes32(r), bytes32(s));
        address key = (mode >> 48) % 2 == 0 ? who : address(uint160(uint256(keccak256(abi.encode(d, mode)))));
        bool want = who != address(0) && key == who;
        try verifier.verifySeller(d, sig, key) {
            assertTrue(want, "admitted where the model refuses");
        } catch (bytes memory err) {
            assertFalse(want, "refused where the model admits");
            assertEq(bytes4(err), Eravon.BadSellerSig.selector);
            assertEq(err.length, 4);
        }
        // any other length refuses
        bytes memory longer = abi.encodePacked(sig, uint8(0));
        try verifier.verifySeller(d, longer, key) {
            assertTrue(false, "a 66-byte signature admitted");
        } catch (bytes memory err) {
            assertEq(bytes4(err), Eravon.BadSellerSig.selector);
        }
    }

    // ==================================================================
    // 7. the refund during and after a dispute at random ladders, with strays in every phase
    // ==================================================================

    function _ladder(uint256 seed, uint256 n) internal pure returns (Eravon.Rung[] memory l) {
        l = new Eravon.Rung[](n);
        uint256[] memory b = new uint256[](n);
        for (uint256 i = 1; i + 1 < n; i++) {
            b[i] = uint256(keccak256(abi.encode(seed, i))) % 10001;
        }
        b[n - 1] = 10000;
        for (uint256 i = 1; i < n; i++) {
            for (uint256 k = i; k > 0 && b[k] < b[k - 1]; k--) {
                (b[k], b[k - 1]) = (b[k - 1], b[k]);
            }
        }
        for (uint256 i = 0; i < n; i++) {
            l[i] = Eravon.Rung(uint16(b[i]), uint16(10000 - b[i]), uint8(uint256(keccak256(abi.encode(seed, i, "fb"))) % 3));
        }
    }

    struct RefundCase {
        uint256 a;
        uint256 p;
        uint256 lf;
        uint256 sF;
        uint256 sC;
        uint256 sD;
        uint256 n;
    }

    /// A disputed case under a random ladder, strays sent while Funded, Claimed and Disputed (the fee taken
    /// from the first two where they cover it, the key pulled for the rest): the seller's refund at or before
    /// T3's end is refused, WrongPhase, nothing moving; from one second later it is the plain refund,
    /// Closed(0, A + strays beyond the fee + lf, P + lf, 0, 0), nothing to the judge.
    function testFuzz_refundRefusedInDisputeThenPlainWithStrays(uint256 seed, uint256 amount, uint16 premium, uint256 strays, uint256 at) public {
        RefundCase memory c;
        c.n = 2 + seed % 15;
        c.a = (seed >> 8) % 4 == 0 ? MAX / 10000 - (amount % 1000) : bound(amount, 1, 1e30);
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.premiumBps = uint16(bound(premium, 0, 10000));
        q.feeBps = uint16((seed >> 16) % 10001);
        q.feeMax = (seed >> 32) % 2 == 0 ? 0 : 500 ether;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, c.a);
        t.offer.ladder = _ladder(seed, c.n);
        c.p = premiumOf(q, c.a);
        c.lf = lfOf(q, c.a);
        fundDeposit(c.p + c.lf);
        Eravon inst = openNamed(t, quote);
        c.sF = (strays % 3 == 0) ? 0 : (strays >> 8) % (2 * c.lf + 2);
        c.sC = ((strays >> 80) % 3 == 0) ? 0 : (strays >> 88) % 1e24;
        c.sD = ((strays >> 160) % 3 == 0) ? 0 : (strays >> 168) % 1e24;
        if (c.sF != 0) tok.mint(address(inst), c.sF);
        claim(inst, MANIFEST);
        if (c.sC != 0) tok.mint(address(inst), c.sC);
        uint256 have = c.sF + c.sC;
        if (have < c.lf) {
            tok.mint(BUYER, c.lf - have);
            vm.prank(BUYER);
            tok.approve(address(inst), c.lf - have);
        }
        vm.prank(BUYER);
        inst.dispute();
        uint256 strayAfter = (have > c.lf ? have - c.lf : 0) + c.sD;
        if (c.sD != 0) tok.mint(address(inst), c.sD);
        uint64 end = inst.seatOf(orderHash(t), 0).windowEnd;
        bool within = at % 2 == 0;
        vm.warp(within ? vm.getBlockTimestamp() + (at >> 8) % (uint256(end) - vm.getBlockTimestamp() + 1) : uint256(end) + 1);
        if (within) {
            vm.prank(SELLER);
            vm.expectRevert(Eravon.WrongPhase.selector);
            inst.refund();
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Disputed), "nothing moved");
            vm.warp(uint256(end) + 1 + (at >> 200) % 30 days);
        }
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Closed(0, c.a + strayAfter + c.lf, c.p + c.lf, 0, 0);
        vm.prank(SELLER);
        inst.refund();
        (, uint256 jc) = inst.accountOf(JUDGE, address(tok));
        assertEq(jc, 0, "nothing to the judge at a refund");
        assertEq(tok.balanceOf(address(inst)), 0, "nothing left");
        assertEq(inst.seatOf(orderHash(t), 0).state, 5);
    }

    // ==================================================================
    // the generators' spread
    // ==================================================================

    /// The generators above reach every class they are meant to: the conversion with nothing scaled, its
    /// scaled price past 2^256 - 1, a converted amount past it, the sum past it, and priced; the feed fresh
    /// and stale, and a read burning its allowance. Deterministic;
    /// the counts are logged.
    function test_arithSpread() public {
        vm.pauseGasMetering();
        uint256[5] memory conv; // nothing scaled, scaled price overflow, converted overflow, sum overflow, priced
        uint256[2] memory fd; // fresh, stale
        uint256 burns; // a read that burns its allowance
        for (uint256 k = 0; k < 3000; k++) {
            (uint8 cc, bool f, bool b) = this.spreadOne(k);
            conv[cc]++;
            if (f) fd[0]++;
            else fd[1]++;
            if (b) burns++;
        }
        string[5] memory cn = ["conv: nothing scaled", "conv: scaled price past 2^256", "conv: a converted amount past 2^256", "conv: the sum past 2^256", "conv: priced"];
        for (uint256 i = 0; i < 5; i++) {
            emit log_named_uint(cn[i], conv[i]);
            assertGt(conv[i], 0, cn[i]);
        }
        emit log_named_uint("feed: fresh", fd[0]);
        emit log_named_uint("feed: stale", fd[1]);
        assertGt(fd[0], 0);
        assertGt(fd[1], 0);
        emit log_named_uint("feed: a read burns its allowance", burns);
        assertGt(burns, 0);
    }

    /// One draw of the two generators, classified (an external call, so that each draw has its own memory).
    function spreadOne(uint256 k) external view returns (uint8 cc, bool f, bool burn) {
        uint256 a = uint256(keccak256(abi.encode(k, 1)));
        uint256 b = uint256(keccak256(abi.encode(k, 2)));
        uint256 c = uint256(keccak256(abi.encode(k, 3)));
        uint256 d = uint256(keccak256(abi.encode(k, 4)));
        Conv memory cv = _conv(a, b, c, d);
        bool none = cv.fixed_ == 0 && cv.min_ == 0 && cv.max_ == 0 && cv.cap == 0;
        uint256 x = cv.fdT + cv.decT;
        uint256 y = cv.fdU + cv.decU;
        bool so;
        if (x >= y) (so,) = _pow10Model(cv.pU, x - y);
        else (so,) = _pow10Model(cv.pT, y - x);
        ConvModel memory m = _convModel(cv);
        if (none) {
            cc = 0;
        } else if (so) {
            cc = 1;
        } else if (m.overflow) {
            Conv memory z = cv;
            z.bps = 0;
            z.amount = 1;
            cc = _convModel(z).overflow ? 2 : 3;
        } else {
            cc = 4;
        }
        if (none && m.overflow) cc = 3;
        RawCase memory rc = _rawCase(a, b, c, d, uint32(d >> 100), vm.getBlockTimestamp());
        (,,, f) = _feedModel(rc.r, rc.rr || rc.rb, rc.d, rc.dr || rc.db, rc.maxStale, vm.getBlockTimestamp());
        burn = rc.rb || rc.db;
    }
}
