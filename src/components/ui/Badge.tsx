import type { LucideIcon } from 'lucide-react';

import { cn } from '@/lib/cn';

/** A small status label: "Recommended", "PAUSED", "650 MB", "Granted". */
export interface BadgeProps {
  tone?: 'neutral' | 'accent' | 'success' | 'warning' | 'danger' | undefined;
  variant?: 'solid' | 'soft' | 'outline' | undefined;
  icon?: LucideIcon | undefined;
  children: React.ReactNode;
}

const SOFT = {
  neutral: 'bg-surface-active text-secondary',
  accent: 'bg-accent-wash text-accent-text',
  success: 'bg-success-wash text-success',
  warning: 'bg-warning-wash text-warning',
  danger: 'bg-danger-wash text-danger',
} as const;

const SOLID = {
  neutral: 'bg-secondary text-surface',
  accent: 'bg-accent text-on-accent',
  success: 'bg-success text-surface',
  warning: 'bg-warning text-surface',
  danger: 'bg-danger text-surface',
} as const;

const OUTLINE = {
  neutral: 'border border-border-strong text-secondary',
  accent: 'border border-border-accent text-accent-text',
  success: 'border border-success text-success',
  warning: 'border border-warning text-warning',
  danger: 'border border-danger text-danger',
} as const;

export function Badge({ tone = 'neutral', variant = 'soft', icon: Icon, children }: BadgeProps) {
  const palette = variant === 'solid' ? SOLID : variant === 'outline' ? OUTLINE : SOFT;

  return (
    <span
      className={cn(
        'inline-flex shrink-0 items-center gap-1 rounded-sm px-1.5 py-0.5',
        'text-2xs font-semibold uppercase tracking-[0.04em]',
        palette[tone],
      )}
    >
      {Icon && <Icon size={11} aria-hidden />}
      {children}
    </span>
  );
}
