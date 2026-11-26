// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {GaleedRouter, RouteView} from "../src/GaleedRouter.sol";

/// The standard router (galeed.venue/1 section 4): every condition and every
/// action, the constructor's refusals, the table read back, and one runtime
/// code for every table.
contract GaleedRouterTest is Test {
    address constant T1 = address(0x7001);
    address constant T2 = address(0x7002);
    address constant A = address(0xA001);
    address constant B = address(0xB001);
    address constant C = address(0xC001);
    address constant D = address(0xD001);

    function _any() internal pure returns (GaleedRouter.Rule memory r) {
        r.amountMax = type(uint256).max;
        r.rungsMax = type(uint64).max;
        r.seatMax = type(uint64).max;
    }

    function _one(GaleedRouter.Rule memory r) internal pure returns (GaleedRouter.Rule[] memory rs) {
        rs = new GaleedRouter.Rule[](1);
        rs[0] = r;
    }

    function _two(GaleedRouter.Rule memory r0, GaleedRouter.Rule memory r1) internal pure returns (GaleedRouter.Rule[] memory rs) {
        rs = new GaleedRouter.Rule[](2);
        rs[0] = r0;
        rs[1] = r1;
    }

    function _addrs(address a) internal pure returns (address[] memory x) { x = new address[](1); x[0] = a; }
    function _addrs(address a, address b) internal pure returns (address[] memory x) { x = new address[](2); x[0] = a; x[1] = b; }
    function _addrs(address a, address b, address c, address d) internal pure returns (address[] memory x) {
        x = new address[](4); x[0] = a; x[1] = b; x[2] = c; x[3] = d;
    }

    function _view() internal pure returns (RouteView memory v) {
        v.caseId = keccak256("case");
        v.seat = 0;
        v.termsHash = keccak256("terms");
        v.token = T1;
        v.amount = 100;
        v.rungs = 3;
        v.parties = _addrs(address(0x9001), address(0x9002));
        v.members = _addrs(A, B, C, D);
        v.seated = new address[](0);
        v.seed = keccak256("seed");
        v.blockTime = 1_800_000_000;
        v.chainId = 1;
        v.venue = address(0x8001);
    }

    // ---- the constructor's refusals ----

    function test_refusesEmptyTable() public {
        vm.expectRevert(GaleedRouter.EmptyTable.selector);
        new GaleedRouter(new GaleedRouter.Rule[](0));
    }

    function test_refusesBadRanges() public {
        GaleedRouter.Rule memory r = _any();
        r.amountMin = 2; r.amountMax = 1;
        vm.expectRevert(abi.encodeWithSelector(GaleedRouter.BadRule.selector, 0));
        new GaleedRouter(_one(r));
        r = _any(); r.rungsMin = 2; r.rungsMax = 1;
        vm.expectRevert(abi.encodeWithSelector(GaleedRouter.BadRule.selector, 1));
        new GaleedRouter(_two(_any(), r));
        r = _any(); r.seatMin = 2; r.seatMax = 1;
        vm.expectRevert(abi.encodeWithSelector(GaleedRouter.BadRule.selector, 0));
        new GaleedRouter(_one(r));
    }

    function test_refusesBadActions() public {
        GaleedRouter.Rule memory r = _any();
        r.mode = 3;
        vm.expectRevert(abi.encodeWithSelector(GaleedRouter.BadRule.selector, 0));
        new GaleedRouter(_one(r));
        r = _any(); r.mode = 1;                                    // weighted, no pool
        vm.expectRevert(abi.encodeWithSelector(GaleedRouter.BadRule.selector, 0));
        new GaleedRouter(_one(r));
        r = _any(); r.mode = 1; r.pool = _addrs(A, B); r.weights = new uint64[](1); r.weights[0] = 1;
        vm.expectRevert(abi.encodeWithSelector(GaleedRouter.BadRule.selector, 0));
        new GaleedRouter(_one(r));
        r = _any(); r.mode = 1; r.pool = _addrs(A, B); r.weights = new uint64[](2); r.weights[0] = 1;
        vm.expectRevert(abi.encodeWithSelector(GaleedRouter.BadRule.selector, 0));
        new GaleedRouter(_one(r));                                  // a zero weight
        r = _any(); r.weights = new uint64[](1); r.weights[0] = 1;  // weights on a uniform rule
        vm.expectRevert(abi.encodeWithSelector(GaleedRouter.BadRule.selector, 0));
        new GaleedRouter(_one(r));
        r = _any(); r.pool = _addrs(A, address(0));
        vm.expectRevert(abi.encodeWithSelector(GaleedRouter.BadRule.selector, 0));
        new GaleedRouter(_one(r));
        r = _any(); r.pool = _addrs(A, A);
        vm.expectRevert(abi.encodeWithSelector(GaleedRouter.BadRule.selector, 0));
        new GaleedRouter(_one(r));
    }

    // ---- the table, the code, the selector ----

    function test_rulesReadBackAsConstructed() public {
        GaleedRouter.Rule memory r0 = _any();
        r0.token = T2; r0.amountMin = 5; r0.amountMax = 50; r0.termsHash = keccak256("x");
        r0.rungsMin = 1; r0.rungsMax = 4; r0.seatMin = 2; r0.seatMax = 9;
        r0.mode = 1; r0.pool = _addrs(C, A); r0.weights = new uint64[](2); r0.weights[0] = 3; r0.weights[1] = 7;
        GaleedRouter.Rule memory r1 = _any();
        r1.mode = 2;
        GaleedRouter g = new GaleedRouter(_two(r0, r1));
        GaleedRouter.Rule[] memory back = g.rules();
        assertEq(keccak256(abi.encode(back)), keccak256(abi.encode(_two(r0, r1))));
    }

    function test_oneRuntimeCodeForEveryTable() public {
        GaleedRouter.Rule memory r1 = _any();
        r1.mode = 2; r1.pool = _addrs(B, C);
        GaleedRouter g0 = new GaleedRouter(_one(_any()));
        GaleedRouter g1 = new GaleedRouter(_two(r1, _any()));
        assertEq(address(g0).codehash, address(g1).codehash);
        assertTrue(keccak256(abi.encode(g0.rules())) != keccak256(abi.encode(g1.rules())));
    }

    /// the codeHash CODEHASH.md records, under the pinned settings
    function test_codeHashIsTheRecordedOne() public {
        assertEq(address(new GaleedRouter(_one(_any()))).codehash,
            0x492e67518544e136c69897b93df1555ccf722f438fbb5bbd3d00f9a1f6348986);
    }

    function test_routeSelectorIsTheInterfaces() public pure {
        bytes4 want = bytes4(keccak256(
            "route((bytes32,uint64,bytes32,address,uint256,uint64,address[],address[],address[],bytes32,uint64,uint256,address))"));
        assertEq(GaleedRouter.route.selector, want);
    }

    function test_tableIsNotExecutable() public {
        GaleedRouter g = new GaleedRouter(_one(_any()));
        address t = address(uint160(uint256(keccak256(abi.encodePacked(hex"d694", address(g), hex"01")))));
        assertEq(uint8(t.code[0]), 0);
        (bool ok, bytes memory ret) = t.call(hex"deadbeef");
        assertTrue(ok);
        assertEq(ret.length, 0);
    }

    /// the router's code over a table another init code wrote: valid ABI from
    /// byte 1 on, but a first byte other than STOP, so that it could execute
    function test_tableNotBeginningWithStopIsRefused() public {
        GaleedRouter g = new GaleedRouter(_one(_any()));
        address t = address(uint160(uint256(keccak256(abi.encodePacked(hex"d694", address(g), hex"01")))));
        bytes memory forged = t.code;
        forged[0] = 0x7f;
        vm.etch(t, forged);
        vm.expectRevert(GaleedRouter.ForeignTable.selector);
        g.rules();
        vm.expectRevert(GaleedRouter.ForeignTable.selector);
        g.route(_view());
        vm.etch(t, "");
        vm.expectRevert(GaleedRouter.ForeignTable.selector);
        g.route(_view());
    }

    // ---- conditions: each one failing passes the draw to the next rule ----

    /// rule 0 draws from [A] under one condition; rule 1 draws from [D]
    function _byCondition(GaleedRouter.Rule memory r0, RouteView memory v) internal returns (uint256) {
        r0.pool = _addrs(A);
        GaleedRouter.Rule memory r1 = _any();
        r1.pool = _addrs(D);
        return new GaleedRouter(_two(r0, r1)).route(v);
    }

    function test_conditionToken() public {
        GaleedRouter.Rule memory r = _any(); r.token = T1;
        RouteView memory v = _view();
        assertEq(_byCondition(r, v), 0);
        v.token = T2;
        assertEq(_byCondition(r, v), 3);
    }

    function test_conditionAmountInclusive() public {
        GaleedRouter.Rule memory r = _any(); r.amountMin = 100; r.amountMax = 200;
        RouteView memory v = _view();
        v.amount = 100; assertEq(_byCondition(r, v), 0);
        v.amount = 200; assertEq(_byCondition(r, v), 0);
        v.amount = 99; assertEq(_byCondition(r, v), 3);
        v.amount = 201; assertEq(_byCondition(r, v), 3);
    }

    function test_conditionTerms() public {
        GaleedRouter.Rule memory r = _any(); r.termsHash = keccak256("terms");
        RouteView memory v = _view();
        assertEq(_byCondition(r, v), 0);
        v.termsHash = keccak256("other terms");
        assertEq(_byCondition(r, v), 3);
    }

    function test_conditionRungsInclusive() public {
        GaleedRouter.Rule memory r = _any(); r.rungsMin = 2; r.rungsMax = 3;
        RouteView memory v = _view();
        v.rungs = 2; assertEq(_byCondition(r, v), 0);
        v.rungs = 3; assertEq(_byCondition(r, v), 0);
        v.rungs = 1; assertEq(_byCondition(r, v), 3);
        v.rungs = 4; assertEq(_byCondition(r, v), 3);
    }

    function test_conditionSeatInclusive() public {
        GaleedRouter.Rule memory r = _any(); r.seatMin = 1; r.seatMax = 1;
        RouteView memory v = _view();
        v.seat = 1; assertEq(_byCondition(r, v), 0);
        v.seat = 0; assertEq(_byCondition(r, v), 3);
        v.seat = 2; assertEq(_byCondition(r, v), 3);
    }

    // ---- actions ----

    function test_uniformDrawsEveryEligibleMemberAndNoOther() public {
        GaleedRouter g = new GaleedRouter(_one(_any()));
        RouteView memory v = _view();
        v.seated = _addrs(B);
        bool[4] memory hit;
        for (uint256 i = 0; i < 64; i++) {
            v.seed = keccak256(abi.encode(i));
            uint256 k = g.route(v);
            assertTrue(k < 4 && v.members[k] != B);
            hit[k] = true;
        }
        assertTrue(hit[0] && hit[2] && hit[3] && !hit[1]);
    }

    function test_uniformIsAFunctionOfSeedCaseAndSeat() public {
        GaleedRouter g = new GaleedRouter(_one(_any()));
        RouteView memory v = _view();
        uint256 k = g.route(v);
        v.blockTime = 1; v.chainId = 7; v.venue = address(1); v.parties = new address[](0);
        assertEq(g.route(v), k);
    }

    function test_weightedFollowsWeights() public {
        GaleedRouter.Rule memory r = _any();
        r.mode = 1; r.pool = _addrs(A, C); r.weights = new uint64[](2); r.weights[0] = 1; r.weights[1] = 63;
        GaleedRouter g = new GaleedRouter(_one(r));
        RouteView memory v = _view();
        uint256 a;
        uint256 c;
        for (uint256 i = 0; i < 256; i++) {
            v.seed = keccak256(abi.encode(i));
            uint256 k = g.route(v);
            if (k == 0) a++;
            else if (k == 2) c++;
            else revert("outside the pool");
        }
        assertGt(a, 0);
        assertGt(c, a * 8);
    }

    function test_weightedSkipsTheSeated() public {
        GaleedRouter.Rule memory r = _any();
        r.mode = 1; r.pool = _addrs(A, C); r.weights = new uint64[](2); r.weights[0] = 1; r.weights[1] = 1000;
        GaleedRouter g = new GaleedRouter(_one(r));
        RouteView memory v = _view();
        v.seated = _addrs(C);
        for (uint256 i = 0; i < 32; i++) {
            v.seed = keccak256(abi.encode(i));
            assertEq(g.route(v), 0);
        }
    }

    function test_rotationIgnoresTheSeedAndAdvancesPastTheSeated() public {
        GaleedRouter.Rule memory r = _any();
        r.mode = 2;
        GaleedRouter g = new GaleedRouter(_one(r));
        RouteView memory v = _view();
        uint256 first = g.route(v);
        v.seed = keccak256("another seed");
        assertEq(g.route(v), first);
        v.seated = _addrs(v.members[first]);
        assertEq(g.route(v), (first + 1) % 4);
        uint256 start = uint256(keccak256(abi.encode(v.caseId))) % 4;
        assertEq(first, start);
    }

    function test_poolMembersOutsideThePanelAreSkipped() public {
        GaleedRouter.Rule memory r = _any();
        r.pool = _addrs(address(0xE001), C);
        GaleedRouter g = new GaleedRouter(_one(r));
        RouteView memory v = _view();
        for (uint256 i = 0; i < 16; i++) {
            v.seed = keccak256(abi.encode(i));
            assertEq(g.route(v), 2);
        }
    }

    function test_firstPlaceOfARepeatedMember() public {
        GaleedRouter.Rule memory r = _any();
        r.pool = _addrs(B);
        GaleedRouter g = new GaleedRouter(_one(r));
        RouteView memory v = _view();
        v.members = _addrs(A, B, C, B);
        assertEq(g.route(v), 1);
    }

    /// a rule whose pool is the panel's members returns the place it lands on
    function test_panelPlacesOfARepeatedMember() public {
        GaleedRouter.Rule memory r = _any();
        r.mode = 2;
        GaleedRouter g = new GaleedRouter(_one(r));
        RouteView memory v = _view();
        v.members = _addrs(B, A, B, C);
        v.seated = _addrs(A, C);
        uint256 start = uint256(keccak256(abi.encode(v.caseId))) % 4;
        uint256 at = g.route(v);
        assertEq(v.members[at], B);
        // rotation from `start` lands on the first place of B at or after it, wrapping
        uint256 want = start;
        while (v.members[want] != B) want = (want + 1) % 4;
        assertEq(at, want);
    }

    /// a uniform rule over a panel of 128 members, eight of them seated, fits in 500,000 gas
    function test_gasOfALargePanel() public {
        GaleedRouter g = new GaleedRouter(_one(_any()));
        RouteView memory v = _view();
        v.members = new address[](128);
        for (uint256 i = 0; i < 128; i++) v.members[i] = address(uint160(0xE0000 + i));
        v.seated = new address[](8);
        for (uint256 i = 0; i < 8; i++) v.seated[i] = v.members[i * 15];
        bytes memory data = abi.encodeCall(GaleedRouter.route, (v));
        uint256 before = gasleft();
        (bool ok, bytes memory ret) = address(g).staticcall(data);
        uint256 used = before - gasleft();
        assertTrue(ok);
        assertLt(used, 500_000);
        uint256 at = abi.decode(ret, (uint256));
        for (uint256 i = 0; i < 8; i++) assertTrue(v.members[at] != v.seated[i]);
    }

    function test_noEligibleMemberFallsThroughThenReverts() public {
        GaleedRouter.Rule memory r0 = _any();
        r0.pool = _addrs(A);
        GaleedRouter.Rule memory r1 = _any();
        r1.mode = 2; r1.pool = _addrs(B);
        GaleedRouter g = new GaleedRouter(_two(r0, r1));
        RouteView memory v = _view();
        v.seated = _addrs(A);
        assertEq(g.route(v), 1);
        v.seated = _addrs(A, B);
        vm.expectRevert(GaleedRouter.NoMember.selector);
        g.route(v);
    }

    function test_noRuleHoldsReverts() public {
        GaleedRouter.Rule memory r = _any();
        r.token = T2;
        GaleedRouter g = new GaleedRouter(_one(r));
        vm.expectRevert(GaleedRouter.NoMember.selector);
        g.route(_view());
    }

    function test_routeIsAView() public {
        GaleedRouter g = new GaleedRouter(_one(_any()));
        RouteView memory v = _view();
        (bool ok, bytes memory ret) = address(g).staticcall{gas: 200_000}(abi.encodeCall(GaleedRouter.route, (v)));
        assertTrue(ok);
        assertEq(ret.length, 32);
    }

    /// a table of eight rules over a panel of sixteen fits in 300,000 gas
    function test_gasOfATypicalTable() public {
        GaleedRouter.Rule[] memory rs = new GaleedRouter.Rule[](8);
        address[] memory sixteen = new address[](16);
        for (uint256 i = 0; i < 16; i++) sixteen[i] = address(uint160(0xF000 + i));
        for (uint256 i = 0; i < 8; i++) {
            rs[i] = _any();
            rs[i].token = i < 7 ? T2 : address(0);
            rs[i].mode = uint8(i % 3);
            address[] memory pool = new address[](8);
            for (uint256 k = 0; k < 8; k++) pool[k] = sixteen[(i + k) % 16];
            rs[i].pool = pool;
            if (rs[i].mode == 1) {
                rs[i].weights = new uint64[](8);
                for (uint256 k = 0; k < 8; k++) rs[i].weights[k] = uint64(k + 1);
            }
        }
        GaleedRouter g = new GaleedRouter(rs);
        RouteView memory v = _view();
        v.members = sixteen;
        v.seated = _addrs(sixteen[7], sixteen[8], sixteen[9], sixteen[10]);
        uint256 before = gasleft();
        g.route(v);
        assertLt(before - gasleft(), 300_000);
    }

    function testFuzz_routeReturnsAnEligibleMember(bytes32 seed, bytes32 caseId, uint8 mode, uint8 seatedMask) public {
        GaleedRouter.Rule memory r = _any();
        r.mode = mode % 3;
        if (r.mode == 1) {
            r.pool = _addrs(A, B, C, D);
            r.weights = new uint64[](4);
            for (uint256 k = 0; k < 4; k++) r.weights[k] = uint64(k + 1);
        }
        GaleedRouter g = new GaleedRouter(_one(r));
        RouteView memory v = _view();
        v.seed = seed;
        v.caseId = caseId;
        uint256 n = 0;
        address[] memory seated = new address[](4);
        for (uint256 k = 0; k < 4; k++) if (seatedMask & (1 << k) != 0) seated[n++] = v.members[k];
        assembly { mstore(seated, n) }
        v.seated = seated;
        if (n == 4) {
            vm.expectRevert(GaleedRouter.NoMember.selector);
            g.route(v);
            return;
        }
        uint256 at = g.route(v);
        assertLt(at, 4);
        assertEq(seatedMask & (1 << at), 0);
    }
}
