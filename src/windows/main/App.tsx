import { useCallback, useEffect, useState } from 'react';

import { Toast } from '@/components/ui';
import { HistoryView } from '@/features/history/HistoryView';
import { OnboardingView } from '@/features/onboarding/OnboardingView';
import { SettingsView } from '@/features/settings/SettingsView';
import { applyTheme } from '@/features/settings/theme';
import { usePreferences } from '@/features/settings/usePreferences';
import { NavigationRail } from '@/features/shell/NavigationRail';
import type { View } from '@/features/shell/views';
import {
  commands,
  events,
  type EngineStatus,
  type HotkeyBindings,
  type UpdateStatus,
} from '@/lib/ipc';

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
  const [version, setVersion] = useState<string | null>(null);
  const [update, setUpdate] = useState<UpdateStatus | null>(null);
  const { preferences } = usePreferences();

  // Re-applied whenever the choice changes, and again on unmount, because "System"
  // subscribes to the desktop setting for as long as it is selected.
  useEffect(() => applyTheme(preferences.theme), [preferences.theme]);

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

    void commands.appInfo().then((result) => {
      if (result.status === 'ok') setVersion(result.data.version);
    });

    const unlisten = events.engineStatusChanged.listen((event) => setEngine(event.payload));
    return () => void unlisten.then((off) => off());
  }, [refreshBindings]);

  // Re-checked when the preference changes, so switching it on asks immediately
  // rather than at the next launch. The command already refuses to touch the network
  // when it is off, and caches the answer for a day when it is on.
  useEffect(() => {
    if (!preferences.checkForUpdates) {
      setUpdate(null);
      return;
    }
    void commands.checkForUpdate(false).then((result) => {
      setUpdate(result.status === 'ok' ? result.data : null);
    });
  }, [preferences.checkForUpdates]);

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
        version={version}
        update={update}
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
