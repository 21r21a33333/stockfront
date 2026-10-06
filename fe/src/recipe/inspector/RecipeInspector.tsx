import { Trash2, X } from "lucide-react";
import type { ReactNode } from "react";
import type {
  PreparedRecipe,
  RecipeDiagnostic,
  RecipeBorrowPolicy,
  RecipeRepayPolicy,
  RecipePreview,
  RecipeSupplyPolicy,
  RecipeTradePolicy,
  TradeQuote,
} from "@/api/generated";
import type {
  AssetOption,
  RecipeFlowNode,
  RecipeNodeData,
} from "../editor/types";
import { diagnosticsForNode } from "../editor/compiler";
import { canSupplyAsset } from "../editor/supplyPolicy";
import { useRecipeEditor } from "../editor/store";
import styles from "../RecipeBuilder.module.css";

import { WalletAssetFields } from "./WalletAssetFields";
import { TradeFields } from "./TradeFields";
import { SupplyFields } from "./SupplyFields";
import { SuppliedPositionFields } from "./SuppliedPositionFields";
import { WithdrawFields } from "./WithdrawFields";
import { BorrowFields } from "./BorrowFields";
import type { SuppliedPositionOption } from "../editor/withdrawalPolicy";
import type { BorrowedPositionOption } from "../editor/repaymentPolicy";
import { BorrowedPositionFields } from "./BorrowedPositionFields";
import { RepayFields } from "./RepayFields";

const emptyBorrowPolicy: RecipeBorrowPolicy = {
  allowedInputs: [],
  debtTokens: [],
  minimumHealthFactor: "1.5",
};
const emptyRepayPolicy: RecipeRepayPolicy = {
  allowedInputs: [],
  debtTokens: [],
};

export function RecipeInspector({
  node,
  assets,
  tradePolicy,
  supplyPolicy,
  borrowPolicy = emptyBorrowPolicy,
  repayPolicy = emptyRepayPolicy,
  suppliedPositions = [],
  borrowedPositions = [],
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
  supplyPolicy: RecipeSupplyPolicy;
  borrowPolicy?: RecipeBorrowPolicy;
  repayPolicy?: RecipeRepayPolicy;
  suppliedPositions?: SuppliedPositionOption[];
  borrowedPositions?: BorrowedPositionOption[];
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
  const nodeKind = node?.data.kind;
  const problems = node ? diagnosticsForNode(diagnostics, node.id) : [];
  const connectedSupply =
    node?.data.kind === "WALLET_ASSET"
      ? editor.edges
          .filter((edge) => edge.source === node.id)
          .map((edge) =>
            editor.nodes.find((candidate) => candidate.id === edge.target),
          )
          .find((candidate) => candidate?.data.kind === "SUPPLY")
      : undefined;
  const connectedSupplyVenue =
    connectedSupply?.data.kind === "SUPPLY"
      ? connectedSupply.data.venue
      : undefined;
  return (
    <aside className={styles.inspector}>
      <header className={styles.inspectorHeader}>
        <span
          className={`${styles.inspectorIcon} ${showingActivity ? "" : inspectorTone(nodeKind)}`}
        >
          ●
        </span>
        <div>
          <strong>
            {showingActivity ? "Recipe activity" : inspectorTitle(nodeKind)}
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
              <WalletAssetFields
                node={node}
                assets={assets}
                availableAssets={
                  connectedSupplyVenue
                    ? assets.filter((asset) =>
                        canSupplyAsset(
                          asset,
                          supplyPolicy,
                          connectedSupplyVenue,
                        ),
                      )
                    : assets
                }
              />
            ) : node.data.kind === "TRADE" ? (
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
            ) : node.data.kind === "SUPPLY" ? (
              <SupplyFields
                node={node}
                assets={assets}
                supplyPolicy={supplyPolicy}
                preview={preview}
                prepared={prepared}
                completed={completed}
                preparing={preparing}
                onPrepare={onPrepare}
              />
            ) : node.data.kind === "SUPPLIED_POSITION" ? (
              <SuppliedPositionFields
                node={node}
                positions={suppliedPositions}
              />
            ) : node.data.kind === "BORROWED_POSITION" ? (
              <BorrowedPositionFields
                node={node}
                positions={borrowedPositions}
              />
            ) : node.data.kind === "WITHDRAW" ? (
              <WithdrawFields
                node={node}
                assets={assets}
                positions={suppliedPositions}
                preview={preview}
                prepared={prepared}
                completed={completed}
                preparing={preparing}
                onPrepare={onPrepare}
              />
            ) : node.data.kind === "BORROW" ? (
              <BorrowFields
                node={node}
                assets={assets}
                positions={suppliedPositions}
                borrowPolicy={borrowPolicy}
                preview={preview}
                prepared={prepared}
                completed={completed}
                preparing={preparing}
                onPrepare={onPrepare}
              />
            ) : (
              <RepayFields
                node={node}
                assets={assets}
                positions={borrowedPositions}
                repayPolicy={repayPolicy}
                preview={preview}
                prepared={prepared}
                completed={completed}
                preparing={preparing}
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

function inspectorTitle(kind: RecipeNodeData["kind"] | undefined): string {
  switch (kind) {
    case "WALLET_ASSET":
      return "Wallet Asset";
    case "SUPPLIED_POSITION":
      return "Supplied position";
    case "BORROWED_POSITION":
      return "Borrowed position";
    case "TRADE":
      return "Trade";
    case "SUPPLY":
      return "Supply";
    case "WITHDRAW":
      return "Withdraw";
    case "BORROW":
      return "Borrow stablecoin";
    case "REPAY":
      return "Repay debt";
    default:
      return "Recipe";
  }
}

function inspectorTone(kind: RecipeNodeData["kind"] | undefined): string {
  switch (kind) {
    case "TRADE":
      return styles.orangeIcon;
    case "SUPPLY":
      return styles.greenIcon;
    case "SUPPLIED_POSITION":
    case "BORROWED_POSITION":
    case "WITHDRAW":
      return styles.blueIcon;
    case "BORROW":
    case "REPAY":
      return styles.orangeIcon;
    default:
      return "";
  }
}
