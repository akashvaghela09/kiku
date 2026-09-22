/**
 * The shipped shortcuts, mirrored for the preset buttons in Settings.
 *
 * The definitions that matter live in Rust, in `src-tauri/src/hotkeys/defaults.rs`,
 * which is what a fresh install is actually bound to. This file exists because the
 * presets are offered *before* any binding is applied - they are what the buttons
 * would set - and a round trip to draw three buttons is not worth the machinery.
 *
 * Keep the two in step. If they drift, Rust wins: it is what the application runs.
 */

/** Whether this is a Mac, which is the only thing the defaults vary on. */
function isMacPlatform(): boolean {
  return navigator.platform.toUpperCase().includes('MAC');
}

/**
 * Hold to talk: a bare right-hand modifier, watched rather than registered.
 *
 * Mac keyboards have no right Control key, so macOS gets Right Option.
 */
export function defaultHold(): string {
  return isMacPlatform() ? 'RightAlt' : 'RightControl';
}

/** How that key is written for the person using it. */
export function defaultHoldLabel(): string {
  return isMacPlatform() ? 'Right ⌥' : 'Right Ctrl';
}

/**
 * Press once to start, again to stop - the chord route into hands-free recording.
 *
 * The primary route is a gesture rather than a binding: tapping the hold key twice
 * latches, and a third tap ends it. That is decided in Rust by the tap machine and is
 * identical on every platform, so it needs no entry here.
 */
export const DEFAULT_TOGGLE = 'Ctrl+Alt+Space';

/** For keyboards without a usable right-hand modifier. */
export const FALLBACK_HOLD = 'Ctrl+Shift+Space';
export const FALLBACK_TOGGLE = 'Ctrl+Alt+Space';

/** A single-key pair for keyboards whose right-hand modifiers are awkward. */
export const FUNCTION_HOLD = 'F9';
export const FUNCTION_TOGGLE = 'F10';
