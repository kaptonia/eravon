// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {IGaleedSuccessionBook} from "./IGaleedVenue.sol";

/// The panel book: the panel face of galeed.venue/1 s4 for every ERAVON
/// instance of a chain (eravon.standard/1 s7). One per chain, deployed by
/// whoever first needs it through the public CREATE2 deployer under the salt
/// keccak256("eravon/1/panel"), designated by code. A guild sends `engage`
/// from its current key on the succession book; every accepted panel is one
/// more record, append-only, under (root, termsHash), `root` being the
/// sender's root on the book. No owner, no upgrade, no balance.
contract EravonPanelBook {
    /// The family's succession book for chains 1 and 11155111 (galeed.roster/1 s4, s9), as every instance names it.
    address internal constant SUCCESSION_BOOK = 0x3dB7c505dd25361F1A66f6c5Bd96b48df27f668c;
    /// The gas an instance hands a router, as every instance answers it (venue/1 s4).
    uint64 internal constant ROUTE_GAS = 1_000_000;

    struct Panel {
        bytes32 panelHash;   // keccak256(abi.encode(members, amanahHashes))
        bytes32 rosterHash;  // the roster document's word (galeed.roster/1 s1), for audit
        uint64 expires;      // unix second after which the panel may no longer be frozen into a case
        uint64 recorded;     // the block the record was made in
        address router;      // zero, or the router the guild names for its draws
        bytes32 routerHash;  // the router's code hash at the call; zero where router is zero
    }

    mapping(address => mapping(bytes32 => Panel[])) private _panels;

    event Engaged(
        address indexed root,
        bytes32 indexed termsHash,
        uint256 index,
        address sender,
        address[] members,
        bytes32[] amanahHashes,
        bytes32 rosterHash,
        uint64 expires,
        address router,
        bytes32 routerHash
    );

    error NotCurrent();
    error LengthMismatch();
    error EmptyMembers();
    error ZeroWord();
    error WordsDiffer();
    error ExpiresNotInFuture();
    error RouterNoCode();
    error NoSuchIndex();

    function successionBook() external pure returns (address) {
        return SUCCESSION_BOOK;
    }

    function routeGas() external pure returns (uint64) {
        return ROUTE_GAS;
    }

    /// venue/1 s4: the sender is its record's current key, then the six
    /// rejections, then one more record under (root, termsHash). `termsHash`
    /// is the key the standard names, for ERAVON its proxyCodeHash.
    function engage(
        bytes32 termsHash,
        address[] calldata members,
        bytes32[] calldata amanahHashes,
        bytes32 rosterHash,
        uint64 expires,
        address router
    ) external {
        IGaleedSuccessionBook book = IGaleedSuccessionBook(SUCCESSION_BOOK);
        if (book.current(msg.sender) != msg.sender) revert NotCurrent();
        if (members.length != amanahHashes.length) revert LengthMismatch();
        if (members.length == 0) revert EmptyMembers();
        for (uint256 i = 0; i < amanahHashes.length; i++) {
            if (amanahHashes[i] == bytes32(0)) revert ZeroWord();
            // one word for the whole panel: the guild's covenant word (venue/1 s2.4, s4)
            if (amanahHashes[i] != amanahHashes[0]) revert WordsDiffer();
        }
        if (expires <= block.timestamp) revert ExpiresNotInFuture();
        if (router != address(0) && router.code.length == 0) revert RouterNoCode();
        address root = book.root(msg.sender);
        bytes32 rh = router == address(0) ? bytes32(0) : router.codehash;
        Panel[] storage seq = _panels[root][termsHash];
        uint256 index = seq.length;
        seq.push(
            Panel({
                panelHash: keccak256(abi.encode(members, amanahHashes)),
                rosterHash: rosterHash,
                expires: expires,
                recorded: uint64(block.number),
                router: router,
                routerHash: rh
            })
        );
        emit Engaged(root, termsHash, index, msg.sender, members, amanahHashes, rosterHash, expires, router, rh);
    }

    /// The number of records under (root, termsHash).
    function count(address root, bytes32 termsHash) external view returns (uint256) {
        return _panels[root][termsHash].length;
    }

    /// One record by index; reverts where the index is beyond the sequence.
    function panelAt(address root, bytes32 termsHash, uint256 index)
        external
        view
        returns (bytes32 panelHash, bytes32 rosterHash, uint64 expires, uint64 recorded, address router, bytes32 routerHash)
    {
        Panel[] storage seq = _panels[root][termsHash];
        if (index >= seq.length) revert NoSuchIndex();
        Panel storage p = seq[index];
        return (p.panelHash, p.rosterHash, p.expires, p.recorded, p.router, p.routerHash);
    }
}
