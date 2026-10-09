import { useState } from "react";
import { tokenImages } from "./tokenImages";
import styles from "./TokenSelect.module.css";

export function TokenDot({
  symbol,
  size = 26,
}: {
  symbol: string;
  size?: number;
}) {
  const [broken, setBroken] = useState(false);
  const image = tokenImages[symbol];
  if (image && !broken) {
    return (
      <img
        className={styles.dot}
        style={{ width: size, height: size }}
        src={image}
        alt=""
        onError={() => setBroken(true)}
      />
    );
  }
  const color =
    symbol === "USDT" ? "#2F9E7A" : symbol === "USDC" ? "#3A75C4" : "#0F110E";
  return (
    <span
      className={`mono ${styles.dot}`}
      style={{
        width: size,
        height: size,
        fontSize: Math.round(size * 0.42),
        background: color,
        color: color === "#0F110E" ? "#D5F56B" : "#FFFFFF",
      }}
    >
      {symbol === "USDT" ? "₮" : symbol === "USDC" ? "$" : symbol.slice(0, 2)}
    </span>
  );
}
