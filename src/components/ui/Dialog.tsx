import { useEffect, useRef } from 'react';

import { Button } from './Button';
import { cn } from '@/lib/cn';

/**
 * The only modal in the application, used solely to confirm something destructive.
 *
 * Built on `<dialog>` so focus trapping, Escape and the backdrop come from the
 * platform rather than from a focus-management library.
 */
export interface DialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string | undefined;
  tone?: 'default' | 'danger' | undefined;
  confirmLabel?: string | undefined;
  cancelLabel?: string | undefined;
  onConfirm: () => void;
  children?: React.ReactNode | undefined;
}

export function Dialog({
  open,
  onOpenChange,
  title,
  description,
  tone = 'default',
  confirmLabel = 'Confirm',
  cancelLabel = 'Cancel',
  onConfirm,
  children,
}: DialogProps) {
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = ref.current;
    if (!dialog) return;

    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  }, [open]);

  return (
    <dialog
      ref={ref}
      onClose={() => onOpenChange(false)}
      className={cn(
        'm-auto w-[min(420px,calc(100vw-48px))] rounded-xl border border-border-default',
        'bg-surface-raised p-0 text-primary shadow-lg',
        'backdrop:bg-[var(--surface-scrim)]',
      )}
      aria-labelledby="dialog-title"
    >
      <div className="px-5 pb-4 pt-5">
        <h2 id="dialog-title" className="text-lg font-semibold text-primary">
          {title}
        </h2>
        {description && <p className="mt-1.5 text-base text-secondary">{description}</p>}
        {children && <div className="mt-3">{children}</div>}
      </div>

      <div className="flex justify-end gap-2 border-t border-border-subtle px-5 py-3">
        <Button variant="ghost" onClick={() => onOpenChange(false)}>
          {cancelLabel}
        </Button>
        <Button
          variant={tone === 'danger' ? 'danger' : 'primary'}
          onClick={() => {
            onConfirm();
            onOpenChange(false);
          }}
        >
          {confirmLabel}
        </Button>
      </div>
    </dialog>
  );
}
