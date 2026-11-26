// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";

/// The standard's identity (standard s13), computed from the compiled
/// creation code alone: the logic address the public deployer yields under
/// the salt keccak256("eravon/1"), the proxy runtime code that embeds it,
/// and that code's keccak256, which is what a judge declares; the chain's
/// panel book's, the verifier's and the board's hashes and addresses beside
/// them, twelve values. The artifacts are read from `out/` (the same bytes
/// `type(C).creationCode` would embed) so that this script embeds no
/// contract. Prints the lines CODEHASH.md carries; `identity.sh` compares them.
contract Identity is Script {
    address internal constant DEPLOYER = 0x4e59b44847b379578588920cA78FbF26c0B4956C;

    function run() external view {
        bytes32 initHash = keccak256(vm.getCode("Eravon.sol:Eravon"));
        address logic = address(uint160(uint256(keccak256(abi.encodePacked(hex"ff", DEPLOYER, keccak256("eravon/1"), initHash)))));
        bytes memory runtime = abi.encodePacked(hex"363d3d373d3d3d363d73", logic, hex"5af43d82803e903d91602b57fd5bf3");
        console.log("logicInitcodeHash %s", vm.toString(initHash));
        console.log("logic %s", vm.toString(logic));
        console.log("proxyCodeHash %s", vm.toString(keccak256(runtime)));
        bytes32 panelHash = keccak256(vm.getCode("EravonPanelBook.sol:EravonPanelBook"));
        address panel = address(uint160(uint256(keccak256(abi.encodePacked(hex"ff", DEPLOYER, keccak256("eravon/1/panel"), panelHash)))));
        console.log("panelInitcodeHash %s", vm.toString(panelHash));
        console.log("panel %s", vm.toString(panel));
        console.log("panelCodeHash %s", vm.toString(keccak256(vm.getDeployedCode("EravonPanelBook.sol:EravonPanelBook"))));
        bytes32 verifyHash = keccak256(vm.getCode("EravonVerifier.sol:EravonVerifier"));
        address verifier = address(uint160(uint256(keccak256(abi.encodePacked(hex"ff", DEPLOYER, keccak256("eravon/1/verify"), verifyHash)))));
        console.log("verifierInitcodeHash %s", vm.toString(verifyHash));
        console.log("verifier %s", vm.toString(verifier));
        console.log("verifierCodeHash %s", vm.toString(keccak256(vm.getDeployedCode("EravonVerifier.sol:EravonVerifier"))));
        bytes32 boardHash = keccak256(vm.getCode("EravonBoard.sol:EravonBoard"));
        address board = address(uint160(uint256(keccak256(abi.encodePacked(hex"ff", DEPLOYER, keccak256("eravon/1/board"), boardHash)))));
        console.log("boardInitcodeHash %s", vm.toString(boardHash));
        console.log("board %s", vm.toString(board));
        console.log("boardCodeHash %s", vm.toString(keccak256(vm.getDeployedCode("EravonBoard.sol:EravonBoard"))));
    }
}
