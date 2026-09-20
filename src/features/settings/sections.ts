import {
  ClipboardCheck,
  Palette,
  History,
  Info,
  Keyboard,
  Mic,
  RefreshCw,
  Volume2,
  Waves,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

/**
 * The Settings sections, in the order they appear.
 *
 * Shared between the view that renders them and the navigation rail that links to
 * them, so the two can never disagree about what exists.
 */
export interface SettingsSection {
  id: string;
  label: string;
  icon: LucideIcon;
}

export const SETTINGS_SECTIONS: SettingsSection[] = [
  { id: 'shortcuts', label: 'Shortcuts', icon: Keyboard },
  { id: 'microphone', label: 'Microphone', icon: Mic },
  { id: 'model', label: 'Speech model', icon: Waves },
  { id: 'output', label: 'Output', icon: ClipboardCheck },
  { id: 'appearance', label: 'Appearance', icon: Palette },
  { id: 'sounds', label: 'Sounds', icon: Volume2 },
  { id: 'history', label: 'History', icon: History },
  { id: 'updates', label: 'Updates', icon: RefreshCw },
  { id: 'about', label: 'About', icon: Info },
];
