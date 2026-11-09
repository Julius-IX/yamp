import HomeIcon from "@mui/icons-material/Home";
import PublicIcon from "@mui/icons-material/Public";
import LibraryMusicIcon from "@mui/icons-material/LibraryMusic";
import RadioIcon from "@mui/icons-material/Radio";
import SettingsIcon from "@mui/icons-material/Settings";
import type { Tab } from "@/router/types";
import { placeholderRoutes } from "./Placeholder";
import { exploreRoutes } from "@/explore/routes";
import { homeRoutes } from "@/home/routes";
import { libraryRoutes } from "@/library/routes";
import { settingsRoutes } from "@/settings/routes";

// The first tab is the one shown at launch and the one Android back falls back to.
export const tabs: Tab[] = [
  { id: "home", label: "Home", icon: <HomeIcon />, routes: homeRoutes, browse: true },
  { id: "explore", label: "Explore", icon: <PublicIcon />, routes: exploreRoutes, browse: true },
  { id: "library", label: "Library", icon: <LibraryMusicIcon />, routes: libraryRoutes, browse: true },
  { id: "radio", label: "Radio", icon: <RadioIcon />, routes: placeholderRoutes("Radio") },
  { id: "settings", label: "Settings", icon: <SettingsIcon />, routes: settingsRoutes },
];
