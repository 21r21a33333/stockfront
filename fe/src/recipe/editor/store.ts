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
import { chainOrder, linearGraph, nextKinds } from "./graphs";

type Position = RecipeFlowNode["position"];

const DRAFT_KEY = "stockfront.recipe-builder.draft.v1";

export interface EditorDefaults {
  settlementToken?: Address;
  debtToken?: Address;
  bstockToken?: Address;
  suppliedPosition?: {
    venue: "venus";
    pool: Address;
    token: Address;
  };
  borrowedPosition?: {
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
    position: Position,
    defaults: EditorDefaults,
  ): void;
  appendTrade(sourceId: string, buyToken?: Address): void;
  appendSupply(sourceId: string): void;
  appendWithdraw(sourceId: string): void;
  appendBorrow(sourceId: string, debtToken?: Address): void;
  appendRepay(sourceId: string): void;
  updateNode(id: string, patch: Partial<RecipeNodeData>): void;
  selectNode(id?: string): void;
  removeSelected(): void;
  setMode(mode: "builder" | "debug"): void;
  startNewRecipe(): void;
  startWithdrawalRecipe(
    position: NonNullable<EditorDefaults["suppliedPosition"]>,
  ): void;
  saveDraft(): void;
  restoreDraft(): boolean;
  name: string;
  setName(name: string): void;
  loadGraph(
    graph: { nodes: RecipeFlowNode[]; edges: RecipeFlowEdge[] },
    name: string,
  ): void;
  /** Adds a block to the end of the chain when the compiler accepts it there. */
  appendToChain(kind: RecipePrimitiveKind, defaults: EditorDefaults): boolean;
  /** Removes a block and everything after it, since later blocks consume its output. */
  removeFrom(id: string): void;
}

function nodeFor(
  kind: RecipePrimitiveKind,
  position: Position,
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
    case "BORROWED_POSITION":
      return {
        id,
        type: flowType(kind),
        position,
        data: {
          kind,
          venue: defaults.borrowedPosition?.venue ?? "venus",
          pool: defaults.borrowedPosition?.pool,
          token: defaults.borrowedPosition?.token,
        },
      };
    case "WITHDRAW":
      return {
        id,
        type: flowType(kind),
        position,
        data: { kind },
      };
    case "BORROW":
      return {
        id,
        type: flowType(kind),
        position,
        data: {
          kind,
          debtToken: defaults.debtToken ?? defaults.settlementToken,
          amount: "1",
        },
      };
    case "REPAY":
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
  name: "Untitled strategy",
  setName: (name) => set({ name }),
  loadGraph: (graph, name) => {
    localStorage.removeItem(DRAFT_KEY);
    set({
      nodes: graph.nodes,
      edges: graph.edges,
      name,
      selectedNodeId: graph.nodes.at(-1)?.id,
      mode: "builder",
      initialized: true,
    });
  },
  appendToChain: (kind, defaults) => {
    const { nodes, edges } = get();
    const ordered = chainOrder(nodes, edges);
    const last = ordered.at(-1);
    if (!nextKinds(last?.data.kind).includes(kind)) return false;
    const node = nodeFor(kind, { x: 0, y: 0 }, defaults);
    const graph = linearGraph([
      ...ordered.map((item) => ({ id: item.id, data: item.data })),
      { id: node.id, data: node.data },
    ]);
    set({ nodes: graph.nodes, edges: graph.edges, selectedNodeId: node.id });
    return true;
  },
  removeFrom: (id) => {
    const ordered = chainOrder(get().nodes, get().edges);
    const index = ordered.findIndex((node) => node.id === id);
    if (index < 0) return;
    const kept = ordered.slice(0, index);
    const graph = linearGraph(
      kept.map((item) => ({ id: item.id, data: item.data })),
    );
    set({ ...graph, selectedNodeId: kept.at(-1)?.id });
  },
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
      const sourceHandle =
        source?.data.kind === "TRADE"
          ? "buy"
          : source?.data.kind === "BORROW"
            ? "borrowed"
            : undefined;
      if (
        !source ||
        !sourceHandle ||
        state.edges.some(
          (edge) =>
            edge.source === sourceId && edge.sourceHandle === sourceHandle,
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
            sourceHandle,
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
  appendBorrow: (sourceId, debtToken) =>
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
        type: "borrow",
        position: { x: source.position.x + 300, y: source.position.y },
        data: { kind: "BORROW", debtToken, amount: "1" },
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
            targetHandle: "collateral",
            type: "asset",
          },
        ],
        selectedNodeId: node.id,
      };
    }),
  appendRepay: (sourceId) =>
    set((state) => {
      const source = state.nodes.find((node) => node.id === sourceId);
      if (
        source?.data.kind !== "BORROWED_POSITION" ||
        state.edges.some(
          (edge) => edge.source === sourceId && edge.sourceHandle === "debt",
        )
      ) {
        return state;
      }
      const node: RecipeFlowNode = {
        id: crypto.randomUUID(),
        type: "repay",
        position: { x: source.position.x + 300, y: source.position.y },
        data: { kind: "REPAY" },
      };
      return {
        nodes: [...state.nodes, node],
        edges: [
          ...state.edges,
          {
            id: crypto.randomUUID(),
            source: sourceId,
            sourceHandle: "debt",
            target: node.id,
            targetHandle: "debt",
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
  setMode: (mode) => set({ mode }),
  startNewRecipe: () => {
    localStorage.removeItem(DRAFT_KEY);
    set({
      name: "Untitled strategy",
      nodes: [],
      edges: [],
      selectedNodeId: undefined,
      mode: "builder",
      initialized: true,
    });
  },
  startWithdrawalRecipe: (position) => {
    localStorage.removeItem(DRAFT_KEY);
    const source = nodeFor(
      "SUPPLIED_POSITION",
      { x: 70, y: 180 },
      { suppliedPosition: position },
    );
    const withdrawal = nodeFor(
      "WITHDRAW",
      { x: 370, y: 180 },
      { suppliedPosition: position },
    );
    set({
      nodes: [source, withdrawal],
      edges: [
        {
          id: crypto.randomUUID(),
          source: source.id,
          sourceHandle: "position",
          target: withdrawal.id,
          targetHandle: "position",
          type: "asset",
        },
      ],
      selectedNodeId: withdrawal.id,
      mode: "builder",
      initialized: true,
    });
  },
  saveDraft: () => {
    const { nodes, edges, name } = get();
    const draft: DraftRecipe = { version: 1, nodes, edges, name };
    localStorage.setItem(DRAFT_KEY, JSON.stringify(draft));
  },
  restoreDraft: () => {
    const value = localStorage.getItem(DRAFT_KEY);
    if (!value) return false;
    try {
      const draft = JSON.parse(value) as DraftRecipe;
      if (draft.version !== 1 || !Array.isArray(draft.nodes)) return false;
      set({
        name: typeof draft.name === "string" ? draft.name : "Untitled strategy",
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
