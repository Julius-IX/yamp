import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import { BottomNavigation, BottomNavigationAction, Box, Paper } from "@mui/material";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { IS_MOBILE } from "@/util/platform";
import { TAB_ROOT } from "@/router/paths";
import { TabsContext } from "@/router/TabsContext";
import type { RouteHandle, Tab } from "@/router/types";
import Sidebar from "./sidebar/Sidebar";
import { tabs } from "./tabs";

type TabRouter = ReturnType<typeof createMemoryRouter>;

// Every tab has its own router, so its own history stack, independent of the others.
const makeRouter = (t: Tab) => createMemoryRouter(t.routes, { initialEntries: [TAB_ROOT] });

// A stack at its first entry has the default key, anything pushed after it does not.
const atRoot = (r: TabRouter) => r.state.location.key === "default";

function useNavHidden(router: TabRouter) {
  return useSyncExternalStore(
    (cb) => router.subscribe(cb),
    () => router.state.matches.some((m) => (m.route.handle as RouteHandle | undefined)?.hideNav === true),
  );
}

export default function AppLayout() {
  const [activeId, setActiveId] = useState(tabs[0].id);
  // `gen` changes the key of a tab's view, a reset remounts it with a fresh router.
  const [stacks, setStacks] = useState(() =>
    Object.fromEntries(tabs.map((t) => [t.id, { router: makeRouter(t), gen: 0 }])),
  );

  const navHidden = useNavHidden(stacks[activeId].router);

  const latest = useRef({ activeId, stacks });
  latest.current = { activeId, stacks };

  const goTo = (tab: Tab) => {
    if (tab.id !== activeId) return setActiveId(tab.id); // the tab keeps its stack
    tab.onReset?.();
    stacks[tab.id].router.dispose();
    setStacks((s) => ({ ...s, [tab.id]: { router: makeRouter(tab), gen: s[tab.id].gen + 1 } }));
  };

  const openTab = useCallback((id: string, to?: string) => {
    setActiveId(id);
    if (to) latest.current.stacks[id].router.navigate(to);
  }, []);
  const tabsApi = useMemo(() => ({ activeId, openTab }), [activeId, openTab]);

  // Android hardware back arrives as a popstate. A sentinel entry is kept pushed so there is always
  // something to pop. It pops the active tab's stack, then goes to the first tab, then lets the OS exit.
  useEffect(() => {
    history.pushState({ sentinel: true }, "");
    const onPop = () => {
      const { activeId, stacks } = latest.current;
      const router = stacks[activeId].router;
      if (!atRoot(router)) {
        history.pushState({ sentinel: true }, "");
        router.navigate(-1);
      } else if (activeId !== tabs[0].id) {
        history.pushState({ sentinel: true }, "");
        setActiveId(tabs[0].id);
      }
    };
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  return (
    <TabsContext.Provider value={tabsApi}>
      <Box
        sx={{
          display: "flex",
          flexDirection: IS_MOBILE ? "column" : "row",
          height: "100dvh",
          pt: IS_MOBILE ? "env(safe-area-inset-top)" : 0,
          // with the bottom bar hidden the safe area is no longer covered by it
          pb: IS_MOBILE && navHidden ? "env(safe-area-inset-bottom)" : 0,
        }}
      >
        {!IS_MOBILE && !navHidden && (
          <Sidebar tabs={tabs} activeId={activeId} onSelect={goTo} />
        )}

        <Box component="main" sx={{ flex: 1, minHeight: 0, position: "relative" }}>
          {tabs.map((t) => (
            <Box
              key={`${t.id}-${stacks[t.id].gen}`}
              sx={{
                display: t.id === activeId ? "block" : "none",
                height: "100%",
                overflowY: "auto",
                scrollbarWidth: "none",
                "&::-webkit-scrollbar": { display: "none" },
              }}
            >
              <RouterProvider router={stacks[t.id].router} />
            </Box>
          ))}
        </Box>

        {IS_MOBILE && !navHidden && (
          <Paper elevation={3} sx={{ pb: "env(safe-area-inset-bottom)" }}>
            <BottomNavigation
              showLabels
              value={activeId}
              onChange={(_, id) => goTo(tabs.find((t) => t.id === id)!)}
            >
              {tabs.map((t) => (
                <BottomNavigationAction key={t.id} value={t.id} label={t.label} icon={t.icon} />
              ))}
            </BottomNavigation>
          </Paper>
        )}
      </Box>
    </TabsContext.Provider>
  );
}
