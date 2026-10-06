import { Handle, Position, type NodeProps } from "@xyflow/react";
import {
  ArrowRightLeft,
  Check,
  CircleDollarSign,
  Landmark,
  Plus,
  Undo2,
  WalletCards,
} from "lucide-react";
import { useState } from "react";
import { formatUnits } from "viem";
import type { PreparedRecipeStep } from "@/api/generated";
import { attemptLabel, type ExecutionAttempt } from "../execution/journal";
import { diagnosticsForNode } from "../editor/compiler";
import {
  inputForBorrow,
  inputForSupply,
  inputForTrade,
  inputForWithdraw,
} from "../editor/flowInput";
import { useRecipeEditor } from "../editor/store";
import { useBuilderContext } from "../editor/context";
import type { AssetOption, RecipeFlowNode } from "../editor/types";
import { suppliedPositionFor } from "../editor/withdrawalPolicy";
import { formatPositionAmount, formatUsd } from "../format";
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

export function SuppliedPositionNode({
  id,
  data,
  selected,
}: NodeProps<RecipeFlowNode>) {
  const {
    diagnostics,
    suppliedPositions,
    connectedOutputs,
    borrowablePositions,
    withdrawablePositions,
    appendWithdraw,
    appendBorrow,
  } = useBuilderContext();
  const [showNextActions, setShowNextActions] = useState(false);
  if (data.kind !== "SUPPLIED_POSITION") return null;
  const position = suppliedPositionFor(
    suppliedPositions,
    data.pool,
    data.token,
  );
  const problem = diagnosticsForNode(diagnostics, id)[0];
  return (
    <NodeShell
      title="Supplied Position"
      tone="blue"
      icon={<Landmark size={13} />}
      selected={selected}
      problem={problem?.message}
    >
      <p>All {position?.catalogAsset.symbol ?? "asset"} supplied to Venus</p>
      <div className={styles.nodeTags}>
        <span>
          {position
            ? `${formatPositionAmount(position.asset.amount)} ${position.catalogAsset.symbol}`
            : "Select position"}
        </span>
        <span>{position ? formatUsd(position.asset.valueUsd) : "Venus"}</span>
      </div>
      <Handle
        id="position"
        type="source"
        position={Position.Right}
        className={styles.handle}
      />
      {!connectedOutputs.includes(id) && position ? (
        <div className={styles.nextActionAnchor}>
          <span className={styles.nextOutputLabel}>
            All supplied {position.catalogAsset.symbol}
          </span>
          <button
            type="button"
            className={`${styles.nextActionButton} nodrag nopan`}
            aria-label="Add position action"
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
            >
              <strong>Use supplied position</strong>
              <button
                type="button"
                disabled={!borrowablePositions.includes(id)}
                onClick={() => {
                  appendBorrow(id);
                  setShowNextActions(false);
                }}
              >
                <CircleDollarSign size={14} /> Borrow stablecoin
              </button>
              <button
                type="button"
                disabled={!withdrawablePositions.includes(id)}
                onClick={() => {
                  appendWithdraw(id);
                  setShowNextActions(false);
                }}
              >
                <Undo2 size={14} /> Withdraw
              </button>
            </div>
          ) : null}
        </div>
      ) : null}
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
    supplyableOutputs,
    appendTrade,
    appendSupply,
    openActivity,
  } = useBuilderContext();
  const [showNextActions, setShowNextActions] = useState(false);
  if (data.kind !== "TRADE") return null;
  const asset = assets.find(
    (item) => item.address.toLowerCase() === data.buyToken?.toLowerCase(),
  );
  const quote = selectedRoutes[id];
  const tradeProgress = execution.find(
    (item) => item.nodeId === id && item.kind === "TRADE",
  );
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
            ? `${tradeProgress?.phase === "complete" ? "Quote " : ""}≈ ${formatCompact(BigInt(quote.buy.amount), asset.decimals)} ${asset.symbol}`
            : `Buy ${asset?.symbol ?? "not selected"}`}
        </span>
        <span>
          {tradeProgress
            ? attemptLabel(tradeProgress)
            : quote
              ? quote.venue
              : "Preview for route"}
        </span>
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
              <button
                type="button"
                disabled={!supplyableOutputs.includes(id)}
                title={
                  supplyableOutputs.includes(id)
                    ? "Supply this bStock to Venus"
                    : "Venus Supply accepts supported bStocks"
                }
                onClick={() => {
                  appendSupply(id);
                  setShowNextActions(false);
                }}
              >
                <Landmark size={14} /> Supply
                {!supplyableOutputs.includes(id) ? (
                  <small>bStocks only</small>
                ) : null}
              </button>
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
          onOpen={openActivity}
        />
      ) : null}
    </NodeShell>
  );
}

export function SupplyNode({ id, data, selected }: NodeProps<RecipeFlowNode>) {
  const nodes = useRecipeEditor((state) => state.nodes);
  const edges = useRecipeEditor((state) => state.edges);
  const { assets, diagnostics, preview, prepared, execution, openActivity } =
    useBuilderContext();
  if (data.kind !== "SUPPLY") return null;
  const input = inputForSupply(nodes, edges, id);
  const asset = assets.find(
    (item) => item.address.toLowerCase() === input?.token?.toLowerCase(),
  );
  const candidate = preview?.nodes.find((item) => item.nodeId === id);
  const previewNode = candidate?.kind === "SUPPLY" ? candidate : undefined;
  const progress = execution.find(
    (item) => item.nodeId === id && item.kind === "SUPPLY",
  );
  const actions =
    prepared?.steps
      .map((step, index) => ({ step, index }))
      .filter(({ step }) => step.nodeId === id) ?? [];
  const problem = diagnosticsForNode(diagnostics, id)[0];
  return (
    <NodeShell
      title="Supply"
      tone="green"
      icon={<Landmark size={13} />}
      selected={selected}
      problem={problem?.message}
    >
      <p>Supply {asset?.symbol ?? "incoming asset"} to Venus</p>
      <div className={styles.nodeTags}>
        <span>
          {previewNode?.market
            ? `${previewNode.market.supplyApyPercent}% base APY`
            : "Venus Core Pool"}
        </span>
        <span>
          {progress
            ? attemptLabel(progress)
            : previewNode?.state === "READY"
              ? "Market ready"
              : "Preview market"}
        </span>
      </div>
      <Handle
        id="asset"
        type="target"
        position={Position.Left}
        className={styles.handle}
      />
      {actions.length > 0 ? (
        <PreparedActionBranch
          actions={actions}
          assets={assets}
          inputSymbol={asset?.symbol}
          execution={execution}
          onOpen={openActivity}
        />
      ) : null}
    </NodeShell>
  );
}

export function WithdrawNode({
  id,
  data,
  selected,
}: NodeProps<RecipeFlowNode>) {
  const nodes = useRecipeEditor((state) => state.nodes);
  const edges = useRecipeEditor((state) => state.edges);
  const { assets, diagnostics, preview, prepared, execution, openActivity } =
    useBuilderContext();
  if (data.kind !== "WITHDRAW") return null;
  const input = inputForWithdraw(nodes, edges, id);
  const asset = assets.find(
    (candidate) =>
      candidate.address.toLowerCase() === input?.token?.toLowerCase(),
  );
  const candidate = preview?.nodes.find((item) => item.nodeId === id);
  const previewNode = candidate?.kind === "WITHDRAW" ? candidate : undefined;
  const progress = execution.find(
    (item) => item.nodeId === id && item.kind === "WITHDRAW",
  );
  const actions =
    prepared?.steps
      .map((step, index) => ({ step, index }))
      .filter(({ step }) => step.nodeId === id) ?? [];
  const problem = diagnosticsForNode(diagnostics, id)[0];
  return (
    <NodeShell
      title="Withdraw"
      tone="blue"
      icon={<Undo2 size={13} />}
      selected={selected}
      problem={problem?.message}
    >
      <p>Withdraw all {asset?.symbol ?? "supplied asset"} from Venus</p>
      <div className={styles.nodeTags}>
        <span>Full position</span>
        <span>
          {progress
            ? attemptLabel(progress)
            : previewNode?.state === "READY"
              ? "Ready"
              : "Preview withdrawal"}
        </span>
      </div>
      <Handle
        id="position"
        type="target"
        position={Position.Left}
        className={styles.handle}
      />
      {actions.length > 0 ? (
        <PreparedActionBranch
          actions={actions}
          assets={assets}
          inputSymbol={asset?.symbol}
          execution={execution}
          onOpen={openActivity}
        />
      ) : null}
    </NodeShell>
  );
}

export function BorrowNode({ id, data, selected }: NodeProps<RecipeFlowNode>) {
  const nodes = useRecipeEditor((state) => state.nodes);
  const edges = useRecipeEditor((state) => state.edges);
  const {
    assets,
    diagnostics,
    preview,
    prepared,
    execution,
    connectedOutputs,
    appendTrade,
    openActivity,
  } = useBuilderContext();
  if (data.kind !== "BORROW") return null;
  const input = inputForBorrow(nodes, edges, id);
  const collateral = assets.find(
    (asset) => asset.address.toLowerCase() === input?.token?.toLowerCase(),
  );
  const debt = assets.find(
    (asset) => asset.address.toLowerCase() === data.debtToken?.toLowerCase(),
  );
  const candidate = preview?.nodes.find((item) => item.nodeId === id);
  const previewNode = candidate?.kind === "BORROW" ? candidate : undefined;
  const progress =
    execution.find((item) => item.nodeId === id && item.kind === "BORROW") ??
    execution.find(
      (item) => item.nodeId === id && item.kind === "ENABLE_COLLATERAL",
    );
  const actions =
    prepared?.steps
      .map((step, index) => ({ step, index }))
      .filter(({ step }) => step.nodeId === id) ?? [];
  const problem = diagnosticsForNode(diagnostics, id)[0];
  return (
    <NodeShell
      title="Borrow"
      tone="orange"
      icon={<CircleDollarSign size={13} />}
      selected={selected}
      problem={problem?.message}
    >
      <p>
        Borrow {data.amount || "0"} {debt?.symbol ?? "stablecoin"} against{" "}
        {collateral?.symbol ?? "collateral"}
      </p>
      <div className={styles.nodeTags}>
        <span>
          {previewNode?.capability
            ? `HF ${previewNode.capability.healthFactorAfter ?? "—"}`
            : `Borrow ${debt?.symbol ?? "asset"}`}
        </span>
        <span>
          {progress
            ? attemptLabel(progress)
            : previewNode?.capability?.stage === "ENABLE_COLLATERAL"
              ? "Activate collateral"
              : previewNode?.state === "READY"
                ? "Market ready"
                : "Preview risk"}
        </span>
      </div>
      <Handle
        id="collateral"
        type="target"
        position={Position.Left}
        className={styles.handle}
      />
      <Handle
        id="borrowed"
        type="source"
        position={Position.Right}
        className={styles.handle}
      />
      {!connectedOutputs.includes(id) && data.debtToken ? (
        <div className={styles.nextActionAnchor}>
          <span className={styles.nextOutputLabel}>
            Asset&lt;{debt?.symbol ?? "?"}&gt;
          </span>
          <button
            type="button"
            className={`${styles.nextActionButton} nodrag nopan`}
            aria-label="Trade borrowed asset"
            onClick={(event) => {
              event.stopPropagation();
              appendTrade(id);
            }}
          >
            <Plus size={17} />
          </button>
          <span className={styles.nextActionHint}>Trade</span>
        </div>
      ) : null}
      {actions.length > 0 ? (
        <PreparedActionBranch
          actions={actions}
          assets={assets}
          inputSymbol={collateral?.symbol}
          buySymbol={debt?.symbol}
          execution={execution}
          onOpen={openActivity}
        />
      ) : null}
    </NodeShell>
  );
}

function PreparedActionBranch({
  actions,
  assets,
  buySymbol,
  inputSymbol,
  execution,
  onOpen,
}: {
  actions: { step: PreparedRecipeStep; index: number }[];
  assets: AssetOption[];
  buySymbol?: string;
  inputSymbol?: string;
  execution: ExecutionAttempt[];
  onOpen(): void;
}) {
  const approval = actions.find(({ step }) => step.kind === "APPROVAL")?.step;
  const trade = actions.find(({ step }) => step.kind === "TRADE")?.step;
  const supplyStep = actions.find(({ step }) => step.kind === "SUPPLY")?.step;
  const supply = supplyStep?.kind === "SUPPLY" ? supplyStep : undefined;
  const withdrawalStep = actions.find(
    ({ step }) => step.kind === "WITHDRAW",
  )?.step;
  const withdrawal =
    withdrawalStep?.kind === "WITHDRAW" ? withdrawalStep : undefined;
  const collateralStep = actions.find(
    ({ step }) => step.kind === "ENABLE_COLLATERAL",
  )?.step;
  const collateral =
    collateralStep?.kind === "ENABLE_COLLATERAL" ? collateralStep : undefined;
  const borrowStep = actions.find(({ step }) => step.kind === "BORROW")?.step;
  const borrow = borrowStep?.kind === "BORROW" ? borrowStep : undefined;
  const nodeId =
    trade?.nodeId ??
    supply?.nodeId ??
    withdrawal?.nodeId ??
    collateral?.nodeId ??
    borrow?.nodeId ??
    approval?.nodeId;
  const state = execution.find(
    (item) =>
      item.nodeId === nodeId &&
      (!borrow || item.kind === "BORROW") &&
      (!collateral || item.kind === "ENABLE_COLLATERAL"),
  )?.phase;
  const approvedAsset =
    approval?.kind === "APPROVAL"
      ? assets.find(
          (asset) =>
            asset.address.toLowerCase() === approval.action.to.toLowerCase(),
        )
      : undefined;
  const label = preparedActionLabel({
    approval,
    trade,
    supply,
    withdrawal,
    collateral,
    borrow,
    approvedSymbol: approvedAsset?.symbol,
    buySymbol,
    inputSymbol,
  });
  return (
    <div className={styles.actionBranch} aria-label="Prepared actions">
      <span className={styles.branchLabel}>prepare</span>
      <div className={styles.branchSteps}>
        <button
          type="button"
          className={`${styles.branchStep} nodrag nopan`}
          onClick={(event) => {
            event.stopPropagation();
            onOpen();
          }}
        >
          <span
            className={`${styles.branchNumber} ${state === "complete" ? styles.branchDone : ""} ${state === "awaiting" || state === "pending" ? styles.branchActive : ""}`}
          >
            {state === "complete" ? <Check size={12} /> : 1}
          </span>
          {label}
        </button>
      </div>
    </div>
  );
}

function preparedActionLabel({
  approval,
  trade,
  supply,
  withdrawal,
  collateral,
  borrow,
  approvedSymbol,
  buySymbol,
  inputSymbol,
}: {
  approval?: PreparedRecipeStep;
  trade?: PreparedRecipeStep;
  supply?: PreparedRecipeStep;
  withdrawal?: PreparedRecipeStep;
  collateral?: PreparedRecipeStep;
  borrow?: PreparedRecipeStep;
  approvedSymbol?: string;
  buySymbol?: string;
  inputSymbol?: string;
}): string {
  if (borrow?.kind === "BORROW") return `Borrow ${buySymbol ?? "stablecoin"}`;
  if (collateral?.kind === "ENABLE_COLLATERAL")
    return `Enable ${inputSymbol ?? "asset"} collateral`;
  if (withdrawal?.kind === "WITHDRAW")
    return `Withdraw all ${inputSymbol ?? "asset"}`;
  if (supply?.kind === "SUPPLY") {
    const needsApproval = supply.action.transactions.some(
      (transaction) => transaction.kind === "APPROVE",
    );
    return `${needsApproval ? `Approve ${inputSymbol ?? "asset"} + ` : ""}supply to Venus`;
  }
  if (trade?.kind !== "TRADE") return `Approve ${approvedSymbol ?? "asset"}`;
  if (approval?.kind === "APPROVAL")
    return `Approve ${approvedSymbol ?? "asset"} + swap → ${buySymbol ?? "asset"}`;
  return `${trade.action.executionMode === "RFQ" ? "Submit RFQ" : "Swap"} → ${buySymbol ?? "asset"}`;
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
  tone: "purple" | "orange" | "green" | "blue";
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
