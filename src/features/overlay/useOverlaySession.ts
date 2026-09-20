import { useEffect, useRef, useState } from 'react';

import { events } from '@/lib/ipc';

/**
 * The overlay's visual state.
 *
 * Deliberately not the same enum as the backend's `DictationState`: the backend knows
 * about idle, listening and processing, while the overlay additionally has to show
 * what *happened* - done, cancelled, failed - for long enough to be read.
 */
export type CapsuleState =
  | 'hidden'
  | 'listening'
  | 'processing'
  | 'done'
  | 'cancelled'
  | 'error';

/**
 * Minimum time the processing state is shown.
 *
 * Transcription is often faster than this. A spinner that appears and vanishes within
 * three frames reads as a glitch rather than as speed, so the state is held briefly
 * even when the work is already finished.
 */
const MIN_PROCESSING_MS = 180;

/** How long each terminal state stays on screen before the overlay exits. */
const HOLD_MS: Partial<Record<CapsuleState, number>> = {
  done: 420,
  cancelled: 200,
  error: 3200,
};

interface Session {
  state: CapsuleState;
  message: string | null;
  /** Latest perceptual level, read every frame by the waveform. */
  levelRef: React.RefObject<number>;
}

export function useOverlaySession(): Session {
  const [state, setState] = useState<CapsuleState>('hidden');
  const [message, setMessage] = useState<string | null>(null);

  // A ref rather than state: the waveform reads this sixty times a second, and a
  // re-render per audio packet would cost far more than the animation itself.
  const levelRef = useRef(0);
  const processingSince = useRef(0);

  useEffect(() => {
    const timers: number[] = [];

    /** Move to a terminal state, then disappear after it has been seen. */
    const settle = (next: CapsuleState, text: string | null = null) => {
      const elapsed = Date.now() - processingSince.current;
      const wait = Math.max(0, MIN_PROCESSING_MS - elapsed);

      timers.push(
        window.setTimeout(() => {
          setState(next);
          setMessage(text);
          timers.push(
            window.setTimeout(() => {
              setState('hidden');
              setMessage(null);
            }, HOLD_MS[next] ?? 400),
          );
        }, wait),
      );
    };

    const unlisten = Promise.all([
      events.levelMeasured.listen((event) => {
        // Already 0..1 and already gated against this microphone's noise floor -
        // see `audio::SpeechLevel`. Nullable only because a Rust f32 can be NaN,
        // which JSON cannot represent.
        levelRef.current = event.payload.speech ?? 0;
      }),

      events.dictationStateChanged.listen((event) => {
        const next = event.payload.state;
        if (next === 'listening') {
          levelRef.current = 0;
          processingSince.current = 0;
          setState('listening');
          setMessage(null);
        } else if (next === 'processing') {
          processingSince.current = Date.now();
          setState('processing');
        }
        // 'idle' is not handled here: the terminal state that follows decides when
        // the overlay disappears, so that done and error stay readable.
      }),

      events.transcriptProduced.listen(() => settle('done')),

      events.dictationDiscarded.listen((event) => {
        // A very short press is a mistap, not a failure - it gets the quiet
        // cancelled treatment rather than an error the user has to read.
        const reason = event.payload;
        if (reason === 'tooShort') settle('cancelled');
        else if (reason === 'silent') settle('cancelled');
        else settle('error', "Didn't catch that");
      }),
    ]);

    return () => {
      timers.forEach((timer) => window.clearTimeout(timer));
      void unlisten.then((offs) => offs.forEach((off) => off()));
    };
  }, []);

  return { state, message, levelRef };
}
