pragma solidity 0.5.15;

interface IUniswapV2Factory {
    event PairCreated(address indexed token0, address indexed token1, address pair, uint);

    function feeTo() external view returns (address);
    function feeToSetter() external view returns (address);

    function getPair(address tokenA, address tokenB) external view returns (address pair);
    function allPairs(uint) external view returns (address pair);
    function allPairsLength() external view returns (uint);

    function createPair(address tokenA, address tokenB) external returns (address pair);

    function setFeeTo(address) external;
    function setFeeToSetter(address) external;
}


interface Hevm {
  function roll(uint256) external;
}

contract Sample {

  address constant UNI_FACT = address(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f);

  Hevm hevm = Hevm(0x7109709ECfa91a80626fF3989D68f67F5b1DD12D);

  constructor() public {

  }

  function test() public returns (uint256) {
      hevm.roll(10);
  }

  function getUniPair() public returns (address) {
    return IUniswapV2Factory(UNI_FACT).getPair(address(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2), address(0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48));
  }
}
