// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

/// The guild's panel book: the panel face of galeed.venue/1 s4 for every
/// ERAVON instance (eravon.standard/1 s7). A guild deploys its own copy
/// through the public CREATE2 deployer under the salt
/// keccak256("eravon/1/panel" || guild) and sends `engage` from its own
/// address; an instance built on the guild's terms reads the latest record
/// under (guild, proxyCodeHash of the standard) at `init`. No owner, no
/// upgrade, no balance, no permission field: a record lives under the key
/// of whoever sent it, and an instance reads only the key the terms name.
contract EravonPanelBook {
    struct Panel {
        bytes32 panelHash;   // keccak256(abi.encode(members, amanahHashes))
        bytes32 rosterHash;  // the roster document's word (galeed.roster/1 s1), for audit
        uint64 expires;      // unix second after which the panel may no longer be frozen into a case
    }

    /// venue/1 s4: the append-only sequence under (sender, termsHash).
    mapping(address => mapping(bytes32 => Panel[])) private _panels;

    event Engaged(
        address indexed guild,
        bytes32 indexed termsHash,
        uint256 index,
        address[] members,
        bytes32[] amanahHashes,
        bytes32 rosterHash,
        uint64 expires
    );

    error LengthMismatch();
    error EmptyMembers();
    error ZeroWord();
    error WordsDiffer();
    error ExpiresNotInFuture();
    error NotAscending();
    error NoPanel();
    error NoSuchIndex();

    /// venue/1 s4: the five rejections, then one more entry under
    /// (msg.sender, termsHash). `termsHash` is the venue key the standard
    /// names, for ERAVON the standard's proxyCodeHash.
    function engage(
        bytes32 termsHash,
        address[] calldata members,
        bytes32[] calldata amanahHashes,
        bytes32 rosterHash,
        uint64 expires
    ) external {
        if (members.length != amanahHashes.length) revert LengthMismatch();
        if (members.length == 0) revert EmptyMembers();
        for (uint256 i = 0; i < amanahHashes.length; i++) {
            if (amanahHashes[i] == bytes32(0)) revert ZeroWord();
            // one word for the whole panel: the guild's covenant word (venue/1 s2.4, s4)
            if (amanahHashes[i] != amanahHashes[0]) revert WordsDiffer();
            // members strictly ascending: uniqueness holds at the write, the instance never re-derives it
            if (i != 0 && members[i] <= members[i - 1]) revert NotAscending();
        }
        if (expires <= block.timestamp) revert ExpiresNotInFuture();
        Panel[] storage seq = _panels[msg.sender][termsHash];
        uint256 index = seq.length;
        seq.push(Panel({panelHash: keccak256(abi.encode(members, amanahHashes)), rosterHash: rosterHash, expires: expires}));
        emit Engaged(msg.sender, termsHash, index, members, amanahHashes, rosterHash, expires);
    }

    /// The number of records under (guild, termsHash).
    function count(address guild, bytes32 termsHash) external view returns (uint256) {
        return _panels[guild][termsHash].length;
    }

    /// The latest record under (guild, termsHash); reverts where none exists.
    function latest(address guild, bytes32 termsHash)
        external
        view
        returns (uint256 index, bytes32 panelHash, bytes32 rosterHash, uint64 expires)
    {
        Panel[] storage seq = _panels[guild][termsHash];
        if (seq.length == 0) revert NoPanel();
        index = seq.length - 1;
        Panel storage p = seq[index];
        return (index, p.panelHash, p.rosterHash, p.expires);
    }

    /// One record by index; reverts where the index is beyond the sequence.
    function panelAt(address guild, bytes32 termsHash, uint256 index)
        external
        view
        returns (bytes32 panelHash, bytes32 rosterHash, uint64 expires)
    {
        Panel[] storage seq = _panels[guild][termsHash];
        if (index >= seq.length) revert NoSuchIndex();
        Panel storage p = seq[index];
        return (p.panelHash, p.rosterHash, p.expires);
    }
}
