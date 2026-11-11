import { useEffect, useState } from "react";
import { Box, ButtonBase, Typography } from "@mui/material";
import CloseIcon from "@mui/icons-material/Close";
import CropSquareIcon from "@mui/icons-material/CropSquare";
import FilterNoneIcon from "@mui/icons-material/FilterNone";
import RemoveIcon from "@mui/icons-material/Remove";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { faint } from "@/themes/ui";

const HEIGHT = 32;

function Control({ label, onClick, danger, children }: { label: string; onClick: () => void; danger?: boolean; children: React.ReactNode }) {
  return (
    <ButtonBase
      aria-label={label}
      onClick={onClick}
      sx={{
        width: 46, height: HEIGHT, color: "text.secondary", borderRadius: 0, transition: "background-color 0.1s",
        "&:hover": danger ? { bgcolor: "#e81123", color: "#fff" } : { bgcolor: "action.hover", color: "text.primary" },
        "& svg": { fontSize: 16 },
      }}
    >
      {children}
    </ButtonBase>
  );
}

/**
 * The window controls on Windows, in place of the native title bar, which is switched off in
 * `tauri.windows.conf.json`. It takes the colors of the app so the window does not have a
 * differently colored strip on top. The empty part drags the window and a double click on it
 * maximizes, which the `data-tauri-drag-region` attribute does by itself.
 */
export default function TitleBar() {
  const [maximized, setMaximized] = useState(false);
  // outside of the app, like the page opened in a browser, there is no window to control
  const win = "__TAURI_INTERNALS__" in window ? getCurrentWindow() : null;

  useEffect(() => {
    if (!win) return;
    const read = () => void win.isMaximized().then(setMaximized);
    read();
    const stop = win.onResized(read);
    return () => void stop.then((off) => off());
  }, [win]);

  return (
    <Box data-tauri-drag-region sx={{ display: "flex", alignItems: "center", height: HEIGHT, flexShrink: 0, userSelect: "none" }}>
      <Typography
        data-tauri-drag-region variant="caption"
        sx={{ flex: 1, pl: 1.5, letterSpacing: 1, fontWeight: 600, ...faint }}
      >
        yamp
      </Typography>
      <Control label="Minimize" onClick={() => void win?.minimize()}><RemoveIcon /></Control>
      <Control label={maximized ? "Restore" : "Maximize"} onClick={() => void win?.toggleMaximize()}>
        {maximized ? <FilterNoneIcon sx={{ fontSize: "14px !important" }} /> : <CropSquareIcon />}
      </Control>
      <Control label="Close" danger onClick={() => void win?.close()}><CloseIcon /></Control>
    </Box>
  );
}
