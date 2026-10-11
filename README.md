# Stockfront

**Say what you want your money to do. Stockfront turns it into transactions you review and sign.**

Stockfront is a non-custodial strategy engine for onchain finance. You describe an outcome in plain language — *"buy Nvidia, post it as collateral, borrow against it and earn on the loan"* — and Stockfront composes that into a typed, validated, executable plan. Every step is prepared against live chain state, every transaction is signed by you, and nothing runs on its own.

Underneath the chat box is the part that makes it work: **a strategy language and a compiler for it.**

---

## Strategies

Live readings from the running deployment on BNB Chain, 2026-10-11. Rates move; the engine re-reads them on every composition.

### Earn

**USD Carry** · *live* · **+1.11% net spread**

Collateralize a tokenized stock, borrow USDT at **5.12% APY**, deploy into the reviewed Lista USDT product at **6.23% APY**, keep the difference. Stock exposure is retained the whole time and the position unwinds through a prepared exit.

The engine ranks **9 discovered earn products** on every composition — currently spanning **2.18% to 11.22%** across Plume, Lista, Venus, Venus Flux and Aave V3. Only reviewed products can be deposited into, so the executable spread is the conservative one; as products clear review the same strategy routes to them with no change to the user's intent. Borrow sizing holds a **1.5 health-factor floor** and comes from the venue, never an estimate.

**Leveraged Stock** · *live* · **1.60× exposure per run**

Buy a stock, post it as collateral, borrow against it, buy more, post that too — one strategy, one review, one signing session. Repeating the run compounds the exposure. Every borrow is gated on the venue confirming the amount holds the health floor, so an overextended loop is refused rather than liquidated.

Collateral terms today: **0.60 CF / 0.70 LT** on majors, **0.50 CF / 0.65 LT** on the rest.

**Index Baskets** · *live* · **0.30%–0.60% annual fee**

One token holding an entire index, backed by its assets and rebalanced onchain. A broad market-cap index at **0.30%** across **17 holdings**, and five thematic onchain folios at **0.60%** covering AI infrastructure, AI power, photonics, neocloud capacity and robotics, **8 to 25 holdings** each. No debt, no liquidation — market risk only.

**Basis trades, covered calls, collars** · *roadmap* — unlocked as perpetual and options venues are adapted.

### Borrow

**Floating-Rate Borrowing** · *live* · **5.12% APY on USDT**

Borrow against tokenized stock collateral without selling it. Collateral factors, liquidation thresholds, oracle validity and health factors are read directly from the lending market rather than from an index. **$40.9M** of borrow liquidity available.

**Venue-Sized Maximums** · *live*

Ask for the most you can safely borrow and no number is written into the strategy at all. The venue states its own limit once the collateral exists, and that figure is what you review before signing.

### Trade

**Spot, aggregated** · *live* · **46 tokenized stocks, 6 index baskets, stablecoin pairs**

Routed across venues, with price impact on every quote and a minimum-received figure on every trade. Approvals and actions batch into a single wallet prompt where EIP-5792 allows.

**Perpetuals and hedging** · *roadmap*

### Exit

Every position carries a prepared exit — repay, withdraw, redeem, sell — and exits stay available even when an entry is no longer offered, so a policy change cannot trap a position.

The full breakdown, including market context and roadmap, is in [PRODUCT.md](PRODUCT.md).

---

## Why a compiler

A language model can write a plausible DeFi strategy in a second. The distance between *plausible* and *executable* is where money is lost.

A strategy that reads perfectly can still name an asset the venue has never listed, state an amount with more decimal places than the token has, feed a lending position into a step that expects tokens, spend the same output twice, borrow past a safe health factor, or quietly form a loop. None of that is visible in prose. All of it is visible to a type system.

So Stockfront never lets the model touch the money. The model proposes an **intent** over symbols — "borrow USDT against TSLAB" — and nothing else. It cannot emit a contract address, a raw token amount, or a connection between steps. Those are derived afterwards, deterministically, from live catalog and account data.

That intent is resolved into a **recipe**: a typed directed graph. Every block has ports, every port carries a value type, and an edge is legal only when an output type meets a matching input type.

```
TOKEN_AMOUNT        tokens that can be spent
SUPPLIED_POSITION   collateral that can be borrowed against or withdrawn
BORROWED_POSITION   debt that can be repaid
EARN_POSITION       a deposit that can be redeemed
```

The compiler then type-checks the whole graph and reports **28 distinct diagnostics** against it — unknown ports, mismatched types, missing inputs, reused outputs, cycles, disconnected blocks, unsupported trade pairs, collateral a venue will not take, amounts that cannot be expressed. Each one points at the exact node, port or field responsible, so a rejected draft is repairable rather than merely refused.

Only a graph that passes becomes a **logical plan** — an ordered, provider-independent description of what must happen. It holds no calldata, no quotes and no provider identifiers, because those change by the second and belong to execution, not to the plan.

**This is what makes an open-ended AI agent safe to hand a signature to.** Without it you get one of two products: a chatbot whose output you have to trust, or a menu of a few hardcoded flows. The compiler gives you the open-ended language *and* the guarantee.

---

## From a sentence to a signature

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

The last stage is the one that matters most in practice. Stockfront never prepares a whole strategy up front. Each call returns only the steps that are **ready right now**, verified against the chain: a borrow is prepared only once its collateral is genuinely posted and the venue confirms the amount is safe. A step that depends on an earlier one reports what it is waiting for, by name. Runs survive a refresh, a closed tab or a rejected signature, and resume from wherever the chain actually is.

The consequence is a strategy can be **sized by reality instead of by guesswork**. Ask to borrow the most you safely can and no number is written into the recipe at all — the venue states its own limit when the collateral exists, and that figure is what you review.

---

## The language

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

That claim is load-bearing, and it has been tested. Index baskets — Reserve's onchain folios, a token holding many assets at once — were added as a **third asset class with no new primitive**. The existing `TRADE` block carried them, the builder's route picker and signing path were reused untouched, and a single new transition in the policy table taught the whole system about them. The same door is open to any asset a venue will quote.

Every slice is a vertical: domain rules and ports in the core, protocols and transport as adapters. No provider's data model is allowed to become the product's model.

---

## Built on Binance Web3

Stockfront builds the strategy layer and leans on Binance's infrastructure for everything beneath it. That is a deliberate trade: the hard, fast-moving work of asset registries, routing and transaction construction is already solved and maintained, and reusing it is what let a small team ship a whole strategy engine.

| Binance Web3 capability | What Stockfront gets from it |
| --- | --- |
| RWA asset registry and market data | The tokenized-asset catalog, issuer and attestation metadata, live and reference pricing, market status |
| Token balance API | Wallet holdings for any token by chain and contract — the same call serves stocks, stablecoins and baskets |
| Trading and quote API | Aggregated routing across venues, price-impact figures and executable quotes |
| Transaction construction | Ready-to-sign approvals and swaps with gas fields, built from the chosen route |
| DeFi discovery and positions | Earn and liquidity opportunities, live APYs, TVL and the user's open positions |
| DeFi transaction building | Deposit, redeem and claim transactions for those positions |

Onchain reads sit alongside the APIs wherever a number decides solvency: collateral factors, liquidation thresholds, oracle validity and health factors are read directly from the lending market, so a safety check never depends on an index being fresh.

---

## The interface

The strategies above are reachable three ways, and all three produce the same artifact through the same compiler.

**The composer** turns plain language into a validated recipe, and it is situated in your actual portfolio. It knows what you hold, so *"sell all my Tesla"* resolves to an exact balance instead of a question — and *"sell my Apple stock"* answers that you do not hold any rather than drafting something that cannot settle. It knows what each basket contains and what it is for, so *"something diversified"*, *"AI infrastructure"* and *"a risk-adjusted fund"* all find the right one. It knows the strategies the product offers and rebuilds them against whichever asset suits the request.

**The builder** is the same recipe on a canvas: a block palette, typed connections, a live inspector with health-factor previews, and route selection per trade. Every edit is validated by the same compiler the composer answers to.

**The run screen** prepares one step at a time, batches calls into a single wallet prompt where EIP-5792 allows, follows a signed order to the venue that quoted it, and keeps a journal so a run resumes after a refresh.

**Curated strategies** lead the compose page with live figures — net carry after borrow cost, exposure per run, trailing index performance — each naming the specific risk it carries rather than wearing a score.

**Positions** brings wallet holdings, collateral, debt with health factor, earn deposits and basket holdings into one view, each with a prepared exit.

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
- Borrowing holds a configured health-factor floor, and limits are read from the venue rather than estimated.
- Deposits are restricted to explicitly reviewed earn products.
- Preparation re-reads live state, so a plan written minutes ago cannot execute against assumptions that have expired.
- Exits stay available even when an entry is no longer offered.

Returns are exposed to variable rates, protocol risk, price movement and transaction costs. Nothing here is principal-protected.

---

## Architecture

Vertical product slices over hexagonal architecture. The domain owns financial rules and ports; protocols, HTTP, chain access and model providers are adapters. Strategy logic stays testable and no single provider's shape leaks into the product.

Contributors and coding agents should read [AGENTS.md](AGENTS.md) first — it defines the architecture boundaries, integration policy, testing standards, generated-code ownership and secret handling.

## License

MIT
