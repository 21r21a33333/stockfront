import type { StrategyStep } from "@/api/generated";
import type { BlockKind } from "@/ui/blocks";

export interface PlanBlock {
  kind: BlockKind;
  name: string;
  detail: string;
}

/** Describes a composed intent in the same words the builder uses for blocks. */
export function planBlocks(
  steps: StrategyStep[],
  stablecoins: Set<string>,
): PlanBlock[] {
  return steps.map((step) => {
    switch (step.kind) {
      case "WALLET_ASSET":
        return {
          kind: "wallet",
          name: `Wallet ${step.symbol}`,
          detail: step.amount
            ? `${step.amount} ${step.symbol}`
            : "Amount needed",
        };
      case "SUPPLIED_POSITION":
        return {
          kind: "supplied",
          name: `Supplied ${step.symbol}`,
          detail: "On Venus",
        };
      case "BORROWED_POSITION":
        return {
          kind: "debt",
          name: `${step.symbol} debt`,
          detail: "On Venus",
        };
      case "TRADE":
        return {
          kind: "trade",
          name: stablecoins.has(step.buySymbol.toUpperCase())
            ? `Sell for ${step.buySymbol}`
            : `Buy ${step.buySymbol}`,
          detail: "Best live route",
        };
      case "SUPPLY":
        return {
          kind: "supply",
          name: "Supply to Venus",
          detail: "Becomes collateral",
        };
      case "BORROW":
        return {
          kind: "borrow",
          name: `Borrow ${step.symbol}`,
          detail: step.amount
            ? `${step.amount} ${step.symbol}`
            : "Amount needed",
        };
      case "REPAY":
        return {
          kind: "repay",
          name: "Repay",
          detail: "Full debt",
        };
      case "WITHDRAW":
        return { kind: "withdraw", name: "Withdraw", detail: "All supplied" };
    }
  });
}

export function planTitle(
  steps: StrategyStep[],
  stablecoins: Set<string>,
): string {
  const source = steps[0];
  const actions = steps.slice(1);
  const trade = actions.find((step) => step.kind === "TRADE");
  if (source?.kind === "BORROWED_POSITION")
    return `Repay ${source.symbol} debt`;
  if (source?.kind === "SUPPLIED_POSITION") {
    if (actions.some((step) => step.kind === "WITHDRAW"))
      return `Withdraw ${source.symbol}`;
    if (actions.some((step) => step.kind === "BORROW"))
      return `Borrow against ${source.symbol}`;
  }
  if (trade?.kind === "TRADE") {
    const selling = stablecoins.has(trade.buySymbol.toUpperCase());
    const verb = selling ? "Sell for" : "Buy";
    return actions.some((step) => step.kind === "SUPPLY")
      ? `${verb} & supply ${trade.buySymbol}`
      : `${verb} ${trade.buySymbol}`;
  }
  if (
    source?.kind === "WALLET_ASSET" &&
    actions.some((step) => step.kind === "SUPPLY")
  )
    return `Supply ${source.symbol}`;
  return "Your strategy";
}
