import type { RecipeFlowEdge, RecipeFlowNode } from "./types";

export function inputForTrade(
  nodes: RecipeFlowNode[],
  edges: RecipeFlowEdge[],
  tradeId: string,
) {
  return inputForAction(nodes, edges, tradeId, "sell");
}

export function inputForSupply(
  nodes: RecipeFlowNode[],
  edges: RecipeFlowEdge[],
  supplyId: string,
) {
  return inputForAction(nodes, edges, supplyId, "asset");
}

function inputForAction(
  nodes: RecipeFlowNode[],
  edges: RecipeFlowEdge[],
  targetId: string,
  targetHandle: string,
) {
  const edge = edges.find(
    (item) => item.target === targetId && item.targetHandle === targetHandle,
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
