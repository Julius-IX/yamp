// Read from the user agent so no native plugin is needed. The Tauri webviews report the
// operating system in it. Android also says Linux, so Linux is only true without Android.
const ua = navigator.userAgent;

export const IS_ANDROID = /Android/i.test(ua);

// iPadOS asks for desktop sites by default and reports as a Mac. A Mac never has a touch
// screen, so that tells the two apart.
export const IS_IOS =
  /iPhone|iPad|iPod/i.test(ua) || (/Macintosh/.test(ua) && navigator.maxTouchPoints > 1);

export const IS_WINDOWS = /Windows/i.test(ua);
export const IS_MAC = /Macintosh/.test(ua) && !IS_IOS;
export const IS_LINUX = /Linux/i.test(ua) && !IS_ANDROID;

/** Mobile gets a bottom navigation bar, desktop gets a sidebar. */
export const IS_MOBILE = IS_ANDROID || IS_IOS;
export const IS_DESKTOP = !IS_MOBILE;
