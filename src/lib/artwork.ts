import { convertFileSrc } from "@tauri-apps/api/core";
import { COMMANDS } from "./commands";
import { logger } from "./log";

const log = logger("artwork");

/**
 * The thumbnail of a local song is `cover:<name>`, the name of a cover file that a scan cached in
 * a folder of the app, see `store::local` in the backend. The folder is read once before the
 * first render, so a cover never has to wait for it.
 */
const LOCAL_COVER = "cover:";
let coverDir = "";

export async function loadCoverDir() {
  try {
    coverDir = await COMMANDS.local.coverDir();
  } catch (e) {
    log.error("couldn't read the cover folder:", e);
  }
}

export const isLocalCover = (url: string | null | undefined) => !!url?.startsWith(LOCAL_COVER);

/** The URL to load an image from. A provider URL is that already, a cached cover is a file. */
export function resolveArtwork(url: string | null | undefined): string | null {
  if (!url) return null;
  return isLocalCover(url) ? convertFileSrc(`${coverDir}/${url.slice(LOCAL_COVER.length)}.jpg`) : url;
}

/**
 * Thumbnails arrive at whatever size the provider listed first, often 120 px or the 320 px
 * YouTube fallback, which looks soft on a modern screen. Both CDNs take the size in the URL,
 * so a larger one is asked for here instead of changing what is stored.
 *
 * Returns the URLs to try in order, the original last. A larger one can be missing, in which
 * case the image fails to load and the next candidate is used.
 */
export function artworkCandidates(url: string | null | undefined, px: number): string[] {
  if (!url) return [];
  // cached at the size it is shown at, there is no larger one to ask for
  if (isLocalCover(url)) return [resolveArtwork(url) as string];
  const video = url.match(/^https?:\/\/(?:i\d?\.ytimg\.com|img\.youtube\.com)\/vi(?:_webp)?\/([\w-]+)\//);
  if (video) {
    // Every failed candidate costs a round trip. Below this size the listed thumbnail is sharp
    // enough, so the upgrade is not worth trying.
    if (px < 640) return [url];
    const base = `https://i.ytimg.com/vi/${video[1]}`;
    // hq720 is 16:9 without the black bars that hqdefault has baked in, but older videos lack it
    return [`${base}/hq720.jpg`, `${base}/hqdefault.jpg`, url];
  }
  // the size is a suffix like =w120-h120-l90-rj or =s88, music art is square so one number is enough
  const sized = url.replace(/=(?:w\d+-h\d+|s\d+)[^/?]*$/, `=w${px}-h${px}-l90-rj`);
  return sized === url ? [url] : [sized, url];
}
