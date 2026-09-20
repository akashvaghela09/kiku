import { useEffect, useRef } from 'react';

import {
  MARK_COUNT,
  MARK_GAP,
  MARK_MAX,
  MARK_SIZE,
  RENDER_EASE,
  RING_COUNT,
  RING_INTERVAL,
  markHeight,
  markOpacity,
  ringDistance,
  smooth,
} from './level';

/**
 * The seven marks at the centre of the capsule.
 *
 * One row of elements carries the whole dictation. While listening they are bars whose
 * heights follow the voice; while the transcript is being produced and pasted the same
 * seven collapse to circles and pulse in a travelling wave. Nothing is mounted or
 * unmounted between those states and the row never changes width, so the capsule reads
 * as one object changing behaviour rather than as three different indicators.
 *
 * The newest sample lands in the middle mark and propagates outward, so sound appears
 * to radiate from a source. A left-to-right scroll would read as a recording timeline,
 * and Kiku stores no audio, so that would be a promise the product does not keep.
 *
 * Listening drives `height` rather than `transform`. That is a layout property, which
 * this file would ordinarily avoid, but it is the only way to keep a correct cap at
 * every size: `border-radius` resolves before a transform, so a scaled bar renders with
 * squashed ends. Seven boxes of fixed width inside a fixed-height row, in a window that
 * contains nothing else, is a layout cost worth paying for a shape that is right.
 */

/** Above this level the idle breathing pulse gives way to real audio. */
const SPEECH_THRESHOLD = 0.08;

/**
 * The resting pulse.
 *
 * Silence must not read as "crashed", so the row breathes while nothing is being said.
 * Slow and shallow on a high floor: a mark this small swinging further or faster stops
 * reading as breathing and starts reading as a flicker.
 */
const PULSE_PERIOD = 2800;
const PULSE_LOW = 0.62;
const PULSE_HIGH = 0.92;

/** How fast the row crosses between breathing and speaking. Roughly 220ms either way. */
const GATE_EASE = 0.2;

interface CapsuleMarksProps {
  /**
   * Latest perceptual level in 0..1, read every frame from a ref by the parent. Absent
   * while working, when the marks are driven by CSS rather than by audio.
   */
  levelRef?: React.RefObject<number> | undefined;
}

export function CapsuleMarks({ levelRef }: CapsuleMarksProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const marksRef = useRef<HTMLSpanElement[]>([]);
  const working = !levelRef;

  useEffect(() => {
    if (!levelRef) return;

    const marks = marksRef.current;
    const container = containerRef.current;
    if (marks.length === 0 || !container) return;

    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    // Ring buffer: index 0 is the centre, index 3 the outermost pair.
    const history = new Float32Array(RING_COUNT);
    // What each mark is currently showing, as opposed to what it is heading toward.
    const rendered = new Float32Array(MARK_COUNT).fill(MARK_SIZE);
    let current = 0;
    let lastPush = 0;
    let gate = 0;
    let frame = 0;
    let timer = 0;

    const paint = () => {
      for (let index = 0; index < marks.length; index += 1) {
        const distance = ringDistance(index);
        const target = markHeight(history[distance] ?? 0, distance);
        // Ease toward the target rather than snapping to it: a ring's value only
        // changes every RING_INTERVAL, and stepping to it in a single frame pops.
        const previous = rendered[index] ?? MARK_SIZE;
        const eased = previous + (target - previous) * RENDER_EASE;
        rendered[index] = eased;
        // Writing style directly, not via React, keeps this off the reconciler.
        marks[index]!.style.height = `${eased.toFixed(2)}px`;
      }
    };

    const step = (now: number) => {
      current = smooth(current, levelRef.current ?? 0);

      if (now - lastPush >= RING_INTERVAL) {
        lastPush = now;
        history.copyWithin(1, 0);
        history[0] = current;
      }

      paint();

      // The idle pulse is driven here rather than by a CSS animation so that handing
      // over to speech is a crossfade. Cancelling a CSS animation when speech starts
      // jumps from mid-cycle straight to full opacity, and a transition cannot smooth
      // that: a transition's start value excludes animation effects, so it would
      // compute 1 to 1 and never fire.
      const speaking = Math.max(...history) > SPEECH_THRESHOLD;
      gate += ((speaking ? 1 : 0) - gate) * GATE_EASE;
      const phase = 0.5 - 0.5 * Math.cos((now / PULSE_PERIOD) * Math.PI * 2);
      const idle = PULSE_LOW + (PULSE_HIGH - PULSE_LOW) * phase;
      container.style.opacity = (idle + (1 - idle) * gate).toFixed(3);

      frame = requestAnimationFrame(step);
    };

    if (reduced) {
      // The meter is information - "am I being heard?" - so it does not stop when
      // motion is reduced. It slows to 10Hz and drops the breathing pulse.
      container.style.opacity = '0.85';
      timer = window.setInterval(() => {
        current = levelRef.current ?? 0;
        history.fill(current);
        paint();
      }, 100);
    } else {
      frame = requestAnimationFrame(step);
    }

    return () => {
      cancelAnimationFrame(frame);
      window.clearInterval(timer);
    };
  }, [levelRef]);

  return (
    <div
      ref={containerRef}
      className="flex items-center"
      style={{ gap: MARK_GAP, height: MARK_MAX }}
      aria-hidden
    >
      {Array.from({ length: MARK_COUNT }, (_, index) => (
        <span
          key={index}
          ref={(element) => {
            if (element) marksRef.current[index] = element;
          }}
          className={working ? 'mark-pulse' : undefined}
          style={{
            width: MARK_SIZE,
            // Always half the width, so a mark at rest is a circle and a mark at any
            // height above that is a stadium. Never distorted, because nothing scales.
            borderRadius: MARK_SIZE / 2,
            height: MARK_SIZE,
            background: 'var(--ov-bar)',
            // Set once: the row's edges soften so it has no hard terminus.
            opacity: markOpacity(ringDistance(index)),
            flexShrink: 0,
            ...(working
              ? // Evenly phased rather than simultaneous: the pulse travels the row
                // once per cycle, which reads as one motion instead of seven.
                { animationDelay: `${index * 140}ms` }
              : { willChange: 'height' }),
          }}
        />
      ))}
    </div>
  );
}
