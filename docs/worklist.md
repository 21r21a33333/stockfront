# Stockfront status

Current as of 2026-10-11, on `main`.

## Shipped

### The strategy engine

- **Recipe language** — 11 primitives over a typed directed graph, with ports
  carrying four value types and edges legal only where the types meet.
- **Compiler** — structural and policy validation producing 28 located
  diagnostics, lowering a valid graph to a provider-independent logical plan.
- **State-gated preparation** — each call returns only the steps ready right
  now, verified against live chain state, so a plan can never execute against
  assumptions that have expired.
- **Supply into borrow in one strategy** — a supply hands back the collateral
  it creates, so buying an asset, posting it, borrowing and putting the loan to
  work is a single reviewed run, and repeating that builds leverage.
- **Venue-sized borrowing** — a borrow amount is optional; its absence asks the
  venue for the most it will lend while holding the health floor, read when the
  collateral exists rather than projected when the recipe is written.

### The composer

- Natural language to a validated recipe, with model output confined to an
  intent over symbols and every address, amount and edge derived deterministically.
- Situated in the connected portfolio: holdings resolve "all of it" to a number
  and let the composer decline what the wallet does not hold.
- Basket descriptions, themes and holdings in context, so thematic requests
  resolve to the right index.
- A single table of the strategies the product offers, rebuilt against whichever
  asset suits the request, with the alternatives named in assumptions.
- Refinement that carries its own question back, so a bare answer lands.
- Pluggable model provider: any OpenAI-compatible endpoint or Gemini.

### Execution and interface

- Builder canvas with typed connections, live health-factor previews and
  per-trade route selection.
- Run screen with EIP-5792 batching, a journal that survives a refresh, orders
  followed to the venue that quoted them, and clear reporting when a step cannot
  be prepared.
- Curated strategies leading the compose page with live figures and named risks.
- Positions across wallet holdings, collateral, debt with health factor, earn
  deposits and baskets, each with a prepared exit.

### Integrations

- Binance Web3: asset registry and market data, token balances, aggregated
  quotes, transaction construction, DeFi discovery and positions, DeFi
  transaction building.
- Collateralised lending with onchain reads for collateral factors, liquidation
  thresholds, oracle validity and health.
- Reviewed earn products, deposit and withdrawal.
- Index baskets, added as a third asset class with no new primitive.
- Response caching with request coalescing across the read paths.

## Next

**Pre-flight simulation.** Simulate the prepared batch before asking for a
signature, so a venue that quotes but cannot fill is reported in words instead
of reaching the wallet as a generic warning, and the next route can be taken
automatically.

**Liquidity as a primitive.** The concentrated-liquidity slice is complete
behind the API — opportunities, entry, fee claims, withdrawal and positions.
Three primitives would bring it into the language and the canvas.

**Market status in the catalog.** Surface venue fillability alongside the
tradable flag, so closed-market hours are visible before a strategy is drafted.

**Minimum sizes.** A real dust threshold on earn positions, so a drained
position is no longer offered for withdrawal.

**Corporate actions.** Dividend, split, rebase and multiplier accounting across
positions.

**MCP server over the recipe API.** Expose the primitives, validation and
preparation so external agents can compose strategies against the same compiler.

**Wallets without atomic batching.** A sequential approve-then-act path for
wallets that do not implement EIP-5792.
