import type {
  PreparedRecipeStep,
  RecipeCompletedNode,
  RecipeDefinition,
  RecipeNodeOutput,
} from "@/api/generated";
import type { Address, Hash } from "viem";

export type ExecutionPhase =
  "awaiting" | "submitting" | "pending" | "complete" | "failed" | "attention";

export type ExecutionReference =
  | {
      kind: "CALL_BATCH";
      id: string;
      atomicRequired: boolean;
      transactionHash?: Hash;
    }
  | {
      kind: "RFQ_ORDER";
      submissionId: string;
      orderId?: string;
      transactionHash?: Hash;
    }
  | { kind: "LEGACY_TRANSACTION"; transactionHash: Hash };

export interface ExecutionAttempt {
  id: string;
  nodeId: string;
  kind: PreparedRecipeStep["kind"];
  attempt: number;
  phase: ExecutionPhase;
  executionMode?: "SWAP" | "RFQ";
  steps: PreparedRecipeStep["kind"][];
  reference?: ExecutionReference;
  firstSubmittedAtMs?: number;
  approvalSkipped?: boolean;
  amountsUnavailable?: boolean;
  settledOutput?: RecipeNodeOutput;
  error?: string;
}

export interface ExecutionRun {
  version: 2;
  id: string;
  wallet: Address;
  chainId: number;
  recipe: RecipeDefinition;
  createdAtMs: number;
  updatedAtMs: number;
  attempts: ExecutionAttempt[];
}

export function createRun(
  recipe: RecipeDefinition,
  wallet: Address,
): ExecutionRun {
  const now = Date.now();
  return {
    version: 2,
    id: crypto.randomUUID(),
    wallet,
    chainId: recipe.chainId,
    recipe,
    createdAtMs: now,
    updatedAtMs: now,
    attempts: [],
  };
}

export function latestAttempts(
  attempts: ExecutionAttempt[],
): ExecutionAttempt[] {
  return [...new Map(attempts.map((item) => [item.nodeId, item])).values()];
}

export function nextAttemptNumber(
  attempts: ExecutionAttempt[],
  nodeId: string,
): number {
  return (
    Math.max(
      0,
      ...attempts
        .filter((item) => item.nodeId === nodeId)
        .map((item) => item.attempt),
    ) + 1
  );
}

export function isUnresolved(attempt: ExecutionAttempt): boolean {
  return ["awaiting", "submitting", "pending", "attention"].includes(
    attempt.phase,
  );
}

// A saved wallet prompt is not proof that the prompt is still open after reload.
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

export function completedRecipeNodes(
  attempts: ExecutionAttempt[],
): RecipeCompletedNode[] {
  return latestAttempts(attempts)
    .filter((item) => item.kind === "TRADE" && item.phase === "complete")
    .map((item) => ({
      nodeId: item.nodeId,
      outputs: item.settledOutput ? [item.settledOutput] : [],
    }));
}

export function patchAttempt(
  run: ExecutionRun,
  attempt: ExecutionAttempt,
  patch: Partial<ExecutionAttempt>,
): ExecutionRun {
  return {
    ...run,
    updatedAtMs: Date.now(),
    attempts: run.attempts.map((item) =>
      item.id === attempt.id ? { ...item, ...patch } : item,
    ),
  };
}

export function sameRecipe(
  left: RecipeDefinition,
  right: RecipeDefinition,
): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

export function attemptLabel(item: ExecutionAttempt): string {
  if (item.phase === "complete")
    return item.amountsUnavailable
      ? "Settled; amount details unavailable"
      : "Complete";
  if (item.phase === "pending") return "Pending settlement";
  if (item.phase === "attention") return "Needs attention — check status";
  if (item.phase === "awaiting") return "Confirm in wallet";
  if (item.phase === "submitting") return "Submitting order";
  return "Failed — refresh route";
}
