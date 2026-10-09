import { formatUnits, parseUnits, type Address } from "viem";
import type {
  RecipeDiagnostic,
  RecipeNodePreview,
  TradeQuote,
} from "@/api/generated";
import { diagnosticsForNode } from "@/recipe/editor/compiler";
import type { BorrowedPositionOption } from "@/recipe/editor/repaymentPolicy";
import type { AssetOption, RecipeFlowNode } from "@/recipe/editor/types";
import type { SuppliedPositionOption } from "@/recipe/editor/withdrawalPolicy";
import { formatDisplayAmount, formatPositionAmount } from "@/recipe/format";
import { formatHealth } from "@/ui/health";

export interface DescribeContext {
  ordered: RecipeFlowNode[];
  assets: AssetOption[];
  suppliedPositions: SuppliedPositionOption[];
  borrowedPositions: BorrowedPositionOption[];
  previews: RecipeNodePreview[];
  routes: Record<string, TradeQuote>;
  diagnostics: RecipeDiagnostic[];
  walletReady: boolean;
  minimumHealthFactor: string;
}

export interface Check {
  ok: boolean;
  text: string;
}

export interface BlockDescription {
  name: string;
  detail: string;
  /** What this block hands to the next one. */
  output?: string;
  checks: Check[];
  bad: boolean;
}

export function assetFor(
  assets: AssetOption[],
  token?: string,
): AssetOption | undefined {
  return assets.find(
    (asset) => asset.address.toLowerCase() === token?.toLowerCase(),
  );
}

/** The token flowing into a block, following the chain back to its source. */
export function inputToken(
  ctx: DescribeContext,
  index: number,
): Address | undefined {
  const previous = ctx.ordered[index - 1]?.data;
  switch (previous?.kind) {
    case "WALLET_ASSET":
      return previous.token;
    case "TRADE":
      return previous.buyToken;
    case "BORROW":
      return previous.debtToken;
    case "SUPPLIED_POSITION":
    case "BORROWED_POSITION":
      return previous.token;
    default:
      return undefined;
  }
}

export function describe(
  ctx: DescribeContext,
  index: number,
): BlockDescription {
  const node = ctx.ordered[index]!;
  const data = node.data;
  const preview = ctx.previews.find((item) => item.nodeId === node.id);
  const checks: Check[] = diagnosticsForNode(ctx.diagnostics, node.id).map(
    (item) => ({
      ok: false,
      text: item.message,
    }),
  );
  const symbol = (token?: string) =>
    assetFor(ctx.assets, token)?.symbol ?? "token";
  const waiting = preview?.state === "WAITING_FOR_UPSTREAM";
  const live = (text: string) =>
    ctx.walletReady ? text : "Connect a wallet for live numbers";
  let description: Omit<BlockDescription, "checks" | "bad">;

  switch (data.kind) {
    case "WALLET_ASSET": {
      const asset = assetFor(ctx.assets, data.token);
      const balance =
        asset?.balanceRaw !== undefined
          ? formatUnits(asset.balanceRaw, asset.decimals)
          : undefined;
      description = {
        name: `Wallet ${asset?.symbol ?? "asset"}`,
        detail: `${data.amount || "0"} ${asset?.symbol ?? ""}${balance ? ` of ${formatDisplayAmount(balance)}` : ""}`,
        output: asset?.symbol,
      };
      if (asset && balance !== undefined) {
        const enough =
          safeUnits(data.amount, asset.decimals) <= (asset.balanceRaw ?? 0n);
        checks.push({
          ok: enough,
          text: enough
            ? `Enough ${asset.symbol} in your wallet`
            : `Not enough ${asset.symbol} in your wallet`,
        });
      }
      break;
    }
    case "TRADE": {
      const buy = assetFor(ctx.assets, data.buyToken);
      const route = ctx.routes[node.id];
      const selling = buy?.kind === "settlement";
      description = {
        name: selling
          ? `Sell for ${buy?.symbol ?? "token"}`
          : `Buy ${buy?.symbol ?? "stock"}`,
        detail: route
          ? `~${formatDisplayAmount(formatUnits(BigInt(route.buy.amount), buy?.decimals ?? 18))} ${buy?.symbol} via ${route.venue}`
          : waiting
            ? "Quoted after the previous step"
            : live("Finding the best route…"),
        output: buy?.symbol,
      };
      if (route)
        checks.push({
          ok: true,
          text: `Route found${route.priceImpactPercent ? ` · impact ${formatPositionAmount(route.priceImpactPercent)}%` : ""}`,
        });
      else if (preview?.kind === "TRADE" && preview.state === "READY")
        checks.push({
          ok: false,
          text: "No route within the price-impact limit",
        });
      break;
    }
    case "SUPPLY": {
      const token = inputToken(ctx, index);
      description = {
        name: "Supply to Venus",
        detail: `${symbol(token)} → collateral`,
        output: `Supplied ${symbol(token)}`,
      };
      if (preview?.kind === "SUPPLY" && preview.market) {
        for (const reason of preview.market.unavailableReasons)
          checks.push({ ok: false, text: supplyReason(reason) });
        if (preview.market.unavailableReasons.length === 0)
          checks.push({
            ok: true,
            text: `Venus accepts ${preview.market.token.symbol}`,
          });
      }
      break;
    }
    case "SUPPLIED_POSITION": {
      const position = ctx.suppliedPositions.find(
        (item) =>
          item.pool.toLowerCase() === data.pool?.toLowerCase() &&
          item.asset.token.contract.toLowerCase() === data.token?.toLowerCase(),
      );
      description = {
        name: `Supplied ${symbol(data.token)}`,
        detail: position
          ? `${formatPositionAmount(position.asset.amount)} on Venus`
          : live("Choose a supplied position"),
        output: `Supplied ${symbol(data.token)}`,
      };
      if (ctx.walletReady && !position)
        checks.push({
          ok: false,
          text: "This wallet has no such Venus position",
        });
      break;
    }
    case "BORROWED_POSITION": {
      const position = ctx.borrowedPositions.find(
        (item) =>
          item.pool.toLowerCase() === data.pool?.toLowerCase() &&
          item.asset.token.contract.toLowerCase() === data.token?.toLowerCase(),
      );
      description = {
        name: `${symbol(data.token)} debt`,
        detail: position
          ? `${formatPositionAmount(position.asset.amount)} on Venus`
          : live("Choose a Venus debt"),
        output: `${symbol(data.token)} debt`,
      };
      if (ctx.walletReady && !position)
        checks.push({ ok: false, text: "This wallet has no such Venus debt" });
      break;
    }
    case "BORROW": {
      const capability =
        preview?.kind === "BORROW" ? preview.capability : undefined;
      description = {
        name: `Borrow ${symbol(data.debtToken)}`,
        detail: `${data.amount || "0"} ${symbol(data.debtToken)}${capability ? ` · health ${formatHealth(capability.healthFactorAfter)}` : ""}`,
        output: symbol(data.debtToken),
      };
      if (capability) {
        for (const reason of capability.unavailableReasons)
          checks.push({
            ok: false,
            text: borrowReason(reason, ctx.minimumHealthFactor),
          });
        if (capability.unavailableReasons.length === 0)
          checks.push(
            {
              ok: true,
              text: `Health stays at or above ${ctx.minimumHealthFactor}`,
            },
            {
              ok: true,
              text: `Enough ${capability.debt.symbol} available to borrow`,
            },
          );
      }
      break;
    }
    case "REPAY": {
      const capability =
        preview?.kind === "REPAY" ? preview.capability : undefined;
      description = {
        name: "Repay",
        detail: capability
          ? `${formatPositionAmount(capability.debtAmount)} ${capability.debt.symbol} in full`
          : "Full debt",
      };
      if (capability) {
        for (const reason of capability.unavailableReasons)
          checks.push({ ok: false, text: repayReason(reason) });
        if (capability.unavailableReasons.length === 0)
          checks.push({ ok: true, text: "Your wallet covers the full debt" });
      }
      break;
    }
    case "WITHDRAW": {
      const token = inputToken(ctx, index);
      description = {
        name: "Withdraw",
        detail: `All supplied ${symbol(token)}`,
      };
      if (preview?.kind === "WITHDRAW") {
        if (preview.unavailableReason)
          checks.push({
            ok: false,
            text: withdrawReason(preview.unavailableReason),
          });
        else if (preview.state === "READY")
          checks.push({ ok: true, text: "No debt blocks this withdrawal" });
      }
      break;
    }
  }
  if (preview?.state === "UNAVAILABLE" && !checks.some((item) => !item.ok))
    checks.push({ ok: false, text: "Unavailable right now" });
  return { ...description, checks, bad: checks.some((item) => !item.ok) };
}

function safeUnits(value: string, decimals: number): bigint {
  try {
    return parseUnits(value.trim() || "0", decimals);
  } catch {
    return 0n;
  }
}

function supplyReason(reason: string): string {
  if (reason === "SUPPLY_PAUSED")
    return "Venus has paused supply for this stock";
  if (reason === "SUPPLY_CAP_REACHED") return "This Venus market is full";
  return "This stock isn't listed on Venus";
}

function borrowReason(reason: string, minimum: string): string {
  switch (reason) {
    case "POSITION_NOT_FOUND":
      return "The supplied position is gone";
    case "UNSUPPORTED_POOL":
      return "This Venus pool isn't supported";
    case "MARKET_UNLISTED":
      return "An asset isn't listed in this Venus market";
    case "COLLATERAL_UNAVAILABLE":
      return "This stock can't be used as collateral right now";
    case "BORROW_PAUSED":
      return "Venus has paused borrowing";
    case "BORROW_CAP_REACHED":
      return "The Venus borrow cap is reached";
    case "INSUFFICIENT_BORROW_LIQUIDITY":
      return "Not enough stablecoin to borrow";
    case "INVALID_ORACLE_PRICE":
      return "A Venus price is unavailable";
    case "HEALTH_FACTOR_UNAVAILABLE":
      return "Loan health can't be calculated";
    default:
      return `Health would fall below ${minimum}`;
  }
}

function repayReason(reason: string): string {
  switch (reason) {
    case "NO_DEBT":
      return "There's no debt left to repay";
    case "INSUFFICIENT_WALLET_BALANCE":
      return "Not enough stablecoin in your wallet for the debt plus a small interest buffer";
    case "UNSUPPORTED_DEBT_TOKEN":
      return "This debt token isn't supported";
    case "UNSUPPORTED_POOL":
      return "This Venus pool isn't supported";
    default:
      return "This debt no longer exists";
  }
}

function withdrawReason(reason: string): string {
  if (reason === "OUTSTANDING_DEBT")
    return "Repay your debt before withdrawing";
  if (reason === "UNSUPPORTED_POOL") return "This Venus pool isn't supported";
  return "This supplied position is gone";
}
