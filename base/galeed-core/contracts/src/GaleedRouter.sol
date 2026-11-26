// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// What a venue holds at a draw (galeed.venue/1 section 1, `RouteView`),
/// declared here field for field so that `route` has the selector the
/// venue interface names.
struct RouteView {
    bytes32   caseId;
    uint64    seat;
    bytes32   termsHash;
    address   token;
    uint256   amount;
    uint64    rungs;
    address[] parties;
    address[] members;
    address[] seated;
    bytes32   seed;
    uint64    blockTime;
    uint256   chainId;
    address   venue;
}

/// @title GaleedRouter: the family's standard router (galeed.venue/1 section 4)
/// @notice No owner, no upgrade, no delegatecall, no storage. The rule table is
///         fixed at construction and kept as the code of a data contract the
///         constructor creates, so that `route` reads its arguments and code
///         alone and every deployment carries one runtime code and one code
///         hash whatever its table.
/// @dev    `route` walks the rules in order. A rule holds where each of its
///         conditions holds; the first rule that holds and whose action finds
///         an eligible member decides, a member being eligible where it is in
///         the panel's `members` and in neither the seated nor the offered
///         members of the case. A rule whose pool is the panel's members draws
///         among the places of `members` and returns the place it lands on, so
///         that a member listed twice is drawn twice as often; a rule with its
///         own pool returns its member's first place in `members`. Where no
///         rule decides, `route` reverts, which leaves the case with no usable
///         panel.
contract GaleedRouter {
    /// Draw uniformly by the seed among the eligible members of the pool.
    uint8 public constant UNIFORM = 0;
    /// Draw by the seed among the eligible members of the pool, each in
    /// proportion to its weight.
    uint8 public constant WEIGHTED = 1;
    /// Start at the pool element the hash of the case id names and take the
    /// first eligible member from there in pool order, wrapping.
    uint8 public constant ROTATION = 2;

    struct Rule {
        // conditions: the zero address and the zero word match every value;
        // each range is inclusive at both ends
        address   token;
        uint256   amountMin;
        uint256   amountMax;
        bytes32   termsHash;
        uint64    rungsMin;
        uint64    rungsMax;
        uint64    seatMin;
        uint64    seatMax;
        // action
        uint8     mode;
        address[] pool;     // the members it draws among, in order; empty: the panel's members
        uint64[]  weights;  // WEIGHTED: one positive weight per pool element; otherwise empty
    }

    error EmptyTable();
    error BadRule(uint256 index);
    error TableTooLarge();
    error NoMember();
    error ForeignTable();

    constructor(Rule[] memory rules_) {
        if (rules_.length == 0) revert EmptyTable();
        for (uint256 i = 0; i < rules_.length; i++) {
            Rule memory r = rules_[i];
            bool ok = r.amountMin <= r.amountMax && r.rungsMin <= r.rungsMax && r.seatMin <= r.seatMax
                && r.mode <= ROTATION;
            if (r.mode == WEIGHTED) {
                ok = ok && r.pool.length != 0 && r.weights.length == r.pool.length;
                for (uint256 k = 0; ok && k < r.weights.length; k++) ok = r.weights[k] != 0;
            } else {
                ok = ok && r.weights.length == 0;
            }
            for (uint256 k = 0; ok && k < r.pool.length; k++) {
                ok = r.pool[k] != address(0);
                for (uint256 m = 0; ok && m < k; m++) ok = r.pool[m] != r.pool[k];
            }
            if (!ok) revert BadRule(i);
        }
        // the table is the code of a contract whose first byte is STOP, so that
        // nothing can execute it and nothing can change it
        bytes memory data = abi.encodePacked(hex"00", abi.encode(rules_));
        if (data.length > 24576) revert TableTooLarge();
        bytes memory init = abi.encodePacked(hex"61", uint16(data.length), hex"80600a3d393df3", data);
        address t;
        assembly {
            t := create(0, add(init, 0x20), mload(init))
        }
        if (t == address(0) || t != _table()) revert TableTooLarge();
    }

    /// The data contract the constructor created: the first CREATE of this
    /// address, whose nonce was 1 at construction.
    function _table() internal view returns (address) {
        return address(uint160(uint256(keccak256(abi.encodePacked(hex"d694", address(this), hex"01")))));
    }

    /// The table the data contract holds. Its first byte is the STOP the
    /// constructor writes; code that begins otherwise could execute, and is no
    /// table of this router.
    function _rules() internal view returns (Rule[] memory) {
        address t = _table();
        uint256 size = t.code.length;
        if (size == 0) revert ForeignTable();
        bytes memory head = new bytes(1);
        bytes memory raw = new bytes(size - 1);
        assembly {
            extcodecopy(t, add(head, 0x20), 0, 1)
            extcodecopy(t, add(raw, 0x20), 1, sub(size, 1))
        }
        if (head[0] != 0x00) revert ForeignTable();
        return abi.decode(raw, (Rule[]));
    }

    /// The rule table as constructed.
    function rules() external view returns (Rule[] memory) {
        return _rules();
    }

    function route(RouteView calldata v) external view returns (uint256 index) {
        Rule[] memory rs = _rules();
        for (uint256 i = 0; i < rs.length; i++) {
            Rule memory r = rs[i];
            if (!_holds(r, v)) continue;
            (bool found, uint256 at) = _act(r, v);
            if (found) return at;
        }
        revert NoMember();
    }

    function _holds(Rule memory r, RouteView calldata v) internal pure returns (bool) {
        return (r.token == address(0) || r.token == v.token)
            && r.amountMin <= v.amount && v.amount <= r.amountMax
            && (r.termsHash == bytes32(0) || r.termsHash == v.termsHash)
            && r.rungsMin <= v.rungs && v.rungs <= r.rungsMax
            && r.seatMin <= v.seat && v.seat <= r.seatMax;
    }

    function _seated(address a, address[] memory seated) internal pure returns (bool) {
        for (uint256 k = 0; k < seated.length; k++) if (seated[k] == a) return true;
        return false;
    }

    /// Whether the `k`th element of the rule's list is eligible, and its place
    /// in `members`: `k` itself where the list is the panel's members, the
    /// member's first place where the rule has its own pool.
    function _eligible(address[] memory list, uint256 k, bool panel, address[] calldata members, address[] memory seated)
        internal pure returns (bool, uint256)
    {
        address a = list[k];
        if (_seated(a, seated)) return (false, 0);
        if (panel) return (true, k);
        for (uint256 j = 0; j < members.length; j++) if (members[j] == a) return (true, j);
        return (false, 0);
    }

    function _act(Rule memory r, RouteView calldata v) internal pure returns (bool, uint256) {
        address[] calldata members = v.members;
        address[] memory seated = v.seated;
        bool panel = r.pool.length == 0;
        address[] memory list = panel ? members : r.pool;
        uint256 n = list.length;
        if (n == 0) return (false, 0);
        if (r.mode == ROTATION) {
            uint256 start = uint256(keccak256(abi.encode(v.caseId))) % n;
            for (uint256 k = 0; k < n; k++) {
                (bool ok, uint256 at) = _eligible(list, (start + k) % n, panel, members, seated);
                if (ok) return (true, at);
            }
            return (false, 0);
        }
        uint256[] memory places = new uint256[](n);
        uint256[] memory weight = new uint256[](n);
        uint256 total = 0;
        uint256 m = 0;
        for (uint256 k = 0; k < n; k++) {
            (bool ok, uint256 at) = _eligible(list, k, panel, members, seated);
            if (!ok) continue;
            places[m] = at;
            weight[m] = r.mode == WEIGHTED ? r.weights[k] : 1;
            total += weight[m];
            m++;
        }
        if (m == 0) return (false, 0);
        uint256 x = uint256(keccak256(abi.encode(v.seed, v.caseId, v.seat))) % total;
        for (uint256 k = 0; k < m; k++) {
            if (x < weight[k]) return (true, places[k]);
            x -= weight[k];
        }
        return (false, 0);
    }
}
