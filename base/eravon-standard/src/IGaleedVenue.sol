// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/// galeed.venue/1 (docs/galeed-venue-v1.md §1), verbatim.
interface IGaleedVenue {
    // ---- case face (the judge) ----
    event Seated(address indexed judge, bytes32 indexed caseId, uint64 seat);
    event Committed(bytes32 indexed caseId, address indexed by, bytes32 manifest);
    event Anchored(address indexed sender, bytes32 indexed hash, bytes entry);

    struct Seat {
        address   judge;
        bytes32   amanahHash;
        bytes32   termsHash;
        bytes32   quoteHash;
        uint64    windowStart;
        uint64    windowEnd;
        uint64    accepted;       // unix second the judge accepted the seat; 0 while the offer stands open
        uint8     state;          // 0 seated, 1 disputed, 2 ruled, 3 resigned, 4 lapsed, 5 closed
        uint64    rungs;
        uint64    rungMin;        // the rungs accepted from this seat now: rungMin..rungMax inclusive
        uint64    rungMax;
        address   token;
        uint256   amount;
        uint256   standbyCredited; // credited to the judge on this seating so far, in token, as standby fee
        uint256   rulingCredited;  // credited to the judge on this seating so far, in token, as ruling fee
        address[] parties;
    }

    struct Outcome {
        uint8     state;          // the seat's state once the action is taken
        uint256[] shares;         // per role of partyRoles(): the part of amount the role receives
        uint256[] rulingFeeFrom;  // per role: the part of the ruling fee the role bears
        uint256   standbyFee;     // what the action credits to the judge as standby fee
        uint256   rulingFee;      // what the action credits to the judge as ruling fee
    }

    function seatOf(bytes32 caseId, uint64 seat) external view returns (Seat memory);
    function quoteType() external view returns (string memory);
    function extraType() external view returns (string memory);
    function partyRoles() external view returns (string memory);
    function outcomeOf(bytes32 caseId, uint64 seat, uint64 rung, bytes calldata extra) external view returns (Outcome memory);
    function advanceOf(bytes32 caseId, uint64 seat) external view returns (Outcome memory);
    function accept(bytes32 caseId, uint64 seat) external;
    function commit(bytes32 caseId, bytes32 manifest) external;
    function rule(bytes32 caseId, uint64 seat, bytes32 entryId, uint64 rung, bytes calldata entry, bytes calldata extra) external;
    function resign(bytes32 caseId, uint64 seat, bytes32 entryId, bytes calldata entry) external;
    function advance(bytes32 caseId, uint64 seat) external;
    function successionBook() external view returns (address);

    // ---- economic face (any seat: judge, verifier) ----
    function setPayout(address key, address payout) external;
    function withdraw(address token, address key, address payout) external;
    function accountOf(address who, address token) external view returns (address payout, uint256 claimable);

    // ---- panel face (the guild) ----
    function engage(bytes32 termsHash, address[] calldata members, bytes32[] calldata amanahHashes, bytes32 rosterHash, uint64 expires, address router) external;
    function routeGas() external view returns (uint64);
}

interface IGaleedSuccessionBook {
    function current(address a) external view returns (address);
    function root(address a) external view returns (address);
}

struct RouteView {
    bytes32   caseId;
    uint64    seat;
    bytes32   termsHash;
    address   token;
    uint256   amount;
    uint64    rungs;
    address[] parties;
    address[] members;
    address[] seated;
    bytes32   seed;
    uint64    blockTime;
    uint256   chainId;
    address   venue;
}

interface IGaleedRouter {
    function route(RouteView calldata v) external view returns (uint256 index);
}
