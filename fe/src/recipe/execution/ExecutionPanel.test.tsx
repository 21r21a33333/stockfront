import { render, screen } from "@testing-library/react";
import { ExecutionPanel } from "./ExecutionPanel";
import { recipe, wallet, hash, token } from "./fixtures.test-support";

const assets = [
  {
    address: token,
    symbol: "USDT",
    name: "Tether",
    decimals: 18,
    kind: "settlement" as const,
  },
  {
    address: wallet,
    symbol: "NVDAB",
    name: "Nvidia",
    decimals: 18,
    kind: "bstock" as const,
  },
];
const props = {
  recipe,
  assets,
  busy: false,
  paused: false,
  sameRecipe: true,
  canExecute: false,
  canRetrySubmission: false,
  onCheck() {},
  onRetry() {},
  onRefresh() {},
  onRestore() {},
  onExecute() {},
};

it("keeps unresolved recovery visible without a selected draft node or prepared calldata", () => {
  render(
    <ExecutionPanel
      {...props}
      run={{
        version: 1,
        id: "run",
        wallet,
        chainId: 56,
        recipe,
        createdAtMs: 1,
        updatedAtMs: 1,
        attempts: [
          {
            nodeId: "trade",
            kind: "TRADE",
            attempt: 1,
            phase: "awaiting",
            transactionHash: hash,
          },
        ],
      }}
      busy={false}
      paused={false}
      sameRecipe={false}
      canRetrySubmission={false}
      onCheck={() => {}}
      onRetry={() => {}}
      onRefresh={() => {}}
    />,
  );
  expect(screen.getByText(hash)).toBeInTheDocument();
  expect(screen.getByRole("status")).toHaveTextContent("Needs attention");
  expect(screen.getByRole("button", { name: "Check status" })).toBeEnabled();
  expect(
    screen.queryByRole("button", { name: "Refresh route" }),
  ).not.toBeInTheDocument();
  expect(
    screen.queryByRole("button", { name: "Execute recipe" }),
  ).not.toBeInTheDocument();
});

it("labels a ready RFQ correctly and has only one execute action", () => {
  render(
    <ExecutionPanel
      {...props}
      canExecute
      prepared={{
        steps: [
          {
            kind: "TRADE",
            nodeId: "trade",
            action: {
              executionMode: "RFQ",
              submissionId: "submission",
              routeQuoteId: "quote",
              venue: "inchFusion",
              typedData: "{}",
            },
          },
        ],
      }}
    />,
  );
  expect(screen.getByText("RFQ USDT → NVDAB")).toBeVisible();
  expect(
    screen.getAllByRole("button", { name: "Execute recipe" }),
  ).toHaveLength(1);
});

it("does not offer execution after settlement", () => {
  render(
    <ExecutionPanel
      {...props}
      run={{
        version: 1,
        id: "run",
        wallet,
        chainId: 56,
        recipe,
        createdAtMs: 1,
        updatedAtMs: 1,
        attempts: [
          {
            nodeId: "trade",
            kind: "TRADE",
            attempt: 1,
            phase: "complete",
            transactionHash: hash,
          },
        ],
      }}
    />,
  );
  expect(screen.getByText("Recipe complete")).toBeVisible();
  expect(
    screen.queryByRole("button", { name: "Execute recipe" }),
  ).not.toBeInTheDocument();
  expect(
    screen.getByRole("link", { name: "View transaction" }),
  ).toHaveAttribute("href", `https://bscscan.com/tx/${hash}`);
  expect(screen.getByText(hash)).not.toBeVisible();
});
