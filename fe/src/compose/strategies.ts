import type { BasketSummary, CarryOpportunity } from "@/api/generated";
import type { AssetOption } from "@/recipe/editor/types";
import type { BlockKind } from "@/ui/blocks";

/**
 * How much can go wrong, relative to the other curated strategies. The tier is
 * a comparison between these three, never a claim that a strategy is safe, so
 * every card also names the specific thing that can cost the user money.
 */
export type RiskTier = "lower" | "high";

export interface CuratedStrategy {
  key: string;
  name: string;
  summary: string;
  chain: BlockKind[];
  tier: RiskTier;
  riskLabel: string;
  risk: string;
  prompt: string;
  /** The one figure the card leads with, and where it came from. */
  headline: { value: string; label: string };
}

/** One Venus market a curated strategy can be written against. */
export interface StrategyMarket {
  stock: AssetOption;
  stable: AssetOption;
  /** The share of the collateral's value Venus lends against. */
  collateralFactor: string;
}

export interface StrategyInputs {
  earn?: StrategyMarket;
  leverage?: StrategyMarket;
  basket?: BasketSummary;
  /** Live earn and borrow rates for the `earn` market. */
  carry?: CarryOpportunity;
}

/**
 * Reserve's own published 30-day return for the index, read from its discover
 * page on 2026-10-11. It is fixed rather than fetched, so the card never waits
 * on Reserve and never falls back to a blank figure; re-check it against
 * reserve.org when the number starts to look old.
 */
const BASKET_RETURN = { percent: 6.99, period: "30 days" };

/**
 * The household names a curated card prefers to be written against, best
 * first. Anything Venus lists but this does not name still works; it just
 * ranks behind these.
 */
const POPULAR = [
  "NVDAB",
  "TSLAB",
  "AAPLB",
  "MSFTB",
  "METAB",
  "GOOGLB",
  "AMZNB",
  "AMDB",
];

/** Where a collateral symbol sits in `POPULAR`; unknown symbols rank last. */
export function popularity(symbol: string): number {
  const index = POPULAR.indexOf(symbol.toUpperCase());
  return index === -1 ? POPULAR.length : index;
}

/**
 * The curated strategies, in the order they are offered: income first,
 * leverage second, and diversified exposure last. A card is only built once
 * the assets its prompt names are known, so a card never offers a chain the
 * catalog cannot express.
 */
export function curatedStrategies({
  earn,
  leverage,
  basket,
  carry,
}: StrategyInputs): CuratedStrategy[] {
  const strategies: CuratedStrategy[] = [];
  if (earn) strategies.push(borrowAndEarn(earn, carry));
  if (leverage) strategies.push(leveragedStock(leverage));
  if (basket) strategies.push(indexBasket(basket));
  return strategies;
}

function borrowAndEarn(
  market: StrategyMarket,
  carry?: CarryOpportunity,
): CuratedStrategy {
  const { stock, stable } = market;
  return {
    key: "borrow-and-earn",
    name: "Borrow & earn",
    summary: `Buy ${stock.symbol}, post it as collateral and borrow ${stable.symbol} against it, then put that ${stable.symbol} into a reviewed earn vault. You keep the stock and collect the spread on top.`,
    chain: ["wallet", "trade", "supply", "borrow", "earn"],
    tier: "lower",
    riskLabel: "Lower risk",
    risk: `Venus can liquidate the collateral if ${stock.symbol} falls far enough.`,
    prompt: `Buy $6 of ${stock.symbol}, supply it to Venus as collateral, then borrow 1 ${stable.symbol} against it and deposit that into earn`,
    headline: {
      value: percent(carry?.baseSpreadPercent),
      label: "Net APY, after borrow cost",
    },
  };
}

function leveragedStock(market: StrategyMarket): CuratedStrategy {
  const { stock, stable } = market;
  return {
    key: "leveraged-stock",
    name: "Leveraged stock",
    summary: `Borrow against the ${stock.symbol} you just supplied, buy more ${stock.symbol} with it and supply that too. Each turn buys more exposure than your own money alone, and magnifies the loss just as much.`,
    chain: ["wallet", "trade", "supply", "borrow", "trade", "supply"],
    tier: "high",
    riskLabel: "High risk",
    risk: `A fall in ${stock.symbol} is amplified by every turn, and liquidates the whole position rather than one leg.`,
    prompt: `Buy $6 of ${stock.symbol}, supply it to Venus as collateral, borrow 1 ${stable.symbol} against it, buy more ${stock.symbol} with that and supply it as well`,
    headline: {
      value: exposure(market.collateralFactor),
      // A leveraged stock has no yield of its own: what one run actually buys
      // is this much more exposure than the cash put in.
      label: `${stock.symbol} exposure per run`,
    },
  };
}

function indexBasket(basket: BasketSummary): CuratedStrategy {
  return {
    key: "index-basket",
    name: basket.name,
    summary:
      basket.description?.trim() ||
      "One token that holds the whole index, backed by the assets themselves and rebalanced onchain. Broad exposure without picking winners or managing weights yourself.",
    chain: ["wallet", "trade"],
    tier: "lower",
    riskLabel: "No leverage",
    risk: "Market risk only — the basket can fall in value, but there is no debt and nothing to liquidate.",
    prompt: `Invest 5 USDT in ${basket.name}`,
    headline: {
      value: signedPercent(BASKET_RETURN.percent),
      label: `Past ${BASKET_RETURN.period}, per Reserve`,
    },
  };
}

/** One turn lends `collateralFactor` of the stock back, so exposure is 1 + it. */
function exposure(collateralFactor: string): string {
  const factor = Number(collateralFactor);
  return Number.isFinite(factor) ? `${(1 + factor).toFixed(2)}×` : "—";
}

/** Rates arrive as long decimal strings; two places is all a card can show. */
function percent(value?: string | null): string {
  const numeric = Number(value);
  if (value === undefined || value === null || !Number.isFinite(numeric))
    return "—";
  return `${numeric.toFixed(2)}%`;
}

function signedPercent(value: number): string {
  return `${value >= 0 ? "+" : "−"}${Math.abs(value).toFixed(2)}%`;
}
