import { useCombobox } from "downshift";
import { useLayoutEffect, useMemo, useRef, useState } from "react";
import { TokenDot } from "./TokenDot";
import { filterTokens, type TokenOption } from "./tokenSearch";
import styles from "./TokenSelect.module.css";

const MENU_HEIGHT = 320;

export function TokenSelect({
  id,
  options,
  value,
  onChange,
  placeholder = "Select a token",
  searchPlaceholder = "Search by ticker or name",
}: {
  id: string;
  options: readonly TokenOption[];
  value?: string;
  onChange(address: `0x${string}`): void;
  placeholder?: string;
  searchPlaceholder?: string;
}) {
  const [query, setQuery] = useState("");
  const [isOpen, setIsOpen] = useState(false);
  const [upward, setUpward] = useState(false);
  const field = useRef<HTMLDivElement>(null);
  const items = useMemo(() => filterTokens(options, query), [options, query]);
  const selected =
    options.find(
      (option) => option.address.toLowerCase() === value?.toLowerCase(),
    ) ?? null;

  const combobox = useCombobox<TokenOption>({
    inputId: id,
    items,
    isOpen,
    defaultHighlightedIndex: 0,
    selectedItem: selected,
    inputValue: query,
    itemToString: (item) => (item ? `${item.symbol} · ${item.name}` : ""),
    itemToKey: (item) => item?.address,
    onIsOpenChange: (change) => {
      setIsOpen(Boolean(change.isOpen));
      setQuery("");
    },
    onInputValueChange: (change) => {
      if (change.type === useCombobox.stateChangeTypes.InputChange)
        setQuery(change.inputValue ?? "");
    },
    onSelectedItemChange: (change) => {
      if (change.selectedItem) onChange(change.selectedItem.address);
    },
  });

  // Side panels sit low on short screens; open upward instead of clipping.
  useLayoutEffect(() => {
    if (!isOpen || !field.current) return;
    const rect = field.current.getBoundingClientRect();
    const below = window.innerHeight - rect.bottom;
    setUpward(below < MENU_HEIGHT && rect.top > below);
  }, [isOpen]);

  return (
    <div className={styles.root}>
      <div
        ref={field}
        className={`${styles.field} ${isOpen ? styles.fieldOpen : ""}`}
      >
        {selected && !isOpen ? (
          <TokenDot symbol={selected.symbol} />
        ) : (
          <svg
            className={styles.search}
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            aria-hidden="true"
          >
            <circle cx="11" cy="11" r="7" />
            <path d="M20 20l-3.5-3.5" />
          </svg>
        )}
        <input
          {...combobox.getInputProps({
            placeholder: isOpen
              ? searchPlaceholder
              : selected
                ? `${selected.symbol} · ${selected.name}`
                : placeholder,
            className: selected && !isOpen ? styles.inputSelected : undefined,
            onFocus: () => {
              if (!isOpen) combobox.openMenu();
            },
          })}
        />
        <button
          type="button"
          className={styles.toggle}
          aria-label="Show options"
          {...combobox.getToggleButtonProps()}
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            <path d="M6 9l6 6 6-6" />
          </svg>
        </button>
      </div>
      <ul
        className={`${styles.menu} ${upward ? styles.menuUp : ""}`}
        hidden={!isOpen}
        {...combobox.getMenuProps()}
      >
        {isOpen && items.length === 0 ? (
          <li className={styles.none}>No match for “{query.trim()}”</li>
        ) : null}
        {isOpen
          ? items.map((item, index) => (
              <li
                key={item.address}
                className={`${styles.option} ${combobox.highlightedIndex === index ? styles.optionActive : ""}`}
                {...combobox.getItemProps({ item, index })}
              >
                <TokenDot symbol={item.symbol} />
                <span className={styles.optionText}>
                  <strong>{item.symbol}</strong>
                  <small>{item.name}</small>
                </span>
                {item.detail ? (
                  <span className={`mono ${styles.detail}`}>{item.detail}</span>
                ) : null}
                {item.address === selected?.address ? (
                  <svg
                    className={styles.check}
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2.4"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    aria-hidden="true"
                  >
                    <path d="M5 12.5l4.5 4.5L19 7.5" />
                  </svg>
                ) : null}
              </li>
            ))
          : null}
      </ul>
    </div>
  );
}
