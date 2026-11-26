// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {MinimalVenue} from "./MinimalVenue.sol";
import {IGaleedSuccessionBook, IGaleedRouter, RouteView} from "../src/IGaleedVenue.sol";

/// The harness's drawing reference venue: the same venue as MinimalVenue,
/// holding the panel face itself (s4). A case freezes, at its creation, the
/// guild's root and the latest of the panels recorded under that root and the
/// case's terms before the creation block that are unexpired at it (a panel
/// is unexpired at its `expires` second); every draw for the case reads that
/// panel alone. Where the panel names a router the
/// draw calls it (s4); otherwise the venue seats the first member neither
/// seated nor offered on the case. After an offer expires unaccepted the
/// venue draws again from the frozen panel, the escrow passing to the new
/// seat, and refunds the buyer where the draw finds no member. A drawn case's
/// `caseId` names the guild in the judge's place in MinimalVenue's
/// derivation. Where a draw finds no usable panel, the creation is refused,
/// and a redraw opens no seat and refunds the buyer the amount and the fee,
/// which ends the case (s4). The seed it hands a router is the hash of the
/// block before the drawing call's block: that block's producer can steer it,
/// and the sender of the drawing call chooses the block (the buyer for the
/// creation's draw, anyone for a redraw once the seat's window has passed),
/// so a router whose answer depends on the seed lets that sender choose among
/// the seeds of the blocks it can wait for; the buyer, the seller and the
/// creation's order also shape the `caseId`, which a rotation reads. It is a
/// fixture and never a venue anyone deploys.
contract MinimalDrawingVenue is MinimalVenue {
    struct Panel {
        address[] members;
        bytes32 word;
        bytes32 rosterHash;
        uint64 expires;
        address router;
        bytes32 routerHash;
        uint64 recorded;
    }

    struct Frozen {
        address root;
        bytes32 terms;
        uint256 index;
    }

    /// s4: the append-only record under (root, termsHash).
    mapping(address => mapping(bytes32 => Panel[])) internal panels;
    /// s4: the panel each drawn case froze at its creation.
    mapping(bytes32 => Frozen) internal frozen;

    constructor(address book_) MinimalVenue(book_) {}

    /// s4: the sender is its record's current key; the six rejections; then
    /// one more entry under (root, termsHash), with the router and its code hash.
    function engage(bytes32 termsHash, address[] calldata members, bytes32[] calldata amanahHashes, bytes32 rosterHash, uint64 expires, address router)
        external virtual override
    {
        require(_current(msg.sender) == msg.sender, "not the current key");
        require(members.length == amanahHashes.length, "lengths");
        require(members.length != 0, "empty");
        for (uint256 i = 0; i < amanahHashes.length; i++) {
            require(amanahHashes[i] != bytes32(0), "zero word");
            require(amanahHashes[i] == amanahHashes[0], "words differ");
        }
        require(expires > block.timestamp, "expires");
        require(router == address(0) || router.code.length != 0, "router without code");
        Panel storage p = panels[IGaleedSuccessionBook(book).root(msg.sender)][termsHash].push();
        for (uint256 i = 0; i < members.length; i++) p.members.push(members[i]);
        p.word = amanahHashes[0];
        p.rosterHash = rosterHash;
        p.expires = expires;
        p.router = router;
        p.routerHash = router == address(0) ? bytes32(0) : router.codehash;
        p.recorded = uint64(block.number);
    }

    // ---- where the panels live: here, or on the book a standard names ----

    function _panelCount(address root, bytes32 terms) internal view virtual returns (uint256) {
        return panels[root][terms].length;
    }

    function _panelAt(address root, bytes32 terms, uint256 i) internal view virtual returns (Panel memory) {
        return panels[root][terms][i];
    }

    /// A case whose seat is drawn from the panel it freezes now (s4).
    function createDrawnCase(address guild, bytes32 termsHash, address token, uint256 amount, uint256 fee, address seller, bytes calldata quote)
        external returns (bytes32 caseId)
    {
        bool autoAccept = _autoAccept(quote);
        address root = IGaleedSuccessionBook(book).root(guild);
        uint256 i = _panelCount(root, termsHash);
        // the latest of the panels recorded before the creation block that are unexpired at it
        while (i > 0) {
            Panel memory q = _panelAt(root, termsHash, i - 1);
            if (q.recorded < block.number && q.expires >= block.timestamp) break;
            i--;
        }
        require(i > 0, "no usable panel");
        Panel memory p = _panelAt(root, termsHash, i - 1);
        caseId = keccak256(abi.encode(address(this), nonce++, msg.sender, seller, guild));
        _newCase(caseId, token, amount, fee, seller);
        cases[caseId].consent = autoAccept;
        frozen[caseId] = Frozen(root, termsHash, i - 1);
        (bool drew, address member) = _draw(caseId, p, termsHash, token, amount);
        require(drew, "no usable panel");
        _openSeat(caseId, member, p.word, termsHash, autoAccept, keccak256(quote));
    }

    /// s2.7, s4: a seat is drawn again from the frozen panel.
    function _redrawOf(bytes32 caseId) internal view returns (bool drew, address member, bytes32 word, bytes32 terms) {
        Frozen storage f = frozen[caseId];
        if (f.root == address(0)) return (false, address(0), bytes32(0), bytes32(0));
        Panel memory p = _panelAt(f.root, f.terms, f.index);
        Case storage c = cases[caseId];
        (drew, member) = _draw(caseId, p, f.terms, c.token, c.amount);
        return (drew, member, p.word, f.terms);
    }

    function _wouldRedraw(bytes32 caseId) internal view virtual override returns (bool drew) {
        (drew, , , ) = _redrawOf(caseId);
    }

    function _redrawAfterOffer(bytes32 caseId) internal virtual override returns (bool) {
        return _seatAgain(caseId);
    }

    /// Opens the next seat by a draw from the frozen panel, the escrow passing to
    /// it and the case's copied consent with it (s2.5); false where the draw finds no member.
    function _seatAgain(bytes32 caseId) internal returns (bool) {
        (bool drew, address member, bytes32 word, bytes32 terms) = _redrawOf(caseId);
        if (!drew) return false;
        Seat[] storage ss = cases[caseId].seats;
        _openSeat(caseId, member, word, terms, cases[caseId].consent, ss[ss.length - 1].quoteHash);
        return true;
    }

    function _taken(bytes32 caseId, address a) internal view returns (bool) {
        Seat[] storage ss = cases[caseId].seats;
        for (uint256 i = 0; i < ss.length; i++) if (ss[i].judge == a) return true;
        return false;
    }

    /// s4: the draw for the case from panel `p`; false leaves the case with no usable panel.
    function _draw(bytes32 caseId, Panel memory p, bytes32 terms, address token, uint256 amount) internal view returns (bool, address) {
        if (p.router == address(0)) {
            for (uint256 i = 0; i < p.members.length; i++) if (!_taken(caseId, p.members[i])) return (true, p.members[i]);
            return (false, address(0));
        }
        if (p.router.codehash != p.routerHash) return (false, address(0));
        Seat[] storage ss = cases[caseId].seats;
        address[] memory seated = new address[](ss.length);
        for (uint256 i = 0; i < ss.length; i++) seated[i] = ss[i].judge;
        (uint64 rungs, , ) = _ladder();
        address[] memory parties = new address[](2);
        parties[0] = cases[caseId].buyer;
        parties[1] = cases[caseId].seller;
        RouteView memory v = RouteView({
            caseId: caseId, seat: uint64(ss.length), termsHash: terms, token: token, amount: amount, rungs: rungs,
            parties: parties, members: p.members, seated: seated, seed: blockhash(block.number - 1),
            blockTime: uint64(block.timestamp), chainId: block.chainid, venue: address(this)
        });
        bytes memory data = abi.encodeCall(IGaleedRouter.route, (v));
        uint256 g = ROUTE_GAS;
        // the router receives exactly routeGas(): a transaction that cannot fund it reverts whole
        require(gasleft() > g + g / 63 + 20_000, "route gas");
        address router = p.router;
        bool ok;
        uint256 size;
        uint256 index;
        assembly {
            ok := staticcall(g, router, add(data, 0x20), mload(data), 0, 0)
            size := returndatasize()
            if and(ok, eq(size, 32)) {
                returndatacopy(0, 0, 32)
                index := mload(0)
            }
        }
        if (!ok || size != 32 || index >= p.members.length) return (false, address(0));
        address member = p.members[index];
        if (_taken(caseId, member)) return (false, address(0));
        return (true, member);
    }
}
