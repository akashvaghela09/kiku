import { useCallback, useEffect, useState } from 'react';

import { Toast } from '@/components/ui';
import { HistoryView } from '@/features/history/HistoryView';
import { OnboardingView } from '@/features/onboarding/OnboardingView';
import { SettingsView } from '@/features/settings/SettingsView';
import { usePreferences } from '@/features/settings/usePreferences';
import { NavigationRail } from '@/features/shell/NavigationRail';
import type { View } from '@/features/shell/views';
import { commands, events, type EngineStatus, type HotkeyBindings } from '@/lib/ipc';

/**
 * The application shell.
 *
 * The shell owns the frame: navigation, the scroll region and the page background.
 * Each view owns only its contents. Previously every view decided its own width,
 * header and navigation, which is why History and Settings looked like two different
 * applications.
 */
export function App() {
  const [engine, setEngine] = useState<EngineStatus>({ state: 'unloaded' });
  const [bindings, setBindings] = useState<HotkeyBindings | null>(null);
  const [view, setView] = useState<View>('history');
  const [section, setSection] = useState<string | null>(null);
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

  const goToSection = useCallback((id: string) => {
    setView('settings');
    setSection(id);
    // The section may not exist yet if Settings is only now mounting.
    requestAnimationFrame(() => {
      document.getElementById(id)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    });
  }, []);

  if (!bindings) return <div className="h-full bg-surface" />;

  // Without a model nothing else is worth showing, and onboarding fills the window
  // rather than sitting beside a navigation rail that leads nowhere useful.
  if (engine.state === 'unloaded' && !dismissedOnboarding) {
    return (
      <div className="h-full bg-surface-sunken">
        <OnboardingView bindings={bindings} onDone={() => setDismissedOnboarding(true)} />
      </div>
    );
  }

  return (
    <div className="relative flex h-full bg-surface-sunken">
      <NavigationRail
        view={view}
        onNavigate={(next) => {
          setView(next);
          if (next === 'history') setSection(null);
        }}
        engine={engine}
        activeSection={section}
        onSection={goToSection}
      />

      <main className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
        {view === 'history' ? (
          <HistoryView
            hotkey={bindings.hold.spec.split('+')}
            paused={!preferences.recordHistory}
            onCopied={() => setToast('Copied to clipboard')}
          />
        ) : (
          <SettingsView
            bindings={bindings}
            onBindingsChanged={refreshBindings}
            onNotify={setToast}
            onSectionInView={setSection}
          />
        )}
      </main>

      {toast && (
        <div className="pointer-events-none absolute inset-x-0 bottom-6 flex justify-center">
          <Toast message={toast} onDismiss={() => setToast(null)} />
        </div>
      )}
    </div>
  );
}
