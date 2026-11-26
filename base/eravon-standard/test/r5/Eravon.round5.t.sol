// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {EravonBase} from "../EravonBase.sol";
import {Eravon} from "../../src/Eravon.sol";
import {EravonPanelBook} from "../../src/EravonPanelBook.sol";
import {IGaleedVenue} from "../../src/IGaleedVenue.sol";
import {Company} from "../mocks/Mocks.sol";

/// Round 5: a settlement token whose balanceOf answers by its own state: a transfer to a blocked payee answers false
/// and moves nothing, and a transfer may arm the token so that every later balanceOf reverts until the next transfer
/// disarms it (a read that does not answer, standard s8). Plain otherwise.
contract R5ReadFlip {
    mapping(address => uint256) public bal;
    mapping(address => mapping(address => uint256)) public allowance;
    mapping(address => bool) public blocked;
    mapping(address => bool) public armsOnTransferTo; // a transfer to this payee leaves balanceOf reverting
    bool public failReads;

    function setBlocked(address a, bool b) external {
        blocked[a] = b;
    }

    function setArm(address a, bool b) external {
        armsOnTransferTo[a] = b;
    }

    function disarm() external {
        failReads = false;
    }

    function mint(address to, uint256 v) external {
        bal[to] += v;
    }

    function approve(address sp, uint256 v) external returns (bool) {
        allowance[msg.sender][sp] = v;
        return true;
    }

    function balanceOf(address a) external view returns (uint256) {
        require(!failReads, "no answer");
        return bal[a];
    }

    function decimals() external pure returns (uint8) {
        return 18;
    }

    function transfer(address to, uint256 v) external returns (bool) {
        failReads = armsOnTransferTo[to];
        if (blocked[to] || bal[msg.sender] < v) return false;
        bal[msg.sender] -= v;
        bal[to] += v;
        return true;
    }

    function transferFrom(address f, address to, uint256 v) external returns (bool) {
        require(allowance[f][msg.sender] >= v && bal[f] >= v, "transferFrom");
        allowance[f][msg.sender] -= v;
        bal[f] -= v;
        bal[to] += v;
        return true;
    }
}

/// Round 5, seat FUZZ: proofs of the round's angles one case at a time: a reorganisation that moves the arming
/// block, EIP-7702 delegated accounts in the judge's and the parties' roles, two parties' acts in one block against
/// the judge, the guild of five whose cases three seats bound, and odd decimals() at the gate.
contract EravonRound5 is EravonBase {
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant STRANGER = address(0x3000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("round 5 covenant");
    bytes internal constant ENTRY = "round 5 entry";
    uint256 internal constant A = 100 ether;
    bytes32 internal E;
    EravonPanelBook internal book;
    address[] internal m5;
    bytes32[] internal w5;

    function setUp() public {
        setUpBase();
        E = sha256(ENTRY);
        book = deployBook(GUILD);
        for (uint256 k = 0; k < 5; k++) {
            m5.push(address(uint160(0x001000000000000000000000000000000000000E01 + k)));
            w5.push(AMANAH);
        }
        bytes32 T = logic.PROXY_CODEHASH();
        vm.prank(GUILD, GUILD);
        book.engage(T, m5, w5, keccak256("roster"), uint64(vm.getBlockTimestamp() + 365 days), address(0));
        vm.roll(vm.getBlockNumber() + 10);
    }

    function _guildOrder(bool auto_) internal returns (Eravon inst, Eravon.OrderTerms memory t, bytes memory quote) {
        Q memory q = defaultQuote(address(tok), address(0), auto_);
        quote = encodeQuote(q);
        t = guildTerms(GUILD, AMANAH, quote, A);
        fundDeposit(q, A);
        inst = deployProxy(t);
        tok.mint(address(inst), A);
    }

    function _seats(Eravon inst) internal view returns (uint256 n) {
        Eravon.CaseStatus memory _st40 = inst.status();
        n = _st40.seats;
    }

    function _pending(Eravon inst) internal view returns (bool p, uint64 db) {
        Eravon.CaseStatus memory _st41 = inst.status();
        p = _st41.pendingRedraw;
        db = _st41.drawBlock;
    }

    function _model(bytes32 id, uint64 db, uint64 seat, address[] memory seatedBefore) internal view returns (address) {
        bytes32 seed = blockhash(db);
        address[] memory el = new address[](5);
        uint256 n;
        for (uint256 k = 0; k < 5; k++) {
            bool before;
            for (uint256 r = 0; r < seatedBefore.length; r++) {
                if (seatedBefore[r] == m5[k]) before = true;
            }
            if (!before) el[n++] = m5[k];
        }
        return el[uint256(keccak256(abi.encodePacked(id, seed, seat))) % n];
    }

    /// A reorganisation moves `init` (the arming) from one block to the next: the member seated by the draw in the block
    /// after is the formula's for whichever block armed it, so that whoever orders the arming block's transactions
    /// (its producer) can move the draw, as s7 says, and nobody else can; the money and the terms are the same in both
    /// histories.
    function test_R5_aReorganisationThatMovesTheArmingMovesOnlyTheSeed() public {
        (Eravon inst, Eravon.OrderTerms memory t, bytes memory quote) = _guildOrder(false);
        bytes32 id = orderHash(t);
        bytes memory data = abi.encodeCall(Eravon.init, (t, quote, buyerSig(t), sellerSig(t), m5, w5));
        uint256 b0 = vm.getBlockNumber();
        uint256 s0 = vm.snapshotState();
        address[2] memory seated;
        uint64[2] memory armed;
        for (uint256 h = 0; h < 2; h++) {
            vm.revertToState(s0);
            vm.roll(b0 + h); // the second history: init one block later
            vm.prank(SELLER);
            (bool ok,) = address(inst).call(data);
            assertTrue(ok, "init lands in either block");
            (bool p, uint64 db) = _pending(inst);
            assertTrue(p && db == b0 + h, "armed in the block init landed in");
            vm.roll(db + 1);
            vm.prank(BUYER);
            inst.redraw(m5, w5);
            seated[h] = inst.seatOf(id, 0).judge;
            armed[h] = db;
            assertEq(seated[h], _model(id, db, 0, new address[](0)), "the formula's member for that block");
            assertEq(depositHeld(inst), (A * 100) / 10000 + 10 ether, "the same deposit in either history");
        }
        emit log_named_address("member when init lands in block b0", seated[0]);
        emit log_named_address("member when init lands in block b0 + 1", seated[1]);
    }

    /// EIP-7702: a guild member whose key carries a delegation designator is drawn like any other (the draw reads no
    /// code), but its accept, rule and resign are refused NotDirect (venue/1 s2.9); the offer's clock runs out, advance
    /// arms the next draw and the next member is seated; the case ends nowhere it would not end undelegated.
    function test_R5_aDelegatedMemberCannotActAndTheClockRedraws() public {
        (Eravon inst, Eravon.OrderTerms memory t, bytes memory quote) = _guildOrder(false);
        bytes32 id = orderHash(t);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), m5, w5);
        vm.roll(vm.getBlockNumber() + 1);
        vm.prank(SELLER);
        inst.redraw(m5, w5);
        address j0 = inst.seatOf(id, 0).judge;
        vm.etch(j0, abi.encodePacked(hex"ef0100", address(new Company())));
        vm.prank(j0, j0);
        vm.expectRevert(Eravon.NotDirect.selector);
        inst.accept(id, 0);
        vm.prank(j0, j0);
        vm.expectRevert(Eravon.NotDirect.selector);
        inst.resign(id, 0, E, ENTRY);
        IGaleedVenue.Seat memory s = inst.seatOf(id, 0);
        vm.warp(uint256(s.windowEnd) + 1);
        inst.advance(id, 0);
        assertEq(inst.seatOf(id, 0).state, 5, "the offer expired");
        (bool p,) = _pending(inst);
        assertTrue(p, "the next draw is armed");
        vm.roll(vm.getBlockNumber() + 1);
        vm.prank(SELLER);
        inst.redraw(m5, w5);
        address j1 = inst.seatOf(id, 1).judge;
        assertTrue(j1 != j0, "another member");
        vm.prank(j1, j1);
        inst.accept(id, 1);
        assertTrue(inst.seatOf(id, 1).accepted != 0, "an undelegated member accepts");
    }

    /// EIP-7702: a seller whose key runs code (a delegation's) consents at init by its own call through that code, an
    /// empty slot, claims and refunds through it; a buyer whose key carries a designator still signs Confirm(); nothing
    /// reads either party's code (s9).
    function test_R5_delegatedPartiesActByCallAndBySignature() public {
        Q memory q = defaultQuote(address(tok), address(0), true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, A);
        t.offer.buyerMayOpen = false;
        fundDeposit(q, A);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), A);
        vm.etch(SELLER, address(new Company()).code);
        vm.etch(BUYER, abi.encodePacked(hex"ef0100", address(this)));
        Company(SELLER).exec(address(inst), abi.encodeCall(Eravon.init, (t, quote, buyerSig(t), bytes(""), new address[](0), new bytes32[](0))));
        assertEq(uint8(phaseOf(inst)), 1, "opened by the seller's own call through its code");
        Company(SELLER).exec(address(inst), abi.encodeCall(Eravon.claimDelivery, (keccak256("m"))));
        bytes memory sig = sign(BUYER_PK, digestOf(address(inst), keccak256(abi.encode(TH_CONFIRM))));
        uint256 s0 = vm.snapshotState();
        vm.prank(BUYER);
        inst.confirmBySig(sig);
        assertEq(uint8(phaseOf(inst)), 5, "a delegated buyer's signature confirms");
        vm.revertToState(s0);
        Company(SELLER).exec(address(inst), abi.encodeCall(Eravon.refund, ()));
        assertEq(uint8(phaseOf(inst)), 4, "the seller refunds through its code");
    }

    /// Two parties' acts sent for one block against the judge while Claimed under a seated seat: the refund and the
    /// confirmation, the settlement and the confirmation, the signed refund and the settlement, in both orders: the
    /// judge is credited P whichever lands (s8), so no ordering of the two parties' acts strips it.
    function test_R5_twoPartiesInOneBlockCannotStripPWhileClaimed() public {
        Q memory q = defaultQuote(address(tok), address(0), true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = openNamed(t, quote);
        bytes32 id = orderHash(t);
        claim(inst, keccak256("m"));
        uint256 P = (A * 100) / 10000;
        bytes memory rs = sign(SELLER_PK, digestOf(address(inst), keccak256(abi.encode(TH_REFUND))));
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 d = digestOf(address(inst), keccak256(abi.encode(TH_SETTLE, uint256(3000), exp)));
        bytes memory settleCall = abi.encodeCall(Eravon.settle, (3000, exp, sign(BUYER_PK, d), sign(SELLER_PK, d)));
        bytes[3] memory x = [abi.encodeCall(Eravon.refund, ()), settleCall, abi.encodeCall(Eravon.refundBySig, (rs))];
        bytes[3] memory y = [abi.encodeCall(Eravon.confirm, ()), abi.encodeCall(Eravon.confirm, ()), settleCall];
        address[3] memory xs = [SELLER, BUYER, BUYER];
        address[3] memory ys = [BUYER, BUYER, SELLER];
        uint256 s0 = vm.snapshotState();
        for (uint256 k = 0; k < 3; k++) {
            for (uint256 o = 0; o < 2; o++) {
                vm.revertToState(s0);
                vm.prank(o == 0 ? xs[k] : ys[k]);
                (bool ok1,) = address(inst).call(o == 0 ? x[k] : y[k]);
                vm.prank(o == 0 ? ys[k] : xs[k]);
                (bool ok2,) = address(inst).call(o == 0 ? y[k] : x[k]);
                assertTrue(ok1 && !ok2, "the first lands, the second is refused");
                (, uint256 cl) = inst.accountOf(JUDGE, address(tok));
                assertEq(cl, P, "the judge holds P whichever lands");
            }
        }
        id;
    }

    /// The guild of five: three unaccepted seats end the case as Resigned with form 3 though two members were never
    /// drawn (s4: at most three seats); an accepted seat that resigns while Funded ends the case and is never redrawn.
    function test_R5_threeSeatsBoundTheGuildOfFive() public {
        (Eravon inst, Eravon.OrderTerms memory t, bytes memory quote) = _guildOrder(false);
        bytes32 id = orderHash(t);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), m5, w5);
        for (uint64 k = 0; k < 3; k++) {
            vm.roll(vm.getBlockNumber() + 1);
            vm.prank(SELLER);
            inst.redraw(m5, w5);
            address j = inst.seatOf(id, k).judge;
            vm.prank(j, j);
            inst.resign(id, k, E, ENTRY);
        }
        assertEq(uint8(phaseOf(inst)), 8, "Resigned after three declines");
        assertEq(_seats(inst), 3, "three seats of a pool of five");
        (bool p,) = _pending(inst);
        assertTrue(!p, "no fourth draw");

        (inst, t, quote) = _guildOrder(false);
        t.offer.nonce = 77;
        inst = deployProxy(t);
        tok.mint(address(inst), A);
        fundDeposit(depositOf(defaultQuote(address(tok), address(0), false), A));
        id = orderHash(t);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), m5, w5);
        vm.roll(vm.getBlockNumber() + 1);
        vm.prank(SELLER);
        inst.redraw(m5, w5);
        address j0 = inst.seatOf(id, 0).judge;
        vm.prank(j0, j0);
        inst.accept(id, 0);
        vm.prank(j0, j0);
        inst.resign(id, 0, E, ENTRY);
        assertEq(uint8(phaseOf(inst)), 8, "Resigned (form 1) while Funded");
        assertEq(_seats(inst), 1, "an accepted seat is never redrawn");
    }

    /// A ruling ahead of a settlement in a dispute: the parties sign Settle(5000) during the dispute and send it; the
    /// judge's `rule` lands first. Settlement first: the judge holds P, the buyer A/2 and its fee back. Ruling first (rung
    /// 0 here): the judge holds P and the buyer's fee, the seller A, and the settlement answers WrongPhase. Both are
    /// s8 rows; the order decides, and the text names no order for this pair.
    function test_R5_aRulingAheadOfASettlementTakesTheBuyersFee() public {
        Q memory q = defaultQuote(address(tok), address(0), true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = openNamed(t, quote);
        bytes32 id = orderHash(t);
        claim(inst, keccak256("m"));
        raise(inst);
        (uint256 lf,,,,) = inst.feeQuoted();
        uint256 P = (A * 100) / 10000;
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 d = digestOf(address(inst), keccak256(abi.encode(TH_SETTLE, uint256(5000), exp)));
        bytes memory settleCall = abi.encodeCall(Eravon.settle, (5000, exp, sign(BUYER_PK, d), sign(SELLER_PK, d)));
        bytes memory ruleCall = abi.encodeCall(Eravon.rule, (id, 0, E, 0, ENTRY, ""));
        uint256 s0 = vm.snapshotState();
        vm.prank(BUYER);
        (bool ok,) = address(inst).call(settleCall);
        assertTrue(ok, "the settlement lands");
        (, uint256 cl) = inst.accountOf(JUDGE, address(tok));
        assertEq(cl, P, "settled first: the judge holds P");
        assertEq(tok.balanceOf(BUYER_PAYOUT), A / 2 + lf, "settled first: the buyer has half and its fee back");
        vm.revertToState(s0);
        vm.prank(JUDGE, JUDGE);
        (ok,) = address(inst).call(ruleCall);
        assertTrue(ok, "the ruling lands");
        vm.prank(BUYER);
        (ok,) = address(inst).call(settleCall);
        assertTrue(!ok, "the settlement behind it is refused");
        (, cl) = inst.accountOf(JUDGE, address(tok));
        assertEq(cl, P + lf, "ruled first: the judge holds P and the buyer's fee");
        assertEq(tok.balanceOf(BUYER_PAYOUT), 0, "ruled first: the buyer has nothing");
    }

    /// The clock ahead of a settlement past T2's end: the parties settle at 5000 one second past T2 and a stranger's
    /// `advance` lands first: Released, the whole amount to the seller, and the settlement answers WrongPhase. The judge's
    /// P is the same either way (round 3's rule); the buyer's half is the order's.
    function test_R5_theClockAheadOfASettlementPastT2() public {
        Q memory q = defaultQuote(address(tok), address(0), true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, A);
        fundDeposit(q, A);
        Eravon inst = openNamed(t, quote);
        bytes32 id = orderHash(t);
        claim(inst, keccak256("m"));
        vm.warp(uint256(inst.seatOf(id, 0).windowEnd) + 1);
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 d = digestOf(address(inst), keccak256(abi.encode(TH_SETTLE, uint256(5000), exp)));
        bytes memory settleCall = abi.encodeCall(Eravon.settle, (5000, exp, sign(BUYER_PK, d), sign(SELLER_PK, d)));
        uint256 s0 = vm.snapshotState();
        vm.prank(BUYER);
        (bool ok,) = address(inst).call(settleCall);
        assertTrue(ok, "the settlement lands past T2");
        assertEq(tok.balanceOf(BUYER_PAYOUT), A / 2, "settled first: the buyer has half");
        vm.revertToState(s0);
        vm.prank(STRANGER);
        inst.advance(id, 0);
        vm.prank(BUYER);
        (ok,) = address(inst).call(settleCall);
        assertTrue(!ok, "the settlement behind the clock is refused");
        assertEq(tok.balanceOf(BUYER_PAYOUT), 0, "the clock first: the buyer has nothing");
        assertEq(tok.balanceOf(SELLER_PAYOUT), A, "the clock first: the seller has the amount");
    }

    function _flipCase(R5ReadFlip ft, uint16 premium) internal returns (Eravon inst, Eravon.OrderTerms memory t) {
        Q memory q = defaultQuote(address(ft), address(0), true);
        q.premiumBps = premium;
        bytes memory quote = encodeQuote(q);
        t = namedTerms(JUDGE, AMANAH, quote, A);
        t.offer.token = address(ft);
        uint256 dep = depositOf(q, A);
        ft.mint(SELLER, dep);
        vm.prank(SELLER);
        ft.approve(address(logic), dep);
        inst = deployProxy(t);
        ft.mint(address(inst), A);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
    }

    /// A push whose second balance read does not answer counts as delivered (ADV5-1's class, without any gas game): the
    /// seller's payout is blocked (its transfer answers false, nothing moves) and that transfer leaves the token's
    /// balanceOf reverting. The confirm's push to the seller's payout reads zero after it, so the instance debits the
    /// whole credit: the seller's payout is owed nothing and has received nothing. The next push, to the seller's key,
    /// reads zero before it and its transfer (which disarms the token) moves the deposit's LF: counted as nothing, the key
    /// is paid and still owed. The amount less that LF sits unrecorded, and the buyer's sweep takes it.
    function test_R5_aPushWhoseSecondReadFailsIsCountedAsPaid() public {
        R5ReadFlip ft = new R5ReadFlip();
        (Eravon inst,) = _flipCase(ft, 100);
        (uint256 lf,,,,) = inst.feeQuoted();
        claim(inst, keccak256("m"));
        ft.setBlocked(SELLER_PAYOUT, true);
        ft.setArm(SELLER_PAYOUT, true);
        vm.prank(BUYER);
        inst.confirm();
        assertEq(ft.bal(SELLER_PAYOUT), 0, "nothing reached the seller's payout");
        assertEq(inst.owed(SELLER_PAYOUT, address(ft)), 0, "yet the seller's payout is owed nothing");
        assertEq(ft.bal(SELLER), lf, "the seller's key received its LF");
        assertEq(inst.owed(SELLER, address(ft)), lf, "and is still owed it");
        Eravon.CaseStatus memory _st42 = inst.status();
        uint256 unpaid = _st42.unpaid;
        assertEq(ft.bal(address(inst)) - unpaid, A - lf, "the amount less that LF sits unrecorded");
        vm.prank(SELLER);
        inst.sweep(BUYER_PAYOUT, address(ft));
        assertEq(ft.bal(BUYER_PAYOUT), A - lf, "the buyer's sweep took the seller's amount");
    }

    /// The other direction, alone: a push whose first read does not answer while its transfer moves counts as nothing,
    /// so the payee is paid and still owed. A settlement at 5000 under a 50% premium: the push to the buyer's payout moves
    /// its half and leaves balanceOf reverting (its second read reads zero and debits the half it did pay); the push to
    /// the seller's payout reads zero before it, moves its half and disarms the token: counted as nothing, still owed.
    /// The instance then owes more than it holds, and the seller's sweep is paid again out of the judge's P.
    function test_R5_aPushWhoseFirstReadFailsPaysTwice() public {
        R5ReadFlip ft = new R5ReadFlip();
        (Eravon inst,) = _flipCase(ft, 5000);
        uint256 P = A / 2;
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 d = digestOf(address(inst), keccak256(abi.encode(TH_SETTLE, uint256(5000), exp)));
        ft.setArm(BUYER_PAYOUT, true);
        vm.prank(SELLER);
        inst.settle(5000, exp, sign(BUYER_PK, d), sign(SELLER_PK, d));
        assertEq(ft.bal(BUYER_PAYOUT), A / 2, "the buyer's payout was paid its half");
        assertEq(ft.bal(SELLER_PAYOUT), A / 2, "the seller's payout was paid its half");
        assertEq(inst.owed(SELLER_PAYOUT, address(ft)), A / 2, "and is still owed its half");
        (, uint256 cl) = inst.accountOf(JUDGE, address(ft));
        assertEq(cl, P, "the judge holds P");
        Eravon.CaseStatus memory _st43 = inst.status();
        uint256 unpaid = _st43.unpaid;
        assertLt(ft.bal(address(inst)), unpaid, "the instance owes more than it holds");
        vm.prank(SELLER);
        inst.sweep(SELLER_PAYOUT, address(ft));
        assertEq(ft.bal(SELLER_PAYOUT), A, "the seller's payout is paid twice");
        assertEq(ft.bal(address(inst)), 0, "out of the judge's P, which nothing backs now");
    }
}
