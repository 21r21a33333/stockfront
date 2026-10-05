import { act, renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { PropsWithChildren } from "react";
import { recipe, prepared, wallet, hash } from "./fixtures.test-support";
import { readRun, writeRun } from "./journal";
import { useRecipeExecution } from "./useRecipeExecution";
import { useState } from "react";
import { flushSync } from "react-dom";
import { StockfrontApiError } from "@/api/client";

const mocks = vi.hoisted(() => ({
  send: vi.fn(),
  receipt: vi.fn(),
  getReceipt: vi.fn(),
  sign: vi.fn(),
  submit: vi.fn(),
  transaction: vi.fn(),
  order: vi.fn(),
  account: {
    address: "0x1111111111111111111111111111111111111111",
    chainId: 56,
  },
}));
vi.mock("wagmi", () => ({
  useAccount: () => mocks.account,
  useWalletClient: () => ({
    data: {
      account: { address: mocks.account.address },
      sendTransaction: mocks.send,
      signTypedData: mocks.sign,
    },
  }),
  usePublicClient: () => ({
    waitForTransactionReceipt: mocks.receipt,
    getTransactionReceipt: mocks.getReceipt,
  }),
}));
vi.mock("@/api/recipes", () => ({
  recipesApi: {
    submitOrder: mocks.submit,
    transactionStatus: mocks.transaction,
    orderStatus: mocks.order,
  },
}));
vi.mock("@/wallet/config", () => ({ stockfrontChain: { id: 56 } }));

function Wrapper({ children }: PropsWithChildren) {
  const [client] = useState(
    () =>
      new QueryClient({
        defaultOptions: { queries: { retry: false, gcTime: 0 } },
      }),
  );
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}
const wrapper = Wrapper;
afterEach(() => {
  vi.useRealTimers();
});
beforeEach(() => {
  Object.defineProperty(navigator, "onLine", {
    configurable: true,
    value: true,
  });
  localStorage.clear();
  vi.clearAllMocks();
  mocks.account = { address: wallet, chainId: 56 };
  mocks.send.mockResolvedValue(hash);
  mocks.receipt.mockReset();
  mocks.receipt.mockResolvedValue({ status: "success" });
  mocks.getReceipt.mockResolvedValue({ status: "success" });
  mocks.transaction.mockResolvedValue({ state: "pending" });
  mocks.order.mockResolvedValue({ state: "pendingVendor" });
  mocks.sign.mockResolvedValue("0xsigned");
  mocks.submit.mockResolvedValue({
    orderId: "order",
    state: "pendingVendor",
    createdAtMs: 1,
  });
  let locked = false;
  Object.defineProperty(navigator, "locks", {
    configurable: true,
    value: {
      request: vi.fn(async (_name, _options, action) => {
        if (locked) return action(null);
        locked = true;
        try {
          return await action({});
        } finally {
          locked = false;
        }
      }),
    },
  });
});

it("invalidates local prepared calldata when another tab changes the execution journal", async () => {
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared }),
    { wrapper },
  );
  expect(result.current.canExecute).toBe(true);
  act(() =>
    window.dispatchEvent(
      new StorageEvent("storage", { key: `stockfront:execution:56:${wallet}` }),
    ),
  );
  await act(() => result.current.execute());
  expect(mocks.send).not.toHaveBeenCalled();
});

it("keeps unknown identifier-less recovery labelled as attention, not a live wallet prompt", async () => {
  writeRun({
    version: 1,
    id: "existing",
    wallet,
    chainId: 56,
    recipe,
    createdAtMs: 1,
    updatedAtMs: 1,
    attempts: [
      { nodeId: "trade", kind: "TRADE", attempt: 1, phase: "awaiting" },
    ],
  });
  const { result } = renderHook(() => useRecipeExecution({ recipe }), {
    wrapper,
  });
  expect(result.current.progress[0]?.phase).toBe("attention");
});

it.each(["failed", "expired", "cancelled"])(
  "marks a terminal RFQ %s as failed without submitting again",
  async (state) => {
    writeRun({
      version: 1,
      id: "existing",
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
          phase: "pending",
          executionMode: "RFQ",
          orderId: "order",
        },
      ],
    });
    mocks.order.mockResolvedValue({ state });
    const { result } = renderHook(() => useRecipeExecution({ recipe }), {
      wrapper,
    });
    await waitFor(() =>
      expect(result.current.progress[0]?.phase).toBe("failed"),
    );
    expect(result.current.completed).toBe(false);
    expect(mocks.submit).not.toHaveBeenCalled();
  },
);

it("keeps an unknown transaction unresolved and refuses resend", async () => {
  mocks.transaction.mockResolvedValue({ state: "unknown" });
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared }),
    { wrapper },
  );
  await act(() => result.current.execute());
  await act(() => result.current.checkStatus());
  await act(() => result.current.execute());
  expect(result.current.blocked).toBe(true);
  expect(mocks.send).toHaveBeenCalledTimes(2);
});

it("does not start without browser locking", async () => {
  Object.defineProperty(navigator, "locks", {
    configurable: true,
    value: undefined,
  });
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared }),
    { wrapper },
  );
  expect(result.current.canExecute).toBe(false);
  await act(() => result.current.execute());
  expect(mocks.send).not.toHaveBeenCalled();
  expect(result.current.error).toMatch(/browser/i);
});

it("records a rejected wallet prompt as failed and requires fresh preparation", async () => {
  mocks.send.mockRejectedValueOnce({ code: 4001 });
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared }),
    { wrapper },
  );
  await act(() => result.current.execute());
  expect(result.current.progress[0]?.phase).toBe("failed");
  expect(result.current.progress[0]?.error).toMatch(/rejected/i);
  expect(result.current.blocked).toBe(false);
  expect(result.current.canExecute).toBe(false);
  await act(() => result.current.execute());
  expect(mocks.send).toHaveBeenCalledTimes(1);
});

it("a second executor cannot send while another tab holds the wallet lock", async () => {
  Object.defineProperty(navigator, "locks", {
    configurable: true,
    value: {
      request: async (
        _name: string,
        _options: object,
        action: (lock: null) => unknown,
      ) => action(null),
    },
  });
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared }),
    { wrapper },
  );
  await act(() => result.current.execute());
  expect(mocks.send).not.toHaveBeenCalled();
  expect(result.current.error).toMatch(/another tab/);
});

it("retains the broadcast hash in memory if its durable write fails", async () => {
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared }),
    { wrapper },
  );
  const spy = vi.spyOn(Storage.prototype, "setItem");
  mocks.send.mockImplementationOnce(async () => {
    spy.mockImplementation(() => {
      throw new Error("quota");
    });
    return hash;
  });
  await act(() => result.current.execute());
  expect(result.current.progress[0]?.transactionHash).toBe(hash);
  expect(mocks.send).toHaveBeenCalledTimes(1);
  expect(result.current.error).toMatch(/Copy/);
  spy.mockRestore();
});

it("requires fresh preparation after a reverted trade and rechecks its new approval", async () => {
  mocks.receipt
    .mockResolvedValueOnce({ status: "success" })
    .mockResolvedValueOnce({ status: "reverted" });
  const { result, rerender } = renderHook(
    ({ plan }) => useRecipeExecution({ recipe, prepared: plan }),
    { wrapper, initialProps: { plan: prepared } },
  );
  await act(() => result.current.execute());
  expect(result.current.progress.at(-1)?.phase).toBe("failed");
  await act(() => result.current.execute());
  expect(mocks.send).toHaveBeenCalledTimes(2);
  rerender({ plan: structuredClone(prepared) });
  await act(() => result.current.execute());
  expect(mocks.send).toHaveBeenCalledTimes(4);
  expect(result.current.run?.attempts).toHaveLength(4);
});

it.each(["cancelled", "replaced", "repriced"])(
  "handles a %s transaction without treating any successful replacement as the intended action",
  async (reason) => {
    const replacementHash = `0x${"cd".repeat(32)}`;
    mocks.receipt.mockImplementationOnce(async ({ onReplaced }) => {
      onReplaced({ reason, transaction: { hash: replacementHash } });
      return { status: "success" };
    });
    const { result } = renderHook(
      () => useRecipeExecution({ recipe, prepared }),
      { wrapper },
    );
    await act(() => result.current.execute());
    expect(result.current.progress[0]?.replacementHash).toBe(replacementHash);
    expect(result.current.progress[0]?.phase).toBe(
      reason === "cancelled"
        ? "failed"
        : reason === "replaced"
          ? "attention"
          : "complete",
    );
    expect(mocks.send).toHaveBeenCalledTimes(reason === "repriced" ? 2 : 1);
    if (reason === "replaced") {
      await act(() => result.current.checkStatus());
      expect(result.current.progress[0]?.phase).toBe("attention");
    }
  },
);

it("does not continue with the next action if the wallet changes during approval", async () => {
  const { result, rerender } = renderHook(
    () => useRecipeExecution({ recipe, prepared }),
    { wrapper },
  );
  mocks.receipt.mockImplementationOnce(async () => {
    mocks.account = {
      address: "0x3333333333333333333333333333333333333333",
      chainId: 56,
    };
    flushSync(() => rerender());
    return { status: "success" };
  });
  await act(() => result.current.execute());
  expect(mocks.send).toHaveBeenCalledTimes(1);
});

it("unmounting during approval must not start the following trade", async () => {
  const { result, unmount } = renderHook(
    () => useRecipeExecution({ recipe, prepared }),
    { wrapper },
  );
  mocks.receipt.mockImplementationOnce(async () => {
    unmount();
    return { status: "success" };
  });
  await act(() => result.current.execute());
  expect(mocks.send).toHaveBeenCalledTimes(1);
});

it("waits for SWAP settlement and does not replay a completed cloned preparation", async () => {
  const { result, rerender } = renderHook(
    ({ plan }) => useRecipeExecution({ recipe, prepared: plan }),
    { wrapper, initialProps: { plan: prepared } },
  );
  await act(() => result.current.execute());
  expect(result.current.completed).toBe(false);
  expect(readRun(wallet, 56)?.attempts.at(-1)?.transactionHash).toBe(hash);
  mocks.transaction.mockResolvedValue({
    state: "succeeded",
    sold: [],
    bought: [],
  });
  await act(() => result.current.checkStatus());
  await waitFor(() => expect(result.current.completed).toBe(true));
  rerender({ plan: structuredClone(prepared) });
  await act(() => result.current.execute());
  expect(mocks.send).toHaveBeenCalledTimes(2);
});

it("RFQ submission persists the order but completion requires a fill", async () => {
  const rfq = {
    steps: [
      {
        kind: "TRADE" as const,
        nodeId: "trade",
        action: {
          executionMode: "RFQ" as const,
          submissionId: "submission",
          routeQuoteId: "route",
          venue: "inchFusion" as const,
          typedData: JSON.stringify({
            domain: {},
            types: {},
            primaryType: "Order",
            message: {},
          }),
        },
      },
    ],
  };
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared: rfq }),
    { wrapper },
  );
  await act(() => result.current.execute());
  expect(result.current.completed).toBe(false);
  expect(readRun(wallet, 56)?.attempts[0]?.orderId).toBe("order");
  expect(JSON.stringify(readRun(wallet, 56))).not.toContain("0xsigned");
  mocks.order.mockResolvedValue({
    state: "filled",
    filledBuyAmount: "23",
    filledSellAmount: "100",
  });
  await act(() => result.current.checkStatus());
  await waitFor(() => expect(result.current.completed).toBe(true));
});

it.each([true, false])(
  "recovers a known hash without any wallet prompt (locking: %s)",
  async (locking) => {
    if (!locking)
      Object.defineProperty(navigator, "locks", {
        configurable: true,
        value: undefined,
      });
    writeRun({
      version: 1,
      id: "existing",
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
          phase: "pending",
          executionMode: "SWAP",
          transactionHash: hash,
        },
      ],
    });
    mocks.transaction.mockResolvedValue({ state: "succeeded" });
    const { result } = renderHook(() => useRecipeExecution({ recipe }), {
      wrapper,
    });
    await waitFor(() => expect(result.current.completed).toBe(true));
    expect(mocks.send).not.toHaveBeenCalled();
    expect(mocks.sign).not.toHaveBeenCalled();
    if (!locking)
      expect(readRun(wallet, 56)?.attempts[0]?.phase).toBe("pending");
  },
);

it("blocks resend after a refresh loses a submission reference", async () => {
  writeRun({
    version: 1,
    id: "existing",
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
        phase: "submitting",
        executionMode: "RFQ",
        submissionId: "submission",
      },
    ],
  });
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared }),
    { wrapper },
  );
  await act(() => result.current.execute());
  expect(result.current.blocked).toBe(true);
  expect(mocks.send).not.toHaveBeenCalled();
  expect(mocks.submit).not.toHaveBeenCalled();
});

it("double-click cannot open a second wallet request", async () => {
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared }),
    { wrapper },
  );
  await act(async () => {
    await Promise.all([result.current.execute(), result.current.execute()]);
  });
  expect(mocks.send).toHaveBeenCalledTimes(2);
});

it("fails closed before submission when persistence is unavailable", async () => {
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared }),
    { wrapper },
  );
  const spy = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
    throw new Error("quota");
  });
  await act(() => result.current.execute());
  expect(mocks.send).not.toHaveBeenCalled();
  expect(result.current.error).toMatch(/sav|stor|recovery/i);
  spy.mockRestore();
});

const rfqPrepared = {
  steps: [
    {
      kind: "TRADE" as const,
      nodeId: "trade",
      action: {
        executionMode: "RFQ" as const,
        submissionId: "submission",
        routeQuoteId: "route",
        venue: "inchFusion" as const,
        typedData: JSON.stringify({
          domain: {},
          types: {},
          primaryType: "Order",
          message: {},
        }),
      },
    },
  ],
};

it("retries an uncertain RFQ with exactly the original signature and submission ID", async () => {
  mocks.submit.mockRejectedValueOnce(new Error("connection lost"));
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared: rfqPrepared }),
    { wrapper },
  );
  await act(() => result.current.execute());
  expect(result.current.canRetrySubmission).toBe(true);
  await act(() => result.current.retrySubmission());
  expect(mocks.sign).toHaveBeenCalledTimes(1);
  expect(mocks.submit.mock.calls[1][0]).toEqual({
    submissionId: "submission",
    routeQuoteId: "route",
    venue: "inchFusion",
    signature: "0xsigned",
  });
  expect(result.current.run?.attempts[0]?.orderId).toBe("order");
});

it("does not retry an uncertain RFQ after the deduplication window", async () => {
  vi.useFakeTimers();
  mocks.submit.mockRejectedValueOnce(new Error("connection lost"));
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared: rfqPrepared }),
    { wrapper },
  );
  await act(() => result.current.execute());
  vi.setSystemTime(Date.now() + 30 * 60_000);
  await act(() => result.current.retrySubmission());
  expect(mocks.submit).toHaveBeenCalledTimes(1);
});

it("bounds unknown-status polling and never uses the timeout to resend", async () => {
  vi.useFakeTimers();
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared }),
    { wrapper },
  );
  await act(() => result.current.execute());
  await act(async () => {
    await vi.advanceTimersByTimeAsync(121_000);
  });
  expect(result.current.paused).toBe(true);
  const reads = mocks.transaction.mock.calls.length;
  await act(async () => {
    await vi.advanceTimersByTimeAsync(30_000);
  });
  expect(mocks.transaction).toHaveBeenCalledTimes(reads);
  expect(mocks.send).toHaveBeenCalledTimes(2);
});

it("honors Retry-After even if Check status is clicked immediately", async () => {
  mocks.transaction.mockRejectedValue(
    new StockfrontApiError("request_failed", "rate limited", [], 60_000),
  );
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared }),
    { wrapper },
  );
  await act(() => result.current.execute());
  await waitFor(() => expect(mocks.transaction).toHaveBeenCalled());
  const reads = mocks.transaction.mock.calls.length;
  await act(() => result.current.checkStatus());
  expect(mocks.transaction).toHaveBeenCalledTimes(reads);
});

it("does not continue polling while offline", async () => {
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared }),
    { wrapper },
  );
  await act(() => result.current.execute());
  Object.defineProperty(navigator, "onLine", {
    configurable: true,
    value: false,
  });
  act(() => window.dispatchEvent(new Event("offline")));
  const reads = mocks.transaction.mock.calls.length;
  await act(() => result.current.checkStatus());
  expect(mocks.transaction).toHaveBeenCalledTimes(reads);
  Object.defineProperty(navigator, "onLine", {
    configurable: true,
    value: true,
  });
});

it("does not send completed nodes again when resuming a later independent trade", async () => {
  const largerRecipe = {
    ...recipe,
    nodes: [
      ...recipe.nodes,
      { id: "trade2", kind: "TRADE" as const, buyToken: wallet },
    ],
  };
  writeRun({
    version: 1,
    id: "existing",
    wallet,
    chainId: 56,
    recipe: largerRecipe,
    createdAtMs: 1,
    updatedAtMs: 1,
    attempts: [
      {
        nodeId: "trade",
        kind: "TRADE",
        attempt: 1,
        phase: "complete",
        executionMode: "SWAP",
        transactionHash: hash,
      },
    ],
  });
  const plan = {
    steps: [
      ...prepared.steps,
      ...prepared.steps.map((step) => ({ ...step, nodeId: "trade2" })),
    ],
  };
  const { result } = renderHook(
    () => useRecipeExecution({ recipe: largerRecipe, prepared: plan }),
    { wrapper },
  );
  await act(() => result.current.execute());
  expect(mocks.send).toHaveBeenCalledTimes(2);
  expect(result.current.run?.attempts.map((item) => item.nodeId)).toEqual([
    "trade",
    "trade2",
    "trade2",
  ]);
});
