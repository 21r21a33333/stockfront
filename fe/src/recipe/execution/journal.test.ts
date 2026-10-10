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
import { hash, recipe, token, wallet } from "./fixtures.test-support";

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

it("reads back a recipe built from the earn primitive", () => {
  // A node kind the journal does not know fails the whole run on read, so the
  // record is written and then refused the moment it is needed.
  const earnRecipe: typeof recipe = {
    schemaVersion: 1,
    chainId: 56,
    nodes: [
      { id: "source", kind: "WALLET_ASSET", token, rawAmount: "100" },
      {
        id: "earn",
        kind: "EARN_DEPOSIT",
        collateralToken: token,
        investmentId: "investment",
      },
    ],
    edges: [
      {
        id: "edge",
        from: { nodeId: "source", port: "asset" },
        to: { nodeId: "earn", port: "deposit" },
      },
    ],
  };
  const stored = attempt({
    id: "earn:1",
    nodeId: "earn",
    kind: "EARN_DEPOSIT",
    executionMode: "EARN_DEPOSIT",
    steps: ["EARN_DEPOSIT"],
    phase: "complete",
    reference: {
      kind: "CALL_BATCH",
      id: "batch",
      atomicRequired: false,
      transactionHash: hash,
    },
  });
  writeRun({ ...run, recipe: earnRecipe, attempts: [stored] });

  expect(readRun(wallet, 56)?.attempts).toEqual([stored]);
});

it("finishes a run whose only action is an earn deposit", () => {
  // A recipe action the completion check does not recognise leaves the run
  // forever unfinished, so the finished step is prepared again and again.
  const earnRecipe: typeof recipe = {
    schemaVersion: 1,
    chainId: 56,
    nodes: [
      { id: "source", kind: "WALLET_ASSET", token, rawAmount: "100" },
      {
        id: "earn",
        kind: "EARN_DEPOSIT",
        collateralToken: token,
        investmentId: "investment",
      },
    ],
    edges: [
      {
        id: "edge",
        from: { nodeId: "source", port: "asset" },
        to: { nodeId: "earn", port: "deposit" },
      },
    ],
  };
  const done = attempt({
    id: "earn:1",
    nodeId: "earn",
    kind: "EARN_DEPOSIT",
    executionMode: "EARN_DEPOSIT",
    steps: ["EARN_DEPOSIT"],
    phase: "complete",
  });

  expect(isRunComplete({ ...run, recipe: earnRecipe, attempts: [done] })).toBe(
    true,
  );
  expect(completedRecipeNodes([done])).toEqual([
    { nodeId: "earn", outputs: [] },
  ]);
});

it("reads back every action the recipe language can execute", () => {
  // A kind the journal does not accept is written and then rejected on the way
  // back, which strands the run that used it rather than failing at the write.
  const kinds = [
    "APPROVAL",
    "TRADE",
    "SUPPLY",
    "EARN_DEPOSIT",
    "WITHDRAW",
    "ENABLE_COLLATERAL",
    "BORROW",
    "REPAY",
  ] as const;

  for (const kind of kinds) {
    localStorage.clear();
    const stored = attempt({
      id: `trade:${kind}`,
      kind,
      steps: [kind],
      executionMode: undefined,
      reference: { kind: "CALL_BATCH", id: "batch", atomicRequired: false },
    });
    writeRun({ ...run, attempts: [stored] });

    expect(readRun(wallet, 56)?.attempts).toEqual([stored]);
  }
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
