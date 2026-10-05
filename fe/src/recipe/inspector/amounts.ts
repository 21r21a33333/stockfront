import { formatUnits } from "viem";

export function formatQuote(value: string, decimals: number): string {
  return formatDisplayAmount(formatUnits(BigInt(value), decimals));
}

export function formatDisplayAmount(value: string): string {
  const [whole, fraction] = value.split(".");
  const visibleFraction = fraction?.slice(0, 6).replace(/0+$/, "");
  return visibleFraction ? `${whole}.${visibleFraction}` : (whole ?? "0");
}
