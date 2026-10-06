import {
  addEdge,
  applyEdgeChanges,
  applyNodeChanges,
  type Connection,
  type EdgeChange,
  type NodeChange,
  type XYPosition,
} from "@xyflow/react";
import { create } from "zustand";
import type { Address } from "viem";
import type { RecipePrimitiveKind } from "@/api/generated";
import type {
  DraftRecipe,
  RecipeFlowEdge,
  RecipeFlowNode,
  RecipeNodeData,
} from "./types";
import { flowType } from "./types";

const DRAFT_KEY = "stockfront.recipe-builder.draft.v1";

export interface EditorDefaults {
  settlementToken?: Address;
  bstockToken?: Address;
  suppliedPosition?: {
    venue: "venus";
    pool: Address;
    token: Address;
  };
}

interface RecipeEditorState {
  nodes: RecipeFlowNode[];
  edges: RecipeFlowEdge[];
  selectedNodeId?: string;
  mode: "builder" | "debug";
  initialized: boolean;
  initialize(defaults: EditorDefaults): void;
  addNode(
    kind: RecipePrimitiveKind,
    position: XYPosition,
    defaults: EditorDefaults,
  ): void;
  appendTrade(sourceId: string, buyToken?: Address): void;
  appendSupply(sourceId: string): void;
  appendWithdraw(sourceId: string): void;
  updateNode(id: string, patch: Partial<RecipeNodeData>): void;
  selectNode(id?: string): void;
  removeSelected(): void;
  connect(connection: Connection): void;
  onNodesChange(changes: NodeChange<RecipeFlowNode>[]): void;
  onEdgesChange(changes: EdgeChange<RecipeFlowEdge>[]): void;
  setMode(mode: "builder" | "debug"): void;
  startNewRecipe(): void;
  saveDraft(): void;
  restoreDraft(): boolean;
}

function nodeFor(
  kind: RecipePrimitiveKind,
  position: XYPosition,
  defaults: EditorDefaults,
): RecipeFlowNode {
  const id = crypto.randomUUID();
  switch (kind) {
    case "WALLET_ASSET":
      return {
        id,
        type: flowType(kind),
        position,
        data: { kind, token: defaults.settlementToken, amount: "6" },
      };
    case "TRADE":
      return {
        id,
        type: flowType(kind),
        position,
        data: { kind, buyToken: defaults.bstockToken },
      };
    case "SUPPLY":
      return {
        id,
        type: flowType(kind),
        position,
        data: { kind, venue: "venus" },
      };
    case "SUPPLIED_POSITION":
      return {
        id,
        type: flowType(kind),
        position,
        data: {
          kind,
          venue: defaults.suppliedPosition?.venue ?? "venus",
          pool: defaults.suppliedPosition?.pool,
          token: defaults.suppliedPosition?.token,
        },
      };
    case "WITHDRAW":
      return {
        id,
        type: flowType(kind),
        position,
        data: { kind },
      };
  }
}

function initialGraph(defaults: EditorDefaults): {
  nodes: RecipeFlowNode[];
  edges: RecipeFlowEdge[];
} {
  const wallet = nodeFor("WALLET_ASSET", { x: 70, y: 180 }, defaults);
  const trade = nodeFor("TRADE", { x: 385, y: 168 }, defaults);
  return {
    nodes: [wallet, trade],
    edges: [
      {
        id: crypto.randomUUID(),
        source: wallet.id,
        sourceHandle: "asset",
        target: trade.id,
        targetHandle: "sell",
        type: "asset",
      },
    ],
  };
}

export const useRecipeEditor = create<RecipeEditorState>((set, get) => ({
  nodes: [],
  edges: [],
  mode: "builder",
  initialized: false,
  initialize: (defaults) => {
    if (get().initialized) return;
    if (get().restoreDraft()) return;
    const graph = initialGraph(defaults);
    set({ ...graph, selectedNodeId: graph.nodes[1]?.id, initialized: true });
  },
  addNode: (kind, position, defaults) => {
    const node = nodeFor(kind, position, defaults);
    set((state) => ({
      nodes: [...state.nodes, node],
      selectedNodeId: node.id,
    }));
  },
  appendTrade: (sourceId, buyToken) =>
    set((state) => {
      const source = state.nodes.find((node) => node.id === sourceId);
      if (
        source?.data.kind !== "TRADE" ||
        state.edges.some(
          (edge) => edge.source === sourceId && edge.sourceHandle === "buy",
        )
      ) {
        return state;
      }
      const node: RecipeFlowNode = {
        id: crypto.randomUUID(),
        type: "trade",
        position: { x: source.position.x + 300, y: source.position.y },
        data: { kind: "TRADE", buyToken },
      };
      return {
        nodes: [...state.nodes, node],
        edges: [
          ...state.edges,
          {
            id: crypto.randomUUID(),
            source: sourceId,
            sourceHandle: "buy",
            target: node.id,
            targetHandle: "sell",
            type: "asset",
          },
        ],
        selectedNodeId: node.id,
      };
    }),
  appendSupply: (sourceId) =>
    set((state) => {
      const source = state.nodes.find((node) => node.id === sourceId);
      if (
        source?.data.kind !== "TRADE" ||
        state.edges.some(
          (edge) => edge.source === sourceId && edge.sourceHandle === "buy",
        )
      ) {
        return state;
      }
      const node: RecipeFlowNode = {
        id: crypto.randomUUID(),
        type: "supply",
        position: { x: source.position.x + 300, y: source.position.y },
        data: { kind: "SUPPLY", venue: "venus" },
      };
      return {
        nodes: [...state.nodes, node],
        edges: [
          ...state.edges,
          {
            id: crypto.randomUUID(),
            source: sourceId,
            sourceHandle: "buy",
            target: node.id,
            targetHandle: "asset",
            type: "asset",
          },
        ],
        selectedNodeId: node.id,
      };
    }),
  appendWithdraw: (sourceId) =>
    set((state) => {
      const source = state.nodes.find((node) => node.id === sourceId);
      if (
        source?.data.kind !== "SUPPLIED_POSITION" ||
        state.edges.some(
          (edge) =>
            edge.source === sourceId && edge.sourceHandle === "position",
        )
      ) {
        return state;
      }
      const node: RecipeFlowNode = {
        id: crypto.randomUUID(),
        type: "withdraw",
        position: { x: source.position.x + 300, y: source.position.y },
        data: { kind: "WITHDRAW" },
      };
      return {
        nodes: [...state.nodes, node],
        edges: [
          ...state.edges,
          {
            id: crypto.randomUUID(),
            source: sourceId,
            sourceHandle: "position",
            target: node.id,
            targetHandle: "position",
            type: "asset",
          },
        ],
        selectedNodeId: node.id,
      };
    }),
  updateNode: (id, patch) =>
    set((state) => ({
      nodes: state.nodes.map((node) =>
        node.id === id
          ? { ...node, data: { ...node.data, ...patch } as RecipeNodeData }
          : node,
      ),
    })),
  selectNode: (selectedNodeId) => set({ selectedNodeId }),
  removeSelected: () => {
    const selected = get().selectedNodeId;
    if (!selected) return;
    set((state) => ({
      nodes: state.nodes.filter((node) => node.id !== selected),
      edges: state.edges.filter(
        (edge) => edge.source !== selected && edge.target !== selected,
      ),
      selectedNodeId: undefined,
    }));
  },
  connect: (connection) =>
    set((state) => ({
      edges: addEdge(
        { ...connection, id: crypto.randomUUID(), type: "asset" },
        state.edges,
      ),
    })),
  onNodesChange: (changes) =>
    set((state) => ({ nodes: applyNodeChanges(changes, state.nodes) })),
  onEdgesChange: (changes) =>
    set((state) => ({ edges: applyEdgeChanges(changes, state.edges) })),
  setMode: (mode) => set({ mode }),
  startNewRecipe: () => {
    localStorage.removeItem(DRAFT_KEY);
    set({
      nodes: [],
      edges: [],
      selectedNodeId: undefined,
      mode: "builder",
      initialized: true,
    });
  },
  saveDraft: () => {
    const { nodes, edges } = get();
    const draft: DraftRecipe = { version: 1, nodes, edges };
    localStorage.setItem(DRAFT_KEY, JSON.stringify(draft));
  },
  restoreDraft: () => {
    const value = localStorage.getItem(DRAFT_KEY);
    if (!value) return false;
    try {
      const draft = JSON.parse(value) as DraftRecipe;
      if (draft.version !== 1 || !Array.isArray(draft.nodes)) return false;
      set({
        nodes: draft.nodes,
        edges: Array.isArray(draft.edges)
          ? draft.edges.map((edge) => ({ ...edge, type: "asset" }))
          : [],
        selectedNodeId: draft.nodes[0]?.id,
        initialized: true,
      });
      return true;
    } catch {
      localStorage.removeItem(DRAFT_KEY);
      return false;
    }
  },
}));
