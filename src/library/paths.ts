import { route } from "@/router/paths";

// Apart from `routes.tsx` on purpose. Screens build these URLs and the routes render the screens.
export const LikedSongs = route("liked");
/** Downloads and the files of the added folders, in one list. */
export const LocalSongs = route("local");
/** A playlist of the library. The browse `Playlist` route is the one of the provider, so the paths differ. */
export const OwnPlaylist = route("library/:id");
