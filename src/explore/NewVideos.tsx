import TrackShelf from "@/browse/TrackShelf";
import { useExploreList } from "./useExplore";

export default function NewVideos() {
  return (
    <TrackShelf
      title="New Music Videos"
      tracks={useExploreList("newVideos")}
      empty="New music videos show up here."
      variant="wide"
      videos
    />
  );
}
