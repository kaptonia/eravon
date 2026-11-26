// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IGaleedSuccessionBook} from "../src/IGaleedVenue.sol";

/// The harness's reference panel book (s4, s7 item 15 third branch): a
/// contract apart from any case that carries the panel face for the venues
/// of a standard. It takes a call only from its record's current key in the
/// succession book it names, applies the six rejections, and records every
/// accepted panel append-only under (root, termsHash) with its router, the
/// router's code hash, and the block; it names the same succession book and
/// the same route gas as the venues built to its standard. It is a fixture
/// and never a book anyone deploys.
contract MinimalPanelBook {
    struct Panel {
        address[] members;
        bytes32 word;
        bytes32 rosterHash;
        uint64 expires;
        address router;
        bytes32 routerHash;
        uint64 recorded;
    }

    address internal immutable book;
    uint64 internal constant ROUTE_GAS = 500_000;

    mapping(address => mapping(bytes32 => Panel[])) internal panels;

    event Engaged(address indexed root, bytes32 indexed termsHash, uint256 index);

    constructor(address book_) {
        book = book_;
    }

    function successionBook() external view returns (address) { return book; }
    function routeGas() external pure returns (uint64) { return ROUTE_GAS; }

    function engage(bytes32 termsHash, address[] calldata members, bytes32[] calldata amanahHashes, bytes32 rosterHash, uint64 expires, address router) external {
        require(IGaleedSuccessionBook(book).current(msg.sender) == msg.sender, "not the current key");
        require(members.length == amanahHashes.length, "lengths");
        require(members.length != 0, "empty");
        for (uint256 i = 0; i < amanahHashes.length; i++) {
            require(amanahHashes[i] != bytes32(0), "zero word");
            require(amanahHashes[i] == amanahHashes[0], "words differ");
        }
        require(expires > block.timestamp, "expires");
        require(router == address(0) || router.code.length != 0, "router without code");
        address root = IGaleedSuccessionBook(book).root(msg.sender);
        Panel storage p = panels[root][termsHash].push();
        for (uint256 i = 0; i < members.length; i++) p.members.push(members[i]);
        p.word = amanahHashes[0];
        p.rosterHash = rosterHash;
        p.expires = expires;
        p.router = router;
        p.routerHash = router == address(0) ? bytes32(0) : router.codehash;
        p.recorded = uint64(block.number);
        emit Engaged(root, termsHash, panels[root][termsHash].length - 1);
    }

    /// The views the standard names: how many panels a root recorded under
    /// the terms, and each of them.
    function panelCount(address root, bytes32 termsHash) external view returns (uint256) {
        return panels[root][termsHash].length;
    }

    function panelAt(address root, bytes32 termsHash, uint256 i) external view returns (Panel memory) {
        return panels[root][termsHash][i];
    }
}
