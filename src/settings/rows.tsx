import type { ReactNode } from "react";
import { alpha, Box, Divider, ListItemButton, Paper, Stack, Typography } from "@mui/material";
import { IS_MOBILE } from "@/util/platform";

/**
 * Building blocks of the settings pages. A page is written once from these and looks different
 * per platform. Desktop gets one rounded card per section with the title inside it and the
 * control on the right of each option, like Snowify. Mobile gets flat full width rows under
 * colored category labels, like Tachiyomi.
 */

export type PrefRowProps = {
  title: string;
  /** A small icon before the text, as a quick identifier. */
  icon?: ReactNode;
  summary?: ReactNode;
  /** Shown at the right edge. */
  control?: ReactNode;
  /** Makes the row pressable on mobile. Desktop rows act through their control. */
  onClick?: () => void;
  danger?: boolean;
};

export function PrefRow({ title, icon, summary, control, onClick, danger }: PrefRowProps) {
  if (!IS_MOBILE) {
    return (
      <Box sx={{ display: "flex", alignItems: "center", gap: 3, py: 1.25 }}>
        {icon && <Box sx={{ display: "flex", color: "text.secondary" }}>{icon}</Box>}
        <Box sx={{ flex: 1, minWidth: 0 }}>
          <Typography sx={{ fontSize: "1.05rem", fontWeight: 500 }} color={danger ? "error" : "text.primary"}>
            {title}
          </Typography>
          {/* the subdued text color of the theme, so the explanation reads as secondary to the option */}
          {summary != null && (
            <Typography variant="body2" color="text.secondary" sx={{ wordBreak: "break-word", color: (t) => alpha(t.palette.text.secondary, 0.8) }}>
              {summary}
            </Typography>
          )}
        </Box>
        {control}
      </Box>
    );
  }

  const content = (
    <>
      {icon && <Box sx={{ display: "flex", color: "text.secondary" }}>{icon}</Box>}
      <Box sx={{ flex: 1, minWidth: 0 }}>
        <Typography color={danger ? "error" : "text.primary"}>{title}</Typography>
        {summary != null && (
          <Typography variant="body2" color="text.secondary" sx={{ wordBreak: "break-word" }}>
            {summary}
          </Typography>
        )}
      </Box>
      {control}
    </>
  );
  const layout = { display: "flex", alignItems: "center", gap: 2, px: 2, py: 1.5, minHeight: 64 };
  return onClick ? (
    <ListItemButton onClick={onClick} sx={layout}>
      {content}
    </ListItemButton>
  ) : (
    <Box sx={layout}>{content}</Box>
  );
}

export function Section({ title, description, mobileHeading = true, children }: {
  title?: string;
  /** A line of explanation under the title, desktop only. */
  description?: string;
  /** Set to false when the title only repeats the top bar of the page, which mobile has. */
  mobileHeading?: boolean;
  children: ReactNode;
}) {
  if (IS_MOBILE) {
    return (
      <Box sx={{ pb: 1 }}>
        {title && mobileHeading && (
          <Typography variant="subtitle2" color="primary" sx={{ px: 2, pt: 2, pb: 0.5, fontWeight: 600 }}>
            {title}
          </Typography>
        )}
        {children}
      </Box>
    );
  }
  return (
    <Paper
      variant="outlined"
      sx={{ mb: 3, p: 3, borderRadius: 2, bgcolor: (t) => alpha(t.palette.background.paper, 0.6) }}
    >
      {title && (
        <>
          <Typography
            color="primary"
            sx={{ fontSize: "0.78rem", fontWeight: 700, letterSpacing: "0.12em", textTransform: "uppercase" }}
          >
            {title}
          </Typography>
          {/* inside the card padding on purpose, so the line stops short of the card edges */}
          <Divider sx={{ mt: 1.25, mb: 1.5 }} />
        </>
      )}
      {description && (
        <Typography variant="body2" color="text.secondary" sx={{ mb: 1.5, color: (t) => alpha(t.palette.text.secondary, 0.8) }}>
          {description}
        </Typography>
      )}
      <Stack>{children}</Stack>
    </Paper>
  );
}
