// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Vm, VmSafe} from "forge-std/Vm.sol";
import {EravonBase} from "../EravonBase.sol";
import {Eravon} from "../../src/Eravon.sol";
import {IGaleedVenue} from "../../src/IGaleedVenue.sol";
import {Company} from "../mocks/Mocks.sol";

/// Round 6: the funding bond and the deposit address (standard s4 step 5, s7a, s8, s10), from every attacker's side:
/// an order naming money that is not bound to the seller, the zero deposit with and without a bond, a funder's
/// allowance cleared after binding or after the case opened, one funder for several sellers and cases, a company
/// seller through every terminal of s8 with its deposit at a bound funder, the push when sellerFund is a payout,
/// acceptFunding through an instance, by a contract and by a delegated key, and the shape of Pulled and FundBound.
contract EravonRound6 is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    address internal constant JUDGE_PAYOUT = address(0x1000000000000000000000000000000000000002);
    bytes32 internal constant AMANAH = keccak256("amanah word of the judge");
    address internal constant STRANGER = address(0x5757);
    address internal constant OTHER_SELLER = address(0x07E4);
    bytes internal constant ENTRY = "round 6 entry";
    bytes32 internal constant PULLED_TOPIC = keccak256("Pulled(address,address,address,address,uint256)");
    bytes32 internal constant FUNDBOUND_TOPIC = keccak256("FundBound(address,address)");

    function setUp() public {
        setUpBase();
    }

    // ---- helpers ----

    function _q(bool auto_) internal view returns (Q memory q, bytes memory quote) {
        q = defaultQuote(address(tok), JUDGE_PAYOUT, auto_);
        quote = encodeQuote(q);
    }

    /// Money and an allowance to the logic at `fund` (pranked, so even where no code could approve).
    function _fund(address fund, uint256 d, bool mint) internal {
        if (mint) tok.mint(fund, d);
        uint256 have = tok.allowance(fund, address(logic));
        vm.prank(fund);
        tok.approve(address(logic), have + d);
    }

    function _bind(address fund, address seller) internal {
        vm.prank(fund);
        logic.acceptFunding(seller);
    }

    function _initBy(Eravon inst, Eravon.OrderTerms memory t, bytes memory quote) internal {
        vm.prank(t.offer.sellerKey);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
    }

    function _countLogs(VmSafe.Log[] memory logs, address emitter, bytes32 topic0) internal pure returns (uint256 n) {
        for (uint256 k = 0; k < logs.length; k++) {
            if (logs[k].emitter == emitter && logs[k].topics.length != 0 && logs[k].topics[0] == topic0) n++;
        }
    }

    // ---- the gate: money not bound to the seller ----

    /// Every address an attacker could name as sellerFund that has not bound itself to the seller is refused by
    /// FundUnbound, the deposit at zero or not, money and an allowance standing there or not: a stranger, another
    /// seller's funder (a contract), the seller's payout, the buyer's payout, the buyer's key, the logic, the
    /// verifier, another case's instance holding A, the judge. Nothing moves; the buyer's money leaves by the expiry
    /// branch. Those that can bind themselves then open the case and pay the deposit; an instance cannot bind
    /// (NotRegistry), and the logic and the verifier never send acceptFunding.
    function test_R6_moneyNotBoundToTheSellerNeverOpensACase() public {
        (Q memory q, bytes memory quote) = _q(true);
        uint256 a = 100 ether;
        uint256 d = depositOf(q, a);
        Eravon.OrderTerms memory tOther = namedTerms(JUDGE, AMANAH, quote, a);
        fundDeposit(d);
        Eravon other = openNamed(tOther, quote);
        Company cf = new Company();
        cf.exec(address(logic), abi.encodeCall(Eravon.acceptFunding, (OTHER_SELLER)));
        address[9] memory fs =
            [STRANGER, address(cf), SELLER_PAYOUT, BUYER_PAYOUT, BUYER, address(logic), address(verifier), address(other), JUDGE];
        Q memory q0 = _q0();
        bytes memory quote0 = encodeQuote(q0);
        Eravon[9] memory insts;
        Eravon[9] memory insts0;
        Eravon.OrderTerms[9] memory ts;
        Eravon.OrderTerms[9] memory ts0;
        for (uint256 k = 0; k < 9; k++) {
            ts[k] = namedTerms(JUDGE, AMANAH, quote, a);
            ts[k].offer.sellerFund = fs[k];
            ts0[k] = namedTerms(JUDGE, AMANAH, quote0, a);
            ts0[k].offer.sellerFund = fs[k];
            bool canHold = fs[k] != address(logic) && fs[k] != address(verifier);
            _fund(fs[k], d, canHold);
            insts[k] = deployProxy(ts[k]);
            tok.mint(address(insts[k]), a);
            insts0[k] = deployProxy(ts0[k]);
            tok.mint(address(insts0[k]), a);
            assertFalse(logic.funds(SELLER, fs[k]));
            uint256 fb = tok.balanceOf(fs[k]);
            uint256 fa = tok.allowance(fs[k], address(logic));
            vm.prank(SELLER);
            vm.expectRevert(Eravon.FundUnbound.selector);
            insts[k].init(ts[k], quote, buyerSig(ts[k]), sellerSig(ts[k]), new address[](0), new bytes32[](0));
            // the zero deposit is no exemption
            vm.prank(SELLER);
            vm.expectRevert(Eravon.FundUnbound.selector);
            insts0[k].init(ts0[k], quote0, buyerSig(ts0[k]), sellerSig(ts0[k]), new address[](0), new bytes32[](0));
            // the buyer opening (buyerMayOpen) meets the same gate
            vm.prank(BUYER);
            vm.expectRevert(Eravon.FundUnbound.selector);
            insts[k].init(ts[k], quote, buyerSig(ts[k]), sellerSig(ts[k]), new address[](0), new bytes32[](0));
            assertEq(tok.balanceOf(fs[k]), fb, "nothing left the address");
            assertEq(tok.allowance(fs[k], address(logic)), fa, "no allowance spent");
            assertEq(uint256(phaseOf(insts[k])), 0);
        }
        // an instance cannot bind itself, the door being the registry's alone
        vm.prank(STRANGER);
        vm.expectRevert(Eravon.NotRegistry.selector);
        other.acceptFunding(SELLER);
        vm.prank(address(other));
        vm.expectRevert(Eravon.NotRegistry.selector);
        other.acceptFunding(SELLER);
        assertFalse(logic.funds(SELLER, address(other)));
        // those that can, bind themselves; the case opens, the deposit leaves that address alone, Pulled names it
        for (uint256 k = 0; k < 9; k++) {
            if (k == 5 || k == 6 || k == 7) continue;
            if (k == 1) cf.exec(address(logic), abi.encodeCall(Eravon.acceptFunding, (SELLER)));
            else _bind(fs[k], SELLER);
            uint256 fb = tok.balanceOf(fs[k]);
            uint256 kb = tok.balanceOf(SELLER);
            vm.recordLogs();
            _initBy(insts[k], ts[k], quote);
            VmSafe.Log[] memory logs = vm.getRecordedLogs();
            assertEq(_countLogs(logs, address(logic), PULLED_TOPIC), 1);
            for (uint256 l = 0; l < logs.length; l++) {
                if (logs[l].topics.length == 0 || logs[l].topics[0] != PULLED_TOPIC) continue;
                assertEq(logs[l].topics.length, 3);
                assertEq(address(uint160(uint256(logs[l].topics[1]))), SELLER, "Pulled's seller");
                assertEq(address(uint160(uint256(logs[l].topics[2]))), fs[k], "Pulled's fund");
                (address tk, address instance, uint256 amount) = abi.decode(logs[l].data, (address, address, uint256));
                assertEq(tk, address(tok));
                assertEq(instance, address(insts[k]));
                assertEq(amount, d);
            }
            assertEq(tok.balanceOf(fs[k]), fb - d, "the deposit left the fund");
            assertEq(tok.balanceOf(SELLER), kb, "the key paid nothing");
            // the zero-deposit order opens with no Pulled
            vm.recordLogs();
            _initBy(insts0[k], ts0[k], quote0);
            assertEq(_countLogs(vm.getRecordedLogs(), address(logic), PULLED_TOPIC), 0);
            assertEq(uint256(phaseOf(insts0[k])), 1);
        }
        // the three that can never bind leave by the expiry branch, the buyer refunded in full
        vm.warp(ts[5].openBy + 1);
        for (uint256 k = 5; k <= 7; k++) {
            uint256 bp = tok.balanceOf(BUYER_PAYOUT);
            vm.prank(BUYER);
            insts[k].init(ts[k], quote, "", "", new address[](0), new bytes32[](0));
            assertEq(tok.balanceOf(BUYER_PAYOUT), bp + a, "refunded by the expiry branch");
        }
    }

    function _q0() internal view returns (Q memory q0) {
        q0 = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q0.premiumBps = 0;
        q0.feeBps = 0;
        q0.feeFixed = 0;
        q0.feeMin = 0;
        q0.feeMax = 0;
    }

    /// The instance's own address cannot be named: the instance is the CREATE2 address of the order's hash, and
    /// the order carries sellerFund, so naming it is a fixed point of keccak256; every candidate tried derives
    /// another instance.
    function test_R6_theInstanceItselfCannotBeNamed() public {
        (, bytes memory quote) = _q(true);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 1 ether);
        for (uint256 k = 0; k < 16; k++) {
            address guess = instanceOf(t);
            t.offer.sellerFund = guess;
            assertTrue(instanceOf(t) != guess, "a fixed point");
        }
    }

    // ---- the allowance as a pool ----

    /// One funder for two sellers and three cases: each case pulls its D from the one allowance, a fourth case finds
    /// the pool short (TransferFailed) while a zero-deposit case still opens; a funder that clears its allowance after
    /// binding stops every later case and none already open; a terminal returns D to the funder's balance whatever
    /// its allowance; a stranger that binds itself to a seller moves nothing for an order naming the seller's key.
    function test_R6_oneFunderSeveralSellersAndCasesAndItsAllowance() public {
        (Q memory q, bytes memory quote) = _q(true);
        address F = address(0xF00D);
        uint256[4] memory amt = [uint256(100 ether), 40 ether, 7 ether, 50 ether];
        uint256 dsum = depositOf(q, amt[0]) + depositOf(q, amt[1]) + depositOf(q, amt[2]);
        _bind(F, SELLER);
        _bind(F, OTHER_SELLER);
        _fund(F, dsum, true);
        Eravon[4] memory inst;
        Eravon.OrderTerms[4] memory t;
        for (uint256 k = 0; k < 4; k++) {
            t[k] = namedTerms(JUDGE, AMANAH, quote, amt[k]);
            t[k].offer.sellerFund = F;
            // the second case is the other seller's, a key that consents by its own call
            if (k == 1) t[k].offer.sellerKey = OTHER_SELLER;
            inst[k] = deployProxy(t[k]);
            tok.mint(address(inst[k]), amt[k]);
        }
        for (uint256 k = 0; k < 3; k++) {
            if (k == 1) {
                vm.prank(OTHER_SELLER);
                inst[k].init(t[k], quote, buyerSig(t[k]), "", new address[](0), new bytes32[](0));
            } else {
                _initBy(inst[k], t[k], quote);
            }
        }
        assertEq(tok.balanceOf(F), 0);
        assertEq(tok.allowance(F, address(logic)), 0);
        tok.mint(F, depositOf(q, amt[3]));
        vm.prank(SELLER);
        vm.expectRevert(Eravon.TransferFailed.selector);
        inst[3].init(t[3], quote, buyerSig(t[3]), sellerSig(t[3]), new address[](0), new bytes32[](0));
        // a zero-deposit order needs no allowance
        Q memory q0 = _q0();
        bytes memory quote0 = encodeQuote(q0);
        Eravon.OrderTerms memory tz = namedTerms(JUDGE, AMANAH, quote0, 5 ether);
        tz.offer.sellerFund = F;
        Eravon iz = deployProxy(tz);
        tok.mint(address(iz), 5 ether);
        _initBy(iz, tz, quote0);
        // allowance restored, then cleared again after the open: the open case is untouched and returns D to F
        vm.prank(F);
        tok.approve(address(logic), depositOf(q, amt[3]));
        _initBy(inst[3], t[3], quote);
        vm.prank(F);
        tok.approve(address(logic), 0);
        uint256 fb = tok.balanceOf(F);
        vm.prank(SELLER);
        inst[3].refund();
        assertEq(tok.balanceOf(F), fb + depositOf(q, amt[3]), "D back to the funder with no allowance standing");
        assertEq(tok.allowance(F, address(logic)), 0, "a returned deposit refills the balance, not the allowance");
        // a stranger binding itself to the seller changes nothing for an order whose fund is the key
        _bind(STRANGER, SELLER);
        _fund(STRANGER, 1000 ether, true);
        Eravon.OrderTerms memory tk = namedTerms(JUDGE, AMANAH, quote, 10 ether);
        fundDeposit(depositOf(q, 10 ether));
        uint256 sb = tok.balanceOf(STRANGER);
        vm.recordLogs();
        Eravon ik = openNamed(tk, quote);
        VmSafe.Log[] memory logs = vm.getRecordedLogs();
        for (uint256 l = 0; l < logs.length; l++) {
            if (logs[l].topics.length == 3 && logs[l].topics[0] == PULLED_TOPIC) {
                assertEq(address(uint160(uint256(logs[l].topics[2]))), SELLER, "the key's own money");
            }
        }
        assertEq(tok.balanceOf(STRANGER), sb, "the stranger's money untouched");
        assertEq(uint256(phaseOf(ik)), 1);
    }

    /// A funder bound in the same block as the init that names it: the init ahead of the bond is FundUnbound and
    /// leaves the order to open later; the init after it opens.
    function test_R6_aBondAndAnInitInOneBlock() public {
        (Q memory q, bytes memory quote) = _q(true);
        address F = address(0xF00E);
        uint256 a = 30 ether;
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, a);
        t.offer.sellerFund = F;
        _fund(F, depositOf(q, a), true);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), a);
        uint256 s = vm.snapshotState();
        // the init first
        vm.prank(SELLER);
        vm.expectRevert(Eravon.FundUnbound.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        _bind(F, SELLER);
        _initBy(inst, t, quote);
        uint256 afterBoth = tok.balanceOf(F);
        vm.revertToState(s);
        // the bond first
        _bind(F, SELLER);
        _initBy(inst, t, quote);
        assertEq(tok.balanceOf(F), afterBoth, "the same pull in either order once both landed");
    }

    /// What a bond hands the seller: the fund's whole allowance, spendable at once. A bound seller, its own sock as
    /// the buyer and a judge whose quote it chose (premiumBps 10000, no litigation fee, consent carried) open and
    /// release one case in one block: the deposit D = P = A is pulled from the funder, A returns to the seller's
    /// payout, and P, the funder's money, is the judge's to withdraw. Nothing in s7a's account of the pool (D held until
    /// the case ends; exposed to buyers under a public listing) names this.
    function test_R6_aBoundSellerSpendsTheFundersWholeAllowanceInOneBlock() public {
        address F = address(0xF0F1);
        uint256 n = 1000 ether;
        Q memory q = defaultQuote(address(tok), JUDGE_PAYOUT, true);
        q.premiumBps = 10000;
        q.feeBps = 0;
        q.feeMin = 0;
        q.feeMax = 0;
        bytes memory quote = encodeQuote(q);
        _bind(F, SELLER);
        _fund(F, n, true);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, n);
        t.offer.sellerFund = F;
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), n); // the sock buyer's A
        uint256 sp = tok.balanceOf(SELLER_PAYOUT);
        _initBy(inst, t, quote);
        claim(inst, keccak256("nothing"));
        vm.prank(BUYER);
        inst.confirm();
        (, uint256 cl) = inst.accountOf(JUDGE, address(tok));
        assertEq(tok.balanceOf(F), 0, "the funder's whole allowance left");
        assertEq(tok.balanceOf(SELLER_PAYOUT), sp + n, "the sock's A back at the seller's payout");
        assertEq(cl, n, "the funder's money is the chosen judge's");
        vm.prank(JUDGE);
        inst.withdraw(address(tok), JUDGE, JUDGE_PAYOUT);
        assertEq(tok.balanceOf(JUDGE_PAYOUT), n);
    }

    // ---- acceptFunding by every kind of sender ----

    /// The bond is the sender's: a contract funder's call, a delegated key's call through its code, and a plain key's
    /// call each bind that sender and nobody else; FundBound names (seller, fund) as topics on the logic; a second
    /// call emits again and changes nothing; acceptFunding names no amount and moves nothing.
    function test_R6_acceptFundingByAContractADelegatedKeyAndAKey() public {
        Company cf = new Company();
        address delegated = address(0xDE1E);
        vm.etch(delegated, address(new Company()).code);
        vm.recordLogs();
        cf.exec(address(logic), abi.encodeCall(Eravon.acceptFunding, (SELLER)));
        Company(delegated).exec(address(logic), abi.encodeCall(Eravon.acceptFunding, (SELLER)));
        vm.prank(STRANGER);
        logic.acceptFunding(SELLER);
        vm.prank(STRANGER);
        logic.acceptFunding(SELLER);
        VmSafe.Log[] memory logs = vm.getRecordedLogs();
        assertEq(_countLogs(logs, address(logic), FUNDBOUND_TOPIC), 4);
        address[4] memory who = [address(cf), delegated, STRANGER, STRANGER];
        uint256 n;
        for (uint256 l = 0; l < logs.length; l++) {
            if (logs[l].topics.length == 0 || logs[l].topics[0] != FUNDBOUND_TOPIC) continue;
            assertEq(logs[l].topics.length, 3);
            assertEq(address(uint160(uint256(logs[l].topics[1]))), SELLER);
            assertEq(address(uint160(uint256(logs[l].topics[2]))), who[n++]);
            assertEq(logs[l].data.length, 0);
        }
        assertTrue(logic.funds(SELLER, address(cf)));
        assertTrue(logic.funds(SELLER, delegated));
        assertTrue(logic.funds(SELLER, STRANGER));
        assertFalse(logic.funds(address(cf), SELLER), "the bond has a direction");
        assertFalse(logic.funds(OTHER_SELLER, STRANGER));
        assertEq(tok.balanceOf(address(logic)), 0);
    }

    // ---- a company seller through every terminal ----

    struct Want {
        uint256 buyer; // to the buyer's payout
        uint256 seller; // to the seller's payout (its share of A)
        uint256 fund; // back to sellerFund
        uint256 judge; // to the judge's claimable
    }

    Company internal co; // the company seller
    Company internal cfund; // its wallet, bound to it
    uint256 internal A_ = 1000 ether;

    function _companyOpen(bool auto_) internal returns (Eravon inst, Eravon.OrderTerms memory t, Q memory q) {
        co = new Company();
        cfund = new Company();
        cfund.exec(address(logic), abi.encodeCall(Eravon.acceptFunding, (address(co))));
        q = defaultQuote(address(tok), JUDGE_PAYOUT, auto_);
        q.premiumBps = 250;
        bytes memory quote = encodeQuote(q);
        t = namedTerms(JUDGE, AMANAH, quote, A_);
        t.offer.sellerKey = address(co);
        t.offer.sellerFund = address(cfund);
        uint256 d = depositOf(q, A_);
        tok.mint(address(cfund), d);
        cfund.exec(address(tok), abi.encodeWithSignature("approve(address,uint256)", address(logic), d));
        inst = deployProxy(t);
        tok.mint(address(inst), A_);
        co.exec(address(inst), abi.encodeCall(Eravon.init, (t, quote, buyerSig(t), bytes(""), new address[](0), new bytes32[](0))));
        assertEq(tok.balanceOf(address(cfund)), 0);
        IGaleedVenue.Seat memory s0 = inst.seatOf(orderHash(t), 0);
        assertEq(s0.parties.length, 5);
        assertEq(s0.parties[0], BUYER);
        assertEq(s0.parties[1], address(co));
        assertEq(s0.parties[2], address(cfund));
        assertEq(s0.parties[3], BUYER_PAYOUT);
        assertEq(s0.parties[4], SELLER_PAYOUT);
    }

    function _accept(Eravon inst, bytes32 id) internal {
        vm.prank(JUDGE, JUDGE);
        inst.accept(id, 0);
    }

    function _claim(Eravon inst) internal {
        co.exec(address(inst), abi.encodeCall(Eravon.claimDelivery, (keccak256("goods"))));
    }

    function _dispute(Eravon inst) internal {
        (uint256 lf,,,,) = inst.feeQuoted();
        tok.mint(BUYER, lf);
        vm.prank(BUYER);
        tok.approve(address(inst), lf);
        vm.prank(BUYER);
        inst.dispute();
    }

    function _settle(Eravon inst, uint16 bps) internal {
        uint256 exp = vm.getBlockTimestamp() + 1 days;
        bytes32 d = digestOf(address(inst), keccak256(abi.encode(TH_SETTLE, uint256(bps), exp)));
        co.exec(address(inst), abi.encodeCall(Eravon.settle, (bps, exp, sign(BUYER_PK, d), bytes(""))));
    }

    /// Runs one row of s8 on an opened company case and returns what the table gives each side.
    function _row(uint8 row, Eravon inst, Eravon.OrderTerms memory t, Q memory q) internal returns (Want memory w) {
        bytes32 id = orderHash(t);
        uint256 p = premiumOf(q, A_);
        uint256 lf = lfOf(q, A_);
        bytes32 eid = sha256(ENTRY);
        if (row == 0) {
            // Refunded cause 1: the offer clock ran out (named track, consent not carried)
            vm.warp(vm.getBlockTimestamp() + q.offerWindow + 1);
            inst.advance(id, 0);
            w = Want(A_, 0, p + lf, 0);
        } else if (row == 1) {
            // Resigned form 0: the named judge declines
            vm.prank(JUDGE, JUDGE);
            inst.resign(id, 0, eid, ENTRY);
            w = Want(A_, 0, p + lf, 0);
        } else if (row == 2) {
            // Refunded cause 0: T1 ran out
            _accept(inst, id);
            vm.warp(vm.getBlockTimestamp() + t.offer.deliveryWindow + 1);
            inst.advance(id, 0);
            w = Want(A_, 0, p + lf, 0);
        } else if (row == 3) {
            // Resigned form 1 while Funded
            _accept(inst, id);
            vm.prank(JUDGE, JUDGE);
            inst.resign(id, 0, eid, ENTRY);
            w = Want(A_, 0, p + lf, 0);
        } else if (row == 4) {
            // Refunded cause 3 while Funded
            _accept(inst, id);
            co.exec(address(inst), abi.encodeCall(Eravon.refund, ()));
            w = Want(A_, 0, p + lf, 0);
        } else if (row == 5) {
            // Refunded cause 3 while Claimed under a seated seat: P to the judge
            _accept(inst, id);
            _claim(inst);
            co.exec(address(inst), abi.encodeCall(Eravon.refund, ()));
            w = Want(A_, 0, lf, p);
        } else if (row == 6) {
            // Released, seat seated
            _accept(inst, id);
            _claim(inst);
            vm.prank(BUYER);
            inst.confirm();
            w = Want(0, A_, lf, p);
        } else if (row == 7) {
            // Released by T2 after the seat resigned while Claimed
            _accept(inst, id);
            _claim(inst);
            vm.prank(JUDGE, JUDGE);
            inst.resign(id, 0, eid, ENTRY);
            vm.warp(vm.getBlockTimestamp() + t.offer.confirmWindow + 1);
            inst.advance(id, 0);
            w = Want(0, A_, p + lf, 0);
        } else if (row == 8) {
            // Settled under an accepted seat in its window
            _accept(inst, id);
            _settle(inst, 3000);
            w = Want((A_ * 3000) / 10000, A_ - (A_ * 3000) / 10000, lf, p);
        } else if (row == 9) {
            // Settled with the seat unaccepted
            _settle(inst, 7000);
            w = Want((A_ * 7000) / 10000, A_ - (A_ * 7000) / 10000, p + lf, 0);
        } else if (row == 10 || row == 11 || row == 12) {
            // Ruled at rung 2 (feeBearer 1), 1 (feeBearer 2), 0 (feeBearer 0)
            _accept(inst, id);
            _claim(inst);
            _dispute(inst);
            uint64 rung = row == 10 ? 2 : (row == 11 ? 1 : 0);
            vm.prank(JUDGE, JUDGE);
            inst.rule(id, 0, eid, rung, ENTRY, "");
            uint256 bps = t.offer.ladder[rung].buyerBps;
            uint8 fb = t.offer.ladder[rung].feeBearer;
            uint256 bond = fb == 1 ? lf : (fb == 2 ? lf / 2 : 0);
            uint256 ba = (A_ * bps) / 10000;
            w = Want(ba + bond, A_ - ba, lf - bond, p + lf);
        } else if (row == 13) {
            // Lapsed by T3 (lapseDefault 2: the last rung), the buyer's fee back
            _accept(inst, id);
            _claim(inst);
            _dispute(inst);
            vm.warp(vm.getBlockTimestamp() + q.rulingWindow + 1);
            inst.advance(id, 0);
            w = Want(A_ + lf, 0, p + lf, 0);
        } else if (row == 14) {
            // Lapsed by a resignation during the dispute
            _accept(inst, id);
            _claim(inst);
            _dispute(inst);
            vm.prank(JUDGE, JUDGE);
            inst.resign(id, 0, eid, ENTRY);
            w = Want(A_ + lf, 0, p + lf, 0);
        } else if (row == 15) {
            // Lapsed by a dispute after the seat resigned while Claimed: no fee pulled
            _accept(inst, id);
            _claim(inst);
            vm.prank(JUDGE, JUDGE);
            inst.resign(id, 0, eid, ENTRY);
            vm.prank(BUYER);
            inst.dispute();
            w = Want(A_, 0, p + lf, 0);
        } else {
            // Refunded cause 3 past T3's end: the plain refund, the buyer's fee back
            _accept(inst, id);
            _claim(inst);
            _dispute(inst);
            vm.warp(vm.getBlockTimestamp() + q.rulingWindow + 1);
            co.exec(address(inst), abi.encodeCall(Eravon.refund, ()));
            w = Want(A_ + lf, 0, p + lf, 0);
        }
    }

    /// A company seller (a contract key consenting by call) whose capital is in a wallet bound to it: at every row
    /// of s8 the deposit's returning part lands at the wallet, the seller's share of A at sellerPayout, P where the
    /// row says, and the company's own address receives nothing; Closed reports the seller's side as the two
    /// together.
    function test_R6_aCompanySellerThroughEveryTerminal() public {
        for (uint8 row = 0; row <= 16; row++) {
            uint256 s = vm.snapshotState();
            (Eravon inst, Eravon.OrderTerms memory t, Q memory q) = _companyOpen(false);
            uint256 bp = tok.balanceOf(BUYER_PAYOUT);
            uint256 sp = tok.balanceOf(SELLER_PAYOUT);
            uint256 fb = tok.balanceOf(address(cfund));
            vm.recordLogs();
            Want memory w = _row(row, inst, t, q);
            VmSafe.Log[] memory logs = vm.getRecordedLogs();
            assertEq(tok.balanceOf(BUYER_PAYOUT) - bp, w.buyer, string(abi.encodePacked("buyer, row ", vm.toString(row))));
            assertEq(tok.balanceOf(SELLER_PAYOUT) - sp, w.seller, string(abi.encodePacked("seller, row ", vm.toString(row))));
            assertEq(tok.balanceOf(address(cfund)) - fb, w.fund, string(abi.encodePacked("fund, row ", vm.toString(row))));
            assertEq(tok.balanceOf(address(co)), 0, "the company's address receives nothing");
            assertEq(inst.owed(address(co), address(tok)), 0);
            (, uint256 cl) = inst.accountOf(JUDGE, address(tok));
            assertEq(cl, w.judge, string(abi.encodePacked("judge, row ", vm.toString(row))));
            assertEq(tok.balanceOf(address(inst)), cl, "only the judge's claimable stays");
            bool seen;
            for (uint256 l = 0; l < logs.length; l++) {
                if (logs[l].emitter != address(inst) || logs[l].topics.length == 0 || logs[l].topics[0] != Eravon.Closed.selector) continue;
                (, uint256 b, uint256 sl, uint256 j,) = abi.decode(logs[l].data, (uint8, uint256, uint256, uint256, uint256));
                assertEq(b, w.buyer);
                assertEq(sl, w.seller + w.fund, "sellerPaid is the share of A and the deposit returned");
                assertEq(j, w.judge);
                seen = true;
            }
            assertTrue(seen, "Closed");
            vm.revertToState(s);
        }
    }

    // ---- sellerFund at a payout: the push ----

    /// sellerFund = sellerPayout (bound): the seller's share of A and the deposit's return are one credit and one
    /// push (one Swept); sellerFund = buyerPayout (bound): the buyer's side and the deposit's return are one push, and
    /// the second push the terminal tries finds nothing owed; a blocked payout keeps the whole sum owed and sweep
    /// pays it once.
    function test_R6_sellerFundAtAPayoutIsOnePush() public {
        (Q memory q, bytes memory quote) = _q(true);
        uint256 a = 200 ether;
        uint256 p = premiumOf(q, a);
        uint256 lf = lfOf(q, a);
        // at the seller's payout, Released under the seated seat
        _bind(SELLER_PAYOUT, SELLER);
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, a);
        t.offer.sellerFund = SELLER_PAYOUT;
        _fund(SELLER_PAYOUT, p + lf, true);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), a);
        _initBy(inst, t, quote);
        claim(inst, keccak256("m"));
        uint256 sp = tok.balanceOf(SELLER_PAYOUT);
        vm.recordLogs();
        vm.prank(BUYER);
        inst.confirm();
        VmSafe.Log[] memory logs = vm.getRecordedLogs();
        uint256 swept;
        for (uint256 l = 0; l < logs.length; l++) {
            if (logs[l].emitter == address(inst) && logs[l].topics[0] == Eravon.Swept.selector) {
                assertEq(address(uint160(uint256(logs[l].topics[1]))), SELLER_PAYOUT);
                (, uint256 v) = abi.decode(logs[l].data, (address, uint256));
                assertEq(v, a + lf, "one push of A and the LF");
                swept++;
            }
        }
        assertEq(swept, 1, "one push");
        assertEq(tok.balanceOf(SELLER_PAYOUT), sp + a + lf);
        // at the buyer's payout, refunded while Funded: A and D in one push
        _bind(BUYER_PAYOUT, SELLER);
        Eravon.OrderTerms memory t2 = namedTerms(JUDGE, AMANAH, quote, a);
        t2.offer.sellerFund = BUYER_PAYOUT;
        _fund(BUYER_PAYOUT, p + lf, true);
        Eravon inst2 = deployProxy(t2);
        tok.mint(address(inst2), a);
        _initBy(inst2, t2, quote);
        uint256 bp = tok.balanceOf(BUYER_PAYOUT);
        vm.recordLogs();
        vm.prank(SELLER);
        inst2.refund();
        logs = vm.getRecordedLogs();
        swept = 0;
        for (uint256 l = 0; l < logs.length; l++) {
            if (logs[l].emitter == address(inst2) && logs[l].topics[0] == Eravon.Swept.selector) {
                (, uint256 v) = abi.decode(logs[l].data, (address, uint256));
                assertEq(v, a + p + lf);
                swept++;
            }
        }
        assertEq(swept, 1, "one push, the second finding nothing owed");
        assertEq(tok.balanceOf(BUYER_PAYOUT), bp + a + p + lf);
    }
}
