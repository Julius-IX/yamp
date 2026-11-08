import ItemShelf from "@/browse/ItemShelf";
import { useExploreList } from "./useExplore";

export default function NewReleases() {
  return <ItemShelf title="New Albums & Singles" items={useExploreList("newReleases")} empty="New releases show up here." />;
}
