import { ShieldCheck } from "lucide-react";
import type {
  PreparedRecipe,
  RecipeRepayPolicy,
  RecipePreview,
  RepaymentCapabilityUnavailableReason,
} from "@/api/generated";
import { AssetIdentity } from "@/ui/assets/AssetIdentity";
import { shortAddress } from "../assets";
import { inputForRepay } from "../editor/flowInput";
import {
  borrowedPositionFor,
  canRepayPosition,
  type BorrowedPositionOption,
} from "../editor/repaymentPolicy";
import { useRecipeEditor } from "../editor/store";
import type { AssetOption, RecipeFlowNode } from "../editor/types";
import { formatPositionAmount } from "../format";
import styles from "../RecipeBuilder.module.css";

export function RepayFields({
  node,
  assets,
  positions,
  repayPolicy,
  preview,
  prepared,
  completed,
  preparing,
  onPrepare,
}: {
  node: RecipeFlowNode;
  assets: AssetOption[];
  positions: BorrowedPositionOption[];
  repayPolicy: RecipeRepayPolicy;
  preview?: RecipePreview;
  prepared?: PreparedRecipe;
  completed: boolean;
  preparing: boolean;
  onPrepare(): void;
}) {
  const editor = useRecipeEditor();
  if (node.data.kind !== "REPAY") return null;
  const input = inputForRepay(editor.nodes, editor.edges, node.id);
  const position = borrowedPositionFor(positions, input?.pool, input?.token);
  const asset = assets.find(
    (candidate) =>
      candidate.address.toLowerCase() === input?.token?.toLowerCase(),
  );
  const candidate = preview?.nodes.find((item) => item.nodeId === node.id);
  const previewNode = candidate?.kind === "REPAY" ? candidate : undefined;
  const capability = previewNode?.capability;
  const preparedRepayment = prepared?.steps.some(
    (step) => step.kind === "REPAY" && step.nodeId === node.id,
  );
  const ready =
    previewNode?.state === "READY" &&
    capability?.unavailableReasons.length === 0 &&
    canRepayPosition(position, repayPolicy);

  return (
    <>
      <div className={styles.inspectorSectionTitle}>
        <strong>Full repayment</strong>
        <span>LIVE DEBT</span>
      </div>
      <p className={styles.portCaption}>
        Venus refreshes the debt, wallet balance, and allowance before preparing
        the wallet request.
      </p>
      <section className={styles.fieldGroup}>
        <div className={styles.portBinding}>
          <span>in</span>
          <code>
            Debt&lt;{asset?.symbol ?? "?"} · All&gt;
            {position ? " · from borrowed position" : " · connect debt"}
          </code>
        </div>
      </section>

      {capability ? (
        <section className={styles.marketPreview}>
          <div className={styles.borrowPreviewHeading}>
            <span>
              <ShieldCheck size={14} /> Venus repayment preview
            </span>
            <b>{ready ? "READY" : "CHECK"}</b>
          </div>
          <div className={styles.withdrawAsset}>
            <AssetIdentity asset={asset} />
            <span>
              <strong>
                {formatPositionAmount(capability.debtAmount)}{" "}
                {asset?.symbol ?? capability.debt.symbol}
              </strong>
              <small>Full live debt</small>
            </span>
          </div>
          <dl>
            <div>
              <dt>Wallet balance</dt>
              <dd>
                {formatPositionAmount(capability.walletBalance)}{" "}
                {asset?.symbol ?? capability.debt.symbol}
              </dd>
            </div>
            <div>
              <dt>Finite approval</dt>
              <dd>
                {formatPositionAmount(capability.approvalAmount)}{" "}
                {asset?.symbol ?? capability.debt.symbol}
              </dd>
            </div>
            <div>
              <dt>Approval</dt>
              <dd>
                {capability.approvalRequired
                  ? "Required · atomic batch"
                  : "Existing allowance"}
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
              {reasonLabel(reason)}
            </p>
          ))}
          {!preparedRepayment && !completed ? (
            <button
              type="button"
              className={styles.prepareButton}
              onClick={onPrepare}
              disabled={!ready || preparing}
            >
              {preparing ? "Preparing…" : "Prepare full repayment"}
            </button>
          ) : null}
        </section>
      ) : previewNode?.state === "COMPLETE" ? (
        <p className={styles.runtimeNotice}>This repayment is complete.</p>
      ) : (
        <p className={styles.runtimeNotice}>
          Preview the recipe to read the current Venus debt.
        </p>
      )}
    </>
  );
}

function reasonLabel(reason: RepaymentCapabilityUnavailableReason): string {
  switch (reason) {
    case "POSITION_NOT_FOUND":
      return "This Venus debt no longer exists. Refresh the board.";
    case "UNSUPPORTED_POOL":
      return "This Venus pool is not supported by repayment recipes.";
    case "UNSUPPORTED_DEBT_TOKEN":
      return "This debt token is not supported by repayment recipes.";
    case "NO_DEBT":
      return "There is no remaining debt to repay.";
    case "INSUFFICIENT_WALLET_BALANCE":
      return "The connected wallet needs enough stablecoin for the debt and the small interest buffer.";
  }
}
