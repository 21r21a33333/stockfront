import { Trash2, X } from "lucide-react";
import type {
  PreparedRecipe,
  RecipeDiagnostic,
  RecipePreview,
  TradeQuote,
} from "@/api/generated";
import type { ExecutionProgress } from "@/wallet/execution";
import type { AssetOption, RecipeFlowNode } from "../editor/types";
import { diagnosticsForNode } from "../editor/compiler";
import { useRecipeEditor } from "../editor/store";
import styles from "../RecipeBuilder.module.css";

import { WalletAssetFields } from "./WalletAssetFields";
import { TradeFields } from "./TradeFields";

export function RecipeInspector({
  node,
  assets,
  diagnostics,
  preview,
  selectedRouteId,
  prepared,
  execution,
  completed,
  preparing,
  executing,
  error,
  onRouteSelect,
  onPrepare,
  onExecute,
}: {
  node?: RecipeFlowNode;
  assets: AssetOption[];
  diagnostics: RecipeDiagnostic[];
  preview?: RecipePreview;
  selectedRouteId?: string;
  prepared?: PreparedRecipe;
  execution: ExecutionProgress[];
  completed: boolean;
  preparing: boolean;
  executing: boolean;
  error?: string;
  onRouteSelect(route: TradeQuote): void;
  onPrepare(): void;
  onExecute(): void;
}) {
  const editor = useRecipeEditor();
  if (!node) {
    return (
      <aside className={styles.inspector}>
        <div className={styles.emptyInspector}>
          <span>✦</span>
          <strong>Select an action</strong>
          <p>Choose a node to configure its fields and review its output.</p>
        </div>
      </aside>
    );
  }
  const problems = diagnosticsForNode(diagnostics, node.id);
  return (
    <aside className={styles.inspector}>
      <header className={styles.inspectorHeader}>
        <span
          className={`${styles.inspectorIcon} ${node.data.kind === "TRADE" ? styles.orangeIcon : ""}`}
        >
          ●
        </span>
        <div>
          <strong>
            {node.data.kind === "TRADE" ? "Trade" : "Wallet Asset"}
          </strong>
          <small>{node.id.slice(0, 8)}</small>
        </div>
        <button
          type="button"
          aria-label="Close inspector"
          onClick={() => editor.selectNode(undefined)}
        >
          <X size={15} />
        </button>
      </header>
      <div className={styles.inspectorScroll}>
        {node.data.kind === "WALLET_ASSET" ? (
          <WalletAssetFields node={node} assets={assets} />
        ) : (
          <TradeFields
            node={node}
            assets={assets}
            preview={preview}
            selectedRouteId={selectedRouteId}
            prepared={prepared}
            execution={execution}
            completed={completed}
            preparing={preparing}
            executing={executing}
            onRouteSelect={onRouteSelect}
            onPrepare={onPrepare}
            onExecute={onExecute}
          />
        )}
        <section className={styles.diagnosticsSection}>
          <div className={styles.inspectorSectionTitle}>
            <strong>Diagnostics</strong>
            <span className={problems.length ? styles.diagnosticWarning : ""}>
              {problems.length
                ? `${problems.length} ISSUE${problems.length > 1 ? "S" : ""}`
                : "CLEAR"}
            </span>
          </div>
          {problems.length > 0 ? (
            <div className={styles.diagnostics}>
              {problems.map((problem) => (
                <p key={`${problem.code}-${problem.message}`}>
                  ! {problem.message}
                </p>
              ))}
            </div>
          ) : (
            <p className={styles.clearDiagnostics}>No issues on this node.</p>
          )}
        </section>
        {error ? <p className={styles.requestError}>{error}</p> : null}
      </div>
      <button
        type="button"
        className={styles.deleteAction}
        onClick={editor.removeSelected}
      >
        <Trash2 size={13} /> Remove action
      </button>
    </aside>
  );
}
