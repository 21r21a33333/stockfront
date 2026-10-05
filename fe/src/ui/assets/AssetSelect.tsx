import { ChevronDown, Search, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { formatUnits } from "viem";
import { AssetIdentity } from "./AssetIdentity";
import type { Address } from "viem";
import type { AssetOption } from "@/recipe/editor/types";
import styles from "./AssetSelect.module.css";

export function AssetSelect({
  assets,
  value,
  onChange,
}: {
  assets: AssetOption[];
  value?: Address;
  onChange(value: Address): void;
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const searchRef = useRef<HTMLInputElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const selected = assets.find(
    (asset) => asset.address.toLowerCase() === value?.toLowerCase(),
  );
  const filtered = useMemo(() => {
    const term = query.trim().toLowerCase();
    return assets
      .filter((asset) =>
        `${asset.symbol} ${asset.name} ${asset.address}`
          .toLowerCase()
          .includes(term),
      )
      .sort(
        (a, b) =>
          Number(b.balanceRaw !== undefined && b.balanceRaw > 0n) -
          Number(a.balanceRaw !== undefined && a.balanceRaw > 0n),
      );
  }, [assets, query]);

  useEffect(() => {
    if (!open) return;
    searchRef.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open]);

  function close() {
    setOpen(false);
    setQuery("");
    requestAnimationFrame(() => triggerRef.current?.focus());
  }

  function choose(asset: AssetOption) {
    onChange(asset.address);
    close();
  }

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        className={styles.field}
        aria-label="Select asset"
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => setOpen(true)}
      >
        <AssetIdentity asset={selected} />
        <ChevronDown size={14} />
      </button>
      {open
        ? createPortal(
            <div className={styles.backdrop} onMouseDown={close}>
              <div
                className={styles.dialog}
                role="dialog"
                aria-modal="true"
                aria-label="Select token"
                onMouseDown={(event) => event.stopPropagation()}
              >
                <header>
                  <strong>Select token</strong>
                  <button
                    type="button"
                    aria-label="Close token selector"
                    onClick={close}
                  >
                    <X size={19} />
                  </button>
                </header>
                <label className={styles.search}>
                  <Search size={18} />
                  <input
                    ref={searchRef}
                    aria-label="Search tokens"
                    placeholder="Search tokens or contract"
                    value={query}
                    onChange={(event) => setQuery(event.target.value)}
                  />
                </label>
                {!query.trim() ? (
                  <div className={styles.quickPicks}>
                    {assets
                      .filter((asset) => asset.kind === "settlement")
                      .slice(0, 3)
                      .map((asset) => (
                        <button
                          type="button"
                          key={asset.address}
                          onClick={() => choose(asset)}
                        >
                          <AssetIdentity asset={asset} />
                        </button>
                      ))}
                  </div>
                ) : null}
                <div className={styles.tokenList}>
                  {filtered.length ? (
                    filtered.map((asset) => (
                      <button
                        type="button"
                        className={styles.tokenRow}
                        key={asset.address}
                        onClick={() => choose(asset)}
                      >
                        <AssetIdentity asset={asset} />
                        <span className={styles.tokenDetails}>
                          <strong>{asset.name}</strong>
                          <small>{asset.symbol}</small>
                        </span>
                        {asset.balanceRaw !== undefined ? (
                          <span className={styles.balance}>
                            {formatBalance(asset.balanceRaw, asset.decimals)}
                          </span>
                        ) : null}
                      </button>
                    ))
                  ) : (
                    <p className={styles.noResults}>
                      No supported tokens match your search.
                    </p>
                  )}
                </div>
              </div>
            </div>,
            document.body,
          )
        : null}
    </>
  );
}

function formatBalance(raw: bigint, decimals: number): string {
  const [whole, fraction] = formatUnits(raw, decimals).split(".");
  return fraction
    ? `${whole}.${fraction.slice(0, 6).replace(/0+$/, "") || "0"}`
    : whole;
}
