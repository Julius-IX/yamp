/**
 * The shared look of the app, in one place. A screen takes its text tiers, fills, surfaces and
 * radii from here instead of writing them inline, so changing one of them changes it
 * everywhere and two screens cannot drift apart.
 *
 * Everything is a plain object to spread into `sx`, and reads its colors from the theme, so it
 * follows the theme and the light or dark mode. Nothing here should hold a color literal that
 * the theme already has. Add to this file when a style is wanted a second time.
 *
 * Choosing between the text tiers:
 * - primary text, which is the default, for what the screen is about, like a title.
 * - `subdued` for what supports it, like the artist under a track or the summary under a setting.
 * - `faint` for what is only there for orientation, like a column heading or a row number.
 */
import { alpha, type Theme } from "@mui/material/styles";

/** Radii are in units of the theme's `shape.borderRadius`. */
export const radius = { row: 1.5, card: 2, pill: 999 } as const;

/** The space at the sides of a page, in theme spacing units. */
export const GUTTER = 2;

// ---- text

/** Supporting text. The secondary color at 80%, so it sits further back than `text.secondary`. */
export const subdued = { color: (t: Theme) => alpha(t.palette.text.secondary, 0.8) } as const;

/** Orientation only, like a row number or an empty heart. */
export const faint = { color: "text.disabled" } as const;

/** The small word above a big title, like ARTIST or ALBUM. */
export const eyebrow = { fontSize: 12, fontWeight: 700, letterSpacing: 2, color: "primary.main" } as const;

/** A heading of a section of a screen. */
export const sectionTitle = { typography: "h5", fontWeight: 700 } as const;

/** The big title at the top of a screen. */
export const heroTitle = {
  fontSize: { xs: 32, md: 56 }, fontWeight: 800, lineHeight: 1.05, wordBreak: "break-word",
} as const;

/** A column heading of a table. */
export const columnHead = { fontSize: 11, letterSpacing: 1.5, fontWeight: 600, ...faint } as const;

// ---- accent

/**
 * A solid accent fill, for an icon on it. The icon takes the color that reads best on the
 * accent. It does not guarantee text contrast, which an accent can only give as a tint, so
 * text on the accent should use `accentTonal`.
 */
export const accentFill = {
  bgcolor: "primary.main",
  color: (t: Theme) => t.palette.getContrastText(t.palette.primary.main),
  "&:hover": { bgcolor: "primary.dark" },
} as const;

/**
 * The accent as a tint with accent colored text. The accent is chosen to read on the surface
 * in every theme, so this reads wherever the accent does, unlike text on a solid accent fill.
 * It is what marks something as selected.
 */
export const accentTonal = {
  color: "primary.main",
  borderColor: "primary.main",
  bgcolor: (t: Theme) => alpha(t.palette.primary.main, 0.18),
  "&:hover": { bgcolor: (t: Theme) => alpha(t.palette.primary.main, 0.28) },
} as const;

// ---- surfaces

/** A bordered opaque surface, for cards and pills. */
export const surface = { border: 1, borderColor: "divider", bgcolor: "background.paper" } as const;

/** A control that floats over content, like an arrow or a back button. */
export const floating = {
  border: 1, borderColor: "divider", bgcolor: (t: Theme) => alpha(t.palette.background.paper, 0.96),
  "&:hover": { bgcolor: "action.hover" },
} as const;

/** A control over a cover. It is dark whatever the theme, since covers are not themed. */
export const scrim = {
  bgcolor: "rgba(0, 0, 0, 0.6)", color: "#fff", "&:hover": { bgcolor: "rgba(0, 0, 0, 0.85)" },
} as const;

/** Translucent with the content behind it blurred, for a panel that opens over a page. */
export const glass = {
  bgcolor: (t: Theme) => alpha(t.palette.background.paper, 0.6),
  backdropFilter: "blur(14px)",
  WebkitBackdropFilter: "blur(14px)",
} as const;

/** A card of settings or other grouped content. */
export const panel = {
  borderRadius: radius.card, bgcolor: (t: Theme) => alpha(t.palette.background.paper, 0.6),
} as const;
