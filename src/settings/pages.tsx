import type { ComponentType, ReactElement } from "react";
import BugReportIcon from "@mui/icons-material/BugReport";
import InfoIcon from "@mui/icons-material/Info";
import PaletteIcon from "@mui/icons-material/Palette";
import PlayCircleIcon from "@mui/icons-material/PlayCircle";
import StorageIcon from "@mui/icons-material/Storage";
import AboutPage from "./AboutPage";
import AppearancePage from "./AppearancePage";
import DataPage from "./DataPage";
import DebuggingPage from "./DebuggingPage";
import PlaybackPage from "./PlaybackPage";

export type SettingsPage = {
  id: string;
  title: string;
  /** The line under the title in the mobile list. */
  summary: string;
  icon: ReactElement;
  Content: ComponentType;
};

/** One list drives both layouts. Desktop stacks the pages on one screen, mobile lists them and opens one at a time. */
export const settingsPages: SettingsPage[] = [
  { id: "appearance", title: "Appearance", summary: "Theme and animations", icon: <PaletteIcon />, Content: AppearancePage },
  { id: "playback", title: "Playback", summary: "Autoplay, audio quality, prefetch", icon: <PlayCircleIcon />, Content: PlaybackPage },
  { id: "data", title: "Data", summary: "Downloads and cloud sync", icon: <StorageIcon />, Content: DataPage },
  { id: "about", title: "About", summary: "Version and source code", icon: <InfoIcon />, Content: AboutPage },
  { id: "debugging", title: "Debugging", summary: "Engine and logs", icon: <BugReportIcon />, Content: DebuggingPage },
];
