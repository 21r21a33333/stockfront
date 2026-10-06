import {
  completedRecipeNodes,
  isRunComplete,
  latestAttempts,
  readRun,
  runKey,
  writeRun,
  type ExecutionAttempt,
  type ExecutionRun,
} from "./journal";
import { hash, recipe, wallet } from "./fixtures.test-support";

const run: ExecutionRun = {
  version: 2,
  id: "run",
  wallet,
  chainId: 56,
  recipe,
  createdAtMs: 1,
  updatedAtMs: 1,
  attempts: [],
};
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

beforeEach(() => localStorage.clear());

it("round-trips the one durable atomic attempt and its exact output", () => {
  const complete = attempt({
    phase: "complete",
    reference: {
      kind: "CALL_BATCH",
      id: "batch",
      atomicRequired: true,
      transactionHash: hash,
    },
    settledOutput: {
      port: "buy",
      asset: { token: wallet, amount: "23" },
    },
  });
  writeRun({ ...run, attempts: [complete] });

  const restored = readRun(wallet.toUpperCase(), 56)!;
  expect(restored.attempts).toEqual([complete]);
  expect(isRunComplete(restored)).toBe(true);
  expect(completedRecipeNodes(restored.attempts)).toEqual([
    { nodeId: "trade", outputs: [complete.settledOutput] },
  ]);
  expect(readRun(wallet, 1)).toBeUndefined();
});

it("migrates version 1 approval and trade rows sharing one batch into one attempt", () => {
  localStorage.setItem(
    runKey(wallet, 56),
    JSON.stringify({
      ...run,
      version: 1,
      attempts: [
        {
          nodeId: "trade",
          kind: "APPROVAL",
          attempt: 1,
          phase: "pending",
          callBatchId: "batch",
        },
        {
          nodeId: "trade",
          kind: "TRADE",
          attempt: 1,
          phase: "pending",
          executionMode: "SWAP",
          callBatchId: "batch",
        },
      ],
    }),
  );

  expect(readRun(wallet, 56)?.attempts).toEqual([
    expect.objectContaining({
      nodeId: "trade",
      kind: "TRADE",
      steps: ["APPROVAL", "TRADE"],
      reference: {
        kind: "CALL_BATCH",
        id: "batch",
        atomicRequired: true,
      },
    }),
  ]);
});

it("keeps history but exposes only the latest node attempt", () => {
  expect(
    latestAttempts([
      attempt({ phase: "failed" }),
      attempt({ id: "trade:2", attempt: 2 }),
    ]),
  ).toEqual([expect.objectContaining({ id: "trade:2" })]);
});

it.each(["{", '{"version":2}', '{"version":1,"attempts":[]}'])(
  "rejects corrupt recovery data instead of silently clearing it: %s",
  (value) => {
    localStorage.setItem(runKey(wallet, 56), value);
    expect(() => readRun(wallet, 56)).toThrow(/recovery/i);
  },
);

it("propagates failed durable writes", () => {
  const spy = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
    throw new Error("quota");
  });
  expect(() => writeRun(run)).toThrow();
  spy.mockRestore();
});

it("does not persist accidental authorization fields", () => {
  writeRun({ ...run, signature: "secret" } as ExecutionRun);
  expect(localStorage.getItem(runKey(wallet, 56))).not.toContain("secret");
});

it("rejects a settled token that cannot be the recipe node output", () => {
  writeRun({
    ...run,
    attempts: [
      attempt({
        phase: "complete",
        settledOutput: {
          port: "buy",
          asset: {
            token: "0x3333333333333333333333333333333333333333",
            amount: "1",
          },
        },
      }),
    ],
  });
  expect(() => readRun(wallet, 56)).toThrow(/recovery/i);
});
