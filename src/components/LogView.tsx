import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  Box,
  FormControlLabel,
  IconButton,
  MenuItem,
  Select,
  Switch,
  TextField,
  ToggleButton,
  ToggleButtonGroup,
  Tooltip,
  Typography,
} from "@mui/material";
import ContentCopyIcon from "@mui/icons-material/ContentCopy";
import DeleteSweepIcon from "@mui/icons-material/DeleteSweep";
import { COMMANDS } from "../lib/commands";
import type { LogEntry, LogLevel, LogSource } from "../lib/types";

/** Entries held in memory. Matches the backend buffer so a reload shows the same history. */
const MAX_ENTRIES = 5000;
/** Rows rendered at once. Filtering happens on all entries, only the tail is drawn. */
const MAX_ROWS = 1000;

const LEVELS: LogLevel[] = ["error", "warn", "info", "debug", "trace"];
const COLORS: Record<LogLevel, string> = {
  error: "error.main",
  warn: "warning.main",
  info: "info.main",
  debug: "text.secondary",
  trace: "text.disabled",
};

type SourceFilter = LogSource | "all";

const time = (ts: number) => new Date(ts).toISOString().slice(11, 23);
const line = (e: LogEntry) =>
  `${time(e.ts)} ${e.level.toUpperCase().padEnd(5)} [${e.source}] ${e.target}: ${e.message}`;

function merge(prev: LogEntry[], incoming: LogEntry[]): LogEntry[] {
  const seen = new Set(prev.map((e) => e.id));
  const fresh = incoming.filter((e) => !seen.has(e.id));
  if (fresh.length === 0) return prev;
  return [...prev, ...fresh].sort((a, b) => a.id - b.id).slice(-MAX_ENTRIES);
}

/** Subscribes before fetching history, so entries logged in between are not lost. */
function useLogEntries() {
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const pending = useRef<LogEntry[]>([]);
  const frame = useRef(0);

  // bursts are applied once per frame, a state update per entry would re-render the list each time
  const enqueue = useCallback((e: LogEntry) => {
    pending.current.push(e);
    if (frame.current) return;
    frame.current = requestAnimationFrame(() => {
      frame.current = 0;
      const batch = pending.current;
      pending.current = [];
      setEntries((prev) => merge(prev, batch));
    });
  }, []);

  useEffect(() => {
    let alive = true;
    const unlisten = [
      listen<LogEntry>("log:entry", (e) => enqueue(e.payload)),
      listen("log:clear", () => {
        pending.current = [];
        setEntries([]);
      }),
    ];
    COMMANDS.logging.list().then((history) => {
      if (alive) setEntries((prev) => merge(prev, history));
    });
    return () => {
      alive = false;
      cancelAnimationFrame(frame.current);
      frame.current = 0;
      unlisten.forEach((p) => p.then((off) => off()));
    };
  }, [enqueue]);

  return entries;
}

export default function LogView() {
  const entries = useLogEntries();
  const [source, setSource] = useState<SourceFilter>("all");
  const [minLevel, setMinLevel] = useState<LogLevel>("trace");
  const [query, setQuery] = useState("");
  const [follow, setFollow] = useState(true);
  const scroller = useRef<HTMLDivElement>(null);

  const needle = query.trim().toLowerCase();
  const maxIndex = LEVELS.indexOf(minLevel);
  const visible = entries
    .filter(
      (e) =>
        (source === "all" || e.source === source) &&
        LEVELS.indexOf(e.level) <= maxIndex &&
        (!needle || `${e.target} ${e.message}`.toLowerCase().includes(needle)),
    )
    .slice(-MAX_ROWS);

  useEffect(() => {
    const el = scroller.current;
    if (follow && el) el.scrollTop = el.scrollHeight;
  }, [visible, follow]);

  // scrolling up pauses following, reaching the bottom again resumes it
  const onScroll = () => {
    const el = scroller.current;
    if (el) setFollow(el.scrollHeight - el.scrollTop - el.clientHeight < 24);
  };

  return (
    <Box sx={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0, gap: 1 }}>
      <Box sx={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 1 }}>
        <ToggleButtonGroup
          size="small"
          exclusive
          value={source}
          onChange={(_, v: SourceFilter | null) => v && setSource(v)}
        >
          <ToggleButton value="all">All</ToggleButton>
          <ToggleButton value="backend">Backend</ToggleButton>
          <ToggleButton value="frontend">Frontend</ToggleButton>
        </ToggleButtonGroup>
        <Select size="small" value={minLevel} onChange={(e) => setMinLevel(e.target.value)}>
          {LEVELS.map((l) => (
            <MenuItem key={l} value={l}>
              {l}
            </MenuItem>
          ))}
        </Select>
        <TextField
          size="small"
          placeholder="Filter"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          sx={{ flex: 1, minWidth: 120 }}
        />
        <FormControlLabel
          control={<Switch size="small" checked={follow} onChange={(_, v) => setFollow(v)} />}
          label="Follow"
        />
        <Tooltip title="Copy shown entries">
          <IconButton onClick={() => navigator.clipboard.writeText(visible.map(line).join("\n"))}>
            <ContentCopyIcon fontSize="small" />
          </IconButton>
        </Tooltip>
        <Tooltip title="Clear">
          <IconButton onClick={() => COMMANDS.logging.clear()}>
            <DeleteSweepIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      </Box>

      <Box
        ref={scroller}
        onScroll={onScroll}
        sx={{
          flex: 1,
          minHeight: 0,
          overflow: "auto",
          fontFamily: "monospace",
          fontSize: 12,
          border: 1,
          borderColor: "divider",
          borderRadius: 1,
          p: 1,
        }}
      >
        {visible.length === 0 && (
          <Typography variant="body2" color="text.secondary">
            No entries.
          </Typography>
        )}
        {visible.map((e) => (
          <Box key={e.id} sx={{ display: "flex", gap: 1, whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
            <Box component="span" sx={{ color: "text.secondary", flexShrink: 0 }}>
              {time(e.ts)}
            </Box>
            <Box component="span" sx={{ color: COLORS[e.level], width: "5ch", flexShrink: 0 }}>
              {e.level.toUpperCase()}
            </Box>
            <Box component="span" sx={{ color: "text.secondary", flexShrink: 0 }}>
              {e.source === "backend" ? "BE" : "FE"}
            </Box>
            <Box component="span">
              <Box component="span" sx={{ color: "text.secondary" }}>
                {e.target}:
              </Box>{" "}
              {e.message}
            </Box>
          </Box>
        ))}
      </Box>
    </Box>
  );
}
