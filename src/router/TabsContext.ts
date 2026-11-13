import { createContext, useContext } from "react";

export type TabsApi = {
  activeId: string;
  /** Switches to a tab, optionally pushing a URL on that tab's own stack. `state` is what the screen reads from its location. */
  openTab: (id: string, to?: string, state?: unknown) => void;
  /** Opens a browse screen, in the active tab when that tab has them and otherwise in the first one that does. */
  openBrowse: (to: string, state?: unknown) => void;
};

export const TabsContext = createContext<TabsApi | null>(null);

export const useTabs = () => {
  const tabs = useContext(TabsContext);
  if (!tabs) throw new Error("useTabs needs to be inside the app layout");
  return tabs;
};
