import { useEffect, useRef, useState } from 'react';
import { Check, Keyboard, X } from 'lucide-react';

import { Button, Kbd } from '@/components/ui';
import { commands, type Hotkey } from '@/lib/ipc';

/**
 * Capture the dictation key by pressing it.
 *
 * Pressing the key is how you find out whether your keyboard has it - which matters,
 * because the only keys Kiku can watch are the three right-hand modifiers, and plenty
 * of compact keyboards are missing one or another of them.
 */
interface HotkeyFieldProps {
  value: Hotkey;
  onChange: (spec: string) => Promise<string | null>;
  label: string;
  /**
   * True for the hands-free key, whose refusal has an extra reason worth giving: a key
   * Kiku cannot watch cannot be seen being tapped twice either.
   */
  needsDoubleTap?: boolean | undefined;
}

/**
 * The keys that can be bound, by the code the browser reports for them.
 *
 * Right-hand modifiers only. A bare modifier is the one thing comfortable to *hold*,
 * and the right-hand one is unclaimed by every platform - nothing uses Right Ctrl
 * alone, so watching it takes nothing away.
 */
const BINDABLE: Record<string, string> = {
  ControlRight: 'RightControl',
  AltRight: 'RightAlt',
  MetaRight: 'RightSuper',
};

/** Their left-hand twins, which are refused with an explanation rather than ignored. */
const LEFT_HAND = new Set(['ControlLeft', 'AltLeft', 'MetaLeft', 'ShiftLeft', 'ShiftRight']);

export function HotkeyField({ value, onChange, label, needsDoubleTap }: HotkeyFieldProps) {
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

      const spec = BINDABLE[event.code];
      if (spec) {
        void submit(spec);
        return;
      }

      // Anything else is refused where the user can see why, rather than being
      // swallowed - a field that ignores most of the keyboard looks broken.
      const reason = LEFT_HAND.has(event.code)
        ? 'Use the right-hand key. The left one stays yours to type with.'
        : 'Kiku listens on a right-hand modifier: Right Ctrl, Right Alt or Right Cmd.';

      setError(
        needsDoubleTap
          ? `${reason} Kiku cannot tell that any other key has been tapped twice.`
          : reason,
      );
      setCapturing(false);
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
            <span className="text-ui text-accent-text">Press a right-hand modifier…</span>
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
