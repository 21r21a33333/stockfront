# Earn Deposit Recipes Design

## Goal

Add a compiler-backed `EARN_DEPOSIT` action that deposits a supported stablecoin
into a reviewed earn opportunity. The stablecoin can come directly from the
connected wallet or from a confirmed `BORROW` node in the same recipe.

The first slice is deliberately terminal:

```text
Wallet asset -> Earn deposit

Supplied position -> Borrow stablecoin -> Earn deposit
```

An earn deposit produces an onchain position, not a freely spendable token.
That position is discovered from refreshed account state after settlement.
Earn withdrawal and delayed redemption claims remain separate recipe slices.

## Design decisions

### Reuse the carry domain

`CarryService` remains the only owner of reviewed investment IDs, minimum
spread policy, current earn opportunities, and deposit preparation. Recipe code
only adds graph semantics and orchestration around that service. It does not
copy provider rules, call Binance Web3 directly, or introduce a second earn
model.

`RecipeService` receives the existing `CarryService` as a dependency. The app
wires the same service instance into the standalone carry endpoints and recipe
endpoints.

### Preserve the collateral benchmark

An earn route is evaluated against the cost of borrowing the deposited asset
from a Venus collateral market. The recipe must therefore retain both the
stablecoin being deposited and the collateral used for that comparison.

- When the input is a `BORROW` output, the compiler derives the collateral
  token from the connected supplied position. The configured collateral field
  must match it and is locked in the UI.
- When the input is a wallet asset, there is no upstream loan. The user selects
  one supported bStock as the comparison collateral. This is a market
  benchmark only; the recipe does not claim that the wallet owns or supplies
  that collateral.

The backend validates this relationship. A client cannot provide an unrelated
collateral token to obtain a misleading spread.

### Preview is informative; prepare is authoritative

Preview shows current eligible opportunities. Prepare refreshes the selected
investment, its earn rate, the matching Venus market, policy qualification, and
the exact input amount. Preview data is never reused as authorization or
calldata.

## Recipe language

Add one action primitive:

```json
{
  "id": "<uuid>",
  "kind": "EARN_DEPOSIT",
  "collateralToken": "<supported-bstock-address>",
  "investmentId": "<reviewed-provider-investment-id>"
}
```

`EARN_DEPOSIT` has one consumable `asset` input of type `TOKEN_AMOUNT` and no
output. The accepted graph shapes are:

```text
WALLET_ASSET.asset -> EARN_DEPOSIT.asset
BORROW.asset       -> EARN_DEPOSIT.asset
```

Trade outputs and other token-producing actions are outside this slice. This
keeps the first version aligned with the two supported product journeys and
avoids implying that arbitrary transient assets are eligible earn deposits.

The catalog adds:

- the `EARN_DEPOSIT` descriptor;
- a collateral field sourced from supported bStocks;
- an investment field whose choices come from live carry opportunities;
- an earn-deposit policy listing the allowed input primitive kinds.

The catalog exposes structural policy only. Reviewed investment IDs and
qualification remain backend-owned live data and are not treated as a static
frontend allowlist.

## Compiler and runtime model

The compiler lowers the action to:

```text
RecipeAction::EarnDeposit {
    asset: AssetRef,
    collateral_token: Address,
    investment_id: String,
}
```

Compilation rejects:

- missing or multiple input edges;
- an input not produced by `WALLET_ASSET` or `BORROW`;
- a token outside the supported settlement-token set;
- an unsupported collateral token;
- an empty investment ID;
- a collateral token that differs from the upstream Borrow collateral.

A wallet input carries its exact configured raw amount. A Borrow input resolves
only from the exact output recorded for a successfully completed upstream node.
Until that output exists, preview reports `WAITING_FOR_UPSTREAM` and prepare
returns `StepNotReady`.

The action is terminal. A completed Earn Deposit node must have an empty output
list. Position balances are never synthesized from a receipt or cached as node
outputs.

## Preview

For a resolved input, `RecipeService` asks `CarryService::opportunities` for the
configured collateral and input stablecoin. The selected investment is ready
only when it is returned with `ELIGIBLE` qualification. An absent, unreviewed,
unsupported, or low-spread investment is unavailable with a typed reason.

The Earn Deposit preview contains the selected `CarryOpportunity`, including:

- investment and protocol identity;
- earn APY and matching borrow APY;
- base spread;
- TVL and available borrow liquidity;
- observation time and qualification reasons.

This is a preview of the selected route, not a prepared transaction. The UI may
also display other eligible opportunities returned for the same pair so the
user can change `investmentId`.

## Prepare and execution

Prepare resolves the input again and converts the raw token amount using the
known settlement-token decimals. It calls `CarryService::prepare_deposit` with
the wallet from `RecipeContext`, the validated collateral token, selected
investment ID, and resolved amount.

The prepared recipe step reuses `PreparedCarryDeposit`. Its transaction list
must be exactly one of:

```text
[APPROVE, DEPOSIT]
[DEPOSIT]
```

Any other ordering or transaction kind is rejected at the recipe boundary.
When approval is required, the frontend submits both calls through the existing
atomic wallet-call path. A wallet without atomic batch support is rejected
before sending either call; the flow is not downgraded to two independently
replayable transactions.

The existing activity journal, receipt reconciliation, 15-second polling, and
fresh-prepare manual retry flow apply unchanged. No prepared earn transaction
is persisted or replayed automatically.

After a confirmed receipt, the frontend marks the node complete with no outputs
and invalidates wallet balances and carry positions. The newly created position
then becomes available to the later Earn Position source flow.

## Frontend behavior

The palette enables `Earn deposit`. It can be added only after a compatible
wallet source or Borrow node.

The inspector shows:

- the bound stablecoin and exact amount from the incoming edge;
- collateral benchmark, locked when inherited from Borrow;
- reviewed eligible opportunities;
- earn APY, borrow APY, net spread, TVL, and liquidity;
- the expected approval/deposit batch before execution.

Opportunity loading, selection, preview, and preparation use generated OpenAPI
types and clients. No handwritten request or response interfaces are added.

If a saved investment becomes unavailable, the node stays editable and the UI
asks the user to select a currently eligible opportunity. It never silently
switches investments.

## Errors and security

- The connected wallet comes only from `RecipeContext`.
- Stablecoin, collateral, input shape, and investment identity are validated by
  the backend.
- Carry policy and live provider data are refreshed during prepare.
- Zero, negative, unresolved, or unrepresentable amounts are rejected.
- A provider-reported opportunity that does not match the input token is
  rejected.
- An unavailable or no-longer-reviewed investment cannot be prepared.
- Prepared calls are unsigned and short-lived; retries always prepare again.
- Only a confirmed successful receipt completes the node.

## Minimal tests

Backend tests cover:

- compilation of wallet and Borrow inputs and rejection of other graph shapes;
- Borrow collateral derivation and mismatch rejection;
- waiting, ready, and unavailable previews;
- policy and investment qualification being rechecked during prepare;
- exact `[APPROVE, DEPOSIT]` and `[DEPOSIT]` ordering;
- empty completed outputs for the terminal action.

Frontend tests cover serialization through generated types, locked Borrow
collateral, wallet collateral selection, opportunity selection, atomic-batch
gating, and account refresh after settlement. Presentational details do not get
duplicate tests.

## Acceptance criteria

- A wallet stablecoin can be deposited into a selected eligible opportunity.
- A confirmed Borrow output can be deposited using its exact settled amount.
- A dependent deposit cannot prepare before its Borrow output is confirmed.
- Unsupported tokens, mismatched collateral, and ineligible investments fail
  before the wallet opens.
- Approval and deposit are submitted atomically when both are required.
- Completion refreshes carry positions and exposes no fake token output.
