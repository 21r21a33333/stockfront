import { fireEvent, render, screen } from "@testing-library/react";
import { ExecutionPanel } from "./ExecutionPanel";
import { hash, prepared, recipe, token, wallet } from "./fixtures.test-support";
import type { ExecutionAttempt, ExecutionRun } from "./model";

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
const attempt = (patch: Partial<ExecutionAttempt> = {}): ExecutionAttempt => ({
  id: "trade:1",
  nodeId: "trade",
  kind: "TRADE",
  attempt: 1,
  phase: "pending",
  executionMode: "SWAP",
  steps: ["APPROVAL", "TRADE"],
  ...patch,
});
const run = (item: ExecutionAttempt): ExecutionRun => ({
  version: 2,
  id: "run",
  wallet,
  chainId: 56,
  recipe,
  createdAtMs: 1,
  updatedAtMs: 1,
  attempts: [item],
});
const props = {
  recipe,
  assets,
  busy: false,
  paused: false,
  sameRecipe: true,
  canExecute: false,
  canRetrySubmission: false,
  onCheck: vi.fn(),
  onRetry: vi.fn(),
  onRetryFailed: vi.fn(),
  onRefresh: vi.fn(),
  onRestore: vi.fn(),
  onExecute: vi.fn(),
};

it("presents approval and swap as one atomic activity", () => {
  render(<ExecutionPanel {...props} prepared={prepared} canExecute />);

  expect(screen.getByText("Swap USDT → NVDAB")).toBeVisible();
  expect(screen.getByText("Approve USDT")).toBeVisible();
  expect(screen.getByText("Swap → NVDAB")).toBeVisible();
  expect(screen.getByText("Atomic")).toBeVisible();
  expect(
    screen.getAllByRole("button", { name: "Execute recipe" }),
  ).toHaveLength(1);
});

it("checks a referenced pending batch without exposing another submit action", () => {
  render(
    <ExecutionPanel
      {...props}
      run={run(
        attempt({
          reference: {
            kind: "CALL_BATCH",
            id: "batch",
            atomicRequired: true,
          },
        }),
      )}
    />,
  );

  fireEvent.click(screen.getByRole("button", { name: "Check status" }));
  expect(props.onCheck).toHaveBeenCalledOnce();
  expect(
    screen.queryByRole("button", { name: /execute recipe/i }),
  ).not.toBeInTheDocument();
});

it("shows settlement or a fresh-route retry, never a stale resubmit", () => {
  const { rerender } = render(
    <ExecutionPanel
      {...props}
      run={run(attempt({ phase: "failed", error: "Atomic batch reverted." }))}
    />,
  );
  fireEvent.click(
    screen.getByRole("button", { name: "Refresh route to retry" }),
  );
  expect(props.onRetryFailed).toHaveBeenCalledWith("trade");

  rerender(
    <ExecutionPanel
      {...props}
      run={run(
        attempt({
          phase: "complete",
          reference: {
            kind: "CALL_BATCH",
            id: "batch",
            atomicRequired: true,
            transactionHash: hash,
          },
        }),
      )}
    />,
  );
  expect(screen.getByText("Recipe complete")).toBeVisible();
  expect(
    screen.getByRole("link", { name: "View transaction" }),
  ).toHaveAttribute("href", `https://bscscan.com/tx/${hash}`);
});
