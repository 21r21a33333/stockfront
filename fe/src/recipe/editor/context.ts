import { createContext, useContext } from "react";
import type {
  PreparedRecipe,
  RecipeDiagnostic,
  RecipePreview,
  TradeQuote,
} from "@/api/generated";
import type { ExecutionAttempt } from "../execution/journal";
import type { AssetOption } from "./types";
import type { SuppliedPositionOption } from "./withdrawalPolicy";
import type { BorrowedPositionOption } from "./repaymentPolicy";

export interface BuilderContextValue {
  assets: AssetOption[];
  suppliedPositions: SuppliedPositionOption[];
  borrowedPositions: BorrowedPositionOption[];
  diagnostics: RecipeDiagnostic[];
  selectedRoutes: Record<string, TradeQuote>;
  preview?: RecipePreview;
  prepared?: PreparedRecipe;
  execution: ExecutionAttempt[];
  connectedOutputs: string[];
  supplyableOutputs: string[];
  borrowablePositions: string[];
  withdrawablePositions: string[];
  repayablePositions: string[];
  appendTrade(sourceId: string): void;
  appendSupply(sourceId: string): void;
  appendWithdraw(sourceId: string): void;
  appendBorrow(sourceId: string): void;
  appendRepay(sourceId: string): void;
  openActivity(): void;
}

export const BuilderContext = createContext<BuilderContextValue | null>(null);

export function useBuilderContext() {
  const value = useContext(BuilderContext);
  if (!value) throw new Error("Recipe nodes require BuilderContext");
  return value;
}
