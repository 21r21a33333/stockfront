import type { BorrowingAccount, RecipeWithdrawalPolicy } from "@/api/generated";
import type { AssetOption } from "./types";
import { selectableSuppliedPositions } from "./withdrawalPolicy";

const token = "0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436" as const;
const allowedPool = "0xfD36E2c2a6789Db23113685031d7F16329158384" as const;
const otherPool = "0x0000000000000000000000000000000000000009" as const;
const asset: AssetOption = {
  address: token,
  symbol: "NVDAB",
  name: "NVIDIA",
  decimals: 18,
  kind: "bstock",
};
const policy: RecipeWithdrawalPolicy = {
  allowedInputs: [{ venue: "venus", pool: allowedPool }],
};

it("keeps only catalogued supplies in the configured Venus pool", () => {
  const account: BorrowingAccount = {
    venue: "venus",
    chainId: 56,
    totalSuppliedUsd: "12",
    totalBorrowedUsd: "0",
    observedAtMs: 1,
    positions: [
      {
        pool: allowedPool,
        borrowed: [],
        supplied: [
          {
            token: { contract: token, symbol: "NVDAB" },
            amount: "0.03",
            valueUsd: "12",
          },
          {
            token: {
              contract: "0x0000000000000000000000000000000000000007",
              symbol: "UNKNOWN",
            },
            amount: "1",
            valueUsd: "1",
          },
        ],
      },
      {
        pool: otherPool,
        borrowed: [],
        supplied: [
          {
            token: { contract: token, symbol: "NVDAB" },
            amount: "1",
            valueUsd: "400",
          },
        ],
      },
    ],
  };

  expect(selectableSuppliedPositions(account, policy, [asset])).toEqual([
    {
      venue: "venus",
      pool: allowedPool,
      asset: account.positions[0].supplied[0],
      catalogAsset: asset,
    },
  ]);
});
