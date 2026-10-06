# Venus Borrow Recipes Design

## Goal

Allow a user to start a fresh recipe from a supported bStock position already
supplied to Venus, borrow an exact stablecoin amount against the wallet's pooled
Venus account, and pass the confirmed borrowed asset to a later recipe action.

The supply may have been created by any earlier recipe or outside Stockfront.
The borrow recipe depends only on current onchain and indexed account state; it
does not depend on the saved draft or execution journal of the supply operation.

## Scope

The first slice supports this graph:

```text
Supplied position -> Borrow -> optional later token-consuming action
```

It supports BNB Chain, the configured Venus Core Pool, Stockfront-supported
bStock collateral, configured stablecoin debt assets, an exact user-entered
borrow amount, and a backend-enforced projected health-factor floor of `1.5`.

The Borrow action may contain two user-authorized stages:

1. enable the selected supplied market as collateral, when it is not enabled;
2. borrow the stablecoin after collateral activation is confirmed.

Percentage amounts, a `Borrow max` action, automatic signatures, atomic
collateral-enable-plus-borrow batching, repayment, deleveraging, and borrowing
from a supply output within the same recipe are outside this slice.

## Design decisions

### Existing positions are first-class sources

`SUPPLIED_POSITION` remains the source. The backend resolves its venue, pool,
and token against the connected wallet every time it previews or prepares.
This makes positions left by another recipe immediately usable without copying
state from the earlier recipe.

### Risk is account-wide

Venus Core Pool debt is not isolated to one collateral asset. The selected
position identifies the market Stockfront may enable, but borrow capacity and
health factor are calculated from the complete wallet account: every enabled
collateral asset and every outstanding debt asset in the configured pool.

The UI must not describe the debt as isolated to the selected bStock.

### Activation and borrowing are resumable stages

Stockfront does not batch collateral activation and borrowing in v1. A borrow
simulation performed before activation does not see the state created by an
unconfirmed `enterMarkets` call. Treating the calls as one prepared result would
therefore weaken simulation and stale-state guarantees.

When activation is required, preparation returns only that action. After its
receipt succeeds, the client refreshes account state and asks the backend to
preview and prepare again. No second transaction is signed automatically.

### The health-factor floor is authoritative

`1.5` is a hard preparation policy, not a display warning. The backend rejects
any borrow whose freshly calculated projected health factor is missing or below
the floor. The same policy applies to the direct borrowing service so the recipe
route cannot be bypassed through another HTTP handler.

## Reused capabilities

- `GET /v1/wallets/{address}/loans` remains the account-position read model used
  by the source selector.
- `BorrowingService::prepare_collateral` remains the collateral-activation use
  case and already avoids a duplicate transaction when membership is enabled.
- `BorrowingService::prepare_borrow` remains the borrow use case.
- `VenusBorrowingAdapter` already reads market state, checks membership, encodes
  `enterMarkets` and `borrow`, simulates unsigned actions, and calculates
  account-wide health factors using liquidation thresholds.
- The existing recipe compiler, preview/prepare boundary, wallet-call journal,
  transaction reconciliation, manual retry, and 15-second status polling remain
  responsible for orchestration and recovery.

No new protocol, provider, endpoint family, frontend DTO, or dependency is
required. The generated OpenAPI contract remains the frontend source of truth.

## Borrowing capability model

The read side gains one domain model for an amount-specific borrow capability.
It is produced from current account, market, oracle, cap, liquidity, and
membership state and contains:

- venue, pool, selected collateral token, and debt token;
- the requested raw token amount and its decimal display value;
- debt-token decimals, borrow APY, and available borrow liquidity;
- current and projected health factor;
- conservative maximum amount at the configured health-factor floor;
- whether the selected collateral is currently enabled;
- observation time and typed unavailable reasons.

The conservative maximum is the non-negative additional debt permitted at a
projected health factor of `1.5`, divided by the debt token's current valid
oracle price, and clamped to current borrow-cap headroom and cash liquidity.
Arithmetic overflow, a missing market, an invalid price, or incomplete account
data makes the capability unavailable rather than producing a guessed value.

For the capability calculation only, the selected collateral is treated as
enabled when calculating the state after the activation stage. Preparation of
the borrow transaction still requires confirmed onchain membership.

The borrowing adapter owns protocol-specific reads. The core borrowing service
owns allowlists, positive-amount validation, the health-factor floor, and the
decision between activation and borrow stages.

## Recipe language

Add one primitive kind:

```json
{
  "id": "<uuid>",
  "kind": "BORROW",
  "debtToken": "<supported-stablecoin-address>",
  "rawAmount": "1000000000000000000"
}
```

The user edits a normal decimal token amount. The saved recipe stores its
positive raw integer value as a string, matching Wallet Asset semantics and
making the expected runtime output exact. The generated catalog provides the
allowed debt assets and the frontend uses their token decimals only to translate
the user's display value. During preview and prepare, the backend checks that
translation against the authoritative Venus debt-market decimals before it
converts the raw value into the existing `PrepareBorrowRequest` decimal amount.

`BORROW` has:

- one required `collateral` input of value type `SUPPLIED_POSITION`;
- one optional-consumer `borrowed` output of value type `TOKEN_AMOUNT`.

The source position may have one consumer in this version, preventing one draft
from concurrently borrowing and withdrawing against the same source reference.
The borrow output behaves like a trade output and may feed a later Trade or
future Earn action after the borrow receipt is confirmed.

The primitive catalog adds a borrow policy containing:

- allowed venue and pool;
- allowed debt-token addresses;
- minimum projected health factor `1.5`;
- `SUPPLIED_POSITIONS` as the allowed input source.

Frontend filtering improves the editing experience, but compiler and service
policy remain authoritative.

## Compiler and logical plan

The pure compiler lowers the source to the existing:

```text
SuppliedPositionRef { venue, pool, token }
```

and lowers Borrow to:

```text
RecipeAction::Borrow {
    collateral: SuppliedPositionRef,
    debt_token,
    raw_amount,
}
```

It registers the node's `borrowed` output as a `NodeOutput` carrying the debt
token. A dependent action waits until runtime context contains the confirmed
raw token amount.

Pure validation rejects a zero or negative amount, unsupported port shapes,
missing input, and graph reuse. Policy validation rejects an unsupported venue,
pool, collateral token, or debt token before any provider call.

Completed runtime context for Borrow must contain exactly one output on
`borrowed`, with the configured debt token and exactly the configured positive
raw amount. An activation receipt never satisfies this output and therefore
cannot complete the logical node.

## Preview

Preview resolves the selected position from the live wallet account, then reads
the amount-specific borrowing capability.

A ready response contains:

- `stage`: `ENABLE_COLLATERAL` or `BORROW`;
- the live supplied position;
- the selected debt asset and requested amount;
- conservative maximum, liquidity, and borrow APY;
- current and projected health factors;
- observation time.

An unavailable preview contains one or more typed reasons:

- `POSITION_NOT_FOUND`
- `UNSUPPORTED_POOL`
- `MARKET_UNLISTED`
- `COLLATERAL_UNAVAILABLE`
- `BORROW_PAUSED`
- `BORROW_CAP_REACHED`
- `INSUFFICIENT_BORROW_LIQUIDITY`
- `INVALID_ORACLE_PRICE`
- `HEALTH_FACTOR_UNAVAILABLE`
- `HEALTH_FACTOR_BELOW_MINIMUM`

Unsupported collateral, unsupported debt tokens, and zero amounts are invalid
recipe definitions. The compiler or policy validator reports them before live
preview rather than representing them as changing market availability.

The response may expose several market reasons together when they describe the
same snapshot. It never treats stale preview data as authorization.

## Prepare and execution

Prepare repeats position, account, market, membership, oracle, amount,
liquidity, cap, and health-factor checks.

When activation is required, the prepared recipe contains exactly one step:

```text
ENABLE_COLLATERAL { nodeId, action }
```

This step belongs to the Borrow node but does not complete it and has no token
output. A successful receipt invalidates the borrowing-account and capability
queries. The next preview must observe confirmed membership before it exposes
the Borrow stage.

When membership is enabled and the borrow is safe, preparation contains exactly
one step:

```text
BORROW { nodeId, action, output: TokenAmount }
```

The prepared output contains the configured debt token and the validated raw
amount from the recipe. It is recorded in completed runtime context only after
the Borrow receipt succeeds.

If membership changes between preview and prepare, prepare returns the stage
currently required. If risk or market state changes, prepare rejects or returns
an updated unavailable reason; it never reuses an earlier capability snapshot.

## Frontend behavior

The user starts a new recipe, selects a live supported supplied position, and
adds Borrow. The canvas displays:

```text
[Supplied TSLAB position] -> [Borrow 1 USDT] -> Asset<USDT> -> [+]
```

The Borrow inspector provides:

- a debt-asset selector limited by generated borrow policy;
- an exact amount field;
- existing supplied value and total account debt;
- conservative maximum at health factor `1.5`;
- current and projected health factor;
- borrow APY and available liquidity;
- the currently required action button.

Amounts above the conservative maximum are disabled before wallet interaction.
Changing the position, debt token, or amount invalidates preview and prepared
state.

Activity presents one logical action with resumable stages:

```text
✓ Enable TSLAB collateral
● Borrow 1 USDT
```

The second stage appears only after refreshed live state confirms activation.
Successful activation remains visible when Borrow is retried. A failed Borrow
does not offer another activation transaction. A successful Borrow completes
the node, publishes the stablecoin output, and refreshes wallet balances and the
Venus account.

## Recovery and security

- The wallet comes only from `RecipeContext`; the saved recipe cannot redirect
  borrowed funds.
- Venue, pool, collateral, and debt token are checked against backend policy on
  validation, preview, and prepare.
- The backend validates the recipe's raw token units against authoritative
  market decimals before constructing the domain borrow request.
- Preview is informational. Prepare re-reads every state value used by the
  safety decision.
- Missing or invalid oracle data fails closed.
- Projected health factor below `1.5` fails before wallet submission.
- Market pause, borrow cap, or insufficient cash fails before submission.
- Activation and Borrow are never automatically signed or replayed.
- Dropped, reverted, rejected, and uncertain submissions reuse the existing
  explicit reconciliation and fresh-prepare retry flow.
- Journal recovery distinguishes activation from Borrow; only a confirmed
  Borrow publishes output or unlocks a dependent node.
- Reloading after activation uses onchain membership as truth even if the local
  activation journal entry is missing.

## Tests

Backend tests protect these behaviors:

- the compiler accepts `SuppliedPosition -> Borrow` and publishes the typed
  output;
- wrong port types, unsupported pool, unsupported collateral, unsupported debt,
  and invalid amounts fail before provider work;
- capability calculation includes the wallet's complete enabled account,
  includes the selected collateral hypothetically when activation is required,
  clamps to cap and liquidity, and rejects invalid oracle data;
- preview distinguishes activation and Borrow stages;
- prepare re-reads state, returns one activation action when required, and one
  Borrow action with canonical output after membership is confirmed;
- a projected health factor below `1.5` is rejected by the borrowing service;
- activation cannot complete the node, while a confirmed Borrow output can
  unlock one dependent token action;
- generated OpenAPI includes the policy, preview, and prepared-step variants.

Frontend tests protect policy filtering, amount validation, stage transitions,
stale-preview invalidation, canonical completed output, and journal recovery.
One browser flow covers activation, live refresh, Borrow, and publication of the
stablecoin output. The existing ignored live Venus borrow test remains the
funds-spending adapter acceptance test; a recipe-level live test may reuse it
without introducing another default funds-spending test.

## Acceptance criteria

1. A connected wallet can select a supported Venus bStock position supplied by
   an earlier recipe or external transaction.
2. The recipe validates as `Supplied position -> Borrow` with an exact supported
   stablecoin amount.
3. Preview shows account-wide current/projected health, conservative maximum,
   liquidity, APY, and the required stage.
4. If necessary, one user confirmation enables the selected collateral and the
   recipe remains incomplete.
5. After confirmed activation, refreshed state enables a separately authorized
   Borrow transaction.
6. Preparation cannot produce a Borrow whose projected health factor is missing
   or below `1.5`.
7. Confirmed Borrow publishes the exact stablecoin output and can unlock a later
   token-consuming action.
8. Stale, unsupported, paused, capped, illiquid, oracle-invalid, reverted,
   dropped, and uncertain states cannot silently submit or replay a transaction.
