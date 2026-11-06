import type { RouteObject } from "react-router-dom";
import { IS_MOBILE } from "@/util/platform";
import DesktopSettings from "./DesktopSettings";
import MobileSettings from "./MobileSettings";

export const settingsRoutes: RouteObject[] = [
  { index: true, element: IS_MOBILE ? <MobileSettings /> : <DesktopSettings /> },
];
