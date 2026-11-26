// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

/// The ERC-20 surface the standard reads and calls.
interface IERC20 {
    function balanceOf(address who) external view returns (uint256);
    function transfer(address to, uint256 value) external returns (bool);
    function transferFrom(address from, address to, uint256 value) external returns (bool);
    function decimals() external view returns (uint8);
}

/// The Chainlink-shaped feed a quote may name (eravon.standard/1 s3).
interface IPriceFeed {
    function decimals() external view returns (uint8);
    function latestRoundData()
        external
        view
        returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound);
}

/// The panel book's read side as the instance uses it (EravonPanelBook.sol).
interface IEravonPanelBook {
    function latest(address guild, bytes32 termsHash)
        external
        view
        returns (uint256 index, bytes32 panelHash, bytes32 rosterHash, uint64 expires);
}
