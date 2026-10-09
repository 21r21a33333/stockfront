import { useEffect, useRef, useState } from "react";
import { UserRejectedRequestError } from "viem";
import { useAccount, useConnect, type Connector } from "wagmi";
import { ArrowUpRight } from "@/ui/BlockIcon";
import { stockfrontChain } from "./config";
import { useConnectModal } from "./connectModal";
import styles from "./ConnectWalletModal.module.css";

const installLinks = [
  { name: "MetaMask", href: "https://metamask.io/download/" },
  { name: "Rabby", href: "https://rabby.io/" },
  { name: "Binance Wallet", href: "https://www.binance.com/en/web3wallet" },
];

type Attempt =
  | { status: "idle" }
  | { status: "pending"; connector: Connector }
  | { status: "failed"; connector: Connector; message: string };

export function ConnectWalletModal() {
  const { open, setOpen } = useConnectModal();
  const { isConnected } = useAccount();
  const { connectors, connectAsync } = useConnect();
  const [attempt, setAttempt] = useState<Attempt>({ status: "idle" });
  const dialog = useRef<HTMLDivElement>(null);

  // EIP-6963 announces each installed wallet by name; the generic injected
  // connector only matters when a wallet injects without announcing itself.
  const announced = connectors.filter((item) => item.id !== "injected");
  const fallback = connectors.find((item) => item.id === "injected");
  const wallets =
    announced.length > 0
      ? announced
      : fallback && typeof window !== "undefined" && window.ethereum
        ? [fallback]
        : [];

  useEffect(() => {
    if (open && isConnected) setOpen(false);
  }, [open, isConnected, setOpen]);

  useEffect(() => {
    if (!open) {
      setAttempt({ status: "idle" });
      return;
    }
    const previous = document.activeElement as HTMLElement | null;
    dialog.current
      ?.querySelector<HTMLElement>("button:not(:disabled), a[href]")
      ?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
      if (event.key !== "Tab" || !dialog.current) return;
      const focusable = [
        ...dialog.current.querySelectorAll<HTMLElement>(
          "button:not(:disabled), a[href]",
        ),
      ];
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last?.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      previous?.focus();
    };
  }, [open, setOpen]);

  async function connect(connector: Connector) {
    setAttempt({ status: "pending", connector });
    try {
      await connectAsync({ connector, chainId: stockfrontChain.id });
    } catch (error) {
      setAttempt({
        status: "failed",
        connector,
        message:
          error instanceof UserRejectedRequestError ||
          (error as { code?: number }).code === 4001
            ? "Request cancelled in your wallet."
            : ((error as { shortMessage?: string }).shortMessage ??
              "Couldn't connect. Try again."),
      });
    }
  }

  return (
    <div
      className={styles.backdrop}
      data-open={open}
      aria-hidden={!open}
      onClick={() => setOpen(false)}
    >
      <div
        ref={dialog}
        className={styles.dialog}
        role="dialog"
        aria-modal="true"
        aria-labelledby="connect-wallet-title"
        onClick={(event) => event.stopPropagation()}
      >
        <div className={styles.head}>
          <div>
            <span className="eyebrow">BNB Chain</span>
            <h2 id="connect-wallet-title" className="disp">
              Connect a wallet
            </h2>
          </div>
          <button
            type="button"
            className={styles.close}
            aria-label="Close"
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

        {wallets.length > 0 ? (
          <>
            <p className={styles.lead}>
              Choose the wallet you want to sign strategies with.
            </p>
            <ul className={styles.list}>
              {wallets.map((connector) => {
                const pending =
                  attempt.status === "pending" &&
                  attempt.connector.uid === connector.uid;
                const failed =
                  attempt.status === "failed" &&
                  attempt.connector.uid === connector.uid;
                return (
                  <li key={connector.uid}>
                    <button
                      type="button"
                      className={`lift ${styles.wallet} ${pending ? styles.walletPending : ""}`}
                      disabled={attempt.status === "pending"}
                      onClick={() => void connect(connector)}
                    >
                      <WalletIcon connector={connector} />
                      <span className={styles.walletText}>
                        <strong>
                          {connector.id === "injected"
                            ? "Browser wallet"
                            : connector.name}
                        </strong>
                        <small className="mono">
                          {pending
                            ? "Approve in your wallet…"
                            : failed
                              ? attempt.message
                              : "Detected"}
                        </small>
                      </span>
                      {pending ? (
                        <span className={`spinner ${styles.spin}`} />
                      ) : (
                        <span className={`mono ${styles.go}`}>
                          {failed ? "Retry" : "Connect"}
                        </span>
                      )}
                    </button>
                  </li>
                );
              })}
            </ul>
          </>
        ) : (
          <>
            <p className={styles.lead}>
              No browser wallet found. Install one, set it to BNB Chain, then
              reload this page.
            </p>
            <ul className={styles.list}>
              {installLinks.map((link) => (
                <li key={link.name}>
                  <a
                    className={`lift ${styles.wallet}`}
                    href={link.href}
                    target="_blank"
                    rel="noreferrer"
                  >
                    <span className={`mono ${styles.letter}`}>
                      {link.name.charAt(0)}
                    </span>
                    <span className={styles.walletText}>
                      <strong>{link.name}</strong>
                      <small className="mono">Install</small>
                    </span>
                    <ArrowUpRight size={14} />
                  </a>
                </li>
              ))}
            </ul>
          </>
        )}

        <p className={`mono ${styles.foot}`}>
          Stockfront never holds your keys. Every step is signed in your wallet.
        </p>
      </div>
    </div>
  );
}

function WalletIcon({ connector }: { connector: Connector }) {
  return connector.icon ? (
    <img className={styles.icon} src={connector.icon} alt="" />
  ) : (
    <span className={`mono ${styles.letter}`}>
      {connector.id === "injected" ? "W" : connector.name.charAt(0)}
    </span>
  );
}
