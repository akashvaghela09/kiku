import type { Theme } from '@/lib/ipc';

/**
 * Applying the chosen palette.
 *
 * The resolved theme is written to `data-theme` on <html>, which is the only mechanism
 * tokens.css knows about, and mirrored into localStorage so the inline script in
 * index.html can read it synchronously before first paint. Rust remains the source of
 * truth; localStorage exists purely because preferences arrive over IPC, which is one
 * round trip too late to avoid a flash of the wrong palette.
 */

/** Must match the key read by the bootstrap script in index.html. */
const STORAGE_KEY = 'kiku-theme';

export const THEMES: readonly { value: Theme; label: string; description: string }[] = [
  { value: 'system', label: 'System', description: 'Follow the desktop' },
  { value: 'light', label: 'Light', description: '' },
  { value: 'dark', label: 'Dark', description: '' },
];

function resolve(theme: Theme): 'light' | 'dark' {
  if (theme !== 'system') return theme;
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

/**
 * Paint the chosen theme, and keep following the desktop while `system` is selected.
 *
 * Returns a cleanup function. The media query listener is what makes "System" mean
 * what it says: changing the desktop theme while Kiku is open should change Kiku,
 * rather than only taking effect at the next launch.
 */
export function applyTheme(theme: Theme): () => void {
  const paint = () => {
    document.documentElement.dataset.theme = resolve(theme);
  };

  paint();

  // Storage can throw in a locked-down webview, and a theme that cannot be cached is
  // worth far less than a window that fails to start.
  try {
    window.localStorage.setItem(STORAGE_KEY, theme);
  } catch {
    /* the bootstrap script will fall back to the desktop setting */
  }

  if (theme !== 'system') return () => {};

  const media = window.matchMedia('(prefers-color-scheme: dark)');
  media.addEventListener('change', paint);
  return () => media.removeEventListener('change', paint);
}
