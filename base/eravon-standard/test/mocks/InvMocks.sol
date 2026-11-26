// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {MockERC20} from "../replay/MockERC20.sol";

/// The campaign's settlement token: the surface of MockERC20, plus a list
/// of receivers a transfer to which reverts, or answers false and moves
/// nothing (a blocked payout, as a token with a deny list has it), and a
/// meter of the largest gas any `transfer` was handed since the handler last
/// reset it (the no-copy reader's pricing calls apart). Round 4: its answers
/// to the standard's own callers (an instance, the logic, the verifier, and
/// the handler's no-copy reader) are shaped: `transfer`, `transferFrom`,
/// `balanceOf` and `decimals` each answer exactly the bytes the campaign
/// sets, the first word the true answer, false, or a garbage word; every
/// other caller is answered plainly. The meter also counts an instance's
/// transfers, and those that answered in the shape set.
contract BlockableToken {
    string public name = "Blockable";
    string public symbol = "BLK";
    uint256 public totalSupply;
    mapping(address => uint256) internal bal;
    mapping(address => mapping(address => uint256)) public allowance;
    mapping(address => bool) public blocked;
    bool public answersFalse;
    /// Receivers whose hook burns every unit of gas the transfer is given (a recipient hook, as ERC-777 has it).
    mapping(address => bool) public burns;
    /// The token's second entry point over the same ledger (a double-entry token).
    address public second;
    /// The largest gas a `transfer` was handed at its entry since the last reset (a transfer that reverted
    /// leaves no reading; a burning payee's push of the standard's own does not revert, so it leaves one).
    uint256 public maxPushGas;
    // round 4: the answers' shapes for the standard's own callers
    bytes32 public proxyCodeHash;
    address public logicA;
    address public verifierA;
    mapping(address => bool) public shapedFor;
    uint256 public lenT = 32;
    uint8 public valT;
    uint256 public lenF = 32;
    uint8 public valF;
    uint256 public lenB = 32;
    uint256 public lenD = 32;
    uint256 public pushes;
    uint256 public answered;

    event Transfer(address indexed from, address indexed to, uint256 value);

    function setStandard(bytes32 ph, address l, address v) external {
        proxyCodeHash = ph;
        logicA = l;
        verifierA = v;
    }

    function mark(address a, bool b) external {
        shapedFor[a] = b;
    }

    /// The four shapes: a length and a word (0 the true answer, 1 false, 2 garbage) for transfer and transferFrom.
    function setShapes(uint256 lt, uint8 vt, uint256 lf, uint8 vf, uint256 lb, uint256 ld) external {
        lenT = lt;
        valT = vt;
        lenF = lf;
        valF = vf;
        lenB = lb;
        lenD = ld;
    }

    function setBlocked(address who, bool b) external {
        blocked[who] = b;
    }

    function setAnswersFalse(bool f) external {
        answersFalse = f;
    }

    function setBurns(address who, bool b) external {
        burns[who] = b;
    }

    function setSecond(address a) external {
        second = a;
    }

    /// The meter's reset: the largest gas, the instance's pushes, and those that answered.
    function resetPushGas() external {
        maxPushGas = 0;
        pushes = 0;
        answered = 0;
    }

    /// The second entry point's transfer: the same ledger, the same rules.
    function secondMove(address from, address to, uint256 v) external returns (bool) {
        require(msg.sender == second, "second");
        return _move(from, to, v);
    }

    function mint(address to, uint256 v) external {
        totalSupply += v;
        bal[to] += v;
        emit Transfer(address(0), to, v);
    }

    function approve(address spender, uint256 v) external returns (bool) {
        allowance[msg.sender][spender] = v;
        return true;
    }

    function _shaped() internal view returns (bool) {
        return msg.sender.codehash == proxyCodeHash || msg.sender == logicA || msg.sender == verifierA || shapedFor[msg.sender];
    }

    /// Answers `len` bytes whose first word is `w` (cut where shorter), from a region past whatever the call's
    /// own path touched, so that the answer's memory costs the same on every path.
    function _answer(uint256 w, uint256 len) internal pure {
        assembly ("memory-safe") {
            mstore(0x3e0, 0)
            mstore(0x400, w)
            return(0x400, len)
        }
    }

    function _word(bool truth, uint8 val) internal pure returns (uint256) {
        if (val == 1) return 0;
        if (val == 2) return 0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef;
        return truth ? 1 : 0;
    }

    function decimals() external view returns (uint8) {
        if (_shaped()) _answer(18, lenD);
        return 18;
    }

    function balanceOf(address a) external view returns (uint256) {
        if (_shaped()) _answer(bal[a], lenB);
        return bal[a];
    }

    /// A push, metered first (the gas it is handed), so that a burning payee's push is metered too: a burning
    /// payee's hook spends everything the push hands it down to a few thousand gas and the token answers false,
    /// having moved nothing. `answered` counts the instance's pushes that answered in the shape set (the only
    /// ones whose cost depends on it). The no-copy reader's calls leave the meter alone, so that the price it
    /// reads holds the answer's own cost alone.
    function transfer(address to, uint256 v) external returns (bool) {
        uint256 g = gasleft();
        bool shaped = _shaped();
        bool inst = shaped && msg.sender.codehash == proxyCodeHash;
        if (inst || !shaped) {
            if (g > maxPushGas) maxPushGas = g;
        }
        if (inst) pushes++;
        if (burns[to]) {
            // an all-gas push (sweep's, withdraw's) burns at once; a push of the standard's own is spent by a loop,
            // so that its meter stands
            if (gasleft() > 1_000_000) {
                assembly ("memory-safe") {
                    invalid()
                }
            }
            while (gasleft() > 6000) {}
            return false;
        }
        bool r = _move(msg.sender, to, v);
        if (shaped) {
            if (inst) answered++;
            _answer(_word(r, valT), lenT);
        }
        return r;
    }

    function transferFrom(address from, address to, uint256 v) external returns (bool) {
        uint256 a = allowance[from][msg.sender];
        require(a >= v, "allowance");
        bool r;
        if (blocked[to]) {
            r = _move(from, to, v);
        } else {
            if (a != type(uint256).max) allowance[from][msg.sender] = a - v;
            r = _move(from, to, v);
        }
        if (_shaped()) _answer(_word(r, valF), lenF);
        return r;
    }

    function _move(address from, address to, uint256 v) internal returns (bool) {
        if (burns[to]) {
            assembly ("memory-safe") {
                invalid()
            }
        }
        if (blocked[to]) {
            if (answersFalse) return false;
            revert("blocked");
        }
        require(bal[from] >= v, "balance");
        bal[from] -= v;
        bal[to] += v;
        emit Transfer(from, to, v);
        return true;
    }
}


/// Round 4: a Chainlink-shaped feed whose round answers `roundLen` bytes (its five words first) and whose
/// decimals() answers `decLen` bytes, whatever the caller (only the verifier reads it).
contract InvShapedFeed {
    uint8 internal dec;
    int256 public answer;
    uint256 public updatedAt;
    uint256 public roundLen = 160;
    uint256 public decLen = 32;

    constructor(uint8 d, int256 a, uint256 t) {
        dec = d;
        answer = a;
        updatedAt = t;
    }

    function setLens(uint256 r, uint256 d) external {
        roundLen = r;
        decLen = d;
    }

    function decimals() external view returns (uint8) {
        uint256 d = dec;
        uint256 len = decLen;
        assembly ("memory-safe") {
            mstore(0xc0, d)
            return(0xc0, len)
        }
    }

    function latestRoundData() external view returns (uint80, int256, uint256, uint256, uint80) {
        int256 a = answer;
        uint256 t = updatedAt;
        uint256 len = roundLen;
        assembly ("memory-safe") {
            mstore(0xc0, 1)
            mstore(0xe0, a)
            mstore(0x100, t)
            mstore(0x120, t)
            mstore(0x140, 1)
            return(0xc0, len)
        }
    }
}

/// Round 4: calls a token the way the standard's push does (no return data copied) and reports the gas, so the
/// handler can price a shaped answer's own cost apart from any copy by the caller.
contract InvNoCopy {
    function gasOf(address target, bytes calldata data) external returns (uint256 used) {
        uint256 g0;
        uint256 g1;
        assembly ("memory-safe") {
            let p := mload(0x40)
            calldatacopy(p, data.offset, data.length)
            g0 := gas()
            pop(call(gas(), target, 0, p, data.length, 0, 0))
            g1 := gas()
        }
        used = g0 - g1;
    }
}

/// Round 5: a settlement token for the odd-decimals and flipping-feed orders: a plain ledger whose decimals()
/// answers a set word (any uint256, a "negative" int8 or a value past 36 included) in a set length, or one of two
/// words by the parity of the gas it is handed (a token whose answer flips between two reads of one init).
contract R5Token {
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;
    uint256 public d1;
    uint256 public d2;
    uint256 public decLen = 32;
    bool public flip;

    function setDec(uint256 a, uint256 b, uint256 len, bool f) external {
        d1 = a;
        d2 = b;
        decLen = len;
        flip = f;
    }

    function decimals() external view returns (uint256) {
        uint256 d = (flip && gasleft() % 2 == 1) ? d2 : d1;
        uint256 len = decLen;
        assembly ("memory-safe") {
            mstore(0, d)
            mstore(32, 0)
            return(0, len)
        }
    }

    function mint(address to, uint256 v) external {
        unchecked {
            balanceOf[to] += v;
        }
    }

    function approve(address s, uint256 v) external returns (bool) {
        allowance[msg.sender][s] = v;
        return true;
    }

    function transfer(address to, uint256 v) external returns (bool) {
        require(balanceOf[msg.sender] >= v, "balance");
        balanceOf[msg.sender] -= v;
        unchecked {
            balanceOf[to] += v;
        }
        return true;
    }

    function transferFrom(address f, address to, uint256 v) external returns (bool) {
        require(allowance[f][msg.sender] >= v, "allowance");
        allowance[f][msg.sender] -= v;
        require(balanceOf[f] >= v, "balance");
        balanceOf[f] -= v;
        unchecked {
            balanceOf[to] += v;
        }
        return true;
    }
}

/// Round 5: a Chainlink-shaped feed (and a decimals() answerer for a reference unit) whose answer and decimals()
/// are set words in a set length, each optionally flipping between two values by the parity of the gas the read
/// is handed: a feed whose answer differs between the two reads one init makes of it.
contract R5Feed {
    int256 public a1;
    int256 public a2;
    uint256 public at;
    uint256 public d1;
    uint256 public d2;
    uint256 public decLen = 32;
    uint8 public flip; // bit 0: the answer flips; bit 1: decimals() flips

    function set(int256 x1, int256 x2, uint256 t, uint256 y1, uint256 y2, uint256 len, uint8 f) external {
        a1 = x1;
        a2 = x2;
        at = t;
        d1 = y1;
        d2 = y2;
        decLen = len;
        flip = f;
    }

    function latestRoundData() external view returns (uint80, int256, uint256, uint256, uint80) {
        int256 a = (flip & 1 != 0 && gasleft() % 2 == 1) ? a2 : a1;
        return (1, a, at, at, 1);
    }

    function decimals() external view returns (uint256) {
        uint256 d = (flip & 2 != 0 && gasleft() % 2 == 1) ? d2 : d1;
        uint256 len = decLen;
        assembly ("memory-safe") {
            mstore(0, d)
            mstore(32, 0)
            return(0, len)
        }
    }
}

/// Round 5: a foreign token sent to an instance whose answers are hostile: its balance lies (more than it holds),
/// its transfer answers true and moves nothing, moves half, reverts, re-enters the instance (refund and sweep,
/// both locked) before it pays, answers 100,000 bytes, or burns every unit of gas it is handed.
contract R5Hostile {
    mapping(address => uint256) internal bal;
    uint8 public mode;
    address public target;
    uint256 public reentries;
    uint256 public reentriesRefused;

    function setMode(uint8 m, address t) external {
        mode = m;
        target = t;
    }

    function mint(address to, uint256 v) external {
        bal[to] += v;
    }

    function realBalance(address a) external view returns (uint256) {
        return bal[a];
    }

    function balanceOf(address a) external view returns (uint256) {
        if (mode == 1) return bal[a] + 1e18;
        return bal[a];
    }

    function transfer(address to, uint256 v) external returns (bool) {
        uint8 m = mode;
        if (m == 2) return true;
        if (m == 4) revert("hostile");
        if (m == 7) {
            while (true) {}
        }
        if (m == 5) {
            reentries++;
            (bool ok1, bytes memory r1) = target.call(abi.encodeWithSignature("refund()"));
            (bool ok2, bytes memory r2) = target.call(abi.encodeWithSignature("sweep(address,address)", to, address(this)));
            if (!ok1 && r1.length == 4 && bytes4(r1) == bytes4(keccak256("Reentered()")) && !ok2 && r2.length == 4
                    && bytes4(r2) == bytes4(keccak256("Reentered()"))) reentriesRefused++;
        }
        uint256 x = m == 3 ? v / 2 : v;
        if (bal[msg.sender] < x) return false;
        bal[msg.sender] -= x;
        bal[to] += x;
        if (m == 6) {
            assembly ("memory-safe") {
                return(0, 100000)
            }
        }
        return true;
    }
}

/// The settlement token's second address over the same ledger: `sweep` takes it for a foreign token, and
/// its transfer lowers the instance's balance of the settlement token (standard s8, `NotForeign`).
contract SecondEntry {
    BlockableToken internal immutable t;

    constructor(BlockableToken t_) {
        t = t_;
    }

    function decimals() external pure returns (uint8) {
        return 18;
    }

    function balanceOf(address a) external view returns (uint256) {
        return t.balanceOf(a);
    }

    function transfer(address to, uint256 v) external returns (bool) {
        return t.secondMove(msg.sender, to, v);
    }
}

/// A token that answers nothing: balanceOf and transfer revert.
contract MuteToken {
    function balanceOf(address) external pure returns (uint256) {
        revert("mute");
    }

    function transfer(address, uint256) external pure returns (bool) {
        revert("mute");
    }
}

/// A Chainlink-shaped feed, and a decimals() answerer for a reference unit, whose reads burn a set amount of gas
/// or all of it, pad the answer with zero words (a return bomb, paid for by the feed's own gas), or cut the round
/// answer short; a feed of the campaign's pricing orders (standard s3, READ_GAS).
contract InvGasFeed {
    int256 public answer;
    uint256 public at;
    uint256 public dec;
    uint256 public burnRound;
    uint256 public burnDec;
    uint256 public padRound;
    uint256 public padDec;
    uint256 public cut; // nonzero: latestRoundData answers this many bytes less one

    function set(int256 a, uint256 t, uint256 d) external {
        answer = a;
        at = t;
        dec = d;
    }

    function setGas(uint256 br, uint256 bd, uint256 pr, uint256 pd, uint256 c) external {
        burnRound = br;
        burnDec = bd;
        padRound = pr;
        padDec = pd;
        cut = c;
    }

    /// Burns `amount` gas, or all of it; the loop also ends where gasleft() stands still (metering paused).
    function _burn(uint256 amount) internal view {
        if (amount == 0) return;
        uint256 g = gasleft();
        uint256 stop = g > amount ? g - amount : 0;
        uint256 last = g;
        uint256 same = 0;
        while (gasleft() > stop) {
            uint256 now_ = gasleft();
            if (now_ == last && ++same > 64) break;
            last = now_;
        }
    }

    function decimals() external view returns (uint256) {
        _burn(burnDec);
        uint256 d = dec;
        uint256 size = (1 + padDec) * 32;
        assembly ("memory-safe") {
            let p := mload(0x40)
            mstore(p, d)
            return(p, size)
        }
    }

    function latestRoundData() external view returns (uint80, int256, uint256, uint256, uint80) {
        _burn(burnRound);
        int256 a = answer;
        uint256 t = at;
        uint256 size = (5 + padRound) * 32;
        if (cut != 0) size = cut - 1;
        assembly ("memory-safe") {
            let p := mload(0x40)
            mstore(p, 1)
            mstore(add(p, 32), a)
            mstore(add(p, 64), t)
            mstore(add(p, 96), t)
            mstore(add(p, 128), 1)
            return(p, size)
        }
    }
}
