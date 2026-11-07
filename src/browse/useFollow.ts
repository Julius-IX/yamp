import { useEffect, useState } from "react";
import { COMMANDS } from "@/lib/commands";
import { logger } from "@/lib/log";
import type { Artist } from "@/lib/types";
import { invalidate } from "./useCached";

const log = logger("browse");

/** The follow state of an artist. `following` is null until the library has answered. */
export function useFollow(artist: Artist | null) {
  const id = artist?.id;
  const [following, setFollowing] = useState<boolean | null>(null);

  useEffect(() => {
    if (!id) return;
    let stale = false;
    COMMANDS.artists
      .isFollowing({ id })
      .then((v) => !stale && setFollowing(v))
      .catch(() => !stale && setFollowing(false));
    return () => {
      stale = true;
    };
  }, [id]);

  const toggle = async () => {
    if (!artist || following === null) return;
    const next = !following;
    setFollowing(next);
    try {
      if (next) await COMMANDS.artists.follow({ artist });
      else await COMMANDS.artists.unfollow({ id: artist.id });
      // the new music shelf on Home was built from the old list
      invalidate("followed-releases");
    } catch (e) {
      log.warn("couldn't change the follow state:", e);
      setFollowing(!next);
    }
  };

  return { following, toggle };
}
