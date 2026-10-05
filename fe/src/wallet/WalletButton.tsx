import { ChevronDown, Unplug } from "lucide-react";
import { shortAddress } from "@/recipe/assets";
import { useWalletSession } from "@/wallet/execution";
import styles from "./WalletButton.module.css";

export function WalletButton() {
  const wallet = useWalletSession();
  if (!wallet.isConnected || !wallet.address) {
    return (
      <button
        type="button"
        className={styles.connect}
        onClick={wallet.connect}
        disabled={wallet.connecting}
      >
        {wallet.connecting ? "Connecting…" : "Connect wallet"}
      </button>
    );
  }
  if (wallet.chainId !== 56) {
    return (
      <button
        type="button"
        className={styles.warning}
        onClick={wallet.switchToBnb}
      >
        Switch to BNB Chain
      </button>
    );
  }
  return (
    <button type="button" className={styles.wallet} onClick={wallet.disconnect}>
      <span className={styles.avatar}>{wallet.address.slice(2, 4)}</span>
      <span>
        <strong>{shortAddress(wallet.address)}</strong>
        <small>BNB Chain · connected</small>
      </span>
      <ChevronDown size={14} />
      <Unplug className={styles.unplug} size={13} />
    </button>
  );
}
