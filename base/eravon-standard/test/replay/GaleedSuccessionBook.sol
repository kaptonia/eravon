// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title GaleedSuccessionBook: who is the current key of each record.
/// @notice No owner, no upgrade, one per chain. A key hands its record on by declare;
///         the named key takes it up by accede; only the two together move the record.
contract GaleedSuccessionBook {
    event Declared(address indexed from, address indexed to);
    event Acceded(address indexed holder, address indexed to, address indexed root);

    mapping(address => address) internal _rootOf;
    mapping(address => address) internal _head;
    mapping(address => address) internal _pending;
    mapping(address => bool) internal _entered;

    function root(address a) public view returns (address) {
        address r = _rootOf[a];
        return r == address(0) ? a : r;
    }

    function current(address a) public view returns (address) {
        address r = root(a);
        address h = _head[r];
        return h == address(0) ? r : h;
    }

    function pending(address a) external view returns (address) {
        return _pending[a];
    }

    function entered(address a) external view returns (bool) {
        return _entered[a];
    }

    function declare(address to) external {
        require(current(msg.sender) == msg.sender);
        require(to != address(0) && to != msg.sender && !_entered[to]);
        _pending[msg.sender] = to;
        _entered[msg.sender] = true;
        emit Declared(msg.sender, to);
    }

    function accede(address holder) external {
        require(!_entered[msg.sender] && _pending[holder] == msg.sender && current(holder) == holder);
        address r = root(holder);
        _rootOf[msg.sender] = r;
        _head[r] = msg.sender;
        delete _pending[holder];
        _entered[msg.sender] = true;
        emit Acceded(holder, msg.sender, r);
    }
}
