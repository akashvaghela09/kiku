import { useCallback, useEffect, useState } from 'react';
import { History, Settings as SettingsIcon } from 'lucide-react';

import { Button, Toast } from '@/components/ui';
import { HistoryView } from '@/features/history/HistoryView';
import { OnboardingView } from '@/features/onboarding/OnboardingView';
import { SettingsView } from '@/features/settings/SettingsView';
import { usePreferences } from '@/features/settings/usePreferences';
import { commands, events, type EngineStatus, type HotkeyBindings } from '@/lib/ipc';

type View = 'history' | 'settings';

/**
 * The main window.
 *
 * Two views and a first-run path, chosen by what the application actually knows: if
 * there is no model, nothing else is worth showing yet.
 */
export function App() {
  const [engine, setEngine] = useState<EngineStatus>({ state: 'unloaded' });
  const [bindings, setBindings] = useState<HotkeyBindings | null>(null);
  const [view, setView] = useState<View>('history');
  const [dismissedOnboarding, setDismissedOnboarding] = useState(false);
  const [toast, setToast] = useState<string | null>(null);
  const { preferences } = usePreferences();

  const refreshBindings = useCallback(() => {
    void commands.hotkeyBindings().then((result) => {
      if (result.status === 'ok') setBindings(result.data);
    });
  }, []);

  useEffect(() => {
    refreshBindings();
    void commands.engineStatus().then((result) => {
      if (result.status === 'ok') setEngine(result.data);
    });

    const unlisten = events.engineStatusChanged.listen((event) => setEngine(event.payload));
    return () => void unlisten.then((off) => off());
  }, [refreshBindings]);

  if (!bindings) return <div className="h-full bg-surface" />;

  // No model means onboarding, regardless of which view was last open.
  if (engine.state === 'unloaded' && !dismissedOnboarding) {
    return (
      <div className="h-full bg-surface">
        <OnboardingView bindings={bindings} onDone={() => setDismissedOnboarding(true)} />
      </div>
    );
  }

  return (
    <div className="relative flex h-full flex-col bg-surface">
      {/* `min-h-0` is what lets the view scroll instead of pushing the nav off the
          bottom of the window — a flex child's default min-height is its content. */}
      <div className="min-h-0 flex-1">
        {view === 'history' ? (
          <HistoryView
            hotkey={bindings.hold.spec.split('+')}
            paused={preferences.historyPaused}
            onCopied={() => setToast('Copied to clipboard')}
          />
        ) : (
          <SettingsView
            bindings={bindings}
            onBindingsChanged={refreshBindings}
            onNotify={setToast}
          />
        )}
      </div>

      <nav className="flex h-11 shrink-0 items-center gap-1 border-t border-border-subtle px-3">
        <Button
          variant={view === 'history' ? 'secondary' : 'ghost'}
          size="sm"
          icon={History}
          onClick={() => setView('history')}
        >
          History
        </Button>
        <Button
          variant={view === 'settings' ? 'secondary' : 'ghost'}
          size="sm"
          icon={SettingsIcon}
          onClick={() => setView('settings')}
        >
          Settings
        </Button>

        {engine.state === 'loading' && (
          <span className="ml-auto text-xs text-muted">Getting the model ready…</span>
        )}
        {engine.state === 'failed' && (
          <span className="ml-auto text-xs text-danger">{engine.detail}</span>
        )}
      </nav>

      {toast && (
        <div className="pointer-events-none absolute inset-x-0 bottom-14 flex justify-center">
          <Toast message={toast} onDismiss={() => setToast(null)} />
        </div>
      )}
    </div>
  );
}
