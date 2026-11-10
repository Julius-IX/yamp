/** Seconds as m:ss. Providers do not always know the length. */
export function duration(secs: number | null): string {
  if (secs == null) return "-:--";
  const s = Math.round(secs);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

/**
 * The summed length of tracks as "2 hr 5 min", or "45 min", or "3 min" under an hour. Tracks
 * without a length add nothing, so the result is empty when none of them has one. Seconds are
 * dropped on purpose, a total of a whole list is read at a glance.
 */
export function playtime(tracks: { durationSecs: number | null }[]): string {
  const minutes = Math.round(tracks.reduce((sum, t) => sum + (t.durationSecs ?? 0), 0) / 60);
  if (minutes <= 0) return "";
  const [h, m] = [Math.floor(minutes / 60), minutes % 60];
  return h === 0 ? `${m} min` : m === 0 ? `${h} hr` : `${h} hr ${m} min`;
}
