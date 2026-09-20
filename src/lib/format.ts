/**
 * Formatting shared across the interface.
 *
 * Kept together so "how Kiku writes a duration" is one decision rather than several
 * that drift.
 */

/** Key symbols as each platform's users expect to read them. */
const MAC_KEYS: Record<string, string> = {
  cmd: '⌘',
  command: '⌘',
  meta: '⌘',
  super: '⌘',
  ctrl: '⌃',
  control: '⌃',
  alt: '⌥',
  option: '⌥',
  shift: '⇧',
  enter: '↵',
  escape: 'Esc',
};

const OTHER_KEYS: Record<string, string> = {
  control: 'Ctrl',
  option: 'Alt',
  meta: 'Win',
  super: 'Win',
  cmd: 'Win',
  command: 'Win',
  escape: 'Esc',
};

export function isMac(): boolean {
  return navigator.userAgent.includes('Mac');
}

/** Render one key name for this platform. */
export function osKeyName(key: string): string {
  const lookup = isMac() ? MAC_KEYS : OTHER_KEYS;
  const mapped = lookup[key.trim().toLowerCase()];
  if (mapped) return mapped;
  return key.charAt(0).toUpperCase() + key.slice(1);
}

/**
 * A duration in milliseconds, as a person would say it.
 *
 * Dictations are seconds long, so this optimises for that range rather than being a
 * general-purpose duration formatter.
 */
export function formatDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return '-';
  const seconds = ms / 1000;
  if (seconds < 10) return `${seconds.toFixed(1)}s`;
  if (seconds < 60) return `${Math.round(seconds)}s`;

  const minutes = Math.floor(seconds / 60);
  const remainder = Math.round(seconds % 60);
  return `${minutes}m ${remainder}s`;
}

/** Bytes as MB or GB, at the precision a download progress line needs. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '-';
  const mb = bytes / 1_048_576;
  if (mb < 1) return `${Math.round(bytes / 1024)} KB`;
  if (mb < 1024) return `${Math.round(mb)} MB`;
  return `${(mb / 1024).toFixed(1)} GB`;
}

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** Parse an RFC 3339 timestamp, the form every Kiku timestamp crosses the IPC boundary in. */
export function parseTimestamp(iso: string): number {
  const parsed = Date.parse(iso);
  return Number.isNaN(parsed) ? 0 : parsed;
}

/**
 * A timestamp as a relative phrase, falling back to a date once "days ago" stops
 * being useful.
 */
export function formatRelativeTime(iso: string, now = Date.now()): string {
  const epochMs = parseTimestamp(iso);
  const elapsed = now - epochMs;

  if (elapsed < 0) return 'just now';
  if (elapsed < MINUTE) return 'just now';
  if (elapsed < HOUR) {
    const minutes = Math.floor(elapsed / MINUTE);
    return `${minutes} min ago`;
  }
  if (elapsed < DAY) {
    const hours = Math.floor(elapsed / HOUR);
    return hours === 1 ? 'an hour ago' : `${hours} hours ago`;
  }
  if (elapsed < 7 * DAY) {
    const days = Math.floor(elapsed / DAY);
    return days === 1 ? 'yesterday' : `${days} days ago`;
  }

  return new Date(epochMs).toLocaleDateString(undefined, {
    day: 'numeric',
    month: 'short',
    year: elapsed > 365 * DAY ? 'numeric' : undefined,
  });
}

/** Day heading used to group the history list. */
export function formatDayGroup(iso: string, now = Date.now()): string {
  const date = new Date(parseTimestamp(iso));
  const today = new Date(now);
  const sameDay = (a: Date, b: Date) => a.toDateString() === b.toDateString();

  if (sameDay(date, today)) return 'Today';

  const yesterday = new Date(now - DAY);
  if (sameDay(date, yesterday)) return 'Yesterday';

  return date.toLocaleDateString(undefined, {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  });
}
