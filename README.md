# Stockfront

**Your AI prime broker for tokenized stocks.**

Users simply specify their financial intent, and Stockfront automatically recommends and executes optimal strategies that unlock yield, leverage, and liquidity from their stock portfolio.

- **Intent-Based Execution.** Users specify a goal and risk tolerance — *"the safest way to earn yield"* — and Stockfront recommends and prepares the optimal strategy, sized to their holdings.
- **Institutional Strategies, Onchain.** Carry, leverage, and index exposure, each with its live rate displayed before execution.
- **Transparent Risk.** Health factor before and after every borrow, liquidation distance, and named risk per strategy.
- **Verified Execution.** Every strategy is compiled, validated against live chain state, and user-signed.

---

## Problem

- **Massive idle equity capital.** ~$100T in listed equities, with the retail share earning nothing beyond price appreciation.
- **Prime brokerage is costly.** Retail margin runs mid-to-high single digits; leveraged ETFs layer an annual fee on top of financing.
- **Onchain, the research burden falls on the user.** Comparing rates across separate protocol UIs, computing spreads by hand, signing at each venue, then unwinding the same way.
- **Risk is invisible.** The number that decides whether a position survives a drawdown sits in a protocol dashboard, if it is shown at all.

The primitives are live. The strategy layer is missing.

---

## Solution: Stockfront

Stockfront is that layer. It reads every venue, composes multi-step strategies across them, prices the risk, and prepares each step against live chain state at the moment of execution.

---

## Product Stack

**Intent-Based Composition.** Users state a goal and risk tolerance; Stockfront selects the strategy, picks the assets, sizes the amounts from the connected wallet, and renders every step for review. It resolves holdings — *"sell all my Tesla"* becomes an exact balance — and declines what the wallet cannot do rather than drafting something that cannot settle.

| Layer | Capability | What it does |
| --- | --- | --- |
| **Earn** | USD Carry | Collateralize a stock, borrow a stablecoin, deploy into a reviewed earn product, keep the spread |
| | Leveraged Stock | Buy, post, borrow, buy more, post again — each turn adds exposure beyond the cash deposited |
| | Index Baskets | One token holding an entire index, backed by its assets and rebalanced onchain |
| **Borrow** | Floating-Rate Borrowing | Stablecoins against tokenized stock collateral, with onchain health verification and venue-sized limits |
| **Trade** | Spot, aggregated | Stocks, stablecoin pairs and index baskets routed across venues, price impact per quote |
| **Risk** | Position health | Health factor before and after every borrow, three-zone scale against a hard floor |
| | Named risk per strategy | The specific thing that can cost money, not a score that hides which |
| | Policy gates | Reviewed-only deposits, venue-sized borrows, minimum received on every quote |
| **Exit** | Prepared exits | Repay, withdraw, redeem, sell — available even when an entry is no longer offered |

*All live today. Perpetuals, options and concentrated liquidity are on the roadmap.*

---

## How It Works

| | Strategy | Return | How it works |
| --- | --- | --- | --- |
| **Live** | USD Carry | **1.11% net** · 11.22% best observed | Yield on stablecoins borrowed against a stock you keep. 9 earn products ranked on every composition; routes to the best as they clear review |
| | Leveraged Stock | **1.60× exposure** | Per run, compounding each repeat. Every borrow sized at the health floor, so an overextended loop is refused rather than liquidated |
| | Index Baskets | **+6.99% past 30d** | CoinMarketCap 20, per Reserve. 6 baskets across AI infrastructure, power, photonics, neocloud and robotics — 8 to 25 holdings each |
| **API live**<br>composer soon | Managed Portfolios | **up to +107% 6m** | 20 curated portfolios via Glider — Magnificent Seven, investor trackers, thematic strategies. Performance per Glider; enrol, fund, rebalance, withdraw |
| | Concentrated Liquidity | **up to 41.80% APR** | PancakeSwap V3 stock pairs, 17 pools live. Fee income with impermanent-loss exposure; opportunities, entry, fee claims, withdrawal |
| **Upcoming** | Basis Trade | — | Delta-neutral funding capture, once a perpetual venue is adapted |
| | Covered Calls | — | Yield from selling upside on held equities |
| | Collars | — | Zero-cost, no-liquidation borrowing against protected downside |

---

## The engine: why a compiler, not a prompt

Writing a strategy takes a model one second. Running it takes minutes and several signatures, with a chain moving in between. Quotes expire. The collateral step four borrows against appears at step three. The safe borrow size depends on both. No better model fixes that — when the model runs, those answers do not exist yet.

So it is asked only for the part that does not expire: **the intent.**

```
plain language · canvas blocks · JSON
    ↓  front end      one typed graph, 11 primitives
  recipe
    ↓  type checker   4 value types · 28 located diagnostics
  logical plan        ordered, provider-independent
    ↓  back end       emitted per step against live chain state
  transactions        reviewed and signed
```

Addresses, amounts and edges come from live data, not the model. The type checker rejects a position fed to a step expecting tokens, an output spent twice, or a cycle.

### The grammar

| Sources | Actions |
| --- | --- |
| `WALLET_ASSET` → tokens | `TRADE` tokens → tokens |
| `SUPPLIED_POSITION` → collateral | `SUPPLY` tokens → collateral |
| `BORROWED_POSITION` → debt | `BORROW` collateral → tokens |
| `EARN_POSITION` → deposit | `WITHDRAW` collateral → wallet |
| | `REPAY` debt → settled |
| | `EARN_DEPOSIT` tokens → deposit |
| | `EARN_WITHDRAW` deposit → wallet |

`TRADE` moves between three asset classes — stocks, stablecoins and index baskets — set by a policy table, not by the grammar.

### Composable by construction

Nothing in the grammar names a protocol. Venues sit behind ports, and that policy table is deployment configuration: index baskets were added as a third asset class with **no new primitive**.

---

## Built on Binance Web3

Stockfront builds the strategy layer; Binance's infrastructure carries everything beneath it.

| Capability | What it provides |
| --- | --- |
| RWA registry and market data | Asset catalog, issuer metadata, live and reference pricing, market status |
| Token balances | Any token by chain and contract — one call for stocks, stablecoins and baskets |
| Trading and quotes | Aggregated routing, price impact, executable quotes |
| Transaction construction | Ready-to-sign approvals and swaps with gas fields |
| DeFi discovery and positions | Earn and liquidity opportunities, APYs, TVL, open positions |
| DeFi transaction building | Deposit, redeem and claim |

Numbers that decide solvency — collateral factors, liquidation thresholds, oracle validity, health — are read onchain, never from an index.

---

## Why this compounds

**Discovery.** Nine earn products, four lending markets, six baskets and forty-six stocks ranked on every composition. Nobody reads four UIs.

**Beachhead:** crypto-native holders of tokenized equities — served by nothing that composes across venues. **Expansion:** equity holders arriving onchain, where the strategy layer is what makes a tokenized stock worth more than the share.

**Next:** automated market making and trading on the same grammar — strategies that run continuously rather than once per signature.

---

## Future

- Perpetual and options venues, unlocking basis capture, covered calls and zero-cost collars.
- Concentrated liquidity as a first-class primitive — the venue integration is already complete behind the API.
- Pre-flight simulation of every prepared batch, so a venue that quotes but cannot fill is reported in words before a signature is requested.
- An MCP server over the strategy API, so external agents compose against the same compiler.
- Corporate-action accounting across dividends, splits, rebases and multipliers.
- Additional chains and venues as adapters, with no change to the language.

---

## License

MIT
