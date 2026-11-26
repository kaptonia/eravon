// SPDX-License-Identifier: CC0-1.0
pragma solidity 0.8.28;

/// A contract a basis may declare as a registry, whose logs break the form of
/// galeed/1 s9.1 in one way each. Every function carries a hash in its
/// calldata at offset 4, so the only thing the scan can object to is the
/// log's shape or its data.
contract Rogue {
    bytes32 private constant T0 = keccak256("Anchored(address,bytes32,bytes)");
    bytes32 private constant T0_OLD = keccak256("Anchored(address,bytes32)");

    /// Two topics; the hash travels in the data instead.
    function twoTopics(bytes32 h) external {
        bytes32 t0 = T0;
        assembly {
            mstore(0, h)
            log2(0, 32, t0, caller())
        }
    }

    /// Three topics under the two-argument declaration's topic 0, no data: the
    /// form an earlier registry emitted, which is no anchor under this grammar.
    function oldForm(bytes32 h) external {
        bytes32 t0 = T0_OLD;
        assembly {
            log3(0, 0, t0, caller(), h)
        }
    }

    /// Four topics and no data.
    function fourTopics(bytes32 h) external {
        bytes32 t0 = T0;
        assembly {
            log4(0, 0, t0, caller(), h, h)
        }
    }

    /// Three well-formed topics whose data is the raw bytes and not the ABI
    /// encoding of one `bytes` value: an anchor that carries nothing.
    function rawData(bytes32 h, bytes calldata entry) external {
        bytes32 t0 = T0;
        bytes memory e = entry;
        assembly {
            log3(add(e, 32), mload(e), t0, caller(), h)
        }
    }

    /// Three well-formed topics whose data encodes bytes that do not hash to
    /// the topic: an anchor that carries nothing.
    function wrongBytes(bytes32 h, bytes calldata other) external {
        emit Anchored(msg.sender, h, other);
    }

    /// The hash is derived from the argument and appears nowhere in the
    /// calldata: no anchor.
    function derived(bytes32 x, bytes calldata entry) external {
        emit Anchored(msg.sender, keccak256(abi.encode(x)), entry);
    }

    event Anchored(address indexed sender, bytes32 indexed hash, bytes entry);
}

/// A contract that anchors through the registry from inside a call: topic 1
/// names this contract, never the transaction's sender.
contract Forwarder {
    function forward(address registry, bytes32 h, bytes calldata entry) external {
        (bool ok, ) = registry.call(abi.encodeWithSignature("anchorCarried(bytes32,bytes)", h, entry));
        require(ok);
    }
}

/// Code for an EOA to delegate to (EIP-7702): every call reverts.
contract Reverter {
    fallback() external payable {
        revert("reverter");
    }

    receive() external payable {
        revert("reverter");
    }
}
