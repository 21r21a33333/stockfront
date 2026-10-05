import type { RecipeFlowEdge, RecipeFlowNode } from "./types";

export function inputForTrade(
  nodes: RecipeFlowNode[],
  edges: RecipeFlowEdge[],
  tradeId: string,
) {
  const edge = edges.find(
    (item) => item.target === tradeId && item.targetHandle === "sell",
  );
  const source = nodes.find((node) => node.id === edge?.source);
  if (source?.data.kind === "WALLET_ASSET") {
    return {
      token: source.data.token,
      amount: source.data.amount,
      sourceName: "Wallet Asset",
    };
  }
  if (source?.data.kind === "TRADE") {
    return { token: source.data.buyToken, sourceName: "Trade" };
  }
  return undefined;
}
