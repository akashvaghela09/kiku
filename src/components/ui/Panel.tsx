import type { LucideIcon } from 'lucide-react';

import { cn } from '@/lib/cn';

/**
 * A titled group of rows.
 *
 * One component covers every settings group, every onboarding card and the macOS
 * permission callout - the tone prop is what turns it into a callout, rather than a
 * second component that would drift from this one.
 */
export interface PanelProps {
  title?: string | undefined;
  /** Small uppercase label above the title. */
  eyebrow?: string | undefined;
  description?: string | undefined;
  icon?: LucideIcon | undefined;
  footer?: React.ReactNode | undefined;
  tone?: 'default' | 'accent' | 'warning' | 'danger' | undefined;
  className?: string | undefined;
  children: React.ReactNode;
}

const TONES = {
  default: 'border-border-default bg-surface-raised',
  accent: 'border-border-accent bg-accent-wash',
  warning: 'border-warning bg-warning-wash',
  danger: 'border-danger bg-danger-wash',
} as const;

const ICON_TONES = {
  default: 'text-muted',
  accent: 'text-accent-text',
  warning: 'text-warning',
  danger: 'text-danger',
} as const;

export function Panel({
  title,
  eyebrow,
  description,
  icon: Icon,
  footer,
  tone = 'default',
  className,
  children,
}: PanelProps) {
  const hasHeader = Boolean(title ?? eyebrow ?? description);

  return (
    <section className={cn('rounded-lg border', TONES[tone], className)}>
      {hasHeader && (
        <header className="flex gap-3 px-4 pb-2 pt-3.5">
          {Icon && <Icon size={16} className={cn('mt-0.5 shrink-0', ICON_TONES[tone])} aria-hidden />}
          <div className="min-w-0">
            {eyebrow && (
              <p className="text-2xs font-semibold uppercase tracking-[0.04em] text-muted">
                {eyebrow}
              </p>
            )}
            {title && <h2 className="text-lg font-semibold text-primary">{title}</h2>}
            {description && <p className="mt-1 text-base text-secondary">{description}</p>}
          </div>
        </header>
      )}

      <div className={cn('px-1 pb-1', hasHeader ? 'pt-0' : 'pt-1')}>{children}</div>

      {footer && (
        <footer className="border-t border-border-subtle px-4 py-2.5">{footer}</footer>
      )}
    </section>
  );
}
