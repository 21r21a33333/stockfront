import { expect, test } from "@playwright/test";
import type { RecipeCatalog } from "../src/api/generated";

const catalog = {
  schemaVersion: 1,
  settlementTokens: ["0x55d398326f99059fF775485246999027B3197955"],
  tradePolicy: {
    allowedTransitions: [
      { sell: "SETTLEMENT_TOKENS", buy: "BSTOCKS" },
      { sell: "BSTOCKS", buy: "SETTLEMENT_TOKENS" },
    ],
  },
  supplyPolicy: {
    allowedInputs: [{ input: "BSTOCKS", venue: "venus" }],
  },
  borrowPolicy: {
    allowedInputs: [
      {
        input: "SUPPLIED_POSITIONS",
        venue: "venus",
        pool: "0x1111111111111111111111111111111111111111",
      },
    ],
    debtTokens: ["0x55d398326f99059fF775485246999027B3197955"],
    minimumHealthFactor: "1.5",
  },
  withdrawalPolicy: {
    allowedInputs: [
      {
        venue: "venus",
        pool: "0x1111111111111111111111111111111111111111",
      },
    ],
  },
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
    {
      kind: "SUPPLIED_POSITION",
      name: "Supplied position",
      description: "Use an existing supplied Venus position.",
      fields: [],
      ports: [],
    },
    {
      kind: "SUPPLY",
      name: "Supply",
      description: "Supply the incoming bStock to Venus.",
      fields: [],
      ports: [],
    },
    {
      kind: "WITHDRAW",
      name: "Withdraw",
      description: "Withdraw a full Venus supplied position.",
      fields: [],
      ports: [],
    },
    {
      kind: "BORROW",
      name: "Borrow stablecoin",
      description: "Borrow against a supplied Venus position.",
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
  await expect(
    page.getByRole("button", { name: /^Borrow stablecoin/ }),
  ).toBeVisible();
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

  await page.getByRole("button", { name: "Add next action" }).click();
  const nextActions = page.locator(".react-flow__node").nth(2);
  await expect(
    nextActions.getByRole("button", { name: "Trade" }),
  ).toBeEnabled();
  await expect(
    nextActions.getByRole("button", { name: "Supply" }),
  ).toBeDisabled();

  await page.getByRole("button", { name: "Select asset" }).click();
  await expect(
    page.getByRole("dialog", { name: "Select token" }),
  ).toBeVisible();
  await page.getByRole("textbox", { name: "Search tokens" }).fill("usdt");
  await expect(page.getByRole("button", { name: /Tether USD/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /Nvidia Corp/ })).toHaveCount(
    0,
  );
});

test("adds a terminal Venus supply from a supported bStock output", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Add next action" }).click();

  const trade = page.locator(".react-flow__node").nth(1);
  await expect(trade.getByRole("button", { name: "Supply" })).toBeEnabled();
  await trade.getByRole("button", { name: "Supply" }).click();

  await expect(page.locator(".react-flow__node")).toHaveCount(3);
  await expect(
    page
      .locator(".react-flow__node")
      .nth(2)
      .getByText("Supply", { exact: true }),
  ).toBeVisible();
  await expect(page.getByText("Supply NVDAB to Venus")).toBeVisible();
  await expect(page.getByText("Asset<NVDAB> · from Trade")).toBeVisible();
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

test("builds and prepares a full Venus position withdrawal", async ({
  page,
}) => {
  const wallet = "0x2222222222222222222222222222222222222222";
  const pool = "0x1111111111111111111111111111111111111111";
  const stock = catalog.bstocks[0].contract;
  await page.addInitScript((address) => {
    Object.defineProperty(window, "ethereum", {
      value: {
        isMetaMask: true,
        on() {},
        removeListener() {},
        async request({ method }: { method: string }) {
          if (method === "eth_chainId") return "0x38";
          if (method === "eth_requestAccounts" || method === "eth_accounts")
            return [address];
          if (method === "wallet_requestPermissions")
            return [{ parentCapability: "eth_accounts" }];
          throw new Error(`Unsupported test wallet method: ${method}`);
        },
      },
    });
  }, wallet);
  await page.route("**/test-api/v1/wallets/*/loans", (route) =>
    route.fulfill({
      json: {
        status: "Ok",
        result: {
          chainId: 56,
          venue: "venus",
          positions: [
            {
              pool,
              supplied: [
                {
                  token: { contract: stock, symbol: "NVDAB" },
                  amount: "1.25",
                  valueUsd: "450.00",
                },
              ],
              borrowed: [],
            },
          ],
          totalSuppliedUsd: "450.00",
          totalBorrowedUsd: "0",
          observedAtMs: Date.now(),
        },
      },
    }),
  );
  await page.route("**/test-api/v1/recipes/preview", async (route) => {
    const body = route.request().postDataJSON();
    const nodeId = body.recipe.nodes.find(
      (node: { kind: string }) => node.kind === "WITHDRAW",
    ).id;
    await route.fulfill({
      json: {
        status: "Ok",
        result: {
          nodes: [
            {
              kind: "WITHDRAW",
              nodeId,
              state: "READY",
              position: {
                token: { contract: stock, symbol: "NVDAB" },
                amount: "1.25",
                valueUsd: "450.00",
              },
            },
          ],
        },
      },
    });
  });
  await page.route("**/test-api/v1/recipes/prepare", async (route) => {
    const body = route.request().postDataJSON();
    const nodeId = body.recipe.nodes.find(
      (node: { kind: string }) => node.kind === "WITHDRAW",
    ).id;
    await route.fulfill({
      json: {
        status: "Ok",
        result: {
          steps: [
            {
              kind: "WITHDRAW",
              nodeId,
              action: {
                transactions: [
                  {
                    kind: "WITHDRAW",
                    to: pool,
                    data: "0x12345678",
                    value: "0",
                  },
                ],
              },
            },
          ],
        },
      },
    });
  });

  await page.goto("/");
  const connect = page.getByRole("button", { name: "Connect wallet" });
  if (await connect.isVisible()) await connect.click();
  page.once("dialog", (dialog) => void dialog.accept());
  await page.getByRole("button", { name: "New recipe" }).click();
  await page.getByRole("button", { name: /^Supplied position/ }).click();
  await expect(
    page.locator(".react-flow__node").getByText("1.25 NVDAB", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Add position action" }).click();
  await page.getByRole("button", { name: "Withdraw", exact: true }).click();
  await page.getByRole("button", { name: "Preview route" }).click();
  await expect(
    page.getByRole("button", { name: "Prepare withdrawal" }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "Prepare withdrawal" }).click();
  await expect(page.getByText("Withdraw all NVDAB from Venus")).toBeVisible();
});
