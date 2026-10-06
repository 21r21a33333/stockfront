import type { Edge, Node } from "@xyflow/react";
import type { Address } from "viem";
import type { BorrowingVenue, RecipePrimitiveKind } from "@/api/generated";

export interface AssetOption {
  address: Address;
  symbol: string;
  name: string;
  decimals: number;
  kind: "settlement" | "bstock";
  logoUrl?: string;
  balanceRaw?: bigint;
}

export type WalletAssetNodeData = {
  kind: "WALLET_ASSET";
  token?: Address;
  amount: string;
};

export type TradeNodeData = {
  kind: "TRADE";
  buyToken?: Address;
};

export type SupplyNodeData = {
  kind: "SUPPLY";
  venue: BorrowingVenue;
};

export type RecipeNodeData =
  WalletAssetNodeData | TradeNodeData | SupplyNodeData;
export type RecipeFlowNode = Node<
  RecipeNodeData,
  "walletAsset" | "trade" | "supply"
>;
export type RecipeFlowEdge = Edge;

export interface DraftRecipe {
  version: 1;
  nodes: RecipeFlowNode[];
  edges: RecipeFlowEdge[];
}

export function flowType(kind: RecipePrimitiveKind): RecipeFlowNode["type"] {
  if (kind === "WALLET_ASSET") return "walletAsset";
  return kind === "TRADE" ? "trade" : "supply";
}
