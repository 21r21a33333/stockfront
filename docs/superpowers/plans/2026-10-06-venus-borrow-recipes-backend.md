# Venus Borrow Recipes Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add backend support for starting a recipe from an existing Venus supplied bStock position, safely enabling it as collateral when required, and borrowing an exact supported stablecoin amount with a projected health-factor floor of `1.5`.

**Architecture:** Extend the existing borrowing domain with one live amount-specific capability read, keep Venus-specific membership/oracle/cap arithmetic in the Venus adapter, and enforce allowlists plus the health-factor floor in `BorrowingService`. Extend the pure recipe language with a typed Borrow action and let `RecipeService` return either an activation stage or a Borrow stage after re-reading live state.

**Tech Stack:** Rust 2024, Tokio, Axum, Alloy, rust_decimal, Serde, Utoipa, existing Venus/Binance adapters.

**Spec:** `docs/superpowers/specs/2026-10-06-venus-borrow-recipes-design.md`

## Global Constraints

- BNB Chain mainnet only (`chainId = 56`).
- Venus Core Pool only for this slice.
- Exact positive borrow amounts only; the saved recipe stores `rawAmount` as a decimal string containing a positive `U256`.
- Only configured stablecoin debt tokens are valid.
- Projected account-wide health factor must be present and at least `1.5` before Borrow preparation.
- Collateral activation and Borrow are separate user-authorized stages.
- Activation never completes the Borrow node and never publishes token output.
- Prepare re-reads live state; preview never authorizes a transaction.
- Reuse existing borrowing and recipe endpoints; add no provider, endpoint family, or dependency.
- Keep generated frontend contracts out of this backend plan.
- Do not use a red-green TDD loop. Add only the focused boundary and regression
  tests named in this plan; every test must protect a distinct production risk.

## Review Focus

- A supplied position exists but belongs to another Venus pool: preview is unavailable and prepare emits no transaction; covered in Task 2 adapter and Task 4 service tests.
- The selected market is supplied but not enabled: preview requests activation and a successful activation does not complete Borrow; covered in Tasks 2 and 4.
- Current account debt uses multiple enabled collateral assets: capability uses the complete pooled account rather than only the selected position; covered in Task 2.
- Requested amount is one raw unit above the HF/cap/liquidity maximum: preview is unavailable and prepare rejects; covered in Tasks 1, 2, and 4.
- A client claims activation completed Borrow or changes the completed output amount/token: runtime context validation rejects it; covered in Task 3.

---

## File map

- `crates/core/src/bstock_borrowing/model.rs`: protocol-independent capability, stage, and unavailable-reason types.
- `crates/core/src/bstock_borrowing/ports.rs`: live capability read port.
- `crates/core/src/bstock_borrowing/service.rs`: allowlist and HF-floor policy; capability orchestration; direct Borrow safety enforcement.
- `crates/adapters/src/venus/risk.rs`: account-wide collateral/debt calculation and conservative maximum.
- `crates/adapters/src/venus/markets.rs`: capability reader implementation using current market and membership state.
- `crates/core/src/recipes/model.rs`: public Borrow primitive, catalog policy, preview, and prepared variants.
- `crates/core/src/recipes/plan.rs`: provider-independent Borrow logical action.
- `crates/core/src/recipes/compiler.rs`: typed lowering and exact output registration.
- `crates/core/src/recipes/service.rs`: live preview, stage-aware prepare, and runtime-context validation.
- `crates/adapters/src/http/openapi.rs`: schema registration only if Utoipa cannot discover a nested new type automatically.

### Task 1: Borrowing capability and safety policy

**Files:**
- Modify: `crates/core/src/bstock_borrowing/model.rs`
- Modify: `crates/core/src/bstock_borrowing/ports.rs`
- Modify: `crates/core/src/bstock_borrowing/service.rs`
- Modify: `crates/core/src/bstock_borrowing/mod.rs`
- Test: `crates/core/src/bstock_borrowing/service.rs`

**Interfaces:**
- Consumes: existing `BorrowingAccount`, `BorrowingToken`, `PrepareBorrowRequest`, `PreparedBorrowingAction`, `BorrowingReader`, and `BorrowingActionGateway`.
- Produces: `BorrowCapabilityRequest`, `BorrowCapability`, `BorrowStage`, `BorrowCapabilityUnavailableReason`, `BorrowingReader::read_borrow_capability`, `BorrowingService::borrow_capability`, `BorrowingService::supported_debt_tokens`, and `BorrowingService::minimum_health_factor`.

- [ ] **Step 1: Add the capability domain types and reader port**

Add the following public shapes in `model.rs`, using existing string serializers for `Address` and `U256` fields and `utoipa::ToSchema` for API-facing types:

```rust
pub const MINIMUM_BORROW_HEALTH_FACTOR: Decimal = Decimal::new(15, 1);

pub struct BorrowCapabilityRequest {
    pub wallet: WalletAddress,
    pub pool: Address,
    pub collateral_token: Address,
    pub debt_token: Address,
    pub raw_amount: U256,
    pub minimum_health_factor: Decimal,
}

pub enum BorrowStage {
    EnableCollateral,
    Borrow,
}

pub enum BorrowCapabilityUnavailableReason {
    PositionNotFound,
    UnsupportedPool,
    MarketUnlisted,
    CollateralUnavailable,
    BorrowPaused,
    BorrowCapReached,
    InsufficientBorrowLiquidity,
    InvalidOraclePrice,
    HealthFactorUnavailable,
    HealthFactorBelowMinimum,
}

pub struct BorrowCapability {
    pub venue: BorrowingVenue,
    pub chain_id: u64,
    pub pool: Address,
    pub collateral: BorrowingToken,
    pub debt: BorrowingToken,
    pub requested_amount: Decimal,
    pub requested_raw_amount: U256,
    pub debt_token_decimals: u32,
    pub conservative_max_amount: Decimal,
    pub available_borrow_liquidity: Decimal,
    pub borrow_apy_percent: Decimal,
    pub health_factor_before: Option<Decimal>,
    pub health_factor_after: Option<Decimal>,
    pub collateral_enabled: bool,
    pub stage: BorrowStage,
    pub unavailable_reasons: Vec<BorrowCapabilityUnavailableReason>,
    pub observed_at_ms: i64,
}
```

Add to `BorrowingReader`:

```rust
async fn read_borrow_capability(
    &self,
    request: &BorrowCapabilityRequest,
) -> Result<BorrowCapability, BorrowingProviderError>;
```

- [ ] **Step 2: Centralize policy and preparation enforcement**

Keep `BorrowingPolicy::new` as the single constructor, expose immutable accessors, and add:

```rust
pub async fn borrow_capability(
    &self,
    wallet: WalletAddress,
    pool: Address,
    collateral_token: Address,
    debt_token: Address,
    raw_amount: U256,
) -> Result<BorrowCapability, BorrowingServiceError>
```

This method must reject zero raw amount, unsupported collateral, and unsupported debt before calling the reader. It passes `MINIMUM_BORROW_HEALTH_FACTOR` in the reader request and checks that an otherwise-ready capability has `health_factor_after >= 1.5`.

Expose forwarding read-only accessors on `BorrowingService` so recipes do not
reach through the service into private policy state:

```rust
pub fn supported_debt_tokens(&self) -> &[Address]
pub const fn minimum_health_factor(&self) -> Decimal
```

After `action_gateway.prepare_borrow`, make `prepare_borrow` reject a missing or sub-floor `action.preview.health_factor_after`:

```rust
let action = self.action_gateway.prepare_borrow(request).await?;
match action.preview.health_factor_after {
    Some(value) if value >= self.policy.minimum_health_factor() => Ok(action),
    Some(_) => Err(BorrowingServiceError::UnsafeHealthFactor),
    None => Err(BorrowingServiceError::HealthFactorUnavailable),
}
```

- [ ] **Step 3: Add focused policy tests and update test doubles**

Add only these policy boundary tests:

```rust
#[test]
fn borrowing_policy_uses_the_v1_health_floor() {
    let policy = BorrowingPolicy::new(vec![DEBT])
        .unwrap_or_else(|error| panic!("policy failed: {error}"));
    assert_eq!(policy.debt_tokens(), &[DEBT]);
    assert_eq!(policy.minimum_health_factor(), Decimal::new(15, 1));
}

#[tokio::test]
async fn borrow_preparation_fails_closed_outside_health_policy() {
    for (health, expected) in [
        (Some(Decimal::new(149, 2)), BorrowingServiceError::UnsafeHealthFactor),
        (None, BorrowingServiceError::HealthFactorUnavailable),
    ] {
        let service = service_with_borrow_preview(health);
        let result = service.prepare_borrow(&PrepareBorrowRequest {
            wallet: wallet(),
            debt_token: DEBT,
            amount: Decimal::ONE,
        }).await;
        assert_eq!(result, Err(expected));
    }
}
```

Every `BorrowingReader` test double must implement `read_borrow_capability`; use `unreachable!` outside tests that exercise it so accidental calls remain visible.

- [ ] **Step 4: Run the core borrowing tests**

Run:

```bash
cargo test -p stockfront-core bstock_borrowing --lib
```

Expected: all borrowing tests pass.

- [ ] **Step 5: Commit Task 1**

```bash
git add crates/core/src/bstock_borrowing
git commit -m "feat(borrowing): enforce conservative borrow capability"
```

### Task 2: Venus live capability calculation

**Files:**
- Modify: `crates/adapters/src/venus/markets.rs`
- Modify: `crates/adapters/src/venus/risk.rs`
- Test: `crates/adapters/src/venus/risk.rs`
- Test: `crates/adapters/tests/venus_borrowing.rs`

**Interfaces:**
- Consumes: Task 1 `BorrowCapabilityRequest`, `BorrowCapability`, and unavailable reasons; existing `MarketState`, `read_binance_account`, and Comptroller `checkMembership`.
- Produces: `VenusBorrowingAdapter::read_borrow_capability` and a pure account-wide calculation helper used by that read.

- [ ] **Step 1: Implement the pure capacity calculation**

Refactor existing checked health arithmetic rather than duplicating it. The calculation must:

```text
current_weighted_collateral = sum(enabled supplied USD * liquidation threshold)
projected_weighted_collateral = current + selected pending collateral contribution
current_debt_usd = sum(all pool debt USD)
requested_debt_usd = requested amount * valid debt oracle price
HF before = current weighted collateral / current debt, or None at zero debt
HF after = projected weighted collateral / (current debt + requested debt)
safe additional debt USD = max(projected weighted collateral / 1.5 - current debt, 0)
safe token amount = safe additional debt USD / debt oracle price
conservative max = min(safe token amount, cap-and-cash liquidity)
```

Round the maximum down to debt-token decimals before comparing it to `requested_raw_amount`. Checked arithmetic failure returns `BorrowingProviderError::InvalidData`.

- [ ] **Step 2: Implement the live Venus reader**

`read_borrow_capability` must:

1. reject a pool other than the configured Comptroller with `UnsupportedPool`;
2. load the selected collateral and debt `MarketState` values;
3. read the wallet account and find a positive supplied asset matching pool and token;
4. query `checkMembership` for every supplied token in the pool;
5. build market unavailability reasons without collapsing invalid oracle, pause, cap, and liquidity states;
6. decode `raw_amount` with the debt market's decimals;
7. call the pure risk calculation, treating the selected position as projected collateral when membership is false;
8. return `EnableCollateral` when membership is false and `Borrow` when true.

Expose only the `MarketState` fields required by `risk.rs` as `pub(super)`; do not make provider details public outside the Venus module.

- [ ] **Step 3: Add focused pure risk tests**

Extract a pure calculation input that contains enabled supplied USD values with liquidation thresholds, current debt USD, debt price, requested amount, liquidity, and minimum HF. Pin at least these cases:

```rust
#[test]
fn capability_uses_all_enabled_collateral_and_selected_pending_collateral() {
    let result = calculate_borrow_capacity(fixture_with_two_enabled_assets_and_selected_pending())
        .unwrap_or_else(|error| panic!("capacity failed: {error}"));
    assert_eq!(result.health_factor_before, Some(Decimal::new(20, 1)));
    assert_eq!(result.health_factor_after, Some(Decimal::new(16, 1)));
    assert_eq!(result.stage, BorrowStage::EnableCollateral);
}

#[test]
fn capability_clamps_max_to_market_liquidity() {
    let result = calculate_borrow_capacity(fixture_with_liquidity(Decimal::new(3, 0)))
        .unwrap_or_else(|error| panic!("capacity failed: {error}"));
    assert_eq!(result.conservative_max_amount, Decimal::new(3, 0));
}

#[test]
fn one_raw_unit_over_the_safe_max_is_unavailable() {
    let result = calculate_borrow_capacity(fixture_one_unit_over_safe_max())
        .unwrap_or_else(|error| panic!("capacity failed: {error}"));
    assert!(result.unavailable_reasons.contains(
        &BorrowCapabilityUnavailableReason::HealthFactorBelowMinimum
    ));
}
```

- [ ] **Step 4: Update the ignored live preparation assertion**

Before its existing `prepare_borrow` call, have `prepares_live_collateral_activation_and_borrow` call `read_borrow_capability` and assert:

```rust
assert!(capability.unavailable_reasons.is_empty());
assert!(capability.health_factor_after.is_some_and(|value| value >= Decimal::new(15, 1)));
assert!(capability.conservative_max_amount >= BORROW_AMOUNT.parse()?);
```

Do not run the ignored funds-spending test during normal verification.

- [ ] **Step 5: Run adapter unit and non-ignored integration tests**

Run:

```bash
cargo test -p stockfront-adapters venus:: --lib
cargo test -p stockfront-adapters --test venus_borrowing -- --skip borrows_live_usdt_against_tesla_bstock_on_venus
```

Expected: unit tests pass; ignored live tests remain ignored and no transaction is sent.

- [ ] **Step 6: Commit Task 2**

```bash
git add crates/adapters/src/venus crates/adapters/tests/venus_borrowing.rs
git commit -m "feat(venus): read conservative borrow capability"
```

### Task 3: Borrow recipe language and compiler

**Files:**
- Modify: `crates/core/src/recipes/model.rs`
- Modify: `crates/core/src/recipes/plan.rs`
- Modify: `crates/core/src/recipes/compiler.rs`
- Modify: `crates/core/src/recipes/diagnostics.rs`
- Test: `crates/core/src/recipes/compiler.rs`
- Test: `crates/core/src/recipes/model.rs`

**Interfaces:**
- Consumes: existing `SuppliedPositionRef`, `AssetRef::NodeOutput`, `TokenAmount`, and Task 1 borrowing capability types.
- Produces: `RecipePrimitiveKind::Borrow`, `RecipePrimitive::Borrow`, `RecipeBorrowPolicy`, `RecipeAction::Borrow`, `RecipeNodePreview::Borrow`, `PreparedRecipeStep::{EnableCollateral,Borrow}`.

- [ ] **Step 1: Add the public language types**

Extend the primitive catalog with:

```rust
RecipePrimitive::Borrow {
    debt_token: Address,
    raw_amount: U256,
}
```

Use JSON fields `debtToken` and `rawAmount`, serialize addresses and amounts with the existing helpers, and define the descriptor with one `collateral: SUPPLIED_POSITION` input plus one `borrowed: TOKEN_AMOUNT` output.

Add:

```rust
pub struct RecipeBorrowRule {
    pub venue: BorrowingVenue,
    pub pool: Address,
    pub input: RecipeOptionSource,
}

pub struct RecipeBorrowPolicy {
    pub allowed_inputs: Vec<RecipeBorrowRule>,
    pub debt_tokens: Vec<Address>,
    pub minimum_health_factor: Decimal,
}
```

Add Borrow preview and prepared variants using Task 1 types:

```rust
RecipeNodePreview::Borrow {
    node_id: Uuid,
    state: RecipeNodeState,
    capability: Option<BorrowCapability>,
    waiting_for: Vec<Uuid>,
}

PreparedRecipeStep::EnableCollateral {
    node_id: Uuid,
    action: PreparedBorrowingAction,
}

PreparedRecipeStep::Borrow {
    node_id: Uuid,
    action: PreparedBorrowingAction,
    output: TokenAmount,
}
```

- [ ] **Step 2: Lower Borrow and register its output**

In `compiler.rs`, read the `collateral` position input, push `RecipeAction::Borrow`, and register `(node_id, "borrowed")` as a `NodeOutput` with the debt token. Validate `raw_amount != 0` in `validate_node_fields`.

Add diagnostic codes for unsupported Borrow position and unsupported Borrow debt token. Reuse `InvalidAmount` for zero amount and existing graph diagnostics for wrong or duplicated edges.

- [ ] **Step 3: Add focused compiler tests**

Create a recipe with a supplied-position source, Borrow, and a dependent Trade. Assert the logical plan contains:

```rust
RecipeAction::Borrow {
    collateral: SuppliedPositionRef {
        venue: BorrowingVenue::Venus,
        pool: POOL,
        token: BSTOCK,
    },
    debt_token: SETTLEMENT,
    raw_amount: U256::from(1_000_000_000_000_000_000_u128),
}
```

and that the dependent Trade consumes:

```rust
AssetRef::NodeOutput {
    node_id: BORROW_ID,
    port: "borrowed".to_owned(),
    token: SETTLEMENT,
}
```

Add separate tests for zero raw amount, connecting Wallet Asset to Borrow, and reusing the Borrow output twice.

- [ ] **Step 4: Run recipe model and compiler tests**

Run:

```bash
cargo test -p stockfront-core recipes::compiler::tests --lib
```

Expected: all tests pass and existing recipe JSON remains compatible.

- [ ] **Step 5: Commit Task 3**

```bash
git add crates/core/src/recipes
git commit -m "feat(recipes): compile Venus borrow actions"
```

### Task 4: Live Borrow preview, staged prepare, and OpenAPI

**Files:**
- Modify: `crates/core/src/recipes/service.rs`
- Modify: `crates/adapters/src/http/openapi.rs` only if nested discovery is incomplete
- Test: `crates/core/src/recipes/service.rs`
- Test: `crates/adapters/src/http/openapi.rs`

**Interfaces:**
- Consumes: Tasks 1-3 `BorrowingService::borrow_capability`, `RecipeAction::Borrow`, `BorrowStage`, and prepared variants.
- Produces: generated Recipe API support for catalog policy, Borrow preview, activation preparation, Borrow preparation, and exact completed output validation.

- [ ] **Step 1: Add Borrow policy to the catalog and validation**

Build `RecipeBorrowPolicy` in `RecipeService::new` from the configured Venus pool and the `BorrowingService::supported_debt_tokens` / `minimum_health_factor` accessors. Add it to `RecipeCatalog`.

Extend action-policy validation so Borrow requires:

```rust
rule.venue == collateral.venue
    && rule.pool == collateral.pool
    && rule.input == RecipeOptionSource::SuppliedPositions
    && borrow_policy.debt_tokens.contains(debt_token)
    && trade.supports_pair_shape(collateral.token, *debt_token)
```

This keeps the supported bStock set aligned with the existing catalog/trade policy without creating a duplicate allowlist.

- [ ] **Step 2: Implement live preview**

For each incomplete Borrow step, call:

```rust
self.borrowing.borrow_capability(
    request.context.wallet,
    collateral.pool,
    collateral.token,
    debt_token,
    raw_amount,
).await
```

Return `READY` only when `unavailable_reasons` is empty; otherwise return `UNAVAILABLE` with the capability so the UI can explain every live reason. A completed Borrow uses `COMPLETE` and no provider read.

- [ ] **Step 3: Implement stage-aware preparation**

Re-read capability during prepare. If unavailable reasons are non-empty, return `StepNotReady`. Otherwise:

```rust
match capability.stage {
    BorrowStage::EnableCollateral => {
        let action = self.borrowing.prepare_collateral(&PrepareCollateralRequest {
            wallet,
            collateral_token: collateral.token,
        }).await?;
        PreparedRecipeStep::EnableCollateral { node_id, action }
    }
    BorrowStage::Borrow => {
        let action = self.borrowing.prepare_borrow(&PrepareBorrowRequest {
            wallet,
            debt_token,
            amount: capability.requested_amount,
        }).await?;
        PreparedRecipeStep::Borrow {
            node_id,
            action,
            output: TokenAmount { token: debt_token, amount: raw_amount },
        }
    }
}
```

Require exactly one correctly typed transaction in either returned action. If
the capability requested activation but `prepare_collateral` returns an empty
plan because membership changed concurrently, re-read capability once and
prepare Borrow only when the fresh stage is `BORROW`; otherwise return
`StepNotReady`. This avoids emitting a no-op activation or stale Borrow.

- [ ] **Step 4: Validate exact completed output semantics**

Extend `validate_completed_context` so Borrow requires exactly one output:

```rust
output.port == "borrowed"
    && output.asset.token == debt_token
    && output.asset.amount == raw_amount
```

Activation has no separate logical node and can never appear in `completedNodes`. Include Borrow's `NodeOutput` in the existing dependency check so a dependent Trade remains blocked until exact Borrow completion.

- [ ] **Step 5: Add focused recipe-service tests**

Extend `TestBorrowing` to record capability, activation, and Borrow requests. Add focused tests for:

```text
preview + disabled membership -> READY, stage ENABLE_COLLATERAL
prepare + disabled membership -> one EnableCollateral step
activation in completedNodes -> InvalidContext
preview + enabled membership -> READY, stage BORROW
prepare + enabled membership -> one Borrow step with exact TokenAmount output
capability with HF below 1.5 -> UNAVAILABLE and no prepared step
completed Borrow wrong token/amount -> InvalidContext
completed Borrow exact token/amount -> dependent Trade becomes READY
unsupported pool/debt -> validation failure before capability read
```

Add one extra test for the activation race: capability says activation, the
gateway returns an empty activation plan, a second capability read says Borrow,
and preparation returns exactly one Borrow step.

- [ ] **Step 6: Run recipe-service tests**

Run:

```bash
cargo test -p stockfront-core recipes::service::tests --lib
```

Expected: all recipe-service tests pass.

- [ ] **Step 7: Verify OpenAPI discovery**

Run the existing OpenAPI test and inspect the Recipe catalog/preview/prepared schemas. Add explicit `components(schemas(...))` entries only for types that Utoipa does not discover through handler response roots.

Run:

```bash
cargo test -p stockfront-adapters http::openapi --lib
```

Expected: the document includes `BORROW`, `ENABLE_COLLATERAL`, borrow policy, capability, and prepared Borrow output.

- [ ] **Step 8: Run the complete backend gate**

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

Expected: formatting, Clippy, unit tests, and non-ignored integrations pass. No live funds-spending test runs.

- [ ] **Step 9: Commit Task 4**

```bash
git add crates/core/src/recipes crates/adapters/src/http
git commit -m "feat(recipes): prepare staged Venus borrows"
```

## Backend acceptance check

After Task 4, verify the working tree contains no frontend change:

```bash
test -z "$(git status --short | grep '^.. fe/')"
git log -4 --oneline
```

The backend slice is ready for frontend implementation only when:

- every new public response type is present in OpenAPI;
- preview distinguishes activation from Borrow without preparing stale calldata;
- preparation always re-reads capability;
- direct and recipe Borrow paths both enforce HF `1.5`;
- only confirmed Borrow can publish the exact stablecoin output;
- the complete backend gate passes.
