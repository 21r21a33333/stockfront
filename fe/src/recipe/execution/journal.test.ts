import {
  isRunComplete,
  latestAttempts,
  readRun,
  writeRun,
  type ExecutionRun,
} from "./journal";
import { recipe, wallet, hash } from "./fixtures.test-support";

const run: ExecutionRun = {
  version: 1,
  id: "run",
  wallet,
  chainId: 56,
  recipe,
  createdAtMs: 1,
  updatedAtMs: 1,
  attempts: [],
};
beforeEach(() => localStorage.clear());

it("isolates runs by wallet and chain and preserves submitted references", () => {
  writeRun({
    ...run,
    attempts: [
      {
        nodeId: "trade",
        kind: "TRADE",
        attempt: 1,
        phase: "pending",
        transactionHash: hash,
        executionMode: "SWAP",
      },
    ],
  });
  expect(
    readRun(wallet.toUpperCase() as typeof wallet, 56)?.attempts[0]
      ?.transactionHash,
  ).toBe(hash);
  expect(readRun(wallet, 1)).toBeUndefined();
});
it("requires every trade to settle, not just its approval or submission", () => {
  expect(isRunComplete(run)).toBe(false);
  const attempt = {
    nodeId: "trade",
    kind: "TRADE",
    attempt: 1,
    phase: "pending",
    orderId: "order",
  } as const;
  expect(isRunComplete({ ...run, attempts: [attempt] })).toBe(false);
  expect(
    isRunComplete({ ...run, attempts: [{ ...attempt, phase: "complete" }] }),
  ).toBe(true);
});
it("keeps prior attempts but projects only the latest action state", () => {
  expect(
    latestAttempts([
      { nodeId: "trade", kind: "TRADE", attempt: 1, phase: "failed" },
      { nodeId: "trade", kind: "TRADE", attempt: 2, phase: "pending" },
    ]),
  ).toHaveLength(1);
});
it.each(["{", '{"version":2}', '{"version":1,"attempts":[]}'])(
  "does not silently discard corrupt recovery data: %s",
  (value) => {
    localStorage.setItem(`stockfront:execution:56:${wallet}`, value);
    expect(() => readRun(wallet, 56)).toThrow(/recovery/i);
  },
);
it("does not serialize arbitrary properties or authorizations", () => {
  writeRun({ ...run, signature: "secret", attempts: [] } as ExecutionRun);
  expect(
    localStorage.getItem(`stockfront:execution:56:${wallet}`),
  ).not.toContain("secret");
});
it("propagates failed durable writes", () => {
  const spy = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
    throw new Error("quota");
  });
  expect(() => writeRun(run)).toThrow();
  spy.mockRestore();
});

it("rejects corrupted snapshot amounts instead of crashing when restoring the editor", () => {
  writeRun({
    ...run,
    recipe: {
      ...recipe,
      nodes: [
        { id: "source", kind: "WALLET_ASSET", token: wallet, rawAmount: "bad" },
      ],
    },
  });
  expect(() => readRun(wallet, 56)).toThrow(/recovery/i);
});
