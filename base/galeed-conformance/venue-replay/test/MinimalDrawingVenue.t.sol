// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {VenueReplay} from "../src/VenueReplay.sol";
import {IGaleedVenue} from "../src/IGaleedVenue.sol";
import {MinimalDrawingVenue} from "./MinimalDrawingVenue.sol";

/// The replay completed for the harness's drawing reference venue: the
/// panel face on the venue itself, the six rejections of s4, a stranger's
/// panel under the same terms recorded apart, a seat drawn from the panel the
/// case froze, by the venue's own rule or by the panel's router, and a draw
/// again after an offer expires.
contract MinimalDrawingVenueReplay is VenueReplay {
    address internal constant BUYER = address(0x5000000000000000000000000000000000000001);
    address internal constant SELLER = address(0x5000000000000000000000000000000000000002);
    MinimalDrawingVenue internal mv;

    function deployVenue() internal override returns (IGaleedVenue) {
        mv = new MinimalDrawingVenue(address(succession));
        token.mint(BUYER, 1_000_000 ether);
        vm.prank(BUYER);
        token.approve(address(mv), type(uint256).max);
        return IGaleedVenue(address(mv));
    }

    function venueDraws() internal pure override returns (bool) { return true; }
    function seatsBeforeAcceptance() internal pure override returns (bool) { return true; }
    function reassignsAfterExpiredOffer() internal pure override returns (bool) { return true; }

    function openCase() internal override returns (bytes32 caseId, uint64 seat) {
        vm.prank(BUYER, BUYER);
        caseId = mv.createDrawnCase(GUILD, panelTerms(), address(token), 100 ether, 5 ether, SELLER, abi.encode(false));
        seat = 0;
    }

    function opensCaseNaming() internal pure override returns (bool) { return true; }

    function openCaseNaming(address guild) internal override returns (bytes32 caseId, uint64 seat) {
        vm.prank(BUYER, BUYER);
        caseId = mv.createDrawnCase(guild, panelTerms(), address(token), 100 ether, 5 ether, SELLER, abi.encode(false));
        seat = 0;
    }

    function openCarriedCase() internal override returns (bytes32 caseId, uint64 seat) {
        vm.prank(BUYER, BUYER);
        caseId = mv.createDrawnCase(GUILD, panelTerms(), address(token), 100 ether, 5 ether, SELLER, abi.encode(true));
        seat = 0;
    }

    function dispute(bytes32 caseId, uint64 seat) internal override {
        if (mv.seatOf(caseId, seat).state == 1) return;
        vm.prank(BUYER, BUYER);
        mv.raise(caseId, seat);
    }

    function party(bytes32) internal pure override returns (address) { return BUYER; }

    /// `partyRoles()` is `buyer,seller`: parties[0] and parties[1] of every seat of the case.
    function roleAddresses(bytes32 c, uint256 r) internal view override returns (address[] memory xs) {
        xs = new address[](1);
        xs[0] = mv.seatOf(c, 0).parties[r];
    }

    /// The quote `openCase()` copies: `(bool autoAccept)` false.
    function quoteOf(bytes32) internal pure override returns (bool, bytes memory) { return (true, abi.encode(false)); }

    /// What the venue holds for a party, the party takes by `claim`.
    function collect(bytes32, address who) internal override {
        vm.prank(who, who);
        mv.claim(address(token));
    }
}
