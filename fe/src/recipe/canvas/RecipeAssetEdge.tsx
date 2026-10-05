import {
  BaseEdge,
  EdgeLabelRenderer,
  getSmoothStepPath,
  type EdgeProps,
} from "@xyflow/react";
import { useBuilderContext } from "../editor/context";
import { useRecipeEditor } from "../editor/store";
import type { RecipeFlowEdge } from "../editor/types";
import styles from "../RecipeBuilder.module.css";

export function RecipeAssetEdge({
  source,
  sourceX,
  sourceY,
  sourcePosition,
  targetX,
  targetY,
  targetPosition,
  markerEnd,
}: EdgeProps<RecipeFlowEdge>) {
  const { assets } = useBuilderContext();
  const sourceNode = useRecipeEditor((state) =>
    state.nodes.find((node) => node.id === source),
  );
  const token =
    sourceNode?.data.kind === "WALLET_ASSET"
      ? sourceNode.data.token
      : sourceNode?.data.kind === "TRADE"
        ? sourceNode.data.buyToken
        : undefined;
  const symbol = assets.find(
    (asset) => asset.address.toLowerCase() === token?.toLowerCase(),
  )?.symbol;
  const [path, labelX, labelY] = getSmoothStepPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
    borderRadius: 10,
  });
  const label = symbol ?? "Asset";
  return (
    <>
      <BaseEdge
        path={path}
        markerEnd={markerEnd}
        style={{ stroke: "#d29b72", strokeWidth: 1.6 }}
      />
      <EdgeLabelRenderer>
        <span
          className={styles.edgeAssetLabel}
          style={{
            transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY - 17}px)`,
          }}
        >
          Asset&lt;{label}&gt;
        </span>
      </EdgeLabelRenderer>
    </>
  );
}
