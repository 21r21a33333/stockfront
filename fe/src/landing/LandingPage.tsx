import { useState, type FormEvent, type MouseEvent } from "react";
import { navigate } from "@/app/route";
import { useComposeStore } from "@/compose/composeStore";
import { ArrowUpRight, Check, Logo, Sparkle } from "@/ui/BlockIcon";
import { blockIcons, blockTints, type BlockKind } from "@/ui/blocks";
import { WalletButton } from "@/wallet/WalletButton";
import styles from "./LandingPage.module.css";

const examples = [
  "Buy $20 of Tesla",
  "Borrow USDT against my Nvidia stock",
  "Swap my Tesla stock back to USDT",
  "Repay my loan and get my stock back",
];

const marquee = [...examples, "Buy $6 of Nvidia and supply it to Venus"];

const blocks: {
  kind: BlockKind;
  name: string;
  line: string;
  venue: string;
  example: string;
  detail: string;
  takes: string;
  gives: string;
  next: string[];
  soon?: boolean;
}[] = [
  {
    kind: "trade",
    name: "Trade",
    line: "Swap stablecoins for tokenized stocks, or back.",
    venue: "Binance Web3 routing",
    example: "Buy 0.0255 NVDAB for 6 USDT",
    detail: "Best live route",
    takes: "USDT",
    gives: "NVDAB",
    next: ["Supply"],
  },
  {
    kind: "supply",
    name: "Supply",
    line: "Put a stock on Venus so it works as collateral.",
    venue: "Venus",
    example: "Supply 0.0255 NVDAB",
    detail: "Becomes collateral",
    takes: "NVDAB",
    gives: "Collateral on Venus",
    next: ["Borrow", "Withdraw"],
  },
  {
    kind: "borrow",
    name: "Borrow",
    line: "Borrow stablecoins against your stock, inside a safe limit.",
    venue: "Venus",
    example: "Borrow 1 USDT, health stays above 1.5",
    detail: "Safe max built in",
    takes: "Collateral",
    gives: "USDT",
    next: ["Trade", "Repay"],
  },
  {
    kind: "repay",
    name: "Repay",
    line: "Pay back a loan in full, in one step.",
    venue: "Venus",
    example: "Repay the full USDT debt",
    detail: "Approval only if needed",
    takes: "Debt",
    gives: "Loan closed",
    next: ["Withdraw"],
  },
  {
    kind: "withdraw",
    name: "Withdraw",
    line: "Bring your supplied stock back to your wallet.",
    venue: "Venus",
    example: "Withdraw all supplied NVDAB",
    detail: "Unlocks once repaid",
    takes: "Supplied stock",
    gives: "Stock in wallet",
    next: ["Trade"],
  },
  {
    kind: "earn",
    name: "Earn",
    line: "Put borrowed stablecoins to work on a reviewed protocol.",
    venue: "Coming soon",
    example: "Earn on borrowed USDT",
    detail: "Coming soon",
    takes: "USDT",
    gives: "Earn position",
    next: ["Repay"],
    soon: true,
  },
];

const trust = [
  {
    n: "[A]",
    title: "You sign everything",
    body: "Every step opens in your wallet for review. Nothing runs automatically.",
  },
  {
    n: "[B]",
    title: "Funds never leave your wallet",
    body: "Stockfront never takes custody. Positions live on-chain, under your address.",
  },
  {
    n: "[C]",
    title: "Live BNB Chain data",
    body: "Prices, loan health and rates come straight from live markets, every time.",
  },
  {
    n: "[D]",
    title: "Built on proven rails",
    body: "Binance Web3 and Venus power the trades and loans.",
  },
];

export function LandingPage() {
  const [prompt, setPrompt] = useState("");
  const [selected, setSelected] = useState(0);
  const setPendingPrompt = useComposeStore((state) => state.setPendingPrompt);
  const current = blocks[selected]!;

  function start(text?: string) {
    if (text?.trim()) setPendingPrompt(text.trim());
    navigate("compose");
  }

  return (
    <div className={styles.page}>
      <section className={styles.hero}>
        <div aria-hidden="true" className={styles.grid} />
        <div
          aria-hidden="true"
          className={styles.cloud}
          style={{ width: 620, height: 240, left: -120, bottom: 60 }}
        />
        <div
          aria-hidden="true"
          className={styles.cloud}
          style={{
            width: 520,
            height: 200,
            right: -80,
            bottom: 140,
            animationDuration: "46s",
          }}
        />
        <div
          aria-hidden="true"
          className={styles.cloud}
          style={{
            width: 900,
            height: 260,
            left: "28%",
            bottom: -90,
            animationDuration: "52s",
          }}
        />
        <div
          aria-hidden="true"
          className={styles.cloud}
          style={{
            width: 300,
            height: 110,
            left: "12%",
            top: 210,
            opacity: 0.7,
            animationDuration: "30s",
          }}
        />
        <div
          aria-hidden="true"
          className={styles.cloud}
          style={{
            width: 360,
            height: 120,
            right: "10%",
            top: 300,
            opacity: 0.6,
            animationDuration: "34s",
          }}
        />

        <header className={styles.header}>
          <a href="#/" className={styles.brand} aria-label="Stockfront home">
            <Logo size={30} />
            <span className="disp">Stockfront</span>
          </a>
          <nav aria-label="Main" className={`mono ${styles.nav}`}>
            <a href="#how" onClick={(event) => scrollTo(event, "how")}>
              How it works
            </a>
            <a href="#blocks" onClick={(event) => scrollTo(event, "blocks")}>
              Blocks
            </a>
            <a href="#trust" onClick={(event) => scrollTo(event, "trust")}>
              Safety
            </a>
          </nav>
          <div className={styles.headerActions}>
            <WalletButton variant="glass" />
            <button
              type="button"
              className={`lift mono ${styles.startPill}`}
              onClick={() => start()}
            >
              Start building
            </button>
          </div>
        </header>

        <div className={styles.heroBody}>
          <h1 className={`disp ${styles.heroTitle}`}>
            Tell{" "}
            <span className={styles.ai}>
              AI
              <span aria-hidden="true" className={styles.twinkle}>
                <svg
                  width="100%"
                  height="100%"
                  viewBox="0 0 24 24"
                  fill="#D5F56B"
                  stroke="#0F110E"
                  strokeWidth="1.4"
                  strokeLinejoin="round"
                >
                  <path d="M12 2l2.2 7.8L22 12l-7.8 2.2L12 22l-2.2-7.8L2 12l7.8-2.2z" />
                </svg>
              </span>
            </span>{" "}
            what your
            <br />
            stocks should do.
          </h1>
          <p className={styles.heroSub}>
            Describe a strategy in plain English. AI turns it into blocks you
            can shape. You sign every step in your own wallet.
          </p>
          <form
            className={styles.heroPrompt}
            onSubmit={(event: FormEvent) => {
              event.preventDefault();
              start(prompt);
            }}
          >
            <span aria-hidden="true" className={styles.promptIcon}>
              <Sparkle />
            </span>
            <label htmlFor="heroPrompt" className="srOnly">
              Describe your strategy
            </label>
            <input
              id="heroPrompt"
              value={prompt}
              onChange={(event) => setPrompt(event.target.value)}
              placeholder="What do you want your stocks to do?"
            />
            <button
              type="submit"
              className={`lift ${styles.promptGo}`}
              aria-label="Compose this strategy"
            >
              <svg
                width="20"
                height="20"
                viewBox="0 0 24 24"
                fill="none"
                stroke="#D5F56B"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                <path d="M5 12h14M13 6l6 6-6 6" />
              </svg>
            </button>
          </form>
          <div className={styles.heroChips}>
            {examples.map((label) => (
              <button
                type="button"
                key={label}
                className={styles.heroChip}
                onClick={() => start(label)}
              >
                {label}
              </button>
            ))}
          </div>
        </div>
        <div style={{ height: 96 }} />
      </section>

      <div aria-label="Example requests" className={styles.marqueeWrap}>
        <div className={styles.marquee}>
          {[...marquee, ...marquee].map((text, index) => (
            <div key={`${text}-${index}`} className={styles.marqueeItem}>
              <span className="disp">“{text}”</span>
              <span aria-hidden="true" />
            </div>
          ))}
        </div>
      </div>

      <section id="how" className={styles.section}>
        <div className={styles.sectionHead}>
          <div className={styles.sectionTitle}>
            <span className="eyebrow">How it works · 01—03</span>
            <h2 className="disp">
              Describe it. Shape it.{" "}
              <span style={{ color: "#6B7066" }}>Sign it.</span>
            </h2>
          </div>
          <p>
            Three moves from an idea to a live position. You stay in control at
            every one of them.
          </p>
        </div>
        <div className={styles.steps}>
          <article className={styles.step} style={{ background: "#A9D3F6" }}>
            <div className={styles.stepTop}>
              <span className="mono">[01]</span>
              <span
                className={`mono ${styles.stepTag}`}
                style={{ background: "rgba(255,255,255,.55)" }}
              >
                AI
              </span>
            </div>
            <div
              className={styles.stepDemo}
              style={{
                background: "#FFFFFF",
                boxShadow: "0 20px 40px -26px rgba(18,52,94,.6)",
              }}
            >
              <span className={`mono ${styles.stepWho}`}>You</span>
              <span className="mono" style={{ fontSize: 17 }}>
                <span className={styles.typing}>
                  Borrow USDT against my Nvidia
                </span>
                <span className={styles.caret} />
              </span>
            </div>
            <div className={styles.stepText}>
              <h3 className="disp">Describe</h3>
              <p>
                Tell the AI what you want in plain English. It drafts a strategy
                you can read at a glance.
              </p>
            </div>
          </article>
          <article
            className={styles.step}
            style={{ background: "#FFFFFF", border: "1px solid #E1E4DC" }}
          >
            <div className={styles.stepTop}>
              <span className="mono">[02]</span>
              <span
                className={`mono ${styles.stepTag}`}
                style={{ background: "#EEF0EB" }}
              >
                Live
              </span>
            </div>
            <div
              className={styles.stepDemo}
              style={{
                background: "#F5F6F2",
                border: "1px solid #E5E8E0",
                gap: 14,
              }}
            >
              <div className={styles.demoRow}>
                <span
                  className={styles.demoIcon}
                  style={{ background: "#F7CB98" }}
                >
                  <svg
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="#0F110E"
                    strokeWidth="1.9"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    aria-hidden="true"
                  >
                    <path d={blockIcons.borrow} />
                  </svg>
                </span>
                <strong>Borrow USDT</strong>
                <span
                  className="mono"
                  style={{ marginLeft: "auto", fontSize: 13 }}
                >
                  1.00 USDT
                </span>
              </div>
              <div className={styles.demoHealth}>
                <div className="mono">
                  <span style={{ color: "#5A5F56" }}>Health factor</span>
                  <span>2.85 · Safe</span>
                </div>
                <div className={styles.demoBar}>
                  <span />
                </div>
              </div>
            </div>
            <div className={styles.stepText}>
              <h3 className="disp">Shape</h3>
              <p>
                Adjust any block. Prices, loan health and live checks update as
                you go, right inside the block.
              </p>
            </div>
          </article>
          <article className={styles.step} style={{ background: "#D5F56B" }}>
            <div className={styles.stepTop}>
              <span className="mono">[03]</span>
              <span
                className={`mono ${styles.stepTag}`}
                style={{ background: "rgba(255,255,255,.6)" }}
              >
                Your wallet
              </span>
            </div>
            <div className={styles.signList}>
              {["Approve + Buy NVDAB", "Supply to Venus"].map((text) => (
                <div key={text} className={styles.signRow}>
                  <span className={styles.signDone}>
                    <Check size={12} />
                  </span>
                  <span>{text}</span>
                  <span className="mono">Done</span>
                </div>
              ))}
              <div className={styles.signRow} style={{ background: "#FFFFFF" }}>
                <span className={styles.signPulse} />
                <strong>Borrow 1 USDT</strong>
                <span className="mono">Sign in wallet</span>
              </div>
            </div>
            <div className={styles.stepText}>
              <h3 className="disp">Sign</h3>
              <p>
                Approve each step in your own wallet. Nothing runs on its own,
                and nothing moves without you.
              </p>
            </div>
          </article>
        </div>
      </section>

      <section
        id="blocks"
        className={`${styles.section} ${styles.sectionTight}`}
      >
        <div className={`${styles.sectionHead} ${styles.sectionHeadRule}`}>
          <div className={styles.sectionTitle}>
            <span className="eyebrow">The blocks · [06]</span>
            <h2 className="disp">
              Six blocks.
              <br />
              Countless strategies.
            </h2>
          </div>
          <p>
            Every strategy is a chain of the same few moves. Pick one to see
            what goes in, what comes out and what usually follows.
          </p>
        </div>
        <div className={styles.blocks}>
          <ul className={styles.blockList}>
            {blocks.map((block, index) => {
              const active = index === selected;
              return (
                <li key={block.name}>
                  <button
                    type="button"
                    className={styles.blockRow}
                    aria-pressed={active}
                    style={{ background: active ? "#FFFFFF" : "transparent" }}
                    onClick={() => setSelected(index)}
                    onMouseEnter={() => setSelected(index)}
                  >
                    <span className="mono">[0{index + 1}]</span>
                    <span className={styles.blockRowText}>
                      <span className="disp">
                        {block.name}
                        {block.soon ? (
                          <span className={`mono ${styles.soon}`}>Soon</span>
                        ) : null}
                      </span>
                      <span>{block.line}</span>
                    </span>
                    <span
                      className={styles.blockArrow}
                      style={{
                        background: active ? "#0F110E" : blockTints[block.kind],
                      }}
                    >
                      <ArrowUpRight
                        size={16}
                        stroke={active ? blockTints[block.kind] : "#0F110E"}
                      />
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
          <div
            aria-live="polite"
            className={styles.blockStage}
            style={{ background: blockTints[current.kind] }}
          >
            <div aria-hidden="true" className={styles.dots} />
            <div className={styles.stageTop}>
              <span className="mono">Block 0{selected + 1} / 06</span>
              <span className={`mono ${styles.stageTag}`}>
                {current.detail}
              </span>
            </div>
            <div className={styles.stageFlow} key={current.name}>
              <span className={`mono ${styles.port}`}>
                in · {current.takes}
              </span>
              <span className={styles.dash} />
              <div className={styles.bigBlock}>
                <div className={styles.bigHead}>
                  <span
                    className={styles.bigIcon}
                    style={{ background: blockTints[current.kind] }}
                  >
                    <svg
                      width="26"
                      height="26"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="#0F110E"
                      strokeWidth="1.8"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      aria-hidden="true"
                    >
                      <path d={blockIcons[current.kind]} />
                    </svg>
                  </span>
                  <div>
                    <span className="disp">{current.name}</span>
                    <span className="mono">{current.venue}</span>
                  </div>
                </div>
                <div className={`mono ${styles.bigExample}`}>
                  {current.example}
                </div>
              </div>
              <span className={styles.dash} />
              <span className={`mono ${styles.port}`}>
                out · {current.gives}
              </span>
            </div>
            <div className={styles.stageNext}>
              <span className="mono">Often followed by</span>
              {current.next.map((name) => {
                const index = blocks.findIndex((block) => block.name === name);
                const block = blocks[index]!;
                return (
                  <button
                    type="button"
                    key={name}
                    onClick={() => setSelected(index)}
                  >
                    <span style={{ background: blockTints[block.kind] }} />
                    {name}
                  </button>
                );
              })}
            </div>
          </div>
        </div>
      </section>

      <section id="trust" className={styles.trustWrap}>
        <div className={styles.trust}>
          {trust.map((item) => (
            <div key={item.n} className={styles.trustItem}>
              <span className="mono">{item.n}</span>
              <div>
                <span className="disp">{item.title}</span>
                <span>{item.body}</span>
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className={styles.cta}>
        <div aria-hidden="true" className={styles.ctaShapes}>
          <div>
            <span style={{ background: "#0F110E" }} />
            <span style={{ border: "2px solid #0F110E" }} />
          </div>
          <div style={{ paddingLeft: 134 }}>
            <span style={{ background: "#FFFFFF" }} />
          </div>
        </div>
        <div className={styles.ctaText}>
          <span className="eyebrow" style={{ color: "#0F110E" }}>
            Start in a sentence
          </span>
          <h2 className="disp">
            Your stocks are sitting still. Put them to work.
          </h2>
        </div>
        <div className={styles.ctaActions}>
          <div>
            <button
              type="button"
              className={`lift ${styles.ctaPrimary}`}
              onClick={() => start()}
            >
              Start building
              <span>
                <ArrowUpRight size={16} />
              </span>
            </button>
            <WalletButton variant="glass" />
          </div>
          <span className="mono">AI proposes. You review and sign.</span>
        </div>
      </section>

      <footer className={styles.footer}>
        <div className={styles.footerBrand}>
          <div>
            <Logo />
            <span className="disp">Stockfront</span>
          </div>
          <p>
            Returns aren&apos;t guaranteed. Tokenized stocks and DeFi carry
            risk, including loss of funds. Stockfront never holds your funds;
            every action is signed by you.
          </p>
        </div>
        <nav aria-label="Footer" className={`mono ${styles.footerNav}`}>
          <a href="#how" onClick={(event) => scrollTo(event, "how")}>
            How it works
          </a>
          <a href="#blocks" onClick={(event) => scrollTo(event, "blocks")}>
            Blocks
          </a>
          <a href="#trust" onClick={(event) => scrollTo(event, "trust")}>
            Safety
          </a>
          <a href="#/compose">Compose</a>
          <a href="#/builder">Builder</a>
        </nav>
      </footer>
    </div>
  );
}

/** In-page anchors must not change the hash, which is the app's route. */
function scrollTo(event: MouseEvent, id: string) {
  event.preventDefault();
  document.getElementById(id)?.scrollIntoView({ behavior: "smooth" });
}
