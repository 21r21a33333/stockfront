export type HealthZone = "At risk" | "Caution" | "Safe";

export function healthZone(value: number): HealthZone {
  if (value < 1.5) return "At risk";
  if (value < 2) return "Caution";
  return "Safe";
}

export const zoneColors: Record<HealthZone, string> = {
  "At risk": "var(--zone-risk)",
  Caution: "var(--zone-caution)",
  Safe: "var(--zone-safe)",
};

export function formatHealth(value?: string | null): string {
  if (!value) return "No debt";
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric.toFixed(2) : "—";
}
