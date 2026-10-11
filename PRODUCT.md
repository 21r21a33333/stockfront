# Stockfront

**Onchain prime brokerage for tokenized equities, driven by intent.**

Figures below are live readings from the running deployment on BNB Chain, taken 2026-10-11. Rates move.

---

## Overview

Stockfront lets holders of tokenized stocks unlock yield, leverage and liquidity from a portfolio they never have to sell.

- **One interface** across earn, borrow, trade and exit against tokenized equities and index baskets.
- **Users state a financial intent** — *"earn on my idle USDT"*, *"the best APY, I can take risk"* — and Stockfront composes the strategy, sizes it against live venue limits, and walks the user through signing it.
- **Advanced strategies for anyone with a wallet.** No minimum, no account approval, no desk.

---

## Problem

- **Idle equity capital at enormous scale.** Global listed equities are on the order of $100T, and the retail share of that sits in brokerage accounts earning nothing beyond price appreciation.
- **Prime brokerage is gated.** Portfolio margin, structured products and carry desks are institutional products behind capital minimums, account approval and jurisdictional limits.
- **Even where retail can borrow, it is expensive.** Retail margin sits in the mid-to-high single digits and above; leveraged ETFs layer an annual fee on top of financing.
- **Onchain, the opportunity exists but is unusable.** Tokenized stocks already trade, lend and earn across separate venues. Finding the best route means reading four protocol UIs, computing a spread by hand, and signing a different transaction in each — then unwinding it the same way.

The primitives are live. The strategy layer is missing.

---

## Solution: Stockfront

Stockfront is the strategy layer. It reads every venue, composes multi-step strategies across them, type-checks the result before anything is signed, and sizes each step against live chain state at the moment of execution.

The user describes an outcome. Stockfront produces the transactions.

---

## Product Stack

### Intent-Based Composition

State a goal, an outlook and a risk tolerance in plain language. Stockfront selects the strategy, picks the assets, sizes the amounts from the connected wallet, and renders the full step list for review before a signature is requested.

It resolves holdings, so *"sell all my Tesla"* becomes an exact balance rather than a question. It reads what each index basket contains, so *"AI infrastructure"* or *"something diversified"* finds the right one out of the catalog. It declines what the wallet cannot do rather than drafting something that will fail.

### Earn

**USD Carry** — *Live.*
Collateralize a tokenized stock, borrow USDT against it, deploy into a reviewed earn product and keep the spread. Stock exposure is retained throughout. The engine compares every discovered earn product against the live borrow rate and deposits only into reviewed venues.

**Leveraged Stock** — *Live.*
Buy a stock, post it, borrow against it and buy more. Each turn adds exposure beyond the cash deposited. Loop depth is explicit in the strategy and every borrow is sized by the venue at a health-factor floor.

**Index Baskets** — *Live.*
One token holding an entire index, backed by its assets and rebalanced onchain. No leverage, no debt, nothing to liquidate.

**Basis and options strategies** — *Roadmap.* Delta-neutral funding capture, covered calls and collars become available as perpetual and options venues are adapted.

### Borrow: liquidity without selling

**Floating-Rate Borrowing** — *Live.*
Borrow stablecoins against tokenized stock collateral with onchain health verification. Collateral factors, liquidation thresholds, oracle validity and health factors are read directly from the lending market.

**Venue-Sized Maximums** — *Live.*
Ask for the most you can safely borrow and no number is written into the strategy. The venue states its own limit once the collateral exists, and that figure is what the user reviews.

### Trade

**Spot, aggregated** — *Live.* Buy and sell tokenized stocks, stablecoin pairs and index baskets with routing across venues and price-impact figures on every quote.

**Batched execution** — *Live.* Approvals and actions are combined into a single wallet prompt where EIP-5792 allows.

**Perpetuals and hedging** — *Roadmap.*

### Exit

Every position carries a prepared exit: repay, withdraw, redeem, sell. Exits stay available even when an entry is no longer offered, so a policy change can never trap a position.

---

## How It Works

### Earn — USD Carry

**+1.11% net spread, live, on reviewed venues.**

Collateralize a tokenized stock, borrow USDT at **5.12% APY**, and deploy into the reviewed Lista USDT product at **6.23% APY**. The strategy holds a **1.5 health-factor floor** and the borrow is sized by the venue, not estimated.

The engine ranks **nine discovered earn products** on every composition and reports what it found. Observed rates on the same asset currently span **2.18% to 11.22%** across Plume, Lista, Venus, Venus Flux and Aave V3 — the top of that range sits above the reviewed allowlist, so the executable spread is the conservative one. As products clear review, the same strategy routes to them with no change to the user's intent.

Borrow liquidity available: **$40.9M USDT**.

### Earn — Leveraged Stock

**1.60× exposure per run** at a 0.60 collateral factor.

Buy the stock, supply it, borrow against it, buy more, supply again — one strategy, one review, one signing session. Every borrow is gated on the venue confirming the amount holds the health floor, so an overextended loop is refused rather than liquidated.

Collateral terms live today: **0.60 CF / 0.70 LT** on majors, **0.50 CF / 0.65 LT** on the rest.

### Earn — Index Baskets

**6 baskets, 8 to 25 holdings each, 0.30%–0.60% annual fee.**

A broad market-cap index at **0.30%** and thematic onchain folios at **0.60%**, covering AI infrastructure, AI power, photonics, neocloud capacity and robotics. Bought and sold through the same trade block as any other asset.

### Borrow — Floating Rate

**5.12% APY on USDT** against tokenized stock collateral, with health factor computed from direct onchain reads rather than an index.

### Trade — Spot

**46 tokenized stocks, 6 index baskets, stablecoin pairs**, routed across venues with price impact reported per quote and a minimum-received figure on every trade.

---

## The engine underneath

What makes intent-based execution safe is that the language model never touches the money.

The model proposes an **intent over symbols**. It cannot emit a contract address, a raw token amount, or a connection between steps — those are derived deterministically from live catalog and account data. The result is a **typed directed graph** across **11 primitives** and **4 value types**, checked by a compiler that reports **28 distinct located diagnostics** before anything reaches a wallet.

Execution is **state-gated**: each call prepares only the steps that are ready right now, verified against the chain. A borrow is prepared only once its collateral is genuinely posted and the venue confirms the size. Runs survive a refresh and resume from wherever the chain actually is.

---

## Why this compounds

**Discovery layer.** Nine earn products, four lending markets, six index baskets and forty-six stocks are ranked and compared on every composition. No user reads four UIs.

**Automated strategies.** Multi-step carry and leverage become a single reviewed run instead of six manual transactions across three protocols.

**Composable positions.** Collateral, debt, earn deposits and basket holdings are all first-class values in the language, so the output of one strategy is the input to the next.

**Venue-agnostic by construction.** The grammar names no protocol. `TRADE` means *exchange one token for another at whichever venue quotes it best*; `SUPPLY` means *post an asset as collateral at a lending venue*. Venues sit behind ports and what may be traded for what is a policy table the deployment supplies — index baskets arrived as a third asset class with **no new primitive**.

**Built on Binance Web3.** Asset registry and market data, token balances, aggregated quotes, transaction construction, DeFi discovery and DeFi transaction building. Reusing that infrastructure is what let the whole strategy engine fit in one repository.

---

## Market

**Beachhead: crypto-native holders of tokenized equities.** Capital already onchain, already familiar with leverage and carry, already willing to adopt new primitives — and currently served by nothing that composes across venues.

**Expansion: equity holders arriving onchain.** As tokenized equity issuance grows, the strategy layer is what makes holding them worth more than holding the underlying.

**Monetization** spans execution, borrowing and structured strategies, with retention driven by cross-product usage rather than a single flow.

---

## Future

- Perpetual and options venues, unlocking basis capture, covered calls and zero-cost collars.
- Concentrated liquidity as a first-class primitive — the venue integration is already complete behind the API.
- Pre-flight simulation of every prepared batch, so a venue that quotes but cannot fill is reported in words before a signature is requested.
- An MCP server over the strategy API, so external agents compose against the same compiler.
- Corporate-action accounting across dividends, splits, rebases and multipliers.
- Additional chains and venues as adapters, with no change to the language.

---

Returns are exposed to variable rates, protocol risk, price movement, liquidation and transaction costs. Nothing here is principal-protected, and every transaction is signed by the user.
