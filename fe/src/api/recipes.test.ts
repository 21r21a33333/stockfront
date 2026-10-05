import { recipesApi } from "./recipes";
import type { RecipeDefinition } from "@/api/generated";

const recipe: RecipeDefinition = {
  schemaVersion: 1,
  chainId: 56,
  nodes: [],
  edges: [],
};

beforeEach(() => {
  vi.restoreAllMocks();
});

it("passes the exact recipe definition to the validation endpoint", async () => {
  const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
    new Response(
      JSON.stringify({
        status: "Ok",
        result: { valid: false, diagnostics: [] },
      }),
      { status: 200, headers: { "content-type": "application/json" } },
    ),
  );

  await recipesApi.validate(recipe);

  const request = fetchMock.mock.calls[0][0] as Request;
  expect(new URL(request.url).pathname).toBe("/api/v1/recipes/validate");
  expect(request.method).toBe("POST");
  expect(await request.json()).toEqual(recipe);
});

it("preserves compiler diagnostics from an API error", async () => {
  vi.spyOn(globalThis, "fetch").mockResolvedValue(
    new Response(
      JSON.stringify({
        status: "Error",
        error: {
          code: "invalid_recipe",
          message: "the recipe is invalid",
          diagnostics: [
            {
              code: "EMPTY_RECIPE",
              severity: "ERROR",
              location: { kind: "RECIPE" },
              message: "Add at least one action.",
            },
          ],
        },
      }),
      { status: 422, headers: { "content-type": "application/json" } },
    ),
  );

  await expect(
    recipesApi.preview({ recipe, context: { wallet: usdt } }),
  ).rejects.toMatchObject({
    code: "invalid_recipe",
    diagnostics: [expect.objectContaining({ code: "EMPTY_RECIPE" })],
  });
});

const usdt = "0x55d398326f99059fF775485246999027B3197955" as const;

it("rejects an error envelope even when HTTP succeeds", async () => {
  vi.spyOn(globalThis, "fetch").mockResolvedValue(
    new Response(
      JSON.stringify({
        status: "Error",
        error: { code: "trade_unavailable", message: "Try another route." },
      }),
      { status: 200, headers: { "content-type": "application/json" } },
    ),
  );
  await expect(recipesApi.catalog()).rejects.toMatchObject({
    code: "trade_unavailable",
    message: "Try another route.",
  });
});

it("reports an HTTP failure when the server returns plain text", async () => {
  vi.spyOn(globalThis, "fetch").mockResolvedValue(
    new Response("upstream unavailable", { status: 502 }),
  );
  await expect(recipesApi.catalog()).rejects.toMatchObject({
    code: "request_failed",
    message: "Stockfront request failed with status 502.",
  });
});

it("reports malformed JSON in a successful response", async () => {
  vi.spyOn(globalThis, "fetch").mockResolvedValue(
    new Response("not json", { status: 200 }),
  );
  await expect(recipesApi.catalog()).rejects.toMatchObject({
    code: "invalid_response",
  });
});
