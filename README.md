# Stockfront

**Your AI prime broker for tokenized stocks.**

Users simply specify their financial intent, and Stockfront automatically recommends and executes optimal strategies that unlock yield, leverage, and liquidity from their stock portfolio.

- **An AI that does the work.** State an outcome — *"the safest way to earn yield"*, *"sell all my Tesla and buy an AI index"*, *"the best APY, I can take risk"* — and Stockfront chooses the strategy, the assets and the amounts from what the wallet actually holds, explains what it assumed, and says what it will not do.
- **Prime brokerage strategies underneath it.** Carry, leverage and index exposure are shapes institutions have run for decades, each shipping with its **live rate on the card** so the return is known before anything is signed.
- **Risk measured, not buried.** Health factor before and after every borrow, distance to liquidation, and the specific thing that can cost money named on every strategy.
- **Checked before it is signed.** Every strategy goes through the same compiler and is prepared against live chain state. Every transaction is signed by the user, and nothing runs on its own.

*Live readings throughout are from the running deployment on BNB Chain, 2026-10-11. Rates move; the engine re-reads them on every composition.*

---

## Problem

- **Idle equity capital at enormous scale.** Global listed equities run to roughly $100T, and the retail share sits in brokerage accounts earning nothing beyond price appreciation.
- **Prime brokerage is gated.** Portfolio margin, structured products and carry desks are institutional products behind capital minimums, account approval and jurisdictional limits.
- **Where retail can borrow, it is expensive.** Margin sits in the mid-to-high single digits and above; leveraged ETFs layer an annual fee on top of financing.
- **Onchain the opportunity exists but is unusable.** Tokenized stocks already trade, lend and earn across separate venues. Finding the best route means reading four protocol UIs, computing a spread by hand, and signing a different transaction at each — then unwinding the same way.
- **And nobody tells you the risk.** The number that decides whether a position survives a drawdown is buried in a protocol dashboard, if it is shown at all.

The primitives are live. The strategy layer is missing.

---

## Solution: Stockfront

Stockfront is that layer. It reads every venue, composes multi-step strategies across them, type-checks the result before anything is signed, sizes each step against live chain state at the moment of execution, and shows the risk at every step.

You describe an outcome. Stockfront produces the transactions.

---

## Product Stack

### Intent-Based Composition

State a goal, an outlook and a risk tolerance in plain language. Stockfront selects the strategy, picks the assets, sizes the amounts from the connected wallet, and renders the full step list for review before a signature is requested.

It resolves holdings, so *"sell all my Tesla"* becomes an exact balance rather than a question. It reads what each index basket contains, so *"AI infrastructure"* or *"something diversified"* finds the right one. And it declines what the wallet cannot do — *"sell my Apple stock"* answers that you do not hold any, rather than drafting something that cannot settle.

### Earn

**USD Carry** · *live* — collateralize a tokenized stock, borrow a stablecoin, deploy into a reviewed earn product, keep the spread. Stock exposure retained throughout.

**Leveraged Stock** · *live* — buy, post, borrow, buy more, post again. Each turn adds exposure beyond the cash deposited, every borrow sized by the venue at the health floor.

**Index Baskets** · *live* — one token holding an entire index, backed by its assets and rebalanced onchain. No debt, nothing to liquidate.

**Basis trades, covered calls, collars** · *roadmap* — unlocked as perpetual and options venues are adapted.

### Borrow

**Floating-Rate Borrowing** · *live* — borrow stablecoins against tokenized stock collateral with onchain health verification.

**Venue-Sized Maximums** · *live* — ask for the most you can safely borrow and no number is written into the strategy. The venue states its own limit once the collateral exists, and that figure is what you review.

### Trade

**Spot, aggregated** · *live* — tokenized stocks, stablecoin pairs and index baskets, routed across venues with price impact on every quote.

**Batched execution** · *live* — approvals and actions combined into a single wallet prompt where EIP-5792 allows.

**Perpetuals and hedging** · *roadmap*

### Risk

**Position health** · *live* — health factor before and after every borrow, on a three-zone scale against a hard floor.

**Named risk per strategy** · *live* — each strategy states the specific thing that can cost money, rather than a score that hides which.

**Policy gates** · *live* — reviewed-only deposits, venue-sized borrows, price impact and minimum received on every quote.

### Exit

Every position carries a prepared exit — repay, withdraw, redeem, sell — and exits stay available even when an entry is no longer offered, so a policy change cannot trap a position.

---

## How It Works

### Earn — USD Carry

**+1.11% net spread, live, on reviewed venues.**

Collateralize a tokenized stock, borrow USDT at **5.12% APY**, deploy into the reviewed Lista USDT product at **6.23% APY**. The strategy holds a **1.5 health-factor floor** and the borrow is sized by the venue, not estimated.

The engine ranks **9 discovered earn products** on every composition. Observed rates on the same asset currently span **2.18% to 11.22%** across Plume, Lista, Venus, Venus Flux and Aave V3 — the top of that range sits above the reviewed allowlist, so the executable spread is the conservative one. As products clear review the same strategy routes to them, with no change to the user's intent.

Borrow liquidity available: **$40.9M USDT**.

### Earn — Leveraged Stock

**1.60× exposure per run** at a 0.60 collateral factor.

Buy the stock, supply it, borrow against it, buy more, supply again — one strategy, one review, one signing session. Repeating the run compounds the exposure. Every borrow is gated on the venue confirming the amount holds the health floor, so an overextended loop is refused rather than liquidated.

Collateral terms today: **0.60 CF / 0.70 LT** on majors, **0.50 CF / 0.65 LT** on the rest.

### Earn — Index Baskets

**6 baskets, 8 to 25 holdings each, 0.30%–0.60% annual fee.**

A broad market-cap index at **0.30%** across **17 holdings**, and five thematic onchain folios at **0.60%** covering AI infrastructure, AI power, photonics, neocloud capacity and robotics. Bought and sold through the same trade block as any other asset.

### Borrow — Floating Rate

**5.12% APY on USDT** against tokenized stock collateral, with health factor computed from direct onchain reads rather than an index.

### Risk — what is actually checked

Health factor **before and after** every borrow, scored on a three-zone scale against a **1.5 floor**. Collateral factors, liquidation thresholds and oracle validity read directly from the lending market, so a safety check never depends on a stale feed. Borrow limits sized by the venue. Deposits restricted to reviewed products. Price impact and a minimum received on every quote. A venue that cannot serve a step reports why, by name.

### Trade — Spot

**46 tokenized stocks, 6 index baskets, stablecoin pairs**, routed across venues with price impact reported per quote and a minimum-received figure on every trade.

---

## The engine: why a compiler

A language model can write a plausible DeFi strategy in a second. The distance between *plausible* and *executable* is where money is lost.

A strategy that reads perfectly can still name an asset the venue has never listed, state an amount with more decimal places than the token has, feed a lending position into a step that expects tokens, spend the same output twice, borrow past a safe health factor, or quietly form a loop. None of that is visible in prose. All of it is visible to a type system.

So the model never touches the money. It proposes an **intent** over symbols — *"borrow USDT against TSLAB"* — and nothing else. It cannot emit a contract address, a raw token amount, or a connection between steps. Those are derived afterwards, deterministically, from live catalog and account data.

That intent becomes a **recipe**: a typed directed graph. Every block has ports, every port carries a value type, and an edge is legal only where an output type meets a matching input type.

```
TOKEN_AMOUNT        tokens that can be spent
SUPPLIED_POSITION   collateral that can be borrowed against or withdrawn
BORROWED_POSITION   debt that can be repaid
EARN_POSITION       a deposit that can be redeemed
```

The compiler type-checks the whole graph and reports **28 distinct diagnostics** — unknown ports, mismatched types, missing inputs, reused outputs, cycles, disconnected blocks, unsupported trade pairs, collateral a venue will not take, amounts that cannot be expressed. Each points at the exact node, port or field responsible, so a rejected draft is repairable rather than merely refused.

**This is what makes an open-ended AI agent safe to hand a signature to.** Without it you get one of two products: a chatbot whose output you have to trust, or a menu of a few hardcoded flows. The compiler gives you the open-ended language *and* the guarantee.

### From a sentence to a signature

```
  prompt
    ↓  the model proposes an intent over symbols
  intent
    ↓  resolved against the live catalog, wallet and policy
  recipe          typed graph, 11 primitives
    ↓  compiled and type-checked — 28 diagnostics
  logical plan    ordered, provider-independent
    ↓  prepared one step at a time against live chain state
  transactions    batched where the wallet supports it
    ↓  you review and sign
  onchain
```

Stockfront never prepares a whole strategy up front. Each call returns only the steps that are **ready right now**, verified against the chain: a borrow is prepared only once its collateral is genuinely posted and the venue confirms the amount is safe. A step waiting on an earlier one says what it is waiting for, by name. Runs survive a refresh, a closed tab or a rejected signature, and resume from wherever the chain actually is.

The consequence is that a strategy can be **sized by reality instead of by guesswork**.

### The language

Eleven primitives. Four describe what you already have; seven describe what to do with it.

| Sources | Actions |
| --- | --- |
| `WALLET_ASSET` → tokens | `TRADE` tokens → tokens |
| `SUPPLIED_POSITION` → collateral | `SUPPLY` tokens → collateral |
| `BORROWED_POSITION` → debt | `BORROW` collateral → tokens |
| `EARN_POSITION` → deposit | `WITHDRAW` collateral → wallet |
| | `REPAY` debt → settled |
| | `EARN_DEPOSIT` tokens → deposit |
| | `EARN_WITHDRAW` deposit → wallet |

They compose freely wherever the types agree, which is what lets a handful of blocks express a very large number of strategies. `SUPPLY` hands back the collateral it creates, so one strategy can buy an asset, post it, borrow against it and put the loan to work — and repeat that to build leverage — in a single reviewed run.

The same recipe can be written three ways: in plain language, by dragging blocks on a canvas, or as JSON through the API. All three produce the identical artifact and go through the identical compiler.

---

## Composable by construction

The language describes **shapes of value movement**, not protocols. `TRADE` means *exchange one token for another at whichever venue quotes it best*. `SUPPLY` means *post an asset as collateral at a lending venue*. Nothing in the grammar, the compiler or the type system names a protocol.

Venues live behind ports. Adding one is an adapter and a line of configuration — never a change to the language, the compiler or the frontend.

Asset classes are data too. What may be traded for what is a **policy table the deployment supplies**, not a branch in code:

```
SETTLEMENT_TOKENS ↔ STOCKS      SETTLEMENT_TOKENS ↔ BASKETS      SETTLEMENT_TOKENS ↔ SETTLEMENT_TOKENS
```

That claim is load-bearing, and it has been tested. Index baskets — onchain folios holding many assets at once — were added as a **third asset class with no new primitive**. The existing `TRADE` block carried them, the route picker and signing path were reused untouched, and a single new transition in the policy table taught the whole system about them. The same door is open to any asset a venue will quote.

---

## Built on Binance Web3

Stockfront builds the strategy layer and leans on Binance's infrastructure beneath it. That is a deliberate trade: asset registries, routing and transaction construction are hard, fast-moving and already solved, and reusing them is what let a small team ship a whole strategy engine.

| Binance Web3 capability | What Stockfront gets from it |
| --- | --- |
| RWA asset registry and market data | The tokenized-asset catalog, issuer and attestation metadata, live and reference pricing, market status |
| Token balance API | Wallet holdings for any token by chain and contract — one call serves stocks, stablecoins and baskets |
| Trading and quote API | Aggregated routing across venues, price-impact figures and executable quotes |
| Transaction construction | Ready-to-sign approvals and swaps with gas fields, built from the chosen route |
| DeFi discovery and positions | Earn and liquidity opportunities, live APYs, TVL and the user's open positions |
| DeFi transaction building | Deposit, redeem and claim transactions for those positions |

Onchain reads sit alongside the APIs wherever a number decides solvency: collateral factors, liquidation thresholds, oracle validity and health factors come straight from the lending market, so a safety check never depends on an index being fresh.

---

## The interface

The strategies above are reachable three ways, and all three produce the same artifact through the same compiler.

**The composer** turns plain language into a validated recipe, situated in your actual portfolio.

**The builder** is the same recipe on a canvas: a block palette, typed connections, a live inspector with health-factor previews, and route selection per trade.

**The run screen** prepares one step at a time, batches calls into a single wallet prompt where EIP-5792 allows, follows a signed order to the venue that quoted it, and keeps a journal so a run resumes after a refresh.

**Curated strategies** lead the compose page with live figures, each naming the risk it carries.

**Positions** brings wallet holdings, collateral, debt with health factor, earn deposits and basket holdings into one view, each with a prepared exit.

---

## Why this compounds

**Discovery layer.** Nine earn products, four lending markets, six index baskets and forty-six stocks ranked and compared on every composition. No user reads four UIs.

**Automated strategies.** Multi-step carry and leverage become a single reviewed run instead of six manual transactions across three protocols.

**Composable positions.** Collateral, debt, earn deposits and basket holdings are all first-class values in the language, so the output of one strategy is the input to the next.

**Venue-agnostic by construction.** One strategy covers a catalog rather than a pair, and re-routes as rates move.

**Risk as a product surface**, not a protocol dashboard you are expected to go and read.

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

## Running it

```sh
cp .env.example .env     # Binance Web3 keys, an RPC URL and a model key
cargo run -p stockfront-app
```

```sh
cd fe && pnpm install && pnpm dev
```

The backend serves its own contract at `/v1/openapi.json`. The frontend's typed client is generated from the Rust handlers rather than hand-written:

```sh
cd fe && pnpm codegen        # exports openapi.json and regenerates the client
cd fe && pnpm gate           # types, lint and tests
```

The strategy model is pluggable — any OpenAI-compatible endpoint or Gemini, selected by configuration.

---

## Safety

- Non-custodial throughout. Positions live in your wallet and every transaction is signed by you.
- The model proposes only; addresses, amounts and connections are derived from live data.
- Borrowing holds a configured health-factor floor, with limits read from the venue rather than estimated.
- Deposits are restricted to explicitly reviewed earn products.
- Preparation re-reads live state, so a plan written minutes ago cannot execute against assumptions that have expired.
- Exits stay available even when an entry is no longer offered.

Returns are exposed to variable rates, protocol risk, price movement, liquidation and transaction costs. Nothing here is principal-protected.

---

## Architecture

Vertical product slices over hexagonal architecture. The domain owns financial rules and ports; protocols, HTTP, chain access and model providers are adapters. Strategy logic stays testable and no single provider's shape leaks into the product.

Contributors and coding agents should read [AGENTS.md](AGENTS.md) first — it defines the architecture boundaries, integration policy, testing standards, generated-code ownership and secret handling.

## License

MIT
