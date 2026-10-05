import { useMutation, useQuery } from "@tanstack/react-query";
import { ReactFlowProvider } from "@xyflow/react";
import { useEffect, useMemo, useState } from "react";
import { useAccount } from "wagmi";
import { StockfrontApiError } from "@/api/client";
import { useRecipeAssets } from "@/recipe/assets";
import { recipesApi } from "@/api/recipes";
import { usePreparedExecution } from "@/wallet/execution";
import type {
  PrepareRecipeRequest,
  RecipeDiagnostic,
  RecipePreview,
  TradeQuote,
} from "@/api/generated";
import { ActionPalette } from "@/recipe/ActionPalette";
import { buildRecipe } from "@/recipe/editor/compiler";
import { BuilderContext } from "@/recipe/editor/context";
import { RecipeCanvas } from "@/recipe/canvas/RecipeCanvas";
import { RecipeHeader } from "@/recipe/RecipeHeader";
import { RecipeInspector } from "@/recipe/inspector/RecipeInspector";
import { useRecipeEditor } from "@/recipe/editor/store";
import styles from "@/recipe/RecipeBuilder.module.css";

export function RecipeBuilderPage() {
  const account = useAccount();
  const editor = useRecipeEditor();
  const catalog = useQuery({
    queryKey: ["recipe-catalog"],
    queryFn: recipesApi.catalog,
  });
  const assets = useRecipeAssets(catalog.data);
  const defaults = useMemo(
    () => ({
      settlementToken: catalog.data?.settlementTokens[0],
      bstockToken: catalog.data?.bstocks[0]?.contract,
    }),
    [catalog.data],
  );

  useEffect(() => {
    if (catalog.data && assets.length > 0) editor.initialize(defaults);
  }, [assets.length, catalog.data, defaults, editor]);

  const recipe = useMemo(
    () =>
      buildRecipe(
        catalog.data?.schemaVersion ?? 1,
        editor.nodes,
        editor.edges,
        assets,
      ),
    [assets, catalog.data?.schemaVersion, editor.edges, editor.nodes],
  );
  const recipeKey = JSON.stringify(recipe);
  const validation = useQuery({
    queryKey: ["recipe-validation", recipeKey],
    queryFn: () => recipesApi.validate(recipe),
    enabled: editor.initialized,
  });
  const diagnostics =
    validation.data?.diagnostics ?? validationError(validation.error);

  const [selectedRoutes, setSelectedRoutes] = useState<
    Record<string, TradeQuote>
  >({});
  const preview = useMutation({
    mutationFn: recipesApi.preview,
    onSuccess: (result) => {
      setSelectedRoutes(
        Object.fromEntries(
          result.nodes.flatMap((node) =>
            node.routes?.[0] ? [[node.nodeId, node.routes[0]]] : [],
          ),
        ),
      );
    },
  });
  const prepare = useMutation({ mutationFn: recipesApi.prepare });
  const execution = usePreparedExecution(prepare.data);
  const [executing, setExecuting] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    preview.reset();
    prepare.reset();
    setSelectedRoutes({});
  }, [account.address, account.chainId, recipeKey]); // eslint-disable-line react-hooks/exhaustive-deps

  const selectedNode = editor.nodes.find(
    (node) => node.id === editor.selectedNodeId,
  );
  const selectedRoute = selectedNode
    ? selectedRoutes[selectedNode.id]
    : undefined;
  const requestError =
    readableError(preview.error) ??
    readableError(prepare.error) ??
    execution.error ??
    readableError(catalog.error);

  function previewRecipe(): void {
    if (!account.address) return;
    prepare.reset();
    preview.mutate({ recipe, context: { wallet: account.address } });
  }

  function prepareRecipe(): void {
    if (!account.address || !preview.data) return;
    const request: PrepareRecipeRequest = {
      recipe,
      context: { wallet: account.address },
      selections: preview.data.nodes.flatMap((node) => {
        const route = selectedRoutes[node.nodeId];
        return route
          ? [
              {
                kind: "TRADE" as const,
                nodeId: node.nodeId,
                routeQuoteId: route.routeQuoteId,
                venue: route.venue,
              },
            ]
          : [];
      }),
    };
    prepare.mutate(request);
  }

  async function executeRecipe(): Promise<void> {
    if (!prepare.data || execution.completed) return;
    setExecuting(true);
    try {
      await execution.execute();
    } catch {
      // The execution hook exposes the wallet error in the inspector.
    } finally {
      setExecuting(false);
    }
  }

  if (catalog.isPending) {
    return <CenteredState label="Loading recipe primitives…" />;
  }
  if (catalog.isError || !catalog.data) {
    return (
      <CenteredState
        label={requestError ?? "Recipe catalog is temporarily unavailable."}
        action={() => catalog.refetch()}
      />
    );
  }

  return (
    <BuilderContext.Provider
      value={{
        assets,
        diagnostics,
        selectedRoutes,
        prepared: prepare.data,
        execution: execution.progress,
        connectedOutputs: editor.edges
          .filter((edge) => edge.sourceHandle === "buy")
          .map((edge) => edge.source),
        appendTrade: (sourceId) => editor.appendTrade(sourceId, defaults),
      }}
    >
      <div className={styles.page}>
        <div className={styles.shell}>
          <ActionPalette
            primitives={catalog.data.primitives}
            mode={editor.mode}
            onModeChange={editor.setMode}
            onAdd={(kind) =>
              editor.addNode(
                kind,
                {
                  x: 160 + editor.nodes.length * 45,
                  y: 145 + editor.nodes.length * 35,
                },
                defaults,
              )
            }
          />
          <section className={styles.workspace}>
            <RecipeHeader
              validation={validation.data}
              validating={validation.isFetching}
              previewing={preview.isPending}
              canPreview={Boolean(account.address && account.chainId === 56)}
              onPreview={previewRecipe}
              onDebug={() => editor.setMode("debug")}
              onSave={() => {
                editor.saveDraft();
                setSaved(true);
                window.setTimeout(() => setSaved(false), 1600);
              }}
            />
            <div className={styles.editorArea}>
              <ReactFlowProvider>
                <RecipeCanvas
                  defaults={defaults}
                  mode={editor.mode}
                  onModeChange={editor.setMode}
                />
              </ReactFlowProvider>
              {editor.mode === "debug" ? (
                <DebugPanel
                  recipe={recipe}
                  diagnostics={diagnostics}
                  onClose={() => editor.setMode("builder")}
                />
              ) : (
                <RecipeInspector
                  node={selectedNode}
                  assets={assets}
                  diagnostics={diagnostics}
                  preview={preview.data as RecipePreview | undefined}
                  selectedRouteId={selectedRoute?.routeQuoteId}
                  prepared={prepare.data}
                  execution={execution.progress}
                  completed={execution.completed}
                  preparing={prepare.isPending}
                  executing={executing}
                  error={requestError}
                  onRouteSelect={(route) => {
                    if (!selectedNode) return;
                    setSelectedRoutes((current) => ({
                      ...current,
                      [selectedNode.id]: route,
                    }));
                    prepare.reset();
                  }}
                  onPrepare={prepareRecipe}
                  onExecute={() => void executeRecipe()}
                />
              )}
              {saved ? (
                <div className={styles.savedToast}>Draft saved locally</div>
              ) : null}
              {!account.address ? (
                <div className={styles.walletHint}>
                  Connect a wallet to preview live routes
                </div>
              ) : account.chainId !== 56 ? (
                <div className={styles.walletHint}>
                  Switch to BNB Chain to continue
                </div>
              ) : null}
            </div>
          </section>
        </div>
      </div>
    </BuilderContext.Provider>
  );
}

function DebugPanel({
  recipe,
  diagnostics,
  onClose,
}: {
  recipe: object;
  diagnostics: RecipeDiagnostic[];
  onClose(): void;
}) {
  return (
    <aside className={`${styles.inspector} ${styles.debugPanel}`} id="debug">
      <header className={styles.inspectorHeader}>
        <span className={styles.inspectorIcon}>{"{}"}</span>
        <div>
          <strong>Compiler input</strong>
          <small>schema v1</small>
        </div>
        <button type="button" aria-label="Close debug panel" onClick={onClose}>
          ×
        </button>
      </header>
      <div className={styles.inspectorScroll}>
        <h3>Recipe JSON</h3>
        <pre>{JSON.stringify(recipe, null, 2)}</pre>
        <h3>Diagnostics</h3>
        {diagnostics.length === 0 ? (
          <p className={styles.debugClear}>No compiler diagnostics.</p>
        ) : (
          diagnostics.map((item) => (
            <div
              className={styles.debugDiagnostic}
              key={`${item.code}-${item.message}`}
            >
              <strong>{item.code}</strong>
              <p>{item.message}</p>
            </div>
          ))
        )}
      </div>
    </aside>
  );
}

function CenteredState({
  label,
  action,
}: {
  label: string;
  action?: () => void;
}) {
  return (
    <main className={styles.centeredState}>
      <span>●</span>
      <strong>{label}</strong>
      {action ? (
        <button type="button" onClick={action}>
          Try again
        </button>
      ) : null}
    </main>
  );
}

function validationError(error: unknown): RecipeDiagnostic[] {
  return error instanceof StockfrontApiError ? error.diagnostics : [];
}

function readableError(error: unknown): string | undefined {
  return error instanceof Error ? error.message : undefined;
}
