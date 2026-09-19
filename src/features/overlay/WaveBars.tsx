import { useEffect, useRef } from 'react';

import {
  BAR_COUNT,
  MIN_SCALE,
  RING_COUNT,
  RING_INTERVAL,
  barScale,
  ringDistance,
  smooth,
} from './level';

/**
 * The waveform: fifteen bars, mirrored outward from the centre.
 *
 * The newest sample lands in the middle bar and propagates outward, so sound appears
 * to radiate from a source — the app's own mark in motion. A left-to-right scroll
 * would read as a recording timeline, and Kiku stores no audio, so that would be a
 * promise the product does not keep.
 *
 * Every frame writes `transform: scaleY()` and nothing else. On a promoted layer that
 * is compositor-only work: no layout, no paint. The loop is imperative and never calls
 * `setState`, because a React render per frame would cost more than the animation.
 */

/** Rendered height of a bar in pixels; it is only ever scaled down from here. */
const BAR_HEIGHT = 20;
const BAR_WIDTH = 4;
const BAR_GAP = 5;

/** Above this level the idle "breathing" pulse gives way to real audio. */
const SPEECH_THRESHOLD = 0.08;

interface WaveBarsProps {
  /** Latest perceptual level in 0..1, read every frame from a ref by the parent. */
  levelRef: React.RefObject<number>;
  /** Height of the row; the bars scale within it. */
  height?: number;
}

export function WaveBars({ levelRef, height = BAR_HEIGHT }: WaveBarsProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const barsRef = useRef<HTMLSpanElement[]>([]);

  useEffect(() => {
    const bars = barsRef.current;
    const container = containerRef.current;
    if (bars.length === 0 || !container) return;

    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    // Ring buffer: index 0 is the centre, index 7 the outermost pair.
    const history = new Float32Array(RING_COUNT);
    let current = 0;
    let lastPush = 0;
    let frame = 0;
    let timer = 0;

    const paint = () => {
      for (let index = 0; index < bars.length; index += 1) {
        const distance = Math.round(ringDistance(index));
        const scale = barScale(history[distance] ?? 0, distance);
        // Writing style.transform directly, not via React, keeps this off the
        // reconciler entirely.
        bars[index]!.style.transform = `scaleY(${scale.toFixed(3)})`;
      }

      // Pause the breathing pulse while there is real audio, so the two never fight.
      const speaking = Math.max(...history) > SPEECH_THRESHOLD;
      container.dataset.speaking = speaking ? 'true' : 'false';
    };

    const step = (now: number) => {
      current = smooth(current, levelRef.current ?? 0);

      if (now - lastPush >= RING_INTERVAL) {
        lastPush = now;
        history.copyWithin(1, 0);
        history[0] = current;
      }

      paint();
      frame = requestAnimationFrame(step);
    };

    if (reduced) {
      // The waveform is information — "am I being heard?" — so it does not stop when
      // motion is reduced. It slows to 10Hz and drops the per-bar animation.
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
      className="wave-bars flex items-center"
      style={{ gap: BAR_GAP, height }}
      aria-hidden
    >
      {Array.from({ length: BAR_COUNT }, (_, index) => (
        <span
          key={index}
          ref={(element) => {
            if (element) barsRef.current[index] = element;
          }}
          style={{
            width: BAR_WIDTH,
            height,
            borderRadius: BAR_WIDTH / 2,
            background: 'var(--ov-bar)',
            transformOrigin: 'center',
            transform: `scaleY(${MIN_SCALE})`,
            willChange: 'transform',
          }}
        />
      ))}
    </div>
  );
}
