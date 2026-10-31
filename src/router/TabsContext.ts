import { createContext, useContext } from "react";

export type TabsApi = {
  activeId: string;
  /** Switches to a tab, optionally pushing a URL on that tab's own stack. */
  openTab: (id: string, to?: string) => void;
};

export const TabsContext = createContext<TabsApi | null>(null);

export const useTabs = () => {
  const tabs = useContext(TabsContext);
  if (!tabs) throw new Error("useTabs needs to be inside the app layout");
  return tabs;
};
