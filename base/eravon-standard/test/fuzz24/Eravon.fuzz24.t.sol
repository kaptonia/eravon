// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "../EravonBase.sol";
import {Eravon} from "../../src/Eravon.sol";
import {FixedRouter, firstEligible} from "../replay/ReplayRouters.sol";
import {RouteView} from "../../src/IGaleedVenue.sol";

/// Standard s7: a router's answer naming the panel's zero entry names no member (the zero address is no judge address,
/// galeed.venue/1 s4), so the draw leaves the case with no usable panel and the instance ends it as Resigned with form 3.
/// The book records a panel with a zero entry (it tests the words, not the members), so the case is reachable by a
/// guild's own panel.
contract EravonFuzz24 is EravonBase {
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("guild covenant");
    uint256 internal constant A = 100 ether;

    function test_F24_aRouterNamingTheZeroEntryEndsTheCase() public {
        setUpBase();
        bytes32 T = logic.PROXY_CODEHASH();
        Q memory q = defaultQuote(address(tok), address(0), false);
        bytes memory quote = encodeQuote(q);
        address[] memory members = new address[](3);
        members[0] = address(uint160(0x001000000000000000000000000000000000000A01));
        members[1] = address(0);
        members[2] = address(uint160(0x001000000000000000000000000000000000000A02));
        bytes32[] memory words = new bytes32[](3);
        for (uint256 i = 0; i < 3; i++) words[i] = AMANAH;
        FixedRouter r = new FixedRouter(1);
        vm.prank(GUILD, GUILD);
        pbook.engage(T, members, words, keccak256("roster"), uint64(vm.getBlockTimestamp() + 30 days), address(r));
        vm.roll(vm.getBlockNumber() + 1);
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, words);
        vm.roll(vm.getBlockNumber() + 1);
        uint256 b0 = tok.balanceOf(BUYER_PAYOUT);
        vm.expectEmit(true, true, true, true, address(inst));
        emit Eravon.Resigned(bytes32(0), 3);
        vm.prank(SELLER);
        inst.redraw(members, words);
        // the answer was one word, index 1 < 3, naming the zero entry: no member, so the case ends with form 3
        assertEq(uint8(inst.status().phase), uint8(Eravon.Phase.Resigned), "Resigned");
        assertEq(inst.status().seats, 0, "no seat opened for members[1]");
        assertEq(tok.balanceOf(BUYER_PAYOUT) - b0, A, "the amount back to the buyer");
    }
}

/// Standard s7, exactly routeGas(): a router that reads its entry gas first and answers only where it equals the entry
/// gas a staticcall of exactly routeGas() gives it (calibrated on the same code with the same view), so that an instance
/// handing routeGas() - 1 or + 1 fails here where the gas sweeps' probes, which accept eight units either side, pass it.
contract ExactGasRouter {
    uint256 public expect;

    function setExpect(uint256 e) external {
        expect = e;
    }

    function route(RouteView calldata v) external view returns (uint256) {
        uint256 g = gasleft();
        if (expect == 0) return g;
        if (g != expect) return type(uint256).max;
        return firstEligible(v);
    }
}

contract EravonFuzz24Gas is EravonBase {
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("guild covenant");
    uint256 internal constant A = 100 ether;

    function test_F24_theRouterIsHandedExactlyRouteGas() public {
        setUpBase();
        bytes32 T = logic.PROXY_CODEHASH();
        Q memory q = defaultQuote(address(tok), address(0), false);
        bytes memory quote = encodeQuote(q);
        address[] memory members = new address[](3);
        members[0] = address(uint160(0x001000000000000000000000000000000000000A01));
        members[1] = address(uint160(0x001000000000000000000000000000000000000A02));
        members[2] = address(uint160(0x001000000000000000000000000000000000000A03));
        bytes32[] memory words = new bytes32[](3);
        for (uint256 i = 0; i < 3; i++) words[i] = AMANAH;
        // the entry gas of route() under exactly routeGas(), read on the router itself before it is told to expect it
        ExactGasRouter r = new ExactGasRouter();
        RouteView memory v;
        v.members = members;
        v.parties = new address[](5);
        v.seated = new address[](0);
        (bool ok, bytes memory ret) = address(r).staticcall{gas: logic.routeGas()}(abi.encodeCall(ExactGasRouter.route, (v)));
        assertTrue(ok && ret.length == 32, "calibration");
        r.setExpect(abi.decode(ret, (uint256)));
        vm.prank(GUILD, GUILD);
        pbook.engage(T, members, words, keccak256("roster"), uint64(vm.getBlockTimestamp() + 30 days), address(r));
        vm.roll(vm.getBlockNumber() + 1);
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), members, words);
        vm.roll(vm.getBlockNumber() + 1);
        vm.prank(SELLER);
        inst.redraw(members, words);
        assertEq(uint8(inst.status().phase), uint8(Eravon.Phase.Funded), "the draw seated: the router was handed exactly routeGas()");
        assertEq(inst.seatOf(orderHash(t), 0).judge, members[0], "members[0]");
    }
}
