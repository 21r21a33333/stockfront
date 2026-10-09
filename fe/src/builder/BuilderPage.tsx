import { useMemo, useState } from "react";
import type { RecipePrimitiveKind } from "@/api/generated";
import { AppHeader } from "@/app/AppHeader";
import { navigate } from "@/app/route";
import { nextKinds } from "@/recipe/editor/graphs";
import { useRecipeEditor } from "@/recipe/editor/store";
import { useRecipeSession } from "@/session/context";
import { BlockIcon, Sparkle } from "@/ui/BlockIcon";
import { blockKind, type BlockKind } from "@/ui/blocks";
import { describe, type DescribeContext } from "./describe";
import { Inspector } from "./Inspector";
import styles from "./BuilderPage.module.css";

const palette: {
  title: string;
  items: { kind: BlockKind; name: string; primitive?: RecipePrimitiveKind }[];
}[] = [
  {
    title: "Start from",
    items: [
      { kind: "wallet", name: "Wallet asset", primitive: "WALLET_ASSET" },
      {
        kind: "supplied",
        name: "Supplied stock",
        primitive: "SUPPLIED_POSITION",
      },
      { kind: "debt", name: "Debt", primitive: "BORROWED_POSITION" },
    ],
  },
  {
    title: "Actions",
    items: [
      { kind: "trade", name: "Trade", primitive: "TRADE" },
      { kind: "supply", name: "Supply", primitive: "SUPPLY" },
      { kind: "borrow", name: "Borrow", primitive: "BORROW" },
      { kind: "repay", name: "Repay", primitive: "REPAY" },
      { kind: "withdraw", name: "Withdraw", primitive: "WITHDRAW" },
      { kind: "earn", name: "Earn" },
      { kind: "liquidity", name: "Liquidity" },
    ],
  },
];

export function BuilderPage() {
  const session = useRecipeSession();
  const editor = useRecipeEditor();
  const [paletteOpen, setPaletteOpen] = useState(true);
  const [zoom, setZoom] = useState(1);
  const [debug, setDebug] = useState(false);

  const ctx: DescribeContext = useMemo(
    () => ({
      ordered: session.ordered,
      assets: session.assets,
      suppliedPositions: session.suppliedPositions,
      borrowedPositions: session.borrowedPositions,
      previews: session.preview?.nodes ?? [],
      routes: session.selectedRoutes,
      diagnostics: session.diagnostics,
      walletReady: session.walletReady,
      minimumHealthFactor:
        session.catalog?.borrowPolicy.minimumHealthFactor ?? "1.5",
    }),
    [session],
  );
  const descriptions = session.ordered.map((_, index) => describe(ctx, index));
  const allowed = nextKinds(session.ordered.at(-1)?.data.kind);
  const hasActions = session.ordered.length > 1;
  const blocked = descriptions.some((item) => item.bad);
  const runnable =
    session.valid && session.walletReady && hasActions && !blocked;
  const pill =
    session.ordered.length === 0
      ? "Draft"
      : runnable || (session.valid && !blocked)
        ? "Ready"
        : "Needs attention";
  const selectedIndex = session.ordered.findIndex(
    (node) => node.id === editor.selectedNodeId,
  );

  function add(kind: RecipePrimitiveKind) {
    editor.appendToChain(kind, session.defaults);
  }

  if (!session.catalog) {
    return (
      <div className={styles.page}>
        <AppHeader current="builder" />
        <div className={styles.loading}>
          {session.catalogError ? (
            <>
              <strong>{session.catalogError}</strong>
              <button
                type="button"
                className={`lift ${styles.secondary}`}
                onClick={session.retryCatalog}
              >
                Try again
              </button>
            </>
          ) : (
            <>
              <span className={`spinner ${styles.loadingSpin}`} />
              <span className="mono">Loading blocks…</span>
            </>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className={styles.page}>
      <AppHeader
        current="builder"
        compactPositions
        middle={
          <div className={styles.titleArea}>
            <label htmlFor="strategyName" className="srOnly">
              Strategy name
            </label>
            <input
              id="strategyName"
              className={styles.name}
              value={editor.name}
              onChange={(event) => editor.setName(event.target.value)}
              onBlur={() => editor.saveDraft()}
            />
            <span
              className={`mono ${styles.pill}`}
              style={{
                background:
                  pill === "Ready"
                    ? "var(--lime)"
                    : pill === "Draft"
                      ? "var(--chip)"
                      : "var(--bad-soft)",
              }}
            >
              <span />
              {pill}
            </span>
          </div>
        }
        actions={
          <>
            <a href="#/compose" className={`lift ${styles.askAi}`}>
              <Sparkle size={15} />
              Ask AI
            </a>
            <button
              type="button"
              className={`lift ${styles.secondary}`}
              onClick={session.refreshPreview}
              disabled={
                !session.walletReady || !session.valid || session.previewing
              }
            >
              {session.previewing ? "Previewing…" : "Preview"}
            </button>
            <button
              type="button"
              className={`lift ${styles.run}`}
              disabled={!runnable}
              title={
                !session.walletReady
                  ? "Connect a wallet on BNB Chain to run"
                  : !hasActions
                    ? "Add an action block"
                    : undefined
              }
              onClick={() => {
                editor.saveDraft();
                navigate("run");
              }}
            >
              Run
              <svg
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="#D5F56B"
                aria-hidden="true"
              >
                <path d="M6 4l14 8-14 8z" />
              </svg>
            </button>
          </>
        }
      />

      <div className={styles.body}>
        <aside
          aria-label="Block palette"
          className={styles.palette}
          style={{ width: paletteOpen ? 232 : 60 }}
        >
          <div className={styles.paletteHead}>
            {paletteOpen ? (
              <span className={`mono ${styles.paletteTitle}`}>Blocks</span>
            ) : null}
            <button
              type="button"
              className={styles.collapse}
              aria-label={paletteOpen ? "Collapse palette" : "Expand palette"}
              onClick={() => setPaletteOpen((value) => !value)}
            >
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="#0F110E"
                strokeWidth="1.8"
                strokeLinecap="round"
                aria-hidden="true"
              >
                <rect x="3.5" y="4.5" width="17" height="15" rx="3" />
                <path d="M9.5 4.5v15" />
              </svg>
            </button>
          </div>
          <div className={styles.paletteBody}>
            {palette.map((group) => (
              <div key={group.title} className={styles.paletteGroup}>
                {paletteOpen ? (
                  <span className={`mono ${styles.groupTitle}`}>
                    {group.title}
                  </span>
                ) : null}
                {group.items.map((item) => {
                  const soon = !item.primitive;
                  const enabled = Boolean(
                    item.primitive && allowed.includes(item.primitive),
                  );
                  return (
                    <button
                      type="button"
                      key={item.name}
                      className={styles.paletteItem}
                      title={
                        soon
                          ? `${item.name} · coming soon`
                          : enabled
                            ? `Add ${item.name}`
                            : `${item.name} can't follow the last block`
                      }
                      disabled={!enabled}
                      onClick={() => item.primitive && add(item.primitive)}
                      style={{ opacity: enabled ? 1 : 0.45 }}
                    >
                      <BlockIcon kind={item.kind} size={30} radius={9} />
                      {paletteOpen ? (
                        <>
                          <span className={styles.paletteName}>
                            {item.name}
                          </span>
                          <span className={`mono ${styles.paletteTag}`}>
                            {soon ? "Soon" : ""}
                          </span>
                        </>
                      ) : null}
                    </button>
                  );
                })}
              </div>
            ))}
          </div>
          {paletteOpen ? (
            <p className={`mono ${styles.paletteHint}`}>
              Click a block to add it to the end of the strategy.
            </p>
          ) : null}
        </aside>

        <section aria-label="Strategy canvas" className={styles.canvas}>
          {session.ordered.length === 0 ? (
            <div className={styles.empty}>
              <h2 className="disp">Start a strategy</h2>
              <p>
                Pick where the money comes from, or ask the AI to draft one.
              </p>
              <div className={styles.emptyActions}>
                {palette[0]!.items.map((item) => (
                  <button
                    type="button"
                    key={item.name}
                    className={`lift ${styles.ghostBlock}`}
                    onClick={() => item.primitive && add(item.primitive)}
                  >
                    <BlockIcon kind={item.kind} size={30} radius={9} />
                    {item.name}
                  </button>
                ))}
                <a
                  href="#/compose"
                  className={`lift ${styles.askAi} ${styles.askAiBlock}`}
                >
                  <Sparkle size={15} />
                  Ask AI
                </a>
              </div>
            </div>
          ) : (
            <div
              className={styles.stage}
              style={{ transform: `scale(${zoom})` }}
            >
              {session.ordered.map((node, index) => {
                const description = descriptions[index]!;
                const selected = node.id === editor.selectedNodeId;
                const left = index % 2 === 0 ? 168 : 328;
                const top = 24 + index * 128;
                const next = session.ordered[index + 1];
                const hot = selected || next?.id === editor.selectedNodeId;
                return (
                  <div key={node.id}>
                    {next ? (
                      <>
                        <svg
                          className={styles.link}
                          style={{ left: 292, top: top + 76 }}
                          width="176"
                          height="52"
                          viewBox="0 0 176 52"
                          fill="none"
                          aria-hidden="true"
                        >
                          <path
                            d={
                              index % 2 === 0
                                ? "M8 0C8 30 168 22 168 52"
                                : "M168 0C168 30 8 22 8 52"
                            }
                            stroke="#C3C7BD"
                            strokeWidth="6"
                            strokeLinecap="round"
                            opacity=".35"
                          />
                          <path
                            className={styles.linkFlow}
                            d={
                              index % 2 === 0
                                ? "M8 0C8 30 168 22 168 52"
                                : "M168 0C168 30 8 22 8 52"
                            }
                            stroke={hot ? "#0F110E" : "#7D8278"}
                            strokeWidth="2"
                            strokeLinecap="round"
                          />
                        </svg>
                        {description.output ? (
                          <span
                            className={`mono ${styles.linkLabel} ${hot ? styles.linkLabelHot : ""}`}
                            style={{ top: top + 102 }}
                          >
                            {description.output}
                          </span>
                        ) : null}
                      </>
                    ) : null}
                    <button
                      type="button"
                      className={`${styles.node} ${selected ? styles.nodeSelected : ""} ${description.bad ? styles.nodeBad : ""}`}
                      style={{ left, top }}
                      aria-pressed={selected}
                      onClick={() => editor.selectNode(node.id)}
                    >
                      <BlockIcon
                        kind={blockKind(node.data.kind)}
                        size={42}
                        radius={12}
                      />
                      <span className={styles.nodeText}>
                        <span className={styles.nodeName}>
                          {description.name}
                        </span>
                        <span className={`mono ${styles.nodeDetail}`}>
                          {description.detail}
                        </span>
                      </span>
                      <span
                        className={styles.nodeDot}
                        style={{
                          background: description.bad
                            ? "#D9705D"
                            : "var(--good)",
                        }}
                      />
                    </button>
                  </div>
                );
              })}
              {allowed.length > 0 ? (
                <>
                  <svg
                    className={styles.link}
                    style={{
                      left: 292,
                      top: 24 + (session.ordered.length - 1) * 128 + 76,
                    }}
                    width="176"
                    height="52"
                    viewBox="0 0 176 52"
                    fill="none"
                    aria-hidden="true"
                  >
                    <path
                      d={
                        (session.ordered.length - 1) % 2 === 0
                          ? "M8 0C8 30 168 22 168 52"
                          : "M168 0C168 30 8 22 8 52"
                      }
                      stroke="#B5BAAE"
                      strokeWidth="2"
                      strokeDasharray="3 6"
                      strokeLinecap="round"
                    />
                  </svg>
                  <div
                    className={styles.ghostRow}
                    style={{
                      top: 24 + session.ordered.length * 128,
                      left: session.ordered.length % 2 === 0 ? 168 : 328,
                    }}
                  >
                    {allowed.map((kind) => (
                      <button
                        type="button"
                        key={kind}
                        className={styles.ghost}
                        onClick={() => add(kind)}
                      >
                        <svg
                          width="16"
                          height="16"
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="#0F110E"
                          strokeWidth="2"
                          strokeLinecap="round"
                          aria-hidden="true"
                        >
                          <path d="M12 5v14M5 12h14" />
                        </svg>
                        Add <span className="mono">{label(kind)}</span>
                      </button>
                    ))}
                  </div>
                </>
              ) : null}
            </div>
          )}

          <div className={styles.zoom}>
            <button
              type="button"
              aria-label="Zoom out"
              onClick={() => setZoom((value) => Math.max(0.6, value - 0.1))}
            >
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="#0F110E"
                strokeWidth="2.2"
                strokeLinecap="round"
                aria-hidden="true"
              >
                <path d="M5 12h14" />
              </svg>
            </button>
            <span className="mono">{Math.round(zoom * 100)}%</span>
            <button
              type="button"
              aria-label="Zoom in"
              onClick={() => setZoom((value) => Math.min(1.3, value + 0.1))}
            >
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="#0F110E"
                strokeWidth="2.2"
                strokeLinecap="round"
                aria-hidden="true"
              >
                <path d="M12 5v14M5 12h14" />
              </svg>
            </button>
            <span className={styles.zoomDivider} />
            <button type="button" className="mono" onClick={() => setZoom(1)}>
              Fit
            </button>
          </div>
          <div className={styles.counter}>
            <span className="mono">
              {session.ordered.length} block
              {session.ordered.length === 1 ? "" : "s"}
              {session.previewing ? " · updating" : ""}
            </span>
            <button
              type="button"
              className="mono"
              onClick={() => setDebug(true)}
            >
              {"{ } JSON"}
            </button>
          </div>
          {!session.walletReady ? (
            <div className={`mono ${styles.walletHint}`}>
              Connect a wallet on BNB Chain for live prices and checks
            </div>
          ) : null}
        </section>

        <Inspector
          ctx={ctx}
          index={selectedIndex}
          description={
            selectedIndex >= 0 ? descriptions[selectedIndex] : undefined
          }
        />
      </div>

      {debug ? (
        <div
          className={styles.debugBackdrop}
          role="dialog"
          aria-modal="true"
          aria-label="Recipe JSON"
        >
          <div className={styles.debug}>
            <div className={styles.debugHead}>
              <strong className="disp">Recipe JSON</strong>
              <button
                type="button"
                className={`lift ${styles.secondary}`}
                onClick={() => setDebug(false)}
              >
                Close
              </button>
            </div>
            <pre className="mono">
              {JSON.stringify(
                { recipe: session.recipe, diagnostics: session.diagnostics },
                null,
                2,
              )}
            </pre>
          </div>
        </div>
      ) : null}
    </div>
  );
}

function label(kind: RecipePrimitiveKind): string {
  const labels: Record<RecipePrimitiveKind, string> = {
    WALLET_ASSET: "Wallet asset",
    SUPPLIED_POSITION: "Supplied stock",
    BORROWED_POSITION: "Debt",
    TRADE: "Trade",
    SUPPLY: "Supply",
    BORROW: "Borrow",
    REPAY: "Repay",
    WITHDRAW: "Withdraw",
  };
  return labels[kind];
}
