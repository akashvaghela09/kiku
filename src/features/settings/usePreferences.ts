import { useCallback, useEffect, useState } from 'react';

import { commands, type Preferences } from '@/lib/ipc';

const FALLBACK: Preferences = {
  autoPaste: true,
  trailingSpace: true,
  sounds: true,
  historyPaused: false,
  retentionDays: null,
  checkForUpdates: true,
};

/**
 * User preferences, written through to Rust as they change.
 *
 * Updated optimistically: every control here is instant and reversible, so waiting
 * for a round trip would only make a toggle feel sticky.
 */
export function usePreferences() {
  const [preferences, setPreferences] = useState<Preferences>(FALLBACK);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    void commands.preferences().then((result) => {
      if (result.status === 'ok') setPreferences(result.data);
      setLoaded(true);
    });
  }, []);

  const update = useCallback((patch: Partial<Preferences>) => {
    setPreferences((current) => {
      const next = { ...current, ...patch };
      void commands.setPreferences(next);
      return next;
    });
  }, []);

  return { preferences, update, loaded };
}
