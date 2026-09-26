use std::error::Error;

use alloy::{
    primitives::{Address, U256},
    providers::Provider,
    sol,
};

sol! {
    #[sol(rpc)]
    interface Erc20Allowance {
        function allowance(address owner, address spender) external view returns (uint256 amount);
    }
}

pub async fn token_allowance(
    provider: &impl Provider,
    token: Address,
    owner: Address,
    spender: Address,
) -> Result<U256, Box<dyn Error>> {
    Ok(Erc20Allowance::new(token, provider)
        .allowance(owner, spender)
        .call()
        .await?)
}
