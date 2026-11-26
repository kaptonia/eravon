// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {GaleedRegistry} from "../src/GaleedRegistry.sol";

contract GaleedRegistryTest is Test {
    GaleedRegistry reg;
    address judge = address(0xA11CE);

    event Published(address indexed publisher, bytes32 indexed word, bytes document);
    event StatusChanged(address indexed who, bool active);
    event Anchored(address indexed sender, bytes32 indexed hash, bytes entry);

    function setUp() public {
        reg = new GaleedRegistry();
    }

    /// publish: the caller is the publisher, the word is keccak256 of the bytes,
    /// and the bytes travel in the log as given.
    function test_publish_emits_caller_word_and_bytes() public {
        bytes memory doc = bytes('{"kind":"galeed.roster/1","guild":"0x00"}');
        vm.prank(judge);
        vm.expectEmit(true, true, false, true, address(reg));
        emit Published(judge, keccak256(doc), doc);
        reg.publish(doc);
    }

    function test_publish_empty_document() public {
        bytes memory doc = "";
        vm.prank(judge);
        vm.expectEmit(true, true, false, true, address(reg));
        emit Published(judge, keccak256(doc), doc);
        reg.publish(doc);
    }

    function test_setStatus_emits() public {
        vm.prank(judge);
        vm.expectEmit(true, false, false, true, address(reg));
        emit StatusChanged(judge, false);
        reg.setStatus(false);
    }

    /// anchorCarried: the hash is the entry's sha256, both indexed, the bytes as data.
    function test_anchorCarried_emits() public {
        bytes memory entry = bytes("an entry");
        bytes32 h = sha256(entry);
        vm.prank(judge);
        vm.expectEmit(true, true, false, true, address(reg));
        emit Anchored(judge, h, entry);
        reg.anchorCarried(h, entry);
    }

    function test_anchorCarried_refuses_mismatch() public {
        bytes memory entry = bytes("an entry");
        vm.prank(judge);
        vm.expectRevert();
        reg.anchorCarried(keccak256(entry), entry);
    }

    function test_anchorCarried_empty_entry() public {
        bytes memory entry = "";
        bytes32 h = sha256(entry);
        vm.prank(judge);
        vm.expectEmit(true, true, false, true, address(reg));
        emit Anchored(judge, h, entry);
        reg.anchorCarried(h, entry);
    }

    function test_anchorCarriedMany_emits_each() public {
        bytes[] memory es = new bytes[](3);
        es[0] = bytes("a"); es[1] = bytes("bb"); es[2] = bytes("ccc");
        bytes32[] memory hs = new bytes32[](3);
        for (uint256 i = 0; i < 3; i++) hs[i] = sha256(es[i]);
        vm.prank(judge);
        for (uint256 i = 0; i < 3; i++) {
            vm.expectEmit(true, true, false, true, address(reg));
            emit Anchored(judge, hs[i], es[i]);
        }
        reg.anchorCarriedMany(hs, es);
    }

    function test_anchorCarriedMany_refuses_one_mismatch() public {
        bytes[] memory es = new bytes[](2);
        es[0] = bytes("a"); es[1] = bytes("bb");
        bytes32[] memory hs = new bytes32[](2);
        hs[0] = sha256(es[0]); hs[1] = sha256(bytes("other"));
        vm.prank(judge);
        vm.expectRevert();
        reg.anchorCarriedMany(hs, es);
    }

    function test_anchorCarriedMany_refuses_length_mismatch() public {
        bytes[] memory es = new bytes[](1);
        es[0] = bytes("a");
        bytes32[] memory hs = new bytes32[](2);
        hs[0] = sha256(es[0]); hs[1] = sha256(es[0]);
        vm.prank(judge);
        vm.expectRevert();
        reg.anchorCarriedMany(hs, es);
    }

    function test_anchorCarriedMany_empty_emits_nothing() public {
        bytes[] memory es = new bytes[](0);
        bytes32[] memory hs = new bytes32[](0);
        vm.recordLogs();
        vm.prank(judge);
        reg.anchorCarriedMany(hs, es);
        assertEq(vm.getRecordedLogs().length, 0, "an empty array emits no log");
    }

    /// galeed/1 s9.1 containment: the hash of anchorCarried sits at calldata offset 4,
    /// and anchorCarriedMany's elements at 4 plus a multiple of 32.
    function test_calldata_containment_offsets() public pure {
        bytes memory entry = bytes("an entry");
        bytes32 h = sha256(entry);
        bytes memory cd = abi.encodeCall(GaleedRegistry.anchorCarried, (h, entry));
        bytes32 w;
        assembly { w := mload(add(cd, 36)) }
        assertEq(w, h, "anchorCarried: the hash at offset 4");
        bytes[] memory es = new bytes[](2);
        es[0] = bytes("a"); es[1] = bytes("bb");
        bytes32[] memory hs = new bytes32[](2);
        hs[0] = sha256(es[0]); hs[1] = sha256(es[1]);
        cd = abi.encodeCall(GaleedRegistry.anchorCarriedMany, (hs, es));
        // head: two offset words at 4 and 36; hashes array at 4 + 64: length, then elements
        bytes32 w0; bytes32 w1;
        assembly {
            w0 := mload(add(cd, add(32, add(4, 96))))
            w1 := mload(add(cd, add(32, add(4, 128))))
        }
        assertEq(w0, hs[0], "anchorCarriedMany: first element at 4 + 96");
        assertEq(w1, hs[1], "anchorCarriedMany: second element at 4 + 128");
    }

    /// Any address, any arguments that hash: permissionlessness is a structural property.
    function testFuzz_permissionless(address caller, bytes calldata doc, bytes calldata entry, bool active) public {
        vm.assume(caller != address(0));
        vm.startPrank(caller);
        reg.publish(doc);
        reg.setStatus(active);
        reg.anchorCarried(sha256(entry), entry);
        vm.stopPrank();
    }

    /// Mechanical proof of zero storage: call every function, then assert that
    /// not a single storage write happened at the contract address.
    function test_zero_storage_writes() public {
        vm.record();
        vm.startPrank(judge);
        reg.publish(bytes("doc"));
        reg.setStatus(true);
        reg.anchorCarried(sha256(bytes("x")), bytes("x"));
        bytes[] memory es = new bytes[](2);
        es[0] = bytes("y"); es[1] = bytes("z");
        bytes32[] memory hs = new bytes32[](2);
        hs[0] = sha256(es[0]); hs[1] = sha256(es[1]);
        reg.anchorCarriedMany(hs, es);
        vm.stopPrank();
        (, bytes32[] memory writes) = vm.accesses(address(reg));
        assertEq(writes.length, 0, "registry must never write storage");
    }

    /// galeed.roster/1 s1: the publisher topic is the registry's caller; a contract that
    /// publishes names itself, never the transaction's origin.
    function test_publish_through_contract_names_the_contract() public {
        Caller c = new Caller(reg);
        bytes memory doc = bytes("{}");
        vm.prank(judge, judge);
        vm.expectEmit(true, true, false, true, address(reg));
        emit Published(address(c), keccak256(doc), doc);
        c.publish(doc);
    }

    /// galeed/1 s9.1: topic 1 names the registry's caller; an anchor sent through an
    /// intermediate contract names that contract, which no reader counts for the seat.
    function test_anchorCarried_through_contract_names_the_contract() public {
        Caller c = new Caller(reg);
        bytes memory entry = bytes("an entry");
        bytes32 h = sha256(entry);
        vm.prank(judge, judge);
        vm.expectEmit(true, true, false, true, address(reg));
        emit Anchored(address(c), h, entry);
        c.relay(h, entry);
    }

    /// galeed/1 s9.1: one log per element of hashes; a hash repeated is two logs.
    function test_anchorCarriedMany_repeated_hash_emits_twice() public {
        bytes[] memory es = new bytes[](2);
        es[0] = bytes("same"); es[1] = bytes("same");
        bytes32[] memory hs = new bytes32[](2);
        hs[0] = sha256(es[0]); hs[1] = sha256(es[1]);
        vm.recordLogs();
        vm.prank(judge);
        reg.anchorCarriedMany(hs, es);
        assertEq(vm.getRecordedLogs().length, 2, "two elements, two logs");
    }

    function test_setStatus_true_emits() public {
        vm.prank(judge);
        vm.expectEmit(true, false, false, true, address(reg));
        emit StatusChanged(judge, true);
        reg.setStatus(true);
    }
}

/// An intermediate contract: its calls into the registry carry its own address as msg.sender.
contract Caller {
    GaleedRegistry immutable reg;
    constructor(GaleedRegistry r) { reg = r; }
    function publish(bytes calldata doc) external { reg.publish(doc); }
    function relay(bytes32 h, bytes calldata entry) external { reg.anchorCarried(h, entry); }
}
