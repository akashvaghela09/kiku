/**
 * Waveform shaping.
 *
 * The hard part — deciding whether a sound is speech or just the room — is done in
 * Rust, where it has the history to learn each microphone's noise floor and is covered
 * by tests. See `audio::SpeechLevel`. What arrives here is already 0 when nobody is
 * talking, so everything below is purely about how the bars move.
 */

/**
 * One step of VU-style smoothing: fast attack, slow release.
 *
 * Rising quickly and falling slowly is the standard meter ballistic, and the reason a
 * good meter reads as responsive rather than twitchy. Roughly three frames to rise and
 * nine to fall at 60fps.
 */
export function smooth(current: number, target: number): number {
  const rate = target > current ? 0.55 : 0.14;
  return current + (target - current) * rate;
}

/** Bars, left to right. The centre bar carries the newest sample. */
export const BAR_COUNT = 15;

/** Distinct rings out from the centre: the centre itself plus seven on each side. */
export const RING_COUNT = 8;

/** How often the newest level is pushed into the ring buffer, in milliseconds. */
export const RING_INTERVAL = 70;

/** Shortest bar, as a fraction of the tallest. Never zero — zero reads as broken. */
export const MIN_SCALE = 0.2;

/**
 * Height of one bar, as a `scaleY` factor.
 *
 * `distance` is how many rings the bar sits from the centre. Energy decays as it
 * travels outward, which is what makes the shape radiate from the middle rather than
 * pump uniformly — the mark's own shortening arcs, animated.
 */
export function barScale(historyAtRing: number, distance: number): number {
  const falloff = 1 - distance * 0.055;
  const value = historyAtRing * falloff;
  return MIN_SCALE + value * (1 - MIN_SCALE);
}

/** Ring distance of a bar from the centre of the row. */
export function ringDistance(index: number): number {
  return Math.abs(index - (BAR_COUNT - 1) / 2);
}
