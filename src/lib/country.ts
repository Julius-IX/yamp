import { useSetting } from "./settings";

/** What the charts of a country are asked for as when none is chosen. */
export const WORLDWIDE = "ZZ";

/** The countries that YouTube Music has a chart for, as ISO 3166 codes. */
const CODES = [
  "AE", "AR", "AT", "AU", "BE", "BO", "BR", "CA", "CH", "CL", "CO", "CR", "CZ", "DE", "DK", "DO", "EC", "EE", "EG", "ES",
  "FI", "FR", "GB", "GT", "HN", "HU", "ID", "IE", "IL", "IN", "IS", "IT", "JP", "KE", "KR", "LU", "MX", "NG", "NI", "NL",
  "NO", "NZ", "PA", "PE", "PL", "PT", "PY", "RO", "RS", "RU", "SA", "SE", "SV", "TR", "TZ", "UA", "UG", "US", "UY", "ZA", "ZW",
];

// a webview without Intl.DisplayNames shows the codes instead
const names = (() => {
  try {
    return new Intl.DisplayNames(["en"], { type: "region" });
  } catch {
    return null;
  }
})();

/** Worldwide first, then the countries by name. */
export const COUNTRIES: { value: string; label: string }[] = [
  { value: WORLDWIDE, label: "Worldwide" },
  ...CODES.map((value) => ({ value, label: names?.of(value) ?? value })).sort((a, b) => a.label.localeCompare(b.label)),
];

const isCountry = (v: unknown): v is string => typeof v === "string" && (v === WORLDWIDE || CODES.includes(v));

/** The chosen country, or an empty string until one is. Worldwide is a choice as well. */
export const useCountry = () => useSetting<string>("country", "", isCountry);
