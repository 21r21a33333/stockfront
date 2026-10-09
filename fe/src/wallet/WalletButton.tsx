import { useEffect, useRef, useState } from "react";
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
  const [copied, setCopied] = useState(false);
  const anchor = useRef<HTMLSpanElement>(null);
  const missing = !wallet.hasInjectedWallet;

  useEffect(() => {
    if (!open) return;
    const onPointer = (event: PointerEvent) => {
      if (!anchor.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) =>
      event.key === "Escape" && setOpen(false);
    document.addEventListener("pointerdown", onPointer);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointer);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => {
      setCopied(false);
      setOpen(false);
    }, 900);
    return () => window.clearTimeout(timer);
  }, [copied]);

  if (!wallet.isConnected || !wallet.address) {
    return (
      <span ref={anchor} className={styles.anchor}>
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
    <span ref={anchor} className={styles.anchor}>
      <button
        type="button"
        className={`lift mono ${styles.wallet}`}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
      >
        <span className={styles.avatar} aria-hidden="true" />
        {shortAddress(wallet.address)}
      </button>
      {open ? (
        <span className={styles.menu} role="menu">
          <span className={`mono ${styles.menuAddress}`}>
            {shortAddress(wallet.address)}
          </span>
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              void navigator.clipboard?.writeText(wallet.address ?? "");
              setCopied(true);
            }}
          >
            {copied ? "Copied ✓" : "Copy address"}
          </button>
          <a
            href={`https://bscscan.com/address/${wallet.address}`}
            target="_blank"
            rel="noreferrer"
            role="menuitem"
            onClick={() => setOpen(false)}
          >
            View on BscScan ↗
          </a>
          <button
            type="button"
            role="menuitem"
            className={styles.danger}
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
