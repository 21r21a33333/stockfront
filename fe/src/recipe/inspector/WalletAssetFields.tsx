import { ChevronDown } from "lucide-react";
import { formatUnits } from "viem";
import { AssetSelect } from "@/ui/assets/AssetSelect";
import type { AssetOption, RecipeFlowNode } from "../editor/types";
import { useRecipeEditor } from "../editor/store";
import { formatDisplayAmount } from "../format";
import styles from "../RecipeBuilder.module.css";

export function WalletAssetFields({
  node,
  assets,
  availableAssets = assets,
}: {
  node: RecipeFlowNode;
  assets: AssetOption[];
  availableAssets?: AssetOption[];
}) {
  const editor = useRecipeEditor();
  if (node.data.kind !== "WALLET_ASSET") return null;
  const token = node.data.token;
  const asset = assets.find(
    (item) => item.address.toLowerCase() === token?.toLowerCase(),
  );
  const balance =
    asset?.balanceRaw !== undefined
      ? formatUnits(asset.balanceRaw, asset.decimals)
      : undefined;
  const supported = availableAssets.some(
    (item) => item.address.toLowerCase() === token?.toLowerCase(),
  );
  return (
    <>
      <div className={styles.inspectorSectionTitle}>
        <strong>Source</strong>
        <ChevronDown size={13} />
      </div>
      <p className={styles.portCaption}>
        Output port Asset&lt;{asset?.symbol ?? "?"}&gt;
      </p>
      <section className={styles.fieldGroup}>
        <label>Asset</label>
        <AssetSelect
          assets={availableAssets}
          value={node.data.token}
          selectedAsset={asset}
          invalid={Boolean(token && !supported)}
          onChange={(token) => editor.updateNode(node.id, { token })}
        />
        {token && !supported ? (
          <p className={`${styles.runtimeNotice} ${styles.runtimeWarning}`}>
            Choose a supported bStock for the connected Venus Supply action.
          </p>
        ) : null}
        <label>Amount</label>
        <div className={styles.amountField}>
          <input
            value={node.data.amount}
            inputMode="decimal"
            onChange={(event) =>
              editor.updateNode(node.id, { amount: event.target.value })
            }
            aria-label="Amount"
          />
          <span>{asset?.symbol ?? "TOKEN"}</span>
        </div>
        <div className={styles.percentages}>
          {[25, 50, 75, 100].map((percentage) => (
            <button
              type="button"
              key={percentage}
              disabled={asset?.balanceRaw === undefined}
              onClick={() =>
                asset?.balanceRaw !== undefined &&
                editor.updateNode(node.id, {
                  amount: formatUnits(
                    (asset.balanceRaw * BigInt(percentage)) / 100n,
                    asset.decimals,
                  ),
                })
              }
            >
              {percentage === 100 ? "Max" : `${percentage}%`}
            </button>
          ))}
        </div>
        <p className={styles.balanceLine}>
          Wallet balance{" "}
          <strong>{balance ? formatDisplayAmount(balance) : "—"}</strong>
        </p>
      </section>
    </>
  );
}
