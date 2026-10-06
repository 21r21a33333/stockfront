import { parseUnits } from "viem";
import type {
  RecipeDefinition,
  RecipeDiagnostic,
  RecipeEdge,
  RecipeNode,
} from "@/api/generated";
import type { AssetOption, RecipeFlowEdge, RecipeFlowNode } from "./types";

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
    if (node.data.kind === "WALLET_ASSET" && node.data.token) {
      const asset = assetByAddress.get(node.data.token.toLowerCase());
      result.push({
        id: node.id,
        kind: node.data.kind,
        token: node.data.token,
        rawAmount: parseAmount(node.data.amount, asset?.decimals ?? 18),
      });
    } else if (node.data.kind === "TRADE" && node.data.buyToken) {
      result.push({
        id: node.id,
        kind: node.data.kind,
        buyToken: node.data.buyToken,
      });
    } else if (node.data.kind === "SUPPLY") {
      result.push({
        id: node.id,
        kind: node.data.kind,
        venue: node.data.venue,
      });
    }
    return result;
  }, []);
  return {
    schemaVersion,
    chainId: 56,
    nodes: recipeNodes,
    edges: edges.map(toRecipeEdge),
  };
}

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
