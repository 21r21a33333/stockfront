import { createContext, useContext } from "react";
import type {
  PreparedRecipe,
  RecipeDiagnostic,
  TradeQuote,
} from "@/api/generated";
import type { ExecutionProgress } from "@/wallet/execution";
import type { AssetOption } from "./types";

export interface BuilderContextValue {
  assets: AssetOption[];
  diagnostics: RecipeDiagnostic[];
  selectedRoutes: Record<string, TradeQuote>;
  prepared?: PreparedRecipe;
  execution: ExecutionProgress[];
  connectedOutputs: string[];
  appendTrade(sourceId: string): void;
}

export const BuilderContext = createContext<BuilderContextValue | null>(null);

export function useBuilderContext() {
  const value = useContext(BuilderContext);
  if (!value) throw new Error("Recipe nodes require BuilderContext");
  return value;
}
