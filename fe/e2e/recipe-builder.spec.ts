import { expect, test } from "@playwright/test";
import type { RecipeCatalog } from "../src/api/generated";

const catalog = {
  schemaVersion: 1,
  settlementTokens: ["0x55d398326f99059fF775485246999027B3197955"],
  bstocks: [
    {
      chainId: 56,
      contract: "0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436",
      tokenName: "NVIDIA (bStocks)",
      tokenSymbol: "NVDAB",
      tokenDecimals: 18,
      underlyingTicker: "NVDA",
      underlyingName: "Nvidia Corp",
      assetKind: "stock",
      tokenToShareRatio: "1",
      tags: [],
    },
  ],
  primitives: [
    {
      kind: "WALLET_ASSET",
      name: "Wallet asset",
      description: "Use an asset held by the connected wallet.",
      fields: [],
      ports: [],
    },
    {
      kind: "TRADE",
      name: "Trade",
      description: "Trade the incoming asset for another token.",
      fields: [],
      ports: [],
    },
  ],
} satisfies RecipeCatalog;

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => localStorage.clear());
  await page.route("**/test-api/v1/recipe-primitives", async (route) => {
    await route.fulfill({ json: { status: "Ok", result: catalog } });
  });
  await page.route("**/test-api/v1/recipes/validate", async (route) => {
    await route.fulfill({
      json: { status: "Ok", result: { valid: true, diagnostics: [] } },
    });
  });
});

test("builds, inspects, searches, and saves a recipe draft", async ({
  page,
}) => {
  await page.goto("/");

  await expect(page.locator(".react-flow__node")).toHaveCount(2);
  await expect(page.getByText("Inputs", { exact: true })).toBeVisible();
  await expect(
    page.getByText("Connect a wallet to preview live routes"),
  ).toBeVisible();

  await page.getByRole("button", { name: /^Wallet asset/ }).click();
  await expect(page.locator(".react-flow__node")).toHaveCount(3);

  const search = page.getByRole("textbox", { name: "Search actions" });
  await search.fill("borrow");
  await expect(page.getByText("Borrow", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: /^Trade/ })).toHaveCount(0);

  await search.fill("");
  const overview = page.getByRole("navigation", { name: "Overview" });
  await overview.getByRole("button", { name: "Debug" }).click();
  await expect(page.getByText("Compiler input")).toBeVisible();
  await expect(page.getByText("Recipe JSON")).toBeVisible();

  await overview.getByRole("button", { name: "Recipe Builder" }).click();
  await page.getByRole("button", { name: "Save draft" }).click();
  await expect(page.getByText("Draft saved locally")).toBeVisible();
  await expect
    .poll(() =>
      page.evaluate(() =>
        Boolean(localStorage.getItem("stockfront.recipe-builder.draft.v1")),
      ),
    )
    .toBe(true);
});

test("shows asset flow guides, adds the next action, and searches supported tokens", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.getByText("Asset<USDT>", { exact: true })).toBeVisible();
  await expect(page.getByText("Asset<NVDAB>", { exact: true })).toBeVisible();
  await expect
    .poll(
      async () =>
        (await page.locator(".react-flow__node").first().boundingBox())?.y,
    )
    .toBeLessThan(400);

  await page.getByRole("button", { name: "Add next action" }).click();
  await page
    .locator(".react-flow__node")
    .nth(1)
    .getByRole("button", { name: "Trade" })
    .click();
  await expect(page.locator(".react-flow__node")).toHaveCount(3);
  await expect(page.getByText("Asset<NVDAB>", { exact: true })).toBeVisible();
  await expect(page.locator(".react-flow__node").nth(2)).toBeInViewport();

  await page.getByRole("button", { name: "Select asset" }).click();
  await expect(
    page.getByRole("dialog", { name: "Select token" }),
  ).toBeVisible();
  await page.getByRole("textbox", { name: "Search tokens" }).fill("nvda");
  await expect(page.getByRole("button", { name: /Nvidia Corp/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /Tether USD/ })).toHaveCount(0);
});

test("keeps the output asset label clear of the trade node and next action", async ({
  page,
}) => {
  await page.goto("/");
  const trade = await page.locator(".react-flow__node").nth(1).boundingBox();
  const label = await page
    .getByText("Asset<NVDAB>", { exact: true })
    .boundingBox();
  const next = await page
    .getByRole("button", { name: "Add next action" })
    .boundingBox();
  expect(trade && label && next).toBeTruthy();
  if (!trade || !label || !next) return;
  expect(label.x).toBeGreaterThanOrEqual(trade.x + trade.width + 5);
  expect(label.y + label.height).toBeLessThanOrEqual(next.y - 4);
});

test("shows a single focus ring around each text field", async ({ page }) => {
  await page.goto("/");
  const actionSearch = page.getByRole("textbox", { name: "Search actions" });
  await actionSearch.focus();
  await expect(actionSearch).toHaveCSS("outline-style", "none");
  await expect(actionSearch.locator("..")).not.toHaveCSS("box-shadow", "none");

  await page.getByRole("button", { name: "Select asset" }).click();
  const tokenSearch = page.getByRole("textbox", { name: "Search tokens" });
  await expect(tokenSearch).toBeFocused();
  await expect(tokenSearch).toHaveCSS("outline-style", "none");
  await expect(tokenSearch.locator("..")).not.toHaveCSS("box-shadow", "none");

  await page.getByRole("button", { name: "Close token selector" }).click();
  await page.locator(".react-flow__node").first().click();
  const amount = page.getByRole("textbox", { name: "Amount" });
  await amount.focus();
  await expect(amount).toHaveCSS("outline-style", "none");
  await expect(amount.locator("..")).not.toHaveCSS("box-shadow", "none");
});
