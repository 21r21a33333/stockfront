import { act, renderHook } from "@testing-library/react";
import type { PreparedRecipe } from "@/api/generated";
import { usePreparedExecution } from "./execution";

const sendTransaction = vi.hoisted(() => vi.fn().mockResolvedValue("0x1234"));
const waitForTransactionReceipt = vi.hoisted(() =>
  vi.fn().mockResolvedValue({ status: "success" }),
);

vi.mock("wagmi", () => ({
  useAccount: vi.fn(),
  useConnect: vi.fn(),
  useDisconnect: vi.fn(),
  useSwitchChain: vi.fn(),
  useWalletClient: () => ({
    data: { account: { address: "0x1234" }, sendTransaction },
  }),
  usePublicClient: () => ({ waitForTransactionReceipt }),
}));
vi.mock("@/wallet/config", () => ({ stockfrontChain: { id: 56 } }));
vi.mock("@/api/recipes", () => ({ recipesApi: { submitOrder: vi.fn() } }));

const token = "0x55d398326f99059fF775485246999027B3197955" as const;
const prepared: PreparedRecipe = {
  steps: [
    {
      kind: "APPROVAL",
      nodeId: "trade",
      action: { to: token, spender: token, data: "0x" },
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
          gasLimit: "1",
          gasPrice: "1",
          minReceiveAmount: "1",
        },
      },
    },
  ],
};

beforeEach(() => {
  sendTransaction.mockClear();
  waitForTransactionReceipt.mockReset();
  waitForTransactionReceipt.mockResolvedValue({ status: "success" });
});

it("does not execute a completed recipe again and resets progress for a new preparation", async () => {
  const { result, rerender } = renderHook(
    ({ plan }) => usePreparedExecution(plan),
    { initialProps: { plan: prepared } },
  );

  await act(async () => {
    await result.current.execute();
  });
  expect(result.current.progress.map((step) => step.state)).toEqual([
    "complete",
    "complete",
  ]);
  expect(result.current.completed).toBe(true);

  await act(async () => {
    await result.current.execute();
  });
  expect(sendTransaction).toHaveBeenCalledTimes(2);

  rerender({ plan: structuredClone(prepared) });
  expect(result.current.progress).toEqual([]);
  expect(result.current.completed).toBe(false);
});

it("does not mark a reverted wallet transaction complete", async () => {
  waitForTransactionReceipt.mockResolvedValueOnce({ status: "reverted" });
  const { result } = renderHook(() => usePreparedExecution(prepared));

  let failure: unknown;
  await act(async () => {
    try {
      await result.current.execute();
    } catch (error) {
      failure = error;
    }
  });
  expect(failure).toEqual(
    new Error("The wallet transaction failed on BNB Chain."),
  );
  expect(result.current.completed).toBe(false);
  expect(result.current.progress[0]?.state).toBe("pending");
  expect(sendTransaction).toHaveBeenCalledTimes(1);
});
