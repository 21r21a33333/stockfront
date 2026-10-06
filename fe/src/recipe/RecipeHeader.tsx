import { BookOpen, ChevronRight, FilePlus2, Save } from "lucide-react";
import type { RecipeValidationReport } from "@/api/generated";
import styles from "./RecipeBuilder.module.css";

export function RecipeHeader({
  title,
  validation,
  validating,
  previewing,
  canPreview,
  onPreview,
  onNewRecipe,
  onSave,
  onDebug,
  runStatus,
  settled = false,
  hasActivity = false,
  onActivity,
  onNewRun,
}: {
  title: string;
  validation?: RecipeValidationReport;
  validating: boolean;
  previewing: boolean;
  canPreview: boolean;
  onPreview(): void;
  onNewRecipe(): void;
  onSave(): void;
  onDebug(): void;
  runStatus?: string;
  settled?: boolean;
  hasActivity?: boolean;
  onActivity?(): void;
  onNewRun?(): void;
}) {
  const errors =
    validation?.diagnostics.filter((item) => item.severity === "ERROR")
      .length ?? 0;
  const warnings =
    validation?.diagnostics.filter((item) => item.severity === "WARNING")
      .length ?? 0;
  const status = validating
    ? { label: "Checking", tone: "pending" }
    : errors > 0
      ? {
          label: `${errors} ${errors === 1 ? "error" : "errors"}`,
          tone: "error",
        }
      : warnings > 0
        ? { label: `Valid · ${warnings} warning`, tone: "warning" }
        : { label: "Valid", tone: "success" };

  return (
    <>
      <header className={styles.topbar}>
        <div className={styles.crumbs}>
          <span>Recipes</span>
          <ChevronRight size={13} />
          <span>Trading</span>
          <ChevronRight size={13} />
          <strong>Builder</strong>
        </div>
        <button type="button" className={styles.docsLink} onClick={onDebug}>
          <BookOpen size={14} /> Recipe API
        </button>
      </header>
      <div className={styles.recipeHeader} aria-label="Recipe toolbar">
        <div>
          <strong>{title}</strong>
          <span
            className={`${runStatus ? styles.runBadge : ""} ${settled ? styles.settledBadge : ""}`}
          >
            {runStatus ?? "DRAFT"}
          </span>
        </div>
        <div className={styles.recipeActions}>
          <span className={`${styles.status} ${styles[status.tone]}`}>
            <i /> {status.label}
          </span>
          <code>/validate</code>
          <button
            type="button"
            className={styles.secondary}
            onClick={onNewRecipe}
          >
            <FilePlus2 size={13} /> New recipe
          </button>
          <button type="button" className={styles.secondary} onClick={onSave}>
            <Save size={13} /> Save draft
          </button>
          {hasActivity && (
            <button
              type="button"
              className={`${styles.secondary} ${styles.activityButton}`}
              onClick={onActivity}
            >
              Activity
            </button>
          )}
          {settled ? (
            <button type="button" className={styles.primary} onClick={onNewRun}>
              New run
            </button>
          ) : (
            <button
              type="button"
              className={styles.primary}
              onClick={onPreview}
              disabled={!validation?.valid || previewing || !canPreview}
              title={
                canPreview
                  ? undefined
                  : hasActivity
                    ? "Review the current execution in Activity"
                    : "Connect a wallet on BNB Chain to preview routes"
              }
            >
              {previewing ? "Finding routes…" : "Preview route"}
            </button>
          )}
        </div>
      </div>
    </>
  );
}
