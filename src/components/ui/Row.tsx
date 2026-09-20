import { cn } from '@/lib/cn';

/**
 * The most reused primitive in the application.
 *
 * A settings row and a transcript row are the same shape - something leading, a title
 * with optional description, something trailing - so they are the same component.
 * Settings contributes no components of its own precisely because of this.
 */
export interface RowProps {
  as?: 'div' | 'li' | undefined;
  leading?: React.ReactNode | undefined;
  title: React.ReactNode;
  description?: React.ReactNode | undefined;
  trailing?: React.ReactNode | undefined;
  /** `start` for multi-line content such as a transcript. */
  align?: 'center' | 'start' | undefined;
  density?: 'comfortable' | 'compact' | undefined;
  /**
   * Hover and focus feedback without button semantics: no click target, no role, no
   * tab stop. For a row whose parts are individually actionable - a transcript with
   * its own Read more, Copy and Delete - where a pointer cursor would be a lie.
   */
  hoverable?: boolean | undefined;
  /** A row that is itself a button. Implies `hoverable`. */
  interactive?: boolean | undefined;
  /**
   * Square the corners and widen the inset, for a row sitting flush inside a Panel.
   * A prop rather than a className override because `cn` concatenates without
   * resolving conflicts, so which padding wins would depend on class order.
   */
  flush?: boolean | undefined;
  selected?: boolean | undefined;
  onActivate?: () => void | undefined;
  className?: string | undefined;
}

export function Row({
  as: Element = 'div',
  leading,
  title,
  description,
  trailing,
  align = 'center',
  density = 'comfortable',
  hoverable = false,
  interactive = false,
  flush = false,
  selected = false,
  onActivate,
  className,
}: RowProps) {
  // Keyboard parity: anything clickable must also be reachable by Tab and activate on
  // Enter or Space, which a bare div does not do for free.
  const activatable = interactive && onActivate;

  const handleKeyDown = (event: React.KeyboardEvent) => {
    if (!activatable) return;
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      onActivate();
    }
  };

  return (
    <Element
      className={cn(
        'group/row flex w-full gap-3',
        flush ? 'rounded-none px-4' : 'rounded-md px-3',
        density === 'compact' ? 'min-h-9 py-1.5' : 'min-h-12 py-2',
        align === 'start' ? 'items-start' : 'items-center',
        // focus-within as well as hover: tabbing to a row's Copy button must light
        // the same row the mouse would.
        (interactive || hoverable) &&
          'transition-colors duration-100 hover:bg-surface-hover focus-within:bg-surface-hover',
        interactive && 'cursor-pointer',
        selected && 'bg-surface-selected',
        className,
      )}
      onClick={activatable ? onActivate : undefined}
      onKeyDown={activatable ? handleKeyDown : undefined}
      tabIndex={activatable ? 0 : undefined}
      role={activatable ? 'button' : undefined}
      aria-pressed={activatable ? selected : undefined}
    >
      {leading && (
        <span className={cn('flex shrink-0', align === 'start' ? 'pt-0.5' : 'items-center')}>
          {leading}
        </span>
      )}

      <span className="min-w-0 flex-1">
        <span className="block text-ui text-primary">{title}</span>
        {description && (
          <span className="mt-0.5 block text-xs text-muted">{description}</span>
        )}
      </span>

      {trailing && (
        <span
          className={cn(
            'flex shrink-0 gap-1.5',
            align === 'start' ? 'pt-0.5' : 'items-center',
          )}
        >
          {trailing}
        </span>
      )}
    </Element>
  );
}
