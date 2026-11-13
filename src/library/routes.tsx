import type { RouteObject } from "react-router-dom";
import { browseRoutes } from "@/browse/routes";
import SearchLayout from "@/search/SearchLayout";
import { Search } from "@/search/paths";
import LibraryPage from "./LibraryPage";
import LibrarySearchResults from "./LibrarySearchResults";
import LikedPage from "./LikedPage";
import LocalPage from "./LocalPage";
import OwnPlaylistPage from "./OwnPlaylistPage";
import { LikedSongs, LocalSongs, OwnPlaylist } from "./paths";

export const libraryRoutes: RouteObject[] = [
  {
    element: <SearchLayout placeholder="Search your library" recents={false} />,
    children: [
      { index: true, element: <LibraryPage /> },
      { path: LikedSongs.path, element: <LikedPage /> },
      { path: LocalSongs.path, element: <LocalPage /> },
      { path: OwnPlaylist.path, element: <OwnPlaylistPage /> },
      { path: Search.path, element: <LibrarySearchResults /> },
      ...browseRoutes,
    ],
  },
];
