import { recipeRuntimeContext } from "./runtimeContext";
import { wallet } from "./fixtures.test-support";

it("builds backend runtime context from only settled trade attempts", () => {
  expect(
    recipeRuntimeContext(wallet, [
      {
        id: "trade:1",
        nodeId: "trade",
        kind: "TRADE",
        attempt: 1,
        phase: "complete",
        steps: ["TRADE"],
        settledOutput: {
          port: "buy",
          asset: { token: wallet, amount: "23" },
        },
      },
      {
        id: "trade2:1",
        nodeId: "trade2",
        kind: "TRADE",
        attempt: 1,
        phase: "pending",
        steps: ["TRADE"],
      },
      {
        id: "borrow:2",
        nodeId: "borrow",
        kind: "BORROW",
        attempt: 2,
        phase: "complete",
        executionMode: "BORROW",
        steps: ["BORROW"],
        settledOutput: {
          port: "borrowed",
          asset: { token: wallet, amount: "11" },
        },
      },
    ]),
  ).toEqual({
    wallet,
    completedNodes: [
      {
        nodeId: "trade",
        outputs: [
          {
            port: "buy",
            asset: { token: wallet, amount: "23" },
          },
        ],
      },
      {
        nodeId: "borrow",
        outputs: [
          {
            port: "borrowed",
            asset: { token: wallet, amount: "11" },
          },
        ],
      },
    ],
  });
});
