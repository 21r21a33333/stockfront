import { createContext, useContext } from "react";
import type {
  PreparedRecipe,
  RecipeDiagnostic,
  RecipePreview,
  TradeQuote,
} from "@/api/generated";
import type { ExecutionAttempt } from "../execution/journal";
import type { AssetOption } from "./types";

export interface BuilderContextValue {
  assets: AssetOption[];
  diagnostics: RecipeDiagnostic[];
  selectedRoutes: Record<string, TradeQuote>;
  preview?: RecipePreview;
  prepared?: PreparedRecipe;
  execution: ExecutionAttempt[];
  connectedOutputs: string[];
  supplyableOutputs: string[];
  appendTrade(sourceId: string): void;
  appendSupply(sourceId: string): void;
  openActivity(): void;
}

export const BuilderContext = createContext<BuilderContextValue | null>(null);

export function useBuilderContext() {
  const value = useContext(BuilderContext);
  if (!value) throw new Error("Recipe nodes require BuilderContext");
  return value;
}
