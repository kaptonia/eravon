// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IGaleedRouter, RouteView} from "./IGaleedVenue.sol";

/// Routers the replay engages panels with (galeed.venue/1 section 8 step 14).
/// They are fixtures of the harness and never routers anyone deploys.

/// Returns one fixed index, whatever the view.
contract FixedRouter is IGaleedRouter {
    uint256 internal immutable at;
    constructor(uint256 at_) { at = at_; }
    function route(RouteView calldata) external view returns (uint256) { return at; }
}

/// Returns the first member neither seated nor offered on the case.
contract FirstEligibleRouter is IGaleedRouter {
    function route(RouteView calldata v) external pure returns (uint256) {
        return firstEligible(v);
    }
}

/// Reverts.
contract RevertRouter is IGaleedRouter {
    function route(RouteView calldata) external pure returns (uint256) { revert("no route"); }
}

/// Spends every unit of gas it is given, where it is given no more than `cap`
/// (the venue's routeGas()); handed more it fails at once, so that a venue that
/// hands a router more than routeGas() meets a failing router here and the
/// replay's budget is never what the router spends.
contract HogRouter is IGaleedRouter {
    uint256 internal immutable cap;
    constructor(uint256 cap_) { cap = cap_; }
    function route(RouteView calldata) external view returns (uint256 i) {
        if (gasleft() > cap) revert("more than routeGas");
        while (gasleft() > 0) i++;
    }
}

/// The first member neither seated nor offered on the case, or reverts.
function firstEligible(RouteView calldata v) pure returns (uint256) {
    for (uint256 i = 0; i < v.members.length; i++) {
        bool taken = false;
        for (uint256 k = 0; k < v.seated.length; k++) if (v.seated[k] == v.members[i]) taken = true;
        if (!taken) return i;
    }
    revert("no member");
}

/// Answers the first eligible member only where the view is the venue's own:
/// the caller is the venue the view names, on this chain, at this block's
/// time, with the panel's members, and as many judges seated as the seat's
/// ordinal; otherwise an index outside members.
contract EchoRouter is IGaleedRouter {
    bytes32 internal immutable membersHash;
    constructor(address[] memory members) { membersHash = keccak256(abi.encode(members)); }
    function route(RouteView calldata v) external view returns (uint256) {
        bool own = v.venue == msg.sender && v.chainId == block.chainid && v.blockTime == block.timestamp
            && keccak256(abi.encode(v.members)) == membersHash && v.seated.length == v.seat;
        return own ? firstEligible(v) : type(uint256).max;
    }
}

/// Answers by the gas it receives: the first eligible member where its entry
/// gas lies within `margin` below `gas_`, the panel's last member otherwise.
contract GasProbeRouter is IGaleedRouter {
    uint256 internal immutable hi;
    uint256 internal immutable lo;
    constructor(uint256 gas_, uint256 margin) { hi = gas_; lo = gas_ - margin; }
    function route(RouteView calldata v) external view returns (uint256) {
        uint256 g = gasleft();
        if (g >= lo && g <= hi) return firstEligible(v);
        return v.members.length - 1;
    }
}

/// Answers the first eligible member where its entry gas lies within `margin`
/// below `gas_`, and reverts at once otherwise: handed less than routeGas() it
/// fails cheaply and leaves the venue its gas, so that a venue that calls it
/// short can finish the path a draw that finds no member takes.
contract GasGateRouter is IGaleedRouter {
    uint256 internal immutable hi;
    uint256 internal immutable lo;
    constructor(uint256 gas_, uint256 margin) { hi = gas_; lo = gas_ - margin; }
    function route(RouteView calldata v) external view returns (uint256) {
        uint256 g = gasleft();
        if (g >= lo && g <= hi) return firstEligible(v);
        revert("off routeGas");
    }
}

/// Answers the first eligible member for a case's first seat and reverts for every later one.
contract SecondFailsRouter is IGaleedRouter {
    function route(RouteView calldata v) external pure returns (uint256) {
        if (v.seat != 0) revert("second draw");
        return firstEligible(v);
    }
}

/// Answers the first eligible member for a case's first seat and an index
/// outside members for every later one.
contract SecondOutOfRangeRouter is IGaleedRouter {
    function route(RouteView calldata v) external pure returns (uint256) {
        if (v.seat != 0) return v.members.length;
        return firstEligible(v);
    }
}

/// Answers the first eligible member below seat ordinal `n` and reverts from it on.
contract NthFailsRouter is IGaleedRouter {
    uint64 internal immutable n;
    constructor(uint64 n_) { n = n_; }
    function route(RouteView calldata v) external view returns (uint256) {
        if (v.seat >= n) revert("nth draw");
        return firstEligible(v);
    }
}

/// Answers the first eligible member below seat ordinal `n` and an index outside members from it on.
contract NthOutOfRangeRouter is IGaleedRouter {
    uint64 internal immutable n;
    constructor(uint64 n_) { n = n_; }
    function route(RouteView calldata v) external view returns (uint256) {
        if (v.seat >= n) return v.members.length;
        return firstEligible(v);
    }
}

/// Names members[seat % 2]: the third draw names the member the first seated.
contract ParityRouter is IGaleedRouter {
    function route(RouteView calldata v) external pure returns (uint256) {
        return v.seat % 2;
    }
}

/// Returns two words.
contract LongReturnRouter {
    fallback() external {
        assembly { mstore(0, 0) mstore(32, 0) return(0, 64) }
    }
}

/// Returns 31 bytes.
contract ShortReturnRouter {
    fallback() external {
        assembly { mstore(0, 0) return(0, 31) }
    }
}

/// The first eligible member, under a code hash of its own: the replay
/// replaces a FirstEligibleRouter's code with it after a case's creation.
contract FirstEligibleRouterB is IGaleedRouter {
    uint256 public constant B = 2;
    function route(RouteView calldata v) external pure returns (uint256) {
        return firstEligible(v);
    }
}

/// Writes storage, then answers the first eligible member: reached by
/// staticcall the write reverts and the case has no usable panel.
contract WriterRouter {
    uint256 public calls;
    function route(RouteView calldata v) external returns (uint256) {
        calls += 1;
        return firstEligible(v);
    }
}

/// Answers the index the hash of the view's case, terms, token, amount,
/// rungs and parties names; the replay recomputes it from seatOf.
contract FieldsRouter is IGaleedRouter {
    function route(RouteView calldata v) external pure returns (uint256) {
        return uint256(keccak256(abi.encode(v.caseId, v.termsHash, v.token, v.amount, v.rungs, v.parties))) % v.members.length;
    }
}

/// From the place the hash of the view's case, terms, token, amount, rungs
/// and parties names, the first member neither seated nor offered; at a
/// redraw the replay predicts the member from seatOf, so the seat it opens
/// shows whether the view carried the venue's fields.
contract FieldsRouterB is IGaleedRouter {
    function route(RouteView calldata v) external pure returns (uint256) {
        uint256 n = v.members.length;
        uint256 h = uint256(keccak256(abi.encode(v.caseId, v.termsHash, v.token, v.amount, v.rungs, v.parties))) % n;
        for (uint256 k = 0; k < n; k++) {
            uint256 i = (h + k) % n;
            bool taken = false;
            for (uint256 j = 0; j < v.seated.length; j++) if (v.seated[j] == v.members[i]) taken = true;
            if (!taken) return i;
        }
        revert("no member");
    }
}
