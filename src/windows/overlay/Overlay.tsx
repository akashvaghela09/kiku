import { Check, TriangleAlert, X } from 'lucide-react';

import { WaveBars } from '@/features/overlay/WaveBars';
import { type CapsuleState, useOverlaySession } from '@/features/overlay/useOverlaySession';

/**
 * The capsule - Kiku's listening indicator, floating over whatever the user is in.
 *
 * It is read peripherally, never focally, so it carries no text in its ordinary states
 * and communicates entirely through width and motion. Height is constant at 44px and
 * the radius is always half of that, so every state is the same pill at a different
 * length; only the width animates.
 *
 * The capsule is always dark, in both OS themes. It sits on an arbitrary application's
 * window rather than on one of ours, so "match the system theme" would be a promise
 * about a surface we do not own.
 */

/** Width of the capsule in each state, in pixels. */
const WIDTH: Record<Exclude<CapsuleState, 'hidden'>, number> = {
  listening: 168,
  processing: 132,
  done: 108,
  cancelled: 44,
  error: 260,
};

const HEIGHT = 44;

export function Overlay() {
  const { state, message, levelRef } = useOverlaySession();

  if (state === 'hidden') return null;

  return (
    // Anchored to the bottom of the window, not centred in it: the window's height is
    // whatever the platform decides, but its bottom edge is pinned to the work area,
    // so measuring the capsule from the bottom is the only stable arrangement.
    <div className="flex h-full w-full items-end justify-center" style={{ paddingBottom: 40 }}>
      <div
        data-state={state}
        className="capsule flex items-center justify-center overflow-hidden"
        style={{
          width: WIDTH[state],
          height: HEIGHT,
          borderRadius: HEIGHT / 2,
          background: 'var(--ov-bg)',
          boxShadow: 'var(--ov-shadow)',
          border: '1px solid var(--ov-hairline)',
        }}
        role="status"
        aria-live="polite"
        aria-label={LABELS[state]}
      >
        <CapsuleContents state={state} message={message} levelRef={levelRef} />
      </div>
    </div>
  );
}

const LABELS: Record<Exclude<CapsuleState, 'hidden'>, string> = {
  listening: 'Listening',
  processing: 'Transcribing',
  done: 'Done',
  cancelled: 'Cancelled',
  error: 'Nothing was transcribed',
};

interface ContentsProps {
  state: Exclude<CapsuleState, 'hidden'>;
  message: string | null;
  levelRef: React.RefObject<number>;
}

function CapsuleContents({ state, message, levelRef }: ContentsProps) {
  switch (state) {
    case 'listening':
      return <WaveBars levelRef={levelRef} />;

    case 'processing':
      return <ProcessingDots />;

    case 'done':
      return <Check size={18} strokeWidth={2.5} style={{ color: 'var(--ov-success)' }} />;

    case 'cancelled':
      return <X size={14} strokeWidth={2.5} style={{ color: 'var(--ov-text)', opacity: 0.6 }} />;

    case 'error':
      return (
        <div className="flex w-full items-center gap-2 px-4">
          <span
            aria-hidden
            style={{
              width: 3,
              height: 20,
              borderRadius: 2,
              background: 'var(--ov-danger)',
              flexShrink: 0,
            }}
          />
          <TriangleAlert size={16} style={{ color: 'var(--ov-danger)', flexShrink: 0 }} />
          <span
            className="truncate text-ui"
            style={{ color: 'var(--ov-text)' }}
          >
            {message ?? 'Nothing was transcribed'}
          </span>
        </div>
      );
  }
}

/** Three dots, offset in phase so they read as one motion rather than three. */
function ProcessingDots() {
  return (
    <div className="flex items-center gap-[6px]" aria-hidden>
      {[0, 1, 2].map((index) => (
        <span
          key={index}
          className="processing-dot"
          style={{
            width: 6,
            height: 6,
            borderRadius: 3,
            background: 'var(--ov-bar)',
            animationDelay: `${index * 140}ms`,
          }}
        />
      ))}
    </div>
  );
}
