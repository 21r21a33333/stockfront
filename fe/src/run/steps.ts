import type { RecipeNode, TradeQuote } from "@/api/generated";
import type { ExecutionAttempt } from "@/recipe/execution/model";

export type StepState =
  | "done"
  | "next"
  | "ready"
  | "waiting"
  | "confirming"
  | "cancelled"
  | "unknown";

export interface RunStep {
  node: RecipeNode;
  state: StepState;
  /** Collateral activation finished but the borrow itself has not. */
  collateralEnabled: boolean;
  error?: string;
}

const actions = new Set([
  "TRADE",
  "SUPPLY",
  "EARN_DEPOSIT",
  "WITHDRAW",
  "BORROW",
  "REPAY",
]);

/**
 * One step per action block, in recipe order. A block is done only when its own
 * action completed; an approval or collateral activation alone never completes it.
 */
export function runSteps(
  nodes: RecipeNode[],
  attempts: ExecutionAttempt[],
  busy: boolean,
): RunStep[] {
  let currentFound = false;
  return nodes
    .filter((node) => actions.has(node.kind))
    .map((node) => {
      const own = attempts.filter((item) => item.nodeId === node.id);
      const latest = own.at(-1);
      const done = own.some(
        (item) => item.kind === node.kind && item.phase === "complete",
      );
      const collateralEnabled = own.some(
        (item) =>
          item.kind === "ENABLE_COLLATERAL" && item.phase === "complete",
      );
      if (done) return { node, state: "done", collateralEnabled };
      if (currentFound) return { node, state: "next", collateralEnabled };
      currentFound = true;
      const state: StepState =
        !latest || latest.phase === "complete"
          ? "ready"
          : latest.phase === "pending"
            ? "confirming"
            : latest.phase === "failed"
              ? "cancelled"
              : latest.phase === "attention" || !busy
                ? "unknown"
                : "waiting";
      return { node, state, collateralEnabled, error: latest?.error };
    });
}

/** A fresh quote that buys less than what the user reviewed must be confirmed again. */
export function priceMoved(
  reviewed: TradeQuote | undefined,
  fresh: TradeQuote | undefined,
  toleranceBps = 10n,
): { was: bigint; now: bigint } | undefined {
  if (!reviewed || !fresh) return undefined;
  const was = BigInt(reviewed.buy.amount);
  const now = BigInt(fresh.buy.amount);
  return now * 10_000n < was * (10_000n - toleranceBps)
    ? { was, now }
    : undefined;
}
