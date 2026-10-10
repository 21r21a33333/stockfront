//! Deterministic translation of a model-drafted intent into a recipe.
//!
//! Every address, raw amount, port and edge is derived here from live catalog
//! and account data, so model output can only select among known values.

use alloy_primitives::{Address, U256};
use rust_decimal::Decimal;
use uuid::Uuid;

use super::{
    ComposerAsset, ComposerAssetKind, ComposerPosition, ComposerPositionKind, StrategyIntent,
    StrategyStep,
};
use crate::{
    bstock_borrowing::{BorrowingAccount, BorrowingVenue},
    bstock_market::{BNB_CHAIN_ID, BstockSummary},
    bstock_trade::SettlementTokenMetadata,
    recipes::{
        RecipeDefinition, RecipeEdge, RecipeNode, RecipePortRef, RecipePrimitive, SCHEMA_VERSION,
    },
};

/// Node IDs are stable per step so diagnostics can be mapped back to steps.
const NODE_ID_BASE: u128 = 0x5354_5246_0000_4000_8000_0000_0000_0000;
const EDGE_ID_BASE: u128 = NODE_ID_BASE + 0x1_0000_0000;

pub(super) enum Resolution {
    Recipe(RecipeDefinition),
    /// Only the user can answer these, so another model attempt cannot help.
    NeedsInput(Vec<String>),
    /// The draft broke a rule the model can fix on another attempt.
    Rejected(Vec<String>),
}

enum StepProblem {
    NeedsInput(String),
    Invalid(String),
}

struct KnownToken {
    address: Address,
    symbol: String,
    name: String,
    decimals: u8,
    aliases: Vec<String>,
    kind: ComposerAssetKind,
}

struct HeldPosition {
    kind: ComposerPositionKind,
    token: Address,
    amount: Decimal,
}

pub(super) struct Resolver {
    tokens: Vec<KnownToken>,
    pool: Option<Address>,
    wallet_connected: bool,
    positions: Vec<HeldPosition>,
}

impl Resolver {
    pub(super) fn new(
        bstocks: &[BstockSummary],
        settlement_tokens: &[SettlementTokenMetadata],
        debt_tokens: &[Address],
        pool: Option<Address>,
        account: Option<&BorrowingAccount>,
    ) -> Self {
        let mut tokens = settlement_tokens
            .iter()
            .map(|token| KnownToken {
                address: token.address,
                symbol: token.symbol.clone(),
                name: token.symbol.clone(),
                decimals: token.decimals,
                aliases: vec![token.symbol.to_lowercase()],
                kind: ComposerAssetKind::Stablecoin {
                    borrowable: debt_tokens.contains(&token.address),
                },
            })
            .collect::<Vec<_>>();
        tokens.extend(bstocks.iter().map(|bstock| {
            KnownToken {
                address: bstock.id.contract,
                symbol: bstock.token_symbol.clone(),
                name: bstock.underlying_name.clone(),
                decimals: bstock.token_decimals,
                aliases: [
                    &bstock.token_symbol,
                    &bstock.underlying_ticker,
                    &bstock.underlying_name,
                ]
                .iter()
                .map(|alias| alias.to_lowercase())
                .collect(),
                kind: ComposerAssetKind::Stock,
            }
        }));

        let positions = account
            .into_iter()
            .flat_map(|account| &account.positions)
            .filter(|position| Some(position.pool) == pool)
            .flat_map(|position| {
                let supplied = position
                    .supplied
                    .iter()
                    .map(|asset| (ComposerPositionKind::Supplied, asset));
                let borrowed = position
                    .borrowed
                    .iter()
                    .map(|asset| (ComposerPositionKind::Borrowed, asset));
                supplied.chain(borrowed)
            })
            .filter(|(_, asset)| asset.amount > Decimal::ZERO)
            .filter(|(_, asset)| {
                tokens
                    .iter()
                    .any(|token| token.address == asset.token.contract)
            })
            .map(|(kind, asset)| HeldPosition {
                kind,
                token: asset.token.contract,
                amount: asset.amount,
            })
            .collect();

        Self {
            tokens,
            pool,
            wallet_connected: account.is_some(),
            positions,
        }
    }

    pub(super) fn assets(&self) -> Vec<ComposerAsset> {
        self.tokens
            .iter()
            .map(|token| ComposerAsset {
                symbol: token.symbol.clone(),
                name: token.name.clone(),
                kind: token.kind,
            })
            .collect()
    }

    pub(super) fn positions(&self) -> Vec<ComposerPosition> {
        self.positions
            .iter()
            .filter_map(|position| {
                self.token_by_address(position.token)
                    .map(|token| ComposerPosition {
                        kind: position.kind,
                        symbol: token.symbol.clone(),
                        amount: position.amount,
                    })
            })
            .collect()
    }

    pub(super) fn resolve(&self, intent: &StrategyIntent) -> Resolution {
        let mut needs_input = Vec::new();
        let mut issues = Vec::new();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for (index, step) in intent.steps.iter().enumerate() {
            let label = format!("Step {}", index + 1);
            let starts_strategy = is_source(step);
            if index == 0 && !starts_strategy {
                issues.push(format!(
                    "{label} must be a wallet asset, a supplied position or a borrowed position"
                ));
            }
            if index > 0 && starts_strategy {
                issues.push(format!(
                    "{label} starts a second chain; a strategy has exactly one starting asset or position"
                ));
            }
            if let Some(previous) = index.checked_sub(1).and_then(|i| intent.steps.get(i)) {
                match (output_port(previous), input_port(step)) {
                    (Some(from), Some(to)) => edges.push(RecipeEdge {
                        id: edge_id(index),
                        from: RecipePortRef {
                            node_id: node_id(index - 1),
                            port: from.to_owned(),
                        },
                        to: RecipePortRef {
                            node_id: node_id(index),
                            port: to.to_owned(),
                        },
                    }),
                    (None, _) => issues.push(format!(
                        "Step {index} ends the strategy, so step {} cannot follow it",
                        index + 1
                    )),
                    (Some(_), None) => {}
                }
            }
            match self.primitive(step, &label) {
                Ok(primitive) => nodes.push(RecipeNode {
                    id: node_id(index),
                    primitive,
                }),
                Err(StepProblem::NeedsInput(message)) => needs_input.push(message),
                Err(StepProblem::Invalid(message)) => issues.push(message),
            }
        }

        if !issues.is_empty() {
            Resolution::Rejected(issues)
        } else if !needs_input.is_empty() {
            Resolution::NeedsInput(needs_input)
        } else {
            Resolution::Recipe(RecipeDefinition {
                schema_version: SCHEMA_VERSION,
                chain_id: BNB_CHAIN_ID,
                nodes,
                edges,
            })
        }
    }

    fn primitive(&self, step: &StrategyStep, label: &str) -> Result<RecipePrimitive, StepProblem> {
        match step {
            StrategyStep::WalletAsset { symbol, amount } => {
                let token = self.token(symbol, label)?;
                let amount = amount.ok_or_else(|| {
                    StepProblem::NeedsInput(format!(
                        "How much {} should the strategy use?",
                        token.symbol
                    ))
                })?;
                Ok(RecipePrimitive::WalletAsset {
                    token: token.address,
                    raw_amount: raw_amount(token, amount, label)?,
                })
            }
            StrategyStep::SuppliedPosition { symbol } => {
                let token = self.token(symbol, label)?;
                if token.kind != ComposerAssetKind::Stock {
                    return Err(StepProblem::Invalid(format!(
                        "{label}: only stocks can be supplied positions, not {}",
                        token.symbol
                    )));
                }
                let pool = self.held(ComposerPositionKind::Supplied, token, "supplied to Venus")?;
                Ok(RecipePrimitive::SuppliedPosition {
                    venue: BorrowingVenue::Venus,
                    pool,
                    token: token.address,
                })
            }
            StrategyStep::BorrowedPosition { symbol } => {
                let token = self.debt_token(symbol, label)?;
                let pool = self.held(ComposerPositionKind::Borrowed, token, "debt on Venus")?;
                Ok(RecipePrimitive::BorrowedPosition {
                    venue: BorrowingVenue::Venus,
                    pool,
                    token: token.address,
                })
            }
            StrategyStep::Trade { buy_symbol } => Ok(RecipePrimitive::Trade {
                buy_token: self.token(buy_symbol, label)?.address,
            }),
            StrategyStep::Supply => Ok(RecipePrimitive::Supply {
                venue: BorrowingVenue::Venus,
            }),
            StrategyStep::Withdraw => Ok(RecipePrimitive::Withdraw),
            StrategyStep::Borrow { symbol, amount } => {
                let token = self.debt_token(symbol, label)?;
                let amount = amount.ok_or_else(|| {
                    StepProblem::NeedsInput(format!(
                        "How much {} should be borrowed?",
                        token.symbol
                    ))
                })?;
                Ok(RecipePrimitive::Borrow {
                    debt_token: token.address,
                    raw_amount: raw_amount(token, amount, label)?,
                })
            }
            StrategyStep::Repay => Ok(RecipePrimitive::Repay),
            StrategyStep::EarnDeposit { symbol } => {
                let token = self.token(symbol, label)?;
                if token.kind != ComposerAssetKind::Stock {
                    return Err(StepProblem::Invalid(format!(
                        "{label}: the earn benchmark must be a stock, not {}",
                        token.symbol
                    )));
                }
                // Which reviewed opportunity to use is live policy, so the
                // composer fills it once the deposit token is known.
                Ok(RecipePrimitive::EarnDeposit {
                    collateral_token: token.address,
                    investment_id: String::new(),
                })
            }
        }
    }

    fn token(&self, symbol: &str, label: &str) -> Result<&KnownToken, StepProblem> {
        let needle = symbol.trim().trim_start_matches('$').to_lowercase();
        let mut matches = self
            .tokens
            .iter()
            .filter(|token| token.aliases.contains(&needle));
        match (matches.next(), matches.next()) {
            (Some(token), None) => Ok(token),
            (Some(first), Some(second)) => Err(StepProblem::Invalid(format!(
                "{label}: {symbol} is ambiguous between {} and {}",
                first.symbol, second.symbol
            ))),
            (None, _) => Err(StepProblem::Invalid(format!(
                "{label}: {symbol} is not a supported asset"
            ))),
        }
    }

    fn debt_token(&self, symbol: &str, label: &str) -> Result<&KnownToken, StepProblem> {
        let token = self.token(symbol, label)?;
        if token.kind == (ComposerAssetKind::Stablecoin { borrowable: true }) {
            Ok(token)
        } else {
            Err(StepProblem::Invalid(format!(
                "{label}: {} cannot be borrowed on Venus",
                token.symbol
            )))
        }
    }

    fn held(
        &self,
        kind: ComposerPositionKind,
        token: &KnownToken,
        description: &str,
    ) -> Result<Address, StepProblem> {
        let pool = self.pool.ok_or_else(|| {
            StepProblem::Invalid("Venus positions are not supported by this deployment".to_owned())
        })?;
        if !self.wallet_connected {
            return Err(StepProblem::NeedsInput(format!(
                "Connect a wallet so Stockfront can use your {} {description}.",
                token.symbol
            )));
        }
        if self
            .positions
            .iter()
            .any(|position| position.kind == kind && position.token == token.address)
        {
            Ok(pool)
        } else {
            Err(StepProblem::NeedsInput(format!(
                "The connected wallet has no {} {description}.",
                token.symbol
            )))
        }
    }

    fn token_by_address(&self, address: Address) -> Option<&KnownToken> {
        self.tokens.iter().find(|token| token.address == address)
    }
}

/// Maps a generated node ID back to its zero-based step index.
pub(super) fn step_index(node: Uuid) -> Option<usize> {
    node.as_u128()
        .checked_sub(NODE_ID_BASE)
        .and_then(|offset| usize::try_from(offset).ok())
        .filter(|index| *index < 0x1_0000_0000)
}

fn node_id(index: usize) -> Uuid {
    Uuid::from_u128(NODE_ID_BASE + index as u128)
}

fn edge_id(index: usize) -> Uuid {
    Uuid::from_u128(EDGE_ID_BASE + index as u128)
}

const fn is_source(step: &StrategyStep) -> bool {
    matches!(
        step,
        StrategyStep::WalletAsset { .. }
            | StrategyStep::SuppliedPosition { .. }
            | StrategyStep::BorrowedPosition { .. }
    )
}

const fn output_port(step: &StrategyStep) -> Option<&'static str> {
    match step {
        StrategyStep::WalletAsset { .. } => Some("asset"),
        StrategyStep::SuppliedPosition { .. } => Some("position"),
        StrategyStep::BorrowedPosition { .. } => Some("debt"),
        StrategyStep::Trade { .. } => Some("buy"),
        StrategyStep::Borrow { .. } => Some("borrowed"),
        StrategyStep::Supply
        | StrategyStep::Withdraw
        | StrategyStep::Repay
        | StrategyStep::EarnDeposit { .. } => None,
    }
}

const fn input_port(step: &StrategyStep) -> Option<&'static str> {
    match step {
        StrategyStep::Trade { .. } => Some("sell"),
        StrategyStep::Supply | StrategyStep::EarnDeposit { .. } => Some("asset"),
        StrategyStep::Withdraw => Some("position"),
        StrategyStep::Borrow { .. } => Some("collateral"),
        StrategyStep::Repay => Some("debt"),
        StrategyStep::WalletAsset { .. }
        | StrategyStep::SuppliedPosition { .. }
        | StrategyStep::BorrowedPosition { .. } => None,
    }
}

fn raw_amount(token: &KnownToken, amount: Decimal, label: &str) -> Result<U256, StepProblem> {
    let normalized = amount.normalize();
    let raw = u128::try_from(normalized.mantissa())
        .ok()
        .filter(|mantissa| *mantissa > 0)
        .zip(
            token
                .decimals
                .checked_sub(u8::try_from(normalized.scale()).unwrap_or(u8::MAX)),
        )
        .and_then(|(mantissa, exponent)| {
            U256::from(10_u8)
                .checked_pow(U256::from(exponent))
                .and_then(|scale| U256::from(mantissa).checked_mul(scale))
        });
    raw.ok_or_else(|| {
        StepProblem::Invalid(format!(
            "{label}: {amount} {} must be positive with at most {} decimal places",
            token.symbol, token.decimals
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bstock_borrowing::{BorrowingAsset, BorrowingPosition, BorrowingToken},
        bstock_market::{AssetKind, BstockId},
    };

    const USDT: Address = Address::with_last_byte(1);
    const TSLAB: Address = Address::with_last_byte(2);
    const POOL: Address = Address::with_last_byte(9);

    #[test]
    fn wallet_trade_supply_resolves_exact_units_and_a_linear_chain() {
        let resolver = resolver(None);
        let intent = intent(vec![
            wallet("USDT", "20"),
            StrategyStep::Trade {
                buy_symbol: "Tesla".to_owned(),
            },
            StrategyStep::Supply,
        ]);

        let Resolution::Recipe(recipe) = resolver.resolve(&intent) else {
            panic!("a complete wallet strategy must resolve");
        };

        assert_eq!(
            recipe.nodes[0].primitive,
            RecipePrimitive::WalletAsset {
                token: USDT,
                raw_amount: U256::from(20_u8) * U256::from(10_u8).pow(U256::from(18_u8)),
            }
        );
        assert_eq!(
            recipe.nodes[1].primitive,
            RecipePrimitive::Trade { buy_token: TSLAB }
        );
        let ports = recipe
            .edges
            .iter()
            .map(|edge| (edge.from.port.as_str(), edge.to.port.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(ports, [("asset", "sell"), ("buy", "asset")]);
        assert_eq!(step_index(recipe.nodes[2].id), Some(2));
    }

    #[test]
    fn amounts_finer_than_token_decimals_are_rejected_not_rounded() {
        let mut resolver = resolver(None);
        resolver.tokens[0].decimals = 6;

        let resolution = resolver.resolve(&intent(vec![wallet("USDT", "1.0000001")]));

        assert!(matches!(resolution, Resolution::Rejected(_)));
    }

    #[test]
    fn missing_amounts_ask_the_user_instead_of_guessing() {
        let resolution = resolver(None).resolve(&intent(vec![StrategyStep::WalletAsset {
            symbol: "USDT".to_owned(),
            amount: None,
        }]));

        assert!(matches!(resolution, Resolution::NeedsInput(_)));
    }

    #[test]
    fn unknown_assets_and_broken_ordering_are_returned_for_repair() {
        let resolver = resolver(None);

        let unknown = resolver.resolve(&intent(vec![wallet("DOGE", "1")]));
        let action_first = resolver.resolve(&intent(vec![StrategyStep::Supply]));
        let after_terminal = resolver.resolve(&intent(vec![
            wallet("TSLAB", "1"),
            StrategyStep::Supply,
            StrategyStep::Withdraw,
        ]));

        assert!(matches!(unknown, Resolution::Rejected(_)));
        assert!(matches!(action_first, Resolution::Rejected(_)));
        assert!(matches!(after_terminal, Resolution::Rejected(_)));
    }

    #[test]
    fn supplied_positions_come_only_from_the_connected_wallets_live_account() {
        let borrow = intent(vec![
            StrategyStep::SuppliedPosition {
                symbol: "TSLAB".to_owned(),
            },
            StrategyStep::Borrow {
                symbol: "USDT".to_owned(),
                amount: Some(Decimal::ONE),
            },
        ]);

        let disconnected = resolver(None).resolve(&borrow);
        let empty_account = account(Vec::new());
        let not_held = resolver(Some(&empty_account)).resolve(&borrow);
        let held_account = account(vec![supplied(TSLAB, "TSLAB")]);
        let held = resolver(Some(&held_account)).resolve(&borrow);

        assert!(matches!(disconnected, Resolution::NeedsInput(_)));
        assert!(matches!(not_held, Resolution::NeedsInput(_)));
        let Resolution::Recipe(recipe) = held else {
            panic!("a held supplied position must resolve");
        };
        assert_eq!(
            recipe.nodes[0].primitive,
            RecipePrimitive::SuppliedPosition {
                venue: BorrowingVenue::Venus,
                pool: POOL,
                token: TSLAB,
            }
        );
    }

    #[test]
    fn stocks_cannot_be_borrowed() {
        let held_account = account(vec![supplied(TSLAB, "TSLAB")]);
        let resolution = resolver(Some(&held_account)).resolve(&intent(vec![
            StrategyStep::SuppliedPosition {
                symbol: "TSLAB".to_owned(),
            },
            StrategyStep::Borrow {
                symbol: "TSLAB".to_owned(),
                amount: Some(Decimal::ONE),
            },
        ]));

        assert!(matches!(resolution, Resolution::Rejected(_)));
    }

    fn resolver(account: Option<&BorrowingAccount>) -> Resolver {
        Resolver::new(
            &[tesla()],
            &[SettlementTokenMetadata {
                address: USDT,
                symbol: "USDT".to_owned(),
                decimals: 18,
            }],
            &[USDT],
            Some(POOL),
            account,
        )
    }

    fn intent(steps: Vec<StrategyStep>) -> StrategyIntent {
        StrategyIntent { steps }
    }

    fn wallet(symbol: &str, amount: &str) -> StrategyStep {
        StrategyStep::WalletAsset {
            symbol: symbol.to_owned(),
            amount: Some(
                amount
                    .parse()
                    .unwrap_or_else(|error| panic!("test amount is invalid: {error}")),
            ),
        }
    }

    fn account(supplied: Vec<BorrowingAsset>) -> BorrowingAccount {
        BorrowingAccount {
            venue: BorrowingVenue::Venus,
            chain_id: BNB_CHAIN_ID,
            positions: vec![BorrowingPosition {
                pool: POOL,
                supplied,
                borrowed: Vec::new(),
            }],
            total_supplied_usd: Decimal::ZERO,
            total_borrowed_usd: Decimal::ZERO,
            health_factor: None,
            observed_at_ms: 0,
        }
    }

    fn supplied(contract: Address, symbol: &str) -> BorrowingAsset {
        BorrowingAsset {
            token: BorrowingToken {
                contract,
                symbol: symbol.to_owned(),
            },
            amount: Decimal::ONE,
            value_usd: Decimal::ONE,
        }
    }

    fn tesla() -> BstockSummary {
        BstockSummary {
            id: BstockId {
                chain_id: BNB_CHAIN_ID,
                contract: TSLAB,
            },
            token_name: "Tesla (bStocks)".to_owned(),
            token_symbol: "TSLAB".to_owned(),
            token_logo_url: None,
            token_decimals: 18,
            underlying_ticker: "TSLA".to_owned(),
            underlying_name: "Tesla".to_owned(),
            asset_kind: AssetKind::Stock,
            token_to_share_ratio: Decimal::ONE,
            token_price_usd: None,
            reference_price_usd: None,
            volume_24h_usd: None,
            market_cap_usd: None,
            market_status: None,
            tags: Vec::new(),
        }
    }
}
