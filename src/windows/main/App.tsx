import { useEffect, useState } from 'react';
import { AlertCircle, Mic } from 'lucide-react';

import { commands, type AppInfo } from '@/lib/ipc';

/**
 * Scaffold shell. Proves the full stack end to end — React renders, Tailwind resolves
 * the design tokens, and a typed command round-trips to Rust and back. The real
 * History interface replaces this in chunk 9.
 */
export function App() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    commands
      .appInfo()
      .then((result) => {
        if (result.status === 'ok') setInfo(result.data);
        else setError(result.error.message);
      })
      .catch((cause: unknown) => setError(String(cause)));
  }, []);

  return (
    <main className="flex h-full flex-col items-center justify-center gap-4 bg-surface p-6">
      <div className="flex size-12 items-center justify-center rounded-xl bg-accent-wash">
        <Mic className="text-accent-graphic" size={24} strokeWidth={1.75} aria-hidden />
      </div>

      <div className="text-center">
        <h1 className="text-xl font-semibold text-primary">Kiku</h1>
        <p className="mt-1 text-base text-secondary">
          Hold a key, speak, and the text appears where you&rsquo;re typing.
        </p>
      </div>

      {error ? (
        <p className="flex items-center gap-2 rounded-md bg-danger-wash px-3 py-2 text-ui text-danger">
          <AlertCircle size={16} aria-hidden />
          {error}
        </p>
      ) : (
        <dl className="grid grid-cols-[auto_auto] gap-x-4 gap-y-1 rounded-lg border border-border-default bg-surface-sunken px-4 py-3 text-sm">
          <dt className="text-muted">Version</dt>
          <dd className="font-mono text-secondary">{info?.version ?? '…'}</dd>
          <dt className="text-muted">Platform</dt>
          <dd className="font-mono text-secondary">{info?.platform ?? '…'}</dd>
        </dl>
      )}
    </main>
  );
}
