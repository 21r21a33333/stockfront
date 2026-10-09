import { useState } from "react";
import { shortAddress } from "@/recipe/assets";
import { useWalletSession } from "@/wallet/execution";
import styles from "./WalletButton.module.css";

/** `glass` sits on the landing sky; `app` is the dark chip used inside the app header. */
export function WalletButton({
  variant = "app",
}: {
  variant?: "app" | "glass";
}) {
  const wallet = useWalletSession();
  const [open, setOpen] = useState(false);
  const missing = !wallet.hasInjectedWallet;

  if (!wallet.isConnected || !wallet.address) {
    return (
      <span className={styles.anchor}>
        <button
          type="button"
          className={`lift ${variant === "glass" ? styles.glass : styles.connect}`}
          onClick={() =>
            missing ? setOpen((value) => !value) : wallet.connect()
          }
          disabled={wallet.connecting}
        >
          {wallet.connecting ? "Connecting…" : "Connect wallet"}
        </button>
        {open && missing ? (
          <span role="status" className={styles.popover}>
            No browser wallet found. Install MetaMask or Rabby, set it to BNB
            Chain, then reload this page.
          </span>
        ) : null}
      </span>
    );
  }
  if (wallet.chainId !== 56) {
    return (
      <button
        type="button"
        className={`lift ${styles.warning}`}
        onClick={wallet.switchToBnb}
      >
        Switch to BNB Chain
      </button>
    );
  }
  return (
    <span className={styles.anchor}>
      <button
        type="button"
        className={`lift mono ${styles.wallet}`}
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
      >
        <span className={styles.avatar} aria-hidden="true" />
        {shortAddress(wallet.address)}
      </button>
      {open ? (
        <span className={styles.menu}>
          <button
            type="button"
            onClick={() => {
              void navigator.clipboard?.writeText(wallet.address ?? "");
              setOpen(false);
            }}
          >
            Copy address
          </button>
          <a
            href={`https://bscscan.com/address/${wallet.address}`}
            target="_blank"
            rel="noreferrer"
            onClick={() => setOpen(false)}
          >
            View on BscScan ↗
          </a>
          <button
            type="button"
            onClick={() => {
              wallet.disconnect();
              setOpen(false);
            }}
          >
            Disconnect
          </button>
        </span>
      ) : null}
    </span>
  );
}
