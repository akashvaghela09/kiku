import { useEffect, useRef, useState } from 'react';
import { Check, Download, Mic, ShieldCheck } from 'lucide-react';

import { Button, Kbd, Panel, Progress } from '@/components/ui';
import { formatBytes } from '@/lib/format';
import { commands, events, type HotkeyBindings, type ModelInfo } from '@/lib/ipc';

/**
 * First run.
 *
 * Three steps, no more: get the model, grant the microphone, try it once. The
 * temptation on a first run is to explain the product; the faster route to someone
 * understanding it is to have them dictate a sentence.
 */

type Step = 'welcome' | 'downloading' | 'ready';

interface OnboardingProps {
  bindings: HotkeyBindings;
  onDone: () => void;
}

export function OnboardingView({ bindings, onDone }: OnboardingProps) {
  const [step, setStep] = useState<Step>('welcome');
  const [model, setModel] = useState<ModelInfo | null>(null);
  const [progress, setProgress] = useState({ downloaded: 0, total: 0 });
  const [error, setError] = useState<string | null>(null);
  const started = useRef(false);

  useEffect(() => {
    void commands.listModels().then((result) => {
      if (result.status === 'ok') setModel(result.data[0] ?? null);
    });
  }, []);

  useEffect(() => {
    const unlisten = events.downloadProgressed.listen((event) => {
      setProgress({
        downloaded: event.payload.downloadedBytes,
        total: event.payload.totalBytes,
      });
    });
    return () => void unlisten.then((off) => off());
  }, []);

  const download = async () => {
    if (!model || started.current) return;
    started.current = true;
    setStep('downloading');
    setError(null);

    const result = await commands.downloadModel(model.id);
    if (result.status === 'error') {
      setError(result.error.message);
      started.current = false;
      setStep('welcome');
      return;
    }
    setStep('ready');
  };

  return (
    <div className="flex h-full items-center justify-center p-8">
      <div className="w-full max-w-[480px] space-y-4">
        <header className="text-center">
          <div className="mx-auto mb-3 flex size-12 items-center justify-center rounded-xl bg-accent-wash">
            <Mic size={24} strokeWidth={1.75} className="text-accent-graphic" aria-hidden />
          </div>
          <h1 className="text-2xl font-semibold text-primary">Welcome to Kiku</h1>
          <p className="mx-auto mt-1.5 max-w-[40ch] text-base text-secondary">
            Hold a key, speak, and the text appears where you&rsquo;re typing.
          </p>
        </header>

        {step === 'welcome' && (
          <Panel
            icon={Download}
            title="Download the speech model"
            description="This is a one-time download. Afterwards Kiku works entirely offline — nothing you say ever leaves this computer."
            footer={
              <div className="flex items-center justify-between gap-3">
                <span className="font-mono text-xs text-muted">
                  {model ? formatBytes(model.totalBytes) : '…'}
                </span>
                <Button variant="primary" size="lg" icon={Download} onClick={() => void download()}>
                  Download
                </Button>
              </div>
            }
          >
            {error && <p className="px-3 py-2 text-ui text-danger">{error}</p>}
          </Panel>
        )}

        {step === 'downloading' && (
          <Panel icon={Download} title="Downloading">
            <div className="px-3 py-3">
              <Progress
                value={progress.total > 0 ? progress.downloaded / progress.total : undefined}
                label="Speech model"
                detail={
                  progress.total > 0
                    ? `${formatBytes(progress.downloaded)} of ${formatBytes(progress.total)}`
                    : 'Starting…'
                }
              />
              <p className="mt-3 text-base text-muted">
                You can leave this running. If the download is interrupted it picks up
                where it stopped.
              </p>
            </div>
          </Panel>
        )}

        {step === 'ready' && (
          <>
            <Panel tone="accent" icon={Check} title="Ready">
              <p className="px-3 pb-2 text-base text-secondary">
                Hold <Kbd keys={bindings.hold.spec.split('+')} tone="accent" /> anywhere,
                say a sentence, and let go. The text will appear wherever your cursor is.
              </p>
            </Panel>

            <Panel icon={ShieldCheck} title="One thing to know">
              <p className="px-3 pb-2 text-base text-secondary">
                Your operating system will ask for microphone access the first time you
                dictate. On macOS you will also need to allow Kiku under Privacy &amp;
                Security → Accessibility, which is what lets it paste into other apps.
              </p>
            </Panel>

            <Button variant="primary" size="lg" fullWidth onClick={onDone}>
              Start using Kiku
            </Button>
          </>
        )}
      </div>
    </div>
  );
}
