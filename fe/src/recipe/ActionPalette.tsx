import {
  ArrowRightLeft,
  Boxes,
  CircleDollarSign,
  GripVertical,
  Landmark,
  Layers3,
  Search,
  ShieldCheck,
  HandCoins,
  Undo2,
  WalletCards,
  type LucideIcon,
} from "lucide-react";
import { useMemo, useState } from "react";
import type {
  RecipePrimitiveKind,
  RecipePrimitiveDescriptor,
} from "@/api/generated";
import { WalletButton } from "@/wallet/WalletButton";
import styles from "./RecipeBuilder.module.css";

const upcoming = [
  ["Earn deposit", Layers3],
  ["Liquidity provision", Boxes],
] as const;

const primitivePresentation: Record<
  RecipePrimitiveKind,
  { className: string; badge: "Source" | "Live"; Icon: LucideIcon }
> = {
  WALLET_ASSET: {
    className: styles.walletPalette,
    badge: "Source",
    Icon: WalletCards,
  },
  SUPPLIED_POSITION: {
    className: styles.positionPalette,
    badge: "Source",
    Icon: Landmark,
  },
  BORROWED_POSITION: {
    className: styles.positionPalette,
    badge: "Source",
    Icon: CircleDollarSign,
  },
  TRADE: {
    className: styles.tradePalette,
    badge: "Live",
    Icon: ArrowRightLeft,
  },
  SUPPLY: {
    className: styles.supplyPalette,
    badge: "Live",
    Icon: ShieldCheck,
  },
  WITHDRAW: {
    className: styles.positionPalette,
    badge: "Live",
    Icon: Undo2,
  },
  BORROW: {
    className: styles.borrowPalette,
    badge: "Live",
    Icon: CircleDollarSign,
  },
  REPAY: {
    className: styles.borrowPalette,
    badge: "Live",
    Icon: HandCoins,
  },
};

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
        {visiblePrimitives.map((primitive) => {
          const { className, badge, Icon } =
            primitivePresentation[primitive.kind];
          return (
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
              className={`${styles.paletteItem} ${className}`}
              title={primitive.description}
            >
              <GripVertical size={13} />
              <span className={styles.paletteIcon}>
                <Icon size={12} />
              </span>
              <strong>{primitive.name}</strong>
              <small>{badge}</small>
            </button>
          );
        })}
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
