import { expect, test, type Page } from "@playwright/test";
import type { ComposedStrategy, RecipeCatalog } from "../src/api/generated";

const usdt = "0x55d398326f99059fF775485246999027B3197955";
const nvda = "0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436";
const tsla = "0x2222222222222222222222222222222222222222";
const pool = "0x1111111111111111111111111111111111111111";

const catalog = {
  schemaVersion: 1,
  primitives: [],
  settlementTokens: [usdt],
  tradePolicy: {
    allowedTransitions: [
      { sell: "SETTLEMENT_TOKENS", buy: "BSTOCKS" },
      { sell: "BSTOCKS", buy: "SETTLEMENT_TOKENS" },
    ],
  },
  supplyPolicy: { allowedInputs: [{ input: "BSTOCKS", venue: "venus" }] },
  withdrawalPolicy: { allowedInputs: [{ venue: "venus", pool }] },
  borrowPolicy: {
    allowedInputs: [{ input: "SUPPLIED_POSITIONS", venue: "venus", pool }],
    debtTokens: [usdt],
    minimumHealthFactor: "1.5",
  },
  repayPolicy: {
    allowedInputs: [{ input: "BORROWED_POSITIONS", venue: "venus", pool }],
    debtTokens: [usdt],
  },
  earnDepositPolicy: { allowedInputs: ["BORROW"], depositTokens: [usdt] },
  baskets: [],
  bstocks: [
    {
      chainId: 56,
      contract: nvda,
      tokenName: "NVIDIA (bStocks)",
      tokenSymbol: "NVDAB",
      tokenDecimals: 18,
      underlyingTicker: "NVDA",
      underlyingName: "NVIDIA",
      assetKind: "stock",
      tokenToShareRatio: "1",
      tokenPriceUsd: "182.10",
      referencePriceUsd: "182.50",
      marketStatus: { session: "closed" },
      tags: [],
    },
    {
      chainId: 56,
      contract: tsla,
      tokenName: "Tesla (bStocks)",
      tokenSymbol: "TSLAB",
      tokenDecimals: 18,
      underlyingTicker: "TSLA",
      underlyingName: "Tesla, Inc.",
      assetKind: "stock",
      tokenToShareRatio: "1",
      tokenPriceUsd: "430.10",
      referencePriceUsd: "430.50",
      marketStatus: { session: "closed" },
      tags: [],
    },
  ],
} as unknown as RecipeCatalog;

const buyNvidia: ComposedStrategy = {
  summary: "Swap 6 USDT for NVDAB.",
  intent: {
    steps: [
      { kind: "WALLET_ASSET", symbol: "USDT", amount: "6" },
      { kind: "TRADE", buySymbol: "NVDAB" },
    ],
  },
  recipe: {
    schemaVersion: 1,
    chainId: 56,
    nodes: [
      {
        id: "53545246-0000-4000-8000-000000000000",
        kind: "WALLET_ASSET",
        token: usdt,
        rawAmount: "6000000000000000000",
      },
      {
        id: "53545246-0000-4000-8000-000000000001",
        kind: "TRADE",
        buyToken: nvda,
      },
    ],
    edges: [
      {
        id: "53545246-0000-4000-8000-000100000001",
        from: { nodeId: "53545246-0000-4000-8000-000000000000", port: "asset" },
        to: { nodeId: "53545246-0000-4000-8000-000000000001", port: "sell" },
      },
    ],
  },
  assumptions: ["$6 was interpreted as 6 USDT."],
  missingInputs: [],
  validation: { valid: true, diagnostics: [] },
};

async function mockApi(page: Page, compose: { status: number; body: object }) {
  await page.route("**/test-api/**", async (route) => {
    const path = new URL(route.request().url()).pathname.replace(
      "/test-api",
      "",
    );
    const reply = (status: number, body: object) =>
      route.fulfill({
        status,
        contentType: "application/json",
        body: JSON.stringify(body),
      });
    if (path === "/v1/recipe-primitives")
      return reply(200, { status: "Ok", result: catalog });
    if (path === "/v1/recipes/validate")
      return reply(200, {
        status: "Ok",
        result: { valid: true, diagnostics: [] },
      });
    if (path === "/v1/strategies/compose")
      return reply(compose.status, compose.body);
    return reply(404, {
      status: "Error",
      error: { code: "not_found", message: "not mocked" },
    });
  });
}

test("a plain-English prompt becomes a strategy that opens in the builder", async ({
  page,
}) => {
  await mockApi(page, {
    status: 200,
    body: { status: "Ok", result: buyNvidia },
  });
  await page.goto("/");

  await page.getByLabel("Describe your strategy").fill("Buy $6 of Nvidia");
  await page.getByRole("button", { name: "Compose this strategy" }).click();

  const plan = page.getByRole("region", { name: "AI plan" });
  await expect(plan.getByRole("heading", { name: "Buy NVDAB" })).toBeVisible();
  await expect(plan.getByText("Ready")).toBeVisible();
  await expect(plan.getByText("$6 was interpreted as 6 USDT.")).toBeVisible();
  await expect(plan.getByText(/US market closed/)).toBeVisible();

  await plan.getByRole("button", { name: "Open in builder" }).click();

  await expect(page).toHaveURL(/#\/builder$/);
  const canvas = page.getByRole("region", { name: "Strategy canvas" });
  await expect(
    canvas.getByRole("button", { name: /Wallet USDT/ }),
  ).toBeVisible();
  await expect(canvas.getByRole("button", { name: /Buy NVDAB/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /^Run/ })).toBeDisabled();
});

test("missing inputs ask the user instead of opening a guessed strategy", async ({
  page,
}) => {
  await mockApi(page, {
    status: 200,
    body: {
      status: "Ok",
      result: {
        ...buyNvidia,
        intent: {
          steps: [
            { kind: "WALLET_ASSET", symbol: "USDT", amount: null },
            { kind: "TRADE", buySymbol: "NVDAB" },
          ],
        },
        recipe: null,
        validation: null,
        missingInputs: ["How much USDT should the strategy use?"],
      },
    },
  });
  await page.goto("/#/compose");

  await page.getByLabel("Describe your strategy").fill("Buy some Nvidia");
  await page.getByRole("button", { name: "Compose", exact: true }).click();

  const plan = page.getByRole("region", { name: "AI plan" });
  await expect(
    plan.getByText("How much USDT should the strategy use?"),
  ).toBeVisible();
  await expect(plan.getByText("Needs your input")).toBeVisible();
  await expect(
    plan.getByRole("button", { name: "Open in builder" }),
  ).toBeDisabled();
});

test("a rate-limited AI shows a plain explanation and nothing is opened", async ({
  page,
}) => {
  await mockApi(page, {
    status: 503,
    body: {
      status: "Error",
      error: { code: "ai_rate_limited", message: "rate limited" },
    },
  });
  await page.goto("/#/compose");

  await page.getByLabel("Describe your strategy").fill("Buy $6 of Nvidia");
  await page.getByRole("button", { name: "Compose", exact: true }).click();

  await expect(page.getByRole("alert")).toContainText(
    "The AI is busy right now.",
  );
  await expect(page.getByRole("region", { name: "AI plan" })).toHaveCount(0);
});

test("the stock picker finds a stock by company name", async ({ page }) => {
  await mockApi(page, {
    status: 200,
    body: { status: "Ok", result: buyNvidia },
  });
  await page.goto("/#/compose");
  await page.getByLabel("Describe your strategy").fill("Buy $6 of Nvidia");
  await page.getByRole("button", { name: "Compose", exact: true }).click();
  await page
    .getByRole("region", { name: "AI plan" })
    .getByRole("button", { name: "Open in builder" })
    .click();

  const canvas = page.getByRole("region", { name: "Strategy canvas" });
  await canvas.getByRole("button", { name: /Buy NVDAB/ }).click();
  const picker = page.getByRole("combobox", { name: "Stock" });
  await picker.click();
  await picker.fill("tesla");

  const options = page.getByRole("option");
  await expect(options).toHaveCount(1);
  await expect(options.first()).toContainText("Tesla, Inc.");
  await picker.press("Enter");

  await expect(canvas.getByRole("button", { name: /Buy TSLAB/ })).toBeVisible();
  await expect(page.getByRole("listbox")).toBeHidden();

  await picker.click();
  await expect(page.getByRole("option")).toHaveCount(2);
  await page.getByRole("option", { name: /NVDAB/ }).hover();
  await page
    .getByRole("complementary", { name: "Inspector" })
    .getByText("Buy TSLAB", { exact: true })
    .click();
  await expect(page.getByRole("listbox")).toBeHidden();
  await expect(canvas.getByRole("button", { name: /Buy TSLAB/ })).toBeVisible();
});
