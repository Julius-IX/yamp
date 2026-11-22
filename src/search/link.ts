/** What a pasted YouTube or YouTube Music link points at. The id is the one the provider commands take. */
export type SearchLink = { kind: "video" | "playlist" | "album" | "artist"; id: string };

const HOSTS = new Set([
  "youtube.com", "www.youtube.com", "m.youtube.com", "music.youtube.com",
  "youtube-nocookie.com", "www.youtube-nocookie.com", "youtu.be",
]);

// Ids end up in requests, so only what the provider itself uses is accepted. A video id is always 11 characters.
const VIDEO_ID = /^[\w-]{11}$/;
const COLLECTION_ID = /^[\w-]{10,64}$/;

/** A host that is typed without its scheme is a link too, as it is when copied from the address bar. */
const asUrl = (text: string): URL | null => {
  const trimmed = text.trim();
  if (/\s/.test(trimmed)) return null;
  try {
    return new URL(/^[a-z][a-z\d+.-]*:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`);
  } catch {
    return null;
  }
};

/**
 * Reads the text of the search field as a link. Null when it is not one that this app can open,
 * and the text is then searched for as usual. A link with a video and a playlist, like the one
 * of a video that is played from a playlist, is the video.
 */
export function parseLink(text: string): SearchLink | null {
  const url = asUrl(text);
  if (!url || !HOSTS.has(url.hostname.toLowerCase())) return null;
  const [first, second] = url.pathname.split("/").filter(Boolean);

  const video = (id: string | undefined): SearchLink | null => (id && VIDEO_ID.test(id) ? { kind: "video", id } : null);
  const collection = (kind: SearchLink["kind"], id: string | null | undefined): SearchLink | null =>
    id && COLLECTION_ID.test(id) ? { kind, id } : null;

  if (url.hostname.toLowerCase() === "youtu.be") return video(first);
  switch (first) {
    case "watch":
      return video(url.searchParams.get("v") ?? undefined) ?? collection("playlist", url.searchParams.get("list"));
    case "shorts":
    case "live":
    case "embed":
    case "v":
      return video(second);
    case "playlist":
      return collection("playlist", url.searchParams.get("list"));
    case "channel":
      return collection("artist", second);
    case "browse":
      // the albums of YouTube Music are browse ids with this prefix, the artists are channel ids
      return second?.startsWith("MPREb_") ? collection("album", second) : second?.startsWith("UC") ? collection("artist", second) : null;
    default:
      return null;
  }
}
