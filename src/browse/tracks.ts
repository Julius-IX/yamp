/**
 * Takes one entry from each list in turn, so no single source fills the front of a shelf.
 * Entries already seen, or in `skip`, are left out.
 */
export function interleave<T extends { id: string }>(lists: T[][], max: number, skip: ReadonlySet<string> = new Set()): T[] {
  const seen = new Set(skip);
  const out: T[] = [];
  const longest = Math.max(0, ...lists.map((l) => l.length));
  for (let i = 0; i < longest && out.length < max; i++) {
    for (const list of lists) {
      const t = list[i];
      if (!t || seen.has(t.id) || out.length >= max) continue;
      seen.add(t.id);
      out.push(t);
    }
  }
  return out;
}

/**
 * Runs `fn` over the items with at most `limit` in flight. Every provider query starts a
 * process on desktop, so firing all of them at once would stall playback at launch.
 */
export async function mapLimit<T, R>(items: T[], limit: number, fn: (item: T) => Promise<R>): Promise<R[]> {
  const out = new Array<R>(items.length);
  let next = 0;
  const worker = async () => {
    while (next < items.length) {
      const i = next++;
      out[i] = await fn(items[i]);
    }
  };
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker));
  return out;
}

/**
 * The first entry of each id. A list can name the same item twice, and React cannot tell two
 * children with one key apart.
 */
export function unique<T extends { id: string }>(list: T[]): T[] {
  const seen = new Set<string>();
  return list.filter((x) => !seen.has(x.id) && !!seen.add(x.id));
}
