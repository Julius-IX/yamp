import { useHistory } from "@/player/history";
import TrackShelf from "./TrackShelf";

export default function RecentlyPlayed() {
  const history = useHistory();
  return <TrackShelf title="Recently Played" tracks={history} empty="Songs that were played show up here." />;
}
