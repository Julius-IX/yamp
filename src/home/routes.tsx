import type { RouteObject } from "react-router-dom";
import { browseRoutes } from "@/browse/routes";
import SearchLayout from "@/search/SearchLayout";
import { searchRoutes } from "@/search/routes";
import HomePage from "./HomePage";

export const homeRoutes: RouteObject[] = [
  {
    element: <SearchLayout />,
    children: [{ index: true, element: <HomePage /> }, ...searchRoutes, ...browseRoutes],
  },
];
