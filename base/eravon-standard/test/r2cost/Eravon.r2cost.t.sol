// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "../EravonBase.sol";
import {Eravon} from "../../src/Eravon.sol";

/// FUZZ, round 2: what a draw without a router costs depends on the member the seed picks and on the seat. The client
/// rule of standard s7 and eravon.link/1 s10 tests "a draw under the panel" before paying; the draw the seed will pick
/// is unknown then. Measured here: seat 0 at the pool's first and last member, and seat 2 (two members declined) at
/// the last, for a panel of n distinct members, each draw's seed set so that the pick is the one named.
contract EravonR2DrawCost is EravonBase {
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("guild covenant");
    uint256 internal constant A = 100 ether;
    bytes internal constant ENTRY = "declined";

    function _seedFor(bytes32 id, uint64 seat, uint256 k, uint256 want) internal pure returns (bytes32 s) {
        for (uint256 i = 1; ; i++) {
            s = keccak256(abi.encode("seed", seat, i));
            if (uint256(keccak256(abi.encodePacked(id, s, seat))) % k == want) return s;
        }
    }

    function _draw(Eravon inst, address[] memory m, bytes32[] memory w, bytes32 id, uint64 seat, uint256 k, uint256 want)
        internal
        returns (uint256 used)
    {
        uint256 armed = inst.status().drawBlock;
        vm.roll(armed + 1);
        vm.setBlockhash(armed, _seedFor(id, seat, k, want));
        vm.prank(SELLER);
        uint256 g0 = gasleft();
        inst.redraw(m, w);
        used = g0 - gasleft();
    }

    function _decline(Eravon inst, bytes32 id, uint64 seat) internal {
        address j = inst.seatOf(id, seat).judge;
        bytes32 e = sha256(ENTRY);
        vm.prank(j, j);
        inst.resign(id, seat, e, ENTRY);
    }

    function _case(uint256 n) internal returns (Eravon inst, address[] memory m, bytes32[] memory w, bytes32 id) {
        m = new address[](n);
        w = new bytes32[](n);
        for (uint256 i = 0; i < n; i++) {
            m[i] = address(uint160(0x0020000000000000000000000000000000000000) + uint160(n << 16) + uint160(i + 1));
            w[i] = AMANAH;
        }
        bytes32 key = logic.PROXY_CODEHASH();
        uint64 exp = uint64(vm.getBlockTimestamp() + 30 days);
        vm.prank(GUILD, GUILD);
        pbook.engage(key, m, w, bytes32(0), exp, address(0));
        vm.roll(vm.getBlockNumber() + 1);
        Q memory q = defaultQuote(address(tok), address(0), false);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        inst = deployProxy(t);
        tok.mint(address(inst), A);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), m, w);
        id = orderHash(t);
    }

    function test_R2_aDrawsCostTurnsOnThePickAndTheSeat() public {
        setUpBase();
        uint256[4] memory ns = [uint256(100), 200, 400, 500];
        for (uint256 x = 0; x < 4; x++) {
            uint256 n = ns[x];
            uint256 snap = vm.snapshotState();
            (Eravon inst, address[] memory m, bytes32[] memory w, bytes32 id) = _case(n);
            uint256 s1 = vm.snapshotState();
            uint256 first = _draw(inst, m, w, id, 0, n, 0);
            assertEq(inst.seatOf(id, 0).judge, m[0], "seat 0, the first member");
            vm.revertToState(s1);
            uint256 last0 = _draw(inst, m, w, id, 0, n, n - 1);
            assertEq(inst.seatOf(id, 0).judge, m[n - 1], "seat 0, the last member");
            vm.revertToStateAndDelete(s1);
            _draw(inst, m, w, id, 0, n, 0);
            _decline(inst, id, 0);
            _draw(inst, m, w, id, 1, n - 1, 0);
            assertEq(inst.seatOf(id, 1).judge, m[1], "seat 1, the second member");
            _decline(inst, id, 1);
            uint256 last2 = _draw(inst, m, w, id, 2, n - 2, n - 3);
            assertEq(inst.seatOf(id, 2).judge, m[n - 1], "seat 2, the last member");
            emit log_named_uint(string.concat("n ", vm.toString(n), " seat 0, first pick"), first);
            emit log_named_uint(string.concat("n ", vm.toString(n), " seat 0, last pick"), last0);
            emit log_named_uint(string.concat("n ", vm.toString(n), " seat 2, last pick"), last2);
            assertGt(last0, (first * 3) / 2, "the last pick costs more than half as much again as the first");
            vm.revertToStateAndDelete(snap);
        }
    }

    /// The intrinsic cost of a transaction carrying `cd` (21,000 and 4 or 16 per calldata byte).
    function _intrinsic(bytes memory cd) internal pure returns (uint256 g) {
        g = 21_000;
        for (uint256 i = 0; i < cd.length; i++) g += cd[i] == 0 ? 4 : 16;
    }

    /// The cost of `init` and of the costliest draw (seat 2, the pool's last member) as whole transactions, for n from
    /// 200 to 450 by 25, against EIP-7825's cap of 2^24 gas on one transaction (Ethereum and Sepolia since Fusaka).
    function test_R2_panelSizesUnderTheTransactionCap() public {
        setUpBase();
        uint256 cap = 1 << 24;
        uint256 nInit;
        uint256 nDraw0;
        uint256 nDraw2;
        for (uint256 n = 200; n <= 450; n += 25) {
            uint256 snap = vm.snapshotState();
            address[] memory m = new address[](n);
            bytes32[] memory w = new bytes32[](n);
            for (uint256 i = 0; i < n; i++) {
                m[i] = address(uint160(0x0020000000000000000000000000000000000000) + uint160(n << 16) + uint160(i + 1));
                w[i] = AMANAH;
            }
            bytes32 key = logic.PROXY_CODEHASH();
            uint64 exp = uint64(vm.getBlockTimestamp() + 30 days);
            vm.prank(GUILD, GUILD);
            pbook.engage(key, m, w, bytes32(0), exp, address(0));
            vm.roll(vm.getBlockNumber() + 1);
            Q memory q = defaultQuote(address(tok), address(0), false);
            bytes memory quote = encodeQuote(q);
            Eravon.OrderTerms memory t = guildTerms(GUILD, AMANAH, quote, A);
            fundDeposit(q, A);
            Eravon inst = deployProxy(t);
            tok.mint(address(inst), A);
            bytes memory cdInit = abi.encodeCall(Eravon.init, (t, quote, buyerSig(t), sellerSig(t), m, w));
            vm.prank(SELLER);
            uint256 g0 = gasleft();
            (bool ok,) = address(inst).call(cdInit);
            uint256 gInit = g0 - gasleft() + _intrinsic(cdInit);
            require(ok, "init");
            bytes32 id = orderHash(t);
            uint256 drawTx = _intrinsic(abi.encodeCall(Eravon.redraw, (m, w)));
            uint256 s1 = vm.snapshotState();
            uint256 last0 = _draw(inst, m, w, id, 0, n, n - 1) + drawTx;
            vm.revertToStateAndDelete(s1);
            _draw(inst, m, w, id, 0, n, 0);
            _decline(inst, id, 0);
            _draw(inst, m, w, id, 1, n - 1, 0);
            _decline(inst, id, 1);
            uint256 last2 = _draw(inst, m, w, id, 2, n - 2, n - 3) + drawTx;
            emit log_named_uint(string.concat("n ", vm.toString(n), " init as a transaction"), gInit);
            emit log_named_uint(string.concat("n ", vm.toString(n), " seat 0 last pick as a transaction"), last0);
            emit log_named_uint(string.concat("n ", vm.toString(n), " seat 2 last pick as a transaction"), last2);
            if (gInit <= cap) nInit = n;
            if (last0 <= cap) nDraw0 = n;
            if (last2 <= cap) nDraw2 = n;
            vm.revertToStateAndDelete(snap);
        }
        emit log_named_uint("largest n of the sweep whose init fits 2^24", nInit);
        emit log_named_uint("largest n of the sweep whose seat-0 last-pick draw fits 2^24", nDraw0);
        emit log_named_uint("largest n of the sweep whose seat-2 last-pick draw fits 2^24", nDraw2);
        assertLt(nDraw2, nInit, "a panel whose init fits the cap can have a draw that does not");
    }
}
