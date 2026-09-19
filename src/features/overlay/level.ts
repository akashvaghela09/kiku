/**
 * Turning a raw RMS reading into something a waveform can show.
 *
 * Linear RMS is useless for this: speech occupies a narrow band near the top of it, so
 * bars driven by it barely move. Mapping through decibels spreads that band across the
 * full range, which is why a dB meter looks alive and a linear one looks broken.
 */

/** Below this, treat the signal as silence. */
const FLOOR_DB = -58;

/** At this level the bars are at full height — a loud voice close to a laptop mic. */
const CEILING_DB = -8;

/**
 * Map an RMS value in 0..1 to a perceptual level in 0..1.
 *
 * These two thresholds are the entire feel of the waveform and are deliberately the
 * only tunables.
 */
export function rmsToLevel(rms: number): number {
  if (!Number.isFinite(rms) || rms <= 0) return 0;
  const db = 20 * Math.log10(rms + 1e-7);
  const level = (db - FLOOR_DB) / (CEILING_DB - FLOOR_DB);
  return Math.min(1, Math.max(0, level));
}

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
