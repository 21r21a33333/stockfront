import { createContext, useContext } from "react";
import type { Address } from "viem";
import type {
  BorrowingAccount,
  CarryPosition,
  PreparedRecipe,
  RecipeCatalog,
  RecipeContext,
  RecipeDefinition,
  RecipeDiagnostic,
  RecipePreview,
  TradeQuote,
} from "@/api/generated";
import type { BorrowedPositionOption } from "@/recipe/editor/repaymentPolicy";
import type { EditorDefaults } from "@/recipe/editor/store";
import type { AssetOption, RecipeFlowNode } from "@/recipe/editor/types";
import type { SuppliedPositionOption } from "@/recipe/editor/withdrawalPolicy";
import type { useRecipeExecution } from "@/recipe/execution/useRecipeExecution";

export interface FreshPreparation {
  prepared: PreparedRecipe;
  /** Routes chosen from the fresh quote, keyed by trade node. */
  routes: Record<string, TradeQuote>;
  /** Routes the user last reviewed, before this refresh. */
  reviewed: Record<string, TradeQuote>;
  preview: RecipePreview;
}

export interface RecipeSessionValue {
  catalog?: RecipeCatalog;
  catalogError?: string;
  retryCatalog(): void;
  assets: AssetOption[];
  refetchBalances(): Promise<unknown>;
  borrowing?: BorrowingAccount;
  refetchBorrowing(): Promise<unknown>;
  suppliedPositions: SuppliedPositionOption[];
  borrowedPositions: BorrowedPositionOption[];
  defaults: EditorDefaults;
  walletReady: boolean;
  wallet?: Address;
  recipe: RecipeDefinition;
  ordered: RecipeFlowNode[];
  diagnostics: RecipeDiagnostic[];
  valid: boolean;
  validating: boolean;
  preview?: RecipePreview;
  previewing: boolean;
  previewError?: string;
  refreshPreview(): void;
  selectedRoutes: Record<string, TradeQuote>;
  selectRoute(nodeId: string, route: TradeQuote): void;
  runtimeContext?: RecipeContext;
  prepareNext(): Promise<FreshPreparation>;
  execution: ReturnType<typeof useRecipeExecution>;
  loadDefinition(recipe: RecipeDefinition, name: string): void;
  startFromPosition(
    source: "SUPPLIED_POSITION" | "BORROWED_POSITION",
    position: { venue: "venus"; pool: Address; token: Address },
    action: "BORROW" | "WITHDRAW" | "REPAY",
  ): void;
  earnPositions: CarryPosition[];
  refetchEarnPositions(): void;
  startEarnWithdrawal(position: CarryPosition): void;
}

export const RecipeSessionContext = createContext<RecipeSessionValue | null>(
  null,
);

export function useRecipeSession(): RecipeSessionValue {
  const value = useContext(RecipeSessionContext);
  if (!value)
    throw new Error("useRecipeSession requires RecipeSessionProvider");
  return value;
}
