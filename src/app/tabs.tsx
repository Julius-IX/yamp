import HomeIcon from "@mui/icons-material/Home";
import AlbumIcon from "@mui/icons-material/Album";
import ExploreIcon from "@mui/icons-material/Explore";
import LibraryMusicIcon from "@mui/icons-material/LibraryMusic";
import SettingsIcon from "@mui/icons-material/Settings";
import type { Tab } from "@/router/types";
import { IS_MOBILE } from "@/util/platform";
import { placeholderRoutes } from "./Placeholder";
import { homeRoutes } from "@/home/routes";
import AlbumsPage from "./sidebar/AlbumsPage";
import { settingsRoutes } from "@/settings/routes";

// The first tab is the one shown at launch and the one Android back falls back to.
export const tabs: Tab[] = [
  { id: "home", label: "Home", icon: <HomeIcon />, routes: homeRoutes },
  { id: "explore", label: "Explore", icon: <ExploreIcon />, routes: placeholderRoutes("Explore") },
  { id: "library", label: "Library", icon: <LibraryMusicIcon />, routes: placeholderRoutes("Library") },
  // desktop shows the albums in the sidebar instead
  ...(IS_MOBILE ? [{ id: "albums", label: "Albums", icon: <AlbumIcon />, routes: [{ index: true, element: <AlbumsPage /> }] }] : []),
  { id: "settings", label: "Settings", icon: <SettingsIcon />, routes: settingsRoutes },
];
