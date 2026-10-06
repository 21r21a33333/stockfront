import type {
  PreparedRecipe,
  RecipePreview,
  RecipeWithdrawalUnavailableReason,
} from "@/api/generated";
import { AssetIdentity } from "@/ui/assets/AssetIdentity";
import { shortAddress } from "../assets";
import type { AssetOption, RecipeFlowNode } from "../editor/types";
import { inputForWithdraw } from "../editor/flowInput";
import { useRecipeEditor } from "../editor/store";
import {
  suppliedPositionFor,
  type SuppliedPositionOption,
} from "../editor/withdrawalPolicy";
import { formatPositionAmount, formatUsd } from "../format";
import styles from "../RecipeBuilder.module.css";

export function WithdrawFields({
  node,
  assets,
  positions,
  preview,
  prepared,
  completed,
  preparing,
  onPrepare,
}: {
  node: RecipeFlowNode;
  assets: AssetOption[];
  positions: SuppliedPositionOption[];
  preview?: RecipePreview;
  prepared?: PreparedRecipe;
  completed: boolean;
  preparing: boolean;
  onPrepare(): void;
}) {
  const editor = useRecipeEditor();
  if (node.data.kind !== "WITHDRAW") return null;
  const input = inputForWithdraw(editor.nodes, editor.edges, node.id);
  const asset = assets.find(
    (candidate) =>
      candidate.address.toLowerCase() === input?.token?.toLowerCase(),
  );
  const position = suppliedPositionFor(positions, input?.pool, input?.token);
  const candidate = preview?.nodes.find((item) => item.nodeId === node.id);
  const previewNode = candidate?.kind === "WITHDRAW" ? candidate : undefined;
  const preparedWithdrawal = prepared?.steps.some(
    (step) => step.kind === "WITHDRAW" && step.nodeId === node.id,
  );

  return (
    <>
      <div className={styles.inspectorSectionTitle}>
        <strong>Withdrawal</strong>
        <span>FULL POSITION</span>
      </div>
      <p className={styles.portCaption}>
        Withdraw the complete supplied balance from its exact Venus pool.
      </p>
      <section className={styles.marketPreview}>
        <div className={styles.withdrawAsset}>
          <AssetIdentity asset={asset} />
          <span>
            <strong>
              {formatPositionAmount(position?.asset.amount)}{" "}
              {asset?.symbol ?? ""}
            </strong>
            <small>{formatUsd(position?.asset.valueUsd)}</small>
          </span>
        </div>
        <dl>
          <div>
            <dt>Amount</dt>
            <dd>All supplied {asset?.symbol ?? "asset"}</dd>
          </div>
          <div>
            <dt>Destination</dt>
            <dd>Connected wallet</dd>
          </div>
          <div>
            <dt>Market</dt>
            <dd>
              {input?.pool ? shortAddress(input.pool) : "Connect position"}
            </dd>
          </div>
        </dl>
        {previewNode?.unavailableReason ? (
          <p className={`${styles.runtimeNotice} ${styles.runtimeWarning}`}>
            {reasonLabel(previewNode.unavailableReason)}
          </p>
        ) : null}
        {previewNode?.state === "COMPLETE" ? (
          <p className={styles.runtimeNotice}>This action is complete.</p>
        ) : null}
        {!preparedWithdrawal && !completed ? (
          <button
            type="button"
            className={styles.prepareButton}
            onClick={onPrepare}
            disabled={previewNode?.state !== "READY" || preparing}
          >
            {preparing ? "Preparing…" : "Prepare withdrawal"}
          </button>
        ) : null}
      </section>
    </>
  );
}

function reasonLabel(reason: RecipeWithdrawalUnavailableReason): string {
  if (reason === "OUTSTANDING_DEBT")
    return "Repay the outstanding Venus debt before withdrawing this position.";
  if (reason === "UNSUPPORTED_POOL")
    return "This Venus pool is not supported by withdrawal recipes.";
  return "This supplied position is no longer available. Refresh the board.";
}
