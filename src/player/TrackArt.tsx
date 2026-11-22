import type { ComponentProps } from "react";
import { useTrackCover } from "@/lib/trackCovers";
import type { Track } from "@/lib/types";
import Cover from "./Cover";

/** The cover of a track, which is the one the user set for it or else its thumbnail. */
export default function TrackArt({ track, ...rest }: { track: Track | null | undefined } & Omit<ComponentProps<typeof Cover>, "src">) {
  return <Cover src={useTrackCover(track)} {...rest} />;
}
