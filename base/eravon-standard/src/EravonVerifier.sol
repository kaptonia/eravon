// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Eravon} from "./Eravon.sol";
import {IGaleedVenue, IGaleedSuccessionBook, IGaleedRouter, RouteView} from "./IGaleedVenue.sol";
import {IERC20, IPriceFeed, IEravonPanelBook} from "./interfaces.sol";

/// The standard's verifier (eravon.standard/1, Shape): the pure and view
/// halves of `init`, of `cancel`, of every terminal and of the two outcome
/// views (s8's `split`), of the guild track's read of the book and of the
/// draw where the panel names no router (s7, `panel`, `draw`) and of the
/// parties' signatures, kept out of the logic so the logic stays under
/// EIP-170. Stateless, no owner, no balance; deployed once per chain through
/// the public CREATE2 deployer under the salt keccak256("eravon/1/verify"),
/// designated by code: the logic derives this address and checks
/// `extcodehash` before every call. Everything here but the pricing, `split`,
/// `panel` and `draw` is terms/1: the terms guards (s2, s4.1), the quote
/// decoding (s3.2), hashStruct(Offer) (s4.2), both parties' signatures (s5)
/// and the two consents at `init` and `settle` (s5.4, s8), each against the
/// party's current key on the succession book (standard s9).
contract EravonVerifier {
    /// The family's succession book (standard Shape, s9).
    address internal constant SUCCESSION_BOOK = 0x3dB7c505dd25361F1A66f6c5Bd96b48df27f668c;
    /// The gas each read the pricing makes (a feed's latestRoundData, a decimals()) is handed: a read that
    /// runs out is a read that did not answer, so no feed can turn init into a bare revert.
    uint256 internal constant READ_GAS = 200_000;
    uint256 internal constant SECP_N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141;
    uint256 internal constant SECP_HALF_N = SECP_N >> 1;
    address internal constant DEPLOYER = 0x4e59b44847b379578588920cA78FbF26c0B4956C;
    /// The proxy initcode's prefix (standard s1), the same bytes the logic pins: an instance's initcode is this prefix
    /// and its runtime, so the hash is computed from the account's own code.
    bytes10 internal constant INIT_PREFIX = 0x3d602d80600a3d3981f3;

    // the same words the logic hashes with (terms/1 s4.2)
    bytes32 internal constant TH_OUTCOME = keccak256("Outcome(uint16 buyerBps,uint16 sellerBps,uint8 feeBearer)");
    bytes32 internal constant TH_ORDER = keccak256(
        "OrderTerms(Offer offer,address buyerKey,address buyerPayout,uint64 openBy,address prev)Offer(address sellerKey,address judge,address guild,address sellerPayout,address sellerFund,address token,uint256 amount,uint32 deliveryWindow,uint32 confirmWindow,uint32 openWindow,bool buyerMayOpen,Outcome[] ladder,bytes32 termsHash,bytes32 amanahHash,bytes32 quoteHash,uint256 nonce,uint256 expiry)Outcome(uint16 buyerBps,uint16 sellerBps,uint8 feeBearer)"
    );
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
    /// fifteen words when it does; and the fee priced (standard s3): `lf` and
    /// `cap` in the settlement token, the two prices and the rate time where a
    /// reference unit is named, and `stale` where a feed answered late, not
    /// positive, or not at all; and whether the deposit address is unbound
    /// (s7a), read on the calling instance's logic. Nothing here reverts on the case's account:
    /// the logic reads every fault in `init`'s own order, after its expiry
    /// branch, which never calls this.
    struct Checked {
        uint8 termsFault;
        bool quoteBad;
        uint256[15] q;
        uint256 lf;
        uint256 cap;
        uint256 priceToken;
        uint256 priceUnit;
        uint64 rateAt;
        bool stale;
        bool overflow;
        bool fundUnbound;
        bytes4 stageFault;
        address seriesJudge;
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
        // standard s4 step 5, s7a: the deposit address is the seller's key or bound to it on the logic
        address fund = t.offer.sellerFund;
        c.fundUnbound = fund != t.offer.sellerKey && !Eravon(Eravon(msg.sender).LOGIC()).funds(t.offer.sellerKey, fund);
        // standard s4a: the stage before, where the order names one
        if (t.prev != address(0)) {
            (bool ok, address sk, address bk, bool ended, uint16 bps, address j) =
                stageOf(t.prev, Eravon(msg.sender).PROXY_CODEHASH());
            if (!ok || sk != t.offer.sellerKey || bk != t.buyerKey) c.stageFault = Eravon.BadPrev.selector;
            else if (!ended || bps != 0) c.stageFault = Eravon.StageOpen.selector;
            else c.seriesJudge = j;
        }
    }

    /// standard s3: the fee priced once; the reference-unit conversion from the two feeds.
    function _price(Checked memory c, uint256 amount, address token) internal view {
        uint256[15] memory q = c.q;
        address unit = address(uint160(q[2]));
        uint256 fixed_ = q[6];
        uint256 min_ = q[8];
        uint256 max_ = q[9];
        uint256 cap = q[10];
        if (unit != address(0)) {
            uint32 maxStale = uint32(q[5]);
            (uint256 pT, uint8 dT, uint64 tT, bool okT) = _readFeed(address(uint160(q[3])), maxStale);
            (uint256 pU, uint8 dU, uint64 tU, bool okU) = _readFeed(address(uint160(q[4])), maxStale);
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
            lf = fixed_ + (amount * q[7]) / 10000;
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

    /// The key that acts for `a` now: the succession book's answer (standard s9, venue/1 s1).
    function _cur(address a) internal view returns (address) {
        return IGaleedSuccessionBook(SUCCESSION_BOOK).current(a);
    }

    /// standard s9, terms/1 s5.4: the two parties' consents to one digest each,
    /// as `init` and `settle` admit them: a signature over the party's digest
    /// by the party's current key, or, on an empty slot, `sender` being that
    /// current key or the logic's registry holding the digest under the party.
    /// Reverts with the party's fault. A zero key consents to nothing in
    /// either form.
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
            if (sender != _cur(sellerKey) && !Eravon(logic).consented(sellerKey, sellerDigest)) revert Eravon.BadSellerSig();
        } else if (_recover(sellerDigest, sigSeller) != _cur(sellerKey)) {
            revert Eravon.BadSellerSig();
        }
        if (buyerKey == address(0)) revert Eravon.BadBuyerSig();
        if (sigBuyer.length == 0) {
            if (sender != _cur(buyerKey) && !Eravon(logic).consented(buyerKey, buyerDigest)) revert Eravon.BadBuyerSig();
        } else {
            verifyBuyer(buyerDigest, sigBuyer, buyerKey);
        }
    }

    /// terms/1 s5.2: the buyer's 65-byte signature over the digest by the
    /// buyer's current key; an empty slot is no signature here (the signed
    /// forms admit none, terms/1 s5.4), nor is any other form (s5.3).
    function verifyBuyer(bytes32 digest, bytes calldata sig, address key) public view {
        address who = _recover(digest, sig);
        if (who == address(0) || key == address(0) || who != _cur(key)) revert Eravon.BadBuyerSig();
    }

    /// terms/1 s5.1: the seller's 65-byte signature over the digest by the seller's current key.
    function verifySeller(bytes32 digest, bytes calldata sig, address key) external view {
        address who = _recover(digest, sig);
        if (who == address(0) || key == address(0) || who != _cur(key)) revert Eravon.BadSellerSig();
    }

    /// Standard s7, the guild track's read of the book at `init`: the guild's
    /// root on the succession book; the chain's panel book, designated by code
    /// at keccak256("eravon/1/panel") under the book's initcode hash, carrying
    /// the book's code; the latest record under (root, proxyCodeHash) recorded
    /// in a block before this one and unexpired now, walked back from the
    /// newest; the panel handed in hashing to it, every word the offer's
    /// amanahHash; and, where the record names no router, a pool of distinct
    /// members that are neither party's key nor zero. The logic hands in the
    /// three hashes it carries. Returns what the instance freezes.
    function panel(
        address guild,
        bytes32[3] calldata hashes, // proxyCodeHash, panelInitcodeHash, panelCodeHash
        address[] calldata members,
        bytes32[] calldata words,
        bytes32 word,
        address sellerKey,
        address buyerKey
    ) external view returns (address root, uint256 index, bytes32 ph, address router, bytes32 routerHash, uint256 pool) {
        root = IGaleedSuccessionBook(SUCCESSION_BOOK).root(guild);
        address book = address(
            uint160(uint256(keccak256(abi.encodePacked(hex"ff", DEPLOYER, keccak256("eravon/1/panel"), hashes[1]))))
        );
        if (book.codehash != hashes[2]) revert Eravon.PanelBookMissing();
        uint256 n = IEravonPanelBook(book).count(root, hashes[0]);
        bool found;
        while (n != 0) {
            n--;
            (bytes32 h,, uint64 expires, uint64 recorded, address r, bytes32 rh) = IEravonPanelBook(book).panelAt(root, hashes[0], n);
            if (recorded < block.number && expires >= block.timestamp) {
                (found, index, ph, router, routerHash) = (true, n, h, r, rh);
                break;
            }
        }
        if (!found) revert Eravon.NoPanel();
        if (keccak256(abi.encode(members, words)) != ph) revert Eravon.PanelMismatch();
        for (uint256 i = 0; i < words.length; i++) {
            if (words[i] != word) revert Eravon.PanelWord();
        }
        for (uint256 i = 0; i < members.length; i++) {
            if (_inPool(members, i, sellerKey, buyerKey)) pool++;
        }
        if (router == address(0) && pool == 0) revert Eravon.NoMembers();
    }

    /// Member `i` counts in the pool: nonzero, neither party's key, and its first place in the panel.
    function _inPool(address[] calldata members, uint256 i, address sellerKey, address buyerKey) internal pure returns (bool) {
        address m = members[i];
        if (m == address(0) || m == sellerKey || m == buyerKey) return false;
        for (uint256 j = 0; j < i; j++) {
            if (members[j] == m) return false;
        }
        return true;
    }

    /// Standard s7, a draw: the panel handed in must hash to the frozen one (PanelMismatch) and the block must
    /// follow the arming (DrawPending). With the seed `w[6]` zero (the chain serves the armed block's hash no
    /// more) it answers no member and no calldata. Where the frozen record names no router (`w[9]` zero), the
    /// member is the one at keccak256(caseId ‖ seed ‖ be64(seat)) mod n over the pool's members not seated
    /// before on the case, each once, in the order of its first place in the panel, zero where none is left.
    /// Where it names one, the answer is the calldata of `route(RouteView)` (venue/1 s4): the caseId, the new
    /// ordinal, the seat's termsHash, the token, the amount, the number of rungs, `parties` as seatOf reports
    /// them, the frozen members, every judge seated or offered on the case in the order of the seats'
    /// ordinals, the seed, the block's timestamp, the chain id and the calling instance.
    /// Where no router is named and the order's `prev` is an instance of this standard whose `stage()` answers a
    /// nonzero series judge (its fifth word, s4a), in the pool and on no seat of this case, that judge is seated
    /// (s7); the pick otherwise.
    /// `w`: caseId, seat, termsHash, token, amount, rungs, seed, panelHash, the block follows the arming, router,
    /// the order's prev.
    function draw(
        uint256[11] calldata w,
        address[5] calldata parties,
        address[] calldata members,
        bytes32[] calldata words,
        address[] calldata seated
    ) external view returns (address m, bytes memory data) {
        if (keccak256(abi.encode(members, words)) != bytes32(w[7])) revert Eravon.PanelMismatch();
        if (w[8] == 0) revert Eravon.DrawPending();
        if (w[6] == 0) return (address(0), data);
        if (w[9] != 0) {
            address[] memory ps = new address[](5);
            for (uint256 i = 0; i < 5; i++) {
                ps[i] = parties[i];
            }
            data = abi.encodeCall(
                IGaleedRouter.route,
                (
                    RouteView({
                        caseId: bytes32(w[0]),
                        seat: uint64(w[1]),
                        termsHash: bytes32(w[2]),
                        token: address(uint160(w[3])),
                        amount: w[4],
                        rungs: uint64(w[5]),
                        parties: ps,
                        members: members,
                        seated: seated,
                        seed: bytes32(w[6]),
                        blockTime: uint64(block.timestamp),
                        chainId: block.chainid,
                        venue: msg.sender
                    })
                )
            );
            return (address(0), data);
        }
        if (w[10] != 0) {
            (bool ok,,,,, address pj) = stageOf(address(uint160(w[10])), Eravon(msg.sender).PROXY_CODEHASH());
            if (ok && pj != address(0)) {
                for (uint256 i = 0; i < members.length; i++) {
                    if (members[i] == pj && _drawable(members, i, seated, parties[1], parties[0])) return (pj, data);
                }
            }
        }
        uint256 n = 0;
        for (uint256 i = 0; i < members.length; i++) {
            if (_drawable(members, i, seated, parties[1], parties[0])) n++;
        }
        if (n == 0) return (address(0), data);
        uint256 pick = uint256(keccak256(abi.encodePacked(bytes32(w[0]), bytes32(w[6]), uint64(w[1])))) % n;
        for (uint256 i = 0; i < members.length; i++) {
            if (!_drawable(members, i, seated, parties[1], parties[0])) continue;
            if (pick == 0) return (members[i], data);
            pick--;
        }
    }

    function _drawable(address[] calldata members, uint256 i, address[] calldata seated, address sellerKey, address buyerKey)
        internal
        pure
        returns (bool)
    {
        if (!_inPool(members, i, sellerKey, buyerKey)) return false;
        for (uint256 j = 0; j < seated.length; j++) {
            if (seated[j] == members[i]) return false;
        }
        return true;
    }

    /// Standard s4a: the stage view of the instance at `prev`, read by a static call handed 100,000 gas, its answer
    /// exactly 192 bytes of six well-formed words; `ok` is false where the account carries another code hash, the
    /// call fails, the answer has another length or form, or the account is not the one its answered salt derives
    /// under s1 (an account whose code is the proxy's but whose storage no order wrote proves no address).
    function stageOf(address prev, bytes32 proxyCodeHash)
        public
        view
        returns (bool ok, address sk, address bk, bool ended, uint16 bps, address judge)
    {
        if (prev.codehash != proxyCodeHash) return (false, sk, bk, ended, bps, judge);
        (bool s, bytes memory r) = prev.staticcall{gas: 100_000}(abi.encodeWithSelector(Eravon.stage.selector));
        if (!s || r.length != 192) return (false, sk, bk, ended, bps, judge);
        uint256 a;
        uint256 b;
        uint256 e;
        uint256 p;
        uint256 j;
        bytes32 salt_;
        assembly ("memory-safe") {
            a := mload(add(r, 32))
            b := mload(add(r, 64))
            e := mload(add(r, 96))
            p := mload(add(r, 128))
            j := mload(add(r, 160))
            salt_ := mload(add(r, 192))
        }
        if (a >> 160 != 0 || b >> 160 != 0 || e > 1 || p >> 16 != 0 || j >> 160 != 0) return (false, sk, bk, ended, bps, judge);
        if (prev != _instanceOf(salt_, prev)) return (false, sk, bk, ended, bps, judge);
        return (true, address(uint160(a)), address(uint160(b)), e == 1, uint16(p), address(uint160(j)));
    }

    /// Standard s1: the address the public deployer gives the proxy initcode (INIT_PREFIX and the account's own
    /// runtime) under `salt`.
    function _instanceOf(bytes32 salt_, address account) internal view returns (address) {
        return address(
            uint160(uint256(keccak256(abi.encodePacked(hex"ff", DEPLOYER, salt_, keccak256(abi.encodePacked(INIT_PREFIX, account.code))))))
        );
    }

    /// Standard s4 step 5 and `cancel` (s4a): the gate on the stage before. 0 passes; 1 BadPrev (no instance of this
    /// standard, or another pair of keys); 2 StageOpen (not ended, or ended leaving the buyer a share); 3 StageHeld
    /// (on `cancel`: no instance of this standard, not ended, or ended leaving the buyer nothing).
    function stageGate(address prev, bytes32 proxyCodeHash, address sellerKey, address buyerKey, bool cancelling)
        public
        view
        returns (uint8)
    {
        (bool ok, address sk, address bk, bool ended, uint16 bps,) = stageOf(prev, proxyCodeHash);
        if (cancelling) return (ok && ended && bps != 0) ? 0 : 3;
        if (!ok || sk != sellerKey || bk != buyerKey) return 1;
        if (!ended || bps != 0) return 2;
        return 0;
    }

    /// Standard s8, the terminal split and the two outcome views, one pure
    /// function: the buyer's share of A plus the buyer's fee where it returns
    /// to it plus the part of the seller's LF sent to it; the seller's share
    /// of A; the judge's P and the buyer's fee where the flags send them there;
    /// the deposit's remainder back to sellerFund; the part of the seller's LF
    /// sent to the buyer (bondTo 0 none, 1 all, 2 half rounded down); and the
    /// venue/1 s2.12 Outcome of the same action: the seat's `state`, the shares
    /// of A per role (buyer, seller), P and the buyer's fee as the standby and
    /// the ruling fee where the judge takes them, and the ruling fee borne per
    /// role (the buyer the fee less what the seller's LF returns it, the
    /// seller that part). The strays are the instance's to add.
    function split(
        uint256 a,
        uint16 buyerBps,
        uint256 premium,
        uint256 deposit,
        uint256 lfHeld,
        bool pToJudge,
        bool lfToJudge,
        uint8 bondTo,
        uint8 state
    )
        external
        pure
        returns (
            uint256 buyerPaid,
            uint256 sellerA,
            uint256 judgePaid,
            uint256 toSellerFund,
            uint256 bondToBuyer,
            IGaleedVenue.Outcome memory o
        )
    {
        uint256 buyerA = (a * buyerBps) / 10000;
        sellerA = a - buyerA;
        buyerPaid = buyerA;
        o.state = state;
        o.shares = new uint256[](2);
        o.rulingFeeFrom = new uint256[](2);
        o.shares[0] = buyerA;
        o.shares[1] = sellerA;
        if (pToJudge) {
            judgePaid += premium;
            o.standbyFee = premium;
        } else {
            toSellerFund += premium;
        }
        uint256 sellerLf = deposit - premium;
        if (bondTo == 1) bondToBuyer = sellerLf;
        else if (bondTo == 2) bondToBuyer = sellerLf / 2;
        if (lfToJudge) {
            judgePaid += lfHeld;
            o.rulingFee = lfHeld;
            o.rulingFeeFrom[0] = lfHeld - bondToBuyer;
            o.rulingFeeFrom[1] = bondToBuyer;
        } else {
            buyerPaid += lfHeld;
        }
        buyerPaid += bondToBuyer;
        toSellerFund += sellerLf - bondToBuyer;
    }

    /// terms/1 s4.2, s4.4: the order's struct hash, which is its salt and its caseId, and the offer's, read
    /// from the calldata of `init` or `cancel` forwarded whole, `t` being the first argument of either.
    function salt(Eravon.OrderTerms calldata t) public pure returns (bytes32 s, bytes32 offerHash) {
        offerHash = hashOffer(t.offer);
        s = keccak256(abi.encode(TH_ORDER, offerHash, t.buyerKey, t.buyerPayout, uint256(t.openBy), t.prev));
    }

    /// `init`'s calldata after the selector, forwarded as is: the salt and the offer's hash as `salt` gives them, the
    /// calling instance proving its address under standard s1 (WrongAddress, s4 step 2).
    function initSalt(Eravon.OrderTerms calldata t, bytes calldata) external view returns (bytes32 s, bytes32 offerHash) {
        (s, offerHash) = salt(t);
        if (msg.sender != _instanceOf(s, msg.sender)) revert Eravon.WrongAddress();
    }

    /// `cancel`'s calldata after the selector, forwarded as is, read in the order standard s4 writes: the salt as
    /// `salt` gives it; the calling instance proves its address under s1 (WrongAddress); `buyerKey` nonzero
    /// (NotBuyer) and `buyerPayout` nonzero (BadTerms(4)); then the stage before's gate (s4a): StageHeld unless
    /// `prev` is zero or that stage ended leaving the buyer a share.
    function cancelCheck(Eravon.OrderTerms calldata t, bytes calldata) external view returns (bytes32 s) {
        (s,) = salt(t);
        if (msg.sender != _instanceOf(s, msg.sender)) revert Eravon.WrongAddress();
        if (t.buyerKey == address(0)) revert Eravon.NotBuyer();
        if (t.buyerPayout == address(0)) revert Eravon.BadTerms(4);
        if (t.prev != address(0) && stageGate(t.prev, Eravon(msg.sender).PROXY_CODEHASH(), address(0), address(0), true) != 0) {
            revert Eravon.StageHeld();
        }
    }

    /// terms/1 s4.2: hashStruct(Offer).
    function hashOffer(Eravon.Offer calldata o) public pure returns (bytes32) {
        bytes32[] memory rungs = new bytes32[](o.ladder.length);
        for (uint256 i = 0; i < rungs.length; i++) {
            Eravon.Rung calldata r = o.ladder[i];
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
            Eravon.Rung calldata r = o.ladder[i];
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
    /// each word read as a big-endian integer; the fifteen words in s3.1's order.
    function _decodeQuote(bytes calldata quote) internal pure returns (bool bad, uint256[15] memory w) {
        if (quote.length != 480) return (true, w);
        for (uint256 i = 0; i < 15; i++) {
            w[i] = uint256(bytes32(quote[i * 32:i * 32 + 32]));
        }
        // address words: the first 12 bytes zero; token nonzero
        if (w[0] == 0 || w[0] >> 160 != 0 || w[2] >> 160 != 0 || w[3] >> 160 != 0 || w[4] >> 160 != 0) return (true, w);
        if (w[1] > 10000 || w[7] > 10000) return (true, w);
        if (w[5] > type(uint32).max || w[11] > type(uint32).max || w[12] > type(uint32).max) return (true, w);
        // lapseDefault 0, 1 or 2 (terms/1 s3.2, s3.3); autoAccept 0 or 1
        if (w[13] > 2 || w[14] > 1) return (true, w);
        if (w[2] == 0 && (w[3] != 0 || w[4] != 0)) return (true, w);
    }
}
