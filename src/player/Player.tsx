import { IS_MOBILE } from "@/util/platform";
import MiniPlayer from "./MiniPlayer";
import PlayerBar from "./PlayerBar";
import { usePlayerMediaSession } from "./mediaSession";

/** The one place the player is mounted. Desktop gets the bar, mobile the mini player. */
export default function Player() {
  usePlayerMediaSession();
  return IS_MOBILE ? <MiniPlayer /> : <PlayerBar />;
}
