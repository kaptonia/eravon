// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {VenueReplay} from "../src/VenueReplay.sol";
import {IGaleedVenue} from "../src/IGaleedVenue.sol";
import {MinimalExtraVenue} from "./MinimalExtraVenue.sol";

/// The replay completed for the harness's extra reference venue: no ladder,
/// the ruling a split carried in `extra`.
contract MinimalExtraVenueReplay is VenueReplay {
    address internal constant BUYER = address(0x5000000000000000000000000000000000000001);
    address internal constant SELLER = address(0x5000000000000000000000000000000000000002);
    MinimalExtraVenue internal mv;

    function deployVenue() internal override returns (IGaleedVenue) {
        mv = new MinimalExtraVenue(address(succession));
        token.mint(BUYER, 1_000_000 ether);
        vm.prank(BUYER);
        token.approve(address(mv), type(uint256).max);
        return IGaleedVenue(address(mv));
    }

    function venueDraws() internal pure override returns (bool) { return false; }
    function seatsBeforeAcceptance() internal pure override returns (bool) { return true; }

    function openCase() internal override returns (bytes32 caseId, uint64 seat) {
        vm.prank(BUYER, BUYER);
        caseId = mv.createCase(JUDGE, AMANAH, keccak256("terms of sale"), address(token), 100 ether, 5 ether, SELLER, abi.encode(false));
        seat = 0;
    }

    /// The same venue with the judge's standing consent copied into the terms (s2.5, s2.7).
    function openCarriedCase() internal override returns (bytes32 caseId, uint64 seat) {
        vm.prank(BUYER, BUYER);
        caseId = mv.createCase(JUDGE, AMANAH, keccak256("terms of sale"), address(token), 100 ether, 5 ether, SELLER, abi.encode(true));
        seat = 0;
    }

    function dispute(bytes32 caseId, uint64 seat) internal override {
        if (mv.seatOf(caseId, seat).state == 1) return;
        vm.prank(BUYER, BUYER);
        mv.raise(caseId, seat);
    }

    function party(bytes32) internal pure override returns (address) { return BUYER; }

    function rulingExtra(bytes32, uint64) internal pure override returns (bytes memory) { return abi.encode(uint16(2500)); }

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
