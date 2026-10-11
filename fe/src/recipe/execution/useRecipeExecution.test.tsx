import { act, renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState, type PropsWithChildren } from "react";
import {
  encodeAbiParameters,
  encodeEventTopics,
  encodeFunctionData,
  erc20Abi,
} from "viem";
import type { PreparedRecipe, RecipeDefinition } from "@/api/generated";
import { hash, recipe, token, wallet } from "./fixtures.test-support";
import { createRun, readRun, writeRun } from "./journal";
import { StockfrontApiError } from "@/api/client";
import { useRecipeExecution } from "./useRecipeExecution";

const mocks = vi.hoisted(() => ({
  sendCalls: vi.fn(),
  getCapabilities: vi.fn(),
  getCallsStatus: vi.fn(),
  publicRequest: vi.fn(),
  getReceipt: vi.fn(),
  sign: vi.fn(),
  submit: vi.fn(),
  order: vi.fn(),
  submitBasket: vi.fn(),
  basketOrder: vi.fn(),
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
      sendCalls: mocks.sendCalls,
      getCapabilities: mocks.getCapabilities,
      getCallsStatus: mocks.getCallsStatus,
      signTypedData: mocks.sign,
    },
  }),
  usePublicClient: () => ({
    request: mocks.publicRequest,
    getTransactionReceipt: mocks.getReceipt,
  }),
}));
vi.mock("@/api/recipes", () => ({
  recipesApi: {
    submitOrder: mocks.submit,
    orderStatus: mocks.order,
    submitBasketOrder: mocks.submitBasket,
    basketOrderStatus: mocks.basketOrder,
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

beforeEach(() => {
  localStorage.clear();
  vi.clearAllMocks();
  mocks.account = { address: wallet, chainId: 56 };
  mocks.sendCalls.mockResolvedValue({ id: "batch" });
  mocks.getCapabilities.mockResolvedValue({ atomic: { status: "supported" } });
  mocks.getCallsStatus.mockResolvedValue({
    status: "pending",
    atomic: true,
    receipts: [],
  });
  mocks.publicRequest.mockResolvedValue("0x0");
  mocks.getReceipt.mockResolvedValue({ status: "success", logs: [] });
  mocks.sign.mockResolvedValue("0xsigned");
  mocks.submit.mockResolvedValue({
    orderId: "order",
    state: "pendingVendor",
    createdAtMs: 1,
  });
  mocks.order.mockResolvedValue({ state: "pendingVendor" });
  mocks.submitBasket.mockResolvedValue({ orderId: "basket-order" });
  mocks.basketOrder.mockResolvedValue({ state: "pendingVendor" });
  Object.defineProperty(navigator, "onLine", {
    configurable: true,
    value: true,
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

it("submits approval and swap as one atomic attempt", async () => {
  const plan = preparedSwap();
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared: plan }),
    { wrapper: Wrapper },
  );

  await act(() => result.current.execute());

  expect(mocks.sendCalls).toHaveBeenCalledWith(
    expect.objectContaining({ forceAtomic: true, calls: expect.any(Array) }),
  );
  expect(mocks.sendCalls.mock.calls[0][0].calls).toHaveLength(2);
  expect(result.current.progress).toEqual([
    expect.objectContaining({
      kind: "TRADE",
      phase: "pending",
      steps: ["APPROVAL", "TRADE"],
      reference: { kind: "CALL_BATCH", id: "batch", atomicRequired: true },
    }),
  ]);
});

it("sends nothing when atomic execution is unsupported", async () => {
  mocks.getCapabilities.mockResolvedValue({
    atomic: { status: "unsupported" },
  });
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared: preparedSwap() }),
    { wrapper: Wrapper },
  );

  await act(() => result.current.execute());

  expect(mocks.sendCalls).not.toHaveBeenCalled();
  expect(result.current.run).toBeUndefined();
  expect(result.current.error).toMatch(/Atomic batching is not supported/);
});

it("settles an atomic batch once and restores the exact output", async () => {
  const plan = preparedSwap();
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared: plan }),
    { wrapper: Wrapper },
  );
  await act(() => result.current.execute());
  mocks.getCallsStatus.mockResolvedValue({
    status: "success",
    atomic: true,
    receipts: [
      {
        transactionHash: hash,
        logs: [transferLog(wallet, token, wallet, 23n)],
      },
    ],
  });

  await act(() => result.current.checkStatus());

  await waitFor(() => expect(result.current.completed).toBe(true));
  expect(readRun(wallet, 56)?.attempts[0]).toMatchObject({
    phase: "complete",
    settledOutput: {
      port: "buy",
      asset: { token: wallet, amount: "23" },
    },
  });
  expect(mocks.sendCalls).toHaveBeenCalledTimes(1);
});

it("settles from the chain when the wallet has forgotten the batch", async () => {
  const plan = preparedSwap();
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared: plan }),
    { wrapper: Wrapper },
  );
  await act(() => result.current.execute());

  // The batch id is only known for the life of the wallet session, so after a
  // reload the hash captured while it was still pending is the way back.
  mocks.getCallsStatus.mockResolvedValueOnce({
    status: "pending",
    receipts: [{ transactionHash: hash }],
  });
  await act(() => result.current.checkStatus());
  expect(readRun(wallet, 56)?.attempts[0]?.reference).toMatchObject({
    transactionHash: hash,
  });

  mocks.getCallsStatus.mockRejectedValue(new Error("unknown bundle id"));
  mocks.getReceipt.mockResolvedValue({
    status: "success",
    logs: [transferLog(wallet, token, wallet, 23n)],
  });

  await act(() => result.current.checkStatus());

  await waitFor(() => expect(result.current.completed).toBe(true));
  expect(mocks.getReceipt).toHaveBeenCalledWith({ hash });
  expect(mocks.sendCalls).toHaveBeenCalledTimes(1);
});

it("states a wallet cancellation once, on the step that was cancelled", async () => {
  mocks.sendCalls.mockRejectedValue(
    Object.assign(new Error("User rejected the request."), { code: 4001 }),
  );
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared: preparedSwap() }),
    { wrapper: Wrapper },
  );

  await act(() => result.current.execute());

  // The step says it was cancelled and nothing was sent; a page-level error
  // beside it would read as a second, worse problem.
  await waitFor(() =>
    expect(readRun(wallet, 56)?.attempts.at(-1)?.phase).toBe("failed"),
  );
  expect(result.current.error).toBeUndefined();
});

it("reports a refused order as refused, not as an unknown outcome", async () => {
  mocks.submit.mockRejectedValue(
    new StockfrontApiError("trade_rejected", "the venue rejected the trade"),
  );
  mocks.publicRequest.mockImplementation(async ({ params }) =>
    params[0].data.startsWith("0xdd62ed3e") ? "0x64" : "0x",
  );
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared: preparedRfq() }),
    { wrapper: Wrapper },
  );

  await act(() => result.current.execute());

  // The venue answered, so nothing was created and the wallet is not worth
  // auditing; the route is what needs refreshing.
  await waitFor(() =>
    expect(readRun(wallet, 56)?.attempts.at(-1)?.phase).toBe("failed"),
  );
  expect(readRun(wallet, 56)?.attempts.at(-1)?.error).toContain(
    "Refresh the route",
  );
});

it("polls a basket order at the venue that quoted it", async () => {
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared: preparedBasketRfq() }),
    { wrapper: Wrapper },
  );

  await act(() => result.current.execute());

  // The reference is rewritten when the order id arrives, and losing the venue
  // there sends every later poll to the wrong endpoint.
  await waitFor(() =>
    expect(readRun(wallet, 56)?.attempts.at(-1)?.reference).toMatchObject({
      kind: "RFQ_ORDER",
      orderId: "basket-order",
      market: "BASKET",
    }),
  );
  expect(mocks.submitBasket).toHaveBeenCalledTimes(1);
  expect(mocks.submit).not.toHaveBeenCalled();

  mocks.basketOrder.mockResolvedValue({
    state: "filled",
    filledBuyAmount: "23",
    transactionHash: hash,
  });
  await act(() => result.current.checkStatus());

  await waitFor(() => expect(result.current.completed).toBe(true));
  expect(mocks.basketOrder).toHaveBeenCalled();
  expect(mocks.order).not.toHaveBeenCalled();
});

it("waits out a venue that has not indexed the order yet", async () => {
  mocks.submit.mockResolvedValue({ orderId: "order" });
  mocks.order.mockRejectedValue(
    new StockfrontApiError(
      "trade_order_not_found",
      "the trade order was not found",
    ),
  );
  mocks.publicRequest.mockImplementation(async ({ params }) =>
    params[0].data.startsWith("0xdd62ed3e") ? "0x64" : "0x",
  );
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared: preparedRfq() }),
    { wrapper: Wrapper },
  );
  await act(() => result.current.execute());

  // The order filled or is about to; the venue simply cannot see it yet, which
  // is not a reason to strand the run.
  await act(() => result.current.checkStatus());
  expect(result.current.error).toBeUndefined();
  expect(readRun(wallet, 56)?.attempts.at(-1)?.phase).toBe("pending");

  mocks.order.mockResolvedValue({
    state: "filled",
    filledBuyAmount: "23",
    transactionHash: hash,
  });
  await act(() => result.current.checkStatus());

  await waitFor(() => expect(result.current.completed).toBe(true));
});

it("skips a proven allowance and simulates before sending one swap call", async () => {
  mocks.publicRequest.mockImplementation(async ({ params }) =>
    params[0].data.startsWith("0xdd62ed3e") ? "0x64" : "0x",
  );
  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared: preparedSwap() }),
    { wrapper: Wrapper },
  );

  await act(() => result.current.execute());

  expect(mocks.getCapabilities).not.toHaveBeenCalled();
  expect(mocks.sendCalls.mock.calls[0][0]).toMatchObject({
    forceAtomic: false,
  });
  expect(mocks.sendCalls.mock.calls[0][0].calls).toHaveLength(1);
  expect(result.current.progress[0]).toMatchObject({
    approvalSkipped: true,
    steps: ["TRADE"],
  });
});

it("submits approval and Venus supply as one atomic terminal action", async () => {
  const supplyRecipe: RecipeDefinition = {
    schemaVersion: 1,
    chainId: 56,
    nodes: [
      { id: "source", kind: "WALLET_ASSET", token, rawAmount: "100" },
      { id: "supply", kind: "SUPPLY", venue: "venus" },
    ],
    edges: [
      {
        id: "edge",
        from: { nodeId: "source", port: "asset" },
        to: { nodeId: "supply", port: "asset" },
      },
    ],
  };
  const plan: PreparedRecipe = {
    steps: [
      {
        kind: "SUPPLY",
        nodeId: "supply",
        action: {
          transactions: [
            {
              kind: "APPROVE",
              to: token,
              data: "0x095ea7b3",
              value: "0",
            },
            {
              kind: "DEPOSIT",
              to: wallet,
              data: "0x12345678",
              value: "0",
            },
          ],
          preview: {},
        },
      },
    ],
  };
  const { result } = renderHook(
    () => useRecipeExecution({ recipe: supplyRecipe, prepared: plan }),
    { wrapper: Wrapper },
  );

  await act(() => result.current.execute());

  expect(mocks.sendCalls).toHaveBeenCalledWith(
    expect.objectContaining({ forceAtomic: true, calls: expect.any(Array) }),
  );
  expect(mocks.sendCalls.mock.calls[0][0].calls).toHaveLength(2);
  expect(result.current.progress[0]).toMatchObject({
    kind: "SUPPLY",
    executionMode: "SUPPLY",
    steps: ["SUPPLY"],
    phase: "pending",
  });

  mocks.getCallsStatus.mockResolvedValue({
    status: "success",
    atomic: true,
    receipts: [{ transactionHash: hash, logs: [] }],
  });
  await act(() => result.current.checkStatus());
  await waitFor(() => expect(result.current.completed).toBe(true));
});

it("sends no repayment call when the wallet cannot guarantee atomic execution", async () => {
  mocks.getCapabilities.mockResolvedValue({
    atomic: { status: "unsupported" },
  });
  const repaymentRecipe: RecipeDefinition = {
    schemaVersion: 1,
    chainId: 56,
    nodes: [
      {
        id: "debt",
        kind: "BORROWED_POSITION",
        venue: "venus",
        pool: wallet,
        token,
      },
      { id: "repay", kind: "REPAY" },
    ],
    edges: [
      {
        id: "repay-edge",
        from: { nodeId: "debt", port: "debt" },
        to: { nodeId: "repay", port: "debt" },
      },
    ],
  };
  const repayment: PreparedRecipe = {
    steps: [
      {
        kind: "REPAY",
        nodeId: "repay",
        action: {
          transactions: [
            {
              kind: "APPROVE",
              to: token,
              data: "0x095ea7b3",
              value: "0",
            },
            {
              kind: "REPAY",
              to: wallet,
              data: "0x12345678",
              value: "0",
            },
          ],
          preview: {},
        },
      },
    ],
  };
  const { result } = renderHook(
    () => useRecipeExecution({ recipe: repaymentRecipe, prepared: repayment }),
    { wrapper: Wrapper },
  );

  await act(() => result.current.execute());

  expect(mocks.sendCalls).not.toHaveBeenCalled();
  expect(result.current.run).toBeUndefined();
  expect(result.current.error).toMatch(/Atomic batching is not supported/);
});

it("settles collateral activation before borrowing and exposes the exact debt output", async () => {
  const borrowRecipe: RecipeDefinition = {
    schemaVersion: 1,
    chainId: 56,
    nodes: [
      {
        id: "position",
        kind: "SUPPLIED_POSITION",
        venue: "venus",
        pool: wallet,
        token,
      },
      {
        id: "borrow",
        kind: "BORROW",
        debtToken: wallet,
        rawAmount: "11",
      },
    ],
    edges: [
      {
        id: "edge",
        from: { nodeId: "position", port: "position" },
        to: { nodeId: "borrow", port: "collateral" },
      },
    ],
  };
  const activation: PreparedRecipe = {
    steps: [
      {
        kind: "ENABLE_COLLATERAL",
        nodeId: "borrow",
        action: {
          transactions: [
            {
              kind: "ENABLE_COLLATERAL",
              to: wallet,
              data: "0x12345678",
              value: "0",
            },
          ],
          preview: {},
        },
      },
    ],
  };
  const borrow: PreparedRecipe = {
    steps: [
      {
        kind: "BORROW",
        nodeId: "borrow",
        action: {
          transactions: [
            {
              kind: "BORROW",
              to: wallet,
              data: "0x12345678",
              value: "0",
            },
          ],
          preview: {},
        },
        output: { token: wallet, amount: "11" },
      },
    ],
  };
  const { result } = renderHook(
    () => useRecipeExecution({ recipe: borrowRecipe, prepared: activation }),
    { wrapper: Wrapper },
  );

  await act(() => result.current.execute());
  mocks.getCallsStatus.mockResolvedValue({
    status: "success",
    atomic: true,
    receipts: [{ transactionHash: hash, logs: [] }],
  });
  await act(() => result.current.checkStatus());
  await waitFor(() =>
    expect(result.current.requiresFreshPreparation).toBe(true),
  );
  expect(result.current.completed).toBe(false);

  await act(() => result.current.execute(borrow));
  await act(() => result.current.checkStatus());

  await waitFor(() => expect(result.current.completed).toBe(true));
  expect(readRun(wallet, 56)?.attempts.at(-1)).toMatchObject({
    kind: "BORROW",
    phase: "complete",
    settledOutput: {
      port: "borrowed",
      asset: { token: wallet, amount: "11" },
    },
  });
});

it("lets a finished run for another draft out of the way", async () => {
  const other: RecipeDefinition = {
    ...recipe,
    nodes: [
      { id: "other-source", kind: "WALLET_ASSET", token, rawAmount: "100" },
      { id: "other-trade", kind: "TRADE", buyToken: wallet },
    ],
    edges: [],
  };
  writeRun({
    ...createRun(other, wallet),
    attempts: [
      {
        id: "old",
        nodeId: "other-trade",
        kind: "TRADE",
        attempt: 1,
        phase: "complete",
        steps: ["TRADE"],
      },
    ],
  });

  const { result } = renderHook(
    () => useRecipeExecution({ recipe, prepared: preparedSwap() }),
    { wrapper: Wrapper },
  );

  expect(result.current.completed).toBe(false);
  expect(result.current.unfinishedOtherRun).toBe(false);

  await act(() => result.current.execute());

  expect(mocks.sendCalls).toHaveBeenCalled();
  expect(readRun(wallet, 56)?.recipe.nodes).toEqual(recipe.nodes);
});

function preparedBasketRfq(): PreparedRecipe {
  const rfq = preparedRfq();
  return {
    steps: rfq.steps.map((step) =>
      step.kind === "TRADE" ? { ...step, kind: "BASKET_TRADE" as const } : step,
    ),
  };
}

function preparedRfq(): PreparedRecipe {
  return {
    steps: [
      {
        kind: "TRADE",
        nodeId: "trade",
        action: {
          executionMode: "RFQ",
          submissionId: "00000000-0000-4000-8000-000000000001",
          routeQuoteId: "quote",
          venue: "inchFusion",
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
}

function preparedSwap(): PreparedRecipe {
  return {
    steps: [
      {
        kind: "APPROVAL",
        nodeId: "trade",
        action: {
          to: token,
          spender: wallet,
          data: encodeFunctionData({
            abi: erc20Abi,
            functionName: "approve",
            args: [wallet, 100n],
          }),
        },
      },
      {
        kind: "TRADE",
        nodeId: "trade",
        action: {
          executionMode: "SWAP",
          transaction: {
            to: wallet,
            data: "0x12345678",
            value: "0",
            gasLimit: "100000",
            gasPrice: "1",
            minReceiveAmount: "1",
          },
        },
      },
    ],
  };
}

function transferLog(
  address: `0x${string}`,
  from: `0x${string}`,
  to: `0x${string}`,
  value: bigint,
) {
  return {
    address,
    topics: encodeEventTopics({
      abi: erc20Abi,
      eventName: "Transfer",
      args: { from, to },
    }),
    data: encodeAbiParameters([{ type: "uint256" }], [value]),
  };
}
