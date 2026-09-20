import { useCallback, useEffect, useState } from 'react';

import { commands, type Preferences } from '@/lib/ipc';

const FALLBACK: Preferences = {
  theme: 'system',
  autoPaste: true,
  trailingSpace: true,
  sounds: true,
  recordHistory: true,
  retentionDays: null,
  checkForUpdates: true,
  modelId: null,
};

/**
 * One copy of the preferences, shared by every caller.
 *
 * Deliberately module state rather than per-hook state. Two components use this - the
 * shell and the settings view - and with a `useState` each they held separate copies:
 * changing a setting updated the view it was changed in and left the other showing
 * what it read at mount. The theme never repainted and the history badge never
 * noticed recording had been turned off, both silently, because nothing was wrong
 * with either component on its own.
 *
 * A context would also work, but the value is genuinely global, there is exactly one
 * writer, and this keeps the providers out of the tree.
 */
let current: Preferences = FALLBACK;
let loadedOnce = false;
const listeners = new Set<(preferences: Preferences) => void>();

function publish(next: Preferences) {
  current = next;
  listeners.forEach((listener) => listener(next));
}

/** Fetched once for the lifetime of the window, however many components ask. */
let inFlight: Promise<void> | null = null;
function loadOnce() {
  inFlight ??= commands.preferences().then((result) => {
    if (result.status === 'ok') publish(result.data);
    loadedOnce = true;
    listeners.forEach((listener) => listener(current));
  });
  return inFlight;
}

/**
 * User preferences, written through to Rust as they change.
 *
 * Updated optimistically: every control here is instant and reversible, so waiting
 * for a round trip would only make a toggle feel sticky.
 */
export function usePreferences() {
  const [preferences, setPreferences] = useState<Preferences>(current);
  const [loaded, setLoaded] = useState(loadedOnce);

  useEffect(() => {
    const listener = (next: Preferences) => {
      setPreferences(next);
      setLoaded(loadedOnce);
    };
    listeners.add(listener);
    void loadOnce();
    return () => {
      listeners.delete(listener);
    };
  }, []);

  const update = useCallback((patch: Partial<Preferences>) => {
    const next = { ...current, ...patch };
    publish(next);
    void commands.setPreferences(next);
  }, []);

  return { preferences, update, loaded };
}
