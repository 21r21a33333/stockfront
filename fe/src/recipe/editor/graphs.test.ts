import type { RecipeDefinition } from "@/api/generated";
import type { AssetOption } from "./types";
import { chainOrder, graphFromDefinition, nextKinds } from "./graphs";

const usdt: AssetOption = {
  address: "0x55d398326f99059fF775485246999027B3197955",
  symbol: "USDT",
  name: "Tether USD",
  decimals: 18,
  kind: "settlement",
};
const nvda: AssetOption = {
  address: "0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436",
  symbol: "NVDAB",
  name: "NVIDIA",
  decimals: 18,
  kind: "bstock",
};

const composed: RecipeDefinition = {
  schemaVersion: 1,
  chainId: 56,
  nodes: [
    {
      id: "53545246-0000-4000-8000-000000000000",
      kind: "WALLET_ASSET",
      token: usdt.address,
      rawAmount: "6000000000000000000",
    },
    {
      id: "53545246-0000-4000-8000-000000000001",
      kind: "TRADE",
      buyToken: nvda.address,
    },
    {
      id: "53545246-0000-4000-8000-000000000002",
      kind: "SUPPLY",
      venue: "venus",
    },
  ],
  edges: [
    {
      id: "53545246-0000-4000-8000-000100000001",
      from: { nodeId: "53545246-0000-4000-8000-000000000000", port: "asset" },
      to: { nodeId: "53545246-0000-4000-8000-000000000001", port: "sell" },
    },
    {
      id: "53545246-0000-4000-8000-000100000002",
      from: { nodeId: "53545246-0000-4000-8000-000000000001", port: "buy" },
      to: { nodeId: "53545246-0000-4000-8000-000000000002", port: "asset" },
    },
  ],
};

describe("graphFromDefinition", () => {
  it("restores display amounts and keeps the composed ports so the recipe compiles unchanged", () => {
    const graph = graphFromDefinition(composed, [usdt, nvda]);

    expect(graph.nodes[0]?.data).toEqual({
      kind: "WALLET_ASSET",
      token: usdt.address,
      amount: "6",
    });
    expect(
      graph.edges.map((edge) => [edge.sourceHandle, edge.targetHandle]),
    ).toEqual([
      ["asset", "sell"],
      ["buy", "asset"],
    ]);
    expect(
      chainOrder(graph.nodes, graph.edges).map((node) => node.data.kind),
    ).toEqual(["WALLET_ASSET", "TRADE", "SUPPLY"]);
  });

  it("refuses a strategy whose token is missing from the catalog instead of guessing decimals", () => {
    expect(() => graphFromDefinition(composed, [nvda])).toThrow(
      /no longer supported/,
    );
  });
});

describe("chainOrder", () => {
  it("orders by edges rather than insertion so the canvas matches execution order", () => {
    const graph = graphFromDefinition(composed, [usdt, nvda]);
    const shuffled = [graph.nodes[2]!, graph.nodes[0]!, graph.nodes[1]!];

    expect(chainOrder(shuffled, graph.edges).map((node) => node.id)).toEqual(
      graph.nodes.map((node) => node.id),
    );
  });
});

describe("nextKinds", () => {
  it("only offers blocks the recipe compiler accepts after the chain end", () => {
    expect(nextKinds(undefined)).toEqual([
      "WALLET_ASSET",
      "SUPPLIED_POSITION",
      "BORROWED_POSITION",
    ]);
    expect(nextKinds("SUPPLY")).toEqual([]);
    expect(nextKinds("BORROWED_POSITION")).toEqual(["REPAY"]);
  });
});
