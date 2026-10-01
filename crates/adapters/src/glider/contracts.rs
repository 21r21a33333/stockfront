use alloy::sol;

sol! {
    interface IERC20 {
        function transfer(address recipient, uint256 amount) external returns (bool);
    }
}
