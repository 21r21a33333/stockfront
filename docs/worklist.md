# Stockfront worklist

Status as of 2026-10-09. Working branch: `feat/s6-recipe-trade`.
Hackathon: BNB Hack: Tokenized Stocks Edition. Submissions lock **Sun 11 Oct 2026, 12:00 UTC**.
Judging: technical 30%, creativity 25%, Developer Experience Report 25%, product/UX 20%.

Every code task follows the AGENTS.md gates: research → HLD/LLD → approval → implement → audit → fmt/clippy/tests → review.

## P0: required for the hackathon submission

### Submission package
- [ ] Decide what goes in: merge `feat/s6-recipe-trade` into `main`, or submit from the branch.
- [ ] Public GitHub repo. Check that no `.env`, keys or private keys are committed, and rotate the Binance and Glider keys that were shared in chat.
- [ ] Confirm team eligibility (restricted jurisdictions) and register / "Apply as Hacker".
- [ ] Deploy the backend and frontend, or write precise run instructions for judges (env vars, ports, wallet on BSC).
- [ ] Demo video, 4 minutes or less, showing the live mainnet flow with small amounts.
- [ ] Developer Experience Report, **written by the team** (AI-generated reports are rejected). Sections: onboarding, documentation issues, API pitfalls, AI stack, tokenized-stock specifics, redesign suggestions, requested capabilities.
  - Claude can compile factual raw notes of the API quirks the code works around (HMAC signing, `tx_time` seconds vs ms, inconsistent business codes, long decimal strings, investment-detail N+1, etc.).
- [ ] Update the README so the pitch matches what is built (AI strategy composer + recipes); drop perps from the pitch.

### AI strategy agent (the "agent" story for the creativity score)
- [x] Natural language → recipe (backend done, branch `feat/s7-strategy-composer`; Gemini via `GEMINI_API_KEY`): a prompt box that turns a plain-English strategy into recipe JSON using the primitives catalog. It calls `/v1/recipes/validate`, repairs errors from the diagnostics, renders the result on the canvas, and the user previews and signs. Needs an LLM choice and API key (Claude suggested).
- [ ] Earn deposit block (`EARN_DEPOSIT`), from wallet USDT or Borrow output into a reviewed earn investment. The spec and plan already exist in `docs/superpowers/`.
- [ ] Configure `STOCKFRONT_CARRY_INVESTMENT_IDS` with at least one reviewed investment (e.g. a Lista USDT route) so earn is eligible.

### Make the demo path safe and reliable
Demo path: wallet USDT → buy bStock → supply to Venus → borrow USDT → earn → repay → withdraw.
- [ ] Recipe server trusts client-reported completed steps: an omitted completed Borrow can be prepared again (double borrow). Reported trade outputs are unchecked. Verify against tx status.
- [ ] Execute silently re-quotes and signs a quote the user never saw. Show the fresh quote and minimum received, and confirm or refuse if worse.
- [ ] A run stuck in "attention" (any non-4001 wallet error, or atomic not reported) has no reset; the only escape is clearing localStorage. Add an abandon/reset action.
- [ ] Venus math behind the 1.5 HF floor:
  - [ ] underlying price mantissa decoded as 18 decimals;
  - [ ] borrow cap 0 treated as "cap reached";
  - [ ] native BNB collateral breaks the risk preview;
  - [ ] the conservative max uses the liquidation threshold instead of the collateral factor;
  - [ ] the repay balance check requires the buffered amount rather than the actual debt.
- [ ] Fix the e2e suite: the stale mocked catalog (missing `borrowPolicy`/`repayPolicy`) and an ambiguous locator. Currently 10 of 15 fail.
- [ ] Rehearse the full flow live with a funded wallet on BSC.

### Frontend (owned by the user, via Claude Design)
- [ ] Strategy-first redesign from `docs/frontend-design-brief.md`: Landing, Compose (AI), Builder, Run, Positions sheet. Markets, baskets, managed portfolios and LP are deliberately out of the UI for the hackathon; the backend support stays.

### Backend for the strategy-first UI (checked 2026-10-10)
- [x] AI composer: `POST /v1/strategies/compose` (prompt + wallet + optional current recipe → summary, recipe, assumptions, missing inputs, validation). Core use case and port, LLM adapter, config in `setting.rs`, validate/repair loop, amount and position resolution.
- [ ] `EARN_DEPOSIT` block plus a configured reviewed carry investment ID.
- [ ] Positions sheet: add `/v1/wallets/{address}/portfolio` and `/carry-positions` to the OpenAPI contract (loans is already there). Block market info (price, reference price, market status) is already in the recipe catalog `bstocks`.
- [ ] Recipe preview: a supply or borrow provider error should mark that node UNAVAILABLE instead of failing the whole preview.
- [ ] `SUPPLY.venue` casing (`venus` vs the upper-case contract).

## P1: raises the score if time allows
- [ ] Market and portfolio screen: bStock catalog with on-chain vs reference price gap and market open/closed status (matches the hackathon's "markets closed, on-chain trades on" framing), plus wallet holdings, the Venus loan and health factor, and carry positions.
- [ ] MCP server over the recipe API so Claude, Cursor or Agent Studio can compose strategies (also a hackathon idea; possible Agent Studio special prize).
- [ ] Use the Market API (prices and candles) for signals and charts. It is currently unused.
- [ ] Wallets without EIP-5792 atomic batching: a sequential approve-then-act path. Today these flows fail.
- [ ] Frontend robustness: an error boundary, unknown node kinds no longer crashing the palette, a "no wallet found" message, a fix for the "Valid" shown when validation fails, and localStorage guards.
- [ ] Sign-safety checks: approval spender and amount, and EIP-712 domain chain/maker before signing; the trade approval venue derived from the chosen quote.

## P2: complete the product after the hackathon
### Backend correctness and safety
- [ ] Fusion (Reserve baskets): validate typed data against the request; use the quote's approval spender instead of the hard-coded 1inch router; implement `transaction_status`.
- [ ] Glider: validate withdrawal typed data (chain, recipient, assets); use checked summation for position totals; stop mapping 4xx to 503.
- [ ] Venus `prepare_supply` overwrites Binance's simulated approve calldata; supply-plan final-step checks are missing.
- [ ] Earn/liquidity opportunities return `InvalidData` when the filtered list is empty; `venus_investment_id` ambiguity; sequential N+1 provider calls.
- [ ] HTTP: `Status` serialises as "Ok"/"Error"; some extractors return plain-text 400s outside the envelope.
- [ ] Consistency: `SUPPLY.venue` expects lowercase `venus` while the rest of the contract is upper-case.
- [ ] Remove the hand-built repay transaction without gas fields (prepare it via Binance).

### AGENTS.md conformance cleanup
- [ ] Recipe core is over-built for linear chains (petgraph, Kahn, Tarjan, DFS, frontier scheduler); simplify.
- [ ] Remove unused variants and fields (`RecipePortFlow::Reference`, `Warning` severity, `required`, unused repayment reasons).
- [ ] Decide whether `utoipa` annotations and UI copy may live in core.
- [ ] Replace free `map_*` functions with `From`/`TryFrom`; use named result types instead of tuples.
- [ ] Remove derive/serde/static-config tests; strengthen live tests (approval spender, scope and amounts).
- [ ] Split `RecipeBuilderPage.tsx` (913 lines); remove the frontend's hand-copied enum unions and cross-feature imports.

### Product scope still missing in the UI
- [ ] Screens for borrowing (markets, loan dashboard), carry/earn (opportunities, positions, withdraw, redemption claims), liquidity (opportunities, positions, fee claims, withdraw), Reserve baskets, and Glider managed portfolios. The backend endpoints exist; add them to the OpenAPI contract and generated client.
- [ ] Recipe blocks: liquidity provision, earn withdrawal, redemption claim, LP fee claim.

### Not built anywhere yet (README roadmap)
- [ ] Risk monitoring and alerts: health factor, carry turning negative, LP out of range, exit liquidity, stock/reference price deviation, with prepared safety actions.
- [ ] Dividend, split, rebase and multiplier accounting.
