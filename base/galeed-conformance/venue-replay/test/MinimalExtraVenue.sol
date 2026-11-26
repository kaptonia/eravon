// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {MinimalVenue} from "./MinimalVenue.sol";

/// The harness's extra reference venue: MinimalVenue with no ladder
/// (`rungs` zero) and the ruling carried in `extra` as `(uint16 buyerBps)`,
/// the buyer's part of the amount in basis points, at most 10000 (s2.5,
/// s2.9). It is a fixture and never a venue anyone deploys.
contract MinimalExtraVenue is MinimalVenue {
    constructor(address book_) MinimalVenue(book_) {}

    function extraType() external pure override returns (string memory) { return "(uint16 buyerBps)"; }

    function _ladder() internal pure override returns (uint64, uint64, uint64) { return (0, 0, 0); }

    /// s2.9: exactly the encoding of one uint16, and a part of at most the whole.
    function _ruling(Case storage c, uint64, bytes calldata extra) internal view override returns (Outcome memory o) {
        require(extra.length == 32, "extra");
        uint256 bps = abi.decode(extra, (uint256));
        require(bps <= 10_000, "share");
        o = _outcome(2);
        o.shares[0] = c.amount * bps / 10_000;
        o.shares[1] = c.amount - o.shares[0];
        (o.standbyFee, o.rulingFee) = _fees(c);
        o.rulingFeeFrom[0] = o.rulingFee;
    }
}
