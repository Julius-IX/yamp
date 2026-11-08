import type { RouteObject } from "react-router-dom";
import { browseRoutes } from "@/browse/routes";
import SearchLayout from "@/search/SearchLayout";
import { searchRoutes } from "@/search/routes";
import LibraryPage from "./LibraryPage";
import LikedPage from "./LikedPage";
import OwnPlaylistPage from "./OwnPlaylistPage";
import { LikedSongs, OwnPlaylist } from "./paths";

export const libraryRoutes: RouteObject[] = [
  {
    element: <SearchLayout />,
    children: [
      { index: true, element: <LibraryPage /> },
      { path: LikedSongs.path, element: <LikedPage /> },
      { path: OwnPlaylist.path, element: <OwnPlaylistPage /> },
      ...searchRoutes,
      ...browseRoutes,
    ],
  },
];
