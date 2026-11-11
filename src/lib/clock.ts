import { isBoolean, useSetting } from "./settings";

/** Whether times of day read as 13:05 and not as 1:05 PM. On unless the user turned it off. */
export const useClock24 = () => useSetting("clock24", true, isBoolean);

/**
 * `hourCycle` and not `hour12` on purpose. `hour12: false` prints midnight as 24:05 in some
 * locales, `h23` always prints 00:05.
 */
export const formatClock = (ms: number, clock24: boolean) =>
  new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hourCycle: clock24 ? "h23" : "h12" });
