/**
 * Typed route paths. A route is defined once and gives both the pattern for the router and
 * a `to()` that builds a URL from it, so a path is never written twice as a string.
 *
 *   const Album = route("album/:albumId");
 *   const AlbumEdit = Album.child("edit");
 *
 *   { path: Album.path, element: <AlbumPage />, children: [
 *       { path: AlbumEdit.segment, element: <EditAlbum /> } ] }
 *
 *   navigate(AlbumEdit.to({ albumId: 7 }, { query: { from: "search" } }));  // /album/7/edit?from=search
 *
 * Param names are read from the pattern at the type level, so a missing or misspelled param
 * is a compile error. A trailing `?` marks a param as optional. Patterns have no leading or
 * trailing slash. Every tab has its own router rooted at "/", so paths never carry a tab prefix.
 */

type ParamValue = string | number;

type Segments<P extends string> = P extends `${infer Head}/${infer Rest}` ? Head | Segments<Rest> : P;
type OptionalKey<S> = S extends `:${infer K}?` ? K : never;
type RequiredKey<S> = S extends `:${string}?` ? never : S extends `:${infer K}` ? K : never;

type ParamsOf<P extends string, V> = { [K in RequiredKey<Segments<P>>]: V } & {
  [K in OptionalKey<Segments<P>>]?: V;
};

/** Params accepted by `to()`. Numbers are fine, they are stringified. */
export type PathParams<P extends string> = ParamsOf<P, ParamValue>;
/** Params as read back from the URL, always strings. */
export type RouteParams<P extends string> = ParamsOf<P, string>;

export type ToOptions = {
  /** Entries with an undefined value are left out. */
  query?: Record<string, ParamValue | boolean | undefined>;
  hash?: string;
};

// The params argument is dropped for paths without any and optional when none is required.
type ToArgs<P extends string> = [RequiredKey<Segments<P>>] extends [never]
  ? [OptionalKey<Segments<P>>] extends [never]
    ? [options?: ToOptions]
    : [params?: PathParams<P>, options?: ToOptions]
  : [params: PathParams<P>, options?: ToOptions];

export type Route<P extends string> = {
  /** Full pattern. Used as `path` of a route that sits at the top of a tab. */
  readonly path: P;
  /** Last piece of the pattern. Used as `path` of a route nested in `children`, where react-router resolves it against the parent. */
  readonly segment: string;
  /** Absolute URL inside the tab, with params filled in and encoded. */
  to: (...args: ToArgs<P>) => string;
  /** A route one level below this one. Its params include the ones of the parent. */
  child: <S extends string>(segment: S) => Route<`${P}/${S}`>;
};

/** URL of a tab's first screen. */
export const TAB_ROOT = "/";

const fill = (pattern: string, params: Record<string, ParamValue | undefined>) =>
  "/" +
  pattern
    .split("/")
    .flatMap((part) => {
      if (!part.startsWith(":")) return [part];
      const optional = part.endsWith("?");
      const key = part.slice(1, optional ? -1 : undefined);
      const value = params[key];
      if (value !== undefined) return [encodeURIComponent(String(value))];
      if (optional) return [];
      throw new Error(`Missing param "${key}" for path "${pattern}"`);
    })
    .join("/");

const suffix = ({ query, hash }: ToOptions = {}) => {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(query ?? {})) {
    if (value !== undefined) search.set(key, String(value));
  }
  const q = search.toString();
  return (q ? `?${q}` : "") + (hash ? `#${hash.replace(/^#/, "")}` : "");
};

const make = <P extends string>(path: P, segment: string): Route<P> => {
  const hasParams = path.split("/").some((part) => part.startsWith(":"));
  return {
    path,
    segment,
    // the runtime split mirrors ToArgs, params come first only when the pattern has any
    to: ((...args: unknown[]) => {
      const [params, options] = (hasParams ? args : [{}, args[0]]) as [
        Record<string, ParamValue | undefined>,
        ToOptions | undefined,
      ];
      return fill(path, params ?? {}) + suffix(options);
    }) as Route<P>["to"],
    child: <S extends string>(next: S) => make(`${path}/${next}`, next),
  };
};

export const route = <P extends string>(path: P): Route<P> => make(path, path);
