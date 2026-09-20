import type { LucideIcon } from 'lucide-react';

import { cn } from '@/lib/cn';

/** A value slider, used for feedback volume. */
export interface SliderProps {
  value: number;
  min?: number | undefined;
  max?: number | undefined;
  step?: number | undefined;
  onChange: (value: number) => void;
  /** Fires on release - the natural moment to play a preview once. */
  onCommit?: (value: number) => void | undefined;
  leadingIcon?: LucideIcon | undefined;
  trailingIcon?: LucideIcon | undefined;
  formatValue?: (value: number) => string | undefined;
  'aria-label'?: string | undefined;
}

export function Slider({
  value,
  min = 0,
  max = 1,
  step = 0.05,
  onChange,
  onCommit,
  leadingIcon: Leading,
  trailingIcon: Trailing,
  formatValue,
  'aria-label': ariaLabel,
}: SliderProps) {
  return (
    <div className="flex items-center gap-2.5">
      {Leading && <Leading size={14} className="shrink-0 text-muted" aria-hidden />}

      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        aria-label={ariaLabel}
        onChange={(event) => onChange(Number(event.target.value))}
        onPointerUp={() => onCommit?.(value)}
        onKeyUp={() => onCommit?.(value)}
        className={cn('kiku-slider h-1.5 w-32 cursor-pointer appearance-none rounded-pill')}
        style={{
          background: `linear-gradient(to right, var(--accent) ${
            ((value - min) / (max - min)) * 100
          }%, var(--surface-active) 0%)`,
        }}
      />

      {Trailing && <Trailing size={14} className="shrink-0 text-muted" aria-hidden />}

      {formatValue && (
        <span className="w-10 shrink-0 text-right font-mono text-xs text-muted">
          {formatValue(value)}
        </span>
      )}
    </div>
  );
}
