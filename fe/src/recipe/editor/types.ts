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

export type EarnDepositNodeData = {
  kind: "EARN_DEPOSIT";
  collateralToken?: Address;
  investmentId?: string;
};

export type SuppliedPositionNodeData = {
  kind: "SUPPLIED_POSITION";
  venue: BorrowingVenue;
  pool?: Address;
  token?: Address;
};

export type BorrowedPositionNodeData = {
  kind: "BORROWED_POSITION";
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

export type RepayNodeData = {
  kind: "REPAY";
};

export type RecipeNodeData =
  | WalletAssetNodeData
  | SuppliedPositionNodeData
  | BorrowedPositionNodeData
  | TradeNodeData
  | SupplyNodeData
  | EarnDepositNodeData
  | WithdrawNodeData
  | BorrowNodeData
  | RepayNodeData;
export interface RecipeFlowNode {
  id: string;
  type:
    | "walletAsset"
    | "suppliedPosition"
    | "borrowedPosition"
    | "trade"
    | "supply"
    | "earnDeposit"
    | "withdraw"
    | "borrow"
    | "repay";
  position: { x: number; y: number };
  data: RecipeNodeData;
}

export interface RecipeFlowEdge {
  id: string;
  source: string;
  sourceHandle?: string | null;
  target: string;
  targetHandle?: string | null;
  type?: string;
}

export interface DraftRecipe {
  version: 1;
  nodes: RecipeFlowNode[];
  edges: RecipeFlowEdge[];
  name?: string;
}

export function flowType(kind: RecipePrimitiveKind): RecipeFlowNode["type"] {
  switch (kind) {
    case "WALLET_ASSET":
      return "walletAsset";
    case "SUPPLIED_POSITION":
      return "suppliedPosition";
    case "BORROWED_POSITION":
      return "borrowedPosition";
    case "TRADE":
      return "trade";
    case "SUPPLY":
      return "supply";
    case "EARN_DEPOSIT":
      return "earnDeposit";
    case "WITHDRAW":
      return "withdraw";
    case "BORROW":
      return "borrow";
    case "REPAY":
      return "repay";
  }
}
