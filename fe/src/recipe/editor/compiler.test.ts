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
const supplyId = "00000000-0000-0000-0000-000000000004";
const supplyEdgeId = "00000000-0000-0000-0000-000000000005";
const positionId = "00000000-0000-0000-0000-000000000006";
const withdrawId = "00000000-0000-0000-0000-000000000007";
const withdrawEdgeId = "00000000-0000-0000-0000-000000000008";
const borrowId = "00000000-0000-0000-0000-000000000009";
const borrowEdgeId = "00000000-0000-0000-0000-000000000010";
const venusPool = "0xfD36E2c2a6789Db23113685031d7F16329158384" as const;

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
  {
    id: supplyId,
    type: "supply",
    position: { x: 400, y: 0 },
    data: { kind: "SUPPLY", venue: "venus" },
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
      {
        id: supplyEdgeId,
        source: tradeId,
        sourceHandle: "buy",
        target: supplyId,
        targetHandle: "asset",
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
      { id: supplyId, kind: "SUPPLY", venue: "venus" },
    ],
    edges: [
      {
        id: edgeId,
        from: { nodeId: walletId, port: "asset" },
        to: { nodeId: tradeId, port: "sell" },
      },
      {
        id: supplyEdgeId,
        from: { nodeId: tradeId, port: "buy" },
        to: { nodeId: supplyId, port: "asset" },
      },
    ],
  });
});

it("serializes a supplied Venus position into a terminal withdrawal", () => {
  const recipe = buildRecipe(
    1,
    [
      {
        id: positionId,
        type: "suppliedPosition",
        position: { x: 0, y: 0 },
        data: {
          kind: "SUPPLIED_POSITION",
          venue: "venus",
          pool: venusPool,
          token: nvdab,
        },
      },
      {
        id: withdrawId,
        type: "withdraw",
        position: { x: 300, y: 0 },
        data: { kind: "WITHDRAW" },
      },
    ],
    [
      {
        id: withdrawEdgeId,
        source: positionId,
        sourceHandle: "position",
        target: withdrawId,
        targetHandle: "position",
      },
    ],
    assets,
  );

  expect(recipe.nodes).toEqual([
    {
      id: positionId,
      kind: "SUPPLIED_POSITION",
      venue: "venus",
      pool: venusPool,
      token: nvdab,
    },
    { id: withdrawId, kind: "WITHDRAW" },
  ]);
  expect(recipe.edges).toEqual([
    {
      id: withdrawEdgeId,
      from: { nodeId: positionId, port: "position" },
      to: { nodeId: withdrawId, port: "position" },
    },
  ]);
});

it("serializes an exact stablecoin borrow against a supplied position", () => {
  const recipe = buildRecipe(
    1,
    [
      {
        id: positionId,
        type: "suppliedPosition",
        position: { x: 0, y: 0 },
        data: {
          kind: "SUPPLIED_POSITION",
          venue: "venus",
          pool: venusPool,
          token: nvdab,
        },
      },
      {
        id: borrowId,
        type: "borrow",
        position: { x: 300, y: 0 },
        data: { kind: "BORROW", debtToken: usdt, amount: "1.25" },
      },
    ],
    [
      {
        id: borrowEdgeId,
        source: positionId,
        sourceHandle: "position",
        target: borrowId,
        targetHandle: "collateral",
      },
    ],
    assets,
  );

  expect(recipe.nodes[1]).toEqual({
    id: borrowId,
    kind: "BORROW",
    debtToken: usdt,
    rawAmount: "1250000000000000000",
  });
  expect(recipe.edges[0]).toEqual({
    id: borrowEdgeId,
    from: { nodeId: positionId, port: "position" },
    to: { nodeId: borrowId, port: "collateral" },
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
