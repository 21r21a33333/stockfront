import type { RecipeNode, TradeQuote } from "@/api/generated";
import type { ExecutionAttempt } from "@/recipe/execution/model";
import { priceMoved, runSteps } from "./steps";

const wallet: RecipeNode = {
  id: "w",
  kind: "WALLET_ASSET",
  token: "0x0000000000000000000000000000000000000001",
  rawAmount: "1",
};
const trade: RecipeNode = {
  id: "t",
  kind: "TRADE",
  buyToken: "0x0000000000000000000000000000000000000002",
};
const supply: RecipeNode = { id: "s", kind: "SUPPLY", venue: "venus" };
const borrow: RecipeNode = {
  id: "b",
  kind: "BORROW",
  debtToken: "0x0000000000000000000000000000000000000001",
  rawAmount: "1",
};

function attempt(
  nodeId: string,
  kind: ExecutionAttempt["kind"],
  phase: ExecutionAttempt["phase"],
): ExecutionAttempt {
  return {
    id: `${nodeId}-${kind}-${phase}`,
    nodeId,
    kind,
    attempt: 1,
    phase,
    steps: [kind],
  };
}

describe("runSteps", () => {
  it("lists only action blocks and makes the first unfinished one current", () => {
    const steps = runSteps(
      [wallet, trade, supply],
      [attempt("t", "TRADE", "complete")],
      false,
    );

    expect(steps.map((step) => [step.node.id, step.state])).toEqual([
      ["t", "done"],
      ["s", "ready"],
    ]);
  });

  it("never treats collateral activation as a completed borrow", () => {
    const steps = runSteps(
      [borrow],
      [attempt("b", "ENABLE_COLLATERAL", "complete")],
      false,
    );

    expect(steps[0]).toMatchObject({ state: "ready", collateralEnabled: true });
  });

  it("maps wallet rejection, pending settlement and lost submissions to distinct states", () => {
    expect(
      runSteps([trade], [attempt("t", "TRADE", "failed")], false)[0]?.state,
    ).toBe("cancelled");
    expect(
      runSteps([trade], [attempt("t", "TRADE", "pending")], false)[0]?.state,
    ).toBe("confirming");
    expect(
      runSteps([trade], [attempt("t", "TRADE", "attention")], false)[0]?.state,
    ).toBe("unknown");
    expect(
      runSteps([trade], [attempt("t", "TRADE", "awaiting")], true)[0]?.state,
    ).toBe("waiting");
  });
});

describe("priceMoved", () => {
  const quote = (amount: string): TradeQuote => ({
    routeQuoteId: amount,
    venue: "LiquidMesh",
    executionMode: "SWAP",
    sell: { token: "0x0000000000000000000000000000000000000001", amount: "1" },
    buy: { token: "0x0000000000000000000000000000000000000002", amount },
  });

  it("asks again only when the fresh quote is worse beyond the tolerance", () => {
    expect(priceMoved(quote("10000"), quote("9995"))).toBeUndefined();
    expect(priceMoved(quote("10000"), quote("9900"))).toEqual({
      was: 10000n,
      now: 9900n,
    });
    expect(priceMoved(quote("10000"), quote("10500"))).toBeUndefined();
    expect(priceMoved(undefined, quote("1"))).toBeUndefined();
  });
});
