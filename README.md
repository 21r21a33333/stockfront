# Stockfront

**More from every stock.**

Stockfront is a non-custodial portfolio and income layer for tokenized stocks on BNB Chain. It helps users discover and trade bStocks, borrow without selling them, deploy borrowed stablecoins into reviewed earn opportunities, provide stock liquidity, and manage risk from one place.

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

Users can supply supported bStocks to Venus, enable them as collateral, and borrow an approved stablecoin while retaining their stock exposure. Stockfront exposes the complete loan position and prepares wallet-signed supply, collateral, borrow, repay, and withdrawal actions with health-factor previews.

### Earn through conservative carry

The first carry strategy composes existing protocols instead of introducing another custody or vault contract:

```text
Supply a supported bStock to Venus
→ enable it as collateral
→ borrow an approved stablecoin conservatively
→ deposit that stablecoin into a reviewed BNB Chain earn opportunity
→ redeem the earn position
→ repay the debt before withdrawing the bStock
```

Stockfront discovers earn products through Binance Web3, compares sustainable base APY with the matching Venus borrow APY, and enables deposits only for explicitly reviewed investments whose spread clears the configured safety margin. The user signs every transaction and retains the resulting protocol position tokens.

Returns remain exposed to variable rates, stablecoin depegging, protocol risk, transaction costs, and bStock collateral liquidation. They are not guaranteed or principal-protected.

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

Alerts prepare concrete actions such as repaying debt, adding collateral, reducing leverage, rebalancing liquidity, or exiting. Users approve and sign every action.

## MVP

The initial product is deliberately focused:

1. bStock discovery, wallet portfolio, trade quotes, and wallet-signed execution.
2. Borrowing against supported bStocks through Venus with direct onchain health verification.
3. Conservative carry using supported bStock collateral, Venus borrowing, and reviewed existing earn products such as Lista.
4. One verified PancakeSwap liquidity flow—or a transparent simulator when no pool passes the safety gate.
5. Live carry, collateral-health, liquidity-range, and exit-liquidity monitoring.
6. User-signed safety and exit actions.
7. Transparent stock exposure, debt, carry positions, rates, and prepared unwind actions.

The MVP does not deploy a custom pooled vault. It composes existing protocol positions in the user's wallet.

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
| Direct bStock collateral and borrowing | Venus |
| Carry opportunity discovery and execution | Binance Web3 DeFi APIs |
| Reviewed initial earn venue | Lista DAO |
| Swaps and liquidity | PancakeSwap V3 / Infinity |
| Post-MVP perpetual hedging | Aster |
| Collateral pricing | Venus Resilient Oracle |
| Future monitoring and prepared actions | Stockfront risk services |

GoldRush may provide historical backfill when primary API coverage is insufficient. DeBank may be used for portfolio reconciliation, but never as the source of truth for liquidation-sensitive decisions.

## Safety principles

- Non-custodial personal positions and user-signed transactions.
- Conservative loan-to-value targets below liquidation thresholds.
- Respect protocol supply, borrow, and liquidity limits.
- Allow deposits only into explicitly reviewed earn investments.
- No new borrowing when expected carry becomes negative.
- No deposits when prices are stale or oracle protection is active.
- Explicit withdrawal liquidity and emergency-unwind paths.
- Sustainable protocol yield separated from temporary incentives.
- Correct accounting for dividends, rebases, splits, and market closures.
- Repayment of strategy debt before collateral is released during an unwind.
- Repayment and withdrawal plans use the wallet's live indexed positions.

## Architecture principles

Stockfront is organized as vertical product slices using hexagonal architecture. The domain owns financial rules and ports; external protocols, HTTP, persistence, and chain access remain adapters. This keeps strategy logic testable and prevents any one provider's data model from becoming the product model.

Contributors and coding agents must read [AGENTS.md](AGENTS.md) before making changes. It defines the architecture boundaries, integration policy, testing standards, generated-code ownership, and secret-handling requirements.

## License

MIT
