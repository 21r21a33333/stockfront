use std::error::Error;

use alloy::{
    primitives::{Address, U256},
    providers::Provider,
    sol,
};

sol! {
    #[sol(rpc)]
    interface Erc20 {
        function balanceOf(address owner) external view returns (uint256 balance);
    }
}

pub async fn token_balance(
    provider: &impl Provider,
    token: Address,
    owner: Address,
) -> Result<U256, Box<dyn Error>> {
    Ok(Erc20::new(token, provider).balanceOf(owner).call().await?)
}
