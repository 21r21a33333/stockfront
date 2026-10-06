import { fireEvent, render, screen, within } from "@testing-library/react";
import type {
  PreparedRecipe,
  RecipePreview,
  RecipeSupplyPolicy,
  RecipeTradePolicy,
} from "@/api/generated";
import { RecipeInspector } from "./RecipeInspector";
import { useRecipeEditor } from "../editor/store";
import type { AssetOption, RecipeFlowNode } from "../editor/types";

const usdt = "0x55d398326f99059fF775485246999027B3197955" as const;
const usdc = "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d" as const;
const bstock = "0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436" as const;
const tradePolicy: RecipeTradePolicy = {
  allowedTransitions: [
    { sell: "SETTLEMENT_TOKENS", buy: "BSTOCKS" },
    { sell: "BSTOCKS", buy: "SETTLEMENT_TOKENS" },
  ],
};
const supplyPolicy: RecipeSupplyPolicy = {
  allowedInputs: [{ input: "BSTOCKS", venue: "venus" }],
};
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
    tradePolicy,
    supplyPolicy,
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
      tradePolicy={tradePolicy}
      supplyPolicy={supplyPolicy}
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

it("shows a live Venus market and prepares a terminal supply", () => {
  const supply: RecipeFlowNode = {
    id: "supply-1",
    type: "supply",
    position: { x: 200, y: 0 },
    data: { kind: "SUPPLY", venue: "venus" },
  };
  useRecipeEditor.setState({
    nodes: [
      {
        id: "wallet-1",
        type: "walletAsset",
        position: { x: 0, y: 0 },
        data: { kind: "WALLET_ASSET", token: bstock, amount: "0.01" },
      },
      supply,
    ],
    edges: [
      {
        id: "edge-1",
        type: "asset",
        source: "wallet-1",
        sourceHandle: "asset",
        target: supply.id,
        targetHandle: "asset",
      },
    ],
  });
  const onPrepare = vi.fn();

  render(
    <RecipeInspector
      node={supply}
      assets={assets}
      tradePolicy={tradePolicy}
      supplyPolicy={supplyPolicy}
      diagnostics={[]}
      preview={{
        nodes: [
          {
            kind: "SUPPLY",
            nodeId: supply.id,
            state: "READY",
            market: {
              chainId: 56,
              venue: "venus",
              market: walletAddress(),
              token: { contract: bstock, symbol: "NVDAB" },
              tokenDecimals: 18,
              supplyApyPercent: "3.42",
              remainingSupplyCapacity: null,
              supplyAvailable: true,
              unavailableReasons: [],
              observedAtMs: 1,
            },
          },
        ],
      }}
      completed={false}
      preparing={false}
      onRouteSelect={() => {}}
      onPrepare={onPrepare}
    />,
  );

  expect(screen.getByText("Asset<NVDAB> · from Wallet Asset")).toBeVisible();
  expect(screen.getByText("3.42%")).toBeVisible();
  expect(screen.getByText("Unlimited")).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Prepare supply" }));
  expect(onPrepare).toHaveBeenCalledOnce();
});

it("keeps an unsupported saved choice editable and offers only compatible replacements", () => {
  const invalidTrade: RecipeFlowNode = {
    ...node,
    data: { kind: "TRADE", buyToken: usdt },
  };
  useRecipeEditor.setState({
    nodes: [
      {
        id: "wallet-1",
        type: "walletAsset",
        position: { x: 0, y: 0 },
        data: { kind: "WALLET_ASSET", token: usdc, amount: "6" },
      },
      invalidTrade,
    ],
    edges: [
      {
        id: "edge-1",
        type: "asset",
        source: "wallet-1",
        sourceHandle: "asset",
        target: invalidTrade.id,
        targetHandle: "sell",
      },
    ],
  });

  render(
    <RecipeInspector
      node={invalidTrade}
      assets={[
        ...assets,
        {
          address: usdc,
          symbol: "USDC",
          name: "USD Coin",
          decimals: 18,
          kind: "settlement",
        },
      ]}
      tradePolicy={tradePolicy}
      supplyPolicy={supplyPolicy}
      diagnostics={[]}
      completed={false}
      preparing={false}
      onRouteSelect={() => {}}
      onPrepare={() => {}}
    />,
  );

  const selector = screen.getByRole("button", { name: "Select asset" });
  expect(selector).toHaveAttribute("aria-invalid", "true");
  expect(
    screen.getByText(
      "USDT cannot be bought with USDC. Choose a supported asset.",
    ),
  ).toBeVisible();

  fireEvent.click(selector);
  const dialog = screen.getByRole("dialog", { name: "Select token" });
  expect(within(dialog).getByText("NVIDIA")).toBeVisible();
  expect(within(dialog).queryByText("Tether USD")).not.toBeInTheDocument();
  expect(within(dialog).queryByText("USD Coin")).not.toBeInTheDocument();

  fireEvent.click(within(dialog).getByText("NVIDIA").closest("button")!);
  const updated = useRecipeEditor
    .getState()
    .nodes.find((candidate) => candidate.id === invalidTrade.id);
  expect(updated?.data).toEqual({ kind: "TRADE", buyToken: bstock });
});

it("keeps activity accessible without a selected node", () => {
  render(
    <RecipeInspector
      assets={assets}
      tradePolicy={tradePolicy}
      supplyPolicy={supplyPolicy}
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

it.each([
  ["WAITING_FOR_UPSTREAM", "Waiting for the previous action to settle"],
  [
    "OUTPUT_UNAVAILABLE",
    "The previous action settled, but its exact output is unavailable",
  ],
] as const)("explains a %s dependent trade", (state, message) => {
  render(
    <RecipeInspector
      node={node}
      assets={assets}
      tradePolicy={tradePolicy}
      supplyPolicy={supplyPolicy}
      diagnostics={[]}
      preview={{
        nodes: [
          {
            kind: "TRADE",
            nodeId: node.id,
            state,
            waitingFor: ["trade-0"],
          },
        ],
      }}
      completed={false}
      preparing={false}
      onRouteSelect={() => {}}
      onPrepare={() => {}}
    />,
  );

  expect(screen.getByText(message)).toBeVisible();
  expect(
    screen.queryByRole("button", { name: "Prepare transactions" }),
  ).not.toBeInTheDocument();
});

function walletAddress() {
  return "0x1111111111111111111111111111111111111111" as const;
}
