use alloy::sol;

sol! {
    #[sol(rpc)]
    interface IVenusComptroller {
        function checkMembership(address account, address vToken) external view returns (bool);
        function enterMarkets(address[] vTokens) external returns (uint256[]);
    }

    #[sol(rpc)]
    interface IVToken {
        function balanceOf(address account) external view returns (uint256);
        function borrow(uint256 amount) external returns (uint256);
        function borrowBalanceCurrent(address account) external returns (uint256);
        function redeem(uint256 redeemTokens) external returns (uint256);
        function redeemUnderlying(uint256 redeemAmount) external returns (uint256);
        function repayBorrow(uint256 amount) external returns (uint256);
    }

    #[sol(rpc)]
    interface IERC20 {
        function balanceOf(address account) external view returns (uint256);
        function allowance(address owner, address spender) external view returns (uint256);
        function approve(address spender, uint256 amount) external returns (bool);
    }
}
