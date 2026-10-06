import { AlertCircle, Check, Clock3, Copy, ExternalLink } from "lucide-react";
import { useState } from "react";
import type {
  PreparedRecipe,
  PreparedRecipeStep,
  RecipeDefinition,
} from "@/api/generated";
import type { AssetOption } from "../editor/types";
import { shortAddress } from "../assets";
import {
  attemptLabel,
  executionProgress,
  isRunComplete,
  isUnresolved,
  latestAttempts,
  type ExecutionAttempt,
  type ExecutionRun,
} from "./model";
import styles from "./ExecutionPanel.module.css";

type ActivityItem = Omit<ExecutionAttempt, "phase"> & {
  phase: ExecutionAttempt["phase"] | "ready";
};

export function ExecutionPanel({
  run,
  recipe,
  prepared,
  assets,
  busy,
  paused,
  sameRecipe,
  canExecute,
  requiresFreshPreparation = false,
  canRetrySubmission,
  onExecute,
  onCheck,
  onRetry,
  onRetryFailed,
  onRefresh,
  onRestore,
}: {
  run?: ExecutionRun;
  recipe: RecipeDefinition;
  prepared?: PreparedRecipe;
  assets: AssetOption[];
  busy: boolean;
  paused: boolean;
  sameRecipe: boolean;
  canExecute: boolean;
  requiresFreshPreparation?: boolean;
  canRetrySubmission: boolean;
  onExecute(): void;
  onCheck(): void;
  onRetry(): void;
  onRetryFailed(nodeId: string): void;
  onRefresh(): void;
  onRestore(): void;
}) {
  const [copyMessage, setCopyMessage] = useState("");
  const attempts = executionProgress(run?.attempts ?? [], busy);
  const actions = mergePreparedActions(attempts, prepared);
  const pending = attempts.find(isUnresolved);
  const failed = attempts.find((item) => item.phase === "failed");
  const complete = Boolean(run && isRunComplete(run));
  const snapshot = run?.recipe ?? recipe;
  const symbol = (token?: string) =>
    assets.find((asset) => asset.address.toLowerCase() === token?.toLowerCase())
      ?.symbol ?? "asset";

  async function copy(value: string) {
    try {
      await navigator.clipboard.writeText(value);
      setCopyMessage("Reference copied");
    } catch {
      setCopyMessage("Copy unavailable. Select the reference in Details.");
    }
  }

  return (
    <section className={styles.panel} aria-label="Execution activity">
      <div className={styles.summary}>
        <strong>{summaryLabel(complete, attempts, pending)}</strong>
        <span>
          {completedActionCount(attempts, snapshot)}
          {" / "}
          {
            snapshot.nodes.filter(
              (node) =>
                node.kind === "TRADE" ||
                node.kind === "SUPPLY" ||
                node.kind === "WITHDRAW" ||
                node.kind === "BORROW",
            ).length
          }
        </span>
      </div>
      <p className={styles.caption}>
        {run
          ? `BNB Chain · ${shortAddress(run.wallet)}`
          : "Review the grouped action before confirming it in your wallet."}
      </p>
      {!sameRecipe && (
        <p className={styles.notice}>
          This activity belongs to the saved execution, not the edited draft.
        </p>
      )}
      {paused && pending && (
        <p className={styles.notice}>
          Automatic status checks paused after two minutes. No action was
          resubmitted.
        </p>
      )}
      <ol className={styles.timeline}>
        {actions.map((item, index) => {
          const node = snapshot.nodes.find((node) => node.id === item.nodeId);
          const sourceId = snapshot.edges.find(
            (edge) => edge.to.nodeId === item.nodeId,
          )?.from.nodeId;
          const source = snapshot.nodes.find((node) => node.id === sourceId);
          const sellToken = sourceToken(source);
          const buyToken =
            node?.kind === "TRADE"
              ? node.buyToken
              : node?.kind === "BORROW"
                ? node.debtToken
                : undefined;
          const label = actionLabel(item, sellToken, buyToken, symbol);
          const hash = transactionHash(item);
          const attention =
            item.phase === "attention" || item.phase === "failed";
          return (
            <li className={styles.card} key={item.id}>
              <div className={styles.action}>
                <span
                  className={`${styles.marker} ${item.phase === "complete" ? styles.done : attention ? styles.warning : ""}`}
                >
                  {item.phase === "complete" ? (
                    <Check size={15} />
                  ) : attention ? (
                    <AlertCircle size={15} />
                  ) : item.phase !== "ready" &&
                    isUnresolved(item as ExecutionAttempt) ? (
                    <Clock3 size={15} />
                  ) : (
                    index + 1
                  )}
                </span>
                <div>
                  <strong>{label}</strong>
                  <span role="status">
                    {item.phase === "ready"
                      ? "Ready"
                      : attemptLabel(item as ExecutionAttempt)}
                  </span>
                </div>
              </div>
              {item.steps.length > 1 && (
                <div className={styles.steps}>
                  <span>Approve {symbol(sellToken)}</span>
                  <span>Swap → {symbol(buyToken)}</span>
                  <b>Atomic</b>
                </div>
              )}
              {item.kind === "SUPPLY" && (
                <div className={styles.steps}>
                  {!item.approvalSkipped && (
                    <span>Approve {symbol(sellToken)}</span>
                  )}
                  <span>Supply to Venus</span>
                  {!item.approvalSkipped && <b>Atomic</b>}
                </div>
              )}
              {item.kind === "WITHDRAW" && (
                <div className={styles.steps}>
                  <span>Withdraw full position</span>
                  <span>Return to wallet</span>
                </div>
              )}
              {item.kind === "ENABLE_COLLATERAL" && (
                <div className={styles.steps}>
                  <span>Enable {symbol(sellToken)} as collateral</span>
                  <span>Borrow is prepared after settlement</span>
                </div>
              )}
              {item.kind === "BORROW" && (
                <div className={styles.steps}>
                  <span>Borrow {symbol(buyToken)} from Venus</span>
                  <span>Receive in connected wallet</span>
                </div>
              )}
              {item.error && <p className={styles.notice}>{item.error}</p>}
              {hash && (
                <div className={styles.reference}>
                  <a
                    href={`https://bscscan.com/tx/${hash}`}
                    target="_blank"
                    rel="noreferrer"
                  >
                    View transaction <ExternalLink size={12} />
                  </a>
                  <button
                    type="button"
                    aria-label={`Copy transaction for ${label}`}
                    onClick={() => void copy(hash)}
                  >
                    <Copy size={12} />
                  </button>
                </div>
              )}
              {item.reference && (
                <details className={styles.details}>
                  <summary>Details</summary>
                  <code>{referenceLabel(item.reference)}</code>
                </details>
              )}
              {item.phase === "attention" && !item.reference && (
                <p className={styles.notice}>
                  No settlement reference was returned. Review wallet activity
                  before starting another spend.
                </p>
              )}
              {item.phase === "failed" && sameRecipe && (
                <button
                  type="button"
                  className={styles.retryAction}
                  onClick={() => onRetryFailed(item.nodeId)}
                  disabled={busy}
                >
                  {item.kind === "WITHDRAW"
                    ? "Prepare withdrawal again"
                    : item.kind === "SUPPLY"
                      ? "Prepare supply again"
                      : item.kind === "ENABLE_COLLATERAL"
                        ? "Prepare collateral again"
                        : item.kind === "BORROW"
                          ? "Refresh risk and retry"
                          : "Refresh route to retry"}
                </button>
              )}
            </li>
          );
        })}
      </ol>
      <div className={styles.controls}>
        {!sameRecipe && (
          <button type="button" onClick={onRestore} disabled={busy}>
            Restore execution recipe
          </button>
        )}
        {pending?.reference && (
          <button type="button" onClick={onCheck} disabled={busy}>
            Check status
          </button>
        )}
        {canRetrySubmission && (
          <button type="button" onClick={onRetry} disabled={busy}>
            Retry same RFQ submission
          </button>
        )}
        {complete ? (
          <p className={styles.success}>
            <Check size={15} /> Recipe complete
          </p>
        ) : pending ? (
          <p className={styles.notice}>
            {busy
              ? "Follow the request in your wallet."
              : "No new transaction will be sent while this outcome is unresolved."}
          </p>
        ) : failed ? null : canExecute ? (
          <button
            type="button"
            className={styles.primary}
            onClick={onExecute}
            disabled={busy}
          >
            {requiresFreshPreparation ? "Prepare next stage" : "Execute recipe"}
          </button>
        ) : sameRecipe && run ? (
          <button type="button" onClick={onRefresh} disabled={busy}>
            Preview next step
          </button>
        ) : null}
      </div>
      {copyMessage && (
        <p className={styles.caption} role="status">
          {copyMessage}
        </p>
      )}
    </section>
  );
}

function mergePreparedActions(
  attempts: ExecutionAttempt[],
  prepared?: PreparedRecipe,
): ActivityItem[] {
  const actions: ActivityItem[] = [...attempts];
  if (!prepared) return actions;
  for (const nodeId of new Set(prepared.steps.map((step) => step.nodeId))) {
    const steps = prepared.steps.filter((step) => step.nodeId === nodeId);
    const supply = steps.find((step) => step.kind === "SUPPLY");
    const action = steps.find((step) => step.kind !== "APPROVAL");
    const kind = action?.kind ?? "APPROVAL";
    if (actions.some((item) => item.nodeId === nodeId && item.kind === kind))
      continue;
    actions.push({
      id: `prepared:${nodeId}`,
      nodeId,
      kind,
      attempt: 0,
      phase: "ready",
      executionMode: preparedExecutionMode(action),
      steps: steps.map((step) => step.kind),
      approvalSkipped:
        supply?.kind === "SUPPLY" &&
        !supply.action.transactions.some(
          (transaction) => transaction.kind === "APPROVE",
        ),
    });
  }
  return actions;
}

function summaryLabel(
  complete: boolean,
  attempts: ExecutionAttempt[],
  pending?: ExecutionAttempt,
): string {
  if (complete) return "Recipe settled";
  if (attempts.some((item) => ["attention", "failed"].includes(item.phase)))
    return "Needs attention";
  return pending ? "Execution in progress" : "Review transaction";
}

function transactionHash(item: ActivityItem): string | undefined {
  return item.reference && "transactionHash" in item.reference
    ? item.reference.transactionHash
    : undefined;
}

function referenceLabel(
  reference: NonNullable<ActivityItem["reference"]>,
): string {
  if (reference.kind === "CALL_BATCH") return `Wallet batch: ${reference.id}`;
  if (reference.kind === "RFQ_ORDER")
    return reference.orderId
      ? `Order: ${reference.orderId}`
      : `Submission: ${reference.submissionId}`;
  return `Legacy transaction: ${reference.transactionHash}`;
}

function sourceToken(
  node: RecipeDefinition["nodes"][number] | undefined,
): string | undefined {
  if (node?.kind === "WALLET_ASSET" || node?.kind === "SUPPLIED_POSITION")
    return node.token;
  return node?.kind === "TRADE" ? node.buyToken : undefined;
}

function actionLabel(
  item: ActivityItem,
  sellToken: string | undefined,
  buyToken: string | undefined,
  symbol: (token?: string) => string,
): string {
  switch (item.kind) {
    case "WITHDRAW":
      return `Withdraw all ${symbol(sellToken)} from Venus`;
    case "SUPPLY":
      return `Supply ${symbol(sellToken)} to Venus`;
    case "ENABLE_COLLATERAL":
      return `Enable ${symbol(sellToken)} as Venus collateral`;
    case "BORROW":
      return `Borrow ${symbol(buyToken)} from Venus`;
    case "APPROVAL":
      return `Approve ${symbol(sellToken)}`;
    case "TRADE":
      return `${item.executionMode === "RFQ" ? "RFQ" : "Swap"} ${symbol(sellToken)} → ${symbol(buyToken)}`;
  }
}

function preparedExecutionMode(
  step: PreparedRecipeStep | undefined,
): ExecutionAttempt["executionMode"] {
  if (step?.kind === "WITHDRAW") return "WITHDRAW";
  if (step?.kind === "SUPPLY") return "SUPPLY";
  if (step?.kind === "ENABLE_COLLATERAL" || step?.kind === "BORROW")
    return "BORROW";
  return step?.kind === "TRADE" ? step.action.executionMode : "SWAP";
}

function completedActionCount(
  attempts: ExecutionAttempt[],
  recipe: RecipeDefinition,
): number {
  return latestAttempts(attempts).filter(
    (attempt) =>
      attempt.phase === "complete" &&
      recipe.nodes.some(
        (node) => node.id === attempt.nodeId && node.kind === attempt.kind,
      ),
  ).length;
}
