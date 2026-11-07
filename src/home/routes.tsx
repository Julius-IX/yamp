import type { RouteObject } from "react-router-dom";
import ArtistPage from "./ArtistPage";
import CollectionPage from "./CollectionPage";
import HomeLayout from "./HomeLayout";
import HomePage from "./HomePage";
import { Album, Artist, Playlist, Search } from "./paths";
import SearchResults from "./SearchResults";

export const homeRoutes: RouteObject[] = [
  {
    element: <HomeLayout />,
    children: [
      { index: true, element: <HomePage /> },
      { path: Search.path, element: <SearchResults /> },
      { path: Artist.path, element: <ArtistPage /> },
      { path: Album.path, element: <CollectionPage kind="album" /> },
      { path: Playlist.path, element: <CollectionPage kind="playlist" /> },
    ],
  },
];
