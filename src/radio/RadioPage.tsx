import { useEffect, useState } from "react";
import { Box, Typography } from "@mui/material";
import { useCached } from "@/browse/useCached";
import CountryHint from "@/explore/CountryHint";
import { COUNTRIES, useCountry, WORLDWIDE } from "@/lib/country";
import { GUTTER } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import { searchStations, topTags } from "./api";
import RadioSearch from "./RadioSearch";
import Section from "./Section";
import StationGrid from "./StationGrid";
import StationShelf from "./StationShelf";
import TagGrid from "./TagGrid";

const SHELF = 20;
const CHART = 20;
const TAGS = 40;
const SEARCH_LIMIT = 60;
/** Every search is a request to a public server, so typing is given a moment to settle. */
const SEARCH_DELAY_MS = 400;
const MIN_QUERY = 2;

/** Stations are looked up by the country that is set, and by none when it is worldwide. */
function useCountryCode() {
  const [country] = useCountry();
  const code = country === WORLDWIDE ? "" : country;
  const label = COUNTRIES.find((c) => c.value === code)?.label ?? "";
  return { code, label };
}

const NONE = "Stations are not available right now.";

function Browse() {
  const { code, label } = useCountryCode();
  const popular = useCached(code ? `radio:popular:${code}` : null, () => searchStations({ country: code, limit: SHELF }), []);
  const local = useCached(code ? `radio:trending:${code}` : null, () => searchStations({ country: code, order: "clicktrend", limit: CHART }), []);
  const world = useCached("radio:trending", () => searchStations({ order: "clicktrend", limit: CHART }), []);
  const tags = useCached("radio:tags", () => topTags(TAGS), []);

  return (
    <>
      <CountryHint />
      {code && (
        <>
          <StationShelf title={`Popular in ${label}`} stations={popular} empty={NONE} />
          <Section title={`Trending in ${label}`} empty={local?.length === 0 ? NONE : undefined}>
            <StationGrid stations={local} numbered />
          </Section>
        </>
      )}
      <Section title="Trending Worldwide" empty={world?.length === 0 ? NONE : undefined}>
        <StationGrid stations={world} numbered />
      </Section>
      <Section title="Browse by Tag" empty={tags?.length === 0 ? NONE : undefined}>
        <TagGrid tags={tags} />
      </Section>
    </>
  );
}

function Results({ query }: { query: string }) {
  const found = useCached(`radio:search:${query.toLowerCase()}`, () => searchStations({ name: query, limit: SEARCH_LIMIT }), []);
  return (
    <Section title={`Results for "${query}"`} empty={found?.length === 0 ? "No stations match this search." : undefined}>
      <StationGrid stations={found} />
    </Section>
  );
}

export default function RadioPage() {
  const [text, setText] = useState("");
  const [query, setQuery] = useState("");

  useEffect(() => {
    const timer = setTimeout(() => setQuery(text.trim()), SEARCH_DELAY_MS);
    return () => clearTimeout(timer);
  }, [text]);

  const searching = query.length >= MIN_QUERY;

  return (
    <Box sx={{ pt: IS_MOBILE ? 2 : 3, pb: 2, marginBottom: 1 }}>
      {/* the field sits in the middle of the row on desktop, and under the title on a narrow screen */}
      <Box
        sx={{
          display: "grid", alignItems: "center", rowGap: 2, columnGap: 2, px: GUTTER, pb: 3,
          gridTemplateColumns: IS_MOBILE ? "minmax(0, 1fr)" : "1fr minmax(0, 360px) 1fr",
        }}
      >
        <Typography variant={IS_MOBILE ? "h5" : "h4"} sx={{ fontWeight: 800, lineHeight: "40px" }}>Radio</Typography>
        <RadioSearch value={text} onChange={setText} />
      </Box>
      {searching ? <Results query={query} /> : <Browse />}
    </Box>
  );
}
