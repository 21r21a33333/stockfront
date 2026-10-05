import type { PreparedRecipe, RecipeDefinition } from "@/api/generated";

export const wallet = "0x1111111111111111111111111111111111111111" as const;
export const token = "0x2222222222222222222222222222222222222222" as const;
export const hash = `0x${"ab".repeat(32)}` as `0x${string}`;
export const recipe: RecipeDefinition = {
  schemaVersion: 1,
  chainId: 56,
  nodes: [
    { id: "source", kind: "WALLET_ASSET", token, rawAmount: "100" },
    { id: "trade", kind: "TRADE", buyToken: wallet },
  ],
  edges: [
    {
      id: "edge",
      from: { nodeId: "source", port: "asset" },
      to: { nodeId: "trade", port: "sell" },
    },
  ],
};
export const prepared: PreparedRecipe = {
  steps: [
    {
      kind: "APPROVAL",
      nodeId: "trade",
      action: { to: token, spender: wallet, data: "0x" },
    },
    {
      kind: "TRADE",
      nodeId: "trade",
      action: {
        executionMode: "SWAP",
        transaction: {
          to: token,
          data: "0x",
          value: "0",
          gasLimit: "100000",
          gasPrice: "1",
          minReceiveAmount: "1",
        },
      },
    },
  ],
};
