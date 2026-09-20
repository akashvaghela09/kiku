import { useMemo, useState } from 'react';
import { Copy, Mic, Search, Trash2 } from 'lucide-react';

import { Badge, Button, Dialog, EmptyState, Input, Kbd, Row } from '@/components/ui';
import { TranscriptText } from './TranscriptText';
import { formatDayGroup, formatDuration, formatRelativeTime } from '@/lib/format';
import type { Entry } from '@/lib/ipc';
import { useHistory } from './useHistory';

interface HistoryViewProps {
  hotkey: string[];
  paused: boolean;
  onCopied: (text: string) => void;
}

/**
 * Everything dictated so far, newest first.
 *
 * Grouped by day rather than shown as one flat list, because the question people
 * actually bring here is "what did I say earlier?" and a date is how they navigate to
 * it.
 */
export function HistoryView({ hotkey, paused, onCopied }: HistoryViewProps) {
  const [query, setQuery] = useState('');
  const [confirmClear, setConfirmClear] = useState(false);
  const [reading, setReading] = useState<Entry | null>(null);
  const { entries, total, loading, error, remove, clear } = useHistory(query);

  const groups = useMemo(() => groupByDay(entries), [entries]);

  const copy = async (entry: Entry) => {
    await navigator.clipboard.writeText(entry.text);
    onCopied(entry.text);
  };

  return (
    <div className="flex h-full flex-col">
      <header className="flex h-14 shrink-0 items-center gap-3 border-b border-border-subtle px-6">
        <h1 className="text-xl font-semibold text-primary">History</h1>
        {paused && <Badge tone="warning">Paused</Badge>}

        <div className="ml-auto flex items-center gap-2">
          <Input
            icon={Search}
            clearable
            value={query}
            placeholder="Search transcripts"
            onChange={(event) => setQuery(event.target.value)}
            onClear={() => setQuery('')}
            className="w-56"
            aria-label="Search transcripts"
          />
          {total > 0 && (
            <Button
              variant="ghost"
              icon={Trash2}
              label="Delete all"
              iconOnly
              onClick={() => setConfirmClear(true)}
            />
          )}
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-auto px-4 py-3">
        {error ? (
          <EmptyState
            icon={Trash2}
            title="History is unavailable"
            description={error}
          />
        ) : entries.length === 0 && !loading ? (
          query ? (
            <EmptyState
              compact
              icon={Search}
              title="Nothing matches that"
              description="Try a different word."
            />
          ) : (
            <EmptyState
              icon={Mic}
              title="Nothing dictated yet"
              description={
                <>
                  Hold <Kbd keys={hotkey} size="sm" /> anywhere, say something, and it
                  will appear here.
                </>
              }
            />
          )
        ) : (
          groups.map(([day, dayEntries]) => (
            <section key={day} className="mb-4">
              <h2 className="px-3 pb-1 text-2xs font-semibold uppercase tracking-[0.04em] text-muted">
                {day}
              </h2>
              <ul>
                {dayEntries.map((entry) => (
                  <TranscriptRow
                    key={entry.id}
                    entry={entry}
                    onCopy={() => void copy(entry)}
                    onDelete={() => void remove(entry.id)}
                    onExpand={() => setReading(entry)}
                  />
                ))}
              </ul>
            </section>
          ))
        )}
      </div>

      <Dialog
        open={reading !== null}
        onOpenChange={(open) => !open && setReading(null)}
        title={reading ? formatRelativeTime(reading.createdAt) : ''}
        description={
          reading
            ? `${formatDuration(reading.audioMs)} of speech`
            : undefined
        }
        confirmLabel="Copy"
        cancelLabel="Close"
        onConfirm={() => {
          if (reading) void copy(reading);
        }}
      >
        <p className="prose-transcript max-h-[46vh] overflow-auto text-base">
          {reading?.text}
        </p>
      </Dialog>

      <Dialog
        open={confirmClear}
        onOpenChange={setConfirmClear}
        tone="danger"
        title="Delete all transcripts?"
        description={`This removes all ${total} entries from this computer. It cannot be undone.`}
        confirmLabel="Delete everything"
        onConfirm={() => void clear()}
      />
    </div>
  );
}

function TranscriptRow({
  entry,
  onCopy,
  onDelete,
  onExpand,
}: {
  entry: Entry;
  onCopy: () => void;
  onDelete: () => void;
  onExpand: () => void;
}) {
  return (
    <Row
      as="li"
      align="start"
      title={<TranscriptText text={entry.text} onExpand={onExpand} />}
      description={
        <span className="flex items-center gap-2">
          <span>{formatRelativeTime(entry.createdAt)}</span>
          <span aria-hidden>·</span>
          <span className="font-mono">{formatDuration(entry.audioMs)}</span>
        </span>
      }
      trailing={
        <>
          <Button size="sm" variant="ghost" icon={Copy} iconOnly label="Copy" onClick={onCopy} />
          <Button
            size="sm"
            variant="ghost"
            icon={Trash2}
            iconOnly
            label="Delete"
            onClick={onDelete}
          />
        </>
      }
    />
  );
}

/** Group entries into day buckets, preserving the newest-first order. */
function groupByDay(entries: Entry[]): [string, Entry[]][] {
  const groups = new Map<string, Entry[]>();
  for (const entry of entries) {
    const day = formatDayGroup(entry.createdAt);
    const bucket = groups.get(day);
    if (bucket) bucket.push(entry);
    else groups.set(day, [entry]);
  }
  return [...groups.entries()];
}
