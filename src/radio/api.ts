import { logger } from "@/lib/log";
import { IS_MOBILE } from "@/util/platform";
import type { Station } from "./station";

/**
 * Client of the Radio Browser (radio-browser.info), a community list of internet radio stations
 * that is free to use and answers cross-origin requests, so the webview asks it directly. There
 * is no single host. The list of servers is asked for once, and a server that fails is skipped.
 */
const log = logger("radio");

const DISCOVERY = "https://all.api.radio-browser.info/json/servers";
/** Used when the list of servers cannot be fetched. */
const FALLBACK = [
  "https://de1.api.radio-browser.info",
  "https://nl1.api.radio-browser.info",
  "https://at1.api.radio-browser.info",
];
const MAX_HOSTS = 3;
const TIMEOUT_MS = 8_000;
/**
 * Stations that the player cannot use are dropped after the query, so more than wanted are
 * asked for to still fill the list.
 */
const OVERFETCH = 2;

export type Order = "clickcount" | "clicktrend";

export type Tag = { name: string; count: number };

type RawStation = {
  stationuuid: string;
  name: string;
  url_resolved: string;
  favicon: string;
  tags: string;
  country: string;
  bitrate: number;
  hls: number;
};

let hosts: Promise<string[]> | undefined;
/** The server that answered last, tried first next time. */
let preferred: string | undefined;

async function fetchJson<T>(url: string): Promise<T> {
  const abort = new AbortController();
  const timer = setTimeout(() => abort.abort(), TIMEOUT_MS);
  try {
    const res = await fetch(url, { signal: abort.signal, headers: { Accept: "application/json" } });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return (await res.json()) as T;
  } finally {
    clearTimeout(timer);
  }
}

const shuffled = <T>(list: T[]): T[] => [...list].sort(() => Math.random() - 0.5);

function discover(): Promise<string[]> {
  hosts ??= fetchJson<{ name: string }[]>(DISCOVERY)
    .then((list) => {
      const found = list.map((s) => `https://${s.name}`);
      return found.length > 0 ? shuffled(found) : FALLBACK;
    })
    .catch((e) => {
      log.warn("couldn't list the radio servers, using the built in ones:", e);
      return FALLBACK;
    });
  return hosts;
}

async function get<T>(path: string): Promise<T> {
  const all = await discover();
  const order = preferred ? [preferred, ...all.filter((h) => h !== preferred)] : all;
  let failure: unknown;
  for (const host of order.slice(0, MAX_HOSTS)) {
    try {
      const body = await fetchJson<T>(host + path);
      preferred = host;
      return body;
    } catch (e) {
      failure = e;
      if (host === preferred) preferred = undefined;
    }
  }
  throw failure;
}

/**
 * HLS needs a player the webviews do not all have. A phone refuses plain HTTP, since a release
 * build blocks cleartext traffic, so there only HTTPS streams are usable.
 */
const playable = (s: RawStation) =>
  !!s.name.trim() && !!s.url_resolved && s.hls !== 1 && (!IS_MOBILE || s.url_resolved.startsWith("https:"));

/** A chart lists the same station under several ids, and only the first of them is kept. */
function toStations(raw: RawStation[], limit: number): Station[] {
  const seen = new Set<string>();
  const out: Station[] = [];
  for (const s of raw) {
    const key = s.name.trim().toLowerCase();
    if (!playable(s) || seen.has(key)) continue;
    seen.add(key);
    out.push({
      id: s.stationuuid,
      name: s.name.trim(),
      country: s.country,
      bitrate: s.bitrate,
      tags: s.tags ? s.tags.split(",").filter(Boolean) : [],
      favicon: s.favicon || null,
    });
    if (out.length >= limit) break;
  }
  return out;
}

export type StationQuery = {
  limit: number;
  /** Most played of all time, or most played lately. */
  order?: Order;
  /** ISO 3166 code. */
  country?: string;
  tag?: string;
  name?: string;
};

export async function searchStations(q: StationQuery): Promise<Station[]> {
  const params = new URLSearchParams({
    order: q.order ?? "clickcount",
    reverse: "true",
    hidebroken: "true",
    limit: String(q.limit * OVERFETCH),
  });
  if (q.country) params.set("countrycode", q.country);
  if (q.name) params.set("name", q.name);
  if (q.tag) {
    params.set("tag", q.tag);
    params.set("tagExact", "true");
  }
  if (IS_MOBILE) params.set("is_https", "true");
  return toStations(await get<RawStation[]>(`/json/stations/search?${params}`), q.limit);
}

export async function topTags(limit: number): Promise<Tag[]> {
  const params = new URLSearchParams({ order: "stationcount", reverse: "true", hidebroken: "true", limit: String(limit) });
  const raw = await get<{ name: string; stationcount: number }[]>(`/json/tags?${params}`);
  return raw.filter((t) => t.name.trim()).map((t) => ({ name: t.name, count: t.stationcount }));
}

/**
 * The address of the stream of a station. It is asked for right before playing and not stored,
 * since the station may have moved, and asking is also how the Radio Browser counts a listener,
 * which is what its charts are made of.
 */
export async function resolveStream(id: string): Promise<string> {
  const res = await get<{ ok: boolean; url?: string; message?: string }>(`/json/url/${encodeURIComponent(id)}`);
  if (!res.ok || !res.url) throw new Error(res.message ?? "the station has no stream");
  return res.url;
}
