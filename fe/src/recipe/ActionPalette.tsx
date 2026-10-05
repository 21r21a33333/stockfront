import {
  ArrowRightLeft,
  Boxes,
  CircleDollarSign,
  GripVertical,
  Layers3,
  Search,
  ShieldCheck,
  WalletCards,
} from "lucide-react";
import { useMemo, useState } from "react";
import type {
  RecipePrimitiveKind,
  RecipePrimitiveDescriptor,
} from "@/api/generated";
import { WalletButton } from "@/wallet/WalletButton";
import styles from "./RecipeBuilder.module.css";

const upcoming = [
  ["Supply collateral", ShieldCheck],
  ["Borrow", CircleDollarSign],
  ["Earn deposit", Layers3],
  ["Liquidity provision", Boxes],
] as const;

export function ActionPalette({
  primitives,
  onAdd,
  mode,
  onModeChange,
}: {
  primitives: RecipePrimitiveDescriptor[];
  onAdd(kind: RecipePrimitiveKind): void;
  mode: "builder" | "debug";
  onModeChange(mode: "builder" | "debug"): void;
}) {
  const [query, setQuery] = useState("");
  const normalizedQuery = query.trim().toLowerCase();
  const visiblePrimitives = useMemo(
    () =>
      primitives.filter((primitive) =>
        `${primitive.name} ${primitive.description} ${primitive.kind}`
          .toLowerCase()
          .includes(normalizedQuery),
      ),
    [normalizedQuery, primitives],
  );
  const visibleUpcoming = upcoming.filter(([label]) =>
    label.toLowerCase().includes(normalizedQuery),
  );

  return (
    <aside className={styles.sidebar}>
      <div className={styles.brand}>
        <span className={styles.brandMark}>●</span>
        <strong>Recipes</strong>
        <span className={styles.collapseIcon} />
      </div>
      <label className={styles.search}>
        <Search size={13} />
        <input
          placeholder="Search actions…"
          aria-label="Search actions"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
        <kbd>{visiblePrimitives.length + visibleUpcoming.length}</kbd>
      </label>
      <nav className={styles.overview} aria-label="Overview">
        <small>OVERVIEW</small>
        <button
          type="button"
          className={mode === "builder" ? styles.activeNav : undefined}
          onClick={() => onModeChange("builder")}
        >
          <span /> Recipe Builder
        </button>
        <button
          type="button"
          className={mode === "debug" ? styles.activeNav : undefined}
          onClick={() => onModeChange("debug")}
        >
          <span /> Debug
        </button>
      </nav>
      <section className={styles.palette}>
        <header>
          <small>ACTIONS</small>
          <code>recipe-primitives</code>
        </header>
        {visiblePrimitives.map((primitive) => (
          <button
            key={primitive.kind}
            type="button"
            draggable
            onDragStart={(event) => {
              event.dataTransfer.setData(
                "application/stockfront-primitive",
                primitive.kind,
              );
              event.dataTransfer.effectAllowed = "move";
            }}
            onClick={() => onAdd(primitive.kind)}
            className={`${styles.paletteItem} ${primitive.kind === "TRADE" ? styles.tradePalette : styles.walletPalette}`}
            title={primitive.description}
          >
            <GripVertical size={13} />
            <span className={styles.paletteIcon}>
              {primitive.kind === "TRADE" ? (
                <ArrowRightLeft size={12} />
              ) : (
                <WalletCards size={12} />
              )}
            </span>
            <strong>{primitive.name}</strong>
            <small>{primitive.kind === "TRADE" ? "Live" : "Source"}</small>
          </button>
        ))}
        {visibleUpcoming.map(([label, Icon]) => (
          <div className={styles.upcoming} key={label}>
            <Icon size={13} />
            <span>{label}</span>
            <small>Soon</small>
          </div>
        ))}
        {visiblePrimitives.length + visibleUpcoming.length === 0 ? (
          <p className={styles.emptyPalette}>No matching actions</p>
        ) : null}
      </section>
      <div className={styles.sidebarBottom}>
        <WalletButton />
      </div>
    </aside>
  );
}
