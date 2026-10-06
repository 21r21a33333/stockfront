import type {
  BorrowingAccount,
  BorrowingAsset,
  BorrowingVenue,
  RecipeRepayPolicy,
} from "@/api/generated";
import type { Address } from "viem";
import type { AssetOption } from "./types";

export interface BorrowedPositionOption {
  venue: BorrowingVenue;
  pool: Address;
  asset: BorrowingAsset;
  catalogAsset: AssetOption;
}

export function borrowedPositionFor(
  positions: BorrowedPositionOption[],
  pool?: string,
  token?: string,
): BorrowedPositionOption | undefined {
  return positions.find(
    (position) =>
      position.pool.toLowerCase() === pool?.toLowerCase() &&
      position.asset.token.contract.toLowerCase() === token?.toLowerCase(),
  );
}

export function selectableBorrowedPositions(
  account: BorrowingAccount | undefined,
  policy: RecipeRepayPolicy,
  assets: AssetOption[],
): BorrowedPositionOption[] {
  if (!account) return [];
  const debtTokens = new Set(
    policy.debtTokens.map((token) => token.toLowerCase()),
  );
  return account.positions.flatMap((position) => {
    const allowed = policy.allowedInputs.some(
      (rule) =>
        rule.input === "BORROWED_POSITIONS" &&
        rule.venue === account.venue &&
        rule.pool.toLowerCase() === position.pool.toLowerCase(),
    );
    if (!allowed) return [];
    return position.borrowed.flatMap((asset) => {
      const catalogAsset = assets.find(
        (candidate) =>
          debtTokens.has(candidate.address.toLowerCase()) &&
          candidate.address.toLowerCase() ===
            asset.token.contract.toLowerCase(),
      );
      return catalogAsset
        ? [
            {
              venue: account.venue,
              pool: position.pool,
              asset,
              catalogAsset,
            },
          ]
        : [];
    });
  });
}

export function canRepayPosition(
  position: BorrowedPositionOption | undefined,
  policy: RecipeRepayPolicy,
): boolean {
  return Boolean(
    position &&
    policy.debtTokens.some(
      (token) =>
        token.toLowerCase() === position.asset.token.contract.toLowerCase(),
    ) &&
    policy.allowedInputs.some(
      (rule) =>
        rule.input === "BORROWED_POSITIONS" &&
        rule.venue === position.venue &&
        rule.pool.toLowerCase() === position.pool.toLowerCase(),
    ),
  );
}
