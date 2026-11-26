// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {EravonBase} from "../EravonBase.sol";
import {Eravon} from "../../src/Eravon.sol";
import {IGaleedVenue} from "../../src/IGaleedVenue.sol";
import {FixedRouter} from "../replay/ReplayRouters.sol";

/// An account that is no instance and answers stage() in the instance's 192-byte form as a paid (or failed) stage
/// with the order's keys, a judge and a salt.
contract StageFakeStage {
    address internal immutable sk;
    address internal immutable bk;
    uint16 internal immutable bps;
    address internal immutable judge;
    bytes32 internal immutable salt;

    constructor(address sk_, address bk_, uint16 bps_, address judge_, bytes32 salt_) {
        sk = sk_;
        bk = bk_;
        bps = bps_;
        judge = judge_;
        salt = salt_;
    }

    function stage() external view returns (address, address, bool, uint16, address, bytes32) {
        return (sk, bk, true, bps, judge, salt);
    }
}

/// Writes chosen storage words in its constructor and returns `runtime` as its code: with the proxy's runtime, an
/// account that carries the proxy code hash and answers stage() from storage no order wrote.
contract StageForger {
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

/// A stateful handler over chains of stages (standard s4a, s7). Stages are created against zero, against an earlier
/// stage (with the same keys or other ones), against an account that is no instance (no code, another code, a
/// contract answering the stage form, an account carrying the proxy code hash over forged storage); they are opened,
/// cancelled, run down the expiry branch (a revocation or openBy), called with terms the address does not prove, and
/// driven to every terminal on the named track, on a guild track without a router (three panels the guild re-engages
/// between stages) and on a guild track with a router. Every gate's answer is predicted from an independent model of
/// the stage before (its keys, whether and how it ended, its series judge), every draw's member from the model of
/// the preference and the pool, and every instance's six-word `stage()` and slot 0 are compared with the model after
/// every call. The series judge of a stage: zero where it ended Lapsed; else its last seat's judge where that seat
/// was accepted and did not resign; else the series judge of the stage before as it stood at this stage's init.
contract StageHandler is EravonBase {
    uint256 internal constant SELLER2_PK = 0xA11CF;
    uint256 internal constant BUYER2_PK = 0xB0C;
    address internal SELLER2;
    address internal BUYER2;
    address internal constant J = address(0x1DD9E0001);
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    address internal constant GUILDR = address(0x6000000000000000000000000000000000000002);
    address internal constant STRANGER_D = address(0x3000000000000000000000000000000000000077);
    bytes32 internal constant AMANAH = keccak256("guild covenant");
    uint256 internal constant A = 100 ether;
    uint256 internal constant MAX_STAGES = 64;
    uint256 internal constant RIDX = 3;

    struct St {
        address inst;
        address seller;
        uint256 sellerPk;
        address buyer;
        uint256 buyerPk;
        uint8 track; // 0 named, 1 guild without a router, 2 guild with a router
        uint8 qi;
        uint64 openBy;
        uint256 expiry;
        address prev;
        uint256 prevIdx; // index + 1 of the stage at prev; 0 where prev is zero or no stage
        uint8 prevKind; // 0 zero, 1 a stage, 2 an account without code, 3 a contract that is no instance, 4 forged storage
        bool opened;
        bool cancelled;
        bool expiredRun;
        bool revoked;
        uint8 panelVar;
        // the model of stage()
        bool mEnded;
        uint16 mBps;
        bool mLapsed;
        address mInherited; // the series judge the stage before handed on at init
        // the model of the last seat
        bool mHasSeat;
        address mSeatJudge;
        bool mAccepted;
        bool mResigned;
        // the stage before as it stood when this stage opened or was cancelled
        bool atGateEnded;
        uint16 atGateBps;
        // money
        uint256 ghostIn;
        int256 netExt;
        uint16 pendingBps; // the share the action under way names (settle, rule)
    }

    St[] internal stages;
    Q[3] internal qs;
    bytes[3] internal quotes;
    address[][3] internal panels;
    uint8 internal guildVar;
    FixedRouter internal router;

    uint256 public violations;
    string[] internal violLog;
    mapping(bytes32 => uint256) public reached;
    uint256 public prefTaken;
    uint256 public prefDeclined;
    uint256 public forgedMade;

    constructor() {
        setUpBase();
        SELLER2 = vm.addr(SELLER2_PK);
        BUYER2 = vm.addr(BUYER2_PK);
        qs[0] = defaultQuote(address(tok), address(0), false); // offers stand open; lapse to the buyer
        qs[1] = defaultQuote(address(tok), address(0), true);
        qs[1].lapseDefault = 0; // auto-accepted seats; lapse to the seller
        qs[2] = defaultQuote(address(tok), address(0), false);
        qs[2].lapseDefault = 1; // a lapse at half
        for (uint256 k = 0; k < 3; k++) quotes[k] = encodeQuote(qs[k]);
        address[7] memory m;
        for (uint256 k = 0; k < 6; k++) m[k] = address(uint160(0x0010000000000000000000000000000000000B00) + uint160(k + 1));
        m[6] = J;
        panels[0] = new address[](7);
        for (uint256 k = 0; k < 7; k++) panels[0][k] = m[k];
        // a panel with a zero entry, a party's key and a repeat; two members and the named judge
        panels[1] = new address[](6);
        panels[1][0] = m[1];
        panels[1][1] = address(0);
        panels[1][2] = SELLER;
        panels[1][3] = m[1];
        panels[1][4] = m[3];
        panels[1][5] = J;
        panels[2] = new address[](4);
        panels[2][0] = m[4];
        panels[2][1] = m[5];
        panels[2][2] = BUYER;
        panels[2][3] = m[0];
        router = new FixedRouter(RIDX);
        bytes32 key = logic.PROXY_CODEHASH();
        vm.prank(GUILDR);
        pbook.engage(key, panels[0], _words(7), bytes32(0), uint64(vm.getBlockTimestamp() + 3650 days), address(router));
        _engage(0);
    }

    // ------------------------------------------------------------------ views for the invariants

    function count() external view returns (uint256) {
        return stages.length;
    }

    function violation(uint256 k) external view returns (string memory) {
        return violLog[k];
    }

    function violCount() external view returns (uint256) {
        return violLog.length;
    }

    function stageAt(uint256 i) external view returns (St memory) {
        return stages[i];
    }

    function tokAddr() external view returns (address) {
        return address(tok);
    }

    function holderSum() external view returns (uint256 s, uint256 supply) {
        supply = tok.totalSupply();
        s = tok.balanceOf(SELLER) + tok.balanceOf(SELLER2) + tok.balanceOf(BUYER) + tok.balanceOf(BUYER2)
            + tok.balanceOf(address(this)) + tok.balanceOf(address(logic));
        for (uint256 i = 0; i < stages.length; i++) {
            s += tok.balanceOf(stages[i].inst) + tok.balanceOf(_sp(i)) + tok.balanceOf(_bp(i));
        }
        for (uint256 k = 0; k < 7; k++) s += tok.balanceOf(panels[0][k]);
    }

    function sp(uint256 i) external pure returns (address) {
        return _sp(i);
    }

    function bp(uint256 i) external pure returns (address) {
        return _bp(i);
    }

    // ------------------------------------------------------------------ helpers

    function _sp(uint256 i) internal pure returns (address) {
        return address(uint160(0x5E00000000 + i));
    }

    function _bp(uint256 i) internal pure returns (address) {
        return address(uint160(0xB000000000 + i));
    }

    function _mark(string memory p) internal {
        reached[keccak256(bytes(p))]++;
    }

    function _viol(string memory why) internal {
        violations++;
        violLog.push(why);
    }

    function _words(uint256 n) internal pure returns (bytes32[] memory w) {
        w = new bytes32[](n);
        for (uint256 k = 0; k < n; k++) w[k] = AMANAH;
    }

    function _engage(uint8 v) internal {
        bytes32 key = logic.PROXY_CODEHASH();
        vm.prank(GUILD);
        pbook.engage(key, panels[v], _words(panels[v].length), bytes32(0), uint64(vm.getBlockTimestamp() + 3650 days), address(0));
        guildVar = v;
        vm.roll(vm.getBlockNumber() + 1);
    }

    function _terms(uint256 i) internal view returns (Eravon.OrderTerms memory t) {
        St storage s = stages[i];
        t.offer.sellerKey = s.seller;
        t.offer.judge = s.track == 0 ? J : address(0);
        t.offer.guild = s.track == 1 ? GUILD : (s.track == 2 ? GUILDR : address(0));
        t.offer.sellerPayout = _sp(i);
        t.offer.sellerFund = s.seller;
        t.offer.token = address(tok);
        t.offer.amount = A;
        t.offer.deliveryWindow = 7 days;
        t.offer.confirmWindow = 5 days;
        t.offer.openWindow = 1 days;
        t.offer.buyerMayOpen = true;
        t.offer.ladder = threeRungs();
        t.offer.termsHash = keccak256(abi.encode("stage terms", i));
        t.offer.amanahHash = AMANAH;
        t.offer.quoteHash = keccak256(quotes[s.qi]);
        t.offer.nonce = i + 1;
        t.offer.expiry = s.expiry;
        t.buyerKey = s.buyer;
        t.buyerPayout = _bp(i);
        t.openBy = s.openBy;
        t.prev = s.prev;
    }

    function _pick(uint256 r, function(uint256) internal view returns (bool) ok) internal view returns (bool, uint256) {
        uint256 n = stages.length;
        if (n == 0) return (false, 0);
        uint256 start = r % n;
        for (uint256 k = 0; k < n; k++) {
            uint256 i = (start + k) % n;
            if (ok(i)) return (true, i);
        }
        return (false, 0);
    }

    function _any(uint256) internal pure returns (bool) {
        return true;
    }

    function _isNone(uint256 i) internal view returns (bool) {
        return !stages[i].opened && !stages[i].cancelled;
    }

    function _live(uint256 i) internal view returns (bool) {
        if (!stages[i].opened) return false;
        uint8 ph = uint8(Eravon(stages[i].inst).status().phase);
        return ph >= 1 && ph <= 3;
    }

    function _phase(uint256 i) internal view returns (uint8) {
        return uint8(Eravon(stages[i].inst).status().phase);
    }

    function _sel(bytes memory ret) internal pure returns (bytes4 s) {
        if (ret.length >= 4) s = bytes4(ret);
    }

    /// A call from `from` on stage i, the money its two keys move counted against the stage.
    function _call(uint256 i, address from, bytes memory data) internal returns (bool ok, bytes memory ret) {
        St storage s = stages[i];
        int256 b0 = int256(tok.balanceOf(s.seller) + tok.balanceOf(s.buyer));
        vm.prank(from, from);
        (ok, ret) = s.inst.call(data);
        s.netExt += int256(tok.balanceOf(s.seller) + tok.balanceOf(s.buyer)) - b0;
    }

    function _fundDeposit(address seller, uint256 d) internal {
        tok.mint(seller, d);
        uint256 have = tok.allowance(seller, address(logic));
        vm.prank(seller);
        tok.approve(address(logic), have + d);
    }

    function _members(uint256 i) internal view returns (address[] memory m, bytes32[] memory w) {
        St storage s = stages[i];
        if (s.track == 0) return (new address[](0), new bytes32[](0));
        uint8 v = s.track == 2 ? 0 : (s.opened ? s.panelVar : guildVar);
        m = panels[v];
        w = _words(m.length);
    }

    /// The series judge of stage i as the model holds it.
    function _mJudge(uint256 i) internal view returns (address) {
        St storage s = stages[i];
        if (s.mLapsed) return address(0);
        if (s.mHasSeat && s.mAccepted && !s.mResigned) return s.mSeatJudge;
        return s.mInherited;
    }

    /// Whether stage i proved its address (init, cancel or the expiry branch): its caseId is nonzero.
    function _keyed(uint256 i) internal view returns (bool) {
        St storage s = stages[i];
        return s.opened || s.cancelled || s.expiredRun;
    }

    /// The stage at prev as the model holds it: whether stageOf answers it ok (an instance that proved its address),
    /// its keys, ended, bps, series judge. A stage that never proved its address, and every other kind, is no
    /// instance.
    function _prevModel(St storage s) internal view returns (bool inst, address sk, address bk, bool ended, uint16 bps, address judge) {
        if (s.prevKind != 1) return (false, sk, bk, ended, bps, judge);
        uint256 j = s.prevIdx - 1;
        if (!_keyed(j)) return (false, sk, bk, ended, bps, judge);
        St storage p = stages[j];
        inst = true;
        sk = p.seller;
        bk = p.buyer;
        ended = p.mEnded;
        bps = p.mBps;
        judge = _mJudge(j);
    }

    /// Whether the series judge of the stage at prev is inherited (its own last seat not serving).
    function _prevInherited(St storage s) internal view returns (bool) {
        if (s.prevKind != 1) return false;
        St storage p = stages[s.prevIdx - 1];
        return !p.mLapsed && !(p.mHasSeat && p.mAccepted && !p.mResigned) && p.mInherited != address(0);
    }

    function _seat(St storage s, address judge) internal {
        s.mHasSeat = true;
        s.mSeatJudge = judge;
        s.mAccepted = qs[s.qi].autoAccept;
        s.mResigned = false;
    }

    /// After any call on stage i: the terminal's share by the phase reached, the judge by the seats, and the
    /// instance's view and slot 0 against the model.
    function _post(uint256 i) internal {
        St storage s = stages[i];
        Eravon e = Eravon(s.inst);
        uint8 ph = _phase(i);
        if (ph >= 4 && !s.mEnded) {
            s.mEnded = true;
            if (ph == 5) {
                s.mBps = 0;
                _mark("terminal.Released");
            } else if (ph == 4 || ph == 8) {
                s.mBps = 10000;
                _mark(ph == 4 ? "terminal.Refunded" : "terminal.Resigned");
            } else if (ph == 6) {
                s.mBps = s.pendingBps;
                _mark(s.pendingBps == 0 ? "terminal.Settled0" : "terminal.SettledShare");
            } else if (ph == 7) {
                s.mBps = s.pendingBps;
                _mark(s.pendingBps == 0 ? "terminal.Ruled0" : "terminal.RuledShare");
            } else if (ph == 9) {
                s.mBps = uint16(qs[s.qi].lapseDefault) * 5000;
                if (s.mHasSeat && s.mAccepted && !s.mResigned) _mark("judge.lapsedOverServed");
                s.mLapsed = true;
                _mark(s.mBps == 0 ? "terminal.Lapsed0" : "terminal.LapsedShare");
            }
        }
        if (s.opened) {
            uint256 n = e.status().seats;
            if ((n != 0) != s.mHasSeat) _viol("model seat: whether a seat stands differs from the model's");
            if (n != 0) {
                IGaleedVenue.Seat memory l = e.seatOf(_id(i), uint64(n - 1));
                if (l.judge != s.mSeatJudge) _viol("model seat: the last seat's judge differs from the model's");
                if ((l.accepted != 0) != s.mAccepted) _viol("model seat: the last seat's acceptance differs from the model's");
                if ((l.state == 3) != s.mResigned) _viol("model seat: the last seat's resignation differs from the model's");
            }
            if (!s.mLapsed && s.mInherited != address(0) && _mJudge(i) == s.mInherited) {
                if (!s.mHasSeat || !s.mAccepted) _mark("judge.inherited.noServedSeat");
                else if (s.mResigned) _mark("judge.inherited.resigned");
            }
        }
        _checkView(i);
    }

    function _id(uint256 i) internal view returns (bytes32) {
        return orderHash(_terms(i));
    }

    function _checkView(uint256 i) internal {
        if (!_viewAgrees(i)) _viol(string.concat("stage() or slot 0 differs from the model at stage ", vm.toString(i)));
    }

    function viewAgrees(uint256 i) external view returns (bool) {
        return _viewAgrees(i);
    }

    function _viewAgrees(uint256 i) internal view returns (bool) {
        St storage s = stages[i];
        Eravon e = Eravon(s.inst);
        (address sk, address bk, bool ended, uint16 bps, address j, bytes32 cid) = e.stage();
        bool keyed = _keyed(i);
        if (sk != (keyed ? s.seller : address(0)) || bk != (keyed ? s.buyer : address(0))) return false;
        if (ended != s.mEnded || bps != (s.mEnded ? s.mBps : 0)) return false;
        if (j != _mJudge(i)) return false;
        if (cid != (keyed ? _id(i) : bytes32(0))) return false;
        // slot 0: the phase at byte 0, the sentinel at bytes 1 and 2
        uint256 w = uint256(vm.load(s.inst, bytes32(0)));
        uint8 ph = uint8(w);
        uint16 sent = uint16(w >> 8);
        if (ph != uint8(e.status().phase)) return false;
        bool term = ph >= 4;
        // ended iff a terminal, a cancel or the expiry branch ran
        if ((sent != 0) != (term || s.cancelled || s.expiredRun)) return false;
        if (sent != 0 && sent - 1 != s.mBps) return false;
        if (sent > 10001) return false;
        return true;
    }

    // ------------------------------------------------------------------ the stages

    function newStage(uint256 a, uint256 b) external {
        _newStage(a, b, 0);
    }

    /// A guild stage chained on a stage that ended paid, with its keys, opened and drawn at once: the preference's
    /// subject.
    function chainStage(uint256 a, uint256 b) external {
        uint256 n = stages.length;
        if (n == 0 || n >= MAX_STAGES) return;
        uint256 j = a % n;
        bool found;
        for (uint256 k = 0; k < n; k++) {
            uint256 jj = (j + k) % n;
            if (stages[jj].opened && stages[jj].mEnded && stages[jj].mBps == 0) {
                j = jj;
                found = true;
                break;
            }
        }
        if (!found) return;
        _newStage(a, b, j + 1);
        uint256 i = stages.length - 1;
        _initStage(i, 1);
        if (_pendingDraw(i)) _draw(i, b >> 8);
        _mark("chain.made");
    }

    function _newStage(uint256 a, uint256 b, uint256 forced) internal {
        if (stages.length >= MAX_STAGES) return;
        uint256 i = stages.length;
        stages.push();
        St storage s = stages[i];
        bool s2 = a % 5 == 0;
        bool b2 = (a >> 8) % 5 == 0;
        uint8 kind;
        uint256 c = b % 10;
        if (forced != 0 || (c >= 2 && c <= 7 && i != 0)) {
            kind = 1;
            uint256 j = forced != 0 ? forced - 1 : (b >> 8) % i;
            if (forced == 0 && (b >> 100) % 2 == 0) {
                for (uint256 k = 0; k < i; k++) {
                    uint256 jj = (j + k) % i;
                    if (stages[jj].opened && stages[jj].mEnded && stages[jj].mBps == 0) {
                        j = jj;
                        break;
                    }
                }
            }
            s.prevIdx = j + 1;
            s.prev = stages[j].inst;
            // mostly the keys of the stage before, so that the gates' far sides are reached
            if (forced != 0 || (a >> 16) % 6 != 0) {
                s2 = stages[j].seller == SELLER2;
                b2 = stages[j].buyer == BUYER2;
            }
        } else if (c == 8) {
            kind = 2;
            s.prev = address(uint160(0xBAD00000 + i));
        } else if (c == 9) {
            kind = 3;
            s.prev = address(verifier);
            uint256 v = (b >> 60) % 3;
            bool paid = (b >> 62) % 2 == 0;
            bytes32 salt = i != 0 && (b >> 63) % 2 == 0 ? _id((b >> 64) % i) : keccak256(abi.encode("forged salt", i));
            if (v == 1) {
                // a contract answering stage() in the instance's form with these keys, ended, at 0 or at the whole share
                s.prev = address(new StageFakeStage(s2 ? SELLER2 : SELLER, b2 ? BUYER2 : BUYER, paid ? 0 : 10000, panels[0][0], salt));
                _mark("new.prevFake");
            } else if (v == 2) {
                kind = 4;
                s.prev = _forge(s2 ? SELLER2 : SELLER, b2 ? BUYER2 : BUYER, paid, panels[0][0], salt);
            }
        }
        s.prevKind = kind;
        s.seller = s2 ? SELLER2 : SELLER;
        s.sellerPk = s2 ? SELLER2_PK : SELLER_PK;
        s.buyer = b2 ? BUYER2 : BUYER;
        s.buyerPk = b2 ? BUYER2_PK : BUYER_PK;
        s.track = forced != 0 ? ((a >> 24) % 4 == 0 ? 2 : 1) : uint8((a >> 24) % 3);
        s.qi = uint8((a >> 32) % 3);
        s.openBy = uint64(vm.getBlockTimestamp() + 60 days + (b >> 40) % 60 days);
        s.expiry = uint256(s.openBy) + 10 days;
        Eravon.OrderTerms memory t = _terms(i);
        s.inst = address(deployProxy(t));
        tok.mint(s.inst, A);
        s.ghostIn = A;
        if (kind == 0) _mark("new.prevZero");
        else if (kind == 1) {
            St storage p = stages[s.prevIdx - 1];
            _mark(p.seller == s.seller && p.buyer == s.buyer ? "new.prevStage" : "new.prevStageOtherKeys");
        } else if (kind == 4) _mark("new.prevForged");
        else _mark(kind == 2 ? "new.prevNoCode" : "new.prevOtherCode");
        _checkView(i);
    }

    function _proxyRuntime() internal view returns (bytes memory) {
        return abi.encodePacked(hex"363d3d373d3d3d363d73", address(logic), hex"5af43d82803e903d91602b57fd5bf3");
    }

    /// An account carrying the proxy code hash whose storage says: ended (Released at 0, or Refunded at the whole
    /// share), these keys, this series judge, this salt; no order wrote it, so its address proves nothing.
    function _forge(address sk, address bk, bool paid, address judge, bytes32 salt) internal returns (address f) {
        uint256[] memory sl = new uint256[](5);
        uint256[] memory v = new uint256[](5);
        sl[0] = 0; // _phase at byte 0, _stageBps (the share plus one) at bytes 1 and 2
        v[0] = paid ? (5 | (uint256(1) << 8)) : (4 | (uint256(10001) << 8));
        sl[1] = 2; // sellerKey
        v[1] = uint256(uint160(sk));
        sl[2] = 3; // buyerKey
        v[2] = uint256(uint160(bk));
        sl[3] = 15; // caseId
        v[3] = uint256(salt);
        sl[4] = 34; // _seriesJudge
        v[4] = uint256(uint160(judge));
        f = address(new StageForger(_proxyRuntime(), sl, v));
        if (f.codehash != logic.PROXY_CODEHASH()) _viol("the forged account does not carry the proxy code hash");
        // the account answers the whole form as an ended stage: only the address proof refuses it
        (bool ok, bytes memory r) = f.staticcall(abi.encodeWithSelector(Eravon.stage.selector));
        if (!ok || r.length != 192) {
            _viol("the forged account does not answer the 192-byte form");
        } else {
            (address a, address b_, bool e, uint16 p, address j, bytes32 c) = abi.decode(r, (address, address, bool, uint16, address, bytes32));
            if (a != sk || b_ != bk || !e || p != (paid ? 0 : 10000) || j != judge || c != salt) _viol("the forged account answers other words than its storage");
        }
        forgedMade++;
    }

    /// init's gate: the answer predicted from the model of the stage before.
    function initStage(uint256 a, uint256 b) external {
        (bool f, uint256 i) = _pick(a, (b >> 40) % 4 == 0 ? _overLive : _isNone);
        if (!f) (f, i) = _pick(a, _isNone);
        if (!f) return;
        _initStage(i, b);
    }

    function _initStage(uint256 i, uint256 b) internal {
        St storage s = stages[i];
        if (s.expiredRun || s.revoked || vm.getBlockTimestamp() > s.openBy || vm.getBlockTimestamp() > s.expiry) return;
        Eravon.OrderTerms memory t = _terms(i);
        (address[] memory m, bytes32[] memory w) = _members(i);
        _fundDeposit(s.seller, depositOf(qs[s.qi], A));
        bytes memory bs = sign(s.buyerPk, digestOf(s.inst, orderHash(t)));
        bytes memory ss = sign(s.sellerPk, digestOf(address(logic), offerHash(t)));
        bytes4 want;
        string memory path;
        if (s.prevKind == 0) {
            path = "init.ok.first";
        } else if (s.prevKind >= 2) {
            want = Eravon.BadPrev.selector;
            path = s.prevKind == 4 ? "init.badPrev.forged" : "init.badPrev.noInstance";
        } else {
            (bool pin, address psk, address pbk, bool ended, uint16 bps,) = _prevModel(s);
            if (!pin) {
                want = Eravon.BadPrev.selector;
                path = "init.badPrev.unopened";
            } else if (psk != s.seller || pbk != s.buyer) {
                want = Eravon.BadPrev.selector;
                path = "init.badPrev.keys";
            } else if (!ended) {
                want = Eravon.StageOpen.selector;
                path = "init.stageOpen.inFlight";
            } else if (bps != 0) {
                want = Eravon.StageOpen.selector;
                path = "init.stageOpen.share";
            } else {
                path = "init.ok.later";
            }
        }
        address from = b % 4 == 0 ? s.buyer : s.seller;
        bytes memory qb = quotes[s.qi];
        (bool ok, bytes memory ret) = _call(i, from, abi.encodeCall(Eravon.init, (t, qb, bs, ss, m, w)));
        if (want == bytes4(0)) {
            if (!ok) {
                _viol(string.concat("init refused where the model opens: ", path, " ", vm.toString(ret)));
                return;
            }
        } else if (ok || _sel(ret) != want) {
            _viol(string.concat("init's gate differs from the model: ", path));
            if (ok) {
                s.opened = true;
            }
            _post(i);
            return;
        }
        _mark(path);
        if (!ok) {
            _checkView(i);
            return;
        }
        s.opened = true;
        if (s.prevKind == 1) {
            (,, bool pe, uint16 pb,,) = Eravon(s.prev).stage();
            s.atGateEnded = pe;
            s.atGateBps = pb;
            (,,,,, address pj) = _prevModel(s);
            s.mInherited = pj;
            uint8 how = _phase(s.prevIdx - 1);
            if (how == 5) _mark("init.ok.overReleased");
            else if (how == 6) _mark("init.ok.overSettled0");
            else if (how == 7) _mark("init.ok.overRuled0");
            else if (how == 9) _mark("init.ok.overLapsed0");
        }
        if (s.track == 0) _seat(s, J);
        else if (s.track == 1) s.panelVar = guildVar;
        _post(i);
    }

    /// cancel's gate: the answer predicted from the model of the stage before.
    function _overLive(uint256 i) internal view returns (bool) {
        St storage s = stages[i];
        if (s.prevKind != 1 || s.opened || s.cancelled) return false;
        St storage p = stages[s.prevIdx - 1];
        return p.opened && !p.mEnded;
    }

    /// cancel's answer on stage i's own terms, as the model predicts it.
    function _cancelWant(uint256 i) internal view returns (bytes4 want, string memory path) {
        St storage s = stages[i];
        if (s.opened || s.cancelled) {
            want = Eravon.AlreadyInitialized.selector;
            path = "cancel.already";
        } else if (s.prevKind == 0) {
            path = "cancel.ok.first";
        } else if (s.prevKind >= 2) {
            want = Eravon.StageHeld.selector;
            path = s.prevKind == 4 ? "cancel.held.forged" : "cancel.held.noInstance";
        } else {
            (bool pin,,, bool ended, uint16 bps,) = _prevModel(s);
            St storage p = stages[s.prevIdx - 1];
            if (!pin || !ended) {
                want = Eravon.StageHeld.selector;
                path = p.opened ? "cancel.held.inFlight" : "cancel.held.unopened";
            } else if (bps == 0) {
                want = Eravon.StageHeld.selector;
                path = "cancel.held.paid";
            } else if (p.cancelled) {
                path = "cancel.ok.overCancelled";
            } else if (p.expiredRun && !p.opened) {
                path = "cancel.ok.overExpiry";
            } else {
                path = bps == 10000 ? "cancel.ok.overWhole" : "cancel.ok.overShare";
            }
        }
    }

    function cancelStage(uint256 a, uint256 b) external {
        (bool f, uint256 i) = _pick(a, (b >> 40) % 3 == 0 ? _overLive : _any);
        if (!f) (f, i) = _pick(a, _any);
        if (!f) return;
        _cancelAt(i, b);
    }

    function _cancelAt(uint256 i, uint256 b) internal {
        St storage s = stages[i];
        Eravon.OrderTerms memory t = _terms(i);
        (bytes4 want, string memory path) = _cancelWant(i);
        bool signed = b % 4 == 0;
        bytes memory sig = signed ? sign(s.buyerPk, digestOf(s.inst, keccak256(abi.encode(TH_CANCEL)))) : bytes("");
        (bool ok, bytes memory ret) = _call(i, signed ? s.seller : s.buyer, abi.encodeCall(Eravon.cancel, (t, sig)));
        if (want == bytes4(0)) {
            if (!ok) {
                _viol(string.concat("cancel refused where the model frees: ", path, " ", vm.toString(ret)));
                return;
            }
        } else if (ok || _sel(ret) != want) {
            _viol(string.concat("cancel's gate differs from the model: ", path));
            if (ok) s.cancelled = true;
            _post(i);
            return;
        }
        _mark(path);
        if (ok) {
            if (signed) _mark("cancel.signedForm");
            s.cancelled = true;
            if (s.prevKind == 1) {
                (,, bool pe, uint16 pb,,) = Eravon(s.prev).stage();
                s.atGateEnded = pe;
                s.atGateBps = pb;
            }
        }
        _post(i);
    }

    /// init or cancel with terms other than the ones the instance's address proves (another nonce): the address proof
    /// (s1, step 2) answers WrongAddress before the expiry branch, the consents and the stage gate; an instance that
    /// opened or was cancelled answers AlreadyInitialized first.
    function wrongAddress(uint256 a, uint256 b) external {
        (bool f, uint256 i) = _pick(a, (b >> 40) % 2 == 0 ? _isNone : _any);
        if (!f) return;
        St storage s = stages[i];
        Eravon.OrderTerms memory t = _terms(i);
        t.offer.nonce += 1 + (b >> 80) % 1000;
        bool isInit = b % 2 == 0;
        bytes4 want = (s.opened || s.cancelled) ? Eravon.AlreadyInitialized.selector : Eravon.WrongAddress.selector;
        bytes memory data = isInit
            ? abi.encodeCall(Eravon.init, (t, quotes[s.qi], bytes(""), bytes(""), new address[](0), new bytes32[](0)))
            : abi.encodeCall(Eravon.cancel, (t, bytes("")));
        (bool ok, bytes memory ret) = _call(i, isInit ? s.seller : s.buyer, data);
        if (ok || _sel(ret) != want) {
            _viol(string.concat(isInit ? "init" : "cancel", " on terms the address does not prove differs from the model"));
            _post(i);
            return;
        }
        if (want == Eravon.WrongAddress.selector) {
            if (isInit) {
                _mark("wrongAddress.init");
                if (s.prevKind >= 2 || (s.prevKind == 1 && !_keyed(s.prevIdx - 1))) _mark("wrongAddress.initOverBadPrev");
            } else {
                _mark("wrongAddress.cancel");
                (bytes4 w,) = _cancelWant(i);
                if (w == Eravon.StageHeld.selector) _mark("wrongAddress.cancelOverHeld");
            }
        }
        _post(i);
    }

    /// The expiry branch on a stage no case opened: by a revocation, by openBy, or run again.
    function expireStage(uint256 a, uint256 b) external {
        if ((b >> 16) % 3 != 0) return;
        (bool f, uint256 i) = _pick(a, _isNone);
        if (!f) return;
        St storage s = stages[i];
        Eravon.OrderTerms memory t = _terms(i);
        uint256 mode = b % 8;
        string memory path;
        if (s.expiredRun || s.revoked || vm.getBlockTimestamp() > s.openBy || vm.getBlockTimestamp() > s.expiry) {
            path = s.expiredRun ? "expiry.again" : "expiry.afterTime";
        } else if (mode == 0) {
            vm.warp(uint256(s.openBy) + 1);
            vm.roll(vm.getBlockNumber() + 1);
            path = "expiry.openBy";
        } else {
            vm.prank(s.seller);
            logic.revoke(s.seller, t.offer.termsHash);
            s.revoked = true;
            path = "expiry.revocation";
        }
        address from = (b >> 8) % 2 == 0 ? s.buyer : s.seller;
        bytes memory qb = quotes[s.qi];
        (bool ok, bytes memory ret) = _call(i, from, abi.encodeCall(Eravon.init, (t, qb, bytes(""), bytes(""), new address[](0), new bytes32[](0))));
        if (!ok) {
            _viol(string.concat("the expiry branch refused: ", path, " ", vm.toString(ret)));
            return;
        }
        _mark(path);
        if (s.prevKind == 1) {
            (,,, bool pe,,) = _prevModel(s);
            _mark(pe ? "expiry.overEnded" : "expiry.overNotEnded");
        }
        s.expiredRun = true;
        s.mEnded = true;
        s.mBps = 10000;
        _post(i);
    }

    // ------------------------------------------------------------------ the draw and the preference

    function _inPool(address[] memory m, address x, address sk, address bk) internal pure returns (bool) {
        if (x == address(0) || x == sk || x == bk) return false;
        for (uint256 k = 0; k < m.length; k++) if (m[k] == x) return true;
        return false;
    }

    function _seatedBefore(uint256 i, address x) internal view returns (bool) {
        Eravon e = Eravon(stages[i].inst);
        uint256 n = e.status().seats;
        bytes32 id = _id(i);
        for (uint256 r = 0; r < n; r++) if (e.seatOf(id, uint64(r)).judge == x) return true;
        return false;
    }

    function _poolPick(uint256 i, address[] memory m, bytes32 seed, uint64 ordinal) internal view returns (address) {
        if (seed == bytes32(0)) return address(0);
        St storage s = stages[i];
        address[] memory el = new address[](m.length);
        uint256 n;
        for (uint256 k = 0; k < m.length; k++) {
            address x = m[k];
            if (x == address(0) || x == s.seller || x == s.buyer) continue;
            bool dup;
            for (uint256 q = 0; q < k; q++) if (m[q] == x) dup = true;
            if (dup || _seatedBefore(i, x)) continue;
            el[n++] = x;
        }
        if (n == 0) return address(0);
        return el[uint256(keccak256(abi.encodePacked(_id(i), seed, ordinal))) % n];
    }

    function _pendingDraw(uint256 i) internal view returns (bool) {
        return _live(i) && Eravon(stages[i].inst).status().pendingRedraw;
    }

    function drawStage(uint256 a, uint256 b) external {
        (bool f, uint256 i) = _pick(a, _pendingDraw);
        if (!f) return;
        _draw(i, b);
    }

    function _draw(uint256 i, uint256 b) internal {
        St storage s = stages[i];
        Eravon e = Eravon(s.inst);
        Eravon.CaseStatus memory st = e.status();
        if (vm.getBlockNumber() <= st.drawBlock) vm.roll(st.drawBlock + 1);
        if (vm.getBlockTimestamp() > st.redrawDeadline && vm.getBlockNumber() > st.drawBlock + 1) return;
        (address[] memory m, bytes32[] memory w) = _members(i);
        uint64 ordinal = uint64(st.seats);
        address want;
        string memory path;
        if (s.track == 2) {
            want = m[RIDX];
            if (_seatedBefore(i, want)) want = address(0);
            (,,,,, address pj) = _prevModel(s);
            path = (pj != address(0) && pj != m[RIDX]) ? "draw.router.noPreference" : "draw.router";
        } else {
            bytes32 seed = blockhash(st.drawBlock);
            (,,,,, address pj) = _prevModel(s);
            if (seed == bytes32(0)) {
                want = address(0);
                path = "draw.staleSeed";
            } else if (s.prevKind != 1) {
                want = _poolPick(i, m, seed, ordinal);
                path = "draw.pref.noPrev";
            } else if (pj == address(0)) {
                want = _poolPick(i, m, seed, ordinal);
                path = "draw.pref.noJudge";
            } else if (!_inPool(m, pj, s.seller, s.buyer)) {
                want = _poolPick(i, m, seed, ordinal);
                path = "draw.pref.offPanel";
            } else if (_seatedBefore(i, pj)) {
                want = _poolPick(i, m, seed, ordinal);
                path = "draw.pref.heldOrOffered";
            } else {
                want = pj;
                path = _prevInherited(s) ? "draw.pref.takenInherited" : "draw.pref.taken";
            }
        }
        // s7: open to any sender; a stranger draws one time in four
        address from = b % 4 == 0 ? s.seller : (b % 4 == 1 ? s.buyer : (b % 4 == 2 ? (s.track == 2 ? GUILDR : GUILD) : STRANGER_D));
        if (b % 4 == 3) _mark("draw.byStranger");
        (bool ok, bytes memory ret) = _call(i, from, abi.encodeCall(Eravon.redraw, (m, w)));
        if (!ok) {
            _viol(string.concat("a draw in its window refused: ", path, " ", vm.toString(ret)));
            return;
        }
        uint256 n = e.status().seats;
        if (want == address(0)) {
            if (n != ordinal || _phase(i) != 8) _viol(string.concat("a draw with no member to seat seated one: ", path));
        } else {
            if (n != ordinal + 1) {
                _viol(string.concat("the draw seated nobody: ", path));
            } else {
                address got = e.seatOf(_id(i), ordinal).judge;
                if (got != want) _viol(string.concat("the draw seated another member than the model: ", path));
                _seat(s, got);
                if (want == _prefJudge(s) && want != address(0)) prefTaken++;
            }
        }
        _mark(path);
        _post(i);
    }

    function _prefJudge(St storage s) internal view returns (address pj) {
        if (s.track == 2) return address(0);
        (,,,,, pj) = _prevModel(s);
    }

    // ------------------------------------------------------------------ the case's acts

    function _lastSeat(uint256 i) internal view returns (bool, IGaleedVenue.Seat memory l) {
        Eravon e = Eravon(stages[i].inst);
        uint256 n = e.status().seats;
        if (n == 0) return (false, l);
        l = e.seatOf(_id(i), uint64(n - 1));
        return (true, l);
    }

    function acceptStage(uint256 a) external {
        (bool f, uint256 i) = _pick(a, _live);
        if (!f) return;
        _accept(i);
    }

    function _accept(uint256 i) internal {
        (bool has, IGaleedVenue.Seat memory l) = _lastSeat(i);
        if (!has || l.accepted != 0 || l.state != 0) return;
        Eravon e = Eravon(stages[i].inst);
        uint64 seat = uint64(e.status().seats - 1);
        (bool ok,) = _call(i, l.judge, abi.encodeCall(Eravon.accept, (_id(i), seat)));
        if (ok) {
            stages[i].mAccepted = true;
            _mark("act.accept");
        }
        _post(i);
    }

    function claimStage(uint256 a) external {
        (bool f, uint256 i) = _pick(a, _live);
        if (!f) return;
        _claim(i);
    }

    function _claim(uint256 i) internal {
        (bool ok,) = _call(i, stages[i].seller, abi.encodeCall(Eravon.claimDelivery, (keccak256("manifest"))));
        if (ok) _mark("act.claim");
        _post(i);
    }

    function confirmStage(uint256 a) external {
        (bool f, uint256 i) = _pick(a, _live);
        if (!f) return;
        _confirm(i);
    }

    function _confirm(uint256 i) internal {
        (bool ok,) = _call(i, stages[i].buyer, abi.encodeCall(Eravon.confirm, ()));
        if (ok) _mark("act.confirm");
        _post(i);
    }

    function refundStage(uint256 a) external {
        if ((a >> 200) % 3 != 0) return;
        (bool f, uint256 i) = _pick(a, _live);
        if (!f) return;
        _refund(i);
    }

    function _refund(uint256 i) internal {
        (bool ok,) = _call(i, stages[i].seller, abi.encodeCall(Eravon.refund, ()));
        if (ok) _mark("act.refund");
        _post(i);
    }

    function settleStage(uint256 a, uint256 b) external {
        (bool f, uint256 i) = _pick(a, _live);
        if (!f) return;
        _settle(i, b);
    }

    function _settle(uint256 i, uint256 b) internal {
        St storage s = stages[i];
        uint256 c = b % 4;
        uint16 bps = c == 0 ? 0 : (c == 1 ? 10000 : (c == 2 ? 2500 : uint16((b >> 8) % 10001)));
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 h = digestOf(s.inst, keccak256(abi.encode(TH_SETTLE, uint256(bps), exp)));
        bytes memory sb = sign(s.buyerPk, h);
        bytes memory ss = sign(s.sellerPk, h);
        s.pendingBps = bps;
        (bool ok,) = _call(i, (b >> 30) % 2 == 0 ? s.seller : s.buyer, abi.encodeCall(Eravon.settle, (bps, exp, sb, ss)));
        if (ok) _mark("act.settle");
        _post(i);
    }

    function disputeStage(uint256 a) external {
        (bool f, uint256 i) = _pick(a, _live);
        if (!f) return;
        _dispute(i);
    }

    function _dispute(uint256 i) internal {
        St storage s = stages[i];
        Eravon e = Eravon(s.inst);
        (uint256 lf,,,,) = e.feeQuoted();
        tok.mint(s.buyer, lf);
        vm.prank(s.buyer);
        tok.approve(s.inst, lf);
        (bool ok,) = _call(i, s.buyer, abi.encodeCall(Eravon.dispute, ()));
        if (ok) _mark("act.dispute");
        _post(i);
    }

    function ruleStage(uint256 a, uint256 b) external {
        (bool f, uint256 i) = _pick(a, _live);
        if (!f) return;
        _rule(i, b);
    }

    function _rule(uint256 i, uint256 b) internal {
        (bool has, IGaleedVenue.Seat memory l) = _lastSeat(i);
        if (!has || l.state != 1) return;
        uint64 rung = uint64(b % 3);
        bytes memory entry = abi.encode("ruling", i, rung);
        bytes32 entryId = sha256(entry);
        uint64 seat = uint64(Eravon(stages[i].inst).status().seats - 1);
        stages[i].pendingBps = threeRungs()[rung].buyerBps;
        (bool ok,) = _call(i, l.judge, abi.encodeCall(Eravon.rule, (_id(i), seat, entryId, rung, entry, bytes(""))));
        if (ok) _mark("act.rule");
        _post(i);
    }

    function resignStage(uint256 a) external {
        (bool f, uint256 i) = _pick(a, _live);
        if (!f) return;
        _resign(i);
    }

    function _resign(uint256 i) internal {
        (bool has, IGaleedVenue.Seat memory l) = _lastSeat(i);
        if (!has || l.state > 1) return;
        bytes memory entry = abi.encode("resign", i);
        bytes32 entryId = sha256(entry);
        uint64 seat = uint64(Eravon(stages[i].inst).status().seats - 1);
        (bool ok,) = _call(i, l.judge, abi.encodeCall(Eravon.resign, (_id(i), seat, entryId, entry)));
        if (ok) {
            stages[i].mResigned = true;
            _mark(l.accepted == 0 ? "act.decline" : "act.resign");
            if (l.accepted == 0 && stages[i].prevKind == 1 && _mJudge(stages[i].prevIdx - 1) == l.judge) {
                prefDeclined++;
                _mark("act.preferredDeclines");
            }
        }
        _post(i);
    }

    /// The clocks: to the last seat's window or the draw's deadline and past it, or a small step.
    function advanceStage(uint256 a, uint256 b) external {
        (bool f, uint256 i) = _pick(a, _live);
        if (!f) return;
        _advance(i, b);
    }

    function _advance(uint256 i, uint256 b) internal {
        Eravon e = Eravon(stages[i].inst);
        Eravon.CaseStatus memory st = e.status();
        (bool has, IGaleedVenue.Seat memory l) = _lastSeat(i);
        uint256 to;
        if (st.pendingRedraw) to = st.redrawDeadline;
        else if (has) to = l.windowEnd;
        if (b % 3 != 0 && to >= vm.getBlockTimestamp()) vm.warp(to + 1);
        else vm.warp(vm.getBlockTimestamp() + 1 + (b >> 8) % 2 days);
        vm.roll(vm.getBlockNumber() + 2);
        uint64 seat = st.seats == 0 ? 0 : uint64(st.seats - 1);
        uint8 ph0 = _phase(i);
        (bool ok,) = _call(i, address(this), abi.encodeCall(Eravon.advance, (_id(i), seat)));
        if (ok && _phase(i) != ph0) _mark("act.advanceMoved");
        _post(i);
    }


    /// One step of a live case along its natural course: the draw, the judge's word, the delivery, the buyer's word,
    /// the ruling or the clocks, so that chains of stages reach their ends.
    function driveStage(uint256 a, uint256 b) external {
        (bool f, uint256 i) = _pick(a, _live);
        if (!f) return;
        Eravon e = Eravon(stages[i].inst);
        Eravon.CaseStatus memory st = e.status();
        uint8 ph = uint8(st.phase);
        uint256 c = b % 10;
        if (st.pendingRedraw) return _draw(i, b >> 8);
        (bool has, IGaleedVenue.Seat memory l) = _lastSeat(i);
        if (!has) return;
        if (l.accepted == 0 && l.state == 0) {
            if (c < 8) return _accept(i);
            return _resign(i);
        }
        if (ph == 1) {
            if (c < 9) return _claim(i);
            return _advance(i, 1);
        }
        if (ph == 2) {
            if (c < 4) return _confirm(i);
            if (c < 8) return _dispute(i);
            if (c < 9) return _settle(i, b >> 8);
            return _advance(i, 1);
        }
        if (ph == 3) {
            if (c < 7) return _rule(i, b >> 8);
            if (c < 9) return _advance(i, 1);
            return _resign(i);
        }
    }

    // ------------------------------------------------------------------ FUZZ r26-2: the thin paths

    function _forceGuild(uint256 a) internal pure returns (uint256) {
        return (a & ~(uint256(3) << 24)) | (uint256(1) << 24);
    }

    function _servedOrInheritedPaid(uint256 j) internal view returns (bool) {
        St storage p = stages[j];
        return p.opened && p.mEnded && p.mBps == 0 && _mJudge(j) != address(0);
    }

    function _inheritedPaid(uint256 j) internal view returns (bool) {
        St storage p = stages[j];
        return p.opened && p.mEnded && p.mBps == 0 && !p.mLapsed && !(p.mHasSeat && p.mAccepted && !p.mResigned)
            && p.mInherited != address(0);
    }

    function _liveUnended(uint256 j) internal view returns (bool) {
        return _live(j) && !stages[j].mEnded;
    }

    /// A stage over a stage in flight, with its keys: cancel is held (StageHeld) and init is refused (StageOpen).
    function liveStage(uint256 a, uint256 b) external {
        if (stages.length >= MAX_STAGES) return;
        (bool f, uint256 j) = _pick(a, _liveUnended);
        if (!f) return;
        _newStage(a, b, j + 1);
        uint256 i = stages.length - 1;
        _cancelAt(i, b);
        _initStage(i, b);
        _mark("thin.live.made");
    }

    /// A guild stage over a paid stage whose series judge it served or inherited; the stage's judge accepts, the seller
    /// claims, the judge resigns at Claimed and the buyer confirms: Released at 0 over a resigned seat, so that its
    /// series judge is the inherited one.
    function unservedMiddle(uint256 a, uint256 b) external {
        if (stages.length >= MAX_STAGES) return;
        (bool f, uint256 j) = _pick(a, _servedOrInheritedPaid);
        if (!f) return;
        _newStage(_forceGuild(a), b, j + 1);
        uint256 i = stages.length - 1;
        _initStage(i, 1);
        if (!stages[i].opened) return;
        if (_pendingDraw(i)) _draw(i, b >> 8);
        if (_pendingDraw(i) || _phase(i) != 1) return;
        _accept(i);
        _claim(i);
        _resign(i);
        _confirm(i);
        if (_phase(i) == 5 && _inheritedPaid(i)) _mark("thin.unserved.made");
    }

    /// A guild stage over a paid stage whose series judge is inherited, opened and drawn at once.
    function inheritChain(uint256 a, uint256 b) external {
        if (stages.length >= MAX_STAGES) return;
        (bool f, uint256 j) = _pick(a, _inheritedPaid);
        if (!f) return;
        _newStage(_forceGuild(a), b, j + 1);
        uint256 i = stages.length - 1;
        _initStage(i, 1);
        if (_pendingDraw(i)) _draw(i, b >> 8);
        _mark("thin.inherit.made");
    }

    function engage(uint256 a) external {
        uint8 v = uint8(a % 3);
        _engage(v);
        _mark(v == 0 ? "engage.v0" : (v == 1 ? "engage.v1" : "engage.v2"));
    }

    function warp(uint256 a) external {
        vm.warp(vm.getBlockTimestamp() + 1 hours + a % 3 days);
        vm.roll(vm.getBlockNumber() + 1);
    }

    // ------------------------------------------------------------------ checks the invariants call

    /// Every stage opened over a stage: an instance with its keys, ended with buyerBps 0 now and at the gate, its
    /// seller paid the whole amount.
    function openedOverPaid() external view returns (uint256 bad) {
        for (uint256 i = 0; i < stages.length; i++) {
            St storage s = stages[i];
            if (!s.opened || s.prevKind == 0) continue;
            if (s.prevKind != 1) {
                bad++;
                continue;
            }
            (address sk, address bk, bool ended, uint16 bps, address pj, bytes32 pc) = Eravon(s.prev).stage();
            if (sk != s.seller || bk != s.buyer || !ended || bps != 0 || !s.atGateEnded || s.atGateBps != 0) bad++;
            // the stage before proved its address, and its series judge is the one this stage inherited
            if (pc == bytes32(0) || pj != s.mInherited) bad++;
            uint256 j = s.prevIdx - 1;
            uint256 sellerGot = tok.received(_sp(j)) + Eravon(stages[j].inst).owed(_sp(j), address(tok));
            if (sellerGot != A) bad++;
        }
    }

    /// Every stage cancelled over a stage: ended with buyerBps nonzero now and at the gate, and the buyer of that
    /// stage paid a share of it.
    function cancelledOverFailed() external view returns (uint256 bad) {
        for (uint256 i = 0; i < stages.length; i++) {
            St storage s = stages[i];
            if (!s.cancelled || s.prevKind == 0) continue;
            if (s.prevKind != 1) {
                bad++;
                continue;
            }
            (,, bool ended, uint16 bps,, bytes32 pc) = Eravon(s.prev).stage();
            if (!ended || bps == 0 || !s.atGateEnded || s.atGateBps == 0 || pc == bytes32(0)) bad++;
            uint256 j = s.prevIdx - 1;
            uint256 sellerGot = tok.received(_sp(j)) + Eravon(stages[j].inst).owed(_sp(j), address(tok));
            if (sellerGot >= A) bad++;
            uint256 buyerGot = tok.received(_bp(j)) + Eravon(stages[j].inst).owed(_bp(j), address(tok));
            if (buyerGot < (A * bps) / 10000) bad++;
        }
    }

    /// A stage whose prev is no instance (no code, another code, the form, forged storage) never opens and is never
    /// cancelled.
    function badPrevMoved() external view returns (uint256 bad) {
        for (uint256 i = 0; i < stages.length; i++) {
            St storage s = stages[i];
            if (s.prevKind < 2) continue;
            if (s.opened || s.cancelled || uint8(Eravon(s.inst).status().phase) != 0) bad++;
        }
    }

    /// Each stage's money: what flowed in (the amount minted to it, the deposit and the fee its keys paid) equals its
    /// balance plus what its two payouts received; its balance equals its credits plus what an open case holds.
    function moneyOff() external view returns (uint256 bad) {
        for (uint256 i = 0; i < stages.length; i++) {
            St storage s = stages[i];
            Eravon e = Eravon(s.inst);
            int256 lhs = int256(s.ghostIn);
            int256 rhs = int256(tok.balanceOf(s.inst) + tok.received(_sp(i)) + tok.received(_bp(i))) + s.netExt;
            if (lhs != rhs) bad++;
            Eravon.CaseStatus memory st = e.status();
            uint8 ph = uint8(st.phase);
            uint256 bal = tok.balanceOf(s.inst);
            if (ph >= 1 && ph <= 3) {
                if (bal != A + st.deposit + st.lfHeld + st.unpaid) bad++;
            } else if (ph >= 4 || s.expiredRun) {
                if (bal != st.unpaid) bad++;
            } else if (bal != s.ghostIn) {
                bad++;
            }
            // the terminal as paid: the seller's payout holds the seller's share of A exactly
            if (s.opened && ph >= 4) {
                uint256 sellerGot = tok.received(_sp(i)) + e.owed(_sp(i), address(tok));
                if (sellerGot != A - (A * s.mBps) / 10000) bad++;
            }
            if ((s.cancelled || s.expiredRun) && !s.opened) {
                if (tok.received(_sp(i)) != 0 || tok.received(_bp(i)) != A) bad++;
            }
        }
    }

    function viewsOff() external view returns (uint256 bad) {
        for (uint256 i = 0; i < stages.length; i++) if (!_viewAgrees(i)) bad++;
    }
}

contract EravonStageInvariants is Test {
    StageHandler internal h;

    function setUp() public {
        h = new StageHandler();
        targetContract(address(h));
        bytes4[] memory sel = new bytes4[](30);
        sel[0] = h.newStage.selector;
        sel[1] = h.newStage.selector;
        sel[2] = h.initStage.selector;
        sel[3] = h.initStage.selector;
        sel[4] = h.cancelStage.selector;
        sel[5] = h.expireStage.selector;
        sel[6] = h.drawStage.selector;
        sel[7] = h.drawStage.selector;
        sel[8] = h.acceptStage.selector;
        sel[9] = h.claimStage.selector;
        sel[10] = h.confirmStage.selector;
        sel[11] = h.refundStage.selector;
        sel[12] = h.settleStage.selector;
        sel[13] = h.disputeStage.selector;
        sel[14] = h.ruleStage.selector;
        sel[15] = h.resignStage.selector;
        sel[16] = h.advanceStage.selector;
        sel[17] = h.engage.selector;
        sel[18] = h.warp.selector;
        sel[19] = h.driveStage.selector;
        sel[20] = h.driveStage.selector;
        sel[21] = h.driveStage.selector;
        sel[22] = h.driveStage.selector;
        sel[23] = h.driveStage.selector;
        sel[24] = h.chainStage.selector;
        sel[25] = h.chainStage.selector;
        sel[26] = h.wrongAddress.selector;
        sel[27] = h.liveStage.selector;
        sel[28] = h.unservedMiddle.selector;
        sel[29] = h.inheritChain.selector;
        targetSelector(FuzzSelector({addr: address(h), selectors: sel}));
    }

    function _log() internal {
        uint256 n = h.violCount();
        for (uint256 k = 0; k < n && k < 5; k++) emit log(h.violation(k));
    }

    function invariant_S1_noViolationAtAnyCall() public {
        if (h.violations() != 0) _log();
        assertEq(h.violations(), 0, "a gate, a draw or a view differed from the model");
    }

    function invariant_S2_aStageOpensOnlyOverAPaidStage() public view {
        assertEq(h.openedOverPaid(), 0);
    }

    function invariant_S3_aStageIsCancelledOnlyOverAFailedStage() public view {
        assertEq(h.cancelledOverFailed(), 0);
    }

    function invariant_S4_aPrevThatIsNoInstanceNeverOpensOrCancels() public view {
        assertEq(h.badPrevMoved(), 0);
    }

    function invariant_S5_everyStagesMoney() public view {
        assertEq(h.moneyOff(), 0);
        (uint256 s, uint256 supply) = h.holderSum();
        assertEq(s, supply, "every token minted sits with a known holder");
    }

    function invariant_S6_theViewAndTheSentinelAgreeWithThePhase() public view {
        assertEq(h.viewsOff(), 0);
    }
}

/// The handler's reach: a fixed walk, rounds added while a path is unreached, every path at least once.
contract EravonStageReach is Test {
    uint256 internal constant N = 70;

    function _want() internal pure returns (string[N] memory) {
        return [
            "new.prevZero",
            "new.prevStage",
            "new.prevStageOtherKeys",
            "new.prevNoCode",
            "new.prevOtherCode",
            "new.prevFake",
            "new.prevForged",
            "init.ok.first",
            "init.ok.later",
            "init.badPrev.noInstance",
            "init.badPrev.forged",
            "init.badPrev.unopened",
            "init.badPrev.keys",
            "init.stageOpen.inFlight",
            "init.stageOpen.share",
            "init.ok.overReleased",
            "init.ok.overSettled0",
            "init.ok.overRuled0",
            "init.ok.overLapsed0",
            "cancel.already",
            "cancel.ok.first",
            "cancel.held.noInstance",
            "cancel.held.forged",
            "cancel.held.inFlight",
            "cancel.held.unopened",
            "cancel.held.paid",
            "cancel.ok.overCancelled",
            "cancel.ok.overExpiry",
            "cancel.ok.overWhole",
            "cancel.ok.overShare",
            "cancel.signedForm",
            "wrongAddress.init",
            "wrongAddress.initOverBadPrev",
            "wrongAddress.cancel",
            "wrongAddress.cancelOverHeld",
            "expiry.openBy",
            "expiry.revocation",
            "expiry.again",
            "expiry.afterTime",
            "expiry.overEnded",
            "expiry.overNotEnded",
            "draw.router",
            "draw.router.noPreference",
            "draw.pref.noPrev",
            "draw.pref.noJudge",
            "draw.pref.offPanel",
            "draw.pref.heldOrOffered",
            "draw.pref.taken",
            "draw.pref.takenInherited",
            "draw.byStranger",
            "act.preferredDeclines",
            "judge.inherited.noServedSeat",
            "judge.inherited.resigned",
            "judge.lapsedOverServed",
            "terminal.Released",
            "terminal.Refunded",
            "terminal.Resigned",
            "terminal.Settled0",
            "terminal.SettledShare",
            "terminal.Ruled0",
            "terminal.RuledShare",
            "terminal.Lapsed0",
            "terminal.LapsedShare",
            "engage.v0",
            "engage.v1",
            "engage.v2",
            "chain.made",
            "thin.live.made",
            "thin.unserved.made",
            "thin.inherit.made"
        ];
    }

    function _sel(StageHandler h) internal pure returns (bytes4[28] memory) {
        return [
            h.chainStage.selector,
            h.chainStage.selector,
            h.newStage.selector,
            h.initStage.selector,
            h.driveStage.selector,
            h.initStage.selector,
            h.cancelStage.selector,
            h.expireStage.selector,
            h.drawStage.selector,
            h.acceptStage.selector,
            h.claimStage.selector,
            h.confirmStage.selector,
            h.refundStage.selector,
            h.settleStage.selector,
            h.disputeStage.selector,
            h.ruleStage.selector,
            h.resignStage.selector,
            h.advanceStage.selector,
            h.engage.selector,
            h.warp.selector,
            h.driveStage.selector,
            h.driveStage.selector,
            h.driveStage.selector,
            h.driveStage.selector,
            h.wrongAddress.selector,
            h.liveStage.selector,
            h.unservedMiddle.selector,
            h.inheritChain.selector
        ];
    }

    function test_stageHandlerReachesEveryPath() public {
        string[N] memory want = _want();
        uint256[N] memory got;
        uint256 calls;
        uint256 okCalls;
        uint256 rounds;
        uint256 taken;
        uint256 declined;
        uint256 forged;
        uint256 base = vm.envOr("STAGE_REACH_SEED", uint256(0x5EA1));
        for (uint256 round = 0; round < 64; round++) {
            rounds = round + 1;
            uint256 snapId = vm.snapshotState();
            StageHandler h = new StageHandler();
            bytes4[28] memory sel = _sel(h);
            uint256 r = uint256(keccak256(abi.encode(base, round)));
            for (uint256 k = 0; k < 1000; k++) {
                r = uint256(keccak256(abi.encode(r, k)));
                vm.resetGasMetering();
                (bool ok,) = address(h).call(abi.encodeWithSelector(sel[r % 28], r >> 8, uint256(keccak256(abi.encode(r, 1)))));
                calls++;
                if (ok) okCalls++;
            }
            for (uint256 p = 0; p < N; p++) got[p] += h.reached(keccak256(bytes(want[p])));
            if (h.violations() != 0) {
                for (uint256 k = 0; k < h.violCount() && k < 5; k++) emit log(h.violation(k));
            }
            assertEq(h.violations(), 0, "a violation on the walk");
            assertEq(h.openedOverPaid() + h.cancelledOverFailed() + h.badPrevMoved() + h.moneyOff() + h.viewsOff(), 0, "an invariant on the walk");
            taken += h.prefTaken();
            declined += h.prefDeclined();
            forged += h.forgedMade();
            vm.revertToState(snapId);
            bool all = true;
            for (uint256 p = 0; p < N; p++) if (got[p] == 0) all = false;
            if (round >= 7 && all) break;
        }
        emit log_named_uint("rounds", rounds);
        emit log_named_uint("calls", calls);
        emit log_named_uint("calls that did not revert", okCalls);
        emit log_named_uint("preferred judges seated", taken);
        emit log_named_uint("preferred judges that declined", declined);
        emit log_named_uint("forged accounts made", forged);
        for (uint256 p = 0; p < N; p++) emit log_named_uint(want[p], got[p]);
        for (uint256 p = 0; p < N; p++) assertGt(got[p], 0, want[p]);
    }
}
