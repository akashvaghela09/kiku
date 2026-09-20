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

  // The live query, readable from a subscription that must outlive it. Putting `query`
  // in that effect's dependencies instead would tear the listener down and build it
  // back up on every keystroke, and since registration is asynchronous, each rebuild
  // leaves a window with nothing listening.
  const queryRef = useRef(query);
  queryRef.current = query;

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

  const reload = useCallback(() => void load(queryRef.current), [load]);

  /**
   * Keeping the list current, by two independent routes.
   *
   * The event is the fast one: a dictation lands in the database before it is
   * announced, so by the time this runs the row is already there to be read.
   *
   * Focus is the safety net, and it is not redundant. `listen` resolves a promise, so
   * an event arriving in the moments before a subscription is live is simply missed,
   * and the window is usually in the background while someone is dictating into
   * another application - which is exactly when the list would otherwise go stale
   * without anyone seeing it happen. Coming back to the window always shows the truth.
   */
  useEffect(() => {
    let cancelled = false;
    let off: (() => void) | undefined;

    // If the effect is torn down before registration completes - which StrictMode does
    // on every mount in development - unsubscribe as soon as there is something to
    // unsubscribe from, rather than leaking a listener or dropping the live one.
    void events.transcriptProduced.listen(reload).then((unlisten) => {
      if (cancelled) unlisten();
      else off = unlisten;
    });

    const refreshIfVisible = () => {
      if (document.visibilityState === 'visible') reload();
    };

    window.addEventListener('focus', refreshIfVisible);
    document.addEventListener('visibilitychange', refreshIfVisible);

    return () => {
      cancelled = true;
      off?.();
      window.removeEventListener('focus', refreshIfVisible);
      document.removeEventListener('visibilitychange', refreshIfVisible);
    };
  }, [reload]);

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
  return { ...state, reload, remove };
}
