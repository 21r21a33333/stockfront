# Earn Deposit Recipes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a terminal compiler-backed Earn Deposit action that accepts a wallet stablecoin or the confirmed output of Borrow and atomically deposits it into a reviewed eligible earn opportunity.

**Architecture:** Extend the recipe language and runtime with `EARN_DEPOSIT`, while keeping `CarryService` authoritative for opportunity policy and transaction preparation. Export the resulting Rust contract through OpenAPI, then add a generated-type-driven React editor, preview, and atomic execution path.

**Tech Stack:** Rust 2024, Axum, Utoipa/OpenAPI, React 18, TypeScript 5.9, TanStack Query, Zustand, React Flow, wagmi/viem, Vitest.

**Spec:** `docs/superpowers/specs/2026-10-06-earn-deposit-recipes-design.md`

## Global Constraints

- BNB Chain only; reuse the configured settlement tokens, bStock catalog, Venus market, reviewed investment IDs, and minimum carry spread.
- `CarryService` remains the only owner of earn qualification and deposit preparation.
- Wallet input requires a selected supported collateral benchmark; Borrow input inherits and locks its upstream supplied collateral.
- Prepare refreshes all live data and accepts only `[APPROVE, DEPOSIT]` or `[DEPOSIT]`.
- Approval plus deposit requires the existing atomic wallet-call capability; never downgrade it to separate submissions.
- Earn Deposit is terminal and completes with an empty output list.
- API request and response types are generated from Rust OpenAPI; do not handwrite transport types.
- Do not use TDD for this feature. Add only the focused tests listed below after the corresponding implementation.

## Review Focus

- A Trade output connected to Earn Deposit must be rejected even if its token is an allowed stablecoin; Task 1 pins the allowed producer kinds.
- A Borrow output whose collateral differs from `collateralToken` must fail compilation; Task 1 pins the mismatch diagnostic.
- A reviewed investment that becomes unavailable between preview and prepare must open no wallet request; Task 2 pins the refreshed qualification check.
- A provider returning transactions out of order or with an unexpected kind must be rejected; Tasks 2 and 5 pin backend and frontend validation.
- A completed terminal deposit restored from the journal must have no spendable output and must not execute twice; Task 5 pins completion and restore behavior.

---

### Task 1: Add Earn Deposit to the recipe language and compiler

**Files:**
- Modify: `crates/core/src/recipes/model.rs`
- Modify: `crates/core/src/recipes/plan.rs`
- Modify: `crates/core/src/recipes/compiler.rs`
- Modify: `crates/core/src/recipes/diagnostics.rs`
- Modify: `crates/core/src/recipes/mod.rs`

**Interfaces:**
- Consumes: existing `AssetRef`, `SuppliedPositionRef`, graph descriptors, settlement-token policy.
- Produces: `RecipePrimitive::EarnDeposit`, `RecipeAction::EarnDeposit`, `RecipeEarnDepositPolicy`, and deterministic compiler diagnostics used by Task 2.

- [ ] **Step 1: Define the public recipe contract**

Append `EarnDeposit` to `RecipePrimitiveKind` and add these public shapes in
`model.rs`:

```rust
pub struct RecipeEarnDepositPolicy {
    pub allowed_inputs: Vec<RecipePrimitiveKind>,
}

EarnDeposit {
    #[serde(rename = "collateralToken", deserialize_with = "deserialize_from_string", serialize_with = "serialize_as_string")]
    collateral_token: Address,
    #[serde(rename = "investmentId")]
    investment_id: String,
}
```

Its descriptor has `asset: TOKEN_AMOUNT` as one consumable input, no output,
`collateralToken` sourced from `BSTOCKS`, and `investmentId` sourced from a new
`EARN_OPPORTUNITIES` option source. Add the policy to `RecipeCatalog` with
`allowed_inputs: [WALLET_ASSET, BORROW]`.

- [ ] **Step 2: Lower only the two supported graph shapes**

Add the provider-independent action in `plan.rs`:

```rust
RecipeAction::EarnDeposit {
    asset: AssetRef,
    collateral_token: Address,
    investment_id: String,
}
```

In `compiler.rs`, add `lower_earn_deposit`. Resolve the incoming `asset` edge,
inspect its source node, and accept only `WALLET_ASSET` or `BORROW`. For Borrow,
retain its `SuppliedPositionRef` while lowering and require
`collateral_token == borrow.collateral.token`. Emit field diagnostics for an
empty investment ID and collateral mismatch, and a port diagnostic for every
other source kind. Add stable diagnostic codes `INVALID_EARN_INVESTMENT`,
`EARN_COLLATERAL_MISMATCH`, and `UNSUPPORTED_EARN_INPUT`.

- [ ] **Step 3: Add focused compiler tests**

In the existing `compiler.rs` test module, add exactly these cases:

```rust
#[test]
fn lowers_wallet_and_borrow_inputs_into_terminal_earn_deposits() {
    for recipe in [wallet_earn_deposit_recipe(), borrow_earn_deposit_recipe(COLLATERAL)] {
        let plan = compile(&recipe).plan.expect("earn deposit should compile");
        assert!(matches!(
            &plan.steps.last().expect("deposit step").action,
            RecipeAction::EarnDeposit { collateral_token, investment_id, .. }
                if *collateral_token == COLLATERAL && investment_id == INVESTMENT_ID
        ));
    }
}

#[test]
fn rejects_trade_input_and_mismatched_borrow_collateral() {
    let trade = compile(&trade_earn_deposit_recipe());
    assert!(trade.report.diagnostics.iter().any(|item|
        item.code == RecipeDiagnosticCode::UnsupportedEarnInput));

    let mismatch = compile(&borrow_earn_deposit_recipe(OTHER_COLLATERAL));
    assert!(mismatch.report.diagnostics.iter().any(|item|
        item.code == RecipeDiagnosticCode::EarnCollateralMismatch));
}
```

Add test fixture constructors with these exact signatures in the same module:
`wallet_earn_deposit_recipe() -> RecipeDefinition`,
`borrow_earn_deposit_recipe(configured_collateral: Address) -> RecipeDefinition`,
and `trade_earn_deposit_recipe() -> RecipeDefinition`. Each fixture contains
one connected terminal Earn Deposit node and uses shared constants for node
IDs, `COLLATERAL`, `OTHER_COLLATERAL`, and `INVESTMENT_ID`.

Run:

```bash
cargo test -p stockfront-core recipes::compiler
cargo clippy -p stockfront-core --all-targets -- -D warnings
```

Expected: both commands pass.

- [ ] **Step 4: Commit the compiler contract**

```bash
git add crates/core/src/recipes
git commit -m "feat(recipes): compile earn deposits"
```

### Task 2: Integrate live carry preview and preparation

**Files:**
- Modify: `crates/core/src/prepared_action.rs`
- Modify: `crates/core/src/carry/model.rs`
- Modify: `crates/core/src/recipes/service.rs`
- Modify: `crates/app/src/main.rs`

**Interfaces:**
- Consumes: `CarryService::opportunities(CarryOpportunityQuery)` and `CarryService::prepare_deposit(&PrepareCarryDepositRequest)`.
- Produces: `RecipeNodePreview::EarnDeposit` and `PreparedRecipeStep::EarnDeposit { action: PreparedCarryDeposit }`.

- [ ] **Step 1: Make carry result types part of the generated recipe contract**

Add `utoipa::ToSchema` to carry types exposed by `CarryOpportunity`,
`PreparedCarryDeposit`, and `CarryAccount`, plus `ActionPreview` and
`AssetBalanceChange` in `prepared_action.rs`. Do not introduce recipe-owned
copies of carry models.

- [ ] **Step 2: Inject the existing CarryService and publish structural policy**

Change the constructor to accept the existing shared instance:

```rust
pub fn new(
    market: Arc<BstockMarketService>,
    trade: Arc<BstockTradeService>,
    borrowing: Arc<BorrowingService>,
    carry: Arc<CarryService>,
    settlement_tokens: Vec<Address>,
    withdrawal_pool: Address,
) -> Self
```

Store `carry` on `RecipeService`, include `earn_deposit_policy` in the catalog,
and pass `carry.clone()` from `crates/app/src/main.rs` while retaining the same
instance for `carry_router`. Add `RecipeError::Carry(#[from]
CarryServiceError)` so carry failures remain typed. Extend
`validate_action_policy` to require the input token to be a configured
settlement token and `trade.supports_pair_shape(collateral_token, input_token)`
to prove that the benchmark is a supported bStock/stablecoin pair.

- [ ] **Step 3: Implement preview states**

Resolve the input with the existing completed-node context. Return:

```rust
RecipeNodePreview::EarnDeposit {
    node_id,
    state,
    opportunities,
    selected_opportunity,
    waiting_for,
}
```

Use `WAITING_FOR_UPSTREAM` for an unsettled Borrow, `OUTPUT_UNAVAILABLE` for an
upstream completion without the expected asset, `READY` only when the selected
investment is currently `ELIGIBLE`, and `UNAVAILABLE` otherwise. Query
`CarryService` with the validated collateral and resolved stablecoin.

- [ ] **Step 4: Implement authoritative preparation**

For a ready step, find the input token decimals from the current matching
opportunity, convert raw units with `decimal_from_raw`, then call:

```rust
PrepareCarryDepositRequest {
    wallet,
    collateral_token,
    investment_id,
    amount,
}
```

Accept only transaction kinds `[Deposit]` or `[Approve, Deposit]`; map every
other provider result to invalid provider data. Return one
`PreparedRecipeStep::EarnDeposit`. Extend completed-context validation so this
terminal step accepts only an empty output list.

- [ ] **Step 5: Add focused service tests**

Extend the existing recipe service fakes with an `EarnReader` and
`EarnActionGateway`. Add only:

```rust
#[tokio::test]
async fn borrow_output_waits_then_prepares_the_exact_atomic_deposit() {
    let (service, gateway) = earn_deposit_service(
        eligible_opportunity(),
        [CarryTransactionKind::Approve, CarryTransactionKind::Deposit],
    );
    let request = borrow_earn_preview_request(Vec::new());
    assert!(matches!(service.preview(&request).await.unwrap().nodes.as_slice(),
        [RecipeNodePreview::Borrow { .. }, RecipeNodePreview::EarnDeposit {
            state: RecipeNodeState::WaitingForUpstream, ..
        }]));

    let prepared = service.prepare(&borrow_earn_prepare_request(completed_borrow())).await.unwrap();
    assert!(matches!(prepared.steps.as_slice(), [PreparedRecipeStep::EarnDeposit { .. }]));
    assert_eq!(gateway.last_deposit().amount, Decimal::new(3_000_000, 6));
}

#[tokio::test]
async fn prepare_rechecks_qualification_and_rejects_bad_provider_order() {
    let (unavailable, _) = earn_deposit_service(
        unavailable_opportunity(),
        [CarryTransactionKind::Deposit],
    );
    assert!(unavailable.prepare(&wallet_earn_prepare_request()).await.is_err());

    let (bad_order, _) = earn_deposit_service(
        eligible_opportunity(),
        [CarryTransactionKind::Deposit, CarryTransactionKind::Approve],
    );
    assert!(bad_order.prepare(&wallet_earn_prepare_request()).await.is_err());
}

#[test]
fn completed_earn_deposit_accepts_no_outputs() {
    let plan = compile(&wallet_earn_deposit_recipe()).plan.unwrap();
    assert!(validate_completed_context(&plan, &[completed_earn_deposit(Vec::new())]).is_ok());
    assert!(validate_completed_context(&plan, &[completed_earn_deposit(vec![token_output()])]).is_err());
}
```

Implement the named fixtures in the service test module. `earn_deposit_service`
must construct the normal `RecipeService` with fake market, trade, borrowing,
and carry dependencies; its second argument controls the exact provider
transaction order. `completed_borrow()` records a 3 USDT `borrowed` output in
six-decimal raw units.

Run:

```bash
cargo test -p stockfront-core recipes::service
cargo test -p stockfront-core carry
cargo clippy -p stockfront-core -p stockfront-app --all-targets -- -D warnings
```

Expected: all commands pass.

- [ ] **Step 6: Commit the live backend slice**

```bash
git add crates/core/src/prepared_action.rs crates/core/src/carry/model.rs crates/core/src/recipes/service.rs crates/app/src/main.rs
git commit -m "feat(recipes): prepare reviewed earn deposits"
```

### Task 3: Export the OpenAPI contract and regenerate TypeScript

**Files:**
- Modify: `crates/adapters/src/http/handlers/carry.rs`
- Modify: `crates/adapters/src/http/openapi.rs`
- Modify: `fe/openapi.json` (generated)
- Modify: `fe/src/api/generated.ts` (generated)
- Modify: `fe/src/api/recipes.ts`

**Interfaces:**
- Consumes: Task 2 Rust models and the existing `/v1/carry/opportunities` handler.
- Produces: generated `EARN_DEPOSIT`, carry opportunity, preview, and prepared-step TypeScript unions plus `recipesApi.carryOpportunities`.

- [ ] **Step 1: Add the existing opportunity endpoint to Utoipa**

Annotate `GET /v1/carry/opportunities` with query parameters
`collateralToken` and `debtToken`, response
`Response<List<CarryOpportunity>>`, and the default error envelope. Add the
handler to `ApiDoc.paths` and the needed carry schemas to `ApiDoc.components`.
Also annotate and export `GET /v1/wallets/{address}/carry-positions` so Task 5
can refresh the resulting position through the generated client.

- [ ] **Step 2: Pin the exported contract**

Add one adapter test asserting that the OpenAPI document contains:

```rust
assert!(schemas["RecipePrimitive"].to_string().contains("EARN_DEPOSIT"));
assert!(schemas["PreparedRecipeStep"].to_string().contains("EARN_DEPOSIT"));
assert!(document["paths"]["/v1/carry/opportunities"]["get"].is_object());
assert!(document["paths"]["/v1/wallets/{address}/carry-positions"]["get"].is_object());
```

- [ ] **Step 3: Regenerate and use the client**

Run:

```bash
cd fe
pnpm codegen
```

Add this generated-client wrapper in `fe/src/api/recipes.ts`:

```ts
carryOpportunities: (collateralToken: string, debtToken: string) =>
  apiResult(api.GET("/v1/carry/opportunities", {
    params: { query: { collateralToken, debtToken } },
  })),
carryPositions: (address: string) =>
  apiResult(api.GET("/v1/wallets/{address}/carry-positions", {
    params: { path: { address } },
  })),
```

- [ ] **Step 4: Verify and commit the contract boundary**

```bash
cargo test -p stockfront-adapters http::openapi
cd fe && pnpm codegen:check && pnpm typecheck
git add crates/adapters/src/http fe/openapi.json fe/src/api/generated.ts fe/src/api/recipes.ts
git commit -m "feat(api): export earn deposit recipe contract"
```

Expected: OpenAPI and TypeScript checks pass.

### Task 4: Add Earn Deposit to the editor and inspector

**Files:**
- Modify: `fe/src/recipe/editor/types.ts`
- Modify: `fe/src/recipe/editor/compiler.ts`
- Modify: `fe/src/recipe/editor/flowInput.ts`
- Modify: `fe/src/recipe/editor/store.ts`
- Modify: `fe/src/recipe/editor/context.ts`
- Modify: `fe/src/recipe/ActionPalette.tsx`
- Modify: `fe/src/recipe/canvas/RecipeCanvas.tsx`
- Modify: `fe/src/recipe/canvas/RecipeNodes.tsx`
- Create: `fe/src/recipe/inspector/EarnDepositFields.tsx`
- Modify: `fe/src/recipe/inspector/RecipeInspector.tsx`
- Modify: `fe/src/recipe/RecipeBuilderPage.tsx`
- Modify: `fe/src/recipe/RecipeBuilder.module.css`
- Test: `fe/src/recipe/editor/compiler.test.ts`
- Test: `fe/src/recipe/editor/store.test.ts`
- Test: `fe/src/recipe/inspector/RecipeInspector.test.tsx`

**Interfaces:**
- Consumes: generated `CarryOpportunity`, `RecipeEarnDepositPolicy`, `RecipeNodePreview`, and `PreparedRecipeStep` types from Task 3.
- Produces: a serializable canvas node, compatible append action, live opportunity selector, and prepare controls used by Task 5.

- [ ] **Step 1: Add the editor node without handwritten API models**

Define only local editor state:

```ts
export type EarnDepositNodeData = {
  kind: "EARN_DEPOSIT";
  collateralToken?: Address;
  investmentId: string;
};
```

Add `earnDeposit` to `flowType`, node registration, canvas node types, palette
presentation, title/tone helpers, and `buildRecipe`. Serialize exactly
`collateralToken` and `investmentId` from editor state.

- [ ] **Step 2: Add safe append semantics**

Add `appendEarnDeposit(sourceId, collateralToken?)`. Permit only a wallet node
or Borrow node whose consumable output is unused. Connect `asset` or `borrowed`
to the new node's `asset` handle. For a Borrow source, derive the collateral by
following its `collateral` input to `SUPPLIED_POSITION`, set that token on the
new node, and keep it locked. A wallet source starts with the default supported
bStock benchmark.

- [ ] **Step 3: Render the node and opportunity inspector**

`EarnDepositFields.tsx` derives the bound stablecoin from the incoming edge and
queries `recipesApi.carryOpportunities(collateralToken, debtToken)` only when
both addresses exist. Filter to `qualification === "ELIGIBLE"`, retain an
unsupported saved choice as visibly unavailable, and select the highest-spread
eligible result only when `investmentId` is empty.

Render asset/amount, collateral benchmark, protocol and investment identity,
earn APY, borrow APY, base spread, TVL, liquidity, observation state, and the
prepared approval/deposit summary. Lock the collateral control for Borrow;
allow supported bStock selection for wallet input. Reuse existing card and
overflow-safe number styles.

- [ ] **Step 4: Add three minimal editor tests**

```ts
it("serializes a wallet stablecoin earn deposit with its collateral benchmark", () => {
  const recipe = buildRecipe(1, [walletNode, earnNode], [walletEarnEdge], assets);
  expect(recipe.nodes[1]).toEqual({
    id: earnNode.id,
    kind: "EARN_DEPOSIT",
    collateralToken: nvdab,
    investmentId: "investment-1",
  });
});

it("appends an earn deposit to Borrow with inherited collateral", () => {
  seedBorrowGraph();
  useRecipeEditor.getState().appendEarnDeposit(borrowId);
  const state = useRecipeEditor.getState();
  expect(state.nodes.at(-1)?.data).toMatchObject({
    kind: "EARN_DEPOSIT",
    collateralToken: nvdab,
  });
  expect(state.edges.at(-1)).toMatchObject({
    source: borrowId,
    sourceHandle: "borrowed",
    targetHandle: "asset",
  });
});

it("locks Borrow collateral and keeps an unavailable saved investment editable", async () => {
  renderEarnInspector({ source: "BORROW", qualification: "UNAVAILABLE" });
  expect(screen.getByRole("button", { name: /collateral/i })).toBeDisabled();
  expect(await screen.findByText(/no longer available/i)).toBeVisible();
  expect(screen.getByRole("combobox", { name: /investment/i })).toBeEnabled();
});
```

Define `walletNode`, `earnNode`, and `walletEarnEdge` beside the existing editor
fixtures. Add `seedBorrowGraph()` to `store.test.ts` and
`renderEarnInspector(options: { source: "BORROW"; qualification:
"UNAVAILABLE" }): void` to the inspector test support using the existing
QueryClient/test renderer and generated response shapes.

Run:

```bash
cd fe
pnpm test -- src/recipe/editor/compiler.test.ts src/recipe/editor/store.test.ts src/recipe/inspector/RecipeInspector.test.tsx
pnpm typecheck
```

Expected: focused tests and typecheck pass.

- [ ] **Step 5: Commit the editor slice**

```bash
git add fe/src/recipe
git commit -m "feat(recipes): build earn deposit actions"
```

### Task 5: Execute, reconcile, and refresh terminal deposits

**Files:**
- Modify: `fe/src/recipe/execution/preparedExecution.ts`
- Modify: `fe/src/recipe/execution/model.ts`
- Modify: `fe/src/recipe/execution/journal.ts`
- Modify: `fe/src/recipe/execution/ExecutionPanel.tsx`
- Modify: `fe/src/recipe/execution/useRecipeExecution.ts`
- Modify: `fe/src/recipe/RecipeBuilderPage.tsx`
- Test: `fe/src/recipe/execution/preparedExecution.test.ts`
- Test: `fe/src/recipe/execution/journal.test.ts`
- Test: `fe/src/recipe/execution/useRecipeExecution.test.tsx`

**Interfaces:**
- Consumes: `PreparedRecipeStep::EarnDeposit` containing a `PreparedCarryDeposit`.
- Produces: one terminal `CALL_BATCH` attempt with `executionMode: "EARN_DEPOSIT"`, durable status only, empty completed outputs, and refreshed account reads.

- [ ] **Step 1: Validate and translate the prepared action**

Extend `PreparedExecution` with `EARN_DEPOSIT`. Validate transaction kinds with
a dedicated helper:

```ts
type EarnDepositTransactions = Extract<
  PreparedRecipe["steps"][number],
  { kind: "EARN_DEPOSIT" }
>["action"]["transactions"];

function earnDepositCalls(transactions: EarnDepositTransactions) {
  const kinds = transactions.map(({ kind }) => kind);
  const deposit = kinds.length === 1 && kinds[0] === "DEPOSIT";
  const approveDeposit =
    kinds.length === 2 && kinds[0] === "APPROVE" && kinds[1] === "DEPOSIT";
  if (!deposit && !approveDeposit)
    throw new Error("The prepared earn deposit action is incomplete.");
  return transactions.map(call);
}
```

Return one call batch, set `atomicRequired` only for the two-call form, and do
not attach an output.

- [ ] **Step 2: Extend durable state without persisting calldata**

Permit `EARN_DEPOSIT` in attempt kinds, step kinds, execution modes, recipe
validation, progress labels, `isRunComplete`, and `completedRecipeNodes`.
Completed Earn Deposit attempts serialize `outputs: []`. Keep journal version 2
because the added union member is backward-compatible; old journals remain
valid.

- [ ] **Step 3: Reuse execution and reconciliation**

Route the new call batch through the existing atomic capability check, wallet
submission, receipt polling, dropped/reverted reconciliation, and fresh-prepare
manual retry. Update the activity label to `Deposit <symbol> into <investment>`
using recipe and prepared metadata; do not create a second polling loop.

Add a `carryPositions` query in `RecipeBuilderPage` keyed by the connected
wallet. After confirmed completion, refetch wallet balances, the borrowing
account, and carry positions together. Keep the position response ready for the
next Earn Position source slice even though this slice does not render that
source yet.

- [ ] **Step 4: Add three execution tests**

```ts
it("keeps approval before earn deposit in one atomic wallet batch", async () => {
  const execution = await preparedExecution(preparedEarn("APPROVE", "DEPOSIT"), client, wallet);
  expect(execution).toMatchObject({
    kind: "CALL_BATCH",
    action: "EARN_DEPOSIT",
    steps: ["APPROVAL", "EARN_DEPOSIT"],
    atomicRequired: true,
  });
  expect(execution.kind === "CALL_BATCH" && execution.calls).toHaveLength(2);
});

it("rejects a malformed earn transaction order before opening the wallet", async () => {
  await expect(preparedExecution(preparedEarn("DEPOSIT", "APPROVE"), client, wallet))
    .rejects.toThrow("The prepared earn deposit action is incomplete.");
  expect(sendCalls).not.toHaveBeenCalled();
});

it("settles an earn deposit with no output and does not replay it after restore", async () => {
  const run = settledEarnRun();
  writeRun(run);
  const restored = readRun(run.wallet, run.chainId)!;
  expect(completedRecipeNodes(restored.attempts)).toEqual([
    { nodeId: earnNodeId, outputs: [] },
  ]);
  expect(isRunComplete(restored)).toBe(true);
});
```

Add `preparedEarn(...kinds: CarryTransactionKind[]): PreparedRecipe` in
`preparedExecution.test.ts` using generated prepared transaction fields and
`settledEarnRun(): ExecutionRun` in `journal.test.ts`. In
`useRecipeExecution.test.tsx`, initialize the existing hook harness from that
settled journal and assert `result.current.completed === true`,
`result.current.canExecute === false`, and `sendCalls` was not called.

Run:

```bash
cd fe
pnpm test -- src/recipe/execution/preparedExecution.test.ts src/recipe/execution/journal.test.ts src/recipe/execution/useRecipeExecution.test.tsx
pnpm gate
```

Expected: focused execution tests and the complete frontend gate pass.

- [ ] **Step 5: Commit the execution slice**

```bash
git add fe/src/recipe
git commit -m "feat(recipes): execute atomic earn deposits"
```

### Task 6: End-to-end verification

**Files:**
- No planned file changes; repair only a concrete defect exposed by the gates.

**Interfaces:**
- Consumes: the complete backend and frontend feature.
- Produces: a clean branch and a reproducible manual test sequence.

- [ ] **Step 1: Run repository gates**

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cd fe && pnpm gate
```

Expected: every command passes.

- [ ] **Step 2: Run the two product journeys**

Start the backend with the configured Binance, Glider, Venus, and RPC settings,
then start Vite. Verify:

```text
Wallet USDT -> Earn Deposit
Supplied TSLAB -> Borrow USDT -> Earn Deposit
```

For each journey, confirm the selected opportunity remains eligible at prepare,
the wallet receives one atomic request when approval is required, the receipt
settles, the node exposes no output edge, and the refreshed carry account shows
the position.

- [ ] **Step 3: Verify the fail-closed path**

Change the selected investment after preview or use an investment that is no
longer eligible. Confirm preparation fails before MetaMask opens and the node
remains editable.

- [ ] **Step 4: Record final state**

```bash
git status --short
git log --oneline -6
```

Expected: no uncommitted files and the compiler, backend, generated contract,
editor, and execution commits are present.
