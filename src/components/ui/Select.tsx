import { useEffect, useRef, useState } from 'react';
import { Check, ChevronDown } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

import { cn } from '@/lib/cn';

/**
 * A custom listbox rather than a native `<select>`.
 *
 * Two reasons, both forced: native select styling cannot be made consistent across
 * the three platforms, and it cannot render the two-line options the microphone and
 * model pickers need.
 */

export interface SelectOption<T> {
  value: T;
  label: string;
  description?: string | undefined;
  icon?: LucideIcon | undefined;
  badge?: string | undefined;
  disabled?: boolean | undefined;
}

export interface SelectProps<T> {
  value: T;
  options: SelectOption<T>[];
  onChange: (value: T) => void;
  placeholder?: string | undefined;
  size?: 'sm' | 'md' | undefined;
  /** Escape hatch so richer pickers reuse this instead of becoming bespoke. */
  renderOption?: ((option: SelectOption<T>) => React.ReactNode) | undefined;
  /** Fired as the list opens, for options that can change while the app is running. */
  onOpen?: (() => void) | undefined;
  'aria-label'?: string | undefined;
}

export function Select<T extends string | number>({
  value,
  options,
  onChange,
  placeholder = 'Select…',
  size = 'md',
  renderOption,
  onOpen,
  'aria-label': ariaLabel,
}: SelectProps<T>) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);

  const selected = options.find((option) => option.value === value);

  useEffect(() => {
    if (!open) return;

    const onPointerDown = (event: PointerEvent) => {
      if (!containerRef.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener('pointerdown', onPointerDown);
    return () => document.removeEventListener('pointerdown', onPointerDown);
  }, [open]);

  useEffect(() => {
    if (open) setActive(Math.max(0, options.findIndex((option) => option.value === value)));
  }, [open, options, value]);

  const choose = (option: SelectOption<T>) => {
    if (option.disabled) return;
    onChange(option.value);
    setOpen(false);
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (!open && (event.key === 'Enter' || event.key === ' ' || event.key === 'ArrowDown')) {
      event.preventDefault();
      onOpen?.();
      setOpen(true);
      return;
    }
    if (!open) return;

    if (event.key === 'Escape') {
      event.preventDefault();
      setOpen(false);
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      setActive((index) => Math.min(options.length - 1, index + 1));
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      setActive((index) => Math.max(0, index - 1));
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      const option = options[active];
      if (option) choose(option);
    }
  };

  const height = size === 'sm' ? 'h-[26px]' : 'h-8';

  return (
    <div ref={containerRef} className="relative">
      <button
        type="button"
        role="combobox"
        aria-expanded={open}
        aria-haspopup="listbox"
        aria-label={ariaLabel}
        onClick={() =>
          setOpen((current) => {
            if (!current) onOpen?.();
            return !current;
          })
        }
        onKeyDown={onKeyDown}
        className={cn(
          'flex w-full items-center justify-between gap-2 rounded-md border px-2.5',
          'border-border-strong bg-surface-sunken text-left text-primary',
          'transition-colors duration-100 hover:bg-surface-hover',
          height,
          size === 'sm' ? 'text-xs' : 'text-ui',
        )}
      >
        <span className="flex min-w-0 items-center gap-2">
          {selected?.icon && <selected.icon size={14} className="shrink-0 text-muted" aria-hidden />}
          <span className={cn('truncate', !selected && 'text-muted')}>
            {selected?.label ?? placeholder}
          </span>
        </span>
        <ChevronDown size={14} className="shrink-0 text-muted" aria-hidden />
      </button>

      {open && (
        <ul
          role="listbox"
          aria-label={ariaLabel}
          className={cn(
            'absolute z-50 mt-1 max-h-64 w-full overflow-auto rounded-lg border p-1',
            'border-border-default bg-surface-raised shadow-lg',
          )}
        >
          {options.map((option, index) => (
            <li key={String(option.value)}>
              <button
                type="button"
                role="option"
                aria-selected={option.value === value}
                disabled={option.disabled}
                onClick={() => choose(option)}
                onPointerEnter={() => setActive(index)}
                className={cn(
                  'flex w-full items-start gap-2 rounded-md px-2 py-1.5 text-left',
                  'disabled:cursor-not-allowed disabled:opacity-45',
                  index === active && 'bg-surface-hover',
                  option.value === value && 'bg-surface-selected',
                )}
              >
                {renderOption ? (
                  renderOption(option)
                ) : (
                  <>
                    {option.icon && (
                      <option.icon size={14} className="mt-0.5 shrink-0 text-muted" aria-hidden />
                    )}
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-ui text-primary">{option.label}</span>
                      {option.description && (
                        <span className="mt-0.5 block text-xs text-muted">
                          {option.description}
                        </span>
                      )}
                    </span>
                    {option.value === value && (
                      <Check size={14} className="mt-0.5 shrink-0 text-accent-text" aria-hidden />
                    )}
                  </>
                )}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
