// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IGaleedVenue, IGaleedSuccessionBook} from "../src/IGaleedVenue.sol";

interface IERC20Min {
    function transfer(address to, uint256 value) external returns (bool);
    function transferFrom(address from, address to, uint256 value) external returns (bool);
}

/// The harness's own reference venue: a judge named in terms, an open offer
/// with a clock unless the judge's copied quote carries a standing consent
/// (`(bool autoAccept)`, s2.5), a three-rung ladder, no draw. The buyer
/// escrows the amount and the fee at creation; once the judge has the seat a
/// delivery clock runs, and its expiry under `advance` releases the amount to
/// the seller; a party's dispute opens the ruling window. The fee is a
/// standby half, credited when the seat releases or rules, and a ruling
/// half, credited only by a ruling and borne by the buyer who paid it.
/// Rung 0 gives the amount to the buyer, rung 1 halves it, rung 2 gives it
/// to the seller. Roles are `buyer,seller`: `parties[0]` is the buyer and
/// `parties[1]` the seller. What a party receives the venue holds for it
/// (`claim`), never through the economic face (s3). The key that acts for a
/// seated judge, and for a seat's account, is that address's current key in
/// the succession book the venue names (s1, s3). The `caseId` is
/// `keccak256(abi.encode(venue, nonce, buyer, seller, judge))`, the nonce
/// counting the venue's creations, and the venue records it in the creation
/// transaction, which is the moment a case exists (s2.8); no case exists
/// without a seat, so none ends with no seat. Its states are the
/// interface's own, mapped one to one: `0` seated, `1` disputed, `2` ruled,
/// `3` resigned, `4` lapsed, `5` closed. The acceptance `outcomeOf` takes as
/// made is the judge's `accept`, or a copied quote whose `autoAccept` is
/// true; the dispute it takes as made is a party's `raise` on the accepted
/// seat, which leaves the range as it was. The terms bytes stay with the
/// parties: `createCase` takes their hash as given and the venue stores only
/// the hash. It exists to prove the replay runs; it is a fixture and never a
/// venue anyone deploys.
contract MinimalVenue is IGaleedVenue {
    struct Case {
        Seat[] seats;
        address buyer;
        address seller;
        address token;
        uint256 amount;
        uint256 fee;
        bool exists;
        bool settled;
        /// the standing consent the copied quote carries (s2.5), for every seat of the case
        bool consent;
    }

    mapping(bytes32 => Case) internal cases;
    /// the rung and the extra each ruled seat took
    mapping(bytes32 => mapping(uint64 => bytes32)) internal ruledWith;
    mapping(address => address) internal payoutOf;
    mapping(address => mapping(address => uint256)) internal claimableOf;
    /// what the venue holds for a party, outside the economic face (s3)
    mapping(address => mapping(address => uint256)) internal heldOf;
    uint256 internal nonce;

    address internal immutable book;
    uint64 internal constant OFFER_WINDOW = 3 days;
    uint64 internal constant DELIVERY_WINDOW = 5 days;
    uint64 internal constant RULING_WINDOW = 7 days;
    uint64 internal constant ROUTE_GAS = 500_000;

    uint8 internal constant DUE_NONE = 0;
    uint8 internal constant DUE_OFFER = 1;
    uint8 internal constant DUE_RELEASE = 2;
    uint8 internal constant DUE_LAPSE = 3;

    constructor(address book_) {
        book = book_;
    }

    function successionBook() external view returns (address) { return book; }
    function routeGas() external pure returns (uint64) { return ROUTE_GAS; }

    function _current(address a) internal view returns (address) {
        return IGaleedSuccessionBook(book).current(a);
    }

    // ---- creation: the parties' own path ----
    function createCase(address judge, bytes32 amanahHash, bytes32 termsHash, address token, uint256 amount, uint256 fee, address seller, bytes calldata quote)
        external returns (bytes32 caseId)
    {
        require(amanahHash != bytes32(0), "zero word");
        caseId = keccak256(abi.encode(address(this), nonce++, msg.sender, seller, judge));
        _newCase(caseId, token, amount, fee, seller);
        cases[caseId].consent = _autoAccept(quote);
        _openSeat(caseId, judge, amanahHash, termsHash, cases[caseId].consent, keccak256(quote));
    }

    function _autoAccept(bytes calldata quote) internal pure returns (bool) {
        require(quote.length == 32, "quote");
        return abi.decode(quote, (bool));
    }

    function _newCase(bytes32 caseId, address token, uint256 amount, uint256 fee, address seller) internal {
        Case storage c = cases[caseId];
        c.exists = true;
        c.buyer = msg.sender;
        c.seller = seller;
        c.token = token;
        c.amount = amount;
        c.fee = fee;
        require(IERC20Min(token).transferFrom(msg.sender, address(this), amount + fee), "escrow");
    }

    function _openSeat(bytes32 caseId, address judge, bytes32 word, bytes32 termsHash, bool autoAccept, bytes32 quoteHash)
        internal returns (uint64 ordinal)
    {
        Case storage c = cases[caseId];
        ordinal = uint64(c.seats.length);
        Seat storage s = c.seats.push();
        s.judge = judge;
        s.amanahHash = word;
        s.termsHash = termsHash;
        s.quoteHash = quoteHash;
        s.windowStart = uint64(block.timestamp);
        if (autoAccept) {
            // the terms copied the judge's standing consent: seated with consent carried, the delivery clock runs
            s.accepted = uint64(block.timestamp);
            s.windowEnd = uint64(block.timestamp) + DELIVERY_WINDOW;
        } else {
            s.windowEnd = uint64(block.timestamp) + OFFER_WINDOW;
        }
        s.state = 0;
        (s.rungs, s.rungMin, s.rungMax) = _ladder();
        s.token = c.token;
        s.amount = c.amount;
        s.parties.push(c.buyer);
        s.parties.push(c.seller);
        emit Seated(judge, caseId, ordinal);
    }

    function _seat(bytes32 caseId, uint64 seat) internal view returns (Seat storage) {
        require(cases[caseId].exists && seat < cases[caseId].seats.length, "unknown seat");
        return cases[caseId].seats[seat];
    }

    /// A party raises the dispute on an accepted seat inside its delivery clock; the ruling window opens.
    function raise(bytes32 caseId, uint64 seat) external {
        Case storage c = cases[caseId];
        require(c.exists && (msg.sender == c.buyer || msg.sender == c.seller), "party");
        Seat storage s = _seat(caseId, seat);
        require(s.state == 0 && s.accepted != 0 && block.timestamp <= s.windowEnd, "not open to dispute");
        s.state = 1;
        s.windowStart = uint64(block.timestamp);
        s.windowEnd = uint64(block.timestamp) + RULING_WINDOW;
        _onDispute(s);
    }

    /// The ladder a new seat reports: three rungs, all accepted.
    function _ladder() internal pure virtual returns (uint64 rungs, uint64 rungMin, uint64 rungMax) { return (3, 0, 2); }

    /// What the dispute does to the seat's range: nothing here.
    function _onDispute(Seat storage) internal virtual {}

    /// A party takes what the venue holds for it.
    function claim(address token) external {
        uint256 v = heldOf[msg.sender][token];
        heldOf[msg.sender][token] = 0;
        require(IERC20Min(token).transfer(msg.sender, v), "transfer");
    }

    // ---- case face ----
    function seatOf(bytes32 caseId, uint64 seat) external view returns (Seat memory) {
        return _seat(caseId, seat);
    }

    function quoteType() external pure returns (string memory) { return "(bool autoAccept)"; }
    function extraType() external pure virtual returns (string memory) { return ""; }
    function partyRoles() external pure returns (string memory) { return "buyer,seller"; }

    /// s2.9: the seated judge's current key, a direct call, no code.
    function _direct(Seat storage s) internal view {
        require(msg.sender == _current(s.judge) && msg.sender == tx.origin && msg.sender.code.length == 0, "direct call");
    }

    function accept(bytes32 caseId, uint64 seat) external {
        Seat storage s = _seat(caseId, seat);
        _direct(s);
        require(s.state == 0 && s.accepted == 0, "not open");
        require(block.timestamp <= s.windowEnd, "offer expired");
        s.accepted = uint64(block.timestamp);
        // the offer clock is spent; the delivery clock runs
        s.windowStart = uint64(block.timestamp);
        s.windowEnd = uint64(block.timestamp) + DELIVERY_WINDOW;
    }

    function commit(bytes32 caseId, bytes32 manifest) external {
        Case storage c = cases[caseId];
        require(c.exists, "unknown case");
        bool live = false;
        for (uint256 i = 0; i < c.seats.length; i++) if (c.seats[i].state <= 1) live = true;
        require(live, "case ended");
        emit Committed(caseId, msg.sender, manifest);
    }

    // ---- the money of a seat (s2.12) ----

    function _fees(Case storage c) internal view returns (uint256 standby, uint256 ruling) {
        ruling = c.fee / 2;
        standby = c.fee - ruling;
    }

    function _outcome(uint8 state) internal pure returns (Outcome memory o) {
        o.state = state;
        o.shares = new uint256[](2);
        o.rulingFeeFrom = new uint256[](2);
    }

    /// What a ruling at `rung` does: the split of the amount, both fee halves, the ruling half borne by the buyer.
    function _ruling(Case storage c, uint64 rung, bytes calldata extra) internal view virtual returns (Outcome memory o) {
        require(extra.length == 0, "extra");
        o = _outcome(2);
        o.shares[0] = c.amount * (2 - rung) / 2;
        o.shares[1] = c.amount - o.shares[0];
        (o.standbyFee, o.rulingFee) = _fees(c);
        o.rulingFeeFrom[0] = o.rulingFee;
    }

    /// What `advance` is due to do on the seat now.
    function _due(Seat storage s) internal view returns (uint8) {
        if (!(s.state == 0 || s.state == 1) || s.windowEnd == 0 || block.timestamp <= s.windowEnd) return DUE_NONE;
        if (s.state == 1) return DUE_LAPSE;
        return s.accepted == 0 ? DUE_OFFER : DUE_RELEASE;
    }

    function _credit(Case storage c, Seat storage s, uint256 standby, uint256 ruling) internal {
        claimableOf[s.judge][c.token] += standby + ruling;
        s.standbyCredited += standby;
        s.rulingCredited += ruling;
    }

    function _hold(address who, address token, uint256 v) internal {
        heldOf[who][token] += v;
    }

    /// Everything the case escrowed goes back to the buyer.
    function _refundAll(Case storage c) internal {
        require(!c.settled, "settled");
        c.settled = true;
        _hold(c.buyer, c.token, c.amount + c.fee);
    }

    function outcomeOf(bytes32 caseId, uint64 seat, uint64 rung, bytes calldata extra) external view returns (Outcome memory) {
        Seat storage s = _seat(caseId, seat);
        if (s.state == 2) {
            require(keccak256(abi.encode(rung, extra)) == ruledWith[caseId][seat], "not the ruling the seat took");
        } else {
            require(s.state == 0 || s.state == 1, "not in flight");
            require(s.rungs == 0 || (rung >= s.rungMin && rung <= s.rungMax), "rung");
        }
        return _ruling(cases[caseId], rung, extra);
    }

    function advanceOf(bytes32 caseId, uint64 seat) external view returns (Outcome memory o) {
        Seat storage s = _seat(caseId, seat);
        Case storage c = cases[caseId];
        uint8 d = _due(s);
        if (d == DUE_NONE) return _outcome(s.state);
        if (d == DUE_LAPSE) {
            o = _outcome(4);
            if (!_wouldRedrawLapse(caseId)) o.shares[0] = c.amount;
        } else if (d == DUE_RELEASE) {
            o = _outcome(5);
            o.shares[1] = c.amount;
            (o.standbyFee, ) = _fees(c);
        } else {
            o = _outcome(5);
            if (!_wouldRedraw(caseId)) o.shares[0] = c.amount;
        }
    }

    function rule(bytes32 caseId, uint64 seat, bytes32 entryId, uint64 rung, bytes calldata entry, bytes calldata extra) external {
        Seat storage s = _seat(caseId, seat);
        require(sha256(entry) == entryId, "entry");
        _direct(s);
        require(s.state == 1, "not disputed");
        require(block.timestamp <= s.windowEnd, "window closed");
        require(s.rungs == 0 || (rung >= s.rungMin && rung <= s.rungMax), "rung");
        Case storage c = cases[caseId];
        Outcome memory o = _ruling(c, rung, extra);
        require(!c.settled, "settled");
        c.settled = true;
        s.state = 2;
        ruledWith[caseId][seat] = keccak256(abi.encode(rung, extra));
        _hold(c.buyer, c.token, o.shares[0]);
        _hold(c.seller, c.token, o.shares[1]);
        // s3: a seat's earnings accrue to the seat's own identity, the address recorded
        _credit(c, s, o.standbyFee, o.rulingFee);
        emit Anchored(msg.sender, entryId, entry);
    }

    function resign(bytes32 caseId, uint64 seat, bytes32 entryId, bytes calldata entry) external {
        Seat storage s = _seat(caseId, seat);
        require(sha256(entry) == entryId, "entry");
        _direct(s);
        require(s.state == 0 || s.state == 1, "not in flight");
        s.state = 3;
        _refundAll(cases[caseId]);
        emit Anchored(msg.sender, entryId, entry);
    }

    function advance(bytes32 caseId, uint64 seat) external {
        if (!cases[caseId].exists || seat >= cases[caseId].seats.length) return;
        Case storage c = cases[caseId];
        Seat storage s = c.seats[seat];
        uint8 d = _due(s);
        if (d == DUE_NONE) return;
        if (d == DUE_LAPSE) {
            // the draw reads the case as advanceOf found it, before the seat's state is written
            bool redrew = _redrawAfterLapse(caseId);
            s.state = 4;
            if (!redrew) _refundAll(c);
        } else if (d == DUE_RELEASE) {
            s.state = 5;
            require(!c.settled, "settled");
            c.settled = true;
            (uint256 standby, uint256 ruling) = _fees(c);
            _hold(c.seller, c.token, c.amount);
            _hold(c.buyer, c.token, ruling);
            _credit(c, s, standby, 0);
        } else {
            bool redrew = _redrawAfterOffer(caseId);
            s.state = 5;
            if (!redrew) _refundAll(c);
        }
    }

    /// Whether the venue seats again after an offer expired unaccepted, and
    /// after a lapse: never here.
    function _wouldRedraw(bytes32) internal view virtual returns (bool) { return false; }
    function _redrawAfterOffer(bytes32) internal virtual returns (bool) { return false; }
    function _wouldRedrawLapse(bytes32) internal view virtual returns (bool) { return false; }
    function _redrawAfterLapse(bytes32) internal virtual returns (bool) { return false; }

    // ---- economic face ----
    function setPayout(address key, address payout) external {
        require(msg.sender == _current(key), "not the current key");
        require(payoutOf[key] == address(0), "payout set");
        require(payout != address(0), "zero");
        payoutOf[key] = payout;
    }

    function withdraw(address token, address key, address payout) external {
        require(msg.sender == _current(key), "not the current key");
        address p = payoutOf[key];
        if (p == address(0)) {
            if (payout != address(0)) {
                payoutOf[key] = payout;
                p = payout;
            }
        } else {
            require(payout == address(0) || payout == p, "another payout");
        }
        require(p != address(0), "no payout");
        uint256 v = claimableOf[key][token];
        claimableOf[key][token] = 0;
        require(IERC20Min(token).transfer(p, v), "transfer");
    }

    function accountOf(address who, address token) external view returns (address payout, uint256 claimable) {
        return (payoutOf[who], claimableOf[who][token]);
    }

    // ---- panel face: this venue never draws ----
    function engage(bytes32, address[] calldata, bytes32[] calldata, bytes32, uint64, address) external virtual {
        revert("no draw");
    }
}
