import { Box, ButtonBase, Skeleton, Typography } from "@mui/material";
import { faint, GUTTER, radius, subdued } from "@/themes/ui";
import { currentTrack, player, usePlayer } from "@/player/store";
import { IS_MOBILE } from "@/util/platform";
import { type Station, stationLine, stationTrack } from "./station";
import StationLogo from "./StationLogo";

const GHOSTS = 8;
const LOGO = 44;

/**
 * Stations as rows in columns. A row reads left to right, so the numbers of a chart run across.
 * Playing a row queues the whole list from it, which makes next and previous move between the
 * stations. `stations` is undefined while loading.
 */
export default function StationGrid({ stations, numbered = false }: { stations: Station[] | undefined; numbered?: boolean }) {
  const playing = currentTrack(usePlayer())?.id;
  const tracks = stations?.map(stationTrack);

  return (
    <Box
      sx={{
        display: "grid", gap: 0.5, mx: GUTTER,
        gridTemplateColumns: IS_MOBILE ? "minmax(0, 1fr)" : "repeat(auto-fill, minmax(300px, 1fr))",
      }}
    >
      {stations && tracks
        ? stations.map((s, i) => (
            <ButtonBase
              key={s.id}
              onClick={() => player.play(tracks, i)}
              sx={{
                display: "flex", justifyContent: "flex-start", gap: 1.5, p: 0.75, borderRadius: radius.row,
                textAlign: "left", minWidth: 0, "&:hover": { bgcolor: "action.hover" },
              }}
            >
              {numbered && <Typography sx={{ width: 28, flexShrink: 0, textAlign: "center", fontWeight: 700, ...faint }}>{i + 1}</Typography>}
              <StationLogo src={s.favicon} sx={{ width: LOGO, height: LOGO, fontSize: 22 }} />
              <Box sx={{ minWidth: 0 }}>
                <Typography variant="body2" noWrap color={tracks[i].id === playing ? "primary" : "text.primary"} sx={{ fontWeight: 600 }}>
                  {s.name}
                </Typography>
                <Typography variant="caption" noWrap component="div" sx={subdued}>{stationLine(s)}</Typography>
              </Box>
            </ButtonBase>
          ))
        : Array.from({ length: GHOSTS }, (_, i) => (
            <Box key={i} sx={{ display: "flex", alignItems: "center", gap: 1.5, p: 0.75 }}>
              {numbered && <Box sx={{ width: 28 }} />}
              <Skeleton variant="rounded" width={LOGO} height={LOGO} />
              <Box sx={{ flex: 1 }}>
                <Skeleton width="70%" />
                <Skeleton width="40%" />
              </Box>
            </Box>
          ))}
    </Box>
  );
}
