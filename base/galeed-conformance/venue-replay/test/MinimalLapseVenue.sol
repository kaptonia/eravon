// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {MinimalDrawingVenue} from "./MinimalDrawingVenue.sol";

/// The harness's lapse-redraw reference venue: MinimalDrawingVenue drawing
/// again from the panel the case froze after a lapse, the escrow passing to
/// the new seat, and refunding the buyer after an expired offer and after a
/// lapse whose draw finds no member. It is a fixture and never a venue anyone
/// deploys.
contract MinimalLapseVenue is MinimalDrawingVenue {
    constructor(address book_) MinimalDrawingVenue(book_) {}

    function _wouldRedraw(bytes32) internal pure override returns (bool) { return false; }
    function _redrawAfterOffer(bytes32) internal pure override returns (bool) { return false; }

    function _wouldRedrawLapse(bytes32 caseId) internal view override returns (bool drew) {
        (drew, , , ) = _redrawOf(caseId);
    }

    function _redrawAfterLapse(bytes32 caseId) internal override returns (bool) {
        return _seatAgain(caseId);
    }
}
