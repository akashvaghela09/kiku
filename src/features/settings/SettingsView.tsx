import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Download,
  Trash2,
} from 'lucide-react';

import { Badge, Button, Dialog, Kbd, Panel, Progress, Row, Select, Toggle } from '@/components/ui';
import { formatBytes, isMac } from '@/lib/format';
import {
  commands,
  events,
  type AppInfo,
  type HotkeyBindings,
  type MicrophoneInfo,
  type ModelInfo,
  type UpdateStatus,
} from '@/lib/ipc';
import { ViewToolbar } from '@/features/shell/ViewToolbar';
import { VIEW_LABELS } from '@/features/shell/views';
import { HotkeyField } from './HotkeyField';
import { SETTINGS_SECTIONS } from './sections';
import { usePreferences } from './usePreferences';

/**
 * Settings: a sticky section list beside one continuous scroll.
 *
 * Not tabs. Eight groups is too many to sit across the top at this width, and tabs
 * would hide whichever section contains the thing currently blocking the user - a
 * missing permission, a model that will not load - behind a click. The nav doubles as
 * a map that quietly says: this is all there is.
 */

interface SettingsViewProps {
  bindings: HotkeyBindings;
  onBindingsChanged: () => void;
  onNotify: (message: string) => void;
  /** Reports which section is in view, so the rail can show where you are. */
  onSectionInView: (id: string) => void;
}


export function SettingsView({
  bindings,
  onBindingsChanged,
  onNotify,
  onSectionInView,
}: SettingsViewProps) {
  const { preferences, update } = usePreferences();
  const [microphones, setMicrophones] = useState<MicrophoneInfo[]>([]);
  const [microphone, setMicrophone] = useState<string>('');
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>({ state: 'unknown' });
  const [confirmClear, setConfirmClear] = useState(false);
  const [activeSection, setActiveSection] = useState<string | null>(null);
  const [busyModel, setBusyModel] = useState<string | null>(null);
  const [downloading, setDownloading] = useState<{ done: number; total: number } | null>(null);

  const refreshMicrophones = useCallback(() => {
    void commands.listMicrophones().then((result) => {
      if (result.status === 'ok') setMicrophones(result.data);
    });
  }, []);

  useEffect(() => {
    void commands.listMicrophones().then((result) => {
      if (result.status === 'ok') {
        setMicrophones(result.data);
        setMicrophone(result.data.find((device) => device.isDefault)?.id ?? '');
      }
    });
    void commands.listModels().then((result) => {
      if (result.status === 'ok') setModels(result.data);
    });
    void commands.appInfo().then((result) => {
      if (result.status === 'ok') setInfo(result.data);
    });
    void commands.checkForUpdate(false).then((result) => {
      if (result.status === 'ok') setUpdateStatus(result.data);
    });
  }, []);

  // Model actions all change what `listModels` would return, so each one refreshes
  // the list rather than trying to patch it locally.
  const act = async (id: string, run: () => Promise<{ status: string }>) => {
    setBusyModel(id);
    const result = await run();
    if (result.status === 'error' && 'error' in result) {
      onNotify((result as { error: { message: string } }).error.message);
    }
    const listed = await commands.listModels();
    if (listed.status === 'ok') setModels(listed.data);
    setBusyModel(null);
    setDownloading(null);
  };

  // Scroll-spy: an eight-item list you cannot locate yourself in is half a
  // navigation. The topmost section intersecting the viewport wins.
  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .filter((entry) => entry.isIntersecting)
          .sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top)[0];
        if (visible?.target.id) {
          setActiveSection(visible.target.id);
          onSectionInView(visible.target.id);
        }
      },
      { rootMargin: '0px 0px -70% 0px', threshold: 0 },
    );

    for (const section of SETTINGS_SECTIONS) {
      const element = document.getElementById(section.id);
      if (element) observer.observe(element);
    }
    return () => observer.disconnect();
  }, [onSectionInView]);

  useEffect(() => {
    const unlisten = events.downloadProgressed.listen((event) =>
      setDownloading({
        done: event.payload.downloadedBytes,
        total: event.payload.totalBytes,
      }),
    );
    return () => void unlisten.then((off) => off());
  }, []);

  const microphoneOptions = useMemo(
    () =>
      microphones.map((device) => ({
        value: device.id,
        label: device.name,
        description: device.isDefault ? 'System default' : undefined,
      })),
    [microphones],
  );

  const rebind = async (which: 'hold' | 'toggle', spec: string): Promise<string | null> => {
    const next = { ...bindings, [which]: { spec, display: spec } };
    const result = await commands.setHotkeys(next);
    if (result.status === 'error') return result.error.message;
    onBindingsChanged();
    onNotify('Shortcut updated');
    return null;
  };

  const applyPreset = async (hold: string, toggle: string) => {
    const result = await commands.setHotkeys({
      hold: { spec: hold, display: hold },
      toggle: { spec: toggle, display: toggle },
    });
    if (result.status === 'error') {
      onNotify(result.error.message);
      return;
    }
    onBindingsChanged();
    onNotify('Shortcuts updated');
  };

  return (
    <>
      <ViewToolbar
        title={VIEW_LABELS.settings}
        meta={SETTINGS_SECTIONS.find((section) => section.id === activeSection)?.label}
      />

      <div className="min-h-0 flex-1 overflow-auto">
        <div className="app-column space-y-6 py-6">

        <section id="shortcuts" className="scroll-mt-6">
          <Panel
            title="Shortcuts"
            description="Hold to talk, or press once to keep listening hands-free."
          >
            <Row
              title="Hold to talk"
              description="Recording stops the moment you let go."
              trailing={
                <HotkeyField
                  value={bindings.hold}
                  label="hold-to-talk shortcut"
                  onChange={(spec) => rebind('hold', spec)}
                />
              }
            />
            <Row
              title="Toggle"
              description="Press once to start, again to stop."
              trailing={
                <HotkeyField
                  value={bindings.toggle}
                  label="toggle shortcut"
                  onChange={(spec) => rebind('toggle', spec)}
                />
              }
            />
            <Row
              align="start"
              title="Presets"
              description="One key is easier to hold than a chord. Right Ctrl is watched rather than registered, so it still works as Ctrl everywhere else - pressing any other key while holding it cancels. Use a chord instead if your keyboard has no right Ctrl."
              trailing={
                <div className="flex flex-wrap justify-end gap-1.5">
                  <Button
                    size="sm"
                    onClick={() => void applyPreset(singleKeyDefault(), 'Ctrl+Alt+Space')}
                  >
                    {singleKeyLabel()}
                  </Button>
                  <Button size="sm" onClick={() => void applyPreset('F9', 'F10')}>
                    F9 / F10
                  </Button>
                  <Button
                    size="sm"
                    onClick={() => void applyPreset('Ctrl+Shift+Space', 'Ctrl+Alt+Space')}
                  >
                    Chord
                  </Button>
                </div>
              }
            />
          </Panel>
        </section>

        <section id="microphone" className="scroll-mt-6">
          <Panel title="Microphone">
            <Row
              align="start"
              title="Input device"
              description="Kiku follows your system default unless you pick one. A Bluetooth headset only offers a microphone in its Handsfree profile - if yours is missing, switch it in your system sound settings and reopen this list."
              trailing={
                <div className="w-64">
                  <Select
                    value={microphone}
                    options={microphoneOptions}
                    aria-label="Microphone"
                    onOpen={refreshMicrophones}
                    onChange={(id) => {
                      setMicrophone(id);
                      void commands.setMicrophone(id || null);
                    }}
                  />
                </div>
              }
            />
          </Panel>
        </section>

        <section id="model" className="scroll-mt-6">
          <Panel
            title="Speech model"
            description="Runs entirely on this computer. Downloaded once, then kept."
          >
            {models.map((model) => (
              <Row
                key={model.id}
                align="start"
                title={
                  <span className="flex items-center gap-2">
                    {model.name}
                    {model.active && <Badge tone="success">In use</Badge>}
                    {!model.active && model.install.state === 'installed' && (
                      <Badge>Downloaded</Badge>
                    )}
                  </span>
                }
                description={
                  <span>
                    {model.summary}
                    <span className="ml-1.5 font-mono">{formatBytes(model.totalBytes)}</span>
                  </span>
                }
                trailing={
                  <ModelActions
                    model={model}
                    busy={busyModel === model.id}
                    onDownload={() => void act(model.id, () => commands.downloadModel(model.id))}
                    onUse={() => void act(model.id, () => commands.useModel(model.id))}
                    onDelete={() => void act(model.id, () => commands.deleteModel(model.id))}
                  />
                }
              />
            ))}
            {busyModel && !downloading && (
              <p className="px-3 pb-2 text-xs text-muted">
                Loading the model into memory. This takes a few seconds and only
                happens when you switch.
              </p>
            )}
            {downloading && (
              <div className="px-3 pb-2 pt-1">
                <Progress
                  value={downloading.total > 0 ? downloading.done / downloading.total : undefined}
                  label="Downloading"
                  detail={`${formatBytes(downloading.done)} of ${formatBytes(downloading.total)}`}
                />
              </div>
            )}
          </Panel>
        </section>

        <section id="output" className="scroll-mt-6">
          <Panel
            title="Output"
            description="Your transcript always goes to the clipboard. Pasting is on top of that."
          >
            <Row
              title="Paste automatically"
              description="Insert the text into whatever app you're typing in."
              trailing={
                <Toggle
                  checked={preferences.autoPaste}
                  onChange={(autoPaste) => update({ autoPaste })}
                  aria-label="Paste automatically"
                />
              }
            />
            <Row
              title="Add a space at the end"
              description="So two dictations in a row don't run together."
              trailing={
                <Toggle
                  checked={preferences.trailingSpace}
                  onChange={(trailingSpace) => update({ trailingSpace })}
                  aria-label="Add a trailing space"
                />
              }
            />
          </Panel>
        </section>

        <section id="sounds" className="scroll-mt-6">
          <Panel title="Sounds">
            <Row
              title="Sound feedback"
              description="A short cue when Kiku starts listening, and another when it stops."
              trailing={
                <div className="flex items-center gap-1.5">
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => void commands.previewSound('start')}
                  >
                    Start
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => void commands.previewSound('stop')}
                  >
                    Stop
                  </Button>
                  <Toggle
                    checked={preferences.sounds}
                    onChange={(sounds) => update({ sounds })}
                    aria-label="Sound feedback"
                  />
                </div>
              }
            />
          </Panel>
        </section>

        <section id="history" className="scroll-mt-6">
          <Panel
            title="History"
            description="Transcripts are stored as text on this computer. No audio is ever kept."
          >
            <Row
              title="Record history"
              description="Keep a copy of each dictation. Turning this off leaves what is already saved alone."
              trailing={
                <Toggle
                  checked={preferences.recordHistory}
                  onChange={(recordHistory) => update({ recordHistory })}
                  aria-label="Record history"
                />
              }
            />
            <Row
              title="Delete older transcripts"
              trailing={
                <div className="w-44">
                  <Select
                    value={preferences.retentionDays ?? 0}
                    aria-label="Retention"
                    options={[
                      { value: 0, label: 'Keep everything' },
                      { value: 7, label: 'After 7 days' },
                      { value: 30, label: 'After 30 days' },
                      { value: 90, label: 'After 90 days' },
                    ]}
                    onChange={(days) => update({ retentionDays: days === 0 ? null : days })}
                  />
                </div>
              }
            />
            <Row
              title="Delete everything"
              description="Removes every transcript from this computer."
              trailing={
                <Button size="sm" variant="danger" icon={Trash2} onClick={() => setConfirmClear(true)}>
                  Delete all
                </Button>
              }
            />
          </Panel>
        </section>

        <section id="updates" className="scroll-mt-6">
          <Panel
            title="Updates"
            description="Kiku never updates itself. It can check once a day whether a newer version exists and show a link."
          >
            <Row
              title="Check for updates"
              description={describeUpdate(updateStatus)}
              trailing={
                <Toggle
                  checked={preferences.checkForUpdates}
                  onChange={(checkForUpdates) => update({ checkForUpdates })}
                  aria-label="Check for updates"
                />
              }
            />
            {updateStatus.state === 'available' && (
              <Row
                title={`Version ${updateStatus.detail.version} is available`}
                trailing={
                  <Button
                    size="sm"
                    icon={Download}
                    onClick={() => void commands.openUrl(updateStatus.detail.url)}
                  >
                    Open release
                  </Button>
                }
              />
            )}
          </Panel>
        </section>

        <section id="about" className="scroll-mt-6">
          <Panel title="About">
            <Row
              title="Kiku"
              description={info ? `Version ${info.version} · ${info.platform}` : '…'}
              trailing={<Kbd keys={bindings.hold.spec.split('+')} size="sm" />}
            />
            <Row
              align="start"
              title="Speech recognition"
              description="NVIDIA Parakeet, used under CC-BY-4.0. Nothing you dictate leaves this computer."
            />
          </Panel>
        </section>
        </div>
      </div>

      <Dialog
        open={confirmClear}
        onOpenChange={setConfirmClear}
        tone="danger"
        title="Delete all transcripts?"
        description="This removes every saved transcript from this computer. It cannot be undone."
        confirmLabel="Delete everything"
        onConfirm={() => {
          void commands.clearHistory().then(() => onNotify('History deleted'));
        }}
      />
    </>
  );
}

/**
 * Download, switch and delete for one model.
 *
 * Deleting is the point of showing the size: a model that is not in use is several
 * hundred megabytes sitting on disk for nothing.
 */
function ModelActions({
  model,
  busy,
  onDownload,
  onUse,
  onDelete,
}: {
  model: ModelInfo;
  busy: boolean;
  onDownload: () => void;
  onUse: () => void;
  onDelete: () => void;
}) {
  const installed = model.install.state === 'installed';

  if (!installed) {
    return (
      <Button size="sm" icon={Download} loading={busy} onClick={onDownload}>
        {model.install.state === 'partial' ? 'Resume' : 'Download'}
      </Button>
    );
  }

  return (
    <>
      {!model.active && (
        <Button size="sm" loading={busy} onClick={onUse}>
          {busy ? 'Loading' : 'Use this'}
        </Button>
      )}
      <Button
        size="sm"
        variant="ghost"
        icon={Trash2}
        iconOnly
        destructive
        label={`Delete ${model.name}`}
        disabled={busy}
        onClick={onDelete}
      />
    </>
  );
}

/**
 * The single key to offer as the hold shortcut.
 *
 * Mac keyboards have no right Control key, so macOS gets Right Option instead.
 */
function singleKeyDefault(): string {
  return isMac() ? 'RightAlt' : 'RightControl';
}

function singleKeyLabel(): string {
  return isMac() ? 'Right \u2325' : 'Right Ctrl';
}

function describeUpdate(status: UpdateStatus): string {
  switch (status.state) {
    case 'upToDate':
      return "You're on the latest version.";
    case 'available':
      return 'A newer version is available on GitHub.';
    case 'unknown':
      return 'Not checked yet.';
  }
}
