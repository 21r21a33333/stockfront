import { parseUnits } from "viem";
import type {
  RecipeDefinition,
  RecipeDiagnostic,
  RecipeEdge,
  RecipeNode,
} from "@/api/generated";
import type {
  AssetOption,
  RecipeFlowEdge,
  RecipeFlowNode,
  RecipeNodeData,
} from "./types";

export function buildRecipe(
  schemaVersion: number,
  nodes: RecipeFlowNode[],
  edges: RecipeFlowEdge[],
  assets: AssetOption[],
): RecipeDefinition {
  const assetByAddress = new Map(
    assets.map((asset) => [asset.address.toLowerCase(), asset]),
  );
  const recipeNodes = nodes.reduce<RecipeNode[]>((result, node) => {
    // The map is keyed by the same union the node carries; TypeScript cannot
    // correlate the two sides of an index like this, so the lookup is widened.
    const build = PRIMITIVES[node.data.kind] as (
      data: RecipeNodeData,
      assets: Assets,
    ) => Primitive | undefined;
    const primitive = build(node.data, assetByAddress);
    // A node still being filled in is left out rather than sent half-formed;
    // validation reports the missing field against the editor's own node.
    if (primitive) result.push({ id: node.id, ...primitive } as RecipeNode);
    return result;
  }, []);
  return {
    schemaVersion,
    chainId: 56,
    nodes: recipeNodes,
    edges: edges.map(toRecipeEdge),
  };
}

type Assets = Map<string, AssetOption>;
type Primitive = Omit<RecipeNode, "id">;

/**
 * How each editor node becomes a recipe primitive, or `undefined` while a
 * required field is still empty. Every kind answers, so a primitive added to
 * the language cannot be silently dropped on its way to the backend.
 */
const PRIMITIVES: {
  [K in RecipeNodeData["kind"]]: (
    data: Extract<RecipeNodeData, { kind: K }>,
    assets: Assets,
  ) => Primitive | undefined;
} = {
  WALLET_ASSET: (data, assets) =>
    !data.token
      ? undefined
      : {
          kind: data.kind,
          token: data.token,
          rawAmount: parseAmount(
            data.amount,
            assets.get(data.token.toLowerCase())?.decimals ?? 18,
          ),
        },
  TRADE: (data) =>
    data.buyToken ? { kind: data.kind, buyToken: data.buyToken } : undefined,
  SUPPLY: (data) => ({ kind: data.kind, venue: data.venue }),
  EARN_DEPOSIT: (data) =>
    data.collateralToken && data.investmentId
      ? {
          kind: data.kind,
          collateralToken: data.collateralToken,
          investmentId: data.investmentId,
        }
      : undefined,
  EARN_POSITION: (data) =>
    data.investmentId
      ? { kind: data.kind, investmentId: data.investmentId }
      : undefined,
  EARN_WITHDRAW: (data) => ({ kind: data.kind }),
  SUPPLIED_POSITION: (data) =>
    data.pool && data.token
      ? {
          kind: data.kind,
          venue: data.venue,
          pool: data.pool,
          token: data.token,
        }
      : undefined,
  BORROWED_POSITION: (data) =>
    data.pool && data.token
      ? {
          kind: data.kind,
          venue: data.venue,
          pool: data.pool,
          token: data.token,
        }
      : undefined,
  WITHDRAW: (data) => ({ kind: data.kind }),
  BORROW: (data, assets) =>
    data.debtToken
      ? {
          kind: data.kind,
          debtToken: data.debtToken,
          // Left blank, the amount is omitted and Venus sizes the safe
          // maximum when the loan is actually prepared.
          ...(data.amount.trim()
            ? {
                rawAmount: parseAmount(
                  data.amount,
                  assets.get(data.debtToken.toLowerCase())?.decimals ?? 18,
                ),
              }
            : {}),
        }
      : undefined,
  REPAY: (data) => ({ kind: data.kind }),
};

function parseAmount(value: string, decimals: number): string {
  try {
    return parseUnits(value.trim(), decimals).toString();
  } catch {
    return "0";
  }
}

function toRecipeEdge(edge: RecipeFlowEdge): RecipeEdge {
  return {
    id: edge.id,
    from: { nodeId: edge.source, port: edge.sourceHandle ?? "asset" },
    to: { nodeId: edge.target, port: edge.targetHandle ?? "sell" },
  };
}

export function diagnosticsForNode(
  diagnostics: RecipeDiagnostic[],
  nodeId: string,
): RecipeDiagnostic[] {
  return diagnostics.filter((diagnostic) => {
    const location = diagnostic.location;
    return "nodeId" in location && location.nodeId === nodeId;
  });
}

export function diagnosticsForEdge(
  diagnostics: RecipeDiagnostic[],
  edgeId: string,
): RecipeDiagnostic[] {
  return diagnostics.filter(
    (diagnostic) =>
      diagnostic.location.kind === "EDGE" &&
      diagnostic.location.edgeId === edgeId,
  );
}
