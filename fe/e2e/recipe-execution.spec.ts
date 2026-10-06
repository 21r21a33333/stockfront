import { expect, test, type BrowserContext, type Page } from "@playwright/test";
import type { RecipeDefinition } from "../src/api/generated";

const wallet = "0x1111111111111111111111111111111111111111";
const usdt = "0x55d398326f99059fF775485246999027B3197955";
const stock = "0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436";

async function mockApp(
  context: BrowserContext,
  executionMode: "RFQ" | "SWAP" = "RFQ",
) {
  let snapshot: RecipeDefinition;
  let state = "pendingVendor";
  let submissions = 0;
  let loseResponse = false;
  await context.addInitScript(
    ({ wallet }) => {
      const listeners = new Map<string, Array<(value: unknown) => void>>();
      const provider = {
        isMetaMask: true,
        on(event: string, callback: (value: unknown) => void) {
          listeners.set(event, [...(listeners.get(event) ?? []), callback]);
        },
        removeListener() {},
        async request({ method }: { method: string }) {
          if (method === "eth_chainId") return "0x38";
          if (method === "eth_requestAccounts" || method === "eth_accounts")
            return [wallet];
          if (method === "wallet_requestPermissions")
            return [{ parentCapability: "eth_accounts" }];
          if (method === "eth_signTypedData_v4") {
            sessionStorage.setItem(
              "signatures",
              String(Number(sessionStorage.getItem("signatures") ?? "0") + 1),
            );
            return `0x${"11".repeat(65)}`;
          }
          throw new Error(`Unsupported test wallet method: ${method}`);
        },
      };
      Object.defineProperty(window, "ethereum", { value: provider });
    },
    { wallet },
  );
  await context.route("**/test-api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    const ok = (result: unknown) =>
      route.fulfill({ json: { status: "Ok", result } });
    if (path.endsWith("/recipe-primitives"))
      return ok({
        schemaVersion: 1,
        settlementTokens: [usdt],
        tradePolicy: {
          allowedTransitions: [
            { sell: "SETTLEMENT_TOKENS", buy: "BSTOCKS" },
            { sell: "BSTOCKS", buy: "SETTLEMENT_TOKENS" },
          ],
        },
        bstocks: [
          {
            chainId: 56,
            contract: stock,
            tokenName: "NVIDIA",
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
            description: "Wallet asset",
            fields: [],
            ports: [],
          },
          {
            kind: "TRADE",
            name: "Trade",
            description: "Trade",
            fields: [],
            ports: [],
          },
        ],
      });
    if (path.endsWith("/validate")) return ok({ valid: true, diagnostics: [] });
    if (path.endsWith("/preview") || path.endsWith("/prepare")) {
      const body = route.request().postDataJSON();
      snapshot = body.recipe;
      const nodeId = body.recipe.nodes.find(
        (node: { kind: string }) => node.kind === "TRADE",
      ).id;
      if (path.endsWith("/preview"))
        return ok({
          nodes: [
            {
              kind: "TRADE",
              nodeId,
              state: "READY",
              routes: [
                {
                  routeQuoteId: "route",
                  venue: "InchFusion",
                  executionMode,
                  sell: { token: usdt, amount: "6000000000000000000" },
                  buy: { token: stock, amount: "12300000000000000" },
                },
              ],
            },
          ],
        });
      if (executionMode === "SWAP")
        return ok({
          steps: [
            {
              kind: "APPROVAL",
              nodeId,
              action: { to: usdt, spender: stock, data: "0x" },
            },
            {
              kind: "TRADE",
              nodeId,
              action: {
                executionMode: "SWAP",
                transaction: {
                  to: stock,
                  data: "0x",
                  value: "0",
                  minReceiveAmount: "12000000000000000",
                },
              },
            },
          ],
        });
      return ok({
        steps: [
          {
            kind: "TRADE",
            nodeId,
            action: {
              executionMode: "RFQ",
              submissionId: "submission",
              routeQuoteId: "route",
              venue: "inchFusion",
              typedData: JSON.stringify({
                domain: { name: "Test Order", version: "1", chainId: 56 },
                primaryType: "Order",
                types: { Order: [{ name: "amount", type: "uint256" }] },
                message: { amount: "6" },
              }),
            },
          },
        ],
      });
    }
    if (path.endsWith("/orders") && route.request().method() === "POST") {
      submissions++;
      if (loseResponse) return route.abort();
      return ok({
        orderId: "order-1",
        state: "pendingVendor",
        createdAtMs: Date.now(),
      });
    }
    if (path.endsWith("/orders/order-1"))
      return ok({
        state,
        ...(state === "filled"
          ? {
              filledSellAmount: "6000000000000000000",
              filledBuyAmount: "12300000000000000",
            }
          : {}),
      });
    return route.fulfill({ status: 404, json: {} });
  });
  // These tests never read or submit to a real RPC service.
  await context.route(
    /https:\/\/.*(?:bnbchain|binance|publicnode).*\//,
    async (route) => {
      const request = route.request().postDataJSON();
      const answer = (item: { id: number }) => ({
        jsonrpc: "2.0",
        id: item.id,
        error: { code: -32601, message: "RPC disabled in browser test" },
      });
      await route.fulfill({
        json: Array.isArray(request) ? request.map(answer) : answer(request),
      });
    },
  );
  return {
    fill: () => {
      state = "filled";
    },
    submissions: () => submissions,
    recipe: () => snapshot,
    loseSubmissionResponse: () => {
      loseResponse = true;
    },
  };
}

async function prepare(page: Page) {
  await page.goto("/");
  const connect = page.getByRole("button", { name: "Connect wallet" });
  if (await connect.isVisible()) await connect.click();
  await expect(
    page.getByRole("button", { name: "Preview route" }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "Preview route" }).click();
  await page.getByRole("button", { name: "Prepare transactions" }).click();
  await expect(
    page.getByRole("button", { name: "Execute recipe" }),
  ).toBeEnabled();
}

test("RFQ stays pending after submission, survives reload, and completes without another signature", async ({
  context,
  page,
}) => {
  const mock = await mockApp(context);
  await prepare(page);
  await page.getByRole("button", { name: "Execute recipe" }).click();
  await expect(page.getByText("Order: order-1")).toBeAttached();
  await expect(
    page.getByRole("button", { name: "Execute recipe" }),
  ).toHaveCount(0);
  await page.reload();
  await expect(page.getByText("Order: order-1")).toBeAttached();
  mock.fill();
  await page.getByRole("button", { name: "Check status" }).click();
  await expect(page.getByText("Recipe settled")).toBeVisible();
  await page.getByRole("button", { name: "Restore execution recipe" }).click();
  await expect(
    page
      .locator(".react-flow__node")
      .filter({ hasText: "Trade" })
      .getByText("Complete", { exact: true }),
  ).toBeVisible();
  expect(mock.submissions()).toBe(1);
  expect(await page.evaluate(() => sessionStorage.getItem("signatures"))).toBe(
    "1",
  );
});

test("a recovered buy shows approval and swap once in Activity and keeps the quote collapsed", async ({
  context,
  page,
}) => {
  const mock = await mockApp(context, "SWAP");
  await prepare(page);
  await page.evaluate(
    ({ wallet, recipe }) => {
      const nodeId = recipe.nodes.find((node) => node.kind === "TRADE")!.id;
      const key = `stockfront:execution:56:${wallet}`;
      localStorage.setItem(
        key,
        JSON.stringify({
          version: 1,
          id: "settled-run",
          wallet,
          chainId: 56,
          recipe,
          createdAtMs: Date.now(),
          updatedAtMs: Date.now(),
          attempts: [
            {
              nodeId,
              kind: "APPROVAL",
              attempt: 1,
              phase: "complete",
              callBatchId: "settled-batch",
            },
            {
              nodeId,
              kind: "TRADE",
              attempt: 1,
              phase: "complete",
              executionMode: "SWAP",
              callBatchId: "settled-batch",
              transactionHash: `0x${"cd".repeat(32)}`,
            },
          ],
        }),
      );
      window.dispatchEvent(new StorageEvent("storage", { key }));
    },
    { wallet, recipe: mock.recipe() },
  );
  const activity = page.getByRole("region", { name: "Execution activity" });
  await expect(activity.getByText("1 / 1")).toBeVisible();
  await expect(activity.getByText("Approve USDT", { exact: true })).toHaveCount(
    1,
  );
  await expect(
    activity.getByText("Swap USDT → NVDAB", { exact: true }),
  ).toHaveCount(1);
  await page.screenshot({
    path: "/tmp/stockfront-activity-swap.png",
    fullPage: true,
  });
  await page
    .getByRole("navigation", { name: "Inspector view" })
    .getByRole("button", { name: "Configure", exact: true })
    .click();
  await expect(
    page.getByText("Quote estimate", { exact: true }),
  ).not.toBeVisible();
  await expect(page.getByText("Minimum received")).not.toBeVisible();
  await page
    .getByRole("navigation", { name: "Inspector view" })
    .getByRole("button", { name: "Activity", exact: true })
    .click();
  await expect(activity).toBeVisible();
  expect(mock.submissions()).toBe(0);
});

test("another tab holding the wallet execution lock prevents submission", async ({
  context,
  page,
}) => {
  const mock = await mockApp(context);
  await page.goto("/");
  await page.evaluate((wallet) => {
    void navigator.locks.request(
      `stockfront:execution:56:${wallet}`,
      () => new Promise<void>(() => {}),
    );
  }, wallet);
  const second = await context.newPage();
  await prepare(second);
  await second.getByRole("button", { name: "Execute recipe" }).click();
  await expect(
    second.getByText("This wallet is executing in another tab."),
  ).toBeVisible();
  expect(mock.submissions()).toBe(0);
  await page.close();
});

test("removing a pending trade from the draft preserves its recovery and settlement", async ({
  context,
  page,
}) => {
  const mock = await mockApp(context);
  await prepare(page);
  await page.getByRole("button", { name: "Execute recipe" }).click();
  await expect(page.getByText("Order: order-1")).toBeAttached();
  await page
    .getByRole("navigation", { name: "Inspector view" })
    .getByRole("button", { name: "Configure", exact: true })
    .click();
  await page.getByRole("button", { name: "Remove action" }).click();
  await page
    .getByLabel("Recipe toolbar")
    .getByRole("button", { name: "Activity", exact: true })
    .click();
  await expect(page.locator(".react-flow__node")).toHaveCount(1);
  await expect(page.getByText("Order: order-1")).toBeAttached();
  await expect(
    page.getByRole("button", { name: "Preview route" }),
  ).toBeDisabled();
  mock.fill();
  await page.getByRole("button", { name: "Check status" }).click();
  await expect(page.getByText("Recipe settled")).toBeVisible();
  await page.getByRole("button", { name: "Restore execution recipe" }).click();
  await expect(page.locator(".react-flow__node")).toHaveCount(2);
  expect(mock.submissions()).toBe(1);
});

test("a preview response for an edited draft cannot enable preparation", async ({
  context,
  page,
}) => {
  await mockApp(context);
  let release!: () => void;
  const responseGate = new Promise<void>((resolve) => {
    release = resolve;
  });
  // Fall back to the context's mock after the gate, never the live service.
  await page.route("**/test-api/v1/recipes/preview", async (route) => {
    await responseGate;
    await route.fallback();
  });
  await page.goto("/");
  const connect = page.getByRole("button", { name: "Connect wallet" });
  if (await connect.isVisible()) await connect.click();
  await expect(
    page.getByRole("button", { name: "Preview route" }),
  ).toBeEnabled();
  const requested = page.waitForRequest("**/test-api/v1/recipes/preview");
  await page.getByRole("button", { name: "Preview route" }).click();
  await requested;
  await page.getByRole("button", { name: /^Wallet asset/ }).click();
  const replied = page.waitForResponse("**/test-api/v1/recipes/preview");
  release();
  await replied;
  await page.locator(".react-flow__node").filter({ hasText: "Trade" }).click();
  await expect(
    page.getByRole("button", { name: "Prepare transactions" }),
  ).toHaveCount(0);
});

for (const width of [1440, 768, 390]) {
  test(`activity stays below the toolbar at ${width}px in ready, pending and settled states`, async ({
    context,
    page,
  }) => {
    const mock = await mockApp(context);
    await prepare(page);
    // Connect using the existing desktop wallet control, then exercise the responsive activity layout.
    await page.setViewportSize({ width, height: 920 });
    const activity = page.getByRole("region", { name: "Execution activity" });
    await expect(activity).toBeVisible();
    async function checkLayout() {
      const toolbar = await page.getByLabel("Recipe toolbar").boundingBox();
      const panel = await page
        .locator("aside")
        .filter({ has: activity })
        .boundingBox();
      expect(toolbar).not.toBeNull();
      expect(panel).not.toBeNull();
      expect(panel!.y).toBeGreaterThanOrEqual(toolbar!.y + toolbar!.height);
      expect(panel!.x + panel!.width).toBeLessThanOrEqual(width);
      expect(
        await page.evaluate(() => document.documentElement.scrollWidth),
      ).toBeLessThanOrEqual(width);
    }
    await checkLayout();
    await page.getByRole("button", { name: "Execute recipe" }).click();
    await expect(page.getByText("Order: order-1")).toBeAttached();
    await expect(
      page.getByRole("button", { name: "Execute recipe" }),
    ).toHaveCount(0);
    await checkLayout();
    mock.fill();
    await page.getByRole("button", { name: "Check status" }).click();
    await expect(page.getByText("Recipe settled")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "New run", exact: true }),
    ).toBeVisible();
    await checkLayout();
    await page.getByRole("button", { name: "Close inspector" }).click();
    await page
      .getByLabel("Recipe toolbar")
      .getByRole("button", { name: "Activity", exact: true })
      .click();
    await expect(activity).toBeVisible();
    await page.screenshot({
      path: `/tmp/stockfront-activity-${width}.png`,
      fullPage: true,
    });
  });
}

test("an uncertain submission shows recovery without offering a new execution", async ({
  context,
  page,
}) => {
  const mock = await mockApp(context);
  mock.loseSubmissionResponse();
  await prepare(page);
  await page.getByRole("button", { name: "Execute recipe" }).click();
  const activity = page.getByRole("region", { name: "Execution activity" });
  await expect(
    activity.getByText("Needs attention", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Execute recipe" }),
  ).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Refresh route" })).toHaveCount(
    0,
  );
  await expect(
    activity.getByRole("button", { name: "Retry same RFQ submission" }),
  ).toBeVisible();
  await page.screenshot({
    path: "/tmp/stockfront-activity-attention.png",
    fullPage: true,
  });
  expect(mock.submissions()).toBe(1);
});
