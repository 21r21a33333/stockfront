import { useMutation, useQuery } from "@tanstack/react-query";
import { Fragment, useEffect, useMemo, useState, type FormEvent } from "react";
import { formatUnits } from "viem";
import type {
  ComposedStrategy,
  RecipePreview,
  StrategyStep,
} from "@/api/generated";
import { StockfrontApiError } from "@/api/client";
import { recipesApi } from "@/api/recipes";
import { AppHeader } from "@/app/AppHeader";
import { navigate } from "@/app/route";
import { formatDisplayAmount, formatPositionAmount } from "@/recipe/format";
import { useRecipeEditor } from "@/recipe/editor/store";
import { useRecipeSession } from "@/session/context";
import { ArrowUpRight, BlockIcon, Check, Sparkle } from "@/ui/BlockIcon";
import { blockKind, blockTints, type BlockKind } from "@/ui/blocks";
import { formatHealth, healthZone } from "@/ui/health";
import { marketNote } from "@/ui/market";
import { PromptInput } from "@/ui/PromptInput";
import { TokenDot } from "@/ui/TokenDot";
import { useComposeStore } from "./composeStore";
import { planBlocks, planTitle } from "./plan";
import {
  curatedStrategies,
  popularity,
  type StrategyMarket,
} from "./strategies";
import styles from "./ComposePage.module.css";

const examples = [
  "Buy $20 of Tesla",
  "Borrow USDT against my Nvidia stock",
  "Swap my Tesla stock back to USDT",
  "Repay my loan and get my stock back",
];

const templates: {
  name: string;
  flow: string;
  chain: BlockKind[];
  prompt?: string;
}[] = [
  {
    name: "Buy a stock",
    flow: "Wallet USDT → Trade",
    chain: ["wallet", "trade"],
    prompt: "Buy $6 of Nvidia",
  },
  {
    name: "Borrow against a stock",
    flow: "Supplied stock → Borrow",
    chain: ["supplied", "borrow"],
    prompt: "Borrow 1 USDT against my supplied stock",
  },
  {
    name: "Borrow & earn",
    flow: "Supplied → Borrow USDT → Earn",
    chain: ["supplied", "borrow", "earn"],
    prompt: "Borrow 1 USDT against my supplied Tesla collateral and earn on it",
  },
  {
    name: "Buy an index basket",
    flow: "Wallet USDT → Basket",
    chain: ["wallet", "liquidity"],
    prompt: "Invest 5 USDT in the CoinMarketCap 20 index",
  },
];

const thinkingLines = [
  "Reading your request…",
  "Checking live markets…",
  "Validating strategy…",
];

export function ComposePage() {
  const session = useRecipeSession();
  const store = useComposeStore();
  const savedName = useRecipeEditor((state) => state.name);
  const hasDraft = useRecipeEditor((state) => state.nodes.length > 0);
  const [stage, setStage] = useState(0);
  const [answer, setAnswer] = useState("");
  const [refining, setRefining] = useState(false);
  const [refinement, setRefinement] = useState("");
  const [openError, setOpenError] = useState<string>();
  const stablecoins = useMemo(
    () =>
      new Set(
        session.assets
          .filter((asset) => asset.kind === "settlement")
          .map((asset) => asset.symbol.toUpperCase()),
      ),
    [session.assets],
  );

  // Each borrow strategy names one stock, and it has to be one Venus will
  // actually take as collateral today, so the choice comes from the live
  // markets rather than from the bStock catalog. Household names rank first,
  // and the two strategies take different stocks so the row is not repetitive.
  const markets = useQuery({
    queryKey: ["borrowing-markets"],
    queryFn: recipesApi.borrowingMarkets,
    staleTime: 5 * 60_000,
  });
  const ranked = useMemo(() => {
    const assetAt = (address: string) =>
      session.assets.find(
        (asset) => asset.address.toLowerCase() === address.toLowerCase(),
      );
    return (markets.data?.items ?? [])
      .filter(
        (item) =>
          item.supplyAvailable &&
          item.borrowAvailable &&
          item.oracleValid &&
          item.unavailableReasons.length === 0,
      )
      .sort(
        (left, right) =>
          popularity(left.collateral.symbol) -
          popularity(right.collateral.symbol),
      )
      .flatMap((item): StrategyMarket[] => {
        const stock = assetAt(item.collateral.contract);
        const stable = assetAt(item.debt.contract);
        return stock && stable
          ? [{ stock, stable, collateralFactor: item.collateralFactor }]
          : [];
      });
  }, [markets.data, session.assets]);
  const earn = ranked[0];
  const leverage = ranked[1] ?? ranked[0];
  const basket = session.catalog?.baskets[0];
  const carry = useQuery({
    queryKey: ["carry-rates", earn?.stock.address, earn?.stable.address],
    queryFn: () =>
      recipesApi.carryOpportunities(earn!.stock.address, earn!.stable.address),
    enabled: Boolean(earn),
    // Rates move slowly and only label the cards here; the builder re-reads
    // them when a recipe actually picks an earn product.
    staleTime: 5 * 60_000,
  });
  const strategies = useMemo(
    () =>
      curatedStrategies({
        earn,
        leverage,
        basket,
        // Only a reviewed product can actually be deposited into, so the
        // headline rate has to come from one. The best rate on the market may
        // belong to a product the policy will not let the user enter.
        carry: carry.data?.items.find(
          (item) => item.qualification === "ELIGIBLE",
        ),
      }),
    [earn, leverage, basket, carry.data],
  );

  const compose = useMutation({
    mutationFn: (request: {
      prompt: string;
      previous?: ComposedStrategy["intent"];
    }) =>
      recipesApi.compose({
        prompt: request.prompt,
        previous: request.previous ?? null,
        wallet: session.walletReady ? session.wallet : undefined,
      }),
    onSuccess: (composed) => store.setComposed(composed),
  });
  const preview = useMutation({ mutationFn: recipesApi.preview });

  function run(prompt: string, previous?: ComposedStrategy["intent"]) {
    const text = prompt.trim();
    if (!text) return;
    store.setPrompt(text);
    store.setComposed(undefined);
    setRefining(false);
    setAnswer("");
    setOpenError(undefined);
    preview.reset();
    compose.mutate({ prompt: text, previous });
  }

  useEffect(() => {
    if (!store.pendingPrompt) return;
    const prompt = store.pendingPrompt;
    store.setPendingPrompt(undefined);
    run(prompt);
  }, [store.pendingPrompt]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (!compose.isPending) return setStage(0);
    const timers = [1200, 2600].map((delay, index) =>
      window.setTimeout(() => setStage(index + 1), delay),
    );
    return () => timers.forEach(window.clearTimeout);
  }, [compose.isPending]);

  const composed = store.composed;
  const recipe = composed?.recipe ?? undefined;
  useEffect(() => {
    if (!recipe || !session.wallet || !session.walletReady) return;
    preview.mutate({ recipe, context: { wallet: session.wallet } });
  }, [recipe, session.wallet, session.walletReady]); // eslint-disable-line react-hooks/exhaustive-deps

  const phase = compose.isPending ? "thinking" : composed ? "plan" : "idle";
  const steps = composed?.intent.steps ?? [];
  const blocks = planBlocks(steps, stablecoins);
  const ready = Boolean(recipe && composed?.validation?.valid);
  const notes = composed
    ? planNotes(composed, steps, session, stablecoins)
    : [];
  const stats =
    recipe && preview.data ? planStats(preview.data, session.assets) : [];

  function openInBuilder() {
    if (!recipe || !composed) return;
    try {
      session.loadDefinition(recipe, planTitle(steps, stablecoins));
      navigate("builder");
    } catch (cause) {
      setOpenError(
        cause instanceof Error
          ? cause.message
          : "This strategy could not be opened.",
      );
    }
  }

  return (
    <div className={styles.page}>
      <AppHeader current="compose" sticky />
      <main className={styles.main}>
        <div className={styles.intro}>
          <span className="eyebrow">Compose · AI</span>
          <h1 className={`disp ${styles.title}`}>
            What do you want your stocks to do?
          </h1>
        </div>

        <div className={styles.promptArea}>
          <form
            className={styles.prompt}
            onSubmit={(event: FormEvent) => {
              event.preventDefault();
              run(store.prompt);
            }}
          >
            <span aria-hidden="true" className={styles.promptIcon}>
              <Sparkle />
            </span>
            <label htmlFor="composePrompt" className="srOnly">
              Describe your strategy
            </label>
            <PromptInput
              id="composePrompt"
              value={store.prompt}
              maxLength={1000}
              onChange={store.setPrompt}
              placeholder="e.g. Buy $6 of Nvidia and supply it to Venus"
            />
            <button
              type="submit"
              className={`lift ${styles.compose}`}
              disabled={compose.isPending || !store.prompt.trim()}
            >
              Compose
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="#D5F56B"
                strokeWidth="2.2"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                <path d="M5 12h14M13 6l6 6-6 6" />
              </svg>
            </button>
          </form>
          <div className={styles.chips}>
            {examples.map((label) => (
              <button
                type="button"
                key={label}
                className={styles.chip}
                onClick={() => run(label)}
              >
                {label}
              </button>
            ))}
          </div>
        </div>

        {compose.error ? (
          <div role="alert" className={`${styles.card} ${styles.errorCard}`}>
            <strong>{composeErrorTitle(compose.error)}</strong>
            <span>{composeErrorBody(compose.error)}</span>
            <button
              type="button"
              className={`lift ${styles.secondary}`}
              onClick={() => run(store.prompt)}
            >
              Try again
            </button>
          </div>
        ) : null}

        {phase === "idle" && !compose.error ? (
          <div className={styles.idle}>
            {strategies.length > 0 ? (
              <section className={styles.group} aria-labelledby="curated">
                <div className={styles.sectionHead}>
                  <h2 id="curated" className="eyebrow">
                    Popular earning strategies
                  </h2>
                  <span className={`mono ${styles.count}`}>
                    [{String(strategies.length).padStart(2, "0")}]
                  </span>
                </div>
                <div className={styles.strategies}>
                  {strategies.map((strategy) => (
                    <article key={strategy.key} className={styles.strategy}>
                      <span className={styles.miniChain}>
                        {strategy.chain.map((kind, index) => (
                          <Fragment key={`${kind}-${index}`}>
                            <BlockIcon kind={kind} size={28} radius={9} />
                            {index < strategy.chain.length - 1 ? (
                              <span className={styles.miniLink} />
                            ) : null}
                          </Fragment>
                        ))}
                      </span>
                      <div className={styles.strategyHead}>
                        <h3 className={`disp ${styles.strategyName}`}>
                          {strategy.name}
                        </h3>
                        <span
                          className={`mono ${styles.risk} ${
                            strategy.tier === "high"
                              ? styles.riskHigh
                              : styles.riskLower
                          }`}
                        >
                          {strategy.riskLabel}
                        </span>
                      </div>
                      <p className={styles.strategySummary}>
                        {strategy.summary}
                      </p>
                      <div className={styles.headline}>
                        <span className={`disp ${styles.headlineValue}`}>
                          {strategy.headline.value}
                        </span>
                        <span className={`mono ${styles.headlineLabel}`}>
                          {strategy.headline.label}
                        </span>
                      </div>
                      <p className={styles.riskNote}>{strategy.risk}</p>
                      <button
                        type="button"
                        className={`lift ${styles.strategyRun}`}
                        onClick={() => run(strategy.prompt)}
                      >
                        Compose this
                        <ArrowUpRight size={14} />
                      </button>
                    </article>
                  ))}
                </div>
              </section>
            ) : null}
            <section className={styles.group} aria-labelledby="templates">
              <div className={styles.sectionHead}>
                <h2 id="templates" className="eyebrow">
                  Start from a template
                </h2>
                <span className={`mono ${styles.count}`}>[04]</span>
              </div>
              <div className={styles.templates}>
                {templates.map((template) => (
                  <button
                    type="button"
                    key={template.name}
                    className={styles.template}
                    disabled={!template.prompt}
                    onClick={() => template.prompt && run(template.prompt)}
                  >
                    <span className={styles.miniChain}>
                      {template.chain.map((kind, index) => (
                        <Fragment key={kind}>
                          <BlockIcon kind={kind} size={34} radius={10} />
                          {index < template.chain.length - 1 ? (
                            <span className={styles.miniLink} />
                          ) : null}
                        </Fragment>
                      ))}
                    </span>
                    <span className={styles.templateText}>
                      <span className={`disp ${styles.templateName}`}>
                        {template.name}
                        {!template.prompt ? (
                          <span className={`mono ${styles.soon}`}>Soon</span>
                        ) : null}
                      </span>
                      <span className={`mono ${styles.templateFlow}`}>
                        {template.flow}
                      </span>
                    </span>
                  </button>
                ))}
              </div>
            </section>
            {hasDraft ? (
              <div className={styles.recent}>
                <h2 className="eyebrow">Recent strategies</h2>
                <a href="#/builder" className={styles.recentChip}>
                  <span className={styles.recentDots}>
                    {session.ordered.map((node) => (
                      <span
                        key={node.id}
                        style={{
                          background: blockTints[blockKind(node.data.kind)],
                        }}
                      />
                    ))}
                  </span>
                  <span>{savedName}</span>
                  <span className={`mono ${styles.recentMeta}`}>
                    Open in builder
                  </span>
                </a>
              </div>
            ) : null}
          </div>
        ) : null}

        {phase === "thinking" ? (
          <section
            aria-live="polite"
            className={`${styles.card} ${styles.thinking}`}
          >
            <div className={styles.thinkBlocks} aria-hidden="true">
              {[0, 1, 2, 3].map((index) => (
                <Fragment key={index}>
                  <span
                    className={styles.thinkBlock}
                    style={{ animationDelay: `${index * 0.2}s` }}
                  />
                  {index < 3 ? (
                    <span className={`flowLine ${styles.thinkLine}`} />
                  ) : null}
                </Fragment>
              ))}
            </div>
            <div className={styles.status}>
              {thinkingLines.map((text, index) => (
                <div
                  key={text}
                  className={styles.statusLine}
                  style={{ opacity: stage < index ? 0.4 : 1 }}
                >
                  {stage > index ? (
                    <span className={styles.statusDone}>
                      <Check size={11} />
                    </span>
                  ) : stage === index ? (
                    <span className={`spinner ${styles.statusSpin}`} />
                  ) : (
                    <span className={styles.statusPending} />
                  )}
                  <span className="mono">{text}</span>
                </div>
              ))}
            </div>
            <div className={styles.shimmers} aria-hidden="true">
              <span />
              <span />
              <span />
            </div>
          </section>
        ) : null}

        {phase === "plan" && composed ? (
          <section aria-label="AI plan" className={styles.plan}>
            <header className={styles.planHead}>
              <div className={styles.planMeta}>
                <span
                  className={`mono ${styles.planStatus} ${ready ? styles.statusReady : styles.statusWarn}`}
                >
                  <span />
                  {ready
                    ? "Ready"
                    : steps.length === 0
                      ? "Not supported yet"
                      : "Needs your input"}
                </span>
                {blocks.length > 0 ? (
                  <span className={`mono ${styles.stepCount}`}>
                    {blocks.length} {blocks.length === 1 ? "step" : "steps"}
                  </span>
                ) : null}
              </div>
              <h2 className={`disp ${styles.planTitle}`}>
                {planTitle(steps, stablecoins)}
              </h2>
              <p className={styles.summary}>{composed.summary}</p>
            </header>

            {blocks.length > 0 ? (
              <div
                className={`${styles.planBody} ${stats.length > 0 ? styles.planBodySplit : ""}`}
              >
                <div className={styles.panel}>
                  <span className={`mono ${styles.panelLabel}`}>Steps</span>
                  <ol className={styles.steps}>
                    {blocks.map((block, index) => (
                      <li
                        key={`${block.name}-${index}`}
                        className={styles.step}
                        style={{ animationDelay: `${0.08 + index * 0.1}s` }}
                      >
                        <BlockIcon kind={block.kind} size={38} radius={11} />
                        <span className={styles.stepText}>
                          <strong>{block.name}</strong>
                          <small>{block.detail}</small>
                        </span>
                        <span className={`mono ${styles.stepIndex}`}>
                          {String(index + 1).padStart(2, "0")}
                        </span>
                      </li>
                    ))}
                  </ol>
                </div>
                {stats.length > 0 ? (
                  <div className={`${styles.panel} ${styles.quote}`}>
                    <span className={`mono ${styles.panelLabel}`}>
                      Live quote
                    </span>
                    <dl className={styles.quoteList}>
                      {stats.map((stat) => (
                        <div key={stat.label} className={styles.quoteRow}>
                          <dt>{stat.label}</dt>
                          <dd>
                            <strong
                              className="mono"
                              style={{ color: stat.color }}
                            >
                              {stat.symbol ? (
                                <TokenDot symbol={stat.symbol} size={18} />
                              ) : null}
                              {stat.value}
                            </strong>
                            <small>{stat.sub}</small>
                          </dd>
                        </div>
                      ))}
                    </dl>
                  </div>
                ) : null}
              </div>
            ) : null}

            {composed.missingInputs.length > 0 ? (
              <form
                className={styles.needsInput}
                onSubmit={(event) => {
                  event.preventDefault();
                  if (answer.trim())
                    run(`${store.prompt}. ${answer.trim()}`, composed.intent);
                }}
              >
                <div className={styles.questions}>
                  {composed.missingInputs.map((question) => (
                    <span key={question}>{question}</span>
                  ))}
                </div>
                <label htmlFor="answer" className="srOnly">
                  Your answer
                </label>
                <input
                  id="answer"
                  value={answer}
                  onChange={(event) => setAnswer(event.target.value)}
                  placeholder="Your answer, e.g. 6 USDT"
                />
                <button
                  type="submit"
                  className={`lift ${styles.primarySmall}`}
                  disabled={!answer.trim()}
                >
                  Update plan
                </button>
              </form>
            ) : null}

            {notes.length > 0 ? (
              <ul className={styles.notes}>
                {notes.map((note) => (
                  <li
                    key={note.text}
                    className={note.warn ? styles.noteWarn : undefined}
                  >
                    {note.text}
                  </li>
                ))}
              </ul>
            ) : null}

            {refining ? (
              <form
                className={styles.refine}
                onSubmit={(event) => {
                  event.preventDefault();
                  if (refinement.trim()) run(refinement, composed.intent);
                }}
              >
                <label htmlFor="refine" className="srOnly">
                  Refine the strategy
                </label>
                <input
                  id="refine"
                  value={refinement}
                  onChange={(event) => setRefinement(event.target.value)}
                  placeholder="e.g. Use Tesla instead and spend 10 USDT"
                />
                <button
                  type="submit"
                  className={`lift ${styles.limeButton}`}
                  disabled={!refinement.trim()}
                >
                  Update plan
                </button>
              </form>
            ) : null}

            {openError ? <p className={styles.openError}>{openError}</p> : null}

            <div className={styles.planActions}>
              <button
                type="button"
                className={`lift ${styles.primary}`}
                disabled={!ready}
                onClick={openInBuilder}
              >
                Open in builder
                <span className={styles.primaryIcon}>
                  <ArrowUpRight size={14} />
                </span>
              </button>
              <button
                type="button"
                className={`lift ${styles.secondary}`}
                onClick={() => setRefining((value) => !value)}
              >
                Refine
              </button>
              <button
                type="button"
                className={`${styles.ghost} ${styles.startOver}`}
                onClick={() => {
                  store.setComposed(undefined);
                  store.setPrompt("");
                  compose.reset();
                }}
              >
                Start over
              </button>
            </div>
          </section>
        ) : null}
      </main>
      <div className={styles.footnote}>
        <span className="mono">
          <span />
          AI proposes. You review and sign. Nothing runs automatically.
        </span>
      </div>
    </div>
  );
}

function planNotes(
  composed: ComposedStrategy,
  steps: StrategyStep[],
  session: ReturnType<typeof useRecipeSession>,
  stablecoins: Set<string>,
): { text: string; warn?: boolean }[] {
  const notes: { text: string; warn?: boolean }[] = composed.assumptions.map(
    (text) => ({ text }),
  );
  const trade = steps.find((step) => step.kind === "TRADE");
  if (
    trade?.kind === "TRADE" &&
    !stablecoins.has(trade.buySymbol.toUpperCase())
  ) {
    const stock = session.catalog?.bstocks.find(
      (item) => item.tokenSymbol === trade.buySymbol,
    );
    const note = stock ? marketNote(stock) : undefined;
    if (note) notes.push({ text: note });
  }
  for (const diagnostic of composed.validation?.diagnostics ?? []) {
    notes.push({ text: diagnostic.message, warn: true });
  }
  if (composed.recipe && !session.walletReady) {
    notes.push({
      text: "Connect a wallet on BNB Chain to see live prices and loan health.",
    });
  }
  return notes;
}

interface PlanStat {
  label: string;
  value: string;
  sub: string;
  color?: string;
  /** Token whose icon leads the value. */
  symbol?: string;
}

function planStats(
  preview: RecipePreview,
  assets: ReturnType<typeof useRecipeSession>["assets"],
): PlanStat[] {
  const symbolOf = (token: string) =>
    assets.find((asset) => asset.address.toLowerCase() === token.toLowerCase());
  const stats: PlanStat[] = [];
  for (const node of preview.nodes) {
    if (node.kind === "TRADE" && node.routes?.[0]) {
      const route = node.routes[0];
      const sell = symbolOf(route.sell.token);
      const buy = symbolOf(route.buy.token);
      stats.push(
        {
          label: "You spend",
          value: `${formatDisplayAmount(formatUnits(BigInt(route.sell.amount), sell?.decimals ?? 18))} ${sell?.symbol ?? ""}`,
          sub: "From your wallet",
          symbol: sell?.symbol,
        },
        {
          label: "You get",
          value: `≈ ${formatDisplayAmount(formatUnits(BigInt(route.buy.amount), buy?.decimals ?? 18))} ${buy?.symbol ?? ""}`,
          sub: `Best quote via ${route.venue}`,
          symbol: buy?.symbol,
        },
        {
          label: "Price impact",
          value: route.priceImpactPercent
            ? `${formatPositionAmount(route.priceImpactPercent)}%`
            : "—",
          sub: "Live quote",
        },
        {
          label: "Route",
          value:
            route.executionMode === "SWAP" ? "Instant swap" : "Signed order",
          sub:
            route.executionMode === "SWAP"
              ? "Settles on-chain now"
              : "Fills when a market maker accepts",
        },
      );
    }
    if (node.kind === "BORROW" && node.capability) {
      const capability = node.capability;
      const after = capability.healthFactorAfter
        ? Number(capability.healthFactorAfter)
        : undefined;
      stats.push(
        {
          label: "You borrow",
          value: `${formatPositionAmount(capability.requestedAmount)} ${capability.debt.symbol}`,
          symbol: capability.debt.symbol,
          sub: `Safe max ${formatPositionAmount(capability.conservativeMaxAmount)} ${capability.debt.symbol}`,
        },
        {
          label: "Borrow rate",
          value: `${formatPositionAmount(capability.borrowApyPercent)}%`,
          sub: "APY, variable",
        },
        {
          label: "Health",
          value: `${formatHealth(capability.healthFactorBefore)} → ${formatHealth(capability.healthFactorAfter)}`,
          sub:
            after !== undefined && Number.isFinite(after)
              ? healthZone(after)
              : "—",
          color: after !== undefined && after < 1.5 ? "var(--bad)" : undefined,
        },
      );
    }
    if (node.kind === "REPAY" && node.capability) {
      const capability = node.capability;
      stats.push(
        {
          label: "You repay",
          value: `${formatPositionAmount(capability.debtAmount)} ${capability.debt.symbol}`,
          sub: "Includes accrued interest",
          symbol: capability.debt.symbol,
        },
        {
          label: "Wallet balance",
          value: `${formatPositionAmount(capability.walletBalance)} ${capability.debt.symbol}`,
          sub: "Available now",
        },
        {
          label: "Approval",
          value: capability.approvalRequired ? "Needed" : "Not needed",
          sub: `${capability.debt.symbol} to Venus`,
        },
      );
    }
    if (node.kind === "WITHDRAW" && node.position) {
      stats.push({
        label: "You get back",
        value: `${formatPositionAmount(node.position.amount)} ${node.position.token.symbol}`,
        symbol: node.position.token.symbol,
        sub:
          node.unavailableReason === "OUTSTANDING_DEBT"
            ? "Repay your debt first"
            : "To your wallet",
      });
    }
  }
  return stats.slice(0, 4);
}

function composeErrorTitle(error: unknown): string {
  if (error instanceof StockfrontApiError && error.code === "ai_rate_limited")
    return "The AI is busy right now.";
  if (
    error instanceof StockfrontApiError &&
    error.code === "invalid_strategy_prompt"
  )
    return "That prompt can't be used.";
  return "We couldn't compose that strategy.";
}

function composeErrorBody(error: unknown): string {
  if (error instanceof StockfrontApiError) {
    if (error.code === "ai_rate_limited")
      return "Try again in a minute. Nothing was sent.";
    if (error.code === "invalid_strategy_prompt")
      return "Write between 1 and 1000 characters.";
    if (error.code === "ai_unavailable" || error.code === "invalid_ai_response")
      return "The AI didn't return a usable plan. Try again, or start from a template.";
  }
  return "Check your connection and try again. Nothing was sent.";
}
