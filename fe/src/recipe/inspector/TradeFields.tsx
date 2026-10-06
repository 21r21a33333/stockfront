import { ChevronDown } from "lucide-react";
import { AssetSelect } from "@/ui/assets/AssetSelect";
import type {
  PreparedRecipe,
  RecipePreview,
  RecipeTradePolicy,
  TradeQuote,
} from "@/api/generated";
import type { AssetOption, RecipeFlowNode } from "../editor/types";
import { inputForTrade } from "../editor/flowInput";
import { useRecipeEditor } from "../editor/store";
import { compatibleTradeAssets } from "../editor/tradePolicy";
import { RoutePreview } from "./RoutePreview";
import { formatQuote } from "./amounts";
import styles from "../RecipeBuilder.module.css";

export function TradeFields({
  node,
  assets,
  tradePolicy,
  preview,
  selectedRouteId,
  prepared,
  completed,
  preparing,
  onRouteSelect,
  onPrepare,
}: {
  node: RecipeFlowNode;
  assets: AssetOption[];
  tradePolicy: RecipeTradePolicy;
  preview?: RecipePreview;
  selectedRouteId?: string;
  prepared?: PreparedRecipe;
  completed: boolean;
  preparing: boolean;
  onRouteSelect(route: TradeQuote): void;
  onPrepare(): void;
}) {
  const editor = useRecipeEditor();
  if (node.data.kind !== "TRADE") return null;
  const buyToken = node.data.buyToken;
  const candidate = preview?.nodes.find((item) => item.nodeId === node.id);
  const previewNode = candidate?.kind === "TRADE" ? candidate : undefined;
  const routes = previewNode?.routes ?? [];
  const input = inputForTrade(editor.nodes, editor.edges, node.id);
  const sellAsset = assets.find(
    (asset) => asset.address.toLowerCase() === input?.token?.toLowerCase(),
  );
  const buyAsset = assets.find(
    (asset) => asset.address.toLowerCase() === buyToken?.toLowerCase(),
  );
  const compatibleAssets = compatibleTradeAssets(
    input?.token,
    assets,
    tradePolicy,
  );
  const supported = compatibleAssets.some(
    (asset) => asset.address.toLowerCase() === buyToken?.toLowerCase(),
  );
  const selectedRoute = routes.find(
    (route) => route.routeQuoteId === selectedRouteId,
  );
  return (
    <>
      <div className={styles.inspectorSectionTitle}>
        <strong>Inputs</strong>
        <ChevronDown size={13} />
      </div>
      <p className={styles.portCaption}>Typed ports from the recipe</p>
      <section className={styles.fieldGroup}>
        <div className={styles.portBinding}>
          <span>in</span>
          <code>
            Asset&lt;{sellAsset?.symbol ?? "?"}&gt;
            {input ? ` · from ${input.sourceName}` : " · connect a source"}
          </code>
        </div>
        <div className={styles.portRow}>
          <span>Sell</span>
          <strong>
            {input?.amount ? `${input.amount} ` : ""}
            {sellAsset?.symbol ?? "Connect asset"}
          </strong>
          <small>{input ? "bound" : "unbound"}</small>
        </div>
        <div className={styles.portRow}>
          <span>Buy</span>
          <AssetSelect
            assets={compatibleAssets}
            value={buyToken}
            selectedAsset={buyAsset}
            invalid={Boolean(input && buyToken && !supported)}
            onChange={(buyToken) => editor.updateNode(node.id, { buyToken })}
          />
        </div>
        {input && buyToken && !supported ? (
          <p className={`${styles.runtimeNotice} ${styles.runtimeWarning}`}>
            {buyAsset?.symbol ?? "This asset"} cannot be bought with{" "}
            {sellAsset?.symbol ?? "the incoming asset"}. Choose a supported
            asset.
          </p>
        ) : null}
      </section>
      {previewNode?.state === "WAITING_FOR_UPSTREAM" ? (
        <p className={styles.runtimeNotice}>
          Waiting for the previous action to settle
        </p>
      ) : previewNode?.state === "OUTPUT_UNAVAILABLE" ? (
        <p className={`${styles.runtimeNotice} ${styles.runtimeWarning}`}>
          The previous action settled, but its exact output is unavailable
        </p>
      ) : previewNode?.state === "COMPLETE" ? (
        <p className={styles.runtimeNotice}>This action is complete</p>
      ) : null}
      {routes.length > 0 ? (
        <details
          open={completed ? undefined : true}
          className={styles.quoteDetails}
        >
          <summary>{completed ? "Quote estimate" : "Route preview"}</summary>
          <section className={styles.routeSection}>
            <div className={styles.inspectorSectionTitle}>
              <strong>Routes</strong>
              <span>LIVE</span>
            </div>
            {routes.map((route) => {
              const asset = assets.find(
                (item) =>
                  item.address.toLowerCase() === route.buy.token.toLowerCase(),
              );
              return (
                <button
                  type="button"
                  key={`${route.venue}-${route.routeQuoteId}`}
                  className={`${styles.route} ${selectedRouteId === route.routeQuoteId ? styles.selectedRoute : ""}`}
                  onClick={() => onRouteSelect(route)}
                >
                  <span>
                    <strong>{route.venue}</strong>
                    <small>{route.executionMode}</small>
                  </span>
                  <span>
                    <strong>
                      {formatQuote(route.buy.amount, asset?.decimals ?? 18)}
                    </strong>
                    <small>{asset?.symbol ?? "tokens"}</small>
                  </span>
                </button>
              );
            })}
            {selectedRoute ? (
              <RoutePreview
                route={selectedRoute}
                assets={assets}
                prepared={prepared}
                nodeId={node.id}
              />
            ) : null}
            {!prepared && !completed ? (
              <button
                type="button"
                className={styles.prepareButton}
                onClick={onPrepare}
                disabled={!selectedRouteId || preparing}
              >
                {preparing ? "Preparing…" : "Prepare transactions"}
              </button>
            ) : null}
          </section>
        </details>
      ) : null}
    </>
  );
}
