import { cn } from '@/lib/cn';

/** A determinate or indeterminate progress bar. */
export interface ProgressProps {
  /** 0..1. Omit for indeterminate. */
  value?: number | undefined;
  label?: string | undefined;
  detail?: string | undefined;
  tone?: 'accent' | 'success' | 'danger' | undefined;
  size?: 'sm' | 'md' | undefined;
}

const TONES = {
  accent: 'bg-accent',
  success: 'bg-success',
  danger: 'bg-danger',
} as const;

export function Progress({ value, label, detail, tone = 'accent', size = 'md' }: ProgressProps) {
  const indeterminate = value === undefined;
  const percent = indeterminate ? 0 : Math.round(Math.min(1, Math.max(0, value)) * 100);

  return (
    <div className="w-full">
      {(label ?? detail) && (
        <div className="mb-1.5 flex items-baseline justify-between gap-3">
          {label && <span className="text-ui text-primary">{label}</span>}
          {detail && <span className="font-mono text-xs text-muted">{detail}</span>}
        </div>
      )}

      <div
        className={cn(
          'w-full overflow-hidden rounded-pill bg-surface-active',
          size === 'sm' ? 'h-[3px]' : 'h-1.5',
        )}
        role="progressbar"
        aria-valuenow={indeterminate ? undefined : percent}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={label}
      >
        <div
          className={cn(
            'h-full rounded-pill',
            TONES[tone],
            indeterminate ? 'progress-indeterminate w-1/3' : 'transition-[width] duration-200',
          )}
          style={indeterminate ? undefined : { width: `${percent}%` }}
        />
      </div>
    </div>
  );
}
