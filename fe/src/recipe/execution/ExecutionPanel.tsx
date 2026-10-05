import { Check, Copy, ExternalLink, AlertCircle, Clock3 } from "lucide-react";
import { useState } from "react";
import type { PreparedRecipe, RecipeDefinition } from "@/api/generated";
import type { AssetOption } from "../editor/types";
import { shortAddress } from "../assets";
import {
  attemptLabel,
  executionProgress,
  isRunComplete,
  isUnresolved,
  type ExecutionAttempt,
  type ExecutionRun,
} from "./journal";
import styles from "./ExecutionPanel.module.css";

export function ExecutionPanel({
  run,
  recipe,
  prepared,
  assets,
  busy,
  paused,
  sameRecipe,
  canExecute,
  canRetrySubmission,
  onExecute,
  onCheck,
  onRetry,
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
  canRetrySubmission: boolean;
  onExecute(): void;
  onCheck(): void;
  onRetry(): void;
  onRefresh(): void;
  onRestore(): void;
}) {
  const [copyMessage, setCopyMessage] = useState("");
  const attempts = executionProgress(run?.attempts ?? [], busy);
  const actions: ExecutionAttempt[] = prepared
    ? prepared.steps.map(
        (step) =>
          attempts.find(
            (item) => item.nodeId === step.nodeId && item.kind === step.kind,
          ) ?? {
            nodeId: step.nodeId,
            kind: step.kind,
            attempt: 0,
            phase: "ready",
            executionMode:
              step.kind === "TRADE" ? step.action.executionMode : undefined,
          },
      )
    : [...attempts];
  for (const attempt of attempts) {
    if (
      !actions.some(
        (item) => item.nodeId === attempt.nodeId && item.kind === attempt.kind,
      )
    )
      actions.push(attempt);
  }
  const pending = attempts.find(isUnresolved);
  const canCheck = Boolean(
    pending &&
    !pending.needsReconciliation &&
    (pending.transactionHash || pending.orderId),
  );
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
        <strong>
          {complete
            ? "Recipe settled"
            : attempts.some(
                  (item) =>
                    item.phase === "attention" || item.phase === "failed",
                )
              ? "Needs attention"
              : pending
                ? "Execution in progress"
                : "Review transactions"}
        </strong>
        <span>
          {actions.filter((item) => item.phase === "complete").length} /{" "}
          {actions.length}
        </span>
      </div>
      <p className={styles.caption}>
        {run
          ? `BNB Chain · ${shortAddress(run.wallet)}`
          : "Review each action before confirming in your wallet."}
      </p>
      {!sameRecipe && (
        <p className={styles.notice}>
          You are editing a different draft. This activity belongs to the saved
          execution.
        </p>
      )}
      {paused && pending && (
        <p className={styles.notice}>
          Status checking is paused. No action was resubmitted.
        </p>
      )}
      <ol className={styles.timeline}>
        {actions.map((item, index) => {
          const node = snapshot.nodes.find((node) => node.id === item.nodeId);
          const sourceId = snapshot.edges.find(
            (edge) => edge.to.nodeId === item.nodeId,
          )?.from.nodeId;
          const source = snapshot.nodes.find((node) => node.id === sourceId);
          const sellToken =
            source?.kind === "WALLET_ASSET"
              ? source.token
              : source?.kind === "TRADE"
                ? source.buyToken
                : undefined;
          const approval = prepared?.steps.find(
            (step) => step.nodeId === item.nodeId && step.kind === "APPROVAL",
          );
          const label =
            item.kind === "APPROVAL"
              ? `Approve ${symbol(approval?.kind === "APPROVAL" ? approval.action.to : sellToken)}`
              : `${item.executionMode === "RFQ" ? "RFQ" : "Swap"} ${symbol(sellToken)} → ${symbol(node?.kind === "TRADE" ? node.buyToken : undefined)}`;
          const hash = item.replacementHash ?? item.transactionHash;
          const attention =
            item.phase === "attention" || item.phase === "failed";
          return (
            <li className={styles.card} key={`${item.nodeId}:${item.kind}`}>
              <div className={styles.action}>
                <span
                  className={`${styles.marker} ${item.phase === "complete" ? styles.done : attention ? styles.warning : ""}`}
                >
                  {item.phase === "complete" ? (
                    <Check size={15} />
                  ) : attention ? (
                    <AlertCircle size={15} />
                  ) : isUnresolved(item) ? (
                    <Clock3 size={15} />
                  ) : (
                    index + 1
                  )}
                </span>
                <div>
                  <strong>{label}</strong>
                  <span role="status">{attemptLabel(item)}</span>
                </div>
              </div>
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
              {(hash || item.orderId || item.submissionId) && (
                <details className={styles.details}>
                  <summary>Details</summary>
                  {hash && <code>{hash}</code>}
                  {item.orderId && <code>Order: {item.orderId}</code>}
                  {item.submissionId && !item.orderId && (
                    <code>Submission: {item.submissionId}</code>
                  )}
                  {item.orderId && (
                    <button
                      type="button"
                      onClick={() => void copy(item.orderId!)}
                    >
                      Copy order ID
                    </button>
                  )}
                </details>
              )}
              {isUnresolved(item) && !hash && !item.orderId && !busy && (
                <p className={styles.notice}>
                  No settlement reference was recovered. Check wallet activity
                  before making another trade.
                </p>
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
        {canCheck && (
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
        ) : canExecute ? (
          <button
            type="button"
            className={styles.primary}
            onClick={onExecute}
            disabled={busy}
          >
            Execute recipe
          </button>
        ) : sameRecipe && run ? (
          <button type="button" onClick={onRefresh} disabled={busy}>
            Refresh route
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
