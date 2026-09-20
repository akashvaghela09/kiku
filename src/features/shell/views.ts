import { History as HistoryIcon, Settings as SettingsIcon } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

/**
 * The two screens, described once.
 *
 * The rail and the toolbar both name the current screen, and a screen whose rail entry
 * and whose heading disagree is the kind of thing nobody notices until a rename. One
 * source removes the possibility.
 */
export type View = 'history' | 'settings';

export const VIEW_LABELS: Record<View, string> = {
  history: 'History',
  settings: 'Settings',
};

export const VIEWS: readonly { id: View; icon: LucideIcon }[] = [
  { id: 'history', icon: HistoryIcon },
  { id: 'settings', icon: SettingsIcon },
];
