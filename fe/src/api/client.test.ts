import { apiResult, StockfrontApiError } from "./client";

it.each(["12", new Date(Date.now() + 12_000).toUTCString()])(
  "preserves a provider retry delay for %s",
  async (delay) => {
    const error = await apiResult(
      Promise.resolve({
        response: new Response(null, {
          status: 429,
          headers: { "Retry-After": delay },
        }),
      }),
    ).catch((cause: unknown) => cause);
    expect(error).toBeInstanceOf(StockfrontApiError);
    expect((error as StockfrontApiError).retryAfterMs).toBeGreaterThan(10_000);
  },
);
