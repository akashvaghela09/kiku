import { useEffect, useRef, useState } from 'react';
import { Check, Keyboard, X } from 'lucide-react';

import { Button, Kbd } from '@/components/ui';
import { commands, type Hotkey } from '@/lib/ipc';

/**
 * Capture a new shortcut by pressing it.
 *
 * Asking someone to type "Ctrl+Shift+Space" into a text box is a worse experience than
 * letting them press the keys, and it is the only way they discover that a chord of
 * two ordinary keys is not a shortcut the system can register.
 */
interface HotkeyFieldProps {
  value: Hotkey;
  onChange: (spec: string) => Promise<string | null>;
  label: string;
}

/** Keys that only modify; a shortcut needs something other than these. */
const MODIFIER_CODES = new Set([
  'ControlLeft',
  'ControlRight',
  'ShiftLeft',
  'ShiftRight',
  'AltLeft',
  'AltRight',
  'MetaLeft',
  'MetaRight',
]);

export function HotkeyField({ value, onChange, label }: HotkeyFieldProps) {
  const [capturing, setCapturing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!capturing) return;

    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();

      if (event.key === 'Escape') {
        setCapturing(false);
        return;
      }

      // Wait for a non-modifier: the user is still assembling the chord.
      if (MODIFIER_CODES.has(event.code)) return;

      const parts: string[] = [];
      if (event.ctrlKey) parts.push('Ctrl');
      if (event.altKey) parts.push('Alt');
      if (event.shiftKey) parts.push('Shift');
      if (event.metaKey) parts.push('Super');
      parts.push(event.code);

      void submit(parts.join('+'));
    };

    window.addEventListener('keydown', onKeyDown, true);
    return () => window.removeEventListener('keydown', onKeyDown, true);
  });

  const submit = async (spec: string) => {
    const validation = await commands.validateHotkey(spec);
    if (validation.status === 'error') {
      setError(validation.error.message);
      setCapturing(false);
      return;
    }

    const failure = await onChange(spec);
    setError(failure);
    setCapturing(false);
  };

  return (
    <div className="flex flex-col items-end gap-1">
      <div className="flex items-center gap-2">
        {capturing ? (
          <>
            <span className="text-ui text-accent-text">Press a shortcut…</span>
            <Button
              size="sm"
              variant="ghost"
              icon={X}
              iconOnly
              label="Cancel"
              onClick={() => setCapturing(false)}
            />
          </>
        ) : (
          <>
            <Kbd keys={value.spec.split('+')} />
            <Button
              ref={buttonRef}
              size="sm"
              icon={Keyboard}
              onClick={() => {
                setError(null);
                setCapturing(true);
              }}
              aria-label={`Change ${label}`}
            >
              Change
            </Button>
          </>
        )}
      </div>

      {error && (
        <p className="max-w-[46ch] text-right text-xs text-danger">{error}</p>
      )}
      {!error && !capturing && value.spec === '' && (
        <p className="text-xs text-muted">
          <Check size={11} className="inline" aria-hidden /> Saved
        </p>
      )}
    </div>
  );
}
