import { expect, test } from "@playwright/test";

test("without a browser wallet the modal offers wallets to install", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Connect wallet" }).first().click();

  const dialog = page.getByRole("dialog", { name: "Connect a wallet" });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole("link", { name: /MetaMask/ })).toHaveAttribute(
    "href",
    "https://metamask.io/download/",
  );

  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
});

test("an announced wallet is listed by name and a rejected request can be retried", async ({
  page,
}) => {
  // A minimal EIP-6963 wallet that refuses to connect.
  await page.addInitScript(() => {
    const provider = {
      request: async ({ method }: { method: string }) => {
        if (method === "eth_chainId") return "0x38";
        if (method === "eth_accounts") return [];
        throw Object.assign(new Error("User rejected the request."), {
          code: 4001,
        });
      },
      on: () => undefined,
      removeListener: () => undefined,
    };
    const announce = () =>
      window.dispatchEvent(
        new CustomEvent("eip6963:announceProvider", {
          detail: Object.freeze({
            info: {
              uuid: "6d5b1f62-2d4e-4b39-9a3c-1f0a5b9c7e11",
              name: "Test Wallet",
              icon: "data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg'/>",
              rdns: "test.wallet",
            },
            provider,
          }),
        }),
      );
    window.addEventListener("eip6963:requestProvider", announce);
    announce();
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Connect wallet" }).first().click();

  const dialog = page.getByRole("dialog", { name: "Connect a wallet" });
  const wallet = dialog.getByRole("button", { name: /Test Wallet/ });
  await expect(wallet).toBeVisible();
  await expect(dialog.getByRole("link", { name: /MetaMask/ })).toHaveCount(0);

  await wallet.click();
  await expect(wallet).toContainText("Request cancelled in your wallet.");
  await expect(wallet).toContainText("Retry");
});
