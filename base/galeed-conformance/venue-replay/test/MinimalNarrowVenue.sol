// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {MinimalVenue} from "./MinimalVenue.sol";

/// The harness's narrowing reference venue: MinimalVenue whose dispute is a
/// claim that narrows the range to rungs 0 and 1 (s2.3, s2.5). It is a
/// fixture and never a venue anyone deploys.
contract MinimalNarrowVenue is MinimalVenue {
    constructor(address book_) MinimalVenue(book_) {}

    function _onDispute(Seat storage s) internal override {
        s.rungMax = 1;
    }
}
