export interface TokenOption {
  address: `0x${string}`;
  symbol: string;
  name: string;
  detail?: string;
}

/** Symbol prefix matches rank above symbol or name substrings, so "nv" puts NVDAB first. */
export function filterTokens<T extends TokenOption>(
  options: readonly T[],
  query: string,
): T[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return [...options];
  const rank = (option: T) => {
    const symbol = option.symbol.toLowerCase();
    if (symbol.startsWith(needle)) return 0;
    if (symbol.includes(needle)) return 1;
    if (option.name.toLowerCase().includes(needle)) return 2;
    return undefined;
  };
  return options
    .map((option) => ({ option, score: rank(option) }))
    .filter(
      (entry): entry is { option: T; score: number } =>
        entry.score !== undefined,
    )
    .sort((a, b) => a.score - b.score)
    .map((entry) => entry.option);
}
