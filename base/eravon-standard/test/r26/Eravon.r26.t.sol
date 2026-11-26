// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "../EravonBase.sol";
import {Eravon} from "../../src/Eravon.sol";

/// An account that is no instance and answers stage() in the 192-byte form an instance does.
contract R26FakeStage {
    address internal immutable sk;
    address internal immutable bk;
    uint16 internal immutable bps;
    bytes32 internal immutable salt;

    constructor(address sk_, address bk_, uint16 bps_, bytes32 salt_) {
        sk = sk_;
        bk = bk_;
        bps = bps_;
        salt = salt_;
    }

    function stage() external view returns (address, address, bool, uint16, address, bytes32) {
        return (sk, bk, true, bps, address(0), salt);
    }
}

/// Writes chosen storage words in its constructor and returns `runtime` as its code: with the proxy's runtime, an
/// account that carries the proxy code hash and answers stage() from storage no order wrote.
contract R26Forger {
    constructor(bytes memory runtime, uint256[] memory slots, uint256[] memory vals) {
        for (uint256 i = 0; i < slots.length; i++) {
            uint256 sl = slots[i];
            uint256 v = vals[i];
            assembly {
                sstore(sl, v)
            }
        }
        assembly {
            return(add(runtime, 32), mload(runtime))
        }
    }
}

/// The one gate read that fails open (the draw's preference falls back to the pool's pick where stageOf answers not
/// ok) measured against the gas a drawer hands redraw; the two gates and the draw against an account that answers
/// stage() in an instance's form without being one (another code, or the proxy's code over forged storage); and
/// stage()'s form at every instance state, with stageOf ok exactly where the instance proved its address.
contract EravonR26 is EravonBase {
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("guild covenant");
    uint256 internal constant A = 100 ether;
    Q internal q;
    bytes internal quote;

    function setUp() public {
        setUpBase();
        q = defaultQuote(address(tok), address(0), false);
        quote = encodeQuote(q);
    }

    function _panel(uint256 n) internal pure returns (address[] memory m, bytes32[] memory w) {
        m = new address[](n);
        w = new bytes32[](n);
        for (uint256 i = 0; i < n; i++) {
            m[i] = address(uint160(0x001000000000000000000000000000000000000B00) + uint160(i + 1));
            w[i] = AMANAH;
        }
    }

    function _guild(address prev, address[] memory m, bytes32[] memory w) internal returns (Eravon inst, Eravon.OrderTerms memory t) {
        t = guildTerms(GUILD, AMANAH, quote, A);
        t.prev = prev;
        fundDeposit(q, A);
        inst = deployProxy(t);
        tok.mint(address(inst), A);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), m, w);
    }

    function _pick(bytes32 id, bytes32 seed, address[] memory m) internal pure returns (address) {
        return m[uint256(keccak256(abi.encodePacked(id, seed, uint64(0)))) % m.length];
    }

    function _redraw(Eravon inst, address[] memory m, bytes32[] memory w, uint256 g) internal returns (bool ok) {
        vm.prank(SELLER);
        (ok,) = address(inst).call{gas: g}(abi.encodeCall(Eravon.redraw, (m, w)));
    }

    /// A guild stage released with its accepted judge: the series judge it hands on.
    function _paidStage(address[] memory m, bytes32[] memory w) internal returns (Eravon inst1, address j1) {
        Eravon.OrderTerms memory t1;
        (inst1, t1) = _guild(address(0), m, w);
        drawNext(inst1, m, w);
        j1 = inst1.seatOf(orderHash(t1), 0).judge;
        vm.prank(j1, j1);
        inst1.accept(orderHash(t1), 0);
        claim(inst1, keccak256("m"));
        vm.prank(BUYER);
        inst1.confirm();
    }

    /// Every gas limit at which a draw lands seats the stage before's judge: no limit makes the stage view's static
    /// call fail while the rest of the draw completes on the pool's pick.
    function test_R26_thePreferenceCannotBeStarvedByTheDrawsGas() public {
        (address[] memory m, bytes32[] memory w) = _panel(6);
        engagePanel(GUILD, m, w, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst1, address j1) = _paidStage(m, w);
        (,,,, address sj,) = inst1.stage();
        assertEq(sj, j1, "the released stage's series judge is its accepted judge");
        // a stage 2 whose pool pick is another member than j1, so that a starved preference would show
        Eravon inst2;
        Eravon.OrderTerms memory t2;
        for (uint256 k = 0; k < 20; k++) {
            (inst2, t2) = _guild(address(inst1), m, w);
            vm.roll(vm.getBlockNumber() + 1);
            if (_pick(orderHash(t2), blockhash(inst2.status().drawBlock), m) != j1) break;
        }
        address other = _pick(orderHash(t2), blockhash(inst2.status().drawBlock), m);
        assertTrue(other != j1, "a case whose pool pick is not the preferred judge");
        // the lowest limit at which the draw lands
        uint256 lo = 30_000;
        uint256 hi = 3_000_000;
        uint256 snap = vm.snapshotState();
        assertTrue(_redraw(inst2, m, w, hi), "lands at 3M");
        vm.revertToState(snap);
        while (hi - lo > 1) {
            uint256 mid = (lo + hi) / 2;
            snap = vm.snapshotState();
            bool ok = _redraw(inst2, m, w, mid);
            vm.revertToState(snap);
            if (ok) hi = mid;
            else lo = mid;
        }
        emit log_named_uint("lowest landing gas", hi);
        uint256 landed;
        uint256 refused;
        // every limit from 3,000 below the lowest to 3,000 above it, then 200,000 above it in steps of 101
        for (uint256 g = hi - 3_000; g < hi + 203_000; g += (g < hi + 3_000 ? 1 : 101)) {
            snap = vm.snapshotState();
            bool ok = _redraw(inst2, m, w, g);
            if (ok) {
                landed++;
                assertEq(inst2.seatOf(orderHash(t2), 0).judge, j1, "a landed draw seats the stage before's judge");
            } else {
                refused++;
                assertEq(inst2.status().seats, 0, "a refused draw seats nobody");
                assertTrue(inst2.status().pendingRedraw, "and leaves the draw armed");
            }
            vm.revertToState(snap);
        }
        emit log_named_uint("limits landed", landed);
        emit log_named_uint("limits refused", refused);
        assertGt(landed, 0);
        assertGt(refused, 0);
    }

    function _proxyRuntime() internal view returns (bytes memory) {
        return abi.encodePacked(hex"363d3d373d3d3d363d73", address(logic), hex"5af43d82803e903d91602b57fd5bf3");
    }

    /// The proxy's code over storage that says: Released at buyerBps `b` (or Refunded where `b` is the whole share),
    /// the order's keys, series judge `j`, caseId `salt`.
    function _forge(uint16 b, address j, bytes32 salt) internal returns (address f) {
        uint256[] memory sl = new uint256[](5);
        uint256[] memory v = new uint256[](5);
        sl[0] = 0; // _phase at byte 0, _stageBps (the share plus one) at bytes 1 and 2
        v[0] = (b == 10000 ? 4 : 5) | (uint256(b) + 1) << 8;
        sl[1] = 2; // sellerKey
        v[1] = uint256(uint160(SELLER));
        sl[2] = 3; // buyerKey
        v[2] = uint256(uint160(BUYER));
        sl[3] = 15; // caseId
        v[3] = uint256(salt);
        sl[4] = 34; // _seriesJudge
        v[4] = uint256(uint160(j));
        f = address(new R26Forger(_proxyRuntime(), sl, v));
        assertEq(f.codehash, logic.PROXY_CODEHASH(), "the forged account carries the proxy code hash");
        (bool ok, bytes memory r) = f.staticcall(abi.encodeWithSelector(Eravon.stage.selector));
        assertTrue(ok);
        assertEq(r.length, 192, "and answers the whole form");
        (address sk, address bk, bool ended, uint16 bps, address sj, bytes32 cid) =
            abi.decode(r, (address, address, bool, uint16, address, bytes32));
        assertEq(sk, SELLER);
        assertEq(bk, BUYER);
        assertTrue(ended);
        assertEq(bps, b);
        assertEq(sj, j);
        assertEq(cid, salt);
    }

    /// Both gates over `prev`: init answers BadPrev whatever the share, cancel StageHeld even where it answers a
    /// share, stageOf not ok.
    function _refusedOver(address prev, uint256 nonce) internal {
        Eravon.OrderTerms memory t = namedTerms(address(0x1DD9E0001), AMANAH, quote, A);
        t.prev = prev;
        t.offer.nonce = nonce;
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        bytes memory bs = buyerSig(t);
        bytes memory ss = sellerSig(t);
        vm.prank(SELLER);
        (bool ok, bytes memory ret) = address(inst).call(abi.encodeCall(Eravon.init, (t, quote, bs, ss, new address[](0), new bytes32[](0))));
        assertFalse(ok, "init over a form");
        assertEq(bytes4(ret), Eravon.BadPrev.selector);
        vm.prank(BUYER);
        (ok, ret) = address(inst).call(abi.encodeCall(Eravon.cancel, (t, bytes(""))));
        assertFalse(ok, "cancel over a form");
        assertEq(bytes4(ret), Eravon.StageHeld.selector);
        (bool sok,,,,,) = verifier.stageOf(prev, logic.PROXY_CODEHASH());
        assertFalse(sok, "stageOf: no instance");
    }

    /// The gates read the proxy code hash and the address proof, not a form: an account answering stage() as an
    /// ended stage with the order's keys, in the 192-byte form, opens nothing (BadPrev) whatever its share and frees
    /// nothing (StageHeld) even where it answers a share; so does an account carrying the proxy code hash whose
    /// storage no order wrote, whatever salt it answers (a genuine instance's included); and at the draw such an
    /// account gives no preference, where a genuine stage naming the same judge does.
    function test_R26_anAccountAnsweringTheFormIsNoStage() public {
        (address[] memory m, bytes32[] memory w) = _panel(6);
        engagePanel(GUILD, m, w, uint64(vm.getBlockTimestamp() + 30 days));
        (Eravon inst1, address j1) = _paidStage(m, w);
        (,,,,, bytes32 salt1) = inst1.stage();
        assertTrue(salt1 != bytes32(0));
        uint256 nonce = 1000;
        for (uint16 b = 0; b <= 10000; b += 10000) {
            _refusedOver(address(new R26FakeStage(SELLER, BUYER, b, keccak256("any salt"))), nonce++);
            // forged storage under an arbitrary salt, and under the salt a genuine instance proved its address with
            _refusedOver(_forge(b, j1, keccak256(abi.encode("forged", b))), nonce++);
            _refusedOver(_forge(b, j1, salt1), nonce++);
        }
        // the draw: a guild case's verifier.draw with the order's prev at a forged account naming j1 gives the
        // pool's pick, and at the genuine stage naming j1 seats j1
        address forged = _forge(0, j1, salt1);
        (Eravon inst2, Eravon.OrderTerms memory t2) = _guild(address(0), m, w);
        uint256[11] memory wd;
        wd[0] = uint256(orderHash(t2));
        wd[7] = uint256(keccak256(abi.encode(m, w)));
        wd[8] = 1;
        address[5] memory parties;
        parties[0] = BUYER;
        parties[1] = SELLER;
        address[] memory seated = new address[](0);
        bool shown;
        for (uint256 k = 1; k < 40; k++) {
            wd[6] = uint256(keccak256(abi.encode("seed", k)));
            wd[10] = 0;
            vm.prank(address(inst2));
            (address pool,) = verifier.draw(wd, parties, m, w, seated);
            if (pool == j1) continue;
            wd[10] = uint256(uint160(forged));
            vm.prank(address(inst2));
            (address viaForged,) = verifier.draw(wd, parties, m, w, seated);
            assertEq(viaForged, pool, "a forged prev gives no preference");
            wd[10] = uint256(uint160(address(inst1)));
            vm.prank(address(inst2));
            (address viaStage,) = verifier.draw(wd, parties, m, w, seated);
            assertEq(viaStage, j1, "the genuine stage's series judge is preferred");
            shown = true;
            break;
        }
        assertTrue(shown, "a seed whose pool pick is not j1");
    }

    /// stage() answers the 192-byte form at every instance state; stageOf answers ok exactly where the instance
    /// proved its address (caseId nonzero: init, cancel, the expiry branch), and not on an instance that never did.
    function test_R26_stageOfAnswersEveryInstanceInForm() public {
        bytes32 key = logic.PROXY_CODEHASH();
        // never opened: the form, caseId zero, not ok
        Eravon.OrderTerms memory t = namedTerms(address(0x1DD9E0001), AMANAH, quote, A);
        Eravon inst = deployProxy(t);
        (bool s, bytes memory r) = address(inst).staticcall(abi.encodeWithSelector(Eravon.stage.selector));
        assertTrue(s);
        assertEq(r.length, 192);
        (,,,,, bytes32 cid) = inst.stage();
        assertEq(cid, bytes32(0));
        (bool ok,,,,,) = verifier.stageOf(address(inst), key);
        assertFalse(ok, "a never-opened instance is no stage");
        // cancelled: ok, ended at the whole share, its salt
        vm.prank(BUYER);
        inst.cancel(t, bytes(""));
        _okAt(inst, orderHash(t), true, 10000);
        // the expiry branch: ok, ended at the whole share, its salt
        Eravon.OrderTerms memory te = namedTerms(address(0x1DD9E0001), AMANAH, quote, A);
        te.offer.nonce = 77;
        Eravon instE = deployProxy(te);
        tok.mint(address(instE), A);
        vm.prank(SELLER);
        logic.revoke(SELLER, te.offer.termsHash);
        vm.prank(BUYER);
        instE.init(te, quote, bytes(""), bytes(""), new address[](0), new bytes32[](0));
        _okAt(instE, orderHash(te), true, 10000);
        // opened: ok, not ended
        Eravon.OrderTerms memory to = namedTerms(address(0x1DD9E0001), AMANAH, quote, A);
        to.offer.nonce = 78;
        to.offer.termsHash = keccak256("other terms");
        fundDeposit(q, A);
        Eravon instO = deployProxy(to);
        tok.mint(address(instO), A);
        vm.prank(SELLER);
        instO.init(to, quote, buyerSig(to), sellerSig(to), new address[](0), new bytes32[](0));
        _okAt(instO, orderHash(to), false, 0);
    }

    function _okAt(Eravon inst, bytes32 salt, bool ended_, uint16 bps_) internal view {
        (bool s, bytes memory r) = address(inst).staticcall(abi.encodeWithSelector(Eravon.stage.selector));
        assertTrue(s);
        assertEq(r.length, 192);
        (,,,,, bytes32 cid) = inst.stage();
        assertEq(cid, salt);
        (bool ok, address sk, address bk, bool ended, uint16 bps,) = verifier.stageOf(address(inst), logic.PROXY_CODEHASH());
        assertTrue(ok);
        assertEq(sk, SELLER);
        assertEq(bk, BUYER);
        assertEq(ended, ended_);
        assertEq(bps, bps_);
    }
}
