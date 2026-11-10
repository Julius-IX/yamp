import type { ReactNode } from "react";
import { Box, Divider, ListItemButton, Paper, Stack, Switch, Tooltip, Typography } from "@mui/material";
import CloudOutlinedIcon from "@mui/icons-material/CloudOutlined";
import { CLOUD_SECTIONS, useCloudSection, type CloudSection } from "@/lib/cloudSettings";
import { useSyncState } from "@/lib/sync";
import { subdued, panel } from "@/themes/ui";
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
            <Typography variant="body2" color="text.secondary" sx={{ wordBreak: "break-word", ...subdued }}>
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
          <Typography variant="body2" color="text.secondary" sx={{ wordBreak: "break-word", ...subdued }}>
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

/**
 * The switch at the top right of a section that saves its settings to the cloud account, see
 * `lib/cloudSettings`. It can be turned on before an account is linked, and starts to work once one is.
 */
function CloudSwitch({ section }: { section: CloudSection }) {
  const [on, setOn] = useCloudSection(section);
  const linked = !!useSyncState().status?.connected;
  const hint = linked
    ? (CLOUD_SECTIONS[section].note ?? "Save these settings to the cloud account and share them with the other devices")
    : "Link a cloud account under Data to share these settings with other devices";
  return (
    <Tooltip title={hint}>
      <Box component="label" sx={{ display: "flex", alignItems: "center", gap: 0.5, cursor: "pointer", flexShrink: 0 }}>
        <CloudOutlinedIcon sx={{ fontSize: 18, color: on ? "primary.main" : "text.secondary" }} />
        <Typography variant="caption" color="text.secondary">Sync</Typography>
        <Switch size="small" checked={on} onChange={(_, v) => setOn(v)} slotProps={{ input: { "aria-label": `Save ${section} settings to the cloud` } }} />
      </Box>
    </Tooltip>
  );
}

export function Section({ title, description, cloud, children }: {
  title?: string;
  /** A line of explanation under the title, desktop only. */
  description?: string;
  /** Puts the switch for saving the section to the cloud account in the title row. */
  cloud?: CloudSection;
  children: ReactNode;
}) {
  if (IS_MOBILE) {
    return (
      <Box sx={{ pb: 1 }}>
        {title && (
          <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 1, px: 2, pt: 2, pb: 0.5 }}>
            <Typography variant="subtitle2" color="primary" sx={{ fontWeight: 600 }}>
              {title}
            </Typography>
            {cloud && <CloudSwitch section={cloud} />}
          </Box>
        )}
        {children}
      </Box>
    );
  }
  return (
    <Paper
      variant="outlined"
      sx={{ mb: 3, p: 3, ...panel }}
    >
      {title && (
        <>
          <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 2, minHeight: 32 }}>
            <Typography
              color="primary"
              sx={{ fontSize: "0.78rem", fontWeight: 700, letterSpacing: "0.12em", textTransform: "uppercase" }}
            >
              {title}
            </Typography>
            {cloud && <CloudSwitch section={cloud} />}
          </Box>
          {/* inside the card padding on purpose, so the line stops short of the card edges */}
          <Divider sx={{ mt: 1.25, mb: 1.5 }} />
        </>
      )}
      {description && (
        <Typography variant="body2" color="text.secondary" sx={{ mb: 1.5, ...subdued }}>
          {description}
        </Typography>
      )}
      <Stack>{children}</Stack>
    </Paper>
  );
}
