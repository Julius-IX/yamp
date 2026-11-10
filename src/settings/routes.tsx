import type { RouteObject } from "react-router-dom";
import { IS_MOBILE } from "@/util/platform";
import StatsPage from "@/stats/StatsPage";
import DesktopSettings from "./DesktopSettings";
import MobileSettings from "./MobileSettings";
import { Statistics } from "./paths";

export const settingsRoutes: RouteObject[] = [
  { index: true, element: IS_MOBILE ? <MobileSettings /> : <DesktopSettings /> },
  { path: Statistics.path, element: <StatsPage /> },
];
