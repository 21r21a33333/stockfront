import { formatUnits } from "viem";

export function formatQuote(value: string, decimals: number): string {
  return formatDisplayAmount(formatUnits(BigInt(value), decimals));
}

export function formatDisplayAmount(value: string): string {
  const [whole, fraction] = value.split(".");
  const visibleFraction = fraction?.slice(0, 6).replace(/0+$/, "");
  return visibleFraction ? `${whole}.${visibleFraction}` : (whole ?? "0");
}

export function formatPositionAmount(value: string | undefined): string {
  if (!value) return "—";
  const amount = Number(value);
  if (!Number.isFinite(amount)) return "—";
  return new Intl.NumberFormat("en-US", {
    maximumSignificantDigits: 4,
  }).format(amount);
}

export function formatUsd(value: string | undefined): string {
  if (!value) return "—";
  const amount = Number(value);
  if (!Number.isFinite(amount)) return "—";
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(amount);
}
