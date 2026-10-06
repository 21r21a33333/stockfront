import { CircleDollarSign } from "lucide-react";
import { AssetIdentity } from "@/ui/assets/AssetIdentity";
import { shortAddress } from "../assets";
import type { RecipeFlowNode } from "../editor/types";
import {
  borrowedPositionFor,
  type BorrowedPositionOption,
} from "../editor/repaymentPolicy";
import { useRecipeEditor } from "../editor/store";
import { formatPositionAmount, formatUsd } from "../format";
import styles from "../RecipeBuilder.module.css";

export function BorrowedPositionFields({
  node,
  positions,
}: {
  node: RecipeFlowNode;
  positions: BorrowedPositionOption[];
}) {
  const editor = useRecipeEditor();
  if (node.data.kind !== "BORROWED_POSITION") return null;
  const selected = borrowedPositionFor(
    positions,
    node.data.pool,
    node.data.token,
  );

  return (
    <>
      <div className={styles.inspectorSectionTitle}>
        <strong>Borrowed position</strong>
        <span>LIVE</span>
      </div>
      <p className={styles.portCaption}>
        Choose an existing Venus debt. Repayment always uses the full live debt.
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
            <CircleDollarSign size={18} />
            <strong>No supported Venus debt</strong>
            <p>Borrow a supported stablecoin, then refresh this board.</p>
          </div>
        ) : null}
      </section>
      {selected ? (
        <section className={styles.positionSummary}>
          <span>Venus Core Pool</span>
          <code>{shortAddress(selected.pool)}</code>
          <p>The exact debt is refreshed again before wallet preparation.</p>
        </section>
      ) : null}
    </>
  );
}
