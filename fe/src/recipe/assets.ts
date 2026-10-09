import { useMemo } from "react";
import { erc20Abi } from "viem";
import { useAccount, useReadContracts } from "wagmi";
import type { RecipeCatalog } from "@/api/generated";
import type { AssetOption } from "@/recipe/editor/types";

export function useRecipeAssets(catalog?: RecipeCatalog): {
  assets: AssetOption[];
  refetchBalances(): Promise<unknown>;
} {
  const { address: wallet } = useAccount();
  const settlementContracts = useMemo(
    () =>
      (catalog?.settlementTokens ?? []).flatMap((address) => [
        {
          address,
          abi: erc20Abi,
          functionName: "symbol" as const,
        },
        {
          address,
          abi: erc20Abi,
          functionName: "decimals" as const,
        },
      ]),
    [catalog?.settlementTokens],
  );
  const settlementMetadata = useReadContracts({
    contracts: settlementContracts,
    query: { enabled: settlementContracts.length > 0, staleTime: Infinity },
  });

  const baseAssets = useMemo<AssetOption[]>(() => {
    if (!catalog) return [];
    const settlements = catalog.settlementTokens.map((address, index) => {
      const symbol = settlementMetadata.data?.[index * 2];
      const decimals = settlementMetadata.data?.[index * 2 + 1];
      const known = knownSettlement(address);
      return {
        address,
        symbol:
          symbol?.status === "success" && typeof symbol.result === "string"
            ? symbol.result
            : (known?.symbol ?? shortAddress(address)),
        name: known?.name ?? "Settlement token",
        logoUrl: known?.logoUrl,
        decimals:
          decimals?.status === "success" && typeof decimals.result === "number"
            ? decimals.result
            : 18,
        kind: "settlement" as const,
      };
    });
    const bstocks = catalog.bstocks.map((asset) => ({
      address: asset.contract,
      symbol: asset.tokenSymbol,
      name: asset.underlyingName,
      decimals: asset.tokenDecimals,
      kind: "bstock" as const,
      logoUrl: asset.tokenLogoUrl,
    }));
    return [...settlements, ...bstocks];
  }, [catalog, settlementMetadata.data]);

  const balanceContracts = useMemo(
    () =>
      wallet
        ? baseAssets.map((asset) => ({
            address: asset.address,
            abi: erc20Abi,
            functionName: "balanceOf" as const,
            args: [wallet] as const,
          }))
        : [],
    [baseAssets, wallet],
  );
  const balances = useReadContracts({
    contracts: balanceContracts,
    query: { enabled: balanceContracts.length > 0, refetchInterval: 15_000 },
  });

  const assets = useMemo(
    () =>
      baseAssets.map((asset, index) => {
        const balance = balances.data?.[index];
        return {
          ...asset,
          balanceRaw:
            balance?.status === "success" && typeof balance.result === "bigint"
              ? balance.result
              : undefined,
        };
      }),
    [balances.data, baseAssets],
  );
  return { assets, refetchBalances: balances.refetch };
}

function knownSettlement(address: string) {
  switch (address.toLowerCase()) {
    case "0x55d398326f99059ff775485246999027b3197955":
      return {
        symbol: "USDT",
        name: "Tether USD",
        logoUrl:
          "https://assets-cdn.trustwallet.com/blockchains/smartchain/assets/0x55d398326f99059fF775485246999027B3197955/logo.png",
      };
    case "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d":
      return {
        symbol: "USDC",
        name: "USD Coin",
        logoUrl:
          "https://raw.githubusercontent.com/trustwallet/assets/master/blockchains/binance/assets/USDC-CD2/logo.png",
      };
    default:
      return undefined;
  }
}

export function shortAddress(address: string): string {
  return `${address.slice(0, 6)}…${address.slice(-4)}`;
}
