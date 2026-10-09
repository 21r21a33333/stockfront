import type { RecipePrimitiveKind } from "@/api/generated";

export type BlockKind =
  | "wallet"
  | "supplied"
  | "debt"
  | "trade"
  | "supply"
  | "borrow"
  | "repay"
  | "withdraw"
  | "earn"
  | "liquidity";

export const blockIcons: Record<BlockKind, string> = {
  wallet:
    "M3 7h15a3 3 0 0 1 3 3v7a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3zM3 7l3-3h10l2 3",
  supplied: "M12 3l9 5-9 5-9-5 9-5zM3 13l9 5 9-5",
  debt: "M6 3h12v18l-3-2-3 2-3-2-3 2zM9 8h6M9 12h6",
  trade: "M4 8h14l-3-3M20 16H6l3 3",
  supply: "M12 3v11M8 10l4 4 4-4M4 15v4h16v-4",
  borrow:
    "M12 3.5a8.5 8.5 0 1 1 0 17a8.5 8.5 0 1 1 0-17M12 7.5v9M8.8 13.3 12 16.5l3.2-3.2",
  repay: "M4 12a8 8 0 1 0 2.6-5.9M4 4v4.5h4.5M9 12.5l2 2 4-4.5",
  withdraw: "M12 20V9M8 13l4-4 4 4M4 4h16",
  earn: "M4 17l5-5 4 4 7-7M14 9h6v6",
  liquidity: "M12 3s6 6.5 6 11a6 6 0 0 1-12 0c0-4.5 6-11 6-11z",
};

export const blockTints: Record<BlockKind, string> = {
  wallet: "#EEF0EB",
  supplied: "#FFFFFF",
  debt: "#FFFFFF",
  trade: "#D5F56B",
  supply: "#AFD6F7",
  borrow: "#F7CB98",
  repay: "#D6CBF7",
  withdraw: "#AEE8DF",
  earn: "#F4E79A",
  liquidity: "#E3E6DE",
};

export function blockKind(kind: RecipePrimitiveKind): BlockKind {
  switch (kind) {
    case "WALLET_ASSET":
      return "wallet";
    case "SUPPLIED_POSITION":
      return "supplied";
    case "BORROWED_POSITION":
      return "debt";
    case "TRADE":
      return "trade";
    case "SUPPLY":
      return "supply";
    case "BORROW":
      return "borrow";
    case "REPAY":
      return "repay";
    case "WITHDRAW":
      return "withdraw";
    default:
      return "liquidity";
  }
}
