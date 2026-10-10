import type { CarryOpportunity, PreparedRecipe } from "@/api/generated";
import { preparedExecution } from "./preparedExecution";

function opportunity(): CarryOpportunity {
  return {
    asset: { contract: market, symbol: "USDT" },
    availableBorrowLiquidity: "100",
    baseEarnRateBps: 900,
    baseEarnRateDisplay: "9.00%",
    borrowApyPercent: "5",
    investmentId: "investment",
    investmentName: "Lista USDT",
    observedAtMs: 1,
    protocolId: "lista",
    protocolName: "Lista",
    qualification: "ELIGIBLE",
    rateType: "APY",
    reasons: [],
  };
}

const owner = "0x0000000000000000000000000000000000000001" as const;
const market = "0x0000000000000000000000000000000000000002" as const;
const transaction = {
  kind: "WITHDRAW" as const,
  to: market,
  data: "0x12345678" as const,
  value: "0",
  gasLimit: "150000",
  gasPrice: "1",
};

it("turns one full withdrawal into one wallet call", async () => {
  const prepared: PreparedRecipe = {
    steps: [
      {
        kind: "WITHDRAW",
        nodeId: "withdraw",
        action: { transactions: [transaction], preview: {} },
      },
    ],
  };

  await expect(
    preparedExecution(prepared, {} as never, owner),
  ).resolves.toEqual({
    kind: "CALL_BATCH",
    nodeId: "withdraw",
    action: "WITHDRAW",
    executionMode: "WITHDRAW",
    steps: ["WITHDRAW"],
    calls: [{ to: market, data: "0x12345678", value: 0n }],
    atomicRequired: false,
  });
});

it.each([
  ["no transaction", []],
  ["two transactions", [transaction, transaction]],
  ["wrong transaction kind", [{ ...transaction, kind: "DEPOSIT" as const }]],
])("rejects a withdrawal with %s", async (_label, transactions) => {
  const prepared: PreparedRecipe = {
    steps: [
      {
        kind: "WITHDRAW",
        nodeId: "withdraw",
        action: { transactions, preview: {} },
      },
    ],
  };

  await expect(preparedExecution(prepared, {} as never, owner)).rejects.toThrow(
    "The prepared withdrawal must contain one withdrawal transaction.",
  );
});

it.each([
  ["ENABLE_COLLATERAL" as const, undefined],
  [
    "BORROW" as const,
    {
      port: "borrowed",
      asset: { token: market, amount: "1000000000000000000" },
    },
  ],
])(
  "prepares the %s borrow stage as one simulated wallet call",
  async (kind, output) => {
    const prepared: PreparedRecipe = {
      steps: [
        kind === "BORROW"
          ? {
              kind,
              nodeId: "borrow",
              action: {
                transactions: [{ ...transaction, kind }],
                preview: {},
              },
              output: output!.asset,
            }
          : {
              kind,
              nodeId: "borrow",
              action: {
                transactions: [{ ...transaction, kind }],
                preview: {},
              },
            },
      ],
    };
    const request = vi.fn().mockResolvedValue("0x");

    await expect(
      preparedExecution(prepared, { request }, owner),
    ).resolves.toMatchObject({
      kind: "CALL_BATCH",
      nodeId: "borrow",
      action: kind,
      executionMode: "BORROW",
      steps: [kind],
      calls: [{ to: market, data: "0x12345678", value: 0n }],
      ...(output ? { output } : {}),
    });
    expect(request).toHaveBeenCalledOnce();
  },
);

it("keeps approval before full repayment in one atomic wallet batch", async () => {
  const prepared: PreparedRecipe = {
    steps: [
      {
        kind: "REPAY",
        nodeId: "repay",
        action: {
          transactions: [
            {
              kind: "APPROVE",
              to: market,
              data: "0x095ea7b3",
              value: "0",
            },
            {
              kind: "REPAY",
              to: owner,
              data: "0x12345678",
              value: "0",
            },
          ],
          preview: {},
        },
      },
    ],
  };

  await expect(
    preparedExecution(prepared, {} as never, owner),
  ).resolves.toEqual({
    kind: "CALL_BATCH",
    nodeId: "repay",
    action: "REPAY",
    executionMode: "REPAY",
    steps: ["APPROVAL", "REPAY"],
    calls: [
      { to: market, data: "0x095ea7b3", value: 0n },
      { to: owner, data: "0x12345678", value: 0n },
    ],
    atomicRequired: true,
  });
});

it("keeps approval before an earn deposit in one atomic wallet batch", async () => {
  const prepared: PreparedRecipe = {
    steps: [
      {
        kind: "EARN_DEPOSIT",
        nodeId: "earn",
        action: {
          opportunity: opportunity(),
          transactions: [
            { kind: "APPROVE", to: market, data: "0x095ea7b3", value: "0" },
            { kind: "DEPOSIT", to: owner, data: "0x12345678", value: "0" },
          ],
          preview: {},
        },
      },
    ],
  };

  await expect(
    preparedExecution(prepared, {} as never, owner),
  ).resolves.toEqual({
    kind: "CALL_BATCH",
    nodeId: "earn",
    action: "EARN_DEPOSIT",
    executionMode: "EARN_DEPOSIT",
    steps: ["EARN_DEPOSIT"],
    calls: [
      { to: market, data: "0x095ea7b3", value: 0n },
      { to: owner, data: "0x12345678", value: 0n },
    ],
    atomicRequired: true,
  });
});

it("refuses an earn deposit whose provider plan is missing the deposit", async () => {
  const prepared: PreparedRecipe = {
    steps: [
      {
        kind: "EARN_DEPOSIT",
        nodeId: "earn",
        action: {
          opportunity: opportunity(),
          transactions: [
            { kind: "APPROVE", to: market, data: "0x095ea7b3", value: "0" },
          ],
          preview: {},
        },
      },
    ],
  };

  await expect(preparedExecution(prepared, {} as never, owner)).rejects.toThrow(
    "The prepared earn deposit is incomplete.",
  );
});
