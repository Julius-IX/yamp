import type { RouteObject } from "react-router-dom";
import type { RouteHandle } from "@/router/types";
import { IS_MOBILE } from "@/util/platform";
import DesktopSettings from "./DesktopSettings";
import { MobileSettingsHome, MobileSettingsPage, SettingsPageRoute } from "./MobileSettings";

export const settingsRoutes: RouteObject[] = IS_MOBILE
  ? [
      { index: true, element: <MobileSettingsHome /> },
      {
        path: SettingsPageRoute.path,
        element: <MobileSettingsPage />,
        handle: { hideNav: true } satisfies RouteHandle,
      },
    ]
  : [{ index: true, element: <DesktopSettings /> }];
