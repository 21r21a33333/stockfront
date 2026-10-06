# Venus Repayment Recipes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` and implement this plan natively, backend first.
> Do not dispatch subagents unless the user explicitly requests them.

**Goal:** Add a full Venus debt-repayment recipe backed by live debt state and
an atomic finite-approval-plus-repayment wallet batch, then hand the user into a
fresh full-collateral-withdrawal recipe.

**Architecture:** Introduce a live borrowed-position reference and repayment
capability in the borrowing domain. The Venus adapter reads debt, balance, and
allowance and prepares one ordered action; the recipe layer exposes that action
through typed compile, preview, and prepare contracts. The frontend is generated
from OpenAPI, submits multi-call repayment only with atomic wallet support, and
refreshes account state before offering withdrawal.

**Tech Stack:** Rust 2024, Axum, Alloy, rust_decimal, Utoipa, React 18,
TypeScript, TanStack Query, Zustand, viem/wagmi, Vitest.

**Spec:** `docs/superpowers/specs/2026-10-06-venus-repayment-recipes-design.md`

## Global Constraints

- BNB Chain and the configured Venus Core Pool only.
- V1 repayment is full-only; no editable or percentage amount.
- Supported debt tokens come from backend policy.
- Preview never authorizes prepare; prepare re-reads debt, balance, and
  allowance.
- Approval is finite and uses the existing interest buffer.
- Approval plus repayment requires atomic wallet calls; never downgrade it to
  two independently replayable submissions.
- Repay produces no recipe output and completes only after the wallet batch is
  confirmed.
- Withdrawal remains a separate recipe and remains blocked until refreshed
  total debt is zero.
- Rust remains the API source of truth; frontend request/response types are
  generated.
- Do not use a red-green TDD loop. Add only the focused tests below after the
  implementation they protect.

## Review Focus

- Debt accrues after preview: prepare refreshes and uses the refreshed buffered
  amount.
- Allowance is below the buffer: the prepared call order is exactly Approve,
  Repay and the frontend requires atomic execution.
- Wallet balance is below the buffered debt: preview is unavailable and prepare
  emits nothing.
- The selected debt disappeared: preview reports `NO_DEBT` and prepare fails
  closed.
- Repayment settles but indexed debt is still non-zero: withdrawal handoff is
  not shown.

## File Map

Backend:

- `crates/core/src/bstock_borrowing/{model,ports,service,mod}.rs`: repayment
  capability and full-plan use case.
- `crates/adapters/src/venus/{contracts,mod,repayment}.rs`: Venus reads and
  ordered call construction.
- `crates/core/src/recipes/{model,plan,compiler,graph,service,mod}.rs`: borrowed
  source, Repay action, policy, preview, and prepare.
- `crates/adapters/src/http/openapi.rs`: generated schema visibility test.

Frontend:

- `fe/openapi.json`, `fe/src/api/generated.ts`: generated contracts.
- `fe/src/recipe/editor/{types,store,compiler,flowInput,repaymentPolicy}.ts`:
  borrowed-position graph and policy helpers.
- `fe/src/recipe/{ActionPalette,RecipeBuilderPage}.tsx` and
  `canvas/RecipeNodes.tsx`: source/action presentation.
- `fe/src/recipe/inspector/{BorrowedPositionFields,RepayFields,RecipeInspector}.tsx`:
  selection and live repayment state.
- `fe/src/recipe/execution/{preparedExecution,model,ExecutionPanel}.ts*`:
  atomic repayment and post-settlement withdrawal handoff.

---

### Task 1: Complete the backend repayment slice

**Interfaces produced:**

```rust
pub struct RepaymentCapabilityRequest {
    pub wallet: WalletAddress,
    pub pool: Address,
    pub debt_token: Address,
}

pub struct RepaymentCapability {
    pub venue: BorrowingVenue,
    pub chain_id: u64,
    pub pool: Address,
    pub debt: BorrowingToken,
    pub debt_amount: Decimal,
    pub wallet_balance: Decimal,
    pub approval_amount: Decimal,
    pub approval_required: bool,
    pub unavailable_reasons: Vec<RepaymentCapabilityUnavailableReason>,
    pub observed_at_ms: i64,
}

async fn read_repayment_capability(
    &self,
    request: &RepaymentCapabilityRequest,
) -> Result<RepaymentCapability, BorrowingProviderError>;

async fn prepare_full_repayment(
    &self,
    request: &PrepareRepaymentRequest,
) -> Result<PreparedBorrowingAction, BorrowingProviderError>;
```

- [ ] Add `RepaymentCapability`, request, and unavailable reasons to the
  borrowing model and exports. Add reader and action-gateway methods without
  changing the existing standalone repayment methods.
- [ ] Add `IERC20::balanceOf` and create `venus/repayment.rs`. Read the current
  vToken debt with `borrowBalanceCurrent`, ERC-20 wallet balance, and allowance;
  compute the existing finite buffered amount from one coherent snapshot per
  operation. Prepare must obtain its own fresh snapshot rather than reuse the
  preview result.
- [ ] Build `[APPROVE, REPAY]` when allowance is below the buffer and `[REPAY]`
  otherwise. Encode Repay as `repayBorrow(U256::MAX)`. The approval call may use
  the existing single-call preparer; the dependent Repay call in an atomic plan
  carries validated `to/data/value` and lets the wallet estimate the sequential
  batch. A repayment-only plan continues through normal server simulation.
- [ ] Make `BorrowingService::repayment_capability` and
  `prepare_full_repayment` enforce supported debt tokens, `PositionAmount::Full`,
  non-zero debt, and wallet balance at least equal to the refreshed buffer.
- [ ] Extend the recipe language with:

```rust
RecipePrimitive::BorrowedPosition { venue, pool, token }
RecipePrimitive::Repay
RecipePortValueType::BorrowedPosition
RecipeAction::Repay { debt: BorrowedPositionRef }
RecipeNodePreview::Repay { node_id, state, capability }
PreparedRecipeStep::Repay { node_id, action }
```

- [ ] Add `RecipeRepayPolicy { allowed_inputs, debt_tokens }` to the catalog.
  Lower only `BORROWED_POSITION.debt -> REPAY.debt`; validate venue, pool, and
  token before provider calls; accept only empty completed outputs.
- [ ] Preview with the live capability. Prepare only ready Repay nodes, call
  `prepare_full_repayment`, and validate that the returned transaction kinds
  are exactly `[APPROVE, REPAY]` or `[REPAY]`.
- [ ] Add four focused backend tests: valid compile/policy; unavailable missing
  debt or insufficient balance; prepared transaction ordering; completed Repay
  rejects outputs while a refreshed zero-debt account makes Withdraw ready.
- [ ] Ensure Utoipa exports all nested types and the loans path still exposes
  borrowed account positions.
- [ ] Verify backend:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

- [ ] Commit the complete backend boundary:

```bash
git add crates/core/src/bstock_borrowing crates/core/src/recipes \
  crates/adapters/src/venus crates/adapters/src/http/openapi.rs
git commit -m "feat(recipes): add full Venus debt repayment"
```

### Task 2: Add the borrowed-position and Repay builder UI

**Interfaces consumed:** Generated `RecipeRepayPolicy`,
`RepaymentCapability`, `BORROWED_POSITION`, `REPAY`, and prepared Repay types.

- [ ] Run API generation and commit only generated results produced from Rust:

```bash
cd fe
pnpm codegen
```

- [ ] Add `BorrowedPositionNodeData` and `RepayNodeData`, their React Flow node
  types, compiler serialization, restoration, defaults, and store append/connect
  behavior. Add a small `repaymentPolicy.ts` that derives selectable live debts
  from `BorrowingAccount.positions[].borrowed` and catalog policy.
- [ ] Add palette and canvas presentation for `Borrowed position` and `Repay
  debt`. The edge guide must read `Debt<USDT · All>` and the Repay node must not
  expose an amount input.
- [ ] Add `BorrowedPositionFields` for selecting a live supported debt and
  `RepayFields` for current debt, wallet balance, finite approval, readiness,
  and `Prepare full repayment`.
- [ ] Pass repayment policy and borrowed positions through
  `RecipeBuilderPage -> RecipeInspector`. Keep unsupported or absent debts out
  of the selector while preserving backend diagnostics as authoritative.
- [ ] Add two focused frontend tests: editor compilation of the approved graph,
  and inspector behavior for ready versus insufficient-balance capability.
- [ ] Verify the editor slice:

```bash
pnpm format
pnpm typecheck
pnpm test -- src/recipe/editor/compiler.test.ts src/recipe/inspector/RecipeInspector.test.tsx
```

### Task 3: Execute repayment atomically and hand off to withdrawal

- [ ] Extend `PreparedExecution` with `REPAY`. Accept only ordered
  `[APPROVE, REPAY]` or `[REPAY]` borrowing transactions. Produce one call batch
  and set `atomicRequired` only for the two-call form.
- [ ] Update execution labels, progress counts, failure retry copy, and runtime
  completion so Repay has no output and is complete only when the wallet call
  batch succeeds.
- [ ] Reuse the existing atomic-capability gate and `forceAtomic: true`; do not
  send either call if the wallet lacks atomic support.
- [ ] After a confirmed Repay run, invalidate/refetch wallet and borrowing
  queries. Show `Start collateral withdrawal` only when refreshed total debt is
  zero and a supported supplied position remains. Its handler starts a new run
  and replaces the board with exactly `SUPPLIED_POSITION -> WITHDRAW`.
- [ ] Add two focused execution tests: prepared repayment call ordering and the
  atomic-support gate. Add one interaction test for the zero-debt withdrawal
  handoff; do not snapshot presentational markup.
- [ ] Run the complete frontend gate:

```bash
pnpm gate
```

- [ ] Commit the frontend boundary:

```bash
git add fe
git commit -m "feat(recipes): add Venus repayment builder"
```

### Task 4: Manual live verification

- [ ] Start backend with the configured Stockfront environment and start Vite
  on `127.0.0.1`.
- [ ] With a wallet that has Venus USDT debt, verify the selector shows the debt
  and the preview shows current debt, balance, and approval state.
- [ ] Execute the full repayment. Confirm the wallet presents one atomic request
  when approval is needed and the activity panel reaches complete.
- [ ] Confirm the refreshed account has zero debt, start the offered withdrawal
  recipe, preview it, and stop before spending unless the user explicitly asks
  to execute the withdrawal.

## Self-Review

- The plan covers every spec acceptance criterion.
- Backend and frontend changes are separated by an explicit backend commit.
- No handwritten frontend DTO duplicates a generated Rust contract.
- The plan adds no generic control-edge system and no partial repayment mode.
- The five review-focus risks each have a named implementation check or test.
