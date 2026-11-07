import { type ReactNode, useCallback, useEffect, useRef, useState } from "react";
import { Box, IconButton, Typography } from "@mui/material";
import ChevronLeftIcon from "@mui/icons-material/ChevronLeft";
import ChevronRightIcon from "@mui/icons-material/ChevronRight";
import { IS_DESKTOP } from "@/util/platform";
import { type CardVariant, CardPlaceholder, CardSkeleton } from "./MediaCard";
import { floating, GUTTER, sectionTitle, subdued } from "@/themes/ui";

const GHOSTS = 6;

const arrowSx = { position: "absolute", top: "50%", transform: "translateY(-50%)", zIndex: 1, ...floating } as const;

type Props = {
  title: string;
  /** How many cards `children` holds, or null while loading. */
  count: number | null;
  /** The reason shown under the title when the shelf is empty. */
  empty: string;
  variant?: CardVariant;
  /** Leaves the shelf out when it is empty, for a section that only makes sense with content. */
  hideWhenEmpty?: boolean;
  /** Shown under the title, for a filter of the row. */
  controls?: ReactNode;
  children: ReactNode;
};

/**
 * A titled row of cards that scrolls sideways. An empty row stays on screen as placeholder
 * cards, so the layout does not jump once something arrives.
 */
export default function Shelf({ title, count, empty, variant = "square", hideWhenEmpty, controls, children }: Props) {
  const scroller = useRef<HTMLDivElement>(null);
  const [edges, setEdges] = useState({ left: false, right: false });

  const measure = useCallback(() => {
    const el = scroller.current;
    if (!el) return;
    setEdges({ left: el.scrollLeft > 4, right: el.scrollLeft + el.clientWidth < el.scrollWidth - 4 });
  }, []);

  useEffect(() => {
    const el = scroller.current;
    if (!el) return;
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [count, measure]);

  // a mouse has no horizontal wheel and the scrollbar is hidden, so desktop gets arrows
  const page = (dir: 1 | -1) =>
    scroller.current?.scrollBy({ left: dir * scroller.current.clientWidth * 0.8, behavior: "smooth" });

  if (count === 0 && hideWhenEmpty) return null;

  return (
    <Box component="section" sx={{ pb: 3, minWidth: 0 }}>
      <Box sx={{ px: GUTTER, pb: 1.5 }}>
        <Typography sx={sectionTitle}>{title}</Typography>
        {count === 0 && <Typography variant="body2" sx={subdued}>{empty}</Typography>}
        {controls}
      </Box>

      {/* the row is clipped inside the page margin, so cards never touch the edge of the page */}
      <Box sx={{ position: "relative", mx: GUTTER }}>
        {IS_DESKTOP && edges.left && (
          <IconButton size="small" aria-label="Scroll left" onClick={() => page(-1)} sx={{ ...arrowSx, left: -14 }}>
            <ChevronLeftIcon />
          </IconButton>
        )}
        {IS_DESKTOP && edges.right && (
          <IconButton size="small" aria-label="Scroll right" onClick={() => page(1)} sx={{ ...arrowSx, right: -14 }}>
            <ChevronRightIcon />
          </IconButton>
        )}
        <Box
          ref={scroller}
          onScroll={measure}
          sx={{
            display: "flex", gap: 1.5, overflowX: "auto", scrollSnapType: "x proximity",
            scrollbarWidth: "none", "&::-webkit-scrollbar": { display: "none" },
          }}
        >
          {count
            ? children
            : Array.from({ length: GHOSTS }, (_, i) =>
                count === null ? <CardSkeleton key={i} variant={variant} /> : <CardPlaceholder key={i} variant={variant} />,
              )}
        </Box>
      </Box>
    </Box>
  );
}
