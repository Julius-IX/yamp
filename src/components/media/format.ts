/** Seconds as m:ss. Providers do not always know the length. */
export function duration(secs: number | null): string {
  if (secs == null) return "-:--";
  const s = Math.round(secs);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}
