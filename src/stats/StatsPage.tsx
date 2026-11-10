import { useEffect, useMemo, useState, type ReactNode } from "react";
import { alpha, Box, Button, IconButton, Paper, ToggleButton, ToggleButtonGroup, Typography } from "@mui/material";
import ArrowBackIcon from "@mui/icons-material/ArrowBack";
import { useNavigate } from "react-router-dom";
import { logger } from "@/lib/log";
import { isBoolean, useSetting } from "@/lib/settings";
import { SYNCED_EVENT } from "@/lib/sync";
import Cover from "@/player/Cover";
import { faint, GUTTER, panel, sectionTitle, subdued } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import { duration, loadDays, RANGES, streak, summarize, type Bucket, type Day, type Range } from "./data";

const log = logger("stats");

/** Rows shown in a list before it is opened up, and the most it opens up to. */
const SHOWN = 8;
const SHOWN_MAX = 25;

function Panel({ title, children }: { title: string; children: ReactNode }) {
  return (
    <Paper variant="outlined" sx={{ p: { xs: 2, md: 3 }, ...panel }}>
      <Typography sx={{ ...sectionTitle, fontSize: "1.15rem", mb: 2 }}>{title}</Typography>
      {children}
    </Paper>
  );
}

function StatCard({ label, value, note }: { label: string; value: string; note?: string }) {
  return (
    <Paper variant="outlined" sx={{ p: 2, ...panel }}>
      <Typography sx={{ fontSize: 12, letterSpacing: 1.2, textTransform: "uppercase", ...faint }}>{label}</Typography>
      <Typography sx={{ fontSize: { xs: 26, md: 32 }, fontWeight: 800, lineHeight: 1.2, color: "primary.main" }}>{value}</Typography>
      {note && <Typography variant="body2" sx={subdued}>{note}</Typography>}
    </Paper>
  );
}

/**
 * Bars that scale to the highest one. A bar is tapped or hovered to read its value, since a
 * phone has no hover and the labels under thirty bars have no room for numbers.
 */
function BarChart({ buckets, height = 150, empty }: { buckets: Bucket[]; height?: number; empty: string }) {
  const [active, setActive] = useState<number | null>(null);
  const max = Math.max(...buckets.map((b) => b.secs), 1);
  const dense = buckets.length > 14;
  const shown = active !== null ? buckets[active] : null;
  const total = buckets.reduce((n, b) => n + b.secs, 0);

  return (
    <Box>
      <Typography variant="body2" sx={{ ...subdued, minHeight: 24 }}>
        {total === 0 ? empty : shown ? `${shown.title}, ${duration(shown.secs)}` : "Tap or hover a bar for the time"}
      </Typography>
      <Box sx={{ display: "flex", alignItems: "flex-end", gap: dense ? 0.25 : 1, height }} onMouseLeave={() => setActive(null)}>
        {buckets.map((b, i) => (
          <Box
            key={i}
            onMouseEnter={() => setActive(i)}
            onClick={() => setActive(i)}
            sx={{ flex: 1, minWidth: 0, height: "100%", display: "flex", alignItems: "flex-end", cursor: "pointer" }}
          >
            <Box
              sx={{
                width: "100%",
                height: `${b.secs > 0 ? Math.max(4, (b.secs / max) * 100) : 1.5}%`,
                borderRadius: "4px 4px 0 0",
                bgcolor: b.secs > 0 ? (i === active ? "primary.light" : "primary.main") : "divider",
                transition: "background-color 120ms",
              }}
            />
          </Box>
        ))}
      </Box>
      <Box sx={{ display: "flex", gap: dense ? 0.25 : 1, mt: 0.75 }}>
        {buckets.map((b, i) => (
          // the label may be wider than its bar, so it is centered on the bar and left to spill over
          <Box key={i} sx={{ flex: 1, minWidth: 0, display: "flex", justifyContent: "center" }}>
            <Typography component="span" sx={{ fontSize: 11, whiteSpace: "nowrap", ...faint }}>{b.label}</Typography>
          </Box>
        ))}
      </Box>
    </Box>
  );
}

type Row = { key: string; title: string; subtitle?: string; art: string | null; round?: boolean; secs: number; plays: number };

/** A ranked list. Each row has a bar behind it that shows its share of the top one. */
function RankList({ rows, empty }: { rows: Row[]; empty: string }) {
  const [open, setOpen] = useState(false);
  if (rows.length === 0) return <Typography sx={subdued}>{empty}</Typography>;
  const top = rows[0].secs || 1;
  const visible = rows.slice(0, open ? SHOWN_MAX : SHOWN);
  return (
    <Box>
      {visible.map((r, i) => (
        <Box
          key={r.key}
          sx={{
            position: "relative", display: "flex", alignItems: "center", gap: 1.5, px: 1, py: 0.75, borderRadius: 1.5, overflow: "hidden",
          }}
        >
          <Box
            sx={{
              position: "absolute", inset: 0, width: `${(r.secs / top) * 100}%`, bgcolor: (t) => alpha(t.palette.primary.main, 0.12), zIndex: 0,
            }}
          />
          <Typography sx={{ position: "relative", width: 22, textAlign: "right", fontSize: 13, ...faint }}>{i + 1}</Typography>
          <Cover src={r.art} px={96} sx={{ position: "relative", width: 44, height: 44, fontSize: 22, borderRadius: r.round ? "50%" : 1 }} />
          <Box sx={{ position: "relative", flex: 1, minWidth: 0 }}>
            <Typography noWrap sx={{ fontWeight: 500 }}>{r.title}</Typography>
            {r.subtitle && <Typography noWrap variant="body2" sx={subdued}>{r.subtitle}</Typography>}
          </Box>
          <Box sx={{ position: "relative", textAlign: "right", flexShrink: 0 }}>
            <Typography sx={{ fontWeight: 600, fontSize: 14 }}>{duration(r.secs)}</Typography>
            {r.plays > 0 && <Typography variant="body2" sx={{ ...subdued, fontSize: 12 }}>{r.plays} {r.plays === 1 ? "play" : "plays"}</Typography>}
          </Box>
        </Box>
      ))}
      {rows.length > SHOWN && (
        <Button size="small" color="inherit" onClick={() => setOpen(!open)} sx={{ mt: 1, textTransform: "none" }}>
          {open ? "Show less" : `Show ${Math.min(rows.length, SHOWN_MAX) - SHOWN} more`}
        </Button>
      )}
    </Box>
  );
}

const WEEKDAYS = [0, 1, 2, 3, 4, 5, 6].map((i) => new Date(2024, 0, 1 + i)); // a Monday first

export default function StatsPage() {
  const navigate = useNavigate();
  const [collecting] = useSetting("statsEnabled", true, isBoolean);
  const [range, setRange] = useState<Range>("month");
  const [days, setDays] = useState<Day[] | null>(null);

  useEffect(() => {
    const read = () =>
      loadDays()
        .then(setDays)
        .catch((e) => {
          log.error("couldn't read the statistics:", e);
          setDays([]);
        });
    void read();
    // a sync can bring the days of another device
    window.addEventListener(SYNCED_EVENT, read);
    return () => window.removeEventListener(SYNCED_EVENT, read);
  }, []);

  const summary = useMemo(() => (days ? summarize(days, range) : null), [days, range]);
  const run = useMemo(() => (days ? streak(days) : 0), [days]);

  const weekdayBuckets: Bucket[] = useMemo(
    () =>
      WEEKDAYS.map((d, i) => ({
        label: d.toLocaleDateString(undefined, { weekday: "short" }),
        title: d.toLocaleDateString(undefined, { weekday: "long" }),
        secs: summary?.weekdays[i] ?? 0,
      })),
    [summary],
  );

  return (
    <Box sx={{ maxWidth: 960, mx: "auto", px: IS_MOBILE ? GUTTER : 3, py: IS_MOBILE ? 2 : 4, display: "flex", flexDirection: "column", gap: 2.5 }}>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1.5 }}>
        <IconButton aria-label="Back" onClick={() => navigate(-1)} edge="start">
          <ArrowBackIcon />
        </IconButton>
        <Box>
          <Typography variant={IS_MOBILE ? "h5" : "h4"} sx={{ fontWeight: 700 }}>Statistics</Typography>
          <Typography variant="body2" sx={subdued}>Adds up the listening of every device that shares this library</Typography>
        </Box>
      </Box>

      {!collecting && (
        <Typography variant="body2" sx={{ ...subdued, px: 1 }}>
          Statistics are turned off, so nothing new is added. What was already collected is shown below.
        </Typography>
      )}

      <ToggleButtonGroup
        exclusive
        fullWidth={IS_MOBILE}
        size="small"
        value={range}
        onChange={(_, v: Range | null) => v && setRange(v)}
        sx={{ alignSelf: IS_MOBILE ? "stretch" : "flex-start" }}
      >
        {RANGES.map((r) => (
          <ToggleButton key={r.value} value={r.value} sx={{ px: 2.5, textTransform: "none" }}>{r.label}</ToggleButton>
        ))}
      </ToggleButtonGroup>

      {summary === null ? (
        <Typography sx={subdued}>Loading...</Typography>
      ) : (
        <>
          <Box sx={{ display: "grid", gap: 1.5, gridTemplateColumns: { xs: "repeat(2, 1fr)", md: "repeat(4, 1fr)" } }}>
            <StatCard label="Listening time" value={duration(summary.secs)} note={summary.radioSecs > 0 ? `${duration(summary.radioSecs)} of it radio` : undefined} />
            <StatCard label="Songs played" value={String(summary.plays)} note={`${summary.uniqueTracks} different`} />
            <StatCard label="Artists" value={String(summary.uniqueArtists)} />
            <StatCard label="Streak" value={`${run} ${run === 1 ? "day" : "days"}`} note={`${duration(summary.dailyAverage)} a day on average`} />
          </Box>

          <Panel title="Listening time">
            <BarChart buckets={summary.buckets} empty="Nothing was played in this period" />
            {summary.best && (
              <Typography variant="body2" sx={{ ...subdued, mt: 1.5 }}>Busiest day: {summary.best.title}, {duration(summary.best.secs)}</Typography>
            )}
          </Panel>

          <Box sx={{ display: "grid", gap: 2.5, gridTemplateColumns: { xs: "1fr", md: "1fr 1fr" }, alignItems: "start" }}>
            <Panel title="Top songs">
              <RankList
                empty="No songs yet"
                rows={summary.tracks.map((t) => ({ key: t.id, title: t.title, subtitle: t.artist, art: t.thumbnail, secs: t.secs, plays: t.plays }))}
              />
            </Panel>
            <Panel title="Top artists">
              <RankList
                empty="No artists yet"
                rows={summary.artists.map((a) => ({ key: a.name, title: a.name || "Unknown artist", art: a.thumbnail, round: true, secs: a.secs, plays: a.plays }))}
              />
            </Panel>
          </Box>

          <Panel title="Days of the week">
            <BarChart buckets={weekdayBuckets} height={110} empty="Nothing was played in this period" />
          </Panel>

          {summary.stations.length > 0 && (
            <Panel title="Radio stations">
              <RankList
                empty=""
                rows={summary.stations.map((t) => ({ key: t.id, title: t.title, subtitle: t.artist, art: t.thumbnail, secs: t.secs, plays: 0 }))}
              />
            </Panel>
          )}
        </>
      )}
    </Box>
  );
}
