import {
  Fragment,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { formatUnits } from "viem";
import type {
  PreparedRecipeStep,
  RecipeDefinition,
  RecipeNode,
} from "@/api/generated";
import { AppHeader } from "@/app/AppHeader";
import { usePositionsSheet } from "@/app/positionsSheet";
import { navigate } from "@/app/route";
import { assetFor } from "@/builder/describe";
import { useComposeStore } from "@/compose/composeStore";
import { shortAddress } from "@/recipe/assets";
import { useRecipeEditor } from "@/recipe/editor/store";
import type { AssetOption } from "@/recipe/editor/types";
import {
  formatDisplayAmount,
  formatPositionAmount,
  formatUsd,
} from "@/recipe/format";
import { useRecipeSession, type FreshPreparation } from "@/session/context";
import { ArrowUpRight, BlockIcon, Check } from "@/ui/BlockIcon";
import { blockKind } from "@/ui/blocks";
import { formatHealth, healthZone, zoneColors } from "@/ui/health";
import { priceMoved, runSteps, type RunStep } from "./steps";
import styles from "./RunPage.module.css";

type LocalPhase = "idle" | "preparing" | "signing" | "price" | "declined";

export function RunPage() {
  const session = useRecipeSession();
  const editorName = useRecipeEditor((state) => state.name);
  const execution = session.execution;
  const openPositions = usePositionsSheet((state) => state.setOpen);
  const definition = execution.run?.recipe ?? session.recipe;
  const nodes = useMemo(() => orderDefinition(definition), [definition]);
  const steps = runSteps(nodes, execution.run?.attempts ?? [], execution.busy);
  const current = steps.find((step) => step.state !== "done");
  const doneCount = steps.filter((step) => step.state === "done").length;
  const [review, setReview] = useState<FreshPreparation>();
  const [phase, setPhase] = useState<LocalPhase>("idle");
  const [moved, setMoved] = useState<{
    fresh: FreshPreparation;
    was: bigint;
    now: bigint;
  }>();
  const [error, setError] = useState<string>();
  const [toast, setToast] = useState<string>();
  const previousDone = useRef(doneCount);

  useEffect(() => {
    if (doneCount > previousDone.current) {
      setToast(`Step ${doneCount} confirmed`);
      setReview(undefined);
      const timer = window.setTimeout(() => setToast(undefined), 2600);
      previousDone.current = doneCount;
      return () => window.clearTimeout(timer);
    }
    previousDone.current = doneCount;
  }, [doneCount]);

  const canAct =
    session.walletReady && execution.sameRecipe && !execution.completed;
  const currentKey = current
    ? `${current.node.id}:${current.collateralEnabled}`
    : "";
  useEffect(() => {
    if (!canAct || !current || current.state !== "ready" || phase !== "idle")
      return;
    if (
      review &&
      review.prepared.steps.some((step) => step.nodeId === current.node.id)
    )
      return;
    setPhase("preparing");
    session
      .prepareNext()
      .then(setReview)
      .catch((cause: unknown) => setError(readable(cause)))
      .finally(() => setPhase("idle"));
  }, [canAct, currentKey]); // eslint-disable-line react-hooks/exhaustive-deps

  async function execute(fresh: FreshPreparation) {
    setPhase("signing");
    setMoved(undefined);
    await execution.execute(fresh.prepared);
    setPhase("idle");
  }

  async function sign() {
    if (!current) return;
    setError(undefined);
    setPhase("preparing");
    try {
      const fresh = await session.prepareNext();
      const change = priceMoved(
        review?.routes[current.node.id],
        fresh.routes[current.node.id],
      );
      setReview(fresh);
      if (change) {
        setMoved({ fresh, ...change });
        setPhase("price");
        return;
      }
      await execute(fresh);
    } catch (cause) {
      setError(readable(cause));
      setPhase("idle");
    }
  }

  async function abandon() {
    if (
      !window.confirm(
        "Abandon this run? Only do this if your wallet shows nothing pending for this strategy. Stockfront will stop tracking it.",
      )
    )
      return;
    await execution.abandonRun();
    setReview(undefined);
    setPhase("idle");
  }

  const name =
    execution.run && !execution.sameRecipe ? "Unfinished strategy" : editorName;
  const header = (
    <AppHeader
      showNav={false}
      middle={
        <div className={styles.headerMiddle}>
          <a href="#/builder" className={styles.back}>
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="#0F110E"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <path d="M19 12H5M11 6l-6 6 6 6" />
            </svg>
            Back to builder
          </a>
          <span className={styles.headerName}>{name}</span>
        </div>
      }
    />
  );

  if (!session.walletReady) {
    return (
      <Shell header={header}>
        <Notice
          title="Connect your wallet"
          body="Connect a wallet on BNB Chain to sign this strategy. Nothing is sent until you approve each step."
        />
      </Shell>
    );
  }
  if (execution.run && !execution.sameRecipe) {
    return (
      <Shell header={header}>
        <Notice
          title="You have an unfinished run"
          body="Another strategy was started in this wallet and isn't finished. Open it to continue, or abandon it if your wallet shows nothing pending."
        >
          <button
            type="button"
            className={`lift ${styles.primary}`}
            onClick={() =>
              execution.run &&
              session.loadDefinition(
                execution.run.recipe,
                "Unfinished strategy",
              )
            }
          >
            Open that strategy
          </button>
          <button
            type="button"
            className={`lift ${styles.secondary}`}
            onClick={() => void abandon()}
          >
            Abandon run
          </button>
        </Notice>
      </Shell>
    );
  }
  if (steps.length === 0) {
    return (
      <Shell header={header}>
        <Notice
          title="Nothing to run yet"
          body="Add an action block in the builder, then come back to sign it."
        >
          <a href="#/builder" className={`lift ${styles.primary}`}>
            Open builder
          </a>
        </Notice>
      </Shell>
    );
  }

  if (execution.completed) {
    return (
      <Shell header={header}>
        <Finished
          nodes={nodes}
          steps={steps.length}
          onAgain={() => {
            void execution.startNewRun();
            useComposeStore.getState().setComposed(undefined);
            navigate("compose");
          }}
          onPositions={() => openPositions(true)}
        />
        {toast ? <Toast text={toast} /> : null}
      </Shell>
    );
  }

  const attempt = execution.run?.attempts
    .filter((item) => item.nodeId === current?.node.id)
    .at(-1);
  const txHash =
    attempt?.reference && "transactionHash" in attempt.reference
      ? attempt.reference.transactionHash
      : undefined;
  const card = current
    ? reviewCard(
        current,
        review,
        nodes,
        session.assets,
        session.runtimeContext?.completedNodes ?? [],
      )
    : undefined;
  const waiting = phase === "signing" || current?.state === "waiting";
  const showSign =
    current &&
    (current.state === "ready" || current.state === "cancelled") &&
    !waiting &&
    phase !== "price";
  const phaseLabel = waiting
    ? ["Waiting for wallet", "var(--lime)"]
    : current?.state === "confirming"
      ? ["Confirming", "var(--sky)"]
      : phase === "price"
        ? ["Needs you", "var(--warn)"]
        : current?.state === "cancelled" || phase === "declined"
          ? ["Cancelled", "var(--bg)"]
          : current?.state === "unknown"
            ? ["Unknown", "var(--bad-soft)"]
            : phase === "preparing"
              ? ["Preparing", "var(--bg)"]
              : ["Ready to sign", "var(--bg)"];

  return (
    <Shell header={header}>
      <main className={styles.main}>
        <section aria-label="Steps" className={styles.steps}>
          <div className={styles.stepsHead}>
            <span className="eyebrow">
              Run · {doneCount} of {steps.length} done
            </span>
            <h1 className={`disp ${styles.h1}`}>Sign each step.</h1>
            <div className={styles.progress}>
              <div style={{ width: `${(doneCount / steps.length) * 100}%` }} />
            </div>
          </div>
          <ol className={styles.list}>
            {steps.map((step, index) => {
              const active = step === current;
              const icon =
                step.state === "done" ? (
                  <span className={styles.done}>
                    <Check size={15} />
                  </span>
                ) : active && waiting ? (
                  <span className={`mono ${styles.waiting}`}>{index + 1}</span>
                ) : active && step.state === "confirming" ? (
                  <span className={`spinner ${styles.confirming}`} />
                ) : active &&
                  (step.state === "unknown" ||
                    step.state === "cancelled" ||
                    phase === "price") ? (
                  <span className={`mono ${styles.problem}`}>!</span>
                ) : (
                  <span
                    className={`mono ${styles.upcoming} ${active ? styles.upcomingActive : ""}`}
                  >
                    {index + 1}
                  </span>
                );
              return (
                <li key={step.node.id} className={styles.item}>
                  <div className={styles.rail}>
                    {icon}
                    {index < steps.length - 1 ? (
                      <span
                        className={styles.railLine}
                        style={{
                          background:
                            step.state === "done"
                              ? "var(--ink)"
                              : "var(--line-2)",
                        }}
                      />
                    ) : null}
                  </div>
                  <div
                    className={styles.itemText}
                    style={{
                      opacity: step.state === "done" || active ? 1 : 0.55,
                    }}
                  >
                    <span style={{ fontWeight: active ? 600 : 500 }}>
                      {stepTitle(step, nodes, session.assets)}
                    </span>
                    <span className="mono">
                      {statusText(step, active, waiting, phase)}
                    </span>
                  </div>
                </li>
              );
            })}
          </ol>
        </section>

        <section aria-live="polite" className={styles.reviewArea}>
          <div className={styles.card}>
            <div className={styles.cardHead}>
              {current ? (
                <BlockIcon
                  kind={blockKind(current.node.kind)}
                  size={46}
                  radius={14}
                />
              ) : null}
              <div className={styles.cardTitle}>
                <span className="mono">
                  Step {current ? steps.indexOf(current) + 1 : steps.length} ·
                  Review
                </span>
                <span className="disp">{card?.title ?? "Preparing…"}</span>
              </div>
              <span
                className={`mono ${styles.phase}`}
                style={{ background: phaseLabel[1] }}
              >
                {phaseLabel[0]}
              </span>
            </div>

            {phase === "price" && moved ? (
              <div className={`${styles.alert} ${styles.alertWarn}`}>
                <div>
                  <strong>The price moved</strong>
                  <span>
                    You&apos;ll now get{" "}
                    <span className="mono">
                      {formatToken(
                        moved.now,
                        buyAsset(current, session.assets),
                      )}
                    </span>{" "}
                    <span className="mono" style={{ color: "var(--muted)" }}>
                      (was{" "}
                      {formatToken(
                        moved.was,
                        buyAsset(current, session.assets),
                      )}
                      )
                    </span>
                    . Nothing was sent.
                  </span>
                </div>
                <div className={styles.alertActions}>
                  <button
                    type="button"
                    className={`lift ${styles.primary}`}
                    onClick={() => void execute(moved.fresh)}
                  >
                    Accept new price
                  </button>
                  <button
                    type="button"
                    className={`lift ${styles.secondary}`}
                    onClick={() => {
                      setMoved(undefined);
                      setPhase("declined");
                    }}
                  >
                    Cancel
                  </button>
                </div>
              </div>
            ) : null}
            {phase === "declined" ||
            (current?.state === "cancelled" && !waiting) ? (
              <div className={`${styles.alert} ${styles.alertSoft}`}>
                <div>
                  <strong>No problem, nothing was sent.</strong>
                  <span>
                    {phase === "declined"
                      ? "You kept the old price, so this step wasn't signed."
                      : "You cancelled in your wallet."}{" "}
                    Try again whenever you&apos;re ready.
                  </span>
                </div>
              </div>
            ) : null}
            {current?.state === "unknown" && !waiting ? (
              <div className={`${styles.alert} ${styles.alertBad}`}>
                <div>
                  <strong>We couldn&apos;t confirm this step</strong>
                  <span>
                    The transaction may still land. Check again before doing
                    anything else.
                  </span>
                </div>
                <div className={styles.alertActions}>
                  <button
                    type="button"
                    className={`lift ${styles.primary}`}
                    onClick={() => void execution.checkStatus()}
                  >
                    Check again
                  </button>
                  {execution.canRetrySubmission ? (
                    <button
                      type="button"
                      className={`lift ${styles.secondary}`}
                      onClick={() => void execution.retrySubmission()}
                    >
                      Resend signed order
                    </button>
                  ) : null}
                  <button
                    type="button"
                    className={`lift ${styles.secondary}`}
                    onClick={() => void abandon()}
                  >
                    Abandon run
                  </button>
                </div>
              </div>
            ) : null}

            {card ? (
              <>
                <div className={styles.flows}>
                  {card.pay ? (
                    <AmountRow
                      label="You pay"
                      value={card.pay}
                      first={Boolean(card.get)}
                    />
                  ) : null}
                  {card.get ? (
                    <AmountRow
                      label="You get"
                      value={card.get}
                      last={Boolean(card.pay)}
                    />
                  ) : null}
                  {card.allow ? (
                    <div className={styles.allow}>
                      <span className={styles.lock} aria-hidden="true">
                        <svg
                          width="15"
                          height="15"
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="#D5F56B"
                          strokeWidth="2"
                          strokeLinecap="round"
                          strokeLinejoin="round"
                        >
                          <rect x="5" y="11" width="14" height="9" rx="2" />
                          <path d="M8 11V8a4 4 0 0 1 8 0" />
                        </svg>
                      </span>
                      {card.allow}
                    </div>
                  ) : null}
                </div>
                <dl className={`mono ${styles.facts}`}>
                  <dt>Network fee</dt>
                  <dd>{card.fee}</dd>
                  {card.minimum ? (
                    <>
                      <dt>Minimum received</dt>
                      <dd>{card.minimum}</dd>
                    </>
                  ) : null}
                  {card.health ? (
                    <>
                      <dt>Loan health</dt>
                      <dd>
                        {card.health.text}
                        {card.health.zone ? (
                          <span
                            className={styles.zone}
                            style={{ background: zoneColors[card.health.zone] }}
                          >
                            {card.health.zone}
                          </span>
                        ) : null}
                      </dd>
                    </>
                  ) : null}
                  <dt>Interacting with</dt>
                  <dd>
                    {card.venue} ·{" "}
                    {card.contract ? shortAddress(card.contract) : "—"}
                  </dd>
                </dl>
                <details className={styles.details}>
                  <summary className="mono">
                    Details
                    <svg
                      width="12"
                      height="12"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="#0F110E"
                      strokeWidth="2.2"
                      strokeLinecap="round"
                      aria-hidden="true"
                    >
                      <path d="M6 9l6 6 6-6" />
                    </svg>
                  </summary>
                  <dl className="mono">
                    <dt>Method</dt>
                    <dd>{card.methods}</dd>
                    <dt>Contract</dt>
                    <dd>{card.contract ?? "—"}</dd>
                    <dt>Chain</dt>
                    <dd>BNB Chain (56)</dd>
                  </dl>
                </details>
              </>
            ) : (
              <div className={styles.placeholder}>
                <span className={`spinner ${styles.placeholderSpin}`} />
                <span className="mono">
                  Getting a fresh quote and preparing the transaction…
                </span>
              </div>
            )}

            {current?.state === "confirming" ? (
              <div className={styles.confirmBar}>
                <div className={styles.confirmHead}>
                  <span>Confirming on-chain…</span>
                  {txHash ? (
                    <a
                      className="mono"
                      href={`https://bscscan.com/tx/${txHash}`}
                      target="_blank"
                      rel="noreferrer"
                    >
                      View on explorer ↗
                    </a>
                  ) : null}
                </div>
                <div className={styles.bar}>
                  <div />
                </div>
              </div>
            ) : null}

            {(error ?? execution.error) ? (
              <p className={styles.error}>{error ?? execution.error}</p>
            ) : null}

            <div className={styles.actions}>
              {showSign ? (
                <>
                  <button
                    type="button"
                    className={`lift ${styles.sign}`}
                    disabled={phase === "preparing"}
                    onClick={() => void sign()}
                  >
                    {current?.state === "cancelled" || phase === "declined"
                      ? "Try again"
                      : phase === "preparing"
                        ? "Refreshing quote…"
                        : "Sign in wallet"}
                    <span className={styles.signIcon}>
                      <BlockIcon
                        kind="wallet"
                        size={36}
                        radius={11}
                        tint="var(--lime)"
                      />
                    </span>
                  </button>
                  <span className={`mono ${styles.hint}`}>
                    Opens your wallet. Nothing is sent until you approve there.
                  </span>
                </>
              ) : null}
              {waiting ? (
                <span className={styles.waitingPill}>
                  <span className={`spinner ${styles.waitingSpin}`} />
                  Waiting for your wallet…
                </span>
              ) : null}
            </div>
          </div>
        </section>
      </main>
      {toast ? <Toast text={toast} /> : null}
    </Shell>
  );
}

function Shell({
  header,
  children,
}: {
  header: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className={styles.page}>
      {header}
      {children}
    </div>
  );
}

function Notice({
  title,
  body,
  children,
}: {
  title: string;
  body: string;
  children?: ReactNode;
}) {
  return (
    <main className={styles.notice}>
      <h1 className="disp">{title}</h1>
      <p>{body}</p>
      {children ? <div className={styles.noticeActions}>{children}</div> : null}
    </main>
  );
}

function Toast({ text }: { text: string }) {
  return (
    <div role="status" className={styles.toast}>
      <span className={styles.toastIcon}>
        <Check size={11} stroke="#0F110E" />
      </span>
      {text}
    </div>
  );
}

function AmountRow({
  label,
  value,
  first,
  last,
}: {
  label: string;
  value: { amount: string; symbol: string; usd?: string };
  first?: boolean;
  last?: boolean;
}) {
  return (
    <div
      className={styles.amountRow}
      style={{
        borderRadius: first
          ? "16px 16px 4px 4px"
          : last
            ? "4px 4px 16px 16px"
            : 16,
      }}
    >
      <span className={styles.amountLabel}>{label}</span>
      <span className={`mono ${styles.amountToken}`}>
        {value.symbol.slice(0, 2)}
      </span>
      <span className={`mono ${styles.amountValue}`}>
        {value.amount} {value.symbol}
      </span>
      {value.usd ? (
        <span className={`mono ${styles.amountUsd}`}>{value.usd}</span>
      ) : null}
    </div>
  );
}

function Finished({
  nodes,
  steps,
  onAgain,
  onPositions,
}: {
  nodes: RecipeNode[];
  steps: number;
  onAgain(): void;
  onPositions(): void;
}) {
  const session = useRecipeSession();
  const stocks = session.assets.filter(
    (asset) => asset.kind === "bstock" && (asset.balanceRaw ?? 0n) > 0n,
  );
  const supplied =
    session.borrowing?.positions.flatMap((position) => position.supplied) ?? [];
  const borrowed =
    session.borrowing?.positions.flatMap((position) => position.borrowed) ?? [];
  const health = session.borrowing?.healthFactor
    ? Number(session.borrowing.healthFactor)
    : undefined;
  const owned = [
    ...stocks.slice(0, 3).map((asset) => ({
      kind: "wallet" as const,
      label: `${asset.symbol} in wallet`,
      sub: asset.name,
      value: `${formatDisplayAmount(formatUnits(asset.balanceRaw ?? 0n, asset.decimals))} ${asset.symbol}`,
      note: "",
    })),
    ...supplied.map((asset) => ({
      kind: "supplied" as const,
      label: "Collateral on Venus",
      sub: `Supplied ${asset.token.symbol}`,
      value: `${formatPositionAmount(asset.amount)} ${asset.token.symbol}`,
      note: formatUsd(asset.valueUsd),
    })),
    ...borrowed.map((asset) => ({
      kind: "borrow" as const,
      label: "Debt",
      sub: "Borrowed on Venus",
      value: `${formatPositionAmount(asset.amount)} ${asset.token.symbol}`,
      note:
        health !== undefined && Number.isFinite(health)
          ? `Health ${health.toFixed(2)} · ${healthZone(health)}`
          : "",
    })),
  ];
  return (
    <main className={styles.finished}>
      <div className={styles.finChain}>
        {nodes.map((node, index) => (
          <Fragment key={node.id}>
            <span
              className={styles.finBlock}
              style={{ animationDelay: `${0.2 + index * 0.28}s` }}
            >
              <BlockIcon
                kind={blockKind(node.kind)}
                size={40}
                radius={12}
                tint="transparent"
              />
            </span>
            {index < nodes.length - 1 ? (
              <span
                className={styles.finLine}
                style={{ animationDelay: `${0.35 + index * 0.28}s` }}
              />
            ) : null}
          </Fragment>
        ))}
      </div>
      <div className={styles.finText}>
        <span className="eyebrow">
          {steps} of {steps} signed · Complete
        </span>
        <h1 className="disp">
          {supplied.length ? "Your stock is at work." : "All done."}
        </h1>
      </div>
      <section aria-label="What you own now" className={styles.owned}>
        <div className={styles.ownedHead}>
          <span className="disp">What you own now</span>
          <span className="mono">Live · BNB Chain</span>
        </div>
        {owned.length === 0 ? (
          <p className={styles.ownedEmpty}>Balances are refreshing…</p>
        ) : (
          owned.map((item) => (
            <div
              key={`${item.label}-${item.value}`}
              className={styles.ownedRow}
            >
              <BlockIcon kind={item.kind} size={38} radius={11} />
              <span className={styles.ownedText}>
                <strong>{item.label}</strong>
                <small>{item.sub}</small>
              </span>
              <span className={styles.ownedValue}>
                <span className="mono">{item.value}</span>
                {item.note ? <small className="mono">{item.note}</small> : null}
              </span>
            </div>
          ))
        )}
      </section>
      <div className={styles.noticeActions}>
        <button
          type="button"
          className={`lift ${styles.sign}`}
          onClick={onAgain}
        >
          Build another strategy
          <span className={styles.signIcon}>
            <span className={styles.limeSquare}>
              <ArrowUpRight size={15} />
            </span>
          </span>
        </button>
        <button
          type="button"
          className={`lift ${styles.secondary}`}
          onClick={onPositions}
        >
          View positions
        </button>
      </div>
    </main>
  );
}

interface ReviewCard {
  title: string;
  pay?: { amount: string; symbol: string; usd?: string };
  get?: { amount: string; symbol: string; usd?: string };
  allow?: string;
  fee: string;
  minimum?: string;
  health?: { text: string; zone?: ReturnType<typeof healthZone> };
  venue: string;
  contract?: string;
  methods: string;
}

function reviewCard(
  step: RunStep,
  fresh: FreshPreparation | undefined,
  nodes: RecipeNode[],
  assets: AssetOption[],
  completed: {
    nodeId: string;
    outputs: { asset: { token: string; amount: string } }[];
  }[],
): ReviewCard | undefined {
  const prepared =
    fresh?.prepared.steps.filter((item) => item.nodeId === step.node.id) ?? [];
  if (prepared.length === 0) return undefined;
  const node = step.node;
  const input = inputOf(node, nodes, completed);
  const transactions = prepared.flatMap(transactionsOf);
  const fee = transactions.reduce(
    (total, tx) => total + BigInt(tx.gasLimit ?? 0) * BigInt(tx.gasPrice ?? 0),
    0n,
  );
  const feeText =
    fee > 0n
      ? `≈ ${formatDisplayAmount(formatUnits(fee, 18))} BNB`
      : "Estimated by your wallet";
  const contract =
    transactions.find((tx) => tx.kind !== "APPROVE")?.to ?? transactions[0]?.to;
  const methods =
    transactions.map((tx) => tx.label).join(" + ") || "signTypedData";
  const preview = fresh?.preview.nodes.find((item) => item.nodeId === node.id);
  const approve = transactions.some((tx) => tx.kind === "APPROVE");

  if (node.kind === "TRADE") {
    const route = fresh?.routes[node.id];
    const sell = assetFor(assets, route?.sell.token);
    const buy = assetFor(assets, route?.buy.token ?? node.buyToken);
    const trade = prepared.find((item) => item.kind === "TRADE");
    const minimum =
      trade?.kind === "TRADE" && trade.action.executionMode === "SWAP"
        ? trade.action.transaction.minReceiveAmount
        : undefined;
    const rfq = trade?.kind === "TRADE" && trade.action.executionMode === "RFQ";
    return {
      title: `${approve ? "Approve + " : ""}${buy?.kind === "settlement" ? `Sell for ${buy.symbol}` : `Buy ${buy?.symbol ?? "stock"}`}`,
      pay: route
        ? {
            amount: amountText(route.sell.amount, sell),
            symbol: sell?.symbol ?? "",
          }
        : undefined,
      get: route
        ? {
            amount: amountText(route.buy.amount, buy),
            symbol: buy?.symbol ?? "",
          }
        : undefined,
      fee: rfq ? "Paid by the market maker" : feeText,
      minimum: minimum
        ? `${amountText(minimum, buy)} ${buy?.symbol ?? ""}`
        : undefined,
      venue: route?.venue ?? "Trading route",
      contract,
      methods: rfq
        ? `${approve ? "approve + " : ""}signTypedData (order)`
        : methods,
    };
  }
  const borrowing = prepared.find(
    (item) => item.kind !== "APPROVAL" && item.kind !== "TRADE",
  );
  const health =
    borrowing && "action" in borrowing && "preview" in borrowing.action
      ? borrowing.action.preview
      : undefined;
  const healthCard =
    health?.healthFactorAfter || health?.healthFactorBefore
      ? {
          text: `${formatHealth(health.healthFactorBefore)} → ${formatHealth(health.healthFactorAfter)}`,
          zone: health.healthFactorAfter
            ? healthZone(Number(health.healthFactorAfter))
            : undefined,
        }
      : undefined;
  const venusFee = health?.estimatedNetworkFee
    ? `≈ ${formatPositionAmount(health.estimatedNetworkFee.amount)} ${health.estimatedNetworkFee.tokenSymbol}`
    : feeText;

  switch (borrowing?.kind) {
    case "SUPPLY": {
      const asset = assetFor(assets, input?.token);
      return {
        title: `${approve ? "Approve + " : ""}Supply to Venus`,
        pay: input
          ? {
              amount: amountText(input.amount, asset),
              symbol: asset?.symbol ?? "",
            }
          : undefined,
        fee: venusFee,
        venue: "Venus",
        contract,
        methods,
      };
    }
    case "ENABLE_COLLATERAL": {
      const asset = assetFor(
        assets,
        node.kind === "BORROW" ? input?.token : undefined,
      );
      return {
        title: "Enable collateral",
        allow: `Use your supplied ${asset?.symbol ?? "stock"} as collateral you can borrow against`,
        fee: venusFee,
        venue: "Venus",
        contract,
        methods,
      };
    }
    case "BORROW": {
      const asset = assetFor(assets, borrowing.output.token);
      return {
        title: `Borrow ${amountText(borrowing.output.amount, asset)} ${asset?.symbol ?? ""}`,
        get: {
          amount: amountText(borrowing.output.amount, asset),
          symbol: asset?.symbol ?? "",
        },
        fee: venusFee,
        health: healthCard,
        venue: "Venus",
        contract,
        methods,
      };
    }
    case "REPAY": {
      const capability =
        preview?.kind === "REPAY" ? preview.capability : undefined;
      return {
        title: `${approve ? "Approve + " : ""}Repay full debt`,
        pay: capability
          ? {
              amount: formatPositionAmount(capability.debtAmount),
              symbol: capability.debt.symbol,
              usd: "incl. interest",
            }
          : undefined,
        fee: venusFee,
        health: healthCard,
        venue: "Venus",
        contract,
        methods,
      };
    }
    case "WITHDRAW": {
      const position =
        preview?.kind === "WITHDRAW" ? preview.position : undefined;
      return {
        title: "Withdraw to your wallet",
        get: position
          ? {
              amount: formatPositionAmount(position.amount),
              symbol: position.token.symbol,
              usd: formatUsd(position.valueUsd),
            }
          : undefined,
        fee: venusFee,
        venue: "Venus",
        contract,
        methods,
      };
    }
    default:
      return undefined;
  }
}

function transactionsOf(step: PreparedRecipeStep): {
  to: string;
  kind: string;
  label: string;
  gasLimit?: string;
  gasPrice?: string;
}[] {
  switch (step.kind) {
    case "APPROVAL":
      return [
        {
          to: step.action.to,
          kind: "APPROVE",
          label: "approve",
          gasLimit: step.action.gasLimit,
          gasPrice: step.action.gasPrice,
        },
      ];
    case "TRADE":
      return step.action.executionMode === "SWAP"
        ? [
            {
              to: step.action.transaction.to,
              kind: "SWAP",
              label: "swap",
              gasLimit: step.action.transaction.gasLimit,
              gasPrice: step.action.transaction.gasPrice,
            },
          ]
        : [];
    default:
      return step.action.transactions.map((tx) => ({
        to: tx.to,
        kind: tx.kind,
        label: tx.kind.toLowerCase().replace("_", " "),
        gasLimit: tx.gasLimit ?? undefined,
        gasPrice: tx.gasPrice ?? undefined,
      }));
  }
}

function inputOf(
  node: RecipeNode,
  nodes: RecipeNode[],
  completed: {
    nodeId: string;
    outputs: { asset: { token: string; amount: string } }[];
  }[],
): { token: string; amount: string } | undefined {
  const index = nodes.findIndex((item) => item.id === node.id);
  const previous = nodes[index - 1];
  if (!previous) return undefined;
  if (previous.kind === "WALLET_ASSET")
    return { token: previous.token, amount: previous.rawAmount };
  if (previous.kind === "SUPPLIED_POSITION")
    return { token: previous.token, amount: "0" };
  return completed.find((item) => item.nodeId === previous.id)?.outputs[0]
    ?.asset;
}

function buyAsset(
  step: RunStep | undefined,
  assets: AssetOption[],
): AssetOption | undefined {
  return step?.node.kind === "TRADE"
    ? assetFor(assets, step.node.buyToken)
    : undefined;
}

function formatToken(raw: bigint, asset?: AssetOption): string {
  return `${formatDisplayAmount(formatUnits(raw, asset?.decimals ?? 18))} ${asset?.symbol ?? ""}`;
}

function amountText(raw: string, asset?: AssetOption): string {
  return formatDisplayAmount(formatUnits(BigInt(raw), asset?.decimals ?? 18));
}

function stepTitle(
  step: RunStep,
  nodes: RecipeNode[],
  assets: AssetOption[],
): string {
  const node = step.node;
  const index = nodes.findIndex((item) => item.id === node.id);
  const previous = nodes[index - 1];
  const inputToken =
    previous && "token" in previous
      ? previous.token
      : previous?.kind === "TRADE"
        ? previous.buyToken
        : undefined;
  switch (node.kind) {
    case "TRADE": {
      const buy = assetFor(assets, node.buyToken);
      return buy?.kind === "settlement"
        ? `Sell for ${buy.symbol}`
        : `Buy ${buy?.symbol ?? "stock"}`;
    }
    case "SUPPLY":
      return `Supply ${assetFor(assets, inputToken)?.symbol ?? ""} to Venus`;
    case "BORROW": {
      const debt = assetFor(assets, node.debtToken);
      return `${step.collateralEnabled ? "" : "Enable collateral, then "}Borrow ${amountText(node.rawAmount, debt)} ${debt?.symbol ?? ""}`;
    }
    case "REPAY":
      return "Repay full debt";
    case "WITHDRAW":
      return "Withdraw to wallet";
    default:
      return node.kind;
  }
}

function statusText(
  step: RunStep,
  active: boolean,
  waiting: boolean,
  phase: LocalPhase,
): string {
  if (step.state === "done") return "Done";
  if (!active) return "Up next";
  if (waiting) return "Waiting for your wallet";
  if (phase === "price") return "Price changed · needs you";
  switch (step.state) {
    case "confirming":
      return "Confirming on-chain";
    case "cancelled":
      return "Cancelled · nothing sent";
    case "unknown":
      return "Result unknown";
    default:
      return phase === "preparing" ? "Preparing" : "Ready to sign";
  }
}

function readable(cause: unknown): string {
  return cause instanceof Error
    ? cause.message
    : "Something went wrong. Nothing was sent.";
}

/** Recipe nodes in execution order, following edges from the source. */
function orderDefinition(recipe: RecipeDefinition): RecipeNode[] {
  const next = new Map(
    recipe.edges.map((edge) => [edge.from.nodeId, edge.to.nodeId]),
  );
  const targets = new Set(recipe.edges.map((edge) => edge.to.nodeId));
  const byId = new Map(recipe.nodes.map((node) => [node.id, node]));
  const ordered: RecipeNode[] = [];
  for (
    let node = recipe.nodes.find((item) => !targets.has(item.id));
    node && !ordered.includes(node);
  ) {
    ordered.push(node);
    const following = next.get(node.id);
    node = following ? byId.get(following) : undefined;
  }
  return [
    ...ordered,
    ...recipe.nodes.filter((node) => !ordered.includes(node)),
  ];
}
