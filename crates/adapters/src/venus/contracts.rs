use alloy::sol;

sol! {
    #[sol(rpc)]
    interface IVenusComptroller {
        function checkMembership(address account, address vToken) external view returns (bool);
        function enterMarkets(address[] vTokens) external returns (uint256[]);
    }

    interface IVToken {
        function borrow(uint256 amount) external returns (uint256);
    }
}
