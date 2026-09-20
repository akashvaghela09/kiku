import { cn } from '@/lib/cn';
import { osKeyName } from '@/lib/format';

/**
 * A keyboard shortcut.
 *
 * Owns the platform key-name mapping so nothing else in the interface has to know
 * that Alt is ⌥ on macOS.
 */
export interface KbdProps {
  keys: string[];
  size?: 'sm' | 'md' | undefined;
  tone?: 'neutral' | 'accent' | undefined;
}

export function Kbd({ keys, size = 'md', tone = 'neutral' }: KbdProps) {
  return (
    <span className="inline-flex items-center gap-1" role="img" aria-label={keys.join(' plus ')}>
      {keys.map((key) => (
        <kbd
          key={key}
          className={cn(
            'inline-flex items-center justify-center rounded-sm border font-mono',
            size === 'sm' ? 'h-[18px] min-w-[18px] px-1 text-2xs' : 'h-5 min-w-5 px-1.5 text-xs',
            tone === 'accent'
              ? 'border-border-accent bg-accent-wash text-accent-text'
              : 'border-border-strong bg-surface-sunken text-secondary',
          )}
        >
          {osKeyName(key)}
        </kbd>
      ))}
    </span>
  );
}
