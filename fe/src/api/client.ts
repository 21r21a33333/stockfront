import createClient from "openapi-fetch";
import type {
  paths,
  RecipeDiagnostic,
  Response_RecipeCatalog,
} from "./generated";

export const api = createClient<paths>({
  baseUrl: new URL(
    import.meta.env.VITE_API_BASE_URL ?? "/api",
    window.location.origin,
  ).href,
  fetch: (request) => globalThis.fetch(request),
  headers: { accept: "application/json" },
});

export class StockfrontApiError extends Error {
  constructor(
    readonly code: string,
    message: string,
    readonly diagnostics: RecipeDiagnostic[] = [],
    readonly retryAfterMs?: number,
  ) {
    super(message);
    this.name = "StockfrontApiError";
  }
}

type Envelope<T> = Omit<Response_RecipeCatalog, "result"> & { result?: T };

export async function apiResult<T>(
  pending: Promise<{
    data?: Envelope<T>;
    error?: Pick<Response_RecipeCatalog, "status" | "error">;
    response: Response;
  }>,
): Promise<T> {
  try {
    const { data, error, response } = await pending;
    const failure = error?.error ?? data?.error;
    const delay = response.headers.get("Retry-After");
    const milliseconds =
      delay === null
        ? NaN
        : /^\d+$/.test(delay)
          ? Number(delay) * 1000
          : Date.parse(delay) - Date.now();
    const retryAfterMs = Number.isFinite(milliseconds)
      ? Math.max(0, milliseconds)
      : undefined;
    if (failure) {
      throw new StockfrontApiError(
        failure.code,
        failure.message,
        failure.diagnostics,
        retryAfterMs,
      );
    }
    if (!response.ok) {
      throw new StockfrontApiError(
        "request_failed",
        `Stockfront request failed with status ${response.status}.`,
        [],
        retryAfterMs,
      );
    }
    if (data?.status !== "Ok" || data.result === undefined) {
      throw new StockfrontApiError(
        "invalid_response",
        "Stockfront returned an unreadable response.",
      );
    }
    return data.result;
  } catch (error) {
    if (error instanceof SyntaxError) {
      throw new StockfrontApiError(
        "invalid_response",
        "Stockfront returned an unreadable response.",
      );
    }
    throw error;
  }
}
