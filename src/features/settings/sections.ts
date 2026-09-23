import { ClipboardCheck, History, Info, Keyboard, SlidersHorizontal } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

/**
 * The Settings sections, in the order they appear.
 *
 * Shared between the view that renders them and the navigation rail that links to
 * them, so the two can never disagree about what exists.
 *
 * Five, each holding several panels. There were nine, one panel each, which turned a
 * short list of settings into a long list of places to look for them - Appearance and
 * Sounds are not two subjects, and Microphone is not a subject apart from Dictation.
 */
export interface SettingsSection {
  id: string;
  label: string;
  icon: LucideIcon;
}

export const SETTINGS_SECTIONS: SettingsSection[] = [
  { id: 'dictation', label: 'Dictation', icon: Keyboard },
  { id: 'output', label: 'Output', icon: ClipboardCheck },
  { id: 'general', label: 'General', icon: SlidersHorizontal },
  { id: 'history', label: 'History', icon: History },
  { id: 'about', label: 'About', icon: Info },
];
