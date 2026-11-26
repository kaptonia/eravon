// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title GaleedRegistry: the family's publication surface and carried anchor, a megaphone
/// @notice Zero storage, no owner, no logic beyond two hashes. Identity is the address;
///         a publication is a broadcast; every reading is a query over logs.
contract GaleedRegistry {
    /// @notice A seat publishes a document of a family law (galeed.roster/1: a roster,
    ///         a succession record, a judge document, a guild covenant; galeed.papers/1:
    ///         a service record). `word` is keccak256 of `document`, the word a roster is
    ///         cited by (galeed.venue/1 §4's rosterHash).
    event Published(address indexed publisher, bytes32 indexed word, bytes document);

    /// @notice Stop/resume accepting new cases (a judge) or retire/resume (a guild).
    event StatusChanged(address indexed who, bool active);

    /// @notice The anchor form of galeed/1 §9.1: the entry's sha256 and its bytes.
    event Anchored(address indexed sender, bytes32 indexed hash, bytes entry);

    function publish(bytes calldata document) external {
        emit Published(msg.sender, keccak256(document), document);
    }

    function setStatus(bool active) external {
        emit StatusChanged(msg.sender, active);
    }

    function anchorCarried(bytes32 hash, bytes calldata entry) external {
        require(sha256(entry) == hash);
        emit Anchored(msg.sender, hash, entry);
    }

    function anchorCarriedMany(bytes32[] calldata hashes, bytes[] calldata entries) external {
        require(hashes.length == entries.length);
        for (uint256 i = 0; i < hashes.length; ++i) {
            require(sha256(entries[i]) == hashes[i]);
            emit Anchored(msg.sender, hashes[i], entries[i]);
        }
    }
}
