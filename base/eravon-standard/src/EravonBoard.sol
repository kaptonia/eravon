// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

/// The ERAVON broadcast board (eravon.standard/1 s7b): one function, one
/// log. A seller posts its listing (eravon.link/1 s2) under the key
/// `termsHash` and its settlement offer (eravon.link/1 s8) under the key
/// `caseId`; a reader finds a document by the `key` or the `hash` topic and
/// authenticates it by its content and its signatures, never by `poster`.
/// No storage, no owner, no pause, no check on the bytes: a bad document
/// costs its poster the gas and spoils nobody's link, since a link names its
/// document's sha256. Deployed once per chain by whoever first needs it
/// through the public CREATE2 deployer under the salt
/// keccak256("eravon/1/board"), designated by code (CODEHASH.md).
contract EravonBoard {
    event Posted(address indexed poster, bytes32 indexed key, bytes32 indexed hash, bytes document);

    /// Any address, any bytes; `hash` is sha256(document) through the precompile.
    function post(bytes32 key, bytes calldata document) external {
        emit Posted(msg.sender, key, sha256(document), document);
    }
}
