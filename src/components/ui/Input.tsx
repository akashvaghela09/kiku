import { forwardRef } from 'react';
import { X } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

import { cn } from '@/lib/cn';

/**
 * A text field. The search box is this with `icon={Search} clearable`, which is why
 * there is no separate SearchInput.
 */

export interface InputProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'size'> {
  size?: 'sm' | 'md' | undefined;
  icon?: LucideIcon | undefined;
  clearable?: boolean | undefined;
  invalid?: boolean | undefined;
  onClear?: () => void | undefined;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { size = 'md', icon: Icon, clearable = false, invalid = false, onClear, className, ...rest },
  ref,
) {
  const height = size === 'sm' ? 'h-[26px]' : 'h-8';
  const iconSize = size === 'sm' ? 14 : 16;
  const showClear = clearable && String(rest.value ?? '').length > 0;

  return (
    <div className={cn('relative flex items-center', className)}>
      {Icon && (
        <Icon
          size={iconSize}
          className="pointer-events-none absolute left-2.5 text-muted"
          aria-hidden
        />
      )}

      <input
        ref={ref}
        className={cn(
          'w-full rounded-md border bg-surface-sunken text-primary',
          'placeholder:text-muted',
          'transition-colors duration-100',
          'disabled:cursor-not-allowed disabled:opacity-45',
          height,
          size === 'sm' ? 'text-xs' : 'text-ui',
          Icon ? 'pl-8' : 'pl-2.5',
          showClear ? 'pr-8' : 'pr-2.5',
          invalid ? 'border-danger' : 'border-border-strong',
        )}
        aria-invalid={invalid || undefined}
        {...rest}
      />

      {showClear && (
        <button
          type="button"
          onClick={onClear}
          aria-label="Clear"
          className="absolute right-1.5 rounded-sm p-1 text-muted hover:bg-surface-hover hover:text-primary"
        >
          <X size={iconSize - 2} aria-hidden />
        </button>
      )}
    </div>
  );
});
