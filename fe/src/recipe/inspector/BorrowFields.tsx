import { ChevronDown, ShieldCheck } from "lucide-react";
import type {
  BorrowCapabilityUnavailableReason,
  PreparedRecipe,
  RecipeBorrowPolicy,
  RecipePreview,
} from "@/api/generated";
import { AssetSelect } from "@/ui/assets/AssetSelect";
import { shortAddress } from "../assets";
import { borrowDebtAssets, canBorrowAgainst } from "../editor/borrowPolicy";
import { inputForBorrow } from "../editor/flowInput";
import { useRecipeEditor } from "../editor/store";
import type { AssetOption, RecipeFlowNode } from "../editor/types";
import {
  suppliedPositionFor,
  type SuppliedPositionOption,
} from "../editor/withdrawalPolicy";
import styles from "../RecipeBuilder.module.css";

export function BorrowFields({
  node,
  assets,
  positions,
  borrowPolicy,
  preview,
  prepared,
  completed,
  preparing,
  onPrepare,
}: {
  node: RecipeFlowNode;
  assets: AssetOption[];
  positions: SuppliedPositionOption[];
  borrowPolicy: RecipeBorrowPolicy;
  preview?: RecipePreview;
  prepared?: PreparedRecipe;
  completed: boolean;
  preparing: boolean;
  onPrepare(): void;
}) {
  const editor = useRecipeEditor();
  if (node.data.kind !== "BORROW") return null;
  const data = node.data;
  const input = inputForBorrow(editor.nodes, editor.edges, node.id);
  const position = suppliedPositionFor(positions, input?.pool, input?.token);
  const debtAssets = borrowDebtAssets(assets, borrowPolicy);
  const debtAsset = assets.find(
    (asset) => asset.address.toLowerCase() === data.debtToken?.toLowerCase(),
  );
  const debtSupported = debtAssets.some(
    (asset) => asset.address.toLowerCase() === data.debtToken?.toLowerCase(),
  );
  const collateralSupported = canBorrowAgainst(position, borrowPolicy);
  const candidate = preview?.nodes.find((item) => item.nodeId === node.id);
  const previewNode = candidate?.kind === "BORROW" ? candidate : undefined;
  const capability = previewNode?.capability;
  const preparedStage = prepared?.steps.find(
    (step) =>
      step.nodeId === node.id &&
      (step.kind === "ENABLE_COLLATERAL" || step.kind === "BORROW"),
  )?.kind;
  const ready =
    previewNode?.state === "READY" &&
    capability?.unavailableReasons.length === 0 &&
    debtSupported &&
    collateralSupported;

  return (
    <>
      <div className={styles.inspectorSectionTitle}>
        <strong>Borrow</strong>
        <ChevronDown size={13} />
      </div>
      <p className={styles.portCaption}>
        Borrow against this position without moving the supplied asset.
      </p>
      <section className={styles.fieldGroup}>
        <div className={styles.portBinding}>
          <span>in</span>
          <code>
            Position&lt;{position?.catalogAsset.symbol ?? "?"}&gt;
            {position ? " · from supplied position" : " · connect collateral"}
          </code>
        </div>
        <div className={styles.portRow}>
          <span>Debt</span>
          <AssetSelect
            assets={debtAssets}
            value={data.debtToken}
            selectedAsset={debtAsset}
            invalid={Boolean(data.debtToken && !debtSupported)}
            onChange={(debtToken) => editor.updateNode(node.id, { debtToken })}
          />
        </div>
        <label htmlFor={`borrow-amount-${node.id}`}>Amount</label>
        <div className={styles.amountField}>
          <input
            id={`borrow-amount-${node.id}`}
            aria-label="Borrow amount"
            inputMode="decimal"
            value={data.amount}
            onChange={(event) =>
              editor.updateNode(node.id, { amount: event.target.value })
            }
          />
          <span>{debtAsset?.symbol ?? "TOKEN"}</span>
        </div>
        {capability ? (
          <button
            type="button"
            className={styles.safeMaxButton}
            onClick={() =>
              editor.updateNode(node.id, {
                amount: capability.conservativeMaxAmount,
              })
            }
          >
            Use conservative max · {capability.conservativeMaxAmount}{" "}
            {debtAsset?.symbol}
          </button>
        ) : null}
      </section>

      {capability ? (
        <section className={styles.marketPreview}>
          <div className={styles.borrowPreviewHeading}>
            <span>
              <ShieldCheck size={14} /> Venus risk preview
            </span>
            <b>{capability.stage === "BORROW" ? "READY" : "ACTIVATE"}</b>
          </div>
          <div className={styles.marketApy}>{capability.borrowApyPercent}%</div>
          <dl>
            <div>
              <dt>Borrow APY</dt>
              <dd>{capability.borrowApyPercent}%</dd>
            </div>
            <div>
              <dt>Health factor</dt>
              <dd>
                {healthFactor(capability.healthFactorBefore)} →{" "}
                {healthFactor(capability.healthFactorAfter)}
              </dd>
            </div>
            <div>
              <dt>Conservative max</dt>
              <dd>
                {capability.conservativeMaxAmount} {debtAsset?.symbol}
              </dd>
            </div>
            <div>
              <dt>Available liquidity</dt>
              <dd>
                {capability.availableBorrowLiquidity} {debtAsset?.symbol}
              </dd>
            </div>
            <div>
              <dt>Collateral</dt>
              <dd>
                {position?.catalogAsset.symbol ?? "—"} ·{" "}
                {capability.collateralEnabled ? "enabled" : "activation needed"}
              </dd>
            </div>
            <div>
              <dt>Market</dt>
              <dd>{shortAddress(capability.pool)}</dd>
            </div>
          </dl>
          {capability.unavailableReasons.map((reason) => (
            <p
              className={`${styles.runtimeNotice} ${styles.runtimeWarning}`}
              key={reason}
            >
              {reasonLabel(reason, borrowPolicy.minimumHealthFactor)}
            </p>
          ))}
          {!preparedStage && !completed ? (
            <button
              type="button"
              className={styles.prepareButton}
              onClick={onPrepare}
              disabled={!ready || preparing}
            >
              {preparing
                ? "Preparing…"
                : capability.stage === "ENABLE_COLLATERAL"
                  ? "Prepare collateral activation"
                  : `Prepare ${debtAsset?.symbol ?? "stablecoin"} borrow`}
            </button>
          ) : preparedStage === "ENABLE_COLLATERAL" ? (
            <p className={styles.runtimeNotice}>
              Confirm collateral activation first. The borrow is prepared from
              fresh account risk after it settles.
            </p>
          ) : null}
        </section>
      ) : previewNode?.state === "WAITING_FOR_UPSTREAM" ? (
        <p className={styles.runtimeNotice}>
          Waiting for the supplied position source.
        </p>
      ) : previewNode?.state === "COMPLETE" ? (
        <p className={styles.runtimeNotice}>This borrow is complete.</p>
      ) : null}
    </>
  );
}

function healthFactor(value?: string | null): string {
  return value ?? "No debt";
}

function reasonLabel(
  reason: BorrowCapabilityUnavailableReason,
  minimumHealthFactor: string,
): string {
  switch (reason) {
    case "POSITION_NOT_FOUND":
      return "This supplied position is no longer available. Refresh the board.";
    case "UNSUPPORTED_POOL":
      return "This Venus pool is not supported by borrow recipes.";
    case "MARKET_UNLISTED":
      return "One of these assets is not listed in this Venus market.";
    case "COLLATERAL_UNAVAILABLE":
      return "This supplied asset cannot currently be used as collateral.";
    case "BORROW_PAUSED":
      return "Venus has paused borrowing for this stablecoin.";
    case "BORROW_CAP_REACHED":
      return "The Venus borrow cap has been reached.";
    case "INSUFFICIENT_BORROW_LIQUIDITY":
      return "The market does not have enough stablecoin liquidity.";
    case "INVALID_ORACLE_PRICE":
      return "A required Venus oracle price is unavailable.";
    case "HEALTH_FACTOR_UNAVAILABLE":
      return "Venus could not calculate the account health factor.";
    case "HEALTH_FACTOR_BELOW_MINIMUM":
      return `Reduce the amount to keep health factor at or above ${minimumHealthFactor}.`;
  }
}
