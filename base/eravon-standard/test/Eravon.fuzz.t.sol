// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {EravonVerifier} from "../src/EravonVerifier.sol";

/// A settlement token or reference unit whose decimals() word is set by the test.
contract DecToken {
    uint256 public decimals;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    constructor(uint256 d) {
        decimals = d;
    }

    function mint(address to, uint256 v) external {
        balanceOf[to] += v;
    }

    function approve(address s, uint256 v) external returns (bool) {
        allowance[msg.sender][s] = v;
        return true;
    }

    function transfer(address to, uint256 v) external returns (bool) {
        balanceOf[msg.sender] -= v;
        balanceOf[to] += v;
        return true;
    }

    function transferFrom(address from, address to, uint256 v) external returns (bool) {
        allowance[from][msg.sender] -= v;
        balanceOf[from] -= v;
        balanceOf[to] += v;
        return true;
    }
}

/// A Chainlink-shaped feed whose every word, its decimals and its failure modes are set by the test.
contract RawFeed {
    uint256 public dec;
    uint256 public roundId;
    int256 public answer;
    uint256 public updatedAt;
    uint8 public mode; // 0 answers, 1 latestRoundData reverts, 2 answers 64 bytes, 3 decimals reverts

    function set(uint256 d, uint256 r, int256 a, uint256 t, uint8 m) external {
        dec = d;
        roundId = r;
        answer = a;
        updatedAt = t;
        mode = m;
    }

    function decimals() external view returns (uint256) {
        if (mode == 3) revert("no decimals");
        return dec;
    }

    function latestRoundData() external view returns (uint256, int256, uint256, uint256, uint256) {
        if (mode == 1) revert("no round");
        if (mode == 2) {
            assembly ("memory-safe") {
                mstore(0, 1)
                mstore(32, 1)
                return(0, 64)
            }
        }
        return (roundId, answer, updatedAt, updatedAt, roundId);
    }
}

/// The verifier with its signature recovery exposed (the logic holds none: its two seller paths call the
/// verifier's verifySeller, terms/1 s5.1).
contract VerifierHarness is EravonVerifier {
    function recoverX(bytes32 d, bytes calldata sig) external pure returns (address) {
        return _recover(d, sig);
    }
}

/// Stateless fuzz of the standard's arithmetic and decoders: init's sixteen
/// terms guards and the quote decoder over random words against a model; the
/// fee priced in the settlement token and in a reference unit against a model
/// (the conversion computed exactly, in limbs, apart from the verifier's own
/// code), through the verifier and through init; the ladder's split at every
/// rung up to 1e30 and at 2^128, the refund refused within T3 and plain past it;
/// the secp256k1 recovery over random signatures, on the verifier and through the logic's
/// signed seller actions.
contract EravonStatelessFuzz is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE_PAYOUT = address(0x4000000000000000000000000000000000000001);
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    address internal constant STRANGER = address(0x3000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah");
    bytes32 internal constant MANIFEST = keccak256("manifest");
    bytes internal constant ENTRY = "judgment entry bytes";
    uint256 internal constant SECP_N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141;
    uint256 internal constant MAX_INT53 = 9007199254740991;
    bytes32 internal E;
    VerifierHarness internal vh;
    /// A live named case, Funded with its seat accepted, for the logic's signed seller paths.
    Eravon internal live;

    function setUp() public {
        setUpBase();
        E = sha256(ENTRY);
        vh = new VerifierHarness();
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        fundDeposit(q, 1 ether);
        live = openNamed(t, quote);
    }

    // ==================================================================
    // 1. init's sixteen terms guards and the quote decoder
    // ==================================================================

    /// standard s4 step 5 as the verifier numbers it, written from terms/1 s2.5 and s4.1: the parties, the
    /// tracks, the judge or guild against the parties, the payouts, the token, the amount within
    /// (2^256 - 1) / 10000, the ladder's size, every rung's sum and bearer, then the first rung, the last
    /// rung, the order; the three hashes; the three integers the documents spell within 2^53 - 1.
    function _termsModel(Eravon.OrderTerms memory t) internal pure returns (uint8) {
        Eravon.Offer memory o = t.offer;
        if (o.sellerKey == address(0) || t.buyerKey == address(0) || o.sellerKey == t.buyerKey) return 1;
        if ((o.judge == address(0)) == (o.guild == address(0))) return 2;
        if (o.judge != address(0) && (o.judge == o.sellerKey || o.judge == t.buyerKey)) return 3;
        if (o.guild != address(0) && (o.guild == o.sellerKey || o.guild == t.buyerKey)) return 3;
        if (o.sellerPayout == address(0) || o.sellerFund == address(0) || t.buyerPayout == address(0)) return 4;
        if (o.token == address(0)) return 5;
        if (o.amount == 0 || o.amount > type(uint256).max / 10000) return 6;
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

    /// Terms from random words over a small pool of addresses, so that the equalities the guards test occur.
    function _randomTerms(uint256 w0, uint256 w1, uint256 lw, uint8 n) internal view returns (Eravon.OrderTerms memory t) {
        address[5] memory pool = [address(0), SELLER, BUYER, JUDGE, GUILD];
        // three in four draws keep each field valid, the fourth draws from the pool or zero
        t.offer.sellerKey = (w0 & 3) == 0 ? pool[(w0 >> 2) % 5] : SELLER;
        t.offer.sellerFund = t.offer.sellerKey == address(0) ? SELLER : t.offer.sellerKey;
        t.buyerKey = ((w0 >> 5) & 3) == 0 ? pool[(w0 >> 7) % 5] : BUYER;
        bool named = (w0 >> 10) & 1 == 0;
        t.offer.judge = named ? JUDGE : address(0);
        t.offer.guild = named ? address(0) : GUILD;
        if (((w0 >> 11) & 7) == 0) t.offer.judge = pool[(w0 >> 14) % 5];
        if (((w0 >> 17) & 7) == 0) t.offer.guild = pool[(w0 >> 20) % 5];
        t.offer.sellerPayout = ((w0 >> 23) & 7) == 0 ? address(0) : SELLER_PAYOUT;
        t.buyerPayout = ((w0 >> 26) & 7) == 0 ? address(0) : BUYER_PAYOUT;
        t.offer.token = ((w0 >> 29) & 7) == 0 ? address(0) : address(tok);
        // zero, a word past the bound, or an amount in range
        t.offer.amount = ((w0 >> 32) & 7) == 0 ? 0 : (((w0 >> 55) & 7) == 0 ? (w1 | (1 << 255)) : 1 + (w1 % 1e30));
        uint256 len = ((w0 >> 35) & 3) == 0 ? (n % 19) : 2 + (n % 15);
        t.offer.ladder = new Eravon.Rung[](len);
        uint256 hs = lw;
        for (uint256 i = 0; i < len; i++) {
            hs = uint256(keccak256(abi.encode(hs, i)));
            uint16 b = uint16(len < 2 ? 0 : (10000 * i) / (len - 1));
            if (hs % 7 == 0) b = uint16(hs >> 8);
            uint16 sb = 10000 - (b > 10000 ? 10000 : b);
            if ((hs >> 24) % 9 == 0) sb = uint16(hs >> 32);
            uint8 fb = uint8((hs >> 48) % 3);
            if ((hs >> 56) % 11 == 0) fb = uint8(hs >> 64);
            t.offer.ladder[i] = Eravon.Rung(b, sb, fb);
        }
        t.offer.termsHash = ((w0 >> 38) & 7) == 0 ? bytes32(0) : keccak256("terms");
        t.offer.amanahHash = ((w0 >> 41) & 7) == 0 ? bytes32(0) : AMANAH;
        t.offer.quoteHash = ((w0 >> 44) & 7) == 0 ? bytes32(0) : keccak256("quote");
        t.offer.buyerMayOpen = true;
        t.offer.deliveryWindow = uint32(w1 >> 64);
        t.offer.confirmWindow = uint32(w1 >> 96);
        t.offer.openWindow = uint32(w1 >> 128);
        // the three integers: in range, or one in eight just past 2^53 - 1 (an expiry and an openBy past it lie in the future)
        t.offer.nonce = ((w0 >> 58) & 7) == 0 ? MAX_INT53 + 1 + (w1 >> 200) : (w1 >> 200) % MAX_INT53;
        t.offer.expiry = ((w0 >> 61) & 7) == 0 ? MAX_INT53 + 1 + (lw % 1e9) : vm.getBlockTimestamp() + 30 days;
        t.openBy = ((w0 >> 64) & 7) == 0 ? uint64(MAX_INT53 + 1 + (lw >> 128) % 1e9) : uint64(vm.getBlockTimestamp() + 1 days);
    }

    /// A valid order with one guard broken on purpose (guard g, 0 for none); the model still decides.
    function _mutatedTerms(uint256 w0, uint256 w1, uint256 lw) internal returns (Eravon.OrderTerms memory t) {
        t = namedTerms(JUDGE, AMANAH, abi.encode(w1), 1 + (w1 % 1e30));
        t.offer.termsHash = keccak256("terms");
        uint256 n = 2 + (lw % 15);
        Eravon.Rung[] memory l = new Eravon.Rung[](n);
        for (uint256 i = 0; i < n; i++) {
            uint16 b = uint16((10000 * i) / (n - 1));
            l[i] = Eravon.Rung(b, 10000 - b, uint8((lw >> (i + 8)) % 3));
        }
        t.offer.ladder = l;
        if ((w0 >> 200) & 1 == 1) {
            t.offer.judge = address(0);
            t.offer.guild = GUILD;
        }
        uint256 g = w0 % 17;
        uint256 v = (w0 >> 8) % 3;
        uint256 k = (w0 >> 16) % n;
        if (g == 1) {
            if (v == 0) t.offer.sellerKey = address(0);
            else if (v == 1) t.buyerKey = address(0);
            else t.buyerKey = t.offer.sellerKey;
        } else if (g == 2) {
            if (v == 0) t.offer.judge = t.offer.guild = address(0);
            else (t.offer.judge, t.offer.guild) = (JUDGE, GUILD);
        } else if (g == 3) {
            // a named judge, or the guild, that is one of the parties
            address party = (w0 >> 24) & 1 == 0 ? t.offer.sellerKey : t.buyerKey;
            if (v == 2) (t.offer.judge, t.offer.guild) = (address(0), party);
            else (t.offer.judge, t.offer.guild) = (party, address(0));
        } else if (g == 4) {
            if (v == 0) t.offer.sellerPayout = address(0);
            else t.buyerPayout = address(0);
        } else if (g == 5) {
            t.offer.token = address(0);
        } else if (g == 6) {
            t.offer.amount = v == 0 ? 0 : (v == 1 ? type(uint256).max / 10000 + 1 + (w1 % 1e9) : type(uint256).max - (w1 % 1e9));
        } else if (g == 7) {
            uint256 bad = v == 0 ? 0 : (v == 1 ? 1 : 17 + (w1 % 8));
            Eravon.Rung[] memory m = new Eravon.Rung[](bad);
            for (uint256 i = 0; i < bad; i++) {
                uint16 b = uint16(bad < 2 ? 0 : (10000 * i) / (bad - 1));
                m[i] = Eravon.Rung(b, 10000 - b, 0);
            }
            t.offer.ladder = m;
        } else if (g == 8) {
            t.offer.ladder[k].sellerBps = uint16(uint256(t.offer.ladder[k].sellerBps) + 1 + (w1 % 50000));
        } else if (g == 9) {
            t.offer.ladder[k].feeBearer = uint8(3 + (w1 % 253));
        } else if (g == 10 && n >= 3) {
            uint256 j = 1 + (k % (n - 2));
            // rung j below its predecessor, sums kept
            uint16 b = t.offer.ladder[j - 1].buyerBps == 0 ? 0 : t.offer.ladder[j - 1].buyerBps - 1;
            if (t.offer.ladder[j - 1].buyerBps == 0) {
                t.offer.ladder[j - 1] = Eravon.Rung(1, 9999, 0);
            }
            t.offer.ladder[j] = Eravon.Rung(b, 10000 - b, 0);
        } else if (g == 11) {
            uint16 b = uint16(1 + (w1 % t.offer.ladder[1].buyerBps));
            t.offer.ladder[0] = Eravon.Rung(b, 10000 - b, 0);
        } else if (g == 12) {
            uint16 lo = t.offer.ladder[n - 2].buyerBps;
            uint16 b = uint16(lo + (w1 % (10000 - uint256(lo))));
            t.offer.ladder[n - 1] = Eravon.Rung(b, 10000 - b, 0);
        } else if (g == 13) {
            t.offer.termsHash = bytes32(0);
        } else if (g == 14) {
            t.offer.amanahHash = bytes32(0);
        } else if (g == 15) {
            t.offer.quoteHash = bytes32(0);
        } else if (g == 16) {
            if (v == 0) t.offer.nonce = MAX_INT53 + 1 + (w1 % 1e9);
            else if (v == 1) t.offer.expiry = MAX_INT53 + 1 + (w1 % 1e9);
            else t.openBy = uint64(MAX_INT53 + 1 + (w1 % 1e9));
        }
        t.offer.buyerMayOpen = true;
    }

    /// The verifier names the first failing guard as the model does; init reverts with it
    /// before the quote is read (standard s4 step 5's order).
    function testFuzz_initGuards(uint256 w0, uint256 w1, uint256 lw, uint8 n) public {
        Eravon.OrderTerms memory t = (w0 >> 250) % 2 == 0 ? _mutatedTerms(w0, w1, lw) : _randomTerms(w0, w1, lw, n);
        uint8 want = _termsModel(t);
        EravonVerifier.Checked memory c = verifier.check(t, new bytes(480), "", "", new address[](0), new bytes32[](0));
        assertEq(c.termsFault, want, "the verifier's guard is the model's");
        assertTrue(c.quoteBad, "an all-zero quote names no token");
        if (want == 0) return;
        Eravon inst = deployProxy(t);
        // the caller rule (s4 step 4) comes before the guards: a party opens, and with no party key nobody can
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

    /// terms/1 s3.2: the fifteen words each made valid or left random.
    function _coerce(uint256 k, bytes32 w) internal pure returns (bytes32) {
        uint256 x = uint256(w);
        if (k == 0 || k == 2 || k == 3 || k == 4) {
            x &= type(uint160).max;
            if (k == 0 && x == 0) x = 1;
        } else if (k == 1 || k == 7) {
            x %= 10001;
        } else if (k == 5 || k == 11 || k == 12) {
            x &= type(uint32).max;
        } else if (k == 13) {
            x %= 3;
        } else if (k == 14) {
            x &= 1;
        } else {
            x %= 1e30; // feeFixed, feeMin, feeMax, amountCap: any word; kept small here, raw in the other branch
        }
        return bytes32(x);
    }

    function _quoteModel(bytes memory q) internal pure returns (bool bad, uint256[15] memory w) {
        if (q.length != 480) return (true, w);
        for (uint256 i = 0; i < 15; i++) {
            uint256 x;
            assembly ("memory-safe") {
                x := mload(add(add(q, 32), mul(i, 32)))
            }
            w[i] = x;
        }
        uint256[4] memory addrs = [uint256(0), 2, 3, 4];
        for (uint256 k = 0; k < 4; k++) {
            if (w[addrs[k]] >> 160 != 0) return (true, w);
        }
        if (w[0] == 0) return (true, w);
        if (w[1] > 10000 || w[7] > 10000) return (true, w);
        if (w[5] > type(uint32).max || w[11] > type(uint32).max || w[12] > type(uint32).max) return (true, w);
        if (w[13] > 2) return (true, w);
        if (w[14] > 1) return (true, w);
        if (w[2] == 0 && (w[3] != 0 || w[4] != 0)) return (true, w);
    }

    /// The fee in the settlement token (terms/1 s6.1) for an amount within the bound, or the overflow terms/1
    /// names E_FEE_OVERFLOW (the sum feeFixed + the share past 2^256 - 1).
    function _lfModel(uint256 amount, uint256 fixed_, uint256 bps, uint256 min_, uint256 max_)
        internal
        pure
        returns (bool overflow, uint256 lf)
    {
        uint256 part = (amount * bps) / 10000;
        if (fixed_ > type(uint256).max - part) return (true, 0);
        lf = fixed_ + part;
        if (min_ != 0 && lf < min_) lf = min_;
        if (max_ != 0 && lf > max_) lf = max_;
    }

    /// The quote decoder over random words: the verifier never reverts; it decodes exactly what the model
    /// decodes, prices the zero-unit fee as the model does, and reports the overflow where the model overflows.
    function testFuzz_quoteDecoder(bytes32[15] memory w, uint256 valid, uint256 len, uint256 amount) public {
        for (uint256 k = 0; k < 15; k++) {
            if (((valid >> (2 * k)) & 3) != 0) w[k] = _coerce(k, w[k]);
        }
        if ((valid >> 40) % 2 == 0) {
            w[2] = 0;
            w[3] = 0;
            w[4] = 0;
        }
        bytes memory q = abi.encodePacked(w);
        if ((valid >> 48) % 8 == 0) {
            uint256 l = len % 700;
            bytes memory r = new bytes(l);
            for (uint256 i = 0; i < l && i < q.length; i++) {
                r[i] = q[i];
            }
            q = r;
        }
        (bool bad, uint256[15] memory words) = _quoteModel(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, q, (valid >> 56) % 4 == 0 ? amount : bound(amount, 1, 1e30));
        try verifier.check(t, q, "", "", new address[](0), new bytes32[](0)) returns (EravonVerifier.Checked memory c) {
            assertEq(c.quoteBad, bad, "the decoder's verdict is the model's");
            if (bad) return;
            for (uint256 k = 0; k < 15; k++) {
                assertEq(c.q[k], words[k], "the words decoded");
            }
            if (t.offer.amount == 0 || t.offer.amount > type(uint256).max / 10000) {
                assertEq(c.termsFault, 6, "an amount past the bound is refused before any fee");
                return;
            }
            if (words[2] == 0) {
                (bool ovf, uint256 lf) = _lfModel(t.offer.amount, words[6], words[7], words[8], words[9]);
                assertEq(c.overflow, ovf, "the overflow as the model's");
                assertFalse(c.stale);
                if (ovf) return;
                assertEq(c.lf, lf, "lf as terms/1 s6.1");
                assertEq(c.cap, words[10], "the cap in the token");
            }
        } catch {
            assertTrue(false, "the verifier's check reverted: every fault is an answer");
        }
    }

    /// Through init: a bad quote is BadQuote, a quote of another hash or token QuoteMismatch, in that order,
    /// after the terms guards; a good one passes to the consents (BadSellerSig with an empty slot from the buyer).
    function testFuzz_quoteThroughInit(bytes32[15] memory w, uint256 valid, uint256 which) public {
        for (uint256 k = 0; k < 15; k++) {
            if (((valid >> (2 * k)) & 3) != 0) w[k] = _coerce(k, w[k]);
        }
        w[2] = 0;
        w[3] = 0;
        w[4] = 0;
        w[6] = bytes32(uint256(w[6]) % 1e30); // the fee prices without overflow here: the order of faults is the subject
        bytes memory q = abi.encodePacked(w);
        (bool bad, uint256[15] memory words) = _quoteModel(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, q, 1 ether);
        uint256 m = which % 3;
        if (m == 1) t.offer.quoteHash = keccak256(abi.encode(q, "other"));
        if (m == 2 && !bad) t.offer.token = address(uint160(words[0]));
        bool tokenMatches = !bad && address(uint160(words[0])) == t.offer.token;
        Eravon inst = deployProxy(t);
        vm.prank(BUYER);
        if (bad) vm.expectRevert(Eravon.BadQuote.selector);
        else if (m == 1 || !tokenMatches) vm.expectRevert(Eravon.QuoteMismatch.selector);
        else vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.init(t, q, "", "", new address[](0), new bytes32[](0));
    }

    // ==================================================================
    // 2. the fee: in the settlement token, and in a reference unit
    // ==================================================================

    /// Through init and feeQuoted(): lf, the cap, the deposit pulled as P + lf, OverCap where the cap is below A.
    function testFuzz_feeInTheToken(uint256 amount, uint256 fixed_, uint16 bps, uint256 min_, uint256 max_, uint256 cap, uint16 prem)
        public
    {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.premiumBps = uint16(bound(prem, 0, 10000));
        q.feeFixed = bound(fixed_, 0, 1e30);
        q.feeBps = uint16(bound(bps, 0, 10000));
        q.feeMin = min_ % 4 == 0 ? 0 : bound(min_, 0, 1e31);
        q.feeMax = max_ % 4 == 0 ? 0 : bound(max_, 0, 1e31);
        q.amountCap = cap % 3 == 0 ? 0 : bound(cap, 1, 1e31);
        amount = bound(amount, 1, 1e30);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, amount);
        (, uint256 lf) = _lfModel(amount, q.feeFixed, q.feeBps, q.feeMin, q.feeMax);
        uint256 p = (amount * q.premiumBps) / 10000;
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), amount);
        fundDeposit(p + lf);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        if (q.amountCap != 0 && amount > q.amountCap) {
            vm.expectRevert(Eravon.OverCap.selector);
            inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
            return;
        }
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        (uint256 lfQ, uint256 capQ, uint256 pT, uint256 pU, uint64 at) = inst.feeQuoted();
        assertEq(lfQ, lf, "lf pinned as terms/1 s6.1");
        assertEq(capQ, q.amountCap);
        assertEq(pT, 0);
        assertEq(pU, 0);
        assertEq(at, 0);
        assertEq(depositHeld(inst), p + lf, "D = P + lf");
        assertEq(premiumHeld(inst), p);
        assertEq(tok.balanceOf(address(inst)), amount + p + lf);
    }

    struct FeedCase {
        uint256 decT; // the settlement token's decimals word
        uint256 decU; // the reference unit's
        uint256 fdT; // the token feed's decimals word
        uint256 fdU;
        uint256 rT; // round ids
        uint256 rU;
        int256 aT; // answers
        int256 aU;
        uint256 tT; // updatedAt
        uint256 tU;
        uint8 mT; // feed modes
        uint8 mU;
        bool zeroT; // a zero feed address
        bool zeroU;
        uint32 maxStale;
        uint256 fixed_;
        uint256 min_;
        uint256 max_;
        uint256 cap;
        uint16 bps;
        uint256 amount;
    }

    struct FeedModel {
        bool stale;
        bool overflow; // a converted amount or the fee past 2^256 - 1
        uint256 lf;
        uint256 cap;
        uint256 pT;
        uint256 pU;
        uint64 at;
    }

    /// One feed as standard s3 reads it: a zero address counts as 1 with no decimals and no clock; a feed that
    /// does not answer, answers short, answers a price not positive or at 2^128 or more, an updatedAt past 2^64,
    /// in the future or older than maxStale, or decimals that do not answer or pass 36, is stale. The round words
    /// are not read.
    function _feedRead(bool zero, uint256 fd, int256 a, uint256 t, uint8 m, uint32 maxStale)
        internal
        view
        returns (bool ok, uint256 price, uint256 dec, uint64 at)
    {
        if (zero) return (true, 1, 0, 0);
        if (m == 1 || m == 2) return (false, 0, 0, 0);
        if (a <= 0 || uint256(a) >= 2 ** 128) return (false, 0, 0, 0);
        uint256 nowT = vm.getBlockTimestamp();
        if (t > type(uint64).max || t > nowT || t + maxStale < nowT) return (false, 0, 0, 0);
        if (m == 3 || fd > 36) return (false, 0, 0, 0);
        return (true, uint256(a), fd, uint64(t));
    }

    // ---- exact integers in 64-bit limbs, twelve of them (768 bits), apart from the verifier's arithmetic ----

    uint256 internal constant LIMB = type(uint64).max;

    function _bn(uint256 x) internal pure returns (uint256[] memory r) {
        r = new uint256[](12);
        for (uint256 i = 0; i < 4; i++) {
            r[i] = (x >> (64 * i)) & LIMB;
        }
    }

    function _bnMul(uint256[] memory a, uint256[] memory b) internal pure returns (uint256[] memory r) {
        r = new uint256[](12);
        for (uint256 i = 0; i < 12; i++) {
            if (a[i] == 0) continue;
            uint256 carry;
            for (uint256 j = 0; i + j < 12; j++) {
                uint256 t = a[i] * b[j] + r[i + j] + carry;
                r[i + j] = t & LIMB;
                carry = t >> 64;
            }
        }
    }

    /// ceil(a / d) for 0 < d < 2^128.
    function _bnDivCeil(uint256[] memory a, uint256 d) internal pure returns (uint256[] memory q) {
        q = new uint256[](12);
        uint256 rem;
        for (uint256 i = 12; i > 0; i--) {
            uint256 cur = (rem << 64) | a[i - 1];
            q[i - 1] = cur / d;
            rem = cur % d;
        }
        if (rem != 0) {
            for (uint256 i = 0; i < 12; i++) {
                q[i] += 1;
                if (q[i] <= LIMB) break;
                q[i] = 0;
            }
        }
    }

    function _bnFits(uint256[] memory a) internal pure returns (bool fits, uint256 x) {
        for (uint256 i = 4; i < 12; i++) {
            if (a[i] != 0) return (false, 0);
        }
        for (uint256 i = 0; i < 4; i++) {
            x |= a[i] << (64 * i);
        }
        return (true, x);
    }

    /// x' = ceil(x * pU * 10^(dT + decT) / (pT * 10^(dU + decU))), exactly: the common power of ten cancels, the
    /// rest is integer arithmetic in limbs; past 2^256 - 1 is an overflow. Zero stays zero.
    function _convertModel(uint256 x, uint256 pT, uint256 a_, uint256 pU, uint256 b_) internal pure returns (bool ovf, uint256 r) {
        if (x == 0) return (false, 0);
        uint256[] memory n = _bnMul(_bn(x), _bn(pU));
        if (a_ >= b_) {
            for (uint256 k = 0; k < a_ - b_; k++) {
                n = _bnMul(n, _bn(10));
            }
            n = _bnDivCeil(n, pT);
        } else {
            // ceil(ceil(y / a) / b) = ceil(y / (a b)) for positive integers
            n = _bnDivCeil(n, pT);
            for (uint256 k = 0; k < b_ - a_; k++) {
                n = _bnDivCeil(n, 10);
            }
        }
        (bool fits, uint256 v) = _bnFits(n);
        return (!fits, v);
    }

    /// standard s3's reference-unit pricing: two feeds, any anomaly stale, the absolutes and the cap converted
    /// once, rounded up, past 2^256 - 1 an overflow; then terms/1 s6.1 over the converted absolutes.
    function _feedModel(FeedCase memory f) internal view returns (FeedModel memory m) {
        (bool okT, uint256 pT, uint256 dT, uint64 tT) = _feedRead(f.zeroT, f.fdT, f.aT, f.tT, f.mT, f.maxStale);
        (bool okU, uint256 pU, uint256 dU, uint64 tU) = _feedRead(f.zeroU, f.fdU, f.aU, f.tU, f.mU, f.maxStale);
        if (!okT || !okU || f.decT > 36 || f.decU > 36) {
            m.stale = true;
            return m;
        }
        uint256 a_ = dT + f.decT;
        uint256 b_ = dU + f.decU;
        // standard s3: a scaled price (pU or pT times the power of ten) past 2^256 - 1 is an overflow where anything
        // is converted, even where every exact quotient would fit
        if (f.fixed_ != 0 || f.min_ != 0 || f.max_ != 0 || f.cap != 0) {
            uint256[] memory sp = _bn(a_ >= b_ ? pU : pT);
            uint256 k_ = a_ >= b_ ? a_ - b_ : b_ - a_;
            for (uint256 i = 0; i < k_; i++) {
                sp = _bnMul(sp, _bn(10));
            }
            (bool spFits,) = _bnFits(sp);
            if (!spFits) {
                m.overflow = true;
                return m;
            }
        }
        bool o1;
        bool o2;
        bool o3;
        bool o4;
        uint256 fixed_;
        uint256 min_;
        uint256 max_;
        (o1, fixed_) = _convertModel(f.fixed_, pT, a_, pU, b_);
        (o2, min_) = _convertModel(f.min_, pT, a_, pU, b_);
        (o3, max_) = _convertModel(f.max_, pT, a_, pU, b_);
        (o4, m.cap) = _convertModel(f.cap, pT, a_, pU, b_);
        if (o1 || o2 || o3 || o4) {
            m.overflow = true;
            return m;
        }
        bool ovf;
        (ovf, m.lf) = _lfModel(f.amount, fixed_, f.bps, min_, max_);
        if (ovf) {
            m.overflow = true;
            return m;
        }
        m.pT = pT;
        m.pU = pU;
        uint64 at = tT;
        if (tU != 0 && (at == 0 || tU < at)) at = tU;
        if (at == 0) at = uint64(vm.getBlockTimestamp());
        m.at = at;
    }

    function _feedCase(uint256 s0, uint256 s1, uint256 s2, uint256 s3) internal view returns (FeedCase memory f) {
        uint256 nowT = vm.getBlockTimestamp();
        // most draws realistic, some at the edges the text names
        f.decT = s0 % 8 == 0 ? (s0 >> 8) % 45 : [uint256(6), 8, 18, 0][(s0 >> 8) % 4];
        f.decU = (s0 >> 16) % 8 == 0 ? (s0 >> 24) % 45 : [uint256(6), 8, 18, 2][(s0 >> 24) % 4];
        f.fdT = (s0 >> 32) % 8 == 0 ? (s0 >> 40) % 45 : 8;
        f.fdU = (s0 >> 48) % 8 == 0 ? (s0 >> 56) % 45 : 8;
        // round words past 80 bits: not read
        f.rT = (s0 >> 64) % 24 == 0 ? (s1 | (1 << 80)) : 1;
        f.rU = (s0 >> 72) % 24 == 0 ? (s1 | (1 << 81)) : 1;
        f.aT = (s1 >> 8) % 12 == 0 ? -int256((s1 >> 16) % 1000) : int256(1 + (s1 >> 16) % ((s1 >> 100) % 4 == 0 ? type(uint128).max : 1e12));
        if ((s1 >> 210) % 32 == 0) f.aT = int256(uint256(2 ** 128 + (s1 >> 220))); // a price at 2^128 or more
        f.aU = (s1 >> 120) % 12 == 0 ? int256(0) : int256(1 + (s1 >> 128) % ((s1 >> 200) % 4 == 0 ? type(uint128).max : 1e12));
        f.maxStale = uint32(1 + (s2 % 2 days));
        f.tT = (s2 >> 32) % 8 == 0 ? (s2 >> 40) % 2 ** 70 : nowT - ((s2 >> 40) % (uint256(f.maxStale) + f.maxStale / 8 + 1));
        f.tU = (s2 >> 112) % 8 == 0 ? nowT + (s2 >> 120) % 1000 : nowT - ((s2 >> 120) % (uint256(f.maxStale) + f.maxStale / 8 + 1));
        f.mT = (s2 >> 192) % 14 < 12 ? 0 : uint8((s2 >> 196) % 4);
        f.mU = (s2 >> 200) % 14 < 12 ? 0 : uint8((s2 >> 204) % 4);
        f.zeroT = (s3 % 6) == 0;
        f.zeroU = ((s3 >> 8) % 6) == 0;
        f.fixed_ = (s3 >> 16) % 4 == 0 ? 0 : (s3 >> 20) % ((s3 >> 90) % 8 == 0 ? type(uint256).max : 1e24);
        f.min_ = (s3 >> 24) % 3 == 0 ? 0 : (s3 >> 28) % 1e24;
        f.max_ = (s3 >> 32) % 3 == 0 ? 0 : (s3 >> 36) % 1e26;
        f.cap = (s3 >> 40) % 3 == 0 ? 0 : (s3 >> 44) % 1e30;
        f.bps = uint16((s3 >> 120) % 10001);
        f.amount = 1 + (s3 >> 136) % 1e27;
    }

    function _feedSetup(FeedCase memory f)
        internal
        returns (DecToken settle, DecToken unit, RawFeed feedT, RawFeed feedU, Q memory q)
    {
        settle = new DecToken(f.decT);
        unit = new DecToken(f.decU);
        feedT = new RawFeed();
        feedU = new RawFeed();
        feedT.set(f.fdT, f.rT, f.aT, f.tT, f.mT);
        feedU.set(f.fdU, f.rU, f.aU, f.tU, f.mU);
        q = defaultQuote(address(settle), JUDGE_PAYOUT, true);
        q.feeUnit = address(unit);
        q.feedToken = f.zeroT ? address(0) : address(feedT);
        q.feedUnit = f.zeroU ? address(0) : address(feedU);
        q.maxStale = f.maxStale;
        q.feeFixed = f.fixed_;
        q.feeBps = f.bps;
        q.feeMin = f.min_;
        q.feeMax = f.max_;
        q.amountCap = f.cap;
    }

    /// The verifier's pricing in a reference unit against the model: it never reverts; stale where the text
    /// says stale, an overflow where a converted amount or the fee passes 2^256 - 1, otherwise the conversion
    /// rounded up and rateAt the older nonzero clock.
    function testFuzz_feeInAReferenceUnit(uint256 s0, uint256 s1, uint256 s2, uint256 s3) public {
        FeedCase memory f = _feedCase(s0, s1, s2, s3);
        (DecToken settle,,,, Q memory q) = _feedSetup(f);
        FeedModel memory m = _feedModel(f);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, f.amount);
        t.offer.token = address(settle);
        try verifier.check(t, quote, "", "", new address[](0), new bytes32[](0)) returns (EravonVerifier.Checked memory c) {
            assertEq(c.stale, m.stale, "stale as the text");
            if (m.stale) return;
            assertEq(c.overflow, m.overflow, "the overflow as the model's");
            if (m.overflow) return;
            assertEq(c.lf, m.lf, "lf converted and bounded");
            assertEq(c.cap, m.cap, "the cap converted");
            assertEq(c.priceToken, m.pT);
            assertEq(c.priceUnit, m.pU);
            assertEq(c.rateAt, m.at, "rateAt the older clock");
        } catch {
            assertTrue(false, "the verifier's pricing reverted: every anomaly is an answer");
        }
    }

    /// Through init: a stale feed refuses with RateStale, an overflow (a converted amount, the fee, or the
    /// deposit P + lf past 2^256 - 1) with FeeOverflow, a converted cap below A with OverCap; otherwise
    /// feeQuoted() pins the model's five values and the deposit pulled is P + lf.
    function testFuzz_feeInAReferenceUnitThroughInit(uint256 s0, uint256 s1, uint256 s2, uint256 s3) public {
        FeedCase memory f = _feedCase(s0, s1, s2, s3);
        f.fixed_ = f.fixed_ % 1e24;
        (DecToken settle,,,, Q memory q) = _feedSetup(f);
        FeedModel memory m = _feedModel(f);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, f.amount);
        t.offer.token = address(settle);
        Eravon inst = deployProxy(t);
        settle.mint(address(inst), f.amount);
        uint256 p = (f.amount * q.premiumBps) / 10000;
        bool depositOverflows = !m.stale && !m.overflow && m.lf > type(uint256).max - p;
        uint256 d = m.stale || m.overflow || depositOverflows ? 0 : p + m.lf;
        // a deposit the instance's balance cannot add to the amount is the test token's limit, not the standard's
        vm.assume(d <= type(uint256).max - f.amount);
        settle.mint(SELLER, d);
        vm.prank(SELLER);
        settle.approve(address(logic), d);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        if (m.stale) {
            vm.expectRevert(Eravon.RateStale.selector);
            inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
            return;
        }
        if (m.overflow) {
            vm.expectRevert(Eravon.FeeOverflow.selector);
            inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
            return;
        }
        if (m.cap != 0 && f.amount > m.cap) {
            vm.expectRevert(Eravon.OverCap.selector);
            inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
            return;
        }
        if (depositOverflows) {
            vm.expectRevert(Eravon.FeeOverflow.selector);
            inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
            return;
        }
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        (uint256 lfQ, uint256 capQ, uint256 pT, uint256 pU, uint64 at) = inst.feeQuoted();
        assertEq(lfQ, m.lf);
        assertEq(capQ, m.cap);
        assertEq(pT, m.pT);
        assertEq(pU, m.pU);
        assertEq(at, m.at);
        assertEq(depositHeld(inst), p + m.lf, "D = P + lf");
    }

    /// standard s3: where every absolute amount is zero nothing is scaled, so a scaled price pU * 10^(a - b) past
    /// 2^256 - 1 is no fault and the fee is A * feeBps in the token; one nonzero absolute brings the scaling
    /// back, and with it FeeOverflow for the scaled price, though the converted amount would fit.
    function test_scaledPriceOverflowIsNoFaultWhereNothingIsConverted() public {
        DecToken settle = new DecToken(36);
        DecToken unit = new DecToken(0);
        RawFeed fT = new RawFeed();
        RawFeed fU = new RawFeed();
        fT.set(36, 1, int256(uint256(2 ** 127)), vm.getBlockTimestamp(), 0); // a = 36 + 36 = 72
        fU.set(0, 1, int256(uint256(2 ** 128 - 1)), vm.getBlockTimestamp(), 0); // b = 0: pU * 10^72 passes 2^256
        Q memory q = defaultQuote(address(settle), JUDGE_PAYOUT, true);
        q.feeUnit = address(unit);
        q.feedToken = address(fT);
        q.feedUnit = address(fU);
        q.maxStale = 1 days;
        q.feeFixed = 0;
        q.feeMin = 0;
        q.feeMax = 0;
        q.amountCap = 0;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t.offer.token = address(settle);
        FeedCase memory f;
        f.decT = 36;
        f.fdT = 36;
        f.aT = int256(uint256(2 ** 127));
        f.tT = vm.getBlockTimestamp();
        f.aU = int256(uint256(2 ** 128 - 1));
        f.tU = vm.getBlockTimestamp();
        f.maxStale = 1 days;
        f.bps = q.feeBps;
        f.amount = 1 ether;
        FeedModel memory m = _feedModel(f);
        assertFalse(m.stale || m.overflow, "the exact computation prices it");
        assertEq(m.lf, 1e17, "the fee is A x feeBps / 10000 in the token, no conversion in it");
        EravonVerifier.Checked memory c = verifier.check(t, quote, "", "", new address[](0), new bytes32[](0));
        assertFalse(c.overflow, "nothing to convert, nothing to overflow");
        assertEq(c.lf, 1e17);
        Eravon inst = deployProxy(t);
        settle.mint(address(inst), 1 ether);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        settle.mint(SELLER, 2 ether);
        vm.prank(SELLER);
        settle.approve(address(logic), 2 ether);
        vm.prank(SELLER);
        inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
        (uint256 lf,,,,) = inst.feeQuoted();
        assertEq(lf, 1e17, "the fee is A x feeBps / 10000 in the token");
        // one nonzero absolute amount brings the scaling back, and with it the named overflow
        q.feeFixed = 1;
        quote = encodeQuote(q);
        t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t.offer.token = address(settle);
        t.offer.nonce = 77;
        c = verifier.check(t, quote, "", "", new address[](0), new bytes32[](0));
        assertTrue(c.overflow, "the scaled price passes 2^256 - 1");
    }

    // ==================================================================
    // 3. the ladder's split at every rung
    // ==================================================================

    function _randomLadder(uint256 seed, uint256 n) internal pure returns (Eravon.Rung[] memory l) {
        l = new Eravon.Rung[](n);
        uint256[] memory b = new uint256[](n);
        for (uint256 i = 1; i + 1 < n; i++) {
            b[i] = uint256(keccak256(abi.encode(seed, i))) % 10001;
        }
        b[n - 1] = 10000;
        // ascending by insertion
        for (uint256 i = 1; i < n; i++) {
            for (uint256 k = i; k > 0 && b[k] < b[k - 1]; k--) {
                (b[k], b[k - 1]) = (b[k - 1], b[k]);
            }
        }
        for (uint256 i = 0; i < n; i++) {
            l[i] = Eravon.Rung(uint16(b[i]), uint16(10000 - b[i]), uint8(uint256(keccak256(abi.encode(seed, i, "fb"))) % 3));
        }
    }

    function _settleAt(Eravon inst, uint16 bps) internal {
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 d = digestOf(address(inst), keccak256(abi.encode(TH_SETTLE, uint256(bps), exp)));
        vm.prank(SELLER);
        inst.settle(bps, exp, sign(BUYER_PK, d), sign(SELLER_PK, d));
    }

    /// A case disputed under a random ladder, then at every rung in turn (each from the same state) the
    /// ruling: shares by the rung, P and the buyer's fee to the judge, the seller's LF by feeBearer, nothing
    /// left beside the judge's credit; and the other terminals from the same state: the lapse, a settlement
    /// at a random value within T3 (P to the judge) and past it (P back), the seller's refund within T3
    /// (refused in both forms, nothing moving) and past it (a plain refund).
    function testFuzz_ladderEveryRung(uint256 seed, uint256 amount, uint16 premium, uint256 feeSeed) public {
        uint256 n = 2 + seed % 15;
        uint256 pick = (seed >> 8) % 6;
        if (pick == 0) amount = 2 ** 128;
        else if (pick == 1) amount = 2 ** 128 - 1 - (amount % 1000);
        else if (pick == 2) amount = 2 ** 128 + (amount % 1000);
        else amount = bound(amount, 1, 1e30);
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.premiumBps = uint16(bound(premium, 0, 10000));
        q.feeBps = uint16(feeSeed % 10001);
        q.feeFixed = (feeSeed >> 16) % 1e24;
        q.feeMin = (feeSeed >> 100) % 3 == 0 ? 0 : (feeSeed >> 104) % 1e24;
        q.feeMax = (feeSeed >> 180) % 3 == 0 ? 0 : (feeSeed >> 184) % 1e30;
        q.lapseDefault = (seed >> 16) % 2 == 0 ? 0 : 2;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, amount);
        t.offer.ladder = _randomLadder(seed, n);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), amount);
        uint256 lf = lfOf(q, amount);
        uint256 p = premiumOf(q, amount);
        fundDeposit(p + lf);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        claim(inst, MANIFEST);
        raise(inst);
        bytes32 id = orderHash(t);
        for (uint256 r = 0; r < n; r++) {
            uint256 snapId = vm.snapshotState();
            vm.prank(JUDGE, JUDGE);
            inst.rule(id, 0, E, uint64(r), ENTRY, "");
            Eravon.Rung memory o = t.offer.ladder[r];
            uint256 buyerA = (amount * o.buyerBps) / 10000;
            uint256 bond = o.feeBearer == 1 ? lf : (o.feeBearer == 2 ? lf / 2 : 0);
            (, uint256 jc) = inst.accountOf(JUDGE, address(tok));
            assertEq(jc, p + lf, "P and the buyer's fee to the judge");
            assertEq(tok.balanceOf(address(inst)), jc, "no residual beside the judge's credit");
            assertEq(tok.balanceOf(BUYER_PAYOUT), buyerA + bond, "the buyer's share and bond");
            assertEq(tok.balanceOf(SELLER_PAYOUT), amount - buyerA, "the seller's share");
            assertEq(tok.balanceOf(SELLER), lf - bond, "the seller's LF less the bond");
            vm.revertToState(snapId);
        }
        // the lapse at the rung lapseDefault names: P and D back to the seller, the fee back to the buyer
        {
            uint256 snapId = vm.snapshotState();
            vm.warp(vm.getBlockTimestamp() + 14 days + 1);
            inst.advance(id, 0);
            uint256 r = q.lapseDefault == 0 ? 0 : n - 1;
            uint256 buyerA = (amount * t.offer.ladder[r].buyerBps) / 10000;
            assertEq(tok.balanceOf(address(inst)), 0, "a lapse leaves nothing");
            assertEq(tok.balanceOf(BUYER_PAYOUT), buyerA + lf, "the buyer's share and its fee back");
            assertEq(tok.balanceOf(SELLER_PAYOUT), amount - buyerA);
            assertEq(tok.balanceOf(SELLER), p + lf, "D back to the key");
            vm.revertToState(snapId);
        }
        uint16 bps = uint16((seed >> 32) % 10001);
        uint256 buyerS = (amount * bps) / 10000;
        // a settlement within T3: P to the judge whose ruling window still runs, the fee back to the buyer
        {
            uint256 snapId = vm.snapshotState();
            vm.warp(vm.getBlockTimestamp() + (seed >> 48) % (14 days + 1));
            _settleAt(inst, bps);
            (, uint256 jc) = inst.accountOf(JUDGE, address(tok));
            assertEq(jc, p, "P to the judge within its window");
            assertEq(tok.balanceOf(address(inst)), p, "nothing else left");
            assertEq(tok.balanceOf(BUYER_PAYOUT), buyerS + lf);
            assertEq(tok.balanceOf(SELLER_PAYOUT), amount - buyerS);
            assertEq(tok.balanceOf(SELLER), lf);
            vm.revertToState(snapId);
        }
        // a settlement past T3, nobody having advanced: P back to the seller
        {
            uint256 snapId = vm.snapshotState();
            vm.warp(vm.getBlockTimestamp() + 14 days + 1 + (seed >> 96) % 30 days);
            _settleAt(inst, bps);
            (, uint256 jc) = inst.accountOf(JUDGE, address(tok));
            assertEq(jc, 0, "nothing to a judge whose window ran out");
            assertEq(tok.balanceOf(address(inst)), 0);
            assertEq(tok.balanceOf(BUYER_PAYOUT), buyerS + lf);
            assertEq(tok.balanceOf(SELLER_PAYOUT), amount - buyerS);
            assertEq(tok.balanceOf(SELLER), p + lf);
            vm.revertToState(snapId);
        }
        // the seller's refund within T3 is refused in both forms, nothing moving (s8)
        {
            uint256 snapId = vm.snapshotState();
            vm.warp(vm.getBlockTimestamp() + (seed >> 144) % (14 days + 1));
            vm.prank(SELLER);
            vm.expectRevert(Eravon.WrongPhase.selector);
            inst.refund();
            vm.prank(SELLER);
            vm.expectRevert(Eravon.WrongPhase.selector);
            inst.refundBySig(sign(SELLER_PK, digestOf(address(inst), keccak256(abi.encode(TH_REFUND)))));
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Disputed), "nothing moved");
            vm.revertToState(snapId);
        }
        // the seller's refund past T3, nobody having advanced: everything back
        {
            vm.warp(vm.getBlockTimestamp() + 14 days + 1);
            vm.prank(SELLER);
            inst.refund();
            (, uint256 jc) = inst.accountOf(JUDGE, address(tok));
            assertEq(jc, 0);
            assertEq(tok.balanceOf(address(inst)), 0);
            assertEq(tok.balanceOf(BUYER_PAYOUT), amount + lf);
            assertEq(tok.balanceOf(SELLER), p + lf);
        }
    }

    // ==================================================================
    // the generators' spread
    // ==================================================================

    /// The generators above reach every class they are meant to: each guard as the first fault, the feeds
    /// stale, priced (a round word past 80 bits among them), and overflowing; a buyer's slot of any other length
    /// than 65 bytes refused by name. Deterministic; the counts are logged.
    function test_inputSpread() public {
        vm.pauseGasMetering();
        uint256[4] memory feed; // stale, priced, overflow, priced with a round word past 80 bits
        uint256[17] memory guards;
        uint256[2] memory blob; // BadBuyerSig, anything else
        for (uint256 k = 0; k < 3000; k++) {
            uint256 a = uint256(keccak256(abi.encode(k, 1)));
            uint256 b = uint256(keccak256(abi.encode(k, 2)));
            uint256 c = uint256(keccak256(abi.encode(k, 3)));
            uint256 d = uint256(keccak256(abi.encode(k, 4)));
            FeedCase memory fc = _feedCase(a, b, c, d);
            FeedModel memory m = _feedModel(fc);
            if (m.stale) feed[0]++;
            else if (m.overflow) feed[2]++;
            else feed[1]++;
            if (!m.stale && !m.overflow && ((!fc.zeroT && fc.rT >= 2 ** 80) || (!fc.zeroU && fc.rU >= 2 ** 80))) feed[3]++;
            Eravon.OrderTerms memory t = (a >> 250) % 2 == 0 ? _mutatedTerms(a, b, c) : _randomTerms(a, b, c, uint8(d));
            guards[_termsModel(t)]++;
            bytes memory x = new bytes(1 + (d >> 8) % 300);
            if (x.length == 65) x = new bytes(66);
            for (uint256 i = 0; i < x.length; i++) {
                x[i] = bytes1(uint8(uint256(keccak256(abi.encode(d, i)))));
            }
            try verifier.verifyBuyer(bytes32(a), x, address(uint160(b))) {
                assertTrue(false, "a blob verified");
            } catch (bytes memory err) {
                if (bytes4(err) == Eravon.BadBuyerSig.selector) blob[0]++;
                else blob[1]++;
            }
        }
        string[4] memory fn = ["feed: stale", "feed: priced", "feed: overflow", "feed: priced, a round word past 80 bits"];
        for (uint256 i = 0; i < 4; i++) {
            emit log_named_uint(fn[i], feed[i]);
            assertGt(feed[i], 0, fn[i]);
        }
        for (uint256 g = 0; g < 17; g++) {
            emit log_named_uint(string(abi.encodePacked("first terms fault ", vm.toString(g))), guards[g]);
            assertGt(guards[g], 0, "a guard never first");
        }
        emit log_named_uint("buyer slot of another length: BadBuyerSig", blob[0]);
        emit log_named_uint("buyer slot of another length: refused otherwise", blob[1]);
        assertGt(blob[0], 0);
        assertEq(blob[1], 0, "a slot refused without a name");
    }

    // ==================================================================
    // 5. secp256k1 recovery
    // ==================================================================

    function _recoverModel(bytes32 d, bytes memory sig) internal pure returns (address) {
        if (sig.length != 65) return address(0);
        bytes32 r;
        bytes32 s;
        assembly ("memory-safe") {
            r := mload(add(sig, 32))
            s := mload(add(sig, 64))
        }
        uint8 v = uint8(sig[64]);
        if (v != 27 && v != 28) return address(0);
        if (uint256(r) == 0 || uint256(r) >= SECP_N || uint256(s) == 0 || uint256(s) > SECP_N / 2) return address(0);
        return ecrecover(d, v, r, s);
    }

    function _claimDigest(bytes32 m) internal view returns (bytes32) {
        return digestOf(address(live), keccak256(abi.encode(TH_CLAIM, m)));
    }

    /// Random signatures: the verifier's recovery agrees with the model; verifySeller admits exactly the
    /// model's nonzero signer; and the logic's signed seller actions, which go through verifySeller, refuse
    /// every signature the model does not recover to the seller.
    function testFuzz_recoverRandom(bytes32 d, bytes32 r, bytes32 s, uint8 v, uint256 lenSel, bytes memory other) public {
        bytes memory sig = abi.encodePacked(r, s, uint8(lenSel % 4 == 0 ? v : 27 + (v % 2)));
        if (lenSel % 7 == 0) sig = other;
        if ((lenSel >> 8) % 5 == 0) {
            // r and s at the range edges
            uint256[6] memory edges = [uint256(0), 1, SECP_N - 1, SECP_N, SECP_N / 2, SECP_N / 2 + 1];
            sig = abi.encodePacked(bytes32(edges[(lenSel >> 16) % 6]), bytes32(edges[(lenSel >> 24) % 6]), uint8(27 + (v % 2)));
        }
        address want = _recoverModel(d, sig);
        assertEq(vh.recoverX(d, sig), want, "the verifier's recovery");
        if (want != address(0)) verifier.verifySeller(d, sig, want);
        address other_ = address(uint160(uint256(keccak256(abi.encode(d, lenSel)))));
        vm.expectRevert(Eravon.BadSellerSig.selector);
        verifier.verifySeller(d, sig, want == address(0) ? address(0) : other_);
        // the logic's paths: the claim's digest is the instance's own
        bytes32 m = keccak256(abi.encode(d, "manifest"));
        if (_recoverModel(_claimDigest(m), sig) != SELLER) {
            vm.prank(SELLER);
            vm.expectRevert(Eravon.BadSellerSig.selector);
            live.claimDeliveryBySig(m, sig);
        }
        if (_recoverModel(digestOf(address(live), keccak256(abi.encode(TH_REFUND))), sig) != SELLER) {
            vm.prank(SELLER);
            vm.expectRevert(Eravon.BadSellerSig.selector);
            live.refundBySig(sig);
        }
    }

    /// Real signatures: the low-s form recovers the signer, the high-s twin and any other v recover nothing,
    /// on the verifier and through the logic's claim: the seller's low-s signature claims, its twin does not,
    /// and another key's signature does not.
    function testFuzz_recoverSigned(uint256 pk, bytes32 d, uint8 v2) public {
        pk = bound(pk, 1, SECP_N - 1);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(pk, d);
        if (uint256(s) > SECP_N / 2) {
            s = bytes32(SECP_N - uint256(s));
            v = v == 27 ? 28 : 27;
        }
        address who = vm.addr(pk);
        assertEq(vh.recoverX(d, abi.encodePacked(r, s, v)), who, "low s recovers the signer");
        verifier.verifySeller(d, abi.encodePacked(r, s, v), who);
        uint8 twinV = v == 27 ? 28 : 27;
        bytes memory twin = abi.encodePacked(r, bytes32(SECP_N - uint256(s)), twinV);
        assertEq(vh.recoverX(d, twin), address(0), "the high-s twin is refused");
        vm.expectRevert(Eravon.BadSellerSig.selector);
        verifier.verifySeller(d, twin, who);
        if (v2 != 27 && v2 != 28) assertEq(vh.recoverX(d, abi.encodePacked(r, s, v2)), address(0), "another v is refused");
        if (v2 == 0 || v2 == 1) assertEq(vh.recoverX(d, abi.encodePacked(r, s, v - 27)), address(0), "v 0 or 1 is refused");
        // through the logic: the seller's claim of the live case
        bytes32 m = keccak256(abi.encode(d, "manifest"));
        bytes32 cd = _claimDigest(m);
        if (pk != SELLER_PK) {
            vm.prank(SELLER);
            vm.expectRevert(Eravon.BadSellerSig.selector);
            live.claimDeliveryBySig(m, sign(pk, cd));
        }
        (uint8 sv, bytes32 sr, bytes32 ss) = vm.sign(SELLER_PK, cd);
        if (uint256(ss) > SECP_N / 2) {
            ss = bytes32(SECP_N - uint256(ss));
            sv = sv == 27 ? 28 : 27;
        }
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        live.claimDeliveryBySig(m, abi.encodePacked(sr, bytes32(SECP_N - uint256(ss)), sv == 27 ? uint8(28) : uint8(27)));
        vm.prank(SELLER);
        live.claimDeliveryBySig(m, abi.encodePacked(sr, ss, sv));
        assertEq(uint8(phaseOf(live)), uint8(Eravon.Phase.Claimed), "the seller's low-s signature claims");
    }
}
