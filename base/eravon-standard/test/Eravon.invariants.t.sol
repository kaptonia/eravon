// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {VmSafe} from "forge-std/Vm.sol";
import {EravonBase} from "./EravonBase.sol";
import {Eravon} from "../src/Eravon.sol";
import {EravonPanelBook} from "../src/EravonPanelBook.sol";
import {EravonVerifier} from "../src/EravonVerifier.sol";
import {IGaleedVenue, RouteView} from "../src/IGaleedVenue.sol";
import {MockERC20} from "./replay/MockERC20.sol";
import {Company, Impostor, MockUSD} from "./mocks/Mocks.sol";
import {
    BlockableToken, InvShapedFeed, InvNoCopy, R5Token, R5Feed, R5Hostile, SecondEntry, MuteToken, InvGasFeed
} from "./mocks/InvMocks.sol";

/// FUZZ r24-1: a router whose answer the handler sets between draws (the draw reaches it by staticcall, which
/// reads the mode and writes nothing). Every answer is given only where the router was handed exactly the gas
/// `expect` names as its entry gas (the handler calibrates it at routeGas()); handed any other gas it answers an
/// index outside every panel, which ends the case (s7). Modes: 0 the first member no seat holds; 1 a revert with
/// a reason; 2 an empty revert; 3 the index `at`; 4 the index the whole view hashes to; 5 two words; 6 31 bytes;
/// 7 no return data; 8 every unit of gas it is handed; 9 a storage write; 10 the invalid opcode; 11 33 bytes;
/// 12 a revert with 4096 bytes; 99 its entry gas (the calibration).
contract InvRouter {
    uint8 public mode;
    uint256 public at;
    uint256 public expect;
    uint256 public writes;

    function set(uint8 m, uint256 a) external {
        mode = m;
        at = a;
    }

    function setExpect(uint256 e) external {
        expect = e;
    }

    function route(RouteView calldata v) external returns (uint256) {
        uint256 g = gasleft();
        uint8 m = mode;
        if (m == 99) return g;
        if (expect != 0 && g != expect) return type(uint256).max;
        if (m == 0) return _first(v);
        if (m == 1) revert("refused");
        if (m == 2) {
            assembly {
                revert(0, 0)
            }
        }
        if (m == 3) return at;
        if (m == 4) return uint256(keccak256(abi.encode(v))) % v.members.length;
        if (m == 5) {
            assembly {
                mstore(0, 0)
                mstore(32, 0)
                return(0, 64)
            }
        }
        if (m == 6) {
            assembly {
                mstore(0, 0)
                return(0, 31)
            }
        }
        if (m == 7) {
            assembly {
                return(0, 0)
            }
        }
        if (m == 8) {
            assembly {
                for {} 1 {} {}
            }
        }
        if (m == 9) {
            writes += 1;
            return _first(v);
        }
        if (m == 10) {
            assembly {
                invalid()
            }
        }
        if (m == 11) {
            assembly {
                mstore(0, 0)
                return(0, 33)
            }
        }
        if (m == 12) {
            assembly {
                revert(0, 4096)
            }
        }
        return _first(v);
    }

    function _first(RouteView calldata v) internal pure returns (uint256) {
        for (uint256 i = 0; i < v.members.length; i++) {
            bool taken = false;
            for (uint256 k = 0; k < v.seated.length; k++) {
                if (v.seated[k] == v.members[i]) taken = true;
            }
            if (!taken) return i;
        }
        revert("no member");
    }
}

/// FUZZ r24-1: another router's code, etched over a frozen router so that its code hash differs from the frozen one.
contract InvRouterB {
    function route(RouteView calldata) external pure returns (uint256) {
        return 0;
    }
}

/// The handler of the invariant campaign: opens orders on three tracks (a
/// named judge; a guild of judges; a guild whose panel lists both parties)
/// for parties that hold keys or are contracts acting by call; consents at
/// init and settle by signature, by call, or by
/// the logic's registry; leaves some orders unopened so that cancel, a
/// second init, revocation and the expiry branch race the opening; drives
/// every entry point of the case from every role under a moving clock and
/// block, the guild's two-step draw included (armed at init and after an
/// unaccepted seat, drawn from the next block, re-armed past the blockhash
/// window, closed by its deadline), with strays and foreign tokens sent in,
/// the fee sent ahead exactly, in excess, split, or not at all, payouts that
/// a token blocks, a guild that engages a new panel, and impostors pulling
/// deposits. After each action it reads the logs and the touched case and
/// compares them with standard s5, s7 and s8 as a model; the invariant
/// functions read every case. Round 3 adds: the draw at the edges of its two
/// clocks (the first block after the arming whatever the deadline, the last
/// served block, the lost seed) by redraw and by advance, checked from the
/// pre-state after every call on an armed draw; quotes whose offerWindow is
/// below a block on every track; the settlement against the clock's own
/// outcome at the same moment (the clock run on a snapshot); the seller's
/// refund within T3 and past it, and the signed refund in every phase; every
/// push of the standard's own metered against PUSH_GAS; every revert of an
/// instance checked against the standard's names; and pricing orders whose
/// feeds burn, bomb or cut their answers, against READ_GAS. Round 4 adds:
/// the token's and the feeds' answers shaped to every length for a while
/// (the whole campaign runs under them), the same action on the same state
/// under two shapes (the same decision and outcome, the push's answer
/// costing the caller nothing), pulls answered with what the text refuses,
/// the refund in both forms at every second of T3 and past it, the judge's
/// credit on every Refunded, and stale Refund() signatures in every phase.
/// Round 5 adds: two acts sent for one block run in both orders from one
/// snapshot (every side's total compared where the text makes the order
/// immaterial), three acts reorganised among three slots, the opening, the
/// cancellation, the revocation and a top-up in both orders around init, the
/// three-seat walk on a guild of judges and on a guild of five, hostile
/// foreign tokens swept, odd decimals and reads that flip within one init,
/// EIP-7702 delegated accounts in every role, a stranger's no-gain calls, and
/// every Closed checked where it lands, P against the four rows of s8.
/// Round 6 adds: the order's sellerFund drawn from five (the seller's key, a
/// funder bound to the seller by acceptFunding, an EOA or a contract, the
/// seller's payout, the buyer's payout, and an address not bound to the
/// seller: a stranger, the logic, the verifier, another case's instance),
/// bound where the draw says and sometimes not; the funds' own bonds and
/// allowance changes mid-case; the gate's FundUnbound and the pool's
/// TransferFailed against a model; every Pulled against the bond; the
/// deposit's return at sellerFund at every terminal that returns it.
/// Every act is sent from the current key the succession book answers for
/// the party, the judge or the guild it is the act of (s9): the campaign
/// moves the seller's, the buyer's, a judge's and a guild's record to fresh
/// keys and probes the former key at every gate it held (refused, its
/// signatures refused, its registrations kept); a stranger's submission of a
/// signed form, a settlement, a sweep, init under buyerMayOpen, the
/// expiry branch and a signed cancel is refused NotParty (s9, s10); judges
/// register their payouts on the instance by setPayout or by the first
/// withdraw naming one (s8, venue/1 s3); the outcome views answer what rule
/// and advance then do (s8), and the seat's credited fields carry what the
/// terminal credited.
contract EravonHandler is EravonBase {
    address internal constant STRANGER = address(0x3000000000000000000000000000000000000001);
    address internal constant GUILD = address(0x6000000000000000000000000000000000000001);
    address internal constant GUILD2 = address(0x6000000000000000000000000000000000000002);
    /// Round 5: a guild of five members, so that three seats, and not the pool, bound its cases.
    address internal constant GUILD3 = address(0x6000000000000000000000000000000000000003);
    bytes32 internal constant AMANAH = keccak256("amanah/invariants");
    bytes32 internal constant ROSTER = keccak256("roster");
    bytes internal constant ENTRY = "entry bytes";
    uint256 internal constant MAX_CASES = 40;
    /// The most records the campaign moves per role (the seller's, the buyer's, a judge's, a guild's).
    uint256 internal constant MAX_MOVES = 3;
    /// Cases beyond MAX_CASES that only the draw's clocks open (a guild order armed in this block, nobody drawing).
    uint256 internal constant MAX_FRESH = 12;
    uint256 internal constant MAX_GHOSTS = 12;
    uint256 internal constant MAX_FEED_ORDERS = 24;
    /// EIP-7825's transaction gas cap: every call the handler observes is sent with this much gas.
    uint256 internal constant TX_GAS = 16_777_216;
    /// The standard's allowance for each push of its own (s8).
    uint256 internal constant PUSH_GAS = 1_000_000;
    uint256 internal constant MAX_INT53 = 9007199254740991;
    bytes32 internal E;

    // the action an observation follows
    uint8 internal constant A_INIT = 1;
    uint8 internal constant A_CANCEL = 2;
    uint8 internal constant A_ACCEPT = 3;
    uint8 internal constant A_RULE = 4;
    uint8 internal constant A_RESIGN = 5;
    uint8 internal constant A_ADVANCE = 6;
    uint8 internal constant A_REDRAW = 7;
    uint8 internal constant A_COMMIT = 8;
    uint8 internal constant A_CLAIM = 9;
    uint8 internal constant A_CONFIRM = 10;
    uint8 internal constant A_DISPUTE = 11;
    uint8 internal constant A_REFUND = 12;
    uint8 internal constant A_SETTLE = 13;
    uint8 internal constant A_MONEY = 14;
    uint8 internal constant A_SWEEP = 15;
    uint8 internal constant A_REVOKE = 16;

    struct C {
        Eravon inst;
        Eravon.OrderTerms t;
        bytes quote;
        bytes32 id;
        uint8 track; // 0 named, 1 the guild of judges, 2 the guild whose panel lists both parties
        uint8 sellerKind; // 0 a key, 1 a company
        uint8 buyerKind; // 0 a key, 1 a company
        uint8 sellerForm; // init's slot: 0 a signature, 1 the party's own call, 2 the registry
        uint8 buyerForm;
        uint8 opener; // 0 the seller, 1 the buyer, 2 the seller after a stranger's attempt is refused (s4 step 3, 4)
        uint8 lapseDefault;
        uint8 panel; // the guild's record frozen at init
        uint32 offerWindow;
        uint32 rulingWindow;
        uint256 a;
        uint256 d;
        uint256 lf;
        uint256 p;
        uint8 lastPhase;
        uint8[] seatStates;
        uint64[] seatAccepted;
        bool closedSeen;
        bool opened; // init opened the case (a cancelled order never did)
        uint256 pulled; // the deposit the logic pulled for this case
        uint256 stray; // ghost: the case token beyond A + D + the fee held + unpaid
        address credited; // the judge a Closed credited
        uint256 judgeCredited;
        uint256 judgeOut; // what left the seat ledger for that judge
        bool badInts; // nonce, expiry or openBy past 2^53 - 1: guard 16 refuses every live init
        bool unit; // priced in a reference unit through two feeds
        bool branchRan; // the expiry branch ran: buyerPayout and the token are fixed
        uint8 noSeatClose; // a guild case closed with no seat: 1 the seller's refund, 2 a settlement, 3 form 3
        uint256 capInToken; // the model's cap in the token, for a unit-priced order
        uint8 fundKind; // round 6: 0 the seller's key, 1 a funder, 2 the seller's payout, 3 the buyer's payout, 4 unbound; round 7: 5 a pool's fund
        uint256 toFundG; // round 7: the model's deposit return to sellerFund at the terminal
        bool payNow; // the named judge registers its payout on the instance right after the opening (s8, venue/1 s3)
    }

    /// An order in a token that has no code or answers nothing: it never opens; the expiry branch and
    /// cancel must still run, credit nothing and leave the address's other tokens to sweep.
    struct G {
        Eravon inst;
        Eravon.OrderTerms t;
        bytes quote;
        uint8 kind; // 0 no code, 1 a mute token
        bool cancelled;
        bool branchRan;
    }

    /// A pricing order of the round-3 feed campaign: its own settlement token, its feeds' kind, whether it opened.
    struct F {
        Eravon inst;
        uint8 kind;
        bool done;
    }

    struct S {
        uint8 ph;
        uint256 seats;
        uint256 premium;
        uint256 deposit;
        uint256 lfHeld;
        uint256 unpaid;
        uint64 claimedAt;
        uint64 disputedAt;
        bool pending;
        uint64 redrawDeadline;
        uint64 drawBlock;
        uint256 panelSize;
    }

    struct Snap {
        uint256 bal;
        uint256 unpaid;
        uint8 ph;
        uint8 lastState;
        uint64 lastAccepted;
        uint64 lastWindowEnd;
        uint256 lfHeld;
        uint256 stray;
        uint256 judgeClaim;
        address lastJudge;
        uint256 seats;
        bool pending;
        uint64 deadline;
        uint64 drawBlock;
        uint256 panelSize;
        uint256[3] owedP; // owed to the buyer's payout, the seller's payout, sellerFund
        uint256[3] balP; // their token balances
        uint256 owedK; // round 6: owed to the seller's key, and its balance
        uint256 balK;
        uint256 allowF; // sellerFund's allowance to the logic
        bool fundBound; // the key is the fund, or the ghost holds the bond, before the action
        // round 7: a credited judge that is also the case's sellerFund: its two ledgers and the two addresses they pay
        bool dual;
        address dPay;
        uint256 dClaim;
        uint256 dOwed;
        uint256 dBalJ;
        uint256 dBalPay;
        // the key that acts for the last seat's judge before the action (s9, venue/1 s1)
        address lastJudgeKey;
        // the outcome view (outcomeOf before a ruling, advanceOf before an advance) read on the pre-state (s8)
        bool vRead;
        bool vInFlight;
        uint8 vState;
        uint256 vShare0;
        uint256 vShare1;
        uint256 vStandby;
        uint256 vRuling;
        uint256 vFrom0;
        uint256 vFrom1;
    }

    struct Ev {
        uint256 anchors;
        uint256 pulls;
        bool closed;
        uint8 term;
        uint256 b;
        uint256 s;
        uint256 j;
        uint256 bond;
        bool refunded;
        uint8 cause;
        uint16 bps;
        uint64 rung;
        bool lapsed;
        uint16 lapseBps;
        uint256 seated;
        address seatedJudge;
        bytes32 seatedCase;
        uint64 seatedOrdinal;
        uint256 instEvents;
        uint256 armed;
        uint64 armedBlock;
        uint64 armedDeadline;
        bool form3;
    }

    /// What an order is built from, beside its terms.
    struct R {
        uint8 track;
        uint8 sk;
        uint8 bk;
        uint256 flags;
        uint256 surplus;
        uint256 d;
        uint256 lf;
        uint256 p;
        uint8 lapse;
        uint32 offerWindow;
        uint32 rulingWindow;
        bool openNow;
        bool unit;
        uint256 capInToken;
        uint8 fundKind;
        bool bindNow;
        bool payNow;
    }

    C[] internal cases;
    address[3] internal namedJudges;
    address[3] internal members;
    address[2] internal g2;
    address[] internal panel2;
    address[2] internal g3;
    mapping(address => address) public payoutOfJudge;
    address[] internal judgePayouts;
    address public coSeller;
    address public coBuyer;
    Impostor internal impostor;
    MockERC20 public ftok;
    BlockableToken public btok;
    uint8 internal guildPanel;
    mapping(address => mapping(bytes32 => bool)) internal revokedTerms;
    Snap internal snap;
    uint256 public totalMinted;
    uint256 public totalMintedForeign;
    mapping(bytes32 => uint256) public reached; // which paths the campaign walked

    // violation counters
    uint256 public anchoredOutside;
    uint256 public anchorWrong;
    uint256 public eventOnWrongEmitter;
    uint256 public closedTwice;
    uint256 public phaseWentBack;
    uint256 public closedSumWrong;
    uint256 public closedModelWrong;
    uint256 public cancelAfterInit;
    uint256 public causeWrong;
    uint256 public pullNotToCaller;
    uint256 public pullCountWrong;
    uint256 public impostorPullPassed;
    uint256 public seatMachineBroken;
    uint256 public seatTableBroken;
    uint256 public drawWrong;
    uint256 public judgeCreditWrong;
    uint256 public secondInitPassed;
    uint256 public raceBroken;
    uint256 public revokedOpened;
    uint256 public revokeTouchedOpen;
    uint256 public expiryBranchWrong;
    uint256 public refusedRedrawPassed;
    uint256 public livenessBroken; // an action the standard owes success reverted
    uint8 public lastLivenessAct;
    bytes public lastLivenessErr;
    uint256 public seatedWrong;
    uint256 public eventWrong;
    // round 2
    uint256 public zeroSeatWrong; // a seatless case outside an armed draw or the three closes without a seat
    uint256 public seatedNotAfterDraw; // a drawn Seated in or before the armed block
    uint256 public pOutsideTable; // the judge's credit outside s8's P and fee rows
    uint256 public terminalWrong; // a terminal, its cause or form 3 other than the pre-state names
    uint256 public notForeignWrong; // NotForeign where the settlement balance would not fall, or none where it would
    uint256 public unnamedRevert; // an instance call answered with a panic, a bare revert, or a string
    uint8 public lastUnnamedAct;
    bytes public lastUnnamed;
    uint256 public ghostWrong; // an order in a token without code or answers misbehaved
    uint256 public burnerPaid; // a payee whose hook burns gas received tokens or lost its credit
    uint256 public pricedWrong; // a unit-priced case pinned other than the model's fee
    uint256 public int53Wrong; // an order past 2^53 - 1 opened, or was refused other than BadTerms(16)
    uint256 public blobUnnamed; // a malformed signature through the instance answered other than by name
    // round 4
    uint256 public refundInT3Wrong; // a refund in a dispute at or before T3's end landed, moved something, or answered otherwise
    uint256 public refundPastT3Wrong; // past T3: the direct refund not the plain one, or the signed one not WrongPhase
    uint256 public refundedJudgeWrong; // a Refunded crediting the judge outside s8's one row (P, Claimed, seated)
    uint256 public lenOutcomeWrong; // an answer's length changed a decision or an outcome
    uint256 public pushLenGasWrong; // a push answer's length changed the caller's own gas
    int256 public lastLenGasOwn;
    uint256 public staleSigWrong; // a Refund() signature answered other than its phase and its signer say
    uint256 public pullShapeWrong; // a pull's answer other than no data or one word 1 was taken, or refused otherwise
    uint256 public balanceCopyMax; // statistic: the most a balance answer's length cost a caller beyond its pushes
    uint256 public pushesMetered; // statistic: the instance pushes the meter read
    InvNoCopy internal noCopy;
    uint64 internal advSeat; // the seat argument of the advance observed
    SecondEntry public second;
    MuteToken internal mute;
    MockUSD internal usd;
    InvShapedFeed internal feedTok;
    InvShapedFeed internal feedUsd;
    uint64 internal feedsAt;
    G[] internal ghosts;
    // round 3
    uint256 public drawOutcomeWrong; // a redraw or an advance on an armed draw outside s7's two clocks
    uint256 public staleRearmed; // a lost seed that left the case open or armed a new draw
    uint256 public drawBlockMoved; // an armed draw whose block changed while it stayed armed
    uint256 public firstBlockRefused; // the block after the arming did not draw
    uint256 public settleVsClockWrong; // a settlement's P other than the clock's at that moment, or the in-window rule
    uint256 public disputeRefundWrong; // a refund that landed during a dispute within T3, or signed during one
    uint256 public refundSigWrong; // refundBySig answered other than s9 and s10 in some phase
    uint256 public pushOverAllowance; // a push of the standard's own handed more than PUSH_GAS
    uint256 public lastPushGas;
    uint8 public lastPushAct;
    uint256 public feedInitUnnamed; // a pricing order's init that reverted without a name under the cap
    uint256 public feedInitWrong; // a pricing order's init whose verdict is not the model's
    bytes public lastFeedErr;
    // what the last observed Closed credited the judge, for the settlement against the clock
    bool internal lastClosedSeen;
    uint256 internal lastClosedJ;
    // the refund under observation is the signed form
    bool internal refundSigned;
    // no keeper draws for the order being opened (the draw's clocks take it from its arming block)
    bool internal noKeeper;
    MockERC20 public ptok; // the pricing orders' settlement token, apart from the campaign's ledger
    F[] internal fcases;
    // round 5
    uint256 public pairStepWrong; // a step of a rolled-back pair or reorganised history broke a model (carried)
    uint256 public pairOrderWrong; // two acts in one block whose totals differ where the text makes the order immaterial
    uint256 public pairAfterTerminal; // an act after a terminal changed a party's total, or a refund landed behind a dispute
    uint256 public rearmWrong; // a draw armed or a seat opened after an accepted seat, past three seats, or in a fourth seat
    uint256 public closedLandingWrong; // a Closed whose amounts did not land where it says, or left money unrecorded
    uint256 public pCreditWrong; // P (or the fee) credited outside the four rows of s8
    uint256 public foreignLedgerWrong; // a sweep of a foreign token moved the settlement ledger or paid otherwise
    uint256 public oddDecWrong; // an odd-decimals pricing order whose verdict or pinned values are not the model's
    uint256 public flipWrong; // a flipping feed or token whose verdict or pinned values no combination of reads gives
    uint256 public delegatedWrong; // an EIP-7702 delegated account in some role answered outside the text
    uint256 public griefMoved; // a stranger's no-gain calls moved a party's total, the phase or a seat
    bytes public lastR5; // the bytes of the last round-5 violation, for the report
    uint256 public lastLivenessCase; // the case of the last liveness violation, for the report
    uint256 public maxGuildSeats; // statistic: the most seats a guild case opened
    uint256 public pairCovered; // statistic: bit k set where unordered pair k ran with at least one act landing
    /// statistic, by the pre-state (phase * 16 + the last seat's state * 2 + an armed draw): bit k set where pair k's two
    /// orders end in different totals or phases and the text makes the order matter (whichever lands first decides)
    mapping(uint256 => uint256) public orderDep;
    /// the same where the judge's total differs between the two orders
    mapping(uint256 => uint256) public orderDepJudge;
    mapping(uint256 => uint8) public creditRowOf; // the s8 row that credited the case's judge (1 refund, 2 settle, 3 release, 4 ruling)
    mapping(address => bool) public delegatedExec; // a party key that runs code (as a delegation would) and acts through it
    uint256 internal execToggle;
    uint8 internal forceTrack; // 1: the order is on the guild of judges' track (the three-seat walks)
    bool internal forceManual; // the order's quote clears autoAccept and keeps a full offer window
    R5Hostile[] internal hostiles;
    uint256 internal oddCount;
    uint256 internal threeCount; // the three-seat walks' and the pairs' fresh cases, beside MAX_CASES
    bool internal forceOpen; // the order opens at once (the pairs' fresh cases)
    uint256 internal constant MAX_THREE = 16;
    // round 6: the funding bond and the deposit address (s4 step 5, s7a, s8)
    address internal constant FUNDER = address(uint160(0x007F00000000000000000000000000000000000F01));
    address internal constant UNBOUND = address(uint160(0x007F00000000000000000000000000000000000F02));
    address public cfunder; // a contract funder: it binds and approves by its own calls
    mapping(address => mapping(address => bool)) public boundG; // the ghost of the logic's funds[seller][fund]
    uint256 public pullFromUnbound; // a Pulled naming a fund other than the order's, or neither the key nor bound before the pull
    uint256 public fundGateWrong; // an init refused other than the fund model says (FundUnbound, TransferFailed), or that moved money
    uint256 public fundPullWrong; // the opening pull did not take exactly D from sellerFund's balance and allowance, or took from the key
    uint256 public fundReturnWrong; // a terminal whose deposit return did not land at sellerFund, or that paid the key where it is not the fund
    uint256 public bondWrong; // a FundBound, a funds() answer or a refusal of acceptFunding other than the bond's rule
    uint256 public allowanceMovedOpen; // a fund's allowance change moved an open case
    /// Fresh addresses that bind themselves in a race against an init naming them (each binds at most once per seller).
    address internal constant RACER = address(uint160(0x007F00000000000000000000000000000000000F10));
    uint256 internal racers;
    address internal forceFund; // the next order's sellerFund, unbound (the race's order)
    // round 7: the pool (s7a): several orders of one seller on one fund, whose allowance and balance the fund moves
    // between their inits and terminals; the fund a fresh key, a named judge or a guild member
    address internal constant POOL = address(uint160(0x007F00000000000000000000000000000000000F40));
    /// A fresh address for the gate-order probes' twins, funded and bound only inside their snapshots.
    address internal constant UNBOUND2 = address(uint160(0x007F00000000000000000000000000000000000F50));
    uint256 internal constant MAX_POOLS = 6;
    struct PoolRec {
        address f;
        address s;
        uint256 first;
        uint256 n;
        uint8 kind; // 0 a fresh key, 1 a named judge, 2 a guild member
        bool late; // the bond lands only after a first init was refused
    }
    PoolRec[] internal poolList;
    uint256 internal poolKeys;
    uint256 public poolCases; // the pools' orders, beside MAX_CASES
    address internal forceSeller; // the next order's seller key
    address internal forceJudge; // the next named order's judge
    bool internal forceShut; // the next order is registered unopened
    mapping(address => bool) public isPoolFund;
    address[] internal poolFunds;
    /// The ghost of a pool fund's allowance to the logic: what its own approves set, less each D an opening pulled.
    mapping(address => uint256) public allowG;
    /// The ghost of a pool fund's balance plus its credits at every instance: what was minted to it, less each D pulled,
    /// plus the deposit's return the table's model gives it at each terminal, less what it sent away.
    mapping(address => int256) public balG;
    mapping(uint256 => uint8) internal poolShort; // why a pool order was refused last: 1 the allowance, 2 the balance, 3 the bond
    bool internal poolRetry; // a pool init retried after a top-up, once
    uint256 public poolWrong; // a pool step off the ghost: a verdict, a pull, a refill, or another open case of the pool moved
    uint256 public dualWrong; // a judge that funds its case: one ledger paid into the other, or to the other's address
    uint256 public fundOrderWrong; // init's order of decision around the bond off s4: caller, consents, bond, balance, deposit
    // the succession book (s9, venue/1 s1), the stranger's submissions (s9, s10) and the payouts (s8, venue/1 s3)
    uint256 public strangerAdmitted; // a stranger's signed form, settlement, sweep, init or signed cancel not refused NotParty
    uint256 public oldKeyAdmitted; // a key that handed its record on acted, signed or registered for something, or a new key was refused
    uint256 public payoutAtInit; // a payout registered on an instance by its opening
    uint256 public payoutRuleWrong; // setPayout or withdraw answered other than venue/1 s3 and s8 say
    uint256 public seatCreditWrong; // a seat's standbyCredited or rulingCredited other than what the terminal credited it
    uint256 public viewWrong; // outcomeOf or advanceOf other than what rule or advance did on the state it read
    uint256 public panelFreezeWrong; // a record made in a block reached a freeze in that block
    /// The private key of the current key of SELLER's and BUYER's records (they move on the succession book).
    uint256 public sellerPk;
    uint256 public buyerPk;
    /// Moves made per role (0 the seller, 1 the buyer, 2 a judge, 3 a guild), and every key that handed a record on.
    uint256[4] public moves;
    uint256 internal moveCount;
    address[] internal oldKeys;
    mapping(address => address) public rootOfOld; // a former key's record (its root)
    mapping(address => uint256) internal pkOfOld; // a former party key's private key, to sign as it after the move
    uint256 internal constant SECP_N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141;

    constructor() {
        setUpBase();
        btok = new BlockableToken();
        tok = MockERC20(address(btok));
        ftok = new MockERC20();
        E = sha256(ENTRY);
        for (uint256 i = 0; i < 3; i++) {
            namedJudges[i] = address(uint160(0x001000000000000000000000000000000000000A01 + i));
            members[i] = address(uint160(0x001000000000000000000000000000000000000B01 + i));
            payoutOfJudge[namedJudges[i]] = address(uint160(0x007000000000000000000000000000000000000A00 + i));
            payoutOfJudge[members[i]] = address(uint160(0x007000000000000000000000000000000000000B00 + i));
        }
        g2[0] = address(uint160(0x001000000000000000000000000000000000000C01));
        g2[1] = address(uint160(0x001000000000000000000000000000000000000C02));
        payoutOfJudge[g2[0]] = address(uint160(0x007000000000000000000000000000000000000C00));
        payoutOfJudge[g2[1]] = address(uint160(0x007000000000000000000000000000000000000C01));
        for (uint256 i = 0; i < 3; i++) {
            judgePayouts.push(payoutOfJudge[namedJudges[i]]);
            judgePayouts.push(payoutOfJudge[members[i]]);
        }
        judgePayouts.push(payoutOfJudge[g2[0]]);
        judgePayouts.push(payoutOfJudge[g2[1]]);
        g3[0] = address(uint160(0x001000000000000000000000000000000000000D01));
        g3[1] = address(uint160(0x001000000000000000000000000000000000000D02));
        payoutOfJudge[g3[0]] = address(uint160(0x007000000000000000000000000000000000000D00));
        payoutOfJudge[g3[1]] = address(uint160(0x007000000000000000000000000000000000000D01));
        judgePayouts.push(payoutOfJudge[g3[0]]);
        judgePayouts.push(payoutOfJudge[g3[1]]);
        second = new SecondEntry(btok);
        btok.setSecond(address(second));
        mute = new MuteToken();
        usd = new MockUSD();
        feedsAt = uint64(vm.getBlockTimestamp());
        feedTok = new InvShapedFeed(8, 2000e8, feedsAt); // the settlement token at 2000.0 units
        feedUsd = new InvShapedFeed(8, 1e8, feedsAt); // the unit at 1.0
        // round 4: the token shapes its answers to the standard's own callers and to the no-copy reader
        btok.setStandard(logic.PROXY_CODEHASH(), address(logic), address(verifier));
        noCopy = new InvNoCopy();
        btok.mark(address(noCopy), true);
        coSeller = address(new Company());
        coBuyer = address(new Company());
        cfunder = address(new Company());
        impostor = new Impostor();
        ptok = new MockERC20();
        sellerPk = SELLER_PK;
        buyerPk = BUYER_PK;
        // the second guild's panel: both parties among its members, ascending
        address[4] memory p2 = [SELLER, BUYER, g2[0], g2[1]];
        for (uint256 i = 0; i < 4; i++) {
            for (uint256 k = i + 1; k < 4; k++) {
                if (p2[k] < p2[i]) (p2[i], p2[k]) = (p2[k], p2[i]);
            }
        }
        for (uint256 i = 0; i < 4; i++) {
            panel2.push(p2[i]);
        }
        // the chain's one panel book (s7): each guild files its record under its root, the key the book's own
        // proxyCodeHash, no router; a record serves from the block after its own
        bytes32 T = logic.PROXY_CODEHASH();
        uint64 exp = uint64(vm.getBlockTimestamp() + 3650 days);
        (address[] memory m0, bytes32[] memory w0) = _panel(1, 0);
        vm.prank(GUILD, GUILD);
        pbook.engage(T, m0, w0, ROSTER, exp, address(0));
        (address[] memory m2, bytes32[] memory w2) = _panel(2, 0);
        vm.prank(GUILD2, GUILD2);
        pbook.engage(T, m2, w2, ROSTER, exp, address(0));
        (address[] memory m3, bytes32[] memory w3) = _panel(3, 0);
        vm.prank(GUILD3, GUILD3);
        pbook.engage(T, m3, w3, ROSTER, exp, address(0));
        // the first block of the campaign, so that a draw armed at once has a block below it and the records serve
        vm.roll(vm.getBlockNumber() + 1);
    }

    // ------------------------------------------------------------------
    // views for the invariants
    // ------------------------------------------------------------------

    function caseCount() external view returns (uint256) {
        return cases.length;
    }

    function caseAt(uint256 i)
        external
        view
        returns (address inst, bytes32 id, bool guildTrack, uint256 a, uint256 d, uint256 lf, uint256 p, uint8 lastPhase, bool closedSeen, uint256 pulled)
    {
        C storage c = cases[i];
        return (address(c.inst), c.id, c.track != 0, c.a, c.d, c.lf, c.p, c.lastPhase, c.closedSeen, c.pulled);
    }

    function caseParties(uint256 i) external view returns (address bk, address sk, address bp, address sp) {
        C storage c = cases[i];
        return (c.t.buyerKey, c.t.offer.sellerKey, c.t.buyerPayout, c.t.offer.sellerPayout);
    }

    function caseGhost(uint256 i)
        external
        view
        returns (uint8 track, uint256 stray, address credited, uint256 judgeCredited, uint256 judgeOut, uint256 rungs)
    {
        C storage c = cases[i];
        return (c.track, c.stray, c.credited, c.judgeCredited, c.judgeOut, c.t.offer.ladder.length);
    }

    function caseOpened(uint256 i) external view returns (bool) {
        return cases[i].opened;
    }

    /// Round 6: the order's sellerFund and the draw that gave it.
    function caseFund(uint256 i) external view returns (address fund, uint8 kind) {
        return (cases[i].t.offer.sellerFund, cases[i].fundKind);
    }

    /// Every violation counter of the campaign, summed (the reach walk asserts it zero).
    function violations() external view returns (uint256) {
        return _viol();
    }

    function _viol() internal view returns (uint256 v) {
        v = anchoredOutside + anchorWrong + eventOnWrongEmitter + closedTwice + phaseWentBack + closedSumWrong + closedModelWrong
            + cancelAfterInit + causeWrong + pullNotToCaller + pullCountWrong + impostorPullPassed + seatMachineBroken;
        v += seatTableBroken + drawWrong + judgeCreditWrong + secondInitPassed + raceBroken + revokedOpened + revokeTouchedOpen
            + expiryBranchWrong + refusedRedrawPassed + livenessBroken + seatedWrong + eventWrong;
        v += zeroSeatWrong + seatedNotAfterDraw + pOutsideTable + terminalWrong + notForeignWrong + unnamedRevert
            + ghostWrong + burnerPaid + pricedWrong + int53Wrong + blobUnnamed;
        v += drawOutcomeWrong + staleRearmed + drawBlockMoved + firstBlockRefused + settleVsClockWrong + disputeRefundWrong
            + refundSigWrong + pushOverAllowance + feedInitUnnamed + feedInitWrong;
        v += refundInT3Wrong + refundPastT3Wrong + refundedJudgeWrong + lenOutcomeWrong + pushLenGasWrong + staleSigWrong
            + pullShapeWrong;
        v += pairStepWrong + pairOrderWrong + pairAfterTerminal + rearmWrong + closedLandingWrong + pCreditWrong
            + foreignLedgerWrong + oddDecWrong + flipWrong + delegatedWrong + griefMoved;
        v += pullFromUnbound + fundGateWrong + fundPullWrong + fundReturnWrong + bondWrong + allowanceMovedOpen;
        v += poolWrong + dualWrong + fundOrderWrong;
        v += strangerAdmitted + oldKeyAdmitted + payoutAtInit + payoutRuleWrong + seatCreditWrong + viewWrong + panelFreezeWrong;
        v += routerWrong;
    }

    function debugCase(uint256 i) external view returns (uint8, uint8, uint8, uint8, bool) {
        return (cases[i].track, cases[i].panel, guildPanel, _ph(i), forceTrack != 0);
    }

    function noSeatCloseOf(uint256 i) external view returns (uint8) {
        return cases[i].noSeatClose;
    }

    /// The members of the panel the case froze, in panel order.
    function frozenMembers(uint256 i) external view returns (address[] memory m) {
        (m,) = _frozen(cases[i]);
    }

    function allJudges() external view returns (address[] memory j) {
        j = new address[](10);
        for (uint256 i = 0; i < 3; i++) {
            j[i] = namedJudges[i];
            j[3 + i] = members[i];
        }
        j[6] = g2[0];
        j[7] = g2[1];
        j[8] = g3[0];
        j[9] = g3[1];
    }

    function allPayouts() external view returns (address[] memory) {
        return judgePayouts;
    }

    /// Every address the campaign can move a token to, beside the instances and the judges' payouts.
    function holders() external view returns (address[] memory a) {
        a = new address[](17 + MAX_POOLS);
        for (uint256 k = 0; k < MAX_POOLS; k++) {
            a[17 + k] = address(uint160(POOL) + uint160(k));
        }
        for (uint256 k = 0; k < 4; k++) {
            a[13 + k] = address(uint160(RACER) + uint160(k));
        }
        a[10] = FUNDER;
        a[11] = UNBOUND;
        a[12] = cfunder;
        a[0] = BUYER;
        a[1] = SELLER;
        a[2] = BUYER_PAYOUT;
        a[3] = SELLER_PAYOUT;
        a[4] = coSeller;
        a[5] = coBuyer;
        a[6] = address(verifier);
        a[7] = STRANGER;
        a[8] = address(impostor);
        a[9] = address(logic);
    }

    /// Every key a record was handed on from, in the order the campaign moved them.
    function oldKeyList() external view returns (address[] memory) {
        return oldKeys;
    }

    function successionAddr() external pure returns (address) {
        return SUCCESSION;
    }

    function panelBookAddr() external view returns (address) {
        return address(pbook);
    }

    function token() external view returns (address) {
        return address(tok);
    }

    function foreignToken() external view returns (address) {
        return address(ftok);
    }

    function buyer() external view returns (address) {
        return BUYER;
    }

    function seller() external view returns (address) {
        return SELLER;
    }

    function buyerPayout() external pure returns (address) {
        return BUYER_PAYOUT;
    }

    function sellerPayout() external pure returns (address) {
        return SELLER_PAYOUT;
    }

    function logicAddr() external view returns (address) {
        return address(logic);
    }

    // ------------------------------------------------------------------
    // helpers
    // ------------------------------------------------------------------

    function _h(uint256 x) internal pure returns (uint256) {
        return uint256(keccak256(abi.encode(x, "eravon/fuzz")));
    }

    function _mark(string memory what) internal {
        reached[keccak256(bytes(what))]++;
    }

    function _panel(uint8 track, uint8 rec) internal view returns (address[] memory m, bytes32[] memory w) {
        if (track == 3) {
            m = new address[](5);
            for (uint256 k = 0; k < 3; k++) {
                m[k] = members[k];
            }
            m[3] = g3[0];
            m[4] = g3[1];
        } else if (track == 2) {
            m = new address[](4);
            for (uint256 k = 0; k < 4; k++) {
                m[k] = panel2[k];
            }
        } else if (rec == 0) {
            m = new address[](3);
            for (uint256 k = 0; k < 3; k++) {
                m[k] = members[k];
            }
        } else {
            m = new address[](2);
            m[0] = members[1];
            m[1] = members[2];
        }
        w = new bytes32[](m.length);
        for (uint256 k = 0; k < m.length; k++) {
            w[k] = AMANAH;
        }
    }

    /// The panel the case froze at init (the guild of judges' record of that day, or the second guild's).
    function _frozen(C storage c) internal view returns (address[] memory, bytes32[] memory) {
        return _panel(c.track == 0 ? 1 : c.track, c.track == 1 ? c.panel : 0);
    }

    function _ladder(uint8 kind) internal pure returns (Eravon.Rung[] memory l) {
        if (kind == 0) return threeRungs();
        if (kind == 1) {
            l = new Eravon.Rung[](2);
            l[0] = Eravon.Rung(0, 10000, 1);
            l[1] = Eravon.Rung(10000, 0, 0);
            return l;
        }
        if (kind == 3) {
            // a fee-shifting middle rung, the last rung's feeBearer 0 (ADV2-2's ladder)
            l = new Eravon.Rung[](3);
            l[0] = Eravon.Rung(0, 10000, 0);
            l[1] = Eravon.Rung(9900, 100, 1);
            l[2] = Eravon.Rung(10000, 0, 0);
            return l;
        }
        if (kind == 4) {
            // the last rung's feeBearer 2
            l = new Eravon.Rung[](4);
            l[0] = Eravon.Rung(0, 10000, 1);
            l[1] = Eravon.Rung(3000, 7000, 0);
            l[2] = Eravon.Rung(7000, 3000, 1);
            l[3] = Eravon.Rung(10000, 0, 2);
            return l;
        }
        l = new Eravon.Rung[](5);
        for (uint256 k = 0; k < 5; k++) {
            uint16 b = uint16(k * 2500);
            l[k] = Eravon.Rung(b, 10000 - b, uint8(k % 3));
        }
    }

    function _mint(address to, uint256 v) internal {
        tok.mint(to, v);
        totalMinted += v;
    }

    // ---- the succession book (s9, venue/1 s1, galeed.roster/1 s4) ----

    /// The key that acts now for the record `a` names: the succession book's answer.
    function _cur(address a) internal view returns (address) {
        return sbook.current(a);
    }

    /// The private key of the current key of a key party's record (SELLER's or BUYER's); zero for any other.
    function _pkOf(address party) internal view returns (uint256) {
        if (party == SELLER) return sellerPk;
        if (party == BUYER) return buyerPk;
        return 0;
    }

    /// A signature by the current key of the seller's or the buyer's record.
    function _signS(bytes32 d) internal view returns (bytes memory) {
        return sign(sellerPk, d);
    }

    function _signB(bytes32 d) internal view returns (bytes memory) {
        return sign(buyerPk, d);
    }

    /// The two consents at init as signatures by the parties' current keys (terms/1 s5.1, s5.2).
    function _sellerSig(Eravon.OrderTerms memory t) internal view returns (bytes memory) {
        return sign(sellerPk, digestOf(address(logic), offerHash(t)));
    }

    function _buyerSig(Eravon.OrderTerms memory t) internal view returns (bytes memory) {
        return sign(buyerPk, digestOf(instanceOf(t), orderHash(t)));
    }

    /// A party's call: a company executes it, a key's current key sends it; a revert comes back with its bytes.
    function _do(address who, address target, bytes memory data) internal {
        (bool ok, bytes memory ret) = _try(who, target, data);
        if (!ok) {
            assembly ("memory-safe") {
                revert(add(ret, 32), mload(ret))
            }
        }
    }

    /// A call from the address itself whatever key now acts for its record (an approve on the token, a funder's
    /// bond: the money and the bond are the address's own, s7a); a company executes it.
    function _doAs(address who, address target, bytes memory data) internal {
        bool ok;
        bytes memory ret;
        if (who == coSeller || who == coBuyer || who == cfunder || _viaCode(who)) {
            try Company(who).exec(target, data) returns (bytes memory r) {
                (ok, ret) = (true, r);
            } catch (bytes memory err) {
                (ok, ret) = (false, err);
            }
        } else {
            vm.prank(who);
            (ok, ret) = target.call(data);
        }
        if (!ok) {
            assembly ("memory-safe") {
                revert(add(ret, 32), mload(ret))
            }
        }
    }

    /// A party's call under the transaction gas cap: a company executes it, a key sends it.
    /// Round 5: a party key that runs code (an EIP-7702 delegation's) acts through it one time in two.
    function _viaCode(address who) internal returns (bool) {
        return delegatedExec[who] && who.code.length != 0 && (++execToggle) % 2 == 0;
    }

    /// The act of the record `who` names, sent by its current key (s9): a company executes it, a key sends it.
    function _try(address who, address target, bytes memory data) internal returns (bool ok, bytes memory ret) {
        address k = _cur(who);
        if (k != who) _mark("move.actedByTheNewKey");
        if (k == coSeller || k == coBuyer || k == cfunder || _viaCode(k)) {
            try Company(k).exec{gas: TX_GAS}(target, data) returns (bytes memory r) {
                return (true, r);
            } catch (bytes memory err) {
                return (false, err);
            }
        }
        vm.prank(k);
        (ok, ret) = target.call{gas: TX_GAS}(data);
    }

    /// The same call from the address itself, no key looked up (a former key, a stranger).
    function _tryAs(address who, address target, bytes memory data) internal returns (bool ok, bytes memory ret) {
        vm.prank(who, who);
        (ok, ret) = target.call{gas: TX_GAS}(data);
    }

    /// s9, s10: a stranger's submission the text gives the parties alone (a signed form, settle, sweep, init
    /// under buyerMayOpen, the expiry branch, cancel's signed form) is refused NotParty, before any other fault.
    function _strangerRefused(address target, bytes memory data, string memory what) internal {
        (bool ok, bytes memory ret) = _tryAs(STRANGER, target, data);
        if (ok || ret.length != 4 || bytes4(ret) != Eravon.NotParty.selector) {
            strangerAdmitted++;
            _r5(abi.encode("stranger", what, ok, ret));
        }
        _mark(string(abi.encodePacked("stranger.", what)));
    }

    /// The party that submits a signed form or a settlement for the case (either party's current key, s9).
    function _submitter(C storage c, uint256 h) internal view returns (address) {
        return h % 2 == 0 ? c.t.offer.sellerKey : c.t.buyerKey;
    }

    /// One of the standard's own names (s10, the gate's panel faults included): every revert of an instance is one,
    /// never a bare revert, a panic, a token's reason or any other bytes.
    function _isNamed(bytes memory ret) internal pure returns (bool) {
        if (ret.length == 36) return bytes4(ret) == Eravon.BadTerms.selector;
        if (ret.length != 4) return false;
        bytes4 x = bytes4(ret);
        bytes4[45] memory names = [
            Eravon.Reentered.selector, Eravon.AlreadyInitialized.selector, Eravon.WrongAddress.selector,
            Eravon.BadQuote.selector, Eravon.QuoteMismatch.selector, Eravon.BadSellerSig.selector,
            Eravon.BadBuyerSig.selector, Eravon.Underfunded.selector, Eravon.OverCap.selector,
            Eravon.RateStale.selector, Eravon.DepositShort.selector, Eravon.PanelBookMissing.selector,
            Eravon.VerifierMissing.selector, Eravon.PanelMismatch.selector, Eravon.NoPanel.selector,
            Eravon.PanelWord.selector, Eravon.NoMembers.selector, Eravon.NotPendingRedraw.selector,
            Eravon.WrongPhase.selector, Eravon.NotDirect.selector, Eravon.UnknownSeat.selector,
            Eravon.NotOpen.selector, Eravon.WindowClosed.selector, Eravon.NotSeller.selector,
            Eravon.NotBuyer.selector, Eravon.NotInstance.selector, Eravon.ZeroManifest.selector,
            Eravon.RungOutOfRange.selector, Eravon.ExtraNotEmpty.selector, Eravon.EntryMismatch.selector,
            Eravon.SettleExpired.selector, Eravon.BpsOver.selector, Eravon.PayoutSet.selector,
            Eravon.ZeroPayout.selector, Eravon.NoPayout.selector, Eravon.TransferFailed.selector,
            Eravon.NoPanelFace.selector, Eravon.NotRegistry.selector, Eravon.DrawPending.selector,
            Eravon.FeeOverflow.selector, Eravon.NotForeign.selector, Eravon.FundUnbound.selector,
            Eravon.NotParty.selector, Eravon.NotCurrent.selector, Eravon.RouterGas.selector
        ];
        for (uint256 k = 0; k < 45; k++) {
            if (x == names[k]) return true;
        }
        return false;
    }

    /// s8: every push of the standard's own (a terminal's, cancel's, the expiry branch's) is handed at most
    /// PUSH_GAS; sweep and withdraw hand the token what their caller brings.
    function _meterPush(uint8 act) internal {
        pushesMetered += btok.pushes();
        uint256 g = btok.maxPushGas();
        if (g == 0 || act == A_SWEEP || act == A_MONEY) return;
        if (g > PUSH_GAS) {
            pushOverAllowance++;
            lastPushGas = g;
            lastPushAct = act;
        }
        _mark("push.metered");
    }

    function _approve(address owner, address spender, uint256 v) internal {
        _doAs(owner, address(tok), abi.encodeCall(MockERC20.approve, (spender, v)));
    }

    function _st(Eravon inst) internal view returns (S memory s) {
        Eravon.CaseStatus memory x = inst.status();
        s.ph = uint8(x.phase);
        s.seats = x.seats;
        s.premium = x.premium;
        s.deposit = x.deposit;
        s.lfHeld = x.lfHeld;
        s.unpaid = x.unpaid;
        s.claimedAt = x.claimedAt;
        s.disputedAt = x.disputedAt;
        s.pending = x.pendingRedraw;
        s.redrawDeadline = x.redrawDeadline;
        s.drawBlock = x.drawBlock;
        s.panelSize = x.panelSize;
    }

    function _ph(uint256 i) internal view returns (uint8) {
        return uint8(phaseOf(cases[i].inst));
    }

    function _seatCount(uint256 i) internal view returns (uint256) {
        return _st(cases[i].inst).seats;
    }

    function _lastSeat(uint256 i) internal view returns (IGaleedVenue.Seat memory) {
        C storage c = cases[i];
        return c.inst.seatOf(c.id, uint64(_seatCount(i) - 1));
    }

    function _lastJudgeOr(uint256 i, address alt) internal view returns (address) {
        if (_ph(i) == 0 || _seatCount(i) == 0) return alt;
        return _lastSeat(i).judge;
    }

    function _now() internal view returns (uint256) {
        return vm.getBlockTimestamp();
    }

    /// A case the predicate admits, searched from the seed; one pick in eight
    /// is any case, so that the refusals stay exercised.
    function _pickWhere(uint256 seed, function(uint256) internal view returns (bool) pred)
        internal
        view
        returns (bool, uint256)
    {
        uint256 n = cases.length;
        if (n == 0) return (false, 0);
        uint256 hs = _h(seed);
        uint256 start = hs % n;
        if ((hs >> 128) % 8 == 0) return (true, start);
        for (uint256 k = 0; k < n; k++) {
            uint256 j = (start + k) % n;
            if (pred(j)) return (true, j);
        }
        return (false, 0);
    }

    function _pickAny(uint256 seed) internal view returns (bool, uint256) {
        if (cases.length == 0) return (false, 0);
        return (true, _h(seed) % cases.length);
    }

    // ---- predicates ----

    function _isNone(uint256 i) internal view returns (bool) {
        return _ph(i) == 0;
    }

    function _isSet(uint256 i) internal view returns (bool) {
        return _ph(i) != 0;
    }

    function _isOpen(uint256 i) internal view returns (bool) {
        uint8 p = _ph(i);
        return p >= 1 && p <= 3;
    }

    function _canAccept(uint256 i) internal view returns (bool) {
        if (_ph(i) != 1 || _seatCount(i) == 0) return false;
        IGaleedVenue.Seat memory s = _lastSeat(i);
        return s.state == 0 && s.accepted == 0 && _now() <= s.windowEnd;
    }

    function _canClaim(uint256 i) internal view returns (bool) {
        if (_ph(i) != 1 || _seatCount(i) == 0) return false;
        IGaleedVenue.Seat memory s = _lastSeat(i);
        return s.state == 0 && s.accepted != 0 && _now() <= s.windowEnd;
    }

    function _canConfirm(uint256 i) internal view returns (bool) {
        return _ph(i) == 2;
    }

    function _canDispute(uint256 i) internal view returns (bool) {
        if (_ph(i) != 2) return false;
        return _now() <= _lastSeat(i).windowEnd;
    }

    function _canRule(uint256 i) internal view returns (bool) {
        if (_ph(i) != 3) return false;
        return _now() <= _lastSeat(i).windowEnd;
    }

    function _canResign(uint256 i) internal view returns (bool) {
        if (!_isOpen(i) || _seatCount(i) == 0) return false;
        IGaleedVenue.Seat memory s = _lastSeat(i);
        return s.state <= 1 && (s.accepted == 0 || _now() <= s.windowEnd);
    }

    function _isLate(uint256 i) internal view returns (bool) {
        uint8 p = _ph(i);
        return p == 2 || p == 3;
    }

    function _canResignDisputed(uint256 i) internal view returns (bool) {
        return _canRule(i);
    }

    function _due(uint256 i) internal view returns (bool) {
        uint8 p = _ph(i);
        if (p < 1 || p > 3) return false;
        S memory st = _st(cases[i].inst);
        if (st.pending) return _drawClosable(st);
        if (st.seats == 0) return false;
        IGaleedVenue.Seat memory s = _lastSeat(i);
        return (s.state <= 1 || (s.state == 3 && p == 2)) && _now() > s.windowEnd;
    }

    function _isDisputedKeySeller(uint256 i) internal view returns (bool) {
        return _ph(i) == 3 && cases[i].sellerKind == 0;
    }

    function _pendingLive(uint256 i) internal view returns (bool) {
        if (_ph(i) != 1) return false;
        S memory st = _st(cases[i].inst);
        return st.pending && _now() <= st.redrawDeadline;
    }

    function _pendingAny(uint256 i) internal view returns (bool) {
        return _ph(i) == 1 && _st(cases[i].inst).pending;
    }

    /// An armed draw past its deadline: refused from the second block after the arming on, drawn in the
    /// block after the arming whatever the clock.
    function _pendingLate(uint256 i) internal view returns (bool) {
        if (_ph(i) != 1) return false;
        S memory st = _st(cases[i].inst);
        return st.pending && _now() > st.redrawDeadline;
    }

    /// An armed draw redraw refuses (NotPendingRedraw): its deadline passed and the block after the arming too.
    function _pendingDead(uint256 i) internal view returns (bool) {
        if (_ph(i) != 1) return false;
        S memory st = _st(cases[i].inst);
        return st.pending && _now() > st.redrawDeadline && vm.getBlockNumber() > uint256(st.drawBlock) + 1;
    }

    /// s7: advance closes an armed draw from the second block after the arming on, past its deadline or
    /// past the 256 blocks whose hash the chain serves.
    function _drawClosable(S memory st) internal view returns (bool) {
        uint256 b = vm.getBlockNumber();
        return st.pending && b > uint256(st.drawBlock) + 1 && (_now() > st.redrawDeadline || b > uint256(st.drawBlock) + 256);
    }

    function _judgeWithoutPayout(uint256 i) internal view returns (bool) {
        if (_ph(i) == 0) return false;
        C storage c = cases[i];
        uint256 n = _seatCount(i);
        for (uint256 s = 0; s < n; s++) {
            (address p,) = c.inst.accountOf(c.inst.seatOf(c.id, uint64(s)).judge, address(tok));
            if (p == address(0)) return true;
        }
        return false;
    }

    /// A seat ledger holding something for the case's credited judge, its payout registered or not (s8: withdraw
    /// registers the payout it names where none is).
    function _judgeHasMoney(uint256 i) internal view returns (bool) {
        C storage c = cases[i];
        if (c.credited == address(0)) return false;
        (, uint256 cl) = c.inst.accountOf(c.credited, address(tok));
        return cl != 0;
    }

    /// The instance knows both parties' keys: a case opened, a cancel, or a run of the expiry branch kept them (s4,
    /// s10); before that NotParty fails every sender.
    function _keysKnown(uint256 i) internal view returns (bool) {
        return _ph(i) != 0 || cases[i].branchRan;
    }

    // ------------------------------------------------------------------
    // the observation
    // ------------------------------------------------------------------

    function _before(uint256 i) internal {
        vm.recordLogs();
        C storage c = cases[i];
        S memory st = _st(c.inst);
        snap.bal = tok.balanceOf(address(c.inst));
        snap.unpaid = st.unpaid;
        snap.ph = st.ph;
        snap.lfHeld = st.lfHeld;
        snap.stray = c.stray;
        snap.seats = st.seats;
        snap.pending = st.pending;
        snap.deadline = st.redrawDeadline;
        snap.drawBlock = st.drawBlock;
        snap.panelSize = st.panelSize;
        address[3] memory pp = [c.t.buyerPayout, c.t.offer.sellerPayout, c.t.offer.sellerFund];
        for (uint256 k = 0; k < 3; k++) {
            snap.owedP[k] = c.inst.owed(pp[k], address(tok));
            snap.balP[k] = tok.balanceOf(pp[k]);
        }
        snap.owedK = c.inst.owed(c.t.offer.sellerKey, address(tok));
        snap.balK = tok.balanceOf(c.t.offer.sellerKey);
        snap.allowF = tok.allowance(c.t.offer.sellerFund, address(logic));
        snap.fundBound = c.t.offer.sellerFund == c.t.offer.sellerKey || boundG[c.t.offer.sellerKey][c.t.offer.sellerFund];
        snap.dual = c.credited != address(0) && c.credited == c.t.offer.sellerFund;
        if (snap.dual) {
            (snap.dPay, snap.dClaim) = c.inst.accountOf(c.credited, address(tok));
            snap.dOwed = c.inst.owed(c.credited, address(tok));
            snap.dBalJ = tok.balanceOf(c.credited);
            snap.dBalPay = snap.dPay == address(0) ? 0 : tok.balanceOf(snap.dPay);
        }
        snap.lastState = 0;
        snap.lastAccepted = 0;
        snap.lastWindowEnd = 0;
        snap.lastJudge = address(0);
        snap.lastJudgeKey = address(0);
        snap.judgeClaim = 0;
        snap.vRead = false;
        if (st.ph != 0 && st.seats != 0) {
            IGaleedVenue.Seat memory l = c.inst.seatOf(c.id, uint64(st.seats - 1));
            snap.lastState = l.state;
            snap.lastAccepted = l.accepted;
            snap.lastWindowEnd = l.windowEnd;
            snap.lastJudge = l.judge;
            snap.lastJudgeKey = _cur(l.judge);
            (, snap.judgeClaim) = c.inst.accountOf(l.judge, address(tok));
        }
    }

    /// s8, venue/1 s2.12: before a ruling, outcomeOf for the seat and rung named; before an advance, advanceOf for the
    /// seat named; both revert UnknownSeat for every pair seatOf refuses. What they answer is compared in `_afterView`
    /// with what the act then did on the state they read.
    function _readView(uint256 i, uint8 act, bytes memory data) internal {
        snap.vRead = false;
        if ((act != A_RULE && act != A_ADVANCE) || data.length < 68) return;
        C storage c = cases[i];
        bytes32 id;
        uint256 seat;
        uint256 rung;
        assembly ("memory-safe") {
            id := mload(add(data, 36))
            seat := mload(add(data, 68))
        }
        if (act == A_RULE) {
            assembly ("memory-safe") {
                rung := mload(add(data, 132))
            }
        }
        if (id != c.id || seat > type(uint64).max || rung > type(uint64).max) return;
        bytes memory q = act == A_RULE
            ? abi.encodeCall(Eravon.outcomeOf, (c.id, uint64(seat), uint64(rung), bytes("")))
            : abi.encodeCall(Eravon.advanceOf, (c.id, uint64(seat)));
        (bool ok, bytes memory ret) = address(c.inst).staticcall(q);
        if (snap.ph == 0 || seat >= snap.seats) {
            if (ok || ret.length != 4 || bytes4(ret) != Eravon.UnknownSeat.selector) {
                viewWrong++;
                _r5(abi.encode("viewUnknownSeat", act, seat, ok, ret));
            }
            _mark("view.unknownSeatRefused");
            return;
        }
        if (!ok) return; // a rung outside the ladder or a seat out of flight: rule refuses the same pair
        IGaleedVenue.Outcome memory o = abi.decode(ret, (IGaleedVenue.Outcome));
        if (o.shares.length != 2 || o.rulingFeeFrom.length != 2) {
            viewWrong++;
            _r5(abi.encode("viewShape", act));
            return;
        }
        snap.vRead = true;
        IGaleedVenue.Seat memory s = c.inst.seatOf(c.id, uint64(seat));
        snap.vInFlight = s.state <= 1 && seat + 1 == snap.seats;
        if (!snap.vInFlight && act == A_ADVANCE) {
            // out of flight: the seat's present state, every amount zero
            if (o.state != s.state || o.shares[0] != 0 || o.shares[1] != 0 || o.standbyFee != 0 || o.rulingFee != 0) {
                viewWrong++;
                _r5(abi.encode("viewOutOfFlight", seat, o.state, s.state));
            }
        }
        snap.vState = o.state;
        snap.vShare0 = o.shares[0];
        snap.vShare1 = o.shares[1];
        snap.vStandby = o.standbyFee;
        snap.vRuling = o.rulingFee;
        snap.vFrom0 = o.rulingFeeFrom[0];
        snap.vFrom1 = o.rulingFeeFrom[1];
    }

    /// The view read before the act against what the act did: the seat's state, the judge's credit, the buyer's
    /// share of A and the part of the seller's LF sent to the buyer (s8: one function answers both views and the split).
    function _afterView(uint256 i, uint8 act, Ev memory ev, uint8 ph) internal {
        if (!snap.vRead) return;
        C storage c = cases[i];
        if (act == A_RULE) {
            uint8 st = c.inst.seatOf(c.id, uint64(snap.seats - 1)).state;
            bool bad = !ev.closed || ev.term != 3 || snap.vState != 2 || st != 2 || ev.j != snap.vStandby + snap.vRuling
                || snap.vShare0 + snap.vShare1 != c.a || ev.bond != snap.vFrom1 || snap.vFrom0 + snap.vFrom1 != snap.vRuling
                || ev.b != snap.vShare0 + snap.stray + snap.vFrom1 || snap.vStandby != c.p || snap.vRuling != snap.lfHeld;
            if (bad) {
                viewWrong++;
                _r5(abi.encode("outcomeOf", ev.j, snap.vStandby, snap.vRuling, ev.b, snap.vShare0, ev.bond, snap.vFrom1));
            }
            _mark("view.outcomeBindsTheRuling");
            return;
        }
        if (!snap.vInFlight) return;
        uint8 stA = c.inst.seatOf(c.id, uint64(snap.seats - 1)).state;
        bool badA = stA != snap.vState || snap.vRuling != 0;
        if (ev.closed) {
            uint256 back = ev.term == 5 ? snap.lfHeld : 0;
            uint256 buyerA = ev.b >= snap.stray + back ? ev.b - snap.stray - back : type(uint256).max;
            if (ev.j != snap.vStandby || buyerA != snap.vShare0 || snap.vShare0 + snap.vShare1 != c.a) badA = true;
            _mark(ph == 9 ? "view.advanceBindsTheLapse" : "view.advanceBindsTheClose");
        } else if (snap.vShare0 != 0 || snap.vShare1 != 0 || snap.vStandby != 0) {
            badA = true;
        }
        if (badA) {
            viewWrong++;
            _r5(abi.encode("advanceOf", stA, snap.vState, ev.j, snap.vStandby, ev.b, snap.vShare0));
        }
    }

    function _scan(C storage c, VmSafe.Log memory l, uint8 act, Ev memory ev) internal {
        if (l.topics.length == 0) return;
        bytes32 t0 = l.topics[0];
        if (t0 == IGaleedVenue.Anchored.selector) {
            if (act != A_RULE && act != A_RESIGN) anchoredOutside++;
            ev.anchors++;
            bytes memory entry = abi.decode(l.data, (bytes));
            if (
                l.emitter != address(c.inst) || address(uint160(uint256(l.topics[1]))) != snap.lastJudgeKey || l.topics[2] != E
                    || keccak256(entry) != keccak256(ENTRY) || ev.anchors > 1
            ) anchorWrong++;
            return;
        }
        if (t0 == Eravon.Pulled.selector || t0 == Eravon.Consented.selector || t0 == Eravon.Revoked.selector) {
            if (l.emitter != address(logic)) eventOnWrongEmitter++;
        }
        if (l.emitter == address(logic) && t0 == Eravon.Pulled.selector) {
            ev.pulls++;
            address seller_ = address(uint160(uint256(l.topics[1])));
            (address token_, address instance, uint256 amount) = abi.decode(l.data, (address, address, uint256));
            if (act != A_INIT || instance != address(c.inst) || amount != c.d || seller_ != c.t.offer.sellerKey || token_ != address(tok)) {
                pullNotToCaller++;
            }
            // round 6 (s7a, s10): Pulled names the fund as its second topic, the order's sellerFund, which is the
            // seller's key or an address bound to the seller before the pull, as the logic itself answers
            address fund_ = l.topics.length == 3 ? address(uint160(uint256(l.topics[2]))) : address(0);
            if (
                l.topics.length != 3 || l.data.length != 96 || fund_ != c.t.offer.sellerFund || !snap.fundBound
                    || (fund_ != seller_ && !logic.funds(seller_, fund_))
            ) {
                pullFromUnbound++;
                _r5(abi.encode("pulled", seller_, fund_, c.t.offer.sellerFund, snap.fundBound));
            }
            c.pulled += amount;
            return;
        }
        if (l.emitter != address(c.inst)) return;
        ev.instEvents++;
        if (t0 == IGaleedVenue.Seated.selector) {
            ev.seated++;
            ev.seatedJudge = address(uint160(uint256(l.topics[1])));
            ev.seatedCase = l.topics[2];
            ev.seatedOrdinal = abi.decode(l.data, (uint64));
            return;
        }
        if (t0 == Eravon.DrawArmed.selector) {
            ev.armed++;
            (ev.armedBlock, ev.armedDeadline) = abi.decode(l.data, (uint64, uint64));
            return;
        }
        if (t0 == Eravon.Closed.selector) {
            if (c.closedSeen) closedTwice++;
            c.closedSeen = true;
            ev.closed = true;
            (ev.term, ev.b, ev.s, ev.j, ev.bond) = abi.decode(l.data, (uint8, uint256, uint256, uint256, uint256));
        } else if (t0 == Eravon.Refunded.selector) {
            ev.refunded = true;
            ev.cause = abi.decode(l.data, (uint8));
        } else if (t0 == Eravon.Settled.selector) {
            ev.bps = abi.decode(l.data, (uint16));
        } else if (t0 == Eravon.Ruled.selector) {
            (ev.rung,) = abi.decode(l.data, (uint64, bytes32));
        } else if (t0 == Eravon.Lapsed.selector) {
            ev.lapsed = true;
            ev.lapseBps = abi.decode(l.data, (uint16));
        } else if (t0 == Eravon.Resigned.selector) {
            (, uint8 form) = abi.decode(l.data, (bytes32, uint8));
            if (form == 3) ev.form3 = true;
        } else if (t0 == Eravon.Swept.selector) {
            address who = address(uint160(uint256(l.topics[1])));
            (address tk, uint256 v) = abi.decode(l.data, (address, uint256));
            // round 7: where the credited judge is also the case's sellerFund, Swept names it for either ledger; what
            // left its seat ledger is read from that ledger instead (_dualAfter)
            if (tk == address(tok) && who != address(0) && who == c.credited && who != c.t.offer.sellerFund) c.judgeOut += v;
        }
    }

    function _after(uint256 i, uint8 act) internal {
        VmSafe.Log[] memory logs = vm.getRecordedLogs();
        C storage c = cases[i];
        Ev memory ev;
        for (uint256 k = 0; k < logs.length; k++) {
            _scan(c, logs[k], act, ev);
        }
        uint8 ph = _ph(i);
        // Refunded's causes (s5)
        if (ev.refunded) {
            if (ev.cause == 4 && (act != A_CANCEL || snap.ph != 0)) cancelAfterInit++;
            if (ev.cause == 2 && (act != A_INIT || snap.ph != 0 || ph != 0)) causeWrong++;
            if (ev.cause == 3 && act != A_REFUND) causeWrong++;
            if ((ev.cause == 0 || ev.cause == 1) && act != A_ADVANCE) causeWrong++;
            if (ev.cause == 1 && c.track != 0) causeWrong++;
            if (ev.cause > 4) causeWrong++;
        }
        // the deposit is pulled once, by the opening init, and never otherwise (s7a)
        bool openedNow = act == A_INIT && snap.ph == 0 && ph == 1;
        if (openedNow) {
            c.opened = true;
            _checkFundPull(i);
            // s8, venue/1 s3: nothing registers a payout at the opening, neither the order nor the quote carrying one
            if (c.track == 0) {
                (address p0,) = c.inst.accountOf(c.t.offer.judge, address(tok));
                if (p0 != address(0)) payoutAtInit++;
                _mark("payout.noneAtInit");
            }
        }
        if (ev.pulls != (openedNow && c.d != 0 ? 1 : 0)) pullCountWrong++;
        // the expiry branch (s4 step 3): no case opens; the whole balance is credited to the buyer's payout
        // and pushed where the token lets it, so that what stays is that credit and nothing else
        if (act == A_INIT && snap.ph == 0) {
            bool dead = _now() > c.t.offer.expiry || _now() > c.t.openBy || revokedTerms[c.t.offer.sellerKey][c.t.offer.termsHash];
            if (dead && ph != 0) revokedOpened++;
            if (ph == 0) {
                uint256 left = tok.balanceOf(address(c.inst));
                uint256 unpaidNow = _st(c.inst).unpaid;
                if (
                    !dead || !ev.refunded || ev.cause != 2 || left != unpaidNow
                        || c.inst.owed(c.t.buyerPayout, address(tok)) != unpaidNow
                ) expiryBranchWrong++;
                _mark("init.expiryBranch");
                if (left != 0) _mark("init.expiryBranchPushFailedCreditKept");
                if (snap.unpaid != 0) _mark("init.expiryBranchAgainWithACreditOutstanding");
            }
            if (openedNow) _markInit(c);
        }
        if (act == A_REVOKE && ph != snap.ph) revokeTouchedOpen++;
        // venue/1 s2.2: one Seated per seat opened, with its ordinal, the case and the judge; on the guild
        // tracks only by a draw, the member the formula of s7 names
        uint256 nAfter = ph == 0 ? 0 : _seatCount(i);
        if (nAfter < snap.seats || ev.seated != nAfter - snap.seats) seatedWrong++;
        if (ev.seated == 1 && nAfter == snap.seats + 1) {
            if (ev.seatedOrdinal != snap.seats || ev.seatedCase != c.id) seatedWrong++;
            address sj = c.inst.seatOf(c.id, uint64(snap.seats)).judge;
            if (ev.seatedJudge != sj) seatedWrong++;
            if (c.track == 0) {
                if (act != A_INIT) seatedWrong++;
            } else if (act != A_REDRAW || !snap.pending || sj != _drawModel(c, snap.drawBlock, uint64(snap.seats))) {
                seatedWrong++;
            }
        }
        // accept emits nothing of its own (s4); commit emits its one Committed (s6)
        if (act == A_ACCEPT && ev.instEvents != 0) eventWrong++;
        if (act == A_COMMIT && ev.instEvents != 1) eventWrong++;
        if (act == A_REVOKE) _mark(snap.ph == 0 ? "revoke.beforeInit" : "revoke.afterInit");
        // the terminal table (s8)
        if (ev.closed) _checkClosed(i, act, ev, ph);
        else if (snap.ph < 4 && ph >= 4) closedModelWrong++;
        // the stray ghost
        if (openedNow) {
            c.stray = 0;
        } else if (snap.ph == 2 && ph == 3) {
            c.stray = snap.stray >= c.lf ? snap.stray - c.lf : 0;
        } else if (snap.ph < 4 && ph >= 4) {
            c.stray = 0;
        }
        _checkDraw(i, act, ev, ph, nAfter);
        _round2(i, act, ev, ph, nAfter);
        _round3(act);
        _round5(i, act, ev, nAfter);
        _afterView(i, act, ev, ph);
        if (snap.ph != ph) _mark(string(abi.encodePacked("phase.", vm.toString(snap.ph), ".to.", vm.toString(ph))));
        if (ev.closed && act != A_CANCEL) {
            if (
                c.inst.owed(c.t.buyerPayout, address(tok)) != 0 || c.inst.owed(c.t.offer.sellerPayout, address(tok)) != 0
                    || c.inst.owed(c.t.offer.sellerFund, address(tok)) != 0
            ) _mark("terminal.pushFailedCreditKept");
        }
        _dualAfter(i, act);
        _observe(i);
        _sameBlockDraw(i);
    }

    /// Round 2's observations of one action: the drawn seat's block past the armed block; the three ways a
    /// guild case closes without a seat; the terminal, its cause and form 3 from the pre-state; s8's P and fee
    /// rows from the pre-state; a payee whose hook burns gas keeps its
    /// credit.
    function _round2(uint256 i, uint8 act, Ev memory ev, uint8 ph, uint256 nAfter) internal {
        C storage c = cases[i];
        if (ev.seated == 1 && c.track != 0 && !(snap.drawBlock < vm.getBlockNumber())) seatedNotAfterDraw++;
        if (act == A_INIT && snap.ph == 0 && ph == 0 && ev.refunded && ev.cause == 2) {
            c.branchRan = true;
            _checkBurners(i);
        }
        if (ev.closed && act != A_CANCEL && nAfter == 0) {
            uint8 how = 0;
            if (act == A_REFUND && ev.refunded && ev.cause == 3 && ev.term == 0) how = 1;
            else if (act == A_SETTLE && ev.term == 2) how = 2;
            else if ((act == A_ADVANCE || act == A_REDRAW) && ev.term == 4 && ev.form3) how = 3;
            if (c.track == 0 || how == 0 || !snap.pending) zeroSeatWrong++;
            c.noSeatClose = how;
            _mark(how == 1 ? "zeroSeats.closedByRefund" : (how == 2 ? "zeroSeats.closedBySettle" : "zeroSeats.closedByForm3"));
        }
        _checkTerminal(i, act, ev, ph);
        if (ev.closed && act != A_CANCEL) _checkP(i, act, ev);
        if (ev.closed) _checkBurners(i);
    }

    /// Round 3's observation of one action (s8, s9): no refund lands during a dispute within T3, its last second
    /// included, and the signed form none during a dispute at all.
    function _round3(uint8 act) internal {
        if (act == A_REFUND && snap.ph == 3 && (refundSigned || _now() <= snap.lastWindowEnd)) disputeRefundWrong++;
    }

    /// The terminal the pre-state names for the action observed (s5, s6, s7, s8), with its Refunded cause and
    /// its form 3, against what the instance emitted; actions that close nothing close nothing.
    function _checkTerminal(uint256 i, uint8 act, Ev memory ev, uint8 ph) internal {
        C storage c = cases[i];
        bool closes;
        uint8 term;
        uint8 cause = 255;
        bool form3;
        bool exhausted = snap.seats >= 3 || snap.seats >= snap.panelSize;
        uint256 nowT = _now();
        if (act == A_CANCEL) {
            closes = snap.ph == 0;
            cause = 4;
        } else if (act == A_REFUND) {
            closes = true;
            cause = 3;
        } else if (act == A_SETTLE) {
            closes = true;
            term = 2;
        } else if (act == A_RULE) {
            closes = true;
            term = 3;
        } else if (act == A_CONFIRM) {
            closes = true;
            term = 1;
        } else if (act == A_DISPUTE) {
            closes = snap.lastState == 3;
            term = 5;
        } else if (act == A_RESIGN) {
            if (snap.lastAccepted == 0) {
                if (c.track == 0 || exhausted) {
                    closes = true;
                    term = 4;
                    form3 = c.track != 0;
                }
            } else if (snap.ph == 1) {
                closes = true;
                term = 4;
            } else if (snap.ph == 3) {
                closes = true;
                term = 5;
            }
        } else if (act == A_REDRAW) {
            // a seed the chain no longer serves closes the case as Resigned with form 3
            if (blockhash(snap.drawBlock) == bytes32(0)) {
                closes = true;
                term = 4;
                form3 = true;
            }
        } else if (act == A_ADVANCE) {
            bool due = snap.seats != 0 && advSeat == snap.seats - 1
                && (snap.lastState <= 1 || (snap.lastState == 3 && snap.ph == 2)) && nowT > snap.lastWindowEnd;
            if (due) {
                closes = true;
                if (snap.lastAccepted == 0) {
                    if (c.track == 0) {
                        cause = 1;
                    } else if (exhausted) {
                        term = 4;
                        form3 = true;
                    } else {
                        closes = false;
                    }
                } else if (snap.ph == 1) {
                    cause = 0;
                } else if (snap.ph == 2) {
                    term = 1;
                } else {
                    term = 5;
                }
            } else if (
                snap.pending && vm.getBlockNumber() > uint256(snap.drawBlock) + 1
                    && (nowT > snap.deadline || vm.getBlockNumber() > uint256(snap.drawBlock) + 256)
            ) {
                // the draw nobody made: by its deadline (the block after the arming excepted) or by its seed's loss
                closes = true;
                term = 4;
                form3 = true;
            }
        }
        if (ev.closed != closes) {
            terminalWrong++;
            return;
        }
        if (!closes) return;
        if (ev.term != term || uint256(ph) != uint256(term) + 4 || ev.form3 != form3) terminalWrong++;
        if (cause != 255 && (!ev.refunded || ev.cause != cause)) terminalWrong++;
        if (cause == 255 && ev.refunded) terminalWrong++;
    }

    /// s8's P and fee rows from the pre-state alone: P to the judge at Released with the seat seated, at a
    /// settlement under an accepted seat in flight at or before its windowEnd or seated while Claimed at any
    /// time, at a ruling, and at a refund while Claimed under a seated seat; the buyer's fee to the judge at
    /// the ruling alone (a refund during a dispute within T3 is refused).
    function _checkP(uint256 i, uint8 act, Ev memory ev) internal {
        C storage c = cases[i];
        bool inWindow = _now() <= snap.lastWindowEnd;
        bool p;
        if (act == A_CONFIRM) p = snap.lastState == 0;
        else if (act == A_ADVANCE) p = snap.ph == 2 && snap.lastState == 0;
        else if (act == A_SETTLE) {
            p = snap.seats != 0 && snap.lastAccepted != 0
                && ((snap.lastState == 0 && (inWindow || snap.ph == 2)) || (snap.lastState == 1 && inWindow));
        }
        else if (act == A_REFUND) p = snap.ph == 2 && snap.lastState == 0;
        else if (act == A_RULE) p = true;
        bool fee = act == A_RULE;
        uint256 want = (p ? c.p : 0) + (fee ? snap.lfHeld : 0);
        if (ev.j != want) pOutsideTable++;
        if (want != 0) {
            (, uint256 cl) = c.inst.accountOf(snap.lastJudge, address(tok));
            if (cl != snap.judgeClaim + want) pOutsideTable++;
        }
    }

    /// s8: a payee whose hook burns the gas a push hands it receives nothing and keeps its whole credit.
    function _checkBurners(uint256 i) internal {
        C storage c = cases[i];
        address[3] memory pp = [c.t.buyerPayout, c.t.offer.sellerPayout, c.t.offer.sellerFund];
        bool any;
        for (uint256 k = 0; k < 3; k++) {
            if (!btok.burns(pp[k])) continue;
            any = true;
            if (tok.balanceOf(pp[k]) != snap.balP[k] || c.inst.owed(pp[k], address(tok)) < snap.owedP[k]) burnerPaid++;
        }
        if (any) _mark("terminal.underABurningHook");
    }

    function _markInit(C storage c) internal {
        if (c.unit) {
            // standard s3: the five values pinned from the two feeds and the converted absolutes
            (uint256 lf, uint256 cap, uint256 pT, uint256 pU, uint64 at) = c.inst.feeQuoted();
            if (lf != c.lf || cap != c.capInToken || pT != 2000e8 || pU != 1e8 || at != feedsAt) pricedWrong++;
            _mark("init.unitPriced");
        }
        if (c.a > type(uint256).max / 10000 - 1000) _mark("init.amountAtTheBound");
        if (c.t.offer.nonce == MAX_INT53 && c.t.offer.expiry == MAX_INT53 && c.t.openBy == MAX_INT53) _mark("init.int53AtTheBound");
        _mark(c.track == 0 ? "init.named" : (c.track == 1 ? "init.guild" : (c.track == 2 ? "init.guildWithParties" : "init.guildOfFive")));
        if (c.sellerKind == 1) _mark("init.companySeller");
        if (c.buyerKind == 1) _mark("init.companyBuyer");
        if (c.sellerForm == 1) _mark("init.sellerByCall");
        if (c.sellerForm == 2) _mark("init.sellerByRegistry");
        if (c.buyerForm == 1) _mark("init.buyerByCall");
        if (c.buyerForm == 2) _mark("init.buyerByRegistry");
        if (c.opener == 1) _mark("init.byBuyer");
        if (c.opener == 2) _mark("init.bySellerAfterAStranger");
        if (_cur(c.t.offer.sellerKey) != c.t.offer.sellerKey || _cur(c.t.buyerKey) != c.t.buyerKey) _mark("init.underAMovedKey");
    }

    /// s7's draw from the text: the frozen panel's members that are neither party's key nor seated before on
    /// this case, in the panel's order; the one at keccak256(caseId, blockhash(drawBlock), seat) mod their count.
    function _drawModel(C storage c, uint64 drawBlock, uint64 ordinal) internal view returns (address) {
        (address[] memory m,) = _frozen(c);
        bytes32 seed = blockhash(drawBlock);
        if (seed == bytes32(0)) return address(0);
        address[] memory el = new address[](m.length);
        uint256 n;
        for (uint256 k = 0; k < m.length; k++) {
            address a = m[k];
            if (a == address(0) || a == c.t.offer.sellerKey || a == c.t.buyerKey) continue;
            bool before;
            for (uint64 r = 0; r < ordinal; r++) {
                if (c.inst.seatOf(c.id, r).judge == a) before = true;
            }
            if (!before) el[n++] = a;
        }
        if (n == 0) return address(0);
        return el[uint256(keccak256(abi.encodePacked(c.id, seed, ordinal))) % n];
    }

    /// s7's draw machine: a draw is armed at the guild's init and after an unaccepted seat ended, on the
    /// block of that action, with its deadline offerWindow on; it stays armed with the same block and deadline
    /// until a draw seats a member from the armed block's hash or a terminal closes the case (a redraw that
    /// finds that block's hash gone closes it as Resigned form 3); DrawArmed is emitted exactly where a draw
    /// is armed.
    function _checkDraw(uint256 i, uint8 act, Ev memory ev, uint8 ph, uint256 nAfter) internal {
        C storage c = cases[i];
        S memory st = _st(c.inst);
        if (snap.pending) _checkClocks(act, ph, nAfter, st);
        if (st.pending) {
            if (ph != 1 || c.track == 0 || st.drawBlock > vm.getBlockNumber()) drawWrong++;
            if (!snap.pending) {
                if (st.drawBlock != vm.getBlockNumber() || st.redrawDeadline != _now() + c.offerWindow) drawWrong++;
                if (ev.armed != 1 || ev.armedBlock != st.drawBlock || ev.armedDeadline != st.redrawDeadline) drawWrong++;
                bool atInit = act == A_INIT && snap.ph == 0 && nAfter == 0;
                bool afterSeat = false;
                if ((act == A_RESIGN || act == A_ADVANCE) && nAfter == snap.seats && nAfter != 0) {
                    IGaleedVenue.Seat memory l = c.inst.seatOf(c.id, uint64(nAfter - 1));
                    afterSeat = l.accepted == 0 && (l.state == 3 || l.state == 5);
                }
                if (!atInit && !afterSeat) drawWrong++;
                _mark(atInit ? "draw.armedAtInit" : "draw.armedAfterAnUnacceptedSeat");
            } else {
                // an armed draw keeps its block and its deadline: nothing re-arms
                if (st.redrawDeadline != snap.deadline || nAfter != snap.seats) drawWrong++;
                if (st.drawBlock != snap.drawBlock || ev.armed != 0) drawWrong++;
            }
        } else {
            if (ev.armed != 0) drawWrong++;
            if (snap.pending) {
                bool drew = act == A_REDRAW && ph == 1 && nAfter == snap.seats + 1;
                bool stale = blockhash(snap.drawBlock) == bytes32(0) && vm.getBlockNumber() > snap.drawBlock + 1;
                if (drew && stale) drawWrong++;
                if (!drew && ph < 4) drawWrong++;
                // a redraw on a live seed draws; on a stale seed it closes the case as Resigned form 3
                if (act == A_REDRAW && !drew && (ph != 8 || !stale)) drawWrong++;
                if (act == A_REDRAW && !drew && ph == 8 && stale) _mark("redraw.stale");
                if (ph >= 4 && nAfter == 0) _mark("terminal.withoutASeat");
            }
        }
    }

    /// s7's two clocks, from the pre-state: redraw is admitted within the deadline or in the block after the
    /// arming whatever the clock, draws the formula's member while the armed block's hash is served (at most 256
    /// blocks back) and closes the case as Resigned form 3 once it is not; advance closes an armed draw from the
    /// second block after the arming by the deadline or by the seed's loss, and leaves it armed otherwise; nothing
    /// moves the armed block while the draw stays armed.
    function _checkClocks(uint8 act, uint8 ph, uint256 nAfter, S memory st) internal {
        uint256 bn = vm.getBlockNumber();
        uint256 db = snap.drawBlock;
        bool late = _now() > snap.deadline;
        bool lost = bn > db + 256;
        if (st.pending && st.drawBlock != snap.drawBlock) drawBlockMoved++;
        if (act == A_ADVANCE) {
            bool due = bn > db + 1 && (late || lost);
            if (due) {
                if (ph != 8 || st.pending || nAfter != snap.seats) {
                    drawOutcomeWrong++;
                    if (lost) staleRearmed++;
                }
                _mark(lost ? "advance.drawSeedLostClosed" : "advance.drawDeadlineClosed");
            } else {
                if (!st.pending || st.drawBlock != snap.drawBlock || ph != 1) drawOutcomeWrong++;
                if (bn == db + 1 && late) _mark("advance.firstBlockLeftArmed");
            }
        } else if (act == A_REDRAW) {
            if ((late && bn > db + 1) || bn <= db) drawOutcomeWrong++; // a call s7 refuses landed
            if (lost) {
                if (ph != 8 || st.pending || nAfter != snap.seats) {
                    drawOutcomeWrong++;
                    staleRearmed++;
                }
                _mark("redraw.seedLostClosed");
            } else {
                if (ph != 1 || nAfter != snap.seats + 1 || st.pending) {
                    drawOutcomeWrong++;
                    if (bn == db + 1) firstBlockRefused++;
                }
                if (bn == db + 1 && late) _mark("redraw.firstBlockPastTheDeadline");
                if (bn == db + 256) _mark("redraw.lastServedBlock");
            }
        }
    }

    /// A draw armed on this block refuses to draw in it (DrawPending), whatever the clock: its seed is a hash
    /// nobody here sees; a stranger meets the same refusal (s7: the draw is open to any sender).
    function _sameBlockDraw(uint256 i) internal {
        C storage c = cases[i];
        S memory st = _st(c.inst);
        if (!st.pending || st.drawBlock != vm.getBlockNumber()) return;
        (address[] memory m, bytes32[] memory w) = _frozen(c);
        bytes memory data = abi.encodeCall(Eravon.redraw, (m, w));
        (bool ok, bytes memory e) = _try(_drawer(c, uint256(keccak256(abi.encode(c.id, st.drawBlock)))), address(c.inst), data);
        if (ok || e.length != 4 || bytes4(e) != Eravon.DrawPending.selector) refusedRedrawPassed++;
        (ok, e) = _try(STRANGER, address(c.inst), data);
        if (ok || e.length != 4 || bytes4(e) != Eravon.DrawPending.selector) refusedRedrawPassed++;
        _mark("redraw.sameBlockRefused");
    }

    /// A drawer chosen by the case and the block (s7: anyone; here a party, the guild or a stranger).
    function _drawerNow(C storage c) internal view returns (address) {
        return _drawer(c, uint256(keccak256(abi.encode(c.id, vm.getBlockNumber(), vm.getBlockTimestamp()))));
    }

    /// Who draws (s7: anyone): the seller's key, the buyer's, the guild's frozen root, or a stranger.
    function _drawer(C storage c, uint256 h) internal view returns (address) {
        uint256 w = h % 4;
        if (w == 0) return c.t.offer.sellerKey;
        if (w == 1) return c.t.buyerKey;
        if (w == 2) return c.t.offer.guild;
        return STRANGER;
    }

    function _checkClosed(uint256 i, uint8 act, Ev memory ev, uint8 ph) internal {
        C storage c = cases[i];
        lastClosedSeen = true;
        lastClosedJ = ev.j;
        if (snap.bal < snap.unpaid || ev.b + ev.s + ev.j != snap.bal - snap.unpaid) closedSumWrong++;
        if (act == A_CANCEL) {
            // the balance beyond what is already owed, all of it to the buyer
            if (
                ev.term != 0 || snap.bal < snap.unpaid || ev.b != snap.bal - snap.unpaid || ev.s != 0 || ev.j != 0 || ev.bond != 0
                    || snap.ph != 0
            ) closedModelWrong++;
            return;
        }
        uint256 bps;
        bool pJ;
        bool lfJ;
        uint256 bond;
        uint8 term = ev.term;
        if (term == 0) {
            // a refund while Claimed under a seated seat pays P to the judge as T2's lapse would; during a
            // dispute within T3 it is refused (probed in doRefund)
            bps = 10000;
            pJ = snap.ph == 2 && snap.lastState == 0;
        } else if (term == 4) {
            bps = 10000;
        } else if (term == 1) {
            pJ = snap.lastState == 0;
        } else if (term == 2) {
            // P to the judge where the last seat was accepted and its window still runs, or where it is seated
            // while the case is Claimed at any time (T2's lapse releases with P to the judge)
            bps = ev.bps;
            pJ = snap.seats != 0 && snap.lastAccepted != 0
                && ((snap.lastState == 0 && (_now() <= snap.lastWindowEnd || snap.ph == 2))
                    || (snap.lastState == 1 && _now() <= snap.lastWindowEnd));
            if (!pJ && snap.seats != 0 && snap.lastState <= 1) _mark("settle.pBackFromASeatThatDidNotStand");
        } else if (term == 3) {
            Eravon.Rung storage o = c.t.offer.ladder[ev.rung];
            bps = o.buyerBps;
            pJ = true;
            lfJ = true;
            bond = o.feeBearer == 1 ? c.lf : (o.feeBearer == 2 ? c.lf / 2 : 0);
            if (o.feeBearer == 2 && c.lf % 2 == 1) _mark("ruled.oddHalf");
        } else {
            // s5, s6: Lapsed at the share lapseDefault names, 0, 5000 or 10000 bps of A to the buyer, as Lapsed reports it
            bps = uint256(c.lapseDefault) * 5000;
            if (!ev.lapsed || ev.lapseBps != bps) closedModelWrong++;
            _mark(c.lapseDefault == 0 ? "lapsed.none" : (c.lapseDefault == 1 ? "lapsed.half" : "lapsed.all"));
        }
        uint256 buyerA = (c.a * bps) / 10000;
        uint256 eb = buyerA + snap.stray + (lfJ ? 0 : snap.lfHeld) + bond;
        uint256 es = (c.a - buyerA) + (pJ ? 0 : c.p) + (c.lf - bond);
        uint256 ej = (pJ ? c.p : 0) + (lfJ ? snap.lfHeld : 0);
        if (ev.b != eb || ev.s != es || ev.j != ej || ev.bond != bond || uint256(term) + 4 != ph) closedModelWrong++;
        // round 5: the seller's share of A lands at sellerPayout; round 6: the deposit returned lands at sellerFund (s8),
        // the three addresses distinct or not, and the seller's key receives nothing where it is none of them
        _checkSplit(c, c.a - buyerA, (pJ ? 0 : c.p) + (c.lf - bond), eb);
        if (act == A_REFUND && !(term == 0 && ev.refunded && ev.cause == 3)) closedModelWrong++;
        // round 4: a Refunded credits the judge P alone, and only from Claimed under a seated seat; a refund never
        // lands in a dispute at or before T3's end
        if (term == 0) {
            if (ev.j != (snap.ph == 2 && snap.lastState == 0 ? c.p : 0)) refundedJudgeWrong++;
            if (snap.ph == 3 && _now() <= snap.lastWindowEnd) refundInT3Wrong++;
        }
        if (ev.j != 0) {
            if (snap.seats == 0) {
                judgeCreditWrong++;
            } else {
                uint8 lastNow = c.inst.seatOf(c.id, uint64(snap.seats - 1)).state;
                bool phaseOk = ph == 5 || ph == 6 || ph == 7 || (ph == 4 && snap.ph == 2 && snap.lastState == 0);
                if (!phaseOk || !(lastNow == 5 || lastNow == 2)) judgeCreditWrong++;
                (, uint256 cl) = c.inst.accountOf(snap.lastJudge, address(tok));
                if (cl != snap.judgeClaim + ev.j) judgeCreditWrong++;
                if (c.credited != address(0) && c.credited != snap.lastJudge) judgeCreditWrong++;
                c.credited = snap.lastJudge;
                c.judgeCredited += ev.j;
                _mark("judge.credited");
                if (snap.lastJudge == c.t.offer.sellerFund) _mark("dual.terminal");
            }
        }
        // s8, venue/1 s2.3: the last seat carries P as its standbyCredited where the terminal credited it to the judge,
        // and the buyer's fee as its rulingCredited where a ruling did; every other seat neither
        uint256 nS = ph == 0 ? 0 : _seatCount(i);
        for (uint256 s = 0; s < nS; s++) {
            IGaleedVenue.Seat memory v = c.inst.seatOf(c.id, uint64(s));
            bool last = s + 1 == nS;
            uint256 wantS = last && ev.j != 0 && pJ ? c.p : 0;
            uint256 wantR = last && ev.j != 0 && lfJ ? snap.lfHeld : 0;
            if (v.standbyCredited != wantS || v.rulingCredited != wantR) {
                seatCreditWrong++;
                _r5(abi.encode("seatCredit", i, s, v.standbyCredited, wantS, v.rulingCredited, wantR));
            }
        }
        if (ev.j != 0 && nS != 0) _mark("seat.creditedFields");
        if (snap.stray != 0) _mark("terminal.withStray");
    }

    function _observe(uint256 i) internal {
        C storage c = cases[i];
        S memory st = _st(c.inst);
        uint8 ph = st.ph;
        if (ph < c.lastPhase) phaseWentBack++;
        if (c.lastPhase >= 4 && ph != c.lastPhase) phaseWentBack++;
        c.lastPhase = ph;
        if (ph == 0) return;
        uint256 n = st.seats;
        if (n == 0) {
            // no seat: a cancelled order; or a guild case whose draw waits, or that closed before any draw
            bool ok0 = c.opened ? (c.track != 0 && (ph == 1 ? st.pending : (ph == 4 || ph == 6 || ph == 8))) : ph == 4;
            if (!ok0) seatTableBroken++;
            return;
        }
        for (uint256 s = 0; s < n; s++) {
            IGaleedVenue.Seat memory v = c.inst.seatOf(c.id, uint64(s));
            if (s < c.seatStates.length) {
                uint8 was = c.seatStates[s];
                // s5: seated to disputed, resigned or closed; disputed to ruled, resigned, lapsed or closed; the rest final
                bool ok = was == v.state || (was == 0 && (v.state == 1 || v.state == 3 || v.state == 5))
                    || (was == 1 && v.state >= 2 && v.state <= 5);
                if (!ok) seatMachineBroken++;
                // accepted is written once, while the seat is seated
                if (c.seatAccepted[s] != 0 && v.accepted != c.seatAccepted[s]) seatMachineBroken++;
                if (c.seatAccepted[s] == 0 && v.accepted != 0 && was != 0) seatMachineBroken++;
                c.seatStates[s] = v.state;
                c.seatAccepted[s] = v.accepted;
            } else {
                if (v.state != 0) seatMachineBroken++;
                c.seatStates.push(v.state);
                c.seatAccepted.push(v.accepted);
            }
            if (v.state == 1 && v.accepted == 0) seatMachineBroken++; // venue/1 s2.7
            if (v.state == 4 && ph != 9) seatMachineBroken++; // lapsed from T3 alone
            if (v.state == 2 && ph != 7) seatMachineBroken++;
            if (v.state <= 1 && (ph >= 4 || s + 1 < n)) seatMachineBroken++; // only the last seat in flight, none after a terminal
            // s5, s9: the five parties as the order recorded them, whatever key the succession book now answers for one
            if (
                v.parties.length != 5 || v.parties[0] != c.t.buyerKey || v.parties[1] != c.t.offer.sellerKey
                    || v.parties[2] != c.t.offer.sellerFund || v.parties[3] != c.t.buyerPayout || v.parties[4] != c.t.offer.sellerPayout
            ) {
                seatTableBroken++;
                _r5(abi.encode("parties", i, s));
            }
            // s8: nothing is credited to a seat before the terminal
            if (ph <= 3 && (v.standbyCredited != 0 || v.rulingCredited != 0)) seatCreditWrong++;
        }
        _table(c, st, c.inst.seatOf(c.id, uint64(n - 1)));
    }

    /// s5's table: the last seat's state and window for each phase of the case.
    function _table(C storage c, S memory st, IGaleedVenue.Seat memory L) internal {
        uint8 ph = st.ph;
        bool ok = !(st.pending && ph != 1);
        if (ph == 1) {
            if (L.state == 0) {
                ok = ok && !st.pending;
                if (L.accepted == 0) {
                    ok = ok && L.windowEnd == L.windowStart + c.offerWindow;
                } else {
                    ok = ok && L.windowStart == L.accepted && L.windowEnd == L.accepted + c.t.offer.deliveryWindow;
                }
            } else {
                ok = ok && st.pending && (L.state == 3 || L.state == 5) && c.track != 0 && L.accepted == 0;
            }
        } else if (ph == 2) {
            ok = ok && (L.state == 0 || L.state == 3) && L.accepted != 0 && L.windowStart == st.claimedAt
                && L.windowEnd == st.claimedAt + c.t.offer.confirmWindow;
        } else if (ph == 3) {
            ok = ok && L.state == 1 && L.windowStart == st.disputedAt && L.windowEnd == st.disputedAt + c.rulingWindow;
        } else if (ph == 7) {
            ok = ok && L.state == 2;
        } else if (ph == 9) {
            ok = ok && (L.state == 3 || L.state == 4);
        } else {
            ok = ok && (L.state == 3 || L.state == 5);
        }
        if (!ok) seatTableBroken++;
    }

    // ------------------------------------------------------------------
    // orders and the gate
    // ------------------------------------------------------------------

    /// A new order: the track, the parties' kinds, the consent forms, the
    /// quote, the ladder, the payouts, and whether it opens at once.
    function newOrder(uint256 flags, uint256 amt, uint256 prem, uint256 surplus) external {
        if (cases.length >= MAX_CASES + threeCount + poolCases) return;
        _newOrder(_h(flags), amt, prem, surplus, false);
    }

    /// A new order from hashed flags; `fresh`: on a guild track and opened at once (the draw's clocks start from it).
    function _newOrder(uint256 flags, uint256 amt, uint256 prem, uint256 surplus, bool fresh) internal {
        R memory r;
        uint8 tr = uint8(flags % 5);
        if (fresh && tr < 3) tr = 3 + uint8((flags >> 4) % 2);
        r.track = tr < 3 ? 0 : tr - 2;
        // round 5: the three-seat walks (1 the guild of judges, 4 the guild of five), the pair matrix (3 the named track)
        if (forceTrack != 0) r.track = forceTrack == 3 ? 0 : (forceTrack == 4 ? 3 : forceTrack);
        r.sk = r.track == 2 ? 0 : ((flags >> 8) % 3 == 0 ? 1 : 0);
        if (forceSeller != address(0)) r.sk = forceSeller == coSeller ? 1 : 0;
        uint256 b4 = (flags >> 16) % 4;
        r.bk = r.track == 2 ? 0 : uint8(b4 < 3 ? 0 : 1);
        r.flags = flags;
        r.surplus = (flags >> 120) & 1 == 0 ? 0 : bound(surplus, 1, 1e24);
        r.openNow = fresh || forceOpen || (flags >> 60) % 4 != 0;
        if (forceShut) r.openNow = false;
        uint256 a = bound(amt, 1, 1e27);
        Q memory q = defaultQuote(address(tok), address(0), (flags >> 24) & 1 == 1);
        q.premiumBps = uint16(bound(prem, 0, 10000));
        // s3, s5: the share a lapse gives the buyer, 0, 5000 or 10000 bps of A
        q.lapseDefault = uint8((flags >> 25) % 3);
        // round 3: one guild quote in five has an offer window below a block (s3, s7: the block after the arming
        // still draws; an offer that short is accepted only in its own block)
        if (r.track != 0 && (flags >> 96) % 5 == 0) {
            uint32[5] memory tiny = [uint32(0), 1, 11, 12, 13];
            q.offerWindow = tiny[(flags >> 100) % 5];
        }
        if ((flags >> 80) % 5 == 0) {
            q.feeBps = 0;
            q.feeMin = 0;
            q.feeMax = 0;
        } else if ((flags >> 164) % 4 == 0) {
            // round 7: an odd LF (the bounds equal), so that the ruled rung's feeBearer 2 sends floor(LF / 2) to the
            // buyer and the odd unit back to sellerFund (s8's half rounded down); the default bounds give an even LF
            q.feeMin = 5 ether + 1;
            q.feeMax = 5 ether + 1;
        }
        // round 2: one order in eight at the guard-6 bound with A x bps at its largest; one in six of the
        // others priced in a reference unit through two feeds
        uint256 r2 = _h(flags ^ 0x5232);
        bool huge = r2 % 8 == 0;
        Q memory qm = _copyQ(q); // the quote's absolutes as the settlement token sees them
        if (huge) {
            a = type(uint256).max / 10000 - (amt % 1000);
            q.feeBps = uint16((r2 >> 8) % 10001);
            q.feeMin = 5 ether;
            q.feeMax = (r2 >> 24) % 2 == 0 ? 0 : 500 ether;
            qm = _copyQ(q);
        } else if ((r2 >> 32) % 6 == 0) {
            q.feeUnit = address(usd);
            q.feedToken = address(feedTok);
            q.feedUnit = address(feedUsd);
            q.maxStale = type(uint32).max;
            q.feeFixed = (r2 >> 40) % 2 == 0 ? 0 : 1e6;
            q.feeMin = 5e6;
            q.feeMax = 500e6;
            q.amountCap = (r2 >> 48) % 3 == 0 ? 1e30 : 0;
            // x' = ceil(x x 1e8 x 10^(8 + 18) / (2000e8 x 10^(8 + 6))) = x x 5e8, exactly
            qm = _copyQ(q);
            qm.feeFixed = q.feeFixed * 5e8;
            qm.feeMin = q.feeMin * 5e8;
            qm.feeMax = q.feeMax * 5e8;
            qm.amountCap = q.amountCap * 5e8;
            r.unit = true;
            r.capInToken = qm.amountCap;
        }
        // round 6: one fee-free order in two also free of P, so that D = P + lf is zero (s4 step 7, s7a); never a forced
        // order (the matrix, the pairs, the three-seat walks keep their P, so that the judge's total stays measured)
        if (!huge && !r.unit && !(fresh || forceOpen || forceTrack != 0) && (flags >> 80) % 5 == 0 && (flags >> 152) % 2 == 0) {
            q.premiumBps = 0;
            qm.premiumBps = 0;
        }
        // round 2: one guild order in six under an offer window shorter than a block (0 or 11 s, consent
        // carried), where only the block after the arming can draw
        if (r.track != 0 && (r2 >> 100) % 6 == 0) {
            q.offerWindow = (r2 >> 104) % 2 == 0 ? 0 : 11;
            q.autoAccept = true;
        }
        if (forceManual) {
            // round 5: consent never carried, a full offer window, so that declines and expiries are reachable
            q.offerWindow = 3 days;
            q.autoAccept = false;
        }
        address judge = r.track == 0 ? namedJudges[(flags >> 56) % 3] : address(0);
        if (forceJudge != address(0) && r.track == 0) judge = forceJudge;
        // one named judge in two registers its payout on the instance right after the opening (no quote carries one)
        r.payNow = judge != address(0) && (flags >> 58) & 1 == 1;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(judge, AMANAH, quote, a);
        if (r.track != 0) t.offer.guild = r.track == 1 ? GUILD : (r.track == 2 ? GUILD2 : GUILD3);
        t.offer.sellerKey = r.sk == 1 ? coSeller : SELLER;
        t.offer.sellerFund = t.offer.sellerKey;
        t.buyerKey = r.bk == 0 ? BUYER : coBuyer;
        t.offer.ladder = _ladder(uint8((flags >> 88) % 5));
        t.offer.termsHash = keccak256(abi.encode("terms", nonceCounter));
        t.offer.buyerMayOpen = (flags >> 26) & 1 == 1;
        uint256 al = (flags >> 48) % 4;
        if (al == 1) t.offer.sellerPayout = t.offer.sellerKey;
        if (al == 2) t.buyerPayout = t.buyerKey;
        if (al == 3) t.buyerPayout = SELLER_PAYOUT;
        uint256 ob = (flags >> 64) % 3;
        t.openBy = uint64(_now() + (ob == 0 ? 1 days : (ob == 1 ? 3 days : 10 minutes)));
        _drawFund(t, r, flags, fresh || forceOpen || forceTrack != 0);
        // round 2: the three ints at 2^53 - 1 (the order opens) or one of them past it (guard 16 refuses it)
        uint256 ints = (r2 >> 56) % 10;
        if (ints == 0) {
            t.offer.nonce = MAX_INT53;
            t.offer.expiry = MAX_INT53;
            t.openBy = uint64(MAX_INT53);
        } else if (ints == 1) {
            uint256 w = (r2 >> 60) % 3;
            if (w == 0) t.offer.nonce = MAX_INT53 + 1;
            else if (w == 1) t.offer.expiry = MAX_INT53 + 1;
            else t.openBy = uint64(MAX_INT53 + 1);
        }
        r.d = depositOf(qm, a);
        r.lf = lfOf(qm, a);
        r.p = premiumOf(qm, a);
        r.lapse = q.lapseDefault;
        r.offerWindow = q.offerWindow;
        r.rulingWindow = q.rulingWindow;
        _register(t, quote, r);
    }

    /// Round 6 (s7a): the order's sellerFund from five: 0 the seller's key; 1 a funder, an EOA or a contract, that
    /// funds both sellers; 2 the seller's payout; 3 the buyer's payout (the buyer's key or the seller's payout where
    /// the order makes them so); 4 an address that is not bound to the seller: a stranger holding money and an
    /// allowance, the logic, the verifier or another case's instance (each with an allowance pranked on it, so that
    /// only the bond's test stands between the order and the money). Draws 1 to 3 bind before the order three
    /// times in four and never otherwise, so that FundUnbound is reached; a forced order (the matrix, the pairs, the
    /// three-seat walks) draws a bound fund.
    function _drawFund(Eravon.OrderTerms memory t, R memory r, uint256 flags, bool forced) internal view {
        uint8 fk = uint8((flags >> 140) % 5);
        if (forced && fk == 4) fk = uint8((flags >> 156) % 4);
        address f = t.offer.sellerKey;
        if (fk == 1) f = (flags >> 144) % 2 == 0 ? FUNDER : cfunder;
        else if (fk == 2) f = t.offer.sellerPayout;
        else if (fk == 3) f = t.buyerPayout;
        else if (fk == 4) {
            uint256 w = (flags >> 144) % 5;
            if (w == 2) f = address(logic);
            else if (w == 3) f = address(verifier);
            else if (w == 4 && cases.length != 0) f = address(cases[(flags >> 160) % cases.length].inst);
            else f = UNBOUND;
        }
        if (forceFund != address(0)) {
            f = forceFund;
            fk = isPoolFund[forceFund] ? 5 : 4;
        }
        t.offer.sellerFund = f;
        r.fundKind = fk;
        r.bindNow = forceFund == address(0) && fk >= 1 && fk <= 3 && (forced || (flags >> 148) % 4 != 0);
    }

    /// Round 6: `fund` binds itself to `seller` on the logic by its own call (an EOA's, a contract's through its own
    /// code): FundBound(seller, fund) from the logic, both topics, no data; funds() answers; the ghost follows.
    function _bindFund(address fund, address sel_) internal {
        vm.recordLogs();
        _doAs(fund, address(logic), abi.encodeCall(Eravon.acceptFunding, (sel_)));
        VmSafe.Log[] memory logs = vm.getRecordedLogs();
        uint256 n;
        for (uint256 k = 0; k < logs.length; k++) {
            if (logs[k].topics.length == 0 || logs[k].topics[0] != Eravon.FundBound.selector) continue;
            n++;
            if (
                logs[k].emitter != address(logic) || logs[k].topics.length != 3 || logs[k].data.length != 0
                    || address(uint160(uint256(logs[k].topics[1]))) != sel_ || address(uint160(uint256(logs[k].topics[2]))) != fund
            ) bondWrong++;
        }
        if (n != 1 || !logic.funds(sel_, fund)) bondWrong++;
        if (boundG[sel_][fund]) _mark("fund.bindAgain");
        boundG[sel_][fund] = true;
        _mark(fund.code.length != 0 ? "fund.bindByCode" : "fund.bind");
        if (boundG[SELLER][FUNDER] && boundG[coSeller][FUNDER]) _mark("fund.oneFunderTwoSellers");
    }

    /// Round 6 (s4 step 5, s7a): what the fund lets init do: 0 the pull, 1 FundUnbound (neither the key nor bound,
    /// whatever D), 2 TransferFailed (bound, D nonzero, the fund's allowance or balance short of D).
    function _fundFault(C storage c) internal view returns (uint8) {
        address sk = c.t.offer.sellerKey;
        address f = c.t.offer.sellerFund;
        if (f != sk && !boundG[sk][f]) return 1;
        if (c.d != 0 && (tok.allowance(f, address(logic)) < c.d || tok.balanceOf(f) < c.d)) return 2;
        return 0;
    }

    /// Round 6: the opening pull took exactly D from sellerFund's balance and allowance and nothing from the key where
    /// the key is not the fund; the parties name the fund third.
    function _checkFundPull(uint256 i) internal {
        C storage c = cases[i];
        address f = c.t.offer.sellerFund;
        address sk = c.t.offer.sellerKey;
        bool bad = tok.balanceOf(f) + c.d != snap.balP[2];
        if (snap.allowF != type(uint256).max && tok.allowance(f, address(logic)) + c.d != snap.allowF) bad = true;
        if (sk != f && tok.balanceOf(sk) != snap.balK) bad = true;
        if (tok.balanceOf(address(c.inst)) != snap.bal + c.d) bad = true;
        if (_seatCount(i) != 0 && c.inst.seatOf(c.id, 0).parties[2] != f) bad = true;
        if (bad) {
            fundPullWrong++;
            _r5(abi.encode("fundPull", f, c.d, snap.balP[2], tok.balanceOf(f)));
        }
        string[6] memory kinds = ["fund.openedKey", "fund.openedFunder", "fund.openedSellerPayout", "fund.openedBuyerPayout", "fund.openedStrangerBoundLater", "fund.openedPool"];
        _mark(kinds[c.fundKind]);
        // round 7: the opening pull draws D from the pool's ghosts; the ghost allowing less is a verdict off the model
        if (isPoolFund[f] && c.d != 0) {
            if (allowG[f] != type(uint256).max) {
                if (allowG[f] < c.d) {
                    poolWrong++;
                    _r5(abi.encode("poolPullOverTheAllowance", i, allowG[f], c.d));
                    allowG[f] = 0;
                } else {
                    allowG[f] -= c.d;
                }
            }
            balG[f] -= int256(c.d);
        }
        if (f == cfunder) _mark("fund.openedContractFunder");
        if (c.d == 0) _mark(f == sk ? "fund.zeroDepositOpenedKey" : "fund.zeroDepositOpenedBound");
    }

    /// Round 6: a live init the fund model refuses answers exactly FundUnbound or TransferFailed and moves nothing.
    struct FG {
        uint256 fb;
        uint256 fa;
        uint256 ib;
        uint256 kb;
    }

    function _fundsAt(C storage c) internal view returns (FG memory g) {
        address f = c.t.offer.sellerFund;
        g.fb = tok.balanceOf(f);
        g.fa = tok.allowance(f, address(logic));
        g.ib = tok.balanceOf(address(c.inst));
        g.kb = tok.balanceOf(c.t.offer.sellerKey);
    }

    function _initRefusedByFund(uint256 i, uint8 ff, address caller, bytes memory data) internal returns (bool) {
        C storage c = cases[i];
        address f = c.t.offer.sellerFund;
        FG memory g0 = _fundsAt(c);
        (bool ok, bytes memory ret) = _try(caller, address(c.inst), data);
        FG memory g1 = _fundsAt(c);
        bytes4 want = ff == 1 ? Eravon.FundUnbound.selector : Eravon.TransferFailed.selector;
        if (ok || ret.length != 4 || bytes4(ret) != want || keccak256(abi.encode(g0)) != keccak256(abi.encode(g1)) || _ph(i) != 0) {
            fundGateWrong++;
            _r5(abi.encode("fundGate", ff, ok, ret, f));
        }
        if (ff == 1) {
            _mark(c.d == 0 ? "fund.zeroDepositUnboundRefused" : "fund.unboundRefused");
            string[6] memory kinds = ["", "fund.unboundFunderRefused", "fund.unboundSellerPayoutRefused", "fund.unboundBuyerPayoutRefused", "fund.strangerRefused", "fund.poolUnboundRefused"];
            if (c.fundKind != 0) _mark(kinds[c.fundKind]);
            if (f == address(logic) || f == address(verifier)) _mark("fund.logicOrVerifierRefused");
            if (f.codehash == logic.PROXY_CODEHASH()) _mark("fund.anInstanceRefused");
        } else {
            _mark("fund.shortRefused");
        }
        return false;
    }

    function _copyQ(Q memory q) internal pure returns (Q memory c) {
        c.token = q.token;
        c.premiumBps = q.premiumBps;
        c.feeUnit = q.feeUnit;
        c.feedToken = q.feedToken;
        c.feedUnit = q.feedUnit;
        c.maxStale = q.maxStale;
        c.feeFixed = q.feeFixed;
        c.feeBps = q.feeBps;
        c.feeMin = q.feeMin;
        c.feeMax = q.feeMax;
        c.amountCap = q.amountCap;
        c.rulingWindow = q.rulingWindow;
        c.offerWindow = q.offerWindow;
        c.lapseDefault = q.lapseDefault;
        c.autoAccept = q.autoAccept;
    }

    /// One offer, another order: the seller's signature or registered consent serves every buyer (s9).
    function newSibling(uint256 seed, uint256 flags) external {
        if (cases.length >= MAX_CASES + threeCount + poolCases) return;
        (bool ok, uint256 k) = _pickAny(seed);
        if (!ok) return;
        C storage o = cases[k];
        flags = _h(flags);
        Eravon.OrderTerms memory t = o.t;
        R memory r;
        r.track = o.track;
        r.sk = o.sellerKind;
        r.bk = o.track == 2 ? 0 : uint8(flags % 2);
        r.flags = flags;
        r.openNow = (flags >> 60) % 4 != 0;
        r.payNow = o.payNow;
        t.buyerKey = r.bk == 0 ? BUYER : coBuyer;
        t.buyerPayout = BUYER_PAYOUT;
        t.openBy = uint64(_now() + 1 days + (++nonceCounter));
        r.d = o.d;
        r.lf = o.lf;
        r.p = o.p;
        r.lapse = o.lapseDefault;
        r.offerWindow = o.offerWindow;
        r.rulingWindow = o.rulingWindow;
        r.unit = o.unit;
        r.capInToken = o.capInToken;
        r.fundKind = o.fundKind;
        _register(t, o.quote, r);
        _mark("order.sibling");
    }

    function _register(Eravon.OrderTerms memory t, bytes memory quote, R memory r) internal {
        Eravon inst = deployProxy(t);
        _mint(address(inst), t.offer.amount + r.surplus);
        // round 6: D at sellerFund (never minted to the logic, the verifier or an instance, whose allowance is pranked)
        address f = t.offer.sellerFund;
        if (isPoolFund[f]) {
            // round 7: a pool's fund is minted its D and raises its own allowance by it, both in the ghost
            _mint(f, r.d);
            balG[f] += int256(r.d);
            _poolApprove(f, allowG[f] == type(uint256).max ? type(uint256).max : allowG[f] + r.d);
        } else if (f != address(logic) && f != address(verifier) && f.codehash != logic.PROXY_CODEHASH()) {
            _mint(f, r.d);
        }
        if (isPoolFund[f]) {
            // approved above
        } else if (f.codehash == logic.PROXY_CODEHASH() || f == address(logic) || f == address(verifier)) {
            vm.prank(f);
            tok.approve(address(logic), tok.allowance(f, address(logic)) + r.d);
        } else {
            _approve(f, address(logic), tok.allowance(f, address(logic)) + r.d);
        }
        if (r.bindNow && f != t.offer.sellerKey && !boundG[t.offer.sellerKey][f]) _bindFund(f, t.offer.sellerKey);
        C storage c = cases.push();
        c.inst = inst;
        c.t = t;
        c.quote = quote;
        c.id = orderHash(t);
        c.track = r.track;
        c.sellerKind = r.sk;
        c.buyerKind = r.bk;
        c.a = t.offer.amount;
        c.d = r.d;
        c.lf = r.lf;
        c.p = r.p;
        c.lapseDefault = r.lapse;
        c.offerWindow = r.offerWindow;
        c.rulingWindow = r.rulingWindow;
        c.unit = r.unit;
        c.capInToken = r.capInToken;
        c.fundKind = r.fundKind;
        c.payNow = r.payNow;
        c.badInts = t.offer.nonce > MAX_INT53 || t.offer.expiry > MAX_INT53 || t.openBy > MAX_INT53;
        _forms(c, r.flags);
        uint256 i = cases.length - 1;
        // s9: a consent by registry is recorded under the party, sent by the party's current key
        if (c.sellerForm == 2) _consent(t.offer.sellerKey, digestOf(address(logic), offerHash(t)));
        if (c.buyerForm == 2) _consent(t.buyerKey, digestOf(address(inst), c.id));
        if (r.openNow) _init(i);
    }

    /// s9: `consent(party, digest)` on the logic from the party's current key; a stranger's or a former key's
    /// for the same party is NotCurrent (probed one time in four), and the registry answers for the party.
    function _consent(address party, bytes32 d) internal {
        if (uint256(keccak256(abi.encode(d, "probe"))) % 4 == 0) {
            (bool ok, bytes memory ret) = _tryAs(STRANGER, address(logic), abi.encodeCall(Eravon.consent, (party, d)));
            if (ok || ret.length != 4 || bytes4(ret) != Eravon.NotCurrent.selector) {
                oldKeyAdmitted++;
                _r5(abi.encode("consentByAStranger", party, ok, ret));
            }
            _mark("consent.strangerRefused");
        }
        _do(party, address(logic), abi.encodeCall(Eravon.consent, (party, d)));
        if (!logic.consented(party, d)) {
            oldKeyAdmitted++;
            _r5(abi.encode("consentNotRecorded", party));
        }
    }

    /// The consent forms at init and the opener, as the parties' kinds and buyerMayOpen admit them.
    function _forms(C storage c, uint256 flags) internal {
        uint8 opener = uint8((flags >> 28) % 3);
        if (!c.t.offer.buyerMayOpen && opener == 1) opener = 0;
        uint8 sf = uint8((flags >> 32) % 3);
        uint8 bf = uint8((flags >> 40) % 3);
        if (c.sellerKind == 1 && sf == 0) sf = 2;
        if (c.buyerKind == 1 && bf == 0) bf = 2;
        if (sf == 1 && opener == 1) sf = c.sellerKind == 1 ? 2 : 0;
        if (bf == 1 && opener != 1) bf = c.buyerKind == 1 ? 2 : 0;
        c.opener = opener;
        c.sellerForm = sf;
        c.buyerForm = bf;
    }

    /// The init call: its consents by the parties' current keys, and its opener (the seller's record, or the buyer's
    /// where the opener is the buyer; a stranger's attempt is made apart, in `_init`).
    function _initCall(uint256 i) internal view returns (address caller, bytes memory data) {
        C storage c = cases[i];
        Eravon.OrderTerms memory t = c.t;
        bytes memory sigS = c.sellerForm == 0 ? _sellerSig(t) : bytes("");
        bytes memory sigB = c.buyerForm == 0 ? _buyerSig(t) : bytes("");
        caller = c.opener == 1 ? t.buyerKey : t.offer.sellerKey;
        address[] memory m;
        bytes32[] memory w;
        if (c.track == 0) {
            m = new address[](0);
            w = new bytes32[](0);
        } else {
            (m, w) = _panel(c.track, c.track == 1 ? guildPanel : 0);
        }
        data = abi.encodeCall(Eravon.init, (t, c.quote, sigB, sigS, m, w));
    }

    /// One observed call to the case, its pushes metered. Where the standard owes it success (`must`) a revert is
    /// a liveness violation, recorded with its bytes; a revert without one of the standard's names is recorded and
    /// the handler's call returns, so that the record survives; elsewhere the handler's call reverts with the same
    /// bytes.
    function _exec(uint256 i, uint8 act, bool must, address who, bool judge, bytes memory data) internal returns (bool) {
        _before(i);
        _readView(i, act, data);
        btok.resetPushGas();
        lastClosedSeen = false;
        bool ok;
        bytes memory ret;
        if (judge) {
            // venue/1 s2.7, s2.9: the judge's direct call from its record's current key
            address k = _cur(who);
            vm.prank(k, k);
            (ok, ret) = address(cases[i].inst).call{gas: TX_GAS}(data);
        } else {
            (ok, ret) = _try(who, address(cases[i].inst), data);
        }
        _meterPush(act);
        if (!ok) {
            bool unnamed = _unnamed(act, ret);
            if (must) {
                livenessBroken++;
                lastLivenessAct = act;
                lastLivenessCase = i;
                lastLivenessErr = ret;
                return false;
            }
            if (unnamed) return false;
            assembly ("memory-safe") {
                revert(add(ret, 32), mload(ret))
            }
        }
        _after(i, act);
        _sweptGhost(i, act, data);
        return true;
    }

    /// s8: a sweep of the settlement token after the terminal credits every stray to buyerPayout, whoever sends it and
    /// from whatever step of the campaign: the stray ghost follows it here, where every observed call returns.
    function _sweptGhost(uint256 i, uint8 act, bytes memory data) internal {
        if (act != A_SWEEP || data.length < 68 || _ph(i) < 4) return;
        address tk;
        assembly ("memory-safe") {
            tk := mload(add(data, 68))
        }
        if (tk == address(tok)) cases[i].stray = 0;
    }

    /// Every fault of the instance is one of the standard's names: a panic, an empty revert (an out-of-gas under
    /// the transaction cap among them), a string or any other bytes are recorded.
    function _unnamed(uint8 act, bytes memory ret) internal returns (bool) {
        if (_isNamed(ret)) return false;
        unnamedRevert++;
        lastUnnamedAct = act;
        lastUnnamed = ret;
        return true;
    }

    /// Whether the order is past its gate: the expiry branch is due.
    function _dead(C storage c) internal view returns (bool) {
        return _now() > c.t.offer.expiry || _now() > c.t.openBy || revokedTerms[c.t.offer.sellerKey][c.t.offer.termsHash];
    }

    function _init(uint256 i) internal returns (bool) {
        C storage c = cases[i];
        (address caller, bytes memory data) = _initCall(i);
        bool dead = _ph(i) == 0 && _dead(c);
        // s4 step 3: the expiry branch is either party's, the buyer's whatever buyerMayOpen says
        if (dead && _h(vm.getBlockNumber() ^ uint256(c.id)) % 2 == 0) caller = c.t.buyerKey;
        // s4 steps 3 and 4, s10: a stranger opens nothing and runs no branch: NotParty on the expiry branch and under
        // buyerMayOpen, NotSeller otherwise
        if (_ph(i) == 0 && (c.opener == 2 || (dead && _h(uint256(c.id) ^ 0x57) % 2 == 0))) {
            bytes4 want = (dead || c.t.offer.buyerMayOpen) ? Eravon.NotParty.selector : Eravon.NotSeller.selector;
            (bool sok, bytes memory sret) = _tryAs(STRANGER, address(c.inst), data);
            if (sok || sret.length != 4 || bytes4(sret) != want) {
                strangerAdmitted++;
                _r5(abi.encode("strangerInit", dead, c.t.offer.buyerMayOpen, sok, sret));
            }
            _mark(dead ? "init.strangerRefusedTheBranch" : (c.t.offer.buyerMayOpen ? "init.strangerRefused" : "init.strangerRefusedNotSeller"));
        }
        if (c.badInts && _ph(i) == 0 && !_dead(c)) {
            // s4 step 5, guard 16: a live order with an int past 2^53 - 1 never opens; the branch and cancel remain
            (bool ok, bytes memory ret) = _try(caller, address(c.inst), data);
            if (ok || keccak256(ret) != keccak256(abi.encodeWithSelector(Eravon.BadTerms.selector, uint8(16)))) int53Wrong++;
            _mark("init.int53PastTheBoundRefused");
            return false;
        }
        // round 6 (s4 step 5, s7a): a live order whose fund is not bound to the seller is FundUnbound, and one whose
        // bound fund is short of D is TransferFailed; nothing moves and the order stays for the branch or a later init
        if (_ph(i) == 0 && !_dead(c)) {
            uint8 ff = _fundFault(c);
            if (ff != 0) return _initRefusedByFund(i, ff, caller, data);
        }
        // the record the guild's book serves today is the one this init freezes
        if (c.track == 1 && _ph(i) == 0) c.panel = guildPanel;
        // s4: an unopened, live, funded order with its consents opens; a dead one runs the expiry branch,
        // which never reverts on the token's account (the push is best effort, the credit stays)
        bool must = _ph(i) == 0;
        bool done = _exec(i, A_INIT, must, caller, false, data);
        if (done && must && _ph(i) == 1 && c.track == 0 && c.payNow) {
            // venue/1 s3: the named judge registers its payout on the instance from its current key
            address j = c.t.offer.judge;
            if (_exec(i, A_MONEY, true, j, false, abi.encodeCall(Eravon.setPayout, (j, payoutOfJudge[j])))) {
                _mark("payout.setAfterInit");
            }
        }
        if (done) _drawSoon(i);
        return done;
    }

    /// A keeper's draw from the next block, as a client makes it, for three armings in four; the fourth is
    /// left to whoever comes, or to the deadline. Where the offer window is shorter than a block the keeper's
    /// block comes a block's time (12 s) later, past the deadline, and draws as the block after the arming
    /// always may; past the deadline any later block is refused, so the keeper does not call. Observed as its
    /// own action.
    function _drawSoon(uint256 i) internal {
        if (noKeeper || _ph(i) != 1) return;
        C storage c = cases[i];
        S memory st = _st(c.inst);
        if (!st.pending) return;
        // round 4: a re-armed draw (after an unaccepted seat) is left to whoever comes one time in two, so that its
        // deadline is run too
        uint256 kh = uint256(keccak256(abi.encode(c.id, _seatCount(i), "keeper")));
        if (kh % 4 == 0 || (_seatCount(i) != 0 && (kh >> 8) % 2 == 0)) return;
        uint256 nextT = _now() + (c.offerWindow < 12 ? 12 : 0);
        bool late = nextT > st.redrawDeadline;
        if (late && vm.getBlockNumber() != st.drawBlock) return;
        (address[] memory m, bytes32[] memory w) = _frozen(c);
        vm.roll(vm.getBlockNumber() + 1);
        if (nextT != _now()) vm.warp(nextT);
        // s7: the draw is anyone's; the handler sends it from a party, the guild or a stranger
        uint256 dw = (kh >> 16) % 4;
        if (_exec(i, A_REDRAW, true, _drawer(c, dw), false, abi.encodeCall(Eravon.redraw, (m, w)))) {
            _mark(late ? "redraw.firstBlockPastDeadline" : "redraw.byKeeper");
            _mark(dw == 0 ? "redraw.bySeller" : (dw == 1 ? "redraw.byBuyer" : (dw == 2 ? "redraw.byGuild" : "redraw.byStranger")));
        }
    }

    function doInit(uint256 seed) external {
        (bool ok, uint256 i) = _pickWhere(seed, _isNone);
        if (!ok) return;
        if (_ph(i) == 0 && _dead(cases[i])) _mark("init.deadOrder");
        _init(i);
    }

    /// A second init and a cancel on a case that exists, both fully formed: AlreadyInitialized, nothing else.
    function doSecondInit(uint256 seed) external {
        (bool ok, uint256 i) = _pickWhere(seed, _isSet);
        if (!ok || _ph(i) == 0) return; // an unopened order is doInit's and doRace's, observed there
        C storage c = cases[i];
        (address caller, bytes memory data) = _initCall(i);
        (bool s1, bytes memory r1) = _try(caller, address(c.inst), data);
        if (s1 || bytes4(r1) != Eravon.AlreadyInitialized.selector) secondInitPassed++;
        (bool s2, bytes memory r2) = _tryCancel(i);
        if (s2 || bytes4(r2) != Eravon.AlreadyInitialized.selector) secondInitPassed++;
        _mark("secondInit.refused");
    }

    /// s4: the direct form from the buyer's current key, or the signed form, a signature by that key over Cancel(),
    /// sent by either party's current key.
    function _cancelCall(uint256 i, bool bySig) internal view returns (address from, bytes memory data) {
        C storage c = cases[i];
        Eravon.OrderTerms memory t = c.t;
        bytes32 d = digestOf(address(c.inst), keccak256(abi.encode(TH_CANCEL)));
        if (bySig && c.buyerKind == 0) {
            return (_submitter(c, uint256(d) ^ vm.getBlockNumber()), abi.encodeCall(Eravon.cancel, (t, _signB(d))));
        }
        return (t.buyerKey, abi.encodeCall(Eravon.cancel, (t, bytes(""))));
    }

    function _tryCancel(uint256 i) internal returns (bool, bytes memory) {
        (address from, bytes memory data) = _cancelCall(i, false);
        return _try(from, address(cases[i].inst), data);
    }

    function _cancel(uint256 i, bool bySig) internal returns (bool done) {
        C storage c = cases[i];
        (address from, bytes memory data) = _cancelCall(i, bySig);
        // s4, s10: the signed form sent by a stranger is refused NotParty, before the signature
        if (bySig && c.buyerKind == 0 && _ph(i) == 0) _strangerRefused(address(c.inst), data, "cancelSigRefused");
        // s4: the buyer's cancel before any case always lands; the push is best effort, the credit stays
        done = _exec(i, A_CANCEL, _ph(i) == 0, from, false, data);
        if (done) _mark(c.buyerKind == 1 ? "cancel.company" : (bySig ? "cancel.signedSentByAParty" : "cancel.key"));
    }

    function doCancel(uint256 seed, uint256 bySig) external {
        (bool ok, uint256 i) = _pickWhere(seed, _isNone);
        if (!ok) return;
        _cancel(i, _h(bySig) % 2 == 0);
    }

    /// cancel and init racing in one block on an unopened order: whichever lands first decides (s4).
    function doRace(uint256 seed, uint256 order) external {
        (bool ok, uint256 i) = _pickWhere(seed, _isNone);
        if (!ok || _ph(i) != 0) return;
        C storage c = cases[i];
        if (_h(order) % 2 == 0) {
            if (!_cancel(i, false)) return;
            (address caller, bytes memory data) = _initCall(i);
            (bool s, bytes memory r) = _try(caller, address(c.inst), data);
            if (s || bytes4(r) != Eravon.AlreadyInitialized.selector) raceBroken++;
            _mark("race.cancelFirst");
        } else {
            if (!_init(i) || _ph(i) == 0) return;
            (bool s, bytes memory r) = _tryCancel(i);
            if (s || bytes4(r) != Eravon.AlreadyInitialized.selector) raceBroken++;
            _mark("race.initFirst");
        }
    }

    /// The seller revokes the order's document on the logic, before or after its case opened.
    function doRevoke(uint256 seed) external {
        (bool ok, uint256 i) = _pickAny(seed);
        if (!ok) return;
        C storage c = cases[i];
        address sk = c.t.offer.sellerKey;
        bytes memory data = abi.encodeCall(Eravon.revoke, (sk, c.t.offer.termsHash));
        // s4, s9: only the seller's current key revokes under its name; a stranger's call is NotCurrent and records nothing
        (bool sok, bytes memory sret) = _tryAs(STRANGER, address(logic), data);
        if (sok || sret.length != 4 || bytes4(sret) != Eravon.NotCurrent.selector) {
            oldKeyAdmitted++;
            _r5(abi.encode("revokeByAStranger", sok, sret));
        }
        if (logic.revoked(sk, c.t.offer.termsHash) != revokedTerms[sk][c.t.offer.termsHash]) oldKeyAdmitted++;
        _before(i);
        _do(sk, address(logic), data);
        revokedTerms[sk][c.t.offer.termsHash] = true;
        if (!logic.revoked(sk, c.t.offer.termsHash)) revokeTouchedOpen++;
        _after(i, A_REVOKE);
    }

    /// The guild engages a new record from its current key: a frozen panel must serve its case to the end (s7); the
    /// record serves from the block after its own, so that in its own block the verifier's read still finds the
    /// record before it.
    function doReengage(uint256 seed) external {
        uint8 rec = uint8(_h(seed) % 2);
        (address[] memory m, bytes32[] memory w) = _panel(1, rec);
        _engage(GUILD, m, w);
        if (rec != guildPanel) {
            // in the record's own block the latest record recorded before it still answers (s7): the new panel is
            // PanelMismatch, the old one passes
            (address[] memory m0, bytes32[] memory w0) = _panel(1, guildPanel);
            bytes32[3] memory hs = [logic.PROXY_CODEHASH(), logic.panelInitCodeHash(), address(pbook).codehash];
            (bool okNew, bytes memory rNew) = address(verifier).staticcall(
                abi.encodeCall(EravonVerifier.panel, (GUILD, hs, m, w, AMANAH, SELLER, BUYER))
            );
            (bool okOld,) = address(verifier).staticcall(abi.encodeCall(EravonVerifier.panel, (GUILD, hs, m0, w0, AMANAH, SELLER, BUYER)));
            if (okNew || rNew.length != 4 || bytes4(rNew) != Eravon.PanelMismatch.selector || !okOld) {
                panelFreezeWrong++;
                _r5(abi.encode("sameBlockRecord", okNew, rNew, okOld));
            }
            _mark("guild.recordNotFrozenInItsOwnBlock");
        }
        vm.roll(vm.getBlockNumber() + 1);
        guildPanel = rec;
        _mark("guild.reengaged");
    }

    /// s7: `engage` on the chain's book from the guild record's current key, filed under the record's root; a former
    /// key of the record is the book's NotCurrent.
    function _engage(address g, address[] memory m, bytes32[] memory w) internal {
        bytes32 T = logic.PROXY_CODEHASH();
        uint64 exp = uint64(_now() + 3650 days);
        bytes memory data = abi.encodeCall(EravonPanelBook.engage, (T, m, w, ROSTER, exp, address(0)));
        for (uint256 k = 0; k < oldKeys.length; k++) {
            if (rootOfOld[oldKeys[k]] != g) continue;
            (bool ok, bytes memory ret) = _tryAs(oldKeys[k], address(pbook), data);
            if (ok || ret.length != 4 || bytes4(ret) != EravonPanelBook.NotCurrent.selector) {
                oldKeyAdmitted++;
                _r5(abi.encode("engageByAFormerKey", ok, ret));
            }
            _mark("move.formerGuildKeyRefused");
        }
        uint256 n0 = pbook.count(g, T);
        _do(g, address(pbook), data);
        if (pbook.count(g, T) != n0 + 1) {
            oldKeyAdmitted++;
            _r5(abi.encode("engageNotFiledUnderTheRoot", g));
        }
    }

    // ------------------------------------------------------------------
    // the seat
    // ------------------------------------------------------------------

    function _lastOrStranger(uint256 i) internal view returns (uint64 s, address j) {
        uint256 n = _ph(i) == 0 ? 0 : _seatCount(i);
        s = n == 0 ? 0 : uint64(n - 1);
        j = n == 0 ? STRANGER : cases[i].inst.seatOf(cases[i].id, s).judge;
    }

    function doAccept(uint256 seed, uint256 wrong) external {
        (bool ok, uint256 i) = _pickWhere(seed, _canAccept);
        if (!ok) return;
        C storage c = cases[i];
        bool guided = _canAccept(i);
        (uint64 s, address j) = _lastOrStranger(i);
        uint256 w = _h(wrong) % 16;
        if (w == 0) j = STRANGER;
        else if (w == 1) s = s + 1;
        if (_judgeCall(i, A_ACCEPT, guided && w >= 2, j, abi.encodeCall(Eravon.accept, (c.id, s)))) _mark("accept");
    }

    function doRule(uint256 seed, uint256 rung, uint256 wrong) external {
        (bool ok, uint256 i) = _pickWhere(seed, _canRule);
        if (!ok) return;
        C storage c = cases[i];
        bool guided = _canRule(i);
        (uint64 s, address j) = _lastOrStranger(i);
        uint256 rungs = c.t.offer.ladder.length;
        uint64 rg = uint64(bound(rung, 0, rungs - 1));
        uint256 w = _h(wrong) % 16;
        if (w == 0) j = STRANGER;
        else if (w == 1) rg = uint64(rungs);
        if (_judgeCall(i, A_RULE, guided && w >= 2, j, abi.encodeCall(Eravon.rule, (c.id, s, E, rg, ENTRY, "")))) _mark("rule");
    }

    function doResign(uint256 seed, uint256 wrong) external {
        bool ok;
        uint256 i;
        if ((_h(wrong) >> 8) % 2 == 0) (ok, i) = _pickWhere(seed, _canResignDisputed);
        if (!ok) (ok, i) = _pickWhere(seed, _canResign);
        if (!ok) return;
        C storage c = cases[i];
        bool guided = _canResign(i);
        (uint64 s, address j) = _lastOrStranger(i);
        bool stranger = _h(wrong) % 16 == 0;
        if (stranger) j = STRANGER;
        uint8 phB = _ph(i);
        bool decline = phB != 0 && _seatCount(i) != 0 && c.inst.seatOf(c.id, s).accepted == 0;
        if (_judgeCall(i, A_RESIGN, guided && !stranger, j, abi.encodeCall(Eravon.resign, (c.id, s, E, ENTRY)))) {
            _mark(decline ? "resign.decline" : (phB == 1 ? "resign.funded" : (phB == 2 ? "resign.claimed" : "resign.disputed")));
            _drawSoon(i);
        }
    }

    /// venue/1 s2.11: advance never reverts, due or not, any pair; a pending draw's deadline runs from any seat index.
    function doAdvance(uint256 seed, uint256 seatSeed) external {
        (bool ok, uint256 i) = _pickWhere(seed, _due);
        if (!ok) {
            (ok, i) = _pickAny(seed);
            if (!ok) return;
        }
        C storage c = cases[i];
        uint256 n = _ph(i) == 0 ? 0 : _seatCount(i);
        uint256 hs = _h(seatSeed);
        uint64 s = hs % 4 == 0 ? uint64((hs >> 8) % (n + 2)) : (n == 0 ? 0 : uint64(n - 1));
        advSeat = s;
        if (!_exec(i, A_ADVANCE, true, STRANGER, false, abi.encodeCall(Eravon.advance, (c.id, s)))) return;
        if (snap.pending && _ph(i) == 8) _mark(s >= n ? "advance.drawDeadlineFromAnUnopenedSeat" : "advance.drawDeadline");
        _drawSoon(i);
    }

    /// The draw (s7): from the block after the arming, by anyone with the frozen panel (the handler sends from a
    /// party, the guild or a stranger); one call in eight lands 300 blocks on, where the armed block's hash is gone
    /// and the call closes the case as Resigned form 3; past the deadline the block after the arming still draws,
    /// and from the second block on the call is refused; a panel the case did not freeze is refused (the same
    /// block's refusal is checked after every action that arms a draw), whoever sends it.
    function doRedraw(uint256 seed, uint256 mode) external {
        uint256 m = _h(mode) % 8;
        (bool ok, uint256 i) = m == 0 ? _pickWhere(seed, _pendingLate) : _pickWhere(seed, _pendingLive);
        if (m == 1 && (!ok || !_pendingLive(i))) (ok, i) = _freshArmed(seed);
        if (!ok) return;
        C storage c = cases[i];
        address who = _drawer(c, _h(mode) >> 8);
        (address[] memory mem, bytes32[] memory w) = _frozen(c);
        if (m == 1) (mem, w) = c.track == 2 ? _panel(1, 0) : _panel(1, 1 - c.panel); // a panel the case did not freeze
        if (m == 0 && _pendingLate(i) && vm.getBlockNumber() <= uint256(_st(c.inst).drawBlock) + 1) {
            // past the deadline, the block after the arming draws whatever the clock (the arming block waits for it)
            uint256 db = _st(c.inst).drawBlock;
            if (vm.getBlockNumber() == db) vm.roll(db + 1);
            if (_exec(i, A_REDRAW, true, who, false, abi.encodeCall(Eravon.redraw, (mem, w)))) {
                _mark("redraw.firstBlockPastDeadline");
            }
            return;
        }
        if ((m == 0 && _pendingLate(i)) || (m == 1 && _pendingLive(i))) {
            bytes4 want = m == 0 ? Eravon.NotPendingRedraw.selector : Eravon.PanelMismatch.selector;
            (bool rok, bytes memory e) = _try(who, address(c.inst), abi.encodeCall(Eravon.redraw, (mem, w)));
            if (rok || e.length != 4 || bytes4(e) != want) refusedRedrawPassed++;
            _mark(m == 0 ? "redraw.lateRefused" : "redraw.wrongPanelRefused");
            return;
        }
        vm.roll(vm.getBlockNumber() + (m == 2 ? 300 : 1));
        if (_exec(i, A_REDRAW, _pendingLive(i), who, false, abi.encodeCall(Eravon.redraw, (mem, w)))) _mark("redraw");
    }

    function doCommit(uint256 seed, uint256 m) external {
        (bool ok, uint256 i) = _pickWhere(seed, _isOpen);
        if (!ok) return;
        C storage c = cases[i];
        _exec(i, A_COMMIT, _isOpen(i), STRANGER, false, abi.encodeCall(Eravon.commit, (c.id, bytes32(m))));
    }

    // ------------------------------------------------------------------
    // the parties
    // ------------------------------------------------------------------

    function doClaim(uint256 seed, uint256 m, uint256 bySig) external {
        (bool ok, uint256 i) = _pickWhere(seed, _canClaim);
        if (!ok) return;
        C storage c = cases[i];
        bool guided = _canClaim(i);
        bytes32 man = bytes32(m);
        if (man == bytes32(0) && _h(seed) % 16 != 0) man = keccak256(abi.encode(seed, "manifest"));
        bytes memory data;
        address who;
        if (_h(bySig) % 2 == 0 && c.sellerKind == 0) {
            data = abi.encodeCall(
                Eravon.claimDeliveryBySig, (man, _signS(digestOf(address(c.inst), keccak256(abi.encode(TH_CLAIM, man)))))
            );
            // s9: sent by either party's current key; a stranger is NotParty, before the manifest and the signature
            who = _submitter(c, _h(bySig) >> 8);
            _strangerRefused(address(c.inst), data, "signedFormRefused");
        } else {
            data = abi.encodeCall(Eravon.claimDelivery, (man));
            who = c.t.offer.sellerKey;
        }
        if (_exec(i, A_CLAIM, guided && man != bytes32(0), who, false, data)) {
            _mark(c.sellerKind == 1 ? "claim.company" : "claim.key");
        }
    }

    /// A buyer's act: a key's signature, sent by either party's current key, or the key's own call; a company's
    /// call. A stranger's submission of the signed form is NotParty (s9, s10), probed one time in two.
    function _buyerCall(C storage c, bytes32 typeHash, bool bySig, bytes memory direct, bytes4 sigSel)
        internal
        returns (address who, bytes memory data)
    {
        bytes32 d = digestOf(address(c.inst), keccak256(abi.encode(typeHash)));
        if (bySig && c.buyerKind == 0) {
            data = abi.encodeWithSelector(sigSel, _signB(d));
            uint256 hb = uint256(keccak256(abi.encode(d, vm.getBlockNumber(), vm.getBlockTimestamp())));
            if (hb % 2 == 0) _strangerRefused(address(c.inst), data, "signedFormRefused");
            return (_submitter(c, hb >> 8), data);
        }
        return (c.t.buyerKey, direct);
    }

    function doConfirm(uint256 seed, uint256 bySig) external {
        (bool ok, uint256 i) = _pickWhere(seed, _canConfirm);
        if (!ok) return;
        C storage c = cases[i];
        (address who, bytes memory data) =
            _buyerCall(c, TH_CONFIRM, _h(bySig) % 2 == 0, abi.encodeCall(Eravon.confirm, ()), Eravon.confirmBySig.selector);
        if (_exec(i, A_CONFIRM, _canConfirm(i), who, false, data)) {
            _mark(c.buyerKind == 1 ? "confirm.company" : (who == c.t.offer.sellerKey ? "confirm.signedSentByTheSeller" : "confirm.key"));
        }
    }

    function _fundBuyerKey(C storage c, uint256 v) internal {
        _mint(c.t.buyerKey, v);
        _approve(c.t.buyerKey, address(c.inst), v);
    }

    /// The dispute, the fee from the key, sent ahead exactly, in excess, split, or not at all.
    function doDispute(uint256 seed, uint256 mode, uint256 extra, uint256 bySig) external {
        (bool ok, uint256 i) = _pickWhere(seed, _canDispute);
        if (!ok) return;
        C storage c = cases[i];
        bool guided = _canDispute(i);
        uint256 hm = _h(mode);
        uint256 f = hm % 5;
        if (f == 4 && (hm >> 8) % 4 != 0) f = 1;
        bool counted = _ph(i) >= 1;
        uint256 lf = c.lf;
        if (f == 0) {
            _fundBuyerKey(c, lf);
        } else if (f == 1) {
            _mint(address(c.inst), lf);
            if (counted) c.stray += lf;
        } else if (f == 2) {
            uint256 x = bound(extra, 1, 1e24);
            _mint(address(c.inst), lf + x);
            if (counted) c.stray += lf + x;
        } else if (f == 3) {
            uint256 half = lf / 2;
            _mint(address(c.inst), half);
            if (counted) c.stray += half;
            _fundBuyerKey(c, lf - half);
        }
        // s8: the fee is available (from the key, sent ahead, or already there), or the resigned seat takes none
        bool funded = f != 4 || c.stray >= lf || (guided && _lastSeat(i).state == 3);
        (address who, bytes memory data) =
            _buyerCall(c, TH_DISPUTE, _h(bySig) % 2 == 0, abi.encodeCall(Eravon.dispute, ()), Eravon.disputeBySig.selector);
        if (_exec(i, A_DISPUTE, guided && funded, who, false, data)) {
            _mark(f == 0 ? "dispute.fromKey" : (f == 1 ? "dispute.sentAhead" : (f == 2 ? "dispute.inExcess" : (f == 3 ? "dispute.split" : "dispute.unfunded"))));
            if (c.buyerKind == 1) _mark("dispute.company");
            if (_cur(c.t.buyerKey) != c.t.buyerKey && f == 0) _mark("dispute.feeFromTheRecordedKeyAfterAMove");
        }
    }

    function doRefund(uint256 seed, uint256 bySig) external {
        // the signed form is refused throughout a dispute (WrongPhase, after the signature); probed on a disputed
        // key-seller case whenever one exists, a probe that changes nothing
        {
            (bool okd, uint256 j) = _pickWhere(seed, _isDisputedKeySeller);
            if (okd && _isDisputedKeySeller(j)) {
                C storage cd = cases[j];
                bytes memory sg = _signS(digestOf(address(cd.inst), keccak256(abi.encode(TH_REFUND))));
                bytes memory dsg = abi.encodeCall(Eravon.refundBySig, (sg));
                (bool rok, bytes memory e) = _try(_submitter(cd, seed), address(cd.inst), dsg);
                if (rok || e.length != 4 || bytes4(e) != Eravon.WrongPhase.selector) livenessBroken++;
                _strangerRefused(address(cd.inst), dsg, "signedFormRefused");
                _mark("refund.sigRefusedInDispute");
            }
        }
        if ((_h(bySig) >> 8) % 3 != 0) return; // one call in three acts, so that cases live on to the later phases
        bool ok;
        uint256 i;
        if ((_h(bySig) >> 16) % 2 == 0) (ok, i) = _pickWhere(seed, _isLate);
        if (!ok) (ok, i) = _pickWhere(seed, _isOpen);
        if (!ok) return;
        C storage c = cases[i];
        if (_ph(i) == 3 && _lastSeat(i).state == 1 && _now() <= _lastSeat(i).windowEnd) {
            // the seller's own call during a dispute within T3 is refused, nothing moving: a dispute ordered
            // ahead of a refund cannot change what the refund does (s8)
            (bool okr, bytes memory er) = _try(c.t.offer.sellerKey, address(c.inst), abi.encodeCall(Eravon.refund, ()));
            if (okr || bytes4(er) != Eravon.WrongPhase.selector) livenessBroken++;
            _mark("refund.refusedInDisputeT3");
            return;
        }
        bytes memory data;
        address who;
        bool signed_ = _h(bySig) % 2 == 0 && c.sellerKind == 0 && _ph(i) != 3;
        if (signed_) {
            // the signed form outside a dispute alone; during one it is refused whatever T3 says; sent by either party
            data = abi.encodeCall(Eravon.refundBySig, (_signS(digestOf(address(c.inst), keccak256(abi.encode(TH_REFUND))))));
            who = _submitter(c, _h(bySig) >> 24);
        } else {
            data = abi.encodeCall(Eravon.refund, ());
            who = c.t.offer.sellerKey;
        }
        refundSigned = signed_;
        if (_exec(i, A_REFUND, _isOpen(i), who, false, data)) _mark(c.sellerKind == 1 ? "refund.company" : "refund.key");
        refundSigned = false;
    }

    /// One party's slot at settle: a signature by its current key, empty by its own call, or empty by the registry
    /// (recorded under the party by its current key).
    function _settleSlot(C storage c, bool isSeller, bool isCaller, uint256 pref, bytes32 d) internal returns (bytes memory) {
        uint8 kind = isSeller ? c.sellerKind : c.buyerKind;
        address who = isSeller ? c.t.offer.sellerKey : c.t.buyerKey;
        if (isCaller && (pref == 1 || kind == 1)) {
            _mark("settle.byCall");
            return "";
        }
        if (kind == 1 || pref == 2) {
            if (!logic.consented(who, d)) _consent(who, d);
            _mark("settle.byRegistry");
            return "";
        }
        return isSeller ? _signS(d) : _signB(d);
    }

    /// s9: settle in every form is either party's current key's; a stranger with both consents in hand is NotParty
    /// (s10: before the phase, the bps, the expiry and the consents), and moves nothing.
    function doSettle(uint256 seed, uint256 bps, uint256 dt, uint256 form) external {
        uint256 hf = _h(form);
        if ((hf >> 40) % 2 == 0) return; // one call in two acts; two in three of those on a Claimed or Disputed case
        bool ok;
        uint256 i;
        if ((hf >> 48) % 3 != 0) (ok, i) = _pickWhere(seed, _isLate);
        if (!ok) (ok, i) = _pickWhere(seed, _isOpen);
        if (!ok) return;
        C storage c = cases[i];
        uint16 b = uint16(bound(bps, 0, 10000));
        if (hf % 32 == 0) b = 10001;
        uint256 exp = _now() + bound(dt, 0, 2 days);
        if ((hf >> 8) % 32 == 0) exp = _now() - 1;
        bytes32 d = digestOf(address(c.inst), keccak256(abi.encode(TH_SETTLE, uint256(b), exp)));
        uint8 who = uint8((hf >> 16) % 3); // 0 a stranger, 1 the seller, 2 the buyer
        bytes memory sigS = _settleSlot(c, true, who == 1, (hf >> 24) % 3, d);
        bytes memory sigB = _settleSlot(c, false, who == 2, (hf >> 32) % 3, d);
        bytes memory data = abi.encodeCall(Eravon.settle, (b, exp, sigB, sigS));
        if (who == 0) {
            _strangerRefused(address(c.inst), data, "settleRefused");
            return;
        }
        address from = who == 1 ? c.t.offer.sellerKey : c.t.buyerKey;
        bool must = _isOpen(i) && b <= 10000 && exp >= _now();
        _exec(i, A_SETTLE, must, from, false, data);
    }

    // ------------------------------------------------------------------
    // round 6: the funds' own acts
    // ------------------------------------------------------------------

    /// An unopened, live order with a nonzero D whose fund is the key or bound, an address that can approve.
    function _boundLive(uint256 i) internal view returns (bool) {
        C storage c = cases[i];
        address f = c.t.offer.sellerFund;
        if (_ph(i) != 0 || _dead(c) || c.badInts || c.d == 0) return false;
        if (f != c.t.offer.sellerKey && !boundG[c.t.offer.sellerKey][f]) return false;
        return f != address(logic) && f != address(verifier) && f.codehash != logic.PROXY_CODEHASH();
    }

    /// An unopened, live order whose fund could bind itself and has not.
    function _unboundLive(uint256 i) internal view returns (bool) {
        C storage c = cases[i];
        address f = c.t.offer.sellerFund;
        if (_ph(i) != 0 || _dead(c) || c.badInts || f == c.t.offer.sellerKey || boundG[c.t.offer.sellerKey][f]) return false;
        return f != address(logic) && f != address(verifier) && f.codehash != logic.PROXY_CODEHASH();
    }

    /// The addresses the campaign names as funds and as sellers, for the bond's ghost.
    function fundCandidates() external view returns (address[] memory f, address[] memory s) {
        f = new address[](10 + poolFunds.length);
        for (uint256 k = 0; k < poolFunds.length; k++) {
            f[10 + k] = poolFunds[k];
        }
        f[0] = FUNDER;
        f[1] = cfunder;
        f[2] = UNBOUND;
        f[3] = SELLER_PAYOUT;
        f[4] = BUYER_PAYOUT;
        f[5] = BUYER;
        f[6] = coBuyer;
        f[7] = SELLER;
        f[8] = address(logic);
        f[9] = address(verifier);
        s = new address[](2);
        s[0] = SELLER;
        s[1] = coSeller;
    }

    /// s7a, s9: the bond and the allowance, by the funds' own acts. A fund (an EOA, a contract, a payout, the buyer's
    /// key, the stranger) binds itself to a seller on the logic, again where it already did; acceptFunding through an
    /// instance is NotRegistry and binds nothing; a fund sets its allowance to the logic mid-case (zero, half, or more):
    /// no open case moves, and the next init it funds answers by the model; a stranger binds itself in the same block
    /// as an init that names it, before or after it.
    function doFund(uint256 seed, uint256 mode) external {
        uint256 hm = _h(mode ^ 0xf0d);
        uint8 m = uint8(hm % 6);
        if (m == 0) {
            address[7] memory fs = [FUNDER, cfunder, UNBOUND, SELLER_PAYOUT, BUYER_PAYOUT, BUYER, coBuyer];
            address f = fs[(hm >> 8) % 7];
            address s = (hm >> 16) % 2 == 0 ? SELLER : coSeller;
            _bindFund(f, s);
            return;
        }
        (bool ok, uint256 i) = _pickAny(seed);
        if (!ok) return;
        C storage c = cases[i];
        address f = c.t.offer.sellerFund;
        if (m == 5) {
            // guard 4 reads sellerFund (s4 step 5): the order's twin with a zero fund is BadTerms(4), from the seller
            // and before any consent is read
            if (_dead(c) || c.badInts) return;
            Eravon.OrderTerms memory t0 = c.t;
            t0.offer.sellerFund = address(0);
            address a0 = instanceOf(t0);
            if (a0.code.length == 0) deployProxy(t0);
            address[] memory m0 = new address[](0);
            bytes32[] memory w0 = new bytes32[](0);
            if (c.track != 0) (m0, w0) = _frozen(c);
            bytes memory data0 = abi.encodeCall(Eravon.init, (t0, c.quote, bytes(""), bytes(""), m0, w0));
            (bool s0, bytes memory r0) = _try(c.t.offer.sellerKey, a0, data0);
            if (s0 || keccak256(r0) != keccak256(abi.encodeWithSelector(Eravon.BadTerms.selector, uint8(4)))) {
                bondWrong++;
                _r5(abi.encode("guard4", r0));
            }
            _mark("fund.zeroFundGuard4");
            return;
        }
        if (m == 1) {
            // through an instance the door is the registry's alone, from a key and from a contract
            bool before = logic.funds(c.t.offer.sellerKey, STRANGER);
            vm.prank(STRANGER);
            try c.inst.acceptFunding(c.t.offer.sellerKey) {
                bondWrong++;
            } catch (bytes memory e) {
                if (bytes4(e) != Eravon.NotRegistry.selector) bondWrong++;
            }
            try Company(cfunder).exec(address(c.inst), abi.encodeCall(Eravon.acceptFunding, (c.t.offer.sellerKey))) {
                bondWrong++;
            } catch (bytes memory e) {
                if (bytes4(e) != Eravon.NotRegistry.selector) bondWrong++;
            }
            if (logic.funds(c.t.offer.sellerKey, STRANGER) != before || c.inst.funds(c.t.offer.sellerKey, cfunder)) bondWrong++;
            _mark("fund.bindThroughInstanceRefused");
            return;
        }
        if (m == 2 || m == 3) {
            if (m == 3) {
                (bool okb, uint256 j) = _pickWhere(seed ^ hm, _boundLive);
                if (okb && _boundLive(j)) {
                    i = j;
                    c = cases[i];
                    f = c.t.offer.sellerFund;
                }
            }
            if (f == address(logic) || f == address(verifier) || f.codehash == logic.PROXY_CODEHASH()) return;
            uint256 cur = tok.allowance(f, address(logic));
            uint256 k = (hm >> 8) % 3;
            uint256 v = k == 0 ? 0 : (k == 1 ? cur / 2 : (cur > type(uint256).max - c.d ? type(uint256).max : cur + c.d));
            // one time in two, an unopened order's pool one short of its D
            if (m == 3 && _boundLive(i) && (hm >> 12) % 2 == 0) v = c.d - 1;
            bytes32 o0 = _outcome(i);
            (uint256 bh0, uint256 s0, uint256 j0) = _tot(i);
            if (isPoolFund[f]) _poolApprove(f, v);
            else _approve(f, address(logic), v);
            // the open case keeps every sum: its deposit was pulled at init and returns by transfer, not by allowance
            (uint256 bh1, uint256 s1, uint256 j1) = _tot(i);
            if (_outcome(i) != o0 || bh1 != bh0 || s1 != s0 || j1 != j0) {
                allowanceMovedOpen++;
                _r5(abi.encode("allowance", i, v));
            }
            _mark(v == 0 ? "fund.allowanceCleared" : "fund.allowanceChanged");
            if (_isOpen(i)) _mark("fund.allowanceChangedWhileOpen");
            if (m == 3 && _ph(i) == 0) {
                if (v < c.d && c.d != 0 && !_dead(c) && !c.badInts && _fundFault(c) == 2) _mark("fund.initAfterTheAllowanceShort");
                _init(i);
            }
            return;
        }
        // the race: a stranger's bond and the init naming it, one block, either order
        (ok, i) = _pickWhere(seed ^ hm, _unboundLive);
        if ((!ok || !_unboundLive(i)) && racers < 4 && cases.length < MAX_CASES + threeCount + poolCases) {
            // a fresh address that funds nothing yet, named by a new live order whose init it races
            forceFund = address(uint160(RACER) + uint160(racers++));
            _newOrder(_h(hm ^ 0x7ace), 1 + seed % 1e24, seed >> 8, 0, false);
            forceFund = address(0);
            i = cases.length - 1;
            ok = true;
        }
        if (!ok) return;
        c = cases[i];
        f = c.t.offer.sellerFund;
        if (_ph(i) != 0 || _dead(c) || c.badInts || f == c.t.offer.sellerKey || boundG[c.t.offer.sellerKey][f]) return;
        if (f == address(logic) || f == address(verifier) || f.codehash == logic.PROXY_CODEHASH()) return;
        if ((hm >> 8) % 2 == 0) {
            _bindFund(f, c.t.offer.sellerKey);
            _init(i);
            _mark("fund.raceBondFirst");
        } else {
            _init(i);
            _bindFund(f, c.t.offer.sellerKey);
            _mark("fund.raceInitFirst");
            _init(i);
        }
    }

    // ------------------------------------------------------------------
    // round 7: the pool, the judge that funds, and init's order around the bond
    // ------------------------------------------------------------------

    function poolFundList() external view returns (address[] memory) {
        return poolFunds;
    }

    function caseToFund(uint256 i) external view returns (uint256) {
        return cases[i].toFundG;
    }

    /// What `f` is owed at every instance of the campaign.
    function owedAll(address f) external view returns (int256) {
        return _owedAll(f);
    }

    function _owedAll(address f) internal view returns (int256 s) {
        for (uint256 k = 0; k < cases.length; k++) {
            s += int256(cases[k].inst.owed(f, address(tok)));
        }
    }

    function _poolApprove(address f, uint256 v) internal {
        _approve(f, address(logic), v);
        allowG[f] = v;
    }

    /// The state of every case of the pool but `skip`: its phase, seats, the money it holds and its balance.
    function _poolFrame(PoolRec storage p, uint256 skip) internal view returns (bytes32[] memory fr) {
        fr = new bytes32[](p.n);
        for (uint256 k = 0; k < p.n; k++) {
            uint256 i = p.first + k;
            if (i == skip) continue;
            Eravon inst = cases[i].inst;
            S memory st = _st(inst);
            fr[k] = keccak256(
                abi.encode(
                    st.ph, st.seats, st.premium, st.deposit, st.lfHeld, st.unpaid, st.claimedAt, st.disputedAt, tok.balanceOf(address(inst))
                )
            );
        }
    }

    function _poolSame(PoolRec storage p, uint256 skip, bytes32[] memory fr, string memory what) internal {
        bytes32[] memory now_ = _poolFrame(p, skip);
        for (uint256 k = 0; k < p.n; k++) {
            if (now_[k] != fr[k]) {
                poolWrong++;
                _r5(abi.encode("poolMoved", what, p.first + k, skip));
            }
        }
    }

    function _poolOpen(PoolRec storage p) internal view returns (uint256 n) {
        for (uint256 k = 0; k < p.n; k++) {
            if (_isOpen(p.first + k)) n++;
        }
    }

    /// A pool with a case still unopened or open, searched from `hm`; its index plus one, or zero.
    function _livePool(uint256 hm) internal view returns (uint256) {
        uint256 n = poolList.length;
        if (n == 0) return 0;
        for (uint256 k = 0; k < n; k++) {
            uint256 x = (hm + k) % n;
            PoolRec storage p = poolList[x];
            for (uint256 m = 0; m < p.n; m++) {
                if (_ph(p.first + m) <= 3) return x + 1;
            }
        }
        return 0;
    }

    /// Round 7 (s7a, s8): one fund, several orders of one seller. A fresh key, a named judge (the judge of some of the
    /// cases it funds) or a guild member (whom the draw may seat on a case it funds) binds itself to the seller; the
    /// orders are registered unopened; the fund sets its allowance and moves its balance, and their inits, the fund's
    /// approves, transfers, a block on it and its bonds, and the cases' roads to a terminal interleave, within a call
    /// and across calls. Every init answers by the pool's ghosts (the bond, the allowance and the balance against D), each
    /// opening pull draws exactly D from both, a terminal refills the balance and never the allowance, and no step on
    /// one case or on the fund moves another case of the pool.
    function doPool(uint256 seed, uint256 mode) external {
        uint256 hm = _h(mode ^ 0x9007);
        uint256 live = _livePool(hm);
        if (live != 0 && ((hm >> 4) % 3 != 0 || poolList.length >= MAX_POOLS)) return _poolSteps(live - 1, hm >> 8, 2 + (hm >> 12) % 4);
        if (poolList.length >= MAX_POOLS) return;
        _newPool(hm, seed);
        _poolSteps(poolList.length - 1, hm >> 16, 3 + (hm >> 20) % 5);
    }

    function _newPool(uint256 hm, uint256 seed) internal {
        uint8 kind = uint8(hm % 3);
        address f;
        uint8 trk; // forceTrack: 3 the named track, 1 the guild of judges, 4 the guild of five
        if (kind == 0) {
            f = address(uint160(POOL) + uint160(poolKeys++));
            trk = (hm >> 40) % 3 == 0 ? 1 : 3;
        } else if (kind == 1) {
            f = namedJudges[(hm >> 40) % 3];
            trk = 3;
        } else {
            uint256 w = (hm >> 40) % 5;
            f = w < 3 ? members[w] : g3[w - 3];
            trk = w < 3 && (hm >> 44) % 2 == 0 ? 1 : 4;
        }
        address sel_ = (hm >> 48) % 2 == 0 ? SELLER : coSeller;
        if (!isPoolFund[f]) {
            isPoolFund[f] = true;
            poolFunds.push(f);
            allowG[f] = tok.allowance(f, address(logic));
            balG[f] = int256(tok.balanceOf(f)) + _owedAll(f);
        }
        PoolRec storage p = poolList.push();
        p.f = f;
        p.s = sel_;
        p.first = cases.length;
        p.kind = kind;
        p.late = (hm >> 52) % 6 == 0 && !boundG[sel_][f];
        if (!p.late && !boundG[sel_][f]) _bindFund(f, sel_);
        uint256 n = 2 + (hm >> 56) % 3;
        forceSeller = sel_;
        forceFund = f;
        forceShut = true;
        forceTrack = trk;
        for (uint256 k = 0; k < n; k++) {
            forceJudge = kind == 1 && k == 0 ? f : address(0);
            uint256 hk = _h(seed ^ (k + 0x9008));
            _newOrder(_h(hm ^ (k + 1)), 1 + hk % 1e24, hk >> 8, 0, false);
        }
        forceSeller = address(0);
        forceFund = address(0);
        forceShut = false;
        forceTrack = 0;
        forceJudge = address(0);
        p.n = cases.length - p.first;
        poolCases += p.n;
        // the fund's allowance: every D, one short of every D, one short of one D, nothing, unlimited, or as registered
        uint256 sum;
        for (uint256 k = 0; k < p.n; k++) {
            sum += cases[p.first + k].d;
        }
        uint256 one = cases[p.first + (hm >> 60) % p.n].d;
        uint256 a = (hm >> 64) % 6;
        if (a == 0) _poolApprove(f, sum);
        else if (a == 1) _poolApprove(f, sum == 0 ? 0 : sum - 1);
        else if (a == 2) _poolApprove(f, one == 0 ? 0 : one - 1);
        else if (a == 3) _poolApprove(f, 0);
        else if (a == 4) _poolApprove(f, type(uint256).max);
        // one pool in three keeps a balance for all its deposits but one at once
        if ((hm >> 68) % 3 == 0 && p.n > 1 && one != 0) {
            uint256 keep = sum - one;
            uint256 b = tok.balanceOf(f);
            if (b > keep) {
                vm.prank(f);
                btok.transfer(STRANGER, b - keep);
                balG[f] -= int256(b - keep);
            }
            _mark("pool.balanceGated");
        }
        _mark(kind == 0 ? "pool.fundKey" : (kind == 1 ? "pool.fundJudge" : "pool.fundMember"));
    }

    function _poolSteps(uint256 pi, uint256 hs, uint256 steps) internal {
        PoolRec storage p = poolList[pi];
        for (uint256 k = 0; k < steps; k++) {
            uint256 h = _h(hs ^ (k << 8) ^ 0x5007);
            uint256 op = h % 8;
            uint256 j = p.first + (h >> 8) % p.n;
            if (op <= 2) _poolInit(p, j);
            else if (op == 3) _poolAllowance(p, h >> 16);
            else if (op <= 5) _poolDrive(p, j, h >> 16);
            else if (op == 6) _poolMoney(p, h >> 16);
            else _poolBond(p, h >> 16);
        }
    }

    function _poolInit(PoolRec storage p, uint256 j0) internal {
        uint256 j = type(uint256).max;
        for (uint256 k = 0; k < p.n; k++) {
            uint256 x = p.first + (j0 - p.first + k) % p.n;
            if (_ph(x) == 0) {
                j = x;
                break;
            }
        }
        if (j == type(uint256).max) return;
        C storage c = cases[j];
        address f = p.f;
        bool live = !_dead(c) && !c.badInts;
        // the ghost's verdict: 0 the pull, 1 FundUnbound, 2 TransferFailed (the allowance, or the balance, short of D)
        uint8 want;
        bool byBal;
        if (f != c.t.offer.sellerKey && !boundG[c.t.offer.sellerKey][f]) {
            want = 1;
        } else if (c.d != 0) {
            if (allowG[f] < c.d) {
                want = 2;
            } else if (balG[f] - _owedAll(f) < int256(c.d)) {
                want = 2;
                byBal = true;
            }
        }
        bytes32[] memory fr = _poolFrame(p, j);
        uint256 a0 = tok.allowance(f, address(logic));
        uint256 b0 = tok.balanceOf(f);
        _init(j);
        if (live) {
            bool opened = _ph(j) != 0;
            if (opened != (want == 0)) {
                poolWrong++;
                _r5(abi.encode("poolInit", j, want, _ph(j), allowG[f], c.d));
            }
            if (!opened && (tok.allowance(f, address(logic)) != a0 || tok.balanceOf(f) != b0)) {
                poolWrong++;
                _r5(abi.encode("poolRefusedMoved", j));
            }
            if (opened) {
                _mark("pool.opened");
                if (poolShort[j] == 1) _mark("pool.openedAfterTheAllowanceRose");
                if (poolShort[j] == 2) _mark("pool.openedAfterTheBalanceRose");
                if (_seatCount(j) != 0 && _lastSeat(j).judge == f) {
                    _mark(p.kind == 1 ? "pool.judgeSeatedOnItsFund" : "pool.memberSeatedOnItsFund");
                }
                if (_poolOpen(p) >= 2) _mark("pool.severalOpenAtOnce");
            } else {
                poolShort[j] = want == 1 ? 3 : (byBal ? 2 : 1);
                _mark(want == 1 ? "pool.unboundRefused" : (byBal ? "pool.balanceShortRefused" : "pool.allowanceShortRefused"));
            }
        }
        _poolSame(p, j, fr, "init");
        // one balance refusal in two is answered at once by a top-up of one D, and the order's init follows
        if (live && _ph(j) == 0 && poolShort[j] == 2 && !poolRetry && (uint256(c.id) >> 5) % 2 == 0) {
            _mint(f, c.d);
            balG[f] += int256(c.d);
            _mark("pool.balanceToppedUp");
            poolRetry = true;
            _poolInit(p, j);
            poolRetry = false;
        }
    }

    function _poolAllowance(PoolRec storage p, uint256 h) internal {
        address f = p.f;
        uint256 dj = cases[p.first + (h >> 8) % p.n].d;
        uint256 rest;
        for (uint256 k = 0; k < p.n; k++) {
            if (_ph(p.first + k) == 0) rest += cases[p.first + k].d;
        }
        uint256 cur = allowG[f];
        uint256 w = (h >> 16) % 6;
        uint256 v;
        if (w == 0) v = 0;
        else if (w == 1) v = cur == type(uint256).max ? rest : cur / 2;
        else if (w == 2) v = cur == type(uint256).max ? cur : cur + dj;
        else if (w == 3) v = dj == 0 ? 0 : dj - 1;
        else if (w == 4) v = rest;
        else v = type(uint256).max;
        bytes32[] memory fr = _poolFrame(p, type(uint256).max);
        _poolApprove(f, v);
        _poolSame(p, type(uint256).max, fr, "approve");
        _mark(v == 0 ? "pool.allowanceCleared" : "pool.allowanceSet");
        if (_poolOpen(p) != 0) _mark("pool.allowanceSetWhileCasesOpen");
    }

    function _poolMoney(PoolRec storage p, uint256 h) internal {
        address f = p.f;
        uint256 w = h % 4;
        // one step in two tops the balance up where an unopened order of the pool was refused for it
        for (uint256 k = 0; k < p.n && (h >> 40) % 2 == 0; k++) {
            if (_ph(p.first + k) == 0 && poolShort[p.first + k] == 2) w = 1;
        }
        bytes32[] memory fr = _poolFrame(p, type(uint256).max);
        if (w == 0) {
            uint256 b = tok.balanceOf(f);
            if (b == 0) return;
            uint256 v = 1 + (h >> 8) % b;
            vm.prank(f);
            btok.transfer(STRANGER, v);
            balG[f] -= int256(v);
            _mark("pool.balanceSent");
        } else if (w == 1) {
            // a top-up by one D; where an unopened order was refused for the balance, by its D, and its init follows
            uint256 j = p.first + (h >> 8) % p.n;
            for (uint256 k = 0; k < p.n; k++) {
                if (_ph(p.first + k) == 0 && poolShort[p.first + k] == 2) j = p.first + k;
            }
            uint256 v = cases[j].d;
            if (v == 0) v = 1;
            _mint(f, v);
            balG[f] += int256(v);
            _mark("pool.balanceToppedUp");
            _poolSame(p, type(uint256).max, fr, "money");
            if (_ph(j) == 0 && poolShort[j] == 2) _poolInit(p, j);
            return;
        } else if (w == 2) {
            btok.setBlocked(f, !btok.blocked(f));
            if (btok.blocked(f)) _mark("pool.fundBlocked");
        } else {
            btok.setBlocked(f, false);
            _poolSame(p, type(uint256).max, fr, "money");
            // every credit the fund holds at a case of the pool, pushed by the seller's sweep (s8: a party's current key)
            for (uint256 k = 0; k < p.n; k++) {
                uint256 i = p.first + k;
                if (_ph(i) == 0 || cases[i].inst.owed(f, address(tok)) == 0) continue;
                _exec(i, A_SWEEP, true, cases[i].t.offer.sellerKey, false, abi.encodeCall(Eravon.sweep, (f, address(tok))));
                if (cases[i].inst.owed(f, address(tok)) != 0 && !btok.burns(f)) {
                    poolWrong++;
                    _r5(abi.encode("poolSweepLeft", i));
                }
                _mark("pool.sweptTheFundsCredit");
            }
            return;
        }
        _poolSame(p, type(uint256).max, fr, "money");
    }

    function _poolBond(PoolRec storage p, uint256 h) internal {
        bytes32[] memory fr = _poolFrame(p, type(uint256).max);
        if (!boundG[p.s][p.f]) {
            _bindFund(p.f, p.s);
            _mark("pool.lateBond");
        } else {
            address other = p.s == SELLER ? coSeller : SELLER;
            _bindFund(p.f, h % 2 == 0 ? other : p.s);
        }
        _poolSame(p, type(uint256).max, fr, "bond");
    }

    /// One step of an open case of the pool down its road: the draw, the seller's refund, accept, claim, confirm,
    /// dispute, rule, resign, or the clock; every other case of the pool unmoved.
    function _poolDrive(PoolRec storage p, uint256 j0, uint256 h) internal {
        uint256 j = type(uint256).max;
        // one step in two drives a case judged by its own fund where the pool has one open
        for (uint256 k = 0; k < p.n && (h >> 24) % 2 == 0; k++) {
            uint256 x = p.first + (j0 - p.first + k) % p.n;
            if (_isOpen(x) && _seatCount(x) != 0 && _lastSeat(x).judge == p.f) {
                j = x;
                break;
            }
        }
        for (uint256 k = 0; k < p.n && j == type(uint256).max; k++) {
            uint256 x = p.first + (j0 - p.first + k) % p.n;
            if (_isOpen(x)) {
                j = x;
                break;
            }
        }
        if (j == type(uint256).max) return;
        C storage c = cases[j];
        bytes32[] memory fr = _poolFrame(p, j);
        S memory st = _st(c.inst);
        // a case judged by its own fund: three times in four the fund is blocked for the step, so that a terminal leaves its
        // credit owed beside the seat's claimable, and a stranger's sweep of it follows (both ledgers in one call)
        bool dualCase = st.seats != 0 && _lastSeat(j).judge == p.f;
        if (dualCase && (h >> 20) % 4 != 0) btok.setBlocked(p.f, true);
        if (st.pending) {
            (address[] memory m, bytes32[] memory w) = _frozen(c);
            if (_pendingLive(j)) {
                vm.roll(vm.getBlockNumber() + 1);
                _exec(j, A_REDRAW, true, _drawer(c, h >> 28), false, abi.encodeCall(Eravon.redraw, (m, w)));
            } else {
                vm.roll(vm.getBlockNumber() + 2);
                advSeat = uint64(st.seats);
                _exec(j, A_ADVANCE, true, STRANGER, false, abi.encodeCall(Eravon.advance, (c.id, uint64(st.seats))));
            }
        } else if (h % 5 == 0 && st.ph <= 2) {
            _exec(j, A_REFUND, true, c.t.offer.sellerKey, false, abi.encodeCall(Eravon.refund, ()));
        } else if (_canAccept(j)) {
            (uint64 s, address jd) = _lastOrStranger(j);
            _judgeCall(j, A_ACCEPT, true, jd, abi.encodeCall(Eravon.accept, (c.id, s)));
        } else if (_canClaim(j)) {
            _exec(j, A_CLAIM, true, c.t.offer.sellerKey, false, abi.encodeCall(Eravon.claimDelivery, (keccak256(abi.encode(h, "pool")))));
        } else if (_canConfirm(j) && (h >> 8) % 3 == 0) {
            (address who, bytes memory data) =
                _buyerCall(c, TH_CONFIRM, false, abi.encodeCall(Eravon.confirm, ()), Eravon.confirmBySig.selector);
            _exec(j, A_CONFIRM, true, who, false, data);
        } else if (_canDispute(j) && _lastSeat(j).state == 0) {
            _fundBuyerKey(c, c.lf);
            (address who, bytes memory data) =
                _buyerCall(c, TH_DISPUTE, true, abi.encodeCall(Eravon.dispute, ()), Eravon.disputeBySig.selector);
            _exec(j, A_DISPUTE, true, who, false, data);
        } else if (_canRule(j)) {
            (uint64 s, address jd) = _lastOrStranger(j);
            uint64 rg = uint64((h >> 12) % c.t.offer.ladder.length);
            _judgeCall(j, A_RULE, true, jd, abi.encodeCall(Eravon.rule, (c.id, s, E, rg, ENTRY, "")));
        } else if (_canResign(j)) {
            (uint64 s, address jd) = _lastOrStranger(j);
            if (_judgeCall(j, A_RESIGN, true, jd, abi.encodeCall(Eravon.resign, (c.id, s, E, ENTRY)))) _drawSoon(j);
        } else if (_due(j)) {
            (uint64 s,) = _lastOrStranger(j);
            advSeat = s;
            _exec(j, A_ADVANCE, true, STRANGER, false, abi.encodeCall(Eravon.advance, (c.id, s)));
        } else if (st.ph <= 2) {
            _exec(j, A_REFUND, true, c.t.offer.sellerKey, false, abi.encodeCall(Eravon.refund, ()));
        }
        if (dualCase && btok.blocked(p.f) && _ph(j) >= 4 && c.inst.owed(p.f, address(tok)) != 0) {
            btok.setBlocked(p.f, false);
            // s9: the case's seated judge sweeps (here the judge that funds it, both its ledgers in one call)
            if (_exec(j, A_SWEEP, true, p.f, false, abi.encodeCall(Eravon.sweep, (p.f, address(tok))))) _mark("sweep.byASeatedJudge");
        }
        _poolSame(p, j, fr, "drive");
        if (_ph(j) >= 4) {
            _mark("pool.closed");
            if (_seatCount(j) != 0 && _lastSeat(j).judge == p.f) _mark("pool.closedByItsFundingJudge");
        }
    }

    /// Round 7 (s8, venue/1 s3): after an action on a case whose credited judge is its sellerFund, what left the seat's
    /// claimable reached the seat's registered payout and nothing else, what left the party credit reached the address
    /// itself and nothing else, withdraw touched no party credit, and neither ledger grew after the terminal.
    function _dualAfter(uint256 i, uint8 act) internal {
        if (!snap.dual) return;
        C storage c = cases[i];
        address jd = c.credited;
        (address pay, uint256 cl) = c.inst.accountOf(jd, address(tok));
        uint256 ow = c.inst.owed(jd, address(tok));
        if (cl > snap.dClaim || ow > snap.dOwed) {
            dualWrong++;
            _r5(abi.encode("dualGrew", i, act, cl, snap.dClaim, ow, snap.dOwed));
            return;
        }
        uint256 outC = snap.dClaim - cl;
        uint256 outO = snap.dOwed - ow;
        c.judgeOut += outC;
        if (tok.balanceOf(jd) != snap.dBalJ + outO) {
            dualWrong++;
            _r5(abi.encode("dualCreditOut", i, act, outO));
        }
        // (a payout registered by this very action had no balance read before it: setPayout moves no money)
        if (pay != address(0) && pay == snap.dPay && pay != jd && tok.balanceOf(pay) != snap.dBalPay + outC) {
            dualWrong++;
            _r5(abi.encode("dualSeatOut", i, act, outC));
        }
        if (act == A_MONEY && outO != 0) {
            dualWrong++;
            _r5(abi.encode("dualWithdrawTookTheCredit", i));
        }
        if (outC != 0) _mark("dual.seatLedgerOut");
        if (outO != 0) _mark("dual.creditOut");
    }

    struct Probe {
        bool ok;
        bytes ret;
        bool moved;
    }

    /// One init of a twin inside a snapshot: the instance holding `instBal`, UNBOUND2 holding D with allowance `allow`
    /// to the logic and bound to SELLER where `bind`; what it answered and whether UNBOUND2's balance moved.
    function _probe(
        Eravon.OrderTerms memory t,
        bytes memory quote,
        bytes memory sigB,
        bytes memory sigS,
        address[] memory m,
        bytes32[] memory w,
        address caller,
        uint256 instBal,
        uint256 d,
        bool bind,
        uint256 allow
    ) internal returns (Probe memory r) {
        uint256 s0 = vm.snapshotState();
        Eravon inst = deployProxy(t);
        btok.mint(address(inst), instBal);
        btok.mint(UNBOUND2, d);
        vm.prank(UNBOUND2);
        btok.approve(address(logic), allow);
        if (bind) {
            vm.prank(UNBOUND2);
            logic.acceptFunding(SELLER);
        }
        uint256 fb = tok.balanceOf(UNBOUND2);
        vm.prank(_cur(caller));
        (r.ok, r.ret) = address(inst).call{gas: TX_GAS}(abi.encodeCall(Eravon.init, (t, quote, sigB, sigS, m, w)));
        r.moved = tok.balanceOf(UNBOUND2) != fb;
        vm.revertToStateAndDelete(s0);
    }

    function _probeIs(Probe memory r, bytes4 want, uint8 k) internal {
        if (r.ok || r.ret.length != 4 || bytes4(r.ret) != want || r.moved) {
            fundOrderWrong++;
            _r5(abi.encode("fundOrder", k, r.ok, r.ret));
        }
    }

    /// Round 7 (s4's order, s10: FundUnbound after the consents): twins of a live order, a key seller and a key buyer,
    /// whose fund is a fresh address holding D with an allowance: the caller rule before everything (NotSeller); a wrong
    /// consent before the bond (BadBuyerSig, BadSellerSig); the bond before the balance (FundUnbound with the instance
    /// one short of A), a zero deposit no exemption; bound, the balance before the deposit (Underfunded), the pull one
    /// short of D (TransferFailed), and the twin opens with both. Each probe in a snapshot that changes nothing.
    function doFundOrder(uint256 seed, uint256 mode) external {
        (bool ok, uint256 i) = _pickAny(seed);
        if (!ok) return;
        C storage c = cases[i];
        if (c.track == 2 || c.unit) return;
        uint256 hm = _h(mode ^ 0xf0de);
        Eravon.OrderTerms memory t = c.t;
        t.offer.sellerKey = SELLER;
        t.buyerKey = BUYER;
        t.buyerPayout = BUYER_PAYOUT;
        t.offer.sellerFund = UNBOUND2;
        t.offer.termsHash = keccak256(abi.encode("twin", ++nonceCounter));
        t.offer.nonce = nonceCounter;
        t.offer.expiry = _now() + 30 days;
        t.openBy = uint64(_now() + 1 days);
        t.offer.buyerMayOpen = true;
        bytes memory quote = c.quote;
        uint256 d = c.d;
        if (hm % 4 == 0) {
            Q memory q0 = defaultQuote(address(tok), address(0), (hm >> 4) % 2 == 0);
            q0.premiumBps = 0;
            q0.feeFixed = 0;
            q0.feeBps = 0;
            q0.feeMin = 0;
            q0.feeMax = 0;
            quote = encodeQuote(q0);
            t.offer.quoteHash = keccak256(quote);
            d = 0;
        }
        address[] memory m;
        bytes32[] memory w;
        if (c.track == 0) {
            m = new address[](0);
            w = new bytes32[](0);
        } else {
            (m, w) = _panel(c.track, c.track == 1 ? guildPanel : 0);
        }
        uint256 a = t.offer.amount;
        bytes memory sB = _buyerSig(t);
        // the caller rule first: a stranger on a seller-only twin (NotSeller), and under buyerMayOpen (NotParty), every
        // later test failing too (the twin is one memory object: buyerMayOpen is cleared for the first probe and set
        // again after it; the seller's signature binds buyerMayOpen and is taken after)
        t.offer.buyerMayOpen = false;
        _probeIs(_probe(t, quote, _sellerSig(t), _buyerSig(t), m, w, STRANGER, a - 1, d, false, 0), Eravon.NotSeller.selector, 0);
        t.offer.buyerMayOpen = true;
        bytes memory sS = _sellerSig(t);
        _probeIs(_probe(t, quote, sS, sB, m, w, STRANGER, a - 1, d, false, 0), Eravon.NotParty.selector, 6);
        _mark("fundOrder.callerRuleFirst");
        // a wrong consent before the bond, the instance short of A too
        _probeIs(_probe(t, quote, sS, sS, m, w, SELLER, a - 1, d, false, d), Eravon.BadBuyerSig.selector, 1);
        _mark("fundOrder.buyerConsentBeforeBond");
        _probeIs(_probe(t, quote, sB, sB, m, w, BUYER, a - 1, d, false, d), Eravon.BadSellerSig.selector, 2);
        _mark("fundOrder.sellerConsentBeforeBond");
        // the bond before the balance
        _probeIs(_probe(t, quote, sB, sS, m, w, SELLER, a - 1, d, false, d), Eravon.FundUnbound.selector, 3);
        _mark(d == 0 ? "fundOrder.zeroDepositNoExemption" : "fundOrder.bondBeforeBalance");
        // bound: the balance before the deposit, then the pull one short
        _probeIs(_probe(t, quote, sB, sS, m, w, SELLER, a - 1, d, true, d == 0 ? 0 : d - 1), Eravon.Underfunded.selector, 4);
        _mark("fundOrder.balanceBeforeDeposit");
        if (d != 0) {
            _probeIs(_probe(t, quote, sB, sS, m, w, SELLER, a, d, true, d - 1), Eravon.TransferFailed.selector, 5);
            _mark("fundOrder.shortPull");
        }
        // bound, funded: the twin opens, D (if any) from UNBOUND2
        Probe memory r = _probe(t, quote, sB, sS, m, w, SELLER, a, d, true, d);
        if (!r.ok || r.moved != (d != 0)) {
            fundOrderWrong++;
            _r5(abi.encode("fundOrderOpen", r.ok, r.ret, d));
        }
        _mark("fundOrder.opens");
    }


    // ------------------------------------------------------------------
    // money
    // ------------------------------------------------------------------

    function doStray(uint256 seed, uint256 v, uint256 foreign) external {
        (bool ok, uint256 i) = _pickAny(seed);
        if (!ok) return;
        C storage c = cases[i];
        v = bound(v, 1, 1e24);
        if (_h(foreign) % 3 == 0) {
            ftok.mint(address(c.inst), v);
            totalMintedForeign += v;
            _mark("stray.foreign");
            return;
        }
        _mint(address(c.inst), v);
        if (_ph(i) >= 1) c.stray += v;
        _mark(_ph(i) == 0 ? "stray.beforeInit" : (_ph(i) <= 3 ? "stray.open" : "stray.afterTerminal"));
    }

    /// s8, s9: sweep never reverts on a plain token, for anyone, sent by either party's current key or a seated
    /// judge's; a stranger's sweep, and every sweep of an instance that knows neither party's key yet, is NotParty.
    function doSweep(uint256 seed, uint256 whoSeed, uint256 foreign) external {
        (bool ok, uint256 i) = _pickAny(seed);
        if (!ok) return;
        C storage c = cases[i];
        address[7] memory who = [
            c.t.buyerPayout,
            c.t.offer.sellerPayout,
            (_h(whoSeed ^ 0xf) % 2 == 0) ? c.t.offer.sellerFund : c.t.offer.sellerKey,
            c.t.buyerKey,
            STRANGER,
            _lastJudgeOr(i, STRANGER),
            c.credited == address(0) ? STRANGER : c.credited
        ];
        bool f = _h(foreign) % 3 == 0;
        address token_ = f ? address(ftok) : address(tok);
        uint256 fBefore = ftok.balanceOf(address(c.inst));
        uint256 hw = _h(whoSeed ^ 0x5e);
        uint256 sk = hw % 4; // the sender: 0 the seller, 1 the buyer, 2 the last seat's judge, 3 a stranger
        address from = sk == 0 ? c.t.offer.sellerKey : (sk == 1 ? c.t.buyerKey : (sk == 2 ? _lastJudgeOr(i, STRANGER) : STRANGER));
        bytes memory data = abi.encodeCall(Eravon.sweep, (who[_h(whoSeed) % 7], token_));
        if (from == STRANGER || !_keysKnown(i)) {
            (bool sok, bytes memory sret) = _try(from, address(c.inst), data);
            if (sok || sret.length != 4 || bytes4(sret) != Eravon.NotParty.selector) {
                strangerAdmitted++;
                _r5(abi.encode("sweep", from, sok, sret));
            }
            _mark(from == STRANGER ? "stranger.sweepRefused" : "sweep.refusedBeforeTheKeysAreKept");
            return;
        }
        if (!_exec(i, A_SWEEP, true, from, false, data)) return;
        if (sk == 2) _mark("sweep.byASeatedJudge");
        if (!f && _ph(i) >= 4) c.stray = 0;
        if (!f && snap.unpaid > c.inst.owed(c.t.buyerPayout, address(tok)) && _st(c.inst).unpaid < snap.unpaid) _mark("sweep.pushedACredit");
        if (f && fBefore != 0 && ftok.balanceOf(address(c.inst)) == 0) _mark("sweep.foreign");
    }

    /// venue/1 s3, s8: setPayout(judge, payout) on an instance, from the judge's current key, once, any nonzero
    /// address: a stranger is NotCurrent, the logic NotInstance, a zero payout ZeroPayout, a second PayoutSet.
    function _payoutProbe(C storage c, address j, address p_, bool had) internal {
        bytes memory data = abi.encodeCall(Eravon.setPayout, (j, p_));
        bool ok;
        bytes memory ret;
        if (j != STRANGER) {
            (ok, ret) = _tryAs(STRANGER, address(c.inst), data);
            _payoutIs(ok, ret, Eravon.NotCurrent.selector, "payout.strangerRefused");
        }
        (ok, ret) = _try(j, address(logic), data);
        _payoutIs(ok, ret, Eravon.NotInstance.selector, "payout.onTheLogicRefused");
        if (!had) {
            (ok, ret) = _try(j, address(c.inst), abi.encodeCall(Eravon.setPayout, (j, address(0))));
            _payoutIs(ok, ret, Eravon.ZeroPayout.selector, "payout.zeroRefused");
        } else {
            (ok, ret) = _try(j, address(c.inst), data);
            _payoutIs(ok, ret, Eravon.PayoutSet.selector, "payout.secondRefused");
        }
    }

    function _payoutIs(bool ok, bytes memory ret, bytes4 want, string memory what) internal {
        if (ok || ret.length != 4 || bytes4(ret) != want) {
            payoutRuleWrong++;
            _r5(abi.encode(what, ok, ret));
        }
        _mark(what);
    }

    function doSetPayout(uint256 seed, uint256 jSeed) external {
        (bool ok, uint256 i) = _pickWhere(seed, _judgeWithoutPayout);
        if (!ok) return;
        C storage c = cases[i];
        uint256 n = _ph(i) == 0 ? 0 : _seatCount(i);
        address j = STRANGER;
        if (n != 0) {
            uint256 start = _h(jSeed) % n;
            j = c.inst.seatOf(c.id, uint64(start)).judge;
            for (uint256 k = 0; k < n; k++) {
                address cand = c.inst.seatOf(c.id, uint64((start + k) % n)).judge;
                (address p,) = c.inst.accountOf(cand, address(tok));
                if (p == address(0)) {
                    j = cand;
                    break;
                }
            }
        }
        (address had,) = c.inst.accountOf(j, address(tok));
        address p_ = payoutOfJudge[j] == address(0) ? STRANGER : payoutOfJudge[j];
        _payoutProbe(c, j, p_, had != address(0));
        if (had != address(0)) return;
        // venue/1 s3: a first registration of a nonzero payout is accepted, from the key's current key
        if (_exec(i, A_MONEY, true, j, false, abi.encodeCall(Eravon.setPayout, (j, p_)))) {
            (address now_,) = c.inst.accountOf(j, address(tok));
            if (now_ != p_) payoutRuleWrong++;
            _mark("payout.set");
        }
    }

    /// venue/1 s3, s8: withdraw(token, judge, payout) from the judge's current key: it first registers `payout` where
    /// none is and it is nonzero, refuses another payout than the registered one (PayoutSet), refuses where none is
    /// registered after that (NoPayout), and pushes the claimable to the registered payout; a stranger is NotCurrent
    /// and the logic NotInstance.
    function doWithdraw(uint256 seed) external {
        (bool ok, uint256 i) = _pickWhere(seed, _judgeHasMoney);
        if (!ok) return;
        C storage c = cases[i];
        if (c.credited == address(0)) return;
        address j = c.credited;
        (address p,) = c.inst.accountOf(j, address(tok));
        uint256 mode = _h(seed ^ 0x3d) % 4;
        address arg = mode == 0 ? address(0) : (mode == 2 && p != address(0) ? STRANGER : payoutOfJudge[j]);
        bytes memory data = abi.encodeCall(Eravon.withdraw, (address(tok), j, arg));
        bool okr;
        bytes memory ret;
        (okr, ret) = _tryAs(STRANGER, address(c.inst), data);
        _payoutIs(okr, ret, Eravon.NotCurrent.selector, "withdraw.strangerRefused");
        (okr, ret) = _try(j, address(logic), data);
        _payoutIs(okr, ret, Eravon.NotInstance.selector, "withdraw.onTheLogicRefused");
        if (p == address(0) && arg == address(0)) {
            (okr, ret) = _try(j, address(c.inst), data);
            _payoutIs(okr, ret, Eravon.NoPayout.selector, "withdraw.noPayoutRefused");
            return;
        }
        if (p != address(0) && arg != address(0) && arg != p) {
            (okr, ret) = _try(j, address(c.inst), data);
            _payoutIs(okr, ret, Eravon.PayoutSet.selector, "withdraw.otherPayoutRefused");
            return;
        }
        address to = p != address(0) ? p : arg;
        bool must = _judgeHasMoney(i) && !btok.blocked(to) && !btok.burns(to);
        if (_exec(i, A_MONEY, must, j, false, data)) {
            (address now_,) = c.inst.accountOf(j, address(tok));
            if (now_ != to) payoutRuleWrong++;
            _mark(p == address(0) ? "withdraw.registersThePayout" : "withdraw");
        }
    }

    /// A payout the token blocks for a while, by a revert or by a false answer.
    function doBlock(uint256 seed) external {
        uint256 hs = _h(seed);
        address[7] memory targets =
            [SELLER_PAYOUT, BUYER_PAYOUT, SELLER, coSeller, payoutOfJudge[namedJudges[0]], payoutOfJudge[members[0]], FUNDER];
        if (hs % 3 == 0) {
            for (uint256 k = 0; k < 7; k++) {
                btok.setBlocked(targets[k], false);
            }
            return;
        }
        address w = targets[(hs >> 8) % 7];
        btok.setBlocked(w, !btok.blocked(w));
        btok.setAnswersFalse((hs >> 16) % 2 == 0);
        _mark("payout.blocked");
    }

    /// Pulls from impostors: a key, a contract that is no proxy, a party's contract, and the instance's own
    /// face; and the instance's face on the logic, which serves instances alone (setPayout there is NotInstance).
    function impostorPull(uint256 seed, uint256 v) external {
        (bool ok, uint256 i) = _pickAny(seed);
        if (!ok) return;
        v = bound(v, 1, 1e24);
        _mint(SELLER, v);
        uint256 have = tok.allowance(SELLER, address(logic));
        vm.prank(SELLER);
        tok.approve(address(logic), have + v);
        vm.prank(STRANGER);
        try logic.pull(SELLER, SELLER, address(tok), v) {
            impostorPullPassed++;
        } catch (bytes memory e) {
            if (bytes4(e) != Eravon.NotInstance.selector) impostorPullPassed++;
        }
        try impostor.tryPull(logic, SELLER, address(tok), v) {
            impostorPullPassed++;
        } catch (bytes memory e) {
            if (bytes4(e) != Eravon.NotInstance.selector) impostorPullPassed++;
        }
        try Company(coBuyer).exec(address(logic), abi.encodeCall(Eravon.pull, (SELLER, SELLER, address(tok), v))) {
            impostorPullPassed++;
        } catch (bytes memory e) {
            if (bytes4(e) != Eravon.NotInstance.selector) impostorPullPassed++;
        }
        try cases[i].inst.pull(SELLER, SELLER, address(tok), v) {
            impostorPullPassed++;
        } catch (bytes memory e) {
            if (bytes4(e) != Eravon.NotRegistry.selector) impostorPullPassed++;
        }
        address jd = namedJudges[seed % 3];
        vm.prank(_cur(jd));
        try logic.setPayout(jd, STRANGER) {
            impostorPullPassed++;
        } catch (bytes memory e) {
            if (bytes4(e) != Eravon.NotInstance.selector) impostorPullPassed++;
        }
        vm.prank(_cur(jd));
        try logic.withdraw(address(tok), jd, STRANGER) {
            impostorPullPassed++;
        } catch (bytes memory e) {
            if (bytes4(e) != Eravon.NotInstance.selector) impostorPullPassed++;
        }
        vm.prank(SELLER);
        tok.approve(address(logic), have);
        _mark("impostor.refused");
    }

    // ------------------------------------------------------------------
    // round 2: the edges the fold moved
    // ------------------------------------------------------------------

    /// One step of a case down the long road (accept, claim, dispute with the fee from the key, rule), so that
    /// the edge actions below, which close cases, leave the campaign its depth.
    function _progress(uint256 seed) internal {
        uint256 hs = _h(seed ^ 0x9e3779b9);
        bool ok;
        uint256 i;
        (ok, i) = _pickWhere(hs, _canRule);
        if (ok && _canRule(i) && (hs >> 8) % 2 == 0) {
            C storage c = cases[i];
            (uint64 s, address j) = _lastOrStranger(i);
            uint64 rg = uint64((hs >> 16) % c.t.offer.ladder.length);
            if (_judgeCall(i, A_RULE, true, j, abi.encodeCall(Eravon.rule, (c.id, s, E, rg, ENTRY, "")))) _mark("progress.rule");
            return;
        }
        (ok, i) = _pickWhere(hs >> 24, _canDispute);
        if (ok && _canDispute(i) && _lastSeat(i).state == 0) {
            C storage c = cases[i];
            _fundBuyerKey(c, c.lf);
            (address who, bytes memory data) =
                _buyerCall(c, TH_DISPUTE, true, abi.encodeCall(Eravon.dispute, ()), Eravon.disputeBySig.selector);
            if (_exec(i, A_DISPUTE, true, who, false, data)) _mark("progress.dispute");
            return;
        }
        (ok, i) = _pickWhere(hs >> 48, _canClaim);
        if (ok && _canClaim(i)) {
            C storage c = cases[i];
            if (_exec(i, A_CLAIM, true, c.t.offer.sellerKey, false, abi.encodeCall(Eravon.claimDelivery, (keccak256(abi.encode(hs, "m")))))) {
                _mark("progress.claim");
            }
            return;
        }
        (ok, i) = _pickWhere(hs >> 72, _canAccept);
        if (ok && _canAccept(i)) {
            C storage c = cases[i];
            (uint64 s, address j) = _lastOrStranger(i);
            if (_judgeCall(i, A_ACCEPT, true, j, abi.encodeCall(Eravon.accept, (c.id, s)))) _mark("progress.accept");
        }
    }

    /// A payee whose hook burns every unit of gas a transfer hands it, for a while.
    function doBurn(uint256 seed) external {
        uint256 hs = _h(seed);
        if ((hs >> 40) % 2 == 0) return _progress(seed);
        address[7] memory targets =
            [SELLER_PAYOUT, BUYER_PAYOUT, SELLER, coSeller, payoutOfJudge[namedJudges[0]], payoutOfJudge[members[0]], FUNDER];
        if (hs % 3 == 0) {
            for (uint256 k = 0; k < 7; k++) {
                btok.setBurns(targets[k], false);
            }
            return;
        }
        address w = targets[(hs >> 8) % 7];
        btok.setBurns(w, !btok.burns(w));
        _mark("payout.burns");
    }

    /// s7: the draw at the edge of the blockhash window: 255 or 256 blocks after the arming it draws, 257 or
    /// more it closes the case as Resigned form 3.
    function doDrawEdge(uint256 seed, uint256 mode) external {
        (bool ok, uint256 i) = _pickWhere(seed, _pendingLive);
        if (!ok || !_pendingLive(i)) return;
        C storage c = cases[i];
        (address[] memory m, bytes32[] memory w) = _frozen(c);
        uint256 armed = _st(c.inst).drawBlock;
        uint256[4] memory gaps = [uint256(255), 256, 257, 300];
        uint256 target = armed + gaps[_h(mode) % 4];
        if (vm.getBlockNumber() < target) vm.roll(target);
        uint256 gap = vm.getBlockNumber() - armed;
        if (_exec(i, A_REDRAW, true, _drawerNow(c), false, abi.encodeCall(Eravon.redraw, (m, w)))) {
            _mark(gap > 256 ? "redraw.edge.closedPast256" : (gap == 256 ? "redraw.edge.drawnAt256" : "redraw.edge.drawnBefore256"));
            _drawSoon(i);
        }
    }

    /// venue/1 s2.11, s7: advance at the last second of the clock in force and one second past it, with the
    /// last ordinal, the next one, the one after, and 2^64 - 1.
    function _pendingWithASeat(uint256 i) internal view returns (bool) {
        return _pendingAny(i) && _seatCount(i) != 0;
    }

    function doAdvanceEdge(uint256 seed, uint256 mode) external {
        if (_h(mode ^ 0xad) % 2 == 0) return _progress(seed);
        bool ok;
        uint256 i;
        // round 4: a re-armed draw (after an unaccepted seat) half the time it exists, so its deadline is run from
        // an opened ordinal too
        if ((_h(mode ^ 0xae) % 2) == 0) (ok, i) = _pickWhere(seed, _pendingWithASeat);
        if (!ok || !_pendingWithASeat(i)) (ok, i) = _pickWhere(seed, _isOpen);
        if (!ok || !_isOpen(i)) return;
        C storage c = cases[i];
        S memory st = _st(c.inst);
        uint256 hm = _h(mode);
        uint256 end_ = st.pending ? st.redrawDeadline : (st.seats != 0 ? _lastSeat(i).windowEnd : 0);
        uint256 to = end_ + hm % 2;
        if (to > _now()) vm.warp(to);
        vm.roll(vm.getBlockNumber() + 1);
        uint256 n = st.seats;
        uint256 pk = (hm >> 8) % 4;
        uint64 s = pk == 0 ? uint64(n == 0 ? 0 : n - 1) : (pk == 1 ? uint64(n) : (pk == 2 ? uint64(n + 1) : type(uint64).max));
        advSeat = s;
        if (_exec(i, A_ADVANCE, true, STRANGER, false, abi.encodeCall(Eravon.advance, (c.id, s)))) {
            _mark(_now() > end_ ? "advance.edge.pastTheEnd" : "advance.edge.atTheEnd");
            if (s >= n) _mark("advance.edge.ordinalNeverOpened");
            // the draw's deadline run from an opened ordinal (the doAdvance path, reached here too)
            if (st.pending && _ph(i) == 8 && s < n) _mark("advance.drawDeadline");
            _drawSoon(i);
        }
    }

    /// s8: the seller's refund at T3's last second (refused, WrongPhase) and one second past it (a plain
    /// refund), and while a draw is armed, before or past its deadline (nothing to the judge).
    function doRefundEdge(uint256 seed, uint256 mode) external {
        uint256 hm = _h(mode);
        if ((hm >> 40) % 2 == 0) return _progress(seed);
        bool ok;
        uint256 i;
        if (hm % 3 == 0) (ok, i) = _pickWhere(seed, _pendingAny);
        if (!ok) (ok, i) = _pickWhere(seed, _canRule);
        if (!ok || !_isOpen(i)) return;
        C storage c = cases[i];
        uint8 ph0 = _ph(i);
        uint256 end_ = 0;
        if (ph0 == 3) {
            end_ = _lastSeat(i).windowEnd;
            uint256 to = end_ + (hm >> 8) % 2;
            if (to > _now()) vm.warp(to);
            if (_now() <= end_ && _lastSeat(i).state == 1) {
                (bool okr, bytes memory er) = _try(c.t.offer.sellerKey, address(c.inst), abi.encodeCall(Eravon.refund, ()));
                if (okr || bytes4(er) != Eravon.WrongPhase.selector) livenessBroken++;
                _mark("refund.edge.refusedAtT3End");
                return;
            }
        }
        if (_exec(i, A_REFUND, true, c.t.offer.sellerKey, false, abi.encodeCall(Eravon.refund, ()))) {
            if (ph0 == 3) _mark("refund.edge.pastT3");
            else if (_seatCount(i) == 0) _mark("refund.edge.undrawn");
            else _mark("refund.edge.otherwise");
        }
    }

    /// s8: a settlement at the last second of the last seat's window and one second past it, on an unaccepted
    /// offer, an accepted seat in T1, T2 or T3, a resigned seat, and while a draw is armed.
    function doSettleEdge(uint256 seed, uint256 mode) external {
        uint256 hm = _h(mode);
        if ((hm >> 40) % 3 != 0) return _progress(seed);
        bool ok;
        uint256 i;
        // one call in four on an offer still open inside its clock, so that its last second is reached too
        if ((hm >> 52) % 4 == 0) (ok, i) = _pickWhere(seed, _canAccept);
        if (!ok && (hm >> 48) % 2 != 0) (ok, i) = _pickWhere(seed, _isLate);
        if (!ok) (ok, i) = _pickWhere(seed, _isOpen);
        if (!ok || !_isOpen(i)) return;
        C storage c = cases[i];
        S memory st = _st(c.inst);
        bool seat = !st.pending && st.seats != 0;
        uint256 end_ = 0;
        uint64 acc = 0;
        if (seat) {
            IGaleedVenue.Seat memory L = _lastSeat(i);
            end_ = L.windowEnd;
            acc = L.accepted;
            uint256 to = end_ + hm % 2;
            if (to > _now()) vm.warp(to);
        }
        uint16 b = uint16((hm >> 24) % 10001);
        uint256 exp = _now() + 1 days;
        bytes32 d = digestOf(address(c.inst), keccak256(abi.encode(TH_SETTLE, uint256(b), exp)));
        bytes memory sigS = _settleSlot(c, true, false, (hm >> 8) % 3, d);
        bytes memory sigB = _settleSlot(c, false, false, (hm >> 16) % 3, d);
        if (_exec(i, A_SETTLE, true, _submitter(c, uint256(d) >> 3), false, abi.encodeCall(Eravon.settle, (b, exp, sigB, sigS)))) {
            if (!seat) _mark("settle.edge.undrawn");
            else _mark(string(abi.encodePacked("settle.edge.", _now() > end_ ? "past" : "in", acc == 0 ? ".unaccepted" : ".accepted")));
        }
    }

    /// An order in a token that has no code or answers nothing (the same address on a chain where the token
    /// is absent, standard s4 step 3): the expiry branch and cancel run, credit nothing, and leave any other
    /// token to sweep.
    function newGhostOrder(uint256 flags) external {
        if (ghosts.length >= MAX_GHOSTS) return;
        flags = _h(flags);
        uint8 kind = uint8(flags % 2);
        address token_ = kind == 0 ? address(uint160(uint256(keccak256(abi.encode("no code", ghosts.length))))) : address(mute);
        bytes memory quote = encodeQuote(defaultQuote(token_, address(0), true));
        Eravon.OrderTerms memory t = namedTerms(namedJudges[0], AMANAH, quote, 1 ether);
        t.offer.termsHash = keccak256(abi.encode("ghost terms", nonceCounter));
        t.offer.token = token_;
        bool dead = (flags >> 8) % 2 == 0;
        if (dead) {
            t.openBy = uint64(_now() - 1);
        } else {
            // live for the campaign's span, so that init with both consents meets the token itself
            t.offer.expiry = _now() + 3650 days;
            t.openBy = uint64(_now() + 3650 days);
        }
        Eravon inst = deployProxy(t);
        G storage g = ghosts.push();
        g.inst = inst;
        g.t = t;
        g.quote = quote;
        g.kind = kind;
        if ((flags >> 16) % 2 == 0) {
            ftok.mint(address(inst), 1e18);
            totalMintedForeign += 1e18;
        }
        _mark(kind == 0 ? "ghost.noCode" : "ghost.mute");
    }

    function _logsHave(VmSafe.Log[] memory logs, address emitter, bytes32 topic, bytes memory data) internal pure returns (bool) {
        for (uint256 k = 0; k < logs.length; k++) {
            if (logs[k].emitter == emitter && logs[k].topics.length != 0 && logs[k].topics[0] == topic && keccak256(logs[k].data) == keccak256(data)) {
                return true;
            }
        }
        return false;
    }

    function _unpaidOf(Eravon inst) internal view returns (uint256 u) {
        u = inst.status().unpaid;
    }

    /// s4 for an order in a token without code or answers: past its gate the expiry branch runs (either party's
    /// current key, a stranger NotParty) and credits nothing; live, with both consents, init reads the balance through
    /// `_balance` and answers Underfunded; cancel lands and credits nothing; sweep, a party's, leaves another token
    /// once the payout is fixed, and is NotParty for every sender before that (no key is kept, s10).
    function doGhost(uint256 seed, uint256 mode) external {
        if (ghosts.length == 0) return;
        G storage g = ghosts[_h(seed) % ghosts.length];
        uint256 m = _h(mode) % 4;
        Eravon inst = g.inst;
        address tk = g.t.offer.token;
        bool dead = _now() > g.t.offer.expiry || _now() > g.t.openBy;
        address party = _h(mode ^ 0x9) % 2 == 0 ? SELLER : BUYER;
        if (m == 0 && !g.cancelled) {
            bytes memory data = dead
                ? abi.encodeCall(Eravon.init, (g.t, bytes(""), bytes(""), bytes(""), new address[](0), new bytes32[](0)))
                : abi.encodeCall(Eravon.init, (g.t, g.quote, _buyerSig(g.t), _sellerSig(g.t), new address[](0), new bytes32[](0)));
            if (dead) _strangerRefused(address(inst), data, "branchRefused");
            vm.recordLogs();
            vm.prank(_cur(dead ? party : SELLER));
            (bool ok, bytes memory ret) = address(inst).call{gas: TX_GAS}(data);
            if (!ok) _unnamed(A_INIT, ret);
            VmSafe.Log[] memory logs = vm.getRecordedLogs();
            if (dead) {
                bool refunded = _logsHave(logs, address(inst), Eravon.Refunded.selector, abi.encode(uint8(2)));
                if (!ok || !refunded || uint8(phaseOf(inst)) != 0 || inst.owed(g.t.buyerPayout, tk) != 0 || _unpaidOf(inst) != 0) ghostWrong++;
                g.branchRan = true;
                _mark("ghost.expiryBranch");
            } else {
                if (ok || ret.length != 4 || bytes4(ret) != Eravon.Underfunded.selector) ghostWrong++;
                _mark("ghost.liveInitUnderfunded");
            }
        } else if (m == 1 && !g.cancelled) {
            vm.recordLogs();
            vm.prank(_cur(BUYER));
            (bool ok, bytes memory ret) = address(inst).call{gas: TX_GAS}(abi.encodeCall(Eravon.cancel, (g.t, bytes(""))));
            if (!ok) _unnamed(A_CANCEL, ret);
            bool closedZero = _logsHave(vm.getRecordedLogs(), address(inst), Eravon.Closed.selector, abi.encode(uint8(0), 0, 0, 0, 0));
            if (!ok || !closedZero || uint8(phaseOf(inst)) != 4 || _unpaidOf(inst) != 0) ghostWrong++;
            g.cancelled = true;
            _mark("ghost.cancel");
        } else {
            uint256 fb = ftok.balanceOf(address(inst));
            vm.prank(_cur(party));
            (bool ok1, bytes memory r1) = address(inst).call{gas: TX_GAS}(abi.encodeCall(Eravon.sweep, (g.t.buyerPayout, tk)));
            vm.prank(_cur(party));
            (bool ok2, bytes memory r2) = address(inst).call{gas: TX_GAS}(abi.encodeCall(Eravon.sweep, (STRANGER, address(ftok))));
            bool fixed_ = g.branchRan || g.cancelled;
            uint256 fa = ftok.balanceOf(address(inst));
            if (fixed_) {
                if (!ok1 || !ok2 || fa != 0) ghostWrong++;
                _strangerRefused(address(inst), abi.encodeCall(Eravon.sweep, (g.t.buyerPayout, tk)), "sweepRefused");
            } else {
                bool np = !ok1 && !ok2 && r1.length == 4 && bytes4(r1) == Eravon.NotParty.selector && r2.length == 4
                    && bytes4(r2) == Eravon.NotParty.selector;
                if (!np || fa != fb) ghostWrong++;
                _mark("ghost.sweepRefusedBeforeTheKeysAreKept");
            }
            if (fixed_ && fb != 0) _mark("ghost.foreignSwept");
            _mark("ghost.sweep");
        }
    }

    function ghostCount() external view returns (uint256) {
        return ghosts.length;
    }

    function ghostAt(uint256 k) external view returns (address) {
        return address(ghosts[k].inst);
    }

    /// s8: sweep of the settlement token's second address, sent by a party's current key, is refused (NotForeign)
    /// exactly where its transfer would lower the instance's balance of the settlement token, and lands with nothing
    /// moved otherwise; before the instance keeps the parties' keys it is NotParty (s10).
    function doSweepSecond(uint256 seed, uint256 whoSeed) external {
        (bool ok, uint256 i) = _pickAny(seed);
        if (!ok) return;
        C storage c = cases[i];
        address inst = address(c.inst);
        bool fixed_ = _ph(i) != 0 || c.branchRan;
        uint256 bal = tok.balanceOf(inst);
        address bp = c.t.buyerPayout;
        bool falls = fixed_ && bal != 0 && !btok.blocked(bp) && !btok.burns(bp);
        address[3] memory who = [bp, c.t.offer.sellerPayout, STRANGER];
        (bool s_, bytes memory ret) =
            _try(_submitter(c, _h(whoSeed) >> 8), inst, abi.encodeCall(Eravon.sweep, (who[_h(whoSeed) % 3], address(second))));
        if (!s_) _unnamed(A_SWEEP, ret);
        if (!fixed_) {
            if (s_ || ret.length != 4 || bytes4(ret) != Eravon.NotParty.selector) notForeignWrong++;
            _mark("sweep.secondEntryBeforeTheKeys");
        } else if (falls) {
            if (s_ || ret.length != 4 || bytes4(ret) != Eravon.NotForeign.selector) notForeignWrong++;
            _mark("sweep.secondEntryRefused");
        } else {
            if (!s_) notForeignWrong++;
            _mark("sweep.secondEntryNothingMoved");
        }
        if (tok.balanceOf(inst) != bal) notForeignWrong++;
    }

    /// terms/1 s5.1, s5.3 through the instance: a buyer's signed act under a blob of any shape (a valid signature's
    /// bytes cut, overwritten or over another digest; heads and lengths at and around their bounds) sent by a party's
    /// current key is refused by name, BadBuyerSig, in every phase where the instance knows the keys, NotParty before,
    /// and changes nothing; a stranger's is NotParty.
    function doBadBlob(uint256 seed, uint256 blobSeed) external {
        (bool ok, uint256 i) = _pickAny(seed);
        if (!ok) return;
        C storage c = cases[i];
        uint256 hb = _h(blobSeed);
        bytes memory blob;
        if (hb % 2 == 0) {
            // a real signature over another digest, then one byte overwritten or the bytes cut
            blob = _signB(digestOf(address(c.inst), keccak256(abi.encode(TH_CONFIRM ^ bytes32(hb)))));
            if ((hb >> 8) % 2 == 0) blob[(hb >> 16) % blob.length] = bytes1(uint8(hb >> 24));
            else assembly ("memory-safe") { mstore(blob, mod(shr(32, hb), mload(blob))) }
        } else {
            uint256 L = (hb >> 8) % 400;
            if (L == 65) L = 66;
            blob = new bytes(L);
            for (uint256 k = 0; k < L; k++) {
                blob[k] = bytes1(uint8(uint256(keccak256(abi.encode(hb, k)))));
            }
            if (L >= 64) {
                uint256 oa = (hb >> 20) % 3 == 0 ? L - 32 : ((hb >> 22) % 3 == 0 ? L - 31 : 192);
                uint256 os = (hb >> 24) % 3 == 0 ? L - 32 : ((hb >> 26) % 3 == 0 ? L + 1 : 224);
                assembly ("memory-safe") {
                    mstore(add(blob, 32), oa)
                    mstore(add(blob, 64), os)
                }
                if (oa + 32 <= L) {
                    uint256 la = (hb >> 28) % 2 == 0 ? L - oa - 32 : L - oa - 31;
                    assembly ("memory-safe") { mstore(add(add(blob, 32), oa), la) }
                }
            }
        }
        if (blob.length == 0) blob = new bytes(1);
        uint256 balBefore = tok.balanceOf(address(c.inst));
        uint8 phBefore = _ph(i);
        bytes memory data = (hb >> 40) % 2 == 0 ? abi.encodeCall(Eravon.confirmBySig, (blob)) : abi.encodeCall(Eravon.disputeBySig, (blob));
        (bool s_, bytes memory ret) = _try(_submitter(c, hb >> 48), address(c.inst), data);
        bytes4 want = _keysKnown(i) ? Eravon.BadBuyerSig.selector : Eravon.NotParty.selector;
        bool named = !s_ && ret.length == 4 && bytes4(ret) == want;
        if (!named) {
            blobUnnamed++;
            lastUnnamed = ret;
        }
        if (tok.balanceOf(address(c.inst)) != balBefore || _ph(i) != phBefore) blobUnnamed++;
        if ((hb >> 56) % 4 == 0) _strangerRefused(address(c.inst), data, "signedFormRefused");
        _mark(_keysKnown(i) ? "blob.refusedOnAKeyCase" : "blob.refusedBeforeTheKeys");
    }

    // ------------------------------------------------------------------
    // round 3: the draw's two clocks, the settlement against the clock, the
    // refund in a dispute, the signed refund in every phase, the feeds
    // against READ_GAS
    // ------------------------------------------------------------------

    /// A guild order opened now with its draw armed in this block and nobody drawing: where the draw's clocks
    /// start from (the first block after the arming is reachable only from here). One in two is drawn in the next
    /// block and its member declines there, so that the draw is armed again in this block with a seat behind it.
    function _freshArmed(uint256 seed) internal returns (bool, uint256) {
        if (cases.length >= MAX_CASES + MAX_FRESH + threeCount + poolCases) return (false, 0);
        noKeeper = true;
        _newOrder(_h(seed ^ 0xa3), 1 + seed % 1e24, seed >> 8, 0, true);
        uint256 i = cases.length - 1;
        if (_h(seed ^ 0xa4) % 2 == 0 && _pendingAny(i)) _rearm(i);
        noKeeper = false;
        return (_pendingAny(i), i);
    }

    /// The draw in the block after the arming, then the drawn member's decline (an unaccepted seat ended): s7
    /// arms the next draw in this block.
    function _rearm(uint256 i) internal {
        C storage c = cases[i];
        (address[] memory m, bytes32[] memory w) = _frozen(c);
        vm.roll(vm.getBlockNumber() + 1);
        if (!_exec(i, A_REDRAW, true, _drawerNow(c), false, abi.encodeCall(Eravon.redraw, (m, w)))) return;
        IGaleedVenue.Seat memory s = _lastSeat(i);
        if (s.accepted != 0 || s.state != 0) return;
        _judgeCall(i, A_RESIGN, true, s.judge, abi.encodeCall(Eravon.resign, (c.id, uint64(_seatCount(i) - 1), E, ENTRY)));
    }

    /// Rolls to `gap` blocks after the armed block (or stays, where the chain is already past it) and sets the
    /// clock: unchanged, the deadline's last second, one second past it, or an hour past it.
    function _edge(C storage c, uint256 gap, uint256 clock) internal {
        S memory st = _st(c.inst);
        uint256 target = uint256(st.drawBlock) + gap;
        if (vm.getBlockNumber() < target) vm.roll(target);
        uint256 dl = st.redrawDeadline;
        uint256 to = clock == 1 ? dl : (clock == 2 ? dl + 1 : (clock == 3 ? dl + 1 hours : 0));
        if (to > _now()) vm.warp(to);
    }

    /// s7's two clocks by redraw: at the first block after the arming whatever the clock, the second and the
    /// third, the last served block and past it, each before the deadline, at its last second, or past it (one
    /// second, an hour); a call s7 refuses is probed (NotPendingRedraw, nothing moved), one it admits is observed
    /// and checked against the two clocks from the pre-state (_checkClocks).
    function doDrawClocks(uint256 seed, uint256 mode) external {
        bool ok;
        uint256 i;
        if (_h(mode ^ 0xf3) % 2 == 0) (ok, i) = _freshArmed(seed);
        if (!ok || !_pendingAny(i)) (ok, i) = _pickWhere(seed, _pendingLive);
        if (!ok || !_pendingAny(i)) (ok, i) = _pickWhere(seed, _pendingAny);
        if (!ok || !_pendingAny(i)) return _progress(seed);
        C storage c = cases[i];
        uint256 hm = _h(mode);
        uint256[7] memory gaps = [uint256(1), 1, 2, 3, 255, 256, 257];
        uint256 gap = (hm >> 16) % 8 == 0 ? 300 : gaps[hm % 7];
        _edge(c, gap, (hm >> 8) % 4);
        (address[] memory m, bytes32[] memory w) = _frozen(c);
        if (_pendingDead(i)) {
            S memory st0 = _st(c.inst);
            (bool rok, bytes memory e) = _try(_drawerNow(c), address(c.inst), abi.encodeCall(Eravon.redraw, (m, w)));
            if (rok || e.length != 4 || bytes4(e) != Eravon.NotPendingRedraw.selector) refusedRedrawPassed++;
            S memory st1 = _st(c.inst);
            if (!st1.pending || st1.drawBlock != st0.drawBlock || st1.seats != st0.seats) drawOutcomeWrong++;
            _mark("drawClocks.refusedPastTheDeadline");
            return;
        }
        S memory st = _st(c.inst);
        uint256 gapNow = vm.getBlockNumber() - st.drawBlock;
        bool late = _now() > st.redrawDeadline;
        if (_exec(i, A_REDRAW, true, _drawerNow(c), false, abi.encodeCall(Eravon.redraw, (m, w)))) {
            _mark(
                gapNow > 256
                    ? "drawClocks.seedLost"
                    : (gapNow == 256 ? "drawClocks.lastServedBlock" : (gapNow == 1 && late ? "drawClocks.firstBlockPastTheDeadline" : "drawClocks.drawn"))
            );
        }
    }

    /// venue/1 s2.11, s7: advance on an armed draw at the first block after the arming, the second, the last
    /// served block and past it, before, at and past the deadline, naming the last ordinal, the next, the one
    /// after, or 2^64 - 1; checked by _checkClocks and _checkTerminal.
    function doAdvanceDraw(uint256 seed, uint256 mode) external {
        bool ok;
        uint256 i;
        if (_h(mode ^ 0xf3) % 2 == 0) (ok, i) = _freshArmed(seed);
        if (!ok || !_pendingAny(i)) (ok, i) = _pickWhere(seed, _pendingLive);
        if (!ok || !_pendingAny(i)) (ok, i) = _pickWhere(seed, _pendingAny);
        if (!ok || !_pendingAny(i)) return _progress(seed);
        C storage c = cases[i];
        uint256 hm = _h(mode);
        uint256[5] memory gaps = [uint256(1), 2, 256, 257, 300];
        _edge(c, gaps[hm % 5], (hm >> 8) % 4);
        uint256 n = _seatCount(i);
        uint256 pk = (hm >> 16) % 4;
        uint64 s = pk == 0 ? uint64(n == 0 ? 0 : n - 1) : (pk == 1 ? uint64(n) : (pk == 2 ? uint64(n + 1) : type(uint64).max));
        advSeat = s;
        if (_exec(i, A_ADVANCE, true, STRANGER, false, abi.encodeCall(Eravon.advance, (c.id, s)))) {
            _mark("advanceDraw");
            // the draw nobody made closes from any seat argument, an opened ordinal or not (venue/1 s2.11)
            if (snap.pending && _ph(i) == 8) _mark(s >= n ? "advance.drawDeadlineFromAnUnopenedSeat" : "advance.drawDeadline");
            _drawSoon(i);
        }
    }

    function _closedJudge(VmSafe.Log[] memory logs, address inst) internal pure returns (bool found, uint256 j) {
        for (uint256 k = 0; k < logs.length; k++) {
            if (logs[k].emitter == inst && logs[k].topics.length != 0 && logs[k].topics[0] == Eravon.Closed.selector) {
                (,,, j,) = abi.decode(logs[k].data, (uint8, uint256, uint256, uint256, uint256));
                return (true, j);
            }
        }
    }

    /// s8 against s5: a settlement's P goes where the clock's own outcome at that moment sends it. On a snapshot
    /// the clock is run (advance on the last ordinal); where it closes the case, the settlement's P to the judge
    /// equals the clock's; where nothing is due, P goes to the judge exactly when the last seat is accepted and in
    /// flight. The moment: now, the last second of the clock in force, one second past it, or an hour past.
    function doSettleVsClock(uint256 seed, uint256 mode) external {
        uint256 hm = _h(mode);
        if ((hm >> 44) % 3 != 0) return _progress(seed); // one call in three acts, so that cases live on
        bool ok;
        uint256 i;
        uint256 pick = (hm >> 40) % 3;
        if (pick == 0) (ok, i) = _pickWhere(seed, _canConfirm);
        if (pick == 1 || !ok) (ok, i) = _pickWhere(seed, _isLate);
        if (!ok) (ok, i) = _pickWhere(seed, _isOpen);
        if (!ok || !_isOpen(i)) return;
        C storage c = cases[i];
        S memory st = _st(c.inst);
        uint256 end_ = st.pending ? st.redrawDeadline : (st.seats != 0 ? _lastSeat(i).windowEnd : 0);
        uint256 wsel = (hm >> 50) % 4 == 0 ? 0 : 1 + hm % 3;
        uint256 to = wsel == 1 ? end_ : (wsel == 2 ? end_ + 1 : (wsel == 3 ? end_ + 1 hours : 0));
        if (to > _now()) vm.warp(to);
        if ((hm >> 8) % 2 == 0) vm.roll(vm.getBlockNumber() + 2);
        st = _st(c.inst);
        bool inFlight;
        if (!st.pending && st.seats != 0) {
            IGaleedVenue.Seat memory L = _lastSeat(i);
            inFlight = L.accepted != 0 && L.state <= 1;
        }
        uint64 last = uint64(st.seats == 0 ? 0 : st.seats - 1);
        // the clock's own outcome at this moment
        uint256 snapId = vm.snapshotState();
        vm.recordLogs();
        vm.prank(STRANGER);
        (bool aok,) = address(c.inst).call{gas: TX_GAS}(abi.encodeCall(Eravon.advance, (c.id, last)));
        (bool clockClosed, uint256 clockJ) = _closedJudge(vm.getRecordedLogs(), address(c.inst));
        vm.revertToStateAndDelete(snapId);
        if (!aok) settleVsClockWrong++;
        // the settlement at the same moment, by either party with both consents
        uint16 b = uint16((hm >> 24) % 10001);
        uint256 exp = _now() + 1 days;
        bytes32 d = digestOf(address(c.inst), keccak256(abi.encode(TH_SETTLE, uint256(b), exp)));
        bytes memory sigS = _settleSlot(c, true, false, (hm >> 16) % 3, d);
        bytes memory sigB = _settleSlot(c, false, false, (hm >> 20) % 3, d);
        if (!_exec(i, A_SETTLE, true, _submitter(c, uint256(d) >> 3), false, abi.encodeCall(Eravon.settle, (b, exp, sigB, sigS)))) return;
        uint256 want = clockClosed ? clockJ : (inFlight ? c.p : 0);
        if (!lastClosedSeen || lastClosedJ != want) settleVsClockWrong++;
        _mark(clockClosed ? (clockJ != 0 ? "settleVsClock.clockDuePaysTheJudge" : "settleVsClock.clockDuePaysTheSeller") : "settleVsClock.clockNotDue");
    }

    /// s8: the seller's own refund during a dispute: within T3, its last second included, refused (WrongPhase),
    /// nothing moved; one second past T3 the plain refund (observed: the amount and the buyer's fee to the buyer,
    /// P and the deposit to the seller, nothing to the judge).
    function doRefundAtT3(uint256 seed, uint256 mode) external {
        uint256 hm = _h(mode);
        if ((hm >> 40) % 2 == 0) return _progress(seed);
        (bool ok, uint256 i) = _pickWhere(seed, _canRule);
        if (!ok || !_canRule(i)) return _progress(seed);
        C storage c = cases[i];
        uint256 end_ = _lastSeat(i).windowEnd;
        uint256 w = hm % 4;
        uint256 to = w == 1 ? end_ : (w >= 2 ? end_ + 1 : 0);
        if (to > _now()) vm.warp(to);
        if (_now() <= end_) {
            uint256 bal = tok.balanceOf(address(c.inst));
            (bool okr, bytes memory er) = _try(c.t.offer.sellerKey, address(c.inst), abi.encodeCall(Eravon.refund, ()));
            if (okr || er.length != 4 || bytes4(er) != Eravon.WrongPhase.selector || _ph(i) != 3 || tok.balanceOf(address(c.inst)) != bal) {
                disputeRefundWrong++;
            }
            _mark("refundAtT3.refusedWithinT3");
            return;
        }
        if (_exec(i, A_REFUND, true, c.t.offer.sellerKey, false, abi.encodeCall(Eravon.refund, ()))) _mark("refundAtT3.plainPastT3");
    }

    /// s9, s10: refundBySig in every phase, sent by either party's current key, the sender and then the signature
    /// tested first: before the instance keeps the parties' keys NotParty for every sender; after a cancel or a run
    /// of the expiry branch, which keep both keys, the seller's valid signature WrongPhase; on a company seller's case
    /// BadSellerSig (a key's signature is not its consent); a junk signature while Disputed BadSellerSig; the seller's
    /// valid signature WrongPhase while Disputed, within T3 and past it (one disputed probe in two is moved past T3's
    /// end, where the seller's own refund is no longer refused), and at a terminal; while Funded or Claimed it lands
    /// as the plain refund (observed: P to the judge while Claimed under a seated seat, to the seller otherwise); a
    /// stranger's submission is NotParty in every phase.
    function doRefundSig(uint256 seed, uint256 mode) external {
        bool ok;
        uint256 i;
        uint256 pick = _h(mode ^ 0x5e) % 3;
        if (pick == 0) (ok, i) = _pickWhere(seed, _canRule);
        if (pick == 1 || !ok) (ok, i) = _pickWhere(seed, _isOpen);
        if (!ok) (ok, i) = _pickAny(seed);
        if (!ok) return;
        C storage c = cases[i];
        uint8 ph = _ph(i);
        if (ph == 3 && (_h(mode) >> 24) % 2 == 0) {
            uint256 past = uint256(_lastSeat(i).windowEnd) + 1;
            if (past > _now()) vm.warp(past);
        }
        bool pastT3 = ph == 3 && _now() > _lastSeat(i).windowEnd;
        bytes32 dg = digestOf(address(c.inst), keccak256(abi.encode(TH_REFUND)));
        bytes memory sg = _signS(dg);
        bool keySeller = c.sellerKind == 0;
        address from = _submitter(c, _h(mode) >> 32);
        _strangerRefused(address(c.inst), abi.encodeCall(Eravon.refundBySig, (sg)), "signedFormRefused");
        if (keySeller && (ph == 1 || ph == 2)) {
            if (_h(mode) % 4 != 0) return _progress(seed); // one in four lands, so that cases live on
            refundSigned = true;
            if (_exec(i, A_REFUND, true, from, false, abi.encodeCall(Eravon.refundBySig, (sg)))) {
                _mark("refundSig.landsOutsideADispute");
            }
            refundSigned = false;
            return;
        }
        // junk while Disputed: 65 zero bytes, or a sound signature by another key
        bool junk = ph == 3 && keySeller && (_h(mode) >> 8) % 2 == 0;
        if (junk) sg = (_h(mode) >> 16) % 2 == 0 ? new bytes(65) : _signB(dg);
        // s10: an instance that holds no case and ran no branch knows neither key: NotParty for every sender; past
        // that the signature is read before the phase
        bytes4 want = !_keysKnown(i)
            ? Eravon.NotParty.selector
            : ((!keySeller || junk) ? Eravon.BadSellerSig.selector : Eravon.WrongPhase.selector);
        uint256 bal = tok.balanceOf(address(c.inst));
        (bool s_, bytes memory ret) = _try(from, address(c.inst), abi.encodeCall(Eravon.refundBySig, (sg)));
        if (s_ || ret.length != 4 || bytes4(ret) != want || _ph(i) != ph || tok.balanceOf(address(c.inst)) != bal) refundSigWrong++;
        _mark(
            ph == 3
                ? (junk ? "refundSig.junkRefusedInDispute" : (pastT3 ? "refundSig.refusedInDisputePastT3" : "refundSig.refusedInDispute"))
                : (ph == 0 ? "refundSig.refusedBeforeACase" : "refundSig.refusedOtherwise")
        );
    }

    /// s3 against READ_GAS: an order priced in a reference unit whose feeds or unit misbehave, opened under the
    /// transaction cap: fresh feeds, a costly but bounded read, and return bombs within READ_GAS open the case
    /// at the model's fee; two feeds burning all they are handed, two decimals() burning, a bomb past READ_GAS, a
    /// codeless feed, a stale clock, a short answer, and a burning unit beside burning feeds answer RateStale;
    /// nothing reverts without a name.
    function doFeedInit(uint256 flags) external {
        uint256 hf = _h(flags);
        if (fcases.length >= MAX_FEED_ORDERS || (hf >> 200) % 2 == 0) return _progress(flags);
        uint8 kind = uint8(hf % 10);
        InvGasFeed fT = new InvGasFeed();
        InvGasFeed fU = new InvGasFeed();
        InvGasFeed unit = new InvGasFeed();
        fT.set(2000e8, _now(), 8);
        fU.set(1e8, _now(), 8);
        unit.set(1, 0, 6);
        address feedT = address(fT);
        bool fresh = kind == 0 || kind == 3 || kind == 5;
        if (kind == 1) {
            // both feeds burn all they are handed
            fT.setGas(type(uint256).max, 0, 0, 0, 0);
            fU.setGas(type(uint256).max, 0, 0, 0, 0);
        } else if (kind == 2) {
            // both feeds answer, both decimals() burn
            fT.setGas(0, type(uint256).max, 0, 0, 0);
            fU.setGas(0, type(uint256).max, 0, 0, 0);
        } else if (kind == 3) {
            // return bombs within READ_GAS on all five reads
            fT.setGas(0, 0, 9000, 9000, 0);
            fU.setGas(0, 0, 9000, 9000, 0);
            unit.setGas(0, 0, 0, 9000, 0);
        } else if (kind == 4) {
            fU.setGas(0, 0, 20000, 0, 0); // a bomb past what READ_GAS pays for
        } else if (kind == 5) {
            fT.setGas(150_000, 100_000, 0, 0, 0); // costly reads within READ_GAS
        } else if (kind == 6) {
            feedT = address(uint160(uint256(keccak256(abi.encode("no feed", hf)))));
        } else if (kind == 7) {
            // the unit's decimals() burns, and so does every feed read
            unit.setGas(0, type(uint256).max, 0, 0, 0);
            fT.setGas(0, type(uint256).max, 0, 0, 0);
            fU.setGas(type(uint256).max, 0, 0, 0, 0);
        } else if (kind == 8) {
            fT.set(2000e8, _now() - 2 days, 8);
        } else if (kind == 9) {
            fU.setGas(0, 0, 0, 0, 160); // a round answer of 159 bytes
        }
        Q memory q = defaultQuote(address(ptok), address(0), true);
        q.feeUnit = address(unit);
        q.feedToken = feedT;
        q.feedUnit = address(fU);
        q.maxStale = 1 days;
        q.feeMin = 5e6;
        q.feeMax = 500e6;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(namedJudges[(hf >> 8) % 3], AMANAH, quote, 1 ether);
        t.offer.token = address(ptok);
        t.offer.termsHash = keccak256(abi.encode("pricing terms", fcases.length, hf));
        Eravon inst = deployProxy(t);
        ptok.mint(address(inst), 1 ether);
        // x' = x * 1e8 * 10^(8 + 18) / (2000e8 * 10^(8 + 6)) = x * 5e8: the bounds do not bind, lf = 10% of A
        uint256 d = 1e16 + 1e17;
        ptok.mint(SELLER, d);
        uint256 al = ptok.allowance(SELLER, address(logic));
        vm.prank(SELLER);
        ptok.approve(address(logic), al + d);
        bytes memory data = abi.encodeCall(Eravon.init, (t, quote, _buyerSig(t), _sellerSig(t), new address[](0), new bytes32[](0)));
        vm.prank(_cur(SELLER));
        (bool ok, bytes memory ret) = address(inst).call{gas: TX_GAS}(data);
        if (!ok && !_isNamed(ret)) {
            feedInitUnnamed++;
            lastFeedErr = ret;
        }
        if (ok != fresh || (!ok && (ret.length != 4 || bytes4(ret) != Eravon.RateStale.selector))) {
            feedInitWrong++;
            lastFeedErr = ret;
        }
        if (ok) {
            (uint256 lfQ,,,,) = inst.feeQuoted();
            if (lfQ != 1e17 || depositHeld(inst) != d) feedInitWrong++;
        }
        fcases.push(F(inst, kind, ok));
        _mark(string(abi.encodePacked("feed.kind", vm.toString(kind), ok ? ".opened" : ".rateStale")));
    }

    // ------------------------------------------------------------------
    // round 4: answers of every length, the refund in a dispute, the stale signed refund
    // ------------------------------------------------------------------

    /// The campaign's answers for a while: the token's transfer answers 0 to 600,000 bytes whose first word is the
    /// truth, false or garbage (the balance moved or not as the payee allows); its balanceOf and decimals() answer
    /// 32 to 4,096 bytes; its transferFrom no data or one word 1 (the only answers a pull takes); the two feeds'
    /// rounds 160 to 240,000 bytes and their decimals() 32 to 240,000. Every model of the campaign holds under any of
    /// them: no length changes a decision or an outcome. One call in four restores the plain answers.
    function doShape(uint256 seed) external {
        uint256 h = _h(seed ^ 0x5ba9e);
        if (h % 4 == 0) {
            btok.setShapes(32, 0, 32, 0, 32, 32);
            feedTok.setLens(160, 32);
            feedUsd.setLens(160, 32);
            _mark("shape.plain");
            return;
        }
        // the pushes' answers up to a bomb the allowance pays for (600,000 bytes, about 743,000 gas of the
        // callee's own), the feeds' up to one READ_GAS pays for (240,000 bytes)
        uint256[10] memory lt = [uint256(0), 31, 32, 33, 159, 160, 161, 4096, 65536, 600_000];
        uint256[6] memory lb = [uint256(32), 33, 159, 160, 161, 4096];
        uint256[4] memory ld = [uint256(32), 33, 161, 4096];
        uint256[5] memory lr = [uint256(160), 161, 4096, 160, 240_000];
        uint256[5] memory lfd = [uint256(32), 33, 4096, 32, 240_000];
        btok.setShapes(lt[(h >> 8) % 10], uint8((h >> 16) % 3), (h >> 24) % 2 == 0 ? 0 : 32, 0, lb[(h >> 32) % 6], ld[(h >> 40) % 4]);
        feedTok.setLens(lr[(h >> 48) % 5], lfd[(h >> 56) % 5]);
        feedUsd.setLens(lr[(h >> 64) % 5], lfd[(h >> 72) % 5]);
        _mark("shape.set");
    }

    struct PR {
        bool ok;
        bytes32 err;
        bytes32 outcome;
        bytes32 closed;
        uint256 gas;
        uint256 answered;
        uint256 cost;
        uint256 maxEntry;
    }

    /// Everything an action could move on a case: the phase, the held sums, the ledger, the instance's and the
    /// payees' balances and credits, and every seat's claimable.
    function _outcome(uint256 i) internal view returns (bytes32) {
        C storage c = cases[i];
        S memory st = _st(c.inst);
        address[3] memory pp = [c.t.buyerPayout, c.t.offer.sellerPayout, c.t.offer.sellerFund];
        bytes memory a = abi.encode(st.ph, st.seats, st.premium, st.deposit, st.lfHeld, st.unpaid, st.pending, tok.balanceOf(address(c.inst)));
        bytes memory b = abi.encode(
            tok.balanceOf(pp[0]),
            tok.balanceOf(pp[1]),
            tok.balanceOf(pp[2]),
            c.inst.owed(pp[0], address(tok)),
            c.inst.owed(pp[1], address(tok)),
            c.inst.owed(pp[2], address(tok))
        );
        bytes32 j;
        if (st.ph != 0) {
            for (uint256 s = 0; s < st.seats; s++) {
                address jj = c.inst.seatOf(c.id, uint64(s)).judge;
                (address po, uint256 cl) = c.inst.accountOf(jj, address(tok));
                j = keccak256(abi.encode(j, jj, po, cl, po == address(0) ? 0 : tok.balanceOf(po), c.inst.seatOf(c.id, uint64(s)).state));
            }
        }
        return keccak256(abi.encodePacked(a, b, j));
    }

    /// A call that copies no return data while it is measured (so that this frame's memory is no part of the
    /// measure); the answer is fetched after. A company's act goes through its exec; a key's is sent by the current
    /// key of its record (s9).
    function _rawCall(address who, bool judge, address inst, bytes memory data) internal returns (bool ok, uint256 used, bytes memory ret) {
        address target = inst;
        who = _cur(who);
        if (who == coSeller || who == coBuyer) {
            target = who;
            data = abi.encodeCall(Company.exec, (inst, data));
        } else if (judge) {
            vm.prank(who, who);
        } else {
            vm.prank(who);
        }
        uint256 g = TX_GAS;
        uint256 g0;
        uint256 g1;
        assembly ("memory-safe") {
            g0 := gas()
            ok := call(g, target, 0, add(data, 32), mload(data), 0, 0)
            g1 := gas()
        }
        used = g0 - g1;
        ret = new bytes(_rds());
        assembly ("memory-safe") {
            returndatacopy(add(ret, 32), 0, mload(ret))
        }
    }

    function _rds() internal pure returns (uint256 n) {
        assembly ("memory-safe") {
            n := returndatasize()
        }
    }

    function _closedOf(VmSafe.Log[] memory logs, address emitter) internal pure returns (bytes32 h) {
        for (uint256 k = 0; k < logs.length; k++) {
            if (logs[k].emitter == emitter && logs[k].topics.length != 0 && logs[k].topics[0] == Eravon.Closed.selector) {
                h = keccak256(logs[k].data);
            }
        }
    }

    /// One run of an action under the given transfer and balance answers, on a snapshot it reverts.
    function _probeRun(uint256 i, address who, bool judge, bytes memory data, uint256 lt, uint8 vt, uint256 lb)
        internal
        returns (PR memory r)
    {
        C storage c = cases[i];
        uint256 sid = vm.snapshotState();
        btok.setShapes(lt, vt, btok.lenF(), btok.valF(), lb, btok.lenD());
        {
            uint256 s2 = vm.snapshotState();
            r.cost = noCopy.gasOf(address(btok), abi.encodeWithSelector(BlockableToken.transfer.selector, address(0xCAFE), uint256(0)));
            vm.revertToStateAndDelete(s2);
        }
        btok.resetPushGas();
        vm.recordLogs();
        bytes memory ret;
        (r.ok, r.gas, ret) = _rawCall(who, judge, address(c.inst), data);
        r.err = keccak256(ret);
        r.closed = _closedOf(vm.getRecordedLogs(), address(c.inst));
        r.outcome = _outcome(i);
        r.answered = btok.answered();
        r.maxEntry = btok.maxPushGas();
        vm.revertToStateAndDelete(sid);
    }

    /// The action a probe runs on the case: whatever its phase admits (init or cancel before a case, advance where
    /// due, the seller's refund, the buyer's confirm, the judge's ruling inside T3, sweep after the terminal).
    function _probeCall(uint256 i, uint256 hm) internal returns (address who, bool judge, bytes memory data, uint8 act) {
        C storage c = cases[i];
        uint8 ph = _ph(i);
        if (ph == 0) {
            if (!_dead(c) && (hm >> 4) % 2 == 0) {
                (address from, bytes memory d) = _cancelCall(i, false);
                return (from, false, d, A_CANCEL);
            }
            if (c.badInts && !_dead(c)) return (address(0), false, "", 0);
            (who, data) = _initCall(i);
            return (who, false, data, A_INIT);
        }
        if (ph >= 4) {
            address[4] memory w = [c.t.buyerPayout, c.t.offer.sellerPayout, c.t.offer.sellerFund, _lastJudgeOr(i, STRANGER)];
            return (_submitter(c, hm >> 20), false, abi.encodeCall(Eravon.sweep, (w[(hm >> 4) % 4], address(tok))), A_SWEEP);
        }
        if (_due(i) && (hm >> 4) % 2 == 0) {
            uint256 n = _seatCount(i);
            return (STRANGER, false, abi.encodeCall(Eravon.advance, (c.id, uint64(n == 0 ? 0 : n - 1))), A_ADVANCE);
        }
        if (ph == 3 && _canRule(i)) {
            (uint64 s, address j) = _lastOrStranger(i);
            uint64 rg = uint64((hm >> 8) % c.t.offer.ladder.length);
            return (j, true, abi.encodeCall(Eravon.rule, (c.id, s, E, rg, ENTRY, "")), A_RULE);
        }
        if (ph == 2 && (hm >> 12) % 2 == 0) {
            (address b, bytes memory d) =
                _buyerCall(c, TH_CONFIRM, (hm >> 13) % 2 == 0, abi.encodeCall(Eravon.confirm, ()), Eravon.confirmBySig.selector);
            return (b, false, d, A_CONFIRM);
        }
        return (c.t.offer.sellerKey, false, abi.encodeCall(Eravon.refund, ()), A_REFUND);
    }

    /// The same action on the same state under two shapes of the token's answers (the campaign's and another
    /// transfer answer, one probe in three another balance answer too): the same success or the same fault, the
    /// same outcome to the wei, the same Closed, the same pushes; where only the transfer answer differs and no
    /// payee burns, the caller's gas differs by exactly the answered pushes' own cost of their answers (a no-copy
    /// reader prices it), so the caller pays nothing for a length; every push of the standard's own under
    /// PUSH_GAS. One probe in five instead sends a pull an answer the text refuses (TransferFailed, nothing moved).
    function doLenProbe(uint256 seed, uint256 mode) external {
        (bool ok, uint256 i) = _pickAny(seed);
        if (!ok) return;
        uint256 hm = _h(mode ^ 0x1e9);
        if ((hm >> 200) % 5 == 0) return _pullProbe(i, hm);
        (address who, bool judge, bytes memory data, uint8 act) = _probeCall(i, hm);
        if ((hm >> 210) % 6 == 0) {
            // a call that fails, after the token has answered where it can: a dispute whose fee the buyer's key has
            // not allowed (the pull refused), or a stranger's refund
            if (_canDispute(i) && _lastSeat(i).state == 0 && cases[i].stray < cases[i].lf && cases[i].buyerKind == 0) {
                who = BUYER;
                data = abi.encodeCall(Eravon.dispute, ());
                vm.prank(BUYER);
                tok.approve(address(cases[i].inst), 0);
            } else {
                who = STRANGER;
                data = abi.encodeCall(Eravon.refund, ());
            }
            judge = false;
            act = A_REFUND;
        }
        if (data.length == 0) return;
        uint256[10] memory lt = [uint256(0), 31, 32, 33, 159, 160, 161, 4096, 65536, 600_000];
        uint256[6] memory lb = [uint256(32), 33, 159, 160, 161, 4096];
        uint256 lbB = (hm >> 16) % 3 == 0 ? lb[(hm >> 20) % 6] : btok.lenB();
        PR memory a = _probeRun(i, who, judge, data, btok.lenT(), btok.valT(), btok.lenB());
        PR memory b = _probeRun(i, who, judge, data, lt[hm % 10], uint8((hm >> 8) % 3), lbB);
        if (a.ok != b.ok || a.err != b.err || a.outcome != b.outcome || a.closed != b.closed || a.answered != b.answered) {
            lenOutcomeWrong++;
        }
        bool all = act == A_SWEEP;
        if (!all && (a.maxEntry > PUSH_GAS || b.maxEntry > PUSH_GAS)) pushOverAllowance++;
        int256 own = int256(b.gas) - int256(a.gas) - int256(b.answered) * (int256(b.cost) - int256(a.cost));
        if (lbB == btok.lenB()) {
            if (!_burnerOn(i) && own != 0) {
                pushLenGasWrong++;
                lastLenGasOwn = own;
            }
            if (b.answered != 0 && !_burnerOn(i)) _mark("lenProbe.gasExact");
        } else {
            uint256 x = own < 0 ? uint256(-own) : uint256(own);
            if (x > balanceCopyMax) balanceCopyMax = x;
            _mark("lenProbe.balanceLen");
        }
        _mark(a.ok ? "lenProbe.sameOutcome" : "lenProbe.sameFault");
    }

    function _burnerOn(uint256 i) internal view returns (bool) {
        C storage c = cases[i];
        if (btok.burns(c.t.buyerPayout) || btok.burns(c.t.offer.sellerPayout) || btok.burns(c.t.offer.sellerFund)) return true;
        address[] memory p = judgePayouts;
        for (uint256 k = 0; k < p.length; k++) {
            if (btok.burns(p[k])) return true;
        }
        return false;
    }

    /// s4 step 7, s8: a pull (the deposit's at init, the fee's at a dispute) answered with anything but no data or
    /// one 32-byte word equal to 1 is TransferFailed by name and nothing moves, the balance having moved or not.
    /// A case the pull probe can run on: an unopened live order whose fund admits the pull of a nonzero D, or a
    /// Claimed case in its dispute window whose fee is pulled from the buyer's key.
    function _pullProbeable(uint256 i) internal view returns (bool) {
        C storage c = cases[i];
        uint8 ph = _ph(i);
        if (ph == 0) return !_dead(c) && !c.badInts && c.d != 0 && _fundFault(c) == 0;
        return _canDispute(i) && _lastSeat(i).state == 0 && c.stray < c.lf;
    }

    function _pullProbe(uint256 i, uint256 hm) internal {
        if (!_pullProbeable(i)) {
            (bool okp, uint256 j) = _pickWhere(hm ^ 0x9b, _pullProbeable);
            if (okp) i = j;
        }
        C storage c = cases[i];
        uint8 ph = _ph(i);
        uint256[7] memory badLen = [uint256(31), 33, 64, 160, 161, 32, 32];
        uint8[7] memory badVal = [uint8(0), 0, 0, 0, 0, 1, 2];
        uint256 k = (hm >> 100) % 7;
        address who;
        bytes memory data;
        if (ph == 0 && !_dead(c) && !c.badInts && c.d != 0 && _fundFault(c) == 0) {
            (who, data) = _initCall(i);
        } else if (_canDispute(i) && _lastSeat(i).state == 0 && c.stray < c.lf) {
            data = abi.encodeCall(Eravon.dispute, ());
            who = c.t.buyerKey;
        } else {
            return;
        }
        uint256 sid = vm.snapshotState();
        if (ph == 2) _fundBuyerKey(c, c.lf);
        bytes32 before = _outcome(i);
        btok.setShapes(btok.lenT(), btok.valT(), badLen[k], badVal[k], btok.lenB(), btok.lenD());
        (bool ok, , bytes memory ret) = _rawCall(who, false, address(c.inst), data);
        bytes32 after_ = _outcome(i);
        vm.revertToStateAndDelete(sid);
        if (ok || ret.length != 4 || bytes4(ret) != Eravon.TransferFailed.selector || after_ != before) pullShapeWrong++;
        _mark("lenProbe.pullRefused");
    }

    function _isDisputed(uint256 i) internal view returns (bool) {
        return _ph(i) == 3;
    }

    /// s5, s8, s9: a refund during a dispute in both forms at a second of T3 (its first, its last, the one before
    /// its last, or any) and past it, on a snapshot: inside T3 both forms are WrongPhase and nothing moves; past
    /// it the signed form is still WrongPhase (BadSellerSig for a seller with no key) and the direct one is the
    /// plain refund: A, the strays and the fee to the buyer, P and the LF to the seller, nothing to the judge.
    function doRefundInDispute(uint256 seed, uint256 when) external {
        (bool ok, uint256 i) = _pickWhere(seed, _isDisputed);
        if (!ok || _ph(i) != 3) return;
        C storage c = cases[i];
        IGaleedVenue.Seat memory L = _lastSeat(i);
        uint256 hw = _h(when);
        uint256 t0 = _now();
        uint256 at;
        bool inT3 = hw % 2 == 0 && t0 <= L.windowEnd;
        if (inT3) {
            uint256 span = uint256(L.windowEnd) - t0;
            uint256 k = (hw >> 8) % 4;
            if (k == 0) at = t0;
            else if (k == 1) at = L.windowEnd;
            else if (k == 2) at = span == 0 ? t0 : uint256(L.windowEnd) - 1;
            else at = t0 + (hw >> 16) % (span + 1);
        } else {
            at = (t0 > L.windowEnd ? t0 : uint256(L.windowEnd) + 1) + ((hw >> 24) % 3 == 0 ? 0 : (hw >> 32) % 30 days);
        }
        S memory st = _st(c.inst);
        uint256 sid = vm.snapshotState();
        vm.warp(at);
        bytes32 o0 = _outcome(i);
        bytes memory sg = _signS(digestOf(address(c.inst), keccak256(abi.encode(TH_REFUND))));
        (bool sOk,, bytes memory sRet) = _rawCall(_submitter(c, hw >> 40), false, address(c.inst), abi.encodeCall(Eravon.refundBySig, (sg)));
        bytes32 o1 = _outcome(i);
        (, uint256 jc0) = c.inst.accountOf(L.judge, address(tok));
        vm.recordLogs();
        (bool dOk,, bytes memory dRet) = _rawCall(c.t.offer.sellerKey, false, address(c.inst), abi.encodeCall(Eravon.refund, ()));
        VmSafe.Log[] memory logs = vm.getRecordedLogs();
        bytes32 o2 = _outcome(i);
        (, uint256 jc1) = c.inst.accountOf(L.judge, address(tok));
        vm.revertToStateAndDelete(sid);
        bytes4 wantSig = c.sellerKind == 1 ? Eravon.BadSellerSig.selector : Eravon.WrongPhase.selector;
        bool sigRight = !sOk && sRet.length == 4 && bytes4(sRet) == wantSig && o1 == o0;
        if (inT3) {
            if (!sigRight || dOk || dRet.length != 4 || bytes4(dRet) != Eravon.WrongPhase.selector || o2 != o0) refundInT3Wrong++;
            _mark(at == L.windowEnd ? "refundT3.refusedAtTheLastSecond" : "refundT3.refusedBoth");
        } else {
            bytes32 want = keccak256(abi.encode(uint8(0), c.a + c.stray + st.lfHeld, c.p + c.lf, uint256(0), uint256(0)));
            if (!sigRight || !dOk || _closedOf(logs, address(c.inst)) != want || jc1 != jc0) refundPastT3Wrong++;
            _mark("refundT3.pastPlain");
        }
    }

    /// The seller's Refund() signature, or a bad one (the buyer's, another domain's, the high-s twin, a cut or a
    /// random blob), submitted by either party's current key in whatever phase the case stands: before the instance
    /// keeps the parties' keys NotParty for every sender and every signature (s10); a company seller has no key
    /// (BadSellerSig); a bad blob is BadSellerSig everywhere, before the phase; the valid one is WrongPhase in a
    /// dispute, at a terminal, after a cancel and after a run of the expiry branch and, while Funded or Claimed,
    /// lands exactly as the seller's own refund does (on a snapshot; one in three for real, observed).
    function doStaleSig(uint256 seed, uint256 kind) external {
        uint256 hk = _h(kind ^ 0x51a);
        bool ok;
        uint256 i;
        uint256 pick = (hk >> 30) % 3;
        if (pick == 0) (ok, i) = _pickWhere(seed, _isDisputedKeySeller);
        else if (pick == 1) (ok, i) = _pickWhere(seed, _isOpen);
        if (!ok) (ok, i) = _pickAny(seed);
        if (!ok) return;
        C storage c = cases[i];
        uint256 k = hk % 9;
        bytes32 d = digestOf(address(c.inst), keccak256(abi.encode(TH_REFUND)));
        bytes memory sg;
        if (k <= 2) {
            sg = _signS(d);
        } else if (k == 3) {
            sg = _signB(d);
        } else if (k == 4) {
            address other = cases.length > 1 ? address(cases[(i + 1) % cases.length].inst) : address(logic);
            sg = _signS(digestOf(other, keccak256(abi.encode(TH_REFUND))));
        } else if (k == 5) {
            (uint8 v, bytes32 rr, bytes32 ss) = vm.sign(sellerPk, d);
            sg = abi.encodePacked(rr, bytes32(0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141 - uint256(ss)), v == 27 ? uint8(28) : uint8(27));
        } else if (k == 6) {
            sg = new bytes([uint256(0), 64, 66][(hk >> 8) % 3]);
        } else if (k == 7) {
            sg = new bytes(65);
            for (uint256 b = 0; b < 65; b++) {
                sg[b] = bytes1(uint8(uint256(keccak256(abi.encode(hk, b)))));
            }
        } else {
            sg = _signS(digestOf(address(logic), keccak256(abi.encode(TH_REFUND))));
        }
        uint8 ph = _ph(i);
        bool keyStored = _keysKnown(i);
        bool valid = k <= 2 && c.sellerKind == 0;
        bytes memory data = abi.encodeCall(Eravon.refundBySig, (sg));
        address from = _submitter(c, hk >> 40);
        if (keyStored && valid && (ph == 1 || ph == 2)) {
            // the signed form lands as the direct one would
            PR memory a = _probeRun(i, c.t.offer.sellerKey, false, abi.encodeCall(Eravon.refund, ()), btok.lenT(), btok.valT(), btok.lenB());
            PR memory b = _probeRun(i, from, false, data, btok.lenT(), btok.valT(), btok.lenB());
            if (!a.ok || !b.ok || a.outcome != b.outcome || a.closed != b.closed) staleSigWrong++;
            _mark("staleSig.lands");
            if ((hk >> 20) % 2 == 0) {
                refundSigned = true;
                if (_exec(i, A_REFUND, true, from, false, data)) _mark("staleSig.landedForReal");
                refundSigned = false;
            }
            return;
        }
        bytes4 want = !keyStored ? Eravon.NotParty.selector : (!valid ? Eravon.BadSellerSig.selector : Eravon.WrongPhase.selector);
        PR memory r = _probeRun(i, from, false, data, btok.lenT(), btok.valT(), btok.lenB());
        bytes32 before = _outcome(i);
        if (r.ok || r.err != keccak256(abi.encodeWithSelector(want)) || r.outcome != before) staleSigWrong++;
        if (!keyStored) _mark("staleSig.noKey");
        else if (c.sellerKind == 1) _mark("staleSig.company");
        else if (!valid) _mark("staleSig.bad");
        else if (ph == 3) _mark("staleSig.inDispute");
        else if (ph == 0) _mark("staleSig.afterTheBranch");
        else if (!c.opened) _mark("staleSig.afterACancel");
        else _mark("staleSig.terminal");
    }

    // ------------------------------------------------------------------
    // round 5: two acts in one block in both orders, reorganised histories, the three-seat guild walk, hostile
    // foreign tokens, odd decimals, feeds that flip between reads, EIP-7702 delegated accounts, griefing
    // ------------------------------------------------------------------

    uint8 internal constant P_CONFIRM = 0;
    uint8 internal constant P_DISPUTE = 1;
    uint8 internal constant P_REFUND = 2;
    uint8 internal constant P_REFUNDSIG = 3;
    uint8 internal constant P_SETTLE = 4;
    uint8 internal constant P_CLAIM = 5;
    uint8 internal constant P_ACCEPT = 6;
    uint8 internal constant P_RULE = 7;
    uint8 internal constant P_RESIGN = 8;
    uint8 internal constant P_ADVANCE = 9;
    uint8 internal constant P_REDRAW = 10;
    uint8 internal constant P_COMMIT = 11;
    uint8 internal constant P_SWEEP = 12;
    uint8 internal constant P_SETPAYOUT = 13;
    uint8 internal constant P_WITHDRAW = 14;
    uint8 internal constant P_STRAY = 15;

    struct Step {
        uint8 code;
        uint8 act;
        address who;
        bool judge;
        bool signed_;
        uint64 seat;
        uint256 v;
        bytes data;
    }

    struct Run {
        bool okA;
        bool okB;
        bytes4 errB;
        uint8 phA;
        uint8 phEnd;
        uint256 bhA;
        uint256 sA;
        uint256 jA;
        uint256 bh;
        uint256 s;
        uint256 j;
        uint256 viol;
        bool moved;
    }

    function _r5(bytes memory what) internal {
        lastR5 = what;
    }

    /// The three sides' totals for case i: the buyer's (its payout's balance and credit, its key's balance, and what
    /// the instance holds unrecorded or for the case), the seller's (its payout and key, balance and credit, where
    /// they are not the buyer's addresses) and the judges' (every seat's claimable and its payout's balance).
    function _tot(uint256 i) internal view returns (uint256 bh, uint256 s, uint256 j) {
        C storage c = cases[i];
        address inst = address(c.inst);
        address bp = c.t.buyerPayout;
        address bk = c.t.buyerKey;
        address sp = c.t.offer.sellerPayout;
        address sk = c.t.offer.sellerKey;
        address t = address(tok);
        uint256 held = tok.balanceOf(inst);
        uint256 u = _st(c.inst).unpaid;
        bh = tok.balanceOf(bp) + c.inst.owed(bp, t) + (held > u ? held - u : 0);
        if (bk != bp) bh += tok.balanceOf(bk) + c.inst.owed(bk, t);
        if (sp != bp && sp != bk) s += tok.balanceOf(sp) + c.inst.owed(sp, t);
        if (sk != bp && sk != bk && sk != sp) s += tok.balanceOf(sk) + c.inst.owed(sk, t);
        address f = c.t.offer.sellerFund;
        if (f != bp && f != bk && f != sp && f != sk) s += tok.balanceOf(f) + c.inst.owed(f, t);
        if (_ph(i) == 0) return (bh, s, 0);
        uint256 n = _seatCount(i);
        for (uint256 k = 0; k < n; k++) {
            address jj = c.inst.seatOf(c.id, uint64(k)).judge;
            (, uint256 cl) = c.inst.accountOf(jj, t);
            j += cl;
            address p = payoutOfJudge[jj];
            if (p != address(0)) j += tok.balanceOf(p);
        }
    }

    /// One observed call that may be refused: a refusal by name is an answer, recorded only where it has no name.
    function _soft(uint256 i, uint8 act, address who, bool judge, bytes memory data) internal returns (bool ok, bytes memory ret) {
        _before(i);
        _readView(i, act, data);
        btok.resetPushGas();
        lastClosedSeen = false;
        if (judge) {
            address k = _cur(who);
            vm.prank(k, k);
            (ok, ret) = address(cases[i].inst).call{gas: TX_GAS}(data);
        } else {
            (ok, ret) = _try(who, address(cases[i].inst), data);
        }
        _meterPush(act);
        if (!ok) {
            _unnamed(act, ret);
            return (false, ret);
        }
        _after(i, act);
        _sweptGhost(i, act, data);
    }

    /// A judge's call (accept, rule, resign): a judge whose key carries code (an EIP-7702 designator) is refused
    /// NotDirect whatever the seat says, and nothing changes (venue/1 s2.9); otherwise the observed call.
    function _judgeCall(uint256 i, uint8 act, bool must, address j, bytes memory data) internal returns (bool) {
        address k = _cur(j);
        if (k.code.length == 0 || j == STRANGER) return _ex(i, act, must, j, true, data);
        C storage c = cases[i];
        S memory s0 = _st(c.inst);
        IGaleedVenue.Seat memory l0;
        if (s0.seats != 0) l0 = _lastSeat(i);
        vm.prank(k, k);
        (bool ok, bytes memory ret) = address(c.inst).call{gas: TX_GAS}(data);
        S memory s1 = _st(c.inst);
        bool same = s1.ph == s0.ph && s1.seats == s0.seats && s1.unpaid == s0.unpaid;
        if (same && s0.seats != 0) {
            IGaleedVenue.Seat memory l1 = _lastSeat(i);
            same = l1.state == l0.state && l1.accepted == l0.accepted;
        }
        // the seat named comes first (UnknownSeat for an ordinal never opened), the direct call next (s10)
        uint256 named;
        assembly ("memory-safe") {
            named := mload(add(data, 68))
        }
        bytes4 want = (s0.ph == 0 || named >= s0.seats) ? Eravon.UnknownSeat.selector : Eravon.NotDirect.selector;
        if (ok || ret.length != 4 || bytes4(ret) != want || !same) {
            delegatedWrong++;
            _r5(ret);
        }
        _mark("delegate.judgeNotDirect");
        return false;
    }

    /// An observed call: one the text owes is a liveness check, any other a soft call.
    function _ex(uint256 i, uint8 act, bool must, address who, bool judge, bytes memory data) internal returns (bool ok) {
        if (must) return _exec(i, act, true, who, judge, data);
        (ok,) = _soft(i, act, who, judge, data);
    }

    function _lastJudge(uint256 i) internal view returns (uint64 s, address j) {
        return _lastOrStranger(i);
    }

    /// The step a code names on case i, built before any snapshot (a fee funded, a consent registered, an
    /// assertion allowed, so that both orders meet the same world).
    function _prep(uint256 i, uint8 code, uint256 h) internal returns (Step memory s) {
        C storage c = cases[i];
        s.code = code;
        (uint64 ls, address lj) = _lastJudge(i);
        if (code == P_CONFIRM) {
            s.act = A_CONFIRM;
            (s.who, s.data) = _buyerCall(c, TH_CONFIRM, h % 2 == 0, abi.encodeCall(Eravon.confirm, ()), Eravon.confirmBySig.selector);
        } else if (code == P_DISPUTE) {
            s.act = A_DISPUTE;
            _fundBuyerKey(c, c.lf);
            (s.who, s.data) = _buyerCall(c, TH_DISPUTE, h % 2 == 0, abi.encodeCall(Eravon.dispute, ()), Eravon.disputeBySig.selector);
        } else if (code == P_REFUND || (code == P_REFUNDSIG && c.sellerKind != 0)) {
            s.code = P_REFUND;
            s.act = A_REFUND;
            s.who = c.t.offer.sellerKey;
            s.data = abi.encodeCall(Eravon.refund, ());
        } else if (code == P_REFUNDSIG) {
            s.act = A_REFUND;
            s.signed_ = true;
            s.who = _submitter(c, h >> 40);
            s.data = abi.encodeCall(Eravon.refundBySig, (_signS(digestOf(address(c.inst), keccak256(abi.encode(TH_REFUND))))));
        } else if (code == P_SETTLE) {
            s.act = A_SETTLE;
            uint16 b = uint16(h % 10001);
            uint256 exp = _now() + 1 days;
            bytes32 d = digestOf(address(c.inst), keccak256(abi.encode(TH_SETTLE, uint256(b), exp)));
            bytes memory sigS = _settleSlot(c, true, false, (h >> 16) % 3, d);
            bytes memory sigB = _settleSlot(c, false, false, (h >> 20) % 3, d);
            s.who = _submitter(c, h >> 44);
            s.data = abi.encodeCall(Eravon.settle, (b, exp, sigB, sigS));
        } else if (code == P_CLAIM) {
            s.act = A_CLAIM;
            bytes32 man = keccak256(abi.encode(h, "r5 manifest"));
            if (c.sellerKind == 0 && (h >> 8) % 2 == 0) {
                s.who = _submitter(c, h >> 48);
                s.data = abi.encodeCall(
                    Eravon.claimDeliveryBySig, (man, _signS(digestOf(address(c.inst), keccak256(abi.encode(TH_CLAIM, man)))))
                );
            } else {
                s.who = c.t.offer.sellerKey;
                s.data = abi.encodeCall(Eravon.claimDelivery, (man));
            }
        } else if (code == P_ACCEPT) {
            s.act = A_ACCEPT;
            s.judge = true;
            s.who = lj;
            s.data = abi.encodeCall(Eravon.accept, (c.id, ls));
        } else if (code == P_RULE) {
            s.act = A_RULE;
            s.judge = true;
            s.who = lj;
            s.data = abi.encodeCall(Eravon.rule, (c.id, ls, E, uint64(h % c.t.offer.ladder.length), ENTRY, ""));
        } else if (code == P_RESIGN) {
            s.act = A_RESIGN;
            s.judge = true;
            s.who = lj;
            s.data = abi.encodeCall(Eravon.resign, (c.id, ls, E, ENTRY));
        } else if (code == P_ADVANCE) {
            s.act = A_ADVANCE;
            s.who = STRANGER;
            s.seat = ls;
            s.data = abi.encodeCall(Eravon.advance, (c.id, ls));
        } else if (code == P_REDRAW) {
            s.act = A_REDRAW;
            s.who = _drawer(c, h >> 52);
            (address[] memory m, bytes32[] memory w) = _frozen(c);
            s.data = abi.encodeCall(Eravon.redraw, (m, w));
        } else if (code == P_COMMIT) {
            s.act = A_COMMIT;
            s.who = STRANGER;
            s.data = abi.encodeCall(Eravon.commit, (c.id, bytes32(h)));
        } else if (code == P_SWEEP) {
            s.act = A_SWEEP;
            s.who = _submitter(c, h >> 56);
            address[4] memory ws = [c.t.buyerPayout, c.t.offer.sellerPayout, c.t.offer.sellerFund, lj];
            s.data = abi.encodeCall(Eravon.sweep, (ws[h % 4], address(tok)));
        } else if (code == P_SETPAYOUT) {
            s.act = A_MONEY;
            s.who = lj;
            address p_ = payoutOfJudge[lj] == address(0) ? STRANGER : payoutOfJudge[lj];
            s.data = abi.encodeCall(Eravon.setPayout, (lj, p_));
        } else if (code == P_WITHDRAW) {
            s.act = A_MONEY;
            s.who = c.credited == address(0) ? lj : c.credited;
            s.data = abi.encodeCall(Eravon.withdraw, (address(tok), s.who, payoutOfJudge[s.who]));
        } else {
            s.code = P_STRAY;
            s.v = 1 + h % 1e18;
        }
    }

    /// Runs one prepared step: a stray is minted to the instance (the buyer's, s8); a judge's act goes through
    /// `_judgeCall`'s rule for a delegated key by its own probe and is otherwise observed like every other call.
    function _runStep(uint256 i, Step memory s) internal returns (bool ok, bytes4 err) {
        C storage c = cases[i];
        if (s.code == P_STRAY) {
            _mint(address(c.inst), s.v);
            if (_ph(i) >= 1) c.stray += s.v;
            return (true, bytes4(0));
        }
        if (s.judge && _cur(s.who).code.length != 0 && s.who != STRANGER) {
            _judgeCall(i, s.act, false, s.who, s.data);
            return (false, Eravon.NotDirect.selector);
        }
        if (s.act == A_ADVANCE) advSeat = s.seat;
        refundSigned = s.signed_;
        bytes memory ret;
        (ok, ret) = _soft(i, s.act, s.who, s.judge, s.data);
        refundSigned = false;
        if (!ok && ret.length >= 4) err = bytes4(ret);
        // a sweep of the settlement token after a terminal credits every stray to the buyer (s8), as doSweep records
        if (ok && s.code == P_SWEEP && _ph(i) >= 4) c.stray = 0;
    }

    function _runPair(uint256 i, Step memory a, Step memory b) internal returns (Run memory r) {
        uint256 v0 = _viol();
        (r.okA,) = _runStep(i, a);
        r.phA = _ph(i);
        (r.bhA, r.sA, r.jA) = _tot(i);
        (r.okB, r.errB) = _runStep(i, b);
        r.phEnd = _ph(i);
        (r.bh, r.s, r.j) = _tot(i);
        if (r.phA >= 4) {
            // after a terminal nothing changes a party's total; a stray sent in is the buyer's (s8)
            uint256 add = b.code == P_STRAY ? b.v : 0;
            if (r.bh != r.bhA + add || r.s != r.sA || r.j != r.jA) r.moved = true;
        }
        // the order the text names: a dispute ahead of a refund leaves the refund refused, in both forms (s8)
        if (a.code == P_DISPUTE && r.okA && r.phA == 3 && (b.code == P_REFUND || b.code == P_REFUNDSIG)) {
            if (r.okB || r.errB != Eravon.WrongPhase.selector) r.moved = true;
            _mark("pair.disputeAheadOfRefund");
        }
        r.viol = _viol() - v0;
    }

    /// The text's rule for an unordered pair of codes from the pre-state: 1 the order is immaterial to every
    /// side's total (an act that moves no money, or a clock against the act its window governs, or two acts the
    /// text closes alike); 2 the order is immaterial to the judge's total once both orders end the case (two
    /// parties' exits, or the clock, while Claimed or Disputed, and a settlement against the clock in every
    /// phase); 0 whichever lands first decides.
    function _ruleOf(uint8 x, uint8 y, uint8 ph, uint8 ls) internal pure returns (uint8) {
        if (ph >= 4) return 1;
        if (x > y) (x, y) = (y, x);
        if (x == P_COMMIT || y == P_COMMIT || x == P_SWEEP || y == P_SWEEP || x == P_SETPAYOUT || y == P_SETPAYOUT) return 1;
        if (x == P_WITHDRAW || y == P_WITHDRAW || x == P_STRAY || y == P_STRAY) return 1;
        if (y == P_ADVANCE && (x == P_CONFIRM || x == P_DISPUTE || x == P_CLAIM || x == P_ACCEPT || x == P_RULE || x == P_RESIGN)) return 1;
        if (x == P_ADVANCE && y == P_REDRAW) return 1;
        if (x == P_DISPUTE && y == P_RESIGN) return 1;
        if (x == P_DISPUTE && y == P_SETTLE && ls != 3) return 1;
        if (x == P_SETTLE && y == P_CLAIM) return 1;
        if ((x == P_REFUND || x == P_REFUNDSIG) && (y == P_ACCEPT || y == P_RULE)) return 1;
        if (x == P_REFUND && y == P_REFUNDSIG) return 1;
        if ((x == P_REFUND || x == P_REFUNDSIG) && y == P_ADVANCE && ph == 1) return 1;
        if (x == P_SETTLE && y == P_ADVANCE) return 2;
        if (ph == 2 || ph == 3) {
            bool ex = x == P_CONFIRM || x == P_REFUND || x == P_REFUNDSIG || x == P_SETTLE || x == P_ADVANCE;
            bool ey = y == P_CONFIRM || y == P_REFUND || y == P_REFUNDSIG || y == P_SETTLE || y == P_ADVANCE;
            if (ex && ey) return 2;
        }
        return 0;
    }

    /// A code for case i: three times in four among the acts its phase admits, otherwise any of the sixteen.
    function _pairCode(uint256 i, uint256 h) internal view returns (uint8) {
        if (h % 4 == 0) return uint8((h >> 8) % 16);
        uint8 ph = _ph(i);
        S memory st = _st(cases[i].inst);
        uint8[7] memory m;
        if (ph == 1 && st.pending) m = [P_REDRAW, P_ADVANCE, P_REFUND, P_SETTLE, P_REFUNDSIG, P_STRAY, P_REDRAW];
        else if (ph == 1 && _lastSeat(i).accepted == 0) m = [P_ACCEPT, P_RESIGN, P_ADVANCE, P_REFUND, P_SETTLE, P_CLAIM, P_ACCEPT];
        else if (ph == 1) m = [P_CLAIM, P_RESIGN, P_ADVANCE, P_REFUND, P_SETTLE, P_REFUNDSIG, P_CLAIM];
        else if (ph == 2) m = [P_CONFIRM, P_DISPUTE, P_REFUND, P_REFUNDSIG, P_SETTLE, P_ADVANCE, P_RESIGN];
        else if (ph == 3) m = [P_RULE, P_RESIGN, P_ADVANCE, P_REFUND, P_REFUNDSIG, P_SETTLE, P_RULE];
        else return uint8((h >> 8) % 16);
        return m[(h >> 8) % 7];
    }

    /// A fresh order opened at once and driven to a phase: Funded as opened, the seat accepted, delivery claimed,
    /// or disputed, through observed calls the text owes (the pairs and the reorganisations start from it).
    function _freshOpen(uint256 seed) internal returns (bool, uint256) {
        if (threeCount >= 2 * MAX_THREE) return (false, 0);
        threeCount++;
        forceOpen = true;
        noKeeper = true;
        _newOrder(_h(seed ^ 0xf5), 1 + seed % 1e24, seed >> 8, 0, false);
        forceOpen = false;
        noKeeper = false;
        uint256 i = cases.length - 1;
        if (!_isOpen(i)) return (false, 0);
        C storage c = cases[i];
        uint256 target = _h(seed ^ 0xf6) % 4;
        if (_st(c.inst).pending) {
            (address[] memory m, bytes32[] memory w) = _frozen(c);
            vm.roll(vm.getBlockNumber() + 1);
            _exec(i, A_REDRAW, true, _drawerNow(c), false, abi.encodeCall(Eravon.redraw, (m, w)));
        }
        if (target >= 1 && _canAccept(i)) {
            (uint64 s, address j) = _lastOrStranger(i);
            _judgeCall(i, A_ACCEPT, true, j, abi.encodeCall(Eravon.accept, (c.id, s)));
        }
        if (target >= 2 && _canClaim(i)) {
            _exec(i, A_CLAIM, true, c.t.offer.sellerKey, false, abi.encodeCall(Eravon.claimDelivery, (keccak256(abi.encode(seed, "fresh")))));
        }
        if (target >= 3 && _canDispute(i) && _lastSeat(i).state == 0) {
            _fundBuyerKey(c, c.lf);
            (address who, bytes memory data) =
                _buyerCall(c, TH_DISPUTE, true, abi.encodeCall(Eravon.dispute, ()), Eravon.disputeBySig.selector);
            _exec(i, A_DISPUTE, true, who, false, data);
        }
        _mark("pair.freshCase");
        return (_isOpen(i), i);
    }

    function _pairIndex(uint8 x, uint8 y) internal pure returns (uint256) {
        if (x > y) (x, y) = (y, x);
        return uint256(x) * 16 + y;
    }

    /// Two acts sent for one block, by any two of the seller, the buyer, the judge and a stranger (collusion and
    /// races alike), run in both orders from one snapshot, each act observed against every model from its own
    /// pre-state; the totals of the three sides compared where the text makes the order immaterial; after a
    /// terminal nothing moves; a dispute ahead of a refund leaves the refund refused. Then one order is kept.
    function doPair(uint256 seed, uint256 codes, uint256 mode) external {
        uint256 hm = _h(mode ^ 0x5a17);
        bool ok;
        uint256 i;
        uint256 pk = hm % 8;
        if (pk < 3) (ok, i) = _freshOpen(seed);
        if (pk < 7 && (!ok || !_isOpen(i))) (ok, i) = _pickWhere(seed, _isLate);
        if (pk < 7 && (!ok || !_isOpen(i))) (ok, i) = _pickWhere(seed, _isOpen);
        if (!ok) (ok, i) = _pickWhere(seed, _isSet);
        if (!ok || _ph(i) == 0) return _progress(seed);
        _moment(i, (hm >> 8) % 4, (hm >> 12) % 4);
        uint8 ph0 = _ph(i);
        uint8 ca = _pairCode(i, _h(codes));
        uint8 cb = _pairCode(i, _h(codes ^ 0xb));
        // one Claimed pair in four is the dispute against the refund, in either form (s8 names that order)
        if (ph0 == 2 && (codes >> 80) % 4 == 0) {
            ca = P_DISPUTE;
            cb = (codes >> 82) % 2 == 0 ? P_REFUND : P_REFUNDSIG;
        }
        if (ca == cb) cb = uint8((uint256(cb) + 1 + (codes >> 64) % 15) % 16);
        _pairOn(i, ca, cb, codes, (hm >> 20) % 2 == 0);
    }

    /// The moment of a pair: now, the last second of the clock in force, or one past it; a draw's block the first
    /// after its arming, the second, or past its seed; otherwise a new block one time in two.
    function _moment(uint256 i, uint256 tsel, uint256 gsel) internal {
        if (!_isOpen(i)) return;
        C storage c = cases[i];
        S memory st = _st(c.inst);
        uint256 end_ = st.pending ? st.redrawDeadline : (st.seats != 0 ? _lastSeat(i).windowEnd : 0);
        if (tsel >= 2 && end_ + (tsel - 2) > _now()) vm.warp(end_ + (tsel - 2));
        if (st.pending) {
            uint256[4] memory gaps = [uint256(1), 1, 2, 257];
            uint256 target = uint256(st.drawBlock) + gaps[gsel % 4];
            if (vm.getBlockNumber() < target) vm.roll(target);
        } else if (gsel % 2 == 0) {
            vm.roll(vm.getBlockNumber() + 1);
        }
    }

    /// Codes ca and cb on case i in both orders from one snapshot, the rules checked; then one order is kept.
    function _pairOn(uint256 i, uint8 ca, uint8 cb, uint256 codes, bool keepAB) internal {
        uint8 ph0 = _ph(i);
        uint8 ls0 = _seatCount(i) == 0 ? 0 : _lastSeat(i).state;
        uint256 key = uint256(ph0) * 16 + uint256(ls0) * 2 + (_st(cases[i].inst).pending ? 1 : 0);
        Step memory a = _prep(i, ca, _h(codes ^ 1));
        Step memory b = _prep(i, cb, _h(codes ^ 2));
        uint256 s0 = vm.snapshotState();
        Run memory ab = _runPair(i, a, b);
        vm.revertToState(s0);
        Run memory ba = _runPair(i, b, a);
        vm.revertToStateAndDelete(s0);
        // what the rolled-back runs found stays found
        if (ab.viol + ba.viol != 0) {
            pairStepWrong += ab.viol + ba.viol;
            _r5(abi.encode("step", a.code, b.code, ab.viol, ba.viol));
        }
        if (ab.moved || ba.moved) {
            pairAfterTerminal++;
            _r5(abi.encode("afterTerminal", a.code, b.code, ab.phA, ba.phA, ab.errB, ba.errB));
        }
        uint8 rule = _ruleOf(a.code, b.code, ph0, ls0);
        bool closedBoth = ab.phEnd >= 4 && ba.phEnd >= 4;
        if (rule == 1) {
            if (ab.bh != ba.bh || ab.s != ba.s || ab.j != ba.j || (ab.phEnd >= 4) != (ba.phEnd >= 4)) {
                pairOrderWrong++;
                _r5(abi.encode("full", a.code, b.code, ph0, ls0, ab.bh, ba.bh, ab.s, ba.s, ab.j, ba.j));
            }
            _mark("pair.fullRule");
        } else if (rule == 2 && closedBoth) {
            if (ab.j != ba.j) {
                pairOrderWrong++;
                _r5(abi.encode("judge", a.code, b.code, ph0, ls0, ab.j, ba.j));
            }
            _mark("pair.judgeRule");
        }
        if (rule != 1 && (ab.bh != ba.bh || ab.s != ba.s || ab.j != ba.j || ab.phEnd != ba.phEnd)) {
            orderDep[key] |= uint256(1) << _pairIndex(a.code, b.code);
            if (ab.j != ba.j) orderDepJudge[key] |= uint256(1) << _pairIndex(a.code, b.code);
        }
        if (ab.okA && ab.okB) _mark("pair.bothLanded");
        if (ab.phA >= 4 || ba.phA >= 4) _mark("pair.afterTerminal");
        if (ab.okA || ab.okB || ba.okA || ba.okB) pairCovered |= (uint256(1) << _pairIndex(a.code, b.code));
        _mark("pair.run");
        _mark(string(abi.encodePacked("pair.pre.", vm.toString(ph0))));
        // the campaign goes on with one of the two orders
        if (keepAB) _runPair(i, a, b);
        else _runPair(i, b, a);
    }

    /// The pair matrix: a fresh case at a chosen point (0 a named offer unaccepted, 1 accepted in T1, 2 Claimed under
    /// a seated seat, 3 Claimed after the seat resigned, 4 Disputed, 5 a guild draw armed, 6 a guild offer unaccepted,
    /// 7 a terminal), the moment chosen, and two given codes run in both orders (EravonPairMatrix walks every pair).
    function pairMatrix(uint8 target, uint8 ca, uint8 cb, uint256 tsel, uint256 seed) external {
        forceOpen = true;
        noKeeper = true;
        forceManual = true;
        forceTrack = (target == 5 || target == 6) ? 1 : 3;
        _newOrder(_h(seed ^ 0x3a7), 1 + seed % 1e24, seed >> 8, 0, false);
        forceOpen = false;
        noKeeper = false;
        forceManual = false;
        forceTrack = 0;
        uint256 i = cases.length - 1;
        if (!_isOpen(i)) return;
        C storage c = cases[i];
        if (target == 6) {
            (address[] memory m, bytes32[] memory w) = _frozen(c);
            vm.roll(vm.getBlockNumber() + 1);
            _exec(i, A_REDRAW, true, _drawerNow(c), false, abi.encodeCall(Eravon.redraw, (m, w)));
        }
        if (target >= 1 && target <= 4 || target == 7) {
            (uint64 s, address j) = _lastOrStranger(i);
            _judgeCall(i, A_ACCEPT, true, j, abi.encodeCall(Eravon.accept, (c.id, s)));
        }
        if (target >= 2 && target <= 4 || target == 7) {
            _exec(i, A_CLAIM, true, c.t.offer.sellerKey, false, abi.encodeCall(Eravon.claimDelivery, (keccak256(abi.encode(seed, "matrix")))));
        }
        if (target == 3) {
            (uint64 s, address j) = _lastOrStranger(i);
            _judgeCall(i, A_RESIGN, true, j, abi.encodeCall(Eravon.resign, (c.id, s, E, ENTRY)));
        }
        if (target == 4) {
            _fundBuyerKey(c, c.lf);
            (address who, bytes memory data) =
                _buyerCall(c, TH_DISPUTE, true, abi.encodeCall(Eravon.dispute, ()), Eravon.disputeBySig.selector);
            _exec(i, A_DISPUTE, true, who, false, data);
        }
        if (target == 7) {
            (address who, bytes memory data) =
                _buyerCall(c, TH_CONFIRM, true, abi.encodeCall(Eravon.confirm, ()), Eravon.confirmBySig.selector);
            _exec(i, A_CONFIRM, true, who, false, data);
        }
        _moment(i, tsel, seed >> 200);
        _pairOn(i, ca, cb, _h(seed ^ 0xc0de), seed % 2 == 0);
    }

    function pairsCovered() external view returns (uint256 n) {
        uint256 x = pairCovered;
        while (x != 0) {
            n += x & 1;
            x >>= 1;
        }
    }

    /// A reorganisation: three acts given three slots (one block, or consecutive blocks a block's time or an hour
    /// apart, or the last across the clock's end), run as sent and again with the acts swapped among the slots, the
    /// arming of a draw among them where the case is a guild's; every act of both histories is observed against
    /// the models from its own pre-state (the drawn member the formula's for whichever block armed it), and one
    /// history is kept.
    function doReorg(uint256 seed, uint256 codes, uint256 mode) external {
        uint256 hm = _h(mode ^ 0x2e09);
        bool ok;
        uint256 i;
        if (hm % 3 == 0) (ok, i) = _freshArmed(seed);
        else if (hm % 3 == 1) (ok, i) = _freshOpen(seed);
        if (!ok) (ok, i) = _pickWhere(seed, _isOpen);
        if (!ok || !_isOpen(i)) return _progress(seed);
        Step[3] memory st;
        for (uint256 k = 0; k < 3; k++) {
            uint8 code = _pairCode(i, _h(codes ^ (k + 0x10)));
            if (_st(cases[i].inst).pending && k == 1 && (hm >> 4) % 2 == 0) code = P_REDRAW;
            st[k] = _prep(i, code, _h(codes ^ (k + 0x20)));
        }
        uint256[3] memory dts;
        for (uint256 k = 0; k < 3; k++) {
            uint256 sel = (hm >> (8 + 4 * k)) % 4;
            dts[k] = sel == 0 ? 0 : (sel == 1 ? 12 : (sel == 2 ? 1 hours : 3 days));
        }
        uint8[3][6] memory P = [[0, 1, 2], [1, 0, 2], [0, 2, 1], [2, 1, 0], [1, 2, 0], [2, 0, 1]];
        uint256 pi = 1 + (hm >> 24) % 5;
        uint256 b0 = vm.getBlockNumber();
        uint256 t0 = _now();
        uint256 s0 = vm.snapshotState();
        uint256 v1 = _history(i, st, P[0], dts, b0, t0);
        vm.revertToState(s0);
        uint256 v2 = _history(i, st, P[pi], dts, b0, t0);
        vm.revertToStateAndDelete(s0);
        if (v1 + v2 != 0) {
            pairStepWrong += v1 + v2;
            _r5(abi.encode("reorg", st[0].code, st[1].code, st[2].code, pi, v1, v2));
        }
        _mark("reorg.run");
        _history(i, st, P[(hm >> 32) % 2 == 0 ? 0 : pi], dts, b0, t0);
    }

    function _history(uint256 i, Step[3] memory st, uint8[3] memory order, uint256[3] memory dts, uint256 b0, uint256 t0)
        internal
        returns (uint256 v)
    {
        uint256 v0 = _viol();
        vm.roll(b0);
        vm.warp(t0);
        uint256 t = t0;
        for (uint256 k = 0; k < 3; k++) {
            t += dts[k];
            uint256 b = b0 + (dts[k] == 0 ? 0 : k + 1);
            if (b > vm.getBlockNumber()) vm.roll(b);
            if (t > _now()) vm.warp(t);
            Step memory s = st[order[k]];
            uint256 nBefore = _ph(i) == 0 ? 0 : _seatCount(i);
            _runStep(i, s);
            if (s.code == P_REDRAW && _ph(i) == 1 && _seatCount(i) > nBefore) _mark("reorg.drewAfterTheMove");
        }
        v = _viol() - v0;
    }

    // ---- around init: the opening, the cancellation, the revocation and a top-up in one block ----

    uint8 internal constant I_INIT = 0;
    uint8 internal constant I_CANCEL = 1;
    uint8 internal constant I_REVOKE = 2;
    uint8 internal constant I_STRAY = 3;

    function _initStep(uint256 i, uint8 code, uint256 v, bytes memory initData, address opener, bytes memory cancelData, address canceller)
        internal
        returns (bool ok)
    {
        C storage c = cases[i];
        if (code == I_STRAY) {
            _mint(address(c.inst), v);
            if (_ph(i) >= 1) c.stray += v;
            return true;
        }
        if (code == I_REVOKE) {
            _before(i);
            (ok,) = _try(c.t.offer.sellerKey, address(logic), abi.encodeCall(Eravon.revoke, (c.t.offer.sellerKey, c.t.offer.termsHash)));
            if (ok) {
                revokedTerms[c.t.offer.sellerKey][c.t.offer.termsHash] = true;
                _after(i, A_REVOKE);
            }
            return ok;
        }
        if (code == I_INIT) {
            if (c.track == 1 && _ph(i) == 0) c.panel = guildPanel;
            (ok,) = _soft(i, A_INIT, opener, false, initData);
            return ok;
        }
        (ok,) = _soft(i, A_CANCEL, canceller, false, cancelData);
    }

    function _initRun(uint256 i, uint8 x, uint8 y, uint256 v, bytes memory initData, address opener, bytes memory cancelData, address canceller)
        internal
        returns (Run memory r)
    {
        uint256 v0 = _viol();
        r.okA = _initStep(i, x, v, initData, opener, cancelData, canceller);
        r.phA = _ph(i);
        (r.bhA, r.sA, r.jA) = _tot(i);
        r.okB = _initStep(i, y, v, initData, opener, cancelData, canceller);
        r.phEnd = _ph(i);
        (r.bh, r.s, r.j) = _tot(i);
        if (r.phA >= 4) {
            uint256 add = y == I_STRAY ? v : 0;
            if (r.bh != r.bhA + add || r.s != r.sA || r.j != r.jA) r.moved = true;
        }
        r.viol = _viol() - v0;
    }

    /// s4 around init: two of the opening, the buyer's cancellation, the seller's revocation and a top-up of the
    /// address, sent for one block on an unopened order, in both orders from one snapshot, now or at the last second
    /// of the order's gate or one past it: every act is the text's from its own pre-state (the opening first opens,
    /// the cancellation first closes, a revocation first sends the opening down the expiry branch); a top-up is the
    /// buyer's whichever lands first, a revocation leaves a cancellation as it was, and on a dead order the expiry
    /// branch and the cancellation credit the buyer alike; after a cancellation nothing moves.
    function doInitOrder(uint256 seed, uint256 codes, uint256 mode) external {
        uint256 hm = _h(mode ^ 0x1717);
        (bool ok, uint256 i) = _pickWhere(seed, _isNone);
        if (!ok || _ph(i) != 0) {
            if (threeCount >= 2 * MAX_THREE) return _progress(seed);
            threeCount++;
            _newOrder(_h(seed ^ 0x1718), 1 + seed % 1e24, seed >> 8, 0, false);
            i = cases.length - 1;
            if (_ph(i) != 0) return;
        }
        C storage c = cases[i];
        // the moment: now, the gate's last second, or one past it
        uint256 gate = c.t.openBy < c.t.offer.expiry ? c.t.openBy : c.t.offer.expiry;
        uint256 tsel = hm % 4;
        if (tsel >= 2 && gate + (tsel - 2) > _now() && gate < _now() + 30 days) vm.warp(gate + (tsel - 2));
        uint8 x = uint8(_h(codes) % 4);
        uint8 y = uint8((x + 1 + _h(codes ^ 1) % 3) % 4);
        uint256 v = 1 + _h(codes ^ 2) % 1e18;
        (address opener, bytes memory initData) = _initCall(i);
        (address canceller, bytes memory cancelData) = _cancelCall(i, (hm >> 8) % 2 == 0);
        bool dead = _dead(c);
        uint256 s0 = vm.snapshotState();
        Run memory ab = _initRun(i, x, y, v, initData, opener, cancelData, canceller);
        vm.revertToState(s0);
        Run memory ba = _initRun(i, y, x, v, initData, opener, cancelData, canceller);
        vm.revertToStateAndDelete(s0);
        if (ab.viol + ba.viol != 0) {
            pairStepWrong += ab.viol + ba.viol;
            _r5(abi.encode("initStep", x, y, ab.viol, ba.viol));
        }
        if (ab.moved || ba.moved) {
            pairAfterTerminal++;
            _r5(abi.encode("initAfterTerminal", x, y));
        }
        uint8 lo = x < y ? x : y;
        uint8 hi = x < y ? y : x;
        bool full = hi == I_STRAY || (lo == I_CANCEL && hi == I_REVOKE) || (lo == I_INIT && hi == I_CANCEL && dead);
        if (full && (ab.bh != ba.bh || ab.s != ba.s || ab.j != ba.j)) {
            pairOrderWrong++;
            _r5(abi.encode("initFull", x, y, dead, ab.bh, ba.bh, ab.s, ba.s));
        }
        _mark(string(abi.encodePacked("initOrder.", vm.toString(lo), ".", vm.toString(hi), dead ? ".dead" : ".live")));
        if ((hm >> 12) % 2 == 0) _initRun(i, x, y, v, initData, opener, cancelData, canceller);
        else _initRun(i, y, x, v, initData, opener, cancelData, canceller);
    }

    // ---- the three-seat guild walk ----

    /// A fresh guild order on the guild of judges' track (three members, consent never carried, a full offer
    /// window), walked through up to twelve steps: draws in the block after the arming, declines, offers left to
    /// expire, a stale offer's decline, acceptances, deliveries, resignations in every phase, disputes, rulings and
    /// the three clocks, with the draw's deadline and the loss of its seed among them. Every step is observed; the
    /// guild case opens at most three seats and never draws after an accepted seat or a lost seed (s4, s6, s7).
    function doThreeSeats(uint256 seed, uint256 script) external {
        if (threeCount >= 2 * MAX_THREE) return _progress(seed);
        threeCount++;
        if (guildPanel != 0 && _h(seed ^ 0x3b) % 2 == 0) {
            (address[] memory m, bytes32[] memory w) = _panel(1, 0);
            _engage(GUILD, m, w);
            // the record serves from the next block on (s7)
            vm.roll(vm.getBlockNumber() + 1);
            guildPanel = 0;
        }
        forceTrack = _h(seed ^ 0x3d) % 2 == 0 ? 1 : 4;
        forceManual = _h(seed ^ 0x3c) % 4 != 0;
        noKeeper = true;
        _newOrder(_h(seed ^ 0x3a), 1 + seed % 1e24, seed >> 8, 0, true);
        forceTrack = 0;
        forceManual = false;
        noKeeper = false;
        uint256 i = cases.length - 1;
        if (!_isOpen(i)) return;
        _mark("three.opened");
        for (uint256 k = 0; k < 12 && _isOpen(i); k++) {
            _threeStep(i, _h(script ^ (k + 0x300)));
        }
        uint256 n = _seatCount(i);
        if (n == 3) _mark(cases[i].track == 3 ? "three.thirdSeatOfFive" : "three.thirdSeat");
        if (n > maxGuildSeats) maxGuildSeats = n;
    }

    function _threeStep(uint256 i, uint256 hk) internal {
        C storage c = cases[i];
        S memory st = _st(c.inst);
        (address[] memory m, bytes32[] memory w) = _frozen(c);
        if (st.pending) {
            uint256 wsel = hk % 10;
            if (wsel < 7) {
                if (vm.getBlockNumber() <= st.drawBlock) vm.roll(uint256(st.drawBlock) + 1);
                bool live = _now() <= st.redrawDeadline || vm.getBlockNumber() == uint256(st.drawBlock) + 1;
                if (_ex(i, A_REDRAW, live, _drawerNow(c), false, abi.encodeCall(Eravon.redraw, (m, w)))) _mark("three.drawn");
            } else if (wsel == 7) {
                vm.warp(uint256(st.redrawDeadline) + 1);
                vm.roll(vm.getBlockNumber() + 2);
                advSeat = uint64(st.seats);
                if (_exec(i, A_ADVANCE, true, STRANGER, false, abi.encodeCall(Eravon.advance, (c.id, uint64(st.seats))))) {
                    _mark("three.deadlinePassed");
                }
            } else if (wsel == 8) {
                if (uint256(st.drawBlock) + 300 > vm.getBlockNumber()) vm.roll(uint256(st.drawBlock) + 300);
                if ((hk >> 8) % 2 == 0) {
                    if (_ex(i, A_REDRAW, _now() <= st.redrawDeadline, _drawerNow(c), false, abi.encodeCall(Eravon.redraw, (m, w)))) _mark("three.seedLost");
                } else {
                    advSeat = 0;
                    if (_exec(i, A_ADVANCE, true, STRANGER, false, abi.encodeCall(Eravon.advance, (c.id, 0)))) _mark("three.seedLost");
                }
            } else {
                _exec(i, A_REFUND, true, c.t.offer.sellerKey, false, abi.encodeCall(Eravon.refund, ()));
            }
            return;
        }
        IGaleedVenue.Seat memory L = _lastSeat(i);
        uint64 ls = uint64(st.seats - 1);
        uint256 x = hk % 20;
        if (L.accepted == 0) {
            if (x < 7) {
                if (_judgeCall(i, A_RESIGN, true, L.judge, abi.encodeCall(Eravon.resign, (c.id, ls, E, ENTRY)))) _mark("three.declined");
            } else if (x < 14) {
                vm.warp(uint256(L.windowEnd) + 1);
                vm.roll(vm.getBlockNumber() + 1);
                advSeat = ls;
                if (_exec(i, A_ADVANCE, true, STRANGER, false, abi.encodeCall(Eravon.advance, (c.id, ls)))) _mark("three.expiredOffer");
            } else if (x < 19) {
                if (_judgeCall(i, A_ACCEPT, _now() <= L.windowEnd, L.judge, abi.encodeCall(Eravon.accept, (c.id, ls)))) {
                    _mark(st.seats > 1 ? "three.acceptedAfterAnEndedSeat" : "three.acceptedFirst");
                }
            } else {
                // an offer past its clock nobody advanced is still in flight: its judge may still decline it (s4)
                vm.warp(uint256(L.windowEnd) + 1);
                if (_judgeCall(i, A_RESIGN, true, L.judge, abi.encodeCall(Eravon.resign, (c.id, ls, E, ENTRY)))) _mark("three.declinedStaleOffer");
            }
            return;
        }
        if (st.ph == 1) {
            if (x < 10) {
                _ex(i, A_CLAIM, _now() <= L.windowEnd, c.t.offer.sellerKey, false, abi.encodeCall(Eravon.claimDelivery, (keccak256(abi.encode(hk)))));
            } else if (x < 13) {
                if (_now() <= L.windowEnd && _judgeCall(i, A_RESIGN, true, L.judge, abi.encodeCall(Eravon.resign, (c.id, ls, E, ENTRY)))) {
                    _mark("three.resignAfterAccept");
                }
            } else if (x < 15) {
                vm.warp(uint256(L.windowEnd) + 1);
                advSeat = ls;
                _exec(i, A_ADVANCE, true, STRANGER, false, abi.encodeCall(Eravon.advance, (c.id, ls)));
            } else if (x < 17) {
                _exec(i, A_REFUND, true, c.t.offer.sellerKey, false, abi.encodeCall(Eravon.refund, ()));
            } else {
                _threeSettle(i, hk);
            }
            return;
        }
        if (st.ph == 2) {
            if (L.state == 0 && x < 5 && _now() <= L.windowEnd) {
                if (_judgeCall(i, A_RESIGN, true, L.judge, abi.encodeCall(Eravon.resign, (c.id, ls, E, ENTRY)))) _mark("three.resignAfterAccept");
            } else if (x < 12 && _now() <= L.windowEnd) {
                _fundBuyerKey(c, c.lf);
                (address who, bytes memory data) =
                    _buyerCall(c, TH_DISPUTE, true, abi.encodeCall(Eravon.dispute, ()), Eravon.disputeBySig.selector);
                _exec(i, A_DISPUTE, true, who, false, data);
            } else if (x < 15) {
                (address who, bytes memory data) =
                    _buyerCall(c, TH_CONFIRM, true, abi.encodeCall(Eravon.confirm, ()), Eravon.confirmBySig.selector);
                _exec(i, A_CONFIRM, true, who, false, data);
            } else if (x < 18) {
                vm.warp(uint256(L.windowEnd) + 1);
                advSeat = ls;
                _exec(i, A_ADVANCE, true, STRANGER, false, abi.encodeCall(Eravon.advance, (c.id, ls)));
            } else {
                _threeSettle(i, hk);
            }
            return;
        }
        // Disputed
        if (x < 8 && _now() <= L.windowEnd) {
            _judgeCall(i, A_RULE, true, L.judge, abi.encodeCall(Eravon.rule, (c.id, ls, E, uint64(hk % c.t.offer.ladder.length), ENTRY, "")));
        } else if (x < 14 && _now() <= L.windowEnd) {
            if (_judgeCall(i, A_RESIGN, true, L.judge, abi.encodeCall(Eravon.resign, (c.id, ls, E, ENTRY)))) _mark("three.resignInDispute");
        } else {
            vm.warp(uint256(L.windowEnd) + 1);
            advSeat = ls;
            _exec(i, A_ADVANCE, true, STRANGER, false, abi.encodeCall(Eravon.advance, (c.id, ls)));
        }
    }

    function _threeSettle(uint256 i, uint256 hk) internal {
        C storage c = cases[i];
        uint16 b = uint16(hk % 10001);
        uint256 exp = _now() + 1 days;
        bytes32 d = digestOf(address(c.inst), keccak256(abi.encode(TH_SETTLE, uint256(b), exp)));
        bytes memory sigS = _settleSlot(c, true, false, (hk >> 8) % 3, d);
        bytes memory sigB = _settleSlot(c, false, false, (hk >> 12) % 3, d);
        _exec(i, A_SETTLE, true, _submitter(c, uint256(d) >> 3), false, abi.encodeCall(Eravon.settle, (b, exp, sigB, sigS)));
    }

    /// Round 5's observation of one action: a draw is armed, and a seat opened, only after an unaccepted seat
    /// ended, never after an accepted one, never past three seats (s4, s6, s7); every Closed's amounts land where it
    /// says, with the judge's credit, and nothing stays unrecorded (s8); P only in the four rows of s8.
    function _round5(uint256 i, uint8 act, Ev memory ev, uint256 nAfter) internal {
        C storage c = cases[i];
        if (c.track != 0 && nAfter != 0) {
            S memory st = _st(c.inst);
            if (nAfter > 3) rearmWrong++;
            if ((st.pending && !snap.pending) || nAfter > snap.seats) {
                uint256 upto = nAfter > snap.seats ? snap.seats : nAfter;
                for (uint256 k = 0; k < upto; k++) {
                    if (c.inst.seatOf(c.id, uint64(k)).accepted != 0) {
                        rearmWrong++;
                        _r5(abi.encode("rearm", act, k, nAfter));
                    }
                }
            }
            if (nAfter > maxGuildSeats) maxGuildSeats = nAfter;
        }
        if (ev.closed) _checkLanding(i, act, ev);
        if (ev.closed && ev.j != 0 && act != A_CANCEL) _checkPRow(i, act, ev);
    }

    /// s8, s10: the buyer's side received exactly buyerPaid (its payout's balance and credit), the seller's side
    /// exactly sellerPaid (its payout and its key), where the two sides share no address (their sum where they do),
    /// the bond within the buyer's amount, and the instance keeps nothing unrecorded.
    function _checkLanding(uint256 i, uint8 act, Ev memory ev) internal {
        C storage c = cases[i];
        address[3] memory pp = [c.t.buyerPayout, c.t.offer.sellerPayout, c.t.offer.sellerFund];
        int256[3] memory d;
        for (uint256 k = 0; k < 3; k++) {
            d[k] = int256(tok.balanceOf(pp[k]) + c.inst.owed(pp[k], address(tok))) - int256(snap.balP[k] + snap.owedP[k]);
        }
        int256 buyerSide = d[0];
        int256 sellerSide = (pp[1] != pp[0] ? d[1] : int256(0)) + (pp[2] != pp[0] && pp[2] != pp[1] ? d[2] : int256(0));
        bool shared = pp[1] == pp[0] || pp[2] == pp[0];
        bool bad = shared ? buyerSide + sellerSide != int256(ev.b + ev.s) : (buyerSide != int256(ev.b) || sellerSide != int256(ev.s));
        if (ev.bond > ev.b) bad = true;
        if (tok.balanceOf(address(c.inst)) != _st(c.inst).unpaid) bad = true;
        if (bad) {
            closedLandingWrong++;
            _r5(abi.encode("landing", act, buyerSide, sellerSide, ev.b, ev.s));
        }
        _mark("landing.checked");
    }

    /// Round 6 (s8): what each of the buyer's payout, the seller's payout and sellerFund received at the terminal
    /// (its balance and its credit), each address summing the shares the table sends it where two of them are one;
    /// the seller's key, where it is none of the three, receives nothing.
    function _checkSplit(C storage c, uint256 toPayout, uint256 toFund, uint256 toBuyer) internal {
        if (_payoutBad(c, toPayout, toFund, toBuyer)) {
            closedLandingWrong++;
            _r5(abi.encode("split", toBuyer, toPayout, toFund));
        }
        if (_fundBad(c, toFund)) {
            fundReturnWrong++;
            _r5(abi.encode("fundReturn", toFund, c.t.offer.sellerFund, c.t.offer.sellerKey));
        }
        _mark("landing.split");
        address f = c.t.offer.sellerFund;
        // round 7: the model's return, kept for the dual-ledger invariant and added to a pool fund's ghost
        c.toFundG = toFund;
        if (isPoolFund[f]) {
            balG[f] += int256(toFund);
            if (toFund != 0) _mark("pool.returnRefilled");
        }
        if (toFund != 0 && f != c.t.offer.sellerKey) {
            _mark(f == c.t.offer.sellerPayout ? "fund.returnedAtSellerPayout" : (f == c.t.buyerPayout ? "fund.returnedAtBuyerPayout" : "fund.returnedAtAFunder"));
        }
    }

    /// The change of an address's balance and credit at this instance since the snapshot.
    function _dl(C storage c, address a, uint256 bal0, uint256 owed0) internal view returns (int256) {
        return int256(tok.balanceOf(a) + c.inst.owed(a, address(tok))) - int256(bal0 + owed0);
    }

    /// The two payouts received what the table sends them (with the deposit's return where the fund is one of them),
    /// and the terminal pushed it where the token lets it.
    function _payoutBad(C storage c, uint256 toPayout, uint256 toFund, uint256 toBuyer) internal view returns (bool bad) {
        address bp = c.t.buyerPayout;
        address sp = c.t.offer.sellerPayout;
        address f = c.t.offer.sellerFund;
        uint256 eb = toBuyer + (sp == bp ? toPayout : 0) + (f == bp ? toFund : 0);
        if (_dl(c, bp, snap.balP[0], snap.owedP[0]) != int256(eb)) bad = true;
        if (sp != bp && _dl(c, sp, snap.balP[1], snap.owedP[1]) != int256(toPayout + (f == sp ? toFund : 0))) bad = true;
        // the terminal pushes what it credits (s8): a creditor the token neither blocks nor burns is owed nothing after
        if (!btok.blocked(bp) && !btok.burns(bp) && c.inst.owed(bp, address(tok)) != 0) bad = true;
        if (!btok.blocked(sp) && !btok.burns(sp) && c.inst.owed(sp, address(tok)) != 0) bad = true;
    }

    /// sellerFund received the deposit's return where it is neither payout, the seller's key nothing where it is none
    /// of the three, and the fund's push landed where the token lets it.
    function _fundBad(C storage c, uint256 toFund) internal view returns (bool bad) {
        address bp = c.t.buyerPayout;
        address sp = c.t.offer.sellerPayout;
        address f = c.t.offer.sellerFund;
        address sk = c.t.offer.sellerKey;
        if (f != bp && f != sp && _dl(c, f, snap.balP[2], snap.owedP[2]) != int256(toFund)) bad = true;
        if (sk != bp && sk != sp && sk != f && _dl(c, sk, snap.balK, snap.owedK) != 0) bad = true;
        if (!btok.blocked(f) && !btok.burns(f) && c.inst.owed(f, address(tok)) != 0) bad = true;
    }

    function _checkPRow(uint256 i, uint8 act, Ev memory ev) internal {
        C storage c = cases[i];
        bool inW = _now() <= snap.lastWindowEnd;
        uint8 row;
        if (ev.term == 0 && snap.ph == 2 && snap.lastState == 0) row = 1;
        else if (ev.term == 2 && snap.lastAccepted != 0 && ((snap.lastState <= 1 && inW) || (snap.lastState == 0 && snap.ph == 2))) row = 2;
        else if (ev.term == 1 && snap.lastState == 0) row = 3;
        else if (ev.term == 3) row = 4;
        uint256 want = row == 4 ? c.p + snap.lfHeld : c.p;
        if (row == 0 || ev.j != want) {
            pCreditWrong++;
            _r5(abi.encode("pRow", act, ev.term, snap.ph, snap.lastState, ev.j, want));
        }
        creditRowOf[i] = row;
        _mark(row == 1 ? "pRow.refundWhileClaimed" : (row == 2 ? "pRow.settlement" : (row == 3 ? "pRow.release" : "pRow.ruling")));
    }

    // ---- hostile foreign tokens ----

    /// s8: a foreign token sent to an instance, of every hostile shape (a lying balance, a transfer that answers
    /// true and moves nothing, half, a revert, a re-entry into the instance's locked doors, a long answer, all the
    /// gas burned); sweep by a party's current key: the settlement ledger (balance, unpaid, every credit, the phase
    /// and the seats) does not move, the foreign token leaves to buyerPayout by what left alone, once that address is
    /// fixed (NotParty before, every sender refused), and Swept reports what left.
    function doForeignHostile(uint256 seed, uint256 mode) external {
        bool ok;
        uint256 i;
        if (_h(mode ^ 0xf0e2) % 4 == 0) (ok, i) = _pickWhere(seed, _isNone);
        if (!ok) (ok, i) = _pickAny(seed);
        if (!ok) return;
        C storage c = cases[i];
        uint256 hm = _h(mode ^ 0xf0e1);
        if (hostiles.length < 3) hostiles.push(new R5Hostile());
        R5Hostile hf = hostiles[hm % hostiles.length];
        uint8 m = uint8((hm >> 8) % 8);
        hf.setMode(m, address(c.inst));
        uint256 v = 1 + (hm >> 16) % 1e24;
        hf.mint(address(c.inst), v);
        address bp = c.t.buyerPayout;
        bool fixed_ = _ph(i) != 0 || c.branchRan;
        uint256 instF = hf.realBalance(address(c.inst));
        uint256 bpF = hf.realBalance(bp);
        uint256 bal = tok.balanceOf(address(c.inst));
        S memory s0 = _st(c.inst);
        (uint256 bh0, uint256 sv0, uint256 j0) = _tot(i);
        uint256 rr = hf.reentriesRefused();
        address[3] memory ws = [bp, c.t.offer.sellerKey, STRANGER];
        // s9: the sweep is a party's (its current key); before the instance keeps the keys every sender is NotParty
        vm.recordLogs();
        (bool s_, bytes memory ret) =
            _try(_submitter(c, hm >> 48), address(c.inst), abi.encodeCall(Eravon.sweep, (ws[(hm >> 40) % 3], address(hf))));
        VmSafe.Log[] memory logs = vm.getRecordedLogs();
        if (!s_) _unnamed(A_SWEEP, ret);
        uint256 moved = instF - hf.realBalance(address(c.inst));
        uint256 want = !fixed_ ? 0 : (m == 0 || m == 5 || m == 6 ? instF : (m == 3 ? instF / 2 : 0));
        S memory s1 = _st(c.inst);
        (uint256 bh1, uint256 sv1, uint256 j1) = _tot(i);
        bool answered = fixed_ ? s_ : (!s_ && ret.length == 4 && bytes4(ret) == Eravon.NotParty.selector);
        bool bad = !answered || moved != want || hf.realBalance(bp) != bpF + moved || tok.balanceOf(address(c.inst)) != bal
            || s1.unpaid != s0.unpaid || s1.ph != s0.ph || s1.seats != s0.seats || bh1 != bh0 || sv1 != sv0 || j1 != j0;
        bool swept = _logsHave(logs, address(c.inst), Eravon.Swept.selector, abi.encode(address(hf), moved));
        if ((moved != 0) != swept) bad = true;
        if (m == 5 && fixed_ && hf.reentriesRefused() != rr + 1) bad = true;
        if (bad) {
            foreignLedgerWrong++;
            _r5(abi.encode("foreign", m, fixed_, moved, want, ret));
        }
        _mark(fixed_ ? string(abi.encodePacked("foreign.hostile", vm.toString(m), ".fixed")) : "foreign.hostile.unfixed");
    }

    // ---- odd decimals and flipping feeds ----

    struct PM {
        uint256[4] dec; // the decimals() words read: feedToken's, feedUnit's, the token's, the unit's
        uint256[4] len; // their answer lengths
        int256 pT;
        int256 pU;
        bool zeroT; // feedToken is the zero address: price 1, no decimals, no clock
        bool zeroU;
        uint256 fixed_;
        uint256 bps;
        uint256 min_;
        uint256 max_;
        uint256 cap;
        uint256 amount;
        uint256 premiumBps;
    }

    /// x * y as a 512-bit pair from 128-bit limbs.
    function _m512(uint256 x, uint256 y) internal pure returns (uint256 hi, uint256 lo) {
        uint256 M = type(uint128).max;
        uint256 x0 = x & M;
        uint256 x1 = x >> 128;
        uint256 y0 = y & M;
        uint256 y1 = y >> 128;
        uint256 p00 = x0 * y0;
        uint256 p01 = x0 * y1;
        uint256 p10 = x1 * y0;
        uint256 p11 = x1 * y1;
        uint256 mid = (p00 >> 128) + (p01 & M) + (p10 & M);
        lo = (p00 & M) | ((mid & M) << 128);
        hi = p11 + (p01 >> 128) + (p10 >> 128) + (mid >> 128);
    }

    /// ceil(x * y / d) by long division of the 512-bit product one bit at a time; (false, 0) past 2^256 - 1.
    function _ceilModel(uint256 x, uint256 y, uint256 d) internal pure returns (bool, uint256) {
        (uint256 hi, uint256 lo) = _m512(x, y);
        uint256 r;
        uint256 qhi;
        uint256 qlo;
        unchecked {
            for (uint256 k = 512; k > 0; k--) {
                uint256 bit = k - 1 >= 256 ? (hi >> (k - 1 - 256)) & 1 : (lo >> (k - 1)) & 1;
                bool carry = r >> 255 == 1;
                r = (r << 1) | bit;
                if (carry || r >= d) {
                    r -= d;
                    if (k - 1 >= 256) qhi |= uint256(1) << (k - 1 - 256);
                    else qlo |= uint256(1) << (k - 1);
                }
            }
        }
        if (qhi != 0) return (false, 0);
        if (r != 0) {
            if (qlo == type(uint256).max) return (false, 0);
            qlo++;
        }
        return (true, qlo);
    }

    /// x * 10^k, (false, 0) past 2^256 - 1 (x nonzero).
    function _p10Model(uint256 x, uint256 k) internal pure returns (bool, uint256) {
        if (k > 77) return (false, 0);
        uint256 p = 1;
        for (uint256 n = 0; n < k; n++) {
            p *= 10;
        }
        if (x > type(uint256).max / p) return (false, 0);
        return (true, x * p);
    }

    /// s3's verdict from the words the reads answered: 0 the case opens at (lf, cap), 1 RateStale, 2 FeeOverflow at
    /// the pricing, 3 OverCap, 4 FeeOverflow at the deposit's sum.
    function _pmVerdict(PM memory m) internal pure returns (uint8, uint256, uint256) {
        for (uint256 k = 0; k < 4; k++) {
            if ((k == 0 && m.zeroT) || (k == 1 && m.zeroU)) continue;
            if (m.len[k] < 32 || m.dec[k] > 36) return (1, 0, 0);
        }
        if (!m.zeroT && (m.pT <= 0 || uint256(m.pT) >= (1 << 128))) return (1, 0, 0);
        if (!m.zeroU && (m.pU <= 0 || uint256(m.pU) >= (1 << 128))) return (1, 0, 0);
        uint256 pT = m.zeroT ? 1 : uint256(m.pT);
        uint256 pU = m.zeroU ? 1 : uint256(m.pU);
        uint256 dT = m.zeroT ? 0 : m.dec[0];
        uint256 dU = m.zeroU ? 0 : m.dec[1];
        uint256[4] memory v = [m.fixed_, m.min_, m.max_, m.cap];
        if (v[0] != 0 || v[1] != 0 || v[2] != 0 || v[3] != 0) {
            uint256 a_ = dT + m.dec[2];
            uint256 b_ = dU + m.dec[3];
            uint256 num = pU;
            uint256 den = pT;
            bool ok = true;
            if (a_ >= b_) (ok, num) = _p10Model(pU, a_ - b_);
            else (ok, den) = _p10Model(pT, b_ - a_);
            if (!ok) return (2, 0, 0);
            for (uint256 k = 0; k < 4; k++) {
                if (v[k] == 0) continue;
                (ok, v[k]) = _ceilModel(v[k], num, den);
                if (!ok) return (2, 0, 0);
            }
        }
        uint256 share = (m.amount * m.bps) / 10000;
        if (v[0] > type(uint256).max - share) return (2, 0, 0);
        uint256 lf = v[0] + share;
        if (v[1] != 0 && lf < v[1]) lf = v[1];
        if (v[2] != 0 && lf > v[2]) lf = v[2];
        if (v[3] != 0 && m.amount > v[3]) return (3, 0, 0);
        uint256 p = (m.amount * m.premiumBps) / 10000;
        if (lf > type(uint256).max - p) return (4, 0, 0);
        // a deposit no token can hold beside the amount: the seller cannot hold it, and the pull fails
        if (p + lf > type(uint256).max - m.amount) return (5, 0, 0);
        return (0, lf, v[3]);
    }

    function _verdictOf(bool ok, bytes memory ret) internal pure returns (uint8) {
        if (ok) return 0;
        if (ret.length != 4) return 255;
        bytes4 e = bytes4(ret);
        if (e == Eravon.RateStale.selector) return 1;
        if (e == Eravon.FeeOverflow.selector) return 2;
        if (e == Eravon.OverCap.selector) return 3;
        if (e == Eravon.TransferFailed.selector) return 5;
        return 254;
    }

    function _pick(uint256 h, uint256[12] memory xs) internal pure returns (uint256) {
        return xs[h % 12];
    }

    /// Opens a pricing order in its own token and its own feeds; returns the verdict and what feeQuoted pinned.
    function _oddOpen(PM memory m, R5Token tkn, R5Feed fT, R5Feed fU, R5Feed unit, uint256 hf)
        internal
        returns (bool ok, bytes memory ret, Eravon inst, uint256 d)
    {
        Q memory q = defaultQuote(address(tkn), address(0), true);
        q.premiumBps = uint16(m.premiumBps);
        q.feeUnit = address(unit);
        q.feedToken = m.zeroT ? address(0) : address(fT);
        q.feedUnit = m.zeroU ? address(0) : address(fU);
        q.maxStale = type(uint32).max;
        q.feeFixed = m.fixed_;
        q.feeBps = uint16(m.bps);
        q.feeMin = m.min_;
        q.feeMax = m.max_;
        q.amountCap = m.cap;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(namedJudges[hf % 3], AMANAH, quote, m.amount);
        t.offer.token = address(tkn);
        t.offer.termsHash = keccak256(abi.encode("odd terms", oddCount++, hf));
        inst = deployProxy(t);
        tkn.mint(address(inst), m.amount);
        (uint8 v, uint256 lf,) = _pmVerdict(m);
        if (v == 0) d = (m.amount * m.premiumBps) / 10000 + lf;
        tkn.mint(SELLER, type(uint256).max - m.amount);
        vm.prank(SELLER);
        tkn.approve(address(logic), type(uint256).max - m.amount);
        bytes memory data = abi.encodeCall(Eravon.init, (t, quote, _buyerSig(t), _sellerSig(t), new address[](0), new bytes32[](0)));
        vm.prank(_cur(SELLER));
        (ok, ret) = address(inst).call{gas: TX_GAS}(data);
    }

    /// s3 under odd decimals(): the settlement token's, the unit's and both feeds' decimals() at 0 to 36 and past it
    /// (37, 64, 255, 256, 2^255, 2^256 - 1), answers 0, 1, 31, 32, 33 or 64 bytes long, prices from 1 to 2^128 - 1
    /// and past it, a zero feed among them, amounts to 2^256 - 1: the verdict (open, RateStale, FeeOverflow,
    /// OverCap) and the pinned values are those of a model that converts with a 512-bit long division.
    function doOddDecimals(uint256 flags) external {
        uint256 hf = _h(flags ^ 0x0dd);
        if (oddCount >= 48 || (hf >> 250) % 2 == 0) return _progress(flags);
        PM memory m;
        uint256[12] memory decs = [uint256(0), 1, 2, 6, 8, 9, 12, 18, 24, 30, 35, 36];
        uint256[6] memory bad = [uint256(37), 64, 255, 256, 1 << 255, type(uint256).max];
        uint256[6] memory lens = [uint256(0), 1, 31, 33, 64, 32];
        for (uint256 k = 0; k < 4; k++) {
            uint256 hk = _h(hf ^ (k + 1));
            m.dec[k] = hk % 24 == 0 ? bad[(hk >> 8) % 6] : decs[(hk >> 16) % 12];
            m.len[k] = hk % 24 == 1 ? lens[(hk >> 24) % 6] : 32;
        }
        uint256[12] memory prices = [uint256(1), 7, 1e8, 2000e8, 1e18, 3 ** 50, (1 << 127) - 1, (1 << 128) - 1, 12345678901, 1e30, 2, 1e6];
        uint256 hp = _h(hf ^ 0x9);
        m.pT = int256(_pick(hp, prices));
        m.pU = int256(_pick(hp >> 8, prices));
        if (hp % 32 == 0) m.pT = (hp >> 16) % 2 == 0 ? int256(0) : -1;
        if ((hp >> 20) % 32 == 0) m.pU = int256(1 << 128);
        m.zeroT = (hp >> 30) % 8 == 0;
        m.zeroU = (hp >> 34) % 8 == 0;
        uint256[12] memory amts = [uint256(0), 0, 1, 1e6, 5e6, 1e18, 1e24, 1e30, 1 << 200, type(uint256).max, 999, 0];
        uint256 ha = _h(hf ^ 0xa);
        m.fixed_ = _pick(ha, amts);
        m.min_ = _pick(ha >> 8, amts);
        m.max_ = _pick(ha >> 16, amts);
        m.cap = (ha >> 24) % 3 == 0 ? _pick(ha >> 28, amts) : 0;
        uint256[3] memory bpsv = [uint256(0), 1000, 10000];
        m.bps = bpsv[(ha >> 36) % 3];
        uint256[5] memory av = [uint256(1), 1000, 1e18, 1e27, type(uint256).max / 10000];
        m.amount = av[(ha >> 40) % 5];
        m.premiumBps = bpsv[(ha >> 44) % 3] / 10;
        R5Token tkn = new R5Token();
        R5Feed fT = new R5Feed();
        R5Feed fU = new R5Feed();
        R5Feed unit = new R5Feed();
        uint256 at = _now() - 1;
        fT.set(m.pT, m.pT, at, m.dec[0], m.dec[0], m.len[0], 0);
        fU.set(m.pU, m.pU, at, m.dec[1], m.dec[1], m.len[1], 0);
        tkn.setDec(m.dec[2], m.dec[2], m.len[2], false);
        unit.set(1, 1, 0, m.dec[3], m.dec[3], m.len[3], 0);
        (uint8 want, uint256 lf, uint256 cap) = _pmVerdict(m);
        (bool ok, bytes memory ret, Eravon inst, uint256 d) = _oddOpen(m, tkn, fT, fU, unit, hf);
        if (!ok && !_isNamed(ret)) {
            unnamedRevert++;
            lastUnnamed = ret;
        }
        uint8 got = _verdictOf(ok, ret);
        if (want == 4) want = 2;
        bool bad_ = got != want;
        if (ok && !bad_) {
            (uint256 lfQ, uint256 capQ, uint256 pTQ, uint256 pUQ, uint64 atQ) = inst.feeQuoted();
            uint256 wantAt = (m.zeroT && m.zeroU) ? _now() : at;
            if (lfQ != lf || capQ != cap || pTQ != (m.zeroT ? 1 : uint256(m.pT)) || pUQ != (m.zeroU ? 1 : uint256(m.pU)) || atQ != wantAt || depositHeld(inst) != d) bad_ = true;
        }
        if (bad_) {
            oddDecWrong++;
            _r5(abi.encode("odd", want, got, lf, cap, ret));
        }
        string[5] memory names = ["odd.opened", "odd.rateStale", "odd.feeOverflow", "odd.overCap", "odd.other"];
        _mark(names[got > 3 ? 4 : got]);
    }

    /// s3 with reads that flip within one init: one feed for both prices whose answer flips by the gas it is handed,
    /// a feed whose decimals() flips, a token whose decimals() flips, and the unit read from the flipping feed itself:
    /// the case opens with pinned prices among the answers, and its fee and cap are the model's for some combination
    /// of the words the reads could have answered, or it is refused by a name some combination gives.
    function doFlipFeed(uint256 flags) external {
        uint256 hf = _h(flags ^ 0xf11b);
        if (oddCount >= 48 || (hf >> 250) % 2 == 0) return _progress(flags);
        uint8 mode = uint8(hf % 4);
        PM memory m;
        m.len = [uint256(32), 32, 32, 32];
        uint256 a1 = [uint256(2000e8), 1e8, 7, 3 ** 30][(hf >> 8) % 4];
        uint256 a2 = [uint256(1999e8), 3e8, 1, 2 ** 100][(hf >> 12) % 4];
        uint256 d1 = [uint256(8), 18, 0, 36][(hf >> 16) % 4];
        uint256 d2 = [uint256(6), 2, 30, 18][(hf >> 20) % 4];
        m.dec = [uint256(8), 8, 18, 6];
        m.fixed_ = (hf >> 24) % 2 == 0 ? 0 : 1e6;
        m.min_ = 5e6;
        m.max_ = (hf >> 26) % 2 == 0 ? 0 : 500e6;
        m.cap = (hf >> 28) % 3 == 0 ? 1e30 : 0;
        m.bps = 1000;
        m.amount = [uint256(1e18), 1e27, 1000][(hf >> 30) % 3];
        m.premiumBps = 100;
        R5Token tkn = new R5Token();
        R5Feed fT = new R5Feed();
        R5Feed fU = new R5Feed();
        R5Feed unit = new R5Feed();
        uint256 at = _now() - 1;
        uint256[2] memory dTs = [uint256(8), 8];
        uint256[2] memory dUs = [uint256(8), 8];
        uint256[2] memory decTs = [uint256(18), 18];
        uint256[2] memory decUs = [uint256(6), 6];
        if (mode == 0) {
            // one feed for both prices, its answer flipping
            fT.set(int256(a1), int256(a2), at, 8, 8, 32, 1);
            fU = fT;
            tkn.setDec(18, 18, 32, false);
            unit.set(1, 1, 0, 6, 6, 32, 0);
        } else if (mode == 1) {
            // feedToken's decimals() flipping
            fT.set(int256(a1), int256(a1), at, d1, d2, 32, 2);
            fU.set(1e8, 1e8, at, 8, 8, 32, 0);
            tkn.setDec(18, 18, 32, false);
            unit.set(1, 1, 0, 6, 6, 32, 0);
            dTs = [d1, d2];
        } else if (mode == 2) {
            // the token's decimals() flipping
            fT.set(int256(a1), int256(a1), at, 8, 8, 32, 0);
            fU.set(1e8, 1e8, at, 8, 8, 32, 0);
            tkn.setDec(d1, d2, 32, true);
            unit.set(1, 1, 0, 6, 6, 32, 0);
            decTs = [d1, d2];
        } else {
            // the unit is the flipping feed itself: its decimals() read twice, as a feed's and as the unit's
            fT.set(int256(a1), int256(a2), at, d1, d2, 32, 3);
            fU.set(1e8, 1e8, at, 8, 8, 32, 0);
            tkn.setDec(18, 18, 32, false);
            unit = fT;
            dTs = [d1, d2];
            decUs = [d1, d2];
        }
        (bool ok, bytes memory ret, Eravon inst,) = _oddOpenFlip(m, tkn, fT, fU, unit, hf, a1);
        uint8 got = _verdictOf(ok, ret);
        bool found;
        uint256 lfQ;
        uint256 capQ;
        uint256 pTQ;
        uint256 pUQ;
        if (ok) (lfQ, capQ, pTQ, pUQ,) = inst.feeQuoted();
        int256[2] memory ans = [int256(a1), int256(a2)];
        for (uint256 x = 0; x < 16 && !found; x++) {
            m.dec = [dTs[x & 1], dUs[(x >> 1) & 1], decTs[(x >> 2) & 1], decUs[(x >> 3) & 1]];
            for (uint256 y = 0; y < 4 && !found; y++) {
                m.pT = mode == 0 || mode == 3 ? ans[y & 1] : int256(a1);
                m.pU = mode == 0 ? ans[(y >> 1) & 1] : int256(1e8);
                if (ok && (uint256(m.pT) != pTQ || uint256(m.pU) != pUQ)) continue;
                (uint8 v, uint256 lf, uint256 cap) = _pmVerdict(m);
                if (v == 4) v = 2;
                if (v != got) continue;
                if (ok && (lf != lfQ || cap != capQ)) continue;
                found = true;
            }
        }
        if (!found) {
            flipWrong++;
            _r5(abi.encode("flip", mode, got, lfQ, capQ, pTQ, pUQ, ret));
        }
        _mark(string(abi.encodePacked("flip.mode", vm.toString(mode), ok ? ".opened" : ".refused")));
    }

    /// The flipping order's open: the deposit minted for the largest fee any combination gives, approved in full.
    function _oddOpenFlip(PM memory m, R5Token tkn, R5Feed fT, R5Feed fU, R5Feed unit, uint256 hf, uint256)
        internal
        returns (bool ok, bytes memory ret, Eravon inst, uint256 d)
    {
        Q memory q = defaultQuote(address(tkn), address(0), true);
        q.premiumBps = uint16(m.premiumBps);
        q.feeUnit = address(unit);
        q.feedToken = address(fT);
        q.feedUnit = address(fU);
        q.maxStale = type(uint32).max;
        q.feeFixed = m.fixed_;
        q.feeBps = uint16(m.bps);
        q.feeMin = m.min_;
        q.feeMax = m.max_;
        q.amountCap = m.cap;
        bytes memory quote = encodeQuote(q);
        Eravon.OrderTerms memory t = namedTerms(namedJudges[hf % 3], AMANAH, quote, m.amount);
        t.offer.token = address(tkn);
        t.offer.termsHash = keccak256(abi.encode("flip terms", oddCount++, hf));
        inst = deployProxy(t);
        tkn.mint(address(inst), m.amount);
        d = type(uint256).max - m.amount;
        tkn.mint(SELLER, d);
        vm.prank(SELLER);
        tkn.approve(address(logic), d);
        bytes memory data = abi.encodeCall(Eravon.init, (t, quote, _buyerSig(t), _sellerSig(t), new address[](0), new bytes32[](0)));
        vm.prank(_cur(SELLER));
        (ok, ret) = address(inst).call{gas: TX_GAS}(data);
    }

    // ---- EIP-7702 delegated accounts ----

    /// An account of the campaign takes code for a while, as an EIP-7702 delegation gives it: a party's key (the
    /// seller's or the buyer's) a designator, or code that executes calls so that the party acts through it; a named
    /// judge or a guild member a designator (its accept, rule and resign are refused NotDirect, venue/1 s2.9, and the
    /// case runs on its clocks); a payout or the guild a designator. One call in four clears every delegation.
    function doDelegate(uint256 seed) external {
        uint256 hs = _h(seed ^ 0x7702);
        address[10] memory roots = [
            SELLER, BUYER, namedJudges[0], namedJudges[1], members[0], members[1], g2[0], SELLER_PAYOUT, BUYER_PAYOUT, GUILD
        ];
        if (hs % 4 == 0) {
            for (uint256 k = 0; k < 10; k++) {
                address[2] memory both = [roots[k], _cur(roots[k])];
                for (uint256 b = 0; b < 2; b++) {
                    if (both[b].code.length != 0) vm.etch(both[b], "");
                    delegatedExec[both[b]] = false;
                }
            }
            return;
        }
        // the delegation lands on the key that acts now for the record (s9): the party's, the judge's or the guild's
        address w = _cur(roots[(hs >> 8) % 10]);
        if (w.code.length != 0) {
            vm.etch(w, "");
            delegatedExec[w] = false;
            return;
        }
        bool party = w == _cur(SELLER) || w == _cur(BUYER);
        if (party && (hs >> 16) % 2 == 0) {
            vm.etch(w, coSeller.code);
            delegatedExec[w] = true;
            _mark("delegate.partyRunsCode");
        } else {
            vm.etch(w, abi.encodePacked(hex"ef0100", coSeller));
            _mark(party ? "delegate.partyDesignator" : "delegate.otherDesignator");
        }
    }

    // ---- griefing ----

    /// Calls a stranger can make for no gain, between the parties' acts: commits, an advance naming an ordinal never
    /// opened while nothing is due, sweeps for anyone, consents and revocations under its own address and under the
    /// parties', a redraw with another panel, every signed form under junk, a settlement, a second init and a cancel,
    /// the judge's three calls, setPayout and withdraw under its own address and under the seat's judge, and a stray
    /// of its own: no side's total moves (the stray is the buyer's), the phase and every seat stay as they were, and
    /// every refusal is the text's: NotParty for the sweep, the signed forms and the settlement (s9, s10),
    /// NotPendingRedraw or PanelMismatch for the redraw with another panel (s7: open to any sender),
    /// NotCurrent for a registration under another's name, AlreadyInitialized for init and cancel, UnknownSeat or
    /// NotDirect for the judge's calls, NoPanelFace for the instance's engage.
    function doGrief(uint256 seed, uint256 mode) external {
        (bool ok, uint256 i) = _pickWhere(seed, _isOpen);
        if (!ok || !_isOpen(i)) return _progress(seed);
        C storage c = cases[i];
        uint256 hm = _h(mode ^ 0x6e1e);
        (uint256 bh0, uint256 s0_, uint256 j0) = _tot(i);
        S memory st0 = _st(c.inst);
        IGaleedVenue.Seat memory l0;
        if (st0.seats != 0) l0 = _lastSeat(i);
        uint256 strays;
        uint256 k = 3 + hm % 4;
        bytes4 want;
        for (uint256 n = 0; n < k; n++) {
            uint256 g = _h(hm ^ n) % 12;
            bytes memory data;
            address to = address(c.inst);
            if (g == 0) {
                data = abi.encodeCall(Eravon.commit, (c.id, bytes32(_h(n))));
            } else if (g == 1) {
                if (_due(i)) continue;
                data = abi.encodeCall(Eravon.advance, (c.id, uint64(st0.seats + 1 + n)));
            } else if (g == 2) {
                address[3] memory ws = [STRANGER, address(logic), payoutOfJudge[members[0]]];
                data = abi.encodeCall(Eravon.sweep, (ws[n % 3], n % 2 == 0 ? address(tok) : address(ftok)));
            } else if (g == 3) {
                to = address(logic);
                uint256 r3 = _h(n ^ 13) % 5;
                if (r3 == 0) data = abi.encodeCall(Eravon.consent, (STRANGER, bytes32(_h(n ^ 7))));
                else if (r3 == 1) data = abi.encodeCall(Eravon.revoke, (STRANGER, c.t.offer.termsHash));
                else if (r3 == 2) data = abi.encodeCall(Eravon.acceptFunding, (c.t.offer.sellerKey));
                else if (r3 == 3) data = abi.encodeCall(Eravon.consent, (n % 2 == 0 ? c.t.offer.sellerKey : c.t.buyerKey, bytes32(_h(n ^ 7))));
                else data = abi.encodeCall(Eravon.revoke, (c.t.offer.sellerKey, c.t.offer.termsHash));
                if (r3 >= 3) want = Eravon.NotCurrent.selector;
            } else if (g == 4) {
                (address[] memory m, bytes32[] memory w) = _panel(1, 1 - (c.track == 1 ? c.panel : 0));
                data = abi.encodeCall(Eravon.redraw, (m, w));
                // s7, s10: open to any sender; no draw armed or a late one is NotPendingRedraw, otherwise the panel
                // test refuses the panel the case did not freeze, before DrawPending
                want = (!st0.pending || (_now() > st0.redrawDeadline && vm.getBlockNumber() > uint256(st0.drawBlock) + 1))
                    ? Eravon.NotPendingRedraw.selector
                    : Eravon.PanelMismatch.selector;
                _mark(want == Eravon.NotPendingRedraw.selector ? "grief.redrawNotPending" : "grief.redrawPanelMismatch");
            } else if (g == 5) {
                bytes memory junk = new bytes(n % 2 == 0 ? 65 : 300);
                junk[3] = 0x11;
                uint256 f = _h(n ^ 9) % 5;
                if (f == 0) data = abi.encodeCall(Eravon.confirmBySig, (junk));
                else if (f == 1) data = abi.encodeCall(Eravon.disputeBySig, (junk));
                else if (f == 2) data = abi.encodeCall(Eravon.refundBySig, (junk));
                else if (f == 3) data = abi.encodeCall(Eravon.claimDeliveryBySig, (bytes32(uint256(1)), junk));
                else data = abi.encodeCall(Eravon.settle, (5000, _now() + 1, junk, junk));
                want = Eravon.NotParty.selector;
            } else if (g == 6) {
                (, data) = _initCall(i);
                want = Eravon.AlreadyInitialized.selector;
            } else if (g == 7) {
                data = abi.encodeCall(Eravon.cancel, (c.t, bytes("")));
                want = Eravon.AlreadyInitialized.selector;
            } else if (g == 8) {
                uint64 ls = uint64(st0.seats == 0 ? 0 : st0.seats - 1);
                uint256 f = _h(n ^ 11) % 3;
                if (f == 0) data = abi.encodeCall(Eravon.accept, (c.id, ls));
                else if (f == 1) data = abi.encodeCall(Eravon.rule, (c.id, ls, E, 0, ENTRY, ""));
                else data = abi.encodeCall(Eravon.resign, (c.id, ls, E, ENTRY));
                want = st0.seats == 0 ? Eravon.UnknownSeat.selector : Eravon.NotDirect.selector;
            } else if (g == 9) {
                address lj = st0.seats == 0 ? address(0) : l0.judge;
                uint256 f = _h(n ^ 17) % 4;
                if (f == 0) {
                    data = abi.encodeCall(Eravon.setPayout, (STRANGER, STRANGER));
                } else if (f == 1) {
                    data = abi.encodeCall(Eravon.withdraw, (address(tok), STRANGER, address(0)));
                } else if (lj == address(0)) {
                    continue;
                } else {
                    data = f == 2 ? abi.encodeCall(Eravon.setPayout, (lj, STRANGER)) : abi.encodeCall(Eravon.withdraw, (address(tok), lj, STRANGER));
                    want = Eravon.NotCurrent.selector;
                }
            } else if (g == 10) {
                uint256 v = 1 + _h(n) % 1e6;
                _mint(address(c.inst), v);
                c.stray += v;
                strays += v;
                continue;
            } else {
                data = abi.encodeCall(Eravon.engage, (c.id, new address[](0), new bytes32[](0), bytes32(0), 0, address(0)));
                want = Eravon.NoPanelFace.selector;
            }
            vm.prank(STRANGER, STRANGER);
            (bool s_, bytes memory ret) = to.call{gas: TX_GAS}(data);
            if (!s_ && !_isNamed(ret)) {
                unnamedRevert++;
                lastUnnamed = ret;
                lastUnnamedAct = 100;
            }
            if (g == 2) want = Eravon.NotParty.selector;
            if (want != bytes4(0) && (s_ || ret.length != 4 || bytes4(ret) != want)) {
                strangerAdmitted++;
                _r5(abi.encode("griefRefusal", g, want, s_, ret));
            }
            want = bytes4(0);
        }
        (uint256 bh1, uint256 s1_, uint256 j1) = _tot(i);
        S memory st1 = _st(c.inst);
        bool same = st1.ph == st0.ph && st1.seats == st0.seats && st1.pending == st0.pending && st1.lfHeld == st0.lfHeld;
        if (same && st0.seats != 0) {
            IGaleedVenue.Seat memory l1 = _lastSeat(i);
            same = l1.state == l0.state && l1.accepted == l0.accepted && l1.windowEnd == l0.windowEnd;
        }
        if (!same || bh1 != bh0 + strays || s1_ != s0_ || j1 != j0) {
            griefMoved++;
            _r5(abi.encode("grief", bh0, bh1, strays, s0_, s1_, j0, j1));
        }
        _mark("grief.nothingMoved");
    }

    // ------------------------------------------------------------------
    // the succession book: records moved to fresh keys (s9, venue/1 s1, galeed.roster/1 s4)
    // ------------------------------------------------------------------

    /// A record moves on the succession book (the current key declares a fresh key, the fresh key accedes): the
    /// seller's, the buyer's, a judge's or a guild's. From then on every gate takes the new key and the former one
    /// acts, signs and registers for nothing; every case in flight runs on under the new key, nothing migrates, and
    /// what the former key registered while current stands (s9). The former key is probed at once.
    function doMoveKey(uint256 seed, uint256 mode) external {
        uint256 hm = _h(mode ^ 0x5ecc);
        uint8 role = uint8(hm % 4);
        if (moves[role] >= MAX_MOVES) return _progress(seed);
        address root;
        if (role == 0) {
            root = SELLER;
        } else if (role == 1) {
            root = BUYER;
        } else if (role == 2) {
            address[10] memory js = [
                namedJudges[0], namedJudges[1], namedJudges[2], members[0], members[1], members[2], g2[0], g2[1], g3[0], g3[1]
            ];
            root = js[(hm >> 8) % 10];
        } else {
            address[3] memory gs = [GUILD, GUILD2, GUILD3];
            root = gs[(hm >> 8) % 3];
        }
        address from = _cur(root);
        uint256 n = ++moveCount;
        uint256 pk;
        address to;
        if (role <= 1) {
            pk = uint256(keccak256(abi.encode("eravon/invariants/key", role, n))) % SECP_N;
            if (pk == 0) pk = 1;
            to = vm.addr(pk);
        } else {
            to = address(uint160(uint256(keccak256(abi.encode("eravon/invariants/address", role, n)))));
        }
        moveKey(from, to);
        if (_cur(root) != to || _cur(to) != to || sbook.root(to) != root) {
            oldKeyAdmitted++;
            _r5(abi.encode("moveKey", root, from, to));
        }
        oldKeys.push(from);
        rootOfOld[from] = root;
        if (role == 0) {
            pkOfOld[from] = sellerPk;
            sellerPk = pk;
        } else if (role == 1) {
            pkOfOld[from] = buyerPk;
            buyerPk = pk;
        }
        moves[role]++;
        _mark(role == 0 ? "move.seller" : (role == 1 ? "move.buyer" : (role == 2 ? "move.judge" : "move.guild")));
        _probeFormerKey(oldKeys.length - 1, hm >> 16);
    }

    /// A former key probed again later, against whatever cases stand then (cases opened after the move among them).
    function doFormerKey(uint256 seed) external {
        if (oldKeys.length == 0) return _progress(seed);
        uint256 hs = _h(seed ^ 0xf0e);
        _probeFormerKey(hs % oldKeys.length, hs >> 8);
    }

    function _formerIs(bool ok, bytes memory ret, bytes4 want, string memory what) internal {
        if (ok || ret.length != 4 || bytes4(ret) != want) {
            oldKeyAdmitted++;
            _r5(abi.encode(what, ok, ret));
        }
        _mark(what);
    }

    /// The former key `oldKeys[k]` at every gate its record held, each probe a refused call that changes nothing: a
    /// party's direct acts (NotSeller, NotBuyer), its signed forms' submission and settle and sweep (NotParty), a
    /// signature it makes sent by the new key (BadSellerSig, BadBuyerSig), its registrations under the record's name
    /// (NotCurrent); a judge's direct calls (NotDirect), its setPayout and withdraw (NotCurrent); a guild's engage on
    /// the book (the book's NotCurrent); a draw is anyone's (s7), so a former guild key's is no probe. The parties'
    /// records as the cases recorded them.
    function _probeFormerKey(uint256 k, uint256 h) internal {
        address old = oldKeys[k];
        address root = rootOfOld[old];
        bool ok;
        bytes memory ret;
        uint256 n = cases.length;
        if (n == 0) return;
        uint256 start = h % n;
        if (root == SELLER || root == BUYER) {
            bool isSeller = root == SELLER;
            bytes32 dg = bytes32(h);
            (ok, ret) = _tryAs(old, address(logic), abi.encodeCall(Eravon.consent, (root, dg)));
            _formerIs(ok, ret, Eravon.NotCurrent.selector, "move.formerKeyRegistersNothing");
            if (isSeller) {
                (ok, ret) = _tryAs(old, address(logic), abi.encodeCall(Eravon.revoke, (root, keccak256(abi.encode(h)))));
                _formerIs(ok, ret, Eravon.NotCurrent.selector, "move.formerKeyRevokesNothing");
            }
            for (uint256 x = 0; x < n; x++) {
                C storage c = cases[(start + x) % n];
                if ((isSeller ? c.t.offer.sellerKey : c.t.buyerKey) != root) continue;
                uint8 ph = uint8(phaseOf(c.inst));
                if (ph >= 1 && ph <= 3) {
                    bytes memory direct = isSeller ? abi.encodeCall(Eravon.refund, ()) : abi.encodeCall(Eravon.confirm, ());
                    (ok, ret) = _tryAs(old, address(c.inst), direct);
                    _formerIs(ok, ret, isSeller ? Eravon.NotSeller.selector : Eravon.NotBuyer.selector, "move.formerKeyRefused");
                    (ok, ret) = _tryAs(old, address(c.inst), abi.encodeCall(Eravon.settle, (5000, _now() + 1 days, bytes(""), bytes(""))));
                    _formerIs(ok, ret, Eravon.NotParty.selector, "move.formerKeySettlesNothing");
                    (ok, ret) = _tryAs(old, address(c.inst), abi.encodeCall(Eravon.sweep, (c.t.buyerPayout, address(tok))));
                    _formerIs(ok, ret, Eravon.NotParty.selector, "move.formerKeySweepsNothing");
                    // a signature the former key makes is refused from the move on, whoever sends it (s9)
                    bytes memory sg;
                    bytes memory signed_;
                    if (isSeller) {
                        bytes32 man = keccak256(abi.encode(h, "former"));
                        sg = sign(pkOfOld[old], digestOf(address(c.inst), keccak256(abi.encode(TH_CLAIM, man))));
                        signed_ = abi.encodeCall(Eravon.claimDeliveryBySig, (man, sg));
                    } else {
                        sg = sign(pkOfOld[old], digestOf(address(c.inst), keccak256(abi.encode(TH_CONFIRM))));
                        signed_ = abi.encodeCall(Eravon.confirmBySig, (sg));
                    }
                    (ok, ret) = _try(root, address(c.inst), signed_);
                    _formerIs(ok, ret, isSeller ? Eravon.BadSellerSig.selector : Eravon.BadBuyerSig.selector, "move.formerSignatureRefused");
                    break;
                }
                if (ph == 0 && !isSeller && !_dead(c)) {
                    // the buyer's cancel from the former key is NotBuyer (s4)
                    (ok, ret) = _tryAs(old, address(c.inst), abi.encodeCall(Eravon.cancel, (c.t, bytes(""))));
                    _formerIs(ok, ret, Eravon.NotBuyer.selector, "move.formerKeyCancelsNothing");
                    break;
                }
            }
            return;
        }
        if (root == GUILD || root == GUILD2 || root == GUILD3) {
            (address[] memory m, bytes32[] memory w) = _panel(root == GUILD ? 1 : (root == GUILD2 ? 2 : 3), 0);
            bytes memory e = abi.encodeCall(EravonPanelBook.engage, (logic.PROXY_CODEHASH(), m, w, ROSTER, uint64(_now() + 1 days), address(0)));
            (ok, ret) = _tryAs(old, address(pbook), e);
            _formerIs(ok, ret, EravonPanelBook.NotCurrent.selector, "move.formerGuildKeyRefused");
            return;
        }
        // a judge's former key
        (ok, ret) = _tryAs(old, address(cases[start].inst), abi.encodeCall(Eravon.setPayout, (root, payoutOfJudge[root])));
        _formerIs(ok, ret, Eravon.NotCurrent.selector, "move.formerJudgeKeyRegistersNothing");
        (ok, ret) = _tryAs(old, address(cases[start].inst), abi.encodeCall(Eravon.withdraw, (address(tok), root, payoutOfJudge[root])));
        _formerIs(ok, ret, Eravon.NotCurrent.selector, "move.formerJudgeKeyWithdrawsNothing");
        for (uint256 x = 0; x < n; x++) {
            uint256 i = (start + x) % n;
            C storage c = cases[i];
            uint8 ph = _ph(i);
            if (ph < 1 || ph > 3 || _seatCount(i) == 0) continue;
            IGaleedVenue.Seat memory L = _lastSeat(i);
            if (L.judge != root || L.state > 1) continue;
            uint64 ls = uint64(_seatCount(i) - 1);
            bytes memory dj = L.accepted == 0 ? abi.encodeCall(Eravon.accept, (c.id, ls)) : abi.encodeCall(Eravon.resign, (c.id, ls, E, ENTRY));
            (ok, ret) = _tryAs(old, address(c.inst), dj);
            _formerIs(ok, ret, Eravon.NotDirect.selector, "move.formerJudgeKeyRefused");
            (ok, ret) = _tryAs(old, address(c.inst), abi.encodeCall(Eravon.sweep, (root, address(tok))));
            _formerIs(ok, ret, Eravon.NotParty.selector, "move.formerJudgeKeySweepsNothing");
            break;
        }
    }

    // ------------------------------------------------------------------
    // FUZZ r24-1: the guild draw under a router (s7, venue/1 s4), every way a router answers or fails
    // ------------------------------------------------------------------

    /// The router guild: its own records on the chain's book, each naming a fresh router, its cases in their own
    /// token apart from the campaign's ledger; its key moves mid-case.
    address internal constant RGUILD = address(0x6000000000000000000000000000000000000005);
    /// A funder bound to the seller for the router cases alone.
    address internal constant RFUND = address(uint160(0x007F00000000000000000000000000000000000F60));
    bytes32 internal constant RTERMS = keccak256("router terms document");
    uint256 internal constant MAX_RCASES = 12;
    uint256 internal constant R_ROUTE_GAS = 1_000_000;

    struct RC {
        Eravon inst;
        Eravon.OrderTerms t;
        bytes32 id;
        InvRouter router;
        uint8 pv; // the panel variant the record carries
        uint256 a;
        uint256 d;
        uint256 p;
        uint256 lf;
        bool autoAccept;
        uint32 offerWindow;
        bool failed; // a draw whose router failed ended the case
        uint256 seatsAtFail;
        bool closed;
        uint8 terminal; // the phase the model closed it in
        uint256 judgeOwed; // what the model left on the seat ledger (P on a release)
    }

    RC[] internal rcases;
    MockERC20 public rtok;
    uint256 public rMinted;
    uint256 internal routerEntry;
    bool internal rInit;
    /// a router draw, its gas, its view, or a router case's step off s7 and s8
    uint256 public routerWrong;
    /// statistic: draws whose router named the panel's zero member (the code ends the case)
    uint256 public routerZeroMember;
    /// statistic: draws made, draws that seated, draws whose router failed, gas probes run
    uint256 public routerDraws;
    uint256 public routerSeated;
    uint256 public routerFailed;
    uint256 public routerGasProbes;
    bytes public lastRouter;
    uint256 internal rguildMoves;
    address[] internal rguildOld;

    function _rr(bytes memory what) internal {
        routerWrong++;
        lastRouter = what;
    }

    function rcaseCount() external view returns (uint256) {
        return rcases.length;
    }

    function rcaseAt(uint256 k)
        external
        view
        returns (address inst, uint256 a, uint256 d, bool failed, uint256 seatsAtFail, bool closed, uint8 terminal, uint256 judgeOwed)
    {
        RC storage c = rcases[k];
        return (address(c.inst), c.a, c.d, c.failed, c.seatsAtFail, c.closed, c.terminal, c.judgeOwed);
    }

    function rtokAddr() external view returns (address) {
        return address(rtok);
    }

    /// Every address the router cases move their token to, beside the instances.
    function rHolders() external view returns (address[] memory a) {
        a = new address[](4 + judgePayouts.length);
        a[0] = BUYER_PAYOUT;
        a[1] = SELLER_PAYOUT;
        a[2] = SELLER;
        a[3] = RFUND;
        for (uint256 k = 0; k < judgePayouts.length; k++) {
            a[4 + k] = judgePayouts[k];
        }
    }

    function rguildOldKeys() external view returns (address[] memory) {
        return rguildOld;
    }

    function _rMint(address to, uint256 v) internal {
        rtok.mint(to, v);
        rMinted += v;
    }

    /// The router token, the funder's bond, and the router's entry gas when handed exactly routeGas(), read twice
    /// on views of different sizes (the entry is the same, so that every later answer tests the gas exactly).
    function _rSetup() internal {
        rInit = true;
        rtok = new MockERC20();
        InvRouter cal = new InvRouter();
        cal.set(99, 0);
        RouteView memory v;
        v.members = new address[](0);
        v.seated = new address[](0);
        v.parties = new address[](0);
        uint256 e1 = _rEntry(cal, v);
        v.members = new address[](5);
        v.seated = new address[](3);
        v.parties = new address[](5);
        uint256 e2 = _rEntry(cal, v);
        if (e1 == 0 || e1 != e2) _rr(abi.encode("calibration", e1, e2));
        routerEntry = e1;
        vm.prank(RFUND);
        logic.acceptFunding(SELLER);
    }

    function _rEntry(InvRouter r, RouteView memory v) internal view returns (uint256) {
        (bool ok, bytes memory ret) = address(r).staticcall{gas: R_ROUTE_GAS}(abi.encodeCall(InvRouter.route, (v)));
        if (!ok || ret.length != 32) return 0;
        return abi.decode(ret, (uint256));
    }

    /// The router guild's panels: 0 five judges; 1 a judge, the buyer's key, a judge, the first judge again and the
    /// zero address; 2 three judges.
    function _rPanel(uint8 pv) internal view returns (address[] memory m, bytes32[] memory w) {
        if (pv == 0) {
            m = new address[](5);
            m[0] = members[0];
            m[1] = members[1];
            m[2] = members[2];
            m[3] = g3[0];
            m[4] = g3[1];
        } else if (pv == 1) {
            m = new address[](5);
            m[0] = members[0];
            m[1] = BUYER;
            m[2] = members[1];
            m[3] = members[0];
            m[4] = address(0);
        } else {
            m = new address[](3);
            m[0] = g2[0];
            m[1] = g2[1];
            m[2] = members[2];
        }
        w = new bytes32[](m.length);
        for (uint256 k = 0; k < m.length; k++) {
            w[k] = AMANAH;
        }
    }

    /// The guild's current key engages a record naming `router`; returns the record's index under the guild's root.
    function _rEngage(uint8 pv, address router) internal returns (uint256 index) {
        (address[] memory m, bytes32[] memory w) = _rPanel(pv);
        address gk = _cur(RGUILD);
        bytes32 T = logic.PROXY_CODEHASH();
        index = pbook.count(RGUILD, T);
        uint64 exp = uint64(_now() + 3650 days);
        vm.prank(gk, gk);
        pbook.engage(T, m, w, ROSTER, exp, router);
    }

    function _rIn(address[] memory s, address a) internal pure returns (bool) {
        for (uint256 i = 0; i < s.length; i++) {
            if (s[i] == a) return true;
        }
        return false;
    }

    function _rSeated(uint256 k) internal view returns (address[] memory s) {
        RC storage c = rcases[k];
        uint256 n = c.inst.status().seats;
        s = new address[](n);
        for (uint256 i = 0; i < n; i++) {
            s[i] = c.inst.seatOf(c.id, uint64(i)).judge;
        }
    }

    function _rSend(address who, RC storage c, bytes memory data, uint256 g) internal returns (bool ok, bytes memory ret) {
        vm.prank(who);
        (ok, ret) = address(c.inst).call{gas: g}(data);
    }

    function _rJudgeSend(address judge, RC storage c, bytes memory data) internal returns (bool ok, bytes memory ret) {
        address jk = _cur(judge);
        vm.prank(jk, jk);
        (ok, ret) = address(c.inst).call{gas: TX_GAS}(data);
    }

    /// What the instance, its last seat and the three addresses its terminal pays hold: a refused call moves none of it.
    function _rState(uint256 k) internal view returns (bytes32 h) {
        RC storage c = rcases[k];
        Eravon.CaseStatus memory st = c.inst.status();
        h = keccak256(abi.encode(st));
        if (st.seats != 0) h = keccak256(abi.encode(h, c.inst.seatOf(c.id, uint64(st.seats - 1))));
        h = keccak256(
            abi.encode(
                h,
                rtok.balanceOf(address(c.inst)),
                rtok.balanceOf(c.t.buyerPayout),
                rtok.balanceOf(c.t.offer.sellerFund),
                rtok.balanceOf(c.t.offer.sellerPayout)
            )
        );
    }

    /// The view s7 hands the router at this draw, built from the order, the frozen panel and the chain.
    function _rView(uint256 k) internal view returns (RouteView memory v) {
        RC storage c = rcases[k];
        Eravon.CaseStatus memory st = c.inst.status();
        v.caseId = c.id;
        v.seat = uint64(st.seats);
        v.termsHash = c.t.offer.termsHash;
        v.token = c.t.offer.token;
        v.amount = c.a;
        v.rungs = uint64(c.t.offer.ladder.length);
        v.parties = new address[](5);
        v.parties[0] = c.t.buyerKey;
        v.parties[1] = c.t.offer.sellerKey;
        v.parties[2] = c.t.offer.sellerFund;
        v.parties[3] = c.t.buyerPayout;
        v.parties[4] = c.t.offer.sellerPayout;
        (v.members,) = _rPanel(c.pv);
        v.seated = _rSeated(k);
        v.seed = blockhash(st.drawBlock);
        v.blockTime = uint64(_now());
        v.chainId = block.chainid;
        v.venue = address(c.inst);
    }

    /// One call: a new router case where few are live, then up to four steps of a live one.
    function doRouter(uint256 seed, uint256 mode) external {
        if (!rInit) _rSetup();
        uint256 hm = _h(mode ^ 0x2a7e);
        (bool found, uint256 k) = _rLive(_h(seed ^ 0x2a7f));
        if (!found || (rcases.length < MAX_RCASES && hm % 3 == 0)) {
            if (rcases.length >= MAX_RCASES) return _progress(seed);
            uint256 n0 = rcases.length;
            _rOpen(hm);
            if (rcases.length == n0) return;
            k = rcases.length - 1;
        }
        uint256 steps = 1 + (hm >> 200) % 4;
        uint256 w0 = routerWrong;
        for (uint256 x = 0; x < steps && !rcases[k].closed && routerWrong == w0; x++) {
            _rStep(k, _h(hm ^ (x + 1)));
        }
    }

    function _rLive(uint256 hs) internal view returns (bool, uint256) {
        uint256 n = rcases.length;
        if (n == 0) return (false, 0);
        uint256 s = hs % n;
        for (uint256 x = 0; x < n; x++) {
            uint256 k = (s + x) % n;
            if (!rcases[k].closed) return (true, k);
        }
        return (false, 0);
    }

    /// s7: the guild's current key engages a record naming a fresh router; the order opens in a later block and
    /// freezes that record, its router and the router's code hash (one opening in six with a second record made in
    /// init's own block, which never reaches the case); the first draw is armed.
    function _rOpen(uint256 hm) internal {
        if (logic.revoked(SELLER, RTERMS)) return;
        InvRouter r = new InvRouter();
        r.setExpect(routerEntry);
        uint8 pv = uint8((hm >> 8) % 3);
        uint256 idx = _rEngage(pv, address(r));
        vm.roll(vm.getBlockNumber() + 1);
        bool sameBlock = (hm >> 12) % 6 == 0;
        if (sameBlock) {
            InvRouter r2 = new InvRouter();
            r2.setExpect(routerEntry);
            _rEngage(pv, address(r2));
        }
        RC storage c = rcases.push();
        c.router = r;
        c.pv = pv;
        if (!_rOrderInit(c, hm, idx)) {
            rcases.pop();
            return;
        }
        _mark("router.opened");
        if (sameBlock) _mark("router.sameBlockRecordNotFrozen");
        if (c.autoAccept) _mark("router.opened.autoAccept");
        if (pv == 1) _mark("router.opened.partyDuplicateZero");
        if (c.t.offer.sellerFund == RFUND) _mark("router.opened.boundFund");
    }

    /// The order: the router token, a quote with lapseDefault and offerWindow drawn, the deposit at the seller's key
    /// or at the bound funder.
    function _rOrderInit(RC storage c, uint256 hm, uint256 idx) internal returns (bool) {
        Q memory q = defaultQuote(address(rtok), address(0), (hm >> 16) % 4 == 0);
        q.lapseDefault = uint8((hm >> 20) % 3);
        q.offerWindow = [uint32(1 hours), uint32(1 days), uint32(3 days)][(hm >> 24) % 3];
        bytes memory quote = encodeQuote(q);
        c.a = 1 ether + ((hm >> 32) % 500) * 1 ether;
        c.d = depositOf(q, c.a);
        c.p = premiumOf(q, c.a);
        c.lf = lfOf(q, c.a);
        c.autoAccept = q.autoAccept;
        c.offerWindow = q.offerWindow;
        Eravon.OrderTerms memory t = guildTerms(RGUILD, AMANAH, quote, c.a);
        t.offer.token = address(rtok);
        t.offer.termsHash = RTERMS;
        t.offer.sellerFund = (hm >> 40) % 2 == 0 ? SELLER : RFUND;
        t.offer.expiry = _now() + 3650 days;
        t.openBy = uint64(_now() + 3650 days);
        c.t = t;
        c.id = orderHash(t);
        c.inst = deployProxy(t);
        return _rInit(c, quote, idx);
    }

    /// init from the seller's current key with both parties' signatures by their current keys: it lands and freezes
    /// the guild's root, the record made before this block (its index, its router), and arms the first draw.
    function _rInit(RC storage c, bytes memory quote, uint256 idx) internal returns (bool) {
        _rMint(address(c.inst), c.a);
        address f = c.t.offer.sellerFund;
        _rMint(f, c.d);
        uint256 have = rtok.allowance(f, address(logic));
        vm.prank(f);
        rtok.approve(address(logic), have + c.d);
        (address[] memory m, bytes32[] memory w) = _rPanel(c.pv);
        Eravon.OrderTerms memory t = c.t;
        bytes memory data = abi.encodeCall(Eravon.init, (t, quote, _buyerSig(t), _sellerSig(t), m, w));
        vm.prank(_cur(SELLER));
        (bool ok, bytes memory ret) = address(c.inst).call{gas: TX_GAS}(data);
        if (!ok) {
            _rr(abi.encode("init", ret));
            return false;
        }
        Eravon.CaseStatus memory st = c.inst.status();
        if (st.router != address(c.router) || st.panelIndex != idx || !st.pendingRedraw || st.drawBlock != vm.getBlockNumber()
            || st.root != RGUILD || st.seats != 0 || st.deposit != c.d || rtok.balanceOf(address(c.inst)) != c.a + c.d) {
            _rr(abi.encode("frozen", st.router, address(c.router), st.panelIndex, idx));
            return false;
        }
        return true;
    }

    function _rStep(uint256 k, uint256 h) internal {
        RC storage c = rcases[k];
        Eravon.CaseStatus memory st = c.inst.status();
        if (uint8(st.phase) != 1) return _rr(abi.encode("liveCaseNotFunded", k, uint8(st.phase)));
        if (st.pendingRedraw) return _rDrawStep(k, h);
        if (st.seats == 0) return _rr(abi.encode("noSeatNoDraw", k));
        uint64 ord = uint64(st.seats - 1);
        IGaleedVenue.Seat memory L = c.inst.seatOf(c.id, ord);
        if (L.accepted == 0) return _rOffered(k, ord, L, h);
        _rAccepted(k, ord, L, h);
    }

    /// The router's mode for this draw: four in ten aimed at a seat (the first free member, an index inside the
    /// panel, the index the view hashes to; seven in ten on the first two draws, so that third seats are reached),
    /// the rest one of the failures (an index naming a seated member or its repeat eight in a hundred); `code` 1 the
    /// router's code replaced, 2 removed.
    function _rPickMode(uint256 k, uint256 h) internal view returns (uint8 mode, uint256 at, uint8 code) {
        RC storage c = rcases[k];
        (address[] memory m,) = _rPanel(c.pv);
        uint256 x = h % 100;
        if (c.inst.status().seats < 2 && (h >> 120) % 2 == 0) x = x % 40;
        if (x < 15) return (0, 0, 0);
        if (x < 28) return (3, (h >> 8) % m.length, 0);
        if (x < 40) return (4, 0, 0);
        if (x < 44) return (1, 0, 0);
        if (x < 48) return (2, 0, 0);
        if (x < 52) return (3, m.length, 0);
        if (x < 56) return (3, type(uint256).max - (h >> 8) % 1000, 0);
        if (x < 64) {
            address[] memory s = _rSeated(k);
            if (s.length == 0) return (3, m.length + (h >> 8) % 3, 0);
            // the panel that lists its first judge twice: the repeat of a seated first judge, one time in two
            if (c.pv == 1 && _rIn(s, m[0]) && (h >> 16) % 2 == 0) return (3, 3, 0);
            address j = s[(h >> 8) % s.length];
            // the last place the seated member holds in the panel: its repeat where the panel lists it twice
            for (uint256 i = 0; i < m.length; i++) {
                if (m[m.length - 1 - i] == j) return (3, m.length - 1 - i, 0);
            }
            return (3, m.length, 0);
        }
        uint8[8] memory fails = [5, 6, 7, 8, 9, 10, 11, 12];
        if (x < 96) return (fails[(x - 64) / 4], 0, 0);
        if (x < 98) return (0, 0, 1);
        return (0, 0, 2);
    }

    /// What s7 says the draw does under that router: the seat for members[index] where the router answers one word
    /// whose index lies inside the panel and names a member no seat holds (compared by address), and otherwise no
    /// usable panel, the case Resigned with form 3. The code reads a zero member as no usable panel (reported).
    function _rModel(uint256 k, uint8 mode, uint256 at, uint8 code) internal view returns (bool, address, string memory) {
        RC storage c = rcases[k];
        if (code == 1) return (true, address(0), "router.fail.codeReplaced");
        if (code == 2) return (true, address(0), "router.fail.codeRemoved");
        (address[] memory m,) = _rPanel(c.pv);
        address[] memory s = _rSeated(k);
        uint256 idx;
        if (mode == 0) {
            idx = type(uint256).max;
            for (uint256 i = 0; i < m.length; i++) {
                if (!_rIn(s, m[i])) {
                    idx = i;
                    break;
                }
            }
            if (idx == type(uint256).max) return (true, address(0), "router.fail.noneFree");
        } else if (mode == 3) {
            idx = at;
        } else if (mode == 4) {
            idx = uint256(keccak256(abi.encode(_rView(k)))) % m.length;
        } else {
            string[13] memory names = [
                "",
                "router.fail.revert",
                "router.fail.emptyRevert",
                "",
                "",
                "router.fail.twoWords",
                "router.fail.short",
                "router.fail.noData",
                "router.fail.burnsItsGas",
                "router.fail.write",
                "router.fail.invalid",
                "router.fail.long",
                "router.fail.bigRevert"
            ];
            return (true, address(0), names[mode]);
        }
        if (idx >= m.length) return (true, address(0), idx == m.length ? "router.fail.justOutside" : "router.fail.outside");
        address j = m[idx];
        if (_rIn(s, j)) {
            for (uint256 i = 0; i < idx; i++) {
                if (m[i] == j) return (true, address(0), "router.fail.duplicateOfASeated");
            }
            return (true, address(0), "router.fail.seated");
        }
        if (j == address(0)) return (true, address(0), "router.fail.zeroMember");
        if (mode == 0) return (false, j, "router.seat.first");
        if (mode == 4) return (false, j, "router.seat.hash");
        if (j == BUYER) return (false, j, "router.seat.party");
        for (uint256 i = 0; i < idx; i++) {
            if (m[i] == j) return (false, j, "router.seat.duplicate");
        }
        return (false, j, "router.seat.fixed");
    }

    function _rDrawStep(uint256 k, uint256 h) internal {
        RC storage c = rcases[k];
        Eravon.CaseStatus memory st = c.inst.status();
        (address[] memory m, bytes32[] memory w) = _rPanel(c.pv);
        bytes memory data = abi.encodeCall(Eravon.redraw, (m, w));
        if (vm.getBlockNumber() <= st.drawBlock) {
            // s7: DrawPending in the arming block itself, nothing moved
            (bool ok0, bytes memory r0) = _rSend(_cur(SELLER), c, data, TX_GAS);
            if (ok0 || r0.length != 4 || bytes4(r0) != Eravon.DrawPending.selector) return _rr(abi.encode("drawPending", ok0, r0));
            _mark("router.drawPendingRefused");
            vm.roll(vm.getBlockNumber() + 1);
        }
        if (_now() > st.redrawDeadline && vm.getBlockNumber() > st.drawBlock + 1) {
            // past the deadline beyond the first block: the draw is refused and advance closes the case (s7)
            (bool ok1, bytes memory r1) = _rSend(_cur(BUYER), c, data, TX_GAS);
            if (ok1 || r1.length != 4 || bytes4(r1) != Eravon.NotPendingRedraw.selector) return _rr(abi.encode("pastDeadline", ok1, r1));
            return _rAdvanceCloses(k, st, "router.deadlineClosed");
        }
        if ((h >> 8) % 8 == 0) _rStrangers(k, data);
        if ((h >> 16) % 16 == 0 && rguildMoves < MAX_MOVES) _rMoveGuild();
        bool seedLost = blockhash(st.drawBlock) == bytes32(0);
        (uint8 mode, uint256 at, uint8 code) = _rPickMode(k, h >> 72);
        (bool setOk,) = address(c.router).call(abi.encodeCall(InvRouter.set, (mode, at)));
        if (!setOk) return _rr(abi.encode("routerSet", k));
        if (code == 1) vm.etch(address(c.router), type(InvRouterB).runtimeCode);
        if (code == 2) vm.etch(address(c.router), "");
        bool fail = true;
        address want;
        string memory what = "router.seedLostAtTheDraw";
        if (!seedLost) (fail, want, what) = _rModel(k, mode, at, code);
        if (!seedLost && ((h >> 24) % 6 == 0 || code != 0)) _rGasProbe(k, data, fail, want, h, code == 0);
        if (!seedLost && (h >> 32) % 8 == 0) _rSeedProbe(k, data, h);
        address[4] memory who = [_cur(SELLER), _cur(BUYER), _cur(RGUILD), STRANGER];
        uint256 wi = (h >> 40) % 4;
        _rDraw(k, who[wi], data, fail, want, what);
        if (wi == 2) _mark(rguildMoves == 0 ? "router.byGuild" : "router.byMovedGuildKey");
        else if (wi == 3) _mark("router.byStranger");
        else _mark("router.byParty");
        if (keccak256(bytes(what)) == keccak256("router.fail.zeroMember")) routerZeroMember++;
    }

    /// The post-state of a draw from the pre-state `st` against the model; empty where it agrees.
    function _rCheck(uint256 k, Eravon.CaseStatus memory st, bool fail, address want, uint256 b0, uint256 f0)
        internal
        view
        returns (bytes memory)
    {
        RC storage c = rcases[k];
        Eravon.CaseStatus memory s2 = c.inst.status();
        if (s2.pendingRedraw) return abi.encode("stillArmed", fail);
        if (fail) {
            // no usable panel: Resigned with form 3, no seat opened, the amount to the buyer, the deposit to the fund
            if (uint8(s2.phase) != 8 || s2.seats != st.seats) return abi.encode("notForm3", uint8(s2.phase), s2.seats, st.seats);
            if (rtok.balanceOf(c.t.buyerPayout) != b0 + c.a || rtok.balanceOf(c.t.offer.sellerFund) != f0 + c.d) {
                return abi.encode("form3Money", rtok.balanceOf(c.t.buyerPayout), b0, rtok.balanceOf(c.t.offer.sellerFund), f0);
            }
            if (rtok.balanceOf(address(c.inst)) != 0 || s2.unpaid != 0 || s2.deposit != 0) return abi.encode("form3Left", s2.unpaid);
            return "";
        }
        if (uint8(s2.phase) != 1 || s2.seats != st.seats + 1) return abi.encode("noSeat", uint8(s2.phase), s2.seats, st.seats, want);
        IGaleedVenue.Seat memory sq = c.inst.seatOf(c.id, uint64(st.seats));
        uint64 nowT = uint64(_now());
        if (sq.judge != want) return abi.encode("otherMember", sq.judge, want);
        bool good = sq.amanahHash == AMANAH && sq.termsHash == RTERMS && sq.state == 0 && sq.windowStart == nowT
            && sq.accepted == (c.autoAccept ? nowT : 0)
            && sq.windowEnd == nowT + (c.autoAccept ? uint64(c.t.offer.deliveryWindow) : uint64(c.offerWindow))
            && sq.parties[0] == c.t.buyerKey && sq.parties[1] == c.t.offer.sellerKey && sq.parties[2] == c.t.offer.sellerFund
            && sq.parties[3] == c.t.buyerPayout && sq.parties[4] == c.t.offer.sellerPayout;
        if (!good) return abi.encode("seatFields", sq.accepted, sq.windowEnd, sq.state);
        if (rtok.balanceOf(c.t.buyerPayout) != b0 || rtok.balanceOf(c.t.offer.sellerFund) != f0
            || rtok.balanceOf(address(c.inst)) != c.a + c.d) {
            return abi.encode("seatMoney");
        }
        return "";
    }

    /// The draw itself, from a party's or the guild's current key or a stranger at the transaction gas cap: it lands, and its
    /// state and its logs are the model's.
    function _rDraw(uint256 k, address who, bytes memory data, bool fail, address want, string memory what) internal {
        RC storage c = rcases[k];
        Eravon.CaseStatus memory st = c.inst.status();
        uint256 b0 = rtok.balanceOf(c.t.buyerPayout);
        uint256 f0 = rtok.balanceOf(c.t.offer.sellerFund);
        routerDraws++;
        vm.recordLogs();
        (bool ok, bytes memory ret) = _rSend(who, c, data, TX_GAS);
        VmSafe.Log[] memory logs = vm.getRecordedLogs();
        if (!ok) return _rr(abi.encode("drawReverted", what, ret));
        bytes memory why = _rCheck(k, st, fail, want, b0, f0);
        if (why.length != 0) return _rr(abi.encode(what, why));
        if (fail) {
            if (!_logsHave(logs, address(c.inst), Eravon.Resigned.selector, abi.encode(bytes32(0), uint8(3)))) {
                return _rr(abi.encode("noForm3Log", what));
            }
            if (!_logsHave(logs, address(c.inst), Eravon.Closed.selector, abi.encode(uint8(4), c.a, c.d, uint256(0), uint256(0)))) {
                return _rr(abi.encode("closedOff", what));
            }
            c.failed = true;
            c.seatsAtFail = st.seats;
            c.closed = true;
            c.terminal = 8;
            routerFailed++;
            if (st.seats == 1) _mark("router.secondDrawFails");
            if (st.seats == 2) _mark("router.thirdDrawFails");
        } else {
            uint256 seen;
            for (uint256 x = 0; x < logs.length; x++) {
                VmSafe.Log memory l = logs[x];
                if (l.emitter == address(c.inst) && l.topics.length == 3 && l.topics[0] == IGaleedVenue.Seated.selector
                    && l.topics[1] == bytes32(uint256(uint160(want))) && l.topics[2] == c.id
                    && keccak256(l.data) == keccak256(abi.encode(uint64(st.seats)))) seen++;
                if (l.emitter == address(c.inst) && l.topics.length != 0 && l.topics[0] == Eravon.Closed.selector) seen += 100;
            }
            if (seen != 1) return _rr(abi.encode("seatedLog", what, seen));
            routerSeated++;
            if (st.seats == 1) _mark("router.secondDrawSeats");
            if (st.seats == 2) _mark("router.thirdDrawSeats");
        }
        _mark(what);
    }

    /// The lowest gas limit at which the draw lands, found on snapshots: there it does what the model says (the
    /// router answers only where it was handed exactly routeGas()); a limit below it reverts with every effect
    /// undone, answering RouterGas or running out of gas.
    function _rGasProbe(uint256 k, bytes memory data, bool fail, address want, uint256 h, bool called) internal {
        RC storage c = rcases[k];
        address who = _cur(SELLER);
        Eravon.CaseStatus memory st = c.inst.status();
        bytes32 before = _rState(k);
        uint256 lo = 30_000;
        uint256 hi = TX_GAS;
        while (hi - lo > 1) {
            uint256 mid = (lo + hi) / 2;
            uint256 s1 = vm.snapshotState();
            (bool okm,) = _rSend(who, c, data, mid);
            vm.revertToStateAndDelete(s1);
            if (okm) hi = mid;
            else lo = mid;
        }
        routerGasProbes++;
        bytes memory why;
        uint256 b0 = rtok.balanceOf(c.t.buyerPayout);
        uint256 f0 = rtok.balanceOf(c.t.offer.sellerFund);
        uint256 s2 = vm.snapshotState();
        (bool ok2, bytes memory r2) = _rSend(who, c, data, hi);
        if (!ok2) why = abi.encode("limitDoesNotLand", hi, r2);
        else why = _rCheck(k, st, fail, want, b0, f0);
        vm.revertToStateAndDelete(s2);
        // a router whose code hash differs is never called, and its draw needs no routeGas() (s7)
        if (called && hi <= R_ROUTE_GAS) return _rr(abi.encode("landsBelowRouteGas", hi));
        if (!called) _mark("router.gas.hashTestedFirst");
        if (why.length != 0) return _rr(abi.encode("atTheLimit", hi, why));
        _mark("router.gas.landsAtTheLimit");
        uint256 g = hi - 1 - (h >> 64) % 200_000;
        if (g < 30_000) g = 30_000;
        uint256 s3 = vm.snapshotState();
        (bool ok3, bytes memory r3) = _rSend(who, c, data, g);
        bytes32 after3 = _rState(k);
        vm.revertToStateAndDelete(s3);
        bool named = r3.length == 4 && bytes4(r3) == Eravon.RouterGas.selector;
        if (ok3 || (r3.length != 0 && !named) || after3 != before) return _rr(abi.encode("belowTheLimit", g, hi, ok3, r3));
        _mark("router.gas.refusedBelow");
        if (named) _mark("router.gas.routerGasNamed");
    }

    /// On a snapshot, the armed block's hash lost (257 blocks on): the draw, or advance once the deadline has passed,
    /// ends the case as Resigned with form 3, whatever the router would answer (s7).
    function _rSeedProbe(uint256 k, bytes memory data, uint256 h) internal {
        RC storage c = rcases[k];
        Eravon.CaseStatus memory st = c.inst.status();
        uint256 b0 = rtok.balanceOf(c.t.buyerPayout);
        uint256 f0 = rtok.balanceOf(c.t.offer.sellerFund);
        uint256 bn0 = vm.getBlockNumber();
        uint256 ts0 = _now();
        uint256 s = vm.snapshotState();
        if (vm.getBlockNumber() < uint256(st.drawBlock) + 257) vm.roll(uint256(st.drawBlock) + 257);
        // one probe in two with the deadline passed too: then advance closes it
        if ((h >> 80) % 2 == 0 && _now() <= st.redrawDeadline) vm.warp(uint256(st.redrawDeadline) + 1);
        bool viaRedraw = _now() <= st.redrawDeadline;
        bool ok;
        bytes memory r;
        if (viaRedraw) {
            (ok, r) = _rSend(_cur(BUYER), c, data, TX_GAS);
        } else {
            vm.prank(STRANGER, STRANGER);
            (ok, r) = address(c.inst).call{gas: TX_GAS}(abi.encodeCall(Eravon.advance, (c.id, 0)));
        }
        bytes memory why = ok ? _rCheck(k, st, true, address(0), b0, f0) : abi.encode("seedProbeReverted", r);
        vm.revertToStateAndDelete(s);
        if (vm.getBlockNumber() != bn0) vm.roll(bn0);
        if (_now() != ts0) vm.warp(ts0);
        if (why.length != 0) return _rr(abi.encode("seedLost", viaRedraw, why));
        _mark(viaRedraw ? "router.seedLost.redraw" : "router.seedLost.advance");
    }

    /// s9: a former key of the guild engages nothing on the book (the draw is anyone's, s7, and the model sends it
    /// from a stranger too).
    function _rStrangers(uint256 k, bytes memory) internal {
        RC storage c = rcases[k];
        bool ok;
        bytes memory r;
        for (uint256 x = 0; x < rguildOld.length; x++) {
            (address[] memory m, bytes32[] memory w) = _rPanel(c.pv);
            (ok, r) = _tryAs(
                rguildOld[x],
                address(pbook),
                abi.encodeCall(EravonPanelBook.engage, (logic.PROXY_CODEHASH(), m, w, ROSTER, uint64(_now() + 1 days), address(c.router)))
            );
            if (ok || r.length != 4 || bytes4(r) != EravonPanelBook.NotCurrent.selector) return _rr(abi.encode("formerGuildEngage", ok, r));
            _mark("router.formerGuildKeyRefused");
        }
    }

    /// The router guild's record moves to a fresh key mid-case (galeed.roster/1 s4): its later records and draws come
    /// from the new key, the cases keep the root they froze.
    function _rMoveGuild() internal {
        address from = _cur(RGUILD);
        address to = address(uint160(uint256(keccak256(abi.encode("eravon/fuzz/rguild", rguildMoves)))));
        moveKey(from, to);
        rguildOld.push(from);
        rguildMoves++;
        if (_cur(RGUILD) != to || sbook.root(to) != RGUILD) return _rr(abi.encode("guildMove", from, to));
        _mark("router.guildKeyMoved");
    }

    /// An unaccepted seat: the judge's acceptance, its decline, its offer clock run out, or the seller's refund.
    function _rOffered(uint256 k, uint64 ord, IGaleedVenue.Seat memory L, uint256 h) internal {
        RC storage c = rcases[k];
        address jk = _cur(L.judge);
        bool coded = jk.code.length != 0;
        bool late = _now() > L.windowEnd;
        uint256 x = h % 4;
        if (coded && (x == 0 || x == 1)) {
            // venue/1 s2.7: a judge key that runs code acts directly for nothing
            (bool okc, bytes memory rc_) = _rJudgeSend(L.judge, c, abi.encodeCall(Eravon.accept, (c.id, ord)));
            if (okc || rc_.length != 4 || bytes4(rc_) != Eravon.NotDirect.selector) return _rr(abi.encode("codedJudge", okc, rc_));
            _mark("router.codedJudgeNotDirect");
            x = 2;
        }
        if (x == 0 && !late) {
            (bool ok, bytes memory r) = _rJudgeSend(L.judge, c, abi.encodeCall(Eravon.accept, (c.id, ord)));
            if (!ok) return _rr(abi.encode("accept", r));
            IGaleedVenue.Seat memory sq = c.inst.seatOf(c.id, ord);
            if (sq.accepted != uint64(_now()) || sq.windowEnd != uint64(_now()) + c.t.offer.deliveryWindow || sq.state != 0) {
                return _rr(abi.encode("acceptFields"));
            }
            _mark("router.accepted");
            return;
        }
        if (x == 1) {
            // the decline (venue/1 s2.10): the next draw, or the case ends after three seats (s7)
            vm.recordLogs();
            (bool ok, bytes memory r) = _rJudgeSend(L.judge, c, abi.encodeCall(Eravon.resign, (c.id, ord, E, ENTRY)));
            VmSafe.Log[] memory logs = vm.getRecordedLogs();
            if (!ok) return _rr(abi.encode("decline", r));
            if (!_logsHave(logs, address(c.inst), Eravon.Resigned.selector, abi.encode(E, uint8(0)))) return _rr(abi.encode("declineLog"));
            return _rAfterOffer(k, ord, 3, logs, "router.declinedRearmed");
        }
        if (x == 2 || late) {
            if (!late) vm.warp(uint256(L.windowEnd) + 1);
            vm.recordLogs();
            vm.prank(STRANGER, STRANGER);
            (bool ok, bytes memory r) = address(c.inst).call{gas: TX_GAS}(abi.encodeCall(Eravon.advance, (c.id, ord)));
            VmSafe.Log[] memory logs = vm.getRecordedLogs();
            if (!ok) return _rr(abi.encode("expire", r));
            return _rAfterOffer(k, ord, 5, logs, "router.expiredRearmed");
        }
        _rClose(k, 0, "router.refundedOffered");
    }

    /// After an unaccepted seat ended: a draw armed from this block, or, three seats opened, the case Resigned with
    /// form 3, the amount to the buyer and the deposit to the fund.
    function _rAfterOffer(uint256 k, uint64 ord, uint8 seatState, VmSafe.Log[] memory logs, string memory what) internal {
        RC storage c = rcases[k];
        Eravon.CaseStatus memory s2 = c.inst.status();
        if (c.inst.seatOf(c.id, ord).state != seatState) return _rr(abi.encode("endedSeatState", seatState));
        if (s2.seats >= 3) {
            if (uint8(s2.phase) != 8 || s2.pendingRedraw || !_logsHave(logs, address(c.inst), Eravon.Resigned.selector, abi.encode(bytes32(0), uint8(3)))
                || !_logsHave(logs, address(c.inst), Eravon.Closed.selector, abi.encode(uint8(4), c.a, c.d, uint256(0), uint256(0)))) {
                return _rr(abi.encode("thirdSeat", uint8(s2.phase)));
            }
            c.closed = true;
            c.terminal = 8;
            _mark("router.thirdSeatEnds");
            return;
        }
        if (uint8(s2.phase) != 1 || !s2.pendingRedraw || s2.drawBlock != vm.getBlockNumber()
            || s2.redrawDeadline != uint64(_now()) + c.offerWindow) {
            return _rr(abi.encode("rearm", uint8(s2.phase), s2.pendingRedraw));
        }
        _mark(what);
    }

    /// advance on a draw past its deadline (beyond the first block) closes the case as Resigned with form 3 (s7).
    function _rAdvanceCloses(uint256 k, Eravon.CaseStatus memory st, string memory what) internal {
        RC storage c = rcases[k];
        uint256 b0 = rtok.balanceOf(c.t.buyerPayout);
        uint256 f0 = rtok.balanceOf(c.t.offer.sellerFund);
        vm.prank(STRANGER, STRANGER);
        (bool ok, bytes memory r) = address(c.inst).call{gas: TX_GAS}(abi.encodeCall(Eravon.advance, (c.id, uint64(st.seats))));
        if (!ok) return _rr(abi.encode("advanceDeadline", r));
        bytes memory why = _rCheck(k, st, true, address(0), b0, f0);
        if (why.length != 0) return _rr(abi.encode(what, why));
        c.closed = true;
        c.terminal = 8;
        _mark(what);
    }

    /// An accepted seat: T1 run out, the seller's refund, or the delivery claimed and confirmed (P to the judge), then
    /// one time in two the judge's withdraw naming its payout (registered there by that call, venue/1 s3).
    function _rAccepted(uint256 k, uint64, IGaleedVenue.Seat memory L, uint256 h) internal {
        RC storage c = rcases[k];
        if (_now() > L.windowEnd) return _rClose(k, 3, "router.t1");
        if (h % 2 == 0) return _rClose(k, 0, "router.refundedAccepted");
        (bool ok, bytes memory r) = _rSend(_cur(SELLER), c, abi.encodeCall(Eravon.claimDelivery, (keccak256(abi.encode(h, "manifest")))), TX_GAS);
        if (!ok) return _rr(abi.encode("claim", r));
        _rClose(k, 1, "router.released");
        if (!c.closed || (h >> 8) % 2 == 0) return;
        address j = L.judge;
        address p = payoutOfJudge[j];
        address jk = _cur(j);
        if (p == address(0) || jk.code.length != 0) return;
        uint256 p0 = rtok.balanceOf(p);
        vm.prank(jk, jk);
        (ok, r) = address(c.inst).call{gas: TX_GAS}(abi.encodeCall(Eravon.withdraw, (address(rtok), j, p)));
        (address reg, uint256 left) = c.inst.accountOf(j, address(rtok));
        if (!ok || reg != p || left != 0 || rtok.balanceOf(p) != p0 + c.p || c.inst.status().unpaid != 0) {
            return _rr(abi.encode("withdraw", ok, r, reg, left));
        }
        c.judgeOwed = 0;
        _mark("router.judgeWithdrew");
    }

    /// One closing act and its Closed: 0 the seller's refund (Refunded, P and the deposit to the fund), 1 the buyer's
    /// confirmation (Released, A to the seller's payout, P to the seat), 3 advance past T1 (Refunded cause 0).
    function _rClose(uint256 k, uint8 kind, string memory what) internal {
        RC storage c = rcases[k];
        uint64 ord = uint64(c.inst.status().seats - 1);
        address j = c.inst.seatOf(c.id, ord).judge;
        bytes memory data = kind == 0
            ? abi.encodeCall(Eravon.refund, ())
            : (kind == 1 ? abi.encodeCall(Eravon.confirm, ()) : abi.encodeCall(Eravon.advance, (c.id, ord)));
        address who = kind == 0 ? _cur(SELLER) : (kind == 1 ? _cur(BUYER) : STRANGER);
        vm.recordLogs();
        (bool ok, bytes memory r) = _rSend(who, c, data, TX_GAS);
        VmSafe.Log[] memory logs = vm.getRecordedLogs();
        if (!ok) return _rr(abi.encode(what, r));
        bytes memory closedData = kind == 1
            ? abi.encode(uint8(1), uint256(0), c.a + c.lf, c.p, uint256(0))
            : abi.encode(uint8(0), c.a, c.d, uint256(0), uint256(0));
        if (!_logsHave(logs, address(c.inst), Eravon.Closed.selector, closedData)) return _rr(abi.encode(what, "closed"));
        Eravon.CaseStatus memory s2 = c.inst.status();
        uint256 owedJ = kind == 1 ? c.p : 0;
        (, uint256 cl) = c.inst.accountOf(j, address(rtok));
        IGaleedVenue.Seat memory sq = c.inst.seatOf(c.id, ord);
        if (uint8(s2.phase) != (kind == 1 ? 5 : 4) || s2.unpaid != owedJ || cl != owedJ || rtok.balanceOf(address(c.inst)) != owedJ
            || sq.standbyCredited != owedJ || sq.rulingCredited != 0 || sq.state != 5) {
            return _rr(abi.encode(what, uint8(s2.phase), s2.unpaid, cl));
        }
        c.closed = true;
        c.terminal = uint8(s2.phase);
        c.judgeOwed = owedJ;
        _mark(what);
    }

    function warp(uint256 dt) external {
        uint256 hs = _h(dt) % 10;
        uint256 span = hs < 7 ? 1 days : (hs < 9 ? 5 days : 20 days);
        vm.warp(_now() + bound(dt, 1, span));
        vm.roll(vm.getBlockNumber() + 1);
    }
}

/// The campaign: each function one property of the standard, read off the
/// handler's cases after every call.
contract EravonInvariants is Test {
    EravonHandler internal h;

    function setUp() public {
        h = new EravonHandler();
        targetContract(address(h));
        bytes4[] memory sel = new bytes4[](65);
        sel[0] = h.newOrder.selector;
        sel[1] = h.newSibling.selector;
        sel[2] = h.doInit.selector;
        sel[3] = h.doSecondInit.selector;
        sel[4] = h.doRace.selector;
        sel[5] = h.doCancel.selector;
        sel[6] = h.doRevoke.selector;
        sel[7] = h.doReengage.selector;
        sel[8] = h.doAccept.selector;
        sel[9] = h.doRule.selector;
        sel[10] = h.doResign.selector;
        sel[11] = h.doAdvance.selector;
        sel[12] = h.doRedraw.selector;
        sel[13] = h.doCommit.selector;
        sel[14] = h.doClaim.selector;
        sel[15] = h.doConfirm.selector;
        sel[16] = h.doDispute.selector;
        sel[17] = h.doRefund.selector;
        sel[18] = h.doSettle.selector;
        sel[19] = h.doStray.selector;
        sel[20] = h.doSweep.selector;
        sel[21] = h.doSetPayout.selector;
        sel[22] = h.doWithdraw.selector;
        sel[23] = h.doBlock.selector;
        sel[24] = h.impostorPull.selector;
        sel[25] = h.warp.selector;
        sel[26] = h.doBurn.selector;
        sel[27] = h.doDrawEdge.selector;
        sel[28] = h.doAdvanceEdge.selector;
        sel[29] = h.doRefundEdge.selector;
        sel[30] = h.doSettleEdge.selector;
        sel[31] = h.newGhostOrder.selector;
        sel[32] = h.doGhost.selector;
        sel[33] = h.doSweepSecond.selector;
        sel[34] = h.doBadBlob.selector;
        sel[35] = h.doDrawClocks.selector;
        sel[36] = h.doDrawClocks.selector; // the draw's clocks twice as often: most cases leave their draw at once
        sel[37] = h.doAdvanceDraw.selector;
        sel[38] = h.doSettleVsClock.selector;
        sel[39] = h.doRefundAtT3.selector;
        sel[40] = h.doRefundSig.selector;
        sel[41] = h.doFeedInit.selector;
        sel[42] = h.doShape.selector;
        sel[43] = h.doLenProbe.selector;
        sel[44] = h.doRefundInDispute.selector;
        sel[45] = h.doStaleSig.selector;
        sel[46] = h.doPair.selector;
        sel[47] = h.doPair.selector; // the pairs twice as often: the order of every pair is the round's subject
        sel[48] = h.doReorg.selector;
        sel[49] = h.doThreeSeats.selector;
        sel[50] = h.doForeignHostile.selector;
        sel[51] = h.doOddDecimals.selector;
        sel[52] = h.doFlipFeed.selector;
        sel[53] = h.doDelegate.selector;
        sel[54] = h.doGrief.selector;
        sel[55] = h.doInitOrder.selector;
        sel[56] = h.doFund.selector;
        sel[57] = h.doFund.selector; // the funds' own acts twice as often: the bond and the allowance are the round's subject
        sel[58] = h.doPool.selector;
        sel[59] = h.doPool.selector; // round 7: the pools twice as often
        sel[60] = h.doFundOrder.selector;
        sel[61] = h.doMoveKey.selector;
        sel[62] = h.doFormerKey.selector;
        sel[63] = h.doRouter.selector; // FUZZ r24-1: the guild draw under a router, twice as often
        sel[64] = h.doRouter.selector;
        targetSelector(FuzzSelector({addr: address(h), selectors: sel}));
    }

    function _tok() internal view returns (address) {
        return h.token();
    }

    function _status(address inst)
        internal
        view
        returns (uint8 ph, uint256 seats, uint256 premium, uint256 deposit, uint256 lfHeld, uint256 unpaid)
    {
        Eravon.CaseStatus memory x = Eravon(inst).status();
        (ph, seats, premium, deposit, lfHeld, unpaid) = (uint8(x.phase), x.seats, x.premium, x.deposit, x.lfHeld, x.unpaid);
    }

    // ---- the ten of the original campaign ----

    /// s8: the balance covers the ledger, the deposit held is the one pulled, and the registry holds nothing.
    function invariant_01_balanceCoversTheLedger() public view {
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst,,, uint256 a,,,, uint8 ph,, uint256 pulled) = h.caseAt(i);
            Eravon.CaseStatus memory xs_ = Eravon(inst).status();
            uint256 premium = xs_.premium;
            uint256 deposit = xs_.deposit;
            uint256 lfHeld = xs_.lfHeld;
            uint256 unpaid = xs_.unpaid;
            uint256 held = unpaid;
            if (ph >= 1 && ph <= 3) held += a + deposit + lfHeld;
            assertGe(IERC20Like(_tok()).balanceOf(inst), held, "balance covers the ledger");
            if (ph >= 1 && ph <= 3) {
                assertEq(deposit, pulled, "the deposit held is the deposit pulled");
                assertLe(premium, deposit, "P is part of D");
            } else {
                assertEq(deposit, 0, "nothing held after the terminal");
                assertEq(lfHeld, 0);
            }
        }
        assertEq(IERC20Like(_tok()).balanceOf(h.logicAddr()), 0, "the logic holds no money");
    }

    /// s5: terminals are final and the seat machine moves one way.
    function invariant_02_terminalsFinalSeatMachine() public view {
        assertEq(h.phaseWentBack(), 0, "a phase went back or a terminal changed");
        assertEq(h.seatMachineBroken(), 0, "a seat left its machine");
        assertEq(h.closedTwice(), 0, "Closed twice");
    }

    /// s6: Anchored only from rule and resign.
    function invariant_03_anchoredOnlyInRuleAndResign() public view {
        assertEq(h.anchoredOutside(), 0);
    }

    /// s4: cancel only before any case; the expiry branch and cancel never pulled a deposit.
    function invariant_04_cancelOnlyBeforeInit() public view {
        assertEq(h.cancelAfterInit(), 0);
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (,,,,,,, uint8 ph,, uint256 pulled) = h.caseAt(i);
            if (ph == 0) assertEq(pulled, 0, "no deposit before a case");
        }
    }

    /// s7a: pull serves only the calling instance, for its own deposit; impostors fail.
    function invariant_05_pullOnlyToTheCaller() public view {
        assertEq(h.pullNotToCaller(), 0);
        assertEq(h.impostorPullPassed(), 0);
    }

    /// s8: the Closed sums are exactly what the instance held beyond prior credits.
    function invariant_06_closedSums() public view {
        assertEq(h.closedSumWrong(), 0);
    }

    /// s8: a judge earns only where the trade completed or it ruled, and only while its seat stood.
    function invariant_07_judgeIncomeOnlyWhereEarned() public view {
        address[] memory judges = h.allJudges();
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst, bytes32 id,,,,,, uint8 ph,,) = h.caseAt(i);
            if (ph == 0) continue;
            Eravon.CaseStatus memory xs_ = Eravon(inst).status();
            uint256 seats = xs_.seats;
            uint64 claimedAt = xs_.claimedAt;
            uint64 disputedAt = xs_.disputedAt;
            for (uint256 j = 0; j < judges.length; j++) {
                (, uint256 c) = Eravon(inst).accountOf(judges[j], _tok());
                if (c == 0) continue;
                // a Refunded pays the judge only at a refund while Claimed under a seated seat (claimedAt set,
                // disputedAt zero); a refund during a dispute within T3 is refused
                assertTrue(ph == 5 || ph == 6 || ph == 7 || (ph == 4 && claimedAt != 0 && disputedAt == 0), "judge income outside Released, Settled, Ruled, a refund while Claimed");
                IGaleedVenue.Seat memory s = Eravon(inst).seatOf(id, uint64(seats - 1));
                assertEq(s.judge, judges[j], "income to the last seat alone");
                assertTrue(s.state == 2 || s.state == 5, "income only where the seat stood to the end");
            }
        }
    }

    /// venue/1 s3: no party money passes through the seat ledger; no party has a registered payout from the quote.
    function invariant_08_noPartyMoneyInClaimable() public view {
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst,,,,,,, uint8 ph,,) = h.caseAt(i);
            if (ph == 0) continue;
            (address bk, address sk, address bp, address sp) = h.caseParties(i);
            (address fd,) = h.caseFund(i);
            (,, address credited,,,) = h.caseGhost(i);
            address[5] memory parties = [bk, sk, bp, sp, fd];
            for (uint256 k = 0; k < 5; k++) {
                // round 7: a fund that is the credited judge holds its seat's claimable (invariants 14 and 62)
                if (k == 4 && fd == credited) continue;
                (, uint256 c) = Eravon(inst).accountOf(parties[k], _tok());
                assertEq(c, 0, "party money in claimable");
            }
        }
    }

    /// s5: rung bounds and parties as the interface shows them; the named track has one seat, the guild at most three distinct members.
    function invariant_09_seatsAndRungs() public view {
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst, bytes32 id, bool g, uint256 a,,,, uint8 ph,,) = h.caseAt(i);
            if (ph == 0) continue;
            Eravon.CaseStatus memory xs_ = Eravon(inst).status();
            uint256 seats = xs_.seats;
            uint256 panelSize = xs_.panelSize;
            if (seats == 0) {
                if (!g) assertEq(ph, 4, "no seat on the named track only where the buyer cancelled before any case");
                else assertTrue(ph == 1 || ph == 4 || ph == 6 || ph == 8, "no seat on the guild track only while a draw waits or the case closed without one");
                continue;
            }
            if (!g) assertEq(seats, 1, "one seat on the named track");
            else assertTrue(seats >= 1 && seats <= 3 && seats <= panelSize, "guild seats bounded");
            for (uint256 s = 0; s < seats; s++) {
                IGaleedVenue.Seat memory st = Eravon(inst).seatOf(id, uint64(s));
                assertEq(st.rungMin, 0);
                assertEq(st.rungMax, st.rungs - 1);
                assertEq(st.amount, a);
                assertEq(st.parties.length, 5);
                for (uint256 r = 0; r < s; r++) {
                    assertTrue(Eravon(inst).seatOf(id, uint64(r)).judge != st.judge, "a member seated twice");
                }
                // only the last seat may be in flight
                if (s + 1 < seats) assertTrue(st.state >= 3, "an earlier seat still in flight");
            }
        }
    }

    /// Conservation: every token minted is at a known address.
    function invariant_10_conservation() public view {
        _conserved(IERC20Like(_tok()), h.totalMinted());
    }

    function _conserved(IERC20Like t, uint256 minted) internal view {
        uint256 sum;
        address[] memory a = h.holders();
        for (uint256 i = 0; i < a.length; i++) {
            sum += t.balanceOf(a[i]);
        }
        address[] memory payouts = h.allPayouts();
        for (uint256 i = 0; i < payouts.length; i++) {
            sum += t.balanceOf(payouts[i]);
        }
        address[] memory judges = h.allJudges();
        for (uint256 i = 0; i < judges.length; i++) {
            sum += t.balanceOf(judges[i]);
        }
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst,,,,,,,,,) = h.caseAt(i);
            sum += t.balanceOf(inst);
        }
        for (uint256 i = 0; i < h.ghostCount(); i++) {
            sum += t.balanceOf(h.ghostAt(i));
        }
        assertEq(sum, minted, "tokens left the known addresses");
    }

    // ---- the audit round's additions ----

    /// s8: balance = A + D + the fee held + unpaid + strays while open, unpaid + strays after a
    /// terminal, the strays being what the handler sent in unrecorded: `_stray()` is exactly the excess.
    function invariant_11_strayIsExactlyTheExcess() public view {
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst,,, uint256 a,,,, uint8 ph,,) = h.caseAt(i);
            if (ph == 0) continue;
            (, uint256 stray,,,,) = h.caseGhost(i);
            (,,, uint256 deposit, uint256 lfHeld, uint256 unpaid) = _status(inst);
            uint256 held = unpaid + stray;
            if (ph <= 3) held += a + deposit + lfHeld;
            assertEq(IERC20Like(_tok()).balanceOf(inst), held, "balance is the ledger plus the strays");
        }
    }

    /// s8: `unpaid` is the parties' credits plus the seats' claimable, and nobody else is owed anything.
    function invariant_12_unpaidIsTheLedgers() public view {
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst, bytes32 id,,,,,, uint8 ph,,) = h.caseAt(i);
            if (ph == 0) continue;
            (address bk, address sk, address bp, address sp) = h.caseParties(i);
            (address fd,) = h.caseFund(i);
            (, uint256 seats,,,, uint256 unpaid) = _status(inst);
            uint256 sum = Eravon(inst).owed(bp, _tok());
            if (sp != bp) sum += Eravon(inst).owed(sp, _tok());
            // round 6: the deposit's creditor is sellerFund; the key is owed nothing where it is not the fund or a payout
            if (fd != bp && fd != sp) sum += Eravon(inst).owed(fd, _tok());
            if (sk != bp && sk != sp && sk != fd) assertEq(Eravon(inst).owed(sk, _tok()), 0, "the seller's key is owed nothing");
            if (bk != bp && bk != sp && bk != sk && bk != fd) assertEq(Eravon(inst).owed(bk, _tok()), 0, "the buyer's key is owed nothing");
            for (uint256 s = 0; s < seats; s++) {
                (, uint256 c) = Eravon(inst).accountOf(Eravon(inst).seatOf(id, uint64(s)).judge, _tok());
                sum += c;
            }
            assertEq(sum, unpaid, "unpaid is the ledgers");
            assertEq(Eravon(inst).owed(bp, h.foreignToken()), 0, "no ledger in a foreign token");
        }
    }

    /// venue/1 s3: no party key or payout ever has a claimable; no seat's judge is ever owed.
    function invariant_13_noPartyClaimableNoJudgeOwed() public view {
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst, bytes32 id,,,,,, uint8 ph,,) = h.caseAt(i);
            if (ph == 0) continue;
            (address bk, address sk, address bp, address sp) = h.caseParties(i);
            (address fd,) = h.caseFund(i);
            (,, address credited,,,) = h.caseGhost(i);
            address[5] memory parties = [bk, sk, bp, sp, fd];
            for (uint256 k = 0; k < 5; k++) {
                if (k == 4 && fd == credited) continue; // round 7: the credited judge as the fund (invariant 62)
                (, uint256 c) = Eravon(inst).accountOf(parties[k], _tok());
                assertEq(c, 0, "a party has a claimable");
                (, c) = Eravon(inst).accountOf(parties[k], h.foreignToken());
                assertEq(c, 0, "a party has a claimable in a foreign token");
            }
            (, uint256 seats,,,,) = _status(inst);
            for (uint256 s = 0; s < seats; s++) {
                address j = Eravon(inst).seatOf(id, uint64(s)).judge;
                // round 7: a seat's judge that is the case's sellerFund is owed the deposit's return alone (invariant 62)
                if (j != fd) assertEq(Eravon(inst).owed(j, _tok()), 0, "a judge is owed party money");
                assertTrue(j != bk && j != sk, "a party seated as judge");
            }
        }
    }

    /// s8: P is credited only at Released, Settled or Ruled, or at a refund while Claimed under a seated seat
    /// (Refunded with claimedAt set and disputedAt zero), to the last seat, which stands at 5 or 2; the seat
    /// ledger is what Closed credited less what left.
    function invariant_14_judgeCreditOnlyWhereEarned() public view {
        assertEq(h.judgeCreditWrong(), 0, "a credit outside the table");
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst, bytes32 id,,,,,, uint8 ph,,) = h.caseAt(i);
            if (ph == 0) continue;
            (,, address credited, uint256 judgeCredited, uint256 judgeOut,) = h.caseGhost(i);
            Eravon.CaseStatus memory xs_ = Eravon(inst).status();
            uint256 seats = xs_.seats;
            uint64 claimedAt = xs_.claimedAt;
            uint64 disputedAt = xs_.disputedAt;
            for (uint256 s = 0; s < seats; s++) {
                address j = Eravon(inst).seatOf(id, uint64(s)).judge;
                (, uint256 c) = Eravon(inst).accountOf(j, _tok());
                if (j == credited) assertEq(c, judgeCredited - judgeOut, "the seat ledger is the credit less what left");
                else assertEq(c, 0, "a seat that earned nothing holds a claimable");
            }
            if (judgeCredited != 0) {
                assertTrue(ph == 5 || ph == 6 || ph == 7 || (ph == 4 && claimedAt != 0 && disputedAt == 0), "credited outside Released, Settled, Ruled, a refund while Claimed");
                IGaleedVenue.Seat memory last = Eravon(inst).seatOf(id, uint64(seats - 1));
                assertEq(last.judge, credited, "credited to the last seat");
                assertTrue(last.state == 5 || last.state == 2, "credited to a seat that did not stand");
            }
        }
    }

    /// s7a: every Pulled names the calling instance and exactly its P + lf, once per opening init (on the guild
    /// track before any seat is drawn), never otherwise: a cancelled or unopened order pulled nothing.
    function invariant_15_pulledIsTheDeposit() public view {
        assertEq(h.pullNotToCaller(), 0, "a pull to another receiver or for another sum");
        assertEq(h.pullCountWrong(), 0, "a pull outside the opening init");
        assertEq(h.eventOnWrongEmitter(), 0, "a logic event from an instance");
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (,,,, uint256 d,,,,, uint256 pulled) = h.caseAt(i);
            if (h.caseOpened(i)) assertEq(pulled, d, "the deposit pulled is P + lf");
            else assertEq(pulled, 0, "pulled without a case");
        }
    }

    /// s4, s5, s7: Refunded(4) only from phase None, Refunded's other causes only from their paths, the
    /// expiry branch opens nothing and leaves only the buyer's credit, a revoked document opens no case and
    /// touches no open one; the draw refuses after its deadline from the second block after the arming on,
    /// with another panel, and in its own block.
    function invariant_16_gateAndCauses() public view {
        assertEq(h.cancelAfterInit(), 0, "Refunded(4) outside phase None");
        assertEq(h.causeWrong(), 0, "a Refunded cause from the wrong path");
        assertEq(h.expiryBranchWrong(), 0, "the expiry branch misbehaved");
        assertEq(h.revokedOpened(), 0, "a dead or revoked order opened");
        assertEq(h.revokeTouchedOpen(), 0, "a revocation touched a case");
        assertEq(h.secondInitPassed(), 0, "a second init or a cancel after init passed");
        assertEq(h.raceBroken(), 0, "both sides of a cancel-init race landed");
        assertEq(h.refusedRedrawPassed(), 0, "a late redraw, one with another panel, or one in the armed block passed");
    }

    /// s6: Anchored only in rule and resign, once, from the instance, naming the seat's judge and the entry.
    function invariant_17_anchoredShape() public view {
        assertEq(h.anchoredOutside(), 0, "Anchored outside rule and resign");
        assertEq(h.anchorWrong(), 0, "Anchored of the wrong shape");
    }

    /// Conservation of the foreign token as well.
    function invariant_18_conservationForeign() public view {
        _conserved(IERC20Like(h.foreignToken()), h.totalMintedForeign());
    }

    /// s5, s7: the table of phases and the last seat's state and window (a guild case with no seat only
    /// while its draw waits or after it closed without one); the seat machine; the draw machine (armed at
    /// the guild's init and after an unaccepted seat, on that block, deadline offerWindow on, kept with its
    /// block and deadline until a draw or a terminal; a seed past the blockhash window closes the case).
    function invariant_19_theTableOfS5() public view {
        assertEq(h.seatTableBroken(), 0, "a phase and its seat disagree with s5");
        assertEq(h.seatMachineBroken(), 0, "a seat transition outside s5");
        assertEq(h.drawWrong(), 0, "the pending draw's block, deadline or event outside s7");
    }

    /// s8: every Closed matches the terminal table computed from the pre-state.
    function invariant_20_closedIsTheTable() public view {
        assertEq(h.closedModelWrong(), 0, "a Closed outside the table of s8");
    }

    /// s4, s7: one seat on the named track; on the guild tracks at most three distinct members within the pool.
    function invariant_21_seatsPerTrack() public view {
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst, bytes32 id,,,,,, uint8 ph,,) = h.caseAt(i);
            if (ph == 0) continue;
            (uint8 track,,,,,) = h.caseGhost(i);
            Eravon.CaseStatus memory xs_ = Eravon(inst).status();
            uint256 seats = xs_.seats;
            uint256 panelSize = xs_.panelSize;
            if (seats == 0) continue;
            if (track == 0) assertEq(seats, 1, "one seat on the named track");
            if (track == 1) assertTrue(seats <= 3 && seats <= panelSize, "the guild's seats bounded");
            if (track == 2) {
                assertEq(panelSize, 2, "the parties are not in the pool");
                assertTrue(seats <= 2, "seats within the pool");
            }
            if (track == 3) {
                assertEq(panelSize, 5, "the guild of five's pool");
                assertTrue(seats <= 3, "three seats, not the pool, bound the guild of five");
            }
            for (uint256 s = 1; s < seats; s++) {
                address js = Eravon(inst).seatOf(id, uint64(s)).judge;
                for (uint256 r = 0; r < s; r++) {
                    assertTrue(Eravon(inst).seatOf(id, uint64(r)).judge != js, "a member seated twice");
                }
            }
        }
    }

    /// s7a: the deposit held equals the deposit pulled while open, zero after.
    function invariant_22_depositHeldIsPulled() public view {
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst,,,, uint256 d,, uint256 p, uint8 ph,, uint256 pulled) = h.caseAt(i);
            if (ph == 0) continue;
            (,, uint256 premium, uint256 deposit,,) = _status(inst);
            if (ph <= 3) {
                assertEq(deposit, pulled, "held is pulled");
                assertEq(deposit, d, "held is P + lf");
                assertEq(premium, p, "P as the quote fixes it");
            } else {
                assertEq(deposit, 0, "nothing held after a terminal");
                assertEq(premium, 0);
            }
        }
    }

    /// s4 to s9: every action the standard owes success succeeded (the failing action and its bytes are kept).
    function invariant_23_liveness() public view {
        assertEq(h.livenessBroken(), 0, string(abi.encodePacked("an owed action reverted: act ", vm.toString(h.lastLivenessAct()), " ", vm.toString(h.lastLivenessErr()))));
    }

    /// venue/1 s2.2, s7: one Seated per seat, its ordinal, case and judge, on the guild tracks only by a draw and
    /// the member s7's formula names; accept emits nothing, commit its one Committed.
    function invariant_24_seatedAndEvents() public view {
        assertEq(h.seatedWrong(), 0, "a Seated missing, extra or wrong");
        assertEq(h.eventWrong(), 0, "accept or commit emitted outside the text");
    }

    // ---- round 2 ----

    /// s5, s7: a case has no seat exactly while a guild's draw is armed, or after a guild case closed without
    /// one (the seller's refund, a settlement, or form 3 by advance or by a redraw on a lost seed), or where
    /// the buyer cancelled before any case.
    function invariant_25_zeroSeatsExactlyWhileArmedOrClosedWithout() public view {
        assertEq(h.zeroSeatWrong(), 0, "a seatless close outside the three");
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst,, bool g,,,,, uint8 ph,,) = h.caseAt(i);
            if (ph == 0) continue;
            Eravon.CaseStatus memory xs_ = Eravon(inst).status();
            uint256 seats = xs_.seats;
            bool pending = xs_.pendingRedraw;
            if (pending) assertTrue(ph == 1 && g, "a draw armed outside a Funded guild case");
            if (seats != 0) continue;
            if (!h.caseOpened(i)) {
                assertEq(ph, 4, "an order never opened holds a cancelled case or none");
                continue;
            }
            assertTrue(g, "a named case without its seat");
            if (ph == 1) assertTrue(pending, "a Funded guild case with no seat and no draw armed");
            else assertTrue(h.noSeatCloseOf(i) != 0 && (ph == 4 || ph == 6 || ph == 8), "closed without a seat otherwise");
        }
    }

    /// s7: every drawn Seated lands in a block after the block whose hash seeded it.
    function invariant_26_drawBlockBelowEverySeated() public view {
        assertEq(h.seatedNotAfterDraw(), 0, "a member seated in or before the armed block");
    }

    /// s7: every seated member of a guild case is in the frozen panel's pool (neither party's key nor zero),
    /// none twice.
    function invariant_27_seatedFromThePoolOnce() public view {
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst, bytes32 id, bool g,,,,, uint8 ph,,) = h.caseAt(i);
            if (ph == 0 || !g) continue;
            (address bk, address sk,,) = h.caseParties(i);
            address[] memory pool = h.frozenMembers(i);
            Eravon.CaseStatus memory xs_ = Eravon(inst).status();
            uint256 seats = xs_.seats;
            for (uint256 s = 0; s < seats; s++) {
                address j = Eravon(inst).seatOf(id, uint64(s)).judge;
                bool inPool;
                for (uint256 k = 0; k < pool.length; k++) {
                    if (pool[k] == j) inPool = true;
                }
                assertTrue(inPool && j != address(0) && j != bk && j != sk, "seated outside the pool");
                for (uint256 r = 0; r < s; r++) {
                    assertTrue(Eravon(inst).seatOf(id, uint64(r)).judge != j, "a pool member seated twice");
                }
            }
        }
    }

    /// s8: the judge is credited P only at Released with the seat seated, at a settlement under an accepted seat
    /// in flight at or before its windowEnd or seated while Claimed at any time, at a ruling and at a refund
    /// while Claimed under a seated seat, and the buyer's fee at the ruling alone.
    function invariant_28_pOnlyWhereTheTableSays() public view {
        assertEq(h.pOutsideTable(), 0, "P or the fee to the judge outside s8's rows");
    }

    /// s5 to s8: every terminal, its Refunded cause and its form 3 are the ones the pre-state names, and no
    /// action closes a case the pre-state leaves open.
    function invariant_29_terminalFromThePreState() public view {
        assertEq(h.terminalWrong(), 0, "a terminal other than the pre-state names");
    }

    /// s8: under a payee whose hook burns gas, every terminal still lands within the transaction gas cap and the
    /// payee keeps its whole credit.
    function invariant_30_gasAllowanceKeepsTerminals() public view {
        assertEq(h.burnerPaid(), 0, "a burning payee was paid or lost its credit");
        assertEq(h.livenessBroken(), 0, "an owed action reverted under the gas cap");
    }

    /// s8: NotForeign exactly where the foreign token's transfer would lower the settlement balance.
    function invariant_31_notForeignExactly() public view {
        assertEq(h.notForeignWrong(), 0, "NotForeign outside the fall of the settlement balance");
    }

    /// s3, s4, s8, s10: every revert of an instance call under the transaction cap is one of the standard's names
    /// (the book's NoPanel at the gate included): no panic, no bare revert, no string, no other bytes (guard 6 and
    /// FeeOverflow make every _finish product exact; every gate fault is named).
    function invariant_32_noPanicNoBareRevert() public view {
        assertEq(h.unnamedRevert(), 0, string(abi.encodePacked("an unnamed revert: act ", vm.toString(h.lastUnnamedAct()), " ", vm.toString(h.lastUnnamed()))));
    }

    /// s4 step 3, cancel: orders in a token without code or answers are Underfunded while live (both consents in
    /// hand), run the branch and cancel, credit nothing, and leave other tokens to sweep; guard 16 refuses every
    /// live order past 2^53 - 1.
    function invariant_33_ghostOrdersAndInt53() public view {
        assertEq(h.ghostWrong(), 0, "an order in an absent token misbehaved");
        assertEq(h.int53Wrong(), 0, "an int past 2^53 - 1 opened or was refused otherwise");
    }

    /// terms/1 s5.1, s5.3, s10: a malformed signature through the instance's signed acts, sent by a party, is refused by
    /// name (BadBuyerSig, NotParty before the instance keeps the keys) and moves nothing.
    function invariant_35_blobsRefusedByName() public view {
        assertEq(h.blobUnnamed(), 0, "a blob refused without its name");
    }

    /// s3: a case priced in a reference unit pins the model's lf, cap, prices and rateAt.
    function invariant_34_pricedAsTheModel() public view {
        assertEq(h.pricedWrong(), 0, "a unit-priced case pinned other values");
    }

    // ---- round 3 ----

    /// s7: redraw and advance on an armed draw answer as the two clocks say, from the pre-state: redraw is
    /// admitted within the deadline or in the block after the arming whatever the clock, draws while the armed
    /// block's hash is served and closes the case as Resigned form 3 once it is not, never re-arming; advance
    /// closes the draw from the second block after the arming by the deadline or by the seed's loss and leaves it
    /// armed otherwise; an armed draw's block never moves while it stays armed.
    function invariant_36_drawClocksFromThePreState() public view {
        assertEq(h.staleRearmed(), 0, "a lost seed left the case open or re-armed");
        assertEq(h.drawBlockMoved(), 0, "an armed draw's block moved while armed");
        assertEq(h.drawOutcomeWrong(), 0, "redraw or advance on an armed draw outside s7's clocks");
    }

    /// s7: a draw in the block after the arming lands whatever the clock.
    function invariant_37_theBlockAfterTheArmingDraws() public view {
        assertEq(h.firstBlockRefused(), 0, "the block after the arming did not draw");
    }

    /// s8: P at a settlement equals P at the outcome the clock gives at that moment, in every phase; where nothing
    /// is due, P goes to the judge exactly where the last seat is accepted and in flight.
    function invariant_38_settlementPIsTheClocks() public view {
        assertEq(h.settleVsClockWrong(), 0, "a settlement's P is not the clock's");
    }

    /// s8, s9, s10: no refund lands during a dispute within T3, its last second included, and the signed form none
    /// during a dispute at all; refundBySig tests the signature first and answers as s9 and s10 say in every phase.
    function invariant_39_noRefundInsideADispute() public view {
        assertEq(h.disputeRefundWrong(), 0, "a refund landed during a dispute within T3, or a signed one during a dispute");
        assertEq(h.refundSigWrong(), 0, "refundBySig answered outside s9 and s10");
    }

    /// s8: every push of the standard's own (a terminal's, cancel's, the expiry branch's) is handed at most PUSH_GAS.
    function invariant_40_everyPushUnderTheAllowance() public view {
        assertEq(
            h.pushOverAllowance(),
            0,
            string(abi.encodePacked("a push handed ", vm.toString(h.lastPushGas()), " gas, act ", vm.toString(h.lastPushAct())))
        );
    }

    /// s3: no feed, unit or answer makes init revert without a name under the transaction cap, and the verdict is
    /// the model's (the case opened at the model's fee, or RateStale).
    function invariant_41_noFeedMakesInitRevertWithoutAName() public view {
        assertEq(h.feedInitUnnamed(), 0, string(abi.encodePacked("init reverted bare: ", vm.toString(h.lastFeedErr()))));
        assertEq(h.feedInitWrong(), 0, string(abi.encodePacked("a pricing verdict off the model: ", vm.toString(h.lastFeedErr()))));
    }

    // ---- round 4 ----

    /// s5, s8, s9: a refund during a dispute at or before T3's end, in either form, at its first second, its last,
    /// the one before its last or any other, always reverts WrongPhase and moves nothing; no refund lands there.
    function invariant_42_aRefundInADisputeWithinT3IsRefused() public view {
        assertEq(h.refundInT3Wrong(), 0, "a refund inside T3 landed, moved something or answered otherwise");
    }

    /// s5, s8: past T3's end the seller's own refund is the plain refund (A, the strays and the fee to the buyer,
    /// P and the LF to the seller, nothing to the judge) and the signed form is still refused throughout the dispute.
    function invariant_43_pastT3TheRefundIsPlain() public view {
        assertEq(h.refundPastT3Wrong(), 0, "past T3 the refund was not the plain one, or the signed form landed");
    }

    /// s8: no Refunded ever credits a judge, except the one row the text names: the seller's refund while Claimed
    /// under a seated seat, which credits P (never the fee, never more), as T2's lapse would.
    function invariant_44_noRefundedCreditsAJudgeOutsideItsRow() public view {
        assertEq(h.refundedJudgeWrong(), 0, "a Refunded credited a judge outside s8's row");
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst,,,,,, uint256 p, uint8 ph,,) = h.caseAt(i);
            if (ph != 4) continue;
            (,,, uint256 judgeCredited,,) = h.caseGhost(i);
            if (judgeCredited == 0) continue;
            Eravon.CaseStatus memory xs_ = Eravon(inst).status();
            uint64 claimedAt = xs_.claimedAt;
            uint64 disputedAt = xs_.disputedAt;
            assertTrue(claimedAt != 0 && disputedAt == 0, "a Refunded credited a judge outside Claimed");
            assertEq(judgeCredited, p, "a Refunded credited the judge other than P");
        }
    }

    /// s3, s4, s8: the length of any answer (a push's 0 to 600,000 bytes with a true, false or garbage word, a
    /// balance's or the token's decimals()' 32 to 4,096, a feed's round 160 to 240,000 and its decimals() 32 to
    /// 240,000) changes no decision and no outcome: the
    /// whole campaign runs under such answers with every model unchanged, and the same action on the same state
    /// under two shapes succeeds or fails alike with the same outcome to the wei; a pull's answer other than no
    /// data or one word 1 is TransferFailed by name.
    function invariant_45_anAnswersLengthChangesNoDecision() public view {
        assertEq(h.lenOutcomeWrong(), 0, "an answer's length changed a decision or an outcome");
        assertEq(h.pullShapeWrong(), 0, "a pull took an answer the text refuses, or refused it otherwise");
    }

    /// s8: every push of the standard's own (a terminal's, cancel's, the expiry branch's) is handed at most
    /// PUSH_GAS, and the caller's gas does not grow with the push's answer: it differs by exactly the answered
    /// pushes' own cost of their answers.
    function invariant_46_pushesUnderTheAllowanceThePushAnswerCostsTheCallerNothing() public view {
        assertEq(h.pushOverAllowance(), 0, "a push of the standard's own was handed more than PUSH_GAS");
        assertEq(h.pushLenGasWrong(), 0, string(abi.encodePacked("a push answer's length cost the caller ", vm.toString(h.lastLenGasOwn()))));
    }

    /// s9, s10: a stale Refund() signature in every phase: BadSellerSig before a case, after a cancel, for a seller
    /// with no key, and for every bad blob (the signature tested first); WrongPhase throughout a dispute and at
    /// every terminal; while Funded or Claimed the seller's own refund to the wei.
    function invariant_47_aStaleRefundSignatureAnswersByItsPhase() public view {
        assertEq(h.staleSigWrong(), 0, "a Refund() signature answered other than its phase and signer say");
    }

    // ---- round 5 ----

    /// s4 to s9: two acts sent for one block, by any two of the seller, the buyer, the judge and a stranger, run in
    /// both orders from one state: every act of either order is the text's from its own pre-state; where the text
    /// makes the order immaterial (an act that moves no money; a clock against the act its window governs; a
    /// resignation against a dispute; a dispute against a settlement; a settlement against a delivery claim; a refund
    /// against an acceptance, a ruling, its own signed form, or the clock while Funded) every side's total is the same,
    /// and where two parties' exits or the clock end a Claimed or Disputed case, or a settlement meets the clock, the
    /// judge's total is the same; after a terminal nothing moves; a dispute ahead of a refund leaves the refund refused
    /// in both forms; three acts reorganised among three slots are the text's act by act in both histories.
    function invariant_48_pairOrdersAreTheTexts() public view {
        assertEq(h.pairStepWrong(), 0, string(abi.encodePacked("an act of a rolled-back order off the models: ", vm.toString(h.lastR5()))));
        assertEq(h.pairOrderWrong(), 0, string(abi.encodePacked("an order changed a total the text keeps: ", vm.toString(h.lastR5()))));
        assertEq(h.pairAfterTerminal(), 0, string(abi.encodePacked("a total moved after a terminal, or a refund landed behind a dispute: ", vm.toString(h.lastR5()))));
    }

    /// s4, s6, s7: a guild case opens at most three seats, within its pool; no seat opens and no draw is armed after an
    /// accepted seat (a resignation after acceptance is not redrawn), nor after a lost seed.
    function invariant_49_guildSeatsAtMostThreeNoRearm() public view {
        assertEq(h.rearmWrong(), 0, string(abi.encodePacked("a draw or a seat after an accepted seat: ", vm.toString(h.lastR5()))));
        assertEq(h.staleRearmed(), 0, "a lost seed re-armed");
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst, bytes32 id, bool g,,,,, uint8 ph,,) = h.caseAt(i);
            if (ph == 0 || !g) continue;
            Eravon.CaseStatus memory xs_ = Eravon(inst).status();
            uint256 seats = xs_.seats;
            bool pending = xs_.pendingRedraw;
            uint256 pool = xs_.panelSize;
            assertLe(seats, 3, "a fourth seat");
            assertLe(seats, pool, "more seats than the pool");
            for (uint256 s = 0; s < seats; s++) {
                if (Eravon(inst).seatOf(id, uint64(s)).accepted != 0) {
                    assertEq(s, seats - 1, "a seat opened after an accepted seat");
                    assertTrue(!pending, "a draw armed after an accepted seat");
                }
            }
        }
    }

    /// s8, s10: every Closed's amounts land where it says (the buyer's side receives exactly buyerPaid, the seller's
    /// side exactly sellerPaid, the judge's seat ledger judgePaid), they sum to what the instance held for the case,
    /// and the instance keeps nothing unrecorded after it.
    function invariant_50_closedLandsWhereItSays() public view {
        assertEq(h.closedLandingWrong(), 0, string(abi.encodePacked("a Closed landed otherwise: ", vm.toString(h.lastR5()))));
        assertEq(h.closedSumWrong(), 0, "a Closed's sums are not what the instance held");
    }

    /// s8: P (and the fee) reaches a judge only at a refund while Claimed under a seated seat, at a settlement under an
    /// accepted seat in flight at or before its windowEnd or seated while Claimed, at Released with the seat seated, and
    /// at Ruled; every credited case's row matches the terminal it closed with.
    function invariant_51_pOnlyInTheFourRows() public view {
        assertEq(h.pCreditWrong(), 0, string(abi.encodePacked("P credited outside the four rows: ", vm.toString(h.lastR5()))));
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (,,,,,,, uint8 ph,,) = h.caseAt(i);
            (,,, uint256 judgeCredited,,) = h.caseGhost(i);
            if (judgeCredited == 0) continue;
            uint8 row = h.creditRowOf(i);
            assertTrue(row >= 1 && row <= 4, "a credit with no row");
            uint8[5] memory term = [0, 4, 6, 5, 7];
            assertEq(ph, term[row], "a credited row and its terminal disagree");
        }
    }

    /// s8: sweeping a foreign token of any hostile shape leaves the settlement ledger as it was and pays buyerPayout by
    /// what left alone; on every case the balance is the ledger plus the strays after every sequence, sweeps of foreign
    /// tokens among them.
    function invariant_52_foreignSweepKeepsTheLedger() public view {
        assertEq(h.foreignLedgerWrong(), 0, string(abi.encodePacked("a foreign sweep moved the ledger: ", vm.toString(h.lastR5()))));
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst,,, uint256 a,,,, uint8 ph,,) = h.caseAt(i);
            if (ph == 0) continue;
            (, uint256 stray,,,,) = h.caseGhost(i);
            (,,, uint256 deposit, uint256 lfHeld, uint256 unpaid) = _status(inst);
            uint256 held = unpaid + stray;
            if (ph <= 3) held += a + deposit + lfHeld;
            assertEq(IERC20Like(_tok()).balanceOf(inst), held, "the ledger does not balance");
        }
    }

    /// s3: under odd decimals() (0 to 36, past 36, short or long answers) and reads that flip within one init, the
    /// verdict and the pinned values are those of an independent model.
    function invariant_53_oddDecimalsAndFlippingReads() public view {
        assertEq(h.oddDecWrong(), 0, string(abi.encodePacked("an odd-decimals verdict off the model: ", vm.toString(h.lastR5()))));
        assertEq(h.flipWrong(), 0, string(abi.encodePacked("a flipping read no combination explains: ", vm.toString(h.lastR5()))));
    }

    /// venue/1 s2.9, s9: EIP-7702 delegated accounts in every role: a delegated judge's accept, rule and resign are
    /// refused NotDirect and change nothing, the case running on its clocks; a delegated party, a payout or the guild
    /// acts as an undelegated one does (every owed act still succeeds).
    function invariant_54_delegatedAccounts() public view {
        assertEq(h.delegatedWrong(), 0, string(abi.encodePacked("a delegated account answered off the text: ", vm.toString(h.lastR5()))));
        assertEq(h.livenessBroken(), 0, "an owed act reverted");
    }

    // ---- round 6: the funding bond and the deposit address ----

    /// s7a, s10, s11: every Pulled names the order's sellerFund as its second topic, the seller's key or an address
    /// bound to the seller before the pull, as the logic's own funds() answers; the opening pull took exactly D from
    /// that address's balance and allowance and nothing from the key; every opened case's fund is the key or bound,
    /// and its seats name the fund third among the parties.
    function invariant_pullFromBoundFundOnly() public view {
        assertEq(h.pullFromUnbound(), 0, string(abi.encodePacked("a Pulled outside the bond: ", vm.toString(h.lastR5()))));
        assertEq(h.fundPullWrong(), 0, string(abi.encodePacked("the opening pull off D at sellerFund: ", vm.toString(h.lastR5()))));
        Eravon lg = Eravon(h.logicAddr());
        for (uint256 i = 0; i < h.caseCount(); i++) {
            if (!h.caseOpened(i)) continue;
            (address inst, bytes32 id,,,,,,,,) = h.caseAt(i);
            (, address sk,,) = h.caseParties(i);
            (address fd,) = h.caseFund(i);
            assertTrue(fd == sk || lg.funds(sk, fd), "an opened case's fund is neither the key nor bound");
            (, uint256 seats,,,,) = _status(inst);
            if (seats != 0) assertEq(Eravon(inst).seatOf(id, 0).parties[2], fd, "parties name sellerFund third");
        }
    }

    /// s8: at every terminal the deposit's returning part (P where it returns, and the seller's LF less what the rung
    /// sends the buyer) lands at sellerFund, summed with whatever else the table sends that address where it is a
    /// payout, and the seller's key receives nothing where it is not the fund.
    function invariant_56_depositReturnsToSellerFund() public view {
        assertEq(h.fundReturnWrong(), 0, string(abi.encodePacked("the deposit's return off sellerFund: ", vm.toString(h.lastR5()))));
    }

    /// s4 step 5, s7a: a live order whose fund is neither the key nor bound is FundUnbound whatever D, one whose bound
    /// fund is short of D is TransferFailed, and neither moves money.
    function invariant_57_theFundGateIsTheModel() public view {
        assertEq(h.fundGateWrong(), 0, string(abi.encodePacked("a fund fault off the model: ", vm.toString(h.lastR5()))));
    }

    /// s7a, s9: a bond is only the fund's own call on the logic (an instance answers NotRegistry), emits FundBound with
    /// both topics, and is never cleared: the logic's funds() is the ghost for every pair the campaign names; an
    /// allowance change moves no open case.
    function invariant_58_bondsAreTheFundsOwnActs() public view {
        assertEq(h.bondWrong(), 0, "a bond off its rule");
        assertEq(h.allowanceMovedOpen(), 0, "an allowance change moved an open case");
        (address[] memory fs, address[] memory ss) = h.fundCandidates();
        Eravon lg = Eravon(h.logicAddr());
        for (uint256 a = 0; a < fs.length; a++) {
            for (uint256 b = 0; b < ss.length; b++) {
                assertEq(lg.funds(ss[b], fs[a]), h.boundG(ss[b], fs[a]), "funds() is the bonds the funds made");
            }
        }
        assertFalse(lg.funds(ss[0], h.logicAddr()), "the logic never binds");
    }

    /// s8 under the five-way draw: for every case the balance covers the ledger, unpaid is the distinct creditors'
    /// credits and the seats' claimable (invariant 12), and no fund holds a credit of a case it does not fund.
    function invariant_59_theLedgerUnderTheFiveWayDraw() public view {
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst,,, uint256 a,,,, uint8 ph,,) = h.caseAt(i);
            if (ph == 0) continue;
            (address bk, address sk, address bp, address sp) = h.caseParties(i);
            (address fd,) = h.caseFund(i);
            (,, , uint256 deposit, uint256 lfHeld, uint256 unpaid) = _status(inst);
            uint256 held = unpaid + (ph <= 3 ? a + deposit + lfHeld : 0);
            assertGe(IERC20Like(_tok()).balanceOf(inst), held, "the balance covers the ledger");
            (address[] memory fs,) = h.fundCandidates();
            for (uint256 k = 0; k < fs.length; k++) {
                address x = fs[k];
                if (x == bp || x == sp || x == fd || x == sk || x == bk) continue;
                assertEq(Eravon(inst).owed(x, _tok()), 0, "a credit at an address the case does not name");
            }
        }
    }

    /// Round 7 (s7a): the pool. Every pool fund's allowance to the logic is its own last approve less each D an opening
    /// pulled since, and its balance plus its credits at every instance is what was minted to it, less each D, plus the
    /// deposit's return the table's model gives it, less what it sent: the allowance bounds every case the pool funds,
    /// the balance the cases open at once, a returned deposit refills the balance and not the allowance; every init of a
    /// pool order answered by these ghosts and no step moved another case of the pool.
    function invariant_60_thePoolIsTheGhost() public view {
        assertEq(h.poolWrong(), 0, string(abi.encodePacked("a pool step off the ghost: ", vm.toString(h.lastR5()))));
        address[] memory fs = h.poolFundList();
        IERC20Like t = IERC20Like(_tok());
        for (uint256 k = 0; k < fs.length; k++) {
            assertEq(MockERC20(_tok()).allowance(fs[k], h.logicAddr()), h.allowG(fs[k]), "a pool fund's allowance off its ghost");
            assertEq(int256(t.balanceOf(fs[k])) + h.owedAll(fs[k]), h.balG(fs[k]), "a pool fund's balance and credits off its ghost");
        }
    }

    /// Round 7 (s4 step 5, s10): init decides the caller rule, the consents, the bond, the balance and the deposit in that
    /// order, the bond whatever D, and a refused init moves nothing.
    function invariant_61_theBondInInitsOrder() public view {
        assertEq(h.fundOrderWrong(), 0, string(abi.encodePacked("init's order around the bond: ", vm.toString(h.lastR5()))));
    }

    /// Round 7 (s8, venue/1 s3): a judge that is the sellerFund of a case it is credited on keeps two ledgers: its seat's
    /// claimable is the table's judge column less what left it, its credit as the fund at most the deposit's return the
    /// model gave it, and each left only to its own address (the seat's registered payout, the fund itself).
    function invariant_62_aJudgeThatFundsKeepsTwoLedgers() public view {
        assertEq(h.dualWrong(), 0, string(abi.encodePacked("a judge's two ledgers mixed: ", vm.toString(h.lastR5()))));
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst,,,,,,, uint8 ph,,) = h.caseAt(i);
            if (ph < 4) continue;
            (address fd,) = h.caseFund(i);
            (,, address credited, uint256 judgeCredited, uint256 judgeOut,) = h.caseGhost(i);
            if (credited == address(0) || credited != fd) continue;
            (, uint256 cl) = Eravon(inst).accountOf(fd, _tok());
            assertEq(cl, judgeCredited - judgeOut, "the seat ledger of a judge that funds the case");
            assertLe(Eravon(inst).owed(fd, _tok()), h.caseToFund(i), "the fund's credit beyond the deposit's return");
        }
    }

    /// s4 to s9: a stranger's no-gain calls between the parties' acts move no side's total (a stray of its own is the
    /// buyer's) and leave the phase and every seat as they were.
    function invariant_55_griefMovesNothing() public view {
        assertEq(h.griefMoved(), 0, string(abi.encodePacked("a stranger's call moved something: ", vm.toString(h.lastR5()))));
    }

    // ---- the succession book, the parties' submissions, the payouts, the outcome views ----

    /// s9, s10: a stranger's submission of a signed form (claimDeliveryBySig, confirmBySig, disputeBySig, refundBySig),
    /// of settle in every form, of sweep, of init under buyerMayOpen and of the expiry branch, and of cancel's
    /// signed form is refused NotParty before every other fault (NotSeller for init where buyerMayOpen is false; before
    /// the instance keeps the parties' keys every sender is NotParty); its registrations under a party's name are
    /// NotCurrent; every other call of the grief walk answers the text's refusal.
    function invariant_63_strangersSubmitNothing() public view {
        assertEq(h.strangerAdmitted(), 0, string(abi.encodePacked("a stranger's submission was not refused as the text says: ", vm.toString(h.lastR5()))));
    }

    /// s9, venue/1 s1, galeed.roster/1 s4: after a record moves on the succession book, the former key is refused at
    /// every gate (a party's direct acts, its signed forms' submission, its signatures sent by the new key, settle,
    /// sweep, consent and revoke under the record's name, cancel; a judge's accept and resign, setPayout and withdraw;
    /// a guild's engage on the book and its draw), the new key acts at every gate (every owed act of the campaign
    /// succeeds under it, invariant 23), the book files the new key's engage under the record's root, and every
    /// seat's parties are the addresses the order recorded.
    function invariant_64_aFormerKeyActsForNothing() public view {
        assertEq(h.oldKeyAdmitted(), 0, string(abi.encodePacked("a former key acted, or a new key was refused: ", vm.toString(h.lastR5()))));
        address sb = h.successionAddr();
        assertEq(Eravon(h.logicAddr()).successionBook(), sb, "the logic reads the succession book at the code's constant");
        assertEq(EravonPanelBook(h.panelBookAddr()).successionBook(), sb, "the panel book reads the same succession book");
        address[] memory old = h.oldKeyList();
        for (uint256 k = 0; k < old.length; k++) {
            address r = h.rootOfOld(old[k]);
            assertTrue(IBookLike(sb).current(r) != old[k], "a former key is the current key of its record");
            assertEq(IBookLike(sb).root(old[k]), r, "a former key left its record");
        }
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst, bytes32 id,,,,,, uint8 ph,,) = h.caseAt(i);
            if (ph == 0) continue;
            (, uint256 seats,,,,) = _status(inst);
            if (seats == 0) continue;
            (address bk, address sk, address bp, address sp) = h.caseParties(i);
            (address fd,) = h.caseFund(i);
            address[] memory p = Eravon(inst).seatOf(id, 0).parties;
            assertTrue(p[0] == bk && p[1] == sk && p[2] == fd && p[3] == bp && p[4] == sp, "parties as the order recorded them");
        }
    }

    /// s8, venue/1 s3: no payout is registered at the opening; setPayout from the key's current key registers once a
    /// nonzero address (a stranger NotCurrent, a second PayoutSet, zero ZeroPayout, the logic NotInstance); withdraw
    /// registers the payout it names where none is, refuses another (PayoutSet) and none at all (NoPayout); a seat's
    /// registered payout is the one the campaign registered for it.
    function invariant_65_payoutsRegisterOnTheInstance() public view {
        assertEq(h.payoutAtInit(), 0, "a payout registered at the opening");
        assertEq(h.payoutRuleWrong(), 0, string(abi.encodePacked("setPayout or withdraw off venue/1 s3: ", vm.toString(h.lastR5()))));
        address[] memory judges = h.allJudges();
        for (uint256 i = 0; i < h.caseCount(); i++) {
            (address inst,,,,,,,,,) = h.caseAt(i);
            for (uint256 j = 0; j < judges.length; j++) {
                (address p,) = Eravon(inst).accountOf(judges[j], _tok());
                assertTrue(p == address(0) || p == h.payoutOfJudge(judges[j]), "a judge's payout other than its own registration");
            }
        }
    }

    /// s5, s6, s8: every Lapsed reports the share lapseDefault names (0, 5000 or 10000 bps) and its Closed pays the
    /// buyer that share of A, the half share included; the seat's standbyCredited and rulingCredited carry P and the
    /// buyer's fee where the terminal credited them to the last seat, and nothing anywhere else.
    function invariant_66_lapseSharesAndSeatCredits() public view {
        assertEq(h.closedModelWrong(), 0, "a Lapsed or another Closed off the table");
        assertEq(h.seatCreditWrong(), 0, string(abi.encodePacked("a seat's credited fields off the terminal: ", vm.toString(h.lastR5()))));
    }

    /// s8, s11 item 18, venue/1 s2.12: outcomeOf before a ruling and advanceOf before an advance answer what the act
    /// then does on the state they read (the seat's state, the judge's credit, the buyer's share of A, the part of the
    /// seller's LF sent to the buyer), both revert UnknownSeat for every pair seatOf refuses, and a record made in a
    /// block reaches no freeze in that block.
    function invariant_67_outcomeViewsBindTheActs() public view {
        assertEq(h.viewWrong(), 0, string(abi.encodePacked("an outcome view off its act: ", vm.toString(h.lastR5()))));
        assertEq(h.panelFreezeWrong(), 0, "a record frozen in its own block");
    }

    // ---- FUZZ r24-1: the guild draw under a router (s7, venue/1 s4) ----

    /// s7, venue/1 s4: under a frozen record that names a router, every draw seats members[index] where the router
    /// answers exactly one word naming a member inside the panel that no seat holds (compared by address, a party's
    /// key and a repeated entry included), the router handed the view field by field and exactly routeGas() gas at
    /// every gas limit at which the draw lands; and a code hash that differs, a revert (with a reason, empty, long), an
    /// answer of 0, 31, 33 or 64 bytes, the gas run out, a write, the invalid opcode, an index outside the panel, an
    /// index naming a seated member or a repeat of one, end the case as Resigned with form 3, A to buyerPayout and D
    /// to sellerFund; a limit below the lowest that lands reverts with nothing moved (RouterGas or out of gas); the
    /// arming block refuses (DrawPending), a stranger's draw lands as a party's does and a former key of the guild
    /// engages nothing (the book's NotCurrent), the lost seed ends the case whatever the router would answer, and a
    /// record made in init's own
    /// block never reaches the case.
    function invariant_68_theRouterDrawIsTheModel() public view {
        assertEq(h.routerWrong(), 0, string(abi.encodePacked("a router draw off s7: ", vm.toString(h.lastRouter()))));
    }

    /// s7, s8 on the router cases: a live case holds A and D; a closed one holds exactly its unpaid credits (P on the
    /// seat ledger after a release the judge has not withdrawn, nothing otherwise), no draw armed and no deposit; the
    /// router token is conserved across the instances and every address the cases pay.
    function invariant_69_routerCasesKeepTheirLedger() public view {
        address rt = h.rtokAddr();
        if (rt == address(0)) return;
        uint256 sum;
        for (uint256 k = 0; k < h.rcaseCount(); k++) {
            (address inst, uint256 a, uint256 d,,, bool closed,, uint256 judgeOwed) = h.rcaseAt(k);
            Eravon.CaseStatus memory st = Eravon(inst).status();
            uint256 bal = IERC20Like(rt).balanceOf(inst);
            sum += bal;
            if (!closed) {
                assertEq(uint8(st.phase), 1, "a live router case is Funded");
                assertEq(bal, a + d + st.unpaid, "a live router case holds A and D");
                assertEq(st.deposit, d, "the deposit held is the one pulled");
            } else {
                assertTrue(uint8(st.phase) >= 4, "a closed router case is in a terminal");
                assertFalse(st.pendingRedraw, "a closed router case arms no draw");
                assertEq(st.deposit, 0, "a closed router case holds no deposit");
                assertEq(bal, st.unpaid, "a closed router case holds its credits alone");
                assertEq(st.unpaid, judgeOwed, "the credits are the model's");
            }
        }
        address[] memory hs = h.rHolders();
        for (uint256 x = 0; x < hs.length; x++) {
            sum += IERC20Like(rt).balanceOf(hs[x]);
        }
        assertEq(sum, h.rMinted(), "the router token is conserved");
    }

    /// s7: a router that fails ends its case for good: Resigned, the seats it had before the failed draw, no draw
    /// armed, never re-armed.
    function invariant_70_aFailedRouterEndsItsCase() public view {
        for (uint256 k = 0; k < h.rcaseCount(); k++) {
            (address inst,,, bool failed, uint256 seatsAtFail,,,) = h.rcaseAt(k);
            if (!failed) continue;
            Eravon.CaseStatus memory st = Eravon(inst).status();
            assertEq(uint8(st.phase), 8, "a failed router's case is Resigned");
            assertEq(st.seats, seatsAtFail, "no seat after a failed router");
            assertFalse(st.pendingRedraw, "no draw re-armed after a failed router");
        }
        address[] memory old = h.rguildOldKeys();
        for (uint256 x = 0; x < old.length; x++) {
            assertTrue(IBookLike(h.successionAddr()).current(old[x]) != old[x], "a former key of the router guild acts for it");
        }
    }
}

interface IBookLike {
    function current(address a) external view returns (address);
    function root(address a) external view returns (address);
}

interface IERC20Like {
    function balanceOf(address) external view returns (uint256);
}

/// A deterministic walk through the handler: every path the campaign is
/// meant to take is taken at least once, so that no action is dead weight.
/// Gas metering stays on, so that the burning hooks and feeds burn; it is
/// reset before every call, so that the walk's total never meets the test's
/// gas limit.
contract EravonHandlerReach is Test {
    uint256 internal constant N_SEL = 60;
    uint256 internal constant N_PATHS = 408;
    uint256 internal constant N_ROUNDS = 32;

    function _sel(EravonHandler h) internal pure returns (bytes4[60] memory) {
        return [
            h.newOrder.selector,
            h.newSibling.selector,
            h.doInit.selector,
            h.doSecondInit.selector,
            h.doRace.selector,
            h.doCancel.selector,
            h.doRevoke.selector,
            h.doReengage.selector,
            h.doAccept.selector,
            h.doRule.selector,
            h.doResign.selector,
            h.doAdvance.selector,
            h.doRedraw.selector,
            h.doCommit.selector,
            h.doClaim.selector,
            h.doConfirm.selector,
            h.doDispute.selector,
            h.doRefund.selector,
            h.doSettle.selector,
            h.doStray.selector,
            h.doSweep.selector,
            h.doSetPayout.selector,
            h.doWithdraw.selector,
            h.doBlock.selector,
            h.impostorPull.selector,
            h.warp.selector,
            h.doBurn.selector,
            h.doDrawEdge.selector,
            h.doAdvanceEdge.selector,
            h.doRefundEdge.selector,
            h.doSettleEdge.selector,
            h.newGhostOrder.selector,
            h.doGhost.selector,
            h.doSweepSecond.selector,
            h.doBadBlob.selector,
            h.doDrawClocks.selector,
            h.doAdvanceDraw.selector,
            h.doSettleVsClock.selector,
            h.doRefundAtT3.selector,
            h.doRefundSig.selector,
            h.doFeedInit.selector,
            h.doShape.selector,
            h.doLenProbe.selector,
            h.doRefundInDispute.selector,
            h.doStaleSig.selector,
            h.doPair.selector,
            h.doReorg.selector,
            h.doThreeSeats.selector,
            h.doForeignHostile.selector,
            h.doOddDecimals.selector,
            h.doFlipFeed.selector,
            h.doDelegate.selector,
            h.doGrief.selector,
            h.doInitOrder.selector,
            h.doFund.selector,
            h.doPool.selector,
            h.doFundOrder.selector,
            h.doMoveKey.selector,
            h.doFormerKey.selector,
            h.doRouter.selector
        ];
    }

    /// Every path the walk must reach, the terminals among them, in one list.
    function _want() internal pure returns (string[408] memory) {
        return [
            "init.named", "init.guild", "init.guildWithParties", "init.companySeller", "init.companyBuyer",
            "init.sellerByCall", "init.sellerByRegistry", "init.buyerByCall", "init.buyerByRegistry", "init.strangerRefused",
            "init.expiryBranch", "init.expiryBranchPushFailedCreditKept", "order.sibling", "secondInit.refused", "race.cancelFirst",
            "race.initFirst", "cancel.key", "cancel.company", "revoke.beforeInit", "revoke.afterInit",
            "guild.reengaged", "accept", "rule", "resign.decline", "resign.funded",
            "resign.claimed", "resign.disputed", "redraw", "redraw.lateRefused", "redraw.wrongPanelRefused",
            "redraw.sameBlockRefused", "redraw.stale", "draw.armedAtInit", "draw.armedAfterAnUnacceptedSeat", "advance.drawDeadline",
            "terminal.withoutASeat", "claim.key", "claim.company", "confirm.key", "confirm.company",
            "dispute.fromKey", "dispute.sentAhead", "dispute.inExcess", "dispute.split", "dispute.company",
            "refund.key", "refund.company", "refund.refusedInDisputeT3", "refund.sigRefusedInDispute", "settle.byCall",
            "settle.byRegistry", "settle.pBackFromASeatThatDidNotStand", "stray.beforeInit", "stray.open", "stray.afterTerminal",
            "stray.foreign", "sweep.foreign", "sweep.pushedACredit", "withdraw", "payout.blocked",
            "terminal.pushFailedCreditKept", "impostor.refused", "judge.credited", "terminal.withStray", "phase.3.to.7",
            "init.deadOrder", "advance.drawDeadlineFromAnUnopenedSeat", "redraw.byKeeper", "init.unitPriced", "init.amountAtTheBound",
            "init.int53AtTheBound", "init.int53PastTheBoundRefused", "payout.burns", "terminal.underABurningHook", "redraw.edge.drawnBefore256",
            "redraw.edge.drawnAt256", "redraw.edge.closedPast256", "advance.edge.atTheEnd", "advance.edge.pastTheEnd", "advance.edge.ordinalNeverOpened",
            "refund.edge.refusedAtT3End", "refund.edge.pastT3", "refund.edge.undrawn", "settle.edge.undrawn", "settle.edge.in.unaccepted",
            "settle.edge.past.unaccepted", "settle.edge.in.accepted", "settle.edge.past.accepted", "zeroSeats.closedByRefund", "zeroSeats.closedBySettle",
            "zeroSeats.closedByForm3", "ghost.noCode", "ghost.mute", "ghost.expiryBranch", "ghost.cancel",
            "ghost.sweep", "ghost.foreignSwept", "sweep.secondEntryRefused", "sweep.secondEntryNothingMoved", "blob.refusedOnAKeyCase",
            "redraw.firstBlockPastDeadline", "phase.0.to.4", "phase.1.to.4", "phase.2.to.4", "phase.3.to.4",
            "phase.2.to.5", "phase.1.to.6", "phase.2.to.6", "phase.3.to.6", "phase.1.to.8",
            "phase.2.to.9", "phase.3.to.9", "phase.1.to.2", "push.metered", "drawClocks.refusedPastTheDeadline",
            "drawClocks.seedLost", "drawClocks.lastServedBlock", "drawClocks.firstBlockPastTheDeadline", "drawClocks.drawn", "advanceDraw",
            "advance.drawSeedLostClosed", "advance.drawDeadlineClosed", "advance.firstBlockLeftArmed", "redraw.seedLostClosed", "redraw.firstBlockPastTheDeadline",
            "redraw.lastServedBlock", "settleVsClock.clockDuePaysTheJudge", "settleVsClock.clockDuePaysTheSeller", "settleVsClock.clockNotDue", "refundAtT3.refusedWithinT3",
            "refundAtT3.plainPastT3", "refundSig.landsOutsideADispute", "refundSig.refusedInDispute", "refundSig.refusedInDisputePastT3", "refundSig.junkRefusedInDispute",
            "refundSig.refusedBeforeACase", "refundSig.refusedOtherwise", "feed.kind0.opened", "feed.kind1.rateStale", "feed.kind2.rateStale",
            "feed.kind3.opened", "feed.kind4.rateStale", "feed.kind5.opened", "feed.kind6.rateStale", "feed.kind7.rateStale",
            "feed.kind8.rateStale", "feed.kind9.rateStale", "ghost.liveInitUnderfunded", "shape.set", "shape.plain",
            "lenProbe.sameOutcome", "lenProbe.sameFault", "lenProbe.gasExact", "lenProbe.balanceLen", "lenProbe.pullRefused",
            "refundT3.refusedBoth", "refundT3.refusedAtTheLastSecond", "refundT3.pastPlain", "staleSig.lands", "staleSig.landedForReal",
            "staleSig.noKey", "staleSig.company", "staleSig.bad", "staleSig.inDispute", "staleSig.terminal",
            "pair.run", "pair.fullRule", "pair.judgeRule", "pair.bothLanded", "pair.afterTerminal",
            "pair.disputeAheadOfRefund", "reorg.run", "reorg.drewAfterTheMove", "three.opened", "three.thirdSeat",
            "three.thirdSeatOfFive", "init.guildOfFive", "three.drawn", "three.declined", "three.expiredOffer",
            "three.acceptedFirst", "three.acceptedAfterAnEndedSeat", "three.declinedStaleOffer", "three.resignAfterAccept", "three.resignInDispute",
            "three.deadlinePassed", "three.seedLost", "landing.checked", "landing.split", "pRow.refundWhileClaimed",
            "pRow.settlement", "pRow.release", "pRow.ruling", "foreign.hostile0.fixed", "foreign.hostile1.fixed",
            "foreign.hostile2.fixed", "foreign.hostile3.fixed", "foreign.hostile4.fixed", "foreign.hostile5.fixed", "foreign.hostile6.fixed",
            "foreign.hostile7.fixed", "foreign.hostile.unfixed", "odd.opened", "odd.rateStale", "odd.feeOverflow",
            "odd.overCap", "flip.mode0.opened", "flip.mode1.opened", "flip.mode2.opened", "flip.mode3.opened",
            "delegate.judgeNotDirect", "delegate.partyRunsCode", "delegate.partyDesignator", "delegate.otherDesignator", "grief.nothingMoved",
            "initOrder.0.1.live", "initOrder.0.1.dead", "initOrder.0.2.live", "initOrder.0.3.live", "initOrder.1.2.live",
            "initOrder.1.3.live", "initOrder.2.3.live", "fund.bind", "fund.bindByCode", "fund.bindAgain",
            "fund.oneFunderTwoSellers", "fund.bindThroughInstanceRefused", "fund.openedKey", "fund.openedFunder", "fund.openedContractFunder",
            "fund.openedSellerPayout", "fund.openedBuyerPayout", "fund.zeroDepositOpenedKey", "fund.zeroDepositOpenedBound", "fund.unboundRefused",
            "fund.zeroDepositUnboundRefused", "fund.unboundFunderRefused", "fund.unboundSellerPayoutRefused", "fund.unboundBuyerPayoutRefused", "fund.strangerRefused",
            "fund.logicOrVerifierRefused", "fund.anInstanceRefused", "fund.shortRefused", "fund.allowanceCleared", "fund.allowanceChanged",
            "fund.allowanceChangedWhileOpen", "fund.raceBondFirst", "fund.raceInitFirst", "fund.returnedAtSellerPayout", "fund.returnedAtBuyerPayout",
            "fund.returnedAtAFunder", "fund.zeroFundGuard4", "pool.fundKey", "pool.fundJudge", "pool.fundMember",
            "pool.balanceGated", "pool.opened", "pool.unboundRefused", "pool.allowanceShortRefused", "pool.balanceShortRefused",
            "pool.openedAfterTheAllowanceRose", "pool.openedAfterTheBalanceRose", "pool.judgeSeatedOnItsFund", "pool.memberSeatedOnItsFund", "pool.allowanceCleared",
            "pool.allowanceSet", "pool.allowanceSetWhileCasesOpen", "pool.balanceSent", "pool.balanceToppedUp", "pool.fundBlocked",
            "pool.sweptTheFundsCredit", "pool.lateBond", "pool.closed", "pool.closedByItsFundingJudge", "pool.returnRefilled",
            "pool.severalOpenAtOnce", "dual.terminal", "dual.seatLedgerOut", "dual.creditOut", "fundOrder.callerRuleFirst",
            "fundOrder.buyerConsentBeforeBond", "fundOrder.sellerConsentBeforeBond", "fundOrder.bondBeforeBalance", "fundOrder.balanceBeforeDeposit", "fundOrder.shortPull",
            "fundOrder.zeroDepositNoExemption", "fundOrder.opens", "fund.openedPool", "ruled.oddHalf", "stranger.signedFormRefused",
            "stranger.settleRefused", "stranger.sweepRefused", "stranger.cancelSigRefused", "stranger.branchRefused",
            "init.strangerRefusedTheBranch", "init.strangerRefusedNotSeller", "init.byBuyer", "init.bySellerAfterAStranger", "cancel.signedSentByAParty",
            "confirm.signedSentByTheSeller", "consent.strangerRefused", "redraw.bySeller", "redraw.byBuyer", "redraw.byGuild", "redraw.byStranger", "grief.redrawNotPending", "grief.redrawPanelMismatch",
            "sweep.byASeatedJudge", "sweep.refusedBeforeTheKeysAreKept", "sweep.secondEntryBeforeTheKeys", "ghost.sweepRefusedBeforeTheKeysAreKept", "blob.refusedBeforeTheKeys",
            "staleSig.afterTheBranch", "staleSig.afterACancel", "move.seller", "move.buyer", "move.judge",
            "move.guild", "move.actedByTheNewKey", "init.underAMovedKey", "dispute.feeFromTheRecordedKeyAfterAMove", "move.formerKeyRegistersNothing",
            "move.formerKeyRevokesNothing", "move.formerKeyRefused", "move.formerKeySettlesNothing", "move.formerKeySweepsNothing", "move.formerSignatureRefused",
            "move.formerKeyCancelsNothing", "move.formerGuildKeyRefused", "move.formerJudgeKeyRegistersNothing", "move.formerJudgeKeyWithdrawsNothing",
            "move.formerJudgeKeyRefused", "move.formerJudgeKeySweepsNothing", "payout.noneAtInit", "payout.setAfterInit", "payout.set",
            "payout.strangerRefused", "payout.onTheLogicRefused", "payout.zeroRefused", "payout.secondRefused", "withdraw.strangerRefused",
            "withdraw.onTheLogicRefused", "withdraw.noPayoutRefused", "withdraw.otherPayoutRefused", "withdraw.registersThePayout", "lapsed.none",
            "lapsed.half", "lapsed.all", "seat.creditedFields", "view.unknownSeatRefused", "view.outcomeBindsTheRuling",
            "view.advanceBindsTheLapse", "view.advanceBindsTheClose", "guild.recordNotFrozenInItsOwnBlock",
            "router.opened", "router.sameBlockRecordNotFrozen", "router.opened.autoAccept", "router.opened.partyDuplicateZero",
            "router.opened.boundFund", "router.drawPendingRefused", "router.deadlineClosed", "router.byStranger",
            "router.formerGuildKeyRefused", "router.guildKeyMoved", "router.byGuild", "router.byMovedGuildKey",
            "router.byParty", "router.seat.first", "router.seat.hash", "router.seat.party",
            "router.seat.fixed", "router.seat.duplicate", "router.fail.codeReplaced", "router.fail.codeRemoved",
            "router.fail.revert", "router.fail.emptyRevert", "router.fail.twoWords", "router.fail.short",
            "router.fail.noData", "router.fail.burnsItsGas", "router.fail.write", "router.fail.invalid",
            "router.fail.long", "router.fail.bigRevert", "router.fail.justOutside", "router.fail.outside",
            "router.fail.seated", "router.fail.duplicateOfASeated", "router.fail.zeroMember", "router.secondDrawFails",
            "router.thirdDrawFails", "router.secondDrawSeats", "router.thirdDrawSeats", "router.gas.landsAtTheLimit",
            "router.gas.refusedBelow", "router.gas.routerGasNamed", "router.gas.hashTestedFirst", "router.seedLost.redraw",
            "router.seedLost.advance", "router.accepted", "router.declinedRearmed", "router.expiredRearmed",
            "router.thirdSeatEnds", "router.refundedOffered", "router.t1", "router.refundedAccepted",
            "router.released", "router.judgeWithdrew"
        ];
    }

    function _report(EravonHandler h, uint256 round) internal {
        emit log_named_uint("round with violations", round);
        emit log_named_uint("liveness", h.livenessBroken());
        emit log_named_bytes("liveness error", h.lastLivenessErr());
        emit log_named_uint("liveness act", h.lastLivenessAct());
        emit log_named_uint("unnamed", h.unnamedRevert());
        emit log_named_bytes("unnamed error", h.lastUnnamed());
        emit log_named_uint("unnamed act", h.lastUnnamedAct());
        emit log_named_uint("terminalWrong", h.terminalWrong());
        emit log_named_uint("pOutsideTable", h.pOutsideTable());
        emit log_named_uint("zeroSeatWrong", h.zeroSeatWrong());
        emit log_named_uint("notForeignWrong", h.notForeignWrong());
        emit log_named_uint("ghostWrong", h.ghostWrong());
        emit log_named_uint("burnerPaid", h.burnerPaid());
        emit log_named_uint("pricedWrong", h.pricedWrong());
        emit log_named_uint("int53Wrong", h.int53Wrong());
        emit log_named_uint("seatedNotAfterDraw", h.seatedNotAfterDraw());
        emit log_named_uint("drawWrong", h.drawWrong());
        emit log_named_uint("closedModelWrong", h.closedModelWrong());
        emit log_named_uint("seatTableBroken", h.seatTableBroken());
        emit log_named_uint("refusedRedrawPassed", h.refusedRedrawPassed());
        emit log_named_uint("drawOutcomeWrong", h.drawOutcomeWrong());
        emit log_named_uint("staleRearmed", h.staleRearmed());
        emit log_named_uint("drawBlockMoved", h.drawBlockMoved());
        emit log_named_uint("firstBlockRefused", h.firstBlockRefused());
        emit log_named_uint("settleVsClockWrong", h.settleVsClockWrong());
        emit log_named_uint("disputeRefundWrong", h.disputeRefundWrong());
        emit log_named_uint("refundSigWrong", h.refundSigWrong());
        emit log_named_uint("pushOverAllowance", h.pushOverAllowance());
        emit log_named_uint("lastPushGas", h.lastPushGas());
        emit log_named_uint("feedInitUnnamed", h.feedInitUnnamed());
        emit log_named_uint("feedInitWrong", h.feedInitWrong());
        emit log_named_bytes("lastFeedErr", h.lastFeedErr());
        emit log_named_uint("liveness act", h.lastLivenessAct());
        emit log_named_uint("refundInT3Wrong", h.refundInT3Wrong());
        emit log_named_uint("refundPastT3Wrong", h.refundPastT3Wrong());
        emit log_named_uint("refundedJudgeWrong", h.refundedJudgeWrong());
        emit log_named_uint("lenOutcomeWrong", h.lenOutcomeWrong());
        emit log_named_uint("pushLenGasWrong", h.pushLenGasWrong());
        emit log_named_int("lastLenGasOwn", h.lastLenGasOwn());
        emit log_named_uint("staleSigWrong", h.staleSigWrong());
        emit log_named_uint("pullShapeWrong", h.pullShapeWrong());
        emit log_named_uint("blobUnnamed", h.blobUnnamed());
        emit log_named_uint("seatedWrong", h.seatedWrong());
        emit log_named_uint("eventWrong", h.eventWrong());
        emit log_named_uint("seatMachineBroken", h.seatMachineBroken());
        emit log_named_uint("judgeCreditWrong", h.judgeCreditWrong());
        emit log_named_uint("closedSumWrong", h.closedSumWrong());
        emit log_named_uint("pairStepWrong", h.pairStepWrong());
        emit log_named_uint("pairOrderWrong", h.pairOrderWrong());
        emit log_named_uint("pairAfterTerminal", h.pairAfterTerminal());
        emit log_named_uint("rearmWrong", h.rearmWrong());
        emit log_named_uint("closedLandingWrong", h.closedLandingWrong());
        emit log_named_uint("pCreditWrong", h.pCreditWrong());
        emit log_named_uint("foreignLedgerWrong", h.foreignLedgerWrong());
        emit log_named_uint("oddDecWrong", h.oddDecWrong());
        emit log_named_uint("flipWrong", h.flipWrong());
        emit log_named_uint("delegatedWrong", h.delegatedWrong());
        emit log_named_uint("griefMoved", h.griefMoved());
        emit log_named_uint("pullFromUnbound", h.pullFromUnbound());
        emit log_named_uint("fundGateWrong", h.fundGateWrong());
        emit log_named_uint("fundPullWrong", h.fundPullWrong());
        emit log_named_uint("fundReturnWrong", h.fundReturnWrong());
        emit log_named_uint("bondWrong", h.bondWrong());
        emit log_named_uint("allowanceMovedOpen", h.allowanceMovedOpen());
        emit log_named_uint("poolWrong", h.poolWrong());
        emit log_named_uint("dualWrong", h.dualWrong());
        emit log_named_uint("fundOrderWrong", h.fundOrderWrong());
        emit log_named_uint("strangerAdmitted", h.strangerAdmitted());
        emit log_named_uint("oldKeyAdmitted", h.oldKeyAdmitted());
        emit log_named_uint("payoutAtInit", h.payoutAtInit());
        emit log_named_uint("payoutRuleWrong", h.payoutRuleWrong());
        emit log_named_uint("seatCreditWrong", h.seatCreditWrong());
        emit log_named_uint("viewWrong", h.viewWrong());
        emit log_named_uint("panelFreezeWrong", h.panelFreezeWrong());
        emit log_named_bytes("lastR5", h.lastR5());
        emit log_named_uint("routerWrong", h.routerWrong());
        emit log_named_bytes("lastRouter", h.lastRouter());
        emit log_named_uint("maxGuildSeats", h.maxGuildSeats());
        emit log_named_uint("pairsCovered", h.pairsCovered());
        if (h.livenessBroken() != 0) {
            (uint8 tr, uint8 pnl, uint8 gp, uint8 ph, bool three) = h.debugCase(h.lastLivenessCase());
            emit log_named_uint("case", h.lastLivenessCase());
            emit log_named_uint("track", tr);
            emit log_named_uint("panel", pnl);
            emit log_named_uint("guildPanel", gp);
            emit log_named_uint("phase", ph);
            emit log_named_uint("forced", three ? 1 : 0);
        }
    }

    function _allReached(uint256[408] memory got) internal pure returns (bool) {
        for (uint256 p = 0; p < N_PATHS; p++) if (got[p] == 0) return false;
        return true;
    }

    function test_everyPathReachable() public {
        string[408] memory want = _want();
        uint256[408] memory got;
        uint256 okCalls;
        uint256 calls;
        uint256 lastPushes;
        uint256 maxCopy;
        uint256[5] memory rstat;
        uint256 rounds;
        for (uint256 round = 0; round < N_ROUNDS || (round < 4 * N_ROUNDS && !_allReached(got)); round++) {
            rounds = round + 1;
            uint256 snapId = vm.snapshotState();
            EravonHandler h = new EravonHandler();
            bytes4[60] memory sel = _sel(h);
            // round 7: the walk's seed from the environment (REACH_SEED), 0xE5A7 where none is set
            uint256 r = uint256(keccak256(abi.encode(vm.envOr("REACH_SEED", uint256(0xE5A7)), round)));
            for (uint256 k = 0; k < 3000; k++) {
                r = uint256(keccak256(abi.encode(r, k)));
                vm.resetGasMetering();
                uint256 a = r % N_SEL;
                if (k % 9 == 0) a = (r >> 8) % 2;
                (bool ok,) = address(h).call(
                    abi.encodeWithSelector(
                        sel[a],
                        r >> 16,
                        uint256(keccak256(abi.encode(r, 1))),
                        uint256(keccak256(abi.encode(r, 2))) % 1e27,
                        uint256(keccak256(abi.encode(r, 3)))
                    )
                );
                calls++;
                if (ok) okCalls++;
            }
            for (uint256 p = 0; p < N_PATHS; p++) {
                got[p] += h.reached(keccak256(bytes(want[p])));
            }
            if (h.violations() != 0) _report(h, round);
            assertEq(h.violations(), 0, "a violation on the walk");
            lastPushes = h.pushesMetered();
            if (h.balanceCopyMax() > maxCopy) maxCopy = h.balanceCopyMax();
            rstat[0] += h.routerDraws();
            rstat[1] += h.routerSeated();
            rstat[2] += h.routerFailed();
            rstat[3] += h.routerGasProbes();
            rstat[4] += h.routerZeroMember();
            vm.revertToState(snapId);
        }
        emit log_named_uint("rounds (the walk adds rounds past N_ROUNDS while a path is unreached)", rounds);
        emit log_named_uint("calls", calls);
        emit log_named_uint("calls that did not revert", okCalls);
        emit log_named_uint("pushes metered (last round)", lastPushes);
        emit log_named_uint("most a balance answer's length cost a caller beyond its pushes (any round)", maxCopy);
        emit log_named_uint("router draws", rstat[0]);
        emit log_named_uint("router draws that seated", rstat[1]);
        emit log_named_uint("router draws whose router failed", rstat[2]);
        emit log_named_uint("router gas probes", rstat[3]);
        emit log_named_uint("router draws naming the zero member", rstat[4]);
        for (uint256 p = 0; p < N_PATHS; p++) {
            emit log_named_uint(want[p], got[p]);
        }
        for (uint256 p = 0; p < N_PATHS; p++) {
            assertGt(got[p], 0, want[p]);
        }
    }
}

/// Round 5: every unordered pair of the sixteen acts, in both orders, on fresh cases at eight points of the machine
/// (a named offer unaccepted, accepted in T1, Claimed under a seated seat, Claimed after the seat resigned, Disputed,
/// a guild draw armed, a guild offer unaccepted, a terminal) and three moments (now, the clock's last second, one
/// second past it): every act of either order is the text's from its own pre-state, the totals agree where the
/// text makes the order immaterial, nothing moves after a terminal, and a dispute ahead of a refund leaves it
/// refused. One handler per point and moment.
contract EravonPairMatrix is Test {
    function _name(uint256 c) internal pure returns (string memory) {
        string[16] memory n = ["confirm", "dispute", "refund", "refundBySig", "settle", "claim", "accept", "rule", "resign", "advance", "redraw",
            "commit", "sweep", "setPayout", "withdraw", "stray"];
        return n[c];
    }

    /// The pairs whose two orders end apart, for the pre-states this handler ran (the report's table).
    function _deps(EravonHandler h, uint8 target, uint256 moment) internal {
        for (uint256 key = 16; key < 64; key++) {
            uint256 d = h.orderDep(key);
            if (d == 0) continue;
            uint256 dj = h.orderDepJudge(key);
            for (uint256 x = 0; x < 16; x++) {
                for (uint256 y = x + 1; y < 16; y++) {
                    if ((d >> (x * 16 + y)) & 1 == 0) continue;
                    emit log(string(abi.encodePacked("orderDep target ", vm.toString(target), " moment ", vm.toString(moment), " pre ", vm.toString(key / 16), "/", vm.toString((key % 16) / 2), "/", vm.toString(key % 2), ": ", _name(x), " x ", _name(y), (dj >> (x * 16 + y)) & 1 == 1 ? " (judge's total differs)" : "")));
                }
            }
        }
    }

    function test_everyPairInBothOrders() public {
        uint256[3] memory moments = [uint256(0), 2, 3];
        uint256 covered;
        uint256 landed;
        uint256 runs;
        uint256 full;
        uint256 judgeR;
        uint256 dar;
        for (uint8 target = 0; target < 8; target++) {
            for (uint256 m = 0; m < 3; m++) {
                uint256 snapId = vm.snapshotState();
                EravonHandler h = new EravonHandler();
                for (uint8 x = 0; x < 16; x++) {
                    for (uint8 y = x + 1; y < 16; y++) {
                        vm.resetGasMetering();
                        h.pairMatrix(target, x, y, moments[m], uint256(keccak256(abi.encode(target, m, x, y))));
                    }
                }
                if (h.violations() != 0) {
                    emit log_named_uint("target", target);
                    emit log_named_uint("moment", moments[m]);
                    emit log_named_uint("pairStepWrong", h.pairStepWrong());
                    emit log_named_uint("pairOrderWrong", h.pairOrderWrong());
                    emit log_named_uint("pairAfterTerminal", h.pairAfterTerminal());
                    emit log_named_uint("liveness", h.livenessBroken());
                    emit log_named_bytes("liveness error", h.lastLivenessErr());
                    emit log_named_uint("unnamed", h.unnamedRevert());
                    emit log_named_bytes("unnamed error", h.lastUnnamed());
                    emit log_named_bytes("lastR5", h.lastR5());
                }
                assertEq(h.violations(), 0, "a violation in the pair matrix");
                covered |= h.pairCovered();
                _deps(h, target, moments[m]);
                landed += h.reached(keccak256("pair.bothLanded"));
                runs += h.reached(keccak256("pair.run"));
                full += h.reached(keccak256("pair.fullRule"));
                judgeR += h.reached(keccak256("pair.judgeRule"));
                dar += h.reached(keccak256("pair.disputeAheadOfRefund"));
                vm.revertToState(snapId);
            }
        }
        uint256 n;
        for (uint256 x = 0; x < 16; x++) {
            for (uint256 y = x + 1; y < 16; y++) {
                if ((covered >> (x * 16 + y)) & 1 == 1) n++;
            }
        }
        emit log_named_uint("pair runs", runs);
        emit log_named_uint("runs under the rule that every total agrees", full);
        emit log_named_uint("runs under the rule that the judge's total agrees", judgeR);
        emit log_named_uint("a dispute ahead of a refund", dar);
        emit log_named_uint("unordered pairs with an act landing, over all points", n);
        emit log_named_uint("pair runs where both acts of the first order landed", landed);
        assertEq(n, 120, "every pair ran with an act landing");
    }
}
