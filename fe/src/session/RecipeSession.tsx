import { useMutation, useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { Address } from "viem";
import { useAccount } from "wagmi";
import type {
  BasketPosition,
  CarryPosition,
  RecipeDefinition,
  RecipePreviewRequest,
  TradeQuote,
} from "@/api/generated";
import { StockfrontApiError } from "@/api/client";
import { recipesApi } from "@/api/recipes";
import { useRecipeAssets } from "@/recipe/assets";
import { buildRecipe } from "@/recipe/editor/compiler";
import {
  chainOrder,
  graphFromDefinition,
  linearGraph,
} from "@/recipe/editor/graphs";
import { selectableBorrowedPositions } from "@/recipe/editor/repaymentPolicy";
import { useRecipeEditor, type EditorDefaults } from "@/recipe/editor/store";
import type { RecipeNodeData } from "@/recipe/editor/types";
import { selectableSuppliedPositions } from "@/recipe/editor/withdrawalPolicy";
import { recipeRuntimeContext } from "@/recipe/execution/runtimeContext";
import { useRecipeExecution } from "@/recipe/execution/useRecipeExecution";
import {
  RecipeSessionContext,
  type FreshPreparation,
  type RecipeSessionValue,
} from "./context";
import { stockfrontChain } from "@/wallet/config";

export function RecipeSessionProvider({
  children,
  autoPreview,
}: {
  children: ReactNode;
  /** Live quotes are only fetched while a screen shows them. */
  autoPreview: boolean;
}) {
  const account = useAccount();
  const editor = useRecipeEditor();
  const catalog = useQuery({
    queryKey: ["recipe-catalog"],
    queryFn: recipesApi.catalog,
    // The language and the asset list outlive a session; prices inside it are
    // only labels here, and live balances come from the chain separately.
    staleTime: 5 * 60_000,
  });
  const { assets, refetchBalances } = useRecipeAssets(catalog.data);
  const walletReady = Boolean(
    account.address && account.chainId === stockfrontChain.id,
  );
  const borrowing = useQuery({
    queryKey: ["borrowing-account", account.address?.toLowerCase()],
    queryFn: () => recipesApi.borrowingAccount(account.address!),
    enabled: walletReady,
    refetchInterval: 15_000,
  });
  const basketHoldings = useQuery({
    queryKey: ["basket-positions", account.address?.toLowerCase()],
    queryFn: () => recipesApi.basketPositions(account.address!),
    enabled: walletReady,
    refetchInterval: 15_000,
  });
  const earn = useQuery({
    queryKey: ["carry-positions", account.address?.toLowerCase()],
    queryFn: () => recipesApi.carryPositions(account.address!),
    enabled: walletReady,
    refetchInterval: 15_000,
  });

  const suppliedPositions = useMemo(
    () =>
      catalog.data
        ? selectableSuppliedPositions(
            borrowing.data,
            {
              allowedInputs: [
                ...catalog.data.withdrawalPolicy.allowedInputs,
                ...catalog.data.borrowPolicy.allowedInputs.map(
                  ({ venue, pool }) => ({ venue, pool }),
                ),
              ],
            },
            assets,
          )
        : [],
    [assets, borrowing.data, catalog.data],
  );
  const borrowedPositions = useMemo(
    () =>
      catalog.data
        ? selectableBorrowedPositions(
            borrowing.data,
            catalog.data.repayPolicy,
            assets,
          )
        : [],
    [assets, borrowing.data, catalog.data],
  );
  const defaults = useMemo<EditorDefaults>(
    () => ({
      settlementToken: catalog.data?.settlementTokens[0],
      debtToken: catalog.data?.borrowPolicy.debtTokens[0],
      bstockToken: catalog.data?.bstocks[0]?.contract,
      suppliedPosition: suppliedPositions[0]
        ? {
            venue: suppliedPositions[0].venue,
            pool: suppliedPositions[0].pool,
            token: suppliedPositions[0].asset.token.contract,
          }
        : undefined,
      borrowedPosition: borrowedPositions[0]
        ? {
            venue: borrowedPositions[0].venue,
            pool: borrowedPositions[0].pool,
            token: borrowedPositions[0].asset.token.contract,
          }
        : undefined,
    }),
    [borrowedPositions, catalog.data, suppliedPositions],
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
  const ordered = useMemo(
    () => chainOrder(editor.nodes, editor.edges),
    [editor.edges, editor.nodes],
  );
  const recipeKey = JSON.stringify(recipe);
  const validation = useQuery({
    queryKey: ["recipe-validation", recipeKey],
    queryFn: () => recipesApi.validate(recipe),
    enabled: editor.initialized && recipe.nodes.length > 0,
  });
  const diagnostics =
    validation.data?.diagnostics ??
    (validation.error instanceof StockfrontApiError
      ? validation.error.diagnostics
      : []);

  const execution = useRecipeExecution({ recipe, prepared: undefined });
  // A run saved for another draft must not contribute completed nodes here.
  const runtimeContext = account.address
    ? recipeRuntimeContext(
        account.address,
        execution.sameRecipe ? (execution.run?.attempts ?? []) : [],
      )
    : undefined;
  const scopeKey = `${account.address?.toLowerCase()}:${account.chainId}:${recipeKey}:${JSON.stringify(runtimeContext?.completedNodes ?? [])}`;
  const scope = useRef(scopeKey);
  scope.current = scopeKey;
  const requestScope = (request: RecipePreviewRequest) =>
    `${request.context.wallet.toLowerCase()}:${account.chainId}:${JSON.stringify(request.recipe)}:${JSON.stringify(request.context.completedNodes ?? [])}`;

  const [selectedRoutes, setSelectedRoutes] = useState<
    Record<string, TradeQuote>
  >({});
  const routesRef = useRef(selectedRoutes);
  routesRef.current = selectedRoutes;
  const preview = useMutation({
    mutationFn: recipesApi.preview,
    onSuccess: (result, request) => {
      if (requestScope(request) !== scope.current) return;
      setSelectedRoutes((current) =>
        Object.fromEntries(
          result.nodes.flatMap((node) => {
            if (node.kind !== "TRADE" || !node.routes?.length) return [];
            const kept = current[node.nodeId];
            const same =
              kept &&
              node.routes.find(
                (route) =>
                  route.venue === kept.venue &&
                  route.executionMode === kept.executionMode,
              );
            return [[node.nodeId, same ?? node.routes[0]!]];
          }),
        ),
      );
    },
  });
  const prepare = useMutation({ mutationFn: recipesApi.prepare });
  const runtimePreview =
    preview.variables && requestScope(preview.variables) === scopeKey
      ? preview.data
      : undefined;

  const canPreview =
    walletReady &&
    Boolean(runtimeContext) &&
    validation.data?.valid === true &&
    !execution.busy &&
    !execution.completed;

  function refreshPreview(): void {
    if (!canPreview || !runtimeContext) return;
    preview.mutate({ recipe, context: runtimeContext });
  }

  useEffect(() => {
    setSelectedRoutes({});
    preview.reset();
  }, [scopeKey]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (!autoPreview || !canPreview) return;
    const timer = window.setTimeout(refreshPreview, 600);
    return () => window.clearTimeout(timer);
  }, [autoPreview, canPreview, scopeKey]); // eslint-disable-line react-hooks/exhaustive-deps

  const refreshedRun = useRef<string>();
  useEffect(() => {
    if (
      !execution.completed ||
      !execution.run?.id ||
      refreshedRun.current === execution.run.id
    )
      return;
    refreshedRun.current = execution.run.id;
    void Promise.all([borrowing.refetch(), refetchBalances()]);
  }, [borrowing, execution.completed, execution.run?.id, refetchBalances]);

  async function prepareNext(): Promise<FreshPreparation> {
    if (!runtimeContext)
      throw new Error("Connect a wallet on BNB Chain first.");
    // Swap calldata and RFQ quotes expire in seconds, so every signature starts from a fresh quote.
    const reviewed = routesRef.current;
    const request: RecipePreviewRequest = { recipe, context: runtimeContext };
    const fresh = await preview.mutateAsync(request);
    const routes: Record<string, TradeQuote> = {};
    for (const node of fresh.nodes) {
      if (
        node.kind !== "TRADE" ||
        node.state !== "READY" ||
        !node.routes?.length
      )
        continue;
      const previous = reviewed[node.nodeId];
      const route = previous
        ? node.routes.find(
            (candidate) =>
              candidate.venue === previous.venue &&
              candidate.executionMode === previous.executionMode,
          )
        : node.routes[0];
      if (!route)
        throw new Error(
          "The route you reviewed is no longer available. Review the new routes in the builder.",
        );
      routes[node.nodeId] = route;
    }
    const prepared = await prepare.mutateAsync({
      recipe,
      context: runtimeContext,
      selections: Object.entries(routes).map(([nodeId, route]) => ({
        kind: "TRADE" as const,
        nodeId,
        routeQuoteId: route.routeQuoteId,
        venue: route.venue,
      })),
    });
    return { prepared, routes, reviewed, preview: fresh };
  }

  function loadDefinition(definition: RecipeDefinition, name: string): void {
    editor.loadGraph(graphFromDefinition(definition, assets), name);
  }

  function startFromPosition(
    source: "SUPPLIED_POSITION" | "BORROWED_POSITION",
    position: { venue: "venus"; pool: Address; token: Address },
    action: "BORROW" | "WITHDRAW" | "REPAY",
  ): void {
    const symbol =
      assets.find(
        (asset) => asset.address.toLowerCase() === position.token.toLowerCase(),
      )?.symbol ?? "position";
    const actionData: RecipeNodeData =
      action === "BORROW"
        ? {
            kind: "BORROW",
            debtToken: catalog.data?.borrowPolicy.debtTokens[0],
            amount: "1",
          }
        : { kind: action };
    editor.loadGraph(
      linearGraph([
        { data: { kind: source, ...position } },
        { data: actionData },
      ]),
      action === "BORROW"
        ? `Borrow against ${symbol}`
        : action === "REPAY"
          ? `Repay ${symbol} debt`
          : `Withdraw ${symbol}`,
    );
  }

  function startBasketSale(position: BasketPosition): void {
    const settlement = catalog.data?.settlementTokens[0];
    if (!settlement) return;
    editor.loadGraph(
      linearGraph([
        {
          data: {
            kind: "WALLET_ASSET",
            token: position.contract,
            amount: position.tokenAmount,
          },
        },
        { data: { kind: "TRADE", buyToken: settlement } },
      ]),
      `Sell ${position.symbol}`,
    );
  }

  function startEarnWithdrawal(position: CarryPosition): void {
    const investmentId = position.investmentIds[0];
    if (!investmentId) return;
    editor.loadGraph(
      linearGraph([
        { data: { kind: "EARN_POSITION", investmentId } },
        { data: { kind: "EARN_WITHDRAW" } },
      ]),
      `Withdraw from ${position.protocolName}`,
    );
  }

  const value: RecipeSessionValue = {
    catalog: catalog.data,
    catalogError:
      catalog.error instanceof Error ? catalog.error.message : undefined,
    retryCatalog: () => void catalog.refetch(),
    assets,
    refetchBalances,
    borrowing: borrowing.data,
    refetchBorrowing: borrowing.refetch,
    suppliedPositions,
    borrowedPositions,
    defaults,
    walletReady,
    wallet: account.address,
    recipe,
    ordered,
    diagnostics,
    valid: validation.data?.valid === true,
    validating: validation.isFetching,
    preview: runtimePreview,
    previewing: preview.isPending,
    previewError:
      preview.error instanceof Error ? preview.error.message : undefined,
    refreshPreview,
    selectedRoutes,
    selectRoute: (nodeId, route) =>
      setSelectedRoutes((current) => ({ ...current, [nodeId]: route })),
    runtimeContext,
    prepareNext,
    execution,
    loadDefinition,
    startFromPosition,
    basketPositions: basketHoldings.data?.positions ?? [],
    startBasketSale,
    earnPositions: earn.data?.positions ?? [],
    refetchEarnPositions: () => void earn.refetch(),
    startEarnWithdrawal,
  };
  return (
    <RecipeSessionContext.Provider value={value}>
      {children}
    </RecipeSessionContext.Provider>
  );
}
