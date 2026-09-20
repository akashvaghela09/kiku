import type { LucideIcon } from 'lucide-react';

import { cn } from '@/lib/cn';

/** Shown when a list has nothing in it, or nothing matching. */
export interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description?: React.ReactNode | undefined;
  action?: React.ReactNode | undefined;
  /** Tighter version, for "no results" inside an otherwise populated list. */
  compact?: boolean | undefined;
}

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  compact = false,
}: EmptyStateProps) {
  return (
    <div
      className={cn(
        'flex flex-col items-center justify-center text-center',
        compact ? 'gap-2 py-10' : 'gap-3 py-16',
      )}
    >
      <Icon
        size={compact ? 20 : 24}
        strokeWidth={1.5}
        className="text-disabled"
        aria-hidden
      />
      <div>
        <p className={cn('text-primary', compact ? 'text-ui' : 'text-lg font-semibold')}>
          {title}
        </p>
        {description && (
          <p className="mx-auto mt-1 max-w-[42ch] text-base text-muted">{description}</p>
        )}
      </div>
      {action}
    </div>
  );
}
