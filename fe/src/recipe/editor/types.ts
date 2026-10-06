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

export type SuppliedPositionNodeData = {
  kind: "SUPPLIED_POSITION";
  venue: BorrowingVenue;
  pool?: Address;
  token?: Address;
};

export type WithdrawNodeData = {
  kind: "WITHDRAW";
};

export type BorrowNodeData = {
  kind: "BORROW";
  debtToken?: Address;
  amount: string;
};

export type RecipeNodeData =
  | WalletAssetNodeData
  | SuppliedPositionNodeData
  | TradeNodeData
  | SupplyNodeData
  | WithdrawNodeData
  | BorrowNodeData;
export type RecipeFlowNode = Node<
  RecipeNodeData,
  | "walletAsset"
  | "suppliedPosition"
  | "trade"
  | "supply"
  | "withdraw"
  | "borrow"
>;
export type RecipeFlowEdge = Edge;

export interface DraftRecipe {
  version: 1;
  nodes: RecipeFlowNode[];
  edges: RecipeFlowEdge[];
}

export function flowType(kind: RecipePrimitiveKind): RecipeFlowNode["type"] {
  switch (kind) {
    case "WALLET_ASSET":
      return "walletAsset";
    case "SUPPLIED_POSITION":
      return "suppliedPosition";
    case "TRADE":
      return "trade";
    case "SUPPLY":
      return "supply";
    case "WITHDRAW":
      return "withdraw";
    case "BORROW":
      return "borrow";
  }
}
