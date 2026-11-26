// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {IGaleedVenue, IGaleedSuccessionBook} from "./IGaleedVenue.sol";
import {IERC20} from "./interfaces.sol";
import {EravonVerifier} from "./EravonVerifier.sol";

/// ERAVON, the standard (docs/eravon-standard-v1.md).
///
/// One immutable logic contract; one EIP-1167 minimal proxy per case,
/// deployed by anyone through the public CREATE2 deployer at the address the
/// signed order determines; every instance is one galeed.venue/1 deployment.
/// No owner, no upgrade, no pause, no allow-list, no selfdestruct, no
/// privileged address anywhere. The logic holds no money: a seller's deposit
/// is an allowance the order's `sellerFund` grants on the token, pulled into
/// the instance at `init` (s7a); the logic's own storage holds the registry
/// of consent by call (s9), the registry of revocation (s4) and the registry
/// of funding bonds (s7a), nothing else. Which key acts for a party, a judge
/// or a guild is read at every act from the family's succession book (s9).
///
/// Constraints in comments only. Section numbers refer to the standard
/// document unless marked venue/1 (galeed.venue/1) or terms/1 (eravon.terms/1).
contract Eravon is IGaleedVenue {
    // ------------------------------------------------------------------
    // constants
    // ------------------------------------------------------------------

    /// The public keyless CREATE2 deployer (Shape, s1).
    address internal constant DEPLOYER = 0x4e59b44847b379578588920cA78FbF26c0B4956C;
    /// The family's succession book for chains 1 and 11155111 (Shape, s9; galeed.roster/1 s4, s9).
    address internal constant SUCCESSION_BOOK = 0x3dB7c505dd25361F1A66f6c5Bd96b48df27f668c;
    /// The gas a router is handed at a draw (s7, venue/1 s4).
    uint64 internal constant ROUTE_GAS = 1_000_000;

    // EIP-712 (s9, terms/1 s4.2, s4.5)
    bytes32 internal constant TH_DOMAIN =
        keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)");
    bytes32 internal constant NAME_HASH = keccak256("ERAVON");
    bytes32 internal constant VERSION_HASH = keccak256("1");
    bytes32 internal constant TH_CLAIM = keccak256("ClaimDelivery(bytes32 manifest)");
    bytes32 internal constant TH_CONFIRM = keccak256("Confirm()");
    bytes32 internal constant TH_DISPUTE = keccak256("Dispute()");
    bytes32 internal constant TH_REFUND = keccak256("Refund()");
    bytes32 internal constant TH_SETTLE = keccak256("Settle(uint16 buyerBps,uint256 expiry)");
    bytes32 internal constant TH_CANCEL = keccak256("Cancel()");

    /// venue/1 s2.5: the quote form, constant for the life of the standard (s3).
    string internal constant QUOTE_TYPE =
        "(address token,uint16 premiumBps,address feeUnit,address feedToken,address feedUnit,uint32 maxStale,uint256 feeFixed,uint16 feeBps,uint256 feeMin,uint256 feeMax,uint256 amountCap,uint32 rulingWindow,uint32 offerWindow,uint8 lapseDefault,bool autoAccept)";

    // proxy initcode (s1): 0x3d602d80600a3d3981f3363d3d373d3d3d363d73 || LOGIC || 0x5af43d82803e903d91602b57fd5bf3
    bytes10 internal constant INIT_PREFIX = 0x3d602d80600a3d3981f3;
    bytes10 internal constant RUNTIME_PREFIX = 0x363d3d373d3d3d363d73;
    bytes15 internal constant RUNTIME_SUFFIX = 0x5af43d82803e903d91602b57fd5bf3;

    /// The logic's own address, the seller's EIP-712 verifying contract (s9).
    address public immutable LOGIC;
    /// keccak256 of the proxy initcode that embeds LOGIC (s1).
    bytes32 internal immutable PROXY_INITCODE_HASH;
    /// keccak256 of the proxy runtime code: the standard's identity, the judge's declared codeHash.
    bytes32 public immutable PROXY_CODEHASH;
    /// The panel book's and the verifier's initcode and runtime code hashes (s7, Shape), as CODEHASH.md lists
    /// them; `pin-hashes.py` writes them from the compiled sources and the identity test recomputes them.
    // pinned hashes: begin
    bytes32 internal constant PANEL_INITCODE_HASH = 0x7a8e34943181637c413e9889d06e945dfd2ed5496eac89f445e9f099f401e864;
    bytes32 internal constant PANEL_CODEHASH = 0x91dc1b8a8092f9a24232897224f2cef9234c75b42e625d3b162092b49f5073d8;
    bytes32 internal constant VERIFIER_INITCODE_HASH = 0x1b71b7e340dd6e1882635ae2b80b0dd4726e604608169e90a40eb091291b1596;
    bytes32 internal constant VERIFIER_CODEHASH = 0x2bbc633e66fd4b81e7660302716aec28dc978145093b7cb3cd2f399ad1c4991d;
    // pinned hashes: end
    /// The chain's panel book and the verifier, designated by code at the addresses the deployer yields.
    address internal immutable PANEL_BOOK;
    address internal immutable VERIFIER;

    // ------------------------------------------------------------------
    // types
    // ------------------------------------------------------------------

    /// One rung of the ladder, EIP-712 type `Outcome` (terms/1 s4.2).
    struct Rung {
        uint16 buyerBps;
        uint16 sellerBps;
        uint8 feeBearer; // 0 buyer, 1 seller, 2 half each
    }

    struct Offer {
        address sellerKey;
        address judge;
        address guild;
        address sellerPayout;
        address sellerFund;
        address token;
        uint256 amount;
        uint32 deliveryWindow;
        uint32 confirmWindow;
        uint32 openWindow;
        bool buyerMayOpen;
        Rung[] ladder;
        bytes32 termsHash;
        bytes32 amanahHash;
        bytes32 quoteHash;
        uint256 nonce;
        uint256 expiry;
    }

    struct OrderTerms {
        Offer offer;
        address buyerKey;
        address buyerPayout;
        uint64 openBy;
        /// s4a: zero, or the instance of the stage before this one, which `init` and `cancel` read.
        address prev;
    }

    /// The case's own machine (s5). Terminals are Refunded and after;
    /// the Closed event's terminal byte is the phase minus 4.
    enum Phase {
        None,
        Funded,
        Claimed,
        Disputed,
        Refunded,
        Released,
        Settled,
        Ruled,
        Resigned,
        Lapsed
    }

    /// One seating (venue/1 s2.2). Only the last seat can be in flight.
    struct SeatRec {
        address judge;
        uint64 accepted;
        uint8 state; // venue/1 s2.6
        uint64 windowStart;
        uint64 windowEnd;
        uint256 standbyCredited;
        uint256 rulingCredited;
    }

    // ------------------------------------------------------------------
    // storage (one instance = one case)
    // ------------------------------------------------------------------

    Phase internal _phase;
    /// s4a: the buyer's share of A at this case's end in basis points, plus one; zero before any end.
    uint16 internal _stageBps;
    uint256 internal _entered;

    // the frozen terms
    address internal sellerKey;
    address internal buyerKey;
    address internal guild; // zero on the named track
    address internal sellerPayout;
    address internal sellerFund;
    address internal buyerPayout;
    address internal _token;
    uint256 internal _amount;
    uint32 internal deliveryWindow;
    uint32 internal confirmWindow;
    Rung[] internal _ladder;
    bytes32 internal _termsHash;
    bytes32 internal _amanahHash;
    bytes32 internal _quoteHash;
    bytes32 internal caseId;

    // the pinned quote (s3)
    uint16 internal premiumBps;
    uint32 internal rulingWindow;
    uint32 internal offerWindow;
    uint8 internal lapseDefault;
    bool internal autoAccept;
    address internal feeUnit;
    uint256 internal _lf;
    uint256 internal _capInToken;
    uint256 internal _priceToken;
    uint256 internal _priceUnit;
    uint64 internal _rateAt;

    // the case's clocks and money (s5, s7a, s8)
    uint64 internal claimedAt;
    uint64 internal disputedAt;
    /// P, the judge's standing fee, computed at init from A (s3).
    uint256 internal _premium;
    /// D = P + lf, the seller's deposit pulled at init (s7a).
    uint256 internal _deposit;
    /// The buyer's litigation fee once a dispute is raised (s8).
    uint256 internal lfHeld;
    /// Credits not yet pushed out, in the case token: owed to parties plus claimable by seats.
    uint256 internal unpaid;
    /// The rung the case was ruled at, for outcomeOf on the ruled seat (s8).
    uint64 internal ruledRung;

    // the guild track (s7)
    address internal _root;
    uint256 internal panelIndex;
    bytes32 internal panelHash;
    uint256 internal panelSize;
    address internal _router;
    bytes32 internal _routerHash;
    bool internal pendingRedraw;
    uint64 internal redrawDeadline;
    /// The block whose hash seeds the pending draw (s7): the block of the arming, read from the next block on.
    uint64 internal drawBlock;

    // s4a: the stage before this one, and the series judge it handed on at init (the judge its `stage()` answered;
    // zero on a first stage)
    address internal _prev;
    address internal _seriesJudge;

    SeatRec[] internal _seats;

    // the economic face (venue/1 s3)
    mapping(address => address) internal _payoutOf;
    mapping(address => mapping(address => uint256)) internal _claimable;
    /// Parties' money by payout address and token (s8); never through the seat ledger.
    mapping(address => mapping(address => uint256)) public owed;

    /// The registry of consent by call (s9, terms/1 s5.4), live on the
    /// logic's own storage alone: a digest a party's current key recorded
    /// under the party. Never removed; the digest's own expiry ends it.
    mapping(address => mapping(bytes32 => bool)) internal _consented;
    /// The registry of revocation (s4), on the logic alone: a seller's word
    /// that no case opens under a document any more. Never removed.
    mapping(address => mapping(bytes32 => bool)) internal _revoked;
    /// s7a: the funding bonds, on the logic alone: funds[seller][fund] once `fund` accepted funding `seller`.
    mapping(address => mapping(address => bool)) public funds;

    // ------------------------------------------------------------------
    // own events (s10)
    // ------------------------------------------------------------------

    event Opened(
        bytes32 indexed caseId, address indexed seller, address indexed buyer, address judge, uint256 amount, bytes32 termsHash
    );
    event Priced(address feeUnit, uint256 lf, uint256 capInToken, uint256 priceToken, uint256 priceUnit, uint64 rateAt);
    event PanelFrozen(address indexed guild, address root, uint256 index, bytes32 panelHash, address router);
    event DrawArmed(uint64 drawBlock, uint64 deadline);
    event DeliveryClaimed(bytes32 manifest);
    event Confirmed();
    event Disputed();
    event Settled(uint16 buyerBps);
    event Ruled(uint64 rung, bytes32 entryId);
    event Resigned(bytes32 entryId, uint8 form); // 0 declined, 1 before dispute, 2 during dispute, 3 no draw or no usable panel
    event Lapsed(uint16 buyerBps);
    event Refunded(uint8 cause); // 0 T1 ran out, 1 offer clock ran out, 2 expiry branch, 3 seller refund, 4 buyer cancel
    event Closed(uint8 terminal, uint256 buyerPaid, uint256 sellerPaid, uint256 judgePaid, uint256 bondToBuyer);
    event Swept(address indexed who, address token, uint256 amount);
    /// On the logic alone (s9, s7a, s4).
    event Consented(address indexed who, bytes32 indexed digest);
    event Pulled(address indexed seller, address indexed fund, address token, address instance, uint256 amount);
    event FundBound(address indexed seller, address indexed fund);
    event Revoked(address indexed seller, bytes32 indexed termsHash);

    // ------------------------------------------------------------------
    // errors
    // ------------------------------------------------------------------

    error Reentered();
    error AlreadyInitialized();
    error WrongAddress();
    error BadTerms(uint8 guard);
    error BadQuote();
    error QuoteMismatch();
    error BadSellerSig();
    error BadBuyerSig();
    error Underfunded();
    error OverCap();
    error RateStale();
    error DepositShort();
    error FundUnbound();
    error PanelBookMissing();
    error VerifierMissing();
    error NoPanel();
    error PanelMismatch();
    error PanelWord();
    error NoMembers();
    error NotPendingRedraw();
    error WrongPhase();
    error NotDirect();
    error UnknownSeat();
    error NotOpen();
    error WindowClosed();
    error NotSeller();
    error NotBuyer();
    error NotParty();
    error NotCurrent();
    error NotInstance();
    error ZeroManifest();
    error RungOutOfRange();
    error ExtraNotEmpty();
    error EntryMismatch();
    error SettleExpired();
    error BpsOver();
    error PayoutSet();
    error ZeroPayout();
    error NoPayout();
    error TransferFailed();
    error NoPanelFace();
    error NotRegistry();
    error BadPrev();
    error StageOpen();
    error StageHeld();
    error DrawPending();
    error RouterGas();
    error FeeOverflow();
    error NotForeign();

    /// The gas a best-effort push of the standard's own (a terminal's, cancel's, the expiry branch's) hands the
    /// token (s8): enough for any plain or proxied ERC-20, bounded so that a recipient's hook cannot put the
    /// call beyond the transaction gas cap nor charge its submitter the whole limit.
    uint256 internal constant PUSH_GAS = 1_000_000;

    modifier nonReentrant() {
        _enter();
        _;
        _entered = 0;
    }

    function _enter() internal {
        if (_entered != 0) revert Reentered();
        _entered = 1;
    }

    // ------------------------------------------------------------------
    // construction (Shape, s13)
    // ------------------------------------------------------------------

    constructor() {
        LOGIC = address(this);
        bytes memory runtime = abi.encodePacked(RUNTIME_PREFIX, address(this), RUNTIME_SUFFIX);
        PROXY_CODEHASH = keccak256(runtime);
        PROXY_INITCODE_HASH = keccak256(abi.encodePacked(INIT_PREFIX, runtime));
        PANEL_BOOK = _create2(keccak256("eravon/1/panel"), PANEL_INITCODE_HASH);
        VERIFIER = _create2(keccak256("eravon/1/verify"), VERIFIER_INITCODE_HASH);
    }

    // ------------------------------------------------------------------
    // s4: the gate
    // ------------------------------------------------------------------

    /// Opens the case, or, on the expiry branch, refunds the address and opens
    /// nothing. `members` and `amanahHashes` are empty on the named track and
    /// the panel read from the chain's panel book on the guild track (s7). The
    /// order of decision is s4's, steps 1 to 8.
    function init(
        OrderTerms calldata t,
        bytes calldata quote,
        bytes calldata sigBuyer,
        bytes calldata sigSeller,
        address[] calldata members,
        bytes32[] calldata amanahHashes
    ) external nonReentrant {
        if (_phase != Phase.None) revert AlreadyInitialized();
        // the salt and the offer's hash, and the address proof (s1, WrongAddress), read on the verifier from this call's own `t`
        (bytes32 salt, bytes32 offerHash) = _saltOf();
        address sk = t.offer.sellerKey;
        address bk = t.buyerKey;
        if (block.timestamp > t.offer.expiry || block.timestamp > t.openBy || Eravon(LOGIC).revoked(sk, t.offer.termsHash)) {
            // the expiry branch: either party, no signature, no quote, no feed, no case; the two keys, the payout
            // and the token are kept so that a later balance leaves by this branch again, another token by sweep,
            // and sweep knows who may call it; what is owed already from an earlier run whose push failed is not
            // credited twice; the push is best effort
            if (msg.sender != _cur(sk) && msg.sender != _cur(bk)) revert NotParty();
            if (t.buyerPayout == address(0)) revert BadTerms(4);
            sellerKey = sk;
            buyerKey = bk;
            buyerPayout = t.buyerPayout;
            _token = t.offer.token;
            caseId = salt;
            _stageBps = 10001;
            uint256 bal = _stray();
            if (bal != 0) _credit(t.buyerPayout, bal);
            _pushOwed(t.buyerPayout);
            emit Refunded(2);
            return;
        }
        // the caller rule: the seller's current key, or, where the order admits it, the buyer's
        if (msg.sender != _cur(sk)) {
            if (!t.offer.buyerMayOpen) revert NotSeller();
            if (msg.sender != _cur(bk)) revert NotParty();
        }
        // the verifier reads the whole calldata once: the terms guards, the quote, the pricing
        EravonVerifier.Checked memory c =
            abi.decode(_verify(abi.encodePacked(EravonVerifier.check.selector, msg.data[4:])), (EravonVerifier.Checked));
        if (c.termsFault != 0) revert BadTerms(c.termsFault);
        if (c.quoteBad) revert BadQuote();
        if (keccak256(quote) != t.offer.quoteHash) revert QuoteMismatch();
        if (address(uint160(c.q[0])) != t.offer.token) revert QuoteMismatch();
        // two consents (s9), each by the party's current key: a signature, or, on an empty slot, the sender or the
        // registry (terms/1 s5.4)
        _consents(sk, _digest(LOGIC, offerHash), sigSeller, bk, _digest(address(this), salt), sigBuyer);
        // the stage before (s4a): an instance of this standard with these keys, ended leaving the buyer no share;
        // the verifier answers the fault as its selector (BadPrev, StageOpen), reverted here with those four bytes
        if (c.stageFault != 0) {
            bytes4 e = c.stageFault;
            assembly ("memory-safe") {
                mstore(0, e)
                revert(0, 4)
            }
        }
        _seriesJudge = c.seriesJudge;
        // the deposit address (s7a): the seller's own key, or an address bound to the seller on the logic
        if (c.fundUnbound) revert FundUnbound();
        uint256 balance = _balance(t.offer.token);
        if (balance < t.offer.amount) revert Underfunded();

        _storeTerms(t, salt);
        _price(c);
        // the seat's preconditions before any money moves: the named track takes no panel, the guild track freezes
        // the guild's root and one panel under it
        if (t.offer.judge != address(0)) {
            if (members.length != 0 || amanahHashes.length != 0) revert BadTerms(20);
        } else {
            _freezePanel(t.offer.guild, members, amanahHashes, t.offer.amanahHash);
        }
        // the seller's deposit (s7a): pulled from sellerFund through the logic, measured here
        uint256 p = (_amount * premiumBps) / 10000;
        uint256 d;
        unchecked {
            d = p + _lf;
        }
        if (d < p) revert FeeOverflow();
        if (d != 0) {
            Eravon(LOGIC).pull(sellerKey, sellerFund, _token, d);
            uint256 after_ = _balance(_token);
            if (after_ < balance || after_ - balance < d) revert DepositShort();
        }
        _premium = p;
        _deposit = d;
        address judge = t.offer.judge;
        if (judge != address(0)) {
            _openSeat(judge);
        } else {
            // the draw waits for a block the opener cannot see (s7): anyone draws from the next block on
            _armDraw();
        }
        _phase = Phase.Funded;
        uint256 surplus = balance - t.offer.amount;
        if (surplus != 0) _credit(t.buyerPayout, surplus);
        emit Opened(salt, sk, bk, judge, t.offer.amount, t.offer.termsHash);
    }

    /// s4: the buyer's cancellation before any case opened. The address
    /// proves itself; the buyer's current key is the caller with an empty
    /// `sig`, or `sig` is a signature by that key over Cancel() under this
    /// instance's domain (terms/1 s4.5) sent by either party's current key; the
    /// whole balance goes to the buyer's payout and the instance holds a closed
    /// case with both keys kept, so that `init` after it is refused and `sweep`
    /// knows who may call it.
    function cancel(OrderTerms calldata t, bytes calldata sig) external nonReentrant {
        if (_phase != Phase.None) revert AlreadyInitialized();
        // the verifier reads this call's own `t` once: the salt, the address proof (WrongAddress), the buyer's key
        // (NotBuyer) and payout (BadTerms(4)), and, where the order names a stage before, its gate (StageHeld, s4a)
        bytes32 salt = abi.decode(_verify(abi.encodePacked(EravonVerifier.cancelCheck.selector, msg.data[4:])), (bytes32));
        address bk = t.buyerKey;
        if (sig.length == 0) {
            if (msg.sender != _cur(bk)) revert NotBuyer();
        } else {
            if (msg.sender != _cur(t.offer.sellerKey) && msg.sender != _cur(bk)) revert NotParty();
            _verifyBuyer(_digest(address(this), keccak256(abi.encode(TH_CANCEL))), sig, bk);
        }
        buyerPayout = t.buyerPayout;
        sellerKey = t.offer.sellerKey;
        buyerKey = bk;
        _token = t.offer.token;
        caseId = salt;
        // the balance beyond what an earlier expiry-branch run left owed; Closed reports what this call credited
        uint256 bal = _stray();
        _phase = Phase.Refunded;
        _stageBps = 10001;
        if (bal != 0) _credit(t.buyerPayout, bal);
        emit Refunded(4);
        emit Closed(0, bal, 0, 0, 0);
        _pushOwed(t.buyerPayout);
    }

    /// The order's salt, caseId and CREATE2 salt alike (s1), hashStruct(OrderTerms) of terms/1 s4.2, and the
    /// offer's hashStruct, computed on the verifier from this call's own `t`, its first argument.
    function _saltOf() internal view returns (bytes32 salt, bytes32 offerHash) {
        return abi.decode(_verify(abi.encodePacked(EravonVerifier.initSalt.selector, msg.data[4:])), (bytes32, bytes32));
    }

    /// terms/1 s4.1's shape, frozen.
    function _storeTerms(OrderTerms calldata t, bytes32 salt) internal {
        Offer calldata o = t.offer;
        sellerKey = o.sellerKey;
        buyerKey = t.buyerKey;
        guild = o.guild;
        sellerPayout = o.sellerPayout;
        sellerFund = o.sellerFund;
        buyerPayout = t.buyerPayout;
        _token = o.token;
        _amount = o.amount;
        deliveryWindow = o.deliveryWindow;
        confirmWindow = o.confirmWindow;
        for (uint256 i = 0; i < o.ladder.length; i++) {
            _ladder.push(o.ladder[i]);
        }
        _termsHash = o.termsHash;
        _amanahHash = o.amanahHash;
        _quoteHash = o.quoteHash;
        caseId = salt;
        _prev = t.prev;
    }

    /// s3: the fee priced once and pinned, as the verifier computed it; the
    /// quote's clocks and switches; the stale rate refused here, after the
    /// signatures.
    function _price(EravonVerifier.Checked memory c) internal {
        uint256[15] memory q = c.q;
        premiumBps = uint16(q[1]);
        rulingWindow = uint32(q[11]);
        offerWindow = uint32(q[12]);
        lapseDefault = uint8(q[13]);
        autoAccept = q[14] == 1;
        address unit = address(uint160(q[2]));
        feeUnit = unit;
        if (unit != address(0)) {
            if (c.stale) revert RateStale();
            if (c.overflow) revert FeeOverflow();
            _priceToken = c.priceToken;
            _priceUnit = c.priceUnit;
            _rateAt = c.rateAt;
        }
        if (c.overflow) revert FeeOverflow();
        _lf = c.lf;
        _capInToken = c.cap;
        if (c.cap != 0 && _amount > c.cap) revert OverCap();
        if (unit != address(0)) emit Priced(unit, c.lf, c.cap, c.priceToken, c.priceUnit, c.rateAt);
    }

    // ------------------------------------------------------------------
    // s7a, s4, s9: the logic's three registries and the deposit face (on the logic alone)
    // ------------------------------------------------------------------

    /// s7a: the one path by which a seller's deposit moves. Accepted on the
    /// logic alone, only from an instance of this standard (the caller's code
    /// is the proxy that embeds this logic), and the receiver is that caller
    /// and never another address; `transferFrom(fund, instance, amount)` on
    /// the token under `fund`'s allowance to the logic, `fund` being the order's
    /// `sellerFund`, which `init` tested as the seller's key or an address
    /// bound to the seller here.
    function pull(address seller, address fund, address token_, uint256 amount) external nonReentrant {
        _onLogic();
        if (msg.sender.codehash != PROXY_CODEHASH) revert NotInstance();
        if (!_transferFromOk(token_, fund, msg.sender, amount)) revert TransferFailed();
        emit Pulled(seller, fund, token_, msg.sender, amount);
    }

    /// s7a: the sender binds itself as a funder of `seller`'s deposits, on the
    /// logic alone; only ever set, never cleared (a funder withdraws by setting
    /// its allowance to zero).
    function acceptFunding(address seller) external {
        _onLogic();
        funds[seller][msg.sender] = true;
        emit FundBound(seller, msg.sender);
    }

    /// s4, s9: the seller's revocation of a document, recorded under `seller`
    /// on the logic alone from `seller`'s current key alone; never withdrawn.
    function revoke(address seller, bytes32 termsHash) external {
        _onLogic();
        if (msg.sender != _cur(seller)) revert NotCurrent();
        _revoked[seller][termsHash] = true;
        emit Revoked(seller, termsHash);
    }

    /// The revocation registry's answer, read on the logic (an instance holds none and answers false).
    function revoked(address seller, bytes32 termsHash) external view returns (bool) {
        return _revoked[seller][termsHash];
    }

    /// s9: `party`'s current key records a digest under `party`; no removal.
    /// Accepted on the logic alone, so that an instance never holds a registry of its own.
    function consent(address party, bytes32 digest) external {
        _onLogic();
        if (msg.sender != _cur(party)) revert NotCurrent();
        _consented[party][digest] = true;
        emit Consented(party, digest);
    }

    /// The registry's answer, read on the logic (an instance holds no registry and answers false).
    function consented(address who, bytes32 digest) external view returns (bool) {
        return _consented[who][digest];
    }

    // ------------------------------------------------------------------
    // s7: the guild track
    // ------------------------------------------------------------------

    /// The guild's root and panel, read and checked on the verifier (the book's code, the record recorded before
    /// this block and unexpired, the panel's hash, its words, and where the record names no router its pool of
    /// distinct members that are neither party), then frozen here.
    function _freezePanel(address g, address[] calldata members, bytes32[] calldata words, bytes32 word) internal {
        bytes memory ret = _verify(
            abi.encodeWithSelector(
                EravonVerifier.panel.selector,
                g,
                [PROXY_CODEHASH, PANEL_INITCODE_HASH, PANEL_CODEHASH],
                members,
                words,
                word,
                sellerKey,
                buyerKey
            )
        );
        (address root, uint256 index, bytes32 ph, address router, bytes32 rh, uint256 pool) =
            abi.decode(ret, (address, uint256, bytes32, address, bytes32, uint256));
        _root = root;
        panelIndex = index;
        panelHash = ph;
        _router = router;
        _routerHash = rh;
        panelSize = pool;
        emit PanelFrozen(g, root, index, ph, router);
    }

    /// Arms a draw (s7): the seed is the hash of this block, which nobody sending in it can see; the draw
    /// is open to anyone from the next block until offerWindow from now.
    function _armDraw() internal {
        pendingRedraw = true;
        redrawDeadline = uint64(block.timestamp) + offerWindow;
        drawBlock = uint64(block.number);
        emit DrawArmed(drawBlock, redrawDeadline);
    }

    /// Every judge seated or offered on the case, in the order of the seats' ordinals.
    function _seated() internal view returns (address[] memory s) {
        s = new address[](_seats.length);
        for (uint256 i = 0; i < s.length; i++) {
            s[i] = _seats[i].judge;
        }
    }

    function _seatedBefore(address who) internal view returns (bool) {
        for (uint256 i = 0; i < _seats.length; i++) {
            if (_seats[i].judge == who) return true;
        }
        return false;
    }

    /// Opens the next seat for `judge`: consent carried under autoAccept, an offer clock otherwise.
    function _openSeat(address judge) internal {
        uint64 seat = uint64(_seats.length);
        SeatRec storage s = _seats.push();
        s.judge = judge;
        s.windowStart = uint64(block.timestamp);
        if (autoAccept) {
            s.accepted = uint64(block.timestamp);
            s.windowEnd = uint64(block.timestamp) + deliveryWindow;
        } else {
            s.windowEnd = uint64(block.timestamp) + offerWindow;
        }
        emit Seated(judge, caseId, seat);
    }

    /// Anyone, with the frozen panel (s7: where no router is named, the answer is the seed's and the panel's,
    /// whoever sends it): the draw of the next seat, the first included, from the armed block's hash, from the
    /// block after the arming on; where the chain no longer
    /// serves that hash, the case ends as Resigned with form 3: a seed nobody drew on buys no second one.
    /// Where the frozen record names a router, the router draws (s7), and a router that fails leaves the case
    /// with no usable panel: Resigned with form 3.
    function redraw(address[] calldata members, bytes32[] calldata amanahHashes) external nonReentrant {
        // the block after the arming may always draw, so no offerWindow makes a draw impossible
        if (!pendingRedraw || (block.timestamp > redrawDeadline && block.number > drawBlock + 1)) revert NotPendingRedraw();
        // the verifier tests the panel against the frozen hash and the block (PanelMismatch, then DrawPending), and
        // answers the member the draw seats where no router is named, or the router's calldata where one is
        bytes32 seed = blockhash(drawBlock);
        (address m, bytes memory data) = abi.decode(
            _verify(
                abi.encodeWithSelector(
                    EravonVerifier.draw.selector,
                    [uint256(caseId), _seats.length, uint256(_termsHash), uint256(uint160(_token)), _amount, _ladder.length, uint256(seed), uint256(panelHash), block.number > drawBlock ? 1 : 0, uint256(uint160(_router)), uint256(uint160(_prev))],
                    [buyerKey, sellerKey, sellerFund, buyerPayout, sellerPayout],
                    members,
                    amanahHashes,
                    _seated()
                )
            ),
            (address, bytes)
        );
        pendingRedraw = false;
        if (data.length != 0) m = _route(members, data);
        if (m == address(0)) _finishResigned(bytes32(0), 3);
        else _openSeat(m);
    }

    /// s7, venue/1 s4: the router's draw, by `staticcall` with exactly ROUTE_GAS; the whole call reverts where the
    /// gas left cannot give it that much; zero where the code hash differs from the frozen one, the call fails or
    /// answers other than one word, or the index lies outside `members` or names a member a seat of the case holds.
    function _route(address[] calldata members, bytes memory data) internal view returns (address) {
        address router = _router;
        if (router.codehash != _routerHash) return address(0);
        uint256 g = ROUTE_GAS;
        if (gasleft() <= g + g / 63 + 20_000) revert RouterGas();
        bool ok;
        uint256 size;
        uint256 index;
        assembly ("memory-safe") {
            ok := staticcall(g, router, add(data, 0x20), mload(data), 0, 0)
            size := returndatasize()
            if and(ok, eq(size, 32)) {
                returndatacopy(0, 0, 32)
                index := mload(0)
            }
        }
        if (!ok || size != 32 || index >= members.length) return address(0);
        address m = members[index];
        if (_seatedBefore(m)) return address(0);
        return m;
    }

    /// After an unaccepted guild seat ended: a draw is armed, or the case ends (three seats opened, or, where the
    /// frozen record names no router, the pool exhausted).
    function _afterUnacceptedSeatEnded() internal {
        if (_caseEndsAfterOffer()) _finishResigned(bytes32(0), 3);
        else _armDraw();
    }

    function _caseEndsAfterOffer() internal view returns (bool) {
        return _seats.length >= 3 || (_router == address(0) && _seats.length >= panelSize);
    }

    // ------------------------------------------------------------------
    // venue/1 case face
    // ------------------------------------------------------------------

    function seatOf(bytes32 id, uint64 seat) external view returns (Seat memory s) {
        SeatRec storage r = _seatChecked(id, seat);
        s.judge = r.judge;
        s.amanahHash = _amanahHash;
        s.termsHash = _termsHash;
        s.quoteHash = _quoteHash;
        s.windowStart = r.windowStart;
        s.windowEnd = r.windowEnd;
        s.accepted = r.accepted;
        s.state = r.state;
        s.rungs = uint64(_ladder.length);
        s.rungMin = 0;
        s.rungMax = uint64(_ladder.length - 1);
        s.token = _token;
        s.amount = _amount;
        s.standbyCredited = r.standbyCredited;
        s.rulingCredited = r.rulingCredited;
        s.parties = new address[](5);
        s.parties[0] = buyerKey;
        s.parties[1] = sellerKey;
        s.parties[2] = sellerFund;
        s.parties[3] = buyerPayout;
        s.parties[4] = sellerPayout;
    }

    function quoteType() external pure returns (string memory) {
        return QUOTE_TYPE;
    }

    function extraType() external pure returns (string memory) {
        return "";
    }

    /// s8: the buyer role is buyerKey and buyerPayout, the seller role sellerKey, sellerFund and sellerPayout.
    function partyRoles() external pure returns (string memory) {
        return "buyer,seller";
    }

    function successionBook() external pure returns (address) {
        return SUCCESSION_BOOK;
    }

    function routeGas() external pure returns (uint64) {
        return ROUTE_GAS;
    }

    /// s4a: the stage view every instance answers: the two keys as recorded, whether the instance ended (a terminal,
    /// `cancel` or the expiry branch), the buyer's share of the amount at that end in basis points, the series judge
    /// as of this stage (the judge of its last seat where that seat was accepted and neither resigned nor lapsed,
    /// else the one the stage before handed on; zero where the case ended Lapsed), and the salt the instance proved
    /// its address under (zero until init, cancel or the expiry branch). `_stageBps` holds the share plus one from
    /// the end on, so one word says both whether and how it ended.
    function stage() external view returns (address, address, bool, uint16, address, bytes32) {
        uint16 e = _stageBps;
        address j;
        if (_phase != Phase.Lapsed) {
            j = _seriesJudge;
            if (_seats.length != 0) {
                SeatRec storage s = _last();
                if (s.accepted != 0 && s.state != 3 && s.state != 4) j = s.judge;
            }
        }
        unchecked {
            return (sellerKey, buyerKey, e != 0, e == 0 ? 0 : e - 1, j, caseId);
        }
    }

    /// s8, venue/1 s2.12: the row Ruled at `rung`, on a seat in flight with the acceptance and the dispute taken
    /// as made and the pinned fee collected, and on the ruled seat for the rung it was ruled at.
    function outcomeOf(bytes32 id, uint64 seat, uint64 rung, bytes calldata extra) external view returns (Outcome memory) {
        SeatRec storage s = _seatChecked(id, seat);
        if (rung >= _ladder.length) revert RungOutOfRange();
        if (extra.length != 0) revert ExtraNotEmpty();
        if (s.state > 1 && (s.state != 2 || rung != ruledRung)) revert NotOpen();
        Rung storage o = _ladder[rung];
        _outcome(_amount, o.buyerBps, true, true, o.feeBearer, 2);
    }

    /// s8, venue/1 s2.12: what `advance` does by the named seat's own clock, on the state the call finds.
    function advanceOf(bytes32 id, uint64 seat) external view returns (Outcome memory) {
        SeatRec storage s = _seatChecked(id, seat);
        uint8 k = _due(seat);
        if (k == 0 || k == 6) _outcome(0, 0, false, false, 0, s.state);
        if (k == 3) _outcome(0, 0, false, false, 0, 5);
        if (k == 5) _outcome(_amount, 0, true, false, 0, 5);
        if (k == 7) _outcome(_amount, _lapseBps(), false, false, 0, 4);
        _outcome(_amount, 10000, false, false, 0, 5);
    }

    /// Answers the call with the Outcome the verifier's `split` reports for one action, the pinned P and fee
    /// standing in for the held ones (equal while the case is open), and returns no further.
    function _outcome(uint256 a, uint16 bps, bool pj, bool lj, uint8 bondTo, uint8 st) internal view {
        uint256 p = (_amount * premiumBps) / 10000;
        bytes memory ret = _split(a, bps, p, p + _lf, _lf, pj, lj, bondTo, st);
        // the five amounts, then the Outcome: its encoding is the tail, the word before it its offset
        assembly ("memory-safe") {
            let q := add(ret, 0xc0)
            mstore(q, 0x20)
            return(q, sub(mload(ret), 0xa0))
        }
    }

    /// venue/1 s2.7: the judge's word on an open offer, from the seated judge's current key.
    function accept(bytes32 id, uint64 seat) external nonReentrant {
        SeatRec storage s = _seatChecked(id, seat);
        _direct(s);
        if (s.state != 0 || s.accepted != 0) revert NotOpen();
        if (block.timestamp > s.windowEnd) revert WindowClosed();
        s.accepted = uint64(block.timestamp);
        s.windowStart = uint64(block.timestamp);
        s.windowEnd = uint64(block.timestamp) + deliveryWindow;
    }

    /// venue/1 s2.8: any address, from the case's existence until it ends.
    function commit(bytes32 id, bytes32 manifest) external {
        if (id != caseId || !_open()) revert NotOpen();
        emit Committed(id, msg.sender, manifest);
    }

    /// venue/1 s2.9: the ruling, with the judgment entry's bytes; rung < rungs (s6).
    function rule(bytes32 id, uint64 seat, bytes32 entryId, uint64 rung, bytes calldata entry, bytes calldata extra)
        external
        nonReentrant
    {
        SeatRec storage s = _seatChecked(id, seat);
        _direct(s);
        if (s.state != 1) revert NotOpen();
        if (block.timestamp > s.windowEnd) revert WindowClosed();
        if (rung >= _ladder.length) revert RungOutOfRange();
        if (extra.length != 0) revert ExtraNotEmpty();
        if (sha256(entry) != entryId) revert EntryMismatch();
        s.state = 2;
        ruledRung = rung;
        emit Anchored(msg.sender, entryId, entry);
        emit Ruled(rung, entryId);
        Rung storage o = _ladder[rung];
        // the seller's LF: back to the seller where the buyer bears the fee (0), to the buyer where the seller does
        // (1), half each otherwise (2)
        _finish(Phase.Ruled, o.buyerBps, true, true, o.feeBearer);
    }

    /// venue/1 s2.10: the one exit, with the resignation entry's bytes; what follows is s6's.
    function resign(bytes32 id, uint64 seat, bytes32 entryId, bytes calldata entry) external nonReentrant {
        SeatRec storage s = _seatChecked(id, seat);
        _direct(s);
        if (s.state > 1) revert NotOpen();
        // venue/1 s2.10: an accepted seat past its window has only the clock's outcome (advance); declining a stale offer stays open
        if (s.accepted != 0 && block.timestamp > s.windowEnd) revert WindowClosed();
        if (sha256(entry) != entryId) revert EntryMismatch();
        s.state = 3;
        emit Anchored(msg.sender, entryId, entry);
        if (s.accepted == 0) {
            emit Resigned(entryId, 0);
            if (guild == address(0)) _finishResigned(entryId, 0);
            else _afterUnacceptedSeatEnded();
        } else if (_phase == Phase.Funded) {
            emit Resigned(entryId, 1);
            _finishResigned(entryId, 1);
        } else if (_phase == Phase.Claimed) {
            // the case runs on without a seat in flight: confirm or T2 releases, dispute lapses (s6)
            emit Resigned(entryId, 1);
        } else {
            emit Resigned(entryId, 2);
            _lapse();
        }
    }

    /// venue/1 s2.11: the clocks (s5), by anyone; nothing due, nothing changed.
    function advance(bytes32 id, uint64 seat) external nonReentrant {
        if (_phase == Phase.None || id != caseId) return;
        if (seat < _seats.length) {
            uint8 k = _due(seat);
            SeatRec storage s = _seats[seat];
            if (k == 1 || k == 4) {
                // an offer that expired on the named track (cause 1), or T1 (cause 0)
                s.state = 5;
                emit Refunded(k == 1 ? 1 : 0);
                _finish(Phase.Refunded, 10000, false, false, 0);
            } else if (k == 2 || k == 3) {
                // an offer that expired on the guild track
                s.state = 5;
                _afterUnacceptedSeatEnded();
            } else if (k == 5) {
                // T2: confirmation lapsed under a seated seat, P to the judge
                s.state = 5;
                _finish(Phase.Released, 0, true, false, 0);
            } else if (k == 6) {
                // T2 after the seat resigned: P back to the seller, the seat staying resigned
                _finish(Phase.Released, 0, false, false, 0);
            } else if (k == 7) {
                // T3: the judge's silence
                s.state = 4;
                _lapse();
            }
        }
        // the venue's other clock: a draw nobody made, by its deadline (the block after the arming excepted) or
        // by the loss of its seed; run whatever seat the call names
        if (pendingRedraw && block.number > drawBlock + 1
                && (block.timestamp > redrawDeadline || block.number > drawBlock + 256)) {
            pendingRedraw = false;
            _finishResigned(bytes32(0), 3);
        }
    }

    /// What `advance` does by the named seat's own clock (s5): 0 nothing; 1 an offer expired on the named track;
    /// 2 an offer expired on the guild track that ends the case; 3 one that arms the next draw; 4 T1; 5 T2 under a
    /// seated seat; 6 T2 after the seat resigned (the seat out of flight); 7 T3.
    function _due(uint64 seat) internal view returns (uint8) {
        SeatRec storage s = _seats[seat];
        if (s.windowEnd == 0 || block.timestamp <= s.windowEnd) return 0;
        if (s.state <= 1) {
            if (s.accepted == 0) return guild == address(0) ? 1 : (_caseEndsAfterOffer() ? 2 : 3);
            if (_phase == Phase.Funded) return 4;
            if (_phase == Phase.Claimed) return 5;
            return 7;
        }
        if (s.state == 3 && _phase == Phase.Claimed && seat == _seats.length - 1) return 6;
        return 0;
    }

    /// The buyer's share `lapseDefault` names (terms/1 s3.3): 0 none of A, 1 half, 2 all of it.
    function _lapseBps() internal view returns (uint16) {
        return uint16(lapseDefault) * 5000;
    }

    /// Lapsed at the share `lapseDefault` names: P and the deposit back to the seller, the fee back to the buyer (s8).
    function _lapse() internal {
        uint16 b = _lapseBps();
        emit Lapsed(b);
        _finish(Phase.Lapsed, b, false, false, 0);
    }

    // ------------------------------------------------------------------
    // s9: the parties' actions
    // ------------------------------------------------------------------

    function claimDelivery(bytes32 manifest) external nonReentrant {
        if (msg.sender != _cur(sellerKey)) revert NotSeller();
        _claimDelivery(manifest);
    }

    function claimDeliveryBySig(bytes32 manifest, bytes calldata sig) external nonReentrant {
        _party();
        if (manifest == bytes32(0)) revert ZeroManifest();
        _verifySeller(_digest(address(this), keccak256(abi.encode(TH_CLAIM, manifest))), sig);
        _claimDelivery(manifest);
    }

    function _claimDelivery(bytes32 manifest) internal {
        if (_phase != Phase.Funded) revert WrongPhase();
        SeatRec storage s = _last();
        if (s.state != 0 || s.accepted == 0) revert NotOpen();
        if (block.timestamp > s.windowEnd) revert WindowClosed();
        if (manifest == bytes32(0)) revert ZeroManifest();
        _phase = Phase.Claimed;
        claimedAt = uint64(block.timestamp);
        s.windowStart = uint64(block.timestamp);
        s.windowEnd = uint64(block.timestamp) + confirmWindow;
        emit DeliveryClaimed(manifest);
        emit Committed(caseId, msg.sender, manifest);
    }

    function confirm() external nonReentrant {
        if (msg.sender != _cur(buyerKey)) revert NotBuyer();
        _confirm();
    }

    function confirmBySig(bytes calldata sig) external nonReentrant {
        _party();
        _verifyBuyer(_digest(address(this), keccak256(abi.encode(TH_CONFIRM))), sig, buyerKey);
        _confirm();
    }

    function _confirm() internal {
        if (_phase != Phase.Claimed) revert WrongPhase();
        SeatRec storage s = _last();
        bool seated = s.state == 0;
        if (seated) s.state = 5;
        emit Confirmed();
        _finish(Phase.Released, 0, seated, false, 0);
    }

    function dispute() external nonReentrant {
        if (msg.sender != _cur(buyerKey)) revert NotBuyer();
        _dispute();
    }

    function disputeBySig(bytes calldata sig) external nonReentrant {
        _party();
        _verifyBuyer(_digest(address(this), keccak256(abi.encode(TH_DISPUTE))), sig, buyerKey);
        _dispute();
    }

    /// The buyer's dispute while Claimed: with the seat seated, the fee is
    /// taken and T3 runs; with the seat resigned (s6), no fee is taken and the
    /// case lapses at once at the share `lapseDefault` names.
    function _dispute() internal {
        if (_phase != Phase.Claimed) revert WrongPhase();
        SeatRec storage s = _last();
        if (block.timestamp > s.windowEnd) revert WindowClosed();
        if (s.state == 3) {
            emit Disputed();
            _lapse();
            return;
        }
        if (s.state != 0 || s.accepted == 0) revert NotOpen();
        _collectLf();
        _phase = Phase.Disputed;
        disputedAt = uint64(block.timestamp);
        s.state = 1;
        s.windowStart = uint64(block.timestamp);
        s.windowEnd = uint64(block.timestamp) + rulingWindow;
        emit Disputed();
    }

    /// The litigation fee: what already flowed in during Claimed, then the rest from buyerKey's own address (s8).
    function _collectLf() internal {
        uint256 have = _stray();
        if (have < _lf) {
            _pull(buyerKey, _lf - have);
            have = _stray();
            if (have < _lf) revert Underfunded();
        }
        lfHeld = _lf;
    }

    function refund() external nonReentrant {
        if (msg.sender != _cur(sellerKey)) revert NotSeller();
        _refund();
    }

    /// The signed form is refused throughout a dispute: a Refund() signed in an earlier state is submitted only
    /// outside one, so a signature handed out while Funded cannot pick the refund over the lapse or the ruling;
    /// the sender and the signature are tested first, as every signed form tests its actor first (s10).
    function refundBySig(bytes calldata sig) external nonReentrant {
        _party();
        _verifySeller(_digest(address(this), keccak256(abi.encode(TH_REFUND))), sig);
        if (_phase == Phase.Disputed) revert WrongPhase();
        _refund();
    }

    /// The seller's unilateral right in any open state but a running dispute: everything back, the deposit to
    /// the seller, and P to the seller, or to the judge where delivery was claimed under a seated seat, as T2's
    /// lapse would give it (s8). While Disputed at or before T3's end the call is refused, so that a dispute
    /// ordered ahead of a refund cannot change what the refund does; the seller leaves a dispute by settle or by
    /// the ruling, and past T3's end its refund is the plain one.
    function _refund() internal {
        if (!_open()) revert WrongPhase();
        bool seatedClaimed = false;
        if (_seats.length != 0) {
            SeatRec storage s = _last();
            if (s.state == 1 && block.timestamp <= s.windowEnd) revert WrongPhase();
            // P is the judge's once delivery was claimed under a seated seat: a refund and a payment off the
            // instance would otherwise complete the trade with P stripped
            seatedClaimed = _phase == Phase.Claimed && s.state == 0;
            if (s.state <= 1) s.state = 5;
        }
        pendingRedraw = false;
        emit Refunded(3);
        _finish(Phase.Refunded, 10000, seatedClaimed, false, 0);
    }

    /// Either party's current key, where both consents to the Settle message hold: a signature by the party's
    /// current key, or on an empty slot the caller or the logic's registry (s9).
    function settle(uint16 buyerBps, uint256 settleExpiry, bytes calldata sigB, bytes calldata sigS) external nonReentrant {
        _party();
        if (!_open()) revert WrongPhase();
        if (buyerBps > 10000) revert BpsOver();
        if (block.timestamp > settleExpiry) revert SettleExpired();
        bytes32 d = _digest(address(this), keccak256(abi.encode(TH_SETTLE, uint256(buyerBps), settleExpiry)));
        _consents(sellerKey, d, sigS, buyerKey, d, sigB);
        // P to the judge where the seat was accepted and its window still runs, back to the seller where the seat
        // resigned, never accepted, ran out of time, or was never drawn (s8)
        bool seated = false;
        if (_seats.length != 0) {
            SeatRec storage s = _last();
            // an accepted seat inside its window, or seated while the case is Claimed at any time (T2's lapse
            // releases with P to the judge, so a settlement past T2 pays the judge too)
            seated = s.accepted != 0
                && ((s.state == 0 && (block.timestamp <= s.windowEnd || _phase == Phase.Claimed))
                    || (s.state == 1 && block.timestamp <= s.windowEnd));
            if (s.state <= 1) s.state = 5;
        }
        pendingRedraw = false;
        emit Settled(buyerBps);
        _finish(Phase.Settled, buyerBps, seated, false, 0);
    }

    // ------------------------------------------------------------------
    // venue/1 economic face, and sweep (s8)
    // ------------------------------------------------------------------

    /// venue/1 s3: from `judge`'s current key, once, any nonzero address.
    function setPayout(address judge, address payout) external {
        if (address(this) == LOGIC) revert NotInstance();
        if (msg.sender != _cur(judge)) revert NotCurrent();
        if (_payoutOf[judge] != address(0)) revert PayoutSet();
        if (payout == address(0)) revert ZeroPayout();
        _payoutOf[judge] = payout;
    }

    /// venue/1 s3: from `judge`'s current key; registers `payout` first where none is and it is nonzero.
    function withdraw(address token_, address judge, address payout) external nonReentrant {
        if (address(this) == LOGIC) revert NotInstance();
        if (msg.sender != _cur(judge)) revert NotCurrent();
        address p = _payoutOf[judge];
        if (p == address(0)) {
            p = payout;
            _payoutOf[judge] = payout;
        } else if (payout != address(0) && payout != p) {
            revert PayoutSet();
        }
        if (p == address(0)) revert NoPayout();
        uint256 c = _claimable[judge][token_];
        if (c != 0) {
            _claimable[judge][token_] = 0;
            uint256 d = _debit(_transfer(token_, p, c, 0), c);
            if (d == 0) revert TransferFailed();
            _claimable[judge][token_] = c - d;
            _swept(judge, token_, d);
        }
    }

    function accountOf(address who, address token_) external view returns (address payout, uint256 claimable) {
        return (_payoutOf[who], _claimable[who][token_]);
    }

    /// A party's or a seated judge's current key: pushes what `who` is owed (a party's payout address, the
    /// order's `sellerFund`, or a seat with a registered payout), and the strays of s8.
    function sweep(address who, address token_) external nonReentrant {
        if (msg.sender != _cur(sellerKey) && msg.sender != _cur(buyerKey)) {
            bool judge = false;
            for (uint256 i = 0; i < _seats.length; i++) {
                if (msg.sender == _cur(_seats[i].judge)) judge = true;
            }
            if (!judge) revert NotParty();
        }
        if (token_ != _token && buyerPayout != address(0)) {
            // a token that is not the case's has no ledger here: all of it is the buyer's stray (s8)
            uint256 foreign = _balance(token_);
            if (foreign != 0) {
                uint256 keep = _balance(_token);
                uint256 d = _transfer(token_, buyerPayout, foreign, 0);
                if (_balance(_token) < keep) revert NotForeign();
                if (d != 0) _swept(buyerPayout, token_, d);
            }
        }
        if (_phase >= Phase.Refunded && token_ == _token) {
            // stray inflow after the terminal goes to the buyer's payout
            uint256 stray = _stray();
            if (stray != 0) _credit(buyerPayout, stray);
        }
        uint256 o = owed[who][token_];
        if (o != 0) {
            owed[who][token_] = 0;
            uint256 d = _debit(_transfer(token_, who, o, 0), o);
            owed[who][token_] = o - d;
            if (d != 0) _swept(who, token_, d);
        }
        uint256 c = _claimable[who][token_];
        address p = _payoutOf[who];
        if (c != 0 && p != address(0)) {
            _claimable[who][token_] = 0;
            uint256 d = _debit(_transfer(token_, p, c, 0), c);
            _claimable[who][token_] = c - d;
            if (d != 0) _swept(who, token_, d);
        }
    }

    /// venue/1 s4: the panel face lives on the chain's panel book (s7).
    function engage(bytes32, address[] calldata, bytes32[] calldata, bytes32, uint64, address) external pure {
        revert NoPanelFace();
    }

    /// The two parties' consents, decided on the verifier against each party's current key: a signature over
    /// the party's digest, or, on an empty slot, the sender being that key or the logic's registry holding the
    /// digest under the party (terms/1 s5.4); the party's fault bubbles up unchanged.
    function _consents(address seller, bytes32 sellerDigest, bytes calldata sigS, address buyer, bytes32 buyerDigest, bytes calldata sigB)
        internal
        view
    {
        _verify(
            abi.encodeWithSelector(
                EravonVerifier.consents.selector, LOGIC, msg.sender, seller, sellerDigest, sigS, buyer, buyerDigest, sigB
            )
        );
    }

    // ------------------------------------------------------------------
    // views beyond the interface
    // ------------------------------------------------------------------

    /// s3: the pinned fee and cap in the settlement token, and the two prices used.
    function feeQuoted()
        external
        view
        returns (uint256 lf, uint256 capInToken, uint256 priceToken, uint256 priceUnit, uint64 rateAt)
    {
        return (_lf, _capInToken, _priceToken, _priceUnit, _rateAt);
    }

    function panelInitCodeHash() external pure returns (bytes32) {
        return PANEL_INITCODE_HASH;
    }

    /// The chain's panel book (s7).
    function panelBook() external view returns (address) {
        return PANEL_BOOK;
    }

    /// The case's own state beyond the interface (s10), sixteen values.
    struct CaseStatus {
        Phase phase;
        uint256 seats;
        uint256 premium;
        uint256 deposit;
        uint256 lfHeld;
        uint256 unpaid;
        uint64 claimedAt;
        uint64 disputedAt;
        bool pendingRedraw;
        uint64 redrawDeadline;
        uint64 drawBlock;
        uint256 panelIndex;
        bytes32 panelHash;
        uint256 panelSize;
        address root;
        address router;
    }

    /// The case's own state beyond the interface, for clients and indexers, encoded as its sixteen values;
    /// the terms are in the init calldata and the Opened event. The desk reads none of it.
    function status() external view returns (CaseStatus memory) {
        return CaseStatus(
            _phase,
            _seats.length,
            _premium,
            _deposit,
            lfHeld,
            unpaid,
            claimedAt,
            disputedAt,
            pendingRedraw,
            redrawDeadline,
            drawBlock,
            panelIndex,
            panelHash,
            panelSize,
            _root,
            _router
        );
    }

    /// The EIP-712 digest under domain {ERAVON, 1, chainid, verifying}.
    function _digest(address verifying, bytes32 structHash) internal view returns (bytes32) {
        bytes32 domain = keccak256(abi.encode(TH_DOMAIN, NAME_HASH, VERSION_HASH, block.chainid, verifying));
        return keccak256(abi.encodePacked(hex"1901", domain, structHash));
    }

    function _create2(bytes32 salt, bytes32 initHash) internal pure returns (address) {
        return address(uint160(uint256(keccak256(abi.encodePacked(hex"ff", DEPLOYER, salt, initHash)))));
    }

    // ------------------------------------------------------------------
    // s8: the terminal table
    // ------------------------------------------------------------------

    /// Splits A by `buyerBps` through the verifier's `split` (s8), the function the two outcome views read;
    /// sends P to the judge where `pToJudge`, back to `sellerFund` otherwise; sends the buyer's held LF to the
    /// judge or back to the buyer; disposes of the seller's LF (0 seller, 1 buyer, 2 half each); returns every
    /// stray to the buyer; credits every payout, the judge's on the last seat, and pushes what it can.
    function _finish(Phase terminal, uint16 buyerBps, bool pToJudge, bool lfToJudge, uint8 bondTo) internal {
        // what flowed in unrecorded, read while the ledger still counts A, D and the fee: the buyer's
        uint256 stray = _stray();
        _phase = terminal;
        _stageBps = buyerBps + 1;
        pendingRedraw = false;
        (uint256 buyerPaid, uint256 sellerA, uint256 judgePaid, uint256 toSellerFund, uint256 bondToBuyer) = abi.decode(
            _split(_amount, buyerBps, _premium, _deposit, lfHeld, pToJudge, lfToJudge, bondTo, 0),
            (uint256, uint256, uint256, uint256, uint256)
        );
        if (judgePaid != 0) {
            SeatRec storage s = _last();
            if (pToJudge) s.standbyCredited += _premium;
            if (lfToJudge) s.rulingCredited += lfHeld;
            _claimable[s.judge][_token] += judgePaid;
            unpaid += judgePaid;
        }
        buyerPaid += stray;
        lfHeld = 0;
        _deposit = 0;
        _premium = 0;
        if (buyerPaid != 0) _credit(buyerPayout, buyerPaid);
        if (sellerA != 0) _credit(sellerPayout, sellerA);
        if (toSellerFund != 0) _credit(sellerFund, toSellerFund);
        emit Closed(uint8(terminal) - 4, buyerPaid, sellerA + toSellerFund, judgePaid, bondToBuyer);
        _pushOwed(buyerPayout);
        _pushOwed(sellerPayout);
        if (sellerFund != sellerPayout) _pushOwed(sellerFund);
    }

    /// The verifier's `split` (s8): the terminal's amounts, then the Outcome of the same action.
    function _split(uint256 a, uint16 bps, uint256 p, uint256 d, uint256 lf, bool pj, bool lj, uint8 bondTo, uint8 st)
        internal
        view
        returns (bytes memory)
    {
        return _verify(abi.encodeWithSelector(EravonVerifier.split.selector, a, bps, p, d, lf, pj, lj, bondTo, st));
    }

    function _finishResigned(bytes32 entryId, uint8 form) internal {
        if (form == 3) emit Resigned(entryId, 3);
        pendingRedraw = false;
        _finish(Phase.Resigned, 10000, false, false, 0);
    }

    function _credit(address payout, uint256 v) internal {
        owed[payout][_token] += v;
        unpaid += v;
    }

    /// Best effort, under PUSH_GAS: a failed push leaves the credit for `sweep`, whose caller hands the token all its gas.
    function _pushOwed(address payout) internal {
        uint256 o = owed[payout][_token];
        if (o == 0) return;
        owed[payout][_token] = 0;
        uint256 d = _debit(_transfer(_token, payout, o, PUSH_GAS), o);
        owed[payout][_token] = o - d;
        if (d != 0) _swept(payout, _token, d);
    }

    function _swept(address who, address token_, uint256 v) internal {
        if (token_ == _token) unpaid -= v;
        emit Swept(who, token_, v);
    }

    /// Balance beyond everything the ledger accounts for.
    function _stray() internal view returns (uint256) {
        uint256 bal = _balance(_token);
        uint256 held = unpaid;
        if (_phase < Phase.Refunded) held += _amount + _deposit + lfHeld;
        return bal > held ? bal - held : 0;
    }

    /// transferFrom measured by the balance received.
    function _pull(address from, uint256 v) internal returns (uint256 got) {
        uint256 before = _balance(_token);
        if (!_transferFromOk(_token, from, address(this), v)) revert TransferFailed();
        uint256 after_ = _balance(_token);
        got = after_ > before ? after_ - before : 0;
    }

    /// The logic's doors (s7a, s4, s9) run on the logic alone; an instance holds no registry.
    function _onLogic() internal view {
        if (address(this) != LOGIC) revert NotRegistry();
    }

    /// `transferFrom(from, to, v)` on `token_`, accepted where the call succeeds and returns no data or exactly one
    /// 32-byte word equal to 1 (s4 step 7, s8).
    function _transferFromOk(address token_, address from, address to, uint256 v) internal returns (bool) {
        (bool ok, bytes memory ret) = token_.call(abi.encodeWithSelector(IERC20.transferFrom.selector, from, to, v));
        return ok && (ret.length == 0 || (ret.length == 32 && abi.decode(ret, (uint256)) == 1));
    }

    /// A push counts by what left this account, whatever the token answered:
    /// a token that moves the balance and returns junk has paid, and one that
    /// returns true and moves nothing has not.
    function _transfer(address token_, address to, uint256 v, uint256 gasCap) internal returns (uint256) {
        uint256 before = _balance(token_);
        bytes memory data = abi.encodeWithSelector(IERC20.transfer.selector, to, v);
        uint256 g = gasCap == 0 ? gasleft() : gasCap;
        bool ok;
        // no return data is copied back: the push is counted by the balance that left, and a long answer costs
        // the caller nothing beyond the allowance
        assembly ("memory-safe") {
            ok := call(g, token_, 0, add(data, 32), mload(data), 0, 0)
        }
        if (!ok) return 0;
        uint256 after_ = _balance(token_);
        return after_ < before ? before - after_ : 0;
    }

    /// The token's balance of this account, zero where the token does not answer (no code, a revert, a short answer).
    function _balance(address token_) internal view returns (uint256) {
        (bool ok, bytes memory r) = token_.staticcall(abi.encodeWithSelector(IERC20.balanceOf.selector, address(this)));
        if (!ok || r.length < 32) return 0;
        return abi.decode(r, (uint256));
    }

    /// A credit is debited by what left, never by more than itself.
    function _debit(uint256 sent, uint256 credit) internal pure returns (uint256) {
        return sent < credit ? sent : credit;
    }

    // ------------------------------------------------------------------
    // helpers
    // ------------------------------------------------------------------

    /// The key that acts for `a` now, the succession book's answer (s9, venue/1 s1); a chain without the book's
    /// code at its address reverts every gate that asks.
    function _cur(address a) internal view returns (address) {
        return IGaleedSuccessionBook(SUCCESSION_BOOK).current(a);
    }

    /// s9: a signed form, settle and the like are sent by either party's current key.
    function _party() internal view {
        if (msg.sender != _cur(sellerKey) && msg.sender != _cur(buyerKey)) revert NotParty();
    }

    function _open() internal view returns (bool) {
        return _phase == Phase.Funded || _phase == Phase.Claimed || _phase == Phase.Disputed;
    }

    function _last() internal view returns (SeatRec storage) {
        if (_seats.length == 0) revert NotOpen();
        return _seats[_seats.length - 1];
    }

    function _seatChecked(bytes32 id, uint64 seat) internal view returns (SeatRec storage) {
        if (_phase == Phase.None || id != caseId || seat >= _seats.length) revert UnknownSeat();
        return _seats[seat];
    }

    /// venue/1 s2.9: a direct call from the seated judge's current key.
    function _direct(SeatRec storage s) internal view {
        if (msg.sender != _cur(s.judge) || msg.sender != tx.origin || msg.sender.code.length != 0) revert NotDirect();
    }

    /// terms/1 s5.1, on the verifier: a 65-byte signature over the digest by the seller's current key, or BadSellerSig.
    function _verifySeller(bytes32 digest, bytes calldata sig) internal view {
        _verify(abi.encodeWithSelector(EravonVerifier.verifySeller.selector, digest, sig, sellerKey));
    }

    /// terms/1 s5.2, on the verifier: by the current key of `key`; its fault bubbles up unchanged.
    function _verifyBuyer(bytes32 digest, bytes calldata sig, address key) internal view {
        _verify(abi.encodeWithSelector(EravonVerifier.verifyBuyer.selector, digest, sig, key));
    }

    /// A static call to the verifier designated by code; a revert there is a revert here, same bytes.
    function _verify(bytes memory data) internal view returns (bytes memory ret) {
        if (VERIFIER.codehash != VERIFIER_CODEHASH) revert VerifierMissing();
        bool ok;
        (ok, ret) = VERIFIER.staticcall(data);
        if (!ok) {
            assembly ("memory-safe") {
                revert(add(ret, 32), mload(ret))
            }
        }
    }
}
