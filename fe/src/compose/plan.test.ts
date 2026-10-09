import { planBlocks, planTitle } from "./plan";

const stablecoins = new Set(["USDT", "USDC"]);

describe("planBlocks", () => {
  it("labels a sale differently from a purchase so the user is not shown the wrong direction", () => {
    const blocks = planBlocks(
      [
        { kind: "WALLET_ASSET", symbol: "TSLAB", amount: "0.5" },
        { kind: "TRADE", buySymbol: "USDT" },
      ],
      stablecoins,
    );

    expect(blocks.map((block) => block.name)).toEqual([
      "Wallet TSLAB",
      "Sell for USDT",
    ]);
    expect(blocks[0]?.detail).toBe("0.5 TSLAB");
  });

  it("marks a missing amount instead of inventing one", () => {
    const blocks = planBlocks(
      [{ kind: "BORROW", symbol: "USDT", amount: null }],
      stablecoins,
    );

    expect(blocks[0]?.detail).toBe("Amount needed");
  });
});

describe("planTitle", () => {
  it("names the strategy after its outcome", () => {
    expect(
      planTitle(
        [
          { kind: "WALLET_ASSET", symbol: "USDT", amount: "6" },
          { kind: "TRADE", buySymbol: "NVDAB" },
          { kind: "SUPPLY" },
        ],
        stablecoins,
      ),
    ).toBe("Buy & supply NVDAB");
    expect(
      planTitle(
        [
          { kind: "SUPPLIED_POSITION", symbol: "TSLAB" },
          { kind: "BORROW", symbol: "USDT", amount: "1" },
        ],
        stablecoins,
      ),
    ).toBe("Borrow against TSLAB");
    expect(planTitle([], stablecoins)).toBe("Your strategy");
  });
});
