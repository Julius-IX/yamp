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
  const video = url.match(/^https?:\/\/(?:i\d?\.ytimg\.com|img\.youtube\.com)\/vi(?:_webp)?\/([\w-]+)\//);
  if (video) {
    const base = `https://i.ytimg.com/vi/${video[1]}`;
    // hq720 is 16:9 without the black bars that hqdefault has baked in, but older videos lack it
    return [`${base}/hq720.jpg`, `${base}/hqdefault.jpg`, url];
  }
  // the size is a suffix like =w120-h120-l90-rj or =s88, music art is square so one number is enough
  const sized = url.replace(/=(?:w\d+-h\d+|s\d+)[^/?]*$/, `=w${px}-h${px}-l90-rj`);
  return sized === url ? [url] : [sized, url];
}
