// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {MockERC20} from "../replay/MockERC20.sol";
import {Eravon} from "../../src/Eravon.sol";

/// A Chainlink-shaped feed the tests set by hand.
contract MockFeed {
    uint8 public decimals;
    int256 public answer;
    uint256 public updatedAt;

    constructor(uint8 d, int256 a, uint256 t) {
        decimals = d;
        answer = a;
        updatedAt = t;
    }

    function set(int256 a, uint256 t) external {
        answer = a;
        updatedAt = t;
    }

    function latestRoundData() external view returns (uint80, int256, uint256, uint256, uint80) {
        return (1, answer, updatedAt, updatedAt, 1);
    }
}

/// A six-decimal stablecoin standing in as the judge's reference unit.
contract MockUSD is MockERC20 {
    constructor() {
        decimals = 6;
    }
}

/// A settlement token with every hook the attack list names. `mode` picks the
/// misbehaviour; `arm` makes the next move re-enter a target with a list of
/// probes and records what each one answered.
contract HostileToken {
    string public name = "Hostile";
    string public symbol = "HOS";
    uint8 public decimals = 18;
    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    event Transfer(address indexed from, address indexed to, uint256 value);

    enum Mode {
        Plain,
        FeeOnTransfer,
        ReturnsFalse,
        ReturnsNothing,
        RevertsOnZero,
        DirtyBool,
        BlocksOne
    }

    Mode public mode;
    uint16 public feeBps = 100;
    address public blocked;

    // the re-entrancy rig
    address public target;
    bytes[] internal probes;
    bytes4[] public results;
    bool public armed;

    function setMode(Mode m) external {
        mode = m;
    }

    function block_(address who) external {
        blocked = who;
    }

    function mint(address to, uint256 v) external {
        totalSupply += v;
        balanceOf[to] += v;
        emit Transfer(address(0), to, v);
    }

    function approve(address spender, uint256 v) external returns (bool) {
        allowance[msg.sender][spender] = v;
        return true;
    }

    function arm(address t, bytes[] calldata p) external {
        target = t;
        delete probes;
        for (uint256 i = 0; i < p.length; i++) {
            probes.push(p[i]);
        }
        delete results;
        armed = true;
    }

    function disarm() external {
        armed = false;
    }

    function resultsLength() external view returns (uint256) {
        return results.length;
    }

    function transfer(address to, uint256 v) external returns (bool) {
        return _run(msg.sender, to, v);
    }

    function transferFrom(address from, address to, uint256 v) external returns (bool) {
        uint256 a = allowance[from][msg.sender];
        require(a >= v, "allowance");
        if (a != type(uint256).max) allowance[from][msg.sender] = a - v;
        return _run(from, to, v);
    }

    function _run(address from, address to, uint256 v) internal returns (bool) {
        if (mode == Mode.RevertsOnZero && v == 0) revert("zero transfer");
        if (mode == Mode.BlocksOne && to == blocked) revert("blocked");
        if (mode == Mode.ReturnsFalse) return false;
        _move(from, to, v);
        _hook();
        if (mode == Mode.ReturnsNothing) {
            assembly {
                return(0, 0)
            }
        }
        if (mode == Mode.DirtyBool) {
            assembly {
                mstore(0, 2)
                return(0, 32)
            }
        }
        return true;
    }

    function _move(address from, address to, uint256 v) internal {
        require(balanceOf[from] >= v, "balance");
        balanceOf[from] -= v;
        uint256 got = v;
        if (mode == Mode.FeeOnTransfer) {
            uint256 fee = (v * feeBps) / 10000;
            got = v - fee;
            totalSupply -= fee;
        }
        balanceOf[to] += got;
        emit Transfer(from, to, got);
    }

    /// Re-enters the target once per move, catching each probe's revert so
    /// that the outer transfer still completes.
    function _hook() internal {
        if (!armed || target == address(0)) return;
        armed = false;
        for (uint256 i = 0; i < probes.length; i++) {
            (bool ok, bytes memory ret) = target.call(probes[i]);
            results.push(ok ? bytes4(0) : _sel(ret));
        }
        armed = true;
    }

    function _sel(bytes memory r) internal pure returns (bytes4 s) {
        if (r.length < 4) return bytes4(0xffffffff);
        assembly {
            s := mload(add(r, 0x20))
        }
    }
}

/// A party that is a contract: it consents by calling (init, settle, consent,
/// cancel, revoke) and performs the party acts by calling the instance.
contract Company {
    function exec(address target, bytes calldata data) external returns (bytes memory) {
        (bool ok, bytes memory ret) = target.call(data);
        if (!ok) {
            assembly {
                revert(add(ret, 32), mload(ret))
            }
        }
        return ret;
    }
}

/// A contract that is no proxy of the logic and tries the logic's pull.
contract Impostor {
    function tryPull(Eravon logic, address seller, address token, uint256 amount) external {
        logic.pull(seller, seller, token, amount);
    }
}

