import { blockIcons, blockTints, type BlockKind } from "./blocks";

export function BlockIcon({
  kind,
  size = 42,
  radius,
  tint,
}: {
  kind: BlockKind;
  size?: number;
  radius?: number;
  tint?: string;
}) {
  return (
    <span
      aria-hidden="true"
      style={{
        flex: "none",
        width: size,
        height: size,
        borderRadius: radius ?? Math.round(size * 0.29),
        background: tint ?? blockTints[kind],
        border: "1px solid rgba(15,17,14,.08)",
        display: "grid",
        placeItems: "center",
      }}
    >
      <svg
        width={Math.round(size * 0.45)}
        height={Math.round(size * 0.45)}
        viewBox="0 0 24 24"
        fill="none"
        stroke="#0F110E"
        strokeWidth="1.8"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d={blockIcons[kind]} />
      </svg>
    </span>
  );
}

export function Logo({ size = 26 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 30 30"
      fill="none"
      aria-hidden="true"
    >
      <rect x="2" y="2" width="12" height="12" rx="3.5" fill="#0F110E" />
      <rect
        x="16"
        y="16"
        width="12"
        height="12"
        rx="3.5"
        fill="#D5F56B"
        stroke="#0F110E"
        strokeWidth="1.6"
      />
      <path
        d="M14 8h4.5a3.5 3.5 0 0 1 3.5 3.5V16"
        stroke="#0F110E"
        strokeWidth="1.6"
        strokeLinecap="round"
      />
    </svg>
  );
}

export function Sparkle({
  size = 20,
  stroke = "#0F110E",
}: {
  size?: number;
  stroke?: string;
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={stroke}
      strokeWidth="1.8"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z" />
    </svg>
  );
}

export function ArrowUpRight({
  size = 16,
  stroke = "#0F110E",
}: {
  size?: number;
  stroke?: string;
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={stroke}
      strokeWidth="2.2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M7 17L17 7M8 7h9v9" />
    </svg>
  );
}

export function Check({
  size = 12,
  stroke = "#D5F56B",
}: {
  size?: number;
  stroke?: string;
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={stroke}
      strokeWidth="3"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M5 12.5l4.5 4.5L19 7" />
    </svg>
  );
}
