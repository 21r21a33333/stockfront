import { describe, expect, it } from "vitest";
import { filterTokens, type TokenOption } from "./tokenSearch";

const token = (symbol: string, name: string): TokenOption => ({
  address: `0x${symbol.padEnd(40, "0")}` as `0x${string}`,
  symbol,
  name,
});

const options = [
  token("AMDB", "Advanced Micro Devices"),
  token("MRVLB", "Marvell Technology, Inc."),
  token("NVDAB", "NVIDIA"),
  token("TSLAB", "Tesla, Inc."),
];

describe("filterTokens", () => {
  it("finds a stock by company name when the ticker differs", () => {
    expect(filterTokens(options, "tesla").map((item) => item.symbol)).toEqual([
      "TSLAB",
    ]);
  });

  it("ranks ticker prefixes, then ticker substrings, then names", () => {
    expect(filterTokens(options, "a").map((item) => item.symbol)).toEqual([
      "AMDB",
      "NVDAB",
      "TSLAB",
      "MRVLB",
    ]);
  });

  it("returns nothing rather than everything for an unknown query", () => {
    expect(filterTokens(options, "zzz")).toEqual([]);
  });
});
