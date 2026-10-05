import type { RecipeDiagnostic } from "@/api/generated";
import {
  buildRecipe,
  diagnosticsForEdge,
  diagnosticsForNode,
} from "./compiler";
import type { AssetOption, RecipeFlowNode } from "./types";

const usdt = "0x55d398326f99059fF775485246999027B3197955" as const;
const nvdab = "0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436" as const;
const walletId = "00000000-0000-0000-0000-000000000001";
const tradeId = "00000000-0000-0000-0000-000000000002";
const edgeId = "00000000-0000-0000-0000-000000000003";

const assets: AssetOption[] = [
  {
    address: usdt,
    symbol: "USDT",
    name: "Tether USD",
    decimals: 18,
    kind: "settlement",
  },
  {
    address: nvdab,
    symbol: "NVDAB",
    name: "NVIDIA",
    decimals: 18,
    kind: "bstock",
  },
];

const nodes: RecipeFlowNode[] = [
  {
    id: walletId,
    type: "walletAsset",
    position: { x: 0, y: 0 },
    data: { kind: "WALLET_ASSET", token: usdt, amount: "6.25" },
  },
  {
    id: tradeId,
    type: "trade",
    position: { x: 200, y: 0 },
    data: { kind: "TRADE", buyToken: nvdab },
  },
];

it("serializes the graph into the compiler contract with exact raw units", () => {
  const recipe = buildRecipe(
    1,
    nodes,
    [
      {
        id: edgeId,
        source: walletId,
        sourceHandle: "asset",
        target: tradeId,
        targetHandle: "sell",
      },
    ],
    assets,
  );

  expect(recipe).toEqual({
    schemaVersion: 1,
    chainId: 56,
    nodes: [
      {
        id: walletId,
        kind: "WALLET_ASSET",
        token: usdt,
        rawAmount: "6250000000000000000",
      },
      { id: tradeId, kind: "TRADE", buyToken: nvdab },
    ],
    edges: [
      {
        id: edgeId,
        from: { nodeId: walletId, port: "asset" },
        to: { nodeId: tradeId, port: "sell" },
      },
    ],
  });
});

it("sends malformed amounts to the compiler as zero for a typed diagnostic", () => {
  const invalid = structuredClone(nodes);
  const first = invalid[0];
  if (first?.data.kind === "WALLET_ASSET") first.data.amount = "not-a-number";

  const recipe = buildRecipe(1, invalid, [], assets);
  expect(recipe.nodes[0]).toMatchObject({ rawAmount: "0" });
});

it("does not reinterpret grouping separators as a different amount", () => {
  const invalid = structuredClone(nodes);
  const first = invalid[0];
  if (first?.data.kind === "WALLET_ASSET") first.data.amount = "1,25";

  const recipe = buildRecipe(1, invalid, [], assets);
  expect(recipe.nodes[0]).toMatchObject({ rawAmount: "0" });
});

it("targets node and edge diagnostics without interpreting backend messages", () => {
  const diagnostics: RecipeDiagnostic[] = [
    {
      code: "INVALID_AMOUNT",
      severity: "ERROR",
      location: { kind: "FIELD", nodeId: walletId, field: "rawAmount" },
      message: "Amount must be greater than zero.",
    },
    {
      code: "UNKNOWN_PORT",
      severity: "ERROR",
      location: { kind: "EDGE", edgeId },
      message: "The edge references an unknown port.",
    },
  ];

  expect(diagnosticsForNode(diagnostics, walletId)).toHaveLength(1);
  expect(diagnosticsForEdge(diagnostics, edgeId)).toHaveLength(1);
});
