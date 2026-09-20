import { useCallback, useEffect, useRef, useState } from 'react';

import { commands, events, type Entry } from '@/lib/ipc';

/** Entries fetched per request. Enough to fill a window twice over. */
const PAGE_SIZE = 60;

/** Delay before a search query is sent, so typing does not query per keystroke. */
const SEARCH_DEBOUNCE_MS = 180;

interface HistoryState {
  entries: Entry[];
  total: number;
  loading: boolean;
  error: string | null;
}

/**
 * The history list.
 *
 * Owns fetching, searching and the optimistic removal of deleted entries, so the view
 * stays a rendering concern. New dictations arrive by event rather than by polling.
 */
export function useHistory(query: string) {
  const [state, setState] = useState<HistoryState>({
    entries: [],
    total: 0,
    loading: true,
    error: null,
  });

  // Guards against an older, slower request overwriting a newer one - which is what
  // makes a search box show results for a query the user has already changed.
  const requestId = useRef(0);

  const load = useCallback(async (search: string) => {
    const id = ++requestId.current;
    setState((current) => ({ ...current, loading: true }));

    const result = await commands.listHistory(search || null, PAGE_SIZE, 0);
    if (id !== requestId.current) return;

    if (result.status === 'ok') {
      setState({
        entries: result.data.entries,
        total: result.data.total,
        loading: false,
        error: null,
      });
    } else {
      setState({ entries: [], total: 0, loading: false, error: result.error.message });
    }
  }, []);

  useEffect(() => {
    const timer = window.setTimeout(() => void load(query), query ? SEARCH_DEBOUNCE_MS : 0);
    return () => window.clearTimeout(timer);
  }, [query, load]);

  // A new dictation should appear without the user doing anything.
  useEffect(() => {
    const unlisten = events.transcriptProduced.listen(() => void load(query));
    return () => void unlisten.then((off) => off());
  }, [query, load]);

  const remove = useCallback(async (id: number) => {
    // Removed from the list first: the database call is fast, and waiting for it
    // makes the interface feel like it is thinking about an obvious request.
    setState((current) => ({
      ...current,
      entries: current.entries.filter((entry) => entry.id !== id),
      total: Math.max(0, current.total - 1),
    }));
    await commands.deleteHistoryEntry(id);
  }, []);

  // No bulk clear here: deleting everything is a settings act, and Settings calls the
  // command directly. A second copy of it would only be dead code.
  return { ...state, reload: () => void load(query), remove };
}
