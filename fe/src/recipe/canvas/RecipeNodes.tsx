import { Handle, Position, type NodeProps } from "@xyflow/react";
import { ArrowRightLeft, Check, Plus, WalletCards } from "lucide-react";
import { useState } from "react";
import { formatUnits } from "viem";
import type { PreparedRecipeStep } from "@/api/generated";
import type { ExecutionProgress } from "@/wallet/execution";
import { diagnosticsForNode } from "../editor/compiler";
import { inputForTrade } from "../editor/flowInput";
import { useRecipeEditor } from "../editor/store";
import { useBuilderContext } from "../editor/context";
import type { AssetOption, RecipeFlowNode } from "../editor/types";
import styles from "../RecipeBuilder.module.css";

export function WalletAssetNode({
  id,
  data,
  selected,
}: NodeProps<RecipeFlowNode>) {
  const { assets, diagnostics } = useBuilderContext();
  if (data.kind !== "WALLET_ASSET") return null;
  const asset = assets.find(
    (item) => item.address.toLowerCase() === data.token?.toLowerCase(),
  );
  const problem = diagnosticsForNode(diagnostics, id)[0];
  return (
    <NodeShell
      title="Wallet Asset"
      tone="purple"
      icon={<WalletCards size={13} />}
      selected={selected}
      problem={problem?.message}
    >
      <p>
        {data.amount || "0"} {asset?.symbol ?? "asset"} from wallet
      </p>
      <div className={styles.nodeTags}>
        <span>{asset?.symbol ?? "Select asset"}</span>
        {asset?.balanceRaw !== undefined && asset ? (
          <span>bal {formatCompact(asset.balanceRaw, asset.decimals)}</span>
        ) : null}
      </div>
      <Handle
        id="asset"
        type="source"
        position={Position.Right}
        className={styles.handle}
      />
    </NodeShell>
  );
}

export function TradeNode({ id, data, selected }: NodeProps<RecipeFlowNode>) {
  const nodes = useRecipeEditor((state) => state.nodes);
  const edges = useRecipeEditor((state) => state.edges);
  const {
    assets,
    diagnostics,
    selectedRoutes,
    prepared,
    execution,
    connectedOutputs,
    appendTrade,
  } = useBuilderContext();
  const [showNextActions, setShowNextActions] = useState(false);
  if (data.kind !== "TRADE") return null;
  const asset = assets.find(
    (item) => item.address.toLowerCase() === data.buyToken?.toLowerCase(),
  );
  const quote = selectedRoutes[id];
  const input = inputForTrade(nodes, edges, id);
  const sellAsset = assets.find(
    (item) =>
      item.address.toLowerCase() ===
      (quote?.sell.token ?? input?.token)?.toLowerCase(),
  );
  const actions =
    prepared?.steps
      .map((step, index) => ({ step, index }))
      .filter(({ step }) => step.nodeId === id) ?? [];
  const problem = diagnosticsForNode(diagnostics, id)[0];
  return (
    <NodeShell
      title="Trade"
      tone="orange"
      icon={<ArrowRightLeft size={13} />}
      selected={selected}
      problem={problem?.message}
    >
      <p>
        Swap {sellAsset?.symbol ?? "incoming asset"} for{" "}
        {asset?.symbol ?? "an asset"}
      </p>
      <div className={styles.nodeTags}>
        <span>
          {quote && asset
            ? `≈ ${formatCompact(BigInt(quote.buy.amount), asset.decimals)} ${asset.symbol}`
            : `Buy ${asset?.symbol ?? "not selected"}`}
        </span>
        <span>{quote ? quote.venue : "Preview for route"}</span>
      </div>
      <Handle
        id="sell"
        type="target"
        position={Position.Left}
        className={styles.handle}
      />
      <Handle
        id="buy"
        type="source"
        position={Position.Right}
        className={styles.handle}
      />
      {!connectedOutputs.includes(id) ? (
        <div className={styles.nextActionAnchor}>
          <span className={styles.nextOutputLabel}>
            Asset&lt;{asset?.symbol ?? "?"}&gt;
          </span>
          <button
            type="button"
            className={`${styles.nextActionButton} nodrag nopan`}
            aria-label="Add next action"
            aria-expanded={showNextActions}
            onClick={(event) => {
              event.stopPropagation();
              setShowNextActions((open) => !open);
            }}
          >
            <Plus size={17} />
          </button>
          <span className={styles.nextActionHint}>Next action</span>
          {showNextActions ? (
            <div
              className={`${styles.nextActionMenu} nodrag nopan`}
              onClick={(event) => event.stopPropagation()}
              onKeyDown={(event) => {
                if (event.key === "Escape") setShowNextActions(false);
              }}
            >
              <strong>Add next action</strong>
              <button
                type="button"
                onClick={() => {
                  appendTrade(id);
                  setShowNextActions(false);
                }}
              >
                <ArrowRightLeft size={14} /> Trade
              </button>
              <span>
                Supply collateral <small>Soon</small>
              </span>
            </div>
          ) : null}
        </div>
      ) : null}
      {actions.length > 0 ? (
        <PreparedActionBranch
          actions={actions}
          assets={assets}
          buySymbol={asset?.symbol}
          execution={execution}
        />
      ) : null}
    </NodeShell>
  );
}

function PreparedActionBranch({
  actions,
  assets,
  buySymbol,
  execution,
}: {
  actions: { step: PreparedRecipeStep; index: number }[];
  assets: AssetOption[];
  buySymbol?: string;
  execution: ExecutionProgress[];
}) {
  return (
    <div className={styles.actionBranch} aria-label="Prepared actions">
      <span className={styles.branchLabel}>prepare</span>
      <div className={styles.branchSteps}>
        {actions.map(({ step, index }) => {
          const state = execution.find((item) => item.index === index)?.state;
          const approvedAsset =
            step.kind === "APPROVAL"
              ? assets.find(
                  (asset) =>
                    asset.address.toLowerCase() ===
                    step.action.to.toLowerCase(),
                )
              : undefined;
          const label =
            step.kind === "APPROVAL"
              ? `Approve ${approvedAsset?.symbol ?? "asset"}`
              : `${step.action.executionMode === "RFQ" ? "Submit RFQ" : "Swap"} → ${buySymbol ?? "asset"}`;
          return (
            <span className={styles.branchStep} key={`${step.kind}-${index}`}>
              <span
                className={`${styles.branchNumber} ${state === "complete" ? styles.branchDone : ""} ${state === "awaiting" || state === "pending" ? styles.branchActive : ""}`}
              >
                {state === "complete" ? <Check size={12} /> : index + 1}
              </span>
              {label}
            </span>
          );
        })}
      </div>
    </div>
  );
}

function NodeShell({
  title,
  tone,
  icon,
  selected,
  problem,
  children,
}: {
  title: string;
  tone: "purple" | "orange";
  icon: React.ReactNode;
  selected: boolean;
  problem?: string;
  children: React.ReactNode;
}) {
  return (
    <div
      className={`${styles.recipeNode} ${styles[tone]} ${selected ? styles.selectedNode : ""} ${problem ? styles.problemNode : ""}`}
    >
      <div className={styles.nodeHead}>
        <span>{icon}</span>
        <strong>{title}</strong>
        <b>•••</b>
      </div>
      <div className={styles.nodeBody}>{children}</div>
      {problem ? <div className={styles.nodeProblem}>! {problem}</div> : null}
    </div>
  );
}

function formatCompact(value: bigint, decimals: number): string {
  const formatted = Number(formatUnits(value, decimals));
  return Number.isFinite(formatted)
    ? new Intl.NumberFormat("en-US", { maximumFractionDigits: 6 }).format(
        formatted,
      )
    : "—";
}
