import { useEffect } from 'react';
import type { LucideIcon } from 'lucide-react';

import { Button } from './Button';
import { cn } from '@/lib/cn';

/**
 * A transient message at the bottom of the main window.
 *
 * One at a time, replacing rather than stacking: a queue of toasts is a queue of
 * things the user has already stopped reading.
 */
export interface ToastProps {
  tone?: 'neutral' | 'success' | 'danger' | undefined;
  icon?: LucideIcon | undefined;
  message: string;
  action?: { label: string | undefined; onClick: () => void };
  duration?: number | undefined;
  onDismiss: () => void;
}

const TONES = {
  neutral: 'border-border-strong bg-surface-raised text-primary',
  success: 'border-success bg-success-wash text-success',
  danger: 'border-danger bg-danger-wash text-danger',
} as const;

export function Toast({
  tone = 'neutral',
  icon: Icon,
  message,
  action,
  duration = 3200,
  onDismiss,
}: ToastProps) {
  useEffect(() => {
    const timer = window.setTimeout(onDismiss, duration);
    return () => window.clearTimeout(timer);
  }, [duration, onDismiss, message]);

  return (
    <div
      role="status"
      aria-live="polite"
      className={cn(
        'pointer-events-auto flex items-center gap-2.5 rounded-lg border px-3.5 py-2.5 shadow-lg',
        'toast-enter',
        TONES[tone],
      )}
    >
      {Icon && <Icon size={16} className="shrink-0" aria-hidden />}
      <span className="text-ui">{message}</span>
      {action && (
        <Button size="sm" variant="ghost" onClick={action.onClick}>
          {action.label}
        </Button>
      )}
    </div>
  );
}
