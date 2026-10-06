import type { BorrowingVenue, RecipeSupplyPolicy } from "@/api/generated";
import type { AssetOption } from "./types";

export function canSupplyAsset(
  asset: AssetOption | undefined,
  policy: RecipeSupplyPolicy,
  venue: BorrowingVenue,
): boolean {
  const input =
    asset?.kind === "bstock"
      ? "BSTOCKS"
      : asset?.kind === "settlement"
        ? "SETTLEMENT_TOKENS"
        : undefined;
  return policy.allowedInputs.some(
    (rule) => rule.venue === venue && rule.input === input,
  );
}
