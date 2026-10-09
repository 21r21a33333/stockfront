import type { BstockSummary } from "@/api/generated";

/** Plain-English market session and on-chain vs reference price gap for one stock. */
export function marketNote(stock: BstockSummary): string | undefined {
  const parts = [sessionLabel(stock), gapLabel(stock)].filter(Boolean);
  return parts.length ? parts.join(" ") : undefined;
}

function sessionLabel(stock: BstockSummary): string | undefined {
  const status = stock.marketStatus;
  if (!status?.session) return undefined;
  const open = status.session === "regular";
  if (open) return "US market open.";
  const next = status.nextOpenAtMs
    ? ` · opens ${new Intl.DateTimeFormat("en-US", { weekday: "short", hour: "numeric", minute: "2-digit" }).format(status.nextOpenAtMs)}`
    : "";
  return `US market ${sessionWord(status.session)}${next}. The on-chain token still trades.`;
}

export function sessionWord(session: string): string {
  switch (session) {
    case "premarket":
      return "in pre-market";
    case "postmarket":
      return "in after-hours";
    case "overnight":
      return "in overnight session";
    case "regular":
      return "open";
    case "paused":
      return "paused";
    default:
      return "closed";
  }
}

export function priceGapPercent(stock: BstockSummary): number | undefined {
  const token = Number(stock.tokenPriceUsd);
  const reference = Number(stock.referencePriceUsd);
  if (!Number.isFinite(token) || !Number.isFinite(reference) || reference <= 0)
    return undefined;
  return ((token - reference) / reference) * 100;
}

function gapLabel(stock: BstockSummary): string | undefined {
  const gap = priceGapPercent(stock);
  if (gap === undefined) return undefined;
  return `${stock.tokenSymbol} ${usd(stock.tokenPriceUsd)} vs ${stock.underlyingTicker} ${usd(stock.referencePriceUsd)}, a ${gap >= 0 ? "+" : "−"}${Math.abs(gap).toFixed(2)}% gap.`;
}

export function usd(value?: string): string {
  const amount = Number(value);
  return Number.isFinite(amount)
    ? new Intl.NumberFormat("en-US", {
        style: "currency",
        currency: "USD",
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      }).format(amount)
    : "—";
}
