# Venus Repayment Recipes Design

## Goal

Allow a user to start a fresh recipe from debt left by an earlier Venus recipe,
repay that debt completely from the connected wallet, and then start a separate
full collateral-withdrawal recipe from refreshed onchain account state.

## Scope

This slice supports two recipes separated by confirmed settlement:

```text
Borrowed position -> Repay full debt

refresh Venus account

Supplied position -> Withdraw all
```

Repayment supports BNB Chain, the configured Venus Core Pool, and the debt
tokens already allowed by Stockfront's borrowing policy. V1 is full repayment
only. Partial repayment, repayment from an upstream trade output, automatic
withdrawal after repayment, and debt-bearing partial withdrawal are outside the
slice.

The existing full-withdrawal primitive remains unchanged: it is unavailable
while any Venus debt exists and becomes ready only after a fresh account read
confirms zero debt.

## Design decisions

### Debt is a first-class recipe source

Add `BORROWED_POSITION`, parallel to `SUPPLIED_POSITION`. It identifies a live
debt by venue, pool, and underlying debt token. It never stores a wallet,
borrowed amount, interest snapshot, or prepared calldata.

The backend resolves the source against the connected wallet on every preview
and prepare. A debt created by Stockfront, another application, or an earlier
recipe is therefore usable without carrying execution state between recipes.

### V1 repays the complete debt

Venus debt accrues between preview, preparation, wallet confirmation, and
mining. Converting a displayed debt snapshot into an exact repayment can leave
interest dust and keep collateral withdrawal blocked. Full repayment uses
Venus's `repayBorrow(type(uint256).max)` behavior and is the only mode in this
slice.

The approval is finite. Its amount is derived from the current debt plus the
existing small interest buffer; the token approval is never unlimited.

### Repayment and withdrawal remain separate recipes

Repayment and withdrawal are separate onchain state transitions. A withdrawal
must use account state observed after the repayment receipt succeeds. The
builder does not add a generic control edge or pretend that an unconfirmed
repayment has removed debt.

After repayment settles, the UI offers `Start collateral withdrawal`. It opens
a clean `SUPPLIED_POSITION -> WITHDRAW` recipe using the refreshed position.
The user still previews, prepares, and confirms that withdrawal explicitly.

## Reused capabilities

- `GET /v1/wallets/{address}/loans` remains the account read model. Its
  `borrowed` assets populate the new source selector.
- `PositionAmount::Full`, repayment approval encoding, `repayBorrow(MAX)`, and
  full withdrawal encoding already exist in the borrowing domain and Venus
  adapter.
- The existing recipe compiler, preview/prepare boundary, generated OpenAPI
  client, wallet-call journal, receipt reconciliation, and manual retry flow
  remain the orchestration path.
- The existing withdrawal rule that rejects `OUTSTANDING_DEBT` remains the
  authoritative collateral safety boundary.

No new provider, persistence store, execution journal, protocol SDK, or
handwritten frontend API type is introduced.

## Repayment capability

The borrowing domain gains a live `RepaymentCapability` for one selected debt:

- venue, chain, pool, and debt token;
- current debt amount and wallet token balance;
- the finite approval amount required for full repayment;
- whether approval is currently required;
- observation time;
- typed unavailable reasons.

Unavailable reasons are:

- `POSITION_NOT_FOUND`
- `UNSUPPORTED_POOL`
- `UNSUPPORTED_DEBT_TOKEN`
- `NO_DEBT`
- `INSUFFICIENT_WALLET_BALANCE`

The Venus adapter owns protocol reads for debt, ERC-20 wallet balance, and
allowance. The core borrowing service owns the pool/token allowlist and decides
whether the capability is ready. Missing or inconsistent provider data is an
error, not a zero balance.

## Recipe language

Add a source primitive:

```json
{
  "id": "<uuid>",
  "kind": "BORROWED_POSITION",
  "venue": "VENUS",
  "pool": "<configured-core-pool>",
  "token": "<supported-debt-token>"
}
```

Add an action primitive:

```json
{
  "id": "<uuid>",
  "kind": "REPAY"
}
```

`BORROWED_POSITION` exposes one consumable `debt` output with the new
`BORROWED_POSITION` port type. `REPAY` requires exactly one `debt` input and has
no output. The absence of an amount field makes the full-repayment promise
explicit and prevents a stale debt snapshot from becoming recipe data.

The catalog adds a repayment policy containing allowed venue/pool pairs and
supported debt-token addresses. Frontend filtering improves editing, while
compiler policy remains authoritative.

## Compiler and runtime context

The compiler lowers the source to:

```text
BorrowedPositionRef { venue, pool, token }
```

and lowers Repay to:

```text
RecipeAction::Repay { debt: BorrowedPositionRef }
```

The graph checker prevents wallet assets, trade outputs, and supplied positions
from connecting to Repay. Policy validation rejects unsupported pools and debt
tokens before any provider request.

Repay is terminal and produces no runtime output. A completed Repay node must
therefore contain an empty output list. Completion records only a confirmed
successful wallet operation; it does not cache the debt amount.

## Preview and prepare

Preview resolves the debt against the live account and returns the repayment
capability. The node is ready only when the debt exists, the pair is supported,
and the connected wallet balance covers the buffered full repayment amount.

Prepare repeats all account, debt, wallet-balance, allowance, and policy reads.
It never reuses the preview capability as authorization.

Preparation returns one Repay recipe step containing an ordered borrowing
action:

```text
[APPROVE finite buffered amount, REPAY full debt]
```

when approval is required, or:

```text
[REPAY full debt]
```

when allowance is already sufficient. The borrowing gateway prepares the plan
as one unit so it does not reject the repayment merely because the approval in
the same plan is not mined yet. Existing standalone repayment endpoints retain
their current behavior.

The frontend submits the ordered calls through the existing atomic wallet-call
path. If the wallet cannot provide atomic call semantics, preparation is not
silently downgraded to independently replayable calls. Rejected, dropped, or
reverted batches use the existing fresh-prepare manual retry flow.

## Frontend behavior

The palette exposes `Borrowed position` as a source and `Repay debt` as an
action. The source selector lists only live borrowed assets allowed by the
repayment policy.

The canvas displays:

```text
[USDT debt on Venus] -> [Repay full debt]
```

The inspector displays current debt, wallet balance, finite approval amount,
and whether the batch contains approval plus repayment. It does not offer an
editable amount or a misleading percentage control.

Activity shows approval and repayment as separate calls inside one atomic
operation. After confirmed completion, account and wallet queries are
invalidated. `Start collateral withdrawal` is shown only after a refreshed
account reports zero total debt and at least one supported supplied position.

## Errors and security

- The wallet comes only from `RecipeContext`.
- Pool and token identity are checked against backend-owned policy.
- Debt, balance, allowance, and approval amount are refreshed during prepare.
- Approval is finite and buffered only enough to cover short-term interest.
- Insufficient wallet balance fails before a wallet request is opened.
- Prepared repayment calls are never persisted or replayed automatically.
- A successful approval without a successful repayment does not mark the node
  complete.
- Collateral withdrawal remains blocked until a later account read confirms
  zero debt.

## Minimal tests

Backend tests cover:

- compiler acceptance of `BORROWED_POSITION -> REPAY` and rejection of other
  port shapes;
- policy rejection of unsupported pools and debt tokens;
- ready, missing-debt, and insufficient-wallet-balance capabilities;
- prepare ordering for `[APPROVE, REPAY]` and `[REPAY]`;
- empty completed outputs for Repay;
- a refreshed zero-debt account making the existing full withdrawal ready.

Frontend tests cover source filtering, Repay serialization, the live capability
card, prepared batch validation, and the successful withdrawal handoff. Tests
remain focused on contract and state-transition risks rather than duplicating
presentational details.

## Acceptance criteria

1. A wallet with supported Venus debt can select that live debt in a fresh
   recipe.
2. The only valid V1 graph is `Borrowed position -> Repay full debt`.
3. Preview blocks missing debt, unsupported positions, and insufficient wallet
   balance before wallet interaction.
4. Prepare returns a fresh finite approval plus full repayment, or repayment
   alone when allowance is already sufficient.
5. The wallet receives one atomic call request and successful settlement marks
   Repay complete.
6. The account is refreshed after settlement and confirms zero debt.
7. The user can start a clean full-withdrawal recipe from a remaining supplied
   position.
8. Withdrawal cannot be prepared while refreshed account debt is non-zero.
