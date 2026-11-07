import { route } from "@/router/paths";

// Apart from `routes.tsx` on purpose. Screens build these URLs and the routes render the screens.
export const Artist = route("artist/:id");
export const Album = route("album/:id");
export const Playlist = route("playlist/:id");
