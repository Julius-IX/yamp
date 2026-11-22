import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { Box, type SxProps, type Theme } from "@mui/material";
import { keyframes } from "@mui/material/styles";
import { isBoolean, useSetting } from "@/lib/settings";

/** Pixels per second while the text moves, slow enough to read it. */
const SPEED = 30;
/** The share of a cycle that the text moves in, it rests at both ends for the rest. */
const MOVING = 0.6;
/** Keeps a text that only just does not fit from darting back and forth. */
const MIN_CYCLE_SECS = 4;

const sweep = keyframes`
  0%, 20% { transform: translateX(0); }
  80%, 100% { transform: translateX(calc(var(--marquee-shift) * -1)); }
`;

/**
 * A single line of text that sweeps to its end and back when it is wider than the room it has,
 * and stays put when it fits. An ellipsis hides the rest of a long title for good, and a phone has
 * no hover to show it in full. With animations off, or a system that asks for less motion, the
 * text does not move and the line is scrolled by hand instead.
 *
 * The line takes the width of its container, so the container needs `minWidth: 0` where it is a
 * flex item.
 */
export default function Marquee({ children, sx }: { children: ReactNode; sx?: SxProps<Theme> }) {
  const [animations] = useSetting("animations", true, isBoolean);
  const box = useRef<HTMLDivElement>(null);
  const text = useRef<HTMLSpanElement>(null);
  const [shift, setShift] = useState(0);

  // The text changes size with the track and the box with the screen, and the observer sees both.
  useLayoutEffect(() => {
    const outer = box.current;
    const inner = text.current;
    if (!outer || !inner) return;
    const measure = () => setShift(Math.max(0, Math.ceil(inner.scrollWidth - outer.clientWidth)));
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(outer);
    observer.observe(inner);
    return () => observer.disconnect();
  }, []);

  const moving = animations && shift > 0;
  return (
    <Box
      ref={box}
      sx={[
        {
          overflow: "hidden", whiteSpace: "nowrap", minWidth: 0,
          // by hand, for what does not move on its own. The bar is left out, it would take a line of height
          ...(!animations && { overflowX: "auto", scrollbarWidth: "none", "&::-webkit-scrollbar": { display: "none" } }),
          "@media (prefers-reduced-motion: reduce)": {
            overflowX: "auto", scrollbarWidth: "none", "&::-webkit-scrollbar": { display: "none" },
            "& > span": { animation: "none" },
          },
          // a finger or a pointer on the text holds it still, so it can be read at a spot
          "&:hover > span, &:active > span": { animationPlayState: "paused" },
        },
        ...(Array.isArray(sx) ? sx : [sx]),
      ]}
    >
      <Box
        ref={text}
        component="span"
        sx={{
          display: "inline-block",
          ...(moving && {
            "--marquee-shift": `${shift}px`,
            animation: `${sweep} ${Math.max(MIN_CYCLE_SECS, shift / SPEED / MOVING).toFixed(1)}s ease-in-out infinite alternate`,
          }),
        }}
      >
        {children}
      </Box>
    </Box>
  );
}
