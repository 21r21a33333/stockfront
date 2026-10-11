import type { Address } from "viem";
import type { RecipeOptionSource, RecipeTradePolicy } from "@/api/generated";
import type { AssetOption } from "./types";

export function compatibleTradeAssets(
  sellToken: Address | undefined,
  assets: AssetOption[],
  policy: RecipeTradePolicy,
): AssetOption[] {
  const sellAsset = assets.find((asset) =>
    sameAddress(asset.address, sellToken),
  );
  if (!sellAsset) return [];

  const sellGroup = groupFor(sellAsset);
  const buyGroups = new Set(
    policy.allowedTransitions
      .filter((transition) => transition.sell === sellGroup)
      .map((transition) => transition.buy),
  );
  return assets.filter((asset) => buyGroups.has(groupFor(asset)));
}

function groupFor(asset: AssetOption): RecipeOptionSource {
  switch (asset.kind) {
    case "settlement":
      return "SETTLEMENT_TOKENS";
    case "basket":
      return "BASKETS";
    case "bstock":
      return "BSTOCKS";
  }
}

function sameAddress(left: Address, right: Address | undefined): boolean {
  return left.toLowerCase() === right?.toLowerCase();
}
