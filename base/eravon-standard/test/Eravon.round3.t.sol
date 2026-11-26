// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {VmSafe} from "forge-std/Vm.sol";
import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {EravonVerifier} from "../src/EravonVerifier.sol";
import {EravonPanelBook} from "../src/EravonPanelBook.sol";
import {RawFeed, DecToken} from "./Eravon.fuzz.t.sol";

/// A plain settlement token that meters the gas each `transfer` is handed (the largest entry gas since the last
/// reset), and whose marked payees run a hook that burns what the push hands it: `burns` loops down to a few
/// thousand gas and answers false having moved nothing, `explodes` spends it all on an invalid opcode.
contract R3MeterToken {
    uint8 public decimals = 18;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;
    mapping(address => bool) public burns;
    mapping(address => bool) public explodes;
    uint256 public maxEntryGas;
    uint256 public transfers;

    function setBurns(address w, bool b) external {
        burns[w] = b;
    }

    function setExplodes(address w, bool b) external {
        explodes[w] = b;
    }

    function resetMeter() external {
        maxEntryGas = 0;
        transfers = 0;
    }

    function mint(address to, uint256 v) external {
        balanceOf[to] += v;
    }

    function approve(address s, uint256 v) external returns (bool) {
        allowance[msg.sender][s] = v;
        return true;
    }

    function transfer(address to, uint256 v) external returns (bool) {
        uint256 g = gasleft();
        if (g > maxEntryGas) maxEntryGas = g;
        transfers++;
        if (explodes[to]) {
            assembly ("memory-safe") {
                invalid()
            }
        }
        if (burns[to]) {
            while (gasleft() > 8000) {}
            return false;
        }
        balanceOf[msg.sender] -= v;
        balanceOf[to] += v;
        return true;
    }

    function transferFrom(address from, address to, uint256 v) external returns (bool) {
        allowance[from][msg.sender] -= v;
        balanceOf[from] -= v;
        balanceOf[to] += v;
        return true;
    }
}

/// A Chainlink-shaped feed (and a decimals() answerer) that burns a set amount of gas before each answer, or all
/// of it, and pads each answer with extra zero words (a return bomb, paid for by the feed's own gas).
contract R3GasFeed {
    int256 public answer;
    uint256 public at;
    uint256 public dec;
    uint256 public burnRound;
    uint256 public burnDec;
    uint256 public padRound;
    uint256 public padDec;
    bool public cutRound;
    uint256 public roundBytes;

    function set(int256 a, uint256 t, uint256 d) external {
        answer = a;
        at = t;
        dec = d;
    }

    function setGas(uint256 br, uint256 bd, uint256 pr, uint256 pd) external {
        burnRound = br;
        burnDec = bd;
        padRound = pr;
        padDec = pd;
    }

    /// latestRoundData answers exactly `n` bytes of its five words (an answer cut short).
    function setCut(bool c, uint256 n) external {
        cutRound = c;
        roundBytes = n;
    }

    function _burn(uint256 amount) internal view {
        if (amount == 0) return;
        uint256 g = gasleft();
        uint256 stop = g > amount ? g - amount : 0;
        while (gasleft() > stop) {}
    }

    function decimals() external view returns (uint256) {
        _burn(burnDec);
        uint256 d = dec;
        uint256 pad = padDec;
        assembly ("memory-safe") {
            let p := mload(0x40)
            mstore(p, d)
            return(p, mul(add(1, pad), 32))
        }
    }

    function latestRoundData() external view returns (uint80, int256, uint256, uint256, uint80) {
        _burn(burnRound);
        int256 a = answer;
        uint256 t = at;
        uint256 size = (5 + padRound) * 32;
        if (cutRound) size = roundBytes;
        assembly ("memory-safe") {
            let p := mload(0x40)
            mstore(p, 1)
            mstore(add(p, 32), a)
            mstore(add(p, 64), t)
            mstore(add(p, 96), t)
            mstore(add(p, 128), 1)
            return(p, size)
        }
    }
}

/// A token whose balanceOf reverts with its own reason.
contract R3MuteToken {
    function balanceOf(address) external pure returns (uint256) {
        revert("mute");
    }

    function transfer(address, uint256) external pure returns (bool) {
        revert("mute");
    }
}

/// A buyer's side contract for a trade completed off the instance: it pays the seller the agreed price from the
/// buyer's payout, under the payout's allowance, only once the instance is Refunded (so the buyer has its amount
/// back first and risks nothing), and it pays once.
contract R3SideSwap {
    Eravon public immutable inst;
    address public immutable token;
    address public immutable from;
    address public immutable to;
    uint256 public immutable price;
    bool public paid;

    constructor(Eravon i, address t, address f, address r, uint256 p) {
        inst = i;
        token = t;
        from = f;
        to = r;
        price = p;
    }

    function collect() external {
        Eravon.CaseStatus memory _st31 = inst.status();
        Eravon.Phase ph = _st31.phase;
        require(ph == Eravon.Phase.Refunded, "not refunded");
        require(!paid, "paid");
        paid = true;
        (bool ok, bytes memory ret) =
            token.call(abi.encodeWithSignature("transferFrom(address,address,uint256)", from, to, price));
        require(ok && (ret.length == 0 || abi.decode(ret, (bool))), "pay");
    }
}

/// A seller that is a contract (s9: it consents and acts by its own calls): it refunds and collects in one
/// transaction, so it never refunds unpaid.
contract R3SellerWallet {
    function exec(address target, bytes calldata data) external returns (bytes memory) {
        (bool ok, bytes memory ret) = target.call(data);
        if (!ok) {
            assembly ("memory-safe") {
                revert(add(ret, 32), mload(ret))
            }
        }
        return ret;
    }

    function refundAndCollect(Eravon inst, R3SideSwap swap) external {
        inst.refund();
        swap.collect();
    }
}

/// Round 3, seat FUZZ: round 2's findings R2-1 to R2-8 (each round-2 proof run on the revised code, the same
/// scenario now failing or answering as the text says, the other seats' duplicates named beside them), ADV2-2
/// and SEC2-L3 (no refund pre-empts a rung), the round's own findings R3-L1, R3-I2, R3-I3 and R3-I4 as the rules
/// that now hold, and the measurements over the code round 2 changed.
contract EravonRound3 is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE_PAYOUT = address(0x4000000000000000000000000000000000000001);
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    address internal constant STRANGER = address(0x3000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah");
    bytes32 internal constant ROSTER = keccak256("roster");
    bytes32 internal constant MANIFEST = keccak256("manifest");
    bytes internal constant ENTRY = "judgment entry bytes";
    uint256 internal constant TX_GAS = 16_777_216;
    uint256 internal constant PUSH_GAS = 1_000_000;
    uint256 internal constant READ_GAS = 200_000;
    bytes32 internal E;

    function setUp() public {
        setUpBase();
        E = sha256(ENTRY);
    }

    // ------------------------------------------------------------------
    // fixtures
    // ------------------------------------------------------------------

    function _open(Q memory q, uint256 amount) internal returns (Eravon inst, Eravon.OrderTerms memory t, bytes memory quote) {
        quote = encodeQuote(q);
        t = namedTerms(JUDGE, AMANAH, quote, amount);
        fundDeposit(depositOf(q, amount));
        inst = openNamed(t, quote);
    }

    function _settle(Eravon inst, uint16 bps) internal {
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 d = digestOf(address(inst), keccak256(abi.encode(TH_SETTLE, uint256(bps), exp)));
        vm.prank(SELLER);
        inst.settle(bps, exp, sign(BUYER_PK, d), sign(SELLER_PK, d));
    }

    function _members(uint256 n) internal pure returns (address[] memory m, bytes32[] memory w) {
        m = new address[](n);
        w = new bytes32[](n);
        for (uint256 i = 0; i < n; i++) {
            m[i] = address(uint160(0x001000000000000000000000000000000000000B01 + i));
            w[i] = AMANAH;
        }
    }

    function _engage(uint256 n) internal returns (address[] memory m, bytes32[] memory w) {
        EravonPanelBook book = deployBook(GUILD);
        (m, w) = _members(n);
        bytes32 key = logic.PROXY_CODEHASH();
        uint64 exp = uint64(vm.getBlockTimestamp() + 3650 days);
        vm.prank(GUILD, GUILD);
        book.engage(key, m, w, ROSTER, exp, address(0));
        vm.roll(vm.getBlockNumber() + 1);
    }

    /// A guild case armed at init (no draw made), under a quote with the given offer window and consent switch.
    function _guildCase(uint256 n, uint32 offerWindow, bool auto_)
        internal
        returns (Eravon inst, Eravon.OrderTerms memory t, address[] memory m, bytes32[] memory w)
    {
        (m, w) = _engage(n);
        Q memory q = defaultQuote(address(tok), address(0), auto_);
        q.offerWindow = offerWindow;
        bytes memory quote = encodeQuote(q);
        t = guildTerms(GUILD, AMANAH, quote, 100 ether);
        fundDeposit(q, 100 ether);
        inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), m, w);
    }

    function _armed(Eravon inst) internal view returns (bool pending, uint64 deadline, uint64 drawBlock) {
        Eravon.CaseStatus memory _st32 = inst.status();
        pending = _st32.pendingRedraw;
        deadline = _st32.redrawDeadline;
        drawBlock = _st32.drawBlock;
    }

    function _seats(Eravon inst) internal view returns (uint256 n) {
        Eravon.CaseStatus memory _st33 = inst.status();
        n = _st33.seats;
    }

    function _member(bytes32 id, bytes32 seed, address[] memory m, uint64 ordinal) internal pure returns (address) {
        return m[uint256(keccak256(abi.encodePacked(id, seed, ordinal))) % m.length];
    }

    function _countTopic(VmSafe.Log[] memory logs, address emitter, bytes32 topic) internal pure returns (uint256 k) {
        for (uint256 i = 0; i < logs.length; i++) {
            if (logs[i].emitter == emitter && logs[i].topics.length != 0 && logs[i].topics[0] == topic) k++;
        }
    }

    function _closed(VmSafe.Log[] memory logs, address emitter)
        internal
        pure
        returns (bool found, uint8 term, uint256 b, uint256 s, uint256 j, uint256 bond)
    {
        for (uint256 i = 0; i < logs.length; i++) {
            if (logs[i].emitter == emitter && logs[i].topics.length != 0 && logs[i].topics[0] == Eravon.Closed.selector) {
                (term, b, s, j, bond) = abi.decode(logs[i].data, (uint8, uint256, uint256, uint256, uint256));
                return (true, term, b, s, j, bond);
            }
        }
    }

    // ==================================================================
    // R2-1 (ADV2-1, SEC2-L1, VULN2-M-1): withholding the draw re-rolled it
    // ==================================================================

    /// Round 2's test_R2_1_aPartyRerollsTheDrawByWithholdingIt: the seller read the member from the armed block's
    /// hash and, where it was not the one it wanted, let 256 blocks pass and called redraw, which re-armed on a
    /// fresh block under the same deadline; every member of an eight-member pool was reachable. Now the same
    /// strategy reaches exactly one member, the first seed's: for every other target the withheld redraw closes
    /// the case as Resigned with form 3 (the amount to the buyer, the deposit to the seller), emits no DrawArmed,
    /// leaves no draw armed and seats nobody.
    function test_R2_1_withholdingTheDrawCannotChooseTheMember() public {
        uint256 n = 8;
        uint256 seatedTargets = 0;
        uint256 closedTargets = 0;
        for (uint256 target = 0; target < n; target++) {
            uint256 snap = vm.snapshotState();
            (Eravon inst, Eravon.OrderTerms memory t, address[] memory m, bytes32[] memory w) = _guildCase(n, 3 days, true);
            bytes32 id = orderHash(t);
            (, uint64 deadline, uint64 drawBlock) = _armed(inst);
            uint256 t0 = vm.getBlockTimestamp();
            vm.roll(uint256(drawBlock) + 1);
            vm.warp(t0 + 12);
            if (_member(id, blockhash(drawBlock), m, 0) == m[target]) {
                vm.prank(SELLER);
                inst.redraw(m, w);
                assertEq(inst.seatOf(id, 0).judge, m[target]);
                seatedTargets++;
            } else {
                // withheld: 256 blocks on, the seed is no longer served
                vm.roll(uint256(drawBlock) + 257);
                vm.warp(t0 + 12 * 257);
                assertLe(vm.getBlockTimestamp(), deadline, "inside the deadline, as round 2's re-roll was");
                uint256 buyerBefore = tok.balanceOf(BUYER_PAYOUT);
                uint256 sellerBefore = tok.balanceOf(SELLER);
                vm.recordLogs();
                vm.prank(SELLER);
                inst.redraw(m, w);
                VmSafe.Log[] memory logs = vm.getRecordedLogs();
                assertEq(_countTopic(logs, address(inst), Eravon.DrawArmed.selector), 0, "no re-arm");
                assertEq(_countTopic(logs, address(inst), Eravon.Resigned.selector), 1, "Resigned form 3");
                assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned));
                (bool pending,, uint64 db2) = _armed(inst);
                assertFalse(pending, "no draw left armed");
                assertEq(db2, drawBlock, "the armed block never moved");
                assertEq(_seats(inst), 0, "nobody seated");
                assertEq(tok.balanceOf(BUYER_PAYOUT) - buyerBefore, 100 ether, "the amount to the buyer");
                assertEq(tok.balanceOf(SELLER) - sellerBefore, depositOf(defaultQuote(address(tok), address(0), true), 100 ether));
                // and nothing later re-opens it
                vm.prank(SELLER);
                vm.expectRevert(Eravon.NotPendingRedraw.selector);
                inst.redraw(m, w);
                closedTargets++;
            }
            vm.revertToState(snap);
        }
        assertEq(seatedTargets, 1, "only the first seed's member can be seated");
        assertEq(closedTargets, n - 1, "every other target ends the case");
    }

    /// The same withholding met by advance: from block drawBlock + 257 on, advance with any seat argument closes
    /// the armed draw as Resigned form 3 inside the deadline; at drawBlock + 256 the seed is still served, advance
    /// leaves the draw armed and redraw seats the formula's member.
    function test_R2_1_advanceClosesALostSeedAndTheLastServedBlockStillDraws() public {
        (Eravon inst, Eravon.OrderTerms memory t, address[] memory m, bytes32[] memory w) = _guildCase(8, 3 days, true);
        bytes32 id = orderHash(t);
        (, uint64 deadline, uint64 drawBlock) = _armed(inst);
        uint256 s0 = vm.snapshotState();
        vm.roll(uint256(drawBlock) + 256);
        vm.warp(vm.getBlockTimestamp() + 12 * 256);
        inst.advance(id, 0);
        (bool pending,,) = _armed(inst);
        assertTrue(pending, "the seed is still served at drawBlock + 256");
        address want = _member(id, blockhash(drawBlock), m, 0);
        vm.prank(SELLER);
        inst.redraw(m, w);
        assertEq(inst.seatOf(id, 0).judge, want, "the last served block draws the formula's member");
        vm.revertToState(s0);
        uint64[3] memory args = [uint64(0), 7, type(uint64).max];
        for (uint256 k = 0; k < 3; k++) {
            uint256 s1 = vm.snapshotState();
            vm.roll(uint256(drawBlock) + 257);
            vm.warp(vm.getBlockTimestamp() + 12 * 257);
            assertLe(vm.getBlockTimestamp(), deadline);
            vm.recordLogs();
            inst.advance(id, args[k]);
            VmSafe.Log[] memory logs = vm.getRecordedLogs();
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned), "advance closes a lost seed");
            assertEq(_countTopic(logs, address(inst), Eravon.DrawArmed.selector), 0, "and never re-arms");
            vm.revertToState(s1);
        }
    }

    // ==================================================================
    // R2-2 (SEC2-I3, VULN2-I-2): cancel's and the expiry branch's pushes ran with all the gas
    // ==================================================================

    /// Round 2's test_R2_2_cancelPushesWithAllTheGas: cancel and the expiry branch handed a burning hook 63/64 of
    /// 8,000,000. Now every push of the standard's own is handed at most PUSH_GAS: the meter reads the gas at the
    /// token's entry for cancel, for the expiry branch, and for a terminal with all three payees burning, the
    /// credit stays owed each time, and each call spends about one allowance per burning push.
    function test_R2_2_everyPushOfTheStandardsOwnIsHandedAtMostPushGas() public {
        R3MeterToken mt = new R3MeterToken();
        Q memory q = defaultQuote(address(mt), JUDGE_PAYOUT, true);
        bytes memory quote = encodeQuote(q);
        mt.setBurns(BUYER_PAYOUT, true);
        // cancel
        Eravon.OrderTerms memory t1 = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t1.offer.token = address(mt);
        Eravon i1 = deployProxy(t1);
        mt.mint(address(i1), 1 ether);
        mt.resetMeter();
        vm.prank(BUYER);
        uint256 g0 = gasleft();
        i1.cancel{gas: 8_000_000}(t1, "");
        uint256 used = g0 - gasleft();
        assertLe(mt.maxEntryGas(), PUSH_GAS, "cancel's push under the allowance");
        assertGt(mt.maxEntryGas(), PUSH_GAS - 20_000, "and handed the whole allowance");
        assertLt(used, 1_500_000);
        assertEq(i1.owed(BUYER_PAYOUT, address(mt)), 1 ether, "the credit stays owed");
        emit log_named_uint("cancel under a burning hook, gas spent of 8,000,000", used);
        // the expiry branch
        Eravon.OrderTerms memory t2 = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t2.offer.token = address(mt);
        Eravon i2 = deployProxy(t2);
        mt.mint(address(i2), 1 ether);
        vm.warp(t2.openBy + 1);
        mt.resetMeter();
        vm.prank(BUYER);
        g0 = gasleft();
        i2.init{gas: 8_000_000}(t2, "", "", "", new address[](0), new bytes32[](0));
        used = g0 - gasleft();
        assertLe(mt.maxEntryGas(), PUSH_GAS, "the expiry branch's push under the allowance");
        assertLt(used, 1_500_000);
        assertEq(i2.owed(BUYER_PAYOUT, address(mt)), 1 ether);
        emit log_named_uint("the expiry branch under a burning hook, gas spent of 8,000,000", used);
        // a terminal with the buyer's payout, the seller's payout and the seller's key all burning
        Eravon.OrderTerms memory t3 = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        t3.offer.token = address(mt);
        Eravon i3 = deployProxy(t3);
        mt.mint(address(i3), 1 ether);
        uint256 d = depositOf(q, 1 ether);
        mt.mint(SELLER, d);
        vm.prank(SELLER);
        mt.approve(address(logic), d);
        bytes memory bs = buyerSig(t3);
        bytes memory ss = sellerSig(t3);
        vm.prank(SELLER);
        i3.init(t3, quote, bs, ss, new address[](0), new bytes32[](0));
        claim(i3, MANIFEST);
        mt.setBurns(SELLER_PAYOUT, true);
        mt.setBurns(SELLER, true);
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 dg = digestOf(address(i3), keccak256(abi.encode(TH_SETTLE, uint256(5000), exp)));
        bytes memory sb = sign(BUYER_PK, dg);
        bytes memory sk = sign(SELLER_PK, dg);
        mt.resetMeter();
        vm.prank(BUYER);
        g0 = gasleft();
        i3.settle{gas: TX_GAS}(5000, exp, sb, sk);
        used = g0 - gasleft();
        assertEq(mt.transfers(), 3, "three pushes");
        assertLe(mt.maxEntryGas(), PUSH_GAS, "each under the allowance");
        assertLt(used, 4_000_000, "three allowances and the case's own work");
        assertEq(uint8(phaseOf(i3)), uint8(Eravon.Phase.Settled));
        emit log_named_uint("a settlement with three burning payees, gas spent", used);
        // sweep and withdraw keep all the gas their caller brings (s8)
        mt.setBurns(BUYER_PAYOUT, false);
        mt.resetMeter();
        vm.prank(BUYER);
        i3.sweep{gas: 8_000_000}(BUYER_PAYOUT, address(mt));
        assertGt(mt.maxEntryGas(), 7_000_000, "sweep hands the token the caller's gas");
        assertEq(i3.owed(BUYER_PAYOUT, address(mt)), 0, "the buyer's credit delivered");
    }

    // ==================================================================
    // R2-3 (ADV2-5, SEC2-I2, VULN2-I-5): init on a codeless or mute token reverted bare
    // ==================================================================

    /// Round 2's test_R2_3_initOnATokenWithoutCodeRevertsBare: with both consents in hand init reverted with no
    /// data on a codeless token and bubbled the token's own reason on a refusing balanceOf. Now both read through
    /// _balance and answer Underfunded; cancel and the expiry branch land on the same orders.
    function test_R2_3_aCodelessOrMuteTokenAtInitIsUnderfunded() public {
        address[2] memory tokens = [address(uint160(uint256(keccak256("no code at this chain")))), address(new R3MuteToken())];
        for (uint256 k = 0; k < 2; k++) {
            Q memory q = defaultQuote(tokens[k], JUDGE_PAYOUT, true);
            bytes memory quote = encodeQuote(q);
            Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
            t.offer.token = tokens[k];
            Eravon inst = deployProxy(t);
            bytes memory bs = buyerSig(t);
            bytes memory ss = sellerSig(t);
            vm.prank(SELLER);
            vm.expectRevert(Eravon.Underfunded.selector);
            inst.init(t, quote, bs, ss, new address[](0), new bytes32[](0));
            uint256 s0 = vm.snapshotState();
            vm.prank(BUYER);
            inst.cancel(t, "");
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded));
            vm.revertToState(s0);
            vm.warp(t.openBy + 1);
            vm.prank(BUYER);
            inst.init(t, "", "", "", new address[](0), new bytes32[](0));
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.None), "the expiry branch opens nothing");
            vm.revertToState(s0);
        }
    }

    // ==================================================================
    // R2-4 (VULN2-I-5): s3 said 32 bytes where the code needs 160 for latestRoundData
    // ==================================================================

    /// Round 2's test_R2_4_aShortRoundAnswerIsStaleThoughTheTextSays32: the code refuses as stale every answer
    /// shorter than 160 bytes. The code is unchanged and s3 now says "latestRoundData reverts or answers fewer than
    /// 160 bytes"; the same scenario is now the text's (and a padded answer, 160 bytes and more, prices).
    function test_R2_4_aRoundAnswerBelow160BytesIsStaleAsTheTextSays() public {
        DecToken unit = new DecToken(6);
        uint256[7] memory lens = [uint256(0), 32, 64, 128, 159, 160, 192];
        for (uint256 k = 0; k < 7; k++) {
            R3GasFeed f = new R3GasFeed();
            f.set(2000e8, vm.getBlockTimestamp(), 8);
            Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
            q.feeUnit = address(unit);
            q.feedToken = address(f);
            q.maxStale = 1 days;
            q.feeMin = 5e6;
            q.feeMax = 500e6;
            bytes memory quote = encodeQuote(q);
            Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
            if (lens[k] >= 160) f.setGas(0, 0, (lens[k] - 160) / 32, 0);
            else f.setCut(true, lens[k]);
            EravonVerifier.Checked memory c = verifier.check(t, quote, "", "", new address[](0), new bytes32[](0));
            assertEq(c.stale, lens[k] < 160, "stale exactly below 160 bytes");
        }
    }

    // ==================================================================
    // R2-5 (SEC2-L2): a settlement past T2 took back the P the clock pays the judge
    // ==================================================================

    /// Round 2's test_R2_5_aSettlementPastT2TakesBackTheJudgesP: Claimed, the seat seated, T2 run out, a 1-bps
    /// settlement paid P back to the seller where advance pays the judge. Now the settlement pays the judge P, as
    /// the clock does; the parties' 1-bps route gains the seller nothing.
    function test_R2_5_aSettlementPastT2PaysTheJudgeAsTheClockWould() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.premiumBps = 300;
        uint256 a = 100 ether;
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, a);
        bytes32 id = orderHash(t);
        claim(inst, MANIFEST);
        vm.warp(uint256(inst.seatOf(id, 0).windowEnd) + 1);
        uint256 p = premiumOf(q, a);
        uint256 s0 = vm.snapshotState();
        inst.advance(id, 0);
        (, uint256 jcClock) = inst.accountOf(JUDGE, address(tok));
        assertEq(jcClock, p);
        vm.revertToState(s0);
        uint256 keyBefore = tok.balanceOf(SELLER);
        _settle(inst, 1);
        (, uint256 jc) = inst.accountOf(JUDGE, address(tok));
        assertEq(jc, p, "the late settlement pays the judge P");
        assertEq(tok.balanceOf(SELLER) - keyBefore, lfOf(q, a), "only the seller's LF back to the key");
        assertEq(tok.balanceOf(SELLER_PAYOUT), a - a / 10000);
        assertEq(inst.seatOf(id, 0).state, 5);
    }

    // ==================================================================
    // R2-6 (VULN2-I-3): two gas-burning feeds made init revert bare under any gas
    // ==================================================================

    /// Round 2's test_R2_6_twoGasBurningFeedsMakeInitRevertBare: two feeds burning what they were handed left the
    /// verifier out of gas under 16,777,216 and 30,000,000 alike. Now each read is handed READ_GAS: both feeds
    /// burning, a decimals() burning, and the unit's decimals() burning each answer RateStale under the
    /// transaction cap, by name, and check answers stale under a million gas.
    function test_R2_6_burningFeedsAndDecimalsAnswerRateStaleUnderTheCap() public {
        uint256 big = type(uint256).max;
        for (uint256 mode = 0; mode < 4; mode++) {
            R3GasFeed fT = new R3GasFeed();
            R3GasFeed fU = new R3GasFeed();
            R3GasFeed unit = new R3GasFeed(); // its decimals() serves as the reference unit's
            fT.set(2000e8, vm.getBlockTimestamp(), 8);
            fU.set(1e8, vm.getBlockTimestamp(), 8);
            unit.set(1, 0, 6);
            if (mode == 0) {
                fT.setGas(big, 0, 0, 0);
                fU.setGas(big, 0, 0, 0);
            } else if (mode == 1) {
                fT.setGas(0, big, 0, 0);
            } else if (mode == 2) {
                unit.setGas(0, big, 0, 0);
            } else {
                fT.setGas(big, big, 0, 0);
                fU.setGas(big, big, 0, 0);
                unit.setGas(0, big, 0, 0);
            }
            Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
            q.feeUnit = address(unit);
            q.feedToken = address(fT);
            q.feedUnit = address(fU);
            q.maxStale = 1 days;
            q.feeMin = 5e6;
            q.feeMax = 500e6;
            bytes memory quote = encodeQuote(q);
            Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
            EravonVerifier.Checked memory c = verifier.check{gas: 1_500_000}(t, quote, "", "", new address[](0), new bytes32[](0));
            assertTrue(c.stale, "stale under a million and a half gas");
            Eravon inst = deployProxy(t);
            tok.mint(address(inst), 1 ether);
            fundDeposit(premiumOf(q, 1 ether) + 1e17);
            bytes memory bs = buyerSig(t);
            bytes memory ss = sellerSig(t);
            vm.prank(SELLER);
            (bool ok, bytes memory ret) =
                address(inst).call{gas: TX_GAS}(abi.encodeCall(Eravon.init, (t, quote, bs, ss, new address[](0), new bytes32[](0))));
            assertFalse(ok);
            assertEq(ret.length, 4, "a name");
            assertEq(bytes4(ret), Eravon.RateStale.selector);
        }
    }

    // ==================================================================
    // R2-7 (ADV2-6, VULN2-I-4): a Refund() signed outside a dispute surrendered inside one
    // ==================================================================

    /// Round 2's test_R2_7_aStaleRefundSurrenders: the holder of a Refund() signed while Funded submitted it within
    /// T3 and the seller paid the judge P and the buyer's fee. Now refundBySig is refused while Disputed
    /// (WrongPhase, after the signature) and nothing moves, within T3 and past it; the seller's own call is refused
    /// within T3 too, and past T3 it is the plain refund: the amount and the buyer's fee to the buyer, P and the
    /// deposit to the seller, nothing to the judge.
    function test_R2_7_aSignedRefundIsRefusedWhileDisputed() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        uint256 a = 100 ether;
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, a);
        bytes32 id = orderHash(t);
        bytes memory refundSig = sign(SELLER_PK, digestOf(address(inst), keccak256(abi.encode(TH_REFUND))));
        claim(inst, MANIFEST);
        raise(inst);
        uint256 bal = tok.balanceOf(address(inst));
        vm.prank(BUYER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.refundBySig(refundSig);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.refund();
        assertEq(tok.balanceOf(address(inst)), bal, "nothing moved within T3");
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Disputed));
        vm.warp(uint256(inst.seatOf(id, 0).windowEnd) + 1);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.WrongPhase.selector);
        inst.refundBySig(refundSig);
        assertEq(tok.balanceOf(address(inst)), bal, "nothing moved past T3");
        uint256 buyer0 = tok.balanceOf(BUYER_PAYOUT);
        uint256 seller0 = tok.balanceOf(SELLER) + tok.balanceOf(SELLER_PAYOUT);
        vm.prank(SELLER);
        inst.refund();
        (, uint256 jc) = inst.accountOf(JUDGE, address(tok));
        assertEq(jc, 0, "nothing to the judge");
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded));
        assertEq(tok.balanceOf(BUYER_PAYOUT) - buyer0, a + lfOf(q, a), "the amount and the buyer's fee back");
        assertEq(tok.balanceOf(SELLER) + tok.balanceOf(SELLER_PAYOUT) - seller0, depositOf(q, a), "P and the deposit back");
    }

    // ==================================================================
    // R2-8 (ADV2-4, SEC2-I1, VULN2-L-1): a guild quote below a block never seated
    // ==================================================================

    /// Round 2's test_R2_8_aGuildQuoteWithAZeroOfferWindowNeverSeats: under offerWindow 0 or 11 the next block,
    /// twelve seconds on, was past the deadline and every redraw answered NotPendingRedraw. Now the block after the
    /// arming always draws: both quotes seat the formula's member in that block, and advance there closes nothing.
    function test_R2_8_aQuoteBelowABlockSeatsInTheNextBlock() public {
        uint32[3] memory windows = [uint32(0), 1, 11];
        for (uint256 k = 0; k < 3; k++) {
            uint256 s0 = vm.snapshotState();
            (Eravon inst, Eravon.OrderTerms memory t, address[] memory m, bytes32[] memory w) = _guildCase(3, windows[k], true);
            bytes32 id = orderHash(t);
            (, uint64 deadline, uint64 drawBlock) = _armed(inst);
            vm.roll(uint256(drawBlock) + 1);
            vm.warp(vm.getBlockTimestamp() + 12);
            assertGt(vm.getBlockTimestamp(), deadline, "the next block is past the deadline");
            inst.advance(id, 0);
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded), "advance in the next block closes nothing");
            address want = _member(id, blockhash(drawBlock), m, 0);
            vm.prank(SELLER);
            inst.redraw(m, w);
            assertEq(inst.seatOf(id, 0).judge, want, "seated in the block after the arming");
            assertTrue(inst.seatOf(id, 0).accepted != 0, "consent carried");
            vm.revertToState(s0);
        }
    }

    // ==================================================================
    // ADV2-2, SEC2-L3: a refund during the dispute pre-empted a costlier rung
    // ==================================================================

    /// ADV's test_FINDING_T10_surrenderPreemptsACostlierRung: ladder [0 fb0, 9900 fb1, 10000 fb0], A 100, the
    /// ruling at rung 1 gave the seller 1 and a refund during the dispute 10. Now the seller's refund within T3,
    /// its last second included, is refused (WrongPhase) and moves nothing: no rung is pre-empted, and the ruling
    /// at rung 1 stands.
    function test_ADV2_2_aRefundCannotPreemptTheRuling() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        uint256 a = 100 ether;
        Eravon.Rung[] memory l = new Eravon.Rung[](3);
        l[0] = Eravon.Rung(0, 10000, 0);
        l[1] = Eravon.Rung(9900, 100, 1);
        l[2] = Eravon.Rung(10000, 0, 0);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, a);
        t.offer.ladder = l;
        fundDeposit(depositOf(q, a));
        Eravon inst = openNamed(t, quote);
        bytes32 id = orderHash(t);
        claim(inst, MANIFEST);
        raise(inst);
        uint256 bal = tok.balanceOf(address(inst));
        uint64 end = inst.seatOf(id, 0).windowEnd;
        uint256[2] memory at = [vm.getBlockTimestamp(), uint256(end)];
        for (uint256 k = 0; k < 2; k++) {
            if (at[k] > vm.getBlockTimestamp()) vm.warp(at[k]);
            vm.prank(SELLER);
            vm.expectRevert(Eravon.WrongPhase.selector);
            inst.refund();
            assertEq(tok.balanceOf(address(inst)), bal, "nothing moved");
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Disputed));
        }
        uint256 sk0 = tok.balanceOf(SELLER);
        vm.prank(JUDGE, JUDGE);
        inst.rule(id, 0, E, 1, ENTRY, "");
        uint256 sellerRuled = tok.balanceOf(SELLER_PAYOUT) + tok.balanceOf(SELLER) - sk0;
        assertEq(sellerRuled, 1 ether, "rung 1 stands: the seller keeps 1 and its LF goes to the buyer");
    }

    // ==================================================================
    // the fresh hunt
    // ==================================================================

    /// R3-L1 (low): in Claimed with the seat seated a refund followed by the price handed back off the instance is
    /// the same completed trade as the release; the buyer's side contract pays only once the instance is Refunded,
    /// and a seller that is a contract (s9) refunds and collects in one transaction, so two parties who agree need
    /// no trust in each other. Every exit of that state pays the judge P: confirm, T2's lapse by anyone's advance,
    /// a settlement at any time, and the seller's refund (s8: P to the judge at a refund while Claimed under a
    /// seated seat, as T2's lapse would give it). Shown at T2's last second and one past it: refund-and-collect
    /// pays the judge P as the release does, and the two parties together end exactly where the release leaves
    /// them, so the route strips nothing.
    function test_R3_L1_aRefundAndASideSwapPayTheJudgeP() public {
        uint256 a = 100 ether;
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.premiumBps = 300;
        uint256 p = premiumOf(q, a);
        uint256 lf = lfOf(q, a);
        bytes memory quote = encodeQuote(q);
        R3SellerWallet wallet = new R3SellerWallet();
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, a);
        t.offer.sellerKey = address(wallet);
        t.offer.sellerFund = address(wallet);
        Eravon inst = deployProxy(t);
        bytes32 id = orderHash(t);
        tok.mint(address(inst), a);
        tok.mint(address(wallet), p + lf);
        wallet.exec(address(tok), abi.encodeWithSignature("approve(address,uint256)", address(logic), p + lf));
        // the seller consents by its own call, the buyer by signature
        wallet.exec(address(inst), abi.encodeCall(Eravon.init, (t, quote, buyerSig(t), bytes(""), new address[](0), new bytes32[](0))));
        wallet.exec(address(inst), abi.encodeCall(Eravon.claimDelivery, (MANIFEST)));
        uint64 end = inst.seatOf(id, 0).windowEnd;
        uint256 price = a - p / 2;
        for (uint256 k = 0; k < 2; k++) {
            uint256 s0 = vm.snapshotState();
            vm.warp(uint256(end) + k); // T2's last second, then one past it
            // the release the clock or a late confirm gives, and the settlement at 0 bps since round 2: P to the judge
            uint256 s1 = vm.snapshotState();
            vm.prank(BUYER);
            inst.confirm();
            (, uint256 jc) = inst.accountOf(JUDGE, address(tok));
            assertEq(jc, p, "the release pays the judge P");
            assertEq(tok.balanceOf(SELLER_PAYOUT), a);
            vm.revertToState(s1);
            uint256 exp = vm.getBlockTimestamp() + 1 days;
            bytes32 d = digestOf(address(inst), keccak256(abi.encode(TH_SETTLE, uint256(0), exp)));
            wallet.exec(address(inst), abi.encodeCall(Eravon.settle, (uint16(0), exp, sign(BUYER_PK, d), bytes(""))));
            (, jc) = inst.accountOf(JUDGE, address(tok));
            assertEq(jc, p, "a settlement pays the judge P");
            vm.revertToState(s1);
            // the same trade by refund and a side payment, atomic for both
            R3SideSwap swap = new R3SideSwap(inst, address(tok), BUYER_PAYOUT, SELLER_PAYOUT, price);
            vm.prank(BUYER_PAYOUT);
            tok.approve(address(swap), price);
            vm.expectRevert(bytes("not refunded"));
            swap.collect(); // the buyer's side pays nothing before its refund
            wallet.refundAndCollect(inst, swap);
            (, jc) = inst.accountOf(JUDGE, address(tok));
            assertEq(jc, p, "the refund pays the judge P, as the release does");
            assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Refunded));
            uint256 sellerGot = tok.balanceOf(SELLER_PAYOUT) + tok.balanceOf(address(wallet));
            assertEq(sellerGot, price + lf, "the seller: the side price and its LF back, P with the judge");
            assertEq(tok.balanceOf(BUYER_PAYOUT), a - price, "the buyer: the amount back less the side price");
            assertEq(sellerGot + tok.balanceOf(BUYER_PAYOUT), a + lf, "the pair together end where the release leaves them");
            vm.revertToState(s0);
        }
    }

    /// R3-I2 (informational): s10 has the direct and the signed form of one action test the actor first and the
    /// phase after, so the same wrong state answers the same fault under either. refundBySig tests the signature
    /// first in every phase: while Disputed a junk signature (65 zero bytes, or a sound one by another key) answers
    /// BadSellerSig, as refund() from a stranger answers NotSeller, and the seller's sound signature answers
    /// WrongPhase; at a terminal the same.
    function test_R3_I2_refundBySigTestsTheSignatureFirst() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, 100 ether);
        bytes32 id = orderHash(t);
        bytes32 dg = digestOf(address(inst), keccak256(abi.encode(TH_REFUND)));
        bytes memory garbage = new bytes(65);
        bytes memory other = sign(BUYER_PK, dg);
        bytes memory sound = sign(SELLER_PK, dg);
        claim(inst, MANIFEST);
        raise(inst);
        for (uint256 k = 0; k < 2; k++) {
            vm.prank(BUYER);
            vm.expectRevert(Eravon.BadSellerSig.selector);
            inst.refundBySig(garbage);
            vm.prank(BUYER);
            vm.expectRevert(Eravon.BadSellerSig.selector);
            inst.refundBySig(other);
            vm.prank(BUYER);
            vm.expectRevert(Eravon.WrongPhase.selector);
            inst.refundBySig(sound);
            vm.prank(STRANGER);
            vm.expectRevert(Eravon.NotSeller.selector);
            inst.refund();
            // then past a terminal, where both forms test the actor first as well
            if (k == 0) {
                vm.prank(JUDGE, JUDGE);
                inst.rule(id, 0, E, 0, ENTRY, "");
            }
        }
        // the claim's signed form in the wrong phase answers the signature first, the rule's general case
        vm.prank(BUYER);
        vm.expectRevert(Eravon.BadSellerSig.selector);
        inst.claimDeliveryBySig(MANIFEST, garbage);
    }

    /// R3-I3 (informational, text): s7 says that where blockhash(drawBlock) is no longer served, "more than 256
    /// blocks have passed, or the chain serves none", redraw closes the case as Resigned form 3 "and advance does
    /// the same from that block on"; advance tests the block count alone (block.number > drawBlock + 256), so on a
    /// chain that serves no hash (BLOCKHASH answering zero) redraw closes the case from the first block while
    /// advance leaves it armed until the deadline or the 257th block. Nothing is lost (anyone's redraw closes it, and
    /// s7 has clients offer the guild track only where hashes come from consensus); the two doors disagree.
    function test_R3_I3_onAChainServingNoHashAdvanceWaitsWhereRedrawCloses() public {
        (Eravon inst, Eravon.OrderTerms memory t, address[] memory m, bytes32[] memory w) = _guildCase(3, 3 days, true);
        bytes32 id = orderHash(t);
        (,, uint64 drawBlock) = _armed(inst);
        vm.roll(uint256(drawBlock) + 2);
        vm.warp(vm.getBlockTimestamp() + 24);
        vm.setBlockhash(drawBlock, bytes32(0)); // the chain serves no hash for the armed block
        assertEq(blockhash(drawBlock), bytes32(0));
        uint256 s0 = vm.snapshotState();
        inst.advance(id, 0);
        (bool pending,,) = _armed(inst);
        assertTrue(pending, "advance leaves the draw armed: it reads the block count, not the hash");
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
        vm.revertToState(s0);
        vm.prank(SELLER);
        inst.redraw(m, w);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned), "redraw closes the case at once");
    }

    /// R3-I4 (informational): feeds and a reference unit whose every read returns the largest answer its 200,000
    /// gas pays for (a return bomb: the five words and about 9,000 zero words). The verifier copies a fixed 160 or
    /// 32 bytes, so the answers' length costs the caller nothing beyond the reads' allowances: init prices by the
    /// answers' first words and its gas stays low, as test_F8 has it; a bomb past what READ_GAS pays for reads as
    /// no answer.
    function test_R3_I4_returnBombFeedsCostNothingBeyondTheAllowance() public {
        uint256 pad = 9000;
        R3GasFeed fT = new R3GasFeed();
        R3GasFeed fU = new R3GasFeed();
        R3GasFeed unit = new R3GasFeed();
        fT.set(2000e8, vm.getBlockTimestamp(), 8);
        fU.set(1e8, vm.getBlockTimestamp(), 8);
        unit.set(1, 0, 6);
        // the settlement token: decimals() bombing too
        uint256 gasPlain;
        uint256 gasBomb;
        for (uint256 k = 0; k < 2; k++) {
            if (k == 1) {
                fT.setGas(0, 0, pad, pad);
                fU.setGas(0, 0, pad, pad);
                unit.setGas(0, 0, 0, pad);
            }
            Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
            q.feeUnit = address(unit);
            q.feedToken = address(fT);
            q.feedUnit = address(fU);
            q.maxStale = 1 days;
            q.feeMin = 5e6;
            q.feeMax = 500e6;
            bytes memory quote = encodeQuote(q);
            Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
            EravonVerifier.Checked memory c = verifier.check(t, quote, "", "", new address[](0), new bytes32[](0));
            assertFalse(c.stale, "a bomb within READ_GAS answers");
            Eravon inst = deployProxy(t);
            tok.mint(address(inst), 1 ether);
            fundDeposit(premiumOf(q, 1 ether) + c.lf);
            bytes memory bs = buyerSig(t);
            bytes memory ss = sellerSig(t);
            vm.prank(SELLER);
            uint256 g0 = gasleft();
            (bool ok, bytes memory ret) =
                address(inst).call{gas: TX_GAS}(abi.encodeCall(Eravon.init, (t, quote, bs, ss, new address[](0), new bytes32[](0))));
            uint256 used = g0 - gasleft();
            assertTrue(ok, string(ret));
            if (k == 0) gasPlain = used;
            else gasBomb = used;
        }
        emit log_named_uint("init, plain feeds, gas", gasPlain);
        emit log_named_uint("init, five bombing reads (two feeds' two reads and the unit's decimals), gas", gasBomb);
        assertLt(gasBomb, 2_000_000, "the answers' length is not the caller's to pay");
        assertLe(gasBomb, gasPlain + 5 * READ_GAS, "nothing beyond the five reads' allowances");
        // a bomb one word past what READ_GAS pays: absent, RateStale by name
        R3GasFeed fB = new R3GasFeed();
        fB.set(2000e8, vm.getBlockTimestamp(), 8);
        fB.setGas(0, 0, 20000, 0);
        Q memory qb = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        qb.feeUnit = address(unit);
        qb.feedToken = address(fB);
        qb.maxStale = 1 days;
        qb.feeMin = 5e6;
        bytes memory quoteB = encodeQuote(qb);
        Eravon.OrderTerms memory tb = namedTerms(JUDGE, AMANAH, quoteB, 1 ether);
        EravonVerifier.Checked memory cb = verifier.check(tb, quoteB, "", "", new address[](0), new bytes32[](0));
        assertTrue(cb.stale, "a bomb past READ_GAS reads as no answer");
    }

    /// The first drawable block admitted whatever the clock, and advance in it: a guild quote whose deadline passed
    /// before the next block (offerWindow 1 day, the next block a week on) still draws in that block; advance in
    /// that block closes nothing; from the second block on the deadline governs (redraw NotPendingRedraw, advance
    /// closes as Resigned form 3).
    function test_R3_theFirstBlockDrawsWhateverTheClockAndTheSecondDoesNot() public {
        (Eravon inst, Eravon.OrderTerms memory t, address[] memory m, bytes32[] memory w) = _guildCase(3, 1 days, false);
        bytes32 id = orderHash(t);
        (, uint64 deadline, uint64 drawBlock) = _armed(inst);
        uint256 s0 = vm.snapshotState();
        vm.roll(uint256(drawBlock) + 1);
        vm.warp(uint256(deadline) + 7 days);
        inst.advance(id, 0);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Funded));
        vm.prank(SELLER);
        inst.redraw(m, w);
        assertEq(_seats(inst), 1, "drawn in the first block");
        assertEq(inst.seatOf(id, 0).windowEnd, uint64(vm.getBlockTimestamp()) + 1 days, "the offer clock starts at the draw");
        vm.revertToState(s0);
        vm.roll(uint256(drawBlock) + 2);
        vm.warp(uint256(deadline) + 1);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.NotPendingRedraw.selector);
        inst.redraw(m, w);
        inst.advance(id, 5);
        assertEq(uint8(phaseOf(inst)), uint8(Eravon.Phase.Resigned), "the deadline governs from the second block");
    }

    /// s8's P at a settlement against the clock's own outcome at the same moment, phase by phase on the named
    /// track (T1, T2, T3 at their last second and one past, the seat resigned in Claimed) and on the guild track
    /// (an unaccepted offer, an armed draw): wherever the clock is due, the settlement's P goes where the clock's
    /// does; where it is not, P goes to the judge exactly when the last seat is accepted and in flight.
    function test_R3_settlePIsTheClocksPInEveryPhase() public {
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        uint256 a = 100 ether;
        uint256 p = premiumOf(q, a);
        (Eravon inst, Eravon.OrderTerms memory t,) = _open(q, a);
        bytes32 id = orderHash(t);
        // Funded, T1
        _compare(inst, id, inst.seatOf(id, 0).windowEnd, p, 0);
        claim(inst, MANIFEST);
        _compare(inst, id, inst.seatOf(id, 0).windowEnd, p, p);
        uint256 s0 = vm.snapshotState();
        // Claimed after the seat resigned
        vm.prank(JUDGE, JUDGE);
        inst.resign(id, 0, E, ENTRY);
        _compare(inst, id, inst.seatOf(id, 0).windowEnd, 0, 0);
        vm.revertToState(s0);
        raise(inst);
        _compare(inst, id, inst.seatOf(id, 0).windowEnd, p, 0);
        // the guild track: an unaccepted offer, then an armed draw
        (Eravon g, Eravon.OrderTerms memory tg, address[] memory m, bytes32[] memory w) = _guildCase(3, 1 days, false);
        bytes32 gid = orderHash(tg);
        (, uint64 deadline,) = _armed(g);
        _compare(g, gid, deadline, 0, 0);
        vm.roll(vm.getBlockNumber() + 1);
        vm.prank(SELLER);
        g.redraw(m, w);
        _compare(g, gid, g.seatOf(gid, 0).windowEnd, 0, 0);
    }

    /// At `end` (the last second of the clock in force) and one second past it: the settlement's P to the judge is
    /// `inWindow` at `end`, and past it equals the clock's (`pastClock`, read from advance at that second).
    function _compare(Eravon inst, bytes32 id, uint256 end, uint256 inWindow, uint256 pastClock) internal {
        uint256 s0 = vm.snapshotState();
        if (end > vm.getBlockTimestamp()) vm.warp(end);
        assertEq(_settleJudge(inst), inWindow, "P at the window's last second");
        vm.revertToState(s0);
        vm.warp(end + 1);
        vm.roll(vm.getBlockNumber() + 2);
        uint256 s1 = vm.snapshotState();
        vm.recordLogs();
        Eravon.CaseStatus memory _st34 = inst.status();
        uint256 seats = _st34.seats;
        inst.advance(id, seats == 0 ? 0 : uint64(seats - 1));
        (bool closed,,,, uint256 jClock,) = _closed(vm.getRecordedLogs(), address(inst));
        uint8 ph = uint8(phaseOf(inst));
        vm.revertToState(s1);
        if (closed) assertEq(jClock, pastClock, "the clock's P as the test expects");
        else assertTrue(ph == 1, "the clock moved without closing only on the guild track's re-arm");
        assertEq(_settleJudge(inst), closed ? jClock : 0, "past the window: the settlement's P is the clock's");
        vm.revertToState(s0);
    }

    function _settleJudge(Eravon inst) internal returns (uint256 j) {
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 d = digestOf(address(inst), keccak256(abi.encode(TH_SETTLE, uint256(4000), exp)));
        vm.recordLogs();
        vm.prank(SELLER);
        inst.settle(4000, exp, sign(BUYER_PK, d), sign(SELLER_PK, d));
        (,,,, j,) = _closed(vm.getRecordedLogs(), address(inst));
    }

    /// The text's client rule for a quote below a block without consent carried: the draw lands in the next block
    /// (R2-8's fix) and opens an offer whose clock ends the second it starts, which the drawn member can accept only
    /// in that block; one block on it is WindowClosed and advance moves on to the next draw. s3 has the client treat
    /// such a quote as no candidate; nothing is lost (the amount and the deposit return at form 3).
    function test_R3_anOfferBelowABlockIsAcceptedOnlyInItsOwnBlock() public {
        (Eravon inst, Eravon.OrderTerms memory t, address[] memory m, bytes32[] memory w) = _guildCase(3, 0, false);
        bytes32 id = orderHash(t);
        vm.roll(vm.getBlockNumber() + 1);
        vm.warp(vm.getBlockTimestamp() + 12);
        vm.prank(SELLER);
        inst.redraw(m, w);
        address j = inst.seatOf(id, 0).judge;
        uint256 s0 = vm.snapshotState();
        vm.prank(j, j);
        inst.accept(id, 0);
        assertTrue(inst.seatOf(id, 0).accepted != 0, "accepted in the draw's own block");
        vm.revertToState(s0);
        vm.roll(vm.getBlockNumber() + 1);
        vm.warp(vm.getBlockTimestamp() + 12);
        vm.prank(j, j);
        vm.expectRevert(Eravon.WindowClosed.selector);
        inst.accept(id, 0);
        inst.advance(id, 0);
        (bool pending,,) = _armed(inst);
        assertTrue(pending, "the expired offer arms the next draw");
    }
}
