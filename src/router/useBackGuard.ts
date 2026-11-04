import { useEffect, useRef } from "react";
import { pushBackGuard } from "./backGuard";

/**
 * Holds the Android back press while `active` is true and calls `onBack` for it. Meant for
 * dialogs and sheets, since the press would otherwise navigate the tab behind them. The latest
 * `onBack` is used without registering again, so an inline callback is fine.
 */
export function useBackGuard(active: boolean, onBack: () => void) {
  const latest = useRef(onBack);
  latest.current = onBack;
  useEffect(() => {
    if (!active) return;
    return pushBackGuard(() => latest.current());
  }, [active]);
}
