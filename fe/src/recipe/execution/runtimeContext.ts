import type { RecipeContext } from "@/api/generated";
import type { Address } from "viem";
import { completedRecipeNodes, type ExecutionAttempt } from "./model";

export function recipeRuntimeContext(
  wallet: Address,
  attempts: ExecutionAttempt[],
): RecipeContext {
  return {
    wallet,
    completedNodes: completedRecipeNodes(attempts),
  };
}
