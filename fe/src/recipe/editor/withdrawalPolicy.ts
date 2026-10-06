import type {
  BorrowingAccount,
  BorrowingAsset,
  BorrowingVenue,
  RecipeWithdrawalPolicy,
} from "@/api/generated";
import type { Address } from "viem";
import type { AssetOption } from "./types";

export interface SuppliedPositionOption {
  venue: BorrowingVenue;
  pool: Address;
  asset: BorrowingAsset;
  catalogAsset: AssetOption;
}

export function suppliedPositionFor(
  positions: SuppliedPositionOption[],
  pool?: string,
  token?: string,
): SuppliedPositionOption | undefined {
  return positions.find(
    (position) =>
      position.pool.toLowerCase() === pool?.toLowerCase() &&
      position.asset.token.contract.toLowerCase() === token?.toLowerCase(),
  );
}

export function selectableSuppliedPositions(
  account: BorrowingAccount | undefined,
  policy: RecipeWithdrawalPolicy,
  assets: AssetOption[],
): SuppliedPositionOption[] {
  if (!account) return [];
  return account.positions.flatMap((position) => {
    const allowed = policy.allowedInputs.some(
      (rule) =>
        rule.venue === account.venue &&
        rule.pool.toLowerCase() === position.pool.toLowerCase(),
    );
    if (!allowed) return [];
    return position.supplied.flatMap((asset) => {
      const catalogAsset = assets.find(
        (candidate) =>
          candidate.kind === "bstock" &&
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
