import { render, screen } from "@testing-library/react";
import type { PreparedRecipe, RecipePreview } from "@/api/generated";
import { RecipeInspector } from "./RecipeInspector";
import { useRecipeEditor } from "../editor/store";
import type { AssetOption, RecipeFlowNode } from "../editor/types";

const usdt = "0x55d398326f99059fF775485246999027B3197955" as const;
const bstock = "0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436" as const;
const node: RecipeFlowNode = {
  id: "trade-1",
  type: "trade",
  position: { x: 0, y: 0 },
  data: { kind: "TRADE", buyToken: bstock },
};
const assets: AssetOption[] = [
  {
    address: usdt,
    symbol: "USDT",
    name: "Tether USD",
    decimals: 18,
    kind: "settlement",
  },
  {
    address: bstock,
    symbol: "NVDAB",
    name: "NVIDIA",
    decimals: 18,
    kind: "bstock",
  },
];
const preview: RecipePreview = {
  nodes: [
    {
      kind: "TRADE",
      nodeId: node.id,
      state: "READY",
      routes: [
        {
          routeQuoteId: "quote-1",
          venue: "LiquidMesh",
          executionMode: "SWAP",
          sell: { token: usdt, amount: "6000000000000000000" },
          buy: { token: bstock, amount: "100000000000000000" },
          priceImpactPercent: "0.38",
        },
      ],
    },
  ],
};
const prepared: PreparedRecipe = {
  steps: [
    {
      kind: "TRADE",
      nodeId: node.id,
      action: {
        executionMode: "SWAP",
        transaction: {
          to: bstock,
          data: "0x",
          value: "0",
          gasLimit: "21000",
          gasPrice: "1000000000",
          minReceiveAmount: "90000000000000000",
        },
      },
    },
  ],
};

it("shows quote output first, then prepared minimum and actual network estimate", () => {
  const props = {
    node,
    assets,
    diagnostics: [],
    preview,
    selectedRouteId: "quote-1",
    completed: false,
    preparing: false,
    onRouteSelect: () => {},
    onPrepare: () => {},
  };
  const view = render(<RecipeInspector {...props} />);
  expect(screen.getByText("Expected received")).toBeInTheDocument();
  expect(screen.queryByText("Estimated network fee")).not.toBeInTheDocument();

  view.rerender(<RecipeInspector {...props} prepared={prepared} />);
  expect(screen.getByText("Minimum received")).toBeInTheDocument();
  expect(screen.getByText("0.09", { exact: false })).toBeInTheDocument();
  expect(screen.getByText("≈ 0.000021 BNB")).toBeInTheDocument();
});

it("shows the bound wallet asset and amount on a trade input", () => {
  useRecipeEditor.setState({
    nodes: [
      {
        id: "wallet-1",
        type: "walletAsset",
        position: { x: 0, y: 0 },
        data: { kind: "WALLET_ASSET", token: usdt, amount: "6" },
      },
      node,
    ],
    edges: [
      {
        id: "edge-1",
        type: "asset",
        source: "wallet-1",
        sourceHandle: "asset",
        target: node.id,
        targetHandle: "sell",
      },
    ],
  });
  render(
    <RecipeInspector
      node={node}
      assets={assets}
      diagnostics={[]}
      completed={false}
      preparing={false}
      onRouteSelect={() => {}}
      onPrepare={() => {}}
    />,
  );
  expect(
    screen.getByText("Asset<USDT> · from Wallet Asset"),
  ).toBeInTheDocument();
  expect(screen.getByText("6 USDT")).toBeInTheDocument();
});

it("keeps activity accessible without a selected node", () => {
  render(
    <RecipeInspector
      assets={assets}
      diagnostics={[]}
      completed={false}
      preparing={false}
      activity={<p>Recovered order</p>}
      view="activity"
      onRouteSelect={() => {}}
      onPrepare={() => {}}
    />,
  );
  expect(screen.getByText("Recovered order")).toBeVisible();
  expect(
    screen.queryByRole("button", { name: "Remove action" }),
  ).not.toBeInTheDocument();
});
