// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Eravon} from "./Eravon.sol";
import {IERC20, IPriceFeed, IEravonPanelBook} from "./interfaces.sol";

/// The standard's verifier (eravon.standard/1, Shape): the pure and view
/// halves of `init`, of `cancel`, of every terminal (s8's `split`), of the
/// guild track's read of the book (s7, `panel`) and of the parties'
/// signatures, kept out of the logic so the logic stays under EIP-170. Stateless, no owner, no
/// balance; deployed once per chain through the public CREATE2 deployer
/// under the salt keccak256("eravon/1/verify"), designated by code: the
/// logic derives this address and checks `extcodehash` before every call.
/// Everything here but the pricing, `split` and `panel` is terms/1: the
/// terms guards (s2, s4.1), the quote decoding (s3.2), hashStruct(Offer)
/// (s4.2), the buyer's two signature forms (s5.2, s5.3) and the two
/// parties' consents at `init` and `settle` (s5.4, s8); the pricing is the
/// standard's s3, `split` its s8, the terminal table's arithmetic, and
/// `panel` its s7.
contract EravonVerifier {
    /// EIP-7951 P-256 verification precompile.
    address internal constant P256VERIFY = 0x0000000000000000000000000000000000000100;
    /// The gas each read the pricing makes (a feed's latestRoundData, a decimals()) is handed: a read that
    /// runs out is a read that did not answer, so no feed can turn init into a bare revert.
    uint256 internal constant READ_GAS = 200_000;
    uint256 internal constant SECP_N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141;
    uint256 internal constant SECP_HALF_N = SECP_N >> 1;
    address internal constant DEPLOYER = 0x4e59b44847b379578588920cA78FbF26c0B4956C;

    // the same words the logic hashes with (terms/1 s4.2)
    bytes32 internal constant TH_OUTCOME = keccak256("Outcome(uint16 buyerBps,uint16 sellerBps,uint8 feeBearer)");
    bytes32 internal constant TH_OFFER = keccak256(
        "Offer(address sellerKey,address judge,address guild,address sellerPayout,address sellerFund,address token,uint256 amount,uint32 deliveryWindow,uint32 confirmWindow,uint32 openWindow,bool buyerMayOpen,Outcome[] ladder,bytes32 termsHash,bytes32 amanahHash,bytes32 quoteHash,uint256 nonce,uint256 expiry)Outcome(uint16 buyerBps,uint16 sellerBps,uint8 feeBearer)"
    );

    /// terms/1 s5.1: a 65-byte recoverable secp256k1 signature, low s, v 27 or 28.
    function _recover(bytes32 digest, bytes calldata sig) internal pure returns (address) {
        if (sig.length != 65) return address(0);
        bytes32 r = bytes32(sig[0:32]);
        bytes32 s = bytes32(sig[32:64]);
        uint8 v = uint8(sig[64]);
        if (v != 27 && v != 28) return address(0);
        if (uint256(r) == 0 || uint256(r) >= SECP_N || uint256(s) == 0 || uint256(s) > SECP_HALF_N) return address(0);
        return ecrecover(digest, v, r, s);
    }

    /// What `check` hands back: the first terms guard that fails (0 for none,
    /// standard s4 step 5); whether the quote decodes (terms/1 s3.2) and its
    /// sixteen words when it does; and the fee priced (standard s3): `lf` and
    /// `cap` in the settlement token, the two prices and the rate time where a
    /// reference unit is named, and `stale` where a feed answered late, not
    /// positive, or not at all. Nothing here reverts on the case's account:
    /// the logic reads every fault in `init`'s own order, after its expiry
    /// branch, which never calls this.
    struct Checked {
        uint8 termsFault;
        bool quoteBad;
        uint256[16] q;
        uint256 lf;
        uint256 cap;
        uint256 priceToken;
        uint256 priceUnit;
        uint64 rateAt;
        bool stale;
        bool overflow;
    }

    /// `init`'s calldata after the selector, forwarded as is.
    function check(
        Eravon.OrderTerms calldata t,
        bytes calldata quote,
        bytes calldata,
        bytes calldata,
        address[] calldata,
        bytes32[] calldata
    ) external view returns (Checked memory c) {
        c.termsFault = _termsFault(t);
        (c.quoteBad, c.q) = _decodeQuote(quote);
        if (!c.quoteBad) _price(c, t.offer.amount, t.offer.token);
    }

    /// standard s3: the fee priced once; the reference-unit conversion from the two feeds.
    function _price(Checked memory c, uint256 amount, address token) internal view {
        uint256[16] memory q = c.q;
        address unit = address(uint160(q[3]));
        uint256 fixed_ = q[7];
        uint256 min_ = q[9];
        uint256 max_ = q[10];
        uint256 cap = q[11];
        if (unit != address(0)) {
            uint32 maxStale = uint32(q[6]);
            (uint256 pT, uint8 dT, uint64 tT, bool okT) = _readFeed(address(uint160(q[4])), maxStale);
            (uint256 pU, uint8 dU, uint64 tU, bool okU) = _readFeed(address(uint160(q[5])), maxStale);
            (uint8 decT, bool okDT) = _decimals(token);
            (uint8 decU, bool okDU) = _decimals(unit);
            if (!okT || !okU || !okDT || !okDU) {
                c.stale = true;
                return;
            }
            // x' = ceil(x * pU * 10^(dT + decT) / (pT * 10^(dU + decU))), the common power of ten cancelled first,
            // the product in 512 bits; a result past 2^256 - 1 is an overflow the logic refuses by name; where every
            // amount to convert is zero nothing is scaled and nothing can overflow
            if (fixed_ != 0 || min_ != 0 || max_ != 0 || cap != 0) {
                uint256 a_ = uint256(dT) + decT;
                uint256 b_ = uint256(dU) + decU;
                uint256 num = pU;
                uint256 den = pT;
                bool ok_ = true;
                if (a_ >= b_) (num, ok_) = _mulPow10(pU, a_ - b_);
                else (den, ok_) = _mulPow10(pT, b_ - a_);
                if (ok_) (fixed_, ok_) = _convert(fixed_, num, den);
                if (ok_) (min_, ok_) = _convert(min_, num, den);
                if (ok_) (max_, ok_) = _convert(max_, num, den);
                if (ok_) (cap, ok_) = _convert(cap, num, den);
                if (!ok_) {
                    c.overflow = true;
                    return;
                }
            }
            c.priceToken = pT;
            c.priceUnit = pU;
            uint64 at = tT;
            if (tU != 0 && (at == 0 || tU < at)) at = tU;
            if (at == 0) at = uint64(block.timestamp);
            c.rateAt = at;
        }
        // terms/1 s6.1: raised to feeMin, then lowered to feeMax, in that order; the sum may overflow, the product
        // cannot (the amount is bounded, _termsFault guard 6)
        uint256 lf;
        unchecked {
            lf = fixed_ + (amount * q[8]) / 10000;
        }
        if (lf < fixed_) {
            c.overflow = true;
            return;
        }
        if (min_ != 0 && lf < min_) lf = min_;
        if (max_ != 0 && lf > max_) lf = max_;
        c.lf = lf;
        c.cap = cap;
    }

    /// A zero feed counts as 1 with no decimals and no clock; a feed that does
    /// not answer, answers late, or answers a non-positive price is not ok.
    /// Every anomaly of a feed is one answer, never a revert (standard s3).
    function _readFeed(address feed, uint32 maxStale) internal view returns (uint256 price, uint8 dec, uint64 at, bool ok) {
        if (feed == address(0)) return (1, 0, 0, true);
        bytes memory data = abi.encodeWithSelector(IPriceFeed.latestRoundData.selector);
        uint256[5] memory w;
        bool c1;
        uint256 size;
        uint256 g = READ_GAS;
        // the five words are copied to a fixed size, so a longer answer costs the caller nothing beyond the
        // allowance; the answer and updatedAt are read as raw words, so a word wider than its type is no revert
        assembly ("memory-safe") {
            c1 := staticcall(g, feed, add(data, 32), mload(data), w, 160)
            size := returndatasize()
        }
        if (!c1 || size < 160) return (0, 0, 0, false);
        int256 answer = int256(w[1]);
        uint256 updatedAt = w[3];
        if (answer <= 0 || uint256(answer) >= (1 << 128)) return (0, 0, 0, false);
        if (updatedAt > type(uint64).max || updatedAt > block.timestamp || updatedAt + maxStale < block.timestamp) return (0, 0, 0, false);
        (uint8 d, bool okD) = _decimals(feed);
        if (!okD) return (0, 0, 0, false);
        return (uint256(answer), d, uint64(updatedAt), true);
    }

    /// `decimals()` read the same way; a value past 36 is out of any token's or feed's range and reads as absent.
    function _decimals(address token) internal view returns (uint8 d, bool ok) {
        bytes memory data = abi.encodeWithSelector(IERC20.decimals.selector);
        uint256[1] memory w;
        bool c;
        uint256 size;
        uint256 g = READ_GAS;
        assembly ("memory-safe") {
            c := staticcall(g, token, add(data, 32), mload(data), w, 32)
            size := returndatasize()
        }
        if (!c || size < 32) return (0, false);
        uint256 x = w[0];
        if (x > 36) return (0, false);
        return (uint8(x), true);
    }

    /// Reference-unit amount to settlement-token amount, rounded up; zero stays zero (no bound).
    function _convert(uint256 v, uint256 num, uint256 den) internal pure returns (uint256, bool) {
        if (v == 0) return (0, true);
        return _mulDivUp(v, num, den);
    }

    /// x * 10^k with its overflow reported.
    function _mulPow10(uint256 x, uint256 k) internal pure returns (uint256, bool) {
        for (uint256 i = 0; i < k; i++) {
            if (x > type(uint256).max / 10) return (0, false);
            x *= 10;
        }
        return (x, true);
    }

    /// ceil(x * y / d) with the product in 512 bits; (0, false) where the result passes 2^256 - 1. d is nonzero.
    function _mulDivUp(uint256 x, uint256 y, uint256 d) internal pure returns (uint256 z, bool ok) {
        uint256 p0;
        uint256 p1;
        assembly {
            let mm := mulmod(x, y, not(0))
            p0 := mul(x, y)
            p1 := sub(sub(mm, p0), lt(mm, p0))
        }
        uint256 r = mulmod(x, y, d);
        if (p1 == 0) {
            z = p0 / d;
            if (r != 0) {
                if (z == type(uint256).max) return (0, false);
                z++;
            }
            return (z, true);
        }
        if (p1 >= d) return (0, false);
        assembly {
            p1 := sub(p1, gt(r, p0))
            p0 := sub(p0, r)
            let twos := and(d, sub(0, d))
            d := div(d, twos)
            p0 := div(p0, twos)
            twos := add(div(sub(0, twos), twos), 1)
            p0 := or(p0, mul(p1, twos))
            let inv := xor(mul(3, d), 2)
            inv := mul(inv, sub(2, mul(d, inv)))
            inv := mul(inv, sub(2, mul(d, inv)))
            inv := mul(inv, sub(2, mul(d, inv)))
            inv := mul(inv, sub(2, mul(d, inv)))
            inv := mul(inv, sub(2, mul(d, inv)))
            inv := mul(inv, sub(2, mul(d, inv)))
            z := mul(p0, inv)
        }
        if (r != 0) {
            if (z == type(uint256).max) return (0, false);
            z++;
        }
        return (z, true);
    }

    /// standard s9, terms/1 s5.4: the two parties' consents to one digest each,
    /// as `init` and `settle` admit them: a signature over the party's digest,
    /// or, on an empty slot, `sender` being the party or the logic's registry
    /// holding the digest under the party. Reverts with the party's fault. A
    /// zero key consents to nothing in either form.
    function consents(
        address logic,
        address sender,
        address sellerKey,
        bytes32 sellerDigest,
        bytes calldata sigSeller,
        address buyerKey,
        bytes32 buyerDigest,
        bytes calldata sigBuyer
    ) external view {
        if (sellerKey == address(0)) revert Eravon.BadSellerSig();
        if (sigSeller.length == 0) {
            if (sender != sellerKey && !Eravon(logic).consented(sellerKey, sellerDigest)) revert Eravon.BadSellerSig();
        } else if (_recover(sellerDigest, sigSeller) != sellerKey) {
            revert Eravon.BadSellerSig();
        }
        if (buyerKey == address(0)) revert Eravon.BadBuyerSig();
        if (sigBuyer.length == 0) {
            if (sender != buyerKey && !Eravon(logic).consented(buyerKey, buyerDigest)) revert Eravon.BadBuyerSig();
        } else {
            _buyer(buyerDigest, sigBuyer, buyerKey);
        }
    }

    /// terms/1 s5.2 and s5.3: the buyer's secp256k1 signature or WebAuthn
    /// assertion; reverts with the fault, returns nothing on success. An empty
    /// slot is no signature here: the signed forms of the five party actions
    /// admit none (terms/1 s5.4), `Cancel` among them.
    function verifyBuyer(bytes32 digest, bytes calldata sig, address key) external view {
        _buyer(digest, sig, key);
    }

    /// terms/1 s5.1: the seller's 65-byte secp256k1 signature over the digest, by the seller's key.
    function verifySeller(bytes32 digest, bytes calldata sig, address key) external pure {
        address who = _recover(digest, sig);
        if (who == address(0) || who != key) revert Eravon.BadSellerSig();
    }

    /// Standard s7, the guild track's read of the book: the book designated by code at
    /// keccak256("eravon/1/panel" || guild) under the book's initcode hash must carry the book's code; its
    /// latest panel for the standard's codeHash must be the one handed in, unexpired, every word the offer's
    /// amanahHash; the pool is the members that are neither party's key, and it must not be empty. The logic
    /// hands in the three hashes it carries. Returns the index, the panel hash and the pool's size; reverts
    /// with the logic's own errors otherwise.
    function panel(
        address guild,
        bytes32[3] calldata hashes, // proxyCodeHash, panelInitcodeHash, panelCodeHash
        address[] calldata members,
        bytes32[] calldata words,
        bytes32 word,
        address sellerKey,
        address buyerKey
    ) external view returns (uint256 index, bytes32 ph, uint256 pool) {
        address book = address(
            uint160(
                uint256(
                    keccak256(
                        abi.encodePacked(hex"ff", DEPLOYER, keccak256(abi.encodePacked("eravon/1/panel", guild)), hashes[1])
                    )
                )
            )
        );
        if (book.codehash != hashes[2]) revert Eravon.PanelBookMissing();
        uint64 expires;
        (index, ph,, expires) = IEravonPanelBook(book).latest(guild, hashes[0]);
        if (keccak256(abi.encode(members, words)) != ph) revert Eravon.PanelMismatch();
        if (expires < block.timestamp) revert Eravon.PanelExpired();
        for (uint256 i = 0; i < words.length; i++) {
            if (words[i] != word) revert Eravon.PanelWord();
        }
        for (uint256 i = 0; i < members.length; i++) {
            address m = members[i];
            if (m != address(0) && m != sellerKey && m != buyerKey) pool++;
        }
        if (pool == 0) revert Eravon.NoMembers();
    }

    function _buyer(bytes32 digest, bytes calldata sig, address key) internal view {
        if (sig.length == 0 || key == address(0)) revert Eravon.BadBuyerSig();
        if (sig.length == 65) {
            if (_recover(digest, sig) != key) revert Eravon.BadBuyerSig();
            return;
        }
        if (!_wellFormed(sig)) revert Eravon.BadPasskey(0);
        (bytes memory authData, string memory clientData, uint256 r, uint256 s, uint256 x, uint256 y) =
            abi.decode(sig, (bytes, string, uint256, uint256, uint256, uint256));
        bytes memory cd = bytes(clientData);
        // terms/1 s5.3's form test: every byte of clientDataJSON in U+0020 through U+007E
        for (uint256 i = 0; i < cd.length; i++) {
            uint8 ch = uint8(cd[i]);
            if (ch < 0x20 || ch > 0x7e) revert Eravon.BadPasskey(0);
        }
        if (address(uint160(uint256(keccak256(abi.encodePacked(x, y))))) != key) revert Eravon.BadPasskey(1);
        if (authData.length < 37 || (uint8(authData[32]) & 1) == 0) revert Eravon.BadPasskey(2);
        if (!_contains(cd, bytes('"type":"webauthn.get"'))) revert Eravon.BadPasskey(3);
        if (!_contains(cd, abi.encodePacked('"challenge":"', _b64url(digest), '"'))) revert Eravon.BadPasskey(4);
        bytes32 h = sha256(abi.encodePacked(authData, sha256(cd)));
        (bool ok, bytes memory out) = P256VERIFY.staticcall(abi.encodePacked(h, r, s, x, y));
        if (!ok || out.length != 32 || abi.decode(out, (uint256)) != 1) revert Eravon.BadPasskey(5);
    }

    /// The ABI packing of the six passkey members decodes: two heads inside the bytes, two lengths inside what
    /// follows them; a blob outside this shape is refused by name and never by the decoder.
    function _wellFormed(bytes calldata sig) internal pure returns (bool) {
        if (sig.length < 192) return false;
        uint256 oa = uint256(bytes32(sig[0:32]));
        uint256 os = uint256(bytes32(sig[32:64]));
        if (oa > sig.length - 32 || os > sig.length - 32) return false;
        uint256 la = uint256(bytes32(sig[oa:oa + 32]));
        uint256 ls = uint256(bytes32(sig[os:os + 32]));
        if (la > sig.length - oa - 32 || ls > sig.length - os - 32) return false;
        return true;
    }

    /// Standard s8, the terminal split, pure: the buyer's share of A plus the buyer's fee where it returns to it
    /// plus the part of the seller's LF sent to it; the seller's share of A; the judge's P and the buyer's fee where
    /// the flags send them there; the deposit's remainder back to sellerFund; and the part of the seller's LF sent
    /// to the buyer (bondTo 0 none, 1 all, 2 half rounded down). The strays are the instance's to add.
    function split(
        uint256 a,
        uint16 buyerBps,
        uint256 premium,
        uint256 deposit,
        uint256 lfHeld,
        bool pToJudge,
        bool lfToJudge,
        uint8 bondTo
    ) external pure returns (uint256 buyerPaid, uint256 sellerA, uint256 judgePaid, uint256 toSellerFund, uint256 bondToBuyer) {
        uint256 buyerA = (a * buyerBps) / 10000;
        sellerA = a - buyerA;
        buyerPaid = buyerA;
        if (pToJudge) judgePaid += premium;
        else toSellerFund += premium;
        if (lfToJudge) judgePaid += lfHeld;
        else buyerPaid += lfHeld;
        uint256 sellerLf = deposit - premium;
        if (bondTo == 1) bondToBuyer = sellerLf;
        else if (bondTo == 2) bondToBuyer = sellerLf / 2;
        buyerPaid += bondToBuyer;
        toSellerFund += sellerLf - bondToBuyer;
    }

    /// terms/1 s4.2: hashStruct(Offer).
    function hashOffer(Eravon.Offer calldata o) public pure returns (bytes32) {
        bytes32[] memory rungs = new bytes32[](o.ladder.length);
        for (uint256 i = 0; i < rungs.length; i++) {
            Eravon.Outcome calldata r = o.ladder[i];
            rungs[i] = keccak256(abi.encode(TH_OUTCOME, uint256(r.buyerBps), uint256(r.sellerBps), uint256(r.feeBearer)));
        }
        bytes memory head = abi.encode(
            TH_OFFER,
            o.sellerKey,
            o.judge,
            o.guild,
            o.sellerPayout,
            o.sellerFund,
            o.token,
            o.amount,
            uint256(o.deliveryWindow),
            uint256(o.confirmWindow),
            uint256(o.openWindow)
        );
        bytes memory tail = abi.encode(
            o.buyerMayOpen ? uint256(1) : uint256(0),
            keccak256(abi.encodePacked(rungs)),
            o.termsHash,
            o.amanahHash,
            o.quoteHash,
            o.nonce,
            o.expiry
        );
        return keccak256(abi.encodePacked(head, tail));
    }

    /// The guards of standard s4 step 5 in the logic's order; the first that fails names itself, 0 for none.
    function _termsFault(Eravon.OrderTerms calldata t) internal pure returns (uint8) {
        Eravon.Offer calldata o = t.offer;
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
            Eravon.Outcome calldata r = o.ladder[i];
            if (uint256(r.buyerBps) + uint256(r.sellerBps) != 10000) return 8;
            if (r.feeBearer > 2) return 9;
        }
        // terms/1 s2.5: the first rung, the last rung, then the order, as that law orders them
        if (o.ladder[0].buyerBps != 0) return 11;
        if (o.ladder[n - 1].buyerBps != 10000) return 12;
        for (uint256 i = 1; i < n; i++) {
            if (o.ladder[i].buyerBps < o.ladder[i - 1].buyerBps) return 10;
        }
        if (o.termsHash == bytes32(0)) return 13;
        if (o.amanahHash == bytes32(0)) return 14;
        if (o.quoteHash == bytes32(0)) return 15;
        // the three ints the documents spell (terms/1 s2.3, s4.4): at most 2^53 - 1
        if (o.nonce > 9007199254740991 || o.expiry > 9007199254740991 || t.openBy > 9007199254740991) return 16;
        return 0;
    }

    /// terms/1 s3.2: the quote's decoding, total, the same tests the law names,
    /// each word read as a big-endian integer; the sixteen words in s3.1's order.
    function _decodeQuote(bytes calldata quote) internal pure returns (bool bad, uint256[16] memory w) {
        if (quote.length != 512) return (true, w);
        for (uint256 i = 0; i < 16; i++) {
            w[i] = uint256(bytes32(quote[i * 32:i * 32 + 32]));
        }
        // address words: the first 12 bytes zero; token nonzero
        if (w[0] == 0 || w[0] >> 160 != 0 || w[1] >> 160 != 0 || w[3] >> 160 != 0 || w[4] >> 160 != 0 || w[5] >> 160 != 0) return (true, w);
        if (w[2] > 10000 || w[8] > 10000) return (true, w);
        if (w[6] > type(uint32).max || w[12] > type(uint32).max || w[13] > type(uint32).max) return (true, w);
        // lapseDefault is 0 or 2 (terms/1 s3.2, s3.3); autoAccept 0 or 1
        if ((w[14] != 0 && w[14] != 2) || w[15] > 1) return (true, w);
        if (w[3] == 0 && (w[4] != 0 || w[5] != 0)) return (true, w);
    }

    function _contains(bytes memory hay, bytes memory needle) internal pure returns (bool) {
        if (needle.length > hay.length) return false;
        uint256 last = hay.length - needle.length;
        for (uint256 i = 0; i <= last; i++) {
            bool same = true;
            for (uint256 j = 0; j < needle.length; j++) {
                if (hay[i + j] != needle[j]) {
                    same = false;
                    break;
                }
            }
            if (same) return true;
        }
        return false;
    }

    /// base64url without padding of 32 bytes: 43 characters.
    function _b64url(bytes32 d) internal pure returns (bytes memory out) {
        bytes memory alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        out = new bytes(43);
        uint256 o = 0;
        for (uint256 i = 0; i < 30; i += 3) {
            uint256 n = (uint256(uint8(d[i])) << 16) | (uint256(uint8(d[i + 1])) << 8) | uint256(uint8(d[i + 2]));
            out[o++] = alphabet[(n >> 18) & 63];
            out[o++] = alphabet[(n >> 12) & 63];
            out[o++] = alphabet[(n >> 6) & 63];
            out[o++] = alphabet[n & 63];
        }
        uint256 t = (uint256(uint8(d[30])) << 16) | (uint256(uint8(d[31])) << 8);
        out[o++] = alphabet[(t >> 18) & 63];
        out[o++] = alphabet[(t >> 12) & 63];
        out[o++] = alphabet[(t >> 6) & 63];
    }
}
