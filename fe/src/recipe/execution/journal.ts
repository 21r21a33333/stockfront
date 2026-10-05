import type { PreparedRecipeStep, RecipeDefinition } from "@/api/generated";
import type { Hash, Address } from "viem";

export type ExecutionPhase =
  | "ready"
  | "awaiting"
  | "submitting"
  | "pending"
  | "complete"
  | "failed"
  | "attention";
export interface ExecutionAttempt {
  nodeId: string;
  kind: PreparedRecipeStep["kind"];
  attempt: number;
  phase: ExecutionPhase;
  executionMode?: "SWAP" | "RFQ";
  transactionHash?: Hash;
  replacementHash?: Hash;
  orderId?: string;
  submissionId?: string;
  firstSubmittedAtMs?: number;
  receiptConfirmed?: boolean;
  needsReconciliation?: boolean;
  amountsUnavailable?: boolean;
  error?: string;
}
export interface ExecutionRun {
  version: 1;
  id: string;
  wallet: Address;
  chainId: number;
  recipe: RecipeDefinition;
  createdAtMs: number;
  updatedAtMs: number;
  attempts: ExecutionAttempt[];
}

export function runKey(wallet: string, chainId: number): string {
  return `stockfront:execution:${chainId}:${wallet.toLowerCase()}`;
}

export function latestAttempts(
  attempts: ExecutionAttempt[],
): ExecutionAttempt[] {
  return [
    ...new Map(
      attempts.map((item) => [`${item.nodeId}:${item.kind}`, item]),
    ).values(),
  ];
}

export function isUnresolved(attempt: ExecutionAttempt): boolean {
  return ["awaiting", "submitting", "pending", "attention"].includes(
    attempt.phase,
  );
}

// A saved prompt is not proof that a wallet prompt is still open after reload.
export function executionProgress(
  attempts: ExecutionAttempt[],
  busy: boolean,
): ExecutionAttempt[] {
  return latestAttempts(attempts).map((item) =>
    !busy && (item.phase === "awaiting" || item.phase === "submitting")
      ? { ...item, phase: "attention" }
      : item,
  );
}

export function isRunComplete(run: ExecutionRun): boolean {
  const trades = run.recipe.nodes.filter((node) => node.kind === "TRADE");
  const latest = latestAttempts(run.attempts);
  return (
    trades.length > 0 &&
    trades.every((node) =>
      latest.some(
        (item) =>
          item.nodeId === node.id &&
          item.kind === "TRADE" &&
          item.phase === "complete",
      ),
    )
  );
}

export function readRun(
  wallet: string,
  chainId: number,
): ExecutionRun | undefined {
  try {
    const raw = localStorage.getItem(runKey(wallet, chainId));
    if (raw === null) return undefined;
    const run = JSON.parse(raw) as ExecutionRun;
    const phases = [
      "ready",
      "awaiting",
      "submitting",
      "pending",
      "complete",
      "failed",
      "attention",
    ];
    if (
      !run ||
      run.version !== 1 ||
      typeof run.id !== "string" ||
      !run.id ||
      run.wallet?.toLowerCase() !== wallet.toLowerCase() ||
      run.chainId !== chainId ||
      !Number.isFinite(run.createdAtMs) ||
      !Number.isFinite(run.updatedAtMs) ||
      !run.recipe ||
      run.recipe.chainId !== chainId ||
      run.recipe.schemaVersion !== 1 ||
      !Array.isArray(run.recipe.nodes) ||
      !Array.isArray(run.recipe.edges) ||
      !run.recipe.nodes.every(
        (node) =>
          typeof node.id === "string" &&
          (node.kind === "TRADE"
            ? typeof node.buyToken === "string" &&
              /^0x[\da-f]{40}$/i.test(node.buyToken)
            : node.kind === "WALLET_ASSET" &&
              typeof node.token === "string" &&
              /^0x[\da-f]{40}$/i.test(node.token) &&
              typeof node.rawAmount === "string" &&
              /^\d+$/.test(node.rawAmount)),
      ) ||
      !run.recipe.edges.every(
        (edge) =>
          edge &&
          typeof edge.id === "string" &&
          typeof edge.from?.nodeId === "string" &&
          typeof edge.from?.port === "string" &&
          typeof edge.to?.nodeId === "string" &&
          typeof edge.to?.port === "string",
      ) ||
      !Array.isArray(run.attempts) ||
      !run.attempts.every(
        (item) =>
          item &&
          typeof item.nodeId === "string" &&
          run.recipe.nodes.some(
            (node) => node.id === item.nodeId && node.kind === "TRADE",
          ) &&
          ["APPROVAL", "TRADE"].includes(item.kind) &&
          phases.includes(item.phase) &&
          Number.isInteger(item.attempt) &&
          item.attempt > 0 &&
          [item.transactionHash, item.replacementHash].every(
            (hash) => hash === undefined || /^0x[\da-f]{64}$/i.test(hash),
          ) &&
          [item.orderId, item.submissionId, item.error].every(
            (value) => value === undefined || typeof value === "string",
          ) &&
          [
            item.receiptConfirmed,
            item.needsReconciliation,
            item.amountsUnavailable,
          ].every(
            (value) => value === undefined || typeof value === "boolean",
          ) &&
          (item.executionMode === undefined ||
            ["SWAP", "RFQ"].includes(item.executionMode)) &&
          (item.firstSubmittedAtMs === undefined ||
            Number.isFinite(item.firstSubmittedAtMs)),
      )
    ) {
      throw new Error("Invalid journal");
    }
    return run;
  } catch {
    throw new Error(
      "Recovery data could not be read. Do not retry a spend until the previous run is reconciled.",
    );
  }
}

export function writeRun(run: ExecutionRun): void {
  // Only public recovery evidence is durable. Wallet authorizations stay in memory.
  const { version, id, wallet, chainId, recipe, createdAtMs, updatedAtMs } =
    run;
  const attempts = run.attempts.map(
    ({
      nodeId,
      kind,
      attempt,
      phase,
      executionMode,
      transactionHash,
      replacementHash,
      orderId,
      submissionId,
      firstSubmittedAtMs,
      receiptConfirmed,
      needsReconciliation,
      amountsUnavailable,
      error,
    }) => ({
      nodeId,
      kind,
      attempt,
      phase,
      executionMode,
      transactionHash,
      replacementHash,
      orderId,
      submissionId,
      firstSubmittedAtMs,
      receiptConfirmed,
      needsReconciliation,
      amountsUnavailable,
      error,
    }),
  );
  localStorage.setItem(
    runKey(wallet, chainId),
    JSON.stringify({
      version,
      id,
      wallet,
      chainId,
      recipe,
      createdAtMs,
      updatedAtMs,
      attempts,
    }),
  );
}

export function attemptLabel(item: ExecutionAttempt): string {
  if (item.phase === "complete")
    return item.amountsUnavailable
      ? "Settled; amount details unavailable"
      : "Complete";
  if (item.phase === "pending")
    return item.receiptConfirmed
      ? "Confirmed; fetching settlement"
      : "Pending settlement";
  if (item.phase === "attention") return "Needs attention — check status";
  if (item.phase === "awaiting") return "Confirm in wallet";
  if (item.phase === "submitting") return "Submitting order";
  return item.phase === "failed" ? "Failed — refresh route" : "Ready";
}
