import { useMutation, useQuery } from "@tanstack/react-query";
import { ReactFlowProvider } from "@xyflow/react";
import { useEffect, useMemo, useRef, useState } from "react";
import { formatUnits } from "viem";
import { useAccount } from "wagmi";
import { StockfrontApiError } from "@/api/client";
import { useRecipeAssets } from "@/recipe/assets";
import { recipesApi } from "@/api/recipes";
import { useRecipeExecution } from "./execution/useRecipeExecution";
import { ExecutionPanel } from "./execution/ExecutionPanel";
import type { RecipeFlowNode } from "./editor/types";
import type {
  PrepareRecipeRequest,
  RecipePreviewRequest,
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
  const requestScope = `${account.address?.toLowerCase()}:${account.chainId}:${recipeKey}`;
  const scope = useRef(requestScope);
  scope.current = requestScope;
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
  const [inspectorView, setInspectorView] = useState<"configure" | "activity">(
    "configure",
  );
  const latestPreviewRequest = useRef<RecipePreviewRequest>();
  const preview = useMutation({
    mutationFn: recipesApi.preview,
    onSuccess: (result, request) => {
      if (
        request !== latestPreviewRequest.current ||
        scope.current !==
          `${request.context.wallet.toLowerCase()}:${request.recipe.chainId}:${JSON.stringify(request.recipe)}`
      )
        return;
      setSelectedRoutes(
        Object.fromEntries(
          result.nodes.flatMap((node) =>
            node.routes?.[0] ? [[node.nodeId, node.routes[0]]] : [],
          ),
        ),
      );
    },
  });
  const prepare = useMutation({
    mutationFn: recipesApi.prepare,
    onSuccess: (_result, request) => {
      if (
        scope.current ===
        `${request.context.wallet.toLowerCase()}:${request.recipe.chainId}:${JSON.stringify(request.recipe)}`
      )
        setInspectorView("activity");
    },
  });
  const matches = (
    request?:
      | PrepareRecipeRequest
      | { recipe: typeof recipe; context: { wallet: string } },
  ) =>
    request &&
    requestScope ===
      `${request.context.wallet.toLowerCase()}:${request.recipe.chainId}:${JSON.stringify(request.recipe)}`;
  const activePrepared = matches(prepare.variables) ? prepare.data : undefined;
  const activePreview = matches(preview.variables) ? preview.data : undefined;
  const execution = useRecipeExecution({ recipe, prepared: activePrepared });
  useEffect(() => {
    if (execution.run?.id) setInspectorView("activity");
  }, [execution.run?.id]);
  const hasActivity = Boolean(execution.run || activePrepared);
  const runStatus = execution.completed
    ? "Settled"
    : execution.progress.some(
          (item) => item.phase === "attention" || item.phase === "failed",
        )
      ? "Needs attention"
      : execution.busy
        ? "Confirming"
        : execution.blocked
          ? "Pending settlement"
          : hasActivity
            ? "Ready to execute"
            : undefined;
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
    if (
      !account.address ||
      execution.blocked ||
      execution.busy ||
      !execution.sameRecipe ||
      execution.completed
    )
      return;
    prepare.reset();
    setInspectorView("configure");
    const request = { recipe, context: { wallet: account.address } };
    latestPreviewRequest.current = request;
    preview.mutate(request);
  }

  function prepareRecipe(): void {
    if (
      !account.address ||
      !activePreview ||
      execution.blocked ||
      execution.busy ||
      !execution.sameRecipe ||
      execution.completed
    )
      return;
    const request: PrepareRecipeRequest = {
      recipe,
      context: { wallet: account.address },
      selections: activePreview.nodes.flatMap((node) => {
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
    if (execution.canExecute) await execution.execute();
  }

  function restoreExecutionRecipe(): void {
    const snapshot = execution.run?.recipe;
    if (!snapshot) return;
    // Token decimals come from the same catalog used to compile the original recipe.
    if (
      snapshot.nodes.some(
        (node) =>
          node.kind === "WALLET_ASSET" &&
          !assets.some(
            (asset) => asset.address.toLowerCase() === node.token.toLowerCase(),
          ),
      )
    )
      return;
    const nodes: RecipeFlowNode[] = snapshot.nodes.map((node, index) => ({
      id: node.id,
      type: node.kind === "TRADE" ? "trade" : "walletAsset",
      position: { x: 70 + index * 385, y: 180 },
      data:
        node.kind === "TRADE"
          ? { kind: "TRADE", buyToken: node.buyToken }
          : {
              kind: "WALLET_ASSET",
              token: node.token,
              amount: formatUnits(
                BigInt(node.rawAmount),
                assets.find(
                  (asset) =>
                    asset.address.toLowerCase() === node.token.toLowerCase(),
                )!.decimals,
              ),
            },
    }));
    useRecipeEditor.setState({
      nodes,
      edges: snapshot.edges.map((edge) => ({
        id: edge.id,
        type: "asset",
        source: edge.from.nodeId,
        sourceHandle: edge.from.port,
        target: edge.to.nodeId,
        targetHandle: edge.to.port,
      })),
      selectedNodeId: nodes.find((node) => node.data.kind === "TRADE")?.id,
    });
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
        prepared: execution.sameRecipe ? activePrepared : undefined,
        execution: execution.sameRecipe ? execution.progress : [],
        connectedOutputs: editor.edges
          .filter((edge) => edge.sourceHandle === "buy")
          .map((edge) => edge.source),
        appendTrade: (sourceId) => editor.appendTrade(sourceId, defaults),
        openActivity: () => {
          editor.setMode("builder");
          setInspectorView("activity");
        },
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
              canPreview={Boolean(
                account.address &&
                account.chainId === 56 &&
                !execution.blocked &&
                !execution.busy &&
                !execution.completed &&
                execution.sameRecipe,
              )}
              onPreview={previewRecipe}
              onDebug={() => editor.setMode("debug")}
              runStatus={
                runStatus && !execution.sameRecipe
                  ? `Previous run · ${runStatus}`
                  : runStatus
              }
              settled={execution.completed}
              hasActivity={hasActivity}
              onActivity={() => {
                editor.setMode("builder");
                setInspectorView("activity");
              }}
              onNewRun={() => {
                void execution.startNewRun().then(() => {
                  prepare.reset();
                  preview.reset();
                  setInspectorView("configure");
                });
              }}
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
                  preview={activePreview as RecipePreview | undefined}
                  selectedRouteId={selectedRoute?.routeQuoteId}
                  prepared={execution.sameRecipe ? activePrepared : undefined}
                  completed={execution.sameRecipe && execution.completed}
                  preparing={prepare.isPending}
                  view={inspectorView}
                  onViewChange={setInspectorView}
                  activity={
                    hasActivity ? (
                      <ExecutionPanel
                        run={execution.run}
                        recipe={recipe}
                        prepared={
                          execution.sameRecipe ? activePrepared : undefined
                        }
                        assets={assets}
                        busy={execution.busy}
                        paused={execution.paused}
                        sameRecipe={execution.sameRecipe}
                        canExecute={execution.canExecute}
                        canRetrySubmission={execution.canRetrySubmission}
                        onExecute={() => void executeRecipe()}
                        onCheck={() => void execution.checkStatus()}
                        onRetry={() => void execution.retrySubmission()}
                        onRefresh={previewRecipe}
                        onRestore={restoreExecutionRecipe}
                      />
                    ) : undefined
                  }
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
