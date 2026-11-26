// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {IGaleedVenue} from "./IGaleedVenue.sol";
import {IERC20, IEravonPanelBook} from "./interfaces.sol";
import {EravonPanelBook} from "./EravonPanelBook.sol";
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
/// of funding bonds (s7a), nothing else.
///
/// Constraints in comments only. Section numbers refer to the standard
/// document unless marked venue/1 (galeed.venue/1) or terms/1 (eravon.terms/1).
contract Eravon is IGaleedVenue {
    // ------------------------------------------------------------------
    // constants
    // ------------------------------------------------------------------

    /// The public keyless CREATE2 deployer (Shape, s1).
    address internal constant DEPLOYER = 0x4e59b44847b379578588920cA78FbF26c0B4956C;

    // EIP-712 (s9, terms/1 s4.2, s4.5)
    bytes32 internal constant TH_DOMAIN =
        keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)");
    bytes32 internal constant NAME_HASH = keccak256("ERAVON");
    bytes32 internal constant VERSION_HASH = keccak256("1");
    bytes32 internal constant TH_ORDER = keccak256(
        "OrderTerms(Offer offer,address buyerKey,address buyerPayout,uint64 openBy)Offer(address sellerKey,address judge,address guild,address sellerPayout,address sellerFund,address token,uint256 amount,uint32 deliveryWindow,uint32 confirmWindow,uint32 openWindow,bool buyerMayOpen,Outcome[] ladder,bytes32 termsHash,bytes32 amanahHash,bytes32 quoteHash,uint256 nonce,uint256 expiry)Outcome(uint16 buyerBps,uint16 sellerBps,uint8 feeBearer)"
    );
    bytes32 internal constant TH_CLAIM = keccak256("ClaimDelivery(bytes32 manifest)");
    bytes32 internal constant TH_CONFIRM = keccak256("Confirm()");
    bytes32 internal constant TH_DISPUTE = keccak256("Dispute()");
    bytes32 internal constant TH_REFUND = keccak256("Refund()");
    bytes32 internal constant TH_SETTLE = keccak256("Settle(uint16 buyerBps,uint256 expiry)");
    bytes32 internal constant TH_CANCEL = keccak256("Cancel()");

    /// venue/1 s2.5: the quote form, constant for the life of the standard (s3).
    string internal constant QUOTE_TYPE =
        "(address token,address payout,uint16 premiumBps,address feeUnit,address feedToken,address feedUnit,uint32 maxStale,uint256 feeFixed,uint16 feeBps,uint256 feeMin,uint256 feeMax,uint256 amountCap,uint32 rulingWindow,uint32 offerWindow,uint8 lapseDefault,bool autoAccept)";

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
    /// The panel book's initcode hash and runtime code hash (s7).
    bytes32 internal immutable PANEL_INITCODE_HASH;
    bytes32 internal immutable PANEL_CODEHASH;
    /// The verifier's address and runtime code hash (Shape), designated by code.
    address internal immutable VERIFIER;
    bytes32 internal immutable VERIFIER_CODEHASH;

    // ------------------------------------------------------------------
    // types
    // ------------------------------------------------------------------

    struct Outcome {
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
        Outcome[] ladder;
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
    }

    // ------------------------------------------------------------------
    // storage (one instance = one case)
    // ------------------------------------------------------------------

    Phase internal _phase;
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
    Outcome[] internal _ladder;
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

    // the guild track (s7)
    uint256 internal panelIndex;
    bytes32 internal panelHash;
    uint256 internal panelSize;
    bool internal pendingRedraw;
    uint64 internal redrawDeadline;
    /// The block whose hash seeds the pending draw (s7): the block of the commit, read from the next block on.
    uint64 internal drawBlock;

    SeatRec[] internal _seats;

    // the economic face (venue/1 s3)
    mapping(address => address) internal _payoutOf;
    mapping(address => mapping(address => uint256)) internal _claimable;
    /// Parties' money by payout address and token (s8); never through the seat ledger.
    mapping(address => mapping(address => uint256)) public owed;

    /// The registry of consent by call (s9, terms/1 s5.4), live on the
    /// logic's own storage alone: a digest a sender recorded under its own
    /// address. Never removed; the digest's own expiry ends it.
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
    event PanelFrozen(address indexed guild, uint256 index, bytes32 panelHash);
    event DrawArmed(uint64 drawBlock, uint64 deadline);
    event DeliveryClaimed(bytes32 manifest);
    event Confirmed();
    event Disputed();
    event Settled(uint16 buyerBps);
    event Ruled(uint64 rung, bytes32 entryId);
    event Resigned(bytes32 entryId, uint8 form); // 0 declined, 1 before dispute, 2 during dispute, 3 pool exhausted
    event Lapsed(uint64 rung);
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
    error BadPasskey(uint8 test);
    error Underfunded();
    error OverCap();
    error RateStale();
    error DepositShort();
    error FundUnbound();
    error PanelBookMissing();
    error VerifierMissing();
    error PanelMismatch();
    error PanelExpired();
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
    error DrawPending();
    error FeeOverflow();
    error NotForeign();

    /// The gas a best-effort push of the standard's own (a terminal's, cancel's, the expiry branch's) hands the
    /// token (s8): enough for any plain or proxied ERC-20, bounded so that a recipient's hook cannot put the
    /// call beyond the transaction gas cap nor charge its submitter the whole limit.
    uint256 internal constant PUSH_GAS = 1_000_000;

    modifier nonReentrant() {
        if (_entered != 0) revert Reentered();
        _entered = 1;
        _;
        _entered = 0;
    }

    // ------------------------------------------------------------------
    // construction (Shape, s13)
    // ------------------------------------------------------------------

    constructor() {
        LOGIC = address(this);
        bytes memory runtime = abi.encodePacked(RUNTIME_PREFIX, address(this), RUNTIME_SUFFIX);
        PROXY_CODEHASH = keccak256(runtime);
        PROXY_INITCODE_HASH = keccak256(abi.encodePacked(INIT_PREFIX, runtime));
        PANEL_INITCODE_HASH = keccak256(type(EravonPanelBook).creationCode);
        PANEL_CODEHASH = keccak256(type(EravonPanelBook).runtimeCode);
        VERIFIER = _create2(keccak256("eravon/1/verify"), keccak256(type(EravonVerifier).creationCode));
        VERIFIER_CODEHASH = keccak256(type(EravonVerifier).runtimeCode);
    }

    // ------------------------------------------------------------------
    // s4: the gate
    // ------------------------------------------------------------------

    /// Opens the case, or, on the expiry branch, refunds the address and opens
    /// nothing. `members` and `amanahHashes` are empty on the named track and
    /// the panel read from the guild's book on the guild track (s7). The order
    /// of decision is s4's, steps 1 to 8.
    function init(
        OrderTerms calldata t,
        bytes calldata quote,
        bytes calldata sigBuyer,
        bytes calldata sigSeller,
        address[] calldata members,
        bytes32[] calldata amanahHashes
    ) external nonReentrant {
        if (_phase != Phase.None) revert AlreadyInitialized();
        bytes32 salt = _saltOf(t);
        if (address(this) != _create2(salt, PROXY_INITCODE_HASH)) revert WrongAddress();
        if (
            block.timestamp > t.offer.expiry || block.timestamp > t.openBy
                || Eravon(LOGIC).revoked(t.offer.sellerKey, t.offer.termsHash)
        ) {
            // the expiry branch: no signature, no quote, no feed, no case; the payout and the token are kept so that
            // a later balance leaves by this branch again and another token by sweep; what is owed already from an
            // earlier run whose push failed is not credited twice; the push is best effort
            if (t.buyerPayout == address(0)) revert BadTerms(4);
            buyerPayout = t.buyerPayout;
            _token = t.offer.token;
            uint256 bal = _stray();
            if (bal != 0) _credit(t.buyerPayout, bal);
            _pushOwed(t.buyerPayout);
            emit Refunded(2);
            return;
        }
        if (!t.offer.buyerMayOpen && msg.sender != t.offer.sellerKey) revert NotSeller();
        // the verifier reads the whole calldata once: the terms guards, the quote, the pricing
        EravonVerifier.Checked memory c =
            abi.decode(_verify(abi.encodePacked(EravonVerifier.check.selector, msg.data[4:])), (EravonVerifier.Checked));
        if (c.termsFault != 0) revert BadTerms(c.termsFault);
        if (c.quoteBad) revert BadQuote();
        if (keccak256(quote) != t.offer.quoteHash) revert QuoteMismatch();
        if (address(uint160(c.q[0])) != t.offer.token) revert QuoteMismatch();
        // two consents (s9): a signature, or, on an empty slot, the sender or the registry (terms/1 s5.4)
        _consents(
            t.offer.sellerKey,
            _digest(LOGIC, _verifyOfferHash(t.offer)),
            sigSeller,
            t.buyerKey,
            _digest(address(this), salt),
            sigBuyer
        );
        // the deposit address (s7a): the seller's own key, or an address bound to the seller on the logic
        if (t.offer.sellerFund != t.offer.sellerKey && !Eravon(LOGIC).funds(t.offer.sellerKey, t.offer.sellerFund)) {
            revert FundUnbound();
        }
        uint256 balance = _balance(t.offer.token);
        if (balance < t.offer.amount) revert Underfunded();

        _storeTerms(t, salt);
        _price(c);
        // the seat's preconditions before any money moves: the named track takes no panel, the guild track freezes one
        if (t.offer.judge != address(0)) {
            if (members.length != 0 || amanahHashes.length != 0) revert BadTerms(20);
            // named track: the judge's payout pre-registered from the quote (s3)
            if (c.q[1] != 0 && _payoutOf[t.offer.judge] == address(0)) _payoutOf[t.offer.judge] = address(uint160(c.q[1]));
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
        emit Opened(salt, t.offer.sellerKey, t.buyerKey, judge, t.offer.amount, t.offer.termsHash);
    }

    /// s4: the buyer's cancellation before any case opened. The address
    /// proves itself; the buyer is the caller with an empty `sig`, or `sig`
    /// is the buyer's signature over Cancel() under this instance's domain
    /// (terms/1 s4.5); the whole balance goes to the buyer's payout and the
    /// instance holds a closed case, so that `init` after it is refused.
    function cancel(OrderTerms calldata t, bytes calldata sig) external nonReentrant {
        if (_phase != Phase.None) revert AlreadyInitialized();
        bytes32 salt = _saltOf(t);
        if (address(this) != _create2(salt, PROXY_INITCODE_HASH)) revert WrongAddress();
        if (t.buyerKey == address(0)) revert NotBuyer();
        if (t.buyerPayout == address(0)) revert BadTerms(4);
        if (sig.length == 0) {
            if (msg.sender != t.buyerKey) revert NotBuyer();
        } else {
            _verifyBuyer(_digest(address(this), keccak256(abi.encode(TH_CANCEL))), sig, t.buyerKey);
        }
        buyerPayout = t.buyerPayout;
        buyerKey = t.buyerKey;
        _token = t.offer.token;
        caseId = salt;
        // the balance beyond what an earlier expiry-branch run left owed; Closed reports what this call credited
        uint256 bal = _stray();
        _phase = Phase.Refunded;
        if (bal != 0) _credit(t.buyerPayout, bal);
        emit Refunded(4);
        emit Closed(0, bal, 0, 0, 0);
        _pushOwed(t.buyerPayout);
    }

    /// The order's salt, caseId and CREATE2 salt alike (s1): hashStruct(OrderTerms) of terms/1 s4.2.
    function _saltOf(OrderTerms calldata t) internal view returns (bytes32) {
        return keccak256(abi.encode(TH_ORDER, _verifyOfferHash(t.offer), t.buyerKey, t.buyerPayout, uint256(t.openBy)));
    }

    /// hashStruct(Offer) on the verifier (pure there, read through the designated code).
    function _verifyOfferHash(Offer calldata o) internal view returns (bytes32) {
        return abi.decode(_verify(abi.encodeWithSelector(EravonVerifier.hashOffer.selector, o)), (bytes32));
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
    }

    /// s3: the fee priced once and pinned, as the verifier computed it; the
    /// quote's clocks and switches; the stale rate refused here, after the
    /// signatures.
    function _price(EravonVerifier.Checked memory c) internal {
        uint256[16] memory q = c.q;
        premiumBps = uint16(q[2]);
        rulingWindow = uint32(q[12]);
        offerWindow = uint32(q[13]);
        lapseDefault = uint8(q[14]);
        autoAccept = q[15] == 1;
        address unit = address(uint160(q[3]));
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
    // s7a, s4: the logic's three registries and the deposit face (on the logic alone)
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

    /// s4: the seller's revocation of a document, recorded under its own
    /// address on the logic alone; never withdrawn.
    function revoke(bytes32 termsHash) external {
        _onLogic();
        _revoked[msg.sender][termsHash] = true;
        emit Revoked(msg.sender, termsHash);
    }

    /// The revocation registry's answer, read on the logic (an instance holds none and answers false).
    function revoked(address seller, bytes32 termsHash) external view returns (bool) {
        return _revoked[seller][termsHash];
    }

    /// s9: the sender records a digest under its own address; no removal. Accepted
    /// on the logic alone, so that an instance never holds a registry of its own.
    function consent(bytes32 digest) external {
        _onLogic();
        _consented[msg.sender][digest] = true;
        emit Consented(msg.sender, digest);
    }

    /// The registry's answer, read on the logic (an instance holds no registry and answers false).
    function consented(address who, bytes32 digest) external view returns (bool) {
        return _consented[who][digest];
    }

    // ------------------------------------------------------------------
    // s7: the guild track
    // ------------------------------------------------------------------

    /// The guild's current panel, read and checked on the verifier (the book's code, the panel's hash, its
    /// expiry, its words, its pool of members that are neither party), then frozen here.
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
        (uint256 index, bytes32 ph, uint256 pool) = abi.decode(ret, (uint256, bytes32, uint256));
        panelIndex = index;
        panelHash = ph;
        panelSize = pool;
        emit PanelFrozen(g, index, ph);
    }

    /// Arms a draw (s7): the seed is the hash of this block, which nobody sending in it can see; the draw
    /// is open to anyone from the next block until offerWindow from now.
    function _armDraw() internal {
        pendingRedraw = true;
        redrawDeadline = uint64(block.timestamp) + offerWindow;
        drawBlock = uint64(block.number);
        emit DrawArmed(drawBlock, redrawDeadline);
    }

    /// Draws one member not seated before on this case (s7), from the armed block's hash.
    function _draw(address[] calldata members, bytes32 seed, uint64 seat) internal view returns (address) {
        uint256 n = members.length;
        uint256 eligible = 0;
        for (uint256 i = 0; i < n; i++) {
            if (_eligible(members[i]) && !_seatedBefore(members[i])) eligible++;
        }
        if (eligible == 0) revert NoMembers();
        uint256 pick = uint256(keccak256(abi.encodePacked(caseId, seed, seat))) % eligible;
        for (uint256 i = 0; i < n; i++) {
            if (!_eligible(members[i]) || _seatedBefore(members[i])) continue;
            if (pick == 0) return members[i];
            pick--;
        }
        revert NoMembers();
    }

    /// A drawn judge is neither party's key, as a named judge is not (s4).
    function _eligible(address who) internal view returns (bool) {
        return who != address(0) && who != sellerKey && who != buyerKey;
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
        SeatRec memory s;
        s.judge = judge;
        if (autoAccept) {
            s.accepted = uint64(block.timestamp);
            s.windowStart = uint64(block.timestamp);
            s.windowEnd = uint64(block.timestamp) + deliveryWindow;
        } else {
            s.windowStart = uint64(block.timestamp);
            s.windowEnd = uint64(block.timestamp) + offerWindow;
        }
        _seats.push(s);
        emit Seated(judge, caseId, seat);
    }

    /// Anyone, with the frozen panel: the draw of the next seat, the first included, from the armed block's
    /// hash, from the block after the arming on; where the chain no longer serves that hash (more than 256
    /// blocks have passed, or the chain serves none), the case ends as Resigned with form 3: a seed nobody
    /// drew on buys no second one.
    function redraw(address[] calldata members, bytes32[] calldata amanahHashes) external nonReentrant {
        // the block after the arming may always draw, so no offerWindow makes a draw impossible
        if (!pendingRedraw || (block.timestamp > redrawDeadline && block.number > drawBlock + 1)) revert NotPendingRedraw();
        if (keccak256(abi.encode(members, amanahHashes)) != panelHash) revert PanelMismatch();
        if (block.number <= drawBlock) revert DrawPending();
        bytes32 seed = blockhash(drawBlock);
        pendingRedraw = false;
        if (seed == bytes32(0)) {
            _finishResigned(bytes32(0), 3);
            return;
        }
        _openSeat(_draw(members, seed, uint64(_seats.length)));
    }

    /// After an unaccepted guild seat ended: a draw is armed, or the pool is exhausted.
    function _afterUnacceptedSeatEnded() internal {
        if (_seats.length >= 3 || _seats.length >= panelSize) {
            _finishResigned(bytes32(0), 3);
        } else {
            _armDraw();
        }
    }

    // ------------------------------------------------------------------
    // venue/1 case face
    // ------------------------------------------------------------------

    function seatOf(bytes32 id, uint64 seat) external view returns (Seat memory s) {
        if (_phase == Phase.None || id != caseId || seat >= _seats.length) revert UnknownSeat();
        SeatRec storage r = _seats[seat];
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

    /// venue/1 s2.7: the judge's word on an open offer.
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
        emit Anchored(msg.sender, entryId, entry);
        emit Ruled(rung, entryId);
        Outcome storage o = _ladder[rung];
        // the seller's LF: to the buyer where the seller bears the fee, back to the seller where the buyer does, half each otherwise
        uint8 bondTo = o.feeBearer == 1 ? 1 : (o.feeBearer == 2 ? 2 : 0);
        _finish(Phase.Ruled, o.buyerBps, true, true, bondTo);
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
            _lapse(uint64(_lapseRung()));
        }
    }

    /// venue/1 s2.11: the clocks (s5), by anyone; nothing due, nothing changed.
    function advance(bytes32 id, uint64 seat) external nonReentrant {
        if (_phase == Phase.None || id != caseId) return;
        if (seat < _seats.length) _advanceSeat(seat);
        // the venue's other clock: a draw nobody made, by its deadline (the block after the arming excepted) or
        // by the loss of its seed; run whatever seat the call names
        if (pendingRedraw && block.number > drawBlock + 1
                && (block.timestamp > redrawDeadline || block.number > drawBlock + 256)) {
            pendingRedraw = false;
            _finishResigned(bytes32(0), 3);
        }
    }

    function _advanceSeat(uint64 seat) internal {
        SeatRec storage s = _seats[seat];
        bool inFlight = s.state <= 1;
        bool resignedInClaimed = s.state == 3 && _phase == Phase.Claimed && seat == _seats.length - 1;
        if ((inFlight || resignedInClaimed) && s.windowEnd != 0 && block.timestamp > s.windowEnd) {
            if (s.accepted == 0) {
                // the offer expired
                s.state = 5;
                if (guild == address(0)) {
                    emit Refunded(1);
                    _finish(Phase.Refunded, 10000, false, false, 0);
                } else {
                    _afterUnacceptedSeatEnded();
                }
            } else if (_phase == Phase.Funded) {
                // T1: delivery lapsed
                s.state = 5;
                emit Refunded(0);
                _finish(Phase.Refunded, 10000, false, false, 0);
            } else if (_phase == Phase.Claimed) {
                // T2: confirmation lapsed; P to the judge where the seat is seated, back to the seller where it resigned
                bool seated = s.state == 0;
                if (seated) s.state = 5;
                _finish(Phase.Released, 0, seated, false, 0);
            } else if (_phase == Phase.Disputed) {
                // T3: the judge's silence
                s.state = 4;
                _lapse(uint64(_lapseRung()));
            }
        }
    }

    /// The rung `lapseDefault` names: 0 rung 0, 2 the last rung (terms/1 s3.3).
    function _lapseRung() internal view returns (uint256) {
        return lapseDefault == 0 ? 0 : _ladder.length - 1;
    }

    /// Lapsed at `rung`: P and the deposit back to the seller, the fee back to the buyer (s8).
    function _lapse(uint64 rung) internal {
        emit Lapsed(rung);
        _finish(Phase.Lapsed, _ladder[rung].buyerBps, false, false, 0);
    }

    // ------------------------------------------------------------------
    // s9: the parties' actions
    // ------------------------------------------------------------------

    function claimDelivery(bytes32 manifest) external nonReentrant {
        if (msg.sender != sellerKey) revert NotSeller();
        _claimDelivery(manifest);
    }

    function claimDeliveryBySig(bytes32 manifest, bytes calldata sig) external nonReentrant {
        if (manifest == bytes32(0)) revert ZeroManifest();
        bytes32 h = keccak256(abi.encode(TH_CLAIM, manifest));
        _verifySeller(_digest(address(this), h), sig);
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
        if (msg.sender != buyerKey) revert NotBuyer();
        _confirm();
    }

    function confirmBySig(bytes calldata sig) external nonReentrant {
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
        if (msg.sender != buyerKey) revert NotBuyer();
        _dispute();
    }

    function disputeBySig(bytes calldata sig) external nonReentrant {
        _verifyBuyer(_digest(address(this), keccak256(abi.encode(TH_DISPUTE))), sig, buyerKey);
        _dispute();
    }

    /// The buyer's dispute while Claimed: with the seat seated, the fee is
    /// taken and T3 runs; with the seat resigned (s6), no fee is taken and the
    /// case lapses at once at the rung `lapseDefault` names.
    function _dispute() internal {
        if (_phase != Phase.Claimed) revert WrongPhase();
        SeatRec storage s = _last();
        if (block.timestamp > s.windowEnd) revert WindowClosed();
        if (s.state == 3) {
            emit Disputed();
            _lapse(uint64(_lapseRung()));
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

    /// The litigation fee: what already flowed in during Claimed, then the rest from the buyer's key (s8).
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
        if (msg.sender != sellerKey) revert NotSeller();
        _refund();
    }

    /// The signed form is refused throughout a dispute: a Refund() signed in an earlier state is submitted only
    /// outside one, so a signature handed out while Funded cannot pick the refund over the lapse or the ruling;
    /// the signature is tested first, as every signed form tests its actor first (s10).
    function refundBySig(bytes calldata sig) external nonReentrant {
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

    /// Anyone, where both consents to the Settle message hold: a signature,
    /// or on an empty slot the caller or the logic's registry (s9).
    function settle(uint16 buyerBps, uint256 settleExpiry, bytes calldata sigB, bytes calldata sigS) external nonReentrant {
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

    function setPayout(address payout) external {
        if (address(this) == LOGIC) revert NotInstance();
        if (_payoutOf[msg.sender] != address(0)) revert PayoutSet();
        if (payout == address(0)) revert ZeroPayout();
        _payoutOf[msg.sender] = payout;
    }

    function withdraw(address token_) external nonReentrant {
        address p = _payoutOf[msg.sender];
        if (p == address(0)) revert NoPayout();
        uint256 c = _claimable[msg.sender][token_];
        if (c != 0) {
            _claimable[msg.sender][token_] = 0;
            uint256 d = _debit(_transfer(token_, p, c, 0), c);
            if (d == 0) revert TransferFailed();
            _claimable[msg.sender][token_] = c - d;
            _swept(msg.sender, token_, d);
        }
    }

    function accountOf(address who, address token_) external view returns (address payout, uint256 claimable) {
        return (_payoutOf[who], _claimable[who][token_]);
    }

    /// Anyone: pushes what `who` is owed (a party's payout address, the order's `sellerFund`, or a seat with a registered payout).
    function sweep(address who, address token_) external nonReentrant {
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

    /// venue/1 s4: the panel face lives on the guild's book (s7).
    function engage(bytes32, address[] calldata, bytes32[] calldata, bytes32, uint64) external pure {
        revert NoPanelFace();
    }

    /// The two parties' consents, decided on the verifier: a signature over
    /// the party's digest, or, on an empty slot, the sender being the party or
    /// the logic's registry holding the digest under the party (terms/1 s5.4);
    /// the party's fault bubbles up unchanged.
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

    function panelInitCodeHash() external view returns (bytes32) {
        return PANEL_INITCODE_HASH;
    }

    /// The guild's book address (s7).
    function panelBookOf(address g) public view returns (address) {
        return _create2(keccak256(abi.encodePacked("eravon/1/panel", g)), PANEL_INITCODE_HASH);
    }

    /// The case's own state beyond the interface, for clients and indexers;
    /// the terms are in the init calldata and the Opened event. The desk reads none of it.
    function status()
        external
        view
        returns (
            Phase phase,
            uint256 seats,
            uint256 premium,
            uint256 deposit,
            uint256 lfHeld_,
            uint256 unpaid_,
            uint64 claimedAt_,
            uint64 disputedAt_,
            bool pendingRedraw_,
            uint64 redrawDeadline_,
            uint64 drawBlock_,
            uint256 panelIndex_,
            bytes32 panelHash_,
            uint256 panelSize_
        )
    {
        return (
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
            panelSize
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

    /// Splits A by `buyerBps` through the verifier's `split` (s8); sends P to
    /// the judge where `pToJudge`, back to
    /// `sellerFund` otherwise; sends the buyer's held LF to the judge or
    /// back to the buyer; disposes of the seller's LF (0 seller, 1 buyer, 2
    /// half each); returns every stray to the buyer; credits every payout,
    /// and pushes what it can.
    function _finish(Phase terminal, uint16 buyerBps, bool pToJudge, bool lfToJudge, uint8 bondTo) internal {
        // what flowed in unrecorded, read while the ledger still counts A, D and the fee: the buyer's
        uint256 stray = _stray();
        _phase = terminal;
        pendingRedraw = false;
        address judge = _seats.length == 0 ? address(0) : _last().judge;
        // the split (s8), pure, on the verifier: A by buyerBps, P and the buyer's fee by the flags, the seller's LF by bondTo
        (uint256 buyerPaid, uint256 sellerA, uint256 judgePaid, uint256 toSellerFund, uint256 bondToBuyer) = abi.decode(
            _verify(
                abi.encodeWithSelector(
                    EravonVerifier.split.selector, _amount, buyerBps, _premium, _deposit, lfHeld, pToJudge, lfToJudge, bondTo
                )
            ),
            (uint256, uint256, uint256, uint256, uint256)
        );
        buyerPaid += stray;
        lfHeld = 0;
        _deposit = 0;
        _premium = 0;
        if (judgePaid != 0) {
            _claimable[judge][_token] += judgePaid;
            unpaid += judgePaid;
        }
        if (buyerPaid != 0) _credit(buyerPayout, buyerPaid);
        if (sellerA != 0) _credit(sellerPayout, sellerA);
        if (toSellerFund != 0) _credit(sellerFund, toSellerFund);
        emit Closed(uint8(terminal) - 4, buyerPaid, sellerA + toSellerFund, judgePaid, bondToBuyer);
        _pushOwed(buyerPayout);
        _pushOwed(sellerPayout);
        if (sellerFund != sellerPayout) _pushOwed(sellerFund);
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

    /// venue/1 s2.9: a direct call from the seated key.
    function _direct(SeatRec storage s) internal view {
        if (msg.sender != s.judge || msg.sender != tx.origin || msg.sender.code.length != 0) revert NotDirect();
    }

    /// terms/1 s5.1, on the verifier: the seller's 65-byte signature over the digest, or BadSellerSig.
    function _verifySeller(bytes32 digest, bytes calldata sig) internal view {
        _verify(abi.encodeWithSelector(EravonVerifier.verifySeller.selector, digest, sig, sellerKey));
    }

    /// terms/1 s5.2 and s5.3, on the verifier; its fault bubbles up unchanged.
    function _verifyBuyer(bytes32 digest, bytes calldata sig, address key) internal view {
        _verify(abi.encodeWithSelector(EravonVerifier.verifyBuyer.selector, digest, sig, key));
    }

    /// A static call to the verifier designated by code; a revert there is a revert here, same bytes.
    function _verify(bytes memory data) internal view returns (bytes memory ret) {
        if (VERIFIER.codehash != VERIFIER_CODEHASH) revert VerifierMissing();
        bool ok;
        (ok, ret) = VERIFIER.staticcall(data);
        if (!ok) {
            assembly {
                revert(add(ret, 32), mload(ret))
            }
        }
    }
}
