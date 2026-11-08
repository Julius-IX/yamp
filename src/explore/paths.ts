import { route } from "@/router/paths";

// `params` is the opaque string a mood carries, the provider is the only one that reads it.
export const MoodPlaylists = route("mood/:params");
