import type { ComponentType } from "react";
import AboutPage from "./AboutPage";
import AppearancePage from "./AppearancePage";
import DataPage from "./DataPage";
import DebuggingPage from "./DebuggingPage";
import GeneralPage from "./GeneralPage";
import PlaybackPage from "./PlaybackPage";

export type SettingsPage = {
  id: string;
  Content: ComponentType;
};

/** One list drives both layouts. Each page is a section that carries its own title, and both screens stack them in this order. */
export const settingsPages: SettingsPage[] = [
  { id: "general", Content: GeneralPage },
  { id: "appearance", Content: AppearancePage },
  { id: "playback", Content: PlaybackPage },
  { id: "data", Content: DataPage },
  { id: "about", Content: AboutPage },
  { id: "debugging", Content: DebuggingPage },
];
