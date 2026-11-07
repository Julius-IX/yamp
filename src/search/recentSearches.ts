import { getSetting, setSetting, useSetting } from "@/lib/settings";

const KEY = "recentSearches";
const MAX = 8;

const isStrings = (v: unknown): v is string[] => Array.isArray(v) && v.every((s) => typeof s === "string");

// A stable fallback, a new array on every read would make the hook re-render forever.
const EMPTY: string[] = [];

export const useRecentSearches = () => useSetting<string[]>(KEY, EMPTY, isStrings)[0];

/** Newest first. Searching the same text again moves it to the front instead of listing it twice. */
export function addRecentSearch(query: string) {
  const rest = getSetting(KEY, EMPTY, isStrings).filter((q) => q.toLowerCase() !== query.toLowerCase());
  setSetting(KEY, [query, ...rest].slice(0, MAX));
}

export const clearRecentSearches = () => setSetting(KEY, EMPTY);
