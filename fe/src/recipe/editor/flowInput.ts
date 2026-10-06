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

export function inputForWithdraw(
  nodes: RecipeFlowNode[],
  edges: RecipeFlowEdge[],
  withdrawId: string,
) {
  const edge = edges.find(
    (item) => item.target === withdrawId && item.targetHandle === "position",
  );
  const source = nodes.find((node) => node.id === edge?.source);
  return source?.data.kind === "SUPPLIED_POSITION" ? source.data : undefined;
}

export function inputForBorrow(
  nodes: RecipeFlowNode[],
  edges: RecipeFlowEdge[],
  borrowId: string,
) {
  const edge = edges.find(
    (item) => item.target === borrowId && item.targetHandle === "collateral",
  );
  const source = nodes.find((node) => node.id === edge?.source);
  return source?.data.kind === "SUPPLIED_POSITION" ? source.data : undefined;
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
  if (source?.data.kind === "BORROW") {
    return {
      token: source.data.debtToken,
      amount: source.data.amount,
      sourceName: "Borrow",
    };
  }
  return undefined;
}
