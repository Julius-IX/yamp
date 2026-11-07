import { useEffect, useRef, useState } from "react";
import { Box, ButtonBase, IconButton, InputBase, List, ListItemButton, ListItemIcon, ListItemText, Typography } from "@mui/material";
import CloseIcon from "@mui/icons-material/Close";
import HistoryIcon from "@mui/icons-material/History";
import SearchIcon from "@mui/icons-material/Search";
import { useNavigate, useSearchParams } from "react-router-dom";
import { glass, radius, subdued, surface } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import { Search } from "./paths";
import { addRecentSearch, useRecentSearches } from "./recentSearches";

const IDLE_WIDTH = 260;
const ACTIVE_WIDTH = 480;
const HEIGHT = 40;
const SHOWN_RECENTS = 5;

const Kbd = ({ children }: { children: string }) => (
  <Box
    component="kbd"
    sx={{ px: 0.75, py: 0.25, mx: 0.25, fontSize: 10, fontFamily: "inherit", border: 1, borderColor: "divider", borderRadius: 0.75, ...subdued }}
  >
    {children}
  </Box>
);

/**
 * A pill centered at the top that opens into a wide field on the left while it has focus or a
 * search is showing. With recent searches to pick from, the field and the list are one card.
 * Searching waits for Enter or a click on a recent search, since every search starts provider
 * queries. The query lives in the URL and the field follows it, so back and forward restore a
 * search. `leftInset` keeps it clear of a button at the left edge.
 */
export default function SearchBar({ leftInset = 0 }: { leftInset?: number }) {
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const urlQuery = params.get("q") ?? "";
  const recents = useRecentSearches();
  const root = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);

  const [text, setText] = useState(urlQuery);
  const [focused, setFocused] = useState(false);
  const [cursor, setCursor] = useState(-1);

  useEffect(() => setText(urlQuery), [urlQuery]);

  // Ctrl+K from anywhere on the screen. Tabs stay mounted while hidden, and a hidden bar must not take the key.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k" && root.current && root.current.offsetParent !== null) {
        e.preventDefault();
        setFocused(true);
        input.current?.focus();
        input.current?.select();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const typed = text.trim().toLowerCase();
  const suggestions = recents.filter((r) => r.toLowerCase().includes(typed)).slice(0, SHOWN_RECENTS);
  const open = focused && suggestions.length > 0;
  // the pill stays a pill until there is something in it or it is being typed into
  const expanded = focused || text !== "" || urlQuery !== "";

  const submit = (raw: string) => {
    const q = raw.trim();
    if (!q) return;
    addRecentSearch(q);
    setText(q);
    input.current?.blur();
    if (q !== urlQuery) navigate(Search.to({ query: { q } }));
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const step = e.key === "ArrowDown" ? 1 : -1;
      setCursor((c) => ((c + 1 + step + suggestions.length + 1) % (suggestions.length + 1)) - 1);
    } else if (e.key === "Enter") {
      submit(suggestions[cursor] ?? text);
    } else if (e.key === "Escape") {
      setText(urlQuery);
      input.current?.blur();
    }
  };

  return (
    <Box ref={root}>
      <Box
        sx={{
          maxWidth: "100%",
          width: IS_MOBILE ? `calc(100% - ${leftInset}px)` : expanded ? ACTIVE_WIDTH : IDLE_WIDTH,
          // auto margins center the pill, the open field sits at the left edge
          ml: IS_MOBILE || expanded ? `${leftInset}px` : "auto",
          mr: IS_MOBILE || expanded ? 0 : "auto",
        }}
      >
        {!expanded ? (
          <ButtonBase
            onClick={() => setFocused(true)}
            aria-label="Search"
            sx={{ ...surface, width: "100%", height: HEIGHT, gap: 1, borderRadius: radius.pill, ...subdued, "&:hover": { color: "text.primary" } }}
          >
            <SearchIcon fontSize="small" />
            <Typography variant="body2">Search</Typography>
          </ButtonBase>
        ) : (
          <Box sx={{ border: 1, borderColor: "divider", borderRadius: open ? radius.card : radius.pill, overflow: "hidden" }}>
            {/* the field is opaque, the list below it is the one that is see-through and blurred */}
            <Box sx={{ bgcolor: "background.paper" }}>
              <InputBase
                fullWidth
                // a bar that is only showing the query of the page must not grab the focus when it mounts
                autoFocus={focused}
                inputRef={input}
                value={text}
                onChange={(e) => {
                  setText(e.target.value);
                  setCursor(-1);
                }}
                onFocus={() => setFocused(true)}
                onBlur={() => setFocused(false)}
                onKeyDown={onKeyDown}
                placeholder="What do you want to listen to?"
                inputProps={{ "aria-label": "Search" }}
                startAdornment={<SearchIcon fontSize="small" sx={{ mr: 1, ...subdued }} />}
                endAdornment={
                  text ? (
                    <IconButton
                      size="small" aria-label="Clear search" edge="end"
                      // keeps the focus in the field, a blur would close the recents first
                      onMouseDown={(e) => e.preventDefault()}
                      onClick={() => {
                        setText("");
                        input.current?.focus();
                      }}
                    >
                      <CloseIcon fontSize="small" />
                    </IconButton>
                  ) : focused && !IS_MOBILE ? (
                    <Box sx={{ display: "flex" }}><Kbd>Ctrl</Kbd><Kbd>K</Kbd></Box>
                  ) : null
                }
                sx={{ height: HEIGHT, px: 2 }}
              />
            </Box>

            {open && (
              // a click on a recent must land before the field loses focus and the list goes away
              <Box onMouseDown={(e) => e.preventDefault()} sx={{ ...glass, borderTop: 1, borderColor: "divider" }}>
                <List dense disablePadding sx={{ py: 0.5 }}>
                  {suggestions.map((q, i) => (
                    <ListItemButton key={q} selected={i === cursor} onClick={() => submit(q)} sx={{ py: 1 }}>
                      <ListItemIcon sx={{ minWidth: 36 }}><HistoryIcon fontSize="small" /></ListItemIcon>
                      <ListItemText primary={q} />
                    </ListItemButton>
                  ))}
                </List>
                {!IS_MOBILE && (
                  <Box sx={{ display: "flex", justifyContent: "space-between", px: 1.5, py: 1, borderTop: 1, borderColor: "divider" }}>
                    <Typography variant="caption" sx={subdued}><Kbd>↑</Kbd><Kbd>↓</Kbd> Navigate</Typography>
                    <Typography variant="caption" sx={subdued}><Kbd>Enter</Kbd> Search</Typography>
                  </Box>
                )}
              </Box>
            )}
          </Box>
        )}
      </Box>
    </Box>
  );
}
