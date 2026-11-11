import { useEffect, useRef, useState, type PointerEvent } from "react";
import { alpha, Box, Checkbox, IconButton, Tooltip, Typography, type Theme } from "@mui/material";
import ChecklistIcon from "@mui/icons-material/Checklist";
import CloseIcon from "@mui/icons-material/Close";
import DragIndicatorIcon from "@mui/icons-material/DragIndicator";
import OndemandVideoIcon from "@mui/icons-material/OndemandVideo";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import PlaylistAddIcon from "@mui/icons-material/PlaylistAdd";
import { isLocalFile } from "@/lib/local";
import type { Track } from "@/lib/types";
import Cover from "@/player/Cover";
import LikeButton from "@/player/LikeButton";
import { currentTrack, player, usePlayer } from "@/player/store";
import { isRadio } from "@/radio/station";
import { video } from "@/video/store";
import { IS_MOBILE } from "@/util/platform";
import AddToPlaylist from "@/library/AddToPlaylist";
import DownloadButton from "@/library/DownloadButton";
import ArtistLink from "./ArtistLink";
import SelectionBar from "./SelectionBar";
import { TrackMenuButton } from "./TrackMenu";
import { unique } from "./tracks";
import { columnHead, faint, GUTTER, radius, subdued } from "@/themes/ui";

// The header and the rows share these so the columns line up. The buttons sit after them.
const columns = (plays: boolean) =>
  IS_MOBILE ? "32px minmax(0, 1fr)" : `40px minmax(0, 1.4fr) minmax(0, 1fr)${plays ? " 96px" : ""}`;
const BUTTON = 40;
/**
 * Desktop has a button for each of queue, playlist, download, video and like, plus remove when
 * the table can remove and the drag handle when it can move rows. All of them but the heart show
 * on hover. Mobile has no room for that, it has the heart and one menu.
 */
const actionsWidth = (removable: boolean, movable: boolean) =>
  IS_MOBILE ? BUTTON * 2 : BUTTON * (5 + (removable ? 1 : 0) + (movable ? 1 : 0));

/** Distance from the edge of the scrolling page where dragging starts to scroll it, and its speed. */
const SCROLL_ZONE = 72;
const SCROLL_STEP = 14;

type Drag = { from: number; to: number };

/**
 * One slot of the action buttons. Every button sits in the same fixed box, centered on both axes,
 * so they line up whatever their own size is. The tooltip wraps a button in an inline span, which
 * would otherwise sit on the text baseline and look higher than a bare button. A slot that is not
 * `shown` appears when the row is hovered.
 */
const cell = (shown: boolean) => ({
  width: BUTTON, height: BUTTON, display: "flex", alignItems: "center", justifyContent: "center",
  opacity: shown ? 1 : 0,
  "& > span": { display: "flex" },
});

type RowProps = {
  track: Track; index: number; active: boolean; plays?: string | null; showPlays: boolean; onPlay: () => void;
  onRemove?: () => void;
  /** Plays the track and shows its picture. */
  onWatch: () => void;
  /** Present when the row can be moved with the mouse. The handle starts the drag and the table follows it. */
  handle?: { onStart: (e: PointerEvent) => void };
  /** Where the dragged row would land relative to this one, for the line that shows it. */
  drop?: "above" | "below" | null;
  dragging?: boolean;
  /** Mobile only. The handles are showing, and the menu offers to show them. */
  reordering?: boolean;
  onReorder?: () => void;
  /** While picking, the row shows a checkbox and a click picks it and does not play. */
  selecting?: boolean;
  selected?: boolean;
  onToggle?: () => void;
};

/**
 * A pointer drag and not the HTML one, which a touch screen does not start. `touchAction: none`
 * keeps the page from scrolling under the finger, the table scrolls it while a row is dragged.
 */
function Handle({ onStart }: { onStart: (e: PointerEvent) => void }) {
  return (
    <Box
      aria-label="Drag to move"
      data-handle
      onPointerDown={onStart}
      onClick={(e) => e.stopPropagation()}
      sx={{ display: "flex", cursor: "grab", touchAction: "none", color: "text.secondary", p: 1 }}
    >
      <DragIndicatorIcon fontSize="small" />
    </Box>
  );
}

/**
 * The row is a div that plays when clicked, not a button. The artist and the actions inside are
 * controls of their own, and a button cannot hold another one.
 */
function Row({ track, index, active, plays, showPlays, onPlay, onWatch, onRemove, handle, drop, dragging, reordering, onReorder, selecting, selected, onToggle }: RowProps) {
  const watchable = !isRadio(track.id) && !isLocalFile(track.id);
  return (
    <Box
      role="button"
      tabIndex={0}
      data-row
      onClick={selecting ? onToggle : onPlay}
      onKeyDown={(e) => {
        if (e.target !== e.currentTarget || (e.key !== "Enter" && e.key !== " ")) return;
        e.preventDefault();
        (selecting ? onToggle : onPlay)?.();
      }}
      sx={{
        display: "flex", alignItems: "center", py: 0.75, borderRadius: radius.row, cursor: "pointer",
        opacity: dragging ? 0.5 : 1,
        boxShadow: (t) => (drop ? `inset 0 ${drop === "above" ? 2 : -2}px 0 ${t.palette.primary.main}` : "none"),
        "&:hover, &:focus-visible": { bgcolor: "action.hover", outline: "none" },
        ...(!selecting && {
          "&:hover .row-index": { display: "none" },
          "&:hover .row-play": { display: "flex" },
          "&:hover .row-action": { opacity: 1 },
        }),
        ...(selected && { bgcolor: (t: Theme) => alpha(t.palette.primary.main, 0.12) }),
      }}
    >
      <Box sx={{ flex: 1, minWidth: 0, display: "grid", gridTemplateColumns: columns(showPlays), alignItems: "center" }}>
        {selecting ? (
          <Box sx={{ display: "flex", justifyContent: "center" }}>
            <Checkbox size="small" checked={!!selected} disableRipple tabIndex={-1} sx={{ p: 0 }} />
          </Box>
        ) : (
          <Typography className="row-index" variant="body2" sx={{ ...(active ? { color: "primary.main" } : faint), textAlign: "center" }}>
            {index + 1}
          </Typography>
        )}
        <Box className="row-play" sx={{ display: "none", justifyContent: "center", color: "text.primary" }}>
          <PlayArrowIcon fontSize="small" />
        </Box>
        <Box sx={{ display: "flex", alignItems: "center", gap: 1.5, minWidth: 0 }}>
          <Cover src={track.thumbnail} px={96} sx={{ width: 40, height: 40, fontSize: 20 }} />
          <Box sx={{ minWidth: 0 }}>
            <Typography variant="body2" noWrap color={active ? "primary" : "text.primary"}>{track.title}</Typography>
            {IS_MOBILE && (
              <Typography variant="caption" noWrap component="div" sx={subdued}><ArtistLink artist={track.artist} /></Typography>
            )}
          </Box>
        </Box>
        {!IS_MOBILE && (
          <Typography variant="body2" noWrap sx={subdued}><ArtistLink artist={track.artist} /></Typography>
        )}
        {!IS_MOBILE && showPlays && (
          <Typography variant="body2" noWrap sx={{ ...subdued, textAlign: "right", pr: 1 }}>{plays ?? ""}</Typography>
        )}
      </Box>

      <Box onClick={(e) => e.stopPropagation()} sx={{ width: actionsWidth(!!onRemove, !!handle), display: "flex", alignItems: "center", justifyContent: "flex-end" }}>
        {selecting ? null : IS_MOBILE ? (
          <>
            <Box sx={cell(true)}><LikeButton track={track} size="small" /></Box>
            <Box sx={cell(true)}>
              {reordering && handle ? (
                <Handle onStart={handle.onStart} />
              ) : (
                <TrackMenuButton track={track} onWatch={onWatch} onRemove={onRemove} onReorder={onReorder} />
              )}
            </Box>
          </>
        ) : (
          <>
            <Box className="row-action" sx={cell(false)}>
              <Tooltip title="Add to queue (Shift: play next)">
                <IconButton
                  size="small" aria-label="Add to queue"
                  onClick={(e) => player.enqueue([track], e.shiftKey ? "next" : "end")}
                >
                  <PlaylistAddIcon fontSize="small" />
                </IconButton>
              </Tooltip>
            </Box>
            <Box className="row-action" sx={cell(false)}><AddToPlaylist track={track} /></Box>
            <Box className="row-action" sx={cell(false)}><DownloadButton track={track} /></Box>
            {/* the slot is kept for what has no picture, so the buttons after it stay in their columns */}
            <Box className="row-action" sx={cell(false)}>
              {watchable && (
                <Tooltip title="Watch video">
                  <IconButton size="small" aria-label="Watch video" onClick={onWatch}>
                    <OndemandVideoIcon fontSize="small" />
                  </IconButton>
                </Tooltip>
              )}
            </Box>
            {handle && <Box className="row-action" sx={cell(false)}><Handle onStart={handle.onStart} /></Box>}
            {onRemove && (
              <Box className="row-action" sx={cell(false)}>
                <Tooltip title="Remove from playlist">
                  <IconButton size="small" aria-label="Remove from playlist" onClick={onRemove}>
                    <CloseIcon fontSize="small" />
                  </IconButton>
                </Tooltip>
              </Box>
            )}
            {/* the heart is the one button that is always there, last so it keeps the same place in every row */}
            <Box sx={cell(true)}><LikeButton track={track} size="small" /></Box>
          </>
        )}
      </Box>
    </Box>
  );
}

/** The nearest ancestor that scrolls, which is the page the table is on. */
function scroller(el: HTMLElement | null): HTMLElement | null {
  for (let p = el?.parentElement; p; p = p.parentElement) {
    const overflow = getComputedStyle(p).overflowY;
    if ((overflow === "auto" || overflow === "scroll") && p.scrollHeight > p.clientHeight) return p;
  }
  return null;
}

/**
 * Tracks as numbered rows. The row of the playing track is tinted, and a click plays from that
 * row. `plays` adds a column of play counts, as display text, one entry per track. `onRemove`
 * adds a button that takes a row out of the list the table shows, for a playlist of the library.
 * `onMove` adds a handle to drag a row to another place, the row ends up at the index it is
 * dropped on. The table does not reorder itself, the caller changes `tracks` in answer.
 */
export default function TrackTable({
  tracks, plays, onPlay, onRemove, onMove,
}: {
  tracks: Track[]; plays?: (string | null)[]; onPlay: (index: number) => void; onRemove?: (index: number) => void;
  onMove?: (from: number, to: number) => void;
}) {
  const activeId = currentTrack(usePlayer())?.id;
  const body = useRef<HTMLDivElement>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  // Mobile has no room for a handle on every row, so the handles are a mode the menu of a row starts
  const [reordering, setReordering] = useState(false);
  // the ids of the picked tracks while picking, null when not
  const [picked, setPicked] = useState<ReadonlySet<string> | null>(null);
  const everyone = unique(tracks);
  const chosen = picked ? everyone.filter((t) => picked.has(t.id)) : [];
  const toggle = (id: string) =>
    setPicked((p) => {
      const next = new Set(p);
      if (!next.delete(id)) next.add(id);
      return next;
    });
  // the latest pointer height, read by the scroll loop which runs between pointer events
  const pointerY = useRef(0);

  const indexAt = (y: number) => {
    const rows = Array.from(body.current?.querySelectorAll<HTMLElement>("[data-row]") ?? []);
    const hit = rows.findIndex((r) => y < r.getBoundingClientRect().top + r.offsetHeight / 2);
    return hit === -1 ? rows.length - 1 : hit;
  };

  // a mirror of the state for the window handlers, which outlive the render they were made in
  const current = useRef<Drag | null>(null);
  const moveRef = useRef(onMove);
  moveRef.current = onMove;
  const set = (d: Drag | null) => {
    current.current = d;
    setDrag(d);
  };
  const follow = (y: number) => {
    const d = current.current;
    const to = d && indexAt(y);
    if (d && to !== d.to) set({ from: d.from, to: to! });
  };

  const start = (from: number) => (e: PointerEvent) => {
    e.preventDefault();
    e.currentTarget.setPointerCapture(e.pointerId);
    pointerY.current = e.clientY;
    set({ from, to: from });
  };
  const finish = (drop: boolean) => {
    const d = current.current;
    set(null);
    if (d && drop && d.from !== d.to) moveRef.current?.(d.from, d.to);
  };

  // the handle holds the pointer capture, so its events reach the window while the drag lasts
  const dragging = drag !== null;
  useEffect(() => {
    if (!dragging) return;
    const moved = (e: globalThis.PointerEvent) => {
      pointerY.current = e.clientY;
      follow(e.clientY);
    };
    const up = () => finish(true);
    const cancel = () => finish(false);
    window.addEventListener("pointermove", moved);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    const page = scroller(body.current);
    let frame = 0;
    const scroll = () => {
      if (page) {
        const { top, bottom } = page.getBoundingClientRect();
        const y = pointerY.current;
        if (y < top + SCROLL_ZONE) page.scrollTop -= SCROLL_STEP;
        else if (y > bottom - SCROLL_ZONE) page.scrollTop += SCROLL_STEP;
        // the rows moved under the pointer, so the target changes without a pointer event
        follow(y);
      }
      frame = requestAnimationFrame(scroll);
    };
    frame = requestAnimationFrame(scroll);
    return () => {
      window.removeEventListener("pointermove", moved);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
      cancelAnimationFrame(frame);
    };
    // the handlers only read refs, so one set of them serves the whole drag
  }, [dragging]);

  // A tap anywhere but on a handle ends the mode. It is caught before it reaches the row under
  // it, which would otherwise start playing. Scrolling is no tap, so the list can be scrolled
  // to bring the target row into view.
  useEffect(() => {
    if (!reordering) return;
    const leave = (e: MouseEvent) => {
      if ((e.target as Element | null)?.closest("[data-handle]")) return;
      e.stopPropagation();
      e.preventDefault();
      setReordering(false);
    };
    // not while the tap that started the mode is still being delivered
    const timer = setTimeout(() => window.addEventListener("click", leave, true));
    return () => {
      clearTimeout(timer);
      window.removeEventListener("click", leave, true);
    };
  }, [reordering]);
  useEffect(() => {
    if (!onMove) setReordering(false);
  }, [onMove]);

  return (
    <Box sx={{ px: GUTTER }}>
      {picked ? (
        <SelectionBar
          picked={chosen} total={everyone.length}
          onToggleAll={() => setPicked(chosen.length === everyone.length ? new Set() : new Set(everyone.map((t) => t.id)))}
          onDone={() => setPicked(null)}
        />
      ) : (
        <Box sx={{ display: "flex", alignItems: "center", borderBottom: 1, borderColor: "divider", pb: 1, mb: 0.5 }}>
          <Box sx={{ flex: 1, display: "grid", gridTemplateColumns: columns(!!plays) }}>
            <Typography sx={{ ...columnHead, textAlign: "center" }}>#</Typography>
            <Typography sx={columnHead}>TITLE</Typography>
            {!IS_MOBILE && <Typography sx={columnHead}>ARTIST</Typography>}
            {!IS_MOBILE && plays && <Typography sx={{ ...columnHead, textAlign: "right", pr: 1 }}>PLAYS</Typography>}
          </Box>
          <Box sx={{ width: actionsWidth(!!onRemove, !!onMove), display: "flex", justifyContent: "flex-end" }}>
            {/* in the column of the heart, which is the last button of a row on both platforms */}
            <Box sx={{ width: BUTTON, display: "flex", justifyContent: "center" }}>
              <Tooltip title="Select tracks">
                <IconButton size="small" aria-label="Select tracks" onClick={() => { setReordering(false); setPicked(new Set()); }}>
                  <ChecklistIcon fontSize="small" />
                </IconButton>
              </Tooltip>
            </Box>
          </Box>
        </Box>
      )}
      {reordering && (
        <Typography variant="caption" sx={{ ...subdued, display: "block", py: 0.5 }}>
          Drag the handles to reorder. Tap anywhere else when done.
        </Typography>
      )}
      <Box ref={body} sx={{ userSelect: drag ? "none" : "auto" }}>
        {tracks.map((t, i) => (
          <Row
            key={`${t.id}:${i}`} track={t} index={i} active={t.id === activeId} plays={plays?.[i]} showPlays={!!plays}
            onPlay={() => onPlay(i)} onWatch={() => video.watch(tracks, i)} onRemove={onRemove && (() => onRemove(i))}
            handle={onMove && (!IS_MOBILE || reordering) ? { onStart: start(i) } : undefined}
            reordering={reordering}
            onReorder={onMove && IS_MOBILE ? () => setReordering(true) : undefined}
            selecting={!!picked} selected={!!picked?.has(t.id)} onToggle={() => toggle(t.id)}
            dragging={drag?.from === i}
            drop={drag && drag.from !== drag.to && drag.to === i ? (drag.to > drag.from ? "below" : "above") : null}
          />
        ))}
      </Box>
    </Box>
  );
}
