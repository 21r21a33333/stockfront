import { Trash2, X } from "lucide-react";
import type { ReactNode } from "react";
import type {
  PreparedRecipe,
  RecipeDiagnostic,
  RecipePreview,
  RecipeTradePolicy,
  TradeQuote,
} from "@/api/generated";
import type { AssetOption, RecipeFlowNode } from "../editor/types";
import { diagnosticsForNode } from "../editor/compiler";
import { useRecipeEditor } from "../editor/store";
import styles from "../RecipeBuilder.module.css";

import { WalletAssetFields } from "./WalletAssetFields";
import { TradeFields } from "./TradeFields";

export function RecipeInspector({
  node,
  assets,
  tradePolicy,
  diagnostics,
  preview,
  selectedRouteId,
  prepared,
  completed,
  preparing,
  activity,
  view = "configure",
  onViewChange,
  error,
  onRouteSelect,
  onPrepare,
}: {
  node?: RecipeFlowNode;
  assets: AssetOption[];
  tradePolicy: RecipeTradePolicy;
  diagnostics: RecipeDiagnostic[];
  preview?: RecipePreview;
  selectedRouteId?: string;
  prepared?: PreparedRecipe;
  completed: boolean;
  preparing: boolean;
  activity?: ReactNode;
  view?: "configure" | "activity";
  onViewChange?(view: "configure" | "activity"): void;
  error?: string;
  onRouteSelect(route: TradeQuote): void;
  onPrepare(): void;
}) {
  const editor = useRecipeEditor();
  if (!node && !activity) {
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
  const showingActivity = view === "activity" && Boolean(activity);
  const problems = node ? diagnosticsForNode(diagnostics, node.id) : [];
  return (
    <aside className={styles.inspector}>
      <header className={styles.inspectorHeader}>
        <span
          className={`${styles.inspectorIcon} ${!showingActivity && node?.data.kind === "TRADE" ? styles.orangeIcon : ""}`}
        >
          ●
        </span>
        <div>
          <strong>
            {showingActivity
              ? "Recipe activity"
              : node?.data.kind === "TRADE"
                ? "Trade"
                : node
                  ? "Wallet Asset"
                  : "Recipe"}
          </strong>
          <small>
            {showingActivity
              ? "Transactions & settlement"
              : "Configure this action"}
          </small>
        </div>
        <button
          type="button"
          aria-label="Close inspector"
          onClick={() => {
            editor.selectNode(undefined);
            onViewChange?.("configure");
          }}
        >
          <X size={15} />
        </button>
      </header>
      {activity && (
        <nav className={styles.inspectorTabs} aria-label="Inspector view">
          <button
            type="button"
            aria-pressed={!showingActivity}
            onClick={() => onViewChange?.("configure")}
          >
            Configure
          </button>
          <button
            type="button"
            aria-pressed={showingActivity}
            onClick={() => onViewChange?.("activity")}
          >
            Activity
          </button>
        </nav>
      )}
      <div className={styles.inspectorScroll}>
        {showingActivity ? (
          activity
        ) : !node ? (
          <p>Select an action to configure it.</p>
        ) : (
          <>
            {node.data.kind === "WALLET_ASSET" ? (
              <WalletAssetFields node={node} assets={assets} />
            ) : (
              <TradeFields
                node={node}
                assets={assets}
                tradePolicy={tradePolicy}
                preview={preview}
                selectedRouteId={selectedRouteId}
                prepared={prepared}
                completed={completed}
                preparing={preparing}
                onRouteSelect={onRouteSelect}
                onPrepare={onPrepare}
              />
            )}
            <section className={styles.diagnosticsSection}>
              <div className={styles.inspectorSectionTitle}>
                <strong>Diagnostics</strong>
                <span
                  className={problems.length ? styles.diagnosticWarning : ""}
                >
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
                <p className={styles.clearDiagnostics}>
                  No issues on this node.
                </p>
              )}
            </section>
          </>
        )}
        {error ? <p className={styles.requestError}>{error}</p> : null}
      </div>
      {!showingActivity && node && (
        <button
          type="button"
          className={styles.deleteAction}
          onClick={editor.removeSelected}
        >
          <Trash2 size={13} /> Remove action
        </button>
      )}
    </aside>
  );
}
