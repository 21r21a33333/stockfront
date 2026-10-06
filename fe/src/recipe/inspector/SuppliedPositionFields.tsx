import { Landmark } from "lucide-react";
import { AssetIdentity } from "@/ui/assets/AssetIdentity";
import { shortAddress } from "../assets";
import type { RecipeFlowNode } from "../editor/types";
import {
  suppliedPositionFor,
  type SuppliedPositionOption,
} from "../editor/withdrawalPolicy";
import { useRecipeEditor } from "../editor/store";
import { formatPositionAmount, formatUsd } from "../format";
import styles from "../RecipeBuilder.module.css";

export function SuppliedPositionFields({
  node,
  positions,
}: {
  node: RecipeFlowNode;
  positions: SuppliedPositionOption[];
}) {
  const editor = useRecipeEditor();
  if (node.data.kind !== "SUPPLIED_POSITION") return null;
  const data = node.data;
  const selected = suppliedPositionFor(positions, data.pool, data.token);

  return (
    <>
      <div className={styles.inspectorSectionTitle}>
        <strong>Supplied position</strong>
        <span>LIVE</span>
      </div>
      <p className={styles.portCaption}>
        Choose an existing Venus position. The pool and asset stay bound
        together.
      </p>
      <section className={styles.positionList}>
        {positions.map((position) => {
          const active = position === selected;
          return (
            <button
              type="button"
              aria-pressed={active}
              key={`${position.pool}:${position.asset.token.contract}`}
              onClick={() =>
                editor.updateNode(node.id, {
                  venue: position.venue,
                  pool: position.pool,
                  token: position.asset.token.contract,
                })
              }
            >
              <AssetIdentity asset={position.catalogAsset} />
              <span>
                <strong>
                  {formatPositionAmount(position.asset.amount)}{" "}
                  {position.catalogAsset.symbol}
                </strong>
                <small>{formatUsd(position.asset.valueUsd)}</small>
              </span>
            </button>
          );
        })}
        {positions.length === 0 ? (
          <div className={styles.emptyPosition}>
            <Landmark size={18} />
            <strong>No supported supplied positions</strong>
            <p>Supply a supported bStock to Venus, then refresh this board.</p>
          </div>
        ) : null}
      </section>
      {selected ? (
        <section className={styles.positionSummary}>
          <span>Venus Core Pool</span>
          <code>{shortAddress(selected.pool)}</code>
          <p>This recipe will use the full supplied balance.</p>
        </section>
      ) : null}
    </>
  );
}
