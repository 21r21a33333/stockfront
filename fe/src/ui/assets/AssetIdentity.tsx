import { useState } from "react";
import type { AssetOption } from "@/recipe/editor/types";
import styles from "./AssetIdentity.module.css";

export function AssetIdentity({ asset }: { asset?: AssetOption }) {
  const label = asset?.symbol ?? "Select";
  return (
    <span className={styles.identity}>
      {asset?.logoUrl ? (
        <AssetLogo key={asset.logoUrl} url={asset.logoUrl} label={label} />
      ) : (
        <span className={styles.fallback}>{label.slice(0, 1)}</span>
      )}
      <span>{label}</span>
    </span>
  );
}

function AssetLogo({ url, label }: { url: string; label: string }) {
  const [failed, setFailed] = useState(false);
  return failed ? (
    <span className={styles.fallback}>{label.slice(0, 1)}</span>
  ) : (
    <img
      src={url}
      alt=""
      className={styles.logo}
      onError={() => setFailed(true)}
    />
  );
}
