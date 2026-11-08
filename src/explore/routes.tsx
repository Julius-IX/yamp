import type { RouteObject } from "react-router-dom";
import { browseRoutes } from "@/browse/routes";
import SearchLayout from "@/search/SearchLayout";
import { searchRoutes } from "@/search/routes";
import ExplorePage from "./ExplorePage";
import MoodPage from "./MoodPage";
import { MoodPlaylists } from "./paths";

export const exploreRoutes: RouteObject[] = [
  {
    element: <SearchLayout />,
    children: [
      { index: true, element: <ExplorePage /> },
      { path: MoodPlaylists.path, element: <MoodPage /> },
      ...searchRoutes,
      ...browseRoutes,
    ],
  },
];
