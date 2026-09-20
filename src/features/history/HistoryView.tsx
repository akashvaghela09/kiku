import { useMemo, useState } from 'react';
import { Copy, Mic, Search, Trash2 } from 'lucide-react';

import { Badge, Button, Dialog, EmptyState, Input, Kbd, Panel, Row } from '@/components/ui';
import { ViewToolbar } from '@/features/shell/ViewToolbar';
import { VIEW_LABELS } from '@/features/shell/views';
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
  const [reading, setReading] = useState<Entry | null>(null);
  const { entries, total, loading, error, remove } = useHistory(query);

  const groups = useMemo(() => groupByDay(entries), [entries]);

  const copy = async (entry: Entry) => {
    await navigator.clipboard.writeText(entry.text);
    onCopied(entry.text);
  };

  return (
    <div className="flex h-full flex-col">
      <ViewToolbar
        title={VIEW_LABELS.history}
        meta={
          <span className="flex items-center gap-2.5">
            <span>{total === 1 ? '1 transcript' : `${total} transcripts`}</span>
            {paused && <Badge tone="warning">Not recording</Badge>}
          </span>
        }
      >
        <Input
          icon={Search}
          clearable
          value={query}
          placeholder="Search transcripts"
          onChange={(event) => setQuery(event.target.value)}
          onClear={() => setQuery('')}
          className="w-1/2 shrink-0"
          aria-label="Search transcripts"
        />
      </ViewToolbar>

      <div className="min-h-0 flex-1 overflow-auto">
        <div className="app-column space-y-4 py-6">
        {error ? (
          <EmptyState
            icon={Trash2}
            title="History is unavailable"
            description={error}
          />
        ) : loading && entries.length === 0 ? (
          <ul aria-hidden className="space-y-1">
            {[0, 1, 2].map((index) => (
              <li key={index} className="flex flex-col gap-2 px-3 py-3">
                <span className="h-3.5 w-3/4 rounded-sm bg-surface-active" />
                <span className="h-2.5 w-24 rounded-sm bg-surface-active" />
              </li>
            ))}
          </ul>
        ) : entries.length === 0 ? (
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
            <Panel key={day} flush eyebrow={day}>
              <ul className="divide-y divide-border-subtle border-t border-border-subtle">
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
            </Panel>
          ))
        )}
        </div>
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
      hoverable
      flush
      title={<TranscriptText text={entry.text} lines={2} onExpand={onExpand} />}
      description={
        <span className="flex items-center gap-2">
          <span>{formatRelativeTime(entry.createdAt)}</span>
          <span aria-hidden>·</span>
          <span className="font-mono">{formatDuration(entry.audioMs)}</span>
        </span>
      }
      trailing={
        <>
          <Button
            size="sm"
            variant="ghost"
            icon={Copy}
            iconOnly
            label="Copy"
            className="row-action text-muted hover:bg-surface-active hover:text-primary"
            onClick={onCopy}
          />
          <Button
            size="sm"
            variant="ghost"
            icon={Trash2}
            iconOnly
            destructive
            label="Delete"
            className="row-action text-muted hover:bg-danger-wash hover:text-danger"
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
