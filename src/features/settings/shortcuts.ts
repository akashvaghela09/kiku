/**
 * The dictation key, mirrored for the preset button in Settings.
 *
 * The definition that matters lives in Rust, in `src-tauri/src/hotkeys/defaults.rs`,
 * which is what a fresh install is bound to. This file exists because the preset is
 * offered *before* any binding is applied - it is what the button would set - and a
 * round trip to draw one button is not worth the machinery.
 *
 * Keep the two in step. If they drift, Rust wins: it is what the application runs.
 */

/** Whether this is a Mac, which is the only thing the default varies on. */
function isMacPlatform(): boolean {
  return navigator.platform.toUpperCase().includes('MAC');
}

/**
 * The key Kiku ships bound to.
 *
 * A bare right-hand modifier, watched rather than registered, carrying both modes:
 * hold it to talk, tap it twice to keep listening. Mac keyboards have no right Control
 * key, so macOS gets Right Option.
 */
export function defaultHold(): string {
  return isMacPlatform() ? 'RightAlt' : 'RightControl';
}

/** How that key is written for the person using it. */
export function defaultHoldLabel(): string {
  return isMacPlatform() ? 'Right ⌥' : 'Right Ctrl';
}
