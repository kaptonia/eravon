// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IPriceFeed} from "../../src/interfaces.sol";

/// A settlement token whose answers to the standard's own callers (an instance of the standard, the logic, the
/// verifier, and any address a test marks) are shaped: each of `transfer`, `transferFrom`, `balanceOf` and
/// `decimals` answers exactly `len` bytes whose first word is the true answer, false (the zero word) or a garbage
/// word, the bytes beyond it whatever the token's memory holds; a push may move the balance and answer false, or
/// answer nothing; every other caller (the test itself) is answered plainly. A meter records the gas each
/// `transfer` from a shaped caller is handed (the largest entry gas since the last reset) and counts them.
contract R4ShapedToken {
    uint8 internal constant TRUE_WORD = 0;
    uint8 internal constant FALSE_WORD = 1;
    uint8 internal constant GARBAGE_WORD = 2;

    struct Shape {
        uint256 len;
        uint8 val;
    }

    mapping(address => uint256) internal bal;
    mapping(address => mapping(address => uint256)) public allowance;
    bytes32 public proxyCodeHash;
    address public logic;
    address public verifier;
    mapping(address => bool) public shapedFor;
    Shape public sTransfer = Shape(32, TRUE_WORD);
    Shape public sFrom = Shape(32, TRUE_WORD);
    Shape public sBalance = Shape(32, TRUE_WORD);
    Shape public sDecimals = Shape(32, TRUE_WORD);
    /// A push that moves nothing (a deny list), answering per sTransfer's word.
    mapping(address => bool) public refuses;
    uint256 public maxEntryGas;
    uint256 public transfers;
    uint256 public totalSupply;
    /// A balanceOf that spends every unit of gas it is handed, to the standard's own callers.
    bool public burnBalance;

    function setStandard(bytes32 ph, address l, address v) external {
        proxyCodeHash = ph;
        logic = l;
        verifier = v;
    }

    function mark(address a, bool b) external {
        shapedFor[a] = b;
    }

    function shape(uint8 which, uint256 len, uint8 val) external {
        Shape memory s = Shape(len, val);
        if (which == 0) sTransfer = s;
        else if (which == 1) sFrom = s;
        else if (which == 2) sBalance = s;
        else sDecimals = s;
    }

    function setRefuses(address a, bool b) external {
        refuses[a] = b;
    }

    function setBurnBalance(bool b) external {
        burnBalance = b;
    }

    function resetMeter() external {
        maxEntryGas = 0;
        transfers = 0;
    }

    function plainBalance(address a) external view returns (uint256) {
        return bal[a];
    }

    function mint(address to, uint256 v) external {
        bal[to] += v;
        totalSupply += v;
    }

    function approve(address s, uint256 v) external returns (bool) {
        allowance[msg.sender][s] = v;
        return true;
    }

    function _shaped() internal view returns (bool) {
        return msg.sender.codehash == proxyCodeHash || msg.sender == logic || msg.sender == verifier || shapedFor[msg.sender];
    }

    /// Answers `len` bytes from the free memory pointer, the first word `w` (cut to `len` where shorter).
    function _answer(uint256 w, uint256 len) internal pure {
        assembly ("memory-safe") {
            let p := mload(0x40)
            mstore(p, w)
            return(p, len)
        }
    }

    function _word(uint256 truth, uint8 val) internal pure returns (uint256) {
        if (val == FALSE_WORD) return 0;
        if (val == GARBAGE_WORD) return 0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef;
        return truth;
    }

    function decimals() external view returns (uint8) {
        if (!_shaped()) return 18;
        Shape memory s = sDecimals;
        _answer(_word(18, s.val), s.len);
    }

    function balanceOf(address a) external view returns (uint256) {
        uint256 v = bal[a];
        if (!_shaped()) return v;
        if (burnBalance) {
            assembly ("memory-safe") {
                invalid()
            }
        }
        Shape memory s = sBalance;
        _answer(_word(v, s.val), s.len);
    }

    function transfer(address to, uint256 v) external returns (bool) {
        bool shaped = _shaped();
        if (shaped) {
            uint256 g = gasleft();
            if (g > maxEntryGas) maxEntryGas = g;
            transfers++;
        }
        bool moved = false;
        if (!refuses[to]) {
            require(bal[msg.sender] >= v, "balance");
            bal[msg.sender] -= v;
            bal[to] += v;
            moved = true;
        }
        if (!shaped) return moved;
        Shape memory s = sTransfer;
        _answer(_word(moved ? 1 : 0, s.val), s.len);
    }

    function transferFrom(address from, address to, uint256 v) external returns (bool) {
        uint256 a = allowance[from][msg.sender];
        require(a >= v, "allowance");
        require(bal[from] >= v, "balance");
        allowance[from][msg.sender] = a - v;
        bal[from] -= v;
        bal[to] += v;
        if (!_shaped()) return true;
        Shape memory s = sFrom;
        _answer(_word(1, s.val), s.len);
    }
}

/// A Chainlink-shaped feed whose round answers `roundLen` bytes (its five words, then whatever memory holds;
/// cut where shorter) and whose decimals() answers `decLen` bytes, each after burning `burn` gas, or all of it.
contract R4ShapedFeed {
    int256 public answer;
    uint256 public at;
    uint256 public dec;
    uint256 public roundLen = 160;
    uint256 public decLen = 32;
    uint256 public burnRound;
    uint256 public burnDec;
    bool public roundReverts;

    function set(int256 a, uint256 t, uint256 d) external {
        answer = a;
        at = t;
        dec = d;
    }

    function setLens(uint256 r, uint256 d) external {
        roundLen = r;
        decLen = d;
    }

    function setBurns(uint256 r, uint256 d) external {
        burnRound = r;
        burnDec = d;
    }

    function setReverts(bool r) external {
        roundReverts = r;
    }

    function _burn(uint256 amount) internal view {
        if (amount == 0) return;
        if (amount == type(uint256).max) {
            assembly ("memory-safe") {
                invalid()
            }
        }
        uint256 g = gasleft();
        uint256 stop = g > amount ? g - amount : 0;
        while (gasleft() > stop) {}
    }

    function decimals() external view returns (uint8) {
        _burn(burnDec);
        uint256 d = dec;
        uint256 len = decLen;
        assembly ("memory-safe") {
            let p := mload(0x40)
            mstore(p, d)
            return(p, len)
        }
    }

    function latestRoundData() external view returns (uint80, int256, uint256, uint256, uint80) {
        _burn(burnRound);
        if (roundReverts) revert("no round");
        int256 a = answer;
        uint256 t = at;
        uint256 len = roundLen;
        assembly ("memory-safe") {
            let p := mload(0x40)
            mstore(p, 1)
            mstore(add(p, 32), a)
            mstore(add(p, 64), t)
            mstore(add(p, 96), t)
            mstore(add(p, 128), 1)
            return(p, len)
        }
    }
}

/// Reads a token or a feed the way the standard's fixed-size reads do (no return data copied), so that a test
/// can price the callee's own cost of an answer's length apart from any copy by its caller.
contract R4NoCopy {
    function gasOf(address target, bytes calldata data, bool isStatic) external returns (uint256 used, uint256 size) {
        uint256 g0;
        uint256 g1;
        assembly ("memory-safe") {
            let p := mload(0x40)
            calldatacopy(p, data.offset, data.length)
            g0 := gas()
            switch isStatic
            case 1 {
                pop(staticcall(gas(), target, p, data.length, 0, 0))
            }
            default {
                pop(call(gas(), target, 0, p, data.length, 0, 0))
            }
            g1 := gas()
            size := returndatasize()
        }
        used = g0 - g1;
    }
}

interface IR4Inst {
    function status()
        external
        view
        returns (uint8, uint256, uint256, uint256, uint256, uint256, uint64, uint64, bool, uint64, uint64, uint256, bytes32, uint256, address, address);
    function refund() external;
}

/// Round 3's R3SideSwap: the buyer's side contract for a trade completed off the instance; it pays the seller the
/// agreed price from the buyer's payout only once the instance is Refunded, and once.
contract R4SideSwap {
    IR4Inst public immutable inst;
    address public immutable token;
    address public immutable from;
    address public immutable to;
    uint256 public immutable price;
    bool public paid;

    constructor(IR4Inst i, address t, address f, address r, uint256 p) {
        inst = i;
        token = t;
        from = f;
        to = r;
        price = p;
    }

    function collect() external {
        (uint8 ph,,,,,,,,,,,,,,,) = inst.status();
        require(ph == 4, "not refunded");
        require(!paid, "paid");
        paid = true;
        (bool ok, bytes memory ret) =
            token.call(abi.encodeWithSignature("transferFrom(address,address,uint256)", from, to, price));
        require(ok && (ret.length == 0 || abi.decode(ret, (bool))), "pay");
    }
}

/// Round 3's R3SellerWallet: a seller that is a contract, refunding and collecting in one transaction.
contract R4SellerWallet {
    function exec(address target, bytes calldata data) external returns (bytes memory) {
        (bool ok, bytes memory ret) = target.call(data);
        if (!ok) {
            assembly ("memory-safe") {
                revert(add(ret, 32), mload(ret))
            }
        }
        return ret;
    }

    function refundAndCollect(IR4Inst inst, R4SideSwap swap) external {
        inst.refund();
        swap.collect();
    }
}
