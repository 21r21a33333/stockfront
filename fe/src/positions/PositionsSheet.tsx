import { useEffect } from "react";
import { formatUnits } from "viem";
import { usePositionsSheet } from "@/app/positionsSheet";
import { navigate } from "@/app/route";
import {
  formatDisplayAmount,
  formatPositionAmount,
  formatUsd,
} from "@/recipe/format";
import { useRecipeSession } from "@/session/context";
import { useConnectModal } from "@/wallet/connectModal";
import { BlockIcon } from "@/ui/BlockIcon";
import { HealthGauge } from "@/ui/HealthGauge";
import styles from "./PositionsSheet.module.css";

export function PositionsSheet() {
  const { open, setOpen } = usePositionsSheet();
  const session = useRecipeSession();
  const openConnect = useConnectModal((state) => state.setOpen);

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) =>
      event.key === "Escape" && setOpen(false);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, setOpen]);

  const catalog = session.catalog;
  const stocks = session.assets.filter(
    (asset) => asset.kind === "bstock" && (asset.balanceRaw ?? 0n) > 0n,
  );
  const shareRatio = (address: string) =>
    catalog?.bstocks.find(
      (item) => item.contract.toLowerCase() === address.toLowerCase(),
    )?.tokenToShareRatio;
  const empty =
    stocks.length === 0 &&
    session.suppliedPositions.length === 0 &&
    session.borrowedPositions.length === 0;

  function use(action: () => void) {
    action();
    setOpen(false);
    navigate("builder");
  }

  return (
    <div
      className={styles.backdrop}
      data-open={open}
      aria-hidden={!open}
      onClick={() => setOpen(false)}
    >
      <aside
        className={styles.sheet}
        role="dialog"
        aria-modal="true"
        aria-label="Positions"
        onClick={(event) => event.stopPropagation()}
      >
        <div className={styles.head}>
          <div>
            <span className="eyebrow">Live · BNB Chain</span>
            <h2 className="disp">Positions</h2>
          </div>
          <button
            type="button"
            className={styles.close}
            aria-label="Close positions"
            onClick={() => setOpen(false)}
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
              <path d="M6 6l12 12M18 6L6 18" />
            </svg>
          </button>
        </div>

        {!session.walletReady ? (
          <div className={styles.emptyState}>
            <strong className="disp">No wallet connected.</strong>
            <p>Connect a wallet on BNB Chain to see your positions.</p>
            <button
              type="button"
              className={`lift ${styles.primary}`}
              onClick={() => {
                setOpen(false);
                openConnect(true);
              }}
            >
              Connect wallet
            </button>
          </div>
        ) : empty ? (
          <div className={styles.emptyState}>
            <strong className="disp">Nothing here yet.</strong>
            <p>Start with your first strategy.</p>
            <button
              type="button"
              className={`lift ${styles.primary}`}
              onClick={() => {
                setOpen(false);
                navigate("compose");
              }}
            >
              Compose a strategy
            </button>
          </div>
        ) : (
          <div className={styles.body}>
            <section className={styles.section}>
              <h3 className="mono">Stocks in wallet</h3>
              {stocks.length === 0 ? (
                <p className={styles.muted}>
                  No tokenized stocks in this wallet.
                </p>
              ) : null}
              {stocks.map((asset) => {
                const amount = formatUnits(
                  asset.balanceRaw ?? 0n,
                  asset.decimals,
                );
                const ratio = shareRatio(asset.address);
                const shares = ratio
                  ? Number(amount) * Number(ratio)
                  : undefined;
                return (
                  <div key={asset.address} className={styles.row}>
                    <BlockIcon kind="wallet" size={36} radius={10} />
                    <span className={styles.rowText}>
                      <strong>{asset.symbol}</strong>
                      <small>
                        {asset.name}
                        {shares !== undefined && Number.isFinite(shares)
                          ? ` · ${formatPositionAmount(String(shares))} shares`
                          : ""}
                      </small>
                    </span>
                    <span className={`mono ${styles.value}`}>
                      {formatDisplayAmount(amount)}
                    </span>
                  </div>
                );
              })}
            </section>

            <section className={styles.section}>
              <h3 className="mono">On Venus</h3>
              {session.suppliedPositions.map((position) => (
                <div
                  key={`${position.pool}:${position.asset.token.contract}`}
                  className={styles.row}
                >
                  <BlockIcon
                    kind="supplied"
                    size={36}
                    radius={10}
                    tint="var(--sky)"
                  />
                  <span className={styles.rowText}>
                    <strong>Supplied {position.catalogAsset.symbol}</strong>
                    <small>
                      {formatPositionAmount(position.asset.amount)} ·{" "}
                      {formatUsd(position.asset.valueUsd)}
                    </small>
                  </span>
                  <span className={styles.rowActions}>
                    <button
                      type="button"
                      onClick={() =>
                        use(() =>
                          session.startFromPosition(
                            "SUPPLIED_POSITION",
                            {
                              venue: position.venue,
                              pool: position.pool,
                              token: position.asset.token.contract,
                            },
                            "BORROW",
                          ),
                        )
                      }
                    >
                      Borrow
                    </button>
                    <button
                      type="button"
                      onClick={() =>
                        use(() =>
                          session.startFromPosition(
                            "SUPPLIED_POSITION",
                            {
                              venue: position.venue,
                              pool: position.pool,
                              token: position.asset.token.contract,
                            },
                            "WITHDRAW",
                          ),
                        )
                      }
                    >
                      Withdraw
                    </button>
                  </span>
                </div>
              ))}
              {session.borrowedPositions.map((position) => (
                <div
                  key={`${position.pool}:${position.asset.token.contract}`}
                  className={styles.row}
                >
                  <BlockIcon kind="borrow" size={36} radius={10} />
                  <span className={styles.rowText}>
                    <strong>{position.catalogAsset.symbol} debt</strong>
                    <small>
                      {formatPositionAmount(position.asset.amount)} ·{" "}
                      {formatUsd(position.asset.valueUsd)}
                    </small>
                  </span>
                  <span className={styles.rowActions}>
                    <button
                      type="button"
                      onClick={() =>
                        use(() =>
                          session.startFromPosition(
                            "BORROWED_POSITION",
                            {
                              venue: position.venue,
                              pool: position.pool,
                              token: position.asset.token.contract,
                            },
                            "REPAY",
                          ),
                        )
                      }
                    >
                      Repay
                    </button>
                  </span>
                </div>
              ))}
              {session.suppliedPositions.length === 0 &&
              session.borrowedPositions.length === 0 ? (
                <p className={styles.muted}>No supported Venus positions.</p>
              ) : null}
              {session.borrowedPositions.length > 0 &&
              session.borrowing?.healthFactor ? (
                <HealthGauge
                  before={session.borrowing.healthFactor}
                  after={session.borrowing.healthFactor}
                />
              ) : null}
            </section>

            <section className={styles.section}>
              <h3 className="mono">Baskets</h3>
              {session.basketPositions.map((position) => (
                <div key={position.contract} className={styles.row}>
                  <BlockIcon
                    kind="liquidity"
                    size={36}
                    radius={10}
                    tint="var(--sky)"
                  />
                  <span className={styles.rowText}>
                    <strong>{position.symbol}</strong>
                    <small>
                      {formatPositionAmount(position.tokenAmount)} ·{" "}
                      {formatUsd(position.valueUsd)}
                    </small>
                  </span>
                  <span className={styles.rowActions}>
                    <button
                      type="button"
                      onClick={() =>
                        use(() => session.startBasketSale(position))
                      }
                    >
                      Sell
                    </button>
                  </span>
                </div>
              ))}
              {session.basketPositions.length === 0 ? (
                <p className={styles.muted}>No index baskets.</p>
              ) : null}
            </section>

            <section className={styles.section}>
              <h3 className="mono">Earning</h3>
              {session.earnPositions
                .filter((position) => Number(position.valueUsd) > 0)
                .map((position) => (
                  <div key={position.positionId} className={styles.row}>
                    <BlockIcon
                      kind="earn"
                      size={36}
                      radius={10}
                      tint="var(--sky)"
                    />
                    <span className={styles.rowText}>
                      <strong>{position.protocolName}</strong>
                      <small>{formatUsd(position.valueUsd)}</small>
                    </span>
                    <span className={styles.rowActions}>
                      <button
                        type="button"
                        onClick={() =>
                          use(() => session.startEarnWithdrawal(position))
                        }
                      >
                        Withdraw
                      </button>
                    </span>
                  </div>
                ))}
              {session.earnPositions.every(
                (position) => !(Number(position.valueUsd) > 0),
              ) ? (
                <p className={styles.muted}>No earn positions.</p>
              ) : null}
            </section>
          </div>
        )}
      </aside>
    </div>
  );
}
