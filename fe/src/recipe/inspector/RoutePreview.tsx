import { formatUnits } from "viem";
import type { PreparedRecipe, TradeQuote } from "@/api/generated";
import type { AssetOption } from "../editor/types";
import { formatQuote, formatDisplayAmount } from "./amounts";
import styles from "../RecipeBuilder.module.css";

export function RoutePreview({
  route,
  assets,
  prepared,
  nodeId,
}: {
  route: TradeQuote;
  assets: AssetOption[];
  prepared?: PreparedRecipe;
  nodeId: string;
}) {
  const sell = assets.find(
    (asset) => asset.address.toLowerCase() === route.sell.token.toLowerCase(),
  );
  const buy = assets.find(
    (asset) => asset.address.toLowerCase() === route.buy.token.toLowerCase(),
  );
  const step = prepared?.steps.find(
    (item) => item.nodeId === nodeId && item.kind === "TRADE",
  );
  const minimum =
    step?.kind === "TRADE" && step.action.executionMode === "SWAP"
      ? step.action.transaction.minReceiveAmount
      : undefined;
  const networkFee = prepared?.steps.reduce((total, item) => {
    const action =
      item.kind === "APPROVAL"
        ? item.action
        : item.action.executionMode === "SWAP"
          ? item.action.transaction
          : undefined;
    return (
      total + BigInt(action?.gasLimit ?? 0) * BigInt(action?.gasPrice ?? 0)
    );
  }, 0n);
  return (
    <div className={styles.routePreview}>
      <div className={styles.routePreviewHeading}>
        <strong>Route preview</strong>
        <span>LIVE</span>
      </div>
      <p>{minimum ? "Minimum received" : "Expected received"}</p>
      <div className={styles.routePreviewAmount}>
        {formatQuote(minimum ?? route.buy.amount, buy?.decimals ?? 18)}{" "}
        <span>{buy?.symbol ?? "tokens"}</span>
      </div>
      <div className={styles.routePath}>
        <span>{sell?.symbol ?? "Asset"}</span>
        <b>→</b>
        <span>{route.venue}</span>
        <b>→</b>
        <span>{buy?.symbol ?? "Asset"}</span>
      </div>
      <dl>
        <div>
          <dt>Expected</dt>
          <dd>
            ≈ {formatQuote(route.buy.amount, buy?.decimals ?? 18)} {buy?.symbol}
          </dd>
        </div>
        {route.priceImpactPercent ? (
          <div>
            <dt>Price impact</dt>
            <dd>{route.priceImpactPercent}%</dd>
          </div>
        ) : null}
        {networkFee !== undefined && networkFee > 0n ? (
          <div>
            <dt>Estimated network fee</dt>
            <dd>≈ {formatDisplayAmount(formatUnits(networkFee, 18))} BNB</dd>
          </div>
        ) : null}
      </dl>
    </div>
  );
}
