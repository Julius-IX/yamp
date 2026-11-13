/**
 * Routes the Android back press. The activity asks `window.yampBack` whether the frontend used
 * the press and hides itself when the answer is no, so the player keeps running. Nothing is
 * pushed to the browser history for this, which would otherwise take a press to pop.
 *
 * A press goes to the newest guard first. A guard belongs to an overlay that is not part of any
 * tab router, like the full screen player, and the tab behind it must not move. Without a guard
 * the press goes to the fallback, which the app layout sets to navigate its tabs.
 */
type Guard = { onBack: () => void };

const guards: Guard[] = [];
let fallback: (() => boolean) | null = null;

/** Returns whether the frontend used the press. */
function handleBack() {
  // the guard stays until its owner releases it, so a guard that ignores the press keeps holding it
  const guard = guards[guards.length - 1];
  if (guard) {
    guard.onBack();
    return true;
  }
  return fallback?.() ?? false;
}

declare global {
  interface Window {
    yampBack?: () => boolean;
  }
}
window.yampBack = handleBack;

/** Returns the release function, meant to be the cleanup of an effect. */
export function pushBackGuard(onBack: () => void) {
  const guard: Guard = { onBack };
  guards.push(guard);
  return () => {
    const at = guards.indexOf(guard);
    if (at >= 0) guards.splice(at, 1);
  };
}

/** Sets what a press does when no guard is held. It returns true when it navigated. */
export function setBackFallback(onBack: () => boolean) {
  fallback = onBack;
  return () => {
    if (fallback === onBack) fallback = null;
  };
}
