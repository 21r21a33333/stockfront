import { formatUnits } from "viem";
import type {
  RecipeDefinition,
  RecipeNode,
  RecipePrimitiveKind,
} from "@/api/generated";
import type {
  AssetOption,
  RecipeFlowEdge,
  RecipeFlowNode,
  RecipeNodeData,
} from "./types";
import { flowType } from "./types";

/** Nodes in execution order: start from the node nothing feeds and follow its single output. */
export function chainOrder(
  nodes: RecipeFlowNode[],
  edges: RecipeFlowEdge[],
): RecipeFlowNode[] {
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const targets = new Set(edges.map((edge) => edge.target));
  const next = new Map(edges.map((edge) => [edge.source, edge.target]));
  const ordered: RecipeFlowNode[] = [];
  const seen = new Set<string>();
  for (const head of nodes.filter((node) => !targets.has(node.id))) {
    for (
      let current: RecipeFlowNode | undefined = head;
      current && !seen.has(current.id);
    ) {
      seen.add(current.id);
      ordered.push(current);
      const following = next.get(current.id);
      current = following ? byId.get(following) : undefined;
    }
  }
  return [...ordered, ...nodes.filter((node) => !seen.has(node.id))];
}

/** Actions that may consume the output of `last`; an empty chain may only start with a source. */
export function nextKinds(
  last?: RecipeNodeData["kind"],
): RecipePrimitiveKind[] {
  switch (last) {
    case undefined:
      return ["WALLET_ASSET", "SUPPLIED_POSITION", "BORROWED_POSITION"];
    case "WALLET_ASSET":
      return ["TRADE", "SUPPLY"];
    case "TRADE":
      return ["TRADE", "SUPPLY"];
    case "BORROW":
      return ["TRADE"];
    case "SUPPLIED_POSITION":
      return ["BORROW", "WITHDRAW"];
    case "BORROWED_POSITION":
      return ["REPAY"];
    default:
      return [];
  }
}

const outputPort: Partial<Record<RecipeNodeData["kind"], string>> = {
  WALLET_ASSET: "asset",
  TRADE: "buy",
  BORROW: "borrowed",
  SUPPLIED_POSITION: "position",
  BORROWED_POSITION: "debt",
};

const inputPort: Partial<Record<RecipeNodeData["kind"], string>> = {
  TRADE: "sell",
  SUPPLY: "asset",
  BORROW: "collateral",
  WITHDRAW: "position",
  REPAY: "debt",
};

/** A linear chain of node data wired output→input, laid out top to bottom. */
export function linearGraph(items: { id?: string; data: RecipeNodeData }[]): {
  nodes: RecipeFlowNode[];
  edges: RecipeFlowEdge[];
} {
  const nodes = items.map((item, index): RecipeFlowNode => ({
    id: item.id ?? crypto.randomUUID(),
    type: flowType(item.data.kind),
    position: { x: index % 2 === 0 ? 0 : 160, y: index * 128 },
    data: item.data,
  }));
  const edges = nodes.slice(1).flatMap((node, index): RecipeFlowEdge[] => {
    const source = nodes[index]!;
    const sourceHandle = outputPort[source.data.kind];
    const targetHandle = inputPort[node.data.kind];
    return sourceHandle && targetHandle
      ? [
          {
            id: crypto.randomUUID(),
            source: source.id,
            sourceHandle,
            target: node.id,
            targetHandle,
            type: "asset",
          },
        ]
      : [];
  });
  return { nodes, edges };
}

/** Converts a backend recipe into editor nodes; amounts are shown in the token's own units. */
export function graphFromDefinition(
  recipe: RecipeDefinition,
  assets: AssetOption[],
): { nodes: RecipeFlowNode[]; edges: RecipeFlowEdge[] } {
  const graph = linearGraph(
    recipe.nodes.map((node) => ({ id: node.id, data: nodeData(node, assets) })),
  );
  return {
    nodes: graph.nodes,
    edges: recipe.edges.map((edge) => ({
      id: edge.id,
      type: "asset",
      source: edge.from.nodeId,
      sourceHandle: edge.from.port,
      target: edge.to.nodeId,
      targetHandle: edge.to.port,
    })),
  };
}

function nodeData(node: RecipeNode, assets: AssetOption[]): RecipeNodeData {
  switch (node.kind) {
    case "TRADE":
      return { kind: node.kind, buyToken: node.buyToken };
    case "SUPPLY":
      return { kind: node.kind, venue: node.venue };
    case "EARN_DEPOSIT":
      return {
        kind: node.kind,
        collateralToken: node.collateralToken,
        investmentId: node.investmentId,
      };
    case "SUPPLIED_POSITION":
    case "BORROWED_POSITION":
      return {
        kind: node.kind,
        venue: node.venue,
        pool: node.pool,
        token: node.token,
      };
    case "WITHDRAW":
    case "REPAY":
      return { kind: node.kind };
    case "BORROW":
      return {
        kind: node.kind,
        debtToken: node.debtToken,
        amount: formatUnits(
          BigInt(node.rawAmount),
          decimalsFor(node.debtToken, assets),
        ),
      };
    case "WALLET_ASSET":
      return {
        kind: node.kind,
        token: node.token,
        amount: formatUnits(
          BigInt(node.rawAmount),
          decimalsFor(node.token, assets),
        ),
      };
  }
}

function decimalsFor(token: string, assets: AssetOption[]): number {
  const asset = assets.find(
    (item) => item.address.toLowerCase() === token.toLowerCase(),
  );
  if (!asset)
    throw new Error("This strategy uses an asset that is no longer supported.");
  return asset.decimals;
}
