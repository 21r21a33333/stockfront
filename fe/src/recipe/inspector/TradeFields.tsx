import { Check, ChevronDown } from "lucide-react";
import { AssetSelect } from "@/ui/assets/AssetSelect";
import type {
  PreparedRecipe,
  RecipePreview,
  TradeQuote,
} from "@/api/generated";
import type { ExecutionProgress } from "@/wallet/execution";
import type { AssetOption, RecipeFlowNode } from "../editor/types";
import { inputForTrade } from "../editor/flowInput";
import { useRecipeEditor } from "../editor/store";
import { shortAddress } from "../assets";
import { RoutePreview } from "./RoutePreview";
import { formatQuote } from "./amounts";
import styles from "../RecipeBuilder.module.css";

export function TradeFields({
  node,
  assets,
  preview,
  selectedRouteId,
  prepared,
  execution,
  completed,
  preparing,
  executing,
  onRouteSelect,
  onPrepare,
  onExecute,
}: {
  node: RecipeFlowNode;
  assets: AssetOption[];
  preview?: RecipePreview;
  selectedRouteId?: string;
  prepared?: PreparedRecipe;
  execution: ExecutionProgress[];
  completed: boolean;
  preparing: boolean;
  executing: boolean;
  onRouteSelect(route: TradeQuote): void;
  onPrepare(): void;
  onExecute(): void;
}) {
  const editor = useRecipeEditor();
  if (node.data.kind !== "TRADE") return null;
  const routes =
    preview?.nodes.find((item) => item.nodeId === node.id)?.routes ?? [];
  const input = inputForTrade(editor.nodes, editor.edges, node.id);
  const sellAsset = assets.find(
    (asset) => asset.address.toLowerCase() === input?.token?.toLowerCase(),
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
            assets={assets}
            value={node.data.buyToken}
            onChange={(buyToken) => editor.updateNode(node.id, { buyToken })}
          />
        </div>
      </section>
      {routes.length > 0 ? (
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
          {!prepared ? (
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
      ) : null}
      {prepared ? (
        <section className={styles.transactionSection}>
          <div className={styles.inspectorSectionTitle}>
            <strong>Transactions</strong>
            <span>
              {execution.filter((item) => item.state === "complete").length} /{" "}
              {prepared.steps.length}
            </span>
          </div>
          <p className={styles.portCaption}>POST /v1/recipes/prepare</p>
          {prepared.steps.map((step, index) => {
            const state = execution.find((item) => item.index === index)?.state;
            const action =
              step.kind === "APPROVAL"
                ? step.action
                : step.action.executionMode === "SWAP"
                  ? step.action.transaction
                  : undefined;
            const approvalAsset =
              step.kind === "APPROVAL"
                ? assets.find(
                    (asset) =>
                      asset.address.toLowerCase() ===
                      step.action.to.toLowerCase(),
                  )
                : undefined;
            const tradeNode = editor.nodes.find(
              (item) => item.id === step.nodeId,
            );
            const buyToken =
              tradeNode?.data.kind === "TRADE"
                ? tradeNode.data.buyToken
                : undefined;
            const buyAsset = assets.find(
              (asset) =>
                asset.address.toLowerCase() === buyToken?.toLowerCase(),
            );
            return (
              <div
                className={styles.transaction}
                key={`${step.kind}-${step.nodeId}-${index}`}
              >
                <div className={styles.transactionTop}>
                  <span
                    className={state === "complete" ? styles.txComplete : ""}
                  >
                    {state === "complete" ? <Check size={11} /> : index + 1}
                  </span>
                  <strong>
                    {step.kind === "APPROVAL"
                      ? `Approve ${approvalAsset?.symbol ?? "asset"}`
                      : `${step.action.executionMode === "RFQ" ? "Submit RFQ" : "Swap"} → ${buyAsset?.symbol ?? "asset"}`}
                  </strong>
                  <small>{state ?? "queued"}</small>
                </div>
                <div className={styles.transactionMeta}>
                  <span>{action ? shortAddress(action.to) : "RFQ order"}</span>
                  <span>
                    {action?.gasLimit
                      ? `${Number(action.gasLimit).toLocaleString("en-US")} gas`
                      : "Wallet signature"}
                  </span>
                </div>
              </div>
            );
          })}
          {completed ? (
            <p className={styles.recipeComplete} role="status">
              <Check size={14} /> Recipe complete
            </p>
          ) : (
            <button
              type="button"
              className={styles.executeButton}
              onClick={onExecute}
              disabled={executing}
            >
              {executing ? "Confirm in wallet…" : "Execute recipe"}
            </button>
          )}
        </section>
      ) : null}
    </>
  );
}
