import type { RouteObject } from "react-router-dom";
import { browseRoutes } from "@/browse/routes";
import HomeLayout from "./HomeLayout";
import HomePage from "./HomePage";
import { Search } from "./paths";
import SearchResults from "./SearchResults";

export const homeRoutes: RouteObject[] = [
  {
    element: <HomeLayout />,
    children: [
      { index: true, element: <HomePage /> },
      { path: Search.path, element: <SearchResults /> },
      ...browseRoutes,
    ],
  },
];
