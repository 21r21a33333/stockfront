import type { RecipeBorrowPolicy } from "@/api/generated";
import type { AssetOption } from "./types";
import type { SuppliedPositionOption } from "./withdrawalPolicy";

export function borrowDebtAssets(
  assets: AssetOption[],
  policy: RecipeBorrowPolicy,
): AssetOption[] {
  const supported = new Set(
    policy.debtTokens.map((token) => token.toLowerCase()),
  );
  return assets.filter((asset) => supported.has(asset.address.toLowerCase()));
}

export function canBorrowAgainst(
  position: SuppliedPositionOption | undefined,
  policy: RecipeBorrowPolicy,
): boolean {
  return Boolean(
    position &&
    policy.allowedInputs.some(
      (rule) =>
        rule.input === "SUPPLIED_POSITIONS" &&
        rule.venue === position.venue &&
        rule.pool.toLowerCase() === position.pool.toLowerCase(),
    ),
  );
}
