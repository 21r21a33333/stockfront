import {
  Background,
  BackgroundVariant,
  Panel,
  ReactFlow,
  type Connection,
  type EdgeTypes,
  type NodeTypes,
  useNodesInitialized,
  useReactFlow,
} from "@xyflow/react";
import { Braces, Maximize2, Minus, PanelRightClose, Plus } from "lucide-react";
import { useCallback, useEffect, type DragEvent } from "react";
import type { RecipePrimitiveKind } from "@/api/generated";
import type { RecipeFlowEdge } from "../editor/types";
import { RecipeAssetEdge } from "./RecipeAssetEdge";
import {
  SuppliedPositionNode,
  SupplyNode,
  TradeNode,
  WalletAssetNode,
  WithdrawNode,
} from "./RecipeNodes";
import { useRecipeEditor, type EditorDefaults } from "../editor/store";
import styles from "../RecipeBuilder.module.css";

const nodeTypes: NodeTypes = {
  walletAsset: WalletAssetNode,
  suppliedPosition: SuppliedPositionNode,
  trade: TradeNode,
  supply: SupplyNode,
  withdraw: WithdrawNode,
};
const edgeTypes: EdgeTypes = { asset: RecipeAssetEdge };

export function RecipeCanvas({
  defaults,
  mode,
  onModeChange,
  onInspectNode,
}: {
  defaults: EditorDefaults;
  mode: "builder" | "debug";
  onModeChange(mode: "builder" | "debug"): void;
  onInspectNode(): void;
}) {
  const editor = useRecipeEditor();
  const flow = useReactFlow();
  const nodesInitialized = useNodesInitialized();
  useEffect(() => {
    if (!nodesInitialized) return;
    void flow
      .fitView({ duration: 0, padding: 0.17, maxZoom: 1.05 })
      .then(() => {
        const viewport = flow.getViewport();
        void flow.setViewport(
          { ...viewport, y: viewport.y - 90 },
          { duration: 0 },
        );
      });
  }, [editor.nodes.length, flow, nodesInitialized]);
  const isValidConnection = useCallback(
    (connection: Connection | RecipeFlowEdge) => {
      if (!connection.source || !connection.target) return false;
      if (connection.source === connection.target) return false;
      if (!connection.targetHandle) return false;
      if (!connection.sourceHandle) return false;
      const source = editor.nodes.find((node) => node.id === connection.source);
      const validPorts =
        connection.targetHandle === "position"
          ? source?.data.kind === "SUPPLIED_POSITION" &&
            connection.sourceHandle === "position"
          : ["sell", "asset"].includes(connection.targetHandle) &&
            ((source?.data.kind === "WALLET_ASSET" &&
              connection.sourceHandle === "asset") ||
              (source?.data.kind === "TRADE" &&
                connection.sourceHandle === "buy"));
      if (!validPorts) return false;
      return !editor.edges.some(
        (edge) =>
          (edge.target === connection.target &&
            edge.targetHandle === connection.targetHandle) ||
          (edge.source === connection.source &&
            edge.sourceHandle === connection.sourceHandle),
      );
    },
    [editor.edges, editor.nodes],
  );
  const onDrop = useCallback(
    (event: DragEvent) => {
      event.preventDefault();
      const kind = event.dataTransfer.getData(
        "application/stockfront-primitive",
      ) as RecipePrimitiveKind;
      if (
        ![
          "WALLET_ASSET",
          "SUPPLIED_POSITION",
          "TRADE",
          "SUPPLY",
          "WITHDRAW",
        ].includes(kind)
      )
        return;
      editor.addNode(
        kind,
        flow.screenToFlowPosition({ x: event.clientX, y: event.clientY }),
        defaults,
      );
    },
    [defaults, editor, flow],
  );

  return (
    <div className={styles.canvas} id="builder">
      <ReactFlow
        nodes={editor.nodes}
        edges={editor.edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodesChange={editor.onNodesChange}
        onEdgesChange={editor.onEdgesChange}
        onConnect={editor.connect}
        isValidConnection={isValidConnection}
        onNodeClick={(_, node) => {
          editor.selectNode(node.id);
          onInspectNode();
        }}
        onPaneClick={() => editor.selectNode(undefined)}
        onDrop={onDrop}
        onDragOver={(event) => {
          event.preventDefault();
          event.dataTransfer.dropEffect = "move";
        }}
        minZoom={0.45}
        maxZoom={1.15}
        defaultEdgeOptions={{
          type: "asset",
          style: { stroke: "#d29b72", strokeWidth: 1.6 },
        }}
        proOptions={{ hideAttribution: true }}
      >
        <Background
          variant={BackgroundVariant.Dots}
          color="#d6d6dc"
          gap={17}
          size={1}
        />
        <Panel position="bottom-center" className={styles.canvasToolbar}>
          <div className={styles.zoomToolbar} aria-label="Canvas zoom">
            <button
              type="button"
              aria-label="Zoom in"
              onClick={() => void flow.zoomIn()}
            >
              <Plus size={17} />
            </button>
            <button
              type="button"
              aria-label="Zoom out"
              onClick={() => void flow.zoomOut()}
            >
              <Minus size={17} />
            </button>
            <button
              type="button"
              aria-label="Fit view"
              onClick={() => void flow.fitView()}
            >
              <Maximize2 size={15} />
            </button>
          </div>
          <div className={styles.modeSwitch}>
            <button
              type="button"
              className={mode === "builder" ? styles.activeMode : ""}
              onClick={() => onModeChange("builder")}
            >
              <PanelRightClose size={13} /> Builder
            </button>
            <button
              type="button"
              className={mode === "debug" ? styles.activeMode : ""}
              onClick={() => onModeChange("debug")}
            >
              <Braces size={13} /> Debug
            </button>
          </div>
        </Panel>
      </ReactFlow>
    </div>
  );
}
