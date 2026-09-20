import { cn } from '@/lib/cn';

/** An on/off switch. */
export interface ToggleProps {
  checked: boolean;
  onChange: (value: boolean) => void;
  size?: 'sm' | 'md' | undefined;
  disabled?: boolean | undefined;
  'aria-label'?: string | undefined;
}

export function Toggle({
  checked,
  onChange,
  size = 'md',
  disabled = false,
  'aria-label': ariaLabel,
}: ToggleProps) {
  const track = size === 'sm' ? 'h-4 w-7' : 'h-5 w-[34px]';
  const knob = size === 'sm' ? 'size-3' : 'size-4';
  const travel = size === 'sm' ? 'translate-x-3' : 'translate-x-3.5';

  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        'relative inline-flex shrink-0 items-center rounded-pill p-0.5',
        'transition-colors duration-150',
        'disabled:cursor-not-allowed disabled:opacity-45',
        track,
        checked ? 'bg-accent' : 'bg-surface-active border border-border-strong',
      )}
    >
      <span
        className={cn(
          'rounded-pill bg-surface shadow-sm transition-transform duration-150',
          knob,
          checked ? travel : 'translate-x-0',
        )}
        style={checked ? undefined : { background: 'var(--text-muted)' }}
        aria-hidden
      />
    </button>
  );
}
