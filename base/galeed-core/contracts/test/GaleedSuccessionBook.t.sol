// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {GaleedSuccessionBook} from "../src/GaleedSuccessionBook.sol";

/// galeed.roster/1 section 4: the succession book. Every test names the clause
/// of section 4.2 it measures; the invariant at the end replays a random walk
/// of declarations and accessions against the replay of section 4.2 kept in
/// the test itself.
contract GaleedSuccessionBookTest is Test {
    GaleedSuccessionBook b;
    address P = address(0xA1);
    address Q = address(0xB2);
    address R = address(0xC3);
    address X = address(0xD4);

    event Declared(address indexed from, address indexed to);
    event Acceded(address indexed holder, address indexed to, address indexed root);

    function setUp() public {
        b = new GaleedSuccessionBook();
    }

    // reads of an address the book never met
    function test_untouchedAddressIsItsOwnRootAndKey() public view {
        assertEq(b.root(X), X);
        assertEq(b.current(X), X);
        assertEq(b.pending(X), address(0));
        assertFalse(b.entered(X));
    }

    // a declaration binds and moves nothing; the emitter enters
    function test_declareBindsAndMovesNothing() public {
        vm.expectEmit(true, true, false, true, address(b));
        emit Declared(P, Q);
        vm.prank(P); b.declare(Q);
        assertEq(b.pending(P), Q);
        assertEq(b.current(P), P);
        assertTrue(b.entered(P));
        assertFalse(b.entered(Q));
    }

    // an accession counts: root, head, pending, entered
    function test_handover() public {
        vm.prank(P); b.declare(Q);
        vm.expectEmit(true, true, true, true, address(b));
        emit Acceded(P, Q, P);
        vm.prank(Q); b.accede(P);
        assertEq(b.current(P), Q);
        assertEq(b.current(Q), Q);
        assertEq(b.root(Q), P);
        assertEq(b.root(P), P);
        assertEq(b.pending(P), address(0));
        assertTrue(b.entered(Q));
    }

    // a key that handed off is no longer current and cannot declare
    function test_retiredKeyCannotDeclare() public {
        vm.prank(P); b.declare(Q); vm.prank(Q); b.accede(P);
        vm.prank(P); vm.expectRevert(); b.declare(X);
    }

    // only the named key accedes
    function test_onlyNamedKeyAccedes() public {
        vm.prank(P); b.declare(Q);
        vm.prank(X); vm.expectRevert(); b.accede(P);
    }

    // an accession before any declaration counts for nothing
    function test_accedeBeforeDeclareReverts() public {
        vm.prank(Q); vm.expectRevert(); b.accede(P);
        vm.prank(P); b.declare(Q);
        vm.prank(Q); b.accede(P);
        assertEq(b.current(P), Q);
    }

    // a holder cannot accede to its own record
    function test_selfAccedeReverts() public {
        vm.prank(P); vm.expectRevert(); b.accede(P);
        vm.prank(P); b.declare(Q);
        vm.prank(P); vm.expectRevert(); b.accede(P);
    }

    // declaring again replaces the pending key
    function test_replaceDeclaration() public {
        vm.prank(P); b.declare(Q); vm.prank(P); b.declare(R);
        assertEq(b.pending(P), R);
        vm.prank(Q); vm.expectRevert(); b.accede(P);
        vm.prank(R); b.accede(P);
        assertEq(b.current(P), R);
    }

    // a key that entered the book is never named again, and never accedes
    function test_enteredKeyCannotBeNamedOrAccede() public {
        vm.prank(P); b.declare(Q); vm.prank(Q); b.accede(P);
        vm.prank(Q); vm.expectRevert(); b.declare(P);   // no return to a key that held office
        vm.prank(X); b.declare(R);                       // X enters by declaring
        vm.prank(Q); vm.expectRevert(); b.declare(X);    // X has entered: cannot be named
        vm.prank(Q); b.declare(R);
        vm.prank(X); vm.expectRevert(); b.accede(Q);     // X has entered: cannot accede
    }

    // a key named by a declaration that entered meanwhile cannot take it up
    function test_namedKeyThatEnteredCannotAccede() public {
        vm.prank(P); b.declare(Q);
        vm.prank(Q); b.declare(R);                       // Q enters by declaring for its own record
        vm.prank(Q); vm.expectRevert(); b.accede(P);
        assertEq(b.current(P), P);
    }

    function test_selfAndZero() public {
        vm.prank(P); vm.expectRevert(); b.declare(P);
        vm.prank(P); vm.expectRevert(); b.declare(address(0));
        assertFalse(b.entered(P));
    }

    function test_twoHops() public {
        vm.prank(P); b.declare(Q); vm.prank(Q); b.accede(P);
        vm.prank(Q); b.declare(R);
        vm.expectEmit(true, true, true, true, address(b));
        emit Acceded(Q, R, P);
        vm.prank(R); b.accede(Q);
        assertEq(b.current(P), R);
        assertEq(b.current(Q), R);
        assertEq(b.root(R), P);
        vm.prank(Q); vm.expectRevert(); b.declare(X);
        vm.prank(P); vm.expectRevert(); b.declare(X);
    }

    // a stranger naming the new key does not make it enter
    function test_strangerCannotBlockNewKey() public {
        vm.prank(P); b.declare(Q);
        vm.prank(X); b.declare(Q);
        assertFalse(b.entered(Q));
        vm.prank(Q); b.accede(P);
        assertEq(b.current(P), Q);
        vm.prank(Q); vm.expectRevert(); b.accede(X);     // Q has entered now
    }

    // an accession counts only while the holder is current
    function test_acceptanceOnlyWhileHolderCurrent() public {
        vm.prank(P); b.declare(Q); vm.prank(P); b.declare(R);
        vm.prank(R); b.accede(P);
        vm.prank(Q); vm.expectRevert(); b.accede(P);
    }

    // a declaration left pending after the record moved is taken up by nobody
    function test_pendingOfARetiredKeyIsDead() public {
        vm.prank(P); b.declare(Q); vm.prank(Q); b.accede(P);
        vm.prank(Q); b.declare(R);
        vm.prank(R); b.accede(Q);
        // P's pending was cleared by Q's accession; a new key X named nowhere cannot take P's record
        vm.prank(X); vm.expectRevert(); b.accede(P);
        vm.prank(X); vm.expectRevert(); b.accede(Q);
    }

    function testFuzz_currentIsNeverRetired(address a, address c) public {
        vm.assume(a != address(0) && c != address(0) && a != c);
        vm.prank(a); b.declare(c); vm.prank(c); b.accede(a);
        assertEq(b.current(a), c);
        vm.prank(a); vm.expectRevert(); b.declare(address(0xBEEF));
    }
}

/// A handler that drives random declarations and accessions among a small
/// set of keys and keeps the replay of galeed.roster/1 section 4.2 beside
/// the contract: every call the book admits is one the replay binds or
/// counts, and every call it refuses is one the replay rejects.
contract BookHandler is Test {
    GaleedSuccessionBook public b;
    address[] public keys;
    mapping(address => address) public rootOf;
    mapping(address => address) public head;
    mapping(address => address) public pend;
    mapping(address => bool) public ent;
    uint256 public mismatches;

    constructor(GaleedSuccessionBook b_) {
        b = b_;
        for (uint256 i = 1; i <= 6; i++) keys.push(address(uint160(0x1000 + i)));
        keys.push(address(0));
    }

    function _root(address a) internal view returns (address) { return rootOf[a] == address(0) ? a : rootOf[a]; }
    function _cur(address a) internal view returns (address) { address r = _root(a); return head[r] == address(0) ? r : head[r]; }
    function curOf(address a) external view returns (address) { return _cur(a); }
    function rootOfKey(address a) external view returns (address) { return _root(a); }
    function nkeys() external view returns (uint256) { return keys.length; }

    function declare(uint256 i, uint256 j) external {
        address e = keys[i % (keys.length - 1)];
        address to = keys[j % keys.length];
        bool binds = _cur(e) == e && to != address(0) && to != e && !ent[to];
        vm.prank(e);
        try b.declare(to) {
            if (!binds) mismatches++;
            pend[e] = to;
            ent[e] = true;
        } catch {
            if (binds) mismatches++;
        }
    }

    function accede(uint256 i, uint256 j) external {
        address e = keys[i % (keys.length - 1)];
        address h = keys[j % (keys.length - 1)];
        bool counts = !ent[e] && pend[h] == e && _cur(h) == h;
        vm.prank(e);
        try b.accede(h) {
            if (!counts) mismatches++;
            address r = _root(h);
            rootOf[e] = r;
            head[r] = e;
            pend[h] = address(0);
            ent[e] = true;
        } catch {
            if (counts) mismatches++;
        }
    }
}

contract GaleedSuccessionBookInvariants is Test {
    GaleedSuccessionBook b;
    BookHandler h;

    function setUp() public {
        b = new GaleedSuccessionBook();
        h = new BookHandler(b);
        targetContract(address(h));
    }

    /// the contract admits exactly what the replay binds and counts
    /// forge-config: default.invariant.runs = 128
    /// forge-config: default.invariant.depth = 64
    function invariant_bookIsTheReplay() public view {
        assertEq(h.mismatches(), 0);
    }

    /// current, root, pending, and entered agree with the replay for every key
    /// forge-config: default.invariant.runs = 128
    /// forge-config: default.invariant.depth = 64
    function invariant_readsAgree() public view {
        for (uint256 i = 0; i < h.nkeys(); i++) {
            address a = h.keys(i);
            assertEq(b.current(a), h.curOf(a));
            assertEq(b.root(a), h.rootOfKey(a));
            assertEq(b.pending(a), h.pend(a));
            assertEq(b.entered(a), h.ent(a));
        }
    }

    /// a current key is the head of its own root: current(current(a)) == current(a)
    /// forge-config: default.invariant.runs = 128
    /// forge-config: default.invariant.depth = 64
    function invariant_currentIsAFixpoint() public view {
        for (uint256 i = 0; i < h.nkeys(); i++) {
            address a = h.keys(i);
            assertEq(b.current(b.current(a)), b.current(a));
            assertEq(b.root(b.current(a)), b.root(a));
        }
    }
}

contract GaleedSuccessionBookIdentity is Test {
    /// the codeHash CODEHASH.md records, under the pinned settings
    function test_codeHashIsTheRecordedOne() public {
        assertEq(address(new GaleedSuccessionBook()).codehash,
            0xdb3293b42187b8b7270e62d5eaafee45af02c9caf3a0c8fe376cb4195f2f6aca);
    }
}
