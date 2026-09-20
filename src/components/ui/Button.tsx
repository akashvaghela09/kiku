import { forwardRef, useState } from 'react';
import type { LucideIcon } from 'lucide-react';

import { cn } from '@/lib/cn';

/**
 * The only button in the application.
 *
 * `iconOnly` replaces what would otherwise be an IconButton component, and
 * `destructive` folds confirm-on-first-click in here rather than making every caller
 * remember to wire up a confirmation.
 */

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | undefined;
  size?: 'sm' | 'md' | 'lg' | undefined;
  icon?: LucideIcon | undefined;
  trailingIcon?: LucideIcon | undefined;
  /** Square button. Requires `label`, which becomes the accessible name. */
  iconOnly?: boolean | undefined;
  label?: string | undefined;
  loading?: boolean | undefined;
  /** Ask for confirmation on the first click, act on the second. */
  destructive?: boolean | undefined;
  fullWidth?: boolean | undefined;
}

const SIZES = {
  sm: { height: 'h-[26px]', padding: 'px-2', text: 'text-xs', icon: 14 },
  md: { height: 'h-8', padding: 'px-3', text: 'text-ui', icon: 16 },
  lg: { height: 'h-10', padding: 'px-4', text: 'text-base', icon: 18 },
} as const;

const VARIANTS = {
  primary:
    'bg-accent text-on-accent hover:bg-accent-hover active:bg-accent-active border border-transparent',
  secondary:
    'bg-surface-raised text-primary border border-border-strong hover:bg-surface-hover active:bg-surface-active',
  ghost: 'bg-transparent text-secondary border border-transparent hover:bg-surface-hover',
  danger: 'bg-transparent text-danger border border-transparent hover:bg-danger-wash',
} as const;

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  {
    variant = 'secondary',
    size = 'md',
    icon: Icon,
    trailingIcon: TrailingIcon,
    iconOnly = false,
    label,
    loading = false,
    destructive = false,
    fullWidth = false,
    className,
    children,
    onClick,
    disabled,
    ...rest
  },
  ref,
) {
  const [armed, setArmed] = useState(false);
  const metrics = SIZES[size];

  const handleClick = (event: React.MouseEvent<HTMLButtonElement>) => {
    if (destructive && !armed) {
      // One click arms, the next commits - cheaper than a dialog for actions that are
      // annoying rather than catastrophic, and it cannot be dismissed by accident.
      event.preventDefault();
      setArmed(true);
      window.setTimeout(() => setArmed(false), 3000);
      return;
    }
    setArmed(false);
    onClick?.(event);
  };

  const text = armed ? 'Click again to confirm' : (children ?? label);

  // An icon-only button has nowhere to put the confirmation text, so it goes to the
  // accessible name and the tooltip instead. Without this, arming a destructive
  // icon button showed only a red tint and kept its original label.
  const accessibleName = armed && iconOnly ? 'Click again to confirm' : label;

  return (
    <button
      ref={ref}
      type="button"
      onClick={handleClick}
      disabled={disabled ?? loading}
      aria-label={iconOnly ? accessibleName : undefined}
      title={iconOnly ? accessibleName : undefined}
      className={cn(
        'inline-flex shrink-0 items-center justify-center gap-1.5 rounded-md font-medium',
        'transition-colors duration-100',
        'disabled:cursor-not-allowed disabled:opacity-45',
        metrics.height,
        metrics.text,
        iconOnly ? 'aspect-square p-0' : metrics.padding,
        armed ? VARIANTS.danger : VARIANTS[variant],
        fullWidth && 'w-full',
        className,
        // After className deliberately: a caller styling an icon button must not be
        // able to hide the confirmation, which is the only signal that a destructive
        // click has been armed.
        armed && 'bg-danger-wash text-danger opacity-100',
      )}
      {...rest}
    >
      {loading ? (
        <span className="inline-flex gap-0.5" aria-hidden>
          {[0, 1, 2].map((index) => (
            <span
              key={index}
              className="loading-dot size-1 rounded-full bg-current"
              style={{ animationDelay: `${index * 140}ms` }}
            />
          ))}
        </span>
      ) : (
        Icon && <Icon size={metrics.icon} strokeWidth={2} aria-hidden />
      )}

      {!iconOnly && text}

      {!iconOnly && TrailingIcon && !loading && (
        <TrailingIcon size={metrics.icon} strokeWidth={2} aria-hidden />
      )}
    </button>
  );
});
