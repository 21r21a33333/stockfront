# Stockfront

**More from every stock.**

Stockfront is a non-custodial portfolio and income layer for tokenized stocks on BNB Chain. It helps users discover and purchase bStocks, borrow without selling them, enter a conservative automated carry vault, provide verified stock liquidity, and manage risk from one place.

Instead of creating another exchange, lending market, or oracle, Stockfront composes established BNB Chain infrastructure and adds the strategy orchestration, accounting, automation, monitoring, and controlled exits needed to make tokenized stocks useful in DeFi.

## The problem

Tokenized stocks provide onchain price exposure, but the surrounding user experience remains fragmented. Users must separately discover assets, evaluate backing, find liquidity, borrow, track changing collateral values, account for dividends or rebases, and unwind positions safely.

Stockfront turns those disconnected primitives into understandable financial workflows while keeping users in control of their assets.

## Product

### Discover and own bStocks

- Search and compare supported tokenized stocks.
- Inspect issuer, market, pricing, liquidity, and collateral information.
- Obtain an aggregated trade quote and prepare a wallet-signed purchase or sale.
- Track bStocks and their effective underlying-share exposure in one portfolio.

This is the trusted entry and accounting layer used by every Stockfront strategy.

### Borrow without selling

Users can supply supported bStocks as collateral and borrow an approved stablecoin while retaining their stock and dividend exposure. Stockfront monitors the position and prepares user-signed actions to repay debt, add collateral, or exit.

### Earn through a Conservative Carry Vault

The first pooled strategy is intentionally narrow:

```text
Deposit NVDAB
→ supply it as collateral on Lista
→ borrow a conservative amount of USDT
→ lend USDT in one approved Aave V3 BNB market
→ compound the positive interest-rate spread
```

The return combines the user's stock exposure with the spread between stablecoin lending yield and borrowing cost. The vault borrows only when expected lending yield exceeds borrowing cost, fees, and a safety margin.

The vault is capped, transparent, and designed around conservative leverage, liquidity reserves, contract-limited automation, and emergency unwinding. It is not principal-protected and does not promise guaranteed returns.

### Provide managed stock liquidity

Stockfront identifies verified bStock/stablecoin pools and helps users create and manage concentrated-liquidity positions. It can balance assets, select a range, compound fees, monitor whether liquidity remains active, and prepare rebalancing or withdrawal actions.

A pool must pass depth, exit-liquidity, and price-impact checks before deposits are enabled. Otherwise, Stockfront exposes monitoring and simulated entry/exit instead of presenting unsafe liquidity as a usable strategy.

### Monitor risk and exit safely

The portfolio and risk layer brings together:

- Position value and realized return.
- Borrowing cost, net carry, health factor, and liquidation proximity.
- Liquidity range, earned fees, impermanent loss, and available exit liquidity.
- Stock/reference-price deviations.
- Dividend, split, rebase, and multiplier-adjusted accounting.

Alerts prepare concrete actions such as repaying debt, adding collateral, reducing leverage, rebalancing liquidity, or exiting. Users approve and sign actions for personal positions; pooled-vault keepers operate only within contract-enforced limits.

## MVP

The initial product is deliberately focused:

1. bStock discovery, wallet portfolio, trade quotes, and wallet-signed execution.
2. Borrowing against supported bStocks with direct onchain health verification.
3. One capped Conservative Carry Vault using NVDAB, Lista, USDT, and one Aave market.
4. One verified PancakeSwap liquidity flow—or a transparent simulator when no pool passes the safety gate.
5. Live carry, collateral-health, liquidity-range, and exit-liquidity monitoring.
6. User-signed safety and exit actions.
7. Transparent vault assets, debt, multiplier, NAV, and share accounting.

Only the Conservative Carry Vault is a custom pooled vault in the MVP.

## Strategy roadmap

After the core accounting and risk system is proven, Stockfront can extend into:

- Leveraged stock exposure with automatic deleveraging.
- Delta-neutral stock basis using an equivalent perpetual hedge.
- Delta-hedged liquidity provision.
- Liquidity between approved representations of the same stock.
- Leveraged liquidity provision.
- Same-stock price-convergence monitoring and transaction preparation.
- Automatically rebalanced thematic and index-style stock baskets.

Cross-chain execution, options strategies, payment cards, permissionless strategy creation, and guaranteed-return products are outside the current scope.

## Protocol stack

| Role | Protocol or service |
| --- | --- |
| Tokenized stocks | bStocks |
| Asset registry and market data | Binance RWA API |
| Wallet and DeFi discovery | Binance Wallet and DeFi APIs |
| Quotes and transaction construction | Binance Trading and Transaction APIs |
| Primary collateral and borrowing | Lista |
| Secondary borrowing venue | Venus |
| Initial stablecoin yield destination | Aave V3 on BNB Chain |
| Swaps and liquidity | PancakeSwap V3 / Infinity |
| Post-MVP perpetual hedging | Aster |
| Collateral pricing | Venus Resilient Oracle |
| Vault accounting | OpenZeppelin ERC-4626 |
| Monitoring and constrained automation | OpenZeppelin Monitor / Relayer |
| Emergency administration | Safe multisig |

GoldRush may provide historical backfill when primary API coverage is insufficient. DeBank may be used for portfolio reconciliation, but never as the source of truth for liquidation-sensitive decisions.

## Safety principles

- Non-custodial personal positions and user-signed transactions.
- Conservative loan-to-value targets below liquidation thresholds.
- Hard deposit and debt caps.
- No new borrowing when expected carry becomes negative.
- No deposits when prices are stale or oracle protection is active.
- Explicit withdrawal liquidity and emergency-unwind paths.
- Sustainable protocol yield separated from temporary incentives.
- Correct accounting for dividends, rebases, splits, and market closures.
- Repayment of strategy debt before collateral is released during an unwind.
- Security review before vault caps are raised.
- Keepers cannot transfer funds to arbitrary addresses.

## Architecture principles

Stockfront is organized as vertical product slices using hexagonal architecture. The domain owns financial rules and ports; external protocols, HTTP, persistence, and chain access remain adapters. This keeps strategy logic testable and prevents any one provider's data model from becoming the product model.

Contributors and coding agents must read [AGENTS.md](AGENTS.md) before making changes. It defines the architecture boundaries, integration policy, testing standards, generated-code ownership, and secret-handling requirements.

## License

MIT
