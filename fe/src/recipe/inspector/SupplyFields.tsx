import { ChevronDown, Landmark } from "lucide-react";
import type {
  PreparedRecipe,
  RecipePreview,
  RecipeSupplyPolicy,
  SupplyMarketUnavailableReason,
} from "@/api/generated";
import type { AssetOption, RecipeFlowNode } from "../editor/types";
import { inputForSupply } from "../editor/flowInput";
import { canSupplyAsset } from "../editor/supplyPolicy";
import { useRecipeEditor } from "../editor/store";
import styles from "../RecipeBuilder.module.css";

export function SupplyFields({
  node,
  assets,
  supplyPolicy,
  preview,
  prepared,
  completed,
  preparing,
  onPrepare,
}: {
  node: RecipeFlowNode;
  assets: AssetOption[];
  supplyPolicy: RecipeSupplyPolicy;
  preview?: RecipePreview;
  prepared?: PreparedRecipe;
  completed: boolean;
  preparing: boolean;
  onPrepare(): void;
}) {
  const editor = useRecipeEditor();
  if (node.data.kind !== "SUPPLY") return null;
  const input = inputForSupply(editor.nodes, editor.edges, node.id);
  const asset = assets.find(
    (item) => item.address.toLowerCase() === input?.token?.toLowerCase(),
  );
  const candidate = preview?.nodes.find((item) => item.nodeId === node.id);
  const previewNode = candidate?.kind === "SUPPLY" ? candidate : undefined;
  const market = previewNode?.market;
  const supported = canSupplyAsset(asset, supplyPolicy, node.data.venue);
  const venues = [
    ...new Set(supplyPolicy.allowedInputs.map((rule) => rule.venue)),
  ];
  const preparedSupply = prepared?.steps.some(
    (step) => step.kind === "SUPPLY" && step.nodeId === node.id,
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
            Asset&lt;{asset?.symbol ?? "?"}&gt;
            {input ? ` · from ${input.sourceName}` : " · connect a source"}
          </code>
        </div>
        <div className={styles.portRow}>
          <span>Supply</span>
          <strong>
            {input?.amount ? `${input.amount} ` : ""}
            {asset?.symbol ?? "Connect asset"}
          </strong>
          <small>{input ? "bound" : "unbound"}</small>
        </div>
        <label htmlFor={`supply-venue-${node.id}`}>Lending market</label>
        <div className={styles.venueSelect}>
          <Landmark size={15} />
          <select
            id={`supply-venue-${node.id}`}
            value={node.data.venue}
            onChange={(event) =>
              editor.updateNode(node.id, {
                venue: event.target.value as "venus",
              })
            }
          >
            {venues.map((venue) => (
              <option value={venue} key={venue}>
                Venus Core Pool
              </option>
            ))}
          </select>
        </div>
      </section>
      {input && !supported ? (
        <p className={`${styles.runtimeNotice} ${styles.runtimeWarning}`}>
          Venus Supply accepts supported bStocks. Connect a bStock output to
          this action.
        </p>
      ) : null}

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

      {market ? (
        <section className={styles.marketPreview}>
          <div className={styles.routePreviewHeading}>
            <strong>Venus market</strong>
            <span>{market.supplyAvailable ? "AVAILABLE" : "UNAVAILABLE"}</span>
          </div>
          <p>Current base supply APY</p>
          <div className={styles.marketApy}>{market.supplyApyPercent}%</div>
          <dl>
            <div>
              <dt>Asset</dt>
              <dd>{market.token.symbol}</dd>
            </div>
            <div>
              <dt>Remaining capacity</dt>
              <dd>
                {market.remainingSupplyCapacity == null
                  ? "Unlimited"
                  : `${market.remainingSupplyCapacity} ${market.token.symbol}`}
              </dd>
            </div>
            <div>
              <dt>Market</dt>
              <dd>{shortAddress(market.market)}</dd>
            </div>
          </dl>
          {market.unavailableReasons.map((reason) => (
            <p
              className={`${styles.runtimeNotice} ${styles.runtimeWarning}`}
              key={reason}
            >
              {reasonLabel(reason)}
            </p>
          ))}
          {!preparedSupply && !completed ? (
            <button
              type="button"
              className={styles.prepareButton}
              onClick={onPrepare}
              disabled={
                !supported || previewNode.state !== "READY" || preparing
              }
            >
              {preparing ? "Preparing…" : "Prepare supply"}
            </button>
          ) : null}
        </section>
      ) : previewNode?.state === "UNAVAILABLE" ? (
        <p className={`${styles.runtimeNotice} ${styles.runtimeWarning}`}>
          This asset has no available Venus supply market.
        </p>
      ) : null}
    </>
  );
}

function reasonLabel(reason: SupplyMarketUnavailableReason): string {
  if (reason === "SUPPLY_PAUSED") return "Supply is paused for this market.";
  if (reason === "SUPPLY_CAP_REACHED")
    return "This market has reached its supply cap.";
  return "This asset is not listed in the Venus market.";
}

function shortAddress(value: string): string {
  return `${value.slice(0, 6)}…${value.slice(-4)}`;
}
