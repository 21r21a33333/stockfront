import type { PreparedRecipe } from "@/api/generated";
import { preparedExecution } from "./preparedExecution";

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
