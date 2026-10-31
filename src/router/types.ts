import type { ReactNode } from "react";
import type { RouteObject } from "react-router-dom";

/** Goes on a route as `handle`. Read from every matched route, so a nested route can hide the bar for its subtree. */
export type RouteHandle = {
  /** Hides the sidebar or bottom bar while the route is shown. */
  hideNav?: boolean;
};

export type Tab = {
  /** Unique, also the key of the tab's own router. */
  id: string;
  label: string;
  icon: ReactNode;
  /** The tab's screens and sub-screens. Paths are relative to the tab root. */
  routes: RouteObject[];
  /** Runs when the active tab is tapped again and it resets to its root. For state kept outside the router. */
  onReset?: () => void;
};
