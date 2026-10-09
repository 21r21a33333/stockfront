import type { ReactNode } from "react";
import { Logo } from "@/ui/BlockIcon";
import { WalletButton } from "@/wallet/WalletButton";
import { usePositionsSheet } from "./positionsSheet";
import { navigate, type Route } from "./route";
import styles from "./AppHeader.module.css";

export function AppHeader({
  current,
  middle,
  actions,
  sticky,
  showNav = true,
  compactPositions,
}: {
  current?: Route;
  middle?: ReactNode;
  actions?: ReactNode;
  sticky?: boolean;
  showNav?: boolean;
  compactPositions?: boolean;
}) {
  const setOpen = usePositionsSheet((state) => state.setOpen);
  return (
    <header className={`${styles.header} ${sticky ? styles.sticky : ""}`}>
      <a href="#/" className={styles.brand} aria-label="Stockfront home">
        <Logo />
        <span className="disp">Stockfront</span>
      </a>
      {showNav ? (
        <nav aria-label="App" className={styles.nav}>
          {(["compose", "builder"] as const).map((route) => (
            <a
              key={route}
              href={`#/${route}`}
              aria-current={current === route ? "page" : undefined}
              onClick={(event) => {
                event.preventDefault();
                navigate(route);
              }}
            >
              {route === "compose" ? "Compose" : "Builder"}
            </a>
          ))}
        </nav>
      ) : null}
      {middle}
      <div className={styles.right}>
        {actions}
        {actions ? <span className={styles.divider} /> : null}
        {showNav ? (
          <button
            type="button"
            className={`lift ${compactPositions ? styles.iconButton : styles.positions}`}
            aria-label="Positions"
            onClick={() => setOpen(true)}
          >
            <LayersIcon />
            {compactPositions ? null : "Positions"}
          </button>
        ) : null}
        <span className={`mono ${styles.chain}`}>
          <span className={styles.chainDot} />
          BNB Chain
        </span>
        <WalletButton />
      </div>
    </header>
  );
}

export function LayersIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="#0F110E"
      strokeWidth="1.8"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M12 3l9 5-9 5-9-5 9-5z" />
      <path d="M3 13l9 5 9-5" />
    </svg>
  );
}
