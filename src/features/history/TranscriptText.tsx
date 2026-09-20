import { useEffect, useRef, useState } from 'react';

import { cn } from '@/lib/cn';

/**
 * A transcript, clamped to a few lines with a "Read more" when it does not fit.
 *
 * The button appears only when the text is genuinely cut off, which is measured rather
 * than guessed: a character-count threshold gets it wrong at every window width, and a
 * "Read more" on something already fully visible is worse than no button at all.
 */
interface TranscriptTextProps {
  text: string;
  /** Lines to show before clamping. */
  lines?: number;
  onExpand: () => void;
}

export function TranscriptText({ text, lines = 3, onExpand }: TranscriptTextProps) {
  // A span, not a p: this renders inside Row's title span, and flow content inside
  // phrasing content is invalid nesting.
  const ref = useRef<HTMLSpanElement>(null);
  const [clipped, setClipped] = useState(false);

  useEffect(() => {
    const element = ref.current;
    if (!element) return;

    // scrollHeight exceeding clientHeight is the browser telling us the clamp bit.
    const measure = () => setClipped(element.scrollHeight > element.clientHeight + 1);
    measure();

    // Re-measure on resize: the same text clamps at one width and not another.
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [text, lines]);

  return (
    <span className="block">
      <span
        ref={ref}
        className={cn('prose-transcript block text-base')}
        style={{
          display: '-webkit-box',
          WebkitBoxOrient: 'vertical',
          WebkitLineClamp: lines,
          overflow: 'hidden',
        }}
      >
        {text}
      </span>

      {clipped && (
        <button
          type="button"
          onClick={onExpand}
          className="mt-0.5 rounded-sm text-xs font-medium text-accent-text hover:underline"
        >
          Read more
        </button>
      )}
    </span>
  );
}
