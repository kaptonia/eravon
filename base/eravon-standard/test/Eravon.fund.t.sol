// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";

/// Standard s7a as revised on 2026-10-18: the deposit is pulled from the order's `sellerFund`, which is the
/// seller's key or an address that bound itself to the seller on the logic by `acceptFunding`; `init` tests the
/// bond after the consents and before the balance, whether or not the deposit is zero; the deposit returns to
/// `sellerFund`; `pull` carries both addresses and `Pulled` names both.
contract EravonFund is EravonBase {
    address internal constant JUDGE = address(0x1000000000000000000000000000000000000001);
    bytes32 internal constant AMANAH = keccak256("amanah word of the judge");
    address internal constant WALLET = address(0xC0A1E7); // a company's wallet, funding the seller SELLER
    address internal constant OTHER = address(0x07E4);

    function setUp() public {
        setUpBase();
    }

    function _companyTerms(bytes memory quote, uint256 amount_) internal returns (Eravon.OrderTerms memory t) {
        t = namedTerms(JUDGE, AMANAH, quote, amount_);
        t.offer.sellerFund = WALLET;
    }

    function _fundWallet(uint256 d) internal {
        tok.mint(WALLET, d);
        vm.prank(WALLET);
        tok.approve(address(logic), d);
    }

    /// acceptFunding: on the logic alone, under the sender, only ever set; the view answers on the logic.
    function test_acceptFunding_registry() public {
        assertFalse(logic.funds(SELLER, WALLET));
        vm.prank(WALLET);
        vm.expectEmit(true, true, true, true, address(logic));
        emit Eravon.FundBound(SELLER, WALLET);
        logic.acceptFunding(SELLER);
        assertTrue(logic.funds(SELLER, WALLET));
        assertFalse(logic.funds(WALLET, SELLER), "the bond has a direction");
        assertFalse(logic.funds(OTHER, WALLET));
        assertFalse(logic.funds(SELLER, OTHER));
        // a second acceptance changes nothing and is admitted
        vm.prank(WALLET);
        logic.acceptFunding(SELLER);
        assertTrue(logic.funds(SELLER, WALLET));
        // through an instance the door is the registry's alone
        Eravon inst = deployProxy(namedTerms(JUDGE, AMANAH, encodeQuote(defaultQuote(address(tok), address(0), true)), 1 ether));
        vm.prank(WALLET);
        vm.expectRevert(Eravon.NotRegistry.selector);
        inst.acceptFunding(SELLER);
        assertFalse(inst.funds(SELLER, WALLET), "an instance holds no registry");
    }

    /// A company seller: sellerFund bound, the deposit pulled from the wallet, Pulled naming both, the deposit
    /// returning to the wallet at a terminal that returns it, the key untouched.
    function test_init_companySeller_pullsFromTheWallet() public {
        Q memory q = defaultQuote(address(tok), address(0), true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = _companyTerms(quote, 100 ether);
        uint256 d = depositOf(q, 100 ether);
        _fundWallet(d);
        vm.prank(WALLET);
        logic.acceptFunding(SELLER);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        uint256 keyBefore = tok.balanceOf(SELLER);
        vm.prank(SELLER);
        vm.expectEmit(true, true, true, true, address(logic));
        emit Eravon.Pulled(SELLER, WALLET, address(tok), address(inst), d);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        assertEq(tok.balanceOf(WALLET), 0, "the wallet's deposit left");
        assertEq(tok.balanceOf(SELLER), keyBefore, "the key paid nothing");
        assertEq(inst.seatOf(orderHash(t), 0).parties.length, 5);
        assertEq(inst.seatOf(orderHash(t), 0).parties[2], WALLET);
        // the seller refunds while Funded: A to the buyer, the deposit back to the wallet, nothing to the key
        vm.prank(SELLER);
        inst.refund();
        assertEq(tok.balanceOf(WALLET), d, "the deposit returned to sellerFund");
        assertEq(tok.balanceOf(SELLER), keyBefore);
        assertEq(tok.balanceOf(BUYER_PAYOUT), 100 ether);
    }

    /// Without the bond, init is FundUnbound after the consents and before the balance test, whether or not the
    /// deposit is zero; the wallet's allowance alone binds nothing.
    function test_init_unboundFund_isRefused() public {
        Q memory q = defaultQuote(address(tok), address(0), true);
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = _companyTerms(quote, 100 ether);
        uint256 d = depositOf(q, 100 ether);
        _fundWallet(d);
        Eravon inst = deployProxy(t);
        // the balance is short too: the bond is tested first, so FundUnbound and not Underfunded
        vm.prank(SELLER);
        vm.expectRevert(Eravon.FundUnbound.selector);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        // a wrong signature is decided before the bond
        vm.prank(SELLER);
        vm.expectRevert(Eravon.BadBuyerSig.selector);
        inst.init(t, quote, sellerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        // a deposit of zero is no exemption
        Q memory q0 = defaultQuote(address(tok), address(0), true);
        q0.premiumBps = 0;
        q0.feeBps = 0;
        q0.feeFixed = 0;
        q0.feeMin = 0;
        bytes memory quote0 = encodeQuote(q0);
        Eravon.OrderTerms memory t0 = _companyTerms(quote0, 100 ether);
        Eravon inst0 = deployProxy(t0);
        tok.mint(address(inst0), 100 ether);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.FundUnbound.selector);
        inst0.init(t0, quote0, buyerSig(t0), sellerSig(t0), new address[](0), new bytes32[](0));
        // a bond to another seller binds nothing for this one
        vm.prank(WALLET);
        logic.acceptFunding(OTHER);
        vm.prank(SELLER);
        vm.expectRevert(Eravon.FundUnbound.selector);
        inst0.init(t0, quote0, buyerSig(t0), sellerSig(t0), new address[](0), new bytes32[](0));
        // bound: the zero-deposit case opens and no Pulled is emitted
        vm.prank(WALLET);
        logic.acceptFunding(SELLER);
        vm.prank(SELLER);
        inst0.init(t0, quote0, buyerSig(t0), sellerSig(t0), new address[](0), new bytes32[](0));
        assertEq(uint256(phaseOf(inst0)), uint256(Eravon.Phase.Funded));
    }

    /// A seller's own key needs no bond; a stranger's bond to the seller does not let an order name the stranger's
    /// money unless the order says so, and an order naming the stranger's address pulls from it alone.
    function test_pull_onlyBoundFund() public {
        Q memory q = defaultQuote(address(tok), address(0), true);
        bytes memory quote = encodeQuote(q);
        uint256 d = depositOf(q, 100 ether);
        // the key as its own fund: no bond, the deposit from the key
        Eravon.OrderTerms memory t = namedTerms(JUDGE, AMANAH, quote, 100 ether);
        assertEq(t.offer.sellerFund, SELLER);
        fundDeposit(q, 100 ether);
        Eravon inst = openNamed(t, quote);
        assertEq(uint256(phaseOf(inst)), uint256(Eravon.Phase.Funded));
        // pull by hand: only an instance, only to itself, from the fund it names
        tok.mint(WALLET, 10 ether);
        vm.prank(WALLET);
        tok.approve(address(logic), 10 ether);
        vm.prank(OTHER);
        vm.expectRevert(Eravon.NotInstance.selector);
        logic.pull(SELLER, WALLET, address(tok), 1 ether);
        vm.expectRevert(Eravon.NotRegistry.selector);
        inst.pull(SELLER, WALLET, address(tok), 1 ether);
        // an instance that pulls names the fund: the money leaves the fund, not the seller named beside it
        uint256 walletBefore = tok.balanceOf(WALLET);
        uint256 sellerBefore = tok.balanceOf(SELLER);
        vm.prank(address(inst));
        vm.expectEmit(true, true, true, true, address(logic));
        emit Eravon.Pulled(OTHER, WALLET, address(tok), address(inst), 2 ether);
        logic.pull(OTHER, WALLET, address(tok), 2 ether);
        assertEq(tok.balanceOf(WALLET), walletBefore - 2 ether);
        assertEq(tok.balanceOf(SELLER), sellerBefore);
        // a fund without allowance: TransferFailed, nothing moves
        vm.prank(address(inst));
        vm.expectRevert(Eravon.TransferFailed.selector);
        logic.pull(SELLER, OTHER, address(tok), 1 ether);
        assertTrue(d != 0);
    }

    /// A key-holding seller whose sellerFund is another key it holds: bound, every terminal returns the deposit there.
    function test_terminals_returnTheDepositToSellerFund() public {
        Q memory q = defaultQuote(address(tok), address(0), true);
        bytes memory quote = encodeQuote(q);
        uint256 d = depositOf(q, 100 ether);
        vm.prank(WALLET);
        logic.acceptFunding(SELLER);
        // Released: the deposit back to the wallet, P to the judge
        Eravon.OrderTerms memory t = _companyTerms(quote, 100 ether);
        _fundWallet(d);
        Eravon inst = deployProxy(t);
        tok.mint(address(inst), 100 ether);
        vm.prank(SELLER);
        inst.init(t, quote, buyerSig(t), sellerSig(t), new address[](0), new bytes32[](0));
        vm.prank(SELLER);
        inst.claimDelivery(keccak256("m"));
        vm.prank(BUYER);
        inst.confirm();
        uint256 p = premiumOf(q, 100 ether);
        assertEq(tok.balanceOf(WALLET), d - p, "the deposit less P back to the wallet");
        assertEq(tok.balanceOf(SELLER_PAYOUT), 100 ether);
        // Ruled at the last rung with feeBearer 1: the seller's LF to the buyer, P to the judge, nothing to the wallet
        Eravon.OrderTerms memory t2 = _companyTerms(quote, 50 ether);
        uint256 d2 = depositOf(q, 50 ether);
        _fundWallet(d2);
        uint256 w = tok.balanceOf(WALLET);
        Eravon inst2 = deployProxy(t2);
        tok.mint(address(inst2), 50 ether);
        vm.prank(SELLER);
        inst2.init(t2, quote, buyerSig(t2), sellerSig(t2), new address[](0), new bytes32[](0));
        vm.prank(SELLER);
        inst2.claimDelivery(keccak256("m"));
        (uint256 lf2,,,,) = inst2.feeQuoted();
        tok.mint(BUYER, lf2);
        vm.prank(BUYER);
        tok.approve(address(inst2), lf2);
        vm.prank(BUYER);
        inst2.dispute();
        bytes memory entry = bytes("judgment");
        bytes32 eid = sha256(entry);
        bytes32 id2 = orderHash(t2);
        vm.prank(JUDGE, JUDGE);
        inst2.rule(id2, 0, eid, 2, entry, "");
        assertEq(tok.balanceOf(WALLET), w - d2, "nothing of the deposit returned at a ruling for the buyer at feeBearer 1");
    }
}
