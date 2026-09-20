/**
 * Level meter shaping.
 *
 * The hard part - deciding whether a sound is speech or just the room - is done in
 * Rust, where it has the history to learn each microphone's noise floor and is covered
 * by tests. See `audio::SpeechLevel`. What arrives here is already 0 when nobody is
 * talking, so everything below is purely about how the marks move.
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

/**
 * Marks in the row. The centre mark carries the newest sample.
 *
 * Odd, so there is a true centre for the ripple to radiate from - `ringDistance`
 * returns an exact integer only for an odd count.
 */
export const MARK_COUNT = 7;

/** Distinct rings out from the centre: the centre itself plus three on each side. */
export const RING_COUNT = 4;

/** How often the newest level is pushed into the ring buffer, in milliseconds. */
export const RING_INTERVAL = 90;

/**
 * Width of a mark, and its height at rest - equal, so silence is a circle.
 *
 * Small on purpose. The mark is a bar for as long as someone is talking, and a bar
 * five pixels wide and thirty tall reads as a waveform where one eight wide and
 * twenty-six tall reads as a stretched dot.
 */
export const MARK_SIZE = 5;

/** Height of the tallest mark. Six times the resting size, inside a 44px capsule. */
export const MARK_MAX = 30;

/** Gap between marks. */
export const MARK_GAP = 9;

/** How fast a mark eases toward its target height each frame. About two frames. */
export const RENDER_EASE = 0.5;

/**
 * Curve applied to the level before it becomes a height.
 *
 * Ordinary speech sits around the middle of the range and rarely approaches the top,
 * so a straight mapping spends most of its travel on levels that never arrive and the
 * meter looks inert. Below 1 this lifts the middle without touching either end - the
 * floor stays flat during silence, and a shout still reaches full height.
 */
const SHAPE = 0.7;

/**
 * Height of one mark in pixels.
 *
 * `distance` is how many rings the mark sits from the centre. Energy decays as it
 * travels outward, which is what makes the shape radiate from the middle rather than
 * pump uniformly.
 *
 * Height, not `scaleY`. A squashed transform cannot keep a round cap - `border-radius`
 * is resolved before the transform, so a 4px radius on a bar scaled to a fifth renders
 * with a 0.8px vertical radius, which is what made the resting marks read as squares.
 * At a real height of MARK_SIZE the same 4px radius is exactly half the width, so
 * silence is a true circle and every height above it is a true stadium.
 */
export function markHeight(historyAtRing: number, distance: number): number {
  const falloff = 1 - distance * 0.09;
  const shaped = historyAtRing > 0 ? Math.pow(historyAtRing, SHAPE) : 0;
  return MARK_SIZE + shaped * falloff * (MARK_MAX - MARK_SIZE);
}

/**
 * Opacity of one mark, by ring distance.
 *
 * Set once at mount and never touched again, so it costs nothing per frame. It gives
 * the row a soft terminus instead of a hard-cut rule, and quietly pre-announces that
 * motion radiates outward from the centre.
 */
export function markOpacity(distance: number): number {
  return 1 - distance * 0.1;
}

/** Ring distance of a mark from the centre of the row. */
export function ringDistance(index: number): number {
  return Math.abs(index - (MARK_COUNT - 1) / 2);
}
