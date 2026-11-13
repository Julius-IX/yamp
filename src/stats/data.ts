import { COMMANDS } from "@/lib/commands";
import { isRadio } from "@/radio/station";
import { dayOf, flushStats, STATS_PREFIX, type TrackStat } from "./record";

/**
 * Reads the stored days and turns them into what the statistics screen shows. Everything is
 * worked out here from the raw days and not in the backend, which keeps the stored shape simple
 * and the screen free to show a new figure without a migration.
 */

export type Day = { day: string; tracks: Record<string, TrackStat> };

export type Range = "week" | "month" | "year" | "all";
export const RANGES: { value: Range; label: string }[] = [
  { value: "week", label: "Week" },
  { value: "month", label: "Month" },
  { value: "year", label: "Year" },
  { value: "all", label: "All time" },
];

/** The days of every device, with a day that more than one device wrote added up. */
export async function loadDays(): Promise<Day[]> {
  // what was heard in this session may not be written yet
  await flushStats();
  const merged = new Map<string, Record<string, TrackStat>>();
  for (const { key, value } of await COMMANDS.values.list({ prefix: STATS_PREFIX })) {
    // stats:<day>:<device>
    const day = key.split(":")[1];
    let doc: { tracks?: Record<string, TrackStat> };
    try {
      doc = JSON.parse(value);
    } catch {
      continue;
    }
    if (!day || !doc.tracks) continue;
    const into = merged.get(day) ?? {};
    merged.set(day, into);
    for (const [id, t] of Object.entries(doc.tracks)) {
      const have = into[id];
      if (have) {
        have.secs += t.secs;
        have.plays += t.plays;
      } else {
        into[id] = { ...t };
      }
    }
  }
  return [...merged].map(([day, tracks]) => ({ day, tracks })).sort((a, b) => (a.day < b.day ? -1 : 1));
}

export type TrackEntry = TrackStat & { id: string };
export type ArtistEntry = { name: string; thumbnail: string | null; secs: number; plays: number };
export type Bucket = { label: string; title: string; secs: number };

export type Summary = {
  secs: number;
  plays: number;
  uniqueTracks: number;
  uniqueArtists: number;
  /** Seconds per day over the range, as an average. */
  dailyAverage: number;
  tracks: TrackEntry[];
  artists: ArtistEntry[];
  stations: TrackEntry[];
  radioSecs: number;
  buckets: Bucket[];
  /** Seconds per weekday, Monday first. */
  weekdays: number[];
  best: { title: string; secs: number } | null;
};

const parse = (day: string) => {
  const [y, m, d] = day.split("-").map(Number);
  return new Date(y, m - 1, d);
};

const RANGE_DAYS: Record<Exclude<Range, "all">, number> = { week: 7, month: 30, year: 365 };

/** The first day of the range, as a day string that compares in order with the stored ones. */
function startOf(range: Range): string {
  if (range === "all") return "";
  const t = new Date();
  return dayOf(new Date(t.getFullYear(), t.getMonth(), t.getDate() - RANGE_DAYS[range] + 1).getTime());
}

const longDay = (d: Date) => d.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" });
const monthName = (d: Date) => d.toLocaleDateString(undefined, { month: "long", year: "numeric" });

function dailyBuckets(range: "week" | "month", secsOf: Map<string, number>): Bucket[] {
  const count = RANGE_DAYS[range];
  const t = new Date();
  const out: Bucket[] = [];
  for (let i = count - 1; i >= 0; i--) {
    const d = new Date(t.getFullYear(), t.getMonth(), t.getDate() - i);
    const label = range === "week" ? d.toLocaleDateString(undefined, { weekday: "short" }) : i % 5 === 0 || i === count - 1 ? String(d.getDate()) : "";
    out.push({ label, title: longDay(d), secs: secsOf.get(dayOf(d.getTime())) ?? 0 });
  }
  return out;
}

/** Months, or years once there are too many months to draw as bars. */
function longBuckets(range: "year" | "all", days: Day[]): Bucket[] {
  const now = new Date();
  let first = new Date(now.getFullYear(), now.getMonth() - 11, 1);
  if (range === "all" && days.length > 0) {
    const f = parse(days[0].day);
    first = new Date(f.getFullYear(), f.getMonth(), 1);
  }
  const months = (now.getFullYear() - first.getFullYear()) * 12 + now.getMonth() - first.getMonth() + 1;
  const byYear = months > 36;
  const per = new Map<string, number>();
  for (const { day, tracks } of days) {
    const d = parse(day);
    const key = byYear ? String(d.getFullYear()) : `${d.getFullYear()}-${d.getMonth()}`;
    const secs = Object.values(tracks).reduce((n, t) => n + t.secs, 0);
    per.set(key, (per.get(key) ?? 0) + secs);
  }
  const out: Bucket[] = [];
  if (byYear) {
    for (let y = first.getFullYear(); y <= now.getFullYear(); y++) {
      out.push({ label: String(y), title: String(y), secs: per.get(String(y)) ?? 0 });
    }
    return out;
  }
  for (let i = 0; i < months; i++) {
    const d = new Date(first.getFullYear(), first.getMonth() + i, 1);
    out.push({
      label: d.toLocaleDateString(undefined, { month: "short" }),
      title: monthName(d),
      secs: per.get(`${d.getFullYear()}-${d.getMonth()}`) ?? 0,
    });
  }
  return out;
}

export function summarize(all: Day[], range: Range): Summary {
  const from = startOf(range);
  const days = all.filter((d) => d.day >= from);

  const tracks = new Map<string, TrackEntry>();
  const artists = new Map<string, ArtistEntry & { top: number }>();
  const perDay = new Map<string, number>();
  const weekdays = [0, 0, 0, 0, 0, 0, 0];
  let radioSecs = 0;
  let secs = 0;

  for (const { day, tracks: heard } of days) {
    const weekday = (parse(day).getDay() + 6) % 7;
    for (const [id, t] of Object.entries(heard)) {
      secs += t.secs;
      perDay.set(day, (perDay.get(day) ?? 0) + t.secs);
      weekdays[weekday] += t.secs;
      if (isRadio(id)) radioSecs += t.secs;
      const have = tracks.get(id);
      if (have) {
        have.secs += t.secs;
        have.plays += t.plays;
      } else {
        tracks.set(id, { ...t, id });
      }
    }
  }

  const songs: TrackEntry[] = [];
  const stations: TrackEntry[] = [];
  for (const t of tracks.values()) (isRadio(t.id) ? stations : songs).push(t);
  const byTime = (a: { secs: number }, b: { secs: number }) => b.secs - a.secs;
  songs.sort(byTime);
  stations.sort(byTime);

  // an artist takes the cover of the track of theirs that was heard the most
  for (const t of songs) {
    const have = artists.get(t.artist);
    if (have) {
      have.secs += t.secs;
      have.plays += t.plays;
      if (t.secs > have.top) [have.top, have.thumbnail] = [t.secs, t.thumbnail];
    } else {
      artists.set(t.artist, { name: t.artist, thumbnail: t.thumbnail, secs: t.secs, plays: t.plays, top: t.secs });
    }
  }

  // days elapsed from a given day through today, inclusive (calendar days, DST-safe)
  const daysSince = (d: Date) => {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    return Math.round((today.getTime() - d.getTime()) / 86_400_000) + 1;
  };

  // divide by the days you've actually had data for, capped at the range length
  const spanDays = days.length
    ? Math.max(1, Math.min(daysSince(parse(days[0].day)), range === "all" ? Infinity : RANGE_DAYS[range]))
    : 1;

  const bestDay = [...perDay].sort((a, b) => b[1] - a[1])[0];

  return {
    secs,
    plays: songs.reduce((n, t) => n + t.plays, 0),
    uniqueTracks: songs.length,
    uniqueArtists: artists.size,
    dailyAverage: secs / spanDays,
    tracks: songs,
    artists: [...artists.values()].sort(byTime),
    stations,
    radioSecs,
    buckets: range === "week" || range === "month" ? dailyBuckets(range, perDay) : longBuckets(range, days),
    weekdays,
    best: bestDay ? { title: longDay(parse(bestDay[0])), secs: bestDay[1] } : null,
  };
}

/** Days in a row with listening, up to today. Today may still be empty without breaking it. */
export function streak(all: Day[]): number {
  const heard = new Set(all.filter((d) => Object.keys(d.tracks).length > 0).map((d) => d.day));
  const t = new Date();
  let offset = heard.has(dayOf(t.getTime())) ? 0 : 1;
  let n = 0;
  while (heard.has(dayOf(new Date(t.getFullYear(), t.getMonth(), t.getDate() - offset).getTime()))) {
    n++;
    offset++;
  }
  return n;
}

/** "3h 05m" from an hour on, "42 min" below it. */
export function duration(secs: number): string {
  const mins = Math.round(secs / 60);
  if (mins < 60) return `${mins} min`;
  const h = Math.floor(mins / 60);
  const m = mins % 60;
  return h >= 100 ? `${h}h` : `${h}h ${String(m).padStart(2, "0")}m`;
}
