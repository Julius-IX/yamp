import type { RouteObject } from "react-router-dom";
import ArtistPage from "./ArtistPage";
import CollectionPage from "./CollectionPage";
import { Album, Artist, Playlist } from "./paths";

/**
 * The screens that any tab can open from a card or a link. A tab spreads these into its own
 * routes, so each tab keeps the screen on its own history and back returns to where it was.
 */
export const browseRoutes: RouteObject[] = [
  { path: Artist.path, element: <ArtistPage /> },
  { path: Album.path, element: <CollectionPage kind="album" /> },
  { path: Playlist.path, element: <CollectionPage kind="playlist" /> },
];
