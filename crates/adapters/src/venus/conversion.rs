use std::str::FromStr;

use alloy_primitives::U256;
use rust_decimal::Decimal;
use stockfront_core::bstock_borrowing::BorrowingProviderError;

pub(super) fn required<T>(value: Option<T>) -> Result<T, BorrowingProviderError> {
    value.ok_or(BorrowingProviderError::InvalidData)
}

#[allow(clippy::option_option)]
pub(super) fn required_nested<T>(value: Option<Option<T>>) -> Result<T, BorrowingProviderError> {
    required(value.flatten())
}

pub(super) fn parse<T>(value: &str) -> Result<T, BorrowingProviderError>
where
    T: FromStr,
{
    value
        .parse()
        .map_err(|_| BorrowingProviderError::InvalidData)
}

pub(super) fn decode_units(value: U256, decimals: u32) -> Result<Decimal, BorrowingProviderError> {
    let scale = 10_u64
        .checked_pow(decimals)
        .map(Decimal::from)
        .ok_or(BorrowingProviderError::InvalidData)?;
    parse::<Decimal>(&value.to_string())?
        .checked_div(scale)
        .ok_or(BorrowingProviderError::InvalidData)
}

pub(super) fn encode_units(amount: Decimal, decimals: u32) -> Result<U256, BorrowingProviderError> {
    let amount = amount.normalize();
    if amount.scale() > decimals {
        return Err(BorrowingProviderError::Rejected);
    }
    let mantissa =
        u128::try_from(amount.mantissa()).map_err(|_| BorrowingProviderError::Rejected)?;
    (amount.scale()..decimals).try_fold(U256::from(mantissa), |value, _| {
        value
            .checked_mul(U256::from(10))
            .ok_or(BorrowingProviderError::InvalidData)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_units_converts_decimal_units_without_rounding() {
        assert_eq!(
            encode_units(Decimal::new(1_250_001, 6), 18),
            Ok(U256::from(1_250_001_000_000_000_000_u128))
        );
        assert_eq!(
            encode_units(Decimal::new(1_000_001, 7), 6),
            Err(BorrowingProviderError::Rejected)
        );
    }
}
