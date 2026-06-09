/** Shared utilities — one source of truth */

/** Format a number with k/M suffixes */
export function formatNum(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 1000) return (n / 1000).toFixed(1) + "K";
  return String(n);
}

/** Format token count with suffix */
export function formatTokenCount(n: number): string {
  return formatNum(n) + " tok";
}
