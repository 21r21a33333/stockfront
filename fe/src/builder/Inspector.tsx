import type { ReactNode } from "react";
import { formatUnits } from "viem";
import { compatibleTradeAssets } from "@/recipe/editor/tradePolicy";
import { borrowDebtAssets } from "@/recipe/editor/borrowPolicy";
import { useRecipeEditor } from "@/recipe/editor/store";
import type { RecipeFlowNode } from "@/recipe/editor/types";
import {
  formatDisplayAmount,
  formatPositionAmount,
  formatUsd,
} from "@/recipe/format";
import { useRecipeSession } from "@/session/context";
import { BlockIcon } from "@/ui/BlockIcon";
import { blockKind } from "@/ui/blocks";
import { HealthGauge } from "@/ui/HealthGauge";
import { priceGapPercent, sessionWord, usd } from "@/ui/market";
import { TokenDot } from "@/ui/TokenDot";
import { TokenSelect } from "@/ui/TokenSelect";
import {
  assetFor,
  inputToken,
  type BlockDescription,
  type DescribeContext,
} from "./describe";
import styles from "./Inspector.module.css";

export function Inspector({
  ctx,
  index,
  description,
}: {
  ctx: DescribeContext;
  index: number;
  description?: BlockDescription;
}) {
  const editor = useRecipeEditor();
  const node = ctx.ordered[index];
  if (!node || !description) {
    return (
      <aside aria-label="Inspector" className={styles.inspector}>
        <div className={styles.empty}>
          <span className="disp">Select a block</span>
          <p>Choose a block on the canvas to shape it.</p>
        </div>
      </aside>
    );
  }
  return (
    <aside aria-label="Inspector" className={styles.inspector}>
      <div className={styles.head}>
        <BlockIcon kind={blockKind(node.data.kind)} size={40} radius={12} />
        <div className={styles.headText}>
          <span className={`mono ${styles.kicker}`}>
            {index === 0 ? "Start from" : kindLabel(node.data.kind)} · Step{" "}
            {index + 1}
          </span>
          <span className={`disp ${styles.title}`}>{description.name}</span>
        </div>
      </div>
      <div className={styles.body} key={node.id}>
        <Fields node={node} ctx={ctx} index={index} />
        <div
          className={styles.checks}
          style={{
            background: description.bad
              ? "var(--bad-soft)"
              : "var(--good-soft)",
          }}
        >
          <span className={`mono ${styles.checksTitle}`}>Checks</span>
          {description.checks.length === 0 ? (
            <div className={styles.check}>
              <span
                className={styles.checkDot}
                style={{ background: "#A9AEA2" }}
              />
              {ctx.walletReady
                ? "Waiting for live data"
                : "Connect a wallet to run live checks"}
            </div>
          ) : (
            description.checks.map((check) => (
              <div key={check.text} className={styles.check}>
                <span
                  className={styles.checkDot}
                  style={{
                    background: check.ok ? "var(--good)" : "var(--bad)",
                  }}
                >
                  <svg
                    width="9"
                    height="9"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="#FFFFFF"
                    strokeWidth="3.6"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    aria-hidden="true"
                  >
                    <path
                      d={check.ok ? "M5 12.5l4.5 4.5L19 7" : "M12 7v6M12 17v.5"}
                    />
                  </svg>
                </span>
                {check.text}
              </div>
            ))
          )}
        </div>
        <button
          type="button"
          className={styles.remove}
          onClick={() => {
            const later = ctx.ordered.length - index - 1;
            if (
              later > 0 &&
              !window.confirm(
                `Remove this block and the ${later} block${later > 1 ? "s" : ""} after it?`,
              )
            )
              return;
            editor.removeFrom(node.id);
          }}
        >
          Remove block
          {index < ctx.ordered.length - 1 ? " and what follows" : ""}
        </button>
      </div>
    </aside>
  );
}

function Fields({
  node,
  ctx,
  index,
}: {
  node: RecipeFlowNode;
  ctx: DescribeContext;
  index: number;
}) {
  switch (node.data.kind) {
    case "WALLET_ASSET":
      return <WalletFields node={node} ctx={ctx} index={index} />;
    case "TRADE":
      return <TradeFields node={node} ctx={ctx} index={index} />;
    case "SUPPLY":
      return <SupplyFields node={node} ctx={ctx} index={index} />;
    case "BORROW":
      return <BorrowFields node={node} ctx={ctx} index={index} />;
    case "REPAY":
      return <RepayFields node={node} ctx={ctx} />;
    case "WITHDRAW":
      return <WithdrawFields node={node} ctx={ctx} index={index} />;
    case "SUPPLIED_POSITION":
    case "BORROWED_POSITION":
      return <PositionFields node={node} ctx={ctx} />;
  }
}

function WalletFields({
  node,
  ctx,
  index,
}: {
  node: RecipeFlowNode;
  ctx: DescribeContext;
  index: number;
}) {
  const editor = useRecipeEditor();
  if (node.data.kind !== "WALLET_ASSET") return null;
  const data = node.data;
  const next = ctx.ordered[index + 1]?.data.kind;
  const options = ctx.assets.filter((asset) =>
    next === "SUPPLY" ? asset.kind === "bstock" : true,
  );
  const asset = assetFor(ctx.assets, data.token);
  const stablecoins = options.filter((item) => item.kind === "settlement");
  const stocks = options.filter((item) => item.kind === "bstock");
  return (
    <>
      <Section label="Token">
        {stablecoins.length > 0 ? (
          <div className={styles.tokenList}>
            {stablecoins.map((item) => (
              <button
                type="button"
                key={item.address}
                className={`${styles.tokenRow} ${item.address === data.token ? styles.tokenRowActive : ""}`}
                onClick={() =>
                  editor.updateNode(node.id, { token: item.address })
                }
              >
                <TokenDot symbol={item.symbol} />
                <span className={styles.tokenSymbol}>{item.symbol}</span>
                <span className={`mono ${styles.tokenBalance}`}>
                  {balanceOf(item)}
                </span>
              </button>
            ))}
          </div>
        ) : null}
        <label className="srOnly" htmlFor={`stock-${node.id}`}>
          Or a stock
        </label>
        <TokenSelect
          id={`stock-${node.id}`}
          options={stocks.map((item) => ({
            ...item,
            detail: heldAmount(item),
          }))}
          value={asset?.kind === "bstock" ? asset.address : undefined}
          placeholder={
            stocks.length ? "Or use a stock you hold…" : "No stocks available"
          }
          searchPlaceholder="Search stocks"
          onChange={(token) => editor.updateNode(node.id, { token })}
        />
      </Section>
      <Section label="Amount" htmlFor={`amount-${node.id}`}>
        <div className={styles.amount}>
          <input
            id={`amount-${node.id}`}
            className="mono"
            inputMode="decimal"
            value={data.amount}
            onChange={(event) =>
              editor.updateNode(node.id, { amount: event.target.value })
            }
          />
          <span className="mono">{asset?.symbol ?? ""}</span>
        </div>
        <div className={styles.percentages}>
          {[25, 50, 75, 100].map((percentage) => (
            <button
              type="button"
              key={percentage}
              className="mono"
              disabled={asset?.balanceRaw === undefined}
              onClick={() =>
                asset?.balanceRaw !== undefined &&
                editor.updateNode(node.id, {
                  amount: formatUnits(
                    (asset.balanceRaw * BigInt(percentage)) / 100n,
                    asset.decimals,
                  ),
                })
              }
            >
              {percentage === 100 ? "Max" : `${percentage}%`}
            </button>
          ))}
        </div>
      </Section>
    </>
  );
}

function TradeFields({
  node,
  ctx,
  index,
}: {
  node: RecipeFlowNode;
  ctx: DescribeContext;
  index: number;
}) {
  const editor = useRecipeEditor();
  const session = useRecipeSession();
  if (node.data.kind !== "TRADE") return null;
  const data = node.data;
  const sellToken = inputToken(ctx, index);
  const sell = assetFor(ctx.assets, sellToken);
  const buy = assetFor(ctx.assets, data.buyToken);
  const source = ctx.ordered[index - 1]?.data;
  const sellAmount =
    source?.kind === "WALLET_ASSET"
      ? `${source.amount || "0"} ${sell?.symbol ?? ""}`
      : `From step ${index}`;
  const choices = session.catalog
    ? compatibleTradeAssets(sellToken, ctx.assets, session.catalog.tradePolicy)
    : [];
  const preview = ctx.previews.find((item) => item.nodeId === node.id);
  const routes = preview?.kind === "TRADE" ? (preview.routes ?? []) : [];
  const selected = ctx.routes[node.id];
  const stock = session.catalog?.bstocks.find(
    (item) => item.contract.toLowerCase() === data.buyToken?.toLowerCase(),
  );
  const gap = stock ? priceGapPercent(stock) : undefined;
  return (
    <>
      <div className={styles.swap}>
        <div className={styles.swapRow}>
          <span>You sell</span>
          <span className="mono">{sellAmount}</span>
        </div>
        <div className={styles.swapRow}>
          <span>You buy</span>
          <span className="mono">
            {selected
              ? `${formatDisplayAmount(formatUnits(BigInt(selected.buy.amount), buy?.decimals ?? 18))} ${buy?.symbol}`
              : (buy?.symbol ?? "—")}
          </span>
        </div>
        <span className={styles.swapArrow} aria-hidden="true">
          <svg
            width="13"
            height="13"
            viewBox="0 0 24 24"
            fill="none"
            stroke="#0F110E"
            strokeWidth="2.4"
            strokeLinecap="round"
          >
            <path d="M12 5v14M6 13l6 6 6-6" />
          </svg>
        </span>
      </div>
      <Section
        label={buy?.kind === "settlement" ? "Receive" : "Stock"}
        htmlFor={`buy-${node.id}`}
      >
        <TokenSelect
          id={`buy-${node.id}`}
          options={choices}
          value={data.buyToken}
          searchPlaceholder={
            buy?.kind === "settlement" ? "Search tokens" : "Search stocks"
          }
          onChange={(buyToken) => editor.updateNode(node.id, { buyToken })}
        />
        {stock ? (
          <div className={styles.stockCard}>
            <div className={styles.stockHead}>
              <span className={`mono ${styles.stockBadge}`}>
                {stock.underlyingTicker.slice(0, 2)}
              </span>
              <span className={styles.stockName}>
                <strong>{stock.tokenSymbol}</strong>
                <small>{stock.underlyingName}</small>
              </span>
              <span className={`mono ${styles.stockPrice}`}>
                {usd(stock.tokenPriceUsd)}
              </span>
            </div>
            {gap !== undefined ? (
              <div className={`mono ${styles.gapRow}`}>
                <span>
                  vs {stock.underlyingTicker} {usd(stock.referencePriceUsd)}
                </span>
                <span
                  style={{ color: gap < 0 ? "var(--bad)" : "var(--good-ink)" }}
                >
                  {gap >= 0 ? "+" : "−"}
                  {Math.abs(gap).toFixed(2)}% gap
                </span>
              </div>
            ) : null}
            {stock.marketStatus?.session ? (
              <span className={`mono ${styles.marketBadge}`}>
                <span
                  style={{
                    background:
                      stock.marketStatus.session === "regular"
                        ? "var(--good)"
                        : "#8A8F86",
                  }}
                />
                US market {sessionWord(stock.marketStatus.session)}
                {stock.marketStatus.session !== "regular" &&
                stock.marketStatus.nextOpenAtMs
                  ? ` · opens ${new Intl.DateTimeFormat("en-US", { weekday: "short", hour: "numeric", minute: "2-digit" }).format(stock.marketStatus.nextOpenAtMs)}`
                  : ""}
              </span>
            ) : null}
          </div>
        ) : null}
      </Section>
      <Section label="Route">
        {routes.length === 0 ? (
          <p className={styles.muted}>
            {preview?.state === "WAITING_FOR_UPSTREAM"
              ? "Routes are quoted once the previous step has settled."
              : ctx.walletReady
                ? "Fetching live routes…"
                : "Connect a wallet to see live routes."}
          </p>
        ) : (
          routes.map((route) => {
            const active = selected?.routeQuoteId === route.routeQuoteId;
            return (
              <button
                type="button"
                key={route.routeQuoteId}
                className={`${styles.option} ${active ? styles.optionActive : ""}`}
                onClick={() => session.selectRoute(node.id, route)}
              >
                <span className={styles.optionHead}>
                  <span
                    className={`${styles.radio} ${active ? styles.radioOn : ""}`}
                  />
                  <strong>
                    {route.executionMode === "SWAP"
                      ? "Instant swap"
                      : "Signed order"}
                  </strong>
                  <span className={`mono ${styles.optionVenue}`}>
                    {route.venue}
                  </span>
                </span>
                {active ? (
                  <span className={`mono ${styles.optionGrid}`}>
                    <span>You receive</span>
                    <span>
                      {formatDisplayAmount(
                        formatUnits(
                          BigInt(route.buy.amount),
                          buy?.decimals ?? 18,
                        ),
                      )}{" "}
                      {buy?.symbol}
                    </span>
                    <span>Price impact</span>
                    <span>
                      {route.priceImpactPercent
                        ? `${formatPositionAmount(route.priceImpactPercent)}%`
                        : "—"}
                    </span>
                    <span>Minimum received</span>
                    <span>Set when you sign</span>
                  </span>
                ) : null}
              </button>
            );
          })
        )}
      </Section>
    </>
  );
}

function SupplyFields({
  node,
  ctx,
  index,
}: {
  node: RecipeFlowNode;
  ctx: DescribeContext;
  index: number;
}) {
  if (node.data.kind !== "SUPPLY") return null;
  const token = assetFor(ctx.assets, inputToken(ctx, index));
  const preview = ctx.previews.find((item) => item.nodeId === node.id);
  const market = preview?.kind === "SUPPLY" ? preview.market : undefined;
  return (
    <>
      <div className={styles.venue}>
        <span className={styles.venueMark}>V</span>
        <span className={styles.stockName}>
          <strong>Venus</strong>
          <small>Lending market</small>
        </span>
        <span className={`mono ${styles.tag}`}>Venue</span>
      </div>
      <Section label="Amount">
        <div className={styles.boundAmount}>
          <span className="mono">All {token?.symbol ?? "tokens"}</span>
          <span className={`mono ${styles.fromTag}`}>from step {index}</span>
        </div>
      </Section>
      <div className={styles.collateral}>
        <span className={styles.stockName}>
          <strong>Use as collateral</strong>
          <small>Becomes collateral you can borrow against.</small>
        </span>
      </div>
      {market ? (
        <dl className={`mono ${styles.facts}`}>
          <dt>Supply APY</dt>
          <dd>{formatPositionAmount(market.supplyApyPercent)}%</dd>
          <dt>Remaining capacity</dt>
          <dd>
            {market.remainingSupplyCapacity == null
              ? "Unlimited"
              : `${formatPositionAmount(market.remainingSupplyCapacity)} ${market.token.symbol}`}
          </dd>
        </dl>
      ) : null}
    </>
  );
}

function BorrowFields({
  node,
  ctx,
  index,
}: {
  node: RecipeFlowNode;
  ctx: DescribeContext;
  index: number;
}) {
  const editor = useRecipeEditor();
  const session = useRecipeSession();
  if (node.data.kind !== "BORROW") return null;
  const data = node.data;
  const preview = ctx.previews.find((item) => item.nodeId === node.id);
  const capability =
    preview?.kind === "BORROW" ? preview.capability : undefined;
  const debtAssets = session.catalog
    ? borrowDebtAssets(ctx.assets, session.catalog.borrowPolicy)
    : [];
  const debt = assetFor(ctx.assets, data.debtToken);
  const collateral = assetFor(ctx.assets, inputToken(ctx, index));
  return (
    <>
      <Section
        label={`Borrow ${debt?.symbol ?? ""}`}
        htmlFor={`borrow-${node.id}`}
      >
        <div className={styles.borrowHead}>
          {capability ? (
            <button
              type="button"
              className={`mono ${styles.safeMax}`}
              onClick={() =>
                editor.updateNode(node.id, {
                  amount: capability.conservativeMaxAmount,
                })
              }
            >
              Safe max ·{" "}
              {formatPositionAmount(capability.conservativeMaxAmount)}
            </button>
          ) : null}
        </div>
        <div className={styles.amount}>
          <input
            id={`borrow-${node.id}`}
            className="mono"
            inputMode="decimal"
            value={data.amount}
            onChange={(event) =>
              editor.updateNode(node.id, { amount: event.target.value })
            }
          />
          <span className="mono">{debt?.symbol ?? ""}</span>
        </div>
        {debtAssets.length > 1 ? (
          <TokenSelect
            id={`debt-${node.id}`}
            options={debtAssets}
            value={data.debtToken}
            searchPlaceholder="Search tokens"
            onChange={(debtToken) => editor.updateNode(node.id, { debtToken })}
          />
        ) : null}
      </Section>
      <HealthGauge
        before={capability?.healthFactorBefore}
        after={capability?.healthFactorAfter}
      />
      {capability ? (
        <dl className={`mono ${styles.facts}`}>
          <dt>Collateral</dt>
          <dd>{collateral?.symbol ?? "—"}</dd>
          <dt>Borrow rate</dt>
          <dd>{formatPositionAmount(capability.borrowApyPercent)}% APY</dd>
          <dt>Available to borrow</dt>
          <dd>
            {formatPositionAmount(capability.availableBorrowLiquidity)}{" "}
            {capability.debt.symbol}
          </dd>
        </dl>
      ) : null}
      {capability?.stage !== "BORROW" ? (
        <div className={styles.note}>
          <span className={`mono ${styles.tag}`}>1 → 2</span>
          First enable collateral, then borrow. Two signatures.
        </div>
      ) : null}
      <p className={styles.small}>
        Venus looks at your whole account: every enabled collateral and every
        debt.
      </p>
    </>
  );
}

function RepayFields({
  node,
  ctx,
}: {
  node: RecipeFlowNode;
  ctx: DescribeContext;
}) {
  if (node.data.kind !== "REPAY") return null;
  const preview = ctx.previews.find((item) => item.nodeId === node.id);
  const capability = preview?.kind === "REPAY" ? preview.capability : undefined;
  return (
    <>
      <Section label="Repays your full debt">
        <div className={styles.boundAmount}>
          <span className="mono">
            {capability
              ? `${formatPositionAmount(capability.debtAmount)} ${capability.debt.symbol}`
              : "Full live debt"}
          </span>
          <span className={`mono ${styles.fromTag}`}>incl. interest</span>
        </div>
      </Section>
      {capability ? (
        <dl className={`mono ${styles.facts}`}>
          <dt>Wallet balance</dt>
          <dd>
            {formatPositionAmount(capability.walletBalance)}{" "}
            {capability.debt.symbol}
          </dd>
          <dt>Approval</dt>
          <dd>
            {capability.approvalRequired
              ? `Needed · ${formatPositionAmount(capability.approvalAmount)}`
              : "Already approved"}
          </dd>
        </dl>
      ) : null}
      <p className={styles.small}>
        Repaying in full avoids leftover interest that would block withdrawing
        your stock.
      </p>
    </>
  );
}

function WithdrawFields({
  node,
  ctx,
  index,
}: {
  node: RecipeFlowNode;
  ctx: DescribeContext;
  index: number;
}) {
  if (node.data.kind !== "WITHDRAW") return null;
  const token = assetFor(ctx.assets, inputToken(ctx, index));
  const preview = ctx.previews.find((item) => item.nodeId === node.id);
  const position = preview?.kind === "WITHDRAW" ? preview.position : undefined;
  return (
    <Section label="Withdraws all of your supplied stock">
      <div className={styles.boundAmount}>
        <span className="mono">
          {position
            ? `${formatPositionAmount(position.amount)} ${position.token.symbol}`
            : `All ${token?.symbol ?? ""}`}
        </span>
        <span className={`mono ${styles.fromTag}`}>
          {position ? formatUsd(position.valueUsd) : "to your wallet"}
        </span>
      </div>
    </Section>
  );
}

function PositionFields({
  node,
  ctx,
}: {
  node: RecipeFlowNode;
  ctx: DescribeContext;
}) {
  const editor = useRecipeEditor();
  const data = node.data;
  if (data.kind !== "SUPPLIED_POSITION" && data.kind !== "BORROWED_POSITION")
    return null;
  const positions =
    data.kind === "SUPPLIED_POSITION"
      ? ctx.suppliedPositions
      : ctx.borrowedPositions;
  return (
    <Section
      label={
        data.kind === "SUPPLIED_POSITION"
          ? "Supplied on Venus"
          : "Debt on Venus"
      }
    >
      {positions.length === 0 ? (
        <p className={styles.muted}>
          {ctx.walletReady
            ? data.kind === "SUPPLIED_POSITION"
              ? "No supplied stock yet. Buy a stock and supply it to Venus first."
              : "No Venus debt in this wallet."
            : "Connect a wallet to see your Venus positions."}
        </p>
      ) : (
        <div className={styles.tokenList}>
          {positions.map((position) => {
            const active =
              position.pool.toLowerCase() === data.pool?.toLowerCase() &&
              position.asset.token.contract.toLowerCase() ===
                data.token?.toLowerCase();
            return (
              <button
                type="button"
                key={`${position.pool}:${position.asset.token.contract}`}
                className={`${styles.tokenRow} ${active ? styles.tokenRowActive : ""}`}
                onClick={() =>
                  editor.updateNode(node.id, {
                    venue: position.venue,
                    pool: position.pool,
                    token: position.asset.token.contract,
                  })
                }
              >
                <TokenDot symbol={position.catalogAsset.symbol} />
                <span className={styles.tokenSymbol}>
                  {position.catalogAsset.symbol}
                </span>
                <span className={`mono ${styles.tokenBalance}`}>
                  {formatPositionAmount(position.asset.amount)} ·{" "}
                  {formatUsd(position.asset.valueUsd)}
                </span>
              </button>
            );
          })}
        </div>
      )}
    </Section>
  );
}

function Section({
  label,
  htmlFor,
  children,
}: {
  label: string;
  htmlFor?: string;
  children: ReactNode;
}) {
  return (
    <div className={styles.section}>
      {htmlFor ? (
        <label htmlFor={htmlFor} className={`mono ${styles.label}`}>
          {label}
        </label>
      ) : (
        <span className={`mono ${styles.label}`}>{label}</span>
      )}
      {children}
    </div>
  );
}

function balanceOf(asset: { balanceRaw?: bigint; decimals: number }): string {
  return asset.balanceRaw !== undefined
    ? `${formatDisplayAmount(formatUnits(asset.balanceRaw, asset.decimals))} available`
    : "—";
}

function heldAmount(asset: {
  balanceRaw?: bigint;
  decimals: number;
}): string | undefined {
  return asset.balanceRaw
    ? formatDisplayAmount(formatUnits(asset.balanceRaw, asset.decimals))
    : undefined;
}

function kindLabel(kind: RecipeFlowNode["data"]["kind"]): string {
  return kind.charAt(0) + kind.slice(1).toLowerCase().replace("_", " ");
}
