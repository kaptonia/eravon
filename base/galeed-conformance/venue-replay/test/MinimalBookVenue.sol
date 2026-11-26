// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {MinimalDrawingVenue} from "./MinimalDrawingVenue.sol";
import {MinimalPanelBook} from "./MinimalPanelBook.sol";

/// The harness's book-branch reference venue (s4, s7 item 15 third branch):
/// a venue whose own `engage` reverts and which freezes and draws from the
/// panels recorded on the panel book its standard names, by the same rule as
/// MinimalDrawingVenue. It is a fixture and never a venue anyone deploys.
contract MinimalBookVenue is MinimalDrawingVenue {
    MinimalPanelBook public immutable panelBook;

    constructor(MinimalPanelBook panelBook_) MinimalDrawingVenue(panelBook_.successionBook()) {
        panelBook = panelBook_;
    }

    /// s7 item 15: the face lives on the book; the venue's own engage reverts.
    function engage(bytes32, address[] calldata, bytes32[] calldata, bytes32, uint64, address) external pure override {
        revert("panel book");
    }

    function _panelCount(address root, bytes32 terms) internal view override returns (uint256) {
        return panelBook.panelCount(root, terms);
    }

    function _panelAt(address root, bytes32 terms, uint256 i) internal view override returns (Panel memory p) {
        MinimalPanelBook.Panel memory q = panelBook.panelAt(root, terms, i);
        p = Panel(q.members, q.word, q.rosterHash, q.expires, q.router, q.routerHash, q.recorded);
    }
}
