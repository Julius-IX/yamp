/**
 * Lets an overlay that is not part of any tab router take the Android back press, like the full
 * screen player. The app layout turns a back press into a tab navigation, so without a guard
 * the tab would change underneath an open overlay.
 *
 * A guard pushes one history entry, which keeps the entry the layout reserved for itself in
 * place. This listener is added when the module loads, ahead of the layout's own, and stops the
 * event from reaching it whenever a guard handles the press.
 */
type Guard = { onBack: () => void };

const guards: Guard[] = [];
/** Pops caused by a guard that was released from the UI, which are not back presses. */
let ignored = 0;

window.addEventListener("popstate", (e) => {
  if (ignored > 0) {
    ignored--;
    e.stopImmediatePropagation();
    return;
  }
  const guard = guards.pop();
  if (!guard) return;
  e.stopImmediatePropagation();
  guard.onBack();
});

/** Returns the release function, meant to be the cleanup of an effect. */
export function pushBackGuard(onBack: () => void) {
  const guard: Guard = { onBack };
  history.pushState({ guard: true }, "");
  guards.push(guard);
  return () => {
    const at = guards.indexOf(guard);
    // already gone means the back press consumed it, and its entry was popped with it
    if (at < 0) return;
    guards.splice(at, 1);
    ignored++;
    history.back();
  };
}
